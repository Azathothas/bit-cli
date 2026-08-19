//! Shared machinery for the verbs that need a live torrent session.
//!
//! `download`, `seed`, `peers`, `trackers`, and `bench` all do the same four
//! things: resolve a source, start a session, attach HTTP sources to it, and
//! then watch until a stop condition fires. That is what lives here, so a
//! flag means one thing across every verb rather than one thing per command.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bit_cli_core::engine::{Engine, EngineOptions, Handle, PeerSnapshot, TorrentSnapshot};
use bit_cli_core::error::{Error, Result};
use bit_cli_core::layout::Layout;
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{format_rate, format_size, parse_duration, parse_rate};
use bit_cli_core::webseed::binding::{BindingSet, SourceSpec};
use bit_cli_core::webseed::bridge::{BridgeParams, BridgeState, BridgeStatus};
use bit_cli_core::webseed::fetch::{Fetcher, Verify};
use serde::Serialize;

use crate::cli::{Global, LimitArgs, TrackerArgs, WebSeedArgs};
use crate::env::Env;
use crate::output::Renderer;

/// A tokio runtime for the commands that do I/O.
///
/// One per invocation. Nothing here outlives the process, so the runtime is
/// built where it is used rather than held in a global.
pub fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::generic(format!("cannot start the async runtime: {e}")))
}

/// Parse a duration flag, naming the flag when it is wrong.
pub fn duration_flag(value: &str, flag: &str) -> Result<Duration> {
    parse_duration(value)
        .map_err(|e| Error::usage(format!("--{flag}: {e}")).with("value", value.to_string()))
}

/// Parse an optional duration flag.
pub fn optional_duration(value: &Option<String>, flag: &str) -> Result<Option<Duration>> {
    value.as_deref().map(|v| duration_flag(v, flag)).transpose()
}

/// Parse a rate flag, naming the flag when it is wrong.
pub fn rate_flag(value: &Option<String>, flag: &str) -> Result<Option<u64>> {
    let Some(text) = value else { return Ok(None) };
    parse_rate(text)
        .map(Some)
        .map_err(|e| Error::usage(format!("--{flag}: {e}")).with("value", text.clone()))
}

/// Parse a `--port` value, which is either `N` or `START-END`.
pub fn port_range(values: &[String]) -> Result<std::ops::RangeInclusive<u16>> {
    if values.is_empty() {
        return Ok(6881..=6889);
    }
    let mut low = u16::MAX;
    let mut high = u16::MIN;
    for value in values {
        let (start, end) = match value.split_once('-') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (value.trim(), value.trim()),
        };
        let parse = |text: &str| -> Result<u16> {
            text.parse::<u16>().map_err(|_| {
                Error::usage(format!(
                    "--port `{value}` is not a port or a START-END range"
                ))
                .with("value", value.clone())
            })
        };
        let (start, end) = (parse(start)?, parse(end)?);
        if start > end {
            return Err(Error::usage(format!("--port `{value}` runs backwards"))
                .with("value", value.clone()));
        }
        low = low.min(start);
        high = high.max(end);
    }
    Ok(low..=high)
}

/// Where payloads are written for this run.
pub fn download_directory(global: &Global, env: &Env) -> PathBuf {
    match &global.dir {
        Some(dir) => env.resolve(dir),
        None => env.cwd.clone(),
    }
}

/// Build the session options from the global flags and one command's limits.
pub struct SessionSetup<'a> {
    pub global: &'a Global,
    pub trackers: &'a TrackerArgs,
    pub limits: &'a LimitArgs,
    pub web_seeds: &'a WebSeedArgs,
    pub listen_ports: std::ops::RangeInclusive<u16>,
    pub no_dht: bool,
    pub no_lsd: bool,
}

impl SessionSetup<'_> {
    /// Turn the flags into engine options.
    ///
    /// `--web-seed-only` is the one flag that reaches across subsystems: it
    /// turns off peers, DHT, LSD, and trackers together, because "HTTP only"
    /// with a tracker still running is not HTTP only.
    pub fn engine_options(&self, env: &Env) -> Result<EngineOptions> {
        let http_only = self.web_seeds.web_seed_only;
        Ok(EngineOptions {
            download_directory: download_directory(self.global, env),
            listen_ports: self.listen_ports.clone(),
            // A real run has to be reachable from the swarm, so the listener
            // binds the wildcard address. `--web-seed-only` needs a port only
            // for the loopback bridge, so it stays on loopback: nothing
            // outside the machine has any reason to reach it, and a host
            // firewall has no reason to ask about it.
            listen_ip: http_only.then(|| std::net::Ipv4Addr::LOCALHOST.into()),
            enable_dht: !http_only && !self.no_dht,
            enable_lsd: !http_only && !self.no_lsd,
            enable_trackers: !http_only && !self.trackers.no_tracker,
            enable_peers: !http_only,
            max_peers: self.limits.max_peers,
            download_rate: rate_flag(&self.limits.max_download_rate, "max-download-rate")?,
            upload_rate: rate_flag(&self.limits.max_upload_rate, "max-upload-rate")?,
            extra_trackers: Vec::new(),
            ipv4_only: false,
            client_name: Some(format!("bit-cli {}", bit_cli_core::VERSION)),
        })
    }

    /// The tracker list for one torrent, after the runtime edits.
    ///
    /// `--tracker`, `--tracker-file`, and `--tracker-list-url` add;
    /// `--exclude-tracker` removes; `--replace-trackers` drops the torrent's
    /// own list first. The `.torrent` is never rewritten.
    pub fn tracker_list(&self, meta: Option<&Metainfo>, env: &Env) -> Result<Option<Vec<String>>> {
        let args = self.trackers;
        if args.no_tracker || self.web_seeds.web_seed_only {
            return Ok(Some(Vec::new()));
        }
        let mut out: Vec<String> = Vec::new();
        if !args.replace_trackers
            && let Some(meta) = meta
        {
            out.extend(meta.trackers());
        }
        out.extend(args.tracker.iter().cloned());
        for path in &args.tracker_file {
            let path = env.resolve(path);
            let text = std::fs::read_to_string(&path).map_err(|e| {
                bit_cli_core::error::from_io(e, format!("cannot read {}", path.display()))
            })?;
            out.extend(bit_cli_core::webseed::table::parse_url_list(&text));
        }

        let excluded: HashSet<&str> = args.exclude_tracker.iter().map(String::as_str).collect();
        if excluded.contains("*") {
            return Ok(Some(Vec::new()));
        }
        out.retain(|url| !excluded.contains(url.as_str()));

        // Keep declaration order but drop repeats, so a tracker listed in both
        // the torrent and a file is announced to once.
        let mut seen = HashSet::new();
        out.retain(|url| seen.insert(url.clone()));

        match out.is_empty() && args.tracker.is_empty() && args.tracker_file.is_empty() {
            true => Ok(None),
            false => Ok(Some(out)),
        }
    }
}

/// One HTTP source attached to a running torrent.
pub struct AttachedSource {
    pub index: usize,
    pub url: String,
    pub origin: &'static str,
    pub scope: String,
    /// Pieces this source can serve on its own.
    pub whole_pieces: usize,
    pub status: Arc<BridgeStatus>,
    task: tokio::task::JoinHandle<()>,
}

impl AttachedSource {
    /// Stop the bridge.
    pub fn stop(&self) {
        self.task.abort();
    }
}

/// What one source looks like in a report.
#[derive(Debug, Clone, Serialize)]
pub struct SourceReport {
    pub index: usize,
    pub url: String,
    pub origin: &'static str,
    pub scope: String,
    pub whole_pieces: usize,
    pub state: BridgeState,
    pub served_bytes: u64,
    pub served_human: String,
    pub blocks: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AttachedSource {
    /// A snapshot of this source for the report.
    pub fn report(&self) -> SourceReport {
        let served = self.status.served_bytes();
        SourceReport {
            index: self.index,
            url: self.url.clone(),
            origin: self.origin,
            scope: self.scope.clone(),
            whole_pieces: self.whole_pieces,
            state: self.status.state(),
            served_bytes: served,
            served_human: format_size(served),
            blocks: self.status.blocks(),
            error: self.status.error(),
        }
    }
}

/// Attach every declared HTTP source to a running torrent.
///
/// The torrent's metadata has to have resolved first, because the layout is
/// what scopes and URLs are computed against. Nothing here touches the
/// `.torrent`: the sources exist for the length of this invocation only.
pub struct AttachOptions {
    /// Fail the run when the sources leave a gap.
    pub require: bool,
    /// Whether peers can cover what the sources cannot.
    pub peers_available: bool,
    /// Windows each source caches.
    pub cache_windows: usize,
    /// Keep a request record for `--trace http`.
    pub trace: bool,
    /// When to hash-check what a source serves.
    pub verify: Verify,
}

pub async fn attach_sources(
    engine: &Engine,
    handle: &Handle,
    layout: &Arc<Layout>,
    specs: &[SourceSpec],
    options: &AttachOptions,
) -> Result<(Vec<AttachedSource>, BindingSet)> {
    let AttachOptions {
        require,
        peers_available,
        cache_windows,
        trace,
        verify,
    } = *options;
    let info_hash = handle.info_hash().as_string();
    let set = BindingSet::resolve(layout, &info_hash, specs)?;
    if require {
        set.require_coverage(peers_available)?;
    }
    if specs.is_empty() {
        return Ok((Vec::new(), set));
    }

    let target = engine.bridge_target().ok_or_else(|| {
        Error::network(
            "web seeds need an incoming peer port and none was bound, so no HTTP source can attach",
        )
    })?;
    let session_peer_id = handle.shared().peer_id;
    let piece_hashes = engine.piece_hashes(handle);

    let mut attached = Vec::with_capacity(set.bindings.len());
    for binding in &set.bindings {
        let params = BridgeParams::for_binding(
            target,
            handle.info_hash(),
            session_peer_id,
            layout,
            binding,
            binding.spec.limits.concurrency.max(1),
        );
        let whole_pieces = params.pieces.len();
        let fetcher = Arc::new(
            Fetcher::new(
                binding.clone(),
                layout.clone(),
                info_hash.clone(),
                cache_windows,
                trace,
            )?
            .with_verification(verify, piece_hashes.clone()),
        );
        let status = Arc::new(BridgeStatus::default());
        let task = tokio::spawn(bit_cli_core::webseed::bridge::run(
            params,
            fetcher,
            status.clone(),
        ));
        attached.push(AttachedSource {
            index: binding.index,
            url: binding.spec.url.clone(),
            origin: binding.spec.origin.as_str(),
            scope: binding.scope.selector.clone(),
            whole_pieces,
            status,
            task,
        });
    }
    Ok((attached, set))
}

/// Loopback ports the attached bridges are connected from.
///
/// This is what tells a bridge apart from a real peer in the peer list, so
/// an HTTP source is never counted as a swarm member.
pub fn bridge_ports(sources: &[AttachedSource]) -> HashSet<u16> {
    sources
        .iter()
        .filter_map(|s| s.status.local_port())
        .collect()
}

/// When to stop watching.
#[derive(Debug, Clone, Default)]
pub struct StopConditions {
    /// Overall deadline for the whole operation.
    pub timeout: Option<Duration>,
    /// Stop after this long regardless of state.
    pub stop_after: Option<Duration>,
    /// Give up when nothing has progressed for this long.
    pub stall: Option<Duration>,
    /// Abort when the rate falls below this, in bytes per second.
    pub lowest_rate: Option<u64>,
    /// Stop seeding at this ratio. Zero means do not seed at all.
    pub seed_ratio: Option<f64>,
    /// Stop seeding after this long.
    pub seed_time: Option<Duration>,
    /// Exit after this long with no connected peers.
    pub exit_when_idle: Option<Duration>,
}

/// Why the watch loop stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stopped {
    /// Every wanted piece is present and verified.
    Completed,
    /// `--seed-ratio` was reached.
    SeedRatio,
    /// `--seed-time` elapsed.
    SeedTime,
    /// `--exit-when-idle` elapsed with no peers.
    Idle,
    /// `--timeout` or `--stop-after` elapsed.
    Deadline,
    /// `--stop-timeout` elapsed with no progress.
    Stalled,
    /// The rate fell below `--lowest-speed-limit`.
    TooSlow,
    /// The user interrupted the run.
    Interrupted,
    /// The torrent failed.
    Failed,
}

impl Stopped {
    /// The stable name used in JSON and text output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::SeedRatio => "seed_ratio",
            Self::SeedTime => "seed_time",
            Self::Idle => "idle",
            Self::Deadline => "deadline",
            Self::Stalled => "stalled",
            Self::TooSlow => "too_slow",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }

    /// The exit code this reason produces.
    pub const fn code(self) -> bit_cli_core::ExitCode {
        use bit_cli_core::ExitCode as E;
        match self {
            Self::Completed | Self::SeedRatio | Self::SeedTime | Self::Idle => E::Success,
            Self::Deadline | Self::Stalled => E::Timeout,
            Self::TooSlow => E::ThresholdNotMet,
            Self::Interrupted => E::Interrupted,
            Self::Failed => E::Generic,
        }
    }
}

/// A change the watch loop noticed between two ticks.
///
/// Piece and file completions are derived by comparing consecutive snapshots
/// rather than pushed by the engine, so the report interval bounds how
/// precisely they are timed. The counts are exact; the timestamps are as
/// precise as the interval.
#[derive(Debug, Default)]
pub struct Tick {
    pub verified_pieces: Vec<u32>,
    pub completed_files: Vec<usize>,
}

/// Tracks what has already been reported, so each event fires once.
pub struct Progress {
    have: Vec<bool>,
    files_done: Vec<bool>,
    file_lengths: Vec<u64>,
    best_progress: u64,
    last_progress_at: Instant,
    started: Instant,
    complete_since: Option<Instant>,
    idle_since: Option<Instant>,
}

impl Progress {
    /// Start tracking a torrent with the given file lengths.
    pub fn new(piece_count: u32, file_lengths: Vec<u64>) -> Self {
        let files = file_lengths.len();
        Self {
            have: vec![false; piece_count as usize],
            files_done: vec![false; files],
            file_lengths,
            best_progress: 0,
            last_progress_at: Instant::now(),
            started: Instant::now(),
            complete_since: None,
            idle_since: None,
        }
    }

    /// How long the run has been going.
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Fold one observation in, returning what changed.
    pub fn observe(
        &mut self,
        snapshot: &TorrentSnapshot,
        have: Option<&[bool]>,
        file_progress: &[u64],
    ) -> Tick {
        let mut tick = Tick::default();

        if let Some(have) = have {
            for (index, present) in have.iter().enumerate() {
                if *present && !self.have.get(index).copied().unwrap_or(true) {
                    self.have[index] = true;
                    tick.verified_pieces.push(index as u32);
                }
            }
        }

        for (index, done) in file_progress.iter().enumerate() {
            let length = self.file_lengths.get(index).copied().unwrap_or(0);
            let already = self.files_done.get(index).copied().unwrap_or(true);
            if !already && length > 0 && *done >= length {
                self.files_done[index] = true;
                tick.completed_files.push(index);
            }
        }

        if snapshot.progress_bytes > self.best_progress {
            self.best_progress = snapshot.progress_bytes;
            self.last_progress_at = Instant::now();
        }
        if snapshot.finished && self.complete_since.is_none() {
            self.complete_since = Some(Instant::now());
        }
        match snapshot.peers.live {
            0 => self.idle_since.get_or_insert_with(Instant::now),
            _ => {
                self.idle_since = None;
                &mut self.started
            }
        };

        tick
    }

    /// Whether a stop condition has fired.
    pub fn should_stop(
        &self,
        snapshot: &TorrentSnapshot,
        stop: &StopConditions,
        seeding: bool,
    ) -> Option<Stopped> {
        if snapshot.state == bit_cli_core::engine::State::Error {
            return Some(Stopped::Failed);
        }
        for limit in [stop.timeout, stop.stop_after].into_iter().flatten() {
            if self.started.elapsed() >= limit {
                return Some(Stopped::Deadline);
            }
        }
        if let Some(stall) = stop.stall
            && !snapshot.finished
            && self.last_progress_at.elapsed() >= stall
        {
            return Some(Stopped::Stalled);
        }
        if let Some(floor) = stop.lowest_rate
            && !snapshot.finished
            && self.started.elapsed() >= Duration::from_secs(10)
            && snapshot.download_rate < floor
        {
            return Some(Stopped::TooSlow);
        }
        if let Some(idle) = stop.exit_when_idle
            && let Some(since) = self.idle_since
            && since.elapsed() >= idle
        {
            return Some(Stopped::Idle);
        }
        if !seeding {
            return snapshot.finished.then_some(Stopped::Completed);
        }
        // Seeding: completion is not a stop condition, the seeding limits are.
        if let Some(ratio) = stop.seed_ratio
            && snapshot.ratio() >= ratio
        {
            return Some(Stopped::SeedRatio);
        }
        if let Some(limit) = stop.seed_time
            && let Some(since) = self.complete_since
            && since.elapsed() >= limit
        {
            return Some(Stopped::SeedTime);
        }
        None
    }
}

/// One line of plain progress, written to stderr.
///
/// Progress is never data, so it never reaches stdout. This is the same
/// information the `progress` event carries, which is the parity requirement:
/// nothing a person can read here is missing from the machine surface.
pub fn progress_line(snapshot: &TorrentSnapshot, sources: &[AttachedSource]) -> String {
    let mut line = format!(
        "{:>6.2}%  {} / {}  down {}  up {}  peers {}",
        snapshot.fraction() * 100.0,
        format_size(snapshot.progress_bytes),
        format_size(snapshot.total_bytes),
        format_rate(snapshot.download_rate),
        format_rate(snapshot.upload_rate),
        snapshot.peers.live,
    );
    let served: u64 = sources.iter().map(|s| s.status.served_bytes()).sum();
    if !sources.is_empty() {
        line.push_str(&format!("  http {}", format_size(served)));
    }
    if let Some(eta) = snapshot.eta_ms {
        line.push_str(&format!(
            "  eta {}",
            bit_cli_core::units::format_duration_ms(eta)
        ));
    }
    line
}

/// Peers rendered as an aligned table.
pub fn peer_table(peers: &[PeerSnapshot]) -> Vec<String> {
    let rows: Vec<Vec<String>> = peers
        .iter()
        .map(|p| {
            vec![
                p.addr.clone(),
                p.state.clone(),
                p.direction.to_string(),
                p.connection.clone().unwrap_or_else(|| "-".into()),
                p.client.clone().unwrap_or_else(|| "-".into()),
                format_size(p.downloaded_bytes),
                format_size(p.uploaded_bytes),
                p.verified_pieces.to_string(),
                match p.web_seed {
                    true => "web seed".to_string(),
                    false => "-".to_string(),
                },
            ]
        })
        .collect();
    crate::output::table(
        &[
            "ADDRESS", "STATE", "DIR", "CONN", "CLIENT", "DOWN", "UP", "PIECES", "KIND",
        ],
        &rows,
    )
}

/// Run a hook, passing everything through the environment.
///
/// Nothing torrent-supplied is ever interpolated into a command line. The
/// command is run as written and the facts arrive as `BIT_CLI_*` variables, so
/// a file named `; rm -rf /` is a file name and not a command.
pub fn run_hook(command: &str, vars: &BTreeMap<String, String>) -> Result<i32> {
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(command);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.arg("-c").arg(command);
        c
    };
    for (key, value) in vars {
        cmd.env(key, value);
    }
    let status = cmd.status().map_err(|e| {
        bit_cli_core::error::from_io(e, format!("hook `{command}` could not be run"))
    })?;
    Ok(status.code().unwrap_or(-1))
}

/// The environment a hook receives.
pub fn hook_vars(
    snapshot: &TorrentSnapshot,
    directory: &std::path::Path,
) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    vars.insert("BIT_CLI_VERSION".into(), bit_cli_core::VERSION.to_string());
    vars.insert("BIT_CLI_INFO_HASH".into(), snapshot.info_hash.clone());
    vars.insert("BIT_CLI_NAME".into(), snapshot.name.clone());
    vars.insert("BIT_CLI_DIR".into(), directory.display().to_string());
    vars.insert(
        "BIT_CLI_TOTAL_BYTES".into(),
        snapshot.total_bytes.to_string(),
    );
    vars.insert(
        "BIT_CLI_PROGRESS_BYTES".into(),
        snapshot.progress_bytes.to_string(),
    );
    vars.insert(
        "BIT_CLI_UPLOADED_BYTES".into(),
        snapshot.uploaded_bytes.to_string(),
    );
    vars.insert("BIT_CLI_STATE".into(), snapshot.state.as_str().to_string());
    vars.insert("BIT_CLI_FINISHED".into(), snapshot.finished.to_string());
    vars
}

/// Report a warning about a source that failed, once per source.
pub fn report_failed_sources(
    sources: &[AttachedSource],
    reported: &mut HashSet<usize>,
    renderer: &Renderer,
    env: &mut Env,
) -> Vec<SourceReport> {
    let mut newly_failed = Vec::new();
    for source in sources {
        if source.status.state() != BridgeState::Failed || !reported.insert(source.index) {
            continue;
        }
        let report = source.report();
        renderer.warn(
            env,
            format!(
                "web seed {} is unusable: {}",
                report.url,
                report.error.as_deref().unwrap_or("no reason given")
            ),
        );
        newly_failed.push(report);
    }
    newly_failed
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_cli_core::engine::{PeerCounts, State};

    fn snapshot(progress: u64, total: u64) -> TorrentSnapshot {
        TorrentSnapshot {
            id: 0,
            info_hash: "0".repeat(40),
            name: "t".into(),
            state: State::Live,
            total_bytes: total,
            progress_bytes: progress,
            uploaded_bytes: 0,
            finished: progress >= total && total > 0,
            download_rate: 0,
            upload_rate: 0,
            eta_ms: None,
            eta_confidence: "none",
            peers: PeerCounts::default(),
            error: None,
        }
    }

    #[test]
    fn a_bare_port_and_a_range_both_parse() {
        assert_eq!(port_range(&[]).unwrap(), 6881..=6889);
        assert_eq!(port_range(&["51413".into()]).unwrap(), 51413..=51413);
        assert_eq!(port_range(&["6881-6891".into()]).unwrap(), 6881..=6891);
    }

    #[test]
    fn several_port_flags_widen_the_range() {
        let range = port_range(&["7000".into(), "6881-6889".into()]).unwrap();
        assert_eq!(range, 6881..=7000);
    }

    #[test]
    fn a_backwards_or_malformed_port_range_is_a_usage_error() {
        for bad in ["6891-6881", "notaport", "6881-", "-6889"] {
            let err = port_range(&[bad.to_string()]).unwrap_err();
            assert_eq!(err.code(), bit_cli_core::ExitCode::Usage, "{bad}");
        }
    }

    #[test]
    fn a_piece_is_only_reported_verified_once() {
        let mut progress = Progress::new(4, vec![1024]);
        let snap = snapshot(2048, 4096);

        let first = progress.observe(&snap, Some(&[true, true, false, false]), &[]);
        assert_eq!(first.verified_pieces, vec![0, 1]);

        let second = progress.observe(&snap, Some(&[true, true, true, false]), &[]);
        assert_eq!(
            second.verified_pieces,
            vec![2],
            "already-known pieces do not repeat"
        );
    }

    #[test]
    fn a_file_completes_once_and_a_zero_length_file_never_does() {
        let mut progress = Progress::new(4, vec![100, 0, 50]);
        let snap = snapshot(150, 150);

        let first = progress.observe(&snap, None, &[100, 0, 20]);
        assert_eq!(first.completed_files, vec![0]);

        let second = progress.observe(&snap, None, &[100, 0, 50]);
        assert_eq!(
            second.completed_files,
            vec![2],
            "a zero-length file never completes"
        );

        let third = progress.observe(&snap, None, &[100, 0, 50]);
        assert!(third.completed_files.is_empty());
    }

    #[test]
    fn completion_stops_a_download_but_not_a_seed() {
        let mut progress = Progress::new(1, vec![10]);
        let done = snapshot(10, 10);
        progress.observe(&done, None, &[10]);

        let stop = StopConditions::default();
        assert_eq!(
            progress.should_stop(&done, &stop, false),
            Some(Stopped::Completed)
        );
        assert_eq!(
            progress.should_stop(&done, &stop, true),
            None,
            "a seed keeps going"
        );
    }

    #[test]
    fn a_failed_torrent_stops_before_any_other_condition() {
        let progress = Progress::new(1, vec![10]);
        let mut snap = snapshot(0, 10);
        snap.state = State::Error;
        let stop = StopConditions {
            timeout: Some(Duration::ZERO),
            ..Default::default()
        };
        assert_eq!(
            progress.should_stop(&snap, &stop, false),
            Some(Stopped::Failed)
        );
    }

    #[test]
    fn a_deadline_that_has_already_passed_stops_immediately() {
        let progress = Progress::new(1, vec![10]);
        let snap = snapshot(0, 10);
        let stop = StopConditions {
            timeout: Some(Duration::ZERO),
            ..Default::default()
        };
        assert_eq!(
            progress.should_stop(&snap, &stop, false),
            Some(Stopped::Deadline)
        );
    }

    #[test]
    fn a_seed_ratio_stops_a_seed() {
        let mut progress = Progress::new(1, vec![10]);
        let mut snap = snapshot(10, 10);
        snap.uploaded_bytes = 25;
        progress.observe(&snap, None, &[10]);

        let stop = StopConditions {
            seed_ratio: Some(2.0),
            ..Default::default()
        };
        assert_eq!(
            progress.should_stop(&snap, &stop, true),
            Some(Stopped::SeedRatio)
        );

        let stop = StopConditions {
            seed_ratio: Some(3.0),
            ..Default::default()
        };
        assert_eq!(progress.should_stop(&snap, &stop, true), None);
    }

    #[test]
    fn stop_reasons_map_to_distinct_exit_codes() {
        use bit_cli_core::ExitCode as E;
        assert_eq!(Stopped::Completed.code(), E::Success);
        assert_eq!(Stopped::Deadline.code(), E::Timeout);
        assert_eq!(Stopped::Stalled.code(), E::Timeout);
        assert_eq!(Stopped::TooSlow.code(), E::ThresholdNotMet);
        assert_eq!(Stopped::Interrupted.code(), E::Interrupted);
        // A finished seed exits zero however it was told to stop, because
        // being told to stop is not a failure.
        for reason in [Stopped::SeedRatio, Stopped::SeedTime, Stopped::Idle] {
            assert_eq!(reason.code(), E::Success, "{reason:?}");
        }
    }

    #[test]
    fn every_stop_reason_has_a_stable_name() {
        for reason in [
            Stopped::Completed,
            Stopped::SeedRatio,
            Stopped::SeedTime,
            Stopped::Idle,
            Stopped::Deadline,
            Stopped::Stalled,
            Stopped::TooSlow,
            Stopped::Interrupted,
            Stopped::Failed,
        ] {
            let name = reason.as_str();
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{name}"
            );
        }
    }

    #[test]
    fn a_progress_line_carries_every_number_the_event_does() {
        let mut snap = snapshot(512, 2048);
        snap.download_rate = 1024 * 1024;
        snap.peers.live = 7;
        let line = progress_line(&snap, &[]);
        assert!(line.contains("25.00%"), "{line}");
        assert!(line.contains("512 B"), "{line}");
        assert!(
            line.contains("2.00 MiB/s") || line.contains("1.00 MiB/s"),
            "{line}"
        );
        assert!(line.contains("peers 7"), "{line}");
    }

    #[test]
    fn hook_variables_never_carry_a_shell_string() {
        let snap = snapshot(1, 2);
        let vars = hook_vars(&snap, std::path::Path::new("/data"));
        assert_eq!(vars["BIT_CLI_INFO_HASH"], snap.info_hash);
        assert_eq!(vars["BIT_CLI_TOTAL_BYTES"], "2");
        assert_eq!(vars["BIT_CLI_DIR"], "/data");
        for key in vars.keys() {
            assert!(key.starts_with("BIT_CLI_"), "{key}");
        }
    }
}
