# Progress

**Read this first.** It says what the last session did and what is in flight.
It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
The work order: [INDEX.md](INDEX.md), "Start here".

---

## State

- **Last session:** 2026-08-21. Documentation and planning. One code change,
  and it was a red CI job rather than feature work.
- **Tests:** 931 passing, 0 failing, measured with `cargo test --workspace`.
- **Gates:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  clean, `cargo fmt --all --check` clean, `cargo deny check` reports advisories,
  bans, licenses and sources all ok.
- **CI:** green on all sixteen jobs at run **32479072800**, against commit
  `8abee2a`. The run before it, 32478382564, turned `Test (macos-latest)` red
  on a documentation-only push, which is [T-182](cli-surface.md) and is fixed.
  Check the current state with `gh run list --limit 1`.
- **Entries:** 141 items. 63 open, 4 partial, 1 blocked, 63 done, 10 deferred
  to Phase C.

## What the last session did

Reconciled every document in this repository against the code, then folded a
twenty-two tree research corpus into `TODO/`. Writing the plan was the work,
and the only code that changed was a test defect CI found on the way.

- **Docs against code, two passes.** The `README.md` protocol table said four
  things the tree does not do, and BEP 7 and BEP 53 were implemented and absent
  from it. `CHANGELOG.md` claimed nothing is stubbed; six flags are.
- **Three entries described a state this tree is not in**, and all three were
  corrected under the entry rather than closed: [T-033](performance.md),
  [T-118](cli-surface.md), [T-161](cli-surface.md).
- **Twenty new entries**, [T-163](peers.md) to [T-182](cli-surface.md), of
  which nineteen are open. Seventeen from the corpus, two from reading this
  tree: `httpseeds` accepted
  as a list only while `url-list` beside it accepts both
  ([T-171](metainfo.md)), and four flags that parse and reach no code
  ([T-181](cli-surface.md)).
- **New files:** [metainfo.md](metainfo.md) for reading a `.torrent` somebody
  else wrote, [RULES.md](RULES.md), this file, and `scripts/git-sync.ps1`,
  which is now the only sanctioned way to commit and push.
- **One code change**, and it was not planned. A documentation-only push turned
  `Test (macos-latest)` red: `sysinfo.rs` asserted `peak_rss >= rss`, and on
  Darwin those two come from different kernel subsystems that share no
  accounting basis. Fixed at the source rather than by weakening the test.
  [T-182](cli-surface.md).
- **The work order in [INDEX.md](INDEX.md) was re-derived** with the corpus in
  hand, and correctness in the web seed path now outranks the open P0.

## In progress

Nothing is half-written. Every entry touched is either complete or explicitly
open with its blocker named.

Two things are carried rather than finished, and both are recorded where they
belong:

- **[T-020](peers.md)** stays the only open P0. Half of it is fixed; the
  stranded-socket half and the poisoned-listener finding are open.
- **[T-040](memory.md)** and **[T-090](bench.md)** stay partial. Neither needs
  a longer run; both need the specific measurements named in their entries.

## Start here next session

Feature work resumes. The order is in [INDEX.md](INDEX.md), and the first four
are:

1. **[T-171](metainfo.md)**, effort S. `Metainfo::http_seeds` at
   `crates/bit-cli-core/src/torrent/metainfo.rs:306` accepts a list only, so a
   torrent whose `httpseeds` is a bare bencoded string yields zero HTTP
   sources, silently. `url_list` at `:293` already handles both shapes with a
   test at `:656`. Corpus: `gosh-dl/src/torrent/metainfo.rs:391`.
2. **[T-166](peers.md)**, effort S, P1. Prove the BEP 10 extension map keys
   peer-id-to-handler and name-to-our-id as two separate directions. Corpus:
   vortex PR 103, where getting it backwards meant extensions had never once
   worked against qBittorrent.
3. **[T-181](cli-surface.md)**, effort M, P1. Four flags accepted in silence.
   Warn today the way `crates/bit-cli/src/cmd/seed.rs:105` does for
   `--superseed`, then build the two that can be built.
4. **[T-167](bep-coverage.md)** then **[T-005](webseed.md)**. BEP 54
   `lt_donthave` first, because it turns T-005's reconnect into a message.
   Corpus: `fx-torrent/src/peer/extension/donthave.rs:19`,
   `torrent/webseed-peer.go:57`.

## Open questions for the operator

None blocking. One worth an answer when convenient:

- **[T-172](metainfo.md)** asks whether `bit-cli` is strict on read for
  unsorted bencode keys and for trailing bytes after the top-level dictionary.
  The entry recommends strict inside `info` and tolerant outside it, with the
  reasoning, and can be built on that recommendation without an answer. Two
  upstream implementations resolved it in opposite directions, which is why it
  is a decision rather than a defect.
