# gosh-dl

> **Note on this copy.** Only BitTorrent-related material was researched. The
> quick-start, building, FAQ and migration-guide sections of this file were
> removed; nothing else was deleted from the tree.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.
A fast, embeddable download engine for Rust applications. Supports HTTP/HTTPS with multi-connection acceleration and full BitTorrent protocol including DHT, PEX, encryption, and WebSeeds.


## Why gosh-dl?

gosh-dl brings download functionality directly into your Rust application as a native library, eliminating the complexity of managing external processes, parsing JSON-RPC responses, or bundling platform-specific binaries. Modern applications demand seamless integration, and gosh-dl delivers exactly that; async function calls that feel natural in your codebase, compile-time type safety that catches errors before runtime, and shared memory that keeps your application lightweight and responsive.

Whether you're building a media application that needs BitTorrent with streaming support, a package manager requiring resilient HTTP downloads with checksums and mirrors, or any software that moves files across the network, gosh-dl provides the complete feature set you need. Multi-connection acceleration splits large downloads across parallel connections for maximum throughput. Automatic resume with ETag validation ensures interrupted transfers pick up exactly where they left off. Full BitTorrent support includes DHT for trackerless operation, peer exchange for efficient swarm discovery, and protocol encryption for privacy.

The engine handles the complexity of segmented downloads, tracker communication, DHT peer discovery, and connection encryption while exposing a clean, intuitive API that integrates naturally with Tokio-based applications. Priority queues let you control which downloads matter most, bandwidth scheduling adapts to time-of-day constraints, and persistence — built-in SQLite, JSON sidecar files, or your own `Storage` implementation — ensures nothing is lost across restarts.

A standalone CLI is available in the companion `gosh-dl-cli` project for users who want command-line access to the engine.

## Features

### Tested & Production-Ready

| Feature | Details |
|---------|---------|
| Multi-connection HTTP/HTTPS | Up to 16 parallel connections per download |
| Content-Disposition detection | Automatic filename from server headers |
| Custom headers | User-Agent, Referer, cookies, arbitrary headers |
| Checksum verification | MD5, SHA-256 |
| Concurrent download management | Priority queue (Critical/High/Normal/Low) |
| Pause / resume / cancel | Full lifecycle control, per download or in batch (`pause_all` / `resume_all` / `cancel_all`) |
| Event system | Broadcast channels for progress, state changes |
| Global statistics | Active count, aggregate speeds |
| SQLite persistence | WAL mode, schema versioning, crash recovery |
| Pluggable persistence | Inject any `Storage` impl via `with_storage()`; built-in `FileStorage` JSON sidecars (aria2 control-file analog) |

### Tested BitTorrent Core

| Feature | BEP | Details |
|---------|-----|---------|
| .torrent parsing | 3 | Single-file and multi-file |
| Magnet URI | 9 | Metadata fetching from peers |
| Multi-peer downloading | 3 | Piece selection, block pipelining |
| Piece hash verification | 3 | SHA-1 per piece |
| HTTP & UDP trackers | 3, 15 | Announce, scrape |
| Sequential download | — | For streaming playback |
| Torrent crash recovery | — | Resume from SQLite-stored torrent data |
| IPv6 tracker peers | 7 | Compact `peers6` parsing |

### Implemented, Lightly Tested

| Feature | BEP | Notes |
|---------|-----|-------|
| DHT peer discovery | 5 | Works, disabled in CI tests |
| Peer Exchange (PEX) | 11 | Implemented, disabled in CI tests |
| Local Peer Discovery | 14 | Implemented, disabled in CI tests |
| Message Stream Encryption | MSE/PE | RC4 + DH key exchange, unit tests only |
| WebSeeds | 17, 19 | Hoffman + GetRight, including cross-file pieces |
| uTP transport | 29 | LEDBAT congestion control, wired into peer connections (opt-in) |
| HTTP resume | — | ETag/Last-Modified validation |
| Mirror/failover | — | Automatic failover to alternate URLs |
| Bandwidth scheduling | — | Time-of-day rules with live runtime limit updates |
| Recursive HTTP mirroring | — | Feature-gated via `recursive-http`; crawls HTML directory indexes with bounded-concurrency discovery and expands into ordinary HTTP downloads |
| Private torrent handling | 27 | Disables DHT/PEX/LPD |
| Choking algorithm | — | Unchoke rotation, optimistic unchoking |

### Planned / Stub

| Feature | Notes |
|---------|-------|
| DHT IPv6 | Depends on upstream `mainline` crate |
| Proxy support | Config field exists, not tested |
| File preallocation | Config field exists, not tested |

## API Overview

All public types are available at the crate root via re-exports. For explicit imports, use `gosh_dl::protocol`:

```rust
use gosh_dl::protocol::{DownloadEvent, DownloadStatus, ProtocolError};
```

### Download Management

```rust
// Add downloads
let http_id = engine.add_http(url, options).await?;
let torrent_id = engine.add_torrent(&torrent_bytes, options).await?;
let magnet_id = engine.add_magnet(magnet_uri, options).await?;

// Control
engine.pause(id).await?;
engine.resume(id).await?;
engine.cancel(id, delete_files).await?;

// Batch control (aria2 pauseAll/unpauseAll analogs).
// Pauses queued downloads too, and returns per-download outcomes.
let result = engine.pause_all().await;
println!("paused {}, skipped {}", result.succeeded.len(), result.skipped.len());
engine.resume_all().await;
engine.cancel_all(delete_files).await;

// Priority
engine.set_priority(id, DownloadPriority::High)?;

// Status
let status = engine.status(id);
let all = engine.list();
let active = engine.active();
let waiting = engine.waiting();
let stopped = engine.stopped();
let stats = engine.global_stats();
```

With `recursive-http` enabled:

```rust
let manifest = engine
    .discover_http_recursive(root_url, &options, &recursive_options)
    .await?;

let job = engine
    .add_http_recursive(root_url, options, recursive_options)
    .await?;

let aggregate = engine.recursive_job_status(&job);
println!(
    "{:?} ({}/{})",
    aggregate.state,
    aggregate.progress.completed_children,
    aggregate.progress.total_children,
);

let tracked = engine.list_recursive_jobs();
println!("tracked jobs: {}", tracked.len());

if let Some(parent) = tracked.first() {
    let mut recursive_events = engine.subscribe_recursive_jobs();
    engine.cancel_recursive_job(parent.id, false).await?;
    engine.remove_recursive_job(parent.id, false).await?;

    for _ in 0..2 {
        if let Ok(event) = recursive_events.recv().await {
            println!("recursive event: {:?}", event);
        }
    }
}
```

### Download Options

```rust
use gosh_dl::{DownloadOptions, DownloadPriority, ExpectedChecksum};

let options = DownloadOptions {
    priority: DownloadPriority::High,
    save_dir: Some(PathBuf::from("/downloads")),
    filename: Some("custom_name.zip".to_string()),
    user_agent: Some("MyApp/1.0".to_string()),
    referer: Some("https://example.com".to_string()),
    headers: vec![("Authorization".to_string(), "Bearer token".to_string())],
    cookies: Some(vec!["session=abc123".to_string()]),
    checksum: ExpectedChecksum::parse("sha256:abcd1234..."),
    mirrors: vec!["https://mirror1.example.com/file.zip".to_string()],
    max_connections: Some(8),
    max_download_speed: Some(5 * 1024 * 1024), // 5 MB/s
    // Torrent-specific
    selected_files: Some(vec![0, 2, 5]), // Download only specific files
    sequential: Some(true), // For streaming playback
    ..Default::default()
};
```

### Recursive HTTP

When the `recursive-http` feature is enabled, the engine also exposes `RecursiveOptions`:

```rust
use gosh_dl::RecursiveOptions;

let recursive = RecursiveOptions {
    max_depth: 4,
    include_patterns: vec!["*.txt".to_string()],
    exclude_patterns: vec!["private/*".to_string()],
    ..Default::default()
};
```

Current scope:

- crawls HTML directory/index pages and follows `<a href>` links
- same-host only by default
- constrained to the root path prefix by default
- discovered files are queued as ordinary HTTP downloads
- rolls back already-added child downloads if recursive enqueue fails partway through
- optional `fail_fast` cancels queued/active sibling child downloads after the first child failure
- persists recursive child runtime context needed for redirect-scope and fail-fast recovery
- persists tracked parent recursive jobs and restores them on restart
- exposes aggregate parent status, lifecycle methods, and a dedicated parent event stream
- propagates headers, cookies, user-agent, and referer during discovery
- discovery fetches pages concurrently, bounded by `max_discovery_concurrency` (default 4)

Current limitations:

- opt-in via the `recursive-http` Cargo feature
- not full `wget -r` parity
- no JavaScript rendering
- recursive parent jobs use a separate event stream via `subscribe_recursive_jobs()`, not the main `DownloadEvent` stream
- recursive redirect scope is enforced in discovery, child file downloads, and resumed child downloads restored from storage, but recursive jobs are still not resumable as crawls
- recursive jobs are persisted and listable, but still do not participate in the main download queue/event model as first-class parent downloads
- no persisted parent-level event or progress history beyond the tracked job record itself

### Events

```rust
use gosh_dl::DownloadEvent;

let mut events = engine.subscribe();
while let Ok(event) = events.recv().await {
    match event {
        DownloadEvent::Added { id } => println!("Added: {}", id),
        DownloadEvent::Started { id } => println!("Started: {}", id),
        DownloadEvent::Progress { id, progress } => {
            println!("{}: {:.1}% at {} KB/s",
                id,
                progress.percentage(),
                progress.download_speed / 1024
            );
        }
        DownloadEvent::StateChanged { id, old_state, new_state } => {
            println!("{}: {:?} -> {:?}", id, old_state, new_state);
        }
        DownloadEvent::Completed { id } => println!("Done: {}", id),
        DownloadEvent::Failed { id, error, retryable } => {
            eprintln!("Failed {}: {} (retryable: {})", id, error, retryable);
        }
        DownloadEvent::Paused { id } => println!("Paused: {}", id),
        DownloadEvent::Resumed { id } => println!("Resumed: {}", id),
        DownloadEvent::Removed { id } => println!("Removed: {}", id),
    }
}
```

## Configuration

```rust
use gosh_dl::{EngineConfig, HttpConfig, TorrentConfig};
use gosh_dl::config::WebSeedConfig;
use std::path::PathBuf;

let config = EngineConfig {
    download_dir: PathBuf::from("/downloads"),
    max_concurrent_downloads: 5,
    max_connections_per_download: 16,
    min_segment_size: 1024 * 1024, // 1 MB
    global_download_limit: Some(10 * 1024 * 1024), // 10 MB/s
    global_upload_limit: Some(5 * 1024 * 1024), // 5 MB/s
    user_agent: "MyApp/1.0".to_string(),
    enable_dht: true,
    enable_pex: true,
    enable_lpd: true,
    max_peers: 55,
    seed_ratio: 1.0,
    database_path: Some(PathBuf::from("/data/gosh-dl.db")),
    http: HttpConfig {
        max_retries: 8,
        read_timeout: 90,
        ..Default::default()
    },
    torrent: TorrentConfig {
        webseed: WebSeedConfig {
            enabled: true,
            max_connections: 6,
            ..Default::default()
        },
        ..Default::default()
    },
    ..Default::default()
};
```

You can also apply a replacement config at runtime with `engine.set_config(config)?;`.
Queue concurrency and global bandwidth limits are applied to the live engine when you do this.

### Persistence & Custom Storage

Setting `database_path` persists download state to the built-in SQLite storage
(`storage` feature) so downloads resume across restarts. If you maintain your
own metadata store — or just prefer plain files — inject any implementation of
the `Storage` trait instead:

```rust
use std::sync::Arc;
use gosh_dl::{DownloadEngine, EngineConfig, FileStorage};

// aria2-control-file style JSON sidecars, one per download:
let storage = Arc::new(FileStorage::new("/data/gosh-dl-state").await?);
let engine = DownloadEngine::with_storage(EngineConfig::default(), storage).await?;
```

`FileStorage` (JSON sidecar files) and `MemoryStorage` ship with the crate and
work without the `storage` feature. To bring your own database, implement the
`Storage` trait (the `#[async_trait]` attribute is re-exported from
`gosh_dl::storage`) and pass it to `DownloadEngine::with_storage`.

### Bandwidth Scheduling

```rust
use gosh_dl::{EngineConfig, ScheduleRule};

// Limit bandwidth during work hours (Mon-Fri, 9am-5pm)
let work_hours = ScheduleRule::weekdays(
    9,                      // start_hour
    17,                     // end_hour
    Some(1024 * 1024),      // download_limit: 1 MB/s
    None,                   // upload_limit: unlimited
);

let config = EngineConfig::default()
    .add_schedule_rule(work_hours);
```

## Why an API Instead of RPC?

Traditional download managers like aria2 use JSON-RPC for external communication. This works well for standalone tools, but creates friction when embedding download functionality into applications:

**With RPC (aria2 approach):**
```
Your App → Serialize JSON → HTTP/WebSocket → aria2 Process → Parse JSON → Execute
         ← Parse JSON    ← HTTP/WebSocket  ←              ← Serialize JSON ← Result
```

**With native API (gosh-dl approach):**
```
Your App → engine.add_http(url, opts) → Result
```

### Benefits of the API Approach

- **Zero serialization overhead**: No JSON encoding/decoding on every call. Function arguments pass directly through memory.
- **Compile-time guarantees**: The Rust compiler catches type mismatches, missing parameters, and invalid states before your code runs. RPC errors only surface at runtime.
- **Native error handling**: Use `?` operator, pattern matching on `Result`, and standard Rust error propagation. No parsing error strings from JSON responses.
- **No process coordination**: No need to spawn aria2, monitor if it crashed, restart it, or manage its lifecycle. The engine lives in your process.
- **Shared memory space**: Progress callbacks, event streams, and status queries happen in-process. No IPC latency or message queue bottlenecks.
- **Single deployment artifact**: Ship one binary. No bundling platform-specific aria2 executables or dealing with PATH issues.
- **IDE integration**: Autocomplete, go-to-definition, inline docs all work. RPC calls are opaque strings to your editor.

---

## Comparison with aria2

gosh-dl was designed as a native Rust alternative to [aria2](https://aria2.github.io/), the popular C++ download utility. While aria2 is excellent as a standalone tool, embedding it in applications requires spawning an external process and communicating via JSON-RPC.

| Aspect | aria2 | gosh-dl |
|--------|-------|---------|
| **Integration** | External process + JSON-RPC | Native library calls |
| **Deployment** | Bundle platform binaries | Single Rust crate |
| **Type Safety** | JSON strings | Rust types with compile-time checks |
| **Error Handling** | Parse JSON responses | Native `Result<T, E>` |
| **Process Management** | Handle lifecycle, crashes | None required |
| **Memory** | Separate process | Shared with your app |

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- Uses [mainline](https://crates.io/crates/mainline) for DHT support
