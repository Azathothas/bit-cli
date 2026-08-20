# Changelog

Notable changes, newest first. Versions follow [semantic
versioning](https://semver.org/spec/v2.0.0.html), and the released version is
driven from the git tag.

## 0.1.0, unreleased

First release. `bit-cli` started as a fork of
[`kist`](https://github.com/QaidVoid/kist) and shares no released version with
it, so the history starts here.

### The reason the project exists

- Web seeds attach to an existing torrent at runtime. The `.torrent` is never
  rewritten, never re-hashed, and the info hash does not change.
- A web seed binding is a `(source, scope, composition)` triple, and the three
  are orthogonal. A mirror holding part of a payload is a first-class case
  rather than an error.
- Four composition modes: `auto` (BEP 19), `exact`, `prefix`, and `template`
  with eleven placeholders.
- Scope selectors by file index, index range, path, glob, negated glob, piece
  range, byte range, and byte range within a file.
- Binding tables in TOML or JSON with the same schema, for the cases a command
  line cannot express cleanly.
- Coverage is computed before any request goes out. A gap names the uncovered
  piece indices and `--web-seed-require` turns it into exit 11.
- BEP 19 (GetRight) and BEP 17 (Hoffman) wire styles.
- HTTP sources are presented to the session as peers over loopback. The
  announced bitfield carries only the pieces the source's scope covers in
  full, and a partial source advertises BEP 21 `upload_only`.
- Fetched pieces are hash-checked at the source, so a mirror serving wrong data
  is named rather than showing up as "a peer sent something wrong".
- `--web-seed-connections <N>` presents one source over N connections, which is
  N peers to the session and so N receive paths. They share one HTTP client,
  one window cache, and one concurrency budget divided between them, so the
  mirror sees the same number of requests. On loopback two connections reach
  1.92 times one, measured in `TODO/webseed.md` under T-009.
- `--prefer-web-seed` doubles each source's connections rather than its request
  budget. On a loopback swarm of one mirror and one peer it moves the HTTP
  share of a 1 GiB payload from 46.72% to 62.60% across five paired runs. It
  moves the odds and not the decision: `librqbit`'s piece picker is not
  reachable from outside the crate. See `TODO/webseed.md` under T-003.
- `--web-seed-speed-limit` and a binding table's `rate_limit` are enforced. They
  parsed and were never applied, so a source told to stay under 24 MiB/s ran at
  116. A token bucket per source now paces requests before they go out.
- `--max-download-rate` and `--max-upload-rate` are measured and hold. A 4MiB/s
  cap sustains 4.10 MiB/s against 223.39 MiB/s uncapped, and the seeder side
  caps a downloader that asked for no cap at 4.01 MiB/s.
  `pwsh scripts/check-rate-limit.ps1` is the measurement. See
  `TODO/performance.md` under T-031.
- A download recovers from every peer going away and coming back, and how long
  it takes is now written down. A dropped peer is retried at about 10 seconds,
  then 70, then 430, so an outage ending between two attempts waits for the
  next one. `--stop-timeout` set shorter than that turns a recoverable outage
  into exit 9. `pwsh scripts/check-peer-recovery.ps1` drives both. See
  `TODO/peers.md` under T-021.
- A seeder no longer goes deaf under a burst of connections that close before
  they handshake. `librqbit`'s accept loop is a `tokio::select!` whose two
  branches can both be disabled at once, and when they are it panics, killing
  the listener while the process carries on reporting itself as seeding.
  Measured, 3000 such connections did it in 79 seconds and 2411 of them then
  failed to connect at all. `bit-cli` removes the branch that carries it, and
  the same flood finishes in 8.8 seconds with the listener alive. See
  `TODO/peers.md` under T-020.
- `--max-handles <N>` stops a run that holds more than that many handles, with
  `"stopped": "handle_ceiling"` and exit 16. Off by default. It is a backstop
  for the socket that a connection closing before its handshake strands about
  half the time, which is upstream and open: `pwsh scripts/check-close-wait.ps1`
  measures it, and `--max-handles` turns an unbounded stranding inside a
  `seed --seed-time 7d` into a loud exit a supervisor restarts.
- Exit code 16, `resource_ceiling`, for a resource ceiling the caller set.
- A source URL may be `file:`, so bytes already on the disk under another name
  are a source with a scope, a composition, a chunk size, a rate limit, and the
  same per-piece verification. `webseed list`, `webseed test`, `webseed probe`,
  and `bench webseed` all take one. It is never offered to a swarm: `file:` is
  in neither BEP 17 nor BEP 19 and exists so the same 64 MiB is not fetched
  three times. `pwsh scripts/check-local-source.ps1` drives six cases with no
  server and no bound port, including one payload landing under three info
  hashes and three piece lengths with one distinct hash between them. A `..` in
  a resolved path is refused, because `auto` and `prefix` composition append
  the torrent's own name and path and a hostile `.torrent` would otherwise
  choose the tail of it. See `TODO/multi-source.md` under T-133.
- `--web-seed-retry-status` and `--web-seed-fatal-status` decide which HTTP
  statuses retire a source, per source, as codes and inclusive ranges. A CDN
  that signs its URLs answers 403 when a signature expires and the next request
  to the stable URL succeeds, so `--web-seed-retry-status 403` is what makes
  that survivable: in the recorded run, 22 signatures expired over 64 MiB and
  the payload completed byte for byte, where the same run without the flag
  downloaded nothing. The binding table takes `retry_status` and `fatal_status`
  per source and in `[default]`. A code in both lists is a usage error. See
  `TODO/multi-source.md` under T-130 and `scripts/check-signed-source.ps1`.
- A source is no longer retired by one request that ran out of retries. A
  transient failure reconnects the bridge instead, bounded by
  `--web-seed-max-errors` consecutive failed requests. Before this, a mirror
  that answered 503 for four seconds and then recovered was lost for the rest
  of the run with no flag set, and `--web-seed-max-errors` could never be
  reached.
- `download` reports `retries` and `retries_by_status` per source, in the text
  output and in `--json`.
- `--peer <ADDR>` dials a peer whether or not a tracker or the DHT answers, and
  `download` takes `--no-dht` and `--no-lsd` as `seed` already did. Together
  they make a swarm of exactly the members named on the command line, which is
  what a measurement needs and what a private network wants.

### Commands

- `download`, `seed`, `peers`, `trackers`, `verify`, `info`, `files`, `magnet`,
  `create`, `edit`, `config show`, `completions`, `man`, `version`.
- `webseed list` resolves every binding and prints the exact URL each file maps
  to, without touching the network.
- `webseed test` probes each source for range support, entity length against
  the torrent, the redirect chain hop by hop, and the negotiated TLS version
  and cipher.
- `webseed probe` measures ranged-GET latency percentiles and throughput as
  concurrency rises.
- `webseed fetch` pulls one named range from one named source and verifies it.
- `trackers` announces and scrapes over HTTP and UDP directly, reporting each
  tracker's tier, interval, seeder and leecher counts, and failure reason.
- `bench webseed` measures HTTP sources: latency percentiles for connection
  establishment, first byte, and completion; a concurrency curve; per-source
  attribution; and error counts by class and by HTTP status.
- `bench leech` measures a download and splits its cost three ways: the block
  request pipeline, piece verification, and the disk. All three are measured
  rather than modelled, and all three appear per interval as well as in the
  summary.
- `bench disk` measures the disk on its own: a payload written through the same
  storage a download writes through, from N threads, with no session and no
  network. `--layout shared|handles|split` decides whether the threads share
  one file behind one handle, share one file behind a handle each, or take a
  file each, and comparing the three is what says where a limit lives. Every
  step reads its payload back and checks each block is the one written to it,
  and exits 7 rather than reporting a rate when it is not.

### Measurement

- Every `bench` report carries the machine it was taken on. `bit-cli` version,
  target triple, build profile, and whether debug assertions were on. OS and
  kernel version, CPU model, logical core count, total memory, and NIC link
  speed. The exact command line and working directory. Start and end timestamps
  in ISO 8601 UTC with millisecond precision. Peak RSS, user and system CPU
  time, and open handle count, sampled on the metrics interval as well as at
  the end. All of it read through the platform's own interfaces, with no new
  dependency.
- Latency percentiles come from a histogram rather than a sorted vector, so a
  six hour run costs the same memory as a six second one.
- The warmup window is reported rather than dropped. A sample taken during
  warmup is marked and excluded from the summary, because "it was slow for the
  first three seconds" is itself a result.
- Connection establishment is measured on its own cadence, one connection per
  source per metrics interval, because a pooled HTTP client cannot report what
  opening a connection costs.
- Four report formats: `json`, `ndjson`, `csv`, and `text`. The report goes to
  stdout unless `--report <PATH>` names a file. `csv` carries the time series
  only, which is said in the docs rather than left to be discovered.
- `--fail-under <RATE>` exits 14 when sustained throughput falls below the
  rate. `--baseline <PATH>` prints a delta per metric with a sign, a
  percentage, and which direction is an improvement, and refuses a comparison
  across different hardware, a different subcommand, or a newer report version,
  naming the reason.
- `--target-rate` paces the run against its own totals rather than per worker,
  so the target is the aggregate.
- Storage counts its positioned reads and writes, their bytes, and their time,
  and brackets every piece check: a check is a run of reads walking the piece
  from its start followed by the session declaring it complete, all on one
  thread, so the wall time between them is the whole cost of the check with the
  SHA-1 included. Two `Instant::now()` calls per operation, always on, because
  a counter that is only on when someone is measuring measures a different
  program.
- The loopback bridge counts the blocks the session has asked for and not yet
  been given, the deepest that ever got, and the time to answer each one. That
  is the session's own request window seen from the other end, and it is what
  says whether the window is what bounds a run.
- `bench leech` refuses to run against an output directory that already holds
  the complete payload. That run finishes without fetching anything and would
  report the hash checker's rate as a download rate.
- Every source reports the bytes it pulled over HTTP beside the bytes that
  reached the session. The two differing is the amplification: separate
  sources at one URL each keep their own window cache and fetch the same
  window once each, which measured 3.98x against 1.004x for the same number of
  connections on one source.
- A share of a stated ceiling is no longer clamped at a hundred percent. It is
  a comparison rather than a progress, and `--ceiling` names a reference the
  caller supplies, so a run that beat it now says so. The clamping renderer is
  still what progress uses.
- Each `bench disk` step drains its writeback after the clock stops and reports
  it as `flush`. Without that, a step that filled the page cache hands its cost
  to whichever step runs after it, and a sweep reports the order the steps ran
  in rather than the thread count.

### Paths

- Every torrent path is planned before anything is opened. A component the
  platform reads as a drive or a root cannot leave the output directory, a name
  the filesystem refuses is rewritten rather than failing the download, and two
  names that collide only on a case-insensitive filesystem both land under
  distinct names.
- The rules run on every platform, not only Windows, so a payload downloaded on
  Linux and copied to a Windows machine still works.
- Every change is reported on stderr and in `--json` as a `renamed` array
  carrying the file index, both paths, and the reason. The key is absent when
  nothing changed.
- `bit-cli` supplies its own storage to do this. Reads and writes are addressed
  by file index and offset rather than by a cursor, so many pieces can be in
  flight against one file.
- Whether a torrent unpacks into a directory of its own follows BEP 3: `name`
  is the file's name when the metainfo carries no `files` list and the
  directory's name when it does, however many entries that list holds. Deciding
  it by counting files instead dropped the directory for a torrent whose
  `files` list held one entry, so two such torrents in one invocation wrote the
  same path and both reported success. `aria2c` 1.37.0 creates the directory
  for the same torrent. See `TODO/performance.md` under T-036.

### Disk

- A payload file is created when it is first written, not when the torrent is
  added, and a read of a file that is not there does not bring one into
  existence. `--select-file 0` therefore writes one file and leaves the rest
  off the disk instead of creating them empty beside it.
- `--max-open-files` caps how many payload files stay open, closing the least
  recently opened when it is reached. The default is 128. A torrent with twenty
  thousand files needs the cap in descriptors and not twenty thousand.
  `scripts/check-handles.ps1` measures it: the steps in the cap and the steps
  in the process handle count match exactly.
- `--file-allocation` does four different things rather than four names for
  one. `none` sets the length, `sparse` marks the file sparse first, `prealloc`
  writes and flushes zeroes, and `falloc` asks the filesystem to reserve the
  blocks. `falloc` on Windows needs a privilege an ordinary process does not
  hold, so it falls back to `prealloc` and says so on stderr.
  `scripts/check-allocation.ps1` measures all four against a real download by
  reading volume free space before the payload arrives.
- Concurrent positioned writes to one file are safe against each other, which
  is why the handle lock is taken by the read half. A test drives eight threads
  at one file and checks every block for the byte its writer owned.
- They are safe but they do not scale, and `bench disk` says why: on NTFS
  writes to one file serialise whatever handle they arrive on, so more handles
  buy nothing and only spreading the work over more files helps. The
  serialisation is charged per operation rather than per byte, so the same
  gigabyte in 1 MiB writes reaches 2.30 times what it reaches in 16 KiB writes
  at eight writers. See `TODO/disk-io.md` under T-017.

### Concurrency

- `download` notices a torrent finishing when it finishes, rather than on the
  next `--report-interval` tick. The watch loop woke only on the tick and
  checked completion afterwards, so a run that finished 1.1 seconds in ended at
  2.0, and `-j 1` with four torrents paid that four times. `--timeout` and
  `--stop-after` had the same lag and now wake the loop themselves. Measured
  against the same runs with only the tick: 1.42x for one torrent alone, 1.31x
  at `-j 1`, 1.36x at `-j 2`, 1.18x at `-j 4`.
- `-j` scales. Four torrents of 256 MiB at `-j 4` finish 3.54 times faster than
  running them one invocation at a time, and 3.50 times faster than `-j 1` in
  the same process, at 71.73% of what the HTTP source serves with no torrent
  machinery at all. Putting the same total connection count on one torrent at a
  time reaches 0.59 times that, so the flag buys concurrency rather than
  connections. `scripts/check-multi-torrent.ps1` is the measurement and it
  writes `bench/multi-torrent-<timestamp>.json`.
- Concurrency costs about 22 MiB of peak RSS and twelve handles per torrent in
  flight. CPU is flat for the same bytes.

### Contract

- 16 exit codes. Codes 11 through 15 exist so a script can tell "your mirrors
  are misconfigured" from "the network is down" from "your server is slow".
- stdout carries data only; stderr carries logs, progress, warnings, and
  errors.
- Every JSON document carries `schema_version`, `generated_at`, and
  `bit_cli_version`.
- `--jsonl` emits one event per line with a monotonic `seq` and an ISO 8601 UTC
  millisecond timestamp.
- Nothing is TTY-gated. Terminal detection decides colour and progress
  rendering and nothing else.
- Six-layer configuration precedence, and `config show` reports the origin of
  every value.
- A failed add carries the code that says why. `download --json` reports a
  `code` per torrent and the run exits with the worst of them, so an existing
  file exits 8 rather than a generic 1.
- `seed` and `verify` carry the same `renamed` array `download` does, because
  they serve and read the files it wrote. `verify` also reads the planned paths
  rather than the torrent's own, which it did not before.
- `--port` reaches `download` and `peers`, not only `seed`. `--no-continue`
  turns off `--continue`, which previously defaulted on with no way to clear
  it. `--init-timeout` bounds the hash check and names the phase when it fires.

### Build

- Windows release binaries link the C runtime statically, so they run with no
  Visual C++ redistributable. `scripts/check-static.ps1` verifies it and CI
  runs it. The `x86_64-pc-windows-msvc` binary imports `kernel32`, `ntdll`,
  `combase`, `bcryptprimitives`, `api-ms-win-core-synch-l1-2-0`, `ws2_32`,
  `shell32`, `crypt32`, `bcrypt`, `userenv`, `advapi32`, and `iphlpapi`. The
  one api-set is a core OS set, not a CRT redirect. The script prints the size
  of whichever binary it checked rather than the number being pinned here,
  because a size moves with every commit and a pinned one goes stale.
- `create` output is byte-identical on repeat runs and independent of input
  order, with paths `/`-separated and sorted by raw bytes so no locale can
  affect it.
- `create`, `verify`, and `seed` round trip through two other implementations.
  `scripts/interop-roundtrip.ps1` seeds a four-file 490,012 byte payload on
  loopback and downloads it with `aria2c` 1.37.0 and with `rqbit` 9.0.0, byte
  for byte, in three cases: plain v1, `--private`, and `--web-seed` with no
  peer at all. `rqbit` skips the third, which it cannot do: it has no BEP 19.
  CI runs the `aria2c` matrix on Linux and Windows. The record is in
  `TODO/create-seed.md` under T-084.
- `edit` splices the original `info` bytes back verbatim and re-hashes before
  returning, so it cannot change the info hash even for a torrent whose
  original encoding was not canonical.

### Not in this release

`bench seed`, `bench probe`, `bench swarm`, Metalink resolution, BEP 52 v2 and
hybrid creation, BEP 16 superseeding, `--log-file` rotation, and
`-i/--input-file`. Each has an entry in `TODO/` with what closes it. Nothing is
stubbed: a command that is not implemented says so and exits with a code a
script can branch on.
