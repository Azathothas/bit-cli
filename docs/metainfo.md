# Metainfo

Reading a torrent somebody else wrote, converting between a torrent and a
magnet, and what a Metalink adds.

The entries behind this are in [`TODO/metainfo.md`](../TODO/metainfo.md).

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
not whitespace or NUL. The error names the rule rather than only the symptom.

**The leading zero is refused deliberately, and not for the reason it looks
like.** It cannot move the info hash either: the same verbatim `info` bytes
that make an unsorted key safe make `i03e` safe. It is refused because no
torrent in the corpus carries one, and a parser that relaxes a rule with no
instance behind it grows tolerance nobody needed and gives a hostile file one
more shape to take. Key order was different on both counts: a real uTorrent
torrent carries it, and a `BTreeMap` discards order for free, where an
integer's byte form would have to be recorded per value to be reportable at
all. If a torrent in the wild turns up with one, this becomes the same work
key order was.

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

**A URL works the same way**, and it is how a Metalink is normally met:
`MirrorBrain` generates one per request rather than publishing a file.

```bash
bit-cli download https://download.example.org/pub/release.msi.meta4
```

The extension is read from the URL's path, so `?mirrorlist` and a fragment
change nothing, and a URL whose path is a `.torrent` is still a torrent. The
report is the same document either way. `--dry-run` is the one difference: a
saved `.meta4` is readable with nothing running and a URL is not, so a dry run
over a URL reports `document_needs_network: true` and does not fetch it, for the
same reason it does not fetch `--web-seed-list-url`.

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

## Editing without moving the info hash

`bit-cli edit` never edits in place and never changes the info hash unless
`--allow-new-infohash` is passed. An edit that would change it exits 15.

The `info` dictionary is hashed from its recorded byte span in the file it was
read from, never re-encoded, so a torrent with keys out of order or trailing
whitespace keeps its identity through an edit. Everything the hash does not
depend on comes out canonical.
