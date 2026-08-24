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

**This session paid for that rule twice**, both times by guessing: `create
--tracker` does not exist and the flag is `--announce`, and a scope selector is
`SELECTOR=URL` rather than `URL=SELECTOR`. Both cost a run that exited 2.

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
[T-238](peers.md) carries the recommendation.

**Decision 7.4, no daemon and no RPC, was not reopened** and this session did
not treat it as reopened. [T-243](phase-c.md) is the draft that collides with
it, and it says so in its own first paragraph.

## State

- **Last session:** 2026-08-24T07:51:30Z, unattended, documentation and
  research. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.
- **Tests:** 1,298 passing, 0 failing, unchanged. No `crates/` source changed:
  the one Rust file this session touched is a fixture,
  `crates/bit-cli-core/examples/loopback-tracker.rs`. Plus **149** in the
  vendored `rqbit` tree and **76** in `librqbit-utp`, which the workspace gates
  do not run. `vendor/` is untouched.
- **Gates:** clean, on rustc 1.98.0. A default run prints **nine**: `text`,
  `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`, `deny`. `docs` is
  new, from the docs gate below.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** **twenty-two** jobs now, `Docs` added beside `Record`, `Tree` and
  `Soak fit`. Green at run **32713374341**, against commit `9faff24`. No run
  was red this session. Two pushes carried `-NoCi` because every staged path
  was documentation.

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

- **Entries:** 192 items. 31 open, 1 partial, 0 blocked, 149 done, 11 deferred
  to Phase C. 149 of 181 workable done, 32 left.
- **Tree:** 97 Rust files, 58,058 lines of code, 14,899 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries, up from
  twenty-two. Plus `reference/HISTORY/`, which is new.
  [`reference-map.md`](reference-map.md) carries the licence per tree and where
  the determination came from.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  Untouched.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

Documentation and research. **Nine entries filed, one closed, none of the
open ones worked on**, which was the assignment: this session writes prose,
records and tooling, and files the work rather than doing it.

Three pieces of tooling were built and run rather than filed:
`scripts/make-client-profile.ps1`, `scripts/check-announce.ps1` and
`scripts/check-docs.ps1`.

### What was read, and what it changed

Every prerequisite was read in full, and three of them moved a decision.

**`man/bit-cli.json` changed two entries before they were written.**
[T-234](peers.md)'s flag surface was going to include a short flag until the
short-flag list showed `-c` is `--config` and the obvious letters are taken,
and [T-239](peers.md) was going to be `bench probe` until the same file showed
that command is a one-shot probe of a peer or an endpoint and not of the local
network.

**`TODO/RULES.md` section 5's line about a test that waits on a condition
changed how the new check is written.** `scripts/check-announce.ps1`'s
`interval` case allows one second of slack and its `totals-match` case asserts
a bound rather than an equality, both because the strict version would be
asserting a scheduling outcome. That is the rule's fourth worked example.

**`TODO/INDEX.md`'s "How an ordering is derived" changed the rewrite of
itself.** It says the argument is worth keeping and the ordering is not, so
the rewritten file keeps one argument and the 2026-08-21 derivation went to
`reference/HISTORY/`.

The two attached procedure documents changed the shape of
`docs/reference-mining.md` rather than its content: they put the SHA rule
before everything else, and this corpus had recorded no SHA for any of its
first twenty-two trees, so that ordering is now the first thing that page says.

`reference/RESEARCH.md` and `reference/README.md` changed nothing about the
plan and were the material for most of it, which is what reading them was for.

### The corpus, and the operator's claim that did not hold

**Seventeen trees added, an organisation triaged, a survey read.** The corpus
is thirty-nine trees in forty-one entries. Every new entry carries a commit
SHA, a licence with its evidence, the passes taken, a verdict, and what the
pass did not do.

**The SHAs are the first this corpus has ever recorded.** The 2026-08-21 pass
captured none and stripped `.git`, so the three re-mines could establish only
that nothing had changed, not what had. `seedchamp`, `n0-mainline` and
`intermodal` are byte-identical to their current upstream HEAD in every file
the corpus kept.

**The operator's claim about `TheDancingDeveloper-org` is wrong and the entry
says how.** Of 33 repositories, four are copyleft, three carry no licence
statement anywhere, and nine declare a licence only in a `Cargo.toml`. Twelve
are `librqbit` renamed to `librtbit` and declared MIT: the crate list matches
this tree's name for name, and their `peer_connection.rs` is this tree's minus
this repository's own patches. `librqbit` is Apache-2.0. Nothing was taken and
nothing was said to anybody, by section 6a.

**One licence record here was wrong too.** `n0-mainline` is `MIT OR
Apache-2.0`; the corpus copy kept only `LICENSE-MIT` and the record said MIT
alone. That is now a rule in `docs/reference-mining.md`: keep every licence
file, not the first one.

### [T-234](peers.md), P2, filed: what a client mask has to carry

**A profile is not a string, and five implementations of one format disagree
about what it means.** `joal` is the origin and `DOAL`, `Seedr`,
`fake-torrent-client` and `rustatio` are the reimplementations.

**All four implementations of qBittorrent's `key` are wrong in the same
direction.** libtorrent writes `key=%08X`, so a real key starts with `0` one
time in sixteen, and none of the four can produce one: one strips leading
zeros, one rejects and regenerates, one replaces the first character, and one
skips the digit at every position on a truncated alphabet. Each reproduced a
format that encodes an algorithm named after a rule the client does not have.

`scripts/make-client-profile.ps1` derives a profile from the client's own
tagged source instead. It agrees with `joal`'s committed prefixes for
qBittorrent 4.6.7, 5.0.0, 5.1.4 and 5.2.3, and exits 2 on 4.1.9 and 3.3.16
because `src/base/version.h.in` is not in those tags, which was checked rather
than assumed.

### [T-235](trackers.md), P1, filed and done: the announce agrees with the run

`scripts/check-announce.ps1`, six cases, all judged and all holding.
`loopback-tracker` gains `--announce-log`, which records the **raw query** as
received because the parser's `BTreeMap` has already sorted the parameter order
away and order is what a tracker fingerprints.

**The six cases passed and the run still found something**, which is the third
time this repository has had that shape. The announce carries the peer id
prefix `-rQ9010-`, the vendored engine's own, and `bit-cli trackers` uses
`-BC0100-`, which libtorrent's `src/identify_client.cpp:161` maps to
**BitComet**. One binary, two identities, neither of them this one, and the
comment above the second says it exists so a tracker attributes the announce
correctly. That is [T-236](peers.md) and its premise is disproved by the thing
it claims.

### [T-238](peers.md) and [T-239](peers.md), P2, filed: traversal, priced

The recommendation is measured rather than asserted: `iroh` 1.0.3 is `MIT OR
Apache-2.0` with 43 direct dependencies, and resolving it in a throwaway crate
outside the tree shows it brings **113 crates this tree does not have**,
against a current 302, replacing nothing.

It is still refused, on a better argument than the retired ruling's: an `iroh`
peer is an ed25519 node id and a BitTorrent peer is an `IP:port`, and there is
nowhere in BEP 5, BEP 11 or a tracker response to publish a node id another
client would understand. Every other mechanism degrades to plain BEP 55 and
plain TCP or uTP; that one does not degrade, it disappears.

**BEP 55 fails on exactly two NAT shapes**, symmetric and carrier grade, and
those are the two worth paying for. The per-shape table is in the entry.
**Nothing in thirty-nine trees classifies a NAT**, which is why
[T-239](peers.md) is new work rather than a port, and why it outranks
[T-238](peers.md) despite being the smaller idea.

### [T-241](metainfo.md), [T-242](performance.md), [T-237](trackers.md), [T-240](dht.md), [T-243](phase-c.md)

`bit-cli` already resolves a magnet to metainfo over BEP 9 from one peer with
no tracker, no DHT and no web seed, which was **measured rather than assumed**:
exit 0 and 2,097,152 bytes landed byte for byte. The gap is that nothing writes
the resolved metainfo back out, which is T-241.

T-242 is the request depth, a constant that [T-001](webseed.md) measured the
run sitting at 40 percent of. T-237 is three announce paths the new check does
not reach. T-240 is DHT node reputation. T-243 is the user interface draft,
deferred, with the 7.4 collision stated and `egui` recommended over `slint`.

### The record, cut to what a session can act on

**`INDEX.md` was 1,154 lines and is 316.** The list is sorted by id now rather
than grouped, so a row is findable by the id it is referred to by. What left is
in `reference/HISTORY/INDEX-history.md`: the triage that produced the first
hundred entries, the counts prose that narrated how the list grew, and the
2026-08-21 ordering with the record of what each closing measured.

**The ordering was re-derived**, which [RULES.md](RULES.md) section 1 step 2
asks a session to say. It gained a question. It used to ask three; the second
is new, "is a measurement blocked on something", because three of this
session's entries are open on a measurement rather than an implementation and
nothing in the previous derivation ranked that shape at all.

One rule was moved before the file that stated it lost it: a `Source:` line
records where an entry came from rather than a path a reader can open. It is in
RULES.md section 5.

**Twelve corpus subsections left `RESEARCH.md` for `reference/HISTORY/`**, each
because the entry it informed is closed and the behaviour was verified present
in `man/bit-cli.json` or at a path that was opened. Six were deliberately kept
although their entry closed, and section G says which and why. Section D was
rebuilt rather than edited, because half its rows pointed at closed work.

### `README.md` is a map, and `docs/` carries the detail

**83,841 bytes across 37 sections, to 12,530 across nine.** Nothing was
truncated: all 37 sections have a recorded destination, and the move was
checked twice, by grepping a distinctive phrase from each large section and by
extracting every command the old README showed and searching for it in the new
tree. That second check found three commands that had been lost, all of them
the loopback fixtures, and they are in `docs/examples/interop.md` now.

Twelve topic pages, three procedure pages and seven worked examples. **Every
command in every example was run before it was written down**, and two of them
were wrong when first written, which is the argument for the rule.

**Where `TODO/cli-surface.md`'s completed items went**, because it is 216 KB
and had no obvious single home:

| what | where |
| --- | --- |
| the `--jsonl` event stream, the schema, the dry run shape | `docs/schema.md` and `docs/examples/machine-output.md` |
| Metalink, and a Metalink named by URL | `docs/metainfo.md` |
| hooks, and the three commands that refuse them | `docs/hooks.md` |
| `--trace` and the eleven subsystems | `docs/trace.md` |
| the config file and where each value came from | `docs/configuration.md` |
| `--select-file`, `--exclude-file`, `--index-out`, `--out`, `--dir` | `docs/configuration.md` |
| `--log-file` and its rotation | `docs/configuration.md` |
| every exit code | `docs/exit-codes.md` |
| the short flag table and the naming conventions | `docs/flags.md`, unchanged |
| the CI entries, T-144 to T-161 and their kin | nowhere. They are process rather than behaviour and stay in the entry |

`docs/AGENTS.md` says at the top that RULES.md is normative and this is the
map, so a later session does not fork them. RULES.md section 2 gains a step 3a
requiring `docs/` and `docs/examples/` in the same push.

### `scripts/check-docs.ps1`, and what it found

Nine gates now, and a `Docs` job in CI beside `Record`, `Tree` and `Soak fit`.
It resolves every relative link and anchor in `README.md` and `docs/`, every
`scripts/` path, and every flag and command an example names against
`man/bit-cli.json`, and it enforces the mechanical half of the prose rule.

**32 problems on its first run, over documents written minutes earlier.**
Sixteen dead links from moved files, one dead anchor, thirteen history markers
and two vocabulary hits.

Two of its rules were narrowed after reading what they caught, and both are
argued in the script rather than quietly widened. `harness` is this
repository's own noun for a test rig, so only the verb is banned. And a bare
date is banned in a reference page and allowed in the three process documents,
because a date beside a rule there is the evidence for the rule.

### What the deep reviews found, which the gates had passed

**`check-todo.ps1` was undercounting every corpus file by eight to ten
percent.** It used `Measure-Object -Line`, which does not count blank lines,
and called two correct citations dead: `herp_test.go:80` in a file it reported
as 77 lines and has 86. That is trap five in the mining procedure this session
wrote, found in this repository's own gate an hour later.

**Nothing had ever checked the corpus index's own citations.** The pass
resolved a corpus path written in a `TODO/` entry and never one written in
`reference/RESEARCH.md`, where 327 of the 330 are. `check-todo.ps1` reads
`RESEARCH.md`, `reference/README.md` and `reference/HISTORY/` now, and all 327
resolve.

**Two rows of the BEP coverage table were false.** BEP 29 said "no. No flag
enables it" and `--transport` has enabled it since the last session; BEP 33, 44
and 51 were named in the prose under the table and had no row at all. Both were
inherited from the README and neither is something `check-docs.ps1` can see.

**164 links in the archived `INDEX-history.md` stopped resolving** when the
text moved two directories away. Caught by the same run.

## In progress

Nothing is half-written. Every entry filed this session is filed complete, with
a priority, an effort, a `Source:` line and an acceptance command.

- **[T-235](trackers.md)** is the only entry closed this session.
- **[T-234](peers.md)**, **[T-236](peers.md)**, **[T-237](trackers.md)**,
  **[T-238](peers.md)**, **[T-239](peers.md)**, **[T-240](dht.md)**,
  **[T-241](metainfo.md)** and **[T-242](performance.md)** are open and
  unstarted.
- **[T-243](phase-c.md)** is deferred and needs an operator ruling before it is
  workable at all.
- The entries the last session left open are untouched: [T-232](memory.md),
  [T-224](memory.md), [T-233](peers.md), [T-101](bep-coverage.md),
  [T-102](bep-coverage.md), [T-168](bep-coverage.md), and
  [T-164](peers.md), still the only partial.

## Start here next session

**The shape of the work order is the operator's, from seven sessions ago.** Not
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

3. **[T-236](peers.md), P1, effort S**, and it is the cheapest real thing on
   this list. `bit-cli` announces under two peer id prefixes and neither is its
   own; one of them is BitComet's. The check that found it prints the prefix
   beside its verdict, so the fix is verifiable in one run:

```bash
pwsh -NoProfile -File scripts/check-announce.ps1
```

   Pick the two character code against libtorrent's `src/identify_client.cpp`
   and `aquatic/crates/peer_id/src/lib.rs:100-120` before using it. The
   constant goes in one place and `SessionOptions::peer_id` is `pub`, so no
   patch to `vendor/` is needed.

4. **[T-233](peers.md), P1, effort M**, unchanged from the last session and
   still the largest thing open. The write side and the transport are both
   eliminated by measurement, so the two candidates left are on the read side
   and are named with their lines. Build the fixture first: a pair of real
   `librqbit_utp` streams in one process.

5. **Then the category pass, and `bep-coverage.md` is still first.**
   [T-101](bep-coverage.md) is open on a latency measurement loopback cannot
   produce, which [T-239](peers.md) is now the prerequisite for.
   [T-102](bep-coverage.md) and [T-168](bep-coverage.md) are the untouched two,
   then `dht.md`.

6. **The three entries open on a decision rather than on work**:
   [T-033](performance.md), the three aria2 flag names, with its curve already
   measured; [T-227](memory.md), the window cache total, which needs one
   throughput curve first; and [T-242](performance.md), the request depth,
   whose first half is a sweep rather than a change.

7. **[T-212](memory.md)**, and it is cheaper than it was:
   `demagnetize-rs/src/consts.rs:15` caps the declared metadata length at 20
   MiB and `dht-crawler/src/metadata.rs:274` counts the refusals, so two
   independent implementations bound what `librqbit` does not.

**Corpus sources the list above wants**, all on this machine and none needing a
fetch: `reference/RESEARCH.md` section D, rebuilt this session, has one row per
open entry; entries 23 to 29 for [T-234](peers.md); entries 30 to 37 for
[T-238](peers.md) and [T-239](peers.md); and `reference/README.md`'s
"The 2026-08-24 trees" section, which carries the actual code lines so a
session implementing any of them need not re-clone. **All of it is a read.**

## Open questions for the operator

**Three, all from [T-238](peers.md) and [T-243](phase-c.md), and each has a
recommendation so the answer can be one word.**

1. **NAT traversal.** Is the recommendation accepted: no `iroh`, and build the
   ladder instead, in the order direct, port mapping, BEP 55, then port
   prediction behind a flag that is off by default? **Recommended: yes.**
   A relay is a separate question and is **not** recommended yet, because it
   needs somebody to run it and a trust assumption to state.

2. **A user interface.** Is one wanted, and native or browser?
   **Recommended: yes, and native.** A native GUI does not collide with
   decision 7.4 at all, because there is no server: the UI is the process. A
   browser UI reverses 7.4 by construction and un-defers
   [T-200](phase-c.md), [T-201](phase-c.md) and [T-203](phase-c.md) with it.
   `egui` over `slint`, and the argument is in [T-243](phase-c.md).

3. **Client masking.** [T-234](peers.md) settles two design points on its own
   authority and both are worth a glance: the default is honest, so `bit-cli`
   advertises itself unless told otherwise; and whatever is advertised appears
   in the machine output, so a measurement is never silently taken under a
   mask. The flag is `--as-client`, with `--announce-as` and `--advertise-as`
   both rejected and the reason recorded.

**One thing to be aware of rather than to decide.** Twelve repositories in
`TheDancingDeveloper-org` redistribute Apache-2.0 code under MIT with no
licence file and no attribution. Nothing was said to anybody about it, by
[RULES.md](RULES.md) section 6a, which is absolute. It is recorded in
`RESEARCH.md` entry 40 so that a later session reading `license = "MIT"` in one
of those manifests does not act on it.

**A second thing.** One dependabot pull request is open on this repository,
`dependabot/github_actions/github-actions-b4f5548579`, and its own CI run is
green. This session did not take it, because it was not on the assignment and a
dependency bump is a change to the build rather than to the documentation.

## One behaviour change worth the operator's eye

Not a decision, and smaller than the last session's two.

**`loopback-tracker` keeps request headers and takes `--announce-log`.** It is
a test fixture rather than the product, so nothing a user runs changed. What
changed for a script driving it: it no longer drains the headers, and with
`--announce-log <PATH>` it appends one JSON object per announce. Existing
callers pass neither flag and see exactly what they saw before.
