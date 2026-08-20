# Memory

Sixteen issues touch memory growth, per-torrent overhead, and buffer pooling.

---

### T-040 Memory and descriptors grow without bound over a long run

Source:      https://github.com/ikatson/rqbit/issues/525 (open)
Category:    memory
Priority:    P0
Effort:      L
Status:      open

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

**The harness is built and a 1.76 hour run is recorded. The six hour run the
acceptance asks for has not been completed, so this stays open.**

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

| series | steady, 2.26 h, 258 samples | idle, 2.55 h, 291 samples |
| --- | --- | --- |
| `rss_bytes` per hour | **+0.93 MiB**, r squared 0.65 | **+0.04 MiB**, r squared 0.01 |
| `rss_bytes` first, last, max | 14.75, 18.23, 20.19 MiB | 13.14, 12.49, 13.67 MiB |
| `handles` per hour | +2.09, r squared 0.015 | **0.00**, and 188 at every sample |
| `tcp_close_wait` max | **0** | **0** |
| leech cycles | 514 | none by design |

**The idle control is the new fact.** A seeder with no tracker and nothing
connecting holds 188 handles at every one of 291 samples over two and a half
hours, and its resident memory does not move: 0.04 MiB an hour at an r squared
of 0.01 is the sampler's own noise. So whatever the `steady` run is doing, it
is the load doing it and not the session's timers, and this entry's report of
descriptors climbing on their own does not reproduce at all.

**The `steady` slope is still not a straight line.** 0.93 MiB an hour at an r
squared of 0.65, with a maximum of 20.19 MiB against a last reading of 18.23,
is a series that rises and falls rather than one that climbs. Two and a half
hours cannot separate a settling curve from a leak, which is exactly what the
six hour run is for and why this stays open.

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
`soak-20260820T155309362Z` (idle, 2.55 h, the control), and
`soak-20260820T181505020Z` (steady, restarted at 18:15 UTC and still running
when the session ended, so its files are not committed).

The restarted run is the one to look for: if `.tmp/soak-steady` and its
bench files are still on the machine, read the JSON before starting another,
because a run that reached five hours is worth more than a fresh one.

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

Source:      PROMPT.md A3.11
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
