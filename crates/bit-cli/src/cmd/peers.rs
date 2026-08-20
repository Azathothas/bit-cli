//! `bit-cli peers`: connect, sample the swarm, report, exit.
//!
//! This takes a duration or a peer count, not a session id, because there is
//! no session to hold one. It joins the swarm, watches for as long as it was
//! told to, and reports every peer it saw with what came from each.

use std::collections::HashSet;
use std::time::Duration;

use bit_cli_core::ExitCode;
use bit_cli_core::engine::{AddOptions, Engine, PeerSnapshot};
use bit_cli_core::error::{Error, Result};
use bit_cli_core::units::Size;
use serde::Serialize;
use serde_json::json;

use crate::cli::{Global, PeersArgs};
use crate::env::Env;
use crate::output::{Renderer, field};
use crate::source::Kind;
use crate::swarm::{self, SessionSetup};

/// What `bit-cli peers` reports.
#[derive(Debug, Clone, Serialize)]
pub struct PeersReport {
    pub info_hash: String,
    pub name: String,
    pub sampled_ms: u64,
    pub sampled_human: String,
    pub live: u32,
    pub connecting: u32,
    pub queued: u32,
    pub seen: u32,
    pub dead: u32,
    pub downloaded: Size,
    pub peers: Vec<PeerSnapshot>,
}

/// How the peer list is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Addr,
    Client,
    Speed,
    Pieces,
}

impl SortKey {
    fn parse(text: &str) -> Result<(Self, bool)> {
        let (key, order) = match text.split_once(':') {
            Some((key, order)) => (key, order),
            None => (text, "asc"),
        };
        let key = match key.trim() {
            "addr" | "address" => Self::Addr,
            "client" => Self::Client,
            "speed" | "down" => Self::Speed,
            "pieces" => Self::Pieces,
            other => {
                return Err(Error::usage(format!(
                    "--sort `{other}` is not a peer key; use addr, client, speed, or pieces"
                ))
                .with("value", other.to_string()));
            }
        };
        let descending = match order.trim() {
            "asc" | "ascending" => false,
            "desc" | "descending" => true,
            other => {
                return Err(
                    Error::usage(format!("--sort order `{other}` is not asc or desc"))
                        .with("value", other.to_string()),
                );
            }
        };
        Ok((key, descending))
    }
}

/// Sort peers in place.
fn sort_peers(peers: &mut [PeerSnapshot], key: SortKey, descending: bool) {
    match key {
        SortKey::Addr => peers.sort_by(|a, b| a.addr.cmp(&b.addr)),
        SortKey::Client => peers.sort_by(|a, b| {
            a.client
                .as_deref()
                .unwrap_or("")
                .cmp(b.client.as_deref().unwrap_or(""))
                .then_with(|| a.addr.cmp(&b.addr))
        }),
        SortKey::Speed => peers.sort_by(|a, b| {
            a.downloaded_bytes
                .cmp(&b.downloaded_bytes)
                .then_with(|| a.addr.cmp(&b.addr))
        }),
        SortKey::Pieces => peers.sort_by(|a, b| {
            a.verified_pieces
                .cmp(&b.verified_pieces)
                .then_with(|| a.addr.cmp(&b.addr))
        }),
    }
    if descending {
        peers.reverse();
    }
}

/// Run the command.
pub fn run(
    args: &PeersArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let duration = swarm::duration_flag(&args.duration, "duration")?;
    let (key, descending) = SortKey::parse(&args.sort)?;
    let kind = Kind::classify(&args.source.source, env)?;

    let trackers = crate::cli::TrackerArgs::default();
    let limits = crate::cli::LimitArgs::default();
    let web_seeds = crate::cli::WebSeedArgs::default();
    let setup = SessionSetup {
        global,
        trackers: &trackers,
        limits: &limits,
        web_seeds: &web_seeds,
        listen_ports: swarm::port_range(&args.port)?,
        no_dht: false,
        no_lsd: false,
    };
    let mut engine_options = setup.engine_options(env)?;
    // Sampling a swarm must never write a payload. The session still needs a
    // directory for its own bookkeeping, so it gets a temporary one that goes
    // away with the process.
    let scratch = tempfile::tempdir()
        .map_err(|e| bit_cli_core::error::from_io(e, "cannot create a scratch directory"))?;
    engine_options.download_directory = scratch.path().to_path_buf();

    let source = args.source.source.clone();
    let count = args.count;
    let _ = kind;
    let runtime = swarm::runtime()?;

    let report = runtime.block_on(async {
        let engine = Engine::start(&engine_options).await?;
        for warning in engine.warnings() {
            renderer.warn(env, warning);
        }
        // Paused keeps the torrent connected to the swarm for peer discovery
        // without pulling any payload, which is the whole point of sampling.
        let add = AddOptions {
            paused: true,
            list_only: false,
            ..Default::default()
        };
        let handle = engine.add(&source, &add).await?;

        renderer.event(
            env,
            "session_start",
            &json!({
                "source": source,
                "duration_ms": duration.as_millis().min(u128::from(u64::MAX)) as u64,
                "count": count,
            }),
        )?;

        let started = std::time::Instant::now();
        let mut ticker = tokio::time::interval(Duration::from_millis(500));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let interrupt = tokio::signal::ctrl_c();
        tokio::pin!(interrupt);

        loop {
            tokio::select! {
                _ = &mut interrupt => break,
                _ = ticker.tick() => {}
            }
            let snapshot = engine.snapshot(&handle);
            if started.elapsed() >= duration {
                break;
            }
            if let Some(count) = count
                && snapshot.peers.seen as usize >= count
            {
                break;
            }
        }

        let elapsed = started.elapsed();
        let snapshot = engine.snapshot(&handle);
        let peers = engine.peers(&handle, &HashSet::new());
        let report = PeersReport {
            info_hash: snapshot.info_hash.clone(),
            name: snapshot.name.clone(),
            sampled_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
            sampled_human: bit_cli_core::units::format_duration(elapsed),
            live: snapshot.peers.live,
            connecting: snapshot.peers.connecting,
            queued: snapshot.peers.queued,
            seen: snapshot.peers.seen,
            dead: snapshot.peers.dead,
            downloaded: Size(snapshot.progress_bytes),
            peers,
        };
        engine.stop().await;
        Ok::<_, Error>(report)
    })?;

    let mut report = report;
    sort_peers(&mut report.peers, key, descending);

    // A swarm with nobody in it is a real answer, not a failure to produce
    // one, but a script needs to tell the two apart from the exit code.
    let code = match report.seen {
        0 => ExitCode::NoUsableSources,
        _ => ExitCode::Success,
    };
    renderer.emit(env, "peers", &report, || lines(&report))?;
    Ok(code)
}

fn lines(report: &PeersReport) -> Vec<String> {
    let mut out = vec![
        field("name", &report.name),
        field("info hash", &report.info_hash),
        field("sampled for", &report.sampled_human),
        field("live", report.live),
        field("connecting", report.connecting),
        field("queued", report.queued),
        field("seen", report.seen),
        field("dead", report.dead),
    ];
    if !report.peers.is_empty() {
        out.push(String::new());
        out.extend(swarm::peer_table(&report.peers));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(addr: &str, client: Option<&str>, down: u64, pieces: u32) -> PeerSnapshot {
        PeerSnapshot {
            addr: addr.into(),
            state: "live".into(),
            client: client.map(ToString::to_string),
            connection: Some("tcp".into()),
            direction: "outgoing",
            downloaded_bytes: down,
            uploaded_bytes: 0,
            verified_pieces: pieces,
            chunks: 0,
            errors: 0,
            connect_ms: 0,
            mean_piece_ms: None,
            web_seed: false,
        }
    }

    fn sample() -> Vec<PeerSnapshot> {
        vec![
            peer("203.0.113.9:6881", Some("rqbit"), 100, 2),
            peer("203.0.113.1:6881", Some("aria2"), 900, 1),
            peer("203.0.113.5:6881", None, 500, 9),
        ]
    }

    #[test]
    fn the_default_sort_is_by_address_ascending() {
        let (key, descending) = SortKey::parse("addr").unwrap();
        assert_eq!(key, SortKey::Addr);
        assert!(!descending);

        let mut peers = sample();
        sort_peers(&mut peers, key, descending);
        assert_eq!(peers[0].addr, "203.0.113.1:6881");
        assert_eq!(peers[2].addr, "203.0.113.9:6881");
    }

    #[test]
    fn every_documented_sort_key_parses() {
        for text in ["addr", "address", "client", "speed", "down", "pieces"] {
            assert!(SortKey::parse(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn a_descending_order_reverses_the_result() {
        let (key, descending) = SortKey::parse("speed:desc").unwrap();
        assert!(descending);
        let mut peers = sample();
        sort_peers(&mut peers, key, descending);
        assert_eq!(peers[0].downloaded_bytes, 900);
        assert_eq!(peers[2].downloaded_bytes, 100);
    }

    #[test]
    fn sorting_by_pieces_orders_by_what_each_peer_actually_gave() {
        let (key, descending) = SortKey::parse("pieces:desc").unwrap();
        let mut peers = sample();
        sort_peers(&mut peers, key, descending);
        assert_eq!(peers[0].verified_pieces, 9);
    }

    #[test]
    fn a_peer_with_no_client_string_still_sorts() {
        let (key, descending) = SortKey::parse("client").unwrap();
        let mut peers = sample();
        sort_peers(&mut peers, key, descending);
        assert_eq!(
            peers[0].client, None,
            "an unknown client sorts before a named one"
        );
    }

    #[test]
    fn a_bad_sort_key_names_the_valid_ones() {
        let err = SortKey::parse("bandwidth").unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
        assert!(err.message().contains("addr"), "{}", err.message());
        assert!(err.message().contains("pieces"), "{}", err.message());
    }

    #[test]
    fn a_bad_sort_order_is_refused_rather_than_ignored() {
        let err = SortKey::parse("addr:sideways").unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
        assert_eq!(err.context()["value"], "sideways");
    }

    #[test]
    fn the_table_labels_a_bridge_as_a_web_seed() {
        let mut peers = sample();
        peers[0].web_seed = true;
        let text = swarm::peer_table(&peers).join("\n");
        assert!(text.contains("web seed"), "{text}");
    }
}
