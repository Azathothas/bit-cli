# Peer connections

Forty issues in the corpus touch peer handling: handshakes, connection churn,
connection limits, choke logic, and slow peers.

---

### T-020 Connections accumulate in CLOSE_WAIT until TCP is unusable

Source:      https://github.com/ikatson/rqbit/issues/311 (open)
Category:    peers
Priority:    P0
Effort:      L
Status:      **done**, 2026-08-22T14:47Z

Problem:     After about two days as a service, a reporter saw 20,000 sockets
             in CLOSE_WAIT and FIN_WAIT, which degraded TCP for the whole
             machine.
Relevance:   The netdisk deployment is exactly this shape: a long-lived process
             with many torrents. `bit-cli` is a one-shot foreground tool, which
             bounds the exposure to one invocation, but a `seed` run with
             `--seed-time 7d` is a two-day process.
Approach:    CLOSE_WAIT means the local side never called close after the peer
             sent FIN, so a task holding the socket is not being dropped.
             Reproduce with a long `bit-cli seed` run and a peer that connects
             and disconnects in a loop, watching `netstat -an` bucket counts.
             If it reproduces, the fix is upstream; carry a connection-count
             ceiling here in the meantime.
Acceptance:  A four-hour `bit-cli seed` run against a peer that reconnects
             every second ends with fewer than 100 sockets in CLOSE_WAIT,
             measured with `Get-NetTCPConnection -State CloseWait` and recorded
             here with the count at start, middle, and end.

**Reproduced, and it is two defects, not one. One is fixed here; the other is
upstream and open, with a ceiling carried here in the meantime.**

Time is not the variable, connections are, so the harness replaces four hours
with a burst.
`crates/bit-cli-core/examples/loopback-churn.rs` connects, optionally
handshakes, and closes, thousands of times.
`pwsh -NoProfile -File scripts/check-close-wait.ps1` drives it against a
seeder and counts the socket states at four moments.

```
mode         completed failed CW during CW after CW drained handles     listening panicked
handshake         2000      0         0        0          0 188 -> 228  yes       no
no-handshake      2000      0       986      986         92 188 -> 1210 yes       no
```

**Defect one: the accept loop panics and the listener silently dies.** Fixed.

`librqbit` 9.0.0's `task_listener` (`session.rs:970-1013`) is a
`tokio::select!` over two branches, both with preconditions. Accepting is
enabled only while the pending handshake-check set is under
`max_pending_incoming_handshake_checks`, and draining that set is enabled only
while it is not empty. A pending check that resolves to `Err` fails the second
branch's `Some(Ok(..))` pattern, which disables it for that iteration, and
when the set is at the cap the first branch is already disabled. Every branch
disabled panics:

```
thread 'tokio-rt-worker' panicked at librqbit-9.0.0/src/session.rs:980:13:
all branches are disabled and there is no else branch
```

A connection that closes before it handshakes is exactly that `Err`. Measured
at the 256 default: 3000 such connections at 64 at a time killed a seeder's
listener in 79 seconds, 2411 of the 3000 then failed to connect at all, and
**the process carried on reporting itself as seeding**. That is worse than a
leak, because nothing in the run says anything is wrong.

`bit-cli` sets `max_pending_incoming_handshake_checks` to `usize::MAX`
(`crates/bit-cli-core/src/engine.rs`, `PENDING_HANDSHAKE_CHECKS`). That is not
papering over it: it removes the branch that carries it, because the first
branch's precondition never goes false and the pair can never both be
disabled. The same flood now finishes in 8.8 seconds with 0 failures and the
listener alive.

**Defect two: a connection that closes before handshaking strands a socket.**
Open, upstream.

With the panic gone the residue is visible. About 0.5 stuck sockets per
no-handshake connection, and it accumulates:

| no-handshake connections | CLOSE_WAIT | handles |
| --- | --- | --- |
| 1000 | 560 | 772 |
| 2000 | 1075 | 1303 |
| 3000 | 1534 | 1776 |
| 4000 | 2053 | 2312 |
| then 100 ordinary connections | **96** | 2339 |

Three things that says:

- **Time releases nothing.** 986 stuck at the moment the churn stopped and 986
  after a 30 second settle. An earlier run held 545 flat for 107 seconds.
- **Ordinary traffic releases almost all of it.** 100 handshaked connections
  took 2053 down to 96. So it is a queue the accept loop only drains inside
  the same `select!` that accepts, not a leak: an idle seeder holds whatever
  the last burst left, and a busy one clears it.
- **A completed handshake strands nothing.** 25,000 handshaked connect and
  close cycles left the seeder holding exactly one socket, its listener, with
  the handle count flat at 228 from 10,000 onward. A handshake for an info
  hash the seeder does not have strands about 6% of the time, so the failing
  read is where nearly all of it is.

The reporter's twenty thousand after two days is this: about forty thousand
connections that never handshaked, against a seeder with too little other
traffic to drain them. Closing it means the accept loop draining its pending
set to empty rather than one item per iteration, which is upstream.

**What is carried here: `--max-handles <N>`.** Off by default. Sampled once
per `--report-interval`, and when the process holds more than that many
handles the run stops with `"stopped": "handle_ceiling"` and exit 16. It does
not close a socket. What it does is turn an unbounded stranding inside a
`seed --seed-time 7d` into a loud exit that a supervisor restarts, which is
what the Approach above asks for.

```
$ bit-cli seed t.torrent --dir . --port 0 --seed-time 30s --max-handles 50 --json
exit=16
  "open_handles": 187,
  "stopped": "handle_ceiling",
```

Status stays **open**: the stranding is not fixed, and `scripts/check-close-wait.ps1
-Ceiling 100`, which is this entry's acceptance as written, still fails. What
the script does assert, and what will now fail the run, is the listener
surviving, so defect one cannot come back unnoticed.


**What the soak adds, 2026-08-20, extended 2026-08-21.** `CLOSE_WAIT` is
**zero at every one of 1,064 samples** across a 4.605 hour `steady` run and a
4.617 hour `idle` one, with handles flat in both and exactly 189 at every
`idle` sample. So this needs the churn shape: connections that close before
they handshake. A seeder under a deployment-shaped load, with real downloads
and a tracker announcing, strands nothing over four and a half hours. See
[T-040](memory.md) for the runs.

**The stranding also stops the target serving, 2026-08-21.** This was known as
a socket count. It is worse than that: while the pending set is full the target
**cannot complete a handshake for any info hash, including one it is
serving**, and it goes on reporting itself as seeding. Found by
[T-092](bench.md)'s acceptance, which used one seeder for every case and read
as a broken handshake in `bench swarm` until the order was changed.

Three runs against one `bit-cli seed`, from
`bench/swarm-20260821T063418798Z.json`, case `listener_poisoned`:

| step | connected | handshaked | bytes |
| --- | --- | --- | --- |
| `bench swarm <T> --for p.torrent --peers 1` | 1 | 1 | 8,388,608 |
| `bench swarm <T> --peers 100 --torrents 4` | 100 | 0 | 0 |
| `bench swarm <T> --for p.torrent --peers 1` | 1 | **0** | **0** |

99 of the 100 ended in `handshake_timeout` and one in
`closed_before_handshake`, and `seeder_still_alive` is true at the end. So the
target accepts the TCP connection, answers no handshake, and never says so.

That changes what this entry costs. A stranded socket is a resource; a
listener that accepts and never answers is an outage that no health check
looking at the process, the port, or the log will see. The `--max-handles`
ceiling carried here was the only mitigation when this was written, and it now
has a second reason to exist. The check below is the second mitigation.

Reproduce:

```powershell
pwsh -NoProfile -File scripts/check-swarm.ps1
```

Case `listener_poisoned`, which carries `judged: false` because this entry is
open and an acceptance script does not fail the build for a defect that is
already recorded.

**The mechanism, and the sentence above it is wrong, 2026-08-22.** The line
"while the pending set is full the target cannot complete a handshake" names
the wrong cause. The set is never full: `PENDING_HANDSHAKE_CHECKS` is
`usize::MAX`, which is what removed defect one. The cap has nothing to do
with it, and a reader who fixed the cap would have fixed nothing.

What it is is the **drain rate**, which is one entry per accepted connection.
`task_listener`'s second `select!` arm is
`Some(Ok((live, checked))) = futs.next(), if !futs.is_empty()`
(`session.rs:1005`, the same file as defect one). A check that resolves to
`Err` fails that pattern, so `tokio::select!` disables the arm for the rest of
that call and waits on `l.accept()`, which on an idle seeder is forever. The
loop cannot come round again, and nothing in `futs` is polled until the next
connection arrives. A check that resolves to `Ok` matches, so it ends an iteration
without consuming an accept and the queued successes drain for free.

Measured, and it is one for one. Twenty connections that handshaked for an
info hash the seeder does not have, then single peers one at a time for a
torrent it does: **the twentieth got a handshake and the nineteen before it
got nothing.** `bench/listener-20260822T045550230Z.json`, case `recovery`,
`connections_to_recover` 20 against `poison_connections` 20, with a peer
served before the load to prove the seeder was working. An earlier run of the
same shape recovered on the thirteenth, and the difference is the load's own
duration: eight of that twenty ended in `closed_before_handshake`, which is
the target having already got to them. Nothing recovers on a timer. What
clears the queue is connections, one each.

**A peer row is kept for every completed handshake, and never reclaimed.**
Twenty-four handshake-and-close connections from loopback left twenty-four
rows, `live 0` and `dead 0` at every sample, all in `not needed`. That is not
a T-020 defect on its own, but it decides the shape of anything that watches
the listener by handshaking with it, and it is a candidate for the linear
slope [T-040](memory.md) is attributing.

**What is carried here, second: `--listener-check <DUR>`.** Off by default,
and on `seed` only. Not on `download`, and that is a decision rather than an
omission: the probe watches one listener, and a `-j` run has one session behind
several watch loops, so the flag would either probe once per torrent per
interval or need somewhere above the loop to live. A `download --seed-time 7d`
is the shape that would want it, and it can have it when the flag has a second
caller asking. The reason is on the `listener: None` line in
`crates/bit-cli/src/cmd/download.rs` as well, so it is not only here. Every interval it dials this run's own listen port over loopback
and completes a real handshake for a torrent the run is serving. Three
failures in a row stop the run with `"stopped": "listener_unhealthy"` and exit
17.

From the acceptance's `poisoned` case, which is a seeder given
`--listener-check 2s` and then the twenty connection load:

```
exit=17
  "stopped": "listener_unhealthy",
  "listener": { "probes": 6, "failed": 3, "consecutive_failures": 3,
                "last_failure": "handshake_timeout" }
```

Three is derived from the drain rate above rather than picked. One failure
means a backlog of at least one, which a real peer clears for itself by
arriving; three means the backlog outlived three connections, so the next
three peers get nothing either.

The probe uses a real info hash rather than an unknown one, and that costs
something. An unknown hash is rejected before the session records a peer, so
it would leave no row. It is also the wrong measurement: it resolves to `Err`,
so it **adds** an entry to the backlog it is measuring, and a backlog of one
becomes a backlog of two while the probe reports an outage on a listener that
a real peer would have got through. A completed handshake takes an entry off
instead. What it costs is one peer row per probe, and those rows
are dropped from `peer_detail` and from the report by the loopback port the
probe dialled from, which is the mechanism the web seed bridge already uses.
They come out of `peers.seen` and `peers.live` too, because `seed` exits 14
when it stopped idle having seen no peer and `--exit-when-idle` measures how
long it has had none live, and a probe a minute would answer both wrong.

Acceptance, four cases, and `recovery` is the drain-rate measurement above:

```powershell
pwsh -NoProfile -File scripts/check-listener.ps1
```

```
case      probes failed consecutive exit stopped             other
healthy        3      0           0    -  -                  peer_rows 0, peers_seen 0, rtt 15 ms
poisoned       6      3           3   17  listener_unhealthy last_failure handshake_timeout
off            -      -           -    9  deadline           no listener key at all
recovery       -      -           -    -  -                  20 connections cleared a 20 backlog
```

Status stays **open**, for the same reason as above and now for a second
mitigation rather than one. Nothing here drains the queue for a peer that is
not us, `scripts/check-close-wait.ps1 -Ceiling 100` still fails, and the fix is
still the accept loop draining its pending set to empty. What has changed is
that the outage is now loud: a supervisor gets exit 17 instead of a process
that reports a ratio and serves nobody.

**Closed 2026-08-22T14:47Z, by one match arm in the vendored tree.**

The mechanism the section above names is the whole defect, and the fix is what
that section says it would be. `task_listener`'s second `select!` arm matched
`Some(Ok((live, checked)))`. A `select!` arm whose pattern fails is disabled
for the rest of that call, so a handshake check resolving to `Err` left the
loop waiting on `l.accept()` alone. The arm now binds the whole result and
handles it inside, so no outcome can disable it. `patches/UPSTREAM.md` under
"librqbit: one failed handshake check stops the accept loop draining" carries
the diff and the reason.

**This entry's own acceptance, as written, and it had never passed:**

```
$ pwsh -NoProfile -File scripts/check-close-wait.ps1 -Ceiling 100

mode         completed failed CW during CW after CW drained handles    listening panicked ok
handshake         2000      0         0        0          0 188 -> 226 yes       no       yes
no-handshake      2000      0         0        0          0 188 -> 194 yes       no       yes

verdict: both modes ended under 100 stuck sockets with the listener alive
```

Against what the same command measured before, in the table at the top of this
entry:

| | before | after |
| --- | --- | --- |
| `no-handshake`, CLOSE_WAIT while the churn ran | 986 | **0** |
| `no-handshake`, CLOSE_WAIT after a 30 s settle | 986 | **0** |
| `no-handshake`, handles | 188 to 1210 | **188 to 194** |
| 4,000 connections, CLOSE_WAIT | 2,053 | not reproduced |

`bench/close-wait-20260822T144628230Z.json` is the run.

**And the outage, which was the worse half.** The stranding stopped the target
serving anything at all, for any info hash, while it went on reporting itself
as seeding.

```
$ pwsh -NoProfile -File scripts/check-listener.ps1
   1 connections cleared a 20 connection backlog
verdict: pass
```

| | before | after |
| --- | --- | --- |
| connections to clear a 20 connection backlog | 20 | **1** |
| probes / failed under the same load | 6 / 3 | **13 / 0** |
| the seeder under that load | exit 17, `listener_unhealthy` | still serving |

`bench/listener-20260822T144737688Z.json`.

**Three of `check-listener.ps1`'s four cases asserted the defect**, so they are
inverted rather than deleted and now hold the fix: `poisoned`, which required
exit 17, is `survives_load` and requires the run to carry on with the listener
healthy; `recovery` required more than one connection to clear the backlog and
now requires exactly one. `check-swarm.ps1`'s `listener_poisoned` case carried
`judged: false` because this entry was open, and is judged now. Both changes
mean the defect cannot come back unnoticed, which is what the cases are for.

**What that costs, said plainly.** The old `poisoned` case was the only
end-to-end proof that `--listener-check` can stop a real run with exit 17, and
there is no longer a way to poison a listener to produce one. The decision
behind the exit is covered by three unit tests in `crates/bit-cli/src/swarm.rs`:
`one_unanswered_probe_does_not_stop_a_seeder`,
`three_unanswered_probes_in_a_row_stop_the_run`, and
`an_answered_probe_clears_the_run_of_failures_before_it`. What is no longer
covered anywhere is the wiring between a real seeder's probe and a real exit.

**The two backstops carried here stay, and one of them needs its reasoning
rewritten.** `--max-handles` and `--listener-check` are both still off by
default and both still do what they did. But the threshold of three was
**derived from the drain rate**: "one failure means a backlog of at least one,
which a real peer clears for itself by arriving; three means the backlog
outlived three connections". There is no backlog now, so that derivation is
gone. Three is still the right number for a different reason, which is that a
single probe can time out on a loaded machine without the listener being
unreachable, and it is no longer measured. Said here rather than left as a
number whose stated justification no longer holds.

**What is not fixed.** A peer row is still kept for every completed handshake
and never reclaimed, which the section above notes and which belongs to
[T-040](memory.md). The `handshake` mode's 188 to 226 handles is that, not this.

### T-021 A temporary network drop stops the download permanently

Source:      https://github.com/ikatson/rqbit/issues/363 (open)
Category:    peers
Priority:    P0
Effort:      M
Status:      **done**

Problem:     Disabling and re-enabling a network adapter mid-download drops the
             rate to zero and it never recovers, even after the adapter is
             back.
Relevance:   This is the failure that makes an unattended download useless. A
             cron job that starts a 40 GB download and comes back to a stalled
             process at 60 percent has failed silently.
Approach:    `bit-cli` covers the symptom, not the cause: `--stop-timeout`
             turns a permanent stall into exit 9 with the stall named, so a
             caller can retry rather than wait forever. The cause is that
             `librqbit` does not re-announce or re-dial after every peer dies.
             Reproduce on Windows with `Disable-NetAdapter`, then decide
             whether a retry belongs in `bit-cli` (re-add the torrent to a
             fresh session and resume) or upstream.
Acceptance:  `bit-cli download <TORRENT> --stop-timeout 60s` through a
             two-minute adapter outage either recovers and completes, or exits
             9 within 60 seconds of the stall with `"stopped": "stalled"`.
             Whichever it does is recorded here with the timeline.

**It does both, and which one depends on a number nobody had looked at.**

The adapter is not the variable and cannot be touched here anyway: disabling
one is a change to the machine. What the client sees is the same either way,
every peer connection dying at once and nothing reachable for a while, so the
outage is the seeder being killed and restarted on the same port.
`pwsh -NoProfile -File scripts/check-peer-recovery.ps1` does that, twice:

```
scenario  stop-timeout exit stopped   downloaded hash    gave up after
patient   120s            0 completed 128.00 MiB matches -
impatient 20s             9 stalled   17.00 MiB  -       19.4s
```

`--stop-timeout 20s` against a 40 second outage exits 9 with `"stopped":
"stalled"` **19.4 seconds after the cut**, which is the acceptance's second
branch and inside the timeout it was given. Left alone for longer, the same
download re-dials the peer and completes with the payload hashing equal, which
is the first branch.

**What decides which is `librqbit`'s peer reconnect backoff, and it is steep.**
`torrent_state/live/peer/stats/atomic.rs:52`: 10 second minimum, **factor 6**,
one hour maximum. So a peer that drops is retried at roughly 10s, 70s, 430s,
and then 36 minutes. An outage that ends between two of those attempts waits
for the next one, however long the network has been back.

That is what makes the entry's own two-minute case look like "never recovers".
Measured directly: a 120 second outage with `--stop-timeout 180s` had the
seeder back at t+129s and the download still sat at 17.00 MiB until its stall
timeout fired at t+189s, because the next attempt was not due until t+438s. The
same shape with a 40 second outage is caught by the 70 second attempt and
completes.

So the report is accurate as an observation and wrong as a diagnosis. Nothing
is stuck. The client is waiting, and the wait grows by six every time.

**What `bit-cli` does about it.** The backoff is not reachable: it is built in
`pub(crate)` code from constants, `SessionOptions` does not carry it, and
`add_peer_if_not_seen` is `pub(crate)` and refuses a peer it has already seen,
so there is no public route to force a re-dial either. What is reachable is
saying so, and `--stop-timeout` already does: a run that cannot continue exits
9 and names the stall, which is what lets an unattended caller retry rather
than wait. `README.md` now states the interaction under "Seeding for days",
because a `--stop-timeout` shorter than the next backoff attempt turns a
recoverable outage into a failure, and that is a choice a caller has to make
deliberately rather than discover.

The residue, forcing a re-dial rather than waiting out the backoff, is
[T-138](#t-138-a-peer-that-comes-back-waits-out-a-backoff-that-grows-by-six),
and it is now **done**. `--redial-after 30s` finishes the same 120 second
outage this entry could not, in four re-dials. The paragraph above stays as
written because it is what happens with the flag off, which is the default.

### T-138 A peer that comes back waits out a backoff that grows by six

Source:      came out of closing T-021
Category:    peers
Priority:    P2
Effort:      M
Status:      **done**

Problem:     `librqbit`'s peer reconnect backoff is 10 seconds minimum with a
             factor of 6, so attempts land at about 10s, 70s, 430s, and then
             36 minutes. A peer that comes back one second after an attempt
             fails is not tried again for six times as long as the last wait.
             On a swarm of one peer, which is what `--peer` builds and what a
             private tracker often is, that is the difference between a
             download finishing and a download timing out.
Relevance:   [T-021](#t-021-a-temporary-network-drop-stops-the-download-permanently)
             measured it: a 120 second outage with the peer back at t+129s left
             the run at 17 of 128 MiB until its stall timeout fired, because
             the next attempt was due at t+438s.
Approach:    Three, none of them free.

             1. **Re-add the torrent on a stall.** `bit-cli` already knows the
                source, the output directory, and the peer list. On a stall it
                could remove the torrent from the session and add it again,
                which resets peer state and re-dials `initial_peers`. The hash
                check on add makes it safe and is what makes it expensive: a
                full read of the payload every time. Bounded by only doing it
                once per stall and by a cap on how many times.
             2. **A second session.** Heavier, same shape, no advantage.
             3. **Reach the backoff.** It is four constants in `pub(crate)`
                code. Making it configurable is the small change upstream and
                the one that fixes it properly, and it is the same fork
                question [T-002](webseed.md) priced.
Acceptance:  A 120 second outage with `--stop-timeout 300s` completes, and the
             report says how long the run waited and how many times it
             re-dialled. Today the same run exits 9 at t+189s with 17.00 MiB
             of 128, recorded under
             [T-021](#t-021-a-temporary-network-drop-stops-the-download-permanently).

**Option 1, and it turned out to cost nothing rather than a hash check.**

The entry priced option 1 as "remove the torrent and add it again", with a full
read of the payload every time. That is not what is needed. `librqbit` 9.0.0
exports `Session::pause` and `Session::unpause`, and the pair does exactly the
job:

- `ManagedTorrent::pause` on a live torrent calls `TorrentStateLive::pause`,
  which takes the piece tracker out and hands back a `TorrentStatePaused`
  holding the chunk tracker (`torrent_state/live/mod.rs:767`). The peer map and
  its backoff counters live in `TorrentStateLive` and are dropped with it.
- `Session::unpause` calls `make_peer_rx_managed_torrent(handle, true)`, which
  rebuilds the peer stream from `initial_peers`, the trackers, the DHT, and
  LSD, then `start`s the torrent (`session.rs:1511` and `session.rs:1610`).
- `Paused` to `Live` is a direct transition. Only a fresh add or an error goes
  through `Initializing`, which is the state that hash checks. So no payload is
  re-read.

So the cost is the live connections, not the disk. Option 3, reaching the
backoff constants, is still the change that fixes it at the source, and it is
still the fork question [T-002](webseed.md) priced. It is not needed for this.

**`--redial-after <DUR>`, off by default, with `--max-redials <N>` at 10.**

`bit_cli_core::engine::Engine::redial` is the pause and unpause pair.
`cmd::download::watch` calls it when the byte count has been flat for
`--redial-after` and the last re-dial was at least that long ago, checked after
the stop conditions so a run that was going to give up this tick does. Every
re-dial goes into the report as `redials[]` with the attempt number, the
milliseconds into the run, how long the run had been stalled, and how many live
peer connections it threw away, and out as a `peer_redial` event under
`--jsonl`.

Off by default because the trigger is a stall and the cost is every live
connection: a swarm where one peer is slow and the rest are working is not a
stall, but a swarm where every peer is choking is, and tearing that down every
thirty seconds is a way to make it worse. A caller who wants an unattended run
to survive an outage says how long to wait first. `bit-cli` warns when
`--redial-after` is not shorter than `--stop-timeout`, because in that order the
run gives up before it ever re-dials.

**The measurement, 2026-08-20T13:01:50.325Z**, in
`bench/peer-recovery-20260820T130150325Z.json`. Three scenarios, the first two
and the third differing in exactly one flag:

```
$ pwsh -NoProfile -File scripts/check-peer-recovery.ps1 \
    -OutageSeconds 120 -StopTimeout 60 -PatientTimeout 300 -RedialAfter 30
```

```
scenario  stop-timeout redial-after exit stopped   downloaded hash    re-dials
patient   300s         off             9 stalled   17.00 MiB  -              0
impatient 60s          off             9 stalled   17.00 MiB  -              0
redial    300s         30s             0 completed 128.00 MiB matches        4
```

`patient` is the acceptance's "today" line reproduced: 300 seconds of patience
against a 120 second outage, and it still exits 9 with 17.00 MiB of 128.
`redial` is the same run with `--redial-after 30s` and it completes with the
payload hashing equal.

The four re-dials, from the report:

| attempt | at | stalled for | peers dropped |
| --- | --- | --- | --- |
| 1 | t+38.2s | 30.1s | 0 |
| 2 | t+68.3s | 60.3s | 0 |
| 3 | t+98.4s | 90.4s | 0 |
| 4 | t+128.5s | 120.5s | 0 |

The seeder was cut at t+9.0s and came back at t+129.4s. The run finished at
t+185.0s, which is 55.6s after the peer returned and is what 111 MiB of 128 at
`--max-download-rate 2MiB/s` takes. So it resumed as soon as there was
something to resume from.

**What actually recovers it is the reset, not the re-dial.** The fourth
re-dial at t+128.5s was still during the outage, one second before the seeder
was back, so its own dial failed like the three before it. What it left behind
was a fresh `TorrentStateLive` whose backoff was back at its 10 second minimum,
so the next automatic attempt was due at about t+138.5s rather than at t+438s.
That is the whole mechanism: the flag does not have to land on the moment the
network returns, it only has to keep the wait bounded by `--redial-after` plus
10 seconds instead of letting it multiply by six.

`peers_dropped` is 0 on all four because there was nothing live to drop during
an outage. It is in the report for the case where a re-dial fires against a
swarm that is connected but not moving, which is where the cost is real.

`pwsh -NoProfile -File scripts/check-peer-recovery.ps1` is the acceptance and
now drives all three scenarios. `patient` is failed only when the outage is
inside the backoff's second attempt at about 70 seconds; past that its stalling
is what [T-021](#t-021-a-temporary-network-drop-stops-the-download-permanently)
recorded, and failing the build for it would fail the build for behaviour that
is documented. `redial` is failed whenever it does not complete, and also when
it completes without re-dialling at all, because a scenario the flag did not
change proves nothing.

Two unit tests cover the plumbing without a network:
`a_stalled_run_redials_up_to_the_cap_and_reports_each_one` holds `--max-redials`
to its cap and checks the interval between attempts, and
`a_stalled_run_without_the_flag_never_redials` checks that the report says
nothing when the flag is off.

### T-022 Peer connections churn on IPv6-only swarms

Source:      https://github.com/ikatson/rqbit/issues/537 (open)
Category:    peers
Priority:    P1
Effort:      M
Status:      **done**, 2026-08-22T17:26Z

Problem:     A session bound to `[::]` announces one address to the tracker.
             On a dual-stack host that means IPv4 peers may never learn a
             reachable address, so they connect, fail, and retry.
Relevance:   `bit-cli` binds `[::]` by default and relies on `librqbit`
             clearing `IPV6_V6ONLY` for a genuine dual-stack socket, which it
             does. The announce side is separate and still single-address.
Approach:    `bit-cli`'s own tracker client (`crates/bit-cli-core/src/tracker.rs`)
             announces one port and lets the tracker take the source address,
             which is right for one family at a time. Announcing both families
             needs two announces, one over each. Decide whether `bit-cli
             trackers` should do that, and whether the session should too.
Acceptance:  `bit-cli trackers <TORRENT> --json` on a dual-stack host reports
             the peers each family's announce returned, separately.

**The decision the Approach asks for, taken 2026-08-22 in an unattended
session.** `bit-cli trackers` announces once per family. It is a diagnostic
whose whole job is to report what a tracker said, and "which of my addresses
did this tracker take" is the question this entry is about. The session is a
separate answer and is below.

**Half of the Approach's premise is wrong, and the pinned dependency is where
to read it.** "Announcing both families needs two announces, one over each.
Decide whether the session should too" reads as though the session announces
once. For **UDP trackers it already announces twice**:
`librqbit-tracker-comms-9.0.0/src/tracker_comms.rs:374-387` resolves the first
IPv4 and the first IPv6 address into `UdpTrackerResolveResult::Two(v4, v6)` and
fires both with `tokio::join!`. For **HTTP trackers it announces once**:
`tracker_comms.rs:293` is a single `reqwest` GET and the family is whatever the
connector picks. So the session half is already done for UDP and is blocked on
`librqbit` for HTTP, at that line. That is the pinned dependency `bit-cli`
actually runs rather than a corpus tree, so it is evidence about `bit-cli`.

**What `bit-cli trackers` did before this, which was worse than either.**
`udp_target` took `to_socket_addrs().next()`, the first address the resolver
happened to return. On a dual-stack host that is not a choice, it is an
ordering, and it can differ between two runs against the same tracker.

**Built.** `Client::announce_on` takes a family, `announce` keeps the old
behaviour by passing `None`, and `bit-cli trackers` grows `--family` with
`auto`, `v4` and `v6`. `auto` resolves the tracker and announces once per
family it has an address in.

- **UDP** filters the resolution to the family and binds the local socket to
  match.
- **HTTP** overrides the resolution. `ClientBuilder::local_address` does **not**
  pin a family, which is worth recording because it is the obvious thing to
  reach for: `hyper-util-0.1.20/src/client/legacy/connect/http.rs:794-820`
  binds the local address only when it already matches the destination's family
  and otherwise falls through to the unspecified address **of the
  destination's own family**, so setting `0.0.0.0` still connects over IPv6.
  `resolve_to_addrs`, with the host resolved and filtered here, is what works,
  because then there is no address of the other family left to choose.
- The announced port is bound on **both** families now. It was IPv4 only, and
  an IPv6 announce naming a port listening only on IPv4 registers exactly the
  black hole [T-061](trackers.md) added that listener to prevent. Two separate
  listeners rather than one dual-stack socket, for
  [T-023](#t-023-the-listen-port-is-chosen-without-checking-both-address-families)'s
  reason.
- `stopped` goes out over the family the announce that succeeded used. Sent
  over the other one it names a different source address and leaves the record
  it meant to remove.

**One tracker's two announces go in sequence, and finding out why is the
measurement worth keeping.** They were concurrent first. `loopback-tracker`
keyed its peer records by peer id alone, as a plain BEP 3 tracker does, so the
second announce **overwrote** the first and one peer announcing over both
families ended up with a single record. Which family survived was whichever
announce landed last, measured at `127.0.0.1:7100` with no `[::1]:7100`:

```
one peer announces over both families on port 7100 and stays
a second peer asks what the swarm holds:
  peers: 127.0.0.1:7100
  count: 1
```

So two announces is what it takes to **tell** a tracker about both addresses,
and whether it **keeps** both is the tracker's choice. That is the whole reason
BEP 7 exists. `loopback-tracker` keys by `(peer id, family)` now, which is what
a tracker holding BEP 7's peer lists does, and it answers with `peers6` beside
`peers`. The same measurement then reads:

```
  peers: 127.0.0.1:7100, [::1]:7100
  count: 2
```

That is the entry's Problem, gone: one host, both addresses registered, and an
IPv4 peer learns a reachable one. Sequencing the two announces also makes the
outcome deterministic against the other kind of tracker, where the last family
in the list wins every time instead of a race deciding it.

**What the two families return is usually the same list, and reporting them
apart is still right.** Measured against the fixture: a peer announcing over
both is told about the same two peers either way, because what the family
decides is what the tracker records **about the announcer**, not what it hands
back. Trackers that answer only same-family peers are common enough that the
report should be able to show it, and this is what shows it.

Acceptance, run 2026-08-22 against `loopback-tracker` bound on `127.0.0.1` and
`[::1]` at one port, announcing to `http://localhost:<port>/announce` so both
families resolve:

```
=== auto (exit 0) ===
trackers=1 announces=2 responded=1 failed=0
  family v4: announces=1 responded=1
  family v6: announces=1 responded=1
  http://localhost:53414/announce family=v4 endpoint=127.0.0.1:53414 ok=True
  http://localhost:53414/announce family=v6 endpoint=[::1]:53414 ok=True
```

and the tracker's own log, which is the other side of it:

```
08:08:54.895Z announce ... from=127.0.0.1 family=ipv4 port=6881 event=started
08:08:54.907Z announce ... from=::1       family=ipv6 port=6881 event=started
08:08:54.917Z announce ... from=127.0.0.1 family=ipv4 port=6881 event=stopped
08:08:54.930Z announce ... from=::1       family=ipv6 port=6881 event=stopped
```

`--family v4` and `--family v6` each send one announce and name the endpoint,
and a family a tracker has no address in fails with the family named rather
than falling back to the other one, which would publish an address the caller
did not ask to publish.

**Closed 2026-08-22 in the vendored tree, which is what the paragraph below
said would be needed.** It said the session's half was "upstream's to make".
The trees were vendored the same day, so it was made here.

`vendor/rqbit/crates/tracker_comms/src/tracker_comms.rs` now resolves an HTTP
tracker the same way it already resolved a UDP one, keeps a `reqwest` client
per address family with the resolution pinned, and announces once over each in
sequence. `librqbit`'s session hands it a factory that rebuilds the session's
own client, so the proxy, the bound interface and the user agent are configured
in one place; behind a proxy it hands `None` and nothing changes, because the
proxy resolves and the local family is not ours to choose.
[`patches/UPSTREAM.md`](../patches/UPSTREAM.md) carries the full section.

**Measured, one `bit-cli seed` against `loopback-tracker` on both loopback
addresses at one port.** The tracker logs the source address of every announce,
which is the thing a tracker actually records about a peer:

| case | tracker URL | before | after |
| --- | --- | --- | --- |
| `dual_host` | `http://localhost:<port>/announce` | **ipv6 only**, from `::1` | **ipv4 from 127.0.0.1 and ipv6 from ::1** |
| `literal_host` | `http://127.0.0.1:<port>/announce` | ipv4, from `127.0.0.1` | ipv4, from `127.0.0.1` |

```bash
pwsh -NoProfile -File scripts/check-tracker-family.ps1
```

`bench/tracker-family-20260822T172231576Z.json` is the before, taken with the
two vendored files stashed and the tree rebuilt, and
`bench/tracker-family-20260822T172549738Z.json` is the after.

**Which family the old code picked was not a choice.** The before run says
`ipv6`, and nothing in `bit-cli` asked for that: it is the order the resolver
returned addresses in. An IPv4-only peer reading that tracker got no address it
could dial, which is this entry's Problem exactly.

**`literal_host` is the control and it has to keep passing.** A URL naming an
address has no resolution to override, so that case takes the fallback path,
which is the old code, and one announce there is correct. A check that reported
two families for both cases would be reporting that something announces twice
regardless.

**What is still one announce, deliberately.** A tracker whose host resolves in
one family only, a tracker named by address, and a session behind a proxy. Each
falls back to the client the session built, so none of them is a new path.

### T-023 The listen port is chosen without checking both address families

Source:      carried from the first session
Category:    peers
Priority:    P1
Effort:      S
Status:      done

Problem:     Probing a candidate port by binding `[::]` alone says nothing
             about IPv4 on Windows, where the standard library leaves
             `IPV6_V6ONLY` on. A port free on IPv6 and taken on IPv4 was
             reported free, and the dual-stack bind `librqbit` then makes fails.
Relevance:   It cost the whole session, not the port.
Approach:    `engine::choose_listen_addr` now requires a port to be free on
             both families before choosing it for a dual-stack listener, falls
             back to a single family with a warning naming which, and then to
             an OS-assigned port. The probe is injected, so the tests describe
             which ports are taken rather than binding sockets.
Acceptance:  `cargo test -p bit-cli-core engine::tests` passes, including
             `a_port_taken_on_ipv4_alone_is_not_chosen_for_a_dual_stack_listener`.
             Done: 2026-08-19.

### T-024 Per-peer choke and unchoke history is not reported

Source:      the operator's brief
Category:    peers
Priority:    P2
Effort:      M
Status:      open

Problem:     `bit-cli seed --json` reports per-peer address, client, direction,
             bytes in each direction, verified pieces, chunks, errors, and
             connect time. It does not report choke and unchoke events or a
             disconnect reason, because `librqbit`'s `PeerStats` snapshot does
             not carry them.
Relevance:   A3.4b names both. Without them "why did this peer stop taking
             bytes" has no answer in the report.
Approach:    `PeerCounters` in `librqbit` carries `times_stolen_from_me` and
             `times_i_stole` but no choke history and no disconnect cause. Add
             them upstream, or infer disconnects from a peer leaving the
             snapshot between two ticks and record that as the weaker answer it
             is.
Acceptance:  `bit-cli seed --json` carries a `disconnects` array per peer with
             a timestamp and a reason, and the reason is a real one rather than
             "gone".

### T-025 PeerStatsFilterState is not exported, so the filter is built by JSON

Source:      `librqbit` 9.0.0 API gap
Category:    peers
Priority:    P3
Effort:      S
Status:      **done**, 2026-08-22T19:38Z

Problem:     `librqbit` exports `PeerStatsFilter` through `http_api_types` but
             not the enum its one field holds, so the value that asks for every
             peer rather than only the connected ones cannot be named in Rust.
Relevance:   `bit-cli` needs every peer, including ones that took two gigabytes
             and left. It builds the filter through the type's own
             `Deserialize` from a fixed literal, which works and reads badly.
Approach:    One line upstream: re-export `PeerStatsFilterState` alongside
             `PeerStatsFilter`. Until then the literal is pinned by a comment
             at `engine::all_peers_filter`.
Acceptance:  `engine::all_peers_filter` constructs the filter with a named
             enum variant and no JSON.

**Closed 2026-08-22, and it was the one line the Approach said it was.**
`http_api_types` re-exports `PeerStatsFilterState` beside `PeerStatsFilter` in
the vendored tree, and `all_peers_filter` is

```rust
PeerStatsFilter {
    state: PeerStatsFilterState::All,
}
```

with no `serde_json`, no literal, and no `unwrap_or_default` whose fallback
would have quietly narrowed the report to live peers if the literal had ever
stopped parsing. Worth doing because it is the smallest possible demonstration
of what owning the fork is for: this sat open as an upstream API gap while the
fix was one line in a file this repository now ships.

### T-142 bit-cli peers never joined the swarm it was sampling

Source:      found building [T-117](cli-surface.md)'s `peers` fixture
Category:    peers
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `bit-cli peers` added its torrent with `paused: true`, and
             `librqbit` 9.0.0 hands a torrent its peer stream only when it
             starts: `ManagedTorrent::start` takes `peer_rx` and documents
             that it must be set unless `start_paused`. So the command never
             announced, never dialled, and reported an empty swarm however
             long it watched. Every run said `seen: 0`, `peers: []`, and exit
             9.
Relevance:   P1 by the definition in [INDEX.md](INDEX.md): a documented
             capability that does not work. `README.md` says "Connect, sample
             the swarm, report peers, exit" and the command could not do the
             first of those.
Approach:    Start the torrent. The comment said paused "keeps the torrent
             connected to the swarm for peer discovery without pulling any
             payload", and neither half of that was true.
Acceptance:  A seeder on loopback and `bit-cli peers` pointed at it report
             that peer, with the bytes that came from it.

**Done, with two other gaps closed alongside it because the fix could not be
proven without them.**

The measurement that found it, 2026-08-20T16:15Z. `loopback-tracker` logs
every announce it gets, one seeder was serving, and the tracker saw exactly
one client:

```
16:15:20.141Z announce ... port=55502 left=0 event=started -> 0 peer(s)     the seeder
                                                                            nothing from `peers`
16:16:11.238Z announce ... port=57261 left=2000 event=started -> 1 peer(s)  `download`, for contrast
```

`bit-cli peers ... --duration 10s` between those two announces reported
`seen: 0` with the seeder up and registered. `bit-cli download` against the
same torrent announced and found it.

**Nothing selected is not the same as nothing wanted.** The first fix tried
was `paused: false` with `only_files: Some(vec![])`, which announces and
dials and still pulls no payload. Measured against the loopback seeder, that
reports the peer and nothing else: `state: "not needed"`, `errors: 1`,
`downloaded_bytes: 0`, and no client string, because neither side wants
anything from the other and the connection is dropped on the handshake. With
the file selection left alone the same fixture reports
`downloaded_bytes: 2000`, `verified_pieces: 2`, `chunks: 2`,
`mean_piece_ms: 10`, and `errors: 0`.

The report is built on the second of those. `--sort speed` orders peers by
bytes that arrived, and `PeersReport` carries `downloaded` and per-peer
`downloaded_bytes`, so a sample that transfers nothing cannot answer what the
command is asked. What the sample pulls goes to a temporary directory that the
process removes when it exits, which is unchanged, and `--duration`,
`--count`, and now `--max-download-rate` are what bound how much moves.

**The command could not be driven offline, which is why this survived.**
`peers` built `TrackerArgs::default()`, hardcoded `no_dht: false` and
`no_lsd: false`, and had no `--peer`. So it could not be pointed at a known
peer, could not be told to stay off the DHT, and could not be tested without
the network. It now flattens `TrackerArgs` and `LimitArgs` and takes `--peer`,
`--no-dht`, and `--no-lsd`, which is the same set `download` and `seed` carry.
`peers --peer <ADDR> --no-tracker --no-dht --no-lsd` samples a swarm of
exactly the members named on the command line and reaches nothing else.

BEP 27 was never at risk here: `librqbit` builds neither the DHT nor the LSD
receiver for a private torrent, in `session.rs` around line 1537, whatever the
session's own settings say.

`cmd::peers::tests::a_sampled_swarm_carries_what_came_from_each_peer` is the
regression test: a real seeder on a thread, `--peer` pointed at it, and
assertions on the bytes that arrived and on the working directory being left
empty. It fails on the old code with `seen: 0`.

```
$ cargo test -p bit-cli --lib peers
test result: ok. 11 passed; 0 failed
```

---

### T-163 MSE/PE peer encryption is not implemented

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    peers
Priority:    P2
Effort:      L
Status:      open

Problem:     `bit-cli` speaks plaintext BitTorrent only. There is no MSE/PE
             (message stream encryption, protocol encryption) in the tree, and
             no way to require it, prefer it, or accept it.
Relevance:   This is an interoperability cost before it is a privacy feature.
             A peer configured to **require** encryption will not exchange
             traffic with a plaintext-only client at all, which superseedr
             [Issue 297](https://github.com/Jagalite/superseedr/issues/297)
             states plainly from the other side of the same gap. So the swarm
             `bit-cli` can reach is smaller than the swarm that exists, and
             nothing in the output says so.
Approach:    Three sources, in the order they are worth reading.

             `mtorrent/mtorrent-core/src/pe/` is the cleanest standalone
             implementation. `key_exchange.rs` carries the 768-bit MSE DH
             prime with generator 2 and `KEY_SIZE = 96`.
             `mtorrent/mtorrent-core/src/pe/handshake.rs:12-17` fixes
             `MODE_PLAINTEXT = 1`, `MODE_RC4 = 2`, `MODE_ANY = 3`,
             `MAX_PADDING_LEN = 512` and `VC_LEN = 8`, with `max_pe3_len` and
             `max_pe4_len` just below so a reader can bound its buffers. `:41`
             `outbound_handshake` and `:164` `inbound_handshake` are the two
             directions.

             `mtorrent/mtorrent-core/src/pe/utils.rs:17` `detect_encryption`
             is the piece that matters most for `bit-cli`'s shape. It reads
             exactly `PROTOCOL_STRING.len()` bytes, compares, and returns the
             stream **with those bytes pushed back**, so one listening port
             serves plaintext and encrypted peers with no second port and no
             mode flag.

             `nanotorrent` is the librqbit-specific route, and it is the one
             that matters, because `bit-cli` builds on librqbit and does not
             fork it. Patches `0003-stream-transform-seam.patch` and
             `0005-incoming-stream-transform-seam.patch` add a
             `StreamTransform` trait plus `SessionOptions::stream_transform`
             for outgoing streams, and an `IncomingStreamTransform` for the
             accept path. The non-obvious half is in 0005: the incoming
             transform is handed **every active info hash**, because the hash
             is not known until the possibly-encrypted handshake has been
             read, and the MSE responder resolves the peer's SKEY against
             them. `nanotorrent/src/bittorrent/mse.rs` is 819 lines of
             implementation against those two seams, and its module doc states
             the policy choice outright: RC4 only, advertise only RC4 in
             `crypto_provide`, drop a peer that will not do RC4, because that
             is what "require encryption" means.
Blocker:     The seams do not exist in `librqbit` 9.0.0. This is the same wall
             [T-002](webseed.md) measured and [T-102](bep-coverage.md)
             records: the connect and accept paths and `PeerConnectionHandler`
             are implemented inside `librqbit` by the torrent state, not by
             anything a dependent crate can supply. What would unblock it is
             two upstream visibility changes of the shape nanotorrent's 0003
             and 0005 make, or a vendored `librqbit`, which decision 7.3 does
             not take. It stays open with the cost named.
Acceptance:  A `bit-cli download` against a peer configured to require
             encryption completes, and the same run against the same peer with
             encryption off completes too, from one listening port with no
             mode flag. `--encryption off|prefer|require` reports which mode
             each peer settled on in `--json`. Both runs recorded here.

### T-164 A peer that sends garbage keeps its connection slot

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    peers
Priority:    P2
Effort:      M
Status:      **partial**. Part 1, `--block-peer`, done 2026-08-22T02:20Z.
             Parts 2 and 3 blocked on `librqbit` 9.0.0, both named below.

Problem:     `bit-cli` has `--web-seed-fatal-status` and
             `--web-seed-max-errors`, so an HTTP source that misbehaves is
             retired and stays retired. There is no equivalent for a peer. A
             peer that fails a piece hash, sends a malformed message, or
             breaks the protocol is dropped and then redialled.
Relevance:   vortex [Issue 125](https://github.com/Nehliin/vortex/issues/125)
             is that failure with the crash already fixed: once the process
             stopped dying on the malformed response, the same peer
             reconnected and kept sending garbage, burning a connection slot,
             and the DHT rediscovered it **every 20 seconds**. Fixing a crash
             without adding a blocklist turns a hard failure into a slow one,
             which is harder to diagnose. The asymmetry is the argument on its
             own: `bit-cli` already decided that a source which misbehaves gets
             retired, and applies that decision to only one of its two kinds
             of source.
Approach:    The proposal in that issue is the shape. Auto-block on a protocol
             violation, check the blocklist **before completing a handshake**
             rather than after, and expose add, remove and query. Persistence
             is optional there, which suits `bit-cli`: decision 7.4 allows no
             state file, so the blocklist lives for the invocation and
             `--block-peer <ADDR>` covers the case a user wants to carry
             across runs.
             `aria2_rust/aria2-core/src/engine/bt_peer_storage/` holds a
             `rejection_state.rs` with blocklist tests beside it, for a second
             opinion on the bookkeeping.

             What makes a violation attributable rather than guessed is
             [T-179](webseed.md), smart ban: with several sources filling one
             piece, a failed hash names a set of peers and not one peer. Build
             that first, or this blocks whichever peer is convenient.

             **T-179 is done, and it built the half that is not peer-specific.**
             `webseed/ledger.rs` records a hash of every block against whoever
             supplied it and convicts every supplier whose hash differs from
             the bytes the session went on to verify, reading those bytes back
             off the disk rather than fetching them again. It is keyed on a
             `usize` source index rather than a URL for this entry's sake, so a
             peer key fits without changing the type. What is missing on the
             peer side is the recording hook: `bit-cli` sees a bridge put a
             block on the wire and does not see a peer's block arrive, because
             that path is inside `librqbit`. Name that seam here before pricing
             this, the way [T-167](bep-coverage.md) had to.
Acceptance:  A synthetic peer that fails a piece hash twice is not redialled
             for the rest of the run, `bit-cli peers --json` names it with the
             reason, and the freed slot measurably goes to another peer.
             `bench swarm` drives it, because it already builds peers that
             misbehave on purpose.

**The seam is named, 2026-08-22T02:10Z, and it splits this entry into three
parts rather than one.** Read before writing any of it, which is what the
paragraph above asked for. Two of the three are blocked and one is not blocked
at all, which is not what "effort M, blocked on a librqbit seam" would have
said.

### 1. A blocklist exists upstream, and it is checked in exactly the right place

This is the part the entry did not know. `librqbit` 9.0.0 has a blocklist and an
allowlist, and both are consulted in both directions:

- **Incoming**, `session.rs:917`, `if self.blocklist.has(incoming_ip)`, and it
  is above the `read_handshake` at `:934`. That is the vortex proposal's
  "check the blocklist **before completing a handshake**", already true.
- **Outgoing**, `torrent_state/live/mod.rs:629`, in the peer-stream loop, before
  a permit is taken or a connection task is spawned.
- Both bump `session_stats` counters, `blocked_incoming` and `blocked_outgoing`.

`SessionOptions::blocklist_url` (`session.rs:461`) is how it is populated, once,
at `Session::new_with_opts` (`session.rs:739-748`). `IpRanges::load_from_url`
(`ip_ranges.rs:61`) takes a **`file:` URL** as well as an HTTP one
(`ip_ranges.rs:64-70`), and the format is PeerGuardian's: `name:start-end` per
line, `#` for a comment, plain or gzip, parsed at `ip_ranges.rs:152`.

**So `--block-peer <ADDR>` is not blocked.** `bit-cli` writes the ranges it was
given to a scratch file and points `blocklist_url` at it before the session
starts. `cmd::peers` already makes a `tempfile::tempdir()` per invocation, so
the pattern exists and decision 7.4 is not touched: this is a scratch file for
the length of one process, not state anything reads back.

### 2. Adding to that blocklist during a run is blocked, and it is a near miss

`Session.blocklist` (`session.rs:141`) is a plain `IpRanges` field, not a lock
and not an `ArcSwap`. `bit-cli` holds an `Arc<Session>` through
`Engine::session`, so there is no `&mut` to be had and no interior mutability to
use.

`IpRanges::new` (`ip_ranges.rs:47`) is `pub` and takes the ranges directly, so
the value could be built. It cannot be named: `lib.rs:60` declares
`mod ip_ranges;` with no `pub`, so `pub` inside it reaches nothing outside the
crate. That is the same shape as [T-167](bep-coverage.md)'s `update_bitfield`,
and it is recorded here for the same reason: so nobody re-derives it.

### 3. Attributing a bad piece to the right peer is blocked, and upstream
### already gets it wrong

This is the half [T-179](webseed.md) built for HTTP sources, and the seam is
`TorrentStorage`.

`file_ops.rs:310`, `write_chunk(&self, who_sent: PeerHandle, data, chunk_info)`,
**has** the peer: `PeerHandle` is `SocketAddr` (`type_aliases.rs:13`). It drops
it one line later. The trait `bit-cli` implements is
`storage/mod.rs:136`, `pwrite_all_vectored(&self, file_id, offset, bufs)`, and
there is no peer in it. `SafeStorage` therefore sees every byte a peer sends and
never sees who sent it. `mod file_ops;` and `mod torrent_state;` are both
private, so there is no second place to look.

And `librqbit` already convicts a peer, incorrectly.
`torrent_state/live/mod.rs:1965-1972`: when `check_piece` returns false it warns
with `?addr`, marks the piece failed, and

```rust
anyhow::bail!("i am probably a bogus peer. dying.")
```

which drops the connection of whichever peer delivered the **last** chunk of
that piece. With several peers filling one piece that is the peer that finished
it, not the peer that broke it. That is exactly the wrong answer T-179 was
written to stop giving, present upstream, and it is why smart ban for peers
cannot be built beside `librqbit`: the conviction happens inside it, before
anything `bit-cli` owns is told.

`webseed/ledger.rs` is still the right machinery and still fits. It is keyed on
a `usize`, and a `SocketAddr` maps to one through a table `bit-cli` would keep.
What is missing is the one call that would fill it.

**What would unblock parts 2 and 3**, smallest upstream change first:

1. `TorrentStorage` gains a `who_sent: Option<PeerHandle>` on the write methods,
   or a separate `fn on_chunk_written(&self, who_sent, file_id, offset, len)`
   with a default empty body. `write_chunk` already holds the value; this is
   passing it on. That alone unblocks part 3 and lets `bit-cli` convict the
   right peer with the ledger it already has.
2. `Session.blocklist` becomes an `ArcSwap<IpRanges>` or a `RwLock`, with a
   `Session::block_ip` beside it, and `pub mod ip_ranges`. That unblocks part 2.
3. Failing 1, `librqbit` stops convicting on the last chunk and takes a
   per-block record of its own. That is the larger change and it is upstream's
   to want.

**Re-priced.** Part 1 was effort S and is **done, 2026-08-22T02:20Z**. Parts 2
and 3 stay open and blocked, with the lines above as the blocker. The entry
keeps its P2 and stays at the height of its value, which is the rule in
[INDEX.md](INDEX.md).

### Part 1, as built

`--block-peer <ADDR>` on `download`, `seed` and `peers`, because it lives in
`LimitArgs` and every command that has a session has those. It takes an
address, an inclusive `START-END` range, or a CIDR block, in either family.
`swarm::blocked_ranges` parses it. Three decisions in it are worth stating:

- **A `HOST:PORT` is refused**, with the address to write instead. The session
  blocks an address, so silently dropping the port would block every port on
  that host without saying so.
- **Nothing is resolved.** `--peer` takes a name because a caller naming a peer
  wants to reach it. A blocklist entry that resolved would block whatever the
  name pointed at when the run started, which is not what a block means.
- **A `/0` and a `/32` are both exact**, because a shift by the full width is
  undefined and the widest block is the one a caller reaches for to test the
  flag.

`Engine::start` writes the ranges to a scratch file in PeerGuardian format and
points `blocklist_url` at its `file:` URL. The file is a `NamedTempFile` held
for that one call and deleted when it drops, so decision 7.4 is untouched: it
is not state, nothing reads it back, and a run that blocks nothing writes no
file at all.

**Measured**, against `target/release/bit-cli`, one loopback seeder holding an
8 KiB payload, the same command twice:

```
$ bit-cli peers blk.torrent --peer 127.0.0.1:51955 --no-tracker --no-dht --no-lsd --duration 4s --port 0
live 0  connecting 0  queued 0  seen 1  dead 0
ADDRESS          STATE       DIR       DOWN      PIECES
127.0.0.1:51955  not needed  outgoing  8.00 KiB  8

$ bit-cli peers blk.torrent --peer 127.0.0.1:51955 --block-peer 127.0.0.1 ...
live 0  connecting 0  queued 1  seen 1  dead 0
blocked              0 incoming, 1 outgoing
ADDRESS          STATE   DIR       DOWN  PIECES
127.0.0.1:51955  queued  outgoing  0 B   0
```

8 KiB and eight pieces against the peer, or nothing and a refusal counted. The
number the flag moves is `blocked_outgoing`, which is the session's own counter
rather than one this tree keeps, read through `Api::api_session_stats`. It is
reported as `blocked` on `peers`, absent when nothing was refused so an
ordinary sample carries no extra field, and it is in `docs/schema.md`.

**`seen` counts a blocked address, and that is recorded rather than
corrected.** `task_peer_adder` registers the address when it is queued and
checks the blocklist when it takes it off the queue
(`torrent_state/live/mod.rs:629`), so a blocked peer sits at `queued` for the
whole run with nothing against it. Subtracting a refusal count from a peer
count would be arithmetic nobody can check: the counter counts refusals, not
addresses, and one address refused twice moves it by two. The two numbers are
reported side by side instead.

Six tests. `a_blocked_peer_is_never_dialled_and_never_joins_the_swarm` is the
acceptance and uses the same loopback-seeder rig as
[T-142](#t-142-bit-cli-peers-never-joined-the-swarm-it-was-sampling)'s.

### T-165 The peer's reqq is ignored, so the queue depth is a fixed 128

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    peers
Priority:    P2
Effort:      S
Status:      open

Problem:     A peer's BEP 10 extended handshake carries `reqq`, the number of
             block requests it will queue. `bit-cli bench leech` reports a
             queue depth of 128 whatever the peer said, and nothing reads the
             advertised value.
Relevance:   mtorrent [Issue 17](https://github.com/DanglingPointer/mtorrent/issues/17)
             carries the whole argument: exceeding `reqq` either wastes every
             request past the limit or gets the connection dropped, depending
             on the peer, and both look like a slow peer from this side. It
             also makes a number `bench leech` prints wrong rather than merely
             unbounded, which matters more here than upstream, because that
             number is evidence under [T-041](memory.md) and
             [T-018](disk-io.md). A fixed constant reported as a measurement is
             the mistake [T-032](performance.md) and [T-141](webseed.md) both
             closed by disproving.
Approach:    `vortex/bittorrent/src/peer_comm/extended_protocol.rs:60`
             `extension_handshake_msg` shows `reqq` beside `m`, `v`, `p`,
             `metadata_size` and `upload_only` in the same handshake
             `bit-cli`'s bridge already builds, so the field is one key away on
             the send side. On the receive side the value bounds the pipeline.
             `seedchamp/docs/design.md:197` is what to bound it *with*: a
             BDP-sized depth from an EMA of that peer's own wire rate,
             `desired = 5 s * rate / 16 KiB`, capped rather than fixed, with a
             20 s request stall and 4 s in endgame. The bridge is the place to
             start because it is `bit-cli`'s own peer implementation. The
             session side needs `librqbit`.
Acceptance:  `bench leech` reports the peer's advertised `reqq` and the depth
             actually used, and the two agree when the peer advertises less
             than the cap. A synthetic peer advertising `reqq = 8` receives no
             more than 8 outstanding requests, asserted in a test rather than
             observed in a report.

### T-166 BEP 10 extension ids are not proven to map in both directions

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    peers
Priority:    P1
Effort:      S
Status:      **done**

Problem:     The web seed bridge implements BEP 10 (`webseed/bridge.rs:83`,
             `:708`) and nothing in the tree asserts that it keeps **our**
             extension ids and **the peer's** apart. They are two independent
             numberings.
Relevance:   vortex [PR 103](https://github.com/Nehliin/vortex/pull/103) is
             the best interop finding in the corpus and it is exactly this
             mistake. The extension map was keyed by the local id and then
             tested against the peer's: `if self.extensions.contains_key(&id)
             { continue; }`. When qBittorrent assigned `ut_metadata = 2` and
             the local side used `1`, incoming id 2 was skipped as "already
             initialised", because the local `upload_only` happened to be 2.
             The stated consequence is that extensions had never once worked
             against qBittorrent. A defect of this shape is silent, is
             invisible against any peer that happens to number its extensions
             the same way, and `bit-cli`'s bridge sits on both ends of a
             loopback pair in every test it has, which is precisely the
             arrangement that hides it.
Approach:    The rule is one sentence: map **peer id to handler** in one
             direction and **name to our id** in the other, as two separate
             tables, and never index one with the other's key. Read the bridge
             against that rule, then write a test whose peer deliberately
             numbers `ut_metadata` and `upload_only` differently from the
             bridge and asserts both are routed.

             Two ordering rules from the same repository are worth asserting
             while that test is being written.
             [PR 156](https://github.com/Nehliin/vortex/pull/156): messages
             arriving in the same TCP read as the handshake were processed
             before the bitfield was queued, so `Interested` could precede
             `Bitfield`, and **the bitfield must be the first message after
             the handshake**. `webseed/bridge.rs:674` already says the order
             matters; a test is what keeps it true.
             [PR 155](https://github.com/Nehliin/vortex/pull/155) is the
             `Have` handling for peers without BEP 6.
Acceptance:  A test in which the peer's extension numbering differs from the
             bridge's, the bridge routes an incoming `ut_metadata` and an
             incoming `upload_only` to the right handlers, and the first
             message after the handshake is asserted to be the bitfield.

**Read against that rule, the bridge had neither table, and the missing one
cost a connection.** The premise of this entry needed correcting before the
test could be written, and the correction is what found the defect.

`bit-cli`'s bridge advertises an **empty** `m` (`webseed/bridge.rs`
`extended_handshake`), which is the honest thing: it seeds and implements no
extension messages. So there is no "name to our id" table, and because every
extension message fell through the receive loop's catch-all there was no "peer
id to handler" table either. A map keyed the wrong way round, which is the
literal vortex PR 103 defect, could not exist here because there was no map.

**What did exist is the same mistake one level down.** The receive loop called
`Message::deserialize`, and `librqbit-peer-protocol` 9.0.0 routes an incoming
extension id against **its own** constants:
`MY_EXTENDED_UT_METADATA = 3` at `librqbit-peer-protocol/src/lib.rs:52` and
`MY_EXTENDED_UT_PEX = 1` at `:55`, dispatched at `src/extended/mod.rs`. Those
are the ids that crate advertises. This bridge
advertises neither, and it was still reading incoming ids through them. That is
an incoming id looked up in a table the two ends never agreed on, which is
exactly the direction confusion this entry names.

The cost is a dropped connection. `UtMetadata::deserialize` refuses a body that
is not a ut_metadata message, `ExtendedHandshake` refuses one with no `m`, and
a deserialize error becomes `BridgeError::Link`, which ends the connection and
starts the reconnect backoff. Measured across the whole id space, with the fix
reverted:

```
EXT ID 0: LINK DIED: early eof      <- decoded as an extended handshake
EXT ID 1: link survived             <- decoded as ut_pex; an empty dict happens to parse
EXT ID 2: link survived
EXT ID 3: LINK DIED: early eof      <- decoded as ut_metadata
EXT ID 4: link survived
EXT ID 7: link survived
EXT ID 9: link survived
EXT ID 200: link survived
```

Two ids out of the sample, and both of them `librqbit`'s. Every id the bridge
had actually advertised, which is none of them, was fine. **Id 1 surviving is
the more instructive result**: it was decoded as `ut_pex` too, and it lived
only because an empty bencode dictionary happens to satisfy that type. It was
never routed correctly, it was routed to the wrong type and got away with it.
That is the silence this entry predicted.

**Fixed by deciding the question against our own map and nowhere else.**
`OUR_EXTENSIONS` is the table of `(name, our id)` pairs the bridge advertises,
`is_our_extension` is the only thing that reads it, and the receive loop drops
an extension frame whose id is not in it before `Message::deserialize` ever
sees the bytes. The table is empty today and the wire form says the same thing,
which a unit test asserts as one claim: an empty table and an empty `1:mde` are
the same statement, so an entry added to one without the other fails.

That is also the seam [T-167](bep-coverage.md) needs. `lt_donthave` adds one
entry to `OUR_EXTENSIONS` and one handler, and the receive direction is right
by construction because the lookup is against the advertised map. The **send**
direction is the second table and does not exist yet: it has to be read out of
the peer's own extended handshake, and T-167 is the first thing that will need
it, because the bridge is the end that sends `lt_donthave`.

**The test is a session written by hand, which is what this entry was for.**
`crates/bit-cli-core/tests/bridge_protocol.rs` speaks the peer protocol byte by
byte, declares the message ids as its own constants rather than importing the
bridge's, and never calls the serializer the bridge calls. Nothing in it can
agree with the bridge by construction. Every other bridge test puts a real
`librqbit` session on the far end, and both ends of that pair number their
extensions identically, which is the arrangement the entry named as the one
that hides this.

The session advertises `ut_metadata = 2`, `upload_only = 4`, `lt_donthave = 7`,
none of which is `librqbit`'s number for any of them, and then sends messages
under those ids **and** under 1 and 3. `no_extension_id_can_end_the_connection`
walks all 256 ids on one connection, then sends a well-formed `ut_metadata`
request under id 3, which is precisely what a peer that got the direction
backwards would send. The assertion in both is behavioural: after all of it the
bridge still answers a `request` with the source's bytes at the offset the
request named.

**On the ordering rule, PR 156 is right and its one-line summary is not the
rule here.** vortex's finding is that a message arriving in the same TCP read
as the handshake was processed before the bitfield had been queued, so
`Interested` could precede `Bitfield`. `bit-cli`'s bridge writes the extended
handshake, the bitfield and `unchoke` as one concatenated buffer in a single
`write_all`, before the receive loop starts, so nothing can interleave with
them. The order on the wire is extended handshake, bitfield, unchoke, and the
extended handshake being first is deliberate rather than an exception: BEP 10
puts it in the handshaking sequence, and it is what carries the BEP 21
`upload_only` flag that tells the session it is looking at a partial seed
rather than a leecher. `the_bitfield_precedes_every_peer_message_after_the_handshake`
asserts that reading, which is the rule that survives contact with a peer that
also speaks BEP 10: **no ordinary peer message precedes the bitfield.**

PR 155, `Have` handling for peers without BEP 6, is not applicable. The bridge
sends a bitfield and then never revises it, so it sends no `Have` at all, and
it ignores every `Have` the session sends because it only seeds. That changes
with [T-167](bep-coverage.md), which is the first message the bridge will send
to revise what it holds.

**Proven by reverting the fix.**

```
$ cargo test -p bit-cli-core --test bridge_protocol    # with the frame skip removed
test the_bitfield_precedes_every_peer_message_after_the_handshake ... ok
test no_extension_id_can_end_the_connection ... FAILED
test a_peer_that_numbers_its_extensions_differently_is_still_served ... FAILED
test result: FAILED. 1 passed; 2 failed

$ cargo test -p bit-cli-core --test bridge_protocol    # with the fix
test the_bitfield_precedes_every_peer_message_after_the_handshake ... ok
test a_peer_that_numbers_its_extensions_differently_is_still_served ... ok
test no_extension_id_can_end_the_connection ... ok
test result: ok. 3 passed; 0 failed

$ cargo test -p bit-cli-core --lib webseed::bridge
test webseed::bridge::tests::an_incoming_extension_id_is_only_read_against_our_own_map ... ok
test result: ok. 17 passed; 0 failed
```

**One note on how nearly this stayed hidden.** The first draft of the
hand-written session sent a malformed extended handshake: two bencode string
lengths were wrong, `12:lt_donthave` for an eleven byte name and `9:fake/1.0`
for an eight byte value. Every id "died", which reads as a much larger defect
than the real one. The lesson is the one [RULES.md](RULES.md) already carries
from T-032 and T-141: the first reading was of the fixture rather than of the
thing. The fixture is now the part of this test worth reading twice.


### T-194 A torrent past 131,960 pieces cannot be served or fetched at all

Source:      [rqbit#637](https://github.com/ikatson/rqbit/issues/637), item 0 of
             `patches/TASKS.md`, measured 2026-08-22
Category:    peers
Priority:    **P0**
Effort:      M
Status:      **done**, 2026-08-22T13:52Z, with a residual ceiling in
             [T-195](peers.md)

Problem:     `Message::Bitfield` is serialized into the fixed per connection
             write buffer, which is `MAX_MSG_LEN` bytes. A bitfield is one bit
             per piece, so its length is a property of the torrent and not of
             the protocol. Past 131,960 pieces it does not fit, `serialize`
             returns `NoSpaceInBuffer`, and the connection is dropped before a
             single piece is served. Both directions fail: a seeder cannot
             answer, and a leecher fetching metadata for such a torrent by
             magnet never resolves it.
Relevance:   This is not a slowdown. A torrent past the threshold does not
             work at all, in either role, against any peer. Nothing in
             `bit-cli` reported it as anything: the seeder logged
             `error managing peer: not enough space in buffer` at DEBUG and
             carried on, and the leecher waited.
Approach:    Stop routing the bitfield through the shared fixed buffer. The
             handler sizes its own buffer, because only it knows the piece
             count. `Message::bitfield_message_len` is the one thing the
             protocol crate has to expose for it.
Acceptance:  A torrent above the old threshold resolves by magnet from a local
             seeder and its file is created.

**Where the number comes from.** `MAX_MSG_LEN` is 16,500 bytes, built in
`peer_binary_protocol/src/lib.rs` for a `ut_metadata` data message: a 16,384
byte chunk plus its bencode header plus 64 bytes of slack. A bitfield message
is `5 + ceil(pieces / 8)` bytes, so it fits while `ceil(pieces / 8) <= 16,495`,
which is 131,960 pieces. The comment above the constant said the `ut_metadata`
request was "the largest known message", and that was the whole mistake.

**Measured, and it is exact to one piece.** Every case is a torrent of 1 KiB
pieces, seeded on loopback with trackers and DHT off, fetched by magnet by a
second process given only `--peer 127.0.0.1:<port>`:

| pieces | `.torrent` | bitfield | before | after |
| --- | --- | --- | --- | --- |
| 131,952 | 2,639,179 B | 16,499 B | resolves | resolves |
| **131,960** | 2,639,339 B | **16,500 B** | **resolves** | resolves |
| **131,961** | 2,639,359 B | **16,501 B** | **no space in buffer** | resolves |
| 131,968 | 2,639,499 B | 16,501 B | no space in buffer | resolves |
| 163,840 | 3,276,939 B | 20,485 B | no space in buffer | resolves |

The two middle rows are one piece apart and 16,500 is `MAX_MSG_LEN` exactly.

**The `.torrent` size is a red herring, and that matters for the upstream
report.** rqbit#637 is titled "rqbit faill to add torrent larger than 2MB" and
has an empty body. Both 2.64 MB torrents in the table above are "larger than
2MB" and one of them works, so the size of the file is not the variable. The
piece count is. A 2 GiB payload at 16 KiB pieces makes a 2,621,581 byte
`.torrent` with 131,072 pieces, and that one seeds, verifies and downloads
fine. Whether the upstream report is this defect cannot be established from an
empty issue body; it is the same neighbourhood and the same order of magnitude,
and that is as far as the evidence goes.

**Adding is not what fails.** `bit-cli create`, `info`, `verify` and `seed` all
handle a 3.13 MiB `.torrent` with no trouble, and `create` builds one from
160 MiB of payload in 0.195 s. Item 0 of `patches/TASKS.md` asked whether
`bit-cli` could make such a fixture quickly enough to test with, and it can.
What fails is the wire.

**The fix**, in `patches/UPSTREAM.md` under "librqbit: a bitfield larger than
MAX_MSG_LEN cannot be sent":

- `PeerConnectionHandler::serialize_bitfield_message_to_buf` takes a
  `&mut Vec<u8>` rather than a `&mut [u8]`, so the implementor sizes it.
- The send site uses a buffer of its own rather than the shared `write_buf`,
  allocated once per connection and dropped after the bitfield is written.
- `Message::bitfield_message_len` is the exact length `serialize` needs.

```
$ pwsh -NoProfile -File scripts/check-bitfield.ps1
bitfield: 163840 pieces, 3276939 B torrent, metadata resolved, file created
bitfield: ok
```

Upstream's own tests still pass, 139 of them, and the new one is
`test_bitfield_larger_than_max_msg_len` in `peer_binary_protocol`.

### T-195 The read side caps the same message at 262,104 pieces

Source:      measured while closing [T-194](peers.md), 2026-08-22
Category:    peers
Priority:    P2
Effort:      M
Status:      **done**, 2026-08-22T18:57Z

Problem:     `ReadBuf` is a ring buffer of `BUFLEN`, 32,768 bytes, in
             `vendor/rqbit/crates/librqbit/src/read_buf.rs:12`. A message that
             cannot fit in it fails with `read buffer is full`. For a bitfield
             that is `5 + ceil(pieces / 8) <= 32,768`, which is 262,104 pieces.
Relevance:   [T-194](peers.md)
             moved the send side off a fixed buffer entirely, so this is now
             the binding limit and the two halves agree on it. It is twice what
             it was and it is still a limit.
Approach:    Not attempted. The ring buffer needs an overflow path for a
             message larger than itself, and `read_message` holds an unsafe
             reborrow with a miri test around it, so this is a larger change to
             somebody else's code than the send side was. Growing `BUFLEN`
             moves the number without removing it.
Acceptance:  A torrent above 262,104 pieces resolves by magnet from a local
             seeder.

**Measured, and exact to one piece**, same harness as T-194, after the T-194
fix:

| pieces | `.torrent` | bitfield | result |
| --- | --- | --- | --- |
| **262,104** | 5,242,219 B | **32,768 B** | resolves |
| **262,105** | 5,242,239 B | **32,769 B** | `read buffer is full. need_additional_bytes=1` |

32,768 is `BUFLEN` exactly, and the client says how far over it is: one byte.

**What this costs in practice.** A torrent needs more than 262,104 pieces to
hit it, which is a 4 GiB payload at 16 KiB pieces and 1 TiB at 4 MiB. Real
clients raise the piece length as the payload grows, so this is reachable but
uncommon. `bit-cli create` refuses to build one above 100,000 pieces without
`--allow piece-count`, which is not a fix and does not help a torrent somebody
else made.

**Closed 2026-08-22, and the Approach's worry was the right one to have.** It
said the ring buffer needs an overflow path and that `read_message` holds an
unsafe reborrow with a miri test around it. Both are true and neither stopped
it.

**The buffer grows.** `buf` is a `Box<[u8]>` rather than a `Box<[u8; BUFLEN]>`,
every use of `BUFLEN` in the ring arithmetic reads the current capacity, and
`grow` doubles into a new allocation, copying the two halves contiguously to
the front. It is called from exactly one place: the `NotEnoughData` arm, when
the buffer is full and the message is not finished. `BUFLEN` is still what a
connection starts with.

**What stops a peer using it to make this process allocate.** Growth is bounded
by `max_len`, and `max_len` is never taken from the length prefix the peer
sent, which is the number a hostile peer picks. It comes from
`PeerConnectionHandler::max_incoming_message_len`, a new trait method whose
default is the old buffer, so an implementor that does not answer behaves
exactly as before:

- **A live torrent answers from its own piece count**, one bitfield plus
  `MAX_MSG_LEN` of slack. A peer can make the buffer as large as one bitfield
  for the torrent it is talking about and no larger.
- **`peer_info_reader` cannot**, and that is the interesting case. A seeder
  sends its bitfield immediately after the handshake, before this side has the
  metadata, so the message that arrives is as large as the torrent makes it
  while the piece count is the exact thing not known yet. It answers with a
  constant, `MAX_BITFIELD_BEFORE_METADATA` = 1 MiB, which is 8,388,568 pieces:
  128 GiB at a 16 KiB piece length and 32 TiB at 4 MiB.

**That second one is why the first attempt did not work end to end.** The unit
test passed and `check-bitfield.ps1` still failed at 262,105, because a magnet
resolves through `peer_info_reader` and it was still holding the default. The
bitfield it choked on was one it had no use for.

**Measured.** `scripts/check-bitfield.ps1`, a seeder and a magnet fetch on
loopback with trackers and DHT off:

| pieces | `.torrent` | bitfield | before | after |
| --- | --- | --- | --- | --- |
| 262,104 | 5,242,219 B | 32,768 B | resolves | resolves |
| **262,105** | 5,242,239 B | 32,769 B | `read buffer is full` | **resolves** |
| **524,288** | 10,485,900 B | 65,541 B | `read buffer is full` | **resolves** |
| **1,048,576** | 20,971,661 B | 131,077 B | `read buffer is full` | **resolves** |

```bash
pwsh -NoProfile -File scripts/check-bitfield.ps1
```

The default cases are now 131,961 and 262,105, which are the two counts this
repository has measured a client dying on, one per side.

**The unsafe reborrow is still sound, and the growth path is inside what proves
it.** `test_read_buf_miri` now reads an oversized bitfield as well as a piece,
so the reallocation happens under miri while the reborrow is in play:

```bash
cargo +nightly miri test --manifest-path vendor/rqbit/Cargo.toml -p librqbit --features miri test_read_buf_miri -- --ignored
```

**Two things about running that on Windows**, because both cost time.
`cargo-miri` fails with "cargo uses an argfile to invoke rustc" when the
command line gets long, and a short `CARGO_TARGET_DIR` is the way past it. And
`with_timeout` is a no-op only under `--features miri`, so a test that reaches
it cannot run outside miri without a tokio runtime; the growth test is a
`#[tokio::test]` for the ordinary suite and the miri one covers the same path.

**What is left, and it is a different shape.** The pre-metadata ceiling is a
constant rather than a fact about the torrent, so it is a limit, not the
absence of one. Removing it properly means skipping a message this side has no
use for rather than buffering it, which changes `read_message`'s contract from
"return a message" to "may drop one". Nothing in this repository needs it: a
torrent past 8,388,568 pieces is 128 GiB at the smallest piece length anyone
uses.

---

### T-210 An incoming peer is recorded under this session's own peer id

Source:      found closing [T-132](multi-source.md), 2026-08-22
Category:    peers
Priority:    P1
Effort:      S
Status:      **done**, 2026-08-22T17:55Z

Problem:     `manage_peer_incoming` builds the handshake it is about to send,
             writes it, and then hands **that** handshake to
             `on_handshake` and asks **it** whether extended messages are
             supported. Both answers are about this session rather than about
             the peer. The outgoing path a few lines below reads the peer's
             handshake off the wire and uses that, which is what says this is
             a slip rather than a design.
Relevance:   Two things follow, and the second is a wire behaviour. Every
             incoming peer is recorded under our own peer id, so anything
             asking "who is this peer" gets ourselves. And
             `Handshake::new` always sets the BEP 10 extension bit, so every
             incoming peer is assumed to speak the extension protocol whether
             or not it said so.
Approach:    Use `incoming.handshake`, which is the peer's, already read and
             already validated for info hash and self-connection eight lines
             above.
Acceptance:  A peer-scoped rate limit keyed on the peer id reaches an outgoing
             peer and not an exempt incoming one, which is
             `scripts/check-rate-scope.ps1`'s `http_peer_cap` row.

**Found by a limiter that did not limit.** [T-132](multi-source.md) needed the
session's download limit to skip one peer, identified by its peer id prefix.
The exemption matched nothing, and the reason was that the peer id every
incoming peer was filed under was this session's own. `bit-cli`'s web seed
bridge dials **in**, so it was exactly the case that took the wrong path.

The fix is three lines in
`vendor/rqbit/crates/librqbit/src/peer_connection.rs`: the handshake built to
send is named `ours`, and the peer's handshake is what reaches
`supports_extended` and `on_handshake`.

**How it is held.** `scripts/check-rate-scope.ps1`'s `http_peer_cap` phase caps
peers and attaches an HTTP source. Before, the source was capped with them at
**8.40 MiB/s**, because its identity was ours; after, it runs at
**151.84 MiB/s** against the same cap. `bench/rate-scope-20260822T175543220Z.json`.

**The second half is not directly measured here and is not left silent.**
Nothing in this repository speaks the extension protocol badly enough to notice
being sent an extended message it did not ask for, and building a peer that
refuses BEP 10 to prove it is [T-166](#t-166-bep-10-extension-ids-are-not-proven-to-map-in-both-directions)'s
shape of work rather than this one's. What is certain from reading is that the
bit came from a constructor rather than from the wire.
