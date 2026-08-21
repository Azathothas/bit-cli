# Creating, editing, and seeding torrents

Twenty-five issues touch creation and metainfo; forty-eight touch seeding,
upload, and ratio.

---

### T-080 librqbit's create_torrent writes an extra piece hash

Source:      found here, 2026-08-19, against the pinned `librqbit` 9.0.0
Category:    create
Priority:    P1
Effort:      S
Status:      **done**

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

**Done, as a differential test rather than a fix.** `bit-cli create` uses
`bit_cli_core::torrent::create`, which writes one hash per piece, and
`crates/bit-cli-core/tests/create_alignment.rs` proves both halves over the
same bytes:

- `bit_cli_writes_one_hash_per_piece_for_an_exactly_aligned_payload` builds a
  327,680 byte payload at a 32,768 byte piece length and asserts the metainfo
  carries exactly ten hashes and parses.
- `librqbit_writes_one_hash_too_many_and_bit_cli_refuses_it` runs
  `librqbit::create_torrent` over the same file and asserts `Metainfo::parse`
  refuses the result naming the counts: eleven pieces declared where 327,680
  bytes at 32,768 needs ten.

The second test is the fixture rule 0.10 asks for: the failing input is
generated rather than committed, because generating it is three lines and a
committed `.torrent` says nothing about which version produced it. If
`librqbit` fixes this, that test fails and this entry gets its answer.

**The upstream report is what is left, and it needs the operator.** Filing it
means posting to `github.com/ikatson/rqbit` from an account, which is not
something this session does on its own. The report is written and ready: the
function is `create_torrent_raw` in `librqbit-9.0.0/src/create_torrent_file.rs`,
the condition is the final flush testing `remaining_piece_length > 0 && length
> 0`, and the reproduction is the test above.

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
Status:      **done** for v1, `--private`, and `--web-seed`. The
             `--version hybrid` case waits on T-081.

Problem:     `bit-cli create` then `verify` then `seed` then a download by a
             different client had never been run. Determinism was proven
             against `bit-cli` itself; interoperability was not proven at all.
Relevance:   A torrent nobody else can read is not a torrent. This was the
             single most important untested claim in the tool.
Approach:    `scripts/interop-roundtrip.ps1` runs the whole round trip on
             loopback. Two fixtures make it possible without a network:
             `cargo run -p bit-cli-core --example loopback-tracker`, a BEP 3
             HTTP tracker that lets two clients on one machine find each other
             without the DHT or LSD, and
             `cargo run -p bit-cli-core --example loopback-fileserver`, a
             static server with byte ranges for the web seed case.
Acceptance:  Byte-identical payload from the second client, with the exact
             commands and the resulting hashes recorded here.

Evidence:    Run at 2026-08-19T18:55:16.569Z on Microsoft Windows 10.0.26200,
             against aria2 1.37.0 (`aria2c --version`), a different
             implementation in a different language.

    pwsh -NoProfile -File scripts/interop-roundtrip.ps1 -Keep

             And again at 2026-08-19T22:21:04.696Z against rqbit 9.0.0, a
             third implementation:

    pwsh -NoProfile -File scripts/interop-roundtrip.ps1 -Client rqbit

    CASE      RESULT  INFO HASH                                 BYTES
    v1        pass    a6291a9a2794b3ff158e6db9d9424e6b166ddca7  490012
    private   pass    7240f139d5bbabedba0e2c7522bcafd6b087e8c5  490012
    webseed   skip    rqbit does not implement BEP 19

             The web seed case is skipped for `rqbit` and named in the report's
             `cases_skipped`, never silently dropped. Skipping is correct here:
             the case asks the second client to resolve a `url-list` with no
             peer at all, and a client without BEP 19 cannot, which says
             nothing about `bit-cli`. That absence is the gap this project
             exists to fill.

             The two clients are checked differently at the parse step because
             they print different things. `aria2c -S` prints the info hash, so
             that is asserted. `rqbit download --list` prints the file list and
             not the hash, so the file names are asserted. Agreement on the
             info hash is proven either way by the transfer: the tracker keys
             its swarm on the hash, so a client that computed a different one
             never finds the seeder and the case fails.

             Exit code 0. Payload: 4 files, 490012 bytes, one directory name
             carrying a space (`disc 1/`), 32 KiB pieces, 15 pieces. The
             payload bytes are generated by a fixed LCG in the script, so the
             info hashes below reproduce.

    CASE      RESULT  INFO HASH                                 BYTES
    v1        pass    a6291a9a2794b3ff158e6db9d9424e6b166ddca7  490012
    private   pass    7240f139d5bbabedba0e2c7522bcafd6b087e8c5  490012
    webseed   pass    a6291a9a2794b3ff158e6db9d9424e6b166ddca7  490012

             Per case, in order: `bit-cli create` wrote the `.torrent`,
             `bit-cli verify` reported `complete: true`, `aria2c -S` reported
             the same info hash, and `aria2c` downloaded to a fresh directory.
             Every file matched its source SHA-256 and no extra file appeared.

             `v1` and `private` transferred over BitTorrent from `bit-cli
             seed`. The seeder's own final report accounts for the bytes, so
             the payload is not attributed by inference:

    "uploaded": 490012, "peers_served": 1, "ratio": "1.000"

             The tracker log shows both ends of that swarm, `-rQ9000-`
             announcing `left=0` and `A2-1-37-0-` announcing `left=490012` and
             then `event=stopped` with `left=0`.

             `webseed` had no peer and no tracker at all. `aria2c` resolved the
             `url-list` and fetched the four files over HTTP, and the server
             log shows the BEP 19 composition including the percent-encoded
             space:

    GET /payload/disc%201/a.flac range=bytes=0-299999 -> 206 300000 byte(s)
    GET /payload/disc%201/b.flac range=bytes=0-149999 -> 206 150000 byte(s)
    GET /payload/extras/notes.nfo range=bytes=0-39999 -> 206 40000 byte(s)
    GET /payload/tiny.bin range=bytes=0-11 -> 206 12 byte(s)

             The script asserts the served total covers the payload, so the
             case cannot pass on bytes that came from somewhere else.

             The `v1` and `webseed` info hashes are identical by design:
             `announce` and `url-list` sit outside the info dict, so attaching
             either does not change it. `--private` does change it, because
             `private` is inside.

             The failure path is exercised too. `-TimeoutSeconds 1` fails all
             three cases and exits 1, naming the unmet deadline, the seeder
             that served nobody, and every hash mismatch. A missing client
             exits 2.

Remaining:   1. `--version hybrid` is not covered because it does not exist.
                Tracked by T-081;
                add a fourth case to the script when it lands.
             2. `transmission` cannot join the matrix on Windows.
                `winget install Transmission.Transmission` was run here on
                2026-08-19 and installs version 4.1.3, which ships
                `transmission-qt.exe` and nothing else: no `transmission-cli`,
                no `transmission-remote`, no `transmission-daemon`, no
                `transmission-show`. Verified with
                `find "/c/Program Files/Transmission" -iname "*.exe"`. A GUI
                cannot be driven headlessly, and rule 0.11 makes a
                TTY-dependent test worthless anyway. What would unblock it: the
                Linux side of the `interop` CI job, where
                `apt-get install transmission-cli` gives a real command-line
                client. Tracked as item 4 below.
             3. `ci.yml` carries an `interop` job on Linux and Windows that
                installs `aria2` and runs the script. It has not run: nothing
                is pushed. Same blocker as T-085.
             4. The Linux leg of that job should also install
                `transmission-cli` and run the script a third time with
                `-Client transmission-cli`, which needs a new branch in the
                script's invocation block. Not written yet, because it cannot
                be run here to check it.

### T-085 Creation determinism is not proven across platforms

Source:      PROMPT.md A3.4a, section 3 matrix item 15
Category:    create
Priority:    P1
Effort:      S
Status:      **done**

Problem:     Byte-identical output on repeat runs is tested. Byte-identical
             output between a Windows and a Linux build is not, and path
             separator handling is exactly the bug that catches.
Relevance:   Reproducible builds of a torrent are what let two mirrors publish
             the same info hash independently.
Approach:    A CI job that builds the same fixture on both platforms and
             compares the BLAKE3 of the `.torrent`.
Acceptance:  `ci.yml` carries the job and it passes.

**The job exists and a second, stronger check now runs beside it.**

`ci.yml` carries `determinism`, which builds the same fixture on
`ubuntu-latest` and `windows-latest` and uploads the SHA-256 of the
`.torrent`, and `determinism-compare`, which fails when the two differ. That
is the acceptance, and it holds only for the commit CI ran on.

The stronger check is a constant.
`cmd::create::tests::a_fixture_torrent_hashes_the_same_on_every_platform`
builds the same fixture the job builds and asserts its SHA-1 is
`069804535e172027dfd40388bc0b7a64d8e8770b`. The test suite runs on both
platforms in CI, so both compare against one number rather than against each
other, and a platform added later is checked by the same line. It also fails
locally, where the job cannot run at all.

The fixture is deliberately the job's: two files, one nested, sorted by path,
`--no-creation-date --no-created-by --piece-length 16KiB`, and `--name
fixture` so the temporary directory's own name cannot reach the metainfo. The
last one is what makes the constant stable across runs on one machine as well
as across platforms.

**The run is in.** CI run 32407214253, 2026-08-20:

| job | result |
| --- | --- |
| `Create determinism (ubuntu-latest)` | pass, 38s |
| `Create determinism (windows-latest)` | pass, 1m35s |
| `Compare determinism hashes` | pass, 4s |

https://github.com/Azathothas/bit-cli/actions/runs/32407214253

The compare job is the one that matters: it is what fails when the two
platforms disagree, and it had never been green before because the run it
first appeared in was red for other reasons. The two hashes it compared are
equal, and the same commit's `Test (windows-latest)` asserted the constant, so
the number both platforms produce is also the number written down.
