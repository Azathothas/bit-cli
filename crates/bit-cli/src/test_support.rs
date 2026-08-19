//! Fixtures and helpers for driving the whole binary in-process.
//!
//! Every test here runs the same code path a shell would, through
//! [`crate::run`], with no terminal attached. That is how the headless parity
//! requirement is checked rather than assumed.

#![cfg(test)]

use std::path::{Path, PathBuf};

use bit_cli_core::ExitCode;
use bit_cli_core::torrent::create::{CreateOptions, InputFile, create};
use bit_cli_core::torrent::{Lint, Metainfo};

use crate::env::Env;

/// A real `.torrent` and its payload, on disk in a temporary directory.
pub struct TorrentFixture {
    /// Kept so the directory outlives the fixture.
    _temp: tempfile::TempDir,
    /// The directory everything lives in.
    pub root: PathBuf,
    /// The `.torrent` path.
    pub torrent: PathBuf,
    /// Its info hash, lower-case hex.
    pub info_hash: String,
    /// Payload files, as `(relative path, bytes)`.
    pub files: Vec<(String, Vec<u8>)>,
}

impl TorrentFixture {
    /// A two-file torrent: `disc 1/a.flac` (1500 bytes) and `notes.nfo` (500),
    /// with a 1024 byte piece length, so two pieces and a boundary that falls
    /// inside a file.
    pub fn multi_file() -> Self {
        Self::build(
            "album",
            true,
            &[
                ("disc 1/a.flac", 1500usize, 0xAAu8),
                ("notes.nfo", 500, 0xBB),
            ],
        )
    }

    /// A one-file torrent: `payload.bin`, 3000 bytes, 1024 byte pieces.
    pub fn single_file() -> Self {
        Self::build("payload.bin", false, &[("payload.bin", 3000usize, 0xCCu8)])
    }

    fn build(name: &str, multi_file: bool, spec: &[(&str, usize, u8)]) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();

        let mut inputs = Vec::new();
        let mut files = Vec::new();
        for (path, length, fill) in spec {
            let bytes = vec![*fill; *length];
            let on_disk = root.join("payload").join(path);
            std::fs::create_dir_all(on_disk.parent().expect("parent")).expect("mkdir");
            std::fs::write(&on_disk, &bytes).expect("write payload");
            inputs.push(InputFile {
                source: on_disk,
                path: path.to_string(),
                length: *length as u64,
            });
            files.push((path.to_string(), bytes));
        }

        let options = CreateOptions {
            name: name.to_string(),
            multi_file,
            piece_length: Some(1024),
            announce_tiers: vec![vec!["udp://tracker.example.com:80".to_string()]],
            web_seeds: vec!["https://mirror.example.com/pub/".to_string()],
            created_by: None,
            creation_date: None,
            allowed_lints: Lint::ALL.iter().copied().collect(),
            ..Default::default()
        };
        let created = create(inputs, &options, |path: &Path| {
            std::fs::File::open(path)
                .map_err(|e| bit_cli_core::error::from_io(e, format!("open {}", path.display())))
        })
        .expect("create the fixture torrent");

        let torrent = root.join(format!("{name}.torrent"));
        std::fs::write(&torrent, &created.bytes).expect("write torrent");
        let info_hash = Metainfo::parse(&created.bytes)
            .expect("parse")
            .info_hash()
            .hex();

        Self {
            _temp: temp,
            root,
            torrent,
            info_hash,
            files,
        }
    }

    /// The `.torrent` path, as an argument.
    pub fn path_str(&self) -> &str {
        self.torrent.to_str().expect("utf-8 path")
    }

    /// The directory to run commands from.
    pub fn dir(&self) -> PathBuf {
        self.root.clone()
    }

    /// Where the payload lives.
    pub fn payload_dir(&self) -> PathBuf {
        self.root.join("payload")
    }
}

/// Run the binary in-process and require success, returning stdout.
pub fn run_ok(args: &[&str], cwd: impl Into<PathBuf>) -> String {
    let (mut env, captured) = Env::test(args, cwd);
    let code = crate::run(&mut env);
    assert_eq!(
        code,
        ExitCode::Success,
        "`bit-cli {}` exited {code}\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        captured.out(),
        captured.err()
    );
    captured.out()
}

/// Run the binary in-process and require a specific failure code.
pub fn run_err(args: &[&str], cwd: impl Into<PathBuf>, expected: ExitCode) -> String {
    let (mut env, captured) = Env::test(args, cwd);
    let code = crate::run(&mut env);
    assert_eq!(
        code,
        expected,
        "`bit-cli {}` exited {code}, expected {expected}\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        captured.out(),
        captured.err()
    );
    captured.err()
}

/// Run the binary and return stdout parsed as JSON.
pub fn run_json(args: &[&str], cwd: impl Into<PathBuf>) -> serde_json::Value {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let (mut env, captured) = Env::test(&full, cwd);
    let code = crate::run(&mut env);
    assert_eq!(
        code,
        ExitCode::Success,
        "`bit-cli {}` exited {code}\nstderr:\n{}",
        full.join(" "),
        captured.err()
    );
    captured
        .json()
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n{}", captured.out()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_multi_file_fixture_is_what_it_claims_to_be() {
        let fixture = TorrentFixture::multi_file();
        let meta = Metainfo::read(&fixture.torrent).unwrap();
        assert_eq!(meta.info().name, "album");
        assert!(meta.info().multi_file);
        assert_eq!(meta.info().total_length(), 2000);
        assert_eq!(meta.info().piece_length, 1024);
        assert_eq!(meta.info().pieces.len(), 2);
        assert_eq!(meta.info_hash().hex(), fixture.info_hash);
    }

    #[test]
    fn the_single_file_fixture_is_what_it_claims_to_be() {
        let fixture = TorrentFixture::single_file();
        let meta = Metainfo::read(&fixture.torrent).unwrap();
        assert!(!meta.info().multi_file);
        assert_eq!(meta.info().total_length(), 3000);
        assert_eq!(meta.info().pieces.len(), 3);
    }

    #[test]
    fn the_fixture_payload_is_on_disk_and_matches_the_torrent() {
        let fixture = TorrentFixture::multi_file();
        for (path, bytes) in &fixture.files {
            let on_disk = std::fs::read(fixture.payload_dir().join(path)).unwrap();
            assert_eq!(&on_disk, bytes, "{path} does not match");
        }
    }

    #[test]
    fn the_fixture_is_deterministic() {
        let one = TorrentFixture::multi_file();
        let other = TorrentFixture::multi_file();
        assert_eq!(one.info_hash, other.info_hash);
        assert_eq!(
            std::fs::read(&one.torrent).unwrap(),
            std::fs::read(&other.torrent).unwrap()
        );
    }
}
