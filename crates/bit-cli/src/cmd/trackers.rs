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
use bit_cli_core::tracker::{Announce, Client, Event, Family, TrackerResult};
use serde::Serialize;

use crate::cli::{AnnounceFamily, Global, TrackersArgs};
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
    /// The port announced, which is a port this command held open for the
    /// length of the announce. Absent on a scrape, which carries none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announced_port: Option<u16>,
    /// Trackers that accepted the `stopped` announce withdrawing the peer
    /// record this command's own announce created. Absent when
    /// `--no-withdraw` left it in place, and on a scrape.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawn: Option<usize>,
    pub tracker_count: usize,
    pub responded: usize,
    pub failed: usize,
    /// Exchanges actually sent, which is more than `tracker_count` when a
    /// tracker was announced to over both families.
    pub announces: usize,
    /// The highest seeder count any tracker reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeders: Option<u64>,
    /// The highest leecher count any tracker reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leechers: Option<u64>,
    /// Distinct peer addresses across every tracker that answered.
    pub peers: Vec<String>,
    /// What each address family's announces returned, separately. Empty on a
    /// scrape, which registers nothing and so needs only one exchange.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub families: Vec<FamilyResult>,
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

    // A port nothing is listening on registers a peer nobody can dial, which
    // is worse than useless on a public tracker: it is handed out to every
    // client that asks for the next hour. So the announce binds one for as
    // long as it lasts, and the run withdraws it afterwards. A scrape carries
    // no port and no event, so it binds nothing.
    //
    // See `TODO/trackers.md`, T-061.
    let listeners = match args.scrape {
        true => Vec::new(),
        false => bind_announce_port(&args.port)?,
    };
    let announced_port = listeners
        .first()
        .and_then(|socket| socket.local_addr().ok())
        .map(|addr| addr.port())
        .unwrap_or(0);

    let request = Announce {
        event: match args.scrape {
            // A scrape carries no event.
            true => Event::None,
            false => Event::Started,
        },
        ..Announce::new(
            info_hash.0,
            peer_id(),
            announced_port,
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
    let wanted = args.family;
    let withdraw = !args.scrape && !args.no_withdraw;
    let runtime = swarm::runtime()?;
    let (results, withdrawn) = runtime.block_on(async {
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
            // A scrape carries no peer record, so there is nothing for a
            // second family to register and one exchange answers the question.
            let families: Vec<Option<Family>> = match scrape {
                true => vec![None],
                false => announce_families(&url, wanted),
            };
            let client = client.clone();
            let request = request.clone();
            work.spawn(async move {
                // One tracker's families go in sequence, not at once, while
                // the trackers themselves stay concurrent.
                //
                // A tracker that keys its peer records by peer id alone keeps
                // one address per peer, so a second announce replaces the
                // first. Sent concurrently, which one survives is a race:
                // measured against `loopback-tracker` keyed that way, one
                // peer announcing over both families left a single record and
                // which family it held was whichever announce landed last.
                // In sequence it is the last family in this list, every time,
                // and a reader can say which that is. See `TODO/peers.md`,
                // T-022.
                let mut out = Vec::with_capacity(families.len());
                for family in families {
                    out.push(match scrape {
                        true => client.scrape(&url, tier, &request).await,
                        false => client.announce_on(&url, tier, &request, family).await,
                    });
                }
                (order, out)
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
        let results: Vec<TrackerResult> = results
            .into_iter()
            .flat_map(|(_, batch)| batch)
            .collect::<Vec<_>>();

        // Withdraw the peer record from every tracker that took it. The
        // result is not reported per tracker: a withdrawal that failed leaves
        // a record that expires on its own, which is the state the command
        // was in before it withdrew anything.
        let mut withdrawn = 0usize;
        if withdraw {
            let stop = Announce {
                event: Event::Stopped,
                ..request.clone()
            };
            let mut work = tokio::task::JoinSet::new();
            for result in results.iter().filter(|result| result.ok) {
                let client = client.clone();
                let stop = stop.clone();
                let url = result.url.clone();
                let tier = result.tier;
                // Withdrawn over the family it was registered on. A `stopped`
                // sent over the other family names a different source address
                // and leaves the record it meant to remove.
                let family = result.family;
                work.spawn(async move { client.announce_on(&url, tier, &stop, family).await });
            }
            while let Some(finished) = work.join_next().await {
                if let Ok(result) = finished
                    && result.ok
                {
                    withdrawn += 1;
                }
            }
        }
        Ok::<_, Error>((results, withdrawn))
    })?;

    let mut peers: Vec<String> = results.iter().flat_map(|r| r.peers.clone()).collect();
    peers.sort();
    peers.dedup();

    let report = TrackersReport {
        info_hash: info_hash.hex(),
        name: meta.as_ref().map(|m| m.layout().name),
        action: action(args),
        announced_port: (!args.scrape).then_some(announced_port),
        withdrawn: withdraw.then_some(withdrawn),
        // Counted over distinct URLs, not over results. One tracker announced
        // to over both families is two results and one tracker, and a tracker
        // that answered on IPv4 and not on IPv6 has responded: reporting it as
        // one responded and one failed would say there were two of it.
        tracker_count: distinct_urls(&results).len(),
        responded: distinct_urls(&results)
            .iter()
            .filter(|url| results.iter().any(|r| &&r.url == url && r.ok))
            .count(),
        failed: distinct_urls(&results)
            .iter()
            .filter(|url| !results.iter().any(|r| &&r.url == url && r.ok))
            .count(),
        announces: results.len(),
        families: by_family(&results),
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
    // Held until here, so every announce and every withdrawal named a port
    // something was listening on.
    drop(listeners);

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

/// Hold a port open for as long as the announce lasts.
///
/// Binds every address, not loopback, because the point of announcing a port
/// is that somebody else can reach it. Tries each port in the range in turn,
/// which is what `--port 6881-6889` asks for, and `0` asks the OS.
///
/// **Both families, on the same port.** This command announces once per family
/// now, and an IPv6 announce naming a port only listening on IPv4 registers
/// exactly the black hole T-061 added this listener to prevent. The IPv6 bind
/// is separate rather than dual-stack because `IPV6_V6ONLY` is left on by the
/// standard library on Windows, which is [T-023](../../TODO/peers.md)'s
/// lesson. A host with no IPv6 at all keeps the IPv4 listener and announces
/// over IPv4 only, because `families_of` will not offer a family the tracker
/// does not resolve to either.
///
/// Returned rather than leaked: the caller drops them after the withdrawal, so
/// the listeners outlive every announce that named them.
fn bind_announce_port(values: &[String]) -> Result<Vec<std::net::TcpListener>> {
    let range = swarm::port_range(values)?;
    let mut last = None;
    for port in range.clone() {
        match std::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port)) {
            Ok(listener) => {
                // The port the OS chose, when the caller asked for `0`.
                let chosen = listener.local_addr().map(|a| a.port()).unwrap_or(port);
                let mut held = vec![listener];
                if let Ok(v6) =
                    std::net::TcpListener::bind((std::net::Ipv6Addr::UNSPECIFIED, chosen))
                {
                    held.push(v6);
                }
                return Ok(held);
            }
            Err(e) => last = Some(e),
        }
    }
    // A range that is entirely taken is the caller's problem to hear about,
    // named with the range they asked for.
    Err(bit_cli_core::error::from_io(
        last.unwrap_or_else(|| std::io::Error::other("no port in range")),
        format!(
            "cannot bind a port in {}-{} to announce",
            range.start(),
            range.end()
        ),
    ))
}

/// Which families one tracker gets an announce over.
///
/// `--family v4` and `--family v6` are the caller's choice and are taken as
/// given: a tracker with no address in that family fails the announce and says
/// so, which is the answer somebody asking for one family wants.
///
/// `auto` asks the resolver. Two addresses is two announces, because a tracker
/// records the source address of the connection and one announce registers one
/// of this host's addresses. A resolution that fails falls back to a single
/// announce with no family pinned, so a tracker this command cannot resolve
/// itself still gets tried the way it always was.
fn announce_families(url: &str, wanted: AnnounceFamily) -> Vec<Option<Family>> {
    match wanted {
        AnnounceFamily::V4 => vec![Some(Family::V4)],
        AnnounceFamily::V6 => vec![Some(Family::V6)],
        AnnounceFamily::Auto => match bit_cli_core::tracker::families_of(url) {
            Ok(families) => families.into_iter().map(Some).collect(),
            Err(_) => vec![None],
        },
    }
}

/// What each family's announces returned, folded.
#[derive(Debug, Clone, Serialize)]
pub struct FamilyResult {
    /// `v4` or `v6`.
    pub family: &'static str,
    /// Announces sent over this family.
    pub announces: usize,
    /// How many of them the tracker answered.
    pub responded: usize,
    /// Distinct peer addresses this family's announces returned.
    ///
    /// The two families are reported apart rather than merged because that is
    /// the question: a peer list that only ever comes back on one of them is a
    /// swarm half this host cannot see.
    pub peers: Vec<String>,
}

/// The tracker URLs in the results, once each, in the order they first appear.
fn distinct_urls(results: &[TrackerResult]) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for result in results {
        if !out.contains(&result.url.as_str()) {
            out.push(&result.url);
        }
    }
    out
}

/// Fold the per-announce results by family, for the families that were pinned.
fn by_family(results: &[TrackerResult]) -> Vec<FamilyResult> {
    let mut out = Vec::new();
    for family in [Family::V4, Family::V6] {
        let matching: Vec<&TrackerResult> = results
            .iter()
            .filter(|result| result.family == Some(family))
            .collect();
        if matching.is_empty() {
            continue;
        }
        let mut peers: Vec<String> = matching.iter().flat_map(|r| r.peers.clone()).collect();
        peers.sort();
        peers.dedup();
        out.push(FamilyResult {
            family: family.as_str(),
            announces: matching.len(),
            responded: matching.iter().filter(|r| r.ok).count(),
            peers,
        });
    }
    out
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
    if let Some(port) = report.announced_port {
        out.push(field("announced port", port));
    }
    if let Some(withdrawn) = report.withdrawn {
        out.push(field("withdrawn from", withdrawn));
    }
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
    // Per family, when there was more than one. One family is the same number
    // twice and saying it twice reads as a second measurement.
    if report.families.len() > 1 {
        for family in &report.families {
            out.push(field(
                &format!("peers over ip{}", family.family),
                format!(
                    "{} from {} of {} announces",
                    family.peers.len(),
                    family.responded,
                    family.announces
                ),
            ));
        }
    }

    let rows: Vec<Vec<String>> = report
        .trackers
        .iter()
        .map(|t| {
            vec![
                t.tier.to_string(),
                t.url.clone(),
                t.family
                    .map(|f| f.as_str().to_string())
                    .unwrap_or_else(|| "-".into()),
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
            "TIER", "TRACKER", "FAMILY", "STATUS", "RTT", "SEED", "LEECH", "INTERVAL", "PEERS",
            "REASON",
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
            family: Some(Family::V4),
            endpoint: Some("1.2.3.4:451".into()),
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
            announced_port: Some(6881),
            withdrawn: Some(2),
            tracker_count: 2,
            responded: 2,
            failed: 0,
            announces: 2,
            families: by_family(&[low.clone(), high.clone()]),
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

    /// The announced port is one this command is holding open, and the peer
    /// record it creates does not outlive the command.
    ///
    /// It announced 6881 unconditionally before, whatever it was doing and
    /// whatever was listening there, which registers a peer nobody can dial
    /// and leaves it for the tracker's interval. See `TODO/trackers.md`,
    /// T-061.
    #[test]
    fn the_announced_port_is_bound_and_the_record_is_withdrawn() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let tracker = crate::test_support::Tracker::start(&[]);
        // A port of this run's choosing rather than the default range, so the
        // assertion is "it announced what it bound" and not "6881 happened to
        // be free".
        let wanted = crate::test_support::free_port();
        let report = crate::test_support::run_json(
            &[
                "trackers",
                fixture.path_str(),
                "--replace-trackers",
                "--tracker",
                &tracker.announce,
                "--port",
                &wanted.to_string(),
            ],
            fixture.dir(),
        );

        let port = report["announced_port"].as_u64().expect("a port");
        assert_eq!(port, u64::from(wanted), "{report}");
        assert_eq!(report["withdrawn"], 1, "{report}");

        // Two announces: the question, then the withdrawal, both naming the
        // port the command held open.
        let events = tracker.param("event");
        assert_eq!(events, ["started", "stopped"], "{:?}", tracker.seen());
        let ports = tracker.param("port");
        assert_eq!(
            ports,
            [port.to_string(), port.to_string()],
            "{:?}",
            tracker.seen()
        );

        // And the port really was bound, which is the whole point: binding it
        // again while the command held it would have failed, and now that it
        // has exited it is free.
        std::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port as u16))
            .expect("the announced port is released when the command exits");
    }

    /// `--no-withdraw` leaves the record, and says so.
    #[test]
    fn no_withdraw_sends_one_announce_and_reports_no_withdrawal() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let tracker = crate::test_support::Tracker::start(&[]);
        let report = crate::test_support::run_json(
            &[
                "trackers",
                fixture.path_str(),
                "--replace-trackers",
                "--no-withdraw",
                "--tracker",
                &tracker.announce,
            ],
            fixture.dir(),
        );
        assert!(report["withdrawn"].is_null(), "{report}");
        assert_eq!(tracker.param("event"), ["started"], "{:?}", tracker.seen());
    }

    /// A scrape announces nothing, so it binds nothing and withdraws nothing.
    #[test]
    fn a_scrape_carries_no_port_and_no_withdrawal() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let tracker = crate::test_support::Tracker::start(&[]);
        let (mut env, captured) = Env::test(
            &[
                "--json",
                "trackers",
                fixture.path_str(),
                "--scrape",
                "--replace-trackers",
                "--tracker",
                &tracker.announce,
            ],
            fixture.dir(),
        );
        let _ = crate::run(&mut env);
        let report = captured.json().expect("a report");
        assert!(report["announced_port"].is_null(), "{report}");
        assert!(report["withdrawn"].is_null(), "{report}");
        assert!(tracker.param("event").is_empty(), "{:?}", tracker.seen());
    }
}
