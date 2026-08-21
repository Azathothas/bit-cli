# Progress

**Read this first.** It says what the last session did and what is in flight.
It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
The work order: [INDEX.md](INDEX.md), "Start here".

---

## State

- **This session:** 2026-08-21, unattended. Feature work. The planning session
  before it wrote the entries; this one builds them.
- **Baseline re-measured at the start rather than trusted**, and it matched
  what the last session recorded:
  - `cargo test --workspace`: **931 passing, 0 failing**.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
    clean.
  - `cargo fmt --all --check`: clean.
  - `cargo deny check`: advisories, bans, licenses and sources all ok.
  - `gh run list`: run **32479641276** green, against commit `0f75330`.
- **Entries at the start:** 141 items. 63 open, 4 partial, 1 blocked, 63 done,
  10 deferred to Phase C.

## What this session is doing

The work order from [INDEX.md](INDEX.md) "Start here", in that order. Each is
named with its entry, its file, and the code it touches.

1. **[T-171](metainfo.md)**, P2, effort S. `Metainfo::http_seeds` at
   `crates/bit-cli-core/src/torrent/metainfo.rs:306` maps the `httpseeds` key
   through `Value::as_text_list`, which returns empty for `Value::Bytes`
   (`torrent/bencode.rs:305`, `:339`), so a torrent whose `httpseeds` is a bare
   bencoded string yields zero HTTP sources with no error. `url_list` at `:293`
   already branches on both shapes. Factor one helper both call.
2. **[T-166](peers.md)**, P1, effort S. Prove the BEP 10 extension map in
   `crates/bit-cli-core/src/webseed/bridge.rs` keys peer-id-to-handler and
   name-to-our-id as two separate directions, with a peer that numbers its
   extensions differently from the bridge. Assert the bitfield is the first
   message after the handshake.
3. **[T-181](cli-surface.md)**, P1, effort M. Four flags parse and reach no
   code. Warn the way `crates/bit-cli/src/cmd/seed.rs:105` does for
   `--superseed`, build what can be built, and add the test that walks the
   `clap` tree so a fifth cannot appear silently.
4. **[T-167](bep-coverage.md)** then **[T-005](webseed.md)**. BEP 54
   `lt_donthave` first, because it turns T-005's reconnect into one message.
5. **[T-177](disk-io.md)** and **[T-174](metainfo.md)**, effort S each. Two
   missing adversarial fixtures: a piece straddling a file boundary, and a
   piece length that is not a multiple of 16 KiB.

Not started, and both say why in their own entries: [T-163](peers.md) (MSE) and
[T-102](bep-coverage.md) (BEP 55). Both are blocked on `librqbit` seams.

## In progress

Nothing yet. This file is rewritten again at the end of the session with what
actually happened.

Carried from before, unchanged until their own entries are worked:

- **[T-020](peers.md)** stays the only open P0. Half of it is fixed; the
  stranded-socket half and the poisoned-listener finding are open.
- **[T-040](memory.md)** and **[T-090](bench.md)** stay partial. Neither needs
  a longer run; both need the specific measurements named in their entries.

## Open questions for the operator

None blocking. One carried, and it does not block the work above:

- **[T-172](metainfo.md)** asks whether `bit-cli` is strict on read for
  unsorted bencode keys and for trailing bytes after the top-level dictionary.
  The entry recommends strict inside `info` and tolerant outside it, with the
  reasoning, and can be built on that recommendation without an answer. Two
  upstream implementations resolved it in opposite directions, which is why it
  is a decision rather than a defect.
