# FluxDown (Rust engine only)

Multi-protocol download manager. This copy has been reduced to the Rust
workspace under `native/`; the Flutter app, browser extension, web SPA,
website, promotion and packaging trees were removed as non-technical for
BitTorrent research.

## Layout

| Path | What |
|---|---|
| `native/engine/` | `fluxdown_engine` — download engine (Tokio). BitTorrent, HTTP/HTTPS, FTP, HLS/DASH, eD2K, SQLite state. |
| `native/engine/vendor/librqbit/` | Vendored `librqbit` 8.1.1 (MIT) — its `Cargo.toml` is the version evidence and is kept. Upstream wired it in with `[patch.crates-io]` in the workspace manifest, which was removed with the rest of the manifests. |
| `native/api/`, `native/cli/`, `native/server/` | REST/aria2-compatible API, CLI, headless server. |
| `native/hub/` | FFI adapter (Rinf signals) used only by the removed Flutter app. |
| `native/nmh/` | Native-messaging host for the removed browser extension. |

## BitTorrent-relevant engine modules

- `native/engine/src/bt_downloader.rs` — librqbit driver: file selection,
  BEP 47 padding-file filtering, completion layout, `verify_pieces_core`.
- `native/engine/src/bt_sparse.rs` — Windows/NTFS `FSCTL_SET_SPARSE` wrapper
  around `FilesystemStorage`, applied after `init` and before any `set_len`.
- `native/engine/src/bt_partfile.rs` — `.parts` sidecar so a partially
  selected torrent can seed: selected files map to their final paths and
  unselected bytes (including cross-file-boundary piece bytes) are served
  from a blob instead of being recreated on disk.
- `native/engine/src/bt_seeding.rs`, `segment_coordinator.rs` (IDM-style
  dynamic HTTP segmentation), `cdn/` (node pool, health, hints).

Comments in the engine are largely in Chinese.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.

