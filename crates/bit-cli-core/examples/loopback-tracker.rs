//! A minimal BEP 3 HTTP tracker bound to loopback.
//!
//! It exists so two different BitTorrent clients running on this machine can
//! find each other without the DHT, without local service discovery, and
//! without touching the network. `scripts/interop-roundtrip.ps1` uses it to
//! prove `bit-cli create` and `bit-cli seed` interoperate with another client
//! (`TODO/create-seed.md`, T-084).
//!
//! It is a test fixture, not a product. It keeps peers in memory, never
//! expires them, answers `announce` and `scrape`, and speaks the compact peer
//! format from BEP 23 plus the dictionary format when `compact=0` is asked
//! for. That is the whole surface a client needs to join a swarm.
//!
//! ```text
//! cargo run -p bit-cli-core --example loopback-tracker -- --port 6969
//! ```
//!
//! Port `0` asks the OS for a free one. The chosen announce URL is printed to
//! stdout as a single line before the first request is served, so a script can
//! read it and pass it to `--announce`. Every announce is logged to stderr
//! with an ISO 8601 UTC millisecond timestamp.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use bit_cli_core::time::now_iso;
use bit_cli_core::torrent::bencode::{Value, encode};

/// One peer in one swarm, as last announced.
#[derive(Debug, Clone)]
struct Peer {
    /// The address other peers are told to connect to. The port comes from
    /// the announce, not from the TCP source port, because a seeding client
    /// listens on a different port than it announces from.
    addr: SocketAddr,
    /// Peer id, kept so the dictionary response can carry it.
    id: Vec<u8>,
    /// Bytes still wanted. Zero means a seeder.
    left: u64,
}

/// One peer record's key: the peer id, and which family it announced over.
///
/// **Not the peer id alone.** A dual-stack client announces once per family,
/// because a tracker records the source address of the connection it was
/// announced over. Keyed by peer id alone the second announce overwrites the
/// first, the client is left reachable on one family, and which one depends on
/// the order the announces landed in. That is the exact failure
/// `TODO/peers.md` T-022 is about, and BEP 7 is the reason to key per family
/// instead: a peer has one address in each and both are worth keeping.
type PeerKey = (Vec<u8>, bool);

/// Every swarm the tracker has seen, keyed by info hash then by peer record.
type Swarms = Arc<Mutex<HashMap<Vec<u8>, HashMap<PeerKey, Peer>>>>;

fn main() {
    let mut port: u16 = 0;
    let mut interval: i64 = 5;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => port = next_value(&mut args, "--port").parse().expect("--port"),
            "--interval" => {
                interval = next_value(&mut args, "--interval")
                    .parse()
                    .expect("--interval")
            }
            "--help" | "-h" => {
                println!("usage: loopback-tracker [--port PORT] [--interval SECONDS]");
                return;
            }
            other => {
                eprintln!("loopback-tracker: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).expect("bind loopback");
    let bound = listener.local_addr().expect("local addr");
    // The same port on IPv6 loopback, so a client can announce to this
    // tracker over either family and the tracker sees a different source
    // address for each. Two listeners rather than one dual-stack socket,
    // because the standard library leaves IPV6_V6ONLY on and turning it off
    // portably is what `TODO/peers.md` T-023 is about.
    //
    // A host with no IPv6 at all keeps the IPv4 listener and says so, because
    // a fixture that refuses to start is worse than one that covers less.
    let listener6 = TcpListener::bind((Ipv6Addr::LOCALHOST, bound.port())).ok();

    // The script reads this line to learn the port, so it goes out before
    // anything else and is flushed immediately.
    println!("http://127.0.0.1:{}/announce", bound.port());
    if listener6.is_some() {
        println!("http://[::1]:{}/announce", bound.port());
    }
    std::io::stdout().flush().ok();
    eprintln!("{} tracker listening on {bound}", now_iso());
    match &listener6 {
        Some(socket) => eprintln!(
            "{} tracker listening on {}",
            now_iso(),
            socket.local_addr().expect("local addr")
        ),
        None => eprintln!("{} no IPv6 loopback: announcing over IPv4 only", now_iso()),
    }

    let swarms: Swarms = Swarms::default();
    for listener in [Some(listener), listener6].into_iter().flatten() {
        let swarms = swarms.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let swarms = swarms.clone();
                std::thread::spawn(move || {
                    if let Err(err) = serve(stream, &swarms, interval) {
                        eprintln!("{} connection failed: {err}", now_iso());
                    }
                });
            }
        });
    }
    // Both accept loops are on their own threads, so the main thread parks
    // rather than returning and taking the process with it. In a loop, because
    // `park` is allowed to return without anything having unparked it, and a
    // spurious wake here would end the run and every script driving it.
    loop {
        std::thread::park();
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> String {
    match args.next() {
        Some(value) => value,
        None => {
            eprintln!("loopback-tracker: {flag} needs a value");
            std::process::exit(2);
        }
    }
}

/// Answer one HTTP/1.1 request and close. No keep-alive: a tracker announce is
/// infrequent enough that the extra connection costs nothing, and closing
/// keeps the parser to the few lines below.
fn serve(mut stream: TcpStream, swarms: &Swarms, interval: i64) -> std::io::Result<()> {
    let peer_ip = stream.peer_addr()?.ip();
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    // Drain the headers. Nothing here depends on any of them.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }

    let target = request_line.split_whitespace().nth(1).unwrap_or("/");
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    let params = parse_query(query);

    let body = match path {
        "/announce" => announce(&params, peer_ip, swarms, interval),
        "/scrape" => scrape(&params, swarms),
        _ => failure("unknown path"),
    };

    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)?;
    stream.flush()
}

/// Record the announcing peer and answer with the rest of the swarm.
fn announce(
    params: &BTreeMap<String, Vec<u8>>,
    peer_ip: std::net::IpAddr,
    swarms: &Swarms,
    interval: i64,
) -> Vec<u8> {
    let Some(info_hash) = params.get("info_hash") else {
        return failure("no info_hash");
    };
    let Some(peer_id) = params.get("peer_id") else {
        return failure("no peer_id");
    };
    let port: u16 = match text(params, "port").and_then(|p| p.parse().ok()) {
        Some(port) => port,
        None => return failure("no port"),
    };
    let left: u64 = text(params, "left")
        .and_then(|l| l.parse().ok())
        .unwrap_or(u64::MAX);
    let event = text(params, "event").unwrap_or_default();
    let compact = text(params, "compact").as_deref() != Some("0");

    let mut swarms = swarms.lock().expect("swarm lock");
    let swarm = swarms.entry(info_hash.clone()).or_default();
    let key: PeerKey = (peer_id.clone(), peer_ip.is_ipv4());
    if event == "stopped" {
        swarm.remove(&key);
    } else {
        swarm.insert(
            key,
            Peer {
                addr: SocketAddr::new(peer_ip, port),
                id: peer_id.clone(),
                left,
            },
        );
    }

    // A peer never gets itself back, which is what makes a two-client swarm on
    // one machine behave like a real one. By peer id and not by the record's
    // key, so a client announcing over both families is not handed its own
    // other address.
    let others: Vec<&Peer> = swarm.values().filter(|p| &p.id != peer_id).collect();
    let (complete, incomplete) = counts(swarm);

    // The source family is logged because it is the thing an announce over
    // one family and an announce over the other differ in, and it is what a
    // script checks to see that both arrived.
    eprintln!(
        "{} announce info_hash={} peer_id={} from={peer_ip} family=ip{} port={port} left={left} event={} -> {} peer(s)",
        now_iso(),
        hex(info_hash),
        printable(peer_id),
        if peer_ip.is_ipv4() { "v4" } else { "v6" },
        if event.is_empty() { "-" } else { &event },
        others.len(),
    );

    // BEP 23 packs IPv4 peers six bytes each into `peers`; BEP 7 packs IPv6
    // peers eighteen bytes each into `peers6`. Both are sent, because a
    // client announcing over one family still wants to hear about the other:
    // which family it reached the tracker over decides what the tracker
    // records about it, not what it is told back.
    let mut packed6 = Vec::with_capacity(others.len() * 18);
    for peer in &others {
        if let IpAddr::V6(ip) = peer.addr.ip() {
            packed6.extend_from_slice(&ip.octets());
            packed6.extend_from_slice(&peer.addr.port().to_be_bytes());
        }
    }
    let peers = if compact {
        let mut packed = Vec::with_capacity(others.len() * 6);
        for peer in &others {
            if let IpAddr::V4(ip) = peer.addr.ip() {
                packed.extend_from_slice(&ip.octets());
                packed.extend_from_slice(&peer.addr.port().to_be_bytes());
            }
        }
        Value::Bytes(packed)
    } else {
        Value::List(
            others
                .iter()
                .map(|peer| {
                    Value::Dict(BTreeMap::from([
                        (b"peer id".to_vec(), Value::Bytes(peer.id.clone())),
                        (
                            b"ip".to_vec(),
                            Value::Bytes(peer.addr.ip().to_string().into_bytes()),
                        ),
                        (b"port".to_vec(), Value::Int(peer.addr.port() as i64)),
                    ]))
                })
                .collect(),
        )
    };

    let mut response = BTreeMap::from([
        (b"interval".to_vec(), Value::Int(interval)),
        // Same as `interval`. A one-second announce storm on loopback buries
        // the log this fixture exists to produce.
        (b"min interval".to_vec(), Value::Int(interval)),
        (b"complete".to_vec(), Value::Int(complete)),
        (b"incomplete".to_vec(), Value::Int(incomplete)),
        (b"peers".to_vec(), peers),
    ]);
    // Only when there is one. An empty `peers6` is a key every client has to
    // parse to learn nothing.
    if compact && !packed6.is_empty() {
        response.insert(b"peers6".to_vec(), Value::Bytes(packed6));
    }
    encode(&Value::Dict(response))
}

/// Seeders and leechers, counted by distinct peer rather than by record.
///
/// A dual-stack peer holds one record per family, and `complete` and
/// `incomplete` are counts of clients: counting records would report one peer
/// announcing over both families as two, and a swarm of one as a swarm of two.
fn counts(swarm: &HashMap<PeerKey, Peer>) -> (i64, i64) {
    let mut seeds: BTreeMap<&[u8], bool> = BTreeMap::new();
    for peer in swarm.values() {
        // A peer that is a seed on any of its records is a seed. It is the
        // same client either way.
        let entry = seeds.entry(&peer.id).or_insert(false);
        *entry |= peer.left == 0;
    }
    let complete = seeds.values().filter(|seed| **seed).count() as i64;
    (complete, seeds.len() as i64 - complete)
}

/// BEP 48 scrape for one or more info hashes.
fn scrape(params: &BTreeMap<String, Vec<u8>>, swarms: &Swarms) -> Vec<u8> {
    let swarms = swarms.lock().expect("swarm lock");
    let mut files = BTreeMap::new();
    let wanted: Vec<Vec<u8>> = match params.get("info_hash") {
        Some(hash) => vec![hash.clone()],
        None => swarms.keys().cloned().collect(),
    };
    for hash in wanted {
        let swarm = swarms.get(&hash).cloned().unwrap_or_default();
        let (complete, incomplete) = counts(&swarm);
        files.insert(
            hash,
            Value::Dict(BTreeMap::from([
                (b"complete".to_vec(), Value::Int(complete)),
                (b"downloaded".to_vec(), Value::Int(0)),
                (b"incomplete".to_vec(), Value::Int(incomplete)),
            ])),
        );
    }
    encode(&Value::Dict(BTreeMap::from([(
        b"files".to_vec(),
        Value::Dict(files),
    )])))
}

fn failure(reason: &str) -> Vec<u8> {
    eprintln!("{} refused: {reason}", now_iso());
    encode(&Value::Dict(BTreeMap::from([(
        b"failure reason".to_vec(),
        Value::Bytes(reason.as_bytes().to_vec()),
    )])))
}

/// A query parameter as UTF-8, for the ones that are always ASCII digits or
/// words. `info_hash` and `peer_id` are raw bytes and never go through this.
fn text(params: &BTreeMap<String, Vec<u8>>, key: &str) -> Option<String> {
    params
        .get(key)
        .map(|value| String::from_utf8_lossy(value).into_owned())
}

/// Split a query string into raw byte values.
///
/// `info_hash` and `peer_id` are twenty arbitrary bytes percent-encoded, so
/// decoding to `String` first would corrupt them. Everything stays as bytes.
fn parse_query(query: &str) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        out.insert(key.to_string(), percent_decode(value));
    }
    out
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
                    // A stray `%` is passed through rather than dropped, so a
                    // malformed request produces a wrong info hash and a
                    // visible failure instead of a silent near-match.
                    Err(_) => out.push(b'%'),
                }
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Peer ids are mostly ASCII with a random tail. Show the readable part and
/// escape the rest, so the log identifies which client announced.
fn printable(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() {
                (b as char).to_string()
            } else {
                format!("%{b:02x}")
            }
        })
        .collect()
}
