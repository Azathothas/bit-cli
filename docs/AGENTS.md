# Working on bit-cli

Orientation for an agent picking this repository up. Read it once, then work
from [`TODO/PROGRESS.md`](../TODO/PROGRESS.md).

**This file is not normative and [`TODO/RULES.md`](../TODO/RULES.md) is.** When
the two disagree, RULES.md wins and this file is wrong. RULES.md is how the
repository is worked on, rule by rule, with what each rule cost to learn. This
is the map: where things are, which tool answers which question, and what a
session owes at the end. Anything that binds is linked rather than restated, so
the two cannot fork.

## What bit-cli is

A non-interactive BitTorrent and HTTP client. Its reason for existing is
per-scope web seeds: attach arbitrary HTTP sources to an existing `.torrent` at
runtime, without rewriting it. [`webseed.md`](webseed.md) is the addressing
model and it is the thing no other client has.

It is built on `librqbit`, which it **vendors** under `vendor/` rather than
depending on the published crates, so anything blocking this repository can be
fixed here rather than described as somebody else's problem. Seven other trees
are vendored beside it for the same reason; [`vendoring.md`](vendoring.md) says
which and why.

## The reading order

1. [`TODO/PROGRESS.md`](../TODO/PROGRESS.md). The measured baseline, what the
   run before yours did, and the work order under "Start here next session". It
   is rewritten each time and carries no history.
2. [`TODO/RULES.md`](../TODO/RULES.md). Section 4a is the tools that already
   exist, section 5 is the rules that bite most often, section 6 is the settled
   decisions, section 6a is remote operations and is absolute.
3. [`TODO/INDEX.md`](../TODO/INDEX.md). Every entry, one line each, sorted by
   id, with the counts and the argument behind the current ordering.
4. This file.
5. [`man/bit-cli.json`](../man/bit-cli.json), before typing a flag. Grep and
   skim; a full read is not needed.

## The tree

| directory | what is in it |
| --- | --- |
| `crates/bit-cli/` | the binary: argument parsing, the commands, the reports |
| `crates/bit-cli-core/` | the library: web seed fetching, the bridge, storage, metainfo, encryption |
| `crates/bit-cli-core/examples/` | loopback fixtures the acceptance scripts drive: a tracker, a file server, a churn generator, and a TLS and HTTP/2 fingerprint oracle |
| `vendor/` | eight vendored upstreams: the `librqbit` trees, and the five the impersonating HTTP client needs. **Changeable here**, and a change is recorded in `patches/` |
| `patches/` | the patch series derived from `vendor/`, plus `UPSTREAM.md`, which is the record Apache-2.0 asks for |
| `TODO/` | the authoritative record: one file per category, plus `INDEX.md`, `PROGRESS.md` and `RULES.md` |
| `docs/` | what the tool does, written for a reader using it |
| `man/` | the whole command surface, generated and committed, in three shapes |
| `scripts/` | the gates, the acceptance checks, the benchmarks, and the only sanctioned way to push |
| `bench/` | committed benchmark evidence. A run that **is** the evidence for an entry goes in deliberately |
| `reference/` | the research corpus. Gitignored on `main`, lives on the `references` branch |
| `.tmp/` | every run's output. Nothing here is ever committed |

## The tools, and reaching for the right one

The full list is [`TODO/RULES.md`](../TODO/RULES.md) section 4a. The ones a
session reaches for most:

| question | tool |
| --- | --- |
| how does this work, who calls it, what is the blast radius | `codegraph_explore`, or `codegraph explore "<question>"`. `.codegraph/` is indexed at the repo root |
| is the tree green | `pwsh -NoProfile -File scripts/gates.ps1` |
| does the record agree with itself | `pwsh -NoProfile -File scripts/check-todo.ps1` |
| an entry closed, so the counts have to move | `pwsh -NoProfile -File scripts/set-status.ps1 -Entry T-NNN -Status done`. It derives every count from the rows; never retype one |
| do the docs still resolve | `pwsh -NoProfile -File scripts/check-docs.ps1` |
| a file came out with the wrong line endings | nothing: `gates.ps1 -Fix` normalises them. `scripts/check-eol.ps1 -Fix` is the same step on its own |
| what is the flag for X | `man/bit-cli.json`. Never grep the source, never page `--help`, never guess |
| what has a run done, measured | `pwsh -NoProfile -File scripts/session-report.ps1 -Since <ISO>` |
| is the browser this tree impersonates still current | `pwsh -NoProfile -File scripts/check-browser-version.ps1`, and `scripts/check-browser-fingerprint.ps1` against a real one. Both emit the replacement values |
| I need a machine this one is not: another libc, a newer browser, a filesystem that is not here | a throwaway WSL2 distro. [`containers.md`](containers.md) is the procedure, and decommissioning is part of it |
| commit and push | `pwsh -NoProfile -File scripts/git-sync.ps1`. Nothing else |

**Reach for the purpose-built tool before the general one.** A general tool
used where a specific one exists produces answers that are plausible and wrong,
which is the hardest kind to catch.

**An exit code is read from the process that produced it, unpiped.** Piping a
check into anything reports the pipeline's status, not the check's, and a guard
that failed reads as green. This has caught sessions here more than once.

## The gate contract

`scripts/gates.ps1` runs every gate and prints one verdict. A default run
prints `text`, `eol`, `man`, `fmt`, `record`, `tree`, `docs`, `clippy`, `test`
and `deny`.

| switch | what it does |
| --- | --- |
| `-Fix` | formats, regenerates the manuals, and normalises line endings, rather than failing on any of them |
| `-Fast` | skips `deny` and the build |
| `-Build` | adds `--bins --examples` |
| `-Json` | for a machine |

**Green here is not green in CI when this machine's toolchain is behind.** CI
installs `stable` on every run and gains clippy lints with every release.
`gates.ps1` warns when the local toolchain is older; `rustup update stable` is
the fix.

Every `scripts/check-*.ps1` follows one contract, and a new one must:

- a header comment saying **what defect it exists to catch**
- exit **0** pass, **1** fail, **2** could not run
- a `-Json` switch
- no dependence on being run from a particular directory

A check that measures an open defect must not fail the build for that defect
alone. `scripts/check-close-wait.ps1` is the pattern: it records the count and
judges it only when a ceiling is passed. The other half of that rule is that
the exemption comes off when the entry closes.

## What a session owes

The full procedure is [`TODO/RULES.md`](../TODO/RULES.md) sections 1 and 2.
What is easy to miss:

**The record is part of the change, not a report about it.**
`TODO/PROGRESS.md`, `TODO/INDEX.md` and the entry are edited in the same push
as the work. `gates.ps1` has a `record` gate and CI has a `Record` job, both
running `check-todo.ps1`, so a count that disagrees with the rows cannot reach
a commit.

`scripts/set-status.ps1` is the writer for those counts and `check-todo.ps1` is
the reader. Closing an entry moves seven numbers across two files, and none of
them is worth doing by hand.

**`docs/` and `docs/examples/` are updated in the same push too**, when the
session changed what the tool does. `scripts/check-docs.ps1` is the gate, and
it compares four things a reader cannot check by looking: every link and
`scripts/` path resolves, every flag and command an example names is in
`man/bit-cli.json`, every output field a page names is in `docs/schema.md`, and
every `T-NNN` a page names is an entry. It also fails a page nothing links to,
because an unlinked page is not read and so is never corrected.

**An entry whose premise a measurement disproves gets the correction written
under it**, never a silent edit of the premise. The same applies to a belief
formed early in a session and measured against later.

**Nothing closes as "won't fix", "upstream problem" or "out of scope."** The
trees are vendored so that anything in `librqbit` can be fixed here. A blocked
entry stays open with the blocker named and what would unblock it.

**Claims need evidence.** A comparative claim without a committed benchmark
does not ship. A flag that does not move a number does not ship.

## The prose rule

Short sentences, no em dashes, no marketing adjectives, no emoji, present
tense, and every doc claim backed by a command a reader can run or a path a
reader can open.

`scripts/check-docs.ps1` enforces the mechanical half over `README.md` and
`docs/`: no emoji, no em dash, no control byte, a banned vocabulary list, and
no history markers. The rest is a reading.

**`docs/` says what the tool does. It does not say what the project did.** A
fixed defect stays in `docs/` only when the reader needs it to use the tool
correctly. "The allocator takes a write lock now" is history and belongs in the
entry. "Two lints exist because another client will refuse the file" is a
constraint and stays.

## Remote operations, and this one is absolute

**`Azathothas/bit-cli` is the only repository an agent may write to.**
Everything else is read only: `git clone`, `gh issue view`, `gh pr view`,
`gh api` for a fetch, `scripts/upstream-scan.ps1`.

Never open an issue, a pull request, a discussion, a comment or a review on
anybody else's repository, under any framing, and never fork or star one. The
patches under `patches/` are for this project and are never offered upstream.

[`TODO/RULES.md`](../TODO/RULES.md) section 6a is the rule and it is not a
judgement call. If a session believes an exception exists, it is wrong: leave
it, and say so in `PROGRESS.md` under open questions for the operator.

## Two procedures with their own pages

- [`reference-mining.md`](reference-mining.md), when the work is to clone,
  mine, survey or investigate somebody else's repository.
- [`task-authoring.md`](task-authoring.md), when the work is to turn a rough
  idea into a filed entry.
- [`containers.md`](containers.md), when a question needs a machine this one is
  not. **Everything it creates is removed in the same run**, and the machine is
  shared: `podman system df` before finishing is the one number that says
  whether something stopped cleaning up after itself.
