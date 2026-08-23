//! The JSON contract, described from what the program actually writes.
//!
//! `--schema-version` prints a number. A number that refers to nothing a caller
//! can check against is not a contract, so `docs/schema.md` lists every
//! document `kind` and every event `type` with the fields each one carries.
//!
//! The document is **generated**, not written. `schema::render` takes the JSON
//! a real run produced and flattens it into a field table, and a test drives
//! every command, renders the whole file, and fails when it differs from what
//! is committed. So a field added to a report changes the generated text and
//! the test says so, rather than the documentation quietly going stale.
//!
//! Regenerate with:
//!
//! ```text
//! BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema
//! ```
//!
//! See `TODO/cli-surface.md`, T-117 and T-110.

use std::collections::BTreeMap;

use serde_json::Value;

/// Where the generated document lives, relative to the repository root.
pub const SCHEMA_PATH: &str = "docs/schema.md";

/// Every document `kind` `bit-cli` emits under `--json`, with what it is.
///
/// The order here is the order in the document. It is grouped by what a reader
/// is doing rather than alphabetically, because the alphabet puts `config`
/// before `download`.
pub const DOCUMENT_KINDS: &[(&str, &str)] = &[
    (
        "info",
        "One torrent's metadata, without touching the network.",
    ),
    (
        "files",
        "The files in a torrent, with sizes, offsets, and piece ranges.",
    ),
    (
        "magnet",
        "A magnet URI built from a torrent, and its parts.",
    ),
    (
        "verify",
        "What a hash check of existing data found, piece by piece.",
    ),
    (
        "hash_mismatch",
        "The document `verify` writes instead when a piece did not check out.",
    ),
    (
        "create",
        "A torrent that was just written, and what went into it.",
    ),
    (
        "edit",
        "A torrent rewritten with new trackers or sources, and its info hash before and after.",
    ),
    (
        "download",
        "A finished download: what arrived, from where, and what it cost.",
    ),
    (
        "download_dry_run",
        "What `download --dry-run` resolved: the sources, what each one is, what it would cost, and whether the network is needed. It has its own `kind` because it shares almost no fields with a real run, and a consumer selecting by `kind` would otherwise get two shapes under one name. `dry_run: true` is also on the document. See `TODO/cli-surface.md`, T-156.",
    ),
    (
        "seed",
        "A finished seeding run: who connected and what they took.",
    ),
    ("peers", "The swarm as sampled over a window."),
    ("trackers", "What each tracker answered."),
    (
        "webseed_list",
        "Every source binding resolved to the exact URLs it would request.",
    ),
    (
        "webseed_test",
        "One request per source: status, ranges, redirects, and timing.",
    ),
    (
        "webseed_probe",
        "A source measured at several concurrencies.",
    ),
    (
        "webseed_fetch",
        "One piece pulled from one source and checked.",
    ),
    (
        "config",
        "Configuration as resolved, with where each value came from.",
    ),
    (
        "version",
        "The build, its features, and the exit code table.",
    ),
    (
        "disk",
        "The report a `bench` run writes, measured here from `bench disk`. Every target writes this document with its own `kind`. `environment` describes the machine rather than the measurement and is left out: it carries fields one platform has and another does not, so a contract holding it would say which machine last regenerated this file. See `TODO/bench.md`, T-189.",
    ),
];

/// Every event `type` `bit-cli` emits under `--jsonl`, with what it means.
///
/// Ordered by when a run emits them, because that is how a reader consuming
/// the stream meets them.
pub const EVENT_TYPES: &[(&str, &str)] = &[
    (
        "session_start",
        "The session is up. Carries the listen address and what it was asked to do.",
    ),
    (
        "torrent_added",
        "A source resolved to a torrent and was added to the session.",
    ),
    (
        "metadata_resolved",
        "The torrent's metadata is known: name, files, pieces.",
    ),
    (
        "source_added",
        "An HTTP or `file:` source was attached, with its scope.",
    ),
    (
        "source_failed",
        "A source is out for the run: it spent its error budget, or it was proved to have served bytes the session then verified as something else. `sources[].convictions` says which, and names the block.",
    ),
    (
        "source_cooling",
        "A source spent its error budget and will be tried again after `--web-seed-cooldown`.",
    ),
    (
        "peer_redial",
        "`--redial-after` fired: every peer connection was dropped and the peer list dialled again.",
    ),
    (
        "metalink_resolved",
        "A Metalink was read and the `.torrent` it names was fetched.",
    ),
    (
        "metalink_checked",
        "The payload was checked against the Metalink's own checksum. `not_checked` says why it was not, when it was not.",
    ),
    (
        "piece_verified",
        "A piece arrived and its hash checked out.",
    ),
    ("file_completed", "Every piece of one file is present."),
    (
        "progress",
        "A tick of the report interval: rates, peers, and what the process costs.",
    ),
    ("bench_sample", "One point of a `bench` time series."),
    (
        "torrent_completed",
        "One torrent finished, with its totals.",
    ),
    (
        "error",
        "Something failed. The same shape the final error document carries.",
    ),
    (
        "session_end",
        "The run is over. Always last, always present, whatever happened.",
    ),
];

/// Names the generator does not yet drive a run for.
///
/// Empty: every document kind and every event type has a run behind it. The
/// constant stays because the coverage test compares against it, so a name
/// that stops being produced fails the build here rather than quietly losing
/// its field table. See `TODO/cli-surface.md`, T-117.
pub const NOT_YET_COVERED: &[&str] = &[];

/// The JSON type of a value, as the document names it.
fn type_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(number) => match number.is_f64() {
            true => "float",
            false => "integer",
        },
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Flatten a document into `path -> type`, one row per leaf.
///
/// Nested objects become dotted paths and arrays become `[]`, so
/// `sources[].scope` is one row however many sources a run had. An empty array
/// contributes its own row and nothing under it, because a run that produced
/// none cannot say what one holds.
pub fn fields(value: &Value) -> BTreeMap<String, &'static str> {
    let mut out = BTreeMap::new();
    walk("", value, &mut out);
    out
}

fn walk(prefix: &str, value: &Value, out: &mut BTreeMap<String, &'static str>) {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                let path = match prefix.is_empty() {
                    true => key.clone(),
                    false => format!("{prefix}.{key}"),
                };
                match child {
                    Value::Object(_) | Value::Array(_) => walk(&path, child, out),
                    _ => {
                        out.insert(path, type_of(child));
                    }
                }
            }
        }
        Value::Array(items) => {
            let path = format!("{prefix}[]");
            if items.is_empty() {
                out.insert(path, "array");
                return;
            }
            for item in items {
                match item {
                    Value::Object(_) | Value::Array(_) => walk(&path, item, out),
                    _ => {
                        out.insert(path.clone(), type_of(item));
                    }
                }
            }
        }
        _ => {
            out.insert(prefix.to_string(), type_of(value));
        }
    }
}

/// One documented shape: what produced it and what it carried.
pub struct Sample {
    /// The `kind` or event `type`.
    pub name: String,
    /// The command that produced it, with volatile arguments removed.
    pub command: String,
    /// Every field, flattened.
    pub fields: BTreeMap<String, &'static str>,
}

impl Sample {
    /// Fold another observation of the same shape in.
    ///
    /// One run rarely exercises every optional field: a download with no
    /// renamed paths omits `renamed`, and one with no sources omits
    /// `sources[]`. Several runs of the same command union together into the
    /// shape a reader should expect.
    pub fn merge(&mut self, other: BTreeMap<String, &'static str>) {
        for (path, kind) in other {
            self.fields.entry(path).or_insert(kind);
        }
    }
}

/// Render the whole document.
pub fn render(documents: &[Sample], events: &[Sample]) -> String {
    let mut out = String::new();
    out.push_str(HEADER);

    out.push_str("\n## Documents\n\nOne document per run, on stdout, when `--json` is given.\n");
    for (kind, description) in DOCUMENT_KINDS {
        out.push_str(&format!("\n### `{kind}`\n\n{description}\n"));
        match documents.iter().find(|sample| sample.name == *kind) {
            Some(sample) => out.push_str(&section(sample)),
            None => out.push_str(&format!(
                "\nNot covered by the generator yet, so its fields are not listed here. See\n`{SCHEMA_PATH}`'s note above.\n"
            )),
        }
    }

    out.push_str(
        "\n## Events\n\nOne object per line, on stdout, when `--jsonl` is given. Every event carries\n`type`, `seq`, and `at` before its own fields; `seq` counts from zero within a\nrun and `at` is ISO 8601 UTC with millisecond precision.\n",
    );
    for (event, description) in EVENT_TYPES {
        out.push_str(&format!("\n### `{event}`\n\n{description}\n"));
        match events.iter().find(|sample| sample.name == *event) {
            Some(sample) => out.push_str(&section(sample)),
            None => out.push_str(
                "\nNot produced by any run the generator drives, so its fields are not listed\nhere.\n",
            ),
        }
    }
    out
}

fn section(sample: &Sample) -> String {
    let mut out = format!(
        "\nFrom `{}`.\n\n| field | type |\n| --- | --- |\n",
        sample.command
    );
    for (path, kind) in &sample.fields {
        out.push_str(&format!("| `{path}` | {kind} |\n"));
    }
    out
}

const HEADER: &str = r##"# The JSON contract

`bit-cli --schema-version` prints the version of everything below. This file is
what that number refers to.

Two surfaces, and they never mix. `--json` writes one document to stdout when
the run ends. `--jsonl` writes one object per line as things happen. stdout
carries data only in both, at every log level, so `bit-cli ... --json | jq`
never sees a log line.

Every document carries four fields before its own: `schema_version`,
`bit_cli_version`, `generated_at`, and `kind`. Every event carries `type`,
`seq`, and `at`.

A `bench` report is the exception, and it is the only one. It carries `kind`
and a `report_version` of its own, because `--baseline` reads a report written
by an older build and has to know which format it is holding. Its `environment`
object is not listed below either: that describes the machine a run was taken
on, and it carries fields one platform has and another does not. See
`TODO/bench.md`, T-189.

Sizes and durations are always an integer plus a rendered string, never the
string alone: `{"bytes": 1048576, "human": "1.00 MiB"}` and
`{"ms": 1500, "human": "1s"}`. Rates use the same shape as a size with
`MiB/s` in the string. Timestamps are ISO 8601 UTC with millisecond precision.

## How this file is kept true

It is generated from what the program actually writes. A test drives every
command, flattens the JSON it produced, renders this file, and fails when the
result differs from what is committed. A field added to a report therefore
fails the build until this file is regenerated:

```bash
BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema
```

A field that a given run did not produce is not listed. Optional fields are
omitted from the JSON rather than written as `null`, so a reader cannot mistake
"not applicable" for "none", and several runs of the same command are folded
together here to cover as many of them as possible.

The check is containment, not equality: a row this file has and a run did not
produce passes, because these runs are timed and a failure-only field like
`sources[].error` appears only when a source fails. **So regenerating is
lossy.** After running the command above, read the diff and put back any row it
removed that is still a real field. There is no automatic way to tell a stale
row from a rare one.
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattening_names_a_nested_field_by_its_path() {
        let value = serde_json::json!({
            "kind": "info",
            "total": { "bytes": 1024, "human": "1.00 KiB" },
            "trackers": ["udp://a", "udp://b"],
            "files": [{ "index": 0, "path": "a.bin" }],
            "nodes": [],
            "private": false,
        });
        let fields = fields(&value);
        assert_eq!(fields["kind"], "string");
        assert_eq!(fields["total.bytes"], "integer");
        assert_eq!(fields["total.human"], "string");
        assert_eq!(fields["trackers[]"], "string");
        assert_eq!(fields["files[].index"], "integer");
        assert_eq!(fields["files[].path"], "string");
        assert_eq!(
            fields["nodes[]"], "array",
            "an empty array says nothing more"
        );
        assert_eq!(fields["private"], "bool");
    }

    /// Two runs of one command rarely carry the same optional fields, so the
    /// union is what a reader should expect.
    #[test]
    fn merging_two_observations_keeps_every_field_either_one_had() {
        let mut sample = Sample {
            name: "download".into(),
            command: "bit-cli download".into(),
            fields: fields(&serde_json::json!({ "a": 1, "b": "x" })),
        };
        sample.merge(fields(&serde_json::json!({ "b": "y", "c": true })));
        assert_eq!(sample.fields.len(), 3);
        assert_eq!(sample.fields["a"], "integer");
        assert_eq!(sample.fields["c"], "bool");
    }

    /// Every name in the two tables is unique, so a section cannot be written
    /// twice and a reader cannot be given two answers.
    #[test]
    fn every_documented_name_appears_once() {
        for table in [DOCUMENT_KINDS, EVENT_TYPES] {
            let mut seen = std::collections::BTreeSet::new();
            for (name, description) in table {
                assert!(seen.insert(*name), "{name} is listed twice");
                assert!(!description.is_empty(), "{name} has no description");
            }
        }
    }
}
