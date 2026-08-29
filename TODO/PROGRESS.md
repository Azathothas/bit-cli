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

- **Last session:** 2026-08-29T07:05:02Z, unattended, and a **one-off** for the
  second time running: the kickoff carried its own work order rather than
  taking this file's, per [RULES.md](RULES.md) section 3. The whole session was
  [T-244](cli-surface.md), which **closed done**. The ordinary work order
  resumes under "Start here next session" below and this file's shape goes back
  to normal with it.
  The duration is not restated here: `scripts/session-report.ps1` derives it
  from the instant above, and a duration written down twice is a number two
  documents disagree about.

  **The plan was written before starting**, per [RULES.md](RULES.md) section 1
  step 4, and it held: the kickoff's eight tasks in its order, vendoring first
  because everything else sat on it. The operator interrupted twice, both times
  about the same defect and both times correctly; the review section says what
  it was.
- **Tests:** 1,462 passing, 0 failing, up from 1,425. Plus **439** in the
  vendored `h2` library, **153** in `rqbit` and **76** in `librqbit-utp`, none
  of which the workspace gates run.

  **One of the 439 fails about one run in one under `--workspace` and is
  upstream's.** `proto::streams::recv::tests::clear_recv_buffer_caps_capacity_before_overflow`
  reproduces on a pristine `v0.4.19` checkout carrying only the two changes
  that make the tree loadable here, so it predates every patch in
  `patches/h2/`. It passes alone and it passes under `--lib`. Nothing was done
  for it and no entry was opened: it is not this repository's code and one
  flake in somebody else's test is not a measurement.

```bash
cargo test --manifest-path vendor/h2/Cargo.toml --workspace --target-dir target/vendor-h2
```

- **Gates:** clean, on rustc 1.98.0. A default run prints **ten**: `text`,
  `eol`, `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-four** jobs, one more than last session: `The rendered tier`
  is new and builds `--features render`, lints it, tests it and drives both
  tiers of the proving ground. Green at run **33250896807** against commit
  `2282658`, with **all twenty-four passing**, including all three release
  targets and the rendered tier driving a real Chrome on the runner. That
  answers the one question T-244 had left to CI: the vendored trees build on
  both musl targets.
  A **second** workflow arrived, `Staleness`, which is a schedule and a
  `workflow_dispatch` and does not run on a push at all.

  **The new job failed once, at run 33249228182, and it was the job's own
  fault rather than the code's.**
  `dtolnay/rust-toolchain` installs a toolchain and nothing else, so a job that
  runs clippy has to name `components: clippy`, and this one did not:
  `cargo-clippy is not installed for the toolchain`. Every other clippy job in
  the file already asked for it. Fixed in the same push as this line.

```bash
gh run list --limit 1
```

- **Soak:** nothing ran this session. The entry it worked on is not about a
  long run.
- **Entries:** 212 items. 32 open, 3 partial, 0 blocked, 166 done, 11 deferred
  to Phase C. 166 of 201 workable done, 35 left.
- **Tree:** 108 Rust files, 65,082 lines of code, 17,216 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. Nothing was mined this session and nothing was read
  from it.
- **Vendored:** **eight upstreams** now, five added this session: `rustls`,
  `h2`, `impit`, `reqwest` and `hyper-util` beside `rqbit` and its two
  siblings. **48 patches** across thirty-four sections in
  [`patches/UPSTREAM.md`](../patches/UPSTREAM.md). 9.1 MB and 824 tracked
  files, from 3.4 MB and 390.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**One entry closed, and it is the largest thing `cli-surface.md` had.** Two
pushes, five upstreams vendored, one new library module, two new acceptance
scripts, one new example, one new workflow and two new entries filed out of
what the work measured.

### [T-244](cli-surface.md), P2, `L`: the fetch is a browser, off the wire

**The JA4 reached Chrome's exactly**, which is what the entry asked for and did
not assume. Measured with the oracle already in the tree, and beside a real
Chrome 151 driven at the same probe on the same machine:

| | before | after | Chrome 151 |
| --- | --- | --- | --- |
| JA4 | `t13i1010h2_61a7ad8aa9b6_3fcd1a44f3e3` | `t13i1515h2_8daaf6152771_806a8c22fdea` | **the same** |
| Akamai | not reachable at all | `1:65536;2:0;4:6291456;6:262144\|15663105\|0\|m,a,s,p` | same but for `1:1:0:255` |
| header order | `accept` first | Chrome's | **the same** |

Ten ciphers and ten extensions became fifteen and fifteen, with ALPS, ECH,
certificate compression, SCT and the ML-DSA signature algorithms. The cipher
list is Chrome's value for value **in Chrome's order**, GREASE included.

**Reading the HTTP/2 half needed a completed handshake and no weakened
verification.** `loopback-tlsprobe` mints its own certificate authority per
run and writes it to `--ca-out`; `BIT_CLI_EXTRA_CA_FILE` adds that one root to
the platform's. Nothing anywhere stops verifying a certificate.

### Five upstreams, and three the survey named that are not among them

`apify/rustls` for the `ClientHello`, `hyperium/h2` at `v0.4.19` for the
pseudo-header order, `apify/impit` for the fingerprint database and the client,
`seanmonstar/reqwest`'s 0.13 line for one line, and `hyperium/hyper-util`
**unpatched** for one method a release has not carried.

Three of the survey's five are not vendored and each is a measurement.
`apify/h2` is `0.4.7` against the `0.4.19` this graph resolves, so a `[patch]`
at that version is declined and the fork never runs; vendoring it would record
a base that does not describe the tree. `apify/tower-http` comments out the two
lines that strip `Content-Encoding` and `Content-Length` after decompressing.
`apify/hyper-util` adds a status code to a proxy tunnel error nothing here
reads.

**Two of the three accepted costs were not paid.** `--cfg reqwest_unstable`
is needed nowhere, because HTTP/3 is removed from the vendored `impit` rather
than carried: the flag would have gone in all three `.cargo/config.toml` target
blocks and all three `RUSTFLAGS` blocks in `ci.yml`, which is
[T-146](cli-surface.md)'s shape. And two `reqwest` majors were already in the
graph before any of this, through `librqbit`.

### Staleness is detected, and the fix is recommended with proof

The operator's first-class requirement. Two checks, both writing versioned JSON
with a `schema` field per ruling 3, both on a weekly schedule rather than on a
push, because a browser shipping is not a defect in a commit.

`scripts/check-browser-version.ps1` asks Google, Mozilla and Microsoft what
stable is, every fetch trapped on its own, and prints the replacement
`BROWSER_MAJOR`, `BROWSER_USER_AGENT` and `sec-ch-ua`. It says **Chrome 153**
today against a profile claiming 151.

`scripts/check-browser-fingerprint.ps1` drives the browser this machine has at
the probe and prints the replacement `BROWSER_HEADERS` in the shape `page.rs`
wants, naming the browser and version they came from. With no browser it exits
**2** naming every path it looked at.

**The profile was not bumped to 153**, and that is a decision: the TLS half is
`impit`'s database, whose newest Chrome is 151, and claiming 153 over 151's
`ClientHello` is a mismatch an origin can see.

### `--render` ships, and it leaves nothing behind

Behind a Cargo feature, off by default, built and run by a CI job. The flag is
in **every** build and a binary without the feature refuses it by name. The
driver navigates, waits for the document to **stop changing** rather than for a
guessed duration, and composes one HTML string out of the document and any open
shadow roots, which the same `extract` reads.

```
check-page-extract: 26 case(s) over static and rendered, 26 passed, 0 failed
  links only a rendered tier reaches:
    L4-script      level 4  static 1  rendered 7  (+6)
    L5-hostile     level 5  static 0  rendered 2  (+2)
    L6-hidden      level 6  static 3  rendered 4  (+1)
    L7-unfriendly  level 7  static 0  rendered 2  (+2)
  no browser was left running: 15 before, 15 after
```

### Both extraction gaps, and the one the entry described wrongly

A link is a torrent now when its path ends `.torrent`, when it declares
`type="application/x-bittorrent"`, or when its **label** says so and its URL
carries an identifier. The label is the anchor text, then the element's
`title`, then a wrapped image's `alt`.

That last fallback is the whole finding. The entry proposed matching the anchor
text; `linuxtracker.org`'s torrent links **have no anchor text**, only an icon
whose `alt` is `Download Torrent`. Over the fifteen pages already fetched, run
through the shipping extractor and committed as
`bench/page-extract-20260829.json`: **75 links on that page where there were
0**, one of them a false positive, and nothing changed on the other fourteen.

## In progress

**Nothing.** [T-244](cli-surface.md) closed and its two residuals are filed
with their own acceptance: [T-262](cli-surface.md), the PRIORITY field of the
Akamai fingerprint, and [T-263](cli-surface.md), GREASE and the extension
order. Both are P3 and both are invisible to JA4.

**Review 1, every claim against the code it cites.** Four things.

- **One citation into a vendored tree was five lines out.**
  `streams.rs:271` names `Protocol`'s removal and the pseudo-header order is
  lifted at `:276`. Corrected by reading the line.
- **Seven citations into `cli.rs` drifted by 29 lines**, four in
  `cli-surface.md` and two in `performance.md`, as `--render`,
  `--browser-path` and `--browser-port` were added above them. The `record`
  gate caught every one.
- **A claim had no test behind it.** "The list fetcher goes through the plain
  client, and there is a test" was written before there was one. There are two
  now, in `crates/bit-cli/src/source.rs`.
- **Two of `impit`'s defaults are load bearing and nothing named them.** The
  abuse read rests on `vanilla_fallback` and `cookie_store` both defaulting
  off. They are written into `patches/UPSTREAM.md` with their lines, so a
  reconciliation that moves them is noticed.

**Review 2, a cold read.** Two numbers that were wrong.

- **`--render` costs 15 packages here and the entry said 136 in three
  places.** The 136 was a standalone probe with an empty graph;
  `chromiumoxide`'s dependencies are almost all already here. **Ruling 2 was
  not reopened** and `--render` is still a feature. The number is corrected in
  the entry so the operator can revisit rather than rediscover.
- **`http::HeaderMap` was not why the header order was wrong.** The previous
  session named two causes and only one is real: `reqwest` appends
  `user-agent` and `accept-encoding` itself. With both in the map at Chrome's
  positions the wire order is Chrome's, and `HeaderMap` iterated in insertion
  order both times it was measured. That retired a patch this session had
  already written and proved changed nothing.

**Review 3, adversarial.** Under "In progress" is the wrong place for it, so it
is here in full below.

**Review 4, the inherited claims.** Below.

**Review 5, the abuse read.** Below.

**Review 3, adversarial: what would make this the wrong implementation?**

1. **Five vendored trees is a reconciliation cost that never ends.** True, and
   it is the strongest objection. What answers it in part is that two of them
   carry **one line and no lines**: `reqwest` is a single statement and
   `hyper-util` is unpatched. The two that carry real patches, `impit` and
   `h2`, are the two whose behaviour this repository actually depends on.
2. **The impersonation is a claim about a browser that keeps moving.** Also
   true, and it is why half this session is the staleness tooling rather than
   the client. The profile is already two majors behind stable and the tool
   said so on its first run.
3. **The label rule reads a human-facing string to make a machine decision.**
   Conceded. It is bounded to a closed list of whole labels and a query
   requirement, measured at 74 true and 1 false over fifteen real pages, and a
   false positive costs one line in a list a person is already reading rather
   than a wrong download. The alternative, fetching each candidate, turns one
   page into one request per link.
4. **What the proving ground still does not generate.** A page behind a login,
   a page whose links are in a JSON API the page fetches and renders into a
   framework component, and an indexer that paginates. The first is out of
   scope by construction. The second is L4's `from-fetch` case in miniature
   and would be found. The third is not: `bit-cli` reads one page and does not
   follow a next link, by the one-hop rule.
5. **What was asserted about an origin never fetched.** Nothing new. The
   fifteen pages were fetched once each, robots honoured, and the only new
   fetch this session was the three `index` group pages, to read
   `linuxtracker.org`'s actual markup rather than guess at it. That guess would
   have been wrong.
6. **What breaks when apify moves.** `rustls` is the tree that matters: its
   emulation module is where the `ClientHello` comes from, and a release that
   restructures it is a real reconciliation. `scripts/vendor-status.ps1` and
   `scripts/upstream-scan.ps1` are what notice. `impit`'s patches are larger in
   line count and smaller in risk, because they are removals.
7. **A driven Chrome is not a stealthy Chrome.** Unchanged and still fine:
   `--render` exists for pages that build links in script, and a challenge is
   refused in both tiers.

**Review 4, the research's claims, re-verified and still asserted.**

Re-run here on the whole tree rather than on a probe:

- **MSRV 1.88 passes**, with everything vendored **and** with
  `--features render`. `cargo +1.88 check --workspace --locked`.
- **`x86_64-pc-windows-msvc` with `+crt-static`**: `scripts/check-static.ps1`
  passes, no VCRUNTIME or UCRT import.
- **Both musl targets build.** CI's `Build` matrix at run **33245679142**,
  which is the one question the entry left to CI.
- **The profile holds on a second platform.** The `Staleness` workflow's first
  run, **33251738663**, drove `ubuntu-latest`'s Chrome 151.0.7922.173 and read
  the **same JA4 and the same Akamai fingerprint** as this machine's Windows
  Chrome 151.0.7922.76. Two platforms, two patch releases, one fingerprint.
- **The Akamai fingerprint is not profile-invariant**, which the survey said it
  was. `impit`'s own database disagrees with itself: `chrome_151`'s connection
  window is 15,728,640 and `chrome_125`'s is 15,663,105, in
  `vendor/impit/impit/src/fingerprint/database/chrome.rs`. Read rather than
  measured on the wire, and said so.

**Still asserted and never tested here**, inherited from the survey and named
so the next session knows: `wreq` and `koon` failing to static-link, the
BoringSSL classification of the four candidates that were never built, every
star count, and every claim about macOS. HTTP/3 and QUIC are unverified and are
now deliberately **not shipped**, which is the honest resolution of that one.

One inherited belief is partly resolved: this tree already carried `aws-lc-rs`,
which is AWS's fork of BoringSSL, through `rustls` and
`librqbit-sha1-wrapper`, and `scripts/setup-nasm.ps1` exists because of it. The
"two TLS stacks" objection to a BoringSSL client was therefore weaker than the
entry recorded.

**Review 5, the abuse read, and the bar is higher than it was.**

- **A CAPTCHA is a refusal in the code.** One `GET`, no retry, no backoff, no
  second request, and the trait that says so is `bit_cli_core::fetch::Fetcher`.
  `impit`'s `vanilla_fallback` is the one path that could produce a second
  request and it defaults off and is never turned on; the default is cited in
  `patches/UPSTREAM.md`. L7's challenge case in the proving ground carries a
  real-looking verification form and its expected answer is a refusal in both
  tiers, so a change that starts posting one fails a check.
- **Nothing retries past a bot check**, because nothing retries at all.
- **Nothing logs a credential or a cookie**, and this session added the one
  thing that could. `--header-values` on the probe records header **values**,
  and it is used by exactly one script, against a browser that script launched
  itself into a throwaway profile at a loopback port having visited nothing.
  `cookie` and `authorization` are dropped even there, and the default shape
  carries names only, which a test asserts. No cookie jar exists on either
  client: `cookie_store` defaults to `None` and nothing sets it.
- **`--render` leaves no browser running when the command fails.** Measured on
  the timeout path, which is the one that would leak, and asserted every run:
  `check-page-extract.ps1` counts browser processes either side of the rendered
  tier and fails if the count grew.
- **A web seed still identifies itself honestly**, and so does a tracker
  announce, a peer handshake and a list fetched by URL. Two tests hold the
  seam.
- **The fetching honoured `robots.txt` and stayed to one request per page.**
  Three pages were fetched this session, through
  `scripts/check-page-fetch.ps1`, which reads `robots.txt` per host and pauses
  between requests. The extraction inventory over all fifteen was run from
  **disk**, through a loopback file server, so no second request reached
  anybody.
- **What this makes easier that it should not.** The client now looks like
  Chrome to a log at every layer an origin reads, by default. Its blast radius
  is one `GET` of a document the caller named. It does not defeat a challenge,
  does not solve one, does not retry into one, and the one new capability that
  could be misused, reading header values, is confined to a browser this
  repository started itself.
- **One thing to keep an eye on.** `impit` exposes
  `with_ignore_tls_errors`, which sets `danger_accept_invalid_certs`.
  `bit-cli` never calls it and there is no flag that would.
  `BIT_CLI_EXTRA_CA_FILE` **adds** a root and is the deliberate alternative.

## Start here next session

**The shape of the work order is the operator's and it has not changed.** Not
priority first. Clear small entries so the open count comes down, then take the
bigger ones a **category at a time**. The counts are derived from the rows:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

**This is an ordinary session's list again.** The two before it were one-offs
whose work order came in the kickoff; that is over and this file is the work
order.

1. **Re-measure the baseline rather than trusting the one above**, which is
   [RULES.md](RULES.md) section 1 step 5. Read the run this session's last push
   started: the CI line above names it. This session touched `vendor/` heavily,
   so run `scripts/vendor-status.ps1` too.

```bash
gh run list --limit 1
```

2. **[T-259](cli-surface.md), P3, `S`.** Still the smallest thing open and the
   cheapest to prove, and untouched for three sessions now: compare the non-row
   lines of `docs/schema.md` for equality while keeping the row lines as
   containment, with the hand-written tail `carry_across` preserves exempt. The
   entry names both seams with their lines.

3. **[T-250](cli-surface.md), P2.** Cheaper than it was: `source_kind` has a
   `page` value and a page's links now carry a `matched` rule each, so "how it
   was resolved" has more to say than when the entry was written. The error
   path already reports `page_links`; the successful one still says nothing
   about which link it took.

4. **[T-253](cli-surface.md), P2, `partial`.** Its blocker went two sessions
   ago and nothing has been done with that: `rcgen` is a dev dependency and
   `loopback-tlsprobe` now generates a certificate authority as well as a leaf,
   so a loopback file server that speaks TLS has two worked examples to copy
   rather than one.

5. **[T-251](trackers.md), P2, `M`, `partial`.** Unchanged: a `[[tracker]]`
   table in the file `--web-seed-config` reads, then the `[[peer]]` table after
   it. `scripts/check-announce.ps1` is where the case goes.

6. **[T-262](cli-surface.md) and [T-263](cli-surface.md), both P3**, both filed
   this session out of what the fingerprint work measured, and both small. The
   first is a PRIORITY payload in `h2`'s frame encoder; the second is GREASE
   and a shuffled extension order in the vendored `rustls`. Neither moves JA4,
   which is what makes them cheap to verify: the goldens do not change.

7. **[T-260](cli-surface.md), P2, `M`, and [T-261](trackers.md), P2, `M`.**
   T-260 publishes what a release already builds plus the data files a program
   wants by URL, and it has more to publish now than when it was filed:
   `fingerprints/*.json`, and the two staleness reports, all three carrying a
   `schema` field for exactly this. T-261 is the tracker list that is the
   second consumer of that format. Neither blocks the other.

8. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

9. **The three entries that were ruled on and are still work.**
   [T-227](memory.md) is a throughput curve then a flag.
   [T-242](performance.md) is two sweeps from `scripts/bench-leech.ps1`.
   [T-234](peers.md) and [T-238](peers.md) are the two large ones and both need
   [T-239](peers.md) first.

10. **Then the category pass, and `bep-coverage.md` is still first.**
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

**None.** The three rulings of 2026-08-29 were all acted on and T-244 closed on
them. Three things to be aware of rather than to decide.

**The `--render` ruling was given a number that is wrong by a factor of nine.**
Ruling 2 made it a Cargo feature because it cost 136 packages. Measured in this
tree it costs **15**: `chromiumoxide`'s dependencies are almost all already
here. The ruling was not reopened and `--render` is a feature. Whether it
should be is now a decision on a different number.

**The browser profile is two majors behind stable**, and the tool that says so
is new. `scripts/check-browser-version.ps1` reports Chrome 153 against a
profile claiming 151, and now also reports that the vendored fingerprint
database stops at 151 and caps its own recommendation there.

**It is not bumped, and the reason is a measurement rather than caution.**
Across the three newest entries in that database the cipher list, key exchange
groups, extension list and extension order are **identical** from 136 through
151, so a hand-written 153 would probably get the `ClientHello` right. What it
would certainly get wrong is the `sec-ch-ua` brand list: those three entries
spell the fake brand `"Not.A/Brand"`, `"Not_A Brand"` and `"Not=A?Brand"` with
the order flipped, because Chrome varies it per major on purpose. A client
announcing `Chrome/153` with 151's signature algorithms and an invented brand
string is a combination that exists nowhere, which is a **stronger** signal
than being one version behind.

**CI cannot supply the capture either.** The `ubuntu-latest` image carries
Chrome 151.0.7922.173, the same major as this machine. What unblocks a bump is
a machine running 153, or `impit` shipping the entry, which
`scripts/upstream-scan.ps1` will notice.

**Twelve repositories in `TheDancingDeveloper-org`** redistribute Apache-2.0
code under MIT with no licence file and no attribution. Nothing was said to
anybody, by [RULES.md](RULES.md) section 6a. Unchanged.

**One dependabot pull request is still open**, number 6,
`ci(deps): bump taiki-e/install-action from 2.86.3 to 2.86.5`. Not taken again,
for the same reason as the last four sessions.

## Behaviour changes worth the operator's eye

**The fetch of a source document is a browser at every layer an origin reads.**
Chrome's `ClientHello`, Chrome's HTTP/2 settings and pseudo-header order,
Chrome's header set in Chrome's order. The JA4 is Chrome 151's own, verified
against a real Chrome 151 on the same machine. `--page-client plain` restores
`bit-cli/<version>`.

**A web seed, a tracker announce, a peer handshake and a list fetched by URL
are all unaffected** and still say `bit-cli`. The list fetchers go through the
plain client whatever `--page-client` says, which is new and is tested.

**`--render` exists.** It needs a build with the `render` feature, which the
released binaries do not carry; without one the flag is refused by name.
`--browser-path` and `--browser-port` came with it. Nine commands gained all
three.

**`BIT_CLI_EXTRA_CA_FILE` is read.** A PEM bundle named by it is trusted **in
addition to** the usual roots when a source document is fetched, and a run that
reads it logs a warning naming the file. There is no flag anywhere that stops
verifying certificates.

**A page's links are matched three ways now**, not one: the `.torrent`
extension, a declared `type="application/x-bittorrent"`, or a torrent label
over a URL carrying an identifier. **A script that counted links on an indexer
page will see more of them**, which is the point: one real index went from 0 to
75. Each link in a refusal carries a new `matched` field saying which rule took
it.

**`bit-cli` does not speak HTTP/3 and did not before**, and now it will not by
construction: the vendored client's HTTP/3 path is removed rather than carried
behind an unstable compiler flag.
