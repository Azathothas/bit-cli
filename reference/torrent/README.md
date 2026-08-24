# anacrolix/torrent

Go BitTorrent library and CLI utilities. In production use since late 2014.

Implemented: protocol encryption (MSE), DHT, PEX, uTP, WebTorrent, WebSeeds
(BEP 17/19), BitTorrent v2 (BEP 52), holepunching (BEP 55), LSD (BEP 14),
canonical peer priority (BEP 40), fast extension (BEP 6), private torrents
(BEP 27).

Storage backends under `storage/`: file (classic and mmap file I/O), bolt,
mmap, sqlite, possum, and a piece-resource interface for custom backends
(`storage.ClientImpl`).

Notable in-repo packages:

- `bencode/` — bencode codec with fuzz corpus and torrent testdata.
- `metainfo/` — v1 + v2 metainfo, magnet v1/v2, file tree, piece layers,
  announce-list, url-list, piece-length selection.
- `merkle/` — BEP 52 SHA-256 merkle tree and streaming `hash.Hash`.
- `peer_protocol/` — wire messages incl. fast extension and v2 hash messages,
  extended handshake, `ut_holepunch/`.
- `tracker/` — HTTP and UDP tracker clients and servers, scrape (BEP 48).
- `webseed/`, `webseed-peer.go`, `webseed-requesting.go` — BEP 19 web seeding
  and its request scheduler.
- `webtorrent/` — WebRTC/WSS tracker client and data-channel transport.
- `smartban/` — per-block peer attribution for hash failures.
- `segments/` — piece/file extent index.

Test fixtures of note:

- `testdata/bittorrent-v2-test.torrent` (pure v2) and
  `testdata/bittorrent-v2-hybrid-test.torrent` (hybrid).
- `metainfo/testdata/flat-url-list.torrent` — `url-list` bencoded as a plain
  string rather than a list.
- `metainfo/testdata/trackerless.torrent`, `minimal-trailing-newline.torrent`,
  `issue_65a.torrent`, `issue_65b.torrent`.

## Building from a checkout

`go.work` adds the `possum` storage backend, vendored as a git submodule at
`storage/possum/lib`. Without the submodule, `go build ./...` fails on
`storage/possum/lib/go/go.mod`. Set `GOWORK=off` to build without it.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.

## Command packages

- `cmd/torrent` — `torrent download <magnet|file>`, and
  `torrent metainfo <file> magnet` to derive a magnet URI (extracts trackers,
  display name, info hash).
- `cmd/torrent-pick`, `cmd/torrent2`, `cmd/magnet-metainfo`.
- `fs/` (`torrentfs`) — FUSE mount serving torrent contents on demand.
