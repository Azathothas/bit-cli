# Windows

Thirty-eight issues touch Windows specifically. Rule 0.3 lists the traps; this
file tracks which are handled and which are not.

---

### T-070 A downloaded executable cannot be run until the process exits

Source:      https://github.com/ikatson/rqbit/issues/369 (open)
Category:    windows
Priority:    P1
Effort:      M
Status:      open

Problem:     A `.exe` inside a completed torrent cannot be launched while the
             session still holds a handle to it, downloading, paused, or
             finished. Windows will not let a file be executed or renamed while
             another process holds it open without sharing.
Relevance:   Rule 0.3 calls this out by name. A one-shot tool that exits after
             the download hides it, but `bit-cli seed` holds handles for the
             whole run by design.
Approach:    Two halves. The finalize half: close every payload handle before
             `download` reports completion, and retry with backoff if a close
             races. The seed half: open with `FILE_SHARE_READ | FILE_SHARE_DELETE`
             so a reader is not locked out. The second needs a storage wrapper,
             because `librqbit` opens the files.
Acceptance:  `bit-cli download <TORRENT WITH EXE>` followed immediately by
             running the executable succeeds, and the same during a concurrent
             `bit-cli seed` of the same payload.

### T-071 Reserved device names in torrent paths are not sanitised

Source:      rule 0.3
Category:    windows
Priority:    P0
Effort:      M
Status:      open

Problem:     A torrent can contain `CON`, `PRN`, `AUX`, `NUL`, `COM1` to
             `COM9`, `LPT1` to `LPT9`, names ending in a dot or a space, and
             the characters `< > : " | ? *`. None can exist on NTFS. Nothing in
             `bit-cli` sanitises them today.
Relevance:   A torrent is untrusted input. This is a correctness bug and a
             small security one: a path that escapes or collides is a file
             written somewhere the caller did not expect.
Approach:    Sanitise on write, record the mapping, and expose it in `--json`
             so a caller can reconcile the names it asked for with the names on
             disk. `bit-cli create` has the same problem from the other side: a
             `create` lint should refuse to build a torrent whose paths will be
             unusable on Windows unless `--allow` says otherwise.
Acceptance:  A fixture torrent containing `CON.txt`, `a<b.bin`, and `x .` is
             downloaded on Windows, every file lands, and `--json` carries a
             `renamed` array mapping each torrent path to its on-disk path.

### T-072 Case-colliding paths silently overwrite

Source:      rule 0.3
Category:    windows
Priority:    P0
Effort:      S
Status:      open

Problem:     NTFS is case insensitive by default. A multi-file torrent
             containing both `README` and `readme` writes one file twice, and
             the second write wins.
Relevance:   Silent data loss on a legal torrent. On Linux the same torrent is
             fine, so it only shows up in production.
Approach:    Detect the collision from the layout before any byte is written,
             which needs no I/O: lower-case every path and look for duplicates.
             Refuse with a clear error naming both paths, and let
             `--allow-overwrite` proceed with a documented rename.
Acceptance:  `bit-cli download <COLLIDING TORRENT>` exits 8 naming both paths,
             and with `--allow-overwrite` both files land under distinct names
             reported in `--json`.

### T-073 Long paths are not tested

Source:      rule 0.3
Category:    windows
Priority:    P1
Effort:      S
Status:      open

Problem:     `git config core.longpaths true` is set, but nothing in `bit-cli`
             tests a payload path past 260 characters, and nothing uses the
             `\?\` prefix.
Relevance:   Torrent payloads routinely exceed 260 characters once the download
             directory is prepended. Without the prefix the write fails with a
             confusing error.
Approach:    Normalise the download directory to an extended-length path on
             Windows before handing it to the session, and add a fixture with a
             deep path.
Acceptance:  A torrent whose deepest path plus the download directory exceeds
             300 characters downloads and verifies on Windows.

### T-074 A false hash-check pass on empty files

Source:      https://github.com/ikatson/rqbit/issues/625 (closed, 2026-08-15)
Category:    windows
Priority:    P1
Effort:      S
Status:      done

Problem:     `FilesystemStorage::pread_exact` ignored the byte count returned
             by `seek_read`, so a read that returned nothing was treated as a
             read of zeroes and the hash check passed over a missing file.
Relevance:   A false verification pass is the worst class of bug in this tool.
Approach:    Verify against the pinned version rather than trusting the closed
             label.
Acceptance:  **Fixed in the pinned 9.0.0.** Verified at
             `storage/filesystem/opened_file.rs:63-74`: the Windows
             `pread_exact` now loops over `seek_read` and returns
             `ErrorKind::UnexpectedEof` when a read returns zero. Checked
             2026-08-19.

### T-075 PowerShell redirection encoding is not documented

Source:      rule 0.3
Category:    windows
Priority:    P2
Effort:      S
Status:      open

Problem:     On Windows PowerShell 5.1, `>` and `Out-File` write UTF-16LE,
             which breaks piping JSON into `jq`. `bit-cli` writes UTF-8 without
             a BOM to stdout regardless of the console code page, but the
             redirection trap is the caller's and needs documenting.
Relevance:   A caller who redirects `--json` to a file and then cannot parse it
             will blame the tool.
Approach:    Document the working invocations in the README: pipe directly to
             `ConvertFrom-Json`, and use `-Encoding utf8NoBOM` when redirecting
             on PowerShell 7.
Acceptance:  The README carries both forms and both have been run.
