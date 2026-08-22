# tc (torrentclient)

A from-scratch BitTorrent client in Rust with a git-style porcelain CLI. No
external torrent libraries: bencode and the peer wire protocol are implemented
in-repo.

**Status: early / largely unimplemented.** `src/core/dht.rs` is empty and every
file under `src/cli/commands/` is empty or a stub. The only substantial code is
`src/core/peer.rs` (911 lines: handshake, message framing, keep-alive),
`src/core/storage.rs`, `src/core/tracker.rs` (HTTP + UDP announce/scrape),
`src/core/bencode.rs`, `src/core/piece.rs` (rarest-first), and
`src/core/torrent.rs` (.torrent + magnet parsing).

Planned architecture (from the upstream README):

```
core/
  bencode.rs   bencode encoder/decoder (no lib)
  torrent.rs   .torrent parsing and magnet link parsing
  tracker.rs   HTTP + UDP tracker announce/scrape
  peer.rs      peer wire protocol (handshake, msgs, keep-alive)
  piece.rs     piece selection (rarest first), block requests
  storage.rs   disk i/o, piece verification w/ sha1, sparse file alloc
  dht.rs       BEP 5 mainline DHT for trackerless torrents
cli/
  main.rs      arg parsing and command dispatch
  commands/    add, status, verify, peers, remove
```

Open plan item: a daemon so `tc status` can answer over a unix socket without
spinning up a swarm connection.

Test fixture: `test_data/ubuntu.torrent`.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.
