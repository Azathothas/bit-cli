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

    /// A torrent whose paths cannot be written as given: a drive component
    /// that escapes the output directory, a reserved Windows device name,
    /// characters NTFS refuses, a name Windows strips to another, and a pair
    /// that collides on a case-insensitive filesystem.
    ///
    /// The bencode is written by hand because `create` refuses all of this,
    /// which is correct on the creating side and exactly the input a hostile
    /// torrent carries on the reading side. No payload is written: the fixture
    /// exists to be added, not completed.
    pub fn hostile() -> Self {
        use std::collections::BTreeMap;

        use bit_cli_core::torrent::bencode::{Value, encode};
        use sha1::{Digest, Sha1};

        const PIECE_LENGTH: usize = 1024;
        let paths = [
            "C:/pwned.txt",
            "CON.txt",
            "a<b.bin",
            "x .",
            "README",
            "readme",
        ];

        let mut payload = Vec::new();
        let mut files = Vec::new();
        let mut recorded = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let bytes = vec![index as u8 + 1; 500];
            payload.extend_from_slice(&bytes);
            files.push(Value::Dict(BTreeMap::from([
                (b"length".to_vec(), Value::Int(bytes.len() as i64)),
                (
                    b"path".to_vec(),
                    Value::List(
                        path.split('/')
                            .map(|c| Value::Bytes(c.as_bytes().to_vec()))
                            .collect(),
                    ),
                ),
            ])));
            recorded.push(((*path).to_string(), bytes));
        }

        let mut pieces = Vec::new();
        for chunk in payload.chunks(PIECE_LENGTH) {
            pieces.extend_from_slice(&Sha1::digest(chunk));
        }
        let info = Value::Dict(BTreeMap::from([
            (b"files".to_vec(), Value::List(files)),
            (b"name".to_vec(), Value::Bytes(b"hostile".to_vec())),
            (b"piece length".to_vec(), Value::Int(PIECE_LENGTH as i64)),
            (b"pieces".to_vec(), Value::Bytes(pieces)),
        ]));
        let bytes = encode(&Value::Dict(BTreeMap::from([(b"info".to_vec(), info)])));

        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();
        let torrent = root.join("hostile.torrent");
        std::fs::write(&torrent, &bytes).expect("write torrent");

        Self {
            _temp: temp,
            root,
            torrent,
            info_hash: Metainfo::parse(&bytes).expect("parse").info_hash().hex(),
            files: recorded,
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

/// A ranged HTTP server over a directory, on a thread, for the tests that
/// need a real web seed rather than a stub.
///
/// It binds port zero and reports what it got, so tests running at once never
/// race for a port. It speaks the little of HTTP/1.1 a web seed needs: `GET`,
/// one `Range: bytes=a-b` header, `206` with `Content-Range`, `404` for a path
/// that is not there. Nothing is kept alive between requests, which is slower
/// than a real mirror and is exactly why throughput assertions do not belong
/// against it.
pub struct FileServer {
    pub base: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl FileServer {
    /// Serve `root` on loopback.
    pub fn start(root: impl Into<PathBuf>) -> Self {
        use std::io::{Read, Write};

        let root = root.into();
        let listener =
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        listener
            .set_nonblocking(true)
            .expect("non-blocking listener");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();

        std::thread::spawn(move || {
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                };
                let root = root.clone();
                std::thread::spawn(move || {
                    let mut request = Vec::new();
                    let mut buf = [0u8; 4096];
                    // Headers end at the blank line. A web seed request has no
                    // body, so that is the whole request.
                    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                        match stream.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => request.extend_from_slice(&buf[..n]),
                        }
                    }
                    let text = String::from_utf8_lossy(&request).to_string();
                    let mut lines = text.lines();
                    let Some(start) = lines.next() else { return };
                    let Some(path) = start.split_whitespace().nth(1) else {
                        return;
                    };
                    let range = text
                        .lines()
                        .find_map(|line| line.strip_prefix("Range: bytes="))
                        .and_then(|spec| spec.split_once('-'))
                        .map(|(a, b)| (a.to_string(), b.to_string()));

                    let relative = percent_decode(path.trim_start_matches('/'));
                    let mut target = root.clone();
                    for part in relative.split('/').filter(|p| !p.is_empty()) {
                        target.push(part);
                    }
                    let Ok(body) = std::fs::read(&target) else {
                        let _ = stream.write_all(
                            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        );
                        return;
                    };

                    let total = body.len();
                    let response = match range {
                        None => format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n"
                        ),
                        Some((from, to)) => {
                            let from: usize = from.parse().unwrap_or(0);
                            let to: usize = to.parse().unwrap_or(total.saturating_sub(1));
                            let to = to.min(total.saturating_sub(1));
                            let slice = body.get(from..=to).unwrap_or(&[]).to_vec();
                            let head = format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{to}/{total}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                                slice.len()
                            );
                            let _ = stream.write_all(head.as_bytes());
                            let _ = stream.write_all(&slice);
                            let _ = stream.flush();
                            return;
                        }
                    };
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(&body);
                    let _ = stream.flush();
                });
            }
        });

        Self {
            base: format!("http://127.0.0.1:{port}/"),
            stop,
        }
    }
}

impl Drop for FileServer {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Decode `%XX` escapes in a request path.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
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

/// Run the binary and return stdout parsed as JSON, requiring a given exit
/// code.
///
/// For the commands whose JSON report is the point even though the run did not
/// succeed: a download that hit its deadline still reports where it wrote.
pub fn run_json_code(
    args: &[&str],
    cwd: impl Into<PathBuf>,
    expected: ExitCode,
) -> serde_json::Value {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let (mut env, captured) = Env::test(&full, cwd);
    let code = crate::run(&mut env);
    assert_eq!(
        code,
        expected,
        "`bit-cli {}` exited {code}, expected {expected}\nstdout:\n{}\nstderr:\n{}",
        full.join(" "),
        captured.out(),
        captured.err()
    );
    captured
        .json()
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n{}", captured.out()))
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
