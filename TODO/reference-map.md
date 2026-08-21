# Reference map

What is kept under `reference/`, and under what licence.

This file keeps the licence determinations, because those are the
safety-critical half and have to survive `reference/` being deleted. See
`TODO/licensing.md` for how each was made.

**No tracked file cites a path under `reference/`, and none depends on
anything there.** Every finding a `TODO/` entry rests on is written into that
entry, which is what T-122 below closed. A corpus citation in a `TODO/` entry
names where somebody else solved a problem; it is never evidence that `bit-cli`
solves it. `reference/` is untracked working material and a reader without it
can still work through this list.

## The corpus as it stands

The operator replaced the four-tree corpus with **twenty-two upstream
implementations** on 2026-08-21, indexed by `reference/RESEARCH.md`. That file
is the entry point: three tiers by usefulness, then cross-cutting sections A to
H. Section D maps `bit-cli` TODO ids to the best source for each, section C
lists eleven metainfo shapes a parser has to survive, section F is the licence
determination, section G records what was removed during cleaning and why.

| Tree | Licence |
| --- | --- |
| `intermodal` | CC0-1.0 |
| every other tree: `torrent`, `nanotorrent`, `TorrentNG`, `superseedr`, `fx-torrent`, `mkbrr`, `gosh-dl`, `vortex`, `rustorrent`, `mtorrent`, `n0-mainline`, `seedchamp`, `aria2_rust`, `FluxDown`, `aquatic`, `torrust-actix`, `create-torrent`, `parse-torrent`, `bqti`, `dht-spider`, `tc` | MIT |

Twenty-one MIT and one CC0-1.0. Every one is permissive and compatible with
`bit-cli`'s MIT licence and its permissive-only `deny.toml`, so **nothing in
this corpus needs deleting** and the deletion that T-122 records does not apply
to any of it. `RESEARCH.md` section F carries the per-tree evidence, read from
the licence file on disk in every case except `nanotorrent` and `mtorrent`,
which carry no `LICENSE` file and declare MIT in their manifest instead.

Two to handle with care, both recorded in section F: `tc`, whose README and
`LICENSE` disagree, and `vortex`, whose badge and `LICENCE.txt` disagree. In
both cases the file on disk is MIT. Confirm before reusing anything from
either.

**Two records this replaces.** `fx-torrent` was recorded here as Apache-2.0;
its own `LICENSE` file is MIT, and the Apache determination was wrong. And this
file described a four-tree corpus of `intermodal`, `fx-torrent`, the `aria2`
documentation and the `rqbit` issue JSON. That corpus is gone, superseded
rather than deleted, and the `aria2-next` and `rqbit` trees it named are not
present. The `rqbit` issue corpus is still the source of most entry `Source:`
lines below and in every other file here; those lines record where an entry
came from and stay true whether or not the JSON is on disk.

Forgetting which pile a file came from is how a copyleft function ends up in an
MIT tree. That risk is why this table exists, and with a wholly permissive
corpus the table is now a record rather than a fence.

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

What was **not** deleted: `intermodal` (CC0-1.0), `fx-torrent`, the `aria2`
documentation, and the `rqbit` issue corpus, which is JSON rather than code.
Those are permissive or are data, they are still cited, and the reason this
entry existed does not apply to them.

Two corrections to this entry as it was written, neither of which changes what
it did. `fx-torrent` is **MIT**, not Apache-2.0: its `LICENSE` file says so and
the earlier determination was wrong. And the corpus this entry describes was
replaced on 2026-08-21 by the twenty-two trees above, so the `aria2-next` and
`rqbit` directories it names are no longer on disk. The entry stays **done**:
what it closed was the removal of four incompatible trees and the rewriting of
the four entries that cited them, and both of those happened.
