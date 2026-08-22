# Rustorrent Test Coverage and Scope

Last inventory refresh: 2026-07-13

## Purpose

The automated suite is designed to catch deterministic correctness, safety, and resilience
regressions without depending on public trackers, public DHT bootstrap nodes, or a particular
consumer router. It is a release gate, not a claim of complete BitTorrent ecosystem
interoperability.

## Current Suite Snapshot

The following inventory was compiled with:

```sh
cargo test --all-features -- --list
```

| Suite | Defined tests |
|---|---:|
| Unit and local-fixture tests (`src/main.rs` and modules) | 450 |
| Adversarial process tests (`tests/process_release_gate.rs`) | 9 |
| Swarm/uTP tests (`tests/soak_swarm.rs`) | 18 |
| **Total** | **477** |

One long-running mixed-swarm soak test is ignored by default. A normal full run therefore executes
476 tests and reports one ignored test.

Exact totals will grow as coverage is added. Treat the commands below—not hard-coded module
counts—as the authoritative release gates.

## What the Suite Covers

| Area | Representative coverage |
|---|---|
| Metainfo and parsing | Canonical bencode, size/depth limits, v1/v2/hybrid torrents, piece layers, malformed/truncated inputs, XML/RSS, HTTP and tracker responses |
| Hashing and piece state | SHA-1/SHA-256 vectors, BEP 52 Merkle roots, hybrid dual verification, shared peer/web-seed memory budgets, reservation cleanup, rarest/sequential selection, priorities |
| Storage and filesystem safety | Cross-file I/O, v2-aligned offsets, bounds checks, cache flush/drop behavior, symlink/hardlink/reparse defenses, Windows ACL and full file-identity checks, safe delete/move/rename behavior |
| Peer and transport protocols | Handshakes, incremental message framing, MSE buffered data, TCP/uTP fixtures, DHT, LPD, HTTP/UDP trackers, proxy parsing and CONNECT behavior |
| Runtime resilience | Rate-limit concurrency, session locking, atomic session/resume recovery, bounded metadata/tracker inputs, peer retry/ban behavior, shutdown and worker lifecycle |
| UI and application behavior | Request authorization, origin/token checks, body limits, add/archive/recheck commands, status JSON, stable identifiers, Python compatibility probing, search/RSS parsing, UTF-8 handling |
| Adversarial process behavior | Slowloris pressure, connection churn, malformed encryption/extension frames, oversized frames, corrupt state recovery, repeated kill/restart loops |
| Local integration fixtures | HTTP tracker, UDP tracker, peer handshake, DHT response, uTP connector/listener, web UI process probes |

## Fuzz Targets

The `fuzz/` package contains buildable entry points for:

- bencode parsing and encode/decode round trips;
- peer-message decoding and framed reads;
- HTTP response and tracker-body parsing;
- storage path-segment validation;
- full torrent-metainfo parsing.

Lint and compile every fuzz target without starting an unbounded fuzz run:

```sh
cargo clippy --locked --manifest-path fuzz/Cargo.toml --bins -- \
  -D warnings -A dead-code
cargo audit --deny warnings
cargo audit --file fuzz/Cargo.lock --deny warnings
cargo about generate --locked --all-features --fail about.hbs \
  --output-file /tmp/THIRD_PARTY_LICENSES.html
cmp THIRD_PARTY_LICENSES.html /tmp/THIRD_PARTY_LICENSES.html
```

The audit commands require `cargo-audit` (`cargo install cargo-audit --version 0.22.2 --locked`).
The license check requires `cargo-about` 0.9.1 with its `cli` feature. CI installs both pinned
tools automatically.

Finite local fuzz smoke runs use a nightly toolchain and an explicit run count, for example:

```sh
cargo +nightly fuzz run -s none bencode_parse -- -runs=100 -timeout=3
```

The 2026-07-13 macOS 26 validation used `-s none` because the native nightly AddressSanitizer
runtime deadlocked during its own initialization. Sanitized, sustained campaigns remain a separate
release activity rather than a claimed result of this smoke command.

## Release Gates

Run the same checks enforced by CI:

```sh
cargo fmt --all --check
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features -- --test-threads=1
cargo test --locked --no-default-features -- --test-threads=1
cargo check --locked --release --all-targets --all-features
cargo clippy --locked --manifest-path fuzz/Cargo.toml --bins -- \
  -D warnings -A dead-code
```

The CI workflow runs the main gate on Linux, checks the declared Rust 1.89 minimum on Linux,
macOS, and Windows, and compile-checks both the full and minimal feature sets on each platform. The
macOS job also builds the universal application bundle, verifies both architectures and the macOS
11 deployment target, and confirms that dependency notices are reproducible.

Run only the adversarial process gate with visible child output:

```sh
cargo test --all-features --test process_release_gate -- --nocapture
```

Run the ignored soak scenario for five minutes:

```sh
RUSTORRENT_SOAK_SECS=300 \
  cargo test --all-features --test soak_swarm -- --ignored --nocapture
```

## Known Gaps

- Public tracker, public DHT, and third-party client interoperability varies beyond the local
  deterministic fixtures.
- Hybrid torrents accept the BEP 52 handshake upgrade, but tracker, DHT, and LPD discovery do not
  yet join the v1 and v2 swarms independently.
- Router-specific UPnP and NAT-PMP behavior needs hardware coverage.
- Long-duration memory, CPU, throughput, reconnect, and queue-latency trends are not continuously
  measured.
- Fuzz targets compile in CI, but sustained corpus growth, minimization, and sanitizer campaigns
  are not yet scheduled.
- Search plugins are executable third-party Python code running with the current user's authority;
  the application warns before installation but does not sandbox them.
- Crash/restart tests validate state-file integrity; they do not yet kill and resume a guaranteed
  active piece transfer at randomized checkpoints.
- Windows-specific handle-relative filesystem behavior cannot run on Unix hosts; the Windows CI
  job therefore runs the full feature/process suite in addition to both compile configurations.
- Credential-free CI does not execute Developer ID signing/notarization, the DMG path, or a full
  launched macOS app flow. Linux aarch64 does not have a continuous runtime gate.

These gaps should remain explicit in release notes until corresponding automated or manual gates
exist.
