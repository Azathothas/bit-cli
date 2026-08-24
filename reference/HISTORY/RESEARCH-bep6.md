# BEP 6, the fast extension

Moved out of `RESEARCH.md` on 2026-08-24. `RESEARCH.md` section A keeps the
canonical test vector and the note that aria2 masks differently, because a
reader debugging a mismatch against an aria2 peer still needs it. What is
here is the three implementations the entry was built from.

Nothing here is current. It is kept because its citations, line numbers and
issue references still resolve, and because a later session asking "where did
this come from" should find the source rather than re-derive it.

Closed by: **T-100**.

---

## From 1. `torrent` — anacrolix/torrent

### BEP 6 — fast extension (`bit-cli` T-100)

- `torrent/peer_protocol/protocol.go:45-49` — `Suggest 0x0d`, `HaveAll 0x0e`,
  `HaveNone 0x0f`, `Reject 0x10`, `AllowedFast 0x11`;
  `torrent/peer_protocol/protocol.go:21` `MessageType.FastExtension()` is the range test.
- `torrent/peer_protocol/handshake.go:24` `ExtensionBitFast = 2`;
  `torrent/peer_protocol/handshake.go:101` `SupportsFast()`.
- `torrent/peerconn.go:1047-1054` — receive path: **the `AllowedFast` case must
  `Add` to `peerAllowedFast`**, otherwise every downstream check reads an empty
  bitmap and the feature is silently inert (this was the bug fixed in PR #1052).
- `torrent/peerconn.go:960-985` — on `Unchoke`, requests for allowed-fast pieces
  are *preserved* rather than dropped; the count is logged and counted.
- `torrent/peerconn.go:1947-1949` — chunks received while choked are attributed
  to "allowed fast".
- `torrent/requesting.go:129-134` — when the peer is choking, requests that
  cannot be served are pushed to the back of the order rather than discarded.

**For `bit-cli`:** the message IDs, the reserved bit, the receive-side bitmap and
the "keep requests across a choke when fast is enabled" behaviour are the whole
of the leech-side work. See §A below for a canonical test vector.

---

## From 9. `vortex` — Nehliin/vortex

### BEP 6 allowed-fast set (T-100) — spec-conformant

`vortex/bittorrent/src/peer_comm/peer_connection.rs:89` `generate_fast_set`:
seed is `(ip.to_bits() & 0xffffff00).to_be_bytes()` — **a /24 mask, which is
what BEP 6 specifies** — concatenated with the 20-byte info hash; then
`x = SHA1(x)` repeatedly, taking five big-endian `u32`s per round mod
`num_pieces`, de-duplicating, with a 300-round attempt cap.

`:684-712` is the send side: on the peer's first `Interested`, if
`fast_ext` and we have not yet sent it, `ALLOWED_FAST_SET_SIZE = 6`; if the
torrent has ≤ 6 pieces the whole set is sent instead of running the algorithm.
`:758-790` is the receive side (validates the index, records it, and may start
requesting that piece while choked); `:792` `HaveAll` and the `HaveNone`
counterpart both hard-error when `fast_ext` was not negotiated.

**Compare with `aria2_rust`'s implementation** (§14): aria2 masks class-A/B
addresses to /16 and class-C to /24, mirroring aria2's C++ code rather than
BEP 6. Two widely-deployed clients therefore compute *different* allowed-fast
sets for the same peer. `bit-cli` should follow BEP 6 (/24, as vortex and
anacrolix do) and treat a mismatch as the peer's problem — but should know the
divergence exists before debugging it.

---

## From 14. `aria2_rust` — balovess/aria2_rust

### BEP 6 `computeFastSet` — and a divergence worth knowing

`aria2_rust/aria2-protocol/src/bittorrent/fast_set.rs`. The module doc spells
out the algorithm; `:59` `compute_fast_set(ip, num_pieces, info_hash, set_size)`,
`:121` `resolve_ip_bytes` (**IPv6 support is an extension beyond aria2's C++,
which returns empty for IPv6: SHA-1 the 16-byte address and take the first 4
bytes**), and `:150` `mask_ip`:

```rust
fn mask_ip(mut ip: [u8; 4]) -> [u8; 4] {
    if (ip[0] & 0x80) == 0 || (ip[0] & 0x40) == 0 {
        ip[2] = 0;                 // class A/B: zero the last two octets, i.e. /16
        ip[3] = 0;
    } else {
        ip[3] = 0;                 // class C: zero the last octet only, i.e. /24
    }
    ip
}
```

(the two trailing comments are added here; the rest is verbatim)

BEP 6 specifies `x = 0xFFFFFF00 & ip`, i.e. always /24 — which is what vortex and
anacrolix implement. aria2 mirrors its own C++ instead. **Two mainstream clients
therefore derive different allowed-fast sets for the same peer.** `bit-cli`
should implement the BEP as written and know this is why an aria2 peer's
advertised set may not match its own computation.

---
