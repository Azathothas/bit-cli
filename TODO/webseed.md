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
Status:      open

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
Status:      open

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
