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

- **This session:** started 2026-08-22T10:30:26Z, unattended. In progress.
- **Tests:** 1,113 passing, 0 failing, re-measured at the start rather than
  taken from the record.
- **Gates:** clean, re-measured. One command:

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** green at run **32567387663** against commit `74c1a08`. The tip is
  `e82d9cd`, which carries `[skip ci]` and so has no run of its own.
- **Entries:** 149 items. 47 open, 7 partial, 2 blocked, 83 done, 10 deferred
  to Phase C. `check-todo.ps1` reports everything agrees.
- **Tree:** 84 Rust files, 49,649 lines of code, 11,471 of comment, measured
  with `scc --no-cocomo crates/`.

## What this session is doing

The work order the last session left, in its order. Written before the work, by
[RULES.md](RULES.md) section 1 step 4.

1. **[T-190](disk-io.md)**, `crates/bit-cli-core/src/engine.rs:575-577` and
   `crates/bit-cli-core/src/storage.rs:402`. One comment and one behaviour
   disagree about where somebody else's bytes land.
2. **[T-189](bench.md)**, `crates/bit-cli/src/schema_gen.rs`. The `bench`
   reports are outside `docs/schema.md` and a field went into one with the
   contract check green.
3. **[T-018](disk-io.md)**, partial. Decide whether `bench disk`'s `shared`
   layout should stop striding or whether the acceptance clause moves to
   `--layout split`.
4. **[T-024](peers.md)**, the choke and unchoke history a peer row does not
   carry.

## In progress

Everything above.

## Start here next session

To be rewritten at the end of this session.

## Open questions for the operator

None yet.
