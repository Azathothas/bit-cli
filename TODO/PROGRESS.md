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

- **Last session:** 2026-08-23T13:20:47Z, unattended, and it was ended on the
  operator's word. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.
- **Tests:** 1,263 passing, 0 failing. 1,228 at the start. Plus **149** in the
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

- **CI:** green on all **nineteen** jobs at run **32638490147**, against commit
  `9d5eb41`, which is the last commit of the **previous** session. This
  session's own run is newer and is named in the next revision of this file;
  read the current one rather than trusting either.

```bash
gh run list --limit 1
```

- **Entries:** 172 items. 26 open, 1 partial, 0 blocked, 135 done, 10 deferred
  to Phase C. 135 of 162 workable done, 27 left.
- **Tree:** 95 Rust files, 56,006 lines of code, 14,009 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **28 patches**
  across twenty sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Three entries closed and two filed, and every one of the three was found or
shaped by running something rather than by reading it.** The operator ruled on
the open question from the previous session, "build rather than delete", so the
session opened with [T-219](cli-surface.md), item 2 of the previous work order,
and the other two came out of it.

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

**Eleven emission points written**, in `SafeStorage`'s read, write, flush and
allocate; `RateLimiter::take`; both retry ladders and
`SourceStats::record_error`; `Resolved::trace`; `Client::announce_on` and
`scrape`; the web seed bridge's handshake, messages and served blocks;
`InOrder::advance`; and `Engine`, once per session, for whether there is a DHT
at all.

**Ten vendored trace calls retargeted**, and it is not cosmetic. A `tracing`
target defaults to the module path and the modules do not divide the way the
subsystems do: `peer_connection` holds the handshake and every wire message, so
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

## In progress

Nothing is half-written. All three entries are closed in [INDEX.md](INDEX.md)
with their acceptance runs recorded, and T-219's evidence is committed at
`bench/trace-subsystems-20260823T140418847Z.json`.

- **[T-212](memory.md)** is open and unchanged, still waiting on a fixture of
  stalling peers.
- **[T-102](bep-coverage.md)** is open and **[T-164](peers.md)** is partial,
  the only partial left.

## Start here next session

**The shape of the work order is the operator's, from four sessions ago.** Not
priority first. Clear as many small entries as possible, so the open count
comes down, and then take the bigger ones a **category at a time**.

**Nothing departs from that shape this time.** The last two orders each put one
measured P1 above it, and both of those are closed. Twenty-six entries are
open and one is P1, so the shape and the priority now agree: item 2 is the
effort S list.

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

2. **The effort S entries**, eight of them, and this is where the open count
   comes down: [T-176](create-seed.md), [T-173](metainfo.md),
   [T-187](metainfo.md), [T-041](memory.md), [T-165](peers.md),
   [T-033](performance.md), [T-008](webseed.md), [T-103](bep-coverage.md).

   **Check each against the tree before building.** That rule has now paid
   four sessions running and this session is the sharpest case yet: T-219's
   premise was true and its Approach was still wrong, because nobody had
   counted what a run already emits, and T-222's filed Approach named a seam a
   better one replaced. One command answered both.

3. **The six hour soak, and it is the operator's to run.** No agent session
   lasts six hours and a session ending kills the process it started. In a
   dedicated foreground terminal, from the repository root:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

   **How a later session reads it.** The run writes `bench/soak-<stamp>.csv`
   one row per sample and rewrites `bench/soak-<stamp>.json` after every
   sample, so a run still going has both. `"complete": true` in the JSON is the
   only thing that says the window finished.

```bash
pwsh -NoProfile -Command "Get-ChildItem bench/soak-*.json | Sort-Object LastWriteTime | Select-Object -Last 1 | Get-Content | ConvertFrom-Json | Select-Object generated_at, complete, elapsed_hours, samples"
```

   The last run reached 1.32 hours of six with 145 samples, zero `CLOSE_WAIT`
   at every one and 288 leech cycles with none failed. The RSS slope is not a
   result at that window: +0.622 MiB/h at r squared 0.105 is noise fitted to a
   line. **A slope needs a window long enough to have one**, and the tree has
   moved a long way since, so restart rather than resume.
4. **[T-212](memory.md)**, whenever a fixture of stalling peers is being built
   anyway.
5. **Then, a category at a time.** `bep-coverage.md` has the most left and the
   most shared machinery. After it, `dht.md`.

**Two corpus sources the list above may want**, both already on this machine
and neither needing a fetch: `reference/RESEARCH.md` sections C and D, and
`contrib/rqbit/` in <https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, whose
`0012` and `0014` are read and not taken. **Both are reads.** Nothing is
opened, filed or commented on either, by [RULES.md](RULES.md) section 6a.

## Open questions for the operator

**None.** The one this file carried last session, whether to build `--trace`
out or delete the ten names, was ruled on: build. It is done and
[T-219](cli-surface.md) is closed rather than left open for a residue.

**Two things worth the operator's eye rather than a decision**, both from
[T-222](cli-surface.md), and both are behaviour changes rather than fixes to
something that was failing.

`bit-cli.toml`, the user config file at the platform config directory, and
every `BIT_CLI_*` variable now change what a run does. Until this session they
changed what `bit-cli config show` printed and nothing else, so a file that has
been sitting on this machine has never had an effect and will have one on the
next run. `bit-cli config show` says what is in force and where each value came
from, and `--no-config` turns the files off.

`--config` naming a file that is not there is an error on every command now.
It was exit 8 on `config show` and exit 0, in silence, on the other fifteen, so
a script that passed a stale path and did not notice will start noticing.
