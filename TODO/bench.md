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
Status:      partial

Problem:     `bench leech|seed|webseed|swarm|probe` parse, appear in `--help`,
             and fail with a message pointing here. `webseed probe` covers part
             of what `bench webseed` should do, and nothing else exists.
Relevance:   Decision 7.12: `bit-cli` is a measurement instrument as well as a
             client, held to the same standard as the download path.
Approach:    Build in this order, because each reuses the last:
             1. The report envelope and environment capture (T-091), which
                every subcommand needs. **Done.**
             2. `bench webseed`, which is `webseed probe` plus the envelope,
                `--baseline`, and `--fail-under`. **Done.**
             3. `bench leech`, which is `download` plus the time series.
                **Done.**
             4. `bench seed`, which is `seed` plus the time series.
             5. `bench probe`, a one-shot reachability check. **Done.**
             6. `bench swarm`, the synthetic load generator, which is the
                largest and should come last. See [T-092](#t-092-bench-swarm-has-no-synthetic-load-generator).

             `bench disk` was added to this list after the fact, by
             [T-017](disk-io.md), which needed the disk measured on its own and
             found the envelope already there to put it in. **Done.**
Acceptance:  Each subcommand writes a report with the metrics A3.11 lists, and
             `--fail-under` set above the observed rate exits 14.

Done so far:

`bench webseed` is built. It reads real payload out of each source's scope and
drops it, so it measures the transport and nothing else: no piece is written,
no hash is checked, and no retry or cooldown runs, because a retry that hides a
failure also hides it from the measurement.

`--fail-under` exits 14 above the observed rate and 0 below it, against a
32 MiB payload on the loopback file server:

```
$ bit-cli bench webseed .tmp/bench/payload.torrent --web-seed $URL \
    --duration 2s --warmup 500ms --fail-under 100GiB/s --format text
threshold              100.00 GiB/s required, 4.23 GiB/s observed: not met
$LASTEXITCODE = 14

$ bit-cli bench webseed .tmp/bench/payload.torrent --web-seed $URL \
    --duration 2s --warmup 500ms --fail-under 1MiB/s --format text
$LASTEXITCODE = 0
```

Every metric A3.11 lists is in the report except the ones that only a peer
carries: choke and unchoke events, request queue depth, and piece verification
have fields and a recorder path, and `bench leech` and `bench seed` are what
will populate them.

`bench leech` is built. It is `download` with the clock and the counters on,
and it answers the question a rate on its own cannot: whether a slow download
was waiting on the network, on the hash, or on the disk. Three measurements
make that possible, and all three are taken from `bit-cli`'s own code rather
than modelled:

- **Verification.** `bit_cli_core::storage::SafeStorage` brackets each piece
  check. A check is a run of positioned reads walking the piece from its
  start, followed by the session declaring the piece complete, all on one
  thread with nothing awaited in between, so the wall time between the first
  of those reads and that declaration is the whole cost of the check, the
  SHA-1 included. It lands in `summary.hashing`.
- **The disk.** The same storage counts positioned reads and writes, their
  bytes, and their time. It lands in `summary.disk`. Two `Instant::now()`
  calls per operation, always on: a counter that is only on when someone is
  measuring measures a different program.
- **The request pipeline.** `BridgeStatus` counts the blocks the session has
  asked for and not yet been given, the deepest that ever got, and the total
  time from a request arriving to its block going back out. It lands in
  `summary.pipeline`, with `window_ceiling`: what a pipeline held at the peak
  depth would sustain at the measured service time.

Every one of those also appears per interval in `series[].costs`, and in the
CSV columns, so the shape over time is visible and not just the total.

Acceptance, 2026-08-20T04:06:06.879Z, release build, 1 GiB payload on the
loopback file server, five runs per step, medians:

```
$ pwsh -NoProfile -File scripts/bench-leech.ps1 -PayloadSize 1GiB -Runs 5 -ConnectionSweep "1,2,4,8"
```

Report: `bench/leech-20260820T040606879Z.json`.

| Stage | Median | Slowest | Fastest | Share of fetch |
| --- | --- | --- | --- | --- |
| `bench webseed`, no bridge | 855.90 MiB/s | | | 100.00% |
| `bench leech`, 1 bridge | 184.40 MiB/s | 169.73 MiB/s | 204.27 MiB/s | 21.55% |
| `bench leech`, 2 bridges | 314.69 MiB/s | 313.53 MiB/s | 340.20 MiB/s | 36.77% |
| `bench leech`, 4 bridges | 338.40 MiB/s | 313.53 MiB/s | 372.23 MiB/s | 39.54% |
| `bench leech`, 8 bridges | 292.07 MiB/s | 213.20 MiB/s | 340.09 MiB/s | 34.12% |
| control: 1 bridge, 64 requests in flight | 150.37 MiB/s | 126.33 MiB/s | 169.54 MiB/s | 17.57% |

These bridges are the same URL named N times, which is N separate sources.
That was the only way to get N connections when this ran.
[T-009](webseed.md) built `--web-seed-connections` and re-measured: the numbers
there are the shipped flag and they are the ones to quote, because N separate
sources at one URL keep N window caches and pull the payload nearly N times
over.

What that says is written up under
[T-001](webseed.md#the-measurement-bench-leech-took), because it is the
answer to that entry's question. In one line: the cost is the per-peer serial
receive path, not the request window, not hashing, and not the disk until
several paths contend for it.

`--fail-under` above the observed rate exits 14 on `leech` as it does on
`webseed`, covered by
`cmd::bench::tests::a_leech_below_the_threshold_exits_fourteen`.

One refusal was added while building it. A payload already sitting in the
output directory hash-checks clean on add, and the torrent is finished before
a byte is fetched. A rate taken from that run describes the hash checker, so
`bench leech` refuses it and names the directory. The benchmark script hit
exactly this when its own cleanup silently failed, which is how it was found.

`bench disk` is built. It writes a payload through the same
`bit_cli_core::storage::SafeStorage` a download writes through, from N threads,
with no session and no network, so the disk can be measured on its own instead
of inferred from a download doing four things at once. It was built for
[T-017](disk-io.md) and it answered that entry: writes to one file serialise
whatever handle they arrive on, and the serialisation is charged per operation
rather than per byte.

Three layouts make that readable, and the difference between two of them is the
whole measurement: `shared` is one file behind one handle, `handles` is the same
file and the same offsets behind one handle per thread, and `split` is one file
per thread. It fills the same envelope as every other subcommand, adds
`disk_steps` for the per-thread cost a concurrency curve cannot carry, and
exits 7 rather than 0 when a step reads back a block it did not write, because
that is a correctness failure and not a slow one.

```
$ bit-cli bench disk --payload-size 1GiB --concurrency-sweep 1,2,4,8 --format text

Writers
  THREADS  LAYOUT   FILES  RATE           WALL      FLUSH     WRITE TOTAL  MEAN WRITE  OVERLAP
  1        shared   1      2.27 GiB/s     440ms     821ms     423ms        6us         0.96
  2        shared   1      1.57 GiB/s     635ms     412ms     1s           18us        1.93
  4        shared   1      1.65 GiB/s     606ms     915ms     2s           34us        3.73
  8        shared   1      1.46 GiB/s     685ms     1s        4s           75us        7.22
```

`scripts/check-disk-contention.ps1` drives the sweep across all three layouts
and a block-size range, alternating the order so no layout always gets the disk
in the same state, and writes the medians and a verdict to
`bench/disk-contention-<timestamp>.json`.

`bench seed` is built. It is `seed` with the clock on, and every counter faces
the other way from `bench leech`: `uploaded_bytes` per peer rather than
`downloaded_bytes`, and positioned reads rather than writes, because a seeder's
storage cost is reading the payload back.

Three things a leech run has that this one does not, and saying so is the
point of the entry. There is no source list, because a seeder has no HTTP
sources: the rows are the peers. There is no pipeline depth, because the
request window belongs to the side asking. And there is no piece verification
inside the measured window: a seeder hash-checks the whole payload once on add
and then serves it, so `--include-hash-check` is what puts that read into the
report rather than leaving it before the clock starts.

Two refusals. Serving a payload that is not there at all is a missing payload
rather than a slow seeder, so it exits 2 and names the directory. A run where
nobody connected exits 6, the same code a leech run with no usable source
takes, because zero bytes with no peer is not a measurement.

`scripts/bench-seed.ps1` drives one seeder and N leechers on loopback, and the
record is `bench/seed-20260820T144744522Z.json` beside
`bench/bench-seed-20260820T144823484Z.json`:

```
$ pwsh -NoProfile -File scripts/bench-seed.ps1 -PayloadSize 256MiB \
    -Leechers 3 -Rate 8MiB/s -IncludeHashCheck
```

```
peer            kind sent       rate
127.0.0.1:50677 peer 245.94 MiB 6.96 MiB/s
127.0.0.1:50678 peer 246.11 MiB 6.97 MiB/s
127.0.0.1:50679 peer 246.20 MiB 6.97 MiB/s

sent 738.25 MiB at 20.90 MiB/s sustained, 24.09 MiB/s peak;
read 772.83 MiB off the disk over 49152 reads
read amplification: 1.047
hash check on add: 256 pieces, 256.00 MiB in 169ms at 1.48 GiB/s
```

**What that run measures is whether the seeder keeps up with three capped
leechers, not how fast it can go.** The cap is what makes a loopback transfer
last long enough for a one second metrics interval to sample it, and the
sustained rate is bounded by three times 8 MiB/s. Reading 20.90 MiB/s as a
capacity number would be reading the cap. The script says so in its header and
takes `-Rate 0` with a larger payload for the capacity run.

The number worth reading here is **read amplification, 1.047**: 772.83 MiB off
the disk to put 738.25 MiB on the wire, with three peers pulling the same
payload at once. Every byte was read about once, so nothing is re-reading a
piece for a second peer.

`--fail-under` above the observed rate exits 14, checked by hand at
`--fail-under 100GiB/s` against this fixture.

One change to the report envelope came with it. Rates were `Size`, so a field
named `rate` serialized `"human": "2.75 MiB"` where ground rule 0.2 says rates
carry `MiB/s`. `bit_cli_core::units::Rate` is the same wire shape with the
right string, so an older report still reads back and `--baseline` still
compares the same field. `a_rate_and_a_size_share_a_wire_shape_and_differ_in_the_string`
is the test.

Still open: `probe` and `swarm` refuse with exit 1 naming this entry, the same
as before. `bench swarm` is [T-092](#t-092-bench-swarm-has-no-synthetic-load-generator)
and is the largest of the three.


`bench probe` is built, which is step 5. It answers the question that comes
before "how fast": is the thing there, and what does it speak. It moves no
payload, so its report carries the environment and the facts and no time
series.

A target is a peer address or an HTTP endpoint, decided from the address
itself. Against a live `bit-cli seed` on loopback:

```
$ bit-cli bench probe 127.0.0.1:51999 --for pb.torrent --format text

Probe
  target               127.0.0.1:51999
  kind                 peer
  reachable            yes
  connect              1ms
  first response       0ms
  peer id              -rQ9000-1%ba%01%06%ad0%b4xM%f5%d0%7f
  client               rqbit 9000
  reserved             0000000000100000
  extensions           extension-protocol
  info hash            echoed
  says it is           bit-cli 0.1.0
  extension messages   ut_metadata, ut_pex
  messages             extended, bitfield, unchoke
  pieces advertised    10
```

Two things that output says which are worth reading twice. The wire peer id is
`librqbit`'s `-rQ9000-` while the extended handshake says `bit-cli 0.1.0`,
because the session is handed a client name and picks its own peer id. And the
reserved bytes claim BEP 10 and nothing else: no DHT bit, no fast extension.
Both are facts about what `bit-cli` puts on the wire, and neither was visible
from inside the tool before this.

Against an HTTP endpoint it is a one-byte ranged `GET`, redirects followed by
hand and reported hop by hop, with the TLS version and cipher when the scheme
is `https`:

```
$ bit-cli bench probe http://127.0.0.1:64341/pb/payload/blob.bin --format text
  status               206
  ranges               supported
  length               292.97 KiB
  http                 HTTP/1.1
```

`--for <SOURCE>` names the torrent a peer is asked about, as a `.torrent`, a
magnet, or an info hash. Without it the handshake carries a zero info hash, a
peer is entitled to hang up on it, and the report says so in a note rather
than leaving the reader to wonder.

A probe ends when the peer goes quiet rather than when the deadline expires.
A peer volunteers its greeting in one burst, and waiting out `--timeout` after
that made every probe cost ten seconds: 8.736s before, 0.546s after, for the
same three messages.

An unreachable target exits 6, `no_usable_sources`, which is what a script
branches on. Four tests cover it, all on loopback: a real seeder read off the
wire, an HTTP endpoint that answers a range, a port nothing listens on, and a
target that is neither.

**Building it found one thing in the fixtures.** `test_support::FileServer`
matched `Range: bytes=` exactly, and every HTTP client writes header names in
lower case, so it had never matched a range in its life: every ranged request
was answered with the whole file and a `200`. Small fixtures still verified,
which is what hid it. It now matches the name case insensitively, and the
probe's `range_support` is the assertion that would have caught it.

`bench swarm` is what is left, and it is [T-092](#t-092-bench-swarm-has-no-synthetic-load-generator).

### T-091 Bench reports do not capture their environment

Source:      PROMPT.md A3.11
Category:    bench
Priority:    P0
Effort:      M
Status:      **done**

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

`bit_cli_core::sysinfo` reads the machine and the process through the
platform's own interfaces rather than a crate. On Windows:
`K32GetProcessMemoryInfo`, `GetProcessTimes`, `GetProcessHandleCount`,
`GlobalMemoryStatusEx`, `RtlGetVersion`, and `GetIfTable`. On Linux:
`/proc/self/status`, `/proc/self/stat`, `/proc/self/fd`, `/proc/meminfo`,
`/proc/sys/kernel`, `/etc/os-release`, and `/sys/class/net`. The CPU model
comes from the `CPUID` brand string on x86, which is the same string on both
platforms and needs no filesystem and no registry.

Acceptance run, 2026-08-19T23:13:33.253Z, release build:

```
$ bit-cli bench webseed .tmp/bench/payload.torrent --web-seed $URL \
    --format json --duration 10s --warmup 2s --concurrency 8 --request-size 1MiB
```

```
started       2026-08-19T23:13:33.253Z
finished      2026-08-19T23:13:43.264Z
os            Windows 10.0.26200
cpu           12th Gen Intel(R) Core(TM) i7-12700H, 20 logical, x86_64
memory        63.63 GiB
link          Hyper-V Virtual Ethernet Adapter #2 at 1.00 Gbit/s;
              ZeroTier Virtual Port at 100.00 Mbit/s
build         0.1.0 x86_64-pc-windows-msvc release debug_assertions=false
peak_rss      42074112 (40.13 MiB)
cpu_ms        29859
open_handles  219
sustained     2.98 GiB/s
requests      24418 errors 0
series        9 samples, 1 in warmup
```

Two decisions worth recording, both made because the first draft reported a
number that was not true:

- A `dwSpeed` of `0xFFFFFFFF` from `GetIfTable` is the saturation value of the
  field, not a 4.29 Gbit/s link. Every NDIS filter layer and virtual adapter on
  a Windows box reports it. Those rows are dropped rather than repeated as a
  speed.
- `GetIfTable` returns every NDIS binding, so one ethernet port comes back once
  as itself and again for each filter driver over it, all sharing a physical
  address. Rows are deduplicated by MAC, keeping the shortest name, because a
  filter layer is named for its parent with a suffix appended.

`debug_assertions` is in the report because it is the difference between a
number and a number that means nothing, and nothing else in the report would
say so.

The Linux half of the acceptance is not run yet: this machine is Windows and
there is no Linux runner wired up. The code is there and the CI matrix in
`.github/workflows/ci.yml` builds Linux, so adding the assertion to CI is what
closes the gap. Recorded in [T-085](create-seed.md), which has the same shape.

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

The report envelope, the recorder, the warmup window, and the periodic metrics
are built and shared with `bench webseed`, so what is left is the load
generator itself.

### T-093 --baseline comparison is not implemented

Source:      PROMPT.md A3.11
Category:    bench
Priority:    P2
Effort:      S
Status:      **done**

Problem:     `--baseline <PATH>` parses and does nothing.
Relevance:   It is what turns a benchmark into a regression test.
Approach:    Read the prior report, compare every summary metric, and print the
             delta with a sign. Refuse to compare reports whose environment
             objects disagree on CPU model or OS, because that comparison is
             not meaningful.
Acceptance:  Two runs, the second with `--baseline` pointing at the first,
             print a delta per metric, and a comparison across different
             hardware refuses with a clear reason.

`bit_cli_core::bench::compare` produces a delta per metric with a sign, a
percentage, and a `higher_is_better` flag so a reader knows which way the sign
points. Fifteen metrics are compared: sustained and peak rate, bytes, requests,
errors, six latency percentiles, peak RSS, CPU time, open handles, and hash
rate where there is one.

A comparison is refused, with the reason named, when the baseline is a
different `bench` subcommand, when its report version is newer than this build
understands, or when the two hosts disagree on CPU model, logical core count,
or OS name. Kernel patch level and total memory do not refuse a comparison: an
OS update is worth measuring across, and a different machine is not.

The refusal is a note in the report and a warning on stderr rather than a
failure, because a run that measured something should not be thrown away
because the baseline beside it was wrong.

Acceptance, run in-process by the test suite:

```
$ cargo test -p bit-cli --lib cmd::bench::tests
```

covering `a_report_written_to_a_file_reads_back_as_a_baseline`,
`a_baseline_from_other_hardware_is_refused_and_the_run_still_reports`, and
`a_baseline_that_is_not_a_report_names_the_file`, plus the unit tests in
`bench::report::tests` for each refusal.

Making this work needed one fix elsewhere: `units::Size` and `units::Millis`
serialized as `{bytes, human}` but deserialized only from a bare integer, so no
document `bit-cli` wrote could be read back. Both now accept either form.

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

The report already carries `environment.tracing_enabled` and
`environment.trace_subsystems`, so a report taken with a trace on is
distinguishable from one taken without. The measurement itself is what is left.

### T-148 The peer probe test asserted an exit code inside its own retry loop

Source:      CI run 32407214253, `Test (ubuntu-latest)`, 2026-08-20
Category:    bench
Priority:    P2
Effort:      S
Status:      **done**

Problem:     `cmd::bench::tests::a_peer_probe_reads_the_handshake_and_what_follows_it`
             starts a real seeder on a thread and dials it. It cannot know when
             the listener is up, so it retries, which is right. The retry went
             through the `report` helper, which asserts an exit code:

             ```
             left: NoUsableSources
              right: Success
             ```

             A dial that arrives before the listener binds exits 6, and that is
             `bench probe` working: `an_unreachable_peer_exits_no_usable_sources`
             asserts the same code on purpose. So the first attempt panicked
             and the loop never ran a second one. It passed on Windows and on
             macOS because the seeder happened to bind first.
Relevance:   A test that fails on whichever machine is slower is a test that
             teaches everyone to re-run CI. It also hid the two real failures
             beside it in the same job, [T-147](windows.md).
Approach:    Run the command without asserting inside the loop, treat any
             non-`Success` exit as "not up yet", and assert once at the end
             that a probe connected. Bound the loop by a deadline rather than
             by a count of attempts: 40 attempts is four seconds on this
             machine and an unknown number on a loaded runner.
Acceptance:  The test passes on `ubuntu-latest`, `windows-latest`, and
             `macos-latest` in the same run, and fails with a message naming
             the port when the seeder never binds.

The deadline is eight seconds against the seeder's own `--stop-after 10s`, so
the loop cannot outlive its subject.
