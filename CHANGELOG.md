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

### Build

- Windows release binaries link the C runtime statically, so they run with no
  Visual C++ redistributable. `scripts/check-static.ps1` verifies it and CI
  runs it. The `x86_64-pc-windows-msvc` binary is 16,082,432 bytes and imports
  `kernel32`, `ntdll`, `combase`, `bcryptprimitives`,
  `api-ms-win-core-synch-l1-2-0`, `ws2_32`, `shell32`, `crypt32`, `bcrypt`,
  `userenv`, `ADVAPI32`, and `iphlpapi`. The one api-set is a core OS set, not
  a CRT redirect.
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

`bench leech`, `bench seed`, `bench probe`, `bench swarm`, Metalink resolution,
BEP 52 v2 and hybrid creation, BEP 16 superseeding, `--log-file` rotation, and
`-i/--input-file`. Each has an entry in `TODO/` with what closes it. Nothing is
stubbed: a command that is not implemented says so and exits with a code a
script can branch on.
