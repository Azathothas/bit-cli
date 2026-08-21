# Progress

**Read this first.** It says what the last session did and what is in flight.
It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
The work order: [INDEX.md](INDEX.md), "Start here".

---

## State

- **Last session:** 2026-08-21, unattended. Feature work. The whole work order
  the previous session wrote was completed.
- **Tests:** 960 passing, 0 failing, measured with `cargo test --workspace`.
  The baseline at the start was 931.
- **Gates:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  clean, `cargo fmt --all --check` clean, `cargo deny check` reports advisories,
  bans, licenses and sources all ok.
- **CI:** green on all sixteen jobs at every one of this session's four pushes:
  runs **32488138363**, **32490561687**, **32491624786** and **32493508597**,
  the last against commit `22f84bc`. Naming the runs rather than "the latest"
  is deliberate: a line that says which commit a run describes stays true.
  Check the current state with `gh run list --limit 1`.
- **Entries:** 143 items. 56 open, 4 partial, 2 blocked, 71 done, 10 deferred
  to Phase C.

## What the last session did

Worked the "Start here" order in [INDEX.md](INDEX.md) and finished it. Eight
entries closed, one moved to blocked, two new ones filed. Four pushes, each
with its own CI run.

**Two silent wrong answers in the web seed path, which is what the order put
first, and both were real.**

- **[T-171](metainfo.md)**, done. `http_seeds` read the BEP 17 `httpseeds` key
  as a list only while `url_list` beside it accepted a string or a list, so a
  torrent whose `httpseeds` is a bare string yielded zero HTTP sources with no
  error. Both keys now read through one accessor.
- **[T-166](peers.md)**, done, and **the premise needed correcting before the
  test could be written, which is what found the defect.** The bridge had
  neither of the two BEP 10 tables, so the vortex PR 103 mistake could not
  exist in bridge code. It existed one level down: `Message::deserialize`
  routes an incoming extension id against `librqbit`'s own constants, which
  this bridge never advertised. Id 3 was decoded as `ut_metadata`, failed to
  parse, and ended the connection. Id 1 was decoded as `ut_pex` and survived
  only because an empty dictionary happens to satisfy that type.
  `crates/bit-cli-core/tests/bridge_protocol.rs` is a session written by hand
  that numbers its extensions unlike the bridge and unlike `librqbit`.

**Four flags that reached no code, and a fifth that could only fail.**

- **[T-181](cli-surface.md)**, done. `--max-overall-*-rate` and
  `--max-download-rate` were two flags aiming at one `librqbit` field and the
  wrong one arrived, so a per-torrent cap capped the whole run and a whole-run
  cap capped nothing. Measured over four torrents with
  `scripts/check-overall-rate.ps1`: 4.20 MiB/s against a 4 MiB/s whole-run cap,
  19.69 MiB/s with the same number per torrent, 392.64 MiB/s uncapped.
  `--tracker-list-url` is a bounded fetch. `--no-pex` warns, because
  `librqbit` 9.0.0 has no switch for peer exchange.
- **[T-183](cli-surface.md)**, new and done. `--web-seed-list-url` was read,
  and read only into a function that always errors, on every call site
  including `download`. Neither the audit that found T-181's four flags nor the
  `clap`-tree test T-181 built could have found it, because both look for a
  field with no reader and this one has a reader.

**A mirror missing one file is still a mirror.**

- **[T-005](webseed.md)**, done. A permanent status on one file retired the
  whole source, including the files it was serving correctly. It now drops the
  pieces that file touches and reconnects with the smaller bitfield. A second
  defect underneath it: a permanent failure also spent
  `--web-seed-max-errors`, so a narrowed mirror went into cooldown through the
  back door.
- **[T-158](cli-surface.md)**, done, because T-005 needed it. Adding two fields
  to a report made the schema check fail correctly, and regenerating the
  documented way deleted two rows it should have kept. `merge_schema` gives the
  writer the tolerance the reader already had.

**Two fixtures, and the arithmetic they were built to break was already
right.**

- **[T-177](disk-io.md)** and **[T-174](metainfo.md)**, both done, one fixture.
  Piece length 1,986,560 over three files chosen so every boundary falls inside
  a piece and the first file is shorter than one piece. Four tests. The
  end-to-end one asserts every file's bytes individually, because fx-torrent
  issue 98 is a payload where every piece hashed and only the first file was
  playable. The write fan-out is counted exactly: 301 blocks plus one per
  straddling boundary is 303 write operations.

**One entry moved to blocked, and reading the code is what moved it.**

- **[T-167](bep-coverage.md)**, BEP 54 `lt_donthave`, was ranked as the
  cheapest correctness win in the corpus. It is twenty lines to send and
  nothing to receive: `librqbit` 9.0.0 has `on_have` and no inverse, and every
  extension message it does not know reaches a catch-all that logs and ignores.
  Sending one would be noise that looks like a feature. Two upstream changes
  would unblock it, both named in the entry.

**One entry filed and not worked.**

- **[T-184](disk-io.md)**, open, out of T-177's fixture. A piece straddling the
  boundary between a selected file and an unselected one has no decided
  behaviour, and since T-013 closed the unselected half has nowhere on disk to
  go. So `--select-file` can leave pieces that can never be proved, and `seed`
  will announce them.

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
of this session. The first four are:

1. **[T-179](webseed.md)**, smart ban, effort M. It is top of the list because
   [T-005](webseed.md) put it there: a partial mirror now keeps serving, so
   more sources fill one payload at once, and a bad piece still cannot be
   attributed. Corpus: `torrent/smartban/smartban.go`, 83 lines.
2. **[T-184](disk-io.md)**, effort M. The boundary piece under
   `--select-file`, filed this session. The entry recommends announcing only
   whole pieces the selection covers, which is the rule the web seed bridge
   already uses for its own bitfield.
3. **[T-004](webseed.md)**, effort S. BEP 17 auto-detection, and only the
   `--web-seed` command-line case is left: the metainfo half is decided by
   which key the URL came from and [T-171](metainfo.md) added the test that
   pins it.
4. **[T-172](metainfo.md)**, effort S. Strictness on read. The entry carries a
   recommendation with its argument and can be built on it without an operator
   answer. [T-174](metainfo.md) arrived at the same position independently from
   the piece-length side.

## Open questions for the operator

None blocking.

- **[T-172](metainfo.md)** is still formally an open question, and it no longer
  needs an answer to start. Two upstream implementations resolved it in
  opposite directions, which is why it is a decision rather than a defect, and
  the entry recommends strict inside `info` and tolerant outside it with the
  reasoning. [T-174](metainfo.md)'s closing took the same position for piece
  lengths, strict on write and tolerant on read, without having read this
  entry first.
