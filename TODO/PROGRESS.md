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

- **Last session:** 2026-08-23T06:14:39Z, unattended, and running.
- **Tests:** 1,166 passing, 0 failing, re-measured at the start rather than
  carried forward. Plus **149** in the vendored trees, which the workspace gates
  do not run.
- **Gates:** clean, 112.8s, on rustc 1.98.0.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green at run **32620536345** against commit `a289977`, all
  **seventeen** jobs. `f055328` is on top of it and is documentation only, so it
  carries `[skip ci]` and started no run.
- **Entries:** 161 items. 44 open, 2 partial, 0 blocked, 105 done, 10 deferred
  to Phase C. 105 of 151 workable done, 46 left.
- **Tree:** 92 Rust files, 52,557 lines of code, 12,567 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **25 patches**
  across seventeen sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**This session is in progress.** What is below is what it set out to do, written
before doing it, by [RULES.md](RULES.md) section 1 step 4. It is rewritten as
each entry closes.

### The two standing instructions, taken first

The operator's amendments to the previous work order, applied before any entry:

1. **The soak is handed to the operator.** Printed as a command for a dedicated
   foreground terminal, and read back from its CSV at the start of every later
   session until the six hour window is complete.
2. **No unauthorised remote operations, ever.** `Azathothas/bit-cli` is the only
   repository an agent may write to. Everything else is read only. The work
   order item that said "Offer the patches upstream" is deleted rather than
   deferred.
3. **The rule is written where a session will hit it**, so it is not proposed a
   third time: [RULES.md](RULES.md) section 6 and the new section 6a,
   `patches/README.md`, the head of `patches/UPSTREAM.md` and all seventeen of
   its `Upstream:` fields, `patches/TASKS.md`, and the three kickoff prompt
   samples on the `references` branch.

**A claim in the previous revision of this file was wrong**, found while reading
the soak it describes. It said the cut-short run left "no JSON summary, because
the script writes that at the end". `bench/soak-20260823T040627780Z.json` exists,
carries all five slopes and says `"complete": false`. That is exactly what
[T-157](memory.md) built: `soak.ps1` rewrites its summary after every sample so
a killed run still leaves them. The numbers the previous session quoted were
right; the sentence about where they came from was not.

### The entries

Nothing is closed yet this session.

## In progress

- **The standing-instruction documentation**, above, is written and unpushed.
- **[T-212](memory.md)** is filed and open, with an acceptance that names the
  fixture it needs.
- **[T-102](bep-coverage.md)**, **[T-164](peers.md)** are open or partial.

## Start here next session

**The shape of the work order is the operator's, from the session before this
one.** Not priority first. Clear as many small entries as possible, so the open
count comes down, and then take the bigger ones a **category at a time**: all of
`bep`, or all of `dht`, in one session rather than one entry from each.

The reading taken of "an easy win": one that is **not** waiting on an open
high-priority entry. That is nearly all of them. **There is no open P0 and
exactly one open P1**, `T-081`, BEP 52 v2 and hybrid torrents, effort XL. The
only open entry that waits on it is [T-134](bep-coverage.md), v1 and v2 info
hash reconciliation. Everything else below is unblocked.

Derived from the `Effort:` line of every open or partial entry, not from memory:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Read the CI run named above before anything else.**
2. **The six hour soak, and it is the operator's to run.** No agent session
   lasts six hours and a session ending kills the process it started, so a
   session prints this and moves on. In a dedicated foreground terminal, from
   the repository root:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

   **How a later session reads it.** The run writes `bench/soak-<stamp>.csv` one
   row per sample and rewrites `bench/soak-<stamp>.json` after every sample, so
   a run still going has both. `"complete": true` in the JSON is the only thing
   that says the window finished; `elapsed_hours` and `samples` say how far it
   got. Six hours at the default 30 second interval is about 720 samples.

```bash
pwsh -NoProfile -Command "Get-ChildItem bench/soak-*.json | Sort-Object LastWriteTime | Select-Object -Last 1 | Get-Content | ConvertFrom-Json | Select-Object generated_at, complete, elapsed_hours, samples"
```

   The last run reached 1.32 hours of six with 145 samples, zero `CLOSE_WAIT` at
   every one of them and 288 leech cycles with none failed. Those two are counts
   and hold at that window. The RSS slope is not: +0.622 MiB/h at r squared
   0.105 is noise fitted to a line, and [T-040](memory.md) already recorded the
   same shape at 5.06 hours. **A slope needs a window long enough to have one.**
3. **The `cli` group, eight entries at effort S**, which is the largest single
   category of easy wins and the one where a reader sees the result:
   [T-115](cli-surface.md) partial, [T-136](cli-surface.md),
   [T-154](cli-surface.md), [T-116](cli-surface.md), [T-118](cli-surface.md),
   [T-155](cli-surface.md), [T-156](cli-surface.md), [T-159](cli-surface.md).
   Two of them, T-118 and T-159, are about the help output itself, so
   `scripts/check-man.ps1 -Fix` follows both.
4. **The `ci` and `windows` groups, four entries at effort S**:
   [T-150](cli-surface.md), [T-161](cli-surface.md), [T-075](windows.md),
   [T-178](windows.md). T-150 and T-161 are workflow edits, so they need a real
   run to prove them and cannot go in a `-NoCi` push.
5. **The `trackers` and `dht` groups, five entries at effort S**:
   [T-180](trackers.md), [T-063](trackers.md), [T-065](trackers.md),
   [T-050](dht.md), [T-051](dht.md).
6. **The rest of the effort S entries**, ten of them, in
   `bench`, `create`, `metainfo`, `memory`, `peers`, `performance`, `webseed`
   and `bep`: [T-094](bench.md), [T-191](bench.md), [T-176](create-seed.md),
   [T-173](metainfo.md), [T-187](metainfo.md), [T-041](memory.md),
   [T-165](peers.md), [T-033](performance.md), [T-008](webseed.md),
   [T-103](bep-coverage.md).
7. **Then, a category at a time.** `bep-coverage.md` is the one with the most
   left and the most shared machinery. After it, `dht.md`.
8. **[T-212](memory.md)** whenever the fixture for it is being built anyway. It
   is the only entry in the record whose numbers are arithmetic rather than
   measurement, and it needs a swarm of peers that answer an extended handshake
   with a large `metadata_size` and then stall.
   `crates/bit-cli-core/src/bench/swarm.rs` already builds synthetic peers.

**Two corpus sources the list above may want**, both already on this machine
and neither needing a fetch: `reference/RESEARCH.md` sections C and D, which is
where seventeen of the T-163 to T-182 block came from, and `contrib/rqbit/` in
<https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, which is fetched per
session rather than kept and whose `0012` and `0014` are read and not taken.
**Both are reads.** Nothing is opened, filed or commented on either.

## Open questions for the operator

None outstanding. The one the previous session left, the unfinished soak, is
answered by the operator's own instruction: it is item 2 above and it is run
outside a session.
