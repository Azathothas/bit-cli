# fastresume, and closing idle peers

Moved out of `RESEARCH.md` on 2026-08-24. `--fastresume` is in
`man/bit-cli.json` and `scripts/check-fastresume.ps1` is its acceptance;
`--max-handles` bounds the socket accumulation and
`scripts/check-close-wait.ps1` measures it.

Nothing here is current. It is kept because its citations, line numbers and
issue references still resolve, and because a later session asking "where did
this come from" should find the source rather than re-derive it.

Closed by: **T-016 and T-020**.

---

## From 3. `TorrentNG` — snapetech/TorrentNG

### fastresume (T-016, currently *blocked* in `bit-cli`)

`TorrentNG/crates/rt-fastresume/src/state.rs`:

- `:7` `PieceState { Valid, Invalid, Unknown, Missing }`.
- `:23` `FileHint { file_index, size, mtime_secs, inode }` — optimistic hints;
  any mismatch resets the affected pieces to `Unknown`.
- `:33` `PartialPieceState { piece, received_blocks }`.
- `:39` `DurabilityWatermark { barrier_generation, dirty_pieces_since_barrier }`
  — **the interesting idea**: after a crash, only pieces written since the last
  completed storage-sync barrier need rechecking, so recovery is bounded rather
  than a full re-hash.
- `:53` `ImportPolicy { RequireVerification, TrustHints, TrustAll }`.
- The type doc states the invariant plainly: "This is an optimization layer, not
  the source of truth. If integrity cannot be established … the caller must fall
  back to full re-verification."

---

## From 13. `seedchamp` — j-c-m/seedchamp

### Idle-peer closing (T-020 CLOSE_WAIT)

`docs/design.md:226-236` — two timers, both `0` to disable:
`limits.redundant_seed_idle_secs` (default 15) when both sides are complete and
nothing is moving, and `limits.useless_peer_idle_secs` (default 60) when there is
no actual transfer. **HAVE and KeepAlive do not reset them**; interest,
outstanding requests and torrent-level downloading do not reset them either. An
idle close holds that listen address out of outbound dial on the same torrent
for 300 s with no fail backoff. `bit-cli`'s CLOSE_WAIT problem is peers that
close before handshaking; this is the adjacent discipline for peers that connect
and do nothing.

---
