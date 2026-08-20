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

### T-021 A temporary network drop stops the download permanently

Source:      https://github.com/ikatson/rqbit/issues/363 (open)
Category:    peers
Priority:    P0
Effort:      M
Status:      open

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

Source:      carried from the first session, PROMPT.md S.1
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

Source:      PROMPT.md A3.4b
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
