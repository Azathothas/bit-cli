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
use bit_cli_core::paths::Rename;
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{Size, format_rate, format_size};
use bit_cli_core::webseed::binding::SourceSpec;
use bit_cli_core::webseed::fetch::Verify;
use serde::Serialize;
use serde_json::json;
use tokio::sync::mpsc;

use crate::cli::{DownloadArgs, Global};
use crate::env::Env;
use crate::output::{Renderer, field};
use crate::source::Kind;
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
    pub elapsed_ms: u64,
    pub elapsed_human: String,
    pub mean_rate: Size,
    pub mean_rate_human: String,
    pub peers_seen: u32,
    pub sources: Vec<SourceReport>,
    pub output_directory: String,
    /// Files whose on-disk path is not the path in the torrent, and why.
    /// Empty for the ordinary torrent. See `bit_cli_core::paths`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub renamed: Vec<Rename>,
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

/// What the whole run reports.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadReport {
    pub torrents: Vec<TorrentReport>,
    pub total: Size,
    pub downloaded: Size,
    pub from_web_seeds: Size,
    pub from_peers: Size,
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
    let directory = engine_options.download_directory.clone();

    if global.dry_run {
        return dry_run(args, global, &setup, renderer, env, &directory);
    }

    // Every source is classified before the session starts, so a typo in the
    // fifth argument fails before the first byte is fetched.
    let mut plans = Vec::with_capacity(args.sources.len());
    for source in &args.sources {
        let kind = Kind::classify(source, env)?;
        let meta = match &kind {
            Kind::File(path) => Some(Metainfo::read(path)?),
            _ => None,
        };
        let specs = webseed_args::collect(
            &args.web_seeds,
            meta.as_ref(),
            env,
            webseed_args::no_network,
        )?;
        let trackers = setup.tracker_list(meta.as_ref(), env)?;
        plans.push(Plan {
            source: source.clone(),
            specs,
            trackers,
        });
    }

    let init_timeout = swarm::duration_flag(&args.limits.init_timeout, "init-timeout")?;
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
    let concurrency = args.max_concurrent_downloads.max(1);
    let started = std::time::Instant::now();
    let runtime = swarm::runtime()?;

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
        let permits = Arc::new(tokio::sync::Semaphore::new(concurrency));
        let mut workers = tokio::task::JoinSet::new();
        for plan in plans {
            let engine = engine.clone();
            let tx = tx.clone();
            let permits = permits.clone();
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
            };
            workers.spawn(async move {
                let _permit = permits.acquire().await;
                let report = one(&engine, plan, options, &tx).await;
                let _ = tx.send(Msg::Done(Box::new(report))).await;
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
    /// Peers to dial before any are discovered, from `--peer`.
    peers: Vec<std::net::SocketAddr>,
}

/// One source and what was resolved for it before the session started.
struct Plan {
    source: String,
    specs: Vec<SourceSpec>,
    trackers: Option<Vec<String>>,
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
                elapsed_ms: 0,
                elapsed_human: "0s".into(),
                mean_rate: Size(0),
                mean_rate_human: format_rate(0),
                peers_seen: 0,
                sources: Vec::new(),
                output_directory: options.directory.display().to_string(),
                renamed: Vec::new(),
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
        ..Default::default()
    };
    let handle = engine.add(&plan.source, &add).await?;
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

    if options.hash_check_only {
        let snapshot = engine.snapshot(&handle);
        return Ok(finish(
            plan,
            options,
            &snapshot,
            &[],
            Stopped::Completed,
            Duration::ZERO,
            renames(engine, &handle),
        ));
    }

    // The whole-run concurrency cap is shared out across the declared sources,
    // so `--web-seed-max-total 8` with four mirrors means two requests each
    // rather than eight each.
    let specs = apply_preference(
        apply_max_total(&plan.specs, options.max_total),
        options.prefer,
    );
    let (sources, _set) = swarm::attach_sources(
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

    let outcome = watch(engine, &handle, &layout, &sources, options, tx).await;
    for source in &sources {
        source.stop();
    }
    let (stopped, elapsed) = outcome;
    let snapshot = engine.snapshot(&handle);
    let report = finish(
        plan,
        options,
        &snapshot,
        &sources,
        stopped,
        elapsed,
        renames(engine, &handle),
    );

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
async fn watch(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    layout: &Layout,
    sources: &[AttachedSource],
    options: &Options,
    tx: &mpsc::Sender<Msg>,
) -> (Stopped, Duration) {
    let lengths: Vec<u64> = layout.files.iter().map(|f| f.length).collect();
    let mut progress = Progress::new(layout.piece_count(), lengths);
    let mut ticker = tokio::time::interval(options.report_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut reported_failures = HashSet::new();
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

    loop {
        tokio::select! {
            _ = &mut interrupt => return (Stopped::Interrupted, progress.elapsed()),
            _ = ticker.tick() => {}
            _ = &mut completion, if !completed => completed = true,
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
            return (Stopped::Failed, progress.elapsed());
        }

        let seeding = options.stop.seed_ratio.is_some() || options.stop.seed_time.is_some();
        if let Some(reason) = progress.should_stop(&snapshot, &options.stop, seeding) {
            return (reason, progress.elapsed());
        }
    }
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

fn finish(
    plan: &Plan,
    options: &Options,
    snapshot: &TorrentSnapshot,
    sources: &[AttachedSource],
    stopped: Stopped,
    elapsed: Duration,
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
        from_peers: Size(snapshot.progress_bytes.saturating_sub(served)),
        elapsed_ms,
        elapsed_human: bit_cli_core::units::format_duration(elapsed),
        mean_rate: Size(mean),
        mean_rate_human: format_rate(mean),
        peers_seen: snapshot.peers.seen,
        sources: sources.iter().map(AttachedSource::report).collect(),
        output_directory: options.directory.display().to_string(),
        renamed,
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
        let specs = webseed_args::collect(
            &args.web_seeds,
            meta.as_ref(),
            env,
            webseed_args::no_network,
        )?;
        let trackers = setup.tracker_list(meta.as_ref(), env)?.unwrap_or_default();
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
        out.push(field("uploaded", format_size(torrent.uploaded.0)));
        out.push(field("elapsed", &torrent.elapsed_human));
        out.push(field("mean rate", &torrent.mean_rate_human));
        out.push(field("peers seen", torrent.peers_seen));
        out.push(field("written to", &torrent.output_directory));
        // A caller that does not know a file was renamed cannot find it, so
        // every rename is listed rather than counted.
        for rename in &torrent.renamed {
            out.push(field(
                &format!("renamed [{}]", rename.index),
                format!("{} -> {}", rename.torrent_path, rename.disk_path),
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
}
