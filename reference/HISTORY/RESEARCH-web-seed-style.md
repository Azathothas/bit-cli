# Web seed style detection and source lifecycle

Moved out of `RESEARCH.md` on 2026-08-24. Both entries closed and the
behaviour is in `man/bit-cli.json`: `--web-seed-style` takes `auto`,
`getright` or `hoffman` and defaults to `auto`, and `--web-seed-cooldown`,
`--web-seed-max-errors`, `--web-seed-retry-status` and
`--web-seed-fatal-status` all exist. `scripts/check-signed-source.ps1` is the
acceptance for the lifecycle half.

Nothing here is current. It is kept because its citations, line numbers and
issue references still resolve, and because a later session asking "where did
this come from" should find the source rather than re-derive it.

Closed by: **T-004, T-130 and T-137**.

---

## From 8. `gosh-dl` — goshitsarch-eng/gosh-dl

### BEP 17 auto-detection — the answer to T-004

This is the only repository here that implements **both** web-seed styles, and
its structure hands `bit-cli` the auto-detection rule directly:

- `gosh-dl/src/torrent/metainfo.rs:125` reads `url-list` and `:128` reads
  `httpseeds` **into two separate fields** (`:36` `url_list`, `:38`
  `httpseeds`), using one shared parser `:391` `parse_url_list` that accepts a
  bencoded string *or* a list and filters to `http://`/`https://`.
- `gosh-dl/src/torrent/webseed.rs:24` `WebSeedType { GetRight, Hoffman }`.
- `:587` `build_piece_url` — GetRight: the URL itself for a single-file torrent,
  otherwise a per-file URL; **Hoffman: `{url}?info_hash={urlencoded}&piece={index}`**.
- `:618` `build_file_url` — trims a trailing `/`, percent-encodes each path
  component, joins with `/`.
- `:662` `download_multifile_piece` — a piece spanning several files becomes one
  ranged `GET` per file, concatenated and then SHA-1'd against the piece hash.
  Every request sets **`Accept-Encoding: identity`** so a transcoding proxy
  cannot silently change the byte range.

**The bug to not copy, and the fix `bit-cli` gets for free:** at `:303` the
manager builds every seed with `let seed_type = WebSeedType::GetRight;` under
the comment "Hoffman-style seeds typically end with specific paths", and
`:479` `all_webseeds()` merges `url_list` and `httpseeds` into one list — so the
style information that was correctly parsed is thrown away before it is used.
`bit-cli` can close T-004 by keying the style off *which metainfo key the URL
came from*, which is what BEP 17 and BEP 19 actually specify, and keeping
`--web-seed-mode` as the override.

---

## From 8. `gosh-dl` — goshitsarch-eng/gosh-dl

### Source lifecycle

`gosh-dl/src/torrent/webseed.rs:33` `WebSeedState { Idle, Downloading, Backoff, Failed }`, `gosh-dl/src/torrent/webseed.rs:138-181`
exponential backoff `initial * 2^min(consecutive-1, 6)` with ±25 % jitter,
capped at `max_backoff` (defaults at `gosh-dl/src/torrent/webseed.rs:239-241`:
5 s initial, 300 s max, 5 consecutive failures), and a `max_failures`
retirement. Six unit tests at `gosh-dl/src/torrent/webseed.rs:782-951` cover state transitions,
backoff growth, cap, and reset-on-success. Compare `bit-cli`'s
`--web-seed-retries` / `--web-seed-max-errors` / `--web-seed-cooldown`: same
model, and the tests are a good template.

---
