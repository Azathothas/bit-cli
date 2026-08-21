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
Status:      open

Problem:     `Metainfo::url_list` accepts the BEP 19 `url-list` key as either a
             bencoded list or a bare bencoded string, which is right.
             `Metainfo::http_seeds` at `torrent/metainfo.rs:306` accepts the
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
Status:      open

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
