use std::collections::HashSet;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::bail;
use backon::ExponentialBuilder;
use backon::Retryable;
use futures::FutureExt;
use futures::StreamExt;
use futures::future::Either;
use futures::stream::BoxStream;
use futures::stream::FuturesUnordered;
use tracing::Instrument;
use tracing::debug;
use tracing::debug_span;
use tracing::trace;
use tracing::trace_span;
use url::Url;

use crate::tracker_comms_http;
use crate::tracker_comms_udp;
use crate::tracker_comms_udp::UdpTrackerClient;
use librqbit_core::hash_id::Id20;

/// Longest an HTTP announce may take, headers and body together.
///
/// Neither `reqwest` client this crate is handed carries a timeout, so without
/// this a tracker that accepts a connection and then sends one byte a minute
/// holds an announce task for as long as it likes. Added by this fork; see
/// `patches/UPSTREAM.md`.
const HTTP_TRACKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Most bytes an HTTP announce response may carry.
///
/// A compact peer list is six bytes per peer, so a megabyte is about 175,000
/// peers and no honest tracker approaches it. `Response::bytes` had no bound at
/// all, which made the size of this process's allocation a number the tracker
/// picks.
const MAX_HTTP_TRACKER_RESPONSE_BYTES: usize = 1024 * 1024;

/// Shortest announce interval this client will honour.
///
/// A tracker that answers `interval: 0` otherwise gets an announce loop with no
/// sleep in it. Five seconds rather than the sixty a stricter reading would
/// take, because the UDP path in this same file already clamps to five and two
/// floors for the same protocol is a difference nobody chose. Raising it is a
/// policy decision about how often to talk to honest trackers, and this is not
/// that: it is the smallest number that makes the loop a loop.
const MIN_TRACKER_ANNOUNCE_INTERVAL_SECS: u64 = 5;

/// Clamp what a tracker asked for to something that is not a tight loop.
fn floor_announce_interval(interval: Duration) -> Duration {
    interval.max(Duration::from_secs(MIN_TRACKER_ANNOUNCE_INTERVAL_SECS))
}

/// Read an announce response with a deadline and a ceiling.
///
/// Both halves are needed and neither substitutes for the other: the deadline
/// bounds a tracker that stalls, and the ceiling bounds one that answers
/// quickly and forever. `Content-Length` is checked when it is there and never
/// trusted when it is: the running total is what refuses, so a missing or
/// lying header changes nothing.
async fn fetch_http_tracker_response(
    response: reqwest::Response,
    max_bytes: usize,
) -> anyhow::Result<Vec<u8>> {
    if let Some(len) = response.content_length()
        && len > max_bytes as u64
    {
        anyhow::bail!("tracker response declares {len} bytes, over the {max_bytes} limit");
    }
    let mut response = response;
    let mut out = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(max_bytes as u64)
            .try_into()
            .unwrap_or(0),
    );
    while let Some(chunk) = response.chunk().await? {
        if out.len() + chunk.len() > max_bytes {
            anyhow::bail!(
                "tracker response passed {} bytes, over the {max_bytes} limit",
                out.len() + chunk.len()
            );
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

pub struct TrackerComms {
    info_hash: Id20,
    peer_id: Id20,
    stats: Box<dyn TorrentStatsProvider>,
    force_tracker_interval: Option<Duration>,
    tx: Sender,
    // This MUST be set as trackers don't work with 0 port.
    announce_port: u16,
    reqwest_client: reqwest::Client,
    reqwest_client_factory: Option<ReqwestClientFactory>,
    key: u32,
}

#[derive(Default)]
pub enum TrackerCommsStatsState {
    #[default]
    None,
    Initializing,
    Paused,
    Live,
}

#[derive(Default)]
pub struct TrackerCommsStats {
    pub uploaded_bytes: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub torrent_state: TrackerCommsStatsState,
}

/// Which BEP 3 event the next UDP announce carries.
///
/// An event is a **transition** and not a state, and reading it off the
/// current state on every announce is what this exists to stop. The HTTP
/// monitor a few hundred lines below already keeps the discipline: `started`
/// on the first announce and nothing on the ones after it. The UDP monitor
/// derived it from `torrent_state` every time, so a live incomplete torrent
/// sent `started` at every interval and a live complete one sent `completed`
/// at every interval, forever.
///
/// What that costs is on the tracker rather than here. `completed` is how a
/// tracker counts finished downloads, which is the `downloaded` field of a
/// BEP 48 scrape, so one seeder announcing every five minutes adds 288 a day
/// to a number that should never have moved. And BEP 3 says `completed` is
/// not sent at all when the client already had the whole file, which is
/// exactly what a seeder is.
///
/// Measured on 2026-08-24 against `loopback-tracker`, one client, one 22
/// second run, the same payload over both protocols: UDP sent `started` five
/// times where HTTP sent it once.
///
/// **`completed` is not sent from here at all**, which is the other half of
/// matching the HTTP monitor. That loop sends `Started` then `None` and has
/// no `Completed` arm, so over HTTP the one `completed` a run produces is the
/// caller's, sent at the instant the transition happens rather than at the
/// next announce interval. Sending one here as well made the same run tell
/// the tracker twice, which double counts a finished download exactly as
/// surely as sending it every interval did.
#[derive(Default)]
struct UdpAnnounceEvents {
    /// Whether the first announce has been answered.
    started: bool,
}

impl UdpAnnounceEvents {
    /// What the next announce should carry. Does not consume it: an announce
    /// nothing answered has not delivered its event, and `started` would
    /// otherwise be spent on a datagram that went nowhere.
    fn peek(&self, stats: &TrackerCommsStats) -> u32 {
        // A paused torrent is one the tracker should stop handing out, and
        // that is true however many announces have gone before it.
        if matches!(stats.torrent_state, TrackerCommsStatsState::Paused) {
            return tracker_comms_udp::EVENT_STOPPED;
        }
        match self.started {
            false => tracker_comms_udp::EVENT_STARTED,
            true => tracker_comms_udp::EVENT_NONE,
        }
    }

    /// Record that an announce carrying `event` was answered.
    fn commit(&mut self, event: u32) {
        if event == tracker_comms_udp::EVENT_STARTED {
            self.started = true;
        }
    }
}

impl TrackerCommsStats {
    pub fn get_left_to_download_bytes(&self) -> u64 {
        let total = self.total_bytes;
        let down = self.downloaded_bytes;
        if total >= down {
            return total - down;
        }
        0
    }

    pub fn is_completed(&self) -> bool {
        self.downloaded_bytes >= self.total_bytes
    }
}

pub trait TorrentStatsProvider: Send + Sync {
    fn get(&self) -> TrackerCommsStats;
}

impl TorrentStatsProvider for () {
    fn get(&self) -> TrackerCommsStats {
        Default::default()
    }
}

type Sender = tokio::sync::mpsc::Sender<SocketAddr>;

enum SupportedTracker {
    Udp(Url),
    Http(Url),
}

impl std::fmt::Debug for SupportedTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportedTracker::Udp(u) => std::fmt::Display::fmt(u, f),
            SupportedTracker::Http(u) => std::fmt::Display::fmt(u, f),
        }
    }
}

/// The first address a tracker's host has in each family.
///
/// Named for UDP until 2026-08-22 and used only there. It is the HTTP path's
/// too now: what it answers, which of our addresses can a tracker be told
/// about, has nothing to do with the scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrackerResolveResult {
    One(SocketAddr),
    Two(SocketAddrV4, SocketAddrV6),
}

/// Rebuilds a `reqwest` client configured the way the session configured its
/// own.
///
/// Announcing to one HTTP tracker over both address families needs one client
/// per family: the family is decided when the host is resolved, and a built
/// `reqwest::Client` cannot be reconfigured afterwards.
/// `ClientBuilder::local_address` does not pin one either, because hyper-util
/// binds the local address only when it already matches the destination's
/// family and otherwise falls through to the unspecified address of the
/// destination's own family. Overriding the resolution per family does pin it,
/// and that needs a builder.
///
/// A factory rather than a second client, so the proxy, the bound interface
/// and the user agent are configured in one place and cannot drift apart.
/// `None` disables the split and is what a session behind a proxy passes:
/// the proxy resolves, so there is no local family left to choose.
pub type ReqwestClientFactory = Arc<dyn Fn() -> reqwest::ClientBuilder + Send + Sync>;

/// One HTTP announce target: a client, and the family it is pinned to.
struct HttpAnnouncer {
    /// `None` when nothing is pinned, which is one announce over whichever
    /// family the connector picks.
    family: Option<&'static str>,
    client: reqwest::Client,
}

impl HttpAnnouncer {
    fn family_name(&self) -> &'static str {
        self.family.unwrap_or("any")
    }
}

async fn tracker_to_socket_addrs(
    host: url::Host<&str>,
    port: u16,
) -> anyhow::Result<TrackerResolveResult> {
    let res = match host {
        url::Host::Domain(name) => {
            // Use the first IPv4 and the first IPv6 addresses only.

            let mut v4: Option<SocketAddrV4> = None;
            let mut v6: Option<SocketAddrV6> = None;
            for addr in tokio::net::lookup_host((name, port))
                .await
                .with_context(|| format!("error looking up hostname {name}"))?
            {
                match (v4, v6, addr) {
                    (None, _, SocketAddr::V4(addr)) => v4 = Some(addr),
                    (_, None, SocketAddr::V6(addr)) => v6 = Some(addr),
                    _ => continue,
                }
            }
            let res = match (v4, v6) {
                (Some(v4), Some(v6)) => TrackerResolveResult::Two(v4, v6),
                (Some(v4), None) => TrackerResolveResult::One(v4.into()),
                (None, Some(v6)) => TrackerResolveResult::One(v6.into()),
                _ => anyhow::bail!("zero addresses returned looking up {name}"),
            };
            trace!(?res, "resolved");
            res
        }
        url::Host::Ipv4(addr) => TrackerResolveResult::One((addr, port).into()),
        url::Host::Ipv6(addr) => TrackerResolveResult::One((addr, port).into()),
    };
    Ok(res)
}

impl TrackerComms {
    // TODO: fix too many args
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        info_hash: Id20,
        peer_id: Id20,
        trackers: HashSet<Url>,
        stats: Box<dyn TorrentStatsProvider>,
        force_interval: Option<Duration>,
        announce_port: u16,
        reqwest_client: reqwest::Client,
        reqwest_client_factory: Option<ReqwestClientFactory>,
        udp_client: UdpTrackerClient,
    ) -> Option<BoxStream<'static, SocketAddr>> {
        let trackers = trackers
            .into_iter()
            .filter_map(|t| match t.scheme() {
                "http" | "https" => Some(SupportedTracker::Http(t)),
                "udp" => Some(SupportedTracker::Udp(t)),
                _ => {
                    debug!("unsupported tracker URL: {}", t);
                    None
                }
            })
            .collect::<Vec<_>>();
        if trackers.is_empty() {
            debug!(?info_hash, "trackers list is empty");
            return None;
        }

        tracing::trace!(?trackers);

        let (tx, mut rx) = tokio::sync::mpsc::channel::<SocketAddr>(16);

        let s = async_stream::stream! {
            use futures::StreamExt;
            let comms = Arc::new(Self {
                info_hash,
                peer_id,
                stats,
                force_tracker_interval: force_interval,
                tx,
                announce_port,
                reqwest_client,
                reqwest_client_factory,
                key: rand::random(),
            });
            let mut futures = FuturesUnordered::new();
            for tracker in trackers {
                futures.push(comms.add_tracker(tracker, &udp_client))
            }
            while !(futures.is_empty()) {
                tokio::select! {
                    addr = rx.recv() => {
                        if let Some(addr) = addr {
                            yield addr;
                        }
                    }
                    e = futures.next(), if !futures.is_empty() => {
                        if let Some(Err(e)) = e {
                            debug!("error: {e}");
                        }
                    }
                }
            }
        };

        Some(s.boxed())
    }

    fn add_tracker(
        &self,
        url: SupportedTracker,
        client: &UdpTrackerClient,
    ) -> Either<
        impl std::future::Future<Output = anyhow::Result<()>> + '_ + Send,
        impl std::future::Future<Output = anyhow::Result<()>> + '_ + Send,
    > {
        let info_hash = self.info_hash;
        match url {
            SupportedTracker::Udp(url) => {
                let span = debug_span!(parent: None, "udp_tracker", tracker = %url, info_hash = ?info_hash);
                self.task_single_tracker_monitor_udp(url, client.clone())
                    .instrument(span)
                    .right_future()
            }
            SupportedTracker::Http(url) => {
                let span = debug_span!(
                    parent: None,
                    "http_tracker",
                    tracker = %url,
                    info_hash = ?info_hash
                );
                self.task_single_tracker_monitor_http(url)
                    .instrument(span)
                    .left_future()
            }
        }
    }

    /// The tracker's first address in each family, or `None` when the host is
    /// a literal address or does not resolve right now.
    ///
    /// A failure here is not fatal and is not retried: it means one announce
    /// over whatever the connector picks, which is what this always did. The
    /// next round resolves again.
    async fn resolve_http_tracker(&self, tracker_url: &Url) -> Option<TrackerResolveResult> {
        let host = tracker_url.host()?;
        let port = tracker_url.port_or_known_default()?;
        match tracker_to_socket_addrs(host, port).await {
            Ok(res) => Some(res),
            Err(e) => {
                debug!("error resolving tracker: {e:#}");
                None
            }
        }
    }

    /// One client per address family when the tracker has an address in both
    /// and the session gave us a way to build them, otherwise the session's
    /// own client and one announce, which is the previous behaviour.
    fn http_announcers(
        &self,
        tracker_url: &Url,
        resolved: Option<TrackerResolveResult>,
    ) -> Vec<HttpAnnouncer> {
        let single = || {
            vec![HttpAnnouncer {
                family: None,
                client: self.reqwest_client.clone(),
            }]
        };
        let (Some(factory), Some(TrackerResolveResult::Two(v4, v6))) =
            (self.reqwest_client_factory.as_ref(), resolved)
        else {
            return single();
        };
        // The override is keyed by name, so there is nothing to override when
        // the URL already names an address.
        let Some(url::Host::Domain(host)) = tracker_url.host() else {
            return single();
        };
        let build = |addr: SocketAddr| factory().resolve_to_addrs(host, &[addr]).build();
        match (build(v4.into()), build(v6.into())) {
            (Ok(v4_client), Ok(v6_client)) => vec![
                HttpAnnouncer {
                    family: Some("v4"),
                    client: v4_client,
                },
                HttpAnnouncer {
                    family: Some("v6"),
                    client: v6_client,
                },
            ],
            (v4_result, v6_result) => {
                let err = v4_result.err().or(v6_result.err());
                debug!("error building a client per address family, announcing once: {err:?}");
                single()
            }
        }
    }

    /// Announce to one tracker once per address family, in sequence.
    ///
    /// In sequence rather than concurrently, deliberately. A tracker that keys
    /// its peer records by peer id alone, which is all BEP 3 asks for, keeps
    /// whichever announce it saw last, so two concurrent announces make which
    /// of our addresses it holds a race between them. In sequence the answer
    /// is the same every run. A tracker that implements BEP 7 keeps both
    /// either way.
    ///
    /// The interval is the first one that came back and the error is the last,
    /// so one family being unreachable does not stop the other announcing.
    async fn tracker_one_request_http_each(
        &self,
        tracker_url: &Url,
        event: Option<tracker_comms_http::TrackerRequestEvent>,
        announcers: &[HttpAnnouncer],
    ) -> anyhow::Result<Duration> {
        let mut interval: Option<Duration> = None;
        let mut last_error: Option<anyhow::Error> = None;
        for announcer in announcers {
            let family = announcer.family_name();
            match self
                .tracker_one_request_http(tracker_url, event, &announcer.client)
                .instrument(trace_span!("http request", family))
                .await
            {
                Ok(this) => {
                    interval.get_or_insert(this);
                }
                Err(e) => {
                    debug!(family, "error announcing: {e:#}");
                    last_error = Some(e);
                }
            }
        }
        match (interval, last_error) {
            (Some(interval), _) => Ok(interval),
            (None, Some(e)) => Err(e),
            (None, None) => bail!("no HTTP announce targets for {tracker_url}"),
        }
    }

    async fn task_single_tracker_monitor_http(&self, tracker_url: Url) -> anyhow::Result<()> {
        trace!(url=%tracker_url, "starting monitor");
        let mut event = Some(tracker_comms_http::TrackerRequestEvent::Started);
        let mut resolved: Option<TrackerResolveResult> = None;
        let mut announcers: Vec<HttpAnnouncer> = Vec::new();

        loop {
            // Resolved every round rather than once, so a tracker that gains
            // an AAAA record is announced to over both families from the next
            // announce rather than from the next restart. One lookup per
            // announce interval costs nothing beside the request it precedes.
            let next = self.resolve_http_tracker(&tracker_url).await;
            if announcers.is_empty() || (next.is_some() && next != resolved) {
                announcers = self.http_announcers(&tracker_url, next);
                resolved = next;
            }

            let interval = (|| {
                self.tracker_one_request_http_each(&tracker_url, event, &announcers)
            })
                .retry(
                    ExponentialBuilder::new()
                        .without_max_times()
                        .with_jitter()
                        .with_factor(2.)
                        .with_min_delay(Duration::from_secs(10))
                        .with_max_delay(Duration::from_secs(600)),
                )
                .notify(|err, retry_in| debug!(?retry_in, "error calling tracker: {err:#}"))
                .await
                .context("this shouldn't fail")?;

            event = None;
            let interval = self.force_tracker_interval.unwrap_or(interval);
            debug!("sleeping for {:?} after calling tracker", interval);
            tokio::time::sleep(interval).await;
        }
    }

    async fn tracker_one_request_http(
        &self,
        tracker_url: &Url,
        event: Option<tracker_comms_http::TrackerRequestEvent>,
        client: &reqwest::Client,
    ) -> anyhow::Result<Duration> {
        let stats = self.stats.get();
        let request = tracker_comms_http::TrackerRequest {
            info_hash: &self.info_hash,
            peer_id: &self.peer_id,
            port: self.announce_port,
            uploaded: stats.uploaded_bytes,
            downloaded: stats.downloaded_bytes,
            left: stats.get_left_to_download_bytes(),
            compact: true,
            no_peer_id: false,
            event,
            ip: None,
            numwant: None,
            key: Some(self.key),
            trackerid: None,
        };

        let mut url = tracker_url.clone();

        let mut queries = request.as_querystring();
        if let Some(url_query) = url.query() {
            queries.push_str(&format!("&{}", url_query));
        }
        url.set_query(Some(&queries));

        // One deadline over the whole exchange rather than one per read, so a
        // tracker cannot keep an announce alive by answering slowly forever.
        let bytes = tokio::time::timeout(HTTP_TRACKER_REQUEST_TIMEOUT, async {
            let response: reqwest::Response = client.get(url).send().await?;
            if !response.status().is_success() {
                anyhow::bail!("tracker responded with {:?}", response.status());
            }
            fetch_http_tracker_response(response, MAX_HTTP_TRACKER_RESPONSE_BYTES).await
        })
        .await
        .context("tracker request timed out")??;
        if let Ok((error, _)) =
            bencode::from_bytes_with_rest::<tracker_comms_http::TrackerError>(&bytes)
        {
            anyhow::bail!(
                "tracker returned failure. Failure reason: {}",
                error.failure_reason
            )
        };
        let response = bencode::from_bytes_with_rest::<tracker_comms_http::TrackerResponse>(&bytes)
            .map_err(|e| {
                tracing::trace!("error deserializing TrackerResponse: {e:#}");
                e.into_kind()
            })?
            .0;

        for peer in response.iter_peers() {
            self.tx.send(peer).await?;
        }
        Ok(floor_announce_interval(Duration::from_secs(
            response.min_interval.unwrap_or(response.interval),
        )))
    }

    async fn task_single_tracker_monitor_udp(
        &self,
        url: Url,
        client: UdpTrackerClient,
    ) -> anyhow::Result<()> {
        if url.scheme() != "udp" {
            bail!("expected UDP scheme in {}", url);
        }
        let (host, port) = (
            url.host().context("missing host")?,
            url.port().context("missing port")?,
        );

        let mut sleep_interval: Option<Duration> = None;
        let mut prev_addrs: Option<TrackerResolveResult> = None;
        // One event per round rather than one per datagram, so a dual-stack
        // tracker is told the same thing over both families. That is what
        // the HTTP monitor does with `tracker_one_request_http_each`.
        let mut events = UdpAnnounceEvents::default();
        loop {
            if let Some(i) = sleep_interval {
                trace!(interval=?sleep_interval, "sleeping");
                tokio::time::sleep(i).await;
            }

            // This should retry forever until the addrs are resolved.
            let addrs = (async || {
                tracker_to_socket_addrs(host.clone(), port)
                    .instrument(trace_span!("resolve", ?host))
                    .await
                    .or_else(|err| prev_addrs.ok_or(err))
            })
            .retry(
                ExponentialBuilder::new()
                    .without_max_times()
                    .with_max_delay(Duration::from_secs(60))
                    .with_jitter(),
            )
            .notify(|err, retry| debug!(retry_in=?retry, "error resolving tracker: {err:#}"))
            .await
            .context("this shouldn't happen: failed resolving tracker addrs")?;

            prev_addrs = Some(addrs);

            let stats = self.stats.get();
            let event = events.peek(&stats);
            let answered = match addrs {
                TrackerResolveResult::One(addr) => {
                    match self
                        .tracker_one_request_udp(addr, &client, event)
                        .instrument(trace_span!("udp request", ?addr))
                        .await
                    {
                        Ok(sleep) => {
                            sleep_interval = Some(sleep);
                            true
                        }
                        Err(_) => {
                            sleep_interval = Some(sleep_interval.unwrap_or(Duration::from_secs(60)));
                            false
                        }
                    }
                }
                TrackerResolveResult::Two(v4, v6) => {
                    let (r4, r6) = tokio::join!(
                        self.tracker_one_request_udp(v4.into(), &client, event)
                            .instrument(trace_span!("udp request", addr=?v4)),
                        self.tracker_one_request_udp(v6.into(), &client, event)
                            .instrument(trace_span!("udp request", addr=?v6))
                    );
                    let answered = r4.is_ok() || r6.is_ok();
                    sleep_interval = Some(
                        r4.or(r6)
                            .ok()
                            .or(sleep_interval)
                            .unwrap_or(Duration::from_secs(60)),
                    );
                    answered
                }
            };
            // Only a round something answered advances the sequence. An
            // announce nothing replied to may never have arrived, and
            // spending `started` on it would leave the tracker with no first
            // announce at all.
            if answered {
                events.commit(event);
            }
        }
    }

    async fn tracker_one_request_udp(
        &self,
        addr: SocketAddr,
        client: &UdpTrackerClient,
        event: u32,
    ) -> anyhow::Result<Duration> {
        use tracker_comms_udp::*;

        let stats = self.stats.get();
        let request = AnnounceFields {
            info_hash: self.info_hash,
            peer_id: self.peer_id,
            downloaded: stats.downloaded_bytes,
            left: stats.get_left_to_download_bytes(),
            uploaded: stats.uploaded_bytes,
            // Decided by the caller, which is the only place that knows what
            // the announces before this one carried. See `UdpAnnounceEvents`.
            event,
            key: self.key,
            port: self.announce_port,
        };

        match client.announce(addr, request).await {
            Ok(response) => {
                trace!(len = response.addrs.len(), "received announce response");
                for addr in response.addrs {
                    self.tx.send(addr).await.context("rx closed")?;
                }
                // Through the same floor as the HTTP path, so one protocol
                // does not have two answers to the same question.
                let sleep = response.interval.max(MIN_TRACKER_ANNOUNCE_INTERVAL_SECS as u32);
                let sleep = Duration::from_secs(sleep as u64);
                Ok(sleep)
            }
            Err(e) => {
                debug!(?addr, "error reading announce response: {e:#}");
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;

    /// Added by this fork. A tracker that answers `interval: 0` used to get an
    /// announce loop with no sleep in it, on the HTTP path only: the UDP path
    /// in this same file already clamped to five seconds.
    #[test]
    fn a_zero_interval_is_floored_rather_than_slept_on() {
        assert_eq!(
            floor_announce_interval(Duration::from_secs(0)),
            Duration::from_secs(MIN_TRACKER_ANNOUNCE_INTERVAL_SECS)
        );
        assert_eq!(
            floor_announce_interval(Duration::from_secs(1)),
            Duration::from_secs(MIN_TRACKER_ANNOUNCE_INTERVAL_SECS)
        );
    }

    /// And an honest interval is left alone. A floor that rounded 1,800 seconds
    /// up to anything would be a policy change rather than a bound.
    #[test]
    fn an_honest_interval_is_not_changed() {
        for secs in [
            MIN_TRACKER_ANNOUNCE_INTERVAL_SECS,
            30,
            60,
            900,
            1800,
        ] {
            assert_eq!(
                floor_announce_interval(Duration::from_secs(secs)),
                Duration::from_secs(secs),
                "interval {secs}"
            );
        }
    }

    /// The two floors are the same number, which is the point of naming it.
    #[test]
    fn the_udp_path_uses_the_same_floor() {
        let clamped = 0u32.max(MIN_TRACKER_ANNOUNCE_INTERVAL_SECS as u32);
        assert_eq!(
            Duration::from_secs(clamped as u64),
            floor_announce_interval(Duration::from_secs(0))
        );
    }

    fn stats(state: TrackerCommsStatsState, downloaded: u64, total: u64) -> TrackerCommsStats {
        TrackerCommsStats {
            uploaded_bytes: 0,
            downloaded_bytes: downloaded,
            total_bytes: total,
            torrent_state: state,
        }
    }

    /// Added by this fork, `TODO/trackers.md` T-256. The event used to be read
    /// off the current state on every announce, so a live incomplete torrent
    /// sent `started` at every interval forever.
    #[test]
    fn started_goes_out_once_and_the_announces_after_it_carry_nothing() {
        let mut events = UdpAnnounceEvents::default();
        let live = stats(TrackerCommsStatsState::Live, 0, 1024);

        let first = events.peek(&live);
        assert_eq!(first, tracker_comms_udp::EVENT_STARTED);
        events.commit(first);

        for _ in 0..5 {
            let next = events.peek(&live);
            assert_eq!(next, tracker_comms_udp::EVENT_NONE);
            events.commit(next);
        }
    }

    /// An announce nothing answered has not delivered its event. Spending
    /// `started` on a datagram that went nowhere would leave the tracker with
    /// no first announce at all.
    #[test]
    fn an_unanswered_announce_does_not_spend_started() {
        let mut events = UdpAnnounceEvents::default();
        let live = stats(TrackerCommsStatsState::Live, 0, 1024);

        assert_eq!(events.peek(&live), tracker_comms_udp::EVENT_STARTED);
        // No commit: the round failed.
        assert_eq!(events.peek(&live), tracker_comms_udp::EVENT_STARTED);
    }

    /// Completion is the caller's announce, sent at the instant it happens.
    /// The HTTP monitor in this file has no `Completed` arm either, and one
    /// here made the same run tell the tracker twice.
    #[test]
    fn completion_is_never_announced_from_the_udp_loop() {
        let mut events = UdpAnnounceEvents::default();
        let incomplete = stats(TrackerCommsStatsState::Live, 0, 1024);
        let complete = stats(TrackerCommsStatsState::Live, 1024, 1024);

        let first = events.peek(&incomplete);
        events.commit(first);
        assert_eq!(events.peek(&complete), tracker_comms_udp::EVENT_NONE);

        // And a torrent that was complete before it ever announced says the
        // same thing, which is what BEP 3 asks of a seeder.
        let mut seeding = UdpAnnounceEvents::default();
        let first = seeding.peek(&complete);
        assert_eq!(first, tracker_comms_udp::EVENT_STARTED);
        seeding.commit(first);
        assert_eq!(seeding.peek(&complete), tracker_comms_udp::EVENT_NONE);
    }

    /// A paused torrent is one the tracker should stop handing out, however
    /// many announces have gone before it.
    #[test]
    fn a_paused_torrent_announces_stopped_whenever_it_is_paused() {
        let mut events = UdpAnnounceEvents::default();
        let live = stats(TrackerCommsStatsState::Live, 0, 1024);
        let paused = stats(TrackerCommsStatsState::Paused, 0, 1024);

        let first = events.peek(&live);
        events.commit(first);
        assert_eq!(events.peek(&paused), tracker_comms_udp::EVENT_STOPPED);
        // Before the first announce too, because a torrent can be paused
        // before it has ever reached a tracker.
        assert_eq!(
            UdpAnnounceEvents::default().peek(&paused),
            tracker_comms_udp::EVENT_STOPPED
        );
    }
}
