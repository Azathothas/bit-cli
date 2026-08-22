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

---

## The pivot, and it changes what the next two sessions are for

**`bit-cli` now vendors its `librqbit` dependencies and controls the fork.**
Three upstream repositories, thirteen crates, under `vendor/`, wired in with
`[patch.crates-io]`. That happened on 2026-08-22 on the operator's instruction,
halfway through an ordinary session.

The reason is in [`docs/vendoring.md`](../docs/vendoring.md) and the numbers are
in [`patches/TASKS.md`](../patches/TASKS.md): **nine entries are held up by a
seam `librqbit` does not expose**, and they include both P0 items in the record,
both partial P1 items, and both blocked entries. None of them could move while
the dependency was a published tarball.

**The next two to three sessions work from [`patches/TASKS.md`](../patches/TASKS.md),
not from this file's usual ordering.** That file is ordered, names the entry each
item unblocks, and says when to stop: when no entry in its table is still
waiting on a seam, the work order goes back to being derived from
[INDEX.md](INDEX.md)'s four questions and the vendored trees become maintenance.

The trees carry **one patch**, recorded in
[`patches/UPSTREAM.md`](../patches/UPSTREAM.md), and it unblocks nothing: it
silences a warning in upstream's own code that CI turns into an error. It is
there because it was the first exercise of the whole workflow, and everything
else the fork exists to do is still ahead.

## State

- **Last session:** 2026-08-22T10:30:26Z to 12:52Z, unattended at the start and
  redirected four times by the operator mid-session. Three of the four
  work-order entries, then the vendoring, then the tooling around it.
- **Tests:** 1,116 passing, 0 failing. The baseline at the start was 1,113.
- **Gates:** clean. One command:

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

- **CI:** green, all sixteen jobs, at run **32573162888** against commit
  `e1b96ef`. Every commit after it carries `[skip ci]`, because none of them
  changes a byte cargo compiles. Two runs before it went **red** and both are worth reading
  about under "What went wrong and was fixed": `1b0117e` on four Windows jobs
  and `bb878ce` on the third party notices.
- **Entries:** 152 items. 47 open, 6 partial, 2 blocked, 87 done, 10 deferred
  to Phase C. 87 of 142 workable done, 55 left.
- **Tree:** 84 Rust files, 49,845 lines of code, 11,608 of comment, measured
  with `scc --no-cocomo crates/`. That excludes `vendor/`, which is 389 files
  and 3.5 MB of somebody else's code.
- **Version:** `bit-cli` moved 0.1.0 to **0.2.0**, because the provenance of the
  shipped binary changed and nothing else in a version number says so.

## What the last session did

### The four-item work order, before the pivot

- **[T-190](disk-io.md)**, **done**, and the premise was wrong. The comment
  saying "a caller that named an output directory gets exactly that directory"
  is **true**: it is about `AddOptions::output_folder`, the per-add override
  that only `seed` sets, and `--dir` is the session's `download_directory`,
  which takes the other branch. Nothing about where bytes land changed. What was
  wrong is that the sentence reads as `--dir` to anyone who does not already
  know the field, and two readers read it that way. A test now names the landing
  path for both a multi-file and a single-file torrent.
- **[T-189](bench.md)**, **done**. The `bench` reports are in `docs/schema.md`
  now, generated from a `bench disk --json` run of its own. `environment` is the
  only thing left out, and it had to be: `host.os.distribution` exists on Linux
  and nowhere else, and the macOS reader has no interface table, so folding it
  in would have made the contract say which machine regenerated it and gone red
  on the next platform. Measured: renaming `Disk::write_calls` now fails the
  build and names the field.
- **[T-018](disk-io.md)**, **done**, and neither option the entry offered was
  the answer. `bench disk`'s striding is one end of a scale nothing could name,
  so the scale is named: `--run-length N`, defaulting to 1, which is exactly
  what it always did. At 64 the 16 KiB run reaches **98.6% and 96.7%** of the
  1 MiB run at two and four threads, against 71.6% and 52.8% while it strided.
  Every configuration asks for 32,768 writes; at a run length of 64, **512**
  reach the device, at every thread count.
- **[T-024](peers.md)** was the fourth item and was **not started**. The pivot
  came first. It stays open.

### The vendoring

- **Three upstreams, 389 files, 3.5 MB.** `ikatson/rqbit` at `v9.0.1`, and
  `librqbit-utp` and `librqbit-dualstack-sockets` pinned by commit because
  neither repository tags the 0.7.0 this tree builds against.
- **`[patch.crates-io]`, not path dependencies.** Only four of the thirteen are
  named directly; a path dependency would leave cargo resolving the other nine
  from crates.io, and two copies of `librqbit-core` are two sets of types that
  do not unify.
- **9.0.0 to 9.0.1 is one code change** and it is not cosmetic: it bounds
  `read_chunk` to the requested chunk when serving a piece, and upstream's own
  message says that without it, **on a partially selected torrent seeding is
  impossible**. This tree has three entries about selection and files that do
  not exist, and no test that would have caught it.
- **Four upstream files are deliberately not vendored.** `CLAUDE.md`,
  `AGENTS.md`, `GEMINI.md` and `AI_POLICY.md` are third-party agent
  instructions, and a file with one of those names anywhere under a repository
  is read as instructions by the tools working in it.

### The tooling around the fork

Six scripts and five documents, and every one was proved by being run rather
than by being read.

- **`vendor-status`** answers in a few seconds whether the fork is healthy:
  what each upstream is pinned to, whether a newer release exists, how far
  behind the base is, whether the patch series matches the trees, whether every
  patch has a section in the record, and whether the version, the changelog and
  the pins agree.
- **`upstream-scan`** fetches **everything** each upstream has, 614 items for
  rqbit against the 262 issues and 346 pull requests the corpus recorded, and
  ranks them. The vocabulary is derived from `INDEX.md`, from the titles of
  entries that are still open, partial or blocked, so it cannot go stale.
  73 need attention and 167 are worth a look.
- **`vendor-sync`** three-way merges a new release onto our tree using the
  recorded base as the common ancestor, and refuses to advance that base while
  anything conflicts.
- **`vendor-diff`** regenerates the patch series from the tree.
- **`release`** moves the version and writes the changelog section that names
  the upstream commit each tree was built from.
- **`check-prompts`** checks that every path and script a kickoff prompt names
  still resolves. The prompts live on the `references` branch, so a commit that
  renames a script never touches them.

Three kickoff prompts are on that branch at `reference/PROMPT-SAMPLEs/`: one for
ordinary work, one for the vendored trees, one for a one-off with blanks to
fill. `RULES.md` section 3 said the kickoff was never written to a file, which
stopped being true, and now says what they are instead.

### What the tooling found by being run

- **`upstream-scan` earned itself on the first run.** It found
  [rqbit#637](https://github.com/ikatson/rqbit/issues/637), a regression report
  against the exact release this tree vendors, **thirty minutes after it was
  filed**, and [rqbit#633](https://github.com/ikatson/rqbit/pull/633), the open
  MSE pull request that unblocks [T-163](peers.md).
- **The nzbd patch series is usable.** Nine maintained patches against rqbit
  8.1.1 at <https://github.com/pjunod/nzbd/tree/main/contrib/rqbit>, **MIT OR
  Apache-2.0** by that repository's README. The GitHub API reports
  `license: null` for it because there is no `LICENSE` file to classify, and
  reading that as "no licence" is wrong. Four of the nine bound the sets
  [T-040](memory.md) is about and one is exactly the seam
  [T-016](disk-io.md) is blocked on. One of them is **not** our fix, and the
  entry explains why: [T-020](peers.md) measured that the cap it adds has
  nothing to do with this repository's defect.

### What went wrong and was fixed

- **CI went red twice.** The second was the third party notices on `bb878ce`:
  they were regenerated before the version moved rather than after, so they
  still said 0.1.0. `release.ps1` prints that step beside the lockfile now,
  because a version bump has three things following it and CI fails on each
  separately.
- **The first was the vendoring commit**, `1b0117e`, on four Windows jobs.
  Cargo passes `--cap-lints allow` to a registry dependency and **not** to a
  path dependency, so `[patch.crates-io]` made upstream's warnings ours: an
  unused parameter in `vendor/librqbit-dualstack-sockets/src/bind_device.rs:27`
  became an error under the workflow-level `RUSTFLAGS: -D warnings`.
  Dropping the flag was tried and **reverted on the operator's instruction**,
  and the reason is the right one: development happens on Windows, so CI is the
  only place a warning on another platform is ever seen, and a build that does
  not fail on one cannot catch sloppy work. The warning is patched instead, and
  it became the first entry in `patches/UPSTREAM.md` and the first exercise of
  the whole patch workflow.
- **The version bump broke one test**, which asserted the extended handshake's
  client string as the literal `bit-cli 0.1.0`. A version change reported as a
  protocol failure. It builds the string from the crate version now.
- **Two defects in the new scripts, both found by running them.**
  `ConvertFrom-Json` turns a commit date into a `[DateTime]`, and interpolating
  one into a URL writes the local culture format, which GitHub cannot read and
  the retry loop spends two and a half minutes failing on. And .NET numbers
  named regex groups after unnamed ones, so a mixed pattern wrote the new
  version straight into the old one: `version = "0.2.00.1.0`.

### What the two reviews found

- **[T-193](cli-surface.md)**, filed and **done**. The mechanical half of the
  two reviews resolved a citation written long, `crates/bit-cli/src/cli.rs:2103`,
  and only checked the file's line count. Most of `TODO/` writes `cli.rs:2103`,
  which **matched nothing at all**. It now resolves short names through an index
  of every `.rs` under `crates/`, and where the prose names a symbol that occurs
  exactly once in the file, the cited line has to be near it. It found **nine
  stale line numbers across seven citations**, five older than this session and
  three made stale hours earlier by this repository's own changes.
- **Four entries closed with a `Status:` instant later than the commit that
  closed them**, guessed forward by five to ten minutes. Every one now carries
  its own commit's instant.
- **T-018 still said "which is why this is partial rather than done"** three
  paragraphs above a `Status:` line saying done.
- **Three claims in the new documentation could not be backed as written**, one
  of them true of nothing: `patches/UPSTREAM.md` said three patch directories
  "are empty" when they do not exist.

## In progress

Nothing is half-written. The vendored trees are unpatched on purpose and
`patches/UPSTREAM.md` says so.

Carried rather than finished:

- **[T-024](peers.md)** was the fourth work-order item and was never started.
- **[T-020](peers.md)** and **[T-040](memory.md)**, both P0, are now
  **actionable** rather than blocked. They are items 1 and 2 in
  [`patches/TASKS.md`](../patches/TASKS.md).
- **[T-191](bench.md)** and **[T-192](disk-io.md)** were filed this session and
  are open.

## Start here next session

**Work from [`patches/TASKS.md`](../patches/TASKS.md).** It is ordered, it names
the entry each item unblocks, and it carries the corpus sources. What follows is
its shape, not a second copy of it.

1. Nothing to read first. CI is green at **32572672382** against the tip.
2. **Item 0 in TASKS.md**: establish whether
   [rqbit#637](https://github.com/ikatson/rqbit/issues/637) affects `bit-cli`
   before building anything on the vendored tree. No fixture here is within
   three orders of magnitude of a two megabyte `.torrent`.
3. **Item 1, [T-020](peers.md)**, the open P0, and it is one `select!` match
   arm in `vendor/rqbit/crates/librqbit/src/session.rs`. The acceptance is
   `scripts/check-close-wait.ps1 -Ceiling 100`, which fails today. Offer it
   upstream.
4. **Item 2, [T-040](memory.md)**, the other P0, with four of the nzbd patches
   as prior art.
5. **Item 3, MSE, [T-163](peers.md).** Decide first whether to take upstream's
   shape from [rqbit#633](https://github.com/ikatson/rqbit/pull/633) or our own.
   Corpus: `reference/FluxDown/native/engine/vendor/librqbit/src/mse/`, which is
   MIT and already on this machine.

**[T-024](peers.md)** is the one ordinary entry left over from the last work
order. It is not blocked on anything and can be done whenever the fork work
leaves room; `bench swarm` can now generate the choke and unchoke events to
test it against.

Do not start the seven entries that were blocked on `librqbit` seams **outside**
the order in `patches/TASKS.md`. They are unblocked now, but they are unblocked
in a sequence, and the P0 items come first.

## Open questions for the operator

None outstanding. Three decisions were put to the operator mid-session and
answered:

- **Vendor all three upstreams**, not just the rqbit workspace, and not just the
  four crates named directly.
- **The tree is the truth and the patch series is derived from it**, rather than
  patches applied at build time or a git subtree.
- **Take 9.0.1 and bump `bit-cli` to 0.2.0 in one step**, rather than vendoring
  9.0.0 first.

One decision was taken unattended and is recorded where it belongs: `-D warnings`
comes out of the workflow-level `RUSTFLAGS` and stays in the clippy job, under
"What went wrong and was fixed" above and in
[`docs/vendoring.md`](../docs/vendoring.md).
