# Rules

How this repository is worked on. `TODO/` is the authoritative record and this
file is the part of it that does not change from session to session.

Read [PROGRESS.md](PROGRESS.md) first for what the last session did.
Read [INDEX.md](INDEX.md) for the work order.

---

## 1. Starting a session

1. Read [PROGRESS.md](PROGRESS.md). It says what the last session did, what is
   in progress, and where to resume. It carries no history.
2. Read [INDEX.md](INDEX.md), the "Start here" section. That is the work order.
3. Rewrite `PROGRESS.md` to say what **this** session is going to do, before
   doing it. Name the entry ids and the files.
4. Re-measure the baseline rather than trusting a recorded one:

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

```bash
cargo fmt --all --check
```

```bash
cargo deny check
```

```bash
gh run list --limit 1
```

## 2. Ending a session

When the operator says the session is ending, in this order:

1. Finish or checkpoint the current task. A half-finished change is recorded in
   `PROGRESS.md` as partial, with the entry id and the file, never left silent.
2. Update `PROGRESS.md`: what happened, what is in progress with a named
   reference to the exact `TODO/<file>` and entry id, what is next.
3. Update the affected `TODO/` entries and [INDEX.md](INDEX.md), including the
   counts table, which must be exact against the rows.
4. **Two deep reviews.** Review 1: every claim written this session against the
   code or the path it cites. Review 2: cold, as if someone else wrote it.
   a doc contradicting another doc, an entry id that does not exist, a cited
   path that does not resolve, counts that no longer add up.
5. Commit and push with `scripts/git-sync.ps1`. Nothing else.
6. Read the CI run the push started. A push that leaves CI red without an entry
   naming why is not finished.
7. Print the kickoff prompt in chat only, in a code block. Section 3 says what
   it must contain.

## 3. The kickoff prompt

Printed in chat, in a code block, never written to a file. It starts the next
session with no prior context, so it must be self-contained. It contains:

- The one-line statement of what `bit-cli` is and why it exists.
- What this session did, in three or four lines.
- Where to resume: the exact `TODO/<file>`, the entry ids, and the corpus
  sources for each, by repository and path.
- The measured test count and the CI state as of the last push, named by run
  id rather than as "the latest".
- A pointer to this file and to `PROGRESS.md` rather than a restatement of
  them. Do not paste these rules into the kickoff; they are tracked and the
  next agent can read them.
- Whether the session is attended or unattended, and what to do when blocked.

## 4. Git

**`scripts/git-sync.ps1` is the only sanctioned way to commit and push.** It
enforces the rules below mechanically, so they stop being things to remember.

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -Message "Subject line" -Body "..."
```

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -PushOnly
```

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -Check
```

Every switch: `-Message`, `-Body`, `-Path` to stage specific paths, `-Evidence`
to force-add one benchmark past `.gitignore`, `-NoPush` to commit only,
`-PushOnly` to push what is already committed, `-Check` to report without
changing anything, `-FetchReferences` to restore the corpus on a fresh clone,
`-SkipGates` for a documentation-only change where the tree is known green, and
`-NoReferences` to skip the corpus mirror on one push. `-SkipGates` prints that
it was used, so a transcript shows the push carried no proof.

What it enforces, and why each rule exists:

- **Identity is `Azathothas <AjamX101@gmail.com>`, author and committer.** Set
  per-invocation with `-c`, so a machine with different global config still
  produces the right commits.
- **No AI attribution anywhere.** No `Co-Authored-By` naming a model or a tool,
  no "generated with" line, no tool name in the commit body. The script
  refuses a commit whose message matches any of those.
- **Nothing under `reference/` is ever committed to `main`.** It is gitignored,
  and the script refuses if a staged path is under it even with `-f`.
- **`bench/*.json` and `*.csv` are gitignored.** A run that **is** the evidence
  for an entry goes in deliberately with `git add -f`; the other ninety do not.
- **The corpus is pushed to the `references` branch**, not to `main`. The
  script syncs it on every push, so `reference/` survives a lost machine
  without ever entering `main`'s history.
- **The gates run before the push**, not after. `cargo fmt --all --check`,
  `cargo clippy -- -D warnings`, and `cargo test --workspace`.

## 5. The rules that bite most often

### Process

- **No deferral.** Nothing closes as "won't fix", "upstream problem", or "out
  of scope". Upstream has no interest in this work, so there is nowhere to
  defer to. A blocked item stays open in `TODO/` with the blocker named and
  what would unblock it. [T-016](disk-io.md), [T-020](peers.md) and
  [T-102](bep-coverage.md) are the worked examples.
- **Claims need evidence.** A comparative claim without a committed benchmark
  does not ship. A flag that does not move a number does not ship.
- **An entry whose premise the measurement disproves gets the correction
  written under it**, never a silent edit of the premise. Worked examples:
  [T-017](disk-io.md), [T-021](peers.md), [T-032](performance.md),
  [T-033](performance.md), [T-037](performance.md), [T-073](windows.md),
  [T-118](cli-surface.md), [T-131](multi-source.md), [T-141](webseed.md),
  [T-145](cli-surface.md), [T-160](cli-surface.md), [T-162](webseed.md).
- **A corpus citation is evidence of what someone else did, not evidence that
  `bit-cli` does it.** Never let one become the other in a doc.
  [T-074](windows.md) is the worked example: the corpus named a librqbit
  defect, and the pinned version already had the fix.
- **Read the code, then the doc, then fix the doc.** Not the other way round.
  Three entries have described a state this tree was not in, and every one took
  one command to check.

### Testing

- **A test waits on the condition, never on a guessed duration**, and never
  asserts that the machine cannot fail some other way.
  [T-148](bench.md), [T-160](cli-surface.md) and [T-162](webseed.md) are the
  three worked examples, and all three cost a red job.
- **An acceptance script that measures an open defect must not fail the build
  for that defect alone.** `scripts/check-close-wait.ps1` is the pattern, and
  `scripts/check-swarm.ps1`'s `listener_poisoned` case follows it with
  `judged: false`.
- **Filter test output for `^test \S+ \.\.\. FAILED`**, not for the summary
  line, or a flake's name is lost.
- Real public mirrors are allowed: `fosstorrents.com`,
  `dl-cdn.alpinelinux.org`, `geo.mirror.pkgbuild.com`. Downloads go to `.tmp/`.
  Pass `--no-torrent-web-seed` when measuring one named mirror.

### Output and prose

- **Prose style:** short sentences, no em dashes, no marketing adjectives, no
  emoji, present tense. Every doc claim backed by a command a reader can run or
  a path a reader can open.
- **Binary units** (KiB, MiB), **ISO 8601 UTC with millisecond precision**, and
  raw integers in JSON with any formatted string alongside rather than instead.
- **Anything consuming `--jsonl` selects by `type` or `kind`, never by
  position.**
- **Headless parity is absolute:** nothing TTY-gated, nothing display-only, no
  prompting. stdout carries data only.

### PowerShell

- **Variable names are case-insensitive**, `$args` inside a function is an
  automatic variable that silently swallows a parameter of that name, and
  `-match` is case-insensitive so `'FAILED'` matches `"0 failed"`. Name locals
  so they cannot collide, and use `-cmatch` when case is the signal.
- **`$PSNativeCommandUseErrorActionPreference` defaults to false from pwsh
  7.4**, so a native command writing to stderr does **not** terminate under
  `$ErrorActionPreference = 'Stop'`. The `Start-Process` pattern stays.

### Platform

- **`cfg(unix)` is a family, not a platform.** `/proc` is Linux;
  `posix_fallocate` is not on Darwin. [T-145](cli-surface.md) is what that
  costs.
- **`cargo build --workspace --examples` builds examples and no binaries.** Use
  `--bins --examples` when a script needs `target/release/bit-cli.exe`.
- **Kill stray release processes before rebuilding:**

```bash
pwsh -NoProfile -Command "Get-Process bit-cli,loopback-fileserver,loopback-tracker,loopback-churn -ErrorAction SilentlyContinue | Stop-Process -Force"
```

- Windows firewall is handled. `aria2c` 1.37.0 and `rqbit` 9.0.1 are installed
  and wired into `scripts/interop-roundtrip.ps1` via `-Client`. The `librqbit`
  **crate** is pinned at 9.0.0; the interop binary and the dependency are two
  different things. `transmission` cannot be added on Windows, see
  [T-084](create-seed.md).

## 6. Settled decisions, not to be relitigated

- **`librqbit` stays the base** (decision 7.3).
- **No daemon and no RPC** (decision 7.4). No SQLite and no state file.
  `bit-cli` must keep working with no config and no state. The corpus has a
  daemon reference stack in `TorrentNG` and an offline/online control-plane
  design in `superseedr`; both are for the deferred Phase C entries
  [T-200 to T-209](phase-c.md) only. Do not un-defer them.
- **`iroh` is not being adopted.** BEP 55 needs no NAT library; the blocker is
  librqbit's `PeerConnectionHandler`. [T-102](bep-coverage.md) carries the
  whole flow inline and the design rationale from `torrent/NOTES.md:15-31`.
  Do not reach for a NAT crate.
- **MSRV is 1.88 and is measured, not chosen.** Raising it needs
  `cargo metadata` to say so.

## 7. The corpus

`reference/` holds twenty-two upstream BitTorrent implementations indexed by
`reference/RESEARCH.md`, all permissive: twenty-one MIT and `intermodal`
CC0-1.0. See [reference-map.md](reference-map.md).

It is gitignored on `main` and lives on the `references` branch, which
`scripts/git-sync.ps1` pushes. To get it on a fresh clone:

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -FetchReferences
```

Trust `RESEARCH.md`. Every repository was read twice and verified twice more:
119 `path:line` citations checked in range and against the symbol they name,
every path resolved against the cleaned tree, all 22 licences read from the
licence file on disk, and 130 issue and PR references fetched with `gh`.

- Do **not** re-derive a licence. Section F is the answer.
- Do **not** re-fetch an issue or PR that `RESEARCH.md` already summarises.
- Do **not** re-clone, re-crawl, or diff the corpus against upstream.
- Do **not** copy corpus files into this repository.
- **Do** verify a path exists before citing it in a `bit-cli` doc. One
  `Test-Path` is the whole check. If a path is wrong, fix the citation and note
  it; do not go re-derive the finding.
- **Do** open and read the cited code when an entry needs the algorithm. That
  is what the citations are for.
