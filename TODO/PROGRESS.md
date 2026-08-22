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

**[T-143](multi-source.md) is done**, 2026-08-22T02:00Z. 1,036 tests passing, 0
failing. Measured before building, and the entry's premise was too kind: above
`-j 1` the takers do not fetch the shared file twice, they have no source at
all and never finish. `scripts/check-shared-files.ps1` gained `-Jobs`, which
the acceptance needed, and both runs are recorded:
`bench/shared-files-20260822T014247442Z.json` is the failure and
`bench/shared-files-20260822T015216397Z.json` is the fix.

**[T-164](peers.md) is partial**, 2026-08-22T02:20Z. 1,043 tests passing, 0
failing. The seam was read before anything was priced, which is what the work
order asked, and it split the entry into three parts rather than one.

- `librqbit` already has a peer blocklist, checked at `session.rs:917` before an
  incoming handshake is read and at `torrent_state/live/mod.rs:629` before an
  outgoing dial, and `IpRanges::load_from_url` takes a `file:` URL. So
  **`--block-peer` needed no upstream change and is done**, on `download`,
  `seed` and `peers`.
- Adding to that list mid-run is **blocked**: `Session::blocklist` is a plain
  field behind an `Arc`, and `IpRanges` is in a private module, so even its
  `pub fn new` is unreachable.
- Attributing a bad piece to the right peer is **blocked**:
  `file_ops.rs:310` has the peer and `TorrentStorage::pwrite_all_vectored` does
  not, and `librqbit` already convicts whichever peer delivered the last chunk
  of a failed piece, which is the wrong answer [T-179](webseed.md) was written
  to stop giving.

**One defect found while running T-164's gates, and fixed rather than filed.**
[T-179](webseed.md)'s acceptance test failed twice under whole-suite load with
`served [655360, 0]`: the honest mirror had finished the whole payload before
the liar's bridge task was scheduled, so nothing was ever disputed. It reran
clean twenty times on an idle machine, including six from a worktree at
`86445bf` with none of this session's changes, which is what rules out the work
in flight. `librqbit`'s `piece_tracker.rs:114` assigns a piece to one peer at a
time unless another steals it, so "both mirrors served" was a scheduling
outcome the test did not control. The liar now attaches first, scoped to half
the payload, and the healthy mirror joins once the liar has served a byte. The
correction is under T-179 and [RULES.md](RULES.md) section 5 carries the shape
as its own line.

**[T-186](cli-surface.md) is done**, 2026-08-22T03:00Z. 1,050 tests passing, 0
failing. Measured before building and the premise held, with one thing the
entry did not know: the wrong spelling does not only report nothing, it
**writes**. `seed` hash-checks on add, so pointing at the torrent directory
left an empty `album/album/` inside it at full length. `crate::payload::resolve`
is now the shared rule and `seed` takes the resolved root as
`AddOptions::output_folder`, which is also what makes it right for a payload
directory the caller renamed.

**That is the whole work order the last session left.** All four entries closed
or advanced, and three defects found and fixed on the way: T-188 filed,
T-179's test race, and T-013's closing claim corrected.

## Open questions for the operator

None. The session is unattended and nothing was left pending by the last one.
