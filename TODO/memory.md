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
Status:      open

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
