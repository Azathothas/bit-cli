# Reference corpus

What was read for `bit-cli`, why, what was learned, and what must not be
copied. Untracked working material, like `PROMPT.md`.

**On 2026-08-21 the four trees whose licence is incompatible with MIT were
deleted, and their sections were removed from this file with them.** Two
AGPL-3.0, one GPL-3.0-or-later, and one with no `LICENSE` at all. Nothing was
taken from any of them, and every `TODO/` entry that used to cite one now
carries what it needed inline, so no tracked file depends on this document or
on anything under `reference/`. The record is
[T-122](../TODO/reference-map.md).

What is left is permissive or is not code:

| Tree | Licence | What is allowed |
| --- | --- | --- |
| `intermodal` | CC0-1.0 | **Copy and adapt directly.** The only one. |
| `fx-torrent` | Apache-2.0 | Permissive. Copying needs the NOTICE and attribution terms; nothing has been copied. Safe to depend on. |
| `rqbit` corpus | n/a, data | Issue and PR JSON, not code. |
| `aria2-next` | documentation | Quote sparingly, with attribution. |

`bit-cli` is MIT and its `cargo deny` gate refuses copyleft dependencies
outright, proven by `scripts/check-licence-gate.ps1` against a probe crate
([T-120, T-121](../TODO/licensing.md)). Every finding below is a **description
of a technique** with a citation to check it against, never a snippet to paste.

---
### fx-torrent — the closest thing to a peer, and Apache-2.0

157 Rust files, ~1.2 MB of source. Apache-2.0, so it is the one modern
BitTorrent tree in this corpus that could legally be copied from or depended
on. It is a full client: DHT, uTP, trackers, a real piece picker, extensions.

**The most valuable file in the whole corpus for us:
`fx-torrent/src/peer/extension/holepunch.rs`, 678 lines. It is a complete
BEP 55 implementation.** See the BEP 55 section below, which is written up
separately because it answers a question that was asked directly.

**What to learn.**

*A real piece picker*, `fx-torrent/src/piece_picker/strategy/`, four strategies
behind one bitflag set:

- `rarest_first.rs:36-39` sorts by a per-piece `availability` counter. That
  counter is the thing `librqbit` 9.0.0 does not have anywhere, which is the
  finding [T-032](../TODO/performance.md) is built on: `librqbit`'s picker is
  not rarest-first, it is first-piece-then-last-piece-then-ascending, and the
  flag that named it `rarest-first` was naming behaviour that does not exist.
- `sequential.rs:23-28` is the trivial version of what `bit-cli` now does with
  a held `FileStream`: sort by piece index, take the queue length.
- `rarest_first.rs:22-26` returns an empty vector when `Priority` is set, and
  `sequential.rs:19-21` when `Sequential` is not. Strategies compose by
  abstaining rather than by a match, which is a clean shape.
- Their issue **#20**, closed, "Implement piece picker logic from libtorrent",
  is where this came from.

*Extension plumbing.* `holepunch.rs:150-152` shows the pattern: an extension is
a struct with a `NAME`, an `on_message(payload, peer)`, and a lookup of the
remote's negotiated message id via `peer.find_remote_extension_number(NAME)`.

**What to avoid, and it is a lot in one file.**

`fx-torrent/src/peer/webseed/http.rs::request_piece`, lines 195-294, is the
same job as `bit_cli_core::webseed::fetch`. Reading it twice found six
defects, and two of them are the root cause of their own open bug:

1. **Line 212**: `let mut buffer = vec![0u8; len]` where `len = blocks.len()`.
   The buffer is sized by the **number of blocks**, not the number of bytes.
   Eight 16 KiB blocks allocate eight bytes.
2. **Lines 217-227**: the loop re-reads `self.torrent.file(&file_index)` from
   the same `file_index` every iteration and never advances it. A piece
   spanning two files re-requests from the first file forever. **This is the
   root cause of their open issue #98**, "Pieces are written entirely into the
   file that they start in, even if they span multiple files", which the
   reporter said they had not been able to track down.
3. **Line 231**: `min(piece.length, file.len())` is the wrong available length
   when the piece starts part way into the file. It should be
   `file.len() - range_start`.
4. **Lines 232-233**: `Range: bytes={start}-{start + request_len}`. HTTP byte
   ranges are **inclusive**, so this asks for one byte too many on every
   request. `bit-cli` writes `start + length - 1`
   (`crates/bit-cli-core/src/webseed/fetch.rs:1053`).
5. **Line 243**: `bytes_in.inc_by(response.content_length())` runs before the
   status is checked, so an error page counts as payload in the metrics.
6. **Line 244**: `if response.status().is_success()` accepts **200** as
   readily as 206. A server that ignores `Range` returns 200 with the whole
   entity, and reading that as if it were the requested range serves wrong
   bytes at every offset. `bit-cli` refuses that case explicitly in
   `fetch.rs::check_status`, which was written before this file was read and
   is now independently justified by it.

`bit-cli` does not have their #98. The `multi_file` fixture in
`crates/bit-cli/src/test_support.rs:35-49` is 1500 + 500 bytes at a 1024 byte
piece length, so **piece 1 straddles the file boundary by construction**, and
every download test asserts both files land byte-identical.

**Issues worth knowing** (30 total):

- **#98 open**, the cross-file piece bug above.
- **#101 open**, "Create new torrent functionality": fx-torrent cannot create
  torrents at all. `bit-cli` can, and round-trips through `aria2c` and `rqbit`.
- **#63, #59, #54, #42** closed: BEP 24 external IP, BEP 7 IPv6 tracker
  extension, BEP 14 LSD, BEP 43 read-only DHT. A useful checklist to read
  against [`TODO/bep-coverage.md`](../TODO/bep-coverage.md).
- **#49** closed, "Make outgoing connections sequential", and **#24**,
  "Peer connections not correctly being bursted": both are the connection-rate
  shape that [T-020](../TODO/peers.md) is about.

## BEP 55 holepunch, and the iroh question

**This section was written on 2026-08-21 under a ruling the operator reversed
on 2026-08-24.** What follows is reconciled against the new one. The mechanism
below is unchanged and still correct; the conclusion is narrower than it was.

**Asked, originally:** whether `iroh` should be used for hole punching to
support BEP 55.

**Answer: BEP 55 does not need it, and that is no longer the whole question.**
BEP 55 is a three message extension over connections that already exist, and
`fx-torrent` implements the whole thing in one 678 line file with no dependency
beyond bencode. That part stands.

What changed is the question around it. `TODO/RULES.md` section 6 used to end
"do not reach for a NAT crate", which closed the subject. It now says
compliance is the floor rather than the ceiling, and a mechanism beyond the
BEPs is allowed when it degrades to plain BEP 55 and plain TCP or uTP and when
the entry says what a standards-only peer sees. The retired paragraph is in
[`HISTORY/RULES-section-6-iroh.md`](HISTORY/RULES-section-6-iroh.md).

**How BEP 55 works**, from `fx-torrent/src/peer/extension/holepunch.rs`:

The extension name is `ut_holepunch` (`:150`). The message is a bencode dict of
`msg_type`, `addr_type`, `addr`, `port`, and an optional `err_code` (`:13-25`).
Three message types (`:55-58`): `rendezvous = 0`, `connect = 1`, `error = 2`.
Four error codes (`:110-118`): `NoSuchPeer = 1`, `NotConnected = 2`,
`NoSupport = 3`, `NoSelf = 4`.

The flow:

1. A direct dial to a peer fails through every dialer
   (`operation/connect_peers.rs:356-359`). That, and only that, is the trigger:
   `if holepunch_supported { holepunch_sender.send(peer_addr) }`.
2. We pick an **already-connected peer** as the rendezvous and send it
   `rendezvous` carrying the unreachable target's address
   (`holepunch.rs:186-240`).
3. That peer looks the target up in its own peer set, checks **both** sides
   advertise `ut_holepunch`, and sends `connect` to **both** of them: to the
   target carrying our address, to us carrying the target's
   (`holepunch.rs:243-306`).
4. Each side, on `connect`, adds the address and dials it
   (`holepunch.rs:309-329`). The two outbound SYNs crossing in flight are what
   opens both NATs.

Timeouts, `operation/connect_peers.rs:22` and `:412-452`: 200 ms to ask whether
a peer supports the extension, 250 ms to start the punch, `HOLEPUNCH_TIMEOUT`
of 6 s for the whole exchange.

**Where BEP 55 stops, which the original section did not say.** It works
because a peer that can see both ends reports an `address:port` that is already
known. On a **symmetric NAT** the external port is allocated per destination,
so the port the rendezvous peer saw is not the port the target will see, and on
**carrier grade NAT** there is no gateway to ask for a mapping either. Those
are the two shapes where something beyond BEP 55 earns its cost, and
`RESEARCH.md` entry 31 is the only mechanism in this corpus that addresses the
first without a relay.

**Why `iroh` is still not the answer, on a better argument than before.** The
original text said adopting it "would make `bit-cli` reachable to other
`bit-cli` instances, which is a private network wearing a BitTorrent costume".
That is right and it is now measured rather than asserted. An `iroh` peer is an
`EndpointId`, an ed25519 public key (`RESEARCH.md` entry 32,
`iroh-fm/crates/server/src/iroh_rpc.rs:47-58`); a BitTorrent peer is an
`IP:port` from a tracker, the DHT or PEX; and there is nowhere in BEP 5, BEP 11
or a tracker response to publish a node id another client would understand. The
cost, resolved on 2026-08-24: `iroh` 1.0.3 is `MIT OR Apache-2.0` with 43
direct dependencies, and adding it brings **113 crates this tree does not
already have**, replacing nothing.

What did change is that the alternatives are now on the table. The ladder from
`RESEARCH.md` entry 30, port mapping through NAT-PMP and PCP, and the port
prediction of entry 31 are all candidates, and
[T-238](../TODO/peers.md) carries the recommendation, the per-NAT-shape table
and the three questions the operator is being asked to rule on.

**What it would cost here, and why T-102 is still blocked.** The wire format is
expressible today: `librqbit-peer-protocol` 9.0.0 has
`ExtendedMessage::Dyn(u8, BencodeValue)` (`extended/mod.rs:47`), an escape hatch
for arbitrary extended messages. What is missing is the same thing
[T-002](../TODO/webseed.md) priced: `librqbit` owns the peer connection.
`PeerConnectionHandler::on_extended_handshake` and
`update_my_extended_handshake` (`librqbit-9.0.0/src/peer_connection.rs:41,49`)
are the hooks that would have to advertise `ut_holepunch` and route its
messages out, and the trait is implemented internally by the torrent state
rather than by anything a dependent crate can supply.

That seam is now inside `vendor/`, so it is this repository's to change.
[T-102](../TODO/bep-coverage.md) is the entry.

---

## The 2026-08-24 trees, and what to do with them

Seventeen trees on three topics. This section is written to be **used**: a
session implementing [T-234](../TODO/peers.md), [T-236](../TODO/peers.md) or
[T-238](../TODO/peers.md) should be able to work from what is below without
opening any of them again.

### Client identity: the numbers, not the names

**A peer id prefix is eight bytes and each version component is one
character**, `0` to `9` then `A` to `Z`. qBittorrent 3.3.13 is `-qB33D0-` and
3.3.16 is `-qB33G0-`
(`joal/resources/clients/qbittorrent-3.3.13.client`, `qbittorrent-3.3.16.client`).
Decimal concatenation gives `-qB33130-`, nine bytes, and that is what
`joal/scripts/bittorrent-client-update-detector/qbittorrent_analyzer.sh:445`
produces. `scripts/make-client-profile.ps1` has the table-driven version and a
self-test over the whole alphabet.

**The fourth character is not padding for three of the five clients.** Deluge
is `{major}{minor}{patch}s`, uTorrent is `...S`, and BitTorrent is
`{major_hex}{minor_hex}{patch}S` (`Seedr/clients/README.md:33-41`).

**Transmission's suffix carries a checksum.** Eleven characters from
`0123456789abcdefghijklmnopqrstuvwxyz` at `byte % 36`, then a twelfth chosen so
the whole suffix sums to a multiple of 36:

```
val = total % 36 != 0 ? 36 - (total % 36) : 0
```

`libtransmission/session.cc:205` at tag `4.1.0`, and
`joal/.../RandomPoolWithChecksumPeerIdAlgorithm.java:98`.
`DOAL/announce/client_emulator_test.go:312` asserts the invariant rather than
the output, which is the test to write:

```go
if total%36 != 0 { t.Fatalf("Transmission suffix checksum = %d mod 36, want 0", total%36) }
```

`Seedr/src/core/client-emulator/generators.ts:74` uses `sum % base` instead and
is right about one draw in eighteen. `fake-torrent-client/src/algorithm.rs:45`
computes no checksum at all.

**qBittorrent's `key` is eight upper case hex digits, zero padded**, from
libtorrent `src/http_tracker_connection.cpp:138` at `v2.0.11`:

```
"&key=%08X"
```

A real one starts with `0` one time in sixteen. **Four separate projects that
exist to emulate clients cannot produce one**, each by a different route:
`joal/.../HashNoLeadingZeroKeyAlgorithm.java:24-33` strips leading zeros,
`Seedr/.../generators.ts:7-13` rejects and regenerates,
`DOAL/announce/client_emulator.go:258-277` replaces the first character, and
`fake-torrent-client/src/algorithm.rs:17-32` skips index 0 at every position on
a truncated alphabet. That is the whole argument for deriving a profile from
the client rather than copying one.

**The query parameter order is per client and is checkable.** qBittorrent 5.2.3
(`joal/resources/clients/qbittorrent-5.2.3.client`):

```
info_hash peer_id port uploaded downloaded left corrupt key event numwant compact no_peer_id supportcrypto redundant
```

`bit-cli` today, measured by `scripts/check-announce.ps1` on 2026-08-24:

```
info_hash peer_id event port uploaded downloaded left compact no_peer_id key
```

No client in joal's ninety-four profiles puts `event` third, and none omits
`numwant`.

**The peer wire is a second identity and no profile format carries it.**
`DOAL/peerwire/server.go:337` is the only place in this group that sets it:

```go
var baseReservedBytes = [8]byte{0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x04}
```

and `DOAL/peerwire/extensions.go:29` hard-codes `reqq` at 250 for every
emulated client. The bencode key order in an extension handshake is **not** a
free variable, because bencode requires keys sorted; the key set and the values
are.

**The verification rule to adopt**, from
`RatioForge/docs/client-profile-audit-2026-08-11.md:22`: a profile is added
only when the version, the peer id construction and the User-Agent come from an
official tagged source release. It also records what it refused and why, which
is the half that keeps a profile set honest.

### Announce fidelity: two shapes of rejection

`RatioTracker/ratiotracker.py:375-390`: a tracker refusal is a non-200 status
**or** a 200 whose bencode carries `failure reason`. A check that reads only
the status calls the second one a success.

`ratiotracker.py:224-225` percent-encodes `info_hash` and `peer_id` with
`safe=""`, nothing exempt, because both are twenty arbitrary bytes.

### NAT traversal: the ladder, and where BEP 55 stops

**The ladder**, `dig-nat/src/strategy.rs:69-71`, and the property that makes it
worth copying is that the order is enforced rather than requested:

```rust
// Guarantee direct-first, relay-last regardless of how the caller ordered `methods`.
let mut ordered = methods;
ordered.sort_by_key(|m| m.kind().rank());
```

Direct, UPnP/IGD, NAT-PMP, PCP, relay-coordinated hole punch, relayed
transport. Each bounded by its own timeout at `:104-110`, and an all-fail
returns **every method's reason in attempt order**.

`dig-nat/Cargo.toml:7-14` argues for implementing NAT-PMP, PCP and STUN
directly, because they are fixed-layout datagrams that can be unit-tested with
no network, and taking only UPnP/IGD as a dependency.

**BEP 55 fails on symmetric and carrier grade NAT**, because the port a
rendezvous peer observed is not the port the target will see. The only
mechanism in the corpus that addresses the first without a relay is port
prediction, `tcp-transfer-ice/README.md:27-38`: probe, model
`delta = public_port - local_port`, predict, widen by the observed deviation,
and for a progressing allocator shift forward by an estimated rate. Its
required socket options are at `tcp-transfer-ice/src/hole_punch.rs:106-111`,
`set_reuse_address` and on Unix `set_reuse_port`, without which there is no
simultaneous open.

**eD2k solved the same problem in 2002 and named two things BEP 55 does not**,
`ed2k-server/src/proto/opcodes.rs:25-45`: an explicit initiator role byte
(`0 = you initiate, 1 = you wait`) and an enumerated failure reason. Its server
"only exchanges small address packets, it never relays file data".

**Report the path, not just the peer.**
`iroh-fm/crates/server/src/iroh_rpc.rs:411-437` labels each connection
`relay`, `direct` or `custom` with its round trip time, and `:241-247` logs it
only when it changes. `bit-cli` says a peer is connected and cannot say whether
the path is direct, which after a hole punch is the whole question.

**Nothing in thirty-nine trees classifies a NAT.** `dig-nat/src/stun.rs` and
`Hollow/crates/p2p-connection/src/stun.rs` both discover a reflexive address
and stop. [T-239](../TODO/peers.md) is therefore new work rather than a port,
and its four-exchange classification is written out there.

**iroh, priced.** `iroh` 1.0.3, `MIT OR Apache-2.0`, 43 direct dependencies,
and **113 crates this tree does not already have** against a current 302,
replacing nothing. Resolved 2026-08-24 in a throwaway crate outside the tree.
The integration surface is small
(`iroh-fm/crates/server/src/iroh_rpc.rs:47-58` and `:251-254`) and the
addressing is the problem: an `EndpointId` is an ed25519 public key and there
is nowhere in BEP 5, BEP 11 or a tracker response to publish one.

### Two bounds worth taking

`demagnetize-rs/src/consts.rs:15`, `MAX_INFO_LENGTH = 20 << 20`. A cap on the
metadata length a peer **declares**, before allocating.
[T-212](../TODO/memory.md) is open on exactly this and `dht-spider` caps at
about 16 MiB, so two independent implementations bound it and `librqbit` does
not. `dht-crawler/src/metadata.rs:274,394` adds the operational half: count the
refusals, so a bound that is never hit and one that is hit constantly can be
told apart.

`demagnetize-rs/src/consts.rs:5`, `LEFT = 65535`: what to send as `left` in an
announce for a magnet, before any metadata exists. `aquatic` PR 254 records
clients that send `-1`. There are at least three answers in the wild.

### What was read and refused, so it is not re-read

- `Hollow`: STUN plus QUIC with a Steam identity. `dig-nat` covers the same
  ground with tests.
- `NetDrop`: its workspace declares `netdrop-core` and the repository does not
  contain it, so the traversal code is not there at all. Its `LICENSE` is
  GPL-3.0 and its manifest says MIT.
- `gaia`: a design document with no implementation, which the tree confirms.
  Two of its three mechanisms are already filed here.
- `TheDancingDeveloper-org`: 33 repositories, four copyleft, and twelve that
  are `librqbit` renamed and declared MIT. Nothing to take.

---

## The earlier corpus, consolidated

These were cloned before 2026-08-21 and their notes lived in
`TODO/reference-map.md`. Merged here; that file now keeps the licence
determinations and points at this one.

### aria2-next — documentation, the parity source

```
aria2-next/aria2-next.rst   4,805 lines
```

The manual, and the source for `PROMPT.md` section 9's parity checklist.
**Premise confirmed:** 207 `.. option::` directives and **zero** web seed
options. The only occurrence of "Web-seeding" is line 2595, inside
`aria2.addTorrent`'s RPC documentation. That gap is why this project exists,
and the claim in `README.md`'s "Why" section is checkable against that line.

### rqbit — corpus, data rather than code

```
rqbit/issues.json   262 issues, 91 open, 171 closed, fetched 2026-08-19
rqbit/prs.json      346 pull requests with their changed file lists
```

The input to the triage that produced every `T-0xx` item in `TODO/`. Data.
`prs.json` is the fastest way to find which upstream file a subsystem lives in.

The `librqbit` **source** is not here. It is a crates.io dependency, and every
claim about "the pinned 9.0.0" in `TODO/` was verified against the registry
cache at
`~/.cargo/registry/src/index.crates.io-*/librqbit-9.0.0/`.

### intermodal — CC0-1.0, the one tree that may be copied

Already adapted into `bit-cli`:

| Their file | Ours |
| --- | --- |
| `intermodal/src/env.rs` (315 lines) | `crates/bit-cli/src/env.rs`. Injects args, cwd, and the three streams rather than reading globals. This is what makes rule 0.11 headless parity testable. |
| `intermodal/src/bytes.rs` | `crates/bit-cli-core/src/units.rs`. SI and binary size parsing. |
| `intermodal/src/table.rs` | `crates/bit-cli/src/output.rs`. Aligned output. |

Read and used as the basis, not copied verbatim:
`subcommand/torrent/create.rs` (3,196 lines, creation and the `--allow <LINT>`
model), `piece_length.rs` (the heuristic), `verify.rs` (803 lines),
`show.rs` (636 lines), `magnet_link.rs` (485 lines), `metainfo.rs` (460
lines), `walker.rs` (traversal honouring `.gitignore`, `.ignore`, and
`.git/info/exclude`, relevant to `create --ignore`).

## What the whole corpus adds up to

**Thirty-nine trees now**, twenty-two from 2026-08-21 and seventeen from
2026-08-24, and the patterns below held across every pass. The sentence this
paragraph used to open with, "four trees kept and four deleted", was about the
first four-tree corpus and is in `reference-map.md`'s own history now.

**Nobody else has per-scope web seeds.** `aria2-next` has no web seed option at
all, `fx-torrent` reads `url_list` and `http_seeds` from the metainfo
(`operation/connect_peers.rs:126-145`) and offers no way to add one at runtime,
and the one tree that had a virtual-peer abstraction kept it as an internal
mechanism rather than a surface. The addressing model in `README.md` is the
thing `bit-cli` has that none of them do, and this corpus is the evidence for
that claim.

**Every one of them that fetches over HTTP gets a range detail wrong or
unchecked**, and `fx-torrent`'s six defects above are the worked example. The
per-piece verification `bit-cli` does by default
([T-136](../TODO/multi-source.md) is what states the guarantee) is not
paranoia; it is the thing that turns those defects into a refused source
instead of a corrupt payload.

**The bugs that survive longest are the ones with a plausible wrong answer.**
fx-torrent #98 is a file that downloads and does not play. pluto's float
`math.log` gives a merkle root that is wrong rather than missing. `bit-cli`'s
own T-145 reported a Mac as a Linux box with no memory, T-152 reported a
benchmark whose time series was silently empty, and T-141 spent a session
blaming a flag that worked. Prefer the check that fails loudly.
