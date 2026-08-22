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

**Asked:** whether `iroh` should be used for hole punching / NAT traversal to
support BEP 55.

**Answer: no, and BEP 55 does not need it.** BEP 55 is not a NAT traversal
library problem. It is a three-message extension over connections that already
exist, and `fx-torrent` implements the whole thing in one 678 line file with no
dependency beyond bencode.

**How it works**, from `fx-torrent/src/peer/extension/holepunch.rs`:

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

**Why iroh does not fit.** iroh is a QUIC overlay with its own node
identities, its own relay servers, and its own transport. Every peer on both
ends must speak it. A BitTorrent client exists to join swarms full of
qBittorrent, Transmission, and libtorrent, none of which will ever dial an
iroh endpoint. Adopting it would not make `bit-cli` reachable behind a NAT to
the swarm; it would make it reachable to other `bit-cli` instances, which is a
private network wearing a BitTorrent costume. The same objection retires the
rendezvous-server model generally, and the security issues above are what that
infrastructure costs.

BEP 55 needs no rendezvous server because **the swarm is the rendezvous**.
That is the whole design.

**What it would actually cost here, and why it is still blocked.** The wire
format is expressible today: `librqbit-peer-protocol` 9.0.0 has
`ExtendedMessage::Dyn(u8, BencodeValue)`
(`extended/mod.rs:47`), an escape hatch for arbitrary extended messages. What
is missing is the same thing [T-002](../TODO/webseed.md) priced: `librqbit`
owns the peer connection. `PeerConnectionHandler::on_extended_handshake` and
`update_my_extended_handshake` (`librqbit-9.0.0/src/peer_connection.rs:41,49`)
are the hooks that would have to advertise `ut_holepunch` and route its
messages out, and the trait is implemented internally by the torrent state
rather than by anything a dependent crate can supply.

So [T-102](../TODO/bep-coverage.md) is blocked on exactly the boundary
[T-135](../TODO/multi-source.md) already names, and adding a NAT library would
not move it one line. This is worth recording under T-102 so nobody reaches
for iroh again.

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

Four trees kept and four deleted, and the pattern across all of them was
consistent enough to be worth stating.

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
