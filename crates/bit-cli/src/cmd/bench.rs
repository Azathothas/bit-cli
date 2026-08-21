//! `bit-cli bench`: measure a target and write a report.
//!
//! Every subcommand fills the same envelope, so a caller parses one shape and
//! `--baseline` compares any run against any earlier run of the same kind. The
//! envelope carries the machine, the exact command line, and what the process
//! cost, because a number without those is not a result.
//!
//! Where the report goes:
//!
//! - By default it is written to stdout in `--format`, which defaults to
//!   `json`. `--json` and `--jsonl` set the format to `json` and `ndjson`, so
//!   `bench` reads the same as every other subcommand.
//! - `--report <PATH>` writes it to that file instead, and stdout carries the
//!   text summary so a CI log shows something. `--report -` is stdout.
//!
//! Nothing is display-only. The text summary is a rendering of the same report
//! the JSON carries, never a source of its own.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bit_cli_core::ExitCode;
use bit_cli_core::bench::render::{self, Format};
use bit_cli_core::bench::report::{Build, Environment, Kind, Parameters, Report, Target};
use bit_cli_core::bench::{disk as bench_disk, recorder, webseed as bench_webseed};
use bit_cli_core::engine::Engine;
use bit_cli_core::error::{Error, Result};
use bit_cli_core::layout::Layout;
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{Millis, Size, parse_duration, parse_rate, parse_size};
use bit_cli_core::webseed::binding::BindingSet;

use crate::cli::{
    BenchArgs, BenchCommand, BenchShared, BenchWebseedArgs, Global, ReportArgs, ReportFormat,
};
use crate::env::Env;
use crate::output::Renderer;
use crate::source::{Kind as SourceKind, load_local};
use crate::swarm::{self, AttachedSource, SessionSetup};
use crate::webseed_args;

/// The triple this binary was built for.
const TARGET: &str = env!("BIT_CLI_TARGET");

/// Run a `bench` subcommand.
pub fn run(
    command: &BenchCommand,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    match command {
        BenchCommand::Webseed(args) => webseed(args, global, renderer, env),
        BenchCommand::Leech(args) => leech(args, global, renderer, env),
        BenchCommand::Disk(args) => disk(args, global, renderer, env),
        BenchCommand::Seed(args) => seed(args, global, renderer, env),
        BenchCommand::Swarm(args) => unbuilt(Kind::Swarm, args),
        BenchCommand::Probe(args) => probe(args, global, renderer, env),
    }
}

/// `bit-cli bench probe`: one exchange with one target.
///
/// This is the question that comes before "how fast": is the thing there, and
/// what does it speak. It moves no payload and runs for one exchange, so the
/// report carries the environment and the facts and no time series.
///
/// See `TODO/bench.md`, T-090, step 5.
fn probe(
    args: &crate::cli::BenchProbeArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    use bit_cli_core::bench::probe as engine;

    let output = Output::resolve(&args.report, global, env)?;
    let timeout = parse_duration(&args.timeout)
        .map_err(|e| Error::usage(format!("--timeout: {e}")).with("value", args.timeout.clone()))?;
    let target = engine::classify(&args.target)?;

    let mut report = Report::new(
        Kind::Probe,
        Environment::begin(
            build(),
            env.args.clone(),
            env.cwd.display().to_string(),
            global.trace.clone(),
        ),
    );
    report.parameters.duration = Millis::from(timeout);
    report.target = Target {
        source: args.target.clone(),
        endpoints: vec![args.target.clone()],
        ..Default::default()
    };

    // A handshake names a torrent. Without one the probe still reaches the
    // handshake and usually no further, which is worth saying in the report
    // rather than leaving as a mystery in the peer's own logs.
    let mut info_hash = [0u8; 20];
    if let Some(source) = &args.source {
        let kind = SourceKind::classify(source, env)?;
        let resolved = match &kind {
            SourceKind::File(path) => Some(Metainfo::read(path)?.info_hash()),
            _ => kind.info_hash(),
        };
        match resolved {
            Some(hash) => {
                info_hash = hash.0;
                report.target.info_hash = Some(hash.hex());
            }
            None => {
                return Err(Error::source_resolution(format!(
                    "{source}: --for needs a torrent, a magnet, or an info hash, and this carries none"
                ))
                .with("value", source.clone()));
            }
        }
    } else if matches!(target, engine::Probe::Peer(_)) {
        report.note(
            "no --for, so the handshake names a zero info hash: a peer is entitled to hang up on it",
        );
    }

    if global.dry_run {
        report.note("dry run: nothing was contacted");
        report.environment.finish();
        return emit(&report, &output, renderer, env, ExitCode::Success);
    }

    let runtime = crate::swarm::runtime()?;
    let found = runtime.block_on(engine::run(
        &target,
        &args.target,
        info_hash,
        peer_id(),
        timeout,
    ));

    if let Some(error) = &found.error {
        renderer.warn(env, format!("{}: {error}", args.target));
    }
    report.summary.duration = found.elapsed;
    report.summary.requests = 1;
    if !found.reachable {
        report.summary.errors.total = 1;
    }
    let reachable = found.reachable;
    report.probe = Some(found);
    report.environment.finish();

    // Exit 6 when the target did not answer. A probe that reports a
    // failure is doing its job, and a script needs the code to branch on.
    let code = match reachable {
        true => ExitCode::Success,
        false => ExitCode::NoUsableSources,
    };
    emit(&report, &output, renderer, env, code)
}

/// A peer id for one probe.
///
/// `-BC` then four version digits, per BEP 20, then twelve bytes that differ
/// between runs. A probe that reused one id would look to a tracker-backed
/// peer like the same client reconnecting.
fn peer_id() -> [u8; 20] {
    let mut out = *b"-BC0100-000000000000";
    let mut state = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x2026_0820)
        | 1;
    for slot in out[8..].iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *slot = b"0123456789abcdefghijklmnopqrstuvwxyz"[(state >> 24) as usize % 36];
    }
    out
}

/// A subcommand that is not built yet.
///
/// It fails loudly with the `TODO/` entry that closes it rather than
/// pretending to work, and it names the one that is built, because a caller
/// pointed at the wrong subcommand should be told which one to use.
fn unbuilt(kind: Kind, _args: &BenchArgs) -> Result<ExitCode> {
    Err(Error::generic(format!(
        "`bit-cli bench {}` is not implemented yet; see TODO/bench.md",
        kind.as_str()
    ))
    .with("todo", "T-090")
    .with("subcommand", kind.as_str())
    .with(
        "hint",
        "`bit-cli bench webseed` measures HTTP sources today",
    ))
}

/// The build metadata every report carries.
fn build() -> Build {
    Build {
        version: bit_cli_core::VERSION.to_string(),
        target: TARGET.to_string(),
        // `debug_assertions` is the only reliable signal of which profile this
        // binary came out of: `PROFILE` in a build script describes the build
        // script, not the crate.
        profile: match cfg!(debug_assertions) {
            true => "debug".to_string(),
            false => "release".to_string(),
        },
        debug_assertions: cfg!(debug_assertions),
    }
}

/// Everything `--format`, `--report`, `--baseline`, and `--fail-under` mean,
/// resolved once.
struct Output {
    format: Format,
    /// Where the report goes. `None` is stdout.
    path: Option<PathBuf>,
    baseline: Option<PathBuf>,
    fail_under: Option<u64>,
}

impl Output {
    fn resolve(args: &ReportArgs, global: &Global, env: &Env) -> Result<Self> {
        // `--json` and `--jsonl` are the global way to ask for a machine
        // surface, so they set the report format rather than sitting beside
        // it. An explicit `--format` still wins over the default.
        let format = match (global.json, global.jsonl, args.format) {
            (_, true, _) => Format::Ndjson,
            (true, _, ReportFormat::Json) => Format::Json,
            (true, _, other) => format_of(other),
            _ => format_of(args.format),
        };
        let path = match args.report.as_deref() {
            None | Some("-") => None,
            Some(path) => Some(env.resolve(std::path::Path::new(path))),
        };
        Ok(Self {
            format,
            path,
            baseline: args.baseline.as_ref().map(|p| env.resolve(p)),
            fail_under: args
                .fail_under
                .as_deref()
                .map(|rate| {
                    parse_rate(rate).map_err(|e| {
                        Error::usage(format!("--fail-under: {e}")).with("value", rate.to_string())
                    })
                })
                .transpose()?,
        })
    }

    /// Whether the report itself goes to stdout.
    fn to_stdout(&self) -> bool {
        self.path.is_none()
    }
}

fn format_of(format: ReportFormat) -> Format {
    match format {
        ReportFormat::Json => Format::Json,
        ReportFormat::Ndjson => Format::Ndjson,
        ReportFormat::Csv => Format::Csv,
        ReportFormat::Text => Format::Text,
    }
}

/// The shared flags, parsed into the report's `parameters` object.
fn parameters(shared: &BenchShared) -> Result<Parameters> {
    let run_for = duration(&shared.duration, "duration")?;
    let warmup = duration_or_zero(&shared.warmup, "warmup")?;
    let interval = duration(&shared.metrics_interval, "metrics-interval")?;
    Ok(Parameters {
        duration: Millis::from(run_for),
        warmup: Millis::from(warmup),
        metrics_interval: Millis::from(interval),
        concurrency: shared.concurrency.max(1),
        concurrency_sweep: sweep(shared.concurrency_sweep.as_deref())?,
        target_rate: rate(shared.target_rate.as_deref(), "target-rate")?
            .map(bit_cli_core::units::Rate),
        fail_under: rate(shared.report.fail_under.as_deref(), "fail-under")?.map(Size),
        ceiling: rate(shared.ceiling.as_deref(), "ceiling")?.map(Size),
        ..Default::default()
    })
}

fn duration(value: &str, flag: &str) -> Result<Duration> {
    let parsed = parse_duration(value)
        .map_err(|e| Error::usage(format!("--{flag}: {e}")).with("value", value.to_string()))?;
    if parsed.is_zero() {
        return Err(
            Error::usage(format!("--{flag} cannot be zero")).with("value", value.to_string())
        );
    }
    Ok(parsed)
}

fn duration_or_zero(value: &str, flag: &str) -> Result<Duration> {
    parse_duration(value)
        .map_err(|e| Error::usage(format!("--{flag}: {e}")).with("value", value.to_string()))
}

fn rate(value: Option<&str>, flag: &str) -> Result<Option<u64>> {
    value
        .map(|text| {
            parse_rate(text)
                .map_err(|e| Error::usage(format!("--{flag}: {e}")).with("value", text.to_string()))
        })
        .transpose()
}

fn size(value: Option<&str>, flag: &str) -> Result<Option<u64>> {
    value
        .map(|text| {
            parse_size(text)
                .map_err(|e| Error::usage(format!("--{flag}: {e}")).with("value", text.to_string()))
        })
        .transpose()
}

/// Parse `--concurrency-sweep`, for example `1,2,4,8,16`.
fn sweep(spec: Option<&str>) -> Result<Vec<usize>> {
    let Some(spec) = spec else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for term in spec.split(',') {
        let term = term.trim();
        if term.is_empty() {
            continue;
        }
        let value: usize = term.parse().map_err(|_| {
            Error::usage(format!("--concurrency-sweep `{term}` is not a number"))
                .with("value", term.to_string())
        })?;
        if value == 0 {
            return Err(Error::usage("--concurrency-sweep cannot include zero"));
        }
        out.push(value);
    }
    if out.is_empty() {
        return Err(Error::usage(
            "--concurrency-sweep needs at least one concurrency",
        ));
    }
    Ok(out)
}

/// `bit-cli bench webseed`: measure HTTP sources.
pub fn webseed(
    args: &BenchWebseedArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let output = Output::resolve(&args.shared.report, global, env)?;
    let parameters = parameters(&args.shared)?;
    let request_size = size(args.shared.request_size.as_deref(), "request-size")?;

    let (meta, layout, bindings) = resolve(&args.source.source, &args.web_seeds, env)?;
    let mut report = Report::new(
        Kind::Webseed,
        Environment::begin(
            build(),
            env.args.clone(),
            env.cwd.display().to_string(),
            global.trace.clone(),
        ),
    );
    report.parameters = parameters.clone();
    report.target = Target {
        source: args.source.source.clone(),
        info_hash: Some(meta.info_hash().hex()),
        name: Some(layout.name.clone()),
        total: Some(Size(layout.total_length)),
        piece_length: Some(Size(u64::from(layout.piece_length))),
        piece_count: Some(layout.piece_count()),
        endpoints: Vec::new(),
    };
    if report.environment.build.debug_assertions {
        report.note("this is a debug build: the numbers describe a debug build and nothing else");
    }
    if parameters.warmup.0 >= parameters.duration.0 {
        report.note(format!(
            "the warmup of {} is not shorter than the run of {}, so nothing is measured",
            parameters.warmup, parameters.duration
        ));
    }

    // A dry run resolves the bindings, describes the target, and stops. It
    // still reads `--baseline` and still writes a report, because "would this
    // even run" is the question it answers and half an answer is no answer.
    if global.dry_run {
        report.note("dry run: no request was made");
        report.target.endpoints = bindings
            .bindings
            .iter()
            .map(|binding| binding.spec.url.clone())
            .collect();
    } else {
        let options = bench_webseed::Options {
            duration: Duration::from_millis(parameters.duration.0),
            warmup: Duration::from_millis(parameters.warmup.0),
            metrics_interval: Duration::from_millis(parameters.metrics_interval.0),
            concurrency: parameters.concurrency,
            concurrency_sweep: parameters.concurrency_sweep.clone(),
            target_rate: parameters.target_rate.map(|rate| rate.0),
            chunk_size: request_size,
        };

        let runtime = crate::swarm::runtime()?;
        let info_hash = meta.info_hash().hex();
        // Samples are collected rather than emitted from inside the runtime,
        // because the streams belong to the calling thread and a worker
        // writing to them would interleave with the report.
        let mut samples = Vec::new();
        let outcome = runtime.block_on(async {
            bench_webseed::run(&bindings, &layout, &info_hash, &options, |sample| {
                samples.push(sample.clone())
            })
            .await
        })?;

        for sample in &samples {
            renderer.event(env, "bench_sample", sample)?;
        }
        for note in &outcome.notes {
            renderer.warn(env, note);
            report.note(note.clone());
        }

        report.series = outcome.series;
        report.concurrency_curve = outcome.concurrency_curve;
        report.summary = outcome.summary;
        report.target.endpoints = outcome.endpoints;
        report.sources = outcome
            .sources
            .iter()
            .map(|source| {
                let mut summary = source.summary.clone();
                summary.failure = source.failure.clone();
                summary
            })
            .collect();
        for source in &outcome.sources {
            if source.range_support == bit_cli_core::webseed::probe::RangeSupport::No {
                report.note(format!(
                    "{} does not honour Range: a download cannot use it",
                    source.summary.label
                ));
            }
        }
        for sample in &report.series {
            report.environment.observe(&sample.process);
        }
        if let Some(ceiling) = parameters.ceiling {
            report.summary.ceiling_share = report.summary.share_of(ceiling.0);
        }
    }

    // A dry run has no measurement, so it gets no verdict. Failing a threshold
    // against a run that never made a request would be a false negative in
    // exactly the place CI reads.
    let met = match global.dry_run {
        true => {
            if output.fail_under.is_some() {
                report.note("--fail-under was not applied: a dry run measures nothing");
            }
            true
        }
        false => report.apply_threshold(output.fail_under),
    };
    compare_against_baseline(&mut report, &output, renderer, env)?;
    report.environment.finish();

    let code = match (global.dry_run, met, report.summary.bytes.0) {
        (true, _, _) => ExitCode::Success,
        (_, false, _) => ExitCode::ThresholdNotMet,
        // Every source answering nothing is not a slow server, it is no
        // server, and a caller has to be able to tell those apart.
        (_, _, 0) => ExitCode::NoUsableSources,
        _ => ExitCode::Success,
    };
    emit(&report, &output, renderer, env, code)
}

/// `bit-cli bench disk`: what the payload file costs under several writers.
///
/// No torrent, no session, no network. The same
/// [`bit_cli_core::storage::SafeStorage`] a download writes through, driven
/// from N threads, so the disk can be measured on its own rather than inferred
/// from a download that is doing three things at once. See `TODO/disk-io.md`,
/// T-017.
pub fn disk(
    args: &crate::cli::BenchDiskArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let output = Output::resolve(&args.report, global, env)?;
    let payload_size = size(Some(&args.payload_size), "payload-size")?.unwrap_or(0);
    let block_size = size(Some(&args.block_size), "block-size")?.unwrap_or(0);
    if payload_size == 0 {
        return Err(Error::usage("--payload-size cannot be zero"));
    }
    if block_size == 0 {
        return Err(Error::usage("--block-size cannot be zero"));
    }
    if block_size > payload_size {
        return Err(Error::usage(
            "--block-size is larger than --payload-size, so nothing would be written",
        )
        .with("block_size", args.block_size.clone())
        .with("payload_size", args.payload_size.clone()));
    }
    let options = bench_disk::Options {
        payload_size,
        block_size,
        threads: args.concurrency.max(1),
        sweep: sweep(args.concurrency_sweep.as_deref())?,
        layout: args.layout.into(),
        allocation: super::download::allocation_of(args.file_allocation),
        max_open_files: args.max_open_files,
        duration: duration(&args.duration, "duration")?,
        metrics_interval: duration(&args.metrics_interval, "metrics-interval")?,
        verify: !args.no_verify,
    };

    let mut report = Report::new(
        Kind::Disk,
        Environment::begin(
            build(),
            env.args.clone(),
            env.cwd.display().to_string(),
            global.trace.clone(),
        ),
    );
    report.parameters = Parameters {
        duration: Millis::from(options.duration),
        metrics_interval: Millis::from(options.metrics_interval),
        concurrency: options.threads,
        concurrency_sweep: options.sweep.clone(),
        fail_under: output.fail_under.map(Size),
        payload_size: Some(Size(payload_size)),
        piece_size: Some(Size(block_size)),
        ..Default::default()
    };
    if report.environment.build.debug_assertions {
        report.note("this is a debug build: the numbers describe a debug build and nothing else");
    }

    // The payload directory has to be one this run owns, because every step
    // removes it afterwards. A caller-named directory is used as given and a
    // default one is made fresh and removed at the end.
    let (root, temporary) = match &args.dir {
        Some(dir) => (env.resolve(dir), false),
        None => (
            std::env::temp_dir().join(format!("bit-cli-bench-disk-{}", std::process::id())),
            true,
        ),
    };
    std::fs::create_dir_all(&root).map_err(|e| {
        bit_cli_core::error::from_io(e, format!("cannot create {}", root.display()))
    })?;
    report.target = Target {
        source: root.display().to_string(),
        name: Some(format!(
            "{} across {} thread{}",
            options.layout.as_str(),
            options.threads,
            match options.threads {
                1 => "",
                _ => "s",
            }
        )),
        total: Some(Size(payload_size)),
        piece_length: Some(Size(block_size)),
        ..Default::default()
    };

    if global.dry_run {
        report.note("dry run: nothing was written");
    } else {
        let mut samples = Vec::new();
        let outcome = bench_disk::run(&root, &options, |sample| samples.push(sample.clone()))
            .map_err(|e| Error::disk(format!("{e:#}")))?;
        for sample in &samples {
            renderer.event(env, "bench_sample", sample)?;
        }
        for note in &outcome.notes {
            renderer.warn(env, note);
            report.note(note.clone());
        }
        report.series = outcome.series;
        report.concurrency_curve = outcome.concurrency_curve;
        report.disk_steps = outcome.steps;
        report.summary = outcome.summary;
        for sample in &report.series {
            report.environment.observe(&sample.process);
        }
    }
    if temporary && let Err(e) = std::fs::remove_dir_all(&root) {
        renderer.warn(env, format!("could not remove {}: {e}", root.display()));
    }

    let met = match global.dry_run {
        true => {
            if output.fail_under.is_some() {
                report.note("--fail-under was not applied: a dry run measures nothing");
            }
            true
        }
        false => report.apply_threshold(output.fail_under),
    };
    compare_against_baseline(&mut report, &output, renderer, env)?;
    report.environment.finish();

    // A step that read back a block it did not write is a correctness failure,
    // not a slow one, and it outranks the threshold.
    let scrambled = report
        .disk_steps
        .iter()
        .any(|step| step.verified == Some(false));
    let code = match (global.dry_run, scrambled, met) {
        (true, _, _) => ExitCode::Success,
        (_, true, _) => ExitCode::HashMismatch,
        (_, _, false) => ExitCode::ThresholdNotMet,
        _ => ExitCode::Success,
    };
    emit(&report, &output, renderer, env, code)
}

/// `bit-cli bench leech`: download a target and measure what it cost.
///
/// The same fetch `download` runs, with the clock and the counters on. Three
/// numbers separate where the time went, which a rate on its own cannot:
///
/// - the block request pipeline, from how many blocks the session had
///   outstanding and how long each took to answer;
/// - verification, from the wall time of every piece read back and hashed;
/// - the disk, from the positioned reads and writes underneath both.
pub fn leech(
    args: &crate::cli::BenchLeechArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let output = Output::resolve(&args.shared.report, global, env)?;
    let parameters = parameters(&args.shared)?;
    let run_for = Duration::from_millis(parameters.duration.0);
    let warmup = Duration::from_millis(parameters.warmup.0);
    let interval = Duration::from_millis(parameters.metrics_interval.0);
    let init_timeout = swarm::duration_flag(&args.limits.init_timeout, "init-timeout")?;
    let peers = swarm::peer_addrs(&args.peers)?;

    let setup = SessionSetup {
        global,
        trackers: &args.trackers,
        limits: &args.limits,
        web_seeds: &args.web_seeds,
        listen_ports: swarm::port_range(&args.port)?,
        no_dht: false,
        no_lsd: false,
        allocation: super::download::allocation_of(args.file_allocation),
    };
    let engine_options = setup.engine_options(env)?;
    let directory = engine_options.download_directory.clone();

    let mut report = Report::new(
        Kind::Leech,
        Environment::begin(
            build(),
            env.args.clone(),
            env.cwd.display().to_string(),
            global.trace.clone(),
        ),
    );
    report.parameters = parameters.clone();
    report.target = Target {
        source: args.source.source.clone(),
        ..Default::default()
    };
    if report.environment.build.debug_assertions {
        report.note("this is a debug build: the numbers describe a debug build and nothing else");
    }

    let kind = SourceKind::classify(&args.source.source, env)?;
    let meta = match &kind {
        SourceKind::File(path) => Some(Metainfo::read(path)?),
        _ => None,
    };
    let specs = webseed_args::collect(
        &args.web_seeds,
        meta.as_ref(),
        None,
        env,
        webseed_args::no_network,
    )?;
    let trackers = setup.tracker_list(meta.as_ref(), env)?;

    if global.dry_run {
        report.note("dry run: nothing was downloaded");
        report.target.endpoints = specs.iter().map(|spec| spec.url.clone()).collect();
        if let Some(meta) = &meta {
            let layout = meta.layout();
            describe(&mut report, meta, &layout);
        }
        report.environment.finish();
        return emit(&report, &output, renderer, env, ExitCode::Success);
    }

    let runtime = crate::swarm::runtime()?;
    let outcome = runtime.block_on(async {
        let engine = Arc::new(Engine::start(&engine_options).await?);
        for warning in engine.warnings() {
            renderer.warn(env, warning);
        }
        let add = bit_cli_core::engine::AddOptions {
            overwrite: !args.keep_existing,
            trackers: trackers.clone(),
            disable_trackers: trackers.as_ref().is_some_and(Vec::is_empty),
            initial_peers: peers.clone(),
            ..Default::default()
        };
        let handle = engine.add(&args.source.source, &add).await?;
        engine
            .wait_until_initialized_within(&handle, init_timeout)
            .await?;
        let layout = Arc::new(engine.layout(&handle).ok_or_else(|| {
            Error::source_resolution(format!(
                "{}: the torrent has no metadata",
                args.source.source
            ))
        })?);

        // A payload already sitting in the output directory hash-checks clean
        // on add and the torrent is finished before a single byte is fetched.
        // A rate taken from that run describes the hash checker, so the run
        // refuses rather than reporting one. This is the failure a benchmark
        // script hits when its own cleanup silently did not happen.
        if engine.snapshot(&handle).finished {
            return Err(Error::usage(format!(
                "the payload is already complete in {}, so there is nothing to download and nothing to measure",
                directory.display()
            ))
            .with("directory", directory.display().to_string())
            .with(
                "hint",
                "remove it, or point --dir at a directory that does not hold this torrent",
            ));
        }

        let (sources, _set) = swarm::attach_sources(
            &engine,
            &handle,
            &layout,
            &specs,
            &swarm::AttachOptions {
                require: args.web_seeds.web_seed_require,
                peers_available: !args.web_seeds.web_seed_only,
                cache_windows: super::download::cache_windows(&specs),
                trace: global.trace.iter().any(|t| t == "http"),
                verify: bit_cli_core::webseed::fetch::Verify::Piece,
            },
        )
        .await?;

        let result = drive_leech(
            &engine,
            &handle,
            &sources,
            &LeechOptions {
                duration: run_for,
                warmup,
                interval,
                stop_on_complete: !args.run_full_duration,
            },
            renderer,
            env,
        )
        .await;
        for source in &sources {
            source.stop();
        }
        for note in engine.storage_notes() {
            renderer.warn(env, note);
        }
        let snapshot = engine.snapshot(&handle);
        let layout = (*layout).clone();
        Arc::try_unwrap(engine).ok().map(Engine::stop);
        result.map(|outcome| (outcome, layout, snapshot))
    });

    let (outcome, layout, snapshot) = outcome?;
    if let Some(meta) = &meta {
        describe(&mut report, meta, &layout);
    } else {
        report.target.name = Some(layout.name.clone());
        report.target.info_hash = Some(snapshot.info_hash.clone());
        report.target.total = Some(Size(layout.total_length));
        report.target.piece_length = Some(Size(u64::from(layout.piece_length)));
        report.target.piece_count = Some(layout.piece_count());
    }
    report.target.endpoints = outcome.endpoints;
    report.series = outcome.series;
    report.sources = outcome.sources;
    report.summary = outcome.summary;
    for note in outcome.notes {
        renderer.warn(env, &note);
        report.note(note);
    }
    for sample in &report.series {
        report.environment.observe(&sample.process);
    }
    if let Some(ceiling) = parameters.ceiling {
        report.summary.ceiling_share = report.summary.share_of(ceiling.0);
    }
    if !snapshot.finished {
        report.note(format!(
            "the torrent did not complete: {} of {} arrived",
            bit_cli_core::units::format_size(snapshot.progress_bytes),
            bit_cli_core::units::format_size(snapshot.total_bytes)
        ));
    }

    let met = report.apply_threshold(output.fail_under);
    compare_against_baseline(&mut report, &output, renderer, env)?;
    report.environment.finish();
    let _ = env.note(format!("payload written to {}", directory.display()));

    let code = match (met, report.summary.bytes.0) {
        (false, _) => ExitCode::ThresholdNotMet,
        // Nothing arrived at all is not a slow swarm, it is no swarm, and a
        // caller has to be able to tell those apart.
        (_, 0) => ExitCode::NoUsableSources,
        _ => ExitCode::Success,
    };
    emit(&report, &output, renderer, env, code)
}

/// Serve a payload and measure what the swarm pulls.
///
/// The same envelope `bench leech` fills, with every counter facing the other
/// way. What is measured is `uploaded_bytes` per peer rather than
/// `downloaded_bytes`, and the disk figures are reads rather than writes,
/// because a seeder's storage cost is reading the payload back.
///
/// Three things a leech run has and this one does not. There is no source
/// list, because a seeder has no HTTP sources: the rows are the peers. There
/// is no pipeline depth, because the request window belongs to the side
/// asking. And there is no piece verification inside the measured window: a
/// seeder hash-checks the whole payload once on add and then serves it, so
/// `--include-hash-check` is what puts that read into the report rather than
/// leaving it before the clock starts.
///
/// See `TODO/bench.md`, T-090.
pub fn seed(
    args: &crate::cli::BenchSeedArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let output = Output::resolve(&args.shared.report, global, env)?;
    let parameters = parameters(&args.shared)?;
    let run_for = Duration::from_millis(parameters.duration.0);
    let warmup = Duration::from_millis(parameters.warmup.0);
    let interval = Duration::from_millis(parameters.metrics_interval.0);
    let init_timeout = swarm::duration_flag(&args.limits.init_timeout, "init-timeout")?;
    let idle = swarm::optional_duration(&args.exit_when_idle, "exit-when-idle")?;

    // A seeder serves what is on disk and creates nothing, so the allocation
    // method never comes into it. Web seeds are the same: this measures a
    // swarm pulling from this process.
    let web_seeds = crate::cli::WebSeedArgs::default();
    let setup = SessionSetup {
        global,
        trackers: &args.trackers,
        limits: &args.limits,
        web_seeds: &web_seeds,
        listen_ports: swarm::port_range(&args.port)?,
        no_dht: args.no_dht,
        no_lsd: args.no_lsd,
        allocation: bit_cli_core::alloc::Allocation::default(),
    };
    let mut engine_options = setup.engine_options(env)?;
    if let Some(data) = &args.data {
        engine_options.download_directory = env.resolve(data);
    }
    let directory = engine_options.download_directory.clone();

    let mut report = Report::new(
        Kind::Seed,
        Environment::begin(
            build(),
            env.args.clone(),
            env.cwd.display().to_string(),
            global.trace.clone(),
        ),
    );
    report.parameters = parameters.clone();
    report.target = Target {
        source: args.source.source.clone(),
        ..Default::default()
    };
    if report.environment.build.debug_assertions {
        report.note("this is a debug build: the numbers describe a debug build and nothing else");
    }

    let kind = SourceKind::classify(&args.source.source, env)?;
    let meta = match &kind {
        SourceKind::File(path) => Some(Metainfo::read(path)?),
        _ => None,
    };
    let trackers = setup.tracker_list(meta.as_ref(), env)?;

    if global.dry_run {
        report.note("dry run: nothing was served");
        if let Some(meta) = &meta {
            let layout = meta.layout();
            describe(&mut report, meta, &layout);
        }
        report.environment.finish();
        return emit(&report, &output, renderer, env, ExitCode::Success);
    }

    // Refuse before the session starts, not after.
    //
    // Adding a torrent for seeding creates its storage, so a run pointed at
    // the wrong directory would build the whole payload tree at full size and
    // only then discover there is nothing in it. On a 40 GB torrent that is a
    // 40 GB surprise. This costs one `exists` call and catches the common
    // case; a torrent whose name the filesystem refuses is caught by the check
    // after the add instead, which is still there.
    if let Some(meta) = &meta {
        let name = &meta.info().name;
        if !name.is_empty() && !directory.join(name).exists() {
            return Err(Error::usage(format!(
                "{} is not in {}, so there is nothing to serve and nothing to measure",
                name,
                directory.display()
            ))
            .with("directory", directory.display().to_string())
            .with("expected", directory.join(name).display().to_string())
            .with("hint", "point --data at the directory holding the payload"));
        }
    }

    let runtime = crate::swarm::runtime()?;
    let outcome = runtime.block_on(async {
        let engine = Arc::new(Engine::start(&engine_options).await?);
        for warning in engine.warnings() {
            renderer.warn(env, warning);
        }
        let add = bit_cli_core::engine::AddOptions {
            // Seeding needs the payload that is already there read and
            // hash-checked, which is what `overwrite` allows. Without it the
            // add fails on the files that are the whole point.
            overwrite: true,
            trackers: trackers.clone(),
            disable_trackers: trackers.as_ref().is_some_and(Vec::is_empty),
            tracker_interval: swarm::optional_duration(
                &args.trackers.tracker_interval,
                "tracker-interval",
            )?,
            ..Default::default()
        };

        // The hash check happens between `add` and `wait_until_initialized`,
        // so bracketing those two is how long it took. Its cost is normally
        // not what a seeding benchmark is about, which is why it is reported
        // separately rather than folded into the rate.
        let check_began = std::time::Instant::now();
        let before = engine.storage_counts();
        let handle = engine.add(&args.source.source, &add).await?;
        engine
            .wait_until_initialized_within(&handle, init_timeout)
            .await?;
        let check_took = check_began.elapsed();
        let check_disk = engine.storage_counts().since(&before);

        let layout = Arc::new(engine.layout(&handle).ok_or_else(|| {
            Error::source_resolution(format!(
                "{}: the torrent has no metadata",
                args.source.source
            ))
        })?);

        // Serving nothing is not a slow seeder, it is a missing payload, and a
        // rate taken from it would be zero for a reason the number cannot
        // carry.
        let start = engine.snapshot(&handle);
        if start.progress_bytes == 0 {
            return Err(Error::usage(format!(
                "none of this torrent's payload is in {}, so there is nothing to serve and nothing to measure",
                directory.display()
            ))
            .with("directory", directory.display().to_string())
            .with("hint", "point --data at the directory holding the payload"));
        }

        let result = drive_seed(
            &engine,
            &handle,
            &SeedOptions {
                duration: run_for,
                warmup,
                interval,
                idle,
                partial: !start.finished,
                hash_check: args.include_hash_check.then(|| HashCheck {
                    took: check_took,
                    read_bytes: check_disk.read_bytes,
                    read_ops: check_disk.read_ops,
                    pieces: layout.piece_count(),
                }),
                have: start.progress_bytes,
            },
            renderer,
            env,
        )
        .await;
        for note in engine.storage_notes() {
            renderer.warn(env, note);
        }
        let snapshot = engine.snapshot(&handle);
        let listen = engine.listen_addr().map(|a| a.to_string());
        let layout = (*layout).clone();
        Arc::try_unwrap(engine).ok().map(Engine::stop);
        result.map(|outcome| (outcome, layout, snapshot, listen))
    });

    let (outcome, layout, snapshot, listen) = outcome?;
    if let Some(meta) = &meta {
        describe(&mut report, meta, &layout);
    } else {
        report.target.name = Some(layout.name.clone());
        report.target.info_hash = Some(snapshot.info_hash.clone());
        report.target.total = Some(Size(layout.total_length));
        report.target.piece_length = Some(Size(u64::from(layout.piece_length)));
        report.target.piece_count = Some(layout.piece_count());
    }
    // A seeder's endpoint is the address it listens on, which is what a
    // leecher has to be given.
    report.target.endpoints = listen.into_iter().collect();
    report.series = outcome.series;
    report.sources = outcome.sources;
    report.summary = outcome.summary;
    for note in outcome.notes {
        renderer.warn(env, &note);
        report.note(note);
    }
    for sample in &report.series {
        report.environment.observe(&sample.process);
    }
    if let Some(ceiling) = parameters.ceiling {
        report.summary.ceiling_share = report.summary.share_of(ceiling.0);
    }

    let met = report.apply_threshold(output.fail_under);
    compare_against_baseline(&mut report, &output, renderer, env)?;
    report.environment.finish();

    let code = match (met, report.summary.bytes.0) {
        (false, _) => ExitCode::ThresholdNotMet,
        // Nobody pulled a byte. That is not a slow seeder and a caller has to
        // be able to tell the two apart, so it takes the same code a leech run
        // with no usable source takes.
        (_, 0) => ExitCode::NoUsableSources,
        _ => ExitCode::Success,
    };
    emit(&report, &output, renderer, env, code)
}

/// What a `bench seed` run was asked to do.
struct SeedOptions {
    duration: Duration,
    warmup: Duration,
    interval: Duration,
    /// Stop when no peer has been connected for this long.
    idle: Option<Duration>,
    /// Whether the payload on disk is incomplete, so this is a partial seed.
    partial: bool,
    /// The hash check on add, when the caller asked for it in the report.
    hash_check: Option<HashCheck>,
    /// Bytes of the payload that are actually present.
    have: u64,
}

/// What the hash check on add cost.
///
/// The read counters rather than the verification ones. `on_piece_completed`
/// is what brackets a verification, and `librqbit` calls it when a piece the
/// session just downloaded checks out, not when the initial check walks a
/// payload that is already there. So the check shows up in this storage as a
/// run of positioned reads and nothing else, and its wall time is what carries
/// the SHA-1.
struct HashCheck {
    took: Duration,
    read_bytes: u64,
    read_ops: u64,
    pieces: u32,
}

/// What a `bench seed` run produced.
struct SeedOutcome {
    series: Vec<bit_cli_core::bench::report::Sample>,
    sources: Vec<bit_cli_core::bench::report::SourceSummary>,
    summary: bit_cli_core::bench::report::Summary,
    notes: Vec<String>,
}

/// Sample a seeding session until its deadline.
async fn drive_seed(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    options: &SeedOptions,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<SeedOutcome> {
    let recorder = recorder::Recorder::new(options.warmup, options.interval, 1);
    let deadline = std::time::Instant::now() + options.duration;
    let mut ticker = tokio::time::interval(options.interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker.tick().await;

    let mut counted: BTreeMap<String, Counted> = BTreeMap::new();
    let mut labels: Vec<(usize, String, String)> = Vec::new();
    let mut next_index = 0usize;
    let mut storage = engine.storage_counts();
    let mut notes = Vec::new();
    let mut idle_since: Option<std::time::Instant> = Some(std::time::Instant::now());
    let mut went_idle = false;

    if options.partial {
        notes.push(format!(
            "only {} of the payload is present, so this is a partial seed and the rate is bounded by what the swarm can want",
            bit_cli_core::units::format_size(options.have)
        ));
    }

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                notes.push("the run was interrupted before its deadline".to_string());
                break;
            }
            _ = ticker.tick() => {}
        }

        // A seeder has no bridges of its own, so every peer in the snapshot is
        // a swarm member and the filter is empty.
        let peers = engine.peers(handle, &std::collections::HashSet::new());
        for peer in &peers {
            let entry = counted.entry(peer.addr.clone()).or_insert_with(|| {
                let index = next_index;
                next_index += 1;
                labels.push((index, peer.addr.clone(), "peer".to_string()));
                Counted {
                    index,
                    ..Default::default()
                }
            });
            // The counter that faces the other way: what this process sent,
            // not what it received.
            let bytes = peer.uploaded_bytes.saturating_sub(entry.bytes);
            entry.bytes = peer.uploaded_bytes;
            recorder.observe_bulk(entry.index, bytes, 0);
        }

        let now = engine.storage_counts();
        let disk = now.since(&storage);
        storage = now;
        recorder.observe_disk(&bit_cli_core::bench::report::Disk {
            read_ops: disk.read_ops,
            read_bytes: Size(disk.read_bytes),
            read_time: Millis(disk.read_nanos / 1_000_000),
            write_ops: disk.write_ops,
            write_bytes: Size(disk.write_bytes),
            write_time: Millis(disk.write_nanos / 1_000_000),
        });

        let snapshot = engine.snapshot(handle);
        recorder.observe_peers(snapshot.peers.live);
        let sample = recorder.sample();
        renderer.event(env, "bench_sample", &sample)?;

        match snapshot.peers.live {
            0 => {
                idle_since.get_or_insert_with(std::time::Instant::now);
            }
            _ => idle_since = None,
        }
        if let Some(limit) = options.idle
            && let Some(since) = idle_since
            && since.elapsed() >= limit
        {
            went_idle = true;
            break;
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
    }
    recorder.stop();

    if !recorder.measured_anything() {
        notes.push(format!(
            "the run ended after {}ms, inside the {}ms warmup, so the whole run is the measured window",
            recorder.elapsed().as_millis(),
            options.warmup.as_millis()
        ));
        recorder.collapse_warmup();
    }
    if went_idle {
        notes.push(
            "no peer was connected for --exit-when-idle, so the run stopped before its deadline"
                .to_string(),
        );
    }

    let mut summary = recorder.summary();
    if let Some(check) = &options.hash_check {
        let ms = check.took.as_millis().min(u128::from(u64::MAX)) as u64;
        summary.hashing = Some(bit_cli_core::bench::report::Hashing {
            pieces: u64::from(check.pieces),
            bytes: Size(check.read_bytes),
            total: Millis(ms),
            rate: bit_cli_core::units::Rate(match ms {
                0 => 0,
                ms => check.read_bytes.saturating_mul(1000) / ms,
            }),
        });
        notes.push(format!(
            "the hash check on add read {} over {} reads in {}ms before the clock started; it is in summary.hashing and not in the rate",
            bit_cli_core::units::format_size(check.read_bytes),
            check.read_ops,
            ms
        ));
    }
    if counted.is_empty() {
        notes.push(
            "no peer connected, so nothing was measured. Give a leecher this run's listen address, or set --exit-when-idle to fail fast".to_string(),
        );
    }

    Ok(SeedOutcome {
        series: recorder.series(),
        sources: recorder.sources(&labels),
        summary,
        notes,
    })
}

/// Fill the target block from a torrent that was read locally.
fn describe(report: &mut Report, meta: &Metainfo, layout: &Layout) {
    report.target.info_hash = Some(meta.info_hash().hex());
    report.target.name = Some(layout.name.clone());
    report.target.total = Some(Size(layout.total_length));
    report.target.piece_length = Some(Size(u64::from(layout.piece_length)));
    report.target.piece_count = Some(layout.piece_count());
}

/// What a `bench leech` run was asked to do.
struct LeechOptions {
    duration: Duration,
    warmup: Duration,
    interval: Duration,
    stop_on_complete: bool,
}

/// What a `bench leech` run produced.
struct LeechOutcome {
    series: Vec<bit_cli_core::bench::report::Sample>,
    summary: bit_cli_core::bench::report::Summary,
    sources: Vec<bit_cli_core::bench::report::SourceSummary>,
    endpoints: Vec<String>,
    notes: Vec<String>,
}

/// Everything one source has been counted for so far.
///
/// Peers and bridges both arrive as rows in the session's peer list, because
/// that is where the session counts what it received. Deltas are kept per
/// address: a bridge that reconnects gets a new loopback port and a new row,
/// and treating it as the same address would report a negative delta.
#[derive(Default)]
struct Counted {
    index: usize,
    bytes: u64,
    chunks: u64,
}

/// Run the download with the clock on.
#[allow(clippy::too_many_lines)]
async fn drive_leech(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
    sources: &[AttachedSource],
    options: &LeechOptions,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<LeechOutcome> {
    let recorder = recorder::Recorder::new(options.warmup, options.interval, sources.len().max(1));
    let deadline = std::time::Instant::now() + options.duration;
    let mut ticker = tokio::time::interval(options.interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker.tick().await;

    let mut counted: BTreeMap<String, Counted> = BTreeMap::new();
    let mut labels: Vec<(usize, String, String)> = sources
        .iter()
        .map(|source| (source.index, source.url.clone(), "web_seed".to_string()))
        .collect();
    // Peers are numbered after the declared sources, so a source keeps the
    // index the caller gave it whether or not any peer ever connects.
    let mut next_index = sources.iter().map(|s| s.index + 1).max().unwrap_or(0);
    let mut storage = engine.storage_counts();
    let mut notes = Vec::new();
    let mut completed = false;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                notes.push("the run was interrupted before its deadline".to_string());
                break;
            }
            _ = ticker.tick() => {}
        }

        let bridge_ports = swarm::bridge_ports(sources);
        let by_port: BTreeMap<u16, (usize, String)> = sources
            .iter()
            .flat_map(|source| {
                source
                    .local_ports()
                    .into_iter()
                    .map(|port| (port, (source.index, source.url.clone())))
            })
            .collect();

        for peer in engine.peers(handle, &bridge_ports) {
            let port = peer
                .addr
                .rsplit_once(':')
                .and_then(|(_, port)| port.parse::<u16>().ok());
            let entry = counted.entry(peer.addr.clone()).or_insert_with(|| {
                let (index, label, kind) = match port.and_then(|port| by_port.get(&port)) {
                    Some((index, url)) => (*index, url.clone(), "web_seed"),
                    None => {
                        let index = next_index;
                        next_index += 1;
                        (index, peer.addr.clone(), "peer")
                    }
                };
                if !labels.iter().any(|(i, _, _)| *i == index) {
                    labels.push((index, label, kind.to_string()));
                }
                Counted {
                    index,
                    ..Default::default()
                }
            });
            let bytes = peer.downloaded_bytes.saturating_sub(entry.bytes);
            let chunks = u64::from(peer.chunks).saturating_sub(entry.chunks);
            entry.bytes = peer.downloaded_bytes;
            entry.chunks = u64::from(peer.chunks);
            recorder.observe_bulk(entry.index, bytes, chunks);
        }

        let now = engine.storage_counts();
        let disk = now.since(&storage);
        storage = now;
        recorder.observe_disk(&bit_cli_core::bench::report::Disk {
            read_ops: disk.read_ops,
            read_bytes: Size(disk.read_bytes),
            read_time: Millis(disk.read_nanos / 1_000_000),
            write_ops: disk.write_ops,
            write_bytes: Size(disk.write_bytes),
            write_time: Millis(disk.write_nanos / 1_000_000),
        });
        recorder.observe_hashing(
            disk.verify_pieces,
            disk.verify_bytes,
            Duration::from_nanos(disk.verify_nanos),
        );

        if let Some(pipeline) = pipeline(sources) {
            recorder.live.in_flight.store(
                pipeline.mean_in_flight,
                std::sync::atomic::Ordering::Relaxed,
            );
            recorder.observe_pipeline(pipeline);
        }

        let snapshot = engine.snapshot(handle);
        recorder.observe_peers(snapshot.peers.live);
        let sample = recorder.sample();
        renderer.event(env, "bench_sample", &sample)?;

        for source in sources {
            if source.state() == bit_cli_core::webseed::BridgeState::Failed {
                let note = format!(
                    "{} is unusable: {}",
                    source.url,
                    source.error().unwrap_or_else(|| "no reason given".into())
                );
                if !notes.contains(&note) {
                    notes.push(note);
                }
            }
        }

        if snapshot.finished && options.stop_on_complete {
            completed = true;
            break;
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
    }

    // One last read of the storage counters before the window closes.
    //
    // The loop reads them at the top of its body and decides whether to stop
    // at the bottom, so the work between the last read and the break is not in
    // any interval. On a short run that is most of the run: the download that
    // ends the loop is the one that verified the last pieces, and without this
    // the report carried no `hashing` block at all and
    // `a_leech_measures_the_transfer_the_hashing_and_the_disk` failed on
    // whichever runner finished inside one `--metrics-interval`. See
    // TODO/bench.md, T-149.
    let last = engine.storage_counts();
    let disk = last.since(&storage);
    recorder.observe_disk(&bit_cli_core::bench::report::Disk {
        read_ops: disk.read_ops,
        read_bytes: Size(disk.read_bytes),
        read_time: Millis(disk.read_nanos / 1_000_000),
        write_ops: disk.write_ops,
        write_bytes: Size(disk.write_bytes),
        write_time: Millis(disk.write_nanos / 1_000_000),
    });
    recorder.observe_hashing(
        disk.verify_pieces,
        disk.verify_bytes,
        Duration::from_nanos(disk.verify_nanos),
    );
    recorder.stop();

    if !recorder.measured_anything() {
        notes.push(format!(
            "the run ended after {}ms, inside the {}ms warmup, so the whole run is the measured window",
            recorder.elapsed().as_millis(),
            options.warmup.as_millis()
        ));
        recorder.collapse_warmup();
    }
    if completed {
        notes.push("the torrent completed before the deadline, so the measured window is the transfer rather than --duration".to_string());
    }

    // The per-source rows come from the recorder, which counts what reached
    // the session. What went over HTTP to get it is the source's own, so it is
    // folded in here: the two differing is the amplification.
    let mut rows = recorder.sources(&labels);
    for row in &mut rows {
        let Some(source) = sources.iter().find(|s| s.index == row.index) else {
            continue;
        };
        let (http_bytes, _) = source.http();
        row.connections = Some(source.connections());
        row.http_bytes = Some(Size(http_bytes));
    }

    Ok(LeechOutcome {
        series: recorder.series(),
        sources: rows,
        summary: recorder.summary(),
        endpoints: sources.iter().map(|s| s.url.clone()).collect(),
        notes,
    })
}

/// Add up what every bridge's request pipeline is doing.
///
/// The peaks are summed rather than maxed: each bridge is its own peer with
/// its own request window, and what bounds the run is the total the session
/// keeps outstanding across all of them.
fn pipeline(sources: &[AttachedSource]) -> Option<bit_cli_core::bench::report::Pipeline> {
    if sources.is_empty() {
        return None;
    }
    let mut total = bit_cli_core::webseed::bridge::BridgePipeline::default();
    let mut served = 0u64;
    for source in sources {
        let one = source.pipeline();
        total.in_flight += one.in_flight;
        total.peak_in_flight += one.peak_in_flight;
        total.requests += one.requests;
        total.blocks += one.blocks;
        total.service_nanos += one.service_nanos;
        served += source.served_bytes();
    }
    Some(bit_cli_core::bench::report::Pipeline {
        peak_in_flight: total.peak_in_flight,
        mean_in_flight: total.in_flight,
        requests: total.requests,
        blocks: total.blocks,
        mean_service_us: total.mean_service_us().unwrap_or(0),
        block_size: Size(match total.blocks {
            0 => 0,
            blocks => served / blocks,
        }),
        ..Default::default()
    })
}

/// Read `--baseline` and fold the comparison into the report.
fn compare_against_baseline(
    report: &mut Report,
    output: &Output,
    renderer: &Renderer,
    env: &mut Env,
) -> Result<()> {
    let Some(path) = &output.baseline else {
        return Ok(());
    };
    let text = std::fs::read_to_string(path).map_err(|e| {
        bit_cli_core::error::from_io(e, format!("cannot read the baseline {}", path.display()))
    })?;
    let baseline: Report = serde_json::from_str(&text).map_err(|e| {
        Error::usage(format!("{} is not a bench report: {e}", path.display()))
            .with("path", path.display().to_string())
            .with(
                "hint",
                "a baseline is a report written by `bench --format json`",
            )
    })?;
    match bit_cli_core::bench::compare(report, &baseline, &path.display().to_string()) {
        Ok(comparison) => report.baseline = Some(comparison),
        Err(reason) => {
            // A comparison that cannot be made is reported rather than
            // silently dropped, because a caller who asked for one and got no
            // deltas would read that as "nothing changed".
            renderer.warn(env, format!("--baseline was not used: {reason}"));
            report.note(format!("the baseline was not comparable: {reason}"));
        }
    }
    Ok(())
}

/// Write the report where it goes and return the exit code.
fn emit(
    report: &Report,
    output: &Output,
    renderer: &mut Renderer,
    env: &mut Env,
    code: ExitCode,
) -> Result<ExitCode> {
    let rendered = render::render(report, output.format)?;
    match &output.path {
        None => {
            env.say(&rendered)
                .map_err(|e| bit_cli_core::error::from_io(e, "cannot write to stdout"))?;
        }
        Some(path) => {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(|e| {
                    bit_cli_core::error::from_io(e, format!("cannot create {}", parent.display()))
                })?;
            }
            std::fs::write(path, format!("{rendered}\n")).map_err(|e| {
                bit_cli_core::error::from_io(e, format!("cannot write {}", path.display()))
            })?;
            // The report went to a file, so stdout carries the summary a
            // person reads. It is the same numbers, rendered.
            if !renderer.quiet {
                for line in render::text(report) {
                    env.say(line)
                        .map_err(|e| bit_cli_core::error::from_io(e, "cannot write to stdout"))?;
                }
            }
            let _ = env.note(format!("report written to {}", path.display()));
        }
    }
    if output.to_stdout() && output.format != Format::Text && !renderer.quiet {
        let _ = env.note(render::headline(report));
    }
    Ok(code)
}

/// Resolve a source and its bindings without touching the network.
fn resolve(
    source: &str,
    web_seeds: &crate::cli::WebSeedArgs,
    env: &mut Env,
) -> Result<(Metainfo, Layout, BindingSet)> {
    let kind = SourceKind::classify(source, env)?;
    let meta = load_local(&kind, env)?;
    let layout = meta.layout();
    let specs = webseed_args::collect(web_seeds, Some(&meta), None, env, webseed_args::no_network)?;
    if specs.is_empty() {
        return Err(Error::no_usable_sources(
            "no web seed sources: the torrent declares none and none were given",
        )
        .with("hint", "pass --web-seed <URL> or --web-seed-config <PATH>"));
    }
    let set = BindingSet::resolve(&layout, &meta.info_hash().hex(), &specs)?;
    Ok((meta, layout, set))
}

/// Re-exported so tests can name the recorder without reaching into core.
#[allow(unused_imports)]
pub use recorder::Observation;

#[cfg(test)]
mod tests {
    use crate::env::Env;
    use crate::test_support::{FileServer, TorrentFixture, run_err, run_ok};
    use bit_cli_core::ExitCode;

    /// Run `bench` with no global format flag and read the report off stdout.
    ///
    /// `bench` writes its report to stdout in `--format`, which defaults to
    /// `json`. Passing `--json` as well would work, but then nothing would
    /// test the documented default.
    fn report(args: &[&str], expected: ExitCode) -> serde_json::Value {
        let (mut env, captured) = Env::test(args, ".");
        let code = crate::run(&mut env);
        assert_eq!(
            code,
            expected,
            "`bit-cli {}` exited {code}, expected {expected}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            captured.out(),
            captured.err()
        );
        captured
            .json()
            .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n{}", captured.out()))
    }

    #[test]
    fn a_dry_run_writes_a_report_with_a_full_environment() {
        let fixture = TorrentFixture::multi_file();
        let doc = report(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
            ],
            ExitCode::Success,
        );
        assert_eq!(doc["kind"], "webseed");
        assert_eq!(doc["report_version"], 1);

        let environment = &doc["environment"];
        assert_eq!(environment["build"]["version"], bit_cli_core::VERSION);
        assert!(
            environment["build"]["target"]
                .as_str()
                .unwrap()
                .contains('-')
        );
        assert!(
            !environment["host"]["cpu"]["model"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        assert!(
            environment["host"]["cpu"]["logical_cores"]
                .as_u64()
                .unwrap()
                >= 1
        );
        assert!(
            !environment["host"]["os"]["name"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        assert!(
            environment["host"]["memory_total"]["bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(environment["process"]["peak_rss_bytes"].as_u64().unwrap() > 0);
        assert!(environment["process"]["open_handles"].as_u64().unwrap() > 0);
        assert!(
            environment["started_at"]["iso"]
                .as_str()
                .unwrap()
                .ends_with('Z')
        );
        assert!(
            environment["finished_at"]["iso"]
                .as_str()
                .unwrap()
                .ends_with('Z')
        );
        assert_eq!(environment["command_line"][0], "bit-cli");
        assert!(
            environment["command_line"]
                .as_array()
                .unwrap()
                .iter()
                .any(|arg| arg == "--dry-run"),
            "the exact command line is recorded"
        );
    }

    #[test]
    fn the_target_is_described_before_anything_is_requested() {
        let fixture = TorrentFixture::multi_file();
        let doc = report(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
            ],
            ExitCode::Success,
        );
        assert_eq!(doc["target"]["info_hash"], fixture.info_hash);
        assert_eq!(doc["target"]["name"], "album");
        assert_eq!(doc["target"]["total"]["bytes"], 2000);
        assert_eq!(doc["target"]["piece_count"], 2);
        assert_eq!(
            doc["target"]["endpoints"][0],
            "https://mirror.example.com/pub/"
        );
    }

    #[test]
    fn the_parameters_record_what_the_flags_asked_for() {
        let fixture = TorrentFixture::multi_file();
        let doc = report(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--duration",
                "45s",
                "--warmup",
                "2s",
                "--metrics-interval",
                "500ms",
                "--concurrency",
                "12",
                "--concurrency-sweep",
                "1,4,16",
                "--ceiling",
                "10MiB/s",
                "--target-rate",
                "1MiB/s",
            ],
            ExitCode::Success,
        );
        let parameters = &doc["parameters"];
        assert_eq!(parameters["duration"]["ms"], 45_000);
        assert_eq!(parameters["warmup"]["ms"], 2000);
        assert_eq!(parameters["metrics_interval"]["ms"], 500);
        assert_eq!(parameters["concurrency"], 12);
        assert_eq!(
            parameters["concurrency_sweep"],
            serde_json::json!([1, 4, 16])
        );
        assert_eq!(parameters["ceiling"]["bytes"], 10 * 1024 * 1024);
        assert_eq!(parameters["target_rate"]["bytes"], 1024 * 1024);
    }

    #[test]
    fn a_torrent_with_no_usable_source_says_so_rather_than_measuring_nothing() {
        let fixture = TorrentFixture::multi_file();
        let error = run_err(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--no-web-seed",
            ],
            ".",
            ExitCode::NoUsableSources,
        );
        assert!(error.contains("no web seed sources"), "{error}");
    }

    /// `swarm` is the last one left, and it says which entry closes it rather
    /// than pretending to work. `probe` was on this list until T-090 step 5,
    /// which is why this is one case rather than a loop.
    #[test]
    fn the_subcommands_that_are_not_built_name_their_todo_entry() {
        let fixture = TorrentFixture::multi_file();
        {
            let subcommand = "swarm";
            let (mut env, captured) = crate::env::Env::test(
                &[
                    "--json",
                    "bench",
                    subcommand,
                    fixture.torrent.to_str().unwrap(),
                ],
                ".",
            );
            let code = crate::run(&mut env);
            assert_ne!(
                code,
                ExitCode::Success,
                "bench {subcommand} claimed to work"
            );
            let doc = captured.json().unwrap();
            assert_eq!(doc["context"]["todo"], "T-090");
            assert_eq!(doc["context"]["subcommand"], subcommand);
        }
    }

    #[test]
    fn a_zero_duration_is_refused_rather_than_measured() {
        let fixture = TorrentFixture::multi_file();
        let (mut env, captured) = crate::env::Env::test(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--duration",
                "0s",
            ],
            ".",
        );
        assert_eq!(crate::run(&mut env), ExitCode::Usage);
        assert!(captured.err().contains("--duration"), "{}", captured.err());
    }

    #[test]
    fn a_warmup_that_swallows_the_run_is_noted_rather_than_hidden() {
        let fixture = TorrentFixture::multi_file();
        let doc = report(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--duration",
                "5s",
                "--warmup",
                "10s",
            ],
            ExitCode::Success,
        );
        let notes = doc["notes"].as_array().unwrap();
        assert!(
            notes
                .iter()
                .any(|note| note.as_str().unwrap().contains("nothing is measured")),
            "{notes:?}"
        );
    }

    #[test]
    fn text_format_renders_the_same_report_a_person_can_read() {
        let fixture = TorrentFixture::multi_file();
        let out = run_ok(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--format",
                "text",
            ],
            ".",
        );
        assert!(out.contains("bench webseed"), "{out}");
        assert!(out.contains("Environment"), "{out}");
        assert!(out.contains("Summary"), "{out}");
        assert!(
            out.contains("album.torrent") || out.contains("album"),
            "{out}"
        );
    }

    #[test]
    fn csv_format_writes_a_header_even_for_an_empty_series() {
        let fixture = TorrentFixture::multi_file();
        let out = run_ok(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--format",
                "csv",
            ],
            ".",
        );
        assert!(
            out.starts_with("at,elapsed_ms,warmup,concurrency,bytes"),
            "{out}"
        );
    }

    #[test]
    fn ndjson_format_writes_one_object_per_line() {
        let fixture = TorrentFixture::multi_file();
        let out = run_ok(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--format",
                "ndjson",
            ],
            ".",
        );
        for line in out.lines().filter(|line| !line.trim().is_empty()) {
            let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
            assert!(value["record"].is_string());
        }
    }

    #[test]
    fn a_report_path_writes_the_file_and_leaves_a_summary_on_stdout() {
        let fixture = TorrentFixture::multi_file();
        let path = fixture.root.join("reports").join("run.json");
        let out = run_ok(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--report",
                path.to_str().unwrap(),
            ],
            ".",
        );
        assert!(out.contains("bench webseed"), "stdout carries the summary");
        let written = std::fs::read_to_string(&path).expect("the report file exists");
        let doc: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(doc["kind"], "webseed");
        assert_eq!(doc["target"]["info_hash"], fixture.info_hash);
    }

    #[test]
    fn a_report_written_to_a_file_reads_back_as_a_baseline() {
        let fixture = TorrentFixture::multi_file();
        let first = fixture.root.join("first.json");
        run_ok(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--report",
                first.to_str().unwrap(),
            ],
            ".",
        );
        let doc = report(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--baseline",
                first.to_str().unwrap(),
            ],
            ExitCode::Success,
        );
        let deltas = doc["baseline"]["deltas"].as_array().unwrap();
        assert!(!deltas.is_empty(), "a baseline produces a delta per metric");
        let metrics: Vec<&str> = deltas
            .iter()
            .map(|d| d["metric"].as_str().unwrap())
            .collect();
        assert!(metrics.contains(&"sustained_rate"), "{metrics:?}");
        assert!(metrics.contains(&"peak_rss_bytes"), "{metrics:?}");
        for delta in deltas {
            assert!(delta["higher_is_better"].is_boolean());
            assert!(delta["human"].as_str().unwrap().starts_with(['+', '-']));
        }
    }

    #[test]
    fn a_baseline_that_is_not_a_report_names_the_file() {
        let fixture = TorrentFixture::multi_file();
        let path = fixture.root.join("nonsense.json");
        std::fs::write(&path, "{\"not\": \"a report\"}").unwrap();
        let (mut env, captured) = crate::env::Env::test(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--baseline",
                path.to_str().unwrap(),
            ],
            ".",
        );
        assert_eq!(crate::run(&mut env), ExitCode::Usage);
        assert!(
            captured.err().contains("nonsense.json"),
            "{}",
            captured.err()
        );
    }

    #[test]
    fn a_baseline_from_other_hardware_is_refused_and_the_run_still_reports() {
        let fixture = TorrentFixture::multi_file();
        let path = fixture.root.join("elsewhere.json");
        run_ok(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--report",
                path.to_str().unwrap(),
            ],
            ".",
        );
        let mut doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        doc["environment"]["host"]["cpu"]["model"] =
            serde_json::Value::String("Some Other Processor".into());
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();

        let out = report(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--dry-run",
                "--baseline",
                path.to_str().unwrap(),
            ],
            ExitCode::Success,
        );
        assert!(out["baseline"].is_null(), "no comparison was made");
        let notes = out["notes"].as_array().unwrap();
        assert!(
            notes
                .iter()
                .any(|note| note.as_str().unwrap().contains("not comparable")),
            "{notes:?}"
        );
    }

    #[test]
    fn a_bad_sweep_names_the_term_that_is_wrong() {
        let fixture = TorrentFixture::multi_file();
        let (mut env, captured) = crate::env::Env::test(
            &[
                "bench",
                "webseed",
                fixture.torrent.to_str().unwrap(),
                "--concurrency-sweep",
                "1,2,x",
            ],
            ".",
        );
        assert_eq!(crate::run(&mut env), ExitCode::Usage);
        assert!(captured.err().contains('x'), "{}", captured.err());
    }

    /// A dry run resolves the target and stops. It is what CI calls to check
    /// that a benchmark would run before spending the time on it.
    #[test]
    fn a_leech_dry_run_describes_the_target_without_downloading() {
        let fixture = TorrentFixture::single_file();
        let doc = report(
            &[
                "bench",
                "leech",
                fixture.path_str(),
                "--web-seed",
                "http://127.0.0.1:9/",
                "--dry-run",
            ],
            ExitCode::Success,
        );
        assert_eq!(doc["kind"], "leech");
        assert_eq!(doc["target"]["info_hash"], fixture.info_hash);
        assert_eq!(doc["target"]["piece_count"], 3);
        assert_eq!(doc["summary"]["bytes"]["bytes"], 0);
        assert!(
            doc["notes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|note| note.as_str().unwrap().contains("dry run")),
            "{}",
            doc["notes"]
        );
        assert!(
            environment_is_complete(&doc["environment"]),
            "{}",
            doc["environment"]
        );
    }

    fn environment_is_complete(environment: &serde_json::Value) -> bool {
        !environment["host"]["cpu"]["model"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
            && environment["host"]["memory_total"]["bytes"]
                .as_u64()
                .unwrap_or_default()
                > 0
            && environment["started_at"]["iso"]
                .as_str()
                .unwrap_or_default()
                .ends_with('Z')
    }

    /// The whole thing, against a real HTTP server on loopback: the payload
    /// arrives, and the report says what it cost in verification, in disk, and
    /// in pipeline depth.
    #[test]
    fn a_leech_measures_the_transfer_the_hashing_and_the_disk() {
        let fixture = TorrentFixture::single_file();
        let server = FileServer::start(fixture.payload_dir());
        let out = fixture.dir().join("out");
        let doc = report(
            &[
                "bench",
                "leech",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed",
                &server.base,
                "--web-seed-only",
                "--port",
                "0",
                "--duration",
                "60s",
                "--warmup",
                "0s",
                "--metrics-interval",
                "100ms",
            ],
            ExitCode::Success,
        );

        let total = fixture.files[0].1.len() as u64;
        assert_eq!(doc["summary"]["bytes"]["bytes"].as_u64().unwrap(), total);
        assert_eq!(
            std::fs::read(out.join("payload.bin")).unwrap(),
            fixture.files[0].1,
            "the payload on disk is the payload in the torrent"
        );

        let hashing = &doc["summary"]["hashing"];
        assert_eq!(hashing["pieces"].as_u64().unwrap(), 3);
        assert_eq!(hashing["bytes"]["bytes"].as_u64().unwrap(), total);

        let disk = &doc["summary"]["disk"];
        assert_eq!(disk["write_bytes"]["bytes"].as_u64().unwrap(), total);
        assert!(disk["read_bytes"]["bytes"].as_u64().unwrap() >= total);

        let pipeline = &doc["summary"]["pipeline"];
        assert!(pipeline["blocks"].as_u64().unwrap() > 0);
        // Not equal: near the end the session re-asks for a block it already
        // has outstanding, and the second answer is dropped rather than sent
        // twice. See TODO/webseed.md, T-008.
        assert!(
            pipeline["requests"].as_u64().unwrap() >= pipeline["blocks"].as_u64().unwrap(),
            "more blocks than requests: {pipeline}"
        );
        assert!(pipeline["peak_in_flight"].as_u64().unwrap() >= 1);

        let sources = doc["sources"].as_array().unwrap();
        assert_eq!(
            sources.len(),
            1,
            "one source served everything: {sources:?}"
        );
        assert_eq!(sources[0]["kind"], "web_seed");
        assert_eq!(sources[0]["label"], server.base);
        assert_eq!(sources[0]["bytes"]["bytes"].as_u64().unwrap(), total);

        assert!(
            !doc["series"].as_array().unwrap().is_empty(),
            "a run with no time series is not a benchmark"
        );
    }

    /// `--fail-under` is what makes a benchmark a CI gate, so it has to work
    /// the same on every subcommand.
    #[test]
    fn a_leech_below_the_threshold_exits_fourteen() {
        let fixture = TorrentFixture::single_file();
        let server = FileServer::start(fixture.payload_dir());
        let out = fixture.dir().join("out");
        let doc = report(
            &[
                "bench",
                "leech",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed",
                &server.base,
                "--web-seed-only",
                "--port",
                "0",
                "--duration",
                "60s",
                "--warmup",
                "0s",
                "--metrics-interval",
                "100ms",
                "--fail-under",
                "100GiB/s",
            ],
            ExitCode::ThresholdNotMet,
        );
        assert_eq!(doc["threshold"]["met"], false);
        assert_eq!(
            doc["threshold"]["fail_under"]["bytes"].as_u64().unwrap(),
            100 * 1024 * 1024 * 1024
        );
    }

    /// A benchmark that finds the payload already there measures the hash
    /// checker, not the transfer. It has to say so rather than report a rate.
    #[test]
    fn a_leech_onto_a_complete_payload_refuses_rather_than_reporting_a_rate() {
        let fixture = TorrentFixture::single_file();
        let server = FileServer::start(fixture.payload_dir());
        let out = fixture.dir().join("out");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("payload.bin"), &fixture.files[0].1).unwrap();

        let stderr = run_err(
            &[
                "bench",
                "leech",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed",
                &server.base,
                "--web-seed-only",
                "--port",
                "0",
                "--duration",
                "30s",
                "--warmup",
                "0s",
            ],
            fixture.dir(),
            ExitCode::Usage,
        );
        assert!(
            stderr.contains("already complete"),
            "the reason has to name what happened: {stderr}"
        );
    }

    /// One source over several connections is still one source in the report,
    /// and every connection serves part of the payload.
    #[test]
    fn a_source_over_several_connections_stays_one_row_and_serves_between_them() {
        let fixture = TorrentFixture::single_file();
        let server = FileServer::start(fixture.payload_dir());
        let out = fixture.dir().join("out");
        let doc = report(
            &[
                "bench",
                "leech",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed",
                &server.base,
                "--web-seed-only",
                "--web-seed-connections",
                "3",
                "--port",
                "0",
                "--duration",
                "60s",
                "--warmup",
                "0s",
                "--metrics-interval",
                "100ms",
            ],
            ExitCode::Success,
        );

        let total = fixture.files[0].1.len() as u64;
        assert_eq!(doc["summary"]["bytes"]["bytes"].as_u64().unwrap(), total);
        assert_eq!(
            std::fs::read(out.join("payload.bin")).unwrap(),
            fixture.files[0].1
        );

        let sources = doc["sources"].as_array().unwrap();
        assert_eq!(
            sources.len(),
            1,
            "three connections are one source: {sources:?}"
        );
        assert_eq!(sources[0]["bytes"]["bytes"].as_u64().unwrap(), total);
    }

    /// `bench disk` writes the payload, reads every block back, and reports
    /// what each thread cost.
    #[test]
    fn a_disk_run_reports_a_step_per_thread_count_and_verifies_what_it_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let doc = report(
            &[
                "bench",
                "disk",
                "--dir",
                dir.path().to_str().unwrap(),
                "--payload-size",
                "2MiB",
                "--block-size",
                "64KiB",
                "--concurrency-sweep",
                "1,2",
                "--metrics-interval",
                "50ms",
            ],
            ExitCode::Success,
        );

        let steps = doc["disk_steps"].as_array().unwrap();
        assert_eq!(steps.len(), 2, "{steps:?}");
        for (index, step) in steps.iter().enumerate() {
            assert_eq!(step["threads"].as_u64().unwrap(), (index + 1) as u64);
            assert_eq!(step["layout"], "shared");
            assert_eq!(step["files"].as_u64().unwrap(), 1);
            assert_eq!(step["bytes"]["bytes"].as_u64().unwrap(), 2 * 1024 * 1024);
            assert_eq!(step["write_ops"].as_u64().unwrap(), 32);
            assert_eq!(step["verified"], true, "the read-back did not check out");
            assert_eq!(
                step["threads_detail"].as_array().unwrap().len(),
                index + 1,
                "one row per thread"
            );
        }
        // Every step writes the whole payload, so the summary is the sum.
        assert_eq!(
            doc["summary"]["bytes"]["bytes"].as_u64().unwrap(),
            4 * 1024 * 1024
        );
        // No torrent and no network, so the payload directory is the target.
        assert_eq!(doc["kind"], "disk");
        // Each step removes its own payload, so nothing is left behind.
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// The `split` layout gives every thread its own file, which is the
    /// control the `shared` reading is taken against.
    #[test]
    fn the_split_layout_reports_one_file_per_thread() {
        let dir = tempfile::tempdir().unwrap();
        let doc = report(
            &[
                "bench",
                "disk",
                "--dir",
                dir.path().to_str().unwrap(),
                "--payload-size",
                "1MiB",
                "--block-size",
                "64KiB",
                "--concurrency",
                "4",
                "--layout",
                "split",
                "--metrics-interval",
                "50ms",
            ],
            ExitCode::Success,
        );
        let step = &doc["disk_steps"][0];
        assert_eq!(step["layout"], "split");
        assert_eq!(step["files"].as_u64().unwrap(), 4);
        assert_eq!(step["verified"], true);
    }

    /// The `handles` layout writes through one handle per thread onto one
    /// file, which is what separates a per-handle limit from a per-file one.
    #[test]
    fn the_handles_layout_writes_one_file_through_several_handles() {
        let dir = tempfile::tempdir().unwrap();
        let doc = report(
            &[
                "bench",
                "disk",
                "--dir",
                dir.path().to_str().unwrap(),
                "--payload-size",
                "1MiB",
                "--block-size",
                "64KiB",
                "--concurrency",
                "4",
                "--layout",
                "handles",
                "--metrics-interval",
                "50ms",
            ],
            ExitCode::Success,
        );
        let step = &doc["disk_steps"][0];
        assert_eq!(step["layout"], "handles");
        assert_eq!(step["files"].as_u64().unwrap(), 1, "handles share one file");
        assert_eq!(step["verified"], true);
    }

    #[test]
    fn a_disk_run_below_the_threshold_exits_fourteen() {
        let dir = tempfile::tempdir().unwrap();
        let doc = report(
            &[
                "bench",
                "disk",
                "--dir",
                dir.path().to_str().unwrap(),
                "--payload-size",
                "1MiB",
                "--block-size",
                "64KiB",
                "--concurrency",
                "2",
                "--metrics-interval",
                "50ms",
                "--fail-under",
                "100GiB/s",
            ],
            ExitCode::ThresholdNotMet,
        );
        assert_eq!(doc["threshold"]["met"], false);
    }

    #[test]
    fn a_block_larger_than_the_payload_is_refused_rather_than_measured() {
        let dir = tempfile::tempdir().unwrap();
        let error = run_err(
            &[
                "bench",
                "disk",
                "--dir",
                dir.path().to_str().unwrap(),
                "--payload-size",
                "64KiB",
                "--block-size",
                "1MiB",
            ],
            ".",
            ExitCode::Usage,
        );
        assert!(error.contains("--block-size"), "{error}");
        assert!(error.contains("--payload-size"), "{error}");
    }

    /// A dry run says what it would do and writes a full report, the same as
    /// every other subcommand.
    #[test]
    fn a_disk_dry_run_describes_the_target_without_writing_a_byte() {
        let dir = tempfile::tempdir().unwrap();
        let doc = report(
            &[
                "--dry-run",
                "bench",
                "disk",
                "--dir",
                dir.path().to_str().unwrap(),
                "--payload-size",
                "1GiB",
                "--concurrency",
                "8",
            ],
            ExitCode::Success,
        );
        assert_eq!(doc["kind"], "disk");
        assert!(environment_is_complete(&doc["environment"]));
        assert_eq!(doc["target"]["name"], "shared across 8 threads");
        assert_eq!(
            doc["target"]["total"]["bytes"].as_u64().unwrap(),
            1024 * 1024 * 1024
        );
        assert!(doc["disk_steps"].as_array().is_none_or(|s| s.is_empty()));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// A seeding benchmark pointed at the wrong directory refuses before it
    /// creates anything.
    ///
    /// Adding a torrent for seeding creates its storage, so without a check
    /// first the run builds the whole payload tree at full size and only then
    /// discovers there is nothing in it. On a 40 GB torrent that is a 40 GB
    /// surprise, and it is how this test's own directory got a stray `album/`
    /// in it. See `TODO/bench.md`, T-090.
    #[test]
    fn a_seed_benchmark_with_no_payload_refuses_without_creating_one() {
        let fixture = TorrentFixture::multi_file();
        let empty = fixture.dir().join("empty");
        std::fs::create_dir_all(&empty).unwrap();

        let (mut env, captured) = crate::env::Env::test(
            &[
                "--json",
                "bench",
                "seed",
                fixture.torrent.to_str().unwrap(),
                "--data",
                empty.to_str().unwrap(),
                "--port",
                "0",
                "--no-dht",
                "--no-lsd",
                "--no-tracker",
                "--duration",
                "1s",
            ],
            fixture.dir(),
        );
        let code = crate::run(&mut env);
        assert_eq!(code, ExitCode::Usage, "stderr:\n{}", captured.err());
        assert!(
            !empty.join("album").exists(),
            "the run created a payload tree it then refused to measure"
        );
        let doc = captured.json().unwrap();
        assert!(
            doc["context"]["expected"]
                .as_str()
                .unwrap_or_default()
                .ends_with("album"),
            "{doc}"
        );
    }

    /// A peer probe says what the peer is and what it holds.
    ///
    /// The seeder is a real one on a thread, so every field comes off the
    /// wire: the reserved bytes it set, the peer id it chose, the extended
    /// handshake it sent, and the bitfield it advertised. See
    /// `TODO/bench.md`, T-090, step 5.
    #[test]
    fn a_peer_probe_reads_the_handshake_and_what_follows_it() {
        let fixture = TorrentFixture::multi_file();
        let data = fixture.dir().join("served");
        fixture.place(&data, &[]);
        let port = crate::test_support::free_port();

        let seeder = {
            let torrent = fixture.path_str().to_string();
            let data = data.to_str().expect("utf-8 path").to_string();
            let cwd = fixture.dir();
            std::thread::spawn(move || {
                let (mut env, _) = Env::test(
                    &[
                        "seed",
                        &torrent,
                        "--data",
                        &data,
                        "--port",
                        &port.to_string(),
                        "--no-dht",
                        "--no-lsd",
                        "--no-tracker",
                        "--stop-after",
                        "10s",
                    ],
                    cwd,
                );
                crate::run(&mut env)
            })
        };

        // The seeder has to be listening before the probe dials it, and how
        // long that takes is the machine's business. Retry rather than sleep a
        // guessed amount. A dial that arrives before the listener is up exits
        // `NoUsableSources`, which is that command working correctly, so the
        // exit code cannot be asserted until a probe has connected: asserting
        // it inside the loop is what made this fail on the slower runners.
        // Eight seconds, against the seeder's own `--stop-after 10s`. A count
        // of attempts is not the same thing: each attempt costs whatever the
        // dial costs, so 40 of them is four seconds on this machine and an
        // unknown number on a loaded runner.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let mut found = serde_json::Value::Null;
        while std::time::Instant::now() < deadline {
            let (mut env, captured) = Env::test(
                &[
                    "bench",
                    "probe",
                    &format!("127.0.0.1:{port}"),
                    "--for",
                    fixture.path_str(),
                    "--timeout",
                    "5s",
                ],
                ".",
            );
            let code = crate::run(&mut env);
            if code == ExitCode::Success {
                let report = captured
                    .json()
                    .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n{}", captured.out()));
                if report["probe"]["reachable"] == true {
                    found = report;
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(
            !found.is_null(),
            "the probe never reached the seeder on 127.0.0.1:{port} within 8 seconds"
        );
        let _ = seeder.join();

        let probe = &found["probe"];
        assert_eq!(probe["kind"], "peer", "{found}");
        assert_eq!(probe["reachable"], true, "{found}");
        let peer = &probe["peer"];
        assert_eq!(peer["info_hash_matches"], true, "{probe}");
        assert!(
            peer["peer_id"]
                .as_str()
                .unwrap_or_default()
                .starts_with('-'),
            "{probe}"
        );
        // Every BitTorrent client sets the BEP 10 bit, and this one advertises
        // its pieces and unchokes as soon as it has said hello.
        assert!(
            peer["extensions"]
                .as_array()
                .expect("extensions")
                .iter()
                .any(|name| name == "extension-protocol"),
            "{probe}"
        );
        assert_eq!(peer["extended"]["client"], "bit-cli 0.1.0", "{probe}");
        assert_eq!(peer["pieces_advertised"], 2, "{probe}");
        assert!(
            peer["messages"]
                .as_array()
                .expect("messages")
                .iter()
                .any(|kind| kind == "bitfield"),
            "{probe}"
        );
    }

    /// An HTTP probe says whether the endpoint answers a range.
    #[test]
    fn an_http_probe_reads_the_status_and_the_range_support() {
        let fixture = TorrentFixture::multi_file();
        let server = FileServer::start(fixture.dir());
        let url = format!("{}payload/notes.nfo", server.base);

        let found = report(
            &["bench", "probe", &url, "--timeout", "5s"],
            ExitCode::Success,
        );
        let probe = &found["probe"];
        assert_eq!(probe["kind"], "http", "{found}");
        assert_eq!(probe["reachable"], true, "{found}");
        assert_eq!(probe["http"]["status"], 206, "{probe}");
        assert_eq!(probe["http"]["range_support"], true, "{probe}");
        assert_eq!(probe["http"]["entity_length"], 500, "{probe}");
    }

    /// A target nothing is listening on exits 6 and says so.
    #[test]
    fn an_unreachable_peer_exits_no_usable_sources() {
        let port = crate::test_support::free_port();
        let found = report(
            &[
                "bench",
                "probe",
                &format!("127.0.0.1:{port}"),
                "--timeout",
                "2s",
            ],
            ExitCode::NoUsableSources,
        );
        assert_eq!(found["probe"]["reachable"], false, "{found}");
        assert!(found["probe"]["error"].is_string(), "{found}");
        // No --for, so the report says what the handshake named.
        assert!(
            found["notes"]
                .as_array()
                .expect("notes")
                .iter()
                .any(|note| note.as_str().unwrap_or_default().contains("zero info hash")),
            "{found}"
        );
    }

    /// A target that is neither an address nor a URL is a usage error.
    #[test]
    fn a_target_that_is_neither_an_address_nor_a_url_is_refused() {
        let (mut env, captured) = Env::test(&["bench", "probe", "mirror.example.com"], ".");
        assert_eq!(crate::run(&mut env), ExitCode::Usage);
        assert!(captured.err().contains("HOST:PORT"), "{}", captured.err());
    }
}
