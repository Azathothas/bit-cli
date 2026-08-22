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

- **Last session:** 2026-08-22T04:05:27Z to 07:10Z, 3h 4m, unattended
  throughout. The
  whole four-item work order, then three things the work found, two more
  entries, and the two reviews.
- **Tests:** 1,091 passing, 0 failing. The baseline at the start was 1,052.
- **Gates:** clean. One command:

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** green on all sixteen jobs at run **32557742549**, against commit
  `6c47829`. Two runs earlier in the session were **red on Clippy and nothing
  else**, **32555391850** (`519742a`) and **32555846984** (`88676cb`), for the
  reason under "the toolchain" below; **32556256557** against `081d943` is
  where that was fixed and every other job in both red runs was green. Two
  commits carry `[skip ci]` because they change no source, `2f202d2` and
  `c77e462`, so they have no run of their own.
- **Entries:** 147 items. 48 open, 7 partial, 2 blocked, 80 done, 10 deferred
  to Phase C. 80 of 137 workable done, 57 left.
- **Tree:** 84 Rust files, 48,351 lines of code, 10,816 of comment, measured
  with `scc --no-cocomo crates/`.

## What the last session did

The four-item work order, in order, and then three things that came out of it.

### The work order

- **[T-020](peers.md)**, the second finding, mitigated and the mechanism
  measured. **The entry named the wrong cause**: it said the pending
  handshake-check set was full, and the set is never full because
  `PENDING_HANDSHAKE_CHECKS` is `usize::MAX`. The cause is the **drain rate**,
  one entry per accepted connection, because `task_listener`'s second
  `select!` arm matches only `Some(Ok(..))` and an `Err` disables it until the
  next accept. Measured one for one: twenty poisoning connections, and the
  twentieth probe after them was the first to be served. What is carried is
  `--listener-check <DUR>` on `seed`: it dials this run's own port over
  loopback, completes a real handshake, and three failures in a row stop the
  run with `"stopped": "listener_unhealthy"` and **exit 17**.
- **[T-040](memory.md)**, attributed and bounded, still partial. **The
  attribution is peer rows**, not wall clock: `librqbit` keeps a row for every
  peer it has ever accepted and never reclaims one, at **2,891 bytes a row**
  measured over 2,000 of them at r squared 0.94. At the soak's 228.5
  completions an hour that is 0.63 MiB/h against the 0.804 measured, so 78
  percent of the slope. **The entry's own plan was superseded**: two runs at
  different leech rates would have moved the peer count and the transferred
  bytes together. What is carried is `--max-rss <SIZE>`, the same shape of
  backstop as `--max-handles`, exit 16.
- **[T-064](trackers.md)**, **done**. The divergence from BEP 15 stands and the
  total it owed is **five attempts**, not three and not six: a UDP announce is
  two exchanges and a connect that is not answered by its third attempt gives
  up, so the announce that would spend three more is never sent. The budget for
  one UDP tracker is `5 * max(--tracker-timeout / 3, 1s)`, fifty seconds at the
  default and never under five, per tracker rather than per torrent.
- **[T-100](bep-coverage.md)**, partial, and **the Approach names the wrong
  half as the reachable one**. It says to start with the bridge; the bridge is
  the half that cannot be done, because its only counterparty is the session in
  the same process and `librqbit` 9.0.0 has no BEP 6 at all. Two of three parts
  landed: `crates/bit-cli-core/src/fast_set.rs` **reproduces the conformance
  vector exactly**, with aria2's divergent mask implemented rather than
  described, and `bench swarm` now advertises the bit and reports what a target
  did with it. `check-swarm.ps1` carries the answer from the wire:
  `fast_negotiated` 0 on every leech case, recorded and not judged.

### What the work found

- **[T-092](bench.md)**'s disk budget clause, **fixed**, and
  `check-swarm.ps1` passes for the first time. `--disk-budget` bounded the
  bytes written and not the bytes on disk; held pieces are packed now rather
  than written at their torrent offset. The two committed swarm records before
  this one both say `verdict: fail`, so for two sessions that script's exit
  code could not tell a new failure from the known one.
- **A defect in `bench swarm` itself.** Every frame went to
  `Message::deserialize`, which knows none of the five BEP 6 ids, so a target
  that spoke the fast extension was reported as `ended: "protocol"`, a broken
  peer. Nothing had noticed because the only target ever pointed at was
  `librqbit`, which never sends one.
- **The toolchain, and it cost two red CI runs.** CI installs `stable` on every
  run and this machine did not: rustc here was 1.97.1 and CI installed 1.98.0,
  released four days earlier, carrying `clippy::chunks_exact_to_as_chunks`. Every
  gate was green here and one job was red there. The local toolchain is
  updated, clippy 1.98 is clean across the workspace, and `gates.ps1` now
  prints the toolchain and warns when the `stable` it is using is behind.

### What the two reviews found

Five things, and two were older than this session.

- **A citation that pointed at a record which did not carry the number.**
  T-100 cited a swarm run for `peers_negotiated`, which lives in the per-run
  `bench swarm` report that `check-swarm.ps1` deletes. `check-swarm.ps1`
  records `fast_negotiated` and `received` on every leech case now, and the
  entry cites the run that has them.
- **A number that two documents disagreed on.** `README.md` said thirteen
  connections cleared the listener backlog and the committed acceptance says
  twenty. Thirteen was a scratchpad run whose load had already drained eight of
  its own connections.
- **A lint named wrong in three places.** It is
  `clippy::chunks_exact_to_as_chunks`, checked by making clippy emit it.
- **A NUL byte in `TODO/trackers.md`**, from an escape interpreted on its way
  to the file while quoting a tracker's NUL-terminated error message.
- **Three more in `crates/bit-cli-core/src/torrent/bencode.rs`, since
  2026-08-21.** `TOLERATED_TRAILING` spelled its five bytes out literally
  instead of as escapes, so for two sessions no search over `crates/` could see
  a line of the largest metainfo file in the tree: `grep` calls such a file
  binary and skips it. Recorded under [T-172](metainfo.md).

One more, found while printing the session's own numbers: **`session-report.ps1`
reported an hour too many.** `[int]` on a double rounds in PowerShell, so
`[int](2.65)` is 3 and a 2h 39m session printed "3h 39m", the hour coming from
the minutes and then printed again beside them. Every session past the half
hour was wrong and the number goes into this file's state line. `[math]::Floor`
now, and the same cast was doing the same to `soak.ps1`'s minute count.

Two guards, because finding these by reading is how they were missed.
`gates.ps1` gains a **`text` gate** over every tracked `.rs`, `.md`, `.ps1`,
`.toml`, `.yml` and `.jq`, which fails and names the file and the offset, and
`check-todo.ps1` checks the same over `TODO/` before it reads anything as text.
Both were checked by injecting a NUL.

### Two more entries, taken after the work order

- **[T-007](webseed.md)**, measured and not built. A stalling source costs
  **133.28 s** at `--web-seed-timeout 5s`, not the 24,247 ms the entry records,
  and the cooldown its Problem blames is never waited on. What multiplies is
  the error budget over the retry ladder, plus **a constant near sixteen
  seconds that no flag moves**. The entry now carries the table, the model and
  both targets, and nothing was written on the strength of a mechanism the
  code does not have.
- **[T-132](multi-source.md)**, partial. The premise holds and **the
  workaround the entry proposes does not survive the measurement**. The session
  cap does bound the bridge, 195.42 MiB/s to 8.41 under an 8 MiB/s cap. The
  Approach's derivation would bound the peer share only while HTTP takes its
  whole bucket, and the hybrid run is what happens when it does not: HTTP at
  1.40 MiB/s against an 8 MiB/s cap because the peer was faster, and the run at
  35.96 MiB/s. The documentation half is done and the peer-only cap is blocked
  on `torrent_state/live/mod.rs:1698-1706`.

## In progress

Nothing is half-written. Every entry touched is complete, or explicitly
partial with its blocker named.

Four things are carried rather than finished:

- **[T-020](peers.md)** stays the only open P0. Both mitigations are backstops.
  Nothing here drains the queue for a peer that is not us, and
  `check-close-wait.ps1 -Ceiling 100` still fails.
- **[T-040](memory.md)** stays partial: attributed and bounded, not fixed.
  Closing it means `librqbit` reclaiming a peer row.
- **[T-100](bep-coverage.md)** and **[T-132](multi-source.md)** are partial with
  the same shape of blocker as [T-102](bep-coverage.md) and
  [T-167](bep-coverage.md): a seam `librqbit` 9.0.0 does not expose, named with
  a line number in each.

## Start here next session

The work order. Re-derived after the last one closed. Every unblocked P0 and P1
is either done or has its blocker named, so this is ordered by what a
measurement can still move.

1. **[T-092](bench.md)**, the last clause of [T-090](bench.md) and the only
   remaining P1 that nothing upstream blocks. **A synthetic peer does not
   serve**: it keeps its verified pieces, announces nothing, and answers no
   request, so `bench swarm` is a hundred leeches rather than a swarm and a
   target that ranks peers by what they uploaded sees nothing. The holding side
   is built and packed now, so what is left is the serving side.
   Corpus: `vortex/bittorrent/src/peer_comm/peer_connection.rs` for a peer that
   both leeches and serves on one connection.
2. **[T-022](peers.md)**, P1, the last open P1 after that. A session bound to
   `[::]` announces one address, so one family's peers may never learn a
   reachable one. `bit-cli`'s own tracker client announces one port and lets
   the tracker take the source address; announcing both families needs two
   announces. The decision the entry asks for is whether `trackers` should do
   that and whether the session should.
   Corpus: `torrent/tracker/` for the two-announce shape.
3. **[T-007](webseed.md)**, P2 and **effort S**, and it is **measured and
   ready to build**: this session ran the reproduction rather than the entry's
   arithmetic. A stalling source costs **133.28 s** at `--web-seed-timeout 5s`,
   not the 24,247 ms the entry records, and the cooldown the Problem blames is
   not waited on at all. What multiplies is the error budget over the retry
   ladder, `max_errors * ((retries + 1) * timeout + backoff)`, and **a constant
   near sixteen seconds that no flag moves**. Two targets, and the entry names
   only the first. Find out what the sixteen seconds is before touching the
   ladder, because it is the whole cost once the ladder is fixed.
4. **[T-018](disk-io.md)**, P2 and **effort M**, and the entry has already
   bounded what it is worth: writes take about 806 ms of an eight-bridge
   `bench leech` run's 2,510 ms, so coalescing to 1 MiB is worth at most 18%.
   The Approach names three correctness constraints and none can be traded, so
   this is not the small change the neighbouring `pwrite_all_vectored` makes it
   look like.

Do not start [T-163](peers.md) MSE, [T-102](bep-coverage.md) BEP 55,
[T-167](bep-coverage.md) BEP 54, [T-016](disk-io.md) fastresume, the send half
of [T-100](bep-coverage.md), or the peer-only cap in
[T-132](multi-source.md). All six are blocked on `librqbit` seams and all six
name the blocker with line numbers.

## Open questions for the operator

None.
