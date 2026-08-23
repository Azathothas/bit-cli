# Progress

**Read this first.** It is the only thing the kickoff prompt tells a session to
read, so everything that changes from session to session is here: the baseline,
what the last session did, and the work order. The prompt carries none of it, by
[RULES.md](RULES.md) section 3.

It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
Every entry, one line each: [INDEX.md](INDEX.md).

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

## Two things are settled and are not to be raised again

**Nothing in `patches/` is ever offered upstream, and this repository is the
only one an agent may write to.** [RULES.md](RULES.md) section 6 carries the
first and section 6a the second. A previous revision of this file listed
"Offer the patches upstream" as work; it was wrong, and the operator has now
given the instruction twice. `patches/UPSTREAM.md`'s `Upstream:` field answers
"could a release retire this patch on its own?" and nothing else.

**The six hour soak is run by the operator, in a foreground terminal.** No agent
session lasts six hours, and a session ending kills the process it started. The
command is under "Start here next session"; a session's job is to read the CSV
the operator's run leaves behind, not to start one.

## `patches/TASKS.md` is finished, and the fork is maintenance now

Twelve of its thirteen rows are done and the thirteenth is not waiting on a
seam any more. That file's own closing section says the signal to look for is
"no entry in the table at the top of this file is still waiting on a seam", and
that is now true: [T-102](bep-coverage.md), BEP 55, waits on a measurement its
own acceptance asks for and on a fixture that can produce an unreachable peer,
neither of which is `librqbit`'s to give.

So this session's work order is derived from [INDEX.md](INDEX.md)'s four
questions again, not from `patches/TASKS.md`, and the vendored trees become
what `patches/README.md` describes: run `scripts/upstream-scan.ps1` on a version
bump, reconcile with `scripts/vendor-sync.ps1`, keep `UPSTREAM.md` true.

## State

- **Last session:** 2026-08-23T08:57:40Z, unattended, and running. The one
  before it started at 2026-08-23T06:14:39Z and was ended on the operator's
  word. The duration is not restated here: `scripts/session-report.ps1` derives
  it from the instant above, and a duration written down twice is a number two
  documents disagree about.
- **Tests:** 1,204 passing, 0 failing. 1,166 at the start. Plus **149** in the
  vendored trees, which the workspace gates do not run.
- **Gates:** clean, on rustc 1.98.0.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green on all **seventeen** jobs at run **32629207782**, against commit
  `a044421`, which is the last commit of the session that changed source. It
  carries [T-216](windows.md)'s fix and both `check-todo.ps1` changes.

  Two commits sit on top of it, `246f341` and `fa78ae2`, both documentation only
  and both carrying `[skip ci]`, so neither started a run.

  **Two jobs went red this session and both are fixed**, [T-215](webseed.md) on
  `Test (windows-latest)` at run 32626337016 and [T-216](windows.md) on
  `Test (ubuntu-latest)` at run 32627489685. Neither was a defect in `bit-cli`:
  both were tests asserting something about the runner. That is now the fourth
  and fifth of that kind.
- **Entries:** 167 items. 29 open, 1 partial, 0 blocked, 127 done, 10 deferred
  to Phase C. 127 of 157 workable done, 30 left.
- **Tree:** 94 Rust files, 54,369 lines of code, 13,312 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **27 patches**
  across nineteen sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Two standing instructions from the operator, the whole `cli` group the last
work order named, and five entries that came out of doing them.** Nine entries
closed and five were filed.

### The two standing instructions, taken first

The operator's amendments to the previous work order, applied before any entry:

1. **The soak is handed to the operator.** Printed as a command for a dedicated
   foreground terminal, and read back from its CSV at the start of every later
   session until the six hour window is complete.
2. **No unauthorised remote operations, ever.** `Azathothas/bit-cli` is the only
   repository an agent may write to. Everything else is read only. The work
   order item that said "Offer the patches upstream" is deleted rather than
   deferred.
3. **The rule is written where a session will hit it**, so it is not proposed a
   third time: [RULES.md](RULES.md) section 6 and the new section 6a,
   `patches/README.md`, the head of `patches/UPSTREAM.md` and all seventeen of
   its `Upstream:` fields, `patches/TASKS.md`, and the three kickoff prompt
   samples on the `references` branch.

**A claim in the previous revision of this file was wrong**, found while reading
the soak it describes. It said the cut-short run left "no JSON summary, because
the script writes that at the end". `bench/soak-20260823T040627780Z.json` exists,
carries all five slopes and says `"complete": false`. That is exactly what
[T-157](memory.md) built: `soak.ps1` rewrites its summary after every sample so
a killed run still leaves them. The numbers the previous session quoted were
right; the sentence about where they came from was not.

### The `cli` group, taken in the work order's item 3

**[T-159](cli-surface.md), P3, done.** Four `bench` subcommands filed their own
flags under **Report options**. `next_help_heading` is a running setting on the
`clap` command rather than a property of the struct that set it, so an argument
declared after a `#[command(flatten)]` inherits whatever that flatten left
behind, and `BenchShared` ends by flattening `ReportArgs`.

**The entry undercounted it.** The fifth place it happens is the front door:
`bit-cli --help` had **no "Arguments" section at all**, because `Cli::sources`
is declared after the `Global` flatten. `[SOURCE]...` was documented 100 lines
below the usage line that names it. That was found by the test rather than by
reading, on its first run.

**[T-156](cli-surface.md), P3, done.** `download --dry-run --json` writes
`kind: "download_dry_run"`. `docs/schema.md` carries its field table now, from
two runs the generator drives.

**The order of those two runs is load-bearing and the first attempt got it
wrong.** `Sample::merge` is `or_insert`, so the **first** observation of a path
names its type. With the Metalink run first, the table said `info_hash`, `name`
and `total_bytes` were `null`, which is what a Metalink dry run leaves them as
and is not what the field is.

**[T-118](cli-surface.md), P3, done**, and its one unmet clause is met as a
**merge** rather than a render. `BIT_CLI_UPDATE_FLAGS=1` keeps an existing row
verbatim, adds an empty one for a new flag, and drops one for a flag that is
gone. Three of the five columns are things `clap` cannot know, so rendering the
table would delete every hand-written cell, which is [T-158](cli-surface.md)
arriving in a second file.

**A second direction of drift had been open the whole time**: the old test
walked the flags and asked the table about each, so a row for a flag that no
longer exists passed. And `-h` was never checked in either direction, because
`clap` creates `--help` while **building** a command and `Cli::command()` hands
back one that is not built.

**[T-155](cli-surface.md), P3, done.** `one_inner` returned for
`--hash-check-only` above the block that built the `metalink` report, so a
Metalink run with that flag said nothing about the document at all. It is
`apply_metalink` now, called at both exits. Over a complete payload the answer
is the strongest one available: the digest computed and matched, 2,097,152
bytes, `bench/metalink-20260823T071256391Z.json` case `hash_check_only`.

**The test was checked against the defect rather than assumed to cover it.**
With the call removed from that exit it fails on `no metalink block`. A test
written for a fixed defect and never run against the defect is a test that may
be asserting something else.

**[T-154](cli-surface.md), P2, done.** A Metalink named by URL is
`Kind::MetalinkUrl` now, not a torrent URL that fails on a bencode parse. The
acceptance ran against the live mirror: `real_by_url` beside `real_as_served` in
`bench/metalink-real-20260823T071745617Z.json`, same exit code and the same
message character for character, from a document `MirrorBrain` generated per
request.

**Two things the entry did not say.** The extension is read from the URL's
**path**, so `?file=r.meta4` is a query naming a file and not a statement about
what the URL serves. And the redirect decision the Approach said was owed turned
out not to be: nothing on either path resolves a mirror URL relative to
anything, so a fetched document is treated exactly as a saved one.

**[T-116](cli-surface.md), P3, done, and the Approach over-priced it.** It
predicted a storage wrapper mapping an index to a path, built alongside T-071.
It is one argument to the function T-071 already built: `paths::plan_with`
applies each override **before** anything else, so a requested path is
sanitised, truncated and disambiguated exactly as a torrent path is. `-O
0=../../etc/passwd` renames the file inside the output directory. Nothing about
`-O` could reach outside it without first defeating T-071.

**Half of it would have shipped without the second command.** `verify` looks
where the bytes went rather than where the torrent said, and it builds that
from the same plan, which knew nothing about `-O`. So the tree could rename a
file its own verifier then called missing. `verify` takes `-O` too now.

**[T-213](cli-surface.md) filed** for the residual, measured and named with a
line: `seed` builds its `AddOptions` at `cmd/seed.rs:260` with no `index_out`,
so a payload downloaded with `-O` cannot be seeded from where it landed.

**[T-115](cli-surface.md), P2, partial to done.** `--on-complete` and
`--on-error` fire **once per torrent** now, and a mixed run fires both, which
the old shape could not express at all: it picked one for the whole run by
`report.failed` and used the first torrent's identity with the run's totals.
`--on-piece-verified` fires for the first time.

**The entry's "probably a rate limit" is answered with a measurement rather
than a flag.** One piece is one process, and **1,025 invocations took 47.55
seconds on this machine**, 46 ms each. The measured command was `cmd /C rem` and
a hook already runs through `cmd /C`, so that is two processes per invocation
and about 23 ms per `cmd`; `docs/hooks.md` says so rather than rounding it into
a bigger number. Two bounds instead of a
rate limit, because a rate limit loses notifications and a caller cannot tell
which: the hook runs on its own thread, and its queue is bounded at 1,024 with
what does not fit **counted** into `hooks.skipped` and warned about.

**A defect in the hook runner that had been there since hooks existed**, found
by the acceptance rather than by reading. `swarm::run_hook` built `cmd /C
<command>` with `Command::arg`, and Rust quotes for the C runtime's parser
while `cmd.exe` uses rules of its own. Any hook with a quoted path, a redirect
or an `&&` reached `cmd` mangled. The acceptance's own hook fired twice, as the
entry asked, and **failed twice**. `raw_arg` is the fix.

`docs/hooks.md` is new and is the Acceptance's second clause.
`ACCEPTED_WITHOUT_A_READER` in `cli.rs` is **empty** now: it held
`on_piece_verified` and `index_out` and both closed today.

**[T-214](cli-surface.md) filed** for the Problem's third clause, which the
Acceptance never covered: `seed` has no `--on-*` flag at all. The entry says
what has to be decided first, because a seeder does not mean the same thing by
"complete" that a download does.

**[T-136](multi-source.md), P2, done, and half its Acceptance was already
met.** Measuring before building found that the first clause is what
[T-179](webseed.md) built and holds: a mirror serving one corrupt piece beside
an honest one, every conviction naming source 0 with a piece index and two
hashes that differ, the honest mirror surviving, and the payload arriving
complete. The piece, the source and the mismatch. Nothing was written for it.

What was owed is `--verify-on-complete` and the contract.
`torrents[].verified_files` carries a sha256 per file, read back off the disk
after the run. It is redundant with the piece checks by construction, which is
the point: it is the check that does not trust the thing that wrote the bytes,
and the only one whose output can be compared against a digest published
somewhere else.

`docs/integrity.md` is the contract the Relevance asks for, and its last two
sections are the ones that matter: **what none of the checks tells you**, which
is whether the `.torrent` describes the file you wanted, and a table naming the
test behind every claim in it.

**One duplicate removed.** `metalink.rs` had a private streaming digest and this
needed the same thing; it is `bit_cli_core::digest` now and both use it. Two
answers to "what does this file hash to" is the one place two answers is the
whole problem. Checked against the **published** vectors rather than against
this code's own previous output.

**A test written earlier in this session turned out to assert something else.**
T-155's metalink hash-check test started a DHT it did not need, and once the
module had enough parallel tests it failed with "error initializing persistent
DHT". Same class as T-215 and found the same way, by running the whole suite
rather than the one test.

### A red job, and the fourth of its kind

**[T-215](webseed.md), P1, filed and done.** CI run **32626337016**, the T-116
push, turned `Test (windows-latest)` red on
`bench_webseed_measures_only_what_a_scope_covers`: `errors.total` was 1 where
the test asserted 0. That commit changed the path planner, the storage factory
and two commands, none of which `bench::webseed::run` touches.

It is a test about **scope**, and it was asserting that a 600 ms bench against a
loopback server on a loaded runner cannot lose a connection. That is the fourth
instance of the rule [RULES.md](RULES.md) section 5 already carries, after
[T-148](bench.md), [T-160](cli-surface.md) and [T-162](webseed.md).

**The lesson is narrower than the rule, and it is about T-162.** Counted rather
than remembered, because the first draft of this paragraph got it wrong:
`webseed_e2e.rs` held exactly **two** assertions that no error can occur. T-162
reshaped two **other** tests sitting between them and left both standing. The
one that went red is 58 lines below T-162's last edit. **When a defect is found
in a file, the fix is the file rather than the line.** So the other,
`bench_webseed_moves_real_bytes_and_reports_them`, went with it, before it
turned anything red.

### A defect in the tooling, found on the way

`scripts/check-man.ps1 -Fix` generated the manuals by running
`target/release/bit-cli.exe` and did not build one first. A stale binary
regenerated all three files from the surface as it was at the last release
build, wrote them, and printed "regenerated"; `git diff man/` was then empty
while `cargo test --test man_is_current` went on failing, because that test
renders from the crate being compiled. **`gates.ps1` printed `man ok` and
`test FAILED` in the same run**, which reads as the test being wrong.

`-Fix` builds first now. Without `-Fix` it does not, because that would put a
release build in front of every `gates.ps1` run; it compares the binary's
timestamp against the newest `.rs` under `crates/` and defers to the test.


### The operator's correction, and the two entries it produced

**[T-161](cli-surface.md) was open and had been done for a session**, which the
operator spotted and no script did. The action it is about,
`ilammy/setup-nasm@v1.5.2`, was replaced by `scripts/setup-nasm.ps1` at all five
call sites when [T-199](cli-surface.md) closed, and the entry went on describing
a workflow the tree does not have.

**Two gaps in `scripts/check-todo.ps1`, both closed.** `.github/` was not in the
cited-path prefixes at all, so this entry's four citations of
`.github/workflows/ci.yml:<line>` were never resolved for anything. And nothing
compared an entry's premise to the workflows: a new `stale-premise` check reads
the `uses:` lines and fails when an **open or partial** entry names an
`owner/name@ref` pin that no workflow carries. That is the one shape of "this
entry describes a state the tree is not in" that can be decided mechanically,
because nothing else in this record is spelled that way.

**The first draft of that check passed T-161**, and the reason is the useful
part: it searched the raw text of the workflows, and `ci.yml` carries the comment
"Ours, not ilammy/setup-nasm: that action is unmaintained". A substring search
found the very action the comment exists to say is gone. It reads `uses:` lines
only now.

**[T-217](windows.md), filed and done, and it is the reason the check took two
tries.** Writing that check put a `0x08` backspace into
`scripts/check-todo.ps1`, from a Python `\b` escape interpreted on its way to the
file. It landed inside the regex `'^###\s+(T-\d+)\b'`, so the pattern required a
byte nothing has, matched no entry, and passed every file **silently**. The
`text` gate said `text ok` on the same run, because it searched for a NUL and
nothing else.

It searches for every C0 byte except tab, newline and return now, and widening it
found three more: another backspace in `gates.ps1`'s own comment about the first,
one in `TODO/windows.md` where `foo\bar` lost its backslash, and **two `0x13`
bytes in `crates/bit-cli-core/src/mse/handshake.rs`**, where the BitTorrent
handshake's length byte was written as itself in a `b"..."` literal rather than
as `\x13`. That last is the same defect [RULES.md](RULES.md) section 5 already
records for `torrent/bencode.rs`, in a file written after that rule was.

**[T-216](windows.md), filed and done**, the second red job. A seeder test waited
up to 20 seconds for a listener that `--stop-after 15s` had already taken away.
Two numbers that have to be ordered and were not. They are ordered by a factor of
two now, and the peer thread returns a `Result<(), String>` so a failure names
which of its three steps failed rather than saying "the peer never completed a
handshake", which was true of all three.

## In progress

Nothing is half-written.

**What this session is doing**, written before doing it, by
[RULES.md](RULES.md) section 1 step 4. The work order below is the previous
session's and this session takes it in its order:

- **Item 1 is done.** The baseline above was re-measured rather than trusted:
  gates clean at 1,204 tests on rustc 1.98.0, `check-todo.ps1` agrees with the
  rows, `vendor-status.ps1` exits 0, and run **32629207782** is green on all
  seventeen jobs with the two commits above it carrying a skip marker.
- **Item 2 is the operator's** and is printed below rather than started.
- **Item 3, the `ci` and `windows` group**: [T-150](cli-surface.md) in
  `.github/workflows/ci.yml`, [T-075](windows.md) in `README.md`, and
  [T-178](windows.md), whose premise is stale because the trees are vendored
  now, in `vendor/rqbit/crates/librqbit/src/storage/filesystem/opened_file.rs`
  and `crates/bit-cli-core/src/storage/`.
- **Item 4, the `trackers` and `dht` group**: [T-180](trackers.md),
  [T-063](trackers.md), [T-065](trackers.md), [T-050](dht.md),
  [T-051](dht.md).
- Then items 5 and 6 as far as they go.

Open before this session and unchanged by it so far:

- **[T-212](memory.md)**, **[T-213](cli-surface.md)**, **[T-214](cli-surface.md)**
  came out of the previous session's own work and each names what it needs.
- **[T-102](bep-coverage.md)** is open and **[T-164](peers.md)** is partial, the
  only partial left.

## Start here next session

**The shape of the work order is the operator's, from two sessions ago.** Not
priority first. Clear as many small entries as possible, so the open count comes
down, and then take the bigger ones a **category at a time**: all of `bep`, or
all of `dht`, in one session rather than one entry from each.

**The `cli` group that item 3 named is finished.** All eight closed this session:
T-115, T-116, T-118, T-136, T-154, T-155, T-156 and T-159. Two more entries came
out of doing them, [T-213](cli-surface.md) and [T-214](cli-surface.md), and three
more out of the CI and tooling work beside them, [T-215](webseed.md),
[T-216](windows.md) and [T-217](windows.md). **There is no open P0 and exactly
one open P1**, `T-081`, BEP 52 v2 and hybrid torrents, effort XL. The only open
entry waiting on it is [T-134](bep-coverage.md). Everything below is unblocked.

Derived from the rows rather than from memory:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Re-measure the baseline rather than trusting the one above**, which is what
   [RULES.md](RULES.md) section 1 step 5 asks for. The CI run named there is
   green on all seventeen jobs and the two commits above it are documentation
   with `[skip ci]`, so there is no red job waiting and nothing to chase:

```bash
gh run list --limit 1
```

2. **The six hour soak, and it is the operator's to run.** No agent session lasts
   six hours and a session ending kills the process it started, so a session
   prints this and moves on. In a dedicated foreground terminal, from the
   repository root:

```bash
pwsh -NoProfile -File scripts/soak.ps1 -Minutes 360 -RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1
```

   **How a later session reads it.** The run writes `bench/soak-<stamp>.csv` one
   row per sample and rewrites `bench/soak-<stamp>.json` after every sample, so a
   run still going has both. `"complete": true` in the JSON is the only thing
   that says the window finished; `elapsed_hours` and `samples` say how far it
   got. Six hours at the default 30 second interval is about 720 samples.

```bash
pwsh -NoProfile -Command "Get-ChildItem bench/soak-*.json | Sort-Object LastWriteTime | Select-Object -Last 1 | Get-Content | ConvertFrom-Json | Select-Object generated_at, complete, elapsed_hours, samples"
```

   The last run reached 1.32 hours of six with 145 samples, zero `CLOSE_WAIT` at
   every one of them and 288 leech cycles with none failed. Those two are counts
   and hold at that window. The RSS slope is not: +0.622 MiB/h at r squared 0.105
   is noise fitted to a line, and [T-040](memory.md) recorded the same shape at
   5.06 hours. **A slope needs a window long enough to have one.** The tree has
   moved a long way since that run, so it is worth restarting rather than
   resuming.
3. **The `ci` and `windows` groups, three entries at effort S**:
   [T-150](cli-surface.md), [T-075](windows.md) and [T-178](windows.md). The
   fourth, T-161, closed this session. T-150 is a workflow edit, so it needs a
   real run to prove it and cannot go in a `-NoCi` push.
4. **The `trackers` and `dht` groups, five entries at effort S**:
   [T-180](trackers.md), [T-063](trackers.md), [T-065](trackers.md),
   [T-050](dht.md), [T-051](dht.md).
5. **The three this session filed**, all effort S and all with the machinery
   already built: [T-213](cli-surface.md) is `-O` on `seed`, which is the flag
   and the test because [T-116](cli-surface.md) built the rest;
   [T-214](cli-surface.md) is hooks on `seed`, which needs a decision about what
   a seeder means by each trigger **before** any code; and [T-212](memory.md)
   whenever a fixture of stalling peers is being built anyway.
6. **The rest of the effort S entries**, in `bench`, `create`, `metainfo`,
   `memory`, `peers`, `performance`, `webseed` and `bep`: [T-094](bench.md),
   [T-191](bench.md), [T-176](create-seed.md), [T-173](metainfo.md),
   [T-187](metainfo.md), [T-041](memory.md), [T-165](peers.md),
   [T-033](performance.md), [T-008](webseed.md), [T-103](bep-coverage.md).
7. **Then, a category at a time.** `bep-coverage.md` has the most left and the
   most shared machinery. After it, `dht.md`.

**Two corpus sources the list above may want**, both already on this machine and
neither needing a fetch: `reference/RESEARCH.md` sections C and D, and
`contrib/rqbit/` in <https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, whose
`0012` and `0014` are read and not taken. **Both are reads.** Nothing is opened,
filed or commented on either, by [RULES.md](RULES.md) section 6a.

## Open questions for the operator

None outstanding.

The soak is item 2 and is the operator's to run, which answers the question the
session before this one left. The two standing instructions this session was
given are written into [RULES.md](RULES.md) sections 6 and 6a, into
`patches/README.md`, `patches/UPSTREAM.md`, `patches/TASKS.md` and the three
kickoff prompt samples, so a session that has read what it is told to read cannot
propose either again.
