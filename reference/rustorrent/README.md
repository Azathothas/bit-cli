# Rustorrent

> **Note on this copy.** Trimmed for BitTorrent research: `docs/screenshots/`,
> the macOS app-bundle tree (`macos/`) and the bundled qBittorrent
> search-plugin runtime (`assets/search_runtime/`, third-party BSD Python)
> were removed, along with the build/contributing/licence sections of this
> file. `docs/DEEP_AUDIT_REPORT_2026-03-20.md` still references `macos/`.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.


A compact BitTorrent client implemented in Rust, with console, terminal, and built-in web
interfaces. The CLI builds as one executable; macOS is also distributed as a native app bundle.
It is still prerelease software, so keep a backup of important data.

Works on **macOS 11 or newer** and **Linux**. Windows support is partial (builds, but NAT-PMP gateway detection is not implemented).

## Features

- **BitTorrent protocol** — v1, v2, and hybrid metainfo with SHA-1/SHA-256 verification
- **Magnet links** — v1 metadata fetching via peers and DHT; verified HTTP sources for v2
- **Trackers** — HTTP and UDP (BEP 15)
- **DHT** — Distributed Hash Table (BEP 5) with K-bucket routing and node persistence
- **PEX** — Peer Exchange
- **LPD** — Local Peer Discovery (BEP 14)
- **uTP** — Micro Transport Protocol (BEP 29)
- **MSE/PE** — Message Stream Encryption for obfuscated connections
- **NAT-PMP & UPnP** — automatic port mapping
- **Web seeds** — HTTP/HTTPS seeding (BEP 19)
- **Web UI** — built-in browser interface
- **IP filtering** — blocklist support
- **Torrent creation** — create `.torrent` files from local paths
- **Selective download** — per-file priority
- **Rate limiting** — global and per-torrent bandwidth control
- **Sequential mode** — download pieces in order for streaming
- **Watch folder** — auto-load torrents from a directory
- **Move on complete** — relocate finished downloads
- **Session persistence** — resume state across restarts

## Dependencies

The full build has five direct Rust dependencies:

- `native-tls` — HTTPS tracker support
- `libc` — safe Unix file-opening flags
- `getrandom` — operating-system entropy for protocol identifiers, temporary names, and MSE
- `num-bigint` — MSE Diffie-Hellman (optional, `mse` feature)
- `num-traits` — bigint helpers (optional, `mse` feature)

Everything else — bencode, SHA-1/SHA-256, HTTP, peer protocol, DHT, uTP, UPnP, NAT-PMP, and the
web UI — is implemented in the repository. Exact dependency versions are recorded in
`Cargo.lock`.

## Platform support

| Platform | Status |
|----------|--------|
| macOS 11+ (x86_64, aarch64) | Fully supported |
| Linux x86_64 | Fully supported and CI-tested |
| Linux aarch64 | Source-supported; runtime is not continuously tested in CI |
| Windows | Builds, but NAT-PMP gateway detection unavailable |

Platform-specific capabilities are compile-time gated. MSE private keys use the operating
system's cryptographic random-number provider.

## Usage

```
rustorrent [options] [file.torrent]
```

The client prints progress to stdout and runs until stopped with `Ctrl+C`. Session state is saved
under `<download-dir>/.rustorrent/` and is resumed on restart. Only one torrent path and one magnet
may be supplied at process startup; add further torrents through the web UI or a watch folder.

### Downloading a torrent file

```sh
rustorrent ubuntu.torrent
```

Output:

```
[ubuntu-25.10-desktop-amd64.iso]
  peers: 12/30  down: 4.2 MB/s  up: 128.0 KB/s  progress: 23.4%  eta: 18:32
```

### Downloading a magnet link

```sh
rustorrent --magnet "magnet:?xt=urn:btih:4b07d0071f9ceb21af6b8ba05b3a3c6f507e3fb2&dn=LibreOffice&tr=http://tracker.example.org:6969/announce"
```

For v1 magnets the client fetches metadata from explicit peers, trackers, and DHT. A v2-only or
hybrid magnet currently needs a verified `xs`/`as` HTTP source containing the complete `.torrent`,
including its piece layers.

For hybrid `.torrent` sessions, peer handshakes support the BEP 52 v2 upgrade, but tracker, DHT,
and LPD discovery currently announce the v1 swarm identifier rather than discovering the v1 and
v2 swarms independently.

### Downloading to a specific directory

```sh
rustorrent --download-dir ~/Downloads ubuntu.torrent
```

### Multiple torrents

```sh
rustorrent --ui --watch ~/watch --download-dir ~/downloads
```

Add torrents through the web UI or copy `.torrent` files into the watch directory. Torrents saved
in an existing session are also restored automatically.

### Sequential download (for media streaming)

```sh
rustorrent --sequential movie.torrent
```

Pieces are downloaded in order so a media player can start playback before the download finishes.

### Rate limiting

```sh
# Limit download to 5 MB/s, upload to 1 MB/s
rustorrent --download-rate 5242880 --upload-rate 1048576 ubuntu.torrent

# Per-torrent limit
rustorrent --torrent-download-rate 2621440 file1.torrent
```

Values are bytes per second. `k`, `m`, and `g` suffixes use powers of 1024; `0` and `unlimited`
both mean unlimited.

### Encryption

```sh
# Prefer encrypted connections (default)
rustorrent ubuntu.torrent

# Require encryption — refuse unencrypted peers
rustorrent --encryption require ubuntu.torrent

# Disable encryption
rustorrent --no-encryption ubuntu.torrent
```

### Move completed downloads

```sh
rustorrent --download-dir ~/incomplete --move-completed ~/complete ubuntu.torrent
```

When a torrent finishes, its files are moved from `~/incomplete` to `~/complete`.
The move is recorded transactionally and the torrent resumes seeding from the destination.

### Watch a folder

```sh
rustorrent --watch ~/watch --download-dir ~/downloads
```

The client periodically scans `~/watch` for new `.torrent` files and adds them automatically. Processed files are moved to a `processed/` subdirectory.

### Preallocate disk space

```sh
rustorrent --preallocate ubuntu.torrent
```

Allocates the full file size on disk before downloading. Avoids fragmentation on HDDs.

### Custom listen port

```sh
rustorrent --port 51413 ubuntu.torrent
```

Default listen port is 6881. The client uses NAT-PMP and UPnP to automatically map the port on your router when possible.

### IP blocklist

```sh
rustorrent --blocklist blocklist.txt ubuntu.torrent
```

The blocklist file contains one IP range per line in the format `start-end` or a single IP per line.

### Disable uTP

```sh
rustorrent --no-utp ubuntu.torrent
```

Forces the client to use TCP only. Useful if uTP causes issues with your network.

### Tuning peer counts

```sh
# Use presets instead of hand-tuning every peer knob
rustorrent --peer-profile conservative ubuntu.torrent
rustorrent --peer-profile balanced ubuntu.torrent
rustorrent --peer-profile aggressive ubuntu.torrent

# Allow more peers globally and per torrent
rustorrent --max-peers 500 --max-peers-torrent 80 ubuntu.torrent

# Limit torrents restored or added through the UI/watch folder
rustorrent --ui --max-active 2
```

Profiles adjust `--max-peers`, `--max-peers-torrent`, `--numwant`, and the magnet metadata peer discovery limit:

| Profile | Global peers | Per torrent | Tracker `numwant` | Magnet metadata peers |
|---------|--------------|-------------|-------------------|-----------------------|
| `conservative` | `80` | `12` | `50` | `20` |
| `balanced` | `200` | `30` | `200` | `80` |
| `aggressive` | `500` | `80` | `500` | `160` |

`balanced` is the default and matches the previous behavior. If you also pass `--max-peers`, `--max-peers-torrent`, or `--numwant`, those explicit flags override the preset.

The web UI exposes the same setting in the Transfer panel.

### Write cache

```sh
# Buffer 16 MB of writes before flushing to disk
rustorrent --write-cache 16777216 ubuntu.torrent
```

Reduces disk I/O by batching writes. Useful for slow disks or when downloading many pieces simultaneously.

### Creating a torrent

```sh
rustorrent --create ./my-project \
  --tracker http://tracker.example.com:6969/announce \
  --output my-project.torrent

# Custom piece length (default: 262144 bytes)
rustorrent --create ./my-project \
  --tracker http://tracker.example.com:6969/announce \
  --output my-project.torrent \
  --piece-length 262144
```

### Configuration file

```sh
rustorrent --config rustorrent.conf ubuntu.torrent
```

Example config:

```ini
peer_profile = conservative
download_rate = 5m
upload_rate = 1m
```

Command-line values override environment variables, which override the config file. Unknown
config keys are rejected so misspellings do not silently change behavior.

Or set the environment variable:

```sh
export RUSTORRENT_CONFIG=~/.config/rustorrent.conf
rustorrent ubuntu.torrent

# Or set the preset directly
export RUSTORRENT_PEER_PROFILE=conservative
rustorrent ubuntu.torrent
```

### With the web UI

```sh
# Enable web UI on default port 8080
rustorrent --ui ubuntu.torrent

# Custom port
rustorrent --ui 9090 ubuntu.torrent

# IPv6 loopback
rustorrent --ui-addr '[::1]:8080' ubuntu.torrent
```

Open `http://127.0.0.1:8080` in your browser. The UI lets you add/remove torrents, see progress,
manage files, and configure settings. The built-in server accepts loopback bind addresses only;
for remote access, use an authenticated SSH tunnel or HTTPS reverse proxy rather than exposing
the UI directly to a network.

The web UI also includes a qBittorrent-compatible search panel. It can install raw `*.py` search
plugins, run searches through the bundled Python runtime, and add results with one click. This
requires Python 3.9 or newer. Community plugins from the
[qBittorrent unofficial search plugin wiki](https://github.com/qbittorrent/search-plugins/wiki/Unofficial-search-plugins)
can be installed directly from the UI. Search plugins are executable third-party code: review and
trust a plugin before installing it.

## All options

| Flag | Default | Description |
|------|---------|-------------|
| `[file.torrent]` | | One torrent file to add at startup |
| `--magnet <link>` | | Add a magnet link |
| `--download-dir <dir>` | `.` | Download directory |
| `--port <port>` | `6881` | Listen port for incoming peers |
| `--ui [port]` | off | Enable web UI (default port: 8080) |
| `--ui-addr <addr>` | `127.0.0.1:8080` | Web UI bind address |
| `--tui` | off | Use the interactive terminal interface |
| `--sequential` | off | Download pieces in order |
| `--preallocate` | off | Preallocate disk space |
| `--encryption <mode>` | `prefer` | `disable`, `prefer`, or `require` |
| `--no-encryption` | | Shorthand for `--encryption disable` |
| `--utp` / `--no-utp` | on | Enable or disable uTP |
| `--peer-profile <name>` | `balanced` | Peer preset: `conservative`, `balanced`, or `aggressive` |
| `--max-peers <n>` | `200` | Global peer limit |
| `--max-peers-torrent <n>` | `30` | Per-torrent peer limit |
| `--max-active <n>` | `4` | Max concurrent active torrents |
| `--numwant <n>` | `200` | Peers to request from tracker |
| `--retry-interval <secs>` | `60` | Tracker re-announce interval |
| `--download-rate <rate>` | `0` | Global download limit; accepts `k`/`m`/`g` suffixes |
| `--upload-rate <rate>` | `0` | Global upload limit; accepts `k`/`m`/`g` suffixes |
| `--torrent-download-rate <rate>` | `0` | Per-torrent download limit; accepts suffixes |
| `--torrent-upload-rate <rate>` | `0` | Per-torrent upload limit; accepts suffixes |
| `--write-cache <size>` | `0` | Write cache size; accepts suffixes (0 = disabled) |
| `--move-completed <dir>` | | Move finished downloads to this directory |
| `--watch <dir>` | | Watch directory for new `.torrent` files |
| `--blocklist <path>` | | IP blocklist file |
| `--proxy <url>` | | Proxy peer TCP and HTTP(S) trackers; disables every unsupported direct network path |
| `--geoip-db <path>` | | Load a CSV IPv4-range/CIDR-to-country database |
| `--seed-ratio <ratio>` | `0` | Stop seeding at this ratio (0 = disabled) |
| `--max-seed-time <minutes>` | `0` | Stop seeding after this duration (0 = disabled) |
| `--on-complete <script>` | | Run a script after a torrent completes |
| `--super-seed` | off | Enable super-seeding for completed torrents |
| `--rss <url>` | | Add an RSS/Atom feed; may be repeated |
| `--rss-rule <feed:pattern>` | | Add an RSS matching rule; may be repeated |
| `--rss-interval <seconds>` | `900` | RSS polling interval |
| `--throttle <name:down_kbps:up_kbps>` | | Define a named throttle group |
| `--ratio-group <name:ratio:action>` | | Define a ratio group (`stop`, `pause`, or `none`) |
| `--schedule <seconds:command>` | | Run a supported scheduler command periodically |
| `--log <path>` | | Append logs to a file |
| `--daemon` | off | Fork into the background on Unix and enable the web UI |
| `--pid-file <path>` | | Write the running process ID after startup locking |
| `--create <path>` | | Create a torrent from this file or directory |
| `--tracker <url>` | required | HTTP(S) or UDP tracker used with `--create` |
| `--output <file>` | `<source>.torrent` | Output file used with `--create` |
| `--piece-length <bytes>` | `262144` | Piece length used with `--create` |
| `--config <path>` | | Config file path (env: `RUSTORRENT_CONFIG`) |

Scheduler commands are `pause_all`, `resume_all`, `stop_ratio_reached`,
`throttle_down:<bps>`, and `throttle_up:<bps>`. Completion scripts receive `TORRENT_NAME`,
`TORRENT_DIR`, `TORRENT_HASH`, and `TORRENT_SIZE` environment variables. A completion script is
claimed durably before it is launched, so it is attempted at most once and is not replayed when an
already-complete torrent is stopped or resumed.

Proxy mode is deliberately fail-closed. Peer TCP and HTTP(S) trackers use the configured SOCKS5
or HTTP proxy. Inbound peers, DHT, LPD, uTP, UDP trackers, port mapping, web seeds, RSS polling,
search downloads, and magnet HTTP sources are disabled so they cannot silently bypass it. The
proxy server's own hostname is resolved by the local operating system; use a literal proxy IP if
even that DNS lookup must not leave the host.
