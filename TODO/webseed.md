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
15.13% of bit-cli's own HTTP path with the bridge removed.

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

## What the number does and does not say

Loopback has no network cost, so this is the worst case for `bit-cli` and the
best case for `curl`: every millisecond the bridge spends is a millisecond the
wire is not spending. On a real link the wire is the bottleneck long before
either of them is. 164.00 MiB/s is 1.38 Gbit/s, which saturates this machine's
fastest interface (1.00 Gbit/s) with 38% to spare. It would not saturate
10 Gbit/s.

So the bridge is not a problem for a gigabit netdisk and is a problem for a
ten gigabit one. That is the decision this number was taken to inform.

The 1.000793 figure is the framing, and it is small: every block the bridge
hands the session crosses loopback inside a BEP 3 `piece` message, which is
four bytes of length prefix, one of message id, four of piece index, and four
of offset, so thirteen bytes per 16 KiB block. 0.08% overhead. The cost is not
the framing, it is the copy and the round trip: the same bytes are read from a
socket, written to a second socket, read back, and only then verified.

Where the remaining 85% goes is not decided by this measurement. Three
candidates, in the order worth checking:

1. `librqbit`'s per-peer request pipeline depth. One peer is asked for a
   bounded number of outstanding blocks, and the bridge is one peer.
   [T-003](#t-003-the-piece-picker-cannot-be-told-to-prefer-http) already
   doubles the source's in-flight budget for a different reason and could be
   measured here.
2. Piece verification. 256 MiB of SHA-1 is real work and the download stage
   pays it while `fetch` does not. `bench leech` will separate it, because the
   recorder already has a hashing series that nothing populates yet.
3. The write to disk, which `fetch` also does not pay.

`bench leech` ([T-090](bench.md)) is what separates those three, and it is the
next thing to build for this reason.

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
