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
Status:      **done** 2026-08-23T08:00Z

Problem:     `--on-complete` and `--on-error` ran once for the whole `download`
             run. `--on-piece-verified` did not run at all, and neither hook
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

**Done 2026-08-23T08:00Z**, both clauses, and the acceptance is a test rather
than a run recorded here: `on_complete_fires_once_per_torrent_with_its_own_info_hash`
downloads two torrents at `-j 2`, and the hook creates a directory named
`on-complete-<info hash>`. Two directories, two hashes. Reading what the hook
wrote rather than what the report says is the point: the report is the run's
account of itself and the directories are what actually ran.

**The old shape could not express a mixed run at all.** It picked one hook for
the whole run by `report.failed`, so a run where one torrent finished and one
did not fired `--on-error` for both or `--on-complete` for both, with the first
torrent's info hash and the run's totals, which describes neither.
`a_mixed_run_fires_on_complete_and_on_error` holds the fix.

**`--on-piece-verified` fires now, and the entry's "probably a rate limit" is
answered with a measurement rather than a flag.** One piece is one process and a
process is not free: **1,025 invocations took 47.55 seconds on this machine**,
46 ms each. That number is honest about what it measured and the doc says so:
the command was `cmd /C rem` and a hook is already run through `cmd /C`, so each
invocation started two processes, about 23 ms per `cmd`. Either way a 4 GiB
torrent at a 1 MiB piece length is 4,096 pieces. Two bounds rather than a rate
limit, because a rate limit silently loses notifications and a caller cannot
tell which:

- **Its own thread.** The watch loop hands over a map and returns. Without this
  a hook at that cost would cap the download at tens of pieces a second whatever
  the network could do.
- **A bounded queue, 1,024 deep, and what does not fit is counted.**
  `--json` carries `hooks.skipped` and a run with any warns on stderr. Nothing
  is dropped silently and nothing waits.

`docs/hooks.md` is the second clause: every variable, what it holds, what the
piece hook costs, and what an exit code does.
`every_hook_variable_is_documented` fails when a variable has no row there and
`every_variable_a_hook_sets_is_in_the_list` fails when the code and the list
disagree **in either direction**, the same pattern
[T-118](#t-118-the-short-flag-table-is-not-checked-in-ci) settled for
`docs/flags.md`.

**A defect the acceptance found in the hook runner itself, which had been there
since hooks existed.** `swarm::run_hook` built `cmd /C <command>` with
`Command::arg`. Rust quotes an argument for the C runtime's parser, and
`cmd.exe` does not use that parser: it re-reads the command line with rules of
its own. So a hook whose command contained a quoted path, a redirect or an `&&`
reached `cmd` mangled and exited with "The filename, directory name, or volume
label syntax is incorrect". The acceptance's own hook is
`mkdir "<dir>\%BIT_CLI_HOOK%-%BIT_CLI_INFO_HASH%"`, and the first run of it
fired twice, as asked, and **failed twice**. `raw_arg` is the fix, which is what
`sh -c` had always effectively done on the other platform. Nothing but a hook
with a quoted argument would have shown it.

**`seed` still runs no hooks**, which is the Problem's third clause and is
**not** done. It is not in the Acceptance and is carried as its own entry rather
than left implied: [T-214](#t-214-seed-runs-no-hooks). `bit-cli seed` has no
`--on-*` flag at all, so there is no flag that does nothing; what is missing is
the feature.

```
$ cargo test -p bit-cli --lib hooks::
test result: ok. 6 passed; 0 failed; 0 ignored; 400 filtered out

$ cargo test -p bit-cli --lib on_complete_fires
test result: ok. 1 passed; 0 failed; 0 ignored; 407 filtered out

$ cargo test -p bit-cli --lib on_piece_verified_fires
test result: ok. 1 passed; 0 failed; 0 ignored; 408 filtered out
```

`ACCEPTED_WITHOUT_A_READER` in `cli.rs` is **empty** now. It held
`on_piece_verified` and `index_out`, and both closed on 2026-08-23.

### T-116 -O/--index-out cannot rename a file

Source:      the operator's brief
Category:    cli
Priority:    P3
Effort:      S
Status:      **done** 2026-08-23T07:40Z

Problem:     `-O/--index-out INDEX=PATH` parses and does nothing.
Relevance:   It is a reserved `aria2` letter and the natural answer to a
             torrent whose paths collide on Windows, T-072.
Approach:    Needs a storage wrapper mapping a torrent file index to a
             different on-disk path, which is the same machinery T-071 needs
             for sanitisation. Build them together.
Acceptance:  `bit-cli download <TORRENT> -O 0=renamed.bin` writes the first
             file as `renamed.bin` and `--json` reports the mapping.

**Done 2026-08-23T07:40Z, and no storage wrapper was needed.** The Approach
priced this as a wrapper mapping an index to a path, built alongside T-071. It
is one argument to the function T-071 already built: `paths::plan_with` takes
the overrides and applies each one **before** anything else happens, so a
requested path is sanitised, truncated and disambiguated exactly as a torrent
path is. `plan` is `plan_with` with an empty map.

**That ordering is the whole safety argument, and it is what makes this small.**
`-O 0=../../etc/passwd` renames the file to `__/__/etc/passwd` inside the output
directory; `-O 0=CON.txt` gets `CON_.txt`; `-O 1=a.bin` against a torrent whose
file 0 is already `a.bin` gets `a-1.bin`. Not one of those decisions is new, and
`a_requested_path_cannot_escape_or_name_a_device` is the case that holds it.
Nothing about `-O` could have reached outside the output directory without
first defeating T-071, which is why it is one function rather than two.

**`Reason::Requested` is a new reason and it is first in the enum.** It is the
only one that is a request rather than a defect in the torrent, and `--json`
carries `reasons` in enum order, so a reader scanning a rename sees it before
anything that reads as a complaint. `renamed[].torrent_path` stays the path the
metainfo gives, because the mapping is only useful with both ends in it.

**An index the torrent does not have is a usage error**, checked before the
session starts wherever the count is already known. A magnet has no count until
its metadata resolves, so `-O` now joins `--exclude-file` and an open-ended
`--select-file` in `plan_selection`'s "await the count" branch: the metadata is
resolved first, which is a round trip the magnet was going to make anyway, and
the index is checked against a real file list. Without that, `-O 9=x` against a
five-file magnet would have renamed nothing and said nothing.

**Half of it would have shipped without the second command, and that half was
found by asking.** `verify` looks where the bytes went rather than where the
torrent said, which is [T-076](windows.md), and it builds that answer from
`paths::plan` — which knows nothing about `-O`. So the tree could rename a file
its own verifier then reported as missing. `verify` takes `-O` too now, and
`verify_finds_a_file_renamed_by_index_out_when_it_is_told` holds both
directions: told, `present: true` and `complete: true`; not told, `present:
false` and a `hash_mismatch` document.

`seed` is **not** covered, and this is the residual, named rather than implied:
`bit-cli seed` resolves its payload through the same plan and has no `-O`, so a
payload downloaded with `-O` cannot be seeded from the directory it landed in.
It is `crates/bit-cli/src/cmd/seed.rs:260`, where `AddOptions` is built without
`index_out`. [T-213](#t-213-seed-cannot-serve-a-payload-renamed-by-index-out)
carries it.

```
$ cargo test -p bit-cli --lib index_out
test result: ok. 4 passed; 0 failed; 0 ignored; 395 filtered out

$ cargo test -p bit-cli-core --lib paths::
test result: ok. 35 passed; 0 failed; 0 ignored; 660 filtered out
```

The acceptance itself, `index_out_writes_the_file_where_the_caller_asked`:
`--json` reports `{"index":0,"disk_path":"renamed/first.bin","reasons":["requested"]}`,
the bytes are at that path and byte-identical to the torrent's first file, and
nothing is left at the path the torrent named.

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
Status:      **done** 2026-08-23T07:05Z

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
| `every_short_flag_is_documented_in_the_flags_table` | `cli.rs:2547` | a short flag with no row in `docs/flags.md` |
| `no_short_flag_is_defined_twice` | `cli.rs:2322` | one letter used twice in one command |
| `short_flags_never_contradict_aria2` | `cli.rs:2358` | an `aria2` letter reassigned to a different concept |
| `short_flags_keep_their_aria2_meanings` | `cli.rs:2053` | `-V` no longer meaning `--check-integrity` |

```
$ cargo test -p bit-cli --lib short_flag
test result: ok. 4 passed; 0 failed; 0 ignored; 303 filtered out
```

The third of those is the one A3.2 actually asked for: it holds the reserved
list: `d` dir, `o` out/output, `j` max-concurrent-downloads, `u`
max-upload-rate, `q` quiet, `c` continue, `V` check-integrity, `O` index-out,
`l` log-file. It requires any flag carrying one of those letters to name the
matching id or not exist.

**One clause of the Acceptance was genuinely unmet, and it is why this stayed
open.** The Acceptance says a test "regenerates it and fails on drift". The test
*asserted* and did not regenerate: it failed with the exact row to add, which a
reader then pasted in. That is a deliberate difference and probably the better
one, see [T-158](#t-158-regenerating-the-schema-deletes-fields-the-sample-did-not-produce),
where the regenerating half of the schema check deletes rows the sample did not
produce, but the entry asked for regeneration and did not get it, so the
honest state is open with the gap narrowed to one clause. Dropped from P2 to
P3: nothing is unprotected.

`docs/flags.md` named the test as `every_short_flag_is_documented`, which is
not its name. Corrected in the same pass. A doc citing a symbol that does not
exist is the same defect class as an entry describing a state the tree is not
in, which is what this correction is.

**Done 2026-08-23T07:05Z**, and the regeneration is a **merge** rather than a
render, which is the only shape T-158 leaves available.

```bash
BIT_CLI_UPDATE_FLAGS=1 cargo test -p bit-cli --lib short_flag
```

Three of the table's five columns, `Scope`, `aria2` and `Note`, are things the
command tree cannot know: nothing in `clap` knows what `aria2` calls a letter or
why `-v` diverges. So `merge_flags_table` keeps an existing row **verbatim**,
adds a row for a flag that has none with those three cells empty for a person,
and drops a row whose flag the binary no longer defines. Rendering the table
instead would delete every hand-written cell in it, which is T-158 arriving in a
second file.

**A second direction of drift was open the whole time and nobody had noticed.**
The old test walked the flags and asked the table about each, so a row for a
flag that no longer exists passed. That is the drift `-O`/`--index-out` would
leave behind if [T-116](#t-116--o--index-out-cannot-rename-a-file) were ever
answered by removing the flag rather than implementing it. Both directions fail
now.

**`-h` is not in the tree the test walks**, which the stale-row check found the
first time it ran. `clap` creates `--help` while **building** a command, and
`Cli::command()` returns one that is not built, so `get_arguments()` does not
carry it. The table's row for `-h` had therefore never been checked in either
direction. `short_flags` adds the pair by hand, with why.

Two tests, not one. The assertion runs against the committed file and the merge
is tested on a fixture of its own, because on the committed file the merge is a
no-op by construction: the assertion fails the build whenever it would not be.
`regenerating_the_flags_table_adds_and_removes_rows_without_touching_prose`
checks that a kept row keeps every hand-written cell, that a new one arrives
empty, that a dead one goes, that "Reserved and not assigned" is untouched, and
that a second run changes nothing.

```
$ cargo test -p bit-cli --lib short_flag
test result: ok. 4 passed; 0 failed; 0 ignored; 386 filtered out

$ BIT_CLI_UPDATE_FLAGS=1 cargo test -p bit-cli --lib every_short_flag_is_documented
test result: ok. 1 passed; 0 failed; 0 ignored; 389 filtered out
$ git diff --stat docs/flags.md
(nothing)
```

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
Status:      **done** 2026-08-23T07:18Z

Problem:     `Kind::classify` checked the `http://` and `https://` prefixes
             before it checked the `.meta4` and `.metalink` extensions, so
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

**Done 2026-08-23T07:18Z.** `Kind::MetalinkUrl(String)` is the fourth branch,
`source::fetch_metalink` is the only new code on the resolve path, and
`resolve_metalink` is unchanged: it takes a parsed document and does not know
where it came from, which is what the Approach predicted.

**The extension is read from the URL's path and not from the whole string**, and
that is a decision the entry did not name. `?file=r.meta4` is a query naming a
file and `#r.metalink` is a fragment, and neither says what the URL serves; a
`MirrorBrain` instance generating a document per request is exactly the place a
query string turns up. `only_the_url_path_decides_whether_it_is_a_metalink`
holds both directions, including `https://e.com/r.meta4?mirrorlist`.

**The redirect case needed no decision after all.** The Approach said one was
owed. Nothing in either path resolves a mirror URL relative to anything, so a
document fetched over HTTP is treated exactly as one read from disk: absolute
URLs are used and relative ones are refused, on both paths. A document with
relative mirror URLs would need a base, and refusing it on one path and
resolving it on the other is the divergence worth avoiding.

**`--dry-run` is the one place the two kinds differ, and it is a decision.** A
saved `.meta4` is readable with nothing running, so a dry run reports every
claim in it. A URL is not: the document itself is the thing to fetch. It is not
fetched, for the reason already written into that same function about
`--web-seed-list-url`, and `torrents[].document_needs_network` on the row is
what says the block is absent because nothing was contacted rather than because
the document was empty.

**The acceptance, run against the live mirror**, is
`bench/metalink-real-20260823T071745617Z.json`, case `real_by_url` beside
`real_as_served`. Same exit code and the same message, character for character,
from a document the instance generated per request:

```
real_as_served  exit 4  the metalink lists no torrent for LibreOffice_...msi,
                        so there is nothing to download here. It lists 58 HTTP
                        mirror(s); ...
real_by_url     exit 4  (identical)
```

**That case cannot prove the download half**, because no reachable MirrorBrain
instance emits `<metaurl mediatype="torrent">`, which is what `real_as_served`
has recorded since [T-113](#t-113-metalink-is-not-implemented). The download
half is proven on loopback: case `url_source` in `scripts/check-metalink.ps1`,
`bench/metalink-20260823T071256391Z.json`, which serves the `v4_ok` document
over HTTP and compares the resulting `metalink` block **field by field** with
the run over the saved copy. They are identical except `checksum.path`, which
must differ because each case writes into its own output directory and which is
asserted separately rather than dropped.

`a_metalink_named_by_url_downloads_the_same_as_one_on_disk` is the same
comparison in `cargo test`, so CI carries it: CI runs neither script.

```
$ cargo test -p bit-cli --lib a_metalink_named_by_url
test result: ok. 1 passed; 0 failed; 0 ignored; 394 filtered out
```

### T-155 --hash-check-only drops the metalink report

Source:      `bit-cli` design, found closing [T-113](#t-113-metalink-is-not-implemented)
Category:    cli
Priority:    P3
Effort:      S
Status:      **done** 2026-08-23T07:06Z

Problem:     `one_inner` returned early for `--hash-check-only`, before the
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

**Done 2026-08-23T07:06Z, and the interesting case is the one that happened.**
The Acceptance allows either a checked digest or a `not_checked` reason, and
over a complete payload it is the digest: `matched: true`, 2,097,152 bytes
hashed, against the file at the path the report names. That is the strongest
thing this flag can report, because the hash check has already proved those
bytes against the torrent and the checksum then proves the same bytes against
the Metalink. `check_metalink` decides it from `report.finished` and needed no
branch of its own, which is what the Approach predicted.

The block that built the report was inline at `one_inner`'s normal exit. It is
`apply_metalink` now, called at both exits, so the two cannot drift apart the
way they did.

**`bench/metalink-20260823T070301761Z.json`** is the run, case
`hash_check_only`, eleventh in `scripts/check-metalink.ps1`:

```json
{"agreement":{"file_index":0,"matched_by":"only_file","metalink_size":2097152,
 "size_agrees":true,"torrent_size":2097152},
 "checksum":{"algorithm":"sha256","matched":true,"bytes_hashed":2097152},
 "mirrors_listed":1,"mirrors_registered":1,"version":"4"}
```

**The same case is in `cargo test` as well, and that is deliberate.** CI does
not run `scripts/check-metalink.ps1`, so an acceptance that lived only there
would catch a return moved back above the call only when somebody ran it by
hand. `hash_check_only_over_a_metalink_still_reports_the_document` downloads the
payload, then checks it, and asserts the block.

**It was checked against the defect rather than assumed to cover it.** With the
`apply_metalink` call removed from that exit the test fails on
`no metalink block`, and the document it prints is a `download` report with no
`metalink` key at all. A test written for a fixed defect and never run against
the defect is a test that may be asserting something else.

```
$ cargo test -p bit-cli --lib hash_check_only_over_a_metalink
test result: ok. 1 passed; 0 failed; 0 ignored; 390 filtered out
```

### T-156 A dry run writes a different shape under the same document kind

Source:      `bit-cli` design, found closing [T-113](#t-113-metalink-is-not-implemented)
Category:    cli
Priority:    P3
Effort:      S
Status:      **done** 2026-08-23T06:44Z

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

**Done, all three clauses.** `dry_run` in `cmd/download.rs` emits
`download_dry_run`, `DOCUMENT_KINDS` in `schema.rs` names it with why it is its
own kind, and `schema_gen.rs` drives two runs it folds into one table.
`dry_run: true` stays on the document, so a reader holding one does not have to
know the kind changed.

**Two runs rather than one, because neither reaches every field.** A Metalink
dry run is the only source kind that fills `torrents[].metalink`; a torrent one
is the only one that resolves a file layout, so it is the only one with
`torrents[].coverage` and a real `info_hash`.

**The order of the two is load-bearing, and the first attempt got it wrong.**
`Sample::merge` is `or_insert`, so the **first** observation of a path names its
type and later ones can only add paths. With the Metalink run first, the
committed table said `torrents[].info_hash`, `name` and `total_bytes` were
`null`, which is what a Metalink dry run leaves them as and is not what the
field is. Taking the torrent run first gives `string`, `string` and `integer`,
and the Metalink run still contributes every `metalink.*` row. This is the same
shape of defect as [T-158](#t-158-regenerating-the-schema-deletes-fields-the-sample-did-not-produce):
what the generator writes depends on what the sample happened to contain.

`a_dry_run_writes_its_own_document_kind` asserts both halves. A real run is
still `download` and carries no `dry_run` field, which is what stops the case
passing if the kind is simply renamed everywhere.

```
$ cargo test -p bit-cli --lib a_dry_run_writes_its_own
test result: ok. 1 passed; 0 failed; 0 ignored; 388 filtered out
```

**A defect in the tooling turned up on the way, and it is fixed here.**
`scripts/check-man.ps1 -Fix` generates the manuals by running
`target/release/bit-cli.exe`, and it did not build one first. A stale binary
regenerated all three files from the command surface as it was at the last
release build, wrote them, and printed "regenerated"; `git diff man/` was then
empty while `cargo test --test man_is_current` went on failing, because that
test renders from the crate being compiled. `gates.ps1` reported `man ok` and
`test FAILED` in the same run, which reads as the test being wrong. `-Fix`
builds first now, and without `-Fix` the script compares the binary's timestamp
against the newest `.rs` under `crates/` and defers to the test rather than
answering about a surface that no longer exists.

### T-158 Regenerating the schema deletes fields the sample did not produce

Source:      `docs/schema.md`, found during the doc pass of 2026-08-21
Category:    cli
Priority:    P2
Effort:      S
Status:      **done**

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
`schema_gen.rs:1154` `the_committed_schema_matches_what_the_program_writes`
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

**Fixed by giving the writer the reader's tolerance, which is what the entry
already said the fix was.**

`merge_schema` in `crates/bit-cli/src/schema_gen.rs` reads the committed file,
indexes its field rows by the section they sit under, and unions them into what
this run rendered. Where both carry a path, **this run's type wins**: the
committed one is a record of an older measurement and this one is current.
Where only the committed file has a path, the row survives.

The section key is the `##` heading and the `###` heading together, not the
`###` alone. A document kind and an event type may share a name, and their
field lists are different things; keying on the inner heading alone would let
one section's rows leak into the other's. The test asserts that directly with a
row that exists only under `## Events`.

**A field that is genuinely gone now has to be deleted on purpose.** That is
the right cost for removing something from a versioned contract, and it is the
trade this entry named.

**Found again by following the instruction, on this session's own change.**
Adding `gone_files` and `pieces_dropped` to `SourceReport` for
[T-005](webseed.md) made the contract check fail, correctly, naming the four
new rows. Regenerating the way the panic message says removed two:

```
$ diff /tmp/committed.md docs/schema.md     # the old overwriting writer
535d534
< | `sources[].error` | string |
793d791
< | `cooldowns` | integer |
794a793,795
> | `gone_files[].file` | integer |
> | `gone_files[].pieces_dropped` | integer |
> | `gone_files[].reason` | string |
798a800
> | `pieces_dropped` | integer |
```

Two rows lost, and neither is the `sources[].error` pair this entry recorded
before: `cooldowns` is new to the list. That is the entry's own point about the
count made a third time. **The number of rows lost is a property of the run**,
so it was one, then two of one kind, then two of two kinds. The mechanism is
the defect.

With the merging writer, the same regeneration on the same tree:

```
$ diff /tmp/committed.md docs/schema.md     # the merging writer
794a795,797
> | `gone_files[].file` | integer |
> | `gone_files[].pieces_dropped` | integer |
> | `gone_files[].reason` | string |
798a802
> | `pieces_dropped` | integer |
```

Additions only. `sources[].error` and `cooldowns` survive.

**Two tests, and they are the acceptance in its own words.**

`regenerating_the_schema_keeps_rows_this_run_did_not_produce` is the unit case
on hand-written input: a row only the committed file has survives, a row only
this run produced is added, a path both carry takes this run's type and not
both, a row from another section does not leak in, and the merged rows stay
sorted by path so merging does not churn the diff.

`regenerating_the_schema_is_idempotent` is "regenerating twice in a row leaves
every row that either run produced, and `git diff` is empty when nothing
changed", stated as two equalities: merging a render into itself reproduces it
exactly, and merging again changes nothing.

**What was not changed.** The read-side check stays a containment check, and
its asymmetry stays deliberate: these runs are timed, so a download that
finishes before its second report tick emits no `progress` and requiring
equality would make the contract check flaky. The two halves are now tolerant
in the same direction, which is all that was ever wrong.

### T-159 Subcommand flags are filed under "Report options" in the help

Source:      `bit-cli bench <SUB> --help`, found in the doc pass of 2026-08-21
Category:    cli
Priority:    P3
Effort:      S
Status:      **done** 2026-08-23T06:52Z

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

**Done, and the entry undercounted the defect.** It named four `bench`
subcommands. The fifth place it happens is the front door: `bit-cli --help` had
**no "Arguments" section at all**, because `Cli::sources` is declared after the
`Global` flatten and a positional inherits the running heading like any other
argument. `[SOURCE]...` was documented at the bottom of "Global options", 100
lines below the usage line that names it. Nothing in the entry predicted that,
and it was found by the test rather than by reading:
`no_positional_is_pulled_into_a_help_heading` walks the whole command tree and
failed on `sources` the first time it ran.

Each of the four subcommand structs now sets its own heading, `Swarm options`,
`Leech options`, `Seed options` and `Disk options`, **and flattens the shared
groups last**. The heading alone is not enough: `next_help_heading` is applied
once at the top of `augment_args`, so a field after a flatten still inherits
whatever that flatten left behind. `bench probe` gets `Probe options` too, so
its two flags are correct by construction rather than by accident.

`help_heading = None` on the three positionals is the other half.
`#[command(next_help_heading)]` covers a struct's positionals as well, so
without it `<TARGET>` moved out of "Arguments" and rendered *after*
`--connect-timeout`, which is a worse place than the one it started in.

Three tests, and the split matters. `only_report_flags_are_filed_under_report_options`
asserts the property rather than the fix, so flattening last is not the only
shape that passes. `every_bench_subcommand_files_its_report_flags_under_report_options`
is its inverse, because a heading that files *nothing* under it would pass the
first one. `no_positional_is_pulled_into_a_help_heading` walks every command,
not just `bench`.

```
$ cargo test -p bit-cli --lib report_options
test result: ok. 2 passed; 0 failed; 0 ignored; 386 filtered out

$ cargo test -p bit-cli --lib no_positional
test result: ok. 1 passed; 0 failed; 0 ignored; 387 filtered out
```

The acceptance, run:

```
webseed  --report --format --baseline --fail-under
leech    --report --format --baseline --fail-under
seed     --report --format --baseline --fail-under
disk     --report --format --baseline --fail-under
probe    --report --format --baseline --fail-under
swarm    --report --format --baseline --fail-under
```

`man/` is unchanged by this, checked with `scripts/check-man.ps1 -Fix` and an
empty `git diff man/`. The generated manuals group flags by command rather than
by help heading, so the heading is a terminal-only surface.

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

**It failed again on 2026-08-21T17:00Z, differently, and the fix above was only
half of it.** CI run **32505742044** turned `Test (ubuntu-latest)` red on the
[T-172](metainfo.md) push:

```
thread '...' panicked at crates/bit-cli/src/cmd/peers.rs:448:9:
assertion `left == right` failed: {... "connecting":1, "live":0, "seen":1,
  "peers":[{"errors":0,"downloaded_bytes":0,"state":"connecting"}]}
  left: Number(0)
 right: 2000
```

Read it against the failure above: `errors: 0` and `state: connecting` where
the first one had `errors: 1` and `state: dead`. The dial was **not** lost this
time. `wait_for_listener` did its job, the peer was reached, and the handshake
was still in flight when the five second sample ended. So the first fix
addressed the race it named and left the guessed duration behind it, which is
the half [RULES.md](RULES.md) actually states: a test waits on the condition,
never on a guessed duration.

Rerunning the same job on the same commit with no change passed, which is what
separates a flake from a break and is worth doing before touching anything.

**Fixed by sampling until the bytes arrive.** `--duration` is the command's own
contract and it samples for exactly that long, so the test cannot make one
sample longer without changing what it is testing. What it can do is sample
again: the run repeats until a report shows bytes moved or twenty-five seconds
pass, and asserts on that report. On an unloaded machine the first sample
succeeds and it costs nothing.

The seeder is no longer joined. It runs for forty seconds so the retries have
something to dial, and waiting for it to time out would have made every run of
this test as long as its worst case: joining a 90 second seeder took the test
from six seconds to ninety-one, measured. The thread dies with the test binary.

**What this says about the previous fix, and about the next one.** T-160's
`Approach` line was already the right rule, written down and then applied to
only one of the two guesses in the test. A fix that quotes the rule and half
applies it reads as complete in review, which is how this cost a second red
job. Every timing assumption in a test has to be listed before one of them is
fixed, not after the next failure names it.

### T-161 A CI action still targets Node.js 20, which is deprecated

Source:      CI run 32457763652 annotations, 2026-08-21
Category:    ci
Priority:    P3
Effort:      S
Status:      **done** 2026-08-23T08:35Z

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

**Done, and it was done in the session of 2026-08-23 that closed
[T-199](#t-199-the-ci-supply-chain-was-unwatched-and-one-action-was-abandoned)
without this entry being closed with it.** The action is gone from all four call
sites and from `release.yml`'s fifth: every one of them runs
`pwsh -NoProfile -File scripts/setup-nasm.ps1`, which pins the version and
checks what it downloads. `.github/workflows/ci.yml:88` is the comment that says
why, and it names the action this entry is about, which is what made it look
present to anything searching the file for a string.

The Acceptance holds: the Windows jobs are green, which is what says NASM is
still being found, and no run since carries a Node.js deprecation annotation for
it. Confirmed against run **32628316314**, and `grep -rn "uses:" .github/`
carries eight distinct actions and `ilammy/setup-nasm` is not one of them.

**Why nothing caught it, and what does now.** Two gaps in
`scripts/check-todo.ps1`, both closed on 2026-08-23:

1. **`.github/` was not in the cited-path prefixes at all.** The regex resolved
   `crates|scripts|docs|vendor|patches|man` and nothing else, so this entry's
   four citations of `.github/workflows/ci.yml:<line>` were never checked for
   the file existing, for the line existing, or for anything else. That is now
   a sixth prefix.
2. **Nothing compared an entry's premise to the workflows.** A new check reads
   the `uses:` lines of `.github/workflows/*.yml` and fails when an **open or
   partial** entry names an `owner/name@ref` pin that no workflow carries. That
   is the one shape of "this entry describes a state the tree is not in" that
   can be decided mechanically: nothing else in this record is spelled
   `owner/name@ref`. Closed entries are exempt, because one quoting the pin it
   removed is evidence, which is the same rule the drifted-line check already
   follows for a fenced citation.

**The first draft of check 2 passed this entry**, and the reason is worth
keeping: it searched the raw text of the workflow files, and `ci.yml` carries
the comment "Ours, not ilammy/setup-nasm: that action is unmaintained". A
substring search found the very action the comment exists to say is gone. It
reads `uses:` lines only now.

```
$ pwsh -NoProfile -File scripts/check-todo.ps1
  [stale-premise] cli-surface.md:1804 : T-161 is open and names the action
  `ilammy/setup-nasm@v1.5.2`, which no workflow uses.
```

That is the output that closed this entry, produced by the check written for it.

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
             `enable_pex` beside them.
             `nanotorrent/patches/0004-pex-toggle.patch` adds exactly that:
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
             `cli.rs:2547` `every_short_flag_is_documented_in_the_flags_table`
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

             The flag is also undocumented: it appears nowhere in `README.md`.
             That is how it stayed unnoticed, because nothing described it and
             so nothing contradicted what it did. `docs/flags.md` is not the
             gap, and saying so would mislead: that file is **short flags
             only**, by its own opening line and by the rule it exists to
             enforce, and no long flag has a row in it. The sibling
             `--web-seed-file` is undocumented in `README.md` too, so this is
             one instance of a wider gap rather than a hole around one flag.
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


### T-185 --exclude-file on its own selects nothing and downloads everything

Source:      found while measuring [T-184](disk-io.md), 2026-08-21
Category:    cli
Priority:    P1
Effort:      S
Status:      **done**, 2026-08-22T01:40Z

Problem:     `--exclude-file <INDEX>` skips files. Used **without**
             `--select-file` it does nothing at all: `selection` in
             `crates/bit-cli/src/cmd/download.rs` returns `None` the moment the
             selected list is empty, `None` means every file, and the excluded
             set is dropped on the floor. The comment above that `return`
             says the exclusion "is applied once the metadata resolves", and
             nothing anywhere applies it. `options.only_files` has exactly one
             reader, the `AddOptions` built in `one_inner`, and it receives
             `None`.
Relevance:   This is the `INDEX.md` P1 definition verbatim, and it is the third
             of its family after [T-181](#t-181-four-flags-are-accepted-in-silence-and-reach-no-code)
             and [T-183](#t-183---web-seed-list-url-is-read-only-into-a-refusal).
             It is a different shape from both, which is why neither audit
             found it: the field **is** read, and the reader **can** succeed.
             What is wrong is that one branch of the function that computes the
             value discards half its input. A flag that works when paired with
             another flag and silently does nothing alone is invisible to any
             check that asks whether a field reaches code.

             The cost is not a missing file, it is a fetched one.
             `--exclude-file` is how a caller skips the 40 GiB extras track in a
             torrent it wants 200 MiB of, so the failure mode is a download
             that is two orders of magnitude larger than asked for and reports
             `completed`.
Approach:    The excluded set alone needs the file count, which is the reason it
             was left. Both halves of that are available:

             1. **A local `.torrent`, a fetched one, or a Metalink.** `run`
                already parses the metainfo into `metas` before any plan starts,
                for [T-140](multi-source.md)'s donation proof. The count is
                there, so the exclusion resolves before `add` and nothing is
                ever fetched.
             2. **A magnet.** No file list until the metadata resolves.
                `librqbit` 9.0.0 has `Api::api_torrent_action_update_only_files`
                at `src/api.rs:337`, so the selection can be narrowed after
                `wait_until_initialized` and before anything is asked for. A
                magnet has no payload to fetch before that point, so nothing is
                wasted.

             Refusing a magnet outright is the cheaper option and is worse: it
             would make the flag work for one source kind and error on another,
             which is the asymmetry `--select-file` does not have.

             While this is open, `--select-file` with an **open-ended** range,
             `--select-file 3-`, is refused for the same missing count. Both
             halves of the count problem have the same two answers, so decide
             them together.
Acceptance:  A two-file torrent served by one mirror, downloaded with
             `--exclude-file` naming one file and no `--select-file`, finishes
             with only the other file under `--dir`, and the mirror is never
             asked for the excluded file's URL.

**Measured, not read.** A `sharing_pair` donor fixture is `extra-a.txt` (1024
bytes) and `shared.bin` (4096) at a 1024 byte piece length, so every file is a
whole number of pieces and no boundary straddles: what lands on disk is exactly
what was selected, with none of [T-184](disk-io.md)'s boundary writes to
confuse it. `create` sorts by path, so index 0 is `extra-a.txt`.

```
$ bit-cli --json download donor.torrent --dir out --web-seed-only \
    --web-seed http://127.0.0.1:PORT/payload/ --web-seed-mode prefix \
    --no-torrent-web-seed --no-tracker --port 0 --exclude-file 1 --stop-after 20s
stopped=completed
files on disk: ["donor/extra-a.txt", "donor/shared.bin"]

$ ... --select-file 0 ...
stopped=completed
files on disk: ["donor/extra-a.txt"]
```

The first run excluded index 1 and downloaded it anyway. The second selected
index 0 and got exactly that, which is the control: the selection machinery
works and only the exclusion-alone path is dead. An earlier run of the first
command against a mirror missing that file failed with a 404, which is the same
finding from the other side: the run asked a mirror for a file it had been told
to skip.

**Closed 2026-08-22T01:40Z**, and both halves of the count problem were decided
together the way the approach asked.

The count is per source, not per run, so it is resolved per source. `run`
already parses the metainfo of a local `.torrent`, a fetched one and a
Metalink's into `metas` before any plan is handed out, so `plan_selection` in
`crates/bit-cli/src/cmd/download.rs` settles each plan's `FileSelection` there,
before the session starts. A usage error surfaces before anything is added
rather than per worker.

A magnet defers, and only when it has to.
`crate::selection::needs_file_count` is the one place that says which two
spellings need a count: an exclusion with no selection beside it, and an
open-ended range. A magnet with neither adds exactly as before and pays
nothing.

**The magnet answer is not the one the approach named, and the reason is worth
recording.** `Api::api_torrent_action_update_only_files` does exist and does
narrow a live torrent, but narrowing **after** the add is too late for the
thing this entry is about. `librqbit`'s initial check creates and opens every
file it was not told to skip, so a selection applied afterwards has already
created what it excludes. Measured twice, from both sides:
`--hash-check-only --select-file 1` against an empty directory creates the
selected file at its full length and no other, and [T-186](#t-186-seed---data-and-verify---data-resolve-the-payload-differently)'s
`seed` against an empty directory, which has no selection at all, creates the
whole tree. `Engine::resolve_with` reads the metadata first, with
the caller's own trackers and `--peer` addresses so it resolves against the
swarm the add is about to use, and it hands back the `.torrent` bytes it built.
The add then takes those bytes, so this is one metadata resolution and not two.
The seam upstream is `librqbit-9.0.0/src/session.rs:1298`, where `list_only`
returns after `resolve_magnet` and before any storage exists.

That resolution is bounded by `--init-timeout`, which `engine.add` never bounded
for a magnet at all. A swarm that never answers now reports the phase rather
than hanging the run.

`crate::selection::resolve` no longer answers `None` when it is asked for an
exclusion's complement without a count. `None` means every file, which is the
flag doing the opposite of what it says, and that silence is what this entry
was. It is a usage error now, so a caller that skips `needs_file_count` fails
loudly instead of quietly downloading everything.

Measured against `target/release/bit-cli`, on the fixture above rebuilt with
`bit-cli create --piece-length 1024`: `extra-a.txt` 1024 bytes at index 0,
`shared.bin` 4096 at index 1, five pieces, nothing straddling.

```
$ bit-cli --json download donor.torrent --dir out --web-seed-only \
    --web-seed http://127.0.0.1:57364/ --web-seed-mode prefix \
    --no-torrent-web-seed --no-tracker --no-dht --no-lsd --port 0 \
    --exclude-file 1 --stop-after 20s
stopped= completed
downloaded= 1024
files on disk: donor/extra-a.txt
mirror was asked for: GET /   GET /extra-a.txt
```

The mirror's own log is the half that says the exclusion was applied before the
fetch rather than after it: `GET /` is [T-004](webseed.md)'s style probe, and
`shared.bin` was never asked for.

The magnet, against a loopback seeder with `--peer` and no tracker, no DHT and
no LSD:

```
$ bit-cli --json download magnet:?xt=urn:btih:9bef473bd4483a6e51c2f5194e983712f8edfec0 \
    --dir out --peer 127.0.0.1:51899 --no-tracker --no-dht --no-lsd --port 0 \
    --exclude-file 1 --init-timeout 60s --stop-after 60s
stopped= completed
downloaded= 1024
files on disk: donor/extra-a.txt
```

And `--select-file 1-`, the open-ended range that was refused for the same
missing count, on the same magnet:

```
$ ... --select-file 1- ...
stopped= completed
downloaded= 4096
files on disk: donor/extra-a.txt (0 bytes), donor/shared.bin (4096 bytes)
```

Six tests. `an_exclusion_with_no_selection_skips_the_file_and_never_asks_for_it`
and `a_magnet_resolves_its_metadata_before_it_applies_an_exclusion` are the two
acceptances, and both were run against the old behaviour first: the magnet one
fails with `["donor/extra-a.txt", "donor/shared.bin"]`, which is the defect.
`crate::test_support::FileServer` grew a request log for the first of them,
because what a mirror was **not** asked for is the only evidence that a
selection was applied before the fetch.

**That third run found something this entry did not**: `extra-a.txt` lands as a
zero byte file when it is not selected and the selection starts after it. It is
not this entry's, and it is not new: `--select-file 1`, which needed no count
and went through unchanged code, does the same. It is filed as
[T-188](disk-io.md) with the cause, and it corrects
[T-013](disk-io.md)'s closing claim.

### T-186 seed --data and verify --data resolve the payload differently

Source:      found while building [T-184](disk-io.md)'s acceptance, 2026-08-21
Category:    cli
Priority:    P3
Effort:      S
Status:      **done**, 2026-08-22T03:00Z

Problem:     A multi-file torrent lays its files under a directory named after
             itself, so a payload can be pointed at two ways: at the parent, or
             at the torrent directory. `verify --data` accepts either and picks
             whichever holds the first file, which its `resolve_root` says in
             so many words. `seed --data` sets the session's download directory
             and only ever looks at `<data>/<name>/`, so the torrent directory
             is refused with no message that says so.
Relevance:   The two commands read the same layout, written by the same
             `download`, and their `--data` flags carry the same name and the
             same help text. A caller who verified a payload one way and seeds
             it the other gets a seeder holding nothing.

             What makes it worth a P3 rather than nothing is the message.
             Pointed at the torrent directory, `seed` reports `have: 0` and
             warns "only 0 B of 3.61 KiB is present, so this is a partial
             seed", which is the right observation with the wrong reason. A
             partial seed is legitimate and the warning is the one a partial
             seed gets, so nothing distinguishes "you have half the payload"
             from "you named the wrong directory".
Approach:    Give `seed` the resolution `verify` already has. It is one call,
             and the two commands would agree by construction rather than by
             both being right separately.

             The alternative, warning when nothing is found and a sibling
             directory would have worked, is a special case where a shared
             function is available, and it leaves the two flags meaning two
             things.

             Watch the direction: `verify` picks whichever candidate holds the
             **first file**, and a run that legitimately holds nothing at all
             has no first file to find. That is why `seed` cannot simply take
             the same function without deciding what it does when neither
             candidate exists, which today is what `--data` said.
Acceptance:  `bit-cli seed <MULTI> --data out/<name>` and
             `--data out` report the same `have` for the same payload on disk,
             and a `seed` that finds nothing where a sibling directory holds
             the payload says which directory it looked in.

**Measured before building, and the premise held exactly.** A two-file torrent,
3,000 and 1,000 bytes at a 1,024 byte piece length:

```
$ bit-cli verify album.torrent --data .tmp/t186        pieces ok 4 of 4
$ bit-cli verify album.torrent --data .tmp/t186/album  pieces ok 4 of 4

$ bit-cli seed album.torrent --data .tmp/t186          have 3.91 KiB of 3.91 KiB
$ bit-cli seed album.torrent --data .tmp/t186/album
warning: only 0 B of 3.91 KiB is present, so this is a partial seed
                                                       have 0 B of 3.91 KiB
```

**One thing the entry did not know**: the wrong spelling does not only report
nothing, it writes. `seed` hash-checks on add, which creates the tree it is
looking for, so pointing at the torrent directory left an empty `album/album/`
inside it at full length.

**Closed 2026-08-22T03:00Z.** `crate::payload::resolve` is the shared rule, in a
module of its own for the reason [`crate::selection`](#t-185---exclude-file-on-its-own-selects-nothing-and-downloads-everything)
is: two commands need the same answer from the same flag, and a second copy is a
second set of off-by-one bugs. `verify::resolve_root` is now the `--data` fallback chain and
one call to it.

`seed` takes the resolved root as `AddOptions::output_folder` rather than as the
session's download directory. That is what makes it right for a **renamed**
payload directory as well: naming the folder means the files hang directly off
it, where letting the session append the torrent's own name assumes the
directory is still called that.

```
$ bit-cli seed album.torrent --data .tmp/t186        have 3.91 KiB of 3.91 KiB
$ bit-cli seed album.torrent --data .tmp/t186/album  have 3.91 KiB of 3.91 KiB
```

and nothing is created a level deeper by either.

**The message went through two shapes and the second one is the point.** The
first said the first file was in neither candidate, which is what
`resolve` actually checks. It is true on the first run and false on every run
after it, because the run before created that file at full length with nothing
in it. Keyed on bytes instead:

```
$ bit-cli seed album.torrent --data .tmp/t186/empty
warning: only 0 B of 3.91 KiB is present, so this is a partial seed
warning: none of album is in <dir>\empty, which is where --data
         resolved to; a multi-file torrent's files also sit under
         <dir>\empty\album
```

Two warnings, and they say different things on purpose. The first is what a
partial seed gets and a partial seed is legitimate. The second only fires on
nothing at all, which is the case a partial seed's wording could not describe,
and it names both directories. A complete seed says neither, which
`a_complete_seed_says_nothing_about_where_it_looked` pins.

Seven tests. `either_spelling_of_data_seeds_the_same_payload` is the acceptance
and was run against the old behaviour first: the torrent directory reports
`have: 0` where the parent reports 2,000.
`a_seed_that_holds_nothing_names_the_directories_it_searched` runs twice over,
because a message keyed on the files existing would pass once and fail after.

### T-193 A citation written short was never checked at all

Source:      found in this session's own review 1, 2026-08-22
Category:    cli-surface
Priority:    P2
Effort:      S
Status:      **done**, 2026-08-22T11:21Z

Problem:     `scripts/check-todo.ps1` resolved a citation written long, as
             `crates/bit-cli/src/cli.rs:2322`, and checked only that the file
             had that many lines. Most of `TODO/` does not write them long. A
             citation written as `cli.rs:2322` matched nothing in the pattern,
             so it was never resolved, never range checked, and never read.
Relevance:   `RULES.md` section 2 step 4 says the mechanical half of the two
             reviews answers "a cited path that does not resolve". For the
             common spelling it answered nothing, and the record is built on
             citations.
Approach:    Index every `.rs` under `crates/` by bare name and resolve a short
             citation through it, skipping a name two files share, because
             guessing which one was meant is worse than saying nothing. Then
             check the line rather than only the count: where the prose names a
             symbol beside the citation, and that symbol occurs **exactly
             once** in the file, the citation has to be within a few lines of
             it. Once, because a name the file uses twice cannot say which
             occurrence was meant, and a wrong complaint is worse than a
             missing one.
Acceptance:  A citation whose target has moved fails the check, named, with the
             line it moved to.

**What it found the day it was written: nine stale line numbers across seven
citations, in prose four sessions of two-deep-reviews had passed.**

The old line numbers are written without their file here, so this record does
not read as seven live citations and report itself.

| file | what it names | said | is at |
| --- | --- | --- | --- |
| `cli.rs` | `short_flags_keep_their_aria2_meanings` | 1833 | 1924 |
| `cli.rs` | `no_short_flag_is_defined_twice` | 2012 | 2103 |
| `cli.rs` | `short_flags_never_contradict_aria2` | 2048 | 2139 |
| `cli.rs` | `every_short_flag_is_documented_in_the_flags_table` | 2107 | 2332 |
| `schema_gen.rs` | `the_committed_schema_matches_what_the_program_writes` | 734 | 1068 |
| `storage.rs` | the two BEP 47 padding guards | 728 and 870 | 1048 and 1216 |
| `storage.rs` | `pwrite_all_vectored` and `pwrite_all` | 799 and 781 | 1119 and 1107 |

Three of the four `storage.rs` numbers, 728, 799 and 781, were correct at
`f46d4fd` and were moved by the write buffer [T-018](disk-io.md) landed the same
morning, checked by reading the file at that commit. 870 was already wrong
there: the guard it names was at 891. The five in `cli.rs` and `schema_gen.rs`
had been wrong for longer. A tenth, `storage.rs:402` in
[T-190](disk-io.md)'s own Approach, was made stale by this session and is
corrected there. Every one of them points at
plausible code, which is what makes them expensive: a reader following the old
line 870 of `storage.rs` lands on `let wanted = slash_path(path)` and has no
reason to doubt it.

Proved by putting two of them back and running the check:

```
[drifted-line] cli-surface.md:557 cites cli.rs:2012 for `no_short_flag_is_defined_twice`, which is at :2103
[drifted-line] cli-surface.md:1178 cites schema_gen.rs:734 for `the_committed_schema_matches_what_the_program_writes`, which is at :1068
```

Then corrected again, and the check is silent.

**What it cannot see**, so the next reader does not expect more of it than it
gives: a citation with no symbol named beside it, a symbol the file uses more
than once, a name shorter than ten characters or without an underscore, and a
citation into `reference/`, which is checked for range only as before. It
catches the drift that comes from editing this tree, which is the drift this
repository produces.

### T-196 A magnet that never resolves hangs download with no diagnostic

Source:      cost a measurement while proving [T-194](peers.md), 2026-08-22
Category:    cli-surface
Priority:    P2
Effort:      S
Status:      **done**, 2026-08-22T18:33Z

Problem:     `bit-cli download <magnet>` bounds metadata resolution by
             `--init-timeout` only when a file selection forces it to resolve
             first. Without `--select-file` or `--exclude-file` it calls
             `engine.add` instead, which resolves with no bound at all, and
             the `wait_until_initialized_within` that would have applied
             `--init-timeout` is on the next line and never reached.
Relevance:   The comment beside the bounded branch already says why the bound
             is there: "a magnet that never resolves used to hang the run
             rather than report why". That is still true of the other branch,
             which is the one an ordinary invocation takes.
Approach:    Bound the unbounded branch by the same `--init-timeout`, and
             report the same timeout error with `phase: resolving_metadata`.
             The bounded branch already builds that error, so this is moving
             it rather than writing it.
Acceptance:  A magnet with one peer that cannot serve it exits non-zero within
             `--init-timeout` and names the phase, rather than running until
             something else kills it.

**How it was found.** A magnet download against a local seeder that could not
send its bitfield ran for **ten minutes** and was killed by the harness, not by
`bit-cli`. `--init-timeout` was not passed, but it would have made no
difference: that invocation had no file selection, so it took the branch with
no bound. The defect it was hiding was [T-194](peers.md), and the ten minutes
bought nothing: the seeder had already logged the reason in the first second.

Both halves of the inconsistency are in one function, about fifty lines apart.

**Closed 2026-08-22.** The add is wrapped in the same `--init-timeout` and
builds the same error, with `phase: resolving_metadata`, which is what the
Approach said to do.

**The per-torrent report carries the phase now, and it did not.** The error has
a context map and `TorrentReport` copied none of it, so a run that gave up
resolving a magnet and a run that gave up fetching its pieces both said
`timeout` and nothing else. `torrents[].phase` is a new optional field in
`docs/schema.md`; a run that got past initialising leaves it out.

**Measured**, `scripts/check-init-timeout.ps1`, a magnet whose one peer
completes the handshake and then says nothing, DHT, LSD and trackers off:

| case | before | after |
| --- | --- | --- |
| `selection`, which was already bounded | 4.05 s, `timeout` | 4.05 s, `timeout`, `resolving_metadata` |
| `no_selection`, an ordinary invocation | **10.04 s**, `source_resolution` | **4.04 s**, `timeout`, `resolving_metadata` |

```bash
pwsh -NoProfile -File scripts/check-init-timeout.ps1
```

`selection` is the control: it forces the branch that already had the bound, so
a failure in the other one cannot be blamed on the fixture.

**Where the ten seconds comes from, and why the fixture cannot show ten
minutes.** Before the fix the branch was not unbounded in this fixture, it was
bounded by somebody else's timeout: with one address and one peer, `librqbit`
gives up with "input address stream exhausted" once that peer's read/write
timeout expires. Three fixtures were tried and the first two are worth
recording, because each looks like it measures this and does not:

- **A closed port.** Two seconds, same exhaustion. The connection fails at once.
- **Accept and never write.** Ten seconds, same exhaustion.
- **Handshake and then silence**, with BEP 10's reserved bit set. Ten seconds,
  same exhaustion. Keep-alives on top of it moved the number by nothing.

What made the original run last ten minutes was a tracker and a DHT still
handing out addresses, so nothing ever exhausted. A fixture cannot reach that
without the network. What it does show is the thing the Acceptance asks for: a
4 second `--init-timeout` was ignored and is now the bound, and the phase is
named. `-Slack` defaults to 5 so a run that falls back to the ten second path
fails on the clock as well as on the code.

### T-197 Running upstream's tests filled the patch series with 14,964 patches

Source:      found by running the command `patches/README.md` gives, 2026-08-22
Category:    cli-surface
Priority:    P1
Effort:      S
Status:      **done**, 2026-08-22T14:20Z

Problem:     `scripts/vendor-diff.ps1` and `scripts/vendor-sync.ps1` walked a
             vendored tree with `Get-ChildItem -Recurse -Force` and treated
             every file they found as vendored source. Building that tree
             leaves `target/`, `node_modules/` and
             `crates/librqbit/webui/dist/` in it. `vendor-diff` then hashed
             7.2 GB across 9,894 files and wrote **14,964 patches**, having
             looked hung for seven and a half minutes first.
Relevance:   The command that produces those directories is the one
             `patches/README.md` step 5 tells a session to run, so this is
             reachable by following the instructions exactly. And a 14,964
             patch series is not a series: `vendor-status` would have reported
             the fork healthy while the record of what this repository changed
             was mostly somebody else's build output.
Approach:    Skip a path that a `.gitignore` **inside the vendored tree**
             ignores. That is upstream saying the file is generated, and it is
             derived rather than listed, so a new build directory needs nothing
             remembered. The qualifier matters: `vendor-sync`'s `Get-Swallowed`
             has to keep reporting a file that this repository's **own root**
             `.gitignore` would eat, which is the `.vscode/` case
             `docs/vendoring.md` describes, so filtering on "ignored" flat
             would have deleted a check while fixing a bug.
Acceptance:  `vendor-diff.ps1` writes the patches for the tree's real changes
             and nothing else, with a build directory present.

**Measured, on the tree that had one:**

| | before | after |
| --- | --- | --- |
| patches written | 14,964 | 7 |
| wall clock | 7 m 33 s | 6.1 s |

The seven are the two changes recorded in
[`patches/UPSTREAM.md`](../patches/UPSTREAM.md) and the two lockfiles that
follow the second one.

**The other half of the fix is not to make the mess.**
`patches/README.md` and `docs/vendoring.md` now give the command with
`--target-dir target/vendor-rqbit`, so cargo writes its build output outside a
tree that is supposed to hold nothing but somebody else's source. The scripts
had to be fixed anyway: a session that forgets the flag, or a `cargo build`
that generates the web UI, must not be able to poison the series.

**What this cost before it was found.** Twelve minutes of a session, and the
first sign of it was `vendor-diff.ps1` producing no output at all, which reads
as a hang rather than as work. It was found by checking what the script was
walking, not by waiting longer.

### T-198 An agent that wants a flag name greps for it

Source:      the operator, 2026-08-22, having watched it happen in that session
Category:    cli-surface
Priority:    P1
Effort:      M
Status:      **done**, 2026-08-22T16:00Z

Problem:     Nothing in this repository stated the command surface in a shape a
             program could read. A caller that needed a flag name had three
             options: grep the source, page `--help` one subcommand at a time,
             or guess. The last one costs a run that exits 2, or worse one that
             succeeds having done something else.
Relevance:   Most of the work on this repository is done by an agent, and the
             cost is paid on every session. It was paid in the session this
             entry was filed in.
Approach:    Generate the surface, commit it, and fail the build when it drifts.
             Three shapes, because the readers are different: troff for a
             terminal, Markdown for prose, and a CLIspec 0.3 document for a
             program.
Acceptance:  A flag renamed without regenerating fails `cargo test -p bit-cli`,
             naming the file and the line.

**What is in `man/`**, all generated from the clap definition, all committed:

| file | bytes | for |
| --- | --- | --- |
| `bit-cli.1` | 51,394 | a person at a terminal |
| `bit-cli.md` | 69,860 | prose, one table per command |
| `bit-cli.json` | 137,020 | a program: [CLIspec](https://github.com/rvben/clispec) 0.3 |

28 commands, 20 global options, and all 17 non-zero exit codes with a
`retryable` flag on each. [`docs/man.md`](../docs/man.md) says what each field
carries and why.

**It cannot go stale.** `cargo test -p bit-cli --test man_is_current` renders
all three from the crate being compiled and compares. That is in
`cargo test --workspace`, so it is in the gates and in CI on every platform.
`scripts/check-man.ps1 -Fix` regenerates, and `gates.ps1` runs the script as a
named `man` gate so a session is told what to run rather than reading a test
name out of a failure. The test is what binds: the script compares against
`target/release/bit-cli`, which can be older than the source in front of it.

**Two bugs it caught in its own first output**, both of the kind a reader would
have believed:

- **`--web-seed` was typed `boolean`** while carrying `value_name: URL`.
  `clap::Arg::get_num_args` is empty until the command is built, so every flag
  that takes a value was reported as one that does not. Read from the action
  now, and the command is built before it is walked.
- **`create --version` disappeared.** Filtering clap's generated `--version` by
  argument id also deleted the metainfo version flag, which takes `v1`, `v2` or
  `hybrid`. Filtered by action now.

Both are in a generated file that nothing was checking, which is the argument
for the test rather than for the generator.

**The one thing not generated** is `effects`, CLIspec's word for whether a
command is `read_only`, `idempotent` or `non_idempotent`, because nothing in a
clap definition says whether a command writes. It is a table in
`crates/bit-cli/src/cmd/spec.rs` and a subcommand missing from it fails
`every_subcommand_is_classified`, rather than shipping an empty `effects` that
a reader would take to mean "no side effects". Eleven nested subcommands were
caught by exactly that on the first run.

The Markdown is rendered from the CLIspec document rather than from clap a
second time, so those two cannot disagree about a flag.

[RULES.md](RULES.md) section 4a carries the rule this exists to serve: read
`man/bit-cli.json` before typing a flag.

### T-199 The CI supply chain was unwatched and one action was abandoned

Source:      the operator, 2026-08-22
Category:    cli-surface
Priority:    P2
Effort:      S
Status:      **done**, 2026-08-22T16:00Z

Problem:     Nothing watched dependency or action versions, and
             `ilammy/setup-nasm@v1.5.2` had gone unmaintained: it is that
             project's newest release, it still runs on node20, and GitHub
             warns about the deprecation on every job. It was used in five
             places across two workflows.
Relevance:   A node20 action stops working when GitHub retires the runtime, and
             the first sign would be every Windows job failing at once. The
             warning had been there long enough to be pinned with a comment
             saying to revisit it.
Approach:    Replace the action with a script in this repository, and add
             `dependabot.yml` so the next one is noticed by a bot rather than
             by a person reading a warning.
Acceptance:  The script installs NASM and refuses an archive whose checksum
             does not match.

**`scripts/setup-nasm.ps1`** does what the action did, in about thirty lines,
and does one thing the action never did: it verifies the download against a
pinned SHA-256. Both halves were run rather than reasoned about:

```
$ pwsh -NoProfile -File scripts/setup-nasm.ps1 -Force
setup-nasm: sha256 ok
setup-nasm: NASM version 2.16.03 compiled on Apr 17 2024

$ pwsh -NoProfile -File scripts/setup-nasm.ps1 -Force -Sha256 0000...
setup-nasm: checksum mismatch for nasm-2.16.03-win64.zip
  expected 0000000000000000000000000000000000000000000000000000000000000000
  got      3ee4782247bcb874378d02f7eab4e294a84d3d15f3f6ee2de2f47a46aa7226e6
exit=2
```

It is a no-op when `nasm` is already on PATH, and on a runner it appends to
`GITHUB_PATH` so later steps see it. NASM is needed because `aws-lc-sys`
assembles its own primitives, and `cargo tree -i aws-lc-rs` says it arrives
under **two** parents: `rustls`, and `librqbit-sha1-wrapper`, which is the
SHA-1 backend every piece hash goes through. Dropping the TLS one would not
remove the need.

**`.github/dependabot.yml`** covers cargo and github-actions, weekly, grouped.
Grouped because a pull request per crate is sixteen CI runs a week for a
lockfile bump nobody reads, and the workflow's concurrency group cancels runs
in flight, so the noise costs real coverage. Two things are deliberately
excluded and the file says why: **`vendor/` is not watched**, because a bot
rewriting a vendored manifest without moving the recorded base is the one state
`patches/README.md` says must never happen, and `scripts/upstream-scan.ps1` is
how those trees are watched instead; and **`librqbit*` is ignored**, because
`[patch.crates-io]` means a registry bump for it cannot reach the build.

**One consequence worth knowing.** `scripts/setup-nasm.ps1` is now invoked by
the workflows, so `git-sync -NoCi` refuses to treat a commit touching it as
documentation-only. That is derived rather than listed: the script reads
`.github/workflows/` to work out which scripts CI depends on.

### T-213 seed cannot serve a payload renamed by --index-out

Source:      found closing [T-116](#t-116--o--index-out-cannot-rename-a-file)
Category:    cli
Priority:    P3
Effort:      S
Status:      open

Problem:     `download -O 0=renamed.bin` writes the first file to
             `renamed.bin`, and `bit-cli seed` against that directory looks for
             it at the path the torrent names. `seed` builds its `AddOptions`
             at `crates/bit-cli/src/cmd/seed.rs:260` with no `index_out`, so
             the storage plan it hands the session is the unmodified one and
             the renamed file is missing as far as the seeder is concerned.
Relevance:   Downloading a payload and then seeding it back is the ordinary
             thing to do with one, and `-O` is the flag that quietly breaks it.
             P3 rather than higher because it needs the caller to have used
             `-O` in the first place, and because the failure is loud: the
             hash check finds the file missing and says so.
Approach:    The same one `verify` took when T-116 closed: an `-O` flag on
             `SeedArgs`, parsed with `crate::selection::index_out` against the
             file count the metainfo already gives, and passed into
             `AddOptions::index_out`, which the engine already carries. The
             work is the flag and the test, because the machinery underneath
             is what T-116 built.

             Worth deciding at the same time: whether `bit-cli files` should
             report the on-disk path a given `-O` would produce, so a caller
             can ask where a file will land before fetching it.
Acceptance:  A payload downloaded with `download -O 0=renamed.bin` is served by
             `seed <TORRENT> --data <DIR> -O 0=renamed.bin` with the hash check
             finding every piece, and without `-O` the same command reports the
             file missing. Both in one test, because the second is what makes
             the first mean anything.

### T-214 seed runs no hooks

Source:      the Problem's third clause in
             [T-115](#t-115-hooks-do-not-fire-for-every-documented-trigger),
             which its Acceptance did not cover
Category:    cli
Priority:    P3
Effort:      S
Status:      open

Problem:     `bit-cli seed` has no `--on-*` flag at all. `--on-complete`,
             `--on-error` and `--on-piece-verified` are on `download` only, so
             a long-lived seeder can tell an external system nothing about what
             it is doing. This is a missing feature rather than a flag that
             does nothing: there is no flag to be inert.
Relevance:   A seeder is the shape that runs for days, which is the shape most
             likely to want a hook. P3 because `--jsonl` already carries every
             event a hook would fire on, so nothing is unreachable, only
             inconvenient for a caller that wants a command rather than a
             stream reader.
Approach:    `crates/bit-cli/src/hooks.rs` is the machinery and it is not
             `download`-specific: `finished_vars` takes a struct of facts
             rather than a `TorrentReport`, and `PieceHook` takes a command.
             What a seeding run means by each trigger is the part to decide
             first, and it is not the same as a download's:

             - **`--on-complete`** has no obvious moment. A seeder does not
                complete. The candidates are "the hash check passed and it is
                now serving", which is the useful one, and "the run ended",
                which is what `--on-error`'s absence would mean.
             - **`--on-error`** is the run failing to start or dying, which is
                well defined.
             - **`--on-piece-verified`** happens once during the hash check on
                add and never again, so it would fire in a burst at the start
                and then be silent. `--on-peer-connected` is what a seeder
                would actually want, and it is a new trigger rather than a
                port of an existing one.

             Decide those before writing any of it, and add whatever variables
             a seeding run needs to `hooks::VARIABLES`, which is the one list
             both `docs/hooks.md` and the tests are held to.
Acceptance:  `bit-cli seed <TORRENT> --data <DIR> --on-complete <CMD>` runs the
             command once, when the payload has been checked and the listener
             is up, with `BIT_CLI_INFO_HASH` set. `docs/hooks.md` says which
             trigger means what on `seed`, and
             `every_hook_variable_is_documented` still passes.
