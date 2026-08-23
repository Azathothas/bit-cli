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
>
> `scripts/check-todo.ps1` checks most of that shape now, and `scripts/gates.ps1`
> runs it, so a missing section or a stale count fails a gate rather than a
> review. [RULES.md](RULES.md) section 5, "The record".

---

## Before typing a `bit-cli` flag, read `man/bit-cli.json`

`man/` holds the whole command surface, generated and committed: `bit-cli.1` for
a terminal, `bit-cli.md` for reading, and **`bit-cli.json`, a CLIspec 0.3
document, for a program**. Every command, every flag, the values it accepts, its
default, and every exit code with whether a retry could succeed.

It cannot go stale: `cargo test -p bit-cli --test man_is_current` fails until it
is regenerated with `pwsh -NoProfile -File scripts/check-man.ps1 -Fix`.
[`docs/man.md`](../docs/man.md) says what each field carries.

## No P0 is open, and nothing in the record is blocked

**All three P0 entries are done.** [T-040](memory.md) was the last and it closed
on a measurement rather than a change: six hours of `scripts/soak.ps1`.

`blocked` went from two entries to zero. Both were blocked on `librqbit` rather
than on anything here, which is what vendoring it was for.

- **[T-016](disk-io.md)**, done: a resume cache needed session persistence, and
  `SessionOptions` takes a `BitVFactory` now.
- **[T-167](bep-coverage.md)**, partial: BEP 54 had no receive side upstream. It
  has one; the send half is this repository's own bridge.

## State

- **Last session:** 2026-08-23T01:53:05Z, unattended, running. It is working
  [`patches/TASKS.md`](../patches/TASKS.md) in order from section 3, MSE, which
  the session before it was told to leave alone. The session before ran
  2026-08-22T16:41:28Z to 23:05Z, unattended, 6h 24m.
- **Tests:** 1,131 passing, 0 failing. 1,126 at the start. Plus **142** in the
  vendored trees, which the workspace gates do not run, up from 139.
- **Gates:** clean, and they have a `record` gate now.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green at run **32603933755** against commit `cdc7bea`, all
  **seventeen** jobs, and that commit is this session's last change to source.
  The seventeenth job is `Record`, added this session, which runs
  `scripts/check-todo.ps1` where `-SkipGates` cannot reach it. Two runs went
  red mid-session and neither was this tree's fault: see [T-211](bench.md).
- **Entries:** 160 items. 44 open, 2 partial, 0 blocked, 104 done, 10 deferred
  to Phase C. 104 of 150 workable done, 46 left.
- **Tree:** 87 Rust files, 50,718 lines of code, 11,912 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **23 patches**
  across twelve sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What this session is doing

Written before the work, by [RULES.md](RULES.md) section 1 step 4. The order is
[`patches/TASKS.md`](../patches/TASKS.md)'s, resumed at its section 3.

1. **[T-163](peers.md), MSE**, `patches/TASKS.md` section 3. The decision it
   asks for is taken below.
2. **[T-211](bench.md)**, the two bench tests that fail on the CI runner and
   pass here.
3. **[T-167](bep-coverage.md)**'s send half, `lt_donthave`.
4. **[T-100](bep-coverage.md)** BEP 6 fast extension and
   **[T-102](bep-coverage.md)** BEP 55 holepunch.
5. **Section 4's four unread nzbd patches**, `0012`, `0014`, `0016`, `0005`.
6. The dependabot pull requests, which all fail CI, and then CI green again.

## What the last session did

### The record cannot go stale silently any more

The operator asked for this first, and the reason was sitting in the working
tree: `patches/TASKS.md` said `T-020 | P0 | open` at HEAD for a whole session
after T-020 closed. Nothing was wrong with any single file. What was missing was
anything that compared two of them.

`scripts/check-todo.ps1` compares them now: every `TASKS.md` row against the
entry it names, for status, priority and link; its totals against its own rows;
and `PROGRESS.md` against what [RULES.md](RULES.md) section 2 step 2 requires,
including every count it quotes and the patch count on disk. `gates.ps1` runs it
as the `record` gate and CI runs the same script as a job, because `-SkipGates`
exists. Proved by breaking it. [RULES.md](RULES.md) section 5 has the rule.

It earned itself the same day: it caught six drifted citations the moment two
new flags moved `cli.rs`, and eleven dead patch paths after a renumbering.
`vendor-diff.ps1` renumbers those citations itself now.

### Seven entries closed, one advanced, two filed

- **[T-022](peers.md)**, P1, done. An HTTP tracker was told about one of this
  host's two addresses while a UDP tracker in the same file was told about both.
  A `reqwest` client per family, pinned by overriding the resolution, announcing
  in sequence. **ipv6 alone before, both after.**
- **[T-132](multi-source.md)**, P1, done. `--max-peer-rate` caps the swarm and
  not an HTTP source this process attached. An 8 MiB/s peer cap holds peers to
  **8.42 MiB/s** and lets HTTP run at **151.84 MiB/s**.
- **[T-210](peers.md)**, P1, filed and done, and it is why the first attempt at
  T-132 did not work. `manage_peer_incoming` handed `on_handshake` the handshake
  it had just built to send, so **every incoming peer was filed under this
  session's own peer id** and was assumed to speak BEP 10.
- **[T-195](peers.md)**, P2, done. `ReadBuf` grows now, bounded by what the
  connection says the torrent could need rather than by what the peer claims to
  be sending. **1,048,576 pieces resolve** where 262,105 did not.
- **[T-016](disk-io.md)**, P2, done. `bit-cli seed --fastresume`. The cache is
  derived data and no session state is written.
- **[T-196](cli-surface.md)**, P2, done. The branch an ordinary
  `download <magnet>` takes ignored `--init-timeout`. **10.04 s to 4.04 s**, and
  the report names the phase.
- **[T-025](peers.md)**, P3, done, one `pub use`.
- **[T-167](bep-coverage.md)**, P2, blocked to partial. BEP 54 `lt_donthave` is
  received and honoured; nothing here sends one yet.
- **[T-211](bench.md)**, P1, filed. Two bench tests fail on the CI runner and
  pass on every local run.

### The six hour soak, and the last P0

**[T-040](memory.md), P0, done**, and it is the entry that has been open
longest. `scripts/soak.ps1` on the `steady` workload, **687 samples over 6.00
hours**, 1,372 leech cycles and none failed.
`bench/soak-20260822T164952755Z.csv`.

**Read the whole-run RSS slope and you would conclude the bound did nothing.**
0.815 MiB an hour against the 0.804 measured before it. That number averages
two regimes and describes neither. The bound is 1,024 rows per torrent and this
workload completes about 229 cycles an hour, so the map fills **4.65 hours in**,
which was read live off the seeder's own `progress` events at 1,024 rows against
1,079 peers seen. Fitting either side of that instant:

| window | samples | slope | r squared |
| --- | --- | --- | --- |
| **before**, 0 to 4.65 h | 531 | **+0.909 MiB/h** | 0.799 |
| **after**, 4.65 to 6.00 h | 156 | **-0.140 MiB/h** | 0.005 |

13.74 MiB to 18.61 MiB, then 18.68 MiB to 18.72 MiB. A straight line for four
and a half hours, then flat. That is the bound working, measured end to end.

**An interim read at 5.06 hours said the opposite.** 55 samples after the elbow,
+1.45 MiB/h at r squared 0.107, which is noise fitted to a line. A slope needs a
window long enough to have a shape.

**Handles flat** at -0.315 an hour, r squared 0.003, and **`CLOSE_WAIT` zero at
all 687 samples**. The second is [T-020](peers.md)'s fix holding for six hours
under load rather than for the length of an acceptance script.

**The soak survived every `gates.ps1` run in those six hours**, which it would
not have before: `gates.ps1` kills stray `bit-cli` processes, and `soak.ps1`
copies its binaries to `.tmp/` precisely so they hold no build output open.
`gates.ps1` leaves anything under `.tmp/` alone now.

### What went wrong and was fixed

- **Two CI runs went red and neither was this tree's fault.** The failed job
  from 32594170837 was re-run on its own commit, unchanged, and passed.
  [T-211](bench.md) carries it. Worth knowing: the CI workflow groups by
  `workflow-ref` with `cancel-in-progress`, so re-running an older commit's job
  while a newer push is in flight cancels one of them, and GitHub then refuses
  to retry a cancelled run at all.
- **Review 1 found three numbers wrong.** The pre-metadata read cap is 8,388,568
  pieces and not 8,388,600; the `record` gate is not the cheapest gate there is;
  and two files opened by quoting the eight minute figure for a 40 GiB seed that
  the same session had already disproved.
- **Three fixtures were tried for [T-196](cli-surface.md) and the first two
  measured the wrong thing.** A closed port stops the run in two seconds and an
  accept-and-never-write listener in ten, both by exhausting the address list
  rather than by the flag, so either would have passed before the fix.
- **A local named `$doc` shadowed the enclosing `$doc`** in `vendor-diff.ps1`,
  which is the exact hazard [RULES.md](RULES.md) section 5 already carries.

## In progress

Nothing is half-written.

- **[T-167](bep-coverage.md)** is partial with the send half specified in three
  steps in the entry.
- **[T-211](bench.md)**, **[T-024](peers.md)** are open.

## Start here next session

1. **Read the CI run named above before anything else.**
2. **Item 3 of [`patches/TASKS.md`](../patches/TASKS.md), MSE,
   [T-163](peers.md)**, which the operator held back from the last session as
   too large for it. Decide first whether to take upstream's shape from
   [rqbit#633](https://github.com/ikatson/rqbit/pull/633) or our own, and read
   `patches/README.md` under "Upstream is not automatically right" before
   deciding. Corpus:
   `reference/FluxDown/native/engine/vendor/librqbit/src/mse/`, MIT, already on
   this machine, holding `dh768.rs`, `rc4.rs`, `stream.rs` and `mod.rs`.
3. **[T-211](bench.md)**, P1, because a test that fails one run in three costs
   every session after this one. Both assertions are counts over a loopback
   swarm running to a wall clock, which is the shape
   [RULES.md](RULES.md) section 5 forbids. Arrange them, do not widen them.
4. **[T-167](bep-coverage.md)'s send half.** The entry has it in three steps:
   the bridge reading the session's `m` out of the extended handshake, the
   `FileGone` path sending one message per dropped piece instead of
   reconnecting, and the entry's own acceptance, which already has a `FileGone`
   fixture behind it.
5. **Item 5's remainder**: [T-100](bep-coverage.md) BEP 6 fast extension, effort
   L, and [T-102](bep-coverage.md) BEP 55 holepunch, P3. `TASKS.md` section 5
   names the seam for each.
6. **Section 4's four unread nzbd patches**, `0012`, `0014`, `0016` and `0005`.
   `0001` is spent: the entry it was for closed another way.

**Offer the patches upstream.** Twelve sections in
[`patches/UPSTREAM.md`](../patches/UPSTREAM.md) and not one has been offered.
[T-020](peers.md) is [rqbit#311](https://github.com/ikatson/rqbit/issues/311),
[T-040](memory.md) is [rqbit#525](https://github.com/ikatson/rqbit/issues/525),
[T-022](peers.md) is [rqbit#537](https://github.com/ikatson/rqbit/issues/537),
and the rest carry a one line reproduction each.

## Open questions for the operator

None outstanding. The operator gave three instructions at the start and all
three are done:

- **Reconcile `patches/TASKS.md` with `PROGRESS.md` and the git history.** Done,
  and it is kept reconciled by a gate rather than by attention: the table has
  thirteen rows now, each checked against the entry it names.
- **Make it impossible for an agent to leave the record stale**, in the rules
  and in an automated check. [RULES.md](RULES.md) section 5 has "The record",
  `gates.ps1` has the `record` gate, and CI has a `Record` job because
  `-SkipGates` exists.
- **Finish `patches/TASKS.md` except MSE.** Not finished. Nine of its thirteen
  rows are done and two more moved; what is left is MSE, which was excluded,
  [T-100](bep-coverage.md) BEP 6 at effort L, [T-102](bep-coverage.md) BEP 55 at
  P3, and [T-167](bep-coverage.md)'s send half. The work order above puts them
  in order. Nothing is half-written.
