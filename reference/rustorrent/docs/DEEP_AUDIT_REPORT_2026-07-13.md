# Whole-Codebase Audit and Correction Report

Date: 2026-07-13

Scope: the complete Rust application, protocol implementations, web/search/RSS surfaces,
filesystem and persistence layer, macOS launchers and packaging, fuzz package, tests, and CI.

## Audit posture

The review treated torrent metadata, magnets, trackers, peers, DHT packets, web seeds, RSS feeds,
search-plugin inputs and outputs, UI requests, state files, and payload directories as untrusted.
It also checked crash recovery, concurrent lifecycle transitions, bounded resource use, proxy
bypasses, and binary-distribution requirements. Corrections were made in place and paired with
deterministic regression tests where the behavior could be exercised locally.

This report records a broad security and correctness pass, not a claim that a networked client can
be proven defect-free. The remaining environmental and interoperability limits are listed below.

## Material corrections

### Parsing and metainfo

- Bencode now rejects non-canonical integers, unsorted or duplicate dictionary keys, excessive
  depth, excessive value counts, invalid lengths, trailing data, and truncated structures.
- Torrent parsing validates exact v1 piece counts, v2 file trees, piece layers, Merkle roots,
  hybrid layout consistency, length arithmetic, file/path collisions, and collection limits.
- XML/RSS parsing is linear and bounded by depth, node, attribute, text, and collection budgets;
  malformed nesting, namespace edge cases, entity bombs, and UTF-8 handling have regression tests.
- Tracker, HTTP, peer, extension, MSE, uTP, DHT, GeoIP, blocklist, config, resume, and session
  decoders now enforce explicit framing and allocation limits before committing resources.

### Network boundaries and protocol behavior

- HTTP(S) tracker and generic HTTP clients use absolute deadlines, bounded response bodies,
  redirect limits, HTTPS downgrade prevention, cancellation points, and public-address policy.
- Tracker, magnet, DHT, PEX, and ordinary peer candidates are normalized and filtered by source;
  public sources cannot inject loopback, private, link-local, mapped-private, or special-use peers.
- Proxy mode is fail-closed: only peer TCP and HTTP(S) trackers use the proxy. Unsupported direct
  paths—including inbound peers, DHT, LPD, uTP, UDP trackers, port mapping, RSS/search downloads,
  web seeds, and magnet HTTP sources—are disabled instead of leaking around it.
- Proxy CONNECT/SOCKS framing preserves over-read bytes, uses bounded resolution workers and
  deadlines, and leaves destination hostname resolution to the proxy where the protocol permits.
- MSE preserves buffered ciphertext/plaintext across handshake boundaries and bounds every padding
  synchronization path. Peer framing, BEP 52 hash messages, hybrid upgrades, uTP windows/queues,
  UDP transaction matching, DHT admission, DHT bucket refresh, and replacement probing were
  corrected and covered by local fixtures.
- Network-derived log text and URLs are sanitized, bounded, and stripped of credentials/query
  secrets before terminal or file logging.

### Lifecycle, memory, and concurrency

- A torrent is reserved from queue removal through worker registration, closing the duplicate
  worker window. Registry collisions fail without replacing the original worker, and stop/delete/
  archive operations refuse a still-loading torrent.
- Path-backed requests are frozen to immutable bytes when queued, preventing a watched or UI-added
  torrent from changing between duplicate validation and worker parsing.
- Peer and web-seed payload buffers share atomic RAII permits with per-torrent and process-wide
  budgets (approximately 64 MiB and 256 MiB at the configured maximum piece size). Budget
  exhaustion releases the piece reservation and applies backpressure; pause, stop, completion,
  redirect, hash failure, stale work, and persistence paths all return permits.
- Peer slots, metadata discovery, trackers, DHT tables/rate state, UI connections and bodies,
  subprocess output, resolver workers, write caches, queues, and shutdown joins are bounded.
- Resume data no longer trusts file timestamps as proof of piece validity: claimed pieces are
  rehashed. Failed writes preserve recoverable buffers, while normal and error teardown release
  reservations and wake blocked peer I/O.

### Filesystem and durable state

- Torrent paths reject traversal, reserved application names, duplicate/prefix collisions,
  symlinks/reparse points, non-regular files, and hard-linked aliases.
- Payload files are exclusively locked, opened beneath a pinned download root, and checked by
  stable file identity. Rename is atomic and no-overwrite; move/delete operations retain durable
  ownership claims and cannot follow a swapped payload parent.
- Session, resume, RSS, DHT, lock, rename-journal, completion-move, and delete-tombstone state is
  bounded and crash-safe. Atomic publication fsyncs data before rename, preserves a bounded backup,
  distinguishes post-publish sync failure, and fails closed when both primary and backup are bad.
- Unix and Windows state/payload backends use descriptor- or handle-relative traversal with
  no-follow semantics. Windows state ACLs are provenance-checked before consumption, new objects
  receive a protected current-user/SYSTEM/Administrators ACL, and ReFS-safe identity comparisons
  use the volume serial plus full 128-bit file identifier. State-directory handles are cached with
  a hard bound and verify their original root/directory identities after eviction.
- Completion moves, file renames, archive, stop, and delete transitions keep the registry and
  ownership claims alive until storage is actually closed, preventing races between UI actions and
  live workers.

### UI, search, and application surfaces

- The web UI binds only to loopback. Mutations require a high-entropy token plus a valid local
  Origin/Host, while DNS-rebinding hosts cannot read the launcher token or mutate state.
- Request lines, headers, chunk metadata, endpoint bodies, uploads, response JSON, and rendered
  untrusted strings are bounded/escaped. Unauthorized uploads are rejected before their declared
  body is consumed.
- Search runtime/plugin paths reject traversal, links, non-regular and oversized files. Network
  installation requires HTTPS; only Python 3.9 or newer is accepted; subprocesses have bounded
  output, deadlines, and descendant cleanup.
- RSS downloads remain retryable until a successful durable enqueue, seen identifiers are scoped
  per feed, and all feed/rule/state collections are capped.

### Build, platform, and distribution

- CI now gates formatting, strict all-target/all-feature linting, full and minimal tests, every
  optional feature, release builds, fuzz-target linting, Rust 1.89, macOS/Windows builds, and RustSec
  audits for both lockfiles.
- macOS packaging explicitly targets macOS 11, type-checks/builds both x86_64 and arm64 launchers,
  creates a universal backend, requires the icon/notices, bounds launcher probes, uses safe append
  logging, and assembles final archives only after app notarization/stapling when credentials exist.
- The source license, qBittorrent-derived search-runtime notice, and a reproducible cargo-about
  report containing license text and attribution for every dependency selected for supported
  distribution targets are included in the app.

## Validation record

The final tree was checked with the following release gates:

| Gate | Result |
|---|---|
| Strict Rust formatting and `git diff --check` | Pass |
| Strict Clippy, all targets and all features | Pass |
| Full feature suite | 450 unit/local tests + 9 process tests + 17 short swarm tests passed; 1 long soak ignored |
| Minimal feature suite | Pass |
| Every optional feature independently | Pass |
| Release, documentation, source package, and Rust 1.89 builds | Pass |
| macOS arm64/x86_64 cross-builds, full and minimal | Pass |
| Windows x86_64 cross-builds, full/minimal/test targets, current Rust and 1.89 | Pass |
| Application and fuzz dependency advisory audits | Pass; no advisories or warnings |
| Fuzz-target formatting/linting and bounded smoke runs | Pass; 100 coverage-guided runs per target without sanitizer |
| Universal macOS application/ZIP inspection | Pass; x86_64 + arm64, minimum macOS 11, icon/notices included |
| Reproducible dependency-license generation | Pass |

The authoritative commands and current test inventory are maintained in
[`TEST_COVERAGE.md`](TEST_COVERAGE.md).

## Remaining limits

- Public tracker/DHT behavior and third-party client interoperability extend beyond deterministic
  local fixtures. Router-specific NAT-PMP/UPnP behavior still needs real hardware coverage.
- Hybrid torrents perform the BEP 52 handshake upgrade, but tracker, DHT, and LPD discovery do not
  independently join both v1 and v2 swarms.
- Resolver worker counts and deadlines are bounded, but operating-system DNS calls themselves
  cannot be forcibly cancelled. A proxy hostname is therefore still resolved locally.
- Installed search plugins are executable third-party Python code and intentionally run with the
  current user's privileges; they are not an application sandbox or a same-user security boundary.
- The normal release gate runs a short mixed-swarm scenario and bounded fuzz smoke runs. Sustained
  fuzz corpus growth, sanitizer campaigns, multi-hour memory/throughput soaks, and randomized
  mid-piece crash recovery remain separate long-running work. On this macOS 26 host, the nightly
  AddressSanitizer runtime deadlocked during its own initialization, so the recorded finite fuzz
  runs used coverage instrumentation without a sanitizer.
- Windows is cross-compiled and has platform-specific filesystem code, but the complete process and
  adversarial suite has not yet been observed on a physical Windows runner in this local audit.
- The credential-free audit could not exercise live Developer ID signing, Apple notarization, or
  stapling. CI builds and inspects the universal ZIP, but not the DMG/notary path or a launched-app
  end-to-end flow. Linux aarch64 is source-supported but not continuously runtime-tested in CI.

These are test-environment or protocol-coverage boundaries, not silent guarantees; they should
remain visible in release notes until corresponding automated or manual gates exist.
