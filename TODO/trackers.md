# Trackers

Forty-one issues touch UDP tracker handling, announce backoff, BEP 12 tier
logic, and scrape.

---

### T-060 The announced port is wrong when no port is configured

Source:      https://github.com/ikatson/rqbit/issues/507 (open)
Category:    trackers
Priority:    P1
Effort:      S
Status:      **done**

Problem:     On 8.1.1 the announced port was always 0, which some trackers
             (aquatic among them) reject. On main it is always 4240 even when
             the session is listening elsewhere.
Relevance:   A wrong announced port means no peer can dial in. The torrent
             still downloads, so it looks fine and seeds nothing.
Approach:    `ListenerOptions::announce_port` exists in 9.0.0
             (`listen.rs:57`) and `bit-cli` leaves it `None`, which makes the
             session announce the port it actually bound. Verify that is what
             reaches the tracker rather than assuming: `bit-cli trackers` uses
             its own client and announces 6881 unconditionally, which is a
             separate bug of the same shape.
Acceptance:  `bit-cli trackers <TORRENT> --json` announces the port the session
             is listening on, and a packet capture or a tracker that echoes the
             peer list confirms the announced address is dialable.

**Done, and it was a verification rather than a fix.** `bit-cli` leaves
`ListenerOptions::announce_port` unset, so the session announces the port it
bound, and the test proves it end to end rather than by reading the source.

`cmd::seed::tests::the_session_announces_the_port_it_listens_on` runs
`bit-cli seed --port <N>` against a loopback tracker that records every
announce, waits for the first one, and asserts two things: the `port`
parameter is `N`, and a TCP connection to that port is accepted while the run
lasts. The second is the half a recorded number does not prove, and it is what
the acceptance asks for in place of a packet capture.

The tracker is `crate::test_support::Tracker`, a fixture that answers every
announce with the same bencoded reply and keeps the request lines. It is not
`crates/bit-cli-core/examples/loopback-tracker.rs`, which tracks a real swarm
and is what the interop scripts drive; a test cannot run an example binary.

The `bit-cli trackers` half of this entry was its own defect and is
[T-061](#t-061-bit-cli-trackers-announces-a-fixed-port).

### T-061 bit-cli trackers announces a fixed port

Source:      `bit-cli` defect, found while writing T-060
Category:    trackers
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `cmd::trackers::run` builds its `Announce` with a hardcoded 6881.
             The command does not start a session, so it has no listening port
             to announce, and announcing one it is not listening on registers
             an unreachable peer with the tracker.
Relevance:   `bit-cli trackers` is a diagnostic. Registering a fake peer as a
             side effect of asking a question is wrong.
Approach:    Two options, and the second is better: either bind a real port for
             the length of the announce, or send `numwant` with the announce
             and no port at all so the tracker treats it as a query. BEP 3
             requires `port`, so the honest version is to bind.
Acceptance:  `bit-cli trackers <TORRENT>` either binds the port it announces,
             or announces `event=stopped` immediately after so the tracker
             record does not linger. Whichever it does is tested.

**Done, and it does both.** The command binds a port for as long as the
announce lasts, announces that port, and then withdraws the record with a
second announce carrying `event=stopped`. Either alone leaves something
wrong: a bound port that stays registered after the process exits is a dead
address for the tracker's whole interval, and a withdrawal of a port nothing
ever listened on is a wrong answer politely retracted.

`--port` takes a port or a `START-END` range, the same spelling `download` and
`seed` use, and defaults to the same `6881-6889`. `--no-withdraw` leaves the
record in place for a caller who wants exactly the announce a client would
send. A scrape binds nothing and withdraws nothing: it carries no port and no
event.

The report carries `announced_port` and `withdrawn`, so what the command did
is in the JSON rather than only in what the tracker saw.

Three tests, all against the recording tracker in `test_support`:

- `the_announced_port_is_bound_and_the_record_is_withdrawn` asserts the
  announced port is neither zero nor 6881, that both announces carry the same
  port, that the events are `started` then `stopped`, and that the port is
  free again once the command exits, which is what says it was held.
- `no_withdraw_sends_one_announce_and_reports_no_withdrawal`.
- `a_scrape_carries_no_port_and_no_withdrawal`.

### T-062 Announce timing has no started, completed, or stopped events

Source:      https://github.com/ikatson/rqbit/issues/539 (open)
Category:    trackers
Priority:    P1
Effort:      M
Status:      **done**

Problem:     The session announces on unpause and then loops on the interval.
             It never sends `completed` when a download finishes, and never
             sends `stopped` when it shuts down.
Relevance:   Trackers use `completed` for the seeder count and `stopped` to
             drop the peer promptly. Without them a private tracker's ratio
             accounting is wrong and a public one keeps handing out a dead
             address for an hour.
Approach:    `bit-cli` runs in the foreground and knows exactly when both
             happen. Send `completed` from the watch loop on the transition to
             finished, and `stopped` in the shutdown path, through
             `bit-cli`'s own tracker client rather than waiting for upstream.
Acceptance:  A capture of a full `bit-cli download` run against a local tracker
             shows `event=started`, then `event=completed`, then
             `event=stopped`, in that order.

**Done, exactly as the approach describes it.** `cmd::download::announce_event`
sends one event to every tracker the torrent uses: `completed` from the watch
loop the moment the torrent finishes, and `stopped` after the loop ends however
it ended.

**The peer id is the part that had to be right.** An announce from a second
identity does not update the session's record, it creates another one, so a
`stopped` sent that way would leave the original peer registered and add a
phantom beside it. Both announces carry `handle.shared().peer_id` and the
session's own listening port, which is what makes them updates rather than a
second peer. The test asserts one peer id and one port across all three
announces.

One thing the shape of this costs. The `completed` announce is awaited inside
the watch loop, so a tracker that is slow to answer delays the next progress
tick by up to `--tracker-timeout`. It is bounded, it happens once, and it
happens at the moment the payload is already on disk, which is why it is
awaited rather than spawned: a run that exits before its own announce has left
has not announced.

The acceptance, run as a test rather than as a capture. The tracker records
every request line and the run is a real transfer from a loopback file server:

```
GET /announce?...&peer_id=-rQ9000-...&event=started&port=59193&...
GET /announce?...&peer_id=-rQ9000-...&port=59193&...&numwant=0&event=completed
GET /announce?...&peer_id=-rQ9000-...&port=59193&...&numwant=0&event=stopped
```

`a_run_announces_started_then_completed_then_stopped` asserts that sequence,
and that the report's `announced` array carries `completed` and `stopped` with
how many trackers accepted each.

**A payload already on disk announces in a different order, and that is not a
defect.** A torrent complete on its hash check finishes before the session's
own `started` announce has left, so a tracker sees `completed` first. The test
fetches its payload for that reason, and it is worth knowing before someone
reads the log of a resumed run and files it as a bug.

Three things this deliberately does not do. It does not fail a run when a
tracker is unreachable at the end: the announce is a courtesy and the payload
is already on disk. It does not send `started` itself, because the session
already does. And it counts trackers rather than reporting each one, because a
withdrawal that failed leaves a record that expires on its own, which is the
state the run was in anyway.

### T-063 Tracker tiers are announced in parallel rather than in order

Source:      `bit-cli` design decision, BEP 12
Category:    trackers
Priority:    P3
Effort:      S
Status:      open

Problem:     `bit-cli trackers` asks every tracker at once. BEP 12 says a
             client should try tier one, and only fall through to tier two if
             every tracker in tier one fails.
Relevance:   For a client trying to stay connected, the tier order is the
             point. For a command whose job is to report on all of them,
             waiting out a dead tier one to reach tier two only makes one dead
             tracker cost the whole run.
Approach:    This is deliberate and documented in `cmd/trackers.rs`. The entry
             exists so the divergence is recorded rather than discovered. If a
             `--respect-tiers` flag is wanted later, it goes here.
Acceptance:  Decide, and either add the flag or close this with the reasoning
             in `docs/`.

### T-064 UDP tracker retry does not follow the BEP 15 backoff

Source:      BEP 15
Category:    trackers
Priority:    P2
Effort:      S
Status:      open

Problem:     BEP 15 specifies retrying at `15 * 2^n` seconds for n from 0 to 8.
             `bit-cli`'s UDP client makes three attempts inside the configured
             timeout instead.
Relevance:   The spec backoff takes up to 62 minutes to give up, which is
             wrong for a foreground diagnostic. Three attempts inside
             `--tracker-timeout` is the right shape for this tool, but it is a
             deliberate divergence and should be written down.
Approach:    Keep the behaviour, document it in `docs/`, and make the attempt
             count configurable if a caller ever needs the spec timing.
Acceptance:  `docs/` states the retry policy and why it differs from BEP 15.

### T-065 Scrape is only implemented for the BEP 48 URL convention

Source:      BEP 48
Category:    trackers
Priority:    P3
Effort:      S
Status:      open

Problem:     `tracker::scrape_url` derives the scrape endpoint by replacing a
             trailing `announce` path component with `scrape`. A tracker whose
             announce path does not end that way has no derivable scrape URL,
             and `bit-cli` reports that rather than guessing.
Relevance:   Guessing produces a 404 that reads like the tracker being down,
             which is a worse answer than "cannot be derived".
Approach:    Add `--scrape-url` so a caller who knows the endpoint can supply
             it.
Acceptance:  `bit-cli trackers <TORRENT> --scrape --scrape-url <URL>` scrapes
             a tracker whose convention differs.

---

## What the 2026-08-21 corpus adds to the three entries above

**T-063, tier order.** `TorrentNG/crates/rt-tracker/src/tier.rs` is the BEP 12
rule implemented: `:8` `Tier { trackers, active }`, `:55`
`TierSet { tiers, active_tier }`, `promote_active()` which **swaps a successful
tracker to the front of its tier**, and `advance()` which moves to the next
tracker on failure and then to the next tier. That is the whole algorithm and
it is small.

`bit-cli`'s divergence stands, and this entry's reasoning survives contact with
it: a command whose job is to report on every tracker should not wait out a
dead tier. What the corpus adds is that a `--respect-tiers` flag would be
cheap, and one fact that changes where the work is. `nanotorrent`'s patch 0008
records that **librqbit flattens `announce_list` tiers into a `HashSet`**, so
tier order is not available from the session at all without patching it.
`bit-cli`'s own `tracker.rs:115` keeps the tier index for the `trackers`
command, so the divergence is real for the command and *forced* for the
download path. Those are two different situations and this entry currently
reads as though they were one. Note also that promoting a working tracker to
the front of its tier is useful even without tier fallthrough, and costs
nothing.

mtorrent [Issue 29](https://github.com/DanglingPointer/mtorrent/issues/29)
adds the ordering rule worth having whatever is decided: **announce to the
torrent's own trackers before any configured extras.** With many trackers
configured, outgoing connects timed out and peers were never reached.

**T-064, UDP backoff.** Two ladders exist and both are defensible, which
supports this entry's decision to diverge deliberately rather than copy.
`torrent/tracker/udp/timeout.go:9` is BEP 15 as written, `15 * 2^n` clamped at
`n = 8`, which is 3840 seconds, in nine lines of code and up to 62 minutes.
`mtorrent/mtorrent-core/src/trackers/udp.rs:150` takes `MAX_RETRANSMISSIONS = 3`
with `:160` `timeout_sec = 15 * (1 << retransmit_n)`, so 15, 30, 60 and 120
seconds, giving up at 225 seconds and documenting that total. `bit-cli` makes
three attempts inside `--tracker-timeout`, dividing it by three
(`tracker.rs:364`), which is a third shape. **Documenting the total budget is
what the other two do and this entry should adopt**: the Acceptance says "state
the retry policy", and stating the worst-case wall clock is what a caller
setting a deadline actually needs.

One thing this entry does not mention and should. Connection ids expire, and a
client that caches one too long **will** be rejected.
`aquatic/crates/udp/src/workers/socket/validator.rs` shows why from the server
side: a `ConnectionId` is four bytes of seconds-since-start plus four bytes of
truncated keyed BLAKE3 over those bytes and the client IP, validated in
constant time and expiring after `max_connection_age`. anacrolix caches ids
with a one-minute reissue rule and carries an explicit workaround for one
tracker, forcing a reconnect when the error body is literally
`"Connection ID missmatch.\x00"`. A one-shot `bit-cli trackers` run is short
enough that this rarely bites, and a `download` that announces
`started`, then `completed`, then `stopped` over a long transfer is not.

**T-065, scrape convention.** Corroborated and closed as a question.
`torrent/tracker/http/scrape.go` derives the scrape URL with
`url.JoinPath("..", "scrape")`, the same BEP 48 convention, and **no repository
in the corpus implements another one**. So "cannot be derived" is the right
answer and `--scrape-url` is the right escape hatch, which is what this entry
already proposes. Related, from aquatic
[Issue 232](https://github.com/greatest-ape/aquatic/issues/232): there is no
canonical announce path for a UDP tracker either. The path in a `udp://` URL
is advisory, carried as a BEP 41 option if wanted, so a client must not
assume `/announce` there.

---

### T-180 A negative left in a tracker exchange has no decided handling

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    trackers
Priority:    P2
Effort:      S
Status:      open

Problem:     Two halves of one question, and neither has been decided.

             **On the way out.** `bit-cli` announces `left` as a byte count.
             Before a magnet's metadata arrives there is no total length, so
             the true answer is unknown, and nothing in the tree records what
             is sent in that window.

             **On the way in.** A tracker or a peer-facing announce relay can
             carry a negative `left`, and `bit-cli`'s response parsing has no
             fixture for one.
Relevance:   aquatic [PR 254](https://github.com/greatest-ape/aquatic/pull/254)
             (MERGED) is the evidence that this is real rather than
             theoretical: **some clients send `left = -1`** when the length is
             unknown, rather than omitting the parameter, and a `usize` parse
             rejected the whole announce. That PR cross-references
             anacrolix/torrent#981, so at least two implementations met it
             independently. `aquatic/crates/ws_protocol/src/incoming/announce.rs:13`
             separately records that `left` **may be absent entirely**, for
             instance when a magnet is opened.

             `bit-cli` is on both sides of this. It announces for real from
             `trackers` and from `download`'s `started`, `completed` and
             `stopped` events, and it parses responses. A magnet is a first
             class source here, so the unknown-length window is a normal path
             and not an edge case.
Approach:    Decide both halves and test both.

             Outbound, there are three candidate answers and the third is
             probably right: send `left=0`, which claims to be a seed and is a
             lie that costs other peers; omit the key, which some trackers
             reject; or send a large sentinel. What settles it is that a
             tracker rejecting the announce is a loud failure and claiming to
             seed a payload you do not have is a silent one, so prefer
             correctness over acceptance and record which trackers refuse.
             `bit-cli trackers <MAGNET>` against a real tracker is the
             measurement, and it is cheap.

             Inbound, accept a negative or absent value and normalise it to
             "unknown" rather than to zero, because zero means seed and
             unknown does not. A signed parse plus an `Option` is the whole
             change.

             While the response parser is open, anacrolix
             [PR 1055](https://github.com/anacrolix/torrent/pull/1055) is the
             other fixture to add: a tracker returning `peers: [42]`, or a peer
             dictionary missing `ip` or `port`, crashed the client. The fix
             keeps the good entries and errors on the bad ones, which is the
             right shape for `bit-cli`, whose trackers come from untrusted
             torrents. aquatic
             [Issue 82](https://github.com/greatest-ape/aquatic/issues/82) adds
             the empty case: a response with **no `peers` key at all** is a
             well-formed empty swarm, not a parse error.
Acceptance:  `bit-cli trackers <MAGNET> --json` states what it sent for `left`
             and why, a fixture response carrying `left = -1` parses to
             "unknown" rather than to a seed, and a fixture response carrying
             `peers: [42]` keeps every valid peer and names the invalid entry
             without failing the run.
