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

From source. This is the only way today: no version has been tagged, so there
are no published binaries yet.

```bash
cargo install --path crates/bit-cli --locked
```

`.github/workflows/release.yml` builds `x86_64-linux`, `aarch64-linux`, and
`x86_64-windows` on a `v*` tag, each with a BLAKE3 checksum and a build
provenance attestation. When a release exists, verify it with:

```bash
b3sum --check bit-cli-x86_64-unknown-linux-musl.tar.xz.b3sum
```

The Windows binary is statically linked against the C runtime, so it runs
without a Visual C++ redistributable. `pwsh scripts/check-static.ps1` proves
that on any build.

## The addressing model

A web seed is normally one flat thing: a URL that serves the whole torrent.
Here a binding is a triple.

**Source** is where bytes come from: an HTTP(S) URL, with its own headers,
auth, user agent, timeouts, concurrency, and rate limit.

**Scope** is what part of the torrent that source may serve. A mirror holding
part of the payload is a first-class case, not an error.

**Composition** is how the request URL is built from the source URL and the
torrent's `name` and `path`.

The three are orthogonal. Any source can serve any scope under any
composition.

### One source, several connections

A source reaches the torrent session as a peer, and a peer's blocks are
written and hash-checked one at a time on that connection's own task. That
path is what bounds the transfer, so a source presented over one connection
runs at one path's speed however fast the mirror is.

`--web-seed-connections <N>` presents the source over N connections, which is
N of those paths. They share one HTTP client, one window cache, and one
concurrency budget divided between them, so the mirror sees the same number of
requests either way.

```bash
bit-cli download release.torrent \
  --web-seed https://mirror.example.com/pub/ --web-seed-connections 2
```

On loopback, two connections reach 1.92 times what one reaches, and the curve
is flat after that. Eight times the requests in flight on a single connection
reaches 0.81 times, so it is the connections and not the requests. The numbers,
the commands, and the control are in `TODO/webseed.md` under T-009, with the
report under `bench/`.

The default is one connection. Two is the measured knee on loopback and
loopback flatters the receive path, so raising the default waits on the same
measurement against a real mirror.

`--prefer-web-seed` is the same lever applied for a different reason. On a
hybrid run where peers and HTTP sources both hold a piece, it doubles each
source's connections, so HTTP is more often the side that answers first. On a
loopback swarm of one mirror and one peer, neither rate limited, it moves the
HTTP share of a 1 GiB payload from a mean of 46.72% to 62.60% across five
paired runs:

```bash
pwsh scripts/check-prefer.ps1 -PayloadSize 1GiB -Runs 5
```

It moves the odds, not the decision. `librqbit`'s piece picker is not reachable
from outside the crate, so a piece a peer happens to answer first still comes
from the peer. `TODO/webseed.md` under T-003 has the numbers and what closing
the gap would take.

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
  origin             command_line
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
bit-cli info <SOURCE>           Parse a torrent or magnet and print its metadata
bit-cli files <SOURCE>          List files with index, path, and size
bit-cli peers <SOURCE>          Connect, sample the swarm, report peers, exit
bit-cli trackers <SOURCE>       Announce or scrape, report tier, interval, seeders, leechers
bit-cli webseed <SUBCOMMAND>    list | test | probe | fetch
bit-cli verify <SOURCE>         Hash-check existing data, per piece
bit-cli create <PATH>           Create a .torrent
bit-cli edit <TORRENT>          Rewrite metainfo fields, writing a new file
bit-cli magnet <SOURCE>         Convert a torrent to a magnet URI
bit-cli seed <SOURCE>           Seed existing data in the foreground
bit-cli bench <SUBCOMMAND>      leech | webseed. Measure a target and write a report
bit-cli config show             Print the resolved configuration with the origin of every value
bit-cli completions <SHELL>     bash | zsh | fish | powershell | elvish | nushell
bit-cli man                     Generate a man page
bit-cli version                 Version, build metadata, features, protocol support
```

`bit-cli <SOURCE>` with no subcommand is `bit-cli download <SOURCE>`.

Sources accepted: a path to a `.torrent`, an HTTP(S) URL to one, a magnet URI,
a bare info hash, and `-` for stdin.

Every command runs in the foreground, does its work, and exits. There is no
daemon and no stored session.

Metalink as a source, and three of the five `bench` subcommands, parse but are
not built yet. Each exits non-zero naming the `TODO/` entry that closes it,
rather than pretending to work:

```bash
bit-cli bench swarm album.torrent
```

```
error: `bit-cli bench swarm` is not implemented yet; see TODO/bench.md
```

## Measuring a mirror

`bench webseed` reads real payload out of each source's scope and drops it. It
measures the transport: latency percentiles, how throughput moves with
concurrency, and what fails and why. No piece is written and no hash is
checked.

```bash
bit-cli bench webseed album.torrent \
  --web-seed https://mirror.example.com/pub/ \
  --duration 30s --warmup 3s --concurrency-sweep 1,2,4,8,16 --format text
```

```
bench webseed

started                2026-08-19T23:13:33.253Z
finished               2026-08-19T23:13:43.264Z
elapsed                10s

Environment
  bit-cli              0.1.0 (x86_64-pc-windows-msvc, release)
  os                   Windows 10.0.26200
  cpu                  12th Gen Intel(R) Core(TM) i7-12700H (20 logical, x86_64)
  memory               63.63 GiB
  link                 Intel(R) Ethernet Connection (16) I219-LM at 1.00 Gbit/s
  cost                 peak RSS 40.13 MiB, CPU 29s, 219 handles

Summary
  measured over        8s
  sustained            2.98 GiB/s
  requests             24418 (0 failed)
  connect              p50 1ms  p90 1ms  p99 1ms  p99.9 1ms  max 1ms
  first byte           p50 1ms  p90 3ms  p99 18ms  p99.9 23ms  max 24ms
```

The report goes to stdout in `--format`, which defaults to `json`. Pass
`--report <PATH>` to write it to a file instead, and stdout carries the text
summary. `--format csv` writes the time series as one row per sample, which is
the part a plotting tool wants; it carries the series and nothing else, because
a report is nested and a table is not.

Two flags turn a measurement into a check a script can branch on:

```bash
bit-cli bench webseed album.torrent --web-seed $URL --fail-under 50MiB/s
bit-cli bench webseed album.torrent --web-seed $URL --baseline last-week.json
```

`--fail-under` exits 14 when sustained throughput falls below the rate.
`--baseline` prints a delta per metric with a sign and a percentage, and
refuses the comparison, with the reason named, when the two reports were taken
on different hardware. Every report carries the machine, the exact command
line, and what the process cost, because two numbers from two machines are not
comparable and nothing in the number itself says so.

## Measuring a download

`bench leech` downloads the target and reports what it cost. It is `download`
with the clock running, so it takes the same source, tracker, and web seed
flags, and it answers what a rate on its own cannot: whether the run was
waiting on the network, on the hash, or on the disk.

```bash
bit-cli bench leech album.torrent \
  --web-seed http://127.0.0.1:52466/ --web-seed-only \
  --dir ./out --port 0 --warmup 0s --metrics-interval 250ms --format text
```

```
Summary
  measured over        5s
  bytes                1.00 GiB
  sustained            185.64 MiB/s
  peak                 242.47 MiB/s
  requests             65536 (0 failed)
  peak peers           1
  verification         1024 pieces, 1.56 GiB/s in 641ms
  choke                0 choke, 0 unchoke, queue depth 128
  disk read            1.00 GiB in 136ms over 16384 reads
  disk write           1.00 GiB in 1s over 65536 writes
  pipeline             24 blocks in flight on average, 128 at peak, 16.00 KiB block, 2092us to answer
  window allows        956.02 MiB/s at that depth and that service time; 185.64 MiB/s was measured, 19.42% of it

Sources
  source               http://127.0.0.1:52466/
    served             1.00 GiB at 185.64 MiB/s over 65536 requests (0 failed)
```

Three of those lines are measurements nothing else in the process can take.
`verification` is the wall time of every piece read back and hashed, bracketed
in `bit-cli`'s own storage. `disk read` and `disk write` are the positioned
reads and writes underneath it. `pipeline` is the session's block request
window seen from the other end of the loopback bridge, with `window allows`
saying what that depth would sustain at the measured service time: close to
the sustained rate means the window is the limit, far above it means something
else is.

The same three appear per interval under `series[].costs` in the JSON and as
columns in `--format csv`, so the shape over time is visible and not just the
total.

`--fail-under` and `--baseline` work here exactly as they do on `bench
webseed`.

### What the whole path costs

`bench webseed` measures the HTTP fetch on its own. Two scripts measure what
the torrent machinery adds on top of it, and both write a committed report to
`bench/`.

`scripts/bench-webseed.ps1` takes the same payload from the same server four
ways in one session: `curl` on one connection, `curl` on N, `bit-cli bench
webseed`, and `bit-cli download --web-seed-only`. Four stages rather than two
because one ratio says "slower" without saying where.

```bash
pwsh scripts/bench-webseed.ps1 -PayloadSize 256MiB -Runs 5
```

```bash
pwsh scripts/bench-webseed.ps1 `
  -Mirror https://geo.mirror.pkgbuild.com/iso/2026.08.01/ `
  -TorrentUrl https://geo.mirror.pkgbuild.com/iso/2026.08.01/archlinux-2026.08.01-x86_64.iso.torrent
```

`scripts/bench-leech.ps1` then divides that gap. It runs `bench webseed` and
`bench leech` against the same payload, steps `--web-seed-connections`, runs a
control that puts the same total HTTP concurrency on a single connection so the
two cannot be confused, and compares against the same URL named N times so the
cost of not sharing a window cache is visible.

```bash
pwsh scripts/bench-leech.ps1 -PayloadSize 1GiB -Runs 5 -ConnectionSweep "1,2,4,8"
```

The results are in `TODO/webseed.md` under T-001 and `TODO/bench.md` under
T-090, with the committed reports under `bench/`. In one line: one source is
one peer, one peer is one serial receive path, and that path is what bounds
the download.

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

`seed` and `verify` carry the same array, because they serve and read the files
`download` wrote:

```bash
bit-cli seed hostile.torrent --data out --json   | jq '.renamed'
bit-cli verify hostile.torrent --data out --json | jq '.renamed'
```

## Disk

A payload file is created when it is first written, not when the torrent is
added. Two things follow.

`--select-file 0` writes one file and leaves the rest off the disk, rather than
creating eleven empty ones beside the one you asked for.

`--max-open-files` is a real cap. Files close on a least-recently-opened basis
when it is reached, so a torrent with twenty thousand files needs the cap in
descriptors and not twenty thousand:

```bash
bit-cli seed many.torrent --data . --max-open-files 64
```

```bash
pwsh scripts/check-handles.ps1
```

measures it: three seeds of a 300-file torrent at caps of 8, 64, and 128, with
the process handle count sampled while each runs. The steps in the cap and the
steps in the handle count match exactly.

`--file-allocation` picks how space is reserved, and the four methods do four
different things:

| Method | What happens |
| --- | --- |
| `none` | The length is set and nothing else. On NTFS that allocates; on ext4 it does not. |
| `sparse` | The file is marked sparse first, so the hole is explicit. The default. |
| `prealloc` | Zeroes are written across the file and flushed. Slow, and the space is certainly there. |
| `falloc` | The filesystem reserves the blocks without writing them. `posix_fallocate` on Linux. |

`falloc` on Windows needs `SeManageVolumePrivilege`, which an ordinary process
does not hold, so it falls back to `prealloc` and says so on stderr rather than
doing something other than what it was told.

```bash
pwsh scripts/check-allocation.ps1
```

measures all four against a real download, reading volume free space before the
payload arrives. On NTFS with a 512 MiB payload, `sparse` costs the volume
nothing and the other three cost it 512 MiB. Every method's output hashes equal
to the source.

## Paths, on the writing side

The rules above are the reading side. On the writing side `bit-cli create`
refuses to build such a torrent at all, through the `windows-path` and
`case-collision` lints, with `--allow <LINT>` to override either one. Those
lints only have anything to catch on a filesystem that can hold the input,
which Windows is not, so they are exercised on Linux and here:

```bash
cargo test -p bit-cli-core lint::
```

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
connections = 2

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
`seed` are checked against two other implementations, `aria2c` and `rqbit`:

```bash
cargo build --workspace --bins --examples
```

```bash
pwsh scripts/interop-roundtrip.ps1
pwsh scripts/interop-roundtrip.ps1 -Client rqbit
```

Each run builds a multi-file payload, creates a `.torrent`, verifies it, seeds
it, and then downloads it with the other client. It passes only when that
client's output is byte-identical to the input, and only when `bit-cli seed`'s
own accounting covers the payload. Three cases run: a plain v1 torrent, the
same with `--private`, and `--web-seed` with no peer at all, where the client
has to resolve the `url-list` and fetch over HTTP alone.

`rqbit` skips the third case, because it does not implement BEP 19. The skip is
named in the report rather than dropped. CI runs the `aria2c` matrix on Linux
and Windows.

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
