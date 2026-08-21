# Reference map

What is kept under `reference/`, and under what licence.

**The findings live in `reference/README.md`.** That file is the single entry
point: what each tree was read for, what was learned from it with file and line
citations, and what must not be copied. This file keeps the licence
determinations, because those are the safety-critical half and have to survive
`reference/` being deleted. See `TODO/licensing.md` for how each was made.

Nothing in a tracked file should cite a path under `reference/` directly. Cite
`reference/README.md`, which is where the detail is, and which is untracked
along with `PROMPT.md`.

## Licences

Forgetting which pile a file came from is how an AGPL function ends up in an
MIT tree.

| Tree | Licence | What is allowed |
| --- | --- | --- |
| `intermodal` | CC0-1.0 | Copy and adapt directly. The only one. |
| `fx-torrent` | Apache-2.0 | Permissive. Copying triggers the NOTICE and attribution terms; nothing has been copied. |
| `rqbit` | n/a, data | The issue and PR JSON corpus, not code. |
| `aria2-next` | documentation | Quote sparingly, with attribution. |
| `superseedr` | GPL-3.0-or-later | Read for shape. Do not copy, translate, or closely paraphrase. |
| `FluxDown` | AGPL-3.0 | Strongest boundary: network-use disclosure would attach to the netdisk deployment. |
| `yip` | AGPL-3.0-or-later | Same boundary as FluxDown. |
| `pluto-bittorrent` | **none stated** | No `LICENSE` file, so all rights reserved by default. Copying is not restricted, it is unlicensed. |

Three of the eight are copyleft and one is unlicensed, which is why every
finding in `reference/README.md` is written as a description of a technique
with a citation to check it against, and never as a snippet.

`cargo deny` refuses copyleft dependencies outright, and
`scripts/check-licence-gate.ps1` proves it against a probe crate. That gate is
what makes the boundary mechanical rather than a matter of memory.

## What is not under reference/

The `librqbit` source. It is a crates.io dependency, and every claim in `TODO/`
about "the pinned 9.0.0" was verified against the registry cache at
`~/.cargo/registry/src/index.crates.io-*/librqbit-9.0.0/`.

---

### T-122 reference/ is not deleted at the end of Phase B

Source:      PROMPT.md section 2.4
Category:    licensing
Priority:    P2
Effort:      S
Status:      open

Problem:     `reference/` holds **three** copyleft source trees and one
             unlicensed one inside the working directory of an MIT project. It
             is gitignored, so it cannot be committed by accident, but it is
             one `git add -f` away.
Relevance:   The safest copy of an AGPL tree is the one that is not there.
Approach:    Delete it when Phase B closes.
Acceptance:  `reference/` does not exist, this file still answers which tree
             carried which licence, and the findings that were in
             `reference/README.md` have been moved somewhere that survives.

**One thing to settle before deleting, and it is not settled yet.**
`reference/README.md` is where the findings now live, with file and line
citations into trees that will not exist afterwards. Deleting the directory
deletes it. Three options, and the choice is the operator's:

1. Move `reference/README.md` into `docs/` and track it. The citations become
   references to code the reader cannot open, which is honest but weaker: a
   claim about `holepunch.rs:356` cannot be checked once the file is gone.
2. Keep `reference/` and drop only the source trees, leaving the README and the
   `rqbit` JSON corpus. Smallest loss, and the copyleft source is still gone.
3. Delete all of it and accept that the findings are gone.

Option 2 is the honest default: the risk this entry names is the **source**
being one `git add -f` away, and a Markdown file of observations is not that
risk. Recorded rather than acted on, because deleting is not reversible.
