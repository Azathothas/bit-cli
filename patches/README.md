# Working on the vendored upstreams

`vendor/` holds three upstream repositories and thirteen crates that `bit-cli`
is built from. Why they are vendored at all is [`docs/vendoring.md`](../docs/vendoring.md).
This is how to work on them.

Three files and four scripts, and nothing else binds:

| | |
| --- | --- |
| [`vendor/upstream.json`](../vendor/upstream.json) | what is vendored, from where, at which commit |
| [`UPSTREAM.md`](UPSTREAM.md) | every change this repository has made, and why |
| [`TASKS.md`](TASKS.md) | the work the fork exists to do, in order |
| `scripts/vendor-sync.ps1` | put a tree in, or reconcile a new release onto it |
| `scripts/vendor-diff.ps1` | regenerate the patch series from the tree |
| `scripts/upstream-scan.ps1` | everything upstream has, ranked against our open entries |
| `scripts/vendor-status.ps1` | one screen: is the fork healthy, is a merge due |

## The model: the tree is the truth

The vendored tree is edited in place, like any other source in this
repository. `patches/<upstream>/*.patch` is **derived** from it and is never
applied to anything.

The alternative, a pristine tree plus patches applied by a setup step, was
considered and rejected: every edit then needs a refresh, a dirty tree is easy
to lose, and `rust-analyzer` reads the applied tree while the truth lives
somewhere else. Here there is nothing to forget, and `cargo build` on a fresh
clone builds what this machine builds.

What the derived series buys is the two things a working tree cannot say:

- **Review.** A change to somebody else's code, on its own, without the 389
  files around it.
- **Attribution.** Apache-2.0 asks a distributor to mark changed files as
  changed. The series and `UPSTREAM.md` are that mark.

So: after changing a vendored file, regenerate.

```bash
pwsh -NoProfile -File scripts/vendor-diff.ps1
```

```bash
pwsh -NoProfile -File scripts/vendor-diff.ps1 -Check
```

`-Check` fails when the series and the tree disagree, which is the state a
commit must never be in.

## Making a change

1. **Read the entry in `TODO/` first.** Every vendored change exists to unblock
   one, and the entry names the seam with a line number. A change with no entry
   behind it is a change nobody can review against anything.
2. **Edit the tree** under `vendor/`.
3. **Write it down in [`UPSTREAM.md`](UPSTREAM.md)**, before running anything.
   One section per change: what it is, which entry it unblocks, why it cannot
   be done outside the vendored tree, and whether it is meant to go upstream.
   The last one matters: a change shaped for upstream and a change shaped for
   this repository are different changes, and mixing them makes both harder.
4. **Regenerate the series** and **run the gates**.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

5. **Run upstream's own tests** when the change is in `librqbit` itself.
   `gates.ps1` runs `cargo test --workspace`, and the vendored crates are not
   workspace members, so their tests are not in it. CI does compile them under
   `-D warnings`, so a warning in them is still ours to patch: cargo caps lints
   for a registry dependency and does not cap them for a path dependency. The
   first entry in `UPSTREAM.md` is exactly that case.

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml
```

## Taking a new upstream release

```bash
pwsh -NoProfile -File scripts/vendor-status.ps1
```

That says whether anything is due at all, in a few seconds. Then read what is
in the release rather than merging blind:

```bash
pwsh -NoProfile -File scripts/upstream-scan.ps1
```

The scan fetches **everything** each upstream has: every release, every issue
and every pull request, open and closed, plus the commits since our base. Then
it ranks them, because six hundred items nobody reads is the same as no scan.
The vocabulary is the nouns in the titles of entries that are still open,
partial or blocked, taken from `TODO/INDEX.md` so it cannot go stale, plus a
short curated list of protocol and type names a title never says. A high tier
means a person should look, not that anything is wrong; the JSON record under
`patches/scan/` keeps every item either way.

```bash
pwsh -NoProfile -File scripts/vendor-sync.ps1 -Upstream rqbit -Ref v9.1.0 -Check
```

`-Check` says what the merge would do and changes nothing: how many files
merge cleanly, how many upstream added or removed, and how many conflict.

```bash
pwsh -NoProfile -File scripts/vendor-sync.ps1 -Upstream rqbit -Ref v9.1.0
```

Without `-Check` it performs the three-way merge, using the base commit
recorded in `vendor/upstream.json` as the common ancestor. A file changed on
one side only is taken from that side. A file changed on both is merged by
`git merge-file`, the same three-way merge git itself runs, and a conflict is
left in place with markers.

**The base is not advanced while anything conflicts.** Resolve the markers, run
the gates, then run the same command again to record the new base. That is
deliberate: a recorded base that does not describe the tree makes the next
merge wrong in a way nothing detects.

Then regenerate the series, because every patch's header names the base commit
it is against, and update the changelog:

```bash
pwsh -NoProfile -File scripts/release.ps1 -Bump patch
```

## What the sync script refuses to do

- **Write over an existing tree under `-Init`.** That is the operation that
  silently loses a fork.
- **Advance the base while a file is in conflict.**
- **Finish while a vendored file is one this repository's own `.gitignore`
  would swallow.** `.vscode/` did exactly that on the first vendoring: the
  files land on disk, never reach a commit, and a fresh clone then builds a
  different tree from the one that was tested. Either exclude the path in
  `vendor/upstream.json` or un-ignore it.

## Sending a change upstream

Nothing here requires it, and `TODO/RULES.md` section 5 says upstream has no
interest in this work, so nothing waits on it. But a fix that is upstream's bug
rather than our preference is worth offering, and the series is already in the
shape a pull request wants.

Mark it in `UPSTREAM.md` as **offered**, with the pull request URL, so the next
reconciliation knows the change may arrive from the other direction. A patch
that upstream accepts should be **deleted** here at the release that carries
it, not merged with itself.
