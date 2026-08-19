//! A static HTTP/1.1 file server on loopback, with byte ranges.
//!
//! It exists so a web seed can be pointed at real files without reaching the
//! network. `scripts/interop-roundtrip.ps1` uses it to prove the `url-list`
//! that `bit-cli create --web-seed` writes is understood by another client
//! (`TODO/create-seed.md`, T-084).
//!
//! It is a test fixture, not a product. It serves `GET` and `HEAD` from one
//! directory, answers a single byte range per request, keeps a connection open
//! for the next request, and speaks no compression and no conditional
//! requests.
//!
//! ```text
//! cargo run -p bit-cli-core --example loopback-fileserver -- --root .tmp/x
//! ```
//!
//! Port `0` asks the OS for a free one. The base URL is printed to stdout as a
//! single line before the first request is served, so a script can read it and
//! pass it to `--web-seed`. Every request is logged to stderr with an ISO 8601
//! UTC millisecond timestamp.
//!
//! Four failure modes, so a client's handling of each can be measured from the
//! same binary rather than by finding a broken mirror in the wild:
//!
//! - `--ignore-range` answers every request with the whole entity and
//!   `200 OK`, which is the misconfigured mirror a client has to detect rather
//!   than accept.
//! - `--status <CODE>` answers every request with that status and no body.
//!   `--status 416` is the range a mirror refuses to serve.
//! - `--stall-after <BYTES>` sends that many bytes of the body and then holds
//!   the connection open without sending another byte or closing it, which is
//!   what a mirror does when its backend hangs. A client that has no read
//!   deadline waits forever here.
//! - `--fail-after <N>` serves the first N requests normally and then switches
//!   to `--status`, which is a mirror that falls over part way through a
//!   transfer.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bit_cli_core::time::now_iso;

/// How the server answers, so a client's range handling can be tested both
/// ways from the same binary.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RangeMode {
    /// Honour `Range` and answer `206` with a `Content-Range`.
    Honour,
    /// Ignore `Range` and answer `200` with the whole entity.
    Ignore,
}

struct Config {
    root: PathBuf,
    range: RangeMode,
    /// Whether a connection is reused for the next request.
    keep_alive: bool,
    /// Answer every request with this status and no body.
    status: Option<u16>,
    /// Send this many body bytes, then stop without closing.
    stall_after: Option<u64>,
    /// Serve this many requests before the failure mode starts.
    healthy_requests: u64,
    /// Requests served so far, across every connection.
    served: AtomicU64,
}

impl Config {
    /// Whether this request falls inside the failure mode.
    fn failing(&self) -> bool {
        self.served.fetch_add(1, Ordering::Relaxed) >= self.healthy_requests
    }
}

fn main() {
    let mut root = PathBuf::from(".");
    let mut port: u16 = 0;
    let mut range = RangeMode::Honour;
    let mut status: Option<u16> = None;
    let mut stall_after: Option<u64> = None;
    let mut healthy_requests: u64 = 0;
    let mut keep_alive = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(next_value(&mut args, "--root")),
            "--port" => port = next_value(&mut args, "--port").parse().expect("--port"),
            "--ignore-range" => range = RangeMode::Ignore,
            "--no-keep-alive" => keep_alive = false,
            "--status" => {
                status = Some(next_value(&mut args, "--status").parse().expect("--status"))
            }
            "--stall-after" => {
                stall_after = Some(
                    next_value(&mut args, "--stall-after")
                        .parse()
                        .expect("--stall-after"),
                )
            }
            "--fail-after" => {
                healthy_requests = next_value(&mut args, "--fail-after")
                    .parse()
                    .expect("--fail-after")
            }
            "--help" | "-h" => {
                println!(
                    "usage: loopback-fileserver [--root DIR] [--port PORT] [--ignore-range]\n\
                     \x20                          [--no-keep-alive] [--status CODE]\n\
                     \x20                          [--stall-after BYTES] [--fail-after N]"
                );
                return;
            }
            other => {
                eprintln!("loopback-fileserver: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }

    let root = root.canonicalize().unwrap_or_else(|err| {
        eprintln!(
            "loopback-fileserver: {} is unreadable: {err}",
            root.display()
        );
        std::process::exit(2);
    });
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).expect("bind loopback");
    let bound = listener.local_addr().expect("local addr");
    // The script reads this line to learn the port, so it goes out before
    // anything else and is flushed immediately. The trailing slash matters:
    // BEP 19 appends the torrent name to a URL that ends in one.
    println!("http://127.0.0.1:{}/", bound.port());
    std::io::stdout().flush().ok();
    eprintln!(
        "{} fileserver listening on {bound}, root {}",
        now_iso(),
        root.display()
    );

    let config = Arc::new(Config {
        root,
        range,
        keep_alive,
        status,
        stall_after,
        healthy_requests,
        served: AtomicU64::new(0),
    });
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let config = config.clone();
        std::thread::spawn(move || {
            if let Err(err) = serve(stream, &config) {
                eprintln!("{} connection failed: {err}", now_iso());
            }
        });
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> String {
    match args.next() {
        Some(value) => value,
        None => {
            eprintln!("loopback-fileserver: {flag} needs a value");
            std::process::exit(2);
        }
    }
}

/// Serve requests on one connection until the client goes away.
///
/// HTTP/1.1 connections are persistent by default, and this server has to be
/// too. A server that closes after every response burns one ephemeral port per
/// request: at a few thousand requests a second, a benchmark run exhausts the
/// 16,384 port dynamic range in seconds and then measures nothing but
/// `connect` failures. `--no-keep-alive` restores the closing behaviour,
/// because a mirror that does that is its own case worth measuring.
fn serve(stream: TcpStream, config: &Config) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut stream = stream;
    loop {
        match serve_one(&mut reader, &mut stream, config)? {
            Disposition::KeepAlive => continue,
            Disposition::Close => return Ok(()),
        }
    }
}

/// What to do with the connection after one response.
enum Disposition {
    KeepAlive,
    Close,
}

fn serve_one(
    reader: &mut BufReader<TcpStream>,
    stream: &mut TcpStream,
    config: &Config,
) -> std::io::Result<Disposition> {
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(Disposition::Close);
    }
    if request_line.trim().is_empty() {
        return Ok(Disposition::Close);
    }
    let mut range_header = None;
    let mut wants_close = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("range") {
                range_header = Some(value.trim().to_string());
            } else if name.eq_ignore_ascii_case("connection") {
                wants_close = value.trim().eq_ignore_ascii_case("close");
            }
        }
    }
    let keep_alive = config.keep_alive && !wants_close;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let target = parts.next().unwrap_or("/");
    let path = target.split(['?', '#']).next().unwrap_or("/");

    // The request counter advances once per request, whatever the outcome, so
    // `--fail-after N` means "the first N requests work" no matter which
    // failure mode follows.
    let failing = config.failing();
    if failing && let Some(code) = config.status {
        return respond_status(
            stream,
            code,
            reason_for(code),
            &method,
            path,
            "forced",
            keep_alive,
        );
    }

    let Some(file_path) = resolve(&config.root, path) else {
        return respond_status(
            stream,
            404,
            "Not Found",
            &method,
            path,
            "bad path",
            keep_alive,
        );
    };
    let Ok(mut file) = File::open(&file_path) else {
        return respond_status(
            stream,
            404,
            "Not Found",
            &method,
            path,
            "no such file",
            keep_alive,
        );
    };
    let length = file.metadata()?.len();

    let wanted = match (&range_header, config.range) {
        (Some(header), RangeMode::Honour) => match parse_range(header, length) {
            Some(range) => Some(range),
            None => {
                eprintln!("{} {method} {path} -> 416 {header}", now_iso());
                write!(
                    stream,
                    "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{length}\r\nContent-Length: 0\r\nConnection: {}\r\n\r\n",
                    connection_header(keep_alive)
                )?;
                stream.flush()?;
                return Ok(disposition(keep_alive));
            }
        },
        _ => None,
    };

    let (status, reason, start, count) = match wanted {
        Some((start, end)) => (206, "Partial Content", start, end - start + 1),
        None => (200, "OK", 0, length),
    };

    eprintln!(
        "{} {method} {path} range={} -> {status} {count} byte(s)",
        now_iso(),
        range_header.as_deref().unwrap_or("-"),
    );

    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nContent-Length: {count}\r\nConnection: {}\r\n",
        connection_header(keep_alive)
    );
    if status == 206 {
        let end = start + count - 1;
        head.push_str(&format!("Content-Range: bytes {start}-{end}/{length}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if method.eq_ignore_ascii_case("HEAD") {
        stream.flush()?;
        return Ok(disposition(keep_alive));
    }

    // A stall is a `Content-Length` the server never satisfies: the promised
    // bytes stop arriving and the connection stays open. A client with no read
    // deadline waits here forever, which is the behaviour being measured.
    let stall_at = match failing {
        true => config.stall_after,
        false => None,
    };

    file.seek(SeekFrom::Start(start))?;
    let mut remaining = count;
    let mut sent = 0u64;
    let mut buffer = vec![0u8; 64 * 1024];
    while remaining > 0 {
        if let Some(limit) = stall_at
            && sent >= limit
        {
            eprintln!(
                "{} {method} {path} stalled after {sent} of {count} byte(s)",
                now_iso()
            );
            stream.flush()?;
            // Hold the connection open without closing it. The client decides
            // how long to wait; this thread is reaped when the process exits.
            std::thread::park();
            return Ok(Disposition::Close);
        }
        let ceiling = stall_at.map_or(remaining, |limit| remaining.min(limit - sent));
        let want = ceiling.max(1).min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..want])?;
        if read == 0 {
            break;
        }
        stream.write_all(&buffer[..read])?;
        remaining -= read as u64;
        sent += read as u64;
    }
    stream.flush()?;
    Ok(disposition(keep_alive))
}

/// What a `Connection` header says, and what it means for the socket.
///
/// HTTP/1.1 keeps a connection open unless told otherwise, but saying so
/// explicitly is what makes a packet capture of a failing run readable.
const fn connection_header(keep_alive: bool) -> &'static str {
    match keep_alive {
        true => "keep-alive",
        false => "close",
    }
}

const fn disposition(keep_alive: bool) -> Disposition {
    match keep_alive {
        true => Disposition::KeepAlive,
        false => Disposition::Close,
    }
}

/// The reason phrase for a status, for the forced-status mode.
///
/// Only the codes this server is asked to produce are named. Anything else
/// gets a phrase that is syntactically valid and says nothing, because a made
/// up phrase would be worse than a generic one.
fn reason_for(status: u16) -> &'static str {
    match status {
        403 => "Forbidden",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

#[allow(clippy::too_many_arguments)]
fn respond_status(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    method: &str,
    path: &str,
    why: &str,
    keep_alive: bool,
) -> std::io::Result<Disposition> {
    eprintln!("{} {method} {path} -> {status} ({why})", now_iso());
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: {}\r\n\r\n",
        connection_header(keep_alive)
    )?;
    stream.flush()?;
    Ok(disposition(keep_alive))
}

/// Map a request path onto a file under `root`, or refuse it.
///
/// Refusing rather than clamping is deliberate: a traversal attempt should
/// show up as a 404 in the log, not as a silently rewritten path.
fn resolve(root: &Path, path: &str) -> Option<PathBuf> {
    let decoded = percent_decode(path.trim_start_matches('/'));
    let decoded = String::from_utf8(decoded).ok()?;
    let mut out = root.to_path_buf();
    for segment in decoded.split('/').filter(|s| !s.is_empty()) {
        let candidate = Path::new(segment);
        if candidate
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return None;
        }
        out.push(segment);
    }
    let out = out.canonicalize().ok()?;
    out.starts_with(root).then_some(out)
}

/// Parse a single `bytes=start-end` range against a known entity length.
///
/// Multipart ranges are not supported: no BitTorrent client asks for one, and
/// answering a multi-range request with a single range would be wrong.
fn parse_range(header: &str, length: u64) -> Option<(u64, u64)> {
    let spec = header.strip_prefix("bytes=")?.trim();
    if spec.contains(',') {
        return None;
    }
    let (start, end) = spec.split_once('-')?;
    let (start, end) = match (start.trim(), end.trim()) {
        // `bytes=-N` is the last N bytes.
        ("", suffix) => {
            let n: u64 = suffix.parse().ok()?;
            (length.checked_sub(n.min(length))?, length.saturating_sub(1))
        }
        (start, "") => (start.parse().ok()?, length.saturating_sub(1)),
        (start, end) => (start.parse().ok()?, end.parse().ok()?),
    };
    if length == 0 || start > end || start >= length {
        return None;
    }
    Some((start, end.min(length - 1)))
}

fn percent_decode(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                match u8::from_str_radix(&value[i + 1..i + 3], 16) {
                    Ok(byte) => out.push(byte),
                    Err(_) => out.push(b'%'),
                }
                i += 3;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    out
}
