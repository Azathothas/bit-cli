# The retired iroh ruling, RULES.md section 6

Superseded 2026-08-24 by the operator. Kept here because the fact that it was
once ruled the other way is worth seeing, and because two `TODO/` entries were
written under it.

## The text, verbatim, as it stood until 2026-08-24

> - **`iroh` is not being adopted.** BEP 55 needs no NAT library; the blocker is
>   librqbit's `PeerConnectionHandler`. [T-102](../../TODO/bep-coverage.md)
>   carries the whole flow inline and the design rationale from
>   `torrent/NOTES.md:15-31`. Do not reach for a NAT crate.

## Why it was written

It came out of the 2026-08-21 corpus pass, which was asked directly whether
`iroh` should be used for hole punching to support BEP 55. The answer rested on
two findings, both of which still hold:

- BEP 55 is three messages over connections that already exist, and
  `fx-torrent/src/peer/extension/holepunch.rs` implements the whole thing in
  678 lines with no dependency beyond bencode.
- What blocks it in this tree is `librqbit`'s ownership of the peer connection,
  not the absence of a traversal library.

## What the operator changed, and it is not those two findings

The ruling of 2026-08-24, in the operator's words:

> bit-cli will be BEP and RFC compliant, but it will not limit itself to BEPs
> and RFCs that were written long ago. In the real world, NATs and heavily
> censored networks are everywhere.

So the sentence that is retired is **"Do not reach for a NAT crate"**, which
closed a question rather than answering it. NAT crates are candidates now, and
a mechanism that goes beyond the BEPs is allowed as long as it degrades to
plain BEP 55 and plain TCP or uTP, and as long as the entry says what a
standards-only peer sees.

## Where the question lives now

[T-238](../../TODO/peers.md) carries the recommendation, the measured cost of
`iroh` 1.0.3, and the per-NAT-shape table. Its recommendation for `iroh`
specifically is still no, and the reason is different from the one above: an
`iroh` peer is an ed25519 node id, a BitTorrent peer is an `IP:port`, and there
is nowhere in BEP 5, BEP 11 or a tracker response to publish a node id that
another client would understand.

[T-102](../../TODO/bep-coverage.md) keeps its inline flow. Nothing found in the
2026-08-24 pass says the inline flow is the wrong shape.
