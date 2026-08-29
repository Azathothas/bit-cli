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
session lasts six hours, and a session ending kills the process it started. A
session's job is to read the CSV the operator's run leaves behind, not to start
one. A short soak is a different thing and a session may run one; this session
ran none, because the one entry it worked on is not about a long run.

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

- **Last session:** 2026-08-29T04:24:07Z, unattended, and a **one-off**: the
  kickoff carried its own work order rather than taking this file's, per
  [RULES.md](RULES.md) section 3. The whole session was
  [T-244](cli-surface.md), the only `L` open in `cli-surface.md`. The ordinary
  work order resumes under "Start here next session" below.
  The duration is not restated here: `scripts/session-report.ps1` derives it
  from the instant above, and a duration written down twice is a number two
  documents disagree about.

  **The plan was written before starting**, per [RULES.md](RULES.md) section 1
  step 4, and it was the kickoff's eight tasks in its order. **It held for the
  first four and then the operator changed it mid-session**: the premise
  measurement came back saying no page needed impersonation, and the ruling was
  that the sample is too narrow to act on and the browser-shaped fetch is to be
  built rather than carried as a contingency. Everything after task 4 was
  re-ordered around that.
- **Tests:** 1,425 passing, 0 failing, up from 1,370. Plus **153** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run. `vendor/` did not move this session.

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **Gates:** clean, on rustc 1.98.0. A default run prints **ten**: `text`,
  `eol`, `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-three** jobs, one more than last session:
  `Page extraction and fingerprint` is new and runs both of this session's
  acceptance scripts. Green at run **33237268599**, against commit `644f473`.
  Two runs this session. The first, **33236102927** against `b18b834`, failed
  on one job, `Third party notices`, because `THIRD_PARTY.md` still named
  `chacha20 0.10.1` after the lockfile moved to `0.10.2`; the review section
  below carries it.

```bash
gh run list --limit 1
```

- **Soak:** nothing ran this session. The one entry it worked on is not about
  a long run.
- **Entries:** 208 items. 28 open, 4 partial, 0 blocked, 165 done, 11 deferred
  to Phase C. 165 of 197 workable done, 32 left.
- **Tree:** 105 Rust files, 63,707 lines of code, 16,651 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. Nothing was mined this session and nothing was read
  from it. The research this session worked from is **not** part of it: it was
  this operator's own prior work under `.tmp/`, and by the kickoff's own
  instruction it does not enter `reference/`, `RESEARCH.md` or
  `reference-map.md`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **32 patches**
  across twenty-two sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  Unchanged: nothing under `vendor/` moved.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**One entry advanced, open to partial, and it is the largest thing in
`cli-surface.md` by effort.** Two pushes, two new acceptance scripts, one new
generator, one new loopback fixture, one new library module for extraction and
one for finding a browser.

### [T-244](cli-surface.md), P2, `L`: a web page is a source now

**The static tier ships.** `crates/bit-cli-core/src/page.rs` extracts every
`href` whose path ends `.torrent` and every `magnet:` URI, with the anchor text
beside each, resolved against the document and any `<base href>`. One function
over an HTML string, so the rendered tier can change where the HTML came from
and nothing else.

One link resolves and the run continues. Several are named with their anchor
text and refused, per the operator's ruling that a page is reported rather than
guessed at. Zero names `--render` without implying a browser is installed.
`--page-select` narrows it. All nine commands that read a source get it,
through the one door `resolve_source` already was.

**A page is told from a `.torrent` by attempt and fall back**: the body is
parsed as bencode first and only asked whether it is markup when that fails. A
metainfo begins `d`, so a mirror serving a real torrent as `text/html` is still
read correctly. One hop and never two.

**The parser choice is measured**, four candidates each resolved and checked on
toolchain 1.88: a hand-written scanner adds **0** packages, `tl` 1, `lol_html`
**44**, `scraper` **57**.

### The proving ground, which is what makes the tier falsifiable

`scripts/make-page-fixture.ps1` emits six levels of escalating difficulty and
four acceptance cases, each with the correct extraction beside it as JSON,
carrying **two** lists: what the static tier must find and what a browser must.
`scripts/check-page-extract.ps1` serves them through `loopback-fileserver` and
compares. **Ten of ten pass**, and the number that justifies `--render` is

| level | static | rendered | only rendered |
| --- | --- | --- | --- |
| L4 script | 1 | 7 | **+6** |
| L5 hostile | 0 | 2 | **+2** |

with **zero** difference on levels 0 to 3, where a difference would be an
extractor defect rather than a property of the page.

### Both halves of the entry's own Acceptance hold, run

A page with one link resolves under `bit-cli info`, exit 0. A page with two of
each exits 4 naming all four with their anchor text. And the header set and the
TLS fingerprint are asserted against a recorded capture rather than eyeballed:
`fingerprints/bit-cli-browser.json` and `fingerprints/bit-cli-plain.json`,
checked by `scripts/check-fingerprint.ps1`, in CI.

### What the measurements said, including the two that disproved a premise

**Every page a plain client asked for was served.** `scripts/check-page-fetch.ps1`
fetched fifteen named pages, robots.txt honoured, one `GET` each: **15 served,
0 blocked**. Committed as `bench/page-fetch-20260829.json`. **The operator ruled
that this does not retire the impersonating tier**, and the reason is sound:
fifteen friendly distribution pages are not the population, and two of them sit
behind Cloudflare already.

**An off-host link is a match, and the work order called it a decoy.**
`kali.org` serves its download page from `www.kali.org` and all **113** of its
torrent links sit on `cdimage.kali.org`. A same-host rule returns nothing there.

**An unquoted `href` is not exotic, and the first count missed 113 links.** The
first run of the fetch measurement used a quoted-only pattern and reported
**0** torrent links on that same page. All three HTML5 framings are read now.

### What this client actually looks like on the wire

Captured with the oracle this session put in the tree, off the wire rather than
out of the code:

| | JA4 |
| --- | --- |
| before | `t13i1010h1_61a7ad8aa9b6_3fcd1a44f3e3` |
| after | `t13i1010h2_61a7ad8aa9b6_3fcd1a44f3e3` |
| Chrome | `t13i1515h2_8daaf6152771_806a8c22fdea` |

**One character moved and that is the honest summary.** `reqwest` gained
`http2` so ALPN offers `h2`. Ten ciphers and ten extensions is what `rustls`
offers against Chrome's fifteen, and no header set changes a `ClientHello`.
The header **set** is Chrome's; the **order** is not, and `reqwest` cannot
express one.

### Whether `impit` can enter this tree, which the survey could not know

| question | answer |
| --- | --- |
| MSRV 1.88 | **passes**, 289 packages |
| `x86_64-pc-windows-msvc` with `+crt-static` | **builds**, 8.59 MiB |
| `scripts/check-static.ps1` on it | **passes** |
| apify's `rustls` fork at this workspace root | **the whole workspace checks**, vendored `librqbit` included |
| the musl targets | not measurable here, CI is the instrument |

The MSRV answer is the one that mattered: a bump would have been a decision
above a `TODO/` item, and there is none. The survey's `h2` patch defect
reproduced twice, independently, on a platform the survey never tested.

### The tooling that moved out of `.tmp/` and into the tree

- **`crates/bit-cli-core/examples/loopback-tlsprobe/`**, the oracle, as a
  `loopback-*` fixture like the other three, with `test = true` so its
  `ClientHello` parser and HPACK decoder are in `cargo test --workspace`. It
  gained `--plain` and a golden-manifest reader here.
- **`crates/bit-cli-core/src/browser.rs`**, the resolver `--render` needs and
  no CDP crate provides, unit tested fifteen ways with no browser present.
- **`fpsync.py` was not ported and `ja3.py` was not copied.** The first is
  answered by `check-fingerprint.ps1`, which asserts against what a browser
  emits rather than against a version number; the second is superseded and its
  JA4 is wrong. The probes under `probes/` stayed throwaway and their
  measurements are in the entry.

### What the reviews found

**Review 1, every claim against the code it cites.** Four things.

- **Two citations in T-244's own Problem were eight lines stale**, `source.rs:68`
  and `:32`, and are `:86` and `:40`. `check-todo.ps1` could not catch them: it
  checks a short citation's line only when the prose names a symbol occurring
  exactly once, and `Kind::Url` occurs four times.
- **The Problem's sentence was imprecise in a way that mattered.** Line 86 does
  not map every `http(s)` string to `Kind::Url`: it reads the extension off the
  **path** and returns `Kind::MetalinkUrl` for a metalink. A page arrives as
  `Kind::Url` because it is not a metalink, not because nothing looks.
- **Fourteen citations drifted twice**, seven each time, as `cli.rs` grew by 48
  and then 31 lines. The `record` gate caught both.
- **The Acceptance's first sentence reads two ways** and only one agrees with
  the Approach. "One `.torrent` link and one magnet link" cannot mean one page
  carrying both, because the Approach refuses a page yielding more than one. It
  is read as two pages and both are cases, with a third for the page carrying
  one of each.

**Review 2, a cold read.** The classifier in `check-page-fetch.ps1` reported
two pages blocked that were both served in full, because Cloudflare injects
`/cdn-cgi/challenge-platform/` into pages it serves **normally**. Challenge
markers and advisory markers are separate lists now, and the corrected count is
15 of 15.

**Review 3, adversarial.** Under "In progress" below, in full.

**Review 4, the research's own claims re-verified.** Under "In progress".

**Review 5, the abuse read.** Under "In progress".

**Review 6 was CI's, and it paid again.** `Third party notices` went red on the
first push because `THIRD_PARTY.md` named `chacha20 0.10.1` while `Cargo.lock`
said `0.10.2`. The version moved because `chacha20 0.10.1` was **yanked
upstream during this session**: the `deny` gate passed at 04:28 and failed at
05:12 on the same tree.

**And one thing nothing caught but a restore.** A `[System.IO.File]::WriteAllText`
with a **relative** path, from a script that had `Set-Location`d into `.tmp/`,
overwrote this repository's root `Cargo.toml` with a throwaway probe manifest.
.NET's current directory is not PowerShell's. Nothing was lost because the file
was committed, which is luck rather than a safeguard, and
[RULES.md](RULES.md) section 5 carries the rule now.

## In progress

- **[T-244](cli-surface.md) is `partial`** and the entry names what is left
  precisely, in six numbered items. The two large ones are the impersonating
  `ClientHello`, which needs `impit` vendored, and `--render`'s driver. Both
  wait on one operator decision, below.

**Review 3, adversarial: what would make this the wrong implementation?**

1. **A tag scanner is not a parser and a real indexer may break it.** Conceded
   in part. It has no tree, so a page relying on implied end tags or on
   mis-nesting recovery could yield a different anchor text. Every level of the
   proving ground and all fifteen fetched pages parse correctly, and the URLs,
   which are what get fetched, come from attributes rather than from structure.
   The risk is confined to anchor **text**.
2. **The proving ground is mine, so it cannot surprise me.** This is the
   strongest objection and it is why the fifteen real pages exist beside it.
   One of them found something the proving ground did not generate: an
   unquoted `href`. Another still is not covered, item 6 below.
3. **What the proving ground does not generate that a real indexer does.**
   `linuxtracker.org` publishes every torrent behind
   `index.php?page=downloadcheck&id=<hex>`, so nothing in the URL says it is a
   torrent. That is a real gap, it is in the entry, and it is **not** the same
   gap as `--render`: no amount of script execution makes that URL end
   `.torrent`.
4. **`<noscript>` is skipped, and that is a choice that could be wrong.** A
   browser with script on does not render it, so skipping it is what keeps the
   two tiers agreeing. A page whose only torrent link is in `<noscript>` yields
   nothing from either tier. Nothing measured says such a page exists.
5. **The browser profile makes `bit-cli` lie about what it is, by default.**
   True, and it is the operator's ruling. It is confined to the source document
   fetch: a web seed still says `bit-cli`, because impersonating at a mirror
   somebody configured buys nothing and hides who is asking.
6. **The fingerprint golden could be a golden of the wrong thing.** It was
   captured on Windows and asserted on Linux by CI on the first run, so it is
   at least platform-independent. It is not a browser's fingerprint and the
   entry says so in the same breath as recording it.

**Review 4, the research's claims, re-verified and still asserted.**

Re-run here, and all three held: `impit` static-links with `+crt-static` and
passes `check-static.ps1`; its JA4 is `t13i1515h2_8daaf6152771_806a8c22fdea`
with the Chrome cipher hash; and the `h2` patch is declined with a **warning**
rather than an error, reproduced twice.

**Still asserted and never tested here**, inherited from the survey: `wreq` and
`koon` failing to static-link, the BoringSSL classification of the four
candidates that were never built, every star count, HTTP/3 and QUIC entirely,
`impit`'s Akamai HTTP/2 fingerprint being profile-invariant, and every claim
about macOS. `impit`'s musl builds are unverified anywhere, including here.

**Review 5, the abuse read, which is specific to this entry.**

- **A CAPTCHA is a refusal in the code and not only in the docs.** There is no
  retry, no backoff and no second request anywhere on the page path: one `GET`,
  and a non-success status is an error carrying the status. The only second
  fetch is the torrent a page named, and it is a different URL.
- **Nothing retries past a bot check**, because nothing retries at all here.
- **Nothing logs a credential or a cookie.** No cookie jar is built and
  `Set-Cookie` is never read: `reqwest`'s `cookies` feature is not enabled and
  the word appears in `crates/bit-cli/src/` in exactly two places, both the web
  seed report's **redaction** list, `cmd/webseed.rs:580` and `cli.rs:1459`. The
  fingerprint capture records header **names** only, by construction:
  `h2fp.rs:92` says so and skips every value, and the cleartext path takes
  `split(':').next()`.
- **`--render` cannot leave a browser running because it does not exist yet.**
  When it does, that is the first thing to test.
- **The fetching honoured `robots.txt` and stayed to one request per page.**
  `check-page-fetch.ps1` reads `robots.txt` per host, applies longest-match
  with Allow winning a tie, records a disallowed path as skipped without
  fetching it, and pauses between requests. Nothing was crawled and no page was
  fetched twice.
- **What this makes easier that it should not.** The header set makes `bit-cli`
  look like a browser to a log. That is the ruled design and its blast radius
  is one `GET` of a document the caller already named. It does not defeat a
  challenge, does not solve one, and does not retry into one.

## Start here next session

**The shape of the work order is the operator's and it has not changed.** Not
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

2. **[T-259](cli-surface.md), P3, `S`.** Still the smallest thing open and the
   cheapest to prove, and untouched for two sessions: compare the non-row lines
   of `docs/schema.md` for equality while keeping the row lines as containment,
   with the hand-written tail `carry_across` preserves exempt. The entry names
   both seams with their lines.

3. **[T-250](cli-surface.md), P2.** Cheaper again: `source_kind` gained a
   **sixth** value this session, `page`, and the resolution of a page to the
   link it named is exactly the "how it was resolved" this entry is about. The
   error path already carries `page_links`; what is missing is the successful
   one saying which link it took.

4. **[T-253](cli-surface.md), P2, `partial`, and its blocker dissolved this
   session.** It needed a self-signed certificate the tree could not make, and
   `rcgen` is a dev-dependency of `bit-cli-core` now, with
   `loopback-tlsprobe/main.rs` as the worked example of using it. Nothing
   stands between this entry and a loopback file server that speaks TLS.

5. **[T-251](trackers.md), P2, `M`, `partial`.** Unchanged: a `[[tracker]]`
   table in the file `--web-seed-config` reads, then the `[[peer]]` table after
   it. `scripts/check-announce.ps1` is where the case goes.

6. **[T-244](cli-surface.md), P2, `partial`, and it needs the ruling in
   "Open questions" before it is workable.** The entry's `Left:` list is six
   items and the first is the one that unblocks two others.

7. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

8. **The three entries that were ruled on and are still work.**
   [T-227](memory.md) is a throughput curve then a flag.
   [T-242](performance.md) is two sweeps from `scripts/bench-leech.ps1`.
   [T-234](peers.md) and [T-238](peers.md) are the two large ones and both need
   [T-239](peers.md) first.

9. **Then the category pass, and `bep-coverage.md` is still first.**
   [T-101](bep-coverage.md) is open on a latency measurement loopback cannot
   produce, which [T-239](peers.md) is the prerequisite for.
   [T-102](bep-coverage.md) and [T-168](bep-coverage.md) are the untouched two,
   then `dht.md`.

**Corpus sources the list above wants**, all on this machine and none needing a
fetch: `reference/RESEARCH.md` section D has one row per open entry; entries 23
to 29 for [T-234](peers.md); entries 30 to 37 for [T-238](peers.md) and
[T-239](peers.md); and `reference/README.md`'s "The 2026-08-24 trees" section.
**All of it is a read.** Nothing was read from it this session.

## Open questions for the operator

**One, and it decides how much of [T-244](cli-surface.md) is left.**

**Does the impersonating client enter this tree, and at what price?** Every
number needed to answer it was measured this session and is in the entry.
`impit` is admissible: MSRV 1.88 passes, Windows static-links, and apify's
`rustls` fork does not break the vendored `librqbit` trees. What adoption costs
is three things, and none is a defect:

- **Five upstream trees have to be vendored**, not forked. `deny.toml` sets
  `unknown-git = "deny"`, so the five apify git dependencies cannot enter as
  git sources, and [RULES.md](RULES.md) section 6a forbids the fork the survey
  recommends by name. `vendor/` with a series under `patches/` is the route,
  and it also carries the `h2` version fix that apify's own patch fails to
  apply.
- **`--cfg reqwest_unstable` becomes tree wide**, in all three
  `.cargo/config.toml` target blocks **and** the three `RUSTFLAGS` blocks in
  `ci.yml`, because `RUSTFLAGS` replaces rather than adds. That is the exact
  shape of [T-146](cli-surface.md).
- **Two `reqwest` majors end up in the graph.** A warning under
  `multiple-versions`, and a real size cost.

**The same ruling covers `--render`.** `chromiumoxide` is **136** packages and
brings `reqwest` 0.13 as well; `headless_chrome` is 143. Both are MIT or
Apache-2.0 and both check on 1.88. The question is whether either lands in
every build or behind a Cargo feature. The survey's own escape hatch is a
feature gate, and its review 5.3 warns the abstraction will leak, so it wants
prototyping rather than planning.

**A recommendation, since the entry is otherwise stalled on it:** vendor and
feature-gate, with the default build unchanged and a CI job that builds the
feature so it cannot rot. That keeps every release artifact exactly as it is
today and puts the cost on whoever asks for it.

**Three things to be aware of rather than to decide.**

**[T-253](cli-surface.md)'s blocker is gone**, which the kickoff asked to have
recorded. It needed a certificate generator and `rcgen` arrived this session as
a **dev** dependency of `bit-cli-core`, so it reaches no released binary and no
notice file. `about.toml` sets `ignore-dev-dependencies`.

**Twelve repositories in `TheDancingDeveloper-org`** redistribute Apache-2.0
code under MIT with no licence file and no attribution. Nothing was said to
anybody, by [RULES.md](RULES.md) section 6a. Unchanged.

**One dependabot pull request is still open**, number 6,
`ci(deps): bump taiki-e/install-action from 2.86.3 to 2.86.5`. Not taken again,
for the same reason as the last three sessions.

## Behaviour changes worth the operator's eye

**A URL that serves a web page is now a source rather than an error.** Every
command that reads a `SOURCE` will fetch a page, extract the torrents it links,
and resolve the one it finds. **A script that branched on exit 4 to mean "that
URL is not a torrent" will now get a torrent** where the page named exactly one.
Where it names several the exit code is still 4 and the message lists them.

**The fetch of a source document presents as Chrome by default.** Chrome's
header set, and ALPN offering HTTP/2 where it offered only HTTP/1.1 before.
`--page-client plain` restores the old behaviour, and
`--web-seed-user-agent` still wins over both. **A web seed is unaffected** and
still sends `bit-cli/<version>`.

**`bit-cli` now decompresses.** `reqwest` gained `gzip`, `brotli`, `deflate`
and `zstd`, so a document fetch advertises and accepts compressed responses
where it previously took them raw.

**Nine commands gained two flags**, `--page-select` and `--page-client`, under
a "Resolving a web page" heading.

**`fingerprints/` is a new top level directory**, holding what this client puts
on the wire. `scripts/check-tree.ps1` was told about it on purpose.
