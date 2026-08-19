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
    let cli = match Cli::try_parse_from(&env.args) {
        Ok(cli) => cli,
        Err(err) => return report_parse_error(env, err),
    };

    let mut renderer = Renderer::new(&cli.global, env);

    if cli.global.schema_version {
        let _ = env.say(output::SCHEMA_VERSION);
        return ExitCode::Success;
    }

    if let Err(error) = logging::install(&cli.global, env) {
        renderer.fail(env, &error);
        return error.code();
    }

    match dispatch(&cli, &mut renderer, env) {
        Ok(code) => code,
        Err(error) => {
            renderer.fail(env, &error);
            error.code()
        }
    }
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
}
