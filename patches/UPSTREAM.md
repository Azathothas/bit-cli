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
             patches/rqbit/0006-crates-librqbit-src-peer_connection.rs.patch
             patches/rqbit/0007-crates-librqbit-src-peer_info_reader-mod.rs.patch
             patches/rqbit/0011-crates-librqbit-src-torrent_state-live-mod.rs.patch
             patches/rqbit/0013-crates-peer_binary_protocol-src-lib.rs.patch
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

**What it did not fix, until later the same day.** The read side. `ReadBuf`
was a 32,768 byte ring buffer, so the same message failed on receipt past
**262,104 pieces** with "read buffer is full". That was
[`TODO/peers.md`](../TODO/peers.md) T-195, and it is closed: the section below,
"a message larger than the read buffer cannot be received", is the change.

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
             patches/rqbit/0016-package-lock.json.patch
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
             patches/rqbit/0009-crates-librqbit-src-session.rs.patch
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
             patches/rqbit/0011-crates-librqbit-src-torrent_state-live-mod.rs.patch
             patches/rqbit/0012-crates-librqbit-src-torrent_state-live-peers-mod.rs.patch
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
---

## librqbit: an HTTP tracker is told about one of our two addresses

```
Unblocks:    T-022, TODO/peers.md, and it is the half that was left open
Files:       vendor/rqbit/crates/tracker_comms/src/tracker_comms.rs
             vendor/rqbit/crates/librqbit/src/session.rs
             patches/rqbit/0009-crates-librqbit-src-session.rs.patch
             patches/rqbit/0015-crates-tracker_comms-src-tracker_comms.rs.patch
Upstream:    not offered yet, and it should be
Added:       2026-08-22T17:26Z
```

A UDP tracker is announced to once per address family. `tracker_comms.rs`
resolves the host, keeps the first IPv4 and the first IPv6 address, and fires
both. An HTTP tracker got one announce, over whichever family the connector
picked, and a tracker records the source address of the connection it was
announced over. So a dual-stack seeder registered one of its two addresses and
peers on the other family learned nothing reachable, connected, failed, and
retried.

The change gives the HTTP path the same two announces:

- `UdpTrackerResolveResult` and `udp_tracker_to_socket_addrs` are
  `TrackerResolveResult` and `tracker_to_socket_addrs`. Same code, same
  behaviour: which of our addresses a tracker can be told about has nothing to
  do with the scheme, and the HTTP path calls it now.
- `TrackerComms` takes a `ReqwestClientFactory` beside the client it already
  took. A built `reqwest::Client` cannot be reconfigured, so a second one has
  to be built, and building it from the session's own builder is what keeps
  the proxy, the bound interface and the user agent from drifting apart.
- `task_single_tracker_monitor_http` resolves the tracker each round and holds
  one client per family, each with `resolve_to_addrs` pinning that family. It
  rebuilds them only when the resolution changes, so a tracker that gains an
  AAAA record is announced to over both families from the next announce rather
  than from the next restart.
- The two announces go **in sequence**. A tracker that keys its peer records
  by peer id alone, which is all BEP 3 asks for, keeps whichever announce it
  saw last, so concurrent announces make which of our addresses it holds a
  race. T-022 measured that against the fixture before it keyed by
  `(peer id, family)`.
- Every path that cannot pin a family falls back to the session's client and
  one announce, which is exactly the previous behaviour: a URL naming an
  address, a host that resolves in one family, a resolution that fails, a
  client that will not build, and a session behind a proxy. `librqbit` passes
  `None` for the factory when a proxy is configured, because then the proxy
  resolves and the local family is not ours to choose.

**`ClientBuilder::local_address` is the obvious thing to reach for and it does
not work.** `hyper-util` binds the local address only when it already matches
the destination's family, and otherwise falls through to the unspecified
address of the destination's own family, so an announce from a client with
`0.0.0.0` set still goes out over IPv6. T-022 recorded that at
`hyper-util-0.1.20/src/client/legacy/connect/http.rs:794-820` when
`bit-cli trackers` hit the same wall. Overriding the resolution is what pins it.

**Why it has to be here.** `TrackerComms` owns its `reqwest::Client`, takes it
as an argument to `TrackerComms::start`, and exposes no seam for a second one.
`task_single_tracker_monitor_http` is a private method and the send site is
inside it. Nothing about any of it is reachable from a dependent crate: there
is no option, no builder and no trait a caller can implement differently, and
`bit-cli` cannot make a session register both of its addresses with an HTTP
tracker by any configuration.

**How it was measured.** `loopback-tracker` bound on `127.0.0.1` and `[::1]` at
one port, keying peer records by `(peer id, family)` and logging the source
address of every announce. One `bit-cli seed`, DHT, LSD and PEX off, announcing
to that tracker under two names.

| case | tracker URL | before | after |
| --- | --- | --- | --- |
| `dual_host` | `http://localhost:<port>/announce` | **ipv6 only**, from `::1` | **ipv4 from 127.0.0.1 and ipv6 from ::1** |
| `literal_host` | `http://127.0.0.1:<port>/announce` | ipv4, from `127.0.0.1` | ipv4, from `127.0.0.1` |

```bash
pwsh -NoProfile -File scripts/check-tracker-family.ps1
```

`bench/tracker-family-20260822T172231576Z.json` is the before, taken with the
two files stashed and the tree rebuilt, and
`bench/tracker-family-20260822T172549738Z.json` is the after.

**`literal_host` is the control and it is the point of having two cases.** It
takes the fallback path, which is the old code, so it announces once and must
keep announcing once. A check that reported two families there would be
reporting that something announces twice regardless, which measures nothing.
The before run is what says the check can fail: it names `ipv6` alone, which is
the resolver's order rather than a choice, and an IPv4-only peer reading that
tracker got nothing it could dial.

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

**Offer it upstream.** It is [rqbit#537](https://github.com/ikatson/rqbit/issues/537),
open, and the UDP path in the same file is the shape to point at: this makes
the HTTP one match it. Until it is offered this section says so rather than
claiming otherwise.
---

## librqbit: an incoming peer is recorded under our own peer id

```
Unblocks:    T-210, TODO/peers.md, and T-132 could not work without it
Files:       vendor/rqbit/crates/librqbit/src/peer_connection.rs
             patches/rqbit/0006-crates-librqbit-src-peer_connection.rs.patch
Upstream:    not offered yet, and it should be
Added:       2026-08-22T17:55Z
```

`manage_peer_incoming` builds the handshake it is about to send, writes it, and
then hands **that** handshake to `on_handshake` and asks **it** whether the
extension protocol is supported:

```rust
let handshake = Handshake::new(self.info_hash, self.peer_id);
let hlen = handshake.serialize_unchecked_len(&mut *write_buf);
// ... written to the peer ...
let handshake_supports_extended = handshake.supports_extended();
self.handler.on_handshake(handshake, incoming.kind)
```

Both answers are about this session rather than about the peer. Two things
follow:

- **Every incoming peer is filed under our own peer id.** `on_handshake` calls
  `set_peer_live`, which records `handshake.peer_id`, so anything asking who a
  peer is gets ourselves.
- **Every incoming peer is assumed to speak BEP 10.** `Handshake::new` always
  sets the extension bit, so `handshake_supports_extended` is unconditionally
  true for an incoming connection whatever the peer said.

`manage_peer_outgoing`, forty lines below, reads the peer's handshake off the
wire and uses that for both. The two paths disagreeing is what says this is a
slip rather than a design.

The change uses `incoming.handshake`, which is the peer's, already read by the
session's accept path and already validated eight lines above for a wrong info
hash and for a self-connection. The handshake being sent is renamed `ours`, so
neither can be reached for by accident again.

**Why it has to be here.** `manage_peer_incoming` is a method on
`PeerConnection`, the handshake never leaves it, and no option, callback or
trait a dependent crate can implement reaches either line.

**How it was measured.** By a rate limit that did not limit.
`scripts/check-rate-scope.ps1`'s `http_peer_cap` phase caps swarm peers and
attaches an HTTP source, which reaches the session as an **incoming** peer over
loopback and is exempt from that cap by its peer id prefix:

| | before | after |
| --- | --- | --- |
| HTTP under an 8 MiB/s peer cap | **8.40 MiB/s**, the cap | **151.84 MiB/s** |

The exemption matched nothing before, because the id it was matching against
was ours. `bench/rate-scope-20260822T175543220Z.json`.

**Offer it upstream.** Three lines, a defect in their code, and the outgoing
path beside it is the argument for the change.

---

## librqbit: a download limit that some peers do not pass through

```
Unblocks:    T-132, TODO/multi-source.md
Files:       vendor/rqbit/crates/librqbit/src/limits.rs
             vendor/rqbit/crates/librqbit/src/torrent_state/live/mod.rs
             patches/rqbit/0005-crates-librqbit-src-limits.rs.patch
             patches/rqbit/0011-crates-librqbit-src-torrent_state-live-mod.rs.patch
Upstream:    not offered yet
Added:       2026-08-22T17:55Z
```

`Limits` had two limiters, a total up and a total down, and `LimitsConfig` has
exactly two fields. Nothing was scoped to a peer, so a cap that excludes one
peer could not be expressed. That is a problem for any client that also feeds
the session from a source of its own: `bit-cli` bridges each HTTP web seed in
as an ordinary peer over loopback, so every download cap reached it too and
there was no way to cap the swarm alone.

The change adds a third limiter and a way to skip it:

- `Limits::peer_down`, a second download limit, off unless set. `down` still
  bounds everything, which is what a total does.
- `Limits::exempt`, a list of peer id prefixes `peer_down` does not apply to.
  A prefix rather than a whole id because a client's own bridge generates a
  fresh id per connection and only the first eight bytes identify it; a prefix
  rather than an address because that bridge dials in from an ephemeral port
  and reconnects on a new one.
- `prepare_for_download_from(peer_id, len)` charges `down` for everyone and
  `peer_down` for everyone not exempt. The old `prepare_for_download` stays.
- `PeerHandler` carries the peer's id in a `OnceLock`, set in `on_handshake`,
  which is the first point an outgoing peer's id is known. The chunk requester
  cannot run before the handshake, so it is always set by the time it is read.

**Set through a setter rather than through `LimitsConfig`.** `LimitsConfig` is
`Serialize`, `Deserialize` and constructed as a struct literal in four places
across two repositories, so a third field would break each of them for a value
that is set at runtime anyway, next to `set_download_bps`.

**There is no upload counterpart and that is deliberate.** A source bridged
into a session is a seed: it sends `Bitfield` and `Unchoke`, answers `Request`,
and never sends `Interested` and never requests. Nothing is uploaded to it, so
the upload limits already reach peers alone. The doc comment on `peer_down`
says so, because the asymmetry is the first thing a reader will ask about.

**Why it has to be here.** `Limits` is `librqbit`'s, both limiter calls are
inside a private method of a private type, and `LimitsConfig` has no field a
caller could use to say "not this peer". `bit-cli` cannot cap its swarm without
capping its own HTTP sources by any configuration.

**How it was measured.** `scripts/check-rate-scope.ps1`, ten phases against one
payload, one mirror and one seeder, `bench/rate-scope-20260822T175543220Z.json`:

| phase | total | HTTP | peers |
| --- | --- | --- | --- |
| `http_peer_cap` | 151.84 MiB/s | 151.84 | 0 |
| `peer_ceiling` | 259.11 MiB/s | 0 | 259.11 |
| `peer_peer_cap` | 8.42 MiB/s | 0 | 8.42 |
| `hybrid_both_caps` | 27.57 MiB/s | 18.31 | 9.26 |

Each assertion is arranged so it is an invariant rather than a race: one source
is the only supplier in the two rows that matter. `peer_ceiling` exists so
`peer_peer_cap` means something, because a cap that holds on a slow peer has
measured nothing.

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

**Not offered upstream yet, and it is the one patch here that may not belong
upstream as written.** The other four are defects in upstream's code. This is a
feature, and a maintainer may want the exemption expressed differently, for
example as a per-connection `LimitsConfig` rather than as a list of peer id
prefixes on the session. It is worth offering as a question rather than as a
patch.
---

## librqbit: a message larger than the read buffer cannot be received

```
Unblocks:    T-195, TODO/peers.md, the residual T-194 left behind
Files:       vendor/rqbit/crates/librqbit/src/read_buf.rs
             vendor/rqbit/crates/librqbit/src/peer_connection.rs
             vendor/rqbit/crates/librqbit/src/peer_info_reader/mod.rs
             vendor/rqbit/crates/librqbit/src/torrent_state/live/mod.rs
Upstream:    not offered yet, and it should be
Added:       2026-08-22T18:57Z
```

`ReadBuf` is the ring buffer every peer connection reads into, and it was a
`Box<[u8; BUFLEN]>` with `BUFLEN` = 32,768. A message that does not fit in it
fails with "read buffer is full" and the connection dies. One message is not
bounded by anything that constant knows about: a bitfield is one bit per piece,
so past **262,104 pieces** it does not fit, and no configuration of either end
changes that. T-194 moved the send side off a fixed buffer; this is the same
defect read from the other side, and it was the binding limit afterwards.

The change is that the buffer grows:

- `buf` is a `Box<[u8]>`, and every place the ring arithmetic said `BUFLEN`
  reads the current capacity instead. `BUFLEN` is what a connection starts
  with.
- `grow` doubles into a new allocation and copies the two halves contiguously
  to the front, which is what `make_contiguous` already did for a different
  reason. It is called from one place, the `NotEnoughData` arm, when the buffer
  is full and the message is not finished. When it refuses, the caller fails
  exactly as it did before.
- `ReadBuf::max_len` bounds it, and `set_max_len` never lowers it below
  `BUFLEN`, so this can only ever permit more than the old behaviour.

**Where the bound comes from is the whole of the design.** It is never the
length prefix the peer sent, which is the number a hostile peer picks. It comes
from a new trait method, `PeerConnectionHandler::max_incoming_message_len`,
whose default is `BUFLEN`, so an implementor that does not answer keeps the
behaviour it had:

- The live torrent's handler answers from **its own piece count**: one bitfield
  plus `MAX_MSG_LEN` of slack. A peer can make the buffer as large as one
  bitfield for the torrent it is talking about and no larger.
- `peer_info_reader` cannot, and that is the interesting case. A seeder sends
  its bitfield immediately after the handshake, before this side has the
  metadata, so the message that arrives is as large as the torrent makes it
  while the piece count is the exact thing not known yet. It answers with a
  constant, `MAX_BITFIELD_BEFORE_METADATA` = 1 MiB, which is 8,388,600 pieces,
  128 GiB at a 16 KiB piece length and 32 TiB at 4 MiB.

The connection sets it: `manage_peer_outgoing` on the buffer it creates, and
`manage_peer_incoming` on the one the session handed it, which was filled with
the handshake before anyone knew which torrent it was for.

**Why it has to be here.** `ReadBuf` is private to `librqbit`, the buffer is a
private field, `read_message` is where the failure is raised, and
`PeerConnectionHandler` is `pub(crate)`. There is no option, no builder and no
trait a dependent crate can reach any of it through. `bit-cli` cannot fetch a
torrent past 262,104 pieces by any configuration.

**How it was measured.** `scripts/check-bitfield.ps1`, a seeder and a magnet
fetch on loopback with trackers and DHT off. Metadata resolving and the file
appearing is the pass, and both need the bitfield to have crossed:

| pieces | `.torrent` | bitfield | before | after |
| --- | --- | --- | --- | --- |
| 262,104 | 5,242,219 B | 32,768 B | resolves | resolves |
| **262,105** | 5,242,239 B | 32,769 B | `read buffer is full` | **resolves** |
| **524,288** | 10,485,900 B | 65,541 B | `read buffer is full` | **resolves** |
| **1,048,576** | 20,971,661 B | 131,077 B | `read buffer is full` | **resolves** |

`bench/bitfield-20260822T185725425Z.json` is the million-piece run.

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

140 upstream tests pass, one of them new:
`test_read_buf_grows_for_a_message_larger_than_itself`, which asserts both
directions, that the message is refused with the default bound and read with a
raised one.

**The unsafe reborrow is still sound and the growth path is inside what proves
it.** `read_message` holds a stacked reborrow of `self` across the deserialize,
and growth reallocates the buffer that reborrow points into.
`test_read_buf_miri` reads an oversized bitfield as well as a piece now, so
that happens under miri:

```bash
cargo +nightly miri test --manifest-path vendor/rqbit/Cargo.toml -p librqbit --features miri test_read_buf_miri -- --ignored
```

Two things about running that on Windows, because both cost time here.
`cargo-miri` fails with "cargo uses an argfile to invoke rustc" once the
command line gets long, and a short `CARGO_TARGET_DIR` is the way past it. And
`with_timeout` is a no-op only under `--features miri`, so a test that reaches
it cannot run outside miri without a tokio runtime.

**What it does not fix.** The pre-metadata bound is a constant rather than a
fact about the torrent, so it is a limit rather than the absence of one.
Removing it properly means skipping a message this side has no use for instead
of buffering it, which changes `read_message` from "return a message" to "may
drop one". T-195 records that and nothing in this repository needs it.

**Offer it upstream.** It is a defect in their code with a one line
reproduction, and the trait method's default means no implementor of theirs has
to change.
---

## librqbit: a resume cache cannot exist without a session store

```
Unblocks:    T-016, TODO/disk-io.md, which was blocked on exactly this
Files:       vendor/rqbit/crates/librqbit/src/lib.rs
             vendor/rqbit/crates/librqbit/src/session.rs
             vendor/rqbit/crates/librqbit/src/torrent_state/initializing.rs
             vendor/rqbit/crates/rqbit/src/main.rs, which builds the struct
             patches/rqbit/0004-crates-librqbit-src-lib.rs.patch
             patches/rqbit/0009-crates-librqbit-src-session.rs.patch
             patches/rqbit/0010-crates-librqbit-src-torrent_state-initializing.rs.patch
             patches/rqbit/0014-crates-rqbit-src-main.rs.patch
Upstream:    not offered yet, and the first two thirds of it should be
Added:       2026-08-22T19:28Z
```

`SessionOptions::fastresume` exists and does nothing unless `persistence` is
also set. `persistence_factory` reads it inside a macro that only the two
persistence arms reach; the `None` arm returns `NonPersistentBitVFactory`
whatever `fastresume` says. So the only way to have a resume cache is to turn
on a store that writes a record of every torrent in the session.

Those are different things. A resume cache is derived data: delete it and the
next run recomputes it, slowly, and is otherwise identical. A session store is
state: delete it and the session forgets what it was doing. `bit-cli` is a
one-shot foreground tool that keeps no state by decision, and re-hashing the
payload on every invocation costs eight minutes on a 40 GiB seed.

The change is a seam, not a policy:

- `SessionOptions::bitv_factory: Option<Arc<dyn BitVFactory>>`. Used wherever
  the session would otherwise use `NonPersistentBitVFactory`, so it changes
  nothing for a caller that leaves it unset or that already has persistence
  and `fastresume` on.
- `bitv` and `bitv_factory` are public modules, and `BitV`, `BoxBitV`,
  `DiskBackedBitV`, `BitVFactory`, `NonPersistentBitVFactory` and `BF` are
  re-exported. A caller supplying a factory has to be able to name the trait,
  return a `BitV` from it, and reach the disk-backed implementation rather than
  write a third one.
- `validate_fastresume` clears by the key `check` loaded with. It cleared by
  `shared.id`, a torrent id, while everything around it used the info hash, so
  a factory keyed by hash could not resolve it and a cache found to be corrupt
  was never removed. That third one is a defect rather than a seam.

**Nothing about the validation changed and none of it needed to.** `librqbit`
already checks the bitfield's length against the torrent, re-hashes at least
one claimed piece per file plus a random sample of the rest, and throws the
whole thing away and clears the cache on a single failure. That is the part
that makes a resume cache safe and it was already written.

**Why it has to be here.** `persistence_factory` is a nested function inside
`Session::new_with_opts`, the modules were private, and `SessionOptions` had no
field to put a factory in. There is no option, builder or trait a dependent
crate could reach any of it through, which is why T-016 sat blocked on a
decision it was not really about.

**How it was measured.** `scripts/check-fastresume.ps1`, one 512 MiB payload of
1 MiB pieces, five `bit-cli seed --announce-only` runs:

| run | `--fastresume` | elapsed | reports complete |
| --- | --- | --- | --- |
| `cold` | yes, empty cache | 2.38 s | yes |
| `warm` | yes | **2.06 s** | yes |
| `stale`, one byte rewritten | yes | 2.38 s | **no** |
| `refresh` | yes | 2.05 s | no |
| `no_flag` | no | 2.37 s | no |

The clock says the check was skipped and the `complete` column says the cache
was right: `warm` claims the whole payload without hashing it, and `stale`
refuses the cache, hashes again, and finds the one piece that changed. A run
that trusted a stale cache would have announced a piece it does not have.

**Offer the first two thirds upstream.** The `bitv_factory` seam and the module
exports are a feature a maintainer may want shaped differently, and the
`validate_fastresume` key is a defect worth sending on its own.
---

## librqbit: the enum a public type's only field holds is private

```
Unblocks:    T-025, TODO/peers.md
Files:       vendor/rqbit/crates/librqbit/src/http_api_types.rs
             patches/rqbit/0003-crates-librqbit-src-http_api_types.rs.patch
Upstream:    not offered yet, and it is a one line change
Added:       2026-08-22T19:38Z
```

`http_api_types` re-exports `PeerStatsFilter`. That type has exactly one field,
of type `PeerStatsFilterState`, and the enum was not re-exported with it. So a
dependent crate could name the filter and not build one: the variant that asks
for every peer rather than only the connected ones has no name outside the
crate.

`bit-cli` needs every peer, including one that took two gigabytes and left, so
it built the value through the type's own `Deserialize` from the literal
`{"state":"all"}` with an `unwrap_or_default()` behind it. That works and reads
badly, and the fallback would have quietly narrowed the report to live peers if
the literal had ever stopped parsing.

The change adds `PeerStatsFilterState` to the same `pub use`.

**Why it has to be here.** It is an export. There is nowhere else.

**How it is held.** `crates/bit-cli-core/src/engine.rs`, `all_peers_filter`,
constructs `PeerStatsFilter { state: PeerStatsFilterState::All }` with no
`serde_json` in it. `cargo test --workspace` covers the reports that read it.

**Offer it upstream.** One line, no behaviour, and the type it completes is
already public.
