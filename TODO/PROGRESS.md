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

- **Last session:** 2026-08-24T11:48:11Z, unattended, and it is working the
  entries rather than filing them. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.
- **Tests:** 1,298 passing, 0 failing, unchanged. No `crates/` source changed
  at all this session. Plus **149** in the vendored `rqbit` tree and **76** in
  `librqbit-utp`, which the workspace gates do not run. `vendor/` is untouched.
- **Gates:** clean, on rustc 1.98.0. A default run prints **nine**: `text`,
  `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-two** jobs. Green at run **32722156167**, against commit
  `31daa1d`. Two runs this session and neither was red.

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

- **Entries:** 203 items. 39 open, 2 partial, 0 blocked, 151 done, 11 deferred
  to Phase C. 151 of 192 workable done, 41 left.
- **Tree:** 97 Rust files, 58,058 lines of code, 14,899 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. [`reference-map.md`](reference-map.md) carries the
  licence per tree and where the determination came from. Nothing was mined
  this session; two crates.io manifests were read and are cited in
  [T-244](cli-surface.md) rather than in the corpus.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  Untouched.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**This section is a plan until the session ends, by [RULES.md](RULES.md)
section 1 step 4.** What is below the plan is the session before this one, kept
until this one rewrites it.

### The plan, written before the work

The work order below is followed in its own order. The soak is printed for the
operator and not started, per the standing instruction.

1. [T-245](cli-surface.md), P1, effort M, `crates/bit-cli/src/source.rs` and
   the six commands that call `load_local`. Four entries are blocked behind it.
2. [T-236](peers.md), P1, effort S, `crates/bit-cli/src/cmd/trackers.rs`,
   `crates/bit-cli/src/cmd/bench.rs` and wherever the one constant ends up.
3. [T-246](cli-surface.md), [T-247](cli-surface.md) and
   [T-249](metainfo.md), all effort S, which share one fixture.

Anything past that is taken from the work order in order.

---

Verification, then documentation. **Eleven entries filed, none of the open ones
worked on**, and eight operator decisions settled. The assignment was to take
eight ideas from the operator's brief, check each against the code by running
it, document what already works and file what does not.

### The eight ideas, and what each turned out to be

Every row was decided by running a command, not by reading. The command and its
output are in the entry.

| the idea | what is there today | filed |
| --- | --- | --- |
| scrape a `.torrent` or magnet out of a web page | nothing. `source.rs:68` maps every URL to "a URL pointing at a `.torrent`", and a page is handed to the bencode parser | [T-244](cli-surface.md) |
| smart detection between input types | most of it. Seven forms, classified by shape, and a local torrent's extension is not consulted at all | [`docs/examples/inputs.md`](../docs/examples/inputs.md) |
| a `diff` subcommand | half of it, filed under `files`. `--against` proves two torrents hold the same file by piece hashes | [T-248](metainfo.md), [`comparing-torrents.md`](../docs/examples/comparing-torrents.md) |
| a `compare` subcommand | refused as a second command, folded into `diff --by files` on the operator's ruling | [T-248](metainfo.md) |
| a `tree` subcommand | nothing. `files` is a flat list, and `bit-cli tree x.torrent` is read as a source named `tree` | [T-249](metainfo.md), [T-246](cli-surface.md) |
| show how an input resolved | nothing. Eleven `--trace` subsystems and none covers resolution | [T-250](cli-surface.md) |
| granular control per tracker, peer and web seed | a web seed has twelve knobs of its own. A tracker has one timeout for all of them | [T-251](trackers.md) |
| a `--stats` flag | the numbers are all in `--json` already. The text rendering reduces the process half to one line | [T-252](cli-surface.md) |

**Three defects fell out of checking rather than out of the ideas.**

[T-245](cli-surface.md), P1: `info`, `files`, `magnet` and `verify` all
document their positional as accepting an HTTP URL and all four refuse one.
`download` fetches the same URL and completes. It is the prerequisite for four
of the rows above, because an input cannot be treated as an abstract object
while it only resolves inside one command.

[T-246](cli-surface.md): `bit-cli info <directory>` reports
`Access is denied. (os error 5)` and `"io_kind": "PermissionDenied"`. Nothing
is denied. Reading a directory as a file is `ERROR_ACCESS_DENIED` on Windows
and `EISDIR` on Unix, so one input produces two different wrong explanations.

[T-247](cli-surface.md): `download --dry-run` over a URL prints `trackers 0`
for a torrent with one tracker, because a dry run does not fetch and nothing
counted. The `--json` form of the same run is correct and says `null`. The
renderer already has the right pattern three lines above the defect.

### Four pages, and the measurements behind them

[`s3-webseed.md`](../docs/examples/s3-webseed.md) is new and every number in it
came from a run against a real AWS S3 bucket, `noaa-goes16`, which is public
and reproducible. One request says the shape:

```
status 206, ranges yes, length matches, server AmazonS3
tls TLSv1_3 TLS13_AES_128_GCM_SHA256, alpn http/1.1
handshake connect 269ms, tls 267ms, ttfb 876ms, total 1414ms
```

Two concurrency sweeps behind it, at 4 MiB and at 256 KiB per request, which
disagree about the best concurrency and are both right: the per-request floor
is about 300ms whatever the chunk size, so the chunk size sets the ceiling one
connection can reach and concurrency is what lifts it. A full download of the
object over HTTP alone made **ten requests for ten pieces and fetched
2,410,916 bytes**, which is the payload to the byte.

[`cloudflare-webseed.md`](../docs/examples/cloudflare-webseed.md) gained the
Cloudflare half it said it did not have. `cdnjs.cloudflare.com`, one of the
most requested hosts on the internet, **cannot be a web seed**: it answers
`200` to a ranged `GET`, which `bit-cli` refuses with exit 6 and
"the server does not honour Range". Beside S3 the same page now carries a
latency table: 6ms connect and 82ms to first byte against 269ms and 876ms, and
`alpn h2` against `http/1.1`.

[`inputs.md`](../docs/examples/inputs.md) and
[`comparing-torrents.md`](../docs/examples/comparing-torrents.md) are the other
two, and they document what already works rather than what is planned.

**A redirect chain worth keeping.** `dl.min.io` is a MinIO server that
redirects twice, to GitHub and then to Azure Blob, and the final URL carries a
presigned signature that expires in an hour. One `webseed test` printed all of
it, including that the TLS report describes the **last** host rather than the
first, and refused the source on a length mismatch. It is the whole failure
matrix in one command and it is in the S3 page.

### The docs gate grew three checks, and each one found something

`scripts/check-docs.ps1` compared prose, links, flags and typography. It now
also compares **output fields against `docs/schema.md`**, which is generated
from what real runs wrote, so a page naming a field is checked against the tool
one step removed.

- A backticked path carrying `[]` has to be a row in the schema or the parent
  of one. That found `sources[].convictions` and `redials[]`, both real fields
  with `skip_serializing_if` that no schema-generating run has ever produced.
- Every key in a `json` fence has to be a field name the schema carries. That
  found `reasons`, which is `torrents[].renamed[].reasons` and is emitted by
  three commands.
- Every page under `docs/` has to be linked from somewhere, and every `T-NNN` a
  page names has to be an entry. 81 ids checked, all resolve.

Both new rules were proved able to fail by planting a defect and watching the
gate go red before it was removed.

**`docs/schema.md` gained fifteen rows, each from output a run produced.** Seven
under `sources[].tls`, three under `sources[].redirects[]`, plus
`sources[].server` and `sources[].resolved_url`, all from the S3 and MinIO runs
above; and four under `context.report.renamed[]`, from a torrent built by hand
with three unwritable paths. `../../pwned.txt` becomes `__/__/pwned.txt` for
two reasons at once, `escape` and `trailing-dot-or-space`.

That is [T-253](cli-surface.md), which is **partial**: the rows are right and
the mechanism that would have produced them is not built. Its acceptance is
that regenerating the schema on a machine with no network puts all fifteen back.

### The profile generator had the defect it exists to catch

`scripts/make-client-profile.ps1` takes `-Latest stable` and `-Latest beta`
now, sorting the tag list by parsed version rather than by the order GitHub
returned it, and refusing a prerelease that is behind the newest stable
release.

Adding that is what found the defect. **The generator hardcoded the fourth
character of both peer id prefixes as `0`, and both clients derive it:**

| | where | stable | beta | dev |
| --- | --- | --- | --- | --- |
| Transmission | `CMakeLists.txt:144-163` | `0` | **`B`** | **`Z`** |
| qBittorrent | `sessionimpl.cpp:1726`, from `QBT_VERSION_BUILD` | `0` | `0` | `0` |

So a Transmission beta announces `-TR410B-` and the generator would have
produced a `0`. qBittorrent is the other way round: the status never reaches
its peer id and always reaches its User-Agent, so `release-5.1.0beta1` is
`-qB5100-` with `qBittorrent/5.1.0beta1`. Both are derived from the tag now,
each behind a guard that asserts the construction, and the profile carries
`prerelease_visible` so a caller knows whether a peer can tell.

A `Test-Profile` gate runs on the finished object before anything is written:
eight byte Azureus prefix, the client's own two letter code, every version
character inside the alphabet, a User-Agent carrying the version it claims, and
a non-empty record of which files it came from. The self-test proves the gate
refuses each of those three ways and accepts a correct profile.

`scripts/check-client-profile.ps1` is the canary, and it is the same instrument
as `scripts/upstream-scan.ps1`: it derives both clients at both kinds and fails
when a guard fails, not when a version changes.

```bash
pwsh -NoProfile -File scripts/check-client-profile.ps1
```

It is deliberately **not** in `gates.ps1`. It needs the network, and a gate
that fails when a network is down is a gate people learn to ignore. It was
proved able to fail by mutating one guard pattern, which produced exit 1 and
named the file, before the pattern was restored.

### What the two reviews found

Both reviews found real things, which is the argument for doing them after the
work rather than instead of it.

**The first review checks every claim against the code or the path it cites.**
Six citations were wrong and are corrected: `source.rs:66` is 68, `:33` is 32,
`:172` is 189, `cmd/download.rs:3017` is 3018, qBittorrent's
`version.h.in:39-43` is 40-44, and Transmission's `session.cc:194-207` is
196-206. One claim was weaker than the truth: `rquest` and
`reqwest-impersonate` are not "stalled at 0.0.0", they have 152 and 62
published versions and **every one of them is yanked**, and `wreq`'s newest
published version is a `6.0.0` release candidate rather than the 0.16.0 stable
the dependency numbers come from.

**The second is a cold read for a document contradicting another document.**
Four things:

- The S3 page said `bit-cli` surfaces no response header while its own output
  shows `server AmazonS3`. `Server` is the one header that is carried, and both
  the page and [T-254](webseed.md) say so now.
- The Cloudflare page's opening said every command in it was run against a
  local origin, and its closing section now says otherwise. The opening was the
  stale half.
- `inputs.md` claimed exit 2 for an unrecognised source. **No input to a
  `SOURCE` argument produces a usage error**: every one exits 4, because the
  classifier's last rule is "treat it as a path". That was measured over four
  shapes rather than reasoned about.
- Which found the third case for [T-246](cli-surface.md):
  `bit-cli info ftp://host/x.torrent` is read as a relative filename and
  reports "The filename, directory name, or volume label syntax is incorrect".

**And one thing the gates caught rather than a review.** Writing these files
through a shell heredoc put a `0x08` and a `0x0C` into three of them, invisible
in an editor. `check-tree.ps1` names the file and the byte offset, and was
proved to still do so by planting one on purpose.

## In progress

Nothing is half-written. Every entry filed this session is filed complete, with
a priority, an effort, a `Source:` line and an acceptance command.

- **[T-253](cli-surface.md)** is `partial`: the fifteen schema rows are in and
  the fixture that would generate them is not.
- **[T-244](cli-surface.md)**, **[T-245](cli-surface.md)**,
  **[T-246](cli-surface.md)**, **[T-247](cli-surface.md)**,
  **[T-248](metainfo.md)**, **[T-249](metainfo.md)**,
  **[T-250](cli-surface.md)**, **[T-251](trackers.md)**,
  **[T-252](cli-surface.md)** and **[T-254](webseed.md)** are open and
  unstarted.
- **[T-033](performance.md)**, **[T-227](memory.md)**, **[T-234](peers.md)**,
  **[T-238](peers.md)** and **[T-242](performance.md)** carry a `Ruled:` block
  now and are no longer waiting on a decision. None of the five was worked on.
- The entries the last sessions left open are untouched: [T-232](memory.md),
  [T-224](memory.md), [T-233](peers.md), [T-236](peers.md),
  [T-237](trackers.md), [T-239](peers.md), [T-240](dht.md),
  [T-241](metainfo.md), [T-101](bep-coverage.md), [T-102](bep-coverage.md),
  [T-168](bep-coverage.md), and [T-164](peers.md).
- **[T-243](phase-c.md)** is deferred and the operator has deliberately not
  ruled on it. Do not raise it.

## Start here next session

**The shape of the work order is the operator's, from eight sessions ago.** Not
priority first. Clear small entries so the open count comes down, then take the
bigger ones a **category at a time**.

The counts are derived from the rows rather than from memory:

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
   left, and `-ListenerCheck 60s` is what [T-232](memory.md) needs to say
   whether the seeder stopped answering or the leechers stopped calling.

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

3. **[T-245](cli-surface.md), P1, effort M**, and it is first among the new
   ones because four other entries are blocked behind it. Four commands refuse
   the URL `download` accepts. `source.rs:189` `load_local` is the refusal and
   the split is named in the entry: a `resolve` that may fetch, and the
   existing local-only path kept for a caller that must not touch the network.

```bash
cargo run -p bit-cli-core --example loopback-fileserver -- --root .
```

4. **[T-236](peers.md), P1, effort S**, still the cheapest real thing on this
   list and untouched. `bit-cli` announces under two peer id prefixes and
   neither is its own; one of them is BitComet's. The check that found it
   prints the prefix beside its verdict, so the fix is verifiable in one run:

```bash
pwsh -NoProfile -File scripts/check-announce.ps1
```

   Pick the two character code against libtorrent's `src/identify_client.cpp`
   and `aquatic/crates/peer_id/src/lib.rs:100-120` before using it. The
   constant goes in one place and `SessionOptions::peer_id` is `pub`, so no
   patch to `vendor/` is needed.

5. **The three small correctness entries, all effort S**, and they share one
   fixture: [T-246](cli-surface.md) and [T-247](cli-surface.md) are both a
   rendering that says something untrue, and [T-249](metainfo.md) is a second
   rendering of a layout that is already computed. A torrent with a hostile
   path, an unwritable directory and a URL is the whole fixture set.

6. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

7. **The five entries that were ruled on and are now work rather than
   questions.** [T-033](performance.md) is three aria2 aliases plus a warning
   naming the per-server against per-source difference, with the man page
   written first per [T-198](cli-surface.md). [T-227](memory.md) is a
   throughput curve then a flag. [T-242](performance.md) is two sweeps from
   `scripts/bench-leech.ps1`. [T-234](peers.md) and [T-238](peers.md) are the
   two large ones and both need [T-239](peers.md) first.

8. **Then the category pass, and `bep-coverage.md` is still first.**
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

**None.** Every question the last session raised has been answered, and the
answers are written into the entries as `Ruled:` blocks rather than left here.

| question | ruling | where it lives |
| --- | --- | --- |
| NAT traversal | the ladder, **with relays**, several of them, ranked by how widely deployed the provider is | [T-238](peers.md) |
| client masking | all three points accepted as designed: honest default, mask always in the machine output, `--as-client` | [T-234](peers.md) |
| the three aria2 spellings | **take all three**, with a warning naming the per-server against per-source difference | [T-033](performance.md) |
| the web seed cache budget | measure, then ship a flag that defaults to today's behaviour | [T-227](memory.md) |
| how far a page scraper should go | static, with a browser-shaped header set and TLS fingerprint, and `--render` as a browser opt-in | [T-244](cli-surface.md) |
| where the resolution chain lives | an `--explain` flag on every command | [T-250](cli-surface.md) |
| `diff` against `compare` | one `diff` with `--by` modes; `compare` refused as a second command | [T-248](metainfo.md) |
| the request depth | run the sweep first | [T-242](performance.md) |

**The relay ruling went against the recommendation and is the one to read.**
The recommendation was no relay yet, because it needs somebody to run it and a
trust assumption to state. The ruling is relays, plural, ranked by provider.
[T-238](peers.md) carries what that changes: the protocol choice comes before
the vendor choice, and it is **TURN, RFC 8656**, because it is the only relay
protocol with more than one provider. Speaking it does not mean taking the
`iroh` crate, and that refusal is unchanged.

The trust assumption is written down there rather than implied: a relay learns
the pair of addresses it joins and how many bytes pass between them, and does
not learn the info hash or the payload while MSE is on. That sentence has to
appear beside the user-facing flag, not only in the entry.

**[T-243](phase-c.md), the user interface, was deliberately not asked.** The
operator deferred it. Do not raise it.

**One thing to be aware of rather than to decide.** Twelve repositories in
`TheDancingDeveloper-org` redistribute Apache-2.0 code under MIT with no licence
file and no attribution. Nothing was said to anybody about it, by
[RULES.md](RULES.md) section 6a, which is absolute. It is recorded in
`RESEARCH.md` entry 40 so that a later session reading `license = "MIT"` in one
of those manifests does not act on it.

**A second thing.** One dependabot pull request is open on this repository,
`dependabot/github_actions/github-actions-b4f5548579`, and its own CI run is
green. It was not taken, because a dependency bump is a change to the build
rather than to the documentation.

## One behaviour change worth the operator's eye

**`scripts/make-client-profile.ps1` produces a different peer id prefix than it
did**, for prereleases only. A stable release is unchanged and every value it
produced before is reproduced exactly. What changed is that a beta of either
client is now derived rather than assumed, and the old answer was wrong for
Transmission. Nothing a user runs is affected: the script is an instrument and
`--as-client` is not built.
