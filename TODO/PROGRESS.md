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
ran none, because nothing it worked on is about a long run.

## Section 6b was applied for the first time, including the part that says no

**A fingerprint is measured, never derived and never inherited.**
[RULES.md](RULES.md) section 6b carries it in full. This session did the two
things it asks for and then declined the third on the ruling's own argument:
[T-264](cli-surface.md) moved the whole profile out of the vendored tree into
`crates/bit-cli-core/src/page.rs`, built the container capture the ruling names
as the second instrument, and then **did not bump the profile**, because the
capture proved a bump today would ship a `ClientHello` that exists nowhere.

## State

- **Last session:** 2026-08-30T01:24:31Z, unattended, and the ordinary work
  order resumed: this file's "Start here next session" rather than the
  kickoff's. The kickoff added one thing on top of it, the new WSL tooling to
  read and the container page to correct, and that is folded into
  [T-264](cli-surface.md) and `docs/containers.md` rather than carried apart
  from them.
  The duration is not restated here: `scripts/session-report.ps1` derives it
  from the instant above, and a duration written down twice is a number two
  documents disagree about.

  **The plan was written before starting**, per [RULES.md](RULES.md) section 1
  step 4: T-264 in the work order's own four-piece order, because each piece
  makes the next testable.
- **Tests:** 1,472 passing, 0 failing, up from 1,462. Plus **439** in the
  vendored `h2` library, **153** in `rqbit` and **76** in `librqbit-utp`, none
  of which the workspace gates run.

  **One of the 439 fails about one run in one under `--workspace` and is
  upstream's**, unchanged and still not this repository's code:
  `proto::streams::recv::tests::clear_recv_buffer_caps_capacity_before_overflow`.

```bash
cargo test --manifest-path vendor/h2/Cargo.toml --workspace --target-dir target/vendor-h2
```

- **Gates:** clean, on rustc 1.98.0. A default run prints **ten**: `text`,
  `eol`, `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-four** jobs. The baseline this session started from is
  run **33253156829**, against commit `f4a10f0`, green with all twenty-four
  passing. A second workflow, `Staleness`, is a schedule and a
  `workflow_dispatch` and does not run on a push.

```bash
gh run list --limit 1
```

- **Vendored:** **eight upstreams** and **49 patches** across thirty-six
  sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md), one more than
  last session. `scripts/vendor-status.ps1` says the series matches the trees,
  every patch has a section, and the version, changelog and pins agree.

  **The profile move edited no vendored file**, which was the point of it, and
  [T-262](cli-surface.md) then edited `h2` and `impit` for a reason that is not
  data: a frame this client could not write.
- **Soak:** nothing ran this session.
- **Entries:** 213 items. 31 open, 4 partial, 0 blocked, 167 done, 11 deferred
  to Phase C. 167 of 202 workable done, 35 left.
- **Tree:** 108 Rust files, 65,392 lines of code, 17,330 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. Nothing was mined this session and nothing was read
  from it.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**[T-262](cli-surface.md) closed and [T-264](cli-surface.md) went from open to
`partial` with three of its four pieces done**, its fourth blocked on something
measured rather than predicted. Two new scripts, one committed pin, two new
probe switches, one whole-file profile move, two new vendored patches, and
`docs/containers.md` rewritten against tooling that gained two actions since the
page was written.

### [T-262](cli-surface.md): the Akamai fingerprint is a browser's in all four fields

The one field of four where this client was still distinguishable. Chrome opens
stream 1 with a PRIORITY block and `h2` wrote none, so the fingerprint carried
`0` where a browser carries `1:1:0:255`.

**The encoder needed nothing new**, which is the finding: the HEADERS encoder
already takes a closure that runs after the head and before the header block,
which is how `PushPromise` writes its promised stream id, and the payload length
is measured after it runs. So the five bytes are counted in the frame length and
in any CONTINUATION split without either being computed by hand, and the part
the entry expected to be delicate was already solved.

Off the wire, not derived:

| | PRIORITY field |
| --- | --- |
| before | `0` |
| after | `1:1:0:255` |
| a real Chrome 151, and a real Chrome 152 | `1:1:0:255` |

The golden moved in that one field and nothing else. The exemption in
`scripts/check-browser-fingerprint.ps1` came off with the entry, which is the
other half of the rule about a check that measures an open defect.

### The profile is this repository's now, and the move is behaviour neutral

`crates/bit-cli-core/src/page.rs` carries the whole of it: the cipher list, the
key exchange groups, the signature algorithms, the extension order, ALPN, the
HTTP/2 settings, the pseudo-header order and the headers.
`page::browser_fingerprint()` constructs `impit`'s type from those values, where
`fetch.rs` used to take `chrome_151::fingerprint()` and overwrite the header
half.

**That it changed nothing on the wire is measured rather than assumed.**
`scripts/check-fingerprint.ps1` passes with the goldens untouched: the same JA4
`t13i1515h2_8daaf6152771_806a8c22fdea` and the same header order.

The one test still making the vendored database the authority asserted that our
TLS half **equals** `impit`'s `chrome_151` entry. It is replaced by one
asserting the client presents what `page.rs` declares, which is the assertion
that survives a bump.

### The capture from a browser this machine does not have

`scripts/check-browser-fingerprint.ps1 -Container` installs a browser in a
throwaway `debian:bookworm-slim` distro, drives it at `loopback-tlsprobe` on the
address a distro reaches this host at, and removes the distro in the same run,
reading the state back rather than trusting it. `-Source cft -Channel
Stable|Beta|Dev|Canary` takes a Chrome for Testing build; `-Source apt` takes
Google's branded stable package. Evidence:
`bench/browser-fingerprint-cft-152.json`.

Two probe switches were needed and both are worth having on their own.
**`--bind`** takes the address, defaults to `127.0.0.1`, puts whatever it names
in the leaf certificate so a verifying client still verifies the name it
dialled, and refuses a hostname and the unspecified address by name.
**`--until-h2`** stops at the first connection that reached HTTP/2.

### Four things the measurement found, and three were not predicted

**Chrome 152's Akamai fingerprint is Chrome 151's exactly**, including the
`1:1:0:255` PRIORITY field. [T-262](cli-surface.md) is now reproduced on two
versions and two platforms.

**Two extensions are new in 152 and this stack can emit neither.** From the raw
hello in wire order, `0x12e0` at position 7 and `0xca34` at position 10.
`0xca34` is the trust anchors draft; `0x12e0` this session could not identify
and says so rather than guessing. `impit`'s `ExtensionType` names neither, and
`vendor/rustls`'s `ClientExtensions` has one typed field per extension type, so
it has nowhere to put either. **That is why the profile was not bumped.** A
client claiming 152 without them sends a `ClientHello` that exists nowhere,
which section 6b says is a stronger tell than being one version behind.

**Chrome for Testing cannot supply the header half of a branded profile.** Its
`sec-ch-ua` carries no Google Chrome entry at all, which is why `-Source apt`
exists. The one header change that is platform independent and already measured
is the **order**: 152 moves `accept-language` from twelfth to fourth.

**A browser opens sockets it abandons.** Driving Chrome 152 at the probe made 13
connections: the **first** carried no HTTP/2 at all, and every one after the
second carried `pre_shared_key` because the session resumed. So neither the
first capture nor the last is the one to read. Both paths now take the first
that reached HTTP/2, which fixes a defect the host path already had and which
was invisible only because a Chrome on loopback happened to win the race.

### The tooling is pinned by a file rather than by a habit

`scripts/wsl-tool.ps1` resolves `Azathothas/ToolKit`'s WSL2 tooling at the
commit `scripts/toolkit-pin.json` names, verifies both digests, and forwards
everything else unchanged. It exists because of a defect this session hit: the
launcher resolves a sibling `wsl-ephemeral.ps1` **ahead of** a pinned ref, so
with the previous session's copy still in `.tmp/`, a run passing both a ref and
a digest ran the stale file and verified nothing. The only sign was one line
reading `Using the copy beside this launcher`. `wsl-tool.ps1` keeps its cache
holding the launcher alone and removes any sibling first.

### `docs/containers.md` is rewritten, and both asks were answered

The operator asked for the page to be corrected against the new tooling. Two
things it described as workarounds are now actions. **`-Action HostAddress`**
prints the address a distro reaches this host at, on stdout alone, without
creating a distro, which retires the `/proc/net/route` little-endian decoding
the page carried; measured here as `172.23.96.1`, agreeing with what a real
distro said. **`-Action Resources`** reports what WSL and the engine hold and
prints the cleanup commands without running one. `-PortForward` was asked for
and refused, with the documentation the ask itself offered as its alternative.

The page also gained what this session measured the hard way: the sibling
shadowing above, that a hard interrupt leaves a registered distro and an
orphaned tarball, that Chrome on Linux does not read its NSS database so a
trusted CA still gives `CertificateUnknown`, that `New -Command` now returns the
inner command's code where it used to report 0, and that `wsl --shutdown` is
machine wide and takes the podman machine down with it.

## In progress

**[T-264](cli-surface.md) is `partial`** and the entry says what is left, in
order: name `0x12e0`; add both codepoints to `impit`'s `ExtensionType` and to
`vendor/rustls`'s extension encoder; capture a branded Chrome 152 with `-Source
apt`; then bump. Only the last of those is waiting on the open question below.

**Nothing else is half done.** [T-262](cli-surface.md) and
[T-263](cli-surface.md) are untouched code and both gained a second
measurement's worth of detail from the 152 capture.

## Start here next session

1. **Re-measure the baseline rather than trusting the one above**, which is
   [RULES.md](RULES.md) section 1 step 5.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
gh run list --limit 1
```

2. **[T-263](cli-surface.md), P3, `M`**, and it is now the entry with the most
   already worked out. It carries the Chrome 152 wire order, the two GREASE
   codepoints that connection chose, and the finding that **the shuffle is half
   built in the vendored tree already**: rustls sorts by a per-connection
   `order_seed` and only the extensions **not** named in `extension_order` take
   part, so naming fewer of them is most of the work. The GREASE half needs a
   new shape, because `ClientExtensions` has one slot at a fixed codepoint and
   Chrome sends two at chosen ones.

3. **[T-264](cli-surface.md)'s remainder**, which is the list under "In
   progress" and finishes with the bump. The first two items are the same
   vendored extension encoder T-263 touches, so doing T-263 first is what makes
   them cheap.

4. **Then the ordinary list resumes**, in the shape the operator has kept
   throughout: clear the small entries so the open count comes down, then take
   the bigger ones a category at a time.

   [T-259](cli-surface.md) P3 `S`, still the smallest thing open and untouched
   for four sessions. [T-250](cli-surface.md) P2. [T-253](cli-surface.md) P2
   `partial`, which now has three worked examples of `rcgen` to copy.
   [T-251](trackers.md) P2 `partial`. Then [T-260](cli-surface.md) and
   [T-261](trackers.md), the publishing pair.

5. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

6. **The three entries that were ruled on and are still work.**
   [T-227](memory.md) is a throughput curve then a flag.
   [T-242](performance.md) is two sweeps from `scripts/bench-leech.ps1`.
   [T-234](peers.md) and [T-238](peers.md) are the two large ones and both need
   [T-239](peers.md) first.

7. **Then the category pass, and `bep-coverage.md` is still first.**
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
the procedure**, rewritten this session and now describing tooling that answers
the host address without building a distro to ask.
`pwsh -NoProfile -File scripts/wsl-tool.ps1 -Action List` is the first thing to
run and the last, and `podman system df` before finishing is the number that
says whether something stopped cleaning up.

## Open questions for the operator

**One, and it is about honesty rather than about work.**

1. **How far may one profile be assembled from more than one capture?** The bump
   to 152 needs three things that no single browser reachable from this machine
   emits together: the TLS half, which the Linux container supplies and which
   run 33251738663 already measured to be identical across platforms for 151;
   the branded `sec-ch-ua`, which only a branded build supplies; and the Windows
   platform strings, which only a Windows build supplies.
   [RULES.md](RULES.md) section 6b says everything the profile claims is
   measured off a browser. It does not say whether that means **one** browser.
   This session read it as one and therefore did not bump. If captures may be
   combined where each field is measured and the seams are written down, the
   bump is a session's work rather than a blocked item.

**Two things to be aware of rather than to decide.**

**The container engine was left as it was found.** The only image pulled this
session was `debian:bookworm-slim`, and `wsl-ephemeral.ps1 -Action List` reports
`(none)` with no orphaned rootfs tarball. One run was killed mid-install and did
leave a distro and a 74.3 MiB tarball; `Purge` removed both in the same session,
which is what that command is for.

**One dependabot pull request is still open**, number 6,
`ci(deps): bump taiki-e/install-action from 2.86.3 to 2.86.5`. Not taken again,
for the same reason as the last five sessions.

## Behaviour changes worth the operator's eye

**One thing `bit-cli` puts on the wire changed, and it is five bytes.** The
HEADERS frame opening a source-document fetch now carries a PRIORITY block,
exclusive on stream 0 with weight 255, which is what a browser sends. Nothing
else moved: the `ClientHello` is byte identical and so is the header set.

**The profile moving file changed nothing at all**, which is the point of that
half: one file this repository owns is now the only place a fingerprint comes
from, and `vendor/impit`'s database is read by nothing that ships. The goldens
did not move for it, only for the five bytes above.

**`loopback-tlsprobe` takes `--bind` and `--until-h2`.** Both are test fixture
switches and neither reaches a shipping binary. `--bind` defaults to loopback
and refuses the unspecified address by name.

**`scripts/check-browser-fingerprint.ps1` takes `-Container`, `-Source` and
`-Channel`**, and its host path now reads the first capture that reached HTTP/2
rather than the second line of output, which is a correctness fix on a path that
already existed.

**`scripts/wsl-tool.ps1` is new** and is the only thing in this repository that
should fetch the WSL tooling.
