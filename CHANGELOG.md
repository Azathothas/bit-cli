# Changelog

Notable changes, newest first. Versions follow [semantic
versioning](https://semver.org/spec/v2.0.0.html), and the released version is
driven from the git tag.

## 0.1.0 — unreleased

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
- `create`, `verify`, and `seed` round trip through a second implementation.
  `scripts/interop-roundtrip.ps1` seeds a four-file 490,012 byte payload on
  loopback and downloads it with `aria2c` 1.37.0, byte for byte, in three
  cases: plain v1, `--private`, and `--web-seed` with no peer at all. CI runs
  it on Linux and Windows. The record is in `TODO/create-seed.md` under T-084.
- `edit` splices the original `info` bytes back verbatim and re-hashes before
  returning, so it cannot change the info hash even for a torrent whose
  original encoding was not canonical.

### Not in this release

`bench`, Metalink resolution, BEP 52 v2 and hybrid creation, BEP 16
superseeding, `--log-file` rotation, and `-i/--input-file`. Each has an entry
in `TODO/` with what closes it. Nothing is stubbed: a command that is not
implemented says so and exits with a code a script can branch on.
