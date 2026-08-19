# CLI surface gaps

Everything in A3 that parses today and does not yet do what `--help` says. A
flag that looks like it works and does not is worse than one that errors, so
each of these either ships or starts refusing.

This file is not in the A4 file list. It exists because these items belong to no
upstream category, and dropping them to match a list would lose them.

---

### T-110 The --jsonl event stream is incomplete

Source:      PROMPT.md A3.10
Category:    cli
Priority:    P1
Effort:      M
Status:      partial

Problem:     A3.10 documents eleven event types. `download` emits
             `session_start`, `torrent_added`, `metadata_resolved`,
             `piece_verified`, `file_completed`, `source_added`,
             `source_failed`, `progress`, `torrent_completed`, and `error`.
             `session_end` is emitted by nothing, and `seed`, `peers`, and
             `trackers` emit only `session_start` and `progress`.
Relevance:   An agent consuming NDJSON needs the stream to end with something
             that says it ended, or it cannot tell "finished" from "the pipe
             broke".
Approach:    Emit `session_end` from the one place every command returns
             through, carrying the exit code and the elapsed time, so it cannot
             be forgotten per command. Then audit each command against the
             eleven.
Acceptance:  `bit-cli <any command> --jsonl` ends with a `session_end` event
             carrying `exit_code`, and `docs/schema.md` has a worked example of
             every type.

### T-111 piece_verified and file_completed are derived from polling

Source:      PROMPT.md A3.10
Category:    cli
Priority:    P2
Effort:      M
Status:      open

Problem:     Both events come from comparing consecutive snapshots on the
             report interval rather than from the engine pushing them. The
             counts are exact; the timestamps are only as precise as
             `--report-interval`.
Relevance:   For a caller measuring per-piece timing, an event stamped up to a
             second late is not a measurement. Rule 0.2 says an estimate has to
             say it is one.
Approach:    Either take a push notification from `librqbit` if one exists, or
             name the imprecision in the event: add `"timing": "polled"` and
             the interval, so a consumer knows what the timestamp is worth.
Acceptance:  Each `piece_verified` event says how its timestamp was obtained.

### T-112 --log-file does not write or rotate anything

Source:      PROMPT.md A3.2
Category:    cli
Priority:    P1
Effort:      M
Status:      open

Problem:     `--log-file`, `--log-max-size`, and `--log-max-files` all parse,
             and `--log-max-size` is even validated, but no file is opened.
Relevance:   A cron job that cannot keep a log has no way to explain a failure
             after the fact.
Approach:    Append to the named file, rotate at `--log-max-size` by renaming
             to `.1`, `.2`, and so on, and keep `--log-max-files` of them.
             Rotation on Windows has to handle a reader holding the file open,
             which means retrying the rename with backoff.
Acceptance:  A run with `--log-file x.log --log-max-size 1KiB --log-max-files 3`
             produces `x.log`, `x.log.1`, `x.log.2`, and no `x.log.3`.

### T-113 Metalink is not implemented

Source:      PROMPT.md A3.4c, decision 7.7
Category:    cli
Priority:    P1
Effort:      L
Status:      open

Problem:     `source.rs` classifies `.meta4` and `.metalink` and reports that
             they need resolving. Nothing resolves them. `quick-xml` is already
             a dependency and unused.
Relevance:   Metalink is in scope because it is a torrent format: one file
             carrying a `.torrent`, a mirror list, and checksums, which is
             exactly the hybrid case this tool exists for. Everything a user
             would otherwise assemble with `--web-seed` repeated twelve times,
             a Metalink gives in one file.
Approach:    RFC 5854 for `.meta4`, plus the older `.metalink`. Parse the
             `<metaurl mediatype="torrent">` entry to find the torrent, the
             `<url>` entries to register as web seeds, `<size>`, and
             `<hash type="sha-256">`. Then the part that matters: verify the
             checksums the Metalink supplies against the piece hashes the
             torrent supplies, and report loudly if they disagree, because that
             means one of the two is wrong and the caller needs to know which.
             Out of scope: language and OS filtering, version negotiation.
Acceptance:  `bit-cli download release.meta4` resolves the torrent, registers
             every listed mirror, downloads, and verifies against the
             Metalink's own checksum. Run against a real `.meta4`.

### T-114 -i/--input-file batch input is not implemented

Source:      PROMPT.md A3.8
Category:    cli
Priority:    P2
Effort:      M
Status:      open

Problem:     `aria2` takes one source per line with indented option lines
             beneath it applying to that entry only. `bit-cli` has no `-i`.
Relevance:   It is how a script drives a hundred downloads with per-entry
             options, and `-i` is one of the reserved `aria2` letters.
Approach:    Parse the `aria2` format exactly, because the point is that an
             existing input file works unchanged. An unindented line is a
             source; an indented `key=value` line sets an option for the
             preceding source only. Reject an option that is not a known flag
             rather than ignoring it.
Acceptance:  An `aria2` input file with three sources and per-entry `dir` and
             `out` options drives `bit-cli download -i` to the same result
             `aria2c -i` produces.

### T-115 Hooks do not fire for every documented trigger

Source:      PROMPT.md A3.7
Category:    cli
Priority:    P2
Effort:      S
Status:      partial

Problem:     `--on-complete` and `--on-error` run once for the whole `download`
             run. `--on-piece-verified` does not run at all, and neither hook
             runs from `seed`.
Relevance:   `--on-complete` firing once per run rather than once per torrent
             is wrong for a `-j 4` invocation.
Approach:    Fire per torrent, from the same place `torrent_completed` is
             emitted. `--on-piece-verified` is high frequency by construction,
             so it needs a documented cost and probably a rate limit. Arguments
             already arrive through the environment as `BIT_CLI_*` and never by
             interpolation into a shell string, which is the part that matters
             for a torrent-supplied filename.
Acceptance:  `bit-cli download a.torrent b.torrent -j 2 --on-complete <CMD>`
             runs the command twice, once per torrent, with
             `BIT_CLI_INFO_HASH` differing. `docs/` lists every variable.

### T-116 -O/--index-out cannot rename a file

Source:      PROMPT.md A3.7
Category:    cli
Priority:    P3
Effort:      S
Status:      open

Problem:     `-O/--index-out INDEX=PATH` parses and does nothing.
Relevance:   It is a reserved `aria2` letter and the natural answer to a
             torrent whose paths collide on Windows, T-072.
Approach:    Needs a storage wrapper mapping a torrent file index to a
             different on-disk path, which is the same machinery T-071 needs
             for sanitisation. Build them together.
Acceptance:  `bit-cli download <TORRENT> -O 0=renamed.bin` writes the first
             file as `renamed.bin` and `--json` reports the mapping.

### T-117 --schema-version has no schema behind it

Source:      PROMPT.md A3.10
Category:    cli
Priority:    P1
Effort:      M
Status:      open

Problem:     `--schema-version` prints `1`. There is no `docs/schema.md`, so
             the number refers to nothing a caller can check against.
Relevance:   A versioned contract nobody has written down is not a contract.
Approach:    Document every JSON document and every event type with a worked
             example, generated from the real types rather than written by hand
             so it cannot drift. A test that serialises one of each and checks
             the example still matches is the mechanism.
Acceptance:  `docs/schema.md` exists, covers every `kind` and every event
             `type`, and a test fails when a field is added without updating it.

### T-118 The short-flag table is not checked in CI

Source:      PROMPT.md A3.2
Category:    cli
Priority:    P2
Effort:      S
Status:      open

Problem:     A3.2 requires `docs/flags.md` with the full short-flag table and a
             CI check, so a new subcommand cannot quietly reuse a letter that
             `aria2` assigns to something else. Neither the file nor the check
             exists.
Relevance:   A script written from `aria2` muscle memory doing something else
             silently is the failure this prevents.
Approach:    Generate the table from the `clap` command tree, compare it to the
             reserved list in A3.2, and fail on any letter used for a different
             concept.
Acceptance:  `docs/flags.md` exists and a test regenerates it and fails on
             drift.
