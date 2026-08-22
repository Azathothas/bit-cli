//! Tracker announce and scrape, over HTTP(S) and UDP.
//!
//! `librqbit` announces on its own while a torrent runs, but it does not
//! expose the result: which tier answered, what interval it asked for, how
//! many seeders and leechers it reported, and why a tracker failed. That is
//! exactly what `bit-cli trackers` exists to show, so the protocol is
//! implemented here rather than inferred from the session's behaviour.
//!
//! - HTTP announce and response: BEP 3, with compact peers from BEP 23.
//! - HTTP scrape: BEP 48.
//! - UDP announce and scrape: BEP 15.
//!
//! Announcing is a read-only operation from `bit-cli`'s point of view. The
//! `.torrent` is never rewritten and no state is stored between runs.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use crate::error::{Error, Result};
use crate::torrent::bencode::{self, Value};

/// The magic connection id every BEP 15 exchange starts from.
const UDP_PROTOCOL_ID: u64 = 0x0417_2710_1980;

/// BEP 15 action numbers.
const ACTION_CONNECT: u32 = 0;
const ACTION_ANNOUNCE: u32 = 1;
const ACTION_SCRAPE: u32 = 2;
const ACTION_ERROR: u32 = 3;

/// Largest UDP response worth reading. An announce reply is 20 bytes plus six
/// per peer, so this holds well over a thousand peers.
const UDP_BUFFER: usize = 8192;

/// What the client tells the tracker it is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    /// A regular interval announce.
    #[default]
    None,
    /// The first announce of a session.
    Started,
    /// The last announce before stopping.
    Stopped,
    /// The download just finished.
    Completed,
}

impl Event {
    /// The `event` query parameter, or `None` when there is none.
    pub const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Started => Some("started"),
            Self::Stopped => Some("stopped"),
            Self::Completed => Some("completed"),
        }
    }

    /// The BEP 15 numeric event.
    pub const fn as_udp(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Completed => 1,
            Self::Started => 2,
            Self::Stopped => 3,
        }
    }
}

/// What to tell the tracker about this client.
#[derive(Debug, Clone)]
pub struct Announce {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: Event,
    pub numwant: u32,
    /// A stable per-run key, which lets a tracker recognise a client whose
    /// address changed.
    pub key: u32,
}

impl Announce {
    /// An announce for a torrent nothing has been downloaded from yet.
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20], port: u16, left: u64) -> Self {
        Self {
            info_hash,
            peer_id,
            port,
            uploaded: 0,
            downloaded: 0,
            left,
            event: Event::Started,
            numwant: 50,
            // A run-scoped key. It has to differ between runs and stay fixed
            // within one, which is exactly what a random value gives.
            key: fastrand_u32(),
        }
    }
}

/// What one tracker said.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerResult {
    /// The URL as it was announced to.
    pub url: String,
    /// Which BEP 12 tier this tracker is in. Zero when the list is flat.
    pub tier: usize,
    /// `http`, `https`, or `udp`.
    pub protocol: String,
    /// Whether the tracker answered with a usable response.
    pub ok: bool,
    /// Round trip time for the whole exchange.
    pub elapsed_ms: u64,
    /// Seeders, from `complete`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeders: Option<u64>,
    /// Leechers, from `incomplete`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leechers: Option<u64>,
    /// Completed downloads, which only a scrape reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    /// Seconds the tracker asked the client to wait before announcing again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_s: Option<u64>,
    /// The shortest interval the tracker will accept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_interval_s: Option<u64>,
    /// HTTP status, for an HTTP tracker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// Peers the tracker returned.
    pub peers: Vec<String>,
    /// The tracker's `warning message`, if it sent one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// Why the announce failed. `failure reason` from the tracker, or the
    /// transport error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    /// Which address family this announce went out over, when it was pinned to
    /// one. Absent when the family was left to the resolver, which is what an
    /// announce with no family asked for.
    ///
    /// This is the whole point of announcing twice. A tracker records the
    /// source address of the connection it was announced over, so an announce
    /// that only ever went out over one family registers a peer only reachable
    /// on that family. See `TODO/peers.md`, T-022.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<Family>,
    /// The address this announce was actually sent to, once the URL's host was
    /// resolved and filtered to the family. Absent when nothing was dialled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

/// One address family, as an announce is pinned to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    V4,
    V6,
}

impl Family {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V4 => "v4",
            Self::V6 => "v6",
        }
    }

    const fn matches(self, addr: &SocketAddr) -> bool {
        matches!(
            (self, addr),
            (Self::V4, SocketAddr::V4(_)) | (Self::V6, SocketAddr::V6(_))
        )
    }

    /// The unspecified address to bind a local socket of this family to.
    const fn unspecified(self) -> SocketAddr {
        match self {
            Self::V4 => SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            Self::V6 => SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
        }
    }
}

impl std::fmt::Display for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TrackerResult {
    fn failed(url: &str, tier: usize, elapsed: Duration, reason: impl Into<String>) -> Self {
        Self {
            protocol: protocol_of(url).to_string(),
            url: url.to_string(),
            tier,
            ok: false,
            elapsed_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
            seeders: None,
            leechers: None,
            completed: None,
            interval_s: None,
            min_interval_s: None,
            http_status: None,
            peers: Vec::new(),
            warning: None,
            failure: Some(reason.into()),
            family: None,
            endpoint: None,
        }
    }
}

/// The scheme part of a tracker URL, lower-cased.
pub fn protocol_of(url: &str) -> &'static str {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("udp://") {
        "udp"
    } else if lower.starts_with("https://") {
        "https"
    } else if lower.starts_with("http://") {
        "http"
    } else {
        "unknown"
    }
}

/// A tracker client for one run.
pub struct Client {
    http: reqwest::Client,
    /// Kept so a family-pinned client can be built with the same settings.
    user_agent: String,
    connect_timeout: Duration,
    timeout: Duration,
}

impl Client {
    /// Build a client.
    pub fn new(user_agent: &str, timeout: Duration, connect_timeout: Duration) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(timeout)
            .connect_timeout(connect_timeout)
            .build()
            .map_err(|e| Error::network(format!("cannot build an HTTP client: {e}")))?;
        Ok(Self {
            http,
            user_agent: user_agent.to_string(),
            connect_timeout,
            timeout,
        })
    }

    /// Announce to one tracker, letting the resolver pick the address family.
    pub async fn announce(&self, url: &str, tier: usize, request: &Announce) -> TrackerResult {
        self.announce_on(url, tier, request, None).await
    }

    /// Announce to one tracker over one address family.
    ///
    /// `None` is the old behaviour: resolve the host and use whatever comes
    /// back first, which on a dual-stack host is whichever family the resolver
    /// happened to order first and is not a choice anyone made.
    ///
    /// A tracker under BEP 3 records the **source address of the connection**,
    /// so the family an announce goes out over decides which of this host's
    /// addresses the swarm is told about. Registering both takes two
    /// announces. See `TODO/peers.md`, T-022.
    pub async fn announce_on(
        &self,
        url: &str,
        tier: usize,
        request: &Announce,
        family: Option<Family>,
    ) -> TrackerResult {
        let started = Instant::now();
        let outcome = match protocol_of(url) {
            "udp" => self.udp(url, request, false, family).await,
            "http" | "https" => self.http_announce(url, request, family).await,
            other => Err(Error::usage(format!(
                "{url}: `{other}` is not a tracker protocol"
            ))),
        };
        match outcome {
            Ok(mut result) => {
                result.url = url.to_string();
                result.tier = tier;
                result.family = family;
                result.elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                result
            }
            Err(error) => {
                let mut result =
                    TrackerResult::failed(url, tier, started.elapsed(), error.to_string());
                result.family = family;
                result
            }
        }
    }

    /// Scrape one tracker.
    pub async fn scrape(&self, url: &str, tier: usize, request: &Announce) -> TrackerResult {
        let started = Instant::now();
        let outcome = match protocol_of(url) {
            "udp" => self.udp(url, request, true, None).await,
            "http" | "https" => match scrape_url(url) {
                Some(scrape) => self.http_scrape(&scrape, request).await,
                None => Err(Error::usage(format!(
                    "{url} does not follow the BEP 48 convention, so its scrape URL cannot be derived"
                ))),
            },
            other => Err(Error::usage(format!(
                "{url}: `{other}` is not a tracker protocol"
            ))),
        };
        match outcome {
            Ok(mut result) => {
                result.url = url.to_string();
                result.tier = tier;
                result.elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                result
            }
            Err(error) => TrackerResult::failed(url, tier, started.elapsed(), error.to_string()),
        }
    }

    async fn http_announce(
        &self,
        url: &str,
        request: &Announce,
        family: Option<Family>,
    ) -> Result<TrackerResult> {
        let full = format!("{}{}", url, announce_query(url, request));
        // A client pinned to one family, or the shared one when no family was
        // asked for. Pinning is a fresh client because the override is a
        // property of the builder; that costs about a millisecond and this is
        // a diagnostic that announces twice per tracker, not a session.
        let pinned = match family {
            None => None,
            Some(family) => Some(self.pinned_http(url, family)?),
        };
        let http = pinned
            .as_ref()
            .map(|(client, _)| client)
            .unwrap_or(&self.http);
        let response = http
            .get(&full)
            .send()
            .await
            .map_err(|e| Error::network(format!("{url}: {e}")))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|e| Error::network(format!("{url}: body was cut short: {e}")))?;
        let mut result = parse_http_response(&body)?;
        result.http_status = Some(status.as_u16());
        result.endpoint = pinned.map(|(_, endpoint)| endpoint);
        if !status.is_success() && result.failure.is_none() {
            result.ok = false;
            result.failure = Some(format!("HTTP {status}"));
        }
        Ok(result)
    }

    /// An HTTP client that will only ever reach this tracker over one family.
    ///
    /// `ClientBuilder::local_address` does **not** do this. `hyper-util` binds
    /// the local address only when it already matches the destination's family
    /// and falls through to the unspecified address of the destination's own
    /// family otherwise, so setting `0.0.0.0` still connects over IPv6. The
    /// mechanism that does work is overriding the resolution: the host is
    /// resolved here, filtered to the family, and handed to the builder, so
    /// there is no address of the other family for it to choose.
    fn pinned_http(&self, url: &str, family: Family) -> Result<(reqwest::Client, String)> {
        let (host, port) = http_authority(url)?;
        let addrs = resolve_family(&host, port, family, url)?;
        let endpoint = addrs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let client = reqwest::Client::builder()
            .user_agent(self.user_agent.clone())
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .resolve_to_addrs(&host, &addrs)
            .build()
            .map_err(|e| Error::network(format!("{url}: cannot build an HTTP client: {e}")))?;
        Ok((client, endpoint))
    }

    async fn http_scrape(&self, url: &str, request: &Announce) -> Result<TrackerResult> {
        let full = format!(
            "{}{}info_hash={}",
            url,
            if url.contains('?') { "&" } else { "?" },
            percent_encode(&request.info_hash)
        );
        let response = self
            .http
            .get(&full)
            .send()
            .await
            .map_err(|e| Error::network(format!("{url}: {e}")))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|e| Error::network(format!("{url}: body was cut short: {e}")))?;
        let mut result = parse_scrape_response(&body, &request.info_hash)?;
        result.http_status = Some(status.as_u16());
        if !status.is_success() && result.failure.is_none() {
            result.ok = false;
            result.failure = Some(format!("HTTP {status}"));
        }
        Ok(result)
    }

    /// One BEP 15 exchange: connect, then announce or scrape.
    async fn udp(
        &self,
        url: &str,
        request: &Announce,
        scrape: bool,
        family: Option<Family>,
    ) -> Result<TrackerResult> {
        let target = udp_target(url, family)?;
        // The local socket is bound in the destination's family either way.
        // With a family asked for, `udp_target` has already made sure the
        // destination is in it.
        let bind: SocketAddr = match target.is_ipv4() {
            true => Family::V4.unspecified(),
            false => Family::V6.unspecified(),
        };
        let socket = UdpSocket::bind(bind)
            .await
            .map_err(|e| Error::network(format!("{url}: cannot open a UDP socket: {e}")))?;
        socket
            .connect(target)
            .await
            .map_err(|e| Error::network(format!("{url}: cannot reach {target}: {e}")))?;

        let transaction = fastrand_u32();
        let reply = self
            .udp_exchange(
                &socket,
                url,
                &connect_request(transaction),
                ACTION_CONNECT,
                transaction,
            )
            .await?;
        if reply.len() < 16 {
            return Err(Error::network(format!("{url}: short connect response")));
        }
        let connection_id = u64::from_be_bytes(reply[8..16].try_into().unwrap_or([0; 8]));

        let transaction = fastrand_u32();
        let (payload, action) = match scrape {
            true => (
                scrape_request(connection_id, transaction, &request.info_hash),
                ACTION_SCRAPE,
            ),
            false => (
                announce_request(connection_id, transaction, request),
                ACTION_ANNOUNCE,
            ),
        };
        let reply = self
            .udp_exchange(&socket, url, &payload, action, transaction)
            .await?;
        let mut result = match scrape {
            true => parse_udp_scrape(&reply)?,
            false => parse_udp_announce(&reply)?,
        };
        result.endpoint = Some(target.to_string());
        Ok(result)
    }

    /// Send one UDP request and read the matching reply.
    ///
    /// BEP 15 says to retry with an exponential backoff. Three attempts inside
    /// the configured timeout is enough to ride out a dropped datagram without
    /// making a dead tracker cost a minute.
    async fn udp_exchange(
        &self,
        socket: &UdpSocket,
        url: &str,
        payload: &[u8],
        expect_action: u32,
        transaction: u32,
    ) -> Result<Vec<u8>> {
        let per_attempt = (self.timeout / 3).max(Duration::from_secs(1));
        let mut last = String::new();
        for _ in 0..3 {
            socket
                .send(payload)
                .await
                .map_err(|e| Error::network(format!("{url}: cannot send: {e}")))?;
            let mut buf = vec![0u8; UDP_BUFFER];
            match tokio::time::timeout(per_attempt, socket.recv(&mut buf)).await {
                Err(_) => last = "timed out waiting for a reply".to_string(),
                Ok(Err(e)) => last = format!("cannot read: {e}"),
                Ok(Ok(n)) => {
                    buf.truncate(n);
                    if n < 8 {
                        last = format!("short reply of {n} bytes");
                        continue;
                    }
                    let action = u32::from_be_bytes(buf[0..4].try_into().unwrap_or([0; 4]));
                    let echoed = u32::from_be_bytes(buf[4..8].try_into().unwrap_or([0; 4]));
                    if echoed != transaction {
                        // A reply to an earlier attempt. Keep waiting rather
                        // than treating a stale datagram as this answer.
                        last = "reply carried a different transaction id".to_string();
                        continue;
                    }
                    if action == ACTION_ERROR {
                        let text = String::from_utf8_lossy(&buf[8..]).trim().to_string();
                        return Err(Error::network(format!("{url}: {text}")));
                    }
                    if action != expect_action {
                        return Err(Error::network(format!(
                            "{url}: expected action {expect_action}, got {action}"
                        )));
                    }
                    return Ok(buf);
                }
            }
        }
        Err(Error::network(format!("{url}: {last}")))
    }
}

/// The query string for an HTTP announce, including the leading separator.
pub fn announce_query(url: &str, request: &Announce) -> String {
    let mut query = String::from(match url.contains('?') {
        true => "&",
        false => "?",
    });
    query.push_str(&format!("info_hash={}", percent_encode(&request.info_hash)));
    query.push_str(&format!("&peer_id={}", percent_encode(&request.peer_id)));
    query.push_str(&format!("&port={}", request.port));
    query.push_str(&format!("&uploaded={}", request.uploaded));
    query.push_str(&format!("&downloaded={}", request.downloaded));
    query.push_str(&format!("&left={}", request.left));
    query.push_str("&compact=1&no_peer_id=1");
    query.push_str(&format!("&numwant={}", request.numwant));
    query.push_str(&format!("&key={:08x}", request.key));
    if let Some(event) = request.event.as_str() {
        query.push_str(&format!("&event={event}"));
    }
    query
}

/// The BEP 48 scrape URL for an announce URL, when one can be derived.
///
/// The convention is that the last path component is `announce` and the scrape
/// endpoint replaces it with `scrape`. A tracker whose path does not end that
/// way has no defined scrape URL, and guessing one produces a 404 that reads
/// like a tracker failure.
pub fn scrape_url(announce: &str) -> Option<String> {
    let (base, query) = match announce.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (announce, None),
    };
    let (head, last) = base.rsplit_once('/')?;
    let rest = last.strip_prefix("announce")?;
    let mut out = format!("{head}/scrape{rest}");
    if let Some(query) = query {
        out.push('?');
        out.push_str(query);
    }
    Some(out)
}

/// Parse a bencoded HTTP announce response.
pub fn parse_http_response(body: &[u8]) -> Result<TrackerResult> {
    let value = bencode::decode(body)
        .map_err(|e| Error::network(format!("the tracker did not send bencode: {e}")))?;

    let mut result = TrackerResult {
        url: String::new(),
        tier: 0,
        protocol: String::new(),
        ok: true,
        elapsed_ms: 0,
        seeders: value
            .get("complete")
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        leechers: value
            .get("incomplete")
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        completed: value
            .get("downloaded")
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        interval_s: value
            .get("interval")
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        min_interval_s: value
            .get("min interval")
            .or_else(|| value.get("min_interval"))
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        http_status: None,
        peers: Vec::new(),
        warning: value.get("warning message").and_then(Value::as_text),
        failure: value.get("failure reason").and_then(Value::as_text),
        family: None,
        endpoint: None,
    };
    if result.failure.is_some() {
        result.ok = false;
        return Ok(result);
    }

    if let Some(peers) = value.get("peers") {
        result.peers.extend(parse_peers(peers, false));
    }
    if let Some(peers) = value.get("peers6") {
        result.peers.extend(parse_peers(peers, true));
    }
    Ok(result)
}

/// Parse a bencoded BEP 48 scrape response.
pub fn parse_scrape_response(body: &[u8], info_hash: &[u8; 20]) -> Result<TrackerResult> {
    let value = bencode::decode(body)
        .map_err(|e| Error::network(format!("the tracker did not send bencode: {e}")))?;

    if let Some(reason) = value
        .get("failure reason")
        .and_then(Value::as_text)
        .or_else(|| value.get("failure_reason").and_then(Value::as_text))
    {
        return Ok(TrackerResult::failed("", 0, Duration::ZERO, reason));
    }

    let entry = value
        .get("files")
        .and_then(Value::as_dict)
        .and_then(|files| files.get(info_hash.as_slice()));
    let Some(entry) = entry else {
        return Ok(TrackerResult::failed(
            "",
            0,
            Duration::ZERO,
            "the tracker does not know this info hash",
        ));
    };

    Ok(TrackerResult {
        url: String::new(),
        tier: 0,
        protocol: String::new(),
        ok: true,
        elapsed_ms: 0,
        seeders: entry
            .get("complete")
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        leechers: entry
            .get("incomplete")
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        completed: entry
            .get("downloaded")
            .and_then(Value::as_int)
            .map(|n| n.max(0) as u64),
        interval_s: None,
        min_interval_s: None,
        http_status: None,
        peers: Vec::new(),
        warning: None,
        failure: None,
        family: None,
        endpoint: None,
    })
}

/// Peers from either the compact form (BEP 23) or the dictionary form.
fn parse_peers(value: &Value, ipv6: bool) -> Vec<String> {
    let stride = match ipv6 {
        true => 18,
        false => 6,
    };
    if let Some(bytes) = value.as_bytes() {
        return bytes
            .chunks_exact(stride)
            .map(|chunk| match ipv6 {
                true => {
                    let mut octets = [0u8; 16];
                    octets.copy_from_slice(&chunk[..16]);
                    let port = u16::from_be_bytes([chunk[16], chunk[17]]);
                    SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(octets), port, 0, 0))
                        .to_string()
                }
                false => {
                    let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                    SocketAddr::V4(SocketAddrV4::new(ip, port)).to_string()
                }
            })
            .collect();
    }
    value
        .as_list()
        .unwrap_or_default()
        .iter()
        .filter_map(|peer| {
            let ip = peer.get("ip")?.as_text()?;
            let port = peer.get("port")?.as_int()?;
            Some(match ip.contains(':') {
                true => format!("[{ip}]:{port}"),
                false => format!("{ip}:{port}"),
            })
        })
        .collect()
}

/// The `host:port` a `udp://` tracker URL points at.
fn udp_target(url: &str, family: Option<Family>) -> Result<SocketAddr> {
    let rest = url
        .trim()
        .strip_prefix("udp://")
        .ok_or_else(|| Error::usage(format!("{url} is not a udp:// URL")))?;
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    let mut resolved = std::net::ToSocketAddrs::to_socket_addrs(&authority)
        .map_err(|e| Error::network(format!("{url}: cannot resolve {authority}: {e}")))?;
    match family {
        // What this used to do, always: take whatever the resolver put first.
        // On a dual-stack host that is not a choice, it is an ordering.
        None => resolved
            .next()
            .ok_or_else(|| Error::network(format!("{url}: {authority} resolved to no address"))),
        Some(family) => resolved.find(|addr| family.matches(addr)).ok_or_else(|| {
            Error::network(format!(
                "{url}: {authority} has no IP{family} address to announce over"
            ))
        }),
    }
}

/// Which address families a tracker URL resolves to, in a stable order.
///
/// This is what decides how many announces a tracker gets. A host with both an
/// A and an AAAA record is two announces, because a tracker records the source
/// address of the connection and one announce registers one of this host's
/// addresses. A host with one is one, and a host that resolves to nothing is
/// the error the caller reports.
pub fn families_of(url: &str) -> Result<Vec<Family>> {
    let (host, port) = match protocol_of(url) {
        "udp" => {
            let rest = url
                .trim()
                .strip_prefix("udp://")
                .ok_or_else(|| Error::usage(format!("{url} is not a udp:// URL")))?;
            let authority = rest.split(['/', '?']).next().unwrap_or(rest);
            match authority.rsplit_once(':') {
                Some((host, port)) => (
                    host.trim_matches(['[', ']']).to_string(),
                    port.parse()
                        .map_err(|_| Error::usage(format!("{url}: `{port}` is not a port")))?,
                ),
                None => {
                    return Err(Error::usage(format!(
                        "{url}: a udp:// tracker needs a port"
                    )));
                }
            }
        }
        "http" | "https" => http_authority(url)?,
        other => {
            return Err(Error::usage(format!(
                "{url}: `{other}` is not a tracker protocol"
            )));
        }
    };
    let mut families: Vec<Family> =
        std::net::ToSocketAddrs::to_socket_addrs(&(host.as_str(), port))
            .map_err(|e| Error::network(format!("{url}: cannot resolve {host}: {e}")))?
            .map(|addr| match addr {
                SocketAddr::V4(_) => Family::V4,
                SocketAddr::V6(_) => Family::V6,
            })
            .collect();
    families.sort();
    families.dedup();
    match families.is_empty() {
        true => Err(Error::network(format!(
            "{url}: {host} resolved to no address"
        ))),
        false => Ok(families),
    }
}

/// The host and port of an HTTP tracker URL, for resolving it by hand.
///
/// Written out rather than pulled from a URL crate because this file already
/// parses these URLs by hand everywhere else, and the shape it has to handle
/// is the same one `protocol_of` and `scrape_url` handle.
fn http_authority(url: &str) -> Result<(String, u16)> {
    let trimmed = url.trim();
    let (scheme, rest) = match trimmed.split_once("://") {
        Some(pair) => pair,
        None => return Err(Error::usage(format!("{url} is not an http:// URL"))),
    };
    let default_port = match scheme.to_ascii_lowercase().as_str() {
        "http" => 80,
        "https" => 443,
        other => return Err(Error::usage(format!("{url}: `{other}` is not HTTP"))),
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    // Userinfo is legal in a URL and is not part of the host.
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    // A bracketed IPv6 literal carries colons of its own, so the port split
    // has to happen after the bracket rather than at the last colon.
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest
            .split_once(']')
            .ok_or_else(|| Error::usage(format!("{url}: unclosed [ in the host")))?;
        let port = match tail.strip_prefix(':') {
            Some(port) => port
                .parse()
                .map_err(|_| Error::usage(format!("{url}: `{port}` is not a port")))?,
            None => default_port,
        };
        return Ok((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => Ok((
            host.to_string(),
            port.parse()
                .map_err(|_| Error::usage(format!("{url}: `{port}` is not a port")))?,
        )),
        None => Ok((authority.to_string(), default_port)),
    }
}

/// Resolve a host to every address it has in one family.
fn resolve_family(host: &str, port: u16, family: Family, url: &str) -> Result<Vec<SocketAddr>> {
    let addrs: Vec<SocketAddr> = std::net::ToSocketAddrs::to_socket_addrs(&(host, port))
        .map_err(|e| Error::network(format!("{url}: cannot resolve {host}: {e}")))?
        .filter(|addr| family.matches(addr))
        .collect();
    match addrs.is_empty() {
        true => Err(Error::network(format!(
            "{url}: {host} has no IP{family} address to announce over"
        ))),
        false => Ok(addrs),
    }
}

fn connect_request(transaction: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&UDP_PROTOCOL_ID.to_be_bytes());
    out.extend_from_slice(&ACTION_CONNECT.to_be_bytes());
    out.extend_from_slice(&transaction.to_be_bytes());
    out
}

fn announce_request(connection_id: u64, transaction: u32, request: &Announce) -> Vec<u8> {
    let mut out = Vec::with_capacity(98);
    out.extend_from_slice(&connection_id.to_be_bytes());
    out.extend_from_slice(&ACTION_ANNOUNCE.to_be_bytes());
    out.extend_from_slice(&transaction.to_be_bytes());
    out.extend_from_slice(&request.info_hash);
    out.extend_from_slice(&request.peer_id);
    out.extend_from_slice(&request.downloaded.to_be_bytes());
    out.extend_from_slice(&request.left.to_be_bytes());
    out.extend_from_slice(&request.uploaded.to_be_bytes());
    out.extend_from_slice(&request.event.as_udp().to_be_bytes());
    // IP address zero means "use the source address of this datagram", which
    // is right for every case that is not an explicit override.
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&request.key.to_be_bytes());
    out.extend_from_slice(&request.numwant.to_be_bytes());
    out.extend_from_slice(&request.port.to_be_bytes());
    out
}

fn scrape_request(connection_id: u64, transaction: u32, info_hash: &[u8; 20]) -> Vec<u8> {
    let mut out = Vec::with_capacity(36);
    out.extend_from_slice(&connection_id.to_be_bytes());
    out.extend_from_slice(&ACTION_SCRAPE.to_be_bytes());
    out.extend_from_slice(&transaction.to_be_bytes());
    out.extend_from_slice(info_hash);
    out
}

/// Parse a BEP 15 announce reply.
pub fn parse_udp_announce(reply: &[u8]) -> Result<TrackerResult> {
    if reply.len() < 20 {
        return Err(Error::network(format!(
            "short announce reply of {} bytes",
            reply.len()
        )));
    }
    let interval = u32::from_be_bytes(reply[8..12].try_into().unwrap_or([0; 4]));
    let leechers = u32::from_be_bytes(reply[12..16].try_into().unwrap_or([0; 4]));
    let seeders = u32::from_be_bytes(reply[16..20].try_into().unwrap_or([0; 4]));
    // Six bytes per peer, four of address and two of port. A trailing partial
    // entry is not a peer and is dropped.
    let (entries, _) = reply[20..].as_chunks::<6>();
    let peers = entries
        .iter()
        .map(|chunk| {
            let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
            let port = u16::from_be_bytes([chunk[4], chunk[5]]);
            SocketAddr::V4(SocketAddrV4::new(ip, port)).to_string()
        })
        .collect();

    Ok(TrackerResult {
        url: String::new(),
        tier: 0,
        protocol: "udp".to_string(),
        ok: true,
        elapsed_ms: 0,
        seeders: Some(u64::from(seeders)),
        leechers: Some(u64::from(leechers)),
        completed: None,
        interval_s: Some(u64::from(interval)),
        min_interval_s: None,
        http_status: None,
        peers,
        warning: None,
        failure: None,
        family: None,
        endpoint: None,
    })
}

/// Parse a BEP 15 scrape reply for one info hash.
pub fn parse_udp_scrape(reply: &[u8]) -> Result<TrackerResult> {
    if reply.len() < 20 {
        return Err(Error::network(format!(
            "short scrape reply of {} bytes",
            reply.len()
        )));
    }
    let seeders = u32::from_be_bytes(reply[8..12].try_into().unwrap_or([0; 4]));
    let completed = u32::from_be_bytes(reply[12..16].try_into().unwrap_or([0; 4]));
    let leechers = u32::from_be_bytes(reply[16..20].try_into().unwrap_or([0; 4]));

    Ok(TrackerResult {
        url: String::new(),
        tier: 0,
        protocol: "udp".to_string(),
        ok: true,
        elapsed_ms: 0,
        seeders: Some(u64::from(seeders)),
        leechers: Some(u64::from(leechers)),
        completed: Some(u64::from(completed)),
        interval_s: None,
        min_interval_s: None,
        http_status: None,
        peers: Vec::new(),
        warning: None,
        failure: None,
        family: None,
        endpoint: None,
    })
}

/// Percent-encode raw bytes for a tracker query string.
///
/// A tracker query carries the twenty raw bytes of an info hash, not its hex
/// rendering, so this encodes everything outside the unreserved set from
/// RFC 3986. Getting this wrong produces a tracker that answers "torrent not
/// found" for a torrent it is tracking.
pub fn percent_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// A random `u32` without pulling in a random number generator.
///
/// Transaction ids and the announce key only have to be unpredictable enough
/// that two runs do not collide, which the system clock plus the address of a
/// stack local provides.
fn fastrand_u32() -> u32 {
    use std::hash::{BuildHasher, Hasher, RandomState};
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default(),
    );
    hasher.finish() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Announce {
        Announce {
            info_hash: [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
                0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
            ],
            peer_id: *b"-BC0100-abcdefghijkl",
            port: 6881,
            uploaded: 10,
            downloaded: 20,
            left: 30,
            event: Event::Started,
            numwant: 50,
            key: 0xdead_beef,
        }
    }

    #[test]
    fn raw_bytes_are_percent_encoded_not_hex_encoded() {
        assert_eq!(percent_encode(&[0x01, 0x02]), "%01%02");
        assert_eq!(percent_encode(b"abcXYZ019-_.~"), "abcXYZ019-_.~");
        assert_eq!(percent_encode(b"/?&="), "%2F%3F%26%3D");
        assert_eq!(percent_encode(&[0xff]), "%FF");
    }

    #[test]
    fn an_announce_query_carries_every_required_parameter() {
        let query = announce_query("http://t.example/announce", &request());
        assert!(query.starts_with('?'), "{query}");
        for key in [
            "info_hash=",
            "peer_id=",
            "port=6881",
            "uploaded=10",
            "downloaded=20",
            "left=30",
            "compact=1",
            "event=started",
        ] {
            assert!(query.contains(key), "{key} missing from {query}");
        }
    }

    #[test]
    fn an_announce_url_that_already_has_a_query_gets_an_ampersand() {
        let query = announce_query("http://t.example/announce?pk=abc", &request());
        assert!(query.starts_with('&'), "{query}");
    }

    #[test]
    fn a_regular_interval_announce_sends_no_event() {
        let mut request = request();
        request.event = Event::None;
        let query = announce_query("http://t.example/announce", &request);
        assert!(!query.contains("event="), "{query}");
    }

    #[test]
    fn scrape_urls_follow_the_bep_48_convention() {
        assert_eq!(
            scrape_url("http://t.example/announce").as_deref(),
            Some("http://t.example/scrape")
        );
        assert_eq!(
            scrape_url("http://t.example/announce.php").as_deref(),
            Some("http://t.example/scrape.php")
        );
        assert_eq!(
            scrape_url("http://t.example/x/announce?pk=abc").as_deref(),
            Some("http://t.example/x/scrape?pk=abc")
        );
    }

    #[test]
    fn a_tracker_with_no_announce_path_has_no_derivable_scrape_url() {
        // Guessing here would produce a 404 that reads like the tracker being
        // down, which is a worse answer than saying it cannot be derived.
        assert_eq!(scrape_url("http://t.example/track"), None);
        assert_eq!(scrape_url("http://t.example/"), None);
    }

    #[test]
    fn a_normal_announce_response_parses() {
        let body = b"d8:completei12e10:incompletei3e8:intervali1800e12:min intervali900e5:peers6:\x7f\x00\x00\x01\x1a\xe1e";
        let result = parse_http_response(body).unwrap();
        assert!(result.ok);
        assert_eq!(result.seeders, Some(12));
        assert_eq!(result.leechers, Some(3));
        assert_eq!(result.interval_s, Some(1800));
        assert_eq!(result.min_interval_s, Some(900));
        assert_eq!(result.peers, vec!["127.0.0.1:6881"]);
    }

    #[test]
    fn a_failure_reason_is_reported_rather_than_parsed_past() {
        let body = b"d14:failure reason17:torrent not founde";
        let result = parse_http_response(body).unwrap();
        assert!(!result.ok);
        assert_eq!(result.failure.as_deref(), Some("torrent not found"));
        assert!(result.peers.is_empty());
    }

    #[test]
    fn a_warning_message_does_not_make_the_announce_a_failure() {
        let body = b"d15:warning message21:tracker is overloaded8:completei1ee";
        let result = parse_http_response(body).unwrap();
        assert!(result.ok);
        assert_eq!(result.warning.as_deref(), Some("tracker is overloaded"));
        assert_eq!(result.seeders, Some(1));
    }

    #[test]
    fn the_dictionary_peer_form_parses_as_well_as_the_compact_one() {
        let body = b"d5:peersld2:ip9:127.0.0.14:porti6881eeee";
        let result = parse_http_response(body).unwrap();
        assert_eq!(result.peers, vec!["127.0.0.1:6881"]);
    }

    #[test]
    fn ipv6_peers_come_back_bracketed() {
        let mut body = Vec::from(&b"d6:peers618:"[..]);
        body.extend_from_slice(&[0u8; 15]);
        body.push(1);
        body.extend_from_slice(&6881u16.to_be_bytes());
        body.push(b'e');
        let result = parse_http_response(&body).unwrap();
        assert_eq!(result.peers, vec!["[::1]:6881"]);
    }

    #[test]
    fn a_truncated_compact_peer_list_drops_the_partial_entry() {
        // Seven bytes is one peer and one stray byte. The stray byte is not a
        // peer and inventing an address from it would be worse than losing it.
        let body = b"d5:peers7:\x7f\x00\x00\x01\x1a\xe1\x00e";
        let result = parse_http_response(body).unwrap();
        assert_eq!(result.peers, vec!["127.0.0.1:6881"]);
    }

    #[test]
    fn a_scrape_response_is_read_for_the_hash_that_was_asked_for() {
        let hash = request().info_hash;
        let mut body = Vec::from(&b"d5:filesd20:"[..]);
        body.extend_from_slice(&hash);
        body.extend_from_slice(b"d8:completei7e10:downloadedi42e10:incompletei2eeee");
        let result = parse_scrape_response(&body, &hash).unwrap();
        assert!(result.ok);
        assert_eq!(result.seeders, Some(7));
        assert_eq!(result.leechers, Some(2));
        assert_eq!(result.completed, Some(42));
    }

    #[test]
    fn a_scrape_for_an_unknown_hash_says_so() {
        let body = b"d5:filesdee";
        let result = parse_scrape_response(body, &request().info_hash).unwrap();
        assert!(!result.ok);
        assert!(result.failure.unwrap().contains("does not know"));
    }

    #[test]
    fn a_udp_connect_request_carries_the_protocol_magic() {
        let payload = connect_request(0x1234_5678);
        assert_eq!(payload.len(), 16);
        assert_eq!(
            u64::from_be_bytes(payload[0..8].try_into().unwrap()),
            UDP_PROTOCOL_ID
        );
        assert_eq!(
            u32::from_be_bytes(payload[8..12].try_into().unwrap()),
            ACTION_CONNECT
        );
        assert_eq!(
            u32::from_be_bytes(payload[12..16].try_into().unwrap()),
            0x1234_5678
        );
    }

    #[test]
    fn a_udp_announce_request_is_ninety_eight_bytes_in_the_documented_order() {
        let payload = announce_request(0xaabb_ccdd_eeff_0011, 7, &request());
        assert_eq!(payload.len(), 98, "BEP 15 fixes the announce request size");
        assert_eq!(
            u32::from_be_bytes(payload[8..12].try_into().unwrap()),
            ACTION_ANNOUNCE
        );
        assert_eq!(&payload[16..36], &request().info_hash);
        assert_eq!(&payload[36..56], &request().peer_id);
        assert_eq!(
            u64::from_be_bytes(payload[56..64].try_into().unwrap()),
            20,
            "downloaded"
        );
        assert_eq!(
            u64::from_be_bytes(payload[64..72].try_into().unwrap()),
            30,
            "left"
        );
        assert_eq!(
            u64::from_be_bytes(payload[72..80].try_into().unwrap()),
            10,
            "uploaded"
        );
        assert_eq!(
            u32::from_be_bytes(payload[80..84].try_into().unwrap()),
            2,
            "started"
        );
        assert_eq!(
            u16::from_be_bytes(payload[96..98].try_into().unwrap()),
            6881
        );
    }

    #[test]
    fn a_udp_scrape_request_is_thirty_six_bytes() {
        let payload = scrape_request(1, 2, &request().info_hash);
        assert_eq!(payload.len(), 36);
        assert_eq!(
            u32::from_be_bytes(payload[8..12].try_into().unwrap()),
            ACTION_SCRAPE
        );
        assert_eq!(&payload[16..36], &request().info_hash);
    }

    #[test]
    fn udp_event_numbers_follow_bep_15() {
        assert_eq!(Event::None.as_udp(), 0);
        assert_eq!(Event::Completed.as_udp(), 1);
        assert_eq!(Event::Started.as_udp(), 2);
        assert_eq!(Event::Stopped.as_udp(), 3);
    }

    #[test]
    fn a_udp_announce_reply_parses_its_counts_and_peers() {
        let mut reply = Vec::new();
        reply.extend_from_slice(&ACTION_ANNOUNCE.to_be_bytes());
        reply.extend_from_slice(&7u32.to_be_bytes());
        reply.extend_from_slice(&1800u32.to_be_bytes());
        reply.extend_from_slice(&3u32.to_be_bytes());
        reply.extend_from_slice(&12u32.to_be_bytes());
        reply.extend_from_slice(&[127, 0, 0, 1]);
        reply.extend_from_slice(&6881u16.to_be_bytes());

        let result = parse_udp_announce(&reply).unwrap();
        assert_eq!(result.interval_s, Some(1800));
        assert_eq!(result.leechers, Some(3));
        assert_eq!(result.seeders, Some(12));
        assert_eq!(result.peers, vec!["127.0.0.1:6881"]);
    }

    #[test]
    fn a_short_udp_reply_is_an_error_rather_than_zeroes() {
        assert!(parse_udp_announce(&[0; 12]).is_err());
        assert!(parse_udp_scrape(&[0; 12]).is_err());
    }

    #[test]
    fn a_udp_scrape_reply_reads_seeders_completed_leechers_in_that_order() {
        let mut reply = Vec::new();
        reply.extend_from_slice(&ACTION_SCRAPE.to_be_bytes());
        reply.extend_from_slice(&7u32.to_be_bytes());
        reply.extend_from_slice(&12u32.to_be_bytes());
        reply.extend_from_slice(&42u32.to_be_bytes());
        reply.extend_from_slice(&3u32.to_be_bytes());

        let result = parse_udp_scrape(&reply).unwrap();
        assert_eq!(result.seeders, Some(12));
        assert_eq!(result.completed, Some(42));
        assert_eq!(result.leechers, Some(3));
    }

    #[test]
    fn protocols_are_recognised_case_insensitively() {
        assert_eq!(protocol_of("UDP://t.example:451/announce"), "udp");
        assert_eq!(protocol_of("HTTPS://t.example/announce"), "https");
        assert_eq!(protocol_of("http://t.example/announce"), "http");
        assert_eq!(protocol_of("wss://t.example"), "unknown");
    }

    #[test]
    fn a_udp_url_resolves_to_its_authority() {
        let addr = udp_target("udp://127.0.0.1:451/announce", None).unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:451");
        assert!(udp_target("http://t.example/announce", None).is_err());
    }

    /// A family that was asked for and is not there is an error naming it,
    /// not a silent fallback to the other one.
    ///
    /// Falling back would be the worst answer available: the caller asked to
    /// announce over one family, and announcing over the other registers an
    /// address they did not ask to publish and reports it as if they had.
    #[test]
    fn a_udp_url_with_no_address_in_the_family_is_refused() {
        let v4 = udp_target("udp://127.0.0.1:451/announce", Some(Family::V4)).unwrap();
        assert_eq!(v4.to_string(), "127.0.0.1:451");
        let err = udp_target("udp://127.0.0.1:451/announce", Some(Family::V6)).unwrap_err();
        assert!(
            err.to_string().contains("IPv6"),
            "the error should name the family: {err}"
        );
        let v6 = udp_target("udp://[::1]:451/announce", Some(Family::V6)).unwrap();
        assert_eq!(v6.to_string(), "[::1]:451");
    }

    #[test]
    fn an_http_url_splits_into_a_host_and_a_port() {
        assert_eq!(
            http_authority("http://t.example/announce").unwrap(),
            ("t.example".to_string(), 80)
        );
        assert_eq!(
            http_authority("https://t.example/announce").unwrap(),
            ("t.example".to_string(), 443)
        );
        assert_eq!(
            http_authority("http://t.example:6969/announce?x=1").unwrap(),
            ("t.example".to_string(), 6969)
        );
        // Userinfo is not the host.
        assert_eq!(
            http_authority("http://user:pw@t.example:8080/a").unwrap(),
            ("t.example".to_string(), 8080)
        );
        assert!(http_authority("udp://t.example:451").is_err());
    }

    /// An IPv6 literal carries colons of its own, so the port cannot be split
    /// off at the last one.
    #[test]
    fn a_bracketed_ipv6_host_keeps_its_colons() {
        assert_eq!(
            http_authority("http://[::1]:6969/announce").unwrap(),
            ("::1".to_string(), 6969)
        );
        assert_eq!(
            http_authority("http://[2001:db8::1]/announce").unwrap(),
            ("2001:db8::1".to_string(), 80)
        );
        assert!(http_authority("http://[::1:6969/announce").is_err());
    }

    /// Literals need no resolver, so this says the same thing on every host.
    #[test]
    fn a_literal_address_resolves_to_its_own_family_only() {
        assert_eq!(
            families_of("udp://127.0.0.1:451/announce").unwrap(),
            vec![Family::V4]
        );
        assert_eq!(
            families_of("udp://[::1]:451/announce").unwrap(),
            vec![Family::V6]
        );
        assert_eq!(
            families_of("http://127.0.0.1:6969/announce").unwrap(),
            vec![Family::V4]
        );
        assert_eq!(
            families_of("http://[::1]:6969/announce").unwrap(),
            vec![Family::V6]
        );
        assert!(families_of("wss://t.example/announce").is_err());
    }

    #[test]
    fn two_random_keys_differ() {
        // The key only has to be unpredictable enough that two runs do not
        // collide, but zero every time would defeat the point.
        let values: std::collections::HashSet<u32> = (0..8).map(|_| fastrand_u32()).collect();
        assert!(values.len() > 1, "the key generator returned a constant");
    }
}
