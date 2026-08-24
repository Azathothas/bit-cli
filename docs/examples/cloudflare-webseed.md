# Serving a torrent's payload from Cloudflare, and proving it works

This is the origin story. `bit-cli` exists because somebody was building a
serverless web seed on Cloudflare and needed tooling accurate enough to keep it
honest: an origin that looks fine in a browser can serve wrong bytes to a
BitTorrent client at every offset, and nothing says so.

This page is the document that job needed. Every `bit-cli` command in it was
run against a local origin while it was written; the Cloudflare half is a
procedure, and the checks are what tell you whether your origin passes.

## Which key the URLs go in, and it decides the request shape

Two BEPs, two keys, two request shapes. A torrent can carry both.

| | key | how a request is built |
| --- | --- | --- |
| **BEP 19, GetRight** | `url-list` | ordinary ranged `GET` at a URL composed from the base plus the torrent's `name` and the file's `path` |
| **BEP 17, Hoffman** | `httpseeds` | one request per piece, at `{url}?info_hash=...&piece=N` |

**Use BEP 19.** It is what a static origin serves without any code: R2 or a
Worker returning an object for a path, with `Range` support, is a complete web
seed. BEP 17 needs the origin to understand `info_hash` and `piece`, which on
Cloudflare means a Worker doing arithmetic that R2 would otherwise do for free.

BEP 17 earns its cost in one case: an origin that cannot serve byte ranges at
all but can compute a piece. That is rare enough to be the exception.

`bit-cli` detects which style a URL is by **which key it came from**, and a
command-line source with no key is probed with one request for one byte.
`--web-seed-style` overrides the detection.

## The base URL, and the one character that decides everything

BEP 19 composes a per-file URL by appending `name/path` to the base **only when
the base ends in `/`**. A base without the trailing slash is treated as naming
the payload itself.

```bash
bit-cli webseed list album.torrent --web-seed "https://mirror.example.com/pub/"
```

```text
[0] https://mirror.example.com/pub/
  scope              * (100.00%, 3 files, 6 whole pieces, 0 partial)
  composition        auto / auto / priority 0
  FILE  IN SCOPE   PATH              URL
  0     1.43 MiB   album.flac        https://mirror.example.com/pub/album/album.flac
  1     39.06 KiB  cover.jpg         https://mirror.example.com/pub/album/cover.jpg
  2     2.93 KiB   extras/notes.txt  https://mirror.example.com/pub/album/extras/notes.txt
```

That is the whole addressing question answered before a byte moves, and it
touches the network not at all. Run it first, every time. The commonest
Cloudflare mistake is an R2 bucket whose keys are `album.flac` while the
torrent's name adds an `album/` prefix, and this prints the URL it will ask for.

`--web-seed-mode exact` uses the base URL unchanged for every file, and
`--web-seed-template` builds it from a template when the object layout is
neither of those.

## Per-file scoping, for a mirror that holds part of the payload

```bash
bit-cli webseed list album.torrent \
  --web-seed "https://a.example/pub/" \
  --web-seed-for "1=https://b.example/only/"
```

```text
[0] https://a.example/pub/
  scope              * (100.00%, 3 files, 6 whole pieces, 0 partial)
[1] https://b.example/only/
  scope              1 (2.59%, 1 files, 0 whole pieces, 1 partial)
  FILE  IN SCOPE   PATH       URL
  1     39.06 KiB  cover.jpg  https://b.example/only/album/cover.jpg
```

**`0 whole pieces, 1 partial` is the line to read.** File 1 is 39 KiB inside a
256 KiB piece, so a mirror holding only that file cannot complete a piece
alone. That is not an error and it is worth knowing: the source contributes
bytes and something else has to finish the piece.

A selector is a file index, a range, a glob, or `*`.
[`../webseed.md`](../webseed.md) has the whole grammar.

## What the origin has to do, and how to check each one

### Serve `206` to a ranged request

This is the requirement. An origin that ignores `Range` and returns `200` with
the whole entity is the dangerous failure: a client that reads the response as
if it were the requested range gets wrong bytes at every offset, and every
piece fails its hash for no visible reason.

`bit-cli` refuses a `200` answer to a ranged request rather than reading it.
Every implementation in the reference corpus that gets this wrong has an open
bug about corrupt downloads.

```bash
bit-cli webseed test album.torrent --web-seed "https://mirror.example.com/pub/"
```

That reports, per source: range support, the entity length against what the
torrent says, the redirect chain hop by hop, the negotiated TLS version and
cipher suite, and the latency. One request per source, one byte of payload at
most.

**R2 through a Worker is where this breaks.** `env.BUCKET.get(key)` without the
`range` option returns the whole object however the request was framed. Pass
the range through:

```js
const range = request.headers.get("range");
const object = await env.BUCKET.get(key, { range: request.headers });
// and answer 206 with a Content-Range, not 200
```

An R2 custom domain, with no Worker in front, handles ranges itself.

### Keep `ETag` and `If-Range` stable across a deploy

`Range` plus `If-Range` is how a client resumes a partial read. If the `ETag`
changes, the origin is entitled to answer `200` with the whole entity instead,
and the client is back to the wrong-bytes case above.

A Worker that computes its own `ETag`, or one that serves a build artefact
whose hash changes on every deploy, breaks resumption mid-download for everyone
holding the old value. Serve R2's own `httpEtag` and let it be stable for as
long as the object is.

### Say what it does with a redirect

A redirect is fine and it is worth measuring, because each hop is latency on
every request and a redirect to a different host may lose `Range` handling.

`bit-cli webseed test` prints the chain hop by hop. `scripts/check-redirect.ps1`
is a different thing despite the name, so read the chain from `webseed test`
rather than reaching for that script.

### Not transcode

`bit-cli` sends `Accept-Encoding: identity` on every web seed request. A
transcoding proxy changes what a byte range means, so a correct request returns
wrong bytes from a healthy origin. Do not configure Cloudflare to compress the
payload path.

## The failure matrix, and which failures are worth retrying

| status | what it means for a source | retry |
| --- | --- | --- |
| 200 to a ranged request | the origin ignores `Range`. Refused | no, it is a configuration defect |
| 206 | correct | |
| 301, 302, 307, 308 | followed, and the chain is reported | |
| 401, 403 | not authorised. Retired by default | no |
| 404, 410 | not there. Retired by default | no |
| 416 | the range is outside the entity. Usually a size mismatch | no |
| 429 | rate limited | **yes**, and this is the one that matters on Cloudflare |
| 500, 502, 503, 504 | transient | **yes** |

`--web-seed-retry-status` and `--web-seed-fatal-status` move a status between
the two columns, `--web-seed-max-errors` is how many failures retire a source,
and `--web-seed-cooldown` is how long a retired source waits before it is tried
again.

**429 is the Cloudflare-specific one.** A free-tier Worker or a bucket behind
an aggressive rate limit will produce them under a real swarm, and a source
retired on the first burst is a source lost for the whole download. Give it a
cooldown rather than a fatal status.

## Measure it before trusting it

```bash
bit-cli webseed probe album.torrent \
  --web-seed "https://mirror.example.com/pub/" --concurrency-sweep 1,2,4,8
```

Latency percentiles and throughput as concurrency rises, per source. Cloudflare
rate limits per object, so the curve usually flattens earlier than the
connection would suggest, and the number where it flattens is the number to
pass to `--web-seed-connections`.

[`mirror-benchmark.md`](mirror-benchmark.md) is the longer version of this,
with the four stages that attribute the cost.

## What guarantees the bytes

Every piece is checked against the torrent's own hashes before it counts,
whatever the source. A mirror that serves wrong bytes is caught by the hash,
and `--web-seed-verify` decides what happens next: the source that supplied the
bad block is convicted and retired rather than the whole download failing.

That is why an origin misconfiguration here is a slow download rather than a
corrupt file. [`../integrity.md`](../integrity.md) states the guarantee in
full.

## What was not run for this page

The Cloudflare side. Every `bit-cli` command above was run against a local
origin while this was written, and the R2 and Worker behaviour is stated from
their documented semantics rather than from a deploy. The checks are the part
that tells you whether your origin actually does what this page says it should,
and `bit-cli webseed test` against your real URL is the one to run first.

`scripts/check-metalink-real.ps1` and `scripts/bench-webseed.ps1` both run
against real public mirrors and are the closest thing here to a live origin
test.
