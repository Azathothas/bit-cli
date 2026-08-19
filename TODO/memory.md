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
