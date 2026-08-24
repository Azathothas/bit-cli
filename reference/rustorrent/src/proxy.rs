use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const MAX_RESOLVED_ADDRESSES: usize = 16;
const MAX_PROXY_RESOLVER_WORKERS: usize = 16;
static ACTIVE_PROXY_RESOLVERS: AtomicUsize = AtomicUsize::new(0);

struct ProxyResolverGuard;

impl Drop for ProxyResolverGuard {
    fn drop(&mut self) {
        ACTIVE_PROXY_RESOLVERS.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone, Debug)]
pub enum ProxyConfig {
    Socks5 { host: String, port: u16 },
    Http { host: String, port: u16 },
}

impl ProxyConfig {
    pub fn parse(url: &str) -> Result<Self, String> {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("socks5://") {
            let (host, port) = parse_host_port(rest, 1080)?;
            Ok(ProxyConfig::Socks5 { host, port })
        } else if let Some(rest) = url.strip_prefix("http://") {
            let (host, port) = parse_host_port(rest, 8080)?;
            Ok(ProxyConfig::Http { host, port })
        } else {
            Err(format!("unsupported proxy scheme: {url}"))
        }
    }
}

fn parse_host_port(s: &str, default_port: u16) -> Result<(String, u16), String> {
    let s = s.strip_suffix('/').unwrap_or(s);
    if s.is_empty() {
        return Err("empty proxy address".to_string());
    }
    if s.bytes()
        .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
        || s.contains(['/', '?', '#', '@'])
    {
        return Err("invalid proxy address".to_string());
    }
    if s.starts_with('[') {
        let bracket_end = s
            .find(']')
            .ok_or_else(|| "invalid proxy address".to_string())?;
        let host = &s[1..bracket_end];
        if host.is_empty() {
            return Err("invalid proxy address".to_string());
        }
        let tail = &s[bracket_end + 1..];
        let port = if tail.is_empty() {
            default_port
        } else if let Some(rest) = tail.strip_prefix(':') {
            rest.parse::<u16>()
                .map_err(|_| "invalid proxy port".to_string())?
        } else {
            return Err("invalid proxy address".to_string());
        };
        if port == 0 || host.parse::<Ipv6Addr>().is_err() {
            return Err("invalid proxy address".to_string());
        }
        return Ok((host.to_string(), port));
    }
    if s.contains(']') {
        return Err("invalid proxy address".to_string());
    }
    if s.matches(':').count() > 1 {
        return Err("IPv6 proxy addresses must be bracketed".to_string());
    }
    if let Some((host, port)) = s.rsplit_once(':') {
        if host.is_empty() {
            return Err("invalid proxy address".to_string());
        }
        let port = port
            .parse::<u16>()
            .map_err(|_| "invalid proxy port".to_string())?;
        if port == 0 {
            return Err("invalid proxy port".to_string());
        }
        return Ok((host.to_string(), port));
    }
    Ok((s.to_string(), default_port))
}

pub fn connect_through_proxy(
    config: &ProxyConfig,
    target: SocketAddr,
    timeout: Duration,
) -> Result<TcpStream, String> {
    connect_through_proxy_host(config, &target.ip().to_string(), target.port(), timeout)
}

pub fn connect_through_proxy_host(
    config: &ProxyConfig,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<TcpStream, String> {
    if target_port == 0 {
        return Err("invalid proxy target port".to_string());
    }
    let target_host = validate_target_host(target_host)?;
    match config {
        ProxyConfig::Socks5 { host, port } => {
            socks5_connect(host, *port, &target_host, target_port, timeout)
        }
        ProxyConfig::Http { host, port } => {
            let target_host = format_target_host(&target_host);
            http_connect(host, *port, &target_host, target_port, timeout)
        }
    }
}

fn validate_target_host(host: &str) -> Result<String, String> {
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if host.is_empty()
        || host.len() > 255
        || host
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        || host.contains(['/', '\\', '?', '#', '@'])
    {
        return Err("invalid proxy target host".to_string());
    }
    Ok(host.to_string())
}

fn format_target_host(host: &str) -> String {
    if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

#[cfg(test)]
fn connect_target_host(target: SocketAddr) -> String {
    match target.ip() {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => format!("[{ip}]"),
    }
}

fn socks5_connect(
    proxy_host: &str,
    proxy_port: u16,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<TcpStream, String> {
    let deadline = Instant::now() + timeout;
    let mut stream = connect_proxy(
        proxy_host,
        proxy_port,
        remaining_until(deadline, "socks5")?,
        "socks5",
    )?;

    // Greeting: version 5, 1 auth method (no-auth)
    write_all_until(
        &mut stream,
        &[0x05, 0x01, 0x00],
        deadline,
        "socks5 greeting",
    )?;

    let mut auth_resp = [0u8; 2];
    read_exact_until(
        &mut stream,
        &mut auth_resp,
        deadline,
        "socks5 auth response",
    )?;
    if auth_resp[0] != 0x05 || auth_resp[1] != 0x00 {
        return Err("socks5 auth rejected".to_string());
    }

    // Connect request
    let mut req = Vec::with_capacity(22);
    req.extend_from_slice(&[0x05, 0x01, 0x00]); // VER, CMD=CONNECT, RSV
    if let Ok(ip) = target_host.parse::<Ipv4Addr>() {
        req.push(0x01); // ATYP = IPv4
        req.extend_from_slice(&ip.octets());
    } else if let Ok(ip) = target_host.parse::<Ipv6Addr>() {
        req.push(0x04); // ATYP = IPv6
        req.extend_from_slice(&ip.octets());
    } else {
        let length = u8::try_from(target_host.len())
            .map_err(|_| "socks5 target host too long".to_string())?;
        req.push(0x03); // ATYP = domain name; DNS stays at the proxy.
        req.push(length);
        req.extend_from_slice(target_host.as_bytes());
    }
    req.extend_from_slice(&target_port.to_be_bytes());

    write_all_until(&mut stream, &req, deadline, "socks5 connect request")?;

    // Read connect response: VER, REP, RSV, ATYP, BIND.ADDR, BIND.PORT
    let mut resp_header = [0u8; 4];
    read_exact_until(
        &mut stream,
        &mut resp_header,
        deadline,
        "socks5 connect response",
    )?;
    if resp_header[0] != 0x05 || resp_header[2] != 0x00 {
        return Err("socks5 invalid version in response".to_string());
    }
    if resp_header[1] != 0x00 {
        return Err(format!("socks5 connect failed: status {}", resp_header[1]));
    }

    // Skip bound address
    match resp_header[3] {
        0x01 => {
            let mut skip = [0u8; 6]; // 4 IP + 2 port
            read_exact_until(&mut stream, &mut skip, deadline, "socks5 bind address")?;
        }
        0x04 => {
            let mut skip = [0u8; 18]; // 16 IP + 2 port
            read_exact_until(&mut stream, &mut skip, deadline, "socks5 bind address")?;
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            read_exact_until(&mut stream, &mut len_buf, deadline, "socks5 domain length")?;
            let mut skip = vec![0u8; len_buf[0] as usize + 2];
            read_exact_until(&mut stream, &mut skip, deadline, "socks5 bind domain")?;
        }
        _ => return Err("socks5 invalid address type in response".to_string()),
    }

    // Remove timeouts for regular use
    stream
        .set_read_timeout(None)
        .map_err(|e| format!("socks5 clear timeout: {e}"))?;
    stream
        .set_write_timeout(None)
        .map_err(|e| format!("socks5 clear timeout: {e}"))?;

    Ok(stream)
}

fn http_connect(
    proxy_host: &str,
    proxy_port: u16,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<TcpStream, String> {
    let deadline = Instant::now() + timeout;
    let mut stream = connect_proxy(
        proxy_host,
        proxy_port,
        remaining_until(deadline, "http proxy")?,
        "http proxy",
    )?;

    let request = format!(
        "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n\r\n"
    );
    write_all_until(
        &mut stream,
        request.as_bytes(),
        deadline,
        "http proxy request",
    )?;

    // Read exactly through the response header. Reading larger chunks can consume
    // bytes sent immediately by the tunneled peer, which cannot be put back into
    // a raw TcpStream.
    let mut response = Vec::with_capacity(256);
    while response.len() < 4096 && !response.ends_with(b"\r\n\r\n") {
        let mut byte = [0u8; 1];
        read_exact_until(&mut stream, &mut byte, deadline, "http proxy response")?;
        response.push(byte[0]);
    }
    if !response.ends_with(b"\r\n\r\n") {
        return Err("http proxy response too large".to_string());
    }

    let response_str =
        std::str::from_utf8(&response).map_err(|_| "http proxy invalid response".to_string())?;
    let status_line = response_str.split("\r\n").next().unwrap_or("");
    let mut parts = status_line.split_whitespace();
    let version = parts.next().unwrap_or("");
    let status_text = parts.next().unwrap_or("");
    if !matches!(version, "HTTP/1.0" | "HTTP/1.1") || status_text.len() != 3 {
        return Err("http proxy invalid response".to_string());
    }
    let status: u16 = status_text
        .parse()
        .map_err(|_| "http proxy invalid status".to_string())?;
    if status != 200 {
        return Err(format!("http proxy connect failed: status {status}"));
    }

    stream
        .set_read_timeout(None)
        .map_err(|e| format!("http proxy clear timeout: {e}"))?;
    stream
        .set_write_timeout(None)
        .map_err(|e| format!("http proxy clear timeout: {e}"))?;

    Ok(stream)
}

fn remaining_until(deadline: Instant, label: &str) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| format!("{label} deadline exceeded"))
}

fn read_exact_until(
    stream: &mut TcpStream,
    mut bytes: &mut [u8],
    deadline: Instant,
    label: &str,
) -> Result<(), String> {
    while !bytes.is_empty() {
        let remaining = remaining_until(deadline, label)?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|err| format!("{label} set timeout: {err}"))?;
        match stream.read(bytes) {
            Ok(0) => return Err(format!("{label}: unexpected eof")),
            Ok(read) => bytes = &mut bytes[read..],
            Err(err) => return Err(format!("{label}: {err}")),
        }
    }
    Ok(())
}

fn write_all_until(
    stream: &mut TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
    label: &str,
) -> Result<(), String> {
    while !bytes.is_empty() {
        let remaining = remaining_until(deadline, label)?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|err| format!("{label} set timeout: {err}"))?;
        match stream.write(bytes) {
            Ok(0) => return Err(format!("{label}: write returned zero")),
            Ok(written) => bytes = &bytes[written..],
            Err(err) => return Err(format!("{label}: {err}")),
        }
    }
    Ok(())
}

fn connect_proxy(
    host: &str,
    port: u16,
    timeout: Duration,
    label: &str,
) -> Result<TcpStream, String> {
    let mut last_err = None;
    let started = Instant::now();
    let addrs = resolve_host(host, port, timeout)?;
    for addr in addrs {
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        match TcpStream::connect_timeout(&addr, remaining) {
            Ok(stream) => return Ok(stream),
            Err(err) => last_err = Some(err),
        }
    }
    Err(format!(
        "{label} connect: {}",
        last_err
            .map(|err| err.to_string())
            .unwrap_or_else(|| "no addresses".to_string())
    ))
}

fn resolve_host(host: &str, port: u16, timeout: Duration) -> Result<Vec<SocketAddr>, String> {
    if let Ok(ip4) = host.parse::<Ipv4Addr>() {
        return Ok(vec![SocketAddr::new(IpAddr::V4(ip4), port)]);
    }
    if let Ok(ip6) = host.parse::<Ipv6Addr>() {
        return Ok(vec![SocketAddr::new(IpAddr::V6(ip6), port)]);
    }
    if ACTIVE_PROXY_RESOLVERS
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < MAX_PROXY_RESOLVER_WORKERS).then_some(active + 1)
        })
        .is_err()
    {
        return Err(format!("proxy resolve {host}: resolver limit reached"));
    }
    let addr_str = format!("{host}:{port}");
    let host_label = host.to_string();
    let (tx, rx) = mpsc::sync_channel(1);
    if let Err(err) = thread::Builder::new()
        .name("proxy-resolver".to_string())
        .spawn(move || {
            let _guard = ProxyResolverGuard;
            let resolved = addr_str
                .to_socket_addrs()
                .map(|addrs| addrs.take(MAX_RESOLVED_ADDRESSES).collect::<Vec<_>>())
                .map_err(|err| format!("proxy resolve {host_label}: {err}"));
            let _ = tx.try_send(resolved);
        })
    {
        ACTIVE_PROXY_RESOLVERS.fetch_sub(1, Ordering::AcqRel);
        return Err(format!("proxy resolver thread: {err}"));
    }
    let addrs = rx
        .recv_timeout(timeout)
        .map_err(|_| format!("proxy resolve {host}: deadline exceeded"))??;
    if addrs.is_empty() {
        return Err(format!("proxy resolve {host}: no addresses"));
    }
    Ok(addrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn parse_socks5_url() {
        let config = ProxyConfig::parse("socks5://127.0.0.1:1080").unwrap();
        match config {
            ProxyConfig::Socks5 { host, port } => {
                assert_eq!(host, "127.0.0.1");
                assert_eq!(port, 1080);
            }
            _ => panic!("expected socks5"),
        }
    }

    #[test]
    fn parse_http_url() {
        let config = ProxyConfig::parse("http://proxy.local:3128").unwrap();
        match config {
            ProxyConfig::Http { host, port } => {
                assert_eq!(host, "proxy.local");
                assert_eq!(port, 3128);
            }
            _ => panic!("expected http"),
        }
    }

    #[test]
    fn parse_socks5_default_port() {
        let config = ProxyConfig::parse("socks5://myproxy").unwrap();
        match config {
            ProxyConfig::Socks5 { host, port } => {
                assert_eq!(host, "myproxy");
                assert_eq!(port, 1080);
            }
            _ => panic!("expected socks5"),
        }
    }

    #[test]
    fn parse_http_default_port() {
        let config = ProxyConfig::parse("http://myproxy").unwrap();
        match config {
            ProxyConfig::Http { host, port } => {
                assert_eq!(host, "myproxy");
                assert_eq!(port, 8080);
            }
            _ => panic!("expected http"),
        }
    }

    #[test]
    fn parse_unsupported_scheme_fails() {
        assert!(ProxyConfig::parse("ftp://proxy").is_err());
    }

    #[test]
    fn parse_trailing_slash_stripped() {
        let config = ProxyConfig::parse("socks5://host:9050/").unwrap();
        match config {
            ProxyConfig::Socks5 { host, port } => {
                assert_eq!(host, "host");
                assert_eq!(port, 9050);
            }
            _ => panic!("expected socks5"),
        }
    }

    #[test]
    fn parse_invalid_bracketed_proxy_fails() {
        assert!(ProxyConfig::parse("socks5://]").is_err());
        assert!(ProxyConfig::parse("socks5://[").is_err());
        assert!(ProxyConfig::parse("socks5://[::1]x").is_err());
        assert!(ProxyConfig::parse("socks5://::1").is_err());
        assert!(ProxyConfig::parse("socks5://host:notaport").is_err());
        assert!(ProxyConfig::parse("socks5://host/path").is_err());
        assert!(ProxyConfig::parse("socks5://host:0").is_err());
    }

    #[test]
    fn connect_target_host_formats_ipv6_with_brackets() {
        let v4: SocketAddr = "127.0.0.1:80".parse().unwrap();
        let v6: SocketAddr = "[2001:db8::1]:443".parse().unwrap();
        assert_eq!(connect_target_host(v4), "127.0.0.1");
        assert_eq!(connect_target_host(v6), "[2001:db8::1]");
    }

    #[test]
    fn http_connect_preserves_bytes_after_response_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            stream
                .write_all(b"HTTP/1.1 200 Connection established\r\n\r\nhello")
                .unwrap();
        });

        let mut stream = http_connect(
            "127.0.0.1",
            port,
            "example.com",
            443,
            Duration::from_secs(2),
        )
        .unwrap();
        let mut payload = [0u8; 5];
        stream.read_exact(&mut payload).unwrap();
        assert_eq!(&payload, b"hello");
        server.join().unwrap();
    }

    #[test]
    fn socks5_domain_target_is_resolved_by_the_proxy() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut greeting = [0u8; 3];
            stream.read_exact(&mut greeting).unwrap();
            assert_eq!(greeting, [0x05, 0x01, 0x00]);
            stream.write_all(&[0x05, 0x00]).unwrap();

            let mut header = [0u8; 4];
            stream.read_exact(&mut header).unwrap();
            assert_eq!(header, [0x05, 0x01, 0x00, 0x03]);
            let mut length = [0u8; 1];
            stream.read_exact(&mut length).unwrap();
            let mut domain = vec![0u8; length[0] as usize];
            stream.read_exact(&mut domain).unwrap();
            let mut port = [0u8; 2];
            stream.read_exact(&mut port).unwrap();
            assert_eq!(&domain, b"tracker.invalid");
            assert_eq!(u16::from_be_bytes(port), 6969);
            stream
                .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
                .unwrap();
        });

        let config = ProxyConfig::Socks5 {
            host: "127.0.0.1".to_string(),
            port: proxy_port,
        };
        let stream =
            connect_through_proxy_host(&config, "tracker.invalid", 6969, Duration::from_secs(2))
                .unwrap();
        drop(stream);
        server.join().unwrap();
    }

    #[test]
    fn http_connect_trickle_cannot_extend_absolute_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                if stream.read_exact(&mut byte).is_err() {
                    return;
                }
                request.push(byte[0]);
            }
            for byte in b"HTTP/1.1 200 Connection established\r\n\r\n" {
                if stream.write_all(&[*byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });

        let started = Instant::now();
        let result = http_connect(
            "127.0.0.1",
            port,
            "example.com",
            443,
            Duration::from_millis(100),
        );
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_millis(500));
        server.join().unwrap();
    }
}
