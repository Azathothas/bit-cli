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

| form | `info`, `files`, `magnet`, `verify` | `download`, `seed` |
| --- | --- | --- |
| local `.torrent` | yes | yes |
| stdin | yes | yes |
| HTTP(S) URL | **no** | yes |
| magnet or info hash | no | yes |
| Metalink | no | yes |

**A URL is the surprising row.** `download` fetches it and completes; the other
four refuse it:

```bash
bit-cli info https://host/album.torrent
```

```text
error: https://host/album.torrent has to be fetched before it can be read
```

The exit code is 4, source resolution. That is a gap rather than a design, and
it is [T-245 in the TODO](../../TODO/cli-surface.md).

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
*links* to one is fetched and handed to the bencode parser, which fails. There
is no HTML parsing anywhere in the tree. That is
[T-244](../../TODO/cli-surface.md), and the design there is static extraction
with a browser opt-in.

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

## The exit codes this page produces

| code | name | when |
| --- | --- | --- |
| 2 | usage | the argument is not a form anything recognises |
| 4 | source resolution | the form is recognised and this command cannot resolve it |

[`../exit-codes.md`](../exit-codes.md) has all seventeen.
