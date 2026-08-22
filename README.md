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

The minimum supported Rust version is **1.88**. That is not a preference: it
is the highest `rust-version` in the resolved dependency graph, which
`cargo metadata` will tell you.

```bash
cargo metadata --format-version 1 --all-features
```

CI pins exactly that toolchain in its `MSRV` job, and a test fails if the
number in `Cargo.toml`, the number in the workflow, and the number in this
paragraph stop agreeing.

`.github/workflows/release.yml` builds `x86_64-linux`, `aarch64-linux`, and
`x86_64-windows` on a `v*` tag, each with a BLAKE3 checksum and a build
provenance attestation. When a release exists, verify it with:

```bash
b3sum --check bit-cli-x86_64-unknown-linux-musl.tar.xz.b3sum
```

Every published binary is self-contained. The Windows one is statically linked
against the C runtime, so it runs without a Visual C++ redistributable; the two
musl ones carry no interpreter and need no shared object at all.

```bash
pwsh scripts/check-static.ps1 -Path target/release/bit-cli
```

That reads the binary rather than trusting the build: a PE has to import no
`VCRUNTIME`, `MSVCP`, `UCRT`, or `api-ms-win-crt-*`, and an ELF has to carry no
`PT_INTERP` and no `DT_NEEDED`. It picks the check from the file's own magic
bytes, so a cross-built artifact is checked the same way wherever the checking
happens, and CI runs it on all three release targets.

## The addressing model

A web seed is normally one flat thing: a URL that serves the whole torrent.
Here a binding is a triple.

**Source** is where bytes come from: an HTTP(S) URL with its own headers, auth,
user agent, timeouts, concurrency, and rate limit, or a `file:` URL naming
bytes already on the disk.

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

### A local path as a source

A source URL may be `file:`. The bytes for a torrent are often already on the
disk under a different name, in a different directory, or inside a finished
copy of a different torrent that happens to hold the same file. Naming that
path is how they get reused instead of fetched again.

```bash
bit-cli download release.torrent \
  --web-seed-for 'file:0=file:///srv/archive/a3f1-blob.dat' \
  --web-seed-mode exact
```

Everything else about a source still applies: scope, composition, chunk size,
rate limit, retries, per-source accounting, and the same loopback bridge. In
particular the source is not trusted. `--web-seed-verify piece` is on by
default, so a local file of exactly the right length holding the wrong bytes is
refused with the path and the piece named, the same way a wrong mirror is.

`auto` composition works against a directory, so a tree you already have is one
flag:

```bash
bit-cli download album.torrent --web-seed file:///mnt/backup/
```

That resolves to `/mnt/backup/album/disc 1/a.flac` and so on, exactly as the
BEP 19 composition does over HTTP. `webseed list` shows the resolved paths
before anything is read.

A `..` in a resolved path is refused. `auto` and `prefix` composition append
the torrent's own `name` and `path` to the source URL, so the tail of it is
written by the `.torrent` rather than by you, and a hostile one naming
`../../../Windows/win.ini` would otherwise make a source rooted at one
directory read out of another.

`file:` is not in BEP 17 or BEP 19 and is never offered to a swarm. It is a
source for one invocation, like every other source here.

```bash
pwsh scripts/check-local-source.ps1
```

That drives eight cases with no server running and no port bound, including the
one this exists for: three torrents with three info hashes and three piece
lengths (2 MiB, 1 MiB, 512 KiB) sharing one 64 MiB file. The file is fetched
once and lands in three output directories with one distinct hash between them.

### Several torrents that hold the same file

A binding normally applies to every torrent in the invocation. When the same
file sits at a different index in each, say which one you mean by prefixing the
selector with that torrent's info hash:

```bash
bit-cli download c.torrent a.torrent b.torrent --dir out -j 1 \
  --web-seed-mode exact \
  --web-seed-for 'e608e60a…:file:0=https://cdn.example.com/blob' \
  --web-seed-for '00c47ee9…:file:0=file:///out/payload_c/a/b/c/file.blob' \
  --web-seed-for '17eb3674…:file:1=file:///out/payload_c/a/b/c/file.blob'
```

Torrent C fetches the file from the CDN. A and B read the copy C wrote. One
invocation, one trip to the CDN, three output directories, and the payload
hashes equal in all three. `-j 1` is what makes that safe: sources start in the
order they were given, so C has finished before A looks for its file.

Exactly forty hexadecimal characters followed by a colon is read as an info
hash. A hash naming no torrent in the run is a usage error, not a binding that
quietly does nothing. The binding table takes the same thing as a `torrent`
field on a `[[source]]`.

Nothing is trusted here. Every piece a `file:` source serves is hash-checked
against the torrent that asked for it, so a wrong binding costs a failed source
rather than a corrupt payload.

### The same thing with nothing written by you

Those three bindings are what the run can work out for itself, and it does:

```bash
bit-cli download c.torrent a.torrent b.torrent --dir out -j 1
```

Before the session starts, every pair of torrents in the invocation is compared
by the piece hashes covering each file. Where the hashes prove two files are
the same bytes, the later torrent gets a `file:` source pointing at the copy the
earlier one wrote, as soon as the earlier one has finished. No path, no info
hash, no flag.

```
torrent   finished over http from disk resumed  shared proven    hash
payload_c     True 20.00 MiB 20.00 MiB 0.00 B        0 0.00 B    42ee6db050db50ce
payload_a     True 0.00 B    16.00 MiB 3.00 MiB      1 16.00 MiB 42ee6db050db50ce
payload_b     True 0.00 B    16.00 MiB 3.00 MiB      1 16.00 MiB 42ee6db050db50ce
```

```bash
pwsh scripts/check-shared-files.ps1
```

That is the run above, measured: three info hashes, the shared file at a
different path and index in each, 16 MiB fetched once over HTTP and read off
the disk twice, one distinct hash across three output directories.

`--json` reports it under `shared`, per torrent, naming the file, the torrent
it came from, and how much of it the piece hashes proved:

```json
"shared": [{
  "index": 0,
  "path": "deep/nested/dirs/file.blob",
  "from_info_hash": "a0f16220418c110ee3b5dba0a689c2c1b4791ca5",
  "from_path": "out/payload_c/a/b/c/file.blob",
  "pieces_compared": 16,
  "bytes_proven": { "bytes": 16777216, "human": "16.00 MiB" }
}]
```

Three things bound it. Only a piece-hash proof counts, never a matching length.
Only a torrent that has already finished donates, so `-j 1` is what makes the
order true and above it nothing is donated. And the source is checked per piece
on the way in like any other, so a proof that was somehow wrong costs a retry
rather than a payload. `--no-share-files` turns the whole thing off.

### Which files two torrents actually share

```bash
bit-cli files a.torrent --against b.torrent --against c.torrent
```

```
INDEX  EVIDENCE  PROVEN  OTHER       OTHER PATH
0      length    -       c2806b5a:1  media/file.blob
0      length    -       31084dc6:0  a/b/c/file.blob
1      length    -       31084dc6:1  a/extra.bin
2      length    -       c2806b5a:2  notes/changelog.txt
```

`piece-hashes` means the pieces line up and their hashes agree, which proves
the bytes equal. `length` means the sizes match and nothing else could be
checked, which proves nothing: two of the four rows above are files that have
the same size and different contents.

A `.torrent` hashes fixed-size pieces of the whole payload, not files, so two
files can be compared by hash only where the pieces cover the same bytes of
each. That needs the same piece length and the same offset modulo it. The three
torrents above have three piece lengths, so nothing among them is provable.
Against a pair that lines up, the same command proves the whole file:

```
INDEX  EVIDENCE      PROVEN     OTHER       OTHER PATH
0      piece-hashes  64.00 MiB  c3dabcae:0  file.blob
```

### Which failures are worth retrying

Whether an HTTP status is worth retrying is a property of the server, not of
the code. `bit-cli` treats 401, 403, 404, 410 and 416 as permanent and retries
the rest. Two flags move a code across that line, per source:

```bash
bit-cli download release.torrent \
  --web-seed-for 'file:0=https://cdn.example.com/blobs/a3f1/payload.iso' \
  --web-seed-retry-status 403
```

**A permanent status on one file does not retire the source.** The file's
pieces are dropped from what that source announces and it goes on serving the
rest, so a mirror holding eleven files of twelve stays a mirror for eleven of
them. A source with nothing left is retired, and the reason says it ran out
rather than naming one file. `--json` carries `gone_files` and
`pieces_dropped` per source, both omitted when nothing was lost.

A source addressed by piece rather than by file, which is BEP 17, has no
per-file request to attribute a failure to, so a permanent status retires it
whole. See `TODO/webseed.md` under T-005.

A CDN that signs its URLs answers 403 when a signature expires, and the next
request to the stable URL is redirected to a fresh one and succeeds. Without
the flag the first expiry ends the source. With it the run rides them out:

```bash
pwsh scripts/check-signed-source.ps1
```

That drives nine cases against a loopback server that signs, redirects,
expires, and falls over. The pair that matters here is the same server and the
same signature window run twice, differing only in the flag. In the recorded
run, 22 signatures expired over 64 MiB: without the flag the run downloaded
nothing and exited 1, with it the payload completed byte for byte. The report
is `bench/signed-source-20260820T132602637Z.json`. The count varies with
timing; whether the run completes does not.

`--web-seed-fatal-status` is the other direction, same spelling: a code it
names is treated as permanent even though the built-in classification would
retry it, so it narrows the source or, on a request that names no file, retires
it. Both take codes and inclusive ranges (`403`, `403,429`, `500-599`). A code
in both lists is a usage error, because there is no defensible answer.

The retries are reported per source, in the text output and in `--json`:

```
source               http://127.0.0.1:57581/cdn/a3f1b2c4-signed-blob.dat
  scope              file:0
  state              active
  served             64.00 MiB
  retries            10 (10 on 403)
```

What bounds a retried source is `--web-seed-retries`, the attempts one request
gets, and `--web-seed-max-errors`, the consecutive failed requests a source
gets before it is out. A request that fails transiently after spending its
retries drops the connection and reconnects, so a mirror that restarts
mid-download is not lost. At the defaults that is four attempts per request and
five requests: measured against a mirror answering 503 forever, the source is
retired and the run exits 1 after 33.4 seconds.

### Giving a mirror another chance

A source that spends that budget is out for the rest of the run.
`--web-seed-cooldown` puts it back to work instead:

```bash
bit-cli download release.torrent \
  --web-seed https://mirror-a.example.com/pub/ \
  --web-seed-cooldown 30s --timeout 10m
```

The source sleeps for that long, then reconnects with the error run cleared. A
mirror that is down for five minutes is usable again at minute six instead of
lost at second seventeen.

It is zero by default, which means the source does not come back. That is what
keeps an unattended run against one dead mirror failing in half a minute rather
than sitting on a timer, and it is why the flag is opt-in: a caller who wants
patience says how much.

While it sleeps the source reports `"state": "cooling"` rather than `failed`,
with `cooldown_until` and `cooldown_remaining_ms` beside it, and `cooldowns`
counts how many times it has been out. A cooling source is not a dead one, so
`--web-seed-require` and the "every source is dead" stop condition keep waiting
for it. Bound that with `--timeout` or `--stop-timeout`.

Measured, one mirror down for twenty seconds and two runs differing only in the
cooldown:

| cooldown | exit | downloaded | state | cooldowns |
| --- | --- | --- | --- | --- |
| 5s | 0 | 64.00 MiB | active | 4 |
| 300s | 9 | 3.00 MiB | cooling | 1 |

Both were given `--timeout 60s`. The first woke into a mirror that was still
down twice, then into one that was back, and finished in 23.5 seconds. The
second was still asleep with 241.1 seconds left when the deadline fired.

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
bit-cli bench <SUBCOMMAND>      leech | seed | webseed | disk | swarm | probe. Measure and report
bit-cli config show             Print the resolved configuration with the origin of every value
bit-cli completions <SHELL>     bash | zsh | fish | powershell | elvish | nushell
bit-cli man                     Generate a man page
bit-cli version                 Version, build metadata, features, protocol support
```

`bit-cli <SOURCE>` with no subcommand is `bit-cli download <SOURCE>`.

Sources accepted: a path to a `.torrent`, an HTTP(S) URL to one, a magnet URI,
a bare info hash, a local Metalink (`.meta4` or `.metalink`), and `-` for
stdin.

Every command runs in the foreground, does its work, and exits. There is no
daemon and no stored session.

## Loading a target

`bench swarm` is the one subcommand that puts load on a machine other than
this one, so it takes a peer address rather than a torrent, and that address is
the only thing it ever contacts. It announces to no tracker, uses no DHT, and
reads no peer list.

```bash
bit-cli bench swarm 10.0.0.5:51413 --for album.torrent --peers 16 --disk-budget 2GiB
```

`--for` names a torrent the target already serves. The synthetic peers
handshake for it, declare interest, request blocks, and check every completed
piece against the torrent's own hashes, so the report measures the target's
serving path and would notice it serving wrong bytes.

```bash
bit-cli bench swarm 10.0.0.5:51413 --peers 100 --torrents 4 --disk-budget 2GiB
```

Without `--for`, four info hashes are generated and the target has none of
them. Nothing can be served, which is the point: what is measured is the accept
and handshake path. How many connections the target takes, how fast it answers
a handshake, and whether its listener survives.

The `.torrent` files for the generated info hashes are written to the scratch
directory, so a run is reproducible and the operator can add one to a target
and come back with `--for`.

Two limits are worth knowing before reading a report. `--disk-budget` bounds
the piece bytes a peer keeps, and a held piece is written at its own offset, so
the file on disk can be larger than the budget. And a synthetic peer keeps what
it verified without serving it, so this is a hundred leeches rather than a
swarm: a target that ranks peers by what they have uploaded sees no difference.
Both are open under [T-092](TODO/bench.md).

## Metalink

A Metalink carries a `.torrent`, a list of HTTP mirrors for the same bytes, and
a checksum over the whole file, in one document. Both spellings are read: RFC
5854 `.meta4` and the older Metalink 3 `.metalink`.

```bash
bit-cli download release.meta4
```

That fetches the `.torrent` the document's `<metaurl>` names, registers every
`<url>` as a web seed source, downloads, and checks the payload against the
document's own checksum.

**The two documents are checked against each other, and the report says which
one is wrong.** A Metalink and a `.torrent` describe the same payload
independently. The declared lengths are compared before a byte moves. The
digest is then checked against a payload the session has already verified piece
by piece against the torrent's own hashes, so a digest that disagrees is
evidence about the Metalink:

```
the metalink's sha256 checksum does not match the payload: it says 0000...,
the bytes hash to ad33.... The payload passed the torrent's own piece hashes,
so the metalink is the document that disagrees.
```

Either disagreement exits 7. `--json` keeps them apart under
`torrents[].metalink`: `agreement.size_agrees` and `checksum.matched`.

`--dry-run` reads the document and touches nothing, which is the cheapest way
to check that a `.meta4` says what its author meant:

```bash
bit-cli --json download release.meta4 --dry-run
```

Worth knowing before you reach for this: **a Metalink generated by MirrorBrain
usually has no torrent in it at all.** The instance has to be configured for
one, and none reachable in August 2026 is, including
`download.documentfoundation.org` and `download.opensuse.org`. Such a document
is a mirror list, and `bit-cli download` says so and names the mirror count
rather than failing obscurely. `pwsh scripts/check-metalink-real.ps1` is the
measurement.

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

## Measuring a seeder

`bench seed` serves a payload and reports what leaves, per peer. It is the same
report `bench leech` writes with every counter facing the other way: bytes sent
rather than received, and positioned reads rather than writes, because a
seeder's storage cost is reading the payload back.

```bash
bit-cli bench seed album.torrent --data ./payload \
  --port 51413 --duration 120s --exit-when-idle 5s \
  --include-hash-check --format text
```

```
Summary
  measured over        35s
  bytes                737.94 MiB
  sustained            20.89 MiB/s
  peak                 24.15 MiB/s
  peak peers           3
  verification         256 pieces, 1.48 GiB/s in 169ms
  disk read            772.83 MiB in 878ms over 49152 reads
  disk write           0 B in 0ms over 0 writes

Peers
  peer                 127.0.0.1:60374
    sent               245.84 MiB at 6.96 MiB/s
```

The rows are peers, not sources: a seeder serving one peer well and another
badly looks the same in the total and different here.

`disk read` against `bytes` is the read amplification. 772.83 MiB read to send
737.94 MiB, with three peers pulling the same payload at once, is 1.047: every
byte was read about once and nothing is re-reading a piece for the second peer.

`--include-hash-check` puts the check on add into the report. A seeder reads and
hashes the whole payload before it serves a byte, and that read is normally not
part of what is being measured, so it is reported separately rather than folded
into the rate.

`--exit-when-idle` stops the run once no peer has been connected for that long.
Without it the seeder waits out `--duration` with nobody connected and the
sustained rate is diluted by the idle tail.

```bash
pwsh scripts/bench-seed.ps1 -PayloadSize 256MiB -Leechers 3 -Rate 8MiB/s
```

That drives one seeder and N leechers on loopback and writes both reports to
`bench/`. The leechers are rate capped, because an uncapped loopback transfer
finishes inside one metrics interval. So the default run measures whether the
seeder keeps up with N capped leechers rather than how fast it can go: the
sustained rate is bounded by `-Leechers` times `-Rate`. Pass `-Rate 0` with a
larger payload for a capacity number.

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

## Asking what something is

```bash
bit-cli bench probe 127.0.0.1:51413 --for album.torrent
bit-cli bench probe https://mirror.example.com/pub/album/disc%201/a.flac
```

The question before "how fast": is it there, and what does it speak. One
exchange, no payload, and the report carries the same environment every other
`bench` report does.

A peer address gets a BitTorrent handshake and then a short listen:

```
Probe
  target               127.0.0.1:51999
  kind                 peer
  reachable            yes
  connect              1ms
  peer id              -rQ9000-1%ba%01%06%ad0%b4xM%f5%d0%7f
  client               rqbit 9000
  reserved             0000000000100000
  extensions           extension-protocol
  info hash            echoed
  says it is           bit-cli 0.1.0
  extension messages   ut_metadata, ut_pex
  messages             extended, bitfield, unchoke
  pieces advertised    10
```

An HTTP endpoint gets one ranged `GET` for a single byte, with the redirect
chain hop by hop and the TLS version and cipher when the scheme is `https`.

`--for` names the torrent a peer is asked about, because a handshake names a
torrent and a peer is entitled to hang up on one it does not have. Without it
the report says the handshake carried a zero info hash. An unreachable target
exits 6.

## Measuring the disk on its own

`bench disk` writes a payload through the same storage a download writes
through, from N threads, with no session and no network. A download has the
network, the session, the hash, and the disk running at once and cannot say
which of them a slow run was waiting for; this takes the other three away.

```bash
bit-cli bench disk --payload-size 1GiB --concurrency-sweep 1,2,4,8 --format text
```

```
Writers
  THREADS  LAYOUT   FILES  RATE           WALL      FLUSH     WRITE TOTAL  MEAN WRITE  OVERLAP
  1        shared   1      2.27 GiB/s     440ms     821ms     423ms        6us         0.96
  2        shared   1      1.57 GiB/s     635ms     412ms     1s           18us        1.93
  4        shared   1      1.65 GiB/s     606ms     915ms     2s           34us        3.73
  8        shared   1      1.46 GiB/s     685ms     1s        4s           75us        7.22
```

`--layout` decides how the same bytes are spread, and comparing the three is
the measurement:

| Layout | Files | Handles | What it is |
| --- | --- | --- | --- |
| `shared` | 1 | 1 | Every thread interleaves blocks into one file. What a torrent with one payload file and several peers does. |
| `handles` | 1 | N | The same file at the same offsets, one handle per thread. |
| `split` | N | N | One file per thread. |

`OVERLAP` is the summed write time over the wall clock: the thread count when
nothing serialises, and 1.00 when everything does. `FLUSH` is what the write
phase left in the page cache, drained after the clock stops so one step does
not hand its cost to the next.

Every step reads the payload back and checks that each block is the block that
was written to it. A step that reads back something else exits 7, because that
is a correctness failure and not a slow one. Pass `--no-verify` to skip it.

```bash
pwsh scripts/check-disk-contention.ps1
```

That runs the sweep across all three layouts and a range of block sizes,
alternating the order so no layout always gets the disk in the same state, and
writes the medians and a verdict to `bench/disk-contention-<timestamp>.json`.
What it found on NTFS is in `TODO/disk-io.md` under T-017: writes to one file
serialise whatever handle they arrive on, and the serialisation is charged per
write operation rather than per byte.

## Several torrents at once

`download` takes any number of sources and `-j` says how many run at a time.

```bash
bit-cli download a.torrent b.torrent c.torrent d.torrent -j 4 --dir ./out
```

```bash
pwsh scripts/check-multi-torrent.ps1 -Torrents 4 -PayloadSize 256MiB -Runs 3
```

```
ceiling:  808.84 MiB/s through bit-cli's own HTTP path, no bridge, no hashing, no disk

mode    wall  bytes      rate         of ceiling peak RSS   CPU ms handles
one     1.46s 256.00 MiB 175.95 MiB/s 21.75%     43.61 MiB    2124     220
serial  6.24s 1.00 GiB   164.02 MiB/s 20.28%     44.48 MiB    8605     228
j1      6.18s 1.00 GiB   165.78 MiB/s 20.50%     48.49 MiB    8468     227
j2      3.01s 1.00 GiB   340.20 MiB/s 42.06%     74.09 MiB    9061     242
j4      1.76s 1.00 GiB   580.17 MiB/s 71.73%     114.24 MiB  10656     264
control 2.97s 1.00 GiB   344.32 MiB/s 42.57%     107.59 MiB  15108     289
```

`serial` is the same four torrents as four separate invocations, one after
another. `control` puts as many connections on one torrent at a time as `-j 4`
has in flight across four, which is what says the flag buys concurrency rather
than connections: `-j 4` reaches 580 MiB/s where the same sixteen connections
on one torrent reach 344.

`ceiling` is what the same source serves through `bit-cli`'s own HTTP path with
no bridge, no hashing, and no disk. Every mode reads off that one server, so a
mode approaching it is describing the server rather than the client.

Concurrency costs about 22 MiB of peak RSS and twelve handles per torrent in
flight, and no extra CPU for the same bytes. The full write-up is in
`TODO/performance.md` under T-030.

## Seeding for days

A `seed` run with `--seed-time 7d` is a long-lived process, and one thing in it
is not bounded. A peer that connects and closes before it sends a handshake
strands a socket in `CLOSE_WAIT` about half the time. Time does not release it
and ordinary traffic does, so a busy seeder clears what a burst left and an
idle one keeps it. Measured: 4000 such connections stranded 2053 sockets, and
100 ordinary connections then took that to 96.

```bash
pwsh scripts/check-close-wait.ps1
```

That is upstream and not fixed here. What `bit-cli` carries is a backstop:

```bash
bit-cli seed release.torrent --seed-time 7d --max-handles 4096
```

`--max-handles` is sampled once per `--report-interval` against the whole
process. Over it, the run stops with `"stopped": "handle_ceiling"` and exit 16,
which a supervisor restarts. It is off by default, because the right number
depends on the deployment; read `cost` in a healthy run's report for a
baseline. The numbers, the reproduction, and what closing it upstream would
take are in `TODO/peers.md` under T-020.

Memory has the same shape and the same backstop:

```bash
bit-cli seed release.torrent --seed-time 7d --max-rss 512MiB
```

A seeder under load grows about 0.8 MiB an hour, and most of that is one thing:
`librqbit` records a peer for every completed handshake and never reclaims the
row. Measured at **2,891 bytes a row over 2,000 rows**, retained after a minute
of no traffic, which at the soak's completion rate is 0.63 MiB of the 0.804.
Nothing here frees a row. `--max-rss` stops the run with
`"stopped": "rss_ceiling"` and exit 16 instead. A seeder with nothing connected
sits near 12 MiB, so pick a number from `cost` in a healthy report rather than
from this paragraph.

```bash
pwsh scripts/check-peer-rows.ps1
```

The write-up is in `TODO/memory.md` under T-040.

One thing that was in reach is fixed. `librqbit`'s accept loop panics when its
pending handshake-check set fills and one of those checks fails, and the panic
kills the listener while the process keeps running and keeps reporting itself
as seeding. Measured, 3000 connections that closed before handshaking did it in
79 seconds. `bit-cli` removes the branch that carries it, and the same flood
now finishes in 8.8 seconds with the listener alive.

## When the port is open and nobody is answered

The stranded sockets are the visible half. The other half is worse: the same
accept loop clears **one** queued handshake check per connection it accepts, so
a run of peers that close before they handshake leaves a backlog, and every
peer that arrives afterwards waits behind it. Measured: 20 such connections
were enough, and the seeder then answered nobody for as long as it was left
alone. Thirteen more connections cleared it; time cleared nothing.

Nothing a supervisor normally watches sees that. The process is alive, the port
accepts, the log is silent, and the ratio in the report is history. So `seed`
can watch its own listener from the outside of the socket:

```bash
bit-cli seed release.torrent --seed-time 7d --listener-check 60s
```

Each check dials this run's own listen port over loopback and completes a real
handshake for a torrent it is serving. Three failures in a row stop the run
with `"stopped": "listener_unhealthy"` and exit 17. Three is derived rather
than picked: one failure means a backlog a real peer would have cleared by
arriving, and three means the backlog outlived three connections, so the next
three peers get nothing either.

The check is off by default and it is not free. A completed handshake is a peer
as far as the session is concerned, so each check leaves one peer row that
`librqbit` keeps and never reclaims: 24 checks, 24 rows, measured. Those rows
are dropped from `peer_detail` and from the report, by the loopback port the
check dialled from, the same way a web seed bridge's connection is told from a
swarm member. An unknown info hash would leave no row at all and is the wrong
measurement: it resolves to an error inside the session, which **adds** an
entry to the backlog it is measuring.

```bash
pwsh scripts/check-listener.ps1
```

## Downloading through an outage

A download whose peers all go away recovers when they come back, but not
immediately. A dropped peer is retried at about 10 seconds, then 70, then 430,
a factor of six each time, so an outage that ends between two attempts waits
for the next one however long the network has been back.

That matters for `--stop-timeout`, which is how long with no progress a run
waits before giving up with exit 9 and `"stopped": "stalled"`. Set shorter than
the next retry, it turns a recoverable outage into a failure. Measured: a 40
second outage is caught by the 70 second attempt and the download completes
byte for byte; a 120 second outage is not, and a run given 180 seconds of
patience still exits 9 because the next attempt was not due for another four
minutes.

```bash
pwsh scripts/check-peer-recovery.ps1
```

So pick `--stop-timeout` deliberately. For an unattended run that a supervisor
retries, short is right: fail in seconds and start again. For one that has to
finish on its own, leave it off or set it past ten minutes. The numbers are in
`TODO/peers.md` under T-021.

`--redial-after` stops the waiting instead of budgeting for it:

```bash
bit-cli download release.torrent \
  --peer 203.0.113.9:51413 \
  --stop-timeout 300s --redial-after 30s
```

After that long with no progress, the torrent is paused and started again. That
throws away every peer connection and the backoff counters behind them, then
dials `--peer` and the trackers from scratch. Piece state is kept and nothing is
re-hashed, so the cost is the live connections and not the disk.

Measured on the same 120 second outage: without it the run exits 9 with 17.00
MiB of 128 after 300 seconds of patience, and with it the run re-dials four
times and completes byte for byte, finishing 55.6 seconds after the peer came
back.

```bash
pwsh scripts/check-peer-recovery.ps1 -OutageSeconds 120 -StopTimeout 60 -PatientTimeout 300
```

It is off by default, and it should stay off for a healthy swarm: the trigger is
no progress at all, and a swarm where every peer is choking is exactly the case
where dropping every connection every thirty seconds can make things worse. Set
`--max-redials` to cap how many times it fires, ten by default. Each one is in
the report as `redials[]` and on the event stream as `peer_redial`, with how
long the run had been stalled and how many live connections it cost.

## Reading a download as it arrives

```bash
bit-cli download film.torrent --piece-selector sequential --web-seed-connections 1
```

Pieces arrive front to back. Measured over ten runs on a 48 piece torrent, that
is **zero out of ten runs with any piece arriving before one already reported**,
against one such piece in every run of the default. It costs nothing at one
connection.

The default is not disordered. It asks for the first piece of each file, then
the last, then the middle in ascending order, so it is almost front to back
already and its one break is the tail arriving early. That is why the flag is
worth having and why it is not the default: `sequential` removes that break,
and above one connection it costs about seven percent of the throughput,
because every connection is pointed at the same part of the file.

Above one connection the order is not exact and cannot be. A selector decides
which piece is asked for next; it cannot decide which of four transfers already
in flight finishes first. Run the measurement yourself:

```bash
pwsh scripts/check-piece-order.ps1 -Runs 10 -Connections 1,2,4
```

`in-order` is the same thing spelled the way `aria2` spells it.

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

## Asking a tracker

```bash
bit-cli trackers album.torrent --json
```

Announces to every tracker in the torrent and reports what each one said: its
tier, its protocol, its interval, its seeder and leecher counts, the peers it
returned, and its failure reason when it has one. `--scrape` asks for the
counts without announcing.

The announce is a real one, so the command binds the port it announces for as
long as the announce lasts and then withdraws the record with a second
announce carrying `event=stopped`. A diagnostic that registers a peer nobody
can dial, and leaves it registered for the tracker's interval, is worse than
no answer. `--port` chooses the port or the range, and `--no-withdraw` leaves
the record in place.

A `download` run announces the same three events a client should:
`started` when the torrent goes live, `completed` the moment it finishes, and
`stopped` when the run ends. The last two come from `bit-cli` rather than from
the session, carrying the session's own peer id and port so the tracker
updates one record, and `--json` reports them under `announced`.

### What a UDP tracker that does not answer costs

BEP 15 says to retry at `15 * 2^n` seconds for `n` from 0 to 8, which is nine
attempts and up to 62 minutes before giving up. **`bit-cli` does not do that,
on purpose.** A foreground diagnostic that can take an hour to say "this
tracker is down" has not answered the question the caller asked. What it does
instead is **three attempts inside `--tracker-timeout`**, one attempt being
`max(--tracker-timeout / 3, 1s)`.

The one second floor is why `--tracker-timeout 1s` and `--tracker-timeout 3s`
cost the same three seconds. Below three seconds the flag buys nothing.

The total is not one number, because a UDP announce is two exchanges, connect
then announce, and either can be the one that dies. Measured:

| what happens | attempts | at `--tracker-timeout 6s` |
| --- | --- | --- |
| nothing answers, so the announce is never sent | 3 | 6.06 s |
| connect answered at once, announce dead | 3 | 6.06 s |
| connect answered on its third attempt, announce dead | 5 | 10.10 s |

**Five attempts is the worst case there is**, so the budget for one UDP tracker
is `5 * max(--tracker-timeout / 3, 1s)`: **fifty seconds** at the default
`--tracker-timeout` of 30 seconds, and never under five. Six attempts cannot
happen, because a connect that is not answered by its third gives up and the
announce that would spend three more is never sent.

Every tracker is asked at once rather than tier by tier, so that budget is per
tracker and not per torrent: a torrent with twelve dead UDP trackers still
answers in fifty seconds.

```bash
pwsh scripts/check-udp-retry.ps1
```

## Sampling a swarm

```bash
bit-cli peers album.torrent --duration 30s --sort speed:desc --json
```

Joins the swarm, watches for `--duration` or until `--count` distinct peers
have been seen, and reports every peer with the address, the state, the
direction, the bytes each way, the pieces it verified, and its mean piece
time. The client string and the connection type come from the peer's extended
handshake, so they are there while it is connected and gone once it is not.

It joins as a real member, so payload arrives. That is what makes
`--sort speed` mean anything: the rows are bytes that actually came from each
peer. What arrives goes to a temporary directory that the process removes when
it exits, and nothing is written where you are standing. Bound it with
`--duration`, `--count`, or `--max-download-rate`.

`--peer HOST:PORT` dials a known member whether or not anything else answers,
and with `--no-tracker --no-dht --no-lsd` the sample is exactly the members
named on the command line:

```bash
bit-cli peers album.torrent --peer 127.0.0.1:51413 \
  --no-tracker --no-dht --no-lsd --duration 5s --json
```

Exit 6 when nobody was seen, which is a real answer rather than a failure to
produce one, and a script tells the two apart by the code.

## Machine output

Two rules, and neither bends.

**stdout carries data only.** JSON, NDJSON, or the requested plain values.
`bit-cli ... --json | jq` never sees a log line in the pipe.

**stderr carries logs, progress, warnings, and errors.**

```bash
bit-cli info album.torrent --json | jq -r .info_hash
```

`--jsonl` emits one event per line as things happen, each with a monotonic
`seq` and an ISO 8601 UTC millisecond timestamp. Every `--jsonl` run ends with
a `session_end` event carrying the exit code, so a consumer can tell "finished"
from "the pipe broke".

`docs/schema.md` lists every document `kind` and every event `type` with the
fields each one carries, and `bit-cli --schema-version` prints the version it
describes. That file is generated from what the program actually writes: a test
drives every command, flattens the JSON, and fails when a report carries a field
the document does not.

Nothing is TTY-gated. Terminal detection reaches exactly two decisions, colour
and progress rendering, and never decides what the program does, computes, or
reports. Anything you can read in the terminal is a field in `--json`.

### Keeping a log

```bash
bit-cli download release.torrent \
  --log-file /var/log/bit-cli.log --log-max-size 16MiB --log-max-files 5
```

The file rotates at `--log-max-size` into `.1`, `.2`, and so on.
`--log-max-files` is the count in total, the live one included, so `5` leaves
`bit-cli.log` plus four rotated. `--log-max-size 0` never rotates.

It is a second destination, not a replacement: stderr still carries the logs,
so `bit-cli ... --json | jq` behaves the same either way. Redirect stderr if
you want only the file. The log file never carries colour escapes, whatever the
terminal is.

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

A long path is not one of the three. A payload whose deepest path plus the
output directory runs past the 260 characters the classic Windows API allows
lands as written and verifies from the same path, with nothing renamed. The
one limit that does apply is per component: a name over 255 bytes is truncated
to fit, keeping its extension, and reported like any other rename.

## Disk

A payload file is created when it is first written, not when the torrent is
added. Two things follow.

`--select-file 0` writes one file and leaves the rest off the disk, rather than
creating eleven empty ones beside the one you asked for.

With one exception, and it is the torrent's shape rather than a choice. A piece
is verified against its whole hash, so a piece straddling the boundary between
a file you selected and one you did not cannot be proved without both halves.
Those bytes are fetched and written into the file they belong to, which leaves
a file you did not ask for holding a few hundred kilobytes of payload and
nothing else. It can even land at its full length, which is what makes it worth
saying rather than leaving to be discovered:

```bash
bit-cli download album.torrent --select-file 1 --json
```

reports every one of them under `torrents[].partial`, with how much of each is
real, how long it ends up on disk, and how long the torrent says it is, and
says the same on stderr. A torrent whose file boundaries fall on piece edges
has none.

`verify` takes the same selection, and needs it to give the right answer:

```bash
bit-cli verify album.torrent --data out/album --select-file 1 --json
```

Without it, every piece outside the selection is reported as a failure and the
command exits non-zero, which is true of the bytes and wrong about the run:
nothing ever asked to fetch them. With it they are listed under `not_selected`,
the counts are against what was asked for, and a selection that arrived intact
is complete. The boundary pieces themselves verify and a `bit-cli seed` over
that directory offers them, because their bytes really are all there.

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
| 16 | A resource ceiling was crossed |
| 17 | This run's own listener stopped answering |

Codes 11 through 17 exist so a script can tell "your mirrors are
misconfigured" from "the network is down" from "your server is slow" from "the
process is out of handles" from "the port is open and answers nobody".

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

[[source]]
url          = "https://signed.example.com/blob"
scope        = "file:1"
mode         = "exact"
retry_status = [403, 429]
fatal_status = ["500-599"]
```

```bash
bit-cli download release.torrent --web-seed-config web-seeds.toml
```

## Reading a torrent somebody else wrote

`bit-cli` reads a `.torrent` whose keys are not in the sorted order BEP 3
requires, and reads whitespace or NUL after the top-level dictionary, and
**says so** rather than accepting either silently: `bit-cli info` reports both,
in the text output and under `encoding` in `--json`.

Tolerance is safe here for one specific reason. The `info` dictionary's bytes
are kept exactly as they were read and spliced back verbatim on the way out, so
its keys are never re-sorted and the info hash cannot move. `bit-cli edit` on
such a torrent re-encodes every key **outside** `info` canonically, leaves
`info` untouched, and proves the hash did not change before it writes. A tool
that re-encoded `info` instead would publish a different torrent from the same
file, which is why the deviation is worth reporting even though it costs
nothing here.

What is still refused: duplicate keys, integers with a leading zero or `-0`,
non-string keys, lengths that run past the end, and any trailing byte that is
not whitespace or NUL. Those are ambiguities rather than untidiness, and the
error names the rule rather than only the symptom.

## Protocol support

Three statuses, and the difference between them matters. **Yes** means
`bit-cli`'s own code implements it and a test covers it; the symbol column
names where. **Inherited** means `librqbit` provides it, `bit-cli` reaches it
through the session, and `bit-cli` has no test of its own. **No** means it is
not there, and the entry that closes it is named.

| BEP | What | Status | Where |
| --- | --- | --- | --- |
| 3 | The BitTorrent protocol | inherited | the session; `tracker.rs:9` for the announce half |
| 5 | DHT | inherited | `--no-dht` reaches `enable_dht`, `swarm.rs:160` |
| 7 | IPv6 tracker extension | yes | `peers6` at `tracker.rs:493`, 18 bytes per entry |
| 9 | Metadata from peers | inherited | magnets resolve through the session |
| 10 | Extension protocol | yes | `webseed/bridge.rs:84` `MSGID_EXTENDED`, `:888` `extended_handshake`, `:102` `OUR_EXTENSIONS` |
| 11 | PEX | inherited | no `bit-cli` code; `--no-pex` warns that it cannot turn it off, [T-181](TODO/cli-surface.md) |
| 12 | Multitracker metadata | yes | `tracker.rs:115` tiers; `create`, `edit`, `trackers` |
| 14 | Local service discovery | inherited | `--no-lsd` reaches `enable_lsd`, `swarm.rs:161` |
| 15 | UDP tracker protocol | yes | `tracker.rs:25`, `:301`, `:643`. The retry ladder diverges on purpose: three attempts inside `--tracker-timeout` rather than `15 * 2^n`, [above](#what-a-udp-tracker-that-does-not-answer-costs) |
| 17 | HTTP seeding, Hoffman style | yes | `webseed/fetch.rs`; the style is keyed by the metainfo list a URL came from, and probed for a `--web-seed` given on the command line |
| 19 | HTTP seeding, GetRight style | yes | `webseed/composition.rs`, the headline feature |
| 20 | Peer id conventions | yes | `webseed/bridge.rs` handshake |
| 21 | Extension for partial seeds | yes | `webseed/bridge.rs:897` `upload_only` |
| 23 | Compact peer lists | yes | `tracker.rs:552` |
| 27 | Private torrents | yes | `torrent/metainfo.rs`, `create`, `edit` |
| 39 | Updating torrents via feed URL | yes | `create`, `edit` |
| 47 | Padding files | read only | parsed and skipped: `torrent/metainfo.rs:116`, `storage.rs:728`; `create` does not emit them ([T-081](TODO/create-seed.md)) |
| 48 | Tracker scrape | yes | `tracker.rs:427`, `:499`; BEP 48 URL convention only ([T-065](TODO/trackers.md)) |
| 53 | Magnet file selection, `so=` | yes | `torrent/magnet.rs:211` |
| 6 | Fast extension | no | [T-100](TODO/bep-coverage.md) |
| 16 | Superseeding | no | [T-082](TODO/create-seed.md). `--superseed` is accepted and warns |
| 29 | uTP | no | [T-101](TODO/bep-coverage.md). No flag enables it |
| 52 | BitTorrent v2 | no | [T-081](TODO/create-seed.md), [T-134](TODO/multi-source.md) |
| 54 | `lt_donthave` | no | [T-167](TODO/bep-coverage.md), blocked: `librqbit` 9.0.0 has no receive side |
| 55 | Holepunch | no | [T-102](TODO/bep-coverage.md) |
| MSE/PE | Peer encryption | no | [T-163](TODO/peers.md) |

`TODO/bep-coverage.md` tracks the gaps.

**uTP is not reachable.** `librqbit-utp` appears in `cargo tree` because
`librqbit` depends on it, not because `bit-cli` uses it: `ListenerOptions::mode`
is never set, so the session stays `TcpOnly`, and no flag changes that. Earlier
revisions of this table said "available, off by default", which read as a
capability a user could turn on. There is nothing to turn on. [T-101](TODO/bep-coverage.md)
carries the work.

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

## Working on this

`TODO/` is the authoritative record of what is done, what is not, and why.

- [TODO/PROGRESS.md](TODO/PROGRESS.md) is the session state: the measured
  baseline, what the last session did, and the work order under "Start here
  next session". It carries no history and is rewritten every session.
- [TODO/INDEX.md](TODO/INDEX.md) is every item on one line each, a counts table
  that is exact against the rows, and the argument that produced the last
  ordering.
- [TODO/RULES.md](TODO/RULES.md) is how the repository is worked on: the
  process rules, the testing rules, the settled decisions, and the git
  protocol.

**`scripts/git-sync.ps1` is the only sanctioned way to commit and push.** It
pins the identity, refuses a commit message carrying AI attribution, keeps
`reference/` out of `main`, runs the gates before the push, and mirrors the
research corpus to the `references` branch.

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -Message "Subject line" -Body "..."
```

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -Check
```

The research corpus is twenty-two upstream BitTorrent implementations, indexed
by `reference/RESEARCH.md`. It is gitignored on `main` and lives on the
`references` branch. On a fresh clone:

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -FetchReferences
```

## Licence and attribution

`bit-cli` is MIT. See `LICENSE`.

It started as a fork of [`kist`](https://github.com/QaidVoid/kist), which is
dual licensed MIT OR Apache-2.0, and its copyright notice is kept in `LICENSE`.
It builds on [`librqbit`](https://github.com/ikatson/rqbit), which is
Apache-2.0. Torrent creation, linting, and the environment-injection pattern
that makes the whole binary drivable from a test are adapted from
[`intermodal`](https://github.com/casey/intermodal), which is CC0-1.0.

`THIRD_PARTY.md` carries the full licence text for all 310 dependencies and is
generated from `Cargo.lock`:

```bash
cargo about generate --config about.toml --output-file THIRD_PARTY.md about.hbs
```

`deny.toml` allows permissive licences only. Everything else fails the build
rather than appearing in a generated file, which is checked both ways:

```bash
cargo deny check
pwsh scripts/check-licence-gate.ps1
```

The first says this tree is clean. The second builds a throwaway crate with
one `GPL-3.0-or-later` dependency and requires the same configuration to
refuse it, because a gate that has never rejected anything has not been
tested.
