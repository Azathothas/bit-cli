# Performance and throughput

Thirty-one issues touch piece picking, pipelining, block size, endgame, and
buffering.

---

### T-030 Throughput collapses with several torrents at once

Source:      https://github.com/ikatson/rqbit/issues/590 (open)
Category:    performance
Priority:    P0
Effort:      L
Status:      **done**

Problem:     Two reports in one: adding several torrents slows all of them well
             past what sharing a link explains, and a single large torrent
             (over 4 GB) shows start-stop-start-stop behaviour where the rate
             drops to zero and only a pause and resume clears it.
Relevance:   `-j` exists to run several sources in one invocation. If that is
             slower than running them one at a time, the flag is a trap.
Approach:    Measure before theorising. Three runs with the same total payload:
             one torrent alone, three torrents at `-j 1`, three at `-j 3`. If
             `-j 3` is slower than `-j 1` in wall time for the same bytes, the
             contention is real and the next question is where: the tokio
             blocking pool, the disk, or the peer connection budget.
Acceptance:  A `bench/multi-torrent-<timestamp>.json` report with the three
             wall times, peak RSS, and CPU time, and this entry naming which
             resource saturated.

**The first report was real and it was two defects, neither of them
contention.** Both are fixed. `-j 4` now moves four torrents 3.54 times faster
than running them one invocation at a time, at 72% of what the HTTP source
serves with no torrent machinery at all. The second report, the intermittent
stall, reproduced once and is [T-037](#t-037-a-run-stalls-for-minutes-roughly-once-in-fifty).

`scripts/check-multi-torrent.ps1` is the measurement. Six modes rather than the
three the acceptance asks for, because three cannot separate what the extra
processes cost from what the shared session costs, and cannot say whether `-j`
bought concurrency or bought connections:

| Mode | What it runs |
| --- | --- |
| `one` | One torrent, one invocation. The per-torrent rate with nothing to share. |
| `serial` | N torrents, N invocations, one after another. What a caller who avoided `-j` would pay, process startup included. |
| `j1` | N torrents, one invocation, `-j 1`. Same session, one download at a time. |
| `j2`, `j4` | N torrents, one invocation, at each step of the sweep. |
| `control` | One torrent at a time with as many connections as the deepest sweep step has in total. |

Every mode moves the same bytes off the same loopback server, and the run
starts by measuring what that server serves through `bit-cli`'s own HTTP path
with no bridge, no hashing, and no disk. Without that ceiling a rate says
nothing: a mode that reaches it is describing the server.

Acceptance, four torrents of 256 MiB, three iterations, medians,
2026-08-20T08:07:01.379Z. Report: `bench/multi-torrent-20260820T080701379Z.json`.

```
$ pwsh -NoProfile -File scripts/check-multi-torrent.ps1 -Torrents 4 -PayloadSize 256MiB -Runs 3

ceiling:  808.84 MiB/s through bit-cli's own HTTP path, no bridge, no hashing, no disk

mode    wall  bytes      rate         of ceiling peak RSS   CPU ms handles
one     1.46s 256.00 MiB 175.95 MiB/s 21.75%     43.61 MiB    2124     220
serial  6.24s 1.00 GiB   164.02 MiB/s 20.28%     44.48 MiB    8605     228
j1      6.18s 1.00 GiB   165.78 MiB/s 20.50%     48.49 MiB    8468     227
j2      3.01s 1.00 GiB   340.20 MiB/s 42.06%     74.09 MiB    9061     242
j4      1.76s 1.00 GiB   580.17 MiB/s 71.73%     114.24 MiB  10656     264
control 2.97s 1.00 GiB   344.32 MiB/s 42.57%     107.59 MiB  15108     289
```

**Which resource saturated: none of `bit-cli`'s.** `-j 4` runs at 71.73% of
what the file server itself serves. Attributing the remaining 28% needs a
faster source than this machine can run beside the client, so the honest answer
is that the measurement ran out of server before it ran out of `bit-cli`.

**`-j` buys concurrency, not connections.** That is what `control` is for.
`-j 4` gives four torrents four connections each, sixteen in flight. Putting
those same sixteen on one torrent at a time reaches 344 MiB/s where `-j 4`
reaches 580, so the flag is worth 1.69 times what the connections alone are
worth.

**Memory scales with the flag and nothing else does.** Peak RSS goes 48.49,
74.09, 114.24 MiB across `-j 1`, `-j 2`, `-j 4`, which is about 22 MiB per
concurrent torrent. Handles go 227, 242, 264, which is about twelve per
concurrent torrent. CPU is flat at 8.5 to 10.7 seconds for the same gigabyte.
Those are the numbers [T-040](memory.md) needs.

## The two defects

### One: completion was noticed on the next report tick

`download`'s watch loop woke only on `--report-interval`, which defaults to one
second, and completion was checked after the tick. So a torrent that finished
1.1 seconds in was noticed at 2.0 seconds, and `-j 1` with four torrents paid
that four times. `--timeout` and `--stop-after` had the same problem and would
fire up to a second late.

The loop now wakes on three things: the tick, the torrent completing, and the
earliest deadline the caller set. `should_stop` still decides what any of them
means, so a seeding run that keeps going after completion is unchanged.

Measured on its own: the same script, the same fixture, the same machine, with
the path fix below already in and the completion and deadline branches of the
`select!` the only difference. Tick-only report:
`bench/multi-torrent-20260820T081542263Z.json`.

| Mode | Woken by the tick only | Also woken by completion | Gain |
| --- | --- | --- | --- |
| `one` | 2.08s | 1.46s | 1.42x |
| `serial` | 8.28s | 6.24s | 1.33x |
| `j1` | 8.12s | 6.18s | 1.31x |
| `j2` | 4.08s | 3.01s | 1.36x |
| `j4` | 2.07s | 1.76s | 1.18x |
| `control` | 5.11s | 2.97s | 1.72x |

The shape is what the explanation predicts. `-j 1` runs four batches and saves
1.94 seconds, which is four times the half-second a uniformly distributed
finish loses to a one-second tick. `-j 4` runs one batch and saves about one
tick's worth. `one` is a single 1.46-second download that was taking 2.08
seconds, which is the whole of the difference.

### Two: a multi-file torrent with one file lost its directory

This is the one that made "several torrents" look like contention, and it is a
correctness bug rather than a slow one. It is written up as
[T-036](#t-036-a-multi-file-torrent-with-one-file-lands-without-its-directory).
In short: four torrents were writing to one file, so the run was paying the
per-file write serialisation [T-017](disk-io.md) measured, and three of the
four payloads were being destroyed while all four reported success.

## What was checked and did not explain it

- **Ephemeral ports.** Ten alternating `-j 4` and `-j 2` runs moved
  `TimeWait` from 276 to 500 against a 16,384-port dynamic range, and
  `CloseWait` stayed at zero throughout. Not the port table.
- **The `-j` semaphore.** The permit is bound to a named local and held for the
  whole download, so it is released when the worker ends and not before.
- **The file server.** It is measured every run and reported as `ceiling`, so a
  mode that approaches it is visible rather than being read as a `bit-cli`
  limit.


### T-031 The rate limit did not apply to the session

Source:      https://github.com/ikatson/rqbit/issues/391 (closed, 2025-06-10)
Category:    performance
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `SessionOptions::ratelimits` was reported not to take effect.
Relevance:   `--max-download-rate` and `--max-upload-rate` go straight into
             `LimitsConfig`. If that is ignored, the flags are decorative, and
             rule 0.10 says a knob that does not move a number does not ship.
Approach:    The issue is closed upstream, which suggests the pinned 9.0.0
             carries the fix, but "closed" is not "verified here". Measure it:
             download a known payload with and without a cap and compare the
             sustained rate.
Acceptance:  `bit-cli download <TORRENT> --max-download-rate 1MiB/s` sustains
             within 10 percent of 1 MiB/s over 60 seconds, and the same run
             uncapped is meaningfully faster. Both numbers recorded here.
Closed:      Both caps hold, and the pinned 9.0.0 does carry the fix.
             `pwsh -NoProfile -File scripts/check-rate-limit.ps1` is the
             measurement: one seeder, one 128 MiB payload, three paired runs
             alternating order, peers only.

             ```
             run mode     exit wall  bytes      rate
               1 capped      0 31.2s 128.00 MiB 4.10 MiB/s
               1 uncapped    0 0.6s  128.00 MiB 220.31 MiB/s
               2 uncapped    0 0.6s  128.00 MiB 229.39 MiB/s
               2 capped      0 31.2s 128.00 MiB 4.10 MiB/s
               3 capped      0 31.2s 128.00 MiB 4.10 MiB/s
               3 uncapped    0 0.6s  128.00 MiB 223.39 MiB/s

             with the seeder capped at 4MiB/s and the downloader uncapped: 4.01 MiB/s
             ```

             `--max-download-rate 4MiB/s` sustains 4.10 MiB/s, 2.5% over the
             cap and inside the 10% the acceptance asks for, against 223.39
             MiB/s uncapped, which is 54 times faster. The other direction is
             the same `LimitsConfig` field seen from the other end: with the
             seeder started under `--max-upload-rate 4MiB/s` and the
             downloader uncapped, the transfer comes out at 4.01 MiB/s.

             The rate is computed from the wall clock and the bytes the report
             says landed rather than from the report's own mean, so the
             limiter is not measured by the thing it limits. Each run gets a
             fresh output directory, because reusing one lets the hash check
             on add find the payload already there and report the disk.

             What this does **not** cover is `--max-overall-download-rate` and
             `--max-overall-upload-rate` across several torrents in one
             invocation, and the asymmetry that
             [T-132](multi-source.md) is about: a session cap applies to peers
             and to HTTP sources together, because a source reaches the
             session as a peer.

### T-032 The piece selector strategy is not implemented

Source:      PROMPT.md A3.6
Category:    performance
Priority:    P1
Effort:      L
Status:      open

Problem:     `--piece-selector rarest-first|sequential|in-order|random` parses
             and is carried through the config, and none of the four reaches
             `librqbit`'s picker, which is rarest-first and not configurable.
Relevance:   Sequential is what makes streaming work, and it is the difference
             between a usable and an unusable `bit-cli download | vlc -`.
Approach:    `librqbit` has a `FileStream` type and streaming support, which
             suggests some ordering control exists; find it before assuming a
             fork is needed. If the picker is genuinely fixed, this needs
             either an upstream API or Candidate C.
Acceptance:  `bit-cli download <TORRENT> --piece-selector sequential --jsonl`
             emits `piece_verified` events whose indices are non-decreasing for
             at least the first 90 percent of the run.

### T-033 --split, -x, and -k do not reach the fetch path

Source:      PROMPT.md A3.6
Category:    performance
Priority:    P2
Effort:      M
Status:      open

Problem:     `-s/--split`, `-x/--max-connection-per-server`, and
             `-k/--min-split-size` parse and do nothing. They are the aria2
             flags a migrating script will already be passing.
Relevance:   Rule 0.10 again. Three flags that look like they work and do not
             are worse than three flags that error.
Approach:    All three are about how one HTTP source is fetched in parallel.
             `--web-seed-concurrency` and `--web-seed-chunk-size` already
             express the same two ideas, so the honest wiring is to make `-x`
             an alias of `--web-seed-concurrency`, `-k` a floor on
             `--web-seed-chunk-size`, and `-s` the segment count per source,
             then prove each moves a number.
Acceptance:  `bench/split-<timestamp>.json` shows throughput at `-x 1`,
             `-x 4`, and `-x 16` against one mirror, with the curve recorded
             here. If the curve is flat, the flags do not ship.

### T-034 Endgame mode is not observable

Source:      corpus, performance category
Category:    performance
Priority:    P3
Effort:      M
Status:      open

Problem:     The last few pieces of a download are the ones that decide the
             wall time, and nothing in the report says whether endgame
             duplication happened or how long the tail took.
Relevance:   `bench leech` is meant to answer "is my server serving well". A
             run whose last piece took 40 seconds looks the same as one that
             was uniformly slow.
Approach:    Record time to 90, 99, and 100 percent separately in the download
             report, and the number of pieces requested from more than one
             source.
Acceptance:  `bit-cli download --json` carries `p90_ms`, `p99_ms`, and
             `total_ms` for progress, and the tail is visible as the difference.

### T-035 The web seed rate limit was never applied

Source:      the [T-003](webseed.md) hybrid measurement
Category:    performance
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `--web-seed-speed-limit`, and `rate_limit` in a binding table,
             parsed, validated, and reached `SourceLimits.rate_limit`. Nothing
             read it. A source told to stay under 24 MiB/s ran at 116 MiB/s.
Relevance:   It is the flag an operator uses to leave headroom on a mirror they
             do not own. A cap that is accepted and ignored is worse than one
             that is refused, because the caller believes they set it.
Approach:    A token bucket per source in `webseed::fetch::Fetcher`, refilled
             continuously, holding one second of burst. Tokens are taken before
             a request goes out rather than after its bytes arrive: a limiter
             that lets the bytes land and then sleeps has not limited anything
             the mirror can see.

             Two details worth keeping. The bucket may go negative, because a
             request larger than a second of burst can never be satisfied from
             a full bucket and taking what it needs and waiting out the deficit
             is what keeps the average right rather than deadlocking. And the
             cap is on bytes off the mirror, so it is taken where a window is
             fetched and not where a block is served: a block answered from the
             window cache crossed no wire.
Acceptance:  A 256 MiB payload under `--web-seed-speed-limit 24MiB/s` takes
             about ten seconds rather than one, and the unit tests pace the
             bucket on a paused clock.

Found while building the [T-003](webseed.md) acceptance, which needed a slow
mirror and did not get one. Under `--web-seed-speed-limit 24MiB/s`, a 256 MiB
payload took 1,114 ms before the fix, about 116 MiB/s, and 10,192 ms after it,
about 25 MiB/s. Reproduce either with:

```
$ bit-cli download <TORRENT> --web-seed <URL> --web-seed-only \
    --web-seed-speed-limit 24MiB/s --json
```

The acceptance run itself is uncapped, because a cap decides the split by
itself, so the committed report under `bench/` does not carry this number.

The unit tests are `webseed::fetch::tests::a_rate_limit_paces_after_the_first_second_of_burst`,
which times the bucket on tokio's paused clock so the assertion is about the
delay the limiter asked for rather than how busy the machine was, and
`a_source_limit_becomes_a_fetcher_rate`, which proves the cap reaches the
fetcher from the spec.

Making the bucket testable needed one decision: it reads `tokio::time::Instant`
rather than `std::time::Instant`, so it refills on the same clock its own
sleeps advance. Outside a test the two are the same clock. Under a paused one
they are not, and a limiter that refills on a clock its sleeps do not advance
cannot be tested at all.

This is not [T-031](#t-031-the-rate-limit-did-not-apply-to-the-session), which
is the session-wide `--max-download-rate` and `--max-overall-download-rate`.
That one is still open.

---

### T-036 A multi-file torrent with one file lands without its directory

Source:      the [T-030](#t-030-throughput-collapses-with-several-torrents-at-once) measurement
Category:    paths
Priority:    P0
Effort:      S
Status:      **done**

Problem:     `SafeStorage` decided whether a torrent unpacks into a directory
             of its own by counting files rather than by asking whether the
             metainfo carries a `files` list. BEP 3 makes `name` the file's
             name in the single-file case and the directory's name in the
             multiple-file case, and a `files` list holding one entry is still
             the multiple-file case. So a torrent named `album` whose one file
             is `movie.bin` wrote `movie.bin` into the output directory instead
             of `album/movie.bin`.
Relevance:   P0 because it loses data silently. Two such torrents in one
             `download` invocation whose one file has the same name write the
             same path, and both report success: each hash-checks its own
             pieces as it writes them, so each check passes at the moment it
             runs and the bytes are gone afterwards. It is the same failure
             [T-072](windows.md) fixed for names that collide only on NTFS,
             reached by a different route.
Acceptance:  A torrent with a one-entry `files` list unpacks into its own
             directory, a torrent with no `files` list does not, and two of the
             first kind carrying the same file name both land intact.

The fix is one line in `storage::subfolder_for`: the multiple-file case is
`metadata.info.info().files.is_some()`, not `file_infos.len() >= 2`. Everything
else about the function is unchanged, and for a torrent with two or more files
the behaviour is identical, because such a torrent always has the list.

`bit-cli info` already reported this correctly, which is what made the
discrepancy findable: the same torrent reads `"multi_file": true,
"file_count": 1`.

`aria2c` 1.37.0 is the external check. Given the same torrent it creates the
directory:

```
$ aria2c --dir=out payload0.torrent
$ find out -type f
out/payload0/movie.bin
```

Before the fix, `bit-cli download` on four such torrents in one invocation:

```
$ find out -type f
out/movie.bin
```

One file, 128 MiB, for four torrents of 128 MiB each, and the run reported
`"completed": 4, "failed": 0`. After:

```
$ find out -type f
out/payload0/movie.bin
out/payload1/movie.bin
out/payload2/movie.bin
out/payload3/movie.bin
```

and every one hashes equal to its source.

Three tests in `crates/bit-cli-core/tests/hostile_paths.rs` hold it, and the
first two are a pair because either half alone would pass with the rule
inverted:

```
$ cargo test -p bit-cli-core --test hostile_paths
test a_one_file_multi_file_torrent_still_gets_its_directory ... ok
test a_single_file_torrent_gets_no_directory_of_its_own ... ok
test two_one_file_torrents_with_the_same_file_name_do_not_collide ... ok
test result: ok. 11 passed; 0 failed
```

The third drives the failure end to end: one session, one output directory, two
torrents whose single file is `movie.bin` in both, and both files present
afterwards.

`scripts/interop-roundtrip.ps1` passes against `aria2c` 1.37.0 and `rqbit`
9.0.0 after the change, which is what says the layout still matches what other
clients produce.

---

### T-037 A run stalls for minutes, roughly once in fifty

Source:      the [T-030](#t-030-throughput-collapses-with-several-torrents-at-once) measurement
Category:    performance
Priority:    P1
Effort:      M
Status:      open, not reproducible on demand

Problem:     One `-j 2` run of four 128 MiB torrents took 274,546 ms where the
             same command usually takes about 3,200 ms. It completed, and every
             byte arrived. CPU time over that run was 5,155 ms, so the process
             was waiting rather than working for four and a half minutes. The
             run is in `bench/multi-torrent-20260820T071833862Z.json` under
             `runs`, taken before either [T-030](#t-030-throughput-collapses-with-several-torrents-at-once)
             fix and therefore with a shorter `commands` list than the script
             writes now.
Relevance:   This is the second half of what [T-030](#t-030-throughput-collapses-with-several-torrents-at-once)
             reports: "start-stop-start-stop behaviour where the rate drops to
             zero and only a pause and resume clears it". The first half is
             fixed and measured; this is not.
Approach:    It has been seen once in about seventy runs and has not been
             reproduced deliberately. What has been ruled out:

             - **Ephemeral ports.** Ten alternating `-j 4` and `-j 2` runs
               moved `TimeWait` from 276 to 500 against a 16,384-port dynamic
               range, with `CloseWait` at zero throughout.
             - **A repeat of the same shape.** Sixty runs stepping `-j` from 1
               to 4 with `--log-level info` produced no run over 8.1 s, and
               that one is explained by the reconnect backoff below.
             - **The `-j` semaphore.** The permit is held for the whole
               download and released when the worker ends.

             What is worth trying next, in order:

             1. The bridge's reconnect backoff is `RECONNECT_BASE` 1 s doubling
                to `RECONNECT_MAX` 30 s, and it never gives up on a link
                failure. Nine consecutive failures is 274 s, which is the
                observed number. Recording every reconnect in the report, with
                the reason, would say whether that is what happened. The
                8,144 ms run in the sixty-run sweep is the same signature at
                three failures.
             2. If it is the reconnect loop, the question becomes why the link
                fails: the bridge dials the session's own listener, and the
                session, the listener, and the torrent are all live by then.
             3. A bound on the loop. A bridge that has reconnected N times
                without serving a byte is not going to, and it should say so
                and fail rather than retry until the run's deadline.
Acceptance:  Either a deliberate reproduction with the log showing where the
             time went, or a bridge that reports its reconnect count and reason
             in `--json` plus a run of at least two hundred invocations with
             none over five times the median. The report and the command go
             here either way.
