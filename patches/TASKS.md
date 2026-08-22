# The work the fork exists to do

Ordered. Every item names the `TODO/` entry it unblocks and the seam it has to
reach, because a vendored change with no entry behind it cannot be reviewed
against anything.

This is not `TODO/INDEX.md` and does not replace it. An item here is a
**vendored change**; the entry it unblocks stays where it is and closes there,
with its own acceptance run. `TODO/` remains the authoritative record.

Written 2026-08-22, the session that vendored the trees and changed nothing in
them. Two to three sessions of work.

## What owning the fork is worth, counted

Nine entries are held up by a seam `librqbit` does not expose. Two of them are
**P0**.

| entry | priority | status | what it is waiting for |
| --- | --- | --- | --- |
| [T-020](../TODO/peers.md) | **P0** | open | a `tokio::select!` arm in upstream's accept loop |
| [T-040](../TODO/memory.md) | **P0** | partial | nothing reclaims a peer row, and nothing bounds the sets |
| [T-022](../TODO/peers.md) | P1 | partial | an HTTP tracker announce per address family |
| [T-132](../TODO/multi-source.md) | P1 | partial | peer identity on `TorrentStorage` |
| [T-016](../TODO/disk-io.md) | P2 | blocked | a resume cache without session persistence |
| [T-100](../TODO/bep-coverage.md) | P2 | partial | the send half of an extension message |
| [T-163](../TODO/peers.md) | P2 | open | MSE, a wire-level handshake |
| [T-167](../TODO/bep-coverage.md) | P2 | blocked | no inverse of `on_have` |
| [T-102](../TODO/bep-coverage.md) | P3 | open | `PeerConnectionHandler`, for BEP 55 |

That is both P0 items in the record, the one open and the one partial, both
partial P1 items, and both blocked entries.

## 0. Before anything: is 9.0.1 broken for us

**[rqbit#637](https://github.com/ikatson/rqbit/issues/637)**, "[regression]
rqbit faill to add torrent larger than 2MB", opened 2026-08-22T11:30:35Z with
an empty body, against the exact release this tree now vendors. It was found by
`scripts/upstream-scan.ps1` half an hour after it was filed, which is the
clearest argument for running that scan there is.

Nothing here reproduces it yet, and nothing here would: the largest fixture in
this repository is a few kilobytes of payload, so no test adds a `.torrent`
anywhere near two megabytes. **Establish whether `bit-cli` is exposed before
building on top of the vendored tree.** A 2 MB `.torrent` is roughly a hundred
thousand piece hashes, which is a payload of about 1.6 GiB at a 16 KiB piece
length or far less at a smaller one, and `bit-cli create` is what would make
one. Whether it can do so quickly enough to be a test is itself unmeasured.

If it reproduces, it is a P0 and the first patch. If it does not, say so in the
entry that records this and move on: an upstream report we could not reproduce
is still worth writing down, because the next reconciliation will meet it
again.

## 1. T-020, the open P0, and it is one match arm

`TODO/peers.md` T-020 already did the work of finding this and the entry is
worth reading in full before touching anything.

Defect two is `task_listener` in `vendor/rqbit/crates/librqbit/src/session.rs`.
Its second `tokio::select!` arm is
`Some(Ok((live, checked))) = futs.next(), if !futs.is_empty()`. A pending
handshake check that resolves to `Err` **fails that pattern**, so `select!`
disables the arm for the rest of the call and waits on `l.accept()`, which on
an idle seeder is forever. Nothing in `futs` is polled until the next
connection arrives, so the queue drains at one entry per accepted connection
and sockets pile up in `CLOSE_WAIT`.

The entry measured it one for one: twenty poisoned connections, then single
peers one at a time, and **the twentieth got a handshake while the nineteen
before it got nothing**. `bench/listener-20260822T045550230Z.json`, case
`recovery`.

The change is to match `Some(_)` and handle the `Err` rather than let the
pattern fail. What has to be proved is the acceptance the entry already names:

```bash
pwsh -NoProfile -File scripts/check-close-wait.ps1 -Ceiling 100
```

That script currently fails, and it is written not to fail the build for this
defect alone, which is the pattern `TODO/RULES.md` section 5 describes. When
the patch lands, that exemption comes off.

**Offer this one upstream.** It is upstream's bug, not our preference, and it
is small enough to review in one screen.

## 2. T-040, the other P0, and there is prior art

`TODO/memory.md` T-040 is partial: attributed and bounded, not fixed. T-020
found the shape of it, that a peer row is kept for every completed handshake
and never reclaimed, and that twenty-four handshake-and-close connections leave
twenty-four rows at `live 0` and `dead 0` forever.

There is a maintained patch series for exactly this class of problem, and it is
usable. See the section on `nzbd` below: four of its nine patches are bounds on
the sets this entry is about.

## 3. MSE, and upstream has a pull request open

[rqbit#633](https://github.com/ikatson/rqbit/pull/633), "feat(mse): Message
Stream Encryption (MSE) support", is open upstream and unblocks
[T-163](../TODO/peers.md).

Two independent implementations exist to read before writing a third:

- **The upstream pull request**, which is the one that will eventually decide
  what `librqbit`'s API looks like. Taking it early means taking the merge cost
  if it changes before it lands.
- **`FluxDown`**, which is already in the corpus and needs no fetching:
  `reference/FluxDown/native/engine/vendor/librqbit/src/mse/` holds `dh768.rs`,
  `rc4.rs`, `stream.rs` and `mod.rs`, against a vendored `librqbit` 8.1.1. It
  landed well before upstream had a pull request at all, which is the
  demonstration that this is reachable from a vendored fork.

`reference/FluxDown` is MIT. `TODO/RULES.md` section 7 says to read cited code
and never to copy corpus files into this repository; an MIT licence permits the
copy with attribution, so if any of it is taken rather than read, it is
attributed in `UPSTREAM.md` and in `THIRD_PARTY.md` both.

**Decide before starting** whether this tree takes upstream's shape or its own.
Taking upstream's costs a merge every time the pull request moves and costs
nothing when it lands. Taking our own is available immediately and has to be
carried forever. The entry does not have to decide it; this session did not.

## 4. The nzbd series: nine patches, and the licence permits using them

<https://github.com/pjunod/nzbd/tree/main/contrib/rqbit> is a maintained series
of nine patches against rqbit **v8.1.1**, by another project vendoring the same
dependency.

**Licence: MIT OR Apache-2.0**, stated in that repository's `README.md` under
`## License`. Note that the GitHub API reports `license: null` for it, because
there is no `LICENSE` file for the detector to classify, and reading that as
"no licence" is wrong. Using any of it means attribution in `UPSTREAM.md` and
in `THIRD_PARTY.md`.

They are against 8.1.1 and this tree is at 9.0.1, so every one needs forward
porting. Every file they touch still exists at 9.0.1 except one test module,
checked against the vendored tree.

| their patch | what it does | ours |
| --- | --- | --- |
| `0009-bound-pending-incoming-handshakes` | caps pre-routing handshake checks at 256 | **not our fix.** See below. |
| `0010-bound-known-peer-records` | 1,024 retained peer records per torrent, 4,096 per session | [T-040](../TODO/memory.md) |
| `0012-bound-peer-response-backlog` | 128 queued piece and metadata responses per peer | [T-040](../TODO/memory.md) |
| `0014-bound-discovery-pressure` | bounds DHT and magnet-metadata queues and retained candidates | [T-040](../TODO/memory.md), [`TODO/dht.md`](../TODO/dht.md) |
| `0016-limit-peer-metadata-before-allocation` | enforces a BEP 9 ceiling **before** allocating | [T-040](../TODO/memory.md), and it is a denial-of-service shape |
| `0001-allow-persistence-without-auto-restore` | keeps persistence available while disabling implicit admission | [T-016](../TODO/disk-io.md), which is **blocked on exactly this** |
| `0005-bound-tracker-requests` | 1 MiB decoded cap, 30 s completion, 60 s minimum announce | [`TODO/trackers.md`](../TODO/trackers.md) |
| `0007-bound-session-peers` | 80 live peers per torrent, 400 per session | `--peer-limit` already exists here; read before adopting |
| `0018-propagate-file-sizing-errors` | stops initialization on the first sizing failure | [`TODO/disk-io.md`](../TODO/disk-io.md) T-014 |

**`0009` is the one to be careful with, and it is instructive.** It caps the
pending handshake set, and T-020 measured that the cap is *not* the cause of
this repository's P0: `bit-cli` already sets
`max_pending_incoming_handshake_checks` to `usize::MAX` deliberately, because
that is what removed the panic that was defect one. The entry says it outright:
"a reader who fixed the cap would have fixed nothing." Two projects, the same
file, and different problems. Adopt the eight, read the ninth, and do not let
its title decide anything.

**`0001` is the highest value of the nine here**, because [T-016](../TODO/disk-io.md)
is the only entry in the whole record that is blocked on a decision rather than
on a defect: decision 7.4 puts session persistence in Phase C, and `librqbit`
9.0.0 offers no way to have a resume cache without it. A seam that separates
persistence from auto-restore removes the conflict without touching 7.4.

## 5. The rest, in the order the entries already argue for

[T-022](../TODO/peers.md) HTTP announce per family, at
`vendor/rqbit/crates/tracker_comms/src/tracker_comms.rs:293`, where UDP already
announces to both at `:374-387`. Then [T-132](../TODO/multi-source.md),
[T-100](../TODO/bep-coverage.md), [T-167](../TODO/bep-coverage.md) and
[T-102](../TODO/bep-coverage.md). Each entry names its seam with a line number
and none of them needs re-deriving.

## Returning to ordinary work

The fork is a means. When the P0 items are closed and the entries above have
moved, `PROGRESS.md`'s work order goes back to being derived from
`TODO/INDEX.md`'s four questions, and the vendored trees become maintenance:
run `scripts/upstream-scan.ps1` on a version bump, reconcile with
`scripts/vendor-sync.ps1`, and keep `UPSTREAM.md` true.

The signal that it is time: no entry in the table at the top of this file is
still waiting on a seam.
