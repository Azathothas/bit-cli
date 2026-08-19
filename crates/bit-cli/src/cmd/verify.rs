//! `bit-cli verify`: hash-check data on disk against a torrent.
//!
//! Every piece is read from the payload and hashed. A piece that spans a file
//! boundary is read across it, and a file that is missing or short is treated
//! as zero bytes rather than aborting, so one absent file does not hide the
//! state of everything else.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use bit_cli_core::ExitCode;
use bit_cli_core::error::{Error, Result, from_io};
use bit_cli_core::layout::Layout;
use bit_cli_core::span::summarize_indices;
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{Size, format_size, percent_of};
use serde::Serialize;
use sha1::{Digest, Sha1};

use crate::cli::{Global, VerifyArgs};
use crate::env::Env;
use crate::output::{Renderer, field};
use crate::source::{Kind, load_local};

/// One piece's result, when `--per-piece` is given.
#[derive(Debug, Clone, Serialize)]
pub struct PieceResult {
    pub piece: u32,
    pub ok: bool,
    pub bytes: u64,
}

/// One file's result.
#[derive(Debug, Clone, Serialize)]
pub struct FileResult {
    pub index: usize,
    pub path: String,
    pub expected: Size,
    pub found: Size,
    pub present: bool,
}

/// What `bit-cli verify` reports.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub info_hash: String,
    pub name: String,
    pub data_dir: String,
    pub total: Size,
    pub piece_count: u32,
    pub pieces_ok: u32,
    pub pieces_bad: u32,
    pub complete: bool,
    pub have: Size,
    pub have_share: String,
    pub bad_pieces: Vec<u32>,
    pub files: Vec<FileResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub per_piece: Vec<PieceResult>,
}

impl Report {
    /// The text rendering.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![
            field("torrent", &self.name),
            field("info hash", &self.info_hash),
            field("data", &self.data_dir),
            field(
                "pieces ok",
                format!("{} of {}", self.pieces_ok, self.piece_count),
            ),
            field(
                "have",
                format!("{} ({})", format_size(self.have.0), self.have_share),
            ),
            field("complete", self.complete),
        ];
        if !self.bad_pieces.is_empty() {
            out.push(field("failed pieces", summarize_indices(&self.bad_pieces)));
        }
        for file in &self.files {
            if !file.present {
                out.push(field("missing", &file.path));
            } else if file.found.0 != file.expected.0 {
                out.push(field(
                    "short",
                    format!(
                        "{} ({} of {})",
                        file.path,
                        format_size(file.found.0),
                        format_size(file.expected.0)
                    ),
                ));
            }
        }
        out
    }
}

/// Reads the torrent's linear byte stream out of the files on disk.
///
/// Files are opened lazily and kept open, because a piece usually spans the
/// same one or two files as the previous piece and reopening per piece would
/// dominate the run.
struct PayloadReader<'a> {
    layout: &'a Layout,
    root: PathBuf,
    open: Vec<Option<Option<std::fs::File>>>,
}

impl<'a> PayloadReader<'a> {
    fn new(layout: &'a Layout, root: PathBuf) -> Self {
        let open = (0..layout.files.len()).map(|_| None).collect();
        Self { layout, root, open }
    }

    /// The on-disk path of one file.
    fn path_of(&self, index: usize) -> Option<PathBuf> {
        let file = self.layout.file(index)?;
        let mut path = self.root.clone();
        // A single-file torrent lays its one file down at the root under the
        // torrent name, so the name is not repeated as a directory.
        for component in &file.path {
            path.push(component);
        }
        Some(path)
    }

    /// Read one byte range of the payload, zero-filling anything missing.
    fn read(&mut self, offset: u64, length: u64) -> Result<Vec<u8>> {
        let mut out = vec![0u8; length as usize];
        for slice in self.layout.split_by_file(offset..offset + length) {
            let start = (slice.file_start(self.layout, offset)) as usize;
            let handle = self.handle(slice.file)?;
            let Some(file) = handle else { continue };
            file.seek(SeekFrom::Start(slice.offset))
                .map_err(|e| from_io(e, "cannot seek in the payload"))?;
            let end = start + slice.length as usize;
            // A short read is not an error: it means the file on disk is
            // shorter than the torrent says, and the piece will simply fail
            // its hash, which is the honest answer.
            let mut filled = start;
            while filled < end {
                match file.read(&mut out[filled..end]) {
                    Ok(0) => break,
                    Ok(n) => filled += n,
                    Err(e) => return Err(from_io(e, "cannot read the payload")),
                }
            }
        }
        Ok(out)
    }

    fn handle(&mut self, index: usize) -> Result<Option<&mut std::fs::File>> {
        if self.open[index].is_none() {
            let opened = match self.path_of(index) {
                None => None,
                Some(path) => std::fs::File::open(&path).ok(),
            };
            self.open[index] = Some(opened);
        }
        Ok(self.open[index].as_mut().and_then(|slot| slot.as_mut()))
    }
}

/// Where a file slice lands in the output buffer.
trait SliceOffset {
    fn file_start(&self, layout: &Layout, request_start: u64) -> u64;
}

impl SliceOffset for bit_cli_core::layout::FileSlice {
    fn file_start(&self, layout: &Layout, request_start: u64) -> u64 {
        let absolute = layout.file(self.file).map(|f| f.offset).unwrap_or(0) + self.offset;
        absolute.saturating_sub(request_start)
    }
}

/// Run the command.
pub fn run(
    args: &VerifyArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let kind = Kind::classify(&args.source.source, env)?;
    let meta = load_local(&kind, env)?;
    let layout = meta.layout();

    let root = resolve_root(args, global, env, &meta);
    let mut reader = PayloadReader::new(&layout, root.clone());

    let files: Vec<FileResult> = layout
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let found = reader
                .path_of(index)
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len());
            FileResult {
                index,
                path: file.display_path(),
                expected: Size(file.length),
                found: Size(found.unwrap_or(0)),
                present: found.is_some(),
            }
        })
        .collect();

    let mut pieces_ok = 0u32;
    let mut bad_pieces = Vec::new();
    let mut per_piece = Vec::new();
    let mut have = 0u64;

    for piece in 0..layout.piece_count() {
        let Some(range) = layout.piece_range(piece) else {
            continue;
        };
        let length = range.end - range.start;
        let data = reader.read(range.start, length)?;
        let expected =
            meta.info().pieces.get(piece as usize).ok_or_else(|| {
                Error::generic(format!("the torrent has no hash for piece {piece}"))
            })?;
        let mut hasher = Sha1::new();
        hasher.update(&data);
        let actual: [u8; 20] = hasher.finalize().into();
        let ok = &actual == expected;
        if ok {
            pieces_ok += 1;
            have += length;
        } else {
            bad_pieces.push(piece);
        }
        if args.per_piece {
            per_piece.push(PieceResult {
                piece,
                ok,
                bytes: length,
            });
        }
    }

    let piece_count = layout.piece_count();
    let report = Report {
        info_hash: meta.info_hash().hex(),
        name: layout.name.clone(),
        data_dir: root.display().to_string(),
        total: Size(layout.total_length),
        piece_count,
        pieces_ok,
        pieces_bad: piece_count - pieces_ok,
        complete: pieces_ok == piece_count && piece_count > 0,
        have: Size(have),
        have_share: percent_of(have, layout.total_length),
        bad_pieces,
        files,
        per_piece,
    };

    // An incomplete or corrupt payload exits non-zero, so a pipeline does not
    // have to parse the report to find out.
    //
    // On failure the report goes into the error's context rather than being
    // emitted first. Emitting both would put two JSON documents on stdout,
    // which is not something `jq` can read.
    if !report.complete {
        return Err(Error::hash_mismatch(format!(
            "{} of {} pieces failed",
            report.pieces_bad, report.piece_count
        ))
        .with("pieces_ok", report.pieces_ok)
        .with("pieces_bad", report.pieces_bad)
        .with(
            "bad_pieces",
            serde_json::to_value(&report.bad_pieces).unwrap_or_default(),
        )
        .with("report", serde_json::to_value(&report).unwrap_or_default()));
    }

    renderer.emit(env, "verify", &report, || report.lines())?;
    Ok(ExitCode::Success)
}

/// Where the payload lives.
///
/// A multi-file torrent lays its files under a directory named after the
/// torrent, so `--data` pointing at the parent and pointing at the directory
/// itself both have to work. Whichever contains the first file wins.
fn resolve_root(args: &VerifyArgs, global: &Global, env: &Env, meta: &Metainfo) -> PathBuf {
    let base = args
        .data
        .clone()
        .or_else(|| global.dir.clone())
        .map(|p| env.resolve(&p))
        .unwrap_or_else(|| env.cwd.clone());

    let layout = meta.layout();
    let Some(first) = layout.files.first() else {
        return base;
    };
    let direct: PathBuf = first.path.iter().fold(base.clone(), |acc, c| acc.join(c));
    if direct.exists() {
        return base;
    }
    let nested = base.join(&layout.name);
    let inside: PathBuf = first.path.iter().fold(nested.clone(), |acc, c| acc.join(c));
    if inside.exists() {
        return nested;
    }
    base
}

/// The path helper is also useful to callers checking a single file.
pub fn payload_path(root: &Path, layout: &Layout, index: usize) -> Option<PathBuf> {
    let file = layout.file(index)?;
    Some(
        file.path
            .iter()
            .fold(root.to_path_buf(), |acc, c| acc.join(c)),
    )
}

#[cfg(test)]
mod tests {
    use crate::test_support::{TorrentFixture, run_err, run_json};
    use bit_cli_core::ExitCode;

    #[test]
    fn a_complete_payload_verifies_and_exits_zero() {
        let fixture = TorrentFixture::multi_file();
        let doc = run_json(
            &[
                "verify",
                "--data",
                fixture.payload_dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        assert_eq!(doc["complete"], true);
        assert_eq!(doc["pieces_ok"], 2);
        assert_eq!(doc["pieces_bad"], 0);
        assert_eq!(doc["have"]["bytes"], 2000);
        assert_eq!(doc["have_share"], "100.00%");
    }

    #[test]
    fn a_corrupt_byte_fails_exactly_one_piece() {
        let fixture = TorrentFixture::multi_file();
        let target = fixture.payload_dir().join("notes.nfo");
        let mut bytes = std::fs::read(&target).unwrap();
        bytes[0] ^= 0xFF;
        std::fs::write(&target, &bytes).unwrap();

        let (mut env, captured) = crate::env::Env::test(
            &[
                "verify",
                "--json",
                "--data",
                fixture.payload_dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        assert_eq!(crate::run(&mut env), ExitCode::HashMismatch);

        // Exactly one JSON document on stdout, carrying which piece failed.
        let doc: serde_json::Value = captured.json().expect("stdout must be one JSON document");
        assert_eq!(doc["code"], 7);
        assert_eq!(doc["kind"], "hash_mismatch");
        // notes.nfo is 1500..2000, which lies inside piece 1.
        assert_eq!(doc["context"]["bad_pieces"], serde_json::json!([1]));
        assert_eq!(doc["context"]["pieces_ok"], 1);
        assert_eq!(doc["context"]["report"]["complete"], false);
        assert!(captured.err().contains("failed"), "{}", captured.err());
    }

    #[test]
    fn a_missing_file_is_reported_rather_than_aborting() {
        let fixture = TorrentFixture::multi_file();
        std::fs::remove_file(fixture.payload_dir().join("notes.nfo")).unwrap();
        let err = run_err(
            &[
                "verify",
                "--data",
                fixture.payload_dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
            ExitCode::HashMismatch,
        );
        assert!(err.contains("failed"), "{err}");
    }

    #[test]
    fn per_piece_reports_every_piece() {
        let fixture = TorrentFixture::multi_file();
        let doc = run_json(
            &[
                "verify",
                "--per-piece",
                "--data",
                fixture.payload_dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        let pieces = doc["per_piece"].as_array().unwrap();
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0]["piece"], 0);
        assert_eq!(pieces[0]["ok"], true);
        assert_eq!(pieces[1]["bytes"], 976, "the last piece is short");
    }

    #[test]
    fn a_single_file_payload_verifies() {
        let fixture = TorrentFixture::single_file();
        let doc = run_json(
            &[
                "verify",
                "--data",
                fixture.payload_dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        assert_eq!(doc["complete"], true);
        assert_eq!(doc["pieces_ok"], 3);
    }

    #[test]
    fn the_data_directory_can_be_the_parent_of_the_torrent_directory() {
        let fixture = TorrentFixture::multi_file();
        // Move the payload under a directory named after the torrent, which is
        // how a real download lays it out.
        let nested = fixture.root.join("downloads").join("album");
        std::fs::create_dir_all(nested.join("disc 1")).unwrap();
        for (path, bytes) in &fixture.files {
            std::fs::write(nested.join(path), bytes).unwrap();
        }
        let doc = run_json(
            &[
                "verify",
                "--data",
                fixture.root.join("downloads").to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        assert_eq!(doc["complete"], true);
    }

    #[test]
    fn file_results_report_what_is_present_and_what_is_short() {
        let fixture = TorrentFixture::multi_file();
        std::fs::write(fixture.payload_dir().join("notes.nfo"), vec![0u8; 100]).unwrap();
        let (mut env, captured) = crate::env::Env::test(
            &[
                "verify",
                "--json",
                "--data",
                fixture.payload_dir().to_str().unwrap(),
                fixture.path_str(),
            ],
            fixture.dir(),
        );
        assert_eq!(crate::run(&mut env), ExitCode::HashMismatch);
        let doc: serde_json::Value = captured.json().expect("one JSON document");
        let files = doc["context"]["report"]["files"].as_array().unwrap();
        assert_eq!(files[1]["expected"]["bytes"], 500);
        assert_eq!(files[1]["found"]["bytes"], 100);
        assert_eq!(files[1]["present"], true);
    }
}
