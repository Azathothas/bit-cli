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
- **Entries:** 213 items. 33 open, 3 partial, 0 blocked, 166 done, 11 deferred
  to Phase C. 166 of 202 workable done, 36 left.
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
`BROWSER_MAJOR`, `BROWSER_USER_AGENT` and `sec-ch-ua`. It says **Chrome 152**
today against a profile claiming 151, and it said 153 until the endpoint it
reads was corrected; the review section below carries that.

`scripts/check-browser-fingerprint.ps1` drives the browser this machine has at
the probe and prints the replacement `BROWSER_HEADERS` in the shape `page.rs`
wants, naming the browser and version they came from. With no browser it exits
**2** naming every path it looked at.

**The profile was not bumped**, and that is a decision: the TLS half is
`impit`'s database, whose newest Chrome is 151, and claiming a version over
another version's `ClientHello` is a mismatch an origin can see.

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

**A sixth review, after the entry closed, and it found the worst defect of the
session in this session's own tooling.**

`scripts/check-browser-version.ps1` read
`.../channels/stable/versions?pageSize=1`, which answers with the highest
version **known** on the channel rather than the version being served. Chrome
rolls out in stages, so for days that is a build almost nobody has. Measured:

| | |
| --- | --- |
| what the check reported as stable | `153.0.8010.12` |
| that build's rollout fraction | **0.005** |
| what was at fraction 1 | `152.0.7977.65` |
| Chrome for Testing's `Stable` | `152.0.7977.64` |

So the check inflated the drift by a whole major, which is the **opposite** of
the defect it was written to catch: chasing a build one user in two hundred has
produces a correct fingerprint of a browser that does not exist. It reads the
releases endpoint and the rollout fraction now, cross-checks against Chrome for
Testing, and prints the highest published version and its fraction beside the
answer so a reader can check the choice rather than take it.

It was found by asking a question the entry had not asked: whether any other
source offers a newer Chrome. Chrome for Testing answered with a different
number, and two first-party endpoints disagreeing is a finding rather than an
error.

**Review 3, adversarial: what would make this the wrong implementation?**

1. **Five vendored trees is a reconciliation cost that never ends.** True, and
   it is the strongest objection. What answers it in part is that two of them
   carry **one line and no lines**: `reqwest` is a single statement and
   `hyper-util` is unpatched. The two that carry real patches, `impit` and
   `h2`, are the two whose behaviour this repository actually depends on.
2. **The impersonation is a claim about a browser that keeps moving.** Also
   true, and it is why half this session is the staleness tooling rather than
   the client. The profile is already a major behind stable and the tool said
   so on its first run, once the endpoint it read was corrected.
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

## A ruling arrived after the entry closed, and it is section 6b now

**A fingerprint is measured, never derived and never inherited.** Given by the
operator on 2026-08-29 once T-244 closed with the profile a major behind
stable. [RULES.md](RULES.md) section 6b carries it in full and
[T-264](cli-surface.md) is the entry that makes it routine.

Three parts, and each one closes a shortcut before somebody reaches for it.
`impit`'s fingerprint database is a starting point and not an authority, and it
has already been wrong here. A value Chromium computes from a version number is
still not one this repository may compute, because it cannot vendor Chromium
and would be the only consumer of a port that drifts. And the profile therefore
moves only behind a capture from a browser of that version.

**The second instrument is a container, and it was used before it was written
down.** Measured on this machine on 2026-08-29:

| | |
| --- | --- |
| `debian:bookworm-slim` plus Google's own apt repository | **Chrome 152.0.7977.64** |
| this host | Chrome 151.0.7922.76 |
| `ubuntu-latest`, run 33251738663 | Chrome 151.0.7922.173 |
| `versionhistory`, Windows stable, actually serving | 152.0.7977.65 |
| Chrome for Testing, Stable | 152.0.7977.64 |
| `versionhistory`, highest published, at fraction 0.005 | 153.0.8010.12 |

**So the container's Chrome 152 is current stable**, and the capture that
closes the gap is available today rather than blocked on anything. The apparent
disagreement between the apt channel and the Windows channel was the check
reading the wrong endpoint, not the channels differing; the correction is
below.

**One piece needs code and the measurement says which.** WSL is in NAT mode on
this host, so a distro cannot reach the Windows loopback and
`loopback-tlsprobe` binds `127.0.0.1` only. The distro **does** reach the host
at the WSL adapter address, `172.23.96.1`, and a listener bound there accepted
the connection. So the probe takes a `--bind` that defaults to loopback, and
the capture binds it to that address, which is a Hyper-V internal network and
is not the LAN. That is T-264's second piece.

[`docs/containers.md`](../docs/containers.md) is the procedure, including the
traps: pin a commit rather than a branch, pass a command as base64 because
PowerShell parses text before the distro sees it, `/bin/sh` is dash and has no
`/dev/tcp`, and a rootfs costs several times its own size as a virtual disk.

**Everything this session created in a container was removed in the same
session.** One distro, `eph-bitcli-chrome`, and
`wsl-ephemeral.ps1 -Action List` reports `(none)` afterwards with no orphaned
rootfs tarball.

## The engine was not left as it was found, and not by this session

Asked to look, reported, and then cleaned on the operator's authorisation.

```bash
podman system df
```

| | total | active | size | reclaimable |
| --- | --- | --- | --- | --- |
| images | 554 | 1 | 31.35 GB | **31.35 GB, 100 percent** |
| containers | 1 | 0 | 5.08 GB | 5.08 GB |
| local volumes | 5 | 0 | 18.78 MB | 18.78 MB |

Nothing is in use. There are **21 dangling images**, one exited container
called `pmarch` from about nineteen hours before this session, and five
orphaned volumes: `xwork`, `fwd`, `dlopen-exp2` and two hash-named ones, none
attached to any container. The named images that survive, `localhost/archlinux`
in five architectures and `ghcr.io/pkgforge-dev/archlinux:loong64`, look
deliberate and were left alone.

`podman system prune -a --volumes` reclaimed **306.2 GB**, which is larger than
the `df` figure above because `df` counts deduplicated image size and the prune
counts every layer and blob it removed. The engine reports zero images, zero
containers and zero volumes afterwards.

It was not run until the operator authorised it: some of those images take a
long time to rebuild and no session should decide that for somebody else.

## Start here next session

**The operator allocated one more session to T-244 and it is the last one.**
The ordinary work order is below it and resumes the session after. This is not
a one-off in the [RULES.md](RULES.md) section 3 sense: the work order is here,
in this file, where it belongs, and the kickoff prompt stays generic.

**What "finish it for real" means**, and it is four entries, not a feeling:
[T-262](cli-surface.md), [T-263](cli-surface.md) and [T-264](cli-surface.md)
all close, and T-264's move takes the profile out of the vendored tree. Every
one has an acceptance command already written and every one has its measurement
already taken. Nothing in the four needs a decision: both that were open were
ruled on and are in [RULES.md](RULES.md) section 6b.

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Re-measure the baseline rather than trusting the one above**, which is
   [RULES.md](RULES.md) section 1 step 5. This session touched `vendor/`
   heavily, so `scripts/vendor-status.ps1` goes first.

```bash
gh run list --limit 1
```

2. **[T-264](cli-surface.md), P2, `M`, and it is the spine.** Four pieces and
   three are already measured; the entry carries the numbers so none of it
   needs re-deriving.

   Do them in this order, because each makes the next testable: `--bind` on
   `loopback-tlsprobe`, then the container capture in
   `scripts/check-browser-fingerprint.ps1 -Container`, then **the move of the
   whole profile into `crates/bit-cli-core/src/page.rs`**, then the bump from
   what Chrome for Testing's Stable emitted.

   The move is the part with the leverage and the part to do carefully: after
   it, a bump edits one file this repository owns and `vendor/impit` carries no
   data this repository authored.

3. **[T-263](cli-surface.md), P3, `M`.** GREASE at both ends of the extension
   list and a shuffled order, in the vendored `rustls` emulation module. Cheap
   to verify because it moves no golden: JA4 and JA4_r both sort, so
   `scripts/check-fingerprint.ps1` passes unchanged and `JA4_ro` is what shows
   the fix landed.

4. **[T-262](cli-surface.md), P3, `S`.** The PRIORITY payload in `h2`'s frame
   encoder, which is the one field of four where the Akamai fingerprint still
   differs from a real Chrome. Smallest of the three and the riskiest code:
   `vendor/h2/src/frame/headers.rs:301` is a protocol library's frame writer.
   `cargo test --manifest-path vendor/h2/Cargo.toml --workspace` is what holds
   it.

5. **Then the ordinary list resumes**, in the shape the operator has kept
   throughout: not priority first, clear the small entries so the open count
   comes down, then take the bigger ones a category at a time.

   [T-259](cli-surface.md) P3 `S`, still the smallest thing open and untouched
   for three sessions. [T-250](cli-surface.md) P2, which has more to report
   than when it was filed now that a page's links carry a `matched` rule each.
   [T-253](cli-surface.md) P2 `partial`, whose blocker went two sessions ago
   and which now has two worked examples of `rcgen` to copy.
   [T-251](trackers.md) P2 `partial`. Then [T-260](cli-surface.md) and
   [T-261](trackers.md), the publishing pair, which have three schema-carrying
   files to publish now rather than one.

6. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

7. **The three entries that were ruled on and are still work.**
   [T-227](memory.md) is a throughput curve then a flag.
   [T-242](performance.md) is two sweeps from `scripts/bench-leech.ps1`.
   [T-234](peers.md) and [T-238](peers.md) are the two large ones and both need
   [T-239](peers.md) first.

8. **Then the category pass, and `bep-coverage.md` is still first.**
   [T-101](bep-coverage.md) is open on a latency measurement loopback cannot
   produce, which [T-239](peers.md) is the prerequisite for.
   [T-102](bep-coverage.md) and [T-168](bep-coverage.md) are the untouched two,
   then `dht.md`.

**Corpus sources the list above wants**, all on this machine and none needing a
fetch: `reference/RESEARCH.md` section D has one row per open entry; entries 23
to 29 for [T-234](peers.md); entries 30 to 37 for [T-238](peers.md) and
[T-239](peers.md); and `reference/README.md`'s "The 2026-08-24 trees" section.
**All of it is a read.** Nothing was read from it this session.

**A container is available and [`docs/containers.md`](../docs/containers.md) is
the procedure.** It answers in seconds what CI answers in five minutes, and
everything it creates is removed in the same run. `podman system df` before
finishing is the number that says whether something stopped cleaning up.

## Open questions for the operator

**None. Two were raised at the end of this session and both were ruled on**,
and they are in [RULES.md](RULES.md) section 6b so the next session acts on
them rather than re-deriving them.

1. **The profile lives in `bit-cli-core`, not in the vendored tree.**
   `page.rs` holds all of it and builds `impit`'s `BrowserFingerprint` from its
   own values. One file is the truth, a staleness recommendation has one file
   to target, and `vendor/impit` carries no data this repository authored.
2. **The shipped profile claims Stable.** Beta is captured and recorded beside
   it so the next bump is ready the day it ships, and it is not what the client
   claims to be.

**Three things to be aware of rather than to decide.**

**The container engine was cleaned, on the operator's authorisation.**
`podman system df` reported 554 images at 31.35 GB with nothing in use, one
exited container and five orphaned volumes, all from earlier sessions that did
not clean up after themselves. `podman system prune -a --volumes` reported
**306.2 GB** reclaimed, which is larger than the `df` figure because `df`
counts deduplicated image size and the prune counts every layer and blob it
removed. The engine reports zero of everything now.

**Two asks were written down for whoever owns `wsl-ephemeral.ps1`**, in
[T-264](cli-surface.md) under Notes. The useful one is `-Action HostAddress`:
today a caller has to create a distro, read `/proc/net/route` and decode
little-endian hex to find out how to talk to the host, and the tool already
knows the networking mode. Neither ask blocks anything.

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
