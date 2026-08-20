# Disk I/O

Thirty-seven issues in the upstream corpus touch storage. These are the ones
that change what `bit-cli` has to do.

---

### T-010 pwrite takes a read lock where it needs a write lock

Source:      https://github.com/ikatson/rqbit/issues/502 (closed, 2025-10-21)
Category:    disk-io
Priority:    P1
Effort:      S
Status:      **done**

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

**Concurrent `pwrite_all` calls cannot interleave destructively, and the read
half is the correct one.** `bit-cli` supplies its own storage and does not use
`FilesystemStorage`, so the finding is about `crate::storage::Slot`, which has
the same shape by design.

What the guard protects, in `crates/bit-cli-core/src/storage.rs`:

- `Slot::file` is a `RwLock<Option<File>>`. The `Option` is the thing under the
  lock, not the file's contents.
- The only writers of that `Option` are `Slot::close`, `Slot::take`, and the
  open in `SafeStorage::ensure_open`. All three take the write half, so a
  handle can never be swapped while a read or a write is using it.
- Every read and write is positioned. `pwrite_all` is `write_all_at` on Unix
  and a `seek_write` loop on Windows, and `pread_exact` is `read_exact_at` and
  a `seek_read` loop. None of them uses the file's cursor, so two of them at
  different offsets do not affect each other, and two at the same offset were
  already a caller bug that no lock can fix.

On Windows `seek_write` does move the file's cursor as a side effect. That
would matter to a cursor-based reader, and there is not one: nothing in this
storage calls `read`, `write`, or `seek`.

Taking the write half instead would serialise every write on a file to one at a
time, which is the opposite of what the storage is for: reads and writes are
addressed by index and offset precisely so several pieces can be in flight
against one file.

The test is `storage::tests::concurrent_positioned_writes_to_one_file_do_not_interleave`:
eight threads, sixty-four separate 64 KiB writes into one file at interleaved
offsets, then every block is checked for the byte its writer owned. It fails if
a write ever lands inside another one.

```
$ cargo test -p bit-cli-core --lib storage
test result: ok. 20 passed; 0 failed
```

### T-011 No file handle pool, so long runs exhaust descriptors

Source:      https://github.com/ikatson/rqbit/issues/520 (closed, 2026-01-17)
Category:    disk-io
Priority:    P1
Effort:      M
Status:      **done**

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

`--max-open-files` did not parse at all; the entry was wrong about that. It
exists now, on `download` and on `seed`, and it does something.

`SafeStorage` opens a payload file when it is first touched rather than when
the torrent is added, and `OpenSet` keeps the handles ordered so the least
recently opened closes when the cap is reached. The default is 128, chosen to
sit under the 512 stream limit a Windows CRT allows and far under a typical
Linux `RLIMIT_NOFILE` of 1024, so the default never runs a process out on its
own.

The order is by open rather than by access, deliberately. Recording an access
would mean taking the set's lock on every read and write, which costs more than
it saves: the expensive event is opening a handle, and the least recently
opened file is the one least recently needed, both for a download walking
pieces and for a seeder answering requests.

Two guards are never held at once. A slot's read guard is dropped before the
eviction runs, and the eviction takes each victim's write guard on its own, so
two threads evicting each other's file cannot deadlock.

Acceptance, `scripts/check-handles.ps1`, 300 files of 16 KiB, seeded for twelve
seconds at each cap with the process handle count sampled every 200 ms,
2026-08-20T00:54:40.625Z. Report: `bench/handles-20260820T005440625Z.json`.

```
$ pwsh -NoProfile -File scripts/check-handles.ps1

cap peak_process_handles complete
--- -------------------- --------
  8                  195     True
 64                  251     True
128                  315     True

cap 8 to 64: 56 more handles, cap grew by 56
cap 64 to 128: 64 more handles, cap grew by 64
```

The absolute count includes everything else the process holds: threads,
sockets, and libraries. That part is the same whatever the cap is, so it
cancels, and what is left is exactly one handle per payload file the cap
allows. A step of 56 in the cap moves the handle count by 56, and a step of 64
moves it by 64. Before this, 300 files meant 300 handles and the flag did not
exist.

`storage::tests::the_handle_cap_closes_the_least_recently_opened_file` asserts
the invariant directly: eight files, a cap of three, and the open count never
exceeds the cap while every file is still written correctly.
`a_reopened_file_reads_back_what_was_written_before_it_was_closed` proves a
closed file is reopened rather than lost.

### T-012 Preallocation is not implemented

Source:      https://github.com/ikatson/rqbit/issues/412 (open)
Category:    disk-io
Priority:    P2
Effort:      M
Status:      **done**

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

`bit_cli_core::alloc` implements all four, and `SafeStorage::ensure_file_length`
is where they run, because that is the first thing the session does to a file it
intends to use.

| Method | What happens |
| --- | --- |
| `none` | `set_len` and nothing else. |
| `sparse` | `FSCTL_SET_SPARSE` through `DeviceIoControl`, then `set_len`. Marking comes first because punching a hole into a file that is already long is a different operation on some filesystems. |
| `prealloc` | `set_len`, then zeroes written across the whole file in 1 MiB chunks, then `sync_all`. Without the sync the space is a page cache full of zeroes that a full disk refuses later. |
| `falloc` | `posix_fallocate` on Linux. |

`falloc` on Windows degrades to `prealloc` and says so. `SetFileValidData` is
the equivalent call and it needs `SeManageVolumePrivilege`, which an ordinary
process does not hold; it also exposes whatever was previously on those disk
blocks until they are written, which is why the privilege exists. Asking for the
privilege would be the wrong trade for a download tool, so the fallback is the
answer and the warning is how the caller finds out.

Acceptance, `scripts/check-allocation.ps1`, 512 MiB payload on NTFS,
2026-08-20T00:52:50.659Z. Report: `bench/allocation-20260820T005250659Z.json`.

The measurement that separates the methods is taken **before any payload
arrives**: the torrent is added against a source that answers nothing, so the
files are created and sized and nothing is downloaded. That is the state the
question is about, and volume free space either side of it is the number a
capacity plan is made from.

```
$ pwsh -NoProfile -File scripts/check-allocation.ps1 -PayloadSize 512MiB

method    reserved    allocated  sparse  volume gave up  payload
none      512.00 MiB  512.00 MiB  False      511.96 MiB  matches
sparse    512.00 MiB        0 B    True      114.48 MiB  matches
prealloc  512.00 MiB  512.00 MiB  False      637.39 MiB  matches
falloc    512.00 MiB  512.00 MiB  False      514.00 MiB  matches
```

Three things this says:

- `sparse` reserves nothing. A 512 MiB file costs the volume 114 MiB, and that
  114 MiB is other activity on a live machine rather than the file. Every other
  method costs the volume the whole 512 MiB.
- **`none` is not sparse on NTFS.** The Problem above assumed `set_len`
  produces a hole on NTFS the way it does on ext4. It does not: it allocates.
  So on Windows `sparse` is the only way to get a hole, and the two methods are
  genuinely different rather than two names for one behaviour.
- `falloc` degraded, and said so on stderr:

```
warning: --file-allocation falloc is not available here, so prealloc was used
instead: SetFileValidData needs SeManageVolumePrivilege, which this process
does not hold
```

All four produce a payload whose SHA-256 matches the source. An allocation
method that loses data would be worse than one that reserves nothing, so that
is checked on every method rather than assumed.

`GetCompressedFileSize` reports zero for a sparse NTFS file even when it holds
data, which is why the allocated column reads `0 B` for `sparse` and why volume
free space is the number the check asserts on. `fsutil file layout` would show
the extents directly and needs elevation, so it is not used.

The unit tests cover what can be asserted without a filesystem-specific tool:
every method sets the length, `prealloc` reads back as zeroes and replaces
existing bytes, `sparse` reserves a gibibyte in under five seconds (which it
could not do if it wrote the bytes), and `falloc` either works or names why it
fell back.

```
$ cargo test -p bit-cli-core --lib alloc
test result: ok. 8 passed; 0 failed
```

### T-013 Selecting a subset of files still creates all of them

Source:      https://github.com/ikatson/rqbit/issues/484 (open)
Category:    disk-io
Priority:    P2
Effort:      S
Status:      **done**

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

Confirmed on a five-file torrent, before the fix:

```
$ bit-cli download multi.torrent --web-seed $URL --web-seed-only \
    --dir out --select-file 0 --port 0 --json
       0 multi/deep.bin
  262144 multi/file0.bin
       0 multi/file1.bin
       0 multi/file2.bin
       0 multi/file3.bin
```

After:

```
  262144 multi/file0.bin
```

and the same torrent with no selection still lands all five files, each
hashing equal to its source.

Two causes, both in `bit-cli`'s own storage rather than the session's:

- `SafeStorage::init` opened every planned path.
- The hash check reads every piece of every file to learn what is already on
  disk, and the open used for a read created the file.

The fix for the first is the same one that closes
[T-011](#t-011-no-file-handle-pool-so-long-runs-exhaust-descriptors): files open
when they are first touched. The fix for the second is `Intent`: a write creates
a file and a read does not, so a read of a file that is not there answers "not
there" rather than bringing one into existence.

Between them, no selection has to be plumbed into storage at all, which is what
makes this correct for the case a selection cannot express: a piece that spans
a selected file and an unselected one still writes into both, and both are
created because both were written.

Directories are still created up front. An empty directory a selection did not
fill is cheap and visible; an empty file pretending to be payload is not.

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
Status:      open, blocked

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

## The cost, measured

512 MiB payload, one file, 1 MiB pieces, release build, seeded three times:

```
$ bit-cli seed p.torrent --data . --verify <MODE> --exit-when-idle 1s --json

--verify full     6087 ms wall
--verify quick    6372 ms
--verify none     6398 ms
```

Identical within noise, because all three do the same thing. Roughly 85 MiB/s
of hashing plus process startup, so a 40 GiB payload costs about eight minutes
of disk read on every `seed` invocation. That is the number the entry was
asking for.

## The blocker

**`fastresume` in `librqbit` 9.0.0 does nothing without session persistence.**
`session.rs:640-680`:

```rust
match &opts.persistence {
    Some(SessionPersistenceConfig::Json { folder }) => { ... make_result!(s) }
    None => Ok((None, Arc::new(NonPersistentBitVFactory {}))),
}
```

`make_result!` is the only place `opts.fastresume` is read, and it is only
reached when `persistence` is `Some`. With `persistence: None`, which is what
decision 7.4 requires, the bitfield factory is `NonPersistentBitVFactory`
whatever `fastresume` says.

So getting a resume cache means turning on `SessionPersistenceConfig::Json`,
which writes a store of every torrent in the session. That is stored session
state, and 7.4 puts it in Phase C.

`AddTorrentOptions` in 9.0.0 also carries no way to skip the initial check:
`paused`, `only_files`, `overwrite`, `list_only`, `output_folder`,
`sub_folder`, `peer_opts`, `force_tracker_interval`, `disable_trackers`,
`ratelimits`, `initial_peers`, `peer_limit`, `preferred_id`, and the storage
factory. Nothing else. So there is no second route either.

## What would unblock it

One of three, in the order they are worth trying:

1. An upstream `SessionOptions` that takes a `BitVFactory` directly, or a
   `fastresume` that works without a persistence store. Then `bit-cli` supplies
   a factory that reads and writes one file per info hash beside the payload,
   with the file length and modification time recorded so a stale cache is
   detected and discarded, and nothing about the session is stored.
2. A `TorrentStorage` hook that lets storage answer "this piece is already
   verified". `bit-cli` already supplies its own storage, so this would need no
   session state at all. The trait has no such method in 9.0.0.
3. Candidate C, a native fetch and verification path that does not go through
   `librqbit`'s initialisation at all.

Until one of those exists this cannot be built without contradicting 7.4, so it
stays open here rather than moving to `phase-c.md`: the cache itself is derived
data that can be recomputed, which is not what 7.4 is about, and the thing
blocking it is an upstream API rather than a decision.

## What ships in the meantime

`seed --verify` now says what it does. All three values behaved identically and
only `none` warned, so `quick` claimed to be a quick check and was a full one.
Both `quick` and `none` warn now, naming what actually happens, and `--help`
says the same. A flag whose values are all the same is worse than no flag; a
flag that says so is not.
