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
    /// What `--listener-check` found. Absent unless it was asked for.
    ///
    /// A seeder whose listener has stopped answering is down, and the rest of
    /// this report cannot say so: the ratio, the uploaded total, and the peer
    /// rows are all history. See `TODO/peers.md`, T-020.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listener: Option<swarm::ListenerReport>,
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
    let listener_check = swarm::optional_duration(&args.listener_check, "listener-check")?;
    let mut stop = StopConditions {
        timeout: swarm::optional_duration(&global.timeout, "timeout")?,
        stop_after: swarm::optional_duration(&global.stop_after, "stop-after")?,
        stall: None,
        lowest_rate: None,
        seed_ratio: args.limits.seed_ratio,
        seed_time: swarm::optional_duration(&args.limits.seed_time, "seed-time")?,
        exit_when_idle: swarm::optional_duration(&args.exit_when_idle, "exit-when-idle")?,
        max_handles: args.limits.max_handles,
        max_rss: swarm::size_flag(&args.limits.max_rss, "max-rss")?,
        // Filled in once the session is live, because the probe needs the
        // port it bound and the info hash it settled on.
        listener: None,
    };
    if args.superseed {
        renderer.warn(
            env,
            "--superseed is accepted but BEP 16 superseeding is not implemented yet; see TODO/create-seed.md",
        );
    }
    // `librqbit` 9.0.0 has no switch for peer exchange: `SessionOptions`
    // carries `dht` and `disable_local_service_discovery` and nothing beside
    // them for PEX. A caller passing this believes their address has stopped
    // being gossiped to the swarm, so silence here is a privacy expectation
    // quietly unmet rather than a performance knob quietly ignored. See
    // `TODO/cli-surface.md`, T-181.
    if args.no_pex {
        renderer.warn(
            env,
            "--no-pex is accepted but peer exchange stays on: librqbit 9.0.0 has no switch for it, so your address is still gossiped to the swarm; see TODO/cli-surface.md T-181",
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
    let base = engine_options.download_directory.clone();

    let kind = Kind::classify(&args.source.source, env)?;
    let meta = match &kind {
        Kind::File(path) => Some(Metainfo::read(path)?),
        _ => None,
    };

    // A multi-file torrent lays its files under a directory named after
    // itself, so `--data` can name the parent or the torrent directory and
    // mean the same payload. `verify` accepted either and this accepted only
    // the parent, which made pointing at the torrent directory a seeder
    // holding nothing and a warning that said "partial seed". Both commands
    // now ask the same function. See `TODO/cli-surface.md`, T-186.
    //
    // A magnet has no layout until its metadata resolves, and by then the
    // session has already decided where to look, so it keeps `--data` as
    // given. Nothing is lost: a magnet has nothing on disk to be pointed at
    // two ways.
    let root = meta
        .as_ref()
        .map(|meta| crate::payload::resolve(&base, &meta.layout()));
    let directory = root
        .as_ref()
        .map_or_else(|| base.clone(), |r| r.path.clone());
    let payload_root = root.as_ref().map(|r| r.path.display().to_string());
    // The resume cache, and where it lives.
    //
    // Beside the payload by default, so moving or deleting the data takes the
    // cache with it and nothing is left behind in a shared directory keyed by
    // a hash nobody can trace back. `--fastresume-dir` overrides it for a
    // caller who wants one place for many torrents. See `TODO/disk-io.md`,
    // T-016.
    if args.fastresume {
        engine_options.resume_cache = Some(match &args.fastresume_dir {
            Some(dir) => env.resolve(dir),
            None => directory.join(bit_cli_core::resume::DEFAULT_DIR_NAME),
        });
    }
    // The place this payload could have been and was not, kept for the warning
    // a seeder holding nothing gets. See `TODO/cli-surface.md`, T-186.
    let other_root = root.as_ref().and_then(|r| r.other.clone());
    if global.dry_run {
        // A dry run reports without doing, so a `--tracker-list-url` is
        // refused rather than fetched. That is the decision
        // `--web-seed-list-url` already takes on `download --dry-run`.
        let trackers = crate::swarm::SessionSetup::tracker_list(
            &setup,
            meta.as_ref(),
            env,
            crate::webseed_args::no_network,
        )?;
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
    let (torrent_download_rate, torrent_upload_rate) = setup.torrent_rates()?;
    let runtime = swarm::runtime()?;
    // `--tracker-list-url` is fetched on the runtime this command already
    // built. See `TODO/cli-surface.md`, T-181.
    let user_agent = bit_cli_core::webseed::fetch::default_user_agent();
    let trackers = setup.tracker_list(
        meta.as_ref(),
        env,
        crate::source::list_fetcher(&runtime, &user_agent),
    )?;

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
            // The resolved payload root, which the files hang directly off.
            // Naming it rather than letting the session append the torrent's
            // own name is what makes `--data <parent>` and
            // `--data <parent>/<name>` the same payload, and it is right even
            // when the directory on disk was renamed. `None` for a magnet,
            // which has no layout to resolve against yet. See
            // `TODO/cli-surface.md`, T-186.
            output_folder: payload_root.clone(),
            trackers: trackers.clone(),
            disable_trackers: trackers.as_ref().is_some_and(Vec::is_empty),
            tracker_interval: swarm::optional_duration(
                &args.trackers.tracker_interval,
                "tracker-interval",
            )?,
            download_rate: torrent_download_rate,
            upload_rate: torrent_upload_rate,
            ..Default::default()
        };
        // What the payload should look like, recorded before the add,
        // because the session loads the cached bitfield during the add. A
        // torrent with no metadata yet, which is a magnet, has nothing to
        // describe and is never served from the cache.
        if let (Some(meta), Some(dir)) = (meta.as_ref(), root.as_ref()) {
            let layout = meta.layout();
            let files: Vec<(String, u64)> = layout
                .files
                .iter()
                .map(|f| (f.path.join("/"), f.length))
                .collect();
            let pieces = layout.total_length.div_ceil(u64::from(layout.piece_length));
            engine.expect_resume(
                &meta.info_hash().hex(),
                bit_cli_core::resume::Fingerprint::of(
                    &dir.path,
                    &files,
                    layout.total_length,
                    pieces.try_into().unwrap_or(u32::MAX),
                ),
            );
        }
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
        // Holding **none** of it is the case a partial seed's warning cannot
        // describe, because a partial seed is legitimate and a wrong `--data`
        // is not. Saying which directory was searched, and which other one a
        // multi-file torrent's files also sit under, is the difference.
        //
        // Keyed on bytes rather than on whether the files exist, and that is
        // deliberate: a seeder creates the tree it was looking for, so by the
        // second run the directory holds full-length files with nothing in
        // them and "the payload is not here" would be false. See
        // `TODO/cli-surface.md`, T-186.
        if snapshot.progress_bytes == 0 {
            let elsewhere = match &other_root {
                Some(other) => format!(
                    "; a multi-file torrent's files also sit under {}",
                    other.display()
                ),
                None => String::new(),
            };
            renderer.warn(
                env,
                format!(
                    "none of {} is in {}, which is where --data resolved to{elsewhere}",
                    layout.name,
                    directory.display()
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
                // Announce-only never serves, so there is no listener to
                // have watched.
                None,
            ));
        }

        // The probe needs the port the session actually bound and an info
        // hash it is actually serving, so this is the first point where both
        // are known. Announce-only returns above it, because a run that never
        // serves has no listener worth watching.
        let listener = match (listener_check, engine.bridge_target()) {
            (Some(interval), Some(target)) => {
                let state =
                    swarm::spawn_listener_probe(target, handle.info_hash().0, interval);
                stop.listener = Some(swarm::ListenerCheck {
                    state: std::sync::Arc::clone(&state),
                    allowed: swarm::LISTENER_FAILURES_ALLOWED,
                });
                Some(state)
            }
            (Some(_), None) => {
                renderer.warn(
                    env,
                    "--listener-check does nothing here: this run bound no listen port, so there is no listener to probe",
                );
                None
            }
            (None, _) => None,
        };

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

            let mut snapshot = engine.snapshot(&handle);
            let probe_ports = listener.as_ref().map(|s| s.ports()).unwrap_or_default();
            let view =
                swarm::without_probe_rows(engine.peers(&handle, &HashSet::new()), &probe_ports);
            // Before `observe` and before the stop conditions, both of which
            // read the peer counts. See `swarm::discount_probe_peers`.
            swarm::discount_probe_peers(&mut snapshot, &view);
            let peers = view.rows;
            progress.observe(&snapshot, None, &handle.stats().file_progress);

            let mut event = json!({
                    "info_hash": snapshot.info_hash,
                    "uploaded_bytes": snapshot.uploaded_bytes,
                    "upload_rate": snapshot.upload_rate,
                    "download_rate": snapshot.download_rate,
                    "ratio": format!("{:.3}", snapshot.ratio()),
                    "peers": snapshot.peers,
                    "peer_detail": peers,
                    // What the process costs right now, so a soak reads a slope out of
                    // the event stream rather than sampling the process from outside.
                    // See `TODO/memory.md`, T-040.
                    "process": bit_cli_core::sysinfo::Process::sample(),
            });
            // The key is absent unless the check was asked for, so a consumer
            // tells "watched and fine" from "not watched" without a flag of
            // its own. Inserted rather than written into the literal above,
            // because `json!` has no way to leave a key out.
            if let Some(state) = &listener
                && let Some(fields) = event.as_object_mut()
            {
                fields.insert(
                    "listener".into(),
                    serde_json::to_value(state.report()).unwrap_or_default(),
                );
            }
            renderer.event(env, "progress", &event)?;
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

        let mut snapshot = engine.snapshot(&handle);
        let probe_ports = listener.as_ref().map(|s| s.ports()).unwrap_or_default();
        let view = swarm::without_probe_rows(engine.peers(&handle, &HashSet::new()), &probe_ports);
        swarm::discount_probe_peers(&mut snapshot, &view);
        let peers = view.rows;
        let report = build(
            &snapshot,
            stopped,
            elapsed,
            &directory,
            engine.listen_addr().map(|a| a.to_string()),
            tracker_list,
            peers,
            renames(&engine, &handle),
            listener.as_ref().map(|s| s.report()),
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
    listener: Option<swarm::ListenerReport>,
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
        listener,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
        );
        let text = lines(&report).join("\n");
        assert!(text.contains("2.000"), "{text}");
        assert!(text.contains("udp://t.example:451"), "{text}");
        assert!(text.contains("203.0.113.5:6881"), "{text}");
        assert!(text.contains("0.0.0.0:6881"), "{text}");
        assert!(text.contains("peak RSS"), "the cost is not display-only");
    }

    /// `TODO/cli-surface.md` T-181. `--no-pex` cannot be built against
    /// `librqbit` 9.0.0, so it says so instead of pretending.
    ///
    /// A caller passing this believes peer exchange is off. It is not, and
    /// their address keeps being gossiped to the swarm, which is a privacy
    /// expectation quietly unmet rather than a knob quietly ignored. The
    /// warning names the entry so a reader can find out when it will change.
    #[test]
    fn no_pex_warns_that_peer_exchange_stays_on() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let (mut env, captured) = crate::env::Env::test(
            &[
                "seed",
                "--dry-run",
                "--no-pex",
                "--data",
                fixture.dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        assert_eq!(crate::run(&mut env), ExitCode::Success);
        let err = captured.err();
        assert!(err.contains("--no-pex"), "{err}");
        assert!(
            err.contains("peer exchange stays on"),
            "the warning has to say what is still happening: {err}"
        );
        assert!(
            err.contains("T-181"),
            "the warning has to name the entry that owns it: {err}"
        );
        assert!(!captured.out().is_empty(), "the report still prints");
    }

    /// Without the flag there is no warning, so the message is about the flag
    /// rather than something every seed prints.
    #[test]
    fn a_seed_without_no_pex_says_nothing_about_peer_exchange() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let (mut env, captured) = crate::env::Env::test(
            &[
                "seed",
                "--dry-run",
                "--data",
                fixture.dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        assert_eq!(crate::run(&mut env), ExitCode::Success);
        assert!(
            !captured.err().contains("peer exchange"),
            "{}",
            captured.err()
        );
    }

    /// A seeder is the long-lived process, so its own high-water marks are
    /// what a soak test reads. See `TODO/memory.md`, T-040.
    /// Seeding what `download --select-file` left behind.
    ///
    /// `TODO/disk-io.md` T-184 expected a seeder to announce pieces it could
    /// not prove. It does not, and the reason is that there are none: the
    /// unselected half of a boundary piece is written into the unselected file
    /// for the piece's sake, so the piece verifies and the hash check finds
    /// it. What the seeder holds is exactly pieces 1 and 2 of four, 2048 bytes
    /// of 3700, and it says so.
    #[test]
    fn a_seeder_of_a_selection_holds_the_boundary_pieces_and_says_so() {
        let fixture = crate::test_support::TorrentFixture::straddling();
        let server = crate::test_support::FileServer::start(fixture.dir());
        let source = format!("{}payload/", server.base);
        let out = fixture.dir().join("out");
        crate::test_support::run_json(
            &[
                "download",
                fixture.path_str(),
                "--dir",
                out.to_str().unwrap(),
                "--web-seed-only",
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
                "--no-torrent-web-seed",
                "--no-tracker",
                "--port",
                "0",
                "--select-file",
                "1",
                "--stop-after",
                "30s",
            ],
            fixture.dir(),
        );

        let report = crate::test_support::run_json_code(
            &[
                "seed",
                fixture.path_str(),
                // The parent. Since `TODO/cli-surface.md` T-186 the torrent
                // directory works too, and
                // `either_spelling_of_data_seeds_the_same_payload` is what
                // pins that.
                "--data",
                out.to_str().unwrap(),
                "--port",
                "0",
                "--no-dht",
                "--no-lsd",
                "--no-tracker",
                "--stop-after",
                "3s",
            ],
            fixture.dir(),
            // Nothing connects, so the run stops on its deadline. What it
            // holds is decided by the hash check and reported either way.
            ExitCode::Timeout,
        );
        assert_eq!(
            report["have"]["bytes"], 2048,
            "a seeder holds both boundary pieces, which is 2048 of 3700 bytes: {report}"
        );
        assert_eq!(report["total"]["bytes"], 3700);
        assert_eq!(
            report["complete"], false,
            "and it does not claim to hold the rest"
        );
    }

    /// `TODO/cli-surface.md` T-186's acceptance.
    ///
    /// A multi-file torrent lays its files under a directory named after
    /// itself, so `--data` can name the parent or the torrent directory. Both
    /// spellings are the same payload, and before this only one of them was:
    /// the other reported `have: 0` with "this is a partial seed", which is
    /// the right observation with the wrong reason, and created an empty tree
    /// one level deeper on its way to saying it.
    #[test]
    fn either_spelling_of_data_seeds_the_same_payload() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let data = fixture.dir().join("data");
        fixture.place(&data, &[]);

        let seed = |dir: &std::path::Path| {
            crate::test_support::run_json_code(
                &[
                    "seed",
                    fixture.path_str(),
                    "--data",
                    dir.to_str().unwrap(),
                    "--port",
                    "0",
                    "--no-dht",
                    "--no-lsd",
                    "--no-tracker",
                    "--stop-after",
                    "1s",
                ],
                fixture.dir(),
                // Nothing connects, so the run stops on its deadline.
                ExitCode::Timeout,
            )
        };

        let parent = seed(&data);
        let torrent_dir = seed(&data.join("album"));
        assert_eq!(parent["have"]["bytes"], 2000, "{parent}");
        assert_eq!(
            torrent_dir["have"]["bytes"], parent["have"]["bytes"],
            "the torrent directory is the same payload as its parent: {torrent_dir}"
        );
        assert_eq!(torrent_dir["complete"], true, "{torrent_dir}");
        // Both resolve to the directory the files hang off, so the report says
        // where the payload is rather than what was typed.
        assert_eq!(
            torrent_dir["data_directory"], parent["data_directory"],
            "both spellings name the same directory: {torrent_dir}"
        );
        // And nothing was created a level deeper on the way.
        assert!(
            !data.join("album").join("album").exists(),
            "a seeder pointed at the torrent directory built one inside it"
        );
    }

    /// A seeder holding nothing says which directory it searched and which
    /// other one a multi-file torrent's files sit under. A partial-seed
    /// warning on its own cannot: a partial seed is legitimate and a `--data`
    /// naming the wrong place is not, and "0 B of 1.95 KiB" is what both look
    /// like. See `TODO/cli-surface.md`, T-186.
    ///
    /// Run twice on purpose. The first run creates the tree it was looking
    /// for, at full length and empty, so a message keyed on whether the files
    /// exist would be true once and false afterwards. This one is keyed on
    /// bytes and says the same thing both times.
    #[test]
    fn a_seed_that_holds_nothing_names_the_directories_it_searched() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let empty = fixture.dir().join("empty");
        std::fs::create_dir_all(&empty).unwrap();

        for run in 1..=2 {
            let (mut env, captured) = crate::env::Env::test(
                &[
                    "seed",
                    fixture.path_str(),
                    "--data",
                    empty.to_str().unwrap(),
                    "--port",
                    "0",
                    "--no-dht",
                    "--no-lsd",
                    "--no-tracker",
                    "--stop-after",
                    "1s",
                ],
                fixture.dir(),
            );
            assert_eq!(crate::run(&mut env), ExitCode::Timeout);
            let err = captured.err();
            assert!(err.contains("none of album is in"), "run {run}: {err}");
            assert!(err.contains(empty.to_str().unwrap()), "run {run}: {err}");
            assert!(
                err.contains(empty.join("album").to_str().unwrap()),
                "run {run}: the other candidate is named too: {err}"
            );
        }
    }

    /// And a seeder that holds the payload says none of that.
    #[test]
    fn a_complete_seed_says_nothing_about_where_it_looked() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let data = fixture.dir().join("data");
        fixture.place(&data, &[]);

        let (mut env, captured) = crate::env::Env::test(
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
                "1s",
            ],
            fixture.dir(),
        );
        assert_eq!(crate::run(&mut env), ExitCode::Timeout);
        let err = captured.err();
        assert!(!err.contains("none of album"), "{err}");
        assert!(!err.contains("partial seed"), "{err}");
    }

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
            None,
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

    /// A soak reads its series out of the event stream.
    ///
    /// `bit-cli seed` is the long-lived process, and the question T-040 asks
    /// is whether its memory and handles grow over hours. Sampling it from
    /// outside needs a second tool per platform, so every `progress` event
    /// carries what the process costs at that moment. See `TODO/memory.md`,
    /// T-040.
    #[test]
    fn every_seed_progress_event_carries_what_the_process_costs() {
        let fixture = crate::test_support::TorrentFixture::hostile();
        let data = fixture.dir().join("data");
        std::fs::create_dir_all(&data).unwrap();

        let (mut env, captured) = crate::env::Env::test(
            &[
                "--jsonl",
                "seed",
                fixture.path_str(),
                "--data",
                data.to_str().unwrap(),
                "--port",
                "0",
                "--no-dht",
                "--no-lsd",
                "--no-tracker",
                "--report-interval",
                "200ms",
                "--stop-after",
                "2s",
            ],
            fixture.dir(),
        );
        let _ = crate::run(&mut env);
        let events = captured.jsonl().expect("stdout was not ndjson");
        let progress: Vec<_> = events
            .iter()
            .filter(|event| event["type"] == "progress")
            .collect();
        assert!(
            progress.len() >= 2,
            "a 2s run at a 200ms interval should tick more than once, got {}",
            progress.len()
        );
        for event in &progress {
            let process = &event["process"];
            assert!(
                process["open_handles"].as_u64().unwrap_or(0) > 0,
                "no handle count in {event}"
            );
            assert!(
                process["rss_bytes"].as_u64().unwrap_or(0) > 1024 * 1024,
                "no resident memory in {event}"
            );
            assert!(
                process["peak_rss_bytes"].as_u64().unwrap_or(0)
                    >= process["rss_bytes"].as_u64().unwrap_or(0),
                "peak below current in {event}"
            );
        }
    }

    /// The session announces the port it is listening on, and that port is
    /// dialable while the run lasts.
    ///
    /// The upstream report this comes from is a session announcing 0 on one
    /// version and a fixed 4240 on another, either of which registers a peer
    /// nobody can reach while the download itself looks fine. `bit-cli` leaves
    /// `ListenerOptions::announce_port` unset so the session announces what it
    /// bound, and this is what says so rather than assuming it. See
    /// `TODO/trackers.md`, T-060.
    #[test]
    fn the_session_announces_the_port_it_listens_on() {
        let fixture = crate::test_support::TorrentFixture::multi_file();
        let tracker = crate::test_support::Tracker::start(&[]);
        let port = crate::test_support::free_port();
        let data = fixture.dir().join("served");
        fixture.place(&data, &[]);

        let seeder = {
            let torrent = fixture.path_str().to_string();
            let data = data.to_str().expect("utf-8 path").to_string();
            let announce = tracker.announce.clone();
            let cwd = fixture.dir();
            std::thread::spawn(move || {
                let (mut env, _) = crate::env::Env::test(
                    &[
                        "seed",
                        &torrent,
                        "--data",
                        &data,
                        "--port",
                        &port.to_string(),
                        "--replace-trackers",
                        "--tracker",
                        &announce,
                        "--no-dht",
                        "--no-lsd",
                        "--stop-after",
                        "4s",
                    ],
                    cwd,
                );
                crate::run(&mut env)
            })
        };

        // Wait for the first announce rather than sleeping a fixed time: the
        // session announces as soon as the torrent is live, which is after a
        // hash check whose length is the machine's business.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let mut announced = Vec::new();
        while announced.is_empty() && std::time::Instant::now() < deadline {
            announced = tracker.param("port");
            if announced.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        assert_eq!(
            announced.first().map(String::as_str),
            Some(port.to_string().as_str()),
            "announced {announced:?}, listening on {port}: {:?}",
            tracker.seen()
        );

        // The announced address is one a peer could dial, which is the half of
        // this that a recorded port number does not prove.
        std::net::TcpStream::connect(("127.0.0.1", port))
            .expect("the announced port is not accepting connections");

        let _ = seeder.join();
    }
}
