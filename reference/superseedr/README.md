# superseedr

Rust + Tokio BitTorrent client with a terminal UI. Actor-style concurrency:
the `App` owns display state, one `TorrentManager` task per torrent owns
protocol logic, and `PeerSession` owns a single TCP/uTP connection.

> **Note on this copy.** The upstream `README.md` was end-user and marketing
> material (install methods, Docker/Gluetun setup, key bindings, screenshots)
> and was replaced by this index. Also removed: `assets/`, `wix/`,
> `packaging/`, `agentic_prompts/`, `agentic_testing/`, `AGENTS.md`,
> `docs/CHANGELOG.md`, `docs/FAQ.md`, `docs/REMOTE_DEV.md`, `docs/ROADMAP.md`,
> `docs/linux-install-artifact-testing.md`,
> `docs/release-candidate-testing.md`, the packaging/installer scripts under
> `scripts/`, and the UI-only plans under `agentic_plans/`.

## Where things are

| Path | What |
|---|---|
| `docs/architecture.md` | Layering, actor model, adaptive pipelining, peer session split |
| `docs/tuning.md`, `docs/synthetic-benchmark.md` | Performance knobs and the synthetic load harness |
| `docs/cli.md`, `docs/shared-config.md`, `docs/configuration-and-backups.md` | CLI surface and layered config |
| `docs/dht-ownership-plan.md` | Why the `mainline` crate was replaced by an in-tree DHT |
| `docs/integration-harness.md`, `docs/integration-e2e-automation-plan.md` | The Docker interop harness |
| `docs/fuzzing.md`, `fuzz/`, `proptest-regressions/` | Fuzz targets and recorded proptest counterexamples |
| `src/dht/` | BEP 5 KRPC, routing, tokens, lookup planner, `bep42.rs` (DHT security extension) |
| `src/networking/` | `session.rs` (peer wire), `utp.rs` (BEP 29), `transport.rs`, `web_seed_worker.rs`, `shared_udp.rs` |
| `src/torrent_manager/` | `manager.rs`, `state.rs`, `piece_manager.rs`, `block_manager.rs`, `merkle.rs` (BEP 52) |
| `src/torrent_file/` | Metainfo parser |
| `src/tracker/` | HTTP + UDP tracker client |
| `src/control_service.rs` | Control requests applied online to a running instance or offline to the catalog |
| `src/integrity_scheduler.rs` | Continuous background integrity probing |
| `integration_tests/` | Docker matrix: qBittorrent, Transmission, libtorrent lab; v1/v2/hybrid fixtures under `integration_tests/torrents/` |
| `agentic_plans/` | Design notes, including `v2_identity_lossiness_review_2026-04-14.md` on v1/v2/hybrid info-hash identity |
| `scripts/` | `extract_merkle.py`, `generate_integration_torrents.py`, `hash.py`, `summarize_dht_soak.py`, fuzz and FD-count helpers |

## Scope

Private-tracker builds can disable DHT and PEX. Magnet links, RSS, watch
folders and a shared multi-instance catalog ("cluster mode") are supported.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / lock files were removed
> corpus-wide, so this tree is for reading, not building.
