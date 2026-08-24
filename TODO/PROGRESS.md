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

- **Last session:** 2026-08-24T07:51:30Z, unattended, documentation and
  research. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.
- **Tests:** 1,298 passing, 0 failing. 1,290 at the start. Plus **149** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run. `vendor/` is untouched by this session, so neither count moved.
- **Gates:** clean, on rustc 1.98.0. A default run prints **eight**: `text`,
  `man`, `fmt`, `record`, `tree`, `clippy`, `test`, `deny`. `tree` is new, from
  [T-230](cli-surface.md).

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-one** jobs now, `Tree` added beside `Record` and `Soak fit`.
  Green at run **32689315662**, against commit `e0a3c18`, which is the last
  commit of this session carrying code. **One run this session was red**,
  32687202487, and it is written up under [T-229](bench.md):
  `Test (macos-latest)` failed on an assertion that bounded something the code
  does not bound, on a slow runner rather than on a defect. Six runs and one of them red,
  which is the first red job in two sessions and is the reason T-229 has a
  correction under it rather than nothing.
- **Soak, finished, and most of it does not count.** The operator's second six
  hour run, `bench/soak-20260823T154716064Z`, is committed with its CSV and
  JSON. It reached its full six hours and reported "every named ceiling held".
  Its workload stopped at `t+4653s`, 1.29 hours in: 298 leech cycles completed,
  **1,080 failed**, and the seeder spent the remaining 4.7 hours alive and
  using 47 milliseconds of CPU. That is [T-232](memory.md).

```bash
pwsh -NoProfile -File scripts/soak.ps1 -ReadCsv bench/soak-20260823T154716064Z.csv
```

```bash
gh run list --limit 1
```

- **Entries:** 192 items. 31 open, 1 partial, 0 blocked, 149 done, 11 deferred
  to Phase C. 149 of 181 workable done, 32 left.
- **Tree:** 97 Rust files, 57,893 lines of code, 14,905 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0. **Nothing was added**, and
  [T-173](metainfo.md) is why that is worth saying: the patch it expected to
  need turned out to be unnecessary.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What this session is doing

Documentation and research. It writes prose, records and three pieces of
tooling, and it files new work rather than implementing it.

1. Mine the references the operator supplied: client masking, announce
   fidelity against a loopback tracker, NAT traversal beyond the BEPs,
   throughput, the magnet and DHT toolkit, the `TheDancingDeveloper-org`
   sweep, and the web UI survey. One reference at a time, its corpus entry
   and its filed entries written before the next one starts.
2. Reconcile `reference/RESEARCH.md` and `reference/README.md` against what
   has since landed, archive what is closed to `reference/HISTORY/`, then
   rewrite [INDEX.md](INDEX.md) once with the new entries in it.
3. Rewrite `README.md` as a map and move its topic prose into `docs/`, adding
   an agent orientation page, a reference-mining procedure, a task-authoring
   procedure and a set of worked examples.
4. Sweep the docs for project history, and add a docs check to the gates and
   to CI.
5. Two deep reviews, the summary, the kickoff.

Three pieces of tooling are built and run here rather than filed: the client
profile generator ported from `joal`, the announce fidelity check ported from
`RatioTracker`, and the docs check. The features the findings describe are
filed and left.

## What the last session did

**The operator's three items first, then the work order.** Four entries closed,
four filed, two advanced, one corrected. **Three of the four filed came out of
running something that was meant to check something else**: a guard written for
a stray file found a damaged benchmark, a soak read for one entry's sake turned
out to have measured an idle process, and a transport flag's own measurement
found a defect in this repository's encryption.

### The operator's two rulings

**`--out` may leave the output directory, and stays that way.**
[T-226](cli-surface.md) carries the argument: `--out` is the caller's own path,
typed on their own command line, and `--dir` is allowed anywhere already.
`out_may_leave_the_output_directory_because_it_is_the_callers_path` pins it, so
tightening it later is a decision made against a passing test.

**The vendored tree is to report a dropped path component**, and
[T-173](metainfo.md) closed on it **without a patch**. See below.

### [T-230](cli-surface.md), P1, filed and done: how `under/inner.bin` reached the remote

**1,000 bytes of `0x41`, tracked on `main` from `2d369db` and pushed.** It came
from [T-226](cli-surface.md)'s own acceptance table, third row,
`--dir .tmp/t226b --out under`: that row demonstrates the resolution T-226
fixed, which made a relative `--out` absolute against the working directory, so
the payload landed at `<repo>/under/inner.bin`. The fix is in the same commit
that carries the file, which is why nothing looked wrong afterwards.

**The defect that wrote it is not the reason it reached the remote.** Any run
that writes into the working tree gets the same ride: `git add -A` takes it,
`.gitignore` covered `*.iso` and `*.img` and not `*.bin`, and **nothing
anywhere compared the result against what this repository is supposed to
contain**.

**The history was rewritten.** `git filter-branch --index-filter` over the
eight commits carrying it, identity and dates unchanged, 198 commits before and
198 after, and the only difference between the old tip and the new one is that
file. Force pushed with `--force-with-lease` pinned to the old tip.
`git-sync.ps1` was not used and could not be: it commits and pushes work, and a
history rewrite is neither. Every push after it is `git-sync`'s again, and
`gh api repos/Azathothas/bit-cli/contents/under` answers 404.

**`scripts/check-tree.ps1` is the guard, in three places**, because the file got
in through the gap between them: the `tree` gate, the `Tree` CI job, and
`git-sync.ps1` after staging and before the commit. Two rules, either of which
alone would have stopped it: a fixed top level, and outside `vendor/` a fixed
set of file kinds. It reads the **index**, which is what lets one script answer
both "what is in this tree" and "what is about to go in".

### [T-231](memory.md), P1, filed and done, and the guard found it on the day it was written

**`bench/soak-20260821T012428252Z.csv` is committed evidence and it ended in
176 NUL bytes.** NTFS flushes a file's size before its bytes, so a soak killed
mid-append leaves the tail zero filled. `Import-Csv` reads that as one more
record of empty strings, `[double]""` is 0, and the fit ran through a final
sample of zeros.

| | reported | true |
| --- | --- | --- |
| samples, hours | 532 over **0** | 531 over 4.605 |
| `rss_bytes` last | **0.00 MiB** | 19.27 MiB |
| `peak_rss_bytes` largest fall | **-42.19 MiB** | 0.00 |

The last row gives it away without knowing anything else: a high-water mark
cannot fall, and the report said it fell by its whole value.

**[T-157](memory.md) found this same failure on the other file of the same run
and fixed it.** The `.json` is rewritten and was made atomic; the `.csv` is
appended and cannot be, so nothing there was to fix and nothing was looked at.
The fix for an append is on the reading side, and no reader had one.

### [T-232](memory.md), P1, filed and open: the soak reported a pass on a dead workload

**At `t+4653s` three things happen in one interval and none comes back**: leech
cycles freeze at 298 for 540 samples, the seeder's `cpu_ms` freezes at 38,250
and reaches 38,297 four and a half hours later, and established connections go
to zero while the listening socket stays bound. 1,080 cycles fail, two per
sample, each inside a 30 second tick rather than waiting out its
`--stop-after 120s`.

The report says `"verdict": "every named ceiling held over 6 hours"` with an
empty failures list, and **every number in it is true**. The flat lines that
are its evidence are flat because nothing was happening.

**It is not a cycle count.** A `steady` run at four leechers and a five second
sample interval completed **552 cycles in 14 minutes with none failed**, nearly
twice the 298 the failing run stopped at, at eight times the rate.
`bench/soak-20260824T023232248Z` is committed as that negative result.

**The instrument is fixed and the cause is not.** `soak.ps1` judges its
workload whether or not a ceiling is named, `-LeechFailurePercent` defaults to
5 and the failing run is at **78.37**, a failing cycle now leaves its exit code
and output behind, `-ListenerCheck` passes `--listener-check` to the seeder, and
`soak.ps1 -ReadCsv` says all of it about a finished CSV. A fifth case in
`scripts/check-soak-fit.ps1` holds it, with the healthy six hour run as the
control.

### [T-224](memory.md), P2, advanced: the reproduction finished and 78 percent of it is idle

**No step**, and the flattest figures this repository has recorded: `rss_bytes`
13.95 to 17.71 MiB, 0.46 MiB/h at r squared 0.54, largest rise 1.99 MiB.

**What survives is the half this entry needed.** The step being answered is at
`t+1.161h` and cycles were still completing at `t+1.292h`, so the reproduction
**did** cross the step point under load and did not step. What does not survive
is everything after `t+1.3h`.

### [T-173](metainfo.md), P3, done on the ruling, and the seam needed no patch

**The entry expected to have to patch `librqbit_core` so `FileDetails` carries
the raw components. It carries them already.** `FileIteratorName::to_vec` is
public, decodes with the same encoding `to_pathbuf` uses, and
`iter_file_details_ext` is the public iterator `TorrentMetadata::new` builds
`file_infos` from. So `vendor/` is untouched and `patches/UPSTREAM.md` gains no
section. **A patch not carried is a patch no reconciliation has to re-apply.**

`SafeStorageFactory::create` plans from those components joined with `/` rather
than from the `PathBuf` the session built, and the planner already did the
rest. `/lead.bin`, `mid//dle.bin` and `trail.bin/` land exactly where they
landed before and now say `DroppedComponent`.

**It also takes the platform out of the answer**, which was not the point and
is the better half of it: `PathBuf::push` treats a backslash as a separator on
Windows and as an ordinary character elsewhere, so a component holding one used
to lay the same torrent out two different ways depending on the target.

**This closed by making an existing test fail.** The pin left behind for it
fired, named T-173, and printed all three renames.

### [T-101](bep-coverage.md), P3, advanced: uTP is reachable, measured, and not usable yet

**`--transport tcp|utp|both`, default `tcp`, on `LimitArgs`**, so every command
that starts a session takes it. Nothing was hand-rolled: `librqbit-utp` is
vendored and `ListenerMode` already existed.

**Two settings decide the answer and only one is obvious.**
`SessionOptions::connect.enable_tcp` governs the **dialer** and defaults to
true whatever the listener says. The first version of the flag set the listener
alone and a `utp` leecher reached a `tcp`-only seeder **over TCP and reported
success**. That row is now the negative control that gives every other case its
meaning.

| case | encryption | finished | rate |
| --- | --- | --- | --- |
| tcp | prefer | yes | 152.38 MiB/s |
| utp | off | yes | **76.19 MiB/s** |
| both | prefer | yes | 152.38 MiB/s |
| tcp against utp | off | **no**, the control | |
| utp | require | **no**, and that is [T-233](peers.md) | |
| tcp | require | yes | 160 MiB/s |

**It stays open on one thing**, and on a claim this session made and then
disproved. The Acceptance's second clause asks for lower induced latency than
TCP, and loopback has no bottleneck to queue at, so nothing here can show it.
The entry also said for an hour that a uTP transfer over the default `[::]`
dual-stack bind does not complete. **It does.** That belief came from two
command line runs at the default `--encryption prefer` and survived the
discovery of the real cause; measured again on purpose it completes, so
`--transport utp` is usable from the command line today with
`--encryption off`. The correction is written under the entry rather than
edited into it.

### [T-233](peers.md), P1, filed and open: MSE over uTP stalls after the handshake

**Every other combination of the two works**: uTP in plaintext, TCP under MSE,
TCP in plaintext. Only the pair fails, and it fails after the connection is
carrying traffic in both directions.

**It is this repository's own code**: the only difference between the working
and the failing case is whether `MseTransform` wraps the connection.

**Then it was measured, and that narrowed it twice.** A probe on
`EncryptedWrite`, kept under `--trace handshake`, shows every byte handed to
the wrapper accepted by the stream below it, in order, with no deferral: the
write side is **eliminated**. Four paired traces agree to the byte on what one
end sent and the other received: uTP is **eliminated** as a carrier. What is
left is the read side.

**And where it stalls is not the same twice**, which is why the entry says so:
sometimes the MSE handshake itself does not complete, and sometimes it
completes and the peer wire messages that follow are never acted on. A first
reading of the entry said "the bytes never leave the leecher"; that is true of
one run and not of another, and the correction is written under it.

### [T-228](cli-surface.md), P3, done, and the third fixed path was not in the entry

`$PID` in `gates.ps1`'s two logs and in `git-sync.ps1`'s, which two pushes at
once would hit for the same reason. A passing run deletes them and a failing
one keeps them, because the detail line names them by path.

### [T-229](bench.md): a correction a red job found

**"The bound is exact rather than a tolerance" was wrong about the code it
described.** The warmup is `while recorder.in_warmup() { drive(..) }`, every
iteration spawns `concurrency` fresh workers, and each leaves a tail past its
deadline. Seven chunks against a bound of four on a loaded `macos-latest`
runner. **The assertion asserted that the machine cannot be slow**, which is
[RULES.md](RULES.md) section 5's line and its fourth worked example. What
replaced it is the control the test's own doc comment already named and never
asserted: two steps at the same concurrency, within an order of magnitude.

## In progress

Nothing is half-written. Every entry that closed is closed in
[INDEX.md](INDEX.md) with its acceptance run recorded.

- **[T-232](memory.md)** is open with its instrument built and its cause
  unnamed. One soak closes it.
- **[T-224](memory.md)** is open with the same soak closing it, at a different
  leech rate.
- **[T-233](peers.md)** is open, pinned by a test, with two candidate seams
  named by line.
- **[T-101](bep-coverage.md)** is open on one thing only, a latency
  measurement loopback cannot produce. The `[::]` bind it was also open on
  turned out to be nothing.
- **[T-102](bep-coverage.md)** and **[T-168](bep-coverage.md)** are open and
  untouched, and **[T-164](peers.md)** is partial, still the only partial.

## Start here next session

**The shape of the work order is the operator's, from six sessions ago.** Not
priority first. Clear small entries so the open count comes down, then take the
bigger ones a **category at a time**.

The counts are derived from the rows rather than from memory:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Re-measure the baseline rather than trusting the one above**, which is
   [RULES.md](RULES.md) section 1 step 5. Read the run this session's last push
   started: the CI line above names it.

```bash
gh run list --limit 1
```

2. **The soak, and it goes before anything else on the operator's
   instruction.** No run is in flight. **One run closes two entries** and it is
   the operator's to start in a foreground terminal: `-Leechers 4` is the
   different leech rate [T-224](memory.md) has left, and `-ListenerCheck 60s`
   is what [T-232](memory.md) needs to say whether the seeder stopped answering
   or the leechers stopped calling.

   Kill the strays first, or two runs share a tracker:

```bash
pwsh -NoProfile -Command "Get-Process bit-cli,loopback-tracker,loopback-churn -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\.tmp\*' } | Stop-Process -Force"
```

```bash
cargo build --release --bins --examples
```

   Then **print this in chat for the operator** and do not start it inside a
   session:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -Leechers 4 -ListenerCheck 60s -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

   **The harness will not report a pass on a dead workload again.** Read the
   result with the command on the state line, which now says what fraction of
   the cycles failed and when the last one completed.

3. **[T-233](peers.md), P1, effort M**, and it is the largest thing this
   session found. Read its second half first: the write side and the transport
   are both **eliminated by measurement**, so the two candidates left are on
   the read side and are named with their lines. Build the fixture before
   anything else, a pair of real `librqbit_utp` streams in one process, because
   a duplex pipe is what the existing unit tests use and they pass.
   `mse_over_utp_does_not_carry_a_torrent` is the pin to invert, and the probe
   the last session used is still there under `--trace handshake`.

4. **Then the category pass, and `bep-coverage.md` is still first.**
   [T-101](bep-coverage.md) is advanced but open, and what is left of it is a
   latency measurement that needs a shaped path with a bounded queue or a
   second machine. Neither is a flag. [T-102](bep-coverage.md) and
   [T-168](bep-coverage.md) are the untouched two, then `dht.md`.

5. **The two entries open on a decision rather than on work**:
   [T-033](performance.md), the three aria2 flag names, with its curve already
   measured; and [T-227](memory.md), the window cache total, which needs one
   throughput curve first.

6. **[T-212](memory.md)**, whenever a fixture of stalling peers is being built
   anyway.

**Two corpus sources the list above may want**, both already on this machine
and neither needing a fetch: `reference/RESEARCH.md` sections C and D, and
`contrib/rqbit/` in <https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, whose
`0012` and `0014` are read and not taken. **Both are reads.** Nothing is
opened, filed or commented on either, by [RULES.md](RULES.md) section 6a.

## Open questions for the operator

**None outstanding.** Both of last session's standing questions were ruled on
and both are recorded where the work is: `--out` may leave the output
directory, [T-226](cli-surface.md), and the vendored tree reports a dropped
path component, [T-173](metainfo.md), which turned out to need no patch at all.

The third question from last session, whether `path: ["", "foo"]` beside
`path: ["foo"]` should be renamed rather than refused, was not re-raised and
the refusal stands. [T-173](metainfo.md) carries the argument and
`an_entry_that_collapses_onto_another_is_refused_whole` carries the test, so
relaxing it later is a decision made against a passing test.

**One thing to be aware of rather than to decide.** The history of `main` was
rewritten this session, so every commit from `2d369db` onward has a new hash. A
clone made before 2026-08-24T02:30Z will not fast-forward and needs a fresh
fetch or a reset. Nothing else changed in any of those commits.

## Two behaviour changes worth the operator's eye

Not decisions. Both change what an existing command does, and both are this
session's.

**`--transport tcp|utp|both` is new on every command that starts a session.**
The default is `tcp`, which is what every run did before, so nothing moves
unless the flag is given. `--transport utp` works today **with
`--encryption off`** and stalls at the default `--encryption prefer`, which is
[T-233](peers.md).

**A torrent whose file path carries an empty component now reports the drop.**
`/lead.bin` still lands at `lead.bin`, exactly where it landed before, and
`renames` in `download --json` now carries a `DroppedComponent` row for it
where it carried nothing. A script testing `renames.is_empty()` on such a
torrent will see a different answer.
