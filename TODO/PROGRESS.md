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
one. This session is the worked example: the operator's run was in flight for
its whole window and it read it at the end.

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

- **Last session:** 2026-08-24T21:01:41Z, unattended, and the operator's six
  hour soak was in flight for its whole window. The duration is not restated
  here: `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.

  It wrote the plan down before starting, per [RULES.md](RULES.md) section 1
  step 4: the soak was already running, so it took the work order's item 4
  first, read the soak when it landed, and re-estimated item 3 rather than
  opening it. The plan held.
- **Tests:** 1,361 passing, 0 failing, up from 1,359. Plus **153** in the
  vendored `rqbit` tree, up from 149, and **76** in `librqbit-utp`, which the
  workspace gates do not run. `vendor/` moved this session, for
  [T-256](trackers.md).

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **Gates:** clean, on rustc 1.98.0. A default run prints **nine**: `text`,
  `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-two** jobs. Green at run **32787614038**, against commit
  `acfd173`, which is this session's last push. Three runs this session and all
  three green, but only the third has all twenty-two jobs green: the first two
  had `Clippy (tracking beta)` failing on this session's own new code, which
  is what that job is for. It does not fail the run.

```bash
gh run list --limit 1
```

- **Soak:** the operator's run of 2026-08-24T16:46:05Z finished inside this
  session's window and **closed [T-224](memory.md)**. Six hours,
  `-Leechers 4 -ListenerCheck 60s`, 704 samples over 5.9999 hours, **2,812
  leech cycles completed and none failed**, every named ceiling held, and
  `tcp_close_wait` was 0 at every sample. `bench/soak-20260824T164609340Z` is
  committed as the evidence for both entries it was started for.

```bash
pwsh -NoProfile -File scripts/soak.ps1 -ReadCsv bench/soak-20260824T164609340Z.csv
```

  **[T-232](memory.md) stays open and what it needs changed.** The stop did not
  reproduce, so neither branch of its acceptance fired. What the run did find is
  that a finished soak records that `--listener-check` was on and never records
  what it saw: no `listener` key in the report, no listener column in the CSV,
  and `.tmp/soak/` deleted at the end. So that branch could not have been
  answered even if the stop had happened. The entry now waits on one field
  rather than on a lucky run.

- **Entries:** 207 items. 32 open, 3 partial, 0 blocked, 161 done, 11 deferred
  to Phase C. 161 of 196 workable done, 35 left.
- **Tree:** 99 Rust files, 60,566 lines of code, 15,826 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. [`reference-map.md`](reference-map.md) carries the
  licence per tree and where the determination came from. Nothing was mined
  this session and nothing was read from it: every entry was about this tree's
  own surface or the operator's soak.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **32 patches**
  across twenty-two sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  One patch and one section are this session's, for [T-256](trackers.md), and
  `patches/TASKS.md` carries its row.
- **Version:** `bit-cli` 0.2.0, unchanged. `CHANGELOG.md` gained three sections
  under the unreleased heading.

## What the last session did

**Three entries closed, one of them filed and closed on the way, two filed, one
re-estimated, and one advanced to partial.** Nothing was mined from the corpus
and nothing needed to be: every entry was about this tree's own surface or the
operator's soak.

**The order was the work order's, with one substitution.** Item 4,
[T-237](trackers.md), went before item 3, [T-241](metainfo.md), because the
soak was in flight for the whole session and T-237 touches nothing it uses,
while T-241 is the item the work order itself said to re-estimate before
opening. That re-estimate is below and it turned T-241 from `S` into `M`.

### [T-237](trackers.md), P2: three announce paths, and the third was hiding a defect

`scripts/check-announce.ps1` has **nine judged cases where it had six**.
`redirect` follows a `302` and compares the three numbers across the hop,
`failure-reason` proves a rejection at HTTP 200 is reported as a failure, and
`udp` is the same six assertions over a BEP 15 announce rather than a second
set. `bench/announce-20260824T222123899Z.json` is the run.

`loopback-tracker` grew `--redirect-announce <N>`, `--fail-announce <REASON>`
and a UDP socket speaking BEP 15, refusing a connection id it never issued.
Both flags record the announce **before** refusing or redirecting it, which is
what lets the check tell a rejection apart from a request that never arrived.

### [T-256](trackers.md), P1: filed and closed, and the `udp` case is what found it

The vendored UDP announce loop read the BEP 3 event off the torrent's **current
state** on every announce, where the HTTP loop in the same file sends `started`
once and nothing after. One client, one 22 second run, the same payload over
both protocols:

| protocol | events the tracker recorded |
| --- | --- |
| udp | `started`, `started`, `started`, `started`, `started`, `stopped` |
| http | `started`, none, none, none, none, `stopped` |

With a seeder present the leecher sent `completed` four times, and the seeder,
which had the whole payload before it started, sent it on every announce, which
BEP 3 says it must not send at all. The cost is the tracker's and is invisible
here: `completed` is how a tracker counts finished downloads.

**Run against the defect, rebuilding in between**: the `udp` row reads
`1 of 5 failed: completed (4 completed events, and BEP 3 asks for one)` and the
check exits 1. Five judged rather than six is part of the evidence, because
with every announce carrying an event the `interval` case has nothing to
measure.

**`completed` is not sent from the loop at all**, which is the half worth
arguing about. The HTTP monitor has no `Completed` arm, and `bit-cli` announces
its own completion at the instant it happens; a `completed` in the loop too
made one run tell the tracker twice, which was measured before it was removed.

### [T-251](trackers.md), P2: advanced to partial, and it is the half [T-245](cli-surface.md) left

`bit-cli trackers` was the one command that refused a `.torrent` named by a
URL, saying the source carried no info hash while the document behind it
carries one. It reads five kinds through `source::resolve_source` now, the same
one line `info`, `files` and `tree` use, and a URL and the file on disk produce
the same report. `left` is what proves the fetch happened: 131,072 bytes either
way, a number nothing but the metainfo could supply.

The entry's own subject, the per-tracker table of timeouts and intervals, is
untouched, so it is `partial` rather than `done`.

### [T-241](metainfo.md), P2: re-estimated from `S` to `M`, and not started

The work order asked for this and it changed both the size and the shape.
Writing a `.torrent` out of a resolved magnet is a few lines; getting a
resolved magnet inside `bit-cli magnet` is the entry, because that command has
no swarm, no tracker and no DHT anywhere in it.

**One premise the entry wrote down is false.** It said
`bit-cli info <magnet> --json` already prints what was resolved. Measured:
`info`, `files` and `tree` all exit 4 on a magnet with the same refusal, and
only `bit-cli magnet` answers, from the URI's own fields. The correction is
under the entry rather than over it.

### [T-257](cli-surface.md) and [T-258](cli-surface.md), P2 each: filed from the running soak's output

**T-257**: `seed --jsonl` and `download --jsonl` both emit `type: "progress"`
and differ in nine of seventeen fields, and `docs/schema.md` credits the union
to one command. `fold_document` panics when two commands claim one `kind`;
`observe_events` keys by `type` and merges whatever arrives, with no such
check. It is [T-191](bench.md) one layer down, and T-191 predicted it.

**T-258**: a seeder re-sends every peer row it holds on every tick. 151,679,859
bytes of stdout over 666 records at 5.5 hours, for a 16 MiB payload.

**T-258's own premise was corrected in the same session, by the review.** It
said the total grows with the square of the run length. It does not: the record
grows with `peers.seen` for two hours, then the row count stops following
`seen` and the size plateaus at about 270 KB. So the finding is a constant
32 MB an hour rather than a curve, which is the more useful claim. And the
plateau is at 894 rows, below [T-040](memory.md)'s 1,024 ceiling, so it is the
reclaim keeping up rather than the cap engaging.

### [T-224](memory.md), P2: closed on the operator's soak, and the step did not reproduce

The entry's second half offered two ways to finish, and the run took the
second: two runs at different leech rates showing the step is not tied to
completed work.

| | committed, 2 leechers | this run, 4 leechers |
| --- | --- | --- |
| samples | 681 over 5.992 h | 704 over 5.993 h |
| leech cycles | 1,360 completed, 0 failed | 2,812 completed, 0 failed |
| `rss_bytes` per hour | 3.71 MiB, r2 0.72 | 1.81 MiB, r2 0.65 |
| largest single rise | **11.61 MiB at t+1.16 h** | 7.82 MiB at t+4.92 h |
| largest single fall | -7.23 MiB | -7.13 MiB |

**There is no step in this run**, and that is the finding. The committed run's
is one move that stays: 15.68, 15.85, then 27.46, and 27.51 and 27.63 after it.
This run instead oscillates from `t+1.045 h` onward, 126 single interval
changes over 3 MiB, every rise matched by a fall of nearly the same size within
a sample or two. The floor of that band holds between 16.5 and 19.3 MiB while
its ceiling drifts from 20 to 26.

The cycle counts are what rule out completed work as the cause: the committed
run's step lands at **264** cycles, and this run passed 264 cycles inside its
first thirty-five minutes with nothing happening. Its own largest rise is at
2,332.

It does not name the allocation, and the entry says so. What it answers is the
question it was filed for: the reported slope was not describing a leak tied to
work.

### What the reviews found

**Review 1, every claim against the code it cites.** Five errors, all in this
session's own writing, and two of them changed what an entry says rather than
how it reads.

- **[T-258](cli-surface.md)'s premise was wrong**, above. Its first table also
  mixed two instants and presented them as one reading, which is how the error
  survived being written down.
- **894 rows was called [T-040](memory.md)'s bound engaging.** T-040's bound is
  1,024 and the table never reaches it.
- Two line citations off by one or two: `magnet.rs:79` for a `run` at 80, and
  `schema_gen.rs:120` for an `observe_events` at 122.
- A detail string in `check-announce.ps1` said "of six" with the six written
  out, so a seventh case would have left it saying six forever. Counted now,
  and the check was re-run so the entry quotes what it prints.

**Review 2, a cold read.** Three things, and the third is the one a reader
would have tripped on.

- `docs/examples/inputs.md` carried a paragraph saying `trackers` is the one
  command that still refuses a URL. It is not, as of this session.
- The `bench/` evidence file's name said `214800Z` and its `generated_at` said
  `222123899Z` after the re-run. Renamed to match.
- Both the fixture's own module documentation and
  `docs/examples/interop.md` said the UDP socket is on the same port as HTTP.
  It asks for that port and does not always get it, which the fixture's own
  fallback exists for, so both now say to read the printed line.

**Review 3, and it was CI's rather than a person's.** The `Clippy (tracking
beta)` job failed on the first push, on this session's own new code:
`AtomicU32::fetch_update` is deprecated on beta in favour of `try_update`,
which is not on stable yet. Neither name is portable, so the fixture uses the
compare and exchange loop both of them wrap. That job fails without failing the
run, which is what made the warning arrive while the code was still in hand.
It is the second time it has paid, after [T-218](cli-surface.md).

**And a measurement that disproved a suspicion rather than a premise.** A
`download` with nothing to talk to looked like it exited 0, which would have
been a defect worth an entry. It exits **9**. The 0 was the measuring script's:
`Start-Process -Wait` does not set `$LASTEXITCODE`, so what was read was the
exit code of whatever ran before it. Nothing was filed and nothing needed
fixing.

## In progress

Nothing is half-written. Both closed entries closed complete, with the
acceptance run and its output recorded in the entry.

- **[T-251](trackers.md)** is `partial`: the source half is done and the
  per-tracker knobs are not. The Acceptance is unchanged and is about the
  knobs.
- **[T-253](cli-surface.md)** is still `partial`, untouched, and still needs a
  certificate generator its acceptance forbids fetching.
- **[T-164](peers.md)** is still `partial`, untouched.
- **[T-232](memory.md)** is open and what it waits on changed: not a
  recurrence, but the listener figures reaching the soak report. The section
  under it says where they already exist and where they have to go.
- The entries the last sessions left open are untouched except where named
  above: [T-233](peers.md), [T-239](peers.md), [T-240](dht.md),
  [T-101](bep-coverage.md), [T-102](bep-coverage.md), [T-168](bep-coverage.md),
  [T-244](cli-surface.md), [T-248](metainfo.md), [T-250](cli-surface.md).
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

2. **[T-232](memory.md)'s one field, and it goes first because it is the
   smallest thing that unblocks a six hour run.** A soak with
   `-ListenerCheck` records that the flag was on and nothing about what it saw.
   `self_reported` in `scripts/soak.ps1` is already built from the seeder's own
   progress events, and `listener` arrives in the same event, so carrying it
   through is one field. Until it is there, the next operator soak cannot
   answer the question T-232 asks even if the stop reproduces.

3. **[T-257](cli-surface.md) and [T-258](cli-surface.md), both P2 and both
   `S`.** They are this session's own filings and the cheapest things open.
   T-257's guard is a few lines modelled on `fold_document` and it fails on
   `progress` the moment it exists, which is what makes its second half
   unavoidable; that second half is a three-way decision with a recommendation
   already written. T-258 is one line at `crates/bit-cli/src/cmd/seed.rs:502`
   plus a decision about what a tick owes, and it wants a **20 minute** soak
   before and after rather than a six hour one.

4. **[T-241](metainfo.md), P2, now `M` rather than `S`.** Re-estimated and not
   started. The fork under the entry is the thing to settle first: `--output`
   on `magnet` alone, or a swarm-backed path under `source::resolve_source` so
   `info`, `files` and `tree` stop exiting 4 on a magnet. Two is recommended on
   [T-245](cli-surface.md)'s own argument and it is the larger job.

5. **[T-251](trackers.md), P2, `M`, and now `partial`.** What is left is the
   entry's own subject: a `[[tracker]]` table in the file `--web-seed-config`
   reads, with `url`, `tier`, `timeout`, `connect_timeout`, `interval`,
   `enabled` and `key`, then the `[[peer]]` table after it. The Acceptance is
   unchanged and `scripts/check-announce.ps1` is where the case goes.

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
which carries the actual code lines. **All of it is a read.** Nothing was read
from it this session.

## Open questions for the operator

**Three, and all three are forks with a recommendation already written under
the entry.** None of them blocks the next session: each entry says what to do
if no ruling arrives, which is to take the recommendation.

**[T-241](metainfo.md): where does magnet resolution live?** `--output` on
`bit-cli magnet` alone, which leaves `info`, `files` and `tree` exiting 4 on a
magnet, or a swarm-backed path under `source::resolve_source` so every command
that reads a source can take one. **Recommended: the second**, on
[T-245](cli-surface.md)'s own argument, that a source kind one command accepts
and four refuse is the defect T-245 closed. It is the reason the entry is now
`M`.

**[T-257](cli-surface.md): two documents share one event `type`.** Rename one
and break every consumer selecting `progress`; emit the union from both
commands with nulls; or keep one `type` and record that it has two shapes.
**Recommended: the third**, because T-191 took the identical fork the same way
for `kind` and because breaking the wire format is what `schema_version` is
for.

**[T-258](cli-surface.md): is narrowing `peer_detail` on a tick a break worth
making?** A tick would carry the peers currently connected rather than every
peer ever seen, and the final document would keep carrying all of them.
**Recommended: yes.** At the last sample of the soak, zero of the 873 rows sent
every 30 seconds were connected peers.

**Three things to be aware of rather than to decide.**

**[T-253](cli-surface.md) cannot close without a certificate generator.** Its
remaining half needs the loopback file server to speak TLS, its acceptance
forbids the network, and nothing in this tree can make a self-signed
certificate: `rustls` and `tokio-rustls` are workspace dependencies already and
`rcgen` is not one. The options are a new dependency or a checked-in test
certificate that expires. Nothing was decided and nothing changed this session.

**Twelve repositories in `TheDancingDeveloper-org`** redistribute Apache-2.0
code under MIT with no licence file and no attribution. Nothing was said to
anybody, by [RULES.md](RULES.md) section 6a, and it is in `RESEARCH.md` entry
40. Unchanged.

**One dependabot pull request is still open**, number 6,
`ci(deps): bump taiki-e/install-action from 2.86.3 to 2.86.5`, with its own CI
run. It was not taken again, for the same reason as last session: a dependency
bump is a change to the build rather than to the work this session was given.

## Behaviour changes worth the operator's eye

**A UDP announce carries three events and no more.** `started` once,
`completed` once when a download finishes, `stopped` at the end, and nothing on
the announces in between. It repeated `started` at every interval while
downloading and `completed` at every interval afterwards, and a seeder sent
`completed` at all, which BEP 3 forbids. Nothing about an HTTP announce
changed, and the three events a run sends are the same three it sent before.
A tracker's completed count for this client will stop climbing.

**`bit-cli trackers` reads a `.torrent` named by a URL or a metalink.** It
exited 4 on both before, saying the source carried no info hash. A script that
branched on 4 to mean "this is not a source I can announce for" will now get a
real announce, which is the point. A magnet and a bare info hash are unchanged.

**`loopback-tracker` prints a third URL.** The IPv4 HTTP URL is still the first
line, which is what `scripts/soak.ps1` and `scripts/interop-roundtrip.ps1`
read, and `scripts/check-tracker-family.ps1` still finds exactly two `^http`
lines. The `udp://` line is last, and its port is not always the HTTP one:
Windows reserves twelve UDP port ranges and a bind inside one fails, so the
fixture falls back to a free port and prints what it got.

The peer id change from two sessions ago is unchanged and still worth knowing:
every peer id `bit-cli` emits is `-CL0200-` now.

