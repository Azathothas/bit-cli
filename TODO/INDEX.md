# TODO

Every item, one line each. Work through this file; each entry closes with the
acceptance command from its own page, actually run, with the output recorded.

**Read [PROGRESS.md](PROGRESS.md) first** for what the last session did and
where to resume, and [RULES.md](RULES.md) for how this repository is worked on,
including the only sanctioned way to commit and push.

Nothing here closes as "won't fix", "upstream problem", or "out of scope".
Upstream has no interest in this work, so there is nowhere to defer to. An item
that is genuinely blocked stays open with the blocker named and what would
unblock it.

`phase-c.md` is the exception: it is written and never worked on, by decision
7.4.

## Sources

Built from the upstream `rqbit` corpus fetched on 2026-08-19 with `gh`:
262 issues (91 open, 171 closed) and 346 pull requests, categorised by
`scripts/triage.jq`. `reference-map.md` keeps the licence determination for
every tree that has been read. **No entry here depends on a tree that is
gone**: what each one needed is written into the entry, which is what
[T-122](reference-map.md) closed.

A second corpus arrived on 2026-08-21: **twenty-two upstream BitTorrent
implementations**, indexed by `reference/RESEARCH.md`, all permissive (twenty-one
MIT, `intermodal` CC0-1.0). Entries below cite it as
`repository/path/file.ext:line`. Those citations are evidence of what somebody
else did and never evidence that `bit-cli` does it; where an entry now rests on
one, it says which. `reference/` is gitignored and is never committed.

**`TODO/` is the authoritative record.** There is no other document that
binds. The operator's working brief, which earlier revisions of these files
cited by section number, is retired: it was never tracked, it was superseded by
this directory, and everything in it that still binds is written down here. The
`Source:` lines that named it now say "the operator's brief", because what a
`Source:` line records is where an entry came from rather than a path a reader
must be able to open. That is the same rule the `rqbit` issue JSON has always
been read under, and the JSON is not on disk either.

Two things it held that nothing else did are now written out in full:
the aria2 parity checklist is in [phase-c.md](phase-c.md), and the rules that
govern how this repository is worked on are in [RULES.md](RULES.md).

[PROGRESS.md](PROGRESS.md) is the session state: what the last session did and
what this one is doing. It carries no history, it is rewritten every session,
and it is the first file to read.

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
| [T-005](webseed.md) | P2 | webseed | open | A source restricted mid-run cannot be re-scoped |
| [T-006](webseed.md) | P1 | webseed | **done** | Prove the failure matrix against a real mirror |
| [T-007](webseed.md) | P2 | webseed | open | A stalling source takes 24 seconds to give up |
| [T-008](webseed.md) | P3 | webseed | open | A duplicate block request is fetched twice |
| [T-179](webseed.md) | P2 | webseed | open | A bad piece cannot be attributed to the source that filled it |
| [T-009](webseed.md) | P1 | webseed | **done** | A source cannot be attached over more than one connection |
| [T-141](webseed.md) | P1 | webseed | **done** | --web-seed-connect-timeout does not bound a connect that never answers |
| [T-162](webseed.md) | P1 | bench | **done** | Two bench webseed tests assumed a loaded runner cannot also fail |
| [T-010](disk-io.md) | P1 | disk-io | **done** | pwrite takes a read lock where it needs a write lock |
| [T-011](disk-io.md) | P1 | disk-io | **done** | No file handle pool, so long runs exhaust descriptors |
| [T-012](disk-io.md) | P2 | disk-io | **done** | Preallocation is not implemented |
| [T-013](disk-io.md) | P2 | disk-io | **done** | Selecting a subset of files still creates all of them |
| [T-014](disk-io.md) | P2 | disk-io | **done** | Adding a torrent can fail with "File exists (os error 17)" |
| [T-015](disk-io.md) | P1 | disk-io | **done** | Hash checking can hang at 0 percent |
| [T-016](disk-io.md) | P2 | disk-io | blocked | fastresume is not used when adding a torrent |
| [T-017](disk-io.md) | P1 | disk-io | **done** | Concurrent receive paths contend on the payload file |
| [T-018](disk-io.md) | P2 | disk-io | open | The write path issues one operation per 16 KiB block |
| [T-177](disk-io.md) | P2 | disk-io | open | A piece that spans a file boundary has no adversarial fixture |
| [T-020](peers.md) | P0 | peers | open | Connections accumulate in CLOSE_WAIT until TCP is unusable |
| [T-021](peers.md) | P0 | peers | **done** | A temporary network drop stops the download permanently |
| [T-022](peers.md) | P1 | peers | open | Peer connections churn on IPv6-only swarms |
| [T-023](peers.md) | P1 | peers | **done** | The listen port is chosen without checking both address families |
| [T-024](peers.md) | P2 | peers | open | Per-peer choke and unchoke history is not reported |
| [T-025](peers.md) | P3 | peers | open | PeerStatsFilterState is not exported, so the filter is built by JSON |
| [T-142](peers.md) | P1 | peers | **done** | bit-cli peers never joined the swarm it was sampling |
| [T-138](peers.md) | P2 | peers | **done** | A peer that comes back waits out a backoff that grows by six |
| [T-163](peers.md) | P2 | peers | open | MSE/PE peer encryption is not implemented |
| [T-164](peers.md) | P2 | peers | open | A peer that sends garbage keeps its connection slot |
| [T-165](peers.md) | P2 | peers | open | The peer's reqq is ignored, so the queue depth is a fixed 128 |
| [T-166](peers.md) | P1 | peers | **done** | BEP 10 extension ids are not proven to map in both directions |
| [T-030](performance.md) | P0 | performance | **done** | Throughput collapses with several torrents at once |
| [T-031](performance.md) | P1 | performance | **done** | The rate limit did not apply to the session |
| [T-032](performance.md) | P1 | performance | **done** | The piece selector strategy is not implemented |
| [T-033](performance.md) | P3 | performance | open | --split, -x, and -k do not reach the fetch path *(title disproved: they do not exist)* |
| [T-034](performance.md) | P3 | performance | open | Endgame mode is not observable |
| [T-035](performance.md) | P1 | performance | **done** | The web seed rate limit was never applied |
| [T-036](performance.md) | P0 | paths | **done** | A multi-file torrent with one file lands without its directory |
| [T-037](performance.md) | P1 | performance | **done** | A run stalls for minutes, roughly once in fifty |
| [T-040](memory.md) | P0 | memory | partial | Memory and descriptors grow without bound over a long run |
| [T-157](memory.md) | P2 | memory | **done** | A killed soak destroys the summary it was rewriting |
| [T-041](memory.md) | P2 | memory | open | Per-source window cache is bounded but not measured |
| [T-042](memory.md) | P1 | memory | **done** | Peak RSS is not captured in any report |
| [T-050](dht.md) | P2 | dht | open | The DHT cache costs disk I/O even when nothing is running |
| [T-051](dht.md) | P2 | dht | open | A magnet with no DHT and no trackers fails without saying so |
| [T-052](dht.md) | P3 | dht | open | DHT is not reported |
| [T-169](dht.md) | P3 | dht | open | BEP 33 DHT scrape and BEP 51 infohash indexing are not implemented |
| [T-170](dht.md) | P3 | dht | open | BEP 44 mutable items are not implemented |
| [T-060](trackers.md) | P1 | trackers | **done** | The announced port is wrong when no port is configured |
| [T-061](trackers.md) | P1 | trackers | **done** | bit-cli trackers announces a fixed port |
| [T-062](trackers.md) | P1 | trackers | **done** | Announce timing has no started, completed, or stopped events |
| [T-063](trackers.md) | P3 | trackers | open | Tracker tiers are announced in parallel rather than in order |
| [T-064](trackers.md) | P2 | trackers | open | UDP tracker retry does not follow the BEP 15 backoff |
| [T-065](trackers.md) | P3 | trackers | open | Scrape is only implemented for the BEP 48 URL convention |
| [T-180](trackers.md) | P2 | trackers | open | A negative left in a tracker exchange has no decided handling |
| [T-070](windows.md) | P1 | windows | **done** | A downloaded executable cannot be run until the process exits |
| [T-071](windows.md) | P0 | windows | **done** | Reserved device names in torrent paths are not sanitised |
| [T-072](windows.md) | P0 | windows | **done** | Case-colliding paths silently overwrite |
| [T-073](windows.md) | P1 | windows | **done** | Long paths are not tested |
| [T-074](windows.md) | P1 | windows | **done** | A false hash-check pass on empty files |
| [T-075](windows.md) | P2 | windows | open | PowerShell redirection encoding is not documented |
| [T-076](windows.md) | P2 | windows | **done** | seed and verify do not report renamed paths |
| [T-178](windows.md) | P3 | windows | open | librqbit's Windows pwrite_all can spin forever on a zero-byte write |
| [T-080](create-seed.md) | P1 | create | **done** | librqbit's create_torrent writes an extra piece hash |
| [T-081](create-seed.md) | P1 | create | open | BEP 52 v2 and hybrid torrents are not implemented |
| [T-082](create-seed.md) | P2 | seeding | open | BEP 16 superseeding is not implemented |
| [T-083](create-seed.md) | P2 | seeding | open | Seeding does not report choke state or disconnect reasons |
| [T-084](create-seed.md) | P0 | create | **done** | The create round trip has not been proven against another client |
| [T-085](create-seed.md) | P1 | create | **done** | Creation determinism is not proven across platforms |
| [T-175](create-seed.md) | P2 | create | open | create does not normalise NFD filenames |
| [T-176](create-seed.md) | P2 | create | open | Three lints the corpus names are missing, and one message is wrong |
| [T-090](bench.md) | P0 | bench | partial | bit-cli bench is not implemented |
| [T-091](bench.md) | P0 | bench | **done** | Bench reports do not capture their environment |
| [T-092](bench.md) | P1 | bench | partial | bench swarm has no synthetic load generator |
| [T-093](bench.md) | P2 | bench | **done** | --baseline comparison is not implemented |
| [T-094](bench.md) | P2 | bench | open | Trace output has no measured cost |
| [T-148](bench.md) | P2 | bench | **done** | The peer probe test asserted an exit code inside its own retry loop |
| [T-149](bench.md) | P1 | bench | **done** | The last window of a leech bench was never counted |
| [T-152](bench.md) | P1 | bench | **done** | A disk bench shorter than one sample interval reported no series at all |
| [T-100](bep-coverage.md) | P2 | bep | open | BEP 6 fast extension is not implemented |
| [T-101](bep-coverage.md) | P3 | bep | open | uTP is available but untested *(title disproved: it is not reachable)* |
| [T-102](bep-coverage.md) | P3 | bep | open | BEP 55 holepunch is not implemented |
| [T-103](bep-coverage.md) | P2 | bep | open | Filenames that are not valid UTF-8 are refused |
| [T-167](bep-coverage.md) | P2 | bep | open | BEP 54 lt_donthave is not implemented |
| [T-168](bep-coverage.md) | P3 | bep | open | WebTorrent peers and WSS trackers are not supported |
| [T-171](metainfo.md) | P2 | metainfo | **done** | httpseeds written as a bencoded string is silently dropped |
| [T-172](metainfo.md) | P2 | metainfo | open | Strictness on read is undecided, and the error does not say |
| [T-173](metainfo.md) | P3 | metainfo | open | A zero-length path component has no defined meaning |
| [T-174](metainfo.md) | P2 | metainfo | open | A piece length that is not a multiple of 16 KiB has no fixture |
| [T-110](cli-surface.md) | P1 | cli | **done** | The --jsonl event stream is incomplete |
| [T-111](cli-surface.md) | P2 | cli | open | piece_verified and file_completed are derived from polling |
| [T-112](cli-surface.md) | P1 | cli | **done** | --log-file does not write or rotate anything |
| [T-113](cli-surface.md) | P1 | cli | **done** | Metalink is not implemented |
| [T-114](cli-surface.md) | P2 | cli | open | -i/--input-file batch input is not implemented |
| [T-115](cli-surface.md) | P2 | cli | partial | Hooks do not fire for every documented trigger |
| [T-116](cli-surface.md) | P3 | cli | open | -O/--index-out cannot rename a file |
| [T-117](cli-surface.md) | P1 | cli | **done** | --schema-version has no schema behind it |
| [T-118](cli-surface.md) | P3 | cli | open | The short-flag table is not checked in CI *(title disproved: it is, by four tests)* |
| [T-144](cli-surface.md) | P1 | ci | **done** | The MSRV job fails: the tree needs a newer rustc than it claims |
| [T-145](cli-surface.md) | P2 | ci | **done** | The macOS test job fails to link |
| [T-146](cli-surface.md) | P1 | ci | **done** | CI built a Windows binary against the dynamic C runtime |
| [T-147](windows.md) | P1 | windows | **done** | The rename reason differed by host, so two tests only passed on Windows |
| [T-150](cli-surface.md) | P2 | ci | open | Clippy pins a floating toolchain, so a Rust release can turn the tree red |
| [T-153](cli-surface.md) | P3 | ci | open | Link speeds are not read on macOS |
| [T-154](cli-surface.md) | P2 | cli | open | A Metalink named by URL is not recognised |
| [T-155](cli-surface.md) | P3 | cli | open | --hash-check-only drops the metalink report |
| [T-156](cli-surface.md) | P3 | cli | open | A dry run writes a different shape under the same document kind |
| [T-158](cli-surface.md) | P2 | cli | open | Regenerating the schema deletes fields the sample did not produce |
| [T-159](cli-surface.md) | P3 | cli | open | Subcommand flags are filed under "Report options" in the help |
| [T-160](cli-surface.md) | P1 | ci | **done** | A peers test raced its own seeder |
| [T-161](cli-surface.md) | P3 | ci | open | A CI action still targets Node.js 20, which is deprecated *(four call sites, not two)* |
| [T-151](cli-surface.md) | P1 | ci | **done** | Only one of the three release targets was checked for static linking |
| [T-181](cli-surface.md) | P1 | cli | open | Four flags are accepted in silence and reach no code |
| [T-182](cli-surface.md) | P1 | ci | **done** | A macOS test asserted an invariant across two kernel subsystems |
| [T-120](licensing.md) | P1 | licensing | **done** | THIRD_PARTY.md is not generated |
| [T-121](licensing.md) | P1 | licensing | **done** | No cargo-deny configuration |
| [T-122](reference-map.md) | P2 | licensing | **done** | The copyleft and unlicensed reference trees are deleted |
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

141 items: 131 to work through, and 10 deferred to Phase C.

**Twenty were added on 2026-08-21 by reading the twenty-two tree corpus
against this tree**, T-163 to T-182. Nineteen of them are open, so the open
count went from 44 to 63 in a session that set out to write no code, which is
the correct direction: a gap nobody had written down was still a gap. The
twentieth, [T-182](cli-surface.md), is done, and it is the one piece of code
this session changed: a documentation-only push turned `Test (macos-latest)`
red on an assertion that two kernel subsystems agree with each other.

Seventeen of the nineteen come from `RESEARCH.md` section C and section D.
**Two did not, and they are the two worth separating**, because they needed no
corpus and were still not found: [T-171](metainfo.md), where `url_list`
accepts a bencoded string or a list and `http_seeds` four lines below it
accepts a list only; and [T-181](cli-surface.md), four flags that parse and are
never read again, found by grepping every `pub` field in `cli.rs` for a reader
outside that file. Both are one command to check and neither had been run.

A third is a hybrid and shows what the corpus is actually for.
[T-176](create-seed.md) exists because intermodal proposed a lint for torrents
with more than 65,535 pieces, since µTorrent refuses to open them. That is the
corpus. The finding is that `bit-cli` already has a piece-count lint and it
fires above **100,000**, so the band between the two numbers passes every check
`bit-cli` has and produces a torrent µTorrent cannot open. That is this tree.
Neither half is the defect on its own.

**Three entries described a state this tree is not in, and all three were
corrected rather than closed.** An entry keeps its original title, because the
title is how it has always been referred to and the history of a mistake is
worth seeing; the correction goes underneath, the way
[T-017](disk-io.md) and [T-021](peers.md) established. Where a title is now
known false the row above says so, so a reader skimming the table is not
misled by a heading nobody may rename. [T-033](performance.md) said three aria2 flags
"parse and do nothing"; they do not exist and exit 2.
[T-118](cli-surface.md) said neither `docs/flags.md` nor its CI check exists;
both do, and four tests enforce them. [T-161](cli-surface.md) named two CI call
sites for a deprecated action; there are four. The common cause is that each
was written from a specification rather than from the binary, and each took one
command to check. That is the argument for pass 1 of this session existing at
all.

Thirty-four earlier items were added by measurements rather than by the
triage. T-007 came out of T-001: a stalling
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
under `-j 1` and not above it. T-144 through T-148 came from reading CI rather
than the code, and between them they say what a red job costs. The MSRV job
had been failing because the tree needs rustc 1.88 and claims 1.85.1, which
also turned off two clippy lints nobody knew were off. The macOS test job
failed to link, and not on any of the three native dependencies its entry
guessed at: `posix_fallocate` was declared under `cfg(unix)` and does not
exist on Darwin. T-146 is the one that mattered most: a workflow-level
`RUSTFLAGS` replaced the per-target rustflags that make the Windows build
static, so CI built against the dynamic C runtime and its own check caught it.
T-147 is the one those three were hiding. Two tests failed on `ubuntu-latest`
and passed on Windows, because the path planner asked the host's own parser
whether a component would escape the output directory, and `C:` is a drive on
one host and a file name on the other. The disk paths agreed; the reason in
`--json` did not. T-148 is the flaky test sitting beside it, which asserted an
exit code inside its own retry loop.

Fixing those four uncovered three more, which is what a red job costs
compounded: every failure it hides is a failure nobody is looking for. T-149,
found once Windows was the only red job left, is a `bench leech` report that
had been dropping its final window of disk operations and piece verification,
so a run short enough to finish inside one `--metrics-interval` reported no
hashing at all. T-150 is that `Clippy` pins a floating `stable`, so three
lints appeared in CI that a current local toolchain does not raise: the gate
moves on its own. And T-151 is that only one of the three release targets was
ever checked for static linking, which matters because T-146 is the proof that
the check is what catches it.

T-152 came out of the round after that, once macOS linked and its process
reader worked: a `bench disk` phase that finished inside one metrics interval
emitted no sample at all, so the schema generator produced no `bench_sample` on
that runner and one everywhere else. It is T-149 in the other bench target. Two
targets, the same sampler mistake, and both were found by fixing whatever was
red above them.

T-154, T-155, and T-156 came out of closing T-113. A Metalink named by URL is
classified as a torrent URL and fails on the bencode parse, which is how a real
one is normally met because `MirrorBrain` generates them per request.
`--hash-check-only` returns before the metalink report is built, so a flag that
could report the size comparison reports nothing. And `download --dry-run`
writes `kind: "download"` with a different shape, which is why the schema
generator does not sample it and its fields are undocumented.

T-157 came out of the soak that answers T-040. `scripts/soak.ps1` rewrites its
summary after every sample so a killed run still leaves its slopes, and it did
that with a `Set-Content` straight onto the path. The `steady` run of
2026-08-21T01:24:28Z was killed mid-rewrite and left **4,833 NUL bytes** where
its slopes should have been. Its CSV survived with all 531 samples, because a
CSV is appended and a summary is rewritten. The whole value of the rewrite was
in the half that was not atomic.

**CI run 32444424026 was green on every job**, the first time the whole matrix
had been. It took four rounds and each one uncovered the next: the macOS link
failure hid six `sysinfo` failures, which hid T-152, which hid one per-platform
assertion that turned out to be the test rather than the code. A red job does
not cost one defect, it costs every defect behind it, and that is the argument
for never leaving one.

**Run 32479072800 is green on all sixteen jobs**, against commit `8abee2a`,
which closes [T-182](cli-surface.md). That tree carries the whole doc
reconciliation, the twenty-two tree corpus folded into these entries,
`TODO/RULES.md`, `TODO/PROGRESS.md` and `scripts/git-sync.ps1`. Its only
annotation is [T-161](cli-surface.md), a build action still targeting
Node.js 20.

**Run 32461172199 was green on all sixteen jobs**, against the commit that
closes [T-162](webseed.md). That tree carries `bench swarm`, the T-040 answer, the
deleted reference trees, and the fixes for both tests that turned a job red this
session. Its only annotation is [T-161](cli-surface.md), a build action still
targeting Node.js 20.

Naming a run rather than "the latest" is deliberate, and so is not claiming it
is the newest. A line that says which commit a run describes stays true; one
that says "the head" is wrong by the next push, and this one was, once. Check
the current run with `gh run list --limit 1`.

**A third documentation-only push turned a job red on 2026-08-21**, run
32478382564, `Test (macos-latest)`. That is [T-182](cli-surface.md), fixed: a
test asserted `peak_rss >= rss`, and on Darwin those two numbers come from two
kernel subsystems that share no accounting basis, so nothing ordered them.
Fifteen jobs were green, including the same test on the other two platforms.
Four now, counting [T-148](bench.md), which was found locally.

**Two earlier pushes turned a job red, and both were
documentation-only commits.** [T-160](cli-surface.md) on `ubuntu-latest` and
[T-162](webseed.md) on `macos-latest`. Neither was a defect in `bit-cli`: one
test dialled a seeder it had not waited for, and two asserted that a loaded
runner cannot fail a request any way but the one they were testing. A commit
that changes only Markdown is the cleanest proof available that a test is wrong
rather than the tree, and this session got that proof twice. The lesson is
narrower than "CI is flaky": a green matrix says nothing about the races a
suite still holds, because this one was green for sixteen jobs immediately
before each of them.

Between those two runs one job went red, and what turned it red is worth
keeping: run 32458314378 was a **documentation-only commit** and
`Test (ubuntu-latest)` failed on it. Nothing about the code had changed, which
is the cleanest available proof that the test was wrong rather than the tree.
That is [T-160](cli-surface.md), fixed, and it had already failed once locally
and been lost because the command reading the output matched only the summary
line and never the test name.

**Two things a measurement disproved on 2026-08-21, and both are the same
mistake.** A `bench swarm` run reported zero peers handshaked in every leech
case and read as a broken handshake. The handshake was fine: the acceptance
script ran the connect load first against a shared seeder, and the connect load
leaves the target unable to handshake at all. And the `steady` soak's resident
memory looked like a 0.73 MiB/h slope at an r squared of 0.27, which is noise.
Removing the three samples that are one thread burst gives 0.804 MiB/h at an r
squared of 0.73, which is a trend. In both cases the first reading was of the
fixture rather than of the thing. That is the same lesson T-032 and T-141 wrote
down, arrived at twice more.

| Priority | Open | Partial | Blocked | Done | Total |
| --- | --- | --- | --- | --- | --- |
| P0 | 1 | 2 | 0 | 8 | 11 |
| P1 | 5 | 1 | 0 | 43 | 49 |
| P2 | 36 | 1 | 1 | 12 | 50 |
| P3 | 21 | 0 | 0 | 0 | 21 |
| Phase C | | | | 10 deferred | 10 |
| **All** | **63** | **4** | **1** | **63** | **141** |

`blocked` is one item, [T-016](disk-io.md): a resume cache cannot be built on
`librqbit` 9.0.0 without turning on the session persistence that decision 7.4
puts in Phase C. It stays here rather than moving, with the upstream lines that
block it and what would unblock it.

## Start here

The work order, re-derived on 2026-08-21 with the twenty-two tree corpus in
hand. Four questions decide it, and they are asked in this order because a
later answer never outranks an earlier one.

### 1. Correctness in the one feature this project exists for

`bit-cli` exists to attach arbitrary HTTP sources to an existing `.torrent`.
A defect in that path outranks every feature below it, including the P0s,
because a P0 that takes the process down is visible and a wrong answer that
reports success is not.

Six items, and the first two are the whole argument.

1. **[T-171](metainfo.md)**. `url_list` accepts a bencoded string or a list;
   `http_seeds` four lines below it accepts a list only, so a torrent whose
   `httpseeds` is a bare string yields **zero** HTTP sources with no error and
   no warning. A web seed tool silently reading none of the seeds a torrent
   names is the worst failure available here. `torrent/metainfo.rs:306` is the
   accessor, `torrent/bencode.rs:339` and `:305` are why it returns empty, and
   `gosh-dl/src/torrent/metainfo.rs:391` is one parser serving both keys.
   Effort S.
2. **[T-005](webseed.md)**. One permanent status on one file retires the
   **whole** source. `README.md` says a mirror holding part of a payload is a
   first-class case and not an error, and this is the code contradicting it at
   exactly the moment the claim is tested. `torrent/webseed-peer.go:57`
   removes only that file's pieces instead. Raised from P3 to P2 for this
   reason. Effort M, and [T-167](bep-coverage.md) below makes it smaller.
3. **[T-181](cli-surface.md)**. Four flags parse and reach no code, P1 by this
   file's own definition. `--no-pex` is the one to fix first even though it is
   the one that cannot be built: a user passing it believes peer exchange is
   off and their address keeps being gossiped. Warn today, the way
   `cmd/seed.rs:105` already does for `--superseed`.
4. **[T-177](disk-io.md)** and **[T-174](metainfo.md)**. Two missing fixtures,
   both for arithmetic that is only ever exercised on the easy case. A piece
   straddling a file boundary, and a piece length that is not a multiple of
   16 KiB. fx-torrent 98 is what the first costs: in a multi-file album only
   the first file plays, every byte transferred. vortex PR 124 is the second:
   a double panic in a destructor. `vortex/bittorrent/src/file_store.rs` has
   eight test names that are the specification. Effort S each, and if the
   arithmetic is already right they cost one test apiece.
5. **[T-179](webseed.md)**, smart ban. Not a defect on its own, and it is what
   [T-164](peers.md) and [T-005](webseed.md) both need in order to name a
   culprit rather than guess one. With several sources filling one piece,
   `torrent/smartban/smartban.go` in 83 lines turns "a source is bad" from a
   guess into a fact.

### 2. The open P0 items, and why each is still P0

6. **[T-020](peers.md)**, the only open P0. Two defects; one fixed. What keeps
   it P0 is not the socket count. `bench swarm` found that while the pending
   handshake set is full the target **cannot complete a handshake for any info
   hash, including one it is serving**, and goes on reporting itself as
   seeding: 8,388,608 bytes, then 100 connected and 0 handshaked, then
   connected, 0 handshaked, 0 bytes. A stranded socket is a resource; a
   listener that accepts and never answers is an outage no health check sees.
   The soak adds that `CLOSE_WAIT` is zero at every one of 1,064 samples, so
   this needs the churn shape and does not appear under a deployment-shaped
   load. [T-164](peers.md) is adjacent and cheap: vortex 125 is a peer
   reconnecting every 20 seconds and burning a slot, which is the same
   resource seen from the other side.
7. **[T-040](memory.md)**, partial, and the open question is answered.
   0.804 MiB an hour, linear, r squared 0.73 over 525 samples, with the
   descriptors half disproved outright. What is left is attribution and not
   wall clock: completions run at a constant 228.5 an hour, so elapsed time
   and completed work are collinear and 0.804 MiB per hour fits exactly as
   well as 3.6 KiB per download. **Two shorter runs at different leech rates**,
   not a longer one. Both commands are in the entry.
8. **[T-090](bench.md)**, partial, with **[T-092](bench.md)** as the residue.
   All six subcommands are built. T-092 does not close on one acceptance
   clause and two unbuilt halves, all three named in the entry.

### 3. What the corpus has made cheap

Somebody else has already written and tested these. Effort here is reading and
adapting rather than designing.

9. **[T-167](bep-coverage.md)**, BEP 54 `lt_donthave`. The protocol is one
   BEP 10 extended message carrying a 4-byte piece index that clears one bit;
   `fx-torrent/src/peer/extension/donthave.rs` is 99 lines including its
   tests. It is the cheapest correctness win in the corpus, and it turns
   T-005's reconnect into a message. **Do this before T-005.**
10. **[T-064](trackers.md)**, BEP 15 backoff. Nine lines at
    `torrent/tracker/udp/timeout.go:9`, with a second, shorter ladder at
    `mtorrent/mtorrent-core/src/trackers/udp.rs:150`. The entry's decision to
    diverge stands; what it owes is the documented total budget, which both
    references state and this one does not.
11. **[T-004](webseed.md)**, BEP 17 auto-detection, now smaller than it looks.
    The style is determined by **which metainfo key the URL came from**, which
    is what BEP 17 and BEP 19 specify and needs no probe, and `bit-cli` already
    keys `httpseeds` sources correctly. Only the `--web-seed` command-line case
    needs the probe.
12. **[T-176](create-seed.md)**, three lints. Two are threshold changes against
    numbers the corpus supplies, and one is splitting a message that is
    currently false.
13. **[T-100](bep-coverage.md)**, BEP 6. The algorithm at
    `vortex/.../peer_connection.rs:89`, the receive-side bug that makes it
    silently inert at `torrent/peerconn.go:1047`, a canonical test vector, and
    a documented divergence in aria2 so a mismatch is not debugged twice.
14. **[T-081](create-seed.md)**, BEP 52. `nanotorrent`'s 618-line v2 and
    hybrid creator is built **on librqbit**, the same base, for the same
    reason. Three independent implementations to check the spec against, real
    v1/v2/hybrid fixtures in two trees, and one construction in the corpus that
    is wrong in a way that passes its own tests.
15. **[T-018](disk-io.md)** and **[T-083](create-seed.md)**. Coalescing with
    tests at `TorrentNG/crates/rt-storage/src/elevator.rs:223`, and the full
    choke algorithm at `vortex/bittorrent/src/torrent.rs:488`, from which the
    report shape T-083 wants simply follows.

### 4. Which BEP gaps cost interoperability today, and which are completeness

The distinction the coverage table could not make until now.

**They cost reach today.**

16. **[T-163](peers.md)**, MSE/PE. The largest single loss of reachable swarm
    in the list: a peer configured to *require* encryption will not exchange
    traffic with a plaintext-only client at all. Blocked on the same librqbit
    seam as [T-002](webseed.md) and [T-102](bep-coverage.md), and
    `nanotorrent`'s patches 0003 and 0005 are the shape of the upstream change.
    High value, not startable.
17. **[T-166](peers.md)**, BEP 10 id direction. P1 and effort S. If it is wrong
    the failure is total and silent against qBittorrent, and `bit-cli`'s bridge
    is tested only against itself, which is the arrangement that hides it. A
    test either proves it or finds it. **Cheapest high-consequence item in the
    file.**
18. **[T-103](bep-coverage.md)**, the `.utf-8` key variants. uTorrent writes
    `name` and `name.utf-8` with different encodings, and preferring the
    `.utf-8` spelling is a read-side rule, not the Shift-JIS work the entry
    leads with.

**They are completeness.**

19. [T-101](bep-coverage.md) uTP, [T-102](bep-coverage.md) BEP 55,
    [T-168](bep-coverage.md) WebTorrent, [T-169](dht.md) BEP 33 and 51,
    [T-170](dht.md) BEP 44, [T-082](create-seed.md) BEP 16. None of these
    stops `bit-cli` talking to a peer it can otherwise reach. uTP is
    politeness, BEP 55 raises the reachable set rather than enabling it, and
    BEP 52's own advocates say it is not widely used: mkbrr
    [Issue 112](https://github.com/autobrr/mkbrr/issues/112) is a v2 request
    whose author writes that it is not really used by many people.
20. [multi-source.md](multi-source.md) is the operator's five scenarios and
    four of the five work in full. Read that file before starting any of T-130
    to T-143; the work left is smaller than the entry count suggests.

### What this ordering changed

The previous order put the long-run failures at item seven and Metalink at
item nine, and it was written when the only correctness question in the web
seed path was performance. Three things moved it.

**The headline feature had a silent parse bug in it.** T-171 was above every
P0 the moment it was found, and it was found by reading two adjacent functions.

**A P0 is not automatically first.** T-020 is real and it is a churn-shaped
failure that a deployment-shaped load does not produce, which the soak
established over 1,064 samples. T-171 is a normal-path failure that produces a
wrong answer and reports success. The second outranks the first.

**Several items got cheaper without changing.** T-167, T-064, T-004 and T-176
are the same work they were, against reference code that is now readable, and
that moves them up the list without any argument about their value.

Nothing moved down for being hard. [T-163](peers.md) is blocked and stays high,
with the blocker named, because the rule in this file is that a blocked item
stays open rather than sinking.

## What is settled, and what each closing measured

The record below is what has already been closed and what the closing
measured. It is not the work order; it is the evidence the work order rests
on. Items one to six are done or mostly done, item seven is the long-run
cluster, item eight is the operator's own list, and item nine is Metalink.

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
   or the status, the range support, and the TLS parameters. **Every one of the
   six subcommands is now built**, which is what `every_bench_subcommand_is_built`
   asserts against `clap`.

   [T-092](bench.md) is **partial** and is the last of T-090. `bench swarm` has
   both its loads. Without `--for` it generates info hashes the target does not
   have and measures the accept path; with `--for` its synthetic peers leech a
   torrent the target serves, check every piece against the torrent's own
   hashes, and hold what they verified once between them. Measured against
   `bit-cli seed`: **333.33 MiB/s at one peer, 666.67 at four, 941.18 at
   sixteen**, so the target's aggregate stops scaling between four and sixteen
   rather than falling over. `pwsh scripts/check-swarm.ps1` drives nine cases.

   It does not close, on one acceptance clause and two unbuilt halves.
   `--disk-budget` bounds the bytes written and not the bytes on disk, because
   a held piece is written at its own offset: a 2 MiB budget leaves a 4.75 MiB
   file. A synthetic peer keeps its pieces and does not serve them, so the load
   is a hundred leeches rather than a swarm. And the case that proves no peer
   but the target is ever dialled is not written yet.
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

   [T-032](performance.md) and [T-141](webseed.md) are **done**, and both
   closed by disproving their own premise. `librqbit` 9.0.0's picker is not
   rarest-first and never was: it yields the first piece of each file, then the
   last, then the middle ascending, so `--piece-selector rarest-first` was
   naming behaviour that does not exist. It is `default` now, `random` is gone
   because nothing can ask for it, and `sequential` holds a `FileStream` at the
   earliest missing piece: **zero descents in ten runs at one connection,
   against one in every run of the default**.
   `scripts/check-piece-order.ps1` is the measurement.
   And `--web-seed-connect-timeout` was never broken. The fixture was: port 9
   on Windows is the **discard** service, which accepts and never answers, so
   the connect succeeded and the request timeout was correctly the bound.
   Against an address that really drops the SYN the flag is the only bound in
   play. `scripts/check-connect-timeout.ps1` drives both directions.
7. The long-run failures. [T-017](disk-io.md), [T-030](performance.md),
   [T-021](peers.md), [T-031](performance.md), [T-037](performance.md), and
   [T-138](peers.md) are **done**. [T-020](peers.md) is the one open P0 left,
   and [T-040](memory.md) is partial: its question is answered and what is left
   is attribution, not wall clock. Measure before theorising: every one of
   these closed that way, and every time the answer was not what the entry
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

   What the soak adds: `CLOSE_WAIT` is zero at **every one of 1,064 samples**
   across a 4.605 hour `steady` run and a 4.617 hour `idle` one, so this needs
   the churn shape and does not appear under a deployment-shaped load.

   What `bench swarm` adds is worse than a socket count. While the pending set
   is full the target **cannot complete a handshake for any info hash,
   including one it is serving**, and it goes on reporting itself as seeding.
   Leech one peer, run the connect load, leech again against the same seeder:
   8,388,608 bytes, then 100 connected and 0 handshaked, then **connected, 0
   handshaked, 0 bytes**. A stranded socket is a resource. A listener that
   accepts and never answers is an outage no health check sees.

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

   [T-040](memory.md) is **partial**, and the open question it carried is
   answered. `scripts/soak.ps1` samples a long-lived seeder under one of six
   workloads and writes the series to `bench/soak-<timestamp>.csv`. The pair
   started on 2026-08-21 ran 4.6 of their 6 hours and hold 1,064 samples
   between them, which turned out to be more than the question needed.

   **The descriptors half is disproved.** The `idle` control holds exactly
   **189 handles at every one of 533 samples**, one TCP socket, and 21 threads,
   with resident memory flat within 0.03 MiB over its last 2.5 hours. Nothing
   moves when nothing is asked of it, so the sampler and the session timers are
   ruled out and every number in the `steady` run is the load.

   **The memory half reproduces and is linear.** 0.804 MiB an hour at an r
   squared of 0.73 over 525 samples, and the last three hours alone give 0.744
   at 0.52, so the slope does not decay. Linear beats logarithmic, square root,
   and every saturating exponential; the best saturating fit needs a time
   constant of eight hours, which over a four and a half hour window is a
   straight line. So: not a settling curve, and not an allocator holding pages.

   **Six hours would not have added the answer.** The discrimination is the
   whole-run slope against the last-three-hours slope, and those agreed at
   three hours. What is missing is not a longer run but two shorter ones at
   different leech rates: completions run at a constant 228.5 an hour, so
   elapsed time and completed work are collinear and 0.804 MiB per hour fits
   these points exactly as well as 3.6 KiB per download. Both commands, and the
   three ceilings the slopes now justify, are in the entry.
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
9. [T-113](cli-surface.md) is **done**. `bit-cli download release.meta4` reads
   a `.meta4` or a `.metalink`, fetches the `.torrent` its `<metaurl>` names,
   registers every mirror as a source, downloads, and checks the payload
   against the document's own checksum.

   **A Metalink and a `.torrent` are two independent descriptions of one
   payload, and the run says which of them is wrong.** The declared lengths are
   compared before a byte moves. The digest is then checked against a payload
   the session has already verified piece by piece against the torrent's own
   hashes, so a digest that disagrees is evidence about the Metalink. Either
   disagreement exits 7 and the two stay apart in `--json`.

   The real-document run found the thing worth knowing: **no `MirrorBrain`
   instance reachable in August 2026 emits `<metaurl mediatype="torrent">`**,
   so the document a user actually gets has 58 mirrors, three checksums, and
   nothing to start a download from. `bit-cli` names the mirror count and says
   so. `pwsh scripts/check-metalink.ps1` drives ten cases on loopback and
   `pwsh scripts/check-metalink-real.ps1` drives four against
   `download.documentfoundation.org`.
