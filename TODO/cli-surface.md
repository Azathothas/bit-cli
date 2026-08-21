# CLI surface gaps

Everything in A3 that parses today and does not yet do what `--help` says. A
flag that looks like it works and does not is worse than one that errors, so
each of these either ships or starts refusing.

This file is not in the A4 file list. It exists because these items belong to no
upstream category, and dropping them to match a list would lose them.

---

### T-110 The --jsonl event stream is incomplete

Source:      the operator's brief
Category:    cli
Priority:    P1
Effort:      M
Status:      **done**

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

**Done.** `session_end` is emitted from `bit_cli::run`, the one place every
command returns through, so a command added later cannot forget it. It carries
`exit_code`, `exit_status`, `ok`, `elapsed_ms`, `elapsed_human`, and `error`
when there was one.

```
$ bit-cli --jsonl info album.torrent | tail -1
{"at":"2026-08-20T15:01:59.553Z","elapsed_human":"4ms","elapsed_ms":4,
 "exit_code":0,"exit_status":"success","ok":true,"seq":0,"type":"session_end"}
```

Three tests: `every_jsonl_run_ends_with_session_end` walks every command that
runs without a network and checks the last line of each,
`a_failed_jsonl_run_ends_with_session_end_carrying_the_error` checks the
failure shape, and `session_end_does_not_appear_outside_jsonl` checks that
`--json` and text output do not gain a stray object.

The one case with no event is a flag that `clap` refuses: before the arguments
parse there is no format to emit one in, so a usage error ends the stream by
ending it. That is stated in `run`.

**It broke one reader, and that is worth knowing before adding another event.**
`scripts/interop-roundtrip.ps1` read the seeder's report as the last line of
its `--jsonl` stream, which was right until `session_end` became the last line.
Both seeding cases then failed with "bit-cli seed served no peer" while the
transfer had in fact succeeded: 490,012 bytes uploaded to `aria2/1.37.0`, in
the stream, two lines up. The script now walks backwards for the object whose
`kind` is `seed`. Anything else consuming this stream by position rather than
by `type` or `kind` has the same fault.

The audit the Approach asks for is `docs/schema.md`, built by
[T-117](#t-117---schema-version-has-no-schema-behind-it). Fourteen event types
are documented, not the eleven A3.10 lists: `source_cooling`, `peer_redial`,
and `bench_sample` were added by later entries.

### T-111 piece_verified and file_completed are derived from polling

Source:      the operator's brief
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

Source:      the operator's brief
Category:    cli
Priority:    P1
Effort:      M
Status:      **done**

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

**Done, exactly as the acceptance states it.**

```
$ bit-cli download torrent_c.torrent --dir out --web-seed file:///.../ \
    --web-seed-only --port 0 --allow-overwrite -vvv \
    --log-file x.log --log-max-size 1KiB --log-max-files 3

-rw-r--r--  258 x.log
-rw-r--r-- 1022 x.log.1
-rw-r--r-- 1002 x.log.2
```

`crates/bit-cli/src/logging.rs` holds a `Rotating` writer behind a mutex, given
to `tracing_subscriber` as a second destination through `MakeWriterExt::and`.
Four decisions in it, each for a reason a reader should not have to guess:

- **It adds a destination rather than replacing stderr.** Ground rule 0.11 says
  stderr carries the logs, and it should hold whatever else is set. A caller
  who wants only the file redirects stderr, which is one shell operator against
  a rule that would otherwise have an exception in it.
- **`--log-max-files N` is N files in total**, the live one included, so `3`
  leaves `x.log`, `.1`, and `.2`. `1` keeps no history and starts the live file
  over rather than leaving a rotated copy the caller said it did not want.
- **The size is seeded from the file that is already there.** Appending to a
  full log rotates on the first write rather than after this process has
  written a whole file's worth of its own.
- **A rename that will not happen is skipped, not fatal.** Windows refuses to
  rename a file another process has open, and a log file is exactly the file
  someone is tailing. Five attempts with a doubling wait covers a reader
  between reads; past that the log keeps growing, which is better than losing
  a line or failing the run.

Five tests in `logging::tests`, four of them driving the writer directly
because a run producing exactly 1 KiB of log lines would be testing the log
volume rather than the rotation:
`rotation_keeps_the_live_file_and_max_files_minus_one_behind_it`,
`a_zero_max_size_never_rotates`,
`one_file_total_truncates_instead_of_keeping_a_copy`,
`an_existing_full_log_rotates_on_the_next_write`, and
`a_run_with_a_log_file_writes_to_it_and_still_writes_to_stderr`.

### T-113 Metalink is not implemented

Source:      the operator's brief, decision 7.7
Category:    cli
Priority:    P1
Effort:      L
Status:      **done**

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

**The parser is done and the wiring is not.** `bit_cli_core::metalink` reads
both versions in one pass over `quick-xml` events, which is the half of this
with all the format knowledge in it:

| | Metalink 4, RFC 5854, `.meta4` | Metalink 3, `.metalink` |
| --- | --- | --- |
| files | `<file>` under `<metalink>` | `<file>` under `<files>` |
| hashes | `<hash type="sha-256">` | `<hash type="sha256">` under `<verification>` |
| mirrors | `<url>` | `<url type="http">` under `<resources>` |
| torrent | `<metaurl mediatype="torrent">` | `<url type="bittorrent">` |
| preference | `priority`, **lower** first | `preference`, **higher** first |

Both come out of the parser under version 4's rule, so a caller sorting by
`priority` gets the document's intent whichever file it read, and `sha-256` and
`sha256` normalise to one spelling.

Four things it refuses or drops on purpose, each with a test:

- A `<metaurl>` that is not a torrent is dropped rather than registered as a
  mirror. It names another document, so a source pointed at it would serve XML
  as payload.
- The per-piece `<hash piece="0">` entries under `<pieces>` are not whole-file
  checksums and are not collected as if they were. Without that a version 3
  file comes out with four checksums, two of which are one piece each.
- `ftp:` mirrors are kept out of the source list and counted, because a source
  this cannot fetch from is worse than one it never had.
- A document that simply stops is refused. `check_end_names` catches a
  mismatched closing tag and not an EOF, so the parser counts depth and fails
  at zero-plus-open. A truncated mirror list that parses is the "plausible
  wrong answer" this repository keeps finding.

```
$ cargo test -p bit-cli-core --lib metalink
test result: ok. 15 passed; 0 failed
```

**The wiring is done and the five steps are closed.** `bit-cli download
release.meta4` reads the document, fetches the `.torrent` it names, registers
every mirror as a source, downloads, and checks the payload against the
document's own checksum.

```
$ pwsh scripts/check-metalink.ps1
verdict: pass          (ten cases on loopback)
$ pwsh scripts/check-metalink-real.ps1
verdict: pass          (four cases against download.documentfoundation.org)
```

Both records are committed: `bench/metalink-20260821T045751697Z.json` and
`bench/metalink-real-20260821T045805559Z.json`.

What each step turned into.

1. **Resolving the torrent.** `source::resolve_metalink` reads the document,
   takes `single_file()`, and fetches the torrent. Not `torrents[0]`:
   `torrents_by_priority()`, and each in turn until one parses, because a
   document that lists several torrents is a mirror list for the `.torrent`
   itself and its first choice can be gone. The failures are kept and reported
   as `torrent_fallbacks`, so a report says the preferred one was not the one
   used. `source::fetch_torrent` now returns the bytes as well as the parse,
   and `Engine::add_bytes` hands those exact bytes to the session.
   **Fetching the URL twice was the alternative and it is wrong**: the session
   would fetch a URL this run has already fetched, and two fetches of one URL
   can return two documents, so the report would describe one torrent while the
   session downloaded another.
2. **Registering the mirrors.** `webseed_args::collect` takes an
   `Option<&MetalinkFile>` and emits one `SourceSpec` per mirror in
   `mirrors_by_priority()` order, with `Origin::Metalink`, which already
   existed and had no producer.

   Two things the entry did not anticipate. The composition is **`exact`**, not
   BEP 19's `auto`: a Metalink `<url>` is the complete resource, never a
   directory to append a name to. And `exact` on a multi-file torrent is a
   binding error unless the scope resolves to one file, so the scope is the
   file the document was attributed to. A document that cannot be attributed to
   exactly one file of a multi-file torrent registers **nothing**, because a
   mirror serving one file's bytes into a piece range nobody has identified is
   worse than no mirror.

   `--no-torrent-web-seed` drops them, and its help now says "the torrent's or
   the metalink's". Both mean "the sources the source document declared rather
   than the ones you named", which is one idea under one flag.
3. **Verifying the checksum.** `Checksum::verify_file` streams the file in 256
   KiB reads through `sha2`, `sha1`, or `md-5`. `sha2` was a declared
   dependency of `bit-cli-core` with no user until now.

   An algorithm this cannot compute is an **error, not a pass**. The report
   carries `not_checked` with the reason, and `matched` is absent rather than
   `true`. Every guard that stops the check writes one: a download that did not
   finish, a file that could not be named on disk, an attribution that failed.
   A checksum that was not computed is not a checksum that passed.
4. **Which document is wrong.** This is the part the entry called the part that
   matters, and it turned into two checks rather than one.

   The **size** check costs nothing and runs before a byte is fetched.
   `MetalinkFile::agreement(&Layout)` attributes the entry to a file in the
   torrent and compares the two declared lengths. Lengths that differ mean the
   two documents describe different files, and the caller learns it before
   spending the bytes rather than after.

   The **digest** check runs on a payload the session has already verified
   piece by piece against the torrent's own SHA-1 hashes. That ordering is the
   whole argument: a digest that then disagrees is evidence about the Metalink,
   not about the bytes, and the warning says so in those words.
   `scripts/check-metalink.ps1` proves it rather than asserting it, by hashing
   the payload on disk against the source bytes in the mismatch case.

   Both exit **7**, `HashMismatch`, and the report keeps them apart:
   `agreement.size_agrees` and `checksum.matched`. One exit code because both
   are the same finding, that the payload does not match what the Metalink
   claims about it.
5. **A real `.meta4`.** `scripts/check-metalink-real.ps1`, and it found the one
   thing worth knowing about this format in practice.

**No MirrorBrain instance reachable in August 2026 emits `<metaurl
mediatype="torrent">`.** `download.documentfoundation.org` generates a document
per file on demand, and the one for
`LibreOffice_25.8.7_Win_x86-64_helppack_ast.msi` carries 58 real HTTPS mirrors
with dense `priority` 1 to 58 and `location` codes, three whole-file checksums,
a `<pieces>` block, and an OpenPGP `<signature>`, and **no torrent at all**.
The same is true of `download.opensuse.org` and of every LibreOffice file
checked. MirrorBrain emits a `<metaurl>` only when its operator has configured
torrents, and none of them has. So the shape a user actually meets is a
Metalink with nothing for `bit-cli download` to start from, and the message is
built for it:

```
$ bit-cli download real.meta4
the metalink lists no torrent for LibreOffice_25.8.7_Win_x86-64_helppack_ast.msi,
so there is nothing to download here. It lists 58 HTTP mirror(s); pass one with
--web-seed against a .torrent you already have.
```

`real_with_torrent` closes the loop without faking anything that could be real.
It adds one `<metaurl>` line to the document the mirror generated and changes
nothing else: the payload comes down over the public internet from the 58
mirrors the mirror chose, and the digest it is verified against is the sha-256
The Document Foundation published. Measured on the run recorded in
`bench/metalink-real-20260821T045805559Z.json`: 3,801,088 bytes served, 58
sources registered with `origin=metalink`, **1 of the 58 mirrors actually
served bytes** on that run and 3 on the run before it, and the published
sha-256 matched both times. How many mirrors take part is the swarm's
decision and not a number this controls.

Two other real-document findings, both now tests.

- **Version 4 writes its per-piece hashes as bare `<hash>` children of
  `<pieces>` with no attributes at all.** The parser's rule was version 3's,
  which marks each child `piece="N"`, and it never saw these. They were dropped
  anyway, by the guard that refuses a hash with an empty `type`, which is the
  right answer for the wrong reason: one document written with a `type` on the
  child would have put two piece hashes in `checksums` and let
  `best_checksum()` return twenty bytes of one piece. The parser now tracks the
  depth `<pieces>` opened at and ignores every `<hash>` inside it.
- The OpenPGP `<signature>` block is text under an element the parser does not
  know, and it must not become the value of the element before it.

**What is not covered**, recorded rather than deferred:

- A Metalink named by URL is still classified as `Kind::Url` and handed to the
  session as a torrent, which fails on the bencode parse. Real documents are
  served over HTTP, so this is the common way to meet one. T-154 has it.
- A multi-file Metalink is refused with the list, by `single_file()`. Several
  files is several downloads, and taking the first would report success for one
  of them.
- `--hash-check-only` returns before the checksum check, so a metalink run with
  it reports no `metalink` block at all. The block is about what was
  downloaded, and that flag downloads nothing, but the document's own claims
  could still be reported. T-155 has it.
- Language, OS, and country filtering, and `<signature>` verification. Out of
  scope by the entry.

### T-114 -i/--input-file batch input is not implemented

Source:      the operator's brief
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

Source:      the operator's brief
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

Source:      the operator's brief
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

Source:      the operator's brief
Category:    cli
Priority:    P1
Effort:      M
Status:      **done**

Problem:     `--schema-version` prints `1`. There is no `docs/schema.md`, so
             the number refers to nothing a caller can check against.
Relevance:   A versioned contract nobody has written down is not a contract.
Approach:    Document every JSON document and every event type with a worked
             example, generated from the real types rather than written by hand
             so it cannot drift. A test that serialises one of each and checks
             the example still matches is the mechanism.
Acceptance:  `docs/schema.md` exists, covers every `kind` and every event
             `type`, and a test fails when a field is added without updating it.

**Done. Every one of the thirty-one names has a run behind it, and
`schema::NOT_YET_COVERED` is empty.**

The document is generated rather than written. `crates/bit-cli/src/schema.rs`
holds the two tables of names with their descriptions and a flattener that
turns a JSON document into `path -> type` rows, dotting nested objects and
collapsing arrays to `[]`. `crates/bit-cli/src/schema_gen.rs` is a test module
that drives every command in process against fixtures, folds what each run
wrote into a sample per name, renders the whole file, and compares.

```bash
BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema
```

is the only way the file is ever edited.

Seventeen document kinds and fourteen event types, **669 field rows and 992
lines**, up from 444 rows and 751 lines when eight names were still uncovered.
`hash_mismatch` was found while building it: `verify` writes a different `kind`
when a piece does not check out, and nothing had said so.

**The comparison is containment, not equality, and the asymmetry is the
point.** A field added to a report produces a row the committed file does not
have and fails the test. A row the committed file has that a given run did not
produce does not fail, because these runs are timed: a download that finished
before its second report tick emits no `progress`, and one that raced its own
deadline emits no `torrent_completed`. Requiring equality made the check flaky
on the first `--workspace` run, and a flaky contract check is worse than none.
Section headings are still compared exactly, because those do not depend on
timing.

Two more tests hold the ends together.
`every_produced_kind_and_event_is_documented` fails when the program writes a
`kind` the tables do not name, which is what caught `hash_mismatch`, and it
names the command that produced it, because an undocumented `kind` is usually
an error document from a run that was meant to succeed.
`coverage_of_the_documented_names_matches_what_is_recorded` compares the set of
names no run produces against `schema::NOT_YET_COVERED`, which is now empty, so
a name that stops being produced fails the build rather than quietly losing its
field table.

**The eight fixtures, and what each one needed.** None of them touches the
network.

| name | what it needed |
| --- | --- |
| `webseed_test`, `webseed_probe`, `webseed_fetch` | the `FileServer` that was already there, plus `--no-torrent-web-seed` |
| `source_failed` | a source that answers, and fails, inside the run |
| `source_cooling` | the same source with `--web-seed-retry-status 404` and a cooldown |
| `bench_sample` | a `bench disk` run long enough to tick |
| `peers` | a seeder on a thread and `--peer` pointed at it |
| `trackers` | a loopback tracker, and a second tracker that is dead |

Four of them found something.

- **`--no-torrent-web-seed`, or the generator reaches the internet.** The
  fixture torrent carries `https://mirror.example.com/pub/` in its url-list, so
  that was source zero: `webseed fetch --piece 0` fetched from it and failed,
  and `test` and `probe` waited out a connect timeout against a name no test
  should be resolving.
- **A source has to answer to fail.** Both failing runs first pointed at
  `http://127.0.0.1:9/`, which on this machine is blackholed rather than
  refused. The bridge makes a request only when the session asks it for a
  block, so the request sat in a connect that never completed: no error, no
  budget spent, no event, for the 30 seconds until the request timeout. That is
  [T-141](webseed.md), written up with its measurements. Pointed at a path the
  live server does not have, the same run fails in the first second.
- **A fatal status never cools down.** 404 is fatal by default, and a fatal
  status retires a source without spending the error budget a cooldown waits
  out, so `source_cooling` needs `--web-seed-retry-status 404` as well as
  `--web-seed-cooldown`. The two runs are otherwise identical, which is what
  makes the pair worth having: they are the two ends of the same state machine.
- **`bench_sample` needs a run longer than its own sample interval.** At
  4 MiB the disk bench finished in 5 ms and emitted no sample at all. 64 MiB at
  a 10 ms interval emits two. It is the same lesson the soak in
  [T-040](memory.md) turns on, at a different scale.

**`peers` produced nothing at all, and that was the command rather than the
fixture.** It added its torrent paused, and a paused torrent in `librqbit`
9.0.0 never gets its peer stream, so it never announced and never dialled.
Every `bit-cli peers` run ever made reported an empty swarm. That is
[T-142](peers.md), fixed and tested.

The `bench` report itself is deliberately not in these tables. It is a
versioned document of its own, with `report_version` and its own `kind`, and
under `--jsonl` it renders as NDJSON records carrying `record` rather than
`type`, so the generator sees only its events.

```
$ cargo test -p bit-cli --lib schema
test result: ok. 7 passed; 0 failed
```

`--schema-version` still prints `1` and now refers to something whole. Bumping
it is a separate decision and belongs with the first field that is removed or
changes meaning, which has not happened.

### T-118 The short-flag table is not checked in CI

Source:      the operator's brief; premise disproved 2026-08-21, see the correction below
Category:    cli
Priority:    P3
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

**"Neither the file nor the check exists" is false, and both have existed for
some time.** `docs/flags.md` is 79 lines with the table, the two rules, and the
`-v` / `-V` reasoning. Four tests read the `clap` command tree and fail on
drift, and they run in `cargo test`, which is to say in CI on all three
platforms:

| Test | Where | What fails it |
| --- | --- | --- |
| `every_short_flag_is_documented_in_the_flags_table` | `cli.rs:2107` | a short flag with no row in `docs/flags.md` |
| `no_short_flag_is_defined_twice` | `cli.rs:2012` | one letter used twice in one command |
| `short_flags_never_contradict_aria2` | `cli.rs:2048` | an `aria2` letter reassigned to a different concept |
| `short_flags_keep_their_aria2_meanings` | `cli.rs:1833` | `-V` no longer meaning `--check-integrity` |

```
$ cargo test -p bit-cli --lib short_flag
test result: ok. 4 passed; 0 failed; 0 ignored; 303 filtered out
```

The third of those is the one A3.2 actually asked for: it holds the reserved
list: `d` dir, `o` out/output, `j` max-concurrent-downloads, `u`
max-upload-rate, `q` quiet, `c` continue, `V` check-integrity, `O` index-out,
`l` log-file. It requires any flag carrying one of those letters to name the
matching id or not exist.

**One clause of the Acceptance is genuinely unmet, and it is the reason this
stays open rather than closing.** The Acceptance says a test "regenerates it
and fails on drift". The test *asserts* and does not regenerate: it fails with
the exact row to add, which a reader then pastes in. That is a deliberate
difference and probably the better one, see [T-158](#t-158-regenerating-the-schema-deletes-fields-the-sample-did-not-produce),
where the regenerating half of the schema check deletes rows the sample did not
produce, but the entry asked for regeneration and did not get it, so the
honest state is open with the gap narrowed to one clause. Dropped from P2 to
P3: nothing is unprotected.

`docs/flags.md` named the test as `every_short_flag_is_documented`, which is
not its name. Corrected in the same pass. A doc citing a symbol that does not
exist is the same defect class as an entry describing a state the tree is not
in, which is what this correction is.

### T-144 The MSRV job fails: the tree needs a newer rustc than it claims

Source:      CI run 32386960166, 2026-08-20
Category:    ci
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `ci.yml`'s `MSRV` job pins rustc 1.85.1 and runs
             `cargo check --workspace --locked --all-features`. It fails:

             ```
             serde_with@3.21.0 requires rustc 1.88
             serde_with_macros@3.21.0 requires rustc 1.88
             where <compatible-ver> is the latest version supporting rustc 1.85.1
             ```

             So the minimum supported version the repository advertises is not
             a version the repository builds on, and the job has been red
             since the dependency moved.
Relevance:   An MSRV nobody can build with is worse than none: it fails every
             push, and a red job that is always red stops being read. It also
             misleads anyone packaging this for a distribution with an older
             toolchain.
Approach:    Three ways, and the choice is the operator's rather than the
             build's. Raise the MSRV to 1.88 and say so in `Cargo.toml` and the
             README. Or pin `serde_with` back to the last release that builds
             on 1.85.1, with `cargo update serde_with@3.21.0 --precise <ver>`,
             and add a comment saying why the pin exists. Or drop the MSRV job
             and the claim with it.

             Raising it is the honest default: nothing here needs an old
             toolchain, and pinning a dependency back to keep a number is the
             tail wagging the dog.
Acceptance:  The `MSRV` job passes, and the version it pins is the version
             `Cargo.toml` and the README name.

**Raised to 1.88, which is measured rather than chosen.** 1.88 is the highest
`rust-version` in the resolved dependency graph, and the graph is what says so:

```
$ cargo metadata --format-version 1 --all-features
```

Nine packages ask for it. `serde_with`, `serde_with_macros`, and `hdrhistogram`
are direct dependencies; `time`, `time-core`, `time-macros`, `darling`,
`darling_core`, and `darling_macro` arrive underneath them. Nothing in the
graph asks for more. So 1.88 is not a round number picked to make a job pass:
it is the number the tree already needed while claiming 1.85.

Three files carried the claim and none of them checked the others, which is how
it drifted in the first place. `crates/bit-cli/tests/msrv_is_declared_once.rs`
now ties them together: it reads `rust-version` out of `Cargo.toml` and fails
if `.github/workflows/ci.yml` does not pin exactly that toolchain, or if
`README.md` does not name it, or if the version grows a patch level that
`cargo` would ignore and `dtolnay/rust-toolchain` would not.

```
$ cargo test -p bit-cli --test msrv_is_declared_once
test result: ok. 3 passed; 0 failed
```

**Raising it turned on two clippy lints and both were real.** Clippy suppresses
a lint whose fix needs an API newer than the declared `rust-version`, so the
1.85 claim had been hiding them:

- `manual_is_multiple_of` in `webseed/fetch.rs`, because `u64::is_multiple_of`
  stabilised in 1.87.
- `collapsible_if` in `source.rs`, because let-chains stabilised in 1.88.

Both are fixed rather than allowed. That is the second thing a wrong MSRV
costs: not just a red job, but lint coverage nobody knew was off.

**And the raise had a second cost that had to be paid before the job could go
green.** With `rust-version` at 1.85 the tree also compiled `core::arch`'s
`__cpuid` and `__get_cpuid_max` without an `unsafe` block, because a current
toolchain has made those safe to call. At 1.88 they are still `unsafe fn`, so
`cargo check` under the pinned toolchain failed with two `E0133`s that no
amount of local testing on a current compiler would ever show. Writing the
block and allowing `unused_unsafe` is what compiles under either, and the
allowance carries the note that says when to drop it.

That is the whole argument for having an `MSRV` job at all: the claim is only
worth making if something compiles against it.

**The run is in.** `MSRV` passed in 1m1s on CI run 32440386139, 2026-08-21,
compiling the whole workspace with `--locked --all-features` on rustc 1.88:

https://github.com/Azathothas/bit-cli/actions/runs/32440386139

`Clippy` passed in the same run, which is the other half: the two lints the
raise turned on are fixed rather than allowed.

### T-145 The macOS test job fails to link

Source:      CI run 32386960166, 2026-08-20
Category:    ci
Priority:    P2
Effort:      M
Status:      **done**

Problem:     `Test (macos-latest)` fails during linking, not compilation:

             ```
             error: linking with `cc` failed: exit status: 1
             clang: error: linker command failed with exit code 1
             ```

             It happens for every test binary, `hostile_paths` and
             `bit_cli_core` among them, on `aarch64-apple-darwin`. The linker
             line carries `aws-lc-sys`, `ring`, and `network-interface` build
             outputs, so the first thing to check is which of those three fails
             to produce a library on that target.
Relevance:   macOS is not a release target: decision 9 names
             `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, and
             `x86_64-pc-windows-msvc`. So this is not a shipped platform, and
             the job is testing something nobody gets. It matters because a red
             job trains everyone to ignore the light.
Approach:    Two honest options. Fix the link, which means finding which native
             dependency does not build for `aarch64-apple-darwin` and whether a
             feature choice avoids it, `rust-tls` against `aws-lc-rs` being the
             likeliest lever. Or take macOS out of the test matrix and say in
             the README that it is untested, which is what decision 9 already
             implies.

             Do not leave it red either way.
Acceptance:  Either `Test (macos-latest)` passes, or the matrix does not
             include it and `README.md` says which platforms are tested.

**The entry's premise is wrong and the log says so.** None of the three native
dependencies fails to build. The undefined symbol is ours:

```
Undefined symbols for architecture arm64:
  "_posix_fallocate", referenced from: ...
ld: symbol(s) not found for architecture arm64
```

`bit_cli_core::alloc::fallocate` was written under `#[cfg(unix)]` with an
`extern "C"` declaration of `posix_fallocate`. That compiles on any unix,
because an extern declaration is a promise rather than a lookup, and it links
only where the symbol exists. It does not exist on the Apple platforms, and it
does not exist on OpenBSD either. So the failure had nothing to do with
`aws-lc-sys`, `ring`, or `network-interface`: those three names are on the
linker line because everything is on the linker line. The `ld:` warnings about
`ring` objects built for a newer macOS are warnings, and they are noise here.

The lesson is the cheaper half of the entry: `cfg(unix)` is not a platform, it
is a family, and an FFI symbol needs the platform.

**Fixed by giving each platform the call it actually has.** Linux and the BSDs
keep `posix_fallocate`. The Apple platforms get `fcntl(F_PREALLOCATE)`, which
is the same idea in a different shape: it reserves blocks without moving the
end of the file, it measures from the current end rather than from an absolute
offset, and it takes a contiguous run first and may refuse, so the request is
repeated without that constraint before it counts as a failure. The length is
set afterwards, which is what makes `falloc` mean the same thing on both.
OpenBSD returns a reason and degrades to `prealloc`, exactly as Windows does.

The Apple path cannot be run on this machine, so what was checked here is that
it compiles for the real target with warnings denied:

```
$ rustup target add aarch64-apple-darwin
$ rustc --target aarch64-apple-darwin --edition 2024 --emit=metadata -D warnings <the function>
```

The behaviour is checked by CI. `alloc::tests::falloc_either_works_or_says_why_it_fell_back`
runs on `macos-latest` and asserts that the file ends up 65536 bytes long and
that the outcome is either `falloc` with no note or `prealloc` with a reason,
and `every_strategy_sets_the_length` runs `Falloc` alongside the other three.
So the macOS job stops being a job nobody reads and becomes the evidence for
this entry.

**The link was the first defect and not the only one.** With it fixed, the job
compiled, linked, ran, and failed six tests, all on the same cause and all the
same shape as the first: `sysinfo::platform` was written `#[cfg(unix)]` and
reads `/proc`. macOS has no `/proc`, so every read missed. The report it
produced on an M-series Mac:

```json
"host": {
  "cpu": {"architecture": "aarch64", "logical_cores": 3, "model": "unknown"},
  "memory_total": {"bytes": 0, "human": "0 B"},
  "os": {"name": "Linux", "version": "unknown"},
  "unavailable": ["os.version", "memory_total", "network"]
},
"process": {"cpu_ms": 0, "open_handles": 0, "peak_rss_bytes": 0, ...}
```

`os.name` says `Linux` on a Mac. The module has an `unavailable` list for
exactly this and it was populated correctly, and the field beside it was still
a lie, because the fallback was a hardcoded `"Linux"` rather than a read that
failed. A benchmark carries its environment so two numbers can be compared;
this one would have said two Macs and a Linux box were the same machine.

There is now a third implementation, from libSystem, with no new dependency:
`getrusage` for processor time and the resident high-water mark, which on
Darwin is in bytes where Linux reports the same field in kilobytes;
`proc_pidinfo` for resident size now and for the open descriptor count; and
`sysctlbyname` for the kernel name and version, the product version, the CPU
brand string, and physical memory. Link speeds are not read and say so:
`getifaddrs` plus an ioctl per interface is more than anything here compares
across machines today.

The struct layouts are transcribed from the system headers, and a
transcription that is one field out does not fail, it reads the wrong offset
and returns a plausible wrong number. `const _: () = assert!(size_of::<..>())`
on all three fails the build instead.

Checked here the same way the link fix was, since this machine is not a Mac:

```
$ rustc --target aarch64-apple-darwin --edition 2024 --emit=metadata -D warnings <the module>
```

**The run is in.** `Test (macos-latest)` passed in 2m10s on CI run
32444424026, 2026-08-21:

https://github.com/Azathothas/bit-cli/actions/runs/32444424026

Every job in that run is green, which is the first time the whole matrix has
been. Getting there took four rounds, and each one uncovered the next: the link
failure hid six `sysinfo` failures, which hid [T-152](bench.md), which hid one
last per-platform assertion. A red job does not cost one defect, it costs every
defect behind it.

The last of those is worth naming because it is not a defect.
`sysinfo::tests::the_host_names_its_cpu_os_and_memory` asserted
`host.unavailable.is_empty()`, and the Apple reader reports `network` as
unavailable on purpose: link speeds need `getifaddrs` plus an `SIOCGIFMEDIA`
ioctl per interface, which nothing measured here compares across machines yet,
and saying so beats reporting an empty list as though the machine had no
interfaces. The test now compares the set to `["network"]` on Apple and to `[]`
elsewhere, so a second field going unreadable still fails the build, and so
does this one being fixed without the expectation being updated. That gap is
[T-153](#t-153-link-speeds-are-not-read-on-macos).


### T-146 CI built a Windows binary against the dynamic C runtime

Source:      CI run 32405312793, 2026-08-20
Category:    ci
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `Build (x86_64-pc-windows-msvc)` failed its own static CRT check:

             ```
             check-static: the binary depends on the dynamic C runtime:
             VCRUNTIME140.dll, api-ms-win-crt-math-l1-1-0.dll, ...
             ```

             `.cargo/config.toml` sets `-C target-feature=+crt-static` for all
             three release targets, and it works locally. `ci.yml` sets
             `RUSTFLAGS: -D warnings` at the workflow level, and the
             `RUSTFLAGS` environment variable **replaces** the per-target
             `rustflags` from `config.toml` rather than adding to them. So
             every CI job built without `+crt-static`, and the one job that
             checks caught it.
Relevance:   A Windows binary that needs a Visual C++ redistributable fails to
             start on a clean machine with a dialog box rather than an error a
             script can read. `scripts/check-static.ps1` exists for exactly
             this and did its job.
Approach:    Repeat the flag where the variable is set. The build step now
             carries `RUSTFLAGS: -D warnings -C target-feature=+crt-static`
             with a comment saying why it cannot be inherited.
Acceptance:  `Build (x86_64-pc-windows-msvc)` passes, and the run is named
             here.

**The run is in.** `Build (x86_64-pc-windows-msvc)` passed in 8m44s on CI run
32407214253, 2026-08-20, which is the first run carrying the repeated flag:

https://github.com/Azathothas/bit-cli/actions/runs/32407214253

The job runs `scripts/check-static.ps1` against the binary it just built, so
the pass is the check rather than the absence of a failure.

**`release.yml` was never affected, which is the part worth knowing.** It sets
no `RUSTFLAGS` at all, so `.cargo/config.toml` applies there and every
published artifact has been statically linked. The defect was in the
verification path rather than in the release path, and the verification path
is where it was caught.

### T-150 Clippy pins a floating toolchain, so a Rust release can turn the tree red

Source:      CI run 32437262089, 2026-08-21
Category:    ci
Priority:    P2
Effort:      S
Status:      open

Problem:     The `Clippy` job pins `toolchain: stable`, which is whatever
             Rust released most recently. Three lints fired there that do not
             fire on the toolchain in front of me:

             ```
             error: using `chunks_exact` with a constant chunk size
               --> crates/bit-cli-core/src/engine.rs:631:25
               --> crates/bit-cli-core/src/torrent/metainfo.rs:459:10
               --> crates/bit-cli-core/src/tracker.rs:655:10
             ```

             `cargo clippy --workspace --all-targets --all-features --
             -D warnings` is clean on rustc 1.97.1 here, on a cold lint of the
             same crate. So a commit that was green when it was written goes
             red six weeks later with nobody having touched it, and the person
             who finds it is whoever pushed next.
Relevance:   `-D warnings` plus a floating toolchain means the build gate moves
             on its own. This is not hypothetical: it happened in the run
             above, and the three findings were mixed in with four real
             failures, which is exactly the noise that makes a red light stop
             being read. The lints themselves were worth fixing, which is the
             argument for keeping a floating job somewhere rather than for
             having the gate float.
Approach:    Two jobs rather than one, which is the shape that keeps both
             properties. A pinned `Clippy` at a named version is the gate and
             blocks the merge. A second job on `stable`, allowed to fail,
             reports what the next toolchain will want. Bumping the pin is then
             a commit with a message, the same as the MSRV in
             [T-144](#t-144-the-msrv-job-fails-the-tree-needs-a-newer-rustc-than-it-claims).

             The same question applies to `Format`, `Test`, and `Build`, which
             all pin `stable` too. `rustfmt` output is stable across releases
             in practice and the test jobs want the newest compiler, so the
             case is weakest there and strongest for the job that runs lints
             with `-D warnings`.
Acceptance:  `ci.yml` names a version for the gating lint job, a second job
             tracks `stable` without blocking, and this entry records a run
             where the tracking job is red and the gate is green.

**Not done, and open on purpose.** The three lints are fixed, so the tree is
green on both toolchains today and there is nothing to demonstrate the split
against. Doing it now would mean adding a job whose whole point cannot be shown
until the next Rust release. The three fixes are recorded under
[T-144](#t-144-the-msrv-job-fails-the-tree-needs-a-newer-rustc-than-it-claims),
because raising the MSRV is what unlocked two of the four lints this round;
this one is the third and came from the toolchain rather than the manifest,
which is precisely the distinction the entry is about.

### T-151 Only one of the three release targets was checked for static linking

Source:      found here, 2026-08-21, while acting on an operator request
Category:    ci
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `scripts/check-static.ps1` reads a PE import table and refuses a
             binary that needs `VCRUNTIME140.dll`. Both `ci.yml` and
             `release.yml` ran it `if: runner.os == 'Windows'`. The two musl
             targets make the same promise and nothing checked it, so
             `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` could
             have been shipping a binary that needs a loader and nobody would
             have found out until it failed to start.
Relevance:   [T-146](#t-146-ci-built-a-windows-binary-against-the-dynamic-c-runtime)
             is the proof that this is not theoretical: CI did build against
             the dynamic CRT, for weeks, and the reason it was caught at all is
             that the one target with a check had one. Two thirds of the
             release matrix had no such luck.
Approach:    One script, two formats, chosen by the file's own magic bytes
             rather than by the host, so a cross-built artifact is checked the
             same way wherever the checking happens. For ELF that is: no
             `PT_INTERP` program header and no `DT_NEEDED` entry in
             `.dynamic`. Read from the file directly rather than through `ldd`,
             which on a static binary prints "not a dynamic executable" on
             glibc and runs the binary on some other libcs, and neither is a
             thing to build a gate on.
Acceptance:  The check runs on all three targets in `ci.yml` and in
             `release.yml`, and it fails a dynamically linked ELF.

**The run is in.** CI run 32440386139, 2026-08-21, with the new flags and the
new check on all three:

| job | result |
| --- | --- |
| `Build (x86_64-unknown-linux-musl)` | pass, 4m19s |
| `Build (aarch64-unknown-linux-musl)` | pass, 3m53s |
| `Build (x86_64-pc-windows-msvc)` | pass, 8m15s |

https://github.com/Azathothas/bit-cli/actions/runs/32440386139

Each one built with `+crt-static -C prefer-dynamic=no`, the musl pair also with
`-C link-self-contained=yes -C link-arg=-Wl,--build-id=none`, and each one then
had its own binary read back. So the two musl artifacts are now known to carry
no `PT_INTERP` and no `DT_NEEDED` rather than assumed to.

**Both directions were proven before it shipped as a gate**, because a check
that cannot fail is not a check and there is no Linux on this machine to try it
against. Two synthetic ELF64 files were built, one with a `PT_INTERP` naming
`/lib/ld-musl-x86_64.so.1` and one `DT_NEEDED` entry, and one with neither:

```
$ pwsh -NoProfile -File scripts/check-static.ps1 -Path static.elf
interp:  none
needed:  0 shared object(s)
static confirmed: no PT_INTERP and no DT_NEEDED          # exit 0

$ pwsh -NoProfile -File scripts/check-static.ps1 -Path dynamic.elf
interp:  /lib/ld-musl-x86_64.so.1
needed:  1 shared object(s)
check-static: the binary is not statically linked: it names the dynamic
loader /lib/ld-musl-x86_64.so.1, it needs 1 shared object(s)   # exit 1
```

The PE path is unchanged and still passes against this machine's own release
build.

### T-153 Link speeds are not read on macOS

Source:      found here, 2026-08-21, while closing [T-145](#t-145-the-macos-test-job-fails-to-link)
Category:    ci
Priority:    P3
Effort:      M
Status:      open

Problem:     `sysinfo::platform::host` on the Apple platforms reports the OS,
             the CPU, the core count, and physical memory, and reports
             `network` as unavailable rather than reading it. Windows uses
             `GetIfTable` and Linux reads `/sys/class/net`; macOS has neither.
Relevance:   `Host::link_speed_bps` is what says whether a throughput number
             was bounded by the wire, and a report from a Mac cannot answer
             that. It is P3 rather than higher because macOS is not a release
             target under decision 9 and nothing measured so far compares
             across machines on link speed, so the field is unused where it is
             missing.
Approach:    `getifaddrs(3)` gives the interface names and the `IFF_UP` flag.
             The speed needs an `SIOCGIFMEDIA` ioctl per interface against a
             datagram socket, decoding `ifm_active` into a rate. That is real
             FFI against `if_media.h` constants that change between releases,
             which is why it is not in already: it cannot be run here and a
             wrong decode would report a plausible wrong speed rather than
             failing.
Acceptance:  `bit-cli bench webseed --format json` on `macos-latest` carries a
             `host.network` array with at least one interface, and
             `host.unavailable` is empty, which is what
             `sysinfo::tests::the_host_names_its_cpu_os_and_memory` asserts per
             platform today.

The test names the gap rather than tolerating any gap: it compares
`host.unavailable` to `["network"]` on Apple and to `[]` everywhere else, so a
second field going unreadable fails the build, and so does this one being
fixed without the expectation being updated.

### T-154 A Metalink named by URL is not recognised

Source:      `bit-cli` design, found closing [T-113](#t-113-metalink-is-not-implemented)
Category:    cli
Priority:    P2
Effort:      S
Status:      open

Problem:     `Kind::classify` checks the `http://` and `https://` prefixes
             before it checks the `.meta4` and `.metalink` extensions, so
             `bit-cli download https://example.org/release.meta4` is a
             `Kind::Url`, is handed to the session as a `.torrent`, and fails
             on the bencode parse with a message about the torrent rather than
             about the metalink.
Relevance:   Every real Metalink is served over HTTP. `MirrorBrain` generates
             one on demand for any file it publishes, so a URL is the way a
             caller normally meets one, and a local `.meta4` is what you get
             after saving it by hand.
Approach:    A fourth branch: a URL whose path ends in `.meta4` or `.metalink`
             is a remote Metalink. `source::resolve_metalink` already takes a
             parsed `Metalink`, so the only new code is fetching the document
             before parsing it, which is `fetch_bytes` plus `Metalink::parse`.
             The redirect case needs a decision the local path does not have:
             a `MirrorBrain` document is generated per request and its
             `<origin dynamic="true">` names the URL it came from, so nothing
             has to be resolved relative to it, but a document with relative
             mirror URLs would.
Acceptance:  `bit-cli download <URL ending .meta4>` behaves exactly as the same
             document saved to disk does, proven by running
             `scripts/check-metalink-real.ps1` against the URL rather than the
             saved copy and getting the same report.

### T-155 --hash-check-only drops the metalink report

Source:      `bit-cli` design, found closing [T-113](#t-113-metalink-is-not-implemented)
Category:    cli
Priority:    P3
Effort:      S
Status:      open

Problem:     `one_inner` returns early for `--hash-check-only`, before the
             block that builds `TorrentReport::metalink`. So a Metalink run
             with that flag reports nothing about the document at all: not the
             mirror count, not the torrent it resolved, not the size
             comparison, none of which needs a download.
Relevance:   `--hash-check-only` over a Metalink is a reasonable thing to ask
             for: check what is already on disk, and tell me whether the two
             documents agree about it. The size comparison in particular is
             computed before the early return and then thrown away.
Approach:    Build the report at both exits rather than at one. `check_metalink`
             already writes a `not_checked` reason for a run that did not
             finish, so the early path needs the same call and no new branch.
             The interesting case is a payload that is complete on disk: the
             hash check proves it against the torrent, so the Metalink's
             checksum could be checked there too and would be the strongest
             thing this flag could report.
Acceptance:  `bit-cli download release.meta4 --hash-check-only --json` over a
             complete payload reports the `metalink` block with
             `agreement.size_agrees` set, and either a checked digest or a
             `not_checked` reason.

### T-156 A dry run writes a different shape under the same document kind

Source:      `bit-cli` design, found closing [T-113](#t-113-metalink-is-not-implemented)
Category:    cli
Priority:    P3
Effort:      S
Status:      open

Problem:     `download --dry-run --json` writes `kind: "download"` and a
             document that shares almost no fields with a real run's:
             `dry_run`, `directory`, and per-torrent `kind`, `needs_network`,
             `coverage`, `trackers[]`, `web_seeds[]`, and `total_bytes`, and
             none of `stopped`, `finished`, `sources[]`, or `total`. A consumer
             selecting by `kind`, which is the documented way to select, gets
             two shapes.
Relevance:   `docs/schema.md` is generated by folding every run of a command
             into one table per `kind`. Sampling the dry run would make the
             `download` table a union of two documents with nothing saying
             which fields belong to which, so the generator does not sample it
             and the dry run's fields are undocumented.
Approach:    Give it its own kind, `download_dry_run`, and sample it. That is a
             breaking change to a document nothing is known to consume, and it
             is the shape the rest of the surface already uses: `verify` writes
             `hash_mismatch` rather than a `verify` with different fields.
             `dry_run: true` stays, because a reader who has the document in
             hand should not have to know the kind changed.
Acceptance:  `bit-cli download <SOURCE> --dry-run --json | jq -r .kind` prints
             `download_dry_run`, `DOCUMENT_KINDS` names it, and
             `docs/schema.md` carries its field table from a run the generator
             drives.

### T-158 Regenerating the schema deletes fields the sample did not produce

Source:      `docs/schema.md`, found during the doc pass of 2026-08-21
Category:    cli
Priority:    P2
Effort:      S
Status:      open

Problem:     `BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema`
             overwrites `docs/schema.md` with exactly what that run produced.
             A field that only appears on a path the sample did not take is
             silently deleted from the document.
Relevance:   That command is the documented way to update the schema, and it is
             in `CHANGELOG.md` and in the panic message the check itself
             prints. Following the instruction makes the document worse.
Approach:    Merge rather than replace. Read the committed file, union its rows
             with the rendered ones, and write the union sorted. A row that is
             genuinely gone then needs deleting on purpose, which is the right
             cost for removing a documented field.
Acceptance:  Regenerating twice in a row on a machine whose sample takes a
             different path both times leaves every row that either run
             produced, and `git diff docs/schema.md` is empty when nothing
             changed.

**Found by following the instruction.** Regenerating on 2026-08-21 removed one
row and added none:

```
-| `sources[].error` | string |
```

**Re-measured on 2026-08-21 in the doc pass, and it removes two rows now, not
one.** Regenerated into a scratch copy and diffed rather than committed, which
is the workaround this entry exists to remove:

```
$ cp docs/schema.md /tmp/committed.md
$ BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema
$ diff /tmp/committed.md docs/schema.md
338d337
< | `torrents[].sources[].error` | string |
535d533
< | `sources[].error` | string |
$ git checkout -- docs/schema.md
```

Both are the same field seen from the two document shapes, and both are real:
a source that errored carries an `error` string. The sample simply had no
erroring source on this machine on this run. Note what that means for the
count: the number of rows lost is a property of the run rather than of the
tree, so it grows and shrinks and "one row" was never the number. The
mechanism is the defect, not the size.

The read-only half of the check is fine and stays fine.
`schema_gen.rs:734` `the_committed_schema_matches_what_the_program_writes`
passes, and it is deliberately a **containment** check rather than an equality
one, for the reason its own comment gives: these runs are timed, so a download
that finished before its second report tick emits no `progress`, and requiring
equality would make the contract check flaky. The regenerating branch at `:739`
is a plain `std::fs::write` of the rendered text, with none of that tolerance.
So the check is asymmetric on purpose and the regenerator is symmetric by
accident, and the fix is to give `:739` the same tolerance the assertion
already has.

That field is real. `crates/bit-cli/src/cmd/webseed.rs:285` and
`crates/bit-cli-core/src/webseed/probe.rs:457` both carry
`error: Option<String>` with `skip_serializing_if`, so it appears when a source
fails and not when every source succeeds. The generator's sample had no failing
source, so the row went. Three regenerations in a row produced the same
deletion, so it is deterministic rather than a flake.

The regeneration was **not committed**, and the committed schema is the
accurate one.

**Why the check did not catch it.** `the_committed_schema_matches_what_the
_program_writes` is a containment check on purpose: a row this run produced and
the file lacks is a failure, and a row the file has and this run did not
produce is not. That asymmetry is right, and its comment explains why: these
runs are timed, so a download that beats its own report tick emits no
`progress`. The gap is that the **writer** does not share the reader's
asymmetry. The check tolerates extra rows and the generator deletes them.

### T-159 Subcommand flags are filed under "Report options" in the help

Source:      `bit-cli bench <SUB> --help`, found in the doc pass of 2026-08-21
Category:    cli
Priority:    P3
Effort:      S
Status:      open

Problem:     `--peers`, `--torrents`, `--dir`, and `--connect-timeout` appear
             under the heading **Report options** in `bench swarm --help`.
             None of them is a report option. `bench leech`, `bench seed`, and
             `bench disk` have the same defect, so four of the six subcommands
             mis-file their own flags.
Relevance:   The headings exist so a reader can find a flag by what it does.
             One that files `--peers` beside `--fail-under` is worse than none,
             because it is confidently wrong.
Approach:    `clap`'s `next_help_heading` applies to every argument declared
             after it, including the ones in the outer struct that follow a
             flattened inner one. `BenchShared` sets the benchmark heading and
             flattens `ReportArgs`, which sets the report heading, and the
             outer struct's own fields are declared after that flatten, so they
             inherit it. Give each subcommand struct its own
             `#[command(next_help_heading = "...")]`, or flatten the shared
             groups last.
Acceptance:  For every `bench` subcommand, the only flags under **Report
             options** are `--report`, `--format`, `--baseline`, and
             `--fail-under`. A test walks `clap`'s command tree and asserts it,
             so the next subcommand cannot reintroduce it.

Reproduce, and see it on four of six:

```bash
for s in webseed leech seed disk probe swarm; do
  echo "== $s"
  bit-cli bench $s --help | sed -n '/^Report options:/,/^[A-Za-z].*options:$/p'
done
```

`webseed` and `probe` are correct, and they are correct by accident: neither
declares a flag after its flatten.

### T-160 A peers test raced its own seeder

Source:      local `cargo test --workspace` and CI run 32458314378, 2026-08-21
Category:    ci
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `cmd::peers::tests::a_sampled_swarm_carries_what_came_from_each_peer`
             starts a seeder on a thread and dials it from the test thread with
             nothing in between. `free_port` binds a port to learn its number
             and drops the listener, so there is a window where the number is
             known and nothing is listening. A dial that lands in that window
             fails, the peer is marked dead with one error, and `librqbit` does
             not retry it for ten seconds, which is twice the test's own
             `--duration 5s`. Every assertion after the dial then fails.
Relevance:   [T-148](bench.md#t-148-the-peer-probe-test-asserted-an-exit-code-inside-its-own-retry-loop)
             is the precedent, and this is the same mistake in another test: a
             fixture whose readiness is assumed rather than waited for. A test
             that fails one run in twenty turns CI red on somebody else's push
             and costs more to diagnose there than here.
Approach:    Wait on the condition, not on a guessed duration.
             `test_support::wait_for_listener` dials the port until something
             accepts or ten seconds pass, and the test asserts it came up
             before it asserts anything about the swarm, so a fixture that
             never started says so instead of failing three assertions later.
Acceptance:  The test is named, the race is named, and the fix is in the test
             rather than in a retry around it, the way T-148 was fixed.

**Found twice and named once.** It failed one local `cargo test --workspace`
and was not reproduced in fourteen further runs, including six sequential and
two concurrent pairs run to provoke it, with the name lost because the command
filtering the output matched only the summary line. Then it failed
`Test (ubuntu-latest)` on CI run 32458314378, which was a **documentation-only
commit**, and the CI log carried what the local filter had thrown away:

```
---- cmd::peers::tests::a_sampled_swarm_carries_what_came_from_each_peer stdout ----
thread '...' panicked at crates/bit-cli/src/cmd/peers.rs:427:9:
assertion `left == right` failed: {... "dead":1, "live":0,
  "peers":[{"errors":1,"downloaded_bytes":0,"verified_pieces":0,"state":"dead"}]}
  left: Number(1)
 right: 0
```

`errors: 1` and `downloaded_bytes: 0` are the whole diagnosis: the peer never
connected, so nothing followed. A commit that changed only Markdown is what
proves the test and not the code.

**Fixed, and in two places.** `crates/bit-cli/src/schema_gen.rs` has the same
seeder-on-a-thread-then-dial shape and now waits too. There it is quieter and
worse: nothing asserts, so a lost race would sample a `peers` document with a
dead peer and silently write a schema missing whatever a live peer carries.
That is [T-158](#t-158-regenerating-the-schema-deletes-fields-the-sample-did-not-produce)
arriving by a second route.

Two things worth keeping from how this was found. Filter for
`^test \S+ \.\.\. FAILED` and not for the summary line, or the name is lost.
And a green run does not mean a suite has no race: this one passed twenty
consecutive local runs and sixteen CI jobs before failing on a commit that
touched no code.

### T-161 A CI action still targets Node.js 20, which is deprecated

Source:      CI run 32457763652 annotations, 2026-08-21
Category:    ci
Priority:    P3
Effort:      S
Status:      open

Problem:     Three jobs annotate:

             ```
             Node.js 20 is deprecated. The following actions target Node.js 20
             but are being forced to run on Node.js 24:
             ilammy/setup-nasm@v1.5.2
             ```

             The run is green. Being forced onto a runtime it was not built
             for is a warning today and a failure whenever the forcing stops.
Relevance:   Same shape as [T-150](#t-150-clippy-pins-a-floating-toolchain-so-a-rust-release-can-turn-the-tree-red):
             a gate that moves without this repository touching it. The
             difference is that this one announces itself first, so it is worth
             acting on before it announces itself as a red job.
Approach:    `ilammy/setup-nasm` is used at **four** call sites in **four**
             jobs, not the two an earlier revision of this entry named:
             `test` at `.github/workflows/ci.yml:62`, `build` at `:97`,
             `interop` at `:199`, and `determinism` at `:238`. On the matrix
             those are `Test (windows-latest)`,
             `Build (x86_64-pc-windows-msvc)`,
             `Create round trip (windows-latest)` and
             `Create determinism (windows-latest)`. Patching two of the four
             leaves the annotation on the other two and leaves the tree half
             fixed, which is the reason the count is written out here. Take a release that
             declares `node24`, or replace it: NASM is needed only by `aws-lc-
             rs`, and `choco install nasm` on the runner is one line with no
             action behind it. Every other action in the file is already on a
             current major.
Acceptance:  A CI run with no Node.js deprecation annotation, and the Windows
             jobs still green, which is what says NASM is still being found.

Recorded rather than acted on, because the run this came from is green on all
sixteen jobs and changing a build dependency of the one target that has to link
statically is not a change to make in the same push as everything else.

### T-181 Four flags are accepted in silence and reach no code

Source:      the flag audit of 2026-08-21
Category:    cli
Priority:    P1
Effort:      M
Status:      **done**

Problem:     Four flags parse, are carried into a struct, and are never read
             again anywhere in the workspace:

             | Flag | Declared | Read |
             | --- | --- | --- |
             | `--no-pex` | `cli.rs:1335` | nowhere |
             | `--tracker-list-url <URL>` | `cli.rs:700` | nowhere |
             | `--max-overall-download-rate <RATE>` | `cli.rs:741` | nowhere |
             | `--max-overall-upload-rate <RATE>` | `cli.rs:745` | nowhere |

             The check is one command: every `pub` field in `cli.rs` grepped
             for outside that file. Six fields have no reader. Two of the six
             are already owned, `index_out` by
             [T-116](#t-116--o--index-out-cannot-rename-a-file) and
             `on_piece_verified` by
             [T-115](#t-115-hooks-do-not-fire-for-every-documented-trigger),
             and these four are owned by nothing.
Relevance:   This is the P1 definition in `INDEX.md` verbatim: "a documented
             capability does not work, or a flag does nothing." It is also the
             rule `cli-surface.md` opens with: a flag that looks like it works
             and does not is worse than one that errors.

             Each of the four fails a different way and none of them fails
             loudly.

             `--no-pex` is the one with a security shape. A user passing it
             believes peer exchange is off. It is not off, and their address
             keeps being gossiped to the swarm. That is a privacy expectation
             silently unmet rather than a performance knob silently ignored.

             `--tracker-list-url` promises a tracker list fetched over HTTP.
             Nothing is fetched, so the run announces to fewer trackers than
             the user asked for and finds fewer peers, which reads as a quiet
             swarm rather than as a missing feature.

             `--max-overall-download-rate` and `--max-overall-upload-rate` are
             the pair that matter most on the operator's own case. They are the
             whole-run caps, and the per-torrent ones next to them
             (`--max-download-rate`, `--max-upload-rate`) **do** work and are
             measured under [T-031](performance.md). So a user who caps a
             single torrent gets a cap and a user who caps the whole run gets
             nothing, from two flags that sit four lines apart in the same
             struct and read identically in `--help`. `performance.md` under
             T-031 already noted these two were not covered by that
             measurement, and no entry picked them up.
Approach:    Two of the four are work and two are a decision.

             **`--max-overall-*-rate`** is the one to build. `librqbit`'s
             `LimitsConfig` is per-session, and `bit-cli` runs one session per
             invocation, so a session-wide cap is where these belong and the
             per-torrent flags are the ones that need dividing. Care is needed
             on one point [T-132](multi-source.md) already records: a session
             cap applies to peers **and** to HTTP sources together, because a
             web seed reaches the session as a peer. So `--max-overall-*` and
             `--web-seed-speed-limit` interact, and the acceptance has to
             measure both together or it proves nothing.

             **`--tracker-list-url`** is a small fetch: GET the URL, one
             tracker per line, blank line separating BEP 12 tiers, which is
             the format `--tracker-file` already parses at `cli.rs:697`. The
             work is reusing that parser and bounding the fetch, because the
             URL is user-supplied and the response is untrusted. Cap the body,
             set a deadline, and refuse a non-HTTP scheme.

             **`--no-pex` cannot be built here.** `librqbit` 9.0.0 has no
             switch for it: `swarm.rs:160-161` shows `--no-dht` and `--no-lsd`
             reaching `enable_dht` and `enable_lsd`, and there is no
             `enable_pex` beside them. `nanotorrent`'s
             `patches/0004-pex-toggle.patch` adds exactly that:
             `SessionOptions::disable_pex`, gating **both** directions, which
             is the shape of the upstream change needed and the evidence that
             it is a small one. Until it exists, the flag must either warn or
             refuse.

             **The pattern for all four in the meantime already exists in this
             tree.** `crates/bit-cli/src/cmd/seed.rs:105`: `--superseed` is
             accepted and prints a warning naming the entry that would close
             it. That is the honest behaviour for a flag that cannot yet do
             what it says, and it is why `--superseed` is not on the list
             above. Do that for all four today, and remove each warning as its
             flag starts working.
Acceptance:  Two parts, and the first is what stops this recurring.

             A test that walks the `clap` command tree and asserts every flag
             either reaches code or is on an explicit, named exception list,
             so a fifth cannot be added silently. The exception list is the
             deliverable: it is short, it is reviewed, and it makes the
             warning above mechanical rather than remembered.
             `cli.rs:2107` `every_short_flag_is_documented_in_the_flags_table`
             is the model: it already walks the tree and fails with the exact
             fix to apply.

             Then, per flag: `--max-overall-download-rate 4MiB/s` over `-j 4`
             holds the aggregate within ten per cent, measured against an
             uncapped run of the same four torrents, with both numbers here;
             `--tracker-list-url` against a loopback URL serving three
             trackers announces to all three and reports them in `--json`;
             `--no-pex` warns, naming this entry, until the upstream switch
             exists.

**All four are resolved, and building two of them found a fifth this entry's
own audit could not have caught.**

| Flag | Now | Where |
| --- | --- | --- |
| `--max-overall-download-rate` | works, session-wide | `swarm.rs` `engine_options` |
| `--max-overall-upload-rate` | works, session-wide | the same |
| `--tracker-list-url` | works, fetched over HTTP | `swarm.rs` `tracker_list` |
| `--no-pex` | warns, naming this entry | `cmd/seed.rs` |

**The rate pair was two flags aiming at one field, and the wrong one arrived.**
`librqbit` 9.0.0 has two rate limits and they are different structures:
`SessionOptions::ratelimits` caps the session and `AddTorrentOptions::ratelimits`
caps one torrent. `bit-cli` set only the session one, and it set it from
`--max-download-rate`. So the per-torrent flag capped the whole run and the
whole-run flag capped nothing. Each flag now goes to the field it names, and
`SessionSetup::torrent_rates` parses the per-torrent pair in one place so a
command cannot wire one and forget the other.

`--max-download-rate` therefore changes behaviour, and the change is the fix.
[T-031](performance.md) measured it at `-j 1`, where per-torrent and whole-run
are the same number, so that measurement stays true. This is the measurement
that tells them apart:

```
$ pwsh -NoProfile -File scripts/check-overall-rate.ps1 -Rate 4MiB/s -PayloadSize 64MiB -Torrents 4

phase       exit wall  bytes     rate
uncapped       0 0.2s  64.00 MiB 392.64 MiB/s
overall        0 15.2s 64.00 MiB 4.20 MiB/s
per_torrent    0 3.3s  64.00 MiB 19.69 MiB/s

verdict: both scopes hold
```

`--max-overall-download-rate 4MiB/s` over `-j 4` holds at **4.20 MiB/s**, 5.05%
over the cap and inside the ten per cent this entry asked for, against **392.64
MiB/s** uncapped, which is 93 times faster. The third phase is the one that
proves the two flags are two fields: `--max-download-rate 4MiB/s` over the same
four torrents reaches **19.69 MiB/s**, near the 16 MiB/s that four torrents at
4 MiB/s each should sum to, and 4.7 times what the whole-run cap allows. Before
this change phases 2 and 3 were the same run.

Evidence: `bench/overall-rate-20260821T140422453Z.json`, and
`scripts/check-overall-rate.ps1` is the script. The sources are HTTP web seeds
rather than peers, which is deliberate: a web seed reaches the session as a
peer, so the session limiter is what bounds it, and that is exactly the
interaction [T-132](multi-source.md) is about. The rate is computed from the
wall clock and the bytes the report says landed, never from the report's own
mean, so the limiter is not measured by the thing it limits.

**`--tracker-list-url` is a bounded fetch, and the bound is the point.** The
URL comes from the caller and the body comes from whoever answers it, so
`crate::source::fetch_list` refuses a scheme that is not HTTP or HTTPS, sets a
thirty second deadline over the whole exchange, and caps the body at one
mebibyte. It reads in chunks rather than calling `bytes()`, so a server
declaring a small `Content-Length` and sending more is stopped at the cap
rather than after it. A body over the cap is **refused rather than truncated**:
half a tracker list is a run announcing to a set of trackers nobody chose, and
a truncated last line is a URL that is not the URL anyone wrote.

It reads with the same parser `--tracker-file` uses, so two flags that read
identically in `--help` behave identically. That parser flattens, and a blank
line does not open a BEP 12 tier here any more than it does in a file;
announcing in tier order is [T-063](trackers.md) and is not this.

The fetcher is injected the way `webseed_args::collect` already takes one, so
the assembly is testable without a network and a command that must not reach
out passes `no_network`. `download --dry-run` is one of those: a dry run
reports without doing, which is the decision `--web-seed-list-url` already
took on that same command.

Proven end to end against three loopback trackers, in
`a_tracker_list_url_is_fetched_and_every_tracker_in_it_is_announced_to`. Three
rather than one, because the failure this guards against is a list read and
then partly dropped, and one tracker cannot tell a whole list from its first
line. Each tracker records what it was asked, so the proof is on the tracker's
side rather than in a count the run reports about itself.

**`--no-pex` cannot be built and now says so.** `librqbit` 9.0.0's
`SessionOptions` carries `dht` and `disable_local_service_discovery` and
nothing beside them for peer exchange, which `swarm.rs` shows: `--no-dht` and
`--no-lsd` reach `enable_dht` and `enable_lsd` and there is no `enable_pex` to
reach. `nanotorrent/patches/0004-pex-toggle.patch` adds exactly that,
`SessionOptions::disable_pex` gating both directions, which is the shape of the
upstream change needed and the evidence that it is a small one.

The warning names what is still happening rather than what is missing, because
this flag's failure is a privacy expectation and not a performance knob:

```
--no-pex is accepted but peer exchange stays on: librqbit 9.0.0 has no switch
for it, so your address is still gossiped to the swarm; see
TODO/cli-surface.md T-181
```

`--no-pex` is declared on `seed` and on no other command, so there is one place
to warn from and it is the one `--superseed` already warns from.

**The test that stops this recurring is
`every_flag_reaches_code_or_is_a_named_exception` in `crates/bit-cli/src/cli.rs`.**
It walks the `clap` tree, reads every `.rs` file in both crates except `cli.rs`
itself, and fails on any flag whose field name appears nowhere. Two names are
on the exception list and each carries the entry that owns it:
`index_out` ([T-116](#t-116--o--index-out-cannot-rename-a-file)) and
`on_piece_verified` ([T-115](#t-115-hooks-do-not-fire-for-every-documented-trigger)).
The list is checked in both directions, so a name that something now reads
fails as stale rather than sitting there.

It reads the tree rather than `include_str!`ing a fixed list, because a file
added later would otherwise silently stop being searched, which is the same
class of gap this test exists for. It asserts it read more than twenty files
and found more than a hundred flags, so a test that is looking at nothing fails
instead of passing.

Proven by unwiring one flag:

```
$ cargo test -p bit-cli --lib -- every_flag_reaches_code   # engine_options download_rate set to None
these flags parse and nothing outside cli.rs reads them...
  max_overall_download_rate  (bit-cli download)
test result: FAILED. 0 passed; 1 failed
```

**What the check is deliberately weak about, and why.** It cannot tell a flag
that works from one that only warns, because `--superseed` and `--no-pex` both
read their field and both do nothing but print. Warning is the honest behaviour
for a flag that cannot yet do what it says, so a test that failed on it would
push the wrong way. What it catches is the case that hid for a whole session: a
field nothing reads at all.

**And it is weak in a second way, which a fifth flag found immediately.**
`--web-seed-list-url` passes this test. Its field is read, in
`crates/bit-cli/src/webseed_args.rs`, and what it was read into was a function
that always errors, on every call site including `download`. So the flag
parsed, was read, and could only ever fail. That is
[T-183](#t-183---web-seed-list-url-is-read-only-into-a-refusal), filed and
fixed in the same session, and it is why the count in `CHANGELOG.md` is now
written as "the test is the point" rather than as a number. Two revisions of
that section have been wrong about the number in opposite directions.

```
$ cargo test --workspace
test cli::tests::every_flag_reaches_code_or_is_a_named_exception ... ok
test swarm::tests::a_tracker_list_url_contributes_every_tracker_it_names ... ok
test swarm::tests::a_tracker_list_url_composes_with_the_flags_beside_it ... ok
test swarm::tests::a_tracker_list_url_on_a_no_network_command_fails_clearly ... ok
test swarm::tests::the_overall_rate_caps_the_session_and_the_plain_one_caps_a_torrent ... ok
test swarm::tests::one_rate_scope_never_stands_in_for_the_other ... ok
test cmd::seed::tests::no_pex_warns_that_peer_exchange_stays_on ... ok
test cmd::seed::tests::a_seed_without_no_pex_says_nothing_about_peer_exchange ... ok
test cmd::download::tests::a_tracker_list_url_is_fetched_and_every_tracker_in_it_is_announced_to ... ok
```

### T-182 A macOS test asserted an invariant across two kernel subsystems

Source:      CI run 32478382564, 2026-08-21
Category:    ci
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `Test (macos-latest)` failed on a **documentation-only commit**:

             ```
             test sysinfo::tests::a_process_sample_reports_memory_cpu_and_handles ... FAILED
             thread '...' panicked at crates/bit-cli-core/src/sysinfo.rs:1144:9:
             assertion failed: sample.peak_rss_bytes >= sample.rss_bytes
             ```

             The other fifteen jobs were green, including `Test
             (windows-latest)` and `Test (ubuntu-latest)` running the same
             test.
Relevance:   A peak below the current reading is not a peak, so the assertion
             is asking for something a reader of the report would also assume.
             It failed anyway, and the reason is that on Darwin the two numbers
             do not come from the same place.
Approach:    Read where each number comes from on each platform before
             deciding whether the test or the code is wrong.
Acceptance:  The assertion holds on all three platforms for a reason rather
             than by luck, and the reason is written where the code is.

**The code was wrong, not the test, and the platform is why.**

`Process::sample()` fills `peak_rss_bytes` and `rss_bytes` from one source on
two platforms and from two sources on the third:

| Platform | `peak_rss_bytes` | `rss_bytes` | Same source? |
| --- | --- | --- | --- |
| Windows | `PROCESS_MEMORY_COUNTERS.PeakWorkingSetSize` (`sysinfo.rs:442`) | the same struct | yes |
| Linux | `VmHWM:` in `/proc/self/status` (`sysinfo.rs:663`) | `VmRSS:` in the same read | yes |
| macOS | `getrusage(RUSAGE_SELF).ru_maxrss` (`sysinfo.rs:986`) | `proc_pidinfo`'s Mach `resident_size` (`sysinfo.rs:993`) | **no** |

`ru_maxrss` is the BSD layer's high-water mark. `resident_size` is the current
Mach task footprint and counts pages the BSD accounting does not. They are two
subsystems' numbers, so on Darwin the current reading can exceed the recorded
peak, and no ordering between them is guaranteed. Windows and Linux each read
both fields from one structure, so neither can disagree with itself this way,
which is why fifteen jobs were green.

**Fixed by clamping at the source rather than by weakening the test.**
`peak_rss_bytes = peak_rss_bytes.max(rss_bytes)` in the Darwin path, applied
only when `getrusage` actually succeeded, so a failed read stays in
`unavailable` rather than being backfilled from another subsystem. The clamp is
honest: the process has just been observed at `rss_bytes`, so its peak is at
least that. Weakening the assertion to `#[cfg(not(target_os = "macos"))]` would
have made the test pass and left `bench` and `soak` reports carrying a
`peak_rss_bytes` that means one thing on two platforms and another on the
third, which is the field [T-042](memory.md) built and [T-040](memory.md)'s
slope rests on.

The assertion now also prints both numbers on failure. The original was a bare
`assert!` and the CI log carried no values, so the first question a reader asks
had to be answered by reasoning about the platform rather than by reading the
output.

**The fourth documentation-only commit to turn a job red, and the fourth time
that was the cleanest available proof the test was wrong rather than the
tree.** [T-160](#t-160-a-peers-test-raced-its-own-seeder),
[T-162](webseed.md) and this one all had nothing but Markdown in the diff.
[T-148](bench.md) is the same family found locally. The rule they keep writing
is in [RULES.md](RULES.md): a test never asserts that the machine cannot fail
some other way. This one asserted that two kernel subsystems agree.

`cfg(unix)` being a family and not a platform is the same lesson
[T-145](#t-145-the-macos-test-job-fails-to-link) cost a red job for, where
`posix_fallocate` was declared under `cfg(unix)` and does not exist on Darwin.
That one was a link error and loud. This one was an assertion that held on the
developer's machine and on two of the three runners, which is quieter and took
a push to find.

```
$ cargo test -p bit-cli-core --lib sysinfo
test sysinfo::tests::a_process_sample_reports_memory_cpu_and_handles ... ok
test result: ok. 20 passed; 0 failed
```

### T-183 --web-seed-list-url is read, only into a refusal

Source:      found while building [T-181](#t-181-four-flags-are-accepted-in-silence-and-reach-no-code), 2026-08-21
Category:    cli
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `--web-seed-list-url <URL>` fetches a newline-separated list of
             web seed URLs. Its field was read, at
             `crates/bit-cli/src/webseed_args.rs`, and what it was read into
             was `webseed_args::no_network`, a function whose entire body is an
             error. **Every** call site passed it: `download`, `bench leech`,
             `bench webseed`, `webseed list`, and the dry runs. So the flag
             parsed, was read, and could only ever fail, on every command that
             accepts it, with a message telling the caller that "this command
             does not use the network" on a command whose whole job is the
             network.
Relevance:   This is the P1 definition in `INDEX.md` verbatim, and it is worse
             than the four T-181 found in one specific way: it is invisible to
             the audit that found them. That audit was "every `pub` field in
             `cli.rs` grepped for a reader outside that file", and this field
             has a reader. So does the `clap`-tree test T-181 built to stop a
             fifth appearing, which is why that test's own entry now says what
             it is weak about.

             The flag is also undocumented. It appears in no table in
             `README.md` and in no row of `docs/flags.md`, which is how it
             stayed unnoticed: nothing described it, so nothing contradicted
             what it did.
Approach:    Give the commands that use the network a real fetcher, and leave
             the ones that must not with the refusal. The fetch itself is
             shared with `--tracker-list-url`, because the two flags read the
             same format, take the same risks, and were built in the same
             session.
Acceptance:  A loopback URL serving one mirror produces one source with
             `origin: "list_url"` in `download --json`, and that source serves
             the payload.

**Fixed with the same bounded fetcher T-181 built.**
`crate::source::fetch_list` refuses a scheme that is not HTTP or HTTPS, sets a
thirty second deadline, caps the body at one mebibyte, and reads in chunks so
the cap bounds what is held rather than only what is returned.
`crate::source::list_fetcher` binds it to the runtime the command has already
made, rather than building a second runtime inside the first.

**Which commands fetch, and which still refuse, is now a decision rather than
an accident.**

| Command | Behaviour | Why |
| --- | --- | --- |
| `download` | fetches | it is the command that downloads |
| `bench leech` | fetches | it downloads for real, and measures what it downloads |
| `download --dry-run` | refuses | a dry run reports without doing |
| `bench leech --dry-run` | refuses | the same |
| `webseed list`, `test`, `fetch` | refuses | `cmd/webseed.rs` `resolve` is documented as resolving "without touching the network", which is what makes `webseed list` safe to run against an unknown torrent |
| `bench webseed` | refuses | it measures the sources it is given, and fetching a list would change what is being measured |

The refusal's message was rewritten while it was there. It named
`--web-seed-list-url` specifically, and it now backs `--tracker-list-url` too,
so it names the URL rather than a flag.

**The lesson is about the audit, not the flag.** "A field with no reader" and
"a field whose reader cannot succeed" look identical from the outside and are
found by different methods. The first is one `grep`. The second was found by
reading the call sites of the function the field is read into, while wiring an
unrelated flag through the same file. Nothing systematic found it, and the
`clap`-tree test in [T-181](#t-181-four-flags-are-accepted-in-silence-and-reach-no-code)
does not find it either. What that test does is stop the cheap case, and the
honest thing is to say so in its own docstring, which it does.

```
$ cargo test -p bit-cli --lib -- a_web_seed_list_url_is_fetched
test cmd::download::tests::a_web_seed_list_url_is_fetched_and_its_sources_are_used ... ok
test result: ok. 1 passed; 0 failed
```

The test points `--web-seed-list-url` at a loopback file server serving a
one-line list, and asserts the torrent finishes, that its single source has
`origin: "list_url"`, and that the source served all 2000 bytes. Before the
fix the run exits on a usage error, so the assertion that fails first is the
exit code.

