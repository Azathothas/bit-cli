use std::fmt;

use crate::bencode::{self, Value};
use crate::sha1;
use crate::sha256;

type DictEntries = Vec<(Vec<u8>, Value)>;
type InfoSpan = Option<(usize, usize)>;
type PieceLayerHashes = Vec<(Vec<u8>, Vec<[u8; 32]>)>;

/// The transfer engine keeps several complete pieces in memory per peer. A
/// bound prevents hostile metainfo from turning that design into a multi-GB
/// allocation while retaining compatibility with large modern torrents.
pub const MAX_PIECE_LENGTH: u64 = 16 * 1024 * 1024;

#[derive(Debug)]
pub struct TorrentMeta {
    pub announce: Option<Vec<u8>>,
    pub announce_list: Vec<Vec<Vec<u8>>>,
    #[cfg_attr(not(feature = "webseed"), allow(dead_code))]
    pub url_list: Vec<Vec<u8>>,
    #[cfg_attr(not(feature = "webseed"), allow(dead_code))]
    pub httpseeds: Vec<Vec<u8>>,
    pub info: InfoDict,
    pub info_hash: [u8; 20],
    #[allow(dead_code)]
    pub info_hash_v2: Option<[u8; 32]>,
    pub piece_layers: PieceLayerHashes,
    pub meta_version: u8,
}

#[derive(Debug)]
pub struct InfoDict {
    pub name: Vec<u8>,
    pub piece_length: u64,
    pub pieces: Vec<[u8; 20]>,
    pub length: Option<u64>,
    pub files: Vec<FileInfo>,
    pub private: bool,
    pub file_tree: Vec<FileTreeEntry>,
}

#[derive(Debug)]
pub struct FileInfo {
    pub length: u64,
    pub path: Vec<Vec<u8>>,
    /// BEP 47 file attribute flags. In particular, `p` marks padding files in
    /// hybrid torrents and is required to validate v1/v2 file alignment.
    pub attr: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FileTreeEntry {
    pub path: Vec<Vec<u8>>,
    pub length: u64,
    pub pieces_root: Option<[u8; 32]>,
}

impl TorrentMeta {
    /// Logical offsets used by piece I/O for each stored file. V1 and hybrid
    /// files are contiguous (hybrid padding files provide alignment); v2-only
    /// non-empty files begin on piece boundaries.
    pub fn file_offsets(&self) -> Option<Vec<u64>> {
        if self.info.length.is_some() {
            return Some(vec![0]);
        }

        let lengths: Vec<u64> = if !self.info.files.is_empty() {
            self.info.files.iter().map(|file| file.length).collect()
        } else {
            self.info.file_tree.iter().map(|file| file.length).collect()
        };
        let mut offsets = Vec::with_capacity(lengths.len());
        let mut offset = 0u64;
        for length in lengths {
            if self.meta_version == 2 && length > 0 {
                offset = align_up(offset, self.info.piece_length)?;
            }
            offsets.push(offset);
            offset = offset.checked_add(length)?;
        }
        Some(offsets)
    }
}

impl InfoDict {
    pub fn total_length(&self) -> u64 {
        if let Some(length) = self.length {
            length
        } else if !self.files.is_empty() {
            self.files
                .iter()
                .fold(0u64, |total, file| total.saturating_add(file.length))
        } else {
            self.file_tree
                .iter()
                .fold(0u64, |total, entry| total.saturating_add(entry.length))
        }
    }

    pub fn checked_total_length(&self) -> Option<u64> {
        if let Some(length) = self.length {
            Some(length)
        } else if !self.files.is_empty() {
            self.files
                .iter()
                .try_fold(0u64, |total, file| total.checked_add(file.length))
        } else {
            self.file_tree
                .iter()
                .try_fold(0u64, |total, entry| total.checked_add(entry.length))
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Bencode(bencode::Error),
    MissingField(&'static str),
    InvalidField(&'static str),
    InvalidType(&'static str),
    InvalidPiecesLength,
    TrailingData,
    InvalidAnnounceList,
    InvalidUrlList,
    InvalidHttpSeeds,
    InvalidFileTree,
    InvalidPieceLayers,
    UnsupportedMetaVersion(i64),
    LengthOverflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Bencode(err) => write!(f, "bencode error: {err}"),
            Error::MissingField(field) => write!(f, "missing field: {field}"),
            Error::InvalidField(field) => write!(f, "invalid field: {field}"),
            Error::InvalidType(field) => write!(f, "invalid type for {field}"),
            Error::InvalidPiecesLength => write!(f, "pieces length is not a multiple of 20"),
            Error::TrailingData => write!(f, "trailing data after torrent dictionary"),
            Error::InvalidAnnounceList => write!(f, "invalid announce-list"),
            Error::InvalidUrlList => write!(f, "invalid url-list"),
            Error::InvalidHttpSeeds => write!(f, "invalid httpseeds"),
            Error::InvalidFileTree => write!(f, "invalid v2 file tree"),
            Error::InvalidPieceLayers => write!(f, "invalid v2 piece layers"),
            Error::UnsupportedMetaVersion(version) => {
                write!(f, "unsupported torrent meta version: {version}")
            }
            Error::LengthOverflow => write!(f, "torrent length exceeds supported range"),
        }
    }
}

impl std::error::Error for Error {}

impl From<bencode::Error> for Error {
    fn from(err: bencode::Error) -> Self {
        Error::Bencode(err)
    }
}

pub fn parse_torrent(data: &[u8]) -> Result<TorrentMeta, Error> {
    let (top_dict, info_span) = parse_top_dict(data)?;
    let info_span = info_span.ok_or(Error::MissingField("info"))?;
    let announce = optional_bytes(&top_dict, b"announce", "announce")?;
    let announce_list = match dict_get(&top_dict, b"announce-list") {
        Some(value) => parse_announce_list(value)?,
        None => Vec::new(),
    };
    let url_list = match dict_get(&top_dict, b"url-list") {
        Some(value) => parse_url_list(value)?,
        None => Vec::new(),
    };
    let httpseeds = match dict_get(&top_dict, b"httpseeds") {
        Some(value) => parse_httpseeds(value)?,
        None => Vec::new(),
    };
    let info_value = dict_get(&top_dict, b"info").ok_or(Error::MissingField("info"))?;
    let info_dict = as_dict_named(info_value, "info")?;
    let declared_meta_version = match dict_get(info_dict, b"meta version") {
        None => None,
        Some(Value::Int(2)) => Some(2u8),
        Some(Value::Int(version)) => return Err(Error::UnsupportedMetaVersion(*version)),
        Some(_) => return Err(Error::InvalidType("meta version")),
    };
    let has_v2 = declared_meta_version == Some(2);
    let has_v1 = dict_get(info_dict, b"pieces").is_some();
    let info = parse_info_dict(info_value, has_v2, has_v1)?;
    let info_bytes = &data[info_span.0..info_span.1];

    let piece_layers_value = dict_get(&top_dict, b"piece layers");
    let piece_layers = match piece_layers_value {
        Some(value) => parse_piece_layers(value)?,
        None => Vec::new(),
    };

    if has_v2 {
        if piece_layers_value.is_none() {
            return Err(Error::MissingField("piece layers"));
        }
        validate_v2_piece_layers(&info, &piece_layers)?;
        if has_v1 {
            validate_hybrid_layout(&info)?;
        }
    } else if piece_layers_value.is_some() {
        return Err(Error::InvalidPieceLayers);
    }

    let meta_version = if has_v2 && has_v1 {
        3 // hybrid
    } else if has_v2 {
        2 // v2-only
    } else {
        1 // v1
    };

    let v2_hash = has_v2.then(|| sha256::sha256(info_bytes));
    let info_hash = if meta_version == 2 {
        let hash = v2_hash.ok_or(Error::MissingField("meta version"))?;
        let mut truncated = [0u8; 20];
        truncated.copy_from_slice(&hash[..20]);
        truncated
    } else {
        sha1::sha1(info_bytes)
    };

    Ok(TorrentMeta {
        announce,
        announce_list,
        url_list,
        httpseeds,
        info,
        info_hash,
        info_hash_v2: v2_hash,
        piece_layers,
        meta_version,
    })
}

fn parse_top_dict(data: &[u8]) -> Result<(DictEntries, InfoSpan), Error> {
    if data.first() != Some(&b'd') {
        return Err(Error::InvalidType("top-level dictionary"));
    }

    let mut dict = Vec::new();
    let mut pos = 1;
    let mut info_span = None;
    let mut previous_key: Option<Vec<u8>> = None;
    let mut terminated = false;

    while pos < data.len() {
        if data[pos] == b'e' {
            pos += 1;
            terminated = true;
            break;
        }
        let (key_value, next) = bencode::parse_value(data, pos)?;
        let key = match key_value {
            Value::Bytes(bytes) => bytes,
            _ => return Err(Error::InvalidType("dictionary key")),
        };
        if previous_key
            .as_ref()
            .is_some_and(|previous| previous.as_slice() >= key.as_slice())
        {
            return Err(bencode::Error::InvalidDictOrder.into());
        }
        pos = next;
        let value_start = pos;
        let (value, next) = bencode::parse_value(data, pos)?;
        pos = next;
        if key == b"info" {
            info_span = Some((value_start, pos));
        }
        previous_key = Some(key.clone());
        dict.push((key, value));
    }

    if !terminated {
        return Err(bencode::Error::UnexpectedEof.into());
    }
    if pos != data.len() {
        return Err(Error::TrailingData);
    }

    Ok((dict, info_span))
}

fn parse_info_dict(value: &Value, has_v2: bool, has_v1: bool) -> Result<InfoDict, Error> {
    let dict = as_dict_named(value, "info")?;
    let name = required_bytes(dict, b"name", "name")?;
    if name.is_empty() {
        return Err(Error::InvalidField("name"));
    }
    let piece_length = required_int(dict, b"piece length", "piece length")?;
    if piece_length == 0 || piece_length > MAX_PIECE_LENGTH {
        return Err(Error::InvalidField("piece length"));
    }
    if has_v2 && (piece_length < 16 * 1024 || !piece_length.is_power_of_two()) {
        return Err(Error::InvalidField("piece length"));
    }

    let file_tree = match dict_get(dict, b"file tree") {
        Some(ft_value) if has_v2 => parse_file_tree(ft_value)?,
        Some(_) => return Err(Error::InvalidField("file tree/meta version")),
        None if has_v2 => return Err(Error::MissingField("file tree")),
        None => Vec::new(),
    };

    let pieces = match dict_get(dict, b"pieces") {
        Some(Value::Bytes(pieces_bytes)) => {
            if pieces_bytes.len() % 20 != 0 {
                return Err(Error::InvalidPiecesLength);
            }
            let mut pieces = Vec::with_capacity(pieces_bytes.len() / 20);
            for chunk in pieces_bytes.chunks_exact(20) {
                let mut hash = [0u8; 20];
                hash.copy_from_slice(chunk);
                pieces.push(hash);
            }
            pieces
        }
        Some(_) => return Err(Error::InvalidType("pieces")),
        None if has_v1 => unreachable!("presence was checked before parsing"),
        None if !has_v2 => return Err(Error::MissingField("pieces")),
        None => Vec::new(),
    };

    let length = optional_int(dict, b"length", "length")?;
    let files_value = dict_get(dict, b"files");
    let files = if let Some(files_value) = files_value {
        parse_files(files_value)?
    } else {
        Vec::new()
    };
    let private = optional_int(dict, b"private", "private")?
        .map(|value| value != 0)
        .unwrap_or(false);

    let has_length = dict_get(dict, b"length").is_some();
    let has_files = files_value.is_some();
    if has_v1 && has_length == has_files {
        return Err(Error::InvalidField("length/files"));
    }
    if !has_v1 && (has_length || has_files) {
        return Err(Error::InvalidField("v1 layout without pieces"));
    }

    let total_length = if has_v1 {
        checked_v1_length(length, &files)?
    } else {
        file_tree
            .iter()
            .try_fold(0u64, |total, entry| total.checked_add(entry.length))
            .ok_or(Error::LengthOverflow)?
    };
    if total_length == 0 {
        return Err(Error::InvalidField("total length"));
    }
    if has_v1 {
        let expected_pieces = total_length.div_ceil(piece_length);
        if u64::try_from(pieces.len()).ok() != Some(expected_pieces) {
            return Err(Error::InvalidField("pieces count"));
        }
    }

    Ok(InfoDict {
        name,
        piece_length,
        pieces,
        length,
        files,
        private,
        file_tree,
    })
}

fn parse_files(value: &Value) -> Result<Vec<FileInfo>, Error> {
    let list = as_list_named(value, "files")?;
    let mut files = Vec::with_capacity(list.len());

    for entry in list {
        let dict = as_dict_named(entry, "file")?;
        let length = required_int(dict, b"length", "length")?;
        let path_value = dict_get(dict, b"path").ok_or(Error::MissingField("path"))?;
        let path_list = as_list_named(path_value, "path")?;
        if path_list.is_empty() {
            return Err(Error::InvalidField("path"));
        }
        let mut path = Vec::with_capacity(path_list.len());
        for segment in path_list {
            match segment {
                Value::Bytes(bytes) if !bytes.is_empty() => path.push(bytes.clone()),
                Value::Bytes(_) => return Err(Error::InvalidField("path segment")),
                _ => return Err(Error::InvalidType("path segment")),
            }
        }
        let attr = optional_bytes(dict, b"attr", "attr")?.unwrap_or_default();
        files.push(FileInfo { length, path, attr });
    }

    Ok(files)
}

fn parse_announce_list(value: &Value) -> Result<Vec<Vec<Vec<u8>>>, Error> {
    let list = match value {
        Value::List(items) => items,
        _ => return Err(Error::InvalidAnnounceList),
    };
    let mut tiers = Vec::with_capacity(list.len());
    for tier in list {
        let tier_list = match tier {
            Value::List(items) => items,
            _ => return Err(Error::InvalidAnnounceList),
        };
        let mut urls = Vec::with_capacity(tier_list.len());
        for entry in tier_list {
            match entry {
                Value::Bytes(bytes) => urls.push(bytes.clone()),
                _ => return Err(Error::InvalidAnnounceList),
            }
        }
        tiers.push(urls);
    }
    Ok(tiers)
}

fn parse_url_list(value: &Value) -> Result<Vec<Vec<u8>>, Error> {
    match value {
        Value::Bytes(bytes) => Ok(vec![bytes.clone()]),
        Value::List(items) => {
            let mut urls = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Bytes(bytes) => urls.push(bytes.clone()),
                    _ => return Err(Error::InvalidUrlList),
                }
            }
            Ok(urls)
        }
        _ => Err(Error::InvalidUrlList),
    }
}

fn parse_httpseeds(value: &Value) -> Result<Vec<Vec<u8>>, Error> {
    let list = match value {
        Value::List(items) => items,
        _ => return Err(Error::InvalidHttpSeeds),
    };
    let mut urls = Vec::with_capacity(list.len());
    for item in list {
        match item {
            Value::Bytes(bytes) => urls.push(bytes.clone()),
            _ => return Err(Error::InvalidHttpSeeds),
        }
    }
    Ok(urls)
}

fn parse_file_tree(value: &Value) -> Result<Vec<FileTreeEntry>, Error> {
    let dict = as_dict_named(value, "file tree")?;
    let mut entries = Vec::new();
    parse_file_tree_recursive(dict, &mut Vec::new(), &mut entries)?;
    if entries.is_empty() {
        return Err(Error::InvalidFileTree);
    }
    Ok(entries)
}

fn parse_file_tree_recursive(
    dict: &[(Vec<u8>, Value)],
    path: &mut Vec<Vec<u8>>,
    entries: &mut Vec<FileTreeEntry>,
) -> Result<(), Error> {
    if dict.is_empty() {
        return Err(Error::InvalidFileTree);
    }
    if let Some((_, properties)) = dict.iter().find(|(key, _)| key.is_empty()) {
        // A file property dictionary is the sole child of a file node. The
        // root itself may never be a file, and files cannot have siblings.
        if path.is_empty() || dict.len() != 1 {
            return Err(Error::InvalidFileTree);
        }
        let properties = as_dict_named(properties, "file properties")?;
        let length = required_int(properties, b"length", "length")?;
        let pieces_root = match dict_get(properties, b"pieces root") {
            Some(Value::Bytes(bytes)) if bytes.len() == 32 => {
                let mut root = [0u8; 32];
                root.copy_from_slice(bytes);
                Some(root)
            }
            Some(Value::Bytes(_)) => return Err(Error::InvalidField("pieces root")),
            Some(_) => return Err(Error::InvalidType("pieces root")),
            None if length > 0 => return Err(Error::MissingField("pieces root")),
            None => None,
        };
        entries.push(FileTreeEntry {
            path: path.clone(),
            length,
            pieces_root,
        });
        return Ok(());
    }

    for (key, value) in dict {
        if key.is_empty() {
            unreachable!("empty key handled before traversal");
        }
        let inner = as_dict_named(value, "file tree node")?;
        path.push(key.clone());
        parse_file_tree_recursive(inner, path, entries)?;
        path.pop();
    }
    Ok(())
}

fn parse_piece_layers(value: &Value) -> Result<PieceLayerHashes, Error> {
    let dict = as_dict_named(value, "piece layers")?;
    let mut layers = Vec::with_capacity(dict.len());
    for (key, value) in dict {
        if key.len() != 32 {
            return Err(Error::InvalidPieceLayers);
        }
        let Value::Bytes(hashes_bytes) = value else {
            return Err(Error::InvalidType("piece layer"));
        };
        if hashes_bytes.is_empty() || hashes_bytes.len() % 32 != 0 {
            return Err(Error::InvalidPieceLayers);
        }
        let mut hashes = Vec::with_capacity(hashes_bytes.len() / 32);
        for chunk in hashes_bytes.chunks_exact(32) {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(chunk);
            hashes.push(hash);
        }
        layers.push((key.clone(), hashes));
    }
    Ok(layers)
}

fn validate_v2_piece_layers(info: &InfoDict, layers: &PieceLayerHashes) -> Result<(), Error> {
    let piece_length = info.piece_length as u32;

    for entry in &info.file_tree {
        if entry.length <= info.piece_length {
            continue;
        }
        let root = entry.pieces_root.ok_or(Error::InvalidFileTree)?;
        let (_, hashes) = layers
            .iter()
            .find(|(key, _)| key.as_slice() == root.as_slice())
            .ok_or(Error::InvalidPieceLayers)?;
        let expected_count = entry.length.div_ceil(info.piece_length);
        if u64::try_from(hashes.len()).ok() != Some(expected_count) {
            return Err(Error::InvalidPieceLayers);
        }
        let reconstructed = sha256::merkle_root_from_piece_layer(hashes, piece_length)
            .ok_or(Error::InvalidPieceLayers)?;
        if reconstructed != root {
            return Err(Error::InvalidPieceLayers);
        }
    }

    // Piece layers are not an extension bucket: every entry must correspond to
    // a file which actually needs a layer.
    for (key, _) in layers {
        let referenced = info.file_tree.iter().any(|entry| {
            entry.length > info.piece_length
                && entry
                    .pieces_root
                    .is_some_and(|root| root.as_slice() == key.as_slice())
        });
        if !referenced {
            return Err(Error::InvalidPieceLayers);
        }
    }
    Ok(())
}

fn validate_hybrid_layout(info: &InfoDict) -> Result<(), Error> {
    if let Some(length) = info.length {
        if info.file_tree.len() != 1
            || info.file_tree[0].path.as_slice() != [info.name.clone()].as_slice()
            || info.file_tree[0].length != length
        {
            return Err(Error::InvalidField("hybrid file layout"));
        }
        return Ok(());
    }

    let mut offset = 0u64;
    let mut tree_index = 0usize;
    for file in &info.files {
        if file.attr.contains(&b'p') {
            let remainder = offset % info.piece_length;
            let expected = if remainder == 0 {
                0
            } else {
                info.piece_length - remainder
            };
            if expected == 0 || file.length != expected {
                return Err(Error::InvalidField("hybrid padding file"));
            }
        } else {
            if file.length > 0 && !offset.is_multiple_of(info.piece_length) {
                return Err(Error::InvalidField("hybrid file alignment"));
            }
            let tree_entry = info
                .file_tree
                .get(tree_index)
                .ok_or(Error::InvalidField("hybrid file layout"))?;
            if tree_entry.path != file.path || tree_entry.length != file.length {
                return Err(Error::InvalidField("hybrid file layout"));
            }
            tree_index += 1;
        }
        offset = offset
            .checked_add(file.length)
            .ok_or(Error::LengthOverflow)?;
    }
    if tree_index != info.file_tree.len() {
        return Err(Error::InvalidField("hybrid file layout"));
    }
    Ok(())
}

fn checked_v1_length(length: Option<u64>, files: &[FileInfo]) -> Result<u64, Error> {
    if let Some(length) = length {
        return Ok(length);
    }
    files
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.length))
        .ok_or(Error::LengthOverflow)
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    if alignment == 0 {
        return None;
    }
    let remainder = value % alignment;
    if remainder == 0 {
        Some(value)
    } else {
        value.checked_add(alignment - remainder)
    }
}

fn dict_get<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter()
        .find_map(|(k, v)| if k.as_slice() == key { Some(v) } else { None })
}

fn required_bytes(
    dict: &[(Vec<u8>, Value)],
    key: &[u8],
    field: &'static str,
) -> Result<Vec<u8>, Error> {
    optional_bytes(dict, key, field)?.ok_or(Error::MissingField(field))
}

fn optional_bytes(
    dict: &[(Vec<u8>, Value)],
    key: &[u8],
    field: &'static str,
) -> Result<Option<Vec<u8>>, Error> {
    match dict_get(dict, key) {
        Some(Value::Bytes(bytes)) => Ok(Some(bytes.clone())),
        Some(_) => Err(Error::InvalidType(field)),
        None => Ok(None),
    }
}

fn required_int(dict: &[(Vec<u8>, Value)], key: &[u8], field: &'static str) -> Result<u64, Error> {
    optional_int(dict, key, field)?.ok_or(Error::MissingField(field))
}

fn optional_int(
    dict: &[(Vec<u8>, Value)],
    key: &[u8],
    field: &'static str,
) -> Result<Option<u64>, Error> {
    match dict_get(dict, key) {
        Some(Value::Int(num)) if *num >= 0 => Ok(Some(*num as u64)),
        Some(Value::Int(_)) => Err(Error::InvalidField(field)),
        Some(_) => Err(Error::InvalidType(field)),
        None => Ok(None),
    }
}

fn as_dict_named<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a [(Vec<u8>, Value)], Error> {
    match value {
        Value::Dict(items) => Ok(items),
        _ => Err(Error::InvalidType(field)),
    }
}

fn as_list_named<'a>(value: &'a Value, field: &'static str) -> Result<&'a [Value], Error> {
    match value {
        Value::List(items) => Ok(items),
        _ => Err(Error::InvalidType(field)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v2_file_tree(name: &[u8], length: u64, root: [u8; 32]) -> Value {
        Value::Dict(vec![(
            name.to_vec(),
            Value::Dict(vec![(
                Vec::new(),
                Value::Dict(vec![
                    (b"length".to_vec(), Value::Int(length as i64)),
                    (b"pieces root".to_vec(), Value::Bytes(root.to_vec())),
                ]),
            )]),
        )])
    }

    fn valid_v2_torrent() -> (Vec<u8>, [u8; 32], Vec<[u8; 32]>) {
        let first = sha256::sha256(&vec![b'a'; 16 * 1024]);
        let second = sha256::sha256(b"end");
        let hashes = vec![first, second];
        let root = sha256::merkle_root_from_piece_layer(&hashes, 16 * 1024).unwrap();
        let info = Value::Dict(vec![
            (
                b"file tree".to_vec(),
                v2_file_tree(b"file.bin", 16 * 1024 + 3, root),
            ),
            (b"meta version".to_vec(), Value::Int(2)),
            (b"name".to_vec(), Value::Bytes(b"bundle".to_vec())),
            (b"piece length".to_vec(), Value::Int(16 * 1024)),
        ]);
        let mut layer_bytes = Vec::new();
        for hash in &hashes {
            layer_bytes.extend_from_slice(hash);
        }
        let top = Value::Dict(vec![
            (b"info".to_vec(), info),
            (
                b"piece layers".to_vec(),
                Value::Dict(vec![(root.to_vec(), Value::Bytes(layer_bytes))]),
            ),
        ]);
        (bencode::encode(&top), root, hashes)
    }

    fn valid_info_dict() -> Value {
        Value::Dict(vec![
            (b"name".to_vec(), Value::Bytes(b"sample".to_vec())),
            (b"piece length".to_vec(), Value::Int(16)),
            (b"pieces".to_vec(), Value::Bytes(vec![7u8; 20])),
            (b"length".to_vec(), Value::Int(16)),
        ])
    }

    #[test]
    fn parse_torrent_extracts_fields_and_info_hash() {
        let info = valid_info_dict();
        let top = Value::Dict(vec![
            (
                b"announce".to_vec(),
                Value::Bytes(b"http://tracker/a".to_vec()),
            ),
            (
                b"announce-list".to_vec(),
                Value::List(vec![
                    Value::List(vec![Value::Bytes(b"http://tracker/a".to_vec())]),
                    Value::List(vec![
                        Value::Bytes(b"http://tracker/b".to_vec()),
                        Value::Bytes(b"http://tracker/c".to_vec()),
                    ]),
                ]),
            ),
            (
                b"url-list".to_vec(),
                Value::List(vec![
                    Value::Bytes(b"http://seed/one".to_vec()),
                    Value::Bytes(b"http://seed/two".to_vec()),
                ]),
            ),
            (
                b"httpseeds".to_vec(),
                Value::List(vec![Value::Bytes(b"http://legacy-seed".to_vec())]),
            ),
            (b"info".to_vec(), info),
        ]);
        let data = bencode::encode(&top);
        let (_, span) = parse_top_dict(&data).unwrap();
        let (start, end) = span.unwrap();
        let expected_hash = sha1::sha1(&data[start..end]);

        let meta = parse_torrent(&data).unwrap();
        assert_eq!(meta.announce, Some(b"http://tracker/a".to_vec()));
        assert_eq!(
            meta.announce_list,
            vec![
                vec![b"http://tracker/a".to_vec()],
                vec![b"http://tracker/b".to_vec(), b"http://tracker/c".to_vec(),],
            ]
        );
        assert_eq!(
            meta.url_list,
            vec![b"http://seed/one".to_vec(), b"http://seed/two".to_vec()]
        );
        assert_eq!(meta.httpseeds, vec![b"http://legacy-seed".to_vec()]);
        assert_eq!(meta.info_hash, expected_hash);
        assert_eq!(meta.info.total_length(), 16);
    }

    #[test]
    fn parse_torrent_rejects_missing_info() {
        let data = bencode::encode(&Value::Dict(vec![(
            b"announce".to_vec(),
            Value::Bytes(b"http://tracker".to_vec()),
        )]));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::MissingField("info"))
        ));
    }

    #[test]
    fn parse_torrent_rejects_invalid_pieces_len() {
        let info = Value::Dict(vec![
            (b"name".to_vec(), Value::Bytes(b"x".to_vec())),
            (b"piece length".to_vec(), Value::Int(4)),
            (b"pieces".to_vec(), Value::Bytes(vec![1u8; 19])),
            (b"length".to_vec(), Value::Int(4)),
        ]);
        let data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), info)]));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::InvalidPiecesLength)
        ));
    }

    #[test]
    fn parse_torrent_rejects_length_and_files_together() {
        let info = Value::Dict(vec![
            (b"name".to_vec(), Value::Bytes(b"root".to_vec())),
            (b"piece length".to_vec(), Value::Int(16)),
            (b"pieces".to_vec(), Value::Bytes(vec![9u8; 20])),
            (b"length".to_vec(), Value::Int(16)),
            (
                b"files".to_vec(),
                Value::List(vec![Value::Dict(vec![
                    (b"length".to_vec(), Value::Int(16)),
                    (
                        b"path".to_vec(),
                        Value::List(vec![Value::Bytes(b"a.bin".to_vec())]),
                    ),
                ])]),
            ),
        ]);
        let data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), info)]));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::InvalidField("length/files"))
        ));
    }

    #[test]
    fn parse_torrent_rejects_invalid_announce_list_type() {
        let info = valid_info_dict();
        let top = Value::Dict(vec![
            (b"info".to_vec(), info),
            (b"announce-list".to_vec(), Value::List(vec![Value::Int(1)])),
        ]);
        let data = bencode::encode(&top);
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::InvalidAnnounceList)
        ));
    }

    #[test]
    fn parse_torrent_rejects_trailing_data() {
        let info = valid_info_dict();
        let mut data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), info)]));
        data.extend_from_slice(b"junk");
        assert!(matches!(parse_torrent(&data), Err(Error::TrailingData)));
    }

    #[test]
    fn parse_torrent_rejects_unterminated_top_dictionary() {
        let info = valid_info_dict();
        let mut data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), info)]));
        data.pop();
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::Bencode(bencode::Error::UnexpectedEof))
        ));
    }

    #[test]
    fn parse_torrent_validates_piece_length_and_exact_piece_count() {
        let zero_piece_length = Value::Dict(vec![
            (b"length".to_vec(), Value::Int(1)),
            (b"name".to_vec(), Value::Bytes(b"x".to_vec())),
            (b"piece length".to_vec(), Value::Int(0)),
            (b"pieces".to_vec(), Value::Bytes(vec![0; 20])),
        ]);
        let data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), zero_piece_length)]));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::InvalidField("piece length"))
        ));

        let wrong_count = Value::Dict(vec![
            (b"length".to_vec(), Value::Int(17)),
            (b"name".to_vec(), Value::Bytes(b"x".to_vec())),
            (b"piece length".to_vec(), Value::Int(16)),
            (b"pieces".to_vec(), Value::Bytes(vec![0; 20])),
        ]);
        let data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), wrong_count)]));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::InvalidField("pieces count"))
        ));
    }

    #[test]
    fn parse_torrent_rejects_empty_file_paths_and_length_overflow() {
        let empty_path = Value::Dict(vec![
            (
                b"files".to_vec(),
                Value::List(vec![Value::Dict(vec![
                    (b"length".to_vec(), Value::Int(1)),
                    (b"path".to_vec(), Value::List(Vec::new())),
                ])]),
            ),
            (b"name".to_vec(), Value::Bytes(b"root".to_vec())),
            (b"piece length".to_vec(), Value::Int(16)),
            (b"pieces".to_vec(), Value::Bytes(vec![0; 20])),
        ]);
        let data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), empty_path)]));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::InvalidField("path"))
        ));

        let huge_file = || {
            Value::Dict(vec![
                (b"length".to_vec(), Value::Int(i64::MAX)),
                (
                    b"path".to_vec(),
                    Value::List(vec![Value::Bytes(b"x".to_vec())]),
                ),
            ])
        };
        let overflowing = Value::Dict(vec![
            (
                b"files".to_vec(),
                Value::List(vec![huge_file(), huge_file(), huge_file()]),
            ),
            (b"name".to_vec(), Value::Bytes(b"root".to_vec())),
            (b"piece length".to_vec(), Value::Int(16)),
            (b"pieces".to_vec(), Value::Bytes(Vec::new())),
        ]);
        let data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), overflowing)]));
        assert!(matches!(parse_torrent(&data), Err(Error::LengthOverflow)));
    }

    #[test]
    fn parses_and_validates_v2_hashes_and_piece_layers() {
        let (data, _, _) = valid_v2_torrent();
        let (_, span) = parse_top_dict(&data).unwrap();
        let (start, end) = span.unwrap();
        let full_v2_hash = sha256::sha256(&data[start..end]);

        let meta = parse_torrent(&data).unwrap();
        assert_eq!(meta.meta_version, 2);
        assert_eq!(meta.info_hash_v2, Some(full_v2_hash));
        assert_eq!(meta.info_hash, full_v2_hash[..20]);
        assert_eq!(meta.info.file_tree.len(), 1);
        assert_eq!(meta.piece_layers[0].1.len(), 2);
        assert_eq!(meta.file_offsets(), Some(vec![0]));
    }

    #[test]
    fn rejects_tampered_v2_piece_layer_and_unsupported_version() {
        let (data, _, _) = valid_v2_torrent();
        let mut top = match bencode::parse(&data).unwrap() {
            Value::Dict(entries) => entries,
            _ => unreachable!(),
        };
        let layers = top
            .iter_mut()
            .find(|(key, _)| key == b"piece layers")
            .map(|(_, value)| value)
            .unwrap();
        let Value::Dict(entries) = layers else {
            unreachable!();
        };
        let Value::Bytes(bytes) = &mut entries[0].1 else {
            unreachable!();
        };
        bytes[0] ^= 1;
        let data = bencode::encode(&Value::Dict(top));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::InvalidPieceLayers)
        ));

        let info = Value::Dict(vec![
            (b"meta version".to_vec(), Value::Int(99)),
            (b"name".to_vec(), Value::Bytes(b"future".to_vec())),
        ]);
        let data = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), info)]));
        assert!(matches!(
            parse_torrent(&data),
            Err(Error::UnsupportedMetaVersion(99))
        ));
    }

    #[test]
    fn v2_requires_piece_layers_dictionary_even_when_empty() {
        let root = sha256::sha256(b"small");
        let info = Value::Dict(vec![
            (b"file tree".to_vec(), v2_file_tree(b"small.bin", 5, root)),
            (b"meta version".to_vec(), Value::Int(2)),
            (b"name".to_vec(), Value::Bytes(b"small".to_vec())),
            (b"piece length".to_vec(), Value::Int(16 * 1024)),
        ]);
        let without_layers = bencode::encode(&Value::Dict(vec![(b"info".to_vec(), info.clone())]));
        assert!(matches!(
            parse_torrent(&without_layers),
            Err(Error::MissingField("piece layers"))
        ));

        let with_empty_layers = bencode::encode(&Value::Dict(vec![
            (b"info".to_vec(), info),
            (b"piece layers".to_vec(), Value::Dict(Vec::new())),
        ]));
        assert_eq!(parse_torrent(&with_empty_layers).unwrap().meta_version, 2);
    }

    #[test]
    fn parses_consistent_hybrid_layout_with_padding() {
        let first_root = sha256::sha256(b"abc");
        let second_root = sha256::sha256(b"hello");
        let file_tree = Value::Dict(vec![
            (
                b"a".to_vec(),
                Value::Dict(vec![(
                    Vec::new(),
                    Value::Dict(vec![
                        (b"length".to_vec(), Value::Int(3)),
                        (b"pieces root".to_vec(), Value::Bytes(first_root.to_vec())),
                    ]),
                )]),
            ),
            (
                b"b".to_vec(),
                Value::Dict(vec![(
                    Vec::new(),
                    Value::Dict(vec![
                        (b"length".to_vec(), Value::Int(5)),
                        (b"pieces root".to_vec(), Value::Bytes(second_root.to_vec())),
                    ]),
                )]),
            ),
        ]);
        let file = |path: &[u8], length: i64, attr: Option<&[u8]>| {
            let mut fields = vec![
                (b"length".to_vec(), Value::Int(length)),
                (
                    b"path".to_vec(),
                    Value::List(vec![Value::Bytes(path.to_vec())]),
                ),
            ];
            if let Some(attr) = attr {
                fields.push((b"attr".to_vec(), Value::Bytes(attr.to_vec())));
            }
            Value::Dict(fields)
        };
        let info = Value::Dict(vec![
            (b"file tree".to_vec(), file_tree),
            (
                b"files".to_vec(),
                Value::List(vec![
                    file(b"a", 3, None),
                    file(b"pad", 16 * 1024 - 3, Some(b"p")),
                    file(b"b", 5, None),
                ]),
            ),
            (b"meta version".to_vec(), Value::Int(2)),
            (b"name".to_vec(), Value::Bytes(b"hybrid".to_vec())),
            (b"piece length".to_vec(), Value::Int(16 * 1024)),
            (b"pieces".to_vec(), Value::Bytes(vec![7; 40])),
        ]);
        let data = bencode::encode(&Value::Dict(vec![
            (b"info".to_vec(), info),
            (b"piece layers".to_vec(), Value::Dict(Vec::new())),
        ]));
        let meta = parse_torrent(&data).unwrap();
        assert_eq!(meta.meta_version, 3);
        assert!(meta.info_hash_v2.is_some());
        assert_eq!(meta.info.files[1].attr, b"p");
        assert_eq!(meta.file_offsets(), Some(vec![0, 3, 16 * 1024]));
    }
}
