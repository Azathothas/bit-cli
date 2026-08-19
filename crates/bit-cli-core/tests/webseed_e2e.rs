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
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let root = root.clone();
            let counter = counter.clone();
            tokio::spawn(async move {
                let _ = handle_request(stream, root, mode, counter).await;
            });
        }
    });
    (format!("http://127.0.0.1:{port}/"), served)
}

async fn handle_request(
    mut stream: TcpStream,
    root: PathBuf,
    mode: ServeMode,
    served: Served,
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
    for binding in &set.bindings {
        let params =
            BridgeParams::for_binding(target, handle.info_hash(), peer_id, &layout, binding, 4);
        let fetcher = Arc::new(
            Fetcher::new(binding.clone(), layout.clone(), info_hash.clone(), 4, false).unwrap(),
        );
        let status = Arc::new(BridgeStatus::default());
        statuses.push(status.clone());
        tokio::spawn(bridge::run(params, fetcher, status));
    }

    Attached {
        engine,
        handle,
        statuses,
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
