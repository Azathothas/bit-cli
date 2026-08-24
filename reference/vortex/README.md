# Vortex

`vortex-bittorrent`: pure `io_uring` BitTorrent library for Linux (min kernel
6.1). `vortex-cli`: trackerless TUI client using it, with DHT peer discovery
via the `mainline` crate.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.

## Design

- Single `io_uring` thread owns all I/O; no async runtime. Piece hashing is
  offloaded to a separate thread pool so the I/O loop never blocks.
- No locking and no `Arc` on the hot path: pieces are handed to the hashing
  thread(s) under Rust lifetimes.
- Optimised for SSD/NVMe; it does not try to optimise write-head locality.
- Communication with the torrent thread is by `Command`; progress arrives as
  `TorrentEvent`.
- `metrics` crate integration behind the `metrics` feature.

Key modules: `bittorrent/src/event_loop.rs` (dispatcher),
`peer_comm/` (connection + protocol + 5.3k lines of protocol unit tests),
`piece_selector.rs` (random-first / rarest-first / endgame),
`file_store.rs` (piece↔file mapping, with misaligned multi-file tests),
`buf_pool.rs` / `buf_ring.rs` (registered buffers),
`connection_manager.rs`.

Integration tests seed from Transmission containers; see
`scripts/transmission_containers.sh` (adapted from cratetorrent).
`fuzz/fuzz_targets/` covers peer-message parse and round-trip.

## BEP implementation status (as stated upstream)

| BEP | Title | Status |
|-----|-------|--------|
| 3 | The BitTorrent Protocol Specification | Implemented |
| 6 | Fast Extension | Implemented |
| 7 | IPv6 Tracker Extension | Not Implemented |
| 9 | Extension for Peers to Send Metadata Files | Implemented |
| 10 | Extension Protocol | Implemented |
| 11 | Peer Exchange (PEX) | Not Implemented |
| 12 | Multitracker Metadata Extension | Won't implement (trackerless) |
| 14 | Local Service Discovery | Not Implemented |
| 19 | WebSeed - HTTP/FTP Seeding (GetRight style) | Not Implemented |
| 20 | Peer ID Conventions | Implemented |
| 21 | Extension for Partial Seeds | Implemented |
| 27 | Private Torrents | Not Implemented |
| 29 | uTorrent Transport Protocol (uTP) | Not implemented |
| 40 | Canonical Peer Priority | Not implemented |
| 52 | The BitTorrent Protocol Specification v2 | Not Implemented |
| 54 | The lt_donthave extension | Not Implemented |

Where a BEP was underspecified, the author states libtorrent was used as the
reference implementation.

## Benchmark method (upstream claim: ~3x transmission-cli 4.0.6)

```
rm -rf ~/.config/transmission && rm -rf ~/Downloads/linuxmint-22-cinnamon-64bit.iso
time transmission-cli -f <script that terminates process> -D linux-mint.torrent
```

```
rm -rf downloads/ && rm -rf ~/.cache/vortex
vortex-cli -t linux-mint.torrent -d downloads
```
