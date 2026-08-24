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
Status:      **done**, with the recommendation corrected below

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

**Closed 2026-08-21T16:57Z. The recommendation is inverted from what this tree
can support, and the correction goes here rather than over it**, the way
[T-017](disk-io.md) and [T-021](peers.md) established.

**First, what the parser actually did**, measured rather than assumed, because
the entry says "whatever the parser does today it does by accident":

```
unsorted keys at the top level:  Ok(...)      accepted, silently
unsorted keys inside `info`:     Ok(...)      accepted, silently
trailing NUL:                    Err(TrailingData)
trailing whitespace:             Err(TrailingData)
trailing junk:                   Err(TrailingData)
non-canonical integer `i03e`:    Err(NonCanonicalInteger)
```

So the accidental position was the **opposite** of the recommendation on both
questions: tolerant on key order everywhere including inside `info`, and strict
on every trailing byte. Keys go into a `BTreeMap`, which reorders them and keeps
no record that the original did not.

**Second, why strict-inside-`info` is the wrong half to keep.** The entry's
argument is that "anything `bit-cli` accepts there it must be able to re-encode
byte-identically or the info hash moves". `bit-cli` never re-encodes `info`.
`Metainfo::parse` hashes it from its recorded byte span and
`Metainfo::write_to_vec` splices those same bytes back, re-encoding only the
keys around them, and then re-reads its own output and refuses to write if the
hash moved. So the premise the recommendation rests on does not hold here.

The entry half-knows this: it cites
`TorrentNG/crates/rt-metainfo/src/parse.rs:20` and says the span technique "is
what makes strictness survivable at all". It is the other way round. Hashing
from the span is what makes **tolerance** survivable: a reader that re-encoded
would have to be strict or publish a different torrent, and a reader that
splices does not. Being strict on top of the span technique buys nothing and
costs exactly what intermodal
[Issue 454](https://github.com/casey/intermodal/issues/454) is about, refusing a
uTorrent/2210 torrent every other client opens.

**So: tolerant on both questions, and neither is silent.**

- **Keys out of order are read, and recorded.** `bencode::Encoding` carries
  `unsorted_dicts`, the byte offset of each dictionary whose keys arrived out
  of order, and `unsorted_inside_info`, which is the one worth separating:
  outside `info` the keys are re-encoded sorted on the way out anyway, and
  inside it they are not, ever.
- **Trailing whitespace and NUL are read, and counted.** mkbrr's list from
  `torrent/update.go:210`: space, tab, CR, LF, NUL. Anything else after the
  top-level dictionary is still refused, and the error now names the rule:
  "`bit-cli` accepts only whitespace and NUL after the top-level dictionary".
- **Reported.** `bit-cli info` carries `encoding` in `--json` and an `encoding`
  line per deviation in the text output, absent for a canonical torrent. The
  reason to report rather than drop is the one the tolerance argument turns on:
  a tool that **does** re-encode `info` produces a different info hash from the
  same file, and the only way to know that ahead of time is to be told.

**The split between `decode` and `decode_torrent` is the load-bearing part.**
`decode_torrent` is the `.torrent` read path and tolerates. `decode` is the
general one, used for an `info` dictionary on its own and for tracker responses,
and it tolerates nothing: trailing bytes inside something that gets hashed are a
different question from trailing bytes after a file.
`the_general_decoder_tolerates_nothing_after_the_value` pins both halves in one
test.

**What is still refused, and why each is an ambiguity rather than untidiness.**
Duplicate keys: a reader taking the first and a reader taking the last disagree
about what the torrent says while agreeing on its hash. Non-canonical integers,
non-string keys, lengths past the end, truncation. Key **order** is in none of
these categories, because the map is the same whichever order it was written in.

**Proved end to end, and the `edit` test is the one that matters.**
`editing_a_torrent_with_unsorted_keys_keeps_its_info_hash` reads a torrent with
`info` before `announce` and `info`'s own keys out of order, adds a web seed,
and asserts the result: the info hash is unchanged, `info_bytes` are identical
byte for byte, the top-level dictionary comes out **sorted** because everything
outside `info` is re-encoded canonically, and `info` comes out **still
unsorted** because it was spliced. The edited file is canonical everywhere the
hash does not depend on and untouched everywhere it does, which is the whole
design in one assertion.

```
$ cargo test -p bit-cli --lib -- cmd::info editing_a_torrent_with_unsorted
test cmd::info::tests::a_torrent_with_unsorted_keys_and_a_trailing_newline_is_read ... ok
test cmd::info::tests::the_text_output_names_the_rule_that_was_bent ... ok
test cmd::info::tests::a_canonical_torrent_reports_no_encoding_notes ... ok
test cmd::info::tests::junk_after_the_top_level_dictionary_is_still_refused ... ok
test cmd::edit::tests::editing_a_torrent_with_unsorted_keys_keeps_its_info_hash ... ok
```

Thirteen more in `torrent/bencode.rs`, including that one tolerated byte does
not excuse the one after it, that the recorded offset names the dictionary
rather than the key, and the three depth tests below.

`README.md` states the position under "Reading a torrent somebody else wrote",
which is the acceptance's last clause.

**One byte of this made the whole file invisible, 2026-08-22.**
`TOLERATED_TRAILING` was written with the five bytes themselves rather than
with escapes, so `torrent/bencode.rs` carried a raw tab, a raw carriage return,
a raw newline and a **raw NUL** inside a byte-string literal, and two more NULs
in the test that exercises it. A file with a NUL in it is what `grep` calls
binary and skips, so for two sessions no search over `crates/` could see any
line of the largest metainfo file in the tree, and the constant itself rendered
as `b" ` on one line and ` ";` on the next with nothing to read. It carries the
five escapes now, a space and then t, r, n and 0 each behind a backslash, for
the same five bytes. Spelled out rather than quoted, because quoting a string
of escapes through a tool that interprets escapes is what put a NUL in this
paragraph twice while it was being written.

`scripts/gates.ps1` fails on a NUL in any tracked text file, so this cannot
come back quietly. See [RULES.md](RULES.md) section 5.

**One thing the measurement turned up that this entry does not cover.**
Non-canonical integers are refused **everywhere**, `info` and out, and by the
argument above the ones outside `info` could be read the same way key order now
is. It is left alone deliberately: unlike the uTorrent key-sorting case, no
real-world torrent carrying `i03e` is in evidence, and changing a rule with no
instance behind it is how a parser grows tolerance nobody needed. Filed as [T-187](#t-187-non-canonical-integers-are-refused-everywhere-with-no-instance-behind-the-rule)
so it is not rediscovered.

**And one the checklist found that had to be fixed here rather than filed.**
The entry says to turn `rustorrent/docs/DEEP_AUDIT_REPORT_2026-07-13.md`'s
adversarial set into fixtures. Duplicate keys, non-string keys, invalid lengths
and truncation each already had a test. **Excessive depth had neither a test nor
a bound, and the fixture does not fail, it kills the process.** `Parser::value`
recurses and nothing stopped it: 1,000 deep parsed fine and 10,000 deep exited
with `STATUS_STACK_OVERFLOW`, which is not a panic and which `catch_unwind`
cannot see. A `.torrent` fetched from a URL and a tracker's response are both
untrusted input, so that is a denial of service in twenty kilobytes, and leaving
it filed while the module was open was not defensible.

`MAX_DEPTH` is 100, counted in `value` because that is the one place every
nested value passes through and a bound two call sites have to remember is a
bound one of them will forget. A real torrent reaches about six and
`announce-list` reaches three, so nothing legitimate is near it;
`nesting_a_real_torrent_reaches_is_well_inside_the_bound` asserts both ends of
that, and `a_long_flat_list_is_not_deep` asserts a hundred thousand flat entries
still read, because the bound is on nesting and not on size. Excessive **value
counts**, the other half of that checklist line, is bounded already by the
length prefix: every value costs at least two input bytes, so a document cannot
declare more values than it carries.

### T-173 A zero-length path component has no defined meaning

Source:      `reference/RESEARCH.md` section C, 2026-08-21
Category:    metainfo
Priority:    P3
Effort:      S
Status:      **done** 2026-08-24

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

**Measured, and the premise is wrong in both halves.** The entry says nothing
says what an empty component means and the planner has no test for it. Both
are answered now, and neither answer is the one the Approach assumed.

**It has a defined meaning and always did.** `crates/bit-cli-core/src/paths.rs`
drops an empty component and a `.`, and the comment beside the `-O` test says
so. Three shapes measured, all of them landing as if the component were not
there:

```
["", "lead.bin"]        -> lead.bin
["mid", "", "dle.bin"]  -> mid/dle.bin
["trail.bin", ""]       -> trail.bin
```

**The case the entry is actually about is refused, whole, before the planner
runs.** `path: ["", "foo"]` beside `path: ["foo"]` is
`BadTorrentDuplicateFilenames` from `librqbit_core`'s `validate` at
`vendor/rqbit/crates/librqbit_core/src/torrent_metainfo.rs:352`, which joins
every file's components and refuses the torrent when two join to the same
name. So the two entries never reach the collision rule the Approach proposed
using:

```
["/foo.bin", "foo.bin"] -> REFUSED: duplicate filenames in torrent
```

**That refusal stays, and the argument is the one
[T-187](#t-187-non-canonical-integers-are-refused-everywhere-with-no-instance-behind-the-rule)
just used.** parse-torrent
[Issue 89](https://github.com/webtorrent/parse-torrent/issues/89) is a parser's
handling, not a torrent anybody has: nothing in the corpus carries an `info`
with two entries that collapse onto one name. A validation relaxed with no
instance behind it is tolerance nobody asked for, and it would be inconsistent
to keep `i03e` strict on that argument in the same session and relax this one.
[T-072](windows.md)'s precedent does not carry over either: a case collision is
a **filesystem** fact, where the torrent is unambiguous and the disk is not,
and this is a **metainfo** fact, where the torrent itself says two things.

`an_entry_that_collapses_onto_another_is_refused_whole` pins it, so relaxing it
later is a decision somebody makes against a failing test rather than a change
nobody notices.

**What is left open is smaller than the entry and is a seam.** The drop is not
reported. `SafeStorage` plans from `TorrentMetadata::file_infos`, whose
`relative_filename` is a `PathBuf` the vendored session has already built
(`crates/bit-cli-core/src/storage.rs:427`), and `PathBuf::push` drops an empty
component on the way in. By the time this repository's planner sees the path
there is nothing left to drop, so it cannot report what it never saw.

`Reason::DroppedComponent` is built and is reported on the one path where the
raw components do reach the planner, `--index-out`:
`-O 0=/abs/x` lands at `abs/x` and says why. Closing the rest needs one of two
things, and both are larger than this entry:

- a patch to `librqbit_core` so `FileDetails` carries the raw components beside
  the built `PathBuf`, or
- `SafeStorage` planning from this repository's own metainfo parse rather than
  from the session's file list, which is a bigger change than it sounds
  because the session's list is also what the piece-to-file mapping is keyed
  on.

Neither is worth doing for a P3 whose only cost is a missing `reasons` entry on
a path that is already correct. The entry stays open with the seam named, which
is what [RULES.md](RULES.md) section 5 asks for.

**Two tests, in `crates/bit-cli-core/tests/hostile_paths.rs`.**
`an_empty_path_component_lands_as_if_it_were_not_there` asserts the three
shapes and pins the absent report, so a change that starts reporting it fails
there and is read as progress.
`an_entry_that_collapses_onto_another_is_refused_whole` pins the refusal.

```bash
cargo test -p bit-cli-core --test hostile_paths
```

## Done 2026-08-24, on the operator's ruling, and the seam needed no patch

**The ruling.** The drop is to be reported. The wider instruction it came with
is the one that shaped the change: upstream must not be able to alter this
tree's behaviour without something here saying so.

**The seam is closed by planning from a different place, not by patching
`librqbit_core`.** The entry named two ways to close it and expected the first:
a patch so `FileDetails` carries the raw components beside the built
`PathBuf`. It carries them already. `FileIteratorName::to_vec` is public, it
returns the components decoded with the same encoding `to_pathbuf` uses, and
`ValidatedTorrentMetaV1Info::iter_file_details_ext` is the public iterator
`TorrentMetadata::new` builds `file_infos` from in the first place. Nothing had
to be added to the vendored tree, so **`vendor/` is untouched and there is no
new section in `patches/UPSTREAM.md`.** A patch not carried is a patch no
reconciliation has to re-apply.

`SafeStorageFactory::create` plans from those components joined with `/`
instead of from `slash_path(&file.relative_filename)`. One expression, and the
planner it feeds already did the rest: `plan_with` splits on `/`, reports
`Reason::DroppedComponent` when any component is empty, and drops it.

| torrent path | disk path | reported |
| --- | --- | --- |
| `/lead.bin` | `lead.bin` | `DroppedComponent` |
| `mid//dle.bin` | `mid/dle.bin` | `DroppedComponent` |
| `trail.bin/` | `trail.bin` | `DroppedComponent` |

**Where the bytes land does not move**, which is the half that had to hold: the
`disk_paths` assertion in the test is the one that was there before and it is
unchanged. What is new beside it is the reason.

**It also takes the platform out of the answer, which was not the point and is
the better half of it.** `PathBuf::push` treats a backslash as a separator on
Windows and as an ordinary character elsewhere, so a component holding one used
to lay the same torrent out two different ways depending on the target. Joined
with `/` and handed to `plan_with`, it is an illegal character on both, which
is what `sanitize_component` already said it was for.

**The invariant is checked rather than assumed.** `disk_paths` comes from the
list above and `padding` is indexed off `file_infos`, and the piece-to-file
mapping is by index in both, so a pair that disagreed in length would put bytes
in the wrong file rather than fail. `create` bails with both counts and this
entry's id. It is upstream's iterator, and upstream is a tree that moves on
reconciliation.

**Three tests.** `an_empty_path_component_lands_as_if_it_were_not_there` is
inverted: it asserted the report was absent and now asserts what it says, which
is what [RULES.md](RULES.md) section 5 asks of an exemption when the entry it
belonged to closes. `a_path_with_nothing_wrong_with_it_is_reported_as_nothing`
is new and is the guard: this change reaches every torrent rather than only the
hostile ones, and a planner that reported a rename on a path nothing changed
would make `renames.is_empty()` useless to every caller that tests it.
`an_entry_that_collapses_onto_another_is_refused_whole` is unchanged, and the
argument above for keeping that refusal is unchanged with it.

**Run against the defect**: this closed by making an existing test fail. The
change was made first and `an_empty_path_component_lands_as_if_it_were_not_there`
failed naming T-173 and printing all three renames, which is the pin doing
exactly what it was written to do.

```bash
cargo test -p bit-cli-core --test hostile_paths
```

What a reader sees is `renames` in `download --json`, through
`engine.path_plan(&handle)` at `crates/bit-cli/src/cmd/download.rs:2435`, which
is the same `PlanHandle` the test reads. That layer has its own cover in
`an_ordinary_torrent_reports_no_renames`.

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


### T-187 Non-canonical integers are refused everywhere, with no instance behind the rule

Source:      found while measuring [T-172](#t-172-strictness-on-read-is-undecided-and-the-error-does-not-say), 2026-08-21
Category:    metainfo
Priority:    P3
Effort:      S
Status:      **done** 2026-08-23T15:00Z

Problem:     `i03e` and `i-0e` are refused wherever they appear, `info` and
             out, by `NonCanonicalInteger` in
             `crates/bit-cli-core/src/torrent/bencode.rs`. T-172's closing
             established that key **order** can be tolerated because the `info`
             bytes are hashed from their recorded span and never re-encoded.
             The same argument applies to an integer's byte form: outside
             `info` nothing is hashed, and inside it the original bytes are
             what the hash is taken over, so a leading zero cannot move it
             either.
Relevance:   Two things keep this at P3 rather than making it the same defect
             T-172 fixed.

             **No instance is in evidence.** intermodal
             [Issue 454](https://github.com/casey/intermodal/issues/454) is a
             real uTorrent/2210 torrent with unsorted keys, reported by a user
             whose file other clients open. Nothing comparable is recorded for
             non-canonical integers: `rustorrent`'s audit lists them as an
             adversarial case to handle, which is not the same as a torrent in
             the wild carrying one. Relaxing a rule with no instance behind it
             is how a parser grows tolerance nobody needed, and every relaxation
             is a shape a hostile file can take.

             **It is not free the way key order is.** A `BTreeMap` discards key
             order at no cost, so tolerating it required only recording that it
             happened. An integer's byte form would have to be recorded per
             value to be reportable at all, and a report that says "some integer
             somewhere had a leading zero" is not worth the field.
Approach:    Wait for an instance. If one turns up, the shape is T-172's:
             accept, record in `bencode::Encoding`, report in
             `bit-cli info --json`, and keep `decode` strict for the paths where
             the bytes are hashed on their own.

             If none turns up, close this by writing down that the rule is
             deliberate, which is the outcome it most likely has. What must not
             happen is the rule staying unexamined a third time.
Acceptance:  Either a fixture from a real torrent that carries one, read and
             reported the way T-172 reads unsorted keys, or a line in
             `README.md` under "Reading a torrent somebody else wrote" saying
             the rule is deliberate and why.

**Done, and the outcome is the one the Approach said it most likely had: the
rule is deliberate and is written down.** No instance turned up, and nothing
was re-fetched to look for one: [RULES.md](RULES.md) section 7 says not to
re-fetch what `RESEARCH.md` already summarises, and what it summarises is an
adversarial case in `rustorrent`'s audit rather than a torrent anybody has.

**What the examination found is that the reason in the code was wrong.**
`crates/bit-cli-core/src/torrent/bencode.rs` justified the rule with "would
make the info hash ambiguous". It would not. `decode_torrent` records the byte
span of `info` and `Metainfo::from_bytes` hashes **those bytes**
(`crates/bit-cli-core/src/torrent/metainfo.rs:185`), so a leading zero inside
`info` moves the hash exactly as much as an unsorted key does, which is not at
all. That is the same argument [T-172](#t-172-strictness-on-read-is-undecided-and-the-error-does-not-say)
made, and this entry is what noticed it applies here too.

So the comment now carries the two reasons that do hold, both about evidence
rather than correctness: no instance, and a cost that key order did not have.
A `BTreeMap` discards key order for free, so tolerating it needed only a record
that it happened; an integer's byte form would have to be recorded per value to
be reportable, and a report saying "some integer somewhere had a leading zero"
is not worth the field.

**Pinned by a test rather than by prose.**
`a_non_canonical_integer_inside_info_is_refused_too` refuses `i03e` inside
`info` and, on the same fixture written canonically, asserts the recorded span
is the `info` bytes. The second half is what makes the paragraph above checkable
rather than asserted: it is the mechanism that would have made tolerance safe.

```bash
cargo test -p bit-cli-core --lib torrent::bencode
```

`README.md`, under "Reading a torrent somebody else wrote", says the same thing
for a reader who is not going to open the parser, and says what would change
the decision: a torrent in the wild that carries one.

---

### T-241 A resolved magnet keeps the payload and loses the metainfo

Source:      `RESEARCH.md` entry 38's gap table, and a run, 2026-08-24
Category:    metainfo
Priority:    P2
Effort:      S
Status:      open

Problem:     `bit-cli` resolves a magnet to metainfo and never writes it out.
             `man/bit-cli.json` has `bit-cli magnet` taking one positional
             `source` and no `--output`, so the conversion is one way: a
             `.torrent` becomes a magnet URI, and a magnet becomes a report.

             A caller who resolves a magnet keeps the payload and keeps
             nothing that would let them do it again offline. Resolving the
             same magnet a second time means finding peers a second time.

Premise:     **The hard half is already done, and it was measured rather than
             read.** A 2 MiB payload, a torrent carrying no tracker, a magnet
             carrying only `xt`, `dn` and `xl`, one seeder given by address,
             and DHT, LSD and trackers off on both ends:

             ```
             bit-cli download "magnet:?xt=urn:btih:1d02661d..." \
               --peer 127.0.0.1:PORT --no-dht --no-lsd --no-tracker \
               --init-timeout 30s --json
             ```

             Exit 0, 2,097,152 bytes, `finished: true`, and the payload landed
             byte for byte. The metainfo came over BEP 9 from that one peer.

             So this entry is not "implement metadata exchange". It is
             "keep what the exchange produced".

Approach:    `--output <PATH>` on `bit-cli magnet`, writing the resolved
             metainfo as a `.torrent`, and `-` for stdout the way
             `bit-cli create --output` and `bit-cli edit --output` already do.
             Checked against `man/bit-cli.json` on 2026-08-24: `magnet` has no
             `--output` and `-o` is free on that command.

             Two things it must get right, and both are already solved
             elsewhere in this tree:

             - **The info hash must not move.** The bytes written are the
               `info` dictionary exactly as received, hashed from its recorded
               span rather than re-encoded, which is what
               [T-172](metainfo.md) established and what exit 15 protects on
               `bit-cli edit`.
             - **A magnet carries things the info dictionary does not**:
               `tr` trackers, `ws` web seeds, `xs` and `as`. Those belong in
               the top level of the written torrent, not inside `info`, and
               writing them inside would change the hash.

             The reverse direction is worth the same flag for symmetry but is
             not this entry: `bit-cli info <magnet> --json` already prints what
             was resolved.

Prove:       ```
             pwsh -NoProfile -File scripts/check-metalink.ps1
             ```

             is the wrong check. This one needs its own case in the interop
             script, because the property that matters is cross-tool:

             ```
             pwsh -NoProfile -File scripts/interop-roundtrip.ps1
             ```

             A new case: create a torrent, print its magnet, resolve that
             magnet from a loopback seeder with `--output`, and assert the
             written file's info hash equals the original's **and** that
             `aria2c` opens it. A torrent this tree wrote from a magnet that
             another client will not open is the failure worth catching, and
             it is the same discipline [T-084](create-seed.md) closed on.
