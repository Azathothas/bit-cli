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

## Before typing a `bit-cli` flag, read `man/bit-cli.json`

New on 2026-08-22, and it exists because an agent was watched greping the source
for flag names. `man/` holds the whole command surface, generated and committed:
`bit-cli.1` for a terminal, `bit-cli.md` for reading, and **`bit-cli.json`, a
CLIspec 0.3 document, for a program**. Every command, every flag, the values it
accepts, its default, and every exit code with whether a retry could succeed.

It cannot go stale: `cargo test -p bit-cli --test man_is_current` fails until it
is regenerated with `pwsh -NoProfile -File scripts/check-man.ps1 -Fix`.
[`docs/man.md`](../docs/man.md) says what each field carries.

## Both P0 items are closed, and the third is bounded

`patches/TASKS.md` items 0, 1 and 2 are done or bounded. **No open P0 is left in
the record.**

- **[T-020](peers.md)**, done: one `select!` match arm.
- **[T-194](peers.md)**, filed and done: a bitfield that does not fit one buffer.
- **[T-040](memory.md)**, still partial, but the defect is fixed: peer rows are
  bounded now, and what remains is a six hour soak rather than a change.

## State

- **Last session:** 2026-08-22T16:41:28Z, running, unattended. It is working
  [`patches/TASKS.md`](../patches/TASKS.md) in order, and the operator excluded
  section 3, MSE, which is large and belongs to the next session. The session
  before it ran 2026-08-22T13:05:27Z to 16:20Z and closed both P0 entries.
- **Tests:** 1,126 passing, 0 failing. 1,116 at the start; the ten new ones are
  the CLIspec generator and the manual drift checks. Plus **139** in the
  vendored trees, which the workspace gates do not run.
- **Gates:** clean, and they have a `man` gate now.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green at run **32583835258** against commit `a1b440e`, all sixteen
  jobs. That run is the one that matters for this session: every Windows job in
  it installed NASM with `scripts/setup-nasm.ps1` rather than the abandoned
  action, so the replacement is proved on a real runner rather than only here.
  GitHub picked up `.github/dependabot.yml` on the same push and started a
  Dependabot run against it.
- **Entries:** 159 items. 45 open, 5 partial, 0 blocked, 99 done, 10 deferred to
  Phase C. 99 of 149 workable done, 50 left.
- **Tree:** 86 Rust files, 50,406 lines of code, 11,741 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **18 patches**
  across four sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

### The vendored work: three patches, two P0 entries closed

- **[T-194](peers.md)**, P0, **done**. `MAX_MSG_LEN` is 16,500 bytes, sized for
  a `ut_metadata` chunk, and a bitfield is one bit per piece. Past **131,960 pieces** it did not fit and the connection was dropped
  before anything was served, in either role. Measured to one piece: 131,960
  works, 131,961 does not, and both are 2.64 MB torrents, which is what rules
  out the file size the upstream report names. Residual, measured and open:
  [T-195](peers.md), the read side, at 262,104 pieces.
- **[T-020](peers.md)**, P0, **done**, and it is one match arm. A `select!` arm
  whose pattern fails is disabled for the rest of that call, so one handshake
  check resolving to `Err` left the accept loop waiting on `accept()` alone.
  The entry's own acceptance had never passed: CLOSE_WAIT **986 to 0**, handles
  **188->1210 to 188->194**. The worse half was never a socket count: a backed
  up queue stopped the seeder handshaking for **any** info hash while it
  reported itself as seeding.
- **[T-040](memory.md)**, P0, still **partial**. Nothing reclaimed a peer row;
  there is a bound now, 1,024 per torrent, taking `NotNeeded` and `Dead` rows
  and never a live one. 2,000 connections leave **exactly 1,024** rows where
  they left 2,000. RSS at that scale is unchanged and that is expected: freeing
  a row returns it to the allocator, not to the operating system. **What is left
  is the six hour soak this entry's acceptance names, not a change.**

### The redirection: the manuals, and two CI things

- **[T-198](cli-surface.md)**, P1, **done**. `man/bit-cli.1`, `man/bit-cli.md`
  and `man/bit-cli.json`, generated, committed, and held current by a test. It
  caught two bugs in its own first output: `--web-seed` typed `boolean` while
  carrying `value_name: URL`, because `get_num_args` is empty until the command
  is built; and `create --version` deleted, because filtering clap's generated
  `--version` by argument id also removed the metainfo one.
- **[T-199](cli-surface.md)**, P2, **done**. `ilammy/setup-nasm@v1.5.2` is
  unmaintained, runs on node20, and was used in five places.
  `scripts/setup-nasm.ps1` replaces it and verifies a pinned SHA-256, which the
  action never did; both the success and the mismatch were run.
  `.github/dependabot.yml` watches cargo and github-actions, grouped, with
  `vendor/` and `librqbit*` deliberately excluded and the file saying why.

### What went wrong and was fixed

- **[T-197](cli-surface.md)**, P1, **done**, and it was found by following the
  instructions. `cargo test --manifest-path vendor/rqbit/Cargo.toml` **could
  never have worked**: the vendored workspace lists `desktop/src-tauri`, which
  is deliberately not vendored. Running it then left **7.2 GB and 9,894 files**
  in the vendored tree and `vendor-diff.ps1` wrote **14,964 patches** after
  looking hung for seven and a half minutes. Both scripts skip a path that a
  `.gitignore` **inside** the tree ignores now.
- **[T-196](cli-surface.md)**, filed, open. `bit-cli download <magnet>` has no
  bound on metadata resolution unless a file selection forces the bounded path.
  Found by a ten minute hang the harness killed.
- **Three acceptance scripts were asserting defects.** Closing T-020 broke them,
  which is the right way round. `check-listener.ps1`'s `poisoned` case required
  exit 17 and is now `survives_load`; `check-swarm.ps1`'s `listener_poisoned`
  carried `judged: false` and is judged; and `sources_ignored` was resting on
  the target being **unable to answer its peers**, so its sampling window went
  from 6 samples to 1 the moment the accept loop drained. The window is made
  rather than borrowed now, by capping the seeder's upload.
- **Review 1 found three of this session's own numbers wrong**: two `man/` file
  sizes recorded before the nested subcommands were classified, and an RSS pair
  divided by a million rather than by 1,048,576. All three corrected. It also
  found "arrives under `rustls`" incomplete: `cargo tree -i aws-lc-rs` says
  `aws-lc-sys` arrives under `librqbit-sha1-wrapper` as well, so dropping TLS
  would not remove the NASM requirement.
- **`check-todo` caught six drifted citations** the moment `ManFormat` moved
  `cli.rs` by seventeen lines.

## In progress

Nothing is half-written.

- **[T-040](memory.md)** is partial: bounded and measured, waiting only on the
  six hour soak its acceptance names.
- **[T-195](peers.md)**, **[T-196](cli-surface.md)** were filed this session and
  are open. **[T-024](peers.md)** is still the ordinary entry left over from two
  work orders ago.

## Start here next session

**The vendor patching list, [`patches/TASKS.md`](../patches/TASKS.md), and the
hope is to finish it.** Items 0, 1 and 2 are done or bounded.

1. Nothing to read first. CI is green at **32583835258** against the tip.
2. **[T-040](memory.md) to done**: run `scripts/soak.ps1` for six hours on the
   `steady` workload and record the slope of each series. That is the whole of
   what is left, and it is a measurement rather than a change. Start it early:
   it outlasts most of a session.
3. **Item 3, MSE, [T-163](peers.md).** Decide first whether to take upstream's
   shape from [rqbit#633](https://github.com/ikatson/rqbit/pull/633) or our own,
   and read `patches/README.md` under "Upstream is not automatically right"
   before deciding. Corpus:
   `reference/FluxDown/native/engine/vendor/librqbit/src/mse/`, MIT, already on
   this machine.
4. **Item 5's list**, in the order `TASKS.md` gives: [T-022](peers.md),
   [T-132](multi-source.md), [T-100](bep-coverage.md),
   [T-167](bep-coverage.md), [T-102](bep-coverage.md). Each names its seam with
   a line number.
5. **[T-195](peers.md)** is the read side of T-194 and is the one remaining
   residual bound. It needs an overflow path in a ring buffer whose
   `read_message` holds an unsafe reborrow with a miri test around it.
   `RULES.md` section 5 now says plainly that a residual bound is allowed only
   while it is measured, named and carried open, which this is.
6. **[T-196](cli-surface.md)** is small: move a bound that already exists fifty
   lines up the same function.

**Offer all three new patches upstream.** [T-020](peers.md) is
[rqbit#311](https://github.com/ikatson/rqbit/issues/311), [T-040](memory.md) is
[rqbit#525](https://github.com/ikatson/rqbit/issues/525), and [T-194](peers.md)
has a one line reproduction. None has been offered and `patches/UPSTREAM.md`
says so on each.

## Open questions for the operator

None outstanding. Four instructions arrived mid-session and all four are done
and written into the rules rather than only into the code:

- **The manuals**, and that agents must read them rather than guess:
  [RULES.md](RULES.md) section 4a and [`docs/man.md`](../docs/man.md).
- **Dependabot and dropping `ilammy/setup-nasm`**: [T-199](cli-surface.md).
- **Stop deferring.** [RULES.md](RULES.md) section 5's no-deferral rule now says
  that "it is in somebody else's code" is the reason the trees are vendored
  rather than an excuse, and that a residual bound is allowed only when it is
  measured, named and carried as its own open entry.
- **Do not trust upstream blindly.** `patches/README.md` has a section,
  "Upstream is not automatically right", with the three questions a
  reconciliation must answer before taking any hunk that touches something this
  repository already changed, and the instruction to keep ours and say why when
  ours is better.
