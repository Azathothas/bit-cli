# Progress

**Read this first.** It says what the last session did and what is in flight.
It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
The work order: [INDEX.md](INDEX.md), "Start here".

---

## State

- **This session:** started 2026-08-22T01:11Z, unattended. Feature work,
  working the "Start here" order in [INDEX.md](INDEX.md).
- **Baseline carried in from 2026-08-21T17:20Z**, re-measured at the start of
  this session rather than trusted: 1,028 tests passing, 0 failing; clippy,
  fmt and `cargo deny` clean; CI green on all sixteen jobs at run
  **32507560214** against commit `76e33e8`.
- **Entries:** 146 items. 56 open, 4 partial, 2 blocked, 74 done, 10 deferred
  to Phase C.

## What this session is going to do

The first four of the "Start here" order, in that order. Each closes with the
acceptance command from its own entry, run, with the output recorded.

1. **[T-185](cli-surface.md)**, `crates/bit-cli/src/cmd/download.rs` and
   `crates/bit-cli/src/selection.rs`, effort S, P1. `--exclude-file` used
   without `--select-file` selects nothing and downloads everything, because
   `download` resolves the selection with no file count and the exclusion-alone
   branch returns `None` for every file. The count exists for every source but
   a magnet: `run` parses the metainfo into `metas` before any plan starts. The
   magnet half and `--select-file 3-` are decided with it, as the entry says.
2. **[T-143](multi-source.md)**, `TODO/multi-source.md`, effort M. Attaching a
   source to a torrent that has already started. [T-005](webseed.md) and
   [T-179](webseed.md) built both halves of the machinery between them; what is
   left is where the binding comes from mid-run.
3. **[T-164](peers.md)**, the peer half of smart ban. **Read the `librqbit`
   seam and name it in the entry before pricing it**, the way
   [T-167](bep-coverage.md) had to. Corpus:
   `aria2_rust/aria2-core/src/engine/bt_peer_storage/rejection_state.rs`.
4. **[T-186](cli-surface.md)**, effort S, P3. `seed --data` and `verify --data`
   resolve a multi-file payload differently, and the wrong one reports "partial
   seed" rather than "wrong directory".

Not started this session, and each names its blocker in its own entry:
[T-163](peers.md) MSE, [T-102](bep-coverage.md) BEP 55,
[T-167](bep-coverage.md) BEP 54, [T-016](disk-io.md) fastresume.

## In progress

**[T-185](cli-surface.md) is done**, 2026-08-22T01:40Z. 1,034 tests passing, 0
failing. Both halves of the count problem were decided together and the magnet
half is not what the entry recommended: the entry's
`api_torrent_action_update_only_files` narrows a torrent that is already added,
and by then `librqbit`'s initial check has created the files the selection
excludes. `Engine::resolve_with` reads the metadata first and hands back the
`.torrent` bytes it built, so the add is one resolution and not two. The
correction is written under the entry.

**[T-188](disk-io.md) filed** out of T-185's third acceptance run, and it
corrects [T-013](disk-io.md)'s closing claim. An unselected file lands as a zero
byte file when the selection starts after it:
`librqbit-9.0.0/src/file_ops.rs:322` skips a file with `>` where the file is
exhausted at `==`, so a chunk starting on a file boundary issues a zero length
write to the file before it, and `SafeStorage` creates a file for any write.
P3, effort S, and the cause and the fix are both in the entry.

T-143 next.

## Open questions for the operator

None. The session is unattended and nothing was left pending by the last one.
