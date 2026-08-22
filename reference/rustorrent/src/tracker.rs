use std::fmt;
use std::io::{Read, Write};
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream, ToSocketAddrs,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use native_tls::{HandshakeError, TlsConnector, TlsStream};

use crate::bencode::{self, Value};
use crate::proxy::ProxyConfig;

const MAX_RESOLVED_ADDRESSES: usize = 16;
const TRACKER_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TRACKER_IO_TIMEOUT: Duration = Duration::from_secs(10);
const TRACKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const TRACKER_IO_POLL_INTERVAL: Duration = Duration::from_millis(100);
const TLS_HANDSHAKE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TRACKER_REDIRECT_LIMIT: usize = 5;
const MAX_TRACKER_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TRACKER_HEADER_BYTES: usize = 64 * 1024;
const MAX_TRACKER_FAILURE_REASON_CHARS: usize = 256;
const MAX_TRACKER_RESOLVER_WORKERS: usize = 16;
static ACTIVE_TRACKER_RESOLVERS: AtomicUsize = AtomicUsize::new(0);

struct TrackerResolverGuard;

impl Drop for TrackerResolverGuard {
    fn drop(&mut self) {
        ACTIVE_TRACKER_RESOLVERS.fetch_sub(1, Ordering::AcqRel);
    }
}
// Keep one tracker response within the application's hard 1024-peer handler ceiling.
const MAX_TRACKER_PEERS: usize = 1024;

#[derive(Clone, Copy)]
struct RequestBudget {
    deadline: Instant,
}

impl RequestBudget {
    fn new(deadline: Instant) -> Self {
        Self { deadline }
    }

    fn remaining(self) -> Result<Duration, Error> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or(Error::Timeout)
    }

    fn io_timeout(self, idle_deadline: Instant) -> Result<Duration, Error> {
        let now = Instant::now();
        let request_remaining = self.remaining()?;
        let idle_remaining = idle_deadline
            .checked_duration_since(now)
            .filter(|remaining| !remaining.is_zero())
            .ok_or(Error::Timeout)?;
        Ok(request_remaining
            .min(idle_remaining)
            .min(TRACKER_IO_POLL_INTERVAL))
    }
}

#[derive(Debug)]
pub struct TrackerResponse {
    pub interval: u64,
    pub peers: Vec<SocketAddr>,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Tls(native_tls::Error),
    UnsupportedScheme,
    InvalidUrl,
    InvalidPort,
    HttpParse,
    Timeout,
    HttpStatus(u16),
    Bencode(bencode::Error),
    FailureReason(String),
    Proxy(String),
    MissingField(&'static str),
    InvalidField(&'static str),
    InvalidPeers,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "io error: {err}"),
            Error::Tls(err) => write!(f, "tls error: {err}"),
            Error::UnsupportedScheme => write!(f, "unsupported scheme (only http/https)"),
            Error::InvalidUrl => write!(f, "invalid tracker url"),
            Error::InvalidPort => write!(f, "invalid tracker port"),
            Error::HttpParse => write!(f, "invalid http response"),
            Error::Timeout => write!(f, "tracker request deadline exceeded"),
            Error::HttpStatus(code) => write!(f, "http status {code}"),
            Error::Bencode(err) => write!(f, "bencode error: {err}"),
            Error::FailureReason(reason) => write!(f, "tracker failure: {reason}"),
            Error::Proxy(reason) => write!(f, "tracker proxy: {reason}"),
            Error::MissingField(field) => write!(f, "missing field: {field}"),
            Error::InvalidField(field) => write!(f, "invalid field: {field}"),
            Error::InvalidPeers => write!(f, "invalid peers list"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Self {
        Error::Tls(err)
    }
}

impl From<bencode::Error> for Error {
    fn from(err: bencode::Error) -> Self {
        Error::Bencode(err)
    }
}

#[allow(dead_code, clippy::too_many_arguments)]
pub fn announce(
    announce_url: &str,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<&str>,
    numwant: u32,
) -> Result<TrackerResponse, Error> {
    announce_with_private(
        announce_url,
        info_hash,
        peer_id,
        port,
        uploaded,
        downloaded,
        left,
        event,
        numwant,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn announce_with_private(
    announce_url: &str,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<&str>,
    numwant: u32,
    private: bool,
) -> Result<TrackerResponse, Error> {
    announce_with_private_until(
        announce_url,
        info_hash,
        peer_id,
        port,
        uploaded,
        downloaded,
        left,
        event,
        numwant,
        private,
        None,
        Instant::now() + TRACKER_REQUEST_TIMEOUT,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn announce_with_private_until(
    announce_url: &str,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<&str>,
    numwant: u32,
    private: bool,
    proxy: Option<&ProxyConfig>,
    deadline: Instant,
) -> Result<TrackerResponse, Error> {
    announce_with_private_until_policy(
        announce_url,
        info_hash,
        peer_id,
        port,
        uploaded,
        downloaded,
        left,
        event,
        numwant,
        private,
        proxy,
        deadline,
        true,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn announce_local_test(
    announce_url: &str,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<&str>,
    numwant: u32,
) -> Result<TrackerResponse, Error> {
    announce_with_private_until_policy(
        announce_url,
        info_hash,
        peer_id,
        port,
        uploaded,
        downloaded,
        left,
        event,
        numwant,
        false,
        None,
        Instant::now() + TRACKER_REQUEST_TIMEOUT,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn announce_with_private_until_policy(
    announce_url: &str,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<&str>,
    numwant: u32,
    private: bool,
    proxy: Option<&ProxyConfig>,
    deadline: Instant,
    require_public_target: bool,
) -> Result<TrackerResponse, Error> {
    let budget = RequestBudget::new(deadline);
    let mut query = build_query(
        info_hash,
        peer_id,
        port,
        uploaded,
        downloaded,
        left,
        event,
        numwant,
        proxy.is_none(),
    );
    if private {
        push_query(&mut query, "private", "1");
    }
    let mut url = announce_url.to_string();
    for _ in 0..=TRACKER_REDIRECT_LIMIT {
        budget.remaining()?;
        let parsed = parse_url(&url)?;
        let path = append_query(&parsed.path, &query);

        let mut stream = connect_stream(&parsed, budget, proxy, require_public_target)?;
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: rustorrent/0.1\r\nConnection: close\r\n\r\n",
            format_authority(&parsed)
        );
        write_all_bounded(&mut stream, request.as_bytes(), budget)?;

        let response = read_response_limited(&mut stream, budget)?;
        budget.remaining()?;
        let response = parse_http_response(&response)?;
        budget.remaining()?;
        if response.status == 200 {
            let response = parse_tracker_body(&response.body)?;
            budget.remaining()?;
            return Ok(response);
        }
        if is_redirect(response.status) {
            let location = header_value(&response.headers, "location")
                .ok_or(Error::HttpStatus(response.status))?;
            let next = resolve_location(&parsed, &location)?;
            if parsed.scheme == Scheme::Https && parse_url(&next)?.scheme == Scheme::Http {
                return Err(Error::InvalidUrl);
            }
            url = next;
            continue;
        }
        return Err(Error::HttpStatus(response.status));
    }
    Err(Error::HttpStatus(310))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scheme {
    Http,
    Https,
}

struct ParsedUrl {
    scheme: Scheme,
    host: String,
    port: u16,
    path: String,
}

struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn parse_url(url: &str) -> Result<ParsedUrl, Error> {
    let (scheme, url) = if let Some(rest) = url.strip_prefix("http://") {
        (Scheme::Http, rest)
    } else if let Some(rest) = url.strip_prefix("https://") {
        (Scheme::Https, rest)
    } else {
        return Err(Error::UnsupportedScheme);
    };
    let authority_end = url.find(['/', '?', '#']).unwrap_or(url.len());
    let host_port = &url[..authority_end];
    let suffix = &url[authority_end..];
    let suffix = suffix.split_once('#').map(|(s, _)| s).unwrap_or(suffix);
    let path = if suffix.is_empty() {
        "/".to_string()
    } else if suffix.starts_with('?') {
        format!("/{suffix}")
    } else {
        suffix.to_string()
    };

    if host_port.is_empty() {
        return Err(Error::InvalidUrl);
    }
    if host_port
        .bytes()
        .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
        || host_port.contains(['@', '/', '\\', '?', '#'])
        || path.bytes().any(|b| b.is_ascii_control() || b == b' ')
    {
        return Err(Error::InvalidUrl);
    }

    let default_port = match scheme {
        Scheme::Http => 80,
        Scheme::Https => 443,
    };

    let (host, port) = if let Some(rest) = host_port.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or(Error::InvalidUrl)?;
        host.parse::<Ipv6Addr>().map_err(|_| Error::InvalidUrl)?;
        let port = if tail.is_empty() {
            default_port
        } else {
            tail.strip_prefix(':')
                .ok_or(Error::InvalidUrl)?
                .parse::<u16>()
                .map_err(|_| Error::InvalidPort)?
        };
        (host.to_string(), port)
    } else {
        if host_port.contains(['[', ']']) || host_port.matches(':').count() > 1 {
            return Err(Error::InvalidUrl);
        }
        match host_port.rsplit_once(':') {
            Some((host, port)) if !host.is_empty() => {
                let port = port.parse::<u16>().map_err(|_| Error::InvalidPort)?;
                (host.to_string(), port)
            }
            _ => (host_port.to_string(), default_port),
        }
    };
    if host.is_empty() || port == 0 {
        return Err(Error::InvalidPort);
    }

    Ok(ParsedUrl {
        scheme,
        host,
        port,
        path,
    })
}

fn format_authority(parsed: &ParsedUrl) -> String {
    let host = if parsed.host.parse::<Ipv6Addr>().is_ok() {
        format!("[{}]", parsed.host)
    } else {
        parsed.host.clone()
    };
    let default_port = match parsed.scheme {
        Scheme::Http => 80,
        Scheme::Https => 443,
    };
    if parsed.port == default_port {
        host
    } else {
        format!("{host}:{}", parsed.port)
    }
}

fn is_retryable_io(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::WouldBlock
    )
}

fn write_all_bounded<S: DeadlineStream>(
    stream: &mut S,
    mut bytes: &[u8],
    budget: RequestBudget,
) -> Result<(), Error> {
    let mut idle_deadline = Instant::now() + TRACKER_IO_TIMEOUT;
    while !bytes.is_empty() {
        let timeout = budget.io_timeout(idle_deadline)?;
        stream.set_write_timeout(Some(timeout))?;
        match stream.write(bytes) {
            Ok(0) => {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "tracker connection closed",
                )));
            }
            Ok(written) => {
                bytes = &bytes[written..];
                idle_deadline = Instant::now() + TRACKER_IO_TIMEOUT;
            }
            Err(err) if is_retryable_io(&err) => {}
            Err(err) => return Err(Error::Io(err)),
        }
    }
    Ok(())
}

fn read_response_limited<S: DeadlineStream>(
    reader: &mut S,
    budget: RequestBudget,
) -> Result<Vec<u8>, Error> {
    let read_limit = MAX_TRACKER_RESPONSE_BYTES.saturating_add(1);
    let mut response = Vec::with_capacity(read_limit.min(8 * 1024));
    let mut chunk = [0u8; 8 * 1024];
    let mut idle_deadline = Instant::now() + TRACKER_IO_TIMEOUT;
    loop {
        let timeout = budget.io_timeout(idle_deadline)?;
        reader.set_read_timeout(Some(timeout))?;
        let remaining = read_limit.saturating_sub(response.len());
        if remaining == 0 {
            return Err(Error::HttpParse);
        }
        let read_len = remaining.min(chunk.len());
        match reader.read(&mut chunk[..read_len]) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&chunk[..read]);
                if response.len() > MAX_TRACKER_RESPONSE_BYTES {
                    return Err(Error::HttpParse);
                }
                idle_deadline = Instant::now() + TRACKER_IO_TIMEOUT;
            }
            Err(err) if is_retryable_io(&err) => {}
            Err(err) => return Err(Error::Io(err)),
        }
    }
    Ok(response)
}

enum TrackerStream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

trait DeadlineStream: Read + Write {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>;
    fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>;
}

impl DeadlineStream for TrackerStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            TrackerStream::Plain(stream) => stream.set_read_timeout(timeout),
            TrackerStream::Tls(stream) => stream.get_ref().set_read_timeout(timeout),
        }
    }

    fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            TrackerStream::Plain(stream) => stream.set_write_timeout(timeout),
            TrackerStream::Tls(stream) => stream.get_ref().set_write_timeout(timeout),
        }
    }
}

impl Read for TrackerStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            TrackerStream::Plain(stream) => stream.read(buf),
            TrackerStream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for TrackerStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            TrackerStream::Plain(stream) => stream.write(buf),
            TrackerStream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            TrackerStream::Plain(stream) => stream.flush(),
            TrackerStream::Tls(stream) => stream.flush(),
        }
    }
}

fn connect_stream(
    parsed: &ParsedUrl,
    budget: RequestBudget,
    proxy: Option<&ProxyConfig>,
    require_public_target: bool,
) -> Result<TrackerStream, Error> {
    let connect_deadline = Instant::now() + TRACKER_CONNECT_TIMEOUT;
    let stream = if let Some(proxy) = proxy {
        validate_proxy_target_host(&parsed.host, require_public_target)?;
        let remaining = budget.remaining()?.min(TRACKER_CONNECT_TIMEOUT);
        crate::proxy::connect_through_proxy_host(proxy, &parsed.host, parsed.port, remaining)
            .map_err(Error::Proxy)?
    } else {
        connect_direct(parsed, budget, connect_deadline, require_public_target)?
    };
    budget.remaining()?;
    match parsed.scheme {
        Scheme::Http => Ok(TrackerStream::Plain(stream)),
        Scheme::Https => {
            let connector = TlsConnector::new()?;
            let stream = connect_tls(&connector, &parsed.host, stream, budget)?;
            Ok(TrackerStream::Tls(stream))
        }
    }
}

fn connect_direct(
    parsed: &ParsedUrl,
    budget: RequestBudget,
    connect_deadline: Instant,
    require_public_target: bool,
) -> Result<TcpStream, Error> {
    let mut last_err: Option<std::io::Error> = None;
    let addrs = resolve_tracker_addrs(&parsed.host, parsed.port, budget, require_public_target)?;
    for addr in addrs {
        let now = Instant::now();
        let Some(connect_remaining) = connect_deadline
            .checked_duration_since(now)
            .filter(|remaining| !remaining.is_zero())
        else {
            break;
        };
        let remaining = budget.remaining()?.min(connect_remaining);
        match TcpStream::connect_timeout(&addr, remaining) {
            Ok(stream) => return Ok(stream),
            Err(err) => last_err = Some(err),
        }
    }
    Err(Error::Io(
        last_err.unwrap_or_else(|| std::io::Error::other("connect failed")),
    ))
}

fn resolve_tracker_addrs(
    host: &str,
    port: u16,
    budget: RequestBudget,
    require_public_target: bool,
) -> Result<Vec<SocketAddr>, Error> {
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        if require_public_target && !crate::http::is_public_http_ip(IpAddr::V4(ip)) {
            return Err(Error::InvalidUrl);
        }
        return Ok(vec![SocketAddr::from((ip, port))]);
    }
    if let Ok(ip) = host.parse::<Ipv6Addr>() {
        if require_public_target && !crate::http::is_public_http_ip(IpAddr::V6(ip)) {
            return Err(Error::InvalidUrl);
        }
        return Ok(vec![SocketAddr::from((ip, port))]);
    }
    if ACTIVE_TRACKER_RESOLVERS
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < MAX_TRACKER_RESOLVER_WORKERS).then_some(active + 1)
        })
        .is_err()
    {
        return Err(Error::Timeout);
    }
    let host_port = format!("{host}:{port}");
    let (tx, rx) = mpsc::sync_channel(1);
    if let Err(err) = thread::Builder::new()
        .name("http-tracker-resolver".to_string())
        .spawn(move || {
            let _guard = TrackerResolverGuard;
            let resolved = host_port.to_socket_addrs().map(|addrs| {
                let mut candidates = Vec::new();
                for addr in addrs.take(MAX_RESOLVED_ADDRESSES) {
                    if !candidates.contains(&addr) {
                        candidates.push(addr);
                    }
                }
                candidates
            });
            let _ = tx.try_send(resolved);
        })
    {
        ACTIVE_TRACKER_RESOLVERS.fetch_sub(1, Ordering::AcqRel);
        return Err(Error::Io(err));
    }
    let addrs = rx
        .recv_timeout(budget.remaining()?)
        .map_err(|_| Error::Timeout)??;
    let addrs: Vec<_> = addrs
        .into_iter()
        .filter(|addr| !require_public_target || crate::http::is_public_http_ip(addr.ip()))
        .collect();
    if addrs.is_empty() {
        return Err(Error::Io(std::io::Error::other("no resolved addresses")));
    }
    budget.remaining()?;
    Ok(addrs)
}

fn validate_proxy_target_host(host: &str, require_public_target: bool) -> Result<(), Error> {
    if !require_public_target {
        return Ok(());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return crate::http::is_public_http_ip(ip)
            .then_some(())
            .ok_or(Error::InvalidUrl);
    }
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host == "home.arpa"
        || host.ends_with(".home.arpa")
    {
        return Err(Error::InvalidUrl);
    }
    Ok(())
}

fn connect_tls(
    connector: &TlsConnector,
    host: &str,
    stream: TcpStream,
    budget: RequestBudget,
) -> Result<TlsStream<TcpStream>, Error> {
    stream.set_nonblocking(true)?;
    let mut mid = match connector.connect(host, stream) {
        Ok(stream) => {
            stream.get_ref().set_nonblocking(false)?;
            return Ok(stream);
        }
        Err(HandshakeError::Failure(err)) => return Err(Error::Tls(err)),
        Err(HandshakeError::WouldBlock(mid)) => mid,
    };
    loop {
        let remaining = budget.remaining()?;
        thread::sleep(remaining.min(TLS_HANDSHAKE_POLL_INTERVAL));
        budget.remaining()?;
        mid = match mid.handshake() {
            Ok(stream) => {
                stream.get_ref().set_nonblocking(false)?;
                return Ok(stream);
            }
            Err(HandshakeError::Failure(err)) => return Err(Error::Tls(err)),
            Err(HandshakeError::WouldBlock(next)) => next,
        };
    }
}

fn append_query(path: &str, query: &str) -> String {
    if path.contains('?') {
        format!("{path}&{query}")
    } else {
        format!("{path}?{query}")
    }
}

#[allow(clippy::too_many_arguments)]
fn build_query(
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<&str>,
    numwant: u32,
    advertise_local_ipv6: bool,
) -> String {
    let mut query = String::new();
    push_query(&mut query, "info_hash", &percent_encode(&info_hash));
    push_query(&mut query, "peer_id", &percent_encode(&peer_id));
    push_query(&mut query, "port", &port.to_string());
    push_query(&mut query, "uploaded", &uploaded.to_string());
    push_query(&mut query, "downloaded", &downloaded.to_string());
    push_query(&mut query, "left", &left.to_string());
    push_query(&mut query, "compact", "1");
    if numwant > 0 {
        push_query(&mut query, "numwant", &numwant.to_string());
    }
    if let Some(event) = event {
        if !event.is_empty() {
            push_query(&mut query, "event", &percent_encode(event.as_bytes()));
        }
    }
    // BEP 7: advertise IPv6 address if available
    if advertise_local_ipv6 {
        if let Some(ipv6) = detect_ipv6_address() {
            push_query(&mut query, "ipv6", &percent_encode(ipv6.as_bytes()));
        }
    }
    query
}

fn detect_ipv6_address() -> Option<String> {
    use std::net::{IpAddr, UdpSocket};
    let socket = UdpSocket::bind("[::]:0").ok()?;
    socket.connect("[2001:4860:4860::8888]:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V6(addr) if !addr.is_loopback() && !addr.is_unspecified() => Some(addr.to_string()),
        _ => None,
    }
}

fn push_query(target: &mut String, key: &str, value: &str) {
    if !target.is_empty() {
        target.push('&');
    }
    target.push_str(key);
    target.push('=');
    target.push_str(value);
}

fn percent_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for &b in bytes {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

fn parse_http_response(data: &[u8]) -> Result<HttpResponse, Error> {
    let header_end = find_header_end(data).ok_or(Error::HttpParse)?;
    if header_end > MAX_TRACKER_HEADER_BYTES {
        return Err(Error::HttpParse);
    }
    let header_bytes = &data[..header_end];
    let body = &data[header_end + 4..];
    let header_str = std::str::from_utf8(header_bytes).map_err(|_| Error::HttpParse)?;
    let mut lines = header_str.split("\r\n");
    let status_line = lines.next().ok_or(Error::HttpParse)?;
    let status = parse_status(status_line)?;

    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or(Error::HttpParse)?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name.is_empty() || value.bytes().any(|b| b.is_ascii_control() && b != b'\t') {
            return Err(Error::HttpParse);
        }
        headers.push((name.clone(), value.to_string()));
        if name == "content-length" {
            let parsed = value.parse::<usize>().map_err(|_| Error::HttpParse)?;
            if content_length
                .replace(parsed)
                .is_some_and(|old| old != parsed)
            {
                return Err(Error::HttpParse);
            }
        } else if name == "transfer-encoding"
            && value
                .split(',')
                .any(|v| v.trim().eq_ignore_ascii_case("chunked"))
        {
            chunked = true;
        }
    }

    let body = if chunked {
        decode_chunked(body)?
    } else if let Some(len) = content_length {
        if body.len() < len {
            return Err(Error::HttpParse);
        }
        body[..len].to_vec()
    } else {
        body.to_vec()
    };
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    headers.iter().find_map(|(key, value)| {
        if *key == name {
            Some(value.clone())
        } else {
            None
        }
    })
}

fn resolve_location(parsed: &ParsedUrl, location: &str) -> Result<String, Error> {
    let location = location.trim();
    if location.is_empty() {
        return Err(Error::InvalidUrl);
    }
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }
    let scheme = match parsed.scheme {
        Scheme::Http => "http",
        Scheme::Https => "https",
    };
    if let Some(rest) = location.strip_prefix("//") {
        return Ok(format!("{scheme}://{rest}"));
    }
    let base = format_base(parsed, scheme);
    if location.starts_with('/') {
        return Ok(format!("{base}{location}"));
    }
    let base_dir = match parsed.path.rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() => dir,
        _ => "/",
    };
    let mut path = base_dir.to_string();
    if !path.ends_with('/') {
        path.push('/');
    }
    path.push_str(location);
    Ok(format!("{base}{path}"))
}

fn format_base(parsed: &ParsedUrl, scheme: &str) -> String {
    format!("{scheme}://{}", format_authority(parsed))
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_status(line: &str) -> Result<u16, Error> {
    let mut parts = line.split_whitespace();
    let http = parts.next().ok_or(Error::HttpParse)?;
    if !matches!(http, "HTTP/1.0" | "HTTP/1.1") {
        return Err(Error::HttpParse);
    }
    let status = parts.next().ok_or(Error::HttpParse)?;
    if status.len() != 3 {
        return Err(Error::HttpParse);
    }
    status.parse::<u16>().map_err(|_| Error::HttpParse)
}

fn decode_chunked(body: &[u8]) -> Result<Vec<u8>, Error> {
    let mut pos = 0;
    let mut out = Vec::new();
    loop {
        let line_end = find_crlf(body, pos).ok_or(Error::HttpParse)?;
        let line = &body[pos..line_end];
        let line_str = std::str::from_utf8(line).map_err(|_| Error::HttpParse)?;
        let size = usize::from_str_radix(line_str.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| Error::HttpParse)?;
        pos = line_end + 2;
        if size == 0 {
            if body.get(pos..pos + 2) == Some(b"\r\n")
                || body
                    .get(pos..)
                    .is_some_and(|rest| rest.windows(4).any(|w| w == b"\r\n\r\n"))
            {
                break;
            }
            return Err(Error::HttpParse);
        }
        let end = pos.checked_add(size).ok_or(Error::HttpParse)?;
        if end > body.len() {
            return Err(Error::HttpParse);
        }
        out.extend_from_slice(&body[pos..end]);
        pos = end;
        if body.get(pos) != Some(&b'\r') || body.get(pos + 1) != Some(&b'\n') {
            return Err(Error::HttpParse);
        }
        pos += 2;
    }
    Ok(out)
}

fn find_crlf(data: &[u8], start: usize) -> Option<usize> {
    data[start..]
        .windows(2)
        .position(|window| window == b"\r\n")
        .map(|pos| start + pos)
}

fn parse_tracker_body(body: &[u8]) -> Result<TrackerResponse, Error> {
    let value = bencode::parse(body)?;
    let dict = match value {
        Value::Dict(items) => items,
        _ => return Err(Error::InvalidField("response")),
    };

    if let Some(reason) = dict_get_bytes(&dict, b"failure reason") {
        return Err(Error::FailureReason(sanitize_failure_reason(&reason)));
    }

    let interval = dict_get_int(&dict, b"interval").ok_or(Error::MissingField("interval"))?;
    let mut peers = Vec::new();
    let mut saw_peers = false;

    // Check for "complete" and "incomplete" fields (seeders/leechers)
    if let Some(complete) = dict_get_int(&dict, b"complete") {
        crate::log_stderr(format_args!("  tracker: {} seeders", complete));
    }
    if let Some(incomplete) = dict_get_int(&dict, b"incomplete") {
        crate::log_stderr(format_args!("  tracker: {} leechers", incomplete));
    }

    if let Some(peers_value) = dict_get(&dict, b"peers") {
        saw_peers = true;
        let parsed = parse_peers(peers_value, MAX_TRACKER_PEERS.saturating_sub(peers.len()))?;
        crate::log_stderr(format_args!(
            "  tracker: peers field has {} entries",
            parsed.len()
        ));
        peers.extend(parsed);
    }
    if let Some(peers6_value) = dict_get(&dict, b"peers6") {
        saw_peers = true;
        let parsed = parse_peers6(peers6_value, MAX_TRACKER_PEERS.saturating_sub(peers.len()))?;
        crate::log_stderr(format_args!(
            "  tracker: peers6 field has {} entries",
            parsed.len()
        ));
        peers.extend(parsed);
    }
    if !saw_peers {
        return Err(Error::MissingField("peers"));
    }

    Ok(TrackerResponse { interval, peers })
}

pub(crate) fn sanitize_failure_reason(reason: &[u8]) -> String {
    let sanitized: String = String::from_utf8_lossy(reason)
        .chars()
        .take(MAX_TRACKER_FAILURE_REASON_CHARS)
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '\u{061c}'
                        | '\u{200b}'..='\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2060}'..='\u{206f}'
                        | '\u{feff}'
                )
            {
                '?'
            } else {
                character
            }
        })
        .collect();
    let sanitized = sanitized.trim();
    if sanitized.is_empty() {
        "unspecified error".to_string()
    } else {
        sanitized.to_string()
    }
}

fn parse_peers(value: &Value, limit: usize) -> Result<Vec<SocketAddr>, Error> {
    match value {
        Value::Bytes(bytes) => parse_compact_peers(bytes, limit),
        Value::List(list) => parse_dict_peers(list, limit),
        _ => Err(Error::InvalidField("peers")),
    }
}

fn parse_compact_peers(bytes: &[u8], limit: usize) -> Result<Vec<SocketAddr>, Error> {
    if !bytes.len().is_multiple_of(6) {
        return Err(Error::InvalidPeers);
    }
    let peer_count = (bytes.len() / 6).min(limit);
    let mut peers = Vec::with_capacity(peer_count);
    for chunk in bytes.chunks_exact(6).take(peer_count) {
        let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        if port == 0 {
            return Err(Error::InvalidPeers);
        }
        peers.push(SocketAddr::V4(SocketAddrV4::new(ip, port)));
    }
    Ok(peers)
}

fn parse_peers6(value: &Value, limit: usize) -> Result<Vec<SocketAddr>, Error> {
    match value {
        Value::Bytes(bytes) => parse_compact_peers6(bytes, limit),
        Value::List(list) => parse_dict_peers(list, limit),
        _ => Err(Error::InvalidField("peers6")),
    }
}

fn parse_compact_peers6(bytes: &[u8], limit: usize) -> Result<Vec<SocketAddr>, Error> {
    if !bytes.len().is_multiple_of(18) {
        return Err(Error::InvalidPeers);
    }
    let peer_count = (bytes.len() / 18).min(limit);
    let mut peers = Vec::with_capacity(peer_count);
    for chunk in bytes.chunks_exact(18).take(peer_count) {
        let ip = Ipv6Addr::from([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            chunk[8], chunk[9], chunk[10], chunk[11], chunk[12], chunk[13], chunk[14], chunk[15],
        ]);
        let port = u16::from_be_bytes([chunk[16], chunk[17]]);
        if port == 0 {
            return Err(Error::InvalidPeers);
        }
        peers.push(SocketAddr::V6(SocketAddrV6::new(ip, port, 0, 0)));
    }
    Ok(peers)
}
fn parse_dict_peers(list: &[Value], limit: usize) -> Result<Vec<SocketAddr>, Error> {
    let peer_count = list.len().min(limit);
    let mut peers = Vec::with_capacity(peer_count);
    for entry in list.iter().take(peer_count) {
        let dict = match entry {
            Value::Dict(items) => items,
            _ => return Err(Error::InvalidPeers),
        };
        let ip_value = dict_get(dict, b"ip").ok_or(Error::MissingField("ip"))?;
        let port = dict_get_int(dict, b"port").ok_or(Error::MissingField("port"))?;
        let port = u16::try_from(port)
            .ok()
            .filter(|port| *port != 0)
            .ok_or(Error::InvalidField("port"))?;
        let ip_str = match ip_value {
            Value::Bytes(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            _ => return Err(Error::InvalidField("ip")),
        };
        let ip = ip_str.parse().map_err(|_| Error::InvalidField("ip"))?;
        peers.push(SocketAddr::new(ip, port));
    }
    Ok(peers)
}

fn dict_get<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter()
        .find_map(|(k, v)| if k.as_slice() == key { Some(v) } else { None })
}

fn dict_get_bytes(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<Vec<u8>> {
    dict_get(dict, key).and_then(|value| match value {
        Value::Bytes(bytes) => Some(bytes.clone()),
        _ => None,
    })
}

fn dict_get_int(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<u64> {
    dict_get(dict, key).and_then(|value| match value {
        Value::Int(num) if *num >= 0 => Some(*num as u64),
        _ => None,
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub seeders: u32,
    pub leechers: u32,
    #[allow(dead_code)]
    pub completed: u32,
}

#[allow(dead_code)]
pub fn scrape(announce_url: &str, info_hash: [u8; 20]) -> Result<ScrapeResult, Error> {
    let budget = RequestBudget::new(Instant::now() + TRACKER_REQUEST_TIMEOUT);
    let scrape_url = announce_to_scrape_url(announce_url)?;
    let query = format!("info_hash={}", percent_encode(&info_hash));
    let parsed = parse_url(&scrape_url)?;
    let path = append_query(&parsed.path, &query);

    let mut stream = connect_stream(&parsed, budget, None, true)?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: rustorrent/0.1\r\nConnection: close\r\n\r\n",
        format_authority(&parsed)
    );
    write_all_bounded(&mut stream, request.as_bytes(), budget)?;
    let response = read_response_limited(&mut stream, budget)?;
    budget.remaining()?;
    let response = parse_http_response(&response)?;
    budget.remaining()?;
    if response.status != 200 {
        return Err(Error::HttpStatus(response.status));
    }

    let value = bencode::parse(&response.body)?;
    let dict = match value {
        Value::Dict(items) => items,
        _ => return Err(Error::InvalidField("scrape response")),
    };
    let files = match dict_get(&dict, b"files") {
        Some(Value::Dict(files)) => files,
        _ => return Err(Error::MissingField("files")),
    };

    for (key, val) in files {
        if key.len() == 20 && key.as_slice() == info_hash {
            if let Value::Dict(stats) = val {
                let seeders = u32::try_from(dict_get_int(stats, b"complete").unwrap_or(0))
                    .map_err(|_| Error::InvalidField("complete"))?;
                let leechers = u32::try_from(dict_get_int(stats, b"incomplete").unwrap_or(0))
                    .map_err(|_| Error::InvalidField("incomplete"))?;
                let completed = u32::try_from(dict_get_int(stats, b"downloaded").unwrap_or(0))
                    .map_err(|_| Error::InvalidField("downloaded"))?;
                return Ok(ScrapeResult {
                    seeders,
                    leechers,
                    completed,
                });
            }
        }
    }
    Err(Error::MissingField("info_hash in scrape"))
}

#[allow(dead_code)]
fn announce_to_scrape_url(url: &str) -> Result<String, Error> {
    if let Some(pos) = url.rfind("/announce") {
        let prefix = &url[..pos];
        let suffix = &url[pos + 9..]; // skip "/announce"
        Ok(format!("{prefix}/scrape{suffix}"))
    } else {
        Err(Error::InvalidUrl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SlowStream<'a> {
        data: &'a [u8],
        position: usize,
        delay: Duration,
    }

    impl Read for SlowStream<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.position >= self.data.len() || buf.is_empty() {
                return Ok(0);
            }
            thread::sleep(self.delay);
            buf[0] = self.data[self.position];
            self.position += 1;
            Ok(1)
        }
    }

    impl Write for SlowStream<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl DeadlineStream for SlowStream<'_> {
        fn set_read_timeout(&self, _: Option<Duration>) -> std::io::Result<()> {
            Ok(())
        }

        fn set_write_timeout(&self, _: Option<Duration>) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn response_reader_enforces_absolute_deadline_against_slow_trickle() {
        let data = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        let mut stream = SlowStream {
            data,
            position: 0,
            delay: Duration::from_millis(5),
        };
        let started = Instant::now();
        let err = read_response_limited(
            &mut stream,
            RequestBudget::new(started + Duration::from_millis(35)),
        )
        .unwrap_err();
        assert!(matches!(err, Error::Timeout));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn percent_encodes_bytes() {
        let encoded = percent_encode(b"\x01a z");
        assert_eq!(encoded, "%01a%20z");
    }

    #[test]
    fn parse_compact_peer_list() {
        let bytes = [127, 0, 0, 1, 0x1A, 0xE1, 10, 0, 0, 2, 0x00, 0x50];
        let peers = parse_compact_peers(&bytes, MAX_TRACKER_PEERS).unwrap();
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0], "127.0.0.1:6881".parse().unwrap());
        assert_eq!(peers[1], "10.0.0.2:80".parse().unwrap());
    }

    #[test]
    fn parse_compact_peer_list_v6() {
        let bytes = [
            0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0x1A, 0xE1,
        ];
        let peers = parse_compact_peers6(&bytes, MAX_TRACKER_PEERS).unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], "[2001:db8::1]:6881".parse().unwrap());
    }

    #[test]
    fn compact_peer_bomb_is_truncated_during_parse() {
        let mut bytes = Vec::with_capacity((MAX_TRACKER_PEERS + 100) * 6);
        for index in 0..MAX_TRACKER_PEERS + 100 {
            let octet = (index % 250 + 1) as u8;
            bytes.extend_from_slice(&[10, 0, 0, octet, 0x1A, 0xE1]);
        }
        let peers = parse_compact_peers(&bytes, MAX_TRACKER_PEERS).unwrap();
        assert_eq!(peers.len(), MAX_TRACKER_PEERS);
    }

    #[test]
    fn tracker_body_caps_combined_ipv4_and_ipv6_peers() {
        let mut peers4 = Vec::with_capacity(MAX_TRACKER_PEERS * 6);
        let mut peers6 = Vec::with_capacity(MAX_TRACKER_PEERS * 18);
        for index in 0..MAX_TRACKER_PEERS {
            let octet = (index % 250 + 1) as u8;
            peers4.extend_from_slice(&[10, 0, 0, octet, 0x1A, 0xE1]);
            let mut addr = [0u8; 18];
            addr[0] = 0x20;
            addr[1] = 0x01;
            addr[15] = octet;
            addr[16..].copy_from_slice(&6881u16.to_be_bytes());
            peers6.extend_from_slice(&addr);
        }
        let body = bencode::encode(&Value::Dict(vec![
            (b"interval".to_vec(), Value::Int(1200)),
            (b"peers".to_vec(), Value::Bytes(peers4)),
            (b"peers6".to_vec(), Value::Bytes(peers6)),
        ]));
        let parsed = parse_tracker_body(&body).unwrap();
        assert_eq!(parsed.peers.len(), MAX_TRACKER_PEERS);
        assert!(parsed.peers.iter().all(SocketAddr::is_ipv4));
    }

    #[test]
    fn parse_compact_peers_rejects_invalid_length() {
        assert!(matches!(
            parse_compact_peers(&[1, 2, 3], MAX_TRACKER_PEERS),
            Err(Error::InvalidPeers)
        ));
        assert!(matches!(
            parse_compact_peers6(&[0u8; 17], MAX_TRACKER_PEERS),
            Err(Error::InvalidPeers)
        ));
        assert!(matches!(
            parse_compact_peers(&[127, 0, 0, 1, 0, 0], MAX_TRACKER_PEERS),
            Err(Error::InvalidPeers)
        ));
    }

    #[test]
    fn parse_tracker_body_handles_failure_reason() {
        let body = bencode::encode(&Value::Dict(vec![(
            b"failure reason".to_vec(),
            Value::Bytes(b"denied".to_vec()),
        )]));
        let err = parse_tracker_body(&body).unwrap_err();
        match err {
            Error::FailureReason(reason) => assert_eq!(reason, "denied"),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn tracker_failure_reason_is_safe_for_terminal_logs_and_bounded() {
        let mut hostile = b"denied\x1b]0;owned\x07\r\n".to_vec();
        hostile.extend(std::iter::repeat_n(
            b'x',
            MAX_TRACKER_FAILURE_REASON_CHARS + 100,
        ));
        let sanitized = sanitize_failure_reason(&hostile);
        assert!(!sanitized.chars().any(char::is_control));
        assert!(sanitized.chars().count() <= MAX_TRACKER_FAILURE_REASON_CHARS);
        assert!(sanitized.starts_with("denied?]0;owned???"));
        assert_eq!(sanitize_failure_reason(b"\r\n"), "??");
        assert_eq!(
            sanitize_failure_reason("safe\u{202e}evil\u{2060}".as_bytes()),
            "safe?evil?"
        );
    }

    #[test]
    fn http_tracker_uses_proxy_domain_without_local_target_dns() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut connect = Vec::new();
            let mut byte = [0u8; 1];
            while !connect.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                connect.push(byte[0]);
            }
            let connect = String::from_utf8(connect).unwrap();
            assert!(connect.starts_with("CONNECT tracker.invalid:80 HTTP/1.1\r\n"));
            stream
                .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                .unwrap();

            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let request = String::from_utf8_lossy(&request);
            assert!(request.starts_with("GET /announce?"));
            assert!(!request.contains("ipv6="));
            let body = bencode::encode(&Value::Dict(vec![
                (b"interval".to_vec(), Value::Int(60)),
                (b"peers".to_vec(), Value::Bytes(Vec::new())),
            ]));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });

        let proxy = ProxyConfig::Http {
            host: "127.0.0.1".to_string(),
            port: proxy_port,
        };
        let response = announce_with_private_until(
            "http://tracker.invalid/announce",
            [1u8; 20],
            [2u8; 20],
            6881,
            0,
            0,
            1,
            Some("started"),
            10,
            false,
            Some(&proxy),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(response.interval, 60);
        assert!(response.peers.is_empty());
        server.join().unwrap();
    }

    #[test]
    fn tracker_policy_rejects_private_targets_and_proxy_redirect_hosts() {
        let budget = RequestBudget::new(Instant::now() + Duration::from_secs(1));
        assert!(resolve_tracker_addrs("127.0.0.1", 80, budget, true).is_err());
        assert!(resolve_tracker_addrs("::ffff:127.0.0.1", 80, budget, true).is_err());
        assert!(validate_proxy_target_host("localhost", true).is_err());
        assert!(validate_proxy_target_host("service.home.arpa", true).is_err());
        assert!(validate_proxy_target_host("10.0.0.1", true).is_err());
        assert!(validate_proxy_target_host("tracker.example", true).is_ok());
    }

    #[test]
    fn parse_tracker_body_accepts_dict_peer_entries() {
        let peers_list = Value::List(vec![
            Value::Dict(vec![
                (b"ip".to_vec(), Value::Bytes(b"127.0.0.1".to_vec())),
                (b"port".to_vec(), Value::Int(6881)),
            ]),
            Value::Dict(vec![
                (b"ip".to_vec(), Value::Bytes(b"10.0.0.2".to_vec())),
                (b"port".to_vec(), Value::Int(80)),
            ]),
        ]);
        let body = bencode::encode(&Value::Dict(vec![
            (b"interval".to_vec(), Value::Int(1200)),
            (b"peers".to_vec(), peers_list),
        ]));
        let parsed = parse_tracker_body(&body).unwrap();
        assert_eq!(parsed.interval, 1200);
        assert_eq!(parsed.peers.len(), 2);
        assert_eq!(parsed.peers[0], "127.0.0.1:6881".parse().unwrap());
        assert_eq!(parsed.peers[1], "10.0.0.2:80".parse().unwrap());
    }

    #[test]
    fn announce_url_maps_to_scrape_url() {
        assert_eq!(
            announce_to_scrape_url("http://tracker.example/announce").unwrap(),
            "http://tracker.example/scrape"
        );
        assert_eq!(
            announce_to_scrape_url("http://tracker.example/announce?x=1").unwrap(),
            "http://tracker.example/scrape?x=1"
        );
        assert!(announce_to_scrape_url("http://tracker.example/a").is_err());
    }

    #[test]
    fn resolve_location_supports_relative_redirects() {
        let parsed = parse_url("http://tracker.example:8080/path/announce").unwrap();
        assert_eq!(
            resolve_location(&parsed, "/new").unwrap(),
            "http://tracker.example:8080/new"
        );
        assert_eq!(
            resolve_location(&parsed, "next").unwrap(),
            "http://tracker.example:8080/path/next"
        );
        assert_eq!(
            resolve_location(&parsed, "//cdn.example/x").unwrap(),
            "http://cdn.example/x"
        );
    }

    #[test]
    fn tracker_url_parser_handles_ipv6_and_rejects_injection() {
        let parsed = parse_url("http://[2001:db8::1]:8080/announce").unwrap();
        assert_eq!(parsed.host, "2001:db8::1");
        assert_eq!(format_authority(&parsed), "[2001:db8::1]:8080");
        assert!(parse_url("http://tracker.example/a\r\nX: bad").is_err());
        assert!(parse_url("http://user@tracker.example/a").is_err());
    }

    #[test]
    fn parse_http_response_supports_chunked_body() {
        let data = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\n\r\n";
        let response = parse_http_response(data).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"abc");
    }
}
