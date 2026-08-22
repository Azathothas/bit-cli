# What this repository changed in the vendored upstreams

One section per change. This file is the record Apache-2.0 asks for, which is
that changed files are marked as changed, and it is what a reviewer reads
instead of a 389 file diff. The patch series beside it is generated from the
tree by `scripts/vendor-diff.ps1`; this is the part a script cannot write.

How to add one: [`README.md`](README.md).

---

## Nothing yet

As of 2026-08-22T12:05Z the three vendored trees are **byte for byte upstream**
at the commits [`vendor/upstream.json`](../vendor/upstream.json) records, so
there is no patch series at all. `scripts/vendor-diff.ps1` creates
`patches/<upstream>/` when it has something to put in it, and it has not, so
those directories do not exist yet.

That is on purpose. The session that vendored them stopped at a green build so
that the first real patch lands against a tree already proved to compile, test
and ship. Everything the fork exists to do is in [`TASKS.md`](TASKS.md), in
order, with the entry each item unblocks.

Verify it for yourself:

```bash
pwsh -NoProfile -File scripts/vendor-diff.ps1 -Check
```

---

## The template

Copy this for the first change and delete the section above.

```
## <upstream>: <what it is>

Unblocks:    T-NNN, and the line in TODO/<file>.md that names the seam
Files:       vendor/<upstream>/<path>, and the patch that carries it
Upstream:    not offered | offered, <url> | landed in <ref>, delete this
Added:       <ISO 8601 UTC>

What it does, in a paragraph a reviewer can check against the diff.

Why it cannot be done outside the vendored tree. This is the part that dates
fastest: a seam that was private may become public, and then the patch should
go rather than stay.

How it was measured, or which test holds it.
```

Three things that section must always answer, because each has already cost
somebody a session in this repository:

- **Which entry.** A vendored change with no `TODO/` entry behind it cannot be
  reviewed against anything, and it is the first thing a reconciliation has to
  weigh when upstream touches the same lines.
- **Whether it is offered upstream.** A change shaped for upstream and a change
  shaped for this repository are different changes. Deciding which one it is
  afterwards means writing it twice.
- **Why it has to be here.** `TODO/RULES.md` section 5 has a rule about a doc
  describing a state the tree is not in, and a patch justified by a seam that
  has since opened is exactly that.
