# Performance and throughput

Thirty-one issues touch piece picking, pipelining, block size, endgame, and
buffering.

---

### T-030 Throughput collapses with several torrents at once

Source:      https://github.com/ikatson/rqbit/issues/590 (open)
Category:    performance
Priority:    P0
Effort:      L
Status:      open

Problem:     Two reports in one: adding several torrents slows all of them well
             past what sharing a link explains, and a single large torrent
             (over 4 GB) shows start-stop-start-stop behaviour where the rate
             drops to zero and only a pause and resume clears it.
Relevance:   `-j` exists to run several sources in one invocation. If that is
             slower than running them one at a time, the flag is a trap.
Approach:    Measure before theorising. Three runs with the same total payload:
             one torrent alone, three torrents at `-j 1`, three at `-j 3`. If
             `-j 3` is slower than `-j 1` in wall time for the same bytes, the
             contention is real and the next question is where: the tokio
             blocking pool, the disk, or the peer connection budget.
Acceptance:  A `bench/multi-torrent-<timestamp>.json` report with the three
             wall times, peak RSS, and CPU time, and this entry naming which
             resource saturated.

### T-031 The rate limit did not apply to the session

Source:      https://github.com/ikatson/rqbit/issues/391 (closed, 2025-06-10)
Category:    performance
Priority:    P1
Effort:      S
Status:      open

Problem:     `SessionOptions::ratelimits` was reported not to take effect.
Relevance:   `--max-download-rate` and `--max-upload-rate` go straight into
             `LimitsConfig`. If that is ignored, the flags are decorative, and
             rule 0.10 says a knob that does not move a number does not ship.
Approach:    The issue is closed upstream, which suggests the pinned 9.0.0
             carries the fix, but "closed" is not "verified here". Measure it:
             download a known payload with and without a cap and compare the
             sustained rate.
Acceptance:  `bit-cli download <TORRENT> --max-download-rate 1MiB/s` sustains
             within 10 percent of 1 MiB/s over 60 seconds, and the same run
             uncapped is meaningfully faster. Both numbers recorded here.

### T-032 The piece selector strategy is not implemented

Source:      PROMPT.md A3.6
Category:    performance
Priority:    P1
Effort:      L
Status:      open

Problem:     `--piece-selector rarest-first|sequential|in-order|random` parses
             and is carried through the config, and none of the four reaches
             `librqbit`'s picker, which is rarest-first and not configurable.
Relevance:   Sequential is what makes streaming work, and it is the difference
             between a usable and an unusable `bit-cli download | vlc -`.
Approach:    `librqbit` has a `FileStream` type and streaming support, which
             suggests some ordering control exists; find it before assuming a
             fork is needed. If the picker is genuinely fixed, this needs
             either an upstream API or Candidate C.
Acceptance:  `bit-cli download <TORRENT> --piece-selector sequential --jsonl`
             emits `piece_verified` events whose indices are non-decreasing for
             at least the first 90 percent of the run.

### T-033 --split, -x, and -k do not reach the fetch path

Source:      PROMPT.md A3.6
Category:    performance
Priority:    P2
Effort:      M
Status:      open

Problem:     `-s/--split`, `-x/--max-connection-per-server`, and
             `-k/--min-split-size` parse and do nothing. They are the aria2
             flags a migrating script will already be passing.
Relevance:   Rule 0.10 again. Three flags that look like they work and do not
             are worse than three flags that error.
Approach:    All three are about how one HTTP source is fetched in parallel.
             `--web-seed-concurrency` and `--web-seed-chunk-size` already
             express the same two ideas, so the honest wiring is to make `-x`
             an alias of `--web-seed-concurrency`, `-k` a floor on
             `--web-seed-chunk-size`, and `-s` the segment count per source,
             then prove each moves a number.
Acceptance:  `bench/split-<timestamp>.json` shows throughput at `-x 1`,
             `-x 4`, and `-x 16` against one mirror, with the curve recorded
             here. If the curve is flat, the flags do not ship.

### T-034 Endgame mode is not observable

Source:      corpus, performance category
Category:    performance
Priority:    P3
Effort:      M
Status:      open

Problem:     The last few pieces of a download are the ones that decide the
             wall time, and nothing in the report says whether endgame
             duplication happened or how long the tail took.
Relevance:   `bench leech` is meant to answer "is my server serving well". A
             run whose last piece took 40 seconds looks the same as one that
             was uniformly slow.
Approach:    Record time to 90, 99, and 100 percent separately in the download
             report, and the number of pieces requested from more than one
             source.
Acceptance:  `bit-cli download --json` carries `p90_ms`, `p99_ms`, and
             `total_ms` for progress, and the tail is visible as the difference.
