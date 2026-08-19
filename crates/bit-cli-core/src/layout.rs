//! The shape of a torrent, independent of any torrent library.
//!
//! Scope resolution, URL composition, piece mapping, and coverage checking all
//! need the same facts: the name, whether it is multi-file, where each file
//! sits in the linear byte stream, and how the stream is cut into pieces.
//! [`Layout`] is that, and nothing else. Keeping it free of `librqbit` types
//! is what lets the addressing model be tested without a session, a network,
//! or a real `.torrent`.

use std::ops::Range;

use serde::{Deserialize, Serialize};

/// One file within a torrent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutFile {
    /// Path components relative to the torrent root, without the torrent name.
    /// Always `/`-separated when rendered, on every platform.
    pub path: Vec<String>,
    /// Byte offset of this file within the torrent's linear byte stream.
    pub offset: u64,
    /// Length of the file in bytes.
    pub length: u64,
}

impl LayoutFile {
    /// A file from a `/`-separated path.
    pub fn new(path: &str, offset: u64, length: u64) -> Self {
        Self {
            path: path
                .split('/')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            offset,
            length,
        }
    }

    /// The path as a single `/`-separated string.
    pub fn display_path(&self) -> String {
        self.path.join("/")
    }

    /// The final path component.
    pub fn file_name(&self) -> &str {
        self.path.last().map(String::as_str).unwrap_or_default()
    }

    /// The byte range this file occupies in the torrent's linear stream.
    pub fn range(&self) -> Range<u64> {
        self.offset..self.offset + self.length
    }
}

/// Everything about a torrent's shape that the addressing model needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    /// The torrent `name`, which is the directory name for a multi-file
    /// torrent and the file name for a single-file one.
    pub name: String,
    /// Whether the metainfo carries a `files` list.
    pub multi_file: bool,
    /// Files in torrent order.
    pub files: Vec<LayoutFile>,
    /// Length of a non-final piece.
    pub piece_length: u32,
    /// Total payload length in bytes.
    pub total_length: u64,
}

impl Layout {
    /// Build a layout, computing offsets from the file lengths in order.
    ///
    /// This is the constructor to use when you have lengths but not offsets,
    /// which is every case where the layout is being built from scratch rather
    /// than read out of a live torrent.
    pub fn from_lengths(
        name: impl Into<String>,
        multi_file: bool,
        piece_length: u32,
        files: impl IntoIterator<Item = (String, u64)>,
    ) -> Self {
        let mut offset = 0;
        let files: Vec<LayoutFile> = files
            .into_iter()
            .map(|(path, length)| {
                let file = LayoutFile::new(&path, offset, length);
                offset += length;
                file
            })
            .collect();
        Self {
            name: name.into(),
            multi_file,
            files,
            piece_length,
            total_length: offset,
        }
    }

    /// Number of pieces, including a short final one.
    pub fn piece_count(&self) -> u32 {
        if self.piece_length == 0 {
            return 0;
        }
        self.total_length.div_ceil(u64::from(self.piece_length)) as u32
    }

    /// Length of `piece`, which is shorter than `piece_length` for the last
    /// piece. `None` when the index is past the end.
    pub fn piece_size(&self, piece: u32) -> Option<u64> {
        let range = self.piece_range(piece)?;
        Some(range.end - range.start)
    }

    /// The byte range `piece` occupies, or `None` when the index is past the
    /// end.
    pub fn piece_range(&self, piece: u32) -> Option<Range<u64>> {
        if piece >= self.piece_count() {
            return None;
        }
        let start = u64::from(piece) * u64::from(self.piece_length);
        Some(start..(start + u64::from(self.piece_length)).min(self.total_length))
    }

    /// The byte range covering pieces `first..=last`, clamped to the payload.
    pub fn pieces_range(&self, first: u32, last: u32) -> Range<u64> {
        let start = u64::from(first) * u64::from(self.piece_length);
        let end = u64::from(last)
            .saturating_add(1)
            .saturating_mul(u64::from(self.piece_length))
            .min(self.total_length);
        start.min(self.total_length)..end
    }

    /// Index of the piece holding `offset`.
    pub fn piece_at(&self, offset: u64) -> Option<u32> {
        if self.piece_length == 0 || offset >= self.total_length {
            return None;
        }
        Some((offset / u64::from(self.piece_length)) as u32)
    }

    /// The whole payload as one range.
    pub fn payload(&self) -> Range<u64> {
        0..self.total_length
    }

    /// The file at `index`.
    pub fn file(&self, index: usize) -> Option<&LayoutFile> {
        self.files.get(index)
    }

    /// Index of the file holding `offset`.
    ///
    /// Zero-length files never hold a byte, so they are never returned.
    pub fn file_at(&self, offset: u64) -> Option<usize> {
        let index = self
            .files
            .partition_point(|f| f.offset + f.length <= offset);
        let file = self.files.get(index)?;
        (file.offset <= offset && file.length > 0).then_some(index)
    }

    /// Split the byte range `range` into per-file ranges, in torrent order.
    ///
    /// A range extending past the end of the payload is truncated, so the
    /// returned lengths may sum to less than the range asked for.
    pub fn split_by_file(&self, range: Range<u64>) -> Vec<FileSlice> {
        let end = range.end.min(self.total_length);
        let mut pos = range.start;
        let mut index = self.files.partition_point(|f| f.offset + f.length <= pos);
        let mut out = Vec::new();
        while pos < end {
            let Some(file) = self.files.get(index) else {
                break;
            };
            if file.length > 0 {
                let offset_in_file = pos - file.offset;
                let take = (file.length - offset_in_file).min(end - pos);
                out.push(FileSlice {
                    file: index,
                    offset: offset_in_file,
                    length: take,
                });
                pos += take;
            }
            index += 1;
        }
        out
    }

    /// Every piece index that overlaps `range`.
    pub fn pieces_overlapping(&self, range: &Range<u64>) -> Range<u32> {
        if self.piece_length == 0 || range.start >= range.end {
            return 0..0;
        }
        let first = (range.start / u64::from(self.piece_length)) as u32;
        let last = ((range.end - 1) / u64::from(self.piece_length)) as u32;
        first..last.saturating_add(1).min(self.piece_count())
    }
}

/// A contiguous byte range inside one file of a torrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSlice {
    /// Index of the file within the torrent.
    pub file: usize,
    /// Offset of the slice within that file.
    pub offset: u64,
    /// Length of the slice in bytes.
    pub length: u64,
}

impl FileSlice {
    /// The range within the file.
    pub fn range(&self) -> Range<u64> {
        self.offset..self.offset + self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn multi() -> Layout {
        Layout::from_lengths(
            "album",
            true,
            1024,
            [
                ("disc 1/a.flac".to_string(), 1500u64),
                ("disc 1/b.flac".to_string(), 500),
                ("notes.txt".to_string(), 100),
            ],
        )
    }

    #[test]
    fn offsets_follow_from_lengths() {
        let layout = multi();
        assert_eq!(layout.file(0).unwrap().offset, 0);
        assert_eq!(layout.file(1).unwrap().offset, 1500);
        assert_eq!(layout.file(2).unwrap().offset, 2000);
        assert_eq!(layout.total_length, 2100);
    }

    #[test]
    fn paths_split_on_forward_slashes() {
        let layout = multi();
        assert_eq!(layout.file(0).unwrap().path, ["disc 1", "a.flac"]);
        assert_eq!(layout.file(0).unwrap().display_path(), "disc 1/a.flac");
        assert_eq!(layout.file(0).unwrap().file_name(), "a.flac");
    }

    #[test]
    fn the_last_piece_is_short() {
        let layout = multi();
        assert_eq!(layout.piece_count(), 3);
        assert_eq!(layout.piece_size(0), Some(1024));
        assert_eq!(layout.piece_size(1), Some(1024));
        assert_eq!(layout.piece_size(2), Some(52));
        assert_eq!(layout.piece_size(3), None);
    }

    #[test]
    fn piece_ranges_clamp_to_the_payload() {
        let layout = multi();
        assert_eq!(layout.piece_range(0), Some(0..1024));
        assert_eq!(layout.piece_range(2), Some(2048..2100));
        assert_eq!(layout.pieces_range(0, 1), 0..2048);
        assert_eq!(layout.pieces_range(0, 99), 0..2100);
    }

    #[test]
    fn offsets_map_back_to_pieces_and_files() {
        let layout = multi();
        assert_eq!(layout.piece_at(0), Some(0));
        assert_eq!(layout.piece_at(1023), Some(0));
        assert_eq!(layout.piece_at(1024), Some(1));
        assert_eq!(layout.piece_at(2100), None);
        assert_eq!(layout.file_at(0), Some(0));
        assert_eq!(layout.file_at(1499), Some(0));
        assert_eq!(layout.file_at(1500), Some(1));
        assert_eq!(layout.file_at(2100), None);
    }

    #[test]
    fn a_range_splits_across_file_boundaries() {
        let layout = multi();
        let slices = layout.split_by_file(1400..2050);
        assert_eq!(
            slices,
            vec![
                FileSlice {
                    file: 0,
                    offset: 1400,
                    length: 100
                },
                FileSlice {
                    file: 1,
                    offset: 0,
                    length: 500
                },
                FileSlice {
                    file: 2,
                    offset: 0,
                    length: 50
                },
            ]
        );
    }

    #[test]
    fn zero_length_files_are_never_asked_for_bytes() {
        let layout = Layout::from_lengths(
            "t",
            true,
            16,
            [
                ("a".to_string(), 50u64),
                ("empty".to_string(), 0),
                ("b".to_string(), 50),
            ],
        );
        assert_eq!(layout.file_at(50), Some(2));
        let slices = layout.split_by_file(40..60);
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].file, 0);
        assert_eq!(slices[1].file, 2);
    }

    #[test]
    fn overlapping_pieces_cover_the_whole_range() {
        let layout = multi();
        assert_eq!(layout.pieces_overlapping(&(0..1)), 0..1);
        assert_eq!(layout.pieces_overlapping(&(1023..1025)), 0..2);
        assert_eq!(layout.pieces_overlapping(&(0..2100)), 0..3);
        assert_eq!(layout.pieces_overlapping(&(5..5)), 0..0);
    }

    #[test]
    fn a_single_file_torrent_has_one_file_at_offset_zero() {
        let layout = Layout::from_lengths(
            "movie.mkv",
            false,
            4096,
            [("movie.mkv".to_string(), 9000u64)],
        );
        assert_eq!(layout.files.len(), 1);
        assert_eq!(layout.payload(), 0..9000);
        assert_eq!(layout.piece_count(), 3);
    }
}
