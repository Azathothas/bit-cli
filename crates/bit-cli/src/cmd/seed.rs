//! `bit-cli seed`: serve existing data in the foreground.
//!
//! Seeding is a peer of downloading, not a mode of it. The question this
//! command exists to answer is "is my server actually serving", so the report
//! is per-peer: who connected, how they connected, what they took, and how
//! fast. Aggregate totals alone cannot answer it.
//!
//! It runs until a stop condition is met and then exits with a code naming
//! which one. There is no daemon.

use std::collections::HashSet;
use std::time::Duration;

use bit_cli_core::ExitCode;
use bit_cli_core::engine::{AddOptions, Engine, PeerSnapshot, TorrentSnapshot};
use bit_cli_core::error::{Error, Result};
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{Size, format_rate, format_size};
use serde::Serialize;
use serde_json::json;

use crate::cli::{Global, SeedArgs, SeedVerify};
use crate::env::Env;
use crate::output::{Renderer, field};
use crate::source::Kind;
use crate::swarm::{self, Progress, SessionSetup, StopConditions, Stopped};

/// What `bit-cli seed` reports.
#[derive(Debug, Clone, Serialize)]
pub struct SeedReport {
    pub info_hash: String,
    pub name: String,
    pub stopped: Stopped,
    pub complete: bool,
    pub total: Size,
    pub have: Size,
    pub uploaded: Size,
    pub uploaded_human: String,
    pub ratio: String,
    pub elapsed_ms: u64,
    pub elapsed_human: String,
    pub mean_upload_rate: Size,
    pub mean_upload_rate_human: String,
    pub peers_seen: u32,
    pub peers_served: usize,
    pub data_directory: String,
    pub listen_addr: Option<String>,
    pub trackers: Vec<String>,
    pub peers: Vec<PeerSnapshot>,
    /// Files whose on-disk path is not the path in the torrent, and why.
    ///
    /// The same array `download --json` reports, because a seeder serves the
    /// files that command wrote. A caller seeding a payload whose paths were
    /// rewritten cannot otherwise tell which file on disk is which file in the
    /// torrent. See `bit_cli_core::paths` and `TODO/windows.md`, T-076.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub renamed: Vec<bit_cli_core::paths::Rename>,
    /// What this run cost: peak RSS, CPU time, and open handles.
    ///
    /// A seeder is the long-lived process, so its own high-water marks are
    /// what a soak test reads. Sampling from outside means sampling a process
    /// that has already exited, which reports zero.
    pub process: bit_cli_core::sysinfo::Process,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Where this torrent's files actually live, when that is not where the
/// torrent said.
///
/// A seeder serves the files a download wrote, and a download rewrites a path
/// the filesystem refuses. Without this the report names files the caller
/// cannot find. See `TODO/windows.md`, T-076.
fn renames(
    engine: &Engine,
    handle: &bit_cli_core::engine::Handle,
) -> Vec<bit_cli_core::paths::Rename> {
    engine
        .path_plan(handle)
        .map(|plan| plan.renames)
        .unwrap_or_default()
}

/// Run the command.
pub fn run(
    args: &SeedArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let report_interval = swarm::duration_flag(&args.report_interval, "report-interval")?;
    let stop = StopConditions {
        timeout: swarm::optional_duration(&global.timeout, "timeout")?,
        stop_after: swarm::optional_duration(&global.stop_after, "stop-after")?,
        stall: None,
        lowest_rate: None,
        seed_ratio: args.limits.seed_ratio,
        seed_time: swarm::optional_duration(&args.limits.seed_time, "seed-time")?,
        exit_when_idle: swarm::optional_duration(&args.exit_when_idle, "exit-when-idle")?,
        max_handles: args.limits.max_handles,
    };
    if args.superseed {
        renderer.warn(
            env,
            "--superseed is accepted but BEP 16 superseeding is not implemented yet; see TODO/create-seed.md",
        );
    }

    let web_seeds = crate::cli::WebSeedArgs::default();
    let setup = SessionSetup {
        global,
        trackers: &args.trackers,
        limits: &args.limits,
        web_seeds: &web_seeds,
        listen_ports: swarm::port_range(&args.port)?,
        no_dht: args.no_dht,
        no_lsd: args.no_lsd,
        // Seeding reads what is already on disk and creates nothing.
        allocation: bit_cli_core::alloc::Allocation::default(),
    };
    let mut engine_options = setup.engine_options(env)?;
    // Seeding reads the payload from where it already lives, which is not
    // necessarily where a download would have written it.
    if let Some(data) = &args.data {
        engine_options.download_directory = env.resolve(data);
    }
    let directory = engine_options.download_directory.clone();

    let kind = Kind::classify(&args.source.source, env)?;
    let meta = match &kind {
        Kind::File(path) => Some(Metainfo::read(path)?),
        _ => None,
    };
    let trackers = setup.tracker_list(meta.as_ref(), env)?;

    if global.dry_run {
        let report = json!({
            "dry_run": true,
            "source": args.source.source,
            "data_directory": directory.display().to_string(),
            "trackers": trackers.clone().unwrap_or_default(),
            "verify": format!("{:?}", args.verify).to_lowercase(),
            "info_hash": meta.as_ref().map(|m| m.info_hash().hex()),
        });
        renderer.emit(env, "seed", &report, || {
            vec![
                field("dry run", "nothing will be served"),
                field("data", directory.display()),
                field("trackers", trackers.clone().unwrap_or_default().len()),
            ]
        })?;
        return Ok(ExitCode::Success);
    }

    // All three values behave the same today, and saying so is better than
    // letting a caller believe `--verify none` skipped anything. `librqbit`
    // 9.0.0 hash-checks on add and `AddTorrentOptions` carries no way to ask
    // it not to, so there is nothing for the other two values to reach.
    // Measured on a 512 MiB payload: 6087 ms, 6372 ms, and 6398 ms. See
    // `TODO/disk-io.md`, T-016.
    if args.verify != SeedVerify::Full {
        renderer.warn(
            env,
            format!(
                "--verify {} still hash-checks the whole payload on start: the session cannot serve unverified data and has no way to skip the check",
                match args.verify {
                    SeedVerify::Quick => "quick",
                    _ => "none",
                }
            ),
        );
    }

    let init_timeout = swarm::duration_flag(&args.limits.init_timeout, "init-timeout")?;
    let source = args.source.source.clone();
    let announce_only = args.announce_only;
    let runtime = swarm::runtime()?;

    let report = runtime.block_on(async {
        let engine = Engine::start(&engine_options).await?;
        for warning in engine.warnings() {
            renderer.warn(env, warning);
        }

        let add = AddOptions {
            // Seeding needs the existing payload read and hash-checked, which
            // is what `overwrite` allows. Without it the add fails on the
            // files that are the whole point of the command.
            overwrite: true,
            trackers: trackers.clone(),
            disable_trackers: trackers.as_ref().is_some_and(Vec::is_empty),
            tracker_interval: swarm::optional_duration(
                &args.trackers.tracker_interval,
                "tracker-interval",
            )?,
            ..Default::default()
        };
        let handle = engine.add(&source, &add).await?;
        renderer.event(
            env,
            "session_start",
            &json!({
                "source": source,
                "data_directory": directory.display().to_string(),
                "listen_addr": engine.listen_addr().map(|a| a.to_string()),
            }),
        )?;

        engine
            .wait_until_initialized_within(&handle, init_timeout)
            .await?;
        let layout = engine.layout(&handle).ok_or_else(|| {
            Error::source_resolution(format!("{source}: the torrent has no metadata"))
        })?;
        let snapshot = engine.snapshot(&handle);

        // Seeding data that is not all there is a partial seed, which is
        // legitimate, but the caller should be told rather than discover it
        // from a ratio that never moves.
        if !snapshot.finished {
            renderer.warn(
                env,
                format!(
                    "only {} of {} is present, so this is a partial seed",
                    format_size(snapshot.progress_bytes),
                    format_size(snapshot.total_bytes)
                ),
            );
        }

        let tracker_list = engine.trackers(&handle);
        if announce_only {
            // The announce already happened when the torrent went live, so
            // this reports it and stops rather than serving.
            tokio::time::sleep(Duration::from_secs(2)).await;
            let snapshot = engine.snapshot(&handle);
            let peers = engine.peers(&handle, &HashSet::new());
            return Ok(build(
                &snapshot,
                Stopped::Completed,
                Duration::from_secs(2),
                &directory,
                engine.listen_addr().map(|a| a.to_string()),
                tracker_list,
                peers,
                renames(&engine, &handle),
            ));
        }

        let lengths: Vec<u64> = layout.files.iter().map(|f| f.length).collect();
        let mut progress = Progress::new(layout.piece_count(), lengths);
        let mut ticker = tokio::time::interval(report_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let interrupt = tokio::signal::ctrl_c();
        tokio::pin!(interrupt);

        let (stopped, elapsed) = loop {
            tokio::select! {
                _ = &mut interrupt => break (Stopped::Interrupted, progress.elapsed()),
                _ = ticker.tick() => {}
            }

            let snapshot = engine.snapshot(&handle);
            progress.observe(&snapshot, None, &handle.stats().file_progress);
            let peers = engine.peers(&handle, &HashSet::new());

            renderer.event(
                env,
                "progress",
                &json!({
                    "info_hash": snapshot.info_hash,
                    "uploaded_bytes": snapshot.uploaded_bytes,
                    "upload_rate": snapshot.upload_rate,
                    "download_rate": snapshot.download_rate,
                    "ratio": format!("{:.3}", snapshot.ratio()),
                    "peers": snapshot.peers,
                    "peer_detail": peers,
                }),
            )?;
            if renderer.progress == crate::cli::ProgressMode::Plain {
                let _ = env.note(format!(
                    "up {}  uploaded {}  ratio {:.3}  peers {}",
                    format_rate(snapshot.upload_rate),
                    format_size(snapshot.uploaded_bytes),
                    snapshot.ratio(),
                    snapshot.peers.live,
                ));
            }

            if let Some(reason) = progress.should_stop(&snapshot, &stop, true) {
                break (reason, progress.elapsed());
            }
        };

        let snapshot = engine.snapshot(&handle);
        let peers = engine.peers(&handle, &HashSet::new());
        let report = build(
            &snapshot,
            stopped,
            elapsed,
            &directory,
            engine.listen_addr().map(|a| a.to_string()),
            tracker_list,
            peers,
            renames(&engine, &handle),
        );
        engine.stop().await;
        Ok::<_, Error>(report)
    })?;

    // Seeding to nobody is the failure a seeding operator most needs to catch,
    // and it is indistinguishable from success in the totals alone.
    let code = match (report.stopped, report.peers_seen) {
        (Stopped::Idle, 0) => ExitCode::ThresholdNotMet,
        (reason, _) => reason.code(),
    };
    renderer.emit(env, "seed", &report, || lines(&report))?;
    Ok(code)
}

#[allow(clippy::too_many_arguments)]
fn build(
    snapshot: &TorrentSnapshot,
    stopped: Stopped,
    elapsed: Duration,
    directory: &std::path::Path,
    listen_addr: Option<String>,
    trackers: Vec<String>,
    peers: Vec<PeerSnapshot>,
    renamed: Vec<bit_cli_core::paths::Rename>,
) -> SeedReport {
    let elapsed_ms = elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
    let mean = match elapsed_ms {
        0 => 0,
        ms => snapshot.uploaded_bytes.saturating_mul(1000) / ms,
    };
    SeedReport {
        info_hash: snapshot.info_hash.clone(),
        name: snapshot.name.clone(),
        stopped,
        complete: snapshot.finished,
        total: Size(snapshot.total_bytes),
        have: Size(snapshot.progress_bytes),
        uploaded: Size(snapshot.uploaded_bytes),
        uploaded_human: format_size(snapshot.uploaded_bytes),
        ratio: bit_cli_core::units::format_ratio(snapshot.ratio()),
        elapsed_ms,
        elapsed_human: bit_cli_core::units::format_duration(elapsed),
        mean_upload_rate: Size(mean),
        mean_upload_rate_human: format_rate(mean),
        peers_seen: snapshot.peers.seen,
        peers_served: peers.iter().filter(|p| p.uploaded_bytes > 0).count(),
        data_directory: directory.display().to_string(),
        listen_addr,
        trackers,
        peers,
        renamed,
        process: bit_cli_core::sysinfo::Process::sample(),
        error: snapshot.error.clone(),
    }
}

fn lines(report: &SeedReport) -> Vec<String> {
    let mut out = vec![
        field("name", &report.name),
        field("info hash", &report.info_hash),
        field("stopped", report.stopped.as_str()),
        field("complete", report.complete),
        field(
            "have",
            format!(
                "{} of {}",
                format_size(report.have.0),
                format_size(report.total.0)
            ),
        ),
        field("uploaded", &report.uploaded_human),
        field("ratio", &report.ratio),
        field("mean up", &report.mean_upload_rate_human),
        field("elapsed", &report.elapsed_human),
        field("peers seen", report.peers_seen),
        field("peers served", report.peers_served),
        field("data", &report.data_directory),
        field("cost", report.process.summary()),
    ];
    if let Some(addr) = &report.listen_addr {
        out.push(field("listening on", addr));
    }
    for tracker in &report.trackers {
        out.push(field("tracker", tracker));
    }
    if let Some(error) = &report.error {
        out.push(field("error", error));
    }
    if !report.peers.is_empty() {
        out.push(String::new());
        out.extend(swarm::peer_table(&report.peers));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_cli_core::engine::{PeerCounts, State};

    fn snapshot(uploaded: u64) -> TorrentSnapshot {
        TorrentSnapshot {
            id: 0,
            info_hash: "a".repeat(40),
            name: "payload".into(),
            state: State::Live,
            total_bytes: 1000,
            progress_bytes: 1000,
            uploaded_bytes: uploaded,
            finished: true,
            download_rate: 0,
            upload_rate: 0,
            eta_ms: None,
            eta_confidence: "none",
            peers: PeerCounts {
                seen: 3,
                ..Default::default()
            },
            error: None,
        }
    }

    fn peer(uploaded: u64) -> PeerSnapshot {
        PeerSnapshot {
            addr: "203.0.113.5:6881".into(),
            state: "live".into(),
            client: Some("rqbit".into()),
            connection: Some("tcp".into()),
            direction: "incoming",
            downloaded_bytes: 0,
            uploaded_bytes: uploaded,
            verified_pieces: 0,
            chunks: 0,
            errors: 0,
            connect_ms: 12,
            mean_piece_ms: None,
            web_seed: false,
        }
    }

    #[test]
    fn the_report_counts_only_peers_that_actually_took_bytes() {
        let report = build(
            &snapshot(2000),
            Stopped::SeedRatio,
            Duration::from_secs(4),
            std::path::Path::new("/data"),
            Some("0.0.0.0:6881".into()),
            vec!["udp://t.example:451".into()],
            vec![peer(2000), peer(0)],
            Vec::new(),
        );
        assert_eq!(
            report.peers_served, 1,
            "a connected peer that took nothing was not served"
        );
        assert_eq!(report.peers.len(), 2, "but both are still reported");
        assert_eq!(report.peers_seen, 3);
    }

    #[test]
    fn the_ratio_is_rendered_to_three_places() {
        let report = build(
            &snapshot(2500),
            Stopped::SeedTime,
            Duration::from_secs(1),
            std::path::Path::new("/data"),
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(report.ratio, "2.500");
    }

    #[test]
    fn the_mean_upload_rate_is_bytes_over_elapsed_seconds() {
        let report = build(
            &snapshot(4000),
            Stopped::SeedTime,
            Duration::from_secs(4),
            std::path::Path::new("/data"),
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(report.mean_upload_rate.0, 1000);
        assert_eq!(report.elapsed_ms, 4000);
    }

    #[test]
    fn a_zero_length_run_does_not_divide_by_zero() {
        let report = build(
            &snapshot(4000),
            Stopped::Interrupted,
            Duration::ZERO,
            std::path::Path::new("/data"),
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(report.mean_upload_rate.0, 0);
    }

    #[test]
    fn the_text_rendering_carries_every_number_the_json_does() {
        let report = build(
            &snapshot(2000),
            Stopped::SeedRatio,
            Duration::from_secs(4),
            std::path::Path::new("/data"),
            Some("0.0.0.0:6881".into()),
            vec!["udp://t.example:451".into()],
            vec![peer(2000)],
            Vec::new(),
        );
        let text = lines(&report).join("\n");
        assert!(text.contains("2.000"), "{text}");
        assert!(text.contains("udp://t.example:451"), "{text}");
        assert!(text.contains("203.0.113.5:6881"), "{text}");
        assert!(text.contains("0.0.0.0:6881"), "{text}");
        assert!(text.contains("peak RSS"), "the cost is not display-only");
    }

    /// A seeder is the long-lived process, so its own high-water marks are
    /// what a soak test reads. See `TODO/memory.md`, T-040.
    #[test]
    fn a_seed_report_carries_what_the_process_cost() {
        let report = build(
            &snapshot(0),
            Stopped::Deadline,
            Duration::from_secs(1),
            std::path::Path::new("/data"),
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert!(report.process.peak_rss_bytes > 1024 * 1024);
        assert!(report.process.open_handles > 0);
        assert!(report.process.unavailable.is_empty());
    }
    /// A seeder reports where the payload actually lives.
    ///
    /// It serves the files a download wrote, and a download rewrites a path
    /// the filesystem refuses. Without the mapping a caller cannot tell which
    /// file on disk is which file in the torrent. The same array
    /// `download --json` and `verify --json` carry. See `TODO/windows.md`,
    /// T-076.
    #[test]
    fn a_seed_of_a_hostile_torrent_reports_every_renamed_path() {
        let fixture = crate::test_support::TorrentFixture::hostile();
        let data = fixture.dir().join("data");
        std::fs::create_dir_all(&data).unwrap();

        let report = crate::test_support::run_json_code(
            &[
                "seed",
                fixture.path_str(),
                "--data",
                data.to_str().unwrap(),
                "--port",
                "0",
                "--no-dht",
                "--no-lsd",
                "--no-tracker",
                "--stop-after",
                "3s",
            ],
            fixture.dir(),
            // Nothing is on disk and nothing connects, so the run stops on its
            // deadline. The mapping is reported either way, which is the point.
            ExitCode::Timeout,
        );
        let renamed = report["renamed"].as_array().expect("a renamed array");
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
    }
}
