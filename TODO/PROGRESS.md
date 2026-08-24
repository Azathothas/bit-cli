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

- **Last session:** 2026-08-24T11:48:11Z, unattended, and it worked the entries
  rather than filing them. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.
- **Tests:** 1,312 passing, 0 failing, up from 1,298. Plus **149** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run. `vendor/` is untouched.
- **Gates:** clean, on rustc 1.98.0. A default run prints **nine**: `text`,
  `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-two** jobs. Green at run **32731290459**, against commit
  `5ec11e8`. Three runs this session and none was red.

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
- **Tree:** 98 Rust files, 58,502 lines of code, 15,187 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. [`reference-map.md`](reference-map.md) carries the
  licence per tree and where the determination came from. Nothing was mined
  this session; five peer id tables already in the corpus were read, and one
  file was fetched from libtorrent, which [T-236](peers.md) records.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  Untouched.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Two entries closed, both P1, both taken from the work order in its own
order.** No new entries were filed. The session before this one filed eleven and
worked none; this one is the other half of that.

**Both entries undercounted their own defect, and both undercounts were found
the same way: by measuring the whole surface before changing any of it rather
than trusting the table in the entry.** That is the one transferable thing here.

### [T-245](cli-surface.md), P1: four commands turned out to be nine

The entry said `info`, `files`, `magnet` and `verify` refuse the HTTP URL that
`download` accepts. Running every command against one URL first said **nine**,
and a tenth refuses it with a different message that is also wrong.

| command | before |
| --- | --- |
| `info`, `files`, `magnet`, `verify` | exit 4, "has to be fetched before it can be read" |
| `webseed list`, `test`, `probe`, `fetch` | exit 4, the same message |
| `bench webseed` | exit 4, the same message |
| `trackers` | exit 4, "an info hash is needed to announce, and this source does not carry one" |
| `download`, `seed`, `peers`, `bench leech` | works |

`trackers` is left as it was and [T-251](trackers.md) owns it: the URL **does**
carry an info hash once fetched, and the entry that owns what a tracker command
knows about its source is the right place for it.

**A metalink is the same defect in the same help string.** All nine offer "a
metalink" in their `SOURCE` text and all nine refused both metalink shapes.
Fixed together, because it is one code path and one sentence of help.

`crates/bit-cli/src/source.rs` gained `resolve`, which fetches, and
`resolve_blocking`, which is what a synchronous command calls. `load_local`
stays as the local-only path and keeps the magnet and info hash refusal,
because those need the swarm rather than one `GET`. The runtime is built only
when the source needs one, and a caller already inside a runtime gets an error
naming the mistake rather than tokio's panic.

Three bounds, each measured rather than reasoned about:

- The deadline is `--timeout` when set and 30s otherwise. Against a
  `--stall-after 64` file server, `--timeout 2s` gave up at 2,081ms and
  `--timeout 5s` at 5,090ms.
- **A fetch that runs out of time exits 9 and names the deadline.** It exited 5
  saying "error decoding response body" until that was fixed, which is
  `reqwest` describing the transport rather than the flag the caller set.
- A `.torrent` body is capped at 16 MiB and a metalink at 1 MiB, counted as the
  bytes arrive. `fetch_metalink` read the whole body and measured it
  afterwards, so its cap bounded what was returned rather than what was held.

### [T-236](peers.md), P1: two peer ids turned out to be six

The entry named the session's `-rQ9010-` and `bit-cli trackers`'s `-BC0100-`.
Grepping for the prefix found **six**, and five of the six claimed BitComet's
code: those two, `bench probe`, the web seed bridge, the swarm bench's
synthetic peer, and the listener health check.

There was a seventh at `listener.rs:194` and it is deliberately **not** ours: a
fixture standing in for a remote peer. It says so in a comment now as well as
in its bytes, because a fixture replying with this client's own prefix would
hide a self-connect rather than exercise one.

**An eighth was a decoder rather than a generator.** `bench probe`'s client
table read `-BC` as `bit-cli`, so probing a real BitComet peer reported this
client's name. It reads `CL` as `bit-cli` and `BC` as BitComet now.

**The entry said `-bC` is taken. It is not.** Checked against libtorrent
`v2.0.11` `src/identify_client.cpp:148-250`, which carries **92** Azureus-style
codes, against `aquatic/crates/peer_id/src/lib.rs:100-120`, and against the
four other tables in the corpus, in `seedchamp`, `torrust-actix`, `gosh-dl` and
`superseedr`. `bC`, `bt`, `bl`, `bi`, `CL` and `cl` are free in all six.

**The code is `CL`**, and the property that separated the candidates was case
twins. The lookup is a byte comparison, so `lt` and `LT` are different clients
and have been for two decades, which makes a twin legal but confusable. Every
candidate containing `b` has one: `bC` twins `BC`, the client this was being
mistaken for; `bt` twins `BT`; `bl` twins `BL`; `bi` twins `BI`. `CL` has no
twin in any of the six in either case, and it reads as the command line.

`crates/bit-cli-core/src/peer_id.rs` is the one place and all six read it. The
version is built from `CARGO_PKG_VERSION_*` at compile time, so `-CL0200-`
follows this crate rather than the vendored tree. Two **compile-time**
assertions rather than runtime checks: a component past 61 has no
single-character encoding, and a prerelease cannot ship while the build slot is
still `0`, because Transmission puts `B` or `Z` there.

The suffix is printable now. It was twelve raw bytes on the session path, which
is why the check's own output used to be half percent escapes.

```
peer id:     -CL0200-nnznnl2zn5d2
```

### What the two reviews found

**Review 1, every claim against the code it cites.** Three errors, all in work
written this session. The libtorrent table carries **92** codes and three
documents said 93; the cited range is `:148-250` and two said `:150-270`; and
two line numbers in [T-236](peers.md) drifted by this session's own edits,
`listener.rs:50` to `:51` and `:186` to `:194`. All corrected.

**Two commit bodies carry a wrong statement and neither is corrected**, because
rewriting a pushed commit message is worse than the statement in it. `4acc095`
says libtorrent's table carries 93 codes; it carries 92. `5ec11e8` ends "so
-NoCi", and the flag was not passed: `git-sync` would have refused it anyway,
because the push carries `crates/bit-cli-core/src/peer_id.rs` and a
documentation-only push carrying a source file is exactly the one that needed
CI. Run **32731290459** is that push's, and it is green. The entries are the
record and the entries are right.

**Review 2, a cold read for a document contradicting another.** Two claims in
[`docs/examples/inputs.md`](../docs/examples/inputs.md) were stated rather than
run. "The four read-only commands were run against all five forms and compared
field for field" was true of the URL row only, and now says so. The body cap
was described from reading the constant; it is measured now, against a 2 MiB
document, and the page carries the message it produced. The same page's table
named neither `peers` nor `trackers`, which left the one command that still
refuses a URL unmentioned; it is named.

**And one thing the gates caught rather than a review**, for the third time in
three sessions. Two tests were first written through a heredoc and the `\r\n`
in their HTTP response headers arrived as real CR and LF bytes. The fixture
then served a malformed response, and the test hung for its full 30 second
deadline before failing on the wrong exit code. [RULES.md](RULES.md) section 5
already says to write text to a file rather than send it through two shells,
and commit `3c29786`, from before this session, says what it costs.

## In progress

Nothing is half-written. Both entries closed complete, with the acceptance run
and its output recorded in the entry.

- **[T-253](cli-surface.md)** is still `partial`, untouched: the fifteen schema
  rows are in and the fixture that would generate them is not.
- **[T-164](peers.md)** is still `partial`, untouched.
- The entries the last sessions left open are untouched: [T-232](memory.md),
  [T-224](memory.md), [T-233](peers.md), [T-237](trackers.md),
  [T-239](peers.md), [T-240](dht.md), [T-241](metainfo.md),
  [T-101](bep-coverage.md), [T-102](bep-coverage.md), [T-168](bep-coverage.md),
  and the nine filed last session that are not [T-245](cli-surface.md).
- **[T-243](phase-c.md)** is deferred and the operator has deliberately not
  ruled on it. Do not raise it.

## Start here next session

**The shape of the work order is the operator's, from nine sessions ago.** Not
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

3. **The three small correctness entries, all effort S, and they share one
   fixture.** [T-246](cli-surface.md) and [T-247](cli-surface.md) are both a
   rendering that says something untrue, and [T-249](metainfo.md) is a second
   rendering of a layout that is already computed. A torrent with a hostile
   path, an unwritable directory and a URL is the whole fixture set.

4. **[T-251](trackers.md), P2**, and it is more interesting than it was.
   `trackers` is the one command left that refuses a URL its own help offers,
   and its refusal names an info hash the URL carries. The source half of it is
   a few lines now that `source::resolve_source` exists; the twelve-knobs half
   is the rest of the entry.

5. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

6. **[T-244](cli-surface.md) is unblocked**, which it was not before this
   session. Its prerequisite was [T-245](cli-surface.md), and a plain
   `.torrent` URL resolves under `info` now. The ruling on it is static
   extraction with a browser-shaped header set and a `--render` opt-in.

7. **The five entries that were ruled on and are now work rather than
   questions.** [T-033](performance.md) is three aria2 aliases plus a warning
   naming the per-server against per-source difference, with the man page
   written first per [T-198](cli-surface.md). [T-227](memory.md) is a
   throughput curve then a flag. [T-242](performance.md) is two sweeps from
   `scripts/bench-leech.ps1`. [T-234](peers.md) and [T-238](peers.md) are the
   two large ones and both need [T-239](peers.md) first.

   [T-234](peers.md) is cheaper than it was: `--as-client` is a second value
   for `bit_cli_core::peer_id::CLIENT_CODE` and its version characters, and
   that module is now the only place either is read from.

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

The five peer id tables this session read are worth knowing about before
[T-234](peers.md): `aquatic/crates/peer_id/src/lib.rs:100-120`,
`seedchamp/crates/engine/src/wire/peer_id.rs`,
`torrust-actix/src/tracker/impls/peer_id.rs`, `gosh-dl/src/torrent/peer.rs`
and `superseedr/src/peer_manager.rs`. The widest is `seedchamp`'s at 91 codes,
and `torrust-actix`'s is the classic BEP 20 list.

## Open questions for the operator

**None.** Nothing this session did needed a ruling, and nothing it found needs
one.

The eight rulings the session before this one received are written into the
entries as `Ruled:` blocks rather than restated here. Two were acted on this
session without further input: [T-245](cli-surface.md) needed none, and
[T-236](peers.md) is the honest-default half of [T-234](peers.md)'s ruling,
which is the half that does not need `--as-client` built.

**One thing to be aware of rather than to decide.** Twelve repositories in
`TheDancingDeveloper-org` redistribute Apache-2.0 code under MIT with no licence
file and no attribution. Nothing was said to anybody about it, by
[RULES.md](RULES.md) section 6a, which is absolute. It is recorded in
`RESEARCH.md` entry 40 so that a later session reading `license = "MIT"` in one
of those manifests does not act on it.

**A second thing, unchanged.** One dependabot pull request is open on this
repository, `dependabot/github_actions/github-actions-b4f5548579`, and its own
CI run is green. It was not taken, because a dependency bump is a change to the
build rather than to the work this session was given.

## One behaviour change worth the operator's eye

**Every peer id `bit-cli` emits changed.** A tracker that was counting this
client as BitComet or as rqbit will count it as an unknown client called `CL`
until somebody's table gains a row, which is the correct answer and is what
[T-236](peers.md) was for. A tracker that keys a peer record by peer id sees
the change as a new peer on the first announce after upgrading, once.

Nothing else about an announce changed: `scripts/check-announce.ps1`'s six
judged cases all hold and the query parameter order is the same.
