# Progress

**Read this first.** It says what the last session did and what is in flight.
It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
The work order: [INDEX.md](INDEX.md), "Start here".

---

## State

- **This session:** 2026-08-21, unattended. Feature work. It is running now and
  this section is what it set out to do, rewritten before any of it was done.
- **Baseline, re-measured at the start rather than trusted:** 960 tests
  passing, 0 failing with `cargo test --workspace`.
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean,
  `cargo fmt --all --check` clean, `cargo deny check` reports advisories, bans,
  licenses and sources all ok. CI green on all sixteen jobs at run
  **32495479998**, against commit `87b389b`.
- **Entries at the start:** 143 items. 56 open, 4 partial, 2 blocked, 71 done,
  10 deferred to Phase C.

## What this session is doing

The "Start here" order in [INDEX.md](INDEX.md), in order, which is:

1. **[T-179](webseed.md)**, `TODO/webseed.md`, effort M. Smart ban. A bad piece
   cannot be attributed to the source that filled it. Corpus:
   `torrent/smartban/smartban.go` and `torrent/smartban.go`.
   Files: a new `crates/bit-cli-core/src/webseed/ledger.rs`,
   `crates/bit-cli-core/src/webseed/bridge.rs`,
   `crates/bit-cli/src/swarm.rs`, `crates/bit-cli/src/cmd/download.rs`,
   `crates/bit-cli-core/tests/webseed_e2e.rs`.
2. **[T-184](disk-io.md)**, `TODO/disk-io.md`, effort M. A boundary piece under
   `--select-file` has no decided behaviour. Corpus:
   `FluxDown/native/engine/src/bt_partfile.rs`.
3. **[T-004](webseed.md)**, `TODO/webseed.md`, effort S. BEP 17
   auto-detection, `--web-seed` command-line case only. Corpus:
   `gosh-dl/src/torrent/webseed.rs`.
4. **[T-172](metainfo.md)**, `TODO/metainfo.md`, effort S. Strictness on read:
   unsorted bencode keys and trailing bytes after the top-level dictionary.
   Corpus: `mkbrr/torrent/update.go`, `TorrentNG/crates/rt-metainfo/src/parse.rs`,
   `rustorrent/docs/DEEP_AUDIT_REPORT_2026-07-13.md`.

T-163 (MSE), T-102 (BEP 55), T-167 (BEP 54) and T-016 (fastresume) are not
started. All four are blocked on `librqbit` seams and all four name the blocker
and what would unblock it in their own entries.

## In progress

This section is rewritten at the end of the session with what actually
happened. Until then it says only that the work above is under way and nothing
is claimed finished.

## Start here next session

Re-derived at the end of this session, in [INDEX.md](INDEX.md) under
"Start here".

## Open questions for the operator

None blocking.

- **[T-172](metainfo.md)** is still formally an open question and does not need
  an answer to start: the entry recommends strict inside `info` and tolerant
  outside it, with the argument. [T-174](metainfo.md)'s closing took the same
  position for piece lengths, strict on write and tolerant on read, without
  having read this entry first.
