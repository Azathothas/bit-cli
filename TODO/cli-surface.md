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

### T-144 The MSRV job fails: the tree needs a newer rustc than it claims

Source:      CI run 32386960166, 2026-08-20
Category:    ci
Priority:    P1
Effort:      S
Status:      open

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

Closing evidence is a green `MSRV` job, recorded below.

### T-145 The macOS test job fails to link

Source:      CI run 32386960166, 2026-08-20
Category:    ci
Priority:    P2
Effort:      M
Status:      open

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

Closing evidence is a green `Test (macos-latest)`, recorded below.


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
