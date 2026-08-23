//! Generating `docs/schema.md` by running every command and reading what it
//! wrote.
//!
//! This is a test module rather than a binary. Making it a subcommand would
//! put a documentation generator on the shipped surface, and making it a
//! separate tool would let the tool and the program drift. A test can drive
//! `run` in process, against fixtures, with no network and no ports, which is
//! the same thing every other test here does.
//!
//! The one test in it renders the whole file and compares it to what is
//! committed. `BIT_CLI_UPDATE_SCHEMA=1` writes it instead, which is the only
//! way the file is ever edited.
//!
//! See `TODO/cli-surface.md`, T-117.

use std::collections::BTreeMap;

use bit_cli_core::ExitCode;

use crate::env::Env;
use crate::schema::{DOCUMENT_KINDS, EVENT_TYPES, Sample, fields, render};
use crate::test_support::{FileServer, TorrentFixture};

/// Run one command in process and return what it wrote to stdout.
///
/// The exit code is not asserted: several of these commands are driven into a
/// failure on purpose, because the document a failure writes is part of the
/// contract too.
fn capture(args: &[&str], cwd: impl Into<std::path::PathBuf>) -> (ExitCode, String) {
    let (mut env, captured) = Env::test(args, cwd);
    let code = crate::run(&mut env);
    (code, captured.out())
}

/// Fold one run's document into the sample for its `kind`.
fn observe_document(into: &mut BTreeMap<String, Sample>, command: &str, out: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(out) else {
        return;
    };
    fold_document(into, command, &value);
}

/// The half of [`observe_document`] that works on a value already parsed.
fn fold_document(into: &mut BTreeMap<String, Sample>, command: &str, value: &serde_json::Value) {
    let Some(kind) = value.get("kind").and_then(|k| k.as_str()) else {
        return;
    };
    let flattened = fields(value);
    into.entry(kind.to_string())
        .and_modify(|sample| sample.merge(flattened.clone()))
        .or_insert_with(|| Sample {
            name: kind.to_string(),
            command: command.to_string(),
            fields: flattened,
        });
}

/// Fold one `bench` report in, without the machine it ran on.
///
/// `environment` is the only thing dropped, and it is dropped because it
/// describes the machine rather than the measurement. `host.os.distribution`
/// is read from `/etc/os-release` and so exists on Linux and nowhere else; the
/// macOS reader has no interface table at all, so `host.network` is empty
/// there and that one row is `array` where Windows and Linux produce the
/// object rows under it; and both `unavailable` lists appear only when a read
/// failed. Folding it in would make the contract a record of whichever machine
/// last regenerated it, and turn the next platform's run red for saying so.
///
/// Nothing a consumer selects is under it. `scripts/check-swarm.ps1` reads
/// `swarm.serving.pieces_announced`, `scripts/bench-leech.ps1` reads
/// `summary.disk.write_time.ms`, and `--baseline` compares `summary`. See
/// `TODO/bench.md`, T-189.
fn observe_report(into: &mut BTreeMap<String, Sample>, command: &str, out: &str) {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(out) else {
        return;
    };
    if let Some(fields) = value.as_object_mut() {
        fields.remove("environment");
    }
    fold_document(into, command, &value);
}

/// Fold one run's events into the sample for each `type`.
fn observe_events(into: &mut BTreeMap<String, Sample>, command: &str, out: &str) {
    for line in out.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(event) = value.get("type").and_then(|t| t.as_str()) else {
            continue;
        };
        let flattened = fields(&value);
        into.entry(event.to_string())
            .and_modify(|sample| sample.merge(flattened.clone()))
            .or_insert_with(|| Sample {
                name: event.to_string(),
                command: command.to_string(),
                fields: flattened,
            });
    }
}

/// Every run the generator drives, and what each one is for.
///
/// A run that fails is as useful as one that succeeds: the `error` event and a
/// `download` document that stopped on its deadline are both part of the
/// contract. So nothing here asserts an exit code.
fn collect() -> (Vec<Sample>, Vec<Sample>) {
    let mut documents: BTreeMap<String, Sample> = BTreeMap::new();
    let mut events: BTreeMap<String, Sample> = BTreeMap::new();

    let fixture = TorrentFixture::multi_file();
    let torrent = fixture.path_str().to_string();
    let dir = fixture.dir();
    let payload = fixture.payload_dir();

    // The commands that touch nothing.
    for (label, args) in [
        (
            "bit-cli info <TORRENT> --json",
            vec!["--json", "info", &torrent],
        ),
        (
            "bit-cli files <TORRENT> --json",
            vec!["--json", "files", &torrent],
        ),
        (
            "bit-cli files <TORRENT> --against <OTHER> --json",
            vec!["--json", "files", &torrent, "--against", &torrent],
        ),
        (
            "bit-cli magnet <TORRENT> --json",
            vec!["--json", "magnet", &torrent],
        ),
        ("bit-cli version --json", vec!["--json", "version"]),
        (
            "bit-cli config show --json",
            vec!["--json", "config", "show"],
        ),
        (
            "bit-cli verify <TORRENT> --dir <DIR> --json",
            vec!["--json", "verify", &torrent, "--dir", dir.to_str().unwrap()],
        ),
        (
            "bit-cli webseed list <TORRENT> --web-seed <URL> --json",
            vec![
                "--json",
                "webseed",
                "list",
                &torrent,
                "--web-seed",
                "https://mirror.example.com/pub/",
            ],
        ),
    ] {
        let (_, out) = capture(&args, dir.clone());
        observe_document(&mut documents, label, &out);
    }

    // `create` and `edit` write files, so they go into the fixture's own
    // directory and nowhere else.
    let created = dir.join("made.torrent");
    let (_, out) = capture(
        &[
            "--json",
            "create",
            payload.to_str().unwrap(),
            "--name",
            "album",
            "--piece-length",
            "1KiB",
            "--no-creation-date",
            "--output",
            created.to_str().unwrap(),
            "--force",
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli create <DIR> --output <TORRENT> --json",
        &out,
    );
    let (_, out) = capture(
        &[
            "--json",
            "edit",
            created.to_str().unwrap(),
            "--announce",
            "udp://tracker.example:451",
            "--force",
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli edit <TORRENT> --announce <URL> --force --json",
        &out,
    );

    // A real transfer, so `piece_verified`, `file_completed`, and a completed
    // download are in the stream rather than only the shapes a failure makes.
    let server = FileServer::start(dir.clone());
    let out_dir = dir.join("out");
    let source = format!("{}payload/", server.base);
    for format in ["--json", "--jsonl"] {
        let (_, out) = capture(
            &[
                format,
                "download",
                &torrent,
                "--dir",
                out_dir.to_str().unwrap(),
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
                "--web-seed-only",
                "--allow-overwrite",
                "--port",
                "0",
                "--report-interval",
                "50ms",
                "--stop-after",
                "20s",
            ],
            dir.clone(),
        );
        match format {
            "--json" => observe_document(
                &mut documents,
                "bit-cli download <TORRENT> --web-seed <URL> --json",
                &out,
            ),
            _ => observe_events(
                &mut events,
                "bit-cli download <TORRENT> --web-seed <URL> --jsonl",
                &out,
            ),
        }
    }

    // One more run of the same command, for the `hooks` block alone. It is
    // omitted from a run that used no hook, so without this the contract would
    // not describe a field the program emits. Folded into the same `kind`, and
    // `Sample::merge` keeps the first command, so the `From` line above stays
    // the one a reader should copy. `--on-complete` fires once per torrent, so
    // this is one process. See `docs/hooks.md` and `TODO/cli-surface.md`,
    // T-115.
    let (_, out) = capture(
        &[
            "--json",
            "download",
            &torrent,
            "--dir",
            out_dir.to_str().unwrap(),
            "--web-seed",
            &source,
            "--web-seed-mode",
            "prefix",
            "--web-seed-only",
            "--allow-overwrite",
            "--port",
            "0",
            "--stop-after",
            "20s",
            "--on-complete",
            match cfg!(windows) {
                true => "rem",
                false => "true",
            },
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli download <TORRENT> --web-seed <URL> --json",
        &out,
    );

    // A selection whose boundary pieces straddle into files it did not choose,
    // so `torrents[].partial` and `verify`'s `not_selected` are documented
    // rather than only implemented. Its own fixture, because the one above has
    // no piece outside any selection: every file of a two-file torrent at this
    // piece length touches both pieces. See `TODO/disk-io.md`, T-184.
    let selection_fixture = TorrentFixture::straddling();
    let selection_dir = selection_fixture.dir();
    let selection_server = FileServer::start(selection_dir.clone());
    let selection_source = format!("{}payload/", selection_server.base);
    let selection_out = selection_dir.join("out");
    let (_, out) = capture(
        &[
            "--json",
            "download",
            selection_fixture.path_str(),
            "--dir",
            selection_out.to_str().unwrap(),
            "--web-seed",
            &selection_source,
            "--web-seed-mode",
            "prefix",
            "--web-seed-only",
            "--no-torrent-web-seed",
            "--no-tracker",
            "--allow-overwrite",
            "--port",
            "0",
            "--select-file",
            "1",
            "--stop-after",
            "20s",
        ],
        selection_dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli download <TORRENT> --select-file <INDEX> --json",
        &out,
    );
    let (_, out) = capture(
        &[
            "--json",
            "verify",
            selection_fixture.path_str(),
            "--data",
            selection_out.join("album").to_str().unwrap(),
            "--select-file",
            "1",
            "--per-piece",
        ],
        selection_dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli verify <TORRENT> --select-file <INDEX> --per-piece --json",
        &out,
    );

    // A Metalink, which is the only source kind that resolves its own torrent
    // and then checks the payload against a second description of it. Its own
    // fixture and its own server, because the document has to name the
    // `.torrent` by URL and that means the server has to be serving the
    // torrent as well as the payload. See `TODO/cli-surface.md`, T-113.
    //
    // The checksum is `sha-1` rather than `sha-256` only because this crate
    // already depends on `sha1` and the schema records field names, not
    // values. Both go through the same code.
    let metalink_fixture = TorrentFixture::single_file();
    let metalink_dir = metalink_fixture.dir();
    let metalink_server = FileServer::start(metalink_dir.clone());
    let payload_bytes = &metalink_fixture.files[0].1;
    let payload_sha1: String = <sha1::Sha1 as sha1::Digest>::digest(payload_bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let meta4 = metalink_dir.join("release.meta4");
    std::fs::write(
        &meta4,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<metalink xmlns="urn:ietf:params:xml:ns:metalink">
  <file name="payload.bin">
    <size>{size}</size>
    <hash type="sha-1">{payload_sha1}</hash>
    <url priority="1">{base}payload/payload.bin</url>
    <metaurl mediatype="torrent">{base}payload.bin.torrent</metaurl>
  </file>
</metalink>
"#,
            size = payload_bytes.len(),
            base = metalink_server.base,
        ),
    )
    .expect("write the metalink");
    let meta4_arg = meta4.to_str().expect("utf-8 path").to_string();
    let metalink_out = metalink_dir.join("out");
    for format in ["--json", "--jsonl"] {
        let (_, out) = capture(
            &[
                format,
                "download",
                &meta4_arg,
                "--dir",
                metalink_out.to_str().unwrap(),
                "--web-seed-only",
                "--allow-overwrite",
                "--port",
                "0",
                "--report-interval",
                "50ms",
                "--stop-after",
                "20s",
            ],
            metalink_dir.clone(),
        );
        match format {
            "--json" => {
                observe_document(&mut documents, "bit-cli download <METALINK> --json", &out)
            }
            _ => observe_events(&mut events, "bit-cli download <METALINK> --jsonl", &out),
        }
    }
    // `--dry-run` is sampled now, under its own `kind`. It carried
    // `kind: "download"` and a shape sharing almost nothing with a real run
    // until T-156, so folding it in would have made the `download` table a
    // union of two documents without saying which fields belong to which.
    //
    // Two runs, merged, because neither reaches every field on its own: the
    // torrent one is the only one that resolves a file layout, so it is the
    // only one with `torrents[].coverage` and a real `info_hash`, and the
    // Metalink one is the only source kind that fills `torrents[].metalink`.
    //
    // The torrent goes first, and the order is load-bearing. `Sample::merge`
    // is `or_insert`, so the first observation of a path names its type and
    // later ones only add paths. Taking the Metalink first documented
    // `info_hash`, `name` and `total_bytes` as `null`, which is what a
    // Metalink dry run leaves them as and is not what the field is.
    // See `TODO/cli-surface.md`, T-156.
    let (_, out) = capture(
        &[
            "--json",
            "download",
            &torrent,
            "--dir",
            out_dir.to_str().unwrap(),
            "--web-seed",
            &source,
            "--web-seed-mode",
            "prefix",
            "--dry-run",
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli download <TORRENT> --web-seed <URL> --dry-run --json",
        &out,
    );
    let (_, out) = capture(
        &[
            "--json",
            "download",
            &meta4_arg,
            "--dir",
            metalink_out.to_str().unwrap(),
            "--dry-run",
        ],
        metalink_dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli download <METALINK> --dry-run --json",
        &out,
    );

    // The three `webseed` verbs that need a server: one request per source,
    // a concurrency sweep, and one piece pulled and checked.
    //
    // `--no-torrent-web-seed` because the fixture torrent carries
    // `https://mirror.example.com/pub/` in its url-list, and without it that
    // is source zero: `fetch --piece 0` reached for the network and failed,
    // and `test` and `probe` waited out a connect timeout against a name this
    // machine should never resolve during a test run.
    for (label, args) in [
        (
            "bit-cli webseed test <TORRENT> --web-seed <URL> --json",
            vec![
                "--json",
                "webseed",
                "test",
                &torrent,
                "--no-torrent-web-seed",
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
            ],
        ),
        (
            "bit-cli webseed probe <TORRENT> --web-seed <URL> --json",
            vec![
                "--json",
                "webseed",
                "probe",
                &torrent,
                "--no-torrent-web-seed",
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
                "--duration",
                "1s",
                "--concurrency-sweep",
                "1,2",
            ],
        ),
        (
            "bit-cli webseed fetch <TORRENT> --piece 0 --web-seed <URL> --json",
            vec![
                "--json",
                "webseed",
                "fetch",
                &torrent,
                "--no-torrent-web-seed",
                "--piece",
                "0",
                "--web-seed",
                &source,
                "--web-seed-mode",
                "prefix",
            ],
        ),
    ] {
        let (_, out) = capture(&args, dir.clone());
        observe_document(&mut documents, label, &out);
    }

    // A download that cannot finish, for `source_failed` and `error`, a
    // cooling one for `source_cooling`, and a stalled one for `peer_redial`.
    //
    // Both failing runs point at a path the server does not have rather than
    // at an address nothing listens on. A source has to answer to fail: the
    // bridge only makes a request when the session asks it for a block, so an
    // address that neither answers nor refuses produces no request, no error,
    // and no event until the request timeout 30 seconds later. That is
    // [T-141](../../TODO/webseed.md), found here. A 404 from a live server
    // fails in the first second.
    //
    // The two runs differ only in `--web-seed-cooldown`, which is what decides
    // whether a spent budget leaves the source `Failed` or `Cooling`. The
    // cooling one also needs `--web-seed-retry-status 404`, because a 404 is
    // fatal by default and a fatal status retires a source without spending
    // the budget that a cooldown waits out.
    let absent = format!("{}absent/", server.base);
    for (label, args) in [
        (
            "bit-cli download <TORRENT> --web-seed <404 URL> --jsonl",
            vec![
                "--jsonl",
                "download",
                &torrent,
                "--dir",
                dir.join("dead").to_str().unwrap(),
                "--web-seed-only",
                "--no-torrent-web-seed",
                "--web-seed",
                &absent,
                "--web-seed-mode",
                "prefix",
                "--web-seed-max-errors",
                "1",
                "--web-seed-retries",
                "0",
                "--no-tracker",
                "--port",
                "0",
                "--report-interval",
                "100ms",
                "--stop-after",
                "10s",
            ],
        ),
        (
            "bit-cli download <TORRENT> --web-seed <404 URL> --web-seed-cooldown <DUR> --jsonl",
            vec![
                "--jsonl",
                "download",
                &torrent,
                "--dir",
                dir.join("cooling").to_str().unwrap(),
                "--web-seed-only",
                "--no-torrent-web-seed",
                "--web-seed",
                &absent,
                "--web-seed-mode",
                "prefix",
                "--web-seed-retry-status",
                "404",
                "--web-seed-max-errors",
                "1",
                "--web-seed-retries",
                "0",
                "--web-seed-cooldown",
                "60s",
                "--no-tracker",
                "--port",
                "0",
                "--report-interval",
                "100ms",
                "--stop-after",
                "5s",
            ],
        ),
        (
            "bit-cli download <TORRENT> --redial-after <DUR> --jsonl",
            vec![
                "--jsonl",
                "download",
                &torrent,
                "--dir",
                dir.join("stalled").to_str().unwrap(),
                "--no-tracker",
                "--no-dht",
                "--no-lsd",
                "--port",
                "0",
                "--report-interval",
                "100ms",
                "--redial-after",
                "300ms",
                "--stop-after",
                "2s",
            ],
        ),
    ] {
        let owned: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
        let (_, out) = capture(&borrowed, dir.clone());
        observe_events(&mut events, label, &out);
    }

    // Two torrents in one run that hold the same file, which is the only way
    // a `download` document carries `shared`. The donor is complete on disk
    // and the receiver has everything except the shared file, so the run needs
    // no source and no network. See `TODO/multi-source.md`, T-140.
    //
    // It is also the run that announces, because a `download` document
    // carries `announced` only when the run has a tracker to tell. The
    // loopback one answers and records nothing.
    let (donor, receiver) = TorrentFixture::sharing_pair();
    let share_dir = donor.dir().join("out");
    donor.place(&share_dir, &[]);
    receiver.place(&share_dir, &["extra-b.txt"]);
    let share_tracker = crate::test_support::Tracker::start(&[]);
    let (_, out) = capture(
        &[
            "--json",
            "download",
            donor.path_str(),
            receiver.path_str(),
            "--dir",
            share_dir.to_str().unwrap(),
            "--no-torrent-web-seed",
            "--replace-trackers",
            "--tracker",
            &share_tracker.announce,
            "--no-dht",
            "--no-lsd",
            "--port",
            "0",
            "-j",
            "1",
            "--report-interval",
            "100ms",
            "--stop-after",
            "20s",
        ],
        donor.dir(),
    );
    observe_document(
        &mut documents,
        "bit-cli download <TORRENT> <OTHER> --json, where both hold one file",
        &out,
    );

    // The download landed the payload, so verifying it is the `verify`
    // document rather than the `hash_mismatch` one the fixture directory
    // produces.
    let (_, out) = capture(
        &[
            "--json",
            "verify",
            &torrent,
            "--dir",
            out_dir.to_str().unwrap(),
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli verify <TORRENT> --dir <DIR> --json",
        &out,
    );

    // A download onto files that are already there without --allow-overwrite,
    // for the `error` event: the worker returns before the session is live, so
    // this is the shape a run that could not start reports.
    let (_, out) = capture(
        &[
            "--jsonl",
            "download",
            &torrent,
            "--dir",
            out_dir.to_str().unwrap(),
            "--no-continue",
            "--no-tracker",
            "--no-dht",
            "--no-lsd",
            "--port",
            "0",
            "--stop-after",
            "5s",
        ],
        dir.clone(),
    );
    observe_events(
        &mut events,
        "bit-cli download <TORRENT> --no-continue --jsonl",
        &out,
    );

    // A failure before any session, for the shape a failed run ends with.
    let (_, out) = capture(&["--jsonl", "info", "nope.torrent"], dir.clone());
    observe_events(&mut events, "bit-cli info <MISSING> --jsonl", &out);

    // Seeding: the `seed` document and a `progress` event with the peer
    // detail a download's progress does not carry.
    for format in ["--json", "--jsonl"] {
        let (_, out) = capture(
            &[
                format,
                "seed",
                &torrent,
                "--data",
                dir.to_str().unwrap(),
                "--port",
                "0",
                "--no-dht",
                "--no-lsd",
                "--no-tracker",
                "--report-interval",
                "100ms",
                "--stop-after",
                "1s",
                // The `listener` block, which is absent unless the check was
                // asked for. Five seconds against a one second run so no
                // probe can complete inside it: what is documented is the
                // block's shape, and a probe that raced the deadline would
                // make `last_rtt_ms` an integer on one machine and null on
                // the next. Types here are what a run produced, and this run
                // has to produce the same one every time.
                "--listener-check",
                "5s",
            ],
            dir.clone(),
        );
        match format {
            "--json" => observe_document(&mut documents, "bit-cli seed <TORRENT> --json", &out),
            _ => observe_events(&mut events, "bit-cli seed <TORRENT> --jsonl", &out),
        }
    }

    // `peers` needs a swarm and `trackers` needs a tracker, and neither may
    // reach the network to get one.
    //
    // The seeder is a real one: it holds the payload under the torrent's own
    // name, which the fixture directory does not, because it keeps its files
    // under `payload/`. `peers` is pointed straight at it with `--peer`, so
    // the sampled peer is a live connection with a client string and a
    // direction rather than an address a tracker named.
    let seeder_port = crate::test_support::free_port();
    let seed_root = dir.join("seedroot");
    for (path, bytes) in &fixture.files {
        let target = seed_root.join("album").join(path);
        std::fs::create_dir_all(target.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&target, bytes).expect("write the seeded payload");
    }

    let seeder = {
        let torrent = torrent.clone();
        let data = seed_root.to_str().unwrap().to_string();
        let cwd = dir.clone();
        std::thread::spawn(move || {
            capture(
                &[
                    "seed",
                    &torrent,
                    "--data",
                    &data,
                    "--port",
                    &seeder_port.to_string(),
                    "--no-dht",
                    "--no-lsd",
                    "--no-tracker",
                    "--stop-after",
                    "10s",
                ],
                cwd,
            )
        })
    };

    // Same race as the `peers` test, and here it is quieter and worse: a dial
    // that beats the listener samples a `peers` document with a dead peer and
    // no bytes, and the schema this generator writes is then missing whatever
    // a live peer carries. See `TODO/cli-surface.md`, T-160 and T-158.
    assert!(
        crate::test_support::wait_for_listener(seeder_port, std::time::Duration::from_secs(10)),
        "the seeder never listened on {seeder_port}"
    );

    let peer = format!("127.0.0.1:{seeder_port}");
    let (_, out) = capture(
        &[
            "--json",
            "peers",
            &torrent,
            "--peer",
            &peer,
            "--no-tracker",
            "--no-dht",
            "--no-lsd",
            "--duration",
            "5s",
            "--port",
            "0",
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli peers <TORRENT> --peer <ADDR> --json",
        &out,
    );

    // Again with the peer blocked, because `blocked` is absent from a sample
    // that refused nothing and a field no run produces is a field the schema
    // does not describe. This one needs no live seeder: the address is refused
    // before it is dialled, so whether anything is listening does not matter.
    // See `TODO/peers.md`, T-164.
    let (_, out) = capture(
        &[
            "--json",
            "peers",
            &torrent,
            "--peer",
            &peer,
            "--block-peer",
            "127.0.0.1",
            "--no-tracker",
            "--no-dht",
            "--no-lsd",
            "--duration",
            "1s",
            "--port",
            "0",
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli peers <TORRENT> --peer <ADDR> --block-peer <ADDR> --json",
        &out,
    );

    // Two trackers, and the second one is dead on purpose: `failure` is only
    // set on a tracker that did not answer, and a document that never carries
    // it does not describe the field. `--replace-trackers` keeps the fixture's
    // own `udp://tracker.example.com:80` out of the run.
    let tracker = crate::test_support::Tracker::start(&[std::net::SocketAddrV4::new(
        std::net::Ipv4Addr::LOCALHOST,
        seeder_port,
    )]);
    let (_, out) = capture(
        &[
            "--json",
            "trackers",
            &torrent,
            "--replace-trackers",
            "--tracker",
            &tracker.announce,
            "--tracker",
            "http://127.0.0.1:1/announce",
            "--tracker-timeout",
            "5s",
        ],
        dir.clone(),
    );
    observe_document(
        &mut documents,
        "bit-cli trackers <TORRENT> --tracker <URL> --json",
        &out,
    );
    let _ = seeder.join();

    // `bench_sample` is one point of a time series, and every `bench` target
    // emits it. `disk` is the one that needs no source, no port, and no
    // network.
    //
    // The payload has to be big enough to outlast a sample interval. 4 MiB
    // finished in 5 ms on this machine and produced no sample at all, which is
    // the same reason a two minute soak says nothing about a six hour one.
    //
    // This run gives the events only. Under `--jsonl` the report renders as
    // NDJSON records carrying `record` rather than `type`, so `observe_events`
    // does not pick it up, and the head of that stream is not the report
    // anyway: `render::ndjson` empties `series`, `sources`,
    // `concurrency_curve` and `disk_steps` out of it and splits them into
    // records of their own. The report is taken from its own run below, in the
    // form `--baseline` and the acceptance scripts actually read.
    let (_, out) = capture(
        &[
            "--jsonl",
            "bench",
            "disk",
            "--dir",
            dir.join("bench").to_str().unwrap(),
            "--payload-size",
            "64MiB",
            "--block-size",
            "64KiB",
            "--concurrency",
            "2",
            "--metrics-interval",
            "10ms",
            "--duration",
            "10s",
            "--no-verify",
        ],
        dir.clone(),
    );
    observe_events(&mut events, "bit-cli bench disk --jsonl", &out);

    // The report itself, from a second run, because `--jsonl` pins the format
    // to NDJSON whatever `--format` says. `bench disk` is the target that
    // needs no source, no port and no network, and every other target writes
    // the same document with a different `kind`. Shorter than the run above:
    // this one is measuring nothing, it only has to produce every field once,
    // and a 10 ms sample interval fills `series` long before the payload is
    // written. See `TODO/bench.md`, T-189.
    let (_, out) = capture(
        &[
            "--json",
            "bench",
            "disk",
            "--dir",
            dir.join("bench-report").to_str().unwrap(),
            "--payload-size",
            "16MiB",
            "--block-size",
            "64KiB",
            "--concurrency",
            "2",
            "--metrics-interval",
            "10ms",
            "--duration",
            "10s",
            "--no-verify",
        ],
        dir.clone(),
    );
    observe_report(&mut documents, "bit-cli bench disk --json", &out);

    let mut documents: Vec<Sample> = documents.into_values().collect();
    let mut events: Vec<Sample> = events.into_values().collect();
    documents.sort_by(|a, b| a.name.cmp(&b.name));
    events.sort_by(|a, b| a.name.cmp(&b.name));
    (documents, events)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_path() -> std::path::PathBuf {
        // The crate directory is the working directory for its tests, and the
        // document lives at the repository root.
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(crate::schema::SCHEMA_PATH)
    }

    /// The field path a schema row names, or `None` if the line is not a row.
    ///
    /// Rows are `| `path` | type |`, and the path is what identifies a field:
    /// the type is this run's measurement of it and may legitimately change.
    fn row_path(line: &str) -> Option<String> {
        let rest = line.strip_prefix("| `")?;
        let (path, _) = rest.split_once('`')?;
        Some(path.to_string())
    }

    /// Every field row in `text`, keyed by the section it appears under.
    ///
    /// The key is the `##` heading and the `###` heading together, because a
    /// document kind and an event type may share a name and their field lists
    /// are different things.
    fn rows_by_section(text: &str) -> BTreeMap<(String, String), BTreeMap<String, String>> {
        let mut out: BTreeMap<(String, String), BTreeMap<String, String>> = BTreeMap::new();
        let mut part = String::new();
        let mut section = String::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                part = rest.to_string();
                section.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("### ") {
                section = rest.to_string();
                continue;
            }
            if let Some(path) = row_path(line) {
                out.entry((part.clone(), section.clone()))
                    .or_default()
                    .insert(path, line.to_string());
            }
        }
        out
    }

    /// Take the rows pending for one section and write them merged.
    fn flush_section(
        out: &mut String,
        previous: &BTreeMap<(String, String), BTreeMap<String, String>>,
        part: &str,
        section: &str,
        pending: &mut Vec<String>,
    ) {
        if pending.is_empty() {
            return;
        }
        let mut merged: BTreeMap<String, String> = previous
            .get(&(part.to_string(), section.to_string()))
            .cloned()
            .unwrap_or_default();
        for row in pending.drain(..) {
            if let Some(path) = row_path(&row) {
                merged.insert(path, row);
            }
        }
        for row in merged.values() {
            out.push_str(row);
            out.push('\n');
        }
    }

    /// Union the committed field rows into the rendered ones, section by
    /// section.
    ///
    /// The read side of this check is a **containment** check on purpose: a
    /// row the committed file has and this run did not produce is not a
    /// failure, because these runs are timed and a download that beats its own
    /// report tick emits no `progress`, and a run with no failing source emits
    /// no `error`. The writer used to be a plain overwrite, so following the
    /// instruction the check itself prints deleted exactly those rows. Two
    /// went missing the last time anyone looked, and the number is a property
    /// of the run rather than of the tree.
    ///
    /// Merging makes the writer as tolerant as the reader. A field that is
    /// genuinely gone now has to be deleted on purpose, which is the right
    /// cost for removing something from a versioned contract. Where both sides
    /// carry a path, this run's type wins: the committed one is a record of an
    /// older measurement and this one is current.
    ///
    /// See `TODO/cli-surface.md`, T-158.
    fn merge_schema(committed: &str, rendered: &str) -> String {
        let previous = rows_by_section(committed);
        let mut out = String::new();
        let mut part = String::new();
        let mut section = String::new();
        let mut pending: Vec<String> = Vec::new();

        for line in rendered.lines() {
            let is_row = row_path(line).is_some();
            if !is_row {
                flush_section(&mut out, &previous, &part, &section, &mut pending);
            }
            if let Some(rest) = line.strip_prefix("## ") {
                part = rest.to_string();
                section.clear();
            } else if let Some(rest) = line.strip_prefix("### ") {
                section = rest.to_string();
            }
            match is_row {
                true => pending.push(line.to_string()),
                false => {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        flush_section(&mut out, &previous, &part, &section, &mut pending);
        out
    }

    /// Merging keeps a row this run did not produce, and adds the ones it did.
    ///
    /// The regression is the whole entry: following the documented way to
    /// update `docs/schema.md` used to delete a documented field. See
    /// `TODO/cli-surface.md`, T-158.
    #[test]
    fn regenerating_the_schema_keeps_rows_this_run_did_not_produce() {
        let committed = "## Documents\n\n### `download`\n\n| field | type |\n| --- | --- |\n| `kept` | string |\n| `shared` | integer |\n\n## Events\n\n### `download`\n\n| field | type |\n| --- | --- |\n| `other_section` | string |\n";
        let rendered = "## Documents\n\n### `download`\n\n| field | type |\n| --- | --- |\n| `added` | integer |\n| `shared` | string |\n";
        let merged = merge_schema(committed, rendered);

        assert!(
            merged.contains("| `kept` | string |"),
            "a row this run did not produce has to survive:\n{merged}"
        );
        assert!(
            merged.contains("| `added` | integer |"),
            "a row this run did produce has to be added:\n{merged}"
        );
        assert!(
            merged.contains("| `shared` | string |"),
            "where both carry a path, this run's type wins:\n{merged}"
        );
        assert!(
            !merged.contains("| `shared` | integer |"),
            "the older type must not survive beside the newer one:\n{merged}"
        );
        assert!(
            !merged.contains("other_section"),
            "a row from a different section must not leak in:\n{merged}"
        );

        // Rows stay sorted by path, which is the order `fields` produces, so
        // merging does not churn the diff.
        let added = merged.find("| `added`").unwrap();
        let kept = merged.find("| `kept`").unwrap();
        let shared = merged.find("| `shared`").unwrap();
        assert!(added < kept && kept < shared, "{merged}");

        // The separator row is not a field row and has to survive as prose.
        assert!(merged.contains("| --- | --- |"), "{merged}");
    }

    /// Merging twice changes nothing the first merge did not.
    ///
    /// This is the acceptance in T-158's own words: regenerating twice in a
    /// row leaves every row that either run produced, and `git diff` is empty
    /// when nothing changed.
    #[test]
    fn regenerating_the_schema_is_idempotent() {
        let (documents, events) = collect();
        let rendered = render(&documents, &events);
        let once = merge_schema(&rendered, &rendered);
        let twice = merge_schema(&once, &rendered);
        assert_eq!(once, twice, "a second regeneration must be a no-op");
        assert_eq!(
            once, rendered,
            "merging a render into itself has to reproduce it exactly"
        );
    }

    /// The committed contract matches what the program writes.
    ///
    /// Adding a field to a report changes the generated text, so this fails
    /// until `docs/schema.md` is regenerated. That is the whole mechanism: a
    /// versioned contract nobody has written down is not a contract, and one
    /// written by hand goes stale the first time a field moves.
    #[test]
    fn the_committed_schema_matches_what_the_program_writes() {
        let (documents, events) = collect();
        let rendered = render(&documents, &events);
        let path = schema_path();

        if std::env::var_os("BIT_CLI_UPDATE_SCHEMA").is_some() {
            // Merged with what is already committed rather than written over
            // it. See [`merge_schema`] and `TODO/cli-surface.md`, T-158.
            // Line endings normalised first, for the same reason the read side
            // normalises them: the file is checked in as LF and git may hand it
            // over as CRLF, and a merge that compared the two forms would keep
            // both copies of every row.
            let merged = match std::fs::read_to_string(&path) {
                Ok(committed) => merge_schema(&committed.replace("\r\n", "\n"), &rendered),
                Err(_) => rendered.clone(),
            };
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(&path, &merged).expect("write the schema");
            return;
        }

        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nRegenerate with BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema",
                path.display()
            )
        });
        // Compare with line endings normalised: the file is checked in as LF
        // and git may have handed it over as CRLF.
        let committed = committed.replace("\r\n", "\n");

        // A containment check rather than equality, and the asymmetry is the
        // point. A field added to a report produces a row the committed file
        // does not have, which fails here. A row the committed file has and
        // this run did not produce does not fail, because these runs are
        // timed: a download that finished before its second report tick emits
        // no `progress`, and a run that raced its own deadline emits no
        // `torrent_completed`. Requiring equality would make the contract
        // check flaky, and a flaky contract check is worse than none.
        let missing: Vec<&str> = rendered
            .lines()
            .filter(|line| line.starts_with("| `"))
            .filter(|line| !committed.contains(*line))
            .collect();
        assert!(
            missing.is_empty(),
            "docs/schema.md does not describe {} field(s) this run produced:\n  {}\nRegenerate with BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema",
            missing.len(),
            missing.join("\n  ")
        );

        // The prose and the section headings are not timing dependent, so
        // those do have to match exactly: a kind added to the tables without
        // regenerating leaves the file without its section.
        let headings: Vec<&str> = rendered
            .lines()
            .filter(|line| line.starts_with("### "))
            .filter(|line| !committed.contains(*line))
            .collect();
        assert!(
            headings.is_empty(),
            "docs/schema.md is missing sections: {headings:?}\nRegenerate with BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema"
        );
    }

    /// Coverage does not go backwards.
    ///
    /// A name the generator drives no run for is listed in
    /// `schema::NOT_YET_COVERED`, so this fails when that set grows and not
    /// for the gap already recorded. It also fails when a name is still on
    /// that list after a run started producing it, so the list cannot go
    /// stale in the other direction. See `TODO/cli-surface.md`, T-117.
    #[test]
    fn coverage_of_the_documented_names_matches_what_is_recorded() {
        let (documents, events) = collect();
        let mut missing: Vec<&str> = DOCUMENT_KINDS
            .iter()
            .map(|(kind, _)| *kind)
            .filter(|kind| !documents.iter().any(|sample| sample.name == *kind))
            .chain(
                EVENT_TYPES
                    .iter()
                    .map(|(event, _)| *event)
                    .filter(|event| !events.iter().any(|sample| sample.name == *event)),
            )
            .collect();
        missing.sort_unstable();
        assert_eq!(
            missing,
            crate::schema::NOT_YET_COVERED,
            "the set of names no run produces changed; update schema::NOT_YET_COVERED and TODO/cli-surface.md T-117"
        );
    }

    /// Nothing the program writes is undocumented, which is the other
    /// direction of the same promise.
    #[test]
    fn every_produced_kind_and_event_is_documented() {
        let (documents, events) = collect();
        // Named with the command that produced them. An undocumented `kind` is
        // usually an error document from a run that was meant to succeed, and
        // the name alone does not say which run that was.
        let undocumented_documents: Vec<String> = documents
            .iter()
            .filter(|sample| !DOCUMENT_KINDS.iter().any(|(kind, _)| *kind == sample.name))
            .map(|sample| format!("{} from `{}`", sample.name, sample.command))
            .collect();
        let undocumented_events: Vec<String> = events
            .iter()
            .filter(|sample| !EVENT_TYPES.iter().any(|(event, _)| *event == sample.name))
            .map(|sample| format!("{} from `{}`", sample.name, sample.command))
            .collect();
        assert!(
            undocumented_documents.is_empty() && undocumented_events.is_empty(),
            "produced but not documented: documents {undocumented_documents:?}, events {undocumented_events:?}"
        );
    }
}
