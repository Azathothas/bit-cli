//! `bit-cli`: a non-interactive BitTorrent and HTTP download tool.
//!
//! The whole program is drivable in-process through [`run`], which takes an
//! [`env::Env`] rather than reading globals. That is what makes the headless
//! parity requirement testable: a test builds an `Env` with in-memory streams
//! and no terminal, runs the same code path a shell would, and asserts the
//! same results and the same exit code.
//!
//! Nothing here is TTY-gated. Terminal detection reaches exactly two
//! decisions, colour and progress rendering, and never decides what the
//! program does, computes, or reports.

pub mod cli;
pub mod cmd;
pub mod env;
pub mod logging;
pub mod output;
pub mod payload;
pub mod schema;
#[cfg(test)]
mod schema_gen;
pub mod selection;
pub mod source;
pub mod swarm;

#[cfg(test)]
mod test_support;
pub mod webseed_args;

use bit_cli_core::error::Error;
use bit_cli_core::exit::ExitCode;
use clap::Parser;

use crate::cli::{Cli, Command};
use crate::env::Env;
use crate::output::Renderer;

/// Run the program and return the exit code.
///
/// This never panics on a user error and never writes anything to a stream the
/// caller did not supply.
pub fn run(env: &mut Env) -> ExitCode {
    let started = std::time::Instant::now();
    let cli = match Cli::try_parse_from(&env.args) {
        Ok(cli) => cli,
        // Before the flags are parsed there is no format to emit an event in,
        // so a usage error ends the stream by ending it.
        Err(err) => return report_parse_error(env, err),
    };

    let mut renderer = Renderer::new(&cli.global, env);

    if cli.global.schema_version {
        let _ = env.say(output::SCHEMA_VERSION);
        return ExitCode::Success;
    }

    if let Err(error) = logging::install(&cli.global, env) {
        renderer.fail(env, &error);
        return end_session(&mut renderer, env, error.code(), started, Some(&error));
    }

    let (code, error) = match dispatch(&cli, &mut renderer, env) {
        Ok(code) => (code, None),
        Err(error) => {
            renderer.fail(env, &error);
            (error.code(), Some(error))
        }
    };
    end_session(&mut renderer, env, code, started, error.as_ref())
}

/// Close the `--jsonl` stream with the event that says it closed.
///
/// Emitted here rather than per command, from the one place every run returns
/// through, so it cannot be forgotten by a command that is added later. An
/// agent reading NDJSON otherwise cannot tell "finished" from "the pipe
/// broke". See `TODO/cli-surface.md`, T-110.
fn end_session(
    renderer: &mut Renderer,
    env: &mut Env,
    code: ExitCode,
    started: std::time::Instant,
    error: Option<&Error>,
) -> ExitCode {
    let elapsed = started.elapsed();
    let mut payload = serde_json::json!({
        "exit_code": code.code(),
        "exit_status": code.kind(),
        "ok": code == ExitCode::Success,
        "elapsed_ms": elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
        "elapsed_human": bit_cli_core::units::format_duration(elapsed),
    });
    if let Some(error) = error
        && let Some(object) = payload.as_object_mut()
    {
        object.insert("error".into(), serde_json::Value::String(error.to_string()));
    }
    // A stream that cannot be written is not a reason to change the exit code:
    // the run already happened, and the caller's pipe closing is theirs.
    let _ = renderer.event(env, "session_end", &payload);
    code
}

fn dispatch(cli: &Cli, renderer: &mut Renderer, env: &mut Env) -> Result<ExitCode, Error> {
    match &cli.command {
        Some(Command::Info(args)) => cmd::info::run(args, &cli.global, renderer, env),
        Some(Command::Files(args)) => cmd::files::run(args, &cli.global, renderer, env),
        Some(Command::Magnet(args)) => cmd::magnet::run(args, &cli.global, renderer, env),
        Some(Command::Verify(args)) => cmd::verify::run(args, &cli.global, renderer, env),
        Some(Command::Create(args)) => cmd::create::run(args, &cli.global, renderer, env),
        Some(Command::Edit(args)) => cmd::edit::run(args, &cli.global, renderer, env),
        Some(Command::Webseed(args)) => cmd::webseed::run(args, &cli.global, renderer, env),
        Some(Command::Config(args)) => cmd::config::run(args, &cli.global, renderer, env),
        Some(Command::Completions(args)) => cmd::completions::run(args, env),
        Some(Command::Man(args)) => cmd::man::run(args, env),
        Some(Command::Version) => cmd::version::run(renderer, env),
        Some(Command::Download(args)) => cmd::download::run(args, &cli.global, renderer, env),
        Some(Command::Seed(args)) => cmd::seed::run(args, &cli.global, renderer, env),
        Some(Command::Peers(args)) => cmd::peers::run(args, &cli.global, renderer, env),
        Some(Command::Trackers(args)) => cmd::trackers::run(args, &cli.global, renderer, env),
        Some(Command::Bench(args)) => cmd::bench::run(args, &cli.global, renderer, env),
        // `bit-cli <SOURCE>` is `bit-cli download <SOURCE>`.
        None if !cli.sources.is_empty() => {
            let args = cli::DownloadArgs::from_sources(cli.sources.clone());
            cmd::download::run(&args, &cli.global, renderer, env)
        }
        None => {
            let _ = env.note("no source given. Run `bit-cli --help`.");
            Ok(ExitCode::Usage)
        }
    }
}

/// Turn a `clap` parse failure into an exit code.
///
/// `--help` and `--version` come back from `clap` as errors but are successful
/// requests, so they print to stdout and exit zero.
fn report_parse_error(env: &mut Env, err: clap::Error) -> ExitCode {
    use clap::error::ErrorKind;
    match err.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            let _ = env.say(err.render().ansi().to_string().trim_end());
            ExitCode::Success
        }
        _ => {
            let _ = env.note(err.render().ansi().to_string().trim_end());
            ExitCode::Usage
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_goes_to_stdout_and_exits_zero() {
        let (mut env, captured) = Env::test(&["--help"], "/w");
        assert_eq!(run(&mut env), ExitCode::Success);
        assert!(captured.out().contains("web seed"), "{}", captured.out());
        assert_eq!(captured.err(), "");
    }

    #[test]
    fn a_bad_flag_goes_to_stderr_and_exits_two() {
        let (mut env, captured) = Env::test(&["--nope"], "/w");
        assert_eq!(run(&mut env), ExitCode::Usage);
        assert_eq!(captured.out(), "", "stdout carries data only");
        assert!(captured.err().contains("--nope"));
    }

    #[test]
    fn no_arguments_at_all_is_a_usage_error() {
        let (mut env, captured) = Env::test(&[], "/w");
        assert_eq!(run(&mut env), ExitCode::Usage);
        assert_eq!(captured.out(), "");
    }

    #[test]
    fn schema_version_prints_and_exits() {
        let (mut env, captured) = Env::test(&["--schema-version"], "/w");
        assert_eq!(run(&mut env), ExitCode::Success);
        assert_eq!(captured.out().trim(), output::SCHEMA_VERSION);
    }

    #[test]
    fn version_reports_the_build() {
        let (mut env, captured) = Env::test(&["version", "--json"], "/w");
        assert_eq!(run(&mut env), ExitCode::Success);
        let doc = captured.json().unwrap();
        assert_eq!(doc["version"], bit_cli_core::VERSION);
        assert!(doc["exit_codes"].as_array().unwrap().len() >= 16);
    }

    #[test]
    fn every_subcommand_help_renders() {
        for sub in [
            "download",
            "info",
            "files",
            "peers",
            "trackers",
            "webseed",
            "verify",
            "create",
            "edit",
            "magnet",
            "seed",
            "bench",
            "config",
            "completions",
            "man",
            "version",
        ] {
            let (mut env, captured) = Env::test(&[sub, "--help"], "/w");
            assert_eq!(run(&mut env), ExitCode::Success, "{sub} --help failed");
            assert!(!captured.out().is_empty(), "{sub} --help printed nothing");
        }
    }

    /// Every `--jsonl` run ends with the event that says it ended.
    ///
    /// An agent reading NDJSON cannot otherwise tell "finished" from "the pipe
    /// broke". It is emitted from `run` rather than per command, so a command
    /// added later cannot forget it, and this test walks every command that
    /// can be driven with no network to prove it. See `TODO/cli-surface.md`,
    /// T-110.
    #[test]
    fn every_jsonl_run_ends_with_session_end() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let torrent = fixture.path_str().to_string();
        let cases: Vec<Vec<&str>> = vec![
            vec!["--jsonl", "info", &torrent],
            vec!["--jsonl", "files", &torrent],
            vec!["--jsonl", "magnet", &torrent],
            vec!["--jsonl", "version"],
            vec!["--jsonl", "config", "show"],
            vec![
                "--jsonl",
                "webseed",
                "list",
                &torrent,
                "--web-seed",
                "https://e.example/",
            ],
        ];
        for args in cases {
            let (mut env, captured) = Env::test(&args, fixture.dir());
            let code = run(&mut env);
            let events = captured.jsonl().unwrap_or_else(|e| {
                panic!("{args:?} did not write ndjson: {e}\n{}", captured.out())
            });
            let last = events
                .last()
                .unwrap_or_else(|| panic!("{args:?} wrote nothing"));
            assert_eq!(last["type"], "session_end", "{args:?} ended with {last}");
            assert_eq!(last["exit_code"], code.code(), "{args:?}");
            assert!(last["elapsed_ms"].is_number(), "{args:?}");
            assert!(last["at"].is_string(), "{args:?}");
        }
    }

    /// A failure ends the stream too, with the code and the reason.
    #[test]
    fn a_failed_jsonl_run_ends_with_session_end_carrying_the_error() {
        let (mut env, captured) = Env::test(&["--jsonl", "info", "nope.torrent"], "/w");
        let code = run(&mut env);
        assert_ne!(code, ExitCode::Success);
        let events = captured.jsonl().unwrap();
        let last = events.last().expect("an event");
        assert_eq!(last["type"], "session_end");
        assert_eq!(last["ok"], false);
        assert_eq!(last["exit_code"], code.code());
        assert!(
            last["error"]
                .as_str()
                .unwrap_or_default()
                .contains("nope.torrent"),
            "{last}"
        );
    }

    /// The event is a `--jsonl` surface only. `--json` carries one document
    /// and text carries lines, and neither gains a stray object at the end.
    #[test]
    fn session_end_does_not_appear_outside_jsonl() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        for flag in ["--json", "--quiet"] {
            let (mut env, captured) = Env::test(&[flag, "info", fixture.path_str()], fixture.dir());
            assert_eq!(run(&mut env), ExitCode::Success);
            assert!(
                !captured.out().contains("session_end"),
                "{flag} leaked an event: {}",
                captured.out()
            );
        }
    }
}
