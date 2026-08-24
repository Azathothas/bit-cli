# Progress

**Read this first.** It is the only thing the kickoff prompt tells a session to
read, so everything that changes from session to session is here: the baseline,
what the last session did, and the work order. The prompt carries none of it, by
[RULES.md](RULES.md) section 3.

It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
Every entry, one line each: [INDEX.md](INDEX.md).
Orientation for an agent: [`docs/AGENTS.md`](../docs/AGENTS.md).

> **The shape this file must keep**, from [RULES.md](RULES.md) section 2 step 2:
> the state line with the session's start instant in ISO 8601 UTC, the measured
> baseline with the CI run named by id, the entry counts, what the session did,
> what is in progress, **Start here next session** as an ordered list with entry
> ids and corpus sources, and open questions for the operator.
> `scripts/session-report.ps1` prints the numbers; do not count them by hand.
>
> `scripts/check-todo.ps1` checks most of that shape now, and `scripts/gates.ps1`
> runs it, so a missing section or a stale count fails a gate rather than a
> review. [RULES.md](RULES.md) section 5, "The record".

---

## Before typing a `bit-cli` flag, read `man/bit-cli.json`

`man/` holds the whole command surface, generated and committed: `bit-cli.1` for
a terminal, `bit-cli.md` for reading, and **`bit-cli.json`, a CLIspec 0.3
document, for a program**. Every command, every flag, the values it accepts, its
default, and every exit code with whether a retry could succeed.

It cannot go stale: `cargo test -p bit-cli --test man_is_current` fails until it
is regenerated with `pwsh -NoProfile -File scripts/check-man.ps1 -Fix`.
[`docs/man.md`](../docs/man.md) says what each field carries.

**That rule has been paid for twice**, both times by guessing rather than
reading: `create --tracker` does not exist and the flag is `--announce`, and a
scope selector is `SELECTOR=URL` rather than `URL=SELECTOR`. Both cost a run
that exited 2.

## Two things are settled and are not to be raised again

**Nothing in `patches/` is ever offered upstream, and this repository is the
only one an agent may write to.** [RULES.md](RULES.md) section 6 carries the
first and section 6a the second. `patches/UPSTREAM.md`'s `Upstream:` field
answers "could a release retire this patch on its own?" and nothing else.

**The six hour soak is run by the operator, in a foreground terminal.** No agent
session lasts six hours, and a session ending kills the process it started. The
command is under "Start here next session"; a session's job is to read the CSV
the operator's run leaves behind, not to start one.

## One decision was reopened, and it is section 6's iroh line

The operator's ruling: `bit-cli` will be BEP and RFC compliant, and will not
limit itself to BEPs and RFCs written long ago, because NATs and heavily
censored networks are everywhere. [RULES.md](RULES.md) section 6 is rewritten,
the retired paragraph is in `reference/HISTORY/RULES-section-6-iroh.md`, and
[T-238](peers.md) carries it.

**The follow-up ruling went further than the recommendation.** Relays are in
scope, several of them rather than one, ranked by how widely deployed the
provider is. That makes the protocol choice first and the vendor choice second,
and the protocol is TURN, RFC 8656, because it is the only relay protocol with
more than one provider. Speaking a relay protocol is not the same as taking the
`iroh` crate, and that refusal is unchanged: it is refused because BitTorrent
has nowhere to put a node id, not because of its size.

**Decision 7.4, no daemon and no RPC, was not reopened** and this session did
not treat it as reopened. [T-243](phase-c.md) is the draft that collides with
it, and it says so in its own first paragraph.

## State

- **Last session:** 2026-08-24T14:20:49Z, unattended, and it worked the
  entries rather than filing them. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.

  It wrote the plan down before starting, per [RULES.md](RULES.md) section 1
  step 4, and the plan was the work order's item 3 with the order inside it
  worked out rather than taken: [T-249](metainfo.md), then
  [T-246](cli-surface.md), then [T-247](cli-surface.md). It held. The operator
  then asked mid-session for at least four more effort-S entries, which is
  where [T-255](cli-surface.md), [T-254](webseed.md),
  [T-252](cli-surface.md) and [T-033](performance.md) come from.
- **Tests:** 1,359 passing, 0 failing, up from 1,312. Plus **149** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run. `vendor/` is untouched.
- **Gates:** clean, on rustc 1.98.0. A default run prints **nine**: `text`,
  `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-two** jobs. Green at run **32751764935**, against commit
  `e460bc5`, which is this session's last push. Two runs this session, both
  green: the middle commit was made with `-NoPush` and pushed with the last.

```bash
gh run list --limit 1
```

- **Soak:** no run in flight, and nothing new was measured. The last one is
  still `bench/soak-20260823T154716064Z`, which reached six hours over a
  workload that stopped after 78 minutes. That is [T-232](memory.md) and it is
  the first thing under the work order.

```bash
pwsh -NoProfile -File scripts/soak.ps1 -ReadCsv bench/soak-20260823T154716064Z.csv
```

- **Entries:** 204 items. 33 open, 2 partial, 0 blocked, 158 done, 11 deferred
  to Phase C. 158 of 193 workable done, 35 left.
- **Tree:** 99 Rust files, 60,182 lines of code, 15,720 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. [`reference-map.md`](reference-map.md) carries the
  licence per tree and where the determination came from. Nothing was mined
  this session and nothing was read from it: every entry was about this tree's
  own surface.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  Untouched.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Seven entries closed and one filed.** Three came from the work order's item
3 and four from the operator asking mid-session for at least four more effort-S
entries. Nothing was mined from the corpus and nothing needed to be: every
entry was about this tree's own surface.

**The order inside item 3 was worked out rather than taken.**
[T-249](metainfo.md) went first because it builds `bit-cli tree` and
[T-246](cli-surface.md)'s acceptance names `tree` as the command a typo has to
be corrected to. T-246's own example used `bit-cli tree one.torrent` as the
subcommand that does not exist. It does now.

### [T-249](metainfo.md), P3: `bit-cli tree`, and a span that does not mean what the entry said

The same `Layout` `files` reads, rendered as a tree, with each directory rolled
up to its size, its file count, and the pieces it spans.

```
PATH                   SIZE      FILES  PIECES
padded/                2.49 KiB  3      0-2
|-- disc 1/            1.95 KiB  2      0-2+
|   |-- lossless/      1.46 KiB  1      0-1+
|   |   `-- a.flac     1.46 KiB         0-1+
|   `-- notes.nfo      500 B            2-2
`-- .pad/              548 B     1      1-1+
    `-- 548 (padding)  548 B            1-1+
```

**The approach's reason for wanting the piece range is not true of the piece
range.** It said the span tells you whether a subtree can be fetched without
touching the rest. It does not: a piece straddling a boundary belongs to both
sides. `shared_pieces` sits beside the span and answers it, and the `+` is what
says so in the text form. `notes.nfo` is the one row without one, because the
padding in front of it pushes it onto a piece boundary.

**The acceptance's IBM437 clause needed a second condition**, not just
`--color`. `Env::out_is_unicode`: on Windows `GetConsoleOutputCP() == 65001`,
elsewhere a UTF-8 locale, asked only when stdout is a terminal because a file
takes the bytes verbatim. This machine's console code page is **437**.

### [T-246](cli-surface.md), P2: the first usage error a source has ever produced

| input | before | now |
| --- | --- | --- |
| a directory | 4, "Access is denied. (os error 5)", `io_kind: PermissionDenied` | 2, "is a directory, not a .torrent", and `bit-cli create` |
| `bit-cli tre album.torrent` | 4, cannot read a file called `tre` | 2, "is not a command", and `bit-cli tree` |
| `ftp://host/x.torrent` | 4, "volume label syntax is incorrect. (os error 123)" | 2, "is not a scheme this reads", and the forms that are |

The directory message is this tree's rather than the operating system's, which
is what makes it identical on both platforms, and nine call sites that read a
caller-supplied torrent path all go through the one function that produces it.
Three of the typo check's four conditions exist to keep a real file out of the
branch: `./tre` and `tre.torrent` are paths, a torrent named `tre` is
downloaded, and `quuxly` is a missing file rather than a guess.

### [T-247](cli-surface.md), P2: a dry run counts only what it took

It says what it did not do, and prints `0 so far` rather than nothing, because
a `--web-seed` on the command line and a Metalink's mirrors are real counts a
dry run does know. The `--json` shape is untouched: it always said the torrent
had not been read, through three nulls.

### [T-255](cli-surface.md), P2: filed and closed, and it was found by reading a diff

`BIT_CLI_UPDATE_SCHEMA=1` deleted **130 lines** of hand-written prose from
`docs/schema.md` while T-249 was adding a document kind, including the only
committed measurement of what seven PowerShell redirection forms do to
non-ASCII output. Both gates passed on the truncated file, measured by
stripping the tail again and running each one unpiped:

| check | on the truncated file |
| --- | --- |
| `cargo test -p bit-cli --lib schema` | exit 0, 11 passed |
| `scripts/check-docs.ps1` | exit 0, "everything resolves" |

Regeneration carries across every `##` section the generator does not produce
now, matched by heading line so it is idempotent and cannot duplicate a
generated one. Run twice, `git diff --stat docs/schema.md` prints nothing both
times.

**The file's own note was describing a writer that stopped existing at T-158.**
It said regenerating is lossy and told a reader to put back rows it removed;
rows have been unioned since T-158. What is true is narrower and was measured:
a row taken out **by hand** that no run produces does not come back.

### [T-254](webseed.md), P2: the headers that say a mirror was cached

`sources[].headers`, an allowlist of twelve, filled from the `HeaderMap` the
probe already had. Nothing is requested twice.

**The acceptance names Cloudflare and this was run against Fastly**,
deliberately: RULES.md section 5 lists three real mirrors a test may use and
`dl-cdn.alpinelinux.org` is the CDN-fronted one. `x-cache` is Fastly's spelling
of `cf-cache-status` and both are on the list.

```
  x-cache            HIT, HIT
  x-served-by        cache-ams-eham8680082-AMS, cache-bom-vanm7210091-BOM
  age                8
```

The other half is in-process, because a claim about what is **not** reported
cannot rest on what one origin happened to send: `FileServer::start_cdn` sends
four headers the report keeps and two it must drop.

### [T-252](cli-surface.md), P3: `--stats`, and a disk half that was plumbing

`--stats` is global and implemented in `Renderer::emit`, the one place a
document becomes text, so every report that exists and every report added later
carries it. Paths are the ones `docs/schema.md` names.

**The entry called the disk totals the one part that is a measurement. They are
not.** `StorageMetrics` has counted `write_bytes` and `write_nanos` on every
run since T-018 and `Engine::storage_counts` already exposed them. What was
missing was a field on the document. Four fields rather than two, because
`write_ops` over `write_calls` is the coalescing factor T-018 exists to move.

### [T-033](performance.md), P3: three aria2 aliases, taken and warned about

`-x`, `-s` and `-k` ship. **The man page went first**, per T-198: the flags were
declared, `check-man.ps1 -Fix` run, and `man/bit-cli.json` read back before any
wiring existed. That is what caught the sentence worth catching, which is that
`-s` has to say it is the same knob as `-x`.

```
warning: -x caps concurrent requests per source, not per server: -x 4 with two sources on one host is 8 requests to that host.
warning: -x and -s are one setting here, so -x 4 -s 16 is 16 concurrent requests per source rather than 64.
```

The second fires only when the two differ, so `-x 8 -s 8` gets one line.

### What the reviews found, and one thing they did not

**Review 1, every claim against the code it cites.** Six errors, all in this
session's own writing. Two counts off by one, nine call sites written as eight
and ten tests as nine; an ordering claim that is true of the grouping and not
of the row order, in a doc and in the code comment it came from; two line
citations made stale by the session's own later edits; and one claim that
named `loopback-fileserver` for a run that used a plain Python HTTP server.

**Review 2, a cold read.** `docs/examples/inputs.md` carried a section titled
"Everything on this page exits 4" that T-246 had just made false, and a
paragraph telling a reader to read the JSON instead of the text for a dry run
over a URL. Both rewritten from runs.

**And one the gates caught rather than a review, for the fourth session
running.** A `\n` sent through a heredoc into Python arrived in a Rust source
file as a real newline inside a char literal. RULES.md section 5 describes that
exact failure and it still happened; every patch after it was written to a file
and run with `python <path>`, which is what the rule says to do.

## In progress

Nothing is half-written. All seven entries closed complete, with the acceptance
run and its output recorded in the entry.

- **[T-253](cli-surface.md)** is still `partial`. One of the three things it
  named is done as a side effect of [T-254](webseed.md): `webseed_test`'s
  description now carries the whole reported header set. The two fixtures are
  not, and the TLS one is the reason: a self-signed certificate needs a
  certificate generator this tree does not have, and pointing the generator at
  a real HTTPS host is refused by its own acceptance, which says "on a machine
  with no network".
- **[T-164](peers.md)** is still `partial`, untouched.
- The entries the last sessions left open are untouched: [T-232](memory.md),
  [T-224](memory.md), [T-233](peers.md), [T-237](trackers.md),
  [T-239](peers.md), [T-240](dht.md), [T-241](metainfo.md),
  [T-101](bep-coverage.md), [T-102](bep-coverage.md), [T-168](bep-coverage.md),
  [T-244](cli-surface.md), [T-248](metainfo.md), [T-250](cli-surface.md),
  [T-251](trackers.md).
- **[T-243](phase-c.md)** is deferred and the operator has deliberately not
  ruled on it. Do not raise it.

## Start here next session

**The shape of the work order is the operator's, and it has not changed.** Not
priority first. Clear small entries so the open count comes down, then take the
bigger ones a **category at a time**. The counts are derived from the rows:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Re-measure the baseline rather than trusting the one above**, which is
   [RULES.md](RULES.md) section 1 step 5. Read the run this session's last push
   started: the CI line above names it.

```bash
gh run list --limit 1
```

2. **The soak, and it goes before anything else on the operator's
   instruction.** No run is in flight and none was started this session. **One
   run closes two entries** and it is the operator's to start in a foreground
   terminal: `-Leechers 4` is the different leech rate [T-224](memory.md) has
   left, and the listener check is what [T-232](memory.md) needs to say whether
   the seeder stopped answering or the leechers stopped calling.

   Kill the strays first, or two runs share a tracker:

```bash
pwsh -NoProfile -Command "Get-Process bit-cli,loopback-tracker,loopback-churn -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\.tmp\*' } | Stop-Process -Force"
```

```bash
cargo build --release --bins --examples
```

   Then **print this in chat for the operator** and do not start it inside a
   session:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -Leechers 4 -ListenerCheck 60s -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

3. **[T-241](metainfo.md), P2, and it is bigger than its `S` says.** The entry
   reads as one flag, `magnet --output`. It is not: `bit-cli magnet <magnet>`
   parses and reports without touching the swarm, measured this session, so
   writing the resolved metainfo means giving that command the peer, tracker
   and DHT plumbing `download` has. The entry's own Prove section wants an
   interop case against `aria2c` on top. Re-estimate it before starting, and
   correct the effort under the entry rather than over it.

4. **[T-237](trackers.md), P2, effort S**, and the two HTTP halves are cheap:
   `--redirect-announce` and `--fail-announce` on `loopback-tracker`, then two
   judged cases in `scripts/check-announce.ps1`. The UDP half is the reason it
   is S rather than XS and it is a BEP 15 connect and announce over a datagram
   socket in the same example binary.

5. **[T-251](trackers.md), P2, effort M.** `trackers` is the one command left
   that refuses a URL its own help offers, and its refusal names an info hash
   the URL carries. The source half is a few lines now that
   `source::resolve_source` exists and `read_torrent_file` is the one door to a
   local file; the twelve-knobs half is the rest of the entry.

6. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

7. **[T-244](cli-surface.md) and [T-250](cli-surface.md)**, which are cheaper
   than they were. T-244's ruling is static extraction with a browser-shaped
   header set and a `--render` opt-in. T-250 wants to report how an input was
   resolved, and `Kind::classify` now produces three distinct refusals, so
   there is something to report.

8. **The three entries that were ruled on and are still work.**
   [T-227](memory.md) is a throughput curve then a flag.
   [T-242](performance.md) is two sweeps from `scripts/bench-leech.ps1`.
   [T-234](peers.md) and [T-238](peers.md) are the two large ones and both need
   [T-239](peers.md) first. T-234 is cheaper than it was: `--as-client` is a
   second value for `bit_cli_core::peer_id::CLIENT_CODE` and its version
   characters, and that module is the only place either is read from.

9. **Then the category pass, and `bep-coverage.md` is still first.**
   [T-101](bep-coverage.md) is open on a latency measurement loopback cannot
   produce, which [T-239](peers.md) is the prerequisite for.
   [T-102](bep-coverage.md) and [T-168](bep-coverage.md) are the untouched two,
   then `dht.md`.

**Corpus sources the list above wants**, all on this machine and none needing a
fetch: `reference/RESEARCH.md` section D has one row per open entry; entries 23
to 29 for [T-234](peers.md); entries 30 to 37 for [T-238](peers.md) and
[T-239](peers.md); and `reference/README.md`'s "The 2026-08-24 trees" section,
which carries the actual code lines. **All of it is a read.**

## Open questions for the operator

**None.** Nothing this session did needed a ruling, and the one ruling it acted
on, [T-033](performance.md)'s, was given before it started.

**Three things to be aware of rather than to decide.**

**[T-253](cli-surface.md) cannot close without a certificate generator.** Its
remaining half needs the loopback file server to speak TLS, its acceptance
forbids the network, and nothing in this tree can make a self-signed
certificate: `rustls` and `tokio-rustls` are workspace dependencies already and
`rcgen` is not one. The options are a new dependency or a checked-in test
certificate that expires. Nothing was decided.

**Twelve repositories in `TheDancingDeveloper-org`** redistribute Apache-2.0
code under MIT with no licence file and no attribution. Nothing was said to
anybody, by [RULES.md](RULES.md) section 6a, and it is in `RESEARCH.md` entry
40.

**One dependabot pull request is open**,
`dependabot/github_actions/github-actions-b4f5548579`, with its own CI run
green. It was not taken, because a dependency bump is a change to the build
rather than to the work this session was given.

## Behaviour changes worth the operator's eye

**Three inputs that exited 4 now exit 2.** A directory, a URL under a scheme
this tree does not speak, and a bare word one edit from a subcommand with no
file of that name. A script branching on 4 to mean "this source failed" sees 2
for those three, which is the point: 2 says no retry and no other mirror will
help. [`docs/exit-codes.md`](../docs/exit-codes.md) carries the rule.

**Three aria2 short flags are now taken**: `-x`, `-s` and `-k`. A script that
was passing them and getting exit 2 gets a download and up to two warnings on
stderr. Nothing that already worked changes: all three are spellings of flags
that existed.

**A `webseed test` report is wider.** Up to twelve response headers per source
where there was one. None of them can carry a credential by construction, and a
header named with `--web-seed-report-header` that could is printed as
`<redacted>` unless `--no-redact` is given.

The peer id change from the session before this one is unchanged and still
worth knowing: every peer id `bit-cli` emits is `-CL0200-` now.
