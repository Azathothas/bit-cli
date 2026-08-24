# RESEARCH.md — BitTorrent research corpus for `bit-cli`

Scope: everything under `reference/`. **Thirty-nine repositories** in forty-one entries, the last two of which are an organisation triage and a document rather than trees. Twenty-two arrived 2026-08-21 and seventeen more on 2026-08-24. Each is
cleaned of `.git`, `.github`, marketing, GUI/packaging trees and unrelated code,
then read twice each — once broadly, once against the specific open items in
`bit-cli`'s `TODO/INDEX.md`.

**What `bit-cli` is, for the reader of this file.** A non-interactive Rust
BitTorrent + HTTP downloader whose differentiator is attaching arbitrary web
seeds to an existing `.torrent` without rewriting it. It is a fork of `kist`,
builds on **`librqbit`** (Apache-2.0), and adapts torrent creation/linting from
**`intermodal`** (CC0-1.0). It is MIT-licensed and its `deny.toml` permits
permissive licences only, so **every repository below is licence-compatible**
(all MIT except `intermodal`, which is CC0-1.0) — see the licence caveats on
`tc` and `vortex`.

`bit-cli` declares no support for BEP 6, 16, 47, 52, 55; uTP (BEP 29) is
available but untested. Its open TODO items referenced throughout are:

| ID | Item |
|---|---|
| T-004 | BEP 17 style is not auto-detected, only declared |
| T-007 / T-008 | Stalling source takes 24s to give up; duplicate block fetched twice |
| T-016 | fastresume is not used when adding a torrent (blocked) |
| T-018 | Write path issues one operation per 16 KiB block |
| T-020 / T-022 | CLOSE_WAIT accumulation; IPv6-only swarm churn |
| T-024 / T-083 | Choke/unchoke history and disconnect reasons not reported |
| T-033 / T-034 | `--split`/`-x`/`-k` do not reach the fetch path; endgame not observable |
| T-041 | Per-source window cache bounded but not measured |
| T-050 / T-051 / T-052 | DHT cache I/O; magnet with no DHT/trackers; DHT not reported |
| T-063 / T-064 / T-065 | Tiers announced in parallel; UDP retry ignores BEP 15 backoff; scrape only BEP 48 convention |
| T-081 / T-082 | BEP 52 v2/hybrid not implemented; BEP 16 superseeding not implemented |
| T-100 / T-101 / T-102 / T-103 | BEP 6; uTP untested; BEP 55; non-UTF-8 filenames refused |
| T-114 / T-116 | `-i/--input-file` batch; `-O/--index-out` cannot rename |
| T-132 / T-134 / T-135 / T-143 | Swarm vs HTTP rate limits; v1/v2 hash reconciliation; runtime source steering; attach source to running torrent |
| T-200…T-209 | Phase C: daemon, JSON-RPC/XML-RPC aria2 parity, session save/restore, watch dirs, RSS |

## How to read the rankings

Tier 1 repositories answer an open `bit-cli` TODO directly with code that can be
read, ported, or measured against. Tier 2 answer one narrower question well.
Tier 3 are corroborating references or negative examples.

**Entries 23 onward each carry an explicit `Passes` and `Verdict` section; the
first twenty-two predate that convention** and carry the same judgement in
prose instead. Where one of the first twenty-two is re-mined it gains both,
which is why entry 13 has them and entry 12 does not.

**Entry numbers are the order an entry was added, and the tier heading above
it is the ranking.** Entries 1 to 22 were written on 2026-08-21 and are
numbered in tier order; 23 onward were added later and sit under the tier they
were ranked into. Renumbering would break every citation already written
against them, which is the same reason a trim deletes rather than moves.

Every file path, line number, symbol and Issue/PR below was verified in the
cleaned tree or fetched with `gh` during this pass. Line numbers refer to the
**files as they stand after cleaning** (only `README.md` files were edited; no
source file was modified).

---

# Tier 1 — highest impact

## 1. `torrent` — anacrolix/torrent

- Upstream: <https://github.com/anacrolix/torrent>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\torrent`
- Licence: **MIT** (`torrent/LICENSE`). The vendored `webtorrent/` subpackage
  carries its own MIT notice, `webtorrent/LICENSE`, © 2019 Michiel De Backker.
- Language: Go. In production since 2014; by far the widest BEP surface here.

### Relevant BEPs / features

BEP 3, 5, 6, 9, 10, 11, 12, 14, 15, 17, 19, 20, 23, 27, 29, 40, 47, 48, 52, 55;
MSE/PE; WebTorrent (WSS tracker + WebRTC data channels); smart-ban; multiple
storage backends.

### BEP 6 — fast extension (`bit-cli` T-100)

Moved to [`HISTORY/RESEARCH-bep6.md`](HISTORY/RESEARCH-bep6.md) on 2026-08-24: T-100 closed and the
behaviour is in the tool.

### BEP 52 v2 and hybrid (T-081, T-134)

- `torrent/merkle/merkle.go:10` `BlockSize = 1<<14`; `:12` `Root`;
  `:28` `RootWithPadHash`; `:47` `CompactLayerToSliceHashes`.
- `torrent/merkle/hash.go:9` `NewHash` — a streaming `hash.Hash` over 16 KiB
  blocks; `:63` `Sum`; `:70` `SumMinLength` pads with zero hashes for a file
  tail that does not fill the piece. This is the exact primitive `bit-cli`
  needs for v2 hashing on the create side.
- `torrent/metainfo/bep52.go:9` `ValidatePieceLayers` — walks the file tree,
  reconstructs each `pieces root` from its piece layer and compares;
  `:50` `HashForPiecePad` derives the pad hash for a given piece length.
- `torrent/metainfo/file-tree.go:105` `upvertedFilesInner` — **v2 files are
  piece-aligned**, so the running offset is bumped to the next piece boundary
  per file; `:149` `PiecesRootAsByteArray`; `:164` `Validate` (non-empty file ⇒
  exactly 32 bytes, empty file ⇒ none).
- `torrent/metainfo/info.go` — `HasV1()`/`HasV2()`/`NumPieces()` handle the
  upgrade path; `MetaVersion == 2` is the discriminator, and `HasV1()` is true
  when `MetaVersion` is 0/1 **or** any of `Files`/`Length`/`Pieces` is present.
- `torrent/types/infohash-v2/infohash-v2.go:60` `ToShort()` — **truncates the
  32-byte v2 hash to 20 bytes for DHT and tracker use**. That single function is
  the answer to T-134: keep both identities, use the truncation only at the
  auxiliary-protocol boundary.
- `torrent/metainfo/magnet-v2.go` — `xt=urn:btmh:` parsing with a multihash
  check (`SHA2_256`, length 32) and `xt=urn:btih:` alongside it, plus
  `MetaInfo.MagnetV2()` emitting both for a hybrid torrent.
- **Fixtures:** `torrent/testdata/bittorrent-v2-test.torrent` (pure v2: `file
  tree`, `meta version`, `piece layers`, no `files`) and
  `torrent/testdata/bittorrent-v2-hybrid-test.torrent` (hybrid: also has
  `files`). Verified by inspecting the bencode keys present in each.

### Web seeding (bit-cli's core)

- `torrent/webseed/request.go:24` `defaultPathEscaper` — path components are
  **query-escaped**, and `+` is additionally escaped to `%2B` because S3
  decodes `+` as a space. `:49` `urlForFileIndex` appends `name` + `path`
  **only when the base URL ends in `/`** (BEP 19). `:64` `newRequest`
  **omits the `Range` header entirely** when the request covers the whole file.
- `torrent/webseed/client.go:29` `MaxDiscardBytes = 48<<10` — when a server
  answers `200` to a ranged request, up to 48 KiB is discarded to reach the
  wanted offset; beyond that the request is failed as
  `ErrStatusOkForRangeRequest` (`torrent/webseed/client.go:207` `recvPartResult`).
- `torrent/webseed/client.go:185` `checkContentLength` — only compares `Content-Length` when
  `Content-Encoding` is `identity` or absent.
- `torrent/webseed/client.go:270` `ErrTooFast` — `503` is treated as backpressure, not death.
- `torrent/webseed-peer.go:57` `webseedFileUnavailable` — **403/404/410/451 remove
  only that file's pieces from the web seed's bitmap** (`:71`
  `removeFilePieces`) instead of retiring the source. `bit-cli` currently
  retires the whole source on 401/403/404/410/416; per-file removal is strictly
  better for a mirror that holds part of a payload — which is exactly
  `bit-cli`'s "scope" model.
- `torrent/webseed-peer.go:46` `convict(err, time.Minute)` — a source is suspended for a
  term rather than killed; compare `bit-cli`'s `--web-seed-cooldown`.
- `torrent/webseed-peer.go:327` `maxChunkDiscard` / `torrent/webseed-peer.go:344` `readChunks` — the response
  body keeps being read after a cancel to make use of already-buffered bytes,
  and the stream is cancelled when no wanted chunk remains inside the discard
  window. This is a concrete answer to T-008 (duplicate block fetched twice).

### Making the picker prefer HTTP (T-003 follow-on, T-135)

`torrent/requesting.go:191-196`:

```go
if t.hasActiveWebseedRequests() {
	// Prefer the highest possible request index, since webseeds prefer the lowest. Additionally,
	// this should mean remote clients serve in reverse order so we meet webseeds responses in
	// the middle.
	ml = ml.Cmp(-cmp.Compare(leftRequest, rightRequest))
} else {
```

`bit-cli`'s README states librqbit's piece picker is not reachable from outside
the crate, so `--prefer-web-seed` can only move the odds. anacrolix's approach —
**invert the peer ordering while web seeds are active so the two ends converge**
— is a design `bit-cli` can adopt if it ever gets picker access, and it costs
nothing but a comparator branch. `torrent/torrent.go:3797`
`hasActiveWebseedRequests` is the gate.

### Smart ban — attributing a bad piece to the right source

`torrent/smartban/smartban.go` (whole file, 83 lines) plus
`torrent/smartban.go`. Every block is recorded with the peer that supplied it;
when a piece fails its hash, `CheckBlock` returns exactly the peers whose block
hashes disagree with the verified data. For `bit-cli`, whose whole model is
several sources filling one piece, this converts "a source is bad" from a guess
into a fact — directly useful for the `--web-seed-verify piece` path.

### Trackers

Moved to [`HISTORY/RESEARCH-trackers.md`](HISTORY/RESEARCH-trackers.md) on 2026-08-24: T-063, T-064 and T-065 closed and the
behaviour is in the tool.

### `url-list` is not always a list (metainfo edge case)

`torrent/metainfo/urllist.go:11` — `UrlList.UnmarshalBencode` branches on the
first byte: `l` ⇒ list, anything else ⇒ a single bencoded string. The fixture
`torrent/metainfo/testdata/flat-url-list.torrent` contains
`8:url-list29:https://archive.org/download/` — a bare string. Any `bit-cli`
parser that assumes a list will silently drop the only web seed in such a
torrent.

### Other in-tree pieces worth reading

- `torrent/bep40.go:51,69` — BEP 40 canonical peer priority: CRC32-Castagnoli
  over masked address pairs, with `ipv4Mask` (`:21`) widening the mask from /16
  to /24 to /32 as the two addresses get closer, and `ipv6Mask` (`:42`).
- `torrent/bep14.go` — LSD: `239.192.152.143:6771` / `[ff15::efc0:988f]:6771`,
  `BT-SEARCH * HTTP/1.1` with one `Infohash:` header per torrent, packets kept
  under 1400 bytes to stay inside an Ethernet MTU, 10 s/1 s timeouts.
- `torrent/peer_protocol/ut-holepunch/ut-holepunch.go` — BEP 55 wire format
  (`rendezvous`/`connect`/`error`, IPv4/IPv6 address types, 4-byte error code)
  in 97 lines. `torrent/ut-holepunching.go` is a stub; the logic lives in
  `peerconn.go`.
- `torrent/peer_protocol/pex.go` — BEP 11 message shape and the flag bits
  (`PexPrefersEncryption`, `PexSeedUploadOnly`, `PexSupportsUtp`,
  `PexHolepunchSupport`, `PexOutgoingConn`); `torrent/pex.go:20-23` carries the
  practical limits: hold drops back below 25 live conns, 25-entry drop buffer,
  50 added/dropped per message.
- `torrent/metainfo/piece-length.go:44,61` — piece-length chooser: start at
  16 KiB and double while the piece count is at or above 2048, bounded by
  optional hard min/max sizes and soft min/max counts.
- `torrent/storage/safe-path.go` — `ToSafeFilePath` cleans each component then
  rejects a result whose first component is `..`; the test table in
  `safe-path_test.go` includes the real-world
  `"NewSuperHeroMovie-2019-English-720p.avi /../../../../../Roaming/…/Startup/test3.exe"`
  case. Useful as a second opinion on `bit-cli`'s own path planner.
- `torrent/webtorrent/` — `tracker-protocol.go` has the JSON announce shape with
  `offers[]`/`answer`/`to_peer_id`/`offer_id`, and `binaryToJsonString` (one
  rune per byte) which is the de-facto encoding for binary fields in WebTorrent
  JSON. `torrent/webtorrent/transport.go:261-303` wraps a detached data channel as an
  `io.ReadWriteCloser` and caps writes (pion/datachannel#59).
- `torrent/analysis/peer-upload-order.go` — instrumentation that records the
  order in which a peer serves the requests you sent it; a cheap way to build
  the "which peer answers first" evidence `bit-cli`'s bench work needs.

### Two web-seed integration tests that are `bit-cli` TODOs

`torrent/tests/` is a separate Go module "to avoid getting run by `./...` in the
root" (`torrent/tests/README`). Two of its directories are directly on point:

- **`torrent/tests/add-webseed-after-priorities/`** — attaching a source to a
  torrent that has **already started**, which is `bit-cli` T-143.
  `torrent/tests/add-webseed-after-priorities/herp_test.go:80-84` is the whole point: `DownloadAll()` first, sleep a
  second, *then* `AddWebSeeds(["http://localhost:3003/test.img"])`. The `README`
  states the acceptance condition: "The seeder should start fetching from HTTP,
  despite the webseed being added after `Torrent.DownloadAll` is called. It
  should still fetch even if the leecher doesn't connect (disable the `AddPeers`
  line)." The `justfile` shows the fixture: a 500 MiB sparse `test.img`
  (`mkfile -n 500m`) served by Python's `rangehttpserver` on port 3003, with
  `test.img.torrent` committed. That is a smaller version of `bit-cli`'s own
  `loopback-fileserver` example, and the "still fetch with no peer at all" case
  is exactly `bit-cli`'s `--web-seed-only`.
- **`torrent/tests/webseed-partial-seed/`** — the same rig used for a different
  property, from anacrolix/torrent discussion 916: "ensure that the seeder and
  leecher progress completed pieces in lock step. The bug was that the leecher
  would reach the end of its max unverified bytes window before hitting a piece
  that the seeder had available." A bounded unverified-bytes window is the same
  resource `bit-cli` T-041 wants measured, and this is the deadlock it can cause.

The remaining directories (`issue-798`, `issue-930`, `issue-952`,
`peers-bootstrapping`) are single-file reproductions kept for the same reason.

### Three design notes in the tree, not in any issue

- **`torrent/NOTES.md:15-31` — why hole-punching is tracked per torrent, not per
  client (BEP 55, `bit-cli` T-102).** The argument: if you send a `rendezvous`
  and later receive a `connect`, you cannot tell whether that connect answers
  *your* rendezvous or one a peer sent to your relay — relays are not required
  to respond, so you cannot enforce a timeout — and therefore you do not know
  which info hash to put in the handshake. Always handshaking passively fails
  because the other side may do the same and neither initiates. The resolution
  is the constraint worth copying: *only rendezvous through relays for the same
  torrent as the relay*, and then every connect can be handled actively.
  The same file carries the NAT arithmetic behind bothering at all: with 30 %
  unrelayable and 50 % bad NAT, relaying takes pairwise connectability from
  75 % to 92.5 %, and notes that on the DHT a bad-NAT node can still query
  without contributing.
- **`torrent/TODO:8-14` — an unpublished web-seed wishlist.** Five items, all of
  which land on `bit-cli`'s open work: favour giving requests to larger torrents
  because "R2 for example limits speed per object"; force web-seed requests to
  be applied synchronously to an available object reader and contiguous; always
  make **open-ended** requests so more requests can be applied; expanding
  request windows once that holds; and close an open object when no further
  request is forthcoming, which frees a slot. `torrent/TODO:15-16` then proposes
  using the unverified-bytes window for per-connection requesting, which "could
  make webseeding cooperate much more effectively with regular peers" — the same
  ground as `bit-cli` T-003, T-041 and T-135.
- **`torrent/internal/request-strategy/NOTES.md` — the request algorithm in
  prose.** Pieces are grouped by shared storage capacity and ordered by
  priority, availability, index, then infohash; scanning stops once cumulative
  piece length exceeds the storage cap; and `:14` gives the other stop
  condition, a **64 MiB default unverified-bytes limit**. `:19-27` lists the
  candidate sort in order: allowed-fast while choked, piece priority, already
  outstanding to this peer, not pending from any peer, then (for a request
  already held elsewhere) that peer's outstanding count and most-recently-
  requested time, then least-available piece. That is the sort `bit-cli` would
  need to reproduce or override to make `--prefer-web-seed` deterministic.

### Issues / PRs worth `bit-cli`'s attention

| # | State | Title | Why it matters |
|---|---|---|---|
| [PR 1052](https://github.com/anacrolix/torrent/pull/1052) | MERGED | Complete BEP 6 Fast Extension: send Allowed Fast Set + record received | Two-part story: received `AllowedFast` was a no-op, and the sending side was missing. **Carries the canonical BEP 6 vector**: `ip=80.4.4.200`, infohash `0xAA`×20, `numPieces=1313`, `k=7` ⇒ `[1059, 431, 808, 1217, 287, 376, 1188]`. Also states the mask is the **/24-masked peer IP**, and that the algorithm is IPv4-only. `bit-cli` should ship exactly this vector as a unit test for T-100. |
| [PR 1056](https://github.com/anacrolix/torrent/pull/1056) | MERGED | fix BEP 52 metainfo handling | A `pieces root` that is not exactly 32 bytes panicked inside the file-tree iterator, reachable from `AddTorrent`, `SetInfoBytes` **and peer metadata exchange**. Fix = validate the tree before `setInfo` touches it. `bit-cli` accepts torrents from magnets, HTTP URLs and stdin; the same hostile input reaches it. |
| [PR 1054](https://github.com/anacrolix/torrent/pull/1054) | MERGED | reject unsolicited BEP 52 hashes messages | A peer-triggered crash: `Hashes` was processed without checking it answered an outstanding `HashRequest`, so a bogus `pieces root` dereferenced a nil file. Fix also rejects unsupported proof layers and length/index mismatches. Required reading before implementing v2 hash exchange. |
| [PR 1066](https://github.com/anacrolix/torrent/pull/1066) | MERGED | Fix panic requesting hashes for files with 512n+1 pieces | A v2 file with `fileNumPieces % 512 == 1` leaves one hash in the last request block, and the BEP 52 minimum request length is 2. Precise off-by-one that any v2 implementation will hit. |
| [PR 1055](https://github.com/anacrolix/torrent/pull/1055) | MERGED | tracker/http: handle malformed non-compact peer list entries | A tracker returning `peers: [42]`, or a dict missing `ip`/`port`, crashed the client. Fix keeps the good entries and errors on the bad ones. Trackers come from untrusted torrents; `bit-cli trackers` announces for real. |
| [PR 1074](https://github.com/anacrolix/torrent/pull/1074) | MERGED | storage: avoid recursive RLock in piece resource WriteTo fallback | `sync.RWMutex` is not recursive; a queued writer between two `RLock`s deadlocks. Same class as `bit-cli` T-010 ("pwrite takes a read lock where it needs a write lock"), with a deterministic reproduction test named in the PR. |
| [PR 1065](https://github.com/anacrolix/torrent/pull/1065) / [PR 1039](https://github.com/anacrolix/torrent/pull/1039) | MERGED | tracker/udp: guard `connIdIssued` reset with `mu` | Data race on the UDP connection-id reissue path. Relevant to T-064. |
| [PR 1051](https://github.com/anacrolix/torrent/pull/1051) | OPEN | Feat/windows storage fast stable | Caches writable handles so a 16 KiB chunk write stops reopening the file; buffers piece-completion persistence on a **1 s / 128 MiB** checkpoint instead of per piece; keeps `complete=false` immediate. Directly addresses T-018 and T-011-adjacent work. |
| [PR 1075](https://github.com/anacrolix/torrent/pull/1075) | OPEN | dialer: add SOCKS5 proxy support for peer connections | Peer dialing goes through registered dialers; only HTTP tracker traffic was proxied before. A capability `bit-cli` lacks entirely. |
| [PR 933](https://github.com/anacrolix/torrent/pull/933) | MERGED | Webseed Client - Close body in same go routine as request | Web-seed lifetime/cancellation hazard. |
| [Issue 1070](https://github.com/anacrolix/torrent/issues/1070) | OPEN | Peer request-update wakeup can be lost by the message writer, wedging the connection | Three-way interaction: the writer subscribes *after* draining, so a tickle in between is dropped; the guard means it is never retried; the keep-alive timer only resets after a successful write so there is no backstop. Symptom = "download wedges at zero bytes with a live, unchoked seeder". This is the shape of `bit-cli` T-037 ("a run stalls for minutes, roughly once in fifty"). |
| [Issue 1036](https://github.com/anacrolix/torrent/issues/1036) | OPEN | panic in Webseed at master | `panicif.False` inside `updateWebseedRequests` — an invariant in the web-seed request scheduler that does not always hold in production. Worth knowing before copying that scheduler's design. |
| [Issue 1013](https://github.com/anacrolix/torrent/issues/1013) | OPEN | Remove/replace non-cgo anacrolix/utp | The maintainer's position: the pure-Go uTP is buggy, use the libutp binding. Evidence for T-101 that a hand-rolled uTP is a real maintenance cost. |
| [Issue 1062](https://github.com/anacrolix/torrent/issues/1062) | OPEN | File is locked, Cannot delete | Windows: `.part` file stays open after `Drop()`+`<-Closed()`, so `os.RemoveAll` fails repeatedly. Same family as `bit-cli` T-070. |
| [Issue 992](https://github.com/anacrolix/torrent/issues/992) | CLOSED | after decoding metainfo: expected EOF | Strict trailing-byte checking rejects `.torrent` files that other clients accept. See `mkbrr`'s `decodeTorrentRoot` for the tolerant version. |
| [Issue 1005](https://github.com/anacrolix/torrent/issues/1005) | OPEN | DHT announce timeboxing prevents full traversal with large torrent counts | Relevant if `bit-cli` ever runs many torrents with DHT (T-050). |

---

## 2. `nanotorrent` — Power2All/nanotorrent

- Upstream: <https://github.com/Power2All/nanotorrent>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\nanotorrent`
- Licence: **MIT**, from `nanotorrent/Cargo.toml` (`license = "MIT"`). There is
  no `LICENSE` file in the repository, so that manifest is the only in-repo
  statement and is one of the five kept back from the manifest sweep (§G).
- Language: Rust 2024. A port of PicoTorrent that **vendors and patches
  `librqbit` 8.1.1** — the same engine `bit-cli` builds on.

This is the single most directly actionable repository in the corpus for
`bit-cli`, because everything in it is "what you have to add on top of
`librqbit`".

### The librqbit patch stack (`nanotorrent/patches/`, described in `nanotorrent/vendor/librqbit/PATCHES.md`)

| Patch | What it exposes / fixes | `bit-cli` relevance |
|---|---|---|
| `0001-expose-chunk-tracker.patch` | `ManagedTorrent::with_chunk_tracker` `pub(crate)` → `pub` | T-025 ("`PeerStatsFilterState` is not exported, so the filter is built by JSON") is the same species of problem; this is the minimal-diff answer. |
| `0002-per-peer-have-pieces.patch` | adds `TorrentStateLive::per_peer_have_pieces() -> Vec<(SocketAddr, u64)>` | `bit-cli peers` reports "pieces it verified"; real per-peer availability needs this. |
| `0003-stream-transform-seam.patch` | `pub trait StreamTransform` + `SessionOptions::stream_transform`; every **outgoing** peer stream is passed through it after connect, before the handshake | The injection point for MSE/PE without forking the engine. |
| `0005-incoming-stream-transform-seam.patch` | `IncomingStreamTransform` for the **accept** path; `CheckedIncomingConnection.stream: TcpStream` becomes boxed read/write halves. The transform receives **all active info hashes** because the incoming hash is not known until the (possibly encrypted) handshake is read — the MSE responder resolves the peer's SKEY against them | The non-obvious half of inbound MSE. |
| `0004-pex-toggle.patch` | `SessionOptions::disable_pex`, gating both PEX directions | A private-torrent / BEP 27 lever `bit-cli` will want. |
| `0006-proxy-scope.patch` | `proxy_peers` / `proxy_trackers` / `proxy_hostnames`, matching libtorrent's semantics; `socks5h` upgrade for proxy-side DNS. **UDP tracker announces are never proxied — a librqbit limitation** | Documents a real gap for anyone adding proxy support. |
| `0007-anonymous-mode.patch` | `SessionOptions::anonymize` clears `handshake.v` in the extended handshake; the random peer id half is done app-side | `bit-cli version` reports "protocol support"; anonymity is a peer-identity decision. |
| `0008` (spans two crates, described in `PATCHES.md`) | per-tracker announce stats: `TrackerStat`/`SharedTrackerStats` in `librqbit-tracker-comms`, `Session::tracker_stats_snapshot(info_hash)` and `tracker_tiers_snapshot(info_hash)` in `librqbit`. **librqbit otherwise flattens `announce_list` tiers into a `HashSet`** | This is the mechanism `bit-cli trackers` needs to report tier, interval, seeders and leechers per tracker — and the note that tiers are flattened upstream is exactly why T-063 exists. The `PATCHES.md` warning about reusing a stable per-info-hash `Arc` in `make_peer_rx` (a fresh `insert` orphans the map the live announcer writes to, leaving the UI on "Updating") is a real trap. |
| `0010-windows-pread-pwrite-exact.patch` | **A genuine upstream bug, not a visibility change.** | See below. |

### `0010` — the Windows short-read/short-write bug in librqbit

`nanotorrent/patches/0010-windows-pread-pwrite-exact.patch` (112 lines, with
tests). In `nanotorrent/vendor/librqbit/src/storage/filesystem/fs.rs`:

- `pread_exact` called `File::seek_read` (one `ReadFile`) and **discarded the
  byte count**. At or past EOF that returns `Ok(0)`, not an error, so a read of
  a file with no data reported success and left the caller's buffer untouched.
  Consequences stated in `PATCHES.md`: `FileOps::initial_check` never saw a file
  as missing/empty, so it hash-checked **every piece** — a fresh 6.5 GiB torrent
  spent ~11 s SHA-1'ing files holding nothing — and any short read hashed,
  streamed **or served to a peer** whatever stale bytes were in the buffer.
- `pwrite_all` re-wrote the whole `buf` at the same `offset` every pass while
  subtracting the written count from `remaining`, so a partial write duplicated
  data and could underflow.

The patch loops, advances buffer and offset, and maps `Ok(0)` to
`UnexpectedEof`/`WriteZero`. Three tests are included
(`pread_exact_fails_on_empty_file`,
`pread_exact_fails_past_eof_and_leaves_no_stale_bytes`,
`pwrite_all_advances_the_offset`).

**For `bit-cli`:** it is Windows-first and builds on librqbit. T-074 ("a false
hash-check pass on empty files") and T-015 ("hash checking can hang at 0
percent") are both consistent with this defect. Check which librqbit version
`bit-cli` pins and whether this is fixed upstream; if not, this patch is a
drop-in.

### BEP 52 v2 + hybrid creation on top of librqbit (T-081)

`nanotorrent/src/bittorrent/torrent_create.rs` (618 lines) exists precisely
because "librqbit only creates v1 torrents". It is the closest thing in the
corpus to the code `bit-cli create --v2/--hybrid` needs:

- `:24` `BLOCK = 16 * 1024`.
- `:121` `merkle_root` over a power-of-two leaf count; `:129` `next_pow2`.
- `:207` `hash_file_v2` — SHA-256 each 16 KiB block, **the final short block is
  hashed as-is, not zero-padded**; leaves are then padded to a power of two with
  the zero hash; the piece layer is the tree level where one node spans one
  piece, truncated to `ceil(num_blocks / blocks_per_piece)` real pieces
  ("trailing all-padding pieces are beyond the end of file and omitted");
  **files of one piece or less get an empty piece layer**, and empty files get
  no `pieces root` at all.
- `:280` `V1Hasher` with `:309` `pad_to_piece` — the hybrid path zero-pads to
  the next piece boundary and emits a **BEP 47 padding file** entry at `:457-466`
  with `attr = "p"` and path `[".pad", "<len>"]`.
- `:342` `tree_insert` builds the nested `file tree` dict; `:402` `build`
  assembles `meta version = 2`, `file tree`, and the top-level `piece layers`.
- `:381` `auto_piece_length` — start at 256 KiB, double while `total/pl > 2000`,
  cap 16 MiB. `:390` `validate_piece_length` — **power of two and ≥ 16 KiB**,
  which BEP 52 requires and v1 does not.
- A hand-rolled `Ben` bencode encoder (`:46-99`) is used because the structure
  is a recursive tree plus a dict keyed by raw 32-byte hashes; serde was
  considered and rejected. Worth noting if `bit-cli` reuses librqbit's bencode.

### MSE/PE through the seams

`nanotorrent/src/bittorrent/mse.rs` (819 lines) implements both directions
against patches 0003/0005: `:30` the 768-bit MSE DH prime, `:37`
`CRYPTO_RC4 = [0,0,0,2]`, `:38` `MAX_PAD = 512`, `:131` `client_handshake`,
`:226`/`:253` the VC synchronisation scans, `:278` `server_handshake`,
`:529` `MseTransform`, `:556` `IncomingMseTransform` (which peeks for the
plaintext protocol string first). The module doc states the policy choice
explicitly: RC4 only, advertise only RC4 in `crypto_provide`, drop a peer that
will not do RC4 — because that is what "require encryption" means.

Also present: `src/core/pico_import.rs` (one-time import of a PicoTorrent data
folder) and `migrations/` (the SQLite settings/resume schema PicoTorrent used),
useful only as a session-format reference.

No Issues or Pull Requests exist on this repository.

---

## 3. `TorrentNG` — snapetech/TorrentNG

- Upstream: <https://github.com/snapetech/TorrentNG>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\TorrentNG`
- Licence: **MIT** (`TorrentNG/LICENSE`); `TorrentNG/NOTICE` adds a
  non-affiliation statement for rTorrent/qBittorrent trademarks.
- Language: Rust, 29 crates. A modular engine plus qBittorrent / Transmission /
  Deluge / rTorrent API facades. This is the reference for `bit-cli`'s deferred
  Phase C (T-200…T-209).

### BEP 12 tier order (T-063)

Moved to [`HISTORY/RESEARCH-trackers.md`](HISTORY/RESEARCH-trackers.md) on 2026-08-24: T-063, T-064 and T-065 closed and the
behaviour is in the tool.

### Tracker backoff and announce storms

Moved to [`HISTORY/RESEARCH-trackers.md`](HISTORY/RESEARCH-trackers.md) on 2026-08-24: T-063, T-064 and T-065 closed and the
behaviour is in the tool.

### uTP (T-101)

`TorrentNG/docs/protocol/UTP.md` is an honest status document: it separates "the
packet codec works" from "the engine can carry peer-wire traffic over it", and
lists exactly what is covered by tests. The code:

- `crates/rt-utp/src/congestion.rs` — LEDBAT: `:3` `TARGET_DELAY_US = 100_000`,
  `:50` `on_ack` (base delay = running min of `timestamp_diff`; grow by
  `bytes_acked * headroom / target`, shrink by `cwnd * overshoot / queuing_delay`
  with an MTU floor), `:77` `on_timeout` halves with the same floor. Three unit
  tests at the bottom of the file.
- `crates/rt-utp/src/selective_ack.rs` — `:11` `EXTENSION_KIND = 1`, `:21`
  `from_received_offsets`, `:37` `is_acknowledged`; the doc comment states the
  bit numbering precisely ("bit 0 of the first byte acknowledges `ack_nr + 2`").
- `crates/rt-utp/src/packet.rs`, `state.rs`, `transport.rs` — header codec,
  connection-ID derivation for initiator vs acceptor, a shared-UDP
  `UtpEndpoint` that demultiplexes by (remote addr, receive connection id) so
  one socket serves many streams, and `write_all`/`read_exact` byte-stream
  helpers "which is the bridge needed for peer-wire handshakes and
  length-prefixed messages".

### fastresume (T-016, currently *blocked* in `bit-cli`)

Moved to [`HISTORY/RESEARCH-fastresume-and-idle-peers.md`](HISTORY/RESEARCH-fastresume-and-idle-peers.md) on 2026-08-24: T-016 and T-020 closed and the
behaviour is in the tool.

### Resume-file formats for real clients (interop)

`TorrentNG/crates/rt-migrate/src/export.rs` — `:42` `ExportFormat`, and one
builder per client, each naming the exact bencode keys:

- `:419` `libtorrent_resume` — `file-format: "libtorrent resume file"`,
  `file-version`, `libtorrent-version`, `save_path` **and** `qBt-savePath`,
  `total_uploaded`/`total_downloaded`, `trackers` as tiers, `info-hash`,
  `qBt-category`, `qBt-tags`, `pieces` (**one byte per piece**, `:588`
  `have_to_piece_bytes`), and `unfinished` as a dict of piece → block list.
- `:465` `transmission_resume` — `destination`, `progress.have` as a **bitfield**
  (`:593` `have_to_bitfield`), `uploaded`, `downloaded`, `corrupt`.
- `:480` `rtorrent_resume` — `complete`, `directory`, `d.custom1`,
  `timestamp.finished`.
- `:499` `utorrent_entry` (`caption`, `path`, `label`, `have`), `:515`
  `biglybt_entry` (`save_dir`, `uploadedEver`, `downloadedEver`).

`TorrentNG/testdata/migration-corpus/` holds real state files for
`qbittorrent`, `deluge`, `rtorrent`, `transmission`, `utorrent`, `biglybt`,
`tixati` and a `generic` set, with `manifest.toml`. That corpus is the fastest
route to a `bit-cli` "import an existing library" feature.

### Storage scheduling (T-017, T-018)

- `crates/rt-storage/src/io_class.rs:7` `IoClass` — `Metadata < Recheck <
  MoveCopy < PeerWrite < PeerRead < Foreground`, with **per-class concurrency
  caps that differ for HDD and SSD** (`:24` `hdd_concurrency`, `:36`
  `ssd_concurrency`: peer reads 4 vs 16, recheck 1 vs 4). The stated invariant —
  "peer reads must never be starved by bulk recheck or background copy" — is
  exactly the failure `bit-cli`'s `bench disk` is built to expose.
- `crates/rt-storage/src/elevator.rs:223` `coalesce_ready_ops` / `:251`
  `can_merge` — adjacent ready ops on the same file are merged into one
  dispatch. The tests are explicit that **reads are offset-sorted and coalesced
  per file while writes are ordered but not coalesced**
  (`ready_reads_are_offset_sorted_and_coalesced_per_file`,
  `writes_are_ordered_but_not_coalesced`). That is a direct, tested answer to
  T-018.
- `crates/rt-storage/src/handle_cache.rs` — path+access-keyed LRU of open
  descriptors, bounded to a fraction of `RLIMIT_NOFILE`, with a time-based idle
  sweep. Its doc names the reason positioned I/O makes a shared fd safe: "no
  per-op `seek`, so concurrent readers/writers do not race a file cursor."
  Compare `bit-cli`'s `--max-open-files`.

### Metainfo parsing

`crates/rt-metainfo/src/parse.rs`:

- `:20` `parse_torrent` — `MAX_TORRENT_BYTES` guard first; **the info dict is
  hashed from its recorded byte span in the original buffer**
  (`decode_torrent_info_span`), never re-encoded. That is the technique that
  keeps an info hash stable through an edit, i.e. what `bit-cli`'s exit code 15
  ("would change the info hash") is protecting.
- `is_v2 = meta_version == Some(2) && has_file_tree`; hybrid computes SHA-1 and
  SHA-256 over the same span.
- `:368` `parse_webseeds` handles `url-list` as **either** a bencoded string or a
  list, `:387` trims, de-duplicates and caps at `MAX_WEBSEED_URLS = 4096`
  (`:16`). The unit test `parse_webseeds_accepts_string_and_list_forms` (`:621`)
  locks both shapes in.
- `get_positive_power_of_two_u64` is applied to `piece length` — note this is
  stricter than v1 requires.

### API parity surface (T-201)

`crates/rt-api-qbit/src/router.rs` enumerates the qBittorrent WebAPI v2 surface
actually implemented: `/auth/login|logout`, `/app/*` (preferences,
buildInfo, networkInterfaceList, setCookies, shutdown…), `/torrents/*`
(add, addPeers, addTrackers, addTags, categories, delete, export, editTracker,
priorities…), `/sync/maindata`, `/sync/torrentPeers`, `/log/main`,
`/log/peers`, `/rss/*` (addFeed, rules, matchingArticles…), `/search/*`
(plugins, results, start/stop). `TorrentNG/crates/rt-api-transmission/` covers the Transmission RPC field
names, and `TorrentNG/crates/rt-api-deluge/` and
`TorrentNG/crates/rt-api-rtorrent/` the other two. `docs/CLIENT_COMPATIBILITY_MATRICES.md` and `docs/INTEROP_MATRIX.md`
record what is complete, partial or a no-op shape.

### Architecture spec

`TorrentNG/torrentng_engine_rewrite_spec.md` (1563 lines) is a crate-by-crate
specification: §4.1 `rt-bencode` … §4.17 `rt-migrate`, then §5 API/SDK design,
§6 test harness, §7 benchmarks, §8 security model, §9 phased implementation
(Phase 11 is DHT/PEX/LSD/uTP). §1.1 sets hard numbers — 15 000 torrents, 200+ TB,
cold start under 120 s, idle RSS under 2.5 GB, "restart must not cause tracker
announce storms", "crash recovery must not force global recheck". If `bit-cli`
ever un-defers Phase C, this is the document to read first.
`TorrentNG/CLAUDE.md` adds two things the spec does not: the exact
qBittorrent WebAPI endpoint list that "must pass *arr/autobrr integration
tests" (login, `app/version`, `app/webapiVersion`, `torrents/info|add|pause|
resume|delete|recheck|reannounce|trackers|editTracker|files|filePrio|
setCategory|addTags`, `sync/maindata`, `transfer/info` — that is the minimum
surface for the automation ecosystem, and a much shorter list than the full
router), and hard release gates: 15k torrents at first paint under 3 s, filter
under 500 ms, `/torrents/info` under 500 ms at 50k synthetic, `/sync/maindata`
delta under 50 ms under churn.

Issues are disabled on this repository and its PRs are Dependabot-only; nothing
of research value there.

---

## 4. `superseedr` — Jagalite/superseedr

- Upstream: <https://github.com/Jagalite/superseedr>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\superseedr`
- Licence: **MIT** (`superseedr/LICENSE`).
- Language: Rust + Tokio, ~160 k lines. TUI client with its own DHT, uTP, v2
  support, RSS, cluster mode and a Docker/libtorrent interop harness.
- Note: the upstream `README.md` was end-user/marketing material and was
  replaced during cleaning by a technical index; the substance lives in
  `superseedr/docs/architecture.md`, `docs/tuning.md`, `docs/cli.md`,
  `docs/shared-config.md` and `docs/synthetic-benchmark.md`.

### BEP 42 — DHT security extension, dependency-free

`superseedr/src/dht/bep42.rs`:

- `:22` `classify_ipv4` → `Compliant` / `NonCompliant` / `ExemptLocal`.
- `:41` `secure_node_id_for_ipv4(ip, entropy)` — writes the 21-bit prefix into
  the first three bytes (`entropy[2] = (prefix[2] & 0xf8) | (entropy[2] & 0x07)`)
  and keeps `r` in the last byte.
- `:104` `ipv4_is_exempt`, `:117` `id_prefix_ipv4` (mask `0x030f3fff`, `r << 29`),
  `:130` `crc32c` — **a 12-line table-free CRC32C**, so BEP 42 costs no new
  dependency.
- `classify_ipv6` at `:95` is a deliberate no-op; IPv6 BEP 42 is unimplemented.
- Also `is_secure_public_candidate` and `same_public_identity_group`, used to
  group peers by verified public identity — a Sybil-resistance primitive.

### v1 / v2 / hybrid identity (T-134)

`superseedr/agentic_plans/v2_identity_lossiness_review_2026-04-14.md` (106 lines)
is a careful write-up of the exact problem `bit-cli` has open. Findings:
`TorrentManager::from_torrent` derives **one** `info_hash` — SHA-1 for v1 and
hybrid, **SHA-256 truncated to 20 bytes for pure v2** — and the tracker client
uses that single value for both HTTP and UDP. The document's conclusion is that
the architectural fix is *not* "use 32 bytes everywhere" but to separate
internal identity, wire-facing identity and control-plane keying. It also
records what the local harness does and does not prove (the bundled HTTP tracker
accepts any 20-byte string, so it proves harness interop, not protocol
correctness).

### v1/v2/hybrid fixtures

`superseedr/integration_tests/torrents/` — 16 real `.torrent` files:
subdirectories `v1`, `v2` and `hybrid`, each containing
`single_4k.bin.torrent`, `single_8k.bin.torrent`, `single_16k.bin.torrent`,
`multi_file.torrent` and `nested.torrent`, plus one extra,
`superseedr/integration_tests/torrents/v1/single_25k.bin.torrent`. Matching payload
descriptors are in `integration_tests/test_data/`. These are the cheapest way
for `bit-cli` to get v2 and hybrid coverage into `cargo test`.

### Cross-client interop harness

`superseedr/integration_tests/` is a Dockerised matrix worth copying wholesale:

- `harness/clients/{qbittorrent,transmission,superseedr}.py` — one adapter per
  client; `harness/scenarios/*.py` — seed/leech pairs in both directions.
- `libtorrent_lab/scenarios/*.json` — 16 scenarios including
  `basic_ul_dl_tcp_only`, `basic_ul_dl_utp_only`,
  `libtorrent_to_superseedr_hybrid_nested`,
  `superseedr_to_libtorrent_v2_multi_file`,
  `libtorrent_dual_stack_to_superseedr_all`.
- `superseedr/integration_tests/docker/tracker.py` — a local HTTP tracker;
  `run_interop.sh`, `run_libtorrent_lab.sh` alongside it.
- `README.md` states the pass criterion plainly: no missing files and no
  hash/content mismatches in the leech output, validated against a manifest.

`bit-cli` currently interops against `aria2c` and `rqbit` via PowerShell, and
skips the web-seed case for `rqbit` because it has no BEP 19. Adding a
libtorrent/qBittorrent leg with v1/v2/hybrid and TCP/uTP variants is the obvious
next step, and this is the template.

### Merkle verification

`superseedr/src/torrent_manager/merkle.rs`: `:7` `verify_merkle_proof`,
`:52` `compute_v2_piece_root` (BLOCK_SIZE 16 384). The file is 541 lines of
which most are regression tests with self-documenting names —
`verify_tail_padding_fix`, `test_v2_small_file_less_than_piece_len`,
`test_v2_merkle_parity_regression`, `test_v2_small_file_root_mismatch_regression`,
`test_compute_root_3_blocks_padding`, `test_v2_verification_context_padding_consistency`.
Each of those names is a v2 edge case someone actually got wrong.

### uTP

`superseedr/src/networking/utp.rs` — the most complete uTP constants block in the
corpus (`:31-67`): `MIN_PACKET_SIZE 150`, `MAX_PACKET_SIZE 2560`,
`NETWORK_MAX_PACKET_SIZE 1200`, `RECEIVE_WINDOW = 256 KiB`,
`MAX_INFLIGHT_PACKETS 64`, `INITIAL_RETRANSMIT_TIMEOUT 1 s` /
`MIN_RETRANSMIT_TIMEOUT 500 ms`, `DELAYED_ACK_DELAY 5 ms` with a 4-packet
threshold, `MAX_RETRANSMITS 8`, `DELAY_TARGET_MICROSECONDS 100_000`,
`BASE_DELAY_WINDOW 120 s` bucketed at 1 s, `MAX_CWND_INCREASE_BYTES_PER_RTT
3000`, `LOSS_WINDOW_FACTOR 0.5`, `MAX_OUT_OF_ORDER_PACKETS 256`, plus
`SUPERSEEDR_UTP_TUNING` as a runtime override.

### Offline/online control plane (Phase C)

`superseedr/src/control_service.rs` — a control request is either sent to a
running instance or, when none is running, **applied offline against the
settings/catalog** (`plan_control_request` `:1154`,
`apply_offline_control_request` `:1364`, `prepare_offline_move_transaction`
`:1069`, `build_offline_purge_plan` `:541`, `ensure_destination_space_for_move`
`:798`). `bit-cli` is deliberately daemonless; this shows a middle path where
the same verbs work with or without a session.

### Web seeding — a negative example

`superseedr/src/networking/web_seed_worker.rs` (141 lines) is the
"web-seed-as-virtual-peer" bridge, i.e. `bit-cli`'s own Candidate A-prime.
Reading it is useful mainly for what it does *not* do: it fetches
`GET {url}` with a byte range computed as `index*piece_length + begin` **against
the whole payload**, so it only works for single-file torrents with an exact
URL; there is no BEP 19 name/path composition, no per-file mapping, and any
non-2xx disconnects the source outright. It also documents that `BulkCancel` is
a no-op because the batch loop cannot cancel mid-request.

### Other

- `src/integrity_scheduler.rs` — continuous background integrity probing with
  fast recovery reprobes; relevant to `bit-cli verify`.
- `src/tuning/mod.rs` — adaptive limit control.
- `proptest-regressions/` — retained; these are recorded proptest counterexamples.
- `scripts/extract_merkle.py`, `scripts/generate_integration_torrents.py`,
  `scripts/hash.py`, `scripts/summarize_dht_soak.py` — small, reusable.

### Issues / PRs

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 170](https://github.com/Jagalite/superseedr/issues/170) | CLOSED | When downloading, files are not grouped under one sub-directory | Multi-file magnet contents landed directly in `~/Downloads` instead of under the torrent name. Identical to `bit-cli` T-036 ("a multi-file torrent with one file lands without its directory"), which is marked done — this is the user-visible shape of that bug. |
| [Issue 236](https://github.com/Jagalite/superseedr/issues/236) | CLOSED | Eliminate data validation on startup | "Validates tens or even hundreds of gigabytes on startup." The proposed design — an `<info-hash>.<chunk>` marker file created before a chunk write and deleted after, so only marked chunks are refetched — is a cruder cousin of TorrentNG's durability watermark, and the same motivation as T-016. |
| [Issue 297](https://github.com/Jagalite/superseedr/issues/297) | OPEN | Traffic encryption | States the interop consequence plainly: without MSE you cannot exchange traffic with clients configured to *require* encryption. `bit-cli` has no MSE at all. |
| [Issue 319](https://github.com/Jagalite/superseedr/issues/319) | OPEN | WebTorrent-compatible peer support | Scopes what WebTorrent actually requires: WebRTC data channels, `ws://`/`wss://` tracker announces, coexistence with TCP/uTP peers in one swarm, and a browser-peer test harness. |
| [Issue 240](https://github.com/Jagalite/superseedr/issues/240) | CLOSED | DHT Implementation | Why they dropped the `mainline` crate: IPv6, a plan/budget-based Kademlia scheduler, continuous searching across many torrents. Relevant background for `bit-cli`'s DHT items. |
| [Issue 142](https://github.com/Jagalite/superseedr/issues/142) | CLOSED | Hangs for minutes on startup if DNS server does not resolve | Tracker hostname resolution on the startup path with no deadline. `bit-cli trackers` announces for real and has a `--timeout`; worth an explicit resolution deadline. |
| [Issue 237](https://github.com/Jagalite/superseedr/issues/237) | CLOSED | File written to even though if paused | Pause must fence in-flight writes, not just stop requesting. |
| [PR 287](https://github.com/Jagalite/superseedr/pull/287) / [PR 267](https://github.com/Jagalite/superseedr/pull/267) | MERGED | Peer manager / Peer transport abstraction | The refactor that made TCP and uTP interchangeable behind one transport trait — the shape `bit-cli` needs to make BEP 29 testable (T-101). |
| [PR 299](https://github.com/Jagalite/superseedr/pull/299) / [PR 298](https://github.com/Jagalite/superseedr/pull/298) | MERGED | Strict native network interface binding / randomized listen ports | Both are `bit-cli` `--port` adjacent (T-023). |

---

## 5. `fx-torrent` — yoep/fx-torrent

- Upstream: <https://github.com/yoep/fx-torrent>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\fx-torrent`
- Licence: **MIT** (`fx-torrent/LICENSE`).
- Language: Rust. **The widest BEP checklist in the corpus.**

Claimed in `fx-torrent/README.md` and, where checked, present in source: BEP 3,
4, 5, 6, 7, 9, 10, 11, 12, 14, 15, 19, 20, 21, 24, 29, 32, 33, 40, 42, 43, 44,
47, 48, 51, 53, 54, 55. BEP 52 is explicitly still WIP.

### The BEPs `bit-cli` does not have

- **BEP 55 holepunch** — `src/peer/extension/holepunch.rs` (678 lines). `:14`
  `HolepunchMessage { msg_type, addr_type, addr, port, err_code }`, `:149`
  `NAME = "ut_holepunch"`, message types `Rendezvous/Connect/Error`. A working
  implementation, not just the codec.
- **BEP 54 `lt_donthave`** — `src/peer/extension/donthave.rs`, `:19`
  `NAME = "lt_donthave"`; a 4-byte big-endian piece index that clears a bit in
  the peer's bitfield. Twenty lines of protocol; it is the cheapest correctness
  win in the list for a client that tracks availability.
- **BEP 33 DHT scrape** — `src/bloom_filter.rs`. `:5` `has_bits` / `:20`
  `set_bits` use the first 4 bytes of the key as two little-endian `u16`
  indices; `:46` `len()` estimates population as `-(m/k)·ln(zero/m)` with
  `k = 2`; `:93` `count_zero_bits` uses a 16-entry nibble table. The DHT side is
  `src/dht/tracker.rs:449` `scrape_peers` and `:2469` `scrape_info_hashes`.
- **BEP 51 DHT infohash indexing** and **BEP 44 put/get** — `src/dht/krpc.rs`
  handles `ping`, `find_node`, `get_peers`, `announce_peer`,
  `sample_infohashes` (`:18`), `put` (`:19`) and `get` (`:20`) in one message
  enum. `src/dht/tracker.rs:1736` even logs "detected spoofed
  sample_infohashes".
- **BEP 47** — `fx-torrent/src/peer/webseed/http.rs:223-226` skips files whose
  attributes contain `FileAttributeFlags::PaddingFile` when planning a web-seed
  fetch. If `bit-cli` implements BEP 47, its web-seed scope selectors must do
  the same or it will request bytes no server has.

### Piece picking as pluggable strategies

`src/piece_picker/strategy/` — `rarest_first.rs`, `sequential.rs`,
`priority.rs`, `suggested_only.rs` behind a common trait, composed in
`src/piece_picker/picker.rs`. `bit-cli` has `--piece-selector sequential |
in-order`; this is the shape for adding more without a rewrite, and
`suggested_only` is the BEP 6 `Suggest` consumer.

### Extension points

The README documents three user-implementable traits — `peer::extension::Extension`
(BEP 10 messages and handshake), `storage::Extension` (read/write backends), and
a `PiecePicker` with per-strategy composition. `src/storage/parts_file.rs` is a
partial-file storage backend.

### Web seeding — read it as a caution

`src/peer/webseed/http.rs:195` `request_piece` allocates
`let mut buffer = vec![0u8; len]` where `len = blocks.len()` — **the number of
blocks** — then indexes that buffer by *byte* offsets (`buffer[cursor..cursor +
body.len()]`, and later `&buffer[block_start..block_end]`). Unless every piece
has exactly as many bytes as it has blocks, that is a length confusion. Cited
here as an edge case to avoid, not to copy. The rest of the file is sound:
`:303` `create_request_url` strips a trailing `/` and appends escaped path
segments; the range is computed per-file as
`piece.offset - file.torrent_offset`.

### Issues / PRs

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 98](https://github.com/yoep/fx-torrent/issues/98) | OPEN | `Piece`s are written entirely into the file that they start in, even if they span multiple files | The reporter's symptom is that in a multi-file FLAC album **only the first file is playable**, with a reproducible CC-licensed magnet in the issue body. A piece that straddles a file boundary must be split at the boundary. This is the single most valuable "do not do this" in the corpus for `bit-cli`'s storage layer, and it is exactly the case `vortex`'s `file_store` tests cover. |
| [Issue 16](https://github.com/yoep/fx-torrent/issues/16) | CLOSED | BEP5 - Allow extended transaction ID's | The DHT transaction id was fixed at 2 bytes; BEP 5 only says 2 bytes is *typically* enough. Real nodes send 4, producing `Invalid Length: 4 (expected: a byte array of length 2)`. A one-line interop trap. |
| [Issue 71](https://github.com/yoep/fx-torrent/issues/71) | CLOSED | Tracker - Http announce invalid request | `https://torrent.ubuntu.com/announce` answered `400 Bad Request` with `"you sent me garbage - invalid event"`. Fixed by PR 74, "tracked `paused` support state within http tracker clients": a client-internal state must not be sent as a BEP 3 `event`. `bit-cli` announces `started`/`completed`/`stopped`; keep it to those three. |
| [Issue 66](https://github.com/yoep/fx-torrent/issues/66) | CLOSED | uTP - Packet parsing | `failed to parse packet (len 30) … failed to fill whole buffer` in the extension-chain parser. Fixed by PR 68. Relevant to T-101. |
| [Issue 21](https://github.com/yoep/fx-torrent/issues/21) | CLOSED | DHT - Incorrect error response parsing | KRPC error responses are a list `[code, message]`, not a dict; getting it wrong turns every error into a parse failure. |
| [Issue 99](https://github.com/yoep/fx-torrent/issues/99) | CLOSED | File priorities are not being updated correctly | Relevant to `--select-file`. |
| [Issue 30](https://github.com/yoep/fx-torrent/issues/30) | CLOSED | DHT - Add option to disable info hash scraping/indexing | BEP 51 participation should be opt-out. |
| [PR 64](https://github.com/yoep/fx-torrent/pull/64) | MERGED | BEP55 - The HolePunch extension | Implementation commit for the holepunch module above. |
| [PR 76](https://github.com/yoep/fx-torrent/pull/76) | MERGED | Piece Picker (closes #20, "implement piece picker logic from libtorrent") | The strategy split described above. |
| [PR 79](https://github.com/yoep/fx-torrent/pull/79) | MERGED | Bugfix: Upload slot flow deadlocking torrent tick | Choke/unchoke bookkeeping that blocks the whole torrent loop — a hazard for any tick-based seeder (T-083). |

---

## 6. `mkbrr` — autobrr/mkbrr

- Upstream: <https://github.com/autobrr/mkbrr>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\mkbrr`
- Licence: **MIT** (`mkbrr/LICENSE`).
- Language: Go. Pure creation/modify/verify tool — the closest analogue to
  `bit-cli create`, `bit-cli edit` and `bit-cli verify`.

### Parallel piece hashing

`mkbrr/torrent/hasher.go`:

- `:38` `optimizeForWorkload` — read-buffer size and worker count chosen from
  file count and *average* file size: 64 KiB/1 worker for a single sub-1 MiB
  file, 256 KiB for average < 1 MiB, 1 MiB for < 10 MiB, 4 MiB for < 1 GiB,
  8 MiB above; worker count never exceeds the piece count.
- `:221` `hashPieceRange` — each worker owns a **contiguous range of pieces** and
  its own `[]*fileReader`, seeking only when its position drifts; a piece that
  spans files is read across them inside the worker. Progress is an atomic
  counter sampled every 200 ms.
- `:340` `buildPieceLayout` precomputes `lastPieceLength` and, for every piece,
  the index of the file it starts in — so a worker can jump straight to its
  first piece without scanning.
- `reusablePieces` (a `map[int][]byte` supplied by the update path) lets a piece
  be copied instead of re-hashed.

`bit-cli`'s `bench leech` reports "verification: N pieces, X GiB/s"; this is the
structure that makes that number good, and it is directly portable.

### Unicode normalisation (T-103 adjacent, and a real interop bug)

`mkbrr/torrent/normalize.go`:

- `:18` `decomposed(s)` — true only when `s` differs from its NFC form *purely*
  by combining marks; canonical singletons (U+212B ANGSTROM SIGN, CJK
  compatibility ideographs) and composition exclusions are deliberately excluded
  because "those bytes are what the filesystem genuinely holds, not
  decomposition artifacts".
- `:58` `nfcPath(dir, rel)` — rewrites to NFC **only if `Lstat` proves both
  spellings are `os.SameFile`**, so a filesystem that genuinely stores NFD is
  left alone.
- `:80` `pathKey` / `:86` `resolveNormalized` — comparison-only normalisation so
  a torrent written in one form still matches files stored in the other.

The motivating report, [Issue 182](https://github.com/autobrr/mkbrr/issues/182)
(CLOSED, fixed by [PR 183](https://github.com/autobrr/mkbrr/pull/183)): torrents
created on macOS against an SMB mount from a Synology NAS wrote **NFD** names
while the origin stored **NFC**. macOS path lookup is normalisation-insensitive,
so the torrent verified clean locally *including with `mkbrr check`* — the
breakage only appeared on Linux/Windows, i.e. after the torrent was public.
Real case: 41-file season pack, 19 files with accented names, all showing as
missing. `bit-cli create` runs on Windows and macOS and has no NFC/NFD handling;
this is a lint or a normalisation step it is currently missing.

### Piece-length selection and tracker rules

- `mkbrr/torrent/create.go:119` `calculatePieceLength` — default range table,
  then a tracker-specific override, clamped to `[2^16, 2^24]` by default and up
  to `2^27` when the user asks.
- `:62` `calculatePieceLengthFromTarget` — derive the exponent from a target
  piece count via `bits.Len64(totalSize/targetCount) - 1`, then clamp; a
  tracker cap is a **hard ceiling the user may lower but not exceed**.
- `mkbrr/internal/trackers/trackers.go:319` `DefaultPieceSizeRanges` — 14 bands
  from 32 KiB (≤64 MB) to 128 MiB (>128 GB).
- `:310` `GetTrackerMaxPieceLength`, `:336` `GetTrackerPieceSizeExp`, `:373`
  `GetTrackerMaxTorrentSize` — per-tracker rules keyed by announce host,
  including exact published tables (e.g. `passthepopcorn.me` with nine size
  bands, `anthelion.me` with a 250 KiB `.torrent` size limit). If `bit-cli`
  wants a `create` that private-tracker users can actually use, this table is
  the prior art.

### Editing without disturbing the info hash

- `mkbrr/torrent/update.go:210` `decodeTorrentRoot` — decodes into
  `map[string]bencode.Bytes` and **tolerates trailing whitespace/NUL bytes**
  (`ErrUnusedTrailingBytes` is accepted when the remainder is only
  `' '`, `\t`, `\r`, `\n`, `0`). Compare anacrolix Issue 992, where the same
  input is a hard error.
- `mkbrr/torrent/modify.go:63` `ModifyTorrent` — changes are collected as
  `infoChange` entries and applied **via the raw map**, with the comment
  "preserving any custom keys (e.g. entropy) that the typed struct would drop".
  A typed round-trip silently deletes unknown info keys and changes the info
  hash; this is the fix.
- `mkbrr/torrent/update.go:731` `writeTorrentAtomically` — temp file in the destination
  directory, permissions copied from the existing file, `rename` over the top.
- `mkbrr/torrent/update.go:445` `findReusablePieces` and `mkbrr/torrent/update.go:664` `preserveMappedFileInfoKeys` —
  how a rename-only update reuses hashes and carries per-file keys across.

### Ignore rules

`mkbrr/torrent/ignore.go:12` `ignoredPatterns` — `.torrent`, `.ds_store`,
`thumbs.db`, `desktop.ini`, `zone.identifier`; `:21` `ignoredDirNames` —
`@eadir` (Synology). Plus doublestar glob support with brace-aware splitting.

### Issues / PRs

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 182](https://github.com/autobrr/mkbrr/issues/182) | CLOSED | NFD-decomposed filenames from macOS network mounts | See above. The best-documented cross-platform torrent-creation bug in the corpus. |
| [PR 154](https://github.com/autobrr/mkbrr/pull/154) | MERGED | fix(check): use torrent-level byte offsets for multi-file verification | Mapped files got *compacted* offsets that skipped missing files, while verification used torrent-level offsets — so **every piece after a missing-file gap was reported bad and completion showed 0 %** even where data was intact. `bit-cli verify` reports per-piece results; this is the exact way to get them all wrong. |
| [Issue 67](https://github.com/autobrr/mkbrr/issues/67) | CLOSED | Some torrents will not hash correctly | ~6 of 150 torrents failed recheck when created by mkbrr but passed when created by mktorrent or dottorrent, on the same files and settings. A reminder that "my hasher agrees with itself" is not the test — cross-tool round-trip is, which is what `bit-cli`'s `interop-roundtrip.ps1` does. |
| [Issue 112](https://github.com/autobrr/mkbrr/issues/112) | OPEN | BitTorrent v2 format (BEP-0052) | Useful for the *decision*, not the code: the requester's own summary is "Is it useful, like practically used by many people? Not really", while noting v2 gives each file a stable merkle hash and 16 KiB re-download granularity. Worth weighing against T-081's priority. |
| [PR 179](https://github.com/autobrr/mkbrr/pull/179) | MERGED | feat(torrent): reuse hashes when updating files | The `reusablePieces` path above. |
| [PR 124](https://github.com/autobrr/mkbrr/pull/124) | MERGED | Ensure multi-tracker torrents use distinct tiers | BEP 12: multiple `--tracker` flags must not all land in tier 0. |
| [Issue 159](https://github.com/autobrr/mkbrr/issues/159) / [PR 161](https://github.com/autobrr/mkbrr/pull/161) | CLOSED / MERGED | tracker.torrentleech.org piece-size cap raised from 2^24 to 2^27 | Shows these tracker caps are maintained against real published limits and do drift. |
| [PR 147](https://github.com/autobrr/mkbrr/pull/147) | MERGED | support path-based glob patterns | `--exclude`/`--include` semantics for `bit-cli create`. |

---

## 7. `intermodal` — casey/intermodal

- Upstream: <https://github.com/casey/intermodal>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\intermodal`
- Licence: **CC0-1.0** (`intermodal/LICENSE`, "Creative Commons Legal Code /
  CC0 1.0 Universal"). This is the tree `bit-cli`'s README already credits for
  torrent creation, linting and the environment-injection test pattern.
- Language: Rust.

### Lints — the three `bit-cli` does not have

`intermodal/src/lint.rs:7-10` — `PrivateTrackerless`, `SmallPieceLength`,
`UnevenPieceLength`, kebab-cased by `strum`; `src/linter.rs` is the
allow/deny set. `bit-cli` has `windows-path` and `case-collision` with
`--allow <LINT>`; the same mechanism, three more checks.

Two more proposed upstream and worth adopting:

- [Issue 499](https://github.com/casey/intermodal/issues/499) (OPEN) — "Add lint
  for torrents with more than 65535 pieces": **µTorrent refuses to open them.**
- [Issue 358](https://github.com/casey/intermodal/issues/358) (OPEN) — "Add lint
  for large piece sizes", with 16 MiB as the practical upper limit.

### Piece-length selection

`src/piece_length_picker.rs:10` `from_content_size`:
`2^(ceil(log2(size))/2 + 4)`, clamped to `[16 KiB, 16 MiB]`. `:31`
`metainfo_size` predicts the resulting `pieces` blob size. The book explains the
constraints: powers of two and ≥ 16 KiB **for BEP 52 compatibility**, and 16 MiB
max because larger has been reported to break clients.

This is a fourth distinct algorithm alongside anacrolix's doubling loop, mkbrr's
band table and nanotorrent's `auto_piece_length`; `bit-cli` should pick one
deliberately and document it.

### The book — reference documentation

`intermodal/book/src/bittorrent/`:

- `bep-support.md` — a **complete BEP 0–55 support matrix** with per-BEP status
  and links to the tracking issue for every unimplemented one. The single most
  useful page in the corpus for planning `bit-cli`'s own coverage table.
- `piece-length-selection.md` and `piece-length.md` — the reasoning, with the
  factors on each side (smaller: faster first upload, less thrown away on
  corruption; larger: less protocol overhead, smaller metainfo, better seek
  ratio on spinning disks).
- `udp-tracker-protocol.md` (207 lines) — the BEP 15 wire format written out.
- `metainfo-utilities.md`, `distributing-large-data-sets.md`, `prior-art.md`.

### `create` option surface

`src/subcommand/torrent/create.rs` (3196 lines) — options `bit-cli create` may
want for parity: `--sort-by <KEY:ORDER>` (deterministic file ordering),
`--md5`, `--glob` (include/exclude), `--include-hidden`, `--include-junk`,
`--ignore` (honours `.gitignore`, `.ignore`, `.git/info/exclude` and
`core.excludesFile`), `--follow-symlinks`, `--node HOST:PORT` (BEP 5 `nodes`),
`--peer` (magnet `x.pe`), `--update-url` (BEP 39 `update-url` in the info dict),
`--source`, `--announce-tier`, `--dry-run`, `--link`, and — for reproducible
builds — `--no-created-by` and `--no-creation-date`.

Also in-repo: `src/tracker/` (a UDP tracker client used by
`imdl torrent announce`), `src/peer/` (handshake + extended handshake),
`src/verifier.rs`, `src/magnet_link.rs`.

### Issues

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 534](https://github.com/casey/intermodal/issues/534) | CLOSED | imdl fails to deserialize torrents with `.utf-8` key variants | µTorrent writes both `name` and `name.utf-8` (and `path`/`path.utf-8`) with different encodings. Not in BEP 3, but universal in practice. The reporter's conclusion — "if the `.utf-8` variant exists, use it preferentially" — is what anacrolix does (`Info.BestName()`) and what parse-torrent does. **This is the practical shape of `bit-cli` T-103.** |
| [Issue 454](https://github.com/casey/intermodal/issues/454) | OPEN | Failed to deserialize torrent metainfo | `bencode encoding corrupted (Keys were not sorted)` on a torrent created by uTorrent/2210 that "works fine in normal torrent clients". A strict bencode reader will reject real torrents. Decide deliberately whether `bit-cli` is strict on read (and if so, say so in the error). |
| [Issue 553](https://github.com/casey/intermodal/issues/553) | CLOSED | imdl torrent link creates invalid magnet | Tracker URLs in `tr=` must be percent-encoded; `urn:btih:` is conventionally left unencoded. Directly applicable to `bit-cli magnet`. |
| [Issue 565](https://github.com/casey/intermodal/issues/565) | CLOSED | `imdl torrent dump` produces invalid json | The binary `info.pieces` blob was emitted unquoted. `bit-cli --json` must encode binary fields; worth a schema test. |
| [Issue 92](https://github.com/casey/intermodal/issues/92) / [Issue 93](https://github.com/casey/intermodal/issues/93) | OPEN | BEP 17 HTTP seeding / BEP 19 GetRight web seeds | Both frame the same two tasks: let the user populate `httpseeds` / `url-list`, **and verify the server at the URL actually serves those requests**. That second half is precisely `bit-cli webseed test` / `webseed probe` — `bit-cli` already has the feature intermodal is still asking for. |
| [Issue 99](https://github.com/casey/intermodal/issues/99) | OPEN | BEP 47 padding files | "Should probably be enabled by default." |
| [Issue 101](https://github.com/casey/intermodal/issues/101) | OPEN | BEP 52 v2 | Tracking issue. |
| [Issue 495](https://github.com/casey/intermodal/issues/495) | OPEN | Parallel Hashing | intermodal's hasher is still single-threaded; mkbrr's is the answer. |

---

## 8. `gosh-dl` — goshitsarch-eng/gosh-dl

- Upstream: <https://github.com/goshitsarch-eng/gosh-dl>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\gosh-dl`
- Licence: **MIT** (`gosh-dl/LICENSE`, © 2025 goshitsarch-eng).
- Language: Rust. Embeddable HTTP + BitTorrent download engine.
- Per instruction, only the BitTorrent-related parts were researched.

### BEP 17 auto-detection — the answer to T-004

Moved to [`HISTORY/RESEARCH-web-seed-style.md`](HISTORY/RESEARCH-web-seed-style.md) on 2026-08-24: T-004, T-130 and T-137 closed and the
behaviour is in the tool.

### Source lifecycle

Moved to [`HISTORY/RESEARCH-web-seed-style.md`](HISTORY/RESEARCH-web-seed-style.md) on 2026-08-24: T-004, T-130 and T-137 closed and the
behaviour is in the tool.

### Other BT modules

`src/torrent/`: `choking.rs` (unchoke rotation + optimistic unchoke),
`dht.rs`, `lpd.rs` (BEP 14), `mse.rs` (906 lines, RC4 + DH),
`pex.rs` (BEP 11), `piece.rs`, `tracker.rs` (HTTP + UDP, `peers6` per BEP 7),
`transport.rs`, `utp/` (`congestion.rs` LEDBAT, `multiplexer.rs`, `packet.rs`,
`socket.rs`, `state.rs`). Private torrents (BEP 27) disable DHT/PEX/LPD.

### Issues / PRs

| # | State | Title | Why it matters |
|---|---|---|---|
| [PR 7](https://github.com/goshitsarch-eng/gosh-dl/pull/7) | MERGED | prevent double-counting verified bytes in endgame mode | In endgame the same piece is requested from several peers; two concurrent completions both incremented `verified_bytes`, so progress exceeded 100 %. The fix is to make the counter increment conditional on winning the `pending.remove()` race. `bit-cli` T-034 wants endgame observable — this is the accounting bug that observability exists to catch. |
| [Issue 6](https://github.com/goshitsarch-eng/gosh-dl/issues/6) | CLOSED | Progress shows >100% due to counting downloaded, not verified, bytes | Same theme from the user's side: 331.5 MB / 263.6 MB = 125.8 %. `bit-cli` T-139 ("a resumed download charges its existing bytes to the swarm") is the same class of accounting error. Report verified bytes, count network bytes separately. |
| [PR 9](https://github.com/goshitsarch-eng/gosh-dl/pull/9) | MERGED | Verify existing files when metadata received for magnet links | The torrent-file path called `verify_existing()`; the magnet path created the `PieceManager` and never did, so a magnet resume restarted from scratch. `bit-cli` accepts magnets and bare info hashes and has `--hash-check-only`; the metadata-arrival hook is easy to miss. |
| [PR 5](https://github.com/goshitsarch-eng/gosh-dl/pull/5) | MERGED | add WSS tracker support and fix async DNS resolution | WebTorrent tracker groundwork plus a DNS fix. |
| [Issue 4](https://github.com/goshitsarch-eng/gosh-dl/issues/4) | CLOSED | macOS: peer discovery fails due to socket permission and DNS errors (DHT/UDP/LPD) | On macOS, DHT and LPD UDP sockets failed with `Operation not permitted` and UDP tracker hostnames failed to resolve, leaving torrents at 0 peers indefinitely. The suggested fix is explicit `socket2` construction with `SO_REUSEADDR` plus `SO_REUSEPORT` on macOS. `bit-cli` has a macOS CI job (T-145) and binds sockets for `trackers`, `peers` and LSD. |
| [Issue 11](https://github.com/goshitsarch-eng/gosh-dl/issues/11) | CLOSED | It lacks some features compared to aria2 | From a user migrating off aria2 RPC: the two things they missed were **batch pause/resume** and **`.aria2` control files** ("this seems to prevent resume-from-breakpoint functionality"). Concrete Phase C requirements (T-202, T-203) from a real migration. |

---

## 23. `joal`, anthonyraymond/joal

- Upstream: <https://github.com/anthonyraymond/joal>
- Local path: `reference/joal`
- Commit: `90e710ba01ac6a8665eb352a612ce4e9581483c8`, cloned 2026-08-24.
- Licence: **Apache-2.0** (`joal/LICENSE`). Not MIT. Anything taken from it is
  an independent implementation written from the observed behaviour, cited
  here with the SHA above.
- Language: Java. 823 stars, 205 issues and 61 pull requests.

This is the origin of the client profile format that every other tool in this
group uses, and it is the source of `scripts/make-client-profile.ps1`.
[T-234](../TODO/peers.md) is the entry it answers.

### What a profile is, and it is not a string

`joal/resources/clients/` holds 94 `.client` files, one per client and version.
Each is a JSON document with six parts, and every one of them is visible to a
tracker:

| part | what it decides |
| --- | --- |
| `peerIdGenerator` | the algorithm, the prefix, the suffix alphabet, and when the id is regenerated |
| `keyGenerator` | the `key` algorithm, its width, its case, and its refresh policy |
| `urlEncoder` | which bytes are left unencoded, and whether the hex is upper or lower case |
| `query` | the exact parameter names **and their order** |
| `numwant` / `numwantOnStop` | the two values, which differ |
| `requestHeaders` | the headers and their order |

`joal/resources/clients/qbittorrent-5.2.3.client` is the shape. Its query is
`info_hash`, `peer_id`, `port`, `uploaded`, `downloaded`, `left`, `corrupt`,
`key`, `event`, `numwant`, `compact`, `no_peer_id`, `supportcrypto`,
`redundant`, in that order, with `Accept-Encoding: gzip` and
`Connection: close` after the `User-Agent`.

Across the 94 files there are five algorithms and four refresh policies:
`REGEX` 90, `HASH_NO_LEADING_ZERO` 75, `HASH` 13, `RANDOM_POOL_WITH_CHECKSUM`
5, `DIGIT_RANGE_TRANSFORMED_TO_HEX_WITHOUT_LEADING_ZEROES` 5; `NEVER` 92,
`TORRENT_PERSISTENT` 78, `TIMED_OR_AFTER_STARTED_ANNOUNCE` 11,
`TORRENT_VOLATILE` 7. Six distinct query strings cover all 94.

### Three mechanisms worth reading in the source

**The Transmission peer id checksum**,
`joal/src/main/java/org/araymond/joal/core/client/emulated/generator/peerid/generation/RandomPoolWithChecksumPeerIdAlgorithm.java:86-101`.
Eleven characters are drawn from a 36 character pool at `byte % base`, then the
final character is chosen so the whole suffix sums to a multiple of the base:
`val = (total % base) != 0 ? base - (total % base) : 0`. That is verifiable
arithmetic, so a wrong Transmission peer id is detectable by calculation rather
than by statistics. Transmission's own construction is at
`libtransmission/session.cc:205` in tag `4.1.0` and agrees.

**The announce query is built by substitution and by removal**,
`joal/src/main/java/org/araymond/joal/core/client/emulated/BitTorrentClient.java:104-152`.
Two removals matter. When the event is `NONE` the whole `event=` pair is
deleted rather than sent empty, which is what a real periodic announce looks
like. `{ip}` and `{ipv6}` are filled only when the local address is of that
family, and any pair left unfilled is deleted with its leading ampersand.

**The peer id is bytes, not text.** The uTorrent and BitTorrent patterns
carry raw high bytes, written in the profile as JSON escapes:
`-UT354S-(\u00d2)(\u00ad)[\u0001-\u00ff]{10}` and
`-BT7a3S-G(\u00ad)[\u0001-\u00ff]{10}`. Those profiles set
`shouldUrlEncode: true` for the peer id, because the bytes are not URL safe.

### The version to character encoding, and where every port loses it

qBittorrent 3.3.13 is `-qB33D0-` and 3.3.16 is `-qB33G0-`
(`joal/resources/clients/qbittorrent-3.3.13.client`,
`qbittorrent-3.3.16.client`). One version component is one character, `0` to
`9` then `A` to `Z`. The committed profiles have this right.

**The script that generates them does not.**
`joal/scripts/bittorrent-client-update-detector/qbittorrent_analyzer.sh:445`
concatenates decimal: `-qB${major}${minor}${patch}0-`. For 3.3.13 that is
`-qB33130-`, nine bytes where the Azureus style prefix is eight. The same file
defines a `version_to_char` helper at `:433` and never calls it, and the helper
would be wrong anyway: it prints the ASCII character at that code point, so 13
is a carriage return rather than `D`.

Nothing has noticed because qBittorrent 4.x and 5.x have kept every component
below ten.

**`transmission.sh` has the same class of defect and the better discipline.**
`joal/scripts/bittorrent-client-update-detector/transmission.sh:79` writes
`local BASE62=($(echo {0..9} {A..A} {a..z}))`, which is 37 entries rather than
62: the whole upper case run has collapsed to a single `A`. Index 11 gives `a`
where Transmission gives `B`. Transmission has released no component above ten
either.

What that script does right is worth more than what it gets wrong. Before
deriving anything it asserts that the upstream construction is still the one it
knows, with four `grep -cF ... -lt 1` guards on the exact lines it depends on
(`:23`, `:39`, `:43`, `:47`, `:66`, `:70`), and exits when one fails. A profile
generator that cannot tell the difference between "the client did not change"
and "I could not find the line" is the one that ships a stale profile quietly.

### Two computations the qBittorrent script discards

`qbittorrent_analyzer.sh:extract_protocol_info` greps the session file for the
user agent line and the peer id line, and `libtorrent_get_key_format` decides
hex against dec from the version. `generate_client_config` at `:553` uses none
of the three: it emits a fixed template with the version interpolated into
`"qBittorrent/" + $version` and the prefix computed separately. The analysis
part of the analyser does not reach the output. That is the same shape as
`gosh-dl`'s discarded web seed style (entry 8) and `fx-torrent`'s discarded
file index (entry 5).

### What the tracker says

Fetched 2026-08-24 and cached at `.tmp/mining/joal-issues.json`: 266 items, 205
issues (48 open, 157 closed) and 61 pull requests (3 open, 58 closed).

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 269](https://github.com/anthonyraymond/joal/issues/269) | OPEN | Replicate client fingerprint ? | Asked in 2026 against the most used tool in this space, with no reply. The user wants the peer id, the version and the "protocol signature" of their own qBittorrent. **joal cannot answer it because joal has no peer wire**: it announces and never connects to a peer, so a joal profile covers the tracker request and nothing else. Every profile in this format is therefore a partial identity, and that is the single most important thing this entry records for [T-234](../TODO/peers.md). |
| [Issue 158](https://github.com/anthonyraymond/joal/issues/158) | CLOSED | will I be detected faking transmission on PC and on my router? | The maintainer: "As long as you use the same version in JOAL & on your router there is no way to tell which is which." That claim is about the announce. It is the correct claim for what joal emits and it does not extend to a client that also speaks to peers. |
| [Issue 270](https://github.com/anthonyraymond/joal/issues/270) | OPEN | Illegal character in query | An announce URL carrying a passkey with a vertical bar in it is rejected by Java's `URI`. The reporter's own workaround is to percent-encode the pipe inside the `.torrent` and fix the bencode length prefix. A tracker URL is not a URI until somebody decides it is, and `bit-cli` should know what it does with one. |
| [Issue 175](https://github.com/anthonyraymond/joal/issues/175) | CLOSED | Can't add torrent, Tracker message violates expected protocol | A commenter writes "ya i have heard ygg detects ratio master". The maintainer diagnosed a stale Docker tag and a changed tracker domain instead. Reported here as what it is: hearsay in a thread that was resolved as something else. |
| [Issue 178](https://github.com/anthonyraymond/joal/issues/178) | OPEN | partial leecher warning | The risk model from the other side: uploading hard at a swarm of partial leechers is what gets an account banned. Recorded because it is the reason `bit-cli` does not go near this. It reports what it transferred. |

### Passes

Four. WHAT: the profile format and the 94 files. MECHANISM: the three
generators and the query builder, cited above. THE HARD PART: the version to
character encoding, where both scripts are wrong and both committed profile
sets are right. AGAINST `bit-cli`: the format is announce only, so it is a
starting point for the tracker half of [T-234](../TODO/peers.md) and says
nothing about the handshake, the reserved bytes, the extension handshake, or
the message order.

### Verdict

**ADOPT.** The profile model, the guard discipline from `transmission.sh:66`,
and the refresh policies go into [T-234](../TODO/peers.md).
`scripts/make-client-profile.ps1` is the independent implementation, and it
agrees with joal's committed prefixes for qBittorrent 4.6.7, 5.0.0, 5.1.4 and
5.2.3.

### What this pass did not do

It did not read joal's web UI, its Spring wiring, or its bandwidth dispatcher,
none of which bear on client identity. It did not read all 157 closed issues;
it read the tracker listing in full and opened the five above. It did not
verify the uTorrent or BitTorrent profiles against those clients, because
neither publishes source (see entry 28).

---

## 24. `DOAL`, DylanBricar/DOAL

- Upstream: <https://github.com/DylanBricar/DOAL>
- Local path: `reference/DOAL`
- Commit: `10551f69c80b96a587a27e35068ae1565ee0c58e`, cloned 2026-08-24.
- Licence: **stated as MIT in `DOAL/README.md:264-266` and nowhere else.**
  There is no `LICENSE` file and no licence key in `go.mod`. The repository is
  a self-described fork of joal (`DOAL/README.md:5`), and joal is Apache-2.0,
  so the MIT line is a relicensing of an Apache-2.0 work with no `NOTICE` and
  no attribution beyond a link. Treated here as read only, like every other
  tree. Nothing is copied from it.
- Language: Go, 145 files. Zero issues and zero pull requests upstream.

The reason this matters more than its star count suggests: **it is the only
tool in this group that has both an announce profile and a peer wire.** joal
announces and never dials a peer. DOAL carries `DOAL/peerwire/`, so it is the
one place where the second half of a client identity is visible.

### The same 90 profiles, and a correct checksum

`DOAL/clients/` holds 90 `.client` files in joal's format. The generator is
gone; the files were carried across.

`DOAL/announce/client_emulator.go:426` computes the Transmission checksum as
`(cfg.Base - total%cfg.Base) % cfg.Base`, which agrees with
`libtransmission/session.cc:205`. Its test asserts the right invariant rather
than the output:
`DOAL/announce/client_emulator_test.go:312` checks `total%36 != 0` over the
generated suffix, so it holds for any prefix and any draw. That is the test to
port, and `scripts/make-client-profile.ps1 -SelfTest` does.

`DOAL/announce/client_emulator.go` handles `HASH_NO_LEADING_ZERO` by replacing
the first character with a random non-zero hex digit. See entry 27 for why all
four implementations of that algorithm are wrong about the client they name.

### The peer wire surface, which no profile format carries

`DOAL/peerwire/server.go:337`:

```go
var baseReservedBytes = [8]byte{0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x04}
```

Extension protocol in byte 5, fast extension in byte 7, and byte 7 bit `0x01`
for DHT added at `:360` when a node is live. **That set is fixed for every
emulated client.** A profile chooses the peer id and the announce; the reserved
bytes are the same whichever profile is loaded.

`DOAL/peerwire/extensions.go:25-38` builds the BEP 10 handshake as a bencode
string by hand:

```
d1:md11:ut_metadatai1e6:ut_pexi2ee13:metadata_sizei%de1:pi%de4:reqqi250e1:v%d:%se
```

Two things follow for `bit-cli`. `reqq` is **250 for every client**, and the
`m` dictionary carries two extensions. A libtorrent client advertises more than
two. So the extension handshake is a second place where the mask is uniform
while the clients it names are not.

**The key order in that string is already sorted**, and that is not a choice:
bencode requires dictionary keys in lexicographic order, so `m`,
`metadata_size`, `p`, `reqq`, `v` is the only legal order and `ut_metadata`
before `ut_pex` likewise. This corrects the working assumption that the `m`
dictionary's key order is part of the fingerprint. The free variables are the
key **set** and the values. A client that emits them unsorted is fingerprinted
by that fact alone, which is the peer wire form of what section C item 4
records for metainfo.

**The message order after the handshake is a fingerprint and this one is
visible**: `DOAL/peerwire/server.go:317-332` sends the extension handshake,
then the bitfield, then unchoke. Read that against `vortex` PR 156 in entry 9,
which says the bitfield must be the first message after the handshake. Both are
right and they are about different peers: with BEP 10 negotiated the extended
handshake precedes the bitfield, and without it the bitfield is first.

### Passes

Three. WHAT: a Go rewrite of joal with a peer wire added. MECHANISM: the
checksum, the reserved bytes and the extension handshake, cited above. AGAINST
`bit-cli`: it supplies the peer wire half of [T-234](../TODO/peers.md) that
joal cannot, and it supplies the negative result that its own peer wire
identity is uniform across profiles. There is no fourth pass and no tracker
pass to take: the repository has no issues and no pull requests, which is
stated here rather than skipped.

### Verdict

**ADOPT**, for one thing: the peer wire fields belong in a profile, and
`DOAL/peerwire/server.go:337` plus `extensions.go:29` are the list of what a
profile has to carry beyond the announce. Filed under
[T-234](../TODO/peers.md). The licence position is why nothing is taken from
the code itself.

---

## 30. `dig-nat`, DIG-Network/dig-nat

- Upstream: <https://github.com/DIG-Network/dig-nat>
- Local path: `reference/dig-nat`
- Commit: `077df12cbe653d623c13264c86e198c5bd926e77`, cloned 2026-08-24.
- Licence: **`Apache-2.0 OR MIT`**, from `dig-nat/Cargo.toml:19` and
  `dig-nat/README.md:66`. There is no licence file in the repository.
- Language: Rust, 67 files. Zero issues and zero pull requests, so there is no
  tracker pass and that is said rather than skipped.

The most directly useful tree for [T-238](../TODO/peers.md), because it is the
only one that treats traversal as a **ladder of methods behind one call** rather
than as one technique.

### The ladder, and the property that makes it safe to copy

`dig-nat/src/strategy.rs:7-16` states it and `:69-71` enforces it:

```rust
// Guarantee direct-first, relay-last regardless of how the caller ordered `methods`.
let mut ordered = methods;
ordered.sort_by_key(|m| m.kind().rank());
```

Six methods in rank order: Direct, UPnP/IGD, NAT-PMP, PCP, relay-coordinated
hole punch, relayed transport. First success wins. Every method is bounded by
its own timeout at `:104-110`, so a hung method cannot hang the call, and when
all fail the caller gets `AllMethodsFailed` carrying **every method's reason in
attempt order** rather than one error.

That last part is the thing to take. A traversal that fails today tells
`bit-cli` nothing; a traversal that fails with six reasons is a diagnostic.

### What it implements itself and what it takes as a dependency

`dig-nat/Cargo.toml:7-14` writes the rule down, and it is the same rule this
repository applies to `vendor/`:

> the small, well-specified NAT datagrams (NAT-PMP RFC 6886, PCP RFC 6887,
> STUN RFC 5389) are implemented DIRECTLY here, they are tiny fixed-layout
> packets, so implementing them keeps the dependency tree small AND makes every
> byte unit-testable against the RFC layout with NO real network.

UPnP/IGD is the exception, taken from `igd-next`, because SSDP discovery plus
SOAP is a genuinely large protocol. `src/method/` has one file per method:
`direct.rs`, `upnp.rs`, `natpmp.rs`, `pcp.rs`, `hole_punch.rs`, `relayed.rs`.

**This is the answer to "what would a NAT crate cost".** Three of the four
protocols are small enough to own, one is not, and the split is defensible on
testability rather than on preference.

### What its STUN does and does not do

`dig-nat/src/stun.rs`, 533 lines: `encode_binding_request` at `:69`,
`parse_binding_response` at `:173`, `query_reflexive_address` at `:309`,
`discover_reflexive_address` at `:369`. It also refuses a reflexive address
that is not usable, and `:500-508` is a test that an IPv4-mapped IPv6 address
cannot smuggle a loopback or link-local address past the check.

**It discovers a reflexive address. It does not classify a NAT.** There is no
RFC 3489 behaviour test here, no full cone against restricted against symmetric.
That absence holds across every tree in this group and it is the single most
useful negative result for [T-239](../TODO/peers.md): a NAT type diagnostic is
new work, not a port.

### The structural mismatch, stated once for the whole group

`dig-nat/README.md:31` keys a peer on
`peer_id = SHA-256(TLS SubjectPublicKeyInfo DER)`. A BitTorrent peer is an
`IP:port` handed over by a tracker, the DHT or PEX. Every identity-keyed
overlay in this group has the same shape and therefore the same limit: it
connects `bit-cli` to things that also speak it, and to nothing else in the
swarm. That is not a defect in any of them. It is what decides how far one can
be adopted, and it is argued in [T-238](../TODO/peers.md).

### Passes

Three. WHAT: one `connect()` over six traversal methods. MECHANISM: the
strategy at `src/strategy.rs:58-110` and the method split at
`src/method/`. AGAINST `bit-cli`: the ladder shape and the
per-method-reason failure transfer; the mTLS identity model does not. No fourth
pass and no tracker pass: the repository has neither issues nor pull requests.

### Verdict

**ADOPT the shape.** The ranked ladder, the per-method bounded timeout, and the
failure that carries every reason go into [T-238](../TODO/peers.md). No
dependency is added by this session.

---

## 38. `demagnetize-rs`, jwodder/demagnetize-rs

- Upstream: <https://github.com/jwodder/demagnetize-rs>
- Local path: `reference/demagnetize-rs`
- Commit: `479a1186914ee3b287c8e3755c0a6789ff48901a`, cloned 2026-08-24.
- Licence: **MIT** (`demagnetize-rs/LICENSE`, Copyright 2023-2026 John Thorvald
  Wodder II).
- Language: Rust, 2,715 lines across `src/`. 29 open tracker items.

One job, done properly: turn a magnet link into a `.torrent` file by pulling
the metadata from peers. BEP 3, BEP 5, BEP 6, BEP 9, BEP 10, BEP 41, MSE, and
both HTTP and UDP trackers, with magnet info hashes in hex **or base 32**.

### The constant that answers an open entry

`demagnetize-rs/src/consts.rs:15`:

```rust
pub(crate) const MAX_INFO_LENGTH: usize = 20 << 20; // 20 MiB
```

[T-212](../TODO/memory.md) is open and is exactly this: the vendored tree runs
128 metadata reads at once and lets each allocate whatever the peer claims, up
to 32 MiB, on the peer's word. That is 4 GiB from 128 hostile peers.

demagnetize caps the **declared** length before allocating, at a figure well
under librqbit's per-peer ceiling. `dht-spider` (entry 21) caps at
`BLOCK * 1000`, about 16 MiB. Two independent implementations bound this and
`librqbit` does not, which is worth more to T-212 than either one alone: it
says 20 MiB is not a tight bound anybody regretted.

It does not answer the other half of T-212, the aggregate across concurrent
peers, and the entry's approach is right that the aggregate is the unbounded
one.

### What it sends before it knows anything

`demagnetize-rs/src/consts.rs:5`:

```rust
pub(crate) const LEFT: u64 = 65535;
```

A client announcing for a magnet has no metadata, so it does not know the
payload size, and `left` is a required announce parameter. This one sends a
fixed 65,535. `aquatic` PR 254 (entry 16) records that some clients send
`left = -1` and that a `usize` parse rejected the whole announce. So there are
at least three answers in the wild: a fixed number, a negative number, and
omitting the key.

`bit-cli` sends the real figure once metadata is resolved, which
[T-235](../TODO/trackers.md) measured. What it sends **before** resolution is
not covered by that check and is worth knowing rather than assuming.

`:23` `UT_METADATA: u8 = 42` is its own extension id, which is a reminder that
the number is ours to choose and theirs to map, and the reason
`vortex` PR 103 (entry 9) is the best interop finding in the corpus.

### The four axis comparison

Established first from `man/bit-cli.json` and the closed entries, then measured
where a claim could be measured rather than read.

| capability | `bit-cli` today | `demagnetize-rs` | `intermodal` |
| --- | --- | --- | --- |
| magnet to metainfo over BEP 9, no tracker, no web seed | **yes**, measured below | yes, its whole purpose | no |
| write the resolved metainfo to a `.torrent` | **no** | yes | no |
| magnet info hash in base 32 | not checked | `src/types.rs:35,64`, hex or base 32 | yes |
| torrent to magnet | `bit-cli magnet <torrent>` | no | `imdl torrent link` |
| create a torrent | `bit-cli create` | no | yes |
| edit metainfo without moving the info hash | `bit-cli edit`, exit 15 guards it | no | no |
| verify data against a torrent | `bit-cli verify` | no | yes |
| Metalink `.meta4` and `.metalink` | **yes**, [T-113](../TODO/cli-surface.md) | no | no |
| DHT `get_peers` | yes | yes | no |
| DHT crawl, `announce_peer` harvesting | no | no | no |
| UDP tracker BEP 41 | not checked | yes | `imdl torrent announce` |
| MSE | yes, [T-163](../TODO/peers.md) | yes | no |

**The interesting axis was measured rather than assumed.** A 2 MiB payload, a
torrent with no tracker in it, a magnet built from that torrent carrying only
`xt`, `dn` and `xl`, one seeder given by address, and DHT, LSD and trackers all
off on both ends:

```
bit-cli download "magnet:?xt=urn:btih:1d02661d..." --peer 127.0.0.1:PORT \
  --no-dht --no-lsd --no-tracker --init-timeout 30s --json
```

Exit 0, 2,097,152 bytes, `finished: true`, and the payload landed byte for
byte. So metadata came over BEP 9 from the one peer, with nothing else to get
it from. **`bit-cli` already does the axis this comparison exists to test**,
and the gap is elsewhere.

### The one real gap

`bit-cli magnet` goes one way. Given a `.torrent` it prints a magnet URI, and
`bit-cli info` and `bit-cli files` accept a magnet and print what they resolve,
but nothing writes the resolved metainfo back out as a `.torrent`.
`man/bit-cli.json` confirms it: `bit-cli magnet` takes a single positional
`source` and no `--output`.

So a caller who resolves a magnet keeps the payload and loses the metainfo, and
resolving it again means finding peers again. That is [T-241](../TODO/metainfo.md).

### Passes

Four. WHAT: one job, magnet to `.torrent`. MECHANISM: `src/consts.rs` and the
peer and tracker modules behind it. THE HARD PART: the two bounds above, which
are what `bit-cli` finds hard and has open as [T-212](../TODO/memory.md).
AGAINST `bit-cli`: the gap table, and the measurement that closed the axis
question.

The tracker was listed and not read issue by issue: 29 open items on a
single-purpose tool, and the two constants above are what this entry needed.
That is stated rather than implied.

### Verdict

**CONFIRMS** on the magnet axis, measured. **ADOPT** the declared-length cap as
corroboration under [T-212](../TODO/memory.md). One gap filed as
[T-241](../TODO/metainfo.md).

---

# Tier 2 — strong on one axis

## 9. `vortex` — Nehliin/vortex

- Upstream: <https://github.com/Nehliin/vortex>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\vortex`
- Licence: **MIT** (`vortex/LICENCE.txt`)
- Language: Rust, `io_uring`, Linux ≥ 6.1. Implements BEP 3, 6, 9, 10, 20, 21.

### BEP 6 allowed-fast set (T-100) — spec-conformant

Moved to [`HISTORY/RESEARCH-bep6.md`](HISTORY/RESEARCH-bep6.md) on 2026-08-24: T-100 closed and the
behaviour is in the tool.

### Piece selection and endgame (T-034)

`bittorrent/src/piece_selector.rs:91` `next_piece(connection_id, &mut endgame)`:

1. `pickable = !downloaded & interesting_peer_pieces[peer]`.
2. If nothing pickable is **unallocated**, set `endgame = true` and return the
   first pickable piece — i.e. endgame is entered per-peer, on demand, and is
   *reported through an out-parameter* rather than inferred.
3. Otherwise, if more than 95 % of pieces are still missing, try five random
   indices (a warm-up phase that avoids everyone converging on piece 0).
4. Otherwise rarest-first: count availability across
   `interesting_peer_pieces`, take the minimum with count > 0.

`bit-cli` T-034 asks for endgame to be observable; this shows the minimal
plumbing — a `&mut bool` threaded out of the selector.

### Choking (T-024, T-083)

`bittorrent/src/torrent.rs:488` `recalculate_unchokes`:

- Peers that are not interested, or pending disconnect, are choked immediately
  and their round counters reset; if such a peer was the optimistic unchoke, the
  optimistic timer is reset so someone else gets the slot.
- **Leeching**: sort by `downloaded_in_last_round` descending.
- **Seeding**: libtorrent-style round robin — a peer that has been unchoked for
  over a minute and has received more than `piece_length *
  seeding_piece_quota` bytes is demoted; ties break on
  `uploaded_in_last_round`, then on time since last unchoke.
- One fifth of `max_unchoked` (minimum 1) is reserved for optimistic unchokes;
  a previously optimistic peer that earns a normal slot is "promoted" and the
  optimistic timer resets. `:594` `recalculate_optimistic_unchokes`.
- Config at `:55-97`: `max_unchoked = 8`, recalc every 15 ticks, optimistic
  recalc every 30.

### Storage: pieces that span files

`bittorrent/src/file_store.rs` — the test names are the specification:
`basic_multifile_alinged`, `small_multifile_misalinged`,
`small_multifile_misalinged_files_and_subpiece`,
`multifile_not_multiple_of_piece_size`, `multifile_misalinged_v2`,
`multifile_misalinged_v3`, `single_file_misaligned`,
`basic_single_file_aligned_unaligned_subpiece`. Run against fx-torrent
Issue 98 above, this is the positive counterpart.

### BEP 9/10 metadata exchange

`bittorrent/src/peer_comm/extended_protocol.rs`: `:20` `init_extension`
(rejects a `metadata_size` that disagrees with metadata we already have), `:60`
`extension_handshake_msg` (`m`, `v`, `p`, `metadata_size`, `upload_only`,
`reqq`), `:173` `MetadataExtension` — 16 KiB pieces, an initial burst of up to
8 requests, SHA-1 of the assembled metadata checked against the info hash before
use, `REJECT` handled by advancing to the next piece, and late `DATA` after
completion ignored rather than treated as an error.

### Issues / PRs

| # | State | Title | Why it matters |
|---|---|---|---|
| [PR 103](https://github.com/Nehliin/vortex/pull/103) | MERGED | fix: Critical bug in extension message handshake | **The best interop finding in the corpus.** The extension map was keyed by *our* extension id but checked against *theirs*: `if self.extensions.contains_key(&id) { continue; }`. So when qBittorrent assigns `ut_metadata = 2` and we use `1`, the incoming id 2 was skipped as "already initialised" because our own `upload_only` is 2. Result: "we've never been able to have extensions work with qBittorrent". Any BEP 10 implementation must map **peer id → handler** and **name → our id** as two separate directions. |
| [PR 156](https://github.com/Nehliin/vortex/pull/156) | MERGED | Incorrect initial message order sent out after handshake | Messages arriving in the same TCP read as the handshake were processed before the bitfield was queued, so `Interested` could precede `Bitfield`. **The bitfield must be the first message after the handshake.** |
| [PR 155](https://github.com/Nehliin/vortex/pull/155) | MERGED | Incorrect handling of `Have` messages for non fast extension peers | Spec conformance for peers without BEP 6. |
| [PR 124](https://github.com/Nehliin/vortex/pull/124) | MERGED | correct subpiece slice bounds for non-power-of-2 piece lengths | With `piece_length = 1,986,560 = 121×16384 + 4096`, the last subpiece of every non-last piece is short; `end_idx = offset + 16384` overflowed the buffer, panicking and then double-panicking in `Buffer::drop`. Fix: `end_idx = (start + SUBPIECE).min(piece_len)`, plus a `!thread::panicking()` guard in the destructor. `bit-cli` should have a fixture with a piece length that is not a multiple of 16 KiB. |
| [PR 129](https://github.com/Nehliin/vortex/pull/129) | MERGED | Expand piece request validation | Consequence of 124: reject invalid requests at the protocol boundary, never let them reach the file layer. |
| [PR 142](https://github.com/Nehliin/vortex/pull/142) | MERGED | Prevent incorrect timeouts/snubs | The in-flight queue was not consulted before snubbing, so non-fast peers were snubbed merely for choking, and fast peers were snubbed after **explicitly rejecting** every request — a reject is not a timeout. |
| [PR 143](https://github.com/Nehliin/vortex/pull/143) | MERGED | Add coarse grained stalled connection timeout | Mirrors libtorrent: drop a connection with no activity in either direction for 15 s **while requests are in flight**. Relevant to `bit-cli` T-007 (a stalling source takes 24 s to give up) and `--stop-timeout`. |
| [Issue 125](https://github.com/Nehliin/vortex/issues/125) | OPEN | peer blocklist to defend against malicious/malformed piece responses | After 124 stopped the crash, the same peer reconnects and keeps sending garbage, burning a connection slot; DHT rediscovers it every 20 s. Proposal: auto-block on protocol violation, check the blocklist before completing a handshake, expose add/remove/query, optional persistence. `bit-cli` has `--web-seed-fatal-status` for HTTP sources but nothing equivalent for peers. |
| [Issue 149](https://github.com/Nehliin/vortex/issues/149) | CLOSED | io_uring_setup failure panics: `Torrent::start` unwraps the ring build | Fixed by PR 150. Environment-dependent init must return an error, not panic — the same discipline `bit-cli` needs for `falloc` without `SeManageVolumePrivilege`. |
| [Issue 151](https://github.com/Nehliin/vortex/issues/151) | OPEN | Check if the system `ulimit` is reasonable in the torrent start method | Related to `bit-cli` T-011 / `--max-open-files` / `--max-handles`. |
| [Issue 91](https://github.com/Nehliin/vortex/issues/91) | OPEN | Support Peer Exchange (PEX) | Confirms the README's "not implemented". |

---

## 10. `rustorrent` — josusanmartin/rustorrent

- Upstream: <https://github.com/josusanmartin/rustorrent>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\rustorrent`
- Licence: **MIT** (`rustorrent/LICENSE`, © 2026 Josu San Martin).
- Language: Rust with **five direct dependencies** (`native-tls`, `libc`,
  `getrandom`, `num-bigint`, `num-traits`). Bencode, SHA-1, SHA-256, HTTP, peer
  protocol, DHT, uTP, UPnP, NAT-PMP, MSE and the web UI are all in-repo. That
  makes it the best place in the corpus to read a self-contained implementation
  of any one subsystem.
- No Issues or Pull Requests exist upstream.

### v1 / v2 / hybrid metainfo (T-081, T-134)

`rustorrent/src/torrent.rs`:

- `:163` `parse_torrent` — `has_v2 = meta version == 2`; `meta_version` resolves
  to hybrid when both v1 and v2 fields are present; the v2 hash is
  `sha256(info_bytes)`, stored as `info_hash_v2` alongside the SHA-1.
- `:300` — for v2, `piece length` must be **≥ 16 KiB and a power of two**.
- `:461` `parse_file_tree` / `:471` `parse_file_tree_recursive`, `:518`
  `parse_piece_layers`.
- `:542` `validate_v2_piece_layers` — files at or below one piece are skipped;
  every other file's layer must have exactly `ceil(length / piece_length)`
  hashes and must reconstruct its `pieces root`; **and the reverse check**, that
  every entry in `piece layers` corresponds to a file that needs one
  ("piece layers are not an extension bucket").
- `:581` `validate_hybrid_layout` — the single-file case must have exactly one
  file-tree entry matching `name` and `length`; the multi-file case walks the v1
  `files` list, requires every **BEP 47 padding file** (`attr` contains `p`) to
  be exactly the number of bytes needed to reach the next piece boundary,
  requires every non-padding file to start on a piece boundary, and requires the
  v1 and v2 file lists to agree pairwise in path and length. This is the most
  complete hybrid validation in the corpus.

`rustorrent/src/sha256.rs` — a from-scratch SHA-256 plus `:164`
`merkle_piece_root` (leaves padded to `piece_length/16 KiB` with the zero hash)
and `:186` `merkle_root_from_piece_layer`, whose doc states the rule exactly:
"omitted balancing nodes are supplied using the zero hash for the selected piece
layer". The pad hash is derived by repeatedly `hash_pair`-ing the zero hash up
from the block level — the correct construction. Tests
`bep52_piece_hash_uses_merkle_nodes_above_16k` and
`bep52_short_final_piece_is_zero_hash_padded` at `:299` and `:311`.

### BEP 16 superseeding (T-082)

The only implementation in the corpus, though a simplified one.
`rustorrent/src/main.rs`:

- `:10577` — outbound connections in super-seed mode send **a single `Have` for
  one pseudo-randomly chosen piece instead of the bitfield**, and remember it as
  `super_seed_piece`.
- `:11050` — when that peer later advertises the same piece (proving it
  redistributed it), advance to `(index + 1) % piece_count` and send that.
- `:12588` — the inbound-connection path does the same, seeded with the peer tag.
- The `else` branches around `:10593` and `:12601` carry the BEP 3 rule that a
  bitfield is always sent as the first message after the handshake, even when
  empty.

What it does **not** do, and what a full BEP 16 needs: tracking which piece each
peer was given, refusing to advance until redistribution is confirmed to a
*different* peer, and disconnecting peers that never redistribute.

### Windows filesystem safety (T-070, T-071, T-072)

`rustorrent/src/windows_fs.rs` (830 lines) is handle-relative Windows I/O:
"Every relative open starts from a pinned directory handle, opens one path
component at a time with `FILE_OPEN_REPARSE_POINT`, and omits
`FILE_SHARE_DELETE`. That makes path validation and the operation which consumes
it one descriptor-bound action instead of a check-then-open race." Key symbols:
`:582` `nt_open_component`, `:463` `checked_regular`, `:675` `rename_relative`,
`:762` `validate_component`, and the `FILE_RENAME_INFORMATION` /
`FILE_DISPOSITION_INFORMATION` / `FILE_ID_INFO` class constants at `:51-53`.

`bit-cli` "plans every path before it opens anything" — a check-then-open
pattern. Against a hostile `.torrent` on a shared filesystem this is exactly the
TOCTOU window this module closes. Related: `src/ownership.rs` (`ClaimKind::File`
/ `Tree`) and `src/state_dir.rs`.

### Audit reports — a checklist of edge cases

`rustorrent/docs/DEEP_AUDIT_REPORT_2026-07-13.md` and
`DEEP_AUDIT_REPORT_2026-03-20.md` treat "torrent metadata, magnets, trackers,
peers, DHT packets, web seeds, RSS feeds, search-plugin inputs and outputs, UI
requests, state files, and payload directories as untrusted". Concrete items
`bit-cli` can lift as test cases:

- Bencode rejects non-canonical integers, unsorted or duplicate keys, excessive
  depth, excessive value counts, invalid lengths, trailing data, truncation.
- Torrent parsing validates exact v1 piece counts, v2 file trees, piece layers,
  merkle roots, hybrid layout consistency, length arithmetic, file/path
  collisions and collection limits.
- HTTP(S) tracker clients use absolute deadlines, bounded bodies, redirect
  limits, HTTPS-downgrade prevention and a public-address policy.
- "Tracker, magnet, DHT, PEX, and ordinary peer candidates are normalized and
  filtered by source; **public sources cannot inject loopback, private,
  link-local, mapped-private, or special-use peers**." `bit-cli peers` accepts
  `--peer HOST:PORT` explicitly, which is fine — the rule is about peers the
  *network* supplies.
- Proxy mode is fail-closed: paths that cannot be proxied are disabled rather
  than allowed to leak around it.
- "Network-derived log text and URLs are sanitized, bounded, and stripped of
  credentials/query secrets before terminal or file logging." `bit-cli`
  supports per-source `Authorization` headers and signed CDN URLs and has
  `--log-file`; this one is worth acting on.

`docs/TEST_COVERAGE.md` records 477 tests across unit, adversarial-process and
swarm/uTP suites, with the areas each covers.

### Other subsystems

`src/mse.rs` (RC4 + DH, `CryptoMode::{Plaintext, Rc4}`), `src/utp.rs`
(`UTP_PAYLOAD_MAX = 1200`, `UTP_ACK_TIMEOUT = 500 ms`), `src/lpd.rs` (BEP 14,
30 s interval), `src/ip_filter.rs` (blocklists bounded at 128 MiB / 2 000 000
rules), `src/upnp.rs` (SSDP, 3 attempts, 1 s timeout), `src/natpmp.rs`
(retry ladder 250 ms/500 ms/1 s/2 s), `src/rss.rs`, `src/proxy.rs`,
`src/search.rs`, `src/xml.rs` (bounded parser), `src/geoip.rs`.
`fuzz/fuzz_targets/`: `bencode_parse`, `torrent_parse`, `peer_decode_message`,
`http_tracker_parsers`, `storage_paths` — the last of which is directly relevant
to `bit-cli`'s path planner.

---

## 11. `mtorrent` — DanglingPointer/mtorrent

- Upstream: <https://github.com/DanglingPointer/mtorrent>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\mtorrent`
- Licence: **MIT**, declared in the workspace manifest
  (`mtorrent/Cargo.toml:15`; member crates inherited it via
  `license.workspace = true` before their own manifests were removed). No
  `LICENSE` file is present, so this manifest is kept as the licence evidence
  (§G).
- Language: Rust. Implements BEP 3, 5, 7, 9, 10, 11, 12, 15, protocol
  encryption, uTP and UPnP. BEP 6 and BEP 52 are explicitly *not* done.

### Protocol encryption (MSE/PE) — the cleanest in the corpus

`mtorrent/mtorrent-core/src/pe/`:

- `key_exchange.rs` — the 768-bit MSE DH prime as a `const_monty_params!`
  constant, generator 2, `local_pubkey = 2^x mod P`, `shared_secret =
  Y^x mod P`, `KEY_SIZE = 96`.
- `mtorrent/mtorrent-core/src/pe/handshake.rs:12-17` — `MODE_PLAINTEXT = 1`, `MODE_RC4 = 2`, `MODE_ANY = 3`,
  `MAX_PADDING_LEN = 512`, `VC_LEN = 8`; the `max_pe3_len`/`max_pe4_len` consts
  just below compute the maximum message lengths so the reader can bound its
  buffers. `:41` `outbound_handshake` advertises `crypto_provide = MODE_ANY`;
  `:164` `inbound_handshake` verifies the VC is all zeroes, rejects
  `crypto_provide & MODE_ANY == 0`, and prefers RC4 when offered.
- `mtorrent/mtorrent-core/src/pe/utils.rs:17` `detect_encryption` — reads exactly `PROTOCOL_STRING.len()`
  bytes, compares, and **returns the stream with those bytes pushed back**
  (`PrefixedStream`), so one listening port serves plaintext and encrypted peers.
  `mtorrent/mtorrent-core/src/pe/utils.rs:9` `MaybeEncrypted<T>`. This is the piece nanotorrent's incoming seam needs.
- `cipher.rs`, `io.rs` (`DecryptingBufReader`, `EncryptingWriter`).

### uTP (T-101)

`mtorrent-core/src/utp/`:

- `protocol.rs` — `TypeVer { Data 0x01, Fin 0x11, State 0x21, Reset 0x31,
  Syn 0x41 }`, 20-byte header codec with explicit `WouldBlock`/`Unsupported`
  errors.
- `mtorrent/mtorrent-core/src/utp/retransmitter.rs:48-50` — `MAX_PACKET_SIZE = 9 KiB` ("macOS default UDP
  limit"), `MIN_PACKET_SIZE = 1472` (Ethernet MTU), `INITIAL_RTO = 1 s`.
  `mtorrent/mtorrent-core/src/utp/retransmitter.rs:108` `process_ack` implements the Jacobson/Karels RTT update
  (`rtt_var += (|Δ| - rtt_var)/4; rtt += (packet_rtt - rtt)/8;
  timeout = max(rtt + 4·rtt_var, 500 ms)`) **only for packets sent once**,
  doubles `packet_size` up to the cap when the queue drains, and fast-retransmits
  on the second duplicate ack. Tests `test_regular_retransmit` and
  `test_fast_retransmit` use `tokio::test(start_paused = true)`.
- `connection.rs`, `handle.rs`.

### Trackers

Moved to [`HISTORY/RESEARCH-trackers.md`](HISTORY/RESEARCH-trackers.md) on 2026-08-24: T-063, T-064 and T-065 closed and the
behaviour is in the tool.

### Issues / PRs

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 17](https://github.com/DanglingPointer/mtorrent/issues/17) | CLOSED | Respect `reqq` | A peer's extended handshake advertises `reqq`, the number of requests it will queue; exceeding it wastes requests or gets you dropped. `bit-cli`'s `bench leech` reports "queue depth 128" — that must be bounded by the peer's `reqq`. |
| [Issue 15](https://github.com/DanglingPointer/mtorrent/issues/15) | CLOSED | Disconnect idle peers | Same conclusion seedchamp reached with two separate idle timers. |
| [Issue 21](https://github.com/DanglingPointer/mtorrent/issues/21) | CLOSED | Retry block requests | On a request timeout, re-send rather than dropping the peer. Compare vortex PR 142. |
| [Issue 29](https://github.com/DanglingPointer/mtorrent/issues/29) | CLOSED | Can't connect to peers when too many trackers | With many configured trackers, outgoing TCP connects timed out. The resolution list is instructive: prune bad trackers from config, **announce to the torrent's/magnet's own trackers before any configured extras**, and move tracker work into the peer runtime. `bit-cli trackers` announces to every tracker in the torrent; ordering and concurrency matter (T-063). |
| [PR 37](https://github.com/DanglingPointer/mtorrent/pull/37) | MERGED | PE/MSE | The encryption work above. |
| [PR 36](https://github.com/DanglingPointer/mtorrent/pull/36) / [PR 41](https://github.com/DanglingPointer/mtorrent/pull/41) | MERGED | Utp / uTP Ipv6 | uTP, then its IPv6 follow-up — relevant to `bit-cli` T-022 (IPv6-only swarm churn). |
| [PR 30](https://github.com/DanglingPointer/mtorrent/pull/30) | MERGED | Connctrl rewrite | Connection-control rewrite, i.e. the outgoing-connect queue from Issue 16. |

Fixtures: `mtorrent/mtorrent-cli/tests/assets/` retains
`big_metainfo_file.torrent` (102 KiB), `example.torrent`, `zeroed_example.torrent`,
`incomplete.torrent`, and the `torrents_with_tracker/` vs
`torrents_without_tracker/` pairs under the same directory. The 25 MiB `pcap/` and 1.1 MiB `screenshots/` payload directories those
torrents describe were removed as binary payload; the `.torrent` files remain
usable as metainfo fixtures.

---

## 12. `n0-mainline` — n0-computer/n0-mainline

- Upstream: <https://github.com/n0-computer/n0-mainline>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\n0-mainline`
- Licence: **MIT** (`n0-mainline/LICENSE-MIT`, © 2021 raptorswing, © 2025
  nuh.dev). An iroh-flavoured fork of `nuhvi/dht`.
- Language: Rust. BEP 5, 42, 43, 44 — client and server.

### The BEP texts

`n0-mainline/beps/` contains the reStructuredText sources of
**BEP 5** (429 lines), **BEP 42** (265), **BEP 43** (78) and **BEP 44** (445),
plus `bep_signed_peers.rst` (191 lines), a draft extension for signed peer
announcements. Having the normative text in the corpus is worth more than any
one implementation when `bit-cli` comes to implement or report on these.

### BEP 42 (compare with superseedr's implementation)

`n0-mainline/src/common/id.rs`: `:16` `IPV4_MASK = 0x030f3fff`,
`:143` `id_prefix_ipv4` (CRC32-Castagnoli over `(ip & mask) | (r << 29)`),
`:123` `first_21_bits` (`[b0, b1, b2 & 0xf8]`), `:108` `is_valid_for_ip`
(private/link-local/loopback are exempt). IPv6 is `unimplemented!()`.

### BEP 44 mutable items

`src/common/mutable.rs`: `:32` `MutableItem::new(signer, value, seq, salt)`,
`:46` `target_from_key` = **SHA-1 of `public_key || salt`**, `:145`
`encode_signable(seq, value, salt)` — the exact bytes that get ed25519-signed.
`src/common/immutable.rs` is the immutable half. `src/core/put_query.rs` is the
put path. This is what `bit-cli` would need for BEP 46 (mutable-torrent
updates), which pairs naturally with its existing BEP 39 `update-url` support.

### DHT tokens and peer store (BEP 5 server side)

`src/core/server/tokens.rs`: `:13` `SECRET_SIZE = 20`, `:14` `TOKEN_SIZE = 4`,
CRC32C over `ip_octets || secret`; `:51` `validate` accepts the current **or**
previous secret, `:59` `rotate` shifts them. `src/core/server/peers.rs` —
an LRU of LRUs keyed by info hash, returning at most 20 random peers.

### Vertical Sybil resistance

`src/common/closest_nodes.rs:79` `take_until_secure(previous_dht_size_estimate,
average_subnets)` — walk the closest nodes, stop once the XOR distance exceeds
`20 · 2^128 / (estimate + 1)` **and** at least `average_subnets` distinct /6
IPv4 prefixes (`:137` `subnet`) have been seen; never return fewer than
`MAX_BUCKET_SIZE_K`. `:127` `dht_size_estimate` and
`docs/dht_size_estimate.md` / `docs/censorship-resistance.md` explain the
statistics. `examples/` includes `measure_dht.rs`, `mark_recapture_dht.rs`,
`count_ips_close_to_key.rs`, `request_filter.rs`, `cache_bootstrap.rs`.

### Adaptive mode

The README documents the default: start as a client, and after 15 minutes with a
publicly reachable address switch to server mode, so only stable reachable nodes
carry routing load. Directly relevant to `bit-cli`'s DHT items (T-050, T-052) —
a short-lived CLI invocation should almost certainly never become a server.

Issues are disabled upstream. Of the nine merged PRs, one is research-relevant:
[PR 9](https://github.com/n0-computer/n0-mainline/pull/9) (MERGED), "port
mainline 6.4.1 mutable put security fixes" — BEP 44 `put` had security fixes
worth reviewing before implementing mutable items.
[PR 4](https://github.com/n0-computer/n0-mainline/pull/4) (MERGED) "Bounded
queues" is the standard defence against a DHT flooding the process.

---

## 13. `seedchamp` — j-c-m/seedchamp

- Upstream: <https://github.com/j-c-m/seedchamp>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\seedchamp`
- Licence: **MIT** (`seedchamp/LICENSE.md`, © 2026 Jesse Miller).
- Language: Rust (Compio). Seedbox-oriented; trackers only, no DHT/PEX/magnets.

`seedchamp/docs/design.md` (442 lines) is the densest performance-engineering
document in the corpus, and almost all of it applies to `bit-cli`'s open
performance items.

### Request pipelining (T-018, T-034, T-041)

`docs/design.md:197` — per-peer request depth is **BDP-sized from an EMA of that
peer's wire download rate**: `desired ≈ 5 s × rate / 16 KiB`, with
`swarm.pipeline` as the initial depth and `swarm.pipeline_max` as the cap.
Request stall is 20 s, **4 s in endgame**, and triggers Cancel + re-Request; a
partial frame stays in the buffer and **only ingested blocks refresh the stall
clock**. On park, `SO_RCVLOWAT` is raised to
`max(mid-frame remainder, min(outstanding × 16 KiB, SO_RCVBUF/2, 256 KiB))` so
`epoll`/`kqueue` wake once per large chunk rather than once per MSS, and is
restored to 1 on drop; the stall timeout deliberately skips the speculation once
so short bursts still drain.

`bit-cli`'s `bench leech` already reports "pipeline: N blocks in flight …,
window allows X MiB/s at that depth and that service time". This paragraph is
the control loop that number is measuring.

### Staging memory (T-041)

`docs/design.md:199` — a per-torrent freelist of piece-sized buffers bounded by
`swarm.staging_mem_limit` (default 256 MiB); cap `N = limit / piece_length`;
lazily allocated, recycled on release. A peer may hold at most enough to fill its
pipeline, at most `⌈N/16⌉` of the freelist, and **at most 2 pieces when
`piece_length ≥ 4 MiB`** so a 1 GiB pool with 16 MiB pieces still serves ~32
peers. `take_for_hash` detaches the buffer so disk writes cannot drain the leech
freelist, and the pool is dropped entirely once all wanted pieces are complete —
seeding then costs no staging RAM. `bit-cli`'s T-041 says its per-source window
cache is "bounded but not measured"; this is what a measured bound looks like.

### Idle-peer closing (T-020 CLOSE_WAIT)

Moved to [`HISTORY/RESEARCH-fastresume-and-idle-peers.md`](HISTORY/RESEARCH-fastresume-and-idle-peers.md) on 2026-08-24: T-016 and T-020 closed and the
behaviour is in the tool.

### Storage backend selection

`docs/design.md:203-217` — a per-OS/per-filesystem table for the seed read path:
Compio `read_at` (io_uring) on FreeBSD and on Linux ext4/xfs/btrfs; **blocking
`pread` on Darwin and on Linux ZFS/tmpfs, because io_uring is slow there for
small reads**; overridable by `[upload].backend` / `SEEDCHAMP_UPLOAD_BACKEND` /
`SEEDCHAMP_UPLOAD_COMPIO_FS=all`. Recheck uses windowed 256 KiB reads. `bit-cli`
has `bench disk` with three layouts; the same "measure, then gate by
filesystem" conclusion applies.

### Session import/export

`docs/rtorrent-session.md` and `docs/transmission-session.md` are precise
mapping tables, complementing TorrentNG's exporters with the *import* direction:

- rTorrent: `$session/<40-HEX-INFOHASH>.torrent{,.rtorrent,.libtorrent_resume}`,
  **uppercase hex**, optional `.meta` for magnet metadata.
  `.rtorrent` → `directory`, `timestamp.started`/`state_changed`/`finished`,
  `total_uploaded`/`total_downloaded`, and on export `state ← want_start`.
  Crucially: "rtorrent's session `directory` already includes that name — import
  strips it so paths do not double-nest."
- Transmission: `torrents/<40-hex>.torrent` + `resume/<40-hex>.resume`;
  `progress.blocks` is complete only when `all`, and **`progress.pieces` is
  "checked pieces only — not used as have-complete"**. Incomplete torrents
  export `none` and require a recheck, because block maps are not reconstructable
  from a piece bitfield.

### BEP posture

`docs/design.md:152-160`: BEP 3, MSE/PE RC4 on all post-handshake bytes when
selected, BEP 10, **BEP 6 with "Suggest recv is honored in the picker; we do not
send Suggest"**, BEP 12, compact peer lists. `network.encryption` takes
`off | prefer-plain | prefer-rc4 | require-rc4`.

`bench/` retains `lt_peer.py` (a libtorrent-driven peer), `diskworker.py`,
`throughput.py`, `gen_seed.py`, `smoke.py` — a Python harness for driving a
client under load.

Pull requests (there are no issues):
[PR 7](https://github.com/j-c-m/seedchamp/pull/7) (MERGED) "Honor BEP 6 Suggest
in the piece picker" — the receive half of BEP 6 that vortex and anacrolix both
have and `bit-cli` does not;
[PR 6](https://github.com/j-c-m/seedchamp/pull/6) (MERGED) "wire tracker
numwant"; [PR 2](https://github.com/j-c-m/seedchamp/pull/2) and
[PR 1](https://github.com/j-c-m/seedchamp/pull/1) (MERGED) implement the
rTorrent/Transmission export and import above;
[PR 3](https://github.com/j-c-m/seedchamp/pull/3) (MERGED) "keep leech-cache
source until after live layout swap" and
[PR 4](https://github.com/j-c-m/seedchamp/pull/4) / [PR 5](https://github.com/j-c-m/seedchamp/pull/5)
(MERGED, then reverted) "drop staging pool when wanted download completes" —
a reverted optimisation is a useful warning.


### Re-mined 2026-08-24, at `4a1acdf8f196328c7ca284368e0f6652540d1a99`

**Nothing upstream has moved.** Every one of the 160 files kept in the corpus
copy is byte-identical to the same file at that commit, except `README.md`,
which section G records as rewritten during cleaning. The last upstream push
was 2026-08-19, two days before the corpus was taken.

**The SHA above is the first one this corpus has recorded for any tree.** The
2026-08-21 pass captured none, so there is no old SHA to compare against and
none can be recovered: the clones were stripped of `.git` at the time. What is
above is therefore a baseline for the next re-mine rather than a difference
from the last one, and the omission is why `docs/reference-mining.md` states
the rule first.

**The tracker was already read.** Seven pull requests, all closed, zero issues,
and the section above lists all seven. Nothing new to fetch.

### What it says against what `bit-cli` has since measured

The design document is still ahead of this tree on one axis and behind it on
another, and the split is the useful part.

**Ahead: the request depth is derived, not configured.** `docs/design.md:197`
sizes the per-peer depth from an exponential moving average of that peer's own
wire rate, `desired = 5 s * rate / 16 KiB`, bounded by `swarm.pipeline_max`.
`bit-cli` reports a pipeline depth and a window ceiling in `bench leech`
([T-090](../TODO/bench.md)) and does not adapt either: `librqbit`'s window is
a fixed 128 blocks and [T-001](../TODO/webseed.md) measured that the run sits
at 40% of what that peak would allow. So the number this tree reports is the
one seedchamp's control loop would be moving.

**Ahead: the staging pool is budgeted in bytes and capped per peer.**
`docs/design.md:199`: a per-torrent freelist bounded by
`swarm.staging_mem_limit`, default 256 MiB, with a peer allowed at most
`ceil(N/16)` of it and at most 2 pieces when the piece length is 4 MiB or
more. [T-041](../TODO/memory.md) closed on reporting `bit-cli`'s own budget
and capping the total across sources, which is the same idea one layer up: the
budget is over HTTP sources rather than over peers. The per-peer fraction is
the part `bit-cli` does not have, and it is what stops one slow peer holding
the pool.

**Behind: none of it is measured in that repository.** `docs/design.md` states
defaults and invariants and carries no numbers from a run.
`bench/lt_peer.py`, `throughput.py` and `diskworker.py` are the harness that
would produce them and no results are committed. `bit-cli` has the opposite
problem and the better one: `bench/` holds committed runs behind every
comparative claim, which is [RULES.md](../TODO/RULES.md) section 5's rule.

### What an ADOPT out of this owes

A comparative claim here needs a committed benchmark, so the adoption is filed
with the bench that would prove it rather than as a preference:
[T-242](../TODO/performance.md), the adaptive request depth, whose acceptance
is a `bench leech` sweep at a fixed depth against a derived one on the same
fixture.

`scripts/bench-leech.ps1` exists and takes the measurement today; what it does
not have is a way to hold the depth fixed while the rate varies, which is why
the entry names the script change rather than assuming the script covers it.

---

## 14. `aria2_rust` — balovess/aria2_rust

- Upstream: <https://github.com/balovess/aria2_rust>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\aria2_rust`
- Licence: **MIT** (`aria2_rust/LICENSE`).
- Language: Rust. A port of aria2 aiming at behavioural compatibility.
- **Per instruction, only BitTorrent functionality was researched**, and the
  non-BitTorrent migration/compatibility bookkeeping (`docs/MIGRATION.md`,
  `docs/compatibility-status.md`, `docs/testing-guide.md`), the C/Node/Python
  bindings, the release scripts and the top-level benches were removed during
  cleaning.

`bit-cli` positions itself explicitly against aria2 ("aria2 has no
`--bt-web-seed` flag"), and Phase C targets aria2 RPC parity, so this is the
reference implementation of the surface `bit-cli` is measured against.

### BEP 6 `computeFastSet` — and a divergence worth knowing

Moved to [`HISTORY/RESEARCH-bep6.md`](HISTORY/RESEARCH-bep6.md) on 2026-08-24: T-100 closed and the
behaviour is in the tool.

### Everything else under `aria2-protocol/src/bittorrent/`

All paths below are under `aria2_rust/aria2-protocol/src/bittorrent/`:
`bencode/`; `dht/` (22 files, including `bucket_tree.rs`, `routing_table.rs`,
`token_tracker.rs`, `persistence.rs`, `replace_node.rs`, `task_peer.rs`);
`extension/` (`mse.rs`, `mse_crypto.rs`, `mse_dh.rs`,
`mse_handshake.rs` at 1228 lines, `pex.rs`, `ut_metadata.rs`,
`ut_metadata_tracker.rs`); `message/` (`factory.rs`, `serializer.rs`,
`validation.rs` at 690 lines, `extension/{handshake,ut_metadata,ut_pex}.rs`);
`peer/` (`encrypted_connection.rs`, `incoming.rs`, `listener.rs`);
`piece/` (`picker.rs` 1197 lines, `bitfield.rs` 760 lines);
`tracker/` (`udp_tracker_protocol.rs` 936 lines, `public_list.rs` 874 lines);
`utp/` (`congestion.rs`, `timer.rs`, `socket.rs`, `metrics.rs`).

`aria2_rust/aria2-protocol/src/bittorrent/piece/picker.rs:9-22` names six strategies — `Sequential`, `RarestFirst`, `Random`,
**`LongestSequence`**, `Priority`, `Geometric` — plus a `PiecePriorityMode` of
`SequentialHead` / `SequentialTail` / `RarestFirst` (`:27-33` in the same file). `bit-cli`'s default is
"first piece of each file, then the last, then the middle ascending", i.e. a
head+tail bias; the taxonomy here is the vocabulary for `--piece-selector`.

`bitfield.rs` is a 1-bit-per-piece bitfield with the memory argument written out
(10 000 pieces: 10 000 B as `Vec<bool>` vs 1 250 B).

Engine side, all under `aria2_rust/aria2-core/src/engine/`: `bt_peer_storage/`
(including `rejection_state.rs` and blocklist tests),
`bt_message_handler/peer_message_handler/` (`choke_state.rs`,
`request_lifecycle.rs`, `maintenance.rs`), `bt_peer_interaction/` with tests
named `choking_interest.rs`, `keepalive_flooding.rs`, `piece_exchange.rs`,
`state_machine.rs`,
`bt_download_execute/execute/{dht_periodic_lookup,peer_management,piece_download}.rs`,
and `bt_download_command_tests/private_torrent.rs`.

`docs/comprehensive_gap_analysis.md` retains a "BitTorrent Core" section
(completed vs remaining gaps) and an "LPD" section, plus "RPC Method Coverage
(36/36)" — the aria2 RPC surface `bit-cli` T-201 would have to match.
`docs/performance-differentiators.md` §4 covers BitTorrent and DHT specifically
(reducing repeated scans, bounding concurrency); it is written in Chinese.

### Issues

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 38](https://github.com/balovess/aria2_rust/issues/38) | CLOSED | 空闲超时被当作了下载时长限制 ("idle timeout treated as a download-duration limit") | `timeout=30` in the config should mean *idle* timeout, but the release treated it as a wall-clock limit, so a healthy download at full speed was aborted after 30 s. `bit-cli` has both `--timeout` (deadline) and `--stop-timeout` (no progress) and documents the difference; this issue is the failure mode when they are conflated, and is worth citing in the docs. |
| [Issue 2](https://github.com/balovess/aria2_rust/issues/2) | CLOSED | bug win系统上总是下载自己 ("on Windows it always downloads itself") | On start-up the binary treated its own path as a URL, reproducibly. A positional-argument classification bug — `bit-cli` accepts paths, URLs, magnets, bare info hashes and `-`, so its source resolver has the same ambiguity to get right (exit code 4). |
| [Issue 23](https://github.com/balovess/aria2_rust/issues/23) / [Issue 14](https://github.com/balovess/aria2_rust/issues/14) / [Issue 3](https://github.com/balovess/aria2_rust/issues/3) | CLOSED | RPC progress tracking; RPC completeness; RPC should be independent of task lifetime | Three separate reports converging on the same design point: the RPC service must not be coupled to whether a download task exists. Phase C (T-201, T-205, T-206) design input. |

---

## 15. `FluxDown` — zerx-lab/FluxDown

- Upstream: <https://github.com/zerx-lab/FluxDown> (taken from the repository's
  own README before cleaning; this repository was not on the supplied list).
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\FluxDown`
- Licence: **MIT** (`FluxDown/LICENSE`).
- Language: Rust engine (Flutter/web/extension trees removed during cleaning).
- **It vendors `librqbit` 8.1.1** at
  `FluxDown/native/engine/vendor/librqbit/` — the same engine `bit-cli` uses,
  unpatched, which makes it a readable copy of upstream for cross-checking.

### The librqbit `create_torrent` defect behind `bit-cli` T-080

Moved to [`HISTORY/RESEARCH-create-torrent-defect.md`](HISTORY/RESEARCH-create-torrent-defect.md) on 2026-08-24: T-080 closed and the
behaviour is in the tool.

### NTFS sparse files — why `--file-allocation sparse` matters

`FluxDown/native/engine/src/bt_sparse.rs` (module doc, lines 1-22, in Chinese)
states the mechanism precisely: `FilesystemStorage` calls `set_len` to the full
size after the initial check. On NTFS, `set_len` on a **non-sparse** file
reserves every cluster immediately (free space drops at once) **and** every
subsequent high-offset write zero-fills `[VDL, offset)` under valid-data-length
semantics. Because BitTorrent piece arrival is near-random, the first high-offset
piece triggers a zero-write across the entire prefix — "an order-of-magnitude
write amplification on large torrents, and the direct source of periodic
download-rate collapse".

The fix at `:36` `sparse_fs_factory` wraps `FilesystemStorageFactory` and sets
`FSCTL_SET_SPARSE` (`:144`) on every non-padding file **after `init` creates it
and before any `set_len` or write**. The doc is equally clear about the cost:
sparse files give up early `ENOSPC` detection (it becomes a per-piece write
error) and cluster contiguity. `ext4`/APFS `set_len` is already sparse, so the
module is Windows-only.

`bit-cli` defaults `--file-allocation` to `sparse` and measures volume free
space in `check-allocation.ps1`. This is the *why* behind that default, with the
write-amplification mechanism named.

### Seeding a partially selected torrent

`FluxDown/native/engine/src/bt_partfile.rs` — the problem, from its module doc:
`FilesystemStorage::init` creates **every** non-padding file regardless of
`only_files`, so re-adding a completed torrent to seed recreates all unselected
files as 0-byte placeholders in the user's directory; and the bytes of
unselected files that fall inside a **cross-file-boundary piece** were deleted
with the staging directory, so those boundary pieces can no longer be verified
or uploaded.

The solution is a `<task_id>.parts` sidecar (`:47` magic `FXPARTS1`, `:48`
version, `:50` a 16 MiB header cap, JSON header + blob) holding (1) the
file-id → final-path map, since completion flattens and de-duplicates names, and
(2) the boundary-byte ranges extracted from the staging by-products *after* the
piece passed its hash. On re-add, `:350` `PartsSeedStorageFactory` maps selected
files to their final paths (never creating) and routes everything else to the
blob.

`bit-cli` has `--select-file` and a `seed` command; this is the edge case that
turns "selected files only" into a torrent that cannot honestly seed its
boundary pieces.

### Padding-file handling and verification

`FluxDown/native/engine/src/bt_downloader.rs`:

- `:2744` `verify_pieces_core` with `:2715` `VerifyFileSpec { path: Option<…>,
  len, selected }` — **a padding file has `path: None` and contributes virtual
  zero bytes with no disk I/O**. Tests at `:5603-5900`:
  `verify_pieces_core_accepts_valid_data`,
  `..._detects_zero_filled_pieces`,
  `..._flags_missing_and_truncated_files`,
  `..._skips_unselected_only_pieces`, and
  `..._hashes_padding_as_zeros` (a 2-byte real file plus 2 bytes of padding
  hashing as `"XY\0\0"`).
- `:3611-3634` filters BEP 47 padding files out of the user-visible file list by
  path heuristic (a `/.pad/` component or a `.pad`-prefixed basename) **while
  keeping the true metainfo index**, because the indices are handed back to
  `update_only_files`. Off-by-one bait; `bit-cli`'s `--select-file N` and
  `-O/--index-out` have the same hazard once BEP 47 exists.
- `:1976` `compute_completion_layout` — the rules for where a finished torrent's
  files land (container vs flat, `custom_name`, dedup against siblings and
  against names claimed by other in-flight tasks), including a Windows-specific
  defence: never reuse a sentinel name that is occupied by a *file* when a
  directory is expected, because `rename(dir, existing_file)` can silently
  swallow it.

### Multi-source HTTP

`FluxDown/native/engine/src/segment_coordinator.rs` (6939 lines) is an IDM-style
dynamic segmenter: a worker pool rather than fixed segments, so a finished
worker either takes a pending segment or **splits the largest in-progress
segment in half**. The module doc states the invariants (segments exactly cover
`[0, total-1]` with no gaps or overlaps) and the crash-safety story (the segment
map is rebuilt from the database; a gap is caught by the end-of-download
integrity check). It also handles a server revising the total size upward
mid-download via the `Content-Range` denominator, with a bounded number of
in-place expansions so a still-uploading file cannot loop forever.

`bit-cli`'s `--web-seed-connections N` presents one source over N connections
sharing a window cache and concurrency budget; the split-in-half rule is the
adaptive alternative when one connection turns out to be slow.

No Issues or Pull Requests were fetched for this repository — it was not on the
supplied list.

---

## 25. `Seedr`, rursache/Seedr

- Upstream: <https://github.com/rursache/Seedr>
- Local path: `reference/Seedr`
- Commit: `33d3430ec6bfd3750f7b5bf858f98359bd2ab25e`, cloned 2026-08-24.
- Licence: **MIT** (`Seedr/LICENSE`, Copyright 2025 Radu Ursache).
- Language: TypeScript. One open issue, two pull requests, both dependency
  bumps.

`Seedr/clients/README.md` is the best written statement of the profile model in
this corpus, and it is the reason this entry is here rather than the code. It
names the thing [T-234](../TODO/peers.md) is about in its second paragraph:
trackers running UNIT3D or Gazelle fingerprint on the peer id, the key, the
headers, the query parameter order and the encoding together, and getting one
wrong produces a refusal rather than a warning.

### What the document gets right, and it is most of it

`Seedr/clients/README.md:33-41` is the version encoding table by client:
qBittorrent and Transmission `{major}{minor}{patch}0`, Deluge
`{major}{minor}{patch}s`, uTorrent `{major}{minor}{patch}S`, BitTorrent
`{major_hex}{minor_hex}{patch}S`. The fourth character is not padding. It is a
release-stage character for three of the five.

`:69-77` is the point about raw bytes: uTorrent and BitTorrent peer ids are not
text, and the profile writes them as escapes that map to byte values.

`:180` is the one every reimplementation should read first: byte `0x7E`,
the tilde, is left unencoded by qBittorrent and percent-encoded by
Transmission, so the **encoder** is per client and not a library default.

`:196` states the rule plainly: the order of the query parameters is part of
what a tracker checks.

### Two places where the document and the code disagree with the client

Both were found by reading the code after the document, which is the order
section 7 of `TODO/RULES.md` asks for.

**The Transmission checksum formula is wrong.**
`Seedr/clients/README.md:93` says the checksum is
`pool[sum_of_random_character_indices % base]`, and
`Seedr/src/core/client-emulator/generators.ts:74` implements exactly that:
`const checksumIdx = sum % base`.

Transmission computes the complement.
`libtransmission/session.cc:205`, tag `4.1.0`:

```cpp
int const val = total % std::size(Pool) != 0 ? std::size(Pool) - (total % std::size(Pool)) : 0;
```

The two agree only when `sum % 36` is 0 or 18, so **a Seedr generated
Transmission peer id carries the correct checksum digit about one time in
eighteen.** Any tracker that validates the digit sees the rest. This is the
worked example for why [T-234](../TODO/peers.md) says a profile has to be
checked against the client rather than copied from another emulator.

**`HASH_NO_LEADING_ZERO` is not what libtorrent does**, and Seedr is one of
four implementations that are wrong in the same direction. See entry 27.

### One more thing worth knowing before copying the regex approach

`Seedr/src/core/client-emulator/generators.ts:42-56` generates a peer id from
the profile's regex, then masks every produced character with `& 0xff`. The
profiles all use explicit character classes so nothing is currently truncated,
but a pattern that ever produces a code point above `0x00ff` would silently
become a different byte rather than an error.

### Passes

Three. WHAT: the profile format explained for a reader. MECHANISM: the four
generator functions at `generators.ts:7-78`. THE HARD PART: the checksum and
the key, where the document, the code and the client are three different
answers. There is no fourth pass: the tracker is three items and two of them
are dependency bumps, and the third,
[Issue 3](https://github.com/rursache/Seedr/issues/3), asks for the same
torrent under two passkeys, which is not about identity.

### Verdict

**ADOPT the document, ANTI-PATTERN EXHIBIT for the code.**
`Seedr/clients/README.md` is the source for the profile field list in
[T-234](../TODO/peers.md). The checksum defect is recorded because its own
documentation states the wrong formula confidently, which is how the wrong
formula travels.

---

## 26. `RatioForge`, tsautier/RatioForge

- Upstream: <https://github.com/tsautier/RatioForge>
- Local path: `reference/RatioForge`
- Commit: `7870575c5288052ac7c3016fd0dd141f8e20d5a0`, cloned 2026-08-24.
- Licence: **MIT** (`RatioForge/LICENSE`, Copyright 2006-2016 Nikolay Kostov).
  The copyright is somebody else's on purpose:
  [Issue 3](https://github.com/tsautier/RatioForge/issues/3) records that this
  is a .NET 8 port of `NikolayIT/RatioMaster.NET`, and the upstream licence and
  holder are carried across unchanged. That is the correct handling of a fork,
  and it is worth naming beside entry 24, where the same situation was
  resolved the other way.
- Language: C#. Five issues, all closed, and four pull requests.

One file was read, on the operator's instruction:
`RatioForge/docs/client-profile-audit-2026-08-11.md`, 22 lines. It is a
procedure rather than an implementation, and the procedure is the thing to
take.

### The verification rule, and it is the one to adopt

`RatioForge/docs/client-profile-audit-2026-08-11.md:22`:

> Release recency alone is insufficient. The implementation requires an
> official source for the version, peer ID construction, and User-Agent.

The table above it gives four profiles with three links each: the tagged
release, the file holding the version constants, and the file holding the peer
id construction. `-qB5230-` for qBittorrent 5.2.3, `-TR4130-` for Transmission
4.1.3, `-BI4100-` for BiglyBT 4.1.0.0, and `-KT26043-` for KTorrent 26.04.3.

`scripts/make-client-profile.ps1` derives the first two from those same
constants and agrees with both.

### What it refused, which is the more useful half

`:17-19` records two refusals with the reason:

- **BitComet 2.21** exists and is dated, and no profile was added because
  BitComet publishes neither source nor a tracker signature, so the peer id and
  the headers could not be verified.
- **uTorrent and BitTorrent Classic** publish no stable build number on their
  download pages, so the existing profiles were kept and no newer one was
  inferred **from third party client-identification tables**.

That second refusal is the discipline. The tables exist, they are easy to copy,
and copying one is how a profile that no client has ever emitted enters a
profile set.

### KTorrent, and why a peer id is bytes

`-KT26043-` is nine characters, then **one NUL byte**, then ten alphanumeric
characters. A peer id field that is typed as a string in any language with
NUL-terminated strings cannot hold it. `bit-cli`'s literal escape hatch has to
take bytes, and the machine output that reports what was advertised has to
encode them.

### Passes

Three, and the fourth is stated as not taken. WHAT: an audit record for a
profile set. MECHANISM: the evidence chain per profile, three links each.
AGAINST `bit-cli`: the verification rule becomes the acceptance condition for
[T-234](../TODO/peers.md), and the KTorrent shape sets the type of the
override flag. The fourth pass, reading the C# emulation itself, was not taken:
the operator named one file and the audit is what answers the question.

The tracker was read in full. Nine items, five issues all closed, and the two
that bear on this are
[Issue 3](https://github.com/tsautier/RatioForge/issues/3) and
[Issue 7](https://github.com/tsautier/RatioForge/issues/7), which are requests
for one more emulated client each.

### Verdict

**ADOPT.** The verification rule and the refusal discipline go into
[T-234](../TODO/peers.md) and into `docs/reference-mining.md`.

---

## 31. `tcp-transfer-ice`, well0nez/tcp-transfer-ice

- Upstream: <https://github.com/well0nez/tcp-transfer-ice>
- Local path: `reference/tcp-transfer-ice`
- Commit: `9be46cc06331039480c2a4c805d1d3e0ff6048fe`, cloned 2026-08-24.
- Licence: **MIT** (`tcp-transfer-ice/LICENSE`, Copyright 2026 well0nez).
- Language: Rust, 2,444 lines across four files, plus one Python relay server.
  Zero issues and zero pull requests.

**Despite the name it does not use ICE.** `tcp-transfer-ice/Cargo.toml`'s
dependencies are `tokio`, `bytes`, `serde`, `sha2` and `clap`. There is no
`webrtc-ice` and no ICE state machine. The traversal is hand-rolled TCP
simultaneous open with a relay used for signalling. Read the code, then the
name.

### The one mechanism BEP 55 does not have

BEP 55 works because a rendezvous peer tells both sides an `IP:port` that is
already known. **It has nothing to say when the NAT allocates a different
external port per destination**, which is what a symmetric NAT does, because
the port the rendezvous peer saw is not the port the target will see.

This tree answers exactly that case, and the README states the method at
`:27-38`:

- The client opens several probe connections to a known port. The server
  records `(local_port, observed_public_port, timestamp)`.
- `delta = public_port - local_port`, `predicted_port = local_port +
  median(delta)`, and an error range of the maximum deviation plus twice the
  standard deviation, floor 2.
- For a NAT whose allocation progresses, it estimates a port allocation **rate**
  and shifts the prediction forward by `port_rate * prediction_delay`, damped
  and capped.
- It classifies the observed pattern as `port_preserved`, `constant_delta`,
  `small`/`medium`/`large_delta_range`, or `random_like`, and builds a bounded
  candidate list: a contiguous window for the predictable patterns, and for
  `random_like` a sparse sample across the observed range.

`src/hole_punch.rs:106-111` is the part every implementation needs and most get
wrong: `set_reuse_address(true)` and, on Unix, `set_reuse_port(true)`, so the
listener and the outbound connector can share one local port. Without it there
is no simultaneous open. `:33-36` carries a four byte pre-handshake magic
`HPCH`, a one second pre-handshake timeout, and a 300 ms grace window, so a
connection that opens is confirmed to be the peer rather than a scan.

### What it says about its own results, and it is honest

The README does not claim a success rate. It says which knob helped
(`--prediction-range-extra-pct` at 20 to 50), that a higher scan cap only helps
because it lets the wider window actually be attempted, and that "results still
depend on the networks and devices involved". It also states the cost: more
outbound connection attempts, and possible throttling by the NAT or the ISP.

### Passes

Three. WHAT: TCP file transfer with relay-signalled hole punching. MECHANISM:
the prediction model in the README at `:27-38` and the socket options at
`src/hole_punch.rs:106-111`. AGAINST `bit-cli`: this is the one traversal
technique in the group that adds reach BEP 55 cannot, and it is also the one
that costs a burst of connection attempts, which is a thing a BitTorrent client
must be careful about. No tracker pass to take.

### Verdict

**ADOPT as evidence, not as code.** It is the citation behind
[T-238](../TODO/peers.md)'s claim that symmetric NAT is the shape where
relay-assisted traversal and port prediction earn their cost, and behind the
statement that the cost is measured in connection attempts.

---

## 32. `iroh-fm`, usagi-coffee/iroh-fm

- Upstream: <https://github.com/usagi-coffee/iroh-fm>
- Local path: `reference/iroh-fm`
- Commit: `d09fcf1953db0701f170c4cb60b76d16940348f4`, cloned 2026-08-24.
- Licence: **MIT** (`iroh-fm/LICENSE`, Kamil Jakubus and contributors).
- Language: Rust plus TypeScript. A music server reached over iroh. Zero issues
  and zero pull requests.

Read for one thing: what adopting `iroh` actually looks like at version 1.0.3
(`iroh-fm/Cargo.toml:19`), because the operator reopened that question.

### The API surface a dependent touches

`iroh-fm/crates/server/src/iroh_rpc.rs`:

- `:251-254` a server is `endpoint_builder(config).alpns(vec![IROH_ALPN]).bind()`,
  then `endpoint.accept()` in a loop.
- `:47-58` a client is `EndpointAddr::new(endpoint_id)`, optionally
  `.with_relay_url(relay_url)`, then `endpoint.connect(addr, ALPN)`.
- `:28` the relay is one optional string of configuration.

So the integration surface is small. What is not small is the identity model:
every address is an `EndpointId`, an ed25519 public key, and there is no form
of `EndpointAddr` that is an ordinary `IP:port` a qBittorrent peer could dial.

### The one thing worth copying outright

`:411-437` `connection_path_label` reports, per connection, whether the
**selected** path is `relay`, `ip` or `custom`, with its round trip time:

```rust
let kind = if path.is_relay() { "relay" }
           else if path.is_ip() { "direct" }
           else { "custom" };
```

and `:241-247` logs it only when it changes, so a connection that migrates from
relay to direct says so once rather than every tick.

**That is the observability a traversal feature needs and BEP 55 does not
have.** `bit-cli`'s peer report says a peer is connected. It does not say
whether the path is direct, and after a hole punch that is the whole question.
It is filed in [T-239](../TODO/peers.md) independently of whether any relay is
ever adopted, because it applies to BEP 55 too.

### Passes

Three. WHAT: a Subsonic music server over iroh. MECHANISM: the endpoint and
connection API above. AGAINST `bit-cli`: the path label transfers, the identity
model does not. No fourth pass: the rest of the tree is a music indexer.

### Verdict

**ADOPT one mechanism**, the selected-path label, into
[T-239](../TODO/peers.md). **REFUSED** as an argument for adopting `iroh`
itself; see [T-238](../TODO/peers.md) for what decides that.

---

# Tier 3 — narrower but verified

## 16. `aquatic` — greatest-ape/aquatic

- Upstream: <https://github.com/greatest-ape/aquatic>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\aquatic`
- Licence: **MIT** (`aquatic/LICENSE`).
- Language: Rust. UDP, HTTP and WebTorrent trackers. The `udp` crate is
  described upstream as production-ready and is used by `explodie.org`.

### Peer-ID → client identification (BEP 20)

`aquatic/crates/peer_id/src/lib.rs` — `:42` `PeerClient`, `:66`
`from_prefix_and_version`, `:148` `from_peer_id`. Three regexes in priority
order: Azureus style `-XX1234-`, mainline style `M1-2-3-`, then a generic
`prefix-`. Per-client version formatting is not uniform: `qB` is `x.y.z`
(three digits), `LT`/`lt` are `x.yz.w`, `AZ` is `x.y.z.w`, `TR` has three
special cases for old Transmission versions, and `BT`/`DE`/`UT`/`UE`/`UM`/`UW`
use a three-digit version plus a pre-release letter (`d`/`a`/`b`/`r`/`s` →
dev/alpha/beta/rc/stable). `bit-cli bench probe` prints a `client` line; this
is the canonical table.

### WebTorrent tracker protocol

`crates/ws_protocol/src/`: `aquatic/crates/ws_protocol/src/incoming/announce.rs:13` `AnnounceRequest` — the
comments record the reference client's actual behaviour, which the BEP does not:
`left` may be absent (e.g. when opening a magnet), `offers` is only sent when
the client wants offers relayed, its length **is** the peer count wanted, the
reference client caps it at 10, and offers are not sent for `stopped` or
`completed`. `aquatic/crates/ws_protocol/src/outgoing/offer.rs` and
`.../outgoing/answer.rs` carry the relay messages; both note that a client ignores an offer/answer whose `peer_id` equals
its own.

### UDP connection IDs

`crates/udp/src/workers/socket/validator.rs` — `ConnectionId` is
`[4 bytes: seconds since start][4 bytes: truncated keyed BLAKE3 of those bytes
and the client IP]`, validated in constant time and expiring after
`max_connection_age`. The doc explains why: forging one costs ~2^31 attempts on
average and is worthless once it expires. This is the mechanism behind the
"Connection ID missmatch" errors anacrolix works around — a client that caches a
connection id too long **will** be rejected, so `bit-cli`'s UDP tracker client
needs a reissue rule (T-064).

### HTTP tracker response shape

`crates/http_protocol/src/response.rs` — `complete`, `incomplete`, `peers`,
**`peers6` at 18 bytes per entry** (BEP 7), and an optional
`warning message` key. `aquatic/crates/udp_protocol/src/request.rs:386-403` is a byte-by-byte BEP 15
announce-request test, field by field down to the trailing `Extensions` u16.

`documents/aquatic-udp-load-test-2024-02-10.md` was kept; the PDFs, PNGs and the
architecture SVG were removed as images.

### Issues / PRs

| # | State | Title | Why it matters |
|---|---|---|---|
| [PR 254](https://github.com/greatest-ape/aquatic/pull/254) | MERGED | ws: allow left / bytes_left to be negative | **Some clients send `left = -1` when the length is unknown** rather than omitting the parameter, and a `usize` parse rejected the whole announce. The PR cross-references anacrolix/torrent#981. `bit-cli` both sends announces (`trackers`, and `started`/`completed`/`stopped` during `download`) and parses responses — accept a negative `left` on the way in, and decide deliberately what to send before metadata arrives. |
| [PR 221](https://github.com/greatest-ape/aquatic/pull/221) / [PR 220](https://github.com/greatest-ape/aquatic/pull/220) | MERGED | http/udp: use separate sockets for IPv4 and IPv6 | The dual-stack conclusion `bit-cli` T-023 reached from the other side ("the listen port is chosen without checking both address families"). |
| [Issue 232](https://github.com/greatest-ape/aquatic/issues/232) | CLOSED | What is canonical announce path for UDP server? | There isn't one — the path in a `udp://` announce URL is advisory (BEP 41 carries it as an option if you want it). A client must not assume `/announce`. |
| [Issue 82](https://github.com/greatest-ape/aquatic/issues/82) | CLOSED | Tracker reply has no peers field | µTorrent 2.2.1 reporting a missing `peers` key — a reminder that an empty swarm response must still be well-formed. |
| [Issue 249](https://github.com/greatest-ape/aquatic/issues/249) / [PR 239](https://github.com/greatest-ape/aquatic/pull/239) | CLOSED / MERGED | BEP 48 full scrape export | The server side of the scrape `bit-cli trackers --scrape` performs. |
| [Issue 256](https://github.com/greatest-ape/aquatic/issues/256) | OPEN | list supported BEPs in readme | A reminder that `bit-cli`'s protocol-support table is a genuine differentiator. |
| [Issue 227](https://github.com/greatest-ape/aquatic/issues/227) | OPEN | Rate-limit number of WebSocket messages per connection per second | Server-side, but the reason (a WebTorrent client can flood offers) informs any client that speaks WSS. |

---

## 17. `torrust-actix` — Power2All/torrust-actix

- Upstream: <https://github.com/Power2All/torrust-actix>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\torrust-actix`
- Licence: **MIT** — `torrust-actix/LICENSE` carries the MIT permission text
  under "Copyright (c) 2024-2026 Power2All" without the "MIT License" heading.
  The upstream `Cargo.toml` also declared `license = "MIT"`; it was removed in
  the manifest sweep described in §G.
- Language: Rust (Actix). Tracker implementing BEP 3, 7, 15, 23, 41, 48.

### `RtcTorrent.md` — a complete WebRTC-BitTorrent white paper

`torrust-actix/RtcTorrent.md` (937 lines, version 4.2.0) is the most detailed
protocol document in the corpus after the BEP texts themselves, and it is
self-contained: tracker announce extensions and their query parameters, the
four-step signalling flow, the tracker-side data model, the WebRTC data-channel
message types (`MSG_PIECE_REQUEST 0x01`, `MSG_PIECE_DATA 0x02`,
`MSG_PIECE_CHUNK 0x04`), chunked transfer, flow control, a client
implementation guide (ICE/SDP lifecycle, announce loop, in-flight request
management, piece verification, peer speed monitoring and blacklisting,
**BEP 19 WebSeed fallback**), torrent-format support, the tracker implementation
guide, URL encoding, CORS, and a glossary.

Two sections are valuable regardless of whether `bit-cli` ever speaks WebRTC:

- **§10 Torrent Format Support** — a compact, correct restatement of v1, v2
  (16 KiB blocks, SHA-256 leaves, padded to the next power of two with zero
  hashes, interior nodes are `SHA-256(left || right)`, roots in `file tree`,
  layers in the top-level `piece layers`, `piece length` a power of two ≥ 16 KiB,
  no `pieces`), and hybrid (`magnet:?xt=urn:btih:<v1>&xt=urn:btmh:1220<v2>`,
  and the parsing rule "if either `file tree` or `meta version: 2` is present,
  treat as v2/hybrid").
- **§14 Known Pitfalls** — five real defects found and fixed, each with symptom,
  cause and fix. §14.1 (a re-announce discards answers deposited since the last
  poll, because the peer entry is removed and reinserted with an empty queue) is
  a general lesson about state that must survive a re-announce. §14.2 is
  double-percent-encoding from `URLSearchParams.set()` on an already-encoded
  value — the same hazard as `bit-cli`'s `{raw:path}` template placeholder.
  §14.4 is a 20-byte binary `peer_id` that must hex-encode to exactly 40
  characters.

§15 states the interop posture: RTC is purely additive, non-RTC clients see only
an extra `"rtc interval"` key they ignore, and mixed swarms work.

### Tracker internals

`src/udp/impls/` — `request.rs`, `response.rs`, `udp_server.rs`, plus three
receive strategies (`batch_recv.rs` using `recvmmsg`, `io_uring_recv.rs`,
`rio_recv.rs` for Windows Registered I/O) and `simple_proxy_protocol.rs` for
Cloudflare's UDP proxy. `src/udp/structs/` has one type per wire structure.
`src/tracker/impls/torrent_sharding.rs` shows how a large peer table is sharded.

### Issues

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 36](https://github.com/Power2All/torrust-actix/issues/36) | CLOSED | Allowing Dual Stack Peers (IPv4 and IPv6) | A peer announcing over IPv6 only was recorded with its IPv6 address, so IPv4-only clients could never find it, even with both `tracker.` and `tracker6.` URLs in the torrent. The client-side lesson: announce to a dual-stack tracker over both families, or accept that half the swarm cannot reach you. Relevant to `bit-cli` T-022 and T-023. |
| [Issue 14](https://github.com/Power2All/torrust-actix/issues/14) | CLOSED | Broken UDP IPv4 handling | A typo in the IPv4 UDP path; corroborates that dual-stack UDP is easy to get wrong. |
| [Issue 30](https://github.com/Power2All/torrust-actix/issues/30) / [Issue 2](https://github.com/Power2All/torrust-actix/issues/2) | CLOSED | Adding WebTorrent support / WebSocket support for WebTorrent | The tracking issues behind `RtcTorrent.md`. |
| [Issue 9](https://github.com/Power2All/torrust-actix/issues/9) | CLOSED | Improving the dead peers scanner | Peer-expiry policy, from the tracker's side of the announce interval. |

---

## 18. `create-torrent` — webtorrent/create-torrent

- Upstream: <https://github.com/webtorrent/create-torrent>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\create-torrent`
- Licence: **MIT** (`create-torrent/LICENSE`, © Feross Aboukhadijeh and
  WebTorrent, LLC).
- Language: JavaScript. Small (401 lines in `index.js`) and worth reading in full.

- `index.js:16-24` — the default `announceList` mixes `udp://` trackers with
  **`wss://` WebTorrent trackers** (`tracker.btorrent.xyz`,
  `tracker.openwebtorrent.com`, `tracker.webtorrent.dev`), each in its own
  BEP 12 tier. If `bit-cli` ever ships default trackers, that tier-per-URL
  structure is the convention.
- `:114-118` — junk filtering is on by default (`filterJunkFiles`), using the
  `junk` package plus a hidden-file rule (`:336` `isJunkPath`).
- `:211` `MAX_OUTSTANDING_HASHES = 5` — backpressure on the hashing pipeline.
- `:252-275` — `announce` (string or array) is upconverted to `announceList`,
  and `globalThis.WEBTORRENT_ANNOUNCE` is merged; the defaults are appended
  **only when the caller supplied neither `announce` nor `announceList`**.
- `:295` `torrent.info.private = Number(opts.private)` — BEP 27 is an integer
  inside `info`, not a boolean, and not at the top level.
- `:305` `torrent['url-list'] = opts.urlList` — BEP 19, top level.
- `:308` piece length is `min(calcPieceLength(totalSize), opts.maxPieceLength)`
  with `maxPieceLength` defaulting to 4 MiB (`:169`) — a *fifth* piece-length
  policy, and the lowest cap of the five.

### Issues

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 126](https://github.com/webtorrent/create-torrent/issues/126) | OPEN | Duplicate paths should not be allowed | Two input files with the same relative path produce a torrent with duplicate entries, which is invalid. A cheap lint for `bit-cli create`, alongside its existing `case-collision`. |
| [Issue 195](https://github.com/webtorrent/create-torrent/issues/195) | CLOSED | `ENOENT` when folder contains non-unicode character | `mkdir $'\344'` then create — the tool cannot stat its own input. The same class as `bit-cli` T-103 ("filenames that are not valid UTF-8 are refused"), on the creation side. |
| [Issue 53](https://github.com/webtorrent/create-torrent/issues/53) | CLOSED | File paths should be normalized | Un-normalised input (`/foo//bar/baz.txt` against root `/foo/bar/`) produced wrong relative paths. |
| [Issue 51](https://github.com/webtorrent/create-torrent/issues/51) | CLOSED | Junk of `@eaDir` subdirectory included on Synology | The same NAS directory mkbrr hardcodes in `ignoredDirNames`. |
| [Issue 265](https://github.com/webtorrent/create-torrent/issues/265) | CLOSED | web seeds / https paths for torrent files | Populating `url-list` at creation. |
| [Issue 177](https://github.com/webtorrent/create-torrent/issues/177) | OPEN | BitTorrent v2 support | Tracking issue. |

---

## 19. `parse-torrent` — webtorrent/parse-torrent

- Upstream: <https://github.com/webtorrent/parse-torrent>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\parse-torrent`
- Licence: **MIT** (`parse-torrent/LICENSE`, © Feross Aboukhadijeh and
  WebTorrent, LLC).
- Language: JavaScript, 262 lines. A compact catalogue of metainfo edge cases.

- `parse-torrent/index.js:27` — an info hash is accepted as **40 hex characters
  or 32 base-32 characters** (`/^[a-z2-7]{32}$/i`). `bit-cli` accepts "a bare info hash";
  base-32 `urn:btih:` values exist in the wild and are the same 20 bytes.
- `parse-torrent/index.js:123-131` — required fields are `info`, `info.name`
  **or** `info['name.utf-8']` (`:124`), `info['piece length']`, `info.pieces`;
  per-file, `path` **or** `path.utf-8` (`:131`). The `.utf-8` variants are
  preferred throughout (`:140` for the name, `:181` for each file path). This
  is intermodal Issue 534 handled correctly.
- `parse-torrent/index.js:155-163` — `announce-list` is **flattened** into a single `announce` array,
  and `parse-torrent/index.js:212-216` in `encodeTorrentFile` re-nests each URL
  into **its own tier**. That
  round trip destroys BEP 12 tier structure, which is
  [Issue 152](https://github.com/webtorrent/parse-torrent/issues/152) (OPEN,
  with a minimal bencoded reproduction in the body). `bit-cli edit` must
  preserve tiers exactly.
- `parse-torrent/index.js:166-170` — `url-list` set to an empty string by some
  clients is normalised
  to `[]`, and a non-empty string is wrapped into a one-element list; both
  `announce` and `urlList` are then de-duplicated via a `Set`.
- `parse-torrent/index.js:181-188` — file `offset` is the running sum, and
  `:196` `lastPieceLength = ((lastOffset + lastLength) % pieceLength) ||
  pieceLength`.
- `parse-torrent/index.js:221` — **`encodeTorrentFile` writes `private` at the
  top level** (`torrent.private = Number(parsed.private)`), while
  `decodeTorrentFile` reads `torrent.info.private`. A round trip therefore loses
  the flag (the info dict itself is passed through verbatim, so the info hash is
  unaffected). Noted as a defect to avoid in `bit-cli edit`.

### Issues

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 152](https://github.com/webtorrent/parse-torrent/issues/152) | OPEN | Roundtripping torrent files through parse-torrent modifies announce lists | See above. Directly applicable to `bit-cli edit`. |
| [Issue 89](https://github.com/webtorrent/parse-torrent/issues/89) | CLOSED | Zero-length file path segments disappear when they get `path.join`ed | A torrent with `path: ["", "foo"]` and one with `path: ["foo"]` are stored differently by at least one common client, but `path.join` collapses them to the same string. `bit-cli`'s path planner must decide what an empty component means and report it like any other rename. |
| [Issue 177](https://github.com/webtorrent/parse-torrent/issues/177) | OPEN | Support utf-8 encoded comment field | `comment.utf-8` exists too, not just `name.utf-8`/`path.utf-8`. |
| [Issue 88](https://github.com/webtorrent/parse-torrent/issues/88) / [PR 193](https://github.com/webtorrent/parse-torrent/pull/193) | OPEN | BitTorrent v2 Support [BEP 52] | Tracking issue and an open PR. |
| [PR 198](https://github.com/webtorrent/parse-torrent/pull/198) | MERGED | Greatly improve efficiency with torrents with a large number of files | Parsing cost is superlinear if you are not careful; `bit-cli` handles twenty-thousand-file torrents (`--max-open-files`). |

---

## 20. `bqti` — OnlyCavas/bqti

- Upstream: <https://github.com/OnlyCavas/bqti>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\bqti`
- Licence: **MIT** (`bqti/LICENSE`).
- Language: Rust. A BitTorrent client with an authenticated Kademlia DHT, an
  I2P transport, and optional TEE/enclave piece attestation. There is no
  top-level README. The 8.4 MiB `bqti-enclave/vendor/` tree (keystone,
  libtomcrypt, tweetnacl) was removed during cleaning as unrelated C.

What is useful to `bit-cli`:

- `src/bit_torrent/torrent/metainfo/v2.rs` — a v2 `FileTreeNode` /
  `FileTreeEntry` model with `piece_layers: HashMap<MerkleRoot, PieceByte>`,
  built from a bencode file-tree node.
- `src/bit_torrent/torrent/builder/{v1_builder.rs,v2_builder.rs}` — a builder
  API for both versions, though `v2_builder.rs` is **incomplete**: `piece_layers`
  and `file_tree` are `#[allow(dead_code)]` with a `NOTE improve torrent v2
  support`. Useful as an API shape, not as an implementation.
- `src/bit_torrent/session/bep/` — `router.rs` (peer/DHT/PEX orchestration with
  `DHT_ANNOUNCE_INTERVAL` 30 min, `DHT_DISCOVERY_INTERVAL` 1 min,
  `PEX_FALLBACK_INTERVAL` 1 min, `REBOOTSTRAP_INTERVAL` 30 s),
  `pipeline.rs` (`MAX_PIPELINE = 128`, `BLOCK_SIZE = 16384`),
  `piece_auth.rs` (signed piece payloads — a non-standard extension),
  `message.rs`.
- `src/i2p/` — SAM-protocol I2P transport (`builder.rs`, `dest_map.rs`,
  `session.rs`, `socket.rs`). The only anonymity-network transport in the corpus.

**A correctness caution.** `src/bit_torrent/torrent/merkle.rs:35`
`from_piece_hashes` reduces the layer by `chunks(2)`, hashing
`H(chunk[0] || 0^32)` whenever a level has an odd count. BEP 52 pads the
**layer to a power of two** with the layer's pad hash before reducing. The two
agree when the layer length is a power of two, and can diverge otherwise — for a
5-hash layer, the correct construction pairs the padding as `H(H(h4,P), H(P,P))`
while this produces `H(H(h4,0), 0)`. `bit-cli` should follow the rustorrent /
anacrolix / nanotorrent construction instead.

Pull requests (there are no issues):
[PR 2](https://github.com/OnlyCavas/bqti/pull/2) (MERGED) "feat(bep): implement
bep protocol", [PR 7](https://github.com/OnlyCavas/bqti/pull/7) (MERGED)
"feat(session): complete IPC, BEP, and session lifecycle overhaul",
[PR 1](https://github.com/OnlyCavas/bqti/pull/1) (MERGED) "kademlia DHT with
auth, rate limiter and store merge semantics" — the last is the only one with
research value, for its DHT store merge semantics and rate limiter.
[PR 8](https://github.com/OnlyCavas/bqti/pull/8) (MERGED) adds the I2P layer.

---

## 21. `dht-spider` — adysec/dht-spider

- Upstream: <https://github.com/adysec/dht-spider>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\dht-spider`
- Licence: **MIT** (`dht-spider/LICENSE`, © 2025 AdySec).
- Language: Rust, ~1500 lines total. Documentation is in Chinese.
- No Issues or Pull Requests exist upstream.

A compact BEP 5 crawler with BEP 9/10 metadata fetch and BEP 11 PEX. Its value
is size: the whole DHT is `src/dht.rs` (398 lines) and `src/routing.rs`
(275 lines), so it is readable in one sitting.

- `src/dht.rs:65` — the `Dht` struct is one line holding config, peer manager,
  blacklist, routing table, callbacks, socket, self id, token manager and
  transaction manager; `:102` `handle_request` and `:179` `handle_response` are
  the whole KRPC surface (`ping`, `find_node`, `get_peers`, `announce_peer`).
- Two modes are documented in the README: **Standard** (strict protocol) and
  **Crawl** (biased toward sniffing info hashes by provoking `announce_peer`).
- `dht-spider/src/routing.rs:11` `KBucketInner { nodes, candidates, prefix,
  last_changed }`
  — bucket splitting with a **candidate list** and ping-then-replace maintenance,
  with nodes kept sorted by `last_active` ascending.
- `src/transaction.rs` — exponential-backoff retry with a blacklist.
- `src/wire.rs:22` — the 28-byte handshake prefix with reserved bytes
  `0,0,0,0,0,0x10,0,0x01` (LTEP bit **and** DHT bit set); `:12-17`
  `REQUEST/DATA/REJECT`, `BLOCK = 16384`,
  `MAX_METADATA_SIZE = BLOCK * 1000` — a bound `bit-cli` should also have when
  fetching metadata from an untrusted peer.
- Output is JSONL, one event per line (`type=peer`, `type=metadata`,
  `type=node`) — the same discipline as `bit-cli --jsonl`.

---

## 22. `tc` — hnpf/tc

- Upstream: <https://github.com/hnpf/tc>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\tc`
- Licence: **MIT** (tc/LICENSE)
- Language: Rust. No Issues or Pull Requests exist upstream.

Early-stage: `src/core/dht.rs` is empty and every file under
`src/cli/commands/` is empty or a one-line stub. The substantive code is
`src/core/peer.rs` (911 lines — `Handshake` at `:9`, `PeerConnection` at `:124`,
`PeerState` at `:211`, `Message` at `:334`), plus `bencode.rs` (123),
`torrent.rs` (88), `tracker.rs` (149), `piece.rs` (95) and `storage.rs` (152).

Two things are worth taking:

1. **The design goal.** A git-style porcelain/plumbing CLI over a real wire
   protocol, with everything implemented from scratch and no torrent library —
   the same shape as `bit-cli`'s verb set (`download`, `info`, `files`, `peers`,
   `trackers`, `verify`, `create`, `edit`, `magnet`, `seed`).
2. **The one open plan item**, from the README's todo table: a background daemon
   so `tc status` can answer over a unix socket "without needing to spin up a
   swarm connection". That is `bit-cli` T-208 (`status --follow` against a live
   session) reached independently — and the reason is not queueing or
   persistence but simply that a status query should not have to join a swarm.

Fixture: `tc/test_data/ubuntu.torrent`.

---

## 27. `fake-torrent-client`, slundi/fake-torrent-client

- Upstream: <https://codeberg.org/slundi/fake-torrent-client>
- Local path: `reference/fake-torrent-client`
- Commit: `b59f1fedfa2ca976e526003c19ae5ae76333acbe`, cloned 2026-08-24.
- Licence: **MIT** (`fake-torrent-client/LICENSE`), and the template was never
  filled in: the file reads `Copyright (c) <year> <copyright holders>`.
- Language: Rust, 2,595 lines across three source files, a CSV of 63 profiles,
  and four shell scripts.
- Tracker: Codeberg rather than GitHub, so `gh` does not reach it. Read through
  the Forgejo API at `https://codeberg.org/api/v1/repos/slundi/fake-torrent-client/issues?state=all`,
  which answers without a credential: **one issue, open, "Add new verions of
  clients".** That is the whole tracker and it is the same request the other
  four projects carry.

The operator's instruction is that this is a reference and never a dependency.
It stays that way for a reason beyond the instruction: three of the mechanisms
in it do not produce what they name.

### The key alphabet, three ways wrong in fourteen lines

`fake-torrent-client/src/algorithm.rs:15-32`:

```rust
const HASH_SYMBOLS: &str = "abcdef0123456789ABCDEF";
...
let i: usize = rng.random_range(0usize..15usize);
if i == 0 && no_leading_zero { continue; }
if uppercase.is_none() || uppercase.unwrap() {
    h.push(HASH_SYMBOLS.chars().nth(i + 6).unwrap());
} else {
    h.push(HASH_SYMBOLS.chars().nth(i).unwrap());
}
```

The symbol table holds 22 characters. The draw is `0..15`, so `i` never reaches
15 and the last symbol of each half is unreachable: an upper case key never
contains `F`, and a lower case key never contains `9`.

`no_leading_zero` skips `i == 0` at **every** position rather than the first,
so an upper case key never contains `0` anywhere and a lower case key never
contains `a` anywhere. The flag does not do what its name says in either case.

An upper case `HASH_NO_LEADING_ZERO` key from this library is therefore drawn
from fourteen of the sixteen hex digits, at a fixed width of eight
(`src/lib.rs:7`, `KEY_LENGTH: usize = 8`). That is a distribution, and a
distribution is what a tracker sees over a few dozen announces.

### The Transmission checksum is absent

`src/algorithm.rs:45-70` `random_pool_with_checksum` fills the whole suffix
with `pool[byte % base]` and never computes a checksum character. The function
name is the only place the checksum appears. A `//TODO:` at `:55` sits directly
above the loop.

### The four implementations of one algorithm, and all four are wrong

This is the finding the whole client masking item turns on, and it is why
[T-234](../TODO/peers.md) says a profile is derived from the client rather than
copied from a profile set.

`HASH_NO_LEADING_ZERO` is qBittorrent's `key`. Four projects implement it:

| project | what it does | key width | can start with `0` |
| --- | --- | --- | --- |
| joal, `HashNoLeadingZeroKeyAlgorithm.java:24-33` | strips leading zeros | 1 to 8, variable | no |
| Seedr, `generators.ts:7-13` | rejects and regenerates | 8 | no |
| DOAL, `client_emulator.go:258-277` | replaces the first character | 8 | no |
| fake-torrent-client, `algorithm.rs:17-32` | skips index 0 everywhere, on a truncated alphabet | 8 | no |

libtorrent, which is what qBittorrent announces through, writes the key with
one format string. `src/http_tracker_connection.cpp:138`, tag `v2.0.11`:

```
"&key=%08X"
```

Eight upper case hex digits, zero padded. **A real qBittorrent key starts with
`0` one time in sixteen, and none of the four can ever produce one.** A tracker
that has logged fifty announces from a claimed qBittorrent and never seen a key
beginning with `0` is looking at something with a probability of about 0.04 of
being real.

Every one of the four reproduced the format faithfully. The format encodes an
algorithm named after a rule the client does not have, and each reimplementation
inherited the name rather than checking the client.

### Passes

Three. WHAT: a small Rust library exposing a client's name, peer id and key.
MECHANISM: the three generators at `src/algorithm.rs:17-70`. AGAINST
`bit-cli`: it is the fourth data point for the key finding above, and it is the
reason the acceptance for [T-234](../TODO/peers.md) is a property test rather
than a golden string. No fourth pass: the tracker is one issue and the library
has no design document.

### Verdict

**ANTI-PATTERN EXHIBIT**, kept on purpose. Three shipped defects in one small
file, none of which its tests catch, and the key defect is shared with three
larger projects.

---

## 28. `rustatio`, takitsu21/rustatio

- Upstream: <https://github.com/takitsu21/rustatio>
- Local path: `reference/rustatio`
- Commit: `04dc139d780765b9f4b5c9635389fa545087e8c3`, cloned 2026-08-24.
- Licence: **MIT** (`rustatio/LICENSE`, Copyright 2025-2026 takitsu21).
- Language: Rust. 287 stars, the largest in this group. 60 issues (21 open) and
  124 pull requests.

One file was read on the operator's instruction,
`rustatio/rustatio-core/src/torrent/client.rs`, 635 lines, plus the tracker.
It does not use joal's profile format. It carries its own model, and the
operator's warning that some of this data is wrong is correct.

### The version encoding, and what each client actually gets

`client.rs:400-407`:

```rust
fn pad_to_width_with_char(&self, width: usize, ch: char) -> String {
    if self.len() >= width { self[..width].to_string() }
    else { format!("{}{}", self, ch.to_string().repeat(width - self.len())) }
}
```

Every client but Transmission strips the dots and pads or truncates to four
(`client.rs:223` for uTorrent, `:244` qBittorrent, `:296` Deluge, `:322`
BitTorrent, `:348` rTorrent). Three consequences, each checkable
against entry 23's committed profiles:

- **A version component of ten or more produces the wrong prefix.**
  qBittorrent 3.3.13 gives `-qB3313-`; joal's committed profile and libtorrent
  both give `-qB33D0-`.
- **The fourth character is treated as padding.** uTorrent 3.5.5 gives
  `-UT3550-`; the real uTorrent profiles are `-UT3500-`, `-UT3515-`,
  `-UT353S-`, `-UT354S-`, where the fourth character carries the release stage.
- **rTorrent is announced as `-RT`.** `aquatic/crates/peer_id/src/lib.rs:103`
  maps `lt` to libtorrent-rakshasa, which is rTorrent's engine, and has no
  entry for `RT` at all. Entry 23's `rtorrent-0.9.6_0.13.6.client` uses
  `-lt0D60-`.

### Transmission, where the test asserts the defect

`client.rs:270` builds the Transmission code from the major and the minor
only: `format!("{}{}", parts[0], parts[1].pad_to_width_with_char(2, '0'))`,
then pads that to four. The patch component is never read.

So version `4.1.3` produces `-TR4100-`. Transmission's own
`update-version-h.sh` and `RatioForge/docs/client-profile-audit-2026-08-11.md`
both give `-TR4130-`, and `scripts/make-client-profile.ps1 -Client transmission
-Version 4.1.3` derives `-TR4130-` from the tag.

`client.rs:455-457` is the test:

```rust
let config = ClientConfig::get(ClientType::Transmission, Some("4.1.3".to_string()));
let peer_id = config.generate_peer_id();
assert!(peer_id.starts_with("-TR4100-"), "Peer ID should include version 4.1.3");
```

The assertion and its own message disagree. Every Transmission 4.1.x collapses
to one prefix, so the profile cannot distinguish a patch release at all.

### One suffix alphabet for six clients

`client.rs:373` draws all twelve suffix characters from
`0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz` whatever the
client is. Transmission's suffix is base 36 lower case with a checksum,
qBittorrent's is `[A-Za-z0-9_~()!.*-]`, and uTorrent's is raw bytes. A single
62 character pool matches none of the three. `client.rs:386` then generates
the key as eight upper case hex digits with leading zeros kept, which is
libtorrent's rule and not Transmission's.

### What the tracker says

Fetched 2026-08-24, cached at `.tmp/mining/takitsu21-rustatio-issues.json`.

| # | State | Title | Why it matters |
|---|---|---|---|
| [Issue 111](https://github.com/takitsu21/rustatio/issues/111) | OPEN | externalize list of clients and versions | The best design input in this group, from a user and then from the maintainer. The complaint: the version list is compiled in, the latest client version differs by platform, and a version that changes whenever the tool is rebuilt is itself the suspicious thing. The maintainer's answer is a refusal with a reason: "I'd prefer to avoid letting users manually select the exact version, since that could easily lead to misconfiguration. That's also why keeping it hardcoded is not necessarily a bad approach." The reporter then proposes an API, a hosted config file, or a plugin system. **`bit-cli` answers this without any of the three**: profiles are generated and committed the way `man/` is, and `scripts/make-client-profile.ps1` refreshes them without a release. That is written into [T-234](../TODO/peers.md). |
| [Issue 176](https://github.com/takitsu21/rustatio/issues/176) | OPEN | Using with private torrent trackers | Two users on detectability, neither with a measurement. One says a tracker can hardly detect it "if it's employed intelligently"; the other says the weak point is upload volume that outpaces the swarm. Recorded as what it is, opinion in a thread, and it is the reason `bit-cli` reports what it actually transferred. |
| [Issue 181](https://github.com/takitsu21/rustatio/issues/181) | OPEN | fails to import .torrent using announce-list instead of announce | A metainfo shape that BEP 12 has required since 2008. Corroborates section C. |

### Passes

Three. WHAT: a large ratio tool with its own client model. MECHANISM:
`client.rs` in full, cited above. AGAINST `bit-cli`: it is the strongest
argument for the acceptance condition in [T-234](../TODO/peers.md), because
every defect above would pass a review that only read the tests. The fourth
pass, reading the rest of the workspace, was not taken: the operator named one
file, and the tracker supplied the design question.

### Verdict

**ANTI-PATTERN EXHIBIT**, and the operator's warning is confirmed rather than
assumed. Three of its six clients are advertised with a prefix no release of
that client has used, and one of the three is locked in by a passing test whose
message contradicts it. [Issue 111](https://github.com/takitsu21/rustatio/issues/111)
is filed as design input under [T-234](../TODO/peers.md).

---

## 29. `RatioTracker`, 7h30th3r0n3/RatioTracker

- Upstream: <https://github.com/7h30th3r0n3/RatioTracker>
- Local path: `reference/RatioTracker`
- Commit: `45dc7d40a365921dc9d050bff06c57a16cd82ab7`, cloned 2026-08-24.
- Licence: **MIT** (`RatioTracker/LICENSE`, Copyright 2026 Flavien).
- Language: Python, one file of 1,157 lines. Zero issues and zero pull
  requests, so there is no tracker pass to take and that is said here rather
  than skipped.

**Its purpose is the opposite of ours, and the scoping matters.** It is a
tracker-side auditor: it sends dishonest announces at a tracker and reports
whether the tracker validated them. Its eight tests are a fake seed, an
inflated upload, a negative `downloaded`, a size bomb, an impossible speed,
several peer ids at once, a stop sequence, and a simulated transfer. **None of
the eight transfers to `bit-cli` and none was ported.** What `bit-cli` wants
from it is the opposite direction, announce fidelity: whether the numbers a
tracker sees are the numbers the run actually made.

### The three mechanisms that do transfer

**Percent-encode `info_hash` and `peer_id` with nothing exempt.**
`ratiotracker.py:224-225`:

```python
ih = urllib.parse.quote(info_hash_bytes, safe="")
pid = urllib.parse.quote(peer_id, safe="")
```

`safe=""` is the point. Both fields are twenty arbitrary bytes and the default
exempt set would leave some of them raw. Compare entry 25, where the exempt set
is per client and is itself a fingerprint.

**A tracker rejection has two shapes and both have to be read.**
`ratiotracker.py:375-390` `is_error_response`: a non-200 status, **or** a 200
whose bencode dictionary carries `failure reason`. A check that reads only the
status code calls a refusal a success.

**Every test registers with `started` and ends with `stopped`, so it stands
alone.** The README states it as the design rule, and it is what makes
`-t 2` and the full suite give the same answer. `scripts/check-announce.ps1`
takes that shape: one torrent, one leecher, and the whole event sequence read
back from the tracker's own record rather than from the client's log.

### One detail it gets right that four larger projects get wrong

`ratiotracker.py:233` generates the `key` as
`"".join(random.choices("0123456789ABCDEF", k=8))`: eight upper case hex
digits, and it **can** begin with `0`. That is libtorrent's rule
(`src/http_tracker_connection.cpp:138`, `key=%08X`), and entry 27 records that
all four projects whose whole purpose is client emulation cannot produce it.
The tool that is not emulating anybody has the closest key.

Its peer id at `ratiotracker.py:203-207` is the other way round: a uniform
`ascii_letters + digits` suffix for whatever prefix is given, which is the same
uniform-alphabet shape as entry 28.

### What was built from it

`scripts/check-announce.ps1` and an `--announce-log` flag on
`crates/bit-cli-core/examples/loopback-tracker.rs`. The tracker appends one
JSON object per announce carrying the **raw query string** as received, because
the parser's `BTreeMap` sorts the parameter order away and order is the thing
nothing else can recover. Six cases, loopback only: the first event is
`started` and carries the whole payload as `left`; `completed` is sent exactly
once and `left` is zero by then; `stopped` is sent; `left` never rises; the
last announce's `downloaded` covers the payload and does not exceed what the
run reports; and the gap between ordinary announces is at least the tracker's
`min interval`. [T-235](../TODO/trackers.md) is the entry.

**All six hold.** What the run found instead is in the identity `bit-cli`
presents, which the check prints beside its verdict: the announce carries the
peer id prefix `-rQ9010-`, which is the vendored `librqbit`'s own, and
`bit-cli trackers` uses `-BC0100-`, which libtorrent's
`src/identify_client.cpp:161` maps to BitComet. That is
[T-236](../TODO/peers.md), filed from this run.

### Passes

Three. WHAT: a tracker-side auditor with eight adversarial tests. MECHANISM:
`do_announce` and `is_error_response`, cited above. AGAINST `bit-cli`: the
announce mechanics and the self-contained event sequence transfer, the eight
tests do not, and the scoping is the entry's own conclusion rather than the
tool's. No fourth pass and no tracker pass: the repository has neither issues
nor pull requests.

### Verdict

**ADOPT**, narrowly, and the narrowing is the finding. Three mechanisms and one
harness shape. Nothing that sends a number the run did not make.

---

## 33. `ed2k-server`, andrey23127/ed2k-server

- Upstream: <https://github.com/andrey23127/ed2k-server>
- Local path: `reference/ed2k-server`
- Commit: `3924f0ab6762dd09ed88ceb646987af4c2fbfd48`, cloned 2026-08-24.
- Licence: **MIT** (`ed2k-server/LICENSE`, Copyright 2026 emule-security.org).
- Language: Rust. An eDonkey2000 index server, a clean-room replacement for
  Lugdunum. Two open issues, neither about traversal.

Not a BitTorrent tree, and it is here for one reason: eDonkey solved the
"peer behind a NAT cannot be dialled" problem in **2002**, in production, and
the answer is server-coordinated rather than swarm-coordinated. Reading it is
how to say what BEP 55 is and is not.

### Two mechanisms, one stock and one an extension

**Stock: the callback.** `ed2k-server/src/proto/opcodes.rs:21`
`OP_CALLBACKREQUEST` and `:51` `OP_CALLBACKREQUESTED`. A client that cannot be
dialled has a "LowID"; a peer that wants it asks the server, and the server
tells the LowID client to dial out. That works because the LowID client already
holds an open TCP connection to the server. It is BEP 55's rendezvous with a
central server in place of a peer, and it is strictly weaker: it needs **one**
of the two ends to be reachable.

**The extension: LowID to LowID.** `:25-45` documents three opcodes the stock
protocol does not have, `OP_LOWID_HOLEPUNCH_REQUEST` `0x60`,
`OP_LOWID_HOLEPUNCH_INFO` `0x61` and `OP_LOWID_HOLEPUNCH_FAIL` `0x62`, which
coordinate a UDP hole punch when **both** ends are unreachable. Three details
worth taking:

- The server "only exchanges small address packets, it never relays file data"
  (`:28-29`). That is the same separation `dig-nat` ranks fifth against sixth
  and the one a relay design has to state.
- `OP_LOWID_HOLEPUNCH_INFO` carries a **role byte**: `0 = you initiate,
  1 = you wait` (`:38`). BEP 55 has no role and both sides dial, which
  `torrent/NOTES.md:15-31` in entry 1 says is the reason a `connect` cannot be
  attributed to a rendezvous you sent.
- The failure reasons are enumerated (`:42-43`): target not connected, target
  is HighID so no punch is needed, requester not logged in. A traversal that
  fails with a reason is the property `dig-nat` has and BEP 55 does not.

### Passes

Three, and the fourth is not available. WHAT: an eD2k index server. MECHANISM:
the callback and the LowID holepunch opcodes, cited above. AGAINST `bit-cli`:
it supplies the role byte and the enumerated failure, both of which are
[T-102](../TODO/bep-coverage.md)'s to consider and neither of which changes
BEP 55's wire format. The tracker is two open issues, read, and neither is about
traversal.

### Verdict

**FILED ELSEWHERE.** The role byte and the failure enumeration are written into
[T-238](../TODO/peers.md) as design input. Nothing here is adopted directly:
eD2k is a different protocol with a central server BitTorrent does not have.

---

## 34. `Hollow`, Gaok1/Hollow

- Upstream: <https://github.com/Gaok1/Hollow>
- Local path: `reference/Hollow`
- Commit: `0d93e5c057c6e1eba93686dccb21e4f833a1b765`, cloned 2026-08-24.
- Licence: **MIT**, declared in `Hollow/Cargo.toml:13` and
  `Hollow/README.md:193`. No licence file.
- Language: Rust, four crates. Zero issues and zero pull requests.

### The README and the code disagree about the transport

`Hollow/README.md:15` says Steam's peer to peer transport "already solves NAT
traversal and already authenticates both ends" and carries everything.

`Hollow/crates/p2p-connection/Cargo.toml` depends on `quinn`, `rcgen` and
`ring`, and the crate holds its own `stun.rs` of 768 lines. There is no Steam
networking dependency in it. Steam supplies the identity and the friends list;
the bytes go over QUIC with STUN-discovered addresses.

Read the doc, then the code, then cite the code. That is section 7's rule and
this is the third tree this session where it paid.

### What its STUN does

`Hollow/crates/p2p-connection/src/stun.rs:58` `detect_public_endpoint`, `:84`
`detect_public_endpoint_on_socket`, `:91` a traced variant, and `:337`
`stun_server_list`. Reflexive address discovery, on the socket the caller is
going to use, which is the part that matters. **No NAT classification**, the
same absence as entry 30.

### Passes

Two, and the third is not worth claiming. WHAT: a Steam-identity chat with a
QUIC data path. MECHANISM: the STUN and QUIC crate above. There is no third
pass against `bit-cli`'s hard problem: this tree has nothing to say about
swarms, pieces or trackers, and claiming a fourth pass over it would be worse
than admitting two.

### Verdict

**REFUSED**, with the reason recorded so it is not re-derived: the traversal is
STUN plus QUIC with an out-of-band identity, which entry 30 covers in more
depth and with tests, and the Steam dependency makes the interesting half
unavailable to anything that is not a Steam client.

---

## 35. `NetDrop`, NETFORY/NetDrop

- Upstream: <https://github.com/NETFORY/NetDrop>
- Local path: `reference/NetDrop`
- Commit: `534d319e06b47164d0268ced4b847f5a665f1f31`, cloned 2026-08-24.
- Licence: **the repository disagrees with itself.** `NetDrop/LICENSE` is
  GPL-3.0; `NetDrop/Cargo.toml:9` declares `license = "MIT"` for the whole
  workspace. `NetDrop/Cargo.toml:10` names the repository as
  `smartholdem/netdrop`, which is neither the owner the operator supplied nor
  the one this clone came from.
- Language: Rust. Zero issues and zero pull requests.

**The part this was read for is not in the repository.**
`NetDrop/Cargo.toml:3` declares three workspace members, `netdrop-core`,
`netdrop-cli` and `netdrop-ui/src-tauri`. Only `netdrop-cli` is present, and it
depends on `netdrop-core` by a relative path that does not exist. The workspace
cannot build.

The README's claim is `iroh (QUIC, hole punching, relay fallback)`. Every iroh
reference in the tree is in prose: `README.md:2`, the CLI crate's `description`
field, and the two user guides. **There is no iroh dependency and no traversal
code in what was published.**

### Passes

One, and the reason there is no second is the finding. WHAT: a CLI wrapper
around a core crate that is not in the repository. There is no mechanism pass
to take, because the mechanism is absent, and saying so is the entry.

### Verdict

**REFUSED**, and recorded rather than dropped, because "the traversal code is
not in this repository" is the kind of thing a later session would otherwise
spend an hour rediscovering. The licence conflict is the second reason: a tree
whose `LICENSE` says GPL-3.0 and whose manifest says MIT is one to read and
never to copy from.

---

## 36. `iroh-experiments`, n0-computer/iroh-experiments

- Upstream: <https://github.com/n0-computer/iroh-experiments>
- Local path: `reference/iroh-experiments`
- Commit: `b66d8b861235d7b5123c62491cb9fafab950c0f4`, cloned 2026-08-24.
- Licence: **`Apache-2.0 OR MIT`** (`iroh-experiments/LICENSE-APACHE` and
  `LICENSE-MIT`; `h3-iroh/README.md:11-13` restates it).
- Language: Rust. Zero issues and zero pull requests.

The operator named `h3-iroh`. It is 5 files: `src/lib.rs`, `src/axum.rs`, and
three examples. What it does is send HTTP/3 over an iroh connection through the
`h3` crate.

**It is the shortest possible statement of what iroh is**: a QUIC connection
you get by node id instead of by address, on which any QUIC-framed protocol
runs unchanged. That is genuinely useful to know for
[T-238](../TODO/peers.md), because it says the cost of adopting iroh is not in
the protocol above it. The cost is entirely in the addressing.

The sibling directories are `content-discovery`, `iroh-dag-sync`,
`iroh-pkarr-naming-system` and `iroh-s3-bao-store`. None was read: the operator
named one and the others are content addressing rather than traversal.

### Passes

Two, stated as two. WHAT: HTTP/3 over iroh. MECHANISM: `h3-iroh/src/lib.rs`
adapting an iroh `Connection` to `h3`'s transport traits. No third pass: five
files with no traversal logic in them cannot answer a traversal question.

### Verdict

**CONFIRMS** entry 32's reading of the iroh integration surface, from a second
tree written by iroh's own authors. Nothing new is filed from it.

---

## 37. `gaia`, vaz3r/gaia

- Upstream: <https://github.com/vaz3r/gaia>
- Local path: `reference/gaia`
- Commit: `30fc0403b92cf599f910bded4f7a2771fd5909a0`, cloned 2026-08-24.
- Licence: **none.** There is no licence file, no licence key in any manifest,
  and no licence statement in any document. The only `license: MIT` strings in
  the tree are frontmatter in six third-party tool definitions under
  `.opencode/skills/`, which describe those tools and not this repository.
  Read only, and nothing is taken.
- Language: Rust and Python. Zero issues and zero pull requests.

The operator named one file and predicted what it would be. The prediction
holds: **`gaia/docs/future_plan_peer_quality.md` is a design document with no
implementation behind it.** It is 80 lines, it opens by answering a question
that is not written down, and grepping the whole tree for its own vocabulary
(`rtt_ema`, `last_useful_response`, "negative cache", "reputation") finds
matches in that file and in one other document, and in no source file.

### The model, which is worth having anyway

Three mechanisms, deliberately separated by lifetime:

1. **Long-term DHT node reputation.** Per node: `last_response_time`,
   `rtt_ema`, `query_count`, `fail_count`, and `last_useful_response`, which
   distinguishes a node that answered from one that answered with something.
   Selection combines XOR distance with reputation rather than replacing one by
   the other, and the fields persist in the routing table snapshot that already
   exists.
2. **Short-term negative swarm-peer cache.** On a connect or fetch timeout,
   mark `IP:port` bad for 10 to 30 minutes, bounded and aggressively expired.
3. **Per-infohash positive history**, and explicitly **not** a global
   cross-torrent positive cache. The argument against the global version is the
   good part: a peer that had torrent A need not have torrent B, residential
   addresses and NAT mappings churn, and the only peers stable across torrents
   are seedboxes, so relying on them concentrates load on a few addresses and
   invites being blocked by them.

It also names the measurements to take before and after: source response rate,
peers per source response, timeout ratio, verified per hour, over a 24 to 48
hour soak.

### Where it lands for `bit-cli`

Mechanism 2 is close to something this repository already has:
`--web-seed-cooldown` and `--web-seed-max-errors` do exactly this for HTTP
sources ([T-130](../TODO/multi-source.md), [T-137](../TODO/multi-source.md)),
and [T-164](../TODO/peers.md) is the peer-side equivalent, partial and blocked
on `Session::blocklist` being immutable. So the document is independent
corroboration for work already filed rather than new work.

Mechanism 1 is new and is filed as [T-240](../TODO/dht.md).

### Passes

Two, and the second is the finding. WHAT: a plan for peer reputation. AGAINST
`bit-cli`: two of its three mechanisms are already filed here and the third is
not. There is no mechanism pass, because there is no mechanism: the document
describes code that does not exist in the repository that carries it, and that
is stated here rather than implied.

### Verdict

**CONFIRMS** for mechanisms 2 and 3, which corroborate
[T-164](../TODO/peers.md) and the web seed cooldown work. **ADOPT** for
mechanism 1, filed as [T-240](../TODO/dht.md). Its status as a plan rather than
an implementation is part of the verdict, not a footnote to it.

---

## 39. `dht-crawler`, 0xddy/dht-crawler

- Upstream: <https://github.com/0xddy/dht-crawler>
- Local path: `reference/dht-crawler`
- Commit: `571472d16f9b4b71fecdf5ba47e90efb74ff9e99`, cloned 2026-08-24.
- Licence: **MIT** (`dht-crawler/LICENSE`, Copyright 2024 dht-crawler
  contributors).
- Language: Rust, 7,998 lines across 20 files. Documentation is in Chinese.
  Zero issues and zero pull requests.

A BEP 5 DHT crawler as a **library**: it joins the DHT, receives
`announce_peer`, and fetches metadata over BEP 9. Compare entry 21,
`dht-spider`, which is the same job in about 1,500 lines; this one is five
times the size and the difference is all in the parts that bound it.

### What the extra six thousand lines are

The file list is the finding. Beyond the protocol (`krpc.rs`, `protocol.rs`,
`node_id.rs`, `addr.rs`) there is `budget.rs`, `scheduler.rs`, `node_pool.rs`,
`udp_buffer.rs`, `udp_ingress.rs`, `runtime_stats.rs`, `routing_snapshot.rs`
and `crawl_config.rs`. A crawler that runs for a day is mostly admission
control, and the protocol is the small half.

`src/metadata.rs:274` and `:394` record a metadata fetch failure with the
reason `size_limit` as its own counter. That is the same bound entry 38 states
as a constant and [T-212](../TODO/memory.md) is open on, seen from the
operations side: the cap is not only enforced, its rejections are counted, so a
run can say how often a peer asked for more than it was allowed.

### Where it lands for `bit-cli`

Not much, and saying so is the entry. `bit-cli` is not a crawler and decision
7.4 keeps it daemonless, so a component that exists to run for days has no home
here. Two things do transfer and both are small:

- **Count the refusals, not just the successes.** A bound that has never been
  hit and a bound that is hit constantly look identical without a counter.
  That is folded into [T-212](../TODO/memory.md)'s acceptance rather than
  filed separately.
- **The routing table snapshot is the right place for per-node state**, which
  is the same conclusion entry 37's plan reaches from the other direction and
  which [T-240](../TODO/dht.md) records.

### Passes

Three. WHAT: a DHT crawler library. MECHANISM: the module split above, read as
a list rather than line by line, which is honest about the depth reached.
AGAINST `bit-cli`: mostly negative, because a crawler's shape is a daemon's
shape and 7.4 rules that out. No fourth pass and no tracker pass: the
repository has neither issues nor pull requests, and its documentation is in a
language this pass did not read closely, which bounds what is claimed here.

### Verdict

**FILED ELSEWHERE.** The counter goes into [T-212](../TODO/memory.md) and the
snapshot point into [T-240](../TODO/dht.md). Nothing is adopted from it
directly.

---

## 40. `TheDancingDeveloper-org`, the whole organisation

- Upstream: <https://github.com/orgs/TheDancingDeveloper-org/repositories>
- Local path: none. Nothing was cloned. Everything below was read through
  `gh api`, and the evidence is cached at `.tmp/mining/tdd-org.tsv` and
  `.tmp/mining/tdd-licence-evidence.txt`.
- Enumerated 2026-08-24: **33 repositories.**

The operator's instruction was that all of them are permissively licensed, and
that the licence be verified from each repository itself rather than from
GitHub's metadata. **The claim does not hold.** Four are copyleft, one is a
relicensing that cannot be right, and one has no licence statement of any kind.

### The licence evidence, per repository

Read from the repository: a licence file's first lines where one exists, the
`license` key in `Cargo.toml` where there is one, and the README's licence
section. The **source** of each determination is named, and a declaration is
distinguished from an inference and from an absence.

| repository | determined | from | note |
| --- | --- | --- | --- |
| Rust-PAR2 | MIT OR Apache-2.0 | `LICENSE-MIT`, `LICENSE-APACHE`, `Cargo.toml`, README | complete and consistent |
| rust-yenc-simd | MIT | `LICENSE`, `Cargo.toml` | consistent |
| nntp-client-bench | MIT | `LICENSE`, README | consistent |
| cadastre | MIT | `LICENSE`, README | consistent |
| rustnzb | MIT | `Cargo.toml`, README badge | **no licence file** |
| nzb-core, nzb-decode, nzb-dispatch, nzb-news, nzb-nntp, nzb-postproc, nzb-web, nzbdav-rs | MIT | `Cargo.toml` only | **no licence file in any of the eight** |
| librtbit and its eleven sibling crates | declared MIT | `Cargo.toml` only | **see below. The declaration cannot be right.** |
| rustTorrent | Apache-2.0 at the root, MIT in the crates | `LICENSE` is Apache-2.0 with Igor Katson's copyright; README credits him; `crates/librtbit*/Cargo.toml` say MIT | **the repository contradicts itself** |
| NGMS | **GPL-3.0-only** | `LICENSE`, `Cargo.toml`, README | copyleft |
| indexarr-rs | **AGPL-3.0-only** | `LICENSE`, `Cargo.toml`, README | copyleft |
| egressy | **AGPL-3.0-only** | `LICENSE`, `Cargo.toml` | copyleft |
| komodo | **GPL-3.0-or-later** | `LICENSE`, `Cargo.toml` | copyleft |
| github-policy | **none found** | no licence file, no manifest, no README statement | |
| transl | **none found** | no licence file, no statement in the Gradle build or the README | |
| agent-harness | **none found** | no licence file, no key in `pyproject.toml`, no README statement | |

Nine of the thirty-three carry no licence file at all, and for eight of those
the only statement is a `Cargo.toml` key. A manifest key is a real declaration
and this corpus already keeps two trees on that basis alone (section G,
`mtorrent` and `nanotorrent`). It is weaker than a file and it is recorded as
what it is.

### The `librtbit` family, and why its licence cannot be right

Twelve repositories named `librtbit`, `librtbit-core`, `librtbit-bencode`,
`librtbit-buffers`, `librtbit-clone-to-owned`, `librtbit-dht`, `librtbit-lsd`,
`librtbit-peer-protocol`, `librtbit-sha1-wrapper`, `librtbit-tracker-comms`,
`librtbit-upnp` and `librtbit-upnp-serve`. Each declares `license = "MIT"` in
its `Cargo.toml` and none carries a licence file.

**They are `librqbit` renamed.** Three checks, in increasing strength:

1. The crate list matches `librqbit`'s exactly, name for name, against
   `vendor/rqbit/crates/` in this tree.
2. `librtbit/src/` is `librqbit/src/` file for file: `api.rs`, `bitv.rs`,
   `chunk_tracker.rs`, `create_torrent_file.rs`, `peer_connection.rs`,
   `piece_tracker.rs`, `torrent_state/`, and the rest, plus `category.rs` and
   `rss/`. `librtbit/Cargo.toml`'s feature list carries `timed_existence`,
   `storage_middleware`, `_disable_disk_write_net_benchmark` and
   `_disable_reconnect_test`, which are `librqbit`'s.
3. Fetched and compared: `librtbit/src/peer_connection.rs` is 644 lines against
   this tree's 662, and the diff is `librqbit_core` renamed to `librtbit_core`
   plus the additions **this repository** made under `patches/`
   (`stream_transform::OutgoingTransform`, `HaveShortcut`). Their copy is
   closer to upstream than ours is.

`librqbit` is **Apache-2.0**, copyright Igor Katson. An Apache-2.0 work cannot
be redistributed under MIT: section 4 of Apache-2.0 requires the licence, the
notices and a statement of changes to travel with it. `rustTorrent` handles the
root correctly, keeping the Apache-2.0 text with Igor Katson's copyright and
crediting him in its README, and then declares MIT in the crate manifests of
the same code.

**For `bit-cli` this is a read-only question and the answer is simple**: take
nothing from any of the twelve. They are a second copy of the tree this
repository already vendors, so there is nothing in them to want that
`vendor/rqbit/` does not already have, and the licence declaration is one that
cannot be relied on. Recorded here so a later session does not read `MIT` in a
manifest and act on it.

### Triage: what was judged relevant, and what was skipped and why

**Relevant, and read:**

- `rustTorrent` and the `librtbit` family, above. Read for the licence
  question, and read again for entry 41: it is `rqbit` plus a React web UI plus
  a qBittorrent-compatible API, which is the worked example of what serving a
  UI costs on the base `bit-cli` builds on.

**Skipped, each with the reason**, because a repository that simply does not
appear is a hole:

- `indexarr-rs`, decentralised torrent indexing, and the most tempting of the
  skips. **AGPL-3.0-only.** `deny.toml` refuses copyleft and
  `scripts/check-licence-gate.ps1` proves the refusal against a probe crate.
  Reading it is permitted and taking anything from it is not, and an indexer is
  not on any `TODO/` list, so the value of reading it is not worth the risk of
  a later session forgetting where an idea came from.
- `NGMS`, GPL-3.0, an embedded BitTorrent and Usenet engine. Same reason. Its
  `bep-uplift.md` would be interesting and the licence makes it a hazard.
- `egressy` AGPL-3.0 and `komodo` GPL-3.0: a Docker VPN gateway and a
  deployment tool. Neither is BitTorrent.
- `rustnzb`, `nzb-core`, `nzb-decode`, `nzb-dispatch`, `nzb-news`, `nzb-nntp`,
  `nzb-postproc`, `nzb-web`, `nzbdav-rs`, `nzb`-adjacent, nine repositories:
  Usenet and NNTP, a different protocol with no piece exchange, no swarm and no
  tracker. `nzb-dispatch`'s per-server worker pool with priority gating and
  retry has a shape in common with a peer scheduler, which is the one thing
  worth returning for if a scheduler entry is ever opened.
- `Rust-PAR2` and `rust-yenc-simd`: forward error correction and yEnc, both
  Usenet. `Rust-PAR2` is the only permissively licensed erasure coding tree in
  the org and BitTorrent has no use for one.
- `nntp-client-bench`, `cadastre`, `agent-harness`, `github-policy`, `transl`:
  a benchmark harness for NNTP, an infrastructure catalogue, an agent harness,
  CI policy, and an Android translator. None touches this work.

### Passes

Three, over the organisation rather than over one tree. WHAT: 33 repositories,
enumerated and listed. MECHANISM: the licence evidence per repository, from the
repository. THE HARD PART: whether the operator's claim holds, which is the
question that was asked and the answer is no. There is no fourth pass and no
tracker pass: no repository here was cloned, and reading 33 trackers to triage
33 repositories is not proportionate to what the triage found.

### Verdict

**REFUSED**, for the organisation as a source, with the reasons above so no
later session re-derives them. The one thing carried forward is entry 41's use
of `rustTorrent` as evidence, and that is a read of a README and a crate list.

**A question for the operator**, recorded in `PROGRESS.md` rather than acted
on: the `librtbit` crates relicense an Apache-2.0 work as MIT. This session
does not contact anybody about it, by [RULES.md](../TODO/RULES.md) section 6a,
which is absolute.

---

## 41. A 2026 Survey of Rust GUI Libraries, wybxc

- Upstream: <https://blog.wybxc.cc/blog/rust-gui-survey-2026/>, dated
  2026-08-22.
- Local path: none. Fetched and read, not cloned, on the operator's
  instruction. Nothing from it is in `reference/`.
- Licence: not stated on the page. Quoted here in short passages with
  attribution, which is what section F's `aria2-next` row already establishes
  for a document.

Fifty-four libraries, each given the same task: a text box that renders a QR
code from what is typed, judged on usability, accessibility and input method
support. The author is on macOS and says so.

### The result

**The winners are `slint` and `egui`**, and the author's reason is not
aesthetics: both have "solid support for IME and accessibility", which is where
most of the fifty-four fail. They are named as the retained-mode and
immediate-mode choices respectively.

**With a WebView allowed**, `Dioxus` or `Tauri` with `tauri-spectra`.

**Just short**, on input method or accessibility gaps: `cushy`, `Freya`,
`Floem`, `Iced`, `Relm4`, `Xilem`.

The conclusion carries the sentence that matters most for a decision here:

> Rust has plenty of promising GUI projects, but the ecosystem has not yet
> converged on a universally accepted, boring choice.

The table backs it. `iced`, `floem`, `cushy`, `GTK 3`, `GTK 4` and `Freya` are
all marked no for accessibility. `egui` needs CJK font setup for input methods.
`GPUI` crashes on input method use and has no text input widget.

### What it costs to have an opinion at all

The article ends with the disk usage of the survey: **94 GB across fifty-four
projects**, from 213 MB for `windows-rs` to 6.1 GB for `ribir`, with `tauri` at
4.6 GB and `slint` at 4.2 GB. That is not a criticism of any of them and it is
a real number for a repository whose CI builds three targets on every push.

### Against `bit-cli`, and the distinction the brief did not draw

The operator's question mixes two things that have different costs, and
separating them is most of the answer.

**A native GUI does not collide with decision 7.4 at all.** A separate binary,
`egui` or `slint`, linking `bit-cli-core` and driving the same session the CLI
drives. No daemon, no RPC, no SQLite, no state file, and `bit-cli` keeps
working with no config. Everything 7.4 forbids is absent, because there is no
server: the UI **is** the process.

**A browser UI reverses 7.4 by construction.** For a page to reach `bit-cli`,
`bit-cli` has to listen, which is [T-200](../TODO/phase-c.md) the daemon and
[T-201](../TODO/phase-c.md) the RPC, and a UI that shows a torrent list across
invocations needs [T-203](../TODO/phase-c.md) session save and restore. That is
three deferred entries un-deferred, and section 6 says do not un-defer them.

**The worked example is in this corpus and it is on the same base.**
`rustTorrent` (entry 40) is `rqbit` with a React and TypeScript web UI, a full
HTTP API, a qBittorrent-compatible endpoint set for the `*arr` tools, RSS
automation, a Docker image and a Tauri desktop wrapper. It is what the browser
answer looks like when somebody finishes it, built on the engine `bit-cli`
vendors. What it also is: a daemon, an HTTP server, an API surface to keep
compatible, a Node build toolchain in the release path, and a second binary.
Reading it is how to price the option rather than argue about it.

### Passes

Two, and there is no third. WHAT: fifty-four libraries surveyed against one
task, with a table. AGAINST `bit-cli`: the separation above, and the finding
that the native answer is cheap and the browser answer is a reversal. There is
no mechanism pass: it is a survey and there is no construction in it to cite at
a line. No tracker: it is a blog post.

### Verdict

**FILED ELSEWHERE.** [T-243](../TODO/phase-c.md) is the draft, marked as
needing an operator ruling, and it does not un-defer Phase C. The
recommendation and the runner-up are argued there rather than here.

---

# Cross-cutting findings

## A. BEP 6 test vector (use this)

From anacrolix PR 1052, and reproducible against `vortex`'s
`generate_fast_set` and `aria2_rust`'s `compute_fast_set`:

```
ip        = 80.4.4.200
info_hash = AA AA … AA  (20 bytes)
numPieces = 1313
k         = 7
=> [1059, 431, 808, 1217, 287, 376, 1188]
```

Masking is `/24` per BEP 6. `aria2_rust` deliberately uses a class-based mask
instead; a `bit-cli` conformance test should assert the vector above and, if it
ever compares against an aria2 peer, expect a mismatch.

## B. Five piece-length algorithms, five answers

| Source | Rule | Bounds |
|---|---|---|
| `intermodal/src/piece_length_picker.rs:10` | `2^(ceil(log2(size))/2 + 4)` | 16 KiB … 16 MiB |
| `torrent/metainfo/piece-length.go:61` | start 16 KiB, double while pieces ≥ 2048 | soft count 1024–2048, optional hard size bounds |
| `mkbrr/internal/trackers/trackers.go:319` | 14-band table by total size, then per-tracker override | 2^16 … 2^24 auto, 2^27 manual |
| `nanotorrent/src/bittorrent/torrent_create.rs:381` | start 256 KiB, double while `total/pl > 2000` | ≤ 16 MiB, must be a power of two |
| `create-torrent/index.js:308` | `piece-length` package, capped | ≤ 4 MiB by default |

For v2 or hybrid the choice is constrained: a power of two and at least 16 KiB
(`nanotorrent/src/bittorrent/torrent_create.rs:390`,
`rustorrent/src/torrent.rs:300`).

## C. Metainfo shapes a parser must survive

Each verified in a local file or a fetched issue:

1. `url-list` as a bencoded **string** rather than a list —
   `torrent/metainfo/testdata/flat-url-list.torrent`, handled at
   `torrent/metainfo/urllist.go:11`, `TorrentNG/crates/rt-metainfo/src/parse.rs:368`,
   `gosh-dl/src/torrent/metainfo.rs:391`.
2. `url-list` as an **empty string** — `parse-torrent/index.js:166`.
3. `name.utf-8` / `path.utf-8` / `comment.utf-8` alongside the plain keys, with
   different encodings — intermodal Issue 534, parse-torrent Issue 177,
   handled at `parse-torrent/index.js:124` and `:140`.
4. **Unsorted bencode keys** from uTorrent — intermodal Issue 454.
5. **Trailing bytes** after the top-level dict — anacrolix Issue 992; tolerated
   for whitespace/NUL at `mkbrr/torrent/update.go:210`.
6. **Zero-length path components** (`path: ["", "foo"]`) — parse-torrent Issue 89.
7. A `pieces root` that is **not 32 bytes** — anacrolix PR 1056.
8. A `piece layers` entry for a file that does not need one —
   `rustorrent/src/torrent.rs:565`.
9. A **piece length that is not a multiple of 16 KiB** in v1 — vortex PR 124.
10. **More than 65535 pieces** (µTorrent refuses to open) — intermodal Issue 499.
11. `private` as an integer inside `info`, never a top-level boolean —
    `create-torrent/index.js:295`; the wrong way at `parse-torrent/index.js:221`.

## D. Where `bit-cli`'s open entries are answered

Rebuilt 2026-08-24 against the entry list as it stands, rather than edited.
Half of the previous version's rows pointed at closed work.

**One row per open or partial entry that the corpus has something to say
about.** An entry that is not here has no source in these thirty-nine trees,
and that is worth knowing too: it means the work is this repository's own.

| entry | best source |
|---|---|
| [T-033](../TODO/performance.md), three aria2 flag names | `aria2_rust/aria2-protocol/src/bittorrent/piece/picker.rs:9-22` names six strategies and a three-value priority mode. It is the vocabulary, not the answer: the entry is open on which three names to use. |
| [T-034](../TODO/performance.md), endgame observable | `vortex/bittorrent/src/piece_selector.rs:91`, an `&mut bool` out-parameter threaded out of the selector, which is the minimal plumbing. `gosh-dl` PR 7 is the double-count bug that observability exists to catch. |
| [T-052](../TODO/dht.md), DHT not reported | `n0-mainline`'s adaptive mode: client for fifteen minutes, then server only if publicly reachable. A short-lived CLI invocation should almost certainly never become a server, and that is what the report has to be able to say. |
| [T-081](../TODO/create-seed.md), BEP 52 create | `nanotorrent/src/bittorrent/torrent_create.rs`, v2 **and** hybrid with BEP 47 padding, built on `librqbit`. `torrent/merkle/` for the primitives, `rustorrent/src/torrent.rs:542,581` for the validation, `superseedr/integration_tests/torrents/` for sixteen real fixtures. |
| [T-082](../TODO/create-seed.md), BEP 16 superseed | `rustorrent/src/main.rs:10577,11050,12588`, the only implementation in the corpus and a simplified one. What it does not do is listed in entry 10 and is most of a full BEP 16. |
| [T-083](../TODO/create-seed.md), choke and disconnect reporting | `vortex/bittorrent/src/torrent.rs:488,594`, the full leech and seed unchoke algorithms with per-round counters, which is the state a report would carry. |
| [T-101](../TODO/bep-coverage.md), uTP | `TorrentNG/crates/rt-utp/` for LEDBAT and selective ack with tests, `mtorrent/mtorrent-core/src/utp/retransmitter.rs` for the RTT and RTO update, `superseedr/src/networking/utp.rs` for the tuning constants. anacrolix Issue 1013 is the maintainer's argument that a hand-rolled uTP is a real maintenance cost. **What is left of this entry is a latency measurement, and none of these produce one.** |
| [T-102](../TODO/bep-coverage.md), BEP 55 | `fx-torrent/src/peer/extension/holepunch.rs`, a working 678 line implementation, and `torrent/NOTES.md:15-31` for why a rendezvous is only sent through a relay for the same torrent. `ed2k-server/src/proto/opcodes.rs:25-45` adds two things BEP 55 lacks: an initiator role byte and an enumerated failure. |
| [T-111](../TODO/cli-surface.md), events derived from polling | nothing in the corpus. Every tree here polls or is a library whose caller polls. |
| [T-114](../TODO/cli-surface.md), batch input | `mkbrr/torrent/batch.go` with `examples/batch.yaml` and `schema/batch.json`, and `intermodal`'s `--input`. |
| [T-134](../TODO/multi-source.md), v1 and v2 identity | `torrent/types/infohash-v2/infohash-v2.go:60` `ToShort`, which truncates the 32 byte v2 hash to 20 for DHT and tracker use, and `superseedr/agentic_plans/v2_identity_lossiness_review_2026-04-14.md` for why one hash field is the wrong model. |
| [T-135](../TODO/multi-source.md), steer sources at run time | `torrent/requesting.go:191-196` inverts the peer ordering while web seeds are active so the two ends converge, and `torrent/internal/request-strategy/NOTES.md:19-27` is the full candidate sort with a 64 MiB unverified-bytes stop at `:14`. |
| [T-153](../TODO/cli-surface.md), link speeds on macOS | nothing in the corpus. |
| [T-164](../TODO/peers.md), bad peers keep a slot | `vortex` Issue 125 is the same problem from the other side: after the crash was fixed the peer reconnects and keeps sending garbage, and the DHT rediscovers it every twenty seconds. `aria2_rust`'s `bt_peer_storage/rejection_state.rs` is the state to keep. |
| [T-168](../TODO/bep-coverage.md), WebTorrent | `torrent/webtorrent/` for the client, `aquatic/crates/ws_protocol/` for the tracker side including what the reference client actually sends, and `torrust-actix/RtcTorrent.md` for the whole protocol written out with its own known pitfalls. |
| [T-169](../TODO/dht.md), BEP 33 and BEP 51 | `fx-torrent/src/bloom_filter.rs` for the BEP 33 scrape filter and `src/dht/krpc.rs` for `sample_infohashes`. |
| [T-170](../TODO/dht.md), BEP 44 | `n0-mainline/src/common/mutable.rs:32,46,145`, and `n0-mainline/beps/bep_0044.rst` is the normative text. PR 9 there is a security fix worth reading before implementing `put`. |
| [T-175](../TODO/create-seed.md), NFD normalisation | `mkbrr/torrent/normalize.go:18,58,80`, and Issue 182 is the best documented cross-platform torrent creation bug in the corpus. |
| [T-192](../TODO/disk-io.md), what the write buffer is worth | `TorrentNG/crates/rt-storage/src/elevator.rs:223,251`, coalescing with tests that state reads are offset-sorted and coalesced per file while writes are ordered and not coalesced. |
| [T-212](../TODO/memory.md), magnet metadata allocation | `demagnetize-rs/src/consts.rs:15` caps the declared length at 20 MiB and `dht-spider/src/wire.rs` at about 16 MiB. `dht-crawler/src/metadata.rs:274,394` counts the refusals, which is the half that says whether a bound is doing anything. |
| [T-224](../TODO/memory.md), the soak RSS slope | nothing in the corpus. It is a measurement of this tree. |
| [T-227](../TODO/memory.md), the window cache total | `seedchamp/docs/design.md:199`, a per-torrent pool bounded in bytes with a per-peer fraction, which is the part `bit-cli` does not have. |
| [T-232](../TODO/memory.md), a pass on a dead workload | nothing in the corpus. |
| [T-233](../TODO/peers.md), MSE over uTP | `mtorrent/mtorrent-core/src/pe/` is the cleanest standalone MSE, and `mtorrent/mtorrent-core/src/pe/utils.rs:17` is the plaintext-or-encrypted detection on one port. Neither runs MSE over uTP, so this stays this repository's own. |
| [T-234](../TODO/peers.md), client identity | entries 23 to 29 in full. The profile model from `joal`, the peer wire fields from `DOAL/peerwire/server.go:337` and `extensions.go:29`, the verification rule from `RatioForge/docs/client-profile-audit-2026-08-11.md:22`, and three anti-patterns. |
| [T-236](../TODO/peers.md), two peer ids | libtorrent `src/identify_client.cpp:161` is the registered prefix table and `aquatic/crates/peer_id/src/lib.rs:100-120` is a second implementation of it. |
| [T-237](../TODO/trackers.md), uncovered announce paths | `RatioTracker/ratiotracker.py:375-390` for the two shapes of rejection. |
| [T-238](../TODO/peers.md), traversal beyond the BEPs | `dig-nat/src/strategy.rs:69-71` for the ranked ladder, `tcp-transfer-ice/README.md:27-38` for symmetric NAT port prediction, `ed2k-server` for the server-coordinated shape, `iroh-fm` for what adopting iroh looks like. |
| [T-239](../TODO/peers.md), network shape and path label | `iroh-fm/crates/server/src/iroh_rpc.rs:411-437` for the selected-path label. **Nothing in the corpus classifies a NAT**, which is the entry's own premise. |
| [T-240](../TODO/dht.md), DHT node reputation | `gaia/docs/future_plan_peer_quality.md`, a design with no implementation behind it, and `dht-crawler`'s `routing_snapshot.rs` for where the state belongs. |
| [T-241](../TODO/metainfo.md), magnet to `.torrent` | `demagnetize-rs`, whose whole purpose this is. |
| [T-242](../TODO/performance.md), the request depth | `seedchamp/docs/design.md:197`, a depth derived from an EMA of the peer's own rate. Unmeasured there, which is what makes it a design to test. |
| BEP 47 padding, no entry yet | `nanotorrent/src/bittorrent/torrent_create.rs:457` writes it, `rustorrent/src/torrent.rs:581` validates it, `FluxDown/native/engine/src/bt_downloader.rs:2744,3611` verifies it as virtual zeros and hides it from the file list, `fx-torrent/src/peer/webseed/http.rs:223` skips it when web seeding. |

## E. Interop harnesses worth copying

`bit-cli` currently drives `aria2c` and `rqbit` from PowerShell against two
loopback fixtures. Three richer harnesses exist here:

- `superseedr/integration_tests/` — Docker, per-client Python adapters
  (qBittorrent, Transmission, superseedr), scenario JSONs covering v1/v2/hybrid
  and TCP-only/uTP-only/dual-stack, a local HTTP tracker, and a
  manifest+hash validator.
- `vortex/scripts/transmission_containers.sh` — the minimal version: start N
  Transmission seed containers and read back their IPs.
- `seedchamp/bench/lt_peer.py` — a libtorrent-driven peer for load and
  conformance testing from the other side.

Adding a libtorrent leg matters specifically because it is the only widely
deployed BEP 52 implementation, so it is the only thing that can validate T-081.

## F. Licences at a glance

Most are permissive and compatible with `bit-cli`'s MIT licence and its
permissive-only `deny.toml`. **Four are not straightforward** and are marked
below: `DOAL`, `NetDrop`, `gaia`, and the `librtbit` family in entry 40.
Nothing in this corpus is copied into `bit-cli`, so an unclear licence is a
reading question rather than a shipping one, and `intermodal` is still the one
tree that may be copied from.

| Repository | Licence | Evidence |
|---|---|---|
| torrent | MIT | `torrent/LICENSE` (+ `torrent/webtorrent/LICENSE`, MIT, © 2019 Michiel De Backker) |
| nanotorrent | MIT | `nanotorrent/Cargo.toml` `license = "MIT"`; **no LICENSE file** |
| TorrentNG | MIT | `TorrentNG/LICENSE`, plus `TorrentNG/NOTICE` |
| superseedr | MIT | `superseedr/LICENSE` |
| fx-torrent | MIT | `fx-torrent/LICENSE` |
| mkbrr | MIT | `mkbrr/LICENSE` |
| intermodal | **CC0-1.0** | `intermodal/LICENSE` ("Creative Commons Legal Code / CC0 1.0 Universal") |
| gosh-dl | MIT | `gosh-dl/LICENSE`, © 2025 goshitsarch-eng |
| vortex | MIT | `vortex/LICENCE.txt` |
| rustorrent | MIT | `rustorrent/LICENSE`, © 2026 Josu San Martin |
| mtorrent | MIT | `mtorrent/Cargo.toml:15`; **no LICENSE file** |
| n0-mainline | MIT | `n0-mainline/LICENSE-MIT`, © 2021 raptorswing, © 2025 nuh.dev |
| seedchamp | MIT | `seedchamp/LICENSE.md`, © 2026 Jesse Miller |
| aria2_rust | MIT | `aria2_rust/LICENSE` |
| FluxDown | MIT | `FluxDown/LICENSE` |
| aquatic | MIT | `aquatic/LICENSE` |
| torrust-actix | MIT | `torrust-actix/LICENSE` (the MIT permission text under "Copyright (c) 2024-2026 Power2All") |
| create-torrent | MIT | `create-torrent/LICENSE`, © Feross Aboukhadijeh and WebTorrent, LLC |
| parse-torrent | MIT | `parse-torrent/LICENSE`, same holders |
| bqti | MIT | `bqti/LICENSE` |
| dht-spider | MIT | `dht-spider/LICENSE`, © 2025 AdySec |
| tc | MIT | `tc/LICENSE` |
| joal | **Apache-2.0** | `joal/LICENSE` |
| DOAL | MIT, **claimed in one README line only** | `DOAL/README.md:264-266`. No licence file, no key in `go.mod`, and it forks Apache-2.0 joal |
| Seedr | MIT | `Seedr/LICENSE`, Copyright 2025 Radu Ursache |
| RatioForge | MIT | `RatioForge/LICENSE`, Copyright 2006-2016 Nikolay Kostov, carried from `NikolayIT/RatioMaster.NET` |
| fake-torrent-client | MIT | `fake-torrent-client/LICENSE`, with the holder and year left as the template placeholders |
| rustatio | MIT | `rustatio/LICENSE`, Copyright 2025-2026 takitsu21 |
| RatioTracker | MIT | `RatioTracker/LICENSE`, Copyright 2026 Flavien |
| dig-nat | Apache-2.0 OR MIT | `dig-nat/Cargo.toml:19` and `dig-nat/README.md:66`; **no licence file** |
| tcp-transfer-ice | MIT | `tcp-transfer-ice/LICENSE`, Copyright 2026 well0nez |
| iroh-fm | MIT | `iroh-fm/LICENSE`, Kamil Jakubus and contributors |
| ed2k-server | MIT | `ed2k-server/LICENSE`, Copyright 2026 emule-security.org |
| Hollow | MIT | `Hollow/Cargo.toml:13` and `Hollow/README.md:193`; **no licence file** |
| NetDrop | **conflicting** | `NetDrop/LICENSE` is GPL-3.0 and `NetDrop/Cargo.toml:9` says MIT |
| iroh-experiments | Apache-2.0 OR MIT | `iroh-experiments/LICENSE-APACHE` and `LICENSE-MIT` |
| gaia | **none found** | no licence file, no manifest key, no statement in any document |
| demagnetize-rs | MIT | `demagnetize-rs/LICENSE`, Copyright 2023-2026 John Thorvald Wodder II |
| dht-crawler | MIT | `dht-crawler/LICENSE`, Copyright 2024 dht-crawler contributors |
| `TheDancingDeveloper-org`, 33 repositories | **mixed, and four are copyleft** | entry 40's table, read per repository. Nothing cloned, nothing taken |
| the 2026 Rust GUI survey | not stated on the page | a document, quoted with attribution the way `aria2-next` is |

Two to handle carefully: **`tc`** (README/LICENSE conflict — confirm before
reusing anything) and **`vortex`** (badge/file conflict — the file is MIT, and
the discrepancy is noted in its trimmed README).

## G. What was removed, and what was kept

Removed as noise: all `.git` and `.github` (none were present on arrival in the
eighteen supplied clones, and the four repositories cloned during this pass had
theirs deleted immediately; verified again after cleaning); Flutter/React/Wails
GUI trees and their platform scaffolding; marketing sites, promotion
directories, donation and chat badges, screenshots, demo GIFs and load-test
PDFs; installers, packaging and release scripts; agent-behaviour prompt files;
`aria2_rust`'s language bindings and its non-BitTorrent
migration/compatibility bookkeeping (per the BitTorrent-only instruction);
`bqti`'s vendored C crypto/TEE libraries; `rustorrent`'s bundled qBittorrent
search-plugin runtime and macOS app bundle; the WebUI vendored inside
`nanotorrent/vendor/librqbit/`, `torrust-actix`'s prebuilt webpack bundle
`lib/rtctorrent/dist/` (its `src/` is kept), and `aquatic`'s committed
`criterion` benchmark output; stray `.gitignore`, `.dockerignore` and cargo
vendoring metadata; and two large binary test payloads (`mtorrent`'s 25 MiB
`pcap/` and 1.1 MiB `screenshots/`, and `fx-torrent`'s 7.8 MiB
`piece-1_30.iso`) whose describing `.torrent` files were kept.

Kept deliberately even though it is not implementation code: every `.torrent`
fixture; `n0-mainline/beps/` (the BEP texts); `intermodal/book/`;
`torrust-actix/RtcTorrent.md`; `TorrentNG/docs/`,
`TorrentNG/torrentng_engine_rewrite_spec.md` and
`TorrentNG/testdata/migration-corpus/`; `superseedr/docs/`,
`superseedr/agentic_plans/` (the technical ones) and
`superseedr/integration_tests/`; `rustorrent/docs/DEEP_AUDIT_REPORT_*.md` and
`rustorrent/docs/TEST_COVERAGE.md`; `seedchamp/docs/` and `seedchamp/bench/`;
all `fuzz/` targets and `proptest-regressions/`; `n0-mainline/docs/plot.png`
and `standard-deviation-vs-lookups.png` (figures the retained
`docs/dht_size_estimate.md` refers to); and `nanotorrent/patches/` with its
`vendor/librqbit/PATCHES.md`.

**Manifest and lock sweep.** Every `Cargo.toml`, `Cargo.lock`, `go.mod`,
`go.sum`, `go.work`, `go.work.sum`, `package.json`, `package-lock.json`,
`pnpm-lock.yaml`, `bun.lock` and JS/TS build config (`tsconfig*.json`,
`vite.config.*`, `webpack.config.js`, `postcss.config.js`,
`tailwind.config.ts`, `vitest.config.ts`) was removed, along with every
`CHANGELOG.md`, `vortex/CLAUDE.md`, the one-line `torrent/fs/TODO`,
`bqti/docker/`, and `gosh-dl`'s two HTTP-only design documents
(`recursive_http_design.md`, `recursive_http_checklist.md`) under the
BitTorrent-only instruction. **Five manifests were kept back, because each is
the only in-repo evidence for a fact cited above:**

| Kept manifest | Why |
|---|---|
| `mtorrent/Cargo.toml` | the only licence statement in the repository (no `LICENSE` file) |
| `nanotorrent/Cargo.toml` | same, and the `[patch.crates-io]` target that `vendor/librqbit/PATCHES.md` points at |
| `nanotorrent/vendor/librqbit/Cargo.toml` | `version = "8.1.1"` — the vendored librqbit version |
| `nanotorrent/vendor/librqbit-tracker-comms/Cargo.toml` | `version = "3.0.0"` |
| `FluxDown/native/engine/vendor/librqbit/Cargo.toml` | `version = "8.1.1"`, the cross-check copy of upstream |

These trees are for reading, not for building: with the manifests gone, none of
them compiles, and that is deliberate.

Where a trimmed tree's own documents still point at something that was removed
(TorrentNG's `docs/` at `webui/` and `deploy/docker/`, `aria2_rust`'s README at
`docs/compatibility-status.md`, `rustorrent`'s March audit at `macos/`), those
documents were left untouched and the removal is recorded at the top of that
repository's `README.md` instead of editing upstream prose.

The corpus went from roughly 180 MB to 52 MB. No source file was modified; only
`README.md` files were trimmed or, where their content is captured above,
replaced or removed. `FluxDown/README.md`, `tc/README.md`,
`torrent/README.md`, `vortex/README.md` and `superseedr/README.md` were
rewritten as technical indexes; every other trimmed README carries a
"Note on this copy" block recording exactly what was removed from that tree.

### Appended 2026-08-24

**Seventeen trees arrived and were trimmed the same way**, by deleting: `.git`
and `.github` after the SHA was captured, `node_modules`, build output, images,
binaries, archives, fonts and lock files. Five files were large enough to name:
`joal/JOAL.psd` at 3.9 MB, two classifier training sets in `gaia/craw` at 3.3
and 3.7 MB, `gaia/craw/repomix-output.xml`, and
`demagnetize-rs/THIRDPARTY.toml` at 2.4 MB, which is a generated licence dump
rather than source. The seventeen went from 51 MB to 25 MB and the whole corpus
is 69 MB.

**Nothing was moved.** Every trim deleted, so every citation written before it
still resolves, which is the rule `docs/reference-mining.md` states first.

**Twelve subsections left `RESEARCH.md` for `reference/HISTORY/`**, each
because the entry it informed is closed and the behaviour was verified present
in `man/bit-cli.json` or at a path that was opened. Each left a one line
pointer at the heading it used to fill. The five files and what they hold are
in `HISTORY/README.md`.

**What was deliberately kept even though its entry is closed**, because
something in it is still true of this tree and nothing else records it:

| kept | why |
| --- | --- |
| entry 1, "Web seeding" | it records that anacrolix removes **only that file's pieces** from a web seed's bitmap on 403, 404, 410 and 451, where `bit-cli` retires the whole source. That is a live difference and it is not filed anywhere yet |
| entry 3, "Storage scheduling" | T-018 closed and [T-192](../TODO/disk-io.md) is its open residue |
| entry 6, "Unicode normalisation" | T-103 closed and [T-175](../TODO/create-seed.md), NFD normalisation on create, is open |
| entry 10, "Windows filesystem safety" | T-070 to T-072 closed, and it names a check-then-open window `bit-cli`'s path planner still has |
| entry 13, "Staging memory" | T-041 closed and [T-242](../TODO/performance.md) is new and rests on the same document |
| entry 15, "NTFS sparse files" | it is the reason `--file-allocation` defaults to `sparse`, which is a constraint a reader needs rather than history |

## H. Repositories on the supplied list, and what happened to each

| Listed repository | Local directory | Status |
|---|---|---|
| anacrolix/torrent | `torrent` | supplied, cleaned |
| Jagalite/superseedr | `superseedr` | supplied, cleaned |
| greatest-ape/aquatic | `aquatic` | supplied, cleaned |
| Power2All/torrust-actix | `torrust-actix` | supplied, cleaned |
| Nehliin/vortex | `vortex` | supplied, cleaned |
| DanglingPointer/mtorrent | `mtorrent` | supplied, cleaned |
| hnpf/tc | `tc` | supplied, cleaned |
| adysec/dht-spider | `dht-spider` | supplied, cleaned |
| Power2All/nanotorrent | `nanotorrent` | **not supplied** — shallow-cloned during this pass, `.git`/`.github` removed |
| j-c-m/seedchamp | `seedchamp` | **not supplied** — shallow-cloned during this pass |
| OnlyCavas/bqti | `bqti` | supplied, cleaned |
| casey/intermodal | `intermodal` | **not supplied** — shallow-cloned during this pass |
| autobrr/mkbrr | `mkbrr` | supplied, cleaned |
| balovess/aria2_rust | `aria2_rust` | supplied, cleaned (BitTorrent-only) |
| yoep/fx-torrent | `fx-torrent` | supplied, cleaned |
| n0-computer/n0-mainline | `n0-mainline` | supplied, cleaned |
| goshitsarch-eng/gosh-dl | `gosh-dl` | **not supplied** — shallow-cloned during this pass (BitTorrent-only) |
| webtorrent/create-torrent | `create-torrent` | supplied, cleaned |
| webtorrent/parse-torrent | `parse-torrent` | supplied, cleaned |
| snapetech/TorrentNG | `TorrentNG` | supplied, cleaned |
| josusanmartin/rustorrent | `rustorrent` | supplied, cleaned |
| *(not on the list)* | `FluxDown` | present locally on arrival; kept for its vendored `librqbit` and BEP 47 / NTFS-sparse work. Upstream link taken from its own README. |
| anthonyraymond/joal | `joal` | **supplied 2026-08-24**, cloned, trimmed, mined |
| DylanBricar/DOAL | `DOAL` | **supplied 2026-08-24**, cloned, trimmed, mined |
| slundi/fake-torrent-client | `fake-torrent-client` | **supplied 2026-08-24**, Codeberg rather than GitHub, cloned and mined; tracker read through the Forgejo API |
| rursache/Seedr | `Seedr` | **supplied 2026-08-24**, cloned; one named file mined plus the implementation behind it |
| takitsu21/rustatio | `rustatio` | **supplied 2026-08-24**, cloned; one named file mined plus the tracker |
| tsautier/RatioForge | `RatioForge` | **supplied 2026-08-24**, cloned; one named document mined plus the tracker |
| 7h30th3r0n3/RatioTracker | `RatioTracker` | **supplied 2026-08-24**, cloned, mined, and ported into `scripts/check-announce.ps1` |
| DIG-Network/dig-nat | `dig-nat` | **supplied 2026-08-24**, cloned, trimmed, mined |
| well0nez/tcp-transfer-ice | `tcp-transfer-ice` | **supplied 2026-08-24**, cloned, trimmed, mined |
| usagi-coffee/iroh-fm | `iroh-fm` | **supplied 2026-08-24**, cloned, trimmed, mined |
| n0-computer/iroh-experiments | `iroh-experiments` | **supplied 2026-08-24**, cloned; only the named `h3-iroh` directory was read |
| andrey23127/ed2k-server | `ed2k-server` | **supplied 2026-08-24**, cloned, trimmed, mined |
| Gaok1/Hollow | `Hollow` | **supplied 2026-08-24**, cloned, trimmed, mined, refused |
| NETFORY/NetDrop | `NetDrop` | **supplied 2026-08-24**, cloned, refused: the crate holding the traversal is not in the repository |
| vaz3r/gaia | `gaia` | **supplied 2026-08-24**, cloned; the named document mined, and it has no implementation behind it |
| jwodder/demagnetize-rs | `demagnetize-rs` | **supplied 2026-08-24**, cloned, trimmed, mined |
| 0xddy/dht-crawler | `dht-crawler` | **supplied 2026-08-24**, cloned, trimmed, mined |
| `orgs/TheDancingDeveloper-org` | none | **supplied 2026-08-24**, enumerated and triaged read-only through `gh api`. Nothing cloned. 33 repositories, and the operator's claim that all are permissive does not hold |
| `blog.wybxc.cc/blog/rust-gui-survey-2026` | none | **supplied 2026-08-24**, fetched and read. A document rather than a repository |

`gh` was used only to read Issues and Pull Requests, for the repositories in
this table. Everything else — source, licences, fixtures, documentation — came
from the local clones. The four missing repositories were obtained with
`git clone --depth 1`, not with `gh`.
