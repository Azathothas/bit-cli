# bit-cli

A non-interactive BitTorrent and HTTP download tool that lets you attach
arbitrary web seeds to an existing torrent, from the command line, without
rewriting the `.torrent`.

```bash
bit-cli download ubuntu.torrent \
  --web-seed https://mirror-a.example.com/pub/ \
  --web-seed-for 'file:0=https://cdn.example.com/blobs/a3f1/payload.iso'
```

The torrent is not modified. Its info hash does not change. The sources exist
for the length of that one invocation.

## Why

No other command-line client has a named, documented way to say "here is a
torrent, here are N extra HTTP sources, go". `aria2` has no `--bt-web-seed`
flag: its 207 documented options contain no web seed option of any kind. What
it does have is an RPC method, `aria2.addTorrent`, whose `uris` array is used
for web seeding, and undocumented positional URIs alongside a `.torrent` on the
command line. Neither gives per-file control, header control, or a way to feed a
list from a file.

`bit-cli` is that surface.

## Install

Prebuilt binaries for `x86_64-linux`, `aarch64-linux`, and `x86_64-windows` are
on the releases page, each with a BLAKE3 checksum and a build provenance
attestation. The Windows binary is statically linked against the C runtime, so
it runs without a Visual C++ redistributable.

```bash
b3sum --check bit-cli-x86_64-unknown-linux-musl.tar.xz.b3sum
```

From source:

```bash
cargo install --path crates/bit-cli --locked
```

## The addressing model

Every other client treats a web seed as one flat thing: a URL that serves the
whole torrent. Here a binding is a triple.

**Source** is where bytes come from: an HTTP(S) URL, with its own headers,
auth, user agent, timeouts, concurrency, and rate limit.

**Scope** is what part of the torrent that source may serve. A mirror holding
part of the payload is a first-class case, not an error.

**Composition** is how the request URL is built from the source URL and the
torrent's `name` and `path`.

The three are orthogonal. Any source can serve any scope under any
composition.

### Composition modes

| Mode | What it does |
| --- | --- |
| `auto` | BEP 19 default. Single-file: a URL ending in `/` gets `name` appended, otherwise the URL is the complete resource. Multi-file: `name` and `path` are appended per file. Matches `aria2`, so migrating a script is mechanical. |
| `exact` | The URL is the complete resource. Nothing is appended. For a mirror whose layout does not match the torrent's, or a file renamed on the server. |
| `prefix` | Appends `path` but not `name`. For mirrors hosting the contents at the root rather than inside a directory named after the torrent. |
| `template` | The URL carries placeholders expanded per request: `{name}` `{path}` `{filename}` `{index}` `{piece}` `{offset}` `{length}` `{end}` `{piece_offset}` `{piece_length}` `{infohash}`. Everything is percent-encoded unless written `{raw:path}`. |

### Scope selectors

```
*                    every file
3                    file index 3
3-7                  file indices 3 through 7, inclusive
3,5,9-               an index list and an open-ended range
path/to/file.iso     an exact path within the torrent
*.iso                a glob against the file path
!*.nfo               a negated glob, subtracted from the selection
piece:0-511          a piece index range
byte:0-1MiB          a byte range within the whole payload
file:3:byte:0-4MiB   a byte range within one file
```

Selectors are checked against the metainfo before any request goes out. A
selector matching nothing is an error, not a silent no-op.

## Check the addressing before you download

`webseed list` resolves every binding and prints the exact URL each file maps
to. It touches no network.

```bash
bit-cli webseed list album.torrent --web-seed https://mirror.example.com/pub/
```

```
torrent              album
info hash            6700edefb64af8f2cf692179ae5b0092f824bda6
size                 43.95 KiB
sources              1
coverage             43.95 KiB of 43.95 KiB (100.00%)

[0] https://mirror.example.com/pub/
  scope              * (100.00%, 2 files, 3 whole pieces, 0 partial)
  composition        auto / auto / priority 0
  origin             torrent_url_list
  FILE  IN SCOPE   PATH           URL
  0     39.06 KiB  disc 1/a.flac  https://mirror.example.com/pub/album/disc%201/a.flac
  1     4.88 KiB   notes.nfo      https://mirror.example.com/pub/album/notes.nfo
```

Note the space in `disc 1` came back as `%20` and the `/` separators did not.
Getting that wrong is the most common way a web seed silently serves nothing.

Then check the mirrors answer:

```bash
bit-cli webseed test album.torrent --web-seed https://mirror.example.com/pub/
```

This reports range support, the entity length against what the torrent says,
the redirect chain hop by hop, the negotiated TLS version and cipher suite, and
the latency. One request per source, one byte of payload at most.

## Commands

```
bit-cli download <SOURCE>...    Fetch to completion in the foreground, then exit
bit-cli info <SOURCE>           Parse a torrent, magnet, or metalink and print its metadata
bit-cli files <SOURCE>          List files with index, path, and size
bit-cli peers <SOURCE>          Connect, sample the swarm, report peers, exit
bit-cli trackers <SOURCE>       Announce or scrape, report tier, interval, seeders, leechers
bit-cli webseed <SUBCOMMAND>    list | test | probe | fetch
bit-cli verify <SOURCE>         Hash-check existing data, per piece
bit-cli create <PATH>           Create a .torrent
bit-cli edit <TORRENT>          Rewrite metainfo fields, writing a new file
bit-cli magnet <SOURCE>         Convert a torrent to a magnet URI
bit-cli seed <SOURCE>           Seed existing data in the foreground
bit-cli bench <SUBCOMMAND>      Measure a target
bit-cli config show             Print the resolved configuration with the origin of every value
bit-cli completions <SHELL>     bash | zsh | fish | powershell | elvish | nushell
bit-cli man                     Generate a man page
bit-cli version                 Version, build metadata, features, protocol support
```

`bit-cli <SOURCE>` with no subcommand is `bit-cli download <SOURCE>`.

Sources accepted: a path to a `.torrent`, an HTTP(S) URL to one, a magnet URI,
a bare info hash, a Metalink, and `-` for stdin.

Every command runs in the foreground, does its work, and exits. There is no
daemon and no stored session.

## Fetch one piece from one mirror

```bash
bit-cli webseed fetch album.torrent \
  --url https://mirror.example.com/pub/ \
  --piece 42 --verify --json
```

Writes nothing unless `--output` is given, exits non-zero on a hash mismatch,
and reports full timing. Under `--trace http` it also prints the equivalent
`curl` command for every request it made, which is the standard the trace is
held to: if you cannot reproduce a failing request by hand from the log, the
trace is not detailed enough.

## Machine output

Two rules, and neither bends.

**stdout carries data only.** JSON, NDJSON, or the requested plain values.
`bit-cli ... --json | jq` never sees a log line in the pipe.

**stderr carries logs, progress, warnings, and errors.**

```bash
bit-cli info album.torrent --json | jq -r .info_hash
```

`--jsonl` emits one event per line as things happen, each with a monotonic
`seq` and an ISO 8601 UTC millisecond timestamp.

Nothing is TTY-gated. Terminal detection reaches exactly two decisions, colour
and progress rendering, and never decides what the program does, computes, or
reports. Anything you can read in the terminal is a field in `--json`.

### On Windows

PowerShell surfaces the exit code in `$LASTEXITCODE`, not `$?`.

Windows PowerShell 5.1 writes UTF-16LE through `>` and `Out-File`, which breaks
piping JSON into `jq`. Pipe directly, or name the encoding on PowerShell 7:

```powershell
bit-cli info album.torrent --json | ConvertFrom-Json
bit-cli info album.torrent --json | Out-File -Encoding utf8NoBOM info.json
```

## Paths

A `.torrent` is untrusted input, and its file names decide where bytes land.
Three of them cannot be used as written:

- A component the platform reads as a drive or a root. On Windows
  `Path::new("D:/out").join("C:")` is `C:`, so a two-character component
  relocates the download out of the output directory.
- A name the filesystem refuses: `CON`, `NUL`, `COM1`, a trailing dot or space,
  or any of `< > : " | ? *`.
- Two names that differ only in case. NTFS and APFS treat `README` and `readme`
  as one file, so the second write wins and the first payload is gone.

`bit-cli` plans every path before it opens anything. Each file lands inside the
output directory, under a name the filesystem accepts, and no two files collide.
The rules run on every platform, so a payload downloaded on Linux and copied to
Windows still works.

Nothing is silent. A changed path is reported on stderr and in `--json`:

```bash
bit-cli download hostile.torrent --json | jq '.torrents[0].renamed'
```

```json
[
  {
    "index": 0,
    "torrent_path": "C:/pwned.txt",
    "disk_path": "C_/pwned.txt",
    "reasons": ["escape", "illegal-character"]
  },
  {
    "index": 1,
    "torrent_path": "CON.txt",
    "disk_path": "CON_.txt",
    "reasons": ["reserved-name"]
  }
]
```

The key is absent when nothing changed, which is the common case. `index` is
the file's index in the torrent, so a caller can reconcile what it asked for
with what is on disk.

`bit-cli create` refuses to build such a torrent in the first place. The
`windows-path` and `case-collision` lints catch it, and `--allow` overrides
either one.

## Exit codes

The exit code is the primary success signal. A caller branches on it without
parsing any text.

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Generic failure |
| 2 | Usage or argument error |
| 3 | Configuration error |
| 4 | Source resolution failed |
| 5 | Network failure |
| 6 | No usable sources |
| 7 | Hash verification failed |
| 8 | Disk error |
| 9 | Timeout or deadline exceeded |
| 10 | Interrupted, partial state saved |
| 11 | Coverage gap: some pieces have no source |
| 12 | Binding error: a scope selector or composition mode is invalid |
| 13 | A lint refused a torrent at creation |
| 14 | Threshold not met |
| 15 | Would change the info hash |

Codes 11 through 15 exist so a script can tell "your mirrors are
misconfigured" from "the network is down" from "your server is slow".

## Configuration

Highest wins:

1. Command-line flags
2. Environment variables, prefixed `BIT_CLI_`
3. `--config <PATH>`
4. `./bit-cli.toml`
5. The user config directory
6. Built-in defaults

```bash
bit-cli config show --json
```

prints every value with where it came from, which is what makes the tool
debuggable in CI. A `BIT_CLI_*` variable matching no setting is an error, not a
silent no-op, because that is how a production setting goes missing.

## Binding tables

Anything expressible on the command line is expressible in a file, in TOML or
JSON, with the same schema.

```toml
[[source]]
url         = "https://mirror-a.example.com/pub/"
scope       = "*"
mode        = "auto"
priority    = 10
concurrency = 8

[[source]]
url   = "https://cdn.example.com/blobs/a3f1b2/payload.bin"
scope = "file:0"
mode  = "exact"

[[source]]
url     = "https://partial.example.com/chunks/{piece}.bin"
scope   = "piece:0-2047"
mode    = "template"
headers = { Authorization = "Bearer ...", X-Region = "apac" }

[[source]]
url        = "https://slow-but-complete.example.com/iso/"
scope      = "*"
mode       = "prefix"
priority   = 1
rate_limit = "5MiB/s"
```

```bash
bit-cli download release.torrent --web-seed-config web-seeds.toml
```

## Protocol support

| BEP | What | Status |
| --- | --- | --- |
| 3 | The BitTorrent protocol | yes |
| 5 | DHT | yes |
| 9 | Metadata from peers | yes |
| 10 | Extension protocol | yes |
| 11 | PEX | yes |
| 12 | Multitracker metadata | yes |
| 14 | Local service discovery | yes |
| 15 | UDP tracker protocol | yes |
| 17 | HTTP seeding, Hoffman style | yes |
| 19 | HTTP seeding, GetRight style | yes |
| 20 | Peer id conventions | yes |
| 21 | Extension for partial seeds | yes |
| 23 | Compact peer lists | yes |
| 27 | Private torrents | yes |
| 29 | uTP | available, off by default |
| 39 | Updating torrents via feed URL | yes |
| 48 | Tracker scrape | yes |
| 6 | Fast extension | no |
| 16 | Superseeding | no |
| 47 | Padding files | no |
| 52 | BitTorrent v2 | no |
| 55 | Holepunch | no |

`TODO/bep-coverage.md` tracks the gaps.

## Building

```bash
cargo build --release --locked
cargo test --workspace
```

Windows release builds link the C runtime statically, set in
`.cargo/config.toml`. Verify it:

```bash
pwsh scripts/check-static.ps1
```

## Interoperability

A torrent only `bit-cli` can read is not a torrent. `create`, `verify`, and
`seed` are checked against a second, unrelated implementation on every run of:

```bash
pwsh scripts/interop-roundtrip.ps1
```

It builds a multi-file payload, creates a `.torrent`, verifies it, seeds it,
and then downloads it with `aria2c`. It passes only when `aria2c`'s output is
byte-identical to the input. Three cases run: a plain v1 torrent, the same with
`--private`, and `--web-seed` with no peer at all, where `aria2c` has to
resolve the `url-list` and fetch over HTTP alone.

Nothing reaches the network. The tracker and the web seed are two fixtures in
this repository, both bound to `127.0.0.1`:

```bash
cargo run -p bit-cli-core --example loopback-tracker
cargo run -p bit-cli-core --example loopback-fileserver -- --root .
```

Each prints its URL on the first line of stdout and logs every request to
stderr. The last recorded run is in `TODO/create-seed.md` under T-084.

## Licence and attribution

`bit-cli` is MIT. See `LICENSE`.

It started as a fork of [`kist`](https://github.com/QaidVoid/kist), which is
dual licensed MIT OR Apache-2.0, and its copyright notice is kept in `LICENSE`.
It builds on [`librqbit`](https://github.com/ikatson/rqbit), which is
Apache-2.0. Torrent creation, linting, and the environment-injection pattern
that makes the whole binary drivable from a test are adapted from
[`intermodal`](https://github.com/casey/intermodal), which is CC0-1.0.

`THIRD_PARTY.md` carries the full licence text for every dependency and is
generated from `Cargo.lock` so it cannot drift.
