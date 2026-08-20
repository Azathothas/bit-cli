# The JSON contract

`bit-cli --schema-version` prints the version of everything below. This file is
what that number refers to.

Two surfaces, and they never mix. `--json` writes one document to stdout when
the run ends. `--jsonl` writes one object per line as things happen. stdout
carries data only in both, at every log level, so `bit-cli ... --json | jq`
never sees a log line.

Every document carries four fields before its own: `schema_version`,
`bit_cli_version`, `generated_at`, and `kind`. Every event carries `type`,
`seq`, and `at`.

Sizes and durations are always an integer plus a rendered string, never the
string alone: `{"bytes": 1048576, "human": "1.00 MiB"}` and
`{"ms": 1500, "human": "1s"}`. Rates use the same shape as a size with
`MiB/s` in the string. Timestamps are ISO 8601 UTC with millisecond precision.

## How this file is kept true

It is generated from what the program actually writes. A test drives every
command, flattens the JSON it produced, renders this file, and fails when the
result differs from what is committed. A field added to a report therefore
fails the build until this file is regenerated:

```bash
BIT_CLI_UPDATE_SCHEMA=1 cargo test -p bit-cli --lib schema
```

A field that a given run did not produce is not listed. Optional fields are
omitted from the JSON rather than written as `null`, so a reader cannot mistake
"not applicable" for "none", and several runs of the same command are folded
together here to cover as many of them as possible.

## Documents

One document per run, on stdout, when `--json` is given.

### `info`

One torrent's metadata, without touching the network.

From `bit-cli info <TORRENT> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `file_count` | integer |
| `generated_at` | string |
| `http_seeds[]` | array |
| `info_hash` | string |
| `kind` | string |
| `magnet` | string |
| `multi_file` | bool |
| `name` | string |
| `nodes[]` | array |
| `piece_count` | integer |
| `piece_length.bytes` | integer |
| `piece_length.human` | string |
| `private` | bool |
| `schema_version` | string |
| `source_kind` | string |
| `total.bytes` | integer |
| `total.human` | string |
| `trackers[][]` | string |
| `web_seeds[]` | string |

### `files`

The files in a torrent, with sizes, offsets, and piece ranges.

From `bit-cli files <TORRENT> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `file_count` | integer |
| `files[].first_piece` | integer |
| `files[].index` | integer |
| `files[].last_piece` | integer |
| `files[].offset` | integer |
| `files[].padding` | bool |
| `files[].path` | string |
| `files[].share` | string |
| `files[].shared[].bytes_proven.bytes` | integer |
| `files[].shared[].bytes_proven.human` | string |
| `files[].shared[].evidence` | string |
| `files[].shared[].index` | integer |
| `files[].shared[].info_hash` | string |
| `files[].shared[].path` | string |
| `files[].shared[].pieces_compared` | integer |
| `files[].shared[].proven` | bool |
| `files[].shared[].torrent` | string |
| `files[].size.bytes` | integer |
| `files[].size.human` | string |
| `generated_at` | string |
| `info_hash` | string |
| `kind` | string |
| `name` | string |
| `schema_version` | string |
| `total.bytes` | integer |
| `total.human` | string |

### `magnet`

A magnet URI built from a torrent, and its parts.

From `bit-cli magnet <TORRENT> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `generated_at` | string |
| `info_hash` | string |
| `kind` | string |
| `length.bytes` | integer |
| `length.human` | string |
| `magnet` | string |
| `name` | string |
| `peers[]` | array |
| `schema_version` | string |
| `selected_files[]` | array |
| `trackers[]` | string |
| `web_seeds[]` | string |

### `verify`

What a hash check of existing data found, piece by piece.

From `bit-cli verify <TORRENT> --dir <DIR> --json`.

| field | type |
| --- | --- |
| `bad_pieces[]` | array |
| `bit_cli_version` | string |
| `complete` | bool |
| `data_dir` | string |
| `files[].expected.bytes` | integer |
| `files[].expected.human` | string |
| `files[].found.bytes` | integer |
| `files[].found.human` | string |
| `files[].index` | integer |
| `files[].path` | string |
| `files[].present` | bool |
| `generated_at` | string |
| `have.bytes` | integer |
| `have.human` | string |
| `have_share` | string |
| `info_hash` | string |
| `kind` | string |
| `name` | string |
| `piece_count` | integer |
| `pieces_bad` | integer |
| `pieces_ok` | integer |
| `schema_version` | string |
| `total.bytes` | integer |
| `total.human` | string |

### `hash_mismatch`

The document `verify` writes instead when a piece did not check out.

From `bit-cli verify <TORRENT> --dir <DIR> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `code` | integer |
| `context.bad_pieces[]` | integer |
| `context.pieces_bad` | integer |
| `context.pieces_ok` | integer |
| `context.report.bad_pieces[]` | integer |
| `context.report.complete` | bool |
| `context.report.data_dir` | string |
| `context.report.files[].expected.bytes` | integer |
| `context.report.files[].expected.human` | string |
| `context.report.files[].found.bytes` | integer |
| `context.report.files[].found.human` | string |
| `context.report.files[].index` | integer |
| `context.report.files[].path` | string |
| `context.report.files[].present` | bool |
| `context.report.have.bytes` | integer |
| `context.report.have.human` | string |
| `context.report.have_share` | string |
| `context.report.info_hash` | string |
| `context.report.name` | string |
| `context.report.piece_count` | integer |
| `context.report.pieces_bad` | integer |
| `context.report.pieces_ok` | integer |
| `context.report.total.bytes` | integer |
| `context.report.total.human` | string |
| `generated_at` | string |
| `kind` | string |
| `message` | string |
| `schema_version` | string |

### `create`

A torrent that was just written, and what went into it.

From `bit-cli create <DIR> --output <TORRENT> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `file_count` | integer |
| `generated_at` | string |
| `info_hash` | string |
| `kind` | string |
| `magnet` | string |
| `name` | string |
| `output` | string |
| `piece_count` | integer |
| `piece_length.bytes` | integer |
| `piece_length.human` | string |
| `piece_length_reason` | string |
| `private` | bool |
| `schema_version` | string |
| `total.bytes` | integer |
| `total.human` | string |
| `written` | bool |

### `edit`

A torrent rewritten with new trackers or sources, and its info hash before and after.

From `bit-cli edit <TORRENT> --announce <URL> --force --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `changes[]` | string |
| `generated_at` | string |
| `http_seeds[]` | array |
| `info_hash_after` | string |
| `info_hash_before` | string |
| `info_hash_changed` | bool |
| `input` | string |
| `kind` | string |
| `output` | string |
| `schema_version` | string |
| `trackers[][]` | string |
| `web_seeds[]` | array |
| `written` | bool |

### `download`

A finished download: what arrived, from where, and what it cost.

From `bit-cli download <TORRENT> --web-seed <URL> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `completed` | integer |
| `downloaded.bytes` | integer |
| `downloaded.human` | string |
| `elapsed_human` | string |
| `elapsed_ms` | integer |
| `failed` | integer |
| `from_peers.bytes` | integer |
| `from_peers.human` | string |
| `from_resume.bytes` | integer |
| `from_resume.human` | string |
| `from_web_seeds.bytes` | integer |
| `from_web_seeds.human` | string |
| `generated_at` | string |
| `kind` | string |
| `process.cpu_ms` | integer |
| `process.cpu_system_ms` | integer |
| `process.cpu_user_ms` | integer |
| `process.open_handles` | integer |
| `process.peak_rss_bytes` | integer |
| `process.rss_bytes` | integer |
| `schema_version` | string |
| `torrents[].code` | string |
| `torrents[].downloaded.bytes` | integer |
| `torrents[].downloaded.human` | string |
| `torrents[].elapsed_human` | string |
| `torrents[].elapsed_ms` | integer |
| `torrents[].finished` | bool |
| `torrents[].from_peers.bytes` | integer |
| `torrents[].from_peers.human` | string |
| `torrents[].from_resume.bytes` | integer |
| `torrents[].from_resume.human` | string |
| `torrents[].from_web_seeds.bytes` | integer |
| `torrents[].from_web_seeds.human` | string |
| `torrents[].info_hash` | string |
| `torrents[].mean_rate.bytes` | integer |
| `torrents[].mean_rate.human` | string |
| `torrents[].mean_rate_human` | string |
| `torrents[].name` | string |
| `torrents[].output_directory` | string |
| `torrents[].peers_seen` | integer |
| `torrents[].source` | string |
| `torrents[].sources[].blocks` | integer |
| `torrents[].sources[].connections` | integer |
| `torrents[].sources[].error` | string |
| `torrents[].sources[].http_bytes` | integer |
| `torrents[].sources[].http_requests` | integer |
| `torrents[].sources[].index` | integer |
| `torrents[].sources[].origin` | string |
| `torrents[].sources[].retries` | integer |
| `torrents[].sources[].scope` | string |
| `torrents[].sources[].served_bytes` | integer |
| `torrents[].sources[].served_human` | string |
| `torrents[].sources[].state` | string |
| `torrents[].sources[].url` | string |
| `torrents[].sources[].whole_pieces` | integer |
| `torrents[].stopped` | string |
| `torrents[].total.bytes` | integer |
| `torrents[].total.human` | string |
| `torrents[].uploaded.bytes` | integer |
| `torrents[].uploaded.human` | string |
| `total.bytes` | integer |
| `total.human` | string |

### `seed`

A finished seeding run: who connected and what they took.

From `bit-cli seed <TORRENT> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `complete` | bool |
| `data_directory` | string |
| `elapsed_human` | string |
| `elapsed_ms` | integer |
| `generated_at` | string |
| `have.bytes` | integer |
| `have.human` | string |
| `info_hash` | string |
| `kind` | string |
| `listen_addr` | string |
| `mean_upload_rate.bytes` | integer |
| `mean_upload_rate.human` | string |
| `mean_upload_rate_human` | string |
| `name` | string |
| `peers[]` | array |
| `peers_seen` | integer |
| `peers_served` | integer |
| `process.cpu_ms` | integer |
| `process.cpu_system_ms` | integer |
| `process.cpu_user_ms` | integer |
| `process.open_handles` | integer |
| `process.peak_rss_bytes` | integer |
| `process.rss_bytes` | integer |
| `ratio` | string |
| `schema_version` | string |
| `stopped` | string |
| `total.bytes` | integer |
| `total.human` | string |
| `trackers[]` | string |
| `uploaded.bytes` | integer |
| `uploaded.human` | string |
| `uploaded_human` | string |

### `peers`

The swarm as sampled over a window.

Not covered by the generator yet, so its fields are not listed here. See
`docs/schema.md`'s note above.

### `trackers`

What each tracker answered.

Not covered by the generator yet, so its fields are not listed here. See
`docs/schema.md`'s note above.

### `webseed_list`

Every source binding resolved to the exact URLs it would request.

From `bit-cli webseed list <TORRENT> --web-seed <URL> --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `complete` | bool |
| `covered.bytes` | integer |
| `covered.human` | string |
| `generated_at` | string |
| `info_hash` | string |
| `kind` | string |
| `name` | string |
| `piece_count` | integer |
| `schema_version` | string |
| `source_count` | integer |
| `sources[].files[]` | integer |
| `sources[].in_scope.bytes` | integer |
| `sources[].in_scope.human` | string |
| `sources[].in_scope_share` | string |
| `sources[].index` | integer |
| `sources[].mode` | string |
| `sources[].origin` | string |
| `sources[].partial_pieces` | integer |
| `sources[].priority` | integer |
| `sources[].scope` | string |
| `sources[].style` | string |
| `sources[].url` | string |
| `sources[].urls[].file` | integer |
| `sources[].urls[].in_scope.bytes` | integer |
| `sources[].urls[].in_scope.human` | string |
| `sources[].urls[].path` | string |
| `sources[].urls[].size.bytes` | integer |
| `sources[].urls[].size.human` | string |
| `sources[].urls[].url` | string |
| `sources[].whole_pieces` | integer |
| `total.bytes` | integer |
| `total.human` | string |
| `uncovered.bytes` | integer |
| `uncovered.human` | string |
| `uncovered_pieces[]` | array |

### `webseed_test`

One request per source: status, ranges, redirects, and timing.

Not covered by the generator yet, so its fields are not listed here. See
`docs/schema.md`'s note above.

### `webseed_probe`

A source measured at several concurrencies.

Not covered by the generator yet, so its fields are not listed here. See
`docs/schema.md`'s note above.

### `webseed_fetch`

One piece pulled from one source and checked.

Not covered by the generator yet, so its fields are not listed here. See
`docs/schema.md`'s note above.

### `config`

Configuration as resolved, with where each value came from.

From `bit-cli config show --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `files_missing[]` | string |
| `files_read[]` | array |
| `generated_at` | string |
| `kind` | string |
| `schema_version` | string |
| `settings.color.origin.kind` | string |
| `settings.color.value` | string |
| `settings.download_directory.origin.kind` | string |
| `settings.download_directory.value` | string |
| `settings.enable_dht.origin.kind` | string |
| `settings.enable_dht.value` | string |
| `settings.enable_lsd.origin.kind` | string |
| `settings.enable_lsd.value` | string |
| `settings.enable_pex.origin.kind` | string |
| `settings.enable_pex.value` | string |
| `settings.enable_web_seeds.origin.kind` | string |
| `settings.enable_web_seeds.value` | string |
| `settings.file_allocation.origin.kind` | string |
| `settings.file_allocation.value` | string |
| `settings.listen_port.origin.kind` | string |
| `settings.listen_port.value` | string |
| `settings.log_format.origin.kind` | string |
| `settings.log_format.value` | string |
| `settings.log_level.origin.kind` | string |
| `settings.log_level.value` | string |
| `settings.max_concurrent_downloads.origin.kind` | string |
| `settings.max_concurrent_downloads.value` | string |
| `settings.max_download_rate.origin.kind` | string |
| `settings.max_download_rate.value` | string |
| `settings.max_peers.origin.kind` | string |
| `settings.max_peers.value` | string |
| `settings.max_peers_total.origin.kind` | string |
| `settings.max_peers_total.value` | string |
| `settings.max_upload_rate.origin.kind` | string |
| `settings.max_upload_rate.value` | string |
| `settings.piece_selector.origin.kind` | string |
| `settings.piece_selector.value` | string |
| `settings.seed_ratio.origin.kind` | string |
| `settings.seed_ratio.value` | string |
| `settings.seed_time.origin.kind` | string |
| `settings.seed_time.value` | string |
| `settings.web_seed_chunk_size.origin.kind` | string |
| `settings.web_seed_chunk_size.value` | string |
| `settings.web_seed_concurrency.origin.kind` | string |
| `settings.web_seed_concurrency.value` | string |
| `settings.web_seed_timeout.origin.kind` | string |
| `settings.web_seed_timeout.value` | string |
| `settings.web_seed_user_agent.origin.kind` | string |
| `settings.web_seed_user_agent.value` | string |

### `version`

The build, its features, and the exit code table.

From `bit-cli version --json`.

| field | type |
| --- | --- |
| `bit_cli_version` | string |
| `composition_modes[]` | string |
| `exit_codes[].code` | integer |
| `exit_codes[].description` | string |
| `exit_codes[].kind` | string |
| `features[]` | string |
| `generated_at` | string |
| `kind` | string |
| `lints[]` | string |
| `schema_version` | string |
| `target` | string |
| `trace_subsystems[].description` | string |
| `trace_subsystems[].name` | string |
| `version` | string |

## Events

One object per line, on stdout, when `--jsonl` is given. Every event carries
`type`, `seq`, and `at` before its own fields; `seq` counts from zero within a
run and `at` is ISO 8601 UTC with millisecond precision.

### `session_start`

The session is up. Carries the listen address and what it was asked to do.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `data_directory` | string |
| `directory` | string |
| `listen_addr` | string |
| `max_concurrent_downloads` | integer |
| `seq` | integer |
| `source` | string |
| `sources` | integer |
| `type` | string |

### `torrent_added`

A source resolved to a torrent and was added to the session.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `info_hash` | string |
| `name` | string |
| `seq` | integer |
| `source` | string |
| `type` | string |

### `metadata_resolved`

The torrent's metadata is known: name, files, pieces.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `files` | integer |
| `info_hash` | string |
| `name` | string |
| `piece_count` | integer |
| `piece_length` | integer |
| `seq` | integer |
| `total_bytes` | integer |
| `type` | string |

### `source_added`

An HTTP or `file:` source was attached, with its scope.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `index` | integer |
| `origin` | string |
| `scope` | string |
| `seq` | integer |
| `type` | string |
| `url` | string |
| `whole_pieces` | integer |

### `source_failed`

A source spent its error budget and is out for the run.

Not produced by any run the generator drives, so its fields are not listed
here.

### `source_cooling`

A source spent its error budget and will be tried again after `--web-seed-cooldown`.

Not produced by any run the generator drives, so its fields are not listed
here.

### `peer_redial`

`--redial-after` fired: every peer connection was dropped and the peer list dialled again.

From `bit-cli download <TORRENT> --redial-after <DUR> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `at_ms` | integer |
| `attempt` | integer |
| `peers_dropped` | integer |
| `seq` | integer |
| `stalled_ms` | integer |
| `type` | string |

### `piece_verified`

A piece arrived and its hash checked out.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `length` | integer |
| `piece` | integer |
| `seq` | integer |
| `type` | string |

### `file_completed`

Every piece of one file is present.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `file` | integer |
| `length` | integer |
| `path` | string |
| `seq` | integer |
| `type` | string |

### `progress`

A tick of the report interval: rates, peers, and what the process costs.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `download_rate` | integer |
| `eta_confidence` | string |
| `eta_ms` | null |
| `from_web_seeds` | integer |
| `info_hash` | string |
| `peer_detail[]` | array |
| `peers.connecting` | integer |
| `peers.dead` | integer |
| `peers.live` | integer |
| `peers.queued` | integer |
| `peers.seen` | integer |
| `percent` | string |
| `process.cpu_ms` | integer |
| `process.cpu_system_ms` | integer |
| `process.cpu_user_ms` | integer |
| `process.open_handles` | integer |
| `process.peak_rss_bytes` | integer |
| `process.rss_bytes` | integer |
| `progress_bytes` | integer |
| `ratio` | string |
| `seq` | integer |
| `total_bytes` | integer |
| `type` | string |
| `upload_rate` | integer |
| `uploaded_bytes` | integer |

### `bench_sample`

One point of a `bench` time series.

Not produced by any run the generator drives, so its fields are not listed
here.

### `torrent_completed`

One torrent finished, with its totals.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `downloaded_bytes` | integer |
| `elapsed_ms` | integer |
| `finished` | bool |
| `from_peers` | integer |
| `from_resume` | integer |
| `from_web_seeds` | integer |
| `info_hash` | string |
| `name` | string |
| `seq` | integer |
| `stopped` | string |
| `type` | string |

### `error`

Something failed. The same shape the final error document carries.

From `bit-cli download <TORRENT> --no-continue --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `code` | integer |
| `context.source` | string |
| `kind` | string |
| `message` | string |
| `seq` | integer |
| `type` | string |

### `session_end`

The run is over. Always last, always present, whatever happened.

From `bit-cli download <TORRENT> --web-seed <URL> --jsonl`.

| field | type |
| --- | --- |
| `at` | string |
| `elapsed_human` | string |
| `elapsed_ms` | integer |
| `error` | string |
| `exit_code` | integer |
| `exit_status` | string |
| `ok` | bool |
| `seq` | integer |
| `type` | string |
