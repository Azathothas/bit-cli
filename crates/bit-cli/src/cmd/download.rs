//! `bit-cli download`: fetch to completion in the foreground, then exit.
//!
//! One invocation, one session, no daemon. Sources are fetched with peers and
//! with HTTP sources at the same time, and the accounting keeps the two apart
//! so a caller can answer "where did these bytes come from".
//!
//! Progress reaches the caller three ways, all carrying the same numbers: a
//! line on stderr for a person, a `progress` event on stdout under `--jsonl`,
//! and the final document under `--json`. Nothing is display-only.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use bit_cli_core::ExitCode;
use bit_cli_core::engine::{AddOptions, Engine, TorrentSnapshot};
use bit_cli_core::error::{Error, Result};
use bit_cli_core::layout::Layout;
use bit_cli_core::metalink::{Agreement, Checksum, Metalink};
use bit_cli_core::paths::Rename;
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{Size, format_rate, format_size};
use bit_cli_core::webseed::binding::SourceSpec;
use bit_cli_core::webseed::fetch::Verify;
use bit_cli_core::webseed::ledger::LedgerStats;
use serde::Serialize;
use serde_json::json;
use tokio::sync::mpsc;

use crate::cli::{DownloadArgs, Global};
use crate::env::Env;
use crate::output::{Renderer, field};
use crate::source::{Kind, ResolvedMetalink};
use crate::swarm::{
    self, AttachedSource, Progress, SessionSetup, SourceReport, StopConditions, Stopped,
};
use crate::webseed_args;

/// What one finished torrent reports.
#[derive(Debug, Clone, Serialize)]
pub struct TorrentReport {
    pub source: String,
    pub info_hash: String,
    pub name: String,
    pub stopped: Stopped,
    pub finished: bool,
    pub total: Size,
    pub downloaded: Size,
    pub uploaded: Size,
    /// Bytes served by HTTP sources. The rest came from peers.
    pub from_web_seeds: Size,
    pub from_peers: Size,
    /// Bytes that were already on the disk when the torrent was added, found
    /// by the hash check. Charged to neither transport, because this run did
    /// not fetch them. See `TODO/multi-source.md`, T-139.
    pub from_resume: Size,
    pub elapsed_ms: u64,
    pub elapsed_human: String,
    pub mean_rate: Size,
    pub mean_rate_human: String,
    pub peers_seen: u32,
    /// Every time `--redial-after` fired, and what the run had been waiting
    /// for when it did. Empty when the flag is off or the run never stalled.
    /// See `TODO/peers.md`, T-138.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub redials: Vec<Redial>,
    pub sources: Vec<SourceReport>,
    pub output_directory: String,
    /// Files whose on-disk path is not the path in the torrent, and why.
    /// Empty for the ordinary torrent. See `bit_cli_core::paths`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub renamed: Vec<Rename>,
    /// Files read from another torrent in the same run rather than fetched.
    /// Empty unless two torrents in one invocation hold the same file.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shared: Vec<SharedFile>,
    /// Announces this run sent itself: `completed` when the download
    /// finished and `stopped` when it ended. Empty when the torrent has no
    /// trackers or `--no-tracker` was given.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub announced: Vec<SentAnnounce>,
    /// What the Metalink said and whether the payload agreed with it. Present
    /// only when the source was a Metalink. See `TODO/cli-surface.md`, T-113.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metalink: Option<MetalinkReport>,
    /// What the block-to-source ledger did, for a run with HTTP sources.
    ///
    /// `evicted` is the one number worth reading on a healthy run: it counts
    /// pieces whose records were dropped before they could be resolved, so it
    /// is how many pieces could no longer have been attributed if they had
    /// turned out wrong. Absent when the run attached no sources, because a
    /// ledger with nothing to record says nothing.
    /// See `TODO/webseed.md`, T-179.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<LedgerStats>,
    /// The exit code this torrent's outcome produces.
    ///
    /// A run's code is the worst of its torrents'. Without this, a torrent
    /// that failed because a file was already there and one that failed
    /// because the tracker was unreachable would both arrive as a generic
    /// failure, which is exactly the distinction the exit code table exists to
    /// make. See `TODO/disk-io.md`, T-014.
    pub code: ExitCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What a Metalink said, and whether the bytes on disk agreed with it.
///
/// A Metalink and a `.torrent` are two independent descriptions of the same
/// payload, and this run has both. Two checks come out of that, and they fail
/// for different reasons.
///
/// The first costs nothing and runs before a byte is fetched: the two
/// documents each declare a length, and lengths that differ mean they describe
/// different files. The second runs on the payload the session has already
/// verified piece by piece against the torrent's own SHA-1 hashes, so a
/// checksum that then disagrees says the Metalink is the document that is
/// wrong, not the torrent. Saying which one is wrong is the whole point of
/// carrying both. See `TODO/cli-surface.md`, T-113.
#[derive(Debug, Clone, Serialize)]
pub struct MetalinkReport {
    /// `4` for RFC 5854 `.meta4`, `3` for the older `.metalink`.
    pub version: &'static str,
    /// The `<file name>` the document carried.
    pub file: String,
    /// The `<metaurl>` the `.torrent` was fetched from.
    pub torrent_url: String,
    /// Torrent URLs tried before that one, and what went wrong with each.
    /// Empty when the document's first choice answered.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub torrent_fallbacks: Vec<MirrorError>,
    /// Mirrors the document listed for the payload.
    pub mirrors_listed: usize,
    /// Mirrors that became sources in this run. Lower than `mirrors_listed`
    /// when `--no-torrent-web-seed` or `--no-web-seed` dropped them, or when
    /// the document's file could not be attributed to one file of a multi-file
    /// torrent, in which case `agreement.matched_by` says why.
    pub mirrors_registered: usize,
    /// Mirrors the document listed under a scheme this cannot fetch, `ftp:`
    /// being the one that occurs. Counted so the report can say the document
    /// had more in it than the run could use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mirrors_unsupported: Vec<String>,
    /// What the two documents say about the file's length, compared before
    /// anything was fetched.
    pub agreement: MetalinkAgreement,
    /// The checksum the document supplied, when it supplied one this can
    /// compute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<MetalinkChecksum>,
}

/// One torrent URL that did not answer.
#[derive(Debug, Clone, Serialize)]
pub struct MirrorError {
    pub url: String,
    pub error: String,
}

/// What the Metalink and the `.torrent` each say about the same file.
#[derive(Debug, Clone, Serialize)]
pub struct MetalinkAgreement {
    /// The file index in the torrent this entry was attributed to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_index: Option<usize>,
    /// Which rule attributed it, or why none could: `only_file`, `path`,
    /// `prefixed_path`, `file_name`, `ambiguous`, `no_match`, `no_name`.
    pub matched_by: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metalink_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torrent_size: Option<u64>,
    /// `true` when both declare a length and the two are equal, `false` when
    /// they differ, absent when either is missing. Absent is neither
    /// agreement nor disagreement and must not be read as either.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_agrees: Option<bool>,
}

/// The Metalink's own checksum, and what checking it found.
#[derive(Debug, Clone, Serialize)]
pub struct MetalinkChecksum {
    pub algorithm: String,
    pub expected: String,
    /// What the payload hashes to. Absent when the check did not run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    /// `true` when the payload matched, `false` when it did not, absent when
    /// the check did not run. Absent is not a pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<bool>,
    /// Bytes read to compute it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_hashed: Option<u64>,
    /// The file that was hashed, as it sits on disk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Why the check did not run, when it did not. A checksum that was not
    /// computed is not a checksum that passed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_checked: Option<String>,
}

/// One announce this run sent itself, beyond the session's own.
///
/// The session announces `started` when a torrent goes live and then repeats
/// on the tracker's interval. It never says a download finished and never says
/// it stopped, so a tracker's seeder count is wrong and a dead address is
/// handed out until the record expires. `bit-cli` runs in the foreground and
/// knows both moments exactly. See `TODO/trackers.md`, T-062.
#[derive(Debug, Clone, Serialize)]
pub struct SentAnnounce {
    /// `completed` or `stopped`.
    pub event: &'static str,
    /// Trackers it was sent to.
    pub trackers: usize,
    /// Trackers that answered without a failure.
    pub accepted: usize,
    /// Milliseconds into the run.
    pub at_ms: u64,
}

/// One file this torrent read from another torrent in the same run.
///
/// The proof is in the metadata: `pieces_compared` whole pieces of this file
/// have the same SHA-1 in both torrents, so the bytes those pieces cover are
/// the same. Nothing here is asserted by the caller, and the source is checked
/// per piece on the way in like every other source. See
/// `TODO/multi-source.md`, T-140.
#[derive(Debug, Clone, Serialize)]
pub struct SharedFile {
    /// File index in this torrent.
    pub index: usize,
    /// This torrent's path for it.
    pub path: String,
    pub length: Size,
    /// The source argument of the torrent it was read from.
    pub from_source: String,
    pub from_info_hash: String,
    /// File index in that torrent.
    pub from_index: usize,
    /// Where it was read from on disk.
    pub from_path: String,
    /// Whole pieces whose hashes were compared, all of which agreed.
    pub pieces_compared: u32,
    pub bytes_proven: Size,
}

/// One forced re-dial: the peer state was thrown away and the peer list
/// dialled again, because nothing had arrived for `--redial-after`.
#[derive(Debug, Clone, Serialize)]
pub struct Redial {
    /// Which re-dial this was, counting from 1.
    pub attempt: u32,
    /// Milliseconds into the run.
    pub at_ms: u64,
    /// How long the byte count had been flat when it fired.
    pub stalled_ms: u64,
    /// Live peer connections thrown away, which is what this cost.
    pub peers_dropped: u32,
    /// The reason it did not happen, when it did not. `None` on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What the whole run reports.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadReport {
    pub torrents: Vec<TorrentReport>,
    pub total: Size,
    pub downloaded: Size,
    pub from_web_seeds: Size,
    pub from_peers: Size,
    /// Bytes that were already on the disk when the torrent was added, found
    /// by the hash check. Charged to neither transport, because this run did
    /// not fetch them. See `TODO/multi-source.md`, T-139.
    pub from_resume: Size,
    pub elapsed_ms: u64,
    pub elapsed_human: String,
    pub completed: usize,
    pub failed: usize,
    /// What this run cost: peak RSS, CPU time, and open handles.
    ///
    /// Measuring a download from outside means sampling a process that has
    /// already exited, which reports zero. The process is the only thing that
    /// can report its own high-water mark, so it does.
    pub process: bit_cli_core::sysinfo::Process,
}

/// A message from a worker to the one thread that owns the output streams.
enum Msg {
    Event(&'static str, serde_json::Value),
    Warn(String),
    Progress(String),
    Done(Box<TorrentReport>),
}

/// Run the command.
pub fn run(
    args: &DownloadArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let report_interval = swarm::duration_flag(&args.report_interval, "report-interval")?;
    let stop = StopConditions {
        timeout: swarm::optional_duration(&global.timeout, "timeout")?,
        stop_after: swarm::optional_duration(&global.stop_after, "stop-after")?,
        stall: swarm::optional_duration(&args.limits.stop_timeout, "stop-timeout")?,
        lowest_rate: swarm::rate_flag(&args.limits.lowest_speed_limit, "lowest-speed-limit")?,
        seed_ratio: args.limits.seed_ratio,
        seed_time: swarm::optional_duration(&args.limits.seed_time, "seed-time")?,
        exit_when_idle: None,
        max_handles: args.limits.max_handles,
    };

    let setup = SessionSetup {
        global,
        trackers: &args.trackers,
        limits: &args.limits,
        web_seeds: &args.web_seeds,
        listen_ports: swarm::port_range(&args.port)?,
        no_dht: args.no_dht,
        no_lsd: args.no_lsd,
        allocation: allocation_of(args.selection.file_allocation),
    };
    let engine_options = setup.engine_options(env)?;
    // Parsed here so a bad rate fails before the session starts, next to the
    // session caps that `engine_options` just read. See `TODO/cli-surface.md`,
    // T-181.
    let (torrent_download_rate, torrent_upload_rate) = setup.torrent_rates()?;
    let directory = engine_options.download_directory.clone();

    if global.dry_run {
        return dry_run(args, global, &setup, renderer, env, &directory);
    }

    // Every source is classified before the session starts, so a typo in the
    // fifth argument fails before the first byte is fetched.
    let kinds: Vec<Kind> = args
        .sources
        .iter()
        .map(|source| Kind::classify(source, env))
        .collect::<Result<_>>()?;

    // One runtime for the whole command. It is built here rather than beside
    // the session because a Metalink has to be resolved over HTTP before the
    // plans can be built, and resolving it needs somewhere to run.
    let runtime = swarm::runtime()?;
    let user_agent = args
        .web_seeds
        .web_seed_user_agent
        .clone()
        .unwrap_or_else(bit_cli_core::webseed::fetch::default_user_agent);
    let mut resolved: std::collections::HashMap<usize, ResolvedMetalink> =
        std::collections::HashMap::new();
    let metalinks: Vec<(usize, std::path::PathBuf)> = kinds
        .iter()
        .enumerate()
        .filter_map(|(index, kind)| match kind {
            Kind::Metalink(path) => Some((index, path.clone())),
            _ => None,
        })
        .collect();
    if !metalinks.is_empty() {
        runtime.block_on(async {
            for (index, path) in metalinks {
                let document = Metalink::read(&path)?;
                let one = crate::source::resolve_metalink(&document, &user_agent).await?;
                renderer.event(
                    env,
                    "metalink_resolved",
                    &json!({
                        "source": args.sources[index],
                        "version": one.version,
                        "file": one.file.name,
                        "torrent_url": one.torrent_url,
                        "info_hash": one.meta.info_hash().hex(),
                        "mirrors": one.file.mirrors.len(),
                        "unsupported_mirrors": one.file.unsupported_mirrors.len(),
                        "checksums": one.file.checksums.len(),
                    }),
                )?;
                resolved.insert(index, one);
            }
            Ok::<(), Error>(())
        })?;
    }

    let mut plans = Vec::with_capacity(args.sources.len());
    let mut known_hashes: HashSet<String> = HashSet::new();
    let mut metas: Vec<Option<Metainfo>> = Vec::with_capacity(args.sources.len());
    for (index, source) in args.sources.iter().enumerate() {
        let one = resolved.remove(&index);
        let meta = match (&kinds[index], &one) {
            (Kind::File(path), _) => Some(Metainfo::read(path)?),
            (Kind::Metalink(_), Some(one)) => Some(one.meta.clone()),
            _ => None,
        };
        if let Some(meta) = &meta {
            known_hashes.insert(meta.info_hash().hex().to_ascii_lowercase());
        }
        // `--web-seed-list-url` is fetched here, on the runtime this command
        // already built. Every caller used to pass `no_network`, including
        // this one, so the flag parsed, was read, and could only ever fail.
        // See `TODO/cli-surface.md`, T-183.
        let specs = webseed_args::collect(
            &args.web_seeds,
            meta.as_ref(),
            one.as_ref().map(|m| &m.file),
            env,
            crate::source::list_fetcher(&runtime, &user_agent),
        )?;
        // `--tracker-list-url` is fetched on the runtime this command already
        // built, rather than on one of its own. See `TODO/cli-surface.md`,
        // T-181.
        let trackers = setup.tracker_list(
            meta.as_ref(),
            env,
            crate::source::list_fetcher(&runtime, &user_agent),
        )?;
        let (torrent_bytes, metalink) = match one {
            None => (None, None),
            Some(one) => {
                let registered = specs
                    .iter()
                    .filter(|spec| spec.origin == bit_cli_core::webseed::binding::Origin::Metalink)
                    .count();
                // `one.meta` is the torrent this Metalink named, so the two
                // documents being compared are always both present here.
                let agreement = one.file.agreement(&one.meta.layout());
                if agreement.disagrees() {
                    renderer.warn(
                        env,
                        format!(
                            "{source}: the metalink says the file is {} bytes and the torrent says {}. One of the two is wrong; the payload is checked against both.",
                            agreement.metalink_size.unwrap_or_default(),
                            agreement.torrent_size.unwrap_or_default(),
                        ),
                    );
                }
                let best = one.file.best_checksum().cloned();
                let unusable_algorithm = match &best {
                    Some(_) => None,
                    None => one.file.checksums.first().map(|c| c.algorithm.clone()),
                };
                let plan = MetalinkPlan {
                    version: one.version,
                    file_name: one.file.name.clone(),
                    torrent_url: one.torrent_url.clone(),
                    torrent_fallbacks: one
                        .torrent_errors
                        .iter()
                        .map(|(url, error)| MirrorError {
                            url: url.clone(),
                            error: error.clone(),
                        })
                        .collect(),
                    mirrors_listed: one.file.mirrors.len(),
                    mirrors_registered: registered,
                    mirrors_unsupported: one.file.unsupported_mirrors.clone(),
                    agreement,
                    checksum: best,
                    unusable_algorithm,
                };
                (Some(one.torrent_bytes), Some(Box::new(plan)))
            }
        };
        plans.push(Plan {
            index,
            source: source.clone(),
            torrent_bytes,
            metalink,
            specs,
            trackers,
            donations: Vec::new(),
        });
        metas.push(meta);
    }

    // Two torrents in one run that hold the same file, proven by their piece
    // hashes, are one fetch and one copy rather than two fetches. The proof is
    // computed here, from metadata that is already read, and costs one pass
    // per pair of torrents. Which of them can actually donate is decided when
    // each starts, because it depends on the donor having finished. See
    // `TODO/multi-source.md`, T-140.
    if !args.no_share_files {
        for (plan, donations) in plans.iter_mut().zip(share_plan(&metas)) {
            plan.donations = donations;
        }
    }
    let donor_files: SharedDonors = Arc::new(std::sync::Mutex::new(
        std::collections::HashMap::with_capacity(plans.len()),
    ));
    for (index, meta) in metas.iter().enumerate() {
        // A magnet has no metadata yet, so it can neither donate nor receive.
        // Recording the source and hash of the ones that do keeps the report
        // able to name the donor without carrying the metainfo around.
        if let Some(meta) = meta {
            let layout = meta.layout();
            let mut map = donor_files.lock().expect("donor registry");
            map.insert(
                index,
                DonorFiles {
                    source: args.sources[index].clone(),
                    info_hash: meta.info_hash().hex(),
                    root: bit_cli_core::storage::payload_root(&directory, &layout),
                    // Filled in when the torrent finishes. An empty list is
                    // what says it has nothing to lend yet.
                    disk_paths: Vec::new(),
                },
            );
        }
    }

    // A binding for a torrent that is not in this invocation binds nothing,
    // and `collect` drops it per torrent without knowing that. A mistyped
    // forty character hash would otherwise be a run that quietly used no
    // source at all.
    for (binding, hash) in webseed_args::qualified_torrents(&args.web_seeds) {
        if !known_hashes.contains(&hash) {
            let known: Vec<String> = known_hashes.iter().cloned().collect();
            return Err(Error::usage(format!(
                "--web-seed-for `{binding}` names info hash {hash}, which is not one of the torrents in this run"
            ))
            .with("value", binding)
            .with("torrents", known.join(", ")));
        }
    }

    let init_timeout = swarm::duration_flag(&args.limits.init_timeout, "init-timeout")?;
    // The courtesy announces at the end of a run use the same timeouts
    // `bit-cli trackers` does, because they are the same client talking to the
    // same trackers. See `TODO/trackers.md`, T-062.
    let tracker_timeout =
        swarm::optional_duration(&args.trackers.tracker_timeout, "tracker-timeout")?
            .unwrap_or(Duration::from_secs(30));
    let tracker_connect_timeout = swarm::optional_duration(
        &args.trackers.tracker_connect_timeout,
        "tracker-connect-timeout",
    )?
    .unwrap_or(Duration::from_secs(10));
    let trace_http = global.trace.iter().any(|t| t == "http");
    // A source-level check is per piece or nothing. `file` asks for a coarser
    // grain than the fetcher works at, and the per-piece check subsumes it, so
    // it gets the stronger check and is told so rather than silently ignored.
    if args.web_seeds.web_seed_verify == crate::cli::VerifyWhen::File {
        renderer.warn(
            env,
            "--web-seed-verify file is served by the per-piece check, which is stricter",
        );
    }
    let verify = match args.web_seeds.web_seed_verify {
        crate::cli::VerifyWhen::None => Verify::None,
        crate::cli::VerifyWhen::Piece | crate::cli::VerifyWhen::File => Verify::Piece,
    };
    let peers = swarm::peer_addrs(&args.peers)?;
    let redial_after = swarm::optional_duration(&args.redial_after, "redial-after")?;
    if let (Some(redial), Some(stall)) = (redial_after, stop.stall)
        && redial >= stall
    {
        renderer.warn(
            env,
            format!(
                "--redial-after {} is not shorter than --stop-timeout {}, so the run gives up before it re-dials",
                bit_cli_core::units::format_duration(redial),
                bit_cli_core::units::format_duration(stall),
            ),
        );
    }
    let concurrency = args.max_concurrent_downloads.max(1);
    let started = std::time::Instant::now();

    let outcome = runtime.block_on(async {
        let engine = Arc::new(Engine::start(&engine_options).await?);
        for warning in engine.warnings() {
            renderer.warn(env, warning);
        }

        renderer.event(
            env,
            "session_start",
            &json!({
                "sources": args.sources.len(),
                "directory": directory.display().to_string(),
                "listen_addr": engine.listen_addr().map(|a| a.to_string()),
                "max_concurrent_downloads": concurrency,
            }),
        )?;

        let (tx, mut rx) = mpsc::channel::<Msg>(256);
        // A queue of plans taken in order by a fixed pool of workers, rather
        // than one task per plan queuing on a semaphore. Two reasons. The
        // order torrents start in is then the order they were given, which is
        // what makes `-j 1` a sequence a caller can depend on: a torrent whose
        // source is a file an earlier torrent writes needs the earlier one to
        // go first. And a hundred sources no longer spawn a hundred tasks that
        // do nothing but wait.
        let queue = Arc::new(tokio::sync::Mutex::new(
            plans.into_iter().collect::<std::collections::VecDeque<_>>(),
        ));
        let workers_wanted = concurrency.min(queue.lock().await.len().max(1));
        let mut workers = tokio::task::JoinSet::new();
        for _ in 0..workers_wanted {
            let engine = engine.clone();
            let tx = tx.clone();
            let queue = queue.clone();
            let options = Options {
                // Existing data is hash-checked on add, and the check is what
                // makes resuming safe. All four of these flags mean "look at
                // what is already on disk", so they all reach the session the
                // same way.
                overwrite: args.allow_overwrite
                    || !args.no_continue
                    || args.check_integrity
                    || args.hash_check_only,
                hash_check_only: args.hash_check_only,
                init_timeout,
                only_files: selection(&args.selection)?,
                report_interval,
                stop: stop.clone(),
                require: args.web_seeds.web_seed_require,
                web_seed_only: args.web_seeds.web_seed_only,
                max_total: args.web_seeds.web_seed_max_total,
                prefer: args.web_seeds.prefer_web_seed,
                verify,
                trace_http,
                directory: directory.clone(),
                peers: peers.clone(),
                in_order: wants_in_order(args.selection.piece_selector),
                redial_after,
                max_redials: args.max_redials,
                donors: donor_files.clone(),
                tracker_timeout,
                tracker_connect_timeout,
                torrent_download_rate,
                torrent_upload_rate,
            };
            workers.spawn(async move {
                loop {
                    // The lock is held only to take the next plan, never
                    // across the download, so one slow torrent does not hold
                    // the queue.
                    let Some(plan) = queue.lock().await.pop_front() else {
                        break;
                    };
                    let report = one(&engine, plan, options.clone(), &tx).await;
                    let _ = tx.send(Msg::Done(Box::new(report))).await;
                }
            });
        }
        drop(tx);

        let mut reports = Vec::new();
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Event(kind, value) => renderer.event(env, kind, &value)?,
                Msg::Warn(text) => renderer.warn(env, text),
                Msg::Progress(line) => {
                    if renderer.progress == crate::cli::ProgressMode::Plain {
                        let _ = env.note(line);
                    }
                }
                Msg::Done(report) => reports.push(*report),
            }
        }
        while workers.join_next().await.is_some() {}
        // Storage runs on the session's threads and the streams belong to this
        // one, so it collects what the caller should know and this is where it
        // is read: an allocation method that could not be used, and what ran
        // instead.
        for note in engine.storage_notes() {
            renderer.warn(env, note);
        }
        Arc::try_unwrap(engine).ok().map(Engine::stop);

        Ok::<_, Error>(reports)
    });

    let mut reports = outcome?;
    reports.sort_by(|a, b| a.source.cmp(&b.source));

    let elapsed = started.elapsed();
    let report = DownloadReport {
        total: Size(reports.iter().map(|r| r.total.0).sum()),
        downloaded: Size(reports.iter().map(|r| r.downloaded.0).sum()),
        from_web_seeds: Size(reports.iter().map(|r| r.from_web_seeds.0).sum()),
        from_peers: Size(reports.iter().map(|r| r.from_peers.0).sum()),
        from_resume: Size(reports.iter().map(|r| r.from_resume.0).sum()),
        completed: reports.iter().filter(|r| r.finished).count(),
        failed: reports.iter().filter(|r| !r.finished).count(),
        elapsed_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
        elapsed_human: bit_cli_core::units::format_duration(elapsed),
        torrents: reports,
        process: bit_cli_core::sysinfo::Process::sample(),
    };

    // The worst outcome decides the exit code, so a run with one failed
    // torrent never exits zero.
    let code = report
        .torrents
        .iter()
        .map(|r| r.code)
        .max_by_key(|c| c.code())
        .unwrap_or(ExitCode::Success);

    run_hooks(&report, args, renderer, env);
    renderer.emit(env, "download", &report, || lines(&report))?;
    Ok(code)
}

/// The CLI's allocation names, as the core knows them.
///
/// Two enums for one concept because the core does not depend on `clap` and
/// the CLI does not define storage behaviour. The mapping is total, so a new
/// method cannot be added on one side without the other failing to compile.
pub(crate) fn allocation_of(method: crate::cli::FileAllocation) -> bit_cli_core::alloc::Allocation {
    use bit_cli_core::alloc::Allocation;
    match method {
        crate::cli::FileAllocation::None => Allocation::None,
        crate::cli::FileAllocation::Sparse => Allocation::Sparse,
        crate::cli::FileAllocation::Prealloc => Allocation::Prealloc,
        crate::cli::FileAllocation::Falloc => Allocation::Falloc,
    }
}

/// Everything one worker needs beyond the plan.
#[derive(Clone)]
struct Options {
    overwrite: bool,
    hash_check_only: bool,
    /// How long the hash check gets before the run gives up on it.
    init_timeout: Duration,
    only_files: Option<Vec<usize>>,
    report_interval: Duration,
    stop: StopConditions,
    require: bool,
    web_seed_only: bool,
    max_total: Option<usize>,
    prefer: bool,
    verify: Verify,
    trace_http: bool,
    directory: std::path::PathBuf,
    /// How long with no progress before every peer connection is dropped and
    /// the peer list is dialled again. `None` never re-dials.
    redial_after: Option<Duration>,
    /// How many times that may happen in one run.
    max_redials: u32,
    /// Peers to dial before any are discovered, from `--peer`.
    peers: Vec<std::net::SocketAddr>,
    /// Whether to hold the session's piece priority at the front of what is
    /// missing. See `TODO/performance.md`, T-032.
    in_order: bool,
    /// Where each torrent in the run wrote its files, filled in as they
    /// finish. See `TODO/multi-source.md`, T-140.
    donors: SharedDonors,
    /// How long a courtesy announce at the end of a run waits. See
    /// `TODO/trackers.md`, T-062.
    tracker_timeout: Duration,
    tracker_connect_timeout: Duration,
    /// The per-torrent rate caps, from `--max-download-rate` and
    /// `--max-upload-rate`. The whole-run pair is on the session instead. See
    /// `TODO/cli-surface.md`, T-181.
    torrent_download_rate: Option<u64>,
    torrent_upload_rate: Option<u64>,
}

/// Whether a selector asks for pieces front to back.
///
/// `sequential` and `in-order` are the same behaviour under two names, one
/// common and one `aria2`'s, and this is the single place that says so. See
/// `TODO/performance.md`, T-032.
const fn wants_in_order(selector: crate::cli::PieceSelector) -> bool {
    matches!(
        selector,
        crate::cli::PieceSelector::Sequential | crate::cli::PieceSelector::InOrder
    )
}

/// One source and what was resolved for it before the session started.
struct Plan {
    /// Position in the run, which is the order the queue hands plans out in.
    index: usize,
    source: String,
    /// The exact `.torrent` bytes, when this run resolved them itself rather
    /// than leaving it to the session. A Metalink names its torrent by URL and
    /// the URL was already fetched; handing the session the same URL would
    /// fetch it twice. See `TODO/cli-surface.md`, T-113.
    torrent_bytes: Option<Vec<u8>>,
    /// What the Metalink said, when the source was one.
    metalink: Option<Box<MetalinkPlan>>,
    specs: Vec<SourceSpec>,
    trackers: Option<Vec<String>>,
    /// Files an earlier torrent in this run is proven to hold, computed from
    /// the metadata before anything starts. See `TODO/multi-source.md`, T-140.
    donations: Vec<Donation>,
}

/// Everything the Metalink said, carried to the end of the run so the
/// checksum can be checked against the payload it describes.
struct MetalinkPlan {
    version: &'static str,
    file_name: String,
    torrent_url: String,
    torrent_fallbacks: Vec<MirrorError>,
    mirrors_listed: usize,
    mirrors_registered: usize,
    mirrors_unsupported: Vec<String>,
    agreement: Agreement,
    /// The strongest checksum in the document that this can compute. `None`
    /// when the document had none, or only ones nothing here hashes.
    checksum: Option<Checksum>,
    /// The strongest checksum in the document, computable or not. Used only to
    /// say why nothing was checked.
    unusable_algorithm: Option<String>,
}

/// One file this torrent could read from an earlier torrent in the run.
///
/// Only the donor's position is fixed here. Whether it can actually be read
/// depends on that torrent having finished, which is known when this one
/// starts and not before.
#[derive(Debug, Clone)]
struct Donation {
    /// File index in the torrent that would read it.
    index: usize,
    /// Position of the donor in the run. Always lower than the receiver's:
    /// a torrent can only read what an earlier one has already written.
    donor: usize,
    /// File index in the donor.
    donor_index: usize,
    length: u64,
    pieces_compared: u32,
    bytes_proven: u64,
}

/// What a finished torrent can lend to the ones after it.
#[derive(Debug, Clone)]
struct DonorFiles {
    source: String,
    info_hash: String,
    /// Directory its payload landed in, subfolder included.
    root: std::path::PathBuf,
    /// One path per file index, relative to `root`, as planned. Empty until
    /// the torrent finishes, which is what says it has nothing to lend yet:
    /// a `file:` source over a half-written file serves bytes that are not
    /// there.
    disk_paths: Vec<String>,
}

/// Every torrent in the run that could donate, keyed by position.
type SharedDonors = Arc<std::sync::Mutex<std::collections::HashMap<usize, DonorFiles>>>;

/// Every file each torrent could take from an earlier one, proven from the
/// metadata.
///
/// Proof only. [`bit_cli_core::equivalence::Evidence::Length`] says two files
/// are the same size and nothing else, and reading a file on that basis is
/// exactly the silent corruption the equivalence module exists to avoid. A
/// piece-hash proof means the whole pieces inside both files have the same
/// SHA-1, which is the same evidence a torrent gives about its own bytes.
///
/// The earliest torrent that holds the file donates it. That is the one most
/// likely to have finished by the time a later one starts, and it makes the
/// choice a function of the command line rather than of the order things
/// happened to complete.
fn share_plan(metas: &[Option<Metainfo>]) -> Vec<Vec<Donation>> {
    let mut out: Vec<Vec<Donation>> = vec![Vec::new(); metas.len()];
    for (index, meta) in metas.iter().enumerate() {
        let Some(meta) = meta else { continue };
        let layout = meta.layout();
        let mut taken: HashSet<usize> = HashSet::new();
        for (donor, other) in metas[..index].iter().enumerate() {
            let Some(other) = other.as_ref() else {
                continue;
            };
            let other_layout = other.layout();
            for found in bit_cli_core::equivalence::matches(
                &layout,
                &meta.info().pieces,
                &other_layout,
                &other.info().pieces,
            ) {
                if !found.evidence.is_proof() || !taken.insert(found.index) {
                    continue;
                }
                out[index].push(Donation {
                    index: found.index,
                    donor,
                    donor_index: found.other_index,
                    length: found.length,
                    pieces_compared: found.pieces_compared,
                    bytes_proven: found.bytes_proven,
                });
            }
        }
    }
    out
}

/// Fetch one source to completion.
async fn one(
    engine: &Engine,
    plan: Plan,
    options: Options,
    tx: &mpsc::Sender<Msg>,
) -> TorrentReport {
    match one_inner(engine, &plan, &options, tx).await {
        Ok(report) => report,
        Err(error) => {
            let _ = tx
                .send(Msg::Event(
                    "error",
                    serde_json::to_value(error.report()).unwrap_or_default(),
                ))
                .await;
            TorrentReport {
                source: plan.source,
                info_hash: String::new(),
                name: String::new(),
                stopped: Stopped::Failed,
                finished: false,
                total: Size(0),
                downloaded: Size(0),
                uploaded: Size(0),
                from_web_seeds: Size(0),
                from_peers: Size(0),
                from_resume: Size(0),
                elapsed_ms: 0,
                elapsed_human: "0s".into(),
                mean_rate: Size(0),
                mean_rate_human: format_rate(0),
                peers_seen: 0,
                redials: Vec::new(),
                sources: Vec::new(),
                output_directory: options.directory.display().to_string(),
                renamed: Vec::new(),
                shared: Vec::new(),
                announced: Vec::new(),
                metalink: None,
                attribution: None,
                code: error.code(),
                error: Some(error.to_string()),
            }
        }
    }
}

async fn one_inner(
    engine: &Engine,
    plan: &Plan,
    options: &Options,
    tx: &mpsc::Sender<Msg>,
) -> Result<TorrentReport> {
    let add = AddOptions {
        overwrite: options.overwrite,
        only_files: options.only_files.clone(),
        trackers: plan.trackers.clone(),
        disable_trackers: plan.trackers.as_ref().is_some_and(Vec::is_empty),
        initial_peers: options.peers.clone(),
        download_rate: options.torrent_download_rate,
        upload_rate: options.torrent_upload_rate,
        ..Default::default()
    };
    let handle = match plan.torrent_bytes.clone() {
        // A Metalink's torrent was fetched while the plans were being built,
        // so the session gets those exact bytes rather than the URL again.
        Some(bytes) => engine.add_bytes(&plan.source, bytes, &add).await?,
        None => engine.add(&plan.source, &add).await?,
    };
    let snapshot = engine.snapshot(&handle);
    let _ = tx
        .send(Msg::Event(
            "torrent_added",
            json!({
                "source": plan.source,
                "info_hash": snapshot.info_hash,
                "name": snapshot.name,
            }),
        ))
        .await;

    engine
        .wait_until_initialized_within(&handle, options.init_timeout)
        .await?;
    // A rename is not an error, but a caller who is not told about one cannot
    // find the file it asked for, so it goes to stderr as well as into the
    // report.
    if let Some(planned) = engine.path_plan(&handle)
        && !planned.is_clean()
    {
        let reasons: Vec<&str> = planned
            .reasons()
            .iter()
            .map(|reason| reason.description())
            .collect();
        let _ = tx
            .send(Msg::Warn(format!(
                "{} of {} paths were changed to be writable here ({}); see `renamed` in --json",
                planned.renames.len(),
                planned.disk_paths.len(),
                reasons.join(", ")
            )))
            .await;
    }
    let layout = Arc::new(engine.layout(&handle).ok_or_else(|| {
        Error::source_resolution(format!("{}: the torrent has no metadata", plan.source))
    })?);
    let _ = tx
        .send(Msg::Event(
            "metadata_resolved",
            json!({
                "info_hash": handle.info_hash().as_string(),
                "name": layout.name,
                "files": layout.files.len(),
                "piece_count": layout.piece_count(),
                "piece_length": layout.piece_length,
                "total_bytes": layout.total_length,
            }),
        ))
        .await;

    // What the hash check found already on disk, read once the check has
    // finished and before anything is fetched.
    //
    // `progress_bytes` is everything the torrent has, not everything this run
    // fetched, so charging `progress_bytes - served` to peers charges them for
    // a resumed download's existing bytes as well. A run that resumed 45 MiB
    // of a 64 MiB file with no peer in the swarm reported 45 MiB from peers.
    // See `TODO/multi-source.md`, T-139.
    let resumed = engine.snapshot(&handle).progress_bytes;

    if options.hash_check_only {
        let snapshot = engine.snapshot(&handle);
        return Ok(finish(
            plan,
            options,
            &snapshot,
            &[],
            Stopped::Completed,
            Duration::ZERO,
            Vec::new(),
            resumed,
            renames(engine, &handle),
        ));
    }

    // `--piece-selector sequential` holds the session's priority window at the
    // earliest piece still missing, and it is registered **here**, before any
    // source is attached, rather than in the watch loop below.
    //
    // The reason is a race that the measurement found rather than the design
    // predicted. `librqbit`'s natural order yields the last piece of a file
    // second, so if anything can ask for a piece before the window exists, the
    // tail arrives early and the order has a descent in it. Registering before
    // the sources means nothing can: under `--web-seed-only` the bridges are
    // the only peers, and they do not exist yet. Against a real swarm it is
    // best effort, because a peer dialled during the hash check may already
    // have been handed one. See `TODO/performance.md`, T-032.
    let mut ordering = match options.in_order {
        false => None,
        true => {
            let mut driver =
                bit_cli_core::piece_order::InOrder::new(handle.clone(), layout.clone());
            if let Some(have) = engine.have_pieces(&handle) {
                // A failure here loses the ordering, not the download: the
                // window is a hint to a picker that works without it.
                if driver.advance(&have).await.is_err() {
                    let _ = tx
                        .send(Msg::Warn(
                            "the session refused a piece priority window, so pieces will arrive in its own order".to_string(),
                        ))
                        .await;
                }
            }
            Some(driver)
        }
    };

    // Files an earlier torrent in this run has already written, which this one
    // is proven to hold too. These are sources like any other: scoped to one
    // file, checked per piece on the way in, and reported with their own
    // origin. See `TODO/multi-source.md`, T-140.
    let (donated, shared) = donated_sources(plan, options, &layout);
    for file in &shared {
        let _ = tx
            .send(Msg::Warn(format!(
                "file {} ({}) is proven to be the file {} holds at index {}, reading it from {} rather than fetching it",
                file.index, file.path, file.from_source, file.from_index, file.from_path
            )))
            .await;
    }
    let mut declared = plan.specs.clone();
    declared.extend(donated);

    // The whole-run concurrency cap is shared out across the declared sources,
    // so `--web-seed-max-total 8` with four mirrors means two requests each
    // rather than eight each.
    let specs = apply_preference(
        apply_max_total(&declared, options.max_total),
        options.prefer,
    );
    let (sources, _set, ledger) = swarm::attach_sources_tracked(
        engine,
        &handle,
        &layout,
        &specs,
        &swarm::AttachOptions {
            require: options.require,
            peers_available: !options.web_seed_only,
            cache_windows: cache_windows(&specs),
            trace: options.trace_http,
            verify: options.verify,
        },
    )
    .await?;
    for source in &sources {
        let _ = tx
            .send(Msg::Event(
                "source_added",
                json!({
                    "index": source.index,
                    "url": source.url,
                    "origin": source.origin,
                    "scope": source.scope,
                    "whole_pieces": source.whole_pieces,
                }),
            ))
            .await;
    }

    let mut announced: Vec<SentAnnounce> = Vec::new();
    let outcome = watch(
        engine,
        &handle,
        &layout,
        &sources,
        &ledger,
        plan,
        options,
        tx,
        &mut announced,
        ordering.take(),
    )
    .await;
    for source in &sources {
        source.stop();
    }
    let (stopped, elapsed, redials) = outcome;

    // `stopped` last, whatever ended the run. A tracker that is not told keeps
    // handing this address out until the record expires, which on a public
    // tracker is the next half hour.
    if let Some(sent) = announce_event(
        engine,
        &handle,
        plan,
        options,
        bit_cli_core::tracker::Event::Stopped,
        elapsed,
    )
    .await
    {
        announced.push(sent);
    }
    let snapshot = engine.snapshot(&handle);
    let mut report = finish(
        plan,
        options,
        &snapshot,
        &sources,
        stopped,
        elapsed,
        redials,
        resumed,
        renames(engine, &handle),
    );
    report.shared = shared;
    report.announced = announced;
    // Set here rather than passed into `finish`, which already takes nine
    // arguments and does not otherwise know the ledger exists.
    report.attribution = (!sources.is_empty()).then(|| ledger.stats());
    if let Some(metalink) = &plan.metalink {
        let (metalink_report, code) = check_metalink(metalink, engine, &handle, options, &report);
        if let Some(checksum) = &metalink_report.checksum {
            // Serialised from the report's own struct rather than rebuilt
            // here, so a field the report omits is omitted from the event too.
            // Rebuilding it with `json!` put `"not_checked": null` in every
            // successful run, which documents a field as always-null and tells
            // a reader nothing.
            let mut payload = serde_json::to_value(checksum).unwrap_or_default();
            if let Some(fields) = payload.as_object_mut() {
                fields.insert("info_hash".to_string(), json!(report.info_hash));
            }
            let _ = tx.send(Msg::Event("metalink_checked", payload)).await;
            if checksum.matched == Some(false) {
                let _ = tx
                    .send(Msg::Warn(format!(
                        "the metalink's {} checksum does not match the payload: it says {}, the bytes hash to {}. The payload passed the torrent's own piece hashes, so the metalink is the document that disagrees.",
                        checksum.algorithm,
                        checksum.expected,
                        checksum.actual.as_deref().unwrap_or("nothing"),
                    )))
                    .await;
            }
        }
        report.metalink = Some(metalink_report);
        // A checksum that disagrees is the failure this feature exists to
        // find, so it decides the torrent's code unless something worse
        // already had.
        if let Some(code) = code
            && report.code == ExitCode::Success
        {
            report.code = code;
        }
    }
    // A finished torrent can lend its files to the ones after it. An
    // unfinished one cannot: its files are on disk but not all of their bytes
    // are.
    if report.finished {
        publish_donor(engine, &handle, plan, options);
    }

    let _ = tx
        .send(Msg::Event(
            "torrent_completed",
            json!({
                "info_hash": report.info_hash,
                "name": report.name,
                "stopped": report.stopped,
                "finished": report.finished,
                "downloaded_bytes": report.downloaded.0,
                "from_web_seeds": report.from_web_seeds.0,
                "from_peers": report.from_peers.0,
                "from_resume": report.from_resume.0,
                "elapsed_ms": report.elapsed_ms,
            }),
        ))
        .await;
    Ok(report)
}

/// Watch one torrent until a stop condition fires.
///
/// Three things wake this loop, and the report interval is only one of them.
/// The other two are the events that end a run: the torrent completing, and
/// the earliest deadline the caller set. Waking only on the tick would make
/// every run as long as the next multiple of `--report-interval`, which
/// defaults to a second, so a download that finished in 1.1 s would take 2 s
/// and a `--timeout 30s` would fire at 31. See `TODO/performance.md`, T-030.
#[allow(clippy::too_many_arguments)]
async fn watch(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    layout: &Arc<Layout>,
    sources: &[AttachedSource],
    // Where every block a source served is recorded, so a piece that failed
    // can name the mirror that broke it. See `TODO/webseed.md`, T-179.
    ledger: &bit_cli_core::webseed::ledger::BlockLedger,
    plan: &Plan,
    options: &Options,
    tx: &mpsc::Sender<Msg>,
    announced: &mut Vec<SentAnnounce>,
    // The piece priority window, already registered by the caller so that
    // nothing could ask for a piece before it existed.
    mut ordering: Option<bit_cli_core::piece_order::InOrder>,
) -> (Stopped, Duration, Vec<Redial>) {
    let lengths: Vec<u64> = layout.files.iter().map(|f| f.length).collect();
    let mut progress = Progress::new(layout.piece_count(), lengths);
    let mut redials: Vec<Redial> = Vec::new();
    // Measured from the last re-dial rather than from the last byte, so a
    // stall that outlasts the interval re-dials once per interval instead of
    // once per report tick. `--stop-timeout` keeps measuring from the last
    // byte, which is what lets a run both re-dial and still give up.
    let mut last_redial = std::time::Instant::now();
    let mut ticker = tokio::time::interval(options.report_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut reported_failures = HashSet::new();
    let mut reported_cooldowns: HashSet<(usize, u64)> = HashSet::new();
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);

    // Completion resolves once and must not be polled again, so it is guarded
    // rather than recreated: a run that goes on seeding after completing is
    // still driven by the tick.
    let completion = engine.wait_until_completed(handle);
    tokio::pin!(completion);
    let mut completed = false;

    // The soonest moment a deadline could fire. `should_stop` decides whether
    // it actually does; this only makes sure the loop is awake to ask, and it
    // measures from here because `should_stop` measures from `progress`, which
    // starts on the line above.
    //
    // With no deadline set the sleep is parked a day out rather than made
    // optional, because an optional future in a `select!` needs either a boxed
    // `Option` or a second arm. A run still going after a day wakes once more
    // than it needed to and nothing else changes.
    const NO_DEADLINE: Duration = Duration::from_secs(86_400);
    let limit = [options.stop.timeout, options.stop.stop_after]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(NO_DEADLINE);
    let deadline = tokio::time::sleep_until(tokio::time::Instant::now() + limit);
    tokio::pin!(deadline);
    let mut deadline_fired = false;

    // The priority window gets a ticker of its own rather than riding the
    // report tick, because how often a caller wants progress printed is not a
    // statement about what order pieces should arrive in: `--report-interval
    // 10s` must not mean a window that moves twice a minute. Fifty
    // milliseconds is well inside the 32 MiB of lookahead the window carries,
    // even on loopback. See `TODO/performance.md`, T-032.
    let mut order_ticker = tokio::time::interval(Duration::from_millis(50));
    order_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = &mut interrupt => return (Stopped::Interrupted, progress.elapsed(), redials),
            _ = ticker.tick() => {}
            _ = order_ticker.tick(), if ordering.is_some() => {
                // This arm does not fall through to the body below, which is a
                // progress report: it fires twenty times a second and a run
                // does not want twenty progress events a second.
                //
                // Losing the ordering loses the ordering and not the download.
                // The window is a hint to a picker that works without it, so a
                // failure here drops back to the natural order rather than
                // failing a run over a preference.
                if let Some(driver) = ordering.as_mut() {
                    let keep = match engine.have_pieces(handle) {
                        // Pointing at nothing means nothing is missing, so the
                        // window has no more work and the stream is released.
                        Some(have) => matches!(driver.advance(&have).await, Ok(Some(_))),
                        // No bitfield yet is not a reason to give up: the
                        // torrent may still be hash-checking, and a tick that
                        // lands in that window used to disable the ordering
                        // for the whole run.
                        None => true,
                    };
                    if !keep {
                        ordering = None;
                    }
                }
                continue;
            }
            _ = &mut completion, if !completed => {
                completed = true;
                // Now, not when the run ends: a run that keeps seeding after
                // completing would otherwise tell the tracker minutes late,
                // and the seeder count is what a tracker uses this for.
                if let Some(sent) = announce_event(
                    engine,
                    handle,
                    plan,
                    options,
                    bit_cli_core::tracker::Event::Completed,
                    progress.elapsed(),
                )
                .await
                {
                    announced.push(sent);
                }
            }
            _ = &mut deadline, if !deadline_fired => deadline_fired = true,
        }

        let snapshot = engine.snapshot(handle);
        let have = engine.have_pieces(handle);
        let file_progress = handle.stats().file_progress;
        let tick = progress.observe(&snapshot, have.as_deref(), &file_progress);

        for piece in tick.verified_pieces {
            let _ = tx
                .send(Msg::Event(
                    "piece_verified",
                    json!({ "piece": piece, "length": layout.piece_size(piece) }),
                ))
                .await;
        }
        for file in tick.completed_files {
            let _ = tx
                .send(Msg::Event(
                    "file_completed",
                    json!({
                        "file": file,
                        "path": layout.file(file).map(|f| f.display_path()),
                        "length": layout.file(file).map(|f| f.length),
                    }),
                ))
                .await;
        }

        // Attribution runs before the failure reporting below, so a source
        // convicted on this tick is retired and reported as failed on this
        // tick rather than the next. The correct bytes come off the disk, from
        // a piece the session has already hash-checked, so nothing is fetched
        // twice; and only a block two sources disagreed about is ever read,
        // which in a healthy run is none of them. See `TODO/webseed.md`,
        // T-179.
        if let Some(have) = have.as_deref() {
            let convicted = swarm::resolve_convictions(ledger, sources, have, |offset, length| {
                read_payload(engine, handle, options, layout, offset, length)
            });
            // Warned here and reported as an event below, by the
            // `source_failed` the retirement produces. A second event carrying
            // a subset of the same `SourceReport` would be two names for one
            // fact, and `sources[].convictions` already carries the piece, the
            // offset and both hashes.
            for conviction in convicted {
                let url = sources
                    .iter()
                    .find(|s| s.index == conviction.source)
                    .map(|s| s.url.clone())
                    .unwrap_or_default();
                let _ = tx
                    .send(Msg::Warn(format!(
                        "web seed {url} {conviction}, so it is retired"
                    )))
                    .await;
            }
        }

        for source in sources {
            if source.state() == bit_cli_core::webseed::BridgeState::Failed
                && reported_failures.insert(source.index)
            {
                let report = source.report();
                let _ = tx
                    .send(Msg::Warn(format!(
                        "web seed {} is unusable: {}",
                        report.url,
                        report.error.as_deref().unwrap_or("no reason given")
                    )))
                    .await;
                let _ = tx
                    .send(Msg::Event(
                        "source_failed",
                        serde_json::to_value(&report).unwrap_or_default(),
                    ))
                    .await;
            }
            // Keyed by how many times the source has cooled down, not by its
            // index, so a mirror that goes out, comes back, and goes out again
            // is reported each time. A run waiting on a sleeping source has to
            // be told, or the wait looks like a hang. See
            // `TODO/multi-source.md`, T-137.
            if source.state() == bit_cli_core::webseed::BridgeState::Cooling {
                let report = source.report();
                if reported_cooldowns.insert((source.index, report.cooldowns)) {
                    let _ = tx
                        .send(Msg::Warn(format!(
                            "web seed {} is cooling down for {}: {}",
                            report.url,
                            bit_cli_core::units::format_duration(Duration::from_millis(
                                report.cooldown_remaining_ms.unwrap_or(0)
                            )),
                            report.error.as_deref().unwrap_or("no reason given")
                        )))
                        .await;
                    let _ = tx
                        .send(Msg::Event(
                            "source_cooling",
                            serde_json::to_value(&report).unwrap_or_default(),
                        ))
                        .await;
                }
            }
        }

        let served: u64 = sources.iter().map(AttachedSource::served_bytes).sum();
        let _ = tx
            .send(Msg::Event(
                "progress",
                json!({
                    "info_hash": snapshot.info_hash,
                    "progress_bytes": snapshot.progress_bytes,
                    "total_bytes": snapshot.total_bytes,
                    "percent": format!("{:.2}", snapshot.fraction() * 100.0),
                    "download_rate": snapshot.download_rate,
                    "upload_rate": snapshot.upload_rate,
                    "peers": snapshot.peers,
                    "from_web_seeds": served,
                    "eta_ms": snapshot.eta_ms,
                    "eta_confidence": snapshot.eta_confidence,
                    // What the process costs right now, so a long run reads a slope out
                    // of the event stream rather than sampling the process from outside.
                    // See `TODO/memory.md`, T-040.
                    "process": bit_cli_core::sysinfo::Process::sample(),
                }),
            ))
            .await;
        let _ = tx
            .send(Msg::Progress(swarm::progress_line(&snapshot, sources)))
            .await;

        // A run with no peers and every HTTP source dead cannot finish, and
        // waiting out the deadline to say so wastes the caller's time.
        if !sources.is_empty()
            && options.web_seed_only
            && sources
                .iter()
                .all(|s| s.state() == bit_cli_core::webseed::BridgeState::Failed)
        {
            return (Stopped::Failed, progress.elapsed(), redials);
        }

        let seeding = options.stop.seed_ratio.is_some() || options.stop.seed_time.is_some();
        if let Some(reason) = progress.should_stop(&snapshot, &options.stop, seeding) {
            return (reason, progress.elapsed(), redials);
        }

        // Checked after the stop conditions, so a run that was going to give
        // up this tick gives up rather than re-dialling on its way out.
        if let Some(interval) = options.redial_after
            && !snapshot.finished
            && (redials.len() as u32) < options.max_redials
            && progress.stalled_for() >= interval
            && last_redial.elapsed() >= interval
        {
            let attempt = redials.len() as u32 + 1;
            let stalled = progress.stalled_for();
            let error = engine.redial(handle).await.err().map(|e| e.to_string());
            last_redial = std::time::Instant::now();
            let redial = Redial {
                attempt,
                at_ms: progress.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                stalled_ms: stalled.as_millis().min(u128::from(u64::MAX)) as u64,
                peers_dropped: snapshot.peers.live,
                error: error.clone(),
            };
            let _ = tx
                .send(Msg::Event(
                    "peer_redial",
                    serde_json::to_value(&redial).unwrap_or_default(),
                ))
                .await;
            if let Some(reason) = &error {
                let _ = tx
                    .send(Msg::Warn(format!("re-dial {attempt} failed: {reason}")))
                    .await;
            }
            redials.push(redial);
        }
    }
}

/// Check the payload against the Metalink's own checksum, and say which of the
/// two documents is wrong when they disagree.
///
/// The order of the guards is the point. The check runs only on a payload the
/// session has finished and hash-checked against the torrent's own piece
/// hashes, so a digest that then disagrees is evidence about the Metalink and
/// not about the bytes. Every guard that stops the check writes a
/// `not_checked` reason, because a checksum that was not computed is not a
/// checksum that passed. See `TODO/cli-surface.md`, T-113.
fn check_metalink(
    metalink: &MetalinkPlan,
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    options: &Options,
    report: &TorrentReport,
) -> (MetalinkReport, Option<ExitCode>) {
    let agreement = MetalinkAgreement {
        file_index: metalink.agreement.file_index,
        matched_by: metalink.agreement.matched_by,
        metalink_size: metalink.agreement.metalink_size,
        torrent_size: metalink.agreement.torrent_size,
        size_agrees: metalink.agreement.size_agrees,
    };
    let mut out = MetalinkReport {
        version: metalink.version,
        file: metalink.file_name.clone(),
        torrent_url: metalink.torrent_url.clone(),
        torrent_fallbacks: metalink.torrent_fallbacks.clone(),
        mirrors_listed: metalink.mirrors_listed,
        mirrors_registered: metalink.mirrors_registered,
        mirrors_unsupported: metalink.mirrors_unsupported.clone(),
        agreement,
        checksum: None,
    };
    // Two documents that declare different lengths describe different files,
    // and that is decided before anything is hashed.
    let code = metalink
        .agreement
        .disagrees()
        .then_some(ExitCode::HashMismatch);

    let Some(checksum) = &metalink.checksum else {
        if let Some(algorithm) = &metalink.unusable_algorithm {
            out.checksum = Some(MetalinkChecksum {
                algorithm: algorithm.clone(),
                expected: String::new(),
                actual: None,
                matched: None,
                bytes_hashed: None,
                path: None,
                not_checked: Some(format!("this cannot compute {algorithm}")),
            });
        }
        return (out, code);
    };
    let mut result = MetalinkChecksum {
        algorithm: checksum.algorithm.clone(),
        expected: checksum.value.clone(),
        actual: None,
        matched: None,
        bytes_hashed: None,
        path: None,
        not_checked: None,
    };

    let stop = |result: &mut MetalinkChecksum, why: String| {
        result.not_checked = Some(why);
    };
    if !report.finished {
        stop(
            &mut result,
            "the download did not finish, so there is nothing complete to hash".to_string(),
        );
    } else if let Some(index) = metalink.agreement.file_index {
        match payload_path(engine, handle, options, index) {
            None => stop(
                &mut result,
                "the torrent's paths were not planned, so the file on disk cannot be named"
                    .to_string(),
            ),
            Some(path) => {
                result.path = Some(path.display().to_string());
                match checksum.verify_file(&path) {
                    Ok(verified) => {
                        result.actual = Some(verified.actual);
                        result.matched = Some(verified.matched);
                        result.bytes_hashed = Some(verified.bytes_hashed);
                    }
                    Err(error) => stop(&mut result, error.to_string()),
                }
            }
        }
    } else {
        stop(
            &mut result,
            format!(
                "the metalink's checksum could not be attributed to a file in the torrent ({})",
                metalink.agreement.matched_by
            ),
        );
    }

    let mismatch = (result.matched == Some(false)).then_some(ExitCode::HashMismatch);
    out.checksum = Some(result);
    (out, code.or(mismatch))
}

/// Where one file of a finished torrent actually sits on disk.
///
/// The torrent's own path is not necessarily the path on disk: a name the
/// filesystem refuses, or one that would leave the output directory, is
/// rewritten before anything is opened. Hashing the torrent's path rather than
/// the planned one would hash a file that is not there.
fn payload_path(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    options: &Options,
    index: usize,
) -> Option<std::path::PathBuf> {
    let layout = engine.layout(handle)?;
    let planned = engine.path_plan(handle)?;
    let relative = planned.disk_paths.get(index)?;
    let root = bit_cli_core::storage::payload_root(&options.directory, &layout);
    Some(root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)))
}

/// Read verified bytes back out of the payload the session is writing.
///
/// This is where the correct bytes for a disputed block come from. Reading
/// them off the disk rather than fetching them again is the whole reason smart
/// ban costs nothing: the session has already hash-checked the piece holding
/// them, so the bytes on disk are the truth by definition, and a source whose
/// recorded hash disagrees with them is proved wrong rather than suspected.
///
/// `None` when the range could not be read, which leaves the piece for the
/// next pass. See `TODO/webseed.md`, T-179.
fn read_payload(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    options: &Options,
    layout: &Layout,
    offset: u64,
    length: u32,
) -> Option<Vec<u8>> {
    let planned = engine.path_plan(handle)?;
    let root = bit_cli_core::storage::payload_root(&options.directory, layout);
    bit_cli_core::storage::read_range(
        &root,
        layout,
        &planned.disk_paths,
        offset..offset + u64::from(length),
    )
}

/// Where this torrent's files were actually written, when that is not where
/// the torrent said.
///
/// A torrent path that cannot exist on the filesystem, or that would leave the
/// output directory, is rewritten before anything is opened. The caller has to
/// be told, or it cannot find what it downloaded.
fn renames(engine: &Engine, handle: &bit_cli_core::engine::Handle) -> Vec<Rename> {
    engine
        .path_plan(handle)
        .map(|plan| plan.renames)
        .unwrap_or_default()
}

/// A `file:` source per donation whose donor has finished, and the report rows
/// that say where each came from.
///
/// A donation whose donor has not finished yet is silently nothing: under
/// `-j 1` the donor ran first and this is decided; above that the two are in
/// flight together and there is nothing to read. That is the honest behaviour
/// either way, and it is why the entry prices attaching a source mid-run
/// separately. See `TODO/multi-source.md`, T-140.
fn donated_sources(
    plan: &Plan,
    options: &Options,
    layout: &Layout,
) -> (Vec<SourceSpec>, Vec<SharedFile>) {
    use bit_cli_core::webseed::binding::Origin;
    use bit_cli_core::webseed::composition::Mode;
    use bit_cli_core::webseed::scope::Scope;

    let mut specs = Vec::new();
    let mut shared = Vec::new();
    if plan.donations.is_empty() {
        return (specs, shared);
    }
    let Ok(donors) = options.donors.lock() else {
        return (specs, shared);
    };
    for donation in &plan.donations {
        let Some(donor) = donors.get(&donation.donor) else {
            continue;
        };
        let Some(relative) = donor.disk_paths.get(donation.donor_index) else {
            continue;
        };
        let mut path = donor.root.clone();
        for component in relative.split('/').filter(|part| !part.is_empty()) {
            path.push(component);
        }
        // A donor that finished has the file. Checking anyway costs one stat
        // and turns a source that would fail every request into no source.
        if !path.is_file() {
            continue;
        }
        let url = bit_cli_core::webseed::local::url_of(&path);
        let Ok(scope) = Scope::parse(&format!("file:{}", donation.index)) else {
            continue;
        };
        specs.push(
            SourceSpec::new(url.clone(), Origin::SharedFile)
                .with_scope(scope)
                .with_mode(Mode::Exact),
        );
        shared.push(SharedFile {
            index: donation.index,
            path: layout
                .file(donation.index)
                .map(|file| file.display_path())
                .unwrap_or_default(),
            length: Size(donation.length),
            from_source: donor.source.clone(),
            from_info_hash: donor.info_hash.clone(),
            from_index: donation.donor_index,
            from_path: path.display().to_string(),
            pieces_compared: donation.pieces_compared,
            bytes_proven: Size(donation.bytes_proven),
        });
    }
    (specs, shared)
}

/// Tell every tracker this torrent uses that something happened.
///
/// The announce carries the session's own peer id and listening port, so a
/// tracker updates the record the session created rather than registering a
/// second peer that then has to be cleaned up. A tracker that fails is counted
/// and nothing else: this is a courtesy announce, and a run does not fail
/// because a tracker was down when it ended.
///
/// See `TODO/trackers.md`, T-062.
async fn announce_event(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    plan: &Plan,
    options: &Options,
    event: bit_cli_core::tracker::Event,
    at: Duration,
) -> Option<SentAnnounce> {
    use bit_cli_core::tracker::{Announce, Client};

    let urls = plan.trackers.clone().unwrap_or_default();
    if urls.is_empty() {
        return None;
    }
    let port = engine.listen_addr().map(|addr| addr.port()).unwrap_or(0);
    let snapshot = engine.snapshot(handle);
    let request = Announce {
        event,
        uploaded: snapshot.uploaded_bytes,
        downloaded: snapshot.progress_bytes,
        left: snapshot.total_bytes.saturating_sub(snapshot.progress_bytes),
        // A client that is leaving or has finished is not asking for peers.
        numwant: 0,
        ..Announce::new(
            handle.info_hash().0,
            handle.shared().peer_id.0,
            port,
            snapshot.total_bytes.saturating_sub(snapshot.progress_bytes),
        )
    };

    let client = Client::new(
        &format!("bit-cli/{}", bit_cli_core::VERSION),
        options.tracker_timeout,
        options.tracker_connect_timeout,
    )
    .ok()?;
    let client = std::sync::Arc::new(client);
    let mut work = tokio::task::JoinSet::new();
    for url in &urls {
        let client = client.clone();
        let url = url.clone();
        let request = request.clone();
        work.spawn(async move { client.announce(&url, 0, &request).await });
    }
    let mut accepted = 0usize;
    while let Some(finished) = work.join_next().await {
        if let Ok(result) = finished
            && result.ok
        {
            accepted += 1;
        }
    }
    Some(SentAnnounce {
        event: event.as_str().unwrap_or("none"),
        trackers: urls.len(),
        accepted,
        at_ms: at.as_millis().min(u128::from(u64::MAX)) as u64,
    })
}

/// Record where a finished torrent put its files, so the ones after it can
/// read them.
fn publish_donor(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    plan: &Plan,
    options: &Options,
) {
    let Some(planned) = engine.path_plan(handle) else {
        return;
    };
    let Ok(mut donors) = options.donors.lock() else {
        return;
    };
    if let Some(entry) = donors.get_mut(&plan.index) {
        entry.disk_paths = planned.disk_paths;
    }
}

#[allow(clippy::too_many_arguments)]
fn finish(
    plan: &Plan,
    options: &Options,
    snapshot: &TorrentSnapshot,
    sources: &[AttachedSource],
    stopped: Stopped,
    elapsed: Duration,
    redials: Vec<Redial>,
    resumed: u64,
    renamed: Vec<Rename>,
) -> TorrentReport {
    let served: u64 = sources.iter().map(AttachedSource::served_bytes).sum();
    let elapsed_ms = elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
    let mean = match elapsed_ms {
        0 => 0,
        ms => snapshot.progress_bytes.saturating_mul(1000) / ms,
    };
    TorrentReport {
        source: plan.source.clone(),
        info_hash: snapshot.info_hash.clone(),
        name: snapshot.name.clone(),
        stopped,
        finished: snapshot.finished,
        total: Size(snapshot.total_bytes),
        downloaded: Size(snapshot.progress_bytes),
        uploaded: Size(snapshot.uploaded_bytes),
        from_web_seeds: Size(served),
        from_peers: Size(
            snapshot
                .progress_bytes
                .saturating_sub(served)
                .saturating_sub(resumed),
        ),
        from_resume: Size(resumed),
        elapsed_ms,
        elapsed_human: bit_cli_core::units::format_duration(elapsed),
        mean_rate: Size(mean),
        mean_rate_human: format_rate(mean),
        peers_seen: snapshot.peers.seen,
        redials,
        sources: sources.iter().map(AttachedSource::report).collect(),
        output_directory: options.directory.display().to_string(),
        renamed,
        shared: Vec::new(),
        announced: Vec::new(),
        metalink: None,
        attribution: None,
        code: stopped.code(),
        error: snapshot.error.clone(),
    }
}

/// Apply `--prefer-web-seed` to the declared sources.
///
/// `bit-cli` cannot reach `librqbit`'s piece picker, so it cannot tell the
/// picker "take this piece from HTTP rather than from that peer". What it can
/// do is give the HTTP source more of what decides which answer arrives first,
/// because the session takes whichever peer answers a block soonest.
///
/// What decides that is receive paths, not requests. The flag used to double
/// the per-source request budget, and `TODO/webseed.md` T-009 measured that at
/// 0.81x: eight times the requests in flight on one connection is slightly
/// slower, not faster. Doubling the connections is 1.92x on the same
/// measurement. So the preference is a doubled connection count, bounded, and
/// the request budget is left alone.
///
/// This is still not the picker. `TODO/webseed.md` T-003 records the gap and
/// what closing it would take.
fn apply_preference(specs: Vec<SourceSpec>, prefer: bool) -> Vec<SourceSpec> {
    if !prefer {
        return specs;
    }
    specs
        .into_iter()
        .map(|mut spec| {
            spec.limits.connections = (spec.limits.connections().saturating_mul(2)).clamp(2, 8);
            spec
        })
        .collect()
}

/// Divide the whole-run request budget across the declared sources.
fn apply_max_total(specs: &[SourceSpec], max_total: Option<usize>) -> Vec<SourceSpec> {
    let Some(total) = max_total.filter(|t| *t > 0) else {
        return specs.to_vec();
    };
    if specs.is_empty() {
        return Vec::new();
    }
    let share = (total / specs.len()).max(1);
    specs
        .iter()
        .map(|spec| {
            let mut spec = spec.clone();
            spec.limits.concurrency = spec.limits.concurrency.min(share).max(1);
            spec
        })
        .collect()
}

/// How many windows each source caches.
///
/// Memory is `windows * chunk_size` per source, so the window count comes down
/// as the chunk size goes up. Four windows of the default 4 MiB is 16 MiB per
/// source, which is the budget a mirror gets before eviction starts.
pub(crate) fn cache_windows(specs: &[SourceSpec]) -> usize {
    let largest = specs
        .iter()
        .map(|s| s.limits.chunk_size)
        .max()
        .unwrap_or(bit_cli_core::units::MIB);
    ((16 * bit_cli_core::units::MIB) / largest.max(1)).clamp(2, 16) as usize
}

/// Resolve `--select-file` and `--exclude-file` into explicit indices.
///
/// `None` means every file, which is not the same as an empty list: an empty
/// list would download nothing.
fn selection(args: &crate::cli::SelectionArgs) -> Result<Option<Vec<usize>>> {
    if args.select_file.is_empty() && args.exclude_file.is_empty() {
        return Ok(None);
    }
    let parse = |values: &[String], flag: &str| -> Result<Vec<usize>> {
        let mut out = Vec::new();
        for value in values {
            for term in value.split(',') {
                let term = term.trim();
                if term.is_empty() {
                    continue;
                }
                match term.split_once('-') {
                    None => out.push(term.parse::<usize>().map_err(|_| index_error(flag, term))?),
                    Some((start, "")) => {
                        // An open-ended range needs the file count, which is
                        // not known until the metadata resolves. Refuse rather
                        // than guessing at an upper bound.
                        let _ = start;
                        return Err(Error::usage(format!(
                            "--{flag} `{term}`: an open-ended range needs the file count; list the indices or use a closed range"
                        )));
                    }
                    Some((start, end)) => {
                        let start: usize =
                            start.trim().parse().map_err(|_| index_error(flag, term))?;
                        let end: usize = end.trim().parse().map_err(|_| index_error(flag, term))?;
                        if start > end {
                            return Err(Error::usage(format!("--{flag} `{term}` runs backwards")));
                        }
                        out.extend(start..=end);
                    }
                }
            }
        }
        Ok(out)
    };

    let selected = parse(&args.select_file, "select-file")?;
    let excluded: HashSet<usize> = parse(&args.exclude_file, "exclude-file")?
        .into_iter()
        .collect();
    if selected.is_empty() {
        // Only exclusions were given, and the file count is not known yet, so
        // the exclusion is applied once the metadata resolves. Until then
        // there is nothing to narrow.
        return Ok(None);
    }
    let mut out: Vec<usize> = selected
        .into_iter()
        .filter(|i| !excluded.contains(i))
        .collect();
    out.sort_unstable();
    out.dedup();
    if out.is_empty() {
        return Err(Error::usage(
            "--select-file and --exclude-file together select no files at all",
        ));
    }
    Ok(Some(out))
}

fn index_error(flag: &str, term: &str) -> Error {
    Error::usage(format!(
        "--{flag} `{term}` is not a file index or an index range"
    ))
    .with("value", term.to_string())
}

/// Resolve and report without fetching anything.
fn dry_run(
    args: &DownloadArgs,
    global: &Global,
    setup: &SessionSetup<'_>,
    renderer: &mut Renderer,
    env: &mut Env,
    directory: &std::path::Path,
) -> Result<ExitCode> {
    let mut planned = Vec::new();
    for source in &args.sources {
        let kind = Kind::classify(source, env)?;
        let meta = match &kind {
            Kind::File(path) => Some(Metainfo::read(path)?),
            _ => None,
        };
        // A dry run reads the Metalink and does not fetch the torrent it
        // names. Everything the document itself claims is reportable without
        // the network: the mirrors, the torrent URL, the size, the checksum.
        // What needs the network is the `.torrent`, and `needs_network` on
        // this row is what says so. This is the cheapest way to check that a
        // `.meta4` says what its author meant.
        let metalink = match &kind {
            Kind::Metalink(path) => {
                let document = Metalink::read(path)?;
                let file = document.single_file()?.clone();
                Some((document.version.as_str(), file))
            }
            _ => None,
        };
        let specs = webseed_args::collect(
            &args.web_seeds,
            meta.as_ref(),
            metalink.as_ref().map(|(_, file)| file),
            env,
            webseed_args::no_network,
        )?;
        // A dry run reports without doing, so a list URL is refused rather
        // than fetched. That is the decision `--web-seed-list-url` already
        // takes on this same command.
        let trackers = setup
            .tracker_list(meta.as_ref(), env, webseed_args::no_network)?
            .unwrap_or_default();
        let coverage = match (&meta, specs.is_empty()) {
            (Some(meta), false) => {
                let layout = meta.layout();
                let set = bit_cli_core::webseed::binding::BindingSet::resolve(
                    &layout,
                    &meta.info_hash().hex(),
                    &specs,
                )?;
                if args.web_seeds.web_seed_require {
                    set.require_coverage(!args.web_seeds.web_seed_only)?;
                }
                Some(json!({
                    "covered_bytes": set.covered.len(),
                    "uncovered_bytes": set.uncovered.len(),
                    "uncovered_pieces": set.uncovered_pieces,
                    "complete": set.is_complete(),
                }))
            }
            _ => None,
        };
        planned.push(json!({
            "source": source,
            "kind": kind.name(),
            "needs_network": kind.needs_network(),
            "name": meta.as_ref().map(|m| m.layout().name),
            "info_hash": meta.as_ref().map(|m| m.info_hash().hex()),
            "total_bytes": meta.as_ref().map(|m| m.layout().total_length),
            "web_seeds": specs.iter().map(|s| json!({
                "url": s.url,
                "origin": s.origin.as_str(),
                "scope": s.scope.text(),
                "mode": s.mode.as_str(),
            })).collect::<Vec<_>>(),
            "trackers": trackers,
            "coverage": coverage,
            "metalink": metalink.as_ref().map(|(version, file)| json!({
                "version": version,
                "file": file.name,
                "size": file.size,
                "torrents": file.torrents_by_priority().iter().map(|m| &m.url).collect::<Vec<_>>(),
                "mirrors_listed": file.mirrors.len(),
                "mirrors_unsupported": file.unsupported_mirrors,
                "checksum": file.best_checksum().map(|c| json!({
                    "algorithm": c.algorithm,
                    "expected": c.value,
                })),
            })),
        }));
    }

    let report = json!({
        "dry_run": true,
        "directory": directory.display().to_string(),
        "torrents": planned,
    });
    let _ = global;
    renderer.emit(env, "download", &report, || {
        let mut out = vec![
            field("dry run", "nothing will be written"),
            field("directory", directory.display()),
        ];
        for torrent in report["torrents"].as_array().into_iter().flatten() {
            out.push(String::new());
            out.push(field(
                "source",
                torrent["source"].as_str().unwrap_or_default(),
            ));
            if let Some(name) = torrent["name"].as_str() {
                out.push(field("name", name));
            }
            out.push(field(
                "web seeds",
                torrent["web_seeds"].as_array().map_or(0, Vec::len),
            ));
            out.push(field(
                "trackers",
                torrent["trackers"].as_array().map_or(0, Vec::len),
            ));
        }
        out
    })?;
    Ok(ExitCode::Success)
}

/// Run the `--on-complete` or `--on-error` hook, once for the whole run.
fn run_hooks(report: &DownloadReport, args: &DownloadArgs, renderer: &Renderer, env: &mut Env) {
    let hook = match report.failed {
        0 => args.limits.on_complete.as_deref(),
        _ => args.limits.on_error.as_deref(),
    };
    let Some(command) = hook else { return };
    let Some(first) = report.torrents.first() else {
        return;
    };
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(
        "BIT_CLI_VERSION".to_string(),
        bit_cli_core::VERSION.to_string(),
    );
    vars.insert("BIT_CLI_INFO_HASH".to_string(), first.info_hash.clone());
    vars.insert("BIT_CLI_NAME".to_string(), first.name.clone());
    vars.insert("BIT_CLI_DIR".to_string(), first.output_directory.clone());
    vars.insert(
        "BIT_CLI_TOTAL_BYTES".to_string(),
        report.total.0.to_string(),
    );
    vars.insert(
        "BIT_CLI_DOWNLOADED_BYTES".to_string(),
        report.downloaded.0.to_string(),
    );
    vars.insert(
        "BIT_CLI_COMPLETED".to_string(),
        report.completed.to_string(),
    );
    vars.insert("BIT_CLI_FAILED".to_string(), report.failed.to_string());
    vars.insert(
        "BIT_CLI_ELAPSED_MS".to_string(),
        report.elapsed_ms.to_string(),
    );

    match swarm::run_hook(command, &vars) {
        Ok(0) => {}
        Ok(code) => renderer.warn(env, format!("hook `{command}` exited {code}")),
        Err(error) => renderer.warn(env, format!("hook `{command}` failed: {error}")),
    }
}

fn lines(report: &DownloadReport) -> Vec<String> {
    let mut out = Vec::new();
    for torrent in &report.torrents {
        out.push(field("name", &torrent.name));
        out.push(field("info hash", &torrent.info_hash));
        out.push(field("stopped", torrent.stopped.as_str()));
        out.push(field(
            "downloaded",
            format!(
                "{} of {}",
                format_size(torrent.downloaded.0),
                format_size(torrent.total.0)
            ),
        ));
        out.push(field("from peers", format_size(torrent.from_peers.0)));
        out.push(field(
            "from web seeds",
            format_size(torrent.from_web_seeds.0),
        ));
        // Only when there were any, because a fresh download resumes nothing
        // and a line reading zero on every one of them is noise.
        if torrent.from_resume.0 > 0 {
            out.push(field("already on disk", format_size(torrent.from_resume.0)));
        }
        out.push(field("uploaded", format_size(torrent.uploaded.0)));
        out.push(field("elapsed", &torrent.elapsed_human));
        out.push(field("mean rate", &torrent.mean_rate_human));
        out.push(field("peers seen", torrent.peers_seen));
        // A run that finished only because it threw its peer state away three
        // times is not the same result as one that never stalled, and the
        // totals alone cannot tell them apart.
        if let Some(last) = torrent.redials.last() {
            out.push(field(
                "re-dialled",
                format!(
                    "{} time(s), last after {} of no progress",
                    torrent.redials.len(),
                    bit_cli_core::units::format_duration(Duration::from_millis(last.stalled_ms)),
                ),
            ));
        }
        out.push(field("written to", &torrent.output_directory));
        // A caller that does not know a file was renamed cannot find it, so
        // every rename is listed rather than counted.
        for rename in &torrent.renamed {
            out.push(field(
                &format!("renamed [{}]", rename.index),
                format!("{} -> {}", rename.torrent_path, rename.disk_path),
            ));
        }
        for file in &torrent.shared {
            out.push(field(
                &format!("shared [{}]", file.index),
                format!(
                    "{} read from {} ({} proven over {} piece(s))",
                    file.path,
                    file.from_path,
                    format_size(file.bytes_proven.0),
                    file.pieces_compared,
                ),
            ));
        }
        if let Some(error) = &torrent.error {
            out.push(field("error", error));
        }
        for source in &torrent.sources {
            out.push(String::new());
            out.push(field("source", &source.url));
            out.push(field("  scope", &source.scope));
            out.push(field(
                "  state",
                format!("{:?}", source.state).to_lowercase(),
            ));
            out.push(field("  served", &source.served_human));
            // Only when there were any. A retry line reading zero on every
            // healthy source is noise, and the absence of the line is the
            // same information.
            if source.retries > 0 {
                let by_status: Vec<String> = source
                    .retries_by_status
                    .iter()
                    .map(|(code, count)| format!("{count} on {code}"))
                    .collect();
                let detail = match by_status.is_empty() {
                    true => String::new(),
                    false => format!(" ({})", by_status.join(", ")),
                };
                out.push(field("  retries", format!("{}{detail}", source.retries)));
            }
            // Same rule: absent when the source never lost its connection,
            // which is the healthy case. When it is there it is the line that
            // says a run was waiting rather than working.
            if source.reconnects > 0 {
                let by_reason: Vec<String> = source
                    .reconnect_reasons
                    .iter()
                    .map(|(reason, count)| format!("{count} {reason}"))
                    .collect();
                out.push(field(
                    "  reconnects",
                    format!(
                        "{} in {} ({})",
                        source.reconnects,
                        bit_cli_core::units::format_duration(Duration::from_millis(
                            source.reconnect_wait_ms
                        )),
                        by_reason.join(", ")
                    ),
                ));
            }
            if source.cooldowns > 0 {
                let left = match source.cooldown_remaining_ms {
                    Some(ms) => format!(
                        ", {} left",
                        bit_cli_core::units::format_duration(Duration::from_millis(ms))
                    ),
                    None => String::new(),
                };
                out.push(field("  cooldowns", format!("{}{left}", source.cooldowns)));
            }
            if let Some(error) = &source.error {
                out.push(field("  error", error));
            }
        }
        out.push(String::new());
    }
    if report.torrents.len() > 1 {
        out.push(field("torrents", report.torrents.len()));
        out.push(field("completed", report.completed));
        out.push(field("failed", report.failed));
        out.push(field("downloaded", format_size(report.downloaded.0)));
        out.push(field("elapsed", &report.elapsed_human));
    }
    out.push(field("cost", report.process.summary()));
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::SelectionArgs;
    use crate::test_support::{TorrentFixture, run_json_code};

    /// Every selector value maps to one of two behaviours, and which is which
    /// is stated once.
    ///
    /// `sequential` and `in-order` are synonyms on purpose: one is the common
    /// name and the other is `aria2`'s. `default` is not a synonym for either,
    /// and the enum used to carry two more values that named behaviour nothing
    /// implemented. See `TODO/performance.md`, T-032.
    #[test]
    fn sequential_and_in_order_are_the_same_selector() {
        use crate::cli::PieceSelector;
        assert!(wants_in_order(PieceSelector::Sequential));
        assert!(wants_in_order(PieceSelector::InOrder));
        assert!(!wants_in_order(PieceSelector::Default));
        assert_eq!(PieceSelector::default(), PieceSelector::Default);
    }

    /// A torrent whose paths cannot be written as given still reports where
    /// its files went. Without this a caller cannot find what it downloaded.
    ///
    /// The run has no peers and no web seeds, so it stops on its deadline. The
    /// storage is created when the torrent is added, which is before any of
    /// that matters, so the mapping is there either way. See
    /// `TODO/windows.md` T-071 and T-072.
    #[test]
    fn a_hostile_torrent_reports_every_renamed_path_in_json() {
        let fixture = TorrentFixture::hostile();
        let out = fixture.dir().join("out");
        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed-only",
                "--web-seed",
                "http://127.0.0.1:9/",
                "--no-tracker",
                // An OS-chosen port, so two tests running at once cannot
                // race for the same one.
                "--port",
                "0",
                "--stop-after",
                "2s",
            ],
            fixture.dir(),
            ExitCode::Timeout,
        );

        let renamed = report["torrents"][0]["renamed"]
            .as_array()
            .expect("a renamed array")
            .clone();
        let pairs: Vec<(String, String)> = renamed
            .iter()
            .map(|entry| {
                (
                    entry["torrent_path"].as_str().unwrap().to_string(),
                    entry["disk_path"].as_str().unwrap().to_string(),
                )
            })
            .collect();

        assert_eq!(
            pairs,
            [
                ("C:/pwned.txt".to_string(), "C_/pwned.txt".to_string()),
                ("CON.txt".to_string(), "CON_.txt".to_string()),
                ("a<b.bin".to_string(), "a_b.bin".to_string()),
                ("x .".to_string(), "x".to_string()),
                ("readme".to_string(), "readme-1".to_string()),
            ]
        );
        // The index ties each entry back to the torrent's own file list, and
        // the reason says which rule applied.
        assert_eq!(renamed[0]["index"], 0);
        assert_eq!(renamed[0]["reasons"][0], "escape");
        assert_eq!(renamed[1]["reasons"][0], "reserved-name");
        assert_eq!(renamed[4]["reasons"][0], "case-collision");

        // Every file landed, including the two that collide only on a
        // case-insensitive filesystem.
        let mut landed: Vec<String> = walk(&out.join("hostile"));
        landed.sort();
        assert_eq!(
            landed,
            [
                "CON_.txt",
                "C_/pwned.txt",
                "README",
                "a_b.bin",
                "readme-1",
                "x"
            ]
        );
    }

    /// An ordinary torrent reports no renames at all, so a caller can test for
    /// an empty list rather than comparing every path.
    #[test]
    fn an_ordinary_torrent_reports_no_renames() {
        let fixture = TorrentFixture::multi_file();
        let out = fixture.dir().join("out");
        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed-only",
                "--web-seed",
                "http://127.0.0.1:9/",
                "--no-tracker",
                // An OS-chosen port, so two tests running at once cannot
                // race for the same one.
                "--port",
                "0",
                "--stop-after",
                "2s",
            ],
            fixture.dir(),
            ExitCode::Timeout,
        );
        assert!(report["torrents"][0].get("renamed").is_none());
    }

    /// Writing over an existing payload without permission is a disk failure,
    /// not a generic one.
    ///
    /// A caller branches on the exit code, and the fix here is a flag, so the
    /// code has to say "disk" and the message has to name the flag. See
    /// `TODO/disk-io.md`, T-014.
    #[test]
    fn a_download_over_an_existing_file_exits_eight_and_names_the_flag() {
        let fixture = TorrentFixture::multi_file();
        let out = fixture.dir().join("out");
        // The payload is already there, written by something else.
        let existing = out.join("album").join("notes.nfo");
        std::fs::create_dir_all(existing.parent().unwrap()).unwrap();
        std::fs::write(&existing, b"not the payload").unwrap();

        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed-only",
                "--web-seed",
                "http://127.0.0.1:9/",
                "--no-tracker",
                "--port",
                "0",
                // `--continue` defaults on and means "resume into what is
                // there", so it has to be off for this to be the refusal case.
                "--no-continue",
                "--stop-after",
                "5s",
            ],
            fixture.dir(),
            ExitCode::Disk,
        );
        let torrent = &report["torrents"][0];
        assert_eq!(torrent["code"], "disk", "the per-torrent code says why");
        let message = torrent["error"].as_str().unwrap_or_default();
        assert!(message.contains("already exists"), "{message}");
        assert!(message.contains("--allow-overwrite"), "{message}");
        // Nothing was written over.
        assert_eq!(std::fs::read(&existing).unwrap(), b"not the payload");
    }

    /// `--init-timeout` is a real value that reaches the wait.
    ///
    /// What the deadline does when it fires is asserted in
    /// `webseed_e2e::a_hash_check_that_has_not_finished_names_the_phase_it_is_in`,
    /// which needs a payload large enough that hashing it takes measurable
    /// time. This is the flag half: that it parses, that it defaults, and that
    /// a bad value is refused rather than ignored. See `TODO/disk-io.md`,
    /// T-015.
    #[test]
    fn the_initialisation_deadline_parses_and_a_bad_one_is_refused() {
        use crate::cli::{Cli, Command};
        use clap::Parser;

        let parse = |extra: &[&str]| {
            let mut args = vec!["bit-cli", "download", "a.torrent"];
            args.extend_from_slice(extra);
            let cli = Cli::try_parse_from(args).unwrap();
            let Some(Command::Download(args)) = cli.command else {
                panic!("expected download")
            };
            args.limits.init_timeout
        };
        assert_eq!(parse(&[]), "10m");
        assert_eq!(parse(&["--init-timeout", "45s"]), "45s");

        let fixture = TorrentFixture::multi_file();
        let error = crate::test_support::run_err(
            &[
                "download",
                fixture.path_str(),
                "--init-timeout",
                "not-a-duration",
            ],
            fixture.dir(),
            ExitCode::Usage,
        );
        assert!(error.contains("--init-timeout"), "{error}");
    }

    /// `--continue` is on by default and `--no-continue` turns it off.
    ///
    /// Before this, `--continue` defaulted to true with nothing to set it
    /// false, so the refusal above was unreachable from the command line and
    /// the flag could not do anything.
    #[test]
    fn continue_is_on_by_default_and_no_continue_turns_it_off() {
        use crate::cli::{Cli, Command};
        use clap::Parser;

        let parse = |extra: &[&str]| {
            let mut args = vec!["bit-cli", "download", "a.torrent"];
            args.extend_from_slice(extra);
            let cli = Cli::try_parse_from(args).unwrap();
            let Some(Command::Download(args)) = cli.command else {
                panic!("expected download")
            };
            args.no_continue
        };
        assert!(!parse(&[]), "resuming is the default");
        assert!(parse(&["--no-continue"]));
        assert!(!parse(&["--continue"]));
        // The later flag wins, so a script can append an override.
        assert!(!parse(&["--no-continue", "--continue"]));
        assert!(parse(&["--continue", "--no-continue"]));
    }

    /// A download reports what it cost.
    ///
    /// Measuring a process from outside means sampling one that has already
    /// exited, which reports zero, so the process is the only thing that can
    /// report its own high-water mark. `scripts/bench-webseed.ps1` reads these
    /// three fields.
    #[test]
    fn a_download_reports_its_own_peak_rss_cpu_and_handles() {
        let fixture = TorrentFixture::multi_file();
        let out = fixture.dir().join("out");
        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed-only",
                "--web-seed",
                "http://127.0.0.1:9/",
                "--no-tracker",
                // An OS-chosen port, so two tests running at once cannot
                // race for the same one.
                "--port",
                "0",
                "--stop-after",
                "1s",
            ],
            fixture.dir(),
            ExitCode::Timeout,
        );
        let process = &report["process"];
        assert!(
            process["peak_rss_bytes"].as_u64().unwrap() > 1024 * 1024,
            "peak RSS of {} is not a running process",
            process["peak_rss_bytes"]
        );
        assert!(process["open_handles"].as_u64().unwrap() > 0);
        assert_eq!(
            process["cpu_ms"].as_u64().unwrap(),
            process["cpu_user_ms"].as_u64().unwrap() + process["cpu_system_ms"].as_u64().unwrap()
        );
        assert!(
            process.get("unavailable").is_none(),
            "some field could not be read: {process}"
        );
    }

    fn walk(root: &std::path::Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(relative) = path.strip_prefix(root) {
                    out.push(
                        relative
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join("/"),
                    );
                }
            }
        }
        out
    }

    fn selection_args(select: &[&str], exclude: &[&str]) -> SelectionArgs {
        SelectionArgs {
            select_file: select.iter().map(ToString::to_string).collect(),
            exclude_file: exclude.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn no_selection_flags_means_every_file() {
        assert_eq!(selection(&selection_args(&[], &[])).unwrap(), None);
    }

    #[test]
    fn indices_and_ranges_both_select() {
        assert_eq!(
            selection(&selection_args(&["0"], &[])).unwrap(),
            Some(vec![0])
        );
        assert_eq!(
            selection(&selection_args(&["1-3"], &[])).unwrap(),
            Some(vec![1, 2, 3])
        );
        assert_eq!(
            selection(&selection_args(&["1-3", "7"], &[])).unwrap(),
            Some(vec![1, 2, 3, 7])
        );
    }

    #[test]
    fn an_exclusion_narrows_a_selection() {
        assert_eq!(
            selection(&selection_args(&["0-4"], &["2"])).unwrap(),
            Some(vec![0, 1, 3, 4])
        );
    }

    #[test]
    fn selecting_nothing_at_all_is_a_usage_error_rather_than_an_empty_download() {
        let err = selection(&selection_args(&["1-2"], &["1-2"])).unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
    }

    #[test]
    fn a_bad_index_names_the_flag_and_the_value() {
        let err = selection(&selection_args(&["two"], &[])).unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
        assert!(err.message().contains("select-file"), "{}", err.message());
        assert_eq!(err.context()["value"], "two");
    }

    #[test]
    fn a_backwards_range_is_refused() {
        let err = selection(&selection_args(&["5-2"], &[])).unwrap_err();
        assert!(err.message().contains("backwards"), "{}", err.message());
    }

    #[test]
    fn an_open_ended_range_says_why_it_cannot_be_resolved_yet() {
        let err = selection(&selection_args(&["2-"], &[])).unwrap_err();
        assert!(err.message().contains("file count"), "{}", err.message());
    }

    #[test]
    fn the_whole_run_request_budget_is_shared_across_sources() {
        let specs: Vec<SourceSpec> = (0..4)
            .map(|i| {
                SourceSpec::new(
                    format!("https://m{i}.example.com/"),
                    bit_cli_core::webseed::Origin::CommandLine,
                )
            })
            .collect();
        // Default per-source concurrency is 4, so four sources want 16.
        let limited = apply_max_total(&specs, Some(8));
        assert_eq!(limited.len(), 4);
        for spec in &limited {
            assert_eq!(spec.limits.concurrency, 2);
        }
    }

    #[test]
    fn a_budget_smaller_than_the_source_count_still_leaves_one_request_each() {
        let specs: Vec<SourceSpec> = (0..4)
            .map(|i| {
                SourceSpec::new(
                    format!("https://m{i}.example.com/"),
                    bit_cli_core::webseed::Origin::CommandLine,
                )
            })
            .collect();
        for spec in apply_max_total(&specs, Some(2)) {
            assert_eq!(
                spec.limits.concurrency, 1,
                "a source with no budget cannot serve"
            );
        }
    }

    #[test]
    fn no_budget_leaves_the_per_source_setting_alone() {
        let specs = vec![SourceSpec::new(
            "https://m.example.com/",
            bit_cli_core::webseed::Origin::CommandLine,
        )];
        assert_eq!(apply_max_total(&specs, None)[0].limits.concurrency, 4);
        assert_eq!(apply_max_total(&specs, Some(0))[0].limits.concurrency, 4);
    }

    #[test]
    fn the_window_cache_stays_inside_its_memory_budget() {
        let mut spec = SourceSpec::new(
            "https://m.example.com/",
            bit_cli_core::webseed::Origin::CommandLine,
        );
        spec.limits.chunk_size = 4 * bit_cli_core::units::MIB;
        assert_eq!(cache_windows(std::slice::from_ref(&spec)), 4);

        spec.limits.chunk_size = 64 * bit_cli_core::units::MIB;
        assert_eq!(
            cache_windows(std::slice::from_ref(&spec)),
            2,
            "never below two windows"
        );

        spec.limits.chunk_size = 64 * bit_cli_core::units::KIB;
        assert_eq!(
            cache_windows(std::slice::from_ref(&spec)),
            16,
            "and never above sixteen"
        );
    }

    /// A run that stalls with `--redial-after` set throws its peer state away
    /// and says so, rather than waiting out a backoff that grows by six.
    ///
    /// Nothing answers here, so every re-dial fires and none of them helps.
    /// That is the point: what is under test is that the flag reaches the
    /// watch loop, that the cap holds, and that the report carries both
    /// numbers T-138's acceptance asks for. Whether a re-dial recovers a real
    /// outage is `scripts/check-peer-recovery.ps1`, which measures it against
    /// a seeder that comes back. See `TODO/peers.md`, T-138.
    #[test]
    fn a_stalled_run_redials_up_to_the_cap_and_reports_each_one() {
        let fixture = TorrentFixture::single_file();
        let out = fixture.dir().join("out");
        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-tracker",
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "--report-interval",
                "200ms",
                "--redial-after",
                "500ms",
                "--max-redials",
                "2",
                "--stop-after",
                "4s",
            ],
            fixture.dir(),
            // Nothing serves the payload, so the run ends on its deadline.
            ExitCode::Timeout,
        );
        let redials = report["torrents"][0]["redials"]
            .as_array()
            .expect("a redials array");
        assert_eq!(redials.len(), 2, "--max-redials 2 is a cap: {redials:?}");
        assert_eq!(redials[0]["attempt"], 1);
        assert_eq!(redials[1]["attempt"], 2);
        for redial in redials {
            assert!(
                redial["stalled_ms"].as_u64().unwrap_or(0) >= 500,
                "a re-dial fired before --redial-after elapsed: {redial}"
            );
            assert!(redial["error"].is_null(), "a re-dial failed: {redial}");
        }
        // The second waits out the interval again rather than firing on the
        // next report tick.
        let first = redials[0]["at_ms"].as_u64().unwrap();
        let second = redials[1]["at_ms"].as_u64().unwrap();
        assert!(
            second >= first + 400,
            "re-dials {first}ms and {second}ms apart, under --redial-after"
        );
    }

    /// With the flag off, nothing re-dials and the report says nothing.
    #[test]
    fn a_stalled_run_without_the_flag_never_redials() {
        let fixture = TorrentFixture::single_file();
        let out = fixture.dir().join("out");
        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-tracker",
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "--report-interval",
                "200ms",
                "--stop-after",
                "2s",
            ],
            fixture.dir(),
            ExitCode::Timeout,
        );
        assert!(
            report["torrents"][0]["redials"].is_null(),
            "an empty array is not serialised: {}",
            report["torrents"][0]
        );
    }

    /// `-j 1` runs the sources in the order they were given.
    ///
    /// A torrent whose source is a file an earlier torrent writes needs the
    /// earlier one to have finished, which only holds if the order is the
    /// caller's rather than the scheduler's. Before the plans became a queue
    /// taken by a fixed pool, every plan was its own task queuing on a
    /// semaphore, and which task reached the semaphore first was up to the
    /// runtime. See `TODO/multi-source.md`, T-133.
    #[test]
    fn sources_start_in_the_order_they_were_given() {
        let first = TorrentFixture::single_file();
        let second = TorrentFixture::multi_file();
        let out = first.dir().join("out");

        let (mut env, captured) = crate::env::Env::test(
            &[
                "--jsonl",
                "download",
                first.path_str(),
                second.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-tracker",
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "-j",
                "1",
                "--report-interval",
                "200ms",
                "--stop-after",
                "1s",
            ],
            first.dir(),
        );
        let _ = crate::run(&mut env);
        let events = captured.jsonl().expect("stdout was not ndjson");
        let added: Vec<String> = events
            .iter()
            .filter(|event| event["type"] == "torrent_added")
            .filter_map(|event| event["info_hash"].as_str().map(str::to_string))
            .collect();
        assert_eq!(
            added,
            [first.info_hash.clone(), second.info_hash.clone()],
            "torrents started out of order: {added:?}"
        );
    }

    /// One torrent's report, by its name.
    ///
    /// The list is sorted by source path and two fixtures live in two
    /// temporary directories, so position says nothing about order.
    fn by_name<'a>(report: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
        report["torrents"]
            .as_array()
            .expect("a torrent array")
            .iter()
            .find(|torrent| torrent["name"] == name)
            .unwrap_or_else(|| panic!("no torrent named {name} in {report}"))
    }

    /// A file one torrent holds is read from it by the next, with no flag.
    ///
    /// The donor is complete on disk, so it finishes on its hash check with no
    /// source at all. The receiver has everything except the shared file and
    /// no source at all either, so the only way it can finish is by reading
    /// the donor's copy. See `TODO/multi-source.md`, T-140.
    #[test]
    fn a_proven_shared_file_is_read_from_the_torrent_that_holds_it() {
        let (donor, receiver) = TorrentFixture::sharing_pair();
        let out = donor.dir().join("out");
        donor.place(&out, &[]);
        receiver.place(&out, &["extra-b.txt"]);

        let report = run_json_code(
            &[
                "download",
                donor.path_str(),
                receiver.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-torrent-web-seed",
                "--no-tracker",
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "-j",
                "1",
                "--report-interval",
                "100ms",
                "--stop-after",
                "20s",
            ],
            donor.dir(),
            ExitCode::Success,
        );

        // Sorted by source path, and the two fixtures are in different temp
        // directories, so the order of the two reports is not the order they
        // ran in. Find it by name.
        let taken = by_name(&report, "receiver");
        assert_eq!(taken["finished"], true, "{taken}");
        let shared = taken["shared"].as_array().expect("a shared array");
        assert_eq!(shared.len(), 1, "{taken}");
        assert_eq!(shared[0]["path"], "shared.bin", "{taken}");
        assert_eq!(shared[0]["from_info_hash"], donor.info_hash, "{taken}");
        assert_eq!(shared[0]["from_index"], 1, "{taken}");
        // Four whole 1 KiB pieces lie entirely inside the 4 KiB file, and all
        // four hashes agree. Nothing is asserted by the caller.
        assert_eq!(shared[0]["pieces_compared"], 4, "{taken}");
        assert_eq!(shared[0]["bytes_proven"]["bytes"], 4096, "{taken}");

        // The whole shared file came from the donor's copy, and the rest was
        // already on disk. No peer served anything: there was no swarm.
        assert_eq!(taken["from_web_seeds"]["bytes"], 4096, "{taken}");
        assert_eq!(taken["from_resume"]["bytes"], 2048, "{taken}");
        assert_eq!(taken["from_peers"]["bytes"], 0, "{taken}");
        assert_eq!(taken["sources"][0]["origin"], "shared_file", "{taken}");
        assert_eq!(taken["sources"][0]["scope"], "file:1", "{taken}");

        // Same bytes in both output directories.
        let from_donor = std::fs::read(out.join("donor").join("shared.bin")).expect("donor file");
        let landed = std::fs::read(out.join("receiver").join("shared.bin")).expect("receiver file");
        assert_eq!(from_donor, landed);
    }

    /// `--no-share-files` turns it off, and then the same run cannot finish.
    ///
    /// A flag that does not move a number does not ship. The number here is
    /// the receiver's completion: with sharing on it finishes from the donor's
    /// copy, and with it off there is no source for the shared file at all.
    #[test]
    fn no_share_files_leaves_the_receiver_with_nothing_to_fetch_from() {
        let (donor, receiver) = TorrentFixture::sharing_pair();
        let out = donor.dir().join("out");
        donor.place(&out, &[]);
        receiver.place(&out, &["extra-b.txt"]);

        let report = run_json_code(
            &[
                "download",
                donor.path_str(),
                receiver.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-torrent-web-seed",
                "--no-share-files",
                "--no-tracker",
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "-j",
                "1",
                "--report-interval",
                "100ms",
                "--stop-after",
                "2s",
            ],
            donor.dir(),
            ExitCode::Timeout,
        );
        let taken = by_name(&report, "receiver");
        assert_eq!(taken["finished"], false, "{taken}");
        assert!(taken["shared"].is_null(), "{taken}");
        assert_eq!(taken["from_web_seeds"]["bytes"], 0, "{taken}");
    }

    /// Bytes that were already on the disk are not charged to peers.
    ///
    /// `progress_bytes` is everything the torrent has, not everything this run
    /// fetched, so `progress_bytes - served` charges a resumed download's
    /// existing bytes to the swarm. This run has no peers and no sources at
    /// all, and the payload is already complete, so anything non-zero in
    /// `from_peers` is that arithmetic and nothing else. See
    /// `TODO/multi-source.md`, T-139.
    #[test]
    fn bytes_already_on_disk_are_reported_as_resumed_rather_than_from_peers() {
        let fixture = TorrentFixture::single_file();
        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                fixture.payload_dir().to_str().unwrap(),
                "--no-tracker",
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "--allow-overwrite",
                "--stop-after",
                "20s",
            ],
            fixture.dir(),
            // The payload is already there, so the hash check finds it
            // complete and the run finishes at once.
            ExitCode::Success,
        );
        let torrent = &report["torrents"][0];
        assert_eq!(torrent["downloaded"]["bytes"], 3000);
        assert_eq!(torrent["from_resume"]["bytes"], 3000, "{torrent}");
        assert_eq!(torrent["from_peers"]["bytes"], 0, "{torrent}");
        assert_eq!(torrent["from_web_seeds"]["bytes"], 0, "{torrent}");
        assert_eq!(report["from_resume"]["bytes"], 3000);
    }

    /// A whole run tells its trackers when it started, when it finished, and
    /// when it stopped.
    ///
    /// The session sends `started` and then repeats on the interval. It never
    /// says a download completed, so a tracker's seeder count is wrong, and it
    /// never says stopped, so a dead address is handed out until the record
    /// expires. Both are sent by `bit-cli` itself, from the session's own peer
    /// id and port, so the tracker updates one record rather than seeing two
    /// peers. See `TODO/trackers.md`, T-062.
    ///
    /// The payload is fetched rather than already on disk. A torrent that is
    /// complete on its hash check finishes before the session's own `started`
    /// announce has left, and the order the tracker sees is then a race rather
    /// than a sequence.
    /// `TODO/cli-surface.md` T-183. `--web-seed-list-url` is fetched over
    /// loopback HTTP and the sources it names are used.
    ///
    /// The flag parsed and was read, and every call site handed the reader a
    /// function that refuses, so it could only ever fail. That is why the flag
    /// audit that found T-181 missed it: it looked for a field nothing reads,
    /// and this one is read.
    #[test]
    fn a_web_seed_list_url_is_fetched_and_its_sources_are_used() {
        let fixture = TorrentFixture::multi_file();
        let server = crate::test_support::FileServer::start(fixture.dir());
        std::fs::write(
            fixture.dir().join("mirrors.txt"),
            format!(
                "# the mirror list
{}payload/
",
                server.base
            ),
        )
        .unwrap();

        let out = fixture.dir().join("out");
        let list_url = format!("{}mirrors.txt", server.base);

        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-torrent-web-seed",
                "--web-seed-list-url",
                &list_url,
                "--web-seed-mode",
                "prefix",
                "--no-dht",
                "--no-lsd",
                "--no-tracker",
                "--port",
                "0",
                "--report-interval",
                "100ms",
                "--stop-after",
                "20s",
            ],
            fixture.dir(),
            ExitCode::Success,
        );

        let torrent = &report["torrents"][0];
        assert_eq!(torrent["finished"], true, "{report}");
        let sources = torrent["sources"].as_array().expect("a sources array");
        assert_eq!(sources.len(), 1, "{report}");
        assert_eq!(sources[0]["origin"], "list_url", "{report}");
        assert_eq!(
            sources[0]["served_bytes"], 2000,
            "the fetched source has to have served the whole payload: {report}"
        );
    }

    /// `TODO/cli-surface.md` T-181. `--tracker-list-url` is fetched over
    /// loopback HTTP and every tracker it names is announced to.
    ///
    /// Three trackers rather than one, because the failure this guards against
    /// is a list that is read and then partly dropped, and one tracker cannot
    /// tell a whole list from the first line of one. Each tracker records what
    /// it was asked, so the proof is on the tracker's side rather than in a
    /// count the run reports about itself.
    #[test]
    fn a_tracker_list_url_is_fetched_and_every_tracker_in_it_is_announced_to() {
        let fixture = TorrentFixture::multi_file();
        let server = crate::test_support::FileServer::start(fixture.dir());
        let trackers = [
            crate::test_support::Tracker::start(&[]),
            crate::test_support::Tracker::start(&[]),
            crate::test_support::Tracker::start(&[]),
        ];
        std::fs::write(
            fixture.dir().join("trackers.txt"),
            format!(
                "# the mirror list
{}

{}
{}
",
                trackers[0].announce, trackers[1].announce, trackers[2].announce
            ),
        )
        .unwrap();

        let out = fixture.dir().join("out");
        let source = format!("{}payload/", server.base);
        let list_url = format!("{}trackers.txt", server.base);

        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-torrent-web-seed",
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
                "--replace-trackers",
                "--tracker-list-url",
                &list_url,
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "--report-interval",
                "100ms",
                "--stop-after",
                "20s",
            ],
            fixture.dir(),
            ExitCode::Success,
        );

        for (index, tracker) in trackers.iter().enumerate() {
            assert!(
                !tracker.seen().is_empty(),
                "tracker {index} was never announced to, so the fetched list did not reach the session: {report}"
            );
        }

        let announced = report["torrents"][0]["announced"]
            .as_array()
            .expect("an announced array");
        assert!(
            announced.iter().any(|sent| sent["trackers"] == 3),
            "the report has to say three trackers were announced to: {report}"
        );
    }

    #[test]
    fn a_run_announces_started_then_completed_then_stopped() {
        let fixture = TorrentFixture::multi_file();
        let server = crate::test_support::FileServer::start(fixture.dir());
        let tracker = crate::test_support::Tracker::start(&[]);
        let out = fixture.dir().join("out");
        let source = format!("{}payload/", server.base);

        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-torrent-web-seed",
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
                "--replace-trackers",
                "--tracker",
                &tracker.announce,
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "--report-interval",
                "100ms",
                "--stop-after",
                "20s",
            ],
            fixture.dir(),
            ExitCode::Success,
        );

        let announced = report["torrents"][0]["announced"]
            .as_array()
            .expect("an announced array");
        let events: Vec<&str> = announced
            .iter()
            .filter_map(|sent| sent["event"].as_str())
            .collect();
        assert_eq!(events, ["completed", "stopped"], "{report}");
        for sent in announced {
            assert_eq!(sent["trackers"], 1, "{sent}");
            assert_eq!(sent["accepted"], 1, "{sent}");
        }

        // What the tracker actually saw, in order. `started` is the session's
        // own; the other two are this run's.
        assert_eq!(
            tracker.param("event"),
            ["started", "completed", "stopped"],
            "{:?}",
            tracker.seen()
        );

        // One peer id and one port throughout, which is what makes these
        // updates to the session's record rather than a second peer.
        let ids: std::collections::HashSet<String> = tracker.param("peer_id").into_iter().collect();
        assert_eq!(ids.len(), 1, "{:?}", tracker.seen());
        let ports: std::collections::HashSet<String> = tracker.param("port").into_iter().collect();
        assert_eq!(ports.len(), 1, "{:?}", tracker.seen());
    }

    /// A payload path past the classic Windows limit lands and verifies.
    ///
    /// The download directory plus this torrent's deepest path is over 300
    /// characters, which is past the 260 the `MAX_PATH` era allows. Nothing
    /// here adds an extended-length prefix: it is a test of whether the tool
    /// needs one. See `TODO/windows.md`, T-073.
    #[test]
    fn a_path_past_the_classic_windows_limit_lands_and_verifies() {
        let fixture = TorrentFixture::deep();
        let server = crate::test_support::FileServer::start(fixture.dir());
        let out = fixture.dir().join("out");
        let source = format!("{}payload/", server.base);

        let landed = out.join("deep").join(&fixture.files[0].0);
        assert!(
            landed.to_string_lossy().chars().count() > 300,
            "the fixture is not long enough to test anything: {}",
            landed.display()
        );

        let report = run_json_code(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--no-torrent-web-seed",
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
                "--web-seed-only",
                "--port",
                "0",
                "--report-interval",
                "100ms",
                "--stop-after",
                "20s",
            ],
            fixture.dir(),
            ExitCode::Success,
        );
        assert_eq!(report["torrents"][0]["finished"], true, "{report}");
        assert!(
            report["torrents"][0]["renamed"].is_null(),
            "a long path was rewritten rather than written: {report}"
        );
        assert_eq!(
            std::fs::read(&landed).expect("the payload is not where it was planned"),
            fixture.files[0].1
        );

        // And the hash check reads it back from the same path.
        let verified = run_json_code(
            &["verify", fixture.path_str(), "--dir", out.to_str().unwrap()],
            fixture.dir(),
            ExitCode::Success,
        );
        assert_eq!(verified["complete"], true, "{verified}");
    }
}
