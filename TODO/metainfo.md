# Metainfo

Reading a `.torrent` somebody else wrote.

`bit-cli` accepts metainfo from a file, from a URL, from stdin, from a magnet
and from a peer. Every one of those is untrusted input, and every one reaches
the same parser, `crates/bit-cli-core/src/torrent/metainfo.rs`. This file
tracks the shapes that parser has to survive.

The list is not guesswork. `reference/RESEARCH.md` section C enumerates
**eleven shapes** a parser meets in the wild, each verified against a fixture
or a fetched issue. `bit-cli` already handles four of them, and each of those
four is worth recording, because the reason to write this file down is so the
next reader does not have to rediscover which half is done.

## What is already handled

| Shape | Where | Test |
| --- | --- | --- |
| `url-list` as a bencoded **string** rather than a list | `torrent/metainfo.rs:293` `url_list` branches on `Value::Bytes` | `:656` `a_url_list_is_read_whether_it_is_a_string_or_a_list` |
| An info hash as **32 base32 characters** as well as 40 hex | `torrent/metainfo.rs:40` `InfoHash::parse`, `:67` `decode_base32` | `:803` `info_hashes_parse_from_hex_and_base32`, and `source.rs:298` for the bare-hash source form |
| BEP 47 padding files, `attr` containing `p` | `torrent/metainfo.rs:107`, `:116` `is_padding` | `:825` `padding_files_are_recognised` |
| `private` read from **inside** `info`, never as a top-level boolean | `torrent/metainfo.rs`, `Info::private` | round-trip coverage in `create` and `edit` |

Two of those four are worth a sentence on why they matter rather than just
that they pass.

**`url-list` as a bare string is the shape that would cost `bit-cli` its
reason to exist.** The fixture is real:
`torrent/metainfo/testdata/flat-url-list.torrent` contains
`8:url-list29:https://archive.org/download/`, a bencoded string where a naive
parser expects a list. `torrent/metainfo/urllist.go:11` handles it by branching
on the first byte, `l` meaning list and anything else meaning a single string;
`TorrentNG/crates/rt-metainfo/src/parse.rs:368` and
`gosh-dl/src/torrent/metainfo.rs:391` do the same. A parser that assumes a list
silently drops **the only web seed such a torrent has**, which for a tool whose
whole subject is web seeds is the worst available failure: no error, no
warning, and a download that falls back to peers as though the torrent had
never named a mirror. `bit-cli` gets this right, and
[T-171](#t-171-httpseeds-written-as-a-bencoded-string-is-silently-dropped)
below is the same defect surviving in the other key.

**Base32 info hashes are not a curiosity.** `parse-torrent/index.js:27`
accepts `/^[a-z2-7]{32}$/i` beside 40 hex characters, because base32
`urn:btih:` values are what older clients emit and they are the same twenty
bytes. `bit-cli` accepts a bare info hash as a source, so this is on the front
door.

---

### T-171 httpseeds written as a bencoded string is silently dropped

Source:      `reference/RESEARCH.md` section C, found in the doc pass of 2026-08-21
Category:    metainfo
Priority:    P2
Effort:      S
Status:      **done**

Problem:     `Metainfo::url_list` accepts the BEP 19 `url-list` key as either a
             bencoded list or a bare bencoded string, which is right.
             `Metainfo::http_seeds` at `torrent/metainfo.rs:311` accepts the
             BEP 17 `httpseeds` key as a **list only**:

             ```rust
             self.root
                 .get("httpseeds")
                 .map(Value::as_text_list)
                 .unwrap_or_default()
             ```

             `Value::as_text_list` (`torrent/bencode.rs:339`) calls
             `as_list()`, which returns `None` for `Value::Bytes`
             (`torrent/bencode.rs:305`), and the `unwrap_or_default()` turns
             that into an empty vector. So a torrent whose `httpseeds` is a
             single bencoded string loses every HTTP seed it has, with no
             error and no warning.
Relevance:   This is the exact defect the `url-list` half was written to avoid,
             surviving in the key next to it. The asymmetry is the tell: one
             accessor branches on the value's shape and the one immediately
             below it does not. `bit-cli` is a web seed tool, so silently
             reading zero sources out of a torrent that names one is a
             correctness bug in the feature the project exists for, not a
             parsing nicety.

             BEP 17 does specify `httpseeds` as a list, so a torrent doing this
             is non-conformant. That is not a defence: BEP 19 specifies
             `url-list` as a list too, and `bit-cli` already decided to accept
             the string form there because it exists in the wild. The decision
             has to be the same on both keys or the reason for it was not a
             reason.
Approach:    One line. Give `http_seeds` the same branch `url_list` has, or
             better, factor the shared behaviour into one helper both call so
             the two cannot drift again. `gosh-dl/src/torrent/metainfo.rs:391`
             `parse_url_list` is that helper in another tree: one parser that
             accepts a bencoded string **or** a list and filters to `http://`
             and `https://`, called from `:125` for `url-list` and `:128` for
             `httpseeds`. Take the structure, and see
             [T-004](webseed.md) for the mistake to leave behind.
Acceptance:  A fixture whose `httpseeds` is a bare bencoded string yields one
             HTTP seed from `bit-cli info --json` and from `webseed list`, and
             the test sits beside
             `a_url_list_is_read_whether_it_is_a_string_or_a_list` so the pair
             is obvious. A second assertion that both accessors are exercised
             by the same fixture.

**Fixed, and the fix is that both keys now read through one accessor.**

`Value::as_text_or_text_list` (`torrent/bencode.rs:352`) takes the shape
branch: a `Value::Bytes` yields one entry, anything else falls through to
`as_text_list` as before. `url_list` (`torrent/metainfo.rs:293`) and
`http_seeds` (`:311`) both call it and neither carries a branch of its own, so
there is no longer a place for the two to drift apart. `url_list` also lost the
duplicated `self.root.get("url-list")` its old branch needed.

`as_text_list` was left alone rather than widened. `announce_tiers`
(`torrent/metainfo.rs:288`) calls it on each tier of `announce-list`, where a
tier is a list by BEP 12 and a bare string means something different from a
one-element tier. Widening the shared accessor would have changed tracker
parsing as a side effect of a web seed fix, so the tolerant reader is a second
method and the callers that want it ask for it. The bencode test asserts both
halves of that: the new accessor takes the string form and the old one still
refuses it.

**The two lists stay separate, which is the half of `gosh-dl` not to copy.**
`gosh-dl/src/torrent/metainfo.rs:391` `parse_url_list` is the structure this
took: one parser, called from `:125` for `url-list` and `:128` for `httpseeds`.
What that tree then does at `webseed.rs:479` is merge the two into one list and
hard-code `WebSeedType::GetRight` at `:303`, throwing away the style it had
just parsed. `bit-cli` marks `httpseeds` sources BEP 17 at collection time
(`crates/bit-cli/src/webseed_args.rs:265`), and which key a URL came from is
the only signal for style that costs no network round trip, which is what
[T-004](webseed.md) rests on. The `webseed list` acceptance asserts the style
survives, so a later merge would fail a test rather than pass quietly.

**Proven by reverting the fix, not by writing the test after it.** With
`http_seeds` put back to `Value::as_text_list`, all four new tests fail and
nothing else does. The two unit tests are in the second run because `cargo
test` stops at the first failing binary and `bit-cli` is ordered ahead of
`bit-cli-core`.

```
$ cargo test --workspace          # with http_seeds reverted to as_text_list
test cmd::info::tests::a_web_seed_key_written_as_a_string_is_still_reported ... FAILED
test cmd::webseed::tests::a_web_seed_key_written_as_a_string_still_resolves_to_a_source ... FAILED
test result: FAILED. 307 passed; 2 failed

$ cargo test -p bit-cli-core --lib -- torrent::metainfo   # same revert
test torrent::metainfo::tests::httpseeds_is_read_whether_it_is_a_string_or_a_list ... FAILED
test torrent::metainfo::tests::both_web_seed_keys_read_the_string_shape_and_stay_separate ... FAILED
test result: FAILED. 19 passed; 2 failed
```

The fixture is `TorrentFixture::web_seed_keys_as_strings`
(`crates/bit-cli/src/test_support.rs`), which is `single_file` with **both**
keys rewritten as a bare bencoded string. One fixture rather than two, because
the defect is one key accepting a shape the key beside it does not, so the two
accessors have to be exercised by the same torrent. Both keys are outside
`info`, so the info hash is unchanged and the test asserts that too.

```
$ cargo test --workspace          # with the fix
test torrent::metainfo::tests::httpseeds_is_read_whether_it_is_a_string_or_a_list ... ok
test torrent::metainfo::tests::both_web_seed_keys_read_the_string_shape_and_stay_separate ... ok
test torrent::bencode::tests::one_string_and_a_list_of_them_read_the_same_way ... ok
test cmd::info::tests::a_web_seed_key_written_as_a_string_is_still_reported ... ok
test cmd::webseed::tests::a_web_seed_key_written_as_a_string_still_resolves_to_a_source ... ok
```

### T-172 Strictness on read is undecided, and the error does not say

Source:      `reference/RESEARCH.md` section C, 2026-08-21
Category:    metainfo
Priority:    P2
Effort:      S
Status:      open

Problem:     Two questions about hostile or sloppy bencode have never been
             answered deliberately, and whatever the parser does today it does
             by accident rather than by decision:

             1. **Unsorted keys.** BEP 3 requires a bencoded dictionary's keys
                to be sorted. Real torrents violate it.
             2. **Trailing bytes** after the top-level dictionary.
Relevance:   Both are real and both have a documented cost.

             intermodal
             [Issue 454](https://github.com/casey/intermodal/issues/454)
             (OPEN) is a torrent created by uTorrent/2210 that "works fine in
             normal torrent clients" and is refused with
             `bencode encoding corrupted (Keys were not sorted)`. A strict
             reader rejects torrents that every other client opens, and the
             user has no way to tell a strictness decision from a corrupt file.

             anacrolix
             [Issue 992](https://github.com/anacrolix/torrent/issues/992)
             (CLOSED) is the trailing-byte case: `after decoding metainfo:
             expected EOF`, again on files other clients accept. The two
             implementations resolved it in opposite directions, which is the
             evidence that this is a decision rather than a right answer.
             `mkbrr/torrent/update.go:210` `decodeTorrentRoot` **tolerates**
             trailing whitespace and NUL, accepting `ErrUnusedTrailingBytes`
             when the remainder is only `' '`, `\t`, `\r`, `\n` or `0`.
Approach:    Pick one position per question, write it in the error, and test
             both branches.

             The recommendation, and the argument for it: **strict on the info
             dictionary, tolerant everywhere else.** The info dictionary is
             hashed, so anything `bit-cli` accepts there it must be able to
             re-encode byte-identically or the info hash moves, which is what
             exit code 15 already protects. Outside `info` nothing is hashed,
             so tolerance costs nothing and buys the uTorrent torrents.
             Trailing whitespace and NUL after the top-level dictionary are
             outside `info` by definition, so follow mkbrr and accept them.

             Whatever is chosen, the error must name the decision. "Keys were
             not sorted" tells a user their file is broken; "this torrent's
             keys are not sorted, which BEP 3 requires and `bit-cli` enforces
             inside `info`" tells them what to do.

             `TorrentNG/crates/rt-metainfo/src/parse.rs:20` `parse_torrent` is
             the technique that makes strictness survivable at all: the info
             dictionary is hashed **from its recorded byte span in the original
             buffer**, never re-encoded. `bit-cli` already relies on that
             property; this entry is about the keys around it.
             `rustorrent/docs/DEEP_AUDIT_REPORT_2026-07-13.md` lists the full
             adversarial set beside these two, non-canonical integers,
             duplicate keys, excessive depth, excessive value counts, invalid
             lengths and truncation, and is the checklist to turn into fixtures.
Acceptance:  A fixture with unsorted keys and a fixture with trailing NUL
             bytes each produce the decided outcome, the error text names the
             rule rather than the symptom when one is refused, and `README.md`
             states the position in one sentence.

### T-173 A zero-length path component has no defined meaning

Source:      `reference/RESEARCH.md` section C, 2026-08-21
Category:    metainfo
Priority:    P3
Effort:      S
Status:      open

Problem:     A file entry may carry `path: ["", "foo"]`. Nothing in `bit-cli`
             says what that means, and the path planner has no test for it.
Relevance:   parse-torrent
             [Issue 89](https://github.com/webtorrent/parse-torrent/issues/89)
             (CLOSED) is the case: a torrent with `path: ["", "foo"]` and one
             with `path: ["foo"]` are **stored differently by at least one
             common client**, and `path.join` collapses them to the same
             string, so the difference disappears at the moment it matters.
             Two entries that are distinct in the metainfo becoming one path on
             disk is the same family as [T-072](windows.md), case-colliding
             paths silently overwriting, which was a P0 here.

             `bit-cli` plans every path before it opens anything and reports
             the mapping in `--json`, so it already has the machinery to handle
             this correctly and visibly. What it does not have is a decision or
             a fixture.
Approach:    Decide, then report. The defensible reading is that an empty
             component is dropped and the drop is **reported like any other
             rename**, because that is what the existing path planner does for
             every other name it changes, and a silent drop is what the issue
             above is complaining about. The alternative, refusing the
             torrent, is worse, because the file is otherwise fetchable.
             Two entries that collapse to one path after the drop must collide
             and be renamed by the existing collision rule rather than
             overwrite, which is the part a test has to prove.
Acceptance:  A fixture with `path: ["", "foo"]` beside `path: ["foo"]` lands
             two files, both named in `--json` with the reason, and neither
             overwrites the other. Sits in `crates/bit-cli-core/tests/hostile_paths.rs`
             with the rest of the planner's adversarial set.

### T-174 A piece length that is not a multiple of 16 KiB has no fixture

Source:      `reference/RESEARCH.md` section C, 2026-08-21
Category:    metainfo
Priority:    P2
Effort:      S
Status:      **done**

Problem:     BEP 3 permits any `piece length`. Every fixture in this
             repository uses a power of two, so the arithmetic on the last
             block of a piece is only ever exercised on the easy case.
Relevance:   vortex [PR 124](https://github.com/Nehliin/vortex/pull/124) is
             what the hard case costs. With `piece_length = 1,986,560`, which
             is `121 * 16384 + 4096`, the **last subpiece of every non-last
             piece is short**. The code computed `end_idx = offset + 16384`,
             which ran past the buffer, panicked, and then **double-panicked in
             the destructor**, so the process died without a usable message.
             The fix is one `min`: `end_idx = (start + SUBPIECE).min(piece_len)`,
             plus a `!thread::panicking()` guard in the `Drop`.
             [PR 129](https://github.com/Nehliin/vortex/pull/129) is the
             follow-on and the more important lesson: reject an invalid piece
             request **at the protocol boundary**, and never let one reach the
             file layer.

             `bit-cli` has two places this arithmetic lives and both are its
             own code rather than `librqbit`'s: the web seed bridge, which
             turns a piece request into byte ranges, and the storage layer's
             span mapping. Neither has a non-power-of-two fixture.
Approach:    A fixture, not a fix. The fix may already be right, and the point
             of the entry is that nothing proves it either way. Build a torrent
             with `piece length = 1986560` over a payload that spans several
             pieces and at least two files, and run it through `verify`, a web
             seed fetch, and a bridge round trip. If the arithmetic is right
             the fixture costs one test; if it is wrong this is a P0 hiding
             behind a missing case.

             Note that BEP 52 removes the question: v2 requires a power of two
             at least 16 KiB (`nanotorrent/src/bittorrent/torrent_create.rs:390`,
             `rustorrent/src/torrent.rs:300`). So this is a v1-only hazard, and
             it will still be a v1-only hazard after [T-081](create-seed.md),
             because v1 torrents do not stop existing.
Acceptance:  `bit-cli verify`, `bit-cli webseed fetch` and a bridge round trip
             all succeed on a `piece length = 1986560` fixture, and the last
             block of a non-final piece is asserted to be 4096 bytes rather
             than 16384.

**The arithmetic was already right. This was a fixture, and it cost one test,
which is the outcome the entry allowed for and named first.**

The fixture is shared with [T-177](disk-io.md) and is described in full there:
piece length **1,986,560**, which is `121 * 16384 + 4096`, over three files of
1,500,000, 2,500,000 and 900,000 bytes. One piece length serves both entries
because a piece that is not a whole number of blocks and a file boundary that
falls inside a piece are the two halves of the same adversarial case.

**What the number is chosen to break.** vortex
[PR 124](https://github.com/Nehliin/vortex/pull/124) is the failure: with a
piece length like this the **last subpiece of every non-final piece is short**,
4,096 bytes rather than 16,384. That tree computed `end_idx = offset + 16384`,
ran past the buffer, panicked, and then double-panicked in the destructor, so
the process died without a usable message. The fix was one `min`.

`the_last_block_of_a_non_final_piece_is_four_kibibytes` asserts the numbers
rather than the absence of a panic, because a fixture that can only fail by
panicking tells a reader nothing when it passes:

- `1,986,560 % 16,384 == 4,096`, and `1,986,560 / 16,384 == 121`. So 121 whole
  blocks and a tail, on every piece but the last.
- The tail block of piece 0 starts at `121 * 16384 = 1,982,464` and is 4,096
  bytes.
- Those 4,096 bytes map into **`b.bin`**, not `a.bin`, because piece 0 crossed
  the boundary at 1,500,000 long before its tail. `split_by_file` puts them at
  offset 482,464 in `b.bin`. A reader that clamped a block to the file its
  piece started in would put them 482,464 bytes into the wrong file.
- The final piece is 926,880 bytes, which is short in a **different** way from
  the tail block, so the two short cases are not the same case and neither
  stands in for the other.

**The whole path is exercised too, not just the arithmetic.** The same fixture
runs through a real `librqbit` session and a real ranged HTTP mirror in
`a_torrent_whose_pieces_straddle_every_boundary_downloads_byte_for_byte`, and
through `Fetcher::read` in
`a_block_that_straddles_a_boundary_is_fetched_as_one_request_per_file`. That
covers the two places the entry named: the web seed bridge turning a piece
request into byte ranges, and the storage layer's span mapping.

**`create` refuses this piece length, and that is correct.** The lint
`piece-length-not-power-of-two` (`crates/bit-cli-core/src/torrent/lint.rs`)
fires, so the fixture is built with that one lint allowed. The asymmetry is
deliberate and worth stating: **strict on write, tolerant on read.** BEP 52
requires a power of two at least 16 KiB
(`nanotorrent/src/bittorrent/torrent_create.rs:390`,
`rustorrent/src/torrent.rs:300`) and the v1 convention is the same, so a
torrent `bit-cli` writes should never have an odd piece length. A torrent
somebody else wrote may, BEP 3 permits it, and refusing to read it would be
refusing a legal torrent over a preference. That is the same position
[T-172](#t-172-strictness-on-read-is-undecided-and-the-error-does-not-say)
recommends for the keys around `info`, arrived at independently.

This stays a v1-only hazard after [T-081](create-seed.md), because v1 torrents
do not stop existing.

```
$ cargo test -p bit-cli-core --test webseed_e2e -- the_last_block_of_a_non_final
test the_last_block_of_a_non_final_piece_is_four_kibibytes ... ok
test result: ok. 1 passed; 0 failed
```

