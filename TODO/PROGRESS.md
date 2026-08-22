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

## The record has no open P0 left

Both of them closed on 2026-08-22, and both took a change to somebody else's
code that no amount of configuration could reach. That is what the fork is for,
and it is the first session where owning it paid.

- **[T-020](peers.md)**, open since the record began: one `select!` match arm.
- **[T-194](peers.md)**, filed and closed the same day: a bitfield that does not
  fit in one message buffer.

The work order is still [`patches/TASKS.md`](../patches/TASKS.md). Item 0 and
item 1 are done; **item 2, [T-040](memory.md), is next** and is the last P0-sized
thing on that list.

## State

- **Last session:** 2026-08-22T13:05:27Z to 15:00Z, unattended throughout, no
  operator redirection.
- **Tests:** 1,116 passing, 0 failing, unchanged from the baseline at the start.
  Plus **139** in the vendored trees, one of them new, which the workspace gates
  do not run.
- **Gates:** clean.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green at run **32578664398** against commit `faa4543`. Run
  **32580352540** against `592900b` was queued when this was written and is the
  one to read first.
- **Entries:** 156 items. 48 open, 6 partial, 2 blocked, 90 done, 10 deferred to
  Phase C. 90 of 146 workable done, 56 left.
- **Tree:** 84 Rust files, 49,845 lines of code, 11,608 of comment, `scc
  --no-cocomo crates/`. That excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **9 patches**
  carried across three sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

### Item 0: is rqbit#637 ours, and it is

**[T-194](peers.md)**, filed and **done**, P0. `patches/TASKS.md` asked whether
[rqbit#637](https://github.com/ikatson/rqbit/issues/637), "rqbit faill to add
torrent larger than 2MB", reaches `bit-cli` before anything is built on the
vendored tree. A defect of that shape is real and it was ours.

- **The size of the `.torrent` is not the variable, the piece count is.** Every
  peer message was serialized into one fixed buffer, `MAX_MSG_LEN` = 16,500
  bytes, sized for a `ut_metadata` chunk. A bitfield is one bit per piece, so
  past **131,960 pieces** it did not fit and the connection was dropped before
  anything was served.
- **Measured to one piece.** 131,960 works, 131,961 does not, and 16,500 is
  `MAX_MSG_LEN` exactly. Both are 2.64 MB torrents, which is what rules the file
  size out: the upstream title says "larger than 2MB" and one of those two is
  fine.
- **Adding is not what fails.** `create`, `info`, `verify` and `seed` all handle
  a 3.13 MiB `.torrent`, and `create` builds one from 160 MiB of payload in
  0.195 s. TASKS.md asked whether that was fast enough to test with. It is.
- Whether this **is** #637 cannot be settled: the issue body is empty. The entry
  says so rather than claiming the scalp.
- **Residual, and it is measured:** [T-195](peers.md), open. The read side is a
  32,768 byte ring buffer, so 262,105 pieces still fails with "read buffer is
  full". Both halves now cap at the same place, twice as far out as before.

### Item 1: T-020, the open P0

**[T-020](peers.md)**, **done**, and it is one match arm. The entry had already
found the mechanism; this session wrote the change and proved it.

- `task_listener`'s second `select!` arm matched `Some(Ok(..))`. A `select!` arm
  whose pattern **fails** is disabled for the rest of that call, so one handshake
  check resolving to `Err` left the loop waiting on `accept()` alone.
- **The entry's own acceptance had never passed.**
  `scripts/check-close-wait.ps1 -Ceiling 100`: CLOSE_WAIT **986 to 0**, handles
  **188->1210 to 188->194**.
- **The worse half was never a socket count.** A backed-up queue stopped the
  seeder handshaking for **any** info hash, including one it was serving, while
  it reported itself as seeding. A 100 connection load now leaves the next peer
  served in full: 8,388,608 bytes where it got 0.

### What closing it broke, which is the part worth reading

- **Three of `check-listener.ps1`'s four cases asserted the defect.** They are
  inverted rather than deleted, so they hold the fix now. `poisoned`, which
  required exit 17, is `survives_load`.
- **What that costs is written down rather than glossed.** The old case was the
  only end-to-end proof that `--listener-check` can stop a real run, and there is
  no longer a way to poison a listener to produce one. Three unit tests cover the
  decision; nothing covers the wiring.
- **`--listener-check`'s threshold of three was derived from the drain rate**,
  and that derivation is gone. Three is still right for a different reason, and
  the entry says which.
- **`sources_ignored` in `check-swarm.ps1` was resting on the defect.** It reads
  the socket table while the run is connected and used the connect load because
  "its peers hold their connections for the whole duration". They did, but only
  because the target could not answer them. With the loop draining, that load
  exits in **53 ms**, shorter than one `Get-NetTCPConnection` call: 6 samples and
  42 sightings became **1 and 0**, and it failed on its own premise, which is the
  one thing it was written to do. The window is made rather than borrowed now, by
  capping the seeder's upload. **An acceptance that needs the system under test
  to be slow is measuring the defect.**

### Two things found by running what the docs already said to run

- **[T-197](cli-surface.md)**, filed and **done**, P1.
  `cargo test --manifest-path vendor/rqbit/Cargo.toml` **could never have
  worked**: the vendored workspace lists `desktop/src-tauri` as a member and
  `desktop/` is one of the four things deliberately not vendored, so cargo could
  not load the workspace at all. Running it then left **7.2 GB and 9,894 files**
  of build output inside the vendored tree, and `vendor-diff.ps1` walked all of
  it and wrote **14,964 patches** after looking hung for seven and a half
  minutes. Both scripts skip a path that a `.gitignore` **inside** the tree
  ignores now, and the qualifier is the point: `vendor-sync` still has to report
  a file this repository's own root `.gitignore` would swallow, which is the
  `.vscode/` case. 7 patches, 6.1 s. The documented command carries
  `--target-dir` so the mess is not made in the first place.
- **[T-196](cli-surface.md)**, filed, open. `bit-cli download <magnet>` bounds
  metadata resolution by `--init-timeout` only when a file selection forces the
  bounded path. Without one it calls `engine.add`, which has no bound. Found by
  a magnet download that ran for **ten minutes** and was killed by the harness
  rather than by `bit-cli`, while the seeder had logged the reason in the first
  second.

### What went wrong and was fixed

- **A measurement was thrown away by a fixed sleep.** The first boundary harness
  waited four seconds for a seeder rather than waiting for the port to accept,
  and one run reported "connection actively refused" as a negative result.
  RULES.md section 5 already carries that rule; the script now polls the port.
  `scripts/check-bitfield.ps1` waits on the listener.
- **`patches/UPSTREAM.md` named patch files by index**, and adding a patch
  renumbers every later one. Three filenames went stale the moment
  `session.rs` sorted in ahead of them. `vendor-status.ps1` catches it, but the
  numbering is a shape worth watching.

## In progress

Nothing is half-written.

- **[T-040](memory.md)** is item 2 in [`patches/TASKS.md`](../patches/TASKS.md)
  and was not started.
- **[T-024](peers.md)** is still the one ordinary entry left over from the work
  order two sessions ago. Not blocked on anything.
- **[T-195](peers.md)** and **[T-196](cli-surface.md)** were filed this session
  and are open.

## Start here next session

**Work from [`patches/TASKS.md`](../patches/TASKS.md).** Items 0 and 1 are done.
What follows is its shape from item 2 on, not a second copy of it.

1. Read CI run **32580352540** against `592900b` first. It was queued when this
   was written.
2. **Item 2, [T-040](memory.md)**, partial, P0-adjacent and the last big one on
   that list. Nothing reclaims a peer row and nothing bounds the sets. Prior art:
   four of the nine `nzbd` patches, MIT OR Apache-2.0, at
   <https://github.com/pjunod/nzbd/tree/main/contrib/rqbit>, forward ported from
   8.1.1. `0010-bound-known-peer-records`, `0012-bound-peer-response-backlog`,
   `0014-bound-discovery-pressure`, `0016-limit-peer-metadata-before-allocation`.
   **Do not adopt `0009`**: [T-020](peers.md) measured that the cap it adds has
   nothing to do with anything here, and now that T-020 is closed the reason is
   even clearer.
   T-020's handshake mode still grows handles 188 to 226 over 2,000 connections,
   and that residue is T-040's, not T-020's.
3. **Item 3, MSE, [T-163](peers.md).** Decide first whether to take upstream's
   shape from [rqbit#633](https://github.com/ikatson/rqbit/pull/633) or our own.
   Corpus: `reference/FluxDown/native/engine/vendor/librqbit/src/mse/`, MIT, on
   this machine already.
4. **[T-195](peers.md)** when the read side is worth the risk. It needs an
   overflow path in a ring buffer whose `read_message` holds an unsafe reborrow
   with a miri test around it, which is a larger change than the send side was.
5. **[T-196](cli-surface.md)** is small and self-contained: move a bound that
   already exists fifty lines up the same function.

**Offer both new patches upstream.** [T-020](peers.md) is
[rqbit#311](https://github.com/ikatson/rqbit/issues/311), open upstream, and the
change is one match arm. [T-194](peers.md) has a one line reproduction. Neither
has been offered; `patches/UPSTREAM.md` says so on each rather than claiming
otherwise.

Do not start the entries that were blocked on `librqbit` seams **outside** the
order in `patches/TASKS.md`.

## Open questions for the operator

None outstanding. Nothing was put to the operator this session and nothing was
blocked.

One decision was taken unattended and is recorded where it belongs: three of
`scripts/check-listener.ps1`'s four cases, and one in `scripts/check-swarm.ps1`,
were **inverted rather than deleted** when [T-020](peers.md) closed, so they hold
the fix instead of recording the defect. The coverage that went with the old
shape is named in the entry rather than left to be discovered.
