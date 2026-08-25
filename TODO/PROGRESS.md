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
one. A short soak is a different thing and a session may run one: this session
ran five, the longest twenty minutes, all of them inside its own window.

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

- **Last session:** 2026-08-25T01:19:24Z, unattended, and the operator ruled on
  all three open questions in the kickoff: every recommendation is accepted.
  The duration is not restated here: `scripts/session-report.ps1` derives it
  from the instant above, and a duration written down twice is a number two
  documents disagree about.

  **The plan was written before starting**, per [RULES.md](RULES.md) section 1
  step 4: the work order's items 2, 3 and 4 in that order, which the three
  accepted rulings had unblocked. **It held**, and the only thing added to it
  was the operator's, twice, mid-session: promote what a scratchpad script did
  into the repository, and wire a manual step into the gates. Both shipped.
- **Tests:** 1,370 passing, 0 failing, up from 1,361. Plus **153** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run. `vendor/` did not move this session.

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **Gates:** clean, on rustc 1.98.0. A default run prints **ten**: `text`,
  `eol`, `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`.
  `eol` is this session's and `-Fix` is what it wants.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-two** jobs. Green at run **32807034096**, against commit
  `fb5a39f`, with all twenty-two green. Five runs this session and four were
  green; run **32806330167** against `f44d5c0` failed on one job,
  `Create round trip (ubuntu-latest)`, because this session's new interop case
  polled a Windows-only cmdlet. The fix is in `fb5a39f` and the review section
  below carries the finding.

```bash
gh run list --limit 1
```

- **Soak:** nothing six hours long ran this session. Five short ones did, and
  four of them are committed as evidence: two three-minute churn runs for
  [T-232](memory.md) and two twenty-minute runs either side of one line for
  [T-258](cli-surface.md).

```bash
pwsh -NoProfile -File scripts/soak.ps1 -ReadCsv bench/soak-20260825T014217900Z.csv
```

- **Entries:** 208 items. 29 open, 3 partial, 0 blocked, 165 done, 11 deferred
  to Phase C. 165 of 197 workable done, 32 left.
- **Tree:** 99 Rust files, 61,217 lines of code, 16,123 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. [`reference-map.md`](reference-map.md) carries the
  licence per tree and where the determination came from. Nothing was mined
  this session and nothing was read from it: every entry was about this tree's
  own surface.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **32 patches**
  across twenty-two sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  Unchanged: nothing under `vendor/` moved.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Four entries closed, one filed, and two tools the operator asked for
mid-session.** The three open questions were all ruled on in the kickoff and
every ruling was the recommendation, so the middle three items of the work
order were unblocked before the session started.

**The plan held and the order was the work order's**, with one addition: the
operator asked twice, mid-session, for scratchpad work to be promoted into the
repository. Both asks are shipped and both are documented where an agent is
told to look.

### [T-232](memory.md), P1: the listener figures reach the report, and a stop names its own side

`scripts/soak.ps1` reads the `listener` block out of the seeder's own progress
events, the same events `peak_rss_bytes` already came from, and carries it into
`self_reported`, into three new CSV columns, and into `-ReadCsv`. A run without
`-ListenerCheck` writes null and empty columns, so a reader tells "not watched"
from "watched and fine" without going to `parameters`.

**The last event's values are not enough and one of the two runs proves it.**
The heavy churn run ends at `"healthy": true` having failed three probes and
been unhealthy at `t+40s`. `worst_consecutive_failures`, `unhealthy_events` and
`first_unhealthy_elapsed_s` sit beside the last values for that reason.

**Both attribution branches ran**, three minutes each, because the spontaneous
stop of 2026-08-23T15:47:16Z cannot be summoned and its shape can:

| run | listener | what the report says |
| --- | --- | --- |
| `bench/soak-20260825T013344925Z` | 13 probes, **3 failed** | the fault is the seeder's |
| `bench/soak-20260825T014217900Z` | 7 probes, **0 failed** | the fault is not the seeder's accept path |

### [T-257](cli-surface.md), P2: one event type, two shapes, and the section says which is which

A `Sample` keys its commands and records, per field, which of them wrote it, so
a section for a shape two commands produce cannot be rendered as a union
credited to one. `docs/schema.md`'s `progress` section names both commands
above the table and carries a `from` column, reading `both` or `all` where
every command writes the field.

**The Approach asked for a guard that panics on `progress` and that is not what
shipped**, because under the accepted ruling a shared `type` is legal and a
panic would refuse what the ruling permits. Attribution removes the failure
mode rather than detecting it. `fold_document`'s panic for a document `kind` is
unchanged.

**Two more shapes were being unioned and the entry named neither.**
`session_start` differs in five of nine fields between `download` and `seed`,
and `session_end` comes from **four** commands with `error` written by
`bit-cli info` alone.

### [T-258](cli-surface.md), P2: a tick carries what is connected

`swarm::currently_held` drops the two terminal states, `dead` and `not needed`,
so a tick's `peer_detail` is `peers.live + peers.connecting + peers.queued` from
the same event. It was a length nothing in the event described.

Two twenty minute soaks, four leechers, one binary either side of one line:

| | before | after |
| --- | --- | --- |
| seeder stdout | 1,046,872 bytes | **16,993 bytes** |
| last record | 50,649 bytes | **410 bytes** |
| rows in it | 160 | **0** |
| leech cycles | 160, none failed | 156, none failed |

### [T-241](metainfo.md), P2: nine commands take a magnet, where eight exited 4

The ruling was option two, so the swarm-backed path is under
`source::resolve_source` rather than on `bit-cli magnet` alone. `info`, `files`,
`tree`, `verify` and the four `webseed` subcommands all exited 4 through the one
door; `bit-cli magnet` is the ninth and answered from the URI's own fields.
`resolve_from_swarm` starts a session with a temporary directory, adds the
source with `list_only`, and parses the bytes it assembled.

`SwarmSourceArgs` is `--peer`, `--no-dht`, `--no-lsd` and `--no-tracker` under
a "Resolving a magnet" heading, flattened into `info`, `files`, `tree`,
`magnet`, `verify` and the four `webseed` subcommands, **last in each struct**
because `next_help_heading` applies from where it appears onward. `trackers`
does not get it: it flattens `TrackerArgs`, which defines `--no-tracker`, and
it does not need one.

`bit-cli magnet --output` writes the resolved metainfo, with `-` for stdout and
`--force` to overwrite. `scripts/interop-roundtrip.ps1` has a fourth case for
it and `aria2c` opens what it writes: **4 of 4 cases round tripped**.

### [T-259](cli-surface.md), P3: filed, and this session's own edit found it

The schema test compares field rows only, so an edit to the generator's own
prose never reaches `docs/schema.md` and nothing fails.

### The two tools, and both were the operator's ask

- **`scripts/set-status.ps1`** is the writer for the numbers `check-todo.ps1`
  reads. Closing one entry moves seven of them across two files and every
  session has done that by hand. `-Entry` with `-Status` moves a row and
  re-derives every count from the rows, `-Recount` does the counts alone,
  `-Check` writes nothing. It does not touch the entry's own `Status:` line,
  which is prose, and prints whether that line agrees.
- **`scripts/check-eol.ps1`** is the `eol` gate. `.gitattributes` normalises
  the index, so a file written with CRLF commits as LF and `git diff` shows
  nothing; what it does not normalise is the working tree, which is what every
  `(?m)^...$` here actually reads. Measured before the gate existed: **99**
  tracked files disagreed, `TODO/create-seed.md` was **mixed**, and ten `.rs`
  files under `crates/` were CRLF. `-Fix` rewrote all of them and the staged
  set did not change by one file, which is the proof that it repairs a working
  tree and never a commit. `vendor/` is reported and left alone, because
  `vendor-diff.ps1` derives the series with `git diff --no-index`.

### What the reviews found

**Review 1, every claim against the code it cites.** Five things, and three
changed what a document says rather than how it reads.

- **A number two documents disagreed about.** The code comments said `progress`
  differs in "nine of seventeen fields", the entry's own measurement; the
  committed section shows **fifteen of thirty-two**, because the generator's
  `seed` run passes `--listener-check` and an ordinary one does not. The
  comments quote the section's figure now and the entry says why both are right.
- **[T-232](memory.md) claimed both attribution runs exit 1** when only one had
  its exit code read unpiped. That is [RULES.md](RULES.md) section 4a's own
  rule and it was broken in the same session that quotes it.
- **`soak.ps1`'s churn comment compared two runs of different lengths.** 22
  cycles over two minutes against 26 over three is not a comparison; zero
  failures against two out of three is, and that is what the comment says now.
- Two citations off by ninety-seven lines after `cli.rs` grew, and seven more
  the `record` gate caught on the same push.
- One `schema_gen.rs` citation off by sixty-seven lines.
- **Two figures in this session's own commit messages are wrong**, both caught
  after the push that carried them and neither correctable: `7b36a12` says
  seven `.rs` files were CRLF where ten were, and `f44d5c0` says five commands
  exited 4 on a magnet where eight did. The entries and
  [RULES.md](RULES.md) carry the right numbers, and this line is here because a
  reader comparing a commit message against a file would otherwise find two
  disagreements and no explanation.

**Review 2, a cold read.** Two things, both in `docs/`.

- `docs/examples/inputs.md` said a magnet is refused by the read-only commands
  and that `bit-cli download` is what does the lookup. Rewritten.
- The short-flag table said `-o`/`--output` is on "create, edit, man". `magnet`
  is on it now, and nothing mechanical checks that column: the test compares
  the `(letter, name)` pair, which was already there.

**Review 3, and it was CI's.** The magnet interop case waited for its seeder
with `Get-NetTCPConnection`, which does not exist on Linux, so
`Create round trip (ubuntu-latest)` failed at run **32806330167** and the
Windows job passed. It waits on the seeder's own first `progress` event now,
which is cross-platform and is also the stronger condition: a bound port is not
a session ready to answer, which is [T-221](windows.md). Every gate here is
Windows only by construction, so the only thing that could have caught this is
a run, and it took one.

**Review 4, and it was the prose gate's.** The first draft of the `docs/`
section for [T-258](cli-surface.md) said a tick's array "is smaller than it
used to be". `check-docs.ps1` failed it twice on one line: `docs/` says what
the tool does, not what the project did.

## In progress

Nothing is half-written. All four closed entries closed complete, with the
acceptance run and its output recorded in the entry.

- **[T-251](trackers.md)** is `partial`, untouched this session: the source
  half is done and the per-tracker knobs are not.
- **[T-253](cli-surface.md)** is still `partial`, untouched, and still needs a
  certificate generator its acceptance forbids fetching.
- **[T-164](peers.md)** is still `partial`, untouched.
- The entries the last sessions left open are untouched except where named
  above: [T-233](peers.md), [T-239](peers.md), [T-240](dht.md),
  [T-101](bep-coverage.md), [T-102](bep-coverage.md), [T-168](bep-coverage.md),
  [T-244](cli-surface.md), [T-248](metainfo.md), [T-250](cli-surface.md).
- **[T-243](phase-c.md)** is deferred and the operator has deliberately not
  ruled on it. Do not raise it.

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

2. **[T-259](cli-surface.md), P3, `S`, and it is this session's own filing.**
   The smallest thing open and the cheapest to prove: compare the non-row lines
   of `docs/schema.md` for equality while keeping the row lines as containment,
   with the hand-written tail `carry_across` preserves exempt. The entry names
   both seams with their lines.

3. **[T-250](cli-surface.md), P2.** Cheaper than it was and now much cheaper
   than that: `Kind::classify` produces three distinct refusals, and this
   session gave `source_kind` a fifth value that a report can carry, because
   `info` over a magnet says `magnet` where the same torrent on disk says
   `file`. What is left is deciding what "how it was resolved" means for a
   fetch and for a swarm lookup, and both now exist to describe.

4. **[T-251](trackers.md), P2, `M`, `partial`.** What is left is the entry's
   own subject: a `[[tracker]]` table in the file `--web-seed-config` reads,
   with `url`, `tier`, `timeout`, `connect_timeout`, `interval`, `enabled` and
   `key`, then the `[[peer]]` table after it. The Acceptance is unchanged and
   `scripts/check-announce.ps1` is where the case goes.

5. **[T-244](cli-surface.md), P2.** The ruling is static extraction with a
   browser-shaped header set and a `--render` opt-in. It is the last source
   kind `docs/examples/inputs.md` lists under "What is not an input yet", and
   that section is down to one item because of this session.

6. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

7. **The three entries that were ruled on and are still work.**
   [T-227](memory.md) is a throughput curve then a flag.
   [T-242](performance.md) is two sweeps from `scripts/bench-leech.ps1`.
   [T-234](peers.md) and [T-238](peers.md) are the two large ones and both need
   [T-239](peers.md) first. T-234 is cheaper than it was: `--as-client` is a
   second value for `bit_cli_core::peer_id::CLIENT_CODE` and its version
   characters, and that module is the only place either is read from.

8. **Then the category pass, and `bep-coverage.md` is still first.**
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

**None.** All three of the last session's were ruled on in this session's
kickoff and all three were the recommendation. Nothing this session found needs
a decision: [T-259](cli-surface.md) is filed with one approach and no fork, and
every entry closed on its own acceptance.

**Three things to be aware of rather than to decide.**

**[T-253](cli-surface.md) cannot close without a certificate generator.** Its
remaining half needs the loopback file server to speak TLS, its acceptance
forbids the network, and nothing in this tree can make a self-signed
certificate: `rustls` and `tokio-rustls` are workspace dependencies already and
`rcgen` is not one. The options are a new dependency or a checked-in test
certificate that expires. Unchanged, and nothing was decided this session.

**Twelve repositories in `TheDancingDeveloper-org`** redistribute Apache-2.0
code under MIT with no licence file and no attribution. Nothing was said to
anybody, by [RULES.md](RULES.md) section 6a, and it is in `RESEARCH.md` entry
40. Unchanged.

**One dependabot pull request is still open**, number 6,
`ci(deps): bump taiki-e/install-action from 2.86.3 to 2.86.5`. Not taken again,
for the same reason as the last two sessions: a dependency bump is a change to
the build rather than to the work the session was given.

## Behaviour changes worth the operator's eye

**Every command that reads a source reads a magnet.** `info`, `files`, `tree`,
`magnet`, `verify` and the four `webseed` subcommands exited 4 on a magnet or a
bare info hash and now join the swarm it names. **That means they can touch the
network where they could not before**: the DHT and local discovery are on by
default, the same as `download`, and `--no-dht --no-lsd --no-tracker` with
`--peer` leaves a swarm of exactly the addresses on the command line. A script
that branched on exit 4 to mean "this is not a source I can read" will now get
a real lookup. The deadline is `--timeout` where set and 60 seconds otherwise,
and running out is exit 9.

**`bit-cli magnet` grew `--output`, `--force` and the four swarm flags.**
Without `--output` the command is unchanged and still costs nothing: it reads
the URI and reports it, no swarm at all.

**A `seed --jsonl` progress tick carries the peers currently held.** A consumer
reading `peer_detail` off a tick to count a swarm gets a smaller number;
`peers.seen` is in the same event and is the count that field never was. The
final document is unchanged and still carries every peer.

**`docs/schema.md`'s `progress`, `session_start` and `session_end` sections
carry a third column.** No event's fields changed and `schema_version` did not
move: what changed is that the file says which command writes which field.

**`scripts/gates.ps1` prints ten gates rather than nine**, and `-Fix`
normalises line endings as well as formatting and regenerating the manuals.
