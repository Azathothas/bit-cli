# Trackers

Forty-one issues touch UDP tracker handling, announce backoff, BEP 12 tier
logic, and scrape.

---

### T-060 The announced port is wrong when no port is configured

Source:      https://github.com/ikatson/rqbit/issues/507 (open)
Category:    trackers
Priority:    P1
Effort:      S
Status:      open

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

### T-061 bit-cli trackers announces a fixed port

Source:      `bit-cli` defect, found while writing T-060
Category:    trackers
Priority:    P1
Effort:      S
Status:      open

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

### T-062 Announce timing has no started, completed, or stopped events

Source:      https://github.com/ikatson/rqbit/issues/539 (open)
Category:    trackers
Priority:    P1
Effort:      M
Status:      open

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
