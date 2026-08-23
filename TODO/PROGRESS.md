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

- **Last session:** 2026-08-23T16:18:55Z, unattended, and it was ended on the
  operator's word. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.
- **Tests:** 1,290 passing, 0 failing. 1,271 at the start. Plus **149** in the
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

- **CI:** green at run **32657376953**, against commit `148d31f`. Five runs
  this session and **none was red**, which is the first session that can say
  so. There is a **twentieth** job now, `Soak fit`, added by
  [T-224](memory.md). Two runs started after this line was first written,
  **32658445998** for `5c5ae0a` and the one this session's last push starts;
  read them rather than trusting this. The final documentation commit carries
  a CI skip marker and starts no run.
- **Soak, finished:** the six hour run of 2026-08-23T09:01:32Z is committed,
  681 samples, 1,360 leech cycles, none failed. Its RSS slope is
  [T-224](memory.md) and the step in it is now a column rather than an
  argument.
- **Soak, running:** the operator's second six hour run,
  `bench/soak-20260823T154716064Z`, started 2026-08-23T15:47:16Z from a release
  build of `d3bc6a5`. It was **329 samples and 2.92 hours in** when this file
  was written and it was still going. **It crossed `t+1.161h` and did not
  step**: largest single rise 1.48 MiB against the committed run's 11.61, and a
  whole-run slope of about 1.1 MiB/h, which is that run's pre-step slope. Read
  the rest of it when it finishes.

```bash
pwsh -NoProfile -File scripts/soak.ps1 -ReadCsv bench/soak-20260823T154716064Z.csv
```

```bash
gh run list --limit 1
```

- **Entries:** 178 items. 23 open, 1 partial, 0 blocked, 144 done, 10 deferred
  to Phase C. 144 of 168 workable done, 24 left.
- **Tree:** 96 Rust files, 57,539 lines of code, 14,705 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Six entries closed, four filed, two advanced, and every single one of them
turned on running something rather than reading it.** Four of the six closed
with their premise disproved or their Approach replaced, which is now the
ordinary outcome rather than the surprising one, and the two biggest findings
of the session were both defects in instruments: one in what a report says a
torrent's files are called, one in what a benchmark says a curve looks like.

The work order's effort S list is **cleared**: T-103, T-041, T-165 and T-008
all closed and T-033's measurement is done and committed.

### [T-103](bep-coverage.md), P2, effort S, done, and its title was wrong

**Nothing is refused. This tree had two decoders for a torrent's file names
and the reports used the wrong one.** `Value::as_text` decoded with
`String::from_utf8_lossy`, and `info`, `files`, `magnet` and `webseed list`
all read through it; the session that downloads reads through the vendored
`librqbit`, whose `detect_encoding` runs `chardetng`. On one cp932 torrent,
measured against `loopback-fileserver`:

| what said it | said |
| --- | --- |
| `bit-cli files`, `path` | one replacement character per byte |
| `bit-cli download`, `name` | `フォルダ` |
| the URL `webseed list` printed | `/%EF%BF%BD…/%EF%BF%BD….bin` |
| the URL that run requested | `/%E3%83%95%E3%82%A9%E3%83%AB%E3%83%80/…` |

`webseed list` is documented as printing "the exact URL each file maps to". For
this torrent it printed a 404 that was also not what the same binary asked for
thirty seconds later. Two cp932 filenames that differ also collapsed onto one
string, so `files` listed one path twice.

**One decoding rule now, called from both sides.** The vendored
`detect_encoding` keeps its behaviour and loses its body to
`detect_encoding_of`, a free function over byte slices, and `bit-cli`'s
`parse_info` calls the same function over the same raw bytes. The `.utf-8`
keys are preferred on **both** sides, because a rule applied on one re-creates
the disagreement with the sides swapped: `Metainfo` reads `name.utf-8`,
`path.utf-8` and `comment.utf-8`, and `TorrentMetaV1Info` and
`TorrentMetaV1File` gained `name_utf8` and `path_utf8`.

**The rule is not a nicety.** Fourteen names across six encodings, and the
detector guessed wrong for six, including the common shape of an ASCII release
name with one non-ASCII filename under it, where the ASCII dominates the input
and every non-ASCII name comes out wrong.

`the_two_decoders_in_this_tree_agree` parses the same bytes with both
implementations over four shapes. Reverting the multi-file half of the
vendored patch fails it and names both sides. Upstream's own tests were run:
**149 passing, unchanged**.

### [T-226](cli-surface.md), P1, effort S, filed and done

**`download -o/--out` was declared and nothing in the workspace read it**, the
sixth of the flag-does-nothing family and the plainest: renaming the field
broke no build. The machinery was already there and `seed` was already using
it, so the change is where the flag is resolved and what it is turned into.

**Three things the entry did not name and two were found by running.** The
first `--dir` resolution **escaped the output directory**: `env.resolve` makes
a relative path absolute against the working directory, so joining it returned
the absolute one and `--out ../../x` wrote two levels above the repository,
which a run confirmed by leaving a file there. `output_directory` in the
report named the run's directory rather than the torrent's. And a `..`
survived into that report, because `canonicalize` needs the path to exist.

### [T-229](bench.md), P1, effort S, filed and done, and it is the sharper of the two instrument defects

**`bench webseed --concurrency-sweep` charged the run's warmup to its own first
steps.** The recorder excludes warmup samples from a step's byte count and
`end_step` divides by the step's own wall time, so a step inside the warmup
reported its real seconds against no bytes. `best_concurrency` comes off that
curve, so the verdict could invert: `--concurrency-sweep 16,1` reported **best
concurrency 1**.

The control is the argument. The same concurrency twice, nothing to tell the
two apart:

| sweep | before | after |
| --- | --- | --- |
| `1,1` | 2.66 MiB/s, 908.73 MiB/s | 897.15 MiB/s, 896.85 MiB/s |
| `16,1` | best **1** | best **16** |

**Why no test caught it is the part worth keeping.** The existing sweep test
already asserts `step.requests > 0` for every step and has always passed: its
options come from `bench_options`, which sets `warmup: Duration::ZERO`. Every
test of the sweep turned off the one thing that breaks it.

### [T-041](memory.md), P2, effort S, done, and the number in it was half the real one

The entry said ten sources at `--web-seed-chunk-size 64MiB` is 640 MiB of
cache. `cache_windows` has a floor of two, so it is **1.25 GiB**. The floor is
right and it is the whole reason the budget is exceeded: one window cannot
hold the window being read and the next one at once.

**A test called `the_window_cache_stays_inside_its_memory_budget` asserted the
case where it does not**: two windows of 64 MiB is 128 MiB against a
per-source budget of 16. The floor stays and the name went.

`webseed list --json` carries `sources[].cache_budget` and
`cache_budget_total`, computed by the same function the run calls. The warning
is on `download` too, once per run.

### [T-165](peers.md), P2, effort S, done, premise and Approach both disproved

`librqbit` 9.0.1 reads `reqq` and sets `flow.request_window` to
`reqq.min(128)`. It is upstream's own code, no patch and no `UPSTREAM.md`
section. The reported depth is observed, not fixed. Run against the claim, by
changing what the bridge advertises:

| `REQUEST_QUEUE` | peak in flight | mean | leech rate |
| --- | --- | --- | --- |
| 250 | 128 | 19 | 120.30 MiB/s |
| 32 | 32 | 7 | 122.14 MiB/s |

**The Approach is disproved by the same two runs.** It proposes a BDP-sized
depth from an EMA of the peer's rate. Mean in flight is 19 against a window of
128, and quartering the window left throughput where it was. A rewrite that
moves no number does not ship, so this closes with **no residual entry behind
it**.

### [T-008](webseed.md), P3, effort S, done, premise no longer reproduces

The Acceptance is `requests` equal to `blocks` on a torrent past a thousand
pieces. Met, and so is every smaller shape including the entry's own 3,000
byte fixture, which it was filed on answering "3 blocks for 5 requests": five
runs give three and three.

**The mechanism is still in the code** and the guard is **not** added, on the
rule that produced the Acceptance: no run makes it save a fetch, and the wire
behaviour is identical either way because the duplicate's answer is already
dropped. What closed in the tree is a comment in
`a_leech_measures_the_transfer_the_hashing_and_the_disk` that stated the
premise as fact and weakened the assertion above it.

### [T-224](memory.md), P2, advanced and left open: the step did not reproduce

**The cheap half is built.** `Get-Slope` reports `largest_rise`,
`largest_rise_hours`, `largest_fall` and `step_share` beside every fit, and
`soak.ps1 -ReadCsv` re-reads a finished run through the same function, with
`-ReadJson` beside it. Reading the committed run's `rss_bytes` line is now the
whole of what this entry had to be computed by hand to say:

```
series      first   last    max per hour   r2 step up at h step down unit
rss_bytes   13.55  35.18  39.20     3.71 0.72   11.61 1.16     -7.23 MiB
```

`scripts/check-soak-fit.ps1` is the acceptance and a CI job, **Soak fit**,
with three cases including a generated ramp at four times the slope and no
step. Every number it asserts comes from `soak.ps1 -ReadJson`, because the
check was written computing its own fit first, which would have passed against
a `soak.ps1` that reported nothing.

**And the operator's reproduction run crossed `t+1.161h` without stepping**:
1.48 MiB where the committed run had 11.61. Its whole-run slope, 1.07 MiB/h,
is the committed run's **pre-step** slope of 1.02. The entry stays open
because its Acceptance asks for two runs at **different** leech rates and this
is one at the same rate.

### [T-033](performance.md), P3, advanced: the curve is measured and it is not flat

Taking it needed T-229 fixed first; believing `1: 0 B/s` for ten seconds is
what found that defect. Once the instrument was honest, 64 MiB loopback, 20
seconds a step, committed at `bench/split-20260823T182709577Z.json`:

| concurrency | 1 | 2 | 4 | 8 | 16 |
| --- | --- | --- | --- | --- | --- |
| rate | 940.53 MiB/s | 1.61 GiB/s | 2.85 GiB/s | **3.44 GiB/s** | 3.38 GiB/s |
| p99 | 18ms | 17ms | 10ms | 14ms | 29ms |

3.7 times from one connection to eight, a knee at eight, and past it
throughput stops while p99 doubles. **Not flat**, so the flags are not
disqualified by their own Acceptance. What is left is the surface decision,
and the entry says why `-s` and `-x` would mean nearly the same thing here.

### Also filed

**[T-227](memory.md), P2, effort M.** T-041's Approach also proposed capping
the window cache total across sources. Not taken: it halves two mirrors'
caches from four windows to two, which is a throughput change with no
measurement behind it. The entry names the curve that decides it.

**[T-228](cli-surface.md), P3, effort S.** Two `gates.ps1` runs at once
collide on `$env:TEMP\bit-cli-gates-tests.txt`, one fixed path per machine,
and the second dies naming a locked file rather than the other run. It cost
this session two minutes and it is [T-225](create-seed.md)'s shape in another
script.

### Three claims this session's own closing review caught

All in T-229 and its note under T-033, none touching code, all pushed as
`0e2baff`. The acceptance command named `scripts/bench-webseed.ps1`, which
runs no sweep and would reproduce none of the table above it. The Acceptance's
first half named an assertion the committed test does not make and could not
make without a tolerance on a throughput number. And "the first point of every
curve it ever printed was near zero" is not true of every curve: the error is
proportional to how much of a step falls inside the warmup, which at the
defaults is about half.

## In progress

Nothing is half-written. All six entries that closed are closed in
[INDEX.md](INDEX.md) with their acceptance runs recorded.

- **[T-224](memory.md)** is open with half its Acceptance met, and what is
  left is one soak at a different leech rate.
- **[T-033](performance.md)** is open with its measurement done and committed,
  and what is left is a surface decision about three aria2 flag names.
- **[T-173](metainfo.md)** is open and unchanged, premise disproved, seam
  named.
- **[T-212](memory.md)** is open and unchanged, still waiting on a fixture of
  stalling peers.
- **[T-102](bep-coverage.md)** is open and **[T-164](peers.md)** is partial,
  the only partial left.

## Start here next session

**The shape of the work order is still the operator's, from five sessions ago.**
Not priority first. Clear as many small entries as possible so the open count
comes down, and then take the bigger ones a **category at a time**.

**The effort S list is finished.** All eight are closed: T-176 and T-187 last
session, and T-103, T-041, T-165 and T-008 this one, with T-033's measurement
done and T-173 open on a seam smaller than its entry. So the shape's first
clause has nothing left to point at and item 4 is the category pass, which is
what it was always going to become.

The counts are derived from the rows rather than from memory:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Re-measure the baseline rather than trusting the one above**, which is
   [RULES.md](RULES.md) section 1 step 5. Read the run this session's push
   started: the CI line above names the last run that had finished when this
   file was written.

```bash
gh run list --limit 1
```

2. **The soak, and it goes before anything else on the operator's
   instruction.** It runs for hours, so it starts first and the session works
   while it runs.

   **A six hour run started 2026-08-23T15:47:16Z was still going when this was
   written**, at 329 samples and 2.92 hours, from a release build of `d3bc6a5`.
   **Check whether it is still running before doing anything else**, and if it
   is, leave it alone, read what it has, and go to item 3. A second soak
   started beside it shares the tracker and neither measures anything.

```bash
pwsh -NoProfile -File scripts/soak.ps1 -ReadCsv bench/soak-20260823T154716064Z.csv
```

   **That command is new and it is the one to use.** It re-reads a finished or
   running CSV through the same `Get-Slope` a live run uses, prints the fits
   with the largest single-interval change each way, and starts nothing.
   `-ReadJson <path>` writes the same numbers for a script.

   **What that run already answered** is [T-224](memory.md)'s first question:
   the 11.7 MiB step at `t+1.161h` **did not reproduce**. It crossed the same
   point with a largest single rise of 1.48 MiB, and its whole-run slope of
   about 1.1 MiB/h is the committed run's pre-step slope. Read the rest of it
   when it finishes and write the final numbers into the entry.

   **What follows is for when there is no soak running and a fresh one is
   wanted.** Kill the strays first, or two runs share a tracker:

```bash
pwsh -NoProfile -Command "Get-Process bit-cli,loopback-tracker,loopback-churn -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\.tmp\*' } | Stop-Process -Force"
```

```bash
cargo build --release --bins --examples
```

   Then **print this in chat for the operator** to run in a dedicated
   foreground terminal, and do not start it inside a session:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

3. **[T-224](memory.md), P2, effort M, and it is one run from closing.** Half
   its Acceptance is met: `soak.ps1` reports the step and
   `scripts/check-soak-fit.ps1` holds it in CI. The other half asks for the
   cause named with a file **or** two runs at different leech rates showing the
   step is not tied to completed work. One run at the same rate exists and did
   not step. What is left is one more at a different rate, which is the
   operator's to start for the reason every soak is:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 120 -Leechers 4 -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

4. **Then a category at a time, and `bep-coverage.md` is first.** It has the
   most left and the most shared machinery, and [T-103](bep-coverage.md)
   closing this session took the decoding question out of the way that the
   rest of it would have tripped over. After it, `dht.md`.

5. **The three entries that are open on a decision rather than on work**, and
   any of them is an afternoon once the decision is made:
   [T-033](performance.md), the three aria2 flag names, with its curve already
   measured; [T-227](memory.md), the window cache total, which needs one
   throughput curve first; [T-228](cli-surface.md), which is one line.

6. **[T-212](memory.md)**, whenever a fixture of stalling peers is being built
   anyway.

**Two corpus sources the list above may want**, both already on this machine
and neither needing a fetch: `reference/RESEARCH.md` sections C and D, and
`contrib/rqbit/` in <https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, whose
`0012` and `0014` are read and not taken. **Both are reads.** Nothing is
opened, filed or commented on either, by [RULES.md](RULES.md) section 6a.

## Open questions for the operator

**One decision this session made on its own and would take a ruling on**, plus
the two from last session that are still standing.

**New. Should `--out` be allowed out of the output directory?** It is today.
`--out ../../x` with `--dir out` writes two levels above the working directory,
and the first version of the flag did that by accident before it did it on
purpose. The argument for allowing it is that `--out` is the caller's own
path, typed on their own command line, and `--dir` is already allowed anywhere;
the argument against is that `-O`/`--index-out` is sanitised and a reader may
expect the same of its neighbour. The difference is that `-O`'s path is a
**file inside** the output directory and `--out` names the destination itself.

**The recommendation is to leave it**, which is what this session did.
`a_relative_out_resolves_against_dir` pins the ordinary case, so tightening it
later is a decision made against a passing test rather than a change nobody
notices.

**Still standing, both from last session and neither blocking anything.**
Whether the vendored tree should be patched so a dropped path component can be
reported, [T-173](metainfo.md), recommendation leave it. And whether
`path: ["", "foo"]` beside `path: ["foo"]` should be renamed rather than
refused, recommendation keep the refusal.

## Two behaviour changes worth the operator's eye

Not decisions. Both change what an existing command does.

**A torrent whose names are not UTF-8 now reports them decoded.** `info`,
`files`, `magnet` and `webseed list` used to print one replacement character
per byte and now print what the download writes, with a `name_encoding` field
saying how. A script matching on those strings will see different ones. The
URL `webseed list` composes changes with them, and it changes from one that
404s to the one the run actually requests.

**`download -o/--out` writes somewhere else now.** It has been accepted and
ignored, so a script passing it has been writing to the download directory and
will start writing where it asked. `--out` with two sources is a usage error
where it used to be accepted and ignored.
