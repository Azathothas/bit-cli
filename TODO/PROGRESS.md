# Progress

**Read this first.** It is the only thing the kickoff prompt tells a session to
read, so everything that changes from session to session is here: the baseline,
what the last session did, and the work order. The prompt carries none of it, by
[RULES.md](RULES.md) section 3.

It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
Every entry, one line each: [INDEX.md](INDEX.md).

> **The shape this file must keep**, from [RULES.md](RULES.md) section 2 step 2:
> the state line with the session's start instant in ISO 8601 UTC, the measured
> baseline with the CI run named by id, the entry counts, what the session did,
> what is in progress, **Start here next session** as an ordered list with entry
> ids and corpus sources, and open questions for the operator.
> `scripts/session-report.ps1` prints the numbers; do not count them by hand.
>
> `scripts/check-todo.ps1` checks most of that shape now, and `scripts/gates.ps1`
> runs it, so a missing section or a stale count fails a gate rather than a
> review. [RULES.md](RULES.md) section 5, "The record".

---

## Before typing a `bit-cli` flag, read `man/bit-cli.json`

`man/` holds the whole command surface, generated and committed: `bit-cli.1` for
a terminal, `bit-cli.md` for reading, and **`bit-cli.json`, a CLIspec 0.3
document, for a program**. Every command, every flag, the values it accepts, its
default, and every exit code with whether a retry could succeed.

It cannot go stale: `cargo test -p bit-cli --test man_is_current` fails until it
is regenerated with `pwsh -NoProfile -File scripts/check-man.ps1 -Fix`.
[`docs/man.md`](../docs/man.md) says what each field carries.

## Two things are settled and are not to be raised again

**Nothing in `patches/` is ever offered upstream, and this repository is the
only one an agent may write to.** [RULES.md](RULES.md) section 6 carries the
first and section 6a the second. `patches/UPSTREAM.md`'s `Upstream:` field
answers "could a release retire this patch on its own?" and nothing else.

**The six hour soak is run by the operator, in a foreground terminal.** No agent
session lasts six hours, and a session ending kills the process it started. The
command is under "Start here next session"; a session's job is to read the CSV
the operator's run leaves behind, not to start one.

## `patches/TASKS.md` is finished, and the fork is maintenance now

Twelve of its thirteen rows are done and the thirteenth is not waiting on a
seam any more. So the work order is derived from [INDEX.md](INDEX.md)'s four
questions, not from `patches/TASKS.md`, and the vendored trees are what
`patches/README.md` describes: run `scripts/upstream-scan.ps1` on a version
bump, reconcile with `scripts/vendor-sync.ps1`, keep `UPSTREAM.md` true.

## State

- **Last session:** 2026-08-23T16:18:55Z, unattended, and running now. The
  duration is not restated here: `scripts/session-report.ps1` derives it from
  the instant above, and a duration written down twice is a number two
  documents disagree about.
- **Tests:** 1,271 passing, 0 failing. 1,228 at the start. Plus **149** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run. The vendored count is unchanged by this session's patch, which
  was the point of running them.
- **Gates:** clean, on rustc 1.98.0.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green on all **nineteen** jobs at run **32650336109**, against
  commit `0ad5792`, which is the last commit of the session. Six runs this
  session and **two were red, both fixed and both entered**:
  **32645146193** on `Test (windows-latest)`, which is [T-223](bench.md), and
  **32649574641** on `Create round trip (windows-latest)`, which is
  [T-225](create-seed.md). One documentation commit follows this line carrying
  a CI skip marker, so it starts no run.
- **Soak, finished:** the six hour run completed at 2026-08-23T15:01:32Z, 681
  samples, 1,360 leech cycles, none failed, every named ceiling held. Its CSV
  and JSON are committed. What its RSS slope actually says is
  [T-224](memory.md).
- **Soak, running:** the operator started a second six hour run at
  2026-08-23T15:47:16Z, from a release build of `d3bc6a5`, and it was 23
  samples in when this file was written. It is `bench/soak-20260823T154716064Z`
  and **it is the reproduction [T-224](memory.md) asks for**: whether the
  11.7 MiB step at `t+1.161h` happens again. That is answered inside the first
  ninety minutes, so the answer may already be in the CSV when the next session
  starts. Read it before starting a third.

```bash
pwsh -NoProfile -Command "Get-Content bench/soak-20260823T154716064Z.json | ConvertFrom-Json | Select-Object generated_at, complete, elapsed_hours, samples"
```

```bash
gh run list --limit 1
```

- **Entries:** 175 items. 24 open, 1 partial, 0 blocked, 140 done, 10 deferred
  to Phase C. 140 of 165 workable done, 25 left.
- **Tree:** 96 Rust files, 56,704 lines of code, 14,358 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What this session is doing

Written before the work, by [RULES.md](RULES.md) section 1 step 4. It is
replaced by "What the last session did" when the session ends.

1. Baseline re-measured: gates clean at 1,271 tests on rustc 1.98.0,
   `check-todo.ps1` agrees, and CI run **32650336109** is green against
   `0ad5792`. The commit after it, `e756289`, carries a CI skip marker.
2. **The soak that was running is left alone.** It was 61 samples and 0.53
   hours in at 16:18:59Z. It crosses `t+1.161h` at about 16:57Z, which is when
   [T-224](memory.md)'s question can be answered from its CSV.
3. **[T-103](bep-coverage.md)**, the effort S entry item 3 of the last order
   names first. Measure before building: what `crates/bit-cli-core/src/torrent/
   metainfo.rs` does with a non-UTF-8 `path`, and whether `name.utf-8` and
   `path.utf-8` are read at all.
4. **[T-224](memory.md)**'s cheap half, `scripts/soak.ps1` reporting a step
   rather than one linear fit, which is also what reads item 2's run.
5. The rest of the effort S list: [T-041](memory.md), [T-165](peers.md),
   [T-033](performance.md), [T-008](webseed.md).

## What the last session did

**Six entries closed, four filed, and one corrected, and every one of them was
found or shaped by running something rather than by reading it.** The operator
ruled on the open question from the previous session, "build rather than
delete", so the session opened with [T-219](cli-surface.md), item 2 of the
previous work order; T-222 and T-223 came out of it, and the effort S list
followed.

### [T-219](cli-surface.md), P1, effort M, done

`--trace` documented eleven subsystems and ten of them raised a `tracing`
target nothing wrote to. All eleven emit now.

**The premise held and the fix was not the one the entry described.** The entry
reads as ten subsystems' worth of instrumentation to write. One run with `-vvv`
and `--log-format json`, counting the `target` field, says otherwise:
**10,986 records over nineteen targets**, and nine of the ten already had their
facts on a target `--trace` did not name. `librqbit::peer_connection` 4,108,
`librqbit::torrent_state::live` 2,154, `librqbit::file_ops` 2,114,
`librqbit_dht::dht` 221, `bit_cli::http` 32.

So a subsystem carries **the targets it raises** now rather than one derived
from its spelling. `SUBSYSTEMS` is a struct, `filter_directive` emits one
directive per target and dedupes on the target, and each name raises
`bit_cli::<name>` where this repository's own code writes plus the vendored
target that carries the same fact.

**Ten subsystems given somewhere to write**, `http` being the one that already
had one: `SafeStorage`'s read, write, flush and allocate; `RateLimiter::take`;
both retry ladders and `SourceStats::record_error`; `Resolved::trace`;
`Client::announce_on` and `scrape`; the web seed bridge's handshake, messages
and served blocks; `InOrder::advance`; and `Engine`, once per session, for
whether there is a DHT at all.

**Thirteen vendored trace calls retargeted**, and it is not cosmetic. A
`tracing` target defaults to the module path and the modules do not divide the
way the subsystems do: `peer_connection` holds the handshake and every wire message, so
`--trace handshake` would have printed 266 records where 2 were asked for.
`patches/UPSTREAM.md` has the section. Upstream's own tests were run: 149
passing, unchanged.

**Measured, on one 2 MiB fixture, one run per name.** The ten names the entry
was filed about write **743** lines of stderr where they wrote **0**. An
untraced run still writes none.

```bash
pwsh -NoProfile -File scripts/check-trace.ps1 -Json bench/trace.json
```

**Run against the defect.** With `bit_cli::disk` renamed at all four storage
call sites the `disk` case fails, naming the target it expected and the one it
found. That is exactly the state the whole surface was in.

The acceptance is `crates/bit-cli/tests/trace_subsystems.rs`, fifteen cases,
driving the **binary** rather than `run`: the subscriber is process-global and
`logging::install` is best-effort by design, so an in-process assertion would
be reading whichever test won the race to install one.
[`docs/trace.md`](../docs/trace.md) is what a caller reads.

### [T-222](cli-surface.md), P1, effort M, filed and done

**A config file reached `config show` and nothing else**, and it is the fifth
flag-does-nothing entry after T-181, T-183, T-185 and T-219. `--config` and
`--no-config` were global flags with two readers in the workspace, both inside
`cmd/config.rs`'s `resolve`, whose only caller was `config show`. `README.md`
documented the whole six-layer chain as the tool's configuration and twenty-two
settings had a default, a description, and no reader.

**The Approach it was filed with named the wrong seam.** It proposed reading
`clap`'s `ArgMatches::value_source` and overwriting each field, which needs a
branch per setting across eight structs. Setting `Arg::default_value` and
parsing the tree a second time moves precedence back into `clap`, which already
knows a supplied value beats a default. Nothing in this repository decides
precedence now; it falls out. `crates/bit-cli/src/config_defaults.rs` is the
module and the correction is written under the entry.

**Three things fell out and none could be separated from it.** `--config`
naming a missing file is the same exit code on every command, where it was
exit 8 on one and exit 0 in silence on fifteen. `user_config_path` takes the
environment instead of reading the process, or a test would resolve against
whatever config file the machine happens to have. And a `BIT_CLI_*` variable
this program sets itself is no longer refused as a typo.

**That last one was found by running.** `apply_env` refuses an unknown
`BIT_CLI_*` name, which is right, and it used to run on one command. Making it
run on every command made **every run under `cargo test` fail** on
`BIT_CLI_TARGET`, which this repository's own build script sets. The larger
case the suite did not reach is the twenty variables a hook receives: a hook
whose command is `bit-cli` would have had the child refuse its parent's
variables. The reserved list is derived from `hooks::VARIABLES` rather than
written twice.

Seven acceptance cases, every one driving a command that is **not**
`config show`, and `download_directory` is the setting under test in most of
them because its effect is a file on the disk rather than a number in a report.

### [T-223](bench.md), P1, effort S, filed and done

**The push that closed T-219 turned `Test (windows-latest)` red**, on
`a_leech_measures_the_transfer_the_hashing_and_the_disk`, at 1,976 bytes of a
3,000 byte payload. That is one 1,024 byte block, and it is the **third**
failure of this test for a **third** distinct reason.

It is [T-149](bench.md)'s defect in the counter the report is named for, and
T-149 is what left it behind: that entry added a final read of the storage
counters after the loop and did not add one for the peer counters. `drive_leech`
read the peer counters at the top of its body and the completion flag near the
bottom, so a block landing between the two was written, hashed, counted as disk
work and counted as transfer nowhere.

**The fix is an ordering rather than a tolerance.** The completion flag is read
**before** the counters, so `finished` true means every read below it happened
after the last byte and there is no window to fall into. The transfer counters
are also read once more after the loop, which covers the deadline and interrupt
paths the ordering does not.

**The new assertion is the useful part**: `summary.bytes >= disk.write_bytes`.
Every byte on the disk came off a source, so the transfer total cannot be under
the write total on a run that started from nothing, and unlike the equality
beside it nothing about scheduling can lower the left side.

**Nothing this session changed caused it.** Every trace added is inert when the
subsystem is off: `tracing` does not evaluate a record's fields unless
something is listening. The commit was simply the first run since the last
green one.

### Item 2 of the order, the effort S list: three taken, and two carried something untrue

**[T-187](metainfo.md), P3, done.** Non-canonical integers stay refused, which
is the outcome the entry said it most likely had. Nothing was re-fetched to
look for an instance: [RULES.md](RULES.md) section 7 says not to, and what
`RESEARCH.md` records is an adversarial case in an audit rather than a torrent
anybody has.

**What the examination found is that the reason in the code was wrong.**
`bencode.rs` justified the rule with "would make the info hash ambiguous". It
would not: `decode_torrent` records the byte span of `info` and `Metainfo`
hashes those bytes, so a leading zero inside `info` moves the hash exactly as
much as an unsorted key does, which is not at all. That is
[T-172](metainfo.md)'s own argument, applied where nobody had applied it. The
comment carries the two reasons that do hold, both about evidence rather than
correctness, and a test pins the decision by asserting the refusal **and** the
recorded span on the same fixture written canonically.

**[T-173](metainfo.md), P3, open, premise disproved and corrected under it.**
The entry says nothing in `bit-cli` defines a zero-length path component and
the planner has no test. Both halves are wrong.

The component is dropped, always was, and three shapes were measured landing
as if it were not there. And the case the entry is actually about, `["", "foo"]`
beside `["foo"]`, never reaches the planner: `librqbit_core`'s `validate`
refuses the whole torrent with `duplicate filenames in torrent`.

**That refusal stays, on T-187's own argument**, and it would have been
inconsistent to do otherwise in the same session: no torrent in evidence
carries one, and a validation relaxed with no instance behind it is tolerance
nobody asked for. [T-072](windows.md)'s precedent does not carry over, because
a case collision is a filesystem fact and this is a metainfo fact.

**What is left is a seam and it is smaller than the entry.** The drop is not
reported, because `SafeStorage` plans from the session's `file_infos`, whose
`relative_filename` is a `PathBuf` that already lost the empty component. The
planner cannot report what it never saw. `Reason::DroppedComponent` is built
and fires on the one path where the raw components do reach it, `--index-out`.
The entry names the two ways to close the rest and why neither is worth it for
a P3 whose only cost is a missing `reasons` entry on a path that is correct.

### [T-176](create-seed.md), P2, done, and the one entry whose Approach also survived

All three claims checked out against the tree before anything was written, and
so did the Approach, which is what separates it from the other four: T-219's
and T-222's premises held and their Approaches did not, and T-173's premise did
not.
`piece-count` fired above 100,000 and nothing below it, so the band from 65,536
to 100,000 passed every check and produced a torrent µTorrent cannot open.
`piece_length::validate` refused only zero, so `--piece-length 64MiB` was
accepted in silence. And the collision check keyed one set on the lower-cased
path, so two identical paths fired `case-collision` with a message about a
casing difference that was not there.

Ten lints to thirteen. `piece-count-unopenable` and `piece-length-too-large`
are separate from the opinions beside them so the two clear independently, and
the piece-length ceiling is read from `piece_length::MAX` rather than written
again. `duplicate-path` splits off, and `case-collision` keeps its message.

Five tests, and two of them assert what does **not** fire, which is the half
the old code would have passed.

### The six hour soak finished, and its headline number is wrong about the mechanism

**The operator's run completed its full window for the first time**: 681
samples over 359 minutes, 1,360 leech cycles, none failed. Every named ceiling
held. `tcp_close_wait` was **zero at every one of the 681 samples**, which is
[T-020](peers.md) staying fixed over the longest window it has had, and handles
and threads are flat at r squared 0.00.

**`rss_bytes` reports 3.708 MiB/h at r squared 0.717 against a ceiling of 4,
and that fit spans a step.** At `t+1.161h` resident memory goes from 15.8 MiB
to 27.5 MiB in one eight second interval and never returns below 27.5. Threads
and handles do not step with it. Fitted either side, the slope is 1.020 MiB/h
before and 1.690 after; from `t+2h` it is 1.371 at r squared 0.418.

**And what is left after the step is a sawtooth.** From `t+2h` the series has
mean 33.8 MiB and standard deviation 2.4, with 49 samples rising more than
3 MiB and 52 falling more than 3. A series that gives back what it takes 52
times is a high-water mark, not a leak.

That is [T-224](memory.md), filed, P2. It carries the numbers, the argument
that a ceiling a run passes or fails depending on how long it ran is not a
ceiling, and a cheap first move: `soak.ps1` reports one linear fit per series
and has no way to say there is a step in it.

### [T-225](create-seed.md), P1, filed and done, and it is the second red job of the session

**The push carrying the record turned `Create round trip (windows-latest)`
red**, on a commit that changed no source the interop path touches. The message
was `Get-FileHash: the process cannot access the file`, and the timestamps say
what it really was: `seeder announced` at 15:49:04 and the failure at 15:52:04,
which is exactly the `-TimeoutSeconds 180` CI passes.

So the leech ran out of budget, `Invoke-Recorded` force-killed it, and the
script hashed the output directory while `aria2c` still held the files.
`Stop-Process -Force` returns before Windows has finished tearing a process
down.

**A slow runner became a red job with a message about the wrong thing**, which
is the worst shape a failure can have: the next session debugs `Get-FileHash`
rather than reading "the download did not finish in 180 seconds". Two changes,
both waiting on the condition: the runner waits for the killed process to
actually exit, and the hash retries a sharing violation until the file opens or
30 seconds pass and then says so with the path.

It is the seventh entry of the family [RULES.md](RULES.md) section 5 names, and
the first in a `scripts/` acceptance rather than a `cargo` test. **What is not
answered is why that leech needed more than 180 seconds**, and the entry says
so: the same case takes 2,143 ms locally. A genuine timeout will now report
itself as one.

## In progress

Nothing is half-written. All five entries that closed are closed in
[INDEX.md](INDEX.md) with their acceptance runs recorded, and T-219's evidence
is committed at `bench/trace-subsystems-20260823T140418847Z.json`.

- **[T-173](metainfo.md)** is open with its premise disproved, the correction
  written under it, and the seam named with a file and a line. What is left is
  the report of a dropped path component, not the behaviour.
- **[T-212](memory.md)** is open and unchanged, still waiting on a fixture of
  stalling peers.
- **[T-102](bep-coverage.md)** is open and **[T-164](peers.md)** is partial,
  the only partial left.

## Start here next session

**The shape of the work order is the operator's, from four sessions ago.** Not
priority first. Clear as many small entries as possible, so the open count
comes down, and then take the bigger ones a **category at a time**.

**Nothing departs from that shape this time.** The last two orders each put one
measured P1 above it, and both of those are closed. Twenty-five entries are
open and the only P1 left is [T-081](create-seed.md), BEP 52, which is effort L
and belongs in item 6's category pass rather than at the top of a list whose
purpose is bringing the open count down. So the shape and the priority agree:
item 3 is the effort S list.

**The soak is item 2 and it goes first**, on the operator's instruction: it
runs for hours, so it starts before anything else and the session works while
it runs. Reading the one that already finished is item 4.

The counts are derived from the rows rather than from memory:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Re-measure the baseline rather than trusting the one above**, which is
   what [RULES.md](RULES.md) section 1 step 5 asks for. Read the run this
   session's push started: the CI line above names the previous session's run,
   because this file was written before the push.

```bash
gh run list --limit 1
```

2. **The soak, and it goes before anything else on the operator's
   instruction.** It runs for hours, so it starts first and the session works
   while it runs. The operator runs it and will say when it has finished.

   **One is running right now**, started 2026-08-23T15:47:16Z from a release
   build of `d3bc6a5`. **Check whether it is still going before doing anything
   else**, and if it is, leave it alone, read what it has so far, and skip to
   item 3. A second soak started beside it shares the tracker and neither
   measures anything.

```bash
pwsh -NoProfile -Command "Get-Content bench/soak-20260823T154716064Z.json | ConvertFrom-Json | Select-Object generated_at, complete, elapsed_hours, samples"
```

   **What follows is for when there is no soak running and a fresh one is
   wanted.**

   **First, make sure no soak is already running.** A previous one leaves
   `bit-cli` and `loopback-tracker` processes behind, running from a copy under
   `.tmp/`, which is why `gates.ps1` reports leaving them alone rather than
   killing them. Kill them before starting a new one, or the two runs share a
   tracker and neither measures anything:

```bash
pwsh -NoProfile -Command "Get-Process bit-cli,loopback-tracker,loopback-churn -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\.tmp\*' } | Stop-Process -Force"
```

   **Then build a fresh binary from the tree the session is about to work on**,
   because a soak measures whatever it was handed and a stale binary measures
   the last session:

```bash
cargo build --release --bins --examples
```

   **Then print this in chat for the operator to run**, in a dedicated
   foreground terminal from the repository root. Six hours and the same three
   ceilings as the run that is committed, so the two are comparable:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

   **Then wait for it to start, record it, and move on.** It writes
   `bench/soak-<stamp>.csv` and rewrites `bench/soak-<stamp>.json` after every
   sample, so a file appearing is the signal that it is running. Poll for one
   newer than the build, write its stamp into this file under the state
   section, and then go to item 3. Do **not** start it from inside a session:
   a session ending kills the process it started, which is why the operator
   runs it.

```bash
pwsh -NoProfile -Command "Get-ChildItem bench/soak-*.json | Sort-Object LastWriteTime | Select-Object -Last 1 | Get-Content | ConvertFrom-Json | Select-Object generated_at, complete, elapsed_hours, samples"
```

   **What this run is for** is [T-224](memory.md)'s first question: does the
   11.7 MiB step at `t+1.161h` reproduce? It is answered inside the first
   ninety minutes, so the answer is available long before the window closes.

3. **The effort S entries**, five left of the eight and all P2 or P3:
   [T-041](memory.md), [T-165](peers.md), [T-033](performance.md),
   [T-008](webseed.md), [T-103](bep-coverage.md). [T-176](create-seed.md) and
   [T-187](metainfo.md) closed, and [T-173](metainfo.md) is open with its
   premise disproved and the seam named, which is a smaller thing than the
   entry describes.

   **[T-103](bep-coverage.md) is the one to take first** of the five: it is the
   only one the ordering in [INDEX.md](INDEX.md) calls out as costing reach
   today, because preferring the `.utf-8` spelling is a read-side rule rather
   than the Shift-JIS work the entry leads with.

   **Check each against the tree before building.** That rule has now paid
   four sessions running and this session is the sharpest case yet: T-219's
   premise was true and its Approach was still wrong, because nobody had
   counted what a run already emits, and T-222's filed Approach named a seam a
   better one replaced. One command answered both.

4. **[T-224](memory.md), P2, effort M.** The six hour soak that just finished
   **is committed**, so
   this is reading it rather than running it. Its headline slope, 3.708 MiB/h
   against a ceiling of 4, is a line fitted across an 11.7 MiB step at
   `t+1.161h`; either side of the step it is 1.0 to 1.7 MiB/h. The entry has
   the numbers and the argument.

   **The cheap half first**, and it is worth doing even if the step is never
   found: `soak.ps1` reports one linear fit per series and cannot say there is
   a discontinuity in it, so a reader has no way to know the number is a fit
   across a step. A largest-single-interval-change column beside each slope
   would have made this entry unnecessary to write by hand.

   The run is committed, so nothing has to be re-run to start:

```bash
pwsh -NoProfile -Command "Get-Content bench/soak-20260823T090132499Z.json | ConvertFrom-Json | Select-Object generated_at, complete, elapsed_hours, samples"
```

   **Another six hour run is not needed to make progress.** The entry's first
   question is whether the step reproduces, and a **two hour** run at the same
   leech rate crosses `t+1.161h` with room to spare. That is the operator's to
   start, in a foreground terminal, for the same reason the six hour one was:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 120 -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

5. **[T-212](memory.md)**, whenever a fixture of stalling peers is being built
   anyway.
6. **Then, a category at a time.** `bep-coverage.md` has the most left and the
   most shared machinery. After it, `dht.md`.

**Two corpus sources the list above may want**, both already on this machine
and neither needing a fetch: `reference/RESEARCH.md` sections C and D, and
`contrib/rqbit/` in <https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, whose
`0012` and `0014` are read and not taken. **Both are reads.** Nothing is
opened, filed or commented on either, by [RULES.md](RULES.md) section 6a.

## Open questions for the operator

**Two decisions this session made on its own and would take a ruling on.**
Neither blocks anything: both are closed in the record with the reasoning, and
either can be reopened by saying so.

**One. Should the vendored tree be patched so a dropped path component can be
reported?** [T-173](metainfo.md) is open on exactly this. `path: ["", "foo"]`
lands correctly at `foo`, but the drop is invisible: `SafeStorage` plans from
the session's `file_infos`, whose `relative_filename` is a `PathBuf` that lost
the empty component before this repository saw it. Closing it needs either a
patch to `librqbit_core` so `FileDetails` carries the raw components, or
`SafeStorage` planning from this repository's own metainfo parse.

**The recommendation is to leave it**, which is what this session did. It is a
P3 whose only cost is a missing `reasons` entry on a path that is already
correct, and the second option is much larger than it sounds because the
session's file list is also what the piece-to-file mapping is keyed on. The
entry carries both routes if that is wrong.

**Two. Should `path: ["", "foo"]` beside `path: ["foo"]` be renamed rather
than refused?** Today the whole torrent is refused, by `librqbit_core`'s
`validate`, before this repository's planner runs. [T-072](windows.md)'s
precedent says two entries distinct in the metainfo should both land.

**The recommendation is to keep the refusal**, which is what this session did,
and the reason is consistency with a decision made the same afternoon:
[T-187](metainfo.md) kept a strict rule because no instance is in evidence, and
relaxing this one would be the same argument answered the other way. The
citation behind it is a parser's issue tracker, not a torrent anybody has.
`an_entry_that_collapses_onto_another_is_refused_whole` pins the behaviour, so
relaxing it later is a decision made against a failing test rather than a
change nobody notices.

## Two behaviour changes worth the operator's eye

Not decisions. Both are from [T-222](cli-surface.md) and both change what an
existing command does.

`bit-cli.toml`, the user config file at the platform config directory, and
every `BIT_CLI_*` variable now change what a run does. Until this session they
changed what `bit-cli config show` printed and nothing else, so a file that has
been sitting on this machine has never had an effect and will have one on the
next run. `bit-cli config show` says what is in force and where each value came
from, `--trace config` shows the whole resolution on any command, and
`--no-config` turns the files off.

`--config` naming a file that is not there is an error on every command now.
It was exit 8 on `config show` and exit 0, in silence, on the other fifteen, so
a script that passed a stale path and did not notice will start noticing.

**The question this file carried last session is answered.** Whether to build
`--trace` out or delete the ten names: the operator ruled build, it is done,
and [T-219](cli-surface.md) is closed rather than left open for a residue.
