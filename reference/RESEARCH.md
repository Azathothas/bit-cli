# RESEARCH.md — BitTorrent research corpus for `bit-cli`

Scope: everything in `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\`. Twenty-two repositories,
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

- `torrent/tracker/udp/timeout.go:9` — BEP 15 backoff is `15 * 2^n` seconds,
  clamped at `n = 8` (3840 s). **This is what T-064 asks for**, in nine lines.
- `torrent/tracker/udp/client.go` — connection-id caching with a one-minute
  reissue rule (`shouldReconnectDefault`), and an explicit workaround: the
  literal error body `"Connection ID missmatch.\x00"` from
  `tracker.torrent.eu.org:451` forces `connIdIssued` to zero so the next request
  reconnects.
- `torrent/tracker/udp/scrape.go:7-12` — the BEP 48 field-name mismatch is
  documented in-code: UDP calls them seeders/completed/leechers, bencode calls
  them `complete`/`downloaded`/`incomplete`.
- `torrent/tracker/http/scrape.go` — derives the scrape URL with
  `url.JoinPath("..", "scrape")`, i.e. the BEP 48 `/announce`→`/scrape`
  convention that T-065 notes as the only one implemented. No other convention
  exists here either, which is corroboration rather than a fix.
- `torrent/metainfo/announcelist.go` — `OverridesAnnounce` decides whether
  `announce-list` supersedes `announce` (it does unless every tier entry is
  empty); `DistinctValues` flattens with de-duplication.

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

`TorrentNG/crates/rt-tracker/src/tier.rs`:

- `:8` `Tier { trackers, active }`, `:55` `TierSet { tiers, active_tier }`.
- `promote_active()` — **on success, swap the successful tracker to the front of
  its tier**, which is the BEP 12 rule `bit-cli` is missing.
- `advance()` — on failure, move to the next tracker in the tier, then the next
  tier.
- The doc comment notes private torrents (BEP 27) put extra constraints on tier
  switching.

### Tracker backoff and announce storms

`TorrentNG/crates/rt-tracker/src/backoff.rs`:

- `:28` `Backoff::tracker_retry()` — base 60 s, double, cap 1800 s, ±20 % jitter
  (`:38` `next_delay`).
- `:58` `jitter_interval(interval, fraction)` — spreads announces when many
  torrents load at once ("for 15k torrents with a 30-min interval, this spreads
  announces over ±6 minutes"). `bit-cli download` takes any number of sources
  with `-j`; the same storm applies at a smaller scale.

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

This is the only repository here that implements **both** web-seed styles, and
its structure hands `bit-cli` the auto-detection rule directly:

- `gosh-dl/src/torrent/metainfo.rs:125` reads `url-list` and `:128` reads
  `httpseeds` **into two separate fields** (`:36` `url_list`, `:38`
  `httpseeds`), using one shared parser `:391` `parse_url_list` that accepts a
  bencoded string *or* a list and filters to `http://`/`https://`.
- `gosh-dl/src/torrent/webseed.rs:24` `WebSeedType { GetRight, Hoffman }`.
- `:587` `build_piece_url` — GetRight: the URL itself for a single-file torrent,
  otherwise a per-file URL; **Hoffman: `{url}?info_hash={urlencoded}&piece={index}`**.
- `:618` `build_file_url` — trims a trailing `/`, percent-encodes each path
  component, joins with `/`.
- `:662` `download_multifile_piece` — a piece spanning several files becomes one
  ranged `GET` per file, concatenated and then SHA-1'd against the piece hash.
  Every request sets **`Accept-Encoding: identity`** so a transcoding proxy
  cannot silently change the byte range.

**The bug to not copy, and the fix `bit-cli` gets for free:** at `:303` the
manager builds every seed with `let seed_type = WebSeedType::GetRight;` under
the comment "Hoffman-style seeds typically end with specific paths", and
`:479` `all_webseeds()` merges `url_list` and `httpseeds` into one list — so the
style information that was correctly parsed is thrown away before it is used.
`bit-cli` can close T-004 by keying the style off *which metainfo key the URL
came from*, which is what BEP 17 and BEP 19 actually specify, and keeping
`--web-seed-mode` as the override.

### Source lifecycle

`gosh-dl/src/torrent/webseed.rs:33` `WebSeedState { Idle, Downloading, Backoff, Failed }`, `gosh-dl/src/torrent/webseed.rs:138-181`
exponential backoff `initial * 2^min(consecutive-1, 6)` with ±25 % jitter,
capped at `max_backoff` (defaults at `gosh-dl/src/torrent/webseed.rs:239-241`:
5 s initial, 300 s max, 5 consecutive failures), and a `max_failures`
retirement. Six unit tests at `gosh-dl/src/torrent/webseed.rs:782-951` cover state transitions,
backoff growth, cap, and reset-on-success. Compare `bit-cli`'s
`--web-seed-retries` / `--web-seed-max-errors` / `--web-seed-cooldown`: same
model, and the tests are a good template.

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

# Tier 2 — strong on one axis

## 9. `vortex` — Nehliin/vortex

- Upstream: <https://github.com/Nehliin/vortex>
- Local path: `C:\Users\AjamX\Downloads\TEMP\bit-cli\reference\vortex`
- Licence: **MIT** (`vortex/LICENCE.txt`)
- Language: Rust, `io_uring`, Linux ≥ 6.1. Implements BEP 3, 6, 9, 10, 20, 21.

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

`mtorrent-core/src/trackers/udp.rs:150` — `MAX_RETRANSMISSIONS = 3`, and `:160`
`timeout_sec = 15 * (1 << retransmit_n)`, i.e. 15/30/60/120 s, "timeout after
225s". A shorter variant of the BEP 15 ladder than anacrolix's; both are
defensible, and `bit-cli` should pick one and document the total budget.
`mtorrent/mtorrent-core/src/trackers/mod.rs` defines the request/response types
including `num_want` and the three announce events.

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

`docs/design.md:226-236` — two timers, both `0` to disable:
`limits.redundant_seed_idle_secs` (default 15) when both sides are complete and
nothing is moving, and `limits.useless_peer_idle_secs` (default 60) when there is
no actual transfer. **HAVE and KeepAlive do not reset them**; interest,
outstanding requests and torrent-level downloading do not reset them either. An
idle close holds that listen address out of outbound dial on the same torrent
for 300 s with no fail backoff. `bit-cli`'s CLOSE_WAIT problem is peers that
close before handshaking; this is the adjacent discipline for peers that connect
and do nothing.

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

## D. Where `bit-cli`'s open TODOs are answered

| `bit-cli` item | Best source |
|---|---|
| T-004 BEP 17 not auto-detected | `gosh-dl/src/torrent/metainfo.rs:125,128` (parse `url-list` and `httpseeds` separately) + `gosh-dl/src/torrent/webseed.rs:587` (Hoffman URL form). Fix = key the style off the metainfo key. |
| T-007 stalling source | `vortex` PR 143 (15 s no-activity timeout while requests are in flight); `seedchamp/docs/design.md:197` (20 s stall, 4 s in endgame). |
| T-008 duplicate block fetched twice | `torrent/webseed-peer.go:327,331,344` — cancel the stream when no wanted chunk lies within the discard window, but keep draining the buffered body. |
| T-016 fastresume (blocked) | `TorrentNG/crates/rt-fastresume/src/state.rs` — `FileHint`, `ImportPolicy`, `DurabilityWatermark`. Plus superseedr Issue 236 for the cheap version. |
| T-018 one op per 16 KiB block | `TorrentNG/crates/rt-storage/src/elevator.rs:223,251` (coalescing, with tests) and `handle_cache.rs`; anacrolix PR 1051 (handle caching + 1 s/128 MiB completion checkpoints). |
| T-020 CLOSE_WAIT | `seedchamp/docs/design.md:226-236` (two idle timers, and what must *not* reset them); `vortex` Issue 125 (blocklist bad peers so they stop consuming slots). |
| T-022 IPv6 churn | `mtorrent` PR 41 (uTP IPv6); `aquatic` PR 220/221 (separate sockets per family); torrust Issue 36 (dual-stack announce consequences). |
| T-024 / T-083 choke reporting | `vortex/bittorrent/src/torrent.rs:488,594` — the full leech and seed unchoke algorithms with per-round counters, i.e. the state that would be reported. |
| T-034 endgame observable | `vortex/bittorrent/src/piece_selector.rs:91` (`&mut endgame` out-param); `gosh-dl` PR 7 (the double-count bug endgame observability catches). |
| T-041 window cache measured | `seedchamp/docs/design.md:199` — a budgeted, per-torrent staging pool with explicit per-peer caps. |
| T-050 / T-052 DHT | `n0-mainline` adaptive mode (client for 15 min, then server) and `docs/dht_size_estimate.md`; `fx-torrent/src/dht/` for a full BEP 5/33/44/51 surface. |
| T-063 tier order | `TorrentNG/crates/rt-tracker/src/tier.rs` — `promote_active()`. Note `nanotorrent`'s patch 0008: **librqbit flattens `announce_list` tiers into a `HashSet`**, so tier order is not available without a patch. |
| T-064 BEP 15 backoff | `torrent/tracker/udp/timeout.go:9` (`15·2^n`, cap n=8); `mtorrent/mtorrent-core/src/trackers/udp.rs:150,160` (3 retransmits, 225 s total); `aquatic`'s connection-id validator for why ids expire. |
| T-065 scrape convention | `torrent/tracker/http/scrape.go` uses the same `/announce`→`/scrape` rule; no repository here implements another, which is itself the answer. |
| T-081 BEP 52 create | `nanotorrent/src/bittorrent/torrent_create.rs` (v2 **and** hybrid with BEP 47 padding, on top of librqbit); `torrent/merkle/` for the primitives; `rustorrent/src/torrent.rs:542,581` for validation. |
| T-082 BEP 16 superseed | `rustorrent/src/main.rs:10577,11050,12588` — the only implementation here, simplified. |
| T-100 BEP 6 | `vortex/bittorrent/src/peer_comm/peer_connection.rs:89` (spec-conformant set) + `torrent/peerconn.go:1047` (receive) + `seedchamp` PR 7 (honour `Suggest`). Vector in §A. |
| T-101 uTP | `TorrentNG/crates/rt-utp/` (LEDBAT + SACK + tests + a status doc), `mtorrent/mtorrent-core/src/utp/retransmitter.rs` (RTT/RTO), `superseedr/src/networking/utp.rs` (tuning constants). anacrolix Issue 1013 is the argument for binding libutp instead. |
| T-102 BEP 55 | `fx-torrent/src/peer/extension/holepunch.rs` (working) and `torrent/peer_protocol/ut-holepunch/` (codec). `torrent/NOTES.md:15-31` is the design rationale: rendezvous only through relays for the same torrent, or you cannot know which info hash to handshake with. |
| T-103 non-UTF-8 filenames | intermodal Issue 534 and create-torrent Issue 195 for the failure modes; `mkbrr/torrent/normalize.go` for the NFC/NFD half; `parse-torrent/index.js:124` and `:140` for `.utf-8` preference. |
| T-114 `-i/--input-file` | `intermodal`'s `--input` accepting a file of inputs; `mkbrr/torrent/batch.go` + `mkbrr/examples/batch.yaml` + `mkbrr/schema/batch.json`. |
| T-003 / T-135 steer sources at run time | `torrent/requesting.go:191-196` (invert peer ordering while web seeds are active) and `torrent/internal/request-strategy/NOTES.md:19-27` (the full candidate sort, with the 64 MiB unverified-bytes stop at `:14`). `torrent/TODO:8-16` is the same author's unpublished wishlist for that path. |
| T-143 attach a source to a running torrent | `torrent/tests/add-webseed-after-priorities/` — an integration test whose whole premise is `AddWebSeeds` after `DownloadAll`, with a 500 MiB fixture served by Python `rangehttpserver` and an explicit "must still fetch with no peer connected" clause. |
| T-134 v1/v2 reconciliation | `torrent/types/infohash-v2/infohash-v2.go:60` (`ToShort`) and `superseedr/agentic_plans/v2_identity_lossiness_review_2026-04-14.md` (why one hash field is the wrong model). |
| T-201 aria2 RPC parity | `aria2_rust/docs/comprehensive_gap_analysis.md` "RPC Method Coverage (36/36)"; `TorrentNG/crates/rt-api-qbit/src/router.rs` for the qBittorrent alternative. |
| T-203 session save/restore | `TorrentNG/crates/rt-migrate/src/export.rs` + `TorrentNG/testdata/migration-corpus/`; `seedchamp/docs/rtorrent-session.md` and `docs/transmission-session.md`. |
| BEP 47 padding | `nanotorrent/src/bittorrent/torrent_create.rs:457` (write), `rustorrent/src/torrent.rs:581` (validate), `FluxDown/native/engine/src/bt_downloader.rs:2744,3611` (verify as virtual zeros; hide from the file list), `fx-torrent/src/peer/webseed/http.rs:223` (skip when web-seeding). |
| MSE/PE (no TODO yet) | `nanotorrent` patches 0003/0005 + `nanotorrent/src/bittorrent/mse.rs` (the librqbit-specific route), `mtorrent/mtorrent-core/src/pe/` (the cleanest standalone), `mtorrent/mtorrent-core/src/pe/utils.rs:17` (plaintext/encrypted detection on one port). |
| WebTorrent (no TODO yet) | `torrent/webtorrent/` (client), `aquatic/crates/ws_protocol/` (tracker), `torrust-actix/RtcTorrent.md` (the full protocol write-up). |

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

All twenty-two are permissive and compatible with `bit-cli`'s MIT licence and
its permissive-only `deny.toml`.

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

`gh` was used only to read Issues and Pull Requests, for the repositories in
this table. Everything else — source, licences, fixtures, documentation — came
from the local clones. The four missing repositories were obtained with
`git clone --depth 1`, not with `gh`.
