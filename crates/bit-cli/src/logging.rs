//! Logging and subsystem tracing.
//!
//! Levels are for severity, subsystem traces are for detail. Turning on
//! `trace` globally in a torrent client buries the thing you are looking for
//! under peer chatter, so `--trace http` raises exactly one subsystem and
//! leaves the rest alone.
//!
//! Logs always go to stderr. A caller doing `bit-cli ... --json | jq` must
//! never see a log line in the pipe, and that holds at every level.

use std::collections::BTreeSet;

use bit_cli_core::error::{Error, Result};
use tracing_subscriber::EnvFilter;

use crate::cli::{Global, LogFormat};
use crate::env::Env;

/// Subsystems that can be traced independently.
pub const SUBSYSTEMS: &[(&str, &str)] = &[
    (
        "peer",
        "Wire messages: type, index, begin, length, direction, peer id",
    ),
    ("handshake", "Peer handshakes and extension negotiation"),
    (
        "tracker",
        "Announce and scrape requests and responses in full",
    ),
    ("dht", "DHT queries, responses, and routing table changes"),
    (
        "http",
        "Web seed requests and responses, status, headers, ranges, redirects, TLS",
    ),
    (
        "piece",
        "Piece request, receipt, verification result, and timing",
    ),
    ("picker", "Why a piece was requested from a given source"),
    (
        "disk",
        "Reads, writes, flushes, and allocation, with offsets and sizes",
    ),
    ("ratelimit", "Token bucket decisions and stalls"),
    ("retry", "Retry attempts, backoff, and cooldown"),
    (
        "config",
        "Resolution of every configuration value and its origin",
    ),
];

/// Check a subsystem name.
pub fn parse_subsystem(name: &str) -> Result<&'static str> {
    let name = name.trim();
    SUBSYSTEMS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(known, _)| *known)
        .ok_or_else(|| {
            let known: Vec<&str> = SUBSYSTEMS.iter().map(|(n, _)| *n).collect();
            Error::usage(format!(
                "`{name}` is not a trace subsystem (known: {})",
                known.join(", ")
            ))
            .with("subsystem", name.to_string())
        })
}

/// Build the `tracing` filter directive for the given flags.
///
/// The global level applies to everything, then each traced subsystem is
/// raised to `trace` on its own target. The result is one directive string,
/// which is exactly what `EnvFilter` takes.
pub fn filter_directive(global: &Global) -> Result<String> {
    let level = global.log_level.raised(global.verbose);
    let mut parts = vec![level.directive().to_string()];
    let mut seen = BTreeSet::new();
    for requested in &global.trace {
        let subsystem = parse_subsystem(requested)?;
        if seen.insert(subsystem) {
            parts.push(format!("bit_cli::{subsystem}=trace"));
        }
    }
    Ok(parts.join(","))
}

/// Install the log subscriber.
///
/// Installation is best-effort by design: a second call in the same process is
/// a no-op rather than an error, so the in-process test harness can run many
/// commands without each one fighting over the global subscriber.
pub fn install(global: &Global, env: &Env) -> Result<()> {
    // Validate the subsystem names even when nothing will be installed, so a
    // typo in --trace is reported rather than silently ignored.
    let directive = filter_directive(global)?;
    if global.log_file.is_some() {
        // A log file is a real file handle with rotation, which the in-process
        // harness must not create as a side effect of a unit test.
        bit_cli_core::units::parse_size(&global.log_max_size)
            .map_err(|e| Error::config(format!("--log-max-size: {e}")))?;
    }

    let filter = EnvFilter::try_new(&directive)
        .map_err(|e| Error::config(format!("cannot build a log filter from `{directive}`: {e}")))?;
    let ansi = env.err_is_terminal && env.wants_color(global.color.into());

    // Logs go to stderr at every level. stdout carries data only.
    let result = match global.log_format {
        LogFormat::Json => tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .try_init(),
        LogFormat::Text => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_ansi(ansi)
            .try_init(),
    };
    // An already-installed subscriber is not a failure worth stopping for.
    let _ = result;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Command};
    use clap::Parser;

    fn global(args: &[&str]) -> Global {
        let mut full = vec!["bit-cli"];
        full.extend_from_slice(args);
        full.extend_from_slice(&["info", "x.torrent"]);
        let cli = Cli::try_parse_from(full).unwrap();
        assert!(matches!(cli.command, Some(Command::Info(_))));
        cli.global
    }

    #[test]
    fn the_default_level_is_warn() {
        assert_eq!(filter_directive(&global(&[])).unwrap(), "warn");
    }

    #[test]
    fn verbosity_raises_the_level() {
        assert_eq!(filter_directive(&global(&["-v"])).unwrap(), "info");
        assert_eq!(filter_directive(&global(&["-vv"])).unwrap(), "debug");
        assert_eq!(filter_directive(&global(&["-vvv"])).unwrap(), "trace");
        assert_eq!(filter_directive(&global(&["-vvvvvv"])).unwrap(), "trace");
    }

    #[test]
    fn an_explicit_level_is_honoured() {
        assert_eq!(
            filter_directive(&global(&["--log-level", "error"])).unwrap(),
            "error"
        );
        assert_eq!(
            filter_directive(&global(&["--log-level", "off"])).unwrap(),
            "off"
        );
    }

    #[test]
    fn a_traced_subsystem_is_raised_without_raising_everything() {
        let directive = filter_directive(&global(&["--trace", "http"])).unwrap();
        assert_eq!(directive, "warn,bit_cli::http=trace");
    }

    #[test]
    fn several_subsystems_can_be_traced_at_once() {
        let directive = filter_directive(&global(&["--trace", "http,piece,picker"])).unwrap();
        assert_eq!(
            directive,
            "warn,bit_cli::http=trace,bit_cli::piece=trace,bit_cli::picker=trace"
        );
    }

    #[test]
    fn a_repeated_subsystem_appears_once() {
        let directive = filter_directive(&global(&["--trace", "http", "--trace", "http"])).unwrap();
        assert_eq!(directive, "warn,bit_cli::http=trace");
    }

    #[test]
    fn an_unknown_subsystem_is_refused_with_the_list() {
        let err = filter_directive(&global(&["--trace", "nope"])).unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::Usage);
        assert!(err.message().contains("http"), "{}", err.message());
    }

    #[test]
    fn every_directive_builds_a_real_filter() {
        for args in [
            vec![],
            vec!["-vvv"],
            vec!["--log-level", "debug", "--trace", "http"],
            vec![
                "--trace",
                "peer,handshake,tracker,dht,http,piece,picker,disk,ratelimit,retry,config",
            ],
        ] {
            let directive = filter_directive(&global(&args)).unwrap();
            EnvFilter::try_new(&directive)
                .unwrap_or_else(|e| panic!("{directive} is not a valid filter: {e}"));
        }
    }

    #[test]
    fn every_subsystem_is_documented_and_uniquely_named() {
        let names: BTreeSet<&str> = SUBSYSTEMS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), SUBSYSTEMS.len());
        for (name, description) in SUBSYSTEMS {
            assert!(!description.is_empty(), "{name} has no description");
            assert_eq!(parse_subsystem(name).unwrap(), *name);
        }
    }

    #[test]
    fn a_bad_log_size_is_reported_as_a_config_error() {
        let g = global(&["--log-file", "x.log", "--log-max-size", "4 potatoes"]);
        let (env, _) = Env::test(&[], "/w");
        let err = install(&g, &env).unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::Config);
    }
}
