mod bencode;
#[cfg(feature = "dht")]
mod dht;
mod geoip;
mod http;
mod ip_filter;
#[cfg(feature = "lpd")]
mod lpd;
#[cfg(feature = "mse")]
mod mse;
#[cfg(feature = "natpmp")]
mod natpmp;
mod ownership;
mod peer;
mod peer_stream;
mod piece;
mod proxy;
mod rss;
mod search;
mod sha1;
mod sha256;
mod state_dir;
mod storage;
mod torrent;
mod tracker;
#[cfg(feature = "udp_tracker")]
mod udp_tracker;
mod ui;
#[cfg(feature = "upnp")]
mod upnp;
#[cfg(feature = "utp")]
mod utp;
#[cfg(windows)]
mod windows_fs;
mod xml;

#[cfg(not(feature = "dht"))]
mod dht {
    use std::net::SocketAddr;
    use std::path::Path;
    use std::sync::mpsc;

    #[derive(Clone)]
    pub struct Dht;

    pub fn start(_port: u16, _download_dir: &Path) -> Dht {
        Dht
    }

    pub fn disabled() -> Dht {
        Dht
    }

    impl Dht {
        pub fn add_torrent(
            &self,
            _info_hash: [u8; 20],
            _port: u16,
            _peers_tx: mpsc::Sender<Vec<SocketAddr>>,
        ) {
        }

        pub fn remove_torrent(&self, _info_hash: [u8; 20]) {}
    }
}

#[cfg(not(feature = "lpd"))]
mod lpd {
    use std::net::SocketAddr;
    use std::sync::mpsc;

    #[derive(Clone)]
    pub struct Lpd;

    pub fn start() -> Lpd {
        Lpd
    }

    pub fn disabled() -> Lpd {
        Lpd
    }

    impl Lpd {
        pub fn add_torrent(
            &self,
            _info_hash: [u8; 20],
            _port: u16,
            _peers_tx: mpsc::Sender<Vec<SocketAddr>>,
        ) {
        }

        pub fn remove_torrent(&self, _info_hash: [u8; 20]) {}
    }
}

#[cfg(not(feature = "udp_tracker"))]
mod udp_tracker {
    use std::fmt;
    use std::time::Instant;

    use crate::tracker::TrackerResponse;

    #[derive(Debug)]
    pub struct Error;

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "udp tracker disabled")
        }
    }

    impl std::error::Error for Error {}

    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub struct ScrapeResult {
        pub seeders: u32,
        pub leechers: u32,
        #[allow(dead_code)]
        pub completed: u32,
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn announce(
        _url: &str,
        _info_hash: [u8; 20],
        _peer_id: [u8; 20],
        _port: u16,
        _uploaded: u64,
        _downloaded: u64,
        _left: u64,
        _event: Option<&str>,
        _numwant: u32,
    ) -> Result<TrackerResponse, Error> {
        Err(Error)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn announce_until(
        _url: &str,
        _info_hash: [u8; 20],
        _peer_id: [u8; 20],
        _port: u16,
        _uploaded: u64,
        _downloaded: u64,
        _left: u64,
        _event: Option<&str>,
        _numwant: u32,
        _deadline: Instant,
    ) -> Result<TrackerResponse, Error> {
        Err(Error)
    }

    #[allow(dead_code)]
    pub fn scrape(_url: &str, _info_hash: [u8; 20]) -> Result<ScrapeResult, Error> {
        Err(Error)
    }
}

#[cfg(not(feature = "natpmp"))]
mod natpmp {
    pub fn map_port(_port: u16, _lifetime: u32) -> Result<(), String> {
        Err("natpmp disabled".to_string())
    }
}

#[cfg(not(feature = "upnp"))]
mod upnp {
    pub fn map_port(_port: u16) -> Result<(), String> {
        Err("upnp disabled".to_string())
    }
}

#[cfg(not(feature = "utp"))]
mod utp {
    use std::io::{Read, Write};
    use std::net::SocketAddr;
    use std::time::Duration;

    #[derive(Clone)]
    pub struct UtpConnector;

    pub struct UtpListener;

    #[derive(Clone)]
    pub struct UtpStream;

    pub fn start(_port: u16) -> (UtpConnector, UtpListener) {
        (UtpConnector, UtpListener)
    }

    impl UtpConnector {
        pub fn connect(&self, _addr: SocketAddr) -> Result<UtpStream, String> {
            Err("utp disabled".to_string())
        }
    }

    impl UtpListener {
        pub fn try_accept(&self) -> Option<UtpStream> {
            None
        }
    }

    impl UtpStream {
        pub fn peer_addr(&self) -> SocketAddr {
            SocketAddr::from(([0, 0, 0, 0], 0))
        }

        pub fn set_read_timeout(&mut self, _timeout: Option<Duration>) {}

        pub fn set_write_timeout(&mut self, _timeout: Option<Duration>) {}
    }

    impl Read for UtpStream {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("utp disabled"))
        }
    }

    impl Write for UtpStream {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("utp disabled"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(not(feature = "mse"))]
mod mse {
    use std::io::{Read, Write};

    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    pub enum CryptoMode {
        Plaintext,
    }

    #[derive(Clone)]
    pub struct CipherState;

    impl CipherState {
        #[cfg_attr(not(test), allow(dead_code))]
        pub fn new(_enc_key: &[u8], _dec_key: &[u8]) -> Self {
            Self
        }

        pub fn encrypt(&mut self, _data: &mut [u8]) {}
        pub fn decrypt(&mut self, _data: &mut [u8]) {}
    }

    pub fn initiate<RW: Read + Write>(
        _stream: &mut RW,
        _info_hash: [u8; 20],
        _allow_plain: bool,
        _initial_payload: &[u8],
    ) -> Result<(CryptoMode, Option<CipherState>, Vec<u8>), String> {
        Err("mse disabled".to_string())
    }

    #[allow(clippy::type_complexity)]
    pub fn accept<RW: Read + Write>(
        _stream: &mut RW,
        _info_hashes: &[[u8; 20]],
        _first_byte: u8,
        _allow_plain: bool,
    ) -> Result<(CryptoMode, Option<CipherState>, [u8; 20], Vec<u8>, Vec<u8>), String> {
        Err("mse disabled".to_string())
    }
}

use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::bencode::Value;
use crate::ip_filter::IpFilter;
use crate::peer_stream::PeerStream;

type BencodeDict = Vec<(Vec<u8>, Value)>;
type ExtendedHandshakeCaps = (Option<u8>, Option<u8>, Option<usize>);

const PIPELINE_DEPTH: usize = 64;
const MIN_PIPELINE_DEPTH: usize = 32;
const MAX_PIPELINE_DEPTH: usize = 256;
const MAX_ACTIVE_PIECES_PER_PEER: usize = 12;
const MAX_SINGLE_PIECE_BUFFER_BYTES: usize = torrent::MAX_PIECE_LENGTH as usize
    + (torrent::MAX_PIECE_LENGTH as usize).div_ceil(piece::BLOCK_LEN as usize);
const MAX_TORRENT_PIECE_BUFFER_BYTES: usize = 4 * MAX_SINGLE_PIECE_BUFFER_BYTES;
const MAX_GLOBAL_PIECE_BUFFER_BYTES: usize = 16 * MAX_SINGLE_PIECE_BUFFER_BYTES;
#[cfg(feature = "webseed")]
const WEBSEED_RESERVATION_ID: u64 = u64::MAX;
#[cfg(feature = "webseed")]
const WEBSEED_HTTP_BODY_SLACK: usize = 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_UPLOAD_BLOCK_LEN: u32 = 64 * 1024;
const MAX_IDLE_TICKS: u32 = 180;
const MAX_IDLE_TICKS_SEED: u32 = 1800;
const SNUB_TIMEOUT: Duration = Duration::from_secs(60);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(60);
const ENDGAME_BLOCKS: usize = 128;
const ENDGAME_DUP_TIMEOUT: Duration = Duration::from_secs(5);
const RESUME_SAVE_INTERVAL: Duration = Duration::from_secs(30);
const LOW_PEER_THRESHOLD: usize = 8;
const UPLOAD_SLOTS: usize = 6;
const UNCHOKE_INTERVAL: Duration = Duration::from_secs(10);
const OPTIMISTIC_UNCHOKE_INTERVAL: Duration = Duration::from_secs(30);
const METADATA_PEER_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const TRANSFER_PEER_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PEER_RETRIES: u32 = 8;
const PEER_RETRY_BASE_SECS: u64 = 2;
const NO_PEER_REANNOUNCE_SECS: u64 = 30;
const PEER_BAN_SECS: u64 = 60;
const PEER_RETRY_EXHAUSTED_BAN_SECS: u64 = 15 * 60;
const PEER_RETRY_MAX_SECS: u64 = 30;
const PEER_THREAD_STACK: usize = 512 * 1024; // 512KB
const TRACKER_ANNOUNCE_WAIT_BUDGET: Duration = Duration::from_secs(4);
const TRACKER_STOPPED_WAIT_BUDGET: Duration = Duration::from_secs(2);
const TRACKER_ANNOUNCE_POLL: Duration = Duration::from_millis(150);
const TORRENT_LOOP_INTERVAL: Duration = Duration::from_millis(200);
const PEER_QUEUE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const STALL_REANNOUNCE_SECS: u64 = 30;
const STARTUP_BURST_MIN_WORKERS: usize = 24;
const STARTUP_BURST_MULTIPLIER: usize = 3;
const STARTUP_BURST_BYTES: u64 = 8 * 1024 * 1024;
const REQUEST_QUEUE_TIME_SECS: f64 = 2.0;
const DEFAULT_PEER_RATE_BPS: f64 = 512.0 * 1024.0;
const MAX_TORRENT_BYTES: usize = 2 * 1024 * 1024;
const MAX_CONFIG_BYTES: usize = 1024 * 1024;
const MAX_PID_FILE_BYTES: usize = 64;
const MAX_RESUME_STATE_BYTES: usize = 16 * 1024 * 1024;
const MAX_SESSION_STATE_BYTES: usize = 64 * 1024 * 1024;
const MAX_SESSION_ENTRIES: usize = 4096;
const MAX_ATOMIC_BACKUP_BYTES: usize = MAX_SESSION_STATE_BYTES;
const MAX_RSS_POLL_WORKERS: usize = 16;
const MAX_RSS_DOWNLOAD_WORKERS: usize = 16;
const MAX_RSS_MATCHES_PER_POLL: usize = 64;
const MAX_TRACKERS_PER_TORRENT: usize = 64;
const MAX_TRACKER_URL_LEN: usize = 2048;
const MAX_MAGNET_SOURCES: usize = 16;
const MAX_MAGNET_WEB_SEEDS: usize = 64;
const MAX_MAGNET_EXPLICIT_PEERS: usize = 256;
const MAX_TRACKER_WORKERS: usize = 8;
const MAX_GLOBAL_TRACKER_WORKERS: usize = 64;
static ACTIVE_TRACKER_WORKERS: AtomicUsize = AtomicUsize::new(0);
const MIN_INBOUND_HANDLER_SLOTS: usize = 64;
const MAX_INBOUND_HANDLER_SLOTS: usize = 1024;
const METADATA_PIECE_LEN: usize = 16 * 1024;
const METADATA_FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const METADATA_TOTAL_TIMEOUT: Duration = Duration::from_secs(90);
const METADATA_REQUEST_RETRY: Duration = Duration::from_secs(3);
const METADATA_PEER_IDLE_TIMEOUT: Duration = Duration::from_secs(10);
const MAGNET_CACHE_URLS: [&str; 2] = [
    "https://itorrents.org/torrent/",
    "https://torrage.info/torrent/",
];
const HANDSHAKE_LEN: usize = 68;
const SHUTDOWN_SLEEP_SLICE_MS: u64 = 50;
const TORRENT_WORKER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);
const TORRENT_RESOURCE_DRAIN_TIMEOUT: Duration = Duration::from_secs(15);
const TEARDOWN_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[cfg(unix)]
const SIGINT: i32 = 2;
#[cfg(unix)]
const SIGTERM: i32 = 15;
#[cfg(unix)]
const SIGPIPE: i32 = 13;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);
static PROGRESS_ACTIVE: AtomicBool = AtomicBool::new(false);
static PROGRESS_LINE_LEN: AtomicUsize = AtomicUsize::new(0);
static LOG_LOCK: Mutex<()> = Mutex::new(());
static LOG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();
static SESSION_DOWNLOADED_BYTES: AtomicU64 = AtomicU64::new(0);
static SESSION_UPLOADED_BYTES: AtomicU64 = AtomicU64::new(0);
static ATOMIC_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
static PEER_CONNECTED: AtomicU64 = AtomicU64::new(0);
static PEER_DISCONNECTED: AtomicU64 = AtomicU64::new(0);
static SEED_RATIO_BITS: AtomicU64 = AtomicU64::new(0);
static MAX_SEED_TIME_SECS: AtomicU64 = AtomicU64::new(0);
static SUPER_SEED: AtomicBool = AtomicBool::new(false);
static ON_COMPLETE_SCRIPT: OnceLock<PathBuf> = OnceLock::new();

#[allow(dead_code)]
struct ThrottleGroup {
    name: String,
    down: Arc<RateLimiter>,
    up: Arc<RateLimiter>,
}

static THROTTLE_GROUPS: OnceLock<Mutex<Vec<ThrottleGroup>>> = OnceLock::new();

struct RatioGroup {
    name: String,
    ratio: f64,
    action: String,
}

static RATIO_GROUPS: OnceLock<Mutex<Vec<RatioGroup>>> = OnceLock::new();

struct ScheduleEntry {
    interval_secs: u64,
    command: String,
    last_run: Instant,
}

static SCHEDULES: OnceLock<Mutex<Vec<ScheduleEntry>>> = OnceLock::new();
static GEOIP_DB: OnceLock<geoip::GeoIpDb> = OnceLock::new();
static RSS_STATE: OnceLock<Mutex<rss::RssState>> = OnceLock::new();

struct RssPollResult {
    url: String,
    parsed: Result<(String, Vec<rss::FeedItem>), String>,
}

struct RssDownloadResult {
    seen_key: String,
    url: String,
    title: String,
    data: Result<Vec<u8>, String>,
}

#[derive(Clone)]
enum TorrentSource {
    Path(String),
    Bytes(Vec<u8>),
    Magnet(String),
}

#[derive(Clone)]
struct TorrentRequest {
    id: u64,
    source: TorrentSource,
    download_dir: PathBuf,
    preallocate: bool,
    initial_label: String,
}

#[derive(Clone)]
struct SessionEntry {
    info_hash: [u8; 20],
    name: String,
    torrent_bytes: Vec<u8>,
    download_dir: PathBuf,
    preallocate: bool,
    label: String,
    completion_state: CompletionState,
    completion_move_dir: Option<PathBuf>,
    pending_delete: bool,
    file_renames: Vec<(usize, String)>,
    pending_file_rename: Option<PendingFileRename>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingFileRename {
    index: usize,
    target: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CompletionState {
    #[default]
    None,
    Pending,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionAction {
    None,
    MarkDone,
    RunScript,
    Move,
}

fn completion_action(
    state: CompletionState,
    was_complete: bool,
    is_complete: bool,
    move_requested: bool,
) -> CompletionAction {
    if !is_complete || state == CompletionState::Done {
        return CompletionAction::None;
    }
    if state == CompletionState::None && was_complete {
        return CompletionAction::MarkDone;
    }
    if move_requested {
        CompletionAction::Move
    } else {
        CompletionAction::RunScript
    }
}

impl CompletionState {
    fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::None => b"none",
            Self::Pending => b"pending",
            Self::Done => b"done",
        }
    }

    fn from_bytes(value: &[u8]) -> Option<Self> {
        match value {
            b"none" => Some(Self::None),
            b"pending" => Some(Self::Pending),
            b"done" => Some(Self::Done),
            _ => None,
        }
    }
}

struct SessionStore {
    path: PathBuf,
    entries: Mutex<HashMap<[u8; 20], SessionEntry>>,
    operations: Mutex<()>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EncryptionMode {
    Disable,
    Prefer,
    Require,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum PeerProfile {
    Conservative,
    #[default]
    Balanced,
    Aggressive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PeerProfileTuning {
    numwant: u32,
    max_peers_global: usize,
    max_peers_torrent: usize,
    metadata_peer_limit: usize,
}

#[derive(Clone)]
struct PeerRuntimeSettings {
    profile: Arc<AtomicUsize>,
    numwant: Arc<AtomicUsize>,
    metadata_peer_limit: Arc<AtomicUsize>,
    max_peers_global: Arc<AtomicUsize>,
    max_peers_torrent: Arc<AtomicUsize>,
}

impl PeerProfile {
    fn code(self) -> usize {
        match self {
            Self::Conservative => 0,
            Self::Balanced => 1,
            Self::Aggressive => 2,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn from_code(code: usize) -> Self {
        match code {
            0 => Self::Conservative,
            2 => Self::Aggressive,
            _ => Self::Balanced,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Conservative => "conservative",
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
        }
    }

    fn tuning(self) -> PeerProfileTuning {
        match self {
            Self::Conservative => PeerProfileTuning {
                numwant: 50,
                max_peers_global: 80,
                max_peers_torrent: 12,
                metadata_peer_limit: 20,
            },
            Self::Balanced => PeerProfileTuning {
                numwant: 200,
                max_peers_global: 200,
                max_peers_torrent: 30,
                metadata_peer_limit: 80,
            },
            Self::Aggressive => PeerProfileTuning {
                numwant: 500,
                max_peers_global: 500,
                max_peers_torrent: 80,
                metadata_peer_limit: 160,
            },
        }
    }
}

impl PeerRuntimeSettings {
    fn new(
        profile: PeerProfile,
        numwant: u32,
        metadata_peer_limit: usize,
        max_peers_global: usize,
        max_peers_torrent: usize,
    ) -> Self {
        Self {
            profile: Arc::new(AtomicUsize::new(profile.code())),
            numwant: Arc::new(AtomicUsize::new(numwant as usize)),
            metadata_peer_limit: Arc::new(AtomicUsize::new(metadata_peer_limit)),
            max_peers_global: Arc::new(AtomicUsize::new(max_peers_global)),
            max_peers_torrent: Arc::new(AtomicUsize::new(max_peers_torrent)),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn profile(&self) -> PeerProfile {
        PeerProfile::from_code(self.profile.load(Ordering::SeqCst))
    }

    fn numwant(&self) -> u32 {
        self.numwant.load(Ordering::SeqCst) as u32
    }

    fn metadata_peer_limit(&self) -> usize {
        self.metadata_peer_limit.load(Ordering::SeqCst)
    }

    fn max_peers_global(&self) -> usize {
        self.max_peers_global.load(Ordering::SeqCst)
    }

    fn max_peers_torrent(&self) -> usize {
        self.max_peers_torrent.load(Ordering::SeqCst)
    }

    fn apply_profile(&self, profile: PeerProfile) -> PeerProfileTuning {
        let tuning = profile.tuning();
        self.profile.store(profile.code(), Ordering::SeqCst);
        self.numwant
            .store(tuning.numwant as usize, Ordering::SeqCst);
        self.metadata_peer_limit
            .store(tuning.metadata_peer_limit, Ordering::SeqCst);
        self.max_peers_global
            .store(tuning.max_peers_global, Ordering::SeqCst);
        self.max_peers_torrent
            .store(tuning.max_peers_torrent, Ordering::SeqCst);
        tuning
    }
}

#[derive(Clone)]
struct ConnectionConfig {
    encryption: EncryptionMode,
    utp: Option<utp::UtpConnector>,
    ip_filter: Option<Arc<IpFilter>>,
    proxy: Option<proxy::ProxyConfig>,
}

#[derive(Clone)]
struct InboundConfig {
    encryption: EncryptionMode,
    ip_filter: Option<Arc<IpFilter>>,
    max_handlers: Arc<AtomicUsize>,
    active_handlers: Arc<AtomicUsize>,
}

struct InboundHandlerGuard {
    active: Option<Arc<AtomicUsize>>,
}

type PeerCancellationRegistry = Arc<Mutex<HashMap<u64, TcpStream>>>;

struct PeerCancellationGuard {
    registry: PeerCancellationRegistry,
    peer_tag: u64,
}

struct RateLimiter {
    limit_bps: AtomicU64,
    state: Mutex<RateState>,
}

struct RateState {
    allowance: f64,
    last: Instant,
}

#[derive(Clone)]
struct TransferLimits {
    global_down: Arc<RateLimiter>,
    global_up: Arc<RateLimiter>,
    torrent_down: Arc<RateLimiter>,
    torrent_up: Arc<RateLimiter>,
}

struct PeerSlots {
    max: AtomicUsize,
    active: AtomicUsize,
}

struct ActiveTorrentGuard {
    counter: Arc<AtomicUsize>,
}

struct InFlightTorrentGuard {
    reservations: InFlightTorrents,
    info_hash: [u8; 20],
    torrent_id: u64,
}

struct PidFileGuard {
    path: PathBuf,
    pid: u32,
}

struct SessionLocks {
    #[cfg(not(windows))]
    _legacy: fs::File,
    #[cfg(unix)]
    _state_directory: fs::File,
    #[cfg(windows)]
    _windows: state_dir::SessionLock,
}

struct UploadManager {
    inner: Mutex<UploadState>,
    max_unchoked: usize,
}

struct UploadState {
    peers: HashMap<u64, PeerUploadInfo>,
    unchoked: HashSet<u64>,
    last_schedule: Instant,
    last_optimistic: Instant,
    optimistic_peer: Option<u64>,
    rng: u64,
}

struct PeerUploadInfo {
    interested: bool,
    uploaded_total: u64,
    last_uploaded_total: u64,
    rate: u64,
    downloaded_total: u64,
    last_downloaded_total: u64,
    download_rate: u64,
}

struct TorrentContext {
    id: u64,
    info_hash: [u8; 20],
    hybrid_v2_info_hash: Option<[u8; 20]>,
    peer_id: [u8; 20],
    pieces: Arc<Mutex<piece::PieceManager>>,
    storage: Arc<Mutex<storage::Storage>>,
    completed_log: Arc<Mutex<Vec<u32>>>,
    base_piece_length: u64,
    v2_hashes: Arc<V2HashStore>,
    file_spans: Arc<Vec<FileSpan>>,
    file_priorities: Arc<Mutex<Vec<u8>>>,
    limits: TransferLimits,
    downloaded: Arc<AtomicU64>,
    uploaded: Arc<AtomicU64>,
    active_peers: Arc<AtomicUsize>,
    interested_peers: Arc<AtomicUsize>,
    upload_requests_served: Arc<AtomicU64>,
    paused: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
    allow_completion_reentry: Arc<AtomicBool>,
    rechecking: Arc<AtomicBool>,
    resume_save_requested: Arc<AtomicBool>,
    delete_data_requested: Arc<AtomicBool>,
    archive_requested: Arc<AtomicBool>,
    teardown_failed: Arc<AtomicBool>,
    upload_manager: Arc<UploadManager>,
    peer_tags: Arc<AtomicU64>,
    peer_cancellations: PeerCancellationRegistry,
    label: Arc<Mutex<String>>,
    trackers: Arc<Mutex<TrackerSet>>,
    #[allow(dead_code)]
    throttle_group: Arc<Mutex<Option<String>>>,
    ratio_group: Arc<Mutex<Option<String>>>,
    file_renames: Arc<Mutex<HashMap<usize, String>>>,
}

type SessionRegistry = Arc<Mutex<HashMap<[u8; 20], Arc<TorrentContext>>>>;
type InFlightTorrents = Arc<Mutex<HashMap<[u8; 20], u64>>>;

macro_rules! log_info {
    ($($t:tt)*) => {
        crate::log_stdout(format_args!($($t)*));
    };
}

macro_rules! log_warn {
    ($($t:tt)*) => {
        crate::log_stderr(format_args!($($t)*));
    };
}

macro_rules! log_debug {
    ($($t:tt)*) => {
        #[cfg(feature = "verbose")]
        {
            crate::log_stderr(format_args!($($t)*));
        }
    };
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn clear_progress_line() {
    if !PROGRESS_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    let len = PROGRESS_LINE_LEN.load(Ordering::SeqCst);
    if len == 0 {
        return;
    }
    eprint!("\r{} \r", " ".repeat(len));
    let _ = io::stderr().flush();
}

fn log_timestamp() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Days since 1970-01-01 to Y-M-D
    let mut y = 1970i64;
    let mut rem = days as i64;
    loop {
        let ylen = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if rem < ylen {
            break;
        }
        rem -= ylen;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut mo = 0usize;
    for &ml in &mdays {
        if rem < ml as i64 {
            break;
        }
        rem -= ml as i64;
        mo += 1;
    }
    format!("[{y:04}-{:02}-{:02} {h:02}:{m:02}:{s:02}]", mo + 1, rem + 1)
}

fn write_to_log_file(args: std::fmt::Arguments) {
    if let Some(file) = LOG_FILE.get() {
        if let Ok(mut f) = file.lock() {
            let ts = log_timestamp();
            let _ = writeln!(f, "{ts} {args}");
            let _ = f.flush();
        }
    }
}

pub(crate) fn log_stdout(args: std::fmt::Arguments) {
    let _guard = LOG_LOCK.lock().ok();
    clear_progress_line();
    println!("{args}");
    write_to_log_file(args);
}

pub(crate) fn log_stderr(args: std::fmt::Arguments) {
    let _guard = LOG_LOCK.lock().ok();
    clear_progress_line();
    eprintln!("{args}");
    write_to_log_file(args);
}

#[cfg_attr(
    not(any(
        feature = "dht",
        feature = "mse",
        feature = "udp_tracker",
        feature = "utp"
    )),
    allow(dead_code)
)]
pub(crate) fn system_entropy_u64() -> u64 {
    let mut random = [0u8; 8];
    if getrandom::fill(&mut random).is_ok() {
        return u64::from_ne_bytes(random);
    }

    // Randomness failure must not turn ordinary bookkeeping (such as a
    // collision-resistant temporary name) into a process-wide outage. MSE
    // and UI authentication call the OS provider directly and fail closed;
    // this mixed fallback is only used by non-secret protocol identifiers.
    use std::sync::OnceLock;
    static BASE: OnceLock<u64> = OnceLock::new();
    let base = *BASE.get_or_init(|| {
        let buf = [0u8; 8];
        let time_part = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let pid = std::process::id() as u64;
        let stack_addr = &buf as *const _ as u64;
        time_part ^ (pid.wrapping_mul(0x9E3779B97F4A7C15)) ^ stack_addr
    });
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let c = CTR.fetch_add(1, Ordering::Relaxed);
    let mut x = base ^ c.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    x
}

fn install_panic_logger() {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(msg) = info.payload().downcast_ref::<&str>() {
            *msg
        } else if let Some(msg) = info.payload().downcast_ref::<String>() {
            msg.as_str()
        } else {
            "panic"
        };
        let location = info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let message = format!("panic: {payload} at {location}");
        eprintln!("{message}");
    }));
}

fn main() {
    if let Err(err) = run() {
        log_warn!("error: {err}");
        std::process::exit(1);
    }
}

#[cfg(unix)]
extern "C" fn handle_signal(_: i32) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
extern "C" fn handle_sigpipe(_: i32) {}

#[cfg(unix)]
extern "C" {
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
}

#[cfg(unix)]
fn install_signal_handlers() {
    unsafe {
        let _ = signal(SIGINT, handle_signal);
        let _ = signal(SIGTERM, handle_signal);
        let _ = signal(SIGPIPE, handle_sigpipe);
    }
}

#[cfg(not(unix))]
fn install_signal_handlers() {}

fn shutdown_requested() -> bool {
    SHUTDOWN.load(Ordering::SeqCst)
}

pub fn request_shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::SeqCst);
}

pub fn is_paused() -> bool {
    PAUSED.load(Ordering::SeqCst)
}

fn torrent_paused(paused: &AtomicBool) -> bool {
    is_paused() || paused.load(Ordering::SeqCst)
}

fn torrent_stop_requested(stop_flag: &AtomicBool) -> bool {
    shutdown_requested() || stop_flag.load(Ordering::SeqCst)
}

fn sleep_with_shutdown(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    let deadline = Instant::now() + duration;
    loop {
        if shutdown_requested() {
            break;
        }
        let now = Instant::now();
        let Some(remaining) = deadline.checked_duration_since(now) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        let slice = remaining.min(Duration::from_millis(SHUTDOWN_SLEEP_SLICE_MS));
        thread::sleep(slice);
    }
}

fn sleep_with_shutdown_or_stop(duration: Duration, stop_flag: &AtomicBool) {
    if duration.is_zero() {
        return;
    }
    let deadline = Instant::now() + duration;
    loop {
        if torrent_stop_requested(stop_flag) {
            break;
        }
        let now = Instant::now();
        let Some(remaining) = deadline.checked_duration_since(now) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        let slice = remaining.min(Duration::from_millis(SHUTDOWN_SLEEP_SLICE_MS));
        thread::sleep(slice);
    }
}

fn join_worker(handle: thread::JoinHandle<()>, label: &str) {
    if handle.join().is_err() {
        log_warn!("{label} panicked");
    }
}

fn join_worker_before(handle: thread::JoinHandle<()>, label: &str, deadline: Instant) -> bool {
    while !handle.is_finished() {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            log_warn!("{label} did not stop before the teardown deadline; detaching it");
            return false;
        };
        if remaining.is_zero() {
            log_warn!("{label} did not stop before the teardown deadline; detaching it");
            return false;
        }
        thread::sleep(remaining.min(TEARDOWN_POLL_INTERVAL));
    }
    join_worker(handle, label);
    true
}

fn wait_for_torrent_resources(
    context: &Arc<TorrentContext>,
    storage: &Arc<Mutex<storage::Storage>>,
    operation: &str,
    deadline: Instant,
) -> Result<(), String> {
    loop {
        let active_peers = context.active_peers.load(Ordering::Acquire);
        let rechecking = context.rechecking.load(Ordering::Acquire);
        let context_refs = Arc::strong_count(context);
        let storage_refs = Arc::strong_count(storage);
        if active_peers == 0 && !rechecking && context_refs <= 1 && storage_refs <= 2 {
            return Ok(());
        }

        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(format!(
                "{operation} timed out waiting for torrent resources to stop \
                 (active peers: {active_peers}, rechecking: {rechecking}, \
                 context refs: {context_refs}, storage refs: {storage_refs})"
            ));
        };
        if remaining.is_zero() {
            return Err(format!(
                "{operation} timed out waiting for torrent resources to stop \
                 (active peers: {active_peers}, rechecking: {rechecking}, \
                 context refs: {context_refs}, storage refs: {storage_refs})"
            ));
        }
        thread::sleep(remaining.min(TEARDOWN_POLL_INTERVAL));
    }
}

fn retain_context_after_teardown_failure(
    registry: &SessionRegistry,
    context: &Arc<TorrentContext>,
) {
    context.teardown_failed.store(true, Ordering::Release);
    let mut registry = lock_or_recover(registry);
    registry
        .entry(context.info_hash)
        .or_insert_with(|| Arc::clone(context));
}

fn wait_for_torrent_resources_or_retain(
    registry: &SessionRegistry,
    context: &Arc<TorrentContext>,
    storage: &Arc<Mutex<storage::Storage>>,
    operation: &str,
    deadline: Instant,
) -> Result<(), String> {
    wait_for_torrent_resources(context, storage, operation, deadline).inspect_err(|_| {
        retain_context_after_teardown_failure(registry, context);
    })
}

fn reap_finished_workers(handles: &mut Vec<thread::JoinHandle<()>>, label: &str) {
    let mut index = 0;
    while index < handles.len() {
        if handles[index].is_finished() {
            let handle = handles.swap_remove(index);
            join_worker(handle, label);
        } else {
            index += 1;
        }
    }
}

fn spawn_detached<F>(name: &str, worker: F) -> bool
where
    F: FnOnce() + Send + 'static,
{
    match thread::Builder::new().name(name.to_string()).spawn(worker) {
        Ok(_) => true,
        Err(err) => {
            log_warn!("{name} worker could not start: {err}");
            false
        }
    }
}

#[cfg(test)]
mod teardown_liveness_tests {
    use super::*;
    use std::io::Read;
    use std::sync::mpsc;

    #[test]
    fn bounded_join_returns_when_a_worker_will_not_stop() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            done_tx.send(()).unwrap();
        });
        started_rx.recv().unwrap();

        let started = Instant::now();
        assert!(!join_worker_before(
            handle,
            "blocked test worker",
            Instant::now() + Duration::from_millis(30),
        ));
        assert!(started.elapsed() < Duration::from_secs(1));

        release_tx.send(()).unwrap();
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    }

    #[test]
    fn peer_cancellation_wakes_a_blocked_tcp_read() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let mut stream = PeerStream::tcp(client);
        // Production peer streams have a bounded read deadline. Retain that
        // invariant in this cross-platform liveness test because Windows does
        // not guarantee that shutdown on a duplicated socket handle promptly
        // interrupts a blocking read on another thread.
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let registry = Arc::new(Mutex::new(HashMap::new()));
        let cancellation = PeerCancellationGuard::new(&registry, 7, &stream);
        let (started_tx, started_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut stream = stream;
            let mut byte = [0u8; 1];
            started_tx.send(()).unwrap();
            result_tx.send(stream.read(&mut byte)).unwrap();
        });
        started_rx.recv().unwrap();

        cancel_peer_connections(&registry);
        let result = result_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(matches!(result, Ok(0) | Err(_)));

        worker.join().unwrap();
        drop(cancellation);
        drop(server);
        assert!(lock_or_recover(&registry).is_empty());
    }
}

fn storage_claims_for_session_entry(
    entry: &SessionEntry,
) -> Result<Vec<ownership::StorageClaim>, String> {
    let meta = torrent::parse_torrent(&entry.torrent_bytes)
        .map_err(|err| format!("storage ownership metainfo: {err}"))?;
    if meta.info_hash != entry.info_hash {
        return Err("storage ownership metainfo hash mismatch".to_string());
    }
    let pending_rename = entry
        .pending_file_rename
        .as_ref()
        .map(|pending| (pending.index, pending.target.as_str()));
    let pending_completion = (entry.completion_state == CompletionState::Pending)
        .then_some(entry.completion_move_dir.as_deref())
        .flatten();
    ownership::claims_for_torrent(
        &meta,
        &entry.download_dir,
        &entry.file_renames,
        pending_rename,
        pending_completion,
    )
}

fn ensure_session_storage_claim_available(
    entries: &HashMap<[u8; 20], SessionEntry>,
    proposed: &SessionEntry,
) -> Result<(), String> {
    if entries
        .keys()
        .all(|info_hash| *info_hash == proposed.info_hash)
    {
        return Ok(());
    }
    let proposed_claims = storage_claims_for_session_entry(proposed)?;
    for (other_hash, other) in entries {
        if *other_hash == proposed.info_hash {
            continue;
        }
        let other_claims = storage_claims_for_session_entry(other)?;
        for proposed_claim in &proposed_claims {
            if let Some(other_claim) = other_claims
                .iter()
                .find(|other_claim| ownership::claims_conflict(proposed_claim, other_claim))
            {
                return Err(format!(
                    "storage path {} conflicts with torrent {} at {}",
                    proposed_claim.path().display(),
                    hex(other_hash),
                    other_claim.path().display()
                ));
            }
        }
    }
    Ok(())
}

impl SessionStore {
    fn load(root: &Path) -> Result<Self, String> {
        let path = session_path(root);
        let entries = load_session_entries_with_recovery(&path, root)
            .map_err(|err| format!("cannot load session state {}: {err}", path.display()))?;
        Ok(Self {
            path,
            entries: Mutex::new(entries),
            operations: Mutex::new(()),
        })
    }

    fn lock_operation(&self) -> MutexGuard<'_, ()> {
        match self.operations.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn list(&self) -> Vec<SessionEntry> {
        let guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.values().cloned().collect()
    }

    fn contains(&self, info_hash: [u8; 20]) -> bool {
        let guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.contains_key(&info_hash)
    }

    fn get(&self, info_hash: [u8; 20]) -> Option<SessionEntry> {
        let guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.get(&info_hash).cloned()
    }

    /// Validate the durable claim before crash reconciliation performs any
    /// physical rename, move adoption, or source cleanup. The caller holds
    /// `operations`, serializing this check with every claim transition.
    fn validate_current_storage_claim(&self, info_hash: [u8; 20]) -> Result<(), String> {
        let guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let entry = guard
            .get(&info_hash)
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        storage_claims_for_session_entry(entry)?;
        ensure_session_storage_claim_available(&guard, entry)
    }

    fn begin_delete(&self, info_hash: [u8; 20]) -> Result<bool, String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.pending_delete {
            return Ok(false);
        }
        if previous.pending_file_rename.is_some() {
            return Err("file rename recovery is pending".to_string());
        }
        guard
            .get_mut(&info_hash)
            .ok_or_else(|| "torrent session metadata disappeared".to_string())?
            .pending_delete = true;
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }

    fn import_file_renames_if_empty(
        &self,
        info_hash: [u8; 20],
        file_renames: &[(usize, String)],
    ) -> Result<(), String> {
        if file_renames.is_empty() {
            return Ok(());
        }
        let mut normalized = file_renames.to_vec();
        normalized.sort_unstable_by_key(|(index, _)| *index);
        if normalized.windows(2).any(|pair| pair[0].0 == pair[1].0)
            || normalized
                .iter()
                .any(|(_, name)| !valid_renamed_file_name(name))
        {
            return Err("invalid persisted file rename".to_string());
        }
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if !previous.file_renames.is_empty() {
            return Ok(());
        }
        let mut proposed = previous.clone();
        proposed.file_renames = normalized;
        storage_claims_for_session_entry(&proposed)?;
        ensure_session_storage_claim_available(&guard, &proposed)?;
        guard.insert(info_hash, proposed);
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(())
    }

    fn begin_file_rename(
        &self,
        info_hash: [u8; 20],
        index: usize,
        target: &str,
    ) -> Result<bool, String> {
        if !valid_renamed_file_name(target) {
            return Err("invalid file name".to_string());
        }
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.pending_delete {
            return Err("torrent deletion is pending".to_string());
        }
        let pending = PendingFileRename {
            index,
            target: target.to_string(),
        };
        if previous.pending_file_rename.as_ref() == Some(&pending) {
            return Ok(false);
        }
        if previous.pending_file_rename.is_some() {
            return Err("another file rename is pending".to_string());
        }
        let mut proposed = previous.clone();
        proposed.pending_file_rename = Some(pending);
        storage_claims_for_session_entry(&proposed)?;
        ensure_session_storage_claim_available(&guard, &proposed)?;
        guard.insert(info_hash, proposed);
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }

    fn commit_file_rename(
        &self,
        info_hash: [u8; 20],
        pending: &PendingFileRename,
    ) -> Result<bool, String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.pending_file_rename.as_ref() != Some(pending) {
            return Ok(false);
        }
        let entry = guard
            .get_mut(&info_hash)
            .ok_or_else(|| "torrent session metadata disappeared".to_string())?;
        if let Some((_, name)) = entry
            .file_renames
            .iter_mut()
            .find(|(index, _)| *index == pending.index)
        {
            *name = pending.target.clone();
        } else {
            entry
                .file_renames
                .push((pending.index, pending.target.clone()));
            entry.file_renames.sort_unstable_by_key(|(index, _)| *index);
        }
        entry.pending_file_rename = None;
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }

    fn cancel_file_rename(
        &self,
        info_hash: [u8; 20],
        pending: &PendingFileRename,
    ) -> Result<bool, String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.pending_file_rename.as_ref() != Some(pending) {
            return Ok(false);
        }
        guard
            .get_mut(&info_hash)
            .ok_or_else(|| "torrent session metadata disappeared".to_string())?
            .pending_file_rename = None;
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }

    #[cfg(test)]
    fn upsert(
        &self,
        info_hash: [u8; 20],
        name: String,
        torrent_bytes: Vec<u8>,
        download_dir: &Path,
        preallocate: bool,
    ) -> Result<(), String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard.get(&info_hash).cloned();
        let (
            label,
            completion_state,
            completion_move_dir,
            pending_delete,
            file_renames,
            pending_file_rename,
        ) = previous
            .as_ref()
            .map(|entry| {
                (
                    entry.label.clone(),
                    entry.completion_state,
                    entry.completion_move_dir.clone(),
                    entry.pending_delete,
                    entry.file_renames.clone(),
                    entry.pending_file_rename.clone(),
                )
            })
            .unwrap_or_default();
        guard.insert(
            info_hash,
            SessionEntry {
                info_hash,
                name,
                torrent_bytes,
                download_dir: download_dir.to_path_buf(),
                preallocate,
                label,
                completion_state,
                completion_move_dir,
                pending_delete,
                file_renames,
                pending_file_rename,
            },
        );
        if let Err(err) = save_session(&self.path, &guard) {
            match previous {
                Some(entry) => {
                    guard.insert(info_hash, entry);
                }
                None => {
                    guard.remove(&info_hash);
                }
            }
            return Err(format!("session save failed: {err}"));
        }
        Ok(())
    }

    /// Atomically persist a session entry and its effective storage claim.
    /// The caller holds `operations`; once this returns, later claim changes
    /// cannot race the Storage open that immediately follows it.
    fn upsert_with_storage_claim(
        &self,
        info_hash: [u8; 20],
        name: String,
        torrent_bytes: Vec<u8>,
        download_dir: &Path,
        preallocate: bool,
        initial_file_renames: &[(usize, String)],
    ) -> Result<(), String> {
        let mut normalized_renames = initial_file_renames.to_vec();
        normalized_renames.sort_unstable_by_key(|(index, _)| *index);
        if normalized_renames
            .windows(2)
            .any(|pair| pair[0].0 == pair[1].0)
            || normalized_renames
                .iter()
                .any(|(_, name)| !valid_renamed_file_name(name))
        {
            return Err("invalid storage claim file rename".to_string());
        }

        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard.get(&info_hash).cloned();
        let (
            label,
            completion_state,
            completion_move_dir,
            pending_delete,
            file_renames,
            pending_file_rename,
        ) = previous
            .as_ref()
            .map(|entry| {
                (
                    entry.label.clone(),
                    entry.completion_state,
                    entry.completion_move_dir.clone(),
                    entry.pending_delete,
                    if entry.file_renames.is_empty() {
                        normalized_renames.clone()
                    } else {
                        entry.file_renames.clone()
                    },
                    entry.pending_file_rename.clone(),
                )
            })
            .unwrap_or_else(|| {
                (
                    String::new(),
                    CompletionState::None,
                    None,
                    false,
                    normalized_renames,
                    None,
                )
            });
        let proposed = SessionEntry {
            info_hash,
            name,
            torrent_bytes,
            download_dir: download_dir.to_path_buf(),
            preallocate,
            label,
            completion_state,
            completion_move_dir,
            pending_delete,
            file_renames,
            pending_file_rename,
        };
        storage_claims_for_session_entry(&proposed)?;
        ensure_session_storage_claim_available(&guard, &proposed)?;
        guard.insert(info_hash, proposed);
        if let Err(err) = save_session(&self.path, &guard) {
            match previous {
                Some(entry) => {
                    guard.insert(info_hash, entry);
                }
                None => {
                    guard.remove(&info_hash);
                }
            }
            return Err(format!("session save failed: {err}"));
        }
        Ok(())
    }

    fn set_label(&self, info_hash: [u8; 20], label: &str) -> Result<(), String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.label == label {
            return Ok(());
        }
        guard
            .get_mut(&info_hash)
            .ok_or_else(|| "torrent session metadata disappeared".to_string())?
            .label = label.to_string();
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(())
    }

    fn completion_state(&self, info_hash: [u8; 20]) -> Option<CompletionState> {
        let guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.get(&info_hash).map(|entry| entry.completion_state)
    }

    fn completion_move_dir(&self, info_hash: [u8; 20]) -> Option<PathBuf> {
        let guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard
            .get(&info_hash)
            .and_then(|entry| entry.completion_move_dir.clone())
    }

    fn begin_completion(
        &self,
        info_hash: [u8; 20],
        move_dir: Option<&Path>,
    ) -> Result<bool, String> {
        let move_dir = match move_dir {
            Some(path) if path.is_absolute() => Some(path.to_path_buf()),
            Some(path) => Some(
                env::current_dir()
                    .map_err(|err| format!("completion move current directory failed: {err}"))?
                    .join(path),
            ),
            None => None,
        };
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.pending_delete || previous.completion_state != CompletionState::None {
            return Ok(false);
        }
        let mut proposed = previous.clone();
        proposed.completion_state = CompletionState::Pending;
        proposed.completion_move_dir = move_dir;
        storage_claims_for_session_entry(&proposed)?;
        ensure_session_storage_claim_available(&guard, &proposed)?;
        guard.insert(info_hash, proposed);
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }

    fn transition_completion_state(
        &self,
        info_hash: [u8; 20],
        expected: CompletionState,
        next: CompletionState,
    ) -> Result<bool, String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.pending_delete || previous.completion_state != expected {
            return Ok(false);
        }
        if expected == next {
            return Ok(true);
        }
        let entry = guard
            .get_mut(&info_hash)
            .ok_or_else(|| "torrent session metadata disappeared".to_string())?;
        entry.completion_state = next;
        if next == CompletionState::Done {
            entry.completion_move_dir = None;
        }
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }

    fn commit_completion_move(
        &self,
        info_hash: [u8; 20],
        download_dir: &Path,
    ) -> Result<bool, String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let previous = guard
            .get(&info_hash)
            .cloned()
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if previous.pending_delete || previous.completion_state != CompletionState::Pending {
            return Ok(false);
        }
        let entry = guard
            .get_mut(&info_hash)
            .ok_or_else(|| "torrent session metadata disappeared".to_string())?;
        entry.download_dir = download_dir.to_path_buf();
        entry.completion_state = CompletionState::Done;
        entry.completion_move_dir = None;
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }

    fn remove(&self, info_hash: [u8; 20]) -> Result<bool, String> {
        let mut guard = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(previous) = guard.remove(&info_hash) else {
            return Ok(false);
        };
        if let Err(err) = save_session(&self.path, &guard) {
            guard.insert(info_hash, previous);
            return Err(format!("session save failed: {err}"));
        }
        Ok(true)
    }
}

fn spawn_on_complete_script(
    script: &Path,
    torrent_name: &str,
    torrent_dir: &Path,
    info_hash: [u8; 20],
    torrent_size: u64,
) {
    let script = script.to_path_buf();
    let torrent_name = torrent_name.to_string();
    let torrent_dir = torrent_dir.display().to_string();
    let torrent_hash = hex(&info_hash);
    if let Err(err) = thread::Builder::new()
        .name("rustorrent-on-complete".to_string())
        .spawn(move || {
            match std::process::Command::new(&script)
                .env("TORRENT_NAME", &torrent_name)
                .env("TORRENT_DIR", &torrent_dir)
                .env("TORRENT_HASH", &torrent_hash)
                .env("TORRENT_SIZE", torrent_size.to_string())
                .status()
            {
                Ok(status) => {
                    if !status.success() {
                        log_warn!("on-complete script exited {status}");
                    }
                }
                Err(err) => {
                    log_warn!("on-complete script error: {err}");
                }
            }
        })
    {
        // Completion is deliberately recorded before spawning. A launch failure is
        // therefore at-most-once, rather than risking duplicate external effects.
        log_warn!("on-complete worker could not start: {err}");
    }
}

impl RateLimiter {
    fn new(limit_bps: u64) -> Self {
        Self {
            limit_bps: AtomicU64::new(limit_bps),
            state: Mutex::new(RateState {
                allowance: limit_bps as f64,
                last: Instant::now(),
            }),
        }
    }

    fn limit_bps(&self) -> u64 {
        self.limit_bps.load(Ordering::SeqCst)
    }

    fn set_limit_bps(&self, limit_bps: u64) {
        self.limit_bps.store(limit_bps, Ordering::SeqCst);
        if let Ok(mut state) = self.state.lock() {
            // A live limit change starts a fresh bucket. In particular, clear
            // any debt accumulated under the previous limit so a newly
            // enabled or raised limit takes effect immediately.
            state.allowance = limit_bps as f64;
            state.last = Instant::now();
        }
    }

    fn throttle(&self, bytes: usize) {
        let limit_bps = self.limit_bps();
        if limit_bps == 0 || bytes == 0 {
            return;
        }
        let delay = self.reserve_delay(bytes, limit_bps, Instant::now());
        if !delay.is_zero() {
            sleep_with_shutdown(delay);
        }
    }

    fn reserve_delay(&self, bytes: usize, limit_bps: u64, now: Instant) -> Duration {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return Duration::ZERO,
        };
        let elapsed = now.duration_since(state.last).as_secs_f64();
        let capacity = limit_bps as f64;
        state.allowance = (state.allowance + elapsed * capacity).min(capacity);
        state.allowance -= bytes as f64;
        let sleep_secs = if state.allowance < 0.0 {
            -state.allowance / capacity
        } else {
            0.0
        };
        state.last = now;
        drop(state);
        // Keep the negative allowance as reserved debt. Concurrent callers
        // then queue behind one another instead of sleeping for the same
        // interval and all waking together, which would multiply the limit by
        // the number of active peers.
        if sleep_secs.is_finite() && sleep_secs > 0.0 {
            Duration::from_secs_f64(sleep_secs)
        } else {
            Duration::ZERO
        }
    }
}

impl InboundConfig {
    fn set_max_handlers(&self, max_handlers: usize) {
        self.max_handlers.store(max_handlers, Ordering::SeqCst);
    }

    fn try_acquire_handler_slot(&self) -> Option<InboundHandlerGuard> {
        let max_handlers = self.max_handlers.load(Ordering::SeqCst);
        if max_handlers == 0 {
            return Some(InboundHandlerGuard { active: None });
        }
        loop {
            let current = self.active_handlers.load(Ordering::SeqCst);
            if current >= max_handlers {
                return None;
            }
            if self
                .active_handlers
                .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Some(InboundHandlerGuard {
                    active: Some(Arc::clone(&self.active_handlers)),
                });
            }
        }
    }
}

impl Drop for InboundHandlerGuard {
    fn drop(&mut self) {
        if let Some(active) = self.active.take() {
            active.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

impl PeerCancellationGuard {
    fn new(registry: &PeerCancellationRegistry, peer_tag: u64, stream: &PeerStream) -> Self {
        let guard = Self {
            registry: Arc::clone(registry),
            peer_tag,
        };
        guard.replace_stream(stream);
        guard
    }

    fn replace_stream(&self, stream: &PeerStream) {
        let mut registry = lock_or_recover(&self.registry);
        if let Some(stream) = stream
            .tcp_stream()
            .and_then(|stream| stream.try_clone().ok())
        {
            registry.insert(self.peer_tag, stream);
        } else {
            registry.remove(&self.peer_tag);
        }
    }
}

impl Drop for PeerCancellationGuard {
    fn drop(&mut self) {
        lock_or_recover(&self.registry).remove(&self.peer_tag);
    }
}

fn cancel_peer_connections(registry: &PeerCancellationRegistry) {
    let registry = lock_or_recover(registry);
    for stream in registry.values() {
        let _ = stream.shutdown(Shutdown::Both);
    }
}

impl PeerSlots {
    fn new(max: usize) -> Self {
        Self {
            max: AtomicUsize::new(max),
            active: AtomicUsize::new(0),
        }
    }

    fn set_max(&self, max: usize) {
        self.max.store(max, Ordering::SeqCst);
    }

    fn acquire(&self, stop_flag: &AtomicBool) -> bool {
        loop {
            if torrent_stop_requested(stop_flag) {
                return false;
            }
            let max = self.max.load(Ordering::SeqCst);
            let current = self.active.load(Ordering::SeqCst);
            if max == 0 || current < max {
                if self
                    .active
                    .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    return true;
                }
            } else {
                sleep_with_shutdown_or_stop(Duration::from_millis(50), stop_flag);
            }
        }
    }

    fn release(&self) {
        self.active.fetch_sub(1, Ordering::SeqCst);
    }
}

impl ActiveTorrentGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self { counter }
    }
}

impl InFlightTorrentGuard {
    fn acquire(
        reservations: &InFlightTorrents,
        info_hash: [u8; 20],
        torrent_id: u64,
    ) -> Result<Self, String> {
        let mut guard = lock_or_recover(reservations);
        if guard.contains_key(&info_hash) {
            return Err("torrent is already loading or active".to_string());
        }
        guard.insert(info_hash, torrent_id);
        drop(guard);
        Ok(Self {
            reservations: Arc::clone(reservations),
            info_hash,
            torrent_id,
        })
    }
}

impl Drop for ActiveTorrentGuard {
    fn drop(&mut self) {
        let _ = self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Drop for InFlightTorrentGuard {
    fn drop(&mut self) {
        let mut guard = lock_or_recover(&self.reservations);
        if guard.get(&self.info_hash) == Some(&self.torrent_id) {
            guard.remove(&self.info_hash);
        }
    }
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        let is_ours = read_file_limited(&self.path, MAX_PID_FILE_BYTES, true)
            .ok()
            .and_then(|contents| String::from_utf8(contents).ok())
            .is_some_and(|contents| contents.trim() == self.pid.to_string());
        if is_ours {
            let _ = remove_file_bound(&self.path);
        }
    }
}

impl UploadManager {
    fn new(max_unchoked: usize) -> Self {
        let now = Instant::now();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0);
        Self {
            inner: Mutex::new(UploadState {
                peers: HashMap::new(),
                unchoked: HashSet::new(),
                last_schedule: now,
                last_optimistic: now - OPTIMISTIC_UNCHOKE_INTERVAL,
                optimistic_peer: None,
                rng: seed ^ 0x9e37_79b9_7f4a_7c15,
            }),
            max_unchoked: max_unchoked.max(1),
        }
    }

    fn register(&self, peer_id: u64) {
        if let Ok(mut state) = self.inner.lock() {
            state.peers.insert(
                peer_id,
                PeerUploadInfo {
                    interested: false,
                    uploaded_total: 0,
                    last_uploaded_total: 0,
                    rate: 0,
                    downloaded_total: 0,
                    last_downloaded_total: 0,
                    download_rate: 0,
                },
            );
        }
    }

    fn unregister(&self, peer_id: u64) {
        if let Ok(mut state) = self.inner.lock() {
            state.peers.remove(&peer_id);
            state.unchoked.remove(&peer_id);
            if state.optimistic_peer == Some(peer_id) {
                state.optimistic_peer = None;
            }
        }
    }

    fn set_interested(&self, peer_id: u64, interested: bool) {
        if let Ok(mut state) = self.inner.lock() {
            if let Some(info) = state.peers.get_mut(&peer_id) {
                let was_interested = info.interested;
                info.interested = interested;
                // Immediately reschedule when interest changes so slots
                // are allocated/freed without waiting for the 10-second timer
                if interested != was_interested {
                    let now = Instant::now();
                    reschedule_uploads(&mut state, self.max_unchoked, now);
                }
            }
        }
    }

    fn record_upload(&self, peer_id: u64, bytes: u64) {
        if bytes == 0 {
            return;
        }
        if let Ok(mut state) = self.inner.lock() {
            if let Some(info) = state.peers.get_mut(&peer_id) {
                info.uploaded_total = info.uploaded_total.saturating_add(bytes);
            }
        }
    }

    fn record_download(&self, peer_id: u64, bytes: u64) {
        if bytes == 0 {
            return;
        }
        if let Ok(mut state) = self.inner.lock() {
            if let Some(info) = state.peers.get_mut(&peer_id) {
                info.downloaded_total = info.downloaded_total.saturating_add(bytes);
            }
        }
    }

    fn should_unchoke(&self, peer_id: u64) -> bool {
        let mut state = match self.inner.lock() {
            Ok(state) => state,
            Err(_) => return false,
        };
        let now = Instant::now();
        if now.duration_since(state.last_schedule) >= UNCHOKE_INTERVAL {
            reschedule_uploads(&mut state, self.max_unchoked, now);
        }
        state.unchoked.contains(&peer_id)
    }
}

fn reschedule_uploads(state: &mut UploadState, max_unchoked: usize, now: Instant) {
    state.last_schedule = now;
    let prev_unchoked = state.unchoked.clone();
    let mut any_downloading = false;
    for info in state.peers.values_mut() {
        info.rate = info.uploaded_total.saturating_sub(info.last_uploaded_total);
        info.last_uploaded_total = info.uploaded_total;
        info.download_rate = info
            .downloaded_total
            .saturating_sub(info.last_downloaded_total);
        info.last_downloaded_total = info.downloaded_total;
        if info.download_rate > 0 {
            any_downloading = true;
        }
    }
    // BEP 3: when leeching, sort by download_rate (what peer gives us).
    // When seeding (no downloads), sort by upload_rate (reward fastest served).
    let mut candidates: Vec<(u64, u64)> = Vec::new();
    for (peer_id, info) in state.peers.iter() {
        if info.interested {
            let base_rate = if any_downloading {
                info.download_rate
            } else {
                info.rate
            };
            // Hysteresis: give previously-unchoked peers a 10% bonus to
            // prevent rapid choke/unchoke oscillation between similar peers.
            let effective_rate = if prev_unchoked.contains(peer_id) {
                base_rate.saturating_add(base_rate / 10)
            } else {
                base_rate
            };
            candidates.push((*peer_id, effective_rate));
        }
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.1));
    state.unchoked.clear();
    for (peer_id, _) in candidates.iter().take(max_unchoked) {
        state.unchoked.insert(*peer_id);
    }

    let remaining = if candidates.len() > max_unchoked {
        &candidates[max_unchoked..]
    } else {
        &[]
    };
    if now.duration_since(state.last_optimistic) >= OPTIMISTIC_UNCHOKE_INTERVAL {
        state.optimistic_peer = if remaining.is_empty() {
            None
        } else {
            let idx = (next_rng(state) as usize) % remaining.len();
            Some(remaining[idx].0)
        };
        state.last_optimistic = now;
    }

    if let Some(peer_id) = state.optimistic_peer {
        if state
            .peers
            .get(&peer_id)
            .map(|info| info.interested)
            .unwrap_or(false)
        {
            state.unchoked.insert(peer_id);
        } else {
            state.optimistic_peer = None;
        }
    }
}

fn next_rng(state: &mut UploadState) -> u64 {
    let mut x = state.rng;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    state.rng = x;
    x
}

fn inbound_handler_slots(max_peers_global: usize) -> usize {
    if max_peers_global == 0 {
        return 0;
    }
    max_peers_global
        .saturating_mul(2)
        .clamp(MIN_INBOUND_HANDLER_SLOTS, MAX_INBOUND_HANDLER_SLOTS)
}

#[cfg(not(windows))]
fn acquire_session_lock(download_dir: &Path) -> Result<SessionLocks, String> {
    let lock_path = download_dir.join(".rustorrent.lock");
    let mut options = fs::OpenOptions::new();
    options.create(true).read(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(&lock_path)
        .map_err(|err| format!("failed to open session lock: {err}"))?;
    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to inspect session lock: {err}"))?;
    if !metadata.is_file() {
        return Err("session lock is not a regular file".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err("session lock must not be hard-linked".to_string());
        }
        let path_metadata = fs::symlink_metadata(&lock_path)
            .map_err(|err| format!("failed to inspect session lock path: {err}"))?;
        if !path_metadata.is_file()
            || path_metadata.dev() != metadata.dev()
            || path_metadata.ino() != metadata.ino()
        {
            return Err("session lock path changed while opening".to_string());
        }
    }
    file.try_lock().map_err(|err| {
        format!(
            "another instance is using {} ({err})",
            download_dir.display()
        )
    })?;
    #[cfg(unix)]
    let state_directory = {
        let directory = state_dir::open_lock_directory(download_dir)
            .map_err(|err| format!("failed to open pinned state-directory lock: {err}"))?;
        directory.try_lock().map_err(|err| {
            format!(
                "another instance is using {} ({err})",
                download_dir.display()
            )
        })?;
        directory
    };
    Ok(SessionLocks {
        _legacy: file,
        #[cfg(unix)]
        _state_directory: state_directory,
    })
}

#[cfg(windows)]
fn acquire_session_lock(download_dir: &Path) -> Result<SessionLocks, String> {
    let lock = state_dir::acquire_session_lock(download_dir).map_err(|err| {
        format!(
            "another instance may be using {} or its state is unsafe ({err})",
            download_dir.display()
        )
    })?;
    Ok(SessionLocks { _windows: lock })
}

pub(crate) fn ensure_private_state_directory(download_dir: &Path) -> Result<(), String> {
    #[cfg(any(unix, windows))]
    {
        state_dir::ensure(download_dir)
            .map_err(|err| format!("failed to secure state directory: {err}"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        fs::create_dir_all(download_dir).map_err(|err| {
            format!(
                "failed to create download directory {}: {err}",
                download_dir.display()
            )
        })?;
        let canonical_download = fs::canonicalize(download_dir).map_err(|err| {
            format!(
                "failed to resolve download directory {}: {err}",
                download_dir.display()
            )
        })?;
        let state_dir = download_dir.join(".rustorrent");
        match fs::symlink_metadata(&state_dir) {
            Ok(metadata) => validate_state_directory_metadata(&state_dir, &metadata)?,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                #[allow(unused_mut)]
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                builder
                    .create(&state_dir)
                    .map_err(|err| format!("failed to create state directory: {err}"))?;
            }
            Err(err) => return Err(format!("failed to inspect state directory: {err}")),
        }

        let path_metadata = fs::symlink_metadata(&state_dir)
            .map_err(|err| format!("failed to inspect state directory: {err}"))?;
        validate_state_directory_metadata(&state_dir, &path_metadata)?;
        let canonical_state = fs::canonicalize(&state_dir)
            .map_err(|err| format!("failed to resolve state directory: {err}"))?;
        if canonical_state.parent() != Some(canonical_download.as_path())
            || !canonical_state
                .file_name()
                .is_some_and(state_dir::is_state_directory_name)
        {
            return Err("state directory escapes the download directory".to_string());
        }
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn validate_state_directory_metadata(path: &Path, metadata: &fs::Metadata) -> Result<(), String> {
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "state path {} is not a real directory",
            path.display()
        ));
    }
    Ok(())
}

fn read_file_limited(path: &Path, limit: usize, no_follow: bool) -> io::Result<Vec<u8>> {
    #[cfg(any(unix, windows))]
    if state_dir::is_state_file_path(path) {
        return state_dir::read_limited(path, limit);
    }
    if no_follow {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path is not a regular file",
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "path is a filesystem reparse point",
                ));
            }
        }
    }

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let flags = if no_follow { libc::O_NOFOLLOW } else { 0 };
        options.custom_flags(flags | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    if no_follow {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a regular file",
        ));
    }
    if metadata.len() > limit as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file exceeds {limit} byte limit"),
        ));
    }
    let mut data = Vec::with_capacity((metadata.len() as usize).min(limit));
    file.take((limit + 1) as u64).read_to_end(&mut data)?;
    if data.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file exceeds {limit} byte limit"),
        ));
    }
    Ok(data)
}

fn remove_file_bound(path: &Path) -> io::Result<()> {
    #[cfg(any(unix, windows))]
    if state_dir::is_state_file_path(path) {
        return state_dir::remove_file(path);
    }
    fs::remove_file(path)
}

fn open_private_log_file(path: &Path) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|err| format!("failed to open log file: {err}"))?;
    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to inspect log file: {err}"))?;
    if !metadata.is_file() {
        return Err("log path is not a regular file".to_string());
    }
    let path_metadata =
        fs::symlink_metadata(path).map_err(|err| format!("failed to inspect log path: {err}"))?;
    if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
        return Err("log path is not a regular file".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.nlink() != 1 {
            return Err("log file must not be hard-linked".to_string());
        }
        if path_metadata.dev() != metadata.dev() || path_metadata.ino() != metadata.ino() {
            return Err("log path changed while opening".to_string());
        }
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|err| format!("failed to secure log file permissions: {err}"))?;
    }
    Ok(file)
}

fn run() -> Result<(), String> {
    install_signal_handlers();
    install_panic_logger();
    let args = parse_args()?;

    fs::create_dir_all(&args.download_dir).map_err(|err| {
        format!(
            "failed to create download directory {}: {err}",
            args.download_dir.display()
        )
    })?;

    // Initialize log file if --log was specified
    if let Some(log_path) = args.log_path.as_ref() {
        let file = open_private_log_file(log_path)?;
        let _ = LOG_FILE.set(Mutex::new(file));
    }
    log_info!(
        "peer profile: {} (global={}, per_torrent={}, numwant={}, metadata={})",
        args.peer_profile.as_str(),
        args.max_peers_global,
        args.max_peers_torrent,
        args.numwant,
        args.metadata_peer_limit
    );

    // Daemon mode: fork and detach on Unix
    #[cfg(unix)]
    if args.daemon {
        extern "C" {
            fn fork() -> i32;
            fn setsid() -> i32;
        }
        let pid = unsafe { fork() };
        if pid < 0 {
            return Err("fork failed".to_string());
        }
        if pid > 0 {
            // Parent: exit immediately
            std::process::exit(0);
        }
        // Child: create new session
        if unsafe { setsid() } < 0 {
            return Err("setsid failed".to_string());
        }
        // Redirect stdin to /dev/null
        if let Ok(devnull) = std::fs::File::open("/dev/null") {
            use std::os::unix::io::AsRawFd;
            extern "C" {
                fn dup2(oldfd: i32, newfd: i32) -> i32;
            }
            unsafe {
                dup2(devnull.as_raw_fd(), 0);
            }
        }
    }

    // Hold an operating-system lock for the process lifetime. A PID-file-only
    // check races when two instances start together and can also reject an
    // unrelated process after PID reuse.
    let _lock_file = acquire_session_lock(&args.download_dir)?;
    ensure_private_state_directory(&args.download_dir)?;

    // Write the optional PID file only after this instance owns the session.
    let _pid_file_guard = if let Some(pid_path) = args.pid_file.as_ref() {
        let pid = std::process::id();
        write_atomic_file(
            pid_path,
            format!("{pid}\n").as_bytes(),
            "PID file",
            false,
            true,
        )?;
        Some(PidFileGuard {
            path: pid_path.clone(),
            pid,
        })
    } else {
        None
    };

    // Initialize seed ratio from CLI args
    if args.seed_ratio > 0.0 {
        SEED_RATIO_BITS.store(args.seed_ratio.to_bits(), Ordering::SeqCst);
    }
    if args.max_seed_time > 0 {
        MAX_SEED_TIME_SECS.store(args.max_seed_time.saturating_mul(60), Ordering::SeqCst);
    }
    if let Some(script) = args.on_complete.clone() {
        let _ = ON_COMPLETE_SCRIPT.set(script);
    }
    if args.super_seed {
        SUPER_SEED.store(true, Ordering::SeqCst);
    }
    if !args.throttle_groups.is_empty() {
        let groups = args
            .throttle_groups
            .iter()
            .map(|(name, down, up)| ThrottleGroup {
                name: name.clone(),
                down: Arc::new(RateLimiter::new(*down)),
                up: Arc::new(RateLimiter::new(*up)),
            })
            .collect();
        let _ = THROTTLE_GROUPS.set(Mutex::new(groups));
    }
    if !args.ratio_groups.is_empty() {
        let groups = args
            .ratio_groups
            .iter()
            .map(|(name, ratio, action)| RatioGroup {
                name: name.clone(),
                ratio: *ratio,
                action: action.clone(),
            })
            .collect();
        let _ = RATIO_GROUPS.set(Mutex::new(groups));
    }
    if !args.schedules.is_empty() {
        let entries = args
            .schedules
            .iter()
            .map(|(interval, command)| ScheduleEntry {
                interval_secs: *interval,
                command: command.clone(),
                last_run: Instant::now(),
            })
            .collect();
        let _ = SCHEDULES.set(Mutex::new(entries));
    }

    let global_down = Arc::new(RateLimiter::new(args.download_rate));
    let global_up = Arc::new(RateLimiter::new(args.upload_rate));
    let peer_settings = Arc::new(PeerRuntimeSettings::new(
        args.peer_profile,
        args.numwant,
        args.metadata_peer_limit,
        args.max_peers_global,
        args.max_peers_torrent,
    ));
    let peer_slots = Arc::new(PeerSlots::new(peer_settings.max_peers_global()));
    let global_piece_buffer_budget =
        Arc::new(piece::PieceBufferBudget::new(MAX_GLOBAL_PIECE_BUFFER_BYTES));
    let active_torrents = Arc::new(AtomicUsize::new(0));
    let session_store = Arc::new(SessionStore::load(&args.download_dir)?);

    let state = Arc::new(Mutex::new(ui::UiState::default()));
    let ui_state = Some(state.clone());
    update_ui(&ui_state, |ui| {
        ui.global_download_limit_bps = args.download_rate;
        ui.global_upload_limit_bps = args.upload_rate;
        ui.seed_ratio = args.seed_ratio;
        ui.peer_profile = args.peer_profile.as_str().to_string();
        ui.peer_profile_global_limit = args.max_peers_global;
        ui.peer_profile_torrent_limit = args.max_peers_torrent;
        ui.peer_profile_numwant = args.numwant;
        ui.incoming_port = args.port;
        ui.natpmp_status = "pending".to_string();
        ui.upnp_status = "pending".to_string();
        ui.proxy_label = match &args.proxy {
            Some(proxy::ProxyConfig::Socks5 { host, port }) => format!("socks5://{host}:{port}"),
            Some(proxy::ProxyConfig::Http { host, port }) => format!("http://{host}:{port}"),
            None => String::new(),
        };
    });
    let (cmd_tx, cmd_rx) = mpsc::channel::<ui::UiCommand>();
    if args.ui {
        search::set_network_enabled(args.proxy.is_none());
        ui::start(args.ui_addr.clone(), state.clone(), Some(cmd_tx.clone()))
            .map_err(|err| format!("UI bind {} failed: {err}", args.ui_addr))?;
        log_info!("ui: http://{}", args.ui_addr);
    }
    if args.ui {
        if let Err(err) = search::prepare(&args.download_dir) {
            log_warn!("search prepare error: {err}");
        }
    }
    if args.ui {
        let search_root = args.download_dir.clone();
        spawn_detached("search-refresh", move || {
            if let Err(err) = search::refresh_plugins() {
                log_warn!("search refresh error: {err}");
                // Re-prepare to reset state, but don't recurse into refresh_plugins
                let _ = search::prepare(&search_root);
            }
        });
    }

    let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
    let progress_handle = if !args.daemon && !args.tui {
        match start_console_progress(state.clone(), registry.clone()) {
            Ok(handle) => Some(handle),
            Err(err) => {
                log_warn!("{err}");
                None
            }
        }
    } else {
        None
    };
    let tui_handle = if args.tui {
        match start_tui(state.clone(), cmd_tx.clone()) {
            Ok(handle) => Some(handle),
            Err(err) => {
                log_warn!("{err}");
                None
            }
        }
    } else {
        None
    };
    let ip_filter = if let Some(path) = args.blocklist_path.as_ref() {
        match IpFilter::from_file(path) {
            Ok(filter) => Some(Arc::new(filter)),
            Err(err) => {
                log_warn!("blocklist error: {err}");
                None
            }
        }
    } else {
        None
    };
    if let Some(path) = args.geoip_db.as_ref() {
        match geoip::GeoIpDb::load(path) {
            Ok(db) => {
                log_info!("geoip loaded {} entries from {}", db.len(), path.display());
                let _ = GEOIP_DB.set(db);
            }
            Err(err) => {
                log_warn!("geoip error: {err}");
            }
        }
    }
    {
        let rss_path = args.download_dir.join(".rustorrent").join("rss.benc");
        let mut rss_state = if rss::saved_state_exists(&rss_path) {
            rss::load_rss_state(&rss_path).unwrap_or_else(|err| {
                log_warn!("rss load error: {err}");
                rss::RssState::new()
            })
        } else {
            rss::RssState::new()
        };
        if rss_state.feeds.len().saturating_add(args.rss_feeds.len()) > rss::MAX_RSS_FEEDS
            || rss_state.rules.len().saturating_add(args.rss_rules.len()) > rss::MAX_RSS_RULES
        {
            return Err(
                "RSS state plus command-line additions exceeds the configured limit".to_string(),
            );
        }
        for url in &args.rss_feeds {
            if !rss_state.feeds.iter().any(|f| f.url == *url) {
                rss_state.feeds.push(rss::RssFeed {
                    url: url.clone(),
                    title: String::new(),
                    items: Vec::new(),
                    last_poll: 0,
                    poll_interval_secs: args.rss_interval,
                });
                log_info!("rss added feed: {}", safe_network_url_label(url));
            }
        }
        for (feed_url, pattern) in &args.rss_rules {
            rss_state.rules.push(rss::RssRule {
                name: pattern.clone(),
                feed_url: feed_url.clone(),
                pattern: pattern.clone(),
            });
        }
        let _ = RSS_STATE.set(Mutex::new(rss_state));
    }
    let inbound = InboundConfig {
        encryption: args.encryption,
        ip_filter: ip_filter.clone(),
        max_handlers: Arc::new(AtomicUsize::new(inbound_handler_slots(
            peer_settings.max_peers_global(),
        ))),
        active_handlers: Arc::new(AtomicUsize::new(0)),
    };
    let direct_discovery = args.proxy.is_none();
    let inbound_listener_handle = if direct_discovery {
        match start_inbound_listener(args.port, registry.clone(), inbound.clone()) {
            Ok(handle) => Some(handle),
            Err(err) => {
                log_warn!("inbound listener failed: {err}");
                None
            }
        }
    } else {
        None
    };
    let mut utp_listener_handle = None;
    let utp_connector = if args.enable_utp && direct_discovery {
        let (connector, listener) = utp::start(args.port);
        match start_utp_listener(listener, registry.clone(), inbound.clone()) {
            Ok(handle) => utp_listener_handle = Some(handle),
            Err(err) => {
                log_warn!("uTP listener failed: {err}");
            }
        }
        Some(connector)
    } else {
        None
    };
    let dht = if direct_discovery {
        // uTP and DHT are different protocols over UDP. Until they share a
        // demultiplexing socket, reserve the configured/mapped UDP port for
        // uTP and let DHT advertise its own ephemeral source port.
        let dht_port = if args.enable_utp { 0 } else { args.port };
        dht::start(dht_port, &args.download_dir)
    } else {
        dht::disabled()
    };
    let lpd = if direct_discovery {
        lpd::start()
    } else {
        lpd::disabled()
    };
    if direct_discovery {
        let port = args.port;
        let mapping_ui = ui_state.clone();
        spawn_detached("nat-pmp-mapping", move || {
            record_port_mapping_result(&mapping_ui, "nat-pmp", port, natpmp::map_port(port, 3600));
        });
        let port = args.port;
        let mapping_ui = ui_state.clone();
        spawn_detached("upnp-mapping", move || {
            run_port_mapping_with_retries(
                &mapping_ui,
                "upnp",
                port,
                &[
                    Duration::from_secs(3),
                    Duration::from_secs(10),
                    Duration::from_secs(30),
                ],
                || upnp::map_port(port),
            );
        });
    } else {
        update_ui(&ui_state, |state| {
            state.natpmp_status = "disabled while proxy is active".to_string();
            state.upnp_status = "disabled while proxy is active".to_string();
        });
    }

    let mut queue: VecDeque<TorrentRequest> = VecDeque::new();
    let in_flight: InFlightTorrents = Arc::new(Mutex::new(HashMap::new()));
    let mut next_id = 1u64;
    let (rss_poll_tx, rss_poll_rx) = mpsc::channel::<RssPollResult>();
    let (rss_download_tx, rss_download_rx) = mpsc::channel::<RssDownloadResult>();
    let mut rss_poll_inflight: HashSet<String> = HashSet::new();
    let mut rss_download_inflight: HashSet<String> = HashSet::new();
    restore_session_entries(&session_store, &mut queue, &ui_state, &mut next_id);
    if let Some(link) = args.magnet.clone() {
        let request = TorrentRequest {
            id: next_id,
            source: TorrentSource::Magnet(link),
            download_dir: args.download_dir.clone(),
            preallocate: args.preallocate,
            initial_label: String::new(),
        };
        if enqueue_request_if_new(
            &registry,
            &mut queue,
            &session_store,
            &in_flight,
            &ui_state,
            request,
            None,
        ) {
            next_id = next_id.saturating_add(1);
        }
    }

    if let Some(path) = args.torrent_path.clone() {
        let request = TorrentRequest {
            id: next_id,
            source: TorrentSource::Path(path),
            download_dir: args.download_dir.clone(),
            preallocate: args.preallocate,
            initial_label: String::new(),
        };
        if enqueue_request_if_new(
            &registry,
            &mut queue,
            &session_store,
            &in_flight,
            &ui_state,
            request,
            None,
        ) {
            next_id = next_id.saturating_add(1);
        }
    }

    update_idle_state(&ui_state, &args, queue.len());

    let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut last_watch_scan = Instant::now();

    loop {
        reap_finished_workers(&mut handles, "torrent");
        if !args.watch_dirs.is_empty() && last_watch_scan.elapsed() >= Duration::from_secs(5) {
            for watch_dir in &args.watch_dirs {
                scan_watch_dir(
                    watch_dir,
                    &mut queue,
                    &ui_state,
                    &mut next_id,
                    &args.download_dir,
                    args.preallocate,
                    &registry,
                    &session_store,
                    &in_flight,
                );
            }
            last_watch_scan = Instant::now();
        }

        drain_ui_commands(
            &cmd_rx,
            &mut queue,
            &ui_state,
            &args,
            &mut next_id,
            &registry,
            &session_store,
            &global_down,
            &global_up,
            &peer_settings,
            &peer_slots,
            &inbound,
            &in_flight,
        );

        // Scheduled commands
        if let Some(schedules) = SCHEDULES.get() {
            if let Ok(mut sched) = schedules.lock() {
                for entry in sched.iter_mut() {
                    if entry.last_run.elapsed().as_secs() >= entry.interval_secs {
                        execute_schedule_command(
                            &entry.command,
                            &global_down,
                            &global_up,
                            &registry,
                        );
                        entry.last_run = Instant::now();
                    }
                }
            }
        }

        // RSS feed polling
        schedule_rss_polls(&args, &rss_poll_tx, &mut rss_poll_inflight);
        drain_rss_poll_results(
            &args,
            &rss_poll_rx,
            &rss_download_tx,
            &mut queue,
            &ui_state,
            &mut next_id,
            &mut rss_poll_inflight,
            &mut rss_download_inflight,
            &registry,
            &session_store,
            &in_flight,
        );
        drain_rss_download_results(
            &args,
            &rss_download_rx,
            &mut queue,
            &ui_state,
            &mut next_id,
            &mut rss_download_inflight,
            &registry,
            &session_store,
            &in_flight,
        );

        let can_start = args.max_active_torrents == 0
            || active_torrents.load(Ordering::SeqCst) < args.max_active_torrents;
        if can_start {
            if let Some(mut request) = queue.pop_front() {
                let request_id = request.id;
                let in_flight_guard = match freeze_request_source(&mut request) {
                    Ok(info_hash) => {
                        InFlightTorrentGuard::acquire(&in_flight, info_hash, request_id)
                    }
                    Err(err) => Err(err),
                };
                let in_flight_guard = match in_flight_guard {
                    Ok(guard) => guard,
                    Err(err) => {
                        update_ui(&ui_state, |state| {
                            state.queue_len = queue.len();
                            state.status = "error".to_string();
                            state.last_error = err.clone();
                            update_torrent_entry(state, request_id, |torrent| {
                                torrent.status = "error".to_string();
                                torrent.last_error = err.clone();
                            });
                        });
                        continue;
                    }
                };
                let retry_request = request.clone();
                let is_magnet = matches!(request.source, TorrentSource::Magnet(_));
                let load_status = if is_magnet {
                    "fetching metadata"
                } else {
                    "loading"
                };
                update_ui(&ui_state, |state| {
                    state.queue_len = queue.len();
                    state.status = load_status.to_string();
                    state.last_error.clear();
                    update_torrent_entry(state, request_id, |torrent| {
                        torrent.status = load_status.to_string();
                        torrent.last_error.clear();
                    });
                });
                active_torrents.fetch_add(1, Ordering::SeqCst);
                let active_guard = ActiveTorrentGuard::new(active_torrents.clone());
                let args_clone = args.clone();
                let ui_clone = ui_state.clone();
                let registry_clone = registry.clone();
                let dht_clone = dht.clone();
                let lpd_clone = lpd.clone();
                let session_clone = session_store.clone();
                let utp_clone = utp_connector.clone();
                let filter_clone = ip_filter.clone();
                let global_down = global_down.clone();
                let global_up = global_up.clone();
                let peer_slots = peer_slots.clone();
                let peer_settings = peer_settings.clone();
                let global_piece_buffer_budget = Arc::clone(&global_piece_buffer_budget);
                match thread::Builder::new()
                    .name(format!("torrent-{request_id}"))
                    .spawn(move || {
                        let _in_flight_guard = in_flight_guard;
                        let _guard = active_guard;
                        if let Err(err) = run_torrent(
                            request,
                            &args_clone,
                            &ui_clone,
                            &registry_clone,
                            &session_clone,
                            &dht_clone,
                            &lpd_clone,
                            utp_clone,
                            filter_clone,
                            global_down,
                            global_up,
                            peer_settings,
                            peer_slots,
                            global_piece_buffer_budget,
                        ) {
                            log_warn!("torrent error: {err}");
                            update_ui(&ui_clone, |state| {
                                state.status = "error".to_string();
                                state.last_error = err;
                                let last_error = state.last_error.clone();
                                update_torrent_entry(state, request_id, |torrent| {
                                    torrent.status = "error".to_string();
                                    torrent.last_error = last_error;
                                });
                            });
                        }
                    }) {
                    Ok(handle) => {
                        handles.push(handle);
                        continue;
                    }
                    Err(err) => {
                        log_warn!("torrent worker could not start: {err}");
                        queue.push_front(retry_request);
                        update_ui(&ui_state, |state| {
                            state.queue_len = queue.len();
                            state.status = "queued".to_string();
                            state.last_error = format!("torrent worker could not start: {err}");
                            update_torrent_entry(state, request_id, |torrent| {
                                torrent.status = "queued".to_string();
                                torrent.last_error =
                                    format!("torrent worker could not start: {err}");
                            });
                        });
                    }
                }
            }
        }

        if shutdown_requested() {
            break;
        }

        update_idle_state(&ui_state, &args, queue.len());
        sleep_with_shutdown(Duration::from_millis(200));
    }

    let shutdown_deadline = Instant::now() + TORRENT_WORKER_SHUTDOWN_TIMEOUT;
    for handle in handles {
        join_worker_before(handle, "torrent", shutdown_deadline);
    }
    if let Some(handle) = inbound_listener_handle {
        join_worker_before(handle, "TCP listener", shutdown_deadline);
    }
    if let Some(handle) = utp_listener_handle {
        join_worker_before(handle, "uTP listener", shutdown_deadline);
    }
    if let Some(handle) = progress_handle {
        join_worker_before(handle, "console progress", shutdown_deadline);
    }
    if let Some(handle) = tui_handle {
        join_worker_before(handle, "terminal UI", shutdown_deadline);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn drain_ui_commands(
    rx: &mpsc::Receiver<ui::UiCommand>,
    queue: &mut VecDeque<TorrentRequest>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    args: &Args,
    next_id: &mut u64,
    registry: &SessionRegistry,
    session_store: &Arc<SessionStore>,
    global_down: &Arc<RateLimiter>,
    global_up: &Arc<RateLimiter>,
    peer_settings: &Arc<PeerRuntimeSettings>,
    peer_slots: &Arc<PeerSlots>,
    inbound: &InboundConfig,
    in_flight: &InFlightTorrents,
) {
    loop {
        match rx.try_recv() {
            Ok(cmd) => match cmd {
                ui::UiCommand::AddTorrent {
                    data,
                    download_dir,
                    preallocate,
                    reply,
                } => {
                    let request = TorrentRequest {
                        id: *next_id,
                        source: TorrentSource::Bytes(data),
                        download_dir: normalize_download_dir(download_dir, &args.download_dir),
                        preallocate,
                        initial_label: String::new(),
                    };
                    if let Ok(info_hash) = info_hash_for_source(&request.source) {
                        if is_duplicate_torrent(
                            registry,
                            queue,
                            session_store,
                            in_flight,
                            info_hash,
                        ) {
                            let message = "torrent already added".to_string();
                            update_ui(ui_state, |state| {
                                state.last_error = message.clone();
                            });
                            let _ = reply.send(Err(message));
                            continue;
                        }
                    }
                    *next_id = next_id.saturating_add(1);
                    let torrent_id = request.id;
                    enqueue_request_with_label(
                        queue,
                        ui_state,
                        request,
                        "torrent upload".to_string(),
                    );
                    let _ = reply.send(Ok(ui::UiCommandSuccess::TorrentAdded { torrent_id }));
                }
                ui::UiCommand::AddMagnet {
                    magnet,
                    download_dir,
                    preallocate,
                    reply,
                } => {
                    let request = TorrentRequest {
                        id: *next_id,
                        source: TorrentSource::Magnet(magnet),
                        download_dir: normalize_download_dir(download_dir, &args.download_dir),
                        preallocate,
                        initial_label: String::new(),
                    };
                    if let Ok(info_hash) = info_hash_for_source(&request.source) {
                        if is_duplicate_torrent(
                            registry,
                            queue,
                            session_store,
                            in_flight,
                            info_hash,
                        ) {
                            let message = "torrent already added".to_string();
                            update_ui(ui_state, |state| {
                                state.last_error = message.clone();
                            });
                            let _ = reply.send(Err(message));
                            continue;
                        }
                    }
                    *next_id = next_id.saturating_add(1);
                    let torrent_id = request.id;
                    enqueue_request_with_label(queue, ui_state, request, "magnet link".to_string());
                    let _ = reply.send(Ok(ui::UiCommandSuccess::TorrentAdded { torrent_id }));
                }
                ui::UiCommand::PauseTorrent { torrent_id, reply } => {
                    let result = set_torrent_paused(registry, ui_state, torrent_id, true)
                        .map(|_| ui::UiCommandSuccess::Ok);
                    if let Err(err) = result.as_ref() {
                        log_warn!("pause torrent error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result);
                }
                ui::UiCommand::ResumeTorrent { torrent_id, reply } => {
                    let result = resume_torrent(
                        registry,
                        ui_state,
                        queue,
                        torrent_id,
                        session_store,
                        in_flight,
                    )
                    .map(|_| ui::UiCommandSuccess::Ok);
                    if let Err(err) = result.as_ref() {
                        log_warn!("resume torrent error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result);
                }
                ui::UiCommand::StopTorrent { torrent_id, reply } => {
                    let result = stop_torrent(
                        registry,
                        ui_state,
                        queue,
                        torrent_id,
                        session_store,
                        in_flight,
                    )
                    .map(|_| ui::UiCommandSuccess::Ok);
                    if let Err(err) = result.as_ref() {
                        log_warn!("stop torrent error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result);
                }
                ui::UiCommand::ArchiveTorrent { torrent_id, reply } => {
                    let result = archive_torrent(
                        registry,
                        ui_state,
                        queue,
                        torrent_id,
                        session_store,
                        in_flight,
                    )
                    .map(|_| ui::UiCommandSuccess::Ok);
                    if let Err(err) = result.as_ref() {
                        log_warn!("archive torrent error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result);
                }
                ui::UiCommand::DeleteTorrent {
                    torrent_id,
                    remove_data,
                    reply,
                } => {
                    let result = delete_torrent(
                        registry,
                        ui_state,
                        queue,
                        torrent_id,
                        remove_data,
                        session_store,
                        in_flight,
                    );
                    if let Err(err) = result.as_ref() {
                        log_warn!("delete torrent error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::SetFilePriority {
                    torrent_id,
                    file_index,
                    priority,
                    reply,
                } => {
                    let result =
                        apply_file_priority(registry, ui_state, torrent_id, file_index, priority);
                    if let Err(err) = result.as_ref() {
                        log_warn!("file priority error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::RenameFile {
                    torrent_id,
                    file_index,
                    new_name,
                    reply,
                } => {
                    let result = apply_file_rename(
                        registry,
                        ui_state,
                        session_store,
                        torrent_id,
                        file_index,
                        &new_name,
                    );
                    if let Err(err) = result.as_ref() {
                        log_warn!("file rename error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::SetRateLimits {
                    download_limit_bps,
                    upload_limit_bps,
                    reply,
                } => {
                    global_down.set_limit_bps(download_limit_bps);
                    global_up.set_limit_bps(upload_limit_bps);
                    update_ui(ui_state, |state| {
                        state.global_download_limit_bps = download_limit_bps;
                        state.global_upload_limit_bps = upload_limit_bps;
                    });
                    let _ = reply.send(Ok(ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::RecheckTorrent { torrent_id, reply } => {
                    let result = recheck_torrent(registry, ui_state, torrent_id);
                    if let Err(err) = result.as_ref() {
                        log_warn!("recheck torrent error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::SetSeedRatio { ratio, reply } => {
                    SEED_RATIO_BITS.store(ratio.to_bits(), Ordering::SeqCst);
                    update_ui(ui_state, |state| {
                        state.seed_ratio = ratio;
                    });
                    let _ = reply.send(Ok(ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::SetPeerProfile { profile, reply } => {
                    let result = parse_peer_profile(&profile).map(|profile| {
                        let tuning = peer_settings.apply_profile(profile);
                        peer_slots.set_max(tuning.max_peers_global);
                        inbound.set_max_handlers(inbound_handler_slots(tuning.max_peers_global));
                        log_info!(
                            "peer profile changed via ui: {} (global={}, per_torrent={}, numwant={}, metadata={})",
                            profile.as_str(),
                            tuning.max_peers_global,
                            tuning.max_peers_torrent,
                            tuning.numwant,
                            tuning.metadata_peer_limit
                        );
                        update_ui(ui_state, |state| {
                            state.peer_profile = profile.as_str().to_string();
                            state.peer_profile_global_limit = tuning.max_peers_global;
                            state.peer_profile_torrent_limit = tuning.max_peers_torrent;
                            state.peer_profile_numwant = tuning.numwant;
                        });
                        ui::UiCommandSuccess::Ok
                    });
                    if let Err(err) = result.as_ref() {
                        log_warn!("peer profile error: {err}");
                        update_ui(ui_state, |state| {
                            state.last_error = err.clone();
                        });
                    }
                    let _ = reply.send(result);
                }
                ui::UiCommand::SetLabel {
                    torrent_id,
                    label,
                    reply,
                } => {
                    let result =
                        set_torrent_label(registry, ui_state, session_store, torrent_id, &label);
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::AddTracker {
                    torrent_id,
                    url,
                    reply,
                } => {
                    let result = add_torrent_tracker(registry, ui_state, torrent_id, &url);
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::RemoveTracker {
                    torrent_id,
                    url,
                    reply,
                } => {
                    let result = remove_torrent_tracker(registry, ui_state, torrent_id, &url);
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::AddRssFeed {
                    url,
                    interval,
                    reply,
                } => {
                    let result = rss_add_feed(&url, interval, &args.download_dir);
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::RemoveRssFeed { url, reply } => {
                    let result = rss_remove_feed(&url, &args.download_dir);
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::AddRssRule {
                    name,
                    feed_url,
                    pattern,
                    reply,
                } => {
                    let result = rss_add_rule(&name, &feed_url, &pattern, &args.download_dir);
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
                ui::UiCommand::RemoveRssRule { name, reply } => {
                    let result = rss_remove_rule(&name, &args.download_dir);
                    let _ = reply.send(result.map(|_| ui::UiCommandSuccess::Ok));
                }
            },
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }
}

fn is_duplicate_torrent(
    registry: &SessionRegistry,
    queue: &VecDeque<TorrentRequest>,
    session_store: &SessionStore,
    in_flight: &InFlightTorrents,
    info_hash: [u8; 20],
) -> bool {
    if let Ok(guard) = registry.lock() {
        if guard.contains_key(&info_hash) {
            return true;
        }
    }
    if session_store.contains(info_hash) {
        return true;
    }
    if lock_or_recover(in_flight).contains_key(&info_hash) {
        return true;
    }
    queue_contains_info_hash(queue, info_hash)
}

fn queue_contains_info_hash(queue: &VecDeque<TorrentRequest>, info_hash: [u8; 20]) -> bool {
    queue.iter().any(|request| {
        info_hash_for_source(&request.source)
            .map(|hash| hash == info_hash)
            .unwrap_or(false)
    })
}

fn session_entry_label(entry: &SessionEntry) -> String {
    if entry.name.is_empty() {
        let short = hex(&entry.info_hash);
        format!("resume {}", short.get(0..8).unwrap_or(&short))
    } else {
        entry.name.clone()
    }
}

fn restore_session_entries(
    session_store: &SessionStore,
    queue: &mut VecDeque<TorrentRequest>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    next_id: &mut u64,
) {
    for entry in session_store.list() {
        if entry.pending_delete {
            match retry_pending_delete(session_store, entry.info_hash) {
                Ok(()) => continue,
                Err(err) => {
                    log_warn!("pending deletion retry failed: {err}");
                    let torrent_id = *next_id;
                    *next_id = next_id.saturating_add(1);
                    update_ui(ui_state, |state| {
                        update_torrent_entry(state, torrent_id, |torrent| {
                            torrent.name = session_entry_label(&entry);
                            torrent.info_hash = hex(&entry.info_hash);
                            torrent.download_dir = entry.download_dir.display().to_string();
                            torrent.preallocate = entry.preallocate;
                            torrent.label = entry.label.clone();
                            torrent.status = "delete failed".to_string();
                            torrent.last_error = err.clone();
                        });
                    });
                    continue;
                }
            }
        }
        let label = session_entry_label(&entry);
        if queue_contains_info_hash(queue, entry.info_hash) {
            log_warn!(
                "duplicate restored session ignored: {}",
                hex(&entry.info_hash)
            );
            continue;
        }
        let request = TorrentRequest {
            id: *next_id,
            source: TorrentSource::Bytes(entry.torrent_bytes.clone()),
            download_dir: entry.download_dir.clone(),
            preallocate: entry.preallocate,
            initial_label: entry.label.clone(),
        };
        *next_id = next_id.saturating_add(1);
        enqueue_request_with_label(queue, ui_state, request, label);
    }
}

fn normalize_download_dir(value: String, fallback: &Path) -> PathBuf {
    if value.trim().is_empty() {
        fallback.to_path_buf()
    } else {
        PathBuf::from(value)
    }
}

fn enqueue_request(
    queue: &mut VecDeque<TorrentRequest>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    request: TorrentRequest,
) {
    let label = label_for_source(&request.source);
    enqueue_request_with_label(queue, ui_state, request, label);
}

#[allow(clippy::too_many_arguments)]
fn enqueue_request_if_new(
    registry: &SessionRegistry,
    queue: &mut VecDeque<TorrentRequest>,
    session_store: &SessionStore,
    in_flight: &InFlightTorrents,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    mut request: TorrentRequest,
    label: Option<String>,
) -> bool {
    let info_hash = match freeze_request_source(&mut request) {
        Ok(info_hash) => info_hash,
        Err(err) => {
            update_ui(ui_state, |state| {
                state.status = "error".to_string();
                state.last_error = err.clone();
                update_torrent_entry(state, request.id, |torrent| {
                    torrent.status = "error".to_string();
                    torrent.last_error = err.clone();
                });
            });
            return false;
        }
    };
    if is_duplicate_torrent(registry, queue, session_store, in_flight, info_hash) {
        return false;
    }
    match label {
        Some(label) => enqueue_request_with_label(queue, ui_state, request, label),
        None => enqueue_request(queue, ui_state, request),
    }
    true
}

fn enqueue_request_with_label(
    queue: &mut VecDeque<TorrentRequest>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    request: TorrentRequest,
    label: String,
) {
    let id = request.id;
    let download_dir = request.download_dir.display().to_string();
    let preallocate = request.preallocate;
    queue.push_back(request);
    update_ui(ui_state, |state| {
        state.queue_len = queue.len();
        state.last_added = label.clone();
        update_torrent_entry(state, id, |torrent| {
            if torrent.name.is_empty() {
                torrent.name = label.clone();
            }
            if torrent.download_dir.is_empty() {
                torrent.download_dir = download_dir.clone();
            }
            torrent.preallocate = preallocate;
            torrent.status = "queued".to_string();
            torrent.last_error.clear();
        });
    });
}

fn label_for_source(source: &TorrentSource) -> String {
    match source {
        TorrentSource::Path(path) => PathBuf::from(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "torrent".to_string()),
        TorrentSource::Bytes(_) => "torrent upload".to_string(),
        TorrentSource::Magnet(link) => match parse_magnet(link) {
            Ok(meta) => {
                let hash = hex(&meta.info_hash);
                let short = hash.get(0..8).unwrap_or(&hash);
                format!("magnet {short}")
            }
            Err(_) => "magnet link".to_string(),
        },
    }
}

fn update_idle_state(ui_state: &Option<Arc<Mutex<ui::UiState>>>, args: &Args, queue_len: usize) {
    update_ui(ui_state, |state| {
        if state.status.is_empty()
            || state.status == "idle"
            || state.status == "queued"
            || state.status == "waiting for torrent"
        {
            state.status = if queue_len > 0 {
                "queued".to_string()
            } else {
                "waiting for torrent".to_string()
            };
        }
        state.queue_len = queue_len;
        if queue_len == 0 && state.status == "waiting for torrent" {
            state.current_id = None;
        }
        if state.download_dir.is_empty() {
            state.download_dir = args.download_dir.display().to_string();
        }
        if state.total_pieces == 0 {
            state.preallocate = args.preallocate;
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn run_torrent(
    mut request: TorrentRequest,
    args: &Args,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    registry: &SessionRegistry,
    session_store: &Arc<SessionStore>,
    dht: &dht::Dht,
    lpd: &lpd::Lpd,
    utp: Option<utp::UtpConnector>,
    ip_filter: Option<Arc<IpFilter>>,
    global_down: Arc<RateLimiter>,
    global_up: Arc<RateLimiter>,
    peer_settings: Arc<PeerRuntimeSettings>,
    peer_slots: Arc<PeerSlots>,
    global_piece_buffer_budget: Arc<piece::PieceBufferBudget>,
) -> Result<(), String> {
    loop {
        let next = run_torrent_once(
            request,
            args,
            ui_state,
            registry,
            session_store,
            dht,
            lpd,
            utp.clone(),
            ip_filter.clone(),
            Arc::clone(&global_down),
            Arc::clone(&global_up),
            Arc::clone(&peer_settings),
            Arc::clone(&peer_slots),
            Arc::clone(&global_piece_buffer_budget),
        )?;
        match next {
            Some(next) if !shutdown_requested() => request = next,
            _ => return Ok(()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_torrent_once(
    request: TorrentRequest,
    args: &Args,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    registry: &SessionRegistry,
    session_store: &Arc<SessionStore>,
    dht: &dht::Dht,
    lpd: &lpd::Lpd,
    utp: Option<utp::UtpConnector>,
    ip_filter: Option<Arc<IpFilter>>,
    global_down: Arc<RateLimiter>,
    global_up: Arc<RateLimiter>,
    peer_settings: Arc<PeerRuntimeSettings>,
    peer_slots: Arc<PeerSlots>,
    global_piece_buffer_budget: Arc<piece::PieceBufferBudget>,
) -> Result<Option<TorrentRequest>, String> {
    let mut request = request;
    let connect_cfg = ConnectionConfig {
        encryption: args.encryption,
        utp,
        ip_filter: ip_filter.clone(),
        proxy: args.proxy.clone(),
    };
    if args.proxy.is_some() {
        log_info!(
            "proxy: outbound peer TCP enabled; inbound peers, DHT/LPD/uTP/UDP trackers disabled"
        );
    }
    let data = resolve_torrent_data(
        &request,
        args.port,
        dht,
        &connect_cfg,
        peer_settings.metadata_peer_limit(),
    )?;
    let meta = torrent::parse_torrent(&data).map_err(|err| format!("parse error: {err}"))?;
    ensure_private_state_directory(&request.download_dir)?;
    let hybrid_v2_info_hash = if meta.meta_version == 3 {
        meta.info_hash_v2.map(truncate_v2_info_hash)
    } else {
        None
    };
    let file_spans = Arc::new(build_file_spans(&meta)?);
    let v2_hashes = Arc::new(V2HashStore::new(&meta)?);
    let getright_multi_file = is_getright_multi_file(&meta);
    let mut resume_path = resume_path(&request.download_dir, meta.info_hash);
    let resume_data = load_resume_data_with_recovery(&resume_path).and_then(|data| {
        if data.info_hash == meta.info_hash {
            Some(data)
        } else {
            log_warn!(
                "resume info hash mismatch; ignoring {}",
                resume_path.display()
            );
            None
        }
    });
    let mut file_priorities = file_spans
        .iter()
        .map(|span| {
            if span.is_padding {
                piece::PRIORITY_SKIP
            } else {
                piece::PRIORITY_NORMAL
            }
        })
        .collect::<Vec<_>>();
    if let Some(resume) = resume_data.as_ref() {
        if resume.file_priorities.len() == file_priorities.len() {
            file_priorities.clone_from(&resume.file_priorities);
        }
    }
    for (priority, span) in file_priorities.iter_mut().zip(file_spans.iter()) {
        if span.is_padding {
            *priority = piece::PRIORITY_SKIP;
        }
    }
    // Reconciliation and the durable claim are one lifecycle transaction.
    // Once the claim is saved, every later rename/move/delete transition is
    // serialized by the same operation lock.
    let storage_claim_operation = session_store.lock_operation();
    let session_entry = session_store.get(meta.info_hash);
    if session_entry.is_some() {
        session_store.validate_current_storage_claim(meta.info_hash)?;
    }
    if session_entry
        .as_ref()
        .is_some_and(|entry| entry.pending_delete)
    {
        return Err("torrent deletion is pending".to_string());
    }
    let legacy_resume_renames = resume_data
        .as_ref()
        .map(|resume| resume.file_renames.clone())
        .unwrap_or_default();
    let mut saved_renames = session_entry
        .as_ref()
        .filter(|entry| !entry.file_renames.is_empty())
        .map(|entry| entry.file_renames.clone())
        .unwrap_or_else(|| legacy_resume_renames.clone());
    saved_renames.sort_unstable_by_key(|(index, _)| *index);
    if session_entry
        .as_ref()
        .is_some_and(|entry| entry.pending_file_rename.is_some())
    {
        let entry = session_store
            .get(meta.info_hash)
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        saved_renames =
            reconcile_pending_file_rename(&meta, &request.download_dir, session_store, &entry)?;
    }
    let initial_renames: HashMap<usize, String> = saved_renames.iter().cloned().collect();
    let mut completion_move_reconciled = false;
    if session_store.completion_state(meta.info_hash) == Some(CompletionState::Pending) {
        let entry = session_store
            .get(meta.info_hash)
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if entry.pending_delete {
            return Err("torrent deletion is pending".to_string());
        }
        if let Some(dest) = entry.completion_move_dir {
            let source_override =
                single_file_source_override(&meta, &request.download_dir, &saved_renames)?;
            let completed_move = completed_move_paths(
                &meta,
                &request.download_dir,
                &dest,
                source_override.as_deref(),
            )?;
            if let CompletionMoveRecovery::AdoptDestination { remove_source } =
                completion_move_recovery(&meta, &dest, &saved_renames, &completed_move)?
            {
                if remove_source {
                    let source_paths = if saved_renames.is_empty() {
                        storage::data_paths(&meta, &request.download_dir)
                    } else {
                        storage::data_paths_with_file_renames(
                            &meta,
                            &request.download_dir,
                            &saved_renames,
                        )
                    }
                    .map_err(|err| format!("completion source paths: {err}"))?;
                    delete_storage_paths(&request.download_dir, &source_paths)?;
                }
                // Source cleanup precedes the durable directory switch. If the
                // process stops between these steps, the still-pending journal
                // will verify and adopt the destination again on next launch.
                // Committing first would lose the only durable evidence that
                // duplicate source files still need cleanup.
                if !session_store.commit_completion_move(meta.info_hash, &dest)? {
                    return Err("completion move state changed during recovery".to_string());
                }
                let relocated_resume = crate::resume_path(&dest, meta.info_hash);
                if let Err(err) = relocate_resume_state(&resume_path, &relocated_resume) {
                    log_warn!("move-completed resume relocation failed: {err}");
                }
                resume_path = relocated_resume;
                request.download_dir = dest;
                completion_move_reconciled = true;
                log_info!(
                    "reconciled completed move at {}",
                    completed_move.destination.display()
                );
            }
        }
    }
    let name = String::from_utf8_lossy(&meta.info.name).into_owned();
    session_store.upsert_with_storage_claim(
        meta.info_hash,
        name.clone(),
        data.clone(),
        &request.download_dir,
        request.preallocate,
        &saved_renames,
    )?;
    drop(storage_claim_operation);
    let mut pieces =
        piece::PieceManager::new(&meta).map_err(|err| format!("piece error: {err}"))?;
    pieces.set_sequential(args.sequential);
    apply_file_priorities(
        &mut pieces,
        &file_spans,
        &file_priorities,
        meta.info.piece_length,
    )
    .map_err(|err| format!("priority error: {err}"))?;
    let storage_options = storage::StorageOptions {
        preallocate: request.preallocate,
        write_cache_bytes: args.write_cache_bytes,
    };
    let mut storage = if saved_renames.is_empty() {
        storage::Storage::new(&meta, &request.download_dir, storage_options)
    } else {
        storage::Storage::new_with_file_renames(
            &meta,
            &request.download_dir,
            storage_options,
            &saved_renames,
        )
    }
    .map_err(|err| format!("storage error: {err}"))?;
    let peer_id = generate_peer_id();

    let resume = resume_from_storage(
        &mut pieces,
        &mut storage,
        meta.info.piece_length,
        &file_spans,
        resume_data.as_ref(),
    )
    .map_err(|err| format!("resume error: {err}"))?;
    let resume_downloaded = resume.completed_bytes.max(
        resume_data
            .as_ref()
            .map(|data| data.downloaded)
            .unwrap_or(0),
    );
    let resume_uploaded = resume_data.as_ref().map(|data| data.uploaded).unwrap_or(0);
    let torrent_down = Arc::new(RateLimiter::new(args.torrent_download_rate));
    let torrent_up = Arc::new(RateLimiter::new(args.torrent_upload_rate));
    let limits = TransferLimits {
        global_down: global_down.clone(),
        global_up: global_up.clone(),
        torrent_down,
        torrent_up,
    };

    let initial_complete = pieces.is_complete();
    if completion_move_reconciled {
        if let Some(script) = ON_COMPLETE_SCRIPT.get() {
            spawn_on_complete_script(
                script,
                &name,
                &request.download_dir,
                meta.info_hash,
                meta.info.total_length(),
            );
        }
    }
    let mut completion_state = session_store
        .completion_state(meta.info_hash)
        .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
    let mut completion_move_dir = session_store.completion_move_dir(meta.info_hash);
    let mut completion_move_pending = false;
    match completion_action(
        completion_state,
        initial_complete,
        initial_complete,
        if completion_state == CompletionState::Pending {
            completion_move_dir.is_some()
        } else {
            args.move_completed.is_some()
        },
    ) {
        CompletionAction::None => {}
        CompletionAction::MarkDone => {
            if session_store.transition_completion_state(
                meta.info_hash,
                CompletionState::None,
                CompletionState::Done,
            )? {
                completion_state = CompletionState::Done;
                completion_move_dir = None;
            }
        }
        CompletionAction::RunScript => {
            // A persisted pending state is recovery evidence from a real prior
            // incomplete -> complete transition. Mark it done before launching
            // the external process so the hook is explicitly at-most-once.
            if session_store.transition_completion_state(
                meta.info_hash,
                CompletionState::Pending,
                CompletionState::Done,
            )? {
                completion_state = CompletionState::Done;
                completion_move_dir = None;
                if let Some(script) = ON_COMPLETE_SCRIPT.get() {
                    spawn_on_complete_script(
                        script,
                        &name,
                        &request.download_dir,
                        meta.info_hash,
                        meta.info.total_length(),
                    );
                }
            }
        }
        CompletionAction::Move => {
            completion_move_pending = true;
        }
    }
    let file_priorities = Arc::new(Mutex::new(file_priorities));
    let downloaded = Arc::new(AtomicU64::new(resume_downloaded));
    let uploaded = Arc::new(AtomicU64::new(resume_uploaded));
    let paused_flag = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::new(AtomicBool::new(false));
    if completion_move_pending {
        stop_flag.store(true, Ordering::SeqCst);
    }
    let peer_queue = Arc::new(Mutex::new(PeerQueue::new_with_local_addrs(
        ip_filter.clone(),
        local_peer_addrs(args.port),
    )));
    if let Some(resume) = resume_data.as_ref() {
        if !resume.peers.is_empty() {
            if let Ok(mut queue) = peer_queue.lock() {
                let restored =
                    queue.enqueue_with_source(resume.peers.iter().copied(), PeerSource::Tracker);
                if restored > 0 {
                    log_info!("restored {restored} peers from resume data");
                }
            }
        }
    }
    let mut ui_files = build_ui_files(&file_spans, &pieces, &lock_or_recover(&file_priorities));
    apply_ui_file_renames(&mut ui_files, &initial_renames);
    let announce = meta
        .announce
        .as_ref()
        .map(|bytes| safe_network_url_label(&String::from_utf8_lossy(bytes)))
        .unwrap_or_else(|| "<none>".to_string());
    let trackers = collect_trackers(&meta);
    let web_seeds = if args.proxy.is_none() {
        collect_web_seeds(&meta)
    } else {
        Vec::new()
    };

    let log_name = tracker::sanitize_failure_reason(name.as_bytes());
    log_info!("name: {log_name}");
    log_info!("announce: {announce}");
    log_info!(
        "trackers: {} (http={}, udp={})",
        trackers.http.len() + trackers.udp.len(),
        trackers.http.len(),
        trackers.udp.len()
    );
    log_info!("piece length: {}", meta.info.piece_length);
    log_info!("pieces: {}", meta.info.pieces.len());
    let wanted_bytes = pieces.wanted_bytes();
    let wanted_pieces = pieces.wanted_pieces();
    let completed_pieces = pieces.completed_pieces();
    let completed_bytes = pieces.completed_bytes();
    log_info!("total size: {}", meta.info.total_length());
    if wanted_bytes != meta.info.total_length() {
        log_info!("wanted size: {wanted_bytes}");
    }
    log_info!("info hash: {}", hex(&meta.info_hash));
    log_info!("bitfield bytes: {}", pieces.bitfield_len());
    log_info!("files: {}", storage.file_count());
    log_info!("preallocate: {}", request.preallocate);
    log_info!("web seeds: {}", web_seeds.len());
    if completed_pieces > 0 {
        log_info!(
            "resumed: {}/{} pieces",
            completed_pieces,
            meta.info.pieces.len()
        );
    }

    let paused = torrent_paused(&paused_flag);
    update_ui(ui_state, |state| {
        state.name = name.clone();
        state.info_hash = hex(&meta.info_hash);
        state.download_dir = request.download_dir.display().to_string();
        state.total_pieces = wanted_pieces;
        state.completed_pieces = completed_pieces;
        state.total_bytes = wanted_bytes;
        state.completed_bytes = completed_bytes;
        state.downloaded_bytes = resume_downloaded;
        state.uploaded_bytes = resume_uploaded;
        state.tracker_peers = 0;
        state.status = "ready".to_string();
        state.last_error.clear();
        state.preallocate = request.preallocate;
        state.paused = is_paused();
        state.files = ui_files.clone();
        state.current_id = Some(request.id);
        update_torrent_entry(state, request.id, |torrent| {
            torrent.name = name.clone();
            torrent.info_hash = hex(&meta.info_hash);
            torrent.download_dir = request.download_dir.display().to_string();
            torrent.preallocate = request.preallocate;
            torrent.status = "ready".to_string();
            torrent.total_bytes = wanted_bytes;
            torrent.completed_bytes = completed_bytes;
            torrent.downloaded_bytes = resume_downloaded;
            torrent.uploaded_bytes = resume_uploaded;
            torrent.total_pieces = wanted_pieces;
            torrent.completed_pieces = completed_pieces;
            torrent.tracker_peers = 0;
            torrent.active_peers = 0;
            torrent.paused = paused;
            torrent.last_error.clear();
            torrent.files = ui_files.clone();
            torrent.trackers = trackers
                .http
                .iter()
                .chain(trackers.udp.iter())
                .cloned()
                .collect();
            torrent.label = request.initial_label.clone();
            torrent.meta_version = meta.meta_version;
        });
    });

    let pieces = Arc::new(Mutex::new(pieces));
    let storage = Arc::new(Mutex::new(storage));
    let completed_log = Arc::new(Mutex::new(Vec::new()));
    let piece_buffer_budgets = piece::PieceBufferBudgets::new(
        Arc::clone(&global_piece_buffer_budget),
        Arc::new(piece::PieceBufferBudget::new(
            MAX_TORRENT_PIECE_BUFFER_BYTES,
        )),
    );
    let peer_tags = Arc::new(AtomicU64::new(1));
    let upload_manager = Arc::new(UploadManager::new(UPLOAD_SLOTS));
    let active_peers = Arc::new(AtomicUsize::new(0));
    let interested_peers = Arc::new(AtomicUsize::new(0));
    let upload_requests_served = Arc::new(AtomicU64::new(0));
    let shared_trackers = Arc::new(Mutex::new(trackers.clone()));
    let context = Arc::new(TorrentContext {
        id: request.id,
        info_hash: meta.info_hash,
        hybrid_v2_info_hash,
        peer_id,
        pieces: Arc::clone(&pieces),
        storage: Arc::clone(&storage),
        completed_log: Arc::clone(&completed_log),
        base_piece_length: meta.info.piece_length,
        v2_hashes: Arc::clone(&v2_hashes),
        file_spans: Arc::clone(&file_spans),
        file_priorities: Arc::clone(&file_priorities),
        limits: limits.clone(),
        downloaded: Arc::clone(&downloaded),
        uploaded: Arc::clone(&uploaded),
        active_peers: Arc::clone(&active_peers),
        interested_peers: Arc::clone(&interested_peers),
        upload_requests_served: Arc::clone(&upload_requests_served),
        paused: Arc::clone(&paused_flag),
        stop_requested: Arc::clone(&stop_flag),
        allow_completion_reentry: Arc::new(AtomicBool::new(true)),
        rechecking: Arc::new(AtomicBool::new(false)),
        resume_save_requested: Arc::new(AtomicBool::new(false)),
        delete_data_requested: Arc::new(AtomicBool::new(false)),
        archive_requested: Arc::new(AtomicBool::new(false)),
        teardown_failed: Arc::new(AtomicBool::new(false)),
        upload_manager: Arc::clone(&upload_manager),
        peer_tags: Arc::clone(&peer_tags),
        peer_cancellations: Arc::new(Mutex::new(HashMap::new())),
        label: Arc::new(Mutex::new(request.initial_label.clone())),
        trackers: Arc::clone(&shared_trackers),
        throttle_group: Arc::new(Mutex::new(None)),
        ratio_group: Arc::new(Mutex::new(None)),
        file_renames: Arc::new(Mutex::new(initial_renames)),
    });
    register_session(registry, Arc::clone(&context))?;
    let resume_handle = match start_resume_worker(
        resume_path.clone(),
        meta.info_hash,
        meta.info.piece_length,
        Arc::clone(&pieces),
        Arc::clone(&storage),
        Arc::clone(&file_priorities),
        Arc::clone(&file_spans),
        Arc::clone(&downloaded),
        Arc::clone(&uploaded),
        Arc::clone(&peer_queue),
        Arc::clone(&stop_flag),
        Arc::clone(&context.file_renames),
        Arc::clone(&context.resume_save_requested),
    ) {
        Ok(handle) => handle,
        Err(err) => {
            unregister_session(registry, meta.info_hash, request.id);
            return Err(err);
        }
    };
    let webseed_handle = match start_webseed_worker(
        web_seeds.clone(),
        Arc::clone(&pieces),
        Arc::clone(&storage),
        Arc::clone(&completed_log),
        Arc::clone(&file_spans),
        getright_multi_file,
        meta.info.piece_length,
        meta.info_hash,
        limits.clone(),
        Arc::clone(&downloaded),
        Arc::clone(&stop_flag),
        piece_buffer_budgets.clone(),
        ui_state.clone(),
        request.id,
    ) {
        Ok(handle) => handle,
        Err(err) => {
            stop_flag.store(true, Ordering::SeqCst);
            let stopped = join_worker_before(
                resume_handle,
                "resume",
                Instant::now() + TORRENT_WORKER_SHUTDOWN_TIMEOUT,
            );
            if stopped {
                unregister_session(registry, meta.info_hash, request.id);
            } else {
                retain_context_after_teardown_failure(registry, &context);
            }
            return Err(err);
        }
    };

    let decentralized_discovery = !meta.info.private && args.proxy.is_none();
    let has_network_sources = tracker_set_has_usable_source(&trackers, args.proxy.is_none())
        || decentralized_discovery
        || !web_seeds.is_empty();
    if !has_network_sources {
        log_warn!(
            "torrent has no outbound discovery source; waiting for an inbound peer or stop request"
        );
    }
    let mut discovery_handles = Vec::new();
    let mut resource_workers_stopped = true;
    let mut was_complete = initial_complete;
    {
        let total_length = wanted_bytes;
        let mut started = true;
        // A resumed/pre-seeded torrent did not complete in this process. Only
        // emit the tracker completed event for a live incomplete -> complete
        // transition, including the run immediately before a completion move.
        let mut completed_sent = initial_complete;
        let mut interval = 1800u64; // Default to 30 minutes
        let mut last_announce = Instant::now() - Duration::from_secs(interval + 1); // Force first announce
        let mut rate_last_at = Instant::now();
        let mut last_downloaded = downloaded.load(Ordering::SeqCst);
        let mut last_progress_at = Instant::now();
        let mut down_rate = 0.0;
        let mut up_rate = 0.0;
        let mut eta_secs = 0u64;
        let mut down_snapshots: std::collections::VecDeque<(u64, Instant)> =
            std::collections::VecDeque::new();
        let mut up_snapshots: std::collections::VecDeque<(u64, Instant)> =
            std::collections::VecDeque::new();
        // Track per-tracker failures for exponential backoff: url -> (fail_count, last_failure)
        let mut tracker_failures: HashMap<String, (u32, Instant)> = HashMap::new();
        let per_torrent_slots = Arc::new(PeerSlots::new(peer_settings.max_peers_torrent()));
        let peer_tags = Arc::clone(&peer_tags);
        let torrent_id = request.id;
        let allow_pex = !meta.info.private;
        if meta.info.private {
            log_info!("private torrent: DHT/PEX/LPD disabled");
        }
        let dht_rx = if decentralized_discovery {
            let (tx, rx) = mpsc::channel();
            dht.add_torrent(meta.info_hash, args.port, tx);
            Some(rx)
        } else {
            None
        };
        if let Some(rx) = dht_rx {
            let queue_clone = Arc::clone(&peer_queue);
            match thread::Builder::new()
                .name(format!("dht-peers-{torrent_id}"))
                .stack_size(PEER_THREAD_STACK)
                .spawn(move || {
                    for peers in rx {
                        let mut queue = lock_or_recover(&queue_clone);
                        queue.enqueue_with_source(peers, PeerSource::Dht);
                    }
                }) {
                Ok(handle) => discovery_handles.push(handle),
                Err(err) => {
                    log_warn!("DHT peer receiver could not start: {err}");
                }
            }
        }
        let lpd_rx = if decentralized_discovery {
            let (tx, rx) = mpsc::channel();
            lpd.add_torrent(meta.info_hash, args.port, tx);
            Some(rx)
        } else {
            None
        };
        if let Some(rx) = lpd_rx {
            let queue_clone = Arc::clone(&peer_queue);
            match thread::Builder::new()
                .name(format!("lpd-peers-{torrent_id}"))
                .stack_size(PEER_THREAD_STACK)
                .spawn(move || {
                    for peers in rx {
                        let mut queue = lock_or_recover(&queue_clone);
                        queue.enqueue_with_source(peers, PeerSource::Lpd);
                    }
                }) {
                Ok(handle) => discovery_handles.push(handle),
                Err(err) => {
                    log_warn!("LPD peer receiver could not start: {err}");
                }
            }
        }

        let mut handles = Vec::new();

        let mut seed_start: Option<Instant> = None;
        while !torrent_stop_requested(&stop_flag) {
            reap_finished_workers(&mut handles, "peer");
            let desired_workers = peer_settings.max_peers_torrent();
            let startup_burst = downloaded.load(Ordering::SeqCst) < STARTUP_BURST_BYTES;
            let live_target = if startup_burst {
                desired_workers
                    .saturating_mul(STARTUP_BURST_MULTIPLIER)
                    .max(STARTUP_BURST_MIN_WORKERS)
                    .min(peer_settings.max_peers_global())
            } else {
                desired_workers
            };
            per_torrent_slots.set_max(live_target);
            while handles.len() < live_target {
                let pieces_clone = Arc::clone(&pieces);
                let storage_clone = Arc::clone(&storage);
                let completed_clone = Arc::clone(&completed_log);
                let queue_clone = Arc::clone(&peer_queue);
                let active_clone = Arc::clone(&active_peers);
                let interested_clone = Arc::clone(&interested_peers);
                let upload_requests_clone = Arc::clone(&upload_requests_served);
                let tags_clone = Arc::clone(&peer_tags);
                let file_spans = Arc::clone(&file_spans);
                let v2_hashes = Arc::clone(&v2_hashes);
                let downloaded = Arc::clone(&downloaded);
                let uploaded = Arc::clone(&uploaded);
                let upload_manager = Arc::clone(&upload_manager);
                let peer_cancellations = Arc::clone(&context.peer_cancellations);
                let paused_flag = Arc::clone(&paused_flag);
                let stop_flag = Arc::clone(&stop_flag);
                let ui_clone = ui_state.clone();
                let info_hash = meta.info_hash;
                let base_piece_length = meta.info.piece_length;
                let connect_cfg = connect_cfg.clone();
                let limits = limits.clone();
                let peer_slots = Arc::clone(&peer_slots);
                let per_torrent_slots = Arc::clone(&per_torrent_slots);
                let piece_buffer_budgets = piece_buffer_budgets.clone();

                let spawn_result = thread::Builder::new()
                    .name(format!("peer-{torrent_id}-{}", handles.len()))
                    .stack_size(PEER_THREAD_STACK)
                    .spawn(move || {
                        peer_worker_loop(
                            info_hash,
                            hybrid_v2_info_hash,
                            peer_id,
                            torrent_id,
                            &tags_clone,
                            &pieces_clone,
                            &storage_clone,
                            &completed_clone,
                            &queue_clone,
                            allow_pex,
                            &active_clone,
                            &interested_clone,
                            &upload_requests_clone,
                            &file_spans,
                            base_piece_length,
                            &v2_hashes,
                            connect_cfg,
                            limits,
                            &downloaded,
                            &uploaded,
                            &upload_manager,
                            &peer_cancellations,
                            &paused_flag,
                            &stop_flag,
                            peer_slots,
                            per_torrent_slots,
                            piece_buffer_budgets,
                            &ui_clone,
                        );
                    });
                match spawn_result {
                    Ok(handle) => handles.push(handle),
                    Err(err) => {
                        log_warn!("peer worker could not start: {err}");
                        break;
                    }
                }
            }

            let (is_complete, completed_pieces, completed_bytes) = {
                let p = lock_or_recover(&pieces);
                (p.is_complete(), p.completed_pieces(), p.completed_bytes())
            };
            let mut completion_recorded = true;
            match completion_action(
                completion_state,
                was_complete,
                is_complete,
                if completion_state == CompletionState::Pending {
                    completion_move_dir.is_some()
                } else {
                    args.move_completed.is_some()
                },
            ) {
                CompletionAction::None => {}
                CompletionAction::MarkDone => {
                    match session_store.transition_completion_state(
                        meta.info_hash,
                        CompletionState::None,
                        CompletionState::Done,
                    ) {
                        Ok(true) => {
                            completion_state = CompletionState::Done;
                            completion_move_dir = None;
                        }
                        Ok(false) => {
                            completion_state = session_store
                                .completion_state(meta.info_hash)
                                .unwrap_or(completion_state);
                            completion_move_dir = session_store.completion_move_dir(meta.info_hash);
                        }
                        Err(err) => {
                            completion_recorded = false;
                            log_warn!("completion state save failed: {err}");
                        }
                    }
                }
                CompletionAction::RunScript => {
                    if completion_state == CompletionState::None {
                        let begin_result = {
                            let _operation = session_store.lock_operation();
                            session_store.begin_completion(meta.info_hash, None)
                        };
                        match begin_result {
                            Ok(true) => {
                                completion_state = CompletionState::Pending;
                                completion_move_dir = None;
                            }
                            Ok(false) => {
                                completion_state = session_store
                                    .completion_state(meta.info_hash)
                                    .unwrap_or(completion_state);
                                completion_move_dir =
                                    session_store.completion_move_dir(meta.info_hash);
                            }
                            Err(err) => {
                                completion_recorded = false;
                                log_warn!("completion pending save failed: {err}");
                            }
                        }
                    }
                    if completion_recorded && completion_state == CompletionState::Pending {
                        match session_store.transition_completion_state(
                            meta.info_hash,
                            CompletionState::Pending,
                            CompletionState::Done,
                        ) {
                            Ok(true) => {
                                completion_state = CompletionState::Done;
                                completion_move_dir = None;
                                if let Some(script) = ON_COMPLETE_SCRIPT.get() {
                                    spawn_on_complete_script(
                                        script,
                                        &name,
                                        &request.download_dir,
                                        meta.info_hash,
                                        meta.info.total_length(),
                                    );
                                }
                            }
                            Ok(false) => {
                                completion_state = session_store
                                    .completion_state(meta.info_hash)
                                    .unwrap_or(completion_state);
                                completion_move_dir =
                                    session_store.completion_move_dir(meta.info_hash);
                            }
                            Err(err) => {
                                log_warn!("completion done save failed: {err}");
                            }
                        }
                    }
                }
                CompletionAction::Move => {
                    if completion_state == CompletionState::None {
                        let begin_result = {
                            let _operation = session_store.lock_operation();
                            session_store
                                .begin_completion(meta.info_hash, args.move_completed.as_deref())
                        };
                        match begin_result {
                            Ok(true) => {
                                completion_state = CompletionState::Pending;
                                completion_move_dir =
                                    session_store.completion_move_dir(meta.info_hash);
                            }
                            Ok(false) => {
                                completion_state = session_store
                                    .completion_state(meta.info_hash)
                                    .unwrap_or(completion_state);
                                completion_move_dir =
                                    session_store.completion_move_dir(meta.info_hash);
                            }
                            Err(err) => {
                                completion_recorded = false;
                                log_warn!("completion pending save failed: {err}");
                            }
                        }
                    }
                    if completion_recorded && completion_state == CompletionState::Pending {
                        completion_move_pending = true;
                        // Moving open content while peer and resume workers are active
                        // creates races. Stop this torrent and move only after all workers
                        // below have joined.
                        stop_flag.store(true, Ordering::SeqCst);
                    }
                }
            }
            if completion_recorded || !is_complete {
                was_complete = is_complete;
            }
            let downloaded = downloaded.load(Ordering::SeqCst);
            let uploaded = uploaded.load(Ordering::SeqCst);
            let left = total_length.saturating_sub(completed_bytes);
            let completed_pending = is_complete && !completed_sent;
            let now = Instant::now();
            let dt = now.duration_since(rate_last_at).as_secs_f64();
            if dt >= 0.2 {
                let delta_down = downloaded.saturating_sub(last_downloaded) as f64;
                if delta_down > 0.0 {
                    last_progress_at = now;
                }
                const RATE_WINDOW: Duration = Duration::from_secs(5);
                down_snapshots.push_back((downloaded, now));
                up_snapshots.push_back((uploaded, now));
                while down_snapshots.len() > 1 {
                    if now.duration_since(down_snapshots[0].1) > RATE_WINDOW {
                        down_snapshots.pop_front();
                    } else {
                        break;
                    }
                }
                while up_snapshots.len() > 1 {
                    if now.duration_since(up_snapshots[0].1) > RATE_WINDOW {
                        up_snapshots.pop_front();
                    } else {
                        break;
                    }
                }
                let sliding_rate = |snaps: &std::collections::VecDeque<(u64, Instant)>| -> f64 {
                    if snaps.len() < 2 {
                        return 0.0;
                    }
                    let (Some(first), Some(last)) = (snaps.front(), snaps.back()) else {
                        return 0.0;
                    };
                    let elapsed = last.1.duration_since(first.1).as_secs_f64();
                    if elapsed < 0.1 {
                        return 0.0;
                    }
                    last.0.saturating_sub(first.0) as f64 / elapsed
                };
                down_rate = sliding_rate(&down_snapshots);
                up_rate = sliding_rate(&up_snapshots);
                eta_secs = if down_rate > 1.0 {
                    (left as f64 / down_rate).round() as u64
                } else {
                    0
                };
                last_downloaded = downloaded;
                rate_last_at = now;
            }

            let (known_count, queue_len) = {
                let q = lock_or_recover(&peer_queue);
                (q.known_len(), q.len())
            };
            let active_count = active_peers.load(Ordering::SeqCst);
            let interested_count = interested_peers.load(Ordering::SeqCst);
            let upload_requests_count = upload_requests_served.load(Ordering::SeqCst);

            // Only announce if enough time has passed OR we have no peers/low activity
            let time_since_announce = last_announce.elapsed().as_secs();
            let no_peers = active_count == 0 && queue_len == 0;
            let need_peers =
                active_count < LOW_PEER_THRESHOLD || queue_len == 0 || known_count == 0;
            let stalled_download = !is_complete
                && last_progress_at.elapsed().as_secs() >= STALL_REANNOUNCE_SECS
                && active_count <= 2;
            const SEED_REANNOUNCE_SECS: u64 = 300;
            let seed_upload_stalled =
                is_complete && up_rate < 1.0 && time_since_announce >= SEED_REANNOUNCE_SECS;
            let seed_needs_peers = is_complete
                && active_count < LOW_PEER_THRESHOLD
                && time_since_announce >= SEED_REANNOUNCE_SECS;
            // Tracker edits are live. In particular, adding the first tracker
            // to a trackerless torrent must make the pending `started`
            // announce eligible without restarting the torrent.
            let current_trackers = lock_or_recover(&shared_trackers).clone();
            let has_trackers =
                tracker_set_has_usable_source(&current_trackers, args.proxy.is_none());
            let should_announce = has_trackers
                && (started
                    || completed_pending
                    || time_since_announce >= interval
                    || (stalled_download && time_since_announce >= STALL_REANNOUNCE_SECS)
                    || (need_peers && time_since_announce >= args.retry_interval)
                    || (no_peers && time_since_announce >= NO_PEER_REANNOUNCE_SECS)
                    || seed_needs_peers
                    || seed_upload_stalled);

            let mut last_error: Option<String> = None;
            let mut any_success = false;
            let paused = torrent_paused(&paused_flag);

            if should_announce {
                if !paused && !is_complete {
                    update_ui(ui_state, |state| {
                        state.status = "announcing".to_string();
                        update_torrent_entry(state, request.id, |torrent| {
                            torrent.status = "announcing".to_string();
                        });
                    });
                }
                let event = if started {
                    Some("started")
                } else if completed_pending {
                    Some("completed")
                } else {
                    None
                };
                let sent_completed_event = event == Some("completed");
                log_info!("announcing to trackers...");
                let announce_numwant = peer_settings.numwant();

                // Filter out trackers in backoff period
                let filtered_trackers = {
                    let now = Instant::now();
                    let should_skip = |url: &str| -> bool {
                        if let Some(&(failures, last_fail)) = tracker_failures.get(url) {
                            if failures >= 3 {
                                let backoff =
                                    300u64.min(30u64.saturating_mul(1u64 << failures.min(10)));
                                if now.duration_since(last_fail).as_secs() < backoff {
                                    log_debug!(
                                        "skipping backed-off tracker {} (failures={failures})",
                                        safe_network_url_label(url)
                                    );
                                    return true;
                                }
                            }
                        }
                        false
                    };
                    TrackerSet {
                        http: current_trackers
                            .http
                            .iter()
                            .filter(|u| !should_skip(u))
                            .cloned()
                            .collect(),
                        udp: if args.proxy.is_none() {
                            current_trackers
                                .udp
                                .iter()
                                .filter(|u| !should_skip(u))
                                .cloned()
                                .collect()
                        } else {
                            Vec::new()
                        },
                    }
                };
                let (announce_rx, mut announce_pending) = spawn_tracker_announces(
                    &filtered_trackers,
                    meta.info_hash,
                    peer_id,
                    args.port,
                    uploaded,
                    downloaded,
                    left,
                    event,
                    announce_numwant,
                    meta.info.private,
                    args.proxy.clone(),
                    TRACKER_ANNOUNCE_WAIT_BUDGET,
                );
                let announce_deadline = Instant::now() + TRACKER_ANNOUNCE_WAIT_BUDGET;
                let mut peers_added = 0usize;
                while announce_pending > 0 {
                    let Some(remaining) = announce_deadline.checked_duration_since(Instant::now())
                    else {
                        break;
                    };
                    if remaining.is_zero() {
                        break;
                    }
                    let wait = remaining.min(TRACKER_ANNOUNCE_POLL);
                    match announce_rx.recv_timeout(wait) {
                        Ok(result) => {
                            announce_pending = announce_pending.saturating_sub(1);
                            match result.response {
                                Ok(response) => {
                                    any_success = true;
                                    tracker_failures.remove(&result.tracker_url);
                                    interval = interval.min(response.interval.max(60));
                                    if started {
                                        log_info!("tracker interval: {}", response.interval);
                                    }
                                    if result.is_udp {
                                        log_info!(
                                            "udp tracker {} returned {} peers",
                                            safe_network_url_label(&result.tracker_url),
                                            response.peers.len()
                                        );
                                    } else {
                                        log_info!(
                                            "tracker {} returned {} peers",
                                            safe_network_url_label(&result.tracker_url),
                                            response.peers.len()
                                        );
                                    }
                                    let mut queue = lock_or_recover(&peer_queue);
                                    peers_added += queue
                                        .enqueue_with_source(response.peers, PeerSource::Tracker);
                                    if started && peers_added >= desired_workers.max(8) {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    let entry = tracker_failures
                                        .entry(result.tracker_url.clone())
                                        .or_insert((0, Instant::now()));
                                    entry.0 = entry.0.saturating_add(1);
                                    entry.1 = Instant::now();
                                    let err = format!(
                                        "{}: {err}",
                                        safe_network_url_label(&result.tracker_url)
                                    );
                                    if result.is_udp {
                                        log_warn!("udp tracker error: {err}");
                                    } else {
                                        log_warn!("tracker error: {err}");
                                    }
                                    last_error = Some(err);
                                }
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                if announce_pending > 0 {
                    let queue_clone = Arc::clone(&peer_queue);
                    thread::Builder::new()
                        .stack_size(PEER_THREAD_STACK)
                        .spawn(move || {
                            let mut pending = announce_pending;
                            while pending > 0 {
                                let Some(remaining) =
                                    announce_deadline.checked_duration_since(Instant::now())
                                else {
                                    break;
                                };
                                if remaining.is_zero() {
                                    break;
                                }
                                let wait = remaining.min(TRACKER_ANNOUNCE_POLL);
                                match announce_rx.recv_timeout(wait) {
                                    Ok(result) => {
                                        pending = pending.saturating_sub(1);
                                        if let Ok(response) = result.response {
                                            if let Ok(mut queue) = queue_clone.lock() {
                                                let _ = queue.enqueue_with_source(
                                                    response.peers,
                                                    PeerSource::Tracker,
                                                );
                                            }
                                        }
                                    }
                                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                                }
                            }
                        })
                        .ok();
                }
                last_announce = Instant::now();
                started = false;
                if sent_completed_event && any_success {
                    completed_sent = true;
                }
            }

            let (known_count, queue_len) = {
                let q = lock_or_recover(&peer_queue);
                (q.known_len(), q.len())
            };

            let stopping = stop_flag.load(Ordering::SeqCst);
            let shown_known_count = if stopping { 0 } else { known_count };
            let shown_active_count = if stopping { 0 } else { active_count };
            let shown_interested_count = if stopping { 0 } else { interested_count };
            let shown_down_rate = if stopping { 0.0 } else { down_rate };
            let shown_up_rate = if stopping { 0.0 } else { up_rate };
            let shown_eta_secs = if stopping { 0 } else { eta_secs };

            let torrent_status = if stopping {
                "stopping"
            } else if paused {
                "paused"
            } else if is_complete {
                "seeding"
            } else if should_announce
                && !any_success
                && last_error.is_some()
                && known_count == 0
                && active_count == 0
            {
                "error"
            } else if active_count == 0 && known_count > 0 {
                "connecting"
            } else if active_count == 0 && queue_len == 0 {
                "waiting for peers"
            } else {
                "downloading"
            };
            update_ui(ui_state, |state| {
                set_torrent_completion_ui(state, request.id, completed_pieces, completed_bytes);
                if state.current_id == Some(request.id) {
                    state.tracker_peers = shown_known_count;
                    state.paused = is_paused();
                    state.downloaded_bytes = downloaded;
                    state.uploaded_bytes = uploaded;
                    state.status = torrent_status.to_string();
                    state.active_peers = shown_active_count;
                    state.interested_peers = shown_interested_count;
                    state.upload_requests_served = upload_requests_count;
                    state.download_rate_bps = shown_down_rate;
                    state.upload_rate_bps = shown_up_rate;
                    state.eta_secs = shown_eta_secs;
                    if let Some(err) = &last_error {
                        state.last_error = err.clone();
                    }
                }
                let last_error = last_error.clone();
                update_torrent_entry(state, request.id, |torrent| {
                    torrent.tracker_peers = shown_known_count;
                    torrent.active_peers = shown_active_count;
                    torrent.interested_peers = shown_interested_count;
                    torrent.upload_requests_served = upload_requests_count;
                    torrent.paused = paused;
                    torrent.status = torrent_status.to_string();
                    torrent.completed_pieces = completed_pieces;
                    torrent.completed_bytes = completed_bytes;
                    torrent.downloaded_bytes = downloaded;
                    torrent.uploaded_bytes = uploaded;
                    torrent.download_rate_bps = shown_down_rate;
                    torrent.upload_rate_bps = shown_up_rate;
                    torrent.eta_secs = shown_eta_secs;
                    if let Some(err) = last_error {
                        torrent.last_error = err;
                    }
                });
            });

            // Max seed time check
            if is_complete {
                if seed_start.is_none() {
                    seed_start = Some(Instant::now());
                }
                if let Some(start) = seed_start {
                    let max = MAX_SEED_TIME_SECS.load(Ordering::SeqCst);
                    if max > 0 && start.elapsed().as_secs() >= max {
                        log_info!("max seed time reached, stopping");
                        stop_flag.store(true, Ordering::SeqCst);
                    }
                }
                // Check ratio group
                let rg = lock_or_recover(&context.ratio_group).clone();
                check_ratio_group(
                    &rg,
                    &context.uploaded,
                    &context.downloaded,
                    &stop_flag,
                    &paused_flag,
                );
            }

            sleep_with_shutdown_or_stop(TORRENT_LOOP_INTERVAL, &stop_flag);
        }

        // Closing cloned TCP handles wakes peers that are blocked in a read,
        // write, or handshake. uTP and any other worker still have the bounded
        // join/resource-drain fallback below.
        cancel_peer_connections(&context.peer_cancellations);

        let mut stop_trackers = lock_or_recover(&shared_trackers).clone();
        if args.proxy.is_some() {
            stop_trackers.udp.clear();
        }
        if tracker_set_has_usable_source(&stop_trackers, args.proxy.is_none()) {
            let completed_bytes = {
                let p = lock_or_recover(&pieces);
                p.completed_bytes()
            };
            let downloaded = downloaded.load(Ordering::SeqCst);
            let uploaded = uploaded.load(Ordering::SeqCst);
            let left = total_length.saturating_sub(completed_bytes);
            let stop_numwant = peer_settings.numwant();
            log_info!("sending tracker stopped event...");
            let (stop_rx, mut stop_pending) = spawn_tracker_announces(
                &stop_trackers,
                meta.info_hash,
                peer_id,
                args.port,
                uploaded,
                downloaded,
                left,
                Some("stopped"),
                stop_numwant,
                meta.info.private,
                args.proxy.clone(),
                TRACKER_STOPPED_WAIT_BUDGET,
            );
            let stop_deadline = Instant::now() + TRACKER_STOPPED_WAIT_BUDGET;
            while stop_pending > 0 {
                let Some(remaining) = stop_deadline.checked_duration_since(Instant::now()) else {
                    break;
                };
                if remaining.is_zero() {
                    break;
                }
                match stop_rx.recv_timeout(remaining.min(TRACKER_ANNOUNCE_POLL)) {
                    Ok(_) => stop_pending = stop_pending.saturating_sub(1),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        }

        let peer_shutdown_deadline = Instant::now() + TORRENT_WORKER_SHUTDOWN_TIMEOUT;
        for handle in handles {
            resource_workers_stopped &= join_worker_before(handle, "peer", peer_shutdown_deadline);
        }
        if decentralized_discovery {
            dht.remove_torrent(meta.info_hash);
            lpd.remove_torrent(meta.info_hash);
        }
        for handle in discovery_handles.drain(..) {
            join_worker_before(handle, "peer discovery", peer_shutdown_deadline);
        }
    }

    let auxiliary_shutdown_deadline = Instant::now() + TORRENT_WORKER_SHUTDOWN_TIMEOUT;
    if let Some(handle) = webseed_handle {
        resource_workers_stopped &=
            join_worker_before(handle, "web seed", auxiliary_shutdown_deadline);
    }

    resource_workers_stopped &=
        join_worker_before(resume_handle, "resume", auxiliary_shutdown_deadline);
    if !resource_workers_stopped {
        retain_context_after_teardown_failure(registry, &context);
        return Err(
            "torrent teardown timed out; restart before retrying lifecycle operations".to_string(),
        );
    }
    let lifecycle_operation = session_store.lock_operation();

    if context.delete_data_requested.load(Ordering::Acquire) {
        let queue_len = ui_state
            .as_ref()
            .and_then(|state| state.lock().ok().map(|state| state.queue_len))
            .unwrap_or(0);
        let entry = retain_delete_error(
            session_store
                .get(meta.info_hash)
                .ok_or_else(|| "torrent session metadata is unavailable".to_string()),
            ui_state,
            request.id,
            queue_len,
        )?;
        if !entry.pending_delete {
            return retain_delete_error(
                Err("active data deletion lost its durable tombstone".to_string()),
                ui_state,
                request.id,
                queue_len,
            );
        }
        if !meta.info.private {
            dht.remove_torrent(meta.info_hash);
            lpd.remove_torrent(meta.info_hash);
        }
        unregister_session(registry, meta.info_hash, request.id);

        // Destructive deletion remains fail-closed: if a detached handler or
        // recheck does not relinquish Storage by the deadline, retain the
        // durable tombstone and leave every payload path untouched.
        retain_delete_error(
            wait_for_torrent_resources_or_retain(
                registry,
                &context,
                &storage,
                "delete data",
                Instant::now() + TORRENT_RESOURCE_DRAIN_TIMEOUT,
            ),
            ui_state,
            request.id,
            queue_len,
        )?;

        let delete_paths = {
            let mut storage_guard = lock_or_recover(&storage);
            retain_delete_error(
                storage_guard
                    .flush()
                    .map_err(|err| format!("delete data flush failed: {err}")),
                ui_state,
                request.id,
                queue_len,
            )?;
            (0..storage_guard.file_count())
                .filter_map(|index| storage_guard.file_path(index).map(Path::to_path_buf))
                .collect::<Vec<_>>()
        };

        drop(context);
        let storage_mutex = retain_delete_error(
            Arc::try_unwrap(storage)
                .map_err(|_| "delete data aborted because storage is still in use".to_string()),
            ui_state,
            request.id,
            queue_len,
        )?;
        drop(match storage_mutex.into_inner() {
            Ok(storage) => storage,
            Err(poisoned) => poisoned.into_inner(),
        });
        if let Err(err) = delete_storage_paths(&request.download_dir, &delete_paths)
            .and_then(|()| remove_resume_files(&resume_path))
        {
            mark_delete_failed_ui(ui_state, request.id, &err, queue_len);
            return Err(err);
        }
        if let Err(err) = session_store.remove(meta.info_hash) {
            mark_delete_failed_ui(ui_state, request.id, &err, queue_len);
            return Err(err);
        }
        remove_torrent_ui(ui_state, request.id, queue_len);
        return Ok(None);
    }

    if context.archive_requested.load(Ordering::Acquire) {
        let queue_len = ui_state
            .as_ref()
            .and_then(|state| state.lock().ok().map(|state| state.queue_len))
            .unwrap_or(0);
        let entry = session_store
            .get(meta.info_hash)
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if entry.pending_delete {
            return Err("data deletion is pending; retry delete with data".to_string());
        }
        unregister_session(registry, meta.info_hash, request.id);
        wait_for_torrent_resources_or_retain(
            registry,
            &context,
            &storage,
            "archive",
            Instant::now() + TORRENT_RESOURCE_DRAIN_TIMEOUT,
        )?;
        {
            let mut storage_guard = lock_or_recover(&storage);
            storage_guard
                .flush()
                .map_err(|err| format!("archive data flush failed: {err}"))?;
        }
        drop(context);
        let storage_mutex = Arc::try_unwrap(storage)
            .map_err(|_| "archive aborted because storage is still in use".to_string())?;
        drop(match storage_mutex.into_inner() {
            Ok(storage) => storage,
            Err(poisoned) => poisoned.into_inner(),
        });
        session_store.remove(meta.info_hash)?;
        remove_torrent_ui(ui_state, request.id, queue_len);
        return Ok(None);
    }

    let finished_complete = {
        let p = lock_or_recover(&pieces);
        p.is_complete()
    };
    let finished_status = if shutdown_requested() {
        "shutdown"
    } else if stop_flag.load(Ordering::SeqCst) {
        "stopped"
    } else if finished_complete {
        "complete"
    } else {
        "stopped"
    };
    let paused = torrent_paused(&paused_flag);
    update_ui(ui_state, |state| {
        if state.current_id == Some(request.id) {
            state.status = finished_status.to_string();
            state.paused = is_paused();
            state.download_rate_bps = 0.0;
            state.upload_rate_bps = 0.0;
            state.active_peers = 0;
            state.interested_peers = 0;
            state.eta_secs = 0;
            state.current_id = None;
        }
        update_torrent_entry(state, request.id, |torrent| {
            torrent.status = finished_status.to_string();
            torrent.paused = paused;
            torrent.download_rate_bps = 0.0;
            torrent.upload_rate_bps = 0.0;
            torrent.active_peers = 0;
            torrent.interested_peers = 0;
            torrent.eta_secs = 0;
        });
    });

    let mut completion_reentry = None;
    if completion_move_pending
        && finished_complete
        && session_store.get(meta.info_hash).is_some_and(|entry| {
            !entry.pending_delete && entry.completion_state == CompletionState::Pending
        })
    {
        let Some(dest) = completion_move_dir.as_ref() else {
            return Err("completion move is pending without a destination".to_string());
        };
        let prepared_source = {
            let mut storage = lock_or_recover(&storage);
            storage.flush().map_err(|err| err.to_string()).map(|()| {
                meta.info
                    .length
                    .and_then(|_| storage.file_path(0).map(Path::to_path_buf))
            })
        };
        match prepared_source {
            Ok(source_override) => {
                // Prevent new inbound lookups before waiting for every existing
                // storage user. Closing Storage before the rename is required on
                // platforms that do not permit moving open files.
                unregister_session(registry, meta.info_hash, request.id);
                wait_for_torrent_resources_or_retain(
                    registry,
                    &context,
                    &storage,
                    "move-completed",
                    Instant::now() + TORRENT_RESOURCE_DRAIN_TIMEOUT,
                )?;
                let allow_reentry = context.allow_completion_reentry.load(Ordering::SeqCst);
                drop(context);
                let storage_mutex = Arc::try_unwrap(storage).map_err(|_| {
                    "move-completed aborted because storage is still in use".to_string()
                })?;
                drop(match storage_mutex.into_inner() {
                    Ok(storage) => storage,
                    Err(poisoned) => poisoned.into_inner(),
                });

                match move_completed_files(
                    &meta,
                    &request.download_dir,
                    dest,
                    source_override.as_deref(),
                ) {
                    Ok(completed_move) => {
                        let commit_result =
                            commit_completed_move(completed_move.as_ref(), || match session_store
                                .commit_completion_move(meta.info_hash, dest)?
                            {
                                true => Ok(()),
                                false => {
                                    Err("completion state changed before move commit".to_string())
                                }
                            });
                        match commit_result {
                            Ok(()) => {
                                let relocated_resume = crate::resume_path(dest, meta.info_hash);
                                if let Err(err) =
                                    relocate_resume_state(&resume_path, &relocated_resume)
                                {
                                    log_warn!("move-completed resume relocation failed: {err}");
                                }
                                update_ui(ui_state, |state| {
                                    update_torrent_entry(state, request.id, |torrent| {
                                        torrent.download_dir = dest.display().to_string();
                                    });
                                });
                                if let Some(script) = ON_COMPLETE_SCRIPT.get() {
                                    spawn_on_complete_script(
                                        script,
                                        &name,
                                        dest,
                                        meta.info_hash,
                                        meta.info.total_length(),
                                    );
                                }
                                if allow_reentry && !shutdown_requested() {
                                    completion_reentry = Some(TorrentRequest {
                                        id: request.id,
                                        source: TorrentSource::Bytes(data.clone()),
                                        download_dir: dest.clone(),
                                        preallocate: request.preallocate,
                                        initial_label: request.initial_label.clone(),
                                    });
                                }
                            }
                            Err(err) => {
                                log_warn!("move-completed commit failed: {err}");
                            }
                        }
                    }
                    Err(err) => {
                        log_warn!("move-completed failed: {err}");
                    }
                }
                if !meta.info.private {
                    dht.remove_torrent(meta.info_hash);
                    lpd.remove_torrent(meta.info_hash);
                }
                unregister_session(registry, meta.info_hash, request.id);
                return Ok(completion_reentry);
            }
            Err(err) => {
                log_warn!("move-completed skipped because data flush failed: {err}");
            }
        }
    }

    if !meta.info.private {
        dht.remove_torrent(meta.info_hash);
        lpd.remove_torrent(meta.info_hash);
    }
    unregister_session(registry, meta.info_hash, request.id);

    // Keep the durable claim until every inbound user has released its
    // context and the payload locks have actually closed. An archive request
    // waiting on `operations` can only remove the session after this point.
    wait_for_torrent_resources_or_retain(
        registry,
        &context,
        &storage,
        "torrent shutdown",
        Instant::now() + TORRENT_RESOURCE_DRAIN_TIMEOUT,
    )?;
    drop(context);
    let storage_mutex = Arc::try_unwrap(storage)
        .map_err(|_| "torrent stopped while storage is still in use".to_string())?;
    drop(match storage_mutex.into_inner() {
        Ok(storage) => storage,
        Err(poisoned) => poisoned.into_inner(),
    });
    drop(lifecycle_operation);

    Ok(completion_reentry)
}

fn resolve_torrent_data(
    request: &TorrentRequest,
    port: u16,
    dht: &dht::Dht,
    connect_cfg: &ConnectionConfig,
    metadata_peer_limit: usize,
) -> Result<Vec<u8>, String> {
    let data = match &request.source {
        TorrentSource::Path(path) => read_file_limited(Path::new(path), MAX_TORRENT_BYTES, false)
            .map_err(|err| format!("failed to read {path}: {err}"))?,
        TorrentSource::Bytes(data) => {
            if data.len() > MAX_TORRENT_BYTES {
                return Err("torrent file too large".to_string());
            }
            data.clone()
        }
        TorrentSource::Magnet(link) => {
            fetch_torrent_from_magnet(link, port, dht, connect_cfg, metadata_peer_limit)?
        }
    };
    if data.len() > MAX_TORRENT_BYTES {
        return Err("torrent file too large".to_string());
    }
    Ok(data)
}

#[derive(Debug)]
struct MagnetMeta {
    /// The 20-byte swarm identifier used by trackers, DHT, and handshakes.
    info_hash: [u8; 20],
    info_hash_v1: Option<[u8; 20]>,
    info_hash_v2: Option<[u8; 32]>,
    sources: Vec<String>,
    trackers: Vec<String>,
    web_seeds: Vec<String>,
    peers: Vec<SocketAddr>,
}

#[derive(Clone, Copy)]
struct ExpectedInfoHashes {
    v1: Option<[u8; 20]>,
    v2: Option<[u8; 32]>,
}

impl MagnetMeta {
    fn expected_hashes(&self) -> ExpectedInfoHashes {
        ExpectedInfoHashes {
            v1: self.info_hash_v1,
            v2: self.info_hash_v2,
        }
    }
}

impl ExpectedInfoHashes {
    fn swarm_id(self) -> Result<[u8; 20], String> {
        if let Some(hash) = self.v1 {
            return Ok(hash);
        }
        let v2 = self
            .v2
            .ok_or_else(|| "magnet is missing an exact topic".to_string())?;
        let mut hash = [0u8; 20];
        hash.copy_from_slice(&v2[..20]);
        Ok(hash)
    }

    fn hybrid_v2_swarm_id(self) -> Option<[u8; 20]> {
        self.v1.and(self.v2.map(truncate_v2_info_hash))
    }
}

fn truncate_v2_info_hash(hash: [u8; 32]) -> [u8; 20] {
    let mut truncated = [0u8; 20];
    truncated.copy_from_slice(&hash[..20]);
    truncated
}

fn validate_info_hashes(info: &[u8], expected: ExpectedInfoHashes) -> Result<(), String> {
    if let Some(hash) = expected.v1 {
        if sha1::sha1(info) != hash {
            return Err("metadata SHA-1 hash mismatch".to_string());
        }
    }
    if let Some(hash) = expected.v2 {
        if sha256::sha256(info) != hash {
            return Err("metadata SHA-256 hash mismatch".to_string());
        }
    }
    Ok(())
}

fn validate_magnet_torrent(data: &[u8], expected: ExpectedInfoHashes) -> Result<(), String> {
    if data.len() > MAX_TORRENT_BYTES {
        return Err("torrent file too large".to_string());
    }
    let meta =
        torrent::parse_torrent(data).map_err(|err| format!("invalid torrent metadata: {err}"))?;
    if let Some(hash) = expected.v1 {
        if meta.meta_version != 1 && meta.meta_version != 3 {
            return Err("source torrent is missing the requested v1 metadata".to_string());
        }
        if meta.info_hash != hash {
            return Err("source torrent SHA-1 hash mismatch".to_string());
        }
    }
    if let Some(hash) = expected.v2 {
        if meta.info_hash_v2 != Some(hash) {
            return Err("source torrent SHA-256 hash mismatch".to_string());
        }
    }
    Ok(())
}

fn fetch_torrent_from_magnet(
    link: &str,
    port: u16,
    dht: &dht::Dht,
    connect_cfg: &ConnectionConfig,
    metadata_peer_limit: usize,
) -> Result<Vec<u8>, String> {
    let meta = parse_magnet(link)?;
    let expected_hashes = meta.expected_hashes();
    let deadline = Instant::now() + METADATA_TOTAL_TIMEOUT;
    let hash = hex(&meta.info_hash);
    log_info!(
        "magnet: info_hash={} trackers={} sources={} web_seeds={} peers={}",
        hash,
        meta.trackers.len(),
        meta.sources.len(),
        meta.web_seeds.len(),
        meta.peers.len()
    );
    for (idx, tracker) in meta.trackers.iter().enumerate() {
        log_info!("magnet: tracker[{idx}]={}", safe_network_url_label(tracker));
    }
    for (idx, source) in meta.sources.iter().enumerate() {
        log_info!("magnet: source[{idx}]={}", safe_network_url_label(source));
    }
    for (idx, seed) in meta.web_seeds.iter().enumerate() {
        log_info!("magnet: webseed[{idx}]={}", safe_network_url_label(seed));
    }
    for (idx, peer) in meta.peers.iter().enumerate() {
        log_info!("magnet: peer[{idx}]={peer}");
    }
    let mut source_err: Option<String> = None;
    let mut metadata_err: Option<String> = None;
    for source in meta.sources.iter().filter(|_| connect_cfg.proxy.is_none()) {
        if shutdown_requested() {
            return Err("metadata fetch cancelled".to_string());
        }
        if Instant::now() >= deadline {
            break;
        }
        let source_label = safe_network_url_label(source);
        log_info!("magnet: fetching source {source_label}");
        match http::get_public_until(source, MAX_TORRENT_BYTES, deadline, Some(&SHUTDOWN)) {
            Ok(data) => match validate_magnet_torrent(&data, expected_hashes) {
                Ok(()) => {
                    log_info!("magnet: source fetch and hash validation ok ({source_label})");
                    return Ok(data);
                }
                Err(err) => {
                    log_warn!("magnet: source validation failed ({source_label}): {err}");
                    if source_err.is_none() {
                        source_err = Some(err);
                    }
                }
            },
            Err(err) => {
                log_warn!("magnet: source fetch failed ({source_label}): {err}");
                if source_err.is_none() {
                    source_err = Some(err);
                }
            }
        }
    }
    if let Some(v1_hash) = meta.info_hash_v1.filter(|_| connect_cfg.proxy.is_none()) {
        let info_hash = hex(&v1_hash);
        for base in MAGNET_CACHE_URLS {
            if shutdown_requested() {
                return Err("metadata fetch cancelled".to_string());
            }
            if Instant::now() >= deadline {
                break;
            }
            let url = format!("{base}{info_hash}.torrent");
            log_info!("magnet: fetching cache {url}");
            match http::get_public_until(&url, MAX_TORRENT_BYTES, deadline, Some(&SHUTDOWN)) {
                Ok(data) => match validate_magnet_torrent(&data, expected_hashes) {
                    Ok(()) => {
                        log_info!("magnet: cache fetch and hash validation ok ({url})");
                        return Ok(data);
                    }
                    Err(err) => {
                        log_warn!("magnet: cache validation failed ({url}): {err}");
                        if source_err.is_none() {
                            source_err = Some(err);
                        }
                    }
                },
                Err(err) => {
                    log_warn!("magnet: cache fetch failed ({url}): {err}");
                    if source_err.is_none() {
                        source_err = Some(err);
                    }
                }
            }
        }
    }
    if meta.info_hash_v2.is_some() {
        return Err(source_err.unwrap_or_else(|| {
            "v2/hybrid magnet peer metadata requires BEP 52 piece-layer exchange; provide an xs/as URL for the complete .torrent file"
                .to_string()
        }));
    }
    if !meta.peers.is_empty() {
        let peer_id = generate_peer_id();
        log_info!("magnet: fetching metadata from explicit peers");
        for addr in &meta.peers {
            if Instant::now() >= deadline {
                break;
            }
            log_info!("metadata: trying explicit peer {addr}");
            match fetch_metadata_from_peer(*addr, expected_hashes, peer_id, deadline, connect_cfg) {
                Ok(info_bytes) => {
                    log_info!("metadata: explicit peer {addr} delivered metadata");
                    let data = wrap_torrent_with_info(&info_bytes, &meta.trackers, &meta.web_seeds);
                    return Ok(data);
                }
                Err(err) => {
                    log_warn!("metadata: explicit peer {addr} failed: {err}");
                    if metadata_err.is_none() {
                        metadata_err = Some(err);
                    }
                }
            }
        }
    }
    if !meta.trackers.is_empty() {
        let peer_id = generate_peer_id();
        log_info!("magnet: fetching metadata from trackers");
        match fetch_metadata_from_trackers(
            expected_hashes,
            peer_id,
            port,
            &meta.trackers,
            deadline,
            connect_cfg,
            metadata_peer_limit,
        ) {
            Ok(info_bytes) => {
                log_info!("magnet: metadata fetched from trackers");
                let data = wrap_torrent_with_info(&info_bytes, &meta.trackers, &meta.web_seeds);
                return Ok(data);
            }
            Err(err) => {
                log_warn!("magnet: tracker metadata failed: {err}");
                if metadata_err.is_none() {
                    metadata_err = Some(err);
                }
            }
        }
    }
    if connect_cfg.proxy.is_none() {
        let peer_id = generate_peer_id();
        log_info!("magnet: fetching metadata from dht");
        match fetch_metadata_from_dht(
            expected_hashes,
            peer_id,
            port,
            deadline,
            dht,
            connect_cfg,
            metadata_peer_limit,
        ) {
            Ok(info_bytes) => {
                log_info!("magnet: metadata fetched from dht");
                let data = wrap_torrent_with_info(&info_bytes, &meta.trackers, &meta.web_seeds);
                return Ok(data);
            }
            Err(err) => {
                log_warn!("magnet: dht metadata failed: {err}");
                if metadata_err.is_none() {
                    metadata_err = Some(err);
                }
            }
        }
    }
    if let Some(err) = metadata_err {
        return Err(err);
    }
    Err(source_err
        .unwrap_or_else(|| "magnet metadata not found (no sources, caches, or peers)".to_string()))
}

fn parse_magnet(link: &str) -> Result<MagnetMeta, String> {
    let trimmed = link.trim();
    let query = trimmed
        .strip_prefix("magnet:?")
        .ok_or_else(|| "invalid magnet link".to_string())?;
    let mut info_hash_v1: Option<[u8; 20]> = None;
    let mut info_hash_v2: Option<[u8; 32]> = None;
    let mut sources = Vec::new();
    let mut trackers = Vec::new();
    let mut web_seeds = Vec::new();
    let mut peers = Vec::new();
    for (key, value) in parse_query_pairs(query) {
        match key.as_str() {
            "xt" => {
                let lower = value.to_ascii_lowercase();
                if let Some(rest) = lower.strip_prefix("urn:btih:") {
                    let hash = parse_info_hash(rest)
                        .ok_or_else(|| "invalid magnet v1 info hash".to_string())?;
                    if info_hash_v1.is_some_and(|existing| existing != hash) {
                        return Err("magnet contains conflicting v1 info hashes".to_string());
                    }
                    info_hash_v1 = Some(hash);
                } else if let Some(rest) = lower.strip_prefix("urn:btmh:") {
                    let hash = parse_multihash_sha256(rest)
                        .ok_or_else(|| "invalid magnet v2 info hash".to_string())?;
                    if info_hash_v2.is_some_and(|existing| existing != hash) {
                        return Err("magnet contains conflicting v2 info hashes".to_string());
                    }
                    info_hash_v2 = Some(hash);
                }
            }
            "xs" | "as" => {
                if sources.len() < MAX_MAGNET_SOURCES
                    && valid_magnet_http_url(&value)
                    && !sources.contains(&value)
                {
                    sources.push(value);
                }
            }
            "tr" => {
                if trackers.len() < MAX_TRACKERS_PER_TORRENT
                    && valid_tracker_url(&value)
                    && !trackers.contains(&value)
                {
                    trackers.push(value);
                }
            }
            "ws" => {
                if web_seeds.len() < MAX_MAGNET_WEB_SEEDS
                    && valid_magnet_http_url(&value)
                    && !web_seeds.contains(&value)
                {
                    web_seeds.push(value);
                }
            }
            "x.pe" if peers.len() < MAX_MAGNET_EXPLICIT_PEERS => {
                if let Ok(addr) = value.parse::<SocketAddr>() {
                    let Some(addr) = safe_metadata_peer(addr, PeerSource::Magnet, None) else {
                        continue;
                    };
                    if !peers.contains(&addr) {
                        peers.push(addr);
                    }
                }
            }
            _ => {}
        }
    }
    let info_hash = match (info_hash_v1, info_hash_v2) {
        (Some(hash), _) => hash,
        (None, Some(hash)) => {
            let mut truncated = [0u8; 20];
            truncated.copy_from_slice(&hash[..20]);
            truncated
        }
        (None, None) => return Err("magnet missing info hash".to_string()),
    };
    Ok(MagnetMeta {
        info_hash,
        info_hash_v1,
        info_hash_v2,
        sources,
        trackers,
        web_seeds,
        peers,
    })
}

fn valid_magnet_http_url(url: &str) -> bool {
    valid_network_url(url, &["http://", "https://"])
}

fn parse_info_hash(value: &str) -> Option<[u8; 20]> {
    let value = value.trim();
    if value.len() == 40 {
        decode_hex_20(value)
    } else if value.len() == 32 {
        decode_base32_20(value)
    } else {
        None
    }
}

fn decode_hex_20(value: &str) -> Option<[u8; 20]> {
    let bytes = value.as_bytes();
    if bytes.len() != 40 {
        return None;
    }
    let mut out = [0u8; 20];
    for (idx, chunk) in bytes.chunks_exact(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)? as u8;
        let lo = (chunk[1] as char).to_digit(16)? as u8;
        out[idx] = (hi << 4) | lo;
    }
    Some(out)
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    let bytes = value.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (idx, chunk) in bytes.chunks_exact(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)? as u8;
        let lo = (chunk[1] as char).to_digit(16)? as u8;
        out[idx] = (hi << 4) | lo;
    }
    Some(out)
}

fn parse_multihash_sha256(value: &str) -> Option<[u8; 32]> {
    // Multihash format: 1220<64 hex chars>
    // 12 = SHA-256 function code, 20 = 32 bytes (0x20) digest length
    let rest = value.strip_prefix("1220")?;
    decode_hex_32(rest)
}

fn decode_base32_20(value: &str) -> Option<[u8; 20]> {
    let mut out = Vec::with_capacity(20);
    let mut buffer: u32 = 0;
    let mut bits: u8 = 0;
    for ch in value.chars() {
        if ch == '=' {
            break;
        }
        let val = base32_value(ch)?;
        buffer = (buffer << 5) | (val as u32);
        bits = bits.saturating_add(5);
        while bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    if out.len() != 20 {
        return None;
    }
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&out);
    Some(arr)
}

fn base32_value(ch: char) -> Option<u8> {
    let ch = ch.to_ascii_uppercase();
    match ch {
        'A'..='Z' => Some((ch as u8) - b'A'),
        '2'..='7' => Some((ch as u8) - b'2' + 26),
        _ => None,
    }
}

fn fetch_metadata_from_trackers(
    expected_hashes: ExpectedInfoHashes,
    peer_id: [u8; 20],
    port: u16,
    trackers: &[String],
    deadline: Instant,
    connect_cfg: &ConnectionConfig,
    metadata_peer_limit: usize,
) -> Result<Vec<u8>, String> {
    let info_hash = expected_hashes.swarm_id()?;
    log_info!(
        "metadata: tracker announce start (trackers={}, deadline={}s)",
        trackers.len(),
        deadline.saturating_duration_since(Instant::now()).as_secs()
    );
    let announce_trackers = TrackerSet {
        http: trackers
            .iter()
            .filter(|tracker_url| {
                tracker_url.starts_with("http://") || tracker_url.starts_with("https://")
            })
            .cloned()
            .collect(),
        udp: if connect_cfg.proxy.is_none() {
            trackers
                .iter()
                .filter(|tracker_url| tracker_url.starts_with("udp://"))
                .cloned()
                .collect()
        } else {
            Vec::new()
        },
    };
    let mut last_err: Option<String> = None;
    let (announce_rx, mut announce_pending) = spawn_tracker_announces(
        &announce_trackers,
        info_hash,
        peer_id,
        port,
        0,
        0,
        1,
        Some("started"),
        metadata_peer_limit as u32,
        false,
        connect_cfg.proxy.clone(),
        TRACKER_ANNOUNCE_WAIT_BUDGET,
    );
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    let announce_deadline = Instant::now() + TRACKER_ANNOUNCE_WAIT_BUDGET;
    while announce_pending > 0 {
        let Some(remaining) = announce_deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        let wait = remaining.min(TRACKER_ANNOUNCE_POLL);
        match announce_rx.recv_timeout(wait) {
            Ok(result) => {
                announce_pending = announce_pending.saturating_sub(1);
                match result.response {
                    Ok(response) => {
                        if result.is_udp {
                            log_info!(
                                "metadata: udp tracker {} returned {} peers",
                                safe_network_url_label(&result.tracker_url),
                                response.peers.len()
                            );
                        } else {
                            log_info!(
                                "metadata: http tracker {} returned {} peers",
                                safe_network_url_label(&result.tracker_url),
                                response.peers.len()
                            );
                        }
                        for peer in response.peers {
                            let Some(peer) = safe_metadata_peer(
                                peer,
                                PeerSource::Tracker,
                                connect_cfg.ip_filter.as_deref(),
                            ) else {
                                continue;
                            };
                            if seen.insert(peer) {
                                unique.push(peer);
                                if unique.len() >= metadata_peer_limit {
                                    break;
                                }
                            }
                        }
                        if !unique.is_empty() && unique.len() >= 8 {
                            break;
                        }
                    }
                    Err(err) => {
                        last_err = Some(format!(
                            "{}: {err}",
                            safe_network_url_label(&result.tracker_url)
                        ));
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if unique.is_empty() {
        return Err(last_err.unwrap_or_else(|| "no peers returned for magnet".to_string()));
    }
    log_info!("metadata: {} unique peers from trackers", unique.len());

    for addr in unique {
        if Instant::now() >= deadline {
            break;
        }
        log_info!("metadata: trying peer {addr}");
        match fetch_metadata_from_peer(addr, expected_hashes, peer_id, deadline, connect_cfg) {
            Ok(data) => {
                log_info!("metadata: peer {addr} delivered metadata");
                return Ok(data);
            }
            Err(err) => {
                log_warn!("metadata: peer {addr} failed: {err}");
                last_err = Some(err)
            }
        }
    }

    Err(last_err.unwrap_or_else(|| "metadata fetch timed out".to_string()))
}

fn fetch_metadata_from_dht(
    expected_hashes: ExpectedInfoHashes,
    peer_id: [u8; 20],
    port: u16,
    deadline: Instant,
    dht: &dht::Dht,
    connect_cfg: &ConnectionConfig,
    metadata_peer_limit: usize,
) -> Result<Vec<u8>, String> {
    let info_hash = expected_hashes.swarm_id()?;
    if !cfg!(feature = "dht") {
        let _ = dht;
        return Err("dht disabled".to_string());
    }
    if Instant::now() >= deadline {
        return Err("metadata fetch timed out".to_string());
    }
    let (tx, rx) = mpsc::channel();
    log_info!("metadata: dht add torrent for discovery");
    dht.add_torrent(info_hash, port, tx);
    let mut last_err: Option<String> = None;
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    let mut result: Option<Vec<u8>> = None;
    let mut total_seen = 0usize;

    while Instant::now() < deadline {
        if shutdown_requested() {
            last_err = Some("shutdown requested".to_string());
            break;
        }

        while let Some(addr) = queue.pop_front() {
            if Instant::now() >= deadline {
                break;
            }
            log_info!("metadata: trying dht peer {addr}");
            match fetch_metadata_from_peer(addr, expected_hashes, peer_id, deadline, connect_cfg) {
                Ok(data) => {
                    log_info!("metadata: dht peer {addr} delivered metadata");
                    result = Some(data);
                    break;
                }
                Err(err) => {
                    log_warn!("metadata: dht peer {addr} failed: {err}");
                    last_err = Some(err)
                }
            }
        }

        if result.is_some() {
            break;
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait = remaining.min(Duration::from_millis(500));
        if wait.is_zero() {
            break;
        }
        match rx.recv_timeout(wait) {
            Ok(peers) => {
                if !peers.is_empty() {
                    log_info!("metadata: dht peers received batch size={}", peers.len());
                }
                for peer in peers {
                    let Some(peer) =
                        safe_metadata_peer(peer, PeerSource::Dht, connect_cfg.ip_filter.as_deref())
                    else {
                        continue;
                    };
                    if seen.len() >= metadata_peer_limit {
                        break;
                    }
                    if seen.insert(peer) {
                        total_seen += 1;
                        queue.push_back(peer);
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    dht.remove_torrent(info_hash);
    log_info!("metadata: dht done (peers_seen={total_seen})");
    if let Some(data) = result {
        return Ok(data);
    }
    Err(last_err.unwrap_or_else(|| "metadata fetch timed out".to_string()))
}

fn fetch_metadata_from_peer(
    addr: SocketAddr,
    expected_hashes: ExpectedInfoHashes,
    peer_id: [u8; 20],
    deadline: Instant,
    connect_cfg: &ConnectionConfig,
) -> Result<Vec<u8>, String> {
    let info_hash = expected_hashes.swarm_id()?;
    let hybrid_v2_info_hash = expected_hashes.hybrid_v2_swarm_id();
    log_info!(
        "metadata: peer {addr} connect (deadline={}s)",
        deadline.saturating_duration_since(Instant::now()).as_secs()
    );
    let mut stream = connect_peer_for_metadata(addr, connect_cfg)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| format!("read timeout failed: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| format!("write timeout failed: {err}"))?;

    let handshake = if connect_cfg.encryption == EncryptionMode::Require {
        outbound_handshake(
            &mut stream,
            info_hash,
            hybrid_v2_info_hash,
            peer_id,
            connect_cfg.encryption,
        )?
    } else {
        match plaintext_handshake(&mut stream, info_hash, hybrid_v2_info_hash, peer_id) {
            Ok(handshake) => handshake,
            Err(_err) if connect_cfg.encryption == EncryptionMode::Prefer => {
                let mut retry = connect_peer_for_metadata(addr, connect_cfg)?;
                retry
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|err| format!("read timeout failed: {err}"))?;
                retry
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .map_err(|err| format!("write timeout failed: {err}"))?;
                let handshake = outbound_handshake(
                    &mut retry,
                    info_hash,
                    hybrid_v2_info_hash,
                    peer_id,
                    EncryptionMode::Prefer,
                )?;
                stream = retry;
                handshake
            }
            Err(err) => return Err(format!("handshake failed: {err}")),
        }
    };
    if !handshake.supports_extensions() {
        return Err("peer does not support extensions".to_string());
    }
    log_info!("metadata: peer {addr} handshake ok, extensions supported");

    let ext_handshake = build_ext_handshake(None, false);
    peer::write_message(
        &mut stream,
        &peer::Message::Extended {
            ext_id: 0,
            payload: ext_handshake,
        },
    )
    .map_err(|err| format!("ext handshake failed: {err}"))?;
    let _ = peer::write_message(&mut stream, &peer::Message::Interested);

    let start = Instant::now();
    let mut reader = peer::MessageReader::new();
    let mut ut_metadata_id: Option<u8> = None;
    let mut metadata_size: Option<usize> = None;
    let mut pieces: Vec<Option<Vec<u8>>> = Vec::new();
    let mut requested = HashSet::new();
    let mut fallback_sent = false;
    let mut fallback_used = false;
    let mut received = 0usize;
    let mut last_progress_log = 0usize;
    let mut requested_any = false;
    let mut last_progress = Instant::now();
    let mut last_request = Instant::now() - METADATA_REQUEST_RETRY;
    let mut last_receive = Instant::now();

    while start.elapsed() < METADATA_FETCH_TIMEOUT && Instant::now() < deadline {
        if shutdown_requested() {
            return Err("shutdown requested".to_string());
        }
        match reader.read_message(&mut stream) {
            Ok(Some(message)) => {
                log_debug!("metadata: peer {addr} msg {}", message_summary(&message));
                match message {
                    peer::Message::Extended { ext_id, payload } => {
                        if ext_id == 0 {
                            let (ut_id, _ut_pex, size) = parse_extended_handshake(&payload)?;
                            if let Some(id) = ut_id {
                                if ut_metadata_id != Some(id) {
                                    ut_metadata_id = Some(id);
                                    if fallback_used {
                                        requested.clear();
                                        last_request = Instant::now() - METADATA_REQUEST_RETRY;
                                    }
                                    log_info!("metadata: peer {addr} ut_metadata id={id}");
                                }
                            }
                            if let Some(size) = size {
                                match metadata_size {
                                    Some(existing) if existing != size => {
                                        return Err("peer changed metadata size".to_string());
                                    }
                                    Some(_) => {}
                                    None => {
                                        metadata_size = Some(size);
                                        pieces = vec![None; metadata_piece_count(size)];
                                        log_info!(
                                            "metadata: peer {addr} metadata_size={size} pieces={}",
                                            pieces.len()
                                        );
                                    }
                                }
                            }
                            if let Some(id) = ut_metadata_id {
                                let sent = request_metadata_pieces(
                                    &mut stream,
                                    id,
                                    &pieces,
                                    &mut requested,
                                    false,
                                )?;
                                if sent > 0 {
                                    if !requested_any {
                                        requested_any = true;
                                        last_progress = Instant::now();
                                    }
                                    log_info!("metadata: peer {addr} requesting {sent} pieces");
                                    last_request = Instant::now();
                                }
                            }
                        } else {
                            let msg = match parse_metadata_message(&payload) {
                                Ok(msg) => msg,
                                Err(_) => {
                                    continue;
                                }
                            };
                            if Some(ext_id) != ut_metadata_id {
                                log_info!("metadata: peer {addr} ut_metadata override id={ext_id}");
                                ut_metadata_id = Some(ext_id);
                                requested.clear();
                                last_request = Instant::now() - METADATA_REQUEST_RETRY;
                            }
                            if msg.msg_type == 2 {
                                log_warn!("metadata: peer {addr} rejected piece {}", msg.piece);
                                return Err("metadata rejected".to_string());
                            }
                            if msg.msg_type == 1 {
                                let advertised_total = msg.total_size.ok_or_else(|| {
                                    "metadata data missing total size".to_string()
                                })?;
                                match metadata_size {
                                    Some(existing) if existing != advertised_total => {
                                        return Err("peer changed metadata size".to_string());
                                    }
                                    Some(_) => {}
                                    None => {
                                        let total = advertised_total;
                                        metadata_size = Some(total);
                                        pieces = vec![None; metadata_piece_count(total)];
                                        log_info!(
                                            "metadata: peer {addr} metadata_size={total} pieces={}",
                                            pieces.len()
                                        );
                                        if let Some(id) = ut_metadata_id {
                                            let sent = request_metadata_pieces(
                                                &mut stream,
                                                id,
                                                &pieces,
                                                &mut requested,
                                                false,
                                            )?;
                                            if sent > 0 {
                                                if !requested_any {
                                                    requested_any = true;
                                                    last_progress = Instant::now();
                                                }
                                                log_info!(
                                            "metadata: peer {addr} requesting {sent} pieces"
                                        );
                                                last_request = Instant::now();
                                            }
                                        }
                                    }
                                }
                                let total = metadata_size
                                    .ok_or_else(|| "metadata size unavailable".to_string())?;
                                let idx = msg.piece as usize;
                                let expected_len = expected_metadata_piece_len(total, idx)
                                    .ok_or_else(|| {
                                        "metadata piece index out of range".to_string()
                                    })?;
                                if msg.data.len() != expected_len {
                                    return Err(format!(
                                        "invalid metadata piece length {} (expected {expected_len})",
                                        msg.data.len()
                                    ));
                                }
                                if pieces[idx].is_none() {
                                    pieces[idx] = Some(msg.data);
                                    received += 1;
                                    last_receive = Instant::now();
                                    last_progress = Instant::now();
                                    if received == 1
                                        || received == pieces.len()
                                        || received - last_progress_log >= 5
                                    {
                                        last_progress_log = received;
                                        log_info!(
                                            "metadata: peer {addr} received {}/{} pieces",
                                            received,
                                            pieces.len()
                                        );
                                    }
                                }
                                if let Some(total) = metadata_size {
                                    if pieces.iter().all(|piece| piece.is_some()) {
                                        let info = assemble_metadata(&pieces, total);
                                        validate_info_hashes(&info, expected_hashes).inspect_err(
                                            |_| {
                                                log_warn!(
                                                    "metadata: peer {addr} metadata hash mismatch"
                                                );
                                            },
                                        )?;
                                        log_info!("metadata: peer {addr} metadata hash ok");
                                        return Ok(info);
                                    }
                                }
                            }
                        }
                    }
                    peer::Message::Unchoke => {
                        if let Some(id) = ut_metadata_id {
                            let sent = request_metadata_pieces(
                                &mut stream,
                                id,
                                &pieces,
                                &mut requested,
                                true,
                            )?;
                            if sent > 0 {
                                if !requested_any {
                                    requested_any = true;
                                    last_progress = Instant::now();
                                }
                                log_info!("metadata: peer {addr} re-requesting {sent} pieces");
                                last_request = Instant::now();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(None) => continue,
            Err(err) => return Err(format!("message read failed: {err}")),
        }

        if ut_metadata_id.is_none() && !fallback_sent && start.elapsed() >= Duration::from_secs(1) {
            ut_metadata_id = Some(1);
            if requested.insert(0usize) {
                log_info!("metadata: peer {addr} fallback ut_metadata=1 request piece 0");
                let _ = send_metadata_request(&mut stream, 1, 0);
                last_request = Instant::now();
                if !requested_any {
                    requested_any = true;
                    last_progress = Instant::now();
                }
            }
            fallback_sent = true;
            fallback_used = true;
        }

        if let Some(id) = ut_metadata_id {
            let complete = !pieces.is_empty() && pieces.iter().all(|piece| piece.is_some());
            if !complete
                && last_receive.elapsed() >= METADATA_REQUEST_RETRY
                && last_request.elapsed() >= METADATA_REQUEST_RETRY
            {
                let sent = request_metadata_pieces(&mut stream, id, &pieces, &mut requested, true)?;
                if sent > 0 {
                    if !requested_any {
                        requested_any = true;
                        last_progress = Instant::now();
                    }
                    log_info!("metadata: peer {addr} re-requesting {sent} pieces");
                    last_request = Instant::now();
                }
            }
        }

        if requested_any && last_progress.elapsed() >= METADATA_PEER_IDLE_TIMEOUT {
            return Err("metadata peer stalled".to_string());
        }
    }

    if let Some(total) = metadata_size {
        log_warn!("metadata: peer {addr} timeout (received {received}/{total} bytes?)");
    } else {
        log_warn!("metadata: peer {addr} timeout (no metadata size)");
    }
    Err("metadata fetch timed out".to_string())
}

fn metadata_piece_count(total: usize) -> usize {
    total.div_ceil(METADATA_PIECE_LEN)
}

fn validate_metadata_size(size: usize) -> Result<usize, String> {
    if size == 0 || size > MAX_TORRENT_BYTES {
        return Err(format!(
            "invalid metadata size {size} (maximum {MAX_TORRENT_BYTES})"
        ));
    }
    Ok(size)
}

fn expected_metadata_piece_len(total: usize, piece: usize) -> Option<usize> {
    let start = piece.checked_mul(METADATA_PIECE_LEN)?;
    if start >= total {
        return None;
    }
    Some((total - start).min(METADATA_PIECE_LEN))
}

fn build_ext_handshake(metadata_size: Option<usize>, allow_pex: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.push(b'd');
    out.extend_from_slice(b"1:m");
    out.push(b'd');
    out.extend_from_slice(b"11:ut_metadatai1e");
    if allow_pex {
        out.extend_from_slice(b"6:ut_pexi2e");
    }
    out.push(b'e');
    if let Some(size) = metadata_size {
        out.extend_from_slice(b"13:metadata_sizei");
        out.extend_from_slice(size.to_string().as_bytes());
        out.push(b'e');
    }
    out.push(b'e');
    out
}

fn send_metadata_request<W: Write>(
    stream: &mut W,
    ut_metadata_id: u8,
    piece: u32,
) -> Result<(), String> {
    log_debug!(
        "metadata: request piece {} (ext_id={})",
        piece,
        ut_metadata_id
    );
    let mut payload = Vec::with_capacity(32);
    payload.extend_from_slice(b"d8:msg_typei0e5:piecei");
    payload.extend_from_slice(piece.to_string().as_bytes());
    payload.extend_from_slice(b"ee");
    peer::write_message(
        stream,
        &peer::Message::Extended {
            ext_id: ut_metadata_id,
            payload,
        },
    )
    .map_err(|err| format!("metadata request failed: {err}"))
}

fn request_metadata_pieces<W: Write>(
    stream: &mut W,
    ut_metadata_id: u8,
    pieces: &[Option<Vec<u8>>],
    requested: &mut HashSet<usize>,
    force: bool,
) -> Result<usize, String> {
    let mut sent = 0usize;
    if pieces.is_empty() {
        if force || requested.insert(0) {
            send_metadata_request(stream, ut_metadata_id, 0)?;
            sent += 1;
        }
        return Ok(sent);
    }
    for (idx, piece) in pieces.iter().enumerate() {
        if piece.is_some() {
            continue;
        }
        if !force && !requested.insert(idx) {
            continue;
        }
        if force {
            requested.insert(idx);
        }
        send_metadata_request(stream, ut_metadata_id, idx as u32)?;
        sent += 1;
    }
    Ok(sent)
}

fn parse_extended_handshake(payload: &[u8]) -> Result<ExtendedHandshakeCaps, String> {
    let (dict, _) = parse_bencode_dict(payload)?;
    let mut ut_metadata = None;
    let mut ut_pex = None;
    let mut metadata_size = None;
    if let Some(Value::Dict(items)) = dict_get(&dict, b"m") {
        for (key, value) in items {
            if key == b"ut_metadata" {
                if let Value::Int(id) = value {
                    if *id >= 0 && *id <= u8::MAX as i64 {
                        ut_metadata = Some(*id as u8);
                    }
                }
            } else if key == b"ut_pex" {
                if let Value::Int(id) = value {
                    if *id >= 0 && *id <= u8::MAX as i64 {
                        ut_pex = Some(*id as u8);
                    }
                }
            }
        }
    }
    if let Some(Value::Int(size)) = dict_get(&dict, b"metadata_size") {
        let size = usize::try_from(*size).map_err(|_| "invalid metadata size".to_string())?;
        metadata_size = Some(validate_metadata_size(size)?);
    }
    Ok((ut_metadata, ut_pex, metadata_size))
}

struct MetadataMessage {
    msg_type: u8,
    piece: u32,
    total_size: Option<usize>,
    data: Vec<u8>,
}

fn parse_metadata_message(payload: &[u8]) -> Result<MetadataMessage, String> {
    let (dict, used) = parse_bencode_dict(payload)?;
    let msg_type = dict_get_int(&dict, b"msg_type").unwrap_or(-1);
    let piece = dict_get_int(&dict, b"piece").unwrap_or(-1);
    if !(0..=2).contains(&msg_type) || piece < 0 || piece > u32::MAX as i64 {
        return Err("invalid metadata message".to_string());
    }
    let total_size = match dict_get_int(&dict, b"total_size") {
        Some(size) => {
            let size = usize::try_from(size).map_err(|_| "invalid metadata size".to_string())?;
            Some(validate_metadata_size(size)?)
        }
        None => None,
    };
    let data = if msg_type == 1 {
        if total_size.is_none() {
            return Err("metadata data message missing total_size".to_string());
        }
        payload[used..].to_vec()
    } else {
        if used != payload.len() {
            return Err("unexpected metadata message payload".to_string());
        }
        Vec::new()
    };
    Ok(MetadataMessage {
        msg_type: msg_type as u8,
        piece: piece as u32,
        total_size,
        data,
    })
}

fn parse_bencode_dict(data: &[u8]) -> Result<(BencodeDict, usize), String> {
    let (value, used) = bencode::parse_value(data, 0).map_err(|err| err.to_string())?;
    match value {
        Value::Dict(items) => Ok((items, used)),
        _ => Err("expected dict".to_string()),
    }
}

fn dict_get<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter()
        .find_map(|(k, v)| if k.as_slice() == key { Some(v) } else { None })
}

fn dict_get_int(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<i64> {
    match dict_get(dict, key) {
        Some(Value::Int(num)) => Some(*num),
        _ => None,
    }
}

fn assemble_metadata(pieces: &[Option<Vec<u8>>], total: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(total);
    for piece in pieces.iter().flatten() {
        out.extend_from_slice(piece);
    }
    out.truncate(total);
    out
}

fn wrap_torrent_with_info(info: &[u8], trackers: &[String], web_seeds: &[String]) -> Vec<u8> {
    let mut out = Vec::with_capacity(info.len() + 128);
    out.push(b'd');
    if !trackers.is_empty() {
        out.extend_from_slice(b"8:announce");
        bencode_bytes(trackers[0].as_bytes(), &mut out);
        out.extend_from_slice(b"13:announce-list");
        out.push(b'l');
        for tracker in trackers {
            out.push(b'l');
            bencode_bytes(tracker.as_bytes(), &mut out);
            out.push(b'e');
        }
        out.push(b'e');
    }
    out.extend_from_slice(b"4:info");
    out.extend_from_slice(info);
    if !web_seeds.is_empty() {
        out.extend_from_slice(b"8:url-list");
        out.push(b'l');
        for seed in web_seeds {
            bencode_bytes(seed.as_bytes(), &mut out);
        }
        out.push(b'e');
    }
    out.push(b'e');
    out
}

fn bencode_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(bytes.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(bytes);
}

fn parse_query_pairs(query: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        out.push((percent_decode(key), percent_decode(value)));
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            b'%' if idx + 2 < bytes.len() => {
                let hi = bytes[idx + 1] as char;
                let lo = bytes[idx + 2] as char;
                if let (Some(hi), Some(lo)) = (hi.to_digit(16), lo.to_digit(16)) {
                    out.push((hi * 16 + lo) as u8);
                    idx += 3;
                    continue;
                }
            }
            b'+' => {
                out.push(b' ');
                idx += 1;
                continue;
            }
            _ => {}
        }
        out.push(bytes[idx]);
        idx += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

struct PeerQueue {
    known: HashSet<SocketAddr>,
    queued: HashSet<SocketAddr>,
    inflight: HashSet<SocketAddr>,
    queue: VecDeque<SocketAddr>,
    deferred: VecDeque<DeferredPeer>,
    failures: HashMap<SocketAddr, u32>,
    banned: HashMap<SocketAddr, Instant>,
    filter: Option<Arc<IpFilter>>,
    local_peer_addrs: HashSet<SocketAddr>,
}

#[derive(Clone, Copy)]
enum PeerSource {
    Tracker,
    Dht,
    Magnet,
    Lpd,
    Pex,
}

struct DeferredPeer {
    addr: SocketAddr,
    ready_at: Instant,
}

impl PeerQueue {
    #[cfg_attr(not(test), allow(dead_code))]
    fn new(filter: Option<Arc<IpFilter>>) -> Self {
        Self::new_with_local_addrs(filter, HashSet::new())
    }

    fn new_with_local_addrs(
        filter: Option<Arc<IpFilter>>,
        local_peer_addrs: HashSet<SocketAddr>,
    ) -> Self {
        Self {
            known: HashSet::new(),
            queued: HashSet::new(),
            inflight: HashSet::new(),
            queue: VecDeque::new(),
            deferred: VecDeque::new(),
            failures: HashMap::new(),
            banned: HashMap::new(),
            filter,
            local_peer_addrs,
        }
    }

    fn enqueue_with_source<I: IntoIterator<Item = SocketAddr>>(
        &mut self,
        peers: I,
        source: PeerSource,
    ) -> usize {
        let mut added = 0usize;
        let high_priority = matches!(source, PeerSource::Tracker);
        for addr in peers {
            let addr = normalize_peer_addr(addr);
            if !is_viable_peer_addr(addr, source) {
                continue;
            }
            if self.is_local_self_peer(addr) {
                continue;
            }
            if self.is_filtered(addr) {
                continue;
            }
            if self.is_banned(addr) {
                continue;
            }
            self.known.insert(addr);
            if self.queued.contains(&addr) || self.inflight.contains(&addr) {
                continue;
            }
            if self.is_deferred(addr) {
                continue;
            }
            if let Some(delay) = self.delay_for_failure(addr) {
                self.schedule_retry(addr, delay);
                continue;
            }
            self.queued.insert(addr);
            if high_priority {
                self.queue.push_front(addr);
            } else {
                self.queue.push_back(addr);
            }
            added += 1;
        }
        added
    }

    fn pop(&mut self) -> Option<SocketAddr> {
        self.promote_ready();
        while let Some(addr) = self.queue.pop_front() {
            if self.is_local_self_peer(addr) || self.is_filtered(addr) || self.is_banned(addr) {
                self.queued.remove(&addr);
                continue;
            }
            self.queued.remove(&addr);
            self.inflight.insert(addr);
            return Some(addr);
        }
        None
    }

    fn finish(&mut self, addr: SocketAddr) {
        self.inflight.remove(&addr);
    }

    fn note_failure(&mut self, addr: SocketAddr) -> Option<Duration> {
        let attempts = self.failures.entry(addr).or_insert(0);
        *attempts = attempts.saturating_add(1);
        if *attempts >= MAX_PEER_RETRIES {
            return None;
        }
        let step = attempts
            .saturating_sub(1)
            .min(MAX_PEER_RETRIES.saturating_sub(1));
        let delay = PEER_RETRY_BASE_SECS
            .saturating_mul(1u64 << step)
            .min(PEER_RETRY_MAX_SECS);
        Some(Duration::from_secs(delay))
    }

    fn schedule_retry(&mut self, addr: SocketAddr, delay: Duration) {
        if self.is_local_self_peer(addr) || self.is_filtered(addr) || self.is_banned(addr) {
            return;
        }
        if self.queued.contains(&addr) || self.inflight.contains(&addr) {
            return;
        }
        if self.deferred.iter().any(|entry| entry.addr == addr) {
            return;
        }
        let ready_at = Instant::now() + delay;
        self.deferred.push_back(DeferredPeer { addr, ready_at });
    }

    fn promote_ready(&mut self) {
        if self.deferred.is_empty() {
            return;
        }
        let now = Instant::now();
        let mut ready = Vec::new();
        self.deferred.retain(|entry| {
            if entry.ready_at <= now {
                ready.push(entry.addr);
                false
            } else {
                true
            }
        });
        for addr in ready {
            if self.is_local_self_peer(addr) || self.is_filtered(addr) || self.is_banned(addr) {
                continue;
            }
            if self.queued.contains(&addr) || self.inflight.contains(&addr) {
                continue;
            }
            self.queued.insert(addr);
            self.queue.push_back(addr);
        }
    }

    fn clear_failure(&mut self, addr: SocketAddr) {
        self.failures.remove(&addr);
        self.deferred.retain(|entry| entry.addr != addr);
        self.banned.remove(&addr);
    }

    fn ban(&mut self, addr: SocketAddr) {
        self.ban_for(addr, Duration::from_secs(PEER_BAN_SECS));
    }

    fn ban_for(&mut self, addr: SocketAddr, duration: Duration) {
        self.queued.remove(&addr);
        self.inflight.remove(&addr);
        self.deferred.retain(|entry| entry.addr != addr);
        self.failures.remove(&addr);
        let until = Instant::now() + duration;
        self.banned.insert(addr, until);
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn known_len(&self) -> usize {
        self.known.len()
    }

    fn sample(&self, max: usize) -> Vec<SocketAddr> {
        self.known
            .iter()
            .filter(|addr| !self.is_local_self_peer(**addr) && !self.is_filtered(**addr))
            .take(max)
            .cloned()
            .collect()
    }

    fn is_deferred(&self, addr: SocketAddr) -> bool {
        self.deferred.iter().any(|entry| entry.addr == addr)
    }

    fn delay_for_failure(&self, addr: SocketAddr) -> Option<Duration> {
        let attempts = *self.failures.get(&addr)?;
        if attempts == 0 {
            return None;
        }
        let step = attempts
            .saturating_sub(1)
            .min(MAX_PEER_RETRIES.saturating_sub(1));
        let delay = PEER_RETRY_BASE_SECS
            .saturating_mul(1u64 << step)
            .min(PEER_RETRY_MAX_SECS);
        Some(Duration::from_secs(delay))
    }

    fn is_banned(&mut self, addr: SocketAddr) -> bool {
        match self.banned.get(&addr) {
            Some(until) if *until > Instant::now() => true,
            Some(_) => {
                self.banned.remove(&addr);
                false
            }
            None => false,
        }
    }

    fn is_filtered(&self, addr: SocketAddr) -> bool {
        self.filter
            .as_ref()
            .map(|filter| filter.is_blocked(addr.ip()))
            .unwrap_or(false)
    }

    fn is_local_self_peer(&self, addr: SocketAddr) -> bool {
        self.local_peer_addrs.contains(&addr)
    }
}

fn normalize_peer_addr(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V6(addr_v6) => addr_v6
            .ip()
            .to_ipv4_mapped()
            .map(|ipv4| SocketAddr::from((ipv4, addr_v6.port())))
            .unwrap_or(SocketAddr::V6(addr_v6)),
        addr_v4 => addr_v4,
    }
}

fn safe_metadata_peer(
    addr: SocketAddr,
    source: PeerSource,
    filter: Option<&IpFilter>,
) -> Option<SocketAddr> {
    let addr = normalize_peer_addr(addr);
    if !is_viable_peer_addr(addr, source)
        || filter.is_some_and(|filter| filter.is_blocked(addr.ip()))
    {
        None
    } else {
        Some(addr)
    }
}

fn is_viable_peer_addr(addr: SocketAddr, source: PeerSource) -> bool {
    let addr = normalize_peer_addr(addr);
    if addr.port() == 0 {
        return false;
    }
    match addr {
        SocketAddr::V4(addr) => {
            let ip = *addr.ip();
            let octets = ip.octets();
            if ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_multicast()
                || ip.is_loopback()
                || octets[0] >= 240
                || octets[0] == 0
                || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 0b0100_0000)
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
            {
                return false;
            }
            let is_private = ip.is_private();
            if matches!(source, PeerSource::Lpd) {
                return true;
            }
            !is_private
        }
        SocketAddr::V6(addr) => {
            let ip = *addr.ip();
            let segments = ip.segments();
            if (segments[0] & 0xfe00) == 0xfc00 {
                return matches!(source, PeerSource::Lpd);
            }
            (segments[0] & 0xe000) == 0x2000
                && !(segments[0] == 0x2001 && (segments[1] & 0xfe00) == 0)
                && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
                && segments[0] != 0x2002
                && (segments[0] & 0xfff0) != 0x3ff0
                && !ip.is_multicast()
        }
    }
}

fn local_peer_addrs(listen_port: u16) -> HashSet<SocketAddr> {
    let mut addrs = HashSet::new();
    addrs.insert(SocketAddr::new(
        IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        listen_port,
    ));
    addrs.insert(SocketAddr::new(
        IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        listen_port,
    ));
    if let Some(ip) = outbound_local_ip("8.8.8.8:80", "0.0.0.0:0") {
        addrs.insert(SocketAddr::new(ip, listen_port));
    }
    if let Some(ip) = outbound_local_ip("[2001:4860:4860::8888]:80", "[::]:0") {
        addrs.insert(SocketAddr::new(ip, listen_port));
    }
    addrs
}

fn outbound_local_ip(remote_addr: &str, bind_addr: &str) -> Option<IpAddr> {
    let socket = UdpSocket::bind(bind_addr).ok()?;
    socket.connect(remote_addr).ok()?;
    Some(socket.local_addr().ok()?.ip())
}

fn is_self_peer_id(local_peer_id: &[u8; 20], remote_peer_id: &[u8; 20]) -> bool {
    local_peer_id == remote_peer_id
}

#[derive(Debug, Clone)]
struct FileSpan {
    path: String,
    #[cfg_attr(not(feature = "webseed"), allow(dead_code))]
    web_path: Vec<u8>,
    is_padding: bool,
    offset: u64,
    length: u64,
}

#[derive(Debug)]
struct V2HashStore {
    piece_length: u64,
    files: Vec<V2HashFile>,
}

#[derive(Debug)]
struct V2HashFile {
    pieces_root: [u8; 32],
    offset: u64,
    length: u64,
    leaf_width: usize,
    tree_height: u32,
    piece_layer: u32,
    /// Complete, power-of-two layers from the piece layer through the root.
    layers: Vec<Vec<[u8; 32]>>,
}

const HASH_REQUEST_WINDOW: Duration = Duration::from_secs(60);
const MAX_HASH_REQUESTS_PER_WINDOW: u32 = 32;
const MAX_HASH_DISK_BYTES_PER_WINDOW: u64 = 64 * 1024 * 1024;
const HASH_MESSAGE_FIXED_PAYLOAD_BYTES: usize = 1 + 32 + (4 * 4);

struct HashRequestBudget {
    window_started: Instant,
    requests: u32,
    disk_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HashBudgetDecision {
    ServeAfter(Duration),
    Reject,
}

impl HashRequestBudget {
    fn new() -> Self {
        Self::new_at(Instant::now())
    }

    fn new_at(now: Instant) -> Self {
        Self {
            window_started: now,
            requests: 0,
            disk_bytes: 0,
        }
    }

    fn reserve(&mut self, disk_bytes: u64, must_serve: bool) -> HashBudgetDecision {
        self.reserve_at(disk_bytes, must_serve, Instant::now())
    }

    fn reserve_at(
        &mut self,
        disk_bytes: u64,
        must_serve: bool,
        now: Instant,
    ) -> HashBudgetDecision {
        if now.saturating_duration_since(self.window_started) >= HASH_REQUEST_WINDOW {
            self.window_started = now;
            self.requests = 0;
            self.disk_bytes = 0;
        }

        let over_budget = self.requests >= MAX_HASH_REQUESTS_PER_WINDOW
            || self.disk_bytes.saturating_add(disk_bytes) > MAX_HASH_DISK_BYTES_PER_WINDOW;
        if over_budget {
            if !must_serve {
                return HashBudgetDecision::Reject;
            }

            // BEP 52 requires an immediately-following leaf-hash request for a
            // serviced chunk to receive a response. Preserve that guarantee by
            // deferring it into the next compute window instead of bypassing the
            // disk/request budget. Each peer loop is sequential, so reserving the
            // next window before waiting cannot race another request.
            let next_window = self
                .window_started
                .checked_add(HASH_REQUEST_WINDOW)
                .unwrap_or(now);
            let delay = next_window.saturating_duration_since(now);
            self.window_started = next_window.max(now);
            self.requests = 1;
            self.disk_bytes = disk_bytes;
            return HashBudgetDecision::ServeAfter(delay);
        }

        self.requests = self.requests.saturating_add(1);
        self.disk_bytes = self.disk_bytes.saturating_add(disk_bytes);
        HashBudgetDecision::ServeAfter(Duration::ZERO)
    }
}

impl V2HashStore {
    fn new(meta: &torrent::TorrentMeta) -> Result<Self, String> {
        if meta.meta_version == 1 {
            return Ok(Self {
                piece_length: meta.info.piece_length,
                files: Vec::new(),
            });
        }
        let block_length = piece::BLOCK_LEN as u64;
        if meta.info.piece_length < block_length || !meta.info.piece_length.is_power_of_two() {
            return Err("invalid v2 piece length".to_string());
        }
        let piece_layer = (meta.info.piece_length / block_length).trailing_zeros();
        let mut padding_hash = [0u8; 32];
        for _ in 0..piece_layer {
            padding_hash = v2_hash_parent(padding_hash, padding_hash);
        }

        let file_offsets = v2_file_offsets(meta)?;
        if file_offsets.len() != meta.info.file_tree.len() {
            return Err("invalid v2 file offsets".to_string());
        }

        let mut files = Vec::new();
        for (entry, offset) in meta.info.file_tree.iter().zip(file_offsets) {
            if entry.length == 0 {
                continue;
            }
            let pieces_root = entry
                .pieces_root
                .ok_or_else(|| "v2 file is missing its pieces root".to_string())?;
            let real_leaf_count = usize::try_from(entry.length.div_ceil(block_length))
                .map_err(|_| "v2 file is too large".to_string())?;
            let leaf_width = real_leaf_count
                .checked_next_power_of_two()
                .ok_or_else(|| "v2 hash tree is too large".to_string())?;
            let tree_height = leaf_width.trailing_zeros();

            let mut layers = Vec::new();
            if entry.length > meta.info.piece_length {
                let piece_hashes = meta
                    .piece_layers
                    .iter()
                    .find_map(|(root, hashes)| {
                        (root.as_slice() == pieces_root.as_slice()).then_some(hashes)
                    })
                    .ok_or_else(|| "v2 file is missing its piece layer".to_string())?;
                let width = piece_hashes
                    .len()
                    .checked_next_power_of_two()
                    .ok_or_else(|| "v2 piece layer is too large".to_string())?;
                let mut base = Vec::new();
                base.try_reserve_exact(width)
                    .map_err(|_| "v2 piece layer is too large".to_string())?;
                base.extend_from_slice(piece_hashes);
                base.resize(width, padding_hash);
                layers.push(base);
                while layers.last().is_some_and(|layer| layer.len() > 1) {
                    let previous = layers
                        .last()
                        .ok_or_else(|| "v2 piece layer disappeared".to_string())?;
                    let mut next = Vec::with_capacity(previous.len() / 2);
                    for pair in previous.chunks_exact(2) {
                        next.push(v2_hash_parent(pair[0], pair[1]));
                    }
                    layers.push(next);
                }
                if layers.last().and_then(|layer| layer.first()).copied() != Some(pieces_root) {
                    return Err("v2 piece layer does not match its root".to_string());
                }
            }
            files.push(V2HashFile {
                pieces_root,
                offset,
                length: entry.length,
                leaf_width,
                tree_height,
                piece_layer,
                layers,
            });
        }
        Ok(Self {
            piece_length: meta.info.piece_length,
            files,
        })
    }

    fn find_file(&self, pieces_root: [u8; 32]) -> Option<&V2HashFile> {
        self.files
            .iter()
            .find(|file| file.pieces_root == pieces_root)
    }

    fn static_hashes_for(&self, request: peer::HashRequest) -> Option<Vec<[u8; 32]>> {
        let file = self.find_file(request.pieces_root)?;
        if request.base_layer != file.piece_layer || file.layers.is_empty() {
            return None;
        }
        let base = file.layers.first()?;
        let start = usize::try_from(request.index).ok()?;
        let length = usize::try_from(request.length).ok()?;
        let end = start.checked_add(length)?;
        let proof_layers = usize::try_from(request.proof_layers).ok()?;
        if length < 2
            || !length.is_power_of_two()
            || start % length != 0
            || end > base.len()
            || proof_layers >= file.layers.len()
        {
            return None;
        }

        let mut hashes = base[start..end].to_vec();
        let mut range_start = start;
        let mut range_length = length;
        for layer_index in 0..proof_layers {
            let layer = file.layers.get(layer_index)?;
            if range_length == 1 {
                hashes.push(*layer.get(range_start ^ 1)?);
            }
            range_start /= 2;
            range_length = (range_length / 2).max(1);
        }
        Some(hashes)
    }

    fn estimated_disk_bytes(&self, request: peer::HashRequest) -> Option<u64> {
        let file = self.find_file(request.pieces_root)?;
        if request.base_layer != 0 {
            self.static_hashes_for(request)?;
            return Some(0);
        }
        let (start, length, proof_layers) = validate_v2_hash_range(file, request)?;
        if start >= usize::try_from(file.length.div_ceil(piece::BLOCK_LEN as u64)).ok()? {
            return None;
        }

        let known_layer = if file.layers.is_empty() {
            file.tree_height
        } else {
            file.piece_layer
        };
        let mut leaf_reads = u64::try_from(length).ok()?;
        let mut range_length = length;
        for layer in 0..proof_layers {
            if range_length == 1 && layer < known_layer {
                leaf_reads = leaf_reads.checked_add(1u64.checked_shl(layer)?)?;
            }
            range_length = (range_length / 2).max(1);
        }
        leaf_reads.checked_mul(piece::BLOCK_LEN as u64)
    }

    fn leaf_hashes_for(
        &self,
        request: peer::HashRequest,
        pieces: &piece::PieceManager,
        storage: &mut storage::Storage,
    ) -> Option<Vec<[u8; 32]>> {
        let file = self.find_file(request.pieces_root)?;
        if request.base_layer != 0 {
            return None;
        }
        let (start, length, proof_layers) = validate_v2_hash_range(file, request)?;
        let real_leaves = usize::try_from(file.length.div_ceil(piece::BLOCK_LEN as u64)).ok()?;
        if start >= real_leaves {
            return None;
        }

        let mut cache = HashMap::new();
        let mut hashes = Vec::with_capacity(length.saturating_add(proof_layers as usize));
        for index in start..start.checked_add(length)? {
            // Requested leaf hashes must be derived from verified local data,
            // even when a 16 KiB piece layer is present in metainfo.
            hashes.push(v2_leaf_hash(self, file, index, pieces, storage)?);
        }

        let mut range_start = start;
        let mut range_length = length;
        for layer in 0..proof_layers {
            if range_length == 1 {
                hashes.push(v2_node_hash(
                    self,
                    file,
                    layer,
                    range_start ^ 1,
                    pieces,
                    storage,
                    &mut cache,
                )?);
            }
            range_start /= 2;
            range_length = (range_length / 2).max(1);
        }
        Some(hashes)
    }

    fn request_covers_chunk(
        &self,
        request: peer::HashRequest,
        piece_index: u32,
        begin: u32,
        length: u32,
        pieces: &piece::PieceManager,
    ) -> bool {
        let Some(file) = self.find_file(request.pieces_root) else {
            return false;
        };
        if request.base_layer != 0 {
            return false;
        }
        let Some((start, hash_count, _)) = validate_v2_hash_range(file, request) else {
            return false;
        };
        let Some(piece_start) = pieces.piece_offset(piece_index) else {
            return false;
        };
        let Some(chunk_start) = piece_start.checked_add(begin as u64) else {
            return false;
        };
        let Some(chunk_end) = chunk_start.checked_add(length as u64) else {
            return false;
        };
        let Some(range_start) = (start as u64)
            .checked_mul(piece::BLOCK_LEN as u64)
            .and_then(|offset| file.offset.checked_add(offset))
        else {
            return false;
        };
        let Some(range_end) = (hash_count as u64)
            .checked_mul(piece::BLOCK_LEN as u64)
            .and_then(|bytes| range_start.checked_add(bytes))
            .map(|end| end.min(file.offset.saturating_add(file.length)))
        else {
            return false;
        };
        chunk_start >= range_start && chunk_end <= range_end
    }
}

fn v2_file_offsets(meta: &torrent::TorrentMeta) -> Result<Vec<u64>, String> {
    if meta.info.length.is_some() || meta.info.files.is_empty() {
        return meta
            .file_offsets()
            .ok_or_else(|| "v2 file layout overflow".to_string());
    }
    let offsets = meta
        .file_offsets()
        .ok_or_else(|| "v2 file layout overflow".to_string())?;
    Ok(meta
        .info
        .files
        .iter()
        .zip(offsets)
        .filter_map(|(file, offset)| (!file.attr.contains(&b'p')).then_some(offset))
        .collect())
}

fn validate_v2_hash_range(
    file: &V2HashFile,
    request: peer::HashRequest,
) -> Option<(usize, usize, u32)> {
    if request.base_layer > file.tree_height
        || request.proof_layers > file.tree_height.checked_sub(request.base_layer)?
    {
        return None;
    }
    let width = file.leaf_width.checked_shr(request.base_layer)?;
    let start = usize::try_from(request.index).ok()?;
    let length = usize::try_from(request.length).ok()?;
    let end = start.checked_add(length)?;
    if length < 2 || !length.is_power_of_two() || start % length != 0 || end > width {
        return None;
    }
    Some((start, length, request.proof_layers))
}

fn v2_known_node(file: &V2HashFile, layer: u32, index: usize) -> Option<[u8; 32]> {
    if layer == file.tree_height && index == 0 {
        return Some(file.pieces_root);
    }
    let relative = layer.checked_sub(file.piece_layer)?;
    file.layers
        .get(relative as usize)
        .and_then(|nodes| nodes.get(index))
        .copied()
}

fn v2_leaf_hash(
    store: &V2HashStore,
    file: &V2HashFile,
    index: usize,
    pieces: &piece::PieceManager,
    storage: &mut storage::Storage,
) -> Option<[u8; 32]> {
    let within_file = u64::try_from(index)
        .ok()?
        .checked_mul(piece::BLOCK_LEN as u64)?;
    if within_file >= file.length {
        return Some([0u8; 32]);
    }
    let read_len = usize::try_from(
        file.length
            .checked_sub(within_file)?
            .min(piece::BLOCK_LEN as u64),
    )
    .ok()?;
    let absolute = file.offset.checked_add(within_file)?;
    let piece_index = u32::try_from(absolute / store.piece_length).ok()?;
    if !pieces.is_piece_complete(piece_index) {
        return None;
    }
    let piece_start = pieces.piece_offset(piece_index)?;
    let piece_end = piece_start.checked_add(pieces.piece_length(piece_index)? as u64)?;
    if absolute < piece_start || absolute.checked_add(read_len as u64)? > piece_end {
        return None;
    }
    let mut data = vec![0u8; read_len];
    storage.read_at(absolute, &mut data).ok()?;
    Some(sha256::sha256(&data))
}

fn v2_node_hash(
    store: &V2HashStore,
    file: &V2HashFile,
    layer: u32,
    index: usize,
    pieces: &piece::PieceManager,
    storage: &mut storage::Storage,
    cache: &mut HashMap<(u32, usize), [u8; 32]>,
) -> Option<[u8; 32]> {
    if let Some(hash) = cache.get(&(layer, index)).copied() {
        return Some(hash);
    }
    if let Some(hash) = v2_known_node(file, layer, index) {
        cache.insert((layer, index), hash);
        return Some(hash);
    }
    let hash = if layer == 0 {
        v2_leaf_hash(store, file, index, pieces, storage)?
    } else {
        let child = index.checked_mul(2)?;
        let left = v2_node_hash(store, file, layer - 1, child, pieces, storage, cache)?;
        let right = v2_node_hash(store, file, layer - 1, child + 1, pieces, storage, cache)?;
        v2_hash_parent(left, right)
    };
    cache.insert((layer, index), hash);
    Some(hash)
}

fn v2_hash_parent(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(&left);
    data[32..].copy_from_slice(&right);
    sha256::sha256(&data)
}

struct V2HashResponseResources<'a> {
    store: &'a V2HashStore,
    pieces: &'a Arc<Mutex<piece::PieceManager>>,
    storage: &'a Arc<Mutex<storage::Storage>>,
    limits: &'a TransferLimits,
    stop_flag: &'a AtomicBool,
}

fn respond_v2_hash_request<W: Write>(
    writer: &mut W,
    resources: V2HashResponseResources<'_>,
    budget: &mut HashRequestBudget,
    must_serve: bool,
    request: peer::HashRequest,
) -> Result<(), String> {
    let estimated_bytes = resources.store.estimated_disk_bytes(request).unwrap_or(0);
    let hashes = match budget.reserve(estimated_bytes, must_serve) {
        HashBudgetDecision::Reject => None,
        HashBudgetDecision::ServeAfter(delay) => {
            sleep_with_shutdown_or_stop(delay, resources.stop_flag);
            if torrent_stop_requested(resources.stop_flag) {
                return Err("hash response cancelled".to_string());
            }
            if request.base_layer == 0 {
                let pieces = lock_or_recover(resources.pieces);
                let mut storage = lock_or_recover(resources.storage);
                resources
                    .store
                    .leaf_hashes_for(request, &pieces, &mut storage)
            } else {
                resources.store.static_hashes_for(request)
            }
        }
    };
    let response = match hashes {
        Some(hashes) => peer::Message::Hashes { request, hashes },
        None => peer::Message::HashReject(request),
    };
    let response_bytes = hash_response_payload_bytes(&response);
    resources.limits.global_up.throttle(response_bytes);
    resources.limits.torrent_up.throttle(response_bytes);
    if torrent_stop_requested(resources.stop_flag) {
        return Err("hash response cancelled".to_string());
    }
    peer::write_message(writer, &response).map_err(|err| format!("hash response failed: {err}"))
}

fn hash_response_payload_bytes(message: &peer::Message) -> usize {
    match message {
        peer::Message::Hashes { hashes, .. } => {
            HASH_MESSAGE_FIXED_PAYLOAD_BYTES.saturating_add(hashes.len().saturating_mul(32))
        }
        peer::Message::HashReject(_) => HASH_MESSAGE_FIXED_PAYLOAD_BYTES,
        _ => 0,
    }
}

fn build_file_spans(meta: &torrent::TorrentMeta) -> Result<Vec<FileSpan>, String> {
    let name = String::from_utf8_lossy(&meta.info.name).into_owned();
    if let Some(length) = meta.info.length {
        return Ok(vec![FileSpan {
            path: name,
            web_path: meta.info.name.clone(),
            is_padding: false,
            offset: 0,
            length,
        }]);
    }

    let mut spans = if !meta.info.files.is_empty() {
        Vec::with_capacity(meta.info.files.len())
    } else {
        Vec::with_capacity(meta.info.file_tree.len())
    };
    let offsets = meta
        .file_offsets()
        .ok_or_else(|| "file layout overflow".to_string())?;
    if !meta.info.files.is_empty() {
        for (file, offset) in meta.info.files.iter().zip(offsets) {
            let mut path = name.clone();
            let mut web_path = meta.info.name.clone();
            for segment in &file.path {
                path.push('/');
                path.push_str(&String::from_utf8_lossy(segment));
                web_path.push(b'/');
                web_path.extend_from_slice(segment);
            }
            spans.push(FileSpan {
                path,
                web_path,
                is_padding: file.attr.contains(&b'p'),
                offset,
                length: file.length,
            });
        }
    } else {
        let single_v2_file = !is_getright_multi_file(meta);
        for (file, offset) in meta.info.file_tree.iter().zip(offsets) {
            let mut path = name.clone();
            let mut web_path = if single_v2_file {
                Vec::new()
            } else {
                meta.info.name.clone()
            };
            for segment in &file.path {
                path.push('/');
                path.push_str(&String::from_utf8_lossy(segment));
                if !web_path.is_empty() {
                    web_path.push(b'/');
                }
                web_path.extend_from_slice(segment);
            }
            spans.push(FileSpan {
                path,
                web_path,
                is_padding: false,
                offset,
                length: file.length,
            });
        }
    }
    Ok(spans)
}

/// BEP 19's URL rules are based on the metainfo layout, not the number of
/// physical spans. A valid multi-file torrent may contain only one file.
fn is_getright_multi_file(meta: &torrent::TorrentMeta) -> bool {
    if meta.info.length.is_some() {
        return false;
    }
    if !meta.info.files.is_empty() {
        return true;
    }
    !(meta.info.file_tree.len() == 1 && meta.info.file_tree[0].path.len() == 1)
}

fn build_ui_files(
    spans: &[FileSpan],
    pieces: &piece::PieceManager,
    file_priorities: &[u8],
) -> Vec<ui::UiFile> {
    let mut files: Vec<ui::UiFile> = spans
        .iter()
        .enumerate()
        .map(|(idx, span)| ui::UiFile {
            path: span.path.clone(),
            length: span.length,
            completed: 0,
            priority: file_priorities.get(idx).copied().unwrap_or(0),
        })
        .collect();

    let piece_count = pieces.piece_count();
    for index in 0..piece_count {
        let index = index as u32;
        if !pieces.is_piece_complete(index) {
            continue;
        }
        let piece_len = match pieces.piece_length(index) {
            Some(len) => len as u64,
            None => continue,
        };
        let Some(piece_start) = pieces.piece_offset(index) else {
            continue;
        };
        apply_piece_to_files(&mut files, spans, piece_start, piece_len);
    }

    files
}

fn apply_ui_file_renames(files: &mut [ui::UiFile], renames: &HashMap<usize, String>) {
    for (index, name) in renames {
        if let Some(file) = files.get_mut(*index) {
            file.path = renamed_display_path(&file.path, name);
        }
    }
}

fn renamed_display_path(path: &str, name: &str) -> String {
    match path.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{name}"),
        None => name.to_string(),
    }
}

fn apply_file_priorities(
    pieces: &mut piece::PieceManager,
    spans: &[FileSpan],
    file_priorities: &[u8],
    base_piece_length: u64,
) -> Result<(), String> {
    let priorities = compute_piece_priorities(
        spans,
        file_priorities,
        base_piece_length,
        pieces.piece_count(),
    );
    pieces
        .set_piece_priorities(&priorities)
        .map_err(|err| err.to_string())
}

fn compute_piece_priorities(
    spans: &[FileSpan],
    file_priorities: &[u8],
    base_piece_length: u64,
    piece_count: usize,
) -> Vec<u8> {
    if piece_count == 0 {
        return Vec::new();
    }
    let mut priorities = vec![piece::PRIORITY_SKIP; piece_count];
    if base_piece_length == 0 {
        return priorities;
    }
    for (idx, span) in spans.iter().enumerate() {
        if span.is_padding || span.length == 0 {
            continue;
        }
        let priority = file_priorities
            .get(idx)
            .copied()
            .unwrap_or(piece::PRIORITY_NORMAL);
        if priority == piece::PRIORITY_SKIP {
            continue;
        }
        let start_piece = span.offset / base_piece_length;
        let end_offset = span.offset.saturating_add(span.length).saturating_sub(1);
        let end_piece = end_offset / base_piece_length;
        for piece_index in start_piece..=end_piece {
            if let Some(slot) = priorities.get_mut(piece_index as usize) {
                if priority > *slot {
                    *slot = priority;
                }
            }
        }
    }
    priorities
}

#[cfg(feature = "webseed")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum WebSeed {
    /// BEP 19/GetRight-style static file or directory URL.
    GetRight(String),
    /// BEP 17/Hoffman-style script endpoint.
    Hoffman(String),
}

#[cfg(feature = "webseed")]
fn collect_web_seeds(meta: &torrent::TorrentMeta) -> Vec<WebSeed> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for url in &meta.url_list {
        if let Ok(url_str) = std::str::from_utf8(url) {
            if seen.insert((0u8, url_str.to_string())) {
                out.push(WebSeed::GetRight(url_str.to_string()));
            }
        }
    }
    for url in &meta.httpseeds {
        if let Ok(url_str) = std::str::from_utf8(url) {
            if seen.insert((1u8, url_str.to_string())) {
                out.push(WebSeed::Hoffman(url_str.to_string()));
            }
        }
    }
    out
}

#[cfg(feature = "webseed")]
fn webseed_memory_budget_bytes(piece_len: u32) -> Option<usize> {
    if piece_len == 0 || piece_len as u64 > torrent::MAX_PIECE_LENGTH {
        return None;
    }
    let max_body = (piece_len as usize).checked_add(WEBSEED_HTTP_BODY_SLACK)?;
    http::response_memory_budget(max_body)
}

#[cfg(feature = "webseed")]
fn try_reserve_webseed_memory(
    pieces: &Mutex<piece::PieceManager>,
    index: u32,
    piece_len: u32,
    budgets: &piece::PieceBufferBudgets,
) -> Option<piece::PieceBufferReservation> {
    let reservation =
        webseed_memory_budget_bytes(piece_len).and_then(|bytes| budgets.try_reserve(bytes));
    if reservation.is_none() {
        lock_or_recover(pieces).release_piece(WEBSEED_RESERVATION_ID, index);
    }
    reservation
}

#[cfg(feature = "webseed")]
#[allow(clippy::too_many_arguments)]
fn start_webseed_worker(
    web_seeds: Vec<WebSeed>,
    pieces: Arc<Mutex<piece::PieceManager>>,
    storage: Arc<Mutex<storage::Storage>>,
    completed_log: Arc<Mutex<Vec<u32>>>,
    file_spans: Arc<Vec<FileSpan>>,
    getright_multi_file: bool,
    _base_piece_length: u64,
    info_hash: [u8; 20],
    limits: TransferLimits,
    downloaded: Arc<AtomicU64>,
    stop_flag: Arc<AtomicBool>,
    piece_buffer_budgets: piece::PieceBufferBudgets,
    ui_state: Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
) -> Result<Option<thread::JoinHandle<()>>, String> {
    if web_seeds.is_empty() {
        return Ok(None);
    }
    thread::Builder::new()
        .name(format!("webseed-{torrent_id}"))
        .spawn(move || loop {
            if torrent_stop_requested(&stop_flag) {
                break;
            }
            let (index, piece_start, piece_len, expected) = {
                let mut p = lock_or_recover(&pieces);
                let available = vec![u8::MAX; p.bitfield_len()];
                let index =
                    match p.reserve_piece_for_peer(WEBSEED_RESERVATION_ID, &available, false) {
                        Some(index) => index,
                        None => break,
                    };
                let length = match p.piece_length(index) {
                    Some(length) => length,
                    None => {
                        p.release_piece(WEBSEED_RESERVATION_ID, index);
                        break;
                    }
                };
                let offset = match p.piece_offset(index) {
                    Some(offset) => offset,
                    None => {
                        p.release_piece(WEBSEED_RESERVATION_ID, index);
                        break;
                    }
                };
                let expected = match p.piece_hash(index) {
                    Some(hash) => hash.clone(),
                    None => {
                        p.release_piece(WEBSEED_RESERVATION_ID, index);
                        break;
                    }
                };
                (index, offset, length, expected)
            };

            let Some(memory_reservation) =
                try_reserve_webseed_memory(&pieces, index, piece_len, &piece_buffer_budgets)
            else {
                // Shared memory pressure is routine backpressure. Give peer
                // and other torrent workers a chance to release buffers.
                sleep_with_shutdown_or_stop(PEER_QUEUE_POLL_INTERVAL, &stop_flag);
                continue;
            };
            let data = match fetch_piece_from_web_seeds(
                &web_seeds,
                &file_spans,
                getright_multi_file,
                info_hash,
                index,
                piece_start,
                piece_len,
            ) {
                Ok(data) => data,
                Err(_) => {
                    lock_or_recover(&pieces).release_piece(WEBSEED_RESERVATION_ID, index);
                    sleep_with_shutdown_or_stop(Duration::from_secs(1), &stop_flag);
                    continue;
                }
            };
            if !verify_piece_hash(&data, &expected) {
                lock_or_recover(&pieces).release_piece(WEBSEED_RESERVATION_ID, index);
                sleep_with_shutdown_or_stop(Duration::from_secs(1), &stop_flag);
                continue;
            }
            let write_ok = {
                let mut s = lock_or_recover(&storage);
                s.write_at(piece_start, &data).is_ok()
            };
            if !write_ok {
                lock_or_recover(&pieces).release_piece(WEBSEED_RESERVATION_ID, index);
                continue;
            }
            drop(data);
            drop(memory_reservation);
            let was_new = {
                let mut p = lock_or_recover(&pieces);
                p.mark_piece_complete(index).unwrap_or(false)
            };
            if !was_new {
                continue;
            }
            SESSION_DOWNLOADED_BYTES.fetch_add(piece_len as u64, Ordering::SeqCst);
            downloaded.fetch_add(piece_len as u64, Ordering::SeqCst);
            limits.global_down.throttle(piece_len as usize);
            limits.torrent_down.throttle(piece_len as usize);
            if let Ok(mut log) = completed_log.lock() {
                log.push(index);
            }
            let piece_len_u64 = piece_len as u64;
            let completed_pieces = {
                let p = lock_or_recover(&pieces);
                p.completed_pieces()
            };
            update_ui(&ui_state, |state| {
                apply_piece_completion_ui(
                    state,
                    torrent_id,
                    completed_pieces,
                    &file_spans,
                    piece_start,
                    piece_len_u64,
                    true,
                );
            });
        })
        .map(Some)
        .map_err(|err| format!("web seed worker could not start: {err}"))
}

#[cfg(not(feature = "webseed"))]
fn collect_web_seeds(_meta: &torrent::TorrentMeta) -> Vec<String> {
    Vec::new()
}

#[cfg(not(feature = "webseed"))]
#[allow(clippy::too_many_arguments)]
fn start_webseed_worker(
    _web_seeds: Vec<String>,
    _pieces: Arc<Mutex<piece::PieceManager>>,
    _storage: Arc<Mutex<storage::Storage>>,
    _completed_log: Arc<Mutex<Vec<u32>>>,
    _file_spans: Arc<Vec<FileSpan>>,
    _getright_multi_file: bool,
    _base_piece_length: u64,
    _info_hash: [u8; 20],
    _limits: TransferLimits,
    _downloaded: Arc<AtomicU64>,
    _stop_flag: Arc<AtomicBool>,
    _piece_buffer_budgets: piece::PieceBufferBudgets,
    _ui_state: Option<Arc<Mutex<ui::UiState>>>,
    _torrent_id: u64,
) -> Result<Option<thread::JoinHandle<()>>, String> {
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn start_resume_worker(
    resume_path: PathBuf,
    info_hash: [u8; 20],
    base_piece_length: u64,
    pieces: Arc<Mutex<piece::PieceManager>>,
    storage: Arc<Mutex<storage::Storage>>,
    file_priorities: Arc<Mutex<Vec<u8>>>,
    file_spans: Arc<Vec<FileSpan>>,
    downloaded: Arc<AtomicU64>,
    uploaded: Arc<AtomicU64>,
    peer_queue: Arc<Mutex<PeerQueue>>,
    stop_flag: Arc<AtomicBool>,
    file_renames: Arc<Mutex<HashMap<usize, String>>>,
    save_requested: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>, String> {
    thread::Builder::new()
        .name(format!("resume-{}", hex(&info_hash[..4])))
        .spawn(move || {
            let mut last_save = Instant::now() - RESUME_SAVE_INTERVAL;
            let mut last_complete: Option<bool> = None;
            loop {
                let stopping = torrent_stop_requested(&stop_flag);
                let complete = {
                    let p = lock_or_recover(&pieces);
                    p.is_complete()
                };
                let requested = save_requested.swap(false, Ordering::AcqRel);
                let completion_changed = last_complete.is_some_and(|last| last != complete);
                if last_save.elapsed() >= RESUME_SAVE_INTERVAL
                    || completion_changed
                    || requested
                    || stopping
                {
                    let priorities = lock_or_recover(&file_priorities).clone();
                    let downloaded = downloaded.load(Ordering::SeqCst);
                    let uploaded = uploaded.load(Ordering::SeqCst);
                    let peers = lock_or_recover(&peer_queue).sample(256);
                    let snapshot = {
                        // Nested runtime locks always follow pieces -> storage -> renames.
                        // This gives the saved bitfield and rename map the exact bytes and
                        // filesystem paths that were made durable by the flush.
                        let p = lock_or_recover(&pieces);
                        let mut s = lock_or_recover(&storage);
                        match s.flush() {
                            Ok(()) => {
                                let renames = lock_or_recover(&file_renames)
                                    .iter()
                                    .map(|(index, name)| (*index, name.clone()))
                                    .collect::<Vec<_>>();
                                Ok((
                                    build_bitfield(&p),
                                    collect_storage_file_stats(&s, &file_spans),
                                    renames,
                                    p.is_complete(),
                                ))
                            }
                            Err(err) => Err(err.to_string()),
                        }
                    };
                    match snapshot {
                        Ok((bitfield, files, renames, snapshot_complete)) => {
                            match save_resume_data(
                                &resume_path,
                                info_hash,
                                base_piece_length,
                                bitfield,
                                &priorities,
                                files,
                                downloaded,
                                uploaded,
                                peers,
                                &renames,
                            ) {
                                Ok(()) => {
                                    last_save = Instant::now();
                                    last_complete = Some(snapshot_complete);
                                }
                                Err(err) => {
                                    log_warn!("resume save failed: {err}");
                                    if !stopping {
                                        save_requested.store(true, Ordering::Release);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            log_warn!("resume flush failed: {err}");
                            if !stopping {
                                save_requested.store(true, Ordering::Release);
                            }
                        }
                    }
                }
                if stopping {
                    break;
                }
                sleep_with_shutdown_or_stop(Duration::from_secs(1), &stop_flag);
            }
        })
        .map_err(|err| format!("resume worker could not start: {err}"))
}

#[cfg(feature = "webseed")]
fn fetch_piece_from_web_seeds(
    web_seeds: &[WebSeed],
    file_spans: &[FileSpan],
    getright_multi_file: bool,
    info_hash: [u8; 20],
    index: u32,
    piece_start: u64,
    piece_len: u32,
) -> Result<Vec<u8>, String> {
    let piece_end = piece_start.saturating_add(piece_len as u64);
    let piece_http_limit = (piece_len as usize)
        .checked_add(WEBSEED_HTTP_BODY_SLACK)
        .ok_or_else(|| "web seed piece length overflow".to_string())?;
    for web_seed in web_seeds {
        if let WebSeed::Hoffman(base) = web_seed {
            let url = build_httpseed_url(base, info_hash, index);
            if let Ok(data) = http::get_public(&url, piece_http_limit) {
                if data.len() == piece_len as usize {
                    return Ok(data);
                }
            }
            continue;
        }
        let WebSeed::GetRight(base) = web_seed else {
            unreachable!("all web seed variants handled")
        };
        let mut out = Vec::with_capacity(piece_len as usize);
        let mut ok = true;
        for span in file_spans {
            let file_start = span.offset;
            let file_end = span.offset.saturating_add(span.length);
            let overlap_start = piece_start.max(file_start);
            let overlap_end = piece_end.min(file_end);
            if overlap_end <= overlap_start {
                continue;
            }
            if span.is_padding {
                out.resize(
                    out.len()
                        .saturating_add((overlap_end - overlap_start) as usize),
                    0,
                );
                continue;
            }
            let range_start = overlap_start.saturating_sub(file_start);
            let range_end = overlap_end.saturating_sub(file_start).saturating_sub(1);
            let expected_len = (overlap_end - overlap_start) as usize;
            let range_http_limit = expected_len
                .checked_add(WEBSEED_HTTP_BODY_SLACK)
                .ok_or_else(|| "web seed range length overflow".to_string())?;
            let url = build_webseed_url(base, &span.web_path, getright_multi_file);
            let data = match http::get_range_public(&url, range_start, range_end, range_http_limit)
            {
                Ok(data) => data,
                Err(_) => {
                    ok = false;
                    break;
                }
            };
            if data.len() != expected_len {
                ok = false;
                break;
            }
            out.extend_from_slice(&data);
        }
        if ok && out.len() == piece_len as usize {
            return Ok(out);
        }
    }
    Err("web seed fetch failed".to_string())
}

#[cfg(feature = "webseed")]
fn build_webseed_url(base: &str, path: &[u8], multi: bool) -> String {
    if !multi && !base.ends_with('/') {
        return base.to_string();
    }
    let mut out = base.trim_end_matches('/').to_string();
    out.push('/');
    out.push_str(&percent_encode_path(path));
    out
}

#[cfg(feature = "webseed")]
fn percent_encode_path(path: &[u8]) -> String {
    let mut out = String::with_capacity(path.len());
    for &b in path {
        if is_unreserved(b) || b == b'/' {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

#[cfg(feature = "webseed")]
fn build_httpseed_url(base: &str, info_hash: [u8; 20], piece: u32) -> String {
    let mut out = base.to_string();
    out.push(if base.contains('?') { '&' } else { '?' });
    out.push_str("info_hash=");
    for byte in info_hash {
        out.push('%');
        out.push_str(&format!("{byte:02X}"));
    }
    out.push_str("&piece=");
    out.push_str(&piece.to_string());
    out
}

#[cfg(feature = "webseed")]
fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

fn apply_piece_to_files(
    files: &mut [ui::UiFile],
    spans: &[FileSpan],
    piece_start: u64,
    piece_len: u64,
) {
    let piece_end = piece_start.saturating_add(piece_len);
    for (idx, span) in spans.iter().enumerate() {
        let file_start = span.offset;
        let file_end = span.offset.saturating_add(span.length);
        let overlap_start = piece_start.max(file_start);
        let overlap_end = piece_end.min(file_end);
        if overlap_end <= overlap_start {
            continue;
        }
        let delta = overlap_end - overlap_start;
        if let Some(file) = files.get_mut(idx) {
            file.completed = (file.completed + delta).min(file.length);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn peer_worker_loop(
    info_hash: [u8; 20],
    hybrid_v2_info_hash: Option<[u8; 20]>,
    peer_id: [u8; 20],
    torrent_id: u64,
    peer_tags: &Arc<AtomicU64>,
    pieces: &Arc<Mutex<piece::PieceManager>>,
    storage: &Arc<Mutex<storage::Storage>>,
    completed_log: &Arc<Mutex<Vec<u32>>>,
    peer_queue: &Arc<Mutex<PeerQueue>>,
    allow_pex: bool,
    active_peers: &Arc<AtomicUsize>,
    interested_peers: &Arc<AtomicUsize>,
    upload_requests_served: &Arc<AtomicU64>,
    file_spans: &Arc<Vec<FileSpan>>,
    base_piece_length: u64,
    v2_hashes: &Arc<V2HashStore>,
    connect_cfg: ConnectionConfig,
    limits: TransferLimits,
    downloaded: &Arc<AtomicU64>,
    uploaded: &Arc<AtomicU64>,
    upload_manager: &Arc<UploadManager>,
    peer_cancellations: &PeerCancellationRegistry,
    paused_flag: &Arc<AtomicBool>,
    stop_flag: &Arc<AtomicBool>,
    peer_slots: Arc<PeerSlots>,
    per_torrent_slots: Arc<PeerSlots>,
    piece_buffer_budgets: piece::PieceBufferBudgets,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
) {
    loop {
        if torrent_stop_requested(stop_flag) {
            break;
        }

        let addr = {
            let mut queue = lock_or_recover(peer_queue);
            queue.pop()
        };

        let addr = match addr {
            Some(addr) => addr,
            None => {
                sleep_with_shutdown_or_stop(PEER_QUEUE_POLL_INTERVAL, stop_flag);
                continue;
            }
        };

        if !per_torrent_slots.acquire(stop_flag) {
            let mut queue = lock_or_recover(peer_queue);
            queue.finish(addr);
            break;
        }
        if !peer_slots.acquire(stop_flag) {
            per_torrent_slots.release();
            let mut queue = lock_or_recover(peer_queue);
            queue.finish(addr);
            break;
        }
        log_info!("connecting to peer {addr}...");

        let peer_tag = peer_tags.fetch_add(1, Ordering::SeqCst);
        let result = download_from_peer_concurrent(
            addr,
            info_hash,
            hybrid_v2_info_hash,
            peer_id,
            torrent_id,
            peer_tag,
            pieces,
            storage,
            completed_log,
            peer_queue,
            allow_pex,
            file_spans,
            base_piece_length,
            v2_hashes,
            &connect_cfg,
            &limits,
            downloaded,
            uploaded,
            active_peers,
            interested_peers,
            upload_requests_served,
            upload_manager,
            peer_cancellations,
            paused_flag,
            stop_flag,
            &piece_buffer_budgets,
            ui_state,
        );

        peer_slots.release();
        per_torrent_slots.release();

        {
            let mut queue = lock_or_recover(peer_queue);
            record_peer_result(&mut queue, addr, &result);
        }

        if let Err(err) = &result {
            log_warn!("peer {addr} error: {err}");
            update_ui(ui_state, |state| {
                state.last_error = err.clone();
                update_torrent_entry(state, torrent_id, |torrent| {
                    torrent.last_error = err.clone();
                });
            });
        }
    }
}

struct ResumeStats {
    completed_bytes: u64,
}

#[derive(Debug)]
struct ResumeData {
    info_hash: [u8; 20],
    piece_length: u64,
    bitfield: Vec<u8>,
    file_priorities: Vec<u8>,
    files: Vec<ResumeFileStat>,
    downloaded: u64,
    uploaded: u64,
    peers: Vec<SocketAddr>,
    file_renames: Vec<(usize, String)>,
}

#[derive(Debug)]
struct ResumeFileStat {
    length: u64,
    mtime: u64,
}

fn resume_from_storage(
    pieces: &mut piece::PieceManager,
    storage: &mut storage::Storage,
    base_piece_length: u64,
    file_spans: &[FileSpan],
    resume: Option<&ResumeData>,
) -> Result<ResumeStats, String> {
    let piece_count = pieces.piece_count();
    if piece_count == 0 {
        return Ok(ResumeStats { completed_bytes: 0 });
    }
    if let Some(resume) = resume {
        if resume.piece_length == base_piece_length
            && resume.bitfield.len() == pieces.bitfield_len()
            && resume.files.len() == file_spans.len()
        {
            let max_len = base_piece_length
                .try_into()
                .map_err(|_| "piece length too large".to_string())?;
            let mut buffer = vec![0u8; max_len];
            for index in 0..piece_count {
                if !bitfield_has(&resume.bitfield, index) {
                    continue;
                }
                // Length and modification time are only cache hints: they can
                // survive bit rot or deliberate metadata restoration. Never
                // advertise resumed data until its piece hash is verified.
                let _ = verify_piece(storage, pieces, index as u32, &mut buffer)?;
            }
            return Ok(ResumeStats {
                completed_bytes: pieces.completed_bytes(),
            });
        }
    }

    full_recheck(pieces, storage, base_piece_length, None)
}

fn full_recheck(
    pieces: &mut piece::PieceManager,
    storage: &mut storage::Storage,
    base_piece_length: u64,
    stop_flag: Option<&AtomicBool>,
) -> Result<ResumeStats, String> {
    let piece_count = pieces.piece_count();
    let max_len = base_piece_length
        .try_into()
        .map_err(|_| "piece length too large".to_string())?;
    let mut buffer = vec![0u8; max_len];
    for index in 0..piece_count {
        if stop_flag.is_some_and(torrent_stop_requested) {
            break;
        }
        verify_piece(storage, pieces, index as u32, &mut buffer)?;
    }
    Ok(ResumeStats {
        completed_bytes: pieces.completed_bytes(),
    })
}

fn verify_piece(
    storage: &mut storage::Storage,
    pieces: &mut piece::PieceManager,
    index: u32,
    buffer: &mut [u8],
) -> Result<bool, String> {
    let length = pieces
        .piece_length(index)
        .ok_or_else(|| "missing piece length".to_string())? as usize;
    let offset = pieces
        .piece_offset(index)
        .ok_or_else(|| "missing piece offset".to_string())?;
    let target = &mut buffer[..length];
    if storage.read_at(offset, target).is_err() {
        return Ok(false);
    }
    let expected = pieces
        .piece_hash(index)
        .ok_or_else(|| "missing piece hash".to_string())?;
    if verify_piece_hash(target, expected) {
        pieces
            .mark_piece_complete(index)
            .map_err(|err| format!("resume mark failed: {err}"))?;
        return Ok(true);
    }
    Ok(false)
}

fn resume_path(download_dir: &Path, info_hash: [u8; 20]) -> PathBuf {
    let mut dir = download_dir.join(".rustorrent");
    dir.push(format!("{}.resume", hex(&info_hash)));
    dir
}

fn relocate_resume_state(source: &Path, destination: &Path) -> Result<(), String> {
    if source == destination {
        return Ok(());
    }
    let data = match read_file_limited(source, MAX_RESUME_STATE_BYTES, true) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("read resume state: {err}")),
    };
    parse_resume_data(&data).map_err(|err| format!("validate resume state: {err}"))?;
    write_atomic_file(destination, &data, "resume relocation", false, true)?;
    remove_file_bound(source).map_err(|err| format!("remove old resume state: {err}"))?;
    let _ = remove_file_bound(&sidecar_path(source, ".bak"));
    Ok(())
}

fn session_path(download_dir: &Path) -> PathBuf {
    download_dir.join(".rustorrent").join("session.benc")
}

#[cfg(unix)]
fn push_session_path(
    dict: &mut Vec<(Vec<u8>, Value)>,
    key: &[u8],
    _wide_key: &[u8],
    path: &Path,
) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    dict.push((
        key.to_vec(),
        Value::Bytes(path.as_os_str().as_bytes().to_vec()),
    ));
    Ok(())
}

#[cfg(windows)]
fn push_session_path(
    dict: &mut Vec<(Vec<u8>, Value)>,
    _key: &[u8],
    wide_key: &[u8],
    path: &Path,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let mut bytes = Vec::new();
    for unit in path.as_os_str().encode_wide() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    dict.push((wide_key.to_vec(), Value::Bytes(bytes)));
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn push_session_path(
    dict: &mut Vec<(Vec<u8>, Value)>,
    key: &[u8],
    _wide_key: &[u8],
    path: &Path,
) -> Result<(), String> {
    let value = path
        .to_str()
        .ok_or_else(|| "session path is not valid UTF-8".to_string())?;
    dict.push((key.to_vec(), Value::Bytes(value.as_bytes().to_vec())));
    Ok(())
}

#[cfg(unix)]
fn decode_session_path(
    items: &[(Vec<u8>, Value)],
    key: &[u8],
    _wide_key: &[u8],
) -> Result<Option<PathBuf>, String> {
    use std::os::unix::ffi::OsStringExt;
    match dict_get(items, key) {
        Some(Value::Bytes(bytes)) if !bytes.is_empty() => Ok(Some(PathBuf::from(
            std::ffi::OsString::from_vec(bytes.clone()),
        ))),
        None => Ok(None),
        _ => Err("invalid session path".to_string()),
    }
}

#[cfg(windows)]
fn decode_session_path(
    items: &[(Vec<u8>, Value)],
    key: &[u8],
    wide_key: &[u8],
) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::OsStringExt;
    if let Some(value) = dict_get(items, wide_key) {
        let Value::Bytes(bytes) = value else {
            return Err("invalid wide session path".to_string());
        };
        if bytes.is_empty() || bytes.len() % 2 != 0 {
            return Err("invalid wide session path".to_string());
        }
        let wide = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        return Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(&wide))));
    }
    match dict_get(items, key) {
        Some(Value::Bytes(bytes)) if !bytes.is_empty() => String::from_utf8(bytes.clone())
            .map(PathBuf::from)
            .map(Some)
            .map_err(|_| "invalid legacy session path".to_string()),
        None => Ok(None),
        _ => Err("invalid legacy session path".to_string()),
    }
}

#[cfg(not(any(unix, windows)))]
fn decode_session_path(
    items: &[(Vec<u8>, Value)],
    key: &[u8],
    _wide_key: &[u8],
) -> Result<Option<PathBuf>, String> {
    match dict_get(items, key) {
        Some(Value::Bytes(bytes)) if !bytes.is_empty() => String::from_utf8(bytes.clone())
            .map(PathBuf::from)
            .map(Some)
            .map_err(|_| "invalid session path".to_string()),
        None => Ok(None),
        _ => Err("invalid session path".to_string()),
    }
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut out = path.as_os_str().to_owned();
    out.push(suffix);
    PathBuf::from(out)
}

fn write_atomic_file(
    path: &Path,
    data: &[u8],
    label: &str,
    keep_backup: bool,
    private: bool,
) -> Result<(), String> {
    #[cfg(any(unix, windows))]
    if state_dir::is_state_file_path(path) {
        let mode = if private { 0o600 } else { 0o644 };
        return state_dir::write_atomic(path, data, keep_backup, mode, MAX_ATOMIC_BACKUP_BYTES)
            .map_err(|err| format!("{label} state write failed: {err}"));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if parent != Path::new(".") {
        if state_dir::is_state_file_path(path) {
            let download_dir = parent
                .parent()
                .ok_or_else(|| format!("{label} state directory has no parent"))?;
            ensure_private_state_directory(download_dir)?;
        } else {
            fs::create_dir_all(parent).map_err(|err| format!("{label} dir failed: {err}"))?;
        }
    }
    if keep_backup && path.exists() {
        let metadata =
            fs::symlink_metadata(path).map_err(|err| format!("{label} metadata failed: {err}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!("{label} target is not a regular file"));
        }
        let backup_path = sidecar_path(path, ".bak");
        let existing = read_file_limited(path, MAX_ATOMIC_BACKUP_BYTES, true)
            .map_err(|err| format!("{label} backup read failed: {err}"))?;
        write_atomic_file(
            &backup_path,
            &existing,
            &format!("{label} backup"),
            false,
            private,
        )?;
    }

    let sequence = ATOMIC_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = sidecar_path(path, &format!(".tmp.{}.{}", std::process::id(), sequence));
    let write_result = (|| -> Result<(), String> {
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(if private { 0o600 } else { 0o644 });
        }
        #[cfg(not(unix))]
        let _ = private;
        let mut tmp = options
            .open(&tmp_path)
            .map_err(|err| format!("{label} temporary file failed: {err}"))?;
        tmp.write_all(data)
            .map_err(|err| format!("{label} write failed: {err}"))?;
        tmp.sync_all()
            .map_err(|err| format!("{label} sync failed: {err}"))?;
        drop(tmp);
        fs::rename(&tmp_path, path).map_err(|err| format!("{label} rename failed: {err}"))?;
        finish_atomic_publish(sync_parent_directory(parent, label), label)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    write_result?;
    Ok(())
}

fn finish_atomic_publish(sync_result: Result<(), String>, label: &str) -> Result<(), String> {
    // Once rename succeeds the new bytes are the visible logical state. A
    // directory-fsync failure weakens crash durability, but reporting the save
    // as uncommitted would make callers roll back memory or physical actions
    // while the new journal is already on disk.
    if let Err(err) = sync_result {
        log_warn!("{label} published but parent directory sync failed: {err}");
    }
    Ok(())
}

fn sync_parent_directory(parent: &Path, label: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|err| format!("{label} directory sync failed: {err}"))?;
    }
    #[cfg(not(unix))]
    let _ = (parent, label);
    Ok(())
}

fn parse_session_entries(
    data: &[u8],
    root: &Path,
) -> Result<HashMap<[u8; 20], SessionEntry>, String> {
    let value = bencode::parse(data).map_err(|err| err.to_string())?;
    let list = match value {
        Value::List(items) => items,
        _ => return Err("invalid format".to_string()),
    };
    if list.len() > MAX_SESSION_ENTRIES {
        return Err(format!(
            "session contains more than {MAX_SESSION_ENTRIES} entries"
        ));
    }
    let mut entries: HashMap<[u8; 20], SessionEntry> = HashMap::new();
    for item in list {
        let Value::Dict(items) = item else {
            continue;
        };
        let info_hash = match dict_get(&items, b"info_hash") {
            Some(Value::Bytes(bytes)) if bytes.len() == 20 => {
                let mut out = [0u8; 20];
                out.copy_from_slice(bytes);
                out
            }
            _ => continue,
        };
        let torrent_bytes = match dict_get(&items, b"torrent") {
            Some(Value::Bytes(bytes)) if !bytes.is_empty() && bytes.len() <= MAX_TORRENT_BYTES => {
                bytes.clone()
            }
            Some(Value::Bytes(bytes)) if bytes.len() > MAX_TORRENT_BYTES => {
                return Err("stored torrent exceeds the metainfo size limit".to_string());
            }
            _ => continue,
        };
        let parsed_torrent = torrent::parse_torrent(&torrent_bytes)
            .map_err(|err| format!("invalid stored torrent: {err}"))?;
        if parsed_torrent.info_hash != info_hash {
            return Err("stored torrent info hash mismatch".to_string());
        }
        let name = match dict_get(&items, b"name") {
            Some(Value::Bytes(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
            _ => String::new(),
        };
        let download_dir = decode_session_path(&items, b"download_dir", b"download_dir_wide")?
            .unwrap_or_else(|| root.to_path_buf());
        let preallocate = dict_get_int(&items, b"preallocate").unwrap_or(0) != 0;
        let label = match dict_get(&items, b"label") {
            Some(Value::Bytes(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
            _ => String::new(),
        };
        let completion_state = match dict_get(&items, b"completion_state") {
            Some(Value::Bytes(bytes)) => CompletionState::from_bytes(bytes)
                .ok_or_else(|| "invalid session completion state".to_string())?,
            None => CompletionState::None,
            _ => return Err("invalid session completion state".to_string()),
        };
        let completion_move_dir =
            decode_session_path(&items, b"completion_move_dir", b"completion_move_dir_wide")?;
        if completion_state != CompletionState::Pending && completion_move_dir.is_some() {
            return Err("completion move directory requires pending state".to_string());
        }
        let pending_delete = match dict_get_int(&items, b"pending_delete") {
            None | Some(0) => false,
            Some(1) => true,
            _ => return Err("invalid session pending delete state".to_string()),
        };
        let mut file_renames = match dict_get(&items, b"file_renames") {
            Some(Value::List(renames)) => renames
                .iter()
                .map(|rename| {
                    let Value::Dict(values) = rename else {
                        return Err("invalid session file rename".to_string());
                    };
                    let index = dict_get_int(values, b"index")
                        .filter(|index| *index >= 0)
                        .ok_or_else(|| "invalid session file rename index".to_string())?
                        as usize;
                    let target = match dict_get(values, b"name") {
                        Some(Value::Bytes(bytes)) => String::from_utf8(bytes.clone())
                            .map_err(|_| "invalid session file rename name".to_string())?,
                        _ => return Err("invalid session file rename name".to_string()),
                    };
                    if !valid_renamed_file_name(&target) {
                        return Err("invalid session file rename name".to_string());
                    }
                    Ok((index, target))
                })
                .collect::<Result<Vec<_>, String>>()?,
            None => Vec::new(),
            _ => return Err("invalid session file renames".to_string()),
        };
        file_renames.sort_unstable_by_key(|(index, _)| *index);
        if file_renames.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err("duplicate session file rename index".to_string());
        }
        let pending_file_rename = match dict_get(&items, b"pending_file_rename") {
            Some(Value::Dict(values)) => {
                let index = dict_get_int(values, b"index")
                    .filter(|index| *index >= 0)
                    .ok_or_else(|| "invalid pending file rename index".to_string())?
                    as usize;
                let target = match dict_get(values, b"name") {
                    Some(Value::Bytes(bytes)) => String::from_utf8(bytes.clone())
                        .map_err(|_| "invalid pending file rename name".to_string())?,
                    _ => return Err("invalid pending file rename name".to_string()),
                };
                if !valid_renamed_file_name(&target) {
                    return Err("invalid pending file rename name".to_string());
                }
                Some(PendingFileRename { index, target })
            }
            None => None,
            _ => return Err("invalid pending file rename".to_string()),
        };
        if pending_delete && pending_file_rename.is_some() {
            return Err("session cannot delete while a file rename is pending".to_string());
        }
        if entries.contains_key(&info_hash) {
            return Err("duplicate session info hash".to_string());
        }
        entries.insert(
            info_hash,
            SessionEntry {
                info_hash,
                name,
                torrent_bytes,
                download_dir,
                preallocate,
                label,
                completion_state,
                completion_move_dir,
                pending_delete,
                file_renames,
                pending_file_rename,
            },
        );
    }
    Ok(entries)
}

fn load_session_entries_with_recovery(
    path: &Path,
    root: &Path,
) -> Result<HashMap<[u8; 20], SessionEntry>, String> {
    let (primary_error, primary_missing) =
        match read_file_limited(path, MAX_SESSION_STATE_BYTES, true) {
            Ok(data) => match parse_session_entries(&data, root) {
                Ok(entries) => return Ok(entries),
                Err(err) => (err, false),
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                ("session file is missing".to_string(), true)
            }
            Err(err) => (format!("session read failed: {err}"), false),
        };

    let backup_path = sidecar_path(path, ".bak");
    let backup_data = match read_file_limited(&backup_path, MAX_SESSION_STATE_BYTES, true) {
        Ok(data) => data,
        Err(err) if err.kind() == io::ErrorKind::NotFound && primary_missing => {
            return Ok(HashMap::new());
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(format!("session load failed: {primary_error}"));
        }
        Err(err) => {
            return Err(format!(
                "session load failed: {primary_error}; backup read failed: {err}"
            ));
        }
    };
    let recovered = parse_session_entries(&backup_data, root).map_err(|backup_err| {
        format!("session load failed: {primary_error}; backup invalid: {backup_err}")
    })?;
    if let Err(err) = write_atomic_file(path, &backup_data, "session restore", false, true) {
        log_warn!("session backup loaded but primary restore failed: {err}");
    }
    log_warn!("session load recovered from backup");
    Ok(recovered)
}

fn save_session(path: &Path, entries: &HashMap<[u8; 20], SessionEntry>) -> Result<(), String> {
    if entries.len() > MAX_SESSION_ENTRIES {
        return Err(format!(
            "session contains more than {MAX_SESSION_ENTRIES} entries"
        ));
    }
    let mut list = Vec::with_capacity(entries.len());
    for entry in entries.values() {
        if entry.torrent_bytes.is_empty() || entry.torrent_bytes.len() > MAX_TORRENT_BYTES {
            return Err("stored torrent exceeds the metainfo size limit".to_string());
        }
        let parsed_torrent = torrent::parse_torrent(&entry.torrent_bytes)
            .map_err(|err| format!("invalid stored torrent: {err}"))?;
        if parsed_torrent.info_hash != entry.info_hash {
            return Err("stored torrent info hash mismatch".to_string());
        }
        let mut dict = Vec::new();
        dict.push((
            b"info_hash".to_vec(),
            Value::Bytes(entry.info_hash.to_vec()),
        ));
        dict.push((
            b"name".to_vec(),
            Value::Bytes(entry.name.as_bytes().to_vec()),
        ));
        dict.push((
            b"torrent".to_vec(),
            Value::Bytes(entry.torrent_bytes.clone()),
        ));
        push_session_path(
            &mut dict,
            b"download_dir",
            b"download_dir_wide",
            &entry.download_dir,
        )?;
        dict.push((
            b"preallocate".to_vec(),
            Value::Int(if entry.preallocate { 1 } else { 0 }),
        ));
        dict.push((
            b"completion_state".to_vec(),
            Value::Bytes(entry.completion_state.as_bytes().to_vec()),
        ));
        if let Some(move_dir) = entry.completion_move_dir.as_ref() {
            push_session_path(
                &mut dict,
                b"completion_move_dir",
                b"completion_move_dir_wide",
                move_dir,
            )?;
        }
        if entry.pending_delete {
            dict.push((b"pending_delete".to_vec(), Value::Int(1)));
        }
        if !entry.file_renames.is_empty() {
            let renames = entry
                .file_renames
                .iter()
                .map(|(index, name)| {
                    Value::Dict(vec![
                        (b"index".to_vec(), Value::Int(*index as i64)),
                        (b"name".to_vec(), Value::Bytes(name.as_bytes().to_vec())),
                    ])
                })
                .collect();
            dict.push((b"file_renames".to_vec(), Value::List(renames)));
        }
        if let Some(pending) = entry.pending_file_rename.as_ref() {
            dict.push((
                b"pending_file_rename".to_vec(),
                Value::Dict(vec![
                    (b"index".to_vec(), Value::Int(pending.index as i64)),
                    (
                        b"name".to_vec(),
                        Value::Bytes(pending.target.as_bytes().to_vec()),
                    ),
                ]),
            ));
        }
        if !entry.label.is_empty() {
            dict.push((
                b"label".to_vec(),
                Value::Bytes(entry.label.as_bytes().to_vec()),
            ));
        }
        list.push(Value::Dict(dict));
    }
    let value = Value::List(list);
    bencode::validate_structure(&value)
        .map_err(|err| format!("session state structure exceeds parser limits: {err}"))?;
    let data = bencode::encode(&value);
    if data.len() > MAX_SESSION_STATE_BYTES {
        return Err("session state exceeds the size limit".to_string());
    }
    write_atomic_file(path, &data, "session", true, true)
}

fn load_resume_data(path: &Path) -> Result<ResumeData, String> {
    let data = read_file_limited(path, MAX_RESUME_STATE_BYTES, true)
        .map_err(|err| format!("resume read failed: {err}"))?;
    parse_resume_data(&data)
}

fn parse_resume_data(data: &[u8]) -> Result<ResumeData, String> {
    let value = bencode::parse(data).map_err(|err| err.to_string())?;
    let dict = match value {
        Value::Dict(items) => items,
        _ => return Err("resume format invalid".to_string()),
    };
    let info_hash = match dict_get(&dict, b"info_hash") {
        Some(Value::Bytes(bytes)) if bytes.len() == 20 => {
            let mut out = [0u8; 20];
            out.copy_from_slice(bytes);
            out
        }
        _ => return Err("resume info hash missing".to_string()),
    };
    let piece_length = dict_get_int(&dict, b"piece_length")
        .filter(|value| *value > 0 && (*value as u64) <= torrent::MAX_PIECE_LENGTH)
        .ok_or_else(|| "resume piece length is invalid".to_string())? as u64;
    let bitfield = match dict_get(&dict, b"pieces") {
        Some(Value::Bytes(bytes)) => bytes.clone(),
        _ => return Err("resume pieces missing".to_string()),
    };
    let file_priorities = match dict_get(&dict, b"file_priority") {
        Some(Value::List(items)) => items
            .iter()
            .map(|item| match item {
                Value::Int(value) if (0..=piece::PRIORITY_HIGH as i64).contains(value) => {
                    Ok(*value as u8)
                }
                _ => Err("resume file priority is invalid".to_string()),
            })
            .collect::<Result<Vec<_>, String>>()?,
        None => Vec::new(),
        Some(_) => return Err("resume file priorities are invalid".to_string()),
    };
    let files = match dict_get(&dict, b"files") {
        Some(Value::List(items)) => items
            .iter()
            .map(|item| {
                let Value::Dict(values) = item else {
                    return Err("resume file stat is invalid".to_string());
                };
                let length = dict_get_int(values, b"length")
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| "resume file length is invalid".to_string())?
                    as u64;
                let mtime = match dict_get(values, b"mtime") {
                    Some(Value::Int(value)) if *value >= 0 => *value as u64,
                    None => 0,
                    _ => return Err("resume file mtime is invalid".to_string()),
                };
                Ok(ResumeFileStat { length, mtime })
            })
            .collect::<Result<Vec<_>, String>>()?,
        None => Vec::new(),
        Some(_) => return Err("resume file stats are invalid".to_string()),
    };
    let downloaded = parse_resume_counter(&dict, b"downloaded", "downloaded")?;
    let uploaded = parse_resume_counter(&dict, b"uploaded", "uploaded")?;
    let peers = match dict_get(&dict, b"peers") {
        Some(Value::List(items)) => items
            .iter()
            .map(|item| match item {
                Value::Bytes(bytes) => std::str::from_utf8(bytes)
                    .map_err(|_| "resume peer address is invalid".to_string())?
                    .parse::<SocketAddr>()
                    .map_err(|_| "resume peer address is invalid".to_string()),
                _ => Err("resume peer address is invalid".to_string()),
            })
            .collect::<Result<Vec<_>, String>>()?,
        None => Vec::new(),
        Some(_) => return Err("resume peer list is invalid".to_string()),
    };
    let mut file_renames = match dict_get(&dict, b"file_renames") {
        Some(Value::List(items)) => items
            .iter()
            .map(|item| {
                let Value::Dict(values) = item else {
                    return Err("resume file rename is invalid".to_string());
                };
                let index = dict_get_int(values, b"index")
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| "resume file rename index is invalid".to_string())?;
                let name = match dict_get(values, b"name") {
                    Some(Value::Bytes(bytes)) => String::from_utf8(bytes.clone())
                        .map_err(|_| "resume file rename name is invalid".to_string())?,
                    _ => return Err("resume file rename name is invalid".to_string()),
                };
                if !valid_renamed_file_name(&name) {
                    return Err("resume file rename name is invalid".to_string());
                }
                Ok((index, name))
            })
            .collect::<Result<Vec<_>, String>>()?,
        None => Vec::new(),
        Some(_) => return Err("resume file renames are invalid".to_string()),
    };
    file_renames.sort_unstable_by_key(|(index, _)| *index);
    if file_renames.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("duplicate resume file rename index".to_string());
    }
    Ok(ResumeData {
        info_hash,
        piece_length,
        bitfield,
        file_priorities,
        files,
        downloaded,
        uploaded,
        peers,
        file_renames,
    })
}

fn parse_resume_counter(dict: &[(Vec<u8>, Value)], key: &[u8], label: &str) -> Result<u64, String> {
    match dict_get(dict, key) {
        Some(Value::Int(value)) if *value >= 0 => Ok(*value as u64),
        None => Ok(0),
        _ => Err(format!("resume {label} counter is invalid")),
    }
}

fn load_resume_data_with_recovery(path: &Path) -> Option<ResumeData> {
    if path.exists() {
        match load_resume_data(path) {
            Ok(resume) => return Some(resume),
            Err(err) => {
                log_warn!("resume load failed: {err}");
            }
        }
    }

    let backup_path = sidecar_path(path, ".bak");
    let backup_data = match read_file_limited(&backup_path, MAX_RESUME_STATE_BYTES, true) {
        Ok(data) => data,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return None,
        Err(err) => {
            log_warn!("resume backup read failed: {err}");
            return None;
        }
    };
    let resume = match parse_resume_data(&backup_data) {
        Ok(resume) => resume,
        Err(err) => {
            log_warn!("resume backup invalid: {err}");
            return None;
        }
    };
    let _ = write_atomic_file(path, &backup_data, "resume restore", false, true);
    log_warn!("resume load recovered from backup");
    Some(resume)
}

#[allow(clippy::too_many_arguments)]
fn save_resume_data(
    path: &Path,
    info_hash: [u8; 20],
    base_piece_length: u64,
    bitfield: Vec<u8>,
    file_priorities: &[u8],
    files: Vec<ResumeFileStat>,
    downloaded: u64,
    uploaded: u64,
    peers: Vec<SocketAddr>,
    file_renames: &[(usize, String)],
) -> Result<(), String> {
    if base_piece_length == 0 || base_piece_length > torrent::MAX_PIECE_LENGTH {
        return Err("resume piece length is invalid".to_string());
    }
    if file_priorities
        .iter()
        .any(|priority| *priority > piece::PRIORITY_HIGH)
    {
        return Err("resume file priority is invalid".to_string());
    }
    if files.iter().any(|stat| stat.length > i64::MAX as u64) {
        return Err("resume file length is invalid".to_string());
    }
    let mut rename_indices = HashSet::with_capacity(file_renames.len());
    if file_renames.iter().any(|(index, name)| {
        !rename_indices.insert(*index)
            || i64::try_from(*index).is_err()
            || !valid_renamed_file_name(name)
    }) {
        return Err("resume file rename is invalid".to_string());
    }

    let mut dict = Vec::new();
    dict.push((b"info_hash".to_vec(), Value::Bytes(info_hash.to_vec())));
    dict.push((
        b"piece_length".to_vec(),
        Value::Int(base_piece_length as i64),
    ));
    let downloaded_i64 = downloaded.min(i64::MAX as u64) as i64;
    let uploaded_i64 = uploaded.min(i64::MAX as u64) as i64;
    dict.push((b"downloaded".to_vec(), Value::Int(downloaded_i64)));
    dict.push((b"uploaded".to_vec(), Value::Int(uploaded_i64)));
    dict.push((b"pieces".to_vec(), Value::Bytes(bitfield)));
    let priorities = file_priorities
        .iter()
        .map(|value| Value::Int(*value as i64))
        .collect::<Vec<_>>();
    dict.push((b"file_priority".to_vec(), Value::List(priorities)));
    let files_list = files
        .into_iter()
        .map(|stat| {
            Value::Dict(vec![
                (b"length".to_vec(), Value::Int(stat.length as i64)),
                (
                    b"mtime".to_vec(),
                    Value::Int(stat.mtime.min(i64::MAX as u64) as i64),
                ),
            ])
        })
        .collect();
    dict.push((b"files".to_vec(), Value::List(files_list)));
    let peers_list = peers
        .into_iter()
        .map(|addr| Value::Bytes(addr.to_string().into_bytes()))
        .collect();
    dict.push((b"peers".to_vec(), Value::List(peers_list)));
    if !file_renames.is_empty() {
        let renames_list = file_renames
            .iter()
            .map(|(idx, name)| {
                Value::Dict(vec![
                    (b"index".to_vec(), Value::Int(*idx as i64)),
                    (b"name".to_vec(), Value::Bytes(name.as_bytes().to_vec())),
                ])
            })
            .collect();
        dict.push((b"file_renames".to_vec(), Value::List(renames_list)));
    }
    let value = Value::Dict(dict);
    bencode::validate_structure(&value)
        .map_err(|err| format!("resume state structure exceeds parser limits: {err}"))?;
    let data = bencode::encode(&value);
    if data.len() > MAX_RESUME_STATE_BYTES {
        return Err("resume state exceeds the size limit".to_string());
    }
    write_atomic_file(path, &data, "resume", true, true)
}

fn collect_storage_file_stats(
    storage: &storage::Storage,
    spans: &[FileSpan],
) -> Vec<ResumeFileStat> {
    spans
        .iter()
        .enumerate()
        .map(|(index, span)| {
            storage
                .file_path(index)
                .and_then(file_stat_path)
                .unwrap_or(ResumeFileStat {
                    length: span.length,
                    mtime: 0,
                })
        })
        .collect()
}

fn file_stat_path(path: &Path) -> Option<ResumeFileStat> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Some(ResumeFileStat {
        length: meta.len(),
        mtime,
    })
}

#[derive(Clone)]
struct Args {
    torrent_path: Option<String>,
    magnet: Option<String>,
    download_dir: std::path::PathBuf,
    preallocate: bool,
    ui: bool,
    ui_addr: String,
    peer_profile: PeerProfile,
    retry_interval: u64,
    numwant: u32,
    metadata_peer_limit: usize,
    port: u16,
    enable_utp: bool,
    encryption: EncryptionMode,
    blocklist_path: Option<PathBuf>,
    max_peers_global: usize,
    max_peers_torrent: usize,
    max_active_torrents: usize,
    download_rate: u64,
    upload_rate: u64,
    torrent_download_rate: u64,
    torrent_upload_rate: u64,
    write_cache_bytes: usize,
    sequential: bool,
    move_completed: Option<PathBuf>,
    log_path: Option<PathBuf>,
    daemon: bool,
    pid_file: Option<PathBuf>,
    seed_ratio: f64,
    max_seed_time: u64,
    on_complete: Option<PathBuf>,
    watch_dirs: Vec<PathBuf>,
    super_seed: bool,
    tui: bool,
    proxy: Option<proxy::ProxyConfig>,
    geoip_db: Option<PathBuf>,
    rss_feeds: Vec<String>,
    rss_rules: Vec<(String, String)>,
    rss_interval: u64,
    throttle_groups: Vec<(String, u64, u64)>,
    ratio_groups: Vec<(String, f64, String)>,
    schedules: Vec<(u64, String)>,
}

#[derive(Default)]
struct ConfigOverrides {
    download_dir: Option<PathBuf>,
    preallocate: Option<bool>,
    ui: Option<bool>,
    ui_addr: Option<String>,
    peer_profile: Option<PeerProfile>,
    retry_interval: Option<u64>,
    numwant: Option<u32>,
    port: Option<u16>,
    enable_utp: Option<bool>,
    encryption: Option<EncryptionMode>,
    blocklist_path: Option<PathBuf>,
    max_peers_global: Option<usize>,
    max_peers_torrent: Option<usize>,
    max_active_torrents: Option<usize>,
    download_rate: Option<u64>,
    upload_rate: Option<u64>,
    torrent_download_rate: Option<u64>,
    torrent_upload_rate: Option<u64>,
    write_cache_bytes: Option<usize>,
    geoip_db: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let args_list = env::args().skip(1).collect::<Vec<_>>();
    let mut config_path = env::var("RUSTORRENT_CONFIG").ok();
    if let Some(idx) = args_list.iter().position(|arg| arg == "--config") {
        let value = args_list
            .get(idx + 1)
            .ok_or_else(|| "missing value for --config".to_string())?;
        config_path = Some(value.clone());
    }
    let config_overrides = match config_path {
        Some(path) => Some(load_config_overrides(Path::new(&path))?),
        None => None,
    };
    let env_overrides = load_env_overrides();

    let mut torrent_path: Option<String> = None;
    let mut magnet: Option<String> = None;
    let mut download_dir = env::current_dir().map_err(|err| format!("cwd error: {err}"))?;
    let mut preallocate = false;
    let mut ui = false;
    let mut ui_addr = "127.0.0.1:8080".to_string();
    let mut peer_profile = PeerProfile::Balanced;
    let peer_tuning = peer_profile.tuning();
    let mut retry_interval = 60u64;
    let mut numwant = peer_tuning.numwant;
    let mut metadata_peer_limit = peer_tuning.metadata_peer_limit;
    let mut port = 6881u16;
    let mut enable_utp = true;
    let mut encryption = EncryptionMode::Prefer;
    let mut blocklist_path: Option<PathBuf> = None;
    let mut max_peers_global = peer_tuning.max_peers_global;
    let mut max_peers_torrent = peer_tuning.max_peers_torrent;
    let mut max_active_torrents = 4usize;
    let mut download_rate = 0u64;
    let mut upload_rate = 0u64;
    let mut torrent_download_rate = 0u64;
    let mut torrent_upload_rate = 0u64;
    let mut write_cache_bytes = 0usize;
    let mut sequential = false;
    let mut move_completed: Option<PathBuf> = None;
    let mut log_path: Option<PathBuf> = None;
    let mut daemon = false;
    let mut pid_file: Option<PathBuf> = None;
    let mut seed_ratio = 0.0f64;
    let mut max_seed_time = 0u64;
    let mut on_complete: Option<PathBuf> = None;
    let mut watch_dirs: Vec<PathBuf> = Vec::new();
    let mut super_seed = false;
    let mut tui = false;
    let mut proxy_config: Option<proxy::ProxyConfig> = None;
    let mut geoip_path: Option<PathBuf> = None;
    let mut rss_feeds: Vec<String> = Vec::new();
    let mut rss_rules: Vec<(String, String)> = Vec::new();
    let mut rss_interval = 900u64;
    let mut throttle_groups: Vec<(String, u64, u64)> = Vec::new();
    let mut ratio_groups: Vec<(String, f64, String)> = Vec::new();
    let mut schedules: Vec<(u64, String)> = Vec::new();
    let mut cli_numwant_set = false;
    let mut cli_max_peers_set = false;
    let mut cli_max_peers_torrent_set = false;

    if let Some(cfg) = config_overrides.as_ref() {
        apply_overrides(
            cfg,
            &mut download_dir,
            &mut preallocate,
            &mut ui,
            &mut ui_addr,
            &mut peer_profile,
            &mut retry_interval,
            &mut numwant,
            &mut metadata_peer_limit,
            &mut port,
            &mut enable_utp,
            &mut encryption,
            &mut blocklist_path,
            &mut max_peers_global,
            &mut max_peers_torrent,
            &mut max_active_torrents,
            &mut download_rate,
            &mut upload_rate,
            &mut torrent_download_rate,
            &mut torrent_upload_rate,
            &mut write_cache_bytes,
            &mut geoip_path,
        );
    }
    apply_overrides(
        &env_overrides,
        &mut download_dir,
        &mut preallocate,
        &mut ui,
        &mut ui_addr,
        &mut peer_profile,
        &mut retry_interval,
        &mut numwant,
        &mut metadata_peer_limit,
        &mut port,
        &mut enable_utp,
        &mut encryption,
        &mut blocklist_path,
        &mut max_peers_global,
        &mut max_peers_torrent,
        &mut max_active_torrents,
        &mut download_rate,
        &mut upload_rate,
        &mut torrent_download_rate,
        &mut torrent_upload_rate,
        &mut write_cache_bytes,
        &mut geoip_path,
    );

    let mut idx = 0usize;
    while idx < args_list.len() {
        let arg = &args_list[idx];
        if arg == "--config" {
            idx += 2;
            continue;
        }
        if arg == "--download-dir" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --download-dir".to_string())?;
            download_dir = value.into();
            idx += 2;
            continue;
        }
        if arg == "--magnet" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --magnet".to_string())?;
            magnet = Some(value.clone());
            idx += 2;
            continue;
        }
        if arg == "--preallocate" {
            preallocate = true;
            idx += 1;
            continue;
        }
        if arg == "--ui" {
            ui = true;
            // Accept optional port number: --ui 8080 sets ui_addr to 127.0.0.1:8080
            if let Some(next) = args_list.get(idx + 1) {
                if !next.starts_with("--") {
                    if let Ok(port) = next.parse::<u16>() {
                        ui_addr = format!("127.0.0.1:{port}");
                        idx += 2;
                        continue;
                    }
                }
            }
            idx += 1;
            continue;
        }
        if arg == "--ui-addr" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --ui-addr".to_string())?;
            ui_addr = value.clone();
            idx += 2;
            continue;
        }
        if arg == "--peer-profile" || arg == "--network-profile" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --peer-profile".to_string())?;
            peer_profile = parse_peer_profile(value)?;
            let tuning = peer_profile.tuning();
            if !cli_numwant_set {
                numwant = tuning.numwant;
            }
            if !cli_max_peers_set {
                max_peers_global = tuning.max_peers_global;
            }
            if !cli_max_peers_torrent_set {
                max_peers_torrent = tuning.max_peers_torrent;
            }
            metadata_peer_limit = tuning.metadata_peer_limit;
            idx += 2;
            continue;
        }
        if arg == "--retry-interval" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --retry-interval".to_string())?;
            retry_interval = value
                .parse::<u64>()
                .map_err(|_| "invalid value for --retry-interval".to_string())?;
            if retry_interval == 0 {
                return Err("retry interval must be > 0".to_string());
            }
            idx += 2;
            continue;
        }
        if arg == "--numwant" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --numwant".to_string())?;
            numwant = value
                .parse::<u32>()
                .map_err(|_| "invalid value for --numwant".to_string())?;
            cli_numwant_set = true;
            idx += 2;
            continue;
        }
        if arg == "--port" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --port".to_string())?;
            port = value
                .parse::<u16>()
                .map_err(|_| "invalid value for --port".to_string())?;
            idx += 2;
            continue;
        }
        if arg == "--encryption" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --encryption".to_string())?;
            encryption = parse_encryption_mode(value)?;
            idx += 2;
            continue;
        }
        if arg == "--no-encryption" {
            encryption = EncryptionMode::Disable;
            idx += 1;
            continue;
        }
        if arg == "--utp" {
            enable_utp = true;
            idx += 1;
            continue;
        }
        if arg == "--no-utp" {
            enable_utp = false;
            idx += 1;
            continue;
        }
        if arg == "--blocklist" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --blocklist".to_string())?;
            blocklist_path = Some(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if arg == "--max-active" || arg == "--max-active-torrents" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --max-active".to_string())?;
            max_active_torrents = value
                .parse::<usize>()
                .map_err(|_| "invalid value for --max-active".to_string())?;
            idx += 2;
            continue;
        }
        if arg == "--max-peers" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --max-peers".to_string())?;
            max_peers_global = value
                .parse::<usize>()
                .map_err(|_| "invalid value for --max-peers".to_string())?;
            cli_max_peers_set = true;
            idx += 2;
            continue;
        }
        if arg == "--max-peers-torrent" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --max-peers-torrent".to_string())?;
            max_peers_torrent = value
                .parse::<usize>()
                .map_err(|_| "invalid value for --max-peers-torrent".to_string())?;
            cli_max_peers_torrent_set = true;
            idx += 2;
            continue;
        }
        if arg == "--download-rate" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --download-rate".to_string())?;
            download_rate = parse_rate(value)?;
            idx += 2;
            continue;
        }
        if arg == "--upload-rate" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --upload-rate".to_string())?;
            upload_rate = parse_rate(value)?;
            idx += 2;
            continue;
        }
        if arg == "--torrent-download-rate" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --torrent-download-rate".to_string())?;
            torrent_download_rate = parse_rate(value)?;
            idx += 2;
            continue;
        }
        if arg == "--torrent-upload-rate" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --torrent-upload-rate".to_string())?;
            torrent_upload_rate = parse_rate(value)?;
            idx += 2;
            continue;
        }
        if arg == "--write-cache" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --write-cache".to_string())?;
            write_cache_bytes = parse_size(value)?;
            idx += 2;
            continue;
        }
        if arg == "--sequential" {
            sequential = true;
            idx += 1;
            continue;
        }
        if arg == "--log" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --log".to_string())?;
            log_path = Some(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if arg == "--daemon" {
            daemon = true;
            idx += 1;
            continue;
        }
        if arg == "--pid-file" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --pid-file".to_string())?;
            pid_file = Some(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if arg == "--seed-ratio" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --seed-ratio".to_string())?;
            seed_ratio = value
                .parse::<f64>()
                .map_err(|_| "invalid value for --seed-ratio".to_string())?;
            if !seed_ratio.is_finite() || seed_ratio < 0.0 {
                return Err("seed ratio must be a finite value >= 0".to_string());
            }
            idx += 2;
            continue;
        }
        if arg == "--max-seed-time" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --max-seed-time".to_string())?;
            max_seed_time = value
                .parse::<u64>()
                .map_err(|_| "invalid value for --max-seed-time".to_string())?;
            if max_seed_time.checked_mul(60).is_none() {
                return Err("max seed time is too large".to_string());
            }
            idx += 2;
            continue;
        }
        if arg == "--on-complete" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --on-complete".to_string())?;
            on_complete = Some(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if arg == "--super-seed" {
            super_seed = true;
            idx += 1;
            continue;
        }
        if arg == "--proxy" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --proxy".to_string())?;
            proxy_config = Some(proxy::ProxyConfig::parse(value)?);
            idx += 2;
            continue;
        }
        if arg == "--geoip-db" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --geoip-db".to_string())?;
            geoip_path = Some(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if arg == "--rss" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --rss".to_string())?;
            if rss_feeds.len() >= rss::MAX_RSS_FEEDS {
                return Err("too many RSS feeds".to_string());
            }
            if value.len() > rss::MAX_RSS_TEXT_BYTES
                || !valid_tracker_url(value)
                || !(value.starts_with("http://") || value.starts_with("https://"))
            {
                return Err("RSS feed must be a valid HTTP or HTTPS URL".to_string());
            }
            rss_feeds.push(value.clone());
            idx += 2;
            continue;
        }
        if arg == "--rss-rule" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --rss-rule".to_string())?;
            let (feed_url, pattern) = parse_rss_rule_arg(value)?;
            if rss_rules.len() >= rss::MAX_RSS_RULES {
                return Err("too many RSS rules".to_string());
            }
            if pattern.len() > rss::MAX_RSS_PATTERN_BYTES
                || feed_url.len() > rss::MAX_RSS_TEXT_BYTES
                || (!feed_url.is_empty()
                    && (!valid_tracker_url(feed_url)
                        || !(feed_url.starts_with("http://") || feed_url.starts_with("https://"))))
            {
                return Err("invalid RSS rule URL or pattern".to_string());
            }
            rss_rules.push((feed_url.to_string(), pattern.to_string()));
            idx += 2;
            continue;
        }
        if arg == "--rss-interval" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --rss-interval".to_string())?;
            rss_interval = value
                .parse::<u64>()
                .map_err(|_| "invalid value for --rss-interval".to_string())?;
            if rss_interval == 0 {
                return Err("rss interval must be > 0".to_string());
            }
            idx += 2;
            continue;
        }
        if arg == "--tui" {
            tui = true;
            idx += 1;
            continue;
        }
        if arg == "--throttle" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --throttle".to_string())?;
            let parts: Vec<&str> = value.splitn(3, ':').collect();
            if parts.len() != 3 {
                return Err("--throttle format: name:down_kbps:up_kbps".to_string());
            }
            let down = parts[1]
                .parse::<u64>()
                .map_err(|_| "invalid throttle down rate".to_string())?
                .checked_mul(1024)
                .ok_or_else(|| "throttle down rate is too large".to_string())?;
            let up = parts[2]
                .parse::<u64>()
                .map_err(|_| "invalid throttle up rate".to_string())?
                .checked_mul(1024)
                .ok_or_else(|| "throttle up rate is too large".to_string())?;
            if parts[0].trim().is_empty() {
                return Err("throttle group name must not be empty".to_string());
            }
            throttle_groups.push((parts[0].to_string(), down, up));
            idx += 2;
            continue;
        }
        if arg == "--ratio-group" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --ratio-group".to_string())?;
            let parts: Vec<&str> = value.splitn(3, ':').collect();
            if parts.len() != 3 {
                return Err("--ratio-group format: name:ratio:action".to_string());
            }
            let ratio = parts[1]
                .parse::<f64>()
                .map_err(|_| "invalid ratio-group ratio".to_string())?;
            if !ratio.is_finite() || ratio < 0.0 {
                return Err("ratio-group ratio must be a finite value >= 0".to_string());
            }
            if parts[0].trim().is_empty() {
                return Err("ratio-group name must not be empty".to_string());
            }
            let action = parts[2].to_string();
            if !matches!(action.as_str(), "stop" | "pause" | "none") {
                return Err("ratio-group action must be stop, pause, or none".to_string());
            }
            ratio_groups.push((parts[0].to_string(), ratio, action));
            idx += 2;
            continue;
        }
        if arg == "--schedule" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --schedule".to_string())?;
            let (interval, command) = parse_schedule_arg(value)?;
            schedules.push((interval, command.to_string()));
            idx += 2;
            continue;
        }
        if arg == "--create" {
            let source = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --create".to_string())?
                .clone();
            let mut create_tracker = String::new();
            let mut create_output = String::new();
            let mut create_piece_length = 256 * 1024u64;
            let mut j = idx + 2;
            while j < args_list.len() {
                if args_list[j] == "--tracker" {
                    create_tracker = args_list
                        .get(j + 1)
                        .ok_or_else(|| "missing value for --tracker".to_string())?
                        .clone();
                    j += 2;
                } else if args_list[j] == "--output" {
                    create_output = args_list
                        .get(j + 1)
                        .ok_or_else(|| "missing value for --output".to_string())?
                        .clone();
                    j += 2;
                } else if args_list[j] == "--piece-length" {
                    create_piece_length = args_list
                        .get(j + 1)
                        .ok_or_else(|| "missing value for --piece-length".to_string())?
                        .parse::<u64>()
                        .map_err(|_| "invalid --piece-length".to_string())?;
                    j += 2;
                } else {
                    break;
                }
            }
            if create_output.is_empty() {
                create_output = format!("{}.torrent", source);
            }
            create_torrent(
                &PathBuf::from(&source),
                &create_tracker,
                &PathBuf::from(&create_output),
                create_piece_length,
            )?;
            std::process::exit(0);
        }
        if arg == "--move-completed" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --move-completed".to_string())?;
            move_completed = Some(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if arg == "--watch" {
            let value = args_list
                .get(idx + 1)
                .ok_or_else(|| "missing value for --watch".to_string())?;
            watch_dirs.push(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if !arg.starts_with("--") && torrent_path.is_none() {
            torrent_path = Some(arg.clone());
            idx += 1;
            continue;
        }
        return Err(format!("unknown argument: {arg}"));
    }
    if !cfg!(feature = "mse") {
        encryption = EncryptionMode::Disable;
    }
    if !cfg!(feature = "utp") {
        enable_utp = false;
    }
    if retry_interval == 0 {
        return Err("retry interval must be > 0".to_string());
    }
    if port == 0 {
        return Err("listen port must be > 0".to_string());
    }
    if rss_interval == 0 {
        return Err("rss interval must be > 0".to_string());
    }

    if daemon {
        ui = true;
        if log_path.is_none() {
            log_path = Some(download_dir.join("rustorrent.log"));
        }
    }

    if ui {
        validate_ui_bind_addr(&ui_addr)?;
    }

    if torrent_path.is_none()
        && magnet.is_none()
        && !ui
        && !tui
        && watch_dirs.is_empty()
        && rss_feeds.is_empty()
    {
        return Err(
            "usage: rustorrent [path.torrent] [--magnet <link>] [--config <path>] [--download-dir <dir>] [--preallocate] [--sequential] [--move-completed <dir>] [--watch <dir>] [--ui] [--ui-addr <addr>] [--tui] [--peer-profile <conservative|balanced|aggressive>] [--retry-interval <secs>] [--numwant <n>] [--port <port>] [--encryption <disable|prefer|require>] [--no-encryption] [--utp|--no-utp] [--blocklist <path>] [--max-active <n>] [--max-peers <n>] [--max-peers-torrent <n>] [--download-rate <bps>] [--upload-rate <bps>] [--torrent-download-rate <bps>] [--torrent-upload-rate <bps>] [--write-cache <bytes>] [--log <path>] [--daemon] [--pid-file <path>] [--seed-ratio <ratio>] [--max-seed-time <minutes>] [--on-complete <script>] [--super-seed] [--throttle <name:down_kbps:up_kbps>] [--ratio-group <name:ratio:action>] [--schedule <interval_secs:command>] [--rss <url>] [--rss-rule <feed_url:pattern>] [--rss-interval <secs>] [--create <path> --tracker <url> --output <file>]".to_string(),
        );
    }
    if max_peers_torrent == 0 {
        return Err("max peers per torrent must be > 0".to_string());
    }
    if max_peers_global == 0 {
        return Err("max peers globally must be > 0".to_string());
    }

    Ok(Args {
        torrent_path,
        magnet,
        download_dir,
        preallocate,
        ui,
        ui_addr,
        peer_profile,
        retry_interval,
        numwant,
        metadata_peer_limit,
        port,
        enable_utp,
        encryption,
        blocklist_path,
        max_peers_global,
        max_peers_torrent,
        max_active_torrents,
        download_rate,
        upload_rate,
        torrent_download_rate,
        torrent_upload_rate,
        write_cache_bytes,
        sequential,
        move_completed,
        watch_dirs,
        log_path,
        daemon,
        pid_file,
        seed_ratio,
        max_seed_time,
        on_complete,
        super_seed,
        tui,
        proxy: proxy_config,
        geoip_db: geoip_path,
        rss_feeds,
        rss_rules,
        rss_interval,
        throttle_groups,
        ratio_groups,
        schedules,
    })
}

fn validate_ui_bind_addr(value: &str) -> Result<SocketAddr, String> {
    let address = value
        .parse::<SocketAddr>()
        .map_err(|_| "web UI address must be a numeric IP address and port".to_string())?;
    if !address.ip().is_loopback() {
        return Err(
            "web UI may only bind to a loopback address; use an authenticated tunnel for remote access"
                .to_string(),
        );
    }
    Ok(address)
}

fn parse_encryption_mode(value: &str) -> Result<EncryptionMode, String> {
    match value.to_ascii_lowercase().as_str() {
        "disable" | "off" | "none" => Ok(EncryptionMode::Disable),
        "prefer" | "on" => Ok(EncryptionMode::Prefer),
        "require" | "force" => Ok(EncryptionMode::Require),
        _ => Err("invalid encryption mode".to_string()),
    }
}

fn parse_peer_profile(value: &str) -> Result<PeerProfile, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "conservative" | "low" | "light" => Ok(PeerProfile::Conservative),
        "balanced" | "default" | "normal" => Ok(PeerProfile::Balanced),
        "aggressive" | "high" | "max" => Ok(PeerProfile::Aggressive),
        _ => Err("invalid peer profile".to_string()),
    }
}

fn apply_peer_profile(
    profile: PeerProfile,
    numwant: &mut u32,
    metadata_peer_limit: &mut usize,
    max_peers_global: &mut usize,
    max_peers_torrent: &mut usize,
) {
    let tuning = profile.tuning();
    *numwant = tuning.numwant;
    *metadata_peer_limit = tuning.metadata_peer_limit;
    *max_peers_global = tuning.max_peers_global;
    *max_peers_torrent = tuning.max_peers_torrent;
}

fn parse_rate(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("invalid rate".to_string());
    }
    if trimmed.eq_ignore_ascii_case("unlimited") {
        return Ok(0);
    }
    let mut multiplier = 1u64;
    let mut number = trimmed;
    if let Some(last) = trimmed.chars().last() {
        if last.is_ascii_alphabetic() {
            multiplier = match last.to_ascii_lowercase() {
                'k' => 1024,
                'm' => 1024 * 1024,
                'g' => 1024 * 1024 * 1024,
                _ => return Err("invalid rate suffix".to_string()),
            };
            number = &trimmed[..trimmed.len() - 1];
        }
    }
    let base = number
        .trim()
        .parse::<u64>()
        .map_err(|_| "invalid rate".to_string())?;
    base.checked_mul(multiplier)
        .ok_or_else(|| "rate is too large".to_string())
}

fn parse_size(value: &str) -> Result<usize, String> {
    usize::try_from(parse_rate(value)?)
        .map_err(|_| "size is too large for this platform".to_string())
}

fn parse_rss_rule_arg(value: &str) -> Result<(&str, &str), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("missing value for --rss-rule".to_string());
    }
    if let Some((feed_url, pattern)) = trimmed.rsplit_once(':') {
        if pattern.trim().is_empty() {
            return Err("rss rule pattern is empty".to_string());
        }
        Ok((feed_url, pattern))
    } else {
        Ok(("", trimmed))
    }
}

fn parse_schedule_arg(value: &str) -> Result<(u64, &str), String> {
    if let Some((interval_str, command)) = value.split_once(':') {
        let interval = interval_str
            .parse::<u64>()
            .map_err(|_| "invalid schedule interval".to_string())?;
        if interval == 0 {
            return Err("schedule interval must be > 0".to_string());
        }
        if command.trim().is_empty() {
            return Err("schedule command must not be empty".to_string());
        }
        Ok((interval, command))
    } else {
        Err("--schedule format: interval_secs:command".to_string())
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_overrides(
    cfg: &ConfigOverrides,
    download_dir: &mut PathBuf,
    preallocate: &mut bool,
    ui: &mut bool,
    ui_addr: &mut String,
    peer_profile: &mut PeerProfile,
    retry_interval: &mut u64,
    numwant: &mut u32,
    metadata_peer_limit: &mut usize,
    port: &mut u16,
    enable_utp: &mut bool,
    encryption: &mut EncryptionMode,
    blocklist_path: &mut Option<PathBuf>,
    max_peers_global: &mut usize,
    max_peers_torrent: &mut usize,
    max_active_torrents: &mut usize,
    download_rate: &mut u64,
    upload_rate: &mut u64,
    torrent_download_rate: &mut u64,
    torrent_upload_rate: &mut u64,
    write_cache_bytes: &mut usize,
    geoip_path: &mut Option<PathBuf>,
) {
    if let Some(dir) = cfg.download_dir.clone() {
        *download_dir = dir;
    }
    if let Some(value) = cfg.preallocate {
        *preallocate = value;
    }
    if let Some(value) = cfg.ui {
        *ui = value;
    }
    if let Some(value) = cfg.ui_addr.clone() {
        *ui_addr = value;
    }
    if let Some(value) = cfg.peer_profile {
        *peer_profile = value;
        apply_peer_profile(
            value,
            numwant,
            metadata_peer_limit,
            max_peers_global,
            max_peers_torrent,
        );
    }
    if let Some(value) = cfg.retry_interval {
        *retry_interval = value;
    }
    if let Some(value) = cfg.numwant {
        *numwant = value;
    }
    if let Some(value) = cfg.port {
        *port = value;
    }
    if let Some(value) = cfg.enable_utp {
        *enable_utp = value;
    }
    if let Some(value) = cfg.encryption {
        *encryption = value;
    }
    if let Some(value) = cfg.blocklist_path.clone() {
        *blocklist_path = Some(value);
    }
    if let Some(value) = cfg.max_peers_global {
        *max_peers_global = value;
    }
    if let Some(value) = cfg.max_peers_torrent {
        *max_peers_torrent = value;
    }
    if let Some(value) = cfg.max_active_torrents {
        *max_active_torrents = value;
    }
    if let Some(value) = cfg.download_rate {
        *download_rate = value;
    }
    if let Some(value) = cfg.upload_rate {
        *upload_rate = value;
    }
    if let Some(value) = cfg.torrent_download_rate {
        *torrent_download_rate = value;
    }
    if let Some(value) = cfg.torrent_upload_rate {
        *torrent_upload_rate = value;
    }
    if let Some(value) = cfg.write_cache_bytes {
        *write_cache_bytes = value;
    }
    if let Some(value) = cfg.geoip_db.clone() {
        *geoip_path = Some(value);
    }
}

fn warn_invalid<T>(value: Option<T>, key: &str, raw: &str) -> Option<T> {
    if value.is_none() {
        eprintln!(
            "warning: invalid value for '{}': '{}', using default",
            key, raw
        );
    }
    value
}

fn load_config_overrides(path: &Path) -> Result<ConfigOverrides, String> {
    let data = read_file_limited(path, MAX_CONFIG_BYTES, false)
        .map_err(|err| format!("config read failed: {err}"))?;
    let text =
        std::str::from_utf8(&data).map_err(|_| "config read failed: invalid UTF-8".to_string())?;
    let mut cfg = ConfigOverrides::default();
    for (line_no, raw) in text.lines().enumerate() {
        let mut line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((left, _)) = line.split_once('#') {
            line = left.trim();
        }
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("config line {} invalid", line_no + 1))?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "download_dir" => cfg.download_dir = Some(PathBuf::from(value)),
            "preallocate" => cfg.preallocate = parse_bool_value(value),
            "ui" => cfg.ui = parse_bool_value(value),
            "ui_addr" => cfg.ui_addr = Some(value.to_string()),
            "peer_profile" | "network_profile" => {
                cfg.peer_profile = warn_invalid(parse_peer_profile(value).ok(), &key, value);
            }
            "retry_interval" => {
                cfg.retry_interval = warn_invalid(value.parse::<u64>().ok(), &key, value);
            }
            "numwant" => cfg.numwant = warn_invalid(value.parse::<u32>().ok(), &key, value),
            "port" => cfg.port = warn_invalid(value.parse::<u16>().ok(), &key, value),
            "utp" => cfg.enable_utp = parse_bool_value(value),
            "encryption" => {
                cfg.encryption = warn_invalid(parse_encryption_mode(value).ok(), &key, value);
            }
            "blocklist" => cfg.blocklist_path = Some(PathBuf::from(value)),
            "max_peers" => {
                cfg.max_peers_global = warn_invalid(value.parse::<usize>().ok(), &key, value);
            }
            "max_peers_torrent" => {
                cfg.max_peers_torrent = warn_invalid(value.parse::<usize>().ok(), &key, value);
            }
            "max_active_torrents" | "max_active" => {
                cfg.max_active_torrents = warn_invalid(value.parse::<usize>().ok(), &key, value);
            }
            "download_rate" => {
                cfg.download_rate = warn_invalid(parse_rate(value).ok(), &key, value);
            }
            "upload_rate" => {
                cfg.upload_rate = warn_invalid(parse_rate(value).ok(), &key, value);
            }
            "torrent_download_rate" => {
                cfg.torrent_download_rate = warn_invalid(parse_rate(value).ok(), &key, value);
            }
            "torrent_upload_rate" => {
                cfg.torrent_upload_rate = warn_invalid(parse_rate(value).ok(), &key, value);
            }
            "write_cache" => {
                cfg.write_cache_bytes = warn_invalid(parse_size(value).ok(), &key, value);
            }
            "geoip_db" | "geoip" => cfg.geoip_db = Some(PathBuf::from(value)),
            _ => return Err(format!("config line {} unknown key", line_no + 1)),
        }
    }
    Ok(cfg)
}

fn load_env_overrides() -> ConfigOverrides {
    let mut cfg = ConfigOverrides::default();
    if let Ok(value) = env::var("RUSTORRENT_DOWNLOAD_DIR") {
        cfg.download_dir = Some(PathBuf::from(value));
    }
    if let Ok(value) = env::var("RUSTORRENT_PREALLOCATE") {
        cfg.preallocate = parse_bool_value(&value);
    }
    if let Ok(value) = env::var("RUSTORRENT_UI") {
        cfg.ui = parse_bool_value(&value);
    }
    if let Ok(value) = env::var("RUSTORRENT_UI_ADDR") {
        cfg.ui_addr = Some(value);
    }
    if let Ok(value) = env::var("RUSTORRENT_PEER_PROFILE") {
        cfg.peer_profile = parse_peer_profile(&value).ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_RETRY_INTERVAL") {
        cfg.retry_interval = value.parse::<u64>().ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_NUMWANT") {
        cfg.numwant = value.parse::<u32>().ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_PORT") {
        cfg.port = value.parse::<u16>().ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_UTP") {
        cfg.enable_utp = parse_bool_value(&value);
    }
    if let Ok(value) = env::var("RUSTORRENT_ENCRYPTION") {
        cfg.encryption = parse_encryption_mode(&value).ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_BLOCKLIST") {
        cfg.blocklist_path = Some(PathBuf::from(value));
    }
    if let Ok(value) = env::var("RUSTORRENT_MAX_PEERS") {
        cfg.max_peers_global = value.parse::<usize>().ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_MAX_PEERS_TORRENT") {
        cfg.max_peers_torrent = value.parse::<usize>().ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_MAX_ACTIVE_TORRENTS") {
        cfg.max_active_torrents = value.parse::<usize>().ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_DOWNLOAD_RATE") {
        cfg.download_rate = parse_rate(&value).ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_UPLOAD_RATE") {
        cfg.upload_rate = parse_rate(&value).ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_TORRENT_DOWNLOAD_RATE") {
        cfg.torrent_download_rate = parse_rate(&value).ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_TORRENT_UPLOAD_RATE") {
        cfg.torrent_upload_rate = parse_rate(&value).ok();
    }
    if let Ok(value) = env::var("RUSTORRENT_WRITE_CACHE") {
        cfg.write_cache_bytes = parse_size(&value).ok();
    }
    cfg
}

fn parse_bool_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn verify_piece_hash(data: &[u8], expected: &piece::PieceHash) -> bool {
    expected.verify(data)
}

#[derive(Clone)]
struct TrackerSet {
    http: Vec<String>,
    udp: Vec<String>,
}

fn tracker_set_has_usable_source(trackers: &TrackerSet, allow_udp: bool) -> bool {
    !trackers.http.is_empty() || (allow_udp && !trackers.udp.is_empty())
}

struct TrackerAnnounceOutcome {
    tracker_url: String,
    is_udp: bool,
    response: Result<tracker::TrackerResponse, String>,
}

struct TrackerWorkerGuard;

impl Drop for TrackerWorkerGuard {
    fn drop(&mut self) {
        ACTIVE_TRACKER_WORKERS.fetch_sub(1, Ordering::SeqCst);
    }
}

fn try_acquire_tracker_worker() -> Option<TrackerWorkerGuard> {
    ACTIVE_TRACKER_WORKERS
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |active| {
            (active < MAX_GLOBAL_TRACKER_WORKERS).then_some(active + 1)
        })
        .ok()
        .map(|_| TrackerWorkerGuard)
}

fn collect_trackers(meta: &torrent::TorrentMeta) -> TrackerSet {
    let mut http = Vec::new();
    let mut udp = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |url: &str| {
        if seen.len() >= MAX_TRACKERS_PER_TORRENT
            || !valid_tracker_url(url)
            || !seen.insert(url.to_string())
        {
            return;
        }
        if url.starts_with("http://") || url.starts_with("https://") {
            http.push(url.to_string());
        } else if url.starts_with("udp://") {
            udp.push(url.to_string());
        }
    };

    // Add trackers from torrent file (BEP 12: iterate tiers)
    for tier in &meta.announce_list {
        for url in tier {
            if let Ok(url_str) = std::str::from_utf8(url) {
                push(url_str);
            }
        }
    }
    if let Some(url) = meta.announce.as_ref() {
        if let Ok(url_str) = std::str::from_utf8(url) {
            push(url_str);
        }
    }

    TrackerSet { http, udp }
}

fn valid_tracker_url(url: &str) -> bool {
    valid_network_url(url, &["http://", "https://", "udp://"])
}

fn valid_network_url(url: &str, allowed_schemes: &[&str]) -> bool {
    if url.is_empty()
        || url.len() > MAX_TRACKER_URL_LEN
        || url
            .chars()
            .any(|character| character.is_whitespace() || unsafe_log_character(character))
    {
        return false;
    }
    let rest = allowed_schemes
        .iter()
        .find_map(|scheme| url.strip_prefix(scheme));
    let Some(rest) = rest else {
        return false;
    };
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    !authority.is_empty()
        && !authority.contains(['@', '\\'])
        && !authority.chars().any(unsafe_log_character)
}

fn unsafe_log_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

fn safe_network_url_label(url: &str) -> String {
    let Some((scheme, rest)) = ["http://", "https://", "udp://"]
        .into_iter()
        .find_map(|scheme| url.strip_prefix(scheme).map(|rest| (scheme, rest)))
    else {
        return "<invalid-url>".to_string();
    };
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return "<invalid-url>".to_string();
    }
    format!(
        "{scheme}{}",
        tracker::sanitize_failure_reason(authority.as_bytes())
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_tracker_announces(
    trackers: &TrackerSet,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<&str>,
    numwant: u32,
    is_private: bool,
    proxy_config: Option<proxy::ProxyConfig>,
    wait_budget: Duration,
) -> (mpsc::Receiver<TrackerAnnounceOutcome>, usize) {
    let (tx, rx) = mpsc::channel::<TrackerAnnounceOutcome>();
    let event = event.map(str::to_string);
    let mut tasks = VecDeque::new();
    for tracker_url in &trackers.http {
        if tasks.len() >= MAX_TRACKERS_PER_TORRENT {
            break;
        }
        if valid_tracker_url(tracker_url) {
            tasks.push_back((tracker_url.clone(), false));
        }
    }
    if proxy_config.is_none() {
        for tracker_url in &trackers.udp {
            if tasks.len() >= MAX_TRACKERS_PER_TORRENT {
                break;
            }
            if valid_tracker_url(tracker_url) {
                tasks.push_back((tracker_url.clone(), true));
            }
        }
    }

    let pending = tasks.len();
    let worker_count = pending.min(MAX_TRACKER_WORKERS);
    let tasks = Arc::new(Mutex::new(tasks));
    let stop_at = Instant::now() + wait_budget;
    let mut workers_started = 0usize;
    for _ in 0..worker_count {
        let Some(worker_guard) = try_acquire_tracker_worker() else {
            break;
        };
        let tx = tx.clone();
        let tasks = Arc::clone(&tasks);
        let event = event.clone();
        let proxy_config = proxy_config.clone();
        match thread::Builder::new()
            .stack_size(PEER_THREAD_STACK)
            .spawn(move || {
                let _worker_guard = worker_guard;
                loop {
                    if Instant::now() >= stop_at {
                        break;
                    }
                    let task = lock_or_recover(&tasks).pop_front();
                    let Some((tracker_url, is_udp)) = task else {
                        break;
                    };
                    let response = if is_udp {
                        udp_tracker::announce_until(
                            &tracker_url,
                            info_hash,
                            peer_id,
                            port,
                            uploaded,
                            downloaded,
                            left,
                            event.as_deref(),
                            numwant,
                            stop_at,
                        )
                        .map_err(|err| err.to_string())
                    } else {
                        tracker::announce_with_private_until(
                            &tracker_url,
                            info_hash,
                            peer_id,
                            port,
                            uploaded,
                            downloaded,
                            left,
                            event.as_deref(),
                            numwant,
                            is_private,
                            proxy_config.as_ref(),
                            stop_at,
                        )
                        .map_err(|err| err.to_string())
                    };
                    let _ = tx.send(TrackerAnnounceOutcome {
                        tracker_url,
                        is_udp,
                        response,
                    });
                }
            }) {
            Ok(_) => workers_started += 1,
            Err(err) => {
                log_warn!("tracker worker spawn failed: {err}");
            }
        }
    }

    drop(tx);
    (rx, if workers_started == 0 { 0 } else { pending })
}

fn generate_peer_id() -> [u8; 20] {
    let mut out = [0u8; 20];
    out[..8].copy_from_slice(b"-RT0001-");
    let mut seed = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as u64,
        Err(_) => 0,
    };
    seed ^= std::process::id() as u64;
    for slot in &mut out[8..] {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *slot = (seed & 0xff) as u8;
    }
    out
}

/// Concurrent version of download_from_peer that works with Arc<Mutex<>> shared state
#[allow(clippy::too_many_arguments)]
fn download_from_peer_concurrent(
    addr: SocketAddr,
    info_hash: [u8; 20],
    hybrid_v2_info_hash: Option<[u8; 20]>,
    peer_id: [u8; 20],
    torrent_id: u64,
    peer_tag: u64,
    pieces: &Arc<Mutex<piece::PieceManager>>,
    storage: &Arc<Mutex<storage::Storage>>,
    completed_log: &Arc<Mutex<Vec<u32>>>,
    peer_queue: &Arc<Mutex<PeerQueue>>,
    allow_pex: bool,
    file_spans: &Arc<Vec<FileSpan>>,
    _base_piece_length: u64,
    v2_hashes: &Arc<V2HashStore>,
    connect_cfg: &ConnectionConfig,
    limits: &TransferLimits,
    downloaded: &Arc<AtomicU64>,
    uploaded: &Arc<AtomicU64>,
    active_peers: &Arc<AtomicUsize>,
    interested_peers: &Arc<AtomicUsize>,
    upload_requests_served: &Arc<AtomicU64>,
    upload_manager: &Arc<UploadManager>,
    peer_cancellations: &PeerCancellationRegistry,
    paused_flag: &Arc<AtomicBool>,
    stop_flag: &Arc<AtomicBool>,
    piece_buffer_budgets: &piece::PieceBufferBudgets,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
) -> Result<(), String> {
    let mut stream = connect_peer(addr, connect_cfg)?;
    let cancellation = PeerCancellationGuard::new(peer_cancellations, peer_tag, &stream);
    if torrent_stop_requested(stop_flag) {
        return Err("torrent stopping".to_string());
    }
    // Use a longer timeout for the handshake phase (peers may be slow to respond)
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| format!("read timeout failed: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| format!("write timeout failed: {err}"))?;

    let handshake = if connect_cfg.encryption == EncryptionMode::Require {
        outbound_handshake(
            &mut stream,
            info_hash,
            hybrid_v2_info_hash,
            peer_id,
            connect_cfg.encryption,
        )?
    } else {
        match plaintext_handshake(&mut stream, info_hash, hybrid_v2_info_hash, peer_id) {
            Ok(handshake) => handshake,
            Err(err) if connect_cfg.encryption == EncryptionMode::Prefer => {
                let _ = err;
                log_debug!("plaintext failed, retrying mse: {err}");
                let mut retry = connect_peer(addr, connect_cfg)?;
                retry
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|err| format!("read timeout failed: {err}"))?;
                retry
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .map_err(|err| format!("write timeout failed: {err}"))?;
                cancellation.replace_stream(&retry);
                if torrent_stop_requested(stop_flag) {
                    return Err("torrent stopping".to_string());
                }
                let handshake = outbound_handshake(
                    &mut retry,
                    info_hash,
                    hybrid_v2_info_hash,
                    peer_id,
                    EncryptionMode::Prefer,
                )
                .map_err(|err| format!("handshake failed: {err}"))?;
                stream = retry;
                handshake
            }
            Err(err) => return Err(format!("handshake failed: {err}")),
        }
    };
    // Reduce timeout for the main peer loop (need responsiveness for piece requests)
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));

    if is_self_peer_id(&peer_id, &handshake.peer_id) {
        return Err("self peer".to_string());
    }

    log_debug!("peer: {addr}");
    log_debug!("peer id: {}", hex(&handshake.peer_id));

    // Send bitfield first (BEP 3: must be first message after handshake)
    let (local_bitfield, have_pieces, mut seed_mode) = {
        let p = lock_or_recover(pieces);
        let bits = build_bitfield(&p);
        let have = p.completed_pieces() > 0;
        let seed = p.is_complete();
        (bits, have, seed)
    };
    let super_seed_mode = seed_mode && SUPER_SEED.load(Ordering::SeqCst);
    let mut super_seed_piece: Option<u32> = None;
    if super_seed_mode && have_pieces {
        // BEP 16: send HAVE for only one piece instead of full bitfield
        let piece_count = {
            let p = lock_or_recover(pieces);
            p.piece_count()
        };
        if piece_count > 0 {
            // Pick a random piece to advertise
            let mut seed_val = std::process::id() as u64;
            seed_val ^= Instant::now().elapsed().as_nanos() as u64;
            seed_val ^= seed_val << 13;
            seed_val ^= seed_val >> 7;
            let idx = (seed_val % piece_count as u64) as u32;
            peer::write_message(&mut stream, &peer::Message::Have(idx))
                .map_err(|err| format!("super-seed have write failed: {err}"))?;
            super_seed_piece = Some(idx);
        }
    } else {
        // BEP 3: always send bitfield (even if empty) as first message after handshake
        peer::write_message(&mut stream, &peer::Message::Bitfield(local_bitfield))
            .map_err(|err| format!("bitfield write failed: {err}"))?;
    }

    let mut peer_ut_pex: Option<u8> = None;
    let mut last_pex = Instant::now();
    if handshake.supports_extensions() {
        let ext_handshake = build_ext_handshake(None, allow_pex);
        peer::write_message(
            &mut stream,
            &peer::Message::Extended {
                ext_id: 0,
                payload: ext_handshake,
            },
        )
        .map_err(|err| format!("ext handshake failed: {err}"))?;
    }

    if seed_mode {
        let _ = peer::write_message(&mut stream, &peer::Message::NotInterested);
    } else {
        peer::write_message(&mut stream, &peer::Message::Interested)
            .map_err(|err| format!("interested write failed: {err}"))?;
    }
    // Flush all handshake messages (bitfield + ext + interested) together
    let _ = stream.flush();

    upload_manager.register(peer_tag);
    PEER_CONNECTED.fetch_add(1, Ordering::SeqCst);
    let geo_cc = add_active_peer_session(ui_state, torrent_id, active_peers, addr);

    let mut reader = peer::MessageReader::new();
    let mut bitfield: Option<Vec<u8>> = None;
    let mut choked = true;
    let mut peer_interested = false;
    let mut am_choking = true;
    let mut pending: Vec<PendingRequest> = Vec::new();
    let mut active_pieces: HashMap<u32, piece::PieceBuffer> = HashMap::new();
    let mut idle = 0u32;
    let mut last_sent = Instant::now();
    let mut endgame_announced = false;
    let mut timed_out: Vec<piece::BlockRequest> = Vec::new();
    let mut pause_sent = false;
    let mut pipeline_depth = PIPELINE_DEPTH;
    let mut peer_rate_bps = DEFAULT_PEER_RATE_BPS;
    let mut peer_rate_sample_bytes = 0usize;
    let mut peer_rate_last_at = Instant::now();
    let mut choke_since: Option<Instant> = None;
    let mut last_piece_data = Instant::now();
    let mut completed_cursor = {
        let log = lock_or_recover(completed_log);
        log.len()
    };
    let mut hash_request_budget = HashRequestBudget::new();
    let mut last_served_chunk: Option<(u32, u32, u32)> = None;
    let ban_peer = |reason: &str| {
        if let Ok(mut queue) = peer_queue.lock() {
            queue.ban(addr);
        }
        let _ = reason;
        log_debug!("banned peer {addr}: {reason}");
    };

    let result = (|| -> Result<(), String> {
        loop {
            if torrent_stop_requested(stop_flag) {
                cancel_pending(&mut stream, &pending)?;
                return Ok(());
            }
            let obsolete_active = {
                let p = lock_or_recover(pieces);
                active_pieces
                    .keys()
                    .copied()
                    .filter(|index| p.is_piece_complete(*index) || !p.is_piece_wanted(*index))
                    .collect::<Vec<_>>()
            };
            if !obsolete_active.is_empty() {
                let mut cancelled = Vec::new();
                pending.retain(|entry| {
                    if obsolete_active.contains(&entry.request.index) {
                        cancelled.push(entry.request);
                        false
                    } else {
                        true
                    }
                });
                for request in cancelled {
                    peer::write_message(
                        &mut stream,
                        &peer::Message::Cancel {
                            index: request.index,
                            begin: request.begin,
                            length: request.length,
                        },
                    )
                    .map_err(|err| format!("cancel obsolete piece failed: {err}"))?;
                }
                {
                    let mut p = lock_or_recover(pieces);
                    for index in &obsolete_active {
                        if !p.is_piece_complete(*index) {
                            let _ = p.reset_piece(*index);
                        }
                        p.release_piece(peer_tag, *index);
                    }
                }
                let released = obsolete_active
                    .into_iter()
                    .filter_map(|index| active_pieces.remove(&index))
                    .collect::<Vec<_>>();
                drop(released);
            }
            const SEED_TO_SEED_IDLE_TICKS: u32 = 240; // 2 min for seeder-to-seeder
            let idle_limit = if seed_mode && !peer_interested {
                SEED_TO_SEED_IDLE_TICKS
            } else if seed_mode {
                MAX_IDLE_TICKS_SEED
            } else {
                MAX_IDLE_TICKS
            };
            if idle > idle_limit {
                return Err("peer timed out".to_string());
            }

            // Release stale reserved piece after prolonged choke (60s)
            if choked {
                if let Some(since) = choke_since {
                    if since.elapsed() > Duration::from_secs(60) {
                        if !active_pieces.is_empty() {
                            let mut released = Vec::new();
                            {
                                let mut p = lock_or_recover(pieces);
                                for active in active_pieces.drain().map(|(_, piece)| piece) {
                                    if !active.is_complete() {
                                        let _ = p.reset_piece(active.index());
                                    }
                                    p.release_piece(peer_tag, active.index());
                                    log_debug!(
                                        "released stale piece {} from choked peer {addr}",
                                        active.index()
                                    );
                                    released.push(active);
                                }
                            }
                            drop(released);
                        }
                        choke_since = None;
                    }
                }
            }

            // Snub detection: disconnect peer if no data for 60s while unchoked
            if !choked
                && !seed_mode
                && !active_pieces.is_empty()
                && last_piece_data.elapsed() > SNUB_TIMEOUT
            {
                return Err("peer snubbed (no data for 60s while unchoked)".to_string());
            }

            // Check completion with lock
            let now_complete = {
                let p = lock_or_recover(pieces);
                p.is_complete()
            };
            if now_complete && !seed_mode {
                seed_mode = true;
                log_info!("download complete");
                if !pending.is_empty() {
                    cancel_pending(&mut stream, &pending)?;
                    pending.clear();
                }
                if !active_pieces.is_empty() {
                    let released = {
                        let mut p = lock_or_recover(pieces);
                        let released = active_pieces
                            .drain()
                            .map(|(_, piece)| piece)
                            .collect::<Vec<_>>();
                        for active in &released {
                            p.release_piece(peer_tag, active.index());
                        }
                        released
                    };
                    drop(released);
                }
                let _ = peer::write_message(&mut stream, &peer::Message::NotInterested);
                update_ui(ui_state, |state| {
                    if state.current_id == Some(torrent_id) {
                        state.status = "seeding".to_string();
                    }
                    update_torrent_entry(state, torrent_id, |torrent| {
                        torrent.status = "seeding".to_string();
                    });
                });
            }

            // Check endgame mode
            let endgame = {
                let p = lock_or_recover(pieces);
                p.remaining_blocks() <= ENDGAME_BLOCKS
            };
            if endgame && !endgame_announced {
                log_debug!("endgame mode");
                endgame_announced = true;
            }

            let paused = torrent_paused(paused_flag);
            if !seed_mode {
                if paused && !pause_sent {
                    if !pending.is_empty() {
                        cancel_pending(&mut stream, &pending)?;
                    }
                    if !pending.is_empty() || !active_pieces.is_empty() {
                        {
                            let mut p = lock_or_recover(pieces);
                            abandon_inflight(&mut p, &mut pending, &active_pieces);
                        }
                        active_pieces.clear();
                    }
                    peer::write_message(&mut stream, &peer::Message::NotInterested)
                        .map_err(|err| format!("not-interested write failed: {err}"))?;
                    pause_sent = true;
                } else if !paused && pause_sent {
                    peer::write_message(&mut stream, &peer::Message::Interested)
                        .map_err(|err| format!("interested write failed: {err}"))?;
                    pause_sent = false;
                }
            }
            if paused && !am_choking {
                if peer::write_message(&mut stream, &peer::Message::Choke).is_ok() {
                    am_choking = true;
                    last_sent = Instant::now();
                }
            } else if !paused {
                let should_unchoke = upload_manager.should_unchoke(peer_tag);
                if should_unchoke && am_choking {
                    if peer::write_message(&mut stream, &peer::Message::Unchoke).is_ok() {
                        am_choking = false;
                        last_sent = Instant::now();
                    }
                } else if !should_unchoke
                    && !am_choking
                    && peer::write_message(&mut stream, &peer::Message::Choke).is_ok()
                {
                    am_choking = true;
                    last_sent = Instant::now();
                }
            }

            if !seed_mode && !choked && !paused {
                if let Some(bits) = bitfield.as_ref() {
                    while pending.len() < pipeline_depth {
                        let mut req = None;
                        let mut active_indexes: Vec<u32> = active_pieces.keys().copied().collect();
                        active_indexes.sort_unstable();
                        for active_index in active_indexes {
                            req = {
                                let mut p = lock_or_recover(pieces);
                                p.next_request_for_piece(active_index, endgame)
                            };
                            if req.is_some() {
                                break;
                            }
                        }
                        if req.is_none() && active_pieces.len() < MAX_ACTIVE_PIECES_PER_PEER {
                            let (selected, has_needed) = {
                                let mut p = lock_or_recover(pieces);
                                let selected = p.reserve_piece_for_peer(peer_tag, bits, endgame);
                                let has_needed = selected.is_some()
                                    || p.has_needed_piece(bits)
                                    || (endgame && p.remaining_blocks() > 0);
                                (selected, has_needed)
                            };
                            // If no piece was reserved normally and not in endgame, try stealing
                            let selected = selected.or_else(|| {
                                if !endgame {
                                    let mut p = lock_or_recover(pieces);
                                    p.steal_stale_piece(peer_tag, bits, Duration::from_secs(30))
                                } else {
                                    None
                                }
                            });
                            if let Some(index) = selected {
                                if active_pieces.contains_key(&index) {
                                    continue;
                                }
                                let length = {
                                    let p = lock_or_recover(pieces);
                                    p.piece_length(index)
                                };
                                let length = match length {
                                    Some(length) => length,
                                    None => {
                                        let mut p = lock_or_recover(pieces);
                                        p.release_piece(peer_tag, index);
                                        return Err("invalid piece length".to_string());
                                    }
                                };
                                let buffer = match allocate_reserved_piece_buffer(
                                    pieces,
                                    peer_tag,
                                    index,
                                    length,
                                    piece_buffer_budgets,
                                ) {
                                    Ok(Some(buffer)) => buffer,
                                    Ok(None) => break,
                                    Err(err) => return Err(err),
                                };
                                log_debug!("selected piece {index} from {addr}");
                                active_pieces.insert(index, buffer);
                                continue;
                            } else if active_pieces.is_empty() && !has_needed {
                                log_debug!("peer {addr} has no needed pieces");
                                return Ok(());
                            }
                        }
                        let req = if let Some(req) = req {
                            req
                        } else {
                            if endgame {
                                if let Some(entry) = oldest_pending(&pending) {
                                    if entry.sent_at.elapsed() > ENDGAME_DUP_TIMEOUT {
                                        peer::write_message(
                                            &mut stream,
                                            &peer::Message::Request {
                                                index: entry.request.index,
                                                begin: entry.request.begin,
                                                length: entry.request.length,
                                            },
                                        )
                                        .map_err(|err| format!("request write failed: {err}"))?;
                                        log_debug!(
                                            "endgame duplicate: piece={} begin={} length={}",
                                            entry.request.index,
                                            entry.request.begin,
                                            entry.request.length
                                        );
                                    }
                                }
                            }
                            break;
                        };
                        peer::write_message(
                            &mut stream,
                            &peer::Message::Request {
                                index: req.index,
                                begin: req.begin,
                                length: req.length,
                            },
                        )
                        .map_err(|err| format!("request write failed: {err}"))?;
                        log_debug!(
                            "requested block: piece={} begin={} length={}",
                            req.index,
                            req.begin,
                            req.length
                        );
                        pending.push(PendingRequest {
                            request: req,
                            sent_at: Instant::now(),
                        });
                    }
                }
            }

            if last_sent.elapsed() >= KEEPALIVE_INTERVAL
                && peer::write_message(&mut stream, &peer::Message::KeepAlive).is_ok()
            {
                last_sent = Instant::now();
                idle = 0;
            }

            send_completed_updates(&mut stream, completed_log, &mut completed_cursor)?;

            if allow_pex {
                if let Some(ext_id) = peer_ut_pex {
                    if last_pex.elapsed() > Duration::from_secs(60) {
                        let peers = {
                            let queue = lock_or_recover(peer_queue);
                            queue.sample(50)
                        };
                        if !peers.is_empty() {
                            let payload = build_ut_pex_payload(&peers, &[]);
                            let _ = peer::write_message(
                                &mut stream,
                                &peer::Message::Extended { ext_id, payload },
                            );
                        }
                        last_pex = Instant::now();
                    }
                }
            }

            match reader.read_message(&mut stream) {
                Ok(Some(message)) => {
                    log_debug!("peer msg: {}", message_summary(&message));
                    idle = 0;
                    let immediately_after_served_chunk = last_served_chunk.take();
                    match message {
                        peer::Message::Extended { ext_id, payload } => {
                            if ext_id == 0 {
                                if let Ok((_ut_meta, ut_pex, _size)) =
                                    parse_extended_handshake(&payload)
                                {
                                    if allow_pex {
                                        peer_ut_pex = ut_pex;
                                    }
                                }
                            } else if allow_pex && Some(ext_id) == peer_ut_pex {
                                if let Ok(peers) = parse_ut_pex(&payload) {
                                    if !peers.is_empty() {
                                        let mut queue = lock_or_recover(peer_queue);
                                        queue.enqueue_with_source(peers, PeerSource::Pex);
                                    }
                                }
                            }
                        }
                        peer::Message::Bitfield(bits) => {
                            let mut p = lock_or_recover(pieces);
                            if bitfield.is_some() {
                                ban_peer("duplicate bitfield");
                                return Err("duplicate bitfield".to_string());
                            }
                            if let Err(err) = p.apply_peer_bitfield(&bits) {
                                ban_peer("invalid bitfield");
                                return Err(format!("bitfield error: {err}"));
                            }
                            bitfield = Some(bits);
                        }
                        peer::Message::Have(index) => {
                            let mut p = lock_or_recover(pieces);
                            let len = p.bitfield_len();
                            if bitfield.is_none() {
                                bitfield = Some(vec![0u8; len]);
                            }
                            if let Some(bits) = bitfield.as_mut() {
                                let idx = index as usize;
                                if idx >= p.piece_count() {
                                    ban_peer("invalid have index");
                                    return Err("have index out of range".to_string());
                                }
                                if !bitfield_has(bits, idx) {
                                    if let Err(err) = p.apply_have(index) {
                                        ban_peer("invalid have");
                                        return Err(format!("have error: {err}"));
                                    }
                                    if let Err(err) = set_bit(bits, idx) {
                                        ban_peer("invalid have index");
                                        return Err(err);
                                    }
                                }
                            }
                            // Super seed: peer redistributed our piece, advertise next
                            if super_seed_mode && super_seed_piece == Some(index) {
                                let piece_count = p.piece_count();
                                if piece_count > 0 {
                                    let next = (index + 1) % piece_count as u32;
                                    let _ = peer::write_message(
                                        &mut stream,
                                        &peer::Message::Have(next),
                                    );
                                    super_seed_piece = Some(next);
                                }
                            }
                        }
                        peer::Message::Interested => {
                            set_peer_interest(interested_peers, &mut peer_interested, true);
                            upload_manager.set_interested(peer_tag, true);
                            // Immediately unchoke if eligible (don't wait for next loop)
                            if am_choking
                                && !paused
                                && upload_manager.should_unchoke(peer_tag)
                                && peer::write_message(&mut stream, &peer::Message::Unchoke).is_ok()
                            {
                                am_choking = false;
                                last_sent = Instant::now();
                            }
                        }
                        peer::Message::NotInterested => {
                            set_peer_interest(interested_peers, &mut peer_interested, false);
                            upload_manager.set_interested(peer_tag, false);
                        }
                        peer::Message::Choke => {
                            choked = true;
                            choke_since = Some(Instant::now());
                            log_debug!(
                                "choked by {addr}, had {} pending, active_pieces={}",
                                pending.len(),
                                active_pieces.len()
                            );
                            cancel_pending(&mut stream, &pending)?;
                            {
                                let mut p = lock_or_recover(pieces);
                                for entry in pending.drain(..) {
                                    p.mark_block_missing(entry.request.index, entry.request.begin)
                                        .map_err(|err| format!("block timeout: {err}"))?;
                                }
                            }
                        }
                        peer::Message::Unchoke => {
                            choked = false;
                            choke_since = None;
                            last_piece_data = Instant::now();
                            log_debug!("unchoked by {addr}");
                        }
                        peer::Message::Request {
                            index,
                            begin,
                            length,
                        } => {
                            if !am_choking && peer_interested {
                                if let Err(err) = handle_upload_request(
                                    &mut stream,
                                    pieces,
                                    storage,
                                    index,
                                    begin,
                                    length,
                                    limits,
                                    uploaded,
                                    upload_requests_served,
                                    upload_manager,
                                    peer_tag,
                                ) {
                                    let _ = err;
                                    log_debug!("upload request rejected: {err}");
                                } else {
                                    last_served_chunk = Some((index, begin, length));
                                    last_sent = Instant::now();
                                    check_seed_ratio(uploaded, downloaded, stop_flag);
                                }
                            }
                        }
                        peer::Message::Piece {
                            index,
                            begin,
                            block,
                        } => {
                            last_piece_data = Instant::now();
                            if let Some(active) = active_pieces.get_mut(&index) {
                                let complete = active
                                    .add_block(begin, &block)
                                    .map_err(|err| format!("block error: {err}"))?;
                                let was_new = {
                                    let mut p = lock_or_recover(pieces);
                                    p.mark_block_complete(index, begin, block.len() as u32)
                                        .map_err(|err| format!("block state error: {err}"))?
                                };
                                if was_new {
                                    SESSION_DOWNLOADED_BYTES
                                        .fetch_add(block.len() as u64, Ordering::SeqCst);
                                    downloaded.fetch_add(block.len() as u64, Ordering::SeqCst);
                                    upload_manager.record_download(peer_tag, block.len() as u64);
                                }
                                peer_rate_sample_bytes =
                                    peer_rate_sample_bytes.saturating_add(block.len());
                                let sample_elapsed = peer_rate_last_at.elapsed().as_secs_f64();
                                if sample_elapsed >= 0.25 {
                                    let instant_rate =
                                        peer_rate_sample_bytes as f64 / sample_elapsed;
                                    peer_rate_bps = if peer_rate_bps <= 0.0 {
                                        instant_rate
                                    } else {
                                        (peer_rate_bps * 0.7) + (instant_rate * 0.3)
                                    };
                                    peer_rate_sample_bytes = 0;
                                    peer_rate_last_at = Instant::now();
                                    pipeline_depth = request_queue_depth_for_rate(peer_rate_bps);
                                }
                                limits.global_down.throttle(block.len());
                                limits.torrent_down.throttle(block.len());
                                if let Some(pos) = pending.iter().position(|entry| {
                                    entry.request.index == index && entry.request.begin == begin
                                }) {
                                    pending.swap_remove(pos);
                                }
                                if complete {
                                    let (expected, piece_start) = {
                                        let p = lock_or_recover(pieces);
                                        let expected = p
                                            .piece_hash(index)
                                            .ok_or_else(|| "missing piece hash".to_string())?
                                            .clone();
                                        let offset = p
                                            .piece_offset(index)
                                            .ok_or_else(|| "missing piece offset".to_string())?;
                                        (expected, offset)
                                    };
                                    if verify_piece_hash(active.data(), &expected) {
                                        let active = persist_active_piece(
                                            &mut active_pieces,
                                            index,
                                            |active| {
                                                let mut s = lock_or_recover(storage);
                                                s.write_at(piece_start, active.data())
                                                    .map_err(|err| format!("write failed: {err}"))
                                            },
                                        )?;
                                        log_debug!(
                                            "piece complete: index={} bytes={} from {addr}",
                                            index,
                                            active.length()
                                        );
                                        let (completed, was_new) = {
                                            let mut p = lock_or_recover(pieces);
                                            let was_new =
                                                p.mark_piece_complete(index).map_err(|err| {
                                                    format!("mark complete failed: {err}")
                                                })?;
                                            p.release_piece(peer_tag, index);
                                            (p.completed_pieces(), was_new)
                                        };
                                        let piece_len = active.length() as u64;
                                        drop(active);
                                        if was_new {
                                            if let Ok(mut log) = completed_log.lock() {
                                                log.push(index);
                                            }
                                            let paused = torrent_paused(paused_flag);
                                            let complete_now = {
                                                let p = lock_or_recover(pieces);
                                                p.is_complete()
                                            };
                                            let status = if paused {
                                                "paused"
                                            } else if complete_now {
                                                "seeding"
                                            } else {
                                                "downloading"
                                            };
                                            update_ui(ui_state, |state| {
                                                apply_piece_completion_ui(
                                                    state,
                                                    torrent_id,
                                                    completed,
                                                    file_spans,
                                                    piece_start,
                                                    piece_len,
                                                    true,
                                                );
                                                update_torrent_entry(
                                                    state,
                                                    torrent_id,
                                                    |torrent| {
                                                        torrent.paused = paused;
                                                        torrent.status = status.to_string();
                                                    },
                                                );
                                                if state.current_id == Some(torrent_id) {
                                                    state.paused = is_paused();
                                                    state.status = status.to_string();
                                                }
                                            });
                                        } else {
                                            let paused = torrent_paused(paused_flag);
                                            let complete_now = {
                                                let p = lock_or_recover(pieces);
                                                p.is_complete()
                                            };
                                            let status = if paused {
                                                "paused"
                                            } else if complete_now {
                                                "seeding"
                                            } else {
                                                "downloading"
                                            };
                                            update_ui(ui_state, |state| {
                                                apply_piece_completion_ui(
                                                    state,
                                                    torrent_id,
                                                    completed,
                                                    file_spans,
                                                    piece_start,
                                                    piece_len,
                                                    false,
                                                );
                                                update_torrent_entry(
                                                    state,
                                                    torrent_id,
                                                    |torrent| {
                                                        torrent.paused = paused;
                                                        torrent.status = status.to_string();
                                                    },
                                                );
                                                if state.current_id == Some(torrent_id) {
                                                    state.paused = is_paused();
                                                    state.status = status.to_string();
                                                }
                                            });
                                        }
                                    } else {
                                        let active = active_pieces
                                            .remove(&index)
                                            .ok_or_else(|| "active piece missing".to_string())?;
                                        log_warn!("piece hash mismatch: index={index}");
                                        ban_peer("piece hash mismatch");
                                        let piece_len = active.length() as u64;
                                        drop(active);
                                        let _ = SESSION_DOWNLOADED_BYTES.fetch_update(
                                            Ordering::SeqCst,
                                            Ordering::SeqCst,
                                            |value| Some(value.saturating_sub(piece_len)),
                                        );
                                        let _ = downloaded.fetch_update(
                                            Ordering::SeqCst,
                                            Ordering::SeqCst,
                                            |value| Some(value.saturating_sub(piece_len)),
                                        );
                                        {
                                            let mut p = lock_or_recover(pieces);
                                            p.reset_piece(index)
                                                .map_err(|err| format!("reset failed: {err}"))?;
                                        }
                                        cancel_pending(&mut stream, &pending)?;
                                        return Err("piece hash mismatch".to_string());
                                    }
                                }
                            }
                        }
                        peer::Message::Cancel { .. } => {
                            // BEP 3: Cancel acknowledged
                        }
                        peer::Message::HaveAll => {
                            // BEP 6: Peer has all pieces
                            let mut p = lock_or_recover(pieces);
                            if bitfield.is_some() {
                                ban_peer("duplicate bitfield state");
                                return Err("duplicate bitfield state".to_string());
                            }
                            let len = p.bitfield_len();
                            let mut all = vec![0xff; len];
                            if let Some(last) = all.last_mut() {
                                let used = p.piece_count() % 8;
                                if used != 0 {
                                    *last = 0xff << (8 - used);
                                }
                            }
                            p.apply_peer_bitfield(&all)
                                .map_err(|err| format!("have-all error: {err}"))?;
                            bitfield = Some(all);
                        }
                        peer::Message::HaveNone => {
                            // BEP 6: Peer has no pieces
                            let mut p = lock_or_recover(pieces);
                            if bitfield.is_some() {
                                ban_peer("duplicate bitfield state");
                                return Err("duplicate bitfield state".to_string());
                            }
                            let len = p.bitfield_len();
                            let none = vec![0; len];
                            p.apply_peer_bitfield(&none)
                                .map_err(|err| format!("have-none error: {err}"))?;
                            bitfield = Some(none);
                        }
                        peer::Message::SuggestPiece(_) => {
                            // BEP 6: Suggestion noted (no special handling)
                        }
                        peer::Message::AllowedFast(_) => {
                            // BEP 6: Allowed fast noted (no special handling yet)
                        }
                        peer::Message::RejectRequest { .. } => {
                            // BEP 6: Peer rejected our request
                        }
                        peer::Message::HashRequest(request) => {
                            let must_serve = immediately_after_served_chunk.is_some_and(
                                |(index, begin, length)| {
                                    let pieces = lock_or_recover(pieces);
                                    v2_hashes.request_covers_chunk(
                                        request, index, begin, length, &pieces,
                                    )
                                },
                            );
                            respond_v2_hash_request(
                                &mut stream,
                                V2HashResponseResources {
                                    store: v2_hashes,
                                    pieces,
                                    storage,
                                    limits,
                                    stop_flag,
                                },
                                &mut hash_request_budget,
                                must_serve,
                                request,
                            )?;
                            last_sent = Instant::now();
                        }
                        peer::Message::Hashes { .. } | peer::Message::HashReject(_) => {
                            return Err("unsolicited BEP 52 hash response".to_string());
                        }
                        _ => {}
                    }
                }
                Ok(None) => {
                    idle += 1;
                }
                Err(err) => return Err(format!("message read failed: {err}")),
            }

            if !pending.is_empty() {
                let now = Instant::now();
                timed_out.clear();
                pending.retain(|entry| {
                    if now.duration_since(entry.sent_at) > REQUEST_TIMEOUT {
                        timed_out.push(entry.request);
                        false
                    } else {
                        true
                    }
                });
                if !timed_out.is_empty() {
                    log_debug!(
                        "{} requests timed out for {addr}, pipeline_depth={pipeline_depth}",
                        timed_out.len()
                    );
                    peer_rate_bps *= 0.75;
                    pipeline_depth = request_queue_depth_for_rate(peer_rate_bps);
                }
                for req in timed_out.drain(..) {
                    peer::write_message(
                        &mut stream,
                        &peer::Message::Cancel {
                            index: req.index,
                            begin: req.begin,
                            length: req.length,
                        },
                    )
                    .map_err(|err| format!("cancel write failed: {err}"))?;
                    {
                        let mut p = lock_or_recover(pieces);
                        p.mark_block_missing(req.index, req.begin)
                            .map_err(|err| format!("block timeout: {err}"))?;
                    }
                }
            }
        }
    })();

    {
        let mut p = lock_or_recover(pieces);
        abandon_inflight(&mut p, &mut pending, &active_pieces);
    }
    active_pieces.clear();

    if let Some(bits) = bitfield {
        let mut p = lock_or_recover(pieces);
        let _ = p.remove_peer_bitfield(&bits);
    }

    set_peer_interest(interested_peers, &mut peer_interested, false);
    upload_manager.unregister(peer_tag);
    PEER_DISCONNECTED.fetch_add(1, Ordering::SeqCst);
    remove_active_peer_session(ui_state, torrent_id, active_peers, geo_cc.as_deref());

    result
}

fn bind_tcp_listeners(port: u16) -> Result<Vec<TcpListener>, String> {
    use std::net::Ipv6Addr;

    let v6_addr = SocketAddr::from((Ipv6Addr::UNSPECIFIED, port));
    match TcpListener::bind(v6_addr) {
        Ok(v6_listener) => {
            let actual_port = v6_listener
                .local_addr()
                .map_err(|err| format!("inspect IPv6 listener: {err}"))?
                .port();
            let v6_only = ipv6_listener_is_v6_only(&v6_listener)
                .map_err(|err| format!("determine IPv6 listener mode: {err}"))?;
            if !v6_only {
                log_info!("listening on [::] (dual-stack) port {actual_port}");
                return Ok(vec![v6_listener]);
            }

            let v4_addr = SocketAddr::from(([0, 0, 0, 0], actual_port));
            let v4_listener = TcpListener::bind(v4_addr).map_err(|err| {
                format!(
                    "IPv6 listener on port {actual_port} is IPv6-only and the IPv4 bind failed: {err}"
                )
            })?;
            log_info!("listening on [::] and 0.0.0.0 port {actual_port}");
            Ok(vec![v6_listener, v4_listener])
        }
        Err(v6_error) => {
            let v4_addr = SocketAddr::from(([0, 0, 0, 0], port));
            let v4_listener = TcpListener::bind(v4_addr).map_err(|v4_error| {
                format!("bind port {port} failed for IPv6 ({v6_error}) and IPv4 ({v4_error})")
            })?;
            let actual_port = v4_listener
                .local_addr()
                .map_err(|err| format!("inspect IPv4 listener: {err}"))?
                .port();
            log_warn!(
                "IPv6 listener unavailable ({v6_error}); listening on IPv4 port {actual_port}"
            );
            Ok(vec![v4_listener])
        }
    }
}

#[cfg(unix)]
fn ipv6_listener_is_v6_only(listener: &TcpListener) -> io::Result<bool> {
    use std::os::fd::AsRawFd;

    let mut value: libc::c_int = 0;
    let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            listener.as_raw_fd(),
            libc::IPPROTO_IPV6,
            libc::IPV6_V6ONLY,
            (&mut value as *mut libc::c_int).cast(),
            &mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if length as usize != std::mem::size_of::<libc::c_int>() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "IPV6_V6ONLY returned an unexpected value size",
        ));
    }
    Ok(value != 0)
}

#[cfg(windows)]
fn ipv6_listener_is_v6_only(listener: &TcpListener) -> io::Result<bool> {
    use std::ffi::c_char;
    use std::os::windows::io::AsRawSocket;

    const IPPROTO_IPV6: i32 = 41;
    const IPV6_V6ONLY: i32 = 27;
    const SOCKET_ERROR: i32 = -1;

    #[link(name = "ws2_32")]
    extern "system" {
        fn getsockopt(
            socket: usize,
            level: i32,
            option_name: i32,
            option_value: *mut c_char,
            option_length: *mut i32,
        ) -> i32;
        fn WSAGetLastError() -> i32;
    }

    let mut value = 0i32;
    let mut length = std::mem::size_of::<i32>() as i32;
    let result = unsafe {
        getsockopt(
            listener.as_raw_socket() as usize,
            IPPROTO_IPV6,
            IPV6_V6ONLY,
            (&mut value as *mut i32).cast(),
            &mut length,
        )
    };
    if result == SOCKET_ERROR {
        return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() }));
    }
    if length as usize != std::mem::size_of::<i32>() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "IPV6_V6ONLY returned an unexpected value size",
        ));
    }
    Ok(value != 0)
}

#[cfg(not(any(unix, windows)))]
fn ipv6_listener_is_v6_only(_listener: &TcpListener) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "IPV6_V6ONLY inspection is unsupported on this platform",
    ))
}

fn connect_peer(addr: SocketAddr, connect_cfg: &ConnectionConfig) -> Result<PeerStream, String> {
    connect_peer_with_timeout(addr, connect_cfg, TRANSFER_PEER_CONNECT_TIMEOUT)
}

fn configure_keepalive(stream: &TcpStream) {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = stream.as_raw_fd();
        const SOL_SOCKET: i32 = {
            #[cfg(target_os = "macos")]
            {
                0xffff
            }
            #[cfg(not(target_os = "macos"))]
            {
                1
            }
        };
        const SO_KEEPALIVE: i32 = {
            #[cfg(target_os = "macos")]
            {
                0x0008
            }
            #[cfg(not(target_os = "macos"))]
            {
                9
            }
        };
        unsafe {
            let enable: i32 = 1;
            let _ = syscall_setsockopt(
                fd,
                SOL_SOCKET,
                SO_KEEPALIVE,
                &enable as *const i32 as *const std::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
    #[cfg(not(unix))]
    let _ = stream;
}

#[cfg(unix)]
extern "C" {
    fn setsockopt(
        socket: i32,
        level: i32,
        option_name: i32,
        option_value: *const std::ffi::c_void,
        option_len: u32,
    ) -> i32;
}

#[cfg(unix)]
unsafe fn syscall_setsockopt(
    socket: i32,
    level: i32,
    option_name: i32,
    option_value: *const std::ffi::c_void,
    option_len: u32,
) -> i32 {
    setsockopt(socket, level, option_name, option_value, option_len)
}

fn connect_tcp_stream(addr: SocketAddr, timeout: Duration) -> Result<TcpStream, String> {
    TcpStream::connect_timeout(&addr, timeout)
        .map_err(|err| format!("connect {addr} failed: {err}"))
}

fn connect_peer_for_metadata(
    addr: SocketAddr,
    connect_cfg: &ConnectionConfig,
) -> Result<PeerStream, String> {
    connect_peer_with_timeout(addr, connect_cfg, METADATA_PEER_CONNECT_TIMEOUT)
}

fn connect_peer_with_timeout(
    addr: SocketAddr,
    connect_cfg: &ConnectionConfig,
    tcp_timeout: Duration,
) -> Result<PeerStream, String> {
    if let Some(filter) = connect_cfg.ip_filter.as_ref() {
        if filter.is_blocked(addr.ip()) {
            return Err("peer blocked".to_string());
        }
    }

    if let Some(proxy_cfg) = connect_cfg.proxy.as_ref() {
        let stream = proxy::connect_through_proxy(proxy_cfg, addr, Duration::from_secs(10))
            .map_err(|err| format!("proxy connect {addr} failed: {err}"))?;
        let _ = stream.set_nodelay(true);
        configure_keepalive(&stream);
        return Ok(PeerStream::tcp(stream));
    }

    let (tx, rx) = mpsc::channel::<Result<PeerStream, String>>();
    let mut attempts = 0usize;

    attempts += 1;
    {
        let result_tx = tx.clone();
        if let Err(err) = thread::Builder::new()
            .name("tcp-connect".to_string())
            .spawn(move || {
                let result = connect_tcp_stream(addr, tcp_timeout).map(|stream| {
                    let _ = stream.set_nodelay(true);
                    configure_keepalive(&stream);
                    PeerStream::tcp(stream)
                });
                let _ = result_tx.send(result);
            })
        {
            let _ = tx.send(Err(format!("TCP connect worker could not start: {err}")));
        }
    }

    if let Some(connector) = connect_cfg.utp.as_ref() {
        attempts += 1;
        let result_tx = tx.clone();
        let connector = connector.clone();
        if let Err(err) = thread::Builder::new()
            .name("utp-connect".to_string())
            .spawn(move || {
                let result = connector.connect(addr).map(PeerStream::utp);
                let _ = result_tx.send(result);
            })
        {
            let _ = tx.send(Err(format!("uTP connect worker could not start: {err}")));
        }
    }

    drop(tx);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_err: Option<String> = None;
    for _ in 0..attempts {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(err)) => last_err = Some(err),
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Err(last_err.unwrap_or_else(|| "connect failed".to_string()))
}

fn outbound_handshake(
    stream: &mut PeerStream,
    info_hash: [u8; 20],
    hybrid_v2_info_hash: Option<[u8; 20]>,
    peer_id: [u8; 20],
    encryption: EncryptionMode,
) -> Result<peer::Handshake, String> {
    match encryption {
        EncryptionMode::Disable => {
            plaintext_handshake(stream, info_hash, hybrid_v2_info_hash, peer_id)
        }
        EncryptionMode::Prefer | EncryptionMode::Require => {
            mse_outbound_handshake(stream, info_hash, hybrid_v2_info_hash, peer_id, encryption)
        }
    }
}

fn plaintext_handshake(
    stream: &mut PeerStream,
    info_hash: [u8; 20],
    hybrid_v2_info_hash: Option<[u8; 20]>,
    peer_id: [u8; 20],
) -> Result<peer::Handshake, String> {
    peer::write_handshake_with_hybrid_upgrade(
        stream,
        info_hash,
        peer_id,
        true,
        hybrid_v2_info_hash.is_some(),
    )
    .map_err(|err| format!("handshake write failed: {err}"))?;
    let handshake =
        peer::read_handshake(stream).map_err(|err| format!("handshake read failed: {err}"))?;
    validate_outbound_handshake_hash(&handshake, info_hash, hybrid_v2_info_hash)?;
    Ok(handshake)
}

fn mse_outbound_handshake(
    stream: &mut PeerStream,
    info_hash: [u8; 20],
    hybrid_v2_info_hash: Option<[u8; 20]>,
    peer_id: [u8; 20],
    encryption: EncryptionMode,
) -> Result<peer::Handshake, String> {
    let allow_plain = encryption != EncryptionMode::Require;
    let handshake_bytes = peer::build_handshake_with_hybrid_upgrade(
        info_hash,
        peer_id,
        true,
        hybrid_v2_info_hash.is_some(),
    );
    let (crypto, cipher, buffered) =
        mse::initiate(stream, info_hash, allow_plain, &handshake_bytes)?;
    if matches!(crypto, mse::CryptoMode::Plaintext) && encryption == EncryptionMode::Require {
        return Err("peer selected plaintext".to_string());
    }
    if let Some(cipher) = cipher {
        stream.enable_encryption(cipher);
    }
    if !buffered.is_empty() {
        stream.prepend_read_buffer(buffered);
    }
    // Peer's BT handshake is the first thing in the encrypted payload stream
    let handshake =
        peer::read_handshake(stream).map_err(|err| format!("mse handshake read: {err}"))?;
    validate_outbound_handshake_hash(&handshake, info_hash, hybrid_v2_info_hash)?;
    Ok(handshake)
}

fn validate_outbound_handshake_hash(
    handshake: &peer::Handshake,
    info_hash: [u8; 20],
    hybrid_v2_info_hash: Option<[u8; 20]>,
) -> Result<(), String> {
    if handshake.info_hash == info_hash || hybrid_v2_info_hash == Some(handshake.info_hash) {
        Ok(())
    } else {
        Err("peer returned wrong info hash".to_string())
    }
}

fn inbound_handshake(
    stream: &mut PeerStream,
    registry: &SessionRegistry,
    encryption: EncryptionMode,
) -> Result<(peer::Handshake, Arc<TorrentContext>), String> {
    let mut first = [0u8; 1];
    stream
        .read_exact(&mut first)
        .map_err(|err| err.to_string())?;
    if first[0] == 19 {
        if encryption == EncryptionMode::Require {
            return Err("plaintext handshake not allowed".to_string());
        }
        let handshake = read_handshake_with_first(stream, first[0])?;
        let context = find_context(registry, handshake.info_hash)?;
        let response_info_hash = inbound_handshake_response_hash(&context, &handshake);
        peer::write_handshake(stream, response_info_hash, context.peer_id, true)
            .map_err(|err| err.to_string())?;
        return Ok((handshake, context));
    }
    if encryption == EncryptionMode::Disable {
        return Err("encryption disabled".to_string());
    }
    let info_hashes = list_info_hashes(registry)?;
    let (crypto, cipher, info_hash, peer_ia, buffered) = mse::accept(
        stream,
        &info_hashes,
        first[0],
        encryption != EncryptionMode::Require,
    )?;
    if let Some(cipher) = cipher {
        stream.enable_encryption(cipher);
    } else if matches!(crypto, mse::CryptoMode::Plaintext) && encryption == EncryptionMode::Require
    {
        return Err("peer selected plaintext".to_string());
    }
    if !buffered.is_empty() {
        stream.prepend_read_buffer(buffered);
    }
    let handshake = peer::parse_handshake(&peer_ia).map_err(|err| err.to_string())?;
    if handshake.info_hash != info_hash {
        return Err("peer returned wrong info hash".to_string());
    }
    let context = find_context(registry, info_hash)?;
    let response_info_hash = inbound_handshake_response_hash(&context, &handshake);
    let response = peer::build_handshake(response_info_hash, context.peer_id, true);
    stream
        .write_all(&response)
        .map_err(|err| format!("mse response write: {err}"))?;
    Ok((handshake, context))
}

fn inbound_handshake_response_hash(
    context: &TorrentContext,
    handshake: &peer::Handshake,
) -> [u8; 20] {
    if handshake.info_hash == context.info_hash && handshake.supports_hybrid_v2_upgrade() {
        context.hybrid_v2_info_hash.unwrap_or(context.info_hash)
    } else {
        // Direct v2 connections (and ordinary v1 connections) must be echoed
        // with the exact swarm identifier the initiator supplied.
        handshake.info_hash
    }
}

fn read_handshake_with_first(
    stream: &mut PeerStream,
    first: u8,
) -> Result<peer::Handshake, String> {
    let mut buf = [0u8; HANDSHAKE_LEN];
    buf[0] = first;
    stream
        .read_exact(&mut buf[1..])
        .map_err(|err| err.to_string())?;
    peer::parse_handshake(&buf).map_err(|err| err.to_string())
}

fn list_info_hashes(registry: &SessionRegistry) -> Result<Vec<[u8; 20]>, String> {
    let guard = registry
        .lock()
        .map_err(|_| "registry lock failed".to_string())?;
    if guard.is_empty() {
        return Err("no torrents available".to_string());
    }
    let mut info_hashes = Vec::with_capacity(guard.len().saturating_mul(2));
    for context in guard.values() {
        info_hashes.push(context.info_hash);
        if let Some(info_hash) = context.hybrid_v2_info_hash {
            info_hashes.push(info_hash);
        }
    }
    info_hashes.sort_unstable();
    info_hashes.dedup();
    Ok(info_hashes)
}

fn find_context(
    registry: &SessionRegistry,
    info_hash: [u8; 20],
) -> Result<Arc<TorrentContext>, String> {
    let guard = registry
        .lock()
        .map_err(|_| "registry lock failed".to_string())?;
    guard
        .get(&info_hash)
        .cloned()
        .or_else(|| {
            guard
                .values()
                .find(|context| context.hybrid_v2_info_hash == Some(info_hash))
                .cloned()
        })
        .ok_or_else(|| "unknown info hash".to_string())
}

fn find_context_by_id(registry: &SessionRegistry, torrent_id: u64) -> Option<Arc<TorrentContext>> {
    let guard = registry.lock().ok()?;
    guard.values().find(|ctx| ctx.id == torrent_id).cloned()
}

fn set_torrent_label(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    session_store: &Arc<SessionStore>,
    torrent_id: u64,
    label: &str,
) -> Result<(), String> {
    let context =
        find_context_by_id(registry, torrent_id).ok_or_else(|| "torrent not found".to_string())?;
    session_store.set_label(context.info_hash, label)?;
    *lock_or_recover(&context.label) = label.to_string();
    update_ui(ui_state, |state| {
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.label = label.to_string();
        });
    });
    Ok(())
}

fn add_torrent_tracker(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    url: &str,
) -> Result<(), String> {
    let context =
        find_context_by_id(registry, torrent_id).ok_or_else(|| "torrent not found".to_string())?;
    if !valid_tracker_url(url) {
        return Err("invalid tracker URL".to_string());
    }
    let mut trackers = lock_or_recover(&context.trackers);
    if trackers.http.len() + trackers.udp.len() >= MAX_TRACKERS_PER_TORRENT
        && !trackers
            .http
            .iter()
            .chain(&trackers.udp)
            .any(|item| item == url)
    {
        return Err(format!(
            "tracker limit reached ({MAX_TRACKERS_PER_TORRENT})"
        ));
    }
    if url.starts_with("udp://") {
        if !trackers.udp.iter().any(|item| item == url) {
            trackers.udp.push(url.to_string());
        }
    } else if (url.starts_with("http://") || url.starts_with("https://"))
        && !trackers.http.iter().any(|item| item == url)
    {
        trackers.http.push(url.to_string());
    }
    let all: Vec<String> = trackers
        .http
        .iter()
        .chain(trackers.udp.iter())
        .cloned()
        .collect();
    drop(trackers);
    update_ui(ui_state, |state| {
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.trackers = all.clone();
        });
    });
    Ok(())
}

fn remove_torrent_tracker(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    url: &str,
) -> Result<(), String> {
    let context =
        find_context_by_id(registry, torrent_id).ok_or_else(|| "torrent not found".to_string())?;
    let mut trackers = lock_or_recover(&context.trackers);
    trackers.http.retain(|u| u != url);
    trackers.udp.retain(|u| u != url);
    let all: Vec<String> = trackers
        .http
        .iter()
        .chain(trackers.udp.iter())
        .cloned()
        .collect();
    drop(trackers);
    update_ui(ui_state, |state| {
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.trackers = all.clone();
        });
    });
    Ok(())
}

fn recheck_torrent(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
) -> Result<(), String> {
    let context =
        find_context_by_id(registry, torrent_id).ok_or_else(|| "torrent not found".to_string())?;
    context
        .rechecking
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "recheck already in progress".to_string())?;
    let pieces_arc = Arc::clone(&context.pieces);
    let storage_arc = Arc::clone(&context.storage);
    let rechecking = Arc::clone(&context.rechecking);
    let save_requested = Arc::clone(&context.resume_save_requested);
    let stop_flag = Arc::clone(&context.stop_requested);
    let base_piece_length = context.base_piece_length;
    let ui_clone = ui_state.clone();
    update_ui(ui_state, |state| {
        state.status = "checking".to_string();
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.status = "checking".to_string();
        });
    });
    let spawn_result = thread::Builder::new()
        .name(format!("recheck-{torrent_id}"))
        .spawn(move || {
            struct RecheckGuard(Arc<AtomicBool>);
            impl Drop for RecheckGuard {
                fn drop(&mut self) {
                    self.0.store(false, Ordering::Release);
                }
            }
            let _guard = RecheckGuard(rechecking);
            let result = {
                // Acquire both locks before resetting so resume saves and peer
                // workers can never observe a transient all-missing bitfield.
                let mut p = lock_or_recover(&pieces_arc);
                let mut s = lock_or_recover(&storage_arc);
                p.reset_verified();
                full_recheck(&mut p, &mut s, base_piece_length, Some(&stop_flag))
                    .map(|_| (p.completed_pieces(), p.piece_count(), p.completed_bytes()))
            };
            match result {
                Ok((completed, total, completed_bytes)) => {
                    save_requested.store(true, Ordering::Release);
                    let status = if stop_flag.load(Ordering::SeqCst) {
                        "stopping"
                    } else if completed == total {
                        "seeding"
                    } else {
                        "downloading"
                    };
                    update_ui(&ui_clone, |state| {
                        if state.current_id == Some(torrent_id) {
                            state.status = status.to_string();
                            state.completed_pieces = completed;
                            state.completed_bytes = completed_bytes;
                        }
                        update_torrent_entry(state, torrent_id, |torrent| {
                            torrent.status = status.to_string();
                            torrent.completed_pieces = completed;
                            torrent.completed_bytes = completed_bytes;
                            torrent.last_error.clear();
                        });
                    });
                    log_info!("recheck complete: {completed}/{total} pieces valid");
                }
                Err(err) => {
                    log_warn!("recheck failed: {err}");
                    update_ui(&ui_clone, |state| {
                        if state.current_id == Some(torrent_id) {
                            state.status = "error".to_string();
                            state.last_error = err.clone();
                        }
                        update_torrent_entry(state, torrent_id, |torrent| {
                            torrent.status = "error".to_string();
                            torrent.last_error = err.clone();
                        });
                    });
                }
            }
        });
    if let Err(err) = spawn_result {
        context.rechecking.store(false, Ordering::Release);
        let message = format!("recheck worker could not start: {err}");
        update_ui(ui_state, |state| {
            if state.current_id == Some(torrent_id) {
                state.status = "error".to_string();
                state.last_error = message.clone();
            }
            update_torrent_entry(state, torrent_id, |torrent| {
                torrent.status = "error".to_string();
                torrent.last_error = message.clone();
            });
        });
        return Err(message);
    }
    Ok(())
}

#[cfg(feature = "verbose")]
fn message_summary(message: &peer::Message) -> String {
    match message {
        peer::Message::KeepAlive => "keep-alive".to_string(),
        peer::Message::Choke => "choke".to_string(),
        peer::Message::Unchoke => "unchoke".to_string(),
        peer::Message::Interested => "interested".to_string(),
        peer::Message::NotInterested => "not-interested".to_string(),
        peer::Message::Have(index) => format!("have index={index}"),
        peer::Message::Bitfield(bits) => format!("bitfield len={}", bits.len()),
        peer::Message::Request {
            index,
            begin,
            length,
        } => {
            format!("request index={index} begin={begin} length={length}")
        }
        peer::Message::Piece {
            index,
            begin,
            block,
        } => {
            format!("piece index={index} begin={begin} block={}", block.len())
        }
        peer::Message::Cancel {
            index,
            begin,
            length,
        } => {
            format!("cancel index={index} begin={begin} length={length}")
        }
        peer::Message::Port(port) => format!("port {port}"),
        peer::Message::Extended { ext_id, payload } => {
            format!("extended id={} len={}", ext_id, payload.len())
        }
        peer::Message::SuggestPiece(index) => format!("suggest-piece index={index}"),
        peer::Message::HaveAll => "have-all".to_string(),
        peer::Message::HaveNone => "have-none".to_string(),
        peer::Message::RejectRequest {
            index,
            begin,
            length,
        } => {
            format!("reject index={index} begin={begin} length={length}")
        }
        peer::Message::AllowedFast(index) => format!("allowed-fast index={index}"),
        peer::Message::HashRequest(request) => format!(
            "hash-request base={} index={} length={} proof={}",
            request.base_layer, request.index, request.length, request.proof_layers
        ),
        peer::Message::Hashes { request, hashes } => format!(
            "hashes base={} index={} length={} proof={} hashes={}",
            request.base_layer,
            request.index,
            request.length,
            request.proof_layers,
            hashes.len()
        ),
        peer::Message::HashReject(request) => format!(
            "hash-reject base={} index={} length={} proof={}",
            request.base_layer, request.index, request.length, request.proof_layers
        ),
    }
}

struct PendingRequest {
    request: piece::BlockRequest,
    sent_at: Instant,
}

fn request_queue_depth_for_rate(rate_bps: f64) -> usize {
    let effective_rate = if rate_bps.is_finite() && rate_bps > 0.0 {
        rate_bps
    } else {
        DEFAULT_PEER_RATE_BPS
    };
    (((effective_rate * REQUEST_QUEUE_TIME_SECS) / piece::BLOCK_LEN as f64).ceil() as usize)
        .clamp(MIN_PIPELINE_DEPTH, MAX_PIPELINE_DEPTH)
}

fn is_retryable_peer_error(err: &str) -> bool {
    let err = err.to_ascii_lowercase();
    if err.contains("wrong info hash") || err.contains("no needed pieces") {
        return false;
    }
    if err.contains("connection refused")
        || err.contains("peer closed connection")
        || err.contains("network is unreachable")
        || err.contains("no route to host")
        || err.contains("self peer")
    {
        return false;
    }
    err.contains("timeout")
        || err.contains("timed out")
        || err.contains("connection reset")
        || err.contains("handshake read failed")
        || err.contains("handshake write failed")
        || err.contains("peer timed out")
        || err.contains("peer snubbed")
        || err == "connect failed"
}

fn record_peer_result(queue: &mut PeerQueue, addr: SocketAddr, result: &Result<(), String>) {
    queue.finish(addr);
    match result {
        Ok(()) => {
            queue.clear_failure(addr);
        }
        Err(err) => {
            if is_retryable_peer_error(err) {
                if let Some(delay) = queue.note_failure(addr) {
                    queue.schedule_retry(addr, delay);
                } else {
                    queue.ban_for(addr, Duration::from_secs(PEER_RETRY_EXHAUSTED_BAN_SECS));
                }
            } else {
                queue.clear_failure(addr);
                queue.ban(addr);
            }
        }
    }
}

fn bitfield_has(bitfield: &[u8], index: usize) -> bool {
    let Some(byte) = bitfield.get(index / 8).copied() else {
        return false;
    };
    let offset = index % 8;
    let mask = 0x80 >> offset;
    (byte & mask) != 0
}

fn set_bit(bitfield: &mut [u8], index: usize) -> Result<(), String> {
    if index >= bitfield.len() * 8 {
        return Err("bitfield index out of range".to_string());
    }
    let byte = index / 8;
    let offset = index % 8;
    let mask = 0x80 >> offset;
    bitfield[byte] |= mask;
    Ok(())
}

fn build_bitfield(pieces: &piece::PieceManager) -> Vec<u8> {
    let mut bitfield = vec![0u8; pieces.bitfield_len()];
    for idx in 0..pieces.piece_count() {
        if pieces.is_piece_complete(idx as u32) {
            let byte = idx / 8;
            let offset = idx % 8;
            bitfield[byte] |= 0x80 >> offset;
        }
    }
    bitfield
}

fn build_ut_pex_payload(peers: &[SocketAddr], dropped: &[SocketAddr]) -> Vec<u8> {
    let mut v4 = Vec::new();
    let mut v4_flags = Vec::new();
    let mut v6 = Vec::new();
    let mut v6_flags = Vec::new();
    let mut drop4 = Vec::new();
    let mut drop6 = Vec::new();
    // BEP 11 flags: 0x01=encryption, 0x02=seed, 0x04=uTP, 0x10=outgoing
    let flags: u8 = 0x10; // outgoing connection
    for peer in peers {
        match peer.ip() {
            std::net::IpAddr::V4(ip) => {
                v4.extend_from_slice(&ip.octets());
                v4.extend_from_slice(&peer.port().to_be_bytes());
                v4_flags.push(flags);
            }
            std::net::IpAddr::V6(ip) => {
                v6.extend_from_slice(&ip.octets());
                v6.extend_from_slice(&peer.port().to_be_bytes());
                v6_flags.push(flags);
            }
        }
    }
    for peer in dropped {
        match peer.ip() {
            std::net::IpAddr::V4(ip) => {
                drop4.extend_from_slice(&ip.octets());
                drop4.extend_from_slice(&peer.port().to_be_bytes());
            }
            std::net::IpAddr::V6(ip) => {
                drop6.extend_from_slice(&ip.octets());
                drop6.extend_from_slice(&peer.port().to_be_bytes());
            }
        }
    }
    let mut dict = Vec::new();
    if !v4.is_empty() {
        dict.push((b"added".to_vec(), Value::Bytes(v4)));
        dict.push((b"added.f".to_vec(), Value::Bytes(v4_flags)));
    }
    if !v6.is_empty() {
        dict.push((b"added6".to_vec(), Value::Bytes(v6)));
        dict.push((b"added6.f".to_vec(), Value::Bytes(v6_flags)));
    }
    if !drop4.is_empty() {
        dict.push((b"dropped".to_vec(), Value::Bytes(drop4)));
    }
    if !drop6.is_empty() {
        dict.push((b"dropped6".to_vec(), Value::Bytes(drop6)));
    }
    bencode::encode(&Value::Dict(dict))
}

fn parse_ut_pex(payload: &[u8]) -> Result<Vec<SocketAddr>, String> {
    let (dict, _) = parse_bencode_dict(payload)?;
    let mut peers = Vec::new();
    if let Some(Value::Bytes(bytes)) = dict_get(&dict, b"added") {
        peers.extend(decode_compact_peers(bytes));
    }
    if let Some(Value::Bytes(bytes)) = dict_get(&dict, b"added6") {
        peers.extend(decode_compact_peers6(bytes));
    }
    Ok(peers)
}

fn decode_compact_peers(bytes: &[u8]) -> Vec<SocketAddr> {
    let mut peers = Vec::new();
    if !bytes.len().is_multiple_of(6) {
        return peers;
    }
    for chunk in bytes.chunks_exact(6) {
        let ip = std::net::Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        peers.push(SocketAddr::new(ip.into(), port));
    }
    peers
}

fn decode_compact_peers6(bytes: &[u8]) -> Vec<SocketAddr> {
    let mut peers = Vec::new();
    if !bytes.len().is_multiple_of(18) {
        return peers;
    }
    for chunk in bytes.chunks_exact(18) {
        let ip = std::net::Ipv6Addr::from([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            chunk[8], chunk[9], chunk[10], chunk[11], chunk[12], chunk[13], chunk[14], chunk[15],
        ]);
        let port = u16::from_be_bytes([chunk[16], chunk[17]]);
        peers.push(SocketAddr::new(ip.into(), port));
    }
    peers
}

fn send_completed_updates<W: Write>(
    stream: &mut W,
    completed_log: &Arc<Mutex<Vec<u32>>>,
    cursor: &mut usize,
) -> Result<(), String> {
    let updates = {
        let log = lock_or_recover(completed_log);
        if *cursor >= log.len() {
            return Ok(());
        }
        let slice = log[*cursor..].to_vec();
        *cursor = log.len();
        slice
    };

    for index in updates {
        peer::write_message(stream, &peer::Message::Have(index))
            .map_err(|err| format!("have write failed: {err}"))?;
    }
    Ok(())
}

fn register_session(
    registry: &SessionRegistry,
    context: Arc<TorrentContext>,
) -> Result<(), String> {
    let mut guard = registry
        .lock()
        .map_err(|_| "torrent registry lock failed".to_string())?;
    if let Some(existing) = guard.get(&context.info_hash) {
        if existing.id != context.id {
            return Err("torrent is already active".to_string());
        }
        return Ok(());
    }
    guard.insert(context.info_hash, context);
    Ok(())
}

fn unregister_session(registry: &SessionRegistry, info_hash: [u8; 20], torrent_id: u64) {
    if let Ok(mut guard) = registry.lock() {
        let should_remove = guard
            .get(&info_hash)
            .map(|context| context.id == torrent_id)
            .unwrap_or(false);
        if should_remove {
            guard.remove(&info_hash);
        }
    }
}

fn start_utp_listener(
    listener: utp::UtpListener,
    registry: SessionRegistry,
    inbound: InboundConfig,
) -> Result<thread::JoinHandle<()>, String> {
    thread::Builder::new()
        .name("utp-listener".to_string())
        .spawn(move || loop {
            if shutdown_requested() {
                break;
            }
            if let Some(stream) = listener.try_accept() {
                if let Some(slot_guard) = inbound.try_acquire_handler_slot() {
                    let registry = Arc::clone(&registry);
                    let inbound = inbound.clone();
                    if let Err(err) = thread::Builder::new()
                        .name("inbound-utp-peer".to_string())
                        .stack_size(PEER_THREAD_STACK)
                        .spawn(move || {
                            let _slot_guard = slot_guard;
                            handle_incoming_peer(PeerStream::utp(stream), registry, inbound);
                        })
                    {
                        log_warn!("inbound uTP peer worker could not start: {err}");
                    }
                } else {
                    log_debug!("dropping inbound uTP peer: handler capacity reached");
                }
            } else {
                sleep_with_shutdown(Duration::from_millis(20));
            }
        })
        .map_err(|err| format!("listener worker could not start: {err}"))
}

fn start_inbound_listener(
    port: u16,
    registry: SessionRegistry,
    inbound: InboundConfig,
) -> Result<thread::JoinHandle<()>, String> {
    let listeners = bind_tcp_listeners(port)?;
    for listener in &listeners {
        listener
            .set_nonblocking(true)
            .map_err(|err| format!("set listener nonblocking: {err}"))?;
    }
    thread::Builder::new()
        .name("tcp-listener".to_string())
        .spawn(move || loop {
            if shutdown_requested() {
                break;
            }
            let mut accepted_connection = false;
            let mut accept_failed = false;
            for listener in &listeners {
                match listener.accept() {
                    Ok((stream, _)) => {
                        accepted_connection = true;
                        if let Some(slot_guard) = inbound.try_acquire_handler_slot() {
                            let registry = Arc::clone(&registry);
                            let inbound = inbound.clone();
                            if let Err(err) = thread::Builder::new()
                                .name("inbound-tcp-peer".to_string())
                                .stack_size(PEER_THREAD_STACK)
                                .spawn(move || {
                                    let _slot_guard = slot_guard;
                                    handle_incoming_peer(
                                        PeerStream::tcp(stream),
                                        registry,
                                        inbound,
                                    );
                                })
                            {
                                log_warn!("inbound TCP peer worker could not start: {err}");
                            }
                        } else {
                            log_debug!("dropping inbound TCP peer: handler capacity reached");
                        }
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                    Err(err) => {
                        accept_failed = true;
                        log_warn!("inbound accept failed: {err}");
                    }
                }
            }
            if accept_failed {
                sleep_with_shutdown(Duration::from_millis(100));
            } else if !accepted_connection {
                sleep_with_shutdown(Duration::from_millis(20));
            }
        })
        .map_err(|err| format!("listener worker could not start: {err}"))
}

fn handle_incoming_peer(mut stream: PeerStream, registry: SessionRegistry, inbound: InboundConfig) {
    let addr = match stream.peer_addr() {
        Some(addr) => addr,
        None => return,
    };
    if let Some(filter) = inbound.ip_filter.as_ref() {
        if filter.is_blocked(addr.ip()) {
            log_debug!("peer {addr} blocked by filter");
            return;
        }
    }
    if let Err(err) = stream.set_read_timeout(Some(Duration::from_millis(500))) {
        let _ = err;
        log_debug!("peer {addr} timeout failed: {err}");
        return;
    }
    if let Err(err) = stream.set_write_timeout(Some(Duration::from_secs(10))) {
        let _ = err;
        log_debug!("peer {addr} timeout failed: {err}");
        return;
    }
    if let Some(tcp) = stream.tcp_stream() {
        configure_keepalive(tcp);
    }

    let (handshake, context) = match inbound_handshake(&mut stream, &registry, inbound.encryption) {
        Ok(result) => result,
        Err(err) => {
            let _ = err;
            log_debug!("peer {addr} handshake failed: {err}");
            return;
        }
    };
    if is_self_peer_id(&context.peer_id, &handshake.peer_id) {
        log_debug!("dropping self peer {addr}");
        return;
    }
    let peer_tag = context.peer_tags.fetch_add(1, Ordering::SeqCst);
    let _cancellation = PeerCancellationGuard::new(&context.peer_cancellations, peer_tag, &stream);
    if torrent_stop_requested(&context.stop_requested) {
        return;
    }
    context.upload_manager.register(peer_tag);
    PEER_CONNECTED.fetch_add(1, Ordering::SeqCst);
    context.active_peers.fetch_add(1, Ordering::SeqCst);

    let mut reader = peer::MessageReader::new();
    let mut peer_interested = false;
    let mut am_choking = true;
    let mut last_sent = Instant::now();
    let mut idle: u32 = 0;
    let mut completed_cursor = {
        let log = lock_or_recover(&context.completed_log);
        log.len()
    };
    let mut hash_request_budget = HashRequestBudget::new();
    let mut last_served_chunk: Option<(u32, u32, u32)> = None;

    let (local_bitfield, have_pieces) = {
        let p = lock_or_recover(&context.pieces);
        let bits = build_bitfield(&p);
        (bits, p.completed_pieces() > 0)
    };
    let inbound_super_seed = have_pieces && SUPER_SEED.load(Ordering::SeqCst);
    if inbound_super_seed {
        // BEP 16: send single HAVE instead of full bitfield
        let piece_count = {
            let p = lock_or_recover(&context.pieces);
            p.piece_count()
        };
        if piece_count > 0 {
            let mut sv = std::process::id() as u64 ^ peer_tag;
            sv ^= sv << 13;
            sv ^= sv >> 7;
            let idx = (sv % piece_count as u64) as u32;
            let _ = peer::write_message(&mut stream, &peer::Message::Have(idx));
        }
    } else {
        // BEP 3: always send bitfield as first message after handshake
        let _ = peer::write_message(&mut stream, &peer::Message::Bitfield(local_bitfield));
    }

    // Declare interest status to match outbound handler behavior.
    if have_pieces {
        let _ = peer::write_message(&mut stream, &peer::Message::NotInterested);
    } else {
        let _ = peer::write_message(&mut stream, &peer::Message::Interested);
    }
    let _ = stream.flush();

    loop {
        if torrent_stop_requested(&context.stop_requested) {
            break;
        }
        if idle > MAX_IDLE_TICKS_SEED {
            break;
        }

        let paused = torrent_paused(&context.paused);
        if paused && !am_choking {
            if peer::write_message(&mut stream, &peer::Message::Choke).is_ok() {
                am_choking = true;
                last_sent = Instant::now();
            }
        } else if !paused {
            let should_unchoke = context.upload_manager.should_unchoke(peer_tag);
            if should_unchoke && am_choking {
                if peer::write_message(&mut stream, &peer::Message::Unchoke).is_ok() {
                    am_choking = false;
                    last_sent = Instant::now();
                }
            } else if !should_unchoke
                && !am_choking
                && peer::write_message(&mut stream, &peer::Message::Choke).is_ok()
            {
                am_choking = true;
                last_sent = Instant::now();
            }
        }

        if last_sent.elapsed() >= KEEPALIVE_INTERVAL
            && peer::write_message(&mut stream, &peer::Message::KeepAlive).is_ok()
        {
            last_sent = Instant::now();
            idle = 0;
        }

        if send_completed_updates(&mut stream, &context.completed_log, &mut completed_cursor)
            .is_err()
        {
            break;
        }

        match reader.read_message(&mut stream) {
            Ok(Some(message)) => {
                idle = 0;
                let immediately_after_served_chunk = last_served_chunk.take();
                match message {
                    peer::Message::Interested => {
                        set_peer_interest(&context.interested_peers, &mut peer_interested, true);
                        context.upload_manager.set_interested(peer_tag, true);
                        // Immediately unchoke if eligible
                        if am_choking
                            && !paused
                            && context.upload_manager.should_unchoke(peer_tag)
                            && peer::write_message(&mut stream, &peer::Message::Unchoke).is_ok()
                        {
                            am_choking = false;
                            last_sent = Instant::now();
                        }
                    }
                    peer::Message::NotInterested => {
                        set_peer_interest(&context.interested_peers, &mut peer_interested, false);
                        context.upload_manager.set_interested(peer_tag, false);
                    }
                    peer::Message::Request {
                        index,
                        begin,
                        length,
                    } => {
                        if !am_choking && peer_interested {
                            if let Err(err) = handle_upload_request(
                                &mut stream,
                                &context.pieces,
                                &context.storage,
                                index,
                                begin,
                                length,
                                &context.limits,
                                &context.uploaded,
                                &context.upload_requests_served,
                                &context.upload_manager,
                                peer_tag,
                            ) {
                                let _ = err;
                                log_debug!("inbound upload rejected: {err}");
                            } else {
                                last_served_chunk = Some((index, begin, length));
                                last_sent = Instant::now();
                                check_seed_ratio(
                                    &context.uploaded,
                                    &context.downloaded,
                                    &context.stop_requested,
                                );
                            }
                        }
                    }
                    peer::Message::Cancel { .. } => {
                        // BEP 3: Cancel acknowledged (we don't queue uploads,
                        // so there's nothing to cancel, but we must not ignore it)
                    }
                    peer::Message::HaveAll => {
                        // BEP 6: Peer has all pieces - treat as full bitfield
                    }
                    peer::Message::HaveNone => {
                        // BEP 6: Peer has no pieces
                    }
                    peer::Message::HashRequest(request) => {
                        let must_serve =
                            immediately_after_served_chunk.is_some_and(|(index, begin, length)| {
                                let pieces = lock_or_recover(&context.pieces);
                                context
                                    .v2_hashes
                                    .request_covers_chunk(request, index, begin, length, &pieces)
                            });
                        if respond_v2_hash_request(
                            &mut stream,
                            V2HashResponseResources {
                                store: &context.v2_hashes,
                                pieces: &context.pieces,
                                storage: &context.storage,
                                limits: &context.limits,
                                stop_flag: &context.stop_requested,
                            },
                            &mut hash_request_budget,
                            must_serve,
                            request,
                        )
                        .is_err()
                        {
                            break;
                        }
                        last_sent = Instant::now();
                    }
                    peer::Message::Hashes { .. } | peer::Message::HashReject(_) => {
                        // We do not originate BEP 52 hash requests, so a
                        // response cannot correlate with any pending request.
                        break;
                    }
                    _ => {}
                }
            }
            Ok(None) => {
                idle += 1;
            }
            Err(_) => break,
        }
    }

    set_peer_interest(&context.interested_peers, &mut peer_interested, false);
    context.upload_manager.unregister(peer_tag);
    PEER_DISCONNECTED.fetch_add(1, Ordering::SeqCst);
    let _ = context
        .active_peers
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
            Some(value.saturating_sub(1))
        });
}

#[allow(clippy::too_many_arguments)]
fn handle_upload_request<W: Write>(
    stream: &mut W,
    pieces: &Arc<Mutex<piece::PieceManager>>,
    storage: &Arc<Mutex<storage::Storage>>,
    index: u32,
    begin: u32,
    length: u32,
    limits: &TransferLimits,
    uploaded: &Arc<AtomicU64>,
    upload_requests_served: &Arc<AtomicU64>,
    upload_manager: &Arc<UploadManager>,
    peer_tag: u64,
) -> Result<(), String> {
    if length == 0 || length > MAX_UPLOAD_BLOCK_LEN {
        return Err("invalid request length".to_string());
    }

    let (piece_len, piece_start) = {
        let p = lock_or_recover(pieces);
        if !p.is_piece_complete(index) {
            return Err("requested piece not available".to_string());
        }
        let length = p
            .piece_length(index)
            .ok_or_else(|| "missing piece length".to_string())?;
        let offset = p
            .piece_offset(index)
            .ok_or_else(|| "missing piece offset".to_string())?;
        (length, offset)
    };

    let end = begin
        .checked_add(length)
        .ok_or_else(|| "invalid request offset".to_string())?;
    if end > piece_len {
        return Err("request out of bounds".to_string());
    }

    let offset = piece_start.saturating_add(begin as u64);
    let mut buf = vec![0u8; length as usize];
    {
        let mut s = lock_or_recover(storage);
        s.read_at(offset, &mut buf)
            .map_err(|err| format!("read failed: {err}"))?;
    }

    limits.global_up.throttle(length as usize);
    limits.torrent_up.throttle(length as usize);
    peer::write_message(
        stream,
        &peer::Message::Piece {
            index,
            begin,
            block: buf,
        },
    )
    .map_err(|err| format!("piece write failed: {err}"))?;
    uploaded.fetch_add(length as u64, Ordering::SeqCst);
    SESSION_UPLOADED_BYTES.fetch_add(length as u64, Ordering::SeqCst);
    upload_requests_served.fetch_add(1, Ordering::SeqCst);
    upload_manager.record_upload(peer_tag, length as u64);
    Ok(())
}

fn check_seed_ratio(
    uploaded: &Arc<AtomicU64>,
    downloaded: &Arc<AtomicU64>,
    stop_flag: &Arc<AtomicBool>,
) {
    let ratio_bits = SEED_RATIO_BITS.load(Ordering::SeqCst);
    if ratio_bits == 0 {
        return;
    }
    let ratio = f64::from_bits(ratio_bits);
    let up = uploaded.load(Ordering::SeqCst) as f64;
    let down = downloaded.load(Ordering::SeqCst).max(1) as f64;
    if up / down >= ratio {
        stop_flag.store(true, Ordering::SeqCst);
        log_info!("seed ratio {:.2} reached, stopping torrent", ratio);
    }
}

fn check_ratio_group(
    ratio_group_name: &Option<String>,
    uploaded: &Arc<AtomicU64>,
    downloaded: &Arc<AtomicU64>,
    stop_flag: &Arc<AtomicBool>,
    paused_flag: &Arc<AtomicBool>,
) {
    let group_name = match ratio_group_name {
        Some(name) => name,
        None => return,
    };
    let groups = match RATIO_GROUPS.get() {
        Some(g) => g,
        None => return,
    };
    let guard = match groups.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let group = match guard.iter().find(|g| g.name == *group_name) {
        Some(g) => g,
        None => return,
    };
    let up = uploaded.load(Ordering::SeqCst) as f64;
    let down = downloaded.load(Ordering::SeqCst).max(1) as f64;
    if up / down >= group.ratio {
        match group.action.as_str() {
            "stop" => {
                stop_flag.store(true, Ordering::SeqCst);
                log_info!(
                    "ratio group '{}' ratio {:.2} reached, stopping",
                    group_name,
                    group.ratio
                );
            }
            "pause" => {
                paused_flag.store(true, Ordering::SeqCst);
                log_info!(
                    "ratio group '{}' ratio {:.2} reached, pausing",
                    group_name,
                    group.ratio
                );
            }
            _ => {
                log_info!(
                    "ratio group '{}' ratio {:.2} reached (no action)",
                    group_name,
                    group.ratio
                );
            }
        }
    }
}

fn execute_schedule_command(
    command: &str,
    global_down: &Arc<RateLimiter>,
    global_up: &Arc<RateLimiter>,
    registry: &SessionRegistry,
) {
    if let Some(rest) = command.strip_prefix("throttle_down:") {
        if let Ok(bps) = rest.parse::<u64>() {
            global_down.set_limit_bps(bps);
            log_info!("schedule: download throttle set to {bps} B/s");
        }
    } else if let Some(rest) = command.strip_prefix("throttle_up:") {
        if let Ok(bps) = rest.parse::<u64>() {
            global_up.set_limit_bps(bps);
            log_info!("schedule: upload throttle set to {bps} B/s");
        }
    } else if command == "pause_all" {
        PAUSED.store(true, Ordering::SeqCst);
        log_info!("schedule: paused all torrents");
    } else if command == "resume_all" {
        PAUSED.store(false, Ordering::SeqCst);
        log_info!("schedule: resumed all torrents");
    } else if command == "stop_ratio_reached" {
        if let Ok(guard) = registry.lock() {
            for ctx in guard.values() {
                check_seed_ratio(&ctx.uploaded, &ctx.downloaded, &ctx.stop_requested);
            }
        }
        log_info!("schedule: checked seed ratios");
    } else {
        let command = tracker::sanitize_failure_reason(command.as_bytes());
        log_warn!("schedule: unknown command '{command}'");
    }
}

fn rss_add_feed(url: &str, interval: u64, download_dir: &Path) -> Result<(), String> {
    let lock = RSS_STATE.get().ok_or("rss not initialized")?;
    let mut state = lock.lock().map_err(|_| "rss lock failed".to_string())?;
    if state.feeds.iter().any(|f| f.url == url) {
        return Err("feed already exists".to_string());
    }
    if state.feeds.len() >= rss::MAX_RSS_FEEDS || url.len() > rss::MAX_RSS_TEXT_BYTES {
        return Err("RSS feed limit exceeded".to_string());
    }
    let previous = state.feeds.clone();
    state.feeds.push(rss::RssFeed {
        url: url.to_string(),
        title: String::new(),
        items: Vec::new(),
        last_poll: 0,
        poll_interval_secs: if interval > 0 { interval } else { 900 },
    });
    let rss_path = download_dir.join(".rustorrent").join("rss.benc");
    if let Err(err) = rss::save_rss_state(&rss_path, &state) {
        state.feeds = previous;
        return Err(err);
    }
    log_info!("rss added feed: {}", safe_network_url_label(url));
    Ok(())
}

fn rss_remove_feed(url: &str, download_dir: &Path) -> Result<(), String> {
    let lock = RSS_STATE.get().ok_or("rss not initialized")?;
    let mut state = lock.lock().map_err(|_| "rss lock failed".to_string())?;
    let previous = state.feeds.clone();
    let before = state.feeds.len();
    state.feeds.retain(|f| f.url != url);
    if state.feeds.len() == before {
        return Err("feed not found".to_string());
    }
    let rss_path = download_dir.join(".rustorrent").join("rss.benc");
    if let Err(err) = rss::save_rss_state(&rss_path, &state) {
        state.feeds = previous;
        return Err(err);
    }
    log_info!("rss removed feed: {}", safe_network_url_label(url));
    Ok(())
}

fn rss_add_rule(
    name: &str,
    feed_url: &str,
    pattern: &str,
    download_dir: &Path,
) -> Result<(), String> {
    let lock = RSS_STATE.get().ok_or("rss not initialized")?;
    let mut state = lock.lock().map_err(|_| "rss lock failed".to_string())?;
    if state.rules.len() >= rss::MAX_RSS_RULES
        || name.len() > rss::MAX_RSS_TEXT_BYTES
        || feed_url.len() > rss::MAX_RSS_TEXT_BYTES
        || pattern.len() > rss::MAX_RSS_PATTERN_BYTES
    {
        return Err("RSS rule limit exceeded".to_string());
    }
    let previous = state.rules.clone();
    state.rules.push(rss::RssRule {
        name: name.to_string(),
        feed_url: feed_url.to_string(),
        pattern: pattern.to_string(),
    });
    let rss_path = download_dir.join(".rustorrent").join("rss.benc");
    if let Err(err) = rss::save_rss_state(&rss_path, &state) {
        state.rules = previous;
        return Err(err);
    }
    log_info!(
        "rss added rule: {} (pattern: {})",
        tracker::sanitize_failure_reason(name.as_bytes()),
        tracker::sanitize_failure_reason(pattern.as_bytes())
    );
    Ok(())
}

fn rss_remove_rule(name: &str, download_dir: &Path) -> Result<(), String> {
    let lock = RSS_STATE.get().ok_or("rss not initialized")?;
    let mut state = lock.lock().map_err(|_| "rss lock failed".to_string())?;
    let previous = state.rules.clone();
    let before = state.rules.len();
    state.rules.retain(|r| r.name != name);
    if state.rules.len() == before {
        return Err("rule not found".to_string());
    }
    let rss_path = download_dir.join(".rustorrent").join("rss.benc");
    if let Err(err) = rss::save_rss_state(&rss_path, &state) {
        state.rules = previous;
        return Err(err);
    }
    log_info!(
        "rss removed rule: {}",
        tracker::sanitize_failure_reason(name.as_bytes())
    );
    Ok(())
}

fn schedule_rss_polls(
    args: &Args,
    poll_tx: &mpsc::Sender<RssPollResult>,
    inflight: &mut HashSet<String>,
) {
    if args.proxy.is_some() {
        return;
    }
    let rss_lock = match RSS_STATE.get() {
        Some(lock) => lock,
        None => return,
    };
    let mut state = match rss_lock.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let now = rss::now_secs();
    let mut due_urls = Vec::new();
    for feed in &mut state.feeds {
        if inflight.len() >= MAX_RSS_POLL_WORKERS {
            break;
        }
        let interval = feed.poll_interval_secs.max(1);
        if now < feed.last_poll.saturating_add(interval) || inflight.contains(&feed.url) {
            continue;
        }
        feed.last_poll = now;
        inflight.insert(feed.url.clone());
        due_urls.push(feed.url.clone());
    }
    if !due_urls.is_empty() {
        let rss_path = args.download_dir.join(".rustorrent").join("rss.benc");
        let _ = rss::save_rss_state(&rss_path, &state);
    }
    drop(state);

    for url in due_urls {
        let tx = poll_tx.clone();
        let failed_url = url.clone();
        if let Err(err) = thread::Builder::new()
            .name("rss-poll".to_string())
            .spawn(move || {
                let parsed = match http::get_public(&url, 2 * 1024 * 1024) {
                    Ok(bytes) => rss::parse_feed(&bytes),
                    Err(err) => Err(err.to_string()),
                };
                let _ = tx.send(RssPollResult { url, parsed });
            })
        {
            inflight.remove(&failed_url);
            if let Ok(mut state) = rss_lock.lock() {
                if let Some(feed) = state.feeds.iter_mut().find(|feed| feed.url == failed_url) {
                    feed.last_poll = 0;
                }
            }
            log_warn!("RSS poll worker could not start: {err}");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn drain_rss_poll_results(
    args: &Args,
    poll_rx: &mpsc::Receiver<RssPollResult>,
    download_tx: &mpsc::Sender<RssDownloadResult>,
    queue: &mut VecDeque<TorrentRequest>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    next_id: &mut u64,
    poll_inflight: &mut HashSet<String>,
    download_inflight: &mut HashSet<String>,
    registry: &SessionRegistry,
    session_store: &SessionStore,
    in_flight: &InFlightTorrents,
) {
    while let Ok(result) = poll_rx.try_recv() {
        poll_inflight.remove(&result.url);

        let rss_lock = match RSS_STATE.get() {
            Some(lock) => lock,
            None => continue,
        };
        let mut state = match rss_lock.lock() {
            Ok(guard) => guard,
            Err(_) => continue,
        };
        let feed_idx = match state.feeds.iter().position(|feed| feed.url == result.url) {
            Some(idx) => idx,
            None => continue,
        };

        let mut download_jobs: Vec<(String, String, String)> = Vec::new();
        let mut scheduled_keys = HashSet::new();
        let mut should_save = false;
        match result.parsed {
            Ok((title, items)) => {
                if !title.is_empty() && state.feeds[feed_idx].title.is_empty() {
                    state.feeds[feed_idx].title = title;
                }
                let matches =
                    rss::match_rules(&items, &state.rules, &state.seen_guids, &result.url);
                let download_slots =
                    MAX_RSS_DOWNLOAD_WORKERS.saturating_sub(download_inflight.len());
                let mut threaded_downloads = 0usize;
                for (item, rule) in matches.iter().take(MAX_RSS_MATCHES_PER_POLL) {
                    let seen_key = rss::seen_key(&result.url, &item.guid);
                    if download_inflight.contains(&seen_key)
                        || !scheduled_keys.insert(seen_key.clone())
                    {
                        continue;
                    }
                    if !rss::is_magnet_link(&item.link) {
                        if threaded_downloads >= download_slots {
                            continue;
                        }
                        threaded_downloads += 1;
                    }
                    log_info!(
                        "rss match: '{}' (rule: '{}')",
                        tracker::sanitize_failure_reason(item.title.as_bytes()),
                        tracker::sanitize_failure_reason(rule.name.as_bytes())
                    );
                    download_jobs.push((seen_key, item.link.clone(), item.title.clone()));
                }
                state.feeds[feed_idx].items = items;
                should_save = true;
            }
            Err(err) => {
                log_warn!("rss poll {}: {err}", safe_network_url_label(&result.url));
            }
        }
        if should_save {
            let rss_path = args.download_dir.join(".rustorrent").join("rss.benc");
            let _ = rss::save_rss_state(&rss_path, &state);
        }
        drop(state);

        for (seen_key, url, title) in download_jobs {
            if rss::is_magnet_link(&url) {
                let request = TorrentRequest {
                    id: *next_id,
                    source: TorrentSource::Magnet(url.clone()),
                    download_dir: args.download_dir.clone(),
                    preallocate: args.preallocate,
                    initial_label: String::new(),
                };
                let queued = enqueue_request_if_new(
                    registry,
                    queue,
                    session_store,
                    in_flight,
                    ui_state,
                    request,
                    Some(format!("rss: {title}")),
                );
                if queued {
                    *next_id = next_id.saturating_add(1);
                }
                if let Err(err) = record_rss_seen(&args.download_dir, seen_key) {
                    log_warn!("rss save after queueing magnet: {err}");
                }
                if queued {
                    log_info!(
                        "rss queued magnet: {}",
                        tracker::sanitize_failure_reason(title.as_bytes())
                    );
                }
                continue;
            }
            if !download_inflight.insert(seen_key.clone()) {
                continue;
            }
            let tx = download_tx.clone();
            let failed_key = seen_key.clone();
            if let Err(err) = thread::Builder::new()
                .name("rss-download".to_string())
                .spawn(move || {
                    let data =
                        http::get_public(&url, MAX_TORRENT_BYTES).map_err(|err| err.to_string());
                    let _ = tx.send(RssDownloadResult {
                        seen_key,
                        url,
                        title,
                        data,
                    });
                })
            {
                download_inflight.remove(&failed_key);
                log_warn!("RSS download worker could not start: {err}");
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn drain_rss_download_results(
    args: &Args,
    download_rx: &mpsc::Receiver<RssDownloadResult>,
    queue: &mut VecDeque<TorrentRequest>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    next_id: &mut u64,
    download_inflight: &mut HashSet<String>,
    registry: &SessionRegistry,
    session_store: &SessionStore,
    in_flight: &InFlightTorrents,
) {
    while let Ok(result) = download_rx.try_recv() {
        download_inflight.remove(&result.seen_key);
        match result.data {
            Ok(data) => {
                if let Err(err) = torrent::parse_torrent(&data) {
                    log_warn!(
                        "rss download {} returned invalid torrent metadata: {err}",
                        safe_network_url_label(&result.url)
                    );
                    continue;
                }
                let request = TorrentRequest {
                    id: *next_id,
                    source: TorrentSource::Bytes(data),
                    download_dir: args.download_dir.clone(),
                    preallocate: args.preallocate,
                    initial_label: String::new(),
                };
                let queued = enqueue_request_if_new(
                    registry,
                    queue,
                    session_store,
                    in_flight,
                    ui_state,
                    request,
                    Some(format!("rss: {}", result.title)),
                );
                if queued {
                    *next_id = next_id.saturating_add(1);
                }
                if let Err(err) = record_rss_seen(&args.download_dir, result.seen_key) {
                    log_warn!("rss save after queueing torrent: {err}");
                }
                if queued {
                    log_info!(
                        "rss queued torrent: {}",
                        tracker::sanitize_failure_reason(result.title.as_bytes())
                    );
                }
            }
            Err(err) => {
                log_warn!(
                    "rss download {}: {err}",
                    safe_network_url_label(&result.url)
                );
            }
        }
    }
}

fn record_rss_seen(download_dir: &Path, seen_key: String) -> Result<(), String> {
    let lock = RSS_STATE.get().ok_or("rss not initialized")?;
    let mut state = lock.lock().map_err(|_| "rss lock failed".to_string())?;
    let previous = state.seen_guids.clone();
    rss::remember_seen(&mut state.seen_guids, seen_key);
    let rss_path = download_dir.join(".rustorrent").join("rss.benc");
    if let Err(err) = rss::save_rss_state(&rss_path, &state) {
        state.seen_guids = previous;
        return Err(err);
    }
    Ok(())
}

fn cancel_pending<W: Write>(stream: &mut W, pending: &[PendingRequest]) -> Result<(), String> {
    for entry in pending {
        peer::write_message(
            stream,
            &peer::Message::Cancel {
                index: entry.request.index,
                begin: entry.request.begin,
                length: entry.request.length,
            },
        )
        .map_err(|err| format!("cancel write failed: {err}"))?;
    }
    Ok(())
}

fn oldest_pending(pending: &[PendingRequest]) -> Option<&PendingRequest> {
    pending.iter().min_by_key(|entry| entry.sent_at)
}

fn abandon_inflight(
    pieces: &mut piece::PieceManager,
    pending: &mut Vec<PendingRequest>,
    active_pieces: &HashMap<u32, piece::PieceBuffer>,
) {
    if !active_pieces.is_empty() {
        for piece in active_pieces.values() {
            pieces.clear_reservation(piece.index());
            if !pieces.is_piece_complete(piece.index()) {
                let _ = pieces.reset_piece(piece.index());
            }
        }
        pending.clear();
        return;
    }

    for entry in pending.drain(..) {
        let _ = pieces.mark_block_missing(entry.request.index, entry.request.begin);
    }
}

fn allocate_reserved_piece_buffer(
    pieces: &Mutex<piece::PieceManager>,
    peer_tag: u64,
    index: u32,
    length: u32,
    budgets: &piece::PieceBufferBudgets,
) -> Result<Option<piece::PieceBuffer>, String> {
    match piece::PieceBuffer::try_new(index, length, budgets) {
        Ok(Some(buffer)) => Ok(Some(buffer)),
        Ok(None) => {
            lock_or_recover(pieces).release_piece(peer_tag, index);
            Ok(None)
        }
        Err(err) => {
            lock_or_recover(pieces).release_piece(peer_tag, index);
            Err(format!("piece buffer error: {err}"))
        }
    }
}

fn persist_active_piece<F>(
    active_pieces: &mut HashMap<u32, piece::PieceBuffer>,
    index: u32,
    persist: F,
) -> Result<piece::PieceBuffer, String>
where
    F: FnOnce(&piece::PieceBuffer) -> Result<(), String>,
{
    let active = active_pieces
        .get(&index)
        .ok_or_else(|| "active piece missing".to_string())?;
    persist(active)?;
    active_pieces
        .remove(&index)
        .ok_or_else(|| "active piece missing".to_string())
}

fn update_ui<F>(state: &Option<Arc<Mutex<ui::UiState>>>, update: F)
where
    F: FnOnce(&mut ui::UiState),
{
    if let Some(state) = state {
        let mut guard = match state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.last_error = "ui state lock poisoned; recovered".to_string();
                guard
            }
        };
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| update(&mut guard))).is_err() {
            guard.last_error = "ui update panicked".to_string();
        }
    }
}

fn port_mapping_status_message(protocol: &str, port: u16, result: &Result<(), String>) -> String {
    match result {
        Ok(()) => format!("mapped {protocol} on port {port}"),
        Err(err) => format!("failed {protocol} on port {port}: {err}"),
    }
}

fn record_port_mapping_result(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    protocol: &str,
    port: u16,
    result: Result<(), String>,
) {
    let message = port_mapping_status_message(protocol, port, &result);
    if result.is_ok() {
        log_info!("{message}");
    } else {
        log_warn!("{message}");
    }
    set_port_mapping_status(ui_state, protocol, message);
}

fn set_port_mapping_status(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    protocol: &str,
    message: String,
) {
    update_ui(ui_state, |state| match protocol {
        "nat-pmp" => state.natpmp_status = message,
        "upnp" => state.upnp_status = message,
        _ => {}
    });
}

fn run_port_mapping_with_retries<F>(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    protocol: &str,
    port: u16,
    retry_delays: &[Duration],
    mut map_port: F,
) where
    F: FnMut() -> Result<(), String>,
{
    for attempt in 0..=retry_delays.len() {
        let result = map_port();
        let error = result.as_ref().err().cloned();
        record_port_mapping_result(ui_state, protocol, port, result);
        let Some(error) = error else {
            return;
        };
        let Some(delay) = retry_delays.get(attempt) else {
            return;
        };
        let message = format!(
            "retrying {protocol} on port {port} in {}s after: {error}",
            delay.as_secs()
        );
        log_info!("{message}");
        set_port_mapping_status(ui_state, protocol, message);
        sleep_with_shutdown(*delay);
        if shutdown_requested() {
            return;
        }
    }
}

fn set_peer_interest(
    interested_peers: &Arc<AtomicUsize>,
    peer_interested: &mut bool,
    interested: bool,
) {
    if *peer_interested == interested {
        return;
    }
    *peer_interested = interested;
    if interested {
        interested_peers.fetch_add(1, Ordering::SeqCst);
    } else {
        let _ = interested_peers.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
            Some(value.saturating_sub(1))
        });
    }
}

fn add_peer_country(torrent: &mut ui::UiTorrent, cc: &str) {
    if let Some(entry) = torrent
        .peer_country_counts
        .iter_mut()
        .find(|(c, _)| c == cc)
    {
        entry.1 += 1;
    } else {
        torrent.peer_country_counts.push((cc.to_string(), 1));
    }
    torrent
        .peer_country_counts
        .sort_by_key(|entry| std::cmp::Reverse(entry.1));
}

fn remove_peer_country(torrent: &mut ui::UiTorrent, cc: &str) {
    if let Some(entry) = torrent
        .peer_country_counts
        .iter_mut()
        .find(|(c, _)| c == cc)
    {
        entry.1 = entry.1.saturating_sub(1);
    }
    torrent.peer_country_counts.retain(|(_, count)| *count > 0);
}

enum PeerCountryDelta<'a> {
    Add(&'a str),
    Remove(&'a str),
    None,
}

#[cfg(test)]
fn set_active_peer_count(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    active_peers: &Arc<AtomicUsize>,
) {
    update_active_peer_ui(
        ui_state,
        torrent_id,
        active_peers.load(Ordering::SeqCst),
        PeerCountryDelta::None,
    );
}

fn add_active_peer_session(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    active_peers: &Arc<AtomicUsize>,
    addr: SocketAddr,
) -> Option<String> {
    let active_count = active_peers.fetch_add(1, Ordering::SeqCst) + 1;
    let geo_cc = GEOIP_DB
        .get()
        .and_then(|db| db.lookup(addr.ip()))
        .map(|cc| cc.to_string());
    update_active_peer_ui(
        ui_state,
        torrent_id,
        active_count,
        geo_cc
            .as_deref()
            .map(PeerCountryDelta::Add)
            .unwrap_or(PeerCountryDelta::None),
    );
    geo_cc
}

fn remove_active_peer_session(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    active_peers: &Arc<AtomicUsize>,
    geo_cc: Option<&str>,
) {
    let previous = active_peers
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
            Some(value.saturating_sub(1))
        })
        .unwrap_or_else(|value| value);
    update_active_peer_ui(
        ui_state,
        torrent_id,
        previous.saturating_sub(1),
        geo_cc
            .map(PeerCountryDelta::Remove)
            .unwrap_or(PeerCountryDelta::None),
    );
}

fn update_active_peer_ui(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    active_count: usize,
    country_delta: PeerCountryDelta<'_>,
) {
    update_ui(ui_state, |state| {
        if state.current_id == Some(torrent_id) {
            state.active_peers = active_count;
        }
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.active_peers = active_count;
            match country_delta {
                PeerCountryDelta::Add(cc) => add_peer_country(torrent, cc),
                PeerCountryDelta::Remove(cc) => remove_peer_country(torrent, cc),
                PeerCountryDelta::None => {}
            }
        });
    });
}

fn update_torrent_entry<F>(state: &mut ui::UiState, torrent_id: u64, update: F)
where
    F: FnOnce(&mut ui::UiTorrent),
{
    if state.deleted_torrents.contains(&torrent_id) {
        return;
    }
    if let Some(pos) = state
        .torrents
        .iter()
        .position(|torrent| torrent.id == torrent_id)
    {
        update(&mut state.torrents[pos]);
        return;
    }
    let mut entry = ui::UiTorrent {
        id: torrent_id,
        ..ui::UiTorrent::default()
    };
    update(&mut entry);
    state.torrents.push(entry);
}

const SPEED_HISTORY_POINTS: usize = 90;

fn push_speed_sample(history: &mut Vec<f64>, value: f64) {
    history.push(value.max(0.0));
    if history.len() > SPEED_HISTORY_POINTS {
        let excess = history.len() - SPEED_HISTORY_POINTS;
        history.drain(..excess);
    }
}

fn aggregate_session_rates(state: &ui::UiState) -> (f64, f64) {
    let download = state
        .torrents
        .iter()
        .map(|torrent| torrent.download_rate_bps)
        .sum();
    let upload = state
        .torrents
        .iter()
        .map(|torrent| torrent.upload_rate_bps)
        .sum();
    (download, upload)
}

#[cfg(test)]
mod session_rate_tests {
    use super::*;

    #[test]
    fn aggregate_session_rates_sums_all_torrent_rates() {
        let state = ui::UiState {
            torrents: vec![
                ui::UiTorrent {
                    download_rate_bps: 2_000_000.0,
                    upload_rate_bps: 100_000.0,
                    ..ui::UiTorrent::default()
                },
                ui::UiTorrent {
                    download_rate_bps: 1_500_000.0,
                    upload_rate_bps: 250_000.0,
                    ..ui::UiTorrent::default()
                },
            ],
            ..ui::UiState::default()
        };

        let (download, upload) = aggregate_session_rates(&state);
        assert_eq!(download, 3_500_000.0);
        assert_eq!(upload, 350_000.0);
    }
}

fn set_torrent_completion_ui(
    state: &mut ui::UiState,
    torrent_id: u64,
    completed_pieces: usize,
    completed_bytes: u64,
) {
    update_torrent_entry(state, torrent_id, |torrent| {
        torrent.completed_pieces = completed_pieces;
        torrent.completed_bytes = completed_bytes;
    });
    if state.current_id == Some(torrent_id) {
        state.completed_pieces = completed_pieces;
        state.completed_bytes = completed_bytes;
    }
}

fn apply_piece_completion_ui(
    state: &mut ui::UiState,
    torrent_id: u64,
    completed_pieces: usize,
    file_spans: &[FileSpan],
    piece_start: u64,
    piece_len: u64,
    increment_bytes: bool,
) {
    update_torrent_entry(state, torrent_id, |torrent| {
        torrent.completed_pieces = completed_pieces;
        if increment_bytes {
            torrent.completed_bytes = torrent.completed_bytes.saturating_add(piece_len);
            apply_piece_to_files(&mut torrent.files, file_spans, piece_start, piece_len);
        }
    });
    if state.current_id == Some(torrent_id) {
        state.completed_pieces = completed_pieces;
        if increment_bytes {
            state.completed_bytes = state.completed_bytes.saturating_add(piece_len);
            apply_piece_to_files(&mut state.files, file_spans, piece_start, piece_len);
        }
    }
}

#[cfg(test)]
mod ui_progress_tests {
    use super::*;

    fn torrent_entry(id: u64, completed_pieces: usize, completed_bytes: u64) -> ui::UiTorrent {
        ui::UiTorrent {
            id,
            completed_pieces,
            completed_bytes,
            ..ui::UiTorrent::default()
        }
    }

    #[test]
    fn piece_completion_updates_only_matching_torrent_bytes() {
        let mut state = ui::UiState {
            current_id: Some(2),
            completed_pieces: 2,
            completed_bytes: 200,
            torrents: vec![torrent_entry(1, 1, 100), torrent_entry(2, 2, 200)],
            ..ui::UiState::default()
        };

        apply_piece_completion_ui(&mut state, 2, 3, &[], 0, 16, true);

        let torrent1 = state
            .torrents
            .iter()
            .find(|torrent| torrent.id == 1)
            .unwrap();
        let torrent2 = state
            .torrents
            .iter()
            .find(|torrent| torrent.id == 2)
            .unwrap();
        assert_eq!(torrent1.completed_pieces, 1);
        assert_eq!(torrent1.completed_bytes, 100);
        assert_eq!(torrent2.completed_pieces, 3);
        assert_eq!(torrent2.completed_bytes, 216);
        assert_eq!(state.completed_pieces, 3);
        assert_eq!(state.completed_bytes, 216);
    }

    #[test]
    fn piece_completion_does_not_mutate_root_for_non_current_torrent() {
        let mut state = ui::UiState {
            current_id: Some(1),
            completed_pieces: 1,
            completed_bytes: 100,
            torrents: vec![torrent_entry(1, 1, 100), torrent_entry(2, 2, 200)],
            ..ui::UiState::default()
        };

        apply_piece_completion_ui(&mut state, 2, 3, &[], 0, 16, true);

        let torrent2 = state
            .torrents
            .iter()
            .find(|torrent| torrent.id == 2)
            .unwrap();
        assert_eq!(torrent2.completed_pieces, 3);
        assert_eq!(torrent2.completed_bytes, 216);
        assert_eq!(state.completed_pieces, 1);
        assert_eq!(state.completed_bytes, 100);
    }

    #[test]
    fn completion_sync_updates_current_torrent_and_root_fields() {
        let mut state = ui::UiState {
            current_id: Some(2),
            completed_pieces: 1,
            completed_bytes: 100,
            torrents: vec![torrent_entry(1, 1, 100), torrent_entry(2, 2, 200)],
            ..ui::UiState::default()
        };

        set_torrent_completion_ui(&mut state, 2, 4, 400);

        let torrent2 = state
            .torrents
            .iter()
            .find(|torrent| torrent.id == 2)
            .unwrap();
        assert_eq!(torrent2.completed_pieces, 4);
        assert_eq!(torrent2.completed_bytes, 400);
        assert_eq!(state.completed_pieces, 4);
        assert_eq!(state.completed_bytes, 400);
    }

    #[test]
    fn completion_sync_does_not_mutate_root_for_non_current_torrent() {
        let mut state = ui::UiState {
            current_id: Some(1),
            completed_pieces: 1,
            completed_bytes: 100,
            torrents: vec![torrent_entry(1, 1, 100), torrent_entry(2, 2, 200)],
            ..ui::UiState::default()
        };

        set_torrent_completion_ui(&mut state, 2, 4, 400);

        let torrent2 = state
            .torrents
            .iter()
            .find(|torrent| torrent.id == 2)
            .unwrap();
        assert_eq!(torrent2.completed_pieces, 4);
        assert_eq!(torrent2.completed_bytes, 400);
        assert_eq!(state.completed_pieces, 1);
        assert_eq!(state.completed_bytes, 100);
    }
}

#[cfg(test)]
mod parsing_tests {
    use super::*;

    #[test]
    fn query_pairs_decode_percent_and_plus() {
        let pairs = parse_query_pairs("dn=hello+world&tr=http%3A%2F%2Ftracker&x=1");
        assert_eq!(pairs[0], ("dn".to_string(), "hello world".to_string()));
        assert_eq!(pairs[1], ("tr".to_string(), "http://tracker".to_string()));
        assert_eq!(pairs[2], ("x".to_string(), "1".to_string()));
    }

    #[test]
    fn query_pairs_preserve_utf8() {
        let pairs = parse_query_pairs("dn=Espa%C3%B1a+%F0%9F%9A%80&raw=café");
        assert_eq!(pairs[0].1, "España 🚀");
        assert_eq!(pairs[1].1, "café");
    }

    #[test]
    fn info_hash_parsing_supports_hex_and_base32() {
        let hex = "00112233445566778899aabbccddeeff00112233";
        let parsed_hex = parse_info_hash(hex).unwrap();
        assert_eq!(parsed_hex[0], 0x00);
        assert_eq!(parsed_hex[1], 0x11);
        assert_eq!(parsed_hex[19], 0x33);

        let parsed_base32 = parse_info_hash("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap();
        assert_eq!(parsed_base32, [0u8; 20]);
        assert!(parse_info_hash("invalid").is_none());
    }

    #[test]
    fn magnet_parser_extracts_trackers_sources_and_peers() {
        let link = "\
magnet:?xt=urn:btih:00112233445566778899AABBCCDDEEFF00112233\
&tr=http%3A%2F%2Ftracker.example%2Fannounce\
&ws=http%3A%2F%2Fseed.example%2Ffile\
&xs=http%3A%2F%2Fmirror.example%2Fmeta.torrent\
&x.pe=8.8.8.8:6881";
        let parsed = parse_magnet(link).unwrap();
        assert_eq!(
            parsed.info_hash,
            parse_info_hash("00112233445566778899aabbccddeeff00112233").unwrap()
        );
        assert_eq!(parsed.info_hash_v1, Some(parsed.info_hash));
        assert_eq!(parsed.trackers, vec!["http://tracker.example/announce"]);
        assert_eq!(parsed.web_seeds, vec!["http://seed.example/file"]);
        assert_eq!(parsed.sources, vec!["http://mirror.example/meta.torrent"]);
        assert_eq!(parsed.peers, vec!["8.8.8.8:6881".parse().unwrap()]);
    }

    #[test]
    fn magnet_parser_rejects_explicit_peers_outside_public_scope() {
        let link = "\
magnet:?xt=urn:btih:00112233445566778899AABBCCDDEEFF00112233\
&x.pe=127.0.0.1:6881\
&x.pe=10.0.0.1:6881\
&x.pe=169.254.1.1:6881\
&x.pe=192.0.0.1:6881\
&x.pe=[::ffff:192.168.1.1]:6881\
&x.pe=[fc00::1]:6881\
&x.pe=[fe80::1]:6881\
&x.pe=8.8.8.8:0\
&x.pe=[::ffff:8.8.8.8]:6881";

        let parsed = parse_magnet(link).unwrap();
        assert_eq!(parsed.peers, vec!["8.8.8.8:6881".parse().unwrap()]);
    }

    #[test]
    fn magnet_parser_requires_info_hash() {
        let err = match parse_magnet("magnet:?tr=http%3A%2F%2Ftracker") {
            Ok(_) => panic!("expected parse error"),
            Err(err) => err,
        };
        assert!(err.contains("missing info hash"));
    }

    #[test]
    fn magnet_parser_preserves_v2_hash_and_rejects_conflicts() {
        let digest = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let parsed = parse_magnet(&format!("magnet:?xt=urn:btmh:1220{digest}")).unwrap();
        let expected = decode_hex_32(digest).unwrap();
        assert_eq!(parsed.info_hash_v1, None);
        assert_eq!(parsed.info_hash_v2, Some(expected));
        assert_eq!(parsed.info_hash, expected[..20]);

        let conflicting = format!(
            "magnet:?xt=urn:btih:{}&xt=urn:btih:{}",
            "00".repeat(20),
            "11".repeat(20)
        );
        assert!(parse_magnet(&conflicting)
            .unwrap_err()
            .contains("conflicting v1"));
    }

    #[test]
    fn metadata_validation_checks_every_exact_topic() {
        let info = b"d6:lengthi5e4:name4:test12:piece lengthi5e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        let expected = ExpectedInfoHashes {
            v1: Some(sha1::sha1(info)),
            v2: Some(sha256::sha256(info)),
        };
        assert!(validate_info_hashes(info, expected).is_ok());

        let mut wrong_v2 = expected;
        wrong_v2.v2 = Some([7u8; 32]);
        assert!(validate_info_hashes(info, wrong_v2)
            .unwrap_err()
            .contains("SHA-256"));

        let torrent_data = wrap_torrent_with_info(info, &[], &[]);
        assert!(validate_magnet_torrent(
            &torrent_data,
            ExpectedInfoHashes {
                v1: Some(sha1::sha1(info)),
                v2: None,
            }
        )
        .is_ok());
        assert!(validate_magnet_torrent(
            &torrent_data,
            ExpectedInfoHashes {
                v1: Some([9u8; 20]),
                v2: None,
            }
        )
        .unwrap_err()
        .contains("SHA-1"));
    }

    #[test]
    fn extended_handshake_roundtrip() {
        let payload = build_ext_handshake(Some(4096), true);
        let (ut_metadata, ut_pex, metadata_size) = parse_extended_handshake(&payload).unwrap();
        assert_eq!(ut_metadata, Some(1));
        assert_eq!(ut_pex, Some(2));
        assert_eq!(metadata_size, Some(4096));
    }

    #[test]
    fn metadata_message_parsing_validates_required_fields() {
        let valid = b"d8:msg_typei1e5:piecei0e10:total_sizei5eehello";
        let msg = parse_metadata_message(valid).unwrap();
        assert_eq!(msg.msg_type, 1);
        assert_eq!(msg.piece, 0);
        assert_eq!(msg.total_size, Some(5));
        assert_eq!(msg.data, b"hello");

        assert!(parse_metadata_message(b"d5:piecei0ee").is_err());
        assert!(parse_metadata_message(b"d8:msg_typei1ee").is_err());
        assert!(parse_metadata_message(b"d8:msg_typei1e5:piecei0eehello").is_err());
        let oversized = format!(
            "d8:msg_typei1e5:piecei0e10:total_sizei{}ee",
            MAX_TORRENT_BYTES + 1
        );
        assert!(parse_metadata_message(oversized.as_bytes()).is_err());
    }

    #[test]
    fn metadata_size_and_piece_bounds_are_enforced() {
        let oversized = format!("d13:metadata_sizei{}ee", MAX_TORRENT_BYTES + 1);
        assert!(parse_extended_handshake(oversized.as_bytes()).is_err());
        assert_eq!(expected_metadata_piece_len(16_385, 0), Some(16_384));
        assert_eq!(expected_metadata_piece_len(16_385, 1), Some(1));
        assert_eq!(expected_metadata_piece_len(16_385, 2), None);
    }

    #[test]
    fn wrap_torrent_with_info_produces_parseable_torrent() {
        let info = b"d6:lengthi5e4:name4:test12:piece lengthi5e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        let wrapped = wrap_torrent_with_info(
            info,
            &["http://tracker.example/announce".to_string()],
            &["http://seed.example/file".to_string()],
        );
        let meta = torrent::parse_torrent(&wrapped).unwrap();
        assert_eq!(
            meta.announce,
            Some(b"http://tracker.example/announce".to_vec())
        );
        assert_eq!(meta.url_list, vec![b"http://seed.example/file".to_vec()]);
        assert_eq!(meta.info.total_length(), 5);
    }

    #[cfg(feature = "webseed")]
    #[test]
    fn webseed_urls_follow_bep17_and_bep19_semantics() {
        assert_eq!(
            build_webseed_url("https://seed.example/files/", b"na me.bin", false),
            "https://seed.example/files/na%20me.bin"
        );
        assert_eq!(
            build_webseed_url("https://seed.example/exact.bin", b"ignored.bin", false),
            "https://seed.example/exact.bin"
        );
        assert_eq!(
            build_webseed_url("https://seed.example/root", b"bundle/a.bin", true),
            "https://seed.example/root/bundle/a.bin"
        );

        let info_hash = [0xabu8; 20];
        let hoffman = build_httpseed_url("https://seed.example/script?token=x", info_hash, 7);
        assert!(hoffman.starts_with("https://seed.example/script?token=x&info_hash="));
        assert!(hoffman.ends_with("&piece=7"));
        assert_eq!(hoffman.matches("%AB").count(), 20);

        let info = b"d6:lengthi5e4:name4:test12:piece lengthi5e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        let mut meta = torrent::parse_torrent(&wrap_torrent_with_info(info, &[], &[])).unwrap();
        meta.url_list = vec![b"https://seed.example/data/".to_vec()];
        meta.httpseeds = vec![b"https://seed.example/script".to_vec()];
        assert_eq!(
            collect_web_seeds(&meta),
            vec![
                WebSeed::GetRight("https://seed.example/data/".to_string()),
                WebSeed::Hoffman("https://seed.example/script".to_string()),
            ]
        );

        meta.info.length = None;
        meta.info.files = vec![torrent::FileInfo {
            length: 5,
            path: vec![b".pad".to_vec(), b"5".to_vec()],
            attr: b"p".to_vec(),
        }];
        let spans = build_file_spans(&meta).unwrap();
        assert!(spans[0].is_padding);
    }

    #[cfg(feature = "webseed")]
    #[test]
    fn webseed_payload_memory_is_bounded_and_denial_releases_piece() {
        let max_charge = webseed_memory_budget_bytes(torrent::MAX_PIECE_LENGTH as u32).unwrap();
        assert!(max_charge <= MAX_TORRENT_PIECE_BUFFER_BYTES);

        let global = Arc::new(piece::PieceBufferBudget::new(max_charge));
        let torrent_budget = Arc::new(piece::PieceBufferBudget::new(max_charge));
        let budgets =
            piece::PieceBufferBudgets::new(Arc::clone(&global), Arc::clone(&torrent_budget));
        let reservation = budgets.try_reserve(max_charge).unwrap();
        assert_eq!(global.used(), max_charge);
        assert_eq!(torrent_budget.used(), max_charge);
        assert!(budgets.try_reserve(1).is_none());
        drop(reservation);
        assert_eq!(global.used(), 0);
        assert_eq!(torrent_budget.used(), 0);

        let info = b"d6:lengthi5e4:name4:test12:piece lengthi5e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        let meta = torrent::parse_torrent(&wrap_torrent_with_info(info, &[], &[])).unwrap();
        let pieces = Mutex::new(piece::PieceManager::new(&meta).unwrap());
        let selected = lock_or_recover(&pieces)
            .reserve_piece_for_peer(WEBSEED_RESERVATION_ID, &[0x80], false)
            .unwrap();
        let denied = piece::PieceBufferBudgets::new(
            Arc::new(piece::PieceBufferBudget::new(0)),
            Arc::new(piece::PieceBufferBudget::new(0)),
        );
        assert!(try_reserve_webseed_memory(
            &pieces,
            selected,
            meta.info.piece_length as u32,
            &denied,
        )
        .is_none());
        assert_eq!(
            lock_or_recover(&pieces).reserve_piece_for_peer(2, &[0x80], false),
            Some(selected)
        );
    }
}

#[cfg(test)]
mod core_helpers_tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustorrent-main-test-{label}-{nanos}"))
    }

    fn empty_in_flight() -> InFlightTorrents {
        Arc::new(Mutex::new(HashMap::new()))
    }

    fn tracker_meta(private: bool) -> torrent::TorrentMeta {
        torrent::TorrentMeta {
            announce: Some(b"http://tracker.local/announce".to_vec()),
            announce_list: vec![vec![
                b"http://tracker.local/announce".to_vec(),
                b"udp://tracker.local:6969/announce".to_vec(),
            ]],
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info_hash: [1u8; 20],
            info_hash_v2: None,
            piece_layers: Vec::new(),
            meta_version: 1,
            info: torrent::InfoDict {
                name: b"t".to_vec(),
                piece_length: 16,
                pieces: vec![[0u8; 20]],
                length: Some(16),
                files: Vec::new(),
                private,
                file_tree: Vec::new(),
            },
        }
    }

    fn test_torrent_bytes() -> Vec<u8> {
        let info = b"d6:lengthi16e4:name4:test12:piece lengthi16e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        wrap_torrent_with_info(info, &[], &[])
    }

    fn single_torrent_bytes(name: &[u8], piece_byte: u8) -> Vec<u8> {
        bencode::encode(&Value::Dict(vec![(
            b"info".to_vec(),
            Value::Dict(vec![
                (b"length".to_vec(), Value::Int(1)),
                (b"name".to_vec(), Value::Bytes(name.to_vec())),
                (b"piece length".to_vec(), Value::Int(16)),
                (b"pieces".to_vec(), Value::Bytes(vec![piece_byte; 20])),
            ]),
        )]))
    }

    fn multifile_torrent_bytes(name: &[u8], piece_byte: u8) -> Vec<u8> {
        bencode::encode(&Value::Dict(vec![(
            b"info".to_vec(),
            Value::Dict(vec![
                (
                    b"files".to_vec(),
                    Value::List(vec![Value::Dict(vec![
                        (b"length".to_vec(), Value::Int(1)),
                        (
                            b"path".to_vec(),
                            Value::List(vec![Value::Bytes(b"file.bin".to_vec())]),
                        ),
                    ])]),
                ),
                (b"name".to_vec(), Value::Bytes(name.to_vec())),
                (b"piece length".to_vec(), Value::Int(16)),
                (b"pieces".to_vec(), Value::Bytes(vec![piece_byte; 20])),
            ]),
        )]))
    }

    fn claim_test_torrent(
        store: &SessionStore,
        torrent_bytes: Vec<u8>,
        download_dir: &Path,
    ) -> Result<[u8; 20], String> {
        let meta = torrent::parse_torrent(&torrent_bytes).map_err(|err| err.to_string())?;
        let _operation = store.lock_operation();
        store.upsert_with_storage_claim(
            meta.info_hash,
            String::from_utf8_lossy(&meta.info.name).into_owned(),
            torrent_bytes,
            download_dir,
            false,
            &[],
        )?;
        Ok(meta.info_hash)
    }

    fn v2_test_meta(data: &[u8], piece_length: u32) -> torrent::TorrentMeta {
        let block_count = data.len().div_ceil(piece::BLOCK_LEN as usize);
        let file_tree_length = block_count.next_power_of_two() * piece::BLOCK_LEN as usize;
        let piece_hashes = data
            .chunks(piece_length as usize)
            .map(|chunk| {
                let tree_length = if data.len() <= piece_length as usize {
                    file_tree_length as u32
                } else {
                    piece_length
                };
                sha256::merkle_piece_root(chunk, tree_length).unwrap()
            })
            .collect::<Vec<_>>();
        let pieces_root = if data.len() <= piece_length as usize {
            piece_hashes[0]
        } else {
            sha256::merkle_root_from_piece_layer(&piece_hashes, piece_length).unwrap()
        };
        torrent::TorrentMeta {
            announce: None,
            announce_list: Vec::new(),
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info_hash: [1u8; 20],
            info_hash_v2: Some([1u8; 32]),
            piece_layers: if data.len() > piece_length as usize {
                vec![(pieces_root.to_vec(), piece_hashes)]
            } else {
                Vec::new()
            },
            meta_version: 2,
            info: torrent::InfoDict {
                name: b"bundle".to_vec(),
                piece_length: piece_length as u64,
                pieces: Vec::new(),
                length: None,
                files: Vec::new(),
                private: false,
                file_tree: vec![torrent::FileTreeEntry {
                    path: vec![b"file.bin".to_vec()],
                    length: data.len() as u64,
                    pieces_root: Some(pieces_root),
                }],
            },
        }
    }

    #[test]
    fn v2_leaf_hashes_are_served_from_verified_small_file_data() {
        let data = vec![b'x'; piece::BLOCK_LEN as usize + 17];
        let meta = v2_test_meta(&data, 32 * 1024);
        let request = peer::HashRequest {
            pieces_root: meta.info.file_tree[0].pieces_root.unwrap(),
            base_layer: 0,
            index: 0,
            length: 2,
            proof_layers: 1,
        };
        let root = temp_path("v2-hashes");
        fs::create_dir_all(&root).unwrap();
        let mut storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        storage.write_at(0, &data).unwrap();
        let mut pieces = piece::PieceManager::new(&meta).unwrap();
        pieces.mark_piece_complete(0).unwrap();
        let store = V2HashStore::new(&meta).unwrap();

        let hashes = store
            .leaf_hashes_for(request, &pieces, &mut storage)
            .unwrap();
        assert_eq!(
            hashes,
            vec![
                sha256::sha256(&data[..piece::BLOCK_LEN as usize]),
                sha256::sha256(&data[piece::BLOCK_LEN as usize..]),
            ]
        );

        pieces.reset_piece(0).unwrap();
        assert!(store
            .leaf_hashes_for(request, &pieces, &mut storage)
            .is_none());
        drop(storage);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn v2_hash_budget_defers_and_charges_mandatory_requests() {
        let now = Instant::now();
        let next_window = now.checked_add(HASH_REQUEST_WINDOW).unwrap();
        let mut count_budget = HashRequestBudget::new_at(now);
        for _ in 0..MAX_HASH_REQUESTS_PER_WINDOW {
            assert_eq!(
                count_budget.reserve_at(0, false, now),
                HashBudgetDecision::ServeAfter(Duration::ZERO)
            );
        }
        assert_eq!(
            count_budget.reserve_at(0, false, now),
            HashBudgetDecision::Reject
        );
        assert_eq!(
            count_budget.reserve_at(MAX_HASH_DISK_BYTES_PER_WINDOW, true, now),
            HashBudgetDecision::ServeAfter(HASH_REQUEST_WINDOW)
        );
        assert_eq!(count_budget.window_started, next_window);
        assert_eq!(count_budget.requests, 1);
        assert_eq!(count_budget.disk_bytes, MAX_HASH_DISK_BYTES_PER_WINDOW);

        let mut byte_budget = HashRequestBudget::new_at(now);
        assert_eq!(
            byte_budget.reserve_at(MAX_HASH_DISK_BYTES_PER_WINDOW, false, now),
            HashBudgetDecision::ServeAfter(Duration::ZERO)
        );
        assert_eq!(
            byte_budget.reserve_at(1, false, now),
            HashBudgetDecision::Reject
        );
        assert_eq!(
            byte_budget.reserve_at(1, true, now),
            HashBudgetDecision::ServeAfter(HASH_REQUEST_WINDOW)
        );
        assert_eq!(byte_budget.window_started, next_window);
        assert_eq!(byte_budget.requests, 1);
        assert_eq!(byte_budget.disk_bytes, 1);
    }

    #[test]
    fn v2_hash_response_payload_charge_includes_header_and_hashes() {
        let request = peer::HashRequest {
            pieces_root: [7; 32],
            base_layer: 0,
            index: 3,
            length: 2,
            proof_layers: 1,
        };
        assert_eq!(
            hash_response_payload_bytes(&peer::Message::HashReject(request)),
            HASH_MESSAGE_FIXED_PAYLOAD_BYTES
        );
        assert_eq!(
            hash_response_payload_bytes(&peer::Message::Hashes {
                request,
                hashes: vec![[1; 32], [2; 32]],
            }),
            HASH_MESSAGE_FIXED_PAYLOAD_BYTES + 64
        );
    }

    #[test]
    fn padding_spans_never_keep_a_piece_wanted() {
        let spans = vec![
            FileSpan {
                path: "bundle/a".to_string(),
                web_path: b"bundle/a".to_vec(),
                is_padding: false,
                offset: 0,
                length: 3,
            },
            FileSpan {
                path: "bundle/.pad/13".to_string(),
                web_path: b"bundle/.pad/13".to_vec(),
                is_padding: true,
                offset: 3,
                length: 13,
            },
            FileSpan {
                path: "bundle/b".to_string(),
                web_path: b"bundle/b".to_vec(),
                is_padding: false,
                offset: 16,
                length: 5,
            },
        ];
        assert_eq!(
            compute_piece_priorities(
                &spans,
                &[
                    piece::PRIORITY_SKIP,
                    piece::PRIORITY_HIGH,
                    piece::PRIORITY_NORMAL,
                ],
                16,
                2,
            ),
            vec![piece::PRIORITY_SKIP, piece::PRIORITY_NORMAL]
        );
    }

    #[cfg(feature = "webseed")]
    #[test]
    fn getright_layout_does_not_infer_multi_file_from_span_count() {
        let mut one_entry_multi = tracker_meta(false);
        one_entry_multi.info.length = None;
        one_entry_multi.info.files = vec![torrent::FileInfo {
            length: 16,
            path: vec![b"only.bin".to_vec()],
            attr: Vec::new(),
        }];
        assert!(is_getright_multi_file(&one_entry_multi));
        let spans = build_file_spans(&one_entry_multi).unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(
            build_webseed_url(
                "https://seed.example/root",
                &spans[0].web_path,
                is_getright_multi_file(&one_entry_multi),
            ),
            "https://seed.example/root/t/only.bin"
        );

        let v2_single = v2_test_meta(b"small", 16 * 1024);
        assert!(!is_getright_multi_file(&v2_single));
        assert_eq!(
            build_file_spans(&v2_single).unwrap()[0].web_path,
            b"file.bin"
        );
    }

    #[test]
    fn rate_limiter_serializes_concurrent_reservations() {
        let limiter = RateLimiter::new(100);
        let now = Instant::now();

        assert_eq!(limiter.reserve_delay(100, 100, now), Duration::ZERO);
        assert_eq!(limiter.reserve_delay(100, 100, now), Duration::from_secs(1));
        assert_eq!(limiter.reserve_delay(100, 100, now), Duration::from_secs(2));
    }

    #[test]
    fn session_lock_rejects_a_second_owner() {
        let root = temp_path("session-lock");
        fs::create_dir_all(&root).unwrap();

        let first = acquire_session_lock(&root).unwrap();
        let second = acquire_session_lock(&root);
        assert!(second.is_err());

        drop(first);
        assert!(acquire_session_lock(&root).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn pinned_state_directory_lock_survives_legacy_lock_replacement() {
        let root = temp_path("session-lock-replaced");
        fs::create_dir_all(&root).unwrap();

        let first = acquire_session_lock(&root).unwrap();
        fs::remove_file(root.join(".rustorrent.lock")).unwrap();
        assert!(acquire_session_lock(&root).is_err());

        drop(first);
        assert!(acquire_session_lock(&root).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pid_file_guard_only_removes_its_own_pid() {
        let root = temp_path("pid-guard");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("rustorrent.pid");
        let pid = std::process::id();
        fs::write(&path, format!("{pid}\n")).unwrap();
        drop(PidFileGuard {
            path: path.clone(),
            pid,
        });
        assert!(!path.exists());

        fs::write(&path, format!("{}\n", pid.wrapping_add(1))).unwrap();
        drop(PidFileGuard {
            path: path.clone(),
            pid,
        });
        assert!(path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn session_lock_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let root = temp_path("session-lock-symlink");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.txt");
        fs::write(&target, b"do-not-touch").unwrap();
        symlink(&target, root.join(".rustorrent.lock")).unwrap();

        assert!(acquire_session_lock(&root).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"do-not-touch");
        let _ = fs::remove_dir_all(root);
    }

    fn make_test_context(id: u64, root: &Path) -> Arc<TorrentContext> {
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let file_spans = Arc::new(build_file_spans(&meta).unwrap());
        let file_priorities = Arc::new(Mutex::new(vec![piece::PRIORITY_NORMAL; file_spans.len()]));
        Arc::new(TorrentContext {
            id,
            info_hash: meta.info_hash,
            hybrid_v2_info_hash: None,
            peer_id: [9u8; 20],
            pieces: Arc::new(Mutex::new(piece::PieceManager::new(&meta).unwrap())),
            storage: Arc::new(Mutex::new(
                storage::Storage::new(&meta, root, storage::StorageOptions::default()).unwrap(),
            )),
            completed_log: Arc::new(Mutex::new(Vec::new())),
            base_piece_length: meta.info.piece_length,
            v2_hashes: Arc::new(V2HashStore::new(&meta).unwrap()),
            file_spans,
            file_priorities,
            limits: TransferLimits {
                global_down: Arc::new(RateLimiter::new(0)),
                global_up: Arc::new(RateLimiter::new(0)),
                torrent_down: Arc::new(RateLimiter::new(0)),
                torrent_up: Arc::new(RateLimiter::new(0)),
            },
            downloaded: Arc::new(AtomicU64::new(0)),
            uploaded: Arc::new(AtomicU64::new(0)),
            active_peers: Arc::new(AtomicUsize::new(0)),
            interested_peers: Arc::new(AtomicUsize::new(0)),
            upload_requests_served: Arc::new(AtomicU64::new(0)),
            paused: Arc::new(AtomicBool::new(false)),
            stop_requested: Arc::new(AtomicBool::new(false)),
            allow_completion_reentry: Arc::new(AtomicBool::new(true)),
            rechecking: Arc::new(AtomicBool::new(false)),
            resume_save_requested: Arc::new(AtomicBool::new(false)),
            delete_data_requested: Arc::new(AtomicBool::new(false)),
            archive_requested: Arc::new(AtomicBool::new(false)),
            teardown_failed: Arc::new(AtomicBool::new(false)),
            upload_manager: Arc::new(UploadManager::new(UPLOAD_SLOTS)),
            peer_tags: Arc::new(AtomicU64::new(1)),
            peer_cancellations: Arc::new(Mutex::new(HashMap::new())),
            label: Arc::new(Mutex::new(String::new())),
            trackers: Arc::new(Mutex::new(collect_trackers(&meta))),
            throttle_group: Arc::new(Mutex::new(None)),
            ratio_group: Arc::new(Mutex::new(None)),
            file_renames: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[test]
    fn in_flight_reservation_closes_the_queue_to_registry_duplicate_window() {
        let root = temp_path("in-flight-dedup");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        let session_store = SessionStore::load(&root).unwrap();
        let in_flight: InFlightTorrents = Arc::new(Mutex::new(HashMap::new()));
        let reservation = InFlightTorrentGuard::acquire(&in_flight, info_hash, 1).unwrap();
        let mut queue = VecDeque::new();
        let duplicate = TorrentRequest {
            id: 2,
            source: TorrentSource::Bytes(torrent_bytes.clone()),
            download_dir: root.join("different-destination"),
            preallocate: false,
            initial_label: String::new(),
        };

        assert!(is_duplicate_torrent(
            &registry,
            &queue,
            &session_store,
            &in_flight,
            info_hash,
        ));
        assert!(!enqueue_request_if_new(
            &registry,
            &mut queue,
            &session_store,
            &in_flight,
            &None,
            duplicate.clone(),
            None,
        ));
        assert!(queue.is_empty());

        drop(reservation);
        assert!(enqueue_request_if_new(
            &registry,
            &mut queue,
            &session_store,
            &in_flight,
            &None,
            duplicate,
            None,
        ));
        assert_eq!(queue.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn path_request_is_frozen_when_it_enters_the_queue() {
        let root = temp_path("freeze-request-source");
        fs::create_dir_all(&root).unwrap();
        let torrent_path = root.join("queued.torrent");
        let torrent_bytes = test_torrent_bytes();
        fs::write(&torrent_path, &torrent_bytes).unwrap();
        let expected = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let request = TorrentRequest {
            id: 1,
            source: TorrentSource::Path(torrent_path.display().to_string()),
            download_dir: root.clone(),
            preallocate: false,
            initial_label: String::new(),
        };
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        let session_store = SessionStore::load(&root).unwrap();
        let in_flight = empty_in_flight();
        let mut queue = VecDeque::new();

        assert!(enqueue_request_if_new(
            &registry,
            &mut queue,
            &session_store,
            &in_flight,
            &None,
            request,
            None,
        ));
        fs::write(&torrent_path, b"malformed replacement").unwrap();
        assert_eq!(info_hash_for_source(&queue[0].source).unwrap(), expected);
        assert!(matches!(queue[0].source, TorrentSource::Bytes(_)));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loading_torrent_cannot_enter_offline_stop_delete_or_archive_paths() {
        let root = temp_path("loading-lifecycle-guard");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        let session_store = Arc::new(SessionStore::load(&root).unwrap());
        session_store
            .upsert(
                meta.info_hash,
                "loading".to_string(),
                torrent_bytes,
                &root,
                false,
            )
            .unwrap();
        let in_flight: InFlightTorrents =
            Arc::new(Mutex::new(HashMap::from([(meta.info_hash, 9)])));
        let mut queue = VecDeque::new();

        assert!(
            stop_torrent(&registry, &None, &mut queue, 9, &session_store, &in_flight,).is_err()
        );
        assert!(delete_torrent(
            &registry,
            &None,
            &mut queue,
            9,
            true,
            &session_store,
            &in_flight,
        )
        .is_err());
        assert!(
            archive_torrent(&registry, &None, &mut queue, 9, &session_store, &in_flight,).is_err()
        );
        assert!(session_store.contains(meta.info_hash));
        assert!(!session_store.get(meta.info_hash).unwrap().pending_delete);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn registry_collision_preserves_the_original_worker_context() {
        let root = temp_path("registry-collision");
        let first_root = root.join("first");
        let second_root = root.join("second");
        fs::create_dir_all(&first_root).unwrap();
        fs::create_dir_all(&second_root).unwrap();
        let first = make_test_context(1, &first_root);
        let second = make_test_context(2, &second_root);
        assert_eq!(first.info_hash, second.info_hash);
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));

        register_session(&registry, Arc::clone(&first)).unwrap();
        assert!(register_session(&registry, Arc::clone(&second)).is_err());
        let registered = find_context(&registry, first.info_hash).unwrap();
        assert_eq!(registered.id, first.id);
        assert!(Arc::ptr_eq(&registered, &first));

        drop(registered);
        drop(registry);
        drop(first);
        drop(second);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn torrent_resource_drain_is_bounded_and_fail_closed() {
        let root = temp_path("resource-drain-deadline");
        fs::create_dir_all(&root).unwrap();
        let context = make_test_context(39, &root);
        let storage = Arc::clone(&context.storage);
        let retained_storage = Arc::clone(&storage);
        let registry = Arc::new(Mutex::new(HashMap::new()));

        let started = Instant::now();
        let err = wait_for_torrent_resources_or_retain(
            &registry,
            &context,
            &storage,
            "test teardown",
            Instant::now() + Duration::from_millis(30),
        )
        .unwrap_err();
        assert!(err.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(context.teardown_failed.load(Ordering::Acquire));
        assert!(find_context_by_id(&registry, context.id)
            .is_some_and(|retained| Arc::ptr_eq(&retained, &context)));

        unregister_session(&registry, context.info_hash, context.id);
        drop(retained_storage);
        wait_for_torrent_resources(
            &context,
            &storage,
            "test teardown",
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        drop(storage);
        drop(context);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hybrid_handshake_upgrade_uses_and_accepts_the_exact_v2_swarm_id() {
        let root = temp_path("hybrid-handshake-upgrade");
        fs::create_dir_all(&root).unwrap();
        let mut context = make_test_context(40, &root);
        let v1 = context.info_hash;
        let v2 = [0xabu8; 20];
        Arc::get_mut(&mut context).unwrap().hybrid_v2_info_hash = Some(v2);

        let upgraded_request = peer::parse_handshake(&peer::build_handshake_with_hybrid_upgrade(
            v1, [7u8; 20], true, true,
        ))
        .unwrap();
        assert_eq!(
            inbound_handshake_response_hash(&context, &upgraded_request),
            v2
        );
        validate_outbound_handshake_hash(
            &peer::parse_handshake(&peer::build_handshake(v2, [8u8; 20], true)).unwrap(),
            v1,
            Some(v2),
        )
        .unwrap();

        let legacy_request =
            peer::parse_handshake(&peer::build_handshake(v1, [7u8; 20], true)).unwrap();
        assert_eq!(
            inbound_handshake_response_hash(&context, &legacy_request),
            v1
        );

        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        register_session(&registry, Arc::clone(&context)).unwrap();
        assert_eq!(find_context(&registry, v2).unwrap().id, context.id);
        let mut expected_hashes = vec![v1, v2];
        expected_hashes.sort_unstable();
        assert_eq!(list_info_hashes(&registry).unwrap(), expected_hashes);

        let wrong =
            peer::parse_handshake(&peer::build_handshake([3u8; 20], [8u8; 20], true)).unwrap();
        assert!(validate_outbound_handshake_hash(&wrong, v1, Some(v2)).is_err());

        drop(registry);
        drop(context);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recheck_rejects_a_concurrent_second_run() {
        SHUTDOWN.store(false, Ordering::SeqCst);
        let root = temp_path("recheck-serialized");
        fs::create_dir_all(&root).unwrap();
        let context = make_test_context(41, &root);
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        register_session(&registry, Arc::clone(&context)).unwrap();

        let storage_guard = context.storage.lock().unwrap();
        recheck_torrent(&registry, &None, context.id).unwrap();
        assert!(recheck_torrent(&registry, &None, context.id).is_err());
        drop(storage_guard);

        let deadline = Instant::now() + Duration::from_secs(2);
        while context.rechecking.load(Ordering::Acquire) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!context.rechecking.load(Ordering::Acquire));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn active_data_delete_is_deferred_until_workers_stop() {
        let root = temp_path("delete-active-deferred");
        fs::create_dir_all(&root).unwrap();
        let context = make_test_context(42, &root);
        let data_path = lock_or_recover(&context.storage)
            .file_path(0)
            .unwrap()
            .to_path_buf();
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        register_session(&registry, Arc::clone(&context)).unwrap();
        let session_store = Arc::new(SessionStore {
            path: root.join("session.benc"),
            entries: Mutex::new(HashMap::new()),
            operations: Mutex::new(()),
        });
        session_store
            .upsert(
                context.info_hash,
                "test".to_string(),
                test_torrent_bytes(),
                &root,
                false,
            )
            .unwrap();
        let mut queue = VecDeque::new();

        delete_torrent(
            &registry,
            &None,
            &mut queue,
            context.id,
            true,
            &session_store,
            &empty_in_flight(),
        )
        .unwrap();

        assert!(context.stop_requested.load(Ordering::SeqCst));
        assert!(context.delete_data_requested.load(Ordering::Acquire));
        assert!(data_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn active_archive_retains_claim_registry_and_ui_until_storage_closes() {
        let root = temp_path("archive-active-deferred");
        fs::create_dir_all(&root).unwrap();
        let context = make_test_context(43, &root);
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        register_session(&registry, Arc::clone(&context)).unwrap();
        let session_store = Arc::new(SessionStore::load(&root).unwrap());
        session_store
            .upsert(
                context.info_hash,
                "test".to_string(),
                test_torrent_bytes(),
                &root,
                false,
            )
            .unwrap();
        let ui = Some(Arc::new(Mutex::new(ui::UiState {
            torrents: vec![ui::UiTorrent {
                id: context.id,
                info_hash: hex(&context.info_hash),
                ..ui::UiTorrent::default()
            }],
            ..ui::UiState::default()
        })));
        let mut queue = VecDeque::new();

        archive_torrent(
            &registry,
            &ui,
            &mut queue,
            context.id,
            &session_store,
            &empty_in_flight(),
        )
        .unwrap();

        assert!(context.stop_requested.load(Ordering::SeqCst));
        assert!(context.archive_requested.load(Ordering::Acquire));
        assert!(session_store.contains(context.info_hash));
        assert!(find_context_by_id(&registry, context.id).is_some());
        assert_eq!(lock_or_recover(ui.as_ref().unwrap()).torrents.len(), 1);

        unregister_session(&registry, context.info_hash, context.id);
        drop(context);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resume_uses_exact_renamed_storage_paths() {
        let root = temp_path("resume-renamed-path");
        fs::create_dir_all(&root).unwrap();
        let payload = [0u8; 16];
        let torrent_bytes = bencode::encode(&Value::Dict(vec![(
            b"info".to_vec(),
            Value::Dict(vec![
                (b"length".to_vec(), Value::Int(payload.len() as i64)),
                (b"name".to_vec(), Value::Bytes(b"test".to_vec())),
                (b"piece length".to_vec(), Value::Int(16)),
                (
                    b"pieces".to_vec(),
                    Value::Bytes(sha1::sha1(&payload).to_vec()),
                ),
            ]),
        )]));
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let spans = build_file_spans(&meta).unwrap();
        let mut storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        storage.write_at(0, &payload).unwrap();
        storage.flush().unwrap();
        let old_path = storage.file_path(0).unwrap().to_path_buf();
        let renamed_path = old_path.with_file_name("renamed.bin");
        storage.rename_file(0, &old_path, &renamed_path).unwrap();
        let files = collect_storage_file_stats(&storage, &spans);
        assert_eq!(files[0].length, 16);
        assert!(files[0].mtime > 1_000_000_000_000);

        let resume = ResumeData {
            info_hash: meta.info_hash,
            piece_length: meta.info.piece_length,
            bitfield: vec![0x80],
            file_priorities: vec![piece::PRIORITY_NORMAL],
            files,
            downloaded: 16,
            uploaded: 0,
            peers: Vec::new(),
            file_renames: vec![(0, "renamed.bin".to_string())],
        };
        let mut pieces = piece::PieceManager::new(&meta).unwrap();
        let stats = resume_from_storage(
            &mut pieces,
            &mut storage,
            meta.info.piece_length,
            &spans,
            Some(&resume),
        )
        .unwrap();
        assert_eq!(stats.completed_bytes, 16);
        assert!(pieces.is_complete());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resume_rehashes_claimed_pieces_even_when_file_stats_match() {
        let root = temp_path("resume-rehashes-claimed");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let spans = build_file_spans(&meta).unwrap();
        let mut storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        storage.write_at(0, &[1u8; 16]).unwrap();
        storage.flush().unwrap();
        let resume = ResumeData {
            info_hash: meta.info_hash,
            piece_length: meta.info.piece_length,
            bitfield: vec![0x80],
            file_priorities: vec![piece::PRIORITY_NORMAL],
            files: collect_storage_file_stats(&storage, &spans),
            downloaded: 16,
            uploaded: 0,
            peers: Vec::new(),
            file_renames: Vec::new(),
        };
        let mut pieces = piece::PieceManager::new(&meta).unwrap();

        let stats = resume_from_storage(
            &mut pieces,
            &mut storage,
            meta.info.piece_length,
            &spans,
            Some(&resume),
        )
        .unwrap();

        assert_eq!(stats.completed_bytes, 0);
        assert!(!pieces.is_complete());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inactive_delete_paths_apply_persisted_file_renames() {
        let root = temp_path("delete-saved-rename");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let spans = build_file_spans(&meta).unwrap();
        let mut storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        storage.write_at(0, &[0u8; 16]).unwrap();
        storage.flush().unwrap();
        let old_path = storage.file_path(0).unwrap().to_path_buf();
        let renamed_path = old_path.with_file_name("saved-name.bin");
        storage.rename_file(0, &old_path, &renamed_path).unwrap();
        let files = collect_storage_file_stats(&storage, &spans);
        drop(storage);
        save_resume_data(
            &resume_path(&root, meta.info_hash),
            meta.info_hash,
            meta.info.piece_length,
            vec![0x80],
            &[piece::PRIORITY_NORMAL],
            files,
            16,
            0,
            Vec::new(),
            &[(0, "saved-name.bin".to_string())],
        )
        .unwrap();
        let request = TorrentRequest {
            id: 43,
            source: TorrentSource::Bytes(torrent_bytes),
            download_dir: root.clone(),
            preallocate: false,
            initial_label: String::new(),
        };
        let (_, _, paths) =
            delete_info_from_request(&request, &[(0, "saved-name.bin".to_string())]).unwrap();
        assert_eq!(paths, vec![renamed_path]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn queued_delete_imports_legacy_resume_renames_before_tombstone() {
        let root = temp_path("queued-delete-legacy-rename");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let spans = build_file_spans(&meta).unwrap();
        let mut storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        storage.write_at(0, &[0u8; 16]).unwrap();
        let old_path = storage.file_path(0).unwrap().to_path_buf();
        let renamed_path = old_path.with_file_name("queued-renamed.bin");
        storage.rename_file(0, &old_path, &renamed_path).unwrap();
        let files = collect_storage_file_stats(&storage, &spans);
        drop(storage);
        save_resume_data(
            &resume_path(&root, meta.info_hash),
            meta.info_hash,
            meta.info.piece_length,
            vec![0x80],
            &[piece::PRIORITY_NORMAL],
            files,
            16,
            0,
            Vec::new(),
            &[(0, "queued-renamed.bin".to_string())],
        )
        .unwrap();
        let mut queue = VecDeque::from([TorrentRequest {
            id: 44,
            source: TorrentSource::Bytes(torrent_bytes),
            download_dir: root.clone(),
            preallocate: false,
            initial_label: String::new(),
        }]);
        let ui_state = Some(Arc::new(Mutex::new(ui::UiState {
            torrents: vec![ui::UiTorrent {
                id: 44,
                info_hash: hex(&meta.info_hash),
                ..ui::UiTorrent::default()
            }],
            ..ui::UiState::default()
        })));
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        let store = Arc::new(SessionStore::load(&root).unwrap());

        delete_torrent(
            &registry,
            &ui_state,
            &mut queue,
            44,
            true,
            &store,
            &empty_in_flight(),
        )
        .unwrap();

        assert!(queue.is_empty());
        assert!(!renamed_path.exists());
        assert!(!store.contains(meta.info_hash));
        assert!(lock_or_recover(ui_state.as_ref().unwrap())
            .torrents
            .is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inactive_delete_imports_legacy_resume_renames_before_tombstone() {
        let root = temp_path("inactive-delete-legacy-rename");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let spans = build_file_spans(&meta).unwrap();
        let mut storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        storage.write_at(0, &[0u8; 16]).unwrap();
        let old_path = storage.file_path(0).unwrap().to_path_buf();
        let renamed_path = old_path.with_file_name("inactive-renamed.bin");
        storage.rename_file(0, &old_path, &renamed_path).unwrap();
        let files = collect_storage_file_stats(&storage, &spans);
        drop(storage);
        save_resume_data(
            &resume_path(&root, meta.info_hash),
            meta.info_hash,
            meta.info.piece_length,
            vec![0x80],
            &[piece::PRIORITY_NORMAL],
            files,
            16,
            0,
            Vec::new(),
            &[(0, "inactive-renamed.bin".to_string())],
        )
        .unwrap();
        let store = Arc::new(SessionStore::load(&root).unwrap());
        store
            .upsert(
                meta.info_hash,
                "test".to_string(),
                torrent_bytes,
                &root,
                false,
            )
            .unwrap();
        let ui_state = Some(Arc::new(Mutex::new(ui::UiState {
            torrents: vec![ui::UiTorrent {
                id: 45,
                info_hash: hex(&meta.info_hash),
                ..ui::UiTorrent::default()
            }],
            ..ui::UiState::default()
        })));
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        let mut queue = VecDeque::new();

        delete_torrent(
            &registry,
            &ui_state,
            &mut queue,
            45,
            true,
            &store,
            &empty_in_flight(),
        )
        .unwrap();

        assert!(!renamed_path.exists());
        assert!(!store.contains(meta.info_hash));
        assert!(lock_or_recover(ui_state.as_ref().unwrap())
            .torrents
            .is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn handle_upload_request_writes_piece_and_updates_counters() {
        let root = temp_path("upload-request");
        fs::create_dir_all(&root).unwrap();
        SESSION_UPLOADED_BYTES.store(0, Ordering::SeqCst);

        let context = make_test_context(31, &root);
        let block = b"abcdefghijklmnop";
        {
            let mut storage = lock_or_recover(&context.storage);
            storage.write_at(0, block).unwrap();
        }
        {
            let mut pieces = lock_or_recover(&context.pieces);
            pieces.mark_piece_complete(0).unwrap();
        }
        context.upload_manager.register(99);

        let mut out = Vec::new();
        handle_upload_request(
            &mut out,
            &context.pieces,
            &context.storage,
            0,
            0,
            block.len() as u32,
            &context.limits,
            &context.uploaded,
            &context.upload_requests_served,
            &context.upload_manager,
            99,
        )
        .unwrap();

        let mut cursor = Cursor::new(out);
        let message = peer::read_message(&mut cursor).unwrap();
        assert_eq!(
            message,
            peer::Message::Piece {
                index: 0,
                begin: 0,
                block: block.to_vec(),
            }
        );
        assert_eq!(context.uploaded.load(Ordering::SeqCst), block.len() as u64);
        assert_eq!(
            SESSION_UPLOADED_BYTES.load(Ordering::SeqCst),
            block.len() as u64
        );
        assert_eq!(context.upload_requests_served.load(Ordering::SeqCst), 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn port_mapping_status_message_reports_success_and_failure() {
        assert_eq!(
            port_mapping_status_message("nat-pmp", 6881, &Ok(())),
            "mapped nat-pmp on port 6881"
        );
        assert_eq!(
            port_mapping_status_message("upnp", 6881, &Err("no gateway".to_string())),
            "failed upnp on port 6881: no gateway"
        );
    }

    #[test]
    fn active_peer_count_updates_target_torrent_from_handshaken_counter() {
        let state = Arc::new(Mutex::new(ui::UiState::default()));
        let ui_state = Some(Arc::clone(&state));
        let active_peers = Arc::new(AtomicUsize::new(3));

        update_ui(&ui_state, |state| {
            state.current_id = Some(7);
            state.active_peers = 0;
            state.torrents.push(ui::UiTorrent {
                id: 7,
                name: "selected".to_string(),
                active_peers: 0,
                ..Default::default()
            });
            state.torrents.push(ui::UiTorrent {
                id: 8,
                name: "other".to_string(),
                active_peers: 9,
                ..Default::default()
            });
        });

        set_active_peer_count(&ui_state, 7, &active_peers);

        let state = state.lock().unwrap();
        assert_eq!(state.active_peers, 3);
        assert_eq!(state.torrents[0].active_peers, 3);
        assert_eq!(state.torrents[1].active_peers, 9);
    }

    #[test]
    fn abandon_inflight_resets_complete_but_unverified_piece_buffer() {
        let root = temp_path("abandon-unverified");
        fs::create_dir_all(&root).unwrap();
        let context = make_test_context(32, &root);
        {
            let mut pieces = lock_or_recover(&context.pieces);
            pieces.mark_block_complete(0, 0, 16).unwrap();
            assert_eq!(pieces.remaining_blocks(), 0);
            assert!(!pieces.is_piece_complete(0));
        }

        let mut active = piece::PieceBuffer::new(0, 16).unwrap();
        active.add_block(0, b"abcdefghijklmnop").unwrap();
        assert!(active.is_complete());

        let mut active_pieces = HashMap::new();
        active_pieces.insert(0, active);
        let mut pending = Vec::new();
        {
            let mut pieces = lock_or_recover(&context.pieces);
            abandon_inflight(&mut pieces, &mut pending, &active_pieces);
            assert_eq!(pieces.remaining_blocks(), 1);
            assert!(!pieces.is_complete());
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn failed_piece_persistence_keeps_buffer_for_recovery() {
        let mut active_pieces = HashMap::new();
        let mut active = piece::PieceBuffer::new(0, 16).unwrap();
        active.add_block(0, b"abcdefghijklmnop").unwrap();
        active_pieces.insert(0, active);

        let error = persist_active_piece(&mut active_pieces, 0, |_| {
            Err("simulated storage failure".to_string())
        })
        .unwrap_err();

        assert_eq!(error, "simulated storage failure");
        assert!(active_pieces.contains_key(&0));
    }

    #[test]
    fn saturated_piece_buffer_budget_releases_the_piece_reservation() {
        let meta = torrent::parse_torrent(&test_torrent_bytes()).unwrap();
        let pieces = Mutex::new(piece::PieceManager::new(&meta).unwrap());
        let selected = lock_or_recover(&pieces)
            .reserve_piece_for_peer(1, &[0x80], false)
            .unwrap();
        let budgets = piece::PieceBufferBudgets::new(
            Arc::new(piece::PieceBufferBudget::new(0)),
            Arc::new(piece::PieceBufferBudget::new(0)),
        );

        assert!(allocate_reserved_piece_buffer(
            &pieces,
            1,
            selected,
            meta.info.piece_length as u32,
            &budgets,
        )
        .unwrap()
        .is_none());
        assert_eq!(
            lock_or_recover(&pieces).reserve_piece_for_peer(2, &[0x80], false),
            Some(selected)
        );
    }

    #[test]
    fn parse_rate_and_encryption_mode_cover_common_variants() {
        assert_eq!(parse_rate("0").unwrap(), 0);
        assert_eq!(parse_rate("10k").unwrap(), 10 * 1024);
        assert_eq!(parse_rate("2M").unwrap(), 2 * 1024 * 1024);
        assert_eq!(parse_rate("3g").unwrap(), 3 * 1024 * 1024 * 1024);
        assert_eq!(parse_rate(" unlimited ").unwrap(), 0);
        assert!(parse_rate("1x").is_err());
        assert!(parse_rate("").is_err());
        assert!(parse_rate(&format!("{}g", u64::MAX)).is_err());

        assert_eq!(
            parse_encryption_mode("disable").unwrap(),
            EncryptionMode::Disable
        );
        assert_eq!(
            parse_encryption_mode("prefer").unwrap(),
            EncryptionMode::Prefer
        );
        assert_eq!(
            parse_encryption_mode("force").unwrap(),
            EncryptionMode::Require
        );
        assert!(parse_encryption_mode("unknown").is_err());
    }

    #[test]
    fn create_torrent_streams_files_across_piece_boundaries() {
        let root = temp_path("create-streaming");
        let source = root.join("source");
        let output = root.join("created.torrent");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.bin"), b"abc").unwrap();
        fs::write(source.join("b.bin"), b"def").unwrap();

        create_torrent(&source, "https://tracker.example/announce", &output, 4).unwrap();

        let meta = torrent::parse_torrent(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(meta.info.total_length(), 6);
        assert_eq!(meta.info.files.len(), 2);
        assert_eq!(
            meta.info.pieces,
            vec![sha1::sha1(b"abcd"), sha1::sha1(b"ef")]
        );
        assert!(create_torrent(&source, "https://tracker.example/announce", &output, 0,).is_err());
        assert!(create_torrent(
            &source,
            "https://tracker.example/announce\r\nInjected: yes",
            &output,
            4,
        )
        .is_err());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_torrent_refuses_output_inside_source_tree() {
        let root = temp_path("create-output-inside-source");
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("payload.bin"), b"payload").unwrap();
        let output = source.join("nested").join("created.torrent");

        let error =
            create_torrent(&source, "https://tracker.example/announce", &output, 4).unwrap_err();

        assert!(error.contains("outside the source directory"));
        assert!(!output.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn move_completed_refuses_to_overwrite_existing_data() {
        let root = temp_path("move-existing");
        let source_dir = root.join("source");
        let destination_dir = root.join("destination");
        let source = source_dir.join("payload.bin");
        let torrent_path = root.join("payload.torrent");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&destination_dir).unwrap();
        fs::write(&source, b"new-data").unwrap();
        fs::write(destination_dir.join("payload.bin"), b"existing-data").unwrap();
        create_torrent(
            &source,
            "https://tracker.example/announce",
            &torrent_path,
            4,
        )
        .unwrap();
        let meta = torrent::parse_torrent(&fs::read(torrent_path).unwrap()).unwrap();

        assert!(move_completed_files(&meta, &source_dir, &destination_dir, None).is_err());
        assert_eq!(fs::read(source).unwrap(), b"new-data");
        assert_eq!(
            fs::read(destination_dir.join("payload.bin")).unwrap(),
            b"existing-data"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn move_completed_preserves_a_single_file_rename() {
        let root = temp_path("move-renamed-single");
        let source_dir = root.join("source");
        let destination_dir = root.join("destination");
        let source = source_dir.join("payload.bin");
        let renamed = source_dir.join("custom-name.bin");
        let torrent_path = root.join("payload.torrent");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(&source, b"payload").unwrap();
        create_torrent(
            &source,
            "https://tracker.example/announce",
            &torrent_path,
            4,
        )
        .unwrap();
        let meta = torrent::parse_torrent(&fs::read(torrent_path).unwrap()).unwrap();
        fs::rename(&source, &renamed).unwrap();

        let _ = move_completed_files(&meta, &source_dir, &destination_dir, Some(&renamed)).unwrap();
        assert_eq!(
            fs::read(destination_dir.join("custom-name.bin")).unwrap(),
            b"payload"
        );
        assert!(!destination_dir.join("payload.bin").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parse_rss_rule_arg_handles_feed_urls_and_pattern_only() {
        let (feed, pattern) = parse_rss_rule_arg("http://feed.example/rss:*ubuntu*").unwrap();
        assert_eq!(feed, "http://feed.example/rss");
        assert_eq!(pattern, "*ubuntu*");

        let (feed, pattern) = parse_rss_rule_arg("http://feed.example:8080/rss:*ubuntu*").unwrap();
        assert_eq!(feed, "http://feed.example:8080/rss");
        assert_eq!(pattern, "*ubuntu*");

        let (feed, pattern) = parse_rss_rule_arg("*debian*").unwrap();
        assert_eq!(feed, "");
        assert_eq!(pattern, "*debian*");

        assert!(parse_rss_rule_arg("http://feed:").is_err());
    }

    #[test]
    fn parse_schedule_arg_rejects_zero_interval() {
        let (interval, command) = parse_schedule_arg("60:resume_all").unwrap();
        assert_eq!(interval, 60);
        assert_eq!(command, "resume_all");
        assert!(parse_schedule_arg("0:resume_all").is_err());
        assert!(parse_schedule_arg("abc:resume_all").is_err());
        assert!(parse_schedule_arg("60").is_err());
    }

    #[test]
    fn web_ui_bind_is_loopback_only() {
        assert!(validate_ui_bind_addr("127.0.0.1:8080").is_ok());
        assert!(validate_ui_bind_addr("[::1]:8080").is_ok());
        assert!(validate_ui_bind_addr("0.0.0.0:8080").is_err());
        assert!(validate_ui_bind_addr("192.168.1.10:8080").is_err());
        assert!(validate_ui_bind_addr("localhost:8080").is_err());
    }

    #[test]
    fn peer_listener_is_reachable_over_every_available_ip_family() {
        fn assert_reachable(listeners: &[TcpListener], address: SocketAddr) {
            let _client = TcpStream::connect_timeout(&address, Duration::from_secs(2))
                .unwrap_or_else(|err| panic!("connect to {address}: {err}"));
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                for listener in listeners {
                    match listener.accept() {
                        Ok(_) => return,
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                        Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                        Err(err) => panic!("accept from {address}: {err}"),
                    }
                }
                assert!(
                    Instant::now() < deadline,
                    "listener did not accept {address}"
                );
                thread::sleep(Duration::from_millis(5));
            }
        }

        let listeners = bind_tcp_listeners(0).unwrap();
        for listener in &listeners {
            listener.set_nonblocking(true).unwrap();
        }
        let port = listeners[0].local_addr().unwrap().port();
        assert_reachable(
            &listeners,
            SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port)),
        );

        let ipv6_probe = TcpListener::bind(SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, 0)));
        if ipv6_probe.is_ok() {
            drop(ipv6_probe);
            assert_reachable(
                &listeners,
                SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, port)),
            );
        }
    }

    #[test]
    fn parse_bool_value_handles_truthy_and_falsy_inputs() {
        assert_eq!(parse_bool_value("true"), Some(true));
        assert_eq!(parse_bool_value("YES"), Some(true));
        assert_eq!(parse_bool_value("off"), Some(false));
        assert_eq!(parse_bool_value("0"), Some(false));
        assert_eq!(parse_bool_value("maybe"), None);
    }

    #[test]
    fn collect_trackers_deduplicates_and_respects_private_flag() {
        let private_meta = tracker_meta(true);
        let private_trackers = collect_trackers(&private_meta);
        assert_eq!(private_trackers.http, vec!["http://tracker.local/announce"]);
        assert_eq!(
            private_trackers.udp,
            vec!["udp://tracker.local:6969/announce"]
        );

        let public_meta = tracker_meta(false);
        let public_trackers = collect_trackers(&public_meta);
        assert!(public_trackers
            .http
            .contains(&"http://tracker.local/announce".to_string()));
        assert!(public_trackers
            .udp
            .contains(&"udp://tracker.local:6969/announce".to_string()));
        assert_eq!(public_trackers.http.len(), 1);
        assert_eq!(public_trackers.udp.len(), 1);
        assert!(tracker_set_has_usable_source(&public_trackers, false));

        let udp_only = TrackerSet {
            http: Vec::new(),
            udp: vec!["udp://tracker.local:6969/announce".to_string()],
        };
        assert!(tracker_set_has_usable_source(&udp_only, true));
        assert!(!tracker_set_has_usable_source(&udp_only, false));
    }

    #[cfg(unix)]
    #[test]
    fn private_log_rejects_links_and_repairs_permissions() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = temp_path("private-log");
        fs::create_dir_all(&root).unwrap();
        let log = root.join("rustorrent.log");
        fs::write(&log, b"existing").unwrap();
        fs::set_permissions(&log, fs::Permissions::from_mode(0o644)).unwrap();

        let file = open_private_log_file(&log).unwrap();
        assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        drop(file);

        let symlink_path = root.join("symlink.log");
        symlink(&log, &symlink_path).unwrap();
        assert!(open_private_log_file(&symlink_path).is_err());

        let hardlink_path = root.join("hardlink.log");
        fs::hard_link(&log, &hardlink_path).unwrap();
        assert!(open_private_log_file(&hardlink_path).is_err());

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn private_state_directory_rejects_links_and_repairs_permissions() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = temp_path("private-state");
        fs::create_dir_all(root.join(".rustorrent")).unwrap();
        fs::set_permissions(root.join(".rustorrent"), fs::Permissions::from_mode(0o755)).unwrap();
        ensure_private_state_directory(&root).unwrap();
        assert_eq!(
            fs::metadata(root.join(".rustorrent"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );

        let linked_root = temp_path("linked-state");
        let outside = temp_path("linked-state-outside");
        fs::create_dir_all(&linked_root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, linked_root.join(".rustorrent")).unwrap();
        assert!(ensure_private_state_directory(&linked_root).is_err());
        assert!(write_atomic_file(
            &linked_root.join(".rustorrent").join("session.benc"),
            b"state",
            "session",
            false,
            true,
        )
        .is_err());
        assert!(!outside.join("session.benc").exists());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(linked_root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn bounded_file_reads_reject_oversized_and_non_regular_inputs() {
        let root = temp_path("bounded-read");
        fs::create_dir_all(&root).unwrap();
        let oversized = root.join("oversized.state");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len((MAX_RESUME_STATE_BYTES + 1) as u64).unwrap();
        drop(file);

        let error = read_file_limited(&oversized, MAX_RESUME_STATE_BYTES, true).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(read_file_limited(&root, MAX_RESUME_STATE_BYTES, true).is_err());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_parser_enforces_entry_and_embedded_torrent_limits() {
        let root = temp_path("session-limits");
        let too_many = bencode::encode(&Value::List(vec![Value::Int(0); MAX_SESSION_ENTRIES + 1]));
        assert!(parse_session_entries(&too_many, &root)
            .err()
            .expect("entry count must be rejected")
            .contains("entries"));

        let oversized_entry = bencode::encode(&Value::List(vec![Value::Dict(vec![
            (b"info_hash".to_vec(), Value::Bytes(vec![1; 20])),
            (
                b"torrent".to_vec(),
                Value::Bytes(vec![0; MAX_TORRENT_BYTES + 1]),
            ),
        ])]));
        assert!(parse_session_entries(&oversized_entry, &root)
            .err()
            .expect("oversized torrent must be rejected")
            .contains("metainfo size limit"));
    }

    #[test]
    fn collect_trackers_caps_untrusted_metainfo_entries() {
        let mut meta = tracker_meta(true);
        meta.announce_list = (0..(MAX_TRACKERS_PER_TORRENT + 20))
            .map(|idx| vec![format!("http://tracker{idx}.example/announce").into_bytes()])
            .collect();
        meta.announce =
            Some(format!("http://example/{}", "x".repeat(MAX_TRACKER_URL_LEN + 1)).into_bytes());
        let trackers = collect_trackers(&meta);
        assert_eq!(
            trackers.http.len() + trackers.udp.len(),
            MAX_TRACKERS_PER_TORRENT
        );
        assert!(trackers
            .http
            .iter()
            .all(|url| url.len() <= MAX_TRACKER_URL_LEN));
        assert!(!valid_tracker_url(
            "http://tracker.example/announce\r\nX: y"
        ));
    }

    #[test]
    fn network_url_validation_and_labels_reject_log_injection_and_hide_secrets() {
        let tracker = "https://tracker.example:8443/private-passkey/announce?token=secret";
        assert_eq!(
            safe_network_url_label(tracker),
            "https://tracker.example:8443"
        );
        assert!(valid_tracker_url(tracker));
        assert!(valid_magnet_http_url("https://mirror.example/meta.torrent"));
        for hostile in [
            "https://mirror.example/meta\nforged",
            "https://mirror.example/\u{202e}spoof",
            "https://mirror.example/\u{2066}spoof",
        ] {
            assert!(!valid_magnet_http_url(hostile));
            assert!(!valid_tracker_url(hostile));
        }
        assert!(!safe_network_url_label("https://user@tracker.example/secret").contains("user"));
    }

    #[test]
    fn proxy_mode_centrally_suppresses_udp_tracker_tasks() {
        let trackers = TrackerSet {
            http: Vec::new(),
            udp: vec!["udp://tracker.example:6969/announce".to_string()],
        };
        let (_, pending) = spawn_tracker_announces(
            &trackers,
            [1u8; 20],
            [2u8; 20],
            6881,
            0,
            0,
            1,
            Some("started"),
            50,
            false,
            Some(proxy::ProxyConfig::Socks5 {
                host: "127.0.0.1".to_string(),
                port: 1,
            }),
            Duration::from_secs(1),
        );
        assert_eq!(pending, 0);
    }

    #[test]
    fn pex_payload_roundtrip_includes_v4_and_v6() {
        let peers = vec![
            "127.0.0.1:6881".parse().unwrap(),
            "[2001:db8::1]:51413".parse().unwrap(),
        ];
        let payload = build_ut_pex_payload(&peers, &[]);
        let parsed = parse_ut_pex(&payload).unwrap();
        assert_eq!(parsed, peers);
    }

    #[test]
    fn bitfield_helpers_set_bits_and_validate_bounds() {
        let mut bits = [0u8; 1];
        set_bit(&mut bits, 0).unwrap();
        set_bit(&mut bits, 7).unwrap();
        assert_eq!(bits[0], 0b1000_0001);
        assert!(set_bit(&mut bits, 8).is_err());
    }

    #[test]
    fn delete_torrent_data_removes_only_safe_relative_paths() {
        let root = temp_path("delete-root");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub")).unwrap();
        let inside = root.join("sub").join("file.bin");
        fs::write(&inside, b"x").unwrap();

        let parent = root.parent().unwrap().to_path_buf();
        let outside = parent.join("outside-keep.bin");
        fs::write(&outside, b"y").unwrap();

        assert!(delete_storage_paths(
            &root,
            &[
                inside.clone(),
                outside.clone(),
                PathBuf::from("/absolute/path.bin"),
            ],
        )
        .is_err());

        assert!(!inside.exists());
        assert!(outside.exists());
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn delete_torrent_data_does_not_follow_parent_symlinks() {
        use std::os::unix::fs::symlink;

        let root = temp_path("delete-symlink-root");
        let outside = temp_path("delete-symlink-outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let outside_file = outside.join("keep.bin");
        fs::write(&outside_file, b"keep").unwrap();
        symlink(&outside, root.join("linked")).unwrap();

        assert!(delete_storage_paths(&root, &[root.join("linked/keep.bin")]).is_err());

        assert!(outside_file.exists());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn resume_data_roundtrip_preserves_fields() {
        let path = temp_path("resume").join("state.resume");
        let info_hash = [2u8; 20];
        let bitfield = vec![0b1010_0000];
        let priorities = vec![0, 2, 3];
        let files = vec![
            ResumeFileStat {
                length: 10,
                mtime: 100,
            },
            ResumeFileStat {
                length: 20,
                mtime: 200,
            },
        ];
        let peers = vec!["127.0.0.1:6881".parse().unwrap()];

        save_resume_data(
            &path,
            info_hash,
            16384,
            bitfield.clone(),
            &priorities,
            files,
            1234,
            5678,
            peers.clone(),
            &[(0, "renamed.txt".to_string())],
        )
        .unwrap();
        let loaded = load_resume_data(&path).unwrap();
        assert_eq!(loaded.info_hash, info_hash);
        assert_eq!(loaded.piece_length, 16384);
        assert_eq!(loaded.bitfield, bitfield);
        assert_eq!(loaded.file_priorities, priorities);
        assert_eq!(loaded.downloaded, 1234);
        assert_eq!(loaded.file_renames, vec![(0, "renamed.txt".to_string())]);
        assert_eq!(loaded.uploaded, 5678);
        assert_eq!(loaded.peers, peers);
        let _ = fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn resume_parser_rejects_malformed_numeric_and_collection_entries() {
        fn base_resume_dict() -> Vec<(Vec<u8>, Value)> {
            vec![
                (b"downloaded".to_vec(), Value::Int(0)),
                (b"file_priority".to_vec(), Value::List(Vec::new())),
                (b"files".to_vec(), Value::List(Vec::new())),
                (b"info_hash".to_vec(), Value::Bytes(vec![1; 20])),
                (b"peers".to_vec(), Value::List(Vec::new())),
                (b"piece_length".to_vec(), Value::Int(16_384)),
                (b"pieces".to_vec(), Value::Bytes(Vec::new())),
                (b"uploaded".to_vec(), Value::Int(0)),
            ]
        }

        fn assert_rejected(key: &[u8], replacement: Value, expected: &str) {
            let mut dict = base_resume_dict();
            let value = dict
                .iter_mut()
                .find(|(candidate, _)| candidate.as_slice() == key)
                .map(|(_, value)| value)
                .unwrap();
            *value = replacement;
            let error = parse_resume_data(&bencode::encode(&Value::Dict(dict))).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        assert_rejected(b"piece_length", Value::Int(-1), "piece length");
        assert_rejected(
            b"file_priority",
            Value::List(vec![Value::Int(piece::PRIORITY_HIGH as i64 + 1)]),
            "priority",
        );
        assert_rejected(
            b"files",
            Value::List(vec![Value::Dict(vec![
                (b"length".to_vec(), Value::Int(-1)),
                (b"mtime".to_vec(), Value::Int(0)),
            ])]),
            "length",
        );
        assert_rejected(
            b"peers",
            Value::List(vec![Value::Bytes(b"not-an-address".to_vec())]),
            "peer address",
        );
        assert_rejected(b"downloaded", Value::Int(-1), "downloaded counter");

        let mut dict = base_resume_dict();
        dict.push((
            b"file_renames".to_vec(),
            Value::List(vec![Value::Dict(vec![
                (b"index".to_vec(), Value::Int(-1)),
                (b"name".to_vec(), Value::Bytes(b"renamed.bin".to_vec())),
            ])]),
        ));
        assert!(parse_resume_data(&bencode::encode(&Value::Dict(dict)))
            .unwrap_err()
            .contains("rename index"));

        let path = temp_path("resume-invalid-save").join("state.resume");
        assert!(save_resume_data(
            &path,
            [1; 20],
            16_384,
            Vec::new(),
            &[piece::PRIORITY_HIGH + 1],
            Vec::new(),
            0,
            0,
            Vec::new(),
            &[],
        )
        .unwrap_err()
        .contains("priority"));
        assert!(save_resume_data(
            &path,
            [1; 20],
            16_384,
            Vec::new(),
            &[],
            Vec::new(),
            0,
            0,
            Vec::new(),
            &[(0, "../escape".to_string())],
        )
        .unwrap_err()
        .contains("rename"));
        assert!(!path.exists());
    }

    #[test]
    fn resume_save_roundtrips_at_the_parser_structure_limit() {
        let root = temp_path("resume-structure-limit");
        let path = root.join("state.resume");
        // The resume dictionary contributes 19 values including its keys and
        // container values. Each rename contributes a dictionary, two keys,
        // and two scalar values.
        let rename_count = (bencode::MAX_VALUES - 19) / 5;
        let mut renames = (0..rename_count)
            .map(|index| (index, "x".to_string()))
            .collect::<Vec<_>>();

        save_resume_data(
            &path,
            [8u8; 20],
            16_384,
            Vec::new(),
            &[],
            Vec::new(),
            0,
            0,
            Vec::new(),
            &renames,
        )
        .unwrap();
        let loaded = load_resume_data(&path).unwrap();
        assert_eq!(loaded.file_renames.len(), rename_count);

        renames.push((rename_count, "x".to_string()));
        let error = save_resume_data(
            &path,
            [8u8; 20],
            16_384,
            Vec::new(),
            &[],
            Vec::new(),
            0,
            0,
            Vec::new(),
            &renames,
        )
        .unwrap_err();
        assert!(error.contains("structure exceeds parser limits"));
        assert_eq!(
            load_resume_data(&path).unwrap().file_renames.len(),
            rename_count
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resume_load_recovers_from_backup_when_primary_is_corrupt() {
        let path = temp_path("resume-recover").join("state.resume");
        let info_hash = [4u8; 20];
        save_resume_data(
            &path,
            info_hash,
            16384,
            vec![0b1000_0000],
            &[1, 2],
            vec![ResumeFileStat {
                length: 8,
                mtime: 9,
            }],
            100,
            50,
            vec!["127.0.0.1:6881".parse().unwrap()],
            &[],
        )
        .unwrap();
        let backup_path = sidecar_path(&path, ".bak");
        fs::copy(&path, &backup_path).unwrap();
        fs::write(&path, b"corrupt").unwrap();

        let loaded = load_resume_data_with_recovery(&path).expect("expected backup recovery");
        assert_eq!(loaded.info_hash, info_hash);
        let restored = load_resume_data(&path).expect("restored primary should parse");
        assert_eq!(restored.info_hash, info_hash);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&backup_path);
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn session_save_writes_bencode_file() {
        let path = temp_path("session").join("session.benc");
        let mut entries = HashMap::new();
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        entries.insert(
            info_hash,
            SessionEntry {
                info_hash,
                name: "demo".to_string(),
                torrent_bytes,
                download_dir: PathBuf::from("/tmp/downloads"),
                preallocate: true,
                label: String::new(),
                completion_state: CompletionState::None,
                completion_move_dir: None,
                pending_delete: false,
                file_renames: Vec::new(),
                pending_file_rename: None,
            },
        );
        save_session(&path, &entries).unwrap();

        let bytes = fs::read(&path).unwrap();
        let value = bencode::parse(&bytes).unwrap();
        match value {
            Value::List(items) => assert_eq!(items.len(), 1),
            _ => panic!("expected list"),
        }

        let _ = fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn session_save_roundtrips_near_the_parser_structure_limit() {
        let root = temp_path("session-structure-limit");
        let path = session_path(&root);
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        // The outer list and entry dictionary plus seven key/value pairs use
        // 16 values. Each persisted rename uses five more values.
        let rename_count = (bencode::MAX_VALUES - 16) / 5;
        let entry = SessionEntry {
            info_hash,
            name: "demo".to_string(),
            torrent_bytes,
            download_dir: root.clone(),
            preallocate: false,
            label: String::new(),
            completion_state: CompletionState::None,
            completion_move_dir: None,
            pending_delete: false,
            file_renames: (0..rename_count)
                .map(|index| (index, "x".to_string()))
                .collect(),
            pending_file_rename: None,
        };
        let mut entries = HashMap::from([(info_hash, entry)]);

        save_session(&path, &entries).unwrap();
        let loaded = SessionStore::load(&root).unwrap();
        assert_eq!(
            loaded.get(info_hash).unwrap().file_renames.len(),
            rename_count
        );

        entries
            .get_mut(&info_hash)
            .unwrap()
            .file_renames
            .push((rename_count, "x".to_string()));
        let error = save_session(&path, &entries).unwrap_err();
        assert!(error.contains("structure exceeds parser limits"));
        assert_eq!(
            SessionStore::load(&root)
                .unwrap()
                .get(info_hash)
                .unwrap()
                .file_renames
                .len(),
            rename_count
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_load_fails_closed_when_primary_and_backup_are_invalid() {
        let root = temp_path("session-fail-closed");
        ensure_private_state_directory(&root).unwrap();
        let path = session_path(&root);
        write_atomic_file(&path, b"invalid primary", "test session", false, true).unwrap();
        write_atomic_file(
            &sidecar_path(&path, ".bak"),
            b"invalid backup",
            "test session backup",
            false,
            true,
        )
        .unwrap();

        let error = match SessionStore::load(&root) {
            Ok(_) => panic!("invalid durable session state must not be ignored"),
            Err(error) => error,
        };
        assert!(error.contains("cannot load session state"));
        assert!(error.contains("backup invalid"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_load_recovers_from_backup_when_primary_is_corrupt() {
        let root = temp_path("session-recover");
        let path = session_path(&root);
        let mut entries = HashMap::new();
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        entries.insert(
            info_hash,
            SessionEntry {
                info_hash,
                name: "demo".to_string(),
                torrent_bytes,
                download_dir: root.clone(),
                preallocate: false,
                label: String::new(),
                completion_state: CompletionState::None,
                completion_move_dir: None,
                pending_delete: false,
                file_renames: Vec::new(),
                pending_file_rename: None,
            },
        );
        save_session(&path, &entries).unwrap();
        let backup_path = sidecar_path(&path, ".bak");
        let saved = fs::read(&path).unwrap();
        write_atomic_file(&backup_path, &saved, "test session backup", false, true).unwrap();
        write_atomic_file(&path, b"corrupt", "test session", false, true).unwrap();

        let store = Arc::new(SessionStore::load(&root).unwrap());
        assert!(store.contains(info_hash));
        let loaded_bytes = fs::read(&path).unwrap();
        assert!(bencode::parse(&loaded_bytes).is_ok());

        fs::remove_file(&path).unwrap();
        let missing_primary_store = SessionStore::load(&root).unwrap();
        assert!(missing_primary_store.contains(info_hash));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn completion_actions_require_a_real_transition() {
        assert_eq!(
            completion_action(CompletionState::None, true, true, false),
            CompletionAction::MarkDone
        );
        assert_eq!(
            completion_action(CompletionState::None, false, true, false),
            CompletionAction::RunScript
        );
        assert_eq!(
            completion_action(CompletionState::None, false, true, true),
            CompletionAction::Move
        );
        assert_eq!(
            completion_action(CompletionState::Pending, true, true, true),
            CompletionAction::Move
        );
        assert_eq!(
            completion_action(CompletionState::Done, false, true, true),
            CompletionAction::None
        );
        assert_eq!(
            completion_action(CompletionState::None, false, false, true),
            CompletionAction::None
        );
    }

    #[test]
    fn durable_claims_reject_same_single_path_and_keep_tombstones_owned() {
        let root = temp_path("storage-claim-single");
        let payload = root.join("payload");
        let store = SessionStore::load(&root).unwrap();
        let first =
            claim_test_torrent(&store, single_torrent_bytes(b"same.bin", b'a'), &payload).unwrap();

        assert!(
            claim_test_torrent(&store, single_torrent_bytes(b"same.bin", b'b'), &payload,)
                .unwrap_err()
                .contains("conflicts with torrent")
        );
        claim_test_torrent(&store, single_torrent_bytes(b"sibling.bin", b'c'), &payload).unwrap();

        {
            let _operation = store.lock_operation();
            store.begin_delete(first).unwrap();
        }
        assert!(
            claim_test_torrent(&store, single_torrent_bytes(b"same.bin", b'd'), &payload,).is_err()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn durable_claims_reject_tree_and_file_tree_overlaps() {
        let root = temp_path("storage-claim-tree");
        let payload = root.join("payload");
        let store = SessionStore::load(&root).unwrap();
        claim_test_torrent(&store, multifile_torrent_bytes(b"bundle", b'a'), &payload).unwrap();

        assert!(
            claim_test_torrent(&store, multifile_torrent_bytes(b"bundle", b'b'), &payload,)
                .is_err()
        );
        assert!(claim_test_torrent(
            &store,
            single_torrent_bytes(b"inside.bin", b'c'),
            &payload.join("bundle"),
        )
        .is_err());
        claim_test_torrent(&store, multifile_torrent_bytes(b"sibling", b'd'), &payload).unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rename_and_completion_destinations_are_reserved_before_mutation() {
        let root = temp_path("storage-claim-transitions");
        let payload = root.join("payload");
        let destination = root.join("completed");
        let store = SessionStore::load(&root).unwrap();
        let rename_source =
            claim_test_torrent(&store, single_torrent_bytes(b"source.bin", b'a'), &payload)
                .unwrap();
        claim_test_torrent(
            &store,
            single_torrent_bytes(b"reserved.bin", b'b'),
            &payload,
        )
        .unwrap();
        let move_source =
            claim_test_torrent(&store, single_torrent_bytes(b"move.bin", b'c'), &payload).unwrap();
        claim_test_torrent(
            &store,
            single_torrent_bytes(b"move.bin", b'd'),
            &destination,
        )
        .unwrap();

        {
            let _operation = store.lock_operation();
            assert!(store
                .begin_file_rename(rename_source, 0, "reserved.bin")
                .is_err());
            assert!(store
                .get(rename_source)
                .unwrap()
                .pending_file_rename
                .is_none());
        }
        {
            let _operation = store.lock_operation();
            assert!(store
                .begin_completion(move_source, Some(&destination))
                .is_err());
            assert_eq!(
                store.get(move_source).unwrap().completion_state,
                CompletionState::None
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_upsert_preserves_label_and_completion_state() {
        let root = temp_path("session-upsert-preserve");
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(
                info_hash,
                "old".to_string(),
                torrent_bytes.clone(),
                &root,
                false,
            )
            .unwrap();
        store.set_label(info_hash, "linux").unwrap();
        let intended_move = root.join("completed");
        assert!(store
            .begin_completion(info_hash, Some(&intended_move))
            .unwrap());

        let new_dir = root.join("new-dir");
        store
            .upsert(info_hash, "new".to_string(), torrent_bytes, &new_dir, true)
            .unwrap();

        let entry = store.get(info_hash).unwrap();
        assert_eq!(entry.name, "new");
        assert_eq!(entry.label, "linux");
        assert_eq!(entry.completion_state, CompletionState::Pending);
        assert_eq!(entry.completion_move_dir, Some(intended_move.clone()));
        assert_eq!(entry.download_dir, new_dir);
        let reloaded = SessionStore::load(&root).unwrap().get(info_hash).unwrap();
        assert_eq!(reloaded.label, "linux");
        assert_eq!(reloaded.completion_state, CompletionState::Pending);
        assert_eq!(reloaded.completion_move_dir, Some(intended_move));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_tombstone_and_rename_journal_survive_upsert_roundtrip() {
        let root = temp_path("session-delete-roundtrip");
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(
                info_hash,
                "old".to_string(),
                torrent_bytes.clone(),
                &root,
                false,
            )
            .unwrap();
        store
            .import_file_renames_if_empty(info_hash, &[(0, "committed.bin".to_string())])
            .unwrap();
        store
            .begin_file_rename(info_hash, 0, "pending.bin")
            .unwrap();

        // A pending rename must be resolved before deletion can begin.
        assert!(store.begin_delete(info_hash).is_err());
        let pending = PendingFileRename {
            index: 0,
            target: "pending.bin".to_string(),
        };
        store.cancel_file_rename(info_hash, &pending).unwrap();
        store.begin_delete(info_hash).unwrap();
        store
            .upsert(
                info_hash,
                "new".to_string(),
                torrent_bytes,
                &root.join("new"),
                true,
            )
            .unwrap();

        let entry = SessionStore::load(&root).unwrap().get(info_hash).unwrap();
        assert!(entry.pending_delete);
        assert_eq!(entry.file_renames, vec![(0, "committed.bin".to_string())]);
        assert_eq!(entry.name, "new");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn begin_delete_rolls_back_memory_when_save_fails() {
        let root = temp_path("session-delete-save-failure");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session-target-is-directory");
        fs::create_dir(&path).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let entry = SessionEntry {
            info_hash,
            name: "demo".to_string(),
            torrent_bytes,
            download_dir: root.clone(),
            preallocate: false,
            label: String::new(),
            completion_state: CompletionState::None,
            completion_move_dir: None,
            pending_delete: false,
            file_renames: Vec::new(),
            pending_file_rename: None,
        };
        let store = SessionStore {
            path,
            entries: Mutex::new(HashMap::from([(info_hash, entry)])),
            operations: Mutex::new(()),
        };

        assert!(store.begin_delete(info_hash).is_err());
        assert!(!store.get(info_hash).unwrap().pending_delete);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn post_publish_directory_sync_failure_is_not_reported_as_rollback_safe() {
        assert!(finish_atomic_publish(
            Err("simulated directory sync failure".to_string()),
            "test journal",
        )
        .is_ok());
    }

    #[test]
    fn startup_retries_tombstone_without_queueing_it() {
        let root = temp_path("startup-delete-retry");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        let data_path = storage.file_path(0).unwrap().to_path_buf();
        drop(storage);
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(
                meta.info_hash,
                "test".to_string(),
                torrent_bytes,
                &root,
                false,
            )
            .unwrap();
        store.begin_delete(meta.info_hash).unwrap();
        let mut queue = VecDeque::new();
        let ui = Some(Arc::new(Mutex::new(ui::UiState::default())));
        let mut next_id = 1;

        restore_session_entries(&store, &mut queue, &ui, &mut next_id);

        assert!(queue.is_empty());
        assert!(!data_path.exists());
        assert!(!store.contains(meta.info_hash));
        assert!(lock_or_recover(ui.as_ref().unwrap()).torrents.is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn startup_retains_failed_tombstone_as_nonqueued_ui_entry() {
        let root = temp_path("startup-delete-failure");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let data_path = storage::data_paths(&meta, &root).unwrap().remove(0);
        fs::create_dir_all(&data_path).unwrap();
        let store = Arc::new(SessionStore::load(&root).unwrap());
        store
            .upsert(
                meta.info_hash,
                "test".to_string(),
                torrent_bytes,
                &root,
                false,
            )
            .unwrap();
        store.begin_delete(meta.info_hash).unwrap();
        let mut queue = VecDeque::new();
        let ui = Some(Arc::new(Mutex::new(ui::UiState::default())));
        let mut next_id = 1;

        restore_session_entries(&store, &mut queue, &ui, &mut next_id);

        assert!(queue.is_empty());
        assert!(store.get(meta.info_hash).unwrap().pending_delete);
        let state = lock_or_recover(ui.as_ref().unwrap());
        assert_eq!(state.torrents.len(), 1);
        assert_eq!(state.torrents[0].info_hash, hex(&meta.info_hash));
        assert_eq!(state.torrents[0].status, "delete failed");
        assert!(!state.torrents[0].last_error.is_empty());
        drop(state);

        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        assert!(delete_torrent(
            &registry,
            &ui,
            &mut queue,
            1,
            true,
            &store,
            &empty_in_flight(),
        )
        .is_err());
        assert!(store.get(meta.info_hash).unwrap().pending_delete);
        assert_eq!(
            lock_or_recover(ui.as_ref().unwrap()).torrents[0].status,
            "delete failed"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pending_delete_is_idempotent_when_files_are_already_absent() {
        let root = temp_path("delete-not-found");
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(
                meta.info_hash,
                "test".to_string(),
                torrent_bytes,
                &root,
                false,
            )
            .unwrap();
        store.begin_delete(meta.info_hash).unwrap();

        retry_pending_delete(&store, meta.info_hash).unwrap();
        assert!(!store.contains(meta.info_hash));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pending_file_rename_recovers_before_storage_open() {
        let root = temp_path("rename-journal-recovery");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        let original = storage.file_path(0).unwrap().to_path_buf();
        drop(storage);
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(
                meta.info_hash,
                "test".to_string(),
                torrent_bytes,
                &root,
                false,
            )
            .unwrap();
        store
            .begin_file_rename(meta.info_hash, 0, "renamed.bin")
            .unwrap();
        let entry = store.get(meta.info_hash).unwrap();

        let renames = reconcile_pending_file_rename(&meta, &root, &store, &entry).unwrap();
        let renamed = original.with_file_name("renamed.bin");
        assert!(!original.exists());
        assert!(renamed.exists());
        assert_eq!(renames, vec![(0, "renamed.bin".to_string())]);
        assert!(store
            .get(meta.info_hash)
            .unwrap()
            .pending_file_rename
            .is_none());

        // Simulate a crash after the second physical rename but before its
        // journal commit. Recovery must commit the already-moved path.
        store
            .begin_file_rename(meta.info_hash, 0, "renamed-again.bin")
            .unwrap();
        let renamed_again = original.with_file_name("renamed-again.bin");
        fs::rename(&renamed, &renamed_again).unwrap();
        let entry = store.get(meta.info_hash).unwrap();
        let renames = reconcile_pending_file_rename(&meta, &root, &store, &entry).unwrap();
        assert_eq!(renames, vec![(0, "renamed-again.bin".to_string())]);
        assert!(renamed_again.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pending_file_rename_refuses_ambiguous_or_missing_paths() {
        let root = temp_path("rename-journal-ambiguous");
        fs::create_dir_all(&root).unwrap();
        let torrent_bytes = test_torrent_bytes();
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let storage =
            storage::Storage::new(&meta, &root, storage::StorageOptions::default()).unwrap();
        let original = storage.file_path(0).unwrap().to_path_buf();
        drop(storage);
        let target = original.with_file_name("ambiguous.bin");
        fs::copy(&original, &target).unwrap();
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(
                meta.info_hash,
                "test".to_string(),
                torrent_bytes,
                &root,
                false,
            )
            .unwrap();
        store
            .begin_file_rename(meta.info_hash, 0, "ambiguous.bin")
            .unwrap();

        let entry = store.get(meta.info_hash).unwrap();
        assert!(reconcile_pending_file_rename(&meta, &root, &store, &entry)
            .unwrap_err()
            .contains("both old and new"));
        assert!(original.exists());
        assert!(target.exists());
        assert!(store
            .get(meta.info_hash)
            .unwrap()
            .pending_file_rename
            .is_some());

        fs::remove_file(&original).unwrap();
        fs::remove_file(&target).unwrap();
        let entry = store.get(meta.info_hash).unwrap();
        assert!(reconcile_pending_file_rename(&meta, &root, &store, &entry)
            .unwrap_err()
            .contains("neither old nor new"));
        assert!(store
            .get(meta.info_hash)
            .unwrap()
            .pending_file_rename
            .is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn session_paths_roundtrip_non_utf8_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let root = temp_path("session-non-utf8-path");
        let raw = std::ffi::OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xff]);
        let download_dir = PathBuf::from(raw);
        let move_raw = std::ffi::OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xfe]);
        let move_dir = PathBuf::from(move_raw);
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(
                info_hash,
                "demo".to_string(),
                torrent_bytes,
                &download_dir,
                false,
            )
            .unwrap();
        store.begin_completion(info_hash, Some(&move_dir)).unwrap();

        let entry = SessionStore::load(&root).unwrap().get(info_hash).unwrap();
        assert_eq!(entry.download_dir, download_dir);
        assert_eq!(entry.completion_move_dir, Some(move_dir));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_session_without_completion_state_loads_as_none() {
        let root = temp_path("legacy-session-state");
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let encoded = bencode::encode(&Value::List(vec![Value::Dict(vec![
            (b"info_hash".to_vec(), Value::Bytes(info_hash.to_vec())),
            (b"name".to_vec(), Value::Bytes(b"legacy".to_vec())),
            (b"torrent".to_vec(), Value::Bytes(torrent_bytes)),
        ])]));
        let entries = parse_session_entries(&encoded, &root).unwrap();
        assert_eq!(
            entries.get(&info_hash).unwrap().completion_state,
            CompletionState::None
        );
    }

    #[test]
    fn completion_move_commit_updates_directory_and_state_together() {
        let root = temp_path("completion-move-commit");
        let torrent_bytes = test_torrent_bytes();
        let info_hash = torrent::parse_torrent(&torrent_bytes).unwrap().info_hash;
        let destination = root.join("destination");
        let store = SessionStore::load(&root).unwrap();
        store
            .upsert(info_hash, "demo".to_string(), torrent_bytes, &root, false)
            .unwrap();
        assert!(store
            .begin_completion(info_hash, Some(&destination))
            .unwrap());
        assert!(store
            .commit_completion_move(info_hash, &destination)
            .unwrap());

        let entry = SessionStore::load(&root).unwrap().get(info_hash).unwrap();
        assert_eq!(entry.download_dir, destination);
        assert_eq!(entry.completion_state, CompletionState::Done);
        assert_eq!(entry.completion_move_dir, None);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn atomic_no_replace_move_preserves_a_racing_destination() {
        let root = temp_path("completion-no-replace");
        fs::create_dir_all(&root).unwrap();

        let source = root.join("source.bin");
        let destination = root.join("destination.bin");
        fs::write(&source, b"source payload").unwrap();
        fs::write(&destination, b"independent payload").unwrap();

        let error = rename_path_no_overwrite(&source, &destination, false).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&source).unwrap(), b"source payload");
        assert_eq!(fs::read(&destination).unwrap(), b"independent payload");

        let source_dir = root.join("source-dir");
        let destination_dir = root.join("destination-dir");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&destination_dir).unwrap();
        fs::write(source_dir.join("source.txt"), b"source").unwrap();
        fs::write(destination_dir.join("destination.txt"), b"destination").unwrap();

        let error = rename_path_no_overwrite(&source_dir, &destination_dir, true).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(source_dir.join("source.txt").exists());
        assert_eq!(
            fs::read(destination_dir.join("destination.txt")).unwrap(),
            b"destination"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_session_commit_rolls_back_completed_move() {
        let root = temp_path("completion-move-rollback");
        let source = root.join("source.bin");
        let destination = root.join("destination.bin");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"payload").unwrap();
        move_path_no_overwrite(&source, &destination).unwrap();
        let completed_move = CompletedMove {
            source: source.clone(),
            destination: destination.clone(),
        };

        let error = commit_completed_move(Some(&completed_move), || {
            Err("simulated session failure".to_string())
        })
        .unwrap_err();
        assert!(error.contains("rolled back"));
        assert_eq!(fs::read(&source).unwrap(), b"payload");
        assert!(!destination.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pending_move_reconciles_only_a_verified_destination() {
        let root = temp_path("completion-move-reconcile");
        let source_dir = root.join("source");
        let destination_dir = root.join("destination");
        let source = source_dir.join("payload.bin");
        let torrent_path = root.join("payload.torrent");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&destination_dir).unwrap();
        fs::write(&source, b"payload").unwrap();
        create_torrent(
            &source,
            "https://tracker.example/announce",
            &torrent_path,
            4,
        )
        .unwrap();
        let meta = torrent::parse_torrent(&fs::read(&torrent_path).unwrap()).unwrap();
        let completed_move =
            completed_move_paths(&meta, &source_dir, &destination_dir, None).unwrap();

        fs::write(&completed_move.destination, b"unrelated").unwrap();
        assert!(completion_move_recovery(&meta, &destination_dir, &[], &completed_move,).is_err());
        fs::remove_file(&completed_move.destination).unwrap();
        fs::copy(&source, &completed_move.destination).unwrap();
        assert_eq!(
            completion_move_recovery(&meta, &destination_dir, &[], &completed_move).unwrap(),
            CompletionMoveRecovery::AdoptDestination {
                remove_source: true
            }
        );
        fs::remove_file(&source).unwrap();
        assert_eq!(
            completion_move_recovery(&meta, &destination_dir, &[], &completed_move).unwrap(),
            CompletionMoveRecovery::AdoptDestination {
                remove_source: false
            }
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn published_multifile_recovery_deletes_only_exact_source_files() {
        let root = temp_path("completion-move-exact-source-cleanup");
        let source_dir = root.join("source");
        let destination_dir = root.join("destination");
        let bundle = source_dir.join("bundle");
        let torrent_path = root.join("bundle.torrent");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("payload.bin"), b"payload").unwrap();
        create_torrent(
            &bundle,
            "https://tracker.example/announce",
            &torrent_path,
            4,
        )
        .unwrap();
        let meta = torrent::parse_torrent(&fs::read(torrent_path).unwrap()).unwrap();
        fs::write(bundle.join("unrelated.txt"), b"keep").unwrap();
        fs::create_dir_all(destination_dir.join("bundle")).unwrap();
        fs::copy(
            bundle.join("payload.bin"),
            destination_dir.join("bundle/payload.bin"),
        )
        .unwrap();
        let completed_move =
            completed_move_paths(&meta, &source_dir, &destination_dir, None).unwrap();
        assert_eq!(
            completion_move_recovery(&meta, &destination_dir, &[], &completed_move).unwrap(),
            CompletionMoveRecovery::AdoptDestination {
                remove_source: true
            }
        );
        let source_paths = storage::data_paths(&meta, &source_dir).unwrap();
        delete_storage_paths(&source_dir, &source_paths).unwrap();
        assert_eq!(fs::read(bundle.join("unrelated.txt")).unwrap(), b"keep");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inbound_handler_slots_enforce_capacity() {
        assert_eq!(inbound_handler_slots(0), 0);
        assert_eq!(inbound_handler_slots(1), MIN_INBOUND_HANDLER_SLOTS);
        assert_eq!(inbound_handler_slots(600), MAX_INBOUND_HANDLER_SLOTS);

        let inbound = InboundConfig {
            encryption: EncryptionMode::Disable,
            ip_filter: None,
            max_handlers: Arc::new(AtomicUsize::new(2)),
            active_handlers: Arc::new(AtomicUsize::new(0)),
        };
        let guard_a = inbound.try_acquire_handler_slot().unwrap();
        let guard_b = inbound.try_acquire_handler_slot().unwrap();
        assert!(inbound.try_acquire_handler_slot().is_none());
        drop(guard_a);
        assert!(inbound.try_acquire_handler_slot().is_some());
        drop(guard_b);
    }

    #[test]
    fn peer_queue_filters_invalid_public_peers() {
        let mut queue = PeerQueue::new(None);
        let added = queue.enqueue_with_source(
            [
                "0.0.0.0:0".parse().unwrap(),
                "127.0.0.1:6881".parse().unwrap(),
                "10.0.0.2:6881".parse().unwrap(),
                "169.254.1.9:6881".parse().unwrap(),
                "203.0.113.10:6881".parse().unwrap(),
                "8.8.8.8:6881".parse().unwrap(),
            ],
            PeerSource::Tracker,
        );
        assert_eq!(added, 1);
        assert_eq!(queue.known_len(), 1);
        assert_eq!(queue.pop(), Some("8.8.8.8:6881".parse().unwrap()));
    }

    #[test]
    fn peer_queue_keeps_private_lpd_peers() {
        let mut queue = PeerQueue::new(None);
        let added = queue.enqueue_with_source(
            [
                "10.0.0.2:6881".parse().unwrap(),
                "192.168.1.5:51413".parse().unwrap(),
            ],
            PeerSource::Lpd,
        );
        assert_eq!(added, 2);
        assert_eq!(queue.known_len(), 2);
    }

    #[test]
    fn peer_queue_normalizes_mapped_ipv4_and_rejects_special_ipv6_routes() {
        let mut queue = PeerQueue::new(None);
        let added = queue.enqueue_with_source(
            [
                "[::ffff:127.0.0.1]:6881".parse().unwrap(),
                "[::ffff:192.168.1.1]:6881".parse().unwrap(),
                "[::ffff:169.254.1.1]:6881".parse().unwrap(),
                "[::ffff:203.0.113.9]:6881".parse().unwrap(),
                "[64:ff9b:1::c0a8:1]:6881".parse().unwrap(),
                "[::ffff:8.8.8.8]:6881".parse().unwrap(),
            ],
            PeerSource::Tracker,
        );
        assert_eq!(added, 1);
        assert_eq!(queue.pop(), Some("8.8.8.8:6881".parse().unwrap()));

        let mut lpd = PeerQueue::new(None);
        assert_eq!(
            lpd.enqueue_with_source(
                ["[::ffff:192.168.1.2]:6881".parse().unwrap()],
                PeerSource::Lpd,
            ),
            1
        );
        assert_eq!(lpd.pop(), Some("192.168.1.2:6881".parse().unwrap()));
    }

    #[test]
    fn metadata_discovery_filters_tracker_dht_and_magnet_scope_before_connecting() {
        for unsafe_peer in [
            "127.0.0.1:6881",
            "10.0.0.1:6881",
            "169.254.1.1:6881",
            "192.0.0.1:6881",
            "[::ffff:192.168.1.1]:6881",
            "[64:ff9b:1::c0a8:1]:6881",
            "[fc00::1]:6881",
            "[fe80::1]:6881",
        ] {
            assert!(
                safe_metadata_peer(unsafe_peer.parse().unwrap(), PeerSource::Tracker, None,)
                    .is_none()
            );
            assert!(
                safe_metadata_peer(unsafe_peer.parse().unwrap(), PeerSource::Dht, None,).is_none()
            );
            assert!(
                safe_metadata_peer(unsafe_peer.parse().unwrap(), PeerSource::Magnet, None,)
                    .is_none()
            );
        }
        assert_eq!(
            safe_metadata_peer(
                "[::ffff:8.8.8.8]:6881".parse().unwrap(),
                PeerSource::Tracker,
                None,
            ),
            Some("8.8.8.8:6881".parse().unwrap())
        );
        assert_eq!(
            safe_metadata_peer(
                "[::ffff:8.8.8.8]:6881".parse().unwrap(),
                PeerSource::Magnet,
                None,
            ),
            Some("8.8.8.8:6881".parse().unwrap())
        );
    }

    #[test]
    fn peer_queue_rejects_local_lpd_self_addresses() {
        let local_addrs = ["192.168.1.10:6881".parse().unwrap()].into_iter().collect();
        let mut queue = PeerQueue::new_with_local_addrs(None, local_addrs);

        let added = queue.enqueue_with_source(
            [
                "192.168.1.10:6881".parse().unwrap(),
                "192.168.1.20:6881".parse().unwrap(),
            ],
            PeerSource::Lpd,
        );

        assert_eq!(added, 1);
        assert_eq!(queue.known_len(), 1);
        assert_eq!(queue.pop(), Some("192.168.1.20:6881".parse().unwrap()));
    }

    #[test]
    fn peer_retry_policy_does_not_requeue_hard_failures() {
        assert!(is_retryable_peer_error(
            "connect 198.51.100.10:6881 failed: connection timed out"
        ));
        assert!(!is_retryable_peer_error(
            "connect 198.51.100.10:6881 failed: Connection refused (os error 61)"
        ));
        assert!(!is_retryable_peer_error(
            "message read failed: io error: peer closed connection"
        ));
    }

    #[test]
    fn hard_peer_failure_is_banned_from_immediate_requeue() {
        let addr: SocketAddr = "8.8.8.8:6881".parse().unwrap();
        let mut queue = PeerQueue::new(None);
        assert_eq!(queue.enqueue_with_source([addr], PeerSource::Tracker), 1);
        assert_eq!(queue.pop(), Some(addr));

        record_peer_result(
            &mut queue,
            addr,
            &Err("connect 8.8.8.8:6881 failed: Connection refused (os error 61)".to_string()),
        );

        assert_eq!(queue.enqueue_with_source([addr], PeerSource::Tracker), 0);
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn repeated_retryable_peer_failures_eventually_ban_peer() {
        let addr: SocketAddr = "8.8.4.4:6881".parse().unwrap();
        let mut queue = PeerQueue::new(None);
        assert_eq!(queue.enqueue_with_source([addr], PeerSource::Tracker), 1);
        assert_eq!(queue.pop(), Some(addr));

        for _ in 0..MAX_PEER_RETRIES {
            record_peer_result(
                &mut queue,
                addr,
                &Err("connect 8.8.4.4:6881 failed: connection timed out".to_string()),
            );
        }

        assert!(queue.is_banned(addr));
        assert!(!queue.is_deferred(addr));
        assert_eq!(queue.enqueue_with_source([addr], PeerSource::Tracker), 0);
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn matching_peer_id_is_self_connection() {
        let local = [7u8; 20];
        let mut remote = local;
        assert!(is_self_peer_id(&local, &remote));

        remote[19] = 8;
        assert!(!is_self_peer_id(&local, &remote));
    }

    #[test]
    fn request_queue_depth_tracks_peer_rate() {
        assert_eq!(request_queue_depth_for_rate(DEFAULT_PEER_RATE_BPS), 64);
        assert_eq!(
            request_queue_depth_for_rate(piece::BLOCK_LEN as f64 * 20.0),
            40
        );
        assert_eq!(request_queue_depth_for_rate(0.0), 64);
    }

    #[test]
    fn peer_runtime_settings_apply_profile_updates_live_limits() {
        let settings = PeerRuntimeSettings::new(PeerProfile::Balanced, 111, 44, 222, 33);
        let tuning = settings.apply_profile(PeerProfile::Conservative);
        assert_eq!(tuning, PeerProfile::Conservative.tuning());
        assert_eq!(settings.profile(), PeerProfile::Conservative);
        assert_eq!(settings.numwant(), 50);
        assert_eq!(settings.metadata_peer_limit(), 20);
        assert_eq!(settings.max_peers_global(), 80);
        assert_eq!(settings.max_peers_torrent(), 12);
    }

    #[test]
    fn reannounce_intervals_do_not_flood_trackers_on_stalls() {
        let no_peer_secs = std::hint::black_box(NO_PEER_REANNOUNCE_SECS);
        let stall_secs = std::hint::black_box(STALL_REANNOUNCE_SECS);
        assert!(no_peer_secs >= 30);
        assert!(stall_secs >= 30);
    }

    #[test]
    fn path_helpers_place_state_under_rustorrent_dir() {
        let root = Path::new("/tmp/downloads");
        let info_hash = [0xABu8; 20];
        let resume = resume_path(root, info_hash);
        let session = session_path(root);
        assert!(resume.to_string_lossy().contains(".rustorrent"));
        assert!(resume.to_string_lossy().ends_with(".resume"));
        assert_eq!(
            session.strip_prefix(root).unwrap(),
            Path::new(".rustorrent").join("session.benc")
        );
    }

    #[test]
    fn apply_overrides_updates_selected_fields() {
        let cfg = ConfigOverrides {
            download_dir: Some(PathBuf::from("/tmp/alt")),
            preallocate: Some(true),
            ui: Some(true),
            ui_addr: Some("127.0.0.1:9090".to_string()),
            peer_profile: None,
            retry_interval: Some(30),
            numwant: Some(80),
            port: Some(7000),
            enable_utp: Some(false),
            encryption: Some(EncryptionMode::Require),
            blocklist_path: Some(PathBuf::from("/tmp/blocklist.txt")),
            max_peers_global: Some(111),
            max_peers_torrent: Some(22),
            max_active_torrents: Some(3),
            download_rate: Some(10),
            upload_rate: Some(20),
            torrent_download_rate: Some(30),
            torrent_upload_rate: Some(40),
            write_cache_bytes: Some(50),
            geoip_db: None,
        };

        let mut download_dir = PathBuf::from(".");
        let mut preallocate = false;
        let mut ui = false;
        let mut ui_addr = "127.0.0.1:8080".to_string();
        let mut peer_profile = PeerProfile::Balanced;
        let mut retry_interval = 60;
        let mut numwant = 200;
        let mut metadata_peer_limit = PeerProfile::Balanced.tuning().metadata_peer_limit;
        let mut port = 6881;
        let mut enable_utp = true;
        let mut encryption = EncryptionMode::Prefer;
        let mut blocklist_path = None;
        let mut max_peers_global = 200;
        let mut max_peers_torrent = 30;
        let mut max_active_torrents = 4;
        let mut download_rate = 0;
        let mut upload_rate = 0;
        let mut torrent_download_rate = 0;
        let mut torrent_upload_rate = 0;
        let mut write_cache_bytes = 0;
        let mut geoip_path = None;
        apply_overrides(
            &cfg,
            &mut download_dir,
            &mut preallocate,
            &mut ui,
            &mut ui_addr,
            &mut peer_profile,
            &mut retry_interval,
            &mut numwant,
            &mut metadata_peer_limit,
            &mut port,
            &mut enable_utp,
            &mut encryption,
            &mut blocklist_path,
            &mut max_peers_global,
            &mut max_peers_torrent,
            &mut max_active_torrents,
            &mut download_rate,
            &mut upload_rate,
            &mut torrent_download_rate,
            &mut torrent_upload_rate,
            &mut write_cache_bytes,
            &mut geoip_path,
        );

        assert_eq!(download_dir, PathBuf::from("/tmp/alt"));
        assert!(preallocate);
        assert!(ui);
        assert_eq!(ui_addr, "127.0.0.1:9090");
        assert_eq!(peer_profile, PeerProfile::Balanced);
        assert_eq!(retry_interval, 30);
        assert_eq!(numwant, 80);
        assert_eq!(
            metadata_peer_limit,
            PeerProfile::Balanced.tuning().metadata_peer_limit
        );
        assert_eq!(port, 7000);
        assert!(!enable_utp);
        assert_eq!(encryption, EncryptionMode::Require);
        assert_eq!(blocklist_path, Some(PathBuf::from("/tmp/blocklist.txt")));
        assert_eq!(max_peers_global, 111);
        assert_eq!(max_peers_torrent, 22);
        assert_eq!(max_active_torrents, 3);
        assert_eq!(download_rate, 10);
        assert_eq!(upload_rate, 20);
        assert_eq!(torrent_download_rate, 30);
        assert_eq!(torrent_upload_rate, 40);
        assert_eq!(write_cache_bytes, 50);
    }

    #[test]
    fn peer_profile_presets_define_expected_limits() {
        assert_eq!(
            PeerProfile::Conservative.tuning(),
            PeerProfileTuning {
                numwant: 50,
                max_peers_global: 80,
                max_peers_torrent: 12,
                metadata_peer_limit: 20,
            }
        );
        assert_eq!(
            PeerProfile::Balanced.tuning(),
            PeerProfileTuning {
                numwant: 200,
                max_peers_global: 200,
                max_peers_torrent: 30,
                metadata_peer_limit: 80,
            }
        );
        assert_eq!(
            PeerProfile::Aggressive.tuning(),
            PeerProfileTuning {
                numwant: 500,
                max_peers_global: 500,
                max_peers_torrent: 80,
                metadata_peer_limit: 160,
            }
        );
    }

    #[test]
    fn apply_overrides_applies_peer_profile_before_explicit_peer_limits() {
        let cfg = ConfigOverrides {
            peer_profile: Some(PeerProfile::Conservative),
            numwant: Some(90),
            max_peers_global: Some(140),
            max_peers_torrent: Some(20),
            ..ConfigOverrides::default()
        };

        let mut download_dir = PathBuf::from(".");
        let mut preallocate = false;
        let mut ui = false;
        let mut ui_addr = "127.0.0.1:8080".to_string();
        let mut peer_profile = PeerProfile::Balanced;
        let mut retry_interval = 60;
        let mut numwant = PeerProfile::Aggressive.tuning().numwant;
        let mut metadata_peer_limit = PeerProfile::Aggressive.tuning().metadata_peer_limit;
        let mut port = 6881;
        let mut enable_utp = true;
        let mut encryption = EncryptionMode::Prefer;
        let mut blocklist_path = None;
        let mut max_peers_global = PeerProfile::Aggressive.tuning().max_peers_global;
        let mut max_peers_torrent = PeerProfile::Aggressive.tuning().max_peers_torrent;
        let mut max_active_torrents = 4;
        let mut download_rate = 0;
        let mut upload_rate = 0;
        let mut torrent_download_rate = 0;
        let mut torrent_upload_rate = 0;
        let mut write_cache_bytes = 0;
        let mut geoip_path = None;

        apply_overrides(
            &cfg,
            &mut download_dir,
            &mut preallocate,
            &mut ui,
            &mut ui_addr,
            &mut peer_profile,
            &mut retry_interval,
            &mut numwant,
            &mut metadata_peer_limit,
            &mut port,
            &mut enable_utp,
            &mut encryption,
            &mut blocklist_path,
            &mut max_peers_global,
            &mut max_peers_torrent,
            &mut max_active_torrents,
            &mut download_rate,
            &mut upload_rate,
            &mut torrent_download_rate,
            &mut torrent_upload_rate,
            &mut write_cache_bytes,
            &mut geoip_path,
        );

        assert_eq!(peer_profile, PeerProfile::Conservative);
        assert_eq!(numwant, 90);
        assert_eq!(
            metadata_peer_limit,
            PeerProfile::Conservative.tuning().metadata_peer_limit
        );
        assert_eq!(max_peers_global, 140);
        assert_eq!(max_peers_torrent, 20);
    }

    #[test]
    fn load_config_overrides_parses_peer_profile() {
        let root = temp_path("peer-profile-config");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("rustorrent.conf");
        fs::write(
            &path,
            "peer_profile = aggressive\nnumwant = 320\nmax_peers = 420\nmax_peers_torrent = 70\n",
        )
        .unwrap();

        let cfg = load_config_overrides(&path).unwrap();
        assert_eq!(cfg.peer_profile, Some(PeerProfile::Aggressive));
        assert_eq!(cfg.numwant, Some(320));
        assert_eq!(cfg.max_peers_global, Some(420));
        assert_eq!(cfg.max_peers_torrent, Some(70));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_torrent_requeues_stopped_session() {
        let root = temp_path("resume-stopped");
        fs::create_dir_all(&root).unwrap();
        let info = b"d6:lengthi1e4:name4:test12:piece lengthi1e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        let torrent_bytes = wrap_torrent_with_info(info, &[], &[]);
        let meta = torrent::parse_torrent(&torrent_bytes).unwrap();
        let session_store = Arc::new(SessionStore::load(&root).unwrap());
        session_store
            .upsert(
                meta.info_hash,
                "bugonia".to_string(),
                torrent_bytes.clone(),
                &root,
                false,
            )
            .unwrap();
        let ui = Arc::new(Mutex::new(ui::UiState {
            current_id: Some(7),
            torrents: vec![ui::UiTorrent {
                id: 7,
                name: "bugonia".to_string(),
                info_hash: hex(&meta.info_hash),
                download_dir: root.display().to_string(),
                status: "stopped".to_string(),
                download_rate_bps: 99.0,
                upload_rate_bps: 12.0,
                active_peers: 4,
                eta_secs: 42,
                ..ui::UiTorrent::default()
            }],
            ..ui::UiState::default()
        }));
        let ui_state = Some(Arc::clone(&ui));
        let registry: SessionRegistry = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let in_flight: InFlightTorrents = Arc::new(Mutex::new(HashMap::new()));
        let mut queue = VecDeque::new();

        resume_torrent(
            &registry,
            &ui_state,
            &mut queue,
            7,
            &session_store,
            &in_flight,
        )
        .unwrap();

        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].id, 7);
        assert_eq!(
            info_hash_for_source(&queue[0].source).unwrap(),
            meta.info_hash
        );

        let state = lock_or_recover(&ui);
        assert_eq!(state.status, "queued");
        assert_eq!(state.download_rate_bps, 0.0);
        assert_eq!(state.upload_rate_bps, 0.0);
        assert_eq!(state.active_peers, 0);
        assert_eq!(state.eta_secs, 0);
        assert_eq!(state.torrents[0].status, "queued");
        assert_eq!(state.torrents[0].download_rate_bps, 0.0);
        assert_eq!(state.torrents[0].upload_rate_bps, 0.0);
        assert_eq!(state.torrents[0].active_peers, 0);
        assert_eq!(state.torrents[0].eta_secs, 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stop_torrent_clears_stale_rates() {
        let root = temp_path("stop-stale");
        fs::create_dir_all(&root).unwrap();
        let info = b"d6:lengthi1e4:name4:test12:piece lengthi1e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        let torrent_bytes = wrap_torrent_with_info(info, &[], &[]);
        let ui = Arc::new(Mutex::new(ui::UiState {
            current_id: Some(11),
            torrents: vec![ui::UiTorrent {
                id: 11,
                name: "bugonia".to_string(),
                status: "downloading".to_string(),
                download_rate_bps: 90_000.0,
                upload_rate_bps: 10.0,
                active_peers: 8,
                eta_secs: 123,
                ..ui::UiTorrent::default()
            }],
            ..ui::UiState::default()
        }));
        let ui_state = Some(Arc::clone(&ui));
        let registry: SessionRegistry = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let session_store = Arc::new(SessionStore::load(&root).unwrap());
        let mut queue = VecDeque::from([TorrentRequest {
            id: 11,
            source: TorrentSource::Bytes(torrent_bytes),
            download_dir: root.clone(),
            preallocate: false,
            initial_label: String::new(),
        }]);

        stop_torrent(
            &registry,
            &ui_state,
            &mut queue,
            11,
            &session_store,
            &empty_in_flight(),
        )
        .unwrap();

        assert!(queue.is_empty());
        let state = lock_or_recover(&ui);
        assert_eq!(state.status, "stopped");
        assert_eq!(state.download_rate_bps, 0.0);
        assert_eq!(state.upload_rate_bps, 0.0);
        assert_eq!(state.active_peers, 0);
        assert_eq!(state.eta_secs, 0);
        assert_eq!(state.torrents[0].status, "stopped");
        assert_eq!(state.torrents[0].download_rate_bps, 0.0);
        assert_eq!(state.torrents[0].upload_rate_bps, 0.0);
        assert_eq!(state.torrents[0].active_peers, 0);
        assert_eq!(state.torrents[0].eta_secs, 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stop_torrent_keeps_context_registered_while_worker_stops() {
        let root = temp_path("stop-registered");
        fs::create_dir_all(&root).unwrap();
        let context = make_test_context(21, &root);
        let registry: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));
        register_session(&registry, Arc::clone(&context)).unwrap();
        let session_store = Arc::new(SessionStore::load(&root).unwrap());
        let ui_state = Some(Arc::new(Mutex::new(ui::UiState {
            torrents: vec![ui::UiTorrent {
                id: 21,
                name: "demo".to_string(),
                ..ui::UiTorrent::default()
            }],
            ..ui::UiState::default()
        })));
        let mut queue = VecDeque::new();

        stop_torrent(
            &registry,
            &ui_state,
            &mut queue,
            21,
            &session_store,
            &empty_in_flight(),
        )
        .unwrap();

        assert!(context.stop_requested.load(Ordering::SeqCst));
        assert!(!context.allow_completion_reentry.load(Ordering::SeqCst));
        assert!(find_context_by_id(&registry, 21).is_some());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn load_config_overrides_parses_and_rejects_unknown_keys() {
        let path = temp_path("config-ok");
        fs::write(
            &path,
            "\
download_dir=/tmp/dl
preallocate=yes
ui=true
port=7001
encryption=require
download_rate=2M
write_cache=64k
",
        )
        .unwrap();
        let cfg = load_config_overrides(&path).unwrap();
        assert_eq!(cfg.download_dir, Some(PathBuf::from("/tmp/dl")));
        assert_eq!(cfg.preallocate, Some(true));
        assert_eq!(cfg.ui, Some(true));
        assert_eq!(cfg.port, Some(7001));
        assert_eq!(cfg.encryption, Some(EncryptionMode::Require));
        assert_eq!(cfg.download_rate, Some(2 * 1024 * 1024));
        assert_eq!(cfg.write_cache_bytes, Some(64 * 1024));
        let _ = fs::remove_file(&path);

        let bad_path = temp_path("config-bad");
        fs::write(&bad_path, "unknown_key=value\n").unwrap();
        let err = match load_config_overrides(&bad_path) {
            Ok(_) => panic!("expected unknown key error"),
            Err(err) => err,
        };
        assert!(err.contains("unknown key"));
        let _ = fs::remove_file(&bad_path);
    }

    #[cfg(feature = "webseed")]
    #[test]
    fn webseed_url_builder_encodes_paths_for_multi_file_mode() {
        assert_eq!(
            build_webseed_url("https://seed.example/base", b"ignored", false),
            "https://seed.example/base"
        );
        assert_eq!(
            build_webseed_url("https://seed.example/base/", b"dir/file name#.bin", true),
            "https://seed.example/base/dir/file%20name%23.bin"
        );
    }
}

#[cfg(test)]
mod local_harness_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[cfg(feature = "dht")]
    use std::net::UdpSocket;
    #[cfg(feature = "dht")]
    use std::sync::mpsc;
    #[cfg(feature = "dht")]
    use std::time::Duration;

    #[cfg(feature = "dht")]
    fn free_udp_port() -> u16 {
        UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn compact_peer(addr: SocketAddr) -> Vec<u8> {
        match addr {
            SocketAddr::V4(v4) => {
                let mut out = Vec::with_capacity(6);
                out.extend_from_slice(&v4.ip().octets());
                out.extend_from_slice(&v4.port().to_be_bytes());
                out
            }
            SocketAddr::V6(_) => Vec::new(),
        }
    }

    #[test]
    fn local_http_tracker_fixture_serves_announce() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let peer_addr: SocketAddr = "127.0.0.1:6881".parse().unwrap();
        let body = bencode::encode(&Value::Dict(vec![
            (b"interval".to_vec(), Value::Int(60)),
            (b"peers".to_vec(), Value::Bytes(compact_peer(peer_addr))),
        ]));
        let body_for_server = body.clone();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = [0u8; 2048];
            let n = stream.read(&mut req).unwrap();
            let request = String::from_utf8_lossy(&req[..n]);
            assert!(request.starts_with("GET /announce?"));
            assert!(request.contains("info_hash="));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body_for_server.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(&body_for_server).unwrap();
        });

        let url = format!("http://127.0.0.1:{}/announce", addr.port());
        let response = tracker::announce_local_test(
            &url,
            [1u8; 20],
            [2u8; 20],
            6881,
            0,
            0,
            1,
            Some("started"),
            5,
        )
        .unwrap();
        assert_eq!(response.interval, 60);
        assert_eq!(response.peers, vec![peer_addr]);
        server.join().unwrap();
    }

    #[cfg(feature = "udp_tracker")]
    #[test]
    fn local_udp_tracker_fixture_serves_announce() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        let peer_addr: SocketAddr = "127.0.0.1:6881".parse().unwrap();
        let server = thread::spawn(move || {
            let mut buf = [0u8; 2048];
            let (n, src) = socket.recv_from(&mut buf).unwrap();
            assert_eq!(n, 16);
            let tx = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);
            let mut connect_resp = [0u8; 16];
            connect_resp[0..4].copy_from_slice(&0u32.to_be_bytes());
            connect_resp[4..8].copy_from_slice(&tx.to_be_bytes());
            connect_resp[8..16].copy_from_slice(&0x1122_3344_5566_7788u64.to_be_bytes());
            socket.send_to(&connect_resp, src).unwrap();

            let (n2, src2) = socket.recv_from(&mut buf).unwrap();
            assert!(n2 >= 98);
            let tx2 = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);
            let mut resp = Vec::new();
            resp.extend_from_slice(&1u32.to_be_bytes()); // announce action
            resp.extend_from_slice(&tx2.to_be_bytes());
            resp.extend_from_slice(&30u32.to_be_bytes()); // interval
            resp.extend_from_slice(&0u32.to_be_bytes()); // leechers
            resp.extend_from_slice(&1u32.to_be_bytes()); // seeders
            resp.extend_from_slice(&compact_peer(peer_addr));
            socket.send_to(&resp, src2).unwrap();
        });

        let url = format!("udp://127.0.0.1:{port}/announce");
        let response = udp_tracker::announce(
            &url,
            [3u8; 20],
            [4u8; 20],
            6881,
            0,
            0,
            1,
            Some("started"),
            10,
        )
        .unwrap();
        assert_eq!(response.interval, 30);
        assert_eq!(response.peers, vec![peer_addr]);
        server.join().unwrap();
    }

    #[test]
    fn local_peer_fixture_completes_plaintext_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let info_hash = [9u8; 20];
        let local_peer_id = [7u8; 20];
        let remote_peer_id = [8u8; 20];

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut peer_stream = PeerStream::tcp(stream);
            let request = peer::read_handshake(&mut peer_stream).unwrap();
            assert_eq!(request.info_hash, info_hash);
            assert!(request.supports_extensions());
            peer::write_handshake(&mut peer_stream, info_hash, remote_peer_id, true).unwrap();
        });

        let cfg = ConnectionConfig {
            encryption: EncryptionMode::Disable,
            utp: None,
            ip_filter: None,
            proxy: None,
        };
        let mut stream = connect_peer_for_metadata(addr, &cfg).unwrap();
        let handshake = plaintext_handshake(&mut stream, info_hash, None, local_peer_id).unwrap();
        assert_eq!(handshake.peer_id, remote_peer_id);
        assert_eq!(handshake.info_hash, info_hash);
        server.join().unwrap();
    }

    #[cfg(feature = "dht")]
    #[test]
    fn local_dht_fixture_node_returns_peers() {
        let dht_port = free_udp_port();
        let download_dir = std::env::temp_dir().join(format!(
            "rustorrent-dht-harness-{}-{dht_port}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&download_dir);
        fs::create_dir(&download_dir).unwrap();
        let fixture = UdpSocket::bind("127.0.0.1:0").unwrap();
        fixture
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let fixture_addr = fixture.local_addr().unwrap();

        let cache_path = download_dir.join(".rustorrent").join("dht_nodes.dat");

        let fixture_thread = thread::spawn(move || {
            for expected_query in [b"ping".as_slice(), b"get_peers".as_slice()] {
                let mut buf = [0u8; 1500];
                let (n, src) = fixture.recv_from(&mut buf).unwrap();
                let parsed = bencode::parse(&buf[..n]).unwrap();
                let Value::Dict(dict) = parsed else {
                    panic!("expected dict");
                };
                let tx = match dict_get(&dict, b"t") {
                    Some(Value::Bytes(tx)) => tx.clone(),
                    _ => panic!("missing tx id"),
                };
                assert_eq!(
                    dict_get(&dict, b"q"),
                    Some(&Value::Bytes(expected_query.to_vec()))
                );
                let mut response_fields = vec![(b"id".to_vec(), Value::Bytes(vec![0x22u8; 20]))];
                if expected_query == b"get_peers" {
                    let peer = compact_peer("127.0.0.1:6881".parse().unwrap());
                    response_fields
                        .push((b"values".to_vec(), Value::List(vec![Value::Bytes(peer)])));
                }
                let response = bencode::encode(&Value::Dict(vec![
                    (b"t".to_vec(), Value::Bytes(tx)),
                    (b"y".to_vec(), Value::Bytes(b"r".to_vec())),
                    (b"r".to_vec(), Value::Dict(response_fields)),
                ]));
                fixture.send_to(&response, src).unwrap();
            }
        });

        let dht =
            dht::start_with_test_candidate(dht_port, &download_dir, [0x22u8; 20], fixture_addr);
        thread::sleep(Duration::from_millis(300));
        let (tx, rx) = mpsc::channel();
        let info_hash = [1u8; 20];
        dht.add_torrent(info_hash, 6881, tx);
        let peers = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(peers.contains(&"127.0.0.1:6881".parse().unwrap()));
        dht.remove_torrent(info_hash);
        fixture_thread.join().unwrap();
        drop(dht);

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if fs::read(&cache_path).is_ok_and(|data| data.starts_with(b"DHTN\x01")) {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        assert!(fs::read(&cache_path).unwrap().starts_with(b"DHTN\x01"));
        // Windows keeps the hardened state-directory binding pinned for the
        // process lifetime, so best-effort cleanup is the portable contract.
        let _ = fs::remove_dir_all(download_dir);
    }
}

fn apply_file_priority(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    file_index: usize,
    priority: u8,
) -> Result<(), String> {
    if priority > piece::PRIORITY_HIGH {
        return Err("invalid priority".to_string());
    }
    let context =
        find_context_by_id(registry, torrent_id).ok_or_else(|| "unknown torrent".to_string())?;
    let priorities_snapshot = {
        let mut priorities = context
            .file_priorities
            .lock()
            .map_err(|_| "priority lock failed".to_string())?;
        if file_index >= priorities.len() {
            return Err("file index out of range".to_string());
        }
        if context
            .file_spans
            .get(file_index)
            .is_some_and(|span| span.is_padding)
        {
            return Err("padding files do not have a selectable priority".to_string());
        }
        priorities[file_index] = priority;
        priorities.clone()
    };
    let piece_priorities = compute_piece_priorities(
        &context.file_spans,
        &priorities_snapshot,
        context.base_piece_length,
        lock_or_recover(&context.pieces).piece_count(),
    );
    let (wanted_bytes, wanted_pieces, completed_bytes, completed_pieces) = {
        let mut pieces = context
            .pieces
            .lock()
            .map_err(|_| "piece lock failed".to_string())?;
        pieces
            .set_piece_priorities(&piece_priorities)
            .map_err(|err| err.to_string())?;
        (
            pieces.wanted_bytes(),
            pieces.wanted_pieces(),
            pieces.completed_bytes(),
            pieces.completed_pieces(),
        )
    };
    context.resume_save_requested.store(true, Ordering::SeqCst);
    update_ui(ui_state, |state| {
        if state.current_id == Some(torrent_id) {
            state.total_bytes = wanted_bytes;
            state.total_pieces = wanted_pieces;
            state.completed_bytes = completed_bytes;
            state.completed_pieces = completed_pieces;
            if let Some(file) = state.files.get_mut(file_index) {
                file.priority = priority;
            }
        }
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.total_bytes = wanted_bytes;
            torrent.total_pieces = wanted_pieces;
            torrent.completed_bytes = completed_bytes;
            torrent.completed_pieces = completed_pieces;
            if let Some(file) = torrent.files.get_mut(file_index) {
                file.priority = priority;
            }
        });
    });
    Ok(())
}

fn valid_renamed_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && name != "."
        && name != ".."
}

fn apply_file_rename(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    session_store: &SessionStore,
    torrent_id: u64,
    file_index: usize,
    new_name: &str,
) -> Result<(), String> {
    if !valid_renamed_file_name(new_name) {
        return Err("invalid file name".to_string());
    }
    let _operation = session_store.lock_operation();
    // Acquire the lifecycle lock before cloning the context. Completion moves
    // and active deletion hold this lock while waiting for all context/storage
    // users to drain; retaining a clone while blocked on the same lock would
    // deadlock that drain.
    let context =
        find_context_by_id(registry, torrent_id).ok_or_else(|| "unknown torrent".to_string())?;
    let spans = &context.file_spans;
    if file_index >= spans.len() {
        return Err("file index out of range".to_string());
    }
    let mut storage = context
        .storage
        .lock()
        .map_err(|_| "storage lock failed".to_string())?;
    let old_path = storage
        .file_path(file_index)
        .ok_or_else(|| "file index out of range".to_string())?
        .to_path_buf();
    let new_path = old_path.with_file_name(new_name);
    if old_path == new_path {
        return Ok(());
    }
    storage
        .validate_file_rename(file_index, &old_path, &new_path)
        .map_err(|err| format!("rename validation failed: {err}"))?;
    let pending = PendingFileRename {
        index: file_index,
        target: new_name.to_string(),
    };
    session_store.begin_file_rename(context.info_hash, file_index, new_name)?;
    if let Err(err) = storage.rename_file(file_index, &old_path, &new_path) {
        let cancel = session_store.cancel_file_rename(context.info_hash, &pending);
        return Err(match cancel {
            Ok(_) => format!("rename failed: {err}"),
            Err(cancel_err) => {
                format!("rename failed: {err}; rename journal cleanup failed: {cancel_err}")
            }
        });
    }
    match session_store.commit_file_rename(context.info_hash, &pending) {
        Ok(true) => {}
        Ok(false) => {
            let _ = storage.rename_file(file_index, &new_path, &old_path);
            return Err("rename journal changed before commit".to_string());
        }
        Err(commit_err) => {
            let rollback = storage.rename_file(file_index, &new_path, &old_path);
            if rollback.is_ok() {
                let _ = session_store.cancel_file_rename(context.info_hash, &pending);
            }
            return Err(match rollback {
                Ok(()) => format!("{commit_err}; physical rename was rolled back"),
                Err(rollback_err) => {
                    format!("{commit_err}; physical rename rollback failed: {rollback_err}")
                }
            });
        }
    }
    let mut renames = context
        .file_renames
        .lock()
        .map_err(|_| "renames lock failed".to_string())?;
    renames.insert(file_index, new_name.to_string());
    drop(renames);
    drop(storage);
    context.resume_save_requested.store(true, Ordering::SeqCst);
    let display_path = renamed_display_path(&spans[file_index].path, new_name);
    update_ui(ui_state, |state| {
        if state.current_id == Some(torrent_id) {
            if let Some(file) = state.files.get_mut(file_index) {
                file.path = display_path.clone();
            }
        }
        update_torrent_entry(state, torrent_id, |torrent| {
            if let Some(file) = torrent.files.get_mut(file_index) {
                file.path = display_path.clone();
            }
        });
    });
    Ok(())
}

fn set_torrent_paused(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    paused: bool,
) -> Result<(), String> {
    let context =
        find_context_by_id(registry, torrent_id).ok_or_else(|| "unknown torrent".to_string())?;
    context.paused.store(paused, Ordering::SeqCst);
    update_ui(ui_state, |state| {
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.paused = paused;
            if paused {
                torrent.status = "paused".to_string();
            } else if torrent.status == "paused" {
                let complete =
                    torrent.total_bytes > 0 && torrent.completed_bytes >= torrent.total_bytes;
                torrent.status = if complete {
                    "seeding".to_string()
                } else {
                    "downloading".to_string()
                };
            }
        });
        if state.current_id == Some(torrent_id) {
            if let Some(torrent) = state
                .torrents
                .iter()
                .find(|torrent| torrent.id == torrent_id)
            {
                state.status = torrent.status.clone();
            }
            state.paused = is_paused();
        }
    });
    Ok(())
}

fn resume_torrent(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    queue: &mut VecDeque<TorrentRequest>,
    torrent_id: u64,
    session_store: &Arc<SessionStore>,
    in_flight: &InFlightTorrents,
) -> Result<(), String> {
    if let Some(context) = find_context_by_id(registry, torrent_id) {
        if context.stop_requested.load(Ordering::SeqCst) {
            return Err("torrent is still stopping; try again in a moment".to_string());
        }
        return set_torrent_paused(registry, ui_state, torrent_id, false);
    }

    let info_hash = info_hash_from_ui(ui_state, torrent_id)
        .ok_or_else(|| "torrent cannot be resumed".to_string())?;
    let already_loading = lock_or_recover(in_flight).contains_key(&info_hash);
    if queue_contains_info_hash(queue, info_hash) || already_loading {
        update_ui(ui_state, |state| {
            update_torrent_entry(state, torrent_id, |torrent| {
                torrent.paused = false;
                torrent.status = if already_loading {
                    "loading".to_string()
                } else {
                    "queued".to_string()
                };
                torrent.download_rate_bps = 0.0;
                torrent.upload_rate_bps = 0.0;
                torrent.active_peers = 0;
                torrent.eta_secs = 0;
                torrent.last_error.clear();
            });
            if state.current_id == Some(torrent_id) {
                state.status = if already_loading {
                    "loading".to_string()
                } else {
                    "queued".to_string()
                };
                state.paused = false;
                state.download_rate_bps = 0.0;
                state.upload_rate_bps = 0.0;
                state.active_peers = 0;
                state.eta_secs = 0;
                state.last_error.clear();
            }
        });
        return Ok(());
    }

    let entry = session_store.get(info_hash).ok_or_else(|| {
        "torrent cannot be resumed because its session metadata is unavailable".to_string()
    })?;
    if entry.pending_delete {
        return Err("torrent deletion is pending; retry deletion instead".to_string());
    }
    let label = session_entry_label(&entry);
    let request = TorrentRequest {
        id: torrent_id,
        source: TorrentSource::Bytes(entry.torrent_bytes.clone()),
        download_dir: entry.download_dir.clone(),
        preallocate: entry.preallocate,
        initial_label: entry.label.clone(),
    };
    enqueue_request_with_label(queue, ui_state, request, label);
    update_ui(ui_state, |state| {
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.paused = false;
            torrent.download_rate_bps = 0.0;
            torrent.upload_rate_bps = 0.0;
            torrent.active_peers = 0;
            torrent.eta_secs = 0;
        });
        if state.current_id == Some(torrent_id) {
            state.status = "queued".to_string();
            state.paused = false;
            state.download_rate_bps = 0.0;
            state.upload_rate_bps = 0.0;
            state.active_peers = 0;
            state.eta_secs = 0;
            state.last_error.clear();
        }
    });
    Ok(())
}

fn stop_torrent(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    queue: &mut VecDeque<TorrentRequest>,
    torrent_id: u64,
    _session_store: &Arc<SessionStore>,
    in_flight: &InFlightTorrents,
) -> Result<(), String> {
    if let Some(context) = find_context_by_id(registry, torrent_id) {
        if context.teardown_failed.load(Ordering::Acquire) {
            cancel_peer_connections(&context.peer_cancellations);
            return Err(
                "torrent teardown previously timed out; restart before retrying".to_string(),
            );
        }
        context
            .allow_completion_reentry
            .store(false, Ordering::SeqCst);
        if context.stop_requested.load(Ordering::SeqCst) {
            cancel_peer_connections(&context.peer_cancellations);
            update_ui(ui_state, |state| {
                update_torrent_entry(state, torrent_id, |torrent| {
                    torrent.status = "stopping".to_string();
                    torrent.paused = false;
                    torrent.download_rate_bps = 0.0;
                    torrent.upload_rate_bps = 0.0;
                    torrent.active_peers = 0;
                    torrent.eta_secs = 0;
                });
                if state.current_id == Some(torrent_id) {
                    state.status = "stopping".to_string();
                    state.paused = false;
                    state.download_rate_bps = 0.0;
                    state.upload_rate_bps = 0.0;
                    state.active_peers = 0;
                    state.eta_secs = 0;
                }
            });
            return Ok(());
        }
        context.stop_requested.store(true, Ordering::SeqCst);
        cancel_peer_connections(&context.peer_cancellations);
        update_ui(ui_state, |state| {
            update_torrent_entry(state, torrent_id, |torrent| {
                torrent.status = "stopping".to_string();
                torrent.paused = false;
                torrent.download_rate_bps = 0.0;
                torrent.upload_rate_bps = 0.0;
                torrent.active_peers = 0;
                torrent.eta_secs = 0;
            });
            if state.current_id == Some(torrent_id) {
                state.status = "stopping".to_string();
                state.paused = false;
                state.download_rate_bps = 0.0;
                state.upload_rate_bps = 0.0;
                state.active_peers = 0;
                state.eta_secs = 0;
            }
        });
        return Ok(());
    }

    if lock_or_recover(in_flight)
        .values()
        .any(|loading_id| *loading_id == torrent_id)
    {
        return Err("torrent is still loading; try again in a moment".to_string());
    }

    let mut removed = false;
    queue.retain(|request| {
        if request.id == torrent_id {
            removed = true;
            false
        } else {
            true
        }
    });
    if removed {
        update_ui(ui_state, |state| {
            state.queue_len = queue.len();
            update_torrent_entry(state, torrent_id, |torrent| {
                torrent.status = "stopped".to_string();
                torrent.paused = false;
                torrent.download_rate_bps = 0.0;
                torrent.upload_rate_bps = 0.0;
                torrent.active_peers = 0;
                torrent.eta_secs = 0;
            });
            if state.current_id == Some(torrent_id) {
                state.status = "stopped".to_string();
                state.paused = false;
                state.download_rate_bps = 0.0;
                state.upload_rate_bps = 0.0;
                state.active_peers = 0;
                state.eta_secs = 0;
            }
        });
        return Ok(());
    }

    Err("unknown torrent".to_string())
}

fn archive_torrent(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    queue: &mut VecDeque<TorrentRequest>,
    torrent_id: u64,
    session_store: &Arc<SessionStore>,
    in_flight: &InFlightTorrents,
) -> Result<(), String> {
    delete_torrent(
        registry,
        ui_state,
        queue,
        torrent_id,
        false,
        session_store,
        in_flight,
    )
}

fn remove_torrent_ui(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    queue_len: usize,
) {
    update_ui(ui_state, |state| {
        state.deleted_torrents.insert(torrent_id);
        state.torrents.retain(|torrent| torrent.id != torrent_id);
        state.queue_len = queue_len;
        if state.current_id == Some(torrent_id) {
            state.current_id = None;
            state.status = if queue_len == 0 {
                "waiting for torrent".to_string()
            } else {
                "queued".to_string()
            };
            state.paused = is_paused();
            state.files.clear();
            state.total_bytes = 0;
            state.completed_bytes = 0;
            state.total_pieces = 0;
            state.completed_pieces = 0;
        }
    });
}

fn mark_delete_failed_ui(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    error: &str,
    queue_len: usize,
) {
    update_ui(ui_state, |state| {
        state.queue_len = queue_len;
        state.last_error = error.to_string();
        if state.current_id == Some(torrent_id) {
            state.status = "delete failed".to_string();
            state.download_rate_bps = 0.0;
            state.upload_rate_bps = 0.0;
            state.active_peers = 0;
        }
        update_torrent_entry(state, torrent_id, |torrent| {
            torrent.status = "delete failed".to_string();
            torrent.last_error = error.to_string();
            torrent.download_rate_bps = 0.0;
            torrent.upload_rate_bps = 0.0;
            torrent.active_peers = 0;
        });
    });
}

fn retain_delete_error<T>(
    result: Result<T, String>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
    queue_len: usize,
) -> Result<T, String> {
    result.inspect_err(|error| {
        mark_delete_failed_ui(ui_state, torrent_id, error, queue_len);
    })
}

fn delete_torrent(
    registry: &SessionRegistry,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    queue: &mut VecDeque<TorrentRequest>,
    torrent_id: u64,
    remove_data: bool,
    session_store: &Arc<SessionStore>,
    in_flight: &InFlightTorrents,
) -> Result<(), String> {
    let _operation = session_store.lock_operation();
    if let Some(context) = find_context_by_id(registry, torrent_id) {
        if context.teardown_failed.load(Ordering::Acquire) {
            cancel_peer_connections(&context.peer_cancellations);
            return Err(
                "torrent teardown previously timed out; restart before retrying".to_string(),
            );
        }
        context
            .allow_completion_reentry
            .store(false, Ordering::SeqCst);
        if remove_data {
            session_store.begin_delete(context.info_hash)?;
            context.delete_data_requested.store(true, Ordering::Release);
            context.stop_requested.store(true, Ordering::SeqCst);
            cancel_peer_connections(&context.peer_cancellations);
            update_ui(ui_state, |state| {
                update_torrent_entry(state, torrent_id, |torrent| {
                    torrent.status = "deleting".to_string();
                    torrent.download_rate_bps = 0.0;
                    torrent.upload_rate_bps = 0.0;
                });
            });
            return Ok(());
        }
        let entry = session_store
            .get(context.info_hash)
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if entry.pending_delete {
            return Err("data deletion is pending; retry delete with data".to_string());
        }
        context.archive_requested.store(true, Ordering::Release);
        context.stop_requested.store(true, Ordering::SeqCst);
        cancel_peer_connections(&context.peer_cancellations);
        update_ui(ui_state, |state| {
            update_torrent_entry(state, torrent_id, |torrent| {
                torrent.status = "archiving".to_string();
                torrent.download_rate_bps = 0.0;
                torrent.upload_rate_bps = 0.0;
            });
        });
        return Ok(());
    }

    if lock_or_recover(in_flight)
        .values()
        .any(|loading_id| *loading_id == torrent_id)
    {
        return Err("torrent is still loading; try again in a moment".to_string());
    }

    if let Some(queue_index) = queue.iter().position(|request| request.id == torrent_id) {
        let request = queue
            .get(queue_index)
            .cloned()
            .ok_or_else(|| "queued torrent disappeared".to_string())?;
        let info_hash = info_hash_for_source(&request.source)
            .map_err(|_| "torrent metadata unavailable for safe deletion".to_string())?;
        if remove_data {
            if !session_store.contains(info_hash) {
                let data = match &request.source {
                    TorrentSource::Bytes(data) if data.len() <= MAX_TORRENT_BYTES => data.clone(),
                    TorrentSource::Bytes(_) => return Err("torrent file too large".to_string()),
                    TorrentSource::Path(path) => {
                        read_file_limited(Path::new(path), MAX_TORRENT_BYTES, false)
                            .map_err(|err| format!("read failed: {err}"))?
                    }
                    TorrentSource::Magnet(_) => {
                        return Err("magnet metadata unavailable for safe deletion".to_string())
                    }
                };
                let meta =
                    torrent::parse_torrent(&data).map_err(|err| format!("parse error: {err}"))?;
                let legacy_renames = legacy_resume_file_renames(&request.download_dir, info_hash)?;
                session_store.upsert_with_storage_claim(
                    info_hash,
                    String::from_utf8_lossy(&meta.info.name).into_owned(),
                    data,
                    &request.download_dir,
                    request.preallocate,
                    &legacy_renames,
                )?;
            }
            if let Some(entry) = session_store
                .get(info_hash)
                .filter(|entry| entry.file_renames.is_empty())
            {
                let legacy_renames = legacy_resume_file_renames(&entry.download_dir, info_hash)?;
                session_store.import_file_renames_if_empty(info_hash, &legacy_renames)?;
            }
            session_store.begin_delete(info_hash)?;
            let _ = queue.remove(queue_index);
            let entry = session_store
                .get(info_hash)
                .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
            if let Err(err) = delete_session_entry_payload(&entry) {
                mark_delete_failed_ui(ui_state, torrent_id, &err, queue.len());
                return Err(err);
            }
            if let Err(err) = session_store.remove(info_hash) {
                mark_delete_failed_ui(ui_state, torrent_id, &err, queue.len());
                return Err(err);
            }
            remove_torrent_ui(ui_state, torrent_id, queue.len());
            return Ok(());
        }
        if let Some(entry) = session_store.get(info_hash) {
            if entry.pending_delete {
                return Err("data deletion is pending; retry delete with data".to_string());
            }
            session_store.remove(info_hash)?;
        }
        let _ = queue.remove(queue_index);
        remove_torrent_ui(ui_state, torrent_id, queue.len());
        return Ok(());
    }

    let info_hash =
        info_hash_from_ui(ui_state, torrent_id).ok_or_else(|| "unknown torrent".to_string())?;
    let mut entry = session_store
        .get(info_hash)
        .ok_or_else(|| "torrent metadata unavailable; refusing unsafe deletion".to_string())?;
    if remove_data {
        if entry.file_renames.is_empty() {
            let legacy_renames = legacy_resume_file_renames(&entry.download_dir, info_hash)?;
            session_store.import_file_renames_if_empty(info_hash, &legacy_renames)?;
        }
        session_store.begin_delete(info_hash)?;
        entry = session_store
            .get(info_hash)
            .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
        if let Err(err) = delete_session_entry_payload(&entry) {
            mark_delete_failed_ui(ui_state, torrent_id, &err, queue.len());
            return Err(err);
        }
    } else if entry.pending_delete {
        return Err("data deletion is pending; retry delete with data".to_string());
    }
    if let Err(err) = session_store.remove(info_hash) {
        mark_delete_failed_ui(ui_state, torrent_id, &err, queue.len());
        return Err(err);
    }
    remove_torrent_ui(ui_state, torrent_id, queue.len());
    Ok(())
}

fn delete_info_from_request(
    request: &TorrentRequest,
    file_renames: &[(usize, String)],
) -> Result<([u8; 20], PathBuf, Vec<PathBuf>), String> {
    let data = match &request.source {
        TorrentSource::Path(path) => read_file_limited(Path::new(path), MAX_TORRENT_BYTES, false)
            .map_err(|err| format!("read failed: {err}"))?,
        TorrentSource::Bytes(data) if data.len() <= MAX_TORRENT_BYTES => data.clone(),
        TorrentSource::Bytes(_) => return Err("torrent file too large".to_string()),
        TorrentSource::Magnet(_) => return Err("magnet metadata unavailable".to_string()),
    };
    let meta = torrent::parse_torrent(&data).map_err(|err| format!("parse error: {err}"))?;
    let paths = if file_renames.is_empty() {
        storage::data_paths(&meta, &request.download_dir)
    } else {
        storage::data_paths_with_file_renames(&meta, &request.download_dir, file_renames)
    }
    .map_err(|err| format!("storage paths: {err}"))?;
    Ok((meta.info_hash, request.download_dir.clone(), paths))
}

fn legacy_resume_file_renames(
    download_dir: &Path,
    info_hash: [u8; 20],
) -> Result<Vec<(usize, String)>, String> {
    let path = resume_path(download_dir, info_hash);
    let existed = path.exists() || sidecar_path(&path, ".bak").exists();
    let resume = load_resume_data_with_recovery(&path);
    if existed && resume.is_none() {
        return Err("resume metadata unavailable for safe renamed-file deletion".to_string());
    }
    match resume {
        Some(resume) if resume.info_hash == info_hash => Ok(resume.file_renames),
        Some(_) => Err("resume metadata info hash mismatch".to_string()),
        None => Ok(Vec::new()),
    }
}

fn delete_info_from_session_entry(
    entry: &SessionEntry,
) -> Result<([u8; 20], PathBuf, Vec<PathBuf>), String> {
    let request = TorrentRequest {
        id: 0,
        source: TorrentSource::Bytes(entry.torrent_bytes.clone()),
        download_dir: entry.download_dir.clone(),
        preallocate: entry.preallocate,
        initial_label: entry.label.clone(),
    };
    let info = delete_info_from_request(&request, &entry.file_renames)?;
    if info.0 != entry.info_hash {
        return Err("session metadata info hash mismatch".to_string());
    }
    Ok(info)
}

fn delete_session_entry_payload(entry: &SessionEntry) -> Result<(), String> {
    let (info_hash, download_dir, paths) = delete_info_from_session_entry(entry)?;
    delete_storage_paths(&download_dir, &paths)?;
    remove_resume_files(&resume_path(&download_dir, info_hash))
}

fn retry_pending_delete(session_store: &SessionStore, info_hash: [u8; 20]) -> Result<(), String> {
    let _operation = session_store.lock_operation();
    let entry = session_store
        .get(info_hash)
        .ok_or_else(|| "torrent session metadata is unavailable".to_string())?;
    if !entry.pending_delete {
        return Err("torrent deletion is not pending".to_string());
    }
    delete_session_entry_payload(&entry)?;
    if !session_store.remove(info_hash)? {
        return Err("torrent session metadata disappeared during deletion".to_string());
    }
    Ok(())
}

fn info_hash_from_ui(
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    torrent_id: u64,
) -> Option<[u8; 20]> {
    let state = ui_state.as_ref()?.lock().ok()?;
    let torrent = state
        .torrents
        .iter()
        .find(|torrent| torrent.id == torrent_id)?;
    parse_hex_20(&torrent.info_hash)
}

fn parse_hex_20(value: &str) -> Option<[u8; 20]> {
    let bytes = value.as_bytes();
    if bytes.len() != 40 {
        return None;
    }
    let mut out = [0u8; 20];
    for idx in 0..20 {
        let hi = hex_nibble(bytes[idx * 2])?;
        let lo = hex_nibble(bytes[idx * 2 + 1])?;
        out[idx] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn delete_storage_paths(download_dir: &Path, paths: &[PathBuf]) -> Result<(), String> {
    #[cfg(unix)]
    {
        delete_storage_paths_unix(download_dir, paths)
    }
    #[cfg(windows)]
    {
        delete_storage_paths_windows(download_dir, paths)
    }
    #[cfg(not(any(unix, windows)))]
    {
        delete_storage_paths_portable(download_dir, paths)
    }
}

#[cfg(unix)]
fn delete_storage_paths_unix(download_dir: &Path, paths: &[PathBuf]) -> Result<(), String> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;

    fn component_name(component: &std::path::Component<'_>) -> Result<CString, String> {
        let std::path::Component::Normal(name) = component else {
            return Err("unsafe storage path component".to_string());
        };
        CString::new(name.as_bytes()).map_err(|_| "storage path contains NUL".to_string())
    }

    fn open_child_directory(
        parent: i32,
        component: &std::path::Component<'_>,
    ) -> io::Result<OwnedFd> {
        let name = component_name(component)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        // SAFETY: `parent` is a live directory descriptor and `name` is a
        // NUL-terminated single path component. O_NOFOLLOW prevents a swapped
        // intermediate symlink from redirecting the walk.
        let fd = unsafe {
            libc::openat(
                parent,
                name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `openat` returned a new owned descriptor.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn unlink_relative(root: &fs::File, relative: &Path, directory: bool) -> io::Result<()> {
        let components = relative.components().collect::<Vec<_>>();
        if components.is_empty()
            || components
                .iter()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsafe storage path",
            ));
        }
        let mut opened = Vec::<OwnedFd>::new();
        let mut parent_fd = root.as_raw_fd();
        for component in components.iter().take(components.len() - 1) {
            let child = open_child_directory(parent_fd, component)?;
            parent_fd = child.as_raw_fd();
            opened.push(child);
        }
        let last = components
            .last()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty storage path"))?;
        let name =
            component_name(last).map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        // SAFETY: the verified parent descriptor remains owned by `root` or
        // `opened` for the duration of the call. unlinkat does not follow the
        // final component when removing a file entry.
        let result = unsafe {
            libc::unlinkat(
                parent_fd,
                name.as_ptr(),
                if directory { libc::AT_REMOVEDIR } else { 0 },
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    let root_name = CString::new(download_dir.as_os_str().as_bytes())
        .map_err(|_| "download directory contains NUL".to_string())?;
    // SAFETY: `root_name` is a valid NUL-terminated path. The returned
    // descriptor is checked for failure and then owned by `OwnedFd`.
    let root_fd = unsafe {
        libc::open(
            root_name.as_ptr(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
        )
    };
    if root_fd < 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::NotFound {
            return Ok(());
        }
        return Err(format!("open download directory safely: {error}"));
    }
    // SAFETY: `open` returned a new owned descriptor.
    let root_fd = unsafe { fs::File::from_raw_fd(root_fd) };
    let canonical_root = fs::canonicalize(download_dir)
        .map_err(|err| format!("resolve download directory: {err}"))?;
    let resolved_metadata = fs::metadata(&canonical_root)
        .map_err(|err| format!("inspect resolved download directory: {err}"))?;
    let opened_metadata = root_fd
        .metadata()
        .map_err(|err| format!("inspect open download directory: {err}"))?;
    use std::os::unix::fs::MetadataExt;
    if !resolved_metadata.is_dir()
        || resolved_metadata.dev() != opened_metadata.dev()
        || resolved_metadata.ino() != opened_metadata.ino()
    {
        return Err("download directory changed while opening for deletion".to_string());
    }

    let mut failures = Vec::new();
    for full_path in paths {
        let relative = match full_path.strip_prefix(download_dir) {
            Ok(relative) => relative,
            Err(_) => {
                failures.push(format!(
                    "path is outside download directory: {}",
                    full_path.display()
                ));
                continue;
            }
        };
        match unlink_relative(&root_fd, relative, false) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                failures.push(format!("remove {}: {err}", full_path.display()));
                continue;
            }
        }

        let mut parent = relative.parent();
        while let Some(directory) = parent.filter(|path| !path.as_os_str().is_empty()) {
            match unlink_relative(&root_fd, directory, true) {
                Ok(()) => parent = directory.parent(),
                Err(_) => break,
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("data deletion failed: {}", failures.join("; ")))
    }
}

#[cfg(windows)]
fn delete_storage_paths_windows(download_dir: &Path, paths: &[PathBuf]) -> Result<(), String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};

    const DELETE: u32 = 0x0001_0000;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_TRAVERSE: u32 = 0x0000_0020;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_OPEN: u32 = 0x0000_0001;
    const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;
    const FILE_NON_DIRECTORY_FILE: u32 = 0x0000_0040;
    const FILE_OPEN_FOR_BACKUP_INTENT: u32 = 0x0000_4000;
    const FILE_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_DISPOSITION_INFO_CLASS: i32 = 4;

    #[repr(C)]
    struct FileDispositionInfo {
        delete_file: i32,
    }

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *mut u16,
    }

    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root_directory: RawHandle,
        object_name: *mut UnicodeString,
        attributes: u32,
        security_descriptor: *mut std::ffi::c_void,
        security_quality_of_service: *mut std::ffi::c_void,
    }

    #[repr(C)]
    union IoStatusBlockStatus {
        status: i32,
        pointer: *mut std::ffi::c_void,
    }

    #[repr(C)]
    struct IoStatusBlock {
        status: IoStatusBlockStatus,
        information: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFinalPathNameByHandleW(
            file: RawHandle,
            path: *mut u16,
            path_len: u32,
            flags: u32,
        ) -> u32;
        fn SetFileInformationByHandle(
            file: RawHandle,
            information_class: i32,
            information: *const std::ffi::c_void,
            information_len: u32,
        ) -> i32;
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtCreateFile(
            file_handle: *mut RawHandle,
            desired_access: u32,
            object_attributes: *mut ObjectAttributes,
            io_status_block: *mut IoStatusBlock,
            allocation_size: *mut i64,
            file_attributes: u32,
            share_access: u32,
            create_disposition: u32,
            create_options: u32,
            ea_buffer: *mut std::ffi::c_void,
            ea_length: u32,
        ) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
    }

    fn open_root(path: &Path) -> io::Result<fs::File> {
        let mut options = fs::OpenOptions::new();
        options
            .access_mode(FILE_READ_ATTRIBUTES | FILE_TRAVERSE)
            // Omitting FILE_SHARE_DELETE pins the directory entry while the
            // operation holds this handle.
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
    }

    fn open_relative(
        root: &fs::File,
        component: &std::ffi::OsStr,
        delete_access: bool,
        directory: bool,
    ) -> io::Result<fs::File> {
        let mut wide = component.encode_wide().collect::<Vec<_>>();
        if wide.is_empty() || wide.contains(&0) || wide.len() > (u16::MAX as usize / 2) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid Windows path component",
            ));
        }
        let byte_length = (wide.len() * 2) as u16;
        let mut name = UnicodeString {
            length: byte_length,
            maximum_length: byte_length,
            buffer: wide.as_mut_ptr(),
        };
        let mut attributes = ObjectAttributes {
            length: std::mem::size_of::<ObjectAttributes>() as u32,
            root_directory: root.as_raw_handle(),
            object_name: &mut name,
            // Deliberately omit OBJ_CASE_INSENSITIVE. This fails closed in a
            // case-sensitive Windows directory instead of opening an alias.
            attributes: 0,
            security_descriptor: std::ptr::null_mut(),
            security_quality_of_service: std::ptr::null_mut(),
        };
        let mut io_status = IoStatusBlock {
            status: IoStatusBlockStatus {
                pointer: std::ptr::null_mut(),
            },
            information: 0,
        };
        let mut handle: RawHandle = std::ptr::null_mut();
        let desired_access = FILE_READ_ATTRIBUTES
            | if directory { FILE_TRAVERSE } else { 0 }
            | if delete_access { DELETE } else { 0 };
        let create_options = FILE_OPEN_FOR_BACKUP_INTENT
            | FILE_OPEN_REPARSE_POINT
            | if directory {
                FILE_DIRECTORY_FILE
            } else {
                FILE_NON_DIRECTORY_FILE
            };
        // SAFETY: all native structures use their documented C layout and
        // remain live for the call. `root` is a live directory handle, the
        // object name is one validated relative component, and a successful
        // call returns a new handle owned below by `File`.
        let status = unsafe {
            NtCreateFile(
                &mut handle,
                desired_access,
                &mut attributes,
                &mut io_status,
                std::ptr::null_mut(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_OPEN,
                create_options,
                std::ptr::null_mut(),
                0,
            )
        };
        if status < 0 {
            // SAFETY: translating an NTSTATUS has no preconditions.
            let error = unsafe { RtlNtStatusToDosError(status) };
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        if handle.is_null() {
            return Err(io::Error::other("Windows returned an empty file handle"));
        }
        // SAFETY: NtCreateFile succeeded and transferred ownership of this
        // newly-created handle to the caller.
        Ok(unsafe { fs::File::from_raw_handle(handle) })
    }

    fn final_path(file: &fs::File) -> io::Result<Vec<u16>> {
        let mut capacity = 512usize;
        loop {
            if capacity > 65_536 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "resolved Windows path is unreasonably long",
                ));
            }
            let mut buffer = vec![0u16; capacity];
            // SAFETY: `file` owns a live Windows handle and `buffer` provides
            // the writable capacity reported to the operating system.
            let length = unsafe {
                GetFinalPathNameByHandleW(
                    file.as_raw_handle(),
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                    0,
                )
            };
            if length == 0 {
                return Err(io::Error::last_os_error());
            }
            if (length as usize) < buffer.len() {
                buffer.truncate(length as usize);
                return Ok(buffer);
            }
            capacity = (length as usize).saturating_add(1);
        }
    }

    fn open_verified(
        root: &fs::File,
        relative: &Path,
        expected: &Path,
        delete_access: bool,
        directory: bool,
    ) -> io::Result<(fs::File, Vec<fs::File>)> {
        let components = relative.components().collect::<Vec<_>>();
        if components.is_empty()
            || components
                .iter()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsafe storage path",
            ));
        }

        // Walk from the pinned root one component at a time and retain every
        // directory handle. Relative native opens bind the walk to the root
        // handle even if an ancestor's textual path is concurrently moved;
        // omitting FILE_SHARE_DELETE prevents each opened entry being swapped.
        let mut opened = Vec::<fs::File>::new();
        for component in components.iter().take(components.len() - 1) {
            let name = match component {
                std::path::Component::Normal(name) => name,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unsafe storage path component",
                    ));
                }
            };
            let parent = opened.last().unwrap_or(root);
            let child = open_relative(parent, name, false, true)?;
            let metadata = child.metadata()?;
            if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "path traverses a filesystem reparse point",
                ));
            }
            opened.push(child);
        }
        let last = components
            .last()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty storage path"))?;
        let name = match last {
            std::path::Component::Normal(name) => name,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsafe storage path component",
                ));
            }
        };
        let parent = opened.last().unwrap_or(root);
        let file = open_relative(parent, name, delete_access, directory)?;
        let resolved = final_path(&file)?;
        let expected_wide = expected.as_os_str().encode_wide().collect::<Vec<_>>();
        // Exact comparison intentionally fails closed for Windows directories
        // with case-sensitive semantics. Files created by rustorrent retain
        // the metainfo spelling, so normal payloads have identical paths.
        if resolved != expected_wide {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "path was redirected through a filesystem reparse point",
            ));
        }
        Ok((file, opened))
    }

    fn mark_delete(file: &fs::File) -> io::Result<()> {
        let information = FileDispositionInfo { delete_file: 1 };
        // SAFETY: the handle was opened with DELETE access and `information`
        // has the exact layout required for FileDispositionInfo.
        let result = unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle(),
                FILE_DISPOSITION_INFO_CLASS,
                (&information as *const FileDispositionInfo).cast(),
                std::mem::size_of::<FileDispositionInfo>() as u32,
            )
        };
        if result != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    let canonical_root = match fs::canonicalize(download_dir) {
        Ok(root) => root,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("resolve download directory: {err}")),
    };
    // Do not share delete access for the root handle. Keeping it open pins the
    // security boundary while descendants are opened and removed.
    let root_handle = open_root(&canonical_root)
        .map_err(|err| format!("open download directory safely: {err}"))?;
    let root_metadata = root_handle
        .metadata()
        .map_err(|err| format!("inspect download directory: {err}"))?;
    if !root_metadata.is_dir()
        || root_metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err("download directory is not a real directory".to_string());
    }
    let root_wide = final_path(&root_handle)
        .map_err(|err| format!("resolve open download directory: {err}"))?;
    let canonical_root_wide = canonical_root.as_os_str().encode_wide().collect::<Vec<_>>();
    if root_wide != canonical_root_wide {
        return Err("download directory changed while opening it".to_string());
    }
    let root_path = PathBuf::from(std::ffi::OsString::from_wide(&root_wide));

    let mut failures = Vec::new();
    for full_path in paths {
        let relative = match full_path.strip_prefix(download_dir) {
            Ok(relative) => relative,
            Err(_) => {
                failures.push(format!(
                    "path is outside download directory: {}",
                    full_path.display()
                ));
                continue;
            }
        };
        let components = relative.components().collect::<Vec<_>>();
        if components.is_empty()
            || components
                .iter()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            failures.push(format!("unsafe storage path: {}", full_path.display()));
            continue;
        }

        let mut expected = root_path.clone();
        for component in &components {
            expected.push(component.as_os_str());
        }
        let (file, opened_parents) =
            match open_verified(&root_handle, relative, &expected, true, false) {
                Ok(opened) => opened,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => {
                    failures.push(format!("open {} safely: {err}", full_path.display()));
                    continue;
                }
            };
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                failures.push(format!("inspect {}: {err}", full_path.display()));
                continue;
            }
        };
        if metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            failures.push(format!(
                "refusing to remove a directory or reparse point as torrent data: {}",
                full_path.display()
            ));
            continue;
        }
        if let Err(err) = mark_delete(&file) {
            failures.push(format!("remove {}: {err}", full_path.display()));
            continue;
        }
        drop(file);
        drop(opened_parents);

        // Empty-parent cleanup is best-effort, as on Unix. Each directory is
        // independently opened without following its final reparse point and
        // must resolve to its exact lexical location beneath the pinned root.
        let mut parent = relative.parent();
        while let Some(directory) = parent.filter(|path| !path.as_os_str().is_empty()) {
            let expected = root_path.join(directory);
            let (directory_handle, opened_parents) =
                match open_verified(&root_handle, directory, &expected, true, true) {
                    Ok(opened) => opened,
                    Err(_) => break,
                };
            let metadata = match directory_handle.metadata() {
                Ok(metadata) => metadata,
                Err(_) => break,
            };
            if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            {
                break;
            }
            if mark_delete(&directory_handle).is_err() {
                break;
            }
            drop(directory_handle);
            drop(opened_parents);
            parent = directory.parent();
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("data deletion failed: {}", failures.join("; ")))
    }
}

#[cfg(not(any(unix, windows)))]
fn delete_storage_paths_portable(download_dir: &Path, paths: &[PathBuf]) -> Result<(), String> {
    let mut failures = Vec::new();
    for full_path in paths {
        let rel_path = match full_path.strip_prefix(download_dir) {
            Ok(path) => path,
            Err(_) => {
                failures.push(format!(
                    "path is outside download directory: {}",
                    full_path.display()
                ));
                continue;
            }
        };
        let components = rel_path.components().collect::<Vec<_>>();
        if components.is_empty()
            || components
                .iter()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            failures.push(format!("unsafe storage path: {}", full_path.display()));
            continue;
        }
        let mut parent = download_dir.to_path_buf();
        let mut unsafe_parent = false;
        for component in components.iter().take(components.len().saturating_sub(1)) {
            parent.push(component.as_os_str());
            match fs::symlink_metadata(&parent) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    unsafe_parent = true;
                    break;
                }
                Ok(_) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => break,
                Err(_) => {
                    unsafe_parent = true;
                    break;
                }
            }
        }
        if unsafe_parent {
            failures.push(format!(
                "path traverses an unsafe parent: {}",
                full_path.display()
            ));
            continue;
        }
        if let Err(err) = fs::remove_file(full_path) {
            if err.kind() != io::ErrorKind::NotFound {
                failures.push(format!("remove {}: {err}", full_path.display()));
                continue;
            }
        }
        cleanup_empty_dirs(full_path, download_dir);
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("data deletion failed: {}", failures.join("; ")))
    }
}

fn remove_resume_files(path: &Path) -> Result<(), String> {
    #[cfg(any(unix, windows))]
    if state_dir::is_state_file_path(path) {
        return state_dir::remove_resume_artifacts(path)
            .map_err(|err| format!("resume deletion failed: {err}"));
    }
    let mut failures = Vec::new();
    for candidate in [
        path.to_path_buf(),
        sidecar_path(path, ".bak"),
        sidecar_path(path, ".tmp"),
    ] {
        if let Err(err) = fs::remove_file(&candidate) {
            if err.kind() != io::ErrorKind::NotFound {
                failures.push(format!("remove {}: {err}", candidate.display()));
            }
        }
    }
    let Some(parent) = path.parent() else {
        return if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("resume deletion failed: {}", failures.join("; ")))
        };
    };
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("resume deletion failed: {}", failures.join("; ")))
        };
    };
    let tmp_prefix = format!("{file_name}.tmp.");
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return if failures.is_empty() {
                Ok(())
            } else {
                Err(format!("resume deletion failed: {}", failures.join("; ")))
            };
        }
        Err(err) => {
            failures.push(format!("read resume directory {}: {err}", parent.display()));
            return Err(format!("resume deletion failed: {}", failures.join("; ")));
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) if entry.file_name().to_string_lossy().starts_with(&tmp_prefix) => {
                if let Err(err) = fs::remove_file(entry.path()) {
                    if err.kind() != io::ErrorKind::NotFound {
                        failures.push(format!("remove {}: {err}", entry.path().display()));
                    }
                }
            }
            Ok(_) => {}
            Err(err) => failures.push(format!("read resume directory entry: {err}")),
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("resume deletion failed: {}", failures.join("; ")))
    }
}

#[cfg(not(any(unix, windows)))]
fn cleanup_empty_dirs(path: &Path, root: &Path) {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == root {
            break;
        }
        match fs::remove_dir(dir) {
            Ok(_) => {
                current = dir.parent();
            }
            Err(_) => break,
        }
    }
}

fn info_hash_for_source(source: &TorrentSource) -> Result<[u8; 20], String> {
    match source {
        TorrentSource::Bytes(data) if data.len() <= MAX_TORRENT_BYTES => {
            torrent::parse_torrent(data)
                .map(|meta| meta.info_hash)
                .map_err(|err| format!("parse error: {err}"))
        }
        TorrentSource::Bytes(_) => Err("torrent file too large".to_string()),
        TorrentSource::Path(path) => {
            let data = read_file_limited(Path::new(path), MAX_TORRENT_BYTES, false)
                .map_err(|err| format!("read failed: {err}"))?;
            torrent::parse_torrent(&data)
                .map(|meta| meta.info_hash)
                .map_err(|err| format!("parse error: {err}"))
        }
        TorrentSource::Magnet(link) => parse_magnet(link)
            .map(|meta| meta.info_hash)
            .map_err(|err| format!("magnet parse error: {err}")),
    }
}

fn freeze_request_source(request: &mut TorrentRequest) -> Result<[u8; 20], String> {
    let TorrentSource::Path(path) = &request.source else {
        return info_hash_for_source(&request.source);
    };
    let data = read_file_limited(Path::new(path), MAX_TORRENT_BYTES, false)
        .map_err(|err| format!("read failed: {err}"))?;
    request.source = TorrentSource::Bytes(data);
    info_hash_for_source(&request.source)
}

struct ProgressStats {
    snapshots: std::collections::VecDeque<(u64, Instant)>,
    speed_bps: f64,
    last_line_len: usize,
}

#[derive(Default)]
struct ProgressSnapshot {
    downloaded_bytes: u64,
    completed_bytes: u64,
    total_bytes: u64,
    active_peers: usize,
    tracker_peers: usize,
    status: String,
}

impl ProgressStats {
    fn new() -> Self {
        Self {
            snapshots: std::collections::VecDeque::new(),
            speed_bps: 0.0,
            last_line_len: 0,
        }
    }

    fn update_speed(&mut self, current: u64) {
        let now = Instant::now();
        if self.snapshots.is_empty() {
            // Seed with current value so resumed bytes don't cause a spike.
            self.snapshots.push_back((current, now));
            return;
        }
        self.snapshots.push_back((current, now));
        let window = Duration::from_secs(5);
        while self.snapshots.len() > 1 {
            if now.duration_since(self.snapshots[0].1) > window {
                self.snapshots.pop_front();
            } else {
                break;
            }
        }
        if self.snapshots.len() >= 2 {
            if let (Some(first), Some(last)) = (self.snapshots.front(), self.snapshots.back()) {
                let elapsed = last.1.duration_since(first.1).as_secs_f64();
                if elapsed >= 0.1 {
                    self.speed_bps = last.0.saturating_sub(first.0) as f64 / elapsed;
                }
            }
        }
    }

    fn render_line(&mut self, state: &ProgressSnapshot) -> (String, f64, u64) {
        self.update_speed(state.downloaded_bytes);

        let total = state.total_bytes.max(1);
        let completed_bytes = state.completed_bytes.min(state.total_bytes);
        let pct = (completed_bytes as f64 / total as f64).clamp(0.0, 1.0);
        let bar_width = 30usize;
        let filled = ((pct * bar_width as f64).round() as usize).min(bar_width);
        let bar = format!("{}{}", "#".repeat(filled), "-".repeat(bar_width - filled));

        let completed = human_bytes(completed_bytes);
        let total = human_bytes(state.total_bytes);
        let speed = human_rate(self.speed_bps);
        let eta_secs = if self.speed_bps > 1.0 {
            let remaining = state.total_bytes.saturating_sub(completed_bytes);
            (remaining as f64 / self.speed_bps).round() as u64
        } else {
            0
        };
        let eta = if eta_secs > 0 {
            format_eta(eta_secs as f64)
        } else {
            "--:--".to_string()
        };

        let mut line = format!(
            "[{bar}] {:>6.2}% {completed}/{total} {speed} ETA {eta} peers {}/{}",
            pct * 100.0,
            state.active_peers,
            state.tracker_peers
        );
        if !state.status.is_empty() {
            line.push(' ');
            line.push_str(&state.status);
        }
        (line, self.speed_bps, eta_secs)
    }

    fn current_speed(&self) -> f64 {
        self.speed_bps
    }
}

fn start_console_progress(
    state: Arc<Mutex<ui::UiState>>,
    registry: SessionRegistry,
) -> Result<thread::JoinHandle<()>, String> {
    thread::Builder::new()
        .name("console-progress".to_string())
        .spawn(move || {
            let mut download_stats = ProgressStats::new();
            let mut upload_stats = ProgressStats::new();
            PROGRESS_ACTIVE.store(true, Ordering::SeqCst);
            loop {
                if shutdown_requested() {
                    break;
                }
                let (mut snapshot, current_id) = match state.lock() {
                    Ok(guard) => (
                        ProgressSnapshot {
                            downloaded_bytes: 0,
                            completed_bytes: guard.completed_bytes,
                            total_bytes: guard.total_bytes,
                            active_peers: guard.active_peers,
                            tracker_peers: guard.tracker_peers,
                            status: guard.status.clone(),
                        },
                        guard.current_id,
                    ),
                    Err(_) => (ProgressSnapshot::default(), None),
                };
                let (downloaded_bytes, uploaded_bytes) = current_id
                    .and_then(|id| find_context_by_id(&registry, id))
                    .map(|ctx| {
                        (
                            ctx.downloaded.load(Ordering::SeqCst),
                            ctx.uploaded.load(Ordering::SeqCst),
                        )
                    })
                    .unwrap_or((0, 0));
                snapshot.downloaded_bytes = downloaded_bytes;

                let (mut line, _speed_bps, eta_secs) = download_stats.render_line(&snapshot);
                upload_stats.update_speed(uploaded_bytes);
                let _upload_speed = upload_stats.current_speed();
                if line.len() < download_stats.last_line_len {
                    line.push_str(&" ".repeat(download_stats.last_line_len - line.len()));
                }
                download_stats.last_line_len = line.len();
                PROGRESS_LINE_LEN.store(download_stats.last_line_len, Ordering::SeqCst);
                let _guard = LOG_LOCK.lock().ok();
                eprint!("\r{line}");
                let _ = io::stderr().flush();

                let metrics = storage::metrics_snapshot();
                let read_ms = if metrics.read_ops > 0 {
                    metrics.read_ns as f64 / metrics.read_ops as f64 / 1_000_000.0
                } else {
                    0.0
                };
                let write_ms = if metrics.write_ops > 0 {
                    metrics.write_ns as f64 / metrics.write_ops as f64 / 1_000_000.0
                } else {
                    0.0
                };
                if let Ok(mut guard) = state.lock() {
                    let (session_download_rate, session_upload_rate) =
                        aggregate_session_rates(&guard);
                    guard.download_rate_bps = session_download_rate;
                    guard.upload_rate_bps = session_upload_rate;
                    guard.eta_secs = eta_secs;
                    guard.paused = is_paused();
                    guard.downloaded_bytes = downloaded_bytes;
                    guard.uploaded_bytes = uploaded_bytes;
                    guard.session_downloaded_bytes =
                        SESSION_DOWNLOADED_BYTES.load(Ordering::SeqCst);
                    guard.session_uploaded_bytes = SESSION_UPLOADED_BYTES.load(Ordering::SeqCst);
                    guard.peer_connected = PEER_CONNECTED.load(Ordering::SeqCst);
                    guard.peer_disconnected = PEER_DISCONNECTED.load(Ordering::SeqCst);
                    guard.disk_read_ms_avg = read_ms;
                    guard.disk_write_ms_avg = write_ms;
                    push_speed_sample(&mut guard.download_history_bps, session_download_rate);
                    push_speed_sample(&mut guard.upload_history_bps, session_upload_rate);
                }

                sleep_with_shutdown(Duration::from_secs(1));
            }
            PROGRESS_ACTIVE.store(false, Ordering::SeqCst);
            eprintln!();
        })
        .map_err(|err| format!("console progress worker could not start: {err}"))
}

fn human_bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} {}", UNITS[unit])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

fn human_rate(bps: f64) -> String {
    const UNITS: [&str; 5] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    if !bps.is_finite() || bps <= 0.0 {
        return "0 B/s".to_string();
    }
    let mut size = bps;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{:.0} {}", size, UNITS[unit])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

fn format_eta(secs: f64) -> String {
    if !secs.is_finite() || secs <= 0.0 {
        return "--:--".to_string();
    }
    let total = secs.round() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompletedMove {
    source: PathBuf,
    destination: PathBuf,
}

fn single_file_source_override(
    meta: &torrent::TorrentMeta,
    src_dir: &Path,
    file_renames: &[(usize, String)],
) -> Result<Option<PathBuf>, String> {
    if meta.info.length.is_none() || !file_renames.iter().any(|(index, _)| *index == 0) {
        return Ok(None);
    }
    let paths = storage::data_paths_with_file_renames(meta, src_dir, file_renames)
        .map_err(|err| format!("renamed completion source: {err}"))?;
    paths
        .into_iter()
        .next()
        .map(Some)
        .ok_or_else(|| "single-file torrent has no storage path".to_string())
}

fn inspect_regular_path(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!("unsafe non-regular path: {}", path.display()));
            }
            Ok(Some(metadata))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("inspect {}: {err}", path.display())),
    }
}

fn reconcile_pending_file_rename(
    meta: &torrent::TorrentMeta,
    download_dir: &Path,
    session_store: &SessionStore,
    entry: &SessionEntry,
) -> Result<Vec<(usize, String)>, String> {
    let Some(pending) = entry.pending_file_rename.as_ref() else {
        return Ok(entry.file_renames.clone());
    };
    let paths = if entry.file_renames.is_empty() {
        storage::data_paths(meta, download_dir)
    } else {
        storage::data_paths_with_file_renames(meta, download_dir, &entry.file_renames)
    }
    .map_err(|err| format!("rename storage paths: {err}"))?;
    let old_path = paths
        .get(pending.index)
        .cloned()
        .ok_or_else(|| "pending file rename index is out of range".to_string())?;
    let new_path = old_path.with_file_name(&pending.target);
    let mut prospective = entry.file_renames.clone();
    if let Some((_, target)) = prospective
        .iter_mut()
        .find(|(index, _)| *index == pending.index)
    {
        *target = pending.target.clone();
    } else {
        prospective.push((pending.index, pending.target.clone()));
    }
    storage::data_paths_with_file_renames(meta, download_dir, &prospective)
        .map_err(|err| format!("pending rename validation failed: {err}"))?;
    if new_path == old_path {
        if !session_store.commit_file_rename(entry.info_hash, pending)? {
            return Err("pending file rename changed during commit".to_string());
        }
        return session_store
            .get(entry.info_hash)
            .map(|entry| entry.file_renames)
            .ok_or_else(|| "torrent session metadata is unavailable".to_string());
    }
    let old_exists = inspect_regular_path(&old_path)?.is_some();
    let new_exists = inspect_regular_path(&new_path)?.is_some();
    match (old_exists, new_exists) {
        (true, false) => {
            rename_path_no_overwrite(&old_path, &new_path, false).map_err(|err| {
                format!(
                    "resume pending rename {} to {}: {err}",
                    old_path.display(),
                    new_path.display()
                )
            })?;
            match session_store.commit_file_rename(entry.info_hash, pending) {
                Ok(true) => {}
                Ok(false) => {
                    let _ = rename_path_no_overwrite(&new_path, &old_path, false);
                    return Err("pending file rename changed during commit".to_string());
                }
                Err(err) => {
                    let rollback = rename_path_no_overwrite(&new_path, &old_path, false);
                    return Err(match rollback {
                        Ok(()) => format!("{err}; physical rename was rolled back"),
                        Err(rollback_err) => {
                            format!("{err}; physical rename rollback failed: {rollback_err}")
                        }
                    });
                }
            }
        }
        (false, true) => {
            if !session_store.commit_file_rename(entry.info_hash, pending)? {
                return Err("pending file rename changed during recovery".to_string());
            }
        }
        (true, true) => return Err("both old and new pending file rename paths exist".to_string()),
        (false, false) => {
            return Err("neither old nor new pending file rename path exists".to_string())
        }
    }
    session_store
        .get(entry.info_hash)
        .map(|entry| entry.file_renames)
        .ok_or_else(|| "torrent session metadata is unavailable".to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionMoveRecovery {
    RetryMove,
    AdoptDestination { remove_source: bool },
}

fn safe_payload_root_exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
                return Err(format!("unsafe payload root: {}", path.display()));
            }
            Ok(true)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(format!("inspect payload root {}: {err}", path.display())),
    }
}

fn verify_existing_payload(
    meta: &torrent::TorrentMeta,
    download_dir: &Path,
    file_renames: &[(usize, String)],
) -> Result<bool, String> {
    let mut storage =
        storage::Storage::open_existing_with_file_renames(meta, download_dir, file_renames)
            .map_err(|err| format!("open completion destination: {err}"))?;
    let mut pieces = piece::PieceManager::new(meta)
        .map_err(|err| format!("completion verification pieces: {err}"))?;
    full_recheck(&mut pieces, &mut storage, meta.info.piece_length, None)?;
    Ok(pieces.is_complete())
}

fn completion_move_recovery(
    meta: &torrent::TorrentMeta,
    destination_dir: &Path,
    file_renames: &[(usize, String)],
    completed_move: &CompletedMove,
) -> Result<CompletionMoveRecovery, String> {
    if completed_move.source == completed_move.destination {
        return Ok(CompletionMoveRecovery::RetryMove);
    }
    let source_exists = safe_payload_root_exists(&completed_move.source)?;
    let destination_exists = safe_payload_root_exists(&completed_move.destination)?;
    if !destination_exists {
        return if source_exists {
            Ok(CompletionMoveRecovery::RetryMove)
        } else {
            Err("neither completion source nor destination exists".to_string())
        };
    }
    if !verify_existing_payload(meta, destination_dir, file_renames)? {
        return Err(format!(
            "completion destination failed payload verification: {}",
            completed_move.destination.display()
        ));
    }
    Ok(CompletionMoveRecovery::AdoptDestination {
        remove_source: source_exists,
    })
}

fn completed_move_paths(
    meta: &torrent::TorrentMeta,
    src_dir: &Path,
    dest_dir: &Path,
    source_override: Option<&Path>,
) -> Result<CompletedMove, String> {
    let src_path = match source_override {
        Some(path) if meta.info.length.is_some() => {
            let relative = path
                .strip_prefix(src_dir)
                .map_err(|_| "renamed source is outside the download directory".to_string())?;
            if relative.components().count() != 1
                || !relative
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)))
            {
                return Err("renamed source path is invalid".to_string());
            }
            path.to_path_buf()
        }
        Some(_) => {
            return Err("source override is only valid for a single-file torrent".to_string())
        }
        None => storage::root_path(meta, src_dir)
            .map_err(|err| format!("invalid source path: {err}"))?,
    };
    let dest_path = if source_override.is_some() {
        let name = src_path
            .file_name()
            .ok_or_else(|| "renamed source has no file name".to_string())?;
        dest_dir.join(name)
    } else {
        storage::root_path(meta, dest_dir)
            .map_err(|err| format!("invalid destination path: {err}"))?
    };
    Ok(CompletedMove {
        source: src_path,
        destination: dest_path,
    })
}

fn move_completed_files(
    meta: &torrent::TorrentMeta,
    src_dir: &Path,
    dest_dir: &Path,
    source_override: Option<&Path>,
) -> Result<Option<CompletedMove>, String> {
    fs::create_dir_all(dest_dir)
        .map_err(|err| format!("failed to create destination {}: {err}", dest_dir.display()))?;
    let completed_move = completed_move_paths(meta, src_dir, dest_dir, source_override)?;
    if completed_move.source == completed_move.destination {
        return Ok(None);
    }
    move_path_no_overwrite(&completed_move.source, &completed_move.destination)?;
    log_info!(
        "moved completed: {} -> {}",
        completed_move.source.display(),
        completed_move.destination.display()
    );
    Ok(Some(completed_move))
}

fn move_path_no_overwrite(src_path: &Path, dest_path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(dest_path) {
        Ok(_) => {
            return Err(format!(
                "destination already exists: {}",
                dest_path.display()
            ));
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(format!(
                "inspect destination {}: {err}",
                dest_path.display()
            ));
        }
    }
    let source_meta = fs::symlink_metadata(src_path)
        .map_err(|err| format!("inspect {}: {err}", src_path.display()))?;
    if source_meta.file_type().is_symlink() || (!source_meta.is_file() && !source_meta.is_dir()) {
        return Err(format!(
            "refusing to move unsafe source {}",
            src_path.display()
        ));
    }
    match rename_path_no_overwrite(src_path, dest_path, source_meta.is_dir()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Err(format!(
            "destination already exists: {}",
            dest_path.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::CrossesDevices => {
            // Copy into a sibling staging path, then atomically publish it.
            // This prevents a failed copy from leaving a partial destination
            // that a later run could mistake for completed data.
            let staging = move_staging_path(dest_path)?;
            let copy_result = if src_path.is_dir() {
                copy_dir_recursive(src_path, &staging)
            } else {
                copy_regular_file(src_path, &staging)
            };
            if let Err(err) = copy_result {
                let _ = remove_path_if_present(&staging);
                return Err(err);
            }
            if let Err(err) = rename_path_no_overwrite(&staging, dest_path, source_meta.is_dir()) {
                let _ = remove_path_if_present(&staging);
                return Err(format!("publish copied data: {err}"));
            }
            if let Err(err) = remove_path_if_present(src_path) {
                // Recursive source cleanup can fail after removing only part of
                // a directory. The published destination is the only known
                // complete copy at that point, so never delete it as a
                // rollback. The durable Pending completion journal will verify
                // and adopt it on recovery, then clean exact source data paths.
                return Err(format!(
                    "copied data but failed to remove source: {err}; published destination retained for recovery"
                ));
            }
            Ok(())
        }
        Err(err) => Err(format!(
            "rename {} to {} failed: {err}",
            src_path.display(),
            dest_path.display()
        )),
    }
}

/// Atomically moves `source` to an absent `destination` without ever replacing
/// a directory entry created by another process between validation and publish.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn rename_path_no_overwrite(
    source: &Path,
    destination: &Path,
    source_is_dir: bool,
) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let source_c = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "source path contains NUL"))?;
    let destination_c =
        std::ffi::CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "destination path contains NUL")
        })?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source_c.as_ptr(),
            libc::AT_FDCWD,
            destination_c.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if matches!(
        error.raw_os_error(),
        Some(code)
            if code == libc::ENOSYS || code == libc::EINVAL || code == libc::EOPNOTSUPP
    ) {
        return rename_path_no_overwrite_fallback(source, destination, source_is_dir);
    }
    Err(error)
}

#[cfg(target_vendor = "apple")]
pub(crate) fn rename_path_no_overwrite(
    source: &Path,
    destination: &Path,
    source_is_dir: bool,
) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let source_c = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "source path contains NUL"))?;
    let destination_c =
        std::ffi::CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "destination path contains NUL")
        })?;
    let result =
        unsafe { libc::renamex_np(source_c.as_ptr(), destination_c.as_ptr(), libc::RENAME_EXCL) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if matches!(
        error.raw_os_error(),
        Some(code) if code == libc::EINVAL || code == libc::ENOTSUP
    ) {
        return rename_path_no_overwrite_fallback(source, destination, source_is_dir);
    }
    Err(error)
}

#[cfg(windows)]
pub(crate) fn rename_path_no_overwrite(
    source: &Path,
    destination: &Path,
    _source_is_dir: bool,
) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileW(existing: *const u16, new: *const u16) -> i32;
    }

    fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
        let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
        if wide.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path contains NUL",
            ));
        }
        wide.push(0);
        Ok(wide)
    }

    let source = wide_path(source)?;
    let destination = wide_path(destination)?;
    if unsafe { MoveFileW(source.as_ptr(), destination.as_ptr()) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_vendor = "apple"))
))]
pub(crate) fn rename_path_no_overwrite(
    source: &Path,
    destination: &Path,
    source_is_dir: bool,
) -> io::Result<()> {
    rename_path_no_overwrite_fallback(source, destination, source_is_dir)
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn rename_path_no_overwrite(
    source: &Path,
    destination: &Path,
    source_is_dir: bool,
) -> io::Result<()> {
    rename_path_no_overwrite_fallback(source, destination, source_is_dir)
}

#[cfg(not(windows))]
fn rename_path_no_overwrite_fallback(
    source: &Path,
    destination: &Path,
    source_is_dir: bool,
) -> io::Result<()> {
    if source_is_dir {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic no-replace directory moves are unsupported on this filesystem",
        ));
    }
    fs::hard_link(source, destination)?;
    fs::remove_file(source)
}

fn rollback_completed_move(completed_move: &CompletedMove) -> Result<(), String> {
    move_path_no_overwrite(&completed_move.destination, &completed_move.source)?;
    log_info!(
        "rolled back completed move: {} -> {}",
        completed_move.destination.display(),
        completed_move.source.display()
    );
    Ok(())
}

fn commit_completed_move<F>(completed_move: Option<&CompletedMove>, commit: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    match commit() {
        Ok(()) => Ok(()),
        Err(commit_err) => {
            let Some(completed_move) = completed_move else {
                return Err(commit_err);
            };
            match rollback_completed_move(completed_move) {
                Ok(()) => Err(format!("{commit_err}; completed move was rolled back")),
                Err(rollback_err) => Err(format!(
                    "{commit_err}; completed move rollback failed: {rollback_err}"
                )),
            }
        }
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    let source_meta = fs::symlink_metadata(src).map_err(|err| err.to_string())?;
    if source_meta.file_type().is_symlink() || !source_meta.is_dir() {
        return Err(format!("refusing to copy non-directory {}", src.display()));
    }
    fs::create_dir(dest).map_err(|err| err.to_string())?;
    let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        let dest_path = dest.join(entry.file_name());
        if ft.is_symlink() {
            return Err(format!(
                "refusing to copy symbolic link {}",
                entry.path().display()
            ));
        } else if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else if ft.is_file() {
            copy_regular_file(&entry.path(), &dest_path)?;
        } else {
            return Err(format!("unsupported file type: {}", entry.path().display()));
        }
    }
    Ok(())
}

fn copy_regular_file(src: &Path, dest: &Path) -> Result<(), String> {
    let source_meta = fs::symlink_metadata(src).map_err(|err| err.to_string())?;
    if source_meta.file_type().is_symlink() || !source_meta.is_file() {
        return Err(format!("refusing to copy non-file {}", src.display()));
    }
    let mut input = fs::File::open(src).map_err(|err| err.to_string())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)
        .map_err(|err| err.to_string())?;
    io::copy(&mut input, &mut output).map_err(|err| err.to_string())?;
    output.sync_all().map_err(|err| err.to_string())?;
    Ok(())
}

fn move_staging_path(dest: &Path) -> Result<PathBuf, String> {
    let parent = dest
        .parent()
        .ok_or_else(|| "destination has no parent".to_string())?;
    let file_name = dest
        .file_name()
        .ok_or_else(|| "destination has no file name".to_string())?;
    for attempt in 0..32u64 {
        let mut staging_name = file_name.to_os_string();
        staging_name.push(format!(
            ".rustorrent-moving-{}-{:016x}",
            std::process::id(),
            system_entropy_u64().wrapping_add(attempt)
        ));
        let candidate = parent.join(staging_name);
        if fs::symlink_metadata(&candidate).is_err() {
            return Ok(candidate);
        }
    }
    Err("could not allocate a move staging path".to_string())
}

fn remove_path_if_present(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[derive(Debug)]
struct CreateFile {
    source_path: PathBuf,
    path_segments: Vec<Vec<u8>>,
    length: u64,
}

fn create_torrent(
    source_path: &Path,
    tracker_url: &str,
    output_path: &Path,
    piece_length: u64,
) -> Result<(), String> {
    if piece_length == 0 || piece_length > u32::MAX as u64 {
        return Err("piece length must be between 1 and 4294967295 bytes".to_string());
    }
    if tracker_url.trim().is_empty() {
        return Err("missing --tracker URL".to_string());
    }
    if !valid_tracker_url(tracker_url) {
        return Err("tracker URL must be a valid http, https, or udp URL".to_string());
    }

    let source_meta = fs::symlink_metadata(source_path)
        .map_err(|err| format!("inspect {}: {err}", source_path.display()))?;
    if source_meta.file_type().is_symlink() {
        return Err("torrent source must not be a symbolic link".to_string());
    }
    if !source_meta.is_file() && !source_meta.is_dir() {
        return Err("torrent source must be a regular file or directory".to_string());
    }
    let source_path = fs::canonicalize(source_path)
        .map_err(|err| format!("resolve {}: {err}", source_path.display()))?;
    let resolved_output = resolve_path_for_safety(output_path)?;
    if resolved_output == source_path {
        return Err("output path must not overwrite the source".to_string());
    }
    if source_meta.is_dir() && resolved_output.starts_with(&source_path) {
        return Err("torrent output must be outside the source directory".to_string());
    }

    let name = source_path
        .file_name()
        .map(os_str_to_torrent_bytes)
        .ok_or("invalid source path")?;

    let multi_file = source_meta.is_dir();
    let mut files_data = Vec::new();
    if multi_file {
        collect_files(&source_path, &[], &mut files_data)?;
        files_data.sort_by(|a, b| a.path_segments.cmp(&b.path_segments));
    } else {
        files_data.push(CreateFile {
            source_path: source_path.clone(),
            path_segments: Vec::new(),
            length: source_meta.len(),
        });
    }

    let total_length = files_data.iter().try_fold(0u64, |total, file| {
        total
            .checked_add(file.length)
            .ok_or_else(|| "source is too large".to_string())
    })?;
    if total_length == 0 {
        return Err("empty source".to_string());
    }
    if total_length > i64::MAX as u64 || files_data.iter().any(|file| file.length > i64::MAX as u64)
    {
        return Err("source is too large for torrent metainfo".to_string());
    }

    let pieces_bytes = hash_create_files(&files_data, piece_length)?;

    let mut info_items = vec![
        (b"name".to_vec(), bencode::Value::Bytes(name)),
        (
            b"piece length".to_vec(),
            bencode::Value::Int(piece_length as i64),
        ),
        (b"pieces".to_vec(), bencode::Value::Bytes(pieces_bytes)),
    ];

    if multi_file {
        let file_list: Vec<bencode::Value> = files_data
            .iter()
            .map(|file| {
                let path_values: Vec<bencode::Value> = file
                    .path_segments
                    .iter()
                    .map(|s| bencode::Value::Bytes(s.clone()))
                    .collect();
                bencode::Value::Dict(vec![
                    (b"length".to_vec(), bencode::Value::Int(file.length as i64)),
                    (b"path".to_vec(), bencode::Value::List(path_values)),
                ])
            })
            .collect();
        info_items.push((b"files".to_vec(), bencode::Value::List(file_list)));
    } else {
        info_items.push((b"length".to_vec(), bencode::Value::Int(total_length as i64)));
    }

    info_items.sort_by(|a, b| a.0.cmp(&b.0));
    let info = bencode::Value::Dict(info_items);
    let info_encoded = bencode::encode(&info);
    let info_hash = sha1::sha1(&info_encoded);

    let mut torrent_items = vec![
        (
            b"announce".to_vec(),
            bencode::Value::Bytes(tracker_url.as_bytes().to_vec()),
        ),
        (b"info".to_vec(), info),
    ];
    torrent_items.sort_by(|a, b| a.0.cmp(&b.0));
    let torrent = bencode::Value::Dict(torrent_items);
    let encoded = bencode::encode(&torrent);

    write_atomic_file(output_path, &encoded, "torrent", true, false)?;

    log_info!(
        "created torrent: {} ({} bytes, {} pieces, info_hash: {})",
        output_path.display(),
        total_length,
        total_length.div_ceil(piece_length),
        hex(&info_hash)
    );
    Ok(())
}

fn resolve_path_for_safety(path: &Path) -> Result<PathBuf, String> {
    let mut cursor = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|err| format!("resolve output directory: {err}"))?
            .join(path)
    };
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    loop {
        match fs::canonicalize(&cursor) {
            Ok(mut resolved) => {
                for component in missing.iter().rev() {
                    match component.as_os_str() {
                        value if value == "." => {}
                        value if value == ".." => {
                            if !resolved.pop() {
                                return Err("output path escapes the filesystem root".to_string());
                            }
                        }
                        value => resolved.push(value),
                    }
                }
                return Ok(resolved);
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let component = cursor
                    .file_name()
                    .ok_or_else(|| format!("resolve output path {}: {err}", path.display()))?;
                missing.push(component.to_os_string());
                if !cursor.pop() {
                    return Err(format!("resolve output path {}: {err}", path.display()));
                }
            }
            Err(err) => {
                return Err(format!("resolve output path {}: {err}", path.display()));
            }
        }
    }
}

fn hash_create_files(files: &[CreateFile], piece_length: u64) -> Result<Vec<u8>, String> {
    let mut pieces = Vec::new();
    let mut hasher = sha1::Sha1::new();
    let mut bytes_in_piece = 0u64;
    let mut read_buffer = [0u8; 64 * 1024];

    for entry in files {
        let mut file = fs::File::open(&entry.source_path)
            .map_err(|err| format!("read {}: {err}", entry.source_path.display()))?;
        let mut file_bytes = 0u64;
        loop {
            let read = file
                .read(&mut read_buffer)
                .map_err(|err| format!("read {}: {err}", entry.source_path.display()))?;
            if read == 0 {
                break;
            }
            file_bytes = file_bytes
                .checked_add(read as u64)
                .ok_or_else(|| "source changed size while hashing".to_string())?;
            let mut chunk = &read_buffer[..read];
            while !chunk.is_empty() {
                let remaining = piece_length - bytes_in_piece;
                let take = chunk.len().min(remaining as usize);
                hasher.update(&chunk[..take]);
                bytes_in_piece += take as u64;
                chunk = &chunk[take..];
                if bytes_in_piece == piece_length {
                    pieces.extend_from_slice(&hasher.finalize());
                    hasher = sha1::Sha1::new();
                    bytes_in_piece = 0;
                }
            }
        }
        if file_bytes != entry.length {
            return Err(format!(
                "source changed size while hashing: {}",
                entry.source_path.display()
            ));
        }
    }
    if bytes_in_piece > 0 {
        pieces.extend_from_slice(&hasher.finalize());
    }
    Ok(pieces)
}

fn collect_files(dir: &Path, prefix: &[Vec<u8>], out: &mut Vec<CreateFile>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    let mut entries: Vec<_> = entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read_dir {}: {err}", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_symlink() {
            return Err(format!(
                "symbolic links are not supported in torrent sources: {}",
                entry.path().display()
            ));
        }
        let name_bytes = os_str_to_torrent_bytes(&entry.file_name());
        let mut path_segments = prefix.to_vec();
        path_segments.push(name_bytes);
        if ft.is_dir() {
            collect_files(&entry.path(), &path_segments, out)?;
        } else if ft.is_file() {
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            out.push(CreateFile {
                source_path: entry.path(),
                path_segments,
                length: meta.len(),
            });
        } else {
            return Err(format!(
                "unsupported source entry: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn os_str_to_torrent_bytes(value: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_str_to_torrent_bytes(value: &std::ffi::OsStr) -> Vec<u8> {
    value.to_string_lossy().into_owned().into_bytes()
}

#[allow(clippy::too_many_arguments)]
fn scan_watch_dir(
    watch_dir: &Path,
    queue: &mut VecDeque<TorrentRequest>,
    ui_state: &Option<Arc<Mutex<ui::UiState>>>,
    next_id: &mut u64,
    download_dir: &Path,
    preallocate: bool,
    registry: &SessionRegistry,
    session_store: &SessionStore,
    in_flight: &InFlightTorrents,
) {
    let entries = match fs::read_dir(watch_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let processed_dir = watch_dir.join("processed");
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("torrent") {
            continue;
        }
        let data = match read_file_limited(&path, MAX_TORRENT_BYTES, true) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let request = TorrentRequest {
            id: *next_id,
            source: TorrentSource::Bytes(data),
            download_dir: download_dir.to_path_buf(),
            preallocate,
            initial_label: String::new(),
        };
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "watch".to_string());
        if enqueue_request_if_new(
            registry,
            queue,
            session_store,
            in_flight,
            ui_state,
            request,
            Some(label),
        ) {
            *next_id = next_id.saturating_add(1);
        }
        // Move to processed
        let _ = fs::create_dir_all(&processed_dir);
        if let Some(name) = path.file_name() {
            let _ = fs::rename(&path, processed_dir.join(name));
        }
    }
}

// ---- TUI (--tui) ----

struct TuiState {
    selected: usize,
    scroll_offset: usize,
    show_detail: bool,
    confirm_delete: Option<u64>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn tui_terminal_size() -> (u16, u16) {
    #[repr(C)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }
    extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    #[cfg(target_os = "macos")]
    const TIOCGWINSZ: u64 = 0x40087468;
    #[cfg(target_os = "linux")]
    const TIOCGWINSZ: u64 = 0x5413;
    let mut ws = Winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        ioctl(1, TIOCGWINSZ, &mut ws as *mut Winsize);
    }
    (ws.ws_row, ws.ws_col)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn tui_terminal_size() -> (u16, u16) {
    (24, 80)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn tui_set_raw_mode() -> Option<[u8; 128]> {
    #[cfg(target_os = "macos")]
    const TERMIOS_SIZE: usize = 72;
    #[cfg(target_os = "linux")]
    const TERMIOS_SIZE: usize = 60;
    extern "C" {
        fn tcgetattr(fd: i32, termios: *mut u8) -> i32;
        fn tcsetattr(fd: i32, action: i32, termios: *const u8) -> i32;
    }
    let mut original = [0u8; 128];
    let mut raw = [0u8; 128];
    if unsafe { tcgetattr(0, original.as_mut_ptr()) } != 0 {
        return None;
    }
    raw[..TERMIOS_SIZE].copy_from_slice(&original[..TERMIOS_SIZE]);
    // Clear ICANON and ECHO (both in c_lflag)
    // c_lflag offset: macOS=16, Linux=12
    #[cfg(target_os = "macos")]
    const LFLAG_OFFSET: usize = 16;
    #[cfg(target_os = "linux")]
    const LFLAG_OFFSET: usize = 12;
    let lflag = u64::from_ne_bytes([
        raw[LFLAG_OFFSET],
        raw[LFLAG_OFFSET + 1],
        raw.get(LFLAG_OFFSET + 2).copied().unwrap_or(0),
        raw.get(LFLAG_OFFSET + 3).copied().unwrap_or(0),
        0,
        0,
        0,
        0,
    ]) as u32;
    // ICANON=0x100, ECHO=0x8 on macOS; ICANON=2, ECHO=8 on Linux
    #[cfg(target_os = "macos")]
    let new_lflag = lflag & !(0x100 | 0x8);
    #[cfg(target_os = "linux")]
    let new_lflag = lflag & !(0x2 | 0x8);
    let bytes = new_lflag.to_ne_bytes();
    raw[LFLAG_OFFSET..LFLAG_OFFSET + 4].copy_from_slice(&bytes);
    // Set VMIN=1, VTIME=0 for non-blocking-ish reads
    // c_cc offset: macOS=20, Linux=17
    #[cfg(target_os = "macos")]
    {
        raw[20 + 16] = 0; // VMIN index=16 on macOS -> set to 0 for non-blocking
        raw[20 + 17] = 1; // VTIME index=17 -> 0.1s timeout
    }
    #[cfg(target_os = "linux")]
    {
        raw[17 + 6] = 0; // VMIN
        raw[17 + 5] = 1; // VTIME
    }

    if unsafe { tcsetattr(0, 0, raw.as_ptr()) } != 0 {
        return None;
    }
    Some(original)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn tui_set_raw_mode() -> Option<[u8; 128]> {
    None
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn tui_restore_mode(original: &[u8; 128]) {
    extern "C" {
        fn tcsetattr(fd: i32, action: i32, termios: *const u8) -> i32;
    }
    unsafe {
        tcsetattr(0, 0, original.as_ptr());
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn tui_restore_mode(_original: &[u8; 128]) {}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn tui_read_key() -> Option<u8> {
    extern "C" {
        fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    }
    let mut buf = [0u8; 1];
    let n = unsafe { read(0, buf.as_mut_ptr(), 1) };
    if n == 1 {
        Some(buf[0])
    } else {
        None
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn tui_read_key() -> Option<u8> {
    None
}

fn tui_read_escape_seq() -> Vec<u8> {
    let mut seq = Vec::new();
    for _ in 0..4 {
        if let Some(b) = tui_read_key() {
            seq.push(b);
            if b.is_ascii_alphabetic() || b == b'~' {
                break;
            }
        } else {
            break;
        }
    }
    seq
}

fn tui_format_bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value}{}", UNITS[unit])
    } else {
        format!("{:.1}{}", size, UNITS[unit])
    }
}

fn tui_format_rate(bps: f64) -> String {
    if bps <= 0.0 || !bps.is_finite() {
        return "0B/s".to_string();
    }
    let formatted = tui_format_bytes(bps as u64);
    format!("{formatted}/s")
}

fn tui_progress_bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    let mut bar = String::with_capacity(width * 3);
    for _ in 0..filled {
        bar.push('\u{2588}');
    }
    for _ in 0..empty {
        bar.push('\u{2591}');
    }
    bar
}

fn tui_status_color(status: &str) -> &'static str {
    match status {
        "seeding" | "complete" => "\x1b[32m", // green
        "downloading" => "\x1b[33m",          // yellow
        "error" => "\x1b[31m",                // red
        "paused" => "\x1b[90m",               // gray
        "announcing" | "loading" | "fetching metadata" => "\x1b[36m", // cyan
        _ => "\x1b[0m",
    }
}

fn tui_status_icon(status: &str, paused: bool) -> &'static str {
    if paused {
        "\u{23f8}"
    } else {
        match status {
            "seeding" | "complete" => "\u{25b2}",
            "downloading" | "announcing" => "\u{25b6}",
            "error" => "\u{2717}",
            _ => " ",
        }
    }
}

fn start_tui(
    state: Arc<Mutex<ui::UiState>>,
    cmd_tx: mpsc::Sender<ui::UiCommand>,
) -> Result<thread::JoinHandle<()>, String> {
    thread::Builder::new()
        .name("terminal-ui".to_string())
        .spawn(move || {
        let original = match tui_set_raw_mode() {
            Some(orig) => orig,
            None => {
                log_warn!("tui: failed to set raw mode");
                return;
            }
        };
        // Enter alternate screen, hide cursor
        let stdout = io::stdout();
        {
            let mut out = stdout.lock();
            let _ = out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J");
            let _ = out.flush();
        }

        let mut tui = TuiState {
            selected: 0,
            scroll_offset: 0,
            show_detail: false,
            confirm_delete: None,
        };

        loop {
            if shutdown_requested() {
                break;
            }

            // Read keyboard input
            if let Some(key) = tui_read_key() {
                match key {
                    b'q' | 3 => {
                        // q or Ctrl-C
                        SHUTDOWN.store(true, Ordering::SeqCst);
                        break;
                    }
                    b'j' | b'B' => {
                        // down (j or arrow down sequence starts with ESC)
                        tui.selected = tui.selected.saturating_add(1);
                        tui.confirm_delete = None;
                    }
                    b'k' | b'A' => {
                        // up
                        tui.selected = tui.selected.saturating_sub(1);
                        tui.confirm_delete = None;
                    }
                    0x1b => {
                        // ESC - start of escape sequence
                        let seq = tui_read_escape_seq();
                        if seq == *b"[A" {
                            tui.selected = tui.selected.saturating_sub(1);
                        } else if seq == *b"[B" {
                            tui.selected = tui.selected.saturating_add(1);
                        }
                        tui.confirm_delete = None;
                    }
                    b'\r' | b'\n' => {
                        tui.show_detail = !tui.show_detail;
                    }
                    b'p' => {
                        // Pause/resume selected torrent
                        let guard = lock_or_recover(&state);
                        if let Some(torrent) = guard.torrents.get(tui.selected) {
                            let id = torrent.id;
                            let paused = torrent.paused;
                            drop(guard);
                            let (reply_tx, _) = mpsc::channel();
                            if paused {
                                let _ = cmd_tx.send(ui::UiCommand::ResumeTorrent {
                                    torrent_id: id,
                                    reply: reply_tx,
                                });
                            } else {
                                let _ = cmd_tx.send(ui::UiCommand::PauseTorrent {
                                    torrent_id: id,
                                    reply: reply_tx,
                                });
                            }
                        }
                    }
                    b's' => {
                        // Stop selected torrent
                        let guard = lock_or_recover(&state);
                        if let Some(torrent) = guard.torrents.get(tui.selected) {
                            let id = torrent.id;
                            drop(guard);
                            let (reply_tx, _) = mpsc::channel();
                            let _ = cmd_tx.send(ui::UiCommand::StopTorrent {
                                torrent_id: id,
                                reply: reply_tx,
                            });
                        }
                    }
                    b'd' => {
                        // Delete selected torrent (requires confirmation)
                        let guard = lock_or_recover(&state);
                        if let Some(torrent) = guard.torrents.get(tui.selected) {
                            if tui.confirm_delete == Some(torrent.id) {
                                // Already confirming - ignore, wait for y/n
                            } else {
                                tui.confirm_delete = Some(torrent.id);
                            }
                        }
                    }
                    b'y' => {
                        if let Some(id) = tui.confirm_delete.take() {
                            let (reply_tx, _) = mpsc::channel();
                            let _ = cmd_tx.send(ui::UiCommand::DeleteTorrent {
                                torrent_id: id,
                                remove_data: false,
                                reply: reply_tx,
                            });
                        }
                    }
                    b'n' => {
                        tui.confirm_delete = None;
                    }
                    b'r' => {
                        // Recheck selected torrent
                        let guard = lock_or_recover(&state);
                        if let Some(torrent) = guard.torrents.get(tui.selected) {
                            let id = torrent.id;
                            drop(guard);
                            let (reply_tx, _) = mpsc::channel();
                            let _ = cmd_tx.send(ui::UiCommand::RecheckTorrent {
                                torrent_id: id,
                                reply: reply_tx,
                            });
                        }
                    }
                    _ => {}
                }
            }

            // Render frame
            let (rows, cols) = tui_terminal_size();
            let cols = cols as usize;
            let rows = rows as usize;
            if rows < 5 || cols < 30 {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let guard = lock_or_recover(&state);
            let torrent_count = guard.torrents.len();
            if torrent_count > 0 && tui.selected >= torrent_count {
                tui.selected = torrent_count - 1;
            }

            // Calculate layout
            let header_rows = 1;
            let footer_rows = 1;
            let detail_rows = if tui.show_detail { 6.min(rows / 3) } else { 0 };
            let list_rows = rows.saturating_sub(header_rows + footer_rows + detail_rows);

            // Adjust scroll
            if tui.selected < tui.scroll_offset {
                tui.scroll_offset = tui.selected;
            }
            if tui.selected >= tui.scroll_offset + list_rows {
                tui.scroll_offset = tui.selected.saturating_sub(list_rows - 1);
            }

            let mut frame = String::with_capacity(cols * rows * 3);

            // Header: status bar
            frame.push_str("\x1b[H");
            let total_down_rate: f64 = guard.torrents.iter().map(|t| t.download_rate_bps).sum();
            let total_up_rate: f64 = guard.torrents.iter().map(|t| t.upload_rate_bps).sum();
            let active = guard
                .torrents
                .iter()
                .filter(|t| t.status == "downloading" || t.status == "seeding")
                .count();
            let header = format!(
                " \x1b[1mRustorrent 0.1.0\x1b[0m  \x1b[32m\u{2193}\x1b[0m {} \x1b[36m\u{2191}\x1b[0m {}  [{}/{}]",
                tui_format_rate(total_down_rate),
                tui_format_rate(total_up_rate),
                active,
                torrent_count,
            );
            frame.push_str(&header);
            let header_visible = strip_ansi_len(&header);
            if header_visible < cols {
                for _ in 0..(cols - header_visible) {
                    frame.push(' ');
                }
            }

            // Torrent list
            for row in 0..list_rows {
                let idx = tui.scroll_offset + row;
                frame.push_str(&format!("\x1b[{};1H", header_rows + row + 1));
                if idx < torrent_count {
                    let t = &guard.torrents[idx];
                    let selected = idx == tui.selected;
                    let pct = if t.total_bytes > 0 {
                        (t.completed_bytes as f64 / t.total_bytes as f64) * 100.0
                    } else {
                        0.0
                    };
                    let bar_width = 10.min(cols / 5);
                    let bar = tui_progress_bar(pct, bar_width);
                    let icon = tui_status_icon(&t.status, t.paused);
                    let color = tui_status_color(&t.status);
                    let reset = "\x1b[0m";

                    let right = if t.status == "seeding" || t.status == "complete" {
                        format!("seeding \u{2191}{}", tui_format_rate(t.upload_rate_bps))
                    } else if t.status == "downloading" {
                        format!(
                            "{:.0}% \u{2193}{}",
                            pct,
                            tui_format_rate(t.download_rate_bps)
                        )
                    } else {
                        t.status.clone()
                    };

                    let name_max = cols.saturating_sub(bar_width + right.len() + 8);
                    let name: String = if t.name.len() > name_max {
                        t.name
                            .chars()
                            .take(name_max.saturating_sub(1))
                            .collect::<String>()
                            + "\u{2026}"
                    } else {
                        t.name.clone()
                    };
                    let name_pad = name_max.saturating_sub(name.len());

                    let sel_start = if selected { "\x1b[7m" } else { "" };
                    let sel_end = if selected { "\x1b[0m" } else { "" };

                    let line = format!(
                        "{sel_start} {icon} {color}{name}{reset}{sel_start}{:name_pad$} [{bar}] {right} {sel_end}",
                        "",
                        name_pad = name_pad,
                    );
                    frame.push_str(&line);
                    let visible_len = strip_ansi_len(&line);
                    if visible_len < cols {
                        for _ in 0..(cols - visible_len) {
                            frame.push(' ');
                        }
                    }
                    if selected {
                        frame.push_str("\x1b[0m");
                    }
                } else {
                    // Empty row
                    for _ in 0..cols {
                        frame.push(' ');
                    }
                }
            }

            // Detail panel
            if tui.show_detail && tui.selected < torrent_count {
                let t = &guard.torrents[tui.selected];
                let detail_start = header_rows + list_rows + 1;
                let ratio_val = if t.downloaded_bytes > 0 {
                    t.uploaded_bytes as f64 / t.downloaded_bytes as f64
                } else {
                    0.0
                };
                let eta = if t.eta_secs > 0 {
                    let h = t.eta_secs / 3600;
                    let m = (t.eta_secs % 3600) / 60;
                    let s = t.eta_secs % 60;
                    if h > 0 {
                        format!("{h}h{m:02}m{s:02}s")
                    } else {
                        format!("{m}m{s:02}s")
                    }
                } else {
                    "--:--".to_string()
                };

                let details = [
                    format!(" Name: {}", t.name),
                    format!(
                        " Size: {}  Down: {}  Up: {}",
                        tui_format_bytes(t.total_bytes),
                        tui_format_bytes(t.downloaded_bytes),
                        tui_format_bytes(t.uploaded_bytes)
                    ),
                    format!(
                        " Ratio: {:.2}  ETA: {}  Peers: {}/{}",
                        ratio_val, eta, t.active_peers, t.tracker_peers
                    ),
                    format!(" Hash: {}", t.info_hash),
                    format!(" Dir: {}", t.download_dir),
                    format!(" Files: {}", t.files.len()),
                ];

                for (i, detail) in details.iter().enumerate() {
                    if i >= detail_rows {
                        break;
                    }
                    frame.push_str(&format!("\x1b[{};1H\x1b[90m", detail_start + i));
                    let truncated: String = detail.chars().take(cols).collect();
                    frame.push_str(&truncated);
                    let pad = cols.saturating_sub(truncated.len());
                    for _ in 0..pad {
                        frame.push(' ');
                    }
                    frame.push_str("\x1b[0m");
                }
            }

            // Footer: keybinds
            frame.push_str(&format!("\x1b[{};1H", rows));
            let confirm_msg = if let Some(id) = tui.confirm_delete {
                format!(" Delete torrent {id}? [y/n] ")
            } else {
                String::new()
            };
            if !confirm_msg.is_empty() {
                frame.push_str("\x1b[33;1m");
                frame.push_str(&confirm_msg);
                let pad = cols.saturating_sub(confirm_msg.len());
                for _ in 0..pad {
                    frame.push(' ');
                }
                frame.push_str("\x1b[0m");
            } else {
                let footer =
                    " [q]uit [p]ause [r]echeck [s]top [d]elete [Enter]detail [\u{2191}\u{2193}]nav";
                frame.push_str("\x1b[7m");
                let truncated: String = footer.chars().take(cols).collect();
                frame.push_str(&truncated);
                let flen = strip_ansi_len(&truncated);
                for _ in 0..cols.saturating_sub(flen) {
                    frame.push(' ');
                }
                frame.push_str("\x1b[0m");
            }

            drop(guard);

            // Write frame
            {
                let mut out = stdout.lock();
                let _ = out.write_all(frame.as_bytes());
                let _ = out.flush();
            }

            thread::sleep(Duration::from_millis(100));
        }

        // Restore terminal
        {
            let mut out = stdout.lock();
            let _ = out.write_all(b"\x1b[?25h\x1b[?1049l");
            let _ = out.flush();
        }
        tui_restore_mode(&original);
    })
        .map_err(|err| format!("terminal UI worker could not start: {err}"))
}

fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0usize;
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() || ch == 'm' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            len += 1;
        }
    }
    len
}
