//! Turning the `--web-seed*` flags into source specs.
//!
//! Sources arrive from six places: the command line, a URL list file, a URL
//! list fetched over HTTP, a binding table, the torrent's own `url-list`
//! (BEP 19), and its `httpseeds` (BEP 17). All six are merged here, and each
//! keeps the origin it came from so `--json` can report which set is which and
//! `--no-torrent-web-seed` can drop exactly one of them.
//!
//! Nothing in this module touches the torrent. Web seeds attach at runtime;
//! the `.torrent` is never rewritten, never re-hashed, and the info hash never
//! changes.

use std::collections::BTreeMap;

use bit_cli_core::error::{Context, Error, Result, from_io};
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{parse_duration_ms, parse_rate, parse_size};
use bit_cli_core::webseed::binding::{Auth, Origin, SourceLimits, SourceSpec};
use bit_cli_core::webseed::table::{Table, parse_url_list};
use bit_cli_core::webseed::{Mode, Scope};

use crate::cli::WebSeedArgs;
use crate::env::Env;

/// Everything the flags say about how CLI-supplied sources behave.
struct Shared {
    mode: Mode,
    template: Option<String>,
    scope: Scope,
    limits: SourceLimits,
    headers: BTreeMap<String, String>,
    user_agent: Option<String>,
    auth: Auth,
    priority: i32,
    style: bit_cli_core::webseed::Style,
}

impl Shared {
    fn from(args: &WebSeedArgs) -> Result<Self> {
        let base = SourceLimits::default();
        let size = |value: &Option<String>, fallback: u64, what: &str| -> Result<u64> {
            match value {
                None => Ok(fallback),
                Some(text) => parse_size(text).map_err(|e| {
                    Error::usage(format!("--{what}: {e}")).with("value", text.clone())
                }),
            }
        };
        let duration = |value: &Option<String>, fallback: u64, what: &str| -> Result<u64> {
            match value {
                None => Ok(fallback),
                Some(text) => parse_duration_ms(text).map_err(|e| {
                    Error::usage(format!("--{what}: {e}")).with("value", text.clone())
                }),
            }
        };

        // The piece and byte restrictions are two spellings of one scope, so
        // asking for both would need a rule about which wins. There is no
        // useful answer, so it is a usage error instead.
        let scope = match (&args.web_seed_pieces, &args.web_seed_bytes) {
            (Some(_), Some(_)) => {
                return Err(Error::usage(
                    "--web-seed-pieces and --web-seed-bytes both restrict the same sources; use one, or --web-seed-for for per-source scopes",
                ));
            }
            (Some(pieces), None) => {
                Scope::parse(&format!("piece:{}", pieces.trim_start_matches("piece:")))?
            }
            (None, Some(bytes)) => {
                Scope::parse(&format!("byte:{}", bytes.trim_start_matches("byte:")))?
            }
            (None, None) => Scope::all(),
        };

        let mut headers = BTreeMap::new();
        for raw in &args.web_seed_header {
            let (name, value) = raw.split_once(':').ok_or_else(|| {
                Error::usage(format!("--web-seed-header `{raw}` is not `Name: value`"))
                    .with("value", raw.clone())
            })?;
            headers.insert(name.trim().to_string(), value.trim().to_string());
        }

        Ok(Self {
            mode: args.web_seed_mode.into(),
            template: args.web_seed_template.clone(),
            scope,
            limits: SourceLimits {
                concurrency: args.web_seed_concurrency.unwrap_or(base.concurrency).max(1),
                chunk_size: size(
                    &args.web_seed_chunk_size,
                    base.chunk_size,
                    "web-seed-chunk-size",
                )?
                .max(1),
                timeout_ms: duration(&args.web_seed_timeout, base.timeout_ms, "web-seed-timeout")?,
                connect_timeout_ms: duration(
                    &args.web_seed_connect_timeout,
                    base.connect_timeout_ms,
                    "web-seed-connect-timeout",
                )?,
                retries: args.web_seed_retries.unwrap_or(base.retries),
                max_errors: args.web_seed_max_errors.unwrap_or(base.max_errors).max(1),
                cooldown_ms: duration(
                    &args.web_seed_cooldown,
                    base.cooldown_ms,
                    "web-seed-cooldown",
                )?,
                rate_limit: match &args.web_seed_speed_limit {
                    None => None,
                    Some(text) => Some(parse_rate(text).map_err(|e| {
                        Error::usage(format!("--web-seed-speed-limit: {e}"))
                            .with("value", text.clone())
                    })?),
                },
            },
            headers,
            user_agent: args.web_seed_user_agent.clone(),
            auth: match &args.web_seed_auth {
                None => Auth::None,
                Some(spec) => Auth::parse(spec)?,
            },
            priority: args.web_seed_priority.unwrap_or(0),
            style: args.web_seed_style.into(),
        })
    }

    fn spec(&self, url: String, origin: Origin, scope: Scope, mode: Mode) -> SourceSpec {
        SourceSpec {
            url,
            scope,
            mode,
            template: self.template.clone(),
            style: self.style,
            priority: self.priority,
            headers: self.headers.clone(),
            user_agent: self.user_agent.clone(),
            auth: self.auth.clone(),
            limits: self.limits.clone(),
            origin,
        }
    }
}

/// Build every source for one torrent, in priority-tie order.
///
/// `fetch_list` fetches a `--web-seed-list-url`. It is a parameter so the
/// assembly is testable without a network.
pub fn collect(
    args: &WebSeedArgs,
    meta: Option<&Metainfo>,
    env: &Env,
    fetch_list: impl Fn(&str) -> Result<String>,
) -> Result<Vec<SourceSpec>> {
    if args.no_web_seed {
        return Ok(Vec::new());
    }
    let shared = Shared::from(args)?;
    let mut specs = Vec::new();

    // The torrent's own sources come first, so a caller-supplied source with
    // an equal priority is tried after them only if it was written later. The
    // caller controls that with --web-seed-priority.
    if !args.no_torrent_web_seed
        && let Some(meta) = meta
    {
        for url in meta.url_list() {
            specs.push(shared.spec(url, Origin::TorrentUrlList, Scope::all(), Mode::Auto));
        }
        for url in meta.http_seeds() {
            let mut spec = shared.spec(url, Origin::TorrentHttpSeeds, Scope::all(), Mode::Auto);
            spec.style = bit_cli_core::webseed::Style::Hoffman;
            specs.push(spec);
        }
    }

    for url in &args.web_seed {
        specs.push(shared.spec(
            url.clone(),
            Origin::CommandLine,
            shared.scope.clone(),
            shared.mode,
        ));
    }
    for url in &args.web_seed_exact {
        specs.push(shared.spec(
            url.clone(),
            Origin::CommandLine,
            shared.scope.clone(),
            Mode::Exact,
        ));
    }
    for binding in &args.web_seed_for {
        let (selector, url) = binding.split_once('=').ok_or_else(|| {
            Error::usage(format!("--web-seed-for `{binding}` is not `SELECTOR=URL`"))
                .with("value", binding.clone())
        })?;
        let scope =
            Scope::parse(selector).with_context(|| format!("--web-seed-for `{binding}`"))?;
        specs.push(shared.spec(url.to_string(), Origin::CommandLine, scope, shared.mode));
    }

    for path in &args.web_seed_file {
        let path = env.resolve(path);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| from_io(e, format!("cannot read {}", path.display())))?;
        for url in parse_url_list(&text) {
            specs.push(shared.spec(url, Origin::File, shared.scope.clone(), shared.mode));
        }
    }
    for url in &args.web_seed_list_url {
        let text = fetch_list(url)?;
        for entry in parse_url_list(&text) {
            specs.push(shared.spec(entry, Origin::ListUrl, shared.scope.clone(), shared.mode));
        }
    }

    // The binding table comes last so its per-source settings are not
    // overwritten by the shared flags, which is the whole reason to use one.
    for path in &args.web_seed_config {
        let path = env.resolve(path);
        let table = Table::load(&path)?;
        specs.extend(table.into_specs(Origin::Config)?);
    }

    if specs.is_empty() && args.web_seed_only {
        return Err(Error::no_usable_sources(
            "--web-seed-only was given but no web seed sources were declared",
        ));
    }
    Ok(specs)
}

/// Refuse to fetch a list over HTTP when nothing was asked for.
///
/// Commands that must not touch the network pass this, so a
/// `--web-seed-list-url` on a no-network command fails clearly instead of
/// quietly reaching out.
pub fn no_network(url: &str) -> Result<String> {
    Err(Error::usage(format!(
        "--web-seed-list-url {url} needs the network, and this command does not use it"
    ))
    .with("url", url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    use crate::cli::{Cli, Command};

    fn args(extra: &[&str]) -> WebSeedArgs {
        let mut full = vec!["bit-cli", "webseed", "list"];
        full.extend_from_slice(extra);
        full.push("x.torrent");
        let cli = Cli::try_parse_from(full).unwrap();
        let Some(Command::Webseed(crate::cli::WebseedCommand::List(list))) = cli.command else {
            panic!("expected webseed list");
        };
        list.web_seeds
    }

    fn env() -> Env {
        Env::test(&[], "/w").0
    }

    fn collect_ok(extra: &[&str]) -> Vec<SourceSpec> {
        collect(&args(extra), None, &env(), no_network).unwrap()
    }

    #[test]
    fn a_plain_web_seed_becomes_one_source() {
        let specs = collect_ok(&["--web-seed", "https://a.example.com/pub/"]);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].url, "https://a.example.com/pub/");
        assert_eq!(specs[0].mode, Mode::Auto);
        assert_eq!(specs[0].origin, Origin::CommandLine);
        assert!(specs[0].scope.is_all());
    }

    #[test]
    fn web_seed_exact_sets_the_composition_mode_regardless_of_the_shared_one() {
        let specs = collect_ok(&[
            "--web-seed-mode",
            "prefix",
            "--web-seed",
            "https://a.example.com/pub/",
            "--web-seed-exact",
            "https://cdn.example.com/blob",
        ]);
        assert_eq!(specs[0].mode, Mode::Prefix);
        assert_eq!(specs[1].mode, Mode::Exact);
    }

    #[test]
    fn web_seed_for_binds_a_scope_to_a_source() {
        let specs = collect_ok(&["--web-seed-for", "piece:0-511=https://b.example.com/"]);
        assert_eq!(specs[0].url, "https://b.example.com/");
        assert_eq!(specs[0].scope.text(), "piece:0-511");
    }

    #[test]
    fn a_malformed_web_seed_for_says_what_it_wanted() {
        let err = collect(
            &args(&["--web-seed-for", "no-equals-sign"]),
            None,
            &env(),
            no_network,
        )
        .unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::Usage);
        assert!(err.message().contains("SELECTOR=URL"), "{}", err.message());
    }

    #[test]
    fn a_bad_selector_in_web_seed_for_names_the_binding() {
        let err = collect(
            &args(&["--web-seed-for", "piece:9-2=https://b.example.com/"]),
            None,
            &env(),
            no_network,
        )
        .unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::Binding);
        assert!(err.to_string().contains("piece:9-2"), "{err}");
    }

    #[test]
    fn piece_and_byte_restrictions_apply_to_cli_sources() {
        let specs = collect_ok(&[
            "--web-seed-pieces",
            "0-511",
            "--web-seed",
            "https://a.example.com/",
        ]);
        assert_eq!(specs[0].scope.text(), "piece:0-511");

        let specs = collect_ok(&[
            "--web-seed-bytes",
            "0-1MiB",
            "--web-seed",
            "https://a.example.com/",
        ]);
        assert_eq!(specs[0].scope.text(), "byte:0-1MiB");
    }

    #[test]
    fn asking_for_both_restrictions_at_once_is_a_usage_error() {
        let err = collect(
            &args(&["--web-seed-pieces", "0-1", "--web-seed-bytes", "0-1MiB"]),
            None,
            &env(),
            no_network,
        )
        .unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::Usage);
    }

    #[test]
    fn headers_auth_and_limits_reach_every_cli_source() {
        let specs = collect_ok(&[
            "--web-seed",
            "https://a.example.com/",
            "--web-seed",
            "https://b.example.com/",
            "--web-seed-header",
            "X-Region: apac",
            "--web-seed-header",
            "X-Trace:on",
            "--web-seed-auth",
            "bearer:tok",
            "--web-seed-concurrency",
            "12",
            "--web-seed-chunk-size",
            "8MiB",
            "--web-seed-timeout",
            "45s",
            "--web-seed-speed-limit",
            "5MiB/s",
            "--web-seed-priority",
            "7",
        ]);
        assert_eq!(specs.len(), 2);
        for spec in &specs {
            assert_eq!(spec.headers["X-Region"], "apac");
            assert_eq!(spec.headers["X-Trace"], "on");
            assert_eq!(
                spec.auth,
                Auth::Bearer {
                    token: "tok".into()
                }
            );
            assert_eq!(spec.limits.concurrency, 12);
            assert_eq!(spec.limits.chunk_size, 8 * bit_cli_core::units::MIB);
            assert_eq!(spec.limits.timeout_ms, 45_000);
            assert_eq!(spec.limits.rate_limit, Some(5 * bit_cli_core::units::MIB));
            assert_eq!(spec.priority, 7);
        }
    }

    #[test]
    fn a_malformed_header_is_a_usage_error() {
        let err = collect(
            &args(&["--web-seed-header", "no-colon"]),
            None,
            &env(),
            no_network,
        )
        .unwrap_err();
        assert!(err.message().contains("Name: value"), "{}", err.message());
    }

    #[test]
    fn a_url_list_file_contributes_sources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mirrors.txt");
        std::fs::write(
            &path,
            "# mirrors\nhttps://a.example.com/\n\nhttps://b.example.com/\n",
        )
        .unwrap();
        let mut env = env();
        env.cwd = dir.path().to_path_buf();
        let specs = collect(
            &args(&["--web-seed-file", "mirrors.txt"]),
            None,
            &env,
            no_network,
        )
        .unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].origin, Origin::File);
    }

    #[test]
    fn a_list_url_is_fetched_through_the_injected_fetcher() {
        let specs = collect(
            &args(&["--web-seed-list-url", "https://e.com/mirrors.txt"]),
            None,
            &env(),
            |_| Ok("https://a.example.com/\nhttps://b.example.com/\n".to_string()),
        )
        .unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].origin, Origin::ListUrl);
    }

    #[test]
    fn a_list_url_on_a_no_network_command_fails_clearly() {
        let err = collect(
            &args(&["--web-seed-list-url", "https://e.com/mirrors.txt"]),
            None,
            &env(),
            no_network,
        )
        .unwrap_err();
        assert!(
            err.message().contains("needs the network"),
            "{}",
            err.message()
        );
    }

    #[test]
    fn a_binding_table_contributes_its_own_sources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("seeds.toml");
        std::fs::write(
            &path,
            "[[source]]\nurl = \"https://a.example.com/\"\nscope = \"piece:0-1\"\npriority = 9\n",
        )
        .unwrap();
        let mut env = env();
        env.cwd = dir.path().to_path_buf();
        let specs = collect(
            &args(&["--web-seed-config", "seeds.toml"]),
            None,
            &env,
            no_network,
        )
        .unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].origin, Origin::Config);
        assert_eq!(specs[0].priority, 9);
        assert_eq!(specs[0].scope.text(), "piece:0-1");
    }

    #[test]
    fn no_web_seed_drops_everything() {
        let specs = collect_ok(&["--no-web-seed", "--web-seed", "https://a.example.com/"]);
        assert!(specs.is_empty());
    }

    #[test]
    fn web_seed_only_with_nothing_declared_is_refused() {
        let err = collect(&args(&["--web-seed-only"]), None, &env(), no_network).unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::NoUsableSources);
    }

    #[test]
    fn sources_keep_the_origin_they_came_from() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m.txt"), "https://f.example.com/\n").unwrap();
        let mut env = env();
        env.cwd = dir.path().to_path_buf();
        let specs = collect(
            &args(&[
                "--web-seed",
                "https://c.example.com/",
                "--web-seed-file",
                "m.txt",
            ]),
            None,
            &env,
            no_network,
        )
        .unwrap();
        let origins: Vec<Origin> = specs.iter().map(|s| s.origin).collect();
        assert_eq!(origins, [Origin::CommandLine, Origin::File]);
    }
}
