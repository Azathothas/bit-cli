# PROMPT_VENDOR

For a session whose work is the **vendored `librqbit` trees** rather than
`bit-cli` itself: upstream shipped a release, or a patch is wanted.

Paste it as-is. Like `PROMPT_TASK.md` it carries no work order and no counts,
because `TODO/PROGRESS.md` and `patches/TASKS.md` are where those are correct.

Two lines near the bottom are the only thing worth editing, and only when the
session is for a specific thing rather than "take what upstream shipped".

---

```
Continue bit-cli at C:\Users\AjamX\Downloads\TEMP\bit-cli

bit-cli is a non-interactive BitTorrent + HTTP client. Its reason for existing
is per-scope web seeds: attach arbitrary HTTP sources to an existing .torrent at
runtime, without rewriting it. It is built on librqbit, and since 2026-08-22 it
VENDORS librqbit rather than depending on the published crates, so that this
repository can fix what it is blocked on.

This session's work is the vendored trees.

=============================================================================
READ FIRST, IN THIS ORDER
=============================================================================

1. patches/README.md    how the vendored trees are worked on: the model, how to
                        make a change, how to take a new upstream release, and
                        what the sync script refuses to do.
2. patches/TASKS.md     the ordered work the fork exists to do, and the TODO/
                        entry each item unblocks. This is the work order for a
                        vendoring session.
3. patches/UPSTREAM.md  every change already made to somebody else's code, and
                        whether an upstream release could retire it. Nothing
                        here is ever offered upstream.
4. docs/vendoring.md    why the dependency is vendored at all, what it costs,
                        and what is deliberately not vendored.
5. TODO/PROGRESS.md     the measured baseline and what the last session did.
6. docs/AGENTS.md       the orientation: the tree, the tools, the gate
                        contract, and what a session owes at the end.
7. TODO/RULES.md        how this repository is worked on, including the only
                        sanctioned way to commit and push. Section 4a lists the
                        tools; section 5 is the rules that bite most often.

TODO/ stays the authoritative record. patches/ describes changes to somebody
else's code; the entry a change unblocks still closes in TODO/, with its own
acceptance run.

=============================================================================
FIRST COMMANDS
=============================================================================

Record the start instant on PROGRESS.md's state line before doing anything.

  date -u +"%Y-%m-%dT%H:%M:%SZ"

Then, in this order. The first is the one that says whether anything is due:

  pwsh -NoProfile -File scripts/vendor-status.ps1
  pwsh -NoProfile -File scripts/gates.ps1
  pwsh -NoProfile -File scripts/check-todo.ps1
  gh run list --limit 1

Before taking a release, read what is in it rather than merging blind:

  pwsh -NoProfile -File scripts/upstream-scan.ps1
  pwsh -NoProfile -File scripts/vendor-sync.ps1 -Upstream <name> -Ref <tag> -Check

The scan fetches every issue and pull request each upstream has, open and
closed, and ranks them against the entries this repository still has open. It
never writes to TODO/.

=============================================================================
THE RULES THAT ARE SPECIFIC TO THIS WORK
=============================================================================

- The vendored tree is the truth. Edit it in place. patches/*.patch is
  DERIVED from it by scripts/vendor-diff.ps1 and is never applied to anything.
  Regenerate after every change, and never hand-edit a patch file.

- Every vendored change gets a section in patches/UPSTREAM.md BEFORE the
  gates are run: what it does, which TODO/ entry it unblocks, why it cannot be
  done outside the vendored tree, and whether an upstream release could retire
  it. A change with no entry behind it says so explicitly rather than
  inventing one. Nothing in patches/ is ever offered upstream: TODO/RULES.md
  section 6 settles that and section 6a is the wider rule.

- The gates do not lint or test the vendored crates, but CI compiles them
  under -D warnings, because cargo caps lints for a registry dependency and
  does not cap them for a path dependency. An upstream warning is therefore
  ours to patch. That is deliberate: development happens on Windows and CI is
  the only place a warning on another platform is ever seen.

- Run upstream's own tests when the change is in librqbit itself.
  --target-dir is not optional: without it cargo writes gigabytes into the
  vendored tree, and TODO/cli-surface.md T-197 is what that cost.

    cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit

- A version bump has three things following it and CI fails on each
  separately: Cargo.lock, THIRD_PARTY.md, and the CHANGELOG heading.
  scripts/release.ps1 prints all three.

- Nothing closes as "upstream problem". That rule is why the trees are
  vendored at all.

=============================================================================
REMOTE OPERATIONS
=============================================================================

Azathothas/bit-cli is the only repository an agent may write to. Everything
else is read only: clone, fetch, read an issue or a pull request, run
scripts/upstream-scan.ps1. Never open an issue, a pull request, a discussion,
a comment or a review on anybody else's repository, under any framing, and
never fork or star one. The patches under patches/ are for this project and
are never offered upstream.

TODO/RULES.md section 6a is the rule and it is not a judgement call. If a
session believes an exception exists, it is wrong: leave it, and say so in
PROGRESS.md under open questions for the operator.

=============================================================================
THIS SESSION IS UNATTENDED
=============================================================================

Resolve any pending operator decisions before starting a task; do not ask if
tasks are already underway. An item that turns out to be blocked stays open
with the blocker named and what would unblock it, and the session moves to the
next item rather than stopping.

A reconciliation that conflicts is not a failure and is not to be forced. The
sync script leaves the markers in place and refuses to advance the recorded
base; resolve them, run the gates, and run it again.

When the operator says the session is ending, follow TODO/RULES.md section 2
exactly, and add to it: patches/UPSTREAM.md must describe every patch, and
scripts/vendor-status.ps1 must exit 0. Take the two deep reviews seriously and
do them yourself. The two that bite hardest are a claim that is true of nothing
and a number two documents disagree about.

=============================================================================
WHAT THIS SESSION IS FOR
=============================================================================

Work patches/TASKS.md in its order unless the line below says otherwise.

  SPECIFIC WORK (delete this block to just work the list):
  <what to do, in one or two sentences>
  <the upstream ref to move to, if that is the point of the session>
```
