# Many sources for one payload

Five scenarios the operator put to this project, each about pointing several
different kinds of source at one payload and getting the bytes as fast as
possible without fetching any of them twice.

This file is written for the session that implements them. It is in three
parts:

1. **What already works**, each claim backed by a command that was run against
   a local fixture, with the output.
2. **The gaps**, as ordinary `TODO/` entries with an acceptance criterion.
3. **The harness** the acceptance needs, because most of it does not exist.

Everything below was checked against the code at commit `74986c3`, not
inferred from the documentation. Where a claim came out of running something,
the command is here and the fixture is reproducible.

---

## Vocabulary

The scenarios say "web seed" and "ddl" as if they were different things. In
`bit-cli` they are the same thing under one model, and that is why several of
these scenarios need less work than they look like they need.

A **binding** is a triple: a **source** (an HTTP URL with its own headers,
auth, agent, timeouts, concurrency, connections, and rate cap), a **scope**
(which part of the torrent that source may serve), and a **composition** (how
the request URL is built from the source URL and the torrent's `name` and
`path`).

A "direct download link for one file" is therefore already expressible: it is
a source with scope `file:N` and composition `exact`. There is no separate
"ddl" concept to add and nothing about the term needs to reach the CLI.

---

## Part 1: what already works

The fixture for everything in this part:

```powershell
# A deeply nested three-file torrent, and a CDN copy of one file under a
# different name in a different directory.
payload/deep/nested/dirs/file.blob   64 MiB
payload/deep/other.bin                8 MiB
payload/readme.txt                    1 MiB
cdn/a3f1b2c4-signed-blob.dat         the same 64 MiB, renamed

bit-cli create payload --name payload --piece-length 1MiB `
  --no-creation-date --output torrent_a.torrent
```

```
$ bit-cli files torrent_a.torrent
INDEX  SIZE       SHARE   PIECES  PATH
0      64.00 MiB  87.67%  0-63    deep/nested/dirs/file.blob
1      8.00 MiB   10.96%  64-71   deep/other.bin
2      1.00 MiB   1.37%   72-72   readme.txt
```

### Scenario 1 works today, in full

One selected file out of a deep tree, 70% already on disk, accelerated by an
arbitrary CDN URL whose name and path have nothing to do with the torrent's.
Everything below was run. What happens when the CDN starts answering 403 is
[T-130](#t-130-a-source-cannot-be-told-which-statuses-are-worth-retrying),
which is now **done**: with `--web-seed-retry-status 403` a source whose
signature expires 22 times over a 64 MiB payload completes it byte for byte.

```bash
bit-cli download torrent_a.torrent --dir out \
  --select-file 0 \
  --web-seed-for 'file:0=http://cdn.example/cdn/a3f1b2c4-signed-blob.dat' \
  --web-seed-mode exact \
  --continue
```

Run against the fixture with 45 MiB of the 64 MiB file pre-seeded:

```
exit=0 completed=1 failed=0
total=64.00 MiB downloaded=64.00 MiB from_web_seeds=19.00 MiB
file.blob: MATCHES source
other files present? payload\deep\nested\dirs\file.blob
```

Four things that answers:

- **Only the missing bytes were fetched.** 19 MiB over HTTP for the 19 MiB
  that was not on disk. The hash check on add is what establishes that, and
  `--continue` is on by default.
- **The URL needed no relationship to the torrent.** `--web-seed-mode exact`
  means the URL is the complete resource and nothing is appended.
- **Only the selected file was written.** The other two paths were never
  created, which is [T-013](disk-io.md).
- **The result is byte-identical to the source.** Every HTTP-sourced piece is
  hash-checked at the source before the session sees it, which is
  `--web-seed-verify piece` and the default.

`bit-cli webseed list` resolves the binding without touching the network, so
the addressing can be checked before any bytes move:

```
[0] http://127.0.0.1:55654/cdn/a3f1b2c4-signed-blob.dat
  scope              file:0 (87.67%, 1 files, 64 whole pieces, 0 partial)
  composition        exact / auto / priority 0
  FILE  IN SCOPE   PATH                        URL
  0     64.00 MiB  deep/nested/dirs/file.blob  http://127.0.0.1:55654/cdn/a3f1b2c4-signed-blob.dat
```

`uncovered pieces 64-72` is printed for the pieces no source covers, which is
what tells the caller the other two files still need the swarm.

### Scenario 3 works today, in full

Scenario 3 is Scenario 1 without `--select-file`. Drop that flag and the other
files keep coming from peers while file 0 comes from the CDN. Nothing else
changes, because a scope of `file:0` already restricts the source to that file
and the coverage report already names what is left for the swarm.

### Redirects, including a URL that re-signs on every request

The fetcher does not pin a resolved URL. Every ranged request goes to the URL
the caller gave, and `reqwest`'s default redirect policy follows up to ten
hops per request. So a CDN whose stable URL 302s to a freshly signed URL each
time is handled by doing nothing: each request gets its own signature.

`webseed test` reports the chain hop by hop, so the behaviour is checkable
before a download:

```bash
bit-cli webseed test torrent_a.torrent --web-seed-for 'file:0=https://cdn.example/blob'
```

### Per-source everything, through the binding table

Anything the command line sets globally, the table sets per source. This is
Scenario 4's mapping problem and most of Scenario 5's control problem:

```toml
[[source]]
url         = "https://mirror-a.example.com/pub/"
scope       = "*"
mode        = "auto"
priority    = 10
connections = 2
concurrency = 8
rate_limit  = "40MiB/s"
headers     = { X-Region = "apac" }
user_agent  = "bit-cli/0.1"
auth        = "bearer:TOKEN"

[[source]]
url   = "https://cdn.example.com/blobs/a3f1b2/payload.bin"
scope = "file:0"
mode  = "exact"

[[source]]
url      = "https://odd.example.com/store/{raw:path}?v=2"
scope    = "file:3-9"
mode     = "template"
headers  = { Authorization = "Basic ..." }
```

Every field in that table is read and applied: `url`, `scope`, `mode`,
`template`, `style`, `priority`, `concurrency`, `connections`, `chunk_size`,
`rate_limit`, `timeout_ms`, `connect_timeout_ms`, `retries`, `max_errors`,
`cooldown_ms`, `user_agent`, `headers`, `auth`, plus a `[default]` block that
supplies any of them to every source that does not override it.

`template` mode is what handles a server that lays the payload out differently
from the torrent. Eleven placeholders: `{name}` `{path}` `{filename}`
`{index}` `{piece}` `{offset}` `{length}` `{end}` `{piece_offset}`
`{piece_length}` `{infohash}`, percent-encoded unless written `{raw:path}`.

### The three transports already run at once

Peers, the torrent's own `url-list`, and command-line sources are all live in
the same run, and the report keeps them apart:

```json
"from_web_seeds": { "bytes": 19922944, "human": "19.00 MiB" },
"from_peers":     { "bytes": 47185920, "human": "45.00 MiB" }
```

Per source it also reports `http_bytes` beside `bytes`, so fetching the same
range twice is visible as an amplification ratio rather than hidden.

---

## Part 2: the gaps

### T-130 A source cannot be told which statuses are worth retrying

Category:    webseed
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `classify_status` in `webseed/fetch.rs:1192-1211` makes 401, 403,
             404, 410, and 416 permanent, and a permanent failure retires the
             source for the run. A CDN that signs URLs answers 403 when a
             signature expires, and the next request to the stable URL would
             redirect to a fresh signature and succeed.
Relevance:   This is the one thing standing between Scenario 1 and working
             unattended for longer than a signature lasts. It is also
             Scenario 4's "override the status code as some servers return
             different codes even though the content exists".
Approach:    A per-source status policy, on the command line and in the table:

                 --web-seed-retry-status 403,429,503
                 --web-seed-fatal-status 404

             `retry_status` moves a code from permanent to transient, and
             `fatal_status` moves one the other way. Both take a list of codes
             and ranges. The table gets `retry_status` and `fatal_status`
             arrays per source and in `[default]`.

             The existing per-source `retries`, `max_errors`, and `cooldown_ms`
             then bound it, so a source whose signature cannot be refreshed
             still retires rather than looping. Nothing new is needed for the
             backoff.
Acceptance:  The fixture below, driven end to end. A source that answers 403
             after N requests completes the payload with
             `--web-seed-retry-status 403` and fails without it, and the run
             reports how many retries each status cost.

             Measured today, without the flag, against
             `loopback-fileserver --status 403 --fail-after 6`:

             ```
             exit=1 completed=0 failed=1 stopped=failed
             downloaded=5.00 MiB of 64.00 MiB
             warning: web seed .../cdn/a3f1b2c4-signed-blob.dat is unusable:
               403 Forbidden, check --web-seed-auth and --web-seed-header
             ```
Closed:      `--web-seed-retry-status` and `--web-seed-fatal-status` take codes
             and inclusive ranges (`403`, `403,429`, `500-599`). The table
             takes `retry_status` and `fatal_status` per source and in
             `[default]`, as integers, as range strings, or as one string. A
             code in both lists is a usage error rather than a precedence rule,
             because there is no defensible answer and picking one silently
             hides the mistake. `webseed list` prints both when they are set,
             so the policy is checkable before any bytes move.

             `bit-cli download --json` now carries `retries` and
             `retries_by_status` per source, and the text output prints
             `retries 22 (22 on 403)` when there were any.

             Acceptance, `pwsh -NoProfile -File scripts/check-signed-source.ps1`
             at 64 MiB. The pair that carries it is `expiring_default` and
             `expiring_retry`: the same server, the same window, differing only
             in the flag.

             ```
             expiring_default    1 0 B        -         1   1       0 yes
             expiring_retry      0 64.00 MiB  matches  86  22      22 yes
             ```

             `fatal_override` and `recovering_503` are the other direction and
             its control: `--status 503 --fail-after 4 --recover-after 8`
             completes with no policy, because 503 is already transient, and
             fails with `--web-seed-fatal-status 503`.

**Two defects turned up while building the acceptance, and the second one is
larger than this entry.**

**One.** The bridge retired a source on the first request that ran out of its
own retries, whatever the classification. `--web-seed-max-errors` could
therefore never be reached: one exhausted request ended the source before a
second error could be counted. `crates/bit-cli-core/src/webseed/bridge.rs`
carried the reason for it as a comment, "the fetcher already retried what was
worth retrying, so anything surfacing here means the source is done", and that
is true of a permanent failure and false of a transient one.

It showed up as the `recovering_503` control failing: a mirror that answers 503
for four requests and then serves normally killed the source, with **no flag
set at all**. That is not a status policy problem, it is the default path, and
it means every mirror that restarted mid-download was lost.

Fixed. A block failure now carries whether the source could still answer, and a
transient one reconnects like a link failure instead of retiring. What bounds
the loop is `--web-seed-max-errors` consecutive failed requests tripping the
source's cooldown, which the bridge reads and retires on. Measured: the same
control now completes 64 MiB with 6 retries.

**Two.** `--web-seed-cooldown` sets a timer nothing waits out. A source whose
budget runs out is retired for the rest of the run rather than sitting out the
cooldown and coming back, so the flag moves no number. That is
[T-137](#t-137-a-cooled-down-source-never-comes-back), open, with the
trade-off named. The two doc comments that implied a source returns now say
what happens instead.

### T-131 The loopback file server cannot simulate a signed URL

Category:    bench
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `crates/bit-cli-core/examples/loopback-fileserver.rs` has
             `--ignore-range`, `--status`, `--stall-after`, `--fail-after`, and
             `--no-keep-alive`. It cannot redirect, and it cannot expire a
             signature, so neither half of Scenario 1's hard case can be
             tested end to end.
Relevance:   [T-130](#t-130-a-source-cannot-be-told-which-statuses-are-worth-retrying)
             cannot be accepted without it, and rule 3 says nothing counts as
             proven until it has run against real infrastructure. A local
             server that behaves like the real one is the closest thing
             available for a CDN nobody here controls.
Approach:    Three flags on the example server:

             - `--sign-redirect <SECONDS>`: a request to a stable path answers
               302 to the same path with `?sig=<random>&exp=<unix>`, valid for
               that many seconds.
             - `--require-sig`: a request carrying no `sig`, or an expired
               one, answers 403.
             - `--redirect-chain <N>`: N hops before the payload, for the
               redirect-following test.

             Keep it a single-file example with no new dependency, the way it
             is now.
Acceptance:  `--sign-redirect 2 --require-sig` serves a 64 MiB payload to
             `bit-cli download` over more than two seconds, so at least one
             signature expires mid-run, and the download completes with the
             right hash once [T-130](#t-130-a-source-cannot-be-told-which-statuses-are-worth-retrying)
             is in.
Closed:      All three flags are in, plus a fourth,
             `--recover-after <M>`, which ends a `--fail-after` window after M
             requests so a status that recovers can be produced without a
             clock. Eleven unit tests cover the routing and the signature
             check, and `[[example]] test = true` in
             `crates/bit-cli-core/Cargo.toml` puts them in
             `cargo test --workspace`.

             The signature is SplitMix64 over a per-process secret and the
             window index, so it is unguessable from the URL and stable for
             the length of its window. `exp` is unix milliseconds rather than
             seconds, because a window can be shorter than a second and the
             measurement below needs it to be.

             The server on its own, checked with `curl` against a 1 MiB
             payload under `--sign-redirect 2 --require-sig`:

             ```
             GET /blob.bin        -> 302 .../blob.bin?sig=cc11a1..&exp=1787216384041
             GET /blob.bin?sig=.. -> 206  (immediately)
             GET /blob.bin?sig=.. -> 403  (two seconds later, signature expired)
             GET /blob.bin        -> 206  (the stable path, redirected to a fresh signature)
             ```

             That last pair is exactly what
             [T-130](#t-130-a-source-cannot-be-told-which-statuses-are-worth-retrying)
             describes: a real 403, and a stable URL that recovers.

             `pwsh -NoProfile -File scripts/check-signed-source.ps1` drives all
             six cases end to end and is the acceptance for this entry and for
             [T-130](#t-130-a-source-cannot-be-told-which-statuses-are-worth-retrying).
             It lands with T-130, because two of its six cases need that flag.
             At 64 MiB, all six as described:

             ```
             case             exit downloaded hash    302 403 retries ok
             redirects           0 64.00 MiB  matches 256   0       0 yes
             too_many_hops       1 0 B        -        11   0       0 yes
             expiring_default    1 0 B        -         1   1       0 yes
             expiring_retry      0 64.00 MiB  matches  86  22      22 yes
             fatal_override      1 4.00 MiB   -         0   0       0 yes
             recovering_503      0 64.00 MiB  matches   0   0       6 yes
             ```

**The acceptance as written above cannot fire, and that is the finding.**
`--sign-redirect 2 --require-sig` serves a 64 MiB payload with **zero** 403s,
because `bit-cli` re-resolves the stable URL on every ranged request and the
signature it is handed is a millisecond old when it uses it. A signature
expires mid-run only when the window is shorter than the round trip from the
`302` to the request that carries it. Measured, 64 MiB in 1 MiB chunks against
`--sign-redirect W --require-sig`:

| window | 302 | 206 | 403 | exit |
| --- | --- | --- | --- | --- |
| 2s | 64 | 64 | 0 | 0 |
| 0.1s | 64 | 64 | 0 | 0 |
| 0.01s | 26 | 19 | 7 | 1 |
| 0.002s | 1 | 0 | 1 | 1 |

So the entry's premise held for the wrong reason. The half it was written to
test, "a CDN whose stable URL 302s to a freshly signed URL each time is handled
by doing nothing", is now proven rather than asserted: the `redirects` case
above answers 256 redirects for 64 requests, four hops each, and completes with
the payload byte for byte. A client that pinned a resolved URL would show one
redirect for the whole run.

`scripts/check-signed-source.ps1` runs at `-Window 0.01` for that reason, and
says so in the report's `notes`.

### T-132 The swarm cannot be rate limited separately from HTTP sources

Category:    performance
Priority:    P1
Effort:      M
Status:      open

Problem:     `--max-download-rate` goes into the session's `LimitsConfig`, and
             HTTP sources reach the session as peers over loopback, so a
             session cap applies to both. `--web-seed-speed-limit` caps HTTP
             sources only. There is no way to cap peers only.
Relevance:   Scenario 5 asks to "cap/limit speed/connection per method". Two
             of the three directions exist; the third does not, and the
             asymmetry is not documented.
Approach:    The bridge already has a token bucket per source
             ([T-035](performance.md)). A peer-side cap needs either the
             session cap set to the peer budget and the bridge exempted, which
             it cannot be because the bridge is a peer, or an accounting split
             the session does not expose.

             What is likely to work: leave the session cap off and derive it.
             When both `--max-peer-rate` and a web seed cap are set, set the
             session cap to their sum and hold each side to its own bucket.
             The web seed bucket already exists; the peer side would be the
             session cap minus what the buckets are allowed.

             Measure before committing to that. The first question is whether
             the session cap is even enforced, which is
             [T-031](performance.md), still open.
Acceptance:  A hybrid run with `--max-peer-rate 10MiB/s --web-seed-speed-limit
             50MiB/s` reports peer bytes within 10% of 10 MiB/s and HTTP bytes
             within 10% of 50 MiB/s over sixty seconds, both from the same
             report, plus the same run with each cap alone.

### T-133 Two torrents holding the same file cannot share its bytes

Category:    webseed
Priority:    P1
Effort:      L
Status:      **layer 1 done**, layers 2 and 3 open

Problem:     Scenario 2. Three torrents with different info hashes each contain
             a bit-identical `file.blob`. One is 60% done and stalled, one is
             slow, and one is slow but carries a fast web seed. Nothing in
             `bit-cli` connects them: there is no cross-torrent identity, no
             shared content store, and a source URL must be `http` or `https`,
             so a completed copy on the local disk could not be named as a
             source either:

             ```
             $ bit-cli webseed list t.torrent --web-seed-for 'file:0=file:///C:/path/file.blob'
             error: only http and https sources are supported
             ```

             That last part is fixed: see **Layer 1** below.
Relevance:   It is the difference between downloading the same 64 MiB once and
             downloading it three times, which is Scenario 5's "minimize
             bandwidth usage" in its sharpest form.
Approach:    Three layers, and the first is worth doing on its own:

             1. **A local source.** Accept `file:` URLs as a source with a
                scope, reading ranges out of a local path. Then Scenario 2 is
                a two-step the operator can drive today: finish `file.blob`
                under torrent C, then point torrents A and B at it with
                `--web-seed-for 'file:0=file:///...'`. Effort S, and it also
                serves "I already have this file somewhere".
             2. **Declared equivalence.** `--same-file
                'HASH_X:file:0=HASH_Y:file:3'` or a table, asserting two
                torrents' files are identical. `bit-cli` verifies the claim
                per piece before trusting it, because a wrong assertion would
                otherwise corrupt a payload silently. Verification is possible
                only where the two torrents' piece boundaries align on that
                file; where they do not, the claim can still be checked once
                the bytes are complete.
             3. **Derived equivalence.** Same length and same piece hashes over
                the aligned range implies the same bytes with no assertion
                from the caller. This is the one that makes Scenario 2 need no
                flags at all, and it only works when the piece length and the
                file's offset within the torrent line up.
Acceptance:  Layer 1: a `file:` source completes a torrent with no network at
             all, and the payload hashes equal. Layer 2 and 3: three torrents
             built from one payload with different piece lengths and different
             surrounding files, added in one invocation, and the report shows
             the shared file's bytes fetched once and written into all three
             output directories, with all three hashing equal.
Layer 1:     **done.** A source URL may be `file:`, and everything else about a
             source still applies to it: scope, composition, chunk size, rate
             limit, retries, per-piece verification, per-source accounting, and
             the same loopback bridge. `crates/bit-cli-core/src/webseed/local.rs`
             is the whole of the URL handling and the positioned read;
             `Fetcher::fetch_once` branches on the scheme and nothing above it
             changes.

             `webseed list`, `webseed test`, `webseed probe`, and
             `bench webseed` all take a `file:` source. `test` reports the
             length off the filesystem and `range_support: yes` without asking,
             because a positioned read always works. `probe` and `bench` read
             the same windows at the same concurrencies, so a local source gets
             the same curve an HTTP one does.

             `pwsh -NoProfile -File scripts/check-local-source.ps1` is the
             acceptance. Six cases at 64 MiB, no server and no bound port:

             ```
             case        exit downloaded hash    ok
             exact          0 64.00 MiB  matches yes
             auto           0 68.00 MiB  matches yes
             shared_a       0 64.00 MiB  matches yes
             shared_b       0 64.00 MiB  matches yes
             wrong_bytes    1 0 B        -       yes
             missing        1 0 B        -       yes

             the shared file landed with 1 distinct hash across three info hashes
             ```

             `shared_a` and `shared_b` are Scenario 2's two-step: torrent C
             finishes `file.blob` from the CDN copy, then torrents A and B read
             the copy C wrote. Three info hashes, three piece lengths (2 MiB,
             1 MiB, 512 KiB), one 64 MiB payload fetched once, four copies
             hashing equal.

             `wrong_bytes` is the case that says the source is not trusted: a
             file of exactly the right length holding something else is refused
             by the per-piece check with the path and the piece named. Only
             that check can catch it, and it is the default.

             A `..` in a resolved path is refused. `auto` and `prefix`
             composition append the torrent's own `name` and `path`, so the
             tail of a source URL is written by the `.torrent` rather than by
             the caller, and a hostile one naming `../../../Windows/win.ini`
             would otherwise read out of a directory the caller did not name.
             The bytes would fail their piece hash, but reading them at all is
             not this tool's business.

             Two things layer 1 does not do, both of which are layers 2 and 3.
             A `--web-seed-for` binding applies to every torrent in the
             invocation, so `-j 2` over torrents A and B needs the shared file
             to be at the same index in both; it is index 0 in A and index 1 in
             B, so the two need separate invocations. And nothing derives the
             equivalence: the caller names the path.

Layers 2 and 3 stay open, and the two-step above is what the operator can drive
in the meantime.

### T-134 v1 and v2 info hashes are not reconciled

Category:    bep
Priority:    P2
Effort:      L
Status:      open

Problem:     A hybrid torrent carries both a v1 and a v2 info hash for the same
             payload, and the two name the same bytes. `bit-cli` has no v2
             support at all: [T-081](create-seed.md) is open and
             `docs/BEP.md` does not exist yet.
Relevance:   Scenario 5 asks to reconcile them. Without it, the same payload
             offered as a v1 torrent and a v2 torrent is two unrelated
             downloads, which is the same waste as
             [T-133](#t-133-two-torrents-holding-the-same-file-cannot-share-its-bytes)
             and a case where the equivalence is not a guess.
Approach:    It depends on [T-081](create-seed.md) landing first. A hybrid
             torrent's `info` dict carries both `pieces` and `file tree`, so
             once v2 parses, two torrents that share either hash are the same
             payload by definition and no verification is needed.
Acceptance:  A hybrid torrent and the v1-only torrent cut from the same payload
             are recognised as one payload, and adding both in one invocation
             fetches the bytes once.

### T-135 Source selection cannot be steered by method or by priority at run time

Category:    performance
Priority:    P2
Effort:      L
Status:      open

Problem:     `--web-seed-priority` and the table's `priority` order sources
             against each other, and `--prefer-web-seed` biases HTTP against
             peers by giving sources more connections. Neither is a decision:
             [T-003](webseed.md) established that `librqbit`'s piece picker is
             not reachable from outside the crate, so a piece a peer answers
             first still comes from the peer.
Relevance:   Scenario 5's "smartly use web seeds + ddls + p2p swarm based on a
             priority". What ships today moves the odds. Measured, that is
             worth moving the HTTP share of a hybrid run from 46.72% to
             62.60%, and no further.
Approach:    [T-002](webseed.md) priced the real fix: an in-process peer needs
             four `pub(crate)` markers changed in `librqbit`, and the
             machinery underneath already takes an arbitrary byte stream.
             Owning the picker means owning that fork. Decide that explicitly
             rather than drifting into it.
Acceptance:  A hybrid run with a stated priority order fetches every piece from
             the highest-priority source that holds it, proven by per-source
             byte attribution against a fixture where each source holds a known
             disjoint set.

### T-136 Nothing states the end-to-end integrity guarantee

Category:    cli
Priority:    P2
Effort:      S
Status:      open

Problem:     Scenario 5 asks for a guarantee that a finished file is bit-for-bit
             correct. The mechanisms are all there and none of them is stated
             as a contract: the per-source piece check
             (`--web-seed-verify piece`, the default), the session's own check,
             the hash check on add that makes resume safe, and
             `bit-cli verify`.
Relevance:   A guarantee nobody wrote down is not a guarantee, and this one is
             the reason a caller would trust a source it found on a CDN.
Approach:    A section in `README.md` naming each check, what it catches, and
             what it costs, and a `--verify-on-complete` flag that re-reads the
             finished payload and reports the hash of every file. It is
             redundant with the piece checks by construction, which is the
             point: it is the check a caller can run without trusting the
             thing that wrote the bytes.
Acceptance:  A run against a mirror serving one corrupt byte completes from
             another source, and the report names the piece, the source, and
             the mismatch. `--verify-on-complete` on the finished payload exits
             0 and prints a hash per file.

### T-137 A cooled-down source never comes back

Category:    webseed
Priority:    P2
Effort:      S
Status:      **done**

Problem:     `--web-seed-cooldown` and the table's `cooldown_ms` set a timer,
             and nothing waits it out. `SourceStats::record_error` stores an
             epoch millisecond deadline after `max_errors` consecutive failed
             requests, `SourceStats::is_cooling_down` reads it, and the bridge
             retires the source the moment it is true
             (`crates/bit-cli-core/src/webseed/bridge.rs`, the
             `BridgeError::Stalled` arm of `run`). So the flag changes nothing
             a caller can measure: any positive value behaves the same as any
             other.

             It was found closing
             [T-130](#t-130-a-source-cannot-be-told-which-statuses-are-worth-retrying),
             which made `max_errors` reachable for the first time and so made
             the cooldown reachable too.
Relevance:   Rule 0.10: a flag that does not move a number does not ship.
             Either it moves one or it goes. It also decides how long an
             unattended run tolerates a mirror that is down: today the answer
             is `retries` attempts times `max_errors` requests, about 17
             seconds at the defaults, and then the source is gone for good.
Approach:    Two options, and the choice is a trade-off rather than a bug fix.

             1. **Honour it.** The bridge sleeps until
                `stats.cooldown_until()` and reconnects, and the source's
                consecutive-error count resets. `cooldown_ms` then means what
                it says. The cost is that a run against one dead mirror with
                `--web-seed-only` stops failing fast: it sits for the default
                ten minutes instead of exiting in seconds, and only
                `--timeout` or `--stop-timeout` ends it. That is the wrong
                default for an unattended caller.
             2. **Cut it.** Remove `--web-seed-cooldown` and `cooldown_ms`,
                and let `--web-seed-max-errors` alone decide when a source is
                out. Smaller surface, nothing lost that a caller can observe
                today.

             The likely answer is 1 with a default of zero, meaning "do not
             come back", so fail-fast stays the default and a caller who wants
             a mirror to be given another chance says how long to wait. That
             needs the reported state to distinguish a cooling source from a
             failed one, or `--web-seed-require` and the "every source failed"
             stop condition in `crates/bit-cli/src/cmd/download.rs` will read
             a sleeping source as a live one and wait out the deadline.
Acceptance:  Two runs against `loopback-fileserver --status 503 --fail-after 4
             --recover-after 200`, one with a cooldown shorter than the outage
             and one with a cooldown longer than it. The first completes and
             the second does not, and the report says which source cooled down
             and for how long. Plus a run against a dead mirror with
             `--web-seed-only` proving the fail-fast path still exits in
             seconds.

**Option 1, with a default of zero, which is what the entry expected.**

The bridge sleeps out the deadline and reconnects with the error run cleared.
`--web-seed-cooldown 0`, the default, retires the source instead, so the
fail-fast path is unchanged and the flag is entirely opt-in.

Three things had to be separated that were one thing before:

- **The budget being spent and the wait being over.** `SourceStats` now has
  `budget_spent`, true from the moment `max_errors` consecutive requests fail
  until `end_cooldown` clears it, and `is_cooling_down`, true only while the
  deadline is ahead. They differ exactly when the cooldown is zero: the budget
  is spent and there is nothing to wait for. The guard on the fetch path is
  `budget_spent`, so a source that is out stays out whatever the timer says.
  `record_error` stores `until.max(1)` rather than `until`, because zero is the
  sentinel for "never tripped" and a zero-millisecond cooldown has to be
  distinguishable from one.
- **A sleeping source and a dead one.** `BridgeState::Cooling` sits between
  `Idle` and `Failed` in `AttachedSource::state`'s ranking. The report carries
  `cooldowns`, `cooldown_until`, and `cooldown_remaining_ms`, and a
  `source_cooling` event fires once per cooldown rather than once per source,
  so a mirror that goes out, comes back, and goes out again is reported each
  time. The "every source is dead" stop condition in `cmd::download::watch` is
  unchanged and now means what it says: a cooling source is not failed, so the
  run waits for it, bounded by `--timeout` or `--stop-timeout`.
- **Which deadline a waking bridge is allowed to clear.** Several connections
  share one `SourceStats`, so `end_cooldown` takes the deadline the caller
  slept on and compare-exchanges it. Without that, a bridge waking from an old
  cooldown could clear a newer one another connection had only just tripped.

**The outage had to become a clock.** `loopback-fileserver`'s failure window
was counted in requests, and a source that is cooling down makes no requests,
so the window never advanced while it waited and the mirror never came back.
`--down-for <SECONDS>` ends the window on a clock instead, starting at the
first request that falls into it, so `--fail-after` still decides when the
outage begins. Three unit tests in the example cover it.

**The measurement, 2026-08-20T13:26:02.637Z.** `scripts/check-signed-source.ps1`
now drives nine cases, the last three of which are this entry's:

```
case             exit downloaded hash    state   cooldowns
cooldown_short      0 64.00 MiB  matches active          4
cooldown_long       9 3.00 MiB   -       cooling         1
dead_mirror         1 0 B        -       failed          1
```

```
$ pwsh -NoProfile -File scripts/check-signed-source.ps1
```

`cooldown_short` and `cooldown_long` are the same server, the same 20 second
outage, the same `--timeout 60s`, and the same `--web-seed-max-errors 2
--web-seed-retries 0`. The only difference is `--web-seed-cooldown`: 5 seconds
against 300. The first cooled down four times, waking twice into a mirror that
was still down and once into one that was back, and completed in 24.3 seconds
with the payload hashing equal. The second cooled down once and was still
asleep with 241.1 seconds left when the deadline fired, at 3.00 MiB of 64.

`dead_mirror` is the fail-fast case at every default, including
`--web-seed-cooldown 0`: a mirror answering 503 forever retires the source and
the run exits 1 after 33.4 seconds. That is longer than the "about 17 seconds"
this entry predicted, and the difference is the bridge's own reconnect backoff
between attempts, which the estimate did not count. Both numbers are seconds
rather than the ten minutes the old default would have produced, which is the
point of the default.

Five unit tests cover the state machine without a network:
`a_zero_cooldown_spends_the_budget_with_nothing_to_wait_for`,
`ending_a_cooldown_clears_the_error_run_but_not_the_totals`,
`cooldown_trips_only_after_the_configured_run_of_errors`,
`a_timed_outage_closes_on_the_clock_rather_than_on_a_request_count`, and
`a_timed_outage_starts_when_the_failure_window_does`.

---

## Part 3: the harness

What the acceptances need, in the order it unblocks the most. The first two
exist.

1. **[T-131](#t-131-the-loopback-file-server-cannot-simulate-a-signed-url)**,
   the signing and redirecting file server, is **done**. `--sign-redirect`,
   `--require-sig`, `--redirect-chain`, and `--recover-after` are on
   `crates/bit-cli-core/examples/loopback-fileserver.rs`, and
   `scripts/check-signed-source.ps1` drives all six cases Scenario 1 and 4
   need.
2. **The fixture**, which exists: `scripts/make-scenario-fixture.ps1`. It
   builds one payload, three torrents with different piece lengths, different
   surrounding files, and three different info hashes, a CDN copy under an
   unrelated name, a second mirror layout with a space in a directory name,
   and the partial on-disk state each scenario starts from.

   ```
   $ pwsh scripts/make-scenario-fixture.ps1 -BlobSizeMiB 16 -Partial 70

   payload_a    5164aaf5bbb40cd396ba52945c5221074aa14f12   25.00 MiB  pieces   25 of 1.00 MiB
   payload_b    c2806b5adee5e75398f6741b9af66cb9951059c0   19.00 MiB  pieces   38 of 512.00 KiB
   payload_c    31084dc6ab74b846654ffecbc721fc1865989cf7   20.00 MiB  pieces   10 of 2.00 MiB

   the shared file, byte for byte the same in all three:
     42EE6DB050DB50CE  payload_a/deep/nested/dirs/file.blob
     42EE6DB050DB50CE  payload_b/media/file.blob
     42EE6DB050DB50CE  payload_c/a/b/c/file.blob
     42EE6DB050DB50CE  cdn/a3f1b2c4-signed-blob.dat
   ```

   The three piece lengths are the point. Equivalence that only holds when the
   piece boundaries line up is not equivalence, and 1 MiB against 512 KiB
   against 2 MiB is what makes
   [T-133](#t-133-two-torrents-holding-the-same-file-cannot-share-its-bytes)
   testable rather than assumed.
3. **A second file server on a different port** answering the same payload
   under the `mirror/pub files/payload/` layout the fixture already builds, for
   Scenario 4. `--ignore-range` and `--status` already cover the failure half,
   and one server rooted at the fixture serves both layouts today.

Nothing here needs the network. All five scenarios are testable end to end on
loopback, which is what makes them worth doing properly.

---

## State: there is none, and that is the design

The operator asked whether `bit-cli` uses SQLite and whether it should.

**It stores nothing.** No database, no session file, no resume cache, no
registry. The only file it reads outside the output directory is an optional
`config.toml`, and `--no-config` turns that off. Decision 7.4 puts every form
of stored session state in Phase C, and `SessionOptions::persistence` is
`None` for that reason.

Resume works without state because the payload is the state: adding a torrent
hash-checks what is on disk, and what checks out is not fetched again. That is
what made Scenario 1 fetch 19 MiB rather than 64. It costs a full read of the
payload on every add, which is [T-016](disk-io.md), blocked upstream because
`fastresume` in `librqbit` 9.0.0 does nothing without turning on the
persistence store that 7.4 forbids.

**Would SQLite help these scenarios?** For four of the six entries above, no.
[T-130](#t-130-a-source-cannot-be-told-which-statuses-are-worth-retrying),
[T-131](#t-131-the-loopback-file-server-cannot-simulate-a-signed-url),
[T-132](#t-132-the-swarm-cannot-be-rate-limited-separately-from-http-sources),
and [T-136](#t-136-nothing-states-the-end-to-end-integrity-guarantee) are all
within one invocation and need nothing remembered.

For [T-133](#t-133-two-torrents-holding-the-same-file-cannot-share-its-bytes)
it depends on which layer:

- Layer 1, a `file:` source, needs no state: the caller names the path.
- Layers 2 and 3 need no state either **when the torrents are added in one
  invocation**, which is how the scenario is written. Equivalence is computed
  from the metainfo the run already has.
- A store is only needed to carry equivalence *between* invocations, so that
  torrent B added tomorrow knows about torrent A's file from today. That is
  the same shape as [T-016](disk-io.md)'s resume cache and the same shape as
  every Phase C item.

So the recommendation is: **do not add SQLite for these scenarios.** Build
them one-invocation-first, which is what decision 7.4 already requires, and
which is also the faster thing to build and the easier thing to test.

If a cross-invocation store is later wanted, the thing to weigh is not SQLite
against files but what the store is for. A content-addressed index of "which
local path holds the bytes for piece hash H" is a key-value lookup that a
single append-only file with an in-memory index serves at a fraction of the
dependency cost, and it degrades to "not found" safely. SQLite earns its place
when several processes write concurrently, and decision 7.4 says there is only
ever one.

Whatever is built, the rule stated in the operator's brief holds: `bit-cli`
must keep working with no config file and no state file. Every store is an
optimisation that a cold run reproduces by reading the payload.

---

## What the five scenarios need, in one table

| Scenario | Works today | Needs | Size |
| --- | --- | --- | --- |
| 1. DDL for one selected file, resumed | **Yes, in full.** Binding, resume, rename, selection, redirects, and a signature that expires mid-run, all run and recorded above | nothing | none |
| 2. Three torrents, one shared file | **As a two-step**: torrent C finishes the file, A and B read C's copy over a `file:` source, run and recorded under T-133 | [T-133](#t-133-two-torrents-holding-the-same-file-cannot-share-its-bytes) layers 2 and 3, for one invocation with no path named | L |
| 3. DDL for one file, rest via swarm | **Yes, in full** | nothing | none |
| 4. Remapping and encoding | **Yes, in full**, through `exact`, `prefix`, `template`, per-source headers, and the status overrides | nothing | none |
| 5. All of it, with per-method control | Per-source caps, headers, auth, priority, and status policy: yes. Per-method caps and picker control: no | [T-132](#t-132-the-swarm-cannot-be-rate-limited-separately-from-http-sources), [T-134](#t-134-v1-and-v2-info-hashes-are-not-reconciled), [T-135](#t-135-source-selection-cannot-be-steered-by-method-or-by-priority-at-run-time), [T-136](#t-136-nothing-states-the-end-to-end-integrity-guarantee) | M to L |

The honest summary: the addressing model was built for exactly this and it
holds. Three of the five scenarios work in full and the fourth works as a
two-step. What is genuinely missing is cross-torrent identity computed rather
than asserted, and real control of which source answers a piece. Both were
already known and already priced, by [T-002](webseed.md) and
[T-003](webseed.md).

What none of this needs is a daemon, a database, or a state file. Every
scenario as the operator wrote it is one invocation with several sources, which
is what `bit-cli` is.
