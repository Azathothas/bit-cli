# Progress

**Read this first.** It says what the last session did and what is in flight.
It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
The work order: [INDEX.md](INDEX.md), "Start here".

---

## State

- **Last session:** 2026-08-21, 15:35Z to 17:20Z, unattended. Feature work. The
  whole work order the previous session wrote was completed.
- **Tests:** 1,028 passing, 0 failing, measured with `cargo test --workspace`.
  The baseline at the start was 960.
- **Gates:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  clean, `cargo fmt --all --check` clean, `cargo deny check` reports advisories,
  bans, licenses and sources all ok.
- **CI:** green on all sixteen jobs at run **32505742044**, against commit
  `f93c2b9`, **after one rerun of one job**. The three pushes before it were
  green first time: **32499547496**, **32502137086** and **32503875870**.
  Naming a run and the commit it describes is deliberate: that line stays true,
  where "the latest" is wrong by the next push.

  The rerun is the part worth reading. `Test (ubuntu-latest)` failed on
  `cmd::peers::tests::a_sampled_swarm_carries_what_came_from_each_peer` with
  the peer still `connecting` and `errors: 0` at the end of its five second
  sample. Rerunning the same job on the same commit passed with no change,
  which is what separates a flake from a break. It is the second red job that
  one test has cost and the entry that owns it,
  [T-160](cli-surface.md), now carries both failures and the fix: it samples
  until the bytes arrive rather than once for a duration hoped to be long
  enough. The first fix quoted the right rule and applied it to only one of the
  test's two timing assumptions.

  The push carrying this file starts a run of its own. Read it before starting
  work: `gh run list --limit 1`.
- **Entries:** 146 items. 56 open, 4 partial, 2 blocked, 74 done, 10 deferred
  to Phase C.
- **New on this machine, and now in [RULES.md](RULES.md) section 4a:**
  `codegraph` with an index at `.codegraph/`, to be reached for before grep or
  a file read; `scc` for counting code; and ISO 8601 UTC timestamps in commit
  bodies and in this file.

## What the last session did

Worked the "Start here" order in [INDEX.md](INDEX.md) and finished it. Four
entries closed, three filed, over four pushes.

**Three of the four ran differently from how they were written**, and measuring
before building is what found it each time.

- **[T-179](webseed.md)**, done, and the one that built what its entry
  described. A piece filled from several HTTP sources could fail its hash with
  no way to say which of them sent the wrong bytes, so the choices were to
  retire every mirror that contributed or none. `webseed/ledger.rs` records a
  SHA-1 of every block against the source that served it and convicts every
  source whose hash differs from the bytes the session then verified, read back
  off the disk rather than fetched again. Only a block two sources disagreed
  about is ever read, so a healthy run pays one `have` dump and nothing else.
  Measured on a 640 KiB fixture with two mirrors, one lying once about every
  range: the honest mirror served 655,360 bytes and the liar 327,680, ten
  pieces resolved, nothing evicted, and all twenty of the liar's blocks
  convicted while the honest mirror finished the torrent.

- **[T-184](disk-io.md)**, done **with its premise corrected under it**. The
  entry predicted a selection holding pieces it can never prove and a seeder
  announcing them. Neither happens: the unselected half of a boundary piece is
  written into the file it belongs to, so the piece verifies and `seed` offers
  exactly what it can serve. [T-013](disk-io.md)'s own closing said so and this
  entry was written without reading it. What the measurement found instead had
  nothing saying it: selecting the middle of a three-file torrent leaves
  `a.bin` at its **full** 3,000,000 bytes holding 1,013,440 real ones, and
  `c.bin` short at 1,959,680. One looks complete in a directory listing.
  `download --json` now reports both under `torrents[].partial`, and `verify`
  takes `--select-file` so pieces outside a selection are `not_selected` rather
  than counted as failures.

- **[T-004](webseed.md)**, done, and smaller than its entry once three of its
  four cases turned out free. A declared style is taken as given, a source from
  `httpseeds` is BEP 17 and one from `url-list` is BEP 19 by the key it came
  from, and a `file:` source has no wire style. Only a command-line HTTP source
  is asked, with one request for one byte. Two defects found while wiring it:
  the `httpseeds` collection overwrote an explicit `--web-seed-style`, and
  `webseed test` composed its probe URL as BEP 19 whatever the style, so a
  healthy Hoffman seed was reported unusable twice over.

- **[T-172](metainfo.md)**, done **with its recommendation corrected under
  it**. It asked for strict inside `info` and tolerant outside, on the argument
  that anything accepted inside `info` must be re-encodable byte-identically.
  This tree never re-encodes `info`: it hashes it from a recorded span and
  splices those bytes back. Hashing from the span is what makes **tolerance**
  survivable, not strictness. So both questions are tolerated and neither is
  silent: keys out of order and trailing whitespace or NUL are read and
  recorded, and `bit-cli info` reports them.

**Two defects were found after the work order was finished, both while
reviewing, and both fixed.** [T-160](cli-surface.md)'s test above. And the
style probe [T-004](webseed.md) added blocked `attach_sources` for a source's
whole connect timeout, ten seconds by default, before **any** bridge started
serving: an unreachable mirror delayed the reachable ones. The pass is now
bounded by five seconds or the caller's own connect timeout, whichever is
shorter, and a source cut off by the budget falls back to BEP 19 and says so.
`one_unreachable_mirror_does_not_hold_up_the_others` points one source at
TEST-NET-3 with a thirty second timeout and asserts the clock as well as the
answer.

**One defect had to be fixed rather than filed.** T-172's approach says to turn
the `rustorrent` adversarial checklist into fixtures. The depth fixture does not
fail, it kills the process: the bencode parser had no recursion bound, and a
twenty kilobyte document of ten thousand nested lists exited with
`STATUS_STACK_OVERFLOW`, which is not a panic and which `catch_unwind` cannot
see. A `.torrent` fetched from a URL and a tracker's response are both untrusted
input. `MAX_DEPTH` is 100 and a real torrent reaches about six.

## In progress

Nothing is half-written. Every entry touched is either complete or explicitly
open or blocked with its blocker named.

Three things are carried rather than finished, and all three are recorded where
they belong:

- **[T-020](peers.md)** stays the only open P0. Half of it is fixed; the
  stranded-socket half and the poisoned-listener finding are open.
- **[T-040](memory.md)** and **[T-090](bench.md)** stay partial. Neither needs
  a longer run; both need the specific measurements named in their entries.
- **[T-167](bep-coverage.md)** is blocked, not abandoned. It sits at the height
  of its value in the work order with the blocker named, which is the rule.

## Start here next session

The order is in [INDEX.md](INDEX.md) under "Start here", re-derived at the end
of this session. The first three are:

1. **[T-185](cli-surface.md)**, effort S, and it is first because it is a P1
   against two P2s. `--exclude-file` used without `--select-file` selects
   nothing and downloads everything. Filed this session out of T-184's
   measurements. Most of the machinery exists: `crate::selection::resolve`
   takes a file count and returns the complement, and `verify` passes one.
   What is left is where `download` gets the count, and the entry names both
   halves of that.
2. **[T-143](multi-source.md)**, effort M. Attaching a source to a torrent that
   has already started, now the highest web seed item because everything above
   it closed.
3. **[T-164](peers.md)**, the peer half of smart ban. [T-179](webseed.md) built
   everything about it that is not peer-specific. Read the `librqbit` seam and
   name it in the entry before pricing it, the way T-167 had to.

## Open questions for the operator

None blocking. T-172, the only one that was formally open, is closed with its
recommendation corrected and the argument written under it.
