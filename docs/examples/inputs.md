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

The split is one question: does the form name a document that one `GET` can
answer, or does it name something only a swarm can.

| form | `info`, `files`, `magnet`, `verify`, `webseed`, `bench webseed` | `download`, `seed` |
| --- | --- | --- |
| local `.torrent` | yes | yes |
| stdin | yes | yes |
| HTTP(S) URL | yes | yes |
| Metalink, local or by URL | yes, after fetching the torrent it names | yes |
| magnet or info hash | no | yes, after a swarm lookup |

Every row was run. The four read-only commands were run against all five forms
and their `--json` output compared field for field against the same torrent
read off disk: everything matches but `generated_at`, which is two runs, and
`source_kind`, which differs because the source genuinely was a URL.

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
- A `.torrent` body is capped at 16 MiB and a Metalink at 1 MiB, measured as
  the bytes arrive rather than after the whole body is in memory. Over the cap
  is exit 4.

A URL that answers with something that is not a torrent fails and says what
arrived, naming the declared content type when the server sent one:

```bash
bit-cli info https://host/downloads/
```

```text
error: https://host/downloads/: the server answered with text/html: not a
valid torrent: unexpected byte '<' at byte 0, expected a bencode value
```

**A magnet and a bare info hash carry no piece hashes**, so there is nothing to
report until the metadata has been pulled from the swarm. The refusal says so
and names what to do:

```text
error: a magnet URI and a bare info hash carry no piece hashes, so the metadata
has to be resolved from the swarm first
```

`bit-cli download` does that lookup. So does `bit-cli magnet` in the other
direction, turning a `.torrent` into a URI without any network at all.

## What is not an input yet

**A directory.** Only `bit-cli create` takes one, and it takes it as a
positional path rather than as a `SOURCE`. Handing a directory to a command
that wants a torrent reports the operating system's reason rather than the real
one, which is [T-246](../../TODO/cli-surface.md).

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

Over a URL it prints less, because a dry run does not fetch and there is
nothing to count. The text rendering shows those counts as `0`, which reads as
"the torrent has none" and means "nothing looked". The `--json` form of the same
run is correct and carries `null`. Read the JSON when the source is a URL, and
see [T-247](../../TODO/cli-surface.md).

A local Metalink is the one case that is fully readable with nothing running:
the document's own claims, its mirrors and its checksums are all in the file.
What needs the network is the `.torrent` the document names by URL.

## Everything on this page exits 4

Every source that fails, fails as **4, source resolution**. There is no input
to a `SOURCE` argument that produces a usage error, because the last rule in
the classifier is "treat it as a path" and a path that is not there is a
resolution failure rather than a usage one.

That includes a scheme nothing here speaks:

```bash
bit-cli info ftp://host/x.torrent
```

```text
error: cannot read C:\...\ftp://host/x.torrent: The filename, directory name,
or volume label syntax is incorrect. (os error 123)
```

An `ftp://` URL is read as a relative filename. It is the same shape as the
directory case above and is filed with it, under
[T-246](../../TODO/cli-surface.md).

[`../exit-codes.md`](../exit-codes.md) has all seventeen codes.
