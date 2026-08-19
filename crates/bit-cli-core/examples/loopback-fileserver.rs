//! A static HTTP/1.1 file server on loopback, with byte ranges.
//!
//! It exists so a web seed can be pointed at real files without reaching the
//! network. `scripts/interop-roundtrip.ps1` uses it to prove the `url-list`
//! that `bit-cli create --web-seed` writes is understood by another client
//! (`TODO/create-seed.md`, T-084).
//!
//! It is a test fixture, not a product. It serves `GET` and `HEAD` from one
//! directory, answers a single byte range per request, and speaks no
//! compression, no keep-alive, and no conditional requests.
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
//! `--ignore-range` makes it answer every request with the whole entity and
//! `200 OK`, which is the misconfigured-mirror case a client has to detect
//! rather than accept.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

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
}

fn main() {
    let mut root = PathBuf::from(".");
    let mut port: u16 = 0;
    let mut range = RangeMode::Honour;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(next_value(&mut args, "--root")),
            "--port" => port = next_value(&mut args, "--port").parse().expect("--port"),
            "--ignore-range" => range = RangeMode::Ignore,
            "--help" | "-h" => {
                println!("usage: loopback-fileserver [--root DIR] [--port PORT] [--ignore-range]");
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

    let config = Arc::new(Config { root, range });
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

fn serve(mut stream: TcpStream, config: &Config) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }
    let mut range_header = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("range")
        {
            range_header = Some(value.trim().to_string());
        }
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let target = parts.next().unwrap_or("/");
    let path = target.split(['?', '#']).next().unwrap_or("/");

    let Some(file_path) = resolve(&config.root, path) else {
        return respond_status(&mut stream, 404, "Not Found", &method, path, "bad path");
    };
    let Ok(mut file) = File::open(&file_path) else {
        return respond_status(&mut stream, 404, "Not Found", &method, path, "no such file");
    };
    let length = file.metadata()?.len();

    let wanted = match (&range_header, config.range) {
        (Some(header), RangeMode::Honour) => match parse_range(header, length) {
            Some(range) => Some(range),
            None => {
                eprintln!("{} {method} {path} -> 416 {header}", now_iso());
                write!(
                    stream,
                    "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{length}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )?;
                return stream.flush();
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
        "HTTP/1.1 {status} {reason}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nContent-Length: {count}\r\nConnection: close\r\n"
    );
    if status == 206 {
        let end = start + count - 1;
        head.push_str(&format!("Content-Range: bytes {start}-{end}/{length}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if method.eq_ignore_ascii_case("HEAD") {
        return stream.flush();
    }

    file.seek(SeekFrom::Start(start))?;
    let mut remaining = count;
    let mut buffer = vec![0u8; 64 * 1024];
    while remaining > 0 {
        let want = remaining.min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..want])?;
        if read == 0 {
            break;
        }
        stream.write_all(&buffer[..read])?;
        remaining -= read as u64;
    }
    stream.flush()
}

fn respond_status(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    method: &str,
    path: &str,
    why: &str,
) -> std::io::Result<()> {
    eprintln!("{} {method} {path} -> {status} ({why})", now_iso());
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )?;
    stream.flush()
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
