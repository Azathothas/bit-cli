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
| [T-001](webseed.md) | P0 | webseed | open | Measure the loopback bridge against a raw curl ceiling |
| [T-002](webseed.md) | P1 | webseed | open | Measure Candidate A-prime, the in-process virtual peer |
| [T-003](webseed.md) | P1 | webseed | open | The piece picker cannot be told to prefer HTTP |
| [T-004](webseed.md) | P2 | webseed | open | BEP 17 style is not auto-detected, only declared |
| [T-005](webseed.md) | P3 | webseed | open | A source restricted mid-run cannot be re-scoped |
| [T-006](webseed.md) | P1 | webseed | open | Prove the failure matrix against a real mirror |
| [T-010](disk-io.md) | P1 | disk-io | open | pwrite takes a read lock where it needs a write lock |
| [T-011](disk-io.md) | P1 | disk-io | open | No file handle pool, so long runs exhaust descriptors |
| [T-012](disk-io.md) | P2 | disk-io | open | Preallocation is not implemented |
| [T-013](disk-io.md) | P2 | disk-io | open | Selecting a subset of files still creates all of them |
| [T-014](disk-io.md) | P2 | disk-io | open | Adding a torrent can fail with "File exists (os error 17)" |
| [T-015](disk-io.md) | P1 | disk-io | open | Hash checking can hang at 0 percent |
| [T-016](disk-io.md) | P2 | disk-io | open | fastresume is not used when adding a torrent |
| [T-020](peers.md) | P0 | peers | open | Connections accumulate in CLOSE_WAIT until TCP is unusable |
| [T-021](peers.md) | P0 | peers | open | A temporary network drop stops the download permanently |
| [T-022](peers.md) | P1 | peers | open | Peer connections churn on IPv6-only swarms |
| [T-023](peers.md) | P1 | peers | **done** | The listen port is chosen without checking both address families |
| [T-024](peers.md) | P2 | peers | open | Per-peer choke and unchoke history is not reported |
| [T-025](peers.md) | P3 | peers | open | PeerStatsFilterState is not exported, so the filter is built by JSON |
| [T-030](performance.md) | P0 | performance | open | Throughput collapses with several torrents at once |
| [T-031](performance.md) | P1 | performance | open | The rate limit did not apply to the session |
| [T-032](performance.md) | P1 | performance | open | The piece selector strategy is not implemented |
| [T-033](performance.md) | P2 | performance | open | --split, -x, and -k do not reach the fetch path |
| [T-034](performance.md) | P3 | performance | open | Endgame mode is not observable |
| [T-040](memory.md) | P0 | memory | open | Memory and descriptors grow without bound over a long run |
| [T-041](memory.md) | P2 | memory | open | Per-source window cache is bounded but not measured |
| [T-042](memory.md) | P1 | memory | open | Peak RSS is not captured in any report |
| [T-050](dht.md) | P2 | dht | open | The DHT cache costs disk I/O even when nothing is running |
| [T-051](dht.md) | P2 | dht | open | A magnet with no DHT and no trackers fails without saying so |
| [T-052](dht.md) | P3 | dht | open | DHT is not reported |
| [T-060](trackers.md) | P1 | trackers | open | The announced port is wrong when no port is configured |
| [T-061](trackers.md) | P1 | trackers | open | bit-cli trackers announces a fixed port |
| [T-062](trackers.md) | P1 | trackers | open | Announce timing has no started, completed, or stopped events |
| [T-063](trackers.md) | P3 | trackers | open | Tracker tiers are announced in parallel rather than in order |
| [T-064](trackers.md) | P2 | trackers | open | UDP tracker retry does not follow the BEP 15 backoff |
| [T-065](trackers.md) | P3 | trackers | open | Scrape is only implemented for the BEP 48 URL convention |
| [T-070](windows.md) | P1 | windows | open | A downloaded executable cannot be run until the process exits |
| [T-071](windows.md) | P0 | windows | open | Reserved device names in torrent paths are not sanitised |
| [T-072](windows.md) | P0 | windows | open | Case-colliding paths silently overwrite |
| [T-073](windows.md) | P1 | windows | open | Long paths are not tested |
| [T-074](windows.md) | P1 | windows | **done** | A false hash-check pass on empty files |
| [T-075](windows.md) | P2 | windows | open | PowerShell redirection encoding is not documented |
| [T-080](create-seed.md) | P1 | create | open | librqbit's create_torrent writes an extra piece hash |
| [T-081](create-seed.md) | P1 | create | open | BEP 52 v2 and hybrid torrents are not implemented |
| [T-082](create-seed.md) | P2 | seeding | open | BEP 16 superseeding is not implemented |
| [T-083](create-seed.md) | P2 | seeding | open | Seeding does not report choke state or disconnect reasons |
| [T-084](create-seed.md) | P0 | create | open | The create round trip has not been proven against another client |
| [T-085](create-seed.md) | P1 | create | open | Creation determinism is not proven across platforms |
| [T-090](bench.md) | P0 | bench | open | bit-cli bench is not implemented |
| [T-091](bench.md) | P0 | bench | open | Bench reports do not capture their environment |
| [T-092](bench.md) | P1 | bench | open | bench swarm has no synthetic load generator |
| [T-093](bench.md) | P2 | bench | open | --baseline comparison is not implemented |
| [T-094](bench.md) | P2 | bench | open | Trace output has no measured cost |
| [T-100](bep-coverage.md) | P2 | bep | open | BEP 6 fast extension is not implemented |
| [T-101](bep-coverage.md) | P3 | bep | open | uTP is available but untested |
| [T-102](bep-coverage.md) | P3 | bep | open | BEP 55 holepunch is not implemented |
| [T-103](bep-coverage.md) | P2 | bep | open | Filenames that are not valid UTF-8 are refused |
| [T-110](cli-surface.md) | P1 | cli | partial | The --jsonl event stream is incomplete |
| [T-111](cli-surface.md) | P2 | cli | open | piece_verified and file_completed are derived from polling |
| [T-112](cli-surface.md) | P1 | cli | open | --log-file does not write or rotate anything |
| [T-113](cli-surface.md) | P1 | cli | open | Metalink is not implemented |
| [T-114](cli-surface.md) | P2 | cli | open | -i/--input-file batch input is not implemented |
| [T-115](cli-surface.md) | P2 | cli | partial | Hooks do not fire for every documented trigger |
| [T-116](cli-surface.md) | P3 | cli | open | -O/--index-out cannot rename a file |
| [T-117](cli-surface.md) | P1 | cli | open | --schema-version has no schema behind it |
| [T-118](cli-surface.md) | P2 | cli | open | The short-flag table is not checked in CI |
| [T-120](licensing.md) | P1 | licensing | open | THIRD_PARTY.md is not generated |
| [T-121](licensing.md) | P1 | licensing | open | No cargo-deny configuration |
| [T-122](reference-map.md) | P2 | licensing | open | reference/ is not deleted at the end of Phase B |
| [T-200](phase-c.md) | — | phase-c | deferred | Session daemon |
| [T-201](phase-c.md) | — | phase-c | deferred | JSON-RPC and XML-RPC, with aria2 method parity |
| [T-202](phase-c.md) | — | phase-c | deferred | Queue management across invocations |
| [T-203](phase-c.md) | — | phase-c | deferred | Session save and restore |
| [T-204](phase-c.md) | — | phase-c | deferred | Persistent attached web seeds |
| [T-205](phase-c.md) | — | phase-c | deferred | Download result registry |
| [T-206](phase-c.md) | — | phase-c | deferred | GID assignment |
| [T-207](phase-c.md) | — | phase-c | deferred | Session-attached verbs from the old TUI |
| [T-208](phase-c.md) | — | phase-c | deferred | status --follow against a live session |
| [T-209](phase-c.md) | — | phase-c | deferred | Watch directories, RSS, cluster mode, and the control service |

## Counts

79 items: 69 to work through, and 10 deferred to Phase C.

| Priority | Open | Partial | Done |
| --- | --- | --- | --- |
| P0 | 10 | 0 | 0 |
| P1 | 24 | 1 | 2 |
| P2 | 22 | 1 | 0 |
| P3 | 9 | 0 | 0 |
| Phase C | 10 deferred | | |

## Start here

The P0 list, in the order that unblocks the most:

1. [T-084](create-seed.md) — prove the create round trip against another
   client. A torrent nobody else can read is not a torrent, and this is the
   most important untested claim in the tool.
2. [T-071](windows.md) and [T-072](windows.md) — path sanitisation and case
   collisions. Both are silent data loss on legal torrents.
3. [T-001](webseed.md) — the web seed benchmark. Every architectural decision
   about the fetch path waits on this number.
4. [T-020](peers.md), [T-021](peers.md), [T-030](performance.md),
   [T-040](memory.md) — the four long-run failures. Likely fewer than four
   distinct defects; measure before theorising.
5. [T-090](bench.md) and [T-091](bench.md) — `bench`, because several of the
   items above close with a measurement and there is nowhere to record one.
