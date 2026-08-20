# TODO

Every item, one line each. Work through this file; each entry closes with the
acceptance command from its own page, actually run, with the output recorded.

Nothing here closes as "won't fix", "upstream problem", or "out of scope".
Upstream has no interest in this work, so there is nowhere to defer to. An item
that is genuinely blocked stays open with the blocker named and what would
unblock it.

`phase-c.md` is the exception: it is written and never worked on, by decision
7.4.

## Sources

Built from the upstream `rqbit` corpus fetched on 2026-08-19 with `gh`:
262 issues (91 open, 171 closed) and 346 pull requests, preserved under
`reference/rqbit/` and categorised by `scripts/triage.jq`.

Category counts from the triage, which is why the files are sized the way they
are: bep 66, seeding 48, trackers 41, peers 40, windows 38, disk-io 37,
performance 31, bench 26, create 25, network 25, dht 22, memory 16, webseed 1,
uncategorised 72. Several issues fall in more than one category.

The webseed count of 1 is not a gap in the triage. Web seeding is what
`bit-cli` adds, so `webseed.md` is design work rather than a defect list.

## Priority

- **P0** breaks correctness, loses data, or takes the process down.
- **P1** a documented capability does not work, or a flag does nothing.
- **P2** worth doing, nothing is wrong without it.
- **P3** worth recording so it is not rediscovered.

## Effort

S is under a day, M is a few days, L is a week, XL is longer.

---

| ID | Priority | Category | Status | Item |
| --- | --- | --- | --- | --- |
| [T-001](webseed.md) | P0 | webseed | **done** | Measure the loopback bridge against a raw curl ceiling |
| [T-002](webseed.md) | P1 | webseed | **done** | Measure Candidate A-prime, the in-process virtual peer |
| [T-003](webseed.md) | P1 | webseed | **done** | The piece picker cannot be told to prefer HTTP |
| [T-004](webseed.md) | P2 | webseed | open | BEP 17 style is not auto-detected, only declared |
| [T-005](webseed.md) | P3 | webseed | open | A source restricted mid-run cannot be re-scoped |
| [T-006](webseed.md) | P1 | webseed | **done** | Prove the failure matrix against a real mirror |
| [T-007](webseed.md) | P2 | webseed | open | A stalling source takes 24 seconds to give up |
| [T-008](webseed.md) | P3 | webseed | open | A duplicate block request is fetched twice |
| [T-009](webseed.md) | P1 | webseed | **done** | A source cannot be attached over more than one connection |
| [T-141](webseed.md) | P1 | webseed | open | --web-seed-connect-timeout does not bound a connect that never answers |
| [T-010](disk-io.md) | P1 | disk-io | **done** | pwrite takes a read lock where it needs a write lock |
| [T-011](disk-io.md) | P1 | disk-io | **done** | No file handle pool, so long runs exhaust descriptors |
| [T-012](disk-io.md) | P2 | disk-io | **done** | Preallocation is not implemented |
| [T-013](disk-io.md) | P2 | disk-io | **done** | Selecting a subset of files still creates all of them |
| [T-014](disk-io.md) | P2 | disk-io | **done** | Adding a torrent can fail with "File exists (os error 17)" |
| [T-015](disk-io.md) | P1 | disk-io | **done** | Hash checking can hang at 0 percent |
| [T-016](disk-io.md) | P2 | disk-io | blocked | fastresume is not used when adding a torrent |
| [T-017](disk-io.md) | P1 | disk-io | **done** | Concurrent receive paths contend on the payload file |
| [T-018](disk-io.md) | P2 | disk-io | open | The write path issues one operation per 16 KiB block |
| [T-020](peers.md) | P0 | peers | open | Connections accumulate in CLOSE_WAIT until TCP is unusable |
| [T-021](peers.md) | P0 | peers | **done** | A temporary network drop stops the download permanently |
| [T-022](peers.md) | P1 | peers | open | Peer connections churn on IPv6-only swarms |
| [T-023](peers.md) | P1 | peers | **done** | The listen port is chosen without checking both address families |
| [T-024](peers.md) | P2 | peers | open | Per-peer choke and unchoke history is not reported |
| [T-025](peers.md) | P3 | peers | open | PeerStatsFilterState is not exported, so the filter is built by JSON |
| [T-142](peers.md) | P1 | peers | **done** | bit-cli peers never joined the swarm it was sampling |
| [T-138](peers.md) | P2 | peers | **done** | A peer that comes back waits out a backoff that grows by six |
| [T-030](performance.md) | P0 | performance | **done** | Throughput collapses with several torrents at once |
| [T-031](performance.md) | P1 | performance | **done** | The rate limit did not apply to the session |
| [T-032](performance.md) | P1 | performance | open | The piece selector strategy is not implemented |
| [T-033](performance.md) | P2 | performance | open | --split, -x, and -k do not reach the fetch path |
| [T-034](performance.md) | P3 | performance | open | Endgame mode is not observable |
| [T-035](performance.md) | P1 | performance | **done** | The web seed rate limit was never applied |
| [T-036](performance.md) | P0 | paths | **done** | A multi-file torrent with one file lands without its directory |
| [T-037](performance.md) | P1 | performance | **done** | A run stalls for minutes, roughly once in fifty |
| [T-040](memory.md) | P0 | memory | partial | Memory and descriptors grow without bound over a long run |
| [T-041](memory.md) | P2 | memory | open | Per-source window cache is bounded but not measured |
| [T-042](memory.md) | P1 | memory | **done** | Peak RSS is not captured in any report |
| [T-050](dht.md) | P2 | dht | open | The DHT cache costs disk I/O even when nothing is running |
| [T-051](dht.md) | P2 | dht | open | A magnet with no DHT and no trackers fails without saying so |
| [T-052](dht.md) | P3 | dht | open | DHT is not reported |
| [T-060](trackers.md) | P1 | trackers | **done** | The announced port is wrong when no port is configured |
| [T-061](trackers.md) | P1 | trackers | **done** | bit-cli trackers announces a fixed port |
| [T-062](trackers.md) | P1 | trackers | **done** | Announce timing has no started, completed, or stopped events |
| [T-063](trackers.md) | P3 | trackers | open | Tracker tiers are announced in parallel rather than in order |
| [T-064](trackers.md) | P2 | trackers | open | UDP tracker retry does not follow the BEP 15 backoff |
| [T-065](trackers.md) | P3 | trackers | open | Scrape is only implemented for the BEP 48 URL convention |
| [T-070](windows.md) | P1 | windows | **done** | A downloaded executable cannot be run until the process exits |
| [T-071](windows.md) | P0 | windows | **done** | Reserved device names in torrent paths are not sanitised |
| [T-072](windows.md) | P0 | windows | **done** | Case-colliding paths silently overwrite |
| [T-073](windows.md) | P1 | windows | **done** | Long paths are not tested |
| [T-074](windows.md) | P1 | windows | **done** | A false hash-check pass on empty files |
| [T-075](windows.md) | P2 | windows | open | PowerShell redirection encoding is not documented |
| [T-076](windows.md) | P2 | windows | **done** | seed and verify do not report renamed paths |
| [T-080](create-seed.md) | P1 | create | **done** | librqbit's create_torrent writes an extra piece hash |
| [T-081](create-seed.md) | P1 | create | open | BEP 52 v2 and hybrid torrents are not implemented |
| [T-082](create-seed.md) | P2 | seeding | open | BEP 16 superseeding is not implemented |
| [T-083](create-seed.md) | P2 | seeding | open | Seeding does not report choke state or disconnect reasons |
| [T-084](create-seed.md) | P0 | create | **done** | The create round trip has not been proven against another client |
| [T-085](create-seed.md) | P1 | create | partial | Creation determinism is not proven across platforms |
| [T-090](bench.md) | P0 | bench | partial | bit-cli bench is not implemented |
| [T-091](bench.md) | P0 | bench | **done** | Bench reports do not capture their environment |
| [T-092](bench.md) | P1 | bench | open | bench swarm has no synthetic load generator |
| [T-093](bench.md) | P2 | bench | **done** | --baseline comparison is not implemented |
| [T-094](bench.md) | P2 | bench | open | Trace output has no measured cost |
| [T-100](bep-coverage.md) | P2 | bep | open | BEP 6 fast extension is not implemented |
| [T-101](bep-coverage.md) | P3 | bep | open | uTP is available but untested |
| [T-102](bep-coverage.md) | P3 | bep | open | BEP 55 holepunch is not implemented |
| [T-103](bep-coverage.md) | P2 | bep | open | Filenames that are not valid UTF-8 are refused |
| [T-110](cli-surface.md) | P1 | cli | **done** | The --jsonl event stream is incomplete |
| [T-111](cli-surface.md) | P2 | cli | open | piece_verified and file_completed are derived from polling |
| [T-112](cli-surface.md) | P1 | cli | **done** | --log-file does not write or rotate anything |
| [T-113](cli-surface.md) | P1 | cli | open | Metalink is not implemented |
| [T-114](cli-surface.md) | P2 | cli | open | -i/--input-file batch input is not implemented |
| [T-115](cli-surface.md) | P2 | cli | partial | Hooks do not fire for every documented trigger |
| [T-116](cli-surface.md) | P3 | cli | open | -O/--index-out cannot rename a file |
| [T-117](cli-surface.md) | P1 | cli | **done** | --schema-version has no schema behind it |
| [T-118](cli-surface.md) | P2 | cli | open | The short-flag table is not checked in CI |
| [T-120](licensing.md) | P1 | licensing | **done** | THIRD_PARTY.md is not generated |
| [T-121](licensing.md) | P1 | licensing | **done** | No cargo-deny configuration |
| [T-122](reference-map.md) | P2 | licensing | open | reference/ is not deleted at the end of Phase B |
| [T-130](multi-source.md) | P1 | webseed | **done** | A source cannot be told which statuses are worth retrying |
| [T-131](multi-source.md) | P1 | bench | **done** | The loopback file server cannot simulate a signed URL |
| [T-132](multi-source.md) | P1 | performance | open | The swarm cannot be rate limited separately from HTTP sources |
| [T-133](multi-source.md) | P1 | webseed | **done** | Two torrents holding the same file cannot share its bytes |
| [T-134](multi-source.md) | P2 | bep | open | v1 and v2 info hashes are not reconciled |
| [T-135](multi-source.md) | P2 | performance | open | Source selection cannot be steered by method or by priority at run time |
| [T-136](multi-source.md) | P2 | cli | open | Nothing states the end-to-end integrity guarantee |
| [T-137](multi-source.md) | P2 | webseed | **done** | A cooled-down source never comes back |
| [T-139](multi-source.md) | P1 | cli | **done** | A resumed download charges its existing bytes to the swarm |
| [T-140](multi-source.md) | P2 | webseed | **done** | A proven shared file is not turned into a source on its own |
| [T-143](multi-source.md) | P2 | webseed | open | A source cannot be attached to a torrent that has already started |
| [T-200](phase-c.md) | n/a | phase-c | deferred | Session daemon |
| [T-201](phase-c.md) | n/a | phase-c | deferred | JSON-RPC and XML-RPC, with aria2 method parity |
| [T-202](phase-c.md) | n/a | phase-c | deferred | Queue management across invocations |
| [T-203](phase-c.md) | n/a | phase-c | deferred | Session save and restore |
| [T-204](phase-c.md) | n/a | phase-c | deferred | Persistent attached web seeds |
| [T-205](phase-c.md) | n/a | phase-c | deferred | Download result registry |
| [T-206](phase-c.md) | n/a | phase-c | deferred | GID assignment |
| [T-207](phase-c.md) | n/a | phase-c | deferred | Session-attached verbs from the old TUI |
| [T-208](phase-c.md) | n/a | phase-c | deferred | status --follow against a live session |
| [T-209](phase-c.md) | n/a | phase-c | deferred | Watch directories, RSS, cluster mode, and the control service |

## Counts

102 items: 92 to work through, and 10 deferred to Phase C. Fifteen were added by
measurements rather than by the triage. T-007 came out of T-001: a stalling
source takes 24 seconds to give up. T-008, T-009, and T-017 came out of
T-090's `bench leech` runs: a duplicate block request is fetched twice, a
source cannot be attached over more than one connection, and concurrent
receive paths contend on the payload file. T-018 came out of T-017's own
measurement, which found the contention is charged per write operation rather
than per byte. T-036 and T-037 came out of T-030's: a multi-file torrent with
one file lands without its directory, and a run stalls for minutes roughly
once in fifty. T-035 came out of building the T-003 acceptance, which needed a
slow mirror and found that the flag for one did nothing. T-138 came out of
closing T-021: a peer that comes back waits out a backoff that grows by six.

T-130 through T-136 came from the operator rather than from the triage: five
scenarios about pointing several kinds of source at one payload. T-137 came
out of closing T-130: making `--web-seed-max-errors` reachable made
`--web-seed-cooldown` reachable too, and it turned out to set a timer nothing
waits out. T-139 and T-140 came out of closing T-133 layer 2: a resumed
download charged its existing bytes to the swarm, and a proven shared file is
still not turned into a source on its own.
[multi-source.md](multi-source.md) records which of the five scenarios work,
with the commands that were run, and what the rest need.

T-141 and T-142 came out of building T-117's last eight fixtures. A source at
a blackholed address makes no request until the request timeout, so
`--web-seed-connect-timeout` bounds nothing and the source is never retired.
And `bit-cli peers` added its torrent paused, which in `librqbit` 9.0.0 means
it never announced: every run of that command had reported an empty swarm.
T-143 is what T-140 left behind: a source can only be attached before a
torrent starts, so a file donated by another torrent in the same run is used
under `-j 1` and not above it.

| Priority | Open | Partial | Blocked | Done |
| --- | --- | --- | --- | --- |
| P0 | 1 | 2 | 0 | 8 |
| P1 | 7 | 1 | 0 | 30 |
| P2 | 23 | 1 | 1 | 8 |
| P3 | 10 | 0 | 0 | 0 |
| Phase C | 10 deferred | | | |

`blocked` is one item, [T-016](disk-io.md): a resume cache cannot be built on
`librqbit` 9.0.0 without turning on the session persistence that decision 7.4
puts in Phase C. It stays here rather than moving, with the upstream lines that
block it and what would unblock it.

## Start here

What is settled and what is next, in the order that unblocks the most. The
first six are done or mostly done, item seven is where the open P0 work is, and
item eight is the operator's own list.

1. [T-084](create-seed.md) is **done**. `bit-cli create`, `verify`, and `seed`
   round trip byte for byte through `aria2c` 1.37.0 for v1, `--private`, and
   `--web-seed`. Run it with
   `pwsh -NoProfile -File scripts/interop-roundtrip.ps1`. The `--version
   hybrid` case is still uncovered and waits on [T-081](create-seed.md).
2. [T-071](windows.md) and [T-072](windows.md) are **done**. Every torrent path
   is planned before anything is opened, so no file leaves the output
   directory, no name the filesystem refuses fails a download, and two names
   that collide only on NTFS both land. The mapping is in `--json`. A `C:`
   component escaping the output directory turned up while fixing it and is
   fixed too.
3. [T-091](bench.md), [T-042](memory.md), and [T-093](bench.md) are **done**,
   and [T-090](bench.md) is **partial**. Every `bench` report carries the
   machine it was taken on, the exact command line, and what the process cost
   in memory, CPU, and handles. `bench webseed` measures HTTP sources with
   latency percentiles, a concurrency curve, per-source attribution, and error
   counts by class and by HTTP status. `bench leech` measures a download and
   splits its cost between the request pipeline, piece verification, and the
   disk, all three measured rather than modelled. `--fail-under` exits 14 and
   `--baseline` prints a delta per metric. `bench seed` measures a seeder with
   every counter facing the other way: bytes sent per peer, and positioned
   reads rather than writes. `bench probe` is a one-shot reachability and
   capability check against a peer address or an HTTP endpoint: the handshake,
   the reserved bytes, the extended handshake, and what the peer volunteers,
   or the status, the range support, and the TLS parameters. `swarm` is the
   one still unbuilt, and it says so.
4. [T-001](webseed.md) is **done**, and so is [T-006](webseed.md).
   `scripts/bench-webseed.ps1` takes the number in four stages so the cost is
   attributed rather than asserted, and it was run twice: on loopback and
   against a real mirror. `scripts/bench-leech.ps1` then took it apart.

   **The bridge costs about five sixths of the available throughput, and the
   reason is that one source is one peer.** A block arriving from a peer is
   written, and at a piece boundary the whole piece is read back and hashed,
   inline on that connection's own task before the next block from that peer
   is processed. One connection reaches 18.18% of `bit-cli`'s own HTTP path on
   loopback; the same source over two reaches 34.90%, which is 1.92x.

   Three things it is **not**. Not the requests in flight: the same 64
   requests on one connection reach 0.81x, slightly worse than 8 on the same
   connection. Not the request window: the bridge sees `librqbit`'s 128 block
   window reached, but the run sits at 40% of what that peak would allow. Not
   hashing: piece checks are 11% of a one-connection run. It looked like the
   disk was the second wall and the thing flattening the curve after two, and
   [T-017](disk-io.md) measured that separately and found it is not: storage
   moves 1.31 GiB/s at eight writers on one file, which is 3.3 times what the
   eight-bridge run asks of it.

   The fix the measurement points at is [T-009](webseed.md), and it is
   **done**. `--web-seed-connections <N>` presents one source over N
   connections, which is N peers and so N receive paths. Two is worth 1.92x on
   loopback and the curve is flat after that. The same requests in flight on
   one connection are worth 0.81x, which is what says it is the paths. The N
   connections share one fetcher, so the mirror serves the payload once: the
   same eight peers built as eight separate sources at one URL pulled it 3.98
   times over, which was not visible until the report carried the HTTP bytes
   beside the served bytes.

   [T-002](webseed.md) is **done** and the answer is no: `librqbit` 9.0.0 has
   no public way to hand the session a peer that is not a socket. Every route
   in takes a `SocketAddr`. What is worth knowing is that the machinery
   underneath already takes an arbitrary byte stream, so an in-process peer
   needs four visibility changes and no redesign. That entry names the five
   file and line references a reader can check.

   `bit-cli`'s HTTP path beats `curl` over a real network, at 156.71% of eight
   parallel `curl` slices. The failure matrix ran against all 468 web seeds in
   the Arch torrent, and two defects that had made `webseed test` unusable
   against any HTTPS mirror were found and fixed there.
5. The disk I/O cluster. [T-010](disk-io.md), [T-011](disk-io.md),
   [T-012](disk-io.md), [T-013](disk-io.md), [T-014](disk-io.md), and
   [T-015](disk-io.md) are **done**, and so are the two Windows items that
   depended on the same storage, [T-070](windows.md) and
   [T-076](windows.md). [T-016](disk-io.md) is blocked upstream.

   One change closed most of them: a payload file opens when it is first
   touched, a read opens for reading only and does not create it, and a write
   opens for writing and does. So a file selection no longer creates the files
   it did not select, `--max-open-files` is a real cap on descriptors, and a
   downloaded executable runs while `bit-cli seed` is serving it.

   All four `--file-allocation` methods now do four different things, measured
   by volume free space before the payload arrives.
   `scripts/check-handles.ps1` and `scripts/check-allocation.ps1` are the
   acceptance for those two.
6. [T-003](webseed.md) and [T-035](performance.md) are **done**.
   `--prefer-web-seed` doubles a source's connections rather than its request
   budget, and moves the HTTP share of a hybrid run from a mean of 46.72% to
   62.60% across five paired runs. `scripts/check-prefer.ps1` is the
   measurement. It found `--web-seed-speed-limit` accepted and ignored, which
   is T-035, now a token bucket per source.
7. The long-run failures. [T-017](disk-io.md), [T-030](performance.md),
   [T-021](peers.md), [T-031](performance.md), [T-037](performance.md), and
   [T-138](peers.md) are **done**. [T-020](peers.md) is the one open P0 left,
   and [T-040](memory.md) is partial: it needs six hours of wall clock and
   nothing else. Measure before theorising: every one of these
   closed that way, and every time the answer was not what the entry
   predicted.

   [T-021](peers.md) does both things its acceptance allows, and which one
   depends on a number nobody had looked at. `librqbit` retries a dropped peer
   at 10s, then 70s, then 430s: a factor of six. An outage that ends between
   two attempts waits for the next one however long the network has been back,
   which is what makes the report read as "never recovers". Measured, a 40
   second outage is caught by the 70 second attempt and the download completes;
   a 120 second one is not, and the run sits until `--stop-timeout` fires. The
   residue was [T-138](peers.md), and it is **done**: `--redial-after 30s`
   pauses and restarts the torrent when nothing has arrived for that long,
   which drops the backoff counters with the live state and costs no hash
   check. The same 120 second outage that leaves the run at 17 of 128 MiB with
   the flag off completes with it, in four re-dials.

   [T-020](peers.md) was two defects. One is fixed: `librqbit`'s accept loop
   panicked when its pending handshake-check set filled and a check failed,
   which killed the listener while the process kept reporting itself as
   seeding. 3000 connections that closed before handshaking did it in 79
   seconds. The other is open: those same connections strand a socket about
   half the time, accumulating linearly, released by later traffic and never
   by time. `--max-handles` bounds it with a loud exit 16.

   What the soak adds: `CLOSE_WAIT` was zero at every sample of a 2.26 hour
   `steady` run and of a 2.55 hour `idle` one, so this needs the churn shape
   and does not appear under a deployment-shaped load.

   [T-017](disk-io.md) rules the disk out rather than in. `bit-cli bench disk`
   writes the same bytes through the same storage from N threads with no
   session, and it found that writes to one file serialise whatever handle
   they arrive on, that the serialisation is charged per operation rather than
   per byte, and that even fully contended the write path moves 1.31 GiB/s
   against the 408 MiB/s an eight-bridge `bench leech` run asks of it. The
   residue is [T-018](disk-io.md), worth at most 18%.

   [T-030](performance.md) was real and was two defects, neither of them
   contention. Completion was noticed on the next `--report-interval` tick
   rather than when it happened, which cost up to a second per torrent, and a
   multi-file torrent holding one file lost its directory, so four torrents in
   one invocation wrote to one file, destroyed each other's payload, and all
   reported success. That second one is [T-036](performance.md), a P0 in its
   own right, and it is what made the first report look like contention: four
   torrents on one file is exactly the shape [T-017](disk-io.md) measured as
   the one that does not scale. With both fixed, `-j 4` moves four torrents
   3.54 times faster than one invocation at a time, at 72% of what the HTTP
   source serves with no torrent machinery at all.

   [T-037](performance.md) was what was left of [T-030](performance.md): one
   run in about seventy stalls for minutes and then completes. It is **done**
   by its acceptance's second branch. A bridge now reports how many times it
   lost its connection to the session, what it waited to make another, and
   what ended the attempt before it, and `scripts/check-stall.ps1` ran the same
   command 200 times twice: median 957 ms and slowest 1201 ms at four
   connections, a ratio of 1.25 against a ceiling of 5, with **zero reconnects
   in 400 invocations**. The reproduction the first branch asks for is still
   unreached, and the counters are what will name it if it happens again.

   [T-042](memory.md) built the sampler these need, and `download` and `seed`
   report their own peak RSS, CPU time, and handle count, in the final report
   and in every `progress` event. [T-011](disk-io.md) removed one of the two
   things [T-040](memory.md) names: descriptors are now bounded by a flag.

   [T-040](memory.md) is **partial** and is the first thing to start, because
   it is six hours of wall clock that nothing else has to wait for.
   `scripts/soak.ps1` samples a long-lived seeder under one of six workloads,
   writes the three series to `bench/soak-<timestamp>.csv`, and rewrites
   `bench/soak-<timestamp>.json` after every sample, so a run that is killed
   still leaves its slopes. Three partial runs are recorded in the entry. The
   `idle` control is the useful new one: **188 handles at every one of 291
   samples over 2.55 hours**, resident memory flat at 0.04 MiB an hour, and
   `CLOSE_WAIT` zero. So the descriptors half of this entry does not reproduce
   at all, and whatever the `steady` load does is the load. Under `steady`,
   over 2.26 hours: `CLOSE_WAIT` zero at all 258 samples, handles noise, and
   resident memory **0.93 MiB an hour at an r squared of 0.65** with a maximum
   above its last reading, which is a series that rises and falls rather than
   one that climbs. Six hours is what separates a settling curve from a leak.
   Both commands are in the entry.
8. [multi-source.md](multi-source.md), the operator's five scenarios about
   pointing several kinds of source at one payload. Read that file before
   starting any of T-130 to T-143: its first part records which scenarios
   already work, with the commands that were run and the output, so the work
   left is smaller than the list of entries suggests.

   **Four of the five work in full, and Scenario 2 now needs no flags at
   all.** [T-131](multi-source.md),
   [T-130](multi-source.md), and [T-137](multi-source.md) are **done**, which
   closed Scenario 1 and Scenario 4. The file server signs, redirects, expires
   a signature, and falls over on a clock, and
   `--web-seed-retry-status`, `--web-seed-fatal-status`, and
   `--web-seed-cooldown` decide which statuses retire a source and whether it
   comes back. `pwsh scripts/check-signed-source.ps1` drives nine cases and is
   the acceptance for all three.

   Closing them found two things the entries did not predict. A signature
   never expires against `bit-cli` at a realistic window, because it
   re-resolves the stable URL on every request; the sweep that establishes
   that is under [T-131](multi-source.md). And the bridge retired a source on
   the first request that ran out of retries, so a mirror that restarted
   mid-download was lost with **no flag set at all** and
   `--web-seed-max-errors` could never be reached. Both are written up under
   their entries. [T-137](multi-source.md) came out of the second.

   [T-133](multi-source.md) **layers 1 and 2 are done, and Scenario 2 is one
   invocation.** A source URL may be `file:`, and a `--web-seed-for` selector
   may name one torrent by info hash, and `-j 1` starts sources in the order
   they were given, so torrent C reads the CDN copy and A and B read what C
   wrote. Measured: exactly one source touched the CDN, 192 MiB over sources,
   one distinct hash across three info hashes and three piece lengths.
   `pwsh scripts/check-local-source.ps1` is the acceptance, eight cases, no
   server and no bound port.

   Layer 3's **detection** is done too. `bit-cli files <T> --against <OTHER>`
   decides from the metadata alone whether two torrents hold the same file, and
   says what the answer rests on: `piece-hashes` when the pieces line up and
   agree, `length` when only the size matches. Against the three-torrent
   fixture nothing is provable, because three different piece lengths is
   exactly the case where piece hashes cannot be compared, and two of its four
   length-only candidates are not the same bytes at all. Against a pair built
   to line up, the whole 64 MiB is proven.

   [T-140](multi-source.md) closed the rest of layer 3, so **Scenario 2 needs
   no flags at all**. `bit-cli download c.torrent a.torrent b.torrent -j 1`
   compares every pair of torrents by the piece hashes covering each file
   before the session starts, and gives each one a `file:` source per file an
   earlier torrent has already written. Measured over three info hashes with
   the shared file at a different path and index in each: 16 MiB fetched once
   over HTTP, read off the disk twice, one distinct hash across three output
   directories, in 511 ms.
   `pwsh scripts/check-shared-files.ps1` is the acceptance.

   Two things bound it and both are recorded. Above `-j 1` nothing has finished
   yet, so nothing is donated: attaching a source to a torrent that has already
   started is [T-143](multi-source.md). And real control of which source
   answers a piece is [T-135](multi-source.md), already priced by
   [T-002](webseed.md) and [T-003](webseed.md).

   `scripts/make-scenario-fixture.ps1` builds the payloads, the three
   torrents, the CDN copy, and the partial on-disk state the acceptances
   start from. `-PieceLength` gives all three one piece length, which is what
   makes the shared file provable from the metadata rather than only
   assertable, and `-WebSeed` puts a real URL in torrent C's url-list.
