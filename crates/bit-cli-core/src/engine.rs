//! The torrent engine: a `librqbit` session wrapped for one-shot commands.
//!
//! Every `bit-cli` verb runs in the foreground, does its work, and exits.
//! There is no daemon and no stored session, so the engine owns a session for
//! the length of one invocation and nothing outlives the process. That is what
//! keeps this module small: no persistence, no restore, no id stability across
//! runs.
//!
//! Everything `librqbit` hands back is translated into the plain types in this
//! module before it leaves. A command never sees a `librqbit` type, which is
//! what lets the rendering layer be written once and stay stable if the engine
//! underneath changes.

use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use librqbit::api::TorrentIdOrHash;
use librqbit::http_api_types::PeerStatsFilter;
use librqbit::limits::LimitsConfig;
use librqbit::storage::StorageFactoryExt;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, Api, DhtSessionConfig, ListenerOptions,
    ManagedTorrent, Session, SessionOptions, TorrentStats, TorrentStatsState,
};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::layout::Layout;
use crate::paths::PathPlan;
use crate::storage::{PlanHandle, SafeStorageFactory};
use crate::torrent::InfoHash;

/// How the session is configured for one run.
#[derive(Debug, Clone)]
pub struct EngineOptions {
    /// Where payloads are written.
    pub download_directory: PathBuf,
    /// Inclusive port range to try for incoming peer connections.
    pub listen_ports: std::ops::RangeInclusive<u16>,
    /// Bind the peer listener to this address only.
    ///
    /// `None` binds the wildcard address, which is what a real run wants:
    /// peers have to be able to reach it. Setting it to loopback confines the
    /// session to the machine, which is what a test wants. It also keeps a
    /// host firewall quiet, because a loopback-only listener is not an
    /// incoming connection as far as Windows Firewall is concerned.
    pub listen_ip: Option<IpAddr>,
    /// Use the DHT.
    pub enable_dht: bool,
    /// Use local service discovery.
    pub enable_lsd: bool,
    /// Announce to trackers.
    pub enable_trackers: bool,
    /// Accept incoming peer connections at all. `false` still binds a port,
    /// because the web seed bridge needs one, but disables discovery.
    pub enable_peers: bool,
    /// Peer connections per torrent.
    pub max_peers: Option<usize>,
    /// Download rate cap in bytes per second.
    pub download_rate: Option<u64>,
    /// Upload rate cap in bytes per second.
    pub upload_rate: Option<u64>,
    /// Trackers added to every torrent in this run.
    pub extra_trackers: Vec<String>,
    /// Restrict to IPv4.
    pub ipv4_only: bool,
    /// The client name announced to peers and trackers.
    pub client_name: Option<String>,
    /// How space is reserved for each payload file.
    pub allocation: crate::alloc::Allocation,
    /// How many payload files stay open at once. Zero means the default.
    pub max_open_files: usize,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            download_directory: PathBuf::from("."),
            listen_ports: 6881..=6889,
            listen_ip: None,
            enable_dht: true,
            enable_lsd: true,
            enable_trackers: true,
            enable_peers: true,
            max_peers: None,
            download_rate: None,
            upload_rate: None,
            extra_trackers: Vec::new(),
            ipv4_only: false,
            client_name: Some(format!("bit-cli {}", crate::VERSION)),
            allocation: crate::alloc::Allocation::default(),
            max_open_files: crate::storage::DEFAULT_MAX_OPEN_FILES,
        }
    }
}

/// How one torrent is added.
#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    /// Start paused.
    pub paused: bool,
    /// Write here instead of the session default.
    pub output_folder: Option<String>,
    /// Only these file indices.
    pub only_files: Option<Vec<usize>>,
    /// Write on top of existing files. Required to resume or to seed.
    pub overwrite: bool,
    /// Read the metadata and stop, without starting the torrent.
    pub list_only: bool,
    /// Trackers for this torrent only.
    pub trackers: Option<Vec<String>>,
    /// Skip tracker announces for this torrent.
    pub disable_trackers: bool,
    /// Override the announce interval.
    pub tracker_interval: Option<Duration>,
    /// Peers to try before any are discovered.
    pub initial_peers: Vec<SocketAddr>,
    /// Peer connections for this torrent.
    pub peer_limit: Option<usize>,
}

/// Coarse state of one torrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Reading the metadata or hash-checking existing data.
    Initializing,
    /// Connected and transferring.
    Live,
    /// Stopped on request.
    Paused,
    /// Stopped by a failure. The failure is in `error`.
    Error,
}

impl State {
    /// The stable name used in JSON and text output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initializing => "initializing",
            Self::Live => "live",
            Self::Paused => "paused",
            Self::Error => "error",
        }
    }
}

/// How many peers are in each state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerCounts {
    /// Connected and usable.
    pub live: u32,
    /// Connecting right now.
    pub connecting: u32,
    /// Known but not yet tried.
    pub queued: u32,
    /// Seen at any point in this run.
    pub seen: u32,
    /// Tried and given up on.
    pub dead: u32,
}

/// One torrent, as every command reports it.
///
/// Byte counts and durations are raw integers. A formatted string may sit
/// beside one in the rendering layer, but never instead of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentSnapshot {
    /// Position in this run. Not stable across runs, by design: there is no
    /// stored session for an id to be stable against.
    pub id: usize,
    pub info_hash: String,
    pub name: String,
    pub state: State,
    pub total_bytes: u64,
    pub progress_bytes: u64,
    pub uploaded_bytes: u64,
    pub finished: bool,
    pub download_rate: u64,
    pub upload_rate: u64,
    /// Estimated time to completion. An estimate, which is why the name says
    /// so and why `eta_confidence` sits beside it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_ms: Option<u64>,
    /// How much the estimate is worth: `none`, `low`, or `measured`.
    pub eta_confidence: &'static str,
    pub peers: PeerCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl TorrentSnapshot {
    /// Progress as a fraction in `0.0..=1.0`.
    pub fn fraction(&self) -> f64 {
        match self.total_bytes {
            0 => 0.0,
            total => (self.progress_bytes as f64 / total as f64).clamp(0.0, 1.0),
        }
    }

    /// Uploaded over downloaded. Zero when nothing has been downloaded.
    pub fn ratio(&self) -> f64 {
        match self.progress_bytes {
            0 => 0.0,
            progress => self.uploaded_bytes as f64 / progress as f64,
        }
    }
}

/// One peer, with the accounting a seeding operator needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSnapshot {
    pub addr: String,
    /// `live`, `connecting`, `queued`, `dead`, or `not needed`.
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    /// `tcp`, `utp`, or `socks`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    /// `incoming` when the peer dialled us, `outgoing` when we dialled it.
    pub direction: &'static str,
    /// Bytes this peer sent us.
    pub downloaded_bytes: u64,
    /// Bytes we sent this peer. The number that answers "is my server
    /// actually serving".
    pub uploaded_bytes: u64,
    /// Pieces received from this peer and verified.
    pub verified_pieces: u32,
    /// Blocks received from this peer.
    pub chunks: u32,
    pub errors: u32,
    /// Total time spent establishing connections to this peer.
    pub connect_ms: u64,
    /// Mean time to download one piece from this peer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean_piece_ms: Option<u64>,
    /// Whether this is one of our own web seed bridges rather than a swarm
    /// member. A bridge is not a peer and must never be counted as one.
    pub web_seed: bool,
}

/// A running torrent.
///
/// Re-exported so a caller can name the type without depending on `librqbit`
/// directly. Everything useful about it is reachable through [`Engine`].
pub type Handle = Arc<ManagedTorrent>;

/// The session, for the length of one invocation.
pub struct Engine {
    session: Arc<Session>,
    api: Api,
    listen_addr: Option<SocketAddr>,
    warnings: Vec<String>,
    download_directory: PathBuf,
    /// One path plan per added torrent, by torrent id. A torrent's files are
    /// not written where the metainfo says when the metainfo says something
    /// the filesystem cannot do, and this is where the caller reads what
    /// happened instead.
    plans: Mutex<HashMap<usize, PlanHandle>>,
    allocation: crate::alloc::Allocation,
    max_open_files: usize,
    /// What storage needed the caller to know, gathered across every torrent.
    storage_notes: Mutex<Vec<Arc<Mutex<Vec<String>>>>>,
}

impl Engine {
    /// Start a session.
    pub async fn start(options: &EngineOptions) -> Result<Self> {
        let (listen_addr, listen_warning) = match options.listen_ip {
            Some(ip) => (bind_on(ip, &options.listen_ports), None),
            None => resolve_listen_addr(&options.listen_ports),
        };
        let mut warnings = Vec::new();
        warnings.extend(listen_warning);

        let trackers = options
            .extra_trackers
            .iter()
            .filter_map(|t| url::Url::parse(t).ok())
            .collect();

        let opts = SessionOptions {
            dht: options.enable_dht.then(DhtSessionConfig::default),
            disable_trackers: !options.enable_trackers,
            disable_local_service_discovery: !options.enable_lsd,
            // No persistence, ever. A stored session is Phase C, and writing
            // one from a foreground command would leave state behind that
            // nothing in this process will read back.
            persistence: None,
            listen: Some(ListenerOptions {
                listen_addr,
                ipv4_only: options.ipv4_only,
                ..Default::default()
            }),
            ratelimits: LimitsConfig {
                download_bps: rate_to_bps(options.download_rate),
                upload_bps: rate_to_bps(options.upload_rate),
            },
            trackers,
            peer_limit: options.max_peers,
            ipv4_only: options.ipv4_only,
            client_name_and_version: options.client_name.clone(),
            ..Default::default()
        };

        let session = Session::new_with_opts(options.download_directory.clone(), opts)
            .await
            .map_err(|e| {
                Error::generic(format!("cannot start the torrent session: {e}")).with(
                    "download_directory",
                    options.download_directory.display().to_string(),
                )
            })?;
        let api = Api::new(session.clone(), None);
        let listen_addr = session.listen_addr();
        if listen_addr.is_none() {
            warnings.push(
                "no peer port was bound, so incoming connections and web seed bridges are unavailable"
                    .to_string(),
            );
        }

        Ok(Self {
            session,
            api,
            listen_addr,
            warnings,
            download_directory: options.download_directory.clone(),
            plans: Mutex::new(HashMap::new()),
            allocation: options.allocation,
            max_open_files: options.max_open_files,
            storage_notes: Mutex::new(Vec::new()),
        })
    }

    /// Non-fatal problems found while starting.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// What storage needed the caller to know, across every torrent added.
    ///
    /// Storage cannot report to a stream itself: it runs on the session's
    /// threads and the streams belong to the caller. So it collects, and the
    /// caller reads this when it is ready to report. The only thing that
    /// appears here today is an allocation method that could not be used, with
    /// what ran instead.
    pub fn storage_notes(&self) -> Vec<String> {
        let Ok(handles) = self.storage_notes.lock() else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for handle in handles.iter() {
            let Ok(notes) = handle.lock() else { continue };
            for note in notes.iter() {
                if !out.contains(note) {
                    out.push(note.clone());
                }
            }
        }
        out
    }

    /// The address incoming peer connections arrive on.
    pub fn listen_addr(&self) -> Option<SocketAddr> {
        self.listen_addr
    }

    /// Where a web seed bridge should dial to reach this session.
    pub fn bridge_target(&self) -> Option<SocketAddr> {
        self.listen_addr.map(loopback_target)
    }

    /// The underlying session, for the few callers that need it.
    pub fn session(&self) -> &Arc<Session> {
        &self.session
    }

    /// Add a torrent and return its handle.
    ///
    /// `source` is anything `librqbit` accepts on a command line: a path, a
    /// URL, a magnet, or a bare info hash.
    pub async fn add(&self, source: &str, options: &AddOptions) -> Result<Arc<ManagedTorrent>> {
        let response = self.add_inner(source, options).await?;
        response.into_handle().ok_or_else(|| {
            Error::source_resolution(format!("{source}: the torrent was listed but not added"))
        })
    }

    /// Where this torrent's files are actually written.
    ///
    /// A torrent path that cannot exist on the filesystem, or that would leave
    /// the output directory, is rewritten before anything is opened. The plan
    /// records every such change with the reason. `None` until the metadata
    /// has resolved and storage has been created, and
    /// [`PathPlan::is_clean`] for the ordinary torrent that needed nothing.
    pub fn path_plan(&self, handle: &ManagedTorrent) -> Option<PathPlan> {
        let plans = self.plans.lock().ok()?;
        plans.get(&handle.id())?.get().cloned()
    }

    /// Read a torrent's metadata without starting it.
    ///
    /// This resolves a magnet against the swarm, which is the one way to turn
    /// a magnet into a layout.
    pub async fn resolve(&self, source: &str) -> Result<ResolvedTorrent> {
        let options = AddOptions {
            list_only: true,
            ..Default::default()
        };
        match self.add_inner(source, &options).await? {
            AddTorrentResponse::ListOnly(list) => {
                let name = list
                    .info
                    .name()
                    .map(|n| n.into_owned())
                    .unwrap_or_else(|| list.info_hash.as_string());
                let multi_file = list.info.info().files.is_some();
                let files = list
                    .info
                    .iter_file_details()
                    .map(|f| (join_components(f.filename.iter_components()), f.len))
                    .collect::<Vec<_>>();
                let layout = Layout::from_lengths(
                    name,
                    multi_file,
                    list.info.lengths().default_piece_length(),
                    files,
                );
                Ok(ResolvedTorrent {
                    info_hash: InfoHash(list.info_hash.0),
                    layout,
                    torrent_bytes: list.torrent_bytes.to_vec(),
                })
            }
            _ => Err(Error::source_resolution(format!(
                "{source}: the torrent started instead of being listed"
            ))),
        }
    }

    async fn add_inner(&self, source: &str, options: &AddOptions) -> Result<AddTorrentResponse> {
        let add = AddTorrent::from_cli_argument(source).map_err(|e| {
            Error::source_resolution(format!("{source}: {e}")).with("source", source.to_string())
        })?;
        // A torrent's own file names decide where its bytes go, and a torrent
        // is untrusted input. The session's default storage joins those names
        // onto the output directory as given, which on Windows is enough to
        // leave it. This factory plans safe paths first. See `crate::storage`.
        // A caller that named an output directory gets exactly that directory.
        // Otherwise the session's rule applies and a multi-file torrent goes
        // into a directory named after itself, which the factory reproduces.
        let (output_folder, subfolder) = match &options.output_folder {
            Some(folder) => (PathBuf::from(folder), false),
            None => (self.download_directory.clone(), true),
        };
        let storage = SafeStorageFactory::new(output_folder, options.overwrite, subfolder)
            .with_allocation(self.allocation)
            .with_max_open_files(self.max_open_files);
        let plan = storage.plan_handle();
        if let Ok(mut notes) = self.storage_notes.lock() {
            notes.push(storage.notes_handle());
        }
        let opts = AddTorrentOptions {
            paused: options.paused,
            output_folder: options.output_folder.clone(),
            only_files: options.only_files.clone(),
            overwrite: options.overwrite,
            list_only: options.list_only,
            trackers: options.trackers.clone(),
            disable_trackers: options.disable_trackers,
            force_tracker_interval: options.tracker_interval,
            initial_peers: (!options.initial_peers.is_empty())
                .then(|| options.initial_peers.clone()),
            peer_limit: options.peer_limit,
            storage_factory: Some(storage.boxed()),
            ..Default::default()
        };
        let response = self
            .session
            .add_torrent(add, Some(opts))
            .await
            .map_err(|e| classify_add_error(source, &e))?;
        if let AddTorrentResponse::Added(id, _) | AddTorrentResponse::AlreadyManaged(id, _) =
            &response
            && let Ok(mut plans) = self.plans.lock()
        {
            plans.insert(*id, plan);
        }
        Ok(response)
    }

    /// A snapshot of one torrent. No I/O.
    pub fn snapshot(&self, handle: &ManagedTorrent) -> TorrentSnapshot {
        let stats = handle.stats();
        let (download_rate, upload_rate, peers) = live_rates(&stats);
        let eta = handle
            .live()
            .and_then(|l| l.down_speed_estimator().time_remaining());
        TorrentSnapshot {
            id: handle.id(),
            info_hash: handle.info_hash().as_string(),
            name: handle
                .name()
                .unwrap_or_else(|| handle.info_hash().as_string()),
            state: to_state(&stats.state),
            total_bytes: stats.total_bytes,
            progress_bytes: stats.progress_bytes,
            uploaded_bytes: stats.uploaded_bytes,
            finished: stats.finished,
            download_rate,
            upload_rate,
            eta_ms: eta.map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64),
            eta_confidence: match (eta, download_rate) {
                (None, _) => "none",
                (Some(_), 0) => "low",
                (Some(_), _) => "measured",
            },
            peers,
            error: stats.error,
        }
    }

    /// Per-peer accounting for one torrent.
    ///
    /// `bridge_ports` are the loopback ports this run's web seed bridges are
    /// connected from. A bridge is our own HTTP source wearing a peer's
    /// clothes, so it is labelled rather than counted as a swarm member.
    pub fn peers(&self, handle: &ManagedTorrent, bridge_ports: &HashSet<u16>) -> Vec<PeerSnapshot> {
        let Some(live) = handle.live() else {
            return Vec::new();
        };
        let snapshot = live.per_peer_stats_snapshot(all_peers_filter());
        let mut rows: Vec<PeerSnapshot> = snapshot
            .peers
            .into_iter()
            .map(|(addr, peer)| {
                let counters = peer.counters;
                let mean_piece_ms = (counters.downloaded_and_checked_pieces > 0).then(|| {
                    counters.total_piece_download_ms
                        / u64::from(counters.downloaded_and_checked_pieces)
                });
                PeerSnapshot {
                    web_seed: is_bridge_addr(&addr, bridge_ports),
                    addr,
                    state: peer.state.to_string(),
                    client: peer.client_name,
                    connection: peer.conn_kind.map(|k| format!("{k:?}").to_lowercase()),
                    direction: match counters.incoming_connections > 0 {
                        true => "incoming",
                        false => "outgoing",
                    },
                    downloaded_bytes: counters.fetched_bytes,
                    uploaded_bytes: counters.uploaded_bytes,
                    verified_pieces: counters.downloaded_and_checked_pieces,
                    chunks: counters.fetched_chunks,
                    errors: counters.errors,
                    connect_ms: counters.total_time_connecting_ms,
                    mean_piece_ms,
                }
            })
            .collect();
        rows.sort_by(|a, b| a.addr.cmp(&b.addr));
        rows
    }

    /// Which pieces are present, one bool per piece.
    ///
    /// The wire bitfield is byte aligned, so it carries spare bits past the
    /// last piece that must not be reported as pieces.
    pub fn have_pieces(&self, handle: &ManagedTorrent) -> Option<Vec<bool>> {
        let (have, total) = self
            .api
            .api_dump_haves(TorrentIdOrHash::Id(handle.id()))
            .ok()?;
        Some(have.iter().map(|bit| *bit).take(total as usize).collect())
    }

    /// The torrent's piece hashes, once its metadata has resolved.
    ///
    /// This is what lets an HTTP source be checked at the source rather than
    /// only by the session, which is the difference between "a peer served
    /// bad data" and "this mirror served piece 4108 wrong".
    pub fn piece_hashes(&self, handle: &ManagedTorrent) -> Option<Arc<Vec<[u8; 20]>>> {
        handle
            .with_metadata(|metadata| {
                let raw = metadata.info.info().pieces.as_ref();
                Arc::new(
                    raw.chunks_exact(20)
                        .filter_map(|chunk| <[u8; 20]>::try_from(chunk).ok())
                        .collect::<Vec<[u8; 20]>>(),
                )
            })
            .ok()
    }

    /// The trackers this torrent announces to, in sorted order.
    pub fn trackers(&self, handle: &ManagedTorrent) -> Vec<String> {
        let mut out: Vec<String> = handle
            .shared()
            .trackers
            .iter()
            .map(|u| u.to_string())
            .collect();
        out.sort();
        out
    }

    /// The torrent's layout, once its metadata has resolved.
    pub fn layout(&self, handle: &ManagedTorrent) -> Option<Layout> {
        let info_hash = handle.info_hash();
        handle
            .with_metadata(|metadata| {
                let name = metadata
                    .info
                    .name()
                    .map(|n| n.into_owned())
                    .unwrap_or_else(|| info_hash.as_string());
                // The on-disk relative filename is sanitized for the platform,
                // so path separators come back as the OS uses them. The layout
                // is `/`-separated everywhere, which is what BEP 19 URL
                // composition needs.
                let files = metadata
                    .file_infos
                    .iter()
                    .map(|f| {
                        let path = f
                            .relative_filename
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join("/");
                        (path, f.len)
                    })
                    .collect::<Vec<_>>();
                Layout::from_lengths(
                    name,
                    metadata.info.info().files.is_some(),
                    metadata.lengths().default_piece_length(),
                    files,
                )
            })
            .ok()
    }

    /// Wait until the torrent's metadata has resolved and any hash check has
    /// finished.
    pub async fn wait_until_initialized(&self, handle: &ManagedTorrent) -> Result<()> {
        handle
            .wait_until_initialized()
            .await
            .map_err(|e| Error::generic(format!("torrent failed to initialize: {e}")))
    }

    /// Wait until every wanted piece is present and verified.
    pub async fn wait_until_completed(&self, handle: &ManagedTorrent) -> Result<()> {
        handle
            .wait_until_completed()
            .await
            .map_err(|e| Error::generic(format!("torrent failed: {e}")))
    }

    /// Change the live rate limits.
    pub fn set_rates(&self, download: Option<u64>, upload: Option<u64>) {
        self.session
            .ratelimits
            .set_download_bps(rate_to_bps(download));
        self.session.ratelimits.set_upload_bps(rate_to_bps(upload));
    }

    /// Stop the session and everything running under it.
    pub async fn stop(self) {
        self.session.stop().await;
    }
}

/// A torrent whose metadata has been resolved without starting it.
pub struct ResolvedTorrent {
    pub info_hash: InfoHash,
    pub layout: Layout,
    /// The `.torrent` bytes, so a magnet can be turned into a file.
    pub torrent_bytes: Vec<u8>,
}

/// A peer filter that keeps every peer, not only the connected ones.
///
/// A peer that sent two gigabytes and then disconnected still belongs in the
/// accounting, so the default filter (connected peers only) is wrong here.
/// `librqbit` 9.0.0 exports `PeerStatsFilter` but not the enum its one field
/// holds, so the value is built through the type's own `Deserialize`, which is
/// public. `TODO/peers.md` carries the upstream export gap. The literal is
/// fixed and known to parse, so the fallback is unreachable rather than
/// silently narrowing the report.
fn all_peers_filter() -> PeerStatsFilter {
    serde_json::from_str::<PeerStatsFilter>(r#"{"state":"all"}"#).unwrap_or_default()
}

/// Join a torrent path's components with `/`, on every platform.
fn join_components<'a>(components: impl Iterator<Item = std::borrow::Cow<'a, str>>) -> String {
    components
        .map(|c| c.into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Turn a bytes-per-second cap into what `librqbit` wants.
///
/// Zero and `None` both mean unlimited. A cap above `u32::MAX` bytes per
/// second is past any real link, so it saturates rather than wrapping.
fn rate_to_bps(rate: Option<u64>) -> Option<NonZeroU32> {
    let rate = rate?;
    NonZeroU32::new(rate.min(u64::from(u32::MAX)) as u32)
}

fn live_rates(stats: &TorrentStats) -> (u64, u64, PeerCounts) {
    match &stats.live {
        Some(live) => {
            let peers = &live.snapshot.peer_stats;
            (
                live.download_speed.as_bytes(),
                live.upload_speed.as_bytes(),
                PeerCounts {
                    live: peers.live,
                    connecting: peers.connecting,
                    queued: peers.queued,
                    seen: peers.seen,
                    dead: peers.dead,
                },
            )
        }
        None => (0, 0, PeerCounts::default()),
    }
}

fn to_state(state: &TorrentStatsState) -> State {
    match state {
        // The `paused` flag here is what the torrent will do once
        // initialization finishes. While it runs, it really is initializing.
        TorrentStatsState::Initializing { .. } => State::Initializing,
        TorrentStatsState::Live => State::Live,
        TorrentStatsState::Paused => State::Paused,
        TorrentStatsState::Error => State::Error,
    }
}

/// Give an add failure the exit code that matches what actually went wrong.
///
/// `librqbit` reports these as one opaque error chain, so the classification
/// is by the text of the chain. A caller branches on the exit code, and
/// "could not write the file" and "the tracker is unreachable" must not both
/// arrive as a generic failure.
fn classify_add_error(source: &str, err: &anyhow::Error) -> Error {
    let text = format!("{err:#}");
    let lower = text.to_lowercase();
    let code = if lower.contains("no such file")
        || lower.contains("cannot find the file")
        || lower.contains("permission denied")
        || lower.contains("os error 17")
    {
        crate::exit::ExitCode::Disk
    } else if lower.contains("dns") || lower.contains("connect") || lower.contains("tls") {
        crate::exit::ExitCode::Network
    } else {
        crate::exit::ExitCode::SourceResolution
    };
    Error::new(code, format!("{source}: {text}")).with("source", source.to_string())
}

/// Address families tried when binding the peer listener.
///
/// IPv6 first: `librqbit` clears `IPV6_V6ONLY` for an unspecified v6 address,
/// so `[::]` is a genuine dual-stack socket on Windows as well as Linux.
const LISTEN_FAMILIES: [IpAddr; 2] = [
    IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    IpAddr::V4(Ipv4Addr::UNSPECIFIED),
];

/// Pick the address to listen on for incoming peer connections.
///
/// `librqbit` binds one address and fails the session if it is taken, so the
/// configured range is walked here and an OS-assigned port is used as a last
/// resort. A port clash costs the preferred port rather than the run.
pub fn resolve_listen_addr(ports: &std::ops::RangeInclusive<u16>) -> (SocketAddr, Option<String>) {
    choose_listen_addr(ports, &bindable)
}

/// The port-selection decision, with the socket probe injected.
///
/// Separating the decision from the probe is what makes it testable: a test
/// describes which addresses are taken and asserts the choice without binding
/// anything. That also keeps the test suite from opening wildcard listeners,
/// which a host firewall asks the user about once per binary.
fn choose_listen_addr(
    ports: &std::ops::RangeInclusive<u16>,
    free: &dyn Fn(&SocketAddr) -> bool,
) -> (SocketAddr, Option<String>) {
    // Probing only `[::]` is not enough. On Windows the standard library
    // leaves `IPV6_V6ONLY` on, so a successful `[::]` bind says nothing about
    // IPv4, and the dual-stack socket `librqbit` then builds fails on a port
    // that is only taken on the IPv4 side.
    let dual_stack_free = |port: u16| {
        LISTEN_FAMILIES
            .iter()
            .all(|ip| free(&SocketAddr::new(*ip, port)))
    };

    if let Some(port) = ports.clone().find(|port| dual_stack_free(*port)) {
        return (SocketAddr::new(LISTEN_FAMILIES[0], port), None);
    }
    // No port in the range is free on both stacks. Try one family at a time
    // before giving the port choice to the operating system.
    for ip in LISTEN_FAMILIES {
        if let Some(addr) = ports
            .clone()
            .map(|port| SocketAddr::new(ip, port))
            .find(|a| free(a))
        {
            return (
                addr,
                Some(format!(
                    "port {} is only free on {}, so the peer listener is not dual-stack",
                    addr.port(),
                    match ip.is_ipv6() {
                        true => "IPv6",
                        false => "IPv4",
                    }
                )),
            );
        }
    }

    let warning = format!(
        "ports {}-{} are unavailable, letting the operating system choose the peer port",
        ports.start(),
        ports.end()
    );
    for ip in LISTEN_FAMILIES {
        let any = SocketAddr::new(ip, 0);
        if free(&any) {
            return (any, Some(warning));
        }
    }
    // Nothing binds at all. Hand back the configured port so `librqbit`
    // reports the real reason rather than this function guessing at it.
    (
        SocketAddr::new(LISTEN_FAMILIES[0], *ports.start()),
        Some(warning),
    )
}

/// The first free port in `ports` on one specific address.
///
/// Port zero is not probed: it means "let the operating system choose", and
/// probing it would only prove that the OS can hand out a port.
fn bind_on(ip: IpAddr, ports: &std::ops::RangeInclusive<u16>) -> SocketAddr {
    if *ports.start() == 0 {
        return SocketAddr::new(ip, 0);
    }
    match ports
        .clone()
        .map(|port| SocketAddr::new(ip, port))
        .find(bindable)
    {
        Some(addr) => addr,
        None => SocketAddr::new(ip, 0),
    }
}

/// Whether `addr` can be bound right now.
///
/// The probe socket closes immediately, so `librqbit` binds it moments later.
/// This is a race in principle; in practice a port that was free a moment ago
/// is the best answer available, and losing the race produces a clear bind
/// error rather than a wrong result.
fn bindable(addr: &SocketAddr) -> bool {
    std::net::TcpListener::bind(addr).is_ok()
}

/// Where a bridge dials to reach the session's own peer listener.
///
/// An unspecified bind address is not connectable, so it becomes loopback.
/// Anything else is already an address the session answers on.
pub fn loopback_target(listen: SocketAddr) -> SocketAddr {
    if !listen.ip().is_unspecified() {
        return listen;
    }
    let ip = match listen.ip() {
        IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
    };
    SocketAddr::new(ip, listen.port())
}

/// Whether a peer address is one of this run's own web seed bridges.
fn is_bridge_addr(addr: &str, ports: &HashSet<u16>) -> bool {
    if ports.is_empty() {
        return false;
    }
    let Some((host, port)) = addr.rsplit_once(':') else {
        return false;
    };
    if !matches!(host, "127.0.0.1" | "[::1]" | "::1" | "localhost") {
        return false;
    }
    port.parse().is_ok_and(|port| ports.contains(&port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unspecified_listen_address_becomes_loopback() {
        let v4: SocketAddr = "0.0.0.0:6881".parse().unwrap();
        assert_eq!(loopback_target(v4), "127.0.0.1:6881".parse().unwrap());

        let v6: SocketAddr = "[::]:6881".parse().unwrap();
        assert_eq!(loopback_target(v6), "[::1]:6881".parse().unwrap());
    }

    #[test]
    fn a_specific_listen_address_is_dialled_as_is() {
        let addr: SocketAddr = "192.0.2.10:51413".parse().unwrap();
        assert_eq!(loopback_target(addr), addr);
    }

    /// A probe that reports the listed addresses as taken and everything else
    /// as free. Nothing is bound, so the test suite never opens a wildcard
    /// listener and never trips a host firewall.
    fn taken(addrs: &[&str]) -> impl Fn(&SocketAddr) -> bool + use<> {
        let taken: Vec<SocketAddr> = addrs.iter().map(|a| a.parse().unwrap()).collect();
        move |addr: &SocketAddr| !taken.contains(addr)
    }

    #[test]
    fn a_free_port_range_yields_its_first_port_on_the_dual_stack_address() {
        let (addr, warning) = choose_listen_addr(&(6881..=6889), &taken(&[]));
        assert_eq!(addr.port(), 6881);
        assert!(addr.is_ipv6() && addr.ip().is_unspecified(), "{addr}");
        assert_eq!(warning, None);
    }

    #[test]
    fn a_busy_port_is_skipped_for_the_next_one_in_the_range() {
        let busy = taken(&["[::]:6881", "0.0.0.0:6881"]);
        let (addr, warning) = choose_listen_addr(&(6881..=6889), &busy);
        assert_eq!(addr.port(), 6882);
        assert_eq!(warning, None);
    }

    #[test]
    fn a_port_taken_on_ipv4_alone_is_not_chosen_for_a_dual_stack_listener() {
        // This is the Windows trap: `[::]` binds IPv6-only there, so probing
        // it alone reports a port free that a dual-stack bind will then fail
        // on. Only the IPv4 side of 6881 is held here.
        let busy = taken(&["0.0.0.0:6881"]);
        let (addr, warning) = choose_listen_addr(&(6881..=6882), &busy);
        assert_eq!(addr.port(), 6882, "6881 is not dual-stack free");
        assert_eq!(warning, None);
    }

    #[test]
    fn a_single_family_fallback_says_which_family_it_settled_for() {
        // Every port in the range is held on IPv4, so no dual-stack bind is
        // possible and the listener settles for IPv6 with a warning.
        let busy = taken(&["0.0.0.0:6881", "0.0.0.0:6882"]);
        let (addr, warning) = choose_listen_addr(&(6881..=6882), &busy);
        assert!(addr.is_ipv6());
        assert_eq!(addr.port(), 6881);
        assert!(warning.unwrap().contains("IPv6"));
    }

    #[test]
    fn an_exhausted_range_falls_back_to_an_os_chosen_port() {
        let busy = taken(&["[::]:6881", "0.0.0.0:6881"]);
        let (addr, warning) = choose_listen_addr(&(6881..=6881), &busy);
        assert_eq!(
            addr.port(),
            0,
            "an OS-assigned port is the documented fallback"
        );
        assert!(warning.unwrap().contains("6881-6881"));
    }

    #[test]
    fn nothing_bindable_at_all_hands_the_configured_port_back() {
        // With no address usable, guessing is worse than letting `librqbit`
        // fail on the port the caller actually asked for and say why.
        let (addr, warning) = choose_listen_addr(&(6881..=6881), &|_| false);
        assert_eq!(addr.port(), 6881);
        assert!(warning.is_some());
    }

    #[test]
    fn a_loopback_only_listener_walks_the_range_on_that_address_alone() {
        assert_eq!(
            bind_on(Ipv4Addr::LOCALHOST.into(), &(0..=0)),
            "127.0.0.1:0".parse().unwrap(),
            "port zero is never probed; it means the OS chooses"
        );
    }

    #[test]
    fn rate_limits_saturate_rather_than_wrapping() {
        assert_eq!(rate_to_bps(None), None);
        assert_eq!(rate_to_bps(Some(0)), None, "zero means unlimited");
        assert_eq!(rate_to_bps(Some(1024)).unwrap().get(), 1024);
        assert_eq!(rate_to_bps(Some(u64::MAX)).unwrap().get(), u32::MAX);
    }

    #[test]
    fn only_loopback_addresses_on_a_known_bridge_port_count_as_web_seeds() {
        let ports: HashSet<u16> = [40001].into_iter().collect();
        assert!(is_bridge_addr("127.0.0.1:40001", &ports));
        assert!(is_bridge_addr("[::1]:40001", &ports));
        assert!(
            !is_bridge_addr("127.0.0.1:40002", &ports),
            "a different port is a real peer"
        );
        assert!(
            !is_bridge_addr("203.0.113.7:40001", &ports),
            "a routable address is a real peer"
        );
        assert!(!is_bridge_addr("127.0.0.1:40001", &HashSet::new()));
        assert!(!is_bridge_addr("garbage", &ports));
    }

    #[test]
    fn states_have_stable_names() {
        for state in [
            State::Initializing,
            State::Live,
            State::Paused,
            State::Error,
        ] {
            assert!(
                state.as_str().chars().all(|c| c.is_ascii_lowercase()),
                "{state:?}"
            );
        }
    }

    #[test]
    fn progress_and_ratio_never_divide_by_zero() {
        let mut snapshot = TorrentSnapshot {
            id: 0,
            info_hash: "0".repeat(40),
            name: "t".into(),
            state: State::Live,
            total_bytes: 0,
            progress_bytes: 0,
            uploaded_bytes: 100,
            finished: false,
            download_rate: 0,
            upload_rate: 0,
            eta_ms: None,
            eta_confidence: "none",
            peers: PeerCounts::default(),
            error: None,
        };
        assert_eq!(snapshot.fraction(), 0.0);
        assert_eq!(snapshot.ratio(), 0.0);

        snapshot.total_bytes = 200;
        snapshot.progress_bytes = 50;
        assert_eq!(snapshot.fraction(), 0.25);
        assert_eq!(snapshot.ratio(), 2.0);

        // Progress past the total, which a re-check can briefly report, must
        // not produce a fraction above one.
        snapshot.progress_bytes = 500;
        assert_eq!(snapshot.fraction(), 1.0);
    }
}
