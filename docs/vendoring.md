# Why the librqbit crates are vendored

`bit-cli` is built on `librqbit`, by decision 7.3, and that decision is not
being relitigated. What this file is about is where the source comes from:
`vendor/`, in this repository, rather than crates.io.

This file says why, and what that costs. How to work with the vendored trees is
[`patches/README.md`](../patches/README.md).

## The reason

Nine entries in `TODO/` are held up by a seam `librqbit` 9.0.0 does not
expose. Seven of them name it with a file and a line number:

| entry | what it needs |
| --- | --- |
| [T-016](../TODO/disk-io.md) | a resume cache without turning on session persistence |
| [T-022](../TODO/peers.md) | an HTTP tracker announce per address family |
| [T-100](../TODO/bep-coverage.md) | the send half of an extension message |
| [T-102](../TODO/bep-coverage.md) | BEP 55 hole punching, through `PeerConnectionHandler` |
| [T-132](../TODO/multi-source.md) | peer identity on `TorrentStorage` |
| [T-163](../TODO/peers.md) | MSE, which is a wire-level handshake |
| [T-167](../TODO/bep-coverage.md) | BEP 54 `lt_donthave`, which has no inverse of `on_have` |

None of the seven could move while the dependency was a published tarball. The
other two were the record's two P0 items. [T-020](../TODO/peers.md): its second
defect was a `tokio::select!` arm in upstream's accept loop that a failed
handshake check disabled, and no amount of configuration reached it.
[T-040](../TODO/memory.md): nothing reclaimed a peer row and nothing bounded
the sets that hold them.

**The table is kept as the argument it was** rather than rewritten into a
status board: [`patches/TASKS.md`](../patches/TASKS.md) is the status board and
`scripts/check-todo.ps1` holds it to the entries. Two of the five were not on
this list at all. [T-210](../TODO/peers.md) came out of building
[T-132](../TODO/multi-source.md), and [T-195](../TODO/peers.md) out of building
[T-194](../TODO/peers.md); neither could have been fixed any other way either.
Nothing in the record is blocked now.

The full table, and what to do about each, is
[`patches/TASKS.md`](../patches/TASKS.md).

`TODO/RULES.md` section 5 says nothing here closes as "upstream problem",
because upstream has no interest in this work and there is nowhere to defer to.
Vendoring is what makes that rule affordable rather than aspirational.

## What is vendored

Three upstreams and thirteen crates, pinned in
[`vendor/upstream.json`](../vendor/upstream.json).

| upstream | crates | pinned by |
| --- | --- | --- |
| `ikatson/rqbit` | eleven, `librqbit` and its siblings | the `v9.0.1` tag |
| `ikatson/librqbit-utp` | `librqbit-utp` | a commit |
| `ikatson/librqbit-dualstack-sockets` | `librqbit-dualstack-sockets` | a commit |

The last two are pinned by commit because neither repository tags the 0.7.0
this tree builds against: `librqbit-utp`'s newest tag is `v0.4.0` and
`librqbit-dualstack-sockets` has no tags at all. Both commits are the default
branch head whose `Cargo.toml` reads 0.7.0.

## How it is wired

`[patch.crates-io]` in the root `Cargo.toml`, not path dependencies.

Only four of the thirteen are named directly. The other nine arrive
transitively, and a path dependency redirects the edge it is written on and
nothing else, so cargo would resolve `librqbit-core` twice: once from
`vendor/`, once from crates.io. Two copies of a crate are two sets of types
that do not unify, and the error appears somewhere unrelated. `[patch]`
redirects the source, so every edge in the graph resolves to the same tree.

`[workspace] exclude = ["vendor"]` goes with it. Without it, cargo walks up from
`vendor/librqbit-utp/Cargo.toml`, finds this workspace, and refuses to build a
package that is neither a member nor excluded.

The vendored crates are **not** workspace members, so `cargo clippy --workspace`
does not lint them and `cargo test --workspace` does not run their tests.

**They are still compiled under `-D warnings`, and that is a decision rather
than an accident.** Cargo passes `--cap-lints allow` to a dependency it resolved
from a registry and does **not** pass it to a path dependency, so
`[patch.crates-io]` made every warning in the vendored trees ours. On the
vendoring commit an unused parameter in
`vendor/librqbit-dualstack-sockets/src/bind_device.rs:27` failed four Windows
jobs, and nobody here wrote it.

Dropping the flag was tried and reverted. Development happens on Windows, so CI
is the only place a warning on another platform is ever seen, and a build that
does not fail on one cannot catch sloppy work. The cost is that an upstream
warning has to be patched in the vendored tree, and
[`patches/UPSTREAM.md`](../patches/UPSTREAM.md) is where each is recorded. The
first entry in that file is exactly this warning, and it is what proved the
whole patch workflow.

`vendor/rqbit` is its own workspace and its own tests are run on purpose, not
by default. `--target-dir` keeps 7.2 GB of build output out of a tree that is
supposed to hold nothing but somebody else's source:

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

Upstream's workspace listed `desktop/src-tauri` as a member, and `desktop/` is
one of the four things this repository deliberately does not vendor, so that
command could not load the workspace at all until the member was removed. It is
the second entry in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).

## What it costs

- **A build compiles thirteen more crates.** The gates took 52.0 s before the
  vendoring and 113.4 s on the first run after it, which is the cold build of
  all thirteen. Warm they take 65.9 s. The cost is paid again whenever a
  vendored file changes, which is what a patch does.
- **3.5 MB in the repository**, 389 files.
- **Upstream stops being visible.** Nobody sees a release note for a dependency
  that no longer has a version to update. `scripts/upstream-scan.ps1` is the
  answer to that and it is meant to be run on every version bump.
- **A new release has to be reconciled rather than accepted.**
  `scripts/vendor-sync.ps1` does the three-way merge and refuses to advance the
  recorded base while a file is in conflict.

## What is deliberately not vendored

`vendor/upstream.json` carries the exclusion list and the reason for each. Two
are worth repeating here.

**`CLAUDE.md`, `AGENTS.md`, `GEMINI.md` and `AI_POLICY.md`.** Upstream ships all
four. A file with one of those names anywhere under a repository is read as
instructions by the tools working in that repository, so vendoring them would
put a third party's instructions inside ours. They are data about upstream's
own process and nothing here needs them.

**`desktop/`.** A Tauri application, 1.6 MB of the 4.4 MB upstream ships,
depending on nothing this tree builds.

**`.vscode/`.** Not for weight but for correctness: this repository's own
`.gitignore` has a `.vscode/` rule, so those files would land on disk and never
reach a commit. A vendored tree that differs between a fresh clone and the
machine that vendored it reports the same files as newly added upstream at
every reconciliation, forever. `scripts/vendor-sync.ps1` checks for that class
of file now and refuses to finish while one exists.

## Licensing

`librqbit` and its siblings are **Apache-2.0 only**, not dual licensed.
Copyright 2021 Igor Katson. Apache-2.0 permits modification and redistribution
provided the licence travels with it, any `NOTICE` content is preserved, and
**changed files are marked as changed**.

[`patches/UPSTREAM.md`](../patches/UPSTREAM.md) is that mark: every change this
repository makes to a vendored tree is recorded there, and
`scripts/vendor-diff.ps1` regenerates the diff itself as a reviewable series.
`THIRD_PARTY.md` states the obligation and is generated from `Cargo.lock`, so
it follows the vendored versions automatically.

`cargo deny check` covers the vendored crates the same as any other: it reads
`Cargo.lock`, which names them.

## The version story

`bit-cli` moved from 0.1.0 to 0.2.0 with the vendoring, because the provenance
of the shipped binary changed and nothing else in a version number says so.
`scripts/release.ps1` moves the version and writes the changelog section, and
that section names the upstream commit each tree was built from. A release is
therefore reproducible from the changelog alone.
