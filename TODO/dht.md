# DHT

Twenty-two issues touch bootstrap, routing table health, announce, and IPv6.

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

Source:      PROMPT.md A3.12
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
