# PROMPT_MISC

For work that is neither an ordinary `TODO/` entry nor the vendored trees: an
investigation, a one-off, a piece of tooling, a question that needs an answer
before it needs a plan.

**This one needs editing before it is pasted.** Everything in `< >` is a blank
to fill. The rest is the scaffolding that makes an unattended session produce
something a later session can use, and it is the same scaffolding in every
prompt here: read the record, measure before building, write down what was
found, and end with the two reviews.

Delete any block that does not apply. A block left in with its placeholder
still in it is worse than a block deleted: it reads as an instruction to do
something nobody asked for.

---

```
Continue bit-cli at C:\Users\AjamX\Downloads\TEMP\bit-cli

bit-cli is a non-interactive BitTorrent + HTTP client. Its reason for existing
is per-scope web seeds: attach arbitrary HTTP sources to an existing .torrent at
runtime, without rewriting it. Since 2026-08-22 it vendors its librqbit
dependencies under vendor/ and controls that fork.

=============================================================================
THE TASK
=============================================================================

<One paragraph: what to do, and what "done" looks like. Write the outcome, not
the method. If the method matters, say why it matters.>

<If this is an investigation rather than a change, say so here, and say what
question it has to answer. An investigation that ends in an entry with a
measurement is a success; one that ends in code nobody asked for is not.>

=============================================================================
READ FIRST
=============================================================================

1. TODO/PROGRESS.md   the measured baseline and what the last session did.
2. TODO/RULES.md      how this repository is worked on, including the only
                      sanctioned way to commit and push. Section 4a lists the
                      tools; section 5 is the rules that bite most often.
3. TODO/INDEX.md      every entry, one line each, sorted by id.
4. docs/AGENTS.md     the orientation: the tree, the tools, the gate contract.

Before typing a bit-cli flag, read man/bit-cli.json rather than grepping the
source or guessing.

<Add anything else this task needs read, with one line saying why. Delete this
line if there is nothing.>

TODO/ is the authoritative record.

=============================================================================
FIRST COMMANDS
=============================================================================

Record the start instant on PROGRESS.md's state line before doing anything.

  date -u +"%Y-%m-%dT%H:%M:%SZ"

Re-measure rather than trusting a recorded baseline:

  pwsh -NoProfile -File scripts/gates.ps1
  pwsh -NoProfile -File scripts/check-todo.ps1
  gh run list --limit 1

<If the task touches vendor/, add:
  pwsh -NoProfile -File scripts/vendor-status.ps1 >

=============================================================================
CONSTRAINTS
=============================================================================

<Delete what does not apply. Each of these has cost a session at least once.>

- Do not relitigate a settled decision. TODO/RULES.md section 6 lists them:
  librqbit stays the base, no daemon and no RPC, no SQLite and no state file,
  iroh is not being adopted, and MSRV is measured rather than chosen.
- Measure before building when the task describes what the code already does.
  Two entries in 2026-08 recommended work the code made unnecessary, and one
  command would have shown it in both cases.
- A claim needs evidence. A comparative claim without a committed benchmark
  does not ship, and a flag that does not move a number does not ship.
- <Anything specific to this task: a file not to touch, a budget, a deadline,
  a machine limitation.>

=============================================================================
WHAT TO PRODUCE
=============================================================================

<Delete what does not apply.>

- A TODO/ entry, in the file that matches the category, with the acceptance
  command actually run and its output recorded.
- <A script under scripts/, with the usage block every other one has.>
- <A document under docs/.>
- <A measurement under bench/, committed deliberately with -Evidence because
  bench/*.json is gitignored.>

If the answer turns out to be "nothing to do here", that is a result. Write it
down as an entry with what was measured, so the next session does not spend the
same hour finding out again.

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
THIS SESSION IS <UNATTENDED | ATTENDED>
=============================================================================

<If unattended: Resolve any pending operator decisions before starting; do not
ask once tasks are underway. An item that turns out to be blocked stays open
with the blocker named and what would unblock it, and the session moves on
rather than stopping.>

<If attended: Surface decisions with a recommendation and wait, rather than
guessing. Say which way you would go and why.>

When the operator says the session is ending, follow TODO/RULES.md section 2
exactly. Take the two deep reviews seriously and do them yourself:
check-todo.ps1 is the mechanical half only. The two that bite hardest are a
claim that is true of nothing and a number two documents disagree about.
```
