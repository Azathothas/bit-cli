# bit-cli(0.2.0)

Fetch, create, verify, and seed torrents, with first-class web seed control.

This file is generated from the command definition by `bit-cli man --format markdown`. Do not edit it: `cargo test -p bit-cli` fails when it stops describing the binary. The same surface is available as a man page in `bit-cli.1` and, for a program, as a CLIspec document in `bit-cli.json`.

## Global options

These are accepted by every command.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `--json` | boolean |  | `false` | Emit machine-readable JSON on stdout. Implies --progress=none |
| `--jsonl` | boolean |  | `false` | Emit newline-delimited JSON events on stdout as they happen |
| `--schema-version` | boolean |  | `false` | Print the output schema version and exit |
| `--quiet`, `-q` | boolean |  | `false` | Suppress all non-error output |
| `--verbose`, `-v` | integer |  | `0` | Increase verbosity. Repeatable: -v, -vv, -vvv |
| `--log-level <LEVEL>` | string | `off`, `error`, `warn`, `info`, `debug`, `trace` | `warn` | Log level |
| `--log-format <FMT>` | string | `text`, `json` | `text` | Log format |
| `--log-file <PATH>`, `-l` | string |  |  | Append logs to a file. Rotates by size and count |
| `--log-max-size <SIZE>` | string |  | `16MiB` | Rotate the log at this size. `0` never rotates |
| `--log-max-files <N>` | string |  | `5` | Keep this many logs in total, the live one included |
| `--trace <SUBSYSTEM>` | array |  |  | Enable detailed tracing for one subsystem without raising the global level |
| `--no-redact` | boolean |  | `false` | Show credentials in trace output instead of redacting them |
| `--color <WHEN>` | string | `auto`, `always`, `never` | `auto` | When to use colour. Honours NO_COLOR |
| `--progress <MODE>` | string | `auto`, `none`, `plain`, `json` | `auto` | Progress rendering. Defaults to none when stdout is not a terminal |
| `--config <PATH>` | string |  |  | Config file path |
| `--no-config` | boolean |  | `false` | Ignore all config files |
| `--dir <DIR>`, `-d` | string |  |  | Output directory |
| `--dry-run` | boolean |  | `false` | Resolve, validate, and report. Write nothing |
| `--stats` | boolean |  | `false` | Print every field of the report, rather than the usual summary |
| `--timeout <DUR>` | string |  |  | Overall operation deadline |
| `--stop-after <DUR>` | string |  |  | Stop after this long regardless of state |

## Commands

| command | effects | what it does |
| --- | --- | --- |
| `bit-cli` | idempotent | Fetch, create, verify, and seed torrents, with first-class web seed control. |
| `bit-cli download` | idempotent | Fetch to completion in the foreground, then exit |
| `bit-cli info` | read_only | Parse a torrent, magnet, or metalink and print its metadata |
| `bit-cli files` | read_only | List files with index, path, size, and priority |
| `bit-cli tree` | read_only | Print the torrent's directory structure, rolled up |
| `bit-cli peers` | read_only | Connect, sample the swarm, report peers, then exit |
| `bit-cli trackers` | read_only | Announce or scrape, report the result, then exit |
| `bit-cli webseed` | read_only | Inspect, validate, and read from HTTP sources |
| `bit-cli webseed list` | read_only | Resolve every binding and print the exact URL each file maps to. No network |
| `bit-cli webseed test` | read_only | Probe each source: range support, size, redirects, TLS, latency |
| `bit-cli webseed probe` | read_only | Measure ranged-GET latency and throughput as concurrency scales |
| `bit-cli webseed fetch` | idempotent | Fetch one range from one source and verify it against the torrent |
| `bit-cli verify` | read_only | Hash-check existing data against the torrent |
| `bit-cli create` | non_idempotent | Create a .torrent |
| `bit-cli edit` | non_idempotent | Rewrite metainfo fields on an existing .torrent, writing a new file |
| `bit-cli magnet` | read_only | Convert a torrent to a magnet URI, or resolve a magnet to metadata |
| `bit-cli seed` | read_only | Seed existing data in the foreground |
| `bit-cli bench` | non_idempotent | Measure a target |
| `bit-cli bench leech` | non_idempotent | Download from a target and measure |
| `bit-cli bench seed` | non_idempotent | Seed and measure what the swarm pulls |
| `bit-cli bench webseed` | non_idempotent | Measure HTTP sources: latency percentiles, concurrency scaling, ranges |
| `bit-cli bench disk` | non_idempotent | Measure the payload file under several writers, with no session |
| `bit-cli bench swarm` | non_idempotent | Synthetic peer load against a target |
| `bit-cli bench probe` | read_only | One-shot capability and reachability probe |
| `bit-cli config` | read_only | Configuration |
| `bit-cli config show` | read_only | Print the fully resolved configuration with the origin of every value |
| `bit-cli completions` | read_only | Generate shell completions |
| `bit-cli man` | read_only | Generate a man page |
| `bit-cli version` | read_only | Version, build metadata, enabled features, and protocol support |

`effects` is CLIspec's word for what running the command does: `read_only` is safe to run to find something out, `idempotent` can be retried after a failure, and `non_idempotent` cannot.

### `bit-cli`

Fetch, create, verify, and seed torrents, with first-class web seed control.

Effects: `idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `sources <SOURCE>` | array |  |  | Sources to download when no subcommand is given |

### `bit-cli download`

Fetch to completion in the foreground, then exit

Effects: `idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `sources <SOURCE>` | array |  |  | Sources to fetch |
| `--web-seed <URL>` | array |  |  | Source for the whole torrent, under the current composition mode |
| `--web-seed-exact <URL>` | array |  |  | Shorthand for a source with composition=exact |
| `--web-seed-for <SEL=URL>` | array |  |  | Bind a scope selector to a source, as SELECTOR=URL |
| `--web-seed-mode <MODE>` | string | `auto`, `exact`, `prefix`, `template` | `auto` | Composition mode for CLI-supplied sources |
| `--web-seed-template <TMPL>` | string |  |  | Template used when the mode is `template` |
| `--web-seed-pieces <RANGE>` | string |  |  | Restrict CLI-supplied sources to these piece indices |
| `--web-seed-bytes <RANGE>` | string |  |  | Restrict CLI-supplied sources to this byte range of the payload |
| `--web-seed-file <PATH>` | array |  |  | One URL per line. Blank lines and # comments are ignored |
| `--web-seed-list-url <URL>` | array |  |  | Fetch a newline-separated URL list over HTTP |
| `--web-seed-config <PATH>` | array |  |  | TOML or JSON binding table. Full control |
| `--web-seed-style <STYLE>` | string | `auto`, `getright`, `hoffman` | `auto` | BEP 19 or BEP 17 wire style |
| `--web-seed-only` | boolean |  | `false` | Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only |
| `--no-web-seed` | boolean |  | `false` | Ignore all web seeds, including the torrent's own url-list |
| `--no-torrent-web-seed` | boolean |  | `false` | Ignore the torrent's url-list but keep CLI-supplied sources |
| `--web-seed-concurrency <N>` | string |  |  | Concurrent ranged requests per source |
| `--max-connection-per-server <N>`, `-x` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. Per source, not per server |
| `--split <N>`, `-s` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. The same knob as `-x` |
| `--web-seed-connections <N>` | string |  |  | Peer connections each source is presented over. Default: 1 |
| `--web-seed-max-total <N>` | string |  |  | Concurrent ranged requests across all sources |
| `--web-seed-chunk-size <SIZE>` | string |  |  | Bytes per ranged request. Independent of the torrent's piece length |
| `--min-split-size <SIZE>`, `-k` | string |  |  | `aria2` spelling of a floor under `--web-seed-chunk-size` |
| `--web-seed-timeout <DUR>` | string |  |  | Per-request timeout |
| `--web-seed-connect-timeout <DUR>` | string |  |  | Connect timeout for web seed requests |
| `--web-seed-max-errors <N>` | string |  |  | Consecutive failed requests before a source is retired |
| `--web-seed-cooldown <DUR>` | string |  |  | Give a source that spent its error budget another chance after this long. Zero, the default, means it does not come back |
| `--web-seed-retries <N>` | string |  |  | Per-request retries before counting an error |
| `--web-seed-retry-status <CODES>` | string |  |  | Statuses to retry that would otherwise retire the source |
| `--web-seed-fatal-status <CODES>` | string |  |  | Statuses that retire the source, which would otherwise be retried |
| `--web-seed-user-agent <UA>` | string |  |  | User-Agent for web seed requests |
| `--web-seed-header <K: V>` | array |  |  | Extra header on web seed requests, as `Name: value` |
| `--web-seed-auth <SPEC>` | string |  |  | Credentials: basic:user:pass, bearer:TOKEN, netrc, or none |
| `--web-seed-speed-limit <RATE>` | string |  |  | Rate cap per source |
| `--web-seed-verify <MODE>` | string | `piece`, `file`, `none` | `piece` | When to hash-check HTTP-sourced data |
| `--web-seed-priority <N>` | string |  |  | Bias among sources. Higher wins when several can serve a piece |
| `--prefer-web-seed` | boolean |  | `false` | Bias the picker toward HTTP when both a peer and a source have a piece |
| `--web-seed-require` | boolean |  | `false` | Fail the run if a declared source turns out to be unusable |
| `--tracker <URL>` | array |  |  | Add a tracker at runtime. The .torrent is never rewritten |
| `--tracker-file <PATH>` | array |  |  | One tracker per line. A blank line separates BEP 12 tiers |
| `--tracker-list-url <URL>` | array |  |  | Fetch a tracker list over HTTP |
| `--exclude-tracker <URL>` | array |  |  | Remove trackers. `*` removes all |
| `--replace-trackers` | boolean |  | `false` | Replace the torrent's tracker list instead of adding to it |
| `--tracker-timeout <DUR>` | string |  |  | Tracker request timeout |
| `--tracker-connect-timeout <DUR>` | string |  |  | Tracker connect timeout |
| `--tracker-interval <DUR>` | string |  |  | Override the announce interval |
| `--no-tracker` | boolean |  | `false` | Disable tracker announces entirely |
| `--max-download-rate <RATE>` | string |  |  | Download rate cap, per torrent |
| `--max-upload-rate <RATE>`, `-u` | string |  |  | Upload rate cap, per torrent |
| `--max-overall-download-rate <RATE>` | string |  |  | Download rate cap across the whole run |
| `--max-overall-upload-rate <RATE>` | string |  |  | Upload rate cap across the whole run |
| `--max-peer-rate <RATE>` | string |  |  | Download rate cap for swarm peers, not for attached HTTP sources |
| `--max-peers <N>` | string |  |  | Peer connections per torrent |
| `--max-peers-total <N>` | string |  |  | Peer connections across the run |
| `--encryption <MODE>` | string | `off`, `prefer`, `require` | `prefer` | Message stream encryption, for peer connections in both directions |
| `--transport <MODE>` | string | `tcp`, `utp`, `both` | `tcp` | Which transports this run listens on and dials |
| `--block-peer <ADDR>` | array |  |  | Refuse this peer for the whole run. Repeatable |
| `--max-open-files <N>` | string |  | `128` | Payload files kept open at once |
| `--max-handles <N>` | string |  |  | Stop when the process holds more than this many handles. Off by default |
| `--max-rss <SIZE>` | string |  |  | Stop when the process holds more than this much resident memory. Off by default |
| `--seed-ratio <RATIO>` | string |  |  | Stop seeding at this ratio. 0 means do not seed |
| `--seed-time <DUR>` | string |  |  | Stop seeding after this long |
| `--stop-timeout <DUR>` | string |  |  | Give up if there is no progress for this long |
| `--init-timeout <DUR>` | string |  | `10m` | Give up if the hash check has not finished in this long |
| `--lowest-speed-limit <RATE>` | string |  |  | Abort if the rate drops below this |
| `--on-complete <COMMAND>` | string |  |  | Run this command on success. Arguments arrive through the environment |
| `--on-error <COMMAND>` | string |  |  | Run this command on failure |
| `--on-piece-verified <COMMAND>` | string |  |  | Run this command after every verified piece. High frequency |
| `--select-file <INDEX>` | array |  |  | Download only these files. Accepts ranges: 1-5,8,10- |
| `--exclude-file <INDEX>` | array |  |  | Skip these files |
| `--index-out <INDEX=PATH>`, `-O` | array |  |  | Rename a file by index, as INDEX=PATH |
| `--out <PATH>`, `-o` | string |  |  | Write the payload here instead of using the torrent's name |
| `--file-allocation <METHOD>` | string | `none`, `prealloc`, `sparse`, `falloc` | `sparse` | How disk space is allocated |
| `--piece-selector <STRATEGY>` | string | `default`, `sequential`, `in-order` | `default` | Which piece to ask for next. `sequential` and `in-order` are the same thing under two names, and both make a download readable front to back |
| `--port <PORT>` | array |  |  | Listen port, or a range as START-END. `0` asks the OS for a free one |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery |
| `--redial-after <DUR>` | string |  |  | Drop every peer connection and dial again after this long with no progress. Off by default |
| `--max-redials <N>` | string |  | `10` | How many times `--redial-after` may fire in one run |
| `--max-concurrent-downloads <N>`, `-j` | string |  | `1` | Sources fetched in parallel within this one invocation |
| `--no-share-files` | boolean |  | `false` | Do not read a file from another torrent in this run that is proven to hold it |
| `--check-integrity`, `-V` | boolean |  | `false` | Hash-check before starting |
| `--hash-check-only` | boolean |  | `false` | Hash-check and exit |
| `--verify-on-complete` | boolean |  | `false` | Re-read the finished payload and report a hash per file |
| `--continue`, `-c` | boolean |  | `false` | Resume a partial download. On by default |
| `--no-continue` | boolean |  | `false` | Refuse to write into a file that is already there |
| `--allow-overwrite` | boolean |  | `false` | Overwrite existing files |
| `--report-interval <DUR>` | string |  | `1s` | Emit a progress event this often |

### `bit-cli info`

Parse a torrent, magnet, or metalink and print its metadata

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli files`

List files with index, path, size, and priority

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--sort <KEY>` | string |  | `index` | Sort key, as KEY or KEY:ORDER. Keys: index, path, size |
| `--against <TORRENT>` | array |  |  | Also report which files another torrent holds identically. Repeatable |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli tree`

Print the torrent's directory structure, rolled up

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--depth <N>` | string |  |  | Stop at this depth and roll the rest up. The root is depth 0 |
| `--no-sizes` | boolean |  | `false` | Print the piece ranges without the size and file count columns |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli peers`

Connect, sample the swarm, report peers, then exit

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--duration <DUR>` | string |  | `15s` | How long to sample the swarm |
| `--count <N>` | string |  |  | Stop once this many distinct peers have been seen |
| `--sort <KEY>` | string |  | `addr` | Sort key, as KEY or KEY:ORDER. Keys: addr, client, speed, pieces |
| `--port <PORT>` | array |  |  | Listen port, or a range as START-END. `0` asks the OS for a free one |
| `--tracker <URL>` | array |  |  | Add a tracker at runtime. The .torrent is never rewritten |
| `--tracker-file <PATH>` | array |  |  | One tracker per line. A blank line separates BEP 12 tiers |
| `--tracker-list-url <URL>` | array |  |  | Fetch a tracker list over HTTP |
| `--exclude-tracker <URL>` | array |  |  | Remove trackers. `*` removes all |
| `--replace-trackers` | boolean |  | `false` | Replace the torrent's tracker list instead of adding to it |
| `--tracker-timeout <DUR>` | string |  |  | Tracker request timeout |
| `--tracker-connect-timeout <DUR>` | string |  |  | Tracker connect timeout |
| `--tracker-interval <DUR>` | string |  |  | Override the announce interval |
| `--no-tracker` | boolean |  | `false` | Disable tracker announces entirely |
| `--max-download-rate <RATE>` | string |  |  | Download rate cap, per torrent |
| `--max-upload-rate <RATE>`, `-u` | string |  |  | Upload rate cap, per torrent |
| `--max-overall-download-rate <RATE>` | string |  |  | Download rate cap across the whole run |
| `--max-overall-upload-rate <RATE>` | string |  |  | Upload rate cap across the whole run |
| `--max-peer-rate <RATE>` | string |  |  | Download rate cap for swarm peers, not for attached HTTP sources |
| `--max-peers <N>` | string |  |  | Peer connections per torrent |
| `--max-peers-total <N>` | string |  |  | Peer connections across the run |
| `--encryption <MODE>` | string | `off`, `prefer`, `require` | `prefer` | Message stream encryption, for peer connections in both directions |
| `--transport <MODE>` | string | `tcp`, `utp`, `both` | `tcp` | Which transports this run listens on and dials |
| `--block-peer <ADDR>` | array |  |  | Refuse this peer for the whole run. Repeatable |
| `--max-open-files <N>` | string |  | `128` | Payload files kept open at once |
| `--max-handles <N>` | string |  |  | Stop when the process holds more than this many handles. Off by default |
| `--max-rss <SIZE>` | string |  |  | Stop when the process holds more than this much resident memory. Off by default |
| `--seed-ratio <RATIO>` | string |  |  | Stop seeding at this ratio. 0 means do not seed |
| `--seed-time <DUR>` | string |  |  | Stop seeding after this long |
| `--stop-timeout <DUR>` | string |  |  | Give up if there is no progress for this long |
| `--init-timeout <DUR>` | string |  | `10m` | Give up if the hash check has not finished in this long |
| `--lowest-speed-limit <RATE>` | string |  |  | Abort if the rate drops below this |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery |

### `bit-cli trackers`

Announce or scrape, report the result, then exit

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--tracker <URL>` | array |  |  | Add a tracker at runtime. The .torrent is never rewritten |
| `--tracker-file <PATH>` | array |  |  | One tracker per line. A blank line separates BEP 12 tiers |
| `--tracker-list-url <URL>` | array |  |  | Fetch a tracker list over HTTP |
| `--exclude-tracker <URL>` | array |  |  | Remove trackers. `*` removes all |
| `--replace-trackers` | boolean |  | `false` | Replace the torrent's tracker list instead of adding to it |
| `--tracker-timeout <DUR>` | string |  |  | Tracker request timeout |
| `--tracker-connect-timeout <DUR>` | string |  |  | Tracker connect timeout |
| `--tracker-interval <DUR>` | string |  |  | Override the announce interval |
| `--no-tracker` | boolean |  | `false` | Disable tracker announces entirely |
| `--scrape` | boolean |  | `false` | Scrape instead of announcing |
| `--scrape-url <URL>` | string |  |  | The scrape endpoint, for a tracker that does not follow BEP 48 |
| `--port <PORT>` | array |  |  | Port to announce, or a range as START-END. `0` asks the OS for a free one |
| `--no-withdraw` | boolean |  | `false` | Announce and leave the peer record behind |
| `--family <FAMILY>` | string | `auto`, `v4`, `v6` | `auto` | Which address family to announce over |

### `bit-cli webseed`

Inspect, validate, and read from HTTP sources

Effects: `read_only`.

Takes no options of its own.

### `bit-cli webseed list`

Resolve every binding and print the exact URL each file maps to. No network

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--web-seed <URL>` | array |  |  | Source for the whole torrent, under the current composition mode |
| `--web-seed-exact <URL>` | array |  |  | Shorthand for a source with composition=exact |
| `--web-seed-for <SEL=URL>` | array |  |  | Bind a scope selector to a source, as SELECTOR=URL |
| `--web-seed-mode <MODE>` | string | `auto`, `exact`, `prefix`, `template` | `auto` | Composition mode for CLI-supplied sources |
| `--web-seed-template <TMPL>` | string |  |  | Template used when the mode is `template` |
| `--web-seed-pieces <RANGE>` | string |  |  | Restrict CLI-supplied sources to these piece indices |
| `--web-seed-bytes <RANGE>` | string |  |  | Restrict CLI-supplied sources to this byte range of the payload |
| `--web-seed-file <PATH>` | array |  |  | One URL per line. Blank lines and # comments are ignored |
| `--web-seed-list-url <URL>` | array |  |  | Fetch a newline-separated URL list over HTTP |
| `--web-seed-config <PATH>` | array |  |  | TOML or JSON binding table. Full control |
| `--web-seed-style <STYLE>` | string | `auto`, `getright`, `hoffman` | `auto` | BEP 19 or BEP 17 wire style |
| `--web-seed-only` | boolean |  | `false` | Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only |
| `--no-web-seed` | boolean |  | `false` | Ignore all web seeds, including the torrent's own url-list |
| `--no-torrent-web-seed` | boolean |  | `false` | Ignore the torrent's url-list but keep CLI-supplied sources |
| `--web-seed-concurrency <N>` | string |  |  | Concurrent ranged requests per source |
| `--max-connection-per-server <N>`, `-x` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. Per source, not per server |
| `--split <N>`, `-s` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. The same knob as `-x` |
| `--web-seed-connections <N>` | string |  |  | Peer connections each source is presented over. Default: 1 |
| `--web-seed-max-total <N>` | string |  |  | Concurrent ranged requests across all sources |
| `--web-seed-chunk-size <SIZE>` | string |  |  | Bytes per ranged request. Independent of the torrent's piece length |
| `--min-split-size <SIZE>`, `-k` | string |  |  | `aria2` spelling of a floor under `--web-seed-chunk-size` |
| `--web-seed-timeout <DUR>` | string |  |  | Per-request timeout |
| `--web-seed-connect-timeout <DUR>` | string |  |  | Connect timeout for web seed requests |
| `--web-seed-max-errors <N>` | string |  |  | Consecutive failed requests before a source is retired |
| `--web-seed-cooldown <DUR>` | string |  |  | Give a source that spent its error budget another chance after this long. Zero, the default, means it does not come back |
| `--web-seed-retries <N>` | string |  |  | Per-request retries before counting an error |
| `--web-seed-retry-status <CODES>` | string |  |  | Statuses to retry that would otherwise retire the source |
| `--web-seed-fatal-status <CODES>` | string |  |  | Statuses that retire the source, which would otherwise be retried |
| `--web-seed-user-agent <UA>` | string |  |  | User-Agent for web seed requests |
| `--web-seed-header <K: V>` | array |  |  | Extra header on web seed requests, as `Name: value` |
| `--web-seed-auth <SPEC>` | string |  |  | Credentials: basic:user:pass, bearer:TOKEN, netrc, or none |
| `--web-seed-speed-limit <RATE>` | string |  |  | Rate cap per source |
| `--web-seed-verify <MODE>` | string | `piece`, `file`, `none` | `piece` | When to hash-check HTTP-sourced data |
| `--web-seed-priority <N>` | string |  |  | Bias among sources. Higher wins when several can serve a piece |
| `--prefer-web-seed` | boolean |  | `false` | Bias the picker toward HTTP when both a peer and a source have a piece |
| `--web-seed-require` | boolean |  | `false` | Fail the run if a declared source turns out to be unusable |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli webseed test`

Probe each source: range support, size, redirects, TLS, latency

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--web-seed <URL>` | array |  |  | Source for the whole torrent, under the current composition mode |
| `--web-seed-exact <URL>` | array |  |  | Shorthand for a source with composition=exact |
| `--web-seed-for <SEL=URL>` | array |  |  | Bind a scope selector to a source, as SELECTOR=URL |
| `--web-seed-mode <MODE>` | string | `auto`, `exact`, `prefix`, `template` | `auto` | Composition mode for CLI-supplied sources |
| `--web-seed-template <TMPL>` | string |  |  | Template used when the mode is `template` |
| `--web-seed-pieces <RANGE>` | string |  |  | Restrict CLI-supplied sources to these piece indices |
| `--web-seed-bytes <RANGE>` | string |  |  | Restrict CLI-supplied sources to this byte range of the payload |
| `--web-seed-file <PATH>` | array |  |  | One URL per line. Blank lines and # comments are ignored |
| `--web-seed-list-url <URL>` | array |  |  | Fetch a newline-separated URL list over HTTP |
| `--web-seed-config <PATH>` | array |  |  | TOML or JSON binding table. Full control |
| `--web-seed-style <STYLE>` | string | `auto`, `getright`, `hoffman` | `auto` | BEP 19 or BEP 17 wire style |
| `--web-seed-only` | boolean |  | `false` | Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only |
| `--no-web-seed` | boolean |  | `false` | Ignore all web seeds, including the torrent's own url-list |
| `--no-torrent-web-seed` | boolean |  | `false` | Ignore the torrent's url-list but keep CLI-supplied sources |
| `--web-seed-concurrency <N>` | string |  |  | Concurrent ranged requests per source |
| `--max-connection-per-server <N>`, `-x` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. Per source, not per server |
| `--split <N>`, `-s` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. The same knob as `-x` |
| `--web-seed-connections <N>` | string |  |  | Peer connections each source is presented over. Default: 1 |
| `--web-seed-max-total <N>` | string |  |  | Concurrent ranged requests across all sources |
| `--web-seed-chunk-size <SIZE>` | string |  |  | Bytes per ranged request. Independent of the torrent's piece length |
| `--min-split-size <SIZE>`, `-k` | string |  |  | `aria2` spelling of a floor under `--web-seed-chunk-size` |
| `--web-seed-timeout <DUR>` | string |  |  | Per-request timeout |
| `--web-seed-connect-timeout <DUR>` | string |  |  | Connect timeout for web seed requests |
| `--web-seed-max-errors <N>` | string |  |  | Consecutive failed requests before a source is retired |
| `--web-seed-cooldown <DUR>` | string |  |  | Give a source that spent its error budget another chance after this long. Zero, the default, means it does not come back |
| `--web-seed-retries <N>` | string |  |  | Per-request retries before counting an error |
| `--web-seed-retry-status <CODES>` | string |  |  | Statuses to retry that would otherwise retire the source |
| `--web-seed-fatal-status <CODES>` | string |  |  | Statuses that retire the source, which would otherwise be retried |
| `--web-seed-user-agent <UA>` | string |  |  | User-Agent for web seed requests |
| `--web-seed-header <K: V>` | array |  |  | Extra header on web seed requests, as `Name: value` |
| `--web-seed-auth <SPEC>` | string |  |  | Credentials: basic:user:pass, bearer:TOKEN, netrc, or none |
| `--web-seed-speed-limit <RATE>` | string |  |  | Rate cap per source |
| `--web-seed-verify <MODE>` | string | `piece`, `file`, `none` | `piece` | When to hash-check HTTP-sourced data |
| `--web-seed-priority <N>` | string |  |  | Bias among sources. Higher wins when several can serve a piece |
| `--prefer-web-seed` | boolean |  | `false` | Bias the picker toward HTTP when both a peer and a source have a piece |
| `--web-seed-require` | boolean |  | `false` | Fail the run if a declared source turns out to be unusable |
| `--head` | boolean |  | `false` | Use HEAD rather than a one-byte ranged GET |
| `--concurrency <N>` | string |  | `16` | Sources probed at once |
| `--web-seed-report-header <NAME>` | array |  |  | Report this response header as well. Repeatable, case insensitive |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli webseed probe`

Measure ranged-GET latency and throughput as concurrency scales

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--web-seed <URL>` | array |  |  | Source for the whole torrent, under the current composition mode |
| `--web-seed-exact <URL>` | array |  |  | Shorthand for a source with composition=exact |
| `--web-seed-for <SEL=URL>` | array |  |  | Bind a scope selector to a source, as SELECTOR=URL |
| `--web-seed-mode <MODE>` | string | `auto`, `exact`, `prefix`, `template` | `auto` | Composition mode for CLI-supplied sources |
| `--web-seed-template <TMPL>` | string |  |  | Template used when the mode is `template` |
| `--web-seed-pieces <RANGE>` | string |  |  | Restrict CLI-supplied sources to these piece indices |
| `--web-seed-bytes <RANGE>` | string |  |  | Restrict CLI-supplied sources to this byte range of the payload |
| `--web-seed-file <PATH>` | array |  |  | One URL per line. Blank lines and # comments are ignored |
| `--web-seed-list-url <URL>` | array |  |  | Fetch a newline-separated URL list over HTTP |
| `--web-seed-config <PATH>` | array |  |  | TOML or JSON binding table. Full control |
| `--web-seed-style <STYLE>` | string | `auto`, `getright`, `hoffman` | `auto` | BEP 19 or BEP 17 wire style |
| `--web-seed-only` | boolean |  | `false` | Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only |
| `--no-web-seed` | boolean |  | `false` | Ignore all web seeds, including the torrent's own url-list |
| `--no-torrent-web-seed` | boolean |  | `false` | Ignore the torrent's url-list but keep CLI-supplied sources |
| `--web-seed-concurrency <N>` | string |  |  | Concurrent ranged requests per source |
| `--max-connection-per-server <N>`, `-x` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. Per source, not per server |
| `--split <N>`, `-s` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. The same knob as `-x` |
| `--web-seed-connections <N>` | string |  |  | Peer connections each source is presented over. Default: 1 |
| `--web-seed-max-total <N>` | string |  |  | Concurrent ranged requests across all sources |
| `--web-seed-chunk-size <SIZE>` | string |  |  | Bytes per ranged request. Independent of the torrent's piece length |
| `--min-split-size <SIZE>`, `-k` | string |  |  | `aria2` spelling of a floor under `--web-seed-chunk-size` |
| `--web-seed-timeout <DUR>` | string |  |  | Per-request timeout |
| `--web-seed-connect-timeout <DUR>` | string |  |  | Connect timeout for web seed requests |
| `--web-seed-max-errors <N>` | string |  |  | Consecutive failed requests before a source is retired |
| `--web-seed-cooldown <DUR>` | string |  |  | Give a source that spent its error budget another chance after this long. Zero, the default, means it does not come back |
| `--web-seed-retries <N>` | string |  |  | Per-request retries before counting an error |
| `--web-seed-retry-status <CODES>` | string |  |  | Statuses to retry that would otherwise retire the source |
| `--web-seed-fatal-status <CODES>` | string |  |  | Statuses that retire the source, which would otherwise be retried |
| `--web-seed-user-agent <UA>` | string |  |  | User-Agent for web seed requests |
| `--web-seed-header <K: V>` | array |  |  | Extra header on web seed requests, as `Name: value` |
| `--web-seed-auth <SPEC>` | string |  |  | Credentials: basic:user:pass, bearer:TOKEN, netrc, or none |
| `--web-seed-speed-limit <RATE>` | string |  |  | Rate cap per source |
| `--web-seed-verify <MODE>` | string | `piece`, `file`, `none` | `piece` | When to hash-check HTTP-sourced data |
| `--web-seed-priority <N>` | string |  |  | Bias among sources. Higher wins when several can serve a piece |
| `--prefer-web-seed` | boolean |  | `false` | Bias the picker toward HTTP when both a peer and a source have a piece |
| `--web-seed-require` | boolean |  | `false` | Fail the run if a declared source turns out to be unusable |
| `--duration <DUR>` | string |  | `10s` | How long to run |
| `--concurrency-sweep <SPEC>` | string |  | `1,2,4,8,16` | Step concurrency and report the curve |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli webseed fetch`

Fetch one range from one source and verify it against the torrent

Effects: `idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--web-seed <URL>` | array |  |  | Source for the whole torrent, under the current composition mode |
| `--web-seed-exact <URL>` | array |  |  | Shorthand for a source with composition=exact |
| `--web-seed-for <SEL=URL>` | array |  |  | Bind a scope selector to a source, as SELECTOR=URL |
| `--web-seed-mode <MODE>` | string | `auto`, `exact`, `prefix`, `template` | `auto` | Composition mode for CLI-supplied sources |
| `--web-seed-template <TMPL>` | string |  |  | Template used when the mode is `template` |
| `--web-seed-pieces <RANGE>` | string |  |  | Restrict CLI-supplied sources to these piece indices |
| `--web-seed-bytes <RANGE>` | string |  |  | Restrict CLI-supplied sources to this byte range of the payload |
| `--web-seed-file <PATH>` | array |  |  | One URL per line. Blank lines and # comments are ignored |
| `--web-seed-list-url <URL>` | array |  |  | Fetch a newline-separated URL list over HTTP |
| `--web-seed-config <PATH>` | array |  |  | TOML or JSON binding table. Full control |
| `--web-seed-style <STYLE>` | string | `auto`, `getright`, `hoffman` | `auto` | BEP 19 or BEP 17 wire style |
| `--web-seed-only` | boolean |  | `false` | Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only |
| `--no-web-seed` | boolean |  | `false` | Ignore all web seeds, including the torrent's own url-list |
| `--no-torrent-web-seed` | boolean |  | `false` | Ignore the torrent's url-list but keep CLI-supplied sources |
| `--web-seed-concurrency <N>` | string |  |  | Concurrent ranged requests per source |
| `--max-connection-per-server <N>`, `-x` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. Per source, not per server |
| `--split <N>`, `-s` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. The same knob as `-x` |
| `--web-seed-connections <N>` | string |  |  | Peer connections each source is presented over. Default: 1 |
| `--web-seed-max-total <N>` | string |  |  | Concurrent ranged requests across all sources |
| `--web-seed-chunk-size <SIZE>` | string |  |  | Bytes per ranged request. Independent of the torrent's piece length |
| `--min-split-size <SIZE>`, `-k` | string |  |  | `aria2` spelling of a floor under `--web-seed-chunk-size` |
| `--web-seed-timeout <DUR>` | string |  |  | Per-request timeout |
| `--web-seed-connect-timeout <DUR>` | string |  |  | Connect timeout for web seed requests |
| `--web-seed-max-errors <N>` | string |  |  | Consecutive failed requests before a source is retired |
| `--web-seed-cooldown <DUR>` | string |  |  | Give a source that spent its error budget another chance after this long. Zero, the default, means it does not come back |
| `--web-seed-retries <N>` | string |  |  | Per-request retries before counting an error |
| `--web-seed-retry-status <CODES>` | string |  |  | Statuses to retry that would otherwise retire the source |
| `--web-seed-fatal-status <CODES>` | string |  |  | Statuses that retire the source, which would otherwise be retried |
| `--web-seed-user-agent <UA>` | string |  |  | User-Agent for web seed requests |
| `--web-seed-header <K: V>` | array |  |  | Extra header on web seed requests, as `Name: value` |
| `--web-seed-auth <SPEC>` | string |  |  | Credentials: basic:user:pass, bearer:TOKEN, netrc, or none |
| `--web-seed-speed-limit <RATE>` | string |  |  | Rate cap per source |
| `--web-seed-verify <MODE>` | string | `piece`, `file`, `none` | `piece` | When to hash-check HTTP-sourced data |
| `--web-seed-priority <N>` | string |  |  | Bias among sources. Higher wins when several can serve a piece |
| `--prefer-web-seed` | boolean |  | `false` | Bias the picker toward HTTP when both a peer and a source have a piece |
| `--web-seed-require` | boolean |  | `false` | Fail the run if a declared source turns out to be unusable |
| `--url <URL>` | string |  |  | Fetch from exactly this URL |
| `--piece <N>` | string |  |  | Fetch one piece |
| `--pieces <RANGE>` | string |  |  | Fetch a piece range |
| `--file <N>` | string |  |  | Fetch a whole file by index |
| `--bytes <RANGE>` | string |  |  | Fetch a byte range |
| `--output <PATH>` | string |  |  | Write the bytes here, or `-` for stdout. Writes nothing without this |
| `--verify` | boolean |  | `true` | Verify against the torrent's piece hashes |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli verify`

Hash-check existing data against the torrent

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--data <PATH>` | string |  |  | Where the payload lives. Defaults to --dir |
| `--per-piece` | boolean |  | `false` | Report the result of every piece, not just the failures |
| `--select-file <INDEX>` | array |  |  | Verify only the files a `--select-file` download asked for. Accepts ranges: 1-5,8 |
| `--exclude-file <INDEX>` | array |  |  | Skip these files, as `--select-file`'s complement |
| `--index-out <INDEX=PATH>`, `-O` | array |  |  | Where a file was written, as INDEX=PATH, for a payload downloaded with `-O`/`--index-out` |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli create`

Create a .torrent

Effects: `non_idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `path <PATH>` | string |  |  | File or directory to build a torrent from |
| `--output <TARGET>`, `-o` | string |  |  | Write here, or `-` for stdout. Defaults to alongside the input |
| `--name <TEXT>` | string |  |  | Torrent name. Defaults to the input filename |
| `--piece-length <SIZE>` | string |  |  | Piece length. Accepts binary units. Chosen by heuristic when absent |
| `--version <V>` | string | `v1`, `v2`, `hybrid` | `v1` | Metainfo version |
| `--announce <URL>` | string |  |  | Primary tracker |
| `--announce-tier <URLS>` | array |  |  | Add a BEP 12 tier. Repeatable. Comma-separates within a tier |
| `--web-seed <URL>` | array |  |  | Web seed written into `url-list` (BEP 19) |
| `--http-seed <URL>` | array |  |  | HTTP seed written into `httpseeds` (BEP 17) |
| `--node <HOST:PORT>` | array |  |  | DHT bootstrap node written into the torrent |
| `--comment <TEXT>` | string |  |  | Free-text comment |
| `--source <TEXT>` | string |  |  | The `source` key in the info dict. Changes the info hash |
| `--update-url <URL>` | string |  |  | BEP 39 feed URL |
| `--private` | boolean |  | `false` | Set the private flag (BEP 27) |
| `--md5` | boolean |  | `false` | Write per-file MD5 checksums. MD5 is not collision resistant |
| `--glob <GLOB>` | array |  |  | Include or, with a leading `!`, exclude paths |
| `--ignore` | boolean |  | `false` | Respect .gitignore, .ignore, and .git/info/exclude |
| `--include-hidden` | boolean |  | `false` | Include hidden files |
| `--include-junk` | boolean |  | `false` | Include junk files such as .DS_Store and Thumbs.db |
| `--follow-symlinks` | boolean |  | `false` | Follow symlinks |
| `--sort-by <KEY:ORDER>` | string |  | `path:asc` | Deterministic file ordering, as KEY:ORDER |
| `--no-created-by` | boolean |  | `false` | Omit the `created by` field |
| `--no-creation-date` | boolean |  | `false` | Omit the creation date. Required for byte-reproducible output |
| `--allow <LINT>` | array |  |  | Permit a lint that would otherwise refuse the build. Repeatable |
| `--force` | boolean |  | `false` | Overwrite an existing output file |
| `--link` | boolean |  | `false` | Print the magnet URI to stdout |
| `--show` | boolean |  | `false` | Print a summary of what was created |

### `bit-cli edit`

Rewrite metainfo fields on an existing .torrent, writing a new file

Effects: `non_idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `torrent <TORRENT>` | string |  |  | The torrent to read |
| `--output <TARGET>`, `-o` | string |  |  | Write here, or `-` for stdout. Never edits in place |
| `--announce <URL>` | string |  |  | Replace the primary tracker |
| `--announce-tier <URLS>` | array |  |  | Add a BEP 12 tier. Repeatable |
| `--no-announce` | boolean |  | `false` | Drop every tracker |
| `--web-seed <URL>` | array |  |  | Add a web seed to `url-list` |
| `--replace-web-seeds` | boolean |  | `false` | Replace `url-list` rather than adding to it |
| `--no-web-seed` | boolean |  | `false` | Drop every web seed |
| `--http-seed <URL>` | array |  |  | Add an HTTP seed to `httpseeds` |
| `--comment <TEXT>` | string |  |  | Replace the comment |
| `--no-comment` | boolean |  | `false` | Drop the comment |
| `--created-by <TEXT>` | string |  |  | Replace the `created by` field |
| `--no-creation-date` | boolean |  | `false` | Drop the creation date |
| `--node <HOST:PORT>` | array |  |  | Add a DHT bootstrap node |
| `--update-url <URL>` | string |  |  | Replace the BEP 39 feed URL |
| `--allow-new-infohash` | boolean |  | `false` | Permit an edit that changes the info hash |
| `--force` | boolean |  | `false` | Overwrite an existing output file |

### `bit-cli magnet`

Convert a torrent to a magnet URI, or resolve a magnet to metadata

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--output <PATH>`, `-o` | string |  |  | Write the resolved metainfo here as a `.torrent`. `-` is stdout |
| `--force` | boolean |  | `false` | Overwrite the output file if it is already there |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--no-dht` | boolean |  | `false` | Disable the DHT while resolving |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery while resolving |
| `--no-tracker` | boolean |  | `false` | Do not announce to the magnet's trackers while resolving |
| `--page-select <TEXT>` | string |  |  | Take the one link on a page whose URL or text contains TEXT |
| `--page-client <PROFILE>` | string | `browser`, `plain` | `browser` | Which client to present as when fetching a source document |
| `--render` | boolean |  | `false` | Read the page after its script has run, through an installed browser |
| `--browser-path <PATH>` | string |  |  | The browser `--render` drives, when it is not where this looks |
| `--browser-port <HOST:PORT>` | string |  |  | Attach to a browser already listening for the DevTools protocol |

### `bit-cli seed`

Seed existing data in the foreground

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--tracker <URL>` | array |  |  | Add a tracker at runtime. The .torrent is never rewritten |
| `--tracker-file <PATH>` | array |  |  | One tracker per line. A blank line separates BEP 12 tiers |
| `--tracker-list-url <URL>` | array |  |  | Fetch a tracker list over HTTP |
| `--exclude-tracker <URL>` | array |  |  | Remove trackers. `*` removes all |
| `--replace-trackers` | boolean |  | `false` | Replace the torrent's tracker list instead of adding to it |
| `--tracker-timeout <DUR>` | string |  |  | Tracker request timeout |
| `--tracker-connect-timeout <DUR>` | string |  |  | Tracker connect timeout |
| `--tracker-interval <DUR>` | string |  |  | Override the announce interval |
| `--no-tracker` | boolean |  | `false` | Disable tracker announces entirely |
| `--max-download-rate <RATE>` | string |  |  | Download rate cap, per torrent |
| `--max-upload-rate <RATE>`, `-u` | string |  |  | Upload rate cap, per torrent |
| `--max-overall-download-rate <RATE>` | string |  |  | Download rate cap across the whole run |
| `--max-overall-upload-rate <RATE>` | string |  |  | Upload rate cap across the whole run |
| `--max-peer-rate <RATE>` | string |  |  | Download rate cap for swarm peers, not for attached HTTP sources |
| `--max-peers <N>` | string |  |  | Peer connections per torrent |
| `--max-peers-total <N>` | string |  |  | Peer connections across the run |
| `--encryption <MODE>` | string | `off`, `prefer`, `require` | `prefer` | Message stream encryption, for peer connections in both directions |
| `--transport <MODE>` | string | `tcp`, `utp`, `both` | `tcp` | Which transports this run listens on and dials |
| `--block-peer <ADDR>` | array |  |  | Refuse this peer for the whole run. Repeatable |
| `--max-open-files <N>` | string |  | `128` | Payload files kept open at once |
| `--max-handles <N>` | string |  |  | Stop when the process holds more than this many handles. Off by default |
| `--max-rss <SIZE>` | string |  |  | Stop when the process holds more than this much resident memory. Off by default |
| `--seed-ratio <RATIO>` | string |  |  | Stop seeding at this ratio. 0 means do not seed |
| `--seed-time <DUR>` | string |  |  | Stop seeding after this long |
| `--stop-timeout <DUR>` | string |  |  | Give up if there is no progress for this long |
| `--init-timeout <DUR>` | string |  | `10m` | Give up if the hash check has not finished in this long |
| `--lowest-speed-limit <RATE>` | string |  |  | Abort if the rate drops below this |
| `--on-complete <COMMAND>` | string |  |  | Run this command on success. Arguments arrive through the environment |
| `--on-error <COMMAND>` | string |  |  | Run this command on failure |
| `--data <PATH>` | string |  |  | Where the payload already lives. Defaults to --dir |
| `--index-out <INDEX=PATH>`, `-O` | array |  |  | Serve this file from this path, as INDEX=PATH |
| `--verify <MODE>` | string | `full`, `quick`, `none` | `full` | Hash-check before announcing |
| `--fastresume` | boolean |  | `false` | Reuse the previous run's hash check when the payload has not changed |
| `--fastresume-dir <DIR>` | string |  |  | Where the resume cache lives. Default: .bit-cli-resume beside the data |
| `--superseed` | boolean |  | `false` | BEP 16 superseeding for initial distribution |
| `--announce-only` | boolean |  | `false` | Announce, report the tracker response, do not serve |
| `--port <PORT>` | array |  |  | Listen port, or a range as START-END |
| `--no-dht` | boolean |  | `false` | Disable the DHT |
| `--no-pex` | boolean |  | `false` | Disable peer exchange |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery |
| `--report-interval <DUR>` | string |  | `5s` | Emit a progress event this often |
| `--exit-when-idle <DUR>` | string |  |  | Exit after this long with no connected peers |
| `--listener-check <DUR>` | string |  |  | Check this often that our own listener still answers. Off by default |

### `bit-cli bench`

Measure a target

Effects: `non_idempotent`.

Takes no options of its own.

### `bit-cli bench leech`

Download from a target and measure

Effects: `non_idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `--port <PORT>` | array |  |  | Listen port, or a range as START-END. `0` asks the OS for a free one |
| `--peer <ADDR>` | array |  |  | Try this peer before any are discovered, as HOST:PORT. Repeatable |
| `--file-allocation <METHOD>` | string | `none`, `prealloc`, `sparse`, `falloc` | `sparse` | How disk space is allocated for the payload |
| `--allow-overwrite` | boolean |  | `true` | Overwrite whatever is already in the output directory |
| `--keep-existing` | boolean |  | `false` | Keep what is already in the output directory and resume onto it |
| `--stop-on-complete` | boolean |  | `true` | Stop once the torrent completes, rather than running out `--duration`. On by default |
| `--run-full-duration` | boolean |  | `false` | Keep running until `--duration` elapses even after the payload is in |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--web-seed <URL>` | array |  |  | Source for the whole torrent, under the current composition mode |
| `--web-seed-exact <URL>` | array |  |  | Shorthand for a source with composition=exact |
| `--web-seed-for <SEL=URL>` | array |  |  | Bind a scope selector to a source, as SELECTOR=URL |
| `--web-seed-mode <MODE>` | string | `auto`, `exact`, `prefix`, `template` | `auto` | Composition mode for CLI-supplied sources |
| `--web-seed-template <TMPL>` | string |  |  | Template used when the mode is `template` |
| `--web-seed-pieces <RANGE>` | string |  |  | Restrict CLI-supplied sources to these piece indices |
| `--web-seed-bytes <RANGE>` | string |  |  | Restrict CLI-supplied sources to this byte range of the payload |
| `--web-seed-file <PATH>` | array |  |  | One URL per line. Blank lines and # comments are ignored |
| `--web-seed-list-url <URL>` | array |  |  | Fetch a newline-separated URL list over HTTP |
| `--web-seed-config <PATH>` | array |  |  | TOML or JSON binding table. Full control |
| `--web-seed-style <STYLE>` | string | `auto`, `getright`, `hoffman` | `auto` | BEP 19 or BEP 17 wire style |
| `--web-seed-only` | boolean |  | `false` | Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only |
| `--no-web-seed` | boolean |  | `false` | Ignore all web seeds, including the torrent's own url-list |
| `--no-torrent-web-seed` | boolean |  | `false` | Ignore the torrent's url-list but keep CLI-supplied sources |
| `--web-seed-concurrency <N>` | string |  |  | Concurrent ranged requests per source |
| `--max-connection-per-server <N>`, `-x` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. Per source, not per server |
| `--split <N>`, `-s` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. The same knob as `-x` |
| `--web-seed-connections <N>` | string |  |  | Peer connections each source is presented over. Default: 1 |
| `--web-seed-max-total <N>` | string |  |  | Concurrent ranged requests across all sources |
| `--web-seed-chunk-size <SIZE>` | string |  |  | Bytes per ranged request. Independent of the torrent's piece length |
| `--min-split-size <SIZE>`, `-k` | string |  |  | `aria2` spelling of a floor under `--web-seed-chunk-size` |
| `--web-seed-timeout <DUR>` | string |  |  | Per-request timeout |
| `--web-seed-connect-timeout <DUR>` | string |  |  | Connect timeout for web seed requests |
| `--web-seed-max-errors <N>` | string |  |  | Consecutive failed requests before a source is retired |
| `--web-seed-cooldown <DUR>` | string |  |  | Give a source that spent its error budget another chance after this long. Zero, the default, means it does not come back |
| `--web-seed-retries <N>` | string |  |  | Per-request retries before counting an error |
| `--web-seed-retry-status <CODES>` | string |  |  | Statuses to retry that would otherwise retire the source |
| `--web-seed-fatal-status <CODES>` | string |  |  | Statuses that retire the source, which would otherwise be retried |
| `--web-seed-user-agent <UA>` | string |  |  | User-Agent for web seed requests |
| `--web-seed-header <K: V>` | array |  |  | Extra header on web seed requests, as `Name: value` |
| `--web-seed-auth <SPEC>` | string |  |  | Credentials: basic:user:pass, bearer:TOKEN, netrc, or none |
| `--web-seed-speed-limit <RATE>` | string |  |  | Rate cap per source |
| `--web-seed-verify <MODE>` | string | `piece`, `file`, `none` | `piece` | When to hash-check HTTP-sourced data |
| `--web-seed-priority <N>` | string |  |  | Bias among sources. Higher wins when several can serve a piece |
| `--prefer-web-seed` | boolean |  | `false` | Bias the picker toward HTTP when both a peer and a source have a piece |
| `--web-seed-require` | boolean |  | `false` | Fail the run if a declared source turns out to be unusable |
| `--tracker <URL>` | array |  |  | Add a tracker at runtime. The .torrent is never rewritten |
| `--tracker-file <PATH>` | array |  |  | One tracker per line. A blank line separates BEP 12 tiers |
| `--tracker-list-url <URL>` | array |  |  | Fetch a tracker list over HTTP |
| `--exclude-tracker <URL>` | array |  |  | Remove trackers. `*` removes all |
| `--replace-trackers` | boolean |  | `false` | Replace the torrent's tracker list instead of adding to it |
| `--tracker-timeout <DUR>` | string |  |  | Tracker request timeout |
| `--tracker-connect-timeout <DUR>` | string |  |  | Tracker connect timeout |
| `--tracker-interval <DUR>` | string |  |  | Override the announce interval |
| `--no-tracker` | boolean |  | `false` | Disable tracker announces entirely |
| `--max-download-rate <RATE>` | string |  |  | Download rate cap, per torrent |
| `--max-upload-rate <RATE>`, `-u` | string |  |  | Upload rate cap, per torrent |
| `--max-overall-download-rate <RATE>` | string |  |  | Download rate cap across the whole run |
| `--max-overall-upload-rate <RATE>` | string |  |  | Upload rate cap across the whole run |
| `--max-peer-rate <RATE>` | string |  |  | Download rate cap for swarm peers, not for attached HTTP sources |
| `--max-peers <N>` | string |  |  | Peer connections per torrent |
| `--max-peers-total <N>` | string |  |  | Peer connections across the run |
| `--encryption <MODE>` | string | `off`, `prefer`, `require` | `prefer` | Message stream encryption, for peer connections in both directions |
| `--transport <MODE>` | string | `tcp`, `utp`, `both` | `tcp` | Which transports this run listens on and dials |
| `--block-peer <ADDR>` | array |  |  | Refuse this peer for the whole run. Repeatable |
| `--max-open-files <N>` | string |  | `128` | Payload files kept open at once |
| `--max-handles <N>` | string |  |  | Stop when the process holds more than this many handles. Off by default |
| `--max-rss <SIZE>` | string |  |  | Stop when the process holds more than this much resident memory. Off by default |
| `--seed-ratio <RATIO>` | string |  |  | Stop seeding at this ratio. 0 means do not seed |
| `--seed-time <DUR>` | string |  |  | Stop seeding after this long |
| `--stop-timeout <DUR>` | string |  |  | Give up if there is no progress for this long |
| `--init-timeout <DUR>` | string |  | `10m` | Give up if the hash check has not finished in this long |
| `--lowest-speed-limit <RATE>` | string |  |  | Abort if the rate drops below this |
| `--duration <DUR>` | string |  | `30s` | How long to run |
| `--warmup <DUR>` | string |  | `3s` | Discard measurements from this initial window |
| `--metrics-interval <DUR>` | string |  | `1s` | Time series resolution |
| `--target-rate <RATE>` | string |  |  | Drive toward this rate rather than running flat out |
| `--concurrency <N>` | string |  | `8` | Fixed concurrency |
| `--concurrency-sweep <SPEC>` | string |  |  | Step concurrency and report the curve |
| `--disk-budget <SIZE>` | string |  | `8GiB` | Cap generated payload on disk |
| `--request-size <SIZE>` | string |  |  | Bytes per request. Defaults to the source's own chunk size |
| `--ceiling <RATE>` | string |  |  | A rate to report the result as a share of, such as what curl reached against the same URL |
| `--report <PATH>` | string |  |  | Write the full report here, or `-` for stdout. Default: stdout |
| `--format <FMT>` | string | `json`, `ndjson`, `csv`, `text` | `json` | Report format: json, ndjson, csv, or text. `csv` carries the time series only, because a report is nested and a table is not |
| `--baseline <PATH>` | string |  |  | Compare against a previous report and print the delta |
| `--fail-under <RATE>` | string |  |  | Exit 14 if sustained throughput falls below this |

### `bit-cli bench seed`

Seed and measure what the swarm pulls

Effects: `non_idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `--data <DIR>` | string |  |  | Where the payload already lives, when that is not `--dir` |
| `--port <PORT>` | array |  |  | Listen port, or a range as START-END. `0` asks the OS for a free one |
| `--no-dht` | boolean |  | `false` | Disable the DHT |
| `--no-lsd` | boolean |  | `false` | Disable local service discovery |
| `--exit-when-idle <DUR>` | string |  |  | Stop once no peer has been connected for this long |
| `--include-hash-check` | boolean |  | `false` | Measure the payload's hash check on add as well |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--tracker <URL>` | array |  |  | Add a tracker at runtime. The .torrent is never rewritten |
| `--tracker-file <PATH>` | array |  |  | One tracker per line. A blank line separates BEP 12 tiers |
| `--tracker-list-url <URL>` | array |  |  | Fetch a tracker list over HTTP |
| `--exclude-tracker <URL>` | array |  |  | Remove trackers. `*` removes all |
| `--replace-trackers` | boolean |  | `false` | Replace the torrent's tracker list instead of adding to it |
| `--tracker-timeout <DUR>` | string |  |  | Tracker request timeout |
| `--tracker-connect-timeout <DUR>` | string |  |  | Tracker connect timeout |
| `--tracker-interval <DUR>` | string |  |  | Override the announce interval |
| `--no-tracker` | boolean |  | `false` | Disable tracker announces entirely |
| `--max-download-rate <RATE>` | string |  |  | Download rate cap, per torrent |
| `--max-upload-rate <RATE>`, `-u` | string |  |  | Upload rate cap, per torrent |
| `--max-overall-download-rate <RATE>` | string |  |  | Download rate cap across the whole run |
| `--max-overall-upload-rate <RATE>` | string |  |  | Upload rate cap across the whole run |
| `--max-peer-rate <RATE>` | string |  |  | Download rate cap for swarm peers, not for attached HTTP sources |
| `--max-peers <N>` | string |  |  | Peer connections per torrent |
| `--max-peers-total <N>` | string |  |  | Peer connections across the run |
| `--encryption <MODE>` | string | `off`, `prefer`, `require` | `prefer` | Message stream encryption, for peer connections in both directions |
| `--transport <MODE>` | string | `tcp`, `utp`, `both` | `tcp` | Which transports this run listens on and dials |
| `--block-peer <ADDR>` | array |  |  | Refuse this peer for the whole run. Repeatable |
| `--max-open-files <N>` | string |  | `128` | Payload files kept open at once |
| `--max-handles <N>` | string |  |  | Stop when the process holds more than this many handles. Off by default |
| `--max-rss <SIZE>` | string |  |  | Stop when the process holds more than this much resident memory. Off by default |
| `--seed-ratio <RATIO>` | string |  |  | Stop seeding at this ratio. 0 means do not seed |
| `--seed-time <DUR>` | string |  |  | Stop seeding after this long |
| `--stop-timeout <DUR>` | string |  |  | Give up if there is no progress for this long |
| `--init-timeout <DUR>` | string |  | `10m` | Give up if the hash check has not finished in this long |
| `--lowest-speed-limit <RATE>` | string |  |  | Abort if the rate drops below this |
| `--duration <DUR>` | string |  | `30s` | How long to run |
| `--warmup <DUR>` | string |  | `3s` | Discard measurements from this initial window |
| `--metrics-interval <DUR>` | string |  | `1s` | Time series resolution |
| `--target-rate <RATE>` | string |  |  | Drive toward this rate rather than running flat out |
| `--concurrency <N>` | string |  | `8` | Fixed concurrency |
| `--concurrency-sweep <SPEC>` | string |  |  | Step concurrency and report the curve |
| `--disk-budget <SIZE>` | string |  | `8GiB` | Cap generated payload on disk |
| `--request-size <SIZE>` | string |  |  | Bytes per request. Defaults to the source's own chunk size |
| `--ceiling <RATE>` | string |  |  | A rate to report the result as a share of, such as what curl reached against the same URL |
| `--report <PATH>` | string |  |  | Write the full report here, or `-` for stdout. Default: stdout |
| `--format <FMT>` | string | `json`, `ndjson`, `csv`, `text` | `json` | Report format: json, ndjson, csv, or text. `csv` carries the time series only, because a report is nested and a table is not |
| `--baseline <PATH>` | string |  |  | Compare against a previous report and print the delta |
| `--fail-under <RATE>` | string |  |  | Exit 14 if sustained throughput falls below this |

### `bit-cli bench webseed`

Measure HTTP sources: latency percentiles, concurrency scaling, ranges

Effects: `non_idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `source <SOURCE>` | string |  |  | A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a metalink, or `-` for stdin |
| `--web-seed <URL>` | array |  |  | Source for the whole torrent, under the current composition mode |
| `--web-seed-exact <URL>` | array |  |  | Shorthand for a source with composition=exact |
| `--web-seed-for <SEL=URL>` | array |  |  | Bind a scope selector to a source, as SELECTOR=URL |
| `--web-seed-mode <MODE>` | string | `auto`, `exact`, `prefix`, `template` | `auto` | Composition mode for CLI-supplied sources |
| `--web-seed-template <TMPL>` | string |  |  | Template used when the mode is `template` |
| `--web-seed-pieces <RANGE>` | string |  |  | Restrict CLI-supplied sources to these piece indices |
| `--web-seed-bytes <RANGE>` | string |  |  | Restrict CLI-supplied sources to this byte range of the payload |
| `--web-seed-file <PATH>` | array |  |  | One URL per line. Blank lines and # comments are ignored |
| `--web-seed-list-url <URL>` | array |  |  | Fetch a newline-separated URL list over HTTP |
| `--web-seed-config <PATH>` | array |  |  | TOML or JSON binding table. Full control |
| `--web-seed-style <STYLE>` | string | `auto`, `getright`, `hoffman` | `auto` | BEP 19 or BEP 17 wire style |
| `--web-seed-only` | boolean |  | `false` | Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only |
| `--no-web-seed` | boolean |  | `false` | Ignore all web seeds, including the torrent's own url-list |
| `--no-torrent-web-seed` | boolean |  | `false` | Ignore the torrent's url-list but keep CLI-supplied sources |
| `--web-seed-concurrency <N>` | string |  |  | Concurrent ranged requests per source |
| `--max-connection-per-server <N>`, `-x` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. Per source, not per server |
| `--split <N>`, `-s` | string |  |  | `aria2` spelling of `--web-seed-concurrency`. The same knob as `-x` |
| `--web-seed-connections <N>` | string |  |  | Peer connections each source is presented over. Default: 1 |
| `--web-seed-max-total <N>` | string |  |  | Concurrent ranged requests across all sources |
| `--web-seed-chunk-size <SIZE>` | string |  |  | Bytes per ranged request. Independent of the torrent's piece length |
| `--min-split-size <SIZE>`, `-k` | string |  |  | `aria2` spelling of a floor under `--web-seed-chunk-size` |
| `--web-seed-timeout <DUR>` | string |  |  | Per-request timeout |
| `--web-seed-connect-timeout <DUR>` | string |  |  | Connect timeout for web seed requests |
| `--web-seed-max-errors <N>` | string |  |  | Consecutive failed requests before a source is retired |
| `--web-seed-cooldown <DUR>` | string |  |  | Give a source that spent its error budget another chance after this long. Zero, the default, means it does not come back |
| `--web-seed-retries <N>` | string |  |  | Per-request retries before counting an error |
| `--web-seed-retry-status <CODES>` | string |  |  | Statuses to retry that would otherwise retire the source |
| `--web-seed-fatal-status <CODES>` | string |  |  | Statuses that retire the source, which would otherwise be retried |
| `--web-seed-user-agent <UA>` | string |  |  | User-Agent for web seed requests |
| `--web-seed-header <K: V>` | array |  |  | Extra header on web seed requests, as `Name: value` |
| `--web-seed-auth <SPEC>` | string |  |  | Credentials: basic:user:pass, bearer:TOKEN, netrc, or none |
| `--web-seed-speed-limit <RATE>` | string |  |  | Rate cap per source |
| `--web-seed-verify <MODE>` | string | `piece`, `file`, `none` | `piece` | When to hash-check HTTP-sourced data |
| `--web-seed-priority <N>` | string |  |  | Bias among sources. Higher wins when several can serve a piece |
| `--prefer-web-seed` | boolean |  | `false` | Bias the picker toward HTTP when both a peer and a source have a piece |
| `--web-seed-require` | boolean |  | `false` | Fail the run if a declared source turns out to be unusable |
| `--duration <DUR>` | string |  | `30s` | How long to run |
| `--warmup <DUR>` | string |  | `3s` | Discard measurements from this initial window |
| `--metrics-interval <DUR>` | string |  | `1s` | Time series resolution |
| `--target-rate <RATE>` | string |  |  | Drive toward this rate rather than running flat out |
| `--concurrency <N>` | string |  | `8` | Fixed concurrency |
| `--concurrency-sweep <SPEC>` | string |  |  | Step concurrency and report the curve |
| `--disk-budget <SIZE>` | string |  | `8GiB` | Cap generated payload on disk |
| `--request-size <SIZE>` | string |  |  | Bytes per request. Defaults to the source's own chunk size |
| `--ceiling <RATE>` | string |  |  | A rate to report the result as a share of, such as what curl reached against the same URL |
| `--report <PATH>` | string |  |  | Write the full report here, or `-` for stdout. Default: stdout |
| `--format <FMT>` | string | `json`, `ndjson`, `csv`, `text` | `json` | Report format: json, ndjson, csv, or text. `csv` carries the time series only, because a report is nested and a table is not |
| `--baseline <PATH>` | string |  |  | Compare against a previous report and print the delta |
| `--fail-under <RATE>` | string |  |  | Exit 14 if sustained throughput falls below this |

### `bit-cli bench disk`

Measure the payload file under several writers, with no session

Effects: `non_idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `--dir <DIR>` | string |  |  | Where the payload is written. Defaults to a directory this run makes under the system temporary directory and removes afterwards |
| `--payload-size <SIZE>` | string |  | `1GiB` | Total bytes written per step |
| `--block-size <SIZE>` | string |  | `16KiB` | Bytes per positioned write. The peer protocol's block is 16 KiB |
| `--concurrency <N>` | string |  | `8` | How many threads write at once |
| `--concurrency-sweep <SPEC>` | string |  |  | Step the thread count and report the curve, for example `1,2,4,8` |
| `--layout <LAYOUT>` | string | `shared`, `split`, `handles` | `shared` | How the payload is spread over files. `shared` is one file with every thread interleaving into it, which is where writes contend. `split` gives each thread its own file, which is the control |
| `--run-length <N>` | string |  | `1` | Consecutive blocks one thread writes before the next takes over, under `shared` and `handles`. 1 strides block by block, which contends most. A receive path writes a whole fetched range at a time, so `64` at the default block size is the shape a download has |
| `--file-allocation <METHOD>` | string | `none`, `prealloc`, `sparse`, `falloc` | `sparse` | How disk space is allocated for the payload |
| `--max-open-files <N>` | string |  | `0` | How many payload files stay open at once. 0 uses the storage default |
| `--metrics-interval <DUR>` | string |  | `1s` | Time series resolution |
| `--duration <DUR>` | string |  | `300s` | Stop a step once this much wall time has passed |
| `--no-verify` | boolean |  | `false` | Skip the read-back that checks every block landed where it was sent |
| `--report <PATH>` | string |  |  | Write the full report here, or `-` for stdout. Default: stdout |
| `--format <FMT>` | string | `json`, `ndjson`, `csv`, `text` | `json` | Report format: json, ndjson, csv, or text. `csv` carries the time series only, because a report is nested and a table is not |
| `--baseline <PATH>` | string |  |  | Compare against a previous report and print the delta |
| `--fail-under <RATE>` | string |  |  | Exit 14 if sustained throughput falls below this |

### `bit-cli bench swarm`

Synthetic peer load against a target

Effects: `non_idempotent`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `target <TARGET>` | string |  |  | `HOST:PORT` of the peer to load. The only address this ever connects to: it announces to no tracker, uses no DHT, and reads no peer list |
| `--for <TORRENT>` | array |  |  | A torrent the target already serves, as a `.torrent` path. Repeatable |
| `--peers <N>` | string |  | `8` | Synthetic peer count |
| `--torrents <N>` | string |  | `1` | How many torrents to generate. Ignored when `--for` is given |
| `--payload-size <SIZE>` | string |  | `256MiB` | The length a generated torrent declares. No payload is written for it: the target does not have the torrent, so nothing will ever be fetched or checked against it |
| `--piece-size <SIZE>` | string |  | `1MiB` | The piece length a generated torrent declares |
| `--dir <DIR>` | string |  |  | Where verified pieces and generated torrents are written. A directory this run makes and removes when not given |
| `--connect-timeout <DUR>` | string |  | `10s` | How long one connect attempt gets before the peer gives up on it |
| `--keep` | boolean |  | `false` | Keep the scratch directory instead of removing it |
| `--duration <DUR>` | string |  | `30s` | How long to run |
| `--warmup <DUR>` | string |  | `3s` | Discard measurements from this initial window |
| `--metrics-interval <DUR>` | string |  | `1s` | Time series resolution |
| `--target-rate <RATE>` | string |  |  | Drive toward this rate rather than running flat out |
| `--concurrency <N>` | string |  | `8` | Fixed concurrency |
| `--concurrency-sweep <SPEC>` | string |  |  | Step concurrency and report the curve |
| `--disk-budget <SIZE>` | string |  | `8GiB` | Cap generated payload on disk |
| `--request-size <SIZE>` | string |  |  | Bytes per request. Defaults to the source's own chunk size |
| `--ceiling <RATE>` | string |  |  | A rate to report the result as a share of, such as what curl reached against the same URL |
| `--report <PATH>` | string |  |  | Write the full report here, or `-` for stdout. Default: stdout |
| `--format <FMT>` | string | `json`, `ndjson`, `csv`, `text` | `json` | Report format: json, ndjson, csv, or text. `csv` carries the time series only, because a report is nested and a table is not |
| `--baseline <PATH>` | string |  |  | Compare against a previous report and print the delta |
| `--fail-under <RATE>` | string |  |  | Exit 14 if sustained throughput falls below this |

### `bit-cli bench probe`

One-shot capability and reachability probe

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `target <TARGET>` | string |  |  | `HOST:PORT` for a peer, or an `http(s)://` URL for a mirror |
| `--for <SOURCE>` | string |  |  | The torrent to ask a peer about, as a `.torrent`, a magnet, or an info hash |
| `--timeout <DUR>` | string |  | `10s` | How long to wait for each step, and how long to listen after the handshake |
| `--report <PATH>` | string |  |  | Write the full report here, or `-` for stdout. Default: stdout |
| `--format <FMT>` | string | `json`, `ndjson`, `csv`, `text` | `json` | Report format: json, ndjson, csv, or text. `csv` carries the time series only, because a report is nested and a table is not |
| `--baseline <PATH>` | string |  |  | Compare against a previous report and print the delta |
| `--fail-under <RATE>` | string |  |  | Exit 14 if sustained throughput falls below this |

### `bit-cli config`

Configuration

Effects: `read_only`.

Takes no options of its own.

### `bit-cli config show`

Print the fully resolved configuration with the origin of every value

Effects: `read_only`.

Takes no options of its own.

### `bit-cli completions`

Generate shell completions

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `shell <SHELL>` | string | `bash`, `zsh`, `fish`, `powershell`, `elvish`, `nushell` |  | Which shell to generate for |

### `bit-cli man`

Generate a man page

Effects: `read_only`.

| option | type | accepts | default | what it does |
| --- | --- | --- | --- | --- |
| `--output <PATH>`, `-o` | string |  |  | Write the man page here instead of to stdout |
| `--format <FMT>` | string | `roff`, `json`, `markdown` | `roff` | What to render. `roff` is the man page; `json` is the same surface as a CLIspec document, for a reader that cannot parse roff |

### `bit-cli version`

Version, build metadata, enabled features, and protocol support

Effects: `read_only`.

Takes no options of its own.

## Exit codes

The exit code is the primary success signal, and no code is ever reused for a second meaning. `retryable` says whether a second attempt could succeed without changing anything.

| code | kind | retryable | meaning |
| --- | --- | --- | --- |
| 0 | `success` | | The command did what it was asked to do |
| 1 | `generic` | yes | Generic failure |
| 2 | `usage` | no | Usage or argument error |
| 3 | `config` | no | Configuration error |
| 4 | `source_resolution` | no | Source resolution failed |
| 5 | `network` | yes | Network failure |
| 6 | `no_usable_sources` | yes | No usable sources |
| 7 | `hash_mismatch` | no | Hash verification failed |
| 8 | `disk` | no | Disk error |
| 9 | `timeout` | yes | Timeout or deadline exceeded |
| 10 | `interrupted` | no | Interrupted by the user, partial state saved |
| 11 | `coverage_gap` | no | Coverage gap: some pieces have no source |
| 12 | `binding` | no | Binding error: a scope selector or composition mode is invalid |
| 13 | `lint_refused` | no | Lint refused a torrent at creation |
| 14 | `threshold_not_met` | no | Threshold not met |
| 15 | `would_change_infohash` | no | Would change the info hash |
| 16 | `resource_ceiling` | yes | A resource ceiling was crossed |
| 17 | `listener_unhealthy` | yes | This run's own listener stopped answering |
