# Kickoff prompts

Three prompts, one per kind of session. Pick one, paste it, and let the
repository say the rest.

| file | for | editing |
| --- | --- | --- |
| [`PROMPT_TASK.md`](PROMPT_TASK.md) | ordinary `bit-cli` work from `TODO/PROGRESS.md`'s order | none |
| [`PROMPT_VENDOR.md`](PROMPT_VENDOR.md) | the vendored `librqbit` trees: a new upstream release, or a patch | one optional block |
| [`PROMPT_MISC.md`](PROMPT_MISC.md) | an investigation, a one-off, a piece of tooling | fill the blanks |

## Why they carry so little

`TODO/RULES.md` section 3 is the rule and it applies to all three: everything
that changes from session to session lives in `TODO/PROGRESS.md`, which is
tracked, versioned and read first anyway. A prompt that restates the work order
is a second copy of it that goes stale the moment an entry closes, and it costs
the next session's context to read something it is about to read again.

So a prompt carries only what a reader cannot get from the repository: what
`bit-cli` is, what to read and in what order, whether the session is attended,
and the one-line command that restores the corpus on a machine that does not
have it.

`PROMPT_MISC.md` is the exception and says so: a one-off task is exactly the
thing the repository cannot describe, so that one has blanks.

## Where these live

`reference/` is gitignored on `main` and lives on the `references` branch,
which `scripts/git-sync.ps1` pushes when it changes. That is the right place
for these: they are not part of the build, nothing reads them at runtime, and
they are wanted on a fresh machine.

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -FetchReferences
```

## Keeping them true

A prompt naming a script that no longer exists is the same defect as a `TODO/`
citation pointing at a line that moved, and it is worse, because it is the
first thing a session reads. `scripts/check-prompts.ps1` checks that every
command and path a prompt names still resolves.

```bash
pwsh -NoProfile -File scripts/check-prompts.ps1
```
