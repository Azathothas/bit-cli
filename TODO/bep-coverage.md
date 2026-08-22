# BEP coverage

Sixty-six issues in the corpus mention a BEP or a protocol feature. This file
tracks what `bit-cli` speaks today and what it does not.

Implemented means there is a test. Inherited means `librqbit` provides it and
`bit-cli` has not verified it independently.

| BEP | What | Status |
| --- | --- | --- |
| 3  | The BitTorrent protocol | inherited |
| 5  | DHT | inherited, not reported (T-052) |
| 6  | Fast extension | **partial** (T-100): the allowed-fast derivation is in `fast_set.rs` and `bench swarm` reads the five messages; nothing sends one, blocked on `librqbit` |
| 7  | IPv6 tracker extension | implemented in `tracker.rs` |
| 9  | Metadata from peers (magnet) | inherited |
| 10 | Extension protocol | implemented in the bridge |
| 11 | PEX | inherited; `--no-pex` reaches nothing (T-181) |
| 12 | Multitracker metadata | implemented in `create`, `edit`, `trackers` |
| 14 | Local service discovery | inherited |
| 15 | UDP tracker protocol | implemented in `tracker.rs` |
| 16 | Superseeding | not implemented (T-082) |
| 17 | HTTP seeding, Hoffman style | implemented in `fetch.rs`, style declared not detected (T-004) |
| 19 | HTTP/FTP seeding, GetRight style | implemented, the headline feature |
| 20 | Peer id conventions | implemented |
| 21 | Extension for partial seeds | implemented in the bridge |
| 23 | Compact peer lists | implemented in `tracker.rs` |
| 27 | Private torrents | implemented in `create`, `edit` |
| 29 | uTP | **not reachable**, no flag enables it (T-101) |
| 33 | DHT scrape | not implemented (T-169) |
| 39 | Updating torrents via feed URL | implemented in `create`, `edit` |
| 44 | DHT store, mutable items | not implemented (T-170) |
| 47 | Padding files | **read only**: parsed and skipped, `create` does not emit them (T-081) |
| 48 | Tracker scrape | implemented in `tracker.rs`, BEP 48 URL convention only (T-065) |
| 51 | DHT infohash indexing | not implemented (T-169) |
| 52 | BitTorrent v2 | not implemented (T-081, T-134) |
| 53 | Magnet file selection, `so=` | implemented in `torrent/magnet.rs` |
| 54 | `lt_donthave` | not implemented (T-167) |
| 55 | Holepunch | not implemented (T-102) |
| MSE/PE | Peer encryption | not implemented (T-163) |
| WebTorrent | WebRTC peers, WSS trackers | not implemented (T-168) |

**Five rows changed on 2026-08-21 and each was wrong in the same direction:
the table described intent rather than the tree.**

- **BEP 29 said "inherited, off by default".** There is no uTP in `bit-cli` at
  all. `ListenerOptions::mode` is never set, so the session stays `TcpOnly`,
  and no flag changes that. `librqbit-utp` 0.7.0 appears in `cargo tree`
  because `librqbit` depends on it, which is not the same thing as a
  capability a user can turn on. "Off by default" reads as a switch; there is
  no switch. See [T-101](#t-101-utp-is-available-but-untested).
- **BEP 47 said "not implemented".** The read side is implemented and tested.
  `torrent/metainfo.rs:107` parses the `attr` key, `:116` `InfoFile::is_padding`
  is the predicate, `storage.rs:728` and `:870` never open a padding file
  because it is alignment rather than data, `cmd/files.rs:176` reports it, and
  `torrent/metainfo.rs:825` `padding_files_are_recognised` is the test. What is
  missing is the **write** side: `create` emits no padding files, which is a
  clause of [T-081](create-seed.md).
- **BEP 53 was absent.** `torrent/magnet.rs:39` and `:211` parse the `so=`
  index-range file selection out of a magnet.
- **BEP 7 was absent.** `tracker.rs:493` reads the `peers6` key at 18 bytes
  per entry beside the 6-byte `peers`, with a test at `:873`
  `ipv6_peers_come_back_bracketed`. Worth naming rather than leaving implicit,
  because [T-022](peers.md) and [T-023](peers.md) are both about IPv6 and
  neither could point at the one piece of it that works.
- **BEP 33, 44, 51 and 54, MSE/PE and WebTorrent were absent.** Six gaps the
  corpus named that no row admitted to. They have entries now rather than
  silence.

The lesson is the one [T-032](performance.md) and [T-141](webseed.md) wrote
down: a table is a claim, and a claim needs a symbol. Every row above now
either names one or names the entry that closes it.

---

### T-100 BEP 6 fast extension is not implemented

Source:      https://github.com/ikatson/rqbit/issues/584 (open)
Category:    bep
Priority:    P2
Effort:      L
Status:      partial

Problem:     No `have all`, `have none`, `suggest piece`, `reject request`, or
             `allowed fast`.
Relevance:   Two parts matter here. `have all` and `have none` replace a
             bitfield with two bytes, which matters on a torrent with a million
             pieces. `reject request` is what lets a partial seed refuse a
             piece cleanly instead of timing out, which is exactly what the web
             seed bridge needs when a source turns out not to hold something it
             announced.
Approach:    The bridge is the natural place to start, because it is
             `bit-cli`'s own peer implementation: set the fast extension
             reserved bit, send `have all` when the scope covers everything and
             a bitfield otherwise, and answer an out-of-scope request with
             `reject request` rather than dropping the connection. The session
             side needs `librqbit`.
Acceptance:  The bridge negotiates BEP 6 with a session that supports it, sends
             `have all` for a complete source, and rejects an out-of-scope
             request without dropping the connection. Covered by an e2e test.

**The corpus supplies the algorithm, a conformance vector, and a warning.**

`vortex/bittorrent/src/peer_comm/peer_connection.rs:89` `generate_fast_set` is
the spec-conformant allowed-fast set: seed is
`(ip.to_bits() & 0xffffff00).to_be_bytes()`, a **/24 mask, which is what BEP 6
specifies**, concatenated with the 20-byte info hash, then `x = SHA1(x)`
repeatedly taking five big-endian `u32`s per round mod `num_pieces`,
de-duplicated, with a 300-round attempt cap. `:684-712` is the send side:
`ALLOWED_FAST_SET_SIZE = 6`, sent on the peer's first `Interested`, and a
torrent of six pieces or fewer gets the whole set rather than the algorithm.
`:758-790` is the receive side, and `:792` hard-errors on `HaveAll` or
`HaveNone` when `fast_ext` was never negotiated.

The receive side is where this goes wrong quietly.
`torrent/peerconn.go:1047-1054` carries the fix from anacrolix
[PR 1052](https://github.com/anacrolix/torrent/pull/1052): **the `AllowedFast`
case must `Add` to the peer's bitmap**, or every downstream check reads an
empty set and the feature is inert while appearing to work.
`torrent/peerconn.go:960-985` is the behaviour that makes it worth having: on
`Unchoke`, requests for allowed-fast pieces are *preserved* rather than
dropped.

**Ship this vector as a unit test.** From that same PR, reproducible against
both implementations named above:

```
ip        = 80.4.4.200
info_hash = AA AA ... AA  (20 bytes)
numPieces = 1313
k         = 7
=> [1059, 431, 808, 1217, 287, 376, 1188]
```

**Expect an aria2 peer to disagree, and do not treat that as a bug here.**
`aria2_rust/aria2-protocol/src/bittorrent/fast_set.rs:150` `mask_ip` mirrors
aria2's own C++ rather than the BEP: class A and B addresses are masked to /16
and class C to /24. So two widely deployed clients derive **different**
allowed-fast sets for the same peer. Implement the BEP as written, as vortex
and anacrolix do, and know the divergence exists before debugging it.

The receive half alone is worth having before the send half:
seedchamp [PR 7](https://github.com/j-c-m/seedchamp/pull/7) honours `Suggest`
in the picker without ever sending one, and `seedchamp/docs/design.md:152-160`
records that as a deliberate posture rather than an unfinished one.

**Partial, 2026-08-22. The Approach names the wrong half as the reachable
one.**

It says "the bridge is the natural place to start, because it is `bit-cli`'s
own peer implementation" and "the session side needs `librqbit`". The bridge
half is the one that cannot be done, and for a reason the Approach does not
consider: **the bridge's only counterparty is the session in the same
process.** It dials this run's own listen port and nothing else, so whatever it
advertises is answered by `librqbit`, and `librqbit` 9.0.0 has no BEP 6 at all.

Measured rather than assumed. `librqbit-peer-protocol` 9.0.0 `lib.rs:40-49`
declares message ids 0 through 8 and 20, and nothing in between: there is no
`HaveAll`, `HaveNone`, `SuggestPiece`, `RejectRequest` or `AllowedFast` variant
to construct, so the bridge could not send one without hand-rolling the wire
format for a peer that would fail to parse it. `Handshake::new` at `lib.rs:480`
sets `1 << 20` for the extension protocol and no other reserved bit, so the
session never offers the fast extension and never accepts an offer of it.
Zero hits for any of the five names in either crate:

```bash
grep -rniE "haveall|havenone|suggestpiece|rejectrequest|allowedfast|fast_ext"   ~/.cargo/registry/src/*/librqbit-9.0.0/src   ~/.cargo/registry/src/*/librqbit-peer-protocol-9.0.0/src
```

So this splits into three parts and only one is blocked.

**Part one, the derivation. Done.** `crates/bit-cli-core/src/fast_set.rs`
implements the allowed-fast set and **reproduces the conformance vector above
exactly**: `80.4.4.200`, twenty `0xAA` bytes, 1313 pieces, k = 7 gives
`[1059, 431, 808, 1217, 287, 376, 1188]`, which is
`the_canonical_vector_reproduces`. This is the part that is hard to get right
and impossible to check later without a reference, so it is written down while
the reference is in hand.

**The aria2 divergence is implemented rather than described.** `Mask::Bep6`
keeps three octets, `Mask::Aria2` keeps two below 192.0.0.0, and
`aria2_derives_a_different_set_below_192` asserts they disagree for the vector's
own address. A warning in prose is something to remember; a `Mask` a
measurement can name is something that reports which of the two the other end
used. `Mask::is_ambiguous` is the third answer: the two rules agree at and
above 192.0.0.0, and **loopback is not an exception** because 127.x is class A
under aria2's rule and agrees too, so a measurement taken over loopback cannot
tell them apart and says `ambiguous` rather than claiming a pass.

**Part two, the receive and measure side. Done, in `bench swarm`.** Every
synthetic peer now sets the fast extension bit, reports whether the target set
it back, counts `have all`, `have none`, `suggest` and `reject request`,
collects the offered allowed-fast set, and says which derivation it matches.
`bench swarm` is the right home rather than the bridge: it is the one part of
this tree that talks to somebody else's client, and `aria2c` 1.37.0 is
installed on this machine, so the divergence has a live counterparty to be
measured against.

It reports the blocker from the wire rather than from the source, which is the
better evidence of the two. `bench/swarm-20260822T062909627Z.json`:

| case | peers handshaked | `fast_negotiated` | `received` |
| --- | --- | --- | --- |
| `leech_1` | 1 | **0** | 8,388,608 |
| `leech_4` | 4 | **0** | 33,554,432 |
| `leech_16` | 16 | **0** | 134,217,728 |

The synthetic peers offered the bit on every one of those connections and
`bit-cli seed` declined it every time, which is `librqbit` saying it has no BEP
6 rather than this entry reading that off its source. Leeching is unchanged by
the offer: the same bytes as the run before the change, and `verdict: pass`.

`check-swarm.ps1` records `fast_negotiated` for exactly this reason and does
not judge it. Zero is what `librqbit` gives, so a script that failed on
anything else would be asserting the blocker rather than measuring it, and the
number that matters is the day it stops being zero.

The leecher acts on what it now understands, which is the difference between
reading the messages and honouring them. `have all` and `have none` stand in
for a bitfield, so a peer that negotiated the extension against a target that
sends two bytes instead of one no longer sees an empty bitfield and requests
nothing. A `reject request` clears the request from the window, which is the
stall BEP 6 exists to prevent and which anacrolix's `peerconn.go:960-985`
records the other side of.

**A defect this found, and it was in this tree.** `bench swarm` handed every
frame to `librqbit_peer_protocol::Message::deserialize`, which knows none of
the five ids, so **a target that spoke BEP 6 was reported as
`ended: "protocol"`**, a broken peer. Nothing had noticed because the only
target ever pointed at was `librqbit`, which never sends one.
`every_bep6_message_is_recognised_rather_than_called_a_protocol_error` is the
regression test.

**Part three, the send side. Blocked, upstream, and this is what keeps the
entry open.** The Acceptance says "the bridge negotiates BEP 6 with a session
that supports it", and no such session exists here. What would unblock it is
`librqbit` gaining the five message variants and the reserved bit, at
`librqbit-peer-protocol` `lib.rs:40-49` and `lib.rs:480`. The same blocker as
[T-102](#t-102-bep-55-holepunch-is-not-implemented) and
[T-167](#t-167-bep-54-lt_donthave-is-not-implemented), and named the same way.

Not blocked and not done: measuring a live `aria2c` seeder with `bench swarm`
to see which mask it uses on the wire. Everything needed is here, and the one
thing standing in the way is that a measurement over loopback is `ambiguous` by
construction, so it needs a target reachable on a class C address or aria2's
own set derived by hand from the address it sees. That is a session's work, not
a blocker.

### T-101 uTP is available but untested

Source:      corpus, `librqbit-utp`
Category:    bep
Priority:    P3
Effort:      M
Status:      open

Problem:     `ListenerOptions::mode` defaults to `TcpOnly`. `bit-cli` does not
             expose a way to enable uTP and has never tried it.
Relevance:   uTP is what keeps a seeding box from saturating its own uplink at
             the expense of everything else on the connection. On a netdisk
             that matters.
Approach:    Add `--transport tcp|utp|both`, default `tcp`, and measure. Rule
             0.10 applies: if it does not move a number, it does not ship.
Acceptance:  A download over uTP completes and verifies, and a run with a
             concurrent latency probe shows lower induced latency than the
             same run over TCP. Both numbers here.

**The title says "available" and it is not, which is a stronger statement of
the same gap.** Checked on 2026-08-21: there is no uTP anywhere in `bit-cli`.
`ListenerOptions::mode` is never set, no `--transport` flag exists, and
`grep -rn utp crates/` finds nothing. `librqbit-utp` 0.7.0 is in `cargo tree`
because `librqbit` depends on it, which is a dependency and not a capability.
The `README.md` protocol table said "available, off by default" and this file
said "inherited, off by default"; both read as a switch a user could flip.
There is no switch. Both are corrected, and the work in this entry is
unchanged: it was always "add the flag and measure", never "test the flag that
exists".

**Three implementations to read, and one argument for not writing one.**

`TorrentNG/crates/rt-utp/` is the most complete and the only one with a status
document: `TorrentNG/docs/protocol/UTP.md` separates "the packet codec works"
from "the engine can carry peer-wire traffic over it", which is exactly the
distinction this entry needs to make about itself. `congestion.rs` is LEDBAT
with `TARGET_DELAY_US = 100_000`, `:50` `on_ack` taking the base delay as a
running minimum of `timestamp_diff` and `:77` `on_timeout` halving with an MTU
floor, with three unit tests in the file. `selective_ack.rs:11` fixes
`EXTENSION_KIND = 1` and its doc states the bit numbering precisely: bit 0 of
the first byte acknowledges `ack_nr + 2`. `packet.rs`, `state.rs` and
`transport.rs` carry the header codec, the initiator-versus-acceptor
connection-ID derivation, and a shared-UDP endpoint that demultiplexes by
(remote address, receive connection id) so one socket serves many streams.

`mtorrent/mtorrent-core/src/utp/retransmitter.rs:48-50` fixes
`MAX_PACKET_SIZE = 9 KiB` (the macOS default UDP limit), `MIN_PACKET_SIZE = 1472`
(Ethernet MTU) and `INITIAL_RTO = 1 s`; `:108` `process_ack` is the
Jacobson/Karels RTT update applied **only to packets sent once**, with a fast
retransmit on the second duplicate ack. Its tests use
`tokio::test(start_paused = true)`, which is the same discipline
[T-035](performance.md) needed to make a token bucket testable.

`superseedr/src/networking/utp.rs:31-67` is the densest constants block in the
corpus if a number is wanted rather than an algorithm.

**And the argument against.** anacrolix
[Issue 1013](https://github.com/anacrolix/torrent/issues/1013) is the
maintainer of the widest-deployed Go implementation saying the pure-Go uTP is
buggy and to bind libutp instead. fx-torrent
[Issue 66](https://github.com/yoep/fx-torrent/issues/66) is one instance of
what that costs: a packet-parsing failure in the extension chain. A
hand-rolled uTP is a real and recurring maintenance cost, and this entry is P3
partly for that reason. If it is built, `librqbit-utp` already being in the
tree is the cheapest route by a wide margin.

### T-102 BEP 55 holepunch is not implemented

Source:      https://github.com/ikatson/rqbit/issues/463 (open)
Category:    bep
Priority:    P3
Effort:      L
Status:      open

Problem:     No holepunch support, so peers behind a filtering NAT are
             unreachable.
Relevance:   It raises the reachable swarm size, which matters for a leecher
             and less for a well-connected seed. The operator's case is the
             seed, so this is low priority here.
Approach:    Needs peer protocol work in `librqbit`.
Acceptance:  Deferred. Revisit if peer reachability shows up as a measured
             limit in `bench swarm`.

**Priced on 2026-08-21, and the answer is that no NAT library helps.** The
question that prompted this was whether `iroh` should be adopted for hole
punching. It should not, and BEP 55 does not want one.

BEP 55 is three bencode messages over connections that already exist. The
extension is `ut_holepunch`; the message carries `msg_type`, `addr_type`,
`addr`, `port`, and an optional `err_code`; the types are `rendezvous`,
`connect`, and `error`; the error codes are `NoSuchPeer`, `NotConnected`,
`NoSupport`, and `NoSelf`. A dial that fails through every route asks an
**already-connected peer** to relay a `rendezvous` naming the unreachable
target; that peer checks both sides advertise the extension and sends
`connect` to each carrying the other's address; both then dial, and the two
outbound SYNs crossing in flight open both NATs. That is the whole protocol,
and it is written out here rather than cited, because the working
implementation it was read against is not a tree this repository keeps.

**The swarm is the rendezvous.** That is the design, and it is why no relay
server, no STUN, and no overlay is needed. `iroh` is a QUIC overlay with its
own node identities and its own relays, and every peer on both ends must speak
it: adopting it would make `bit-cli` reachable to other `bit-cli` instances
rather than to the swarm, which is a private network wearing a BitTorrent
costume. The same objection retires the rendezvous-server model generally.

**What blocks it is the boundary this repository already knows.** The wire
format is expressible today: `librqbit-peer-protocol` 9.0.0 carries
`ExtendedMessage::Dyn(u8, BencodeValue)`, an escape hatch for an arbitrary
extended message. What is missing is a way in: `PeerConnectionHandler`'s
`on_extended_handshake` and `update_my_extended_handshake` are what would
advertise `ut_holepunch` and route its messages, and that trait is implemented
inside `librqbit` by the torrent state rather than by anything a dependent
crate supplies. It is the same wall [T-002](webseed.md) measured and
[T-135](multi-source.md) records the decision for.

So this stays P3 and open, blocked on that boundary and not on a missing
library. Nobody should reach for a NAT crate for it again.

**The 2026-08-21 corpus supplies both an implementation and the design
argument, and the design argument is the more valuable half.**

`fx-torrent/src/peer/extension/holepunch.rs` is 678 lines of working
implementation rather than a codec: `:14` `HolepunchMessage { msg_type,
addr_type, addr, port, err_code }`, `:149` `NAME = "ut_holepunch"`, message
types `Rendezvous`, `Connect`, `Error`. It landed in
[PR 64](https://github.com/yoep/fx-torrent/pull/64). The wire format alone, in
97 lines, is `torrent/peer_protocol/ut-holepunch/ut-holepunch.go`.

`torrent/NOTES.md:15-31` is the part worth adding to this entry, because it
answers a question the protocol write-up above does not.
**Rendezvous only through relays for the same torrent.** The argument: if you
send a `rendezvous` and later receive a `connect`, you cannot tell whether that
connect answers *your* rendezvous or one some other peer sent to your relay.
Relays are not required to respond, so you cannot enforce a timeout and time
the two apart. Therefore **you do not know which info hash to put in the
handshake**. Handshaking passively always fails, because the other side may do
the same and neither initiates. Constraining rendezvous to relays for the same
torrent removes the ambiguity, and then every `connect` can be handled
actively. That is a constraint on the design, not an optimisation, and getting
it wrong produces connections that open and then hang.

The same file carries the arithmetic for whether to bother: with 30 per cent
of peers unrelayable and 50 per cent behind a bad NAT, relaying takes pairwise
connectability from 75 per cent to 92.5 per cent. That is the number this
entry's "raises the reachable swarm size" should be measured against if it is
ever built.

### T-103 Filenames that are not valid UTF-8 are refused

Source:      https://github.com/ikatson/rqbit/issues/452 (closed, 2025-07-09)
Category:    bep
Priority:    P2
Effort:      S
Status:      open

Problem:     `add_torrent` failed with "cannot decode filename bit as UTF-8" on
             a torrent with a non-UTF-8 path.
Relevance:   BEP 3 does not require UTF-8. Real torrents carry Shift-JIS and
             CP1251 names, and the `encoding` key exists to say so.
             `librqbit` 9.0.0 carries an encoding detector
             (`torrent_metainfo.rs::detect_encoding`), so the closed label is
             probably right, but `bit-cli`'s own `Metainfo` parser decodes
             paths as UTF-8 and has not been tested against anything else.
Approach:    Add a fixture with a Shift-JIS path and check that `bit-cli info`,
             `files`, and `webseed list` all handle it, including the percent
             encoding of the composed URL.
Acceptance:  A non-UTF-8 fixture parses, lists, and composes a correct web seed
             URL, and the reported path says which encoding was used.

**The practical shape of this is not Shift-JIS, it is the `.utf-8` key
variants, and that half is cheaper and more common.** intermodal
[Issue 534](https://github.com/casey/intermodal/issues/534) (CLOSED): uTorrent
writes **both** `name` and `name.utf-8`, and both `path` and `path.utf-8`, with
different encodings in each. Neither variant is in BEP 3 and both are universal
in practice. The reporter's conclusion, which is what anacrolix's
`Info.BestName()` does and what parse-torrent does, is the rule to adopt:
**if the `.utf-8` variant exists, prefer it**.
`parse-torrent/index.js:123-131` treats `info.name` **or** `info['name.utf-8']`
as satisfying the required-field check, and `path` **or** `path.utf-8`
per file; `:140` and `:181` then prefer the `.utf-8` spelling throughout.
parse-torrent [Issue 177](https://github.com/webtorrent/parse-torrent/issues/177)
adds that `comment.utf-8` exists too. `bit-cli`'s `Metainfo` parser reads
`name` and `path` only, so a uTorrent torrent carrying a mojibake `name` beside
a correct `name.utf-8` gets the mojibake.

**The creation side has its own version of this and it is a worse bug, because
it ships.** mkbrr [Issue 182](https://github.com/autobrr/mkbrr/issues/182) is
in [T-175](create-seed.md): a torrent created on macOS against an SMB mount
wrote NFD filenames, verified clean locally including with the tool's own
check, and broke for everyone else. create-torrent
[Issue 195](https://github.com/webtorrent/create-torrent/issues/195) is the
blunter form: `mkdir $'ä'` then create, and the tool cannot stat its own
input.

So this entry splits into two pieces of work that share one fixture set:
prefer the `.utf-8` variants on read (small, and the win is immediate), and
decide what a non-UTF-8 path becomes on the way to a filesystem and to a
percent-encoded URL (larger, and it interacts with the path planner
[T-071](windows.md) already built).

### T-167 BEP 54 lt_donthave is not implemented

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    bep
Priority:    P2
Effort:      S
Status:      blocked

Problem:     A peer's bitfield only ever grows. BEP 3 has `Have` and no
             inverse, so once a peer has claimed a piece there is no way for
             it to withdraw the claim, and no way for `bit-cli` to hear one
             withdrawn.
Relevance:   This is the cheapest correctness win in the whole corpus for
             anything that tracks availability, and `bit-cli` tracks
             availability in two places that matter. The web seed bridge
             advertises a bitfield of exactly the pieces a source's scope
             covers in full, and [T-005](webseed.md) is the request to
             re-scope a source mid-run, which today cannot be expressed on the
             wire at all: a source that loses a file has no way to say so, and
             the session keeps asking. `lt_donthave` is that message. It is
             also what a partial seed needs when a mirror drops a file
             underneath it, which is the mirror case `bit-cli` exists for.
Approach:    `fx-torrent/src/peer/extension/donthave.rs` is the whole
             protocol, and it is small: `:19` `NAME = "lt_donthave"`, and the
             payload is a 4-byte big-endian piece index that clears one bit in
             the peer's bitfield. It is a BEP 10 extended message, so it costs
             one entry in the `m` dictionary the bridge already sends at
             `webseed/bridge.rs:708` and one handler on the receive side.
             Send it from the bridge when a scope narrows; honour it on
             receive by clearing the bit.
Acceptance:  A source re-scoped mid-run sends `lt_donthave` for every piece it
             has given up, the session stops requesting those pieces from it
             without dropping the connection, and a test asserts both. Pairs
             with [T-005](webseed.md), which is the reason to want it.

**Blocked on `librqbit` 9.0.0, and the blocker is the receive side rather than
the send side.** Read before writing any of it, which is what
[RULES.md](RULES.md) asks for and what this entry's own approach did not do.

Sending `lt_donthave` is as small as this entry says. Honouring one is not
`bit-cli`'s to do, and nothing in the session does it.

`librqbit-9.0.0/src/torrent_state/live/mod.rs:1076` dispatches
`Message::Have(h) => self.on_have(h)`, and `on_have` at `:1523` sets one bit in
`live.bitfield`. There is no inverse. Every extension message the session does
not know falls to the catch-all at `:1112`:

```rust
message => {
    warn!("received unsupported message {:?}, ignoring", message)
}
```

An `lt_donthave` arrives there as `ExtendedMessage::Dyn(id, ..)`, because
`PeerExtendedMessageIds` (`librqbit-peer-protocol-9.0.0/src/extended/mod.rs`)
carries `ut_metadata` and `ut_pex` and nothing else. So the bridge sending one
would produce a log line per retracted piece and change nothing about what the
session requests. That is worse than not sending it: a message the far end
warns about and ignores is noise that looks like a feature.

**There is no seam to do it locally either, and the near miss is worth
recording so nobody re-derives it.**
`librqbit-9.0.0/src/torrent_state/live/peers/mod.rs:114` is
`pub fn update_bitfield(&self, handle: PeerHandle, bitfield: BF)`, which is
exactly the operation needed and is declared `pub`. It is unreachable:
`lib.rs:75` declares `mod torrent_state;` with no `pub`, so the whole module
tree under it is private to the crate and `pub` inside a private module reaches
nothing. `bit-cli` holds a `ManagedTorrent` and has no path to its live peer
state.

**What would unblock it**, in the order of how much has to change upstream:

1. `librqbit` adds `lt_donthave` to `PeerExtendedMessageIds` and an
   `on_donthave` beside `on_have` that clears the bit. That is the correct fix
   and it is small: `on_have` is twenty lines and the inverse is the same
   twenty with `false` instead of `true`.
2. Failing that, `librqbit` makes `torrent_state` public, or exposes
   `update_bitfield` through `ManagedTorrent`. Then `bit-cli` could parse the
   message in the bridge and clear the bit locally, which is not the protocol
   but is the same outcome for an in-process pair.

`fx-torrent/src/peer/extension/donthave.rs:19` is still the whole protocol and
still the reference to build from: `NAME = "lt_donthave"`, a 4-byte big-endian
piece index, and `set_remote_has_piece(piece, false)`. What that tree has and
this one does not is a peer layer of its own. `bit-cli`'s peer layer is
`librqbit`'s, by decision 7.3.

**One half of this entry is not blocked, and it is deliberately not built
yet.** Any extension message the bridge **sends** needs the peer's numbering,
read out of the peer's own extended handshake, which is the second of the two
BEP 10 tables [T-166](peers.md) names. The first table, `OUR_EXTENSIONS` in
`crates/bit-cli-core/src/webseed/bridge.rs`, exists and is the receive
direction. The second does not, because nothing sends an extension message and
a table with no caller is infrastructure written against a guess. T-166 records
the seam; this entry is the first thing that will need it.

**[T-005](webseed.md) does not wait on this.** That entry's own approach,
narrow the scope and reconnect with the smaller bitfield, needs no extension at
all. What `lt_donthave` would have bought is one message instead of one
reconnect, which is an optimisation of a path that has to exist either way.
T-005 was built on the reconnect, and this entry becomes an optimisation of it
rather than a prerequisite for it. The work order that put this first was
written before the dispatch above had been read.

### T-168 WebTorrent peers and WSS trackers are not supported

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    bep
Priority:    P3
Effort:      XL
Status:      open

Problem:     `bit-cli` speaks TCP to peers and HTTP or UDP to trackers. A
             `wss://` tracker URL in a torrent is not announced to, and a
             WebTorrent peer cannot be reached at all.
Relevance:   WebTorrent is a separate swarm sharing the same info hash. A
             torrent whose `announce-list` carries `wss://` tiers, which is
             the default for anything created by `create-torrent`, see
             `create-torrent/index.js:16-24`, where three `wss://` trackers sit
             beside the `udp://` ones each in its own BEP 12 tier, has peers
             `bit-cli` cannot see and does not report. `bit-cli trackers`
             announcing to every tracker in a torrent except the `wss://` ones
             is the visible half of that.

             Weighed honestly this is completeness rather than reach for
             `bit-cli`'s stated case. The operator's case is a seedbox and a
             netdisk, and a browser peer is neither. It is P3 for that reason
             and not because the work is large.
Approach:    Three sources, one per layer, and they are unusually complete for
             a protocol with no BEP.

             `torrust-actix/RtcTorrent.md` is 937 lines and self-contained:
             tracker announce extensions and their query parameters, the
             four-step signalling flow, the WebRTC data-channel message types
             (`MSG_PIECE_REQUEST 0x01`, `MSG_PIECE_DATA 0x02`,
             `MSG_PIECE_CHUNK 0x04`), chunked transfer, flow control, and a
             client implementation guide covering the ICE and SDP lifecycle,
             the announce loop, in-flight request management, piece
             verification and peer blacklisting. Its section 15 states the
             interop posture that makes it safe to add: RTC is purely
             additive, non-RTC clients see one extra `"rtc interval"` key they
             ignore, and mixed swarms work. Its section 14, five real defects
             with symptom, cause and fix, is worth reading before writing any
             of it.

             `torrent/webtorrent/` is the client side.
             `tracker-protocol.go` has the JSON announce shape with
             `offers[]`, `answer` and `to_peer_id`, plus `binaryToJsonString`,
             one rune per byte, which is the de-facto encoding for binary
             fields in WebTorrent JSON.
             `torrent/webtorrent/transport.go:261-303` wraps a detached data
             channel as an `io.ReadWriteCloser` and caps writes.

             `aquatic/crates/ws_protocol/` is the tracker side, and its
             comments record what the reference client actually does rather
             than what any document says:
             `aquatic/crates/ws_protocol/src/incoming/announce.rs:13` notes
             that `left` may be absent when a magnet is opened, that the
             length of `offers` **is** the peer count wanted, that the
             reference client caps it at 10, and that offers are not sent for
             `stopped` or `completed`.

             superseedr [Issue 319](https://github.com/Jagalite/superseedr/issues/319)
             scopes the whole job from a client author's side: WebRTC data
             channels, `ws://` and `wss://` announces, coexistence with TCP
             peers in one swarm, and a browser-peer test harness.
Acceptance:  Cannot be met incrementally, so it splits. First half:
             `bit-cli trackers` announces over `wss://` and reports the peers
             it is told about, which is useful on its own and needs no WebRTC.
             Second half: a WebTorrent browser peer and `bit-cli` exchange a
             verified piece. Record the first half's output here when it lands
             and leave the second open.
