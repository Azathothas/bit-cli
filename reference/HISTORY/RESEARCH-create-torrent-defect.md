# The librqbit create_torrent extra piece hash

Moved out of `RESEARCH.md` on 2026-08-24. The defect is fixed in this
repository's own creation path and `scripts/interop-roundtrip.ps1` proves the
result round trips through `aria2c` and `rqbit`.

Nothing here is current. It is kept because its citations, line numbers and
issue references still resolve, and because a later session asking "where did
this come from" should find the source rather than re-derive it.

Closed by: **T-080**.

---

## From 15. `FluxDown` — zerx-lab/FluxDown

### The librqbit `create_torrent` defect behind `bit-cli` T-080

`bit-cli` T-080 records "librqbit's `create_torrent` writes an extra piece hash"
as **done**. The upstream code is here, so the cause can be stated exactly.
`FluxDown/native/engine/vendor/librqbit/src/create_torrent_file.rs`:

- `:101` `let mut length = 0;` declares the accumulator **outside** the file
  loop, but `:111` `length = 0;` resets it **at the top of every file**, so after
  the loop `length` holds only the last file's length.
- `:145-149` on a full piece: append the hash and reset
  `remaining_piece_length = piece_length`.
- `:153` `if remaining_piece_length > 0 && length > 0 { piece_hashes.extend(…) }`
  — the final flush. When the payload is an exact multiple of the piece length,
  `remaining_piece_length` was just reset to `piece_length` (so `> 0`) and the
  last file is non-empty (so `length > 0`), and an **extra hash of an empty
  piece** is appended. Conversely a trailing zero-length file makes `length == 0`
  and **drops** a legitimate final partial piece.

Two more limitations in the same file, both relevant to `bit-cli create`:
`:56` `choose_piece_length` returns a **hardcoded 2 MiB** regardless of payload
size (`// TODO: make this smarter or smth`), and `:170` writes
`private: false` unconditionally — there is no BEP 27 option on this path.

`FluxDown/native/engine/vendor/librqbit/src/session.rs:865`
`task_tcp_listener` is the accept loop `bit-cli`'s README discusses; the
`Some(Ok((live, checked))) = futs.next(), if !futs.is_empty()` branch inside the
`tokio::select!` is the one it says it removed. Recorded here as the code
location for comparison against whichever librqbit version `bit-cli` pins; no
claim is made about the panic mechanism, which was not reproduced in this pass.

---
