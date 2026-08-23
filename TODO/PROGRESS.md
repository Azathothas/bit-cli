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
first and section 6a the second. A previous revision of this file listed
"Offer the patches upstream" as work; it was wrong, and the operator has now
given the instruction twice. `patches/UPSTREAM.md`'s `Upstream:` field answers
"could a release retire this patch on its own?" and nothing else.

**The six hour soak is run by the operator, in a foreground terminal.** No agent
session lasts six hours, and a session ending kills the process it started. The
command is under "Start here next session"; a session's job is to read the CSV
the operator's run leaves behind, not to start one.

## `patches/TASKS.md` is finished, and the fork is maintenance now

Twelve of its thirteen rows are done and the thirteenth is not waiting on a
seam any more. That file's own closing section says the signal to look for is
"no entry in the table at the top of this file is still waiting on a seam", and
that is now true: [T-102](bep-coverage.md), BEP 55, waits on a measurement its
own acceptance asks for and on a fixture that can produce an unreachable peer,
neither of which is `librqbit`'s to give.

So this session's work order is derived from [INDEX.md](INDEX.md)'s four
questions again, not from `patches/TASKS.md`, and the vendored trees become
what `patches/README.md` describes: run `scripts/upstream-scan.ps1` on a version
bump, reconcile with `scripts/vendor-sync.ps1`, keep `UPSTREAM.md` true.

## State

- **Last session:** 2026-08-23T08:57:40Z, unattended, and it was ended on the
  operator's word. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.
- **Tests:** 1,228 passing, 0 failing. 1,204 at the start. Plus **149** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run.
- **Gates:** clean, on rustc 1.98.0.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green on all **nineteen** jobs at run **32635532834**, against commit
  `35707ee`. The pushes after it are named under "Start here next session"
  item 1, which is to read the last of them. Seventeen jobs until this session:
  [T-150](cli-surface.md) added `Clippy (tracking stable)` and
  `Clippy (tracking beta)`.

  **One job went red and it is fixed.** Run **32637486414** failed `Record` on
  a `TODO/` citation that the same local `gates.ps1 -Fix` run had approved and
  then moved: the record gate ran before `cargo fmt --all` rewrote the file.
  That is [T-220](cli-surface.md), and the gate runs after the two that rewrite
  now.

  **Nothing went red that was not meant to.** Run **32631078557** carries the
  demonstration T-150's acceptance asked for: the run's own conclusion is
  `success` with `Clippy (tracking beta)` failing, because the tracking job
  does not block. What it found is [T-218](cli-surface.md), fixed two pushes
  later, and the run above is green on all nineteen including that job.
- **Entries:** 169 items. 27 open, 1 partial, 0 blocked, 131 done, 10 deferred
  to Phase C. 131 of 159 workable done, 28 left.
- **Tree:** 94 Rust files, 55,302 lines of code, 13,766 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **27 patches**
  across nineteen sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Fourteen entries closed, three filed, and nine of the fourteen had a premise
the measurement disproved.** That is the highest proportion any session has
had, and every one of them was found by running something rather than by
reading it.

The work order was taken in its order: items 1 and 2, then the `ci` and
`windows` group, the `trackers` and `dht` groups, the three entries the
previous session filed, and two from the effort S list.

### The premise corrections, because they are most of the session

- **[T-178](windows.md)** said `bit-cli` cannot fix a `librqbit` loop and that
  the loop is on the payload write path. Neither is true: the trees are
  vendored, and the one `add_torrent` call in the workspace installs
  `SafeStorageFactory`, whose own copy of that loop had carried the guard since
  the day before the entry was filed.
- **[T-075](windows.md)** asked for two documented forms to be run. Running
  them is what the entry was for: one does not exist on the host it is for and
  the other silently corrupts the data.
- **[T-150](cli-surface.md)** said the case for pinning is strongest for the
  lint job. `RUSTFLAGS: -D warnings` is set for the whole workflow, so every
  job that compiles is a lint gate.
- **[T-180](trackers.md)** was filed as an undecided question and was a live
  defect: every magnet this tool announced said `left=0`, which means seed.
- **[T-050](dht.md)** supposed a short run "may still write" a cache. It wrote
  one, into **another program's** directory.
- **[T-051](dht.md)** said the run waits on a deadline. It failed in 0.01
  seconds, with the wrong exit code and a message naming a `librqbit` field.
- **[T-213](cli-surface.md)** predicted "the flag and the test". The flag broke
  the resolver that finds the payload, and `verify` had the same defect with a
  test that passed anyway.
- **[T-214](cli-surface.md)** said "there is no flag to be inert". There were
  three, on four commands.
- **[T-094](bench.md)** said `--trace http` records every request in memory and
  proposed measuring it with `bench webseed`. It is a log filter, and
  `bench webseed` has its own HTTP client, so that command's traced runs are
  comparisons of a run with itself.

### Item 3, the `ci` and `windows` group

**[T-178](windows.md), P3, done.** The Windows positioned-write loop takes its
write as an argument now, so a double can return `Ok(0)`, and the error names
the offset and the bytes left. Five tests. **Run against the defect**: with the
guard replaced by `continue` the test does not fail, it hangs, and was killed
at 90 seconds. The vendored loop carries the guard too, mirroring the read-side
guard upstream already wrote, as patch 26.

**[T-075](windows.md), P2, done.** `scripts/check-redirect.ps1` builds a
torrent named `café-λ-日本.bin` and runs seven ways of capturing `--json` on
both hosts. `[Console]::OutputEncoding` and `$OutputEncoding` are two settings
and neither defaults to UTF-8: on this machine both hosts read at `IBM437` and
5.1 writes `us-ascii` into a native command. `| ConvertFrom-Json`, which the
README recommended, returns a name that is not the name, **and the JSON still
parses**. `utf8NoBOM` does not exist before PowerShell 6. The README now
carries the recipe and the table, and `bench/redirect-*.json` are the runs.

**[T-150](cli-surface.md), P2, done.** `RUST_GATE` pins all seven gating jobs
and `release.yml` takes it too. `clippy-next` tracks `stable` and `beta`
without blocking. `check-todo.ps1` fails when two workflows disagree about the
pin or when a job floats without `continue-on-error`, and **both checks were
run against the defect they claim to catch**.

### The tracking job earned itself in one afternoon

**[T-218](cli-surface.md), filed and done.** `beta` found
`Atomic::fetch_update` deprecated in 1.99, which `-D warnings` makes an error.
Running the acceptance **the way CI runs it** found a second one in
`vendor/librqbit-utp`: `use std::{f64, ...}` imports the module, so
`f64::INFINITY` resolves to the legacy constant. Both fixed, neither by taking
the new name: `try_update` does not exist on the MSRV.

### Item 4, the `trackers` and `dht` groups

**[T-180](trackers.md), P2, done.** `Announce::left` is `Option<u64>`, which is
what turned the second call site into a compile error. Unknown goes out as
`i64::MAX`, and the corpus decided that value: `torrent/tracker/http/http.go:36`
carries the two failures that rule out the alternatives, both from a real
tracker. Inbound, a negative count is `None` rather than zero. `peers: [42]`
was already survived and never mentioned; five shapes are named now.

**[T-065](trackers.md), P3, done.** `--scrape-url`. It names one endpoint, so a
run with several trackers is refused rather than reporting one answer as many.

**[T-063](trackers.md), P3, done.** Decided rather than built: parallel,
everywhere, with the reasoning in [`docs/trackers.md`](../docs/trackers.md).
The corpus note calling the download path "forced" no longer holds, because the
`HashSet` that flattens the tiers is in a tree this repository owns.

**[T-050](dht.md), P2, done.** `DhtSessionConfig::default()` persists, and the
path is `com.rqbit.dht`, so `bit-cli` was rewriting
`%LOCALAPPDATA%/rqbit/dht/cache/dht.json`. One 90 second run took it from
95,248 bytes to 81,752. With `persistence: None` the same run leaves it byte
for byte and second for second.

**[T-051](dht.md), P2, done.** A magnet with no way to fetch metadata is exit 2
before the session is built, and the message says what to do. The check is on
the condition rather than on `--web-seed-only`, and `--peer` keeps a run alive
because BEP 9 carries metadata from a peer.

### Item 5, the three the previous session filed

**[T-213](cli-surface.md), P3, done.** `-O` on `seed`, and the resolver that
finds a multi-file payload now looks for file 0 where the caller said it would
be. `verify` had the same defect.

**[T-214](cli-surface.md), P3, done.** `--on-complete` on `seed` fires once,
when the payload has passed its hash check and the listener is up. The three
hook flags are a `HookArgs` struct now; `peers`, `bench leech` and `bench seed`
refuse them instead of ignoring them. `docs/hooks.md` has both tables.

**[T-212](memory.md)** is untouched and still waits on a fixture of stalling
peers, which nothing this session needed.

### Item 6, two from the effort S list

**[T-191](bench.md), P2, done.** `fold_document` refuses to fold a document
under a `kind` another **command** already claimed, and both directions are
tested: the `seed` pair fails, and one command run two ways still merges.

**[T-094](bench.md), P2, done.** Measured on `download --web-seed-only`, where
the trace fires. Three configurations, up to 16,384 trace lines: in every one
the difference between the arms is smaller than the plain arm's own run-to-run
spread, and in one the traced arm used less memory.

**[T-219](cli-surface.md) filed, P1**, and it is the largest flag-does-nothing
found so far: ten of the eleven documented `--trace` subsystems raise a target
nothing writes to. Measured on one run tracing all ten: zero lines of stderr,
against 257 for `http` on the same run.

### The last push went red, and the gate that missed it is fixed

**[T-220](cli-surface.md), filed and done.** `gates.ps1` ran the `record` gate
before `man` and `fmt`, both of which rewrite files under `-Fix`. A local run
printed `record ok` and `all gates pass`; the push went red on `Record`,
because `cargo fmt --all` had moved a cited line by ten **in the same run that
had just approved it**. The gate runs after both now. Same shape as the
`check-man.ps1 -Fix` defect found earlier the same day.

## In progress

Nothing is half-written. Every entry this session touched is closed in
[INDEX.md](INDEX.md) with its acceptance run recorded.

- **[T-219](cli-surface.md)** is filed, open, P1, effort M, and is the one
  thing this session found and did not fix. It names the two subsystems that
  are cheap and the ones that need a seam in `vendor/`.
- **[T-212](memory.md)** is open and unchanged.
- **[T-102](bep-coverage.md)** is open and **[T-164](peers.md)** is partial,
  the only partial left.

## Start here next session

**The shape of the work order is the operator's, from three sessions ago.** Not
priority first. Clear as many small entries as possible, so the open count
comes down, and then take the bigger ones a **category at a time**.

**Item 2 below is the one place this order departs from that shape**, and it
does it deliberately: [T-219](cli-surface.md) is effort M rather than S, and it
is taken first because it is P1, because it is ten documented capabilities that
do nothing, and because it is measured and ready to start. A session that would
rather hold the shape takes item 4 first and loses nothing by it.

The counts are derived from the rows rather than from memory:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Read the run this session's last push started**, which is the only thing
   left unread:

```bash
gh run list --limit 1
```

2. **[T-219](cli-surface.md), P1, effort M.** Ten documented `--trace`
   subsystems that raise a target nothing writes to. It is the biggest thing
   open, it is measured, and its Approach names the order: `ratelimit` and
   `retry` need no seam, `disk` goes through `SafeStorage::write_through`, and
   `piece` and `picker` are decided in the vendored session and are their own
   work. Its acceptance is a test per subsystem, so the list a caller reads and
   the list that works cannot drift again.
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
4. **The rest of the effort S entries**, eight rather than the ten the last
   work order listed, because [T-094](bench.md) and [T-191](bench.md) closed:
   [T-176](create-seed.md), [T-173](metainfo.md), [T-187](metainfo.md),
   [T-041](memory.md), [T-165](peers.md), [T-033](performance.md),
   [T-008](webseed.md), [T-103](bep-coverage.md). **Check each against the tree
   before building**: nine of this session's thirteen said something the tree
   did not do, and every one of those was found by running a command rather
   than by reading.
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

**One, and it is a scope question rather than a blocker.**

[T-219](cli-surface.md) is P1 and effort M, and its Acceptance offers two ways
out per subsystem: emit on the target, or stop documenting it. The cheap answer
is to delete ten names from `SUBSYSTEMS` and the manuals, which takes an hour
and makes the surface honest. The expensive one is to make each subsystem emit
what its sentence promises, which is where the value is and which needs a seam
in `vendor/` for `piece` and `picker`.

**The recommendation is to build rather than delete**, in the order the entry
names, because `--trace` is the only debugging surface this tool has and
`--jsonl` does not cover `disk`, `picker` or `ratelimit`. A session can start
with `ratelimit`, `retry` and `disk`, which need nothing from `vendor/`, and
the entry stays open for the rest. If the operator wants the surface honest
sooner, deleting the ten names first and re-adding each as it lands is the
other order and costs nothing that cannot be undone.

The soak is item 3 and remains the operator's to run.
