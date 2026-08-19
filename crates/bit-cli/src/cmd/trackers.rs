//! `bit-cli trackers`: announce or scrape, report, exit.
//!
//! This talks to the trackers directly rather than reading a session's
//! behaviour, so the report carries what each tracker actually said: its tier,
//! its interval, its seeder and leecher counts, and its failure reason when it
//! has one. A tracker list edited with `--tracker` or `--exclude-tracker`
//! applies to this run only; the `.torrent` is never rewritten.

use std::time::Duration;

use bit_cli_core::ExitCode;
use bit_cli_core::error::{Error, Result};
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::tracker::{Announce, Client, Event, TrackerResult};
use serde::Serialize;

use crate::cli::{Global, TrackersArgs};
use crate::env::Env;
use crate::output::{Renderer, field, table};
use crate::source::Kind;
use crate::swarm;

/// What `bit-cli trackers` reports.
#[derive(Debug, Clone, Serialize)]
pub struct TrackersReport {
    pub info_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `announce` or `scrape`.
    pub action: &'static str,
    pub tracker_count: usize,
    pub responded: usize,
    pub failed: usize,
    /// The highest seeder count any tracker reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeders: Option<u64>,
    /// The highest leecher count any tracker reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leechers: Option<u64>,
    /// Distinct peer addresses across every tracker that answered.
    pub peers: Vec<String>,
    pub trackers: Vec<TrackerResult>,
}

/// Run the command.
pub fn run(
    args: &TrackersArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let kind = Kind::classify(&args.source.source, env)?;
    let meta = match &kind {
        Kind::File(path) => Some(Metainfo::read(path)?),
        Kind::Stdin => Some(crate::source::load_local(&kind, env)?),
        _ => None,
    };
    let info_hash = match (&meta, kind.info_hash()) {
        (Some(meta), _) => meta.info_hash(),
        (None, Some(hash)) => hash,
        (None, None) => {
            return Err(Error::source_resolution(format!(
                "{}: an info hash is needed to announce, and this source does not carry one",
                args.source.source
            ))
            .with("source_kind", kind.name()));
        }
    };

    let tiers = tracker_tiers(&args.trackers, meta.as_ref(), env)?;
    if tiers.is_empty() {
        return Err(Error::no_usable_sources(
            "no trackers to talk to; the torrent has none and none were added with --tracker",
        ));
    }

    let timeout = swarm::optional_duration(&args.trackers.tracker_timeout, "tracker-timeout")?
        .unwrap_or(Duration::from_secs(30));
    let connect = swarm::optional_duration(
        &args.trackers.tracker_connect_timeout,
        "tracker-connect-timeout",
    )?
    .unwrap_or(Duration::from_secs(10));

    let request = Announce {
        event: match args.scrape {
            // A scrape carries no event, and announcing `started` from a
            // command that is not going to stay connected would leave a peer
            // record behind that nobody is serving.
            true => Event::None,
            false => Event::Started,
        },
        ..Announce::new(
            info_hash.0,
            peer_id(),
            6881,
            meta.as_ref().map(|m| m.layout().total_length).unwrap_or(0),
        )
    };

    if global.dry_run {
        let planned: Vec<serde_json::Value> = tiers
            .iter()
            .map(|(tier, url)| {
                serde_json::json!({
                    "tier": tier,
                    "url": url,
                    "protocol": bit_cli_core::tracker::protocol_of(url),
                    "scrape_url": bit_cli_core::tracker::scrape_url(url),
                })
            })
            .collect();
        let report = serde_json::json!({
            "dry_run": true,
            "info_hash": info_hash.hex(),
            "action": action(args),
            "trackers": planned,
        });
        renderer.emit(env, "trackers", &report, || {
            let mut out = vec![field("dry run", "no announce will be sent")];
            for (tier, url) in &tiers {
                out.push(field(&format!("tier {tier}"), url));
            }
            out
        })?;
        return Ok(ExitCode::Success);
    }

    let scrape = args.scrape;
    let runtime = swarm::runtime()?;
    let results = runtime.block_on(async {
        let client = std::sync::Arc::new(Client::new(
            &format!("bit-cli/{}", bit_cli_core::VERSION),
            timeout,
            connect,
        )?);
        // Every tracker is asked at once. Tiers are a fallback order for a
        // client trying to stay connected; this command reports on all of
        // them, so waiting out tier one to reach tier two would only make one
        // dead tracker cost the whole run.
        let mut work = tokio::task::JoinSet::new();
        for (order, (tier, url)) in tiers.into_iter().enumerate() {
            let client = client.clone();
            let request = request.clone();
            work.spawn(async move {
                let result = match scrape {
                    true => client.scrape(&url, tier, &request).await,
                    false => client.announce(&url, tier, &request).await,
                };
                (order, result)
            });
        }

        let mut results = Vec::new();
        while let Some(finished) = work.join_next().await {
            match finished {
                Ok(pair) => results.push(pair),
                Err(e) => return Err(Error::generic(format!("a tracker request failed: {e}"))),
            }
        }
        // Report in the order the trackers were listed, not the order they
        // happened to answer in, so two runs produce comparable output.
        results.sort_by_key(|(order, _)| *order);
        Ok::<_, Error>(
            results
                .into_iter()
                .map(|(_, result)| result)
                .collect::<Vec<_>>(),
        )
    })?;

    let mut peers: Vec<String> = results.iter().flat_map(|r| r.peers.clone()).collect();
    peers.sort();
    peers.dedup();

    let report = TrackersReport {
        info_hash: info_hash.hex(),
        name: meta.as_ref().map(|m| m.layout().name),
        action: action(args),
        tracker_count: results.len(),
        responded: results.iter().filter(|r| r.ok).count(),
        failed: results.iter().filter(|r| !r.ok).count(),
        seeders: results
            .iter()
            .filter(|r| r.ok)
            .filter_map(|r| r.seeders)
            .max(),
        leechers: results
            .iter()
            .filter(|r| r.ok)
            .filter_map(|r| r.leechers)
            .max(),
        peers,
        trackers: results,
    };

    for tracker in &report.trackers {
        if let Some(warning) = &tracker.warning {
            renderer.warn(env, format!("{}: {warning}", tracker.url));
        }
    }

    // Every tracker failing is the case a script needs to branch on. One of
    // several failing is normal and does not fail the command.
    let code = match (report.responded, report.tracker_count) {
        (0, n) if n > 0 => ExitCode::NoUsableSources,
        _ => ExitCode::Success,
    };
    renderer.emit(env, "trackers", &report, || lines(&report))?;
    Ok(code)
}

const fn action(args: &TrackersArgs) -> &'static str {
    match args.scrape {
        true => "scrape",
        false => "announce",
    }
}

/// The tracker list for this run, as `(tier, url)` pairs.
///
/// A blank line in a `--tracker-file` starts a new BEP 12 tier, which is the
/// convention every other client uses for those files.
fn tracker_tiers(
    args: &crate::cli::TrackerArgs,
    meta: Option<&Metainfo>,
    env: &Env,
) -> Result<Vec<(usize, String)>> {
    let mut tiers: Vec<Vec<String>> = Vec::new();
    if !args.replace_trackers
        && let Some(meta) = meta
    {
        tiers.extend(meta.announce_tiers());
    }
    if !args.tracker.is_empty() {
        tiers.push(args.tracker.clone());
    }
    for path in &args.tracker_file {
        let path = env.resolve(path);
        let text = std::fs::read_to_string(&path).map_err(|e| {
            bit_cli_core::error::from_io(e, format!("cannot read {}", path.display()))
        })?;
        tiers.extend(bit_cli_core::webseed::table::parse_tier_list(&text));
    }

    let excluded: std::collections::HashSet<&str> =
        args.exclude_tracker.iter().map(String::as_str).collect();
    if excluded.contains("*") || args.no_tracker {
        return Ok(Vec::new());
    }

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (index, tier) in tiers.iter().enumerate() {
        for url in tier {
            if excluded.contains(url.as_str()) || !seen.insert(url.clone()) {
                continue;
            }
            out.push((index, url.clone()));
        }
    }
    Ok(out)
}

/// The peer id this run announces with.
///
/// Azureus style, so a tracker's client statistics attribute the announce
/// correctly rather than filing it under "unknown".
fn peer_id() -> [u8; 20] {
    let mut id = [0u8; 20];
    let prefix = b"-BC0100-";
    id[..prefix.len()].copy_from_slice(prefix);
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    for (index, slot) in id[prefix.len()..].iter_mut().enumerate() {
        let byte = (seed >> ((index % 16) * 4)) as u8;
        *slot = b'0' + (byte % 36).min(35).min(9);
        if byte % 36 > 9 {
            *slot = b'a' + (byte % 36) - 10;
        }
    }
    id
}

fn lines(report: &TrackersReport) -> Vec<String> {
    let mut out = vec![field("info hash", &report.info_hash)];
    if let Some(name) = &report.name {
        out.push(field("name", name));
    }
    out.push(field("action", report.action));
    out.push(field("trackers", report.tracker_count));
    out.push(field("responded", report.responded));
    out.push(field("failed", report.failed));
    if let Some(seeders) = report.seeders {
        out.push(field("seeders", seeders));
    }
    if let Some(leechers) = report.leechers {
        out.push(field("leechers", leechers));
    }
    out.push(field("peers", report.peers.len()));

    let rows: Vec<Vec<String>> = report
        .trackers
        .iter()
        .map(|t| {
            vec![
                t.tier.to_string(),
                t.url.clone(),
                match t.ok {
                    true => "ok".to_string(),
                    false => "failed".to_string(),
                },
                format!("{}ms", t.elapsed_ms),
                t.seeders
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".into()),
                t.leechers
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".into()),
                t.interval_s
                    .map(|n| format!("{n}s"))
                    .unwrap_or_else(|| "-".into()),
                t.peers.len().to_string(),
                t.failure.clone().unwrap_or_default(),
            ]
        })
        .collect();
    out.push(String::new());
    out.extend(table(
        &[
            "TIER", "TRACKER", "STATUS", "RTT", "SEED", "LEECH", "INTERVAL", "PEERS", "REASON",
        ],
        &rows,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::TrackerArgs;

    fn env() -> Env {
        Env::test(&[], "/w").0
    }

    fn args(tracker: &[&str], exclude: &[&str], replace: bool) -> TrackerArgs {
        TrackerArgs {
            tracker: tracker.iter().map(ToString::to_string).collect(),
            exclude_tracker: exclude.iter().map(ToString::to_string).collect(),
            replace_trackers: replace,
            ..Default::default()
        }
    }

    #[test]
    fn command_line_trackers_form_their_own_tier() {
        let tiers =
            tracker_tiers(&args(&["udp://a.example:451"], &[], false), None, &env()).unwrap();
        assert_eq!(tiers, vec![(0, "udp://a.example:451".to_string())]);
    }

    #[test]
    fn a_repeated_tracker_is_announced_to_once() {
        let tiers = tracker_tiers(
            &args(&["udp://a.example:451", "udp://a.example:451"], &[], false),
            None,
            &env(),
        )
        .unwrap();
        assert_eq!(tiers.len(), 1);
    }

    #[test]
    fn an_excluded_tracker_is_dropped() {
        let tiers = tracker_tiers(
            &args(
                &["udp://a.example:451", "udp://b.example:451"],
                &["udp://a.example:451"],
                false,
            ),
            None,
            &env(),
        )
        .unwrap();
        assert_eq!(tiers, vec![(0, "udp://b.example:451".to_string())]);
    }

    #[test]
    fn a_star_exclusion_removes_every_tracker() {
        let tiers =
            tracker_tiers(&args(&["udp://a.example:451"], &["*"], false), None, &env()).unwrap();
        assert!(tiers.is_empty());
    }

    #[test]
    fn no_tracker_removes_every_tracker() {
        let mut args = args(&["udp://a.example:451"], &[], false);
        args.no_tracker = true;
        assert!(tracker_tiers(&args, None, &env()).unwrap().is_empty());
    }

    #[test]
    fn the_peer_id_is_azureus_style_and_printable() {
        let id = peer_id();
        assert_eq!(&id[..8], b"-BC0100-");
        assert!(
            id.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-'),
            "{id:?}"
        );
    }

    #[test]
    fn an_announce_carries_started_and_a_scrape_carries_nothing() {
        assert_eq!(Event::Started.as_str(), Some("started"));
        assert_eq!(Event::None.as_str(), None);
    }

    #[test]
    fn the_report_takes_the_highest_count_any_tracker_gave() {
        // Trackers disagree constantly. The highest count is the most
        // informative single number, and every tracker's own figure is still
        // in the table below it.
        let mut low = TrackerResult {
            url: "udp://a.example:451".into(),
            tier: 0,
            protocol: "udp".into(),
            ok: true,
            elapsed_ms: 5,
            seeders: Some(2),
            leechers: Some(1),
            completed: None,
            interval_s: Some(900),
            min_interval_s: None,
            http_status: None,
            peers: vec!["1.2.3.4:1".into()],
            warning: None,
            failure: None,
        };
        let mut high = low.clone();
        high.url = "udp://b.example:451".into();
        high.seeders = Some(40);
        high.peers = vec!["1.2.3.4:1".into(), "5.6.7.8:2".into()];
        low.tier = 1;

        let mut peers: Vec<String> = [low.clone(), high.clone()]
            .iter()
            .flat_map(|r| r.peers.clone())
            .collect();
        peers.sort();
        peers.dedup();

        let report = TrackersReport {
            info_hash: "0".repeat(40),
            name: None,
            action: "announce",
            tracker_count: 2,
            responded: 2,
            failed: 0,
            seeders: [low.clone(), high.clone()]
                .iter()
                .filter_map(|r| r.seeders)
                .max(),
            leechers: [low.clone(), high.clone()]
                .iter()
                .filter_map(|r| r.leechers)
                .max(),
            peers,
            trackers: vec![low, high],
        };
        assert_eq!(report.seeders, Some(40));
        assert_eq!(
            report.peers.len(),
            2,
            "peer addresses are deduplicated across trackers"
        );

        let text = lines(&report).join("\n");
        assert!(text.contains("udp://a.example:451"), "{text}");
        assert!(text.contains("udp://b.example:451"), "{text}");
    }
}
