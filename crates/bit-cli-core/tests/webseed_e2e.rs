//! End-to-end tests for the web seed bridge.
//!
//! These run a real `librqbit` session and a stub HTTP server over loopback,
//! so they exercise the whole path: handshake, extended handshake, bitfield,
//! piece requests, ranged GETs, and the session's own hash verification.
//!
//! Nothing here reaches the network. The stub server binds `127.0.0.1:0` and
//! the session binds an OS-chosen port, so the tests never collide with each
//! other or with anything else on the machine.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bit_cli_core::engine::{AddOptions, Engine, EngineOptions};
use bit_cli_core::layout::Layout;
use bit_cli_core::webseed::binding::{BindingSet, Origin, SourceSpec};
use bit_cli_core::webseed::bridge::{self, BridgeParams, BridgeState, BridgeStatus};
use bit_cli_core::webseed::fetch::Fetcher;
use bit_cli_core::webseed::scope::Scope;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Piece length for every fixture. Small enough to keep the payloads tiny,
/// large enough that pieces span file boundaries in the multi-file cases.
const PIECE_LENGTH: u32 = 32 * 1024;

/// How the stub server answers, so the failure paths are exercised too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServeMode {
    /// Honour `Range` properly.
    Ranges,
    /// Ignore `Range` and return the whole entity with `200 OK`.
    IgnoreRange,
    /// Answer everything with `404`.
    NotFound,
    /// Honour `Range` but return the wrong bytes.
    Corrupt,
    /// Speak BEP 17: `?info_hash=&piece=&ranges=` instead of a `Range` header.
    Hoffman,
    /// Redirect once, then serve properly from the new location.
    Redirect,
    /// Answer 403 the first time each range is asked for and serve it on the
    /// retry. It is what a signing CDN does when a signature expires: the
    /// refusal is real, and the next request to the same URL succeeds because
    /// it is redirected to a fresh signature.
    ExpiringSignature,
}

/// Deterministic pseudorandom bytes, so fixtures have real piece hashes
/// without depending on a random source.
fn content(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as u8
        })
        .collect()
}

/// Bytes the stub server has sent, so a test can catch a source being asked
/// for the same range twice.
type Served = Arc<AtomicU64>;

/// Serve `root` over HTTP on loopback, returning the base URL.
///
/// Deliberately minimal: enough of HTTP/1.1 to answer the ranged GETs the
/// fetcher issues, and nothing else.
async fn serve(root: PathBuf, mode: ServeMode) -> (String, Served) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let served: Served = Served::default();
    let counter = served.clone();
    let refused: Refused = Refused::default();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let root = root.clone();
            let counter = counter.clone();
            let refused = refused.clone();
            tokio::spawn(async move {
                let _ = handle_request(stream, root, mode, counter, refused).await;
            });
        }
    });
    (format!("http://127.0.0.1:{port}/"), served)
}

/// Ranges [`ServeMode::ExpiringSignature`] has already refused once.
///
/// Keyed by the target and the range, so the refusal follows the range rather
/// than the connection and every distinct range is refused exactly once.
type Refused = Arc<std::sync::Mutex<std::collections::HashSet<String>>>;

async fn handle_request(
    mut stream: TcpStream,
    root: PathBuf,
    mode: ServeMode,
    served: Served,
    refused: Refused,
) -> std::io::Result<()> {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).await? == 0 {
            return Ok(());
        }
        request.push(byte[0]);
    }
    let request = String::from_utf8_lossy(&request).to_string();
    let mut lines = request.lines();
    let target = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();
    // Header names are case-insensitive, and every HTTP client spells this
    // one differently.
    let range = lines
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("range")
                .then(|| value.trim().strip_prefix("bytes="))?
        })
        .and_then(parse_range);

    // BEP 17 puts the piece and the sub-range in the query string, and there
    // is no `Range` header at all.
    if mode == ServeMode::Hoffman {
        return serve_hoffman(&mut stream, &root, &target, served).await;
    }
    if mode == ServeMode::ExpiringSignature {
        let key = format!("{target} {range:?}");
        let first_time = refused.lock().unwrap().insert(key);
        if first_time {
            return respond(&mut stream, 403, "Forbidden", None, b"").await;
        }
    }
    if mode == ServeMode::Redirect && !target.starts_with("/moved/") {
        let head = format!(
            "HTTP/1.1 302 Found\r\nLocation: /moved{target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(head.as_bytes()).await?;
        return stream.flush().await;
    }
    let target = target
        .strip_prefix("/moved")
        .map(str::to_string)
        .unwrap_or(target);

    let path = root.join(percent_decode(target.trim_start_matches('/')));
    let Ok(body) = std::fs::read(&path) else {
        return respond(&mut stream, 404, "Not Found", None, b"missing").await;
    };
    if mode == ServeMode::NotFound {
        return respond(&mut stream, 404, "Not Found", None, b"missing").await;
    }
    if mode == ServeMode::IgnoreRange || range.is_none() {
        return respond(&mut stream, 200, "OK", None, &body).await;
    }

    let (start, end) = range.unwrap();
    let end = end.min(body.len().saturating_sub(1));
    if start > end {
        return respond(&mut stream, 416, "Range Not Satisfiable", None, b"").await;
    }
    let mut slice = body[start..=end].to_vec();
    if mode == ServeMode::Corrupt {
        // Flip every byte, so the data is the right length and hashes wrong.
        for byte in &mut slice {
            *byte = !*byte;
        }
    }
    let header = format!("bytes {start}-{end}/{}", body.len());
    served.fetch_add(slice.len() as u64, Ordering::Relaxed);
    respond(&mut stream, 206, "Partial Content", Some(&header), &slice).await
}

/// Answer one BEP 17 request.
///
/// The whole payload is served from one file on disk, so the piece index and
/// the sub-range inside it are turned back into an absolute offset. That is
/// exactly the mapping a real Hoffman seed does.
async fn serve_hoffman(
    stream: &mut TcpStream,
    root: &Path,
    target: &str,
    served: Served,
) -> std::io::Result<()> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut piece: Option<u64> = None;
    let mut range: Option<(usize, usize)> = None;
    let mut has_info_hash = false;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("piece", value)) => piece = value.parse().ok(),
            Some(("ranges", value)) => range = parse_range(value),
            Some(("info_hash", value)) => has_info_hash = !value.is_empty(),
            _ => {}
        }
    }
    let Ok(body) = std::fs::read(root.join(percent_decode(path.trim_start_matches('/')))) else {
        return respond(stream, 404, "Not Found", None, b"missing").await;
    };
    let (Some(piece), Some((begin, end)), true) = (piece, range, has_info_hash) else {
        return respond(stream, 400, "Bad Request", None, b"not a BEP 17 request").await;
    };

    let piece_length = PIECE_LENGTH as u64;
    let start = (piece * piece_length) as usize + begin;
    let stop = ((piece * piece_length) as usize + end).min(body.len().saturating_sub(1));
    if start > stop {
        return respond(stream, 416, "Range Not Satisfiable", None, b"").await;
    }
    let slice = body[start..=stop].to_vec();
    served.fetch_add(slice.len() as u64, Ordering::Relaxed);
    respond(stream, 200, "OK", None, &slice).await
}

fn parse_range(spec: &str) -> Option<(usize, usize)> {
    let (start, end) = spec.trim().split_once('-')?;
    Some((start.parse().ok()?, end.parse().unwrap_or(usize::MAX)))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn respond(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    content_range: Option<&str>,
    body: &[u8],
) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {code} {reason}\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(range) = content_range {
        head.push_str(&format!("Content-Range: {range}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

/// A session with no discovery at all, so the only source is the bridge.
async fn engine(download_dir: &Path) -> Engine {
    Engine::start(&EngineOptions {
        download_directory: download_dir.to_path_buf(),
        // Port zero means the OS chooses, so tests never collide. Binding
        // loopback rather than the wildcard address keeps the whole test to
        // this machine, and stops a host firewall asking about every fresh
        // test binary.
        listen_ports: 0..=0,
        listen_ip: Some(Ipv4Addr::LOCALHOST.into()),
        enable_dht: false,
        enable_lsd: false,
        enable_trackers: false,
        enable_peers: false,
        ..Default::default()
    })
    .await
    .unwrap()
}

/// A torrent built from `source`, as `.torrent` bytes written to `path`.
///
/// This uses `bit-cli`'s own creator rather than `librqbit`'s. `librqbit`
/// 9.0.0's `create_torrent` appends one extra piece hash when the payload is
/// an exact multiple of the piece length, because its final flush tests
/// `remaining_piece_length > 0` after resetting that counter to a full piece.
/// Fixtures built with it are rejected by any client that checks the piece
/// count, this one included. `TODO/create-seed.md` records the upstream
/// defect.
async fn make_torrent(source: &Path, path: &Path) -> Vec<u8> {
    use bit_cli_core::torrent::create::{CreateOptions, InputFile, create};

    let mut files = Vec::new();
    let (name, multi_file) = match source.is_dir() {
        false => {
            let name = source.file_name().unwrap().to_string_lossy().into_owned();
            files.push(InputFile {
                source: source.to_path_buf(),
                path: name.clone(),
                length: std::fs::metadata(source).unwrap().len(),
            });
            (name, false)
        }
        true => {
            for entry in walk(source) {
                let relative = entry
                    .strip_prefix(source)
                    .unwrap()
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                files.push(InputFile {
                    length: std::fs::metadata(&entry).unwrap().len(),
                    source: entry,
                    path: relative,
                });
            }
            (
                source.file_name().unwrap().to_string_lossy().into_owned(),
                true,
            )
        }
    };

    let created = create(
        files,
        &CreateOptions {
            name,
            multi_file,
            piece_length: Some(PIECE_LENGTH),
            creation_date: None,
            ..Default::default()
        },
        |path| {
            std::fs::File::open(path).map_err(|e| {
                bit_cli_core::error::from_io(e, format!("cannot open {}", path.display()))
            })
        },
    )
    .unwrap();
    std::fs::write(path, &created.bytes).unwrap();
    created.bytes
}

/// Every file under `root`, sorted, so fixtures are deterministic.
fn walk(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        entries.sort();
        for entry in entries {
            match entry.is_dir() {
                true => stack.push(entry),
                false => out.push(entry),
            }
        }
    }
    out.sort();
    out
}

/// Everything one attached run needs to be inspected afterwards.
struct Attached {
    engine: Engine,
    handle: bit_cli_core::engine::Handle,
    statuses: Vec<Arc<BridgeStatus>>,
    fetchers: Vec<Arc<Fetcher>>,
    layout: Arc<Layout>,
}

impl Attached {
    fn finished(&self) -> bool {
        self.handle.stats().finished
    }

    fn served(&self) -> u64 {
        self.statuses.iter().map(|s| s.served_bytes()).sum()
    }

    fn failed(&self) -> bool {
        self.statuses
            .iter()
            .any(|s| s.state() == BridgeState::Failed)
    }

    fn reasons(&self) -> Vec<String> {
        self.statuses.iter().filter_map(|s| s.error()).collect()
    }

    /// Retries across every source, and what status each was spent on.
    fn retries_by_status(&self) -> std::collections::BTreeMap<u16, u64> {
        let mut total = std::collections::BTreeMap::new();
        for fetcher in &self.fetchers {
            for (code, count) in fetcher.stats().retries_by_status() {
                *total.entry(code).or_default() += count;
            }
        }
        total
    }
}

/// Build a torrent from `source`, add it to a fresh session downloading into
/// `download_dir`, and attach one bridge per spec.
async fn attach(
    source: &Path,
    download_dir: &Path,
    torrent_dir: &Path,
    specs: Vec<SourceSpec>,
) -> Attached {
    let torrent_path = torrent_dir.join("fixture.torrent");
    make_torrent(source, &torrent_path).await;

    let engine = engine(download_dir).await;
    let handle = engine
        .add(
            torrent_path.to_str().unwrap(),
            &AddOptions {
                overwrite: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    engine.wait_until_initialized(&handle).await.unwrap();

    let layout = Arc::new(engine.layout(&handle).unwrap());
    let info_hash = handle.info_hash().as_string();
    let set = BindingSet::resolve(&layout, &info_hash, &specs).unwrap();
    let target = engine.bridge_target().unwrap();
    let peer_id = handle.shared().peer_id;

    let mut statuses = Vec::new();
    let mut fetchers = Vec::new();
    for binding in &set.bindings {
        let params =
            BridgeParams::for_binding(target, handle.info_hash(), peer_id, &layout, binding, 4);
        let fetcher = Arc::new(
            Fetcher::new(binding.clone(), layout.clone(), info_hash.clone(), 4, false).unwrap(),
        );
        fetchers.push(fetcher.clone());
        let status = Arc::new(BridgeStatus::default());
        statuses.push(status.clone());
        tokio::spawn(bridge::run(params, fetcher, status));
    }

    Attached {
        engine,
        handle,
        statuses,
        fetchers,
        layout,
    }
}

/// Poll until `check` passes or the timeout expires.
async fn wait_for(timeout: Duration, mut check: impl FnMut() -> bool) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        if check() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    check()
}

/// A source serving the whole torrent from `base`.
fn whole(base: &str) -> SourceSpec {
    SourceSpec::new(base, Origin::CommandLine)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_single_file_torrent_downloads_from_a_web_seed_alone() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(300 * 1024, 7);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "did not complete from HTTP alone: {:?}",
        run.reasons()
    );
    assert_eq!(std::fs::read(out.path().join("movie.bin")).unwrap(), data);
    assert!(
        run.served() > 0,
        "the source should have served the payload"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_multi_file_torrent_downloads_across_file_boundaries() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = src.path().join("album");
    std::fs::create_dir_all(root.join("disc 1")).unwrap();
    // Sizes chosen so pieces straddle both file boundaries.
    let a = content(50 * 1024, 1);
    let b = content(40 * 1024, 2);
    let c = content(60 * 1024, 3);
    std::fs::write(root.join("disc 1").join("one.bin"), &a).unwrap();
    std::fs::write(root.join("disc 1").join("two.bin"), &b).unwrap();
    std::fs::write(root.join("three.bin"), &c).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let run = attach(&root, out.path(), tmp.path(), vec![whole(&base)]).await;

    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "did not complete: {:?}",
        run.reasons()
    );
    let got = out.path().join("album");
    assert_eq!(
        std::fs::read(got.join("disc 1").join("one.bin")).unwrap(),
        a
    );
    assert_eq!(
        std::fs::read(got.join("disc 1").join("two.bin")).unwrap(),
        b
    );
    assert_eq!(std::fs::read(got.join("three.bin")).unwrap(), c);
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_server_that_ignores_range_fails_the_source_instead_of_serving_wrong_bytes() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    // Larger than one window, so at least one request is a real sub-range and
    // a 200 response is unambiguously wrong.
    let data = content(300 * 1024, 11);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::IgnoreRange).await;
    let mut spec = whole(&base);
    spec.limits.chunk_size = 64 * 1024;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![spec],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(30), || run.failed()).await,
        "a server that ignores Range has to fail the source"
    );
    assert!(!run.finished(), "nothing should have completed");
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_missing_file_fails_the_source() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(100 * 1024, 13);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::NotFound).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(30), || run.failed()).await,
        "404 has to fail the source"
    );
    let reasons = run.reasons().join(" ");
    assert!(
        reasons.contains("404"),
        "the reason should name the status: {reasons}"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_payload_is_never_fetched_twice_over() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(300 * 1024, 17);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, served) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "did not complete: {:?}",
        run.reasons()
    );
    let bytes = served.load(Ordering::Relaxed);
    // Some slack for a window that overlaps the tail of the file, but nothing
    // close to a second full pass.
    assert!(
        bytes < (data.len() as u64) * 3 / 2,
        "fetched {bytes} bytes for a {} byte payload",
        data.len()
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_connected_source_reports_active_before_it_is_asked_for_anything() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(64 * 1024, 19);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(30), || {
            run.statuses[0].state() == BridgeState::Active
        })
        .await,
        "a connected and unchoked source is available whether or not it is being asked for anything"
    );
    assert!(
        run.statuses[0].local_port().is_some(),
        "an active bridge has a loopback port"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn corrupt_data_never_completes_the_torrent() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(200 * 1024, 23);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Corrupt).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;

    // Nothing verifies, so nothing completes. The bridge does not hash-check;
    // the session does, which is exactly how a lying peer is handled.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        !run.finished(),
        "a source serving wrong bytes must never complete a torrent"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_scoped_sources_cover_a_torrent_between_them() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    // Ten pieces of 32 KiB.
    let data = content(320 * 1024, 29);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let front = whole(&base).with_scope(Scope::parse("piece:0-4").unwrap());
    let back = whole(&base).with_scope(Scope::parse("piece:5-").unwrap());
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![front, back],
    )
    .await;

    assert_eq!(run.layout.piece_count(), 10);
    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "two partial sources should cover the payload between them: {:?}",
        run.reasons()
    );
    assert_eq!(std::fs::read(out.path().join("movie.bin")).unwrap(), data);
    // Both sources did work, which is what proves the pieces were split rather
    // than one source quietly serving everything.
    for status in &run.statuses {
        assert!(
            status.served_bytes() > 0,
            "both sources should have served something"
        );
    }
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_partial_source_never_completes_a_torrent_on_its_own() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(320 * 1024, 31);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, served) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let half = whole(&base).with_scope(Scope::parse("piece:0-4").unwrap());
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![half],
    )
    .await;

    // The source announces five of ten pieces, so the session can never
    // finish, and it must never ask for a piece outside that set.
    tokio::time::sleep(Duration::from_secs(6)).await;
    assert!(!run.finished());
    assert!(
        !run.failed(),
        "a partial source is not a broken one: {:?}",
        run.reasons()
    );
    let bytes = served.load(Ordering::Relaxed);
    assert!(bytes > 0, "the in-scope half should still have been served");
    assert!(
        bytes <= (data.len() as u64) / 2 + PIECE_LENGTH as u64,
        "served {bytes} bytes, which is more than the scope allows"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_bep_17_source_downloads_a_torrent() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(200 * 1024, 37);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, served) = serve(src.path().to_path_buf(), ServeMode::Hoffman).await;
    // BEP 17 addresses the torrent, not a file, so the URL is the base with
    // nothing appended.
    let mut spec = SourceSpec::new(format!("{base}movie.bin"), Origin::TorrentHttpSeeds);
    spec.style = bit_cli_core::webseed::Style::Hoffman;
    spec.mode = bit_cli_core::webseed::Mode::Exact;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![spec],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "a BEP 17 source should complete a torrent: {:?}",
        run.reasons()
    );
    assert_eq!(std::fs::read(out.path().join("movie.bin")).unwrap(), data);
    assert!(
        served.load(Ordering::Relaxed) > 0,
        "the BEP 17 path served nothing"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_side_verification_names_the_mirror_that_served_a_wrong_piece() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(200 * 1024, 41);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = Arc::new(meta.layout());
    let hashes = Arc::new(meta.info().pieces.clone());

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Corrupt).await;
    let spec = whole(&base);
    let set = BindingSet::resolve(&layout, &meta.info_hash().hex(), &[spec]).unwrap();
    let fetcher = Fetcher::new(
        set.bindings[0].clone(),
        layout.clone(),
        meta.info_hash().hex(),
        4,
        false,
    )
    .unwrap()
    .with_verification(bit_cli_core::webseed::fetch::Verify::Piece, Some(hashes));

    // The window covers whole pieces, so the mismatch is caught at the source
    // rather than several hops later inside the session.
    let err = fetcher.read(0, 16 * 1024).await.unwrap_err();
    assert_eq!(err.class(), "hash_mismatch", "{err}");
    let text = err.to_string();
    assert!(text.contains(&base), "the mirror has to be named: {text}");
    assert!(
        text.contains("piece 0"),
        "the piece has to be named: {text}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webseed_test_reports_range_support_size_and_the_redirect_chain() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(200 * 1024, 43);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let set = BindingSet::resolve(&layout, &meta.info_hash().hex(), &[whole(&base)]).unwrap();
    let report = bit_cli_core::webseed::probe::test_source(
        &set.bindings[0],
        &layout,
        &meta.info_hash().hex(),
        false,
    )
    .await;

    assert!(report.ok, "{:?}", report.error);
    assert_eq!(report.status, Some(206));
    assert_eq!(
        report.range_support,
        bit_cli_core::webseed::probe::RangeSupport::Yes
    );
    assert_eq!(report.content_length, Some(data.len() as u64));
    assert_eq!(report.length_matches, Some(true));
    assert!(report.redirects.is_empty());
    assert!(report.tls.is_none(), "plain HTTP has no TLS to report");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webseed_test_follows_and_reports_every_redirect_hop() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(64 * 1024, 47);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Redirect).await;
    let set = BindingSet::resolve(&layout, &meta.info_hash().hex(), &[whole(&base)]).unwrap();
    let report = bit_cli_core::webseed::probe::test_source(
        &set.bindings[0],
        &layout,
        &meta.info_hash().hex(),
        false,
    )
    .await;

    assert!(report.ok, "{:?}", report.error);
    assert_eq!(
        report.redirects.len(),
        1,
        "the chain has to be reported hop by hop"
    );
    assert_eq!(report.redirects[0].status, 302);
    assert!(
        report.redirects[0].to.contains("/moved/"),
        "{:?}",
        report.redirects[0]
    );
    assert!(
        report.resolved_url.is_some(),
        "the resolved URL is what to request next"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webseed_test_says_no_when_the_server_ignores_range() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(64 * 1024, 53);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::IgnoreRange).await;
    let set = BindingSet::resolve(&layout, &meta.info_hash().hex(), &[whole(&base)]).unwrap();
    let report = bit_cli_core::webseed::probe::test_source(
        &set.bindings[0],
        &layout,
        &meta.info_hash().hex(),
        false,
    )
    .await;

    assert!(!report.ok);
    assert_eq!(
        report.range_support,
        bit_cli_core::webseed::probe::RangeSupport::No
    );
    assert!(report.error.unwrap().contains("Range"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webseed_probe_produces_a_concurrency_curve() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(320 * 1024, 59);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let mut spec = whole(&base);
    spec.limits.chunk_size = 32 * 1024;
    let set = BindingSet::resolve(&layout, &meta.info_hash().hex(), &[spec]).unwrap();
    let report = bit_cli_core::webseed::probe::probe_source(
        &set.bindings[0],
        &layout,
        &meta.info_hash().hex(),
        &[1, 2],
        Duration::from_millis(600),
    )
    .await;

    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.steps.len(), 2, "one step per concurrency");
    for step in &report.steps {
        assert!(
            step.requests > 0,
            "no requests at concurrency {}",
            step.concurrency
        );
        assert_eq!(step.errors, 0, "step {} had errors", step.concurrency);
        assert!(step.bytes > 0);
        assert!(step.p99_ms >= step.p50_ms, "percentiles have to be ordered");
    }
    assert!(report.best_concurrency.is_some());
    assert!(report.best_throughput > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_scope_that_leaves_a_gap_names_the_uncovered_pieces() {
    // No session needed: coverage is decided before any request goes out,
    // which is the point of checking it there.
    let layout = Layout::from_lengths(
        "movie.bin",
        false,
        PIECE_LENGTH,
        [("movie.bin".to_string(), 320 * 1024u64)],
    );
    let spec = whole("https://mirror.example.com/").with_scope(Scope::parse("piece:0-3").unwrap());
    let set = BindingSet::resolve(&layout, &"0".repeat(40), &[spec]).unwrap();

    assert!(!set.is_complete());
    assert_eq!(set.uncovered_pieces, vec![4, 5, 6, 7, 8, 9]);

    // With peers available a gap is fine; without them it is a hard error.
    assert!(set.require_coverage(true).is_ok());
    let err = set.require_coverage(false).unwrap_err();
    assert_eq!(err.code(), bit_cli_core::ExitCode::CoverageGap);
    assert_eq!(
        err.context()["uncovered_pieces"],
        serde_json::json!([4, 5, 6, 7, 8, 9])
    );
}

// -- bench webseed ---------------------------------------------------------
//
// `bench webseed` reads real payload off a real socket and throws it away.
// These drive the whole path against the same loopback server the download
// tests use, so the numbers in a report come from bytes that actually moved.

/// Options for a short bench run: no warmup, a fine sampling interval, and a
/// chunk size small enough that a fraction of a second still issues many
/// requests.
fn bench_options(duration_ms: u64) -> bit_cli_core::bench::webseed::Options {
    bit_cli_core::bench::webseed::Options {
        duration: Duration::from_millis(duration_ms),
        warmup: Duration::ZERO,
        metrics_interval: Duration::from_millis(100),
        concurrency: 4,
        concurrency_sweep: Vec::new(),
        target_rate: None,
        chunk_size: Some(16 * 1024),
    }
}

/// A torrent, a server, and the bindings that join them.
async fn bench_fixture(
    mode: ServeMode,
) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Layout,
    String,
    BindingSet,
) {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("movie.bin"), content(512 * 1024, 71)).unwrap();

    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();
    let info_hash = meta.info_hash().hex();

    let (base, _) = serve(src.path().to_path_buf(), mode).await;
    let mut spec = whole(&base);
    spec.limits.chunk_size = 16 * 1024;
    let set = BindingSet::resolve(&layout, &info_hash, &[spec]).unwrap();
    (src, tmp, layout, info_hash, set)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_moves_real_bytes_and_reports_them() {
    let (_src, _tmp, layout, info_hash, set) = bench_fixture(ServeMode::Ranges).await;
    let mut samples = 0usize;
    let outcome =
        bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &bench_options(700), |_| {
            samples += 1
        })
        .await
        .unwrap();

    assert!(
        outcome.summary.bytes.0 > 0,
        "no bytes moved: {:?}",
        outcome.summary
    );
    assert!(outcome.summary.requests > 0);
    assert_eq!(
        outcome.summary.errors.total, 0,
        "{:?}",
        outcome.summary.errors
    );
    assert!(outcome.summary.sustained_rate.0 > 0);
    assert!(outcome.summary.peak_rate.0 > 0);
    assert!(samples > 0, "the time series was never sampled");
    assert_eq!(outcome.series.len(), samples);

    let complete = &outcome.summary.latency.complete;
    assert!(complete.count > 0);
    assert!(complete.p50_ms <= complete.p90_ms);
    assert!(complete.p90_ms <= complete.p99_ms);
    assert!(complete.p99_ms <= complete.max_ms);
    assert!(
        outcome.summary.latency.first_byte.count > 0,
        "first byte latency is not recorded"
    );
    assert!(
        outcome.summary.latency.connect.count > 0,
        "connection establishment is measured on its own cadence"
    );

    assert_eq!(outcome.sources.len(), 1);
    let source = &outcome.sources[0];
    assert_eq!(
        source.range_support,
        bit_cli_core::webseed::probe::RangeSupport::Yes
    );
    assert_eq!(source.summary.bytes, outcome.summary.bytes);
    assert!(source.failure.is_none());
    assert_eq!(outcome.endpoints.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_series_totals_agree_with_the_summary() {
    let (_src, _tmp, layout, info_hash, set) = bench_fixture(ServeMode::Ranges).await;
    let outcome =
        bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &bench_options(700), |_| {})
            .await
            .unwrap();

    let from_series: u64 = outcome.series.iter().map(|s| s.bytes.0).sum();
    // The series is sampled on the interval and the run stops between ticks,
    // so the last partial interval is in the summary and not yet in a sample.
    assert!(
        from_series <= outcome.summary.bytes.0,
        "the series claims {from_series} bytes but the summary claims {}",
        outcome.summary.bytes.0
    );
    assert!(
        from_series > 0,
        "the series recorded no bytes at all: {:?}",
        outcome.series
    );
    let last = outcome.series.last().unwrap();
    assert_eq!(
        last.cumulative_bytes.0, from_series,
        "the cumulative column is the running total of the interval column"
    );
    for sample in &outcome.series {
        assert!(sample.process.peak_rss_bytes > 0, "no cost was sampled");
        assert!(!sample.warmup, "this run had no warmup window");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_reports_a_concurrency_curve_with_its_own_latency() {
    let (_src, _tmp, layout, info_hash, set) = bench_fixture(ServeMode::Ranges).await;
    let mut options = bench_options(900);
    options.concurrency_sweep = vec![1, 4];
    let outcome = bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &options, |_| {})
        .await
        .unwrap();

    assert_eq!(outcome.concurrency_curve.len(), 2);
    for step in &outcome.concurrency_curve {
        assert!(
            step.requests > 0,
            "concurrency {} issued no request",
            step.concurrency
        );
        assert!(step.bytes.0 > 0);
        assert!(
            step.latency.complete.count > 0,
            "a step carries its own latency, which is what makes a knee visible"
        );
        assert!(step.latency.complete.p99_ms >= step.latency.complete.p50_ms);
    }
    assert_eq!(outcome.concurrency_curve[0].concurrency, 1);
    assert_eq!(outcome.concurrency_curve[1].concurrency, 4);
    assert!(outcome.summary.best_concurrency.is_some());
    let total: u64 = outcome.concurrency_curve.iter().map(|s| s.bytes.0).sum();
    assert_eq!(
        total, outcome.summary.bytes.0,
        "the steps add up to the run"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_names_a_server_that_ignores_range() {
    let (_src, _tmp, layout, info_hash, set) = bench_fixture(ServeMode::IgnoreRange).await;
    let outcome =
        bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &bench_options(500), |_| {})
            .await
            .unwrap();

    assert_eq!(
        outcome.summary.bytes.0, 0,
        "a server that ignores Range serves no usable byte"
    );
    assert!(outcome.summary.errors.total > 0);
    assert_eq!(
        outcome
            .summary
            .errors
            .by_class
            .get("range_ignored")
            .copied(),
        Some(outcome.summary.errors.total)
    );
    assert_eq!(
        outcome.summary.errors.by_status.get("200").copied(),
        Some(outcome.summary.errors.total)
    );
    assert_eq!(
        outcome.sources[0].range_support,
        bit_cli_core::webseed::probe::RangeSupport::No
    );
    assert!(
        outcome
            .notes
            .iter()
            .any(|note| note.contains("does not honour Range")),
        "{:?}",
        outcome.notes
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_counts_a_404_by_class_and_by_status() {
    let (_src, _tmp, layout, info_hash, set) = bench_fixture(ServeMode::NotFound).await;
    let outcome =
        bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &bench_options(500), |_| {})
            .await
            .unwrap();

    assert_eq!(outcome.summary.bytes.0, 0);
    assert!(outcome.summary.errors.total > 0);
    assert_eq!(
        outcome.summary.errors.by_class.get("not_found").copied(),
        Some(outcome.summary.errors.total)
    );
    assert_eq!(
        outcome.summary.errors.by_status.get("404").copied(),
        Some(outcome.summary.errors.total)
    );
    assert!(
        outcome.summary.latency.complete.count > 0,
        "the timing of a failing request is still a measurement"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_honours_a_target_rate() {
    let (_src, _tmp, layout, info_hash, set) = bench_fixture(ServeMode::Ranges).await;
    let mut options = bench_options(1500);
    // Loopback serves far faster than this, so the pacer has to hold it down
    // or the flag does nothing.
    options.target_rate = Some(64 * 1024);
    let outcome = bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &options, |_| {})
        .await
        .unwrap();

    assert!(outcome.summary.bytes.0 > 0);
    assert!(
        outcome.summary.sustained_rate.0 <= 4 * 64 * 1024,
        "asked for 64 KiB/s and got {} B/s",
        outcome.summary.sustained_rate.0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_measures_only_what_a_scope_covers() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("album")).unwrap();
    for (name, seed) in [("a.bin", 11u64), ("b.bin", 12)] {
        std::fs::write(
            src.path().join("album").join(name),
            content(256 * 1024, seed),
        )
        .unwrap();
    }
    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("album"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();
    let info_hash = meta.info_hash().hex();

    let (base, served) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let mut spec = whole(&base).with_scope(Scope::parse("0").unwrap());
    spec.limits.chunk_size = 16 * 1024;
    let set = BindingSet::resolve(&layout, &info_hash, &[spec]).unwrap();

    let outcome =
        bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &bench_options(600), |_| {})
            .await
            .unwrap();

    assert!(outcome.summary.bytes.0 > 0);
    assert_eq!(outcome.summary.errors.total, 0);
    assert!(served.load(Ordering::Relaxed) > 0);
    assert!(
        outcome.endpoints[0].ends_with("a.bin"),
        "a scope of file 0 reads file 0: {}",
        outcome.endpoints[0]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_webseed_keeps_a_broken_mirror_apart_from_a_healthy_one() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("movie.bin"), content(256 * 1024, 83)).unwrap();
    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();
    let info_hash = meta.info_hash().hex();

    let (good, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let (bad, _) = serve(src.path().to_path_buf(), ServeMode::NotFound).await;
    let specs: Vec<SourceSpec> = [good, bad]
        .iter()
        .map(|base| {
            let mut spec = whole(base);
            spec.limits.chunk_size = 16 * 1024;
            spec
        })
        .collect();
    let set = BindingSet::resolve(&layout, &info_hash, &specs).unwrap();

    let mut options = bench_options(700);
    options.concurrency = 2;
    let outcome = bit_cli_core::bench::webseed::run(&set, &layout, &info_hash, &options, |_| {})
        .await
        .unwrap();

    assert_eq!(outcome.sources.len(), 2, "one row per source");
    let healthy = &outcome.sources[0];
    let broken = &outcome.sources[1];
    assert!(healthy.summary.bytes.0 > 0);
    assert_eq!(healthy.summary.errors, 0);
    assert_eq!(broken.summary.bytes.0, 0);
    assert!(broken.summary.errors > 0);
    assert_eq!(
        broken
            .summary
            .error_detail
            .as_ref()
            .unwrap()
            .by_status
            .get("404")
            .copied(),
        Some(broken.summary.errors),
        "the failing mirror is visible rather than averaged away"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_sources_are_probed_in_parallel_and_every_one_is_reported() {
    // A real torrent carries hundreds of web seeds: the Arch Linux ISO torrent
    // carries 468. Probing them one at a time takes minutes, so they are
    // probed in parallel. What has to hold is that every declared source comes
    // back, in the order it was declared, with its own result.
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("movie.bin"), content(64 * 1024, 91)).unwrap();
    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = meta.layout();
    let info_hash = meta.info_hash().hex();

    // Half the sources answer and half return 404, so the results cannot be
    // told apart by anything except which source produced them.
    let (good, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let (bad, _) = serve(src.path().to_path_buf(), ServeMode::NotFound).await;
    let specs: Vec<SourceSpec> = (0..32)
        .map(|index| match index % 2 {
            0 => whole(&good),
            _ => whole(&bad),
        })
        .collect();
    let set = BindingSet::resolve(&layout, &info_hash, &specs).unwrap();

    let mut workers = tokio::task::JoinSet::new();
    for (index, binding) in set.bindings.iter().enumerate() {
        let binding = binding.clone();
        let layout = layout.clone();
        let info_hash = info_hash.clone();
        workers.spawn(async move {
            (
                index,
                bit_cli_core::webseed::probe::test_source(&binding, &layout, &info_hash, false)
                    .await,
            )
        });
    }
    let mut results: Vec<Option<bit_cli_core::webseed::probe::SourceTest>> = vec![None; 32];
    while let Some(Ok((index, result))) = workers.join_next().await {
        results[index] = Some(result);
    }

    for (index, result) in results.iter().enumerate() {
        let result = result.as_ref().unwrap_or_else(|| panic!("source {index}"));
        assert_eq!(
            result.index, index,
            "a result landed under the wrong source"
        );
        match index % 2 {
            0 => {
                assert!(result.ok, "source {index} should be usable: {result:?}");
                assert_eq!(result.status, Some(206));
            }
            _ => {
                assert!(!result.ok, "source {index} should be unusable");
                assert_eq!(result.status, Some(404));
            }
        }
    }
}

/// A hash check that has not finished reports the phase rather than a bare
/// deadline.
///
/// Upstream reports roughly one add in twenty of a torrent with existing files
/// sticking at "checking files" and never leaving. A run bounded only by
/// `--timeout` survives that but reports a deadline with no reason attached,
/// so `--init-timeout` fires first and the error names the phase, how far the
/// check had got, and how long it waited.
///
/// The hang is simulated by a deadline shorter than the check rather than by a
/// stuck volume: what is under test is that the wait is bounded and that the
/// error says what was happening, and both are the same either way. See
/// `TODO/disk-io.md`, T-015.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_hash_check_that_has_not_finished_names_the_phase_it_is_in() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    // Large enough that hashing it cannot finish inside one poll.
    std::fs::write(src.path().join("movie.bin"), content(64 * 1024 * 1024, 17)).unwrap();
    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;

    let engine = engine(src.path()).await;
    let handle = engine
        .add(
            torrent_path.to_str().unwrap(),
            &AddOptions {
                // The payload is already there, so adding it means hashing it.
                overwrite: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let began = std::time::Instant::now();
    let error = engine
        .wait_until_initialized_within(&handle, Duration::from_millis(1))
        .await
        .expect_err("a 64 MiB hash check does not finish in a millisecond");

    assert_eq!(error.code(), bit_cli_core::ExitCode::Timeout);
    assert_eq!(error.context()["phase"], "initializing");
    assert_eq!(error.context()["waited_ms"], 1);
    assert!(error.context().contains_key("checked_percent"));
    assert!(error.context().contains_key("total_bytes"));
    assert!(error.message().contains("still initializing"), "{error}");
    assert!(error.message().contains("hash-checked"), "{error}");
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "the deadline did not bound the wait: {:?}",
        began.elapsed()
    );

    // Without a deadline the same wait finishes, so the timeout is the only
    // thing that ended it.
    engine.wait_until_initialized(&handle).await.unwrap();
    engine.stop().await;
}

/// Storage counts what a download actually did on disk.
///
/// The three numbers `bench leech` separates cost from throughput all come
/// from here: the piece checks, the writes underneath them, and the reads the
/// checks perform. Nothing else in the process can report them, because the
/// session does the hashing and only storage sees the I/O it takes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_download_reports_its_reads_writes_and_piece_checks() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(512 * 1024, 31);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;
    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "did not complete: {:?}",
        run.reasons()
    );

    let counts = run.engine.storage_counts();
    let pieces = run.layout.piece_count() as u64;

    assert_eq!(
        counts.verify_pieces, pieces,
        "every piece is read back and hashed once"
    );
    assert_eq!(
        counts.verify_bytes,
        data.len() as u64,
        "a check reads exactly the piece it is checking"
    );
    assert!(
        counts.verify_nanos > 0,
        "a check that read {} bytes took no time",
        counts.verify_bytes
    );
    assert_eq!(
        counts.write_bytes,
        data.len() as u64,
        "the payload is written once"
    );
    assert!(counts.write_ops >= pieces, "{} writes", counts.write_ops);
    assert!(
        counts.read_bytes >= counts.verify_bytes,
        "the checks' reads are part of the reads"
    );
    run.engine.stop().await;
}

/// The bridge reports the session's request window from the other end.
///
/// A peer answers a bounded number of block requests at a time, and that
/// bound is what caps throughput when the link is faster than the pipeline.
/// The bridge is the only place `bit-cli` can see it, because it is the thing
/// being asked.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_bridge_reports_how_many_blocks_the_session_keeps_outstanding() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(2 * 1024 * 1024, 11);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::Ranges).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;
    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "did not complete: {:?}",
        run.reasons()
    );

    let pipeline = run.statuses[0].pipeline();
    assert!(pipeline.requests > 0, "the session asked for nothing");
    assert_eq!(
        pipeline.blocks, pipeline.requests,
        "every request was answered"
    );
    assert!(
        pipeline.peak_in_flight > 1,
        "the session pipelined nothing: peak depth {}",
        pipeline.peak_in_flight
    );
    assert_eq!(
        pipeline.in_flight, 0,
        "nothing is outstanding once the transfer is done"
    );
    assert!(
        pipeline.mean_service_us().is_some_and(|us| us > 0),
        "blocks were answered in no measurable time"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_403_retires_a_source_when_the_caller_has_said_nothing() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(200 * 1024, 41);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::ExpiringSignature).await;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![whole(&base)],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(30), || run.failed()).await,
        "403 is permanent by default, so the source has to give up"
    );
    assert!(!run.finished(), "nothing should have completed");
    let reasons = run.reasons().join(" ");
    assert!(
        reasons.contains("403"),
        "the reason names the status: {reasons}"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_403_the_caller_calls_retryable_completes_the_torrent() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(200 * 1024, 41);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    let (base, _) = serve(src.path().to_path_buf(), ServeMode::ExpiringSignature).await;
    let mut spec = whole(&base);
    spec.limits.retry_status = bit_cli_core::webseed::binding::StatusSet::parse("403").unwrap();
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![spec],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(120), || run.finished()).await,
        "did not complete: {:?}",
        run.reasons()
    );
    assert_eq!(
        std::fs::read(out.path().join("movie.bin")).unwrap(),
        data,
        "the payload has to be byte for byte the source"
    );
    // Every distinct range was refused once, so the retries are the proof the
    // 403s happened rather than the server having quietly served them.
    let by_status = run.retries_by_status();
    assert!(
        by_status.get(&403).copied().unwrap_or(0) > 0,
        "no retry was charged to 403: {by_status:?}"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_404_the_caller_calls_retryable_is_still_bounded_by_the_retry_count() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let data = content(100 * 1024, 43);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    // A mirror that answers 404 forever. Calling it retryable does not make
    // it recover, and the run has to end rather than loop.
    let (base, _) = serve(src.path().to_path_buf(), ServeMode::NotFound).await;
    let mut spec = whole(&base);
    spec.limits.retry_status = bit_cli_core::webseed::binding::StatusSet::parse("404").unwrap();
    spec.limits.retries = 1;
    spec.limits.max_errors = 1;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![spec],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(60), || run.failed()).await,
        "a source that never recovers has to retire even when its status is retryable"
    );
    assert!(!run.finished());
    assert!(
        run.retries_by_status().get(&404).copied().unwrap_or(0) > 0,
        "the retry should have been charged to 404"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_file_source_completes_a_torrent_with_no_server_at_all() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let data = content(200 * 1024, 61);
    std::fs::write(src.path().join("movie.bin"), &data).unwrap();

    // The same bytes under a name and a directory the torrent knows nothing
    // about, which is the case this exists for.
    let copy = elsewhere.path().join("a3f1-blob.dat");
    std::fs::write(&copy, &data).unwrap();

    let mut spec = SourceSpec::new(
        bit_cli_core::webseed::local::url_of(&copy),
        Origin::CommandLine,
    );
    spec.mode = bit_cli_core::webseed::Mode::Exact;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![spec],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "did not complete: {:?}",
        run.reasons()
    );
    assert_eq!(
        std::fs::read(out.path().join("movie.bin")).unwrap(),
        data,
        "the payload has to be byte for byte the source"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_file_source_holding_the_wrong_bytes_is_caught_at_the_source() {
    let src = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("movie.bin"), content(200 * 1024, 63)).unwrap();

    // The right length and the wrong bytes: the case a length check misses
    // and the per-piece check is for.
    let wrong = elsewhere.path().join("not-it.dat");
    std::fs::write(&wrong, content(200 * 1024, 64)).unwrap();

    let torrent_path = tmp.path().join("fixture.torrent");
    make_torrent(&src.path().join("movie.bin"), &torrent_path).await;
    let meta = bit_cli_core::torrent::Metainfo::read(&torrent_path).unwrap();
    let layout = Arc::new(meta.layout());
    let hashes = Arc::new(meta.info().pieces.clone());

    let mut spec = SourceSpec::new(
        bit_cli_core::webseed::local::url_of(&wrong),
        Origin::CommandLine,
    );
    spec.mode = bit_cli_core::webseed::Mode::Exact;
    let set = BindingSet::resolve(&layout, &meta.info_hash().hex(), &[spec]).unwrap();
    let fetcher = Fetcher::new(
        set.bindings[0].clone(),
        layout.clone(),
        meta.info_hash().hex(),
        4,
        false,
    )
    .unwrap()
    .with_verification(bit_cli_core::webseed::fetch::Verify::Piece, Some(hashes));

    let err = fetcher.read(0, 16 * 1024).await.unwrap_err();
    assert_eq!(err.class(), "hash_mismatch", "{err}");
    let text = err.to_string();
    assert!(
        text.contains("not-it.dat"),
        "the path has to be named: {text}"
    );
    assert!(
        text.contains("piece 0"),
        "the piece has to be named: {text}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_file_source_that_is_not_there_fails_the_source_by_name() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("movie.bin"), content(100 * 1024, 65)).unwrap();

    let mut spec = SourceSpec::new(
        bit_cli_core::webseed::local::url_of(&elsewhere.path().join("gone.dat")),
        Origin::CommandLine,
    );
    spec.mode = bit_cli_core::webseed::Mode::Exact;
    let run = attach(
        &src.path().join("movie.bin"),
        out.path(),
        tmp.path(),
        vec![spec],
    )
    .await;

    assert!(
        wait_for(Duration::from_secs(30), || run.failed()).await,
        "a path that is not there has to fail the source"
    );
    let reasons = run.reasons().join(" ");
    assert!(
        reasons.contains("gone.dat") && reasons.contains("no such file"),
        "the reason should name the path: {reasons}"
    );
    run.engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_file_source_composes_a_directory_the_way_an_http_one_does() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    // A multi-file torrent, so the composition has both a name and a path to
    // append. `auto` against a directory is what "I already have this tree"
    // looks like.
    let tree = src.path().join("album");
    std::fs::create_dir_all(tree.join("disc 1")).unwrap();
    let first = content(90 * 1024, 67);
    let second = content(40 * 1024, 68);
    std::fs::write(tree.join("disc 1/a.flac"), &first).unwrap();
    std::fs::write(tree.join("notes.nfo"), &second).unwrap();

    // The base is the directory holding `album`, so `auto` composes
    // `<base>/album/disc 1/a.flac`, space and all.
    let spec = SourceSpec::new(
        bit_cli_core::webseed::local::url_of(src.path()),
        Origin::CommandLine,
    );
    let run = attach(&tree, out.path(), tmp.path(), vec![spec]).await;

    assert!(
        wait_for(Duration::from_secs(60), || run.finished()).await,
        "did not complete: {:?}",
        run.reasons()
    );
    assert_eq!(
        std::fs::read(out.path().join("album/disc 1/a.flac")).unwrap(),
        first
    );
    assert_eq!(
        std::fs::read(out.path().join("album/notes.nfo")).unwrap(),
        second
    );
    run.engine.stop().await;
}
