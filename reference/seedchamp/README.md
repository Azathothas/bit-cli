# seedchamp

> **Note on this copy.** Trimmed for BitTorrent research: `scripts/tag-release.sh`
> and `AGENTS.md` were removed, along with the install, quick-start,
> everyday-use, development and licence sections of this file. Retained:
> `crates/`, `src/`, `docs/`, `bench/`.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.
**The Seed Champion** — a high-performance BitTorrent client built for seedboxes that refuse to compromise. Seed massive libraries without drowning in RAM: the catalog lives in SQLite, and only active torrents hold wire state. When you need to rip data in, pile on an SSD leech cache and huge memory buffers for ultra-high-speed leech.

Use the terminal UI day to day, or run headless with `serve`. Same engine either way. Linux, FreeBSD, and macOS.

## Why seedchamp

- **Large libraries, efficient resources** — thousands of torrents in the catalog without densifying idle ones
- **Seedbox first** — headless serve, rate limits, watch dirs, rtorrent / Transmission session import and export
- **Fast I/O path** — Compio-based networking; platform-aware upload and disk backends
- **Terminal native** — ratatui list, detail, peers, files, and activity log

It talks to trackers only (no DHT or PEX). Magnets, WebUI, and Windows are out of scope.

## CLI cheatsheet

```bash
seedchamp torrent add ./film.torrent
seedchamp torrent add https://example.com/a.torrent --start
seedchamp torrent list
seedchamp torrent start <id-or-infohash-prefix>
seedchamp torrent stop  <id-or-infohash-prefix>
seedchamp torrent del   <id-or-infohash-prefix>
seedchamp torrent recheck <id-or-infohash-prefix>
seedchamp torrent --json list

seedchamp import rtorrent /path/to/session
seedchamp import transmission ~/.config/transmission-daemon
seedchamp export rtorrent /path/to/session --all
seedchamp export transmission /path/to/session --all

seedchamp config show
seedchamp doctor
seedchamp version
```

Global flags: `--config PATH`, `--db PATH`.

## Configuration

```bash
seedchamp config init    # commented template
seedchamp config show    # effective config after CLI/env/file
```

Settings resolve in order: CLI flags, `SEEDCHAMP_*` environment variables, the config file, then built-ins.

Encryption mode (`network.encryption`): `prefer-plain` by default; also `off`, `prefer-rc4`, `require-rc4`.

The init template covers paths, peer limits, upload/disk backends, watch dirs, rate limits, and optional leech cache (stage downloads on a fast volume before the permanent data root).

## Docs

| | |
|--|--|
| [docs/design.md](docs/design.md) | Architecture |
| [docs/domains.md](docs/domains.md) | Modules and I/O |
| [docs/roadmap.md](docs/roadmap.md) | Open work |
| [docs/rtorrent-session.md](docs/rtorrent-session.md) | rtorrent import / export |
| [docs/transmission-session.md](docs/transmission-session.md) | Transmission import / export |
| [bench/README.md](bench/README.md) | Smoke and throughput harness |
