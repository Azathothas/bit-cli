# reference/HISTORY

What left a live document, and why it left.

Nothing here is current. Every file is text that was true when it was written,
was superseded, and was moved rather than deleted so that the citations,
commit SHAs and issue references inside it keep resolving. A live document that
gave something up carries a one line pointer to the file that received it.

This directory lives on the `references` branch with the rest of `reference/`,
which is gitignored on `main`. To get it on a fresh clone:

```bash
pwsh -NoProfile -File scripts/git-sync.ps1 -FetchReferences
```

| file | what it holds | what superseded it |
| --- | --- | --- |
| [`RULES-section-6-iroh.md`](RULES-section-6-iroh.md) | the retired paragraph of `TODO/RULES.md` section 6 ruling that `iroh` is not adopted and that no NAT crate is to be reached for | the operator's ruling of 2026-08-24, and [T-238](../../TODO/peers.md) |
| [`RESEARCH-web-seed-style.md`](RESEARCH-web-seed-style.md) | Web seed style detection and source lifecycle | T-004, T-130 and T-137 |
| [`RESEARCH-trackers.md`](RESEARCH-trackers.md) | Tracker tiers, the BEP 15 backoff, and the scrape convention | T-063, T-064 and T-065 |
| [`RESEARCH-bep6.md`](RESEARCH-bep6.md) | BEP 6, the fast extension | T-100 |
| [`RESEARCH-fastresume-and-idle-peers.md`](RESEARCH-fastresume-and-idle-peers.md) | fastresume, and closing idle peers | T-016 and T-020 |
| [`RESEARCH-create-torrent-defect.md`](RESEARCH-create-torrent-defect.md) | The librqbit create_torrent extra piece hash | T-080 |

The five `RESEARCH-*.md` files are corpus sections whose work finished. Each
left a one line pointer in `reference/RESEARCH.md` at the heading it used to
fill, so a reader following a citation lands here rather than on nothing.

## What belongs here and what does not

Here: a superseded ruling, an ordering argument that a later ordering replaced,
the record of a working brief that has been absorbed, a corpus section whose
work is finished.

Not here: anything that still binds. A rule that only one document states goes
into `TODO/RULES.md` before that document loses it, and a fact that only one
document holds goes into the entry or into `docs/` before it is cut. Checking
that deliberately is the whole risk of moving something into an archive.

Not here either: a run's output, which goes under `.tmp/`, and a benchmark that
is evidence for an entry, which goes under `bench/`.
