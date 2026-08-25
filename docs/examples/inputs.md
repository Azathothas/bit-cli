# What you can hand to bit-cli, and what happens to it

Most commands take one positional `SOURCE`. This page is what that word means,
which commands accept which form, and where the edges are. Every result below
was produced by running the command.

## The seven forms, and how each is recognised

Classification happens before anything is opened or fetched, by the shape of
the string. Nothing is probed to find out what it is.

| form | recognised by | example |
| --- | --- | --- |
| a local `.torrent` | anything left over after the rules below | `album.torrent` |
| an HTTP(S) URL | the `http://` or `https://` prefix | `https://host/a.torrent` |
| a magnet URI | the `magnet:?` prefix, case insensitive | `magnet:?xt=urn:btih:...` |
| a bare info hash | 40 hex characters, or 32 base32, and nothing else | `9e20e33071fae16f...` |
| a local Metalink | a `.meta4` or `.metalink` extension | `release.meta4` |
| a Metalink by URL | the same extension, on the URL's **path** | `https://host/r.meta4` |
| standard input | the single character `-` | `-` |

The order matters in two places. A bare info hash is tested before the path
rule, so a 40-character hex filename in the working directory is still read as
a hash. A Metalink URL is decided from the path only, so `?file=x.meta4` is a
query naming a file and does not make the URL a Metalink.

The rules are in
[`../../crates/bit-cli/src/source.rs`](../../crates/bit-cli/src/source.rs).

## A local torrent's filename does not matter

The extension is not consulted for a local file. What is read is the bencode
inside it.

```bash
bit-cli info payload.bin
```

```text
name                 payload
info hash            9e20e33071fae16fc950cd95e5fc6ec0059d9a63
size                 1.67 MiB
```

The same file with no extension at all reads the same. This is useful with a
downloaded file whose name the browser chose, and it is why a `.torrent`
extension is a convention here rather than a requirement.

The exception is the Metalink pair, because a Metalink is XML and a `.torrent`
is bencode, and the extension is what says which parser to use.

## Standard input

```bash
curl -sS https://host/album.torrent | bit-cli info -
```

The whole of stdin is read and parsed as one torrent. It is how a torrent that
never touches the disk gets inspected, and it works on every command that takes
a `SOURCE`.

## Which commands accept which form

The forms above are what the argument parser understands. What a command can
then do with one is narrower, and the difference is worth knowing before it
surprises you.

Every form reaches every command that reads a source. What differs is what
reading one costs: a file is read, a URL and a Metalink are fetched, and a
magnet or a bare info hash is resolved against the swarm it names.

| form | `info`, `files`, `tree`, `magnet`, `verify`, `webseed`, `bench webseed` | `download`, `seed` |
| --- | --- | --- |
| local `.torrent` | yes | yes |
| stdin | yes | yes |
| HTTP(S) URL | yes, one `GET` | yes |
| Metalink, local or by URL | yes, after fetching the torrent it names | yes |
| magnet or info hash | yes, after a swarm lookup | yes, after a swarm lookup |

`bit-cli trackers` is the one that does not resolve a magnet, and it is worth
its own line. An announce needs the info hash and the length, and a magnet or a
bare info hash carries the hash already, so it announces from that rather than
joining a swarm to learn something it has. What it does not have is a length,
and it says so rather than claiming zero:

```bash
bit-cli trackers https://host/album.torrent --json
```

Every cell was run. For the URL row `info`, `files`, `tree`, `magnet` and
`verify` had their `--json` output compared field for field against the same
torrent read off disk: everything matches but `generated_at`, which is two runs, and
`source_kind`, which differs because the source genuinely was a URL. That
comparison is a test, `read_only_commands_resolve_a_torrent_over_http_and_report_what_the_file_reports`,
so it holds on every build rather than on the day it was written.

`bit-cli peers` is not in either column either: it takes every form the
right-hand column does, because it starts an engine.

**A URL and a Metalink are fetched, not refused.** A `.torrent` at a URL is one
request:

```bash
bit-cli info https://host/album.torrent
```

A Metalink is two, because the document names its `.torrent` and that has to be
fetched as well. Both shapes work, the local `.meta4` and the one named by URL.

Two bounds apply to any of those fetches, and neither is configurable except
through `--timeout`:

- The deadline is `--timeout` when you set one and 30 seconds when you do not.
  A fetch that runs out of time exits **9** and names the deadline in
  milliseconds, rather than reporting a decoding failure.
- A `.torrent` body is capped at 16 MiB and a Metalink at 1 MiB, counted as the
  bytes arrive rather than after the whole body is in memory. Over the cap is
  exit 4: `answered with more than 1048576 bytes, which is larger than any
  document a source can be`.

A URL that answers with something that is not a torrent fails and says what
arrived, naming the declared content type when the server sent one:

```bash
bit-cli info https://host/downloads/
```

```text
error: https://host/downloads/: the server answered with text/html: not a
valid torrent: unexpected byte '<' at byte 0, expected a bencode value
```

## A magnet is resolved, and that is not a fetch

**A magnet and a bare info hash carry no piece hashes**, so there is nothing to
report until the metadata has been pulled from the swarm. Every command that
reads a source does that lookup:

```bash
bit-cli info "magnet:?xt=urn:btih:..." --json
```

It joins the swarm the source names, asks a peer for the `info` dictionary over
BEP 9, and reports the same document the `.torrent` would have produced. That
is a different operation from a `GET`, so it has flags of its own, under
**Resolving a magnet** in any of those commands' help:

| flag | what it does |
| --- | --- |
| `--peer <ADDR>` | ask this peer before any are discovered. Repeatable |
| `--no-dht` | do not use the DHT |
| `--no-lsd` | do not use local service discovery |
| `--no-tracker` | do not announce to the trackers the magnet names |

All four together leave a swarm of exactly the addresses on the command line,
which is what a private network wants and what every test here uses:

```bash
bit-cli info "magnet:?xt=urn:btih:..." --peer 127.0.0.1:6881 --no-dht --no-lsd --no-tracker
```

The deadline is `--timeout` where you set one and **60 seconds** where you do
not, which is longer than the 30 a document fetch gets because the work is
larger: find a peer, handshake it, then pull the metadata. Running out exits
**9** and names the deadline in milliseconds.

## Keeping what a resolution produced

Resolving the same magnet twice means finding peers twice. `bit-cli magnet
--output` writes what came back as a `.torrent`, so the second time is a file
read:

```bash
bit-cli magnet "magnet:?xt=urn:btih:..." --peer 127.0.0.1:6881 --no-dht --no-lsd --no-tracker --output album.torrent
```

`-` writes it to stdout, and `--force` overwrites a file that is already there.

**The info hash cannot move.** The `info` dictionary is spliced in as the bytes
that arrived rather than re-encoded, and the written file is decoded again and
its hash compared before anything reaches the disk. The trackers the magnet
named are in the file; a `ws=` web seed is carried across as `url-list` when
the magnet had one.

Without `--output`, `bit-cli magnet` on a magnet reads the URI and stops:
no swarm, no tracker, no DHT. `--output` is the only thing on that command that
needs the metadata behind the URI. In the other direction it turns a
`.torrent` into a URI with no network at all.

## What is not an input yet

**A web page.** A URL is assumed to name a `.torrent` itself. A page that
*links* to one is fetched and handed to the bencode parser, which fails and
names the content type it got. There is no HTML parsing anywhere in the tree.
That is [T-244](../../TODO/cli-surface.md), and the design there is static
extraction with a browser opt-in.

## Several sources in one invocation

`download` takes any number of them, of mixed forms:

```bash
bit-cli download a.torrent b.torrent https://host/c.torrent --dir out
```

`-j`, which is `--max-concurrent-downloads`, sets how many run at once.
`-j 1` runs them in the order given, which is what makes the
file-sharing-between-torrents case in [`../webseed.md`](../webseed.md) safe:
the torrent that fetches a file has finished before the torrent that reads it
from disk starts looking.

Every source in the run is compared with every other before the session starts,
by the piece hashes covering each file. Where the hashes prove two files are
the same bytes, the later torrent reads the copy the earlier one wrote instead
of fetching it again. Nothing is passed to make that happen. See
[`comparing-torrents.md`](comparing-torrents.md) for what that comparison can
and cannot prove.

## Checking what would happen without doing it

```bash
bit-cli download album.torrent --dry-run
```

Resolve, validate, report, write nothing. It prints the directory, the source,
the name, and the web seed and tracker counts.

Over a URL it says what it did not do, because a dry run does not fetch and the
torrent's own web seeds and trackers are therefore unknown:

```text
source               http://127.0.0.1:8099/tracked.torrent
not fetched          a dry run does not fetch the torrent, so its own web seeds and trackers are not counted
web seeds            0 so far
trackers             0 so far
```

`so far` is what was named without the torrent: the command line, a
`--web-seed-file`, a Metalink's mirrors. The same torrent read off disk prints
`name`, and the counts with no qualifier on them, because they are then
complete.

A local Metalink is the one case that is fully readable with nothing running:
the document's own claims, its mirrors and its checksums are all in the file.
What needs the network is the `.torrent` the document names by URL.

## What a failing source exits with

Most of them exit **4, source resolution**: the source names something, and
that something could not be read. A `.torrent` that is not there, or a URL that
answered with a page.

**Running out of time is 9 rather than 4**, and it is its own answer because a
retry could succeed where a re-read of the same bad document could not. A fetch
that misses its deadline and a magnet whose metadata never arrived both exit
there, naming the deadline in milliseconds.

Three exit **2** instead, because they are not sources at all and no retry
makes them one. A directory:

```bash
bit-cli info ideas/payload
```

```text
error: C:\...\ideas/payload is a directory, not a .torrent. `bit-cli create` is the command that takes a directory
```

A scheme nothing here speaks:

```bash
bit-cli info ftp://host/x.torrent
```

```text
error: `ftp:` is not a scheme this reads. A source is an http:// or https:// URL, a magnet: URI, a .torrent or metalink path, a bare info hash, or `-` for stdin
```

And a subcommand with a typo. The root command takes positional sources, so
`bit-cli tre album.torrent` is a download of something called `tre` unless
something says otherwise, and this is what says otherwise:

```text
error: `tre` is not a command, and there is no file of that name. Did you mean `bit-cli tree`?
```

That last one only fires on a bare word with nothing of that name on disk. A
source written as a path is a path: `./tre` exits 4 with a missing file, and a
torrent actually named `tre` is downloaded.

[`../exit-codes.md`](../exit-codes.md) has all seventeen codes and the rule for
when 2 is right rather than 4.
