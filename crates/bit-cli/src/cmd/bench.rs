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

use std::path::PathBuf;
use std::time::Duration;

use bit_cli_core::ExitCode;
use bit_cli_core::bench::render::{self, Format};
use bit_cli_core::bench::report::{Build, Environment, Kind, Parameters, Report, Target};
use bit_cli_core::bench::{recorder, webseed as bench_webseed};
use bit_cli_core::error::{Error, Result};
use bit_cli_core::layout::Layout;
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{Millis, Size, parse_duration, parse_rate, parse_size};
use bit_cli_core::webseed::binding::BindingSet;

use crate::cli::{BenchArgs, BenchCommand, BenchShared, BenchWebseedArgs, Global, ReportFormat};
use crate::env::Env;
use crate::output::Renderer;
use crate::source::{Kind as SourceKind, load_local};
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
        BenchCommand::Leech(args) => unbuilt(Kind::Leech, args),
        BenchCommand::Seed(args) => unbuilt(Kind::Seed, args),
        BenchCommand::Swarm(args) => unbuilt(Kind::Swarm, args),
        BenchCommand::Probe(args) => unbuilt(Kind::Probe, args),
    }
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
    fn resolve(shared: &BenchShared, global: &Global, env: &Env) -> Result<Self> {
        // `--json` and `--jsonl` are the global way to ask for a machine
        // surface, so they set the report format rather than sitting beside
        // it. An explicit `--format` still wins over the default.
        let format = match (global.json, global.jsonl, shared.format) {
            (_, true, _) => Format::Ndjson,
            (true, _, ReportFormat::Json) => Format::Json,
            (true, _, other) => format_of(other),
            _ => format_of(shared.format),
        };
        let path = match shared.report.as_deref() {
            None | Some("-") => None,
            Some(path) => Some(env.resolve(std::path::Path::new(path))),
        };
        Ok(Self {
            format,
            path,
            baseline: shared.baseline.as_ref().map(|p| env.resolve(p)),
            fail_under: shared
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
        target_rate: rate(shared.target_rate.as_deref(), "target-rate")?.map(Size),
        fail_under: rate(shared.fail_under.as_deref(), "fail-under")?.map(Size),
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
    let output = Output::resolve(&args.shared, global, env)?;
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
    let specs = webseed_args::collect(web_seeds, Some(&meta), env, webseed_args::no_network)?;
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
    use crate::test_support::{TorrentFixture, run_err, run_ok};
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

    #[test]
    fn the_subcommands_that_are_not_built_name_their_todo_entry() {
        let fixture = TorrentFixture::multi_file();
        for subcommand in ["leech", "seed", "swarm", "probe"] {
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
}
