//! Reading and writing `.torrent` metainfo.
//!
//! The info hash is the SHA-1 of the `info` dictionary's encoded bytes.
//! Everything about editing a torrent turns on keeping those bytes exactly as
//! they were: `announce`, `announce-list`, `url-list`, `httpseeds`, `comment`,
//! `created by`, `creation date`, and `nodes` all live outside `info` and can
//! change freely, and anything inside `info` produces a different torrent.
//!
//! [`Metainfo`] keeps the original `info` bytes and splices them back in
//! verbatim on write, so an edit cannot change the info hash even if the
//! original encoding was not canonical. [`Metainfo::write_to_vec`] proves it
//! by recomputing the hash from what it just produced.

use std::collections::BTreeMap;

use sha1::{Digest, Sha1};

use crate::error::{Error, Result};
use crate::layout::Layout;
use crate::time::Timestamp;
use crate::torrent::bencode::{self, Value};

/// A 20-byte SHA-1 info hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InfoHash(pub [u8; 20]);

impl InfoHash {
    /// The hash of some bytes.
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        Self(hasher.finalize().into())
    }

    /// Lower-case hex.
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Parse from 40 hex characters, or from 32 base32 characters as a magnet
    /// URI may carry.
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        if text.len() == 40 {
            let mut out = [0u8; 20];
            for (index, pair) in text.as_bytes().chunks(2).enumerate() {
                let hex = std::str::from_utf8(pair).map_err(|_| bad_hash(text))?;
                out[index] = u8::from_str_radix(hex, 16).map_err(|_| bad_hash(text))?;
            }
            return Ok(Self(out));
        }
        if text.len() == 32 {
            return decode_base32(text).map(Self).ok_or_else(|| bad_hash(text));
        }
        Err(bad_hash(text))
    }
}

fn bad_hash(text: &str) -> Error {
    Error::source_resolution(format!(
        "`{text}` is not an info hash (expected 40 hex characters or 32 base32 characters)"
    ))
    .with("value", text.to_string())
}

/// Decode RFC 4648 base32 without padding into 20 bytes.
fn decode_base32(text: &str) -> Option<[u8; 20]> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u32;
    let mut count = 0u32;
    let mut out = Vec::with_capacity(20);
    for c in text.bytes() {
        let upper = c.to_ascii_uppercase();
        let index = ALPHABET.iter().position(|a| *a == upper)? as u32;
        bits = (bits << 5) | index;
        count += 5;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    out.try_into().ok()
}

impl std::fmt::Display for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.hex())
    }
}

impl serde::Serialize for InfoHash {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex())
    }
}

/// One file inside a multi-file torrent's `info` dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoFile {
    /// Path components relative to the torrent root.
    pub path: Vec<String>,
    /// Length in bytes.
    pub length: u64,
    /// The BEP 47 `attr` string, when present. `p` marks a padding file.
    pub attr: Option<String>,
    /// Per-file MD5, when the creator wrote one.
    pub md5sum: Option<String>,
}

impl InfoFile {
    /// Whether this is a BEP 47 padding file, which carries no real data and
    /// is not shown to the user as a file.
    pub fn is_padding(&self) -> bool {
        self.attr.as_deref().is_some_and(|a| a.contains('p'))
    }
}

/// The parsed `info` dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    /// The torrent name: a directory name for multi-file, a file name for
    /// single-file.
    pub name: String,
    /// Length of a non-final piece.
    pub piece_length: u32,
    /// The SHA-1 of each piece, in order.
    pub pieces: Vec<[u8; 20]>,
    /// Files, always populated. A single-file torrent has exactly one entry
    /// whose path is the torrent name.
    pub files: Vec<InfoFile>,
    /// Whether the metainfo carried a `files` list.
    pub multi_file: bool,
    /// BEP 27 private flag.
    pub private: bool,
    /// The `source` key, used for cross-seeding. It is inside `info`, so
    /// changing it changes the info hash.
    pub source: Option<String>,
    /// Whether a BEP 52 `meta version` key is present, and its value.
    pub meta_version: Option<i64>,
}

impl Info {
    /// Total payload length.
    pub fn total_length(&self) -> u64 {
        self.files.iter().map(|f| f.length).sum()
    }

    /// The torrent's shape, for the addressing model.
    pub fn layout(&self) -> Layout {
        Layout::from_lengths(
            self.name.clone(),
            self.multi_file,
            self.piece_length,
            self.files.iter().map(|f| (f.path.join("/"), f.length)),
        )
    }
}

/// A parsed `.torrent`.
#[derive(Debug, Clone)]
pub struct Metainfo {
    /// The whole top-level dictionary, as parsed.
    root: Value,
    /// The exact bytes of the `info` dictionary's value.
    info_bytes: Vec<u8>,
    /// The info hash of those bytes.
    info_hash: InfoHash,
    /// The parsed `info` dictionary.
    info: Info,
}

impl Metainfo {
    /// Parse a `.torrent` from bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (root, span) = bencode::decode_with_info_span(bytes)
            .map_err(|e| Error::source_resolution(format!("not a valid torrent: {e}")))?;
        let span =
            span.ok_or_else(|| Error::source_resolution("torrent has no `info` dictionary"))?;
        let info_bytes = bytes[span].to_vec();
        let info_hash = InfoHash::of(&info_bytes);
        let info = parse_info(&root)?;
        Ok(Self {
            root,
            info_bytes,
            info_hash,
            info,
        })
    }

    /// Read a `.torrent` from a file.
    ///
    /// A torrent that cannot be read is a source resolution failure, not a
    /// disk failure: from the caller's side "the file is not there" and "the
    /// file is not a torrent" are the same problem, and the exit code table
    /// puts an unreadable torrent under code 4.
    pub fn read(path: &std::path::Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|e| {
            Error::source_resolution(format!("cannot read {}: {e}", path.display()))
                .with("path", path.display().to_string())
                .with("io_kind", format!("{:?}", e.kind()))
        })?;
        Self::parse(&bytes)
            .map_err(|e| Error::source_resolution(format!("{}: {e}", path.display())))
    }

    /// Build a metainfo from an already-encoded `info` dictionary.
    ///
    /// This is the path `bit-cli create` takes: the `info` dictionary is
    /// encoded once, its bytes are hashed, and those same bytes are what get
    /// written. There is no second encoding that could differ.
    pub fn from_info_bytes(info_bytes: Vec<u8>) -> Result<Self> {
        let info_value = bencode::decode(&info_bytes).map_err(|e| {
            Error::generic(format!("the info dictionary is not valid bencode: {e}"))
        })?;
        let mut map = BTreeMap::new();
        map.insert(b"info".to_vec(), info_value);
        let root = Value::Dict(map);
        let info = parse_info(&root)?;
        let info_hash = InfoHash::of(&info_bytes);
        Ok(Self {
            root,
            info_bytes,
            info_hash,
            info,
        })
    }

    /// The info hash.
    pub fn info_hash(&self) -> InfoHash {
        self.info_hash
    }

    /// The exact bytes the info hash was computed over.
    pub fn info_bytes(&self) -> &[u8] {
        &self.info_bytes
    }

    /// The parsed `info` dictionary.
    pub fn info(&self) -> &Info {
        &self.info
    }

    /// The torrent's shape, for the addressing model.
    pub fn layout(&self) -> Layout {
        self.info.layout()
    }

    /// The top-level dictionary, for fields this type does not name.
    pub fn root(&self) -> &Value {
        &self.root
    }

    /// The primary tracker.
    pub fn announce(&self) -> Option<String> {
        self.root.get("announce").and_then(Value::as_text)
    }

    /// The BEP 12 tracker tiers.
    ///
    /// A torrent with only `announce` reads as one tier holding it, so callers
    /// never have to handle both shapes.
    pub fn announce_tiers(&self) -> Vec<Vec<String>> {
        let tiers: Vec<Vec<String>> = self
            .root
            .get("announce-list")
            .and_then(Value::as_list)
            .map(|tiers| {
                tiers
                    .iter()
                    .map(Value::as_text_list)
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        if !tiers.is_empty() {
            return tiers;
        }
        self.announce().map(|a| vec![vec![a]]).unwrap_or_default()
    }

    /// Every tracker, flattened.
    pub fn trackers(&self) -> Vec<String> {
        self.announce_tiers().into_iter().flatten().collect()
    }

    /// BEP 19 `url-list` web seeds.
    ///
    /// The key is a string for a single entry and a list for several, and both
    /// appear in the wild.
    pub fn url_list(&self) -> Vec<String> {
        match self.root.get("url-list") {
            Some(Value::Bytes(_)) => self
                .root
                .get("url-list")
                .and_then(Value::as_text)
                .into_iter()
                .collect(),
            Some(value) => value.as_text_list(),
            None => Vec::new(),
        }
    }

    /// BEP 17 `httpseeds`.
    pub fn http_seeds(&self) -> Vec<String> {
        self.root
            .get("httpseeds")
            .map(Value::as_text_list)
            .unwrap_or_default()
    }

    /// DHT bootstrap nodes written into the torrent, as `host:port`.
    pub fn nodes(&self) -> Vec<String> {
        self.root
            .get("nodes")
            .and_then(Value::as_list)
            .map(|nodes| {
                nodes
                    .iter()
                    .filter_map(|node| {
                        let pair = node.as_list()?;
                        let host = pair.first()?.as_text()?;
                        let port = pair.get(1)?.as_int()?;
                        Some(format!("{host}:{port}"))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The `comment` field.
    pub fn comment(&self) -> Option<String> {
        self.root.get("comment").and_then(Value::as_text)
    }

    /// The `created by` field.
    pub fn created_by(&self) -> Option<String> {
        self.root.get("created by").and_then(Value::as_text)
    }

    /// The `creation date` field.
    pub fn creation_date(&self) -> Option<Timestamp> {
        self.root
            .get("creation date")
            .and_then(Value::as_int)
            .map(Timestamp::from_epoch_secs)
    }

    /// The BEP 39 `update-url` feed.
    pub fn update_url(&self) -> Option<String> {
        self.root.get("update-url").and_then(Value::as_text)
    }

    /// Replace a top-level field, or remove it when `value` is `None`.
    ///
    /// Refuses to touch `info`, because that is the one field whose bytes the
    /// info hash depends on.
    pub fn set(&mut self, key: &str, value: Option<Value>) -> Result<()> {
        if key == "info" {
            return Err(Error::would_change_infohash(
                "the `info` dictionary cannot be edited in place: it is what the info hash is computed over",
            ));
        }
        let map = self
            .root
            .as_dict_mut()
            .ok_or_else(|| Error::generic("the torrent's root is not a dictionary"))?;
        match value {
            Some(value) => map.insert(key.as_bytes().to_vec(), value),
            None => map.remove(key.as_bytes()),
        };
        Ok(())
    }

    /// Encode the torrent.
    ///
    /// Every key other than `info` is re-encoded canonically. `info` is
    /// spliced in as the original bytes, so the info hash is preserved exactly.
    /// The result is checked before it is returned.
    pub fn write_to_vec(&self) -> Result<Vec<u8>> {
        let map = self
            .root
            .as_dict()
            .ok_or_else(|| Error::generic("the torrent's root is not a dictionary"))?;
        let mut out = Vec::with_capacity(self.info_bytes.len() + 512);
        out.push(b'd');
        // Keys are emitted in sorted byte order, with `info` taking its place
        // in that order like any other key.
        let info_key = b"info".to_vec();
        for (key, value) in map {
            bencode::encode_into(&Value::Bytes(key.clone()), &mut out);
            if *key == info_key {
                out.extend_from_slice(&self.info_bytes);
            } else {
                bencode::encode_into(value, &mut out);
            }
        }
        out.push(b'e');

        // Prove the splice worked rather than trusting it. Getting this wrong
        // would silently publish a different torrent.
        let (_, span) = bencode::decode_with_info_span(&out)
            .map_err(|e| Error::generic(format!("produced an invalid torrent: {e}")))?;
        let span =
            span.ok_or_else(|| Error::generic("produced a torrent with no info dictionary"))?;
        let written = InfoHash::of(&out[span]);
        if written != self.info_hash {
            return Err(Error::would_change_infohash(format!(
                "writing the torrent would change the info hash from {} to {written}",
                self.info_hash
            ))
            .with("before", self.info_hash.hex())
            .with("after", written.hex()));
        }
        Ok(out)
    }
}

fn parse_info(root: &Value) -> Result<Info> {
    let info = root
        .get("info")
        .ok_or_else(|| Error::source_resolution("torrent has no `info` dictionary"))?;
    let missing = |key: &str| {
        Error::source_resolution(format!("torrent `info` dictionary has no `{key}`"))
            .with("key", key.to_string())
    };

    let name = info
        .get("name")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("name"))?;
    let piece_length = info
        .get("piece length")
        .and_then(Value::as_int)
        .ok_or_else(|| missing("piece length"))?;
    let piece_length = u32::try_from(piece_length).map_err(|_| {
        Error::source_resolution(format!(
            "piece length {piece_length} does not fit in 32 bits"
        ))
    })?;
    if piece_length == 0 {
        return Err(Error::source_resolution("piece length is zero"));
    }

    let raw_pieces = info
        .get("pieces")
        .and_then(Value::as_bytes)
        .ok_or_else(|| missing("pieces"))?;
    if raw_pieces.len() % 20 != 0 {
        return Err(Error::source_resolution(format!(
            "`pieces` is {} bytes, which is not a multiple of 20",
            raw_pieces.len()
        ))
        .with("pieces_bytes", raw_pieces.len()));
    }
    // The length was checked to be a multiple of 20 just above, so the
    // remainder here is always empty.
    let (chunks, _) = raw_pieces.as_chunks::<20>();
    let pieces: Vec<[u8; 20]> = chunks.to_vec();

    let (files, multi_file) = match info.get("files").and_then(Value::as_list) {
        Some(entries) => {
            let mut files = Vec::with_capacity(entries.len());
            for entry in entries {
                let length = entry
                    .get("length")
                    .and_then(Value::as_int)
                    .ok_or_else(|| Error::source_resolution("a file entry has no `length`"))?;
                let path: Vec<String> = entry
                    .get("path")
                    .map(Value::as_text_list)
                    .filter(|p| !p.is_empty())
                    .ok_or_else(|| Error::source_resolution("a file entry has no `path`"))?;
                files.push(InfoFile {
                    path,
                    length: u64::try_from(length).map_err(|_| {
                        Error::source_resolution(format!("file length {length} is negative"))
                    })?,
                    attr: entry.get("attr").and_then(Value::as_text),
                    md5sum: entry.get("md5sum").and_then(Value::as_text),
                });
            }
            (files, true)
        }
        None => {
            let length = info.get("length").and_then(Value::as_int).ok_or_else(|| {
                Error::source_resolution(
                    "torrent has neither `files` nor `length`, so it describes no data",
                )
            })?;
            let file = InfoFile {
                path: vec![name.clone()],
                length: u64::try_from(length).map_err(|_| {
                    Error::source_resolution(format!("length {length} is negative"))
                })?,
                attr: info.get("attr").and_then(Value::as_text),
                md5sum: info.get("md5sum").and_then(Value::as_text),
            };
            (vec![file], false)
        }
    };

    let total: u64 = files.iter().map(|f| f.length).sum();
    let expected_pieces = total.div_ceil(u64::from(piece_length)) as usize;
    if pieces.len() != expected_pieces {
        return Err(Error::source_resolution(format!(
            "torrent declares {} pieces but {total} bytes at {piece_length} bytes per piece needs {expected_pieces}",
            pieces.len()
        ))
        .with("declared_pieces", pieces.len())
        .with("expected_pieces", expected_pieces)
        .with("total_bytes", total));
    }

    Ok(Info {
        name,
        piece_length,
        pieces,
        files,
        multi_file,
        private: info
            .get("private")
            .and_then(Value::as_int)
            .is_some_and(|v| v != 0),
        source: info.get("source").and_then(Value::as_text),
        meta_version: info.get("meta version").and_then(Value::as_int),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(pairs: Vec<(&str, Value)>) -> Value {
        Value::Dict(
            pairs
                .into_iter()
                .map(|(k, v)| (k.as_bytes().to_vec(), v))
                .collect(),
        )
    }

    /// A single-file torrent: 3000 bytes, 1024-byte pieces, so three pieces.
    fn single_file() -> Vec<u8> {
        bencode::encode(&dict(vec![
            ("announce", Value::text("udp://tracker.example.com:80")),
            ("comment", Value::text("hello")),
            ("creation date", Value::Int(1_787_140_323)),
            (
                "info",
                dict(vec![
                    ("length", Value::Int(3000)),
                    ("name", Value::text("payload.bin")),
                    ("piece length", Value::Int(1024)),
                    ("pieces", Value::Bytes(vec![0u8; 60])),
                ]),
            ),
            (
                "url-list",
                Value::List(vec![Value::text("https://e.com/pub/")]),
            ),
        ]))
    }

    /// A multi-file torrent: 1500 + 500 bytes, 1024-byte pieces.
    fn multi_file() -> Vec<u8> {
        let file = |path: &[&str], length: i64| {
            dict(vec![
                ("length", Value::Int(length)),
                (
                    "path",
                    Value::List(path.iter().map(|p| Value::text(*p)).collect()),
                ),
            ])
        };
        bencode::encode(&dict(vec![
            (
                "announce-list",
                Value::List(vec![
                    Value::List(vec![Value::text("udp://a:80"), Value::text("udp://b:80")]),
                    Value::List(vec![Value::text("udp://c:80")]),
                ]),
            ),
            (
                "info",
                dict(vec![
                    (
                        "files",
                        Value::List(vec![
                            file(&["disc 1", "a.flac"], 1500),
                            file(&["notes.nfo"], 500),
                        ]),
                    ),
                    ("name", Value::text("album")),
                    ("piece length", Value::Int(1024)),
                    ("pieces", Value::Bytes(vec![0u8; 40])),
                    ("private", Value::Int(1)),
                ]),
            ),
        ]))
    }

    #[test]
    fn a_single_file_torrent_parses_into_one_file() {
        let meta = Metainfo::parse(&single_file()).unwrap();
        assert_eq!(meta.info().name, "payload.bin");
        assert_eq!(meta.info().piece_length, 1024);
        assert_eq!(meta.info().pieces.len(), 3);
        assert!(!meta.info().multi_file);
        assert_eq!(meta.info().files.len(), 1);
        assert_eq!(meta.info().files[0].path, ["payload.bin"]);
        assert_eq!(meta.info().total_length(), 3000);
    }

    #[test]
    fn a_multi_file_torrent_parses_every_file_in_order() {
        let meta = Metainfo::parse(&multi_file()).unwrap();
        assert!(meta.info().multi_file);
        assert!(meta.info().private);
        assert_eq!(meta.info().files.len(), 2);
        assert_eq!(meta.info().files[0].path, ["disc 1", "a.flac"]);
        assert_eq!(meta.info().total_length(), 2000);
    }

    #[test]
    fn the_layout_matches_the_metainfo() {
        let layout = Metainfo::parse(&multi_file()).unwrap().layout();
        assert_eq!(layout.name, "album");
        assert!(layout.multi_file);
        assert_eq!(layout.total_length, 2000);
        assert_eq!(layout.piece_count(), 2);
        assert_eq!(layout.file(1).unwrap().offset, 1500);
    }

    #[test]
    fn trackers_read_from_either_key() {
        let single = Metainfo::parse(&single_file()).unwrap();
        assert_eq!(
            single.announce().as_deref(),
            Some("udp://tracker.example.com:80")
        );
        assert_eq!(
            single.announce_tiers(),
            vec![vec!["udp://tracker.example.com:80".to_string()]]
        );

        let multi = Metainfo::parse(&multi_file()).unwrap();
        assert_eq!(multi.announce_tiers().len(), 2);
        assert_eq!(multi.announce_tiers()[0].len(), 2);
        assert_eq!(multi.trackers().len(), 3);
    }

    #[test]
    fn a_url_list_is_read_whether_it_is_a_string_or_a_list() {
        let as_list = Metainfo::parse(&single_file()).unwrap();
        assert_eq!(as_list.url_list(), vec!["https://e.com/pub/".to_string()]);

        let mut torrent = bencode::decode(&single_file()).unwrap();
        torrent.as_dict_mut().unwrap().insert(
            b"url-list".to_vec(),
            Value::text("https://only.example.com/"),
        );
        let as_string = Metainfo::parse(&bencode::encode(&torrent)).unwrap();
        assert_eq!(
            as_string.url_list(),
            vec!["https://only.example.com/".to_string()]
        );
    }

    #[test]
    fn the_info_hash_is_the_sha1_of_the_info_dictionary_bytes() {
        let meta = Metainfo::parse(&single_file()).unwrap();
        assert_eq!(meta.info_hash(), InfoHash::of(meta.info_bytes()));
        assert_eq!(meta.info_hash().hex().len(), 40);
    }

    #[test]
    fn writing_an_unedited_torrent_reproduces_it_byte_for_byte() {
        let original = single_file();
        let meta = Metainfo::parse(&original).unwrap();
        assert_eq!(meta.write_to_vec().unwrap(), original);
    }

    #[test]
    fn editing_fields_outside_info_keeps_the_info_hash() {
        let mut meta = Metainfo::parse(&single_file()).unwrap();
        let before = meta.info_hash();

        meta.set(
            "url-list",
            Some(Value::List(vec![
                Value::text("https://mirror-a.example.com/pub/"),
                Value::text("https://mirror-b.example.com/pub/"),
            ])),
        )
        .unwrap();
        meta.set("comment", Some(Value::text("edited"))).unwrap();
        meta.set("creation date", None).unwrap();
        meta.set(
            "httpseeds",
            Some(Value::List(vec![Value::text("https://old.example.com/")])),
        )
        .unwrap();

        let written = meta.write_to_vec().unwrap();
        let reread = Metainfo::parse(&written).unwrap();
        assert_eq!(
            reread.info_hash(),
            before,
            "the info hash must survive an edit"
        );
        assert_eq!(reread.url_list().len(), 2);
        assert_eq!(reread.comment().as_deref(), Some("edited"));
        assert!(reread.creation_date().is_none());
        assert_eq!(reread.http_seeds().len(), 1);
    }

    #[test]
    fn the_info_dictionary_cannot_be_edited_through_set() {
        let mut meta = Metainfo::parse(&single_file()).unwrap();
        let err = meta.set("info", Some(Value::Int(1))).unwrap_err();
        assert_eq!(err.code(), crate::exit::ExitCode::WouldChangeInfoHash);
    }

    #[test]
    fn a_non_canonical_info_encoding_still_round_trips() {
        // Hand-built with `piece length` before `name`, which is not sorted
        // order. A re-encode would reorder it and change the hash; splicing
        // the original bytes does not.
        let torrent = b"d8:announce3:foo4:infod12:piece lengthi1024e4:name3:bin6:lengthi1024e6:pieces20:00000000000000000000ee";
        let meta = Metainfo::parse(torrent).unwrap();
        let expected = InfoHash::of(
            b"d12:piece lengthi1024e4:name3:bin6:lengthi1024e6:pieces20:00000000000000000000e",
        );
        assert_eq!(meta.info_hash(), expected);
        let written = meta.write_to_vec().unwrap();
        assert_eq!(Metainfo::parse(&written).unwrap().info_hash(), expected);
    }

    #[test]
    fn a_torrent_whose_piece_count_disagrees_with_its_length_is_refused() {
        let bad = bencode::encode(&dict(vec![(
            "info",
            dict(vec![
                ("length", Value::Int(3000)),
                ("name", Value::text("x")),
                ("piece length", Value::Int(1024)),
                // Two pieces where three are needed.
                ("pieces", Value::Bytes(vec![0u8; 40])),
            ]),
        )]));
        let err = Metainfo::parse(&bad).unwrap_err();
        assert!(err.message().contains("needs 3"), "{}", err.message());
        assert_eq!(err.context()["expected_pieces"], 3);
    }

    #[test]
    fn a_pieces_field_that_is_not_a_multiple_of_twenty_is_refused() {
        let bad = bencode::encode(&dict(vec![(
            "info",
            dict(vec![
                ("length", Value::Int(1024)),
                ("name", Value::text("x")),
                ("piece length", Value::Int(1024)),
                ("pieces", Value::Bytes(vec![0u8; 19])),
            ]),
        )]));
        assert!(
            Metainfo::parse(&bad)
                .unwrap_err()
                .message()
                .contains("multiple of 20")
        );
    }

    #[test]
    fn a_torrent_describing_no_data_is_refused() {
        let bad = bencode::encode(&dict(vec![(
            "info",
            dict(vec![
                ("name", Value::text("x")),
                ("piece length", Value::Int(1024)),
                ("pieces", Value::Bytes(Vec::new())),
            ]),
        )]));
        assert!(
            Metainfo::parse(&bad)
                .unwrap_err()
                .message()
                .contains("describes no data")
        );
    }

    #[test]
    fn garbage_is_refused_with_a_source_resolution_code() {
        let err = Metainfo::parse(b"this is not bencode").unwrap_err();
        assert_eq!(err.code(), crate::exit::ExitCode::SourceResolution);
    }

    #[test]
    fn info_hashes_parse_from_hex_and_base32() {
        let hex = "0102030405060708090a0b0c0d0e0f1011121314";
        let hash = InfoHash::parse(hex).unwrap();
        assert_eq!(hash.hex(), hex);
        assert_eq!(hash.0[0], 1);
        assert_eq!(hash.0[19], 0x14);

        // The same hash, base32 encoded.
        let base32 = "AEBAGBAFAYDQQCIKBMGA2DQPCAIREEYU";
        assert_eq!(InfoHash::parse(base32).unwrap(), hash);
        assert_eq!(InfoHash::parse(&base32.to_lowercase()).unwrap(), hash);
    }

    #[test]
    fn a_bad_info_hash_says_what_was_expected() {
        for bad in ["", "abc", "z".repeat(40).as_str(), "1".repeat(39).as_str()] {
            let err = InfoHash::parse(bad).unwrap_err();
            assert_eq!(err.code(), crate::exit::ExitCode::SourceResolution);
        }
    }

    #[test]
    fn padding_files_are_recognised() {
        let padding = InfoFile {
            path: vec![".pad".into()],
            length: 100,
            attr: Some("p".into()),
            md5sum: None,
        };
        assert!(padding.is_padding());
        let real = InfoFile {
            path: vec!["a.bin".into()],
            length: 100,
            attr: None,
            md5sum: None,
        };
        assert!(!real.is_padding());
    }

    #[test]
    fn nodes_render_as_host_and_port() {
        let mut torrent = bencode::decode(&single_file()).unwrap();
        torrent.as_dict_mut().unwrap().insert(
            b"nodes".to_vec(),
            Value::List(vec![Value::List(vec![
                Value::text("dht.example.com"),
                Value::Int(6881),
            ])]),
        );
        let meta = Metainfo::parse(&bencode::encode(&torrent)).unwrap();
        assert_eq!(meta.nodes(), vec!["dht.example.com:6881".to_string()]);
    }

    #[test]
    fn creation_date_reads_as_an_iso_timestamp() {
        let meta = Metainfo::parse(&single_file()).unwrap();
        assert_eq!(
            meta.creation_date().unwrap().iso(),
            "2026-08-19T11:52:03.000Z"
        );
    }
}
