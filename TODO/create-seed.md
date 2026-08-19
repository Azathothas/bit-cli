# Creating, editing, and seeding torrents

Twenty-five issues touch creation and metainfo; forty-eight touch seeding,
upload, and ratio.

---

### T-080 librqbit's create_torrent writes an extra piece hash

Source:      found here, 2026-08-19, against the pinned `librqbit` 9.0.0
Category:    create
Priority:    P1
Effort:      S
Status:      open

Problem:     `create_torrent` appends one spurious piece hash when the payload
             is an exact multiple of the piece length. Its final flush tests
             `remaining_piece_length > 0 && length > 0`, but
             `remaining_piece_length` has already been reset to a full piece by
             the loop that just closed the last complete piece, and `length` is
             the last file's length rather than the total. So it hashes an
             empty SHA-1 and appends it.

             Reproduced with a 327,680-byte payload at a 32,768-byte piece
             length: 11 hashes where 10 pieces exist. `bit-cli`'s own parser
             rejects the result:
             `torrent declares 11 pieces but 327680 bytes at 32768 bytes per
             piece needs 10`.
Relevance:   `bit-cli create` uses its own creator, so nothing shipped is
             affected. It matters because it was producing the test fixtures,
             and because any `.torrent` from `rqbit` with an exactly-aligned
             payload is malformed and will be rejected by strict clients.
Approach:    Upstream: the final flush should test whether any bytes are
             pending in the current piece, not whether the counter is non-zero.
             Here: fixtures are built with `bit_cli_core::torrent::create`,
             which is the code that ships. Report it upstream.
Acceptance:  A `.torrent` for a payload that is an exact multiple of the piece
             length, built by `bit-cli create`, has exactly
             `total_length / piece_length` hashes. Covered by
             `webseed_e2e::a_bep_17_source_downloads_a_torrent` and friends,
             which use exactly-aligned payloads.

### T-081 BEP 52 v2 and hybrid torrents are not implemented

Source:      https://github.com/ikatson/rqbit/issues/546 (open), PROMPT.md A3.4a
Category:    create
Priority:    P1
Effort:      XL
Status:      open

Problem:     `bit-cli create --version v2|hybrid` returns a usage error naming
             this item. Neither the merkle tree construction nor the BEP 47
             padding files a hybrid torrent needs exist.
Relevance:   v1 is what everything reads today, so this is not urgent, but a
             creation tool that cannot make a hybrid torrent will age badly.
Approach:    `superseedr/src/torrent_manager/merkle.rs` is the reference for
             the tree shape (GPL-3.0, read only). Upstream issue #546 carries a
             full design for `rqbit`. Creation is the tractable half: `bit-cli
             create --version hybrid` needs the v2 `file tree`, the
             `piece layers`, and BEP 47 padding between files. Downloading a v2
             torrent needs `librqbit` support and is a separate, larger item.
Acceptance:  `bit-cli create <PATH> --version hybrid` produces a torrent that
             `intermodal` and one mainline client both accept, and whose v1
             info hash matches a `--version v1` build of the same payload.

### T-082 BEP 16 superseeding is not implemented

Source:      PROMPT.md A3.4b
Category:    seeding
Priority:    P2
Effort:      M
Status:      open

Problem:     `bit-cli seed --superseed` is accepted and warns that it does
             nothing.
Relevance:   Superseeding is what makes initial distribution of a large payload
             from one seed efficient, which is exactly the netdisk case.
Approach:    Superseeding means advertising one piece at a time per peer and
             only advertising the next once the first has been seen elsewhere
             in the swarm. That is picker and bitfield control, which
             `librqbit` does not expose. Same blocker as
             [T-032](performance.md).
Acceptance:  `bit-cli seed --superseed --json` reports, per peer, which single
             piece it was offered and when that changed.

### T-083 Seeding does not report choke state or disconnect reasons

Source:      PROMPT.md A3.4b
Category:    seeding
Priority:    P2
Effort:      M
Status:      open

Problem:     See [T-024](peers.md). The seed report carries bytes, pieces,
             chunks, errors, direction, client, and connect time, and not choke
             history or why a peer left.
Relevance:   A3.4b names both.
Approach:    Blocked on the same upstream stats gap.
Acceptance:  As T-024.

### T-084 The create round trip has not been proven against another client

Source:      PROMPT.md A3.4a, section 3 matrix item 14
Category:    create
Priority:    P0
Effort:      M
Status:      open

Problem:     `bit-cli create` then `verify` then `seed` then a download by a
             different client has never been run. Determinism is proven against
             `bit-cli` itself; interoperability is not proven at all.
Relevance:   A torrent nobody else can read is not a torrent. This is the
             single most important untested claim in the tool.
Approach:    Create a multi-file payload, seed it on loopback, and download it
             with `rqbit`, `aria2c`, or `transmission-cli`. Repeat with
             `--private`, with `--web-seed`, and once `--version hybrid` exists,
             with that.
Acceptance:  Byte-identical payload from the second client, with the exact
             commands and the resulting hashes recorded here.

### T-085 Creation determinism is not proven across platforms

Source:      PROMPT.md A3.4a, section 3 matrix item 15
Category:    create
Priority:    P1
Effort:      S
Status:      open

Problem:     Byte-identical output on repeat runs is tested. Byte-identical
             output between a Windows and a Linux build is not, and path
             separator handling is exactly the bug that catches.
Relevance:   Reproducible builds of a torrent are what let two mirrors publish
             the same info hash independently.
Approach:    A CI job that builds the same fixture on both platforms and
             compares the BLAKE3 of the `.torrent`.
Acceptance:  `ci.yml` carries the job and it passes.
