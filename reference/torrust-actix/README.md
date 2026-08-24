# Torrust-Actix Tracker

> **Note on this copy.** Trimmed for BitTorrent research: `icon.ico`,
> the prebuilt webpack bundle `lib/rtctorrent/dist/` (its `src/` is kept),
> `ICON_INSTALLATION.md` and `CODE_OF_CONDUCT.md` were removed, as were the
> installation, Docker, environment-variable, API-auth, caching, PHP-example,
> changelog and credits sections of this file. `RtcTorrent.md` is retained in
> full and is the authoritative protocol document.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.

## Project Description
Torrust-Actix Tracker is a lightweight but incredibly powerful and feature-rich BitTorrent Tracker made using Rust.

Currently, it's being actively used at https://www.gbitt.info/.

This project originated from Torrust-Tracker code originally developed by Mick van Dijke, further developed by Power2All as alternative for OpenTracker and other tracker code available on GitHub.

## Features
* [X] Block array for TCP tracking (HTTP/HTTPS), UDP tracking and API (HTTP/HTTPS)
* [X] Full IPv4 and IPv6 support
* [X] Persistence saving supported using SQLite3, MySQL or PostgresSQL database
* [X] Customize table and database structure in the configuration
* [X] Whitelist system for private tracking
* [X] Blacklist system for blocking unwelcome hashes
* [X] Torrent key support for locking access to announcement through keys as info_hash with a timeout
* [X] User account support, configurable for also database support
* [X] Swagger UI built-in in the API (toggleable), useful both for testing API and documentation for API
* [X] Sentry SaaS and self-hosted support
* [X] Full Stand-Alone/Master/Slave cluster mode
* [X] Optional Redis/Memcache Caching for peers data (can be used to show on a website for instance, to less burden SQL)
* [X] Cloudflare's "Simple Proxy Protocol" support added (https://developers.cloudflare.com/spectrum/how-to/enable-proxy-protocol/#enable-simple-proxy-protocol-for-udp)
* [X] RtcTorrent implementation (as alternative/replacement for WebTorrent)
* [X] Configurable LZ4/Zstd compression for RTC SDP data (lz4 default, enabled by default)
* [X] Adding UDP receive method with recvmmsg or io_uring

## Implemented BEPs
* [BEP 3](https://www.bittorrent.org/beps/bep_0003.html): The BitTorrent Protocol
* [BEP 7](https://www.bittorrent.org/beps/bep_0007.html): IPv6 Support
* [BEP 15](https://www.bittorrent.org/beps/bep_0015.html): UDP Tracker Protocol for BitTorrent
* [BEP 23](https://www.bittorrent.org/beps/bep_0023.html): Tracker Returns Compact Peer Lists
* [BEP 41](https://www.bittorrent.org/beps/bep_0041.html): UDP Tracker Protocol Extensions
* [BEP 48](https://www.bittorrent.org/beps/bep_0048.html): Tracker Protocol Extension: Scrape


---

## RtcTorrent — WebRTC BitTorrent in the Browser

RtcTorrent is the built-in WebRTC peer-to-peer library that lets a browser (or Node.js process) act as a BitTorrent seeder or leecher **without any browser plugin or native binary**. It uses the standard HTTP announce endpoint with additional query parameters for WebRTC signalling.

A full protocol white paper is available in [RtcTorrent.md](./RtcTorrent.md).

### How It Works

```
Browser (leecher) ──announce + rtctorrent=1──► Tracker (Torrust-Actix)
                   ◄── SDP offer from seeder ──
Browser ──answer + rtcanswerfor=<peer_id>────► Tracker
Seeder  ──poll (announce) ───────────────────► Tracker
        ◄── SDP answer ──
WebRTC Data Channel established directly between Browser ↔ Seeder
```

### Building the Browser Bundle

```bash
cd lib/rtctorrent
npm install
npm run build          # produces dist/rtctorrent.browser.js (minified)
npm run dev            # watch mode for development
```

The build outputs two bundles:

| File | Target | Use |
|------|--------|-----|
| `dist/rtctorrent.browser.js` | Browser (`<script>`) | Website player/downloader |
| `dist/rtctorrent.node.js` | Node.js (`require`) | CLI seeder, server-side |

### Using the Library on a Website

Copy `dist/rtctorrent.browser.js` to your web server's static assets, then include it in your HTML:

```html
<script src="/assets/rtctorrent.browser.js"></script>
```

#### Downloading a Torrent (Leecher)

```html
<!DOCTYPE html>
<html>
<head><title>RtcTorrent Demo</title></head>
<body>
  <video id="player" controls autoplay style="width:100%"></video>
  <script src="/assets/rtctorrent.browser.js"></script>
  <script>
    const client = new RtcTorrent({
      trackerUrl: 'http://your-tracker.example.com/announce',
      // Optional: override ICE/STUN servers
      iceServers: [
        { urls: 'stun:stun.l.google.com:19302' }
      ]
    });

    // Download via .torrent URL, magnet URI, or parsed torrent object
    client.download('magnet:?xt=urn:btih:INFOHASH&dn=MyVideo&tr=http://your-tracker.example.com/announce')
      .then(torrent => {
        console.log('Started downloading:', torrent.name);
      });

    // Stream a video file directly into a <video> element
    client.streamVideo('INFOHASH_HEX', 0, document.getElementById('player'));
  </script>
</body>
</html>
```

#### Seeding a File from the Browser

```html
<input type="file" id="filePicker">
<script src="/assets/rtctorrent.browser.js"></script>
<script>
  const client = new RtcTorrent({
    trackerUrl: 'http://your-tracker.example.com/announce'
  });

  document.getElementById('filePicker').addEventListener('change', async (e) => {
    const file = e.target.files[0];

    // Create a torrent from the selected file
    const { torrent, magnetUri, infoHash } = await client.create([file], {
      version: 'v1',   // or 'v2' / 'hybrid'
      name: file.name
    });

    console.log('Magnet URI:', magnetUri);
    console.log('Info Hash:', infoHash);

    // Start seeding — the tracker handles WebRTC signalling
    await client.seed(torrent, [file]);
  });
</script>
```

#### Constructor Options

| Option | Default | Description |
|--------|---------|-------------|
| `trackerUrl` | `''` | HTTP announce URL of the Torrust-Actix tracker |
| `announceInterval` | `30000` | Re-announce interval in milliseconds |
| `rtcInterval` | `10000` | WebRTC signalling poll interval in milliseconds |
| `maxPeers` | `50` | Maximum simultaneous WebRTC peers |
| `iceServers` | Google STUN | Array of ICE server objects |

#### Key Methods

| Method | Description |
|--------|-------------|
| `create(files, options)` | Create a torrent from File objects (browser) or file paths (Node) |
| `download(torrentData)` | Download via magnet URI, `.torrent` URL, or parsed torrent object |
| `seed(torrentData, files)` | Seed an existing torrent |
| `streamVideo(infoHash, fileIndex, videoEl)` | Stream a video piece-by-piece into a `<video>` element |
| `stop()` | Stop all torrents and close connections |
| `parseMagnet(uri)` | Parse a magnet URI into a torrent object |
| `parseTorrentFile(buffer)` | Parse a `.torrent` file buffer |
| `calculateInfoHash(info)` | Calculate the SHA-1 info hash of a torrent info dictionary |

### Tracker Configuration for RtcTorrent

Enable RtcTorrent support on the HTTP listener in `config.toml`:

```toml
[[http_trackers]]
enabled = true
bind_address = "0.0.0.0:6969"
rtctorrent = true          # Enable WebRTC signalling endpoint
```

Or via environment variable:
```
HTTP_0_RTCTORRENT=true
```

### CLI Seeder (Node.js)

The `bin/seed.js` script seeds files from the command line:

```bash
# Install dependencies
cd lib/rtctorrent && npm install

# Single file
node bin/seed.js --tracker http://your-tracker.example.com/announce \
                 --name "My Movie" \
                 --out movie.torrent \
                 /path/to/movie.mp4

# Re-seed from an existing .torrent (no re-hashing)
node bin/seed.js --torrent-file movie.torrent /path/to/movie.mp4

# Seed from a magnet URI
node bin/seed.js --magnet "magnet:?xt=urn:btih:..." /path/to/movie.mp4

# Multi-torrent mode via YAML config
node bin/seed.js --torrents torrents.yaml
```

**YAML multi-torrent config example:**

```yaml
torrents:
  - name: "My Movie"
    file:
      - "/data/movie.mp4"
    trackers:
      - "http://your-tracker.example.com/announce"
    out: "/data/movie.torrent"
    version: v1          # v1 | v2 | hybrid
    webseed:
      - "https://cdn.example.com/movie.mp4"
```

Reload the YAML config without restarting by sending `SIGHUP` (Linux/macOS) or simply saving the file (polled every 2 seconds on all platforms).

### Demo

Run the built-in demo server to test locally:

```bash
cd lib/rtctorrent
npm install && npm run build
npm run serve    # serves demo at http://localhost:8080/demo/
```

---


---

Install, Docker, environment-variable overrides, API auth, Redis/Memcache
key formats, PHP examples, the changelog and credits were removed from this
copy as deployment/operations material. The authoritative protocol document
is `RtcTorrent.md` in this repository.
