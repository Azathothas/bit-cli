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
`bench/leech-20260820T040606879Z.json`.

```
$ pwsh -NoProfile -File scripts/bench-leech.ps1 -PayloadSize 1GiB -Runs 5 -BridgeSweep "1,2,4,8"
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

### T-009 A source cannot be attached over more than one connection

Source:      the [T-090](bench.md) `bench leech` measurement
Category:    webseed
Priority:    P1
Effort:      M
Status:      open

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

### T-002 Measure Candidate A-prime, the in-process virtual peer

Source:      PROMPT.md section 2.5, `superseedr/src/networking/web_seed_worker.rs`
Category:    webseed
Priority:    P1
Effort:      L
Status:      open

Problem:     `superseedr` implements the same "a web seed is a peer"
             abstraction without a socket: the worker talks to the torrent
             manager over channels. If that shape is reachable through
             `librqbit`'s public API, Candidate A's loopback hop and second
             copy are removable with no fork.
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

What [T-090](bench.md) measured changes what this entry is worth. The loopback
hop and the second copy are not where the throughput goes: the per-peer serial
receive path is, and an in-process virtual peer would still be one peer with
one of those. So removing the socket buys the framing and a copy, which
`bench leech` puts at a small share, and not the five sixths this entry was
written to chase. It is still worth answering, because the answer also decides
what Candidate B and Candidate C would cost, but it is no longer the cheapest
large win. [T-009](#t-009-a-source-cannot-be-attached-over-more-than-one-connection)
is, and it needs no upstream change at all.

### T-003 The piece picker cannot be told to prefer HTTP

Source:      `--prefer-web-seed`, PROMPT.md A3.4
Category:    webseed
Priority:    P1
Effort:      M
Status:      open

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

[T-090](bench.md) measured what the current implementation is worth and the
answer is nothing: one bridge at 64 requests in flight reaches 0.82x the same
bridge at 8. Doubling a source's in-flight budget does not make it answer
sooner, because the budget is not what bounds it. So the flag as it ships
changes a number that does not move the outcome, which is worse than a flag
that does nothing and says so.

What would give `--prefer-web-seed` a real effect without reaching the picker
is [T-009](#t-009-a-source-cannot-be-attached-over-more-than-one-connection):
give the preferred source more receive paths than the swarm gets. That is a
measured lever rather than a hoped-for one, and it is the shape this flag
should take when T-009 lands.

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

### T-005 A source restricted mid-run cannot be re-scoped

Source:      design gap
Category:    webseed
Priority:    P3
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
