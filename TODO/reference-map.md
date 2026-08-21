# Reference map

What is kept under `reference/`, and under what licence.

This file keeps the licence determinations, because those are the
safety-critical half and have to survive `reference/` being deleted. See
`TODO/licensing.md` for how each was made.

**No tracked file cites a path under `reference/`, and none depends on
anything there.** Every finding a `TODO/` entry rests on is written into that
entry, which is what T-122 below closed. `reference/` and `PROMPT.md` are both
untracked working material, and a reader with neither can still work through
this list.

## Licences

Forgetting which pile a file came from is how a copyleft function ends up in an
MIT tree.

| Tree | Licence | What is allowed |
| --- | --- | --- |
| `intermodal` | CC0-1.0 | Copy and adapt directly. The only one. |
| `fx-torrent` | Apache-2.0 | Permissive. Copying triggers the NOTICE and attribution terms; nothing has been copied. |
| `rqbit` | n/a, data | The issue and PR JSON corpus, not code. |
| `aria2-next` | documentation | Quote sparingly, with attribution. |

Every tree still here is permissive or is not code. That is the whole point of
the 2026-08-21 pass: the boundary is now a property of what is on disk rather
than of what a reader remembers.

`cargo deny` refuses copyleft dependencies outright, and
`scripts/check-licence-gate.ps1` proves it against a probe crate. That gate is
what makes the boundary mechanical.

## What is not under reference/

The `librqbit` source. It is a crates.io dependency, and every claim in `TODO/`
about "the pinned 9.0.0" was verified against the registry cache at
`~/.cargo/registry/src/index.crates.io-*/librqbit-9.0.0/`.

---

### T-122 The copyleft and unlicensed reference trees are deleted

Source:      PROMPT.md section 2.4, closed by the operator on 2026-08-21
Category:    licensing
Priority:    P2
Effort:      S
Status:      **done**

Problem:     `reference/` held **three** copyleft source trees and one
             unlicensed one inside the working directory of an MIT project. It
             is gitignored, so it could not be committed by accident, but it
             was one `git add -f` away.
Relevance:   The safest copy of an AGPL tree is the one that is not there.
Approach:    Delete them, and rewrite every entry that cited one so it rests on
             the specification or the decision rather than on the tree.
Acceptance:  None of the four is on disk, no tracked file names one, and every
             entry that used to cite one still says what it needs to say.

**Done.** Four trees were removed on 2026-08-21: two AGPL-3.0, one
GPL-3.0-or-later, and one with no `LICENSE` file at all. The reason is the one
this entry always gave, plus the operator's: their licences are incompatible
with MIT and the work they were read for is finished.

**Nothing was taken from any of them.** The provenance tables in
`TODO/licensing.md` that existed to record it were empty when they were
deleted, and they were empty because every finding in the corpus is written as
a description of a technique with a citation, never as a snippet.

Four entries cited one of the four by path, and all four now stand on their
own:

| Entry | Cited | Now rests on |
| --- | --- | --- |
| [T-081](create-seed.md) | a v2 merkle implementation | BEP 52 itself, with the padding-truncation case written into the entry |
| [T-092](bench.md) | a synthetic load generator | the four properties the entry needs, listed in its Approach |
| [T-102](bep-coverage.md) | a NAT traversal crate | BEP 55, and the `librqbit` type that blocks it |
| [T-207](phase-c.md), [T-209](phase-c.md) | a status command's mode enum | decision 7.4 and the aria2 parity list |

What was **not** deleted: `intermodal` (CC0-1.0), `fx-torrent` (Apache-2.0),
the `aria2` documentation, and the `rqbit` issue corpus, which is JSON rather
than code. Those are permissive or are data, they are still cited, and the
reason this entry existed does not apply to them.

`reference/README.md` survives with the four trees' sections removed. It stays
untracked, because it is a working document and not a deliverable.
