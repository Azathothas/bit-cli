//! `bit-cli files`: list the files in a torrent.

use bit_cli_core::ExitCode;
use bit_cli_core::error::{Error, Result};
use bit_cli_core::units::{Size, format_size, percent_of};
use serde::Serialize;

use crate::cli::{FilesArgs, Global};
use crate::env::Env;
use crate::output::{Renderer, table};
use crate::source::{Kind, load_local};

/// One row of the listing.
#[derive(Debug, Clone, Serialize)]
pub struct FileRow {
    pub index: usize,
    pub path: String,
    pub size: Size,
    /// Byte offset within the torrent's linear payload.
    pub offset: u64,
    /// Piece indices this file touches, as a half-open range.
    pub first_piece: u32,
    pub last_piece: u32,
    /// Share of the whole payload, to two decimal places.
    pub share: String,
    /// Whether this is a BEP 47 padding file, which carries no real data.
    pub padding: bool,
}

/// What `bit-cli files` reports.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub info_hash: String,
    pub name: String,
    pub total: Size,
    pub file_count: usize,
    pub files: Vec<FileRow>,
}

impl Report {
    /// The text rendering.
    pub fn lines(&self) -> Vec<String> {
        let rows: Vec<Vec<String>> = self
            .files
            .iter()
            .map(|f| {
                vec![
                    f.index.to_string(),
                    format_size(f.size.0),
                    f.share.clone(),
                    format!("{}-{}", f.first_piece, f.last_piece),
                    f.path.clone(),
                ]
            })
            .collect();
        table(&["INDEX", "SIZE", "SHARE", "PIECES", "PATH"], &rows)
    }
}

/// A sort key for the listing.
fn sort_rows(rows: &mut [FileRow], spec: &str) -> Result<()> {
    let (key, order) = spec.split_once(':').unwrap_or((spec, "asc"));
    let descending = match order.trim().to_ascii_lowercase().as_str() {
        "asc" | "ascending" => false,
        "desc" | "descending" => true,
        other => {
            return Err(Error::usage(format!(
                "`{other}` is not a sort order (use asc or desc)"
            )));
        }
    };
    match key.trim().to_ascii_lowercase().as_str() {
        "index" => rows.sort_by_key(|r| r.index),
        "path" | "name" => rows.sort_by(|a, b| a.path.cmp(&b.path)),
        "size" | "length" => {
            rows.sort_by(|a, b| a.size.0.cmp(&b.size.0).then(a.index.cmp(&b.index)))
        }
        other => {
            return Err(Error::usage(format!(
                "`{other}` is not a sort key for `files` (use index, path, or size)"
            )));
        }
    }
    if descending {
        rows.reverse();
    }
    Ok(())
}

/// Run the command.
pub fn run(
    args: &FilesArgs,
    _global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let kind = Kind::classify(&args.source.source, env)?;
    let meta = load_local(&kind, env)?;
    let layout = meta.layout();
    let total = layout.total_length;

    let mut files: Vec<FileRow> = layout
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let pieces = layout.pieces_overlapping(&file.range());
            FileRow {
                index,
                path: file.display_path(),
                size: Size(file.length),
                offset: file.offset,
                first_piece: pieces.start,
                last_piece: pieces.end.saturating_sub(1),
                share: percent_of(file.length, total),
                padding: meta.info().files.get(index).is_some_and(|f| f.is_padding()),
            }
        })
        .collect();
    sort_rows(&mut files, &args.sort)?;

    let report = Report {
        info_hash: meta.info_hash().hex(),
        name: meta.info().name.clone(),
        total: Size(total),
        file_count: files.len(),
        files,
    };
    renderer.emit(env, "files", &report, || report.lines())?;
    Ok(ExitCode::Success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TorrentFixture, run_err, run_json, run_ok};

    #[test]
    fn files_lists_every_file_with_its_index() {
        let fixture = TorrentFixture::multi_file();
        let out = run_ok(&["files", fixture.path_str()], fixture.dir());
        assert!(out.contains("disc 1/a.flac"), "{out}");
        assert!(out.contains("notes.nfo"), "{out}");
        assert!(out.starts_with("INDEX"), "{out}");
    }

    #[test]
    fn the_json_form_carries_raw_bytes_alongside_the_human_string() {
        let fixture = TorrentFixture::multi_file();
        let doc = run_json(&["files", fixture.path_str()], fixture.dir());
        let files = doc["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0]["index"], 0);
        assert_eq!(files[0]["path"], "disc 1/a.flac");
        assert_eq!(files[0]["size"]["bytes"], 1500);
        assert_eq!(files[0]["size"]["human"], "1.46 KiB");
        assert_eq!(files[0]["offset"], 0);
        assert_eq!(files[1]["offset"], 1500);
    }

    #[test]
    fn piece_ranges_show_which_pieces_a_file_touches() {
        let fixture = TorrentFixture::multi_file();
        let doc = run_json(&["files", fixture.path_str()], fixture.dir());
        let files = doc["files"].as_array().unwrap();
        // 0..1500 with 1024 byte pieces touches pieces 0 and 1.
        assert_eq!(files[0]["first_piece"], 0);
        assert_eq!(files[0]["last_piece"], 1);
        // 1500..2000 lies entirely inside piece 1.
        assert_eq!(files[1]["first_piece"], 1);
        assert_eq!(files[1]["last_piece"], 1);
    }

    #[test]
    fn shares_add_up_to_the_whole_payload() {
        let fixture = TorrentFixture::multi_file();
        let doc = run_json(&["files", fixture.path_str()], fixture.dir());
        let total: f64 = doc["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                f["share"]
                    .as_str()
                    .unwrap()
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .unwrap()
            })
            .sum();
        assert!((total - 100.0).abs() < 0.01, "shares summed to {total}");
    }

    #[test]
    fn sorting_by_size_reorders_the_listing() {
        let fixture = TorrentFixture::multi_file();
        let doc = run_json(
            &["files", "--sort", "size", fixture.path_str()],
            fixture.dir(),
        );
        let files = doc["files"].as_array().unwrap();
        assert_eq!(files[0]["path"], "notes.nfo", "smallest first");

        let doc = run_json(
            &["files", "--sort", "size:desc", fixture.path_str()],
            fixture.dir(),
        );
        let files = doc["files"].as_array().unwrap();
        assert_eq!(files[0]["path"], "disc 1/a.flac", "largest first");
    }

    #[test]
    fn a_bad_sort_key_is_a_usage_error_that_names_the_valid_keys() {
        let fixture = TorrentFixture::multi_file();
        let err = run_err(
            &["files", "--sort", "mtime", fixture.path_str()],
            fixture.dir(),
            ExitCode::Usage,
        );
        assert!(err.contains("index, path, or size"), "{err}");
    }

    #[test]
    fn a_single_file_torrent_lists_one_file() {
        let fixture = TorrentFixture::single_file();
        let doc = run_json(&["files", fixture.path_str()], fixture.dir());
        assert_eq!(doc["file_count"], 1);
        assert_eq!(doc["files"][0]["path"], "payload.bin");
        assert_eq!(doc["files"][0]["share"], "100.00%");
    }
}
