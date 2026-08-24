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

- **Last session:** 2026-08-24T14:20:49Z, unattended, and it worked the
  entries rather than filing them. The duration is not restated here:
  `scripts/session-report.ps1` derives it from the instant above, and a
  duration written down twice is a number two documents disagree about.

  It wrote the plan down before starting, per [RULES.md](RULES.md) section 1
  step 4, and the plan was the work order's item 3 with the order inside it
  worked out rather than taken: [T-249](metainfo.md), then
  [T-246](cli-surface.md), then [T-247](cli-surface.md). It held.
- **Tests:** 1,341 passing, 0 failing, up from 1,312. Plus **149** in the
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

- **Entries:** 204 items. 35 open, 2 partial, 0 blocked, 156 done, 11 deferred
  to Phase C. 156 of 193 workable done, 37 left.
- **Tree:** 99 Rust files, 59,585 lines of code, 15,459 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Corpus:** **thirty-nine trees** in forty-one `RESEARCH.md` entries. Plus
  `reference/HISTORY/`. [`reference-map.md`](reference-map.md) carries the
  licence per tree and where the determination came from. Nothing was mined
  this session and nothing was read from it: all three entries were about this
  tree's own surface.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **31 patches**
  across twenty-one sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  Untouched.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

**Three entries closed, all from the work order's item 3, in an order the work
order did not give.** [T-249](metainfo.md) went first because it builds
`bit-cli tree` and [T-246](cli-surface.md)'s acceptance names `tree` as the
command a typo has to be corrected to. Filing that order before starting was
the cheapest thing the session did: T-246's own example used
`bit-cli tree one.torrent` as the subcommand that does not exist, and it does
exist now.

**One entry was filed**, [T-255](cli-surface.md), for a defect found while
closing T-249 rather than looked for.

### [T-249](metainfo.md), P3: `bit-cli tree`, and a span that does not mean what the entry said

`crates/bit-cli/src/cmd/tree.rs`. The same `Layout` `files` reads, rendered as a
tree, with each directory rolled up to its size, its file count, and the pieces
it spans.

```
PATH                   SIZE      FILES  PIECES
padded/                2.49 KiB  3      0-2
|-- disc 1/            1.95 KiB  2      0-2+
|   |-- lossless/      1.46 KiB  1      0-1+
|   |   `-- a.flac     1.46 KiB         0-1+
|   `-- notes.nfo      500 B            2-2
`-- .pad/              548 B     1      1-1+
    `-- 548 (padding)  548 B            1-1+
```

**The approach's reason for wanting the piece range is not true of the piece
range.** It said the span is what tells you whether a subtree can be fetched
without touching the rest. It is not: a piece straddling a boundary belongs to
both sides. Every row above carries a `+` except `notes.nfo`, which is the one
file the padding pushes onto a piece boundary. So `shared_pieces` sits beside
the span and is the field that answers the question, and the `+` is what says
so in the text form.

**The acceptance's IBM437 clause needed a second condition**, not just
`--color`. Tying the glyphs to colour alone leaves an interactive console at
`IBM437` getting box drawing, which is the case the entry names.
`Env::out_is_unicode` is the second: on Windows `GetConsoleOutputCP() == 65001`,
elsewhere a UTF-8 locale, asked only when stdout is a terminal because a file
takes the bytes verbatim. **This machine's console code page is 437**, from
`[Console]::OutputEncoding.CodePage`, so the case is not hypothetical here.

### [T-246](cli-surface.md), P2: three inputs, and the first usage error a source has ever produced

A directory, a mistyped subcommand and a URL under another scheme all exit **2**
now and each says what to do. The entry's fourth fact was "no input to a
`SOURCE` argument produces a usage error"; three do.

| input | before | now |
| --- | --- | --- |
| a directory | exit 4, "Access is denied. (os error 5)", `io_kind: PermissionDenied` | exit 2, "is a directory, not a .torrent", and `bit-cli create` |
| `bit-cli tre album.torrent` | exit 4, cannot read a file called `tre` | exit 2, "is not a command", and `bit-cli tree` |
| `ftp://host/x.torrent` | exit 4, "volume label syntax is incorrect. (os error 123)" | exit 2, "is not a scheme this reads", and the forms that are |

The directory message is this tree's rather than the operating system's, which
is what makes it the same on both platforms: `read_torrent_file` tests
`path.is_dir()` before the read, so neither `ERROR_ACCESS_DENIED` nor `EISDIR`
is reached. Eight call sites read a caller-supplied `.torrent` path and all
eight go through it.

**Three of the typo check's four conditions exist to keep a real file out of
it**: no `/`, `\`, `.` or `:` in the word; nothing of that name on disk; and a
subcommand within one edit. `./tre` and `tre.torrent` are paths, a torrent
actually named `tre` is downloaded, and `quuxly` is a missing file rather than a
guess. The names come from `Cli::command()`, so a subcommand added later is
suggestible with nothing to remember.

### [T-247](cli-surface.md), P2: a dry run counts only what it took

`download --dry-run <URL>` printed `web seeds 0` and `trackers 0` for a torrent
it had not fetched. It says what it did not do now, and `0 so far` rather than
nothing, because a `--web-seed` on the command line and a Metalink's mirrors
are real counts a dry run does know.

```
source               http://127.0.0.1:8099/tracked.torrent
not fetched          a dry run does not fetch the torrent, so its own web seeds and trackers are not counted
web seeds            0 so far
trackers             0 so far
```

The same torrent read off disk prints `name`, `web seeds 1` and `trackers 1`
with no qualifier. The `--json` shape is untouched: it always said the torrent
had not been read, through `name`, `info_hash` and `total_bytes` being null.

### [T-255](cli-surface.md), filed: the schema generator deletes prose and nothing fails

`BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema` deleted **130
lines** from `docs/schema.md` this session: four hand-written sections, one of
which carries the only committed measurement of what seven PowerShell
redirection forms do to non-ASCII output. Put back by hand.

Both gates then passed on the truncated file, and that was measured rather than
reasoned about, by stripping the tail again and running each one unpiped:

| check | on the truncated file |
| --- | --- |
| `cargo test -p bit-cli --lib schema` | exit 0, 11 passed |
| `scripts/check-docs.ps1` | exit 0, "everything resolves" |

The schema test is a containment check over fields, so prose is invisible to
it, and `docs/examples/machine-output.md` stays reachable through `README.md`
so no link broke.

### What else moved

- The URL parity test lost the count in its name. It was
  `four_commands_resolve_a_torrent_over_http_and_report_what_the_file_reports`
  and `tree` made it five; it is `read_only_commands_resolve_...` now, and
  [`docs/examples/inputs.md`](../docs/examples/inputs.md) names the five rather
  than counting them. A count in a test name is one more number two documents
  can disagree about.
- `docs/exit-codes.md` gained "What exits 2, and what exits 4".
  `docs/metainfo.md` gained "The shape a torrent carries".
  `docs/examples/inputs.md` had three passages rewritten, including a section
  titled "Everything on this page exits 4" that three inputs had just made
  false.
- Every command in every one of those passages was run before it was written
  down, including `bit-cli tree` over all five source forms: a local
  `.torrent`, stdin, an HTTP URL, a local Metalink, and a magnet, which is
  refused with exit 4 as the other read-only commands refuse it.

## In progress

Nothing is half-written. All three entries closed complete, with the acceptance
run and its output recorded in the entry.

- **[T-253](cli-surface.md)** is still `partial`, untouched: the fifteen schema
  rows are in and the fixture that would generate them is not. It is the
  neighbour of [T-255](cli-surface.md) and the two want the same session.
- **[T-164](peers.md)** is still `partial`, untouched.
- The entries the last sessions left open are untouched: [T-232](memory.md),
  [T-224](memory.md), [T-233](peers.md), [T-237](trackers.md),
  [T-239](peers.md), [T-240](dht.md), [T-241](metainfo.md),
  [T-101](bep-coverage.md), [T-102](bep-coverage.md), [T-168](bep-coverage.md),
  [T-244](cli-surface.md), [T-248](metainfo.md), [T-250](cli-surface.md),
  [T-251](trackers.md), [T-252](cli-surface.md), [T-254](webseed.md).
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

3. **[T-255](cli-surface.md) and [T-253](cli-surface.md) together, both effort
   S and both about the same generator.** T-255 is the carry-across so
   regenerating stops deleting prose, and T-253's remaining half is two
   fixtures so the thirteen rows it added by hand are produced rather than
   inherited. One session, one file: `crates/bit-cli/src/schema_gen.rs`.

4. **[T-251](trackers.md), P2, effort M.** `trackers` is the one command left
   that refuses a URL its own help offers, and its refusal names an info hash
   the URL carries. The source half is a few lines now that
   `source::resolve_source` exists and `read_torrent_file` is the one door to a
   local file; the twelve-knobs half is the rest of the entry.

5. **[T-233](peers.md), P1, effort M**, unchanged and still the largest thing
   open. The write side and the transport are both eliminated by measurement,
   so the two candidates left are on the read side and are named with their
   lines. Build the fixture first: a pair of real `librqbit_utp` streams in one
   process.

6. **[T-244](cli-surface.md)**, unblocked since [T-245](cli-surface.md) closed.
   The ruling on it is static extraction with a browser-shaped header set and a
   `--render` opt-in. [T-250](cli-surface.md) is its neighbour and is cheaper
   than it was: `Kind::classify` now produces three distinct refusals, so
   "report how an input was resolved" has something to report.

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
which carries the actual code lines. **All of it is a read.** Nothing was mined
this session and nothing needs to be.

## Open questions for the operator

**None.** Nothing this session did needed a ruling, and nothing it found needs
one.

**Two things to be aware of rather than to decide**, both carried forward
unchanged. Twelve repositories in `TheDancingDeveloper-org` redistribute
Apache-2.0 code under MIT with no licence file and no attribution; nothing was
said to anybody, by [RULES.md](RULES.md) section 6a, and it is recorded in
`RESEARCH.md` entry 40. And one dependabot pull request is open on this
repository, `dependabot/github_actions/github-actions-b4f5548579`, with its own
CI run green. It was not taken, because a dependency bump is a change to the
build rather than to the work this session was given.

## One behaviour change worth the operator's eye

**Three inputs that exited 4 now exit 2.** A directory, a URL under a scheme
this tree does not speak, and a bare word one edit from a subcommand with no
file of that name. A script branching on 4 to mean "this source failed" sees 2
for those three, which is the point: 2 says no retry and no other mirror will
help, because the argument is not a source. [`docs/exit-codes.md`](../docs/exit-codes.md)
carries the rule and every other input still exits 4.

The peer id change from the session before this one is unchanged and still
worth knowing: every peer id `bit-cli` emits is `-CL0200-` now, so a tracker
counts this client as an unknown client called `CL` until somebody's table
gains a row.
