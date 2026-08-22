# Rules

How this repository is worked on. `TODO/` is the authoritative record and this
file is the part of it that does not change from session to session.

Read [PROGRESS.md](PROGRESS.md) first: what the last session did, the measured
baseline, and the work order under "Start here next session".
Read [INDEX.md](INDEX.md) for every entry, one line each, and for how an
ordering is derived.

---

## 1. Starting a session

1. Read [PROGRESS.md](PROGRESS.md). It says what the last session did, what is
   in progress, and where to resume. It carries no history.
2. The work order is `PROGRESS.md`'s "Start here next session", not
   [INDEX.md](INDEX.md)'s. INDEX carries the entry list and the argument that
   produced the last ordering; read that when re-deriving one, and say in
   `PROGRESS.md` that you did.
3. Record the start instant on `PROGRESS.md`'s state line, in ISO 8601 UTC.
   Everything at the end that measures the session reads it from there.

```bash
date -u +"%Y-%m-%dT%H:%M:%SZ"
```

4. Rewrite `PROGRESS.md` to say what **this** session is going to do, before
   doing it. Name the entry ids and the files.
5. Re-measure the baseline rather than trusting a recorded one. One command,
   not four: it kills stray release processes first, filters test failures by
   test name rather than by the summary line, and prints one verdict.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

```bash
gh run list --limit 1
```

## 2. Ending a session

When the operator says the session is ending, in this order:

1. Finish or checkpoint the current task. A half-finished change is recorded in
   `PROGRESS.md` as partial, with the entry id and the file, never left silent.
2. Update `PROGRESS.md`. It is the only thing the next session is told to read,
   so it carries everything the kickoff prompt does not, and section 3 says the
   kickoff carries almost nothing. It must hold:

   - **The state line**: when this session ran, in ISO 8601 UTC, and whether it
     was attended.
   - **The measured baseline**: tests passing, the gates, and the CI run id and
     the commit it describes. Named by id, never as "the latest".
   - **The entry counts**, which `scripts/session-report.ps1` prints.
   - **What this session did**, in a few lines per entry, and every premise a
     measurement disproved.
   - **What is in progress**, with the `TODO/<file>` and the entry id.
   - **Start here next session**: the ordered list, with the entry ids, the
     files, and the corpus sources for each by repository and path. This is the
     work order. It used to live in the kickoff prompt and it belongs here.
   - **Open questions for the operator**, or that there are none.
3. Update the affected `TODO/` entries and [INDEX.md](INDEX.md), including the
   counts table, which must be exact against the rows.
4. **Two deep reviews, and the machine does the half it can.**

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

   That answers four of the questions review 2 asks: a status in `INDEX.md`
   that disagrees with the entry's own `Status:` line, an entry id that names
   nothing, a cited path that does not resolve, counts that no longer add up.
   It is one second and it has already caught two things that had been wrong
   for a session.

   What it does not answer is whether a claim is **true**, and that is review
   1: every claim written this session against the code or the path it cites.
   Doing the mechanical half in one second is what leaves the time for the half
   that needs reading. Review 2 is then a cold read of the prose, as if someone
   else wrote it.
5. Commit and push with `scripts/git-sync.ps1`. Nothing else. Pass `-Summary
   -Since <the start instant>` on the last push of the session, so the numbers
   in `PROGRESS.md` are measured rather than counted by hand.
6. Read the CI run the push started. A push that leaves CI red without an entry
   naming why is not finished. A push carrying only documentation should carry
   `-NoCi` and then there is no run to read.
7. Print the kickoff prompt in chat only, in a code block. Section 3 says what
   it must contain.

## 3. The kickoff prompt

Printed in chat, in a code block, never written to a file.

**It is generic and it stays generic.** Everything that changes from session to
session lives in `PROGRESS.md`, which is tracked, versioned, and read first
anyway. A prompt that restates the work order is a second copy of it that goes
stale the moment an entry closes, and it costs the next session's context to
read something it is about to read again.

So the prompt carries only what a reader cannot get from the repository:

- The one-line statement of what `bit-cli` is and why it exists, because the
  next agent has to know that before it opens anything.
- What to read, in order, and nothing about what is in it.
- Whether the session is attended or unattended, and what to do when blocked.
- The one-line restore command for the corpus, because a fresh machine cannot
  read `RULES.md` section 7 out of a directory it does not have.

It carries **no** entry ids, no counts, no test numbers, no CI run id, and no
work order. Those are `PROGRESS.md`'s, which is where they are already correct.

The kickoff is therefore the same text every session, and the only thing that
makes one session different from the next is what `PROGRESS.md` says. That is
the point: the prompt is a pointer, and the record is the record.

`PROGRESS.md` has to hold up its end. Section 2 step 2 says what it must carry,
and the template at the top of the file says it again where a session will
see it.

## 4. Git

**`scripts/git-sync.ps1` is the only sanctioned way to commit and push.** It
enforces the rules below mechanically, so they stop being things to remember.

**Write the commit body to a file and pass `-BodyFile`.** Never type a body
into the shell. A body typed into `bash` as a PowerShell here-string ends at the
first apostrophe, and everything after it becomes shell commands: "the run's own
deadline" is enough to do it. That has cost two failed pushes and it will cost
more, because the failure is silent until git-sync reports a subject it never
received.

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -Message "Subject line" -BodyFile /tmp/msg.txt
```

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -PushOnly
```

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -Check
```

Every switch: `-Message`, `-BodyFile` (preferred) or `-Body`, `-Path` to stage
specific paths, `-Evidence` to force-add one benchmark past `.gitignore`,
`-NoPush` to commit only, `-PushOnly` to push what is already committed,
`-Check` to report without changing anything, `-FetchReferences` to restore the
corpus on a fresh clone, `-SkipGates` for a documentation-only change where the
tree is known green, `-NoReferences` to skip the corpus mirror on one push,
`-NoCi` to mark the commit so no CI run starts, `-Summary -Since <ISO>` to
print what the session did, and `-Force` to override the one refusal that is a
judgement rather than a rule. `-SkipGates` prints that it was used, so a
transcript shows the push carried no proof.

**A commit message that mentions a CI skip marker skips CI.** GitHub reads
`[skip ci]` anywhere in the message and does not care whether the sentence
around it meant it. The commit that introduced `-NoCi` explained the marker in
a sentence and shipped without a run: sixteen jobs skipped, silently, on the one
commit that changed the push tool. `git-sync` refuses a message carrying any of
the five markers unless `-NoCi` was passed, the same way it refuses an
attribution line, and for the same reason: rewriting a commit message on
somebody's behalf is worse than refusing one. Write it as `skip-ci` in prose, or
pass the flag.

**`-NoCi` for a push CI could not have caught anything in.** A run is sixteen
jobs and about five minutes, and the workflow's concurrency group cancels one
in flight when the next push lands, so a documentation push both costs a run
and destroys the one before it. `-NoCi` puts `[skip ci]` on the commit. It is
refused unless every staged path is under `TODO/`, `docs/`, `bench/`, `scripts/`
or a handful of root files: a "documentation-only" push carrying a source file
is exactly the one that needed CI. Two exceptions, both derived rather than
listed. `.github/` is never safe, because a workflow edit is the change whose
effect is only visible in a run. And a script the workflows actually invoke is
not safe either, which the script works out by reading `.github/workflows/`, so
a workflow that starts calling a new script makes that script unsafe on the same
commit with nothing for anybody to remember.

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
- **The corpus is pushed to the `references` branch**, not to `main`, and only
  when it changed. The script writes the tree, compares its hash against what
  `origin/references` already holds, and pushes nothing when they match. Before
  that it force-pushed 52 MB on every push, which is most pushes, for a corpus
  that changes about once a month.
- **The gates run before the push**, not after. `cargo fmt --all --check`,
  `cargo clippy -- -D warnings`, and `cargo test --workspace`.

## 4a. Tools on this machine

What a session should reach for before its own habits. Recorded here because
these were added mid-session and the next agent will not otherwise know they
exist.

### The four scripts a session runs, in the order it runs them

- **`scripts/gates.ps1`.** Every gate, one command, one answer. It is not a
  wrapper for convenience: it kills stray `bit-cli` and `loopback-*` processes
  first, because a release binary left running by an acceptance script holds its
  own executable open and the next build fails on a locked file that names
  neither; and it filters test failures with `^test \S+ \.\.\. FAILED` and
  `-CaseSensitive`, because `-match 'FAILED'` matches "0 failed" in the summary
  line and loses a flake's name exactly when it is needed. `-Fix` formats,
  `-Fast` skips `deny` and the build, `-Build` adds `--bins --examples`, `-Json`
  for a machine.

  It prints the toolchain first and warns when the `stable` it is using is
  behind the one CI would install. **Green here is not green there when this
  machine's rustc is older**: CI pins `stable`, which moves, and clippy gains
  lints with every release. `clippy::chunks_exact_to_as_chunks` arrived in 1.98
  on 2026-08-18 and a push that passed every gate on 1.97.1 was red on that one
  job four days later. It warns rather than fails, because a toolchain nobody
  has updated is not a reason to stop working, and it warns only about the
  toolchain actually in use, because `rustup check` lists every one installed.

```bash
rustup update stable
```

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **`scripts/check-todo.ps1`.** The mechanical half of the two deep reviews:
  statuses that disagree between `INDEX.md` and the entry, rows without entries
  and entries without rows, counts that do not add up in either the prose or
  the priority table, `T-NNN` references to entries that do not exist, dead
  links between `TODO/` files, and cited paths and line numbers that do not
  resolve, in this tree and in `reference/` when it is present.

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

- **`scripts/session-report.ps1`.** What the session did, measured: elapsed
  time from the start instant on `PROGRESS.md`'s state line, commits, files
  changed, lines added and removed, `scc` over `crates/`, entries done out of
  workable, and which entries closed, advanced or were filed. `git-sync
  -Summary -Since <ISO>` runs it after the push.

```bash
pwsh -NoProfile -File scripts/session-report.ps1 -Since 2026-08-22T01:11:24Z
```

- **`scripts/git-sync.ps1`.** Section 4.

### The rest

- **`codegraph`.** This repository has a `.codegraph/` index. Reach for it
  **before** `grep`, `find`, or opening a file, when the question is "how does
  X work", "where is X", or "what am I about to change". One call returns the
  verbatim line-numbered source of the relevant symbols plus the call paths
  between them and a blast-radius summary, which is what a grep-and-read loop
  costs several round trips to assemble. The MCP tool is
  `codegraph_explore`; the shell equivalent is
  `codegraph explore "<symbols or question>"`.

```bash
codegraph sync
```

  after editing, when a query does not reflect a change yet. It is incremental
  and takes under a second for a few files.

  What it is not for: raw byte inspection. A question about literal CR bytes or
  file encoding is `cat -A`'s, not codegraph's.

- **`scc`** counts code. `scc --no-cocomo crates/` is the whole usage. Use it
  when a doc claims a size, so the number is measured rather than remembered.

- **ISO 8601 UTC in the record.** A commit body and a `PROGRESS.md` state line
  carry the time the work was done, so a reader can line a commit up with a CI
  run and a `TODO/` entry without inferring order from position.

```bash
date -u +"%Y-%m-%dT%H:%M:%SZ"
```

  The same format the JSON output uses, which is the rule
  [section 5](#5-the-rules-that-bite-most-often) already carries for output.
  An entry that closes says when, on its `Status:` line or in its first
  closing sentence.

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
  [T-145](cli-surface.md), [T-160](cli-surface.md), [T-162](webseed.md),
  [T-184](disk-io.md), [T-172](metainfo.md).
- **Measure before building, when the entry describes what the code does.**
  Both 2026-08-21 additions above cost nothing to check and inverted the work:
  [T-184](disk-io.md) predicted pieces that can never be proved and they verify
  fine, and [T-172](metainfo.md) recommended strictness on the one dictionary
  this tree never re-encodes. One command each.
- **A corpus citation is evidence of what someone else did, not evidence that
  `bit-cli` does it.** Never let one become the other in a doc.
  [T-074](windows.md) is the worked example: the corpus named a librqbit
  defect, and the pinned version already had the fix.
- **Read the code, then the doc, then fix the doc.** Not the other way round.
  Three entries have described a state this tree was not in, and every one took
  one command to check.
- **The gates are as current as the toolchain under them.** CI installs
  `stable` on every run and this machine does not, so a clippy lint released
  between the two is invisible here and fatal there. `gates.ps1` warns about it
  now; `rustup update stable` is the fix and it costs a minute. Section 4a has
  the case that cost a red job.

### Testing

- **A test waits on the condition, never on a guessed duration**, and never
  asserts that the machine cannot fail some other way.
  [T-148](bench.md), [T-160](cli-surface.md) and [T-162](webseed.md) are the
  three worked examples, and all three cost a red job.
- **"Both of these will happen" is the same assumption as "this will happen in
  N seconds".** A fixture with two sources, two peers or two tasks that asserts
  each did some work is asserting a scheduling outcome it does not control.
  Arrange it: make each one the only supplier of something, and wait on the
  condition between the stages. [T-179](webseed.md)'s acceptance is the worked
  example, and it is the fourth entry on this line.
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
- **A control byte goes in a source file as an escape, never as itself.** A raw
  NUL makes the whole file binary to `grep`, which then skips it and says so in
  a line nobody reads. Two got in and neither was noticed for a session:
  `TOLERATED_TRAILING` in `torrent/bencode.rs` spelled its five bytes out
  literally, and a `TODO/` entry quoting a tracker's NUL-terminated error
  message had the escape interpreted on its way to the file. `gates.ps1` has a
  `text` gate for it now, and it fails rather than warns.
- **Headless parity is absolute:** nothing TTY-gated, nothing display-only, no
  prompting. stdout carries data only.

### PowerShell, and passing text between two shells

- **Never send prose through two shells.** A PowerShell here-string written
  inside a `bash` command is parsed by bash first, and `@'...'@` is `@` plus a
  single-quoted string plus `@`: it ends at the first apostrophe in the text.
  "the run's own deadline" turns the rest of a commit message into shell
  commands, and the first sign of it is a subject line that reads
  `on 'main', not 'of'`. It has happened twice.

  The fix is not better quoting. Write the text to a file with a file-writing
  tool, and pass the path:

  - a commit body: `git-sync -BodyFile <path>`;
  - anything else: write a `.ps1` to the scratchpad and run
    `pwsh -NoProfile -File <path>`. A script file is parsed once, by
    PowerShell, and nothing in it needs escaping.

  Same rule for `python -c` and `python - <<'PY'`: a heredoc is fine for code
  with no apostrophes and no backslashes, and a Windows path in a Python string
  literal has both. Prefer a file.
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
