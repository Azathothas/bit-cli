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
    let Some(kind) = value.get("kind").and_then(|k| k.as_str()) else {
        return;
    };
    let flattened = fields(&value);
    into.entry(kind.to_string())
        .and_modify(|sample| sample.merge(flattened.clone()))
        .or_insert_with(|| Sample {
            name: kind.to_string(),
            command: command.to_string(),
            fields: flattened,
        });
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

    // A download that cannot finish, for `source_failed` and `error`, and a
    // stalled one for `peer_redial`.
    for (label, args) in [
        (
            "bit-cli download <TORRENT> --web-seed <DEAD URL> --jsonl",
            vec![
                "--jsonl",
                "download",
                &torrent,
                "--dir",
                dir.join("dead").to_str().unwrap(),
                "--web-seed-only",
                "--web-seed",
                "http://127.0.0.1:9/",
                "--no-tracker",
                "--port",
                "0",
                "--stop-after",
                "10s",
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
            ],
            dir.clone(),
        );
        match format {
            "--json" => observe_document(&mut documents, "bit-cli seed <TORRENT> --json", &out),
            _ => observe_events(&mut events, "bit-cli seed <TORRENT> --jsonl", &out),
        }
    }

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
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(&path, &rendered).expect("write the schema");
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
        let undocumented_documents: Vec<&str> = documents
            .iter()
            .map(|sample| sample.name.as_str())
            .filter(|name| !DOCUMENT_KINDS.iter().any(|(kind, _)| kind == name))
            .collect();
        let undocumented_events: Vec<&str> = events
            .iter()
            .map(|sample| sample.name.as_str())
            .filter(|name| !EVENT_TYPES.iter().any(|(event, _)| event == name))
            .collect();
        assert!(
            undocumented_documents.is_empty() && undocumented_events.is_empty(),
            "produced but not documented: documents {undocumented_documents:?}, events {undocumented_events:?}"
        );
    }
}
