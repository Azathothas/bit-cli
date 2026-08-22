# Progress

**Read this first.** It is the only thing the kickoff prompt tells a session to
read, so everything that changes from session to session is here: the baseline,
what the last session did, and the work order. The prompt carries none of it, by
[RULES.md](RULES.md) section 3.

It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
Every entry, one line each: [INDEX.md](INDEX.md).

> **The shape this file must keep**, from [RULES.md](RULES.md) section 2 step 2:
> the state line with the session's start instant in ISO 8601 UTC, the measured
> baseline with the CI run named by id, the entry counts, what the session did,
> what is in progress, **Start here next session** as an ordered list with entry
> ids and corpus sources, and open questions for the operator.
> `scripts/session-report.ps1` prints the numbers; do not count them by hand.

---

## State

- **Last session:** 2026-08-22, started 01:11Z, unattended. Feature work plus
  a round of tooling the operator asked for mid-session.
- **Tests:** 1,052 passing, 0 failing. The baseline at the start was 1,028.
- **Gates:** clean. One command now:

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** green at run **32548057725**, against commit `14dd46d`. The three
  before it were green too: **32543990448**, **32545039478**, **32546921561**.
  Naming a run and the commit it describes is deliberate: that line stays true,
  where "the latest" is wrong by the next push.
- **Entries:** 147 items. 51 open, 5 partial, 2 blocked, 79 done, 10 deferred
  to Phase C. 79 of 137 workable done, 58 left.
- **Tree:** 82 Rust files, 47,270 lines of code, 10,354 of comment, measured
  with `scc --no-cocomo crates/`.

## What the last session did

The whole four-item work order, and then the tooling.

- **[T-185](cli-surface.md)**, done. `--exclude-file` without `--select-file`
  resolved to `None`, every file, so the flag skipped nothing and the run
  fetched what it had been told to skip. The count is per source, not per run:
  `plan_selection` settles it from metadata `run` already parsed. A magnet
  defers, and only for the two spellings that need a count. **The magnet answer
  is not the one the entry recommended**: narrowing a live torrent is too late,
  because `librqbit` initialises by creating every file it was not told to skip.
- **[T-143](multi-source.md)**, done, and **the entry's premise was too kind**.
  Above `-j 1` the takers do not fetch the shared file twice, they have no
  source at all and never finish. Measured before building, both runs recorded.
- **[T-164](peers.md)**, partial, and the seam is named. It splits into three
  parts and only two are blocked: `librqbit` already has a peer blocklist,
  checked before an incoming handshake and before an outgoing dial, and it
  loads from a `file:` URL, so `--block-peer` needed no upstream change.
- **[T-186](cli-surface.md)**, done, and **the entry did not know the wrong
  spelling also writes**: `seed` hash-checks on add, so pointing at the torrent
  directory left an empty tree one level inside it.
- **[T-188](disk-io.md)**, filed and closed. Found by measuring T-185. An
  unselected file landed at zero bytes because `librqbit` issues a zero length
  write to the file before a chunk that starts on a boundary. It corrects
  [T-013](disk-io.md)'s closing claim, and the correction stays under T-013.

**Two defects found while running gates, not by reading.**
[T-179](webseed.md)'s acceptance test depended on a race and failed twice under
whole-suite load; it is arranged now rather than hoped for, and
[RULES.md](RULES.md) section 5 carries the shape as its own line. And
`check-shared-files.ps1` waited exactly as long as the run's own `--stop-after`,
so a run that stopped on its own deadline was killed at the same instant and
read as one that wrote no report.

**The tooling round**, all of it from the operator's list:

- `git-sync` no longer force-pushes 52 MB of corpus on a push that did not
  change it; it compares the tree hash against `origin/references` first.
- `-BodyFile`, because a commit body typed into a shell has to survive that
  shell's quoting and twice it did not.
- `-NoCi`, which puts `[skip ci]` on a commit and is refused unless every
  staged path is documentation.
- `scripts/gates.ps1`: fmt, clippy, test, deny, optionally build, one verdict,
  stray processes killed first.
- `scripts/check-todo.ps1`: the mechanical half of the two deep reviews. It
  found four stale citations on its first run, and it would have found the two
  things this session fixed by hand.
- `scripts/session-report.ps1`: elapsed, commits, lines, `scc`, entries done
  out of workable, and what closed.
- [RULES.md](RULES.md) section 3 rewritten: the kickoff prompt is generic now
  and this file carries the work order.

## In progress

Nothing is half-written. Every entry touched is complete, or explicitly open or
partial with its blocker named.

Three things are carried rather than finished:

- **[T-020](peers.md)** stays the only open P0. Half of it is fixed; the
  stranded-socket half and the poisoned-listener finding are open.
- **[T-040](memory.md)** and **[T-090](bench.md)** stay partial. Neither needs
  a longer run; both need the specific measurements named in their entries.
- **[T-164](peers.md)** is partial, with two of its three parts blocked on
  `librqbit` seams that the entry now names with line numbers.

## Start here next session

The work order. Re-derive it if the reasoning below no longer holds, and say so
in this file if you do.

1. **[T-020](peers.md)**, the only open P0, and it has been outranked for three
   sessions by correctness work that is now done. `bench swarm` found that
   while the pending handshake set is full the target **cannot complete a
   handshake for any info hash, including one it is serving**, and goes on
   reporting itself as seeding. A stranded socket is a resource; a listener
   that accepts and never answers is an outage no health check sees. The
   `PENDING_HANDSHAKE_CHECKS` change in `crates/bit-cli-core/src/engine.rs`
   removed the panic; the entry's second finding is open.
   Corpus: `aria2_rust/aria2-core/src/engine/bt_peer_storage/`.
2. **[T-040](memory.md)**, partial, and the open question is answered:
   0.804 MiB an hour, linear, r squared 0.73 over 525 samples. What is left is
   attribution and not wall clock, because completions run at a constant 228.5
   an hour and the two are collinear. **Two shorter runs at different leech
   rates**, not a longer one. Both commands are in the entry.
3. **[T-064](trackers.md)**, BEP 15 backoff, effort S and the corpus has
   written it twice: nine lines at `torrent/tracker/udp/timeout.go:9`, and a
   shorter ladder at `mtorrent/mtorrent-core/src/trackers/udp.rs:150`. The
   entry's decision to diverge stands; what it owes is the documented total
   budget, which both references state and this one does not.
4. **[T-100](bep-coverage.md)**, BEP 6 Fast Extension. The algorithm at
   `vortex/bittorrent/src/peer_comm/peer_connection.rs:89`, the receive-side
   bug that makes it silently inert at `torrent/peerconn.go:1047`, a canonical
   test vector, and a documented divergence in aria2 so a mismatch is not
   debugged twice.

Do not start [T-163](peers.md) MSE, [T-102](bep-coverage.md) BEP 55,
[T-167](bep-coverage.md) BEP 54, or [T-016](disk-io.md) fastresume. All four are
blocked on `librqbit` seams and all four name the blocker with line numbers.

## Open questions for the operator

None. Everything the operator asked for mid-session on 2026-08-22 is built and
recorded above.
