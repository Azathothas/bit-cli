# Tracker tiers, the BEP 15 backoff, and the scrape convention

Moved out of `RESEARCH.md` on 2026-08-24. All three entries closed.
`--announce-tier` and `--tracker-interval` are in `man/bit-cli.json`,
`scripts/check-udp-retry.ps1` is the acceptance for the backoff and
`scripts/check-tracker-family.ps1` for the family split.

Nothing here is current. It is kept because its citations, line numbers and
issue references still resolve, and because a later session asking "where did
this come from" should find the source rather than re-derive it.

Closed by: **T-063, T-064 and T-065**.

---

## From 1. `torrent` — anacrolix/torrent

### Trackers

- `torrent/tracker/udp/timeout.go:9` — BEP 15 backoff is `15 * 2^n` seconds,
  clamped at `n = 8` (3840 s). **This is what T-064 asks for**, in nine lines.
- `torrent/tracker/udp/client.go` — connection-id caching with a one-minute
  reissue rule (`shouldReconnectDefault`), and an explicit workaround: the
  literal error body `"Connection ID missmatch.\x00"` from
  `tracker.torrent.eu.org:451` forces `connIdIssued` to zero so the next request
  reconnects.
- `torrent/tracker/udp/scrape.go:7-12` — the BEP 48 field-name mismatch is
  documented in-code: UDP calls them seeders/completed/leechers, bencode calls
  them `complete`/`downloaded`/`incomplete`.
- `torrent/tracker/http/scrape.go` — derives the scrape URL with
  `url.JoinPath("..", "scrape")`, i.e. the BEP 48 `/announce`→`/scrape`
  convention that T-065 notes as the only one implemented. No other convention
  exists here either, which is corroboration rather than a fix.
- `torrent/metainfo/announcelist.go` — `OverridesAnnounce` decides whether
  `announce-list` supersedes `announce` (it does unless every tier entry is
  empty); `DistinctValues` flattens with de-duplication.

---

## From 3. `TorrentNG` — snapetech/TorrentNG

### BEP 12 tier order (T-063)

`TorrentNG/crates/rt-tracker/src/tier.rs`:

- `:8` `Tier { trackers, active }`, `:55` `TierSet { tiers, active_tier }`.
- `promote_active()` — **on success, swap the successful tracker to the front of
  its tier**, which is the BEP 12 rule `bit-cli` is missing.
- `advance()` — on failure, move to the next tracker in the tier, then the next
  tier.
- The doc comment notes private torrents (BEP 27) put extra constraints on tier
  switching.

---

## From 3. `TorrentNG` — snapetech/TorrentNG

### Tracker backoff and announce storms

`TorrentNG/crates/rt-tracker/src/backoff.rs`:

- `:28` `Backoff::tracker_retry()` — base 60 s, double, cap 1800 s, ±20 % jitter
  (`:38` `next_delay`).
- `:58` `jitter_interval(interval, fraction)` — spreads announces when many
  torrents load at once ("for 15k torrents with a 30-min interval, this spreads
  announces over ±6 minutes"). `bit-cli download` takes any number of sources
  with `-j`; the same storm applies at a smaller scale.

---

## From 11. `mtorrent` — DanglingPointer/mtorrent

### Trackers

`mtorrent-core/src/trackers/udp.rs:150` — `MAX_RETRANSMISSIONS = 3`, and `:160`
`timeout_sec = 15 * (1 << retransmit_n)`, i.e. 15/30/60/120 s, "timeout after
225s". A shorter variant of the BEP 15 ladder than anacrolix's; both are
defensible, and `bit-cli` should pick one and document the total budget.
`mtorrent/mtorrent-core/src/trackers/mod.rs` defines the request/response types
including `num_want` and the three announce events.

---
