# PROMPT_TASK

The ordinary development kickoff. Paste it as-is; it takes no editing.

It is generic on
purpose, by `TODO/RULES.md` section 3: everything that changes from session to
session lives in `TODO/PROGRESS.md`, which is tracked, versioned and read
first anyway. A prompt that restates the work order is a second copy of it that
goes stale the moment an entry closes.

Use this when the work is `bit-cli` itself. Use `PROMPT_VENDOR.md` when the
work is the vendored `librqbit` trees.

---

```
Continue bit-cli at C:\Users\AjamX\Downloads\TEMP\bit-cli

bit-cli is a non-interactive BitTorrent + HTTP client. Its reason for existing
is per-scope web seeds: attach arbitrary HTTP sources to an existing .torrent at
runtime, without rewriting it.

=============================================================================
READ FIRST, IN THIS ORDER
=============================================================================

1. TODO/PROGRESS.md   the measured baseline, what the last session did, and the
                      work order under "Start here next session". This prompt
                      carries none of that on purpose: that file is where it is
                      correct, and a second copy goes stale the moment an entry
                      closes.
2. TODO/RULES.md      how this repository is worked on, including the only
                      sanctioned way to commit and push. Read section 4a even
                      if you think you know the tools, and section 3 before
                      writing the next kickoff prompt.
3. TODO/INDEX.md      every entry, one line each, sorted by id, with the
                      counts and the argument behind the current ordering.
4. docs/AGENTS.md     the orientation: the tree layout, which tool answers
                      which question, the gate contract, and what a session
                      owes at the end. RULES.md is the normative one and this
                      is the map.

Before typing a bit-cli flag, read man/bit-cli.json. It is the whole command
surface as a CLIspec document: every command, every flag, the values it
accepts, its default, and every exit code with whether a retry could succeed.
Grep and skim it; a full read is not needed. Do not grep the source for a flag
name and do not guess.

TODO/ is the authoritative record. There is no PROMPT.md.

=============================================================================
FIRST COMMANDS
=============================================================================

Record the start instant on PROGRESS.md's state line before doing anything.
Everything at the end that measures the session reads it from there.

  date -u +"%Y-%m-%dT%H:%M:%SZ"

Then re-measure rather than trusting a recorded baseline:

  pwsh -NoProfile -File scripts/gates.ps1
  pwsh -NoProfile -File scripts/check-todo.ps1
  pwsh -NoProfile -File scripts/check-docs.ps1
  gh run list --limit 1

If the work touches vendor/, add:

  pwsh -NoProfile -File scripts/vendor-status.ps1

=============================================================================
THE CORPUS
=============================================================================

reference/ holds 39 upstream implementations indexed by reference/RESEARCH.md,
plus reference/HISTORY/ for text a live document gave up. Most are MIT and four
need care: TODO/reference-map.md carries the licence determination and the
evidence for each. It is gitignored on main and lives on the `references`
branch. If it is missing on this machine:

  pwsh -NoProfile -File scripts/git-sync.ps1 -FetchReferences

Trust RESEARCH.md. Do not re-derive licences, re-fetch issues, or re-clone.
Do verify a path exists before citing it, and read the cited code before
ranking work on it. If the work is to clone, mine, survey or investigate
somebody else's repository, docs/reference-mining.md is the procedure and it
is binding. If the work is to turn an idea into a filed entry,
docs/task-authoring.md is.

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
tasks are already underway. An item that turns out to be blocked stays open in
TODO/ with the blocker named and what would unblock it, and the session moves
to the next item rather than stopping.

docs/ and docs/examples/ are updated in the same push as any change to what
the tool does, by RULES.md section 2 step 3a, and scripts/check-docs.ps1 is the
gate that holds them. Every command in an example is run before it is written
down.

When the operator says the session is ending, follow TODO/RULES.md section 2
exactly. Take the two deep reviews seriously and do them yourself:
check-todo.ps1 is the mechanical half only. Every session so far has found
things it passed, in its own work and in work older than itself, and the two
that bite hardest are a claim that is true of nothing and a number two
documents disagree about.
```
