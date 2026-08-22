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

- **Last session:** 2026-08-22T07:17:07Z to 10:56Z, unattended throughout and
  ended on the operator's word. The whole four-item work order, then the two
  reviews, then the storage-shaped acceptance scripts run against the one
  change that touches the write path.
- **Tests:** 1,113 passing, 0 failing. The baseline at the start was 1,091.
- **Gates:** clean. One command:

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** green on every push this session, last read at run
  **32566533746** against commit `4ee2948`. Three commits carry `[skip ci]`
  because they change no source: `ec82a24`, `f46d4fd` and `47a6cea`.
- **Entries:** 149 items. 47 open, 7 partial, 2 blocked, 83 done, 10 deferred
  to Phase C. 83 of 139 workable done, 56 left.
- **Tree:** 84 Rust files, 49,649 lines of code, 11,471 of comment, measured
  with `scc --no-cocomo crates/`.

## What the last session did

The four-item work order, in order, and then the reviews.

### The work order

- **[T-092](bench.md)** and **[T-090](bench.md)**, both **done**, and T-090 is
  a P0. A synthetic peer **serves** now: a bitfield, an unchoke, a `have` per
  piece it verified and kept, and requests answered out of the packed hold
  file, with a BEP 6 `reject` where the fast extension was negotiated. Packing
  had to become reversible for it, and a piece the budget refused is not
  announced. **A synthetic peer can never put a byte into the target**: its
  only source is the target, so everything it can announce is something the
  target already has. Measured: 32, 128 and 512 pieces announced across the
  three leech cases and the target asked for **none** of them. The target
  model's other counterparty, serving the other synthetic peers, contradicts
  the clause `sources_ignored` now proves from the operating system's socket
  table. Ten cases, `bench/swarm-20260822T074823843Z.json`, verdict pass.
- **[T-022](peers.md)**, **partial**, and half the decision it asks for was
  already made upstream. `bit-cli trackers --family` announces once per family
  a tracker resolves to and reports each separately. **`librqbit` already
  announces both families to UDP trackers**, at
  `librqbit-tracker-comms-9.0.0/src/tracker_comms.rs:374-387`; for HTTP it
  announces once at `:293`, and that is the blocked half.
  `ClientBuilder::local_address` does **not** pin a family, which is the
  obvious thing to reach for and is wrong; overriding the resolution is what
  works.
- **[T-007](webseed.md)**, **done**. The "constant near sixteen seconds that no
  flag moves" is the **bridge reconnect backoff**, `bridge.rs:577` and
  `:701-703`, and `--web-seed-max-errors` moves it: it looked constant because
  every row of the table held that flag at its default of 5, where 1+2+4+8 is
  15. A stall is its own failure class now, told from a short body by
  `is_timeout` rather than by anything in the text. **24,247 ms to 6,108 ms**
  in the acceptance's own venue, and 133.28 s to 10.11 s in the harsher
  reproduction.
- **[T-018](disk-io.md)**, **partial**: built, shipped, measured, one
  acceptance clause not met. `Coalescer` combines a run of block writes into
  one operation. **405.71 to 508.44 MiB/s** at eight bridges, wall 1,262 to
  1,007 ms, writing the payload 5,101 ms to 1,806 ms.

### What the work found

- **A defect in `check-swarm.ps1` that had been passing.** A local called
  `$peers` is the script's own `$Peers` parameter, because PowerShell variable
  names are case-insensitive, so every case after it built its argument list
  from 0. `listener_poisoned`'s connect load exited on `--peers cannot be
  zero`, opened no socket, poisoned nothing, and **the case recorded three
  nulls and passed**. [RULES.md](RULES.md) section 5 names that exact trap. The
  case is what let it through, so it records all three exit codes now and fails
  when any run wrote no report.
- **The style probe costs a whole `--web-seed-timeout`** before `source_added`
  is emitted, bounded by `probe.rs:956` at five seconds. It is also the
  difference between T-007's two tables: the earlier numbers were measured from
  after the source was added.
- **`bench disk`'s `shared` layout strides its blocks across threads**, so
  nothing is ever contiguous and the write buffer coalesces nothing there. That
  is why T-018's first acceptance clause cannot be met by that fixture, and it
  is the instrument's shape rather than the download's: `split`, whose threads
  write contiguous ranges like a receive path does, reaches **1.56 times** the
  raw unbuffered path at eight threads.

### What the two reviews found

Three claims that had stopped being true, and two of them were made stale by
this session's own work hours earlier.

- **T-001's failure matrix** still said the stall case ended after 24,247 ms
  and explained it as the retry and cooldown machinery. T-007 disproved that
  reading and the same case ends in 6,108 ms.
- **T-188** described the straddling-block fan-out as an assertion on
  `write_ops`, which T-018 moved to `write_calls` when the two counters stopped
  being one number.
- **T-178's Blocker** said its no-progress guard should wait for somewhere that
  batches writes to exist. It exists, and the entry names the function.

And two gaps worth entries of their own.

**[T-189](bench.md)**: `docs/schema.md` is the versioned contract and the
**`bench` reports are not in it at all**. T-018 added `write_calls` to
`bench::report::Disk`, a field every `leech` and `seed` report carries, and
regenerating the schema produced **no diff** with the test green. The same
session added seven fields to the `trackers` document and the check caught
every one, so the mechanism works where it reaches. The exclusion turns out to
be deliberate and written down, and the entry argues with the reason rather
than assuming an oversight.

**[T-190](disk-io.md)**, found running `check-allocation.ps1` against the new
write buffer. The script reported **the payload does not match the source** on
all four allocation methods, which read as data corruption from a change made
an hour earlier. It is not: every download was byte for byte correct, and the
script was looking one directory too high. It fails the same way on a worktree
at `f46d4fd`, before the buffer landed. What it exposed is real, though:
`engine.rs:575-577` says a caller who named an output directory "gets exactly
that directory" and a multi-file torrent with `--dir` lands under
`<dir>/<name>/` anyway, which is what every end-to-end test asserts. The
paths are fixed and the script measures again; `sparse` reserving four
kilobytes of volume for a 32 MiB file against `prealloc` reserving all of it
is the distinction it exists to draw and it had been invisible.

## In progress

Nothing is half-written. Every entry touched is complete, or explicitly partial
with its blocker named.

Four things are carried rather than finished:

- **[T-020](peers.md)** stays the only open P0. Both mitigations are backstops,
  nothing drains the queue for a peer that is not us, and
  `check-close-wait.ps1 -Ceiling 100` still fails.
- **[T-040](memory.md)** stays partial: attributed and bounded, not fixed.
- **[T-018](disk-io.md)** is partial on one acceptance clause, and closing it is
  a decision about what `bench disk` is for rather than a change to the write
  path.
- **[T-022](peers.md)**, **[T-100](bep-coverage.md)** and
  **[T-132](multi-source.md)** are partial with the same shape of blocker as
  [T-102](bep-coverage.md) and [T-167](bep-coverage.md): a seam `librqbit`
  9.0.0 does not expose, named with a line number in each.

## Start here next session

Re-derived after the last work order closed. [INDEX.md](INDEX.md)'s "How an
ordering is derived" was read for the four questions it asks; its own list is
the derivation of 2026-08-21 and is kept as written.

1. **Read the CI run the last push started.** Everything before it was green,
   last read at 32566533746.
2. **[T-190](disk-io.md)**, P2, effort S. One comment and one behaviour
   disagree about where somebody else's bytes are written, and the evidence
   that the behaviour is right is already in the entry. Read what
   `subfolder: false` achieves before changing the comment: the extra
   directory may be the session's rather than the factory's, and then the
   comment should say so.
   Corpus: none needed; `engine.rs:575-577` and `storage.rs:402` are the
   whole surface.
3. **[T-189](bench.md)**, P2, effort S, and it is the only item here that a
   measurement has already justified rather than estimated. The `bench` reports
   are outside `docs/schema.md`, and a field went into one this session with
   the contract check green. The exclusion is deliberate and the entry names
   both ways out; pick one.
   Corpus: none needed; `crates/bit-cli/src/schema_gen.rs`'s `collect` is the
   whole surface.
4. **[T-018](disk-io.md)**, P2, partial, and what is left is **not** the write
   path. Decide whether `bench disk`'s `shared` layout should stop striding, so
   the instrument can show what the download path does, or whether the
   acceptance clause should move to `--layout split`. Either is a decision
   about [T-017](disk-io.md)'s question. The measurements are all in the entry.
   Corpus: `TorrentNG/crates/rt-storage/src/io_class.rs:7` for per-class
   concurrency caps, which is the piece this tree still does not have.
5. **[T-024](peers.md)**, P2, the choke and unchoke history a peer row does not
   carry. It is the last of A3.4b that is missing and `bench swarm` can now
   generate the events to test it against, which it could not before this
   session.
   Corpus: `vortex/bittorrent/src/peer_comm/peer_connection.rs` for a peer that
   tracks both directions.

Do not start [T-163](peers.md) MSE, [T-102](bep-coverage.md) BEP 55,
[T-167](bep-coverage.md) BEP 54, [T-016](disk-io.md) fastresume, the send half
of [T-100](bep-coverage.md), the peer-only cap in [T-132](multi-source.md), or
the HTTP half of [T-022](peers.md). All seven are blocked on `librqbit` seams
and all seven name the blocker with line numbers.

## Open questions for the operator

None. Two decisions were taken unattended and are recorded where they belong:
`bit-cli trackers` announces once per family, under [T-022](peers.md), and a
stall retires a source on the first request that runs out of time, under
[T-007](webseed.md) with the trade-off stated.
