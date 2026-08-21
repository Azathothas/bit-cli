# Peer connections

Forty issues in the corpus touch peer handling: handshakes, connection churn,
connection limits, choke logic, and slow peers.

---

### T-020 Connections accumulate in CLOSE_WAIT until TCP is unusable

Source:      https://github.com/ikatson/rqbit/issues/311 (open)
Category:    peers
Priority:    P0
Effort:      L
Status:      open

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
ceiling carried here is still the only mitigation, and it now has a second
reason to exist.

Reproduce:

```powershell
pwsh -NoProfile -File scripts/check-swarm.ps1
```

Case `listener_poisoned`, which carries `judged: false` because this entry is
open and an acceptance script does not fail the build for a defect that is
already recorded.

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
Status:      open

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
Status:      open

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
Status:      open

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
Acceptance:  A synthetic peer that fails a piece hash twice is not redialled
             for the rest of the run, `bit-cli peers --json` names it with the
             reason, and the freed slot measurably goes to another peer.
             `bench swarm` drives it, because it already builds peers that
             misbehave on purpose.

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
`MY_EXTENDED_UT_PEX = 1` and `MY_EXTENDED_UT_METADATA = 3`
(`librqbit-peer-protocol/src/lib.rs:52`, `:55`, dispatched at
`src/extended/mod.rs`). Those are the ids that crate advertises. This bridge
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

