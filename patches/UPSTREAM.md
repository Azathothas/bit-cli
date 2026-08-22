# What this repository changed in the vendored upstreams

One section per change. This file is the record Apache-2.0 asks for, which is
that changed files are marked as changed, and it is what a reviewer reads
instead of a 389 file diff. The patch series beside it is generated from the
tree by `scripts/vendor-diff.ps1`; this is the part a script cannot write.

How to add one: [`README.md`](README.md).

Verify that this file describes the tree:

```bash
pwsh -NoProfile -File scripts/vendor-diff.ps1 -Check
```

---

## librqbit-dualstack-sockets: the Windows `new_from_name` ignores its argument

```
Unblocks:    nothing, and that is why it is first
Files:       vendor/librqbit-dualstack-sockets/src/bind_device.rs
             patches/librqbit-dualstack-sockets/0001-src-bind_device.rs.patch
Upstream:    not offered yet, and it should be
Added:       2026-08-22T12:24Z
```

`BindDevice::new_from_name` has two bodies. The `#[cfg(not(windows))]` one
resolves the interface name to an index. The `#[cfg(windows)]` one returns
`Error::BindDeviceNotSupported` without looking at its argument, and takes that
argument as `name`, so rustc's `unused_variables` fires on every Windows build.
The parameter is now `_name`, which is the whole change: the signature, the
behaviour and the public API are identical.

**Why it has to be here.** Because this repository ships the crate. Cargo passes
`--cap-lints allow` to a dependency it resolved from a registry and does **not**
pass it to a path dependency, so `[patch.crates-io]` made every warning in the
vendored trees ours. Under CI's `RUSTFLAGS: -D warnings` this one failed four
Windows jobs on the vendoring commit.

Dropping `-D warnings` was tried first and reverted on the operator's
instruction, and the reason is worth keeping: development happens on Windows,
so CI is the only place a warning on another platform is ever seen. A build that
does not fail on one cannot catch sloppy work. The cost of that decision is
exactly this file.

**How it was proved.** The build is clean with the flag on, where before it was
not:

```bash
RUSTFLAGS="-D warnings" cargo build --workspace --all-features
```

**Offer it upstream.** It is a one-word fix to a real lint in their code, with
no behaviour attached, which is the easiest kind of change for a maintainer to
take. Until it is offered, this section says so rather than claiming otherwise.

---

## The template

Copy this for the next change.

```
## <upstream>: <what it is>

Unblocks:    T-NNN, and the line in TODO/<file>.md that names the seam
Files:       vendor/<upstream>/<path>, and the patch that carries it
Upstream:    not offered | offered, <url> | landed in <ref>, delete this
Added:       <ISO 8601 UTC>

What it does, in a paragraph a reviewer can check against the diff.

Why it cannot be done outside the vendored tree. This is the part that dates
fastest: a seam that was private may become public, and then the patch should
go rather than stay.

How it was measured, or which test holds it.
```

Three things that section must always answer, because each has already cost
somebody a session in this repository:

- **Which entry.** A vendored change with no `TODO/` entry behind it cannot be
  reviewed against anything, and it is the first thing a reconciliation has to
  weigh when upstream touches the same lines. The one above has none, and says
  so: it exists because of a build flag, not because of a defect in `bit-cli`.
- **Whether it is offered upstream.** A change shaped for upstream and a change
  shaped for this repository are different changes. Deciding which one it is
  afterwards means writing it twice.
- **Why it has to be here.** `TODO/RULES.md` section 5 has a rule about a doc
  describing a state the tree is not in, and a patch justified by a seam that
  has since opened is exactly that.

---

## librqbit: a bitfield larger than MAX_MSG_LEN cannot be sent

```
Unblocks:    T-194, TODO/peers.md, and it is a P0
Files:       vendor/rqbit/crates/peer_binary_protocol/src/lib.rs
             vendor/rqbit/crates/librqbit/src/peer_connection.rs
             vendor/rqbit/crates/librqbit/src/peer_info_reader/mod.rs
             vendor/rqbit/crates/librqbit/src/torrent_state/live/mod.rs
             patches/rqbit/0003-crates-librqbit-src-peer_connection.rs.patch
             patches/rqbit/0004-crates-librqbit-src-peer_info_reader-mod.rs.patch
             patches/rqbit/0006-crates-librqbit-src-torrent_state-live-mod.rs.patch
             patches/rqbit/0008-crates-peer_binary_protocol-src-lib.rs.patch
Upstream:    not offered yet, and it should be
Added:       2026-08-22T13:52Z
```

Every peer message is serialized into one fixed buffer per connection, sized
`MAX_MSG_LEN` = 16,500 bytes. That number is built for a `ut_metadata` data
message: a 16,384 byte chunk, its bencode header, and 64 bytes of slack. The
comment above it says the `ut_metadata` request is "the largest known message".

A bitfield is not bounded that way. It carries one bit per piece, so its length
is a property of the torrent: `5 + ceil(pieces / 8)` bytes. Past **131,960
pieces** it does not fit, `Message::serialize` returns `NoSpaceInBuffer`, and
`manage_peer` drops the connection with "not enough space in buffer". A seeder
then serves nobody and a magnet for such a torrent never resolves, in both
cases with nothing above DEBUG to say why.

The change is that the bitfield no longer goes through the shared buffer:

- `Message::bitfield_message_len` is the exact length `serialize` needs, so a
  caller can size a buffer without knowing the wire layout.
- `PeerConnectionHandler::serialize_bitfield_message_to_buf` takes a
  `&mut Vec<u8>` rather than a `&mut [u8]`. Only the implementor knows the
  piece count, so that is where the buffer is sized.
- The send site in `manage_peer` uses a buffer of its own rather than
  `write_buf`. It is allocated once per connection, written once, and dropped.
- The comment on `MAX_MSG_LEN` now says what it actually bounds.

`peer_info_reader`'s handler is the other implementor and returns `Ok(0)`
without touching the buffer, so it takes the signature and nothing else.

**Why it has to be here.** The buffer, the trait and both implementors are
private to `librqbit`, and the constant is in `peer_binary_protocol`. Nothing
about it is reachable from a dependent crate: there is no option, no builder
and no trait a caller can implement differently. `bit-cli` cannot seed or fetch
a torrent past the threshold by any configuration.

**How it was measured.** Torrents of 1 KiB pieces, seeded on loopback with
trackers and DHT off, fetched by magnet by a second process given nothing but
`--peer 127.0.0.1:<port>`. The pass is metadata resolving and the file
appearing, both of which need the bitfield to have crossed.

| pieces | `.torrent` | bitfield | before | after |
| --- | --- | --- | --- | --- |
| 131,960 | 2,639,339 B | 16,500 B | resolves | resolves |
| 131,961 | 2,639,359 B | 16,501 B | no space in buffer | resolves |
| 163,840 | 3,276,939 B | 20,485 B | no space in buffer | resolves |

One piece apart, and 16,500 is `MAX_MSG_LEN` exactly.

```bash
pwsh -NoProfile -File scripts/check-bitfield.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

139 upstream tests pass, one of them new:
`test_bitfield_larger_than_max_msg_len` in `peer_binary_protocol`, which asserts
that the fixed buffer refuses a 131,961 piece bitfield and a sized one round
trips it.

**What it does not fix.** The read side. `ReadBuf` is a 32,768 byte ring
buffer, so the same message fails on receipt past **262,104 pieces** with "read
buffer is full". Both halves now cap at the same place, twice as far out as
before. [`TODO/peers.md`](../TODO/peers.md) T-195 is that limit, open, with the
measurement.

**Offer it upstream.** It is a defect in their code with a one line reproduction
and no behaviour attached beyond the message getting sent. Until it is offered
this section says so rather than claiming otherwise.

---

## rqbit: the workspace lists a member this repository does not vendor

```
Unblocks:    nothing. It makes a documented command run at all
Files:       vendor/rqbit/Cargo.toml, and the two lockfiles that follow it
             vendor/rqbit/Cargo.lock
             vendor/rqbit/package-lock.json
             patches/rqbit/0001-Cargo.lock.patch
             patches/rqbit/0002-Cargo.toml.patch
             patches/rqbit/0009-package-lock.json.patch
Upstream:    never. This is a consequence of our exclusion list, not their bug
Added:       2026-08-22T13:47Z
```

`vendor/rqbit/Cargo.toml` lists `desktop/src-tauri` in `[workspace] members`.
`desktop/` is deliberately not vendored: it is a Tauri application, 1.6 MB of
the 4.4 MB upstream ships, depending on nothing this tree builds.
[`vendor/upstream.json`](../vendor/upstream.json) carries the exclusion and
[`docs/vendoring.md`](../docs/vendoring.md) carries the reason.

Cargo loads every member's manifest before it does anything, `default-members`
included, so the missing directory made the whole workspace unloadable:

```
error: failed to load manifest for workspace member
`vendor\rqbit\desktop/src-tauri`
```

That is the manifest [`README.md`](README.md) and
[`docs/vendoring.md`](../docs/vendoring.md) both tell a session to run
upstream's tests against, and the kickoff prompt names it too. The command had
never worked. The member is removed rather than the directory vendored, because
vendoring a Tauri desktop application to satisfy a manifest is the wrong half of
the trade.

`default-members` already excluded it, so nothing that used to build stopped.

**The two lockfiles follow from it and are most of the diff.** Removing the
member removed its dependencies: `Cargo.lock` loses 220 packages, 739 down to
519, and `package-lock.json` loses the `desktop` workspace entry. Neither gains
anything. Every `name`/`version` pair left in `Cargo.lock` is one the base
already had, so nothing was added and nothing was bumped; the 55 added lines are
the diff re-flowing around removed blocks. They are committed rather than
reverted because they are stable: cargo and npm rewrite them to exactly this on
the next run, and a reverted lockfile would make `vendor-status` report the
series stale every time somebody ran upstream's tests.

**Why it has to be here.** It is the vendored manifest. There is nowhere else
to say it.

**How it was proved.** The command in `README.md` runs:

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

139 passed, 0 failed, 6 ignored, across thirteen crates.

---

## librqbit: one failed handshake check stops the accept loop draining

```
Unblocks:    T-020, TODO/peers.md, the record's only open P0
Files:       vendor/rqbit/crates/librqbit/src/session.rs
             patches/rqbit/0005-crates-librqbit-src-session.rs.patch
Upstream:    not offered yet, and it should be
Added:       2026-08-22T14:37Z
```

`task_listener` is a `tokio::select!` over two arms: accept a connection, or
take a finished handshake check off `futs`. The second arm was

```rust
Some(Ok((live, checked))) = futs.next(), if !futs.is_empty() => { ... }
```

A `select!` arm whose **pattern fails** is disabled for the rest of that
`select!` call. A handshake check that resolves to `Err`, which is what a peer
that connects and closes without handshaking produces, fails `Some(Ok(..))`.
The arm goes away and the loop waits on `l.accept()` alone, which on an idle
seeder is forever. Nothing in `futs` is polled until the next connection
arrives, so the queue drained **one entry per accepted connection**.

A check that resolves to `Ok` matched, ended the iteration, and cost no accept,
which is why this was invisible under ordinary traffic and why a busy seeder
cleared itself.

The arm now matches `Some(result)` and handles the error inside, so the pattern
cannot fail. The error was already logged by the `map_err` on the future where
it is pushed, so the `Err` case does nothing but let the loop come round.

**What it cost, which is more than a socket count.** While the queue was
backed up the seeder accepted TCP and completed **no handshake for any info
hash, including one it was serving**, and went on reporting itself as seeding.
A supervisor watching the process, the port or the log saw nothing. T-020
measured it one for one: twenty connections that closed without handshaking,
then single peers one at a time, and the twentieth peer got a handshake while
the nineteen before it got nothing.

**Why it has to be here.** `task_listener` is a private method on `Session`
and the `select!` is inside it. There is no option, no callback and no trait
between a caller and that loop. `bit-cli` already sets
`max_pending_incoming_handshake_checks` to `usize::MAX`, which removed a panic
that was the same entry's first defect, and T-020 records that the cap has
nothing to do with this one: "a reader who fixed the cap would have fixed
nothing."

**How it was measured.**

```bash
pwsh -NoProfile -File scripts/check-listener.ps1
```

| case | before | after |
| --- | --- | --- |
| `recovery`, connections to clear a 20 connection backlog | 20 | **1** |
| the same load, probes / failed | 6 / 3 | **13 / 0** |
| the seeder | stopped, exit 17, `listener_unhealthy` | still serving |

Three of that script's four cases asserted the defect, so they are inverted
rather than deleted and now hold the fix. `check-swarm.ps1`'s
`listener_poisoned` case carried `judged: false` for as long as T-020 was open,
and is judged now.

**Offer it upstream.** It is [rqbit#311](https://github.com/ikatson/rqbit/issues/311),
open since before this repository existed, and the change is one match arm.

---

## librqbit: nothing ever reclaimed a peer row

```
Unblocks:    T-040, TODO/memory.md, the record's other P0
Files:       vendor/rqbit/crates/librqbit/src/torrent_state/live/peers/mod.rs
             vendor/rqbit/crates/librqbit/src/torrent_state/live/mod.rs
             patches/rqbit/0006-crates-librqbit-src-torrent_state-live-mod.rs.patch
             patches/rqbit/0007-crates-librqbit-src-torrent_state-live-peers-mod.rs.patch
Upstream:    not offered yet, and it should be
Added:       2026-08-22T15:30Z
```

`PeerStates::states` is a `DashMap<SocketAddr, Peer>` that only ever grew. A
peer that hands over cleanly ends in `PeerState::NotNeeded` and stays there;
`drop_peer` was called on exactly two paths, a bug branch and backoff
exhaustion. T-040 measured it: **one row per completed handshake, exactly**,
2,000 connections leaving 2,000 rows, `live` and `dead` both zero, and a minute
of silence returning none of the memory. A row is 2,891 to 4,281 bytes
depending on the range fitted, which is most of a long seeder's memory slope.

The change is a bound:

- `MAX_PEER_RECORDS`, 1,024 per torrent, well above any real swarm's live peer
  count, so a working torrent never reaches it and this only reclaims history.
  Zero disables it, which is the previous behaviour.
- `PeerStates::reclaim_records` takes rows in `NotNeeded` or `Dead` and never
  `Live`, `Connecting` or `Queued`, because those have a task or a dial behind
  them. It counts what it may take rather than evicting by age.
- Called **before** an insert on both paths that add a row, and never while an
  `Entry` on the same map is held: `DashMap` locks per shard and a second guard
  on the same shard deadlocks.
- Reclaimed rows go through `Peer::destroy`, which decrements both the
  per-torrent and the session counters and clears `live_outgoing_peers`, rather
  than through `drop_peer`, which does not do the last of those.

**The second half, and it is the part that would have looked like a bug.** A
`Dead` row can be sitting in the dial queue when it is reclaimed, and
`task_manage_outgoing_peer` answered a missing row with `Error::BugPeerNotFound`.
A bound that logs "bug" for its own correct behaviour is worse than no bound,
so that path now returns quietly: a queued handle outliving its row is ordinary
once the map is bounded.

**Why it has to be here.** `PeerStates` is `pub(crate)` inside `librqbit`, the
map is private, and no option, builder or trait reaches it. `bit-cli` carries
`--max-rss` as a backstop precisely because nothing in this tree could free a
row, and that flag stops the run rather than fixing anything.

**How it was measured.** `scripts/check-peer-rows.ps1`, 2,000 connections in
steps of 200 against one seeder, reading the row count out of the seeder's own
`progress` events:

| connections | rows before | rows after |
| --- | --- | --- |
| 1,000 | 1,000 | 1,000 |
| 1,200 | 1,200 | **1,024** |
| 2,000 | 2,000 | **1,024** |

Exactly 1,024 and flat, and one row per handshake below the bound, which is the
attribution T-040 rests on and is asserted separately: a bound that reclaimed a
live peer would also make the count flat.

**RSS at 2,000 connections is unchanged, and that is expected rather than a
disappointment.** Freeing a row returns it to the allocator, not to the
operating system, so the saving does not show as resident memory at this scale;
976 reclaimed rows are inside the run-to-run variation. What the bound changes
is that demand stops growing, which is what a process that fails at 3am needs.
The six hour soak that would show it as a flat line is `TODO/memory.md` T-040's
own acceptance and has not been run since the change.

**Offer it upstream.** It is [rqbit#525](https://github.com/ikatson/rqbit/issues/525),
open, and reported as exactly this: RSS climbing in a long-lived server.
