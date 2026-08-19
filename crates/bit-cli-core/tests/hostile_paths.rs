//! A torrent is untrusted input, and its file names decide where bytes land.
//!
//! These tests drive a real session with a torrent whose paths are hostile,
//! illegal, or colliding, and assert that every file lands inside the output
//! directory under a name the filesystem accepts. They cover `TODO/windows.md`
//! T-071 and T-072.
//!
//! The fixtures are built here rather than committed as `.torrent` files
//! because several of them cannot be checked out on Windows: a repository
//! cannot contain a directory called `C:` or a file called `CON.txt`, which is
//! the whole point. The bencode is written by hand so the paths reach the
//! parser exactly as a hostile torrent would carry them.
//!
//! Nothing here touches the network. The session binds loopback and no peer or
//! tracker is configured, so the torrent is added, its storage is created, and
//! that is the whole run.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use bit_cli_core::engine::{AddOptions, Engine, EngineOptions};
use bit_cli_core::paths::{PathPlan, Reason};
use bit_cli_core::torrent::bencode::{Value, encode};
use sha1::{Digest, Sha1};

const PIECE_LENGTH: usize = 16 * 1024;

/// Deterministic bytes, so the fixture has real piece hashes.
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

/// Build a multi-file `.torrent` whose paths are exactly `paths`.
///
/// No sanitising, no linting, no validation beyond what the parser does. This
/// is the adversary's side of the wire.
fn hostile_torrent(name: &str, paths: &[&str], each: usize) -> Vec<u8> {
    let mut payload = Vec::new();
    let mut files = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        payload.extend_from_slice(&content(each, index as u64 + 1));
        let components: Vec<Value> = path
            .split('/')
            .map(|c| Value::Bytes(c.as_bytes().to_vec()))
            .collect();
        files.push(Value::Dict(BTreeMap::from([
            (b"length".to_vec(), Value::Int(each as i64)),
            (b"path".to_vec(), Value::List(components)),
        ])));
    }

    let mut pieces = Vec::new();
    for chunk in payload.chunks(PIECE_LENGTH) {
        pieces.extend_from_slice(&Sha1::digest(chunk));
    }

    let info = Value::Dict(BTreeMap::from([
        (b"files".to_vec(), Value::List(files)),
        (b"name".to_vec(), Value::Bytes(name.as_bytes().to_vec())),
        (b"piece length".to_vec(), Value::Int(PIECE_LENGTH as i64)),
        (b"pieces".to_vec(), Value::Bytes(pieces)),
    ]));
    encode(&Value::Dict(BTreeMap::from([(b"info".to_vec(), info)])))
}

/// What one run of a hostile torrent produced.
struct Run {
    /// Held so the directories outlive the assertions.
    out: tempfile::TempDir,
    _meta: tempfile::TempDir,
    plan: PathPlan,
}

impl Run {
    fn root(&self) -> &Path {
        self.out.path()
    }
}

/// Add a torrent to a session that can reach nothing, and report where its
/// files were planned and what landed on disk.
///
/// The payload is not on disk, so the hash check finds nothing. The files are
/// still created by the storage, which is what these tests are about.
async fn add(torrent: &[u8]) -> Run {
    let out = tempfile::tempdir().unwrap();
    let meta = tempfile::tempdir().unwrap();
    let path = meta.path().join("hostile.torrent");
    std::fs::write(&path, torrent).unwrap();

    let engine = Engine::start(&EngineOptions {
        download_directory: out.path().to_path_buf(),
        // An OS-chosen port. The default range is nine ports wide and these
        // tests run concurrently, so a fixed range makes them race for it.
        listen_ports: 0..=0,
        listen_ip: Some(std::net::Ipv4Addr::LOCALHOST.into()),
        enable_dht: false,
        enable_lsd: false,
        enable_trackers: false,
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = engine
        .add(
            &path.display().to_string(),
            &AddOptions {
                overwrite: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let _ = engine.wait_until_initialized(&handle).await;
    let plan = engine.path_plan(&handle).expect("storage produced a plan");
    engine.stop().await;

    Run {
        out,
        _meta: meta,
        plan,
    }
}

/// Every file that exists under `root`, as `/`-separated relative paths.
fn tree(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    out.sort();
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_drive_letter_path_cannot_write_outside_the_output_directory() {
    // The naive join is the bug this exists to prevent. On Windows it leaves
    // the output directory entirely; the assertion below states the platform
    // behaviour that makes the rest of the test necessary.
    let naive = Path::new("D:").join("out").join("C:");
    if cfg!(windows) {
        assert!(
            !naive.starts_with(Path::new("D:").join("out")),
            "joining `C:` no longer escapes, so this fixture is stale"
        );
    }

    let torrent = hostile_torrent("album", &["C:/pwned.txt", "safe.bin"], 1024);
    let run = add(&torrent).await;
    let disk = run.plan.disk_paths.clone();

    assert_eq!(disk, ["C_/pwned.txt", "safe.bin"]);
    let landed = tree(run.root());
    assert_eq!(landed, ["album/C_/pwned.txt", "album/safe.bin"]);
    for relative in &landed {
        let full = run.root().join(relative);
        assert!(full.starts_with(run.root()), "{} escaped", full.display());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reserved_device_names_are_written_as_ordinary_files() {
    let torrent = hostile_torrent("album", &["CON.txt", "sub/NUL", "lpt9.log", "ok.bin"], 1024);
    let run = add(&torrent).await;
    let disk = run.plan.disk_paths.clone();

    assert_eq!(disk, ["CON_.txt", "sub/NUL_", "lpt9_.log", "ok.bin"]);
    assert_eq!(
        tree(run.root()),
        [
            "album/CON_.txt",
            "album/lpt9_.log",
            "album/ok.bin",
            "album/sub/NUL_"
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn case_colliding_paths_both_land_and_neither_is_lost() {
    let torrent = hostile_torrent("album", &["README", "readme", "ReadMe"], 1024);
    let run = add(&torrent).await;
    let disk = run.plan.disk_paths.clone();

    assert_eq!(disk, ["README", "readme-1", "ReadMe-2"]);
    // Three distinct files on disk. Without the plan this is one file written
    // three times on NTFS and APFS, and the first two payloads are gone.
    assert_eq!(tree(run.root()).len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn characters_ntfs_refuses_do_not_fail_the_download() {
    let torrent = hostile_torrent("album", &["a<b>c|d?e*f.bin", "x .", "ok.bin"], 1024);
    let run = add(&torrent).await;
    let disk = run.plan.disk_paths.clone();

    assert_eq!(disk, ["a_b_c_d_e_f.bin", "x", "ok.bin"]);
    assert_eq!(tree(run.root()).len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_torrent_named_after_a_device_still_unpacks() {
    // The directory a multi-file torrent unpacks into comes from its `name`,
    // which is no more trustworthy than the file names under it.
    let torrent = hostile_torrent("CON", &["a.bin", "b.bin"], 1024);
    let run = add(&torrent).await;

    assert_eq!(tree(run.root()), ["CON_/a.bin", "CON_/b.bin"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ordinary_torrent_is_written_exactly_where_it_says() {
    let torrent = hostile_torrent("album", &["disc 1/a.flac", "notes.nfo"], 1024);
    let run = add(&torrent).await;
    let disk = run.plan.disk_paths.clone();

    assert_eq!(disk, ["disc 1/a.flac", "notes.nfo"]);
    assert_eq!(tree(run.root()), ["album/disc 1/a.flac", "album/notes.nfo"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_plan_names_every_change_and_its_reason() {
    let torrent = hostile_torrent("album", &["C:/x.bin", "CON.txt", "ok.bin"], 1024);
    let out = tempfile::tempdir().unwrap();
    let meta = tempfile::tempdir().unwrap();
    let path = meta.path().join("hostile.torrent");
    std::fs::write(&path, &torrent).unwrap();

    let engine = Engine::start(&EngineOptions {
        download_directory: out.path().to_path_buf(),
        // An OS-chosen port. The default range is nine ports wide and these
        // tests run concurrently, so a fixed range makes them race for it.
        listen_ports: 0..=0,
        listen_ip: Some(std::net::Ipv4Addr::LOCALHOST.into()),
        enable_dht: false,
        enable_lsd: false,
        enable_trackers: false,
        ..Default::default()
    })
    .await
    .unwrap();
    let handle = engine
        .add(
            &path.display().to_string(),
            &AddOptions {
                overwrite: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let _ = engine.wait_until_initialized(&handle).await;

    let plan = engine.path_plan(&handle).unwrap();
    assert!(!plan.is_clean());
    // Only the files that changed are reported, so a caller can test for an
    // empty list on the ordinary torrent.
    assert_eq!(plan.renames.len(), 2);
    assert_eq!(plan.renames[0].index, 0);
    assert_eq!(plan.renames[0].torrent_path, "C:/x.bin");
    assert_eq!(plan.renames[0].disk_path, "C_/x.bin");
    assert!(plan.renames[0].reasons.contains(&Reason::Escape));
    assert_eq!(plan.renames[1].torrent_path, "CON.txt");
    assert!(plan.renames[1].reasons.contains(&Reason::ReservedName));
    engine.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_written_path_is_a_plain_relative_path() {
    let torrent = hostile_torrent(
        "album",
        &[
            "C:/a.bin",
            "CON",
            "a<b.bin",
            "x .",
            "README",
            "readme",
            "deep/deeper/c.bin",
        ],
        1024,
    );
    let run = add(&torrent).await;
    let disk = run.plan.disk_paths.clone();

    for relative in &disk {
        let path = Path::new(relative);
        assert!(path.is_relative(), "{relative} is not relative");
        for component in path.components() {
            assert!(
                matches!(component, Component::Normal(_)),
                "{relative} carries {component:?}"
            );
        }
    }
    // Nothing was dropped: one file on disk per file in the torrent.
    assert_eq!(tree(run.root()).len(), disk.len());
}
