# Memory

Sixteen issues touch memory growth, per-torrent overhead, and buffer pooling.

---

### T-040 Memory and descriptors grow without bound over a long run

Source:      https://github.com/ikatson/rqbit/issues/525 (open)
Category:    memory
Priority:    P0
Effort:      L
Status:      partial

Problem:     A reporter running `librqbit` inside a long-lived server saw both
             RSS and open descriptors climb until the process failed. It
             started after changing trackers, which points at the tracker or
             peer discovery path rather than at storage.
Relevance:   The netdisk deployment is a long-lived process. This is the
             failure mode that takes it down at 3am.
Approach:    Related to [T-011](disk-io.md) and [T-020](peers.md), and possibly
             the same defect seen from three angles. Do not guess: run one
             `bit-cli seed` for six hours with a sampler recording RSS, handle
             count, and socket state counts every 30 seconds, and plot it. A
             flat line closes this; a slope names the subsystem.
Acceptance:  `scripts/soak.ps1` writes `bench/soak-<timestamp>.csv` with the
             three series, and this entry records the slope of each over six
             hours.

**Where this stands, 2026-08-21.** Read this first; the rest of the entry is
the history in order, and its earlier summaries were true when they were
written.

- **Descriptors: disproved.** An idle seeder holds exactly 189 handles across
  533 samples over 4.6 hours, and a loaded one shows no trend. `CLOSE_WAIT` is
  zero at all 1,064 samples across both runs.
- **Memory: reproduced, quantified, linear.** 0.804 MiB an hour under `steady`,
  r squared 0.73 over 525 samples, and the last three hours give the same
  slope. Not a settling curve.
- **Open, on attribution rather than on wall clock.** What the byte is charged
  to is not yet separable, because completions ran at a constant rate for the
  whole run. The next measurement is two two-hour runs at different leech
  rates, not another six-hour one.

The evidence and the fits are in
[the 2026-08-21 section](#session-of-2026-08-21-the-question-is-answered-and-the-answer-is-linear).

---

**The harness is built and a 1.76 hour run is recorded. The six hour run the
acceptance asks for has not been completed, so this stays open.** (2026-08-20;
superseded by the section above.)

`scripts/soak.ps1` samples one long-lived `bit-cli seed` every
`-SampleSeconds` and writes `bench/soak-<timestamp>.csv` with resident memory,
peak resident memory, handles, threads, CPU time, and the TCP socket states
broken out by state. Six workloads, because a slope has to name a subsystem
rather than "the process":

| workload | what it drives |
| --- | --- |
| `idle` | a seeder with no tracker and nothing connecting. The control. |
| `announce` | a loopback tracker at a five second interval. The tracker never expires a peer, so the peer list handed to the seeder grows for the whole run, which is the path this entry's report points at. |
| `leech` | real downloads against the seeder, one finishing and another starting. |
| `steady` | announce and leech together. The deployment, and the default. |
| `churn` | connections that open and close without handshaking. T-020's shape, and the known positive. |
| `all` | steady plus churn. |

`all` is deliberately not the default. Churn strands sockets at about 30,000
handles an hour, which is [T-020](peers.md) rather than this entry and swamps
every other series in the same chart. It also starves the leechers: the same
run that completed 22 downloads in two minutes without churn completed 1 and
failed 2 with it.

Two things the harness does that are worth keeping. It runs from its own copy
of `target/release/bit-cli.exe`, because a six hour run would otherwise hold
that file for six hours and Windows will not let `cargo` replace a running
executable. And the seeder reports its own RSS and handle count in every
`progress` event under `--jsonl`, so the summary cross-checks the sampler
against the subject: a sampler that disagrees with the process is measuring
something else.

**The measurement so far**, `bench/soak-20260820T132757504Z.csv`, workload
`steady`, 16 MiB payload, two leechers, 30 second samples, **1.76 hours and 398
completed leech cycles**:

| series | first | last | max | per hour | r squared |
| --- | --- | --- | --- | --- | --- |
| `rss_bytes` | 14.81 MiB | 16.31 MiB | 17.03 MiB | **+0.58 MiB** | 0.63 |
| `peak_rss_bytes` | 14.94 MiB | 17.27 MiB | 17.27 MiB | +0.85 MiB | 0.85 |
| `handles` | 210 | 216 | 240 | **+0.77** | 0.004 |
| `threads` | 29 | 27 | 35 | -0.02 | 0.00 |
| `tcp_total` | 1 | 1 | 2 | +0.01 | 0.001 |
| `tcp_close_wait` | 0 | 0 | **0** | 0 | n/a |
| `cpu_ms` | 156 | 30,438 | | +17,156 | 0.9995 |

What that says, and what it does not.

- **Descriptors are flat.** 0.77 handles an hour at an r squared of 0.004 is
  noise, not a trend, and `CLOSE_WAIT` was zero at every one of the 200
  samples. So the half of this entry that names descriptors is not reproducing
  under a deployment-shaped load. [T-011](disk-io.md) bounding open files with
  `--max-open-files` is the likeliest reason.
- **CPU is flat as a rate.** 17,156 ms of CPU per hour is 4.8 ms per second of
  wall time, under 0.5% of one core, and the r squared of 0.9995 says it is a
  straight line rather than an acceleration.
- **Memory rises, slowly, and the fit is weak.** 0.58 MiB an hour at an r
  squared of 0.63 over 1.76 hours is about 14 MiB a day if it is linear, and
  1.76 hours is not long enough to say whether it is linear, a settling curve,
  or an allocator that has not returned pages yet. `peak_rss_bytes` is a
  high-water mark, so its slope is bounded below by zero and says less than its
  r squared suggests.

**What is left is the run, not the harness.** Six hours of `steady`, and an
`idle` control of the same length to separate the session's own timers from
the load:

```powershell
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -Workload steady -PayloadMiB 16
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -Workload idle
```

Once both are in, the ceilings turn the record into a check:
`-RssCeilingMiBPerHour`, `-HandleCeilingPerHour`, and
`-CloseWaitCeilingPerHour` each fail the run when the slope passes them, and
with none named the slopes are recorded rather than judged.

One residue in the harness itself: the summary JSON is written only when the
sampling window ends, so a run that is killed early leaves the CSV and no
summary. The numbers above were computed from the CSV by hand for that reason.
Writing the summary on every sample, or on a signal, is a small change and
would have saved that step. **Done in the session below, and it was needed
within the hour.**

**Session of 2026-08-20, second run: the harness is fixed, an idle control is
in, and the six hour run is still the thing that is missing.**

Two runs were started together, `steady` and `idle`, so the load could be
separated from the session's own timers. Neither reached six hours before the
session ended, and what they did reach is recorded here because the summary is
now written after every sample rather than only at the end. That change is the
harness residue this entry named, and it paid for itself on the first run: the
`steady` run died at 2.26 hours and its record survived.

| series | steady, 2.26 h, 258 samples | idle, 2.76 h, 315 samples |
| --- | --- | --- |
| `rss_bytes` per hour | **+0.93 MiB**, r squared 0.65 | **-0.15 MiB**, r squared 0.11 |
| `rss_bytes` first, last, max | 14.75, 18.23, 20.19 MiB | 13.14, 12.38, 13.67 MiB |
| `handles` per hour | +2.09, r squared 0.015 | **0.00**, and 188 at every sample |
| `tcp_close_wait` max | **0** | **0** |
| leech cycles | 514 | none by design |

**The idle control is the new fact.** A seeder with no tracker and nothing
connecting holds 188 handles at every one of 315 samples over 2.76 hours, and
its resident memory does not rise: the slope is slightly negative at an r
squared of 0.11, which is a flat line with noise on it. So whatever the `steady` run is doing, it
is the load doing it and not the session's timers, and this entry's report of
descriptors climbing on their own does not reproduce at all.

**The `steady` slope is still not a straight line.** 0.93 MiB an hour at an r
squared of 0.65, with a maximum of 20.19 MiB against a last reading of 18.23,
is a series that rises and falls rather than one that climbs. Two and a half
hours cannot separate a settling curve from a leak, which is exactly what the
six hour run is for and why this stays open.

*That reading was wrong, and the next section says why.* The maximum above the
last reading is one thread burst, not the series changing direction. Excluding
it the slope is 0.804 MiB an hour at an r squared of 0.73, and it is a line.

Both runs also shared the machine with a full `cargo build --release` and the
test suite, several times over. That is worth knowing when reading the RSS
series: the leech cycles compete with whatever else is running.

**One harness defect, found the hard way.** The first `steady` run ended at
2.26 hours with `ScriptHalted`. `Start-Process` for the next leecher threw,
almost certainly on the redirected output file the previous leecher had not
finished releasing, and with `$ErrorActionPreference = 'Stop'` and a trap above
it that one throw ended a six hour run. Fixed two ways: starting a process
retries three times before giving up, and the whole sampling body is inside a
`try` that counts a failure and carries on. The count is `cycles.load_errors`
in the summary, because a run with a hundred of them is measuring something
else.

**What the next session does.** Both commands, from a clean tree:

```powershell
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -Workload steady -PayloadMiB 16 -Root .tmp/soak-steady
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -Workload idle -Root .tmp/soak-idle
```

Start them first, before anything else, and leave the machine as quiet as the
rest of the work allows. `bench/soak-<timestamp>.json` is readable while the
run is going: `complete` is `false` until the window ends, and the slopes in it
are the slopes so far. When both are in, put the numbers in the table above,
answer the one open question, and set the ceilings:

```powershell
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -Workload steady `
  -RssCeilingMiBPerHour <answer> -HandleCeilingPerHour <answer> -CloseWaitCeilingPerHour 1
```

The question is only whether the `steady` RSS slope is linear, a settling
curve, or an allocator holding pages. A slope that flattens after the first
hour is the second; one that holds 0.9 MiB an hour to hour six is the first and
names a leak worth chasing.

Runs recorded so far, all partial, all under `bench/`:
`soak-20260820T132757504Z` (steady, 1.76 h, the first),
`soak-20260820T155246381Z` (steady, 2.26 h, killed by the harness defect above),
`soak-20260820T155309362Z` (idle, 2.76 h, the control, stopped with the
session), and
`soak-20260820T181505020Z` (steady, restarted at 18:15 UTC and still running
when the session ended, so its files are not committed).

The restarted run is the one to look for: if `.tmp/soak-steady` and its
bench files are still on the machine, read the JSON before starting another,
because a run that reached five hours is worth more than a fresh one.

## Session of 2026-08-21: the question is answered, and the answer is linear

The pair started at 2026-08-21T01:24:28Z ran **4.61 hours of the six** and were
killed with the session, not by a defect. Between them they hold 1,064 samples,
which is more than the six hour run was ever going to need. Both are committed:

- `bench/soak-20260821T012428252Z.csv`, `steady`, 16 MiB payload, two leechers,
  531 samples over 4.605 hours, 1,060 completed leech cycles.
- `bench/soak-20260821T012429347Z.csv` and `.json`, `idle`, the control, 533
  samples over 4.617 hours.

**Six hours was not needed, and the reason is in the data rather than in the
schedule.** The discrimination the entry asks for is settled by comparing the
slope over the whole run against the slope over its last three hours. If those
agree, the series is a line. If the second is smaller, it is settling. They
agree, and they already agreed at three hours.

### The steady run

Fitted against elapsed hours, over the 525 samples that are not the one thread
burst described below:

| model | fit | r squared | rmse (MiB) |
| --- | --- | --- | --- |
| **linear in `t`** | 14.886 + **0.804** MiB/h | **0.733** | **0.652** |
| logarithmic, `log(1+t)` | 14.322 + 2.207 | 0.673 | 0.722 |
| square root, `sqrt(t)` | 13.857 + 2.018 | 0.673 | 0.723 |
| saturating, `1-exp(-t/2h)` | 14.300 + 4.016 | 0.645 | 0.752 |
| saturating, `1-exp(-t/5h)` | 14.626 + 6.105 | 0.712 | 0.678 |
| saturating, `1-exp(-t/8h)` | 14.721 + 8.423 | 0.723 | 0.665 |

Linear wins outright. Every saturating model fits worse, and they improve
monotonically as the time constant grows, which is the signature of a curve
that does not bend inside the window: at eight hours the exponential is a
straight line over four and a half. The last three hours on their own give
**0.744 MiB/h at r squared 0.52**, against 0.804 over the whole run. The slope
does not decay.

So the answer to the open question is **linear**. Not a settling curve, and not
an allocator holding pages and then releasing them.

The half-hourly shape, which is what a single whole-run slope hides:

| window | min | median | max | mean threads | mean handles |
| --- | --- | --- | --- | --- | --- |
| 0.0-0.5 h | 15.00 | 15.62 | 16.49 | 26.9 | 216 |
| 0.5-1.0 h | 14.43 | 14.74 | 16.03 | 26.6 | 216 |
| 1.0-1.5 h | 14.91 | 15.53 | **41.70** | 39.1 | 252 |
| 1.5-2.0 h | 15.29 | 16.34 | 20.63 | 27.9 | 219 |
| 2.0-2.5 h | 15.49 | 17.17 | 23.72 | 28.2 | 221 |
| 2.5-3.0 h | 15.55 | 17.40 | 17.77 | 26.9 | 217 |
| 3.0-3.5 h | 15.74 | 17.59 | 18.73 | 26.8 | 217 |
| 3.5-4.0 h | 16.08 | 17.94 | 18.62 | 26.7 | 216 |
| 4.0-4.5 h | 16.05 | 18.61 | 19.17 | 27.0 | 217 |

All figures MiB. The floor rises from 14.43 to 16.05 and the median from 14.74
to 18.61, over three and a half hours. The rise is in the level, not only in
the peaks.

### The one spike, and why it is not the trend

Resident memory's maximum of 41.70 MiB, the handle maximum of 1,150, and the
thread maximum of 352 are **all the same sample**, number 130, at 1.107 hours.
Resident memory's 99th percentile is 19.39 MiB, so the maximum is 2.15 times
the 99th. Three samples in the whole run are above 100 threads: 1.11, 1.12, and
2.02 hours.

The three series move together. `corr(threads, handles)` is **0.9984** and
`corr(threads, rss)` is 0.767, and a straight fit of resident memory against
thread count gives **14.645 MiB + 79.5 KiB per thread**, which is the size of a
thread's committed stack. So a handle spike is a thread spike, a thread spike
is a memory spike, and all three retire. That is a burst of blocking work, not
growth.

This is why the whole-run slope including the spikes is 0.732 MiB/h at r
squared 0.27 and the slope excluding them is 0.804 at r squared 0.73. Removing
three samples makes the trend clearer, not weaker, which is the opposite of
what removing evidence for a trend would do.

### The idle control

The control is what makes the steady number mean anything.

| series | over 533 samples and 4.617 hours |
| --- | --- |
| `handles` | **189 at every sample.** Minimum 189, maximum 189. |
| `threads` | 21 from hour two onward, no variation |
| `tcp_total` | 1 at every sample, which is the listener |
| `tcp_close_wait` | **0 at every sample** |
| `rss_bytes` | 13.75 MiB falling to 12.02, then flat within 0.03 MiB for the last 2.5 hours |
| `peak_rss_bytes` | last rose at hour 1, then flat |

A seeder with no tracker and nothing connecting does not move. So the sampler
is not the source, the session's own timers are not the source, and every
number in the `steady` run is the load.

### What this closes and what it does not

- **The descriptors half of this entry is disproved.** `idle` holds exactly 189
  handles across 533 samples. `steady` is -2.18 an hour at an r squared of
  0.003, which is noise. `CLOSE_WAIT` is **zero at all 1,064 samples across
  both runs**, so [T-020](peers.md) needs the churn shape and does not appear
  under a deployment-shaped load. That was already the reading at 2.76 hours
  and 4.6 hours does not change it.
- **The memory half reproduces and is now quantified.** 0.804 MiB an hour under
  `steady`, linear, r squared 0.73 over 525 samples. That is 19.3 MiB a day and
  579 MiB over thirty if it holds, which is the shape this entry's report
  describes.
- **What is not answered is what the byte is charged to.** Leech completions
  run at a constant 228.5 an hour at an r squared of 0.9999 for the whole run,
  so elapsed time and completed work are collinear in this data and cannot be
  separated by it. 0.804 MiB an hour and 3.6 KiB per completed leech fit the
  same points exactly as well.

**The next measurement is not a longer run.** It is two shorter ones at
different leech rates, because that is the only thing that separates per-hour
from per-download:

```powershell
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 120 -Workload steady -PayloadMiB 16 -Leechers 1 -Root .tmp/soak-rate1
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 120 -Workload steady -PayloadMiB 16 -Leechers 4 -Root .tmp/soak-rate4
```

Four times the completion rate against the same wall clock. If the MiB per hour
quadruples, it is per download and the leech path is where to look. If it does
not move, it is per hour and the announce and timer paths are. Two hours is
enough for both, because the discrimination above needed three.

### The ceilings, set

The slopes above are now the reference, so `scripts/soak.ps1` can judge rather
than record. These are the values a regression run should carry:

```powershell
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 120 -Workload steady -PayloadMiB 16 `
  -RssCeilingMiBPerHour 2 -HandleCeilingPerHour 32 -CloseWaitCeilingPerHour 1
```

- `-RssCeilingMiBPerHour 2` is 2.5 times the measured 0.804. Above it something
  new is happening; at it, this entry's own finding is not what fails the run,
  which is the [check-close-wait.ps1](../scripts/check-close-wait.ps1) rule.
- `-HandleCeilingPerHour 32` against a measured slope of zero in `idle` and
  -2.18 in `steady`. It has to clear the thread bursts, which reach 1,150
  handles for two samples and come straight back.
- `-CloseWaitCeilingPerHour 1` against zero at 1,064 samples. Anything at all
  here is [T-020](peers.md) arriving under a load that has never shown it.

Reproduce the analysis from the committed CSVs:

```powershell
pwsh -NoProfile -Command "Import-Csv bench/soak-20260821T012428252Z.csv | Where-Object { $_.iso -match '^\d{4}-' } | Measure-Object -Property rss_bytes -Minimum -Maximum -Average"
```

### One harness defect this run found: T-157

The `steady` run's JSON is **4,833 NUL bytes**. Its CSV survived with all 531
samples because a CSV is appended and a summary is rewritten. The whole point
of rewriting the summary after every sample is that a killed run still leaves
its slopes, and the kill destroyed exactly that. Written up as
[T-157](#t-157-a-killed-soak-destroys-the-summary-it-was-rewriting) below, and
fixed.

### T-157 A killed soak destroys the summary it was rewriting

Source:      `bench/soak-20260821T012428252Z.json`, 2026-08-21
Category:    memory
Priority:    P2
Effort:      S
Status:      **done**

Problem:     `scripts/soak.ps1` rewrote `bench/soak-<timestamp>.json` with
             `Set-Content` straight onto the path. That truncates first and
             fills after, so a process killed between the two leaves a file of
             NUL bytes rather than the previous summary.
Relevance:   The rewrite exists so a killed run leaves its slopes. Doing it
             non-atomically means a killed run leaves less than nothing: a file
             that parses as an object with every field empty.
Approach:    Write to `<path>.tmp` and rename over the target. A rename within
             one directory is atomic on both NTFS and POSIX.
Acceptance:  A short run writes the summary, leaves no `.tmp` behind, and the
             summary parses.

The `steady` run of 2026-08-21T01:24:28Z is the worked example. 4.605 hours and
531 samples in the CSV, 4,833 NUL bytes in the JSON, and the slopes in this
entry had to be recomputed from the CSV. `Set-Content` now writes
`$jsonPath.tmp` and `Move-Item -Force` renames it over `$jsonPath`.

Acceptance, run:

```powershell
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 1 -Workload idle -Root .tmp/soak-atomic-check
```

`complete=True`, 2 samples, and zero `bench/*.tmp` left behind.

### T-041 Per-source window cache is bounded but not measured

Source:      `bit-cli` design
Category:    memory
Priority:    P2
Effort:      S
Status:      open

Problem:     Each HTTP source caches whole windows in memory. The bound is
             `cache_windows * chunk_size`, and `cmd::download::cache_windows`
             picks the count so the product stays near 16 MiB per source. With
             twelve mirrors that is 192 MiB, which is a real number nobody has
             measured.
Relevance:   `--web-seed-chunk-size 64MiB` with ten sources is 640 MiB of cache
             by construction, and nothing warns about it.
Approach:    Report the computed cache budget in `webseed list --json` so it is
             visible before the run, and cap the total across sources rather
             than per source.
Acceptance:  `bit-cli webseed list <TORRENT> --json` carries
             `"cache_budget_bytes"` per source and a total, and a run with ten
             sources at a 64 MiB chunk size warns when the total exceeds
             256 MiB.

### T-042 Peak RSS is not captured in any report

Source:      the operator's brief
Category:    memory
Priority:    P1
Effort:      S
Status:      **done**

Problem:     A3.11 requires peak RSS, total CPU time, and open handle count in
             every `bench` report. None is collected.
Relevance:   "A benchmark without its environment recorded is not a result."
             Two throughput numbers with different memory ceilings are not
             comparable.
Approach:    On Windows, `GetProcessMemoryInfo` gives `PeakWorkingSetSize` and
             `GetProcessHandleCount` gives handles; on Linux, read
             `VmHWM` from `/proc/self/status` and count `/proc/self/fd`. Both
             are a few lines and need no new dependency.
Acceptance:  Every `bench` report carries `peak_rss_bytes`, `cpu_ms`, and
             `open_handles`, and `bit-cli bench webseed --format json` shows
             all three non-zero.

`bit_cli_core::sysinfo::Process::sample` reads all three, with no new
dependency: raw `extern "system"` declarations against `kernel32` on Windows
and `/proc` reads on Linux. It also splits CPU time into user and system,
because on a loopback benchmark the split is the result: the run below spent
29.9 s of CPU over 10 s of wall time and most of it in the kernel, which says
the ceiling is the socket rather than the client.

Every sample of the time series carries the three figures as well as the
summary, so a leak shows up as a slope rather than as one number at the end.
`Process::max` folds samples so a spike halfway through a run is not lost when
memory is released before the end.

Acceptance, 2026-08-19T23:13:33.253Z, release build:

```
$ bit-cli bench webseed .tmp/bench/payload.torrent --web-seed $URL \
    --format json --duration 10s --warmup 2s --concurrency 8 --request-size 1MiB
```

```
"process": {
  "peak_rss_bytes": 42074112,
  "rss_bytes": 33861632,
  "cpu_ms": 29859,
  "cpu_user_ms": 8609,
  "cpu_system_ms": 15234,
  "open_handles": 219
}
```

All three are non-zero. The unit tests in `sysinfo::tests` assert that a sample
reads every field, that CPU time only goes up, and that a delta never goes
negative:

```
$ cargo test -p bit-cli-core --lib sysinfo
test result: ok. 14 passed; 0 failed
```

The Linux path is written and compiles under `#[cfg(unix)]` but has not been
run here: this machine is Windows. See the same note under
[T-091](bench.md).
