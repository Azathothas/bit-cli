# Progress

**Read this first.** It is the only thing the kickoff prompt tells a session to
read, so everything that changes from session to session is here: the baseline,
what the last session did, and the work order. The prompt carries none of it, by
[RULES.md](RULES.md) section 3.

It carries no history: every session rewrites it. For history, read the git log
and the entries themselves.

Rules for working on this repository: [RULES.md](RULES.md).
Every entry, one line each: [INDEX.md](INDEX.md).

> **The shape this file must keep**, from [RULES.md](RULES.md) section 2 step 2:
> the state line with the session's start instant in ISO 8601 UTC, the measured
> baseline with the CI run named by id, the entry counts, what the session did,
> what is in progress, **Start here next session** as an ordered list with entry
> ids and corpus sources, and open questions for the operator.
> `scripts/session-report.ps1` prints the numbers; do not count them by hand.
>
> `scripts/check-todo.ps1` checks most of that shape now, and `scripts/gates.ps1`
> runs it, so a missing section or a stale count fails a gate rather than a
> review. [RULES.md](RULES.md) section 5, "The record".

---

## Before typing a `bit-cli` flag, read `man/bit-cli.json`

`man/` holds the whole command surface, generated and committed: `bit-cli.1` for
a terminal, `bit-cli.md` for reading, and **`bit-cli.json`, a CLIspec 0.3
document, for a program**. Every command, every flag, the values it accepts, its
default, and every exit code with whether a retry could succeed.

It cannot go stale: `cargo test -p bit-cli --test man_is_current` fails until it
is regenerated with `pwsh -NoProfile -File scripts/check-man.ps1 -Fix`.
[`docs/man.md`](../docs/man.md) says what each field carries.

## `patches/TASKS.md` is finished, and the fork is maintenance now

Twelve of its thirteen rows are done and the thirteenth is not waiting on a
seam any more. That file's own closing section says the signal to look for is
"no entry in the table at the top of this file is still waiting on a seam", and
that is now true: [T-102](bep-coverage.md), BEP 55, waits on a measurement its
own acceptance asks for and on a fixture that can produce an unreachable peer,
neither of which is `librqbit`'s to give.

So the next session's work order is derived from [INDEX.md](INDEX.md)'s four
questions again, not from `patches/TASKS.md`, and the vendored trees become
what `patches/README.md` describes: run `scripts/upstream-scan.ps1` on a version
bump, reconcile with `scripts/vendor-sync.ps1`, keep `UPSTREAM.md` true.

## State

- **Last session:** 2026-08-23T01:53:05Z, unattended, 3h 44m. It was
  ended on the operator's word rather than by running out of work.
- **Tests:** 1,166 passing, 0 failing. 1,131 at the start. Plus **149** in the
  vendored trees, which the workspace gates do not run, up from 142.
- **Gates:** clean.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green at run **32620536345** against commit `a289977`, all
  **seventeen** jobs, and that commit is this session's last change to source.
- **Entries:** 161 items. 44 open, 2 partial, 0 blocked, 105 done, 10 deferred
  to Phase C. 105 of 151 workable done, 46 left.
- **Tree:** 92 Rust files, 52,557 lines of code, 12,567 of comment,
  `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **25 patches**
  across seventeen sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
  `scripts/vendor-status.ps1` exits 0.
- **Version:** `bit-cli` 0.2.0, unchanged.

## What the last session did

It worked [`patches/TASKS.md`](../patches/TASKS.md) in order from section 3 and
reached the end of it, then took the five dependabot pull requests.

### Message stream encryption, and the decision section 3 asked for

**[T-163](peers.md), P2, done.** It was the largest single loss of reachable
swarm in the record: a peer configured to require encryption will not exchange
a byte with a plaintext-only client.

**The decision was to take our own shape rather than
[rqbit#633](https://github.com/ikatson/rqbit/pull/633)'s**, and the argument is
where the tests run and what a reconciliation has to read. Their pull request
puts `crates/librqbit/src/mse/` inside the library; `cargo test --workspace`
does not run the vendored crates' tests, so none of it would be in the gates,
and every future release would carry somebody else's crypto through the merge.
What went into the vendored tree instead is one trait, `StreamTransform`,
called once per connection in each direction before the BitTorrent handshake
crosses it. The implementation is `crates/bit-cli-core/src/mse/`.

Nothing was copied from the corpus and nothing cryptographic came from a
dependency. The exchange is checked against `pow(2, x, P)` from an arbitrary
precision implementation and the cipher against RFC 6229, so `THIRD_PARTY.md`
is owed nothing. **One 768 bit exponentiation costs 51.4 microseconds** and a
handshake needs two.

**All seven acceptance phases hold**,
`bench/encryption-20260823T030511908Z.json`. Three seeders differing only in
`--encryption`, and **two of the seven are controls that must fetch nothing**:
without them a `require` that quietly accepted plaintext would pass every other
row.

**A premise the measurement disproved.** The first implementation let a
responder with `--encryption off` complete the key exchange and refuse
afterwards. That told the dialling end its handshake had worked, so the
fallback never fired and the run looped until its deadline. The refusal is at
the first twenty bytes now.

### The two BEPs, both directions of both

**[T-167](bep-coverage.md), P2, partial to done.** The send half of BEP 54. The
bridge reads the session's `m` out of its extended handshake, which is the
second BEP 10 table [T-166](peers.md) records and which did not exist because
nothing sent an extension message. A source that loses a file sends one
`lt_donthave` per piece given up and **stays connected**: four retracted, four
dropped, zero reconnects charged to `file_gone`, one loopback port used.

Two things the entry did not predict, and both cost a red test. Every block in
flight against a lost file fails the same way, so narrowing on each of them
reported the file once per failure and then retired the source for being unable
to narrow. And a request already sent for a piece just retracted arrives after
the retraction; refusing it ends the connection, which is what the extension
exists to avoid.

**Clearing the bit was not the whole of honouring one.** It stops the peer being
picked for that piece again and does nothing about the piece already assigned to
it. `PieceTracker::release_piece_owned_by` gives it back to the queue.

**[T-100](bep-coverage.md), P2, partial to done.** BEP 6, which `librqbit` had
none of: five message ids and a reserved bit, so a peer that spoke the fast
extension was answered with an unsupported-message error and dropped. Measured
from the wire, `bench/swarm-20260823T040125619Z.json`:

| case | `fast_negotiated` before | after | `have_all` | `received` |
| --- | --- | --- | --- | --- |
| `leech_1` | 0 | **1** | **1** | 8,388,608 |
| `leech_4` | 0 | **4** | **4** | 33,554,432 |
| `leech_16` | 0 | **16** | **16** | 134,217,728 |

The bytes received are identical to the run before the change, so the extension
changed what is said and not what is transferred.

**A test that had been dead for a session**, found on the way:
`test_bitfield_larger_than_max_msg_len`, which is [T-194](peers.md)'s own
regression test, carried no `#[test]`. The one it needed had landed on the test
above it, which then carried two. Nothing in the workspace gates catches this,
because `cargo clippy --workspace` does not compile the vendored crates' test
targets.

### The two bench flakes were different defects

**[T-211](bench.md), P1, done.** Two pieces where three were hashed was a lost
interval, not a lost piece: `drive_leech` took its storage baseline after
`attach_sources` had returned with the bridges dialling, and every counter in
the report is a sum of interval deltas. 4,024 bytes against 3,000 was the wrong
assertion: `summary.bytes` counts what arrived, and with three connections the
session can ask twice for a block already outstanding.

**Proved by running the whole bench module 50 times at `--test-threads 8`, zero
failures.** Fifty local runs is evidence and not a proof; what makes it
closable is that both causes were found and named, and neither was a tolerance.

### The nzbd series, all nine read now

**`0005-bound-tracker-requests`, taken in part.** Neither `reqwest` client the
session builds carried a timeout, `Response::bytes()` read an announce body
with no ceiling, and `interval: 0` gave an announce loop with no sleep in it.
**The floor is five seconds and not the sixty the patch takes**, because the
UDP path in the same file already clamped to five and raising both is a policy
decision about how often to talk to honest trackers.

**`0016`, not taken, and it found something.** The cap it adds already exists at
9.0.1. What is unbounded is the product: `dht_utils.rs:42` runs 128 metadata
reads at once and each may allocate 32 MiB on a peer's word.
**[T-212](memory.md) filed** for that, with the arithmetic and both citations,
and it says plainly which of its numbers is measured and which is multiplied.

**`0012` and `0014`, read and not taken**, with the reasons in
[`patches/TASKS.md`](../patches/TASKS.md) section 4. One of `0014`'s findings is
a defect rather than a bound and nobody has checked it against 9.0.1: a
recursive DHT request sent twice, once for the callback and once for traversal.

### The five dependabot pull requests

All five failed one job and only one: `Third party notices`. `THIRD_PARTY.md` is
generated from `Cargo.lock` and dependabot cannot regenerate it, so every lock
bump it opens fails that job and passes the other sixteen. Taking the five
together cost one CI run instead of five.

**The two crypto bumps could not go in alone.** `md-5` 0.10 carries `digest`
0.10 and `sha1` 0.11 carries `digest` 0.11, and two files import `Digest` from
one and call it on the other, so the three had to move together. One test then
failed on a real change: `sha1` 0.11 returns `hybrid_array::Array` where 0.10
returned a `GenericArray`, and only the second implements `LowerHex`.

The create-determinism fixture hash is unchanged, which is that test's whole
point: the bytes this platform writes are the same after five dependency bumps.
All five pull requests are closed.

### The soak, cut short at 1.32 hours

**It did not finish and the record says so.** `scripts/soak.ps1` was started
with ceilings rather than with the numbers merely recorded, which is what
[T-040](memory.md) closing made possible, and the session ended before its six
hours were up. `bench/soak-20260823T040627780Z.csv` has 145 samples over
**1.32 hours**, and no JSON summary, because the script writes that at the end.

| | |
| --- | --- |
| leech cycles | **288 completed, 0 failed** |
| `CLOSE_WAIT` | **0 at all 145 samples** |
| RSS | 13.58 MiB to 15.20 MiB, slope **+0.622 MiB/h at r squared 0.105** |
| handles | 203 to 215, slope not usable at r squared 0.014 |
| peak RSS | 21.43 MiB |

**Two of those five rows mean something and three do not, at this window
length.** Zero `CLOSE_WAIT` at every sample and zero failed cycles out of 288
are statements a 1.32 hour run can make: they are counts rather than fits. The
RSS slope is not, and the last session already wrote down why in T-040's own
words: an interim read at 5.06 hours gave +1.45 MiB/h at r squared 0.107, which
is noise fitted to a line, and 0.105 here is the same shape. **A slope needs a
window long enough to have a shape**, and this one does not.

So what this run says is that a session which changed the peer connection path
in four places did not break the seeder and did not leak a socket in 288
cycles. What it does not say is anything about the memory slope. A full six
hours against this tree is the first item in the next session's list.

### The reason a peer died was already in hand

**[T-024](peers.md), P2, done**, taken after `patches/TASKS.md` ran out. Its
Approach offered two routes and said the second was the weaker one: add the
counters upstream, "or infer disconnects from a peer leaving the snapshot
between two ticks". The trees are vendored, so the first was available.

`on_peer_died` takes the reason as an `Option<Error>` and **threw it away**, so
a report could say a peer was `dead`, which is a fact about the row rather than
about what happened. `peers[].disconnects` carries it now with an ISO 8601
time, bounded at four per peer, and `peers[].choked` and `peers[].unchoked`
tell a peer that stopped being allowed to send from one that is slow.

```json
{"at":"2026-08-23T05:17:45.446Z","reason":"error writing: An established connection was aborted by the software in your host machine. (os error 10053)"}
```

The entry's title asks for a choke **history** and its Acceptance does not; two
counters answer "how often" and not "when", and the entry says so rather than
leaving a title that promises more than it delivered.

### What review 1 found

Four claims written this session were wrong and two more were loose. The four:
a comparison between something built and something not built, twice; a posture
attributed to `seedchamp` that is the opposite of the one it takes, in two
files; and two `--help` texts describing [T-020](peers.md)'s defect in the
present tense after it was fixed. The man page regenerates with the last of
those.

Three test-count claims in `patches/UPSTREAM.md` read as present tense and were
the counts on the day. They say so now, and point here for the current one.

## In progress

Nothing is half-written.

- **[T-212](memory.md)** is filed and open, with an acceptance that names the
  fixture it needs.
- **[T-102](bep-coverage.md)**, **[T-024](peers.md)** are open.

## Start here next session

**The operator changed the shape of the work order for this one.** Not priority
first. Clear as many small entries as possible, so the open count comes down,
and then take the bigger ones a **category at a time**: all of `bep`, or all of
`dht`, in one session rather than one entry from each.

The reading taken of "provided they depend on a high priority task that's still
open": an easy win is one that is **not** waiting on an open high-priority
entry. That is nearly all of them, and it is worth saying why rather than
asserting it. **There is no open P0 and exactly one open P1**, `T-081`, BEP 52
v2 and hybrid torrents, effort XL. The only open entry that waits on it is
[T-134](bep-coverage.md), v1 and v2 info hash reconciliation. Everything else
below is unblocked.

**27 entries are open at effort S**, none of them P0 or P1. Derived from the
`Effort:` line of every open or partial entry, not from memory:

```bash
pwsh -NoProfile -File scripts/check-todo.ps1
```

1. **Read the CI run named above before anything else.**
2. **A full six hour `scripts/soak.ps1` against this tree**, started first
   because it outlasts most of a session and `gates.ps1` leaves it alone. This
   session's was cut to 1.32 hours and its RSS slope is not usable at that
   window. Ceilings that held for 1.32 hours and are worth keeping:
   `-RssCeilingMiBPerHour 4 -HandleCeilingPerHour 20 -CloseWaitCeilingPerHour 1`.
3. **The `cli` group, eight entries at effort S**, which is the largest single
   category of easy wins and the one where a reader sees the result:
   [T-115](cli-surface.md) partial, [T-136](cli-surface.md),
   [T-154](cli-surface.md), [T-116](cli-surface.md), [T-118](cli-surface.md),
   [T-155](cli-surface.md), [T-156](cli-surface.md), [T-159](cli-surface.md).
   Two of them, T-118 and T-159, are about the help output itself, so
   `scripts/check-man.ps1 -Fix` follows both.
4. **The `ci` and `windows` groups, four entries at effort S**:
   [T-150](cli-surface.md), [T-161](cli-surface.md), [T-075](windows.md),
   [T-178](windows.md). T-150 and T-161 are workflow edits, so they need a real
   run to prove them and cannot go in a `-NoCi` push.
5. **The `trackers` and `dht` groups, five entries at effort S**:
   [T-180](trackers.md), [T-063](trackers.md), [T-065](trackers.md),
   [T-050](dht.md), [T-051](dht.md).
6. **The rest of the effort S entries**, ten of them, in
   `bench`, `create`, `metainfo`, `memory`, `peers`, `performance`, `webseed`
   and `bep`: [T-094](bench.md), [T-191](bench.md), [T-176](create-seed.md),
   [T-173](metainfo.md), [T-187](metainfo.md), [T-041](memory.md),
   [T-165](peers.md), [T-033](performance.md), [T-008](webseed.md),
   [T-103](bep-coverage.md).
7. **Then, a category at a time.** `bep-coverage.md` is the one with the most
   left and the most shared machinery, and this session did three of its
   entries in a row for exactly that reason. After it, `dht.md`.
8. **[T-212](memory.md)** whenever the fixture for it is being built anyway. It
   is the only entry in the record whose numbers are arithmetic rather than
   measurement, and it needs a swarm of peers that answer an extended handshake
   with a large `metadata_size` and then stall.
   `crates/bit-cli-core/src/bench/swarm.rs` already builds synthetic peers.
9. **Offer the patches upstream.** Seventeen sections in
   [`patches/UPSTREAM.md`](../patches/UPSTREAM.md) and not one has been
   offered. [T-020](peers.md) is
   [rqbit#311](https://github.com/ikatson/rqbit/issues/311),
   [T-040](memory.md) is
   [rqbit#525](https://github.com/ikatson/rqbit/issues/525),
   [T-022](peers.md) is
   [rqbit#537](https://github.com/ikatson/rqbit/issues/537),
   [T-100](bep-coverage.md) is
   [rqbit#584](https://github.com/ikatson/rqbit/issues/584), and the rest carry
   a one line reproduction each.

**Two corpus sources the list above may want**, both already on this machine
and neither needing a fetch: `reference/RESEARCH.md` sections C and D, which is
where seventeen of the T-163 to T-182 block came from, and `contrib/rqbit/` in
<https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, which is fetched per
session rather than kept and whose `0012` and `0014` are read and not taken.

## Open questions for the operator

One, and it is named rather than asked, because the session ended before it
could be answered.

- **The soak did not finish.** It was cut to 1.32 hours by the session ending,
  and at that window its RSS slope is noise. Nothing is known about the memory
  slope of a tree that changed the peer connection path four times. It is the
  first item in the list above and it needs six hours of somebody's session.

The operator's two instructions at the start are otherwise done.

- **Finish `patches/TASKS.md`.** Done, including section 3, MSE, which the
  session before this one was told to leave alone. Twelve of thirteen rows are
  done and the thirteenth, [T-102](bep-coverage.md), is no longer waiting on a
  seam.
- **The dependabot pull requests until CI is green.** Done, and all five are
  closed as applied.

One thing worth a decision rather than a question, recorded so the next session
does not re-take it: **`--encryption` defaults to `prefer`**, which changes what
a default run does. It dials with MSE and dials again in plaintext when the peer
does not answer, which is what mainline clients do and which costs one extra
dial per plaintext peer. `off` is one flag away.
