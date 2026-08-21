# Phase C backlog

Everything here needs a persistent process or a user at a keyboard. Decision 7.4
settles it: none of this is built in Phase A or Phase B. It is written in the
same entry format as the rest so the backlog is executable rather than a wish
list, and so the Phase A architecture can be checked against it: the core
library must not assume a process lifetime, and state must be addressable.

**Do not work on this file.** It exists to keep the ideas out of the code.

---

### T-200 Session daemon

Source:      decision 7.4
Category:    phase-c
Priority:    P0 within Phase C
Effort:      XL
Status:      deferred

Problem:     Every `bit-cli` command starts a session, does its work, and
             exits. A torrent cannot outlive an invocation, so there is no way
             to add one now and check on it later.
Relevance:   It is the foundation every other item here sits on.
Approach:    A foreground process that owns a long-lived `Session` and serves
             the verbs over a local socket. The core library already keeps
             configuration explicit and holds no global state, which is what
             makes this addable without a rewrite.
Acceptance:  `bit-cli daemon start` runs, `bit-cli add <SOURCE>` returns
             immediately, and `bit-cli status` reports the torrent from a
             second process.

### T-201 JSON-RPC and XML-RPC, with aria2 method parity

Source:      decision 7.4, PROMPT.md 9.10
Category:    phase-c
Priority:    P1 within Phase C
Effort:      XL
Status:      deferred

Problem:     No RPC surface of any kind.
Relevance:   `aria2.addTorrent` with a `uris` array is the only documented web
             seed surface any existing tool has, so parity with it is what lets
             an existing deployment migrate.
Approach:    `--enable-rpc` and the `--rpc-*` option family, `aria2.*` method
             names, and the secret token scheme.
Acceptance:  An existing `aria2` RPC client drives a download end to end
             without modification.

### T-202 Queue management across invocations

Source:      decision 7.4
Category:    phase-c
Priority:    P1 within Phase C
Effort:      L
Status:      deferred

Problem:     `-j` limits parallelism inside one invocation. There is no queue
             that spans invocations and no way to reorder one.
Relevance:   `--max-concurrent-downloads` in `aria2` is a queue depth, not a
             parallelism cap, and a migrating script will expect that.
Approach:    Needs the daemon. `changePosition` is the reordering primitive.
Acceptance:  Three torrents added with a queue depth of one run in the order
             they were added, and `changePosition` moves the third to the
             front.

### T-203 Session save and restore

Source:      decision 7.4
Category:    phase-c
Priority:    P1 within Phase C
Effort:      L
Status:      deferred

Problem:     `--save-session`, `--force-save`, and `--auto-save-interval` have
             no equivalent. `librqbit`'s `SessionPersistenceConfig` is
             deliberately left off.
Relevance:   Restarting a box should not lose a queue.
Approach:    Needs the daemon. The format has to carry enough to reconstruct
             the queue, the file selections, and the per-torrent limits.
Acceptance:  A daemon restarted mid-download resumes every torrent at the
             progress it had.

### T-204 Persistent attached web seeds

Source:      PROMPT.md S.2, the deleted `src/webseed/state.rs`
Category:    phase-c
Priority:    P2 within Phase C
Effort:      M
Status:      deferred

Problem:     `kist` persisted attached web seeds keyed by info hash, so a
             source attached once stayed attached across restarts. That file
             was deleted during the crate split, because a stored record is a
             session concept.
Relevance:   This is the one Phase C item that touches the headline feature, so
             the boundary matters: in Phase A and B, web seeds attach per
             invocation through flags, and `bit-cli edit` is how a source is
             made permanent by writing it into the `.torrent`. Only a daemon
             needs a third option.
Approach:    Keyed by info hash, stored alongside the session state from T-203,
             with the same binding table schema `--web-seed-config` already
             uses so the two are interchangeable.
Acceptance:  A source attached through the daemon survives a restart and
             appears in `bit-cli webseed list` against the running session.

### T-205 Download result registry

Source:      decision 7.4
Category:    phase-c
Priority:    P3 within Phase C
Effort:      M
Status:      deferred

Problem:     No `--max-download-result`, `--download-result`, or
             `purgeDownloadResult`.
Relevance:   It is how an RPC client learns that something finished while it
             was not looking.
Approach:    Needs the daemon.
Acceptance:  A finished torrent appears in the result list and is purgeable.

### T-206 GID assignment

Source:      decision 7.4
Category:    phase-c
Priority:    P3 within Phase C
Effort:      S
Status:      deferred

Problem:     Torrents have a per-run index and no stable identifier.
Relevance:   Every `aria2` RPC method takes a GID.
Approach:    Needs the daemon. The info hash is the natural key and is not
             unique when the same torrent is added twice with different file
             selections, so a GID is a separate identifier.
Acceptance:  `bit-cli add` returns a GID that `bit-cli status <GID>` accepts.

### T-207 Session-attached verbs from the old TUI

Source:      decision 7.4, `docs/command-mapping.md`
Category:    phase-c
Priority:    P2 within Phase C
Effort:      M
Status:      deferred

Problem:     `add` to a queue, `pause`, `resume`, `remove`, and marking a
             torrent all need something to mutate.
Relevance:   Six of the old `CommandId` variants map here.
Approach:    Needs the daemon.
Acceptance:  Each verb works against a running daemon.

### T-208 status --follow against a live session

Source:      decision 7.4
Category:    phase-c
Priority:    P3 within Phase C
Effort:      M
Status:      deferred

Problem:     `bit-cli` can stream progress for a download it is running itself
             (`--jsonl`), and cannot report on a download another process is
             running.
Relevance:   The distinction matters and is easy to blur: a streaming mode a
             single foreground invocation produces itself is Phase A;
             following someone else's session is Phase C.
Approach:    One subcommand serving both a one-shot query and a stream, with
             the mode a value rather than a separate verb: snapshot, follow,
             set the interval, stop. That shape keeps `--follow` from becoming
             a second command that drifts from the first.
Acceptance:  `bit-cli status --follow` streams events from a daemon.

### T-209 Watch directories, RSS, cluster mode, and the control service

Source:      decision 7.4, PROMPT.md 2.5
Category:    phase-c
Priority:    P3 within Phase C
Effort:      XL
Status:      deferred

Problem:     Everything that needs a running process: RSS ingestion, watch
             directories, Docker and VPN integration, cluster mode, and the
             control service.
Relevance:   Recorded so they are not rediscovered as Phase B ideas.
Approach:    All need the daemon.
Acceptance:  Deferred as a group.
