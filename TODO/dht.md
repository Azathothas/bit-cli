# DHT

Twenty-two issues touch bootstrap, routing table health, announce, and IPv6.

**The 2026-08-21 corpus adds three things to this file.** Two new entries
below, [T-169](#t-169-bep-33-dht-scrape-and-bep-51-infohash-indexing-are-not-implemented)
and [T-170](#t-170-bep-44-mutable-items-are-not-implemented), and one design
answer for [T-050](#t-050-the-dht-cache-costs-disk-io-even-when-nothing-is-running)
and [T-052](#t-052-dht-is-not-reported) that is worth stating before either.

**A short-lived CLI should almost certainly never become a DHT server.**
`n0-mainline`'s documented default is to start as a client and switch to
server mode only after **fifteen minutes** with a publicly reachable address,
so that only stable reachable nodes carry routing load. `bit-cli` is a
foreground one-shot: a `download` that runs for ninety seconds has no business
in anyone's routing table, and taking queries it will not be around to answer
is a cost imposed on the network for no gain here. That is the argument
T-050 needs, and it is stronger than "check what the default persists": the
answer is client mode, no persistence, and a documented path if persistence is
ever wanted. `n0-mainline/src/common/closest_nodes.rs:127` `dht_size_estimate`
and `docs/dht_size_estimate.md` are what a `dht` report object could carry
beyond a routing-table count, for T-052.

`fx-torrent/src/dht/` is the widest BEP 5 surface in the corpus to read
against: `krpc.rs` handles `ping`, `find_node`, `get_peers`, `announce_peer`,
`sample_infohashes` (`:18`), `put` (`:19`) and `get` (`:20`) in one message
enum. Two of its closed issues are one-line interop traps worth knowing before
touching any of this. [Issue 16](https://github.com/yoep/fx-torrent/issues/16):
the KRPC transaction id was fixed at two bytes, because BEP 5 says two is
*typically* enough; real nodes send four, and the result is
`Invalid Length: 4 (expected: a byte array of length 2)` on every reply from
those nodes. [Issue 21](https://github.com/yoep/fx-torrent/issues/21): a KRPC
error response is a **list** `[code, message]` and not a dict, and reading it
as a dict turns every error into a parse failure, which hides the error that
was being reported.

---

### T-050 The DHT cache costs disk I/O even when nothing is running

Source:      https://github.com/ikatson/rqbit/issues/310 (open)
Category:    dht
Priority:    P2
Effort:      S
Status:      open

Problem:     A reporter running the daemon with no active torrents saw it as
             the busiest writer on the machine, from periodically saving the
             DHT routing table.
Relevance:   `bit-cli` is a foreground one-shot, so it does not sit idle
             writing a cache. It uses `DhtSessionConfig::default()`, which
             enables persistence, so a short run may still write one.
Approach:    Check what `DhtSessionConfig::default()` persists and where. If it
             writes outside the download directory, that is state a one-shot
             tool leaves behind, which decision 7.4 rules out. Either turn
             persistence off or document the path.
Acceptance:  `bit-cli download <MAGNET>` writes nothing outside `--dir` and the
             system temp directory, verified by watching the process with
             Process Monitor for one run and recording the write list here.

### T-051 A magnet with no DHT and no trackers fails without saying so

Source:      design gap
Category:    dht
Priority:    P2
Effort:      S
Status:      open

Problem:     `--web-seed-only` turns off DHT, LSD, and trackers. A magnet
             source then has no way to resolve its metadata, so the run waits
             on `wait_until_initialized` until the deadline.
Relevance:   The combination is a reasonable thing to ask for and it cannot
             work. It should fail immediately with a clear reason.
Approach:    Refuse at argument-validation time: a magnet or bare info hash
             with `--web-seed-only` and no `.torrent` is a usage error, because
             web seeds carry payload and not metadata.
Acceptance:  `bit-cli download <MAGNET> --web-seed-only --web-seed <URL>` exits
             2 immediately, naming the conflict, rather than timing out.

### T-052 DHT is not reported

Source:      the operator's brief
Category:    dht
Priority:    P3
Effort:      M
Status:      open

Problem:     `--trace dht` is accepted and enables the tracing target, but
             nothing in the JSON reports says whether the DHT found anything:
             no bootstrap status, no routing table size, no announce result.
Relevance:   On a torrent with dead trackers the DHT is the only discovery
             path, and "did it work" currently has to be inferred from the peer
             count.
Approach:    `librqbit` exposes DHT stats through its API. Surface bootstrap
             state, routing table size, and peers found through the DHT as a
             `dht` object in the download and peers reports.
Acceptance:  `bit-cli peers <MAGNET> --json` carries `"dht": {"bootstrapped":
             true, "routing_table_size": N, "peers_found": M}`.

### T-169 BEP 33 DHT scrape and BEP 51 infohash indexing are not implemented

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    dht
Priority:    P3
Effort:      M
Status:      open

Problem:     `bit-cli trackers --scrape` scrapes a tracker. There is no way to
             ask the DHT the same question, and no way to participate in
             BEP 51 infohash sampling.
Relevance:   BEP 33 is the one that earns its place. It answers "how many
             seeders and leechers" from the DHT rather than from a tracker,
             which is exactly the case a torrent with dead trackers leaves
             `bit-cli` unable to answer at all — the same case
             [T-052](#t-052-dht-is-not-reported) exists for. BEP 51 is
             discovery infrastructure rather than a capability a download
             needs, and its main relevance here is that participation should
             be **opt-out**, which fx-torrent
             [Issue 30](https://github.com/yoep/fx-torrent/issues/30) concluded
             independently. A one-shot CLI that indexes info hashes for
             strangers by default is the same overreach as becoming a server
             by default.
Approach:    BEP 33 answers a `get_peers` with two bloom filters, one for
             seeders and one for leechers, and the arithmetic is the whole
             trick. `fx-torrent/src/bloom_filter.rs` is 229 lines with the
             implementation in the first 130 and eight tests after: `:5`
             `has_bits` and `:20` `set_bits` take the
             **first 4 bytes of the key as two little-endian `u16`
             indices**; `:46` `len()` estimates
             the population as `-(m/k) * ln(zero/m)` with `k = 2`; `:93`
             `count_zero_bits` uses a 16-entry nibble table. The DHT side is
             `fx-torrent/src/dht/tracker.rs:449` `scrape_peers` and `:2469`
             `scrape_info_hashes`.

             For BEP 51, `fx-torrent/src/dht/krpc.rs:18` carries
             `sample_infohashes` in the message enum, and
             `fx-torrent/src/dht/tracker.rs:1736` logs "detected spoofed
             sample_infohashes", which is the reminder that a sampling response
             is untrusted input like any other.
Blocker:     `bit-cli` does not own its DHT. `librqbit` supplies it and
             `librqbit-dht` exposes no hook for a custom KRPC method or a
             custom `get_peers` response. What would unblock BEP 33 is either
             an upstream change or a second DHT client used only for scrape,
             which is a real option because the query is stateless and needs
             no routing table of its own beyond bootstrap.
Acceptance:  `bit-cli trackers <TORRENT> --scrape --dht` reports seeder and
             leecher estimates from the DHT beside the tracker's own numbers,
             and the two are printed separately rather than merged, because
             they are measuring different populations.

### T-170 BEP 44 mutable items are not implemented

Source:      `reference/RESEARCH.md` section D, 2026-08-21
Category:    dht
Priority:    P3
Effort:      L
Status:      open

Problem:     No DHT put or get of arbitrary items, mutable or immutable.
Relevance:   This is the half that pairs with something `bit-cli` already has.
             `create` and `edit` write BEP 39 `update-url`, an HTTP URL a
             client can poll for a newer version of a torrent. BEP 44 mutable
             items are the same idea without the HTTP server: a public key
             addresses a slot in the DHT, the holder of the private key signs
             updates into it, and a reader who knows the key gets the current
             version. That is what BEP 46 mutable torrents are built on. For a
             tool whose whole subject is attaching sources to a torrent that
             already exists, a torrent that can announce its own successor
             without a web server is a natural fit, and `bit-cli` is already
             half of it.
Approach:    `n0-mainline/src/common/mutable.rs` is the reference and it is
             small. `:32` `MutableItem::new(signer, value, seq, salt)`, `:46`
             `target_from_key` = **SHA-1 of `public_key || salt`**, and `:145`
             `encode_signable(seq, value, salt)`, which is the exact byte
             sequence that gets ed25519-signed and therefore the only part
             where a mistake is silent rather than loud.
             `src/common/immutable.rs` is the immutable half and
             `src/core/put_query.rs` is the put path.
             `n0-mainline/beps/` carries the normative reStructuredText of
             **BEP 5, 42, 43 and 44**, which is worth more than any
             implementation when the question is what the specification
             actually requires.
             [PR 9](https://github.com/n0-computer/n0-mainline/pull/9)
             (MERGED) ports "mainline 6.4.1 mutable put security fixes" and is
             required reading before implementing `put`.
Blocker:     Same as [T-169](#t-169-bep-33-dht-scrape-and-bep-51-infohash-indexing-are-not-implemented):
             `librqbit`'s DHT exposes no put or get. Unlike T-169 this one is
             genuinely separable, because a BEP 44 client needs no
             relationship to any torrent, so a small standalone DHT client is
             a legitimate route rather than a workaround.
Acceptance:  `bit-cli` reads a mutable item by public key and salt, verifies
             its signature, and resolves it to a torrent; and writes one,
             re-read from a second process. Both directions, with the sequence
             number visible in `--json`, because a reader has to be able to
             tell a stale read from a current one.
