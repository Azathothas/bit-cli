# Disk I/O

Thirty-seven issues in the upstream corpus touch storage. These are the ones
that change what `bit-cli` has to do.

---

### T-010 pwrite takes a read lock where it needs a write lock

Source:      https://github.com/ikatson/rqbit/issues/502 (closed, 2025-10-21)
Category:    disk-io
Priority:    P1
Effort:      S
Status:      open

Problem:     `FilesystemStorage::pwrite_all` and `pwrite_all_vectored` take
             `lock_read()` on the opened file. The lock exists to keep other
             threads out while the file handle is being swapped; taking the
             read half lets two writers proceed at once.
Relevance:   **Not fixed in the pinned 9.0.0.** Verified in
             `storage/filesystem/fs.rs:69-88`: both writers still call
             `lock_read()` on non-Windows. `lock_write` exists in
             `opened_file.rs:169` and is marked `#[allow(dead_code)]`, so it is
             defined and unused. On Windows the path goes through
             `try_mark_sparse()` instead, which also returns a read guard.
Approach:    The guard protects an `Option<File>` swap, not the file contents,
             and `pwrite` at an offset is safe against itself at a different
             offset. So this may be benign in practice. Establish which it is
             before doing anything: read what the lock protects, then either
             switch to `lock_write` or record why the read half is correct.
Acceptance:  This entry states, with the line numbers, whether concurrent
             `pwrite_all` calls through one `OpenedFile` can interleave
             destructively. If they can, `bit-cli` carries a storage wrapper
             that serialises them and a test that fails without it.

### T-011 No file handle pool, so long runs exhaust descriptors

Source:      https://github.com/ikatson/rqbit/issues/520 (closed, 2026-01-17)
Category:    disk-io
Priority:    P1
Effort:      M
Status:      open

Problem:     `FilesystemStorage` opens one handle per file and keeps it. A
             reporter measured 5,194 open handles and 8.6 GB RSS after six and
             a half days.
Relevance:   The netdisk deployment seeds many torrents from one process. A
             torrent with 20,000 files is one torrent; ten of them is 200,000
             handles.
Approach:    `--max-open-files` parses today and does nothing. Implement it as
             an LRU over opened handles in a storage wrapper, closing the least
             recently used file when the cap is reached. Measure RSS and handle
             count before and after over a long seed.
Acceptance:  `bit-cli seed <TORRENT> --max-open-files 64` on a torrent with
             more than 64 files keeps the process below 64 payload handles for
             the whole run, measured with `Get-Process | Select-Object
             HandleCount` on Windows and `/proc/<pid>/fd` on Linux.

### T-012 Preallocation is not implemented

Source:      https://github.com/ikatson/rqbit/issues/412 (open)
Category:    disk-io
Priority:    P2
Effort:      M
Status:      open

Problem:     `--file-allocation none|prealloc|sparse|falloc` parses and is
             carried through the config, but nothing acts on it. `librqbit`
             calls `set_len` and relies on the filesystem, which produces a
             sparse file on NTFS and ext4 and a fully allocated one elsewhere.
Relevance:   Rule 0.3 requires an explicit allocation strategy. On a netdisk
             the difference between sparse and preallocated is whether a
             half-finished 40 GB torrent shows as 40 GB of committed space.
Approach:    Four real behaviours: `none` writes nothing up front, `sparse`
             marks the file sparse (`FSCTL_SET_SPARSE` on Windows, the default
             on ext4), `prealloc` writes zeroes, `falloc` calls
             `posix_fallocate` on Linux and `SetFileValidData` on Windows.
             Windows `SetFileValidData` needs `SeManageVolumePrivilege`, so it
             has to degrade to `prealloc` with a warning rather than fail.
Acceptance:  For each method, `bit-cli download --file-allocation <M>
             --dry-run=false` on a 1 GiB torrent, then the on-disk size
             reported by `fsutil file layout` on Windows and `du --apparent-size`
             against `du` on Linux, both recorded here.

### T-013 Selecting a subset of files still creates all of them

Source:      https://github.com/ikatson/rqbit/issues/484 (open)
Category:    disk-io
Priority:    P2
Effort:      S
Status:      open

Problem:     Adding a torrent with `only_files` set creates every path in the
             torrent, not only the selected ones.
Relevance:   `--select-file` is how a caller pulls one ISO out of a
             twelve-image torrent. Creating the other eleven as empty files is
             surprising and, on a filesystem without sparse support, expensive.
Approach:    Confirm against the pinned 9.0.0 first. If it still creates them,
             either delete the unselected files after initialisation or supply
             a storage factory that refuses to create them.
Acceptance:  `bit-cli download <MULTI> --select-file 0 --json` finishes with
             only the selected file present under `--dir`, and the JSON lists
             the skipped paths.

### T-014 Adding a torrent can fail with "File exists (os error 17)"

Source:      https://github.com/ikatson/rqbit/issues/504 (open)
Category:    disk-io
Priority:    P2
Effort:      S
Status:      open

Problem:     Adding a torrent fails outright when the session's own cache files
             already exist.
Relevance:   `bit-cli` runs with persistence off, so its exposure is smaller,
             but the same class of failure reaches `add` through `overwrite`.
Approach:    `bit-cli` maps this to `ExitCode::Disk` in
             `engine::classify_add_error` by matching "os error 17" in the
             error chain. That is text matching and it is fragile. Replace it
             with a real classification once `librqbit` exposes a typed error,
             and meanwhile add a test that pins the string.
Acceptance:  A test adds a torrent over an existing conflicting path and
             asserts exit code 8, not exit code 1.

### T-015 Hash checking can hang at 0 percent

Source:      https://github.com/ikatson/rqbit/issues/347 (open)
Category:    disk-io
Priority:    P1
Effort:      M
Status:      open

Problem:     Roughly one add in twenty of a torrent with existing files sticks
             at 0 percent or 100 percent "checking files" and never leaves.
             Removing and re-adding sometimes clears it.
Relevance:   `bit-cli download` and `bit-cli seed` both wait on
             `wait_until_initialized`. A hang there is a hang with no output
             and no exit.
Approach:    `--timeout` and `--stop-after` already bound the whole run, so a
             hang is survivable today, but the run reports a deadline rather
             than the real cause. Add an initialisation-specific deadline that
             names the hash check, and reproduce the hang with a torrent whose
             files are on a slow or contended volume.
Acceptance:  `bit-cli download <TORRENT> --timeout 30s` against a stuck hash
             check exits 9 with `"phase": "initializing"` in the error context.

### T-016 fastresume is not used when adding a torrent

Source:      https://github.com/ikatson/rqbit/issues/349 (open)
Category:    disk-io
Priority:    P2
Effort:      M
Status:      open

Problem:     A cached bitfield at `.cache/rqbit/{infohash}.bitv` is not read
             when a torrent is added, so every add re-hashes the whole payload.
Relevance:   Re-hashing a 40 GB payload to seed it costs minutes of disk read
             every invocation. For a foreground one-shot tool that is the
             difference between usable and not.
Approach:    `SessionOptions::fastresume` exists in 9.0.0 and `bit-cli` leaves
             it off, because a stored bitfield is state that outlives the
             process and decision 7.4 puts stored session state in Phase C.
             The distinction worth making: a resume cache is derived data that
             can be recomputed, not session state. Decide explicitly whether
             `--fastresume` is in scope for Phase B, and if so where the cache
             lives and how it is invalidated.
Acceptance:  Either a `--fastresume` flag with a documented cache location and
             a test that a stale cache is detected and discarded, or an entry
             in `phase-c.md` saying why not.
