# Web seeds

The headline feature. Everything here is about HTTP sources attached to a
torrent at runtime.

Upstream `rqbit` has one issue on this subject, [#500][500], which is where the
StarCitizenToolBox patch series came from. That is not a gap in the triage: web
seeding is what `bit-cli` adds, so almost all of the work below is design work
rather than a defect list.

[500]: https://github.com/ikatson/rqbit/issues/500

---

## The section 2.2 benchmark

### T-001 Measure the loopback bridge against a raw curl ceiling

Source:      PROMPT.md section 2.2 (the operator's decision gate)
Category:    webseed
Priority:    P0
Effort:      M
Status:      **done**

Problem:     `bit-cli` presents an HTTP source to `librqbit` as a peer over a
             loopback TCP connection (Candidate A). That costs a round trip, a
             second copy of every byte, peer protocol framing, and a peer slot.
             Nobody has measured what it costs in practice, so nobody knows
             whether removing it is worth anything.
Relevance:   Every architectural decision about the fetch path sits on this
             number. Without it, Phase B has nothing to compare against.
Approach:    Same torrent, same mirror, same machine, same session, back to
             back. Disable DHT, PEX, LSD, and trackers so HTTP is the only
             source. Pin concurrency, pin the output directory to one
             filesystem, drop the page cache between runs. Record wall time
             (minimum of five runs, with the spread), sustained MiB/s as a
             percentage of a `curl` baseline against the same URL, peak RSS,
             total CPU time, bytes copied per payload byte, and time to first
             verified piece. Then the same under a source that stalls mid
             transfer and one that returns 416.
Acceptance:  `scripts/bench-webseed.ps1` runs and writes a report to
             `bench/webseed-<timestamp>.json` carrying every metric above with
             ISO 8601 UTC millisecond timestamps, and this file records
             Candidate A's throughput as a percentage of the `curl` ceiling.

## The measurement

`scripts/bench-webseed.ps1` takes the same payload from the same server four
ways, in one session, on one machine. Four rather than two, because one ratio
would say "slower" without saying where:

| Stage | What it is |
| --- | --- |
| `serial` | `curl`, one connection, one request for the whole file. |
| `parallel` | `curl`, N connections, one contiguous slice each. The ceiling. |
| `fetch` | `bit-cli bench webseed`. bit-cli's HTTP path, no bridge, no hashing, no disk. |
| `download` | `bit-cli download --web-seed-only`. Fetch, bridge, verify, write. |

The ceiling is the parallel one. Both bit-cli stages open eight connections,
and comparing eight against one would have been wrong in bit-cli's favour,
which is the wrong direction to be wrong in. Both `curl` stages write to the
null device, because `fetch` discards too; the disk cost lands in the gap
between `fetch` and `download` where it belongs.

## The number

Loopback, 256 MiB payload, 256 pieces, 1 MiB requests, concurrency 8, five
runs per stage, release build, 2026-08-20T00:18:28.470Z. Report:
`bench/webseed-20260820T001828470Z.json`.

```
$ pwsh -NoProfile -File scripts/bench-webseed.ps1 -PayloadSize 256MiB -Runs 5
```

| Stage | Rate | Share of ceiling | Wall, minimum | Spread |
| --- | --- | --- | --- | --- |
| `curl`, 1 connection | 1.75 GiB/s | 69.93% | 143 ms | 23.78% |
| `curl`, 8 connections | 2.50 GiB/s | 100.00% | 100 ms | 38.00% |
| `bit-cli` fetch, no bridge | 1.06 GiB/s | 42.34% | n/a | n/a |
| `bit-cli` download, bridge | 164.00 MiB/s | **6.41%** | 1561 ms | 2.24% |

**Candidate A reaches 6.41% of the `curl` ceiling on loopback.** It reaches
15.13% of bit-cli's own HTTP path with the bridge removed. The mirror run
below puts the same two ratios at 30.11% and 19.22%, and the second of those is
the one that matters.

The rest of the metrics the entry asks for:

| Metric | Value |
| --- | --- |
| Peak RSS, download | 64,208,896 B (61.23 MiB) |
| Peak RSS, fetch | 23,633,920 B (22.54 MiB) |
| CPU time, download, minimum | 1,937 ms over 1,561 ms of wall time |
| CPU time, fetch | 2,796 ms |
| Open handles, download | 265 |
| Loopback bytes per payload byte | 1.000793 |
| Time to first verified piece | 103 ms minimum, 108 ms median, 123 ms worst |

## The same measurement against a real mirror

Loopback has no network cost, so it is the worst case for `bit-cli` and the
best case for `curl`. The obvious reading of the table above is that the wire
is the bottleneck long before the bridge is, and the mirror run says that
reading is wrong.

Arch Linux ISO, 1.49 GiB, 3047 pieces, two runs per stage, release build,
2026-08-20T01:18:04.998Z. Report: `bench/webseed-20260820T011804992Z.json`.

```
$ pwsh -NoProfile -File scripts/bench-webseed.ps1 `
    -Mirror https://geo.mirror.pkgbuild.com/iso/2026.08.01/ `
    -TorrentUrl https://geo.mirror.pkgbuild.com/iso/2026.08.01/archlinux-2026.08.01-x86_64.iso.torrent `
    -Runs 2
```

| Stage | Rate | Share of ceiling | Wall, minimum | Spread |
| --- | --- | --- | --- | --- |
| `curl`, 1 connection | 33.55 MiB/s | 114.60% | 45,395 ms | 343.32% |
| `curl`, 8 connections | 29.28 MiB/s | 100.00% | 52,024 ms | 42.77% |
| `bit-cli` fetch, no bridge | 45.88 MiB/s | **156.71%** | n/a | n/a |
| `bit-cli` download, bridge | 8.82 MiB/s | 30.11% | 172,757 ms | 10.63% |

Peak RSS 43.07 MiB, 306 handles, first verified piece at 768 ms. Only the
named mirror is used: the torrent's own `url-list` carries 468 entries and
leaving them in would measure the internet rather than the mirror.

Two things change here.

**`bit-cli`'s HTTP path beats `curl` over a real network**, at 156.71% of the
reference. Many small ranged requests over pooled connections adapt to a
varying link; eight fixed slices do not, and the whole transfer waits for the
slowest one. The `curl` single-connection spread of 343% is the same effect
seen directly: 45 s on one run and 201 s on the next. So the reference stops
being a ceiling over a real network, and the script says so rather than
printing a percentage above a hundred and leaving it.

**The bridge is a hard limiter, not a constant overhead.** On loopback the
download path reached 164.00 MiB/s. Against a mirror that supplies 45.88 MiB/s
to the same client on the same machine, it reaches 8.82 MiB/s: 19.22% of what
`bit-cli`'s own fetch path gets. If the bridge were a fixed CPU cost it would
disappear at these rates. It does not, which means the limit is latency
sensitive, and the shape of that is a bounded number of requests in flight.

## What the numbers say together

| | loopback | mirror |
| --- | --- | --- |
| fetch, no bridge | 1.06 GiB/s | 45.88 MiB/s |
| download, bridge | 164.00 MiB/s | 8.82 MiB/s |
| bridge share of fetch | 15.13% | 19.22% |

The share is roughly constant across a 24-fold difference in available
bandwidth, which is what a pipeline-depth limit looks like and is not what a
per-byte cost looks like. A per-byte cost would take a smaller share as the
network got slower.

So: **the bridge costs about five sixths of the available throughput, at both
ends of the range measured.** 8.82 MiB/s is 0.07 Gbit/s, which does not
saturate this machine's 1.00 Gbit/s interface. The loopback number alone would
have said it did.

The 1.000793 figure is the framing, and it is not the cost: every block the
bridge hands the session crosses loopback inside a BEP 3 `piece` message, which
is four bytes of length prefix, one of message id, four of piece index, and
four of offset, so thirteen bytes per 16 KiB block. 0.08%.

Three candidates for where the rest goes, in the order worth checking:

1. **The request pipeline depth.** `librqbit` asks one peer for a bounded
   number of outstanding blocks, and the bridge is one peer. A bound in blocks
   caps throughput at depth times block size over round trip, which matches
   both measurements. This is the first thing to test.
   [T-003](#t-003-the-piece-picker-cannot-be-told-to-prefer-http) already
   doubles the source's in-flight budget for a different reason, so the
   experiment is cheap.
2. Piece verification. 1.49 GiB of SHA-1 is real work and the download stage
   pays it while `fetch` does not.
3. The write to disk, which `fetch` also does not pay.

`bench leech` ([T-090](bench.md)) separates those three, because the recorder
already carries a hashing series and a queue-depth series that nothing
populates yet. That is the next thing to build, and this is the reason.

## The measurement `bench leech` took

`bench leech` is built and the three were separated. **None of the three
candidates above is the answer.** The answer is that one source is one peer,
and one peer is one serial receive path.

`scripts/bench-leech.ps1` takes the same payload from the same loopback server
two ways, five runs per step, and steps the number of bridge connections the
one source is attached over. Run 2026-08-20T04:06:06.879Z, release build,
1 GiB payload, 1024 pieces, medians. Report:
`bench/leech-20260820T040606879Z.json`. The sweep steps the same URL named N
times, which was the only way to get N connections when this ran;
[T-009](#t-009-a-source-cannot-be-attached-over-more-than-one-connection)
re-measured it through `--web-seed-connections` and those are the numbers to
quote.

```
$ pwsh -NoProfile -File scripts/bench-leech.ps1 -PayloadSize 1GiB -Runs 5 -ConnectionSweep "1,2,4,8"
```

| Stage | Median | Slowest | Fastest | Share of fetch | Against 1 bridge |
| --- | --- | --- | --- | --- | --- |
| `bench webseed`, no bridge | 855.90 MiB/s | | | 100.00% | |
| `bench leech`, 1 bridge | 184.40 MiB/s | 169.73 MiB/s | 204.27 MiB/s | 21.55% | 1.00x |
| `bench leech`, 2 bridges | 314.69 MiB/s | 313.53 MiB/s | 340.20 MiB/s | 36.77% | **1.71x** |
| `bench leech`, 4 bridges | 338.40 MiB/s | 313.53 MiB/s | 372.23 MiB/s | 39.54% | **1.84x** |
| `bench leech`, 8 bridges | 292.07 MiB/s | 213.20 MiB/s | 340.09 MiB/s | 34.12% | 1.58x |
| control: 1 bridge, 64 requests in flight | 150.37 MiB/s | 126.33 MiB/s | 169.54 MiB/s | 17.57% | 0.82x |

### It is not the requests in flight

The control is the row that settles it. Every extra bridge is an extra peer
and also an extra set of HTTP requests in flight, so the sweep on its own
cannot say which of the two the gain came from. The control holds the HTTP
concurrency at what the widest step used, 64 requests, and puts all of it on
one bridge. It reaches 0.82x, slightly **worse** than the same bridge at 8.
Four bridges carrying the same 64 requests between them reach 1.84x.

So the gain is the receive paths. `--web-seed-concurrency` does not buy it and
neither would a deeper request window.

### It is not the request window either

The bridge now reports the session's window from the other end. `librqbit`'s
`DEFAULT_PEER_REQUEST_WINDOW` is 128 blocks and the bridge sees exactly that
as its peak per connection, so the window is real and is reached. But the mean
depth is far below it, and what the peak would allow is far above what is
measured: at eight bridges the peak reaches 1024 blocks, which at the measured
21,937 us service time would sustain 729.36 MiB/s, and the run reached
292.07 MiB/s, 40.04% of it. A pipeline that is the limit runs at its ceiling.
This one does not.

### It is not hashing

At one bridge, 1 GiB of piece checks costs 613 ms out of a 5.5 second run,
about 11%. Every piece is read back from disk and hashed, so that figure is
the read and the SHA-1 together. It is real and it is not five sixths of
anything.

### The disk is the second wall, and it is what caps the sweep

The same 1 GiB of writes costs 1,137 ms at one receive path and 14,036 ms
totalled across eight. Per path that is 20% of the run at one bridge and 50%
of the available path time at eight. That is why eight bridges are slower than
four: the paths stop being independent once they contend for the payload file.

Recorded as its own item, [T-017](disk-io.md), with the two candidate causes
and what would separate them.

### What it means

A block arriving from a peer is written, and at a piece boundary the whole
piece is read back and hashed, inline on that connection's own task before the
next block from that peer is processed. So one peer's throughput is bounded by
block size over per-block processing time, whatever the link underneath can
do. The bridge inherits that bound because it presents one source as one peer.

The fix follows from the measurement and is
[T-009](#t-009-a-source-cannot-be-attached-over-more-than-one-connection):
attach one source over several bridge connections. Two is worth 1.71x and four
1.84x on this machine, with no extra HTTP traffic, because the session's picker
divides the pieces between them: the per-source rows in the report add up to
the payload rather than to N copies of it.

Two limits on the number worth stating. It is loopback, so the wire costs
nothing and the receive path is a larger share of the total than it would be
against a real mirror. And the knee moved between four and eight across
repeated sweeps on this machine, so "several is better than one" is solid and
"four exactly" is not.

## The failure cases

Both ran against the loopback file server's new `--stall-after` and `--status`
modes, on the same payload:

| Case | What the server did | Outcome |
| --- | --- | --- |
| stall | sent 64 KiB of a response, then stopped without closing | ended after 24,247 ms, exit 1 |
| 416 | answered every ranged request with `416` | ended after 1,077 ms, exit 1 |

Both end, which is the requirement. The 416 case ends in about a second, which
is right. The stall case takes 24 seconds with `--web-seed-timeout 5s`, which
is the per-request timeout multiplied by the retry and cooldown machinery
rather than a bug, but 24 seconds to notice a dead mirror is longer than it
should be. Recorded as its own item:
[T-007](#t-007-a-stalling-source-takes-24-seconds-to-give-up).

## What was not done

The page cache is not dropped between runs. Windows has no supported way to do
it, and both the ceiling and the candidates read the same file through the same
server, so the cache helps each of them equally. Named in the report's `notes`
rather than left for a reader to work out.

### T-007 A stalling source takes 24 seconds to give up

Source:      the [T-001](#t-001-measure-the-loopback-bridge-against-a-raw-curl-ceiling) failure matrix
Category:    webseed
Priority:    P2
Effort:      S
Status:      open

Problem:     A source that sends part of a response and then stops without
             closing takes 24,247 ms to fail the run, with
             `--web-seed-timeout 5s`. The per-request timeout fires correctly;
             what takes the rest is the retry count multiplied by the cooldown
             before the source is declared unusable.
Relevance:   A mirror behind a hung backend behaves exactly like this, and
             24 seconds of a download stalled on one dead source is 24 seconds
             the other sources were not asked.
Approach:    A source whose requests all time out has not proven itself slow,
             it has proven itself absent, and the two want different handling.
             Count consecutive timeouts separately from other errors and cool
             the source down after the first one rather than after
             `--web-seed-max-errors`. Reproduce with
             `loopback-fileserver --stall-after 65536 --fail-after 2`.
Acceptance:  The stall case in `bench/webseed-<timestamp>.json` ends in under
             three times `--web-seed-timeout`, and this file records the before
             and after.

**Two independent implementations reached the same rule this entry proposes,
and both bound it tighter.** vortex
[PR 143](https://github.com/Nehliin/vortex/pull/143) mirrors libtorrent: drop
a connection with **no activity in either direction for 15 seconds while
requests are in flight**. The in-flight condition is the important half — an
idle connection with nothing outstanding is not stalled, it is idle, and
dropping it is a different decision. `seedchamp/docs/design.md:197` uses a
20 second request stall, **4 seconds in endgame**, and triggers Cancel plus
re-Request rather than killing the source. Two more details there are worth
copying exactly: a partial frame stays in the buffer, and **only ingested
blocks refresh the stall clock**, so a source dribbling bytes that never
complete a block is correctly seen as stalled rather than as slow. That is
precisely the `--stall-after 65536` case this entry reproduces.

vortex [PR 142](https://github.com/Nehliin/vortex/pull/142) is the mistake to
avoid on the way: the in-flight queue was not consulted before snubbing, so
peers were snubbed merely for choking, and fast peers were snubbed after
**explicitly rejecting** every request. A reject is not a timeout, and neither
is a 503 — see [T-005](#t-005-a-source-restricted-mid-run-cannot-be-re-scoped)
on treating 503 as backpressure.

### T-008 A duplicate block request is fetched twice

Source:      the [T-090](bench.md) `bench leech` measurement
Category:    webseed
Priority:    P3
Effort:      S
Status:      open

Problem:     The bridge keeps a set of the blocks the session is waiting on.
             A `request` for a block already in that set inserts nothing new,
             and a second fetch task is spawned for it anyway. The first to
             finish removes the key and sends the block; the second finds the
             key gone and drops what it fetched. So the block was fetched
             twice and served once.
Relevance:   It is small and it is real. On a 3,000 byte torrent the bridge
             answered 3 blocks for 5 requests; on a 2 MiB one it answered 128
             for 128, so it is the tail of a transfer rather than the body of
             it. The window cache absorbs most of the wasted fetch, which is
             why it has not shown up as traffic.
Approach:    Skipping the second spawn is one line, and it is not obviously
             safe: `librqbit` counts its own outstanding requests per block,
             and a peer that answers one `piece` message for two `request`
             messages may leave an entry to time out rather than clear. What
             settles it is reading `remove_inflight_request` in
             `librqbit`'s `torrent_state/live/mod.rs` and then measuring
             `pipeline.requests` against `pipeline.blocks` on a long run with
             and without the guard.
Acceptance:  `summary.pipeline.requests` equals `summary.pipeline.blocks` on a
             `bench leech` run of a torrent with more than a thousand pieces,
             and the run still completes.

**anacrolix solves this from the other end, and the answer is better than
skipping the second spawn.** `torrent/webseed-peer.go:327` `maxChunkDiscard`
and `:344` `readChunks`: the response body **keeps being read after a cancel**,
so already-buffered bytes are used rather than thrown away, and the stream is
cancelled only when no wanted chunk remains inside the discard window. The
related constant is `torrent/webseed/client.go:29`
`MaxDiscardBytes = 48 << 10`: when a server answers `200` to a ranged request,
up to 48 KiB is read and discarded to reach the wanted offset, and beyond that
the request fails as `ErrStatusOkForRangeRequest`.

That reframes this entry. The waste is not "a block was fetched twice", it is
"bytes already in flight were discarded because the request that wanted them
was cancelled". `bit-cli` has a per-source window cache, which is why the cost
has not shown up as traffic, so the two designs are solving the same problem
from opposite sides and the measurement in the Acceptance is what says which
one this tree needs. Read the discard-window rule before writing the one-line
guard: if the window cache already absorbs it, the guard is the whole fix and
the discard window is not needed.

`torrent/webseed/client.go:185` `checkContentLength` is a third small rule from
the same file: compare `Content-Length` **only** when `Content-Encoding` is
`identity` or absent. See [T-004](#t-004-bep-17-style-is-not-auto-detected-only-declared)
on why a transcoding proxy makes any other comparison wrong.

### T-009 A source cannot be attached over more than one connection

Source:      the [T-090](bench.md) `bench leech` measurement
Category:    webseed
Priority:    P1
Effort:      M
Status:      **done**

Problem:     One `--web-seed` is one binding, one bridge, one peer, and one
             serial receive path. That path is what bounds the download, and
             the same source attached twice on the command line goes 1.71x
             faster for it. There is no flag that says so, and repeating a URL
             to get a second connection is a trick rather than an interface.
Relevance:   It is the largest measured win available on the web seed path and
             it needs no fork: 21.55% of the no-bridge fetch rate at one
             connection against 39.54% at four, on the same source over the
             same server in the same session.
Approach:    `--web-seed-connections <N>`, defaulting to 1, expanding one
             binding into N bridges that share the source's scope, its
             concurrency budget, and its window cache. Three things have to be
             right: the per-source accounting stays one row rather than N, the
             concurrency budget is divided rather than multiplied so a mirror
             is not hit N times harder, and the default stays 1 until the
             number is confirmed against a real mirror rather than loopback.
Acceptance:  `bench leech` against the loopback server with
             `--web-seed-connections 4` reaches within 5% of the same run with
             the URL repeated four times, reports one source row, and the
             report records both. Then the same against a real mirror, with
             the number recorded here, because loopback flatters the receive
             path.

## What shipped

`--web-seed-connections <N>`, default 1, on every command that attaches a web
seed. It is also `connections` in a binding table, per source or as a table
default. One binding becomes N bridges, so N peers, and they share one
`Fetcher`: one window cache, one HTTP client, and one concurrency budget
divided between them rather than multiplied by them. The accounting stays one
row per source.

Sharing the fetcher turned out to matter more than the connection count. Two
things follow from it, and the second was not why it was done:

- **The window cache is shared**, so a 4 MiB window is fetched once for the
  whole source rather than once per connection.
- **The concurrency budget is a budget.** Four connections with
  `--web-seed-concurrency 8` make eight requests between them, not thirty-two.

`SourceReport` now carries `connections`, `http_bytes`, and `http_requests`,
and `bench leech`'s per-source rows carry `connections` and `http_bytes`
beside the bytes that reached the session. The two differing is the
amplification, and it was not visible before.

## The loopback number

2026-08-20T04:54:31.219Z, release build, 1 GiB payload, 1024 pieces, five runs
per step, medians. Report: `bench/leech-20260820T045431219Z.json`.

```
$ pwsh -NoProfile -File scripts/bench-leech.ps1 -PayloadSize 1GiB -Runs 5 -ConnectionSweep "1,2,4,8"
```

| Stage | Median | Slowest | Fastest | Share of fetch | Against one |
| --- | --- | --- | --- | --- | --- |
| `bench webseed`, no bridge | 1.04 GiB/s | | | 100.00% | |
| 1 connection | 193.24 MiB/s | 185.24 MiB/s | 194.23 MiB/s | 18.18% | 1.00x |
| 2 connections | 371.01 MiB/s | 337.62 MiB/s | 407.00 MiB/s | 34.90% | **1.92x** |
| 4 connections | 370.34 MiB/s | 338.85 MiB/s | 372.23 MiB/s | 34.84% | 1.92x |
| 8 connections | 408.62 MiB/s | 339.64 MiB/s | 453.50 MiB/s | 38.44% | 2.11x |
| control: 1 connection, 64 requests in flight | 157.01 MiB/s | 150.70 MiB/s | 162.49 MiB/s | 14.77% | 0.81x |
| the URL named 8 times, 8 sources | 271.62 MiB/s | 202.28 MiB/s | 314.30 MiB/s | 25.55% | 1.41x |

**Two connections is worth 1.92x and the curve is flat after that.** Eight
reads higher at the median but its slowest run is no faster than two, and the
spread is a third of the rate. Two is where the gain is; four is free; eight
is not worth the peer slots.

The control row is the one that names the cause. Eight times the requests in
flight on a single connection reaches 0.81x, slightly **worse** than the same
connection at eight. It is the receive paths.

## What sharing the fetcher is worth

| Form | Rate | Pulled off the mirror for a 1 GiB payload |
| --- | --- | --- |
| `--web-seed-connections 8` | 408.62 MiB/s | 1.00 GiB, **1.004x** |
| the URL named 8 times | 271.62 MiB/s | 3.98 GiB, **3.984x** |

Same eight peers either way. Eight separate sources have eight window caches,
so each one fetches the same 4 MiB window itself and the mirror serves the
payload nearly four times over.

The acceptance above asked for parity with the repeated form. It is 1.50x the
rate on a quarter of the mirror's bandwidth, so the criterion is met in the
direction that matters, and the reason it was written expecting parity is that
the amplification was not measurable until `http_bytes` existed.

Worth recording for what it says about the earlier runs: every
"attach the same URL N times" experiment in this file's history was pulling N
times the payload. Those numbers stand as measurements of that configuration
and none of them is what `--web-seed-connections` does.

### T-002 Measure Candidate A-prime, the in-process virtual peer

Source:      PROMPT.md section 2.5
Category:    webseed
Priority:    P1
Effort:      L
Status:      **done**

Problem:     The same "a web seed is a peer" abstraction is implementable
             without a socket: the worker talks to the torrent manager over
             channels. If that shape is reachable through `librqbit`'s public
             API, Candidate A's loopback hop and second copy are removable with
             no fork.
Relevance:   It is the cheapest large win available, if it is available.
Approach:    Establish first whether `librqbit` 9.0.0 can accept a peer that is
             not a socket. `Session::add_torrent` takes `initial_peers` as
             `SocketAddr`, and `TorrentStateLive` owns its peer connections, so
             the answer is probably no without a fork. Write down the answer
             either way with the API surface that decides it. If it needs a
             fork, it collapses into Candidate B and this item closes with that
             finding.
Acceptance:  This file states, with the `librqbit` types named, whether an
             in-process virtual peer is reachable through the public API. If it
             is, T-001's benchmark runs against it too.

## The answer

**No, not through `librqbit` 9.0.0's public API. But the machinery underneath
already takes an arbitrary byte stream rather than a socket, so what stands in
the way is four `pub(crate)` markers rather than a design that assumes TCP.**

Read it for yourself:

```
$ cargo tree -p librqbit --depth 0
$ grep -rn "pub(crate) fn add_incoming_peer" -A 4 ~/.cargo/registry/src/*/librqbit-9.0.0/src/torrent_state/live/mod.rs
```

The five places that decide it, all in `librqbit-9.0.0/src`:

| Where | What it takes | Visibility |
| --- | --- | --- |
| `session.rs:281`, `AddTorrentOptions::initial_peers` | `Option<Vec<SocketAddr>>` | public |
| `listen.rs:52`, `ListenerOptions` | `listen_addr: SocketAddr` | public, but the sockets it produces land in `ListenResult` (`listen.rs:15`), which is not |
| `stream_connect.rs:129`, `StreamConnector` | `connect(&self, addr: SocketAddr)` | `pub(crate)` |
| `torrent_state/live/mod.rs:722`, `add_peer_if_not_seen` | `SocketAddr` | `pub(crate)` |
| `torrent_state/live/mod.rs:362`, `add_incoming_peer` | `CheckedIncomingConnection` | `pub(crate)` |

Every public route in, an address. So a caller outside the crate has no way to
say "here is a peer, talk to it over this".

The last row is the interesting one. `CheckedIncomingConnection`
(`session.rs:533`) is not a socket:

```rust
pub(crate) struct CheckedIncomingConnection {
    pub kind: ConnectionKind,
    pub addr: SocketAddr,
    pub reader: BoxAsyncReadVectored,
    pub writer: BoxAsyncWrite,
    pub read_buf: ReadBuf,
    pub handshake: Handshake,
}
```

`BoxAsyncReadVectored` and `BoxAsyncWrite` (`type_aliases.rs:19` and `:20`) are
`Box<dyn AsyncReadVectored + Unpin + Send>` and `Box<dyn AsyncWrite + Unpin +
Send>`. A `tokio::io::duplex` pair satisfies both. The session would accept an
in-process peer today if it could be handed one, and `AddIncomingPeerResult`
(`live/mod.rs:175`) is already public, as is `TorrentStateLive` through
`ManagedTorrent::live()`.

What is not public is the way in: `add_incoming_peer` itself, the struct it
takes, the two box aliases (`mod type_aliases` is private), and `ReadBuf` (`mod
read_buf` is private, `read_buf.rs:17`). Four items and two module markers.

So Candidate A-prime needs a fork, and it collapses into Candidate B exactly as
this entry allowed for. It is a small fork, which is worth recording: it does
not need a behaviour change upstream, only visibility.

## What it is worth now

Less than when this was written, and [T-090](bench.md) is why. The loopback hop
and the second copy are not where the throughput goes. `bench leech` measured
the per-peer serial receive path as the bound, and an in-process virtual peer
is still one peer with one of those. The framing the socket costs was measured
by [T-001](#t-001-measure-the-loopback-bridge-against-a-raw-curl-ceiling) at
0.08% of payload bytes.

So this is no longer the cheapest large win. It is a fork for a small gain.
[T-009](#t-009-a-source-cannot-be-attached-over-more-than-one-connection) is
the large one and it needs no fork at all.

The finding is still load bearing for two other entries. Candidate B and
Candidate C both start from "what would a fork cost", and the answer is now
concrete rather than a guess: four visibility changes, no redesign. If the four
ever become public upstream, this reopens as an ordinary piece of work rather
than as a fork.

### T-003 The piece picker cannot be told to prefer HTTP

Source:      `--prefer-web-seed`, PROMPT.md A3.4
Category:    webseed
Priority:    P1
Effort:      M
Status:      **done**

Problem:     `--prefer-web-seed` is documented as "bias the picker toward HTTP
             when both a peer and a source have a piece". `librqbit`'s piece
             picker is not reachable from outside the crate, so `bit-cli`
             cannot express that preference directly.
Relevance:   On a hybrid run the flag is what decides whether a fast mirror or
             a slow peer serves a piece. Today it changes the odds rather than
             the decision.
Approach:    What ships now: the flag doubles each source's in-flight request
             budget (bounded at 32), so an HTTP source answers a block sooner
             and `librqbit` takes the first answer. What it does not do: reach
             the picker. Closing the gap needs either a `librqbit` API for peer
             preference or Candidate C, a native `Source` trait with its own
             picker integration.
Acceptance:  A hybrid run with one fast local mirror and one slow peer, run
             twice, shows a measurable shift in the peer/web-seed byte split
             with the flag on. Both splits are recorded here with the commands.

## What the old implementation was worth

Nothing. [T-090](bench.md) measured it: one connection at 64 requests in flight
reaches 0.81x the same connection at 8. Doubling a source's in-flight budget
does not make it answer sooner, because the budget is not what bounds it. So
the flag changed a number that did not move the outcome, which is worse than a
flag that does nothing and says so.

## What ships now

The flag doubles each source's **connections** rather than its request budget,
bounded to at most eight. With the default of one connection that is two, which
is where [T-009](#t-009-a-source-cannot-be-attached-over-more-than-one-connection)
measured the knee. That is a lever with a number behind it: connections are
worth 1.92x on the same measurement where requests are worth 0.81x.

It is still not the picker. `bit-cli` cannot say "take this piece from HTTP";
what it can do is make HTTP the side that answers first, and the session takes
the first answer.

## The number

`scripts/check-prefer.ps1` builds a hybrid swarm entirely on loopback: the file
server as an HTTP source, a second `bit-cli` seeding the same payload as a
peer, and a leecher given both. Neither side is rate limited, which is the
point: a cap on either side decides the split by itself, and a flag measured
against capped sources measures the caps.

The peer announces nothing and the leecher is given its address with `--peer`,
with `--no-tracker --no-dht --no-lsd` on both, so the swarm is exactly two
members.

2026-08-20T05:23:37.591Z, release build, 1 GiB payload, five pairs. Each pair
is one run without the flag and one with it, back to back against the same two
sources. Report: `bench/prefer-20260820T052337591Z.json`.

```
$ pwsh -NoProfile -File scripts/check-prefer.ps1 -PayloadSize 1GiB -Runs 5
```

| Pair | HTTP share without | HTTP share with | Shift |
| --- | --- | --- | --- |
| 1 | 50.00% | 66.22% | +16.22 |
| 2 | 46.00% | 62.89% | +16.89 |
| 3 | 46.19% | 61.43% | +15.23 |
| 4 | 45.80% | 62.40% | +16.60 |
| 5 | 45.61% | 60.06% | +14.45 |
| mean | **46.72%** | **62.60%** | **+15.88** |

Every pair shifted toward HTTP and none shifted back. Uncapped, the two sides
split the payload nearly evenly without the flag, which is what says neither
was throttled into the answer.

## What is still not closed

The picker. This moves the odds, not the decision, and a piece a peer happens
to answer first still comes from the peer. Reaching the decision needs a
`librqbit` API for peer preference or Candidate C, a native `Source` trait with
its own picker integration, and [T-002](#t-002-measure-candidate-a-prime-the-in-process-virtual-peer)
prices what that fork would cost. The flag's help says what it does rather than
what it would ideally do.

One defect turned up while building the measurement and is fixed with it:
`--web-seed-speed-limit` parsed, validated, reached the source spec, and was
never applied. See [T-035](performance.md).

---

## Wire protocol

### T-004 BEP 17 style is not auto-detected, only declared

Source:      PROMPT.md A3.4, `--web-seed-style auto`
Category:    webseed
Priority:    P2
Effort:      S
Status:      open

Problem:     `--web-seed-style auto` resolves to BEP 19 for every
             command-line source. Sources from the torrent's `httpseeds` key
             are marked BEP 17 at collection time, which covers the case the
             metainfo declares, but a caller pointing `--web-seed` at a Hoffman
             seed has to say `--web-seed-style hoffman` by hand.
Relevance:   Getting the style wrong produces a 404 or a wrong-length body from
             a healthy server, which reads as a broken mirror.
Approach:    `bit-cli webseed test` already learns enough to decide: a Hoffman
             seed answers a `?info_hash=&piece=0&ranges=0-0` probe and a
             GetRight seed does not. Wire that probe into `auto` so the style
             is decided once per source before the first real request, and
             report which style was chosen in `webseed list --json`.
Acceptance:  `bit-cli webseed test <TORRENT> --web-seed <HOFFMAN URL>` reports
             `"style": "hoffman"` without the flag, and a download from that
             source completes.

**The corpus holds the only implementation of both styles, and it makes half of
this entry unnecessary.** `gosh-dl` parses the two metainfo keys into two
separate fields: `gosh-dl/src/torrent/metainfo.rs:125` reads `url-list` into
`:36` `url_list` and `:128` reads `httpseeds` into `:38` `httpseeds`, both
through one shared parser at `:391` `parse_url_list` that accepts a bencoded
string or a list and filters to `http://` and `https://`. Its
`gosh-dl/src/torrent/webseed.rs:24` has the type,
`WebSeedType { GetRight, Hoffman }`, and `:587` `build_piece_url` has both URL
forms: GetRight is the URL itself for a single-file torrent and a per-file URL
otherwise, and **Hoffman is `{url}?info_hash={urlencoded}&piece={index}`**.
`:618` `build_file_url` trims a trailing `/`, percent-encodes each path
component and joins with `/`.

**Then it throws the distinction away, and that is the part to not copy.** At
`gosh-dl/src/torrent/webseed.rs:303` the manager builds every source with
`let seed_type = WebSeedType::GetRight;` under a comment guessing that Hoffman
seeds "typically end with specific paths", and `:479` `all_webseeds()` merges
`url_list` and `httpseeds` into one list. The style was parsed correctly and
discarded before use.

**Which hands this entry a cheaper answer than the probe.** BEP 17 and BEP 19
are distinguished by *which metainfo key the URL came from* — that is what the
two BEPs specify, and it needs no network round trip. `bit-cli` already marks
sources from `httpseeds` as BEP 17 at collection time, so the metainfo half is
done. What is left is genuinely only the command-line case, where there is no
key to read and the probe is the only signal. So this entry shrinks to: keep
the metainfo keying, add the probe for `--web-seed` sources alone, and keep
`--web-seed-style` as the override for both.

The probe also needs `Accept-Encoding: identity`, which gosh-dl sets on every
web seed request for a reason worth carrying: a transcoding proxy that
re-encodes the body silently changes the byte range that comes back, so a
correct request returns wrong bytes and the piece fails its hash against a
healthy mirror.

Related and separate: `bit-cli` reads `url-list` as a string or a list and
reads `httpseeds` as a list only, which is [T-171](metainfo.md).

### T-005 A source restricted mid-run cannot be re-scoped

Source:      design gap; corroborated by `reference/RESEARCH.md` section D, 2026-08-21
Category:    webseed
Priority:    P2
Effort:      M
Status:      open

Problem:     Scopes are resolved once, before the first request. A mirror that
             turns out to hold less than it claimed is dropped whole rather
             than narrowed to what it can serve.
Relevance:   A CDN that 404s on one file of a twelve-file torrent currently
             costs the whole source.
Approach:    On a permanent per-file failure, subtract that file's byte range
             from the binding's scope, recompute coverage, and re-announce the
             narrowed bitfield. The bridge cannot retract bits it has already
             announced, so this means dropping the connection and reconnecting
             with the smaller bitfield.
Acceptance:  A torrent with two files, a mirror that serves one and 404s the
             other, and a peer for the rest: the run completes, and
             `--json` reports the source's scope narrowed to the file it does
             hold.

**Raised from P3 to P2 on 2026-08-21, and the argument is that this is a
correctness bug in the one feature `bit-cli` exists for rather than a
refinement.**

`bit-cli`'s whole scope model says a mirror holding part of a payload is a
first-class case and not an error. `README.md` says so, and it is the
difference between this tool and every other one. The retirement rule
contradicts it: a permanent status on **one file** — 401, 403, 404, 410 or
416 — retires the **whole source**, including the files it was serving
correctly a moment earlier. So `bit-cli` supports partial mirrors right up to
the moment a mirror turns out to be partial in a way the scope did not
predict, which is the case the model is for.

`torrent/webseed-peer.go:57` `webseedFileUnavailable` is the rule this should
be: on 403, 404, 410 or 451, remove **only that file's pieces** from the web
seed's bitmap, via `:71` `removeFilePieces`. The source keeps serving
everything else. That is strictly better for a partial mirror and it is not
more code, it is different code in the same place.

Two more things from the same file are worth taking while this is open.
`torrent/webseed-peer.go:46` uses `convict(err, time.Minute)` — a source is
**suspended for a term** rather than killed, which is what
[T-137](multi-source.md) already built here as `--web-seed-cooldown`, so the
two designs agree and this entry only has to extend it from whole-source to
per-file. And `torrent/webseed/client.go:270` treats **503 as backpressure
rather than death**, which is a status `bit-cli` should never let a user
configure as fatal by accident.

Note the wire-level obstacle in this entry's Approach — that the bridge cannot
retract bits it has already announced, so re-scoping means a reconnect — has an
answer in the corpus that removes it entirely. BEP 54 `lt_donthave` is one
extended message carrying a piece index that clears exactly one bit in the
peer's bitfield. That is [T-167](bep-coverage.md), it is about twenty lines of
protocol, and it turns this entry's reconnect into a message. Do that first and
this gets smaller.

---

## Correctness axes, from the section 2.2 checklist

These are pass or fail, not measured. Each has a covering test today; the entry
records what still needs a real mirror rather than a stub.

### T-006 Prove the failure matrix against a real mirror

Source:      PROMPT.md section 3, matrix items 5 to 7
Category:    webseed
Priority:    P1
Effort:      M
Status:      **done**

Problem:     404, 403, 416, 5xx, a redirect chain, and a server that ignores
             `Range` are all covered by the stub server in
             `crates/bit-cli-core/tests/webseed_e2e.rs`. None has been seen
             from a real mirror.
Relevance:   A stub answers exactly what it was written to answer. Real CDNs
             return 403 with an HTML body, redirect to a login page, and send
             `Accept-Ranges: none` while still honouring `Range`.
Approach:    Point `bit-cli webseed test` at the Alpine and Arch mirrors and at
             a deliberately wrong path on each. Record what came back.
Acceptance:  This file records, per mirror, the status and the classification
             `bit-cli` gave it, and every classification is the right one.

The Arch Linux ISO torrent carries 468 web seeds in its `url-list`, which is a
better failure matrix than anything that could be written by hand. Every one
was probed, 2026-08-20T00:07Z:

```
$ bit-cli webseed test archlinux-2026.08.01-x86_64.iso.torrent \
    --concurrency 32 --timeout 120s --json
```

42 seconds, 468 sources, 391 usable and 77 not:

| Result | Count | What it was | `bit-cli` said |
| --- | --- | --- | --- |
| `206 Partial Content` | 391 | a working mirror | usable, `range_support: yes` |
| `200 OK` to a ranged GET | 13 | the server ignored `Range` | unusable, `range_support: no` |
| no response at all | 49 | connect or TLS failure | unusable, the transport error verbatim |
| `404` | 7 | the path is not on that mirror | unusable, `HTTP 404 Not Found` |
| `522` | 3 | a Cloudflare origin timeout | unusable, `HTTP 522` |
| `403` | 2 | the mirror refuses that path | unusable, `HTTP 403 Forbidden` |
| `502`, `503` | 2 | the mirror is down | unusable, the status |
| `418` | 1 | a mirror that answers with a teapot | unusable, `HTTP 418` |

Every classification is right. Three details worth keeping:

- The thirteen servers that ignore `Range` are mostly Cloudflare and nginx,
  and they answer `200` with the whole 1.49 GiB entity. Detecting that from
  the status rather than from `Accept-Ranges` is what makes it reliable: an
  `Accept-Ranges: bytes` header from a server that then ignores the range
  would have passed a header check.
- Five mirrors redirect, and each chain is reported hop by hop with the status
  and the resolved URL. `mirrors.kernel.org` answers `301` to
  `mirrors.edge.kernel.org`; two more answer `302`.
- Thirteen sources report `length_matches: false`. All thirteen are error
  responses whose `Content-Length` is the size of the error page, so the
  mismatch is real and the source was already unusable on its status. No
  false positive.

Two defects were found and fixed while running this, both of which meant the
command had never worked against a real HTTPS mirror:

- `rustls` 0.23 refuses to choose a cryptography provider on its own and
  panics when a `ClientConfig` is built without one. `reqwest` installs one for
  its own connections, but the TLS probe opens its own connection through
  `tokio-rustls` and got nothing, so every HTTPS source panicked with exit 101
  instead of reporting a cipher suite. Nothing caught it because every test
  until then used loopback HTTP.
- The TLS probe had no deadline of any kind. A mirror that accepted the
  connection and then said nothing held the command open indefinitely.

Both now have regression tests in `webseed::probe::tests` that need no network.
The probe also runs sources in parallel now: at one source at a time, 468
mirrors would have taken between ten and forty minutes.

---

### T-141 --web-seed-connect-timeout does not bound a connect that never answers

Source:      found building [T-117](cli-surface.md)'s `source_failed` fixture
Category:    webseed
Priority:    P1
Effort:      M
Status:      **done**

Problem:     `--web-seed-connect-timeout` parses, reaches
             `Limits::connect_timeout_ms`, and is handed to
             `reqwest::ClientBuilder::connect_timeout` in both
             `webseed::fetch::Fetcher::new` and `webseed::probe`. It changes
             nothing a caller can measure. Against an address that drops the
             SYN rather than refusing it, the attempt ends on
             `--web-seed-timeout` instead, whatever the connect timeout says.
Relevance:   Rule 0.10: a flag that does not move a number does not ship. It
             also decides how long an unattended run waits on a mirror behind
             a firewall that blackholes traffic, which is the common failure
             on a public mirror list. Today the answer is the request timeout,
             30 seconds by default, per attempt.
Approach:    Three things to establish, in order. First whether
             `reqwest` 0.12.28's `connect_timeout` fires at all on this
             platform, with a repro of ten lines against a blackholed address
             and no `bit-cli` in the way. Second, if it does not, whether the
             fix is ours: wrapping the request future in
             `tokio::time::timeout(connect_timeout)` bounds the whole request
             rather than the connect, which is not the same promise and would
             be wrong to ship under this name. Third, if neither, the flag
             either becomes a documented alias for the shorter of the two
             timeouts or it goes, per rule 0.10.
Acceptance:  A run against a blackholed address with
             `--web-seed-connect-timeout 2s --web-seed-timeout 45s` ends in
             about 2 seconds rather than 45, and a test proves it without
             needing a firewall.

**The measurement, 2026-08-20T16:07Z, release build.** `127.0.0.1:9` is
blackholed on this machine: Windows drops the SYN rather than refusing it, and
`curl -m 6` against it times out with no response rather than failing fast.

```
$ bit-cli --json webseed test album.torrent --no-torrent-web-seed \
    --web-seed http://127.0.0.1:9/ [--web-seed-connect-timeout ...] [--web-seed-timeout ...]
```

| connect timeout | request timeout | wall time |
| --- | --- | --- |
| default 10s | default 30s | 30.138s |
| **2s** | default 30s | 30.108s |
| **2s** | **45s** | **45.110s** |

The third row is what makes it decisive. Halving the connect timeout does not
move the number and raising the request timeout moves it exactly, so the
request timeout is the only bound in play.

It has a second effect worth naming, because it is what led here. A source at
a blackholed address makes no request until that timeout expires, so it records
no error, spends none of its `--web-seed-max-errors` budget, and is not
retired. A `--web-seed-only` run against one such mirror reports the source as
`active` with `http_requests: 0` and sits there. Measured with
`--web-seed-only --web-seed-max-errors 1 --web-seed-retries 0` and nothing
else to fetch from, the whole run takes **30.364s** against a blackholed
address and **1.109s** against a live server answering 404. That is the same
defect from the other end, and it is why the generator uses the 404.

---

## The correction: the flag was never broken, and the address was not a blackhole

**Everything above is measured correctly and concludes the wrong thing.**
`127.0.0.1:9` is not blackholed on this machine. Port 9 is `discard`, and
Windows ships it in the optional **Simple TCP/IP Services** feature, which is
installed here:

```
$ Get-NetTCPConnection -LocalPort 9 -State Listen
LocalAddress  LocalPort  OwningProcess
0.0.0.0       9          5736          # TCPSVCS
```

It **accepts** the connection and never sends a byte. Watching the socket while
a request is in flight is what shows it, and it is not subtle:

```
0.0s  n=1  49946:Established
5.4s  n=1  49946:Established
```

`Established`, not `SYN_SENT`. So the connect succeeded in microseconds, the
run was correctly bounded by the request timeout, and
`--web-seed-connect-timeout` was doing exactly what its name says by not
firing. The entry accused the flag of bounding nothing on the strength of a
case where there was nothing for it to bound.

**Against an address that really does drop the SYN, the flag is the only bound
in play.** RFC 5737 reserves `192.0.2.0/24` as TEST-NET-1 and no network routes
it. First with no `bit-cli` in the way, which is what the approach above asked
for as step one:

| connect timeout | request timeout | wall time | what `reqwest` said |
| --- | --- | --- | --- |
| 2s | 30s | **2.013s** | `is_connect=true is_timeout=true`, deadline has elapsed |
| 6s | 30s | **6.011s** | same |
| none | 30s | 21.038s | os error 10060, Windows' own TCP connect timeout |

Then through `bit-cli webseed test`, sweeping both flags against each other:

| connect timeout | request timeout | wall time |
| --- | --- | --- |
| 2s | 30s | **2.063s** |
| 2s | 45s | **2.074s** |
| 5s | 30s | **5.061s** |
| 5s | 45s | **5.056s** |
| 10s | 30s | **10.050s** |
| 10s | 45s | **10.056s** |
| default, 10s | 45s | 10.053s |

The request timeout moves the number by 11 ms across a 15 second change in it.
The connect timeout moves it by exactly itself. That is the acceptance's "about
2 seconds rather than 45", met.

**The second effect does not reproduce either.** A `--web-seed-only` download
against the blackholed address, with `--web-seed-connect-timeout 2s
--web-seed-timeout 30s --web-seed-max-errors 1 --web-seed-retries 0`, ends in
**3.07s** with the source retired:

```json
{"state": "failed", "http_requests": 1, "cooldowns": 1, "retries": 0,
 "error": "http://192.0.2.1/...: connect timed out, raise --web-seed-connect-timeout"}
```

`http_requests` is 1 and the state is `failed`, where the entry predicted 0 and
`active`. The same run against the discard service takes 30.4s and is bounded
by the request timeout, which is right: that connect succeeded.

**One real defect came out of it, and it is the reporting.** A connect timeout
sets both `is_connect()` and `is_timeout()` on a `reqwest::Error`, and
`classify_transport` asked about the timeout first, so every connect timeout
was reported as `timed out` and the connect branch below it was unreachable. A
reader raising `--web-seed-timeout` in response would have changed nothing. The
messages now name which of the two expired, and both paths give the same one:
`webseed test` used to print `reqwest`'s own `error sending request for url
(...)` because it formatted the error itself rather than classifying it, so the
same failure read differently depending on which command found it. Both go
through `fetch::transport_reason` now.

**Acceptance.** `scripts/check-connect-timeout.ps1` drives both directions,
because a flag that bounds everything is as wrong as one that bounds nothing:

- On a blackholed address the connect timeout must be the bound, and the
  request timeout must not move the wall clock.
- On a listener that accepts and never answers, the reverse. That listener is
  served by the script rather than borrowed from the machine, so the check does
  not depend on Simple TCP/IP Services being installed.
- Each run's message must name the timeout that expired.

The blackhole cannot be served, because it is the absence of an answer. So the
script proves the address is one before it measures anything: a raw connect
must still be pending after `-ProbeSeconds`, and it exits 2 rather than passing
if the network answers.

```
$ pwsh -NoProfile -File scripts/check-connect-timeout.ps1
2026-08-21T02:13:00.994Z 192.0.2.1:80 is still pending after 2s
  blackhole connect= 2s request=30s     2053 ms  connect timed out, raise --web-seed-connect-timeout
  blackhole connect= 2s request=45s     2071 ms  connect timed out, raise --web-seed-connect-timeout
  blackhole connect= 5s request=30s     5056 ms  connect timed out, raise --web-seed-connect-timeout
  blackhole connect= 5s request=45s     5057 ms  connect timed out, raise --web-seed-connect-timeout
  discard   connect= 2s request=30s    30055 ms  timed out waiting for the response, raise --web-seed-timeout
  discard   connect= 2s request=45s    45057 ms  timed out waiting for the response, raise --web-seed-timeout
  discard   connect= 5s request=30s    30041 ms  timed out waiting for the response, raise --web-seed-timeout
  discard   connect= 5s request=45s    45075 ms  timed out waiting for the response, raise --web-seed-timeout
verdict: pass
```

`webseed::fetch::tests::a_transport_failure_names_the_timeout_that_expired`
covers the classification with no network and no firewall: a listener that
accepts and never answers is a request timeout, and a port nothing listens on
is a refused connect. The third shape, a connect that never completes, is what
the script above is for.

**What this cost, and what to take from it.** A P1 stood for a session against
a flag that worked, and the fixture was what was wrong. `curl -m 6` against the
same address also "times out with no response", which is what made the original
reading look confirmed: a discard service and a blackhole are indistinguishable
from the client's wall clock and distinguishable in one line of
`Get-NetTCPConnection`. When a measurement says a flag does nothing, check that
the condition the flag names is the condition being produced.


### T-162 Two bench webseed tests assumed a loaded runner cannot also fail

Source:      CI run 32460302583, `Test (macos-latest)`, 2026-08-21
Category:    bench
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `bench_webseed_names_a_server_that_ignores_range` asserted that
             `errors.by_class["range_ignored"]` equals `errors.total`, and
             `bench_webseed_counts_a_404_by_class_and_by_status` asserted the
             same for `not_found`. Both say "this is the only way a request in
             this run can fail", which is a claim about the machine and not
             about the code.
Relevance:   It turned `Test (macos-latest)` red on a **documentation-only
             commit**, which is the second time in one session that a green
             matrix hid a test making an assumption about timing rather than
             about behaviour. [T-160](cli-surface.md) is the first.
Approach:    Assert what the code is responsible for. The class is present and
             counted, every response that arrived is the one the class names,
             and every error carries a class. Not that no other class can
             exist.
Acceptance:  Both tests pass on all three platforms, and an error with no class
             still fails them.

The numbers, from `crates/bit-cli-core/tests/webseed_e2e.rs:1198`:

```
assertion `left == right` failed
  left: Some(1828)
 right: Some(7557)
```

7,557 requests failed in a 500 ms burst at concurrency 4 and 1,828 of them were
classified `range_ignored`. The rest never reached the range check: under that
rate a loopback server's accept backlog fills and connections are refused or
reset, which is a transport failure and correctly a different class. The run
was still right about everything it exists to prove: zero usable bytes,
`range_support: No`, and a note naming the server.

What each test asserts now:

- `summary.bytes.0 == 0`, unchanged, and the point of the test.
- The named class is present and above zero.
- `by_status["200"]` equals `by_class["range_ignored"]`, and `by_status["404"]`
  equals `by_class["not_found"]`. That ties the class to the status without
  claiming the status is the only outcome.
- `by_class.values().sum() == errors.total`, which is **stronger** than what
  was there: an error that reaches the total and no class is now a failure,
  and it was not before.

The last one is the point. Replacing a brittle assertion with a weaker one
would have traded a red job for a blind spot. This one is narrower about the
machine and wider about the code.

---

### T-179 A bad piece cannot be attributed to the source that filled it

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    webseed
Priority:    P2
Effort:      M
Status:      open

Problem:     When a piece fails its hash, `bit-cli` knows the piece failed and
             does not know **who supplied the wrong bytes**. A piece is filled
             from blocks that may have come from several HTTP sources, several
             peers, or a mix, and nothing records which block came from where
             once the block has been written.
Relevance:   This is the gap under two other entries and it is the reason both
             of them have to guess.

             [T-164](peers.md) wants to block a peer that sends garbage, and
             [T-005](#t-005-a-source-restricted-mid-run-cannot-be-re-scoped)
             wants to narrow a source that serves the wrong thing. Both need to
             name a culprit. Without attribution the choices are to punish
             everyone who contributed to the piece, which retires healthy
             mirrors, or to punish nobody, which is what happens today.

             It matters more here than in a normal client, and the reason is
             `bit-cli`'s own design. A conventional client fills a piece from
             one peer most of the time, so "the piece failed" and "that peer is
             bad" are nearly the same statement. `bit-cli` exists to point
             **several sources at one payload**, so its normal case is the
             ambiguous one. `--web-seed-verify piece` already hash-checks at
             the source, which covers a source serving a whole wrong piece; it
             does not cover a piece assembled from two sources where one of
             them is wrong.
Approach:    `torrent/smartban/smartban.go` is 83 lines and does exactly this,
             with `torrent/smartban.go` as the integration. Every block is
             recorded with the peer that supplied it. When a piece fails its
             hash, `CheckBlock` re-hashes each block against the verified data
             once it arrives and returns **exactly the peers whose blocks
             disagree**. That converts "a source is bad" from a guess into a
             fact, and it does it without a second fetch, because the correct
             data arrives anyway on the retry.

             The cost is the block-to-source map, which is bounded by the
             pieces in flight rather than by the torrent size, so it belongs
             with the accounting [T-041](memory.md) is already measuring rather
             than as a new allocation to justify. `--web-seed-verify piece`
             gives a head start: the per-source hash check already exists and
             the map is what generalises it to the mixed case.

             gosh-dl [PR 7](https://github.com/goshitsarch-eng/gosh-dl/pull/7)
             is the accounting hazard to avoid while touching this code. In
             endgame the same piece is requested from several sources, and two
             concurrent completions both incremented the verified-byte counter,
             so progress exceeded 100 per cent — reported by a user as
             331.5 MB of 263.6 MB, or 125.8 per cent, in
             [Issue 6](https://github.com/goshitsarch-eng/gosh-dl/issues/6).
             The fix is to make the increment conditional on winning the
             `pending.remove()` race. `bit-cli` closed
             [T-139](multi-source.md) over the same class of error, a resumed
             download charging its existing bytes to the swarm, so the rule is
             already established here: **report verified bytes, count network
             bytes separately.**
Acceptance:  A run with two sources for one torrent, one of which serves wrong
             bytes for part of a piece, names that source and only that source
             in `--json`, retires it, and completes from the other. The healthy
             source is asserted to still be active at the end, because
             retiring both is the failure this entry exists to prevent.
