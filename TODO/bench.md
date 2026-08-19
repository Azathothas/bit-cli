# Measurement, load generation, and telemetry

Twenty-six issues touch metrics and statistics. This file is mostly forward
work: `bench` is a first-class deliverable by decision 7.12 and it is the
largest unbuilt piece.

---

### T-090 bit-cli bench is not implemented

Source:      PROMPT.md A3.11
Category:    bench
Priority:    P0
Effort:      XL
Status:      open

Problem:     `bench leech|seed|webseed|swarm|probe` parse, appear in `--help`,
             and fail with a message pointing here. `webseed probe` covers part
             of what `bench webseed` should do, and nothing else exists.
Relevance:   Decision 7.12: `bit-cli` is a measurement instrument as well as a
             client, held to the same standard as the download path.
Approach:    Build in this order, because each reuses the last:
             1. The report envelope and environment capture (T-091), which
                every subcommand needs.
             2. `bench webseed`, which is `webseed probe` plus the envelope,
                `--baseline`, and `--fail-under`.
             3. `bench leech`, which is `download` plus the time series.
             4. `bench seed`, which is `seed` plus the time series.
             5. `bench probe`, a one-shot reachability check.
             6. `bench swarm`, the synthetic load generator, which is the
                largest and should come last.
Acceptance:  Each subcommand writes a report with the metrics A3.11 lists, and
             `--fail-under` set above the observed rate exits 14.

### T-091 Bench reports do not capture their environment

Source:      PROMPT.md A3.11
Category:    bench
Priority:    P0
Effort:      M
Status:      open

Problem:     "A benchmark without its environment recorded is not a result, and
             the `--baseline` comparison is meaningless without it."
Relevance:   Comparing two numbers taken on different machines, or before and
             after a kernel update, without knowing that is how a benchmark
             lies.
Approach:    Capture `bit-cli` version and build metadata (the target triple is
             already recorded by `build.rs`), OS and kernel version, CPU model
             and logical count, total memory, NIC link speed where obtainable,
             the exact command line, and start and end timestamps in ISO 8601
             UTC with millisecond precision. Peak RSS, CPU time, and handle
             count come from [T-042](memory.md).
Acceptance:  `bit-cli bench webseed <TORRENT> --format json` carries an
             `environment` object with every field above populated on Windows
             and on Linux.

### T-092 bench swarm has no synthetic load generator

Source:      PROMPT.md A3.11, `superseedr/src/synthetic_load.rs`
Category:    bench
Priority:    P1
Effort:      XL
Status:      open

Problem:     `bench swarm` is meant to generate synthetic peers and torrents to
             load a target. Nothing exists.
Relevance:   It is how the operator answers "where does my seeding
             infrastructure fall over".
Approach:    `superseedr`'s `synthetic_load.rs` is 5,748 lines and GPL-3.0:
             read it for the shape, do not copy it. The shape worth taking is
             the warmup window, the bounded disk budget, the adaptive step
             search toward a target rate, and periodic metrics. Three hard
             requirements: the disk budget is enforced and never exceeded,
             generated payload lives in the scratch directory and is cleaned
             up, and the tool refuses to load-test a host it was not explicitly
             pointed at.
Acceptance:  `bit-cli bench swarm <TARGET> --peers 100 --torrents 4
             --disk-budget 2GiB --duration 60s` completes, never exceeds 2 GiB
             on disk, cleans up, and refuses to run without an explicit target.

### T-093 --baseline comparison is not implemented

Source:      PROMPT.md A3.11
Category:    bench
Priority:    P2
Effort:      S
Status:      open

Problem:     `--baseline <PATH>` parses and does nothing.
Relevance:   It is what turns a benchmark into a regression test.
Approach:    Read the prior report, compare every summary metric, and print the
             delta with a sign. Refuse to compare reports whose environment
             objects disagree on CPU model or OS, because that comparison is
             not meaningful.
Acceptance:  Two runs, the second with `--baseline` pointing at the first,
             print a delta per metric, and a comparison across different
             hardware refuses with a clear reason.

### T-094 Trace output has no measured cost

Source:      PROMPT.md A3.12
Category:    bench
Priority:    P2
Effort:      S
Status:      open

Problem:     "Tracing never changes behaviour or timing in a way that
             invalidates a measurement. If enabling a trace costs measurable
             throughput, say so in the docs and in the bench report."
             Nobody has measured it.
Relevance:   `--trace http` records every request in memory. On a long run that
             is both memory and time.
Approach:    Run `bench webseed` with and without `--trace http` and compare
             sustained throughput and peak RSS.
Acceptance:  Both numbers recorded here, and if the cost is measurable, the
             bench report carries a `tracing_enabled` field and the docs say
             what it costs.
