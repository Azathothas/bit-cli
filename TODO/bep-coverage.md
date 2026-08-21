# BEP coverage

Sixty-six issues in the corpus mention a BEP or a protocol feature. This file
tracks what `bit-cli` speaks today and what it does not.

Implemented means there is a test. Inherited means `librqbit` provides it and
`bit-cli` has not verified it independently.

| BEP | What | Status |
| --- | --- | --- |
| 3  | The BitTorrent protocol | inherited |
| 5  | DHT | inherited, not reported (T-052) |
| 6  | Fast extension | not implemented (T-100) |
| 9  | Metadata from peers (magnet) | inherited |
| 10 | Extension protocol | implemented in the bridge |
| 11 | PEX | inherited |
| 12 | Multitracker metadata | implemented in `create`, `edit`, `trackers` |
| 14 | Local service discovery | inherited |
| 15 | UDP tracker protocol | implemented in `tracker.rs` |
| 16 | Superseeding | not implemented (T-082) |
| 17 | HTTP seeding, Hoffman style | implemented in `fetch.rs` |
| 19 | HTTP/FTP seeding, GetRight style | implemented, the headline feature |
| 20 | Peer id conventions | implemented |
| 21 | Extension for partial seeds | implemented in the bridge |
| 23 | Compact peer lists | implemented in `tracker.rs` |
| 27 | Private torrents | implemented in `create`, `edit` |
| 29 | uTP | inherited, off by default |
| 39 | Updating torrents via feed URL | implemented in `create`, `edit` |
| 47 | Padding files | not implemented (T-081) |
| 48 | Tracker scrape | implemented in `tracker.rs` |
| 52 | BitTorrent v2 | not implemented (T-081) |
| 55 | Holepunch | not implemented (T-102) |

---

### T-100 BEP 6 fast extension is not implemented

Source:      https://github.com/ikatson/rqbit/issues/584 (open)
Category:    bep
Priority:    P2
Effort:      L
Status:      open

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
outbound SYNs crossing in flight open both NATs. `reference/README.md` has the
whole flow with file and line citations into a complete 678 line
implementation.

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
