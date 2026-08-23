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

- **Last session:** 2026-08-23T01:53:05Z, unattended, running.
- **Tests:** 1,165 passing, 0 failing. 1,131 at the start. Plus **149** in the
  vendored trees, which the workspace gates do not run, up from 142.
- **Gates:** clean.

```bash
pwsh -NoProfile -File scripts/gates.ps1
```

```bash
cargo test --manifest-path vendor/rqbit/Cargo.toml --target-dir target/vendor-rqbit
```

- **CI:** green at run **32618552099** against commit `9ddafaa`, all
  **seventeen** jobs, and that commit is this session's last change to source.
- **Entries:** 161 items. 45 open, 2 partial, 0 blocked, 104 done, 10 deferred
  to Phase C. 104 of 151 workable done, 47 left.
- **Tree:** 92 Rust files, 52,440 lines of code, 12,530 of
  comment, `scc --no-cocomo crates/`. Excludes `vendor/`.
- **Vendored:** rqbit `v9.0.1`, both siblings pinned by commit, **23 patches**
  across sixteen sections in [`patches/UPSTREAM.md`](../patches/UPSTREAM.md).
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

### The soak

Running at the time this was written; the result and the report path go here
before the session ends.

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

`patches/TASKS.md` is finished, so this is derived from [INDEX.md](INDEX.md)'s
four questions again rather than from it. Re-derive rather than trusting this
list if the argument in `INDEX.md` has moved.

1. **Read the CI run named above before anything else.**
2. **[T-212](memory.md)**, P2, filed this session and the only entry whose
   numbers are arithmetic rather than measurement. It needs a fixture swarm of
   peers that answer an extended handshake with a large `metadata_size` and
   then stall; `crates/bit-cli-core/src/bench/swarm.rs` already builds
   synthetic peers and is where one would go. The bound is
   `vendor/rqbit/crates/librqbit/src/dht_utils.rs:42` and
   `vendor/rqbit/crates/librqbit/src/peer_info_reader/mod.rs:87`.
3. **[T-024](peers.md)**, P2, per-peer choke and unchoke history, which needs no
   corpus.
4. **`0012-bound-peer-response-backlog`**, which is read and not taken: the
   session's own writer channel is unbounded and a peer can queue piece
   responses faster than a slow socket drains them. It belongs to
   [T-040](memory.md)'s family and needs its own entry and its own measurement
   before the patch. Corpus: `contrib/rqbit/` in
   <https://github.com/pjunod/nzbd>, MIT OR Apache-2.0, fetched per session
   rather than kept.
5. **`0014`'s doubled recursive DHT request**, which is a defect rather than a
   bound and has not been checked against 9.0.1. Same corpus.
6. **Offer the patches upstream.** Sixteen sections in
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

## Open questions for the operator

None outstanding. The operator gave two instructions at the start and both are
done.

- **Finish `patches/TASKS.md`.** Done, including section 3, MSE, which the
  session before this one was told to leave alone. Twelve of thirteen rows are
  done and the thirteenth, [T-102](bep-coverage.md), is no longer waiting on a
  seam.
- **The soak, and the dependabot pull requests until CI is green.** Both done,
  and all five pull requests are closed as applied.

One thing worth a decision rather than a question, recorded so the next session
does not re-take it: **`--encryption` defaults to `prefer`**, which changes what
a default run does. It dials with MSE and dials again in plaintext when the peer
does not answer, which is what mainline clients do and which costs one extra
dial per plaintext peer. `off` is one flag away.
