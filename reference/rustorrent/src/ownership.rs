use std::env;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::storage;
use crate::torrent::TorrentMeta;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimKind {
    File,
    Tree,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageClaim {
    path: PathBuf,
    comparison_path: PathBuf,
    kind: ClaimKind,
}

impl StorageClaim {
    pub fn new(path: PathBuf, kind: ClaimKind) -> Result<Self, String> {
        let path = normalize_absolute_path(&path)?;
        let comparison_path = comparison_path(&path);
        Ok(Self {
            path,
            comparison_path,
            kind,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(test)]
    fn kind(&self) -> ClaimKind {
        self.kind
    }
}

/// Build every path that must remain exclusively owned by one torrent.
///
/// Single-file torrents own their effective file path. Multi-file torrents,
/// including v2 torrents whose file tree happens to contain one file, own the
/// entire metainfo root because completion moves and cleanup operate on that
/// root as a unit. Pending rename and completion destinations are retained as
/// additional claims so recovery cannot race another torrent into either
/// side of an unfinished transaction.
pub fn claims_for_torrent(
    meta: &TorrentMeta,
    download_dir: &Path,
    file_renames: &[(usize, String)],
    pending_file_rename: Option<(usize, &str)>,
    pending_completion_dir: Option<&Path>,
) -> Result<Vec<StorageClaim>, String> {
    let mut claims = Vec::new();

    if meta.info.length.is_some() {
        let mut rename_variants = vec![file_renames.to_vec()];
        if let Some((index, target)) = pending_file_rename {
            let mut prospective = file_renames.to_vec();
            if let Some((_, current)) = prospective
                .iter_mut()
                .find(|(current_index, _)| *current_index == index)
            {
                *current = target.to_string();
            } else {
                prospective.push((index, target.to_string()));
                prospective.sort_unstable_by_key(|(current_index, _)| *current_index);
            }
            rename_variants.push(prospective);
        }

        for renames in rename_variants {
            let source = storage::data_paths_with_file_renames(meta, download_dir, &renames)
                .map_err(|err| format!("storage ownership path: {err}"))?
                .into_iter()
                .next()
                .ok_or_else(|| "single-file torrent has no storage path".to_string())?;
            push_claim(
                &mut claims,
                StorageClaim::new(source.clone(), ClaimKind::File)?,
            );
            if let Some(destination_dir) = pending_completion_dir {
                let name = source
                    .file_name()
                    .ok_or_else(|| "single-file ownership path has no file name".to_string())?;
                push_claim(
                    &mut claims,
                    StorageClaim::new(destination_dir.join(name), ClaimKind::File)?,
                );
            }
        }
    } else {
        let source = storage::root_path(meta, download_dir)
            .map_err(|err| format!("storage ownership root: {err}"))?;
        push_claim(&mut claims, StorageClaim::new(source, ClaimKind::Tree)?);
        if let Some(destination_dir) = pending_completion_dir {
            let destination = storage::root_path(meta, destination_dir)
                .map_err(|err| format!("completion ownership root: {err}"))?;
            push_claim(
                &mut claims,
                StorageClaim::new(destination, ClaimKind::Tree)?,
            );
        }

        // The tree claim already covers every committed or pending file name.
        // Still validate rename variants here so a malformed journal cannot
        // bypass ownership checks and fail only after another claim is saved.
        storage::data_paths_with_file_renames(meta, download_dir, file_renames)
            .map_err(|err| format!("storage ownership paths: {err}"))?;
        if let Some((index, target)) = pending_file_rename {
            let mut prospective = file_renames.to_vec();
            if let Some((_, current)) = prospective
                .iter_mut()
                .find(|(current_index, _)| *current_index == index)
            {
                *current = target.to_string();
            } else {
                prospective.push((index, target.to_string()));
                prospective.sort_unstable_by_key(|(current_index, _)| *current_index);
            }
            storage::data_paths_with_file_renames(meta, download_dir, &prospective)
                .map_err(|err| format!("pending rename ownership paths: {err}"))?;
        }
    }

    Ok(claims)
}

pub fn claims_conflict(left: &StorageClaim, right: &StorageClaim) -> bool {
    match (left.kind, right.kind) {
        (ClaimKind::File, ClaimKind::File) => left.comparison_path == right.comparison_path,
        _ => {
            left.comparison_path.starts_with(&right.comparison_path)
                || right.comparison_path.starts_with(&left.comparison_path)
        }
    }
}

fn push_claim(claims: &mut Vec<StorageClaim>, claim: StorageClaim) {
    if !claims.iter().any(|current| {
        current.kind == claim.kind && current.comparison_path == claim.comparison_path
    }) {
        claims.push(claim);
    }
}

fn normalize_absolute_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|err| format!("storage ownership current directory: {err}"))?
            .join(path)
    };

    // Resolve the deepest path prefix that exists. This catches aliases
    // through symlinked download directories while retaining the exact tail
    // for payload files and directories that have not been created yet.
    let components = absolute.components().collect::<Vec<_>>();
    for split in (1..=components.len()).rev() {
        let mut prefix = PathBuf::new();
        for component in &components[..split] {
            prefix.push(component.as_os_str());
        }
        match fs::canonicalize(&prefix) {
            Ok(mut resolved) => {
                for component in &components[split..] {
                    match component {
                        Component::Normal(_) => resolved.push(component.as_os_str()),
                        Component::CurDir => {}
                        Component::ParentDir => {
                            if !resolved.pop() {
                                return Err(format!(
                                    "storage ownership path escapes the filesystem root: {}",
                                    absolute.display()
                                ));
                            }
                        }
                        Component::Prefix(_) | Component::RootDir => {
                            return Err("invalid absolute storage ownership path".to_string());
                        }
                    }
                }
                return Ok(lexical_normalize(&resolved));
            }
            Err(err)
                if err.kind() == io::ErrorKind::NotFound
                    || err.kind() == io::ErrorKind::NotADirectory => {}
            Err(err) => {
                return Err(format!(
                    "canonicalize storage ownership path {}: {err}",
                    prefix.display()
                ));
            }
        }
    }
    Err(format!(
        "storage ownership path has no existing ancestor: {}",
        absolute.display()
    ))
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
        }
    }
    normalized
}

#[cfg(any(target_os = "macos", windows))]
fn comparison_path(path: &Path) -> PathBuf {
    let mut folded = PathBuf::new();
    for component in path.components() {
        let value = component.as_os_str().to_string_lossy().to_lowercase();
        folded.push(value);
    }
    folded
}

#[cfg(not(any(target_os = "macos", windows)))]
fn comparison_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::torrent::{FileInfo, InfoDict, TorrentMeta};
    use std::ffi::OsString;

    fn single(name: &[u8]) -> TorrentMeta {
        TorrentMeta {
            announce: None,
            announce_list: Vec::new(),
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info: InfoDict {
                name: name.to_vec(),
                piece_length: 16_384,
                pieces: Vec::new(),
                length: Some(1),
                files: Vec::new(),
                private: false,
                file_tree: Vec::new(),
            },
            info_hash: [0; 20],
            info_hash_v2: None,
            piece_layers: Vec::new(),
            meta_version: 1,
        }
    }

    fn multi(name: &[u8]) -> TorrentMeta {
        TorrentMeta {
            announce: None,
            announce_list: Vec::new(),
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info: InfoDict {
                name: name.to_vec(),
                piece_length: 16_384,
                pieces: Vec::new(),
                length: None,
                files: vec![FileInfo {
                    length: 1,
                    path: vec![b"file.bin".to_vec()],
                    attr: Vec::new(),
                }],
                private: false,
                file_tree: Vec::new(),
            },
            info_hash: [0; 20],
            info_hash_v2: None,
            piece_layers: Vec::new(),
            meta_version: 1,
        }
    }

    fn base() -> PathBuf {
        env::temp_dir().join("rustorrent-ownership-tests")
    }

    #[test]
    fn different_torrents_with_the_same_single_file_name_conflict() {
        let meta = single(b"same.bin");
        let left = claims_for_torrent(&meta, &base(), &[], None, None).unwrap();
        let right = claims_for_torrent(&meta, &base(), &[], None, None).unwrap();
        assert!(claims_conflict(&left[0], &right[0]));
    }

    #[test]
    fn multifile_torrents_with_the_same_root_conflict() {
        let meta = multi(b"bundle");
        let left = claims_for_torrent(&meta, &base(), &[], None, None).unwrap();
        let right = claims_for_torrent(&meta, &base(), &[], None, None).unwrap();
        assert_eq!(left[0].kind(), ClaimKind::Tree);
        assert!(claims_conflict(&left[0], &right[0]));
    }

    #[test]
    fn file_inside_a_claimed_tree_conflicts() {
        let tree = claims_for_torrent(&multi(b"bundle"), &base(), &[], None, None).unwrap();
        let file = claims_for_torrent(
            &single(b"file.bin"),
            &base().join("bundle"),
            &[],
            None,
            None,
        )
        .unwrap();
        assert!(claims_conflict(&tree[0], &file[0]));
    }

    #[test]
    fn sibling_paths_do_not_conflict() {
        let left = claims_for_torrent(&single(b"left.bin"), &base(), &[], None, None).unwrap();
        let right = claims_for_torrent(&single(b"right.bin"), &base(), &[], None, None).unwrap();
        assert!(!claims_conflict(&left[0], &right[0]));
    }

    #[test]
    fn pending_rename_and_completion_keep_both_sides_claimed() {
        let meta = single(b"old.bin");
        let destination = base().join("complete");
        let claims = claims_for_torrent(
            &meta,
            &base(),
            &[],
            Some((0, "new.bin")),
            Some(&destination),
        )
        .unwrap();
        let paths = claims
            .iter()
            .map(|claim| claim.path().file_name().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(claims.len(), 4);
        assert!(paths.contains(&OsString::from("old.bin")));
        assert!(paths.contains(&OsString::from("new.bin")));
        assert!(claims.iter().any(|claim| {
            claim.path().parent().and_then(Path::file_name)
                == Some(std::ffi::OsStr::new("complete"))
        }));
    }

    #[test]
    fn existing_ancestor_aliases_are_canonicalized() {
        let root = env::current_dir().unwrap();
        let direct = StorageClaim::new(root.join("future/one.bin"), ClaimKind::File).unwrap();
        let dotted =
            StorageClaim::new(root.join("future/../future/one.bin"), ClaimKind::File).unwrap();
        assert!(claims_conflict(&direct, &dotted));
    }

    #[cfg(unix)]
    #[test]
    fn parent_components_after_symlinks_follow_filesystem_resolution() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("rustorrent-ownership-symlink-{unique}"));
        let target = root.join("target");
        fs::create_dir_all(target.join("child")).unwrap();
        symlink(target.join("child"), root.join("alias")).unwrap();

        let through_alias = StorageClaim::new(
            root.join("alias").join("..").join("payload.bin"),
            ClaimKind::File,
        )
        .unwrap();
        let direct = StorageClaim::new(target.join("payload.bin"), ClaimKind::File).unwrap();
        assert!(claims_conflict(&through_alias, &direct));

        let _ = fs::remove_dir_all(root);
    }
}
