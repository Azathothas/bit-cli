//! Where a torrent's bytes are written.
//!
//! `bit-cli` supplies its own storage rather than the session's default for
//! one reason: a `.torrent` is untrusted input, and the default joins its file
//! names onto the output directory as given. On Windows that is enough to
//! leave the output directory entirely, because `Path::new("D:/out").join("C:")`
//! is `C:`. See [`crate::paths`] for the three ways it goes wrong.
//!
//! This storage runs every path through [`crate::paths::plan`] first, so:
//!
//! - every file lands inside the output directory, always;
//! - a name that cannot exist on the filesystem is rewritten rather than
//!   failing the download;
//! - two names that collide on a case-insensitive filesystem both land, under
//!   distinct names, instead of one overwriting the other;
//! - the mapping is recorded and reported, so a caller can reconcile the names
//!   it asked for with the names on disk.
//!
//! Everything else, the positioned reads and writes, matches what the session
//! expects: reads and writes are addressed by file index and offset, never by
//! a cursor, so many pieces can be in flight against one file at once.

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::IoSlice;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use librqbit::storage::{BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage};
use librqbit::{ManagedTorrentShared, TorrentMetadata};
use librqbit_core::lengths::ValidPieceIndex;

use crate::paths::{PathPlan, plan};

/// Builds a [`SafeStorage`] for one torrent.
///
/// One factory per `add`, because the output directory is per-torrent. The
/// plan it produces is shared with whoever created it, so the caller can
/// report the renames without reaching into the session.
#[derive(Clone)]
pub struct SafeStorageFactory {
    output_folder: PathBuf,
    overwrite: bool,
    subfolder: bool,
    plan: Arc<OnceLock<PathPlan>>,
}

impl SafeStorageFactory {
    /// A factory writing under `output_folder`.
    ///
    /// `overwrite` matches the session's own flag: with it, an existing file
    /// is opened and written through, which is what makes a resumed download
    /// work. Without it, an existing file is an error, so a run cannot
    /// silently destroy data it did not create.
    ///
    /// `subfolder` follows the session's rule for the directory a torrent
    /// unpacks into: a multi-file torrent goes into a directory named after
    /// it, and a caller that named an output directory explicitly gets exactly
    /// that directory. It is set when the caller did not name one.
    pub fn new(output_folder: impl Into<PathBuf>, overwrite: bool, subfolder: bool) -> Self {
        Self {
            output_folder: output_folder.into(),
            overwrite,
            subfolder,
            plan: Arc::new(OnceLock::new()),
        }
    }

    /// A handle on the plan this factory will produce.
    ///
    /// Taken before the factory is handed to the session, because that is the
    /// last moment the caller owns it. The plan appears once the metadata has
    /// resolved and storage has been created, and never for a `list_only` add
    /// that opens nothing.
    pub fn plan_handle(&self) -> PlanHandle {
        PlanHandle(self.plan.clone())
    }
}

/// A read handle on the path plan of one torrent.
#[derive(Clone, Default)]
pub struct PlanHandle(Arc<OnceLock<PathPlan>>);

impl PlanHandle {
    /// The plan, or `None` if storage has not been created yet.
    pub fn get(&self) -> Option<&PathPlan> {
        self.0.get()
    }
}

impl std::fmt::Debug for PlanHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PlanHandle").field(&self.0.get()).finish()
    }
}

impl StorageFactory for SafeStorageFactory {
    type Storage = SafeStorage;

    fn create(
        &self,
        _shared: &ManagedTorrentShared,
        metadata: &TorrentMetadata,
    ) -> anyhow::Result<SafeStorage> {
        let torrent_paths: Vec<String> = metadata
            .file_infos
            .iter()
            .map(|file| slash_path(&file.relative_filename))
            .collect();
        let planned = plan(&torrent_paths);
        // `set` fails only if storage is created twice for the same torrent,
        // which happens when it is paused and resumed. The plan is a pure
        // function of the file list, so the second one is the same as the
        // first and the first is kept.
        let _ = self.plan.set(planned.clone());

        let root = match self.subfolder.then(|| subfolder_for(metadata)).flatten() {
            Some(name) => join_relative(&self.output_folder, &name),
            None => self.output_folder.clone(),
        };

        Ok(SafeStorage {
            root,
            overwrite: self.overwrite,
            disk_paths: planned.disk_paths,
            padding: metadata
                .file_infos
                .iter()
                .map(|file| file.attrs.padding)
                .collect(),
            files: Vec::new(),
        })
    }

    fn clone_box(&self) -> BoxStorageFactory {
        self.clone().boxed()
    }
}

/// One torrent's files, opened at planned paths.
pub struct SafeStorage {
    root: PathBuf,
    overwrite: bool,
    /// Relative, `/`-separated, one per file, from the plan.
    disk_paths: Vec<String>,
    /// BEP 47 padding files hold no data and are never opened.
    padding: Vec<bool>,
    files: Vec<Slot>,
}

/// One open payload file.
///
/// The lock guards the handle, not the file position: every read and write is
/// positioned, so several can be in flight against one file at once. It exists
/// so `take` can close the handles when the torrent is paused without
/// invalidating the indices the session addresses files by.
#[derive(Default)]
struct Slot {
    path: PathBuf,
    file: RwLock<Option<File>>,
}

impl Slot {
    fn with<T>(
        &self,
        what: &str,
        f: impl FnOnce(&File) -> std::io::Result<T>,
    ) -> anyhow::Result<T> {
        let guard = self
            .file
            .read()
            .map_err(|_| anyhow::anyhow!("the lock on {} is poisoned", self.path.display()))?;
        let file = guard.as_ref().ok_or_else(|| {
            anyhow::anyhow!("{} is closed, so it cannot be {what}", self.path.display())
        })?;
        f(file).map_err(|e| anyhow::anyhow!("cannot {what} {}: {e}", self.path.display()))
    }

    /// Move the handle out, leaving the slot closed.
    fn take(&self) -> anyhow::Result<Self> {
        let mut guard = self
            .file
            .write()
            .map_err(|_| anyhow::anyhow!("the lock on {} is poisoned", self.path.display()))?;
        Ok(Self {
            path: self.path.clone(),
            file: RwLock::new(guard.take()),
        })
    }
}

impl SafeStorage {
    fn slot(&self, file_id: usize) -> anyhow::Result<&Slot> {
        self.files
            .get(file_id)
            .ok_or_else(|| anyhow::anyhow!("no file with index {file_id} in this torrent"))
    }
}

impl TorrentStorage for SafeStorage {
    fn init(
        &mut self,
        _shared: &ManagedTorrentShared,
        _metadata: &TorrentMetadata,
    ) -> anyhow::Result<()> {
        // Directories are created once each rather than once per file. A
        // torrent with many files in one directory is the common case, and
        // `create_dir_all` per file is a syscall per file for no reason.
        let mut made = BTreeSet::new();
        let mut files = Vec::with_capacity(self.disk_paths.len());

        for (index, relative) in self.disk_paths.iter().enumerate() {
            if self.padding.get(index).copied().unwrap_or(false) {
                // A BEP 47 padding file is alignment, not data. Nothing reads
                // or writes it, so nothing opens it.
                files.push(Slot::default());
                continue;
            }
            let path = join_relative(&self.root, relative);
            if let Some(parent) = path.parent()
                && made.insert(parent.to_path_buf())
            {
                std::fs::create_dir_all(parent).map_err(|e| {
                    anyhow::anyhow!("cannot create the directory {}: {e}", parent.display())
                })?;
            }
            let file = open(&path, self.overwrite)?;
            files.push(Slot {
                path,
                file: RwLock::new(Some(file)),
            });
        }

        self.files = files;
        Ok(())
    }

    fn pread_exact(&self, file_id: usize, offset: u64, buf: &mut [u8]) -> anyhow::Result<()> {
        self.slot(file_id)?
            .with("read", |file| pread_exact(file, offset, buf))
    }

    fn pwrite_all(&self, file_id: usize, offset: u64, buf: &[u8]) -> anyhow::Result<()> {
        self.slot(file_id)?
            .with("write to", |file| pwrite_all(file, offset, buf))
    }

    fn pwrite_all_vectored(
        &self,
        file_id: usize,
        offset: u64,
        bufs: [IoSlice<'_>; 2],
    ) -> anyhow::Result<usize> {
        self.slot(file_id)?.with("write to", |file| {
            let mut at = offset;
            let mut written = 0;
            for slice in bufs {
                if slice.is_empty() {
                    continue;
                }
                pwrite_all(file, at, &slice)?;
                at += slice.len() as u64;
                written += slice.len();
            }
            Ok(written)
        })
    }

    fn remove_file(&self, file_id: usize, _filename: &Path) -> anyhow::Result<()> {
        // The name the session passes is the torrent's, which is exactly the
        // name that may not exist on disk. The index is what identifies the
        // file here.
        let slot = self.slot(file_id)?;
        if slot.path.as_os_str().is_empty() {
            return Ok(());
        }
        std::fs::remove_file(&slot.path)
            .map_err(|e| anyhow::anyhow!("cannot remove {}: {e}", slot.path.display()))
    }

    fn remove_directory_if_empty(&self, path: &Path) -> anyhow::Result<()> {
        // Same problem as `remove_file`, without an index to resolve it. The
        // directory is derived from the planned paths instead, and a directory
        // that is not there is not an error: this is cleanup.
        let wanted = slash_path(path);
        let Some(planned) = self.planned_directory(&wanted) else {
            return Ok(());
        };
        let full = join_relative(&self.root, &planned);
        if !full.is_dir() {
            return Ok(());
        }
        if std::fs::read_dir(&full)?.next().is_none() {
            std::fs::remove_dir(&full)
                .map_err(|e| anyhow::anyhow!("cannot remove {}: {e}", full.display()))?;
        }
        Ok(())
    }

    fn ensure_file_length(&self, file_id: usize, length: u64) -> anyhow::Result<()> {
        if self.padding.get(file_id).copied().unwrap_or(false) {
            return Ok(());
        }
        self.slot(file_id)?
            .with("set the length of", |file| file.set_len(length))
    }

    fn take(&self) -> anyhow::Result<Box<dyn TorrentStorage>> {
        Ok(Box::new(Self {
            root: self.root.clone(),
            overwrite: self.overwrite,
            disk_paths: self.disk_paths.clone(),
            padding: self.padding.clone(),
            files: self
                .files
                .iter()
                .map(Slot::take)
                .collect::<anyhow::Result<Vec<_>>>()?,
        }))
    }

    fn on_piece_completed(&self, _piece: ValidPieceIndex) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SafeStorage {
    /// The planned directory matching a torrent-relative directory path.
    ///
    /// A directory only exists on disk because a file was planned into it, so
    /// the answer comes from the plan rather than from another sanitising
    /// pass, which would have to reproduce the collision suffixes exactly.
    fn planned_directory(&self, torrent_relative: &str) -> Option<String> {
        let depth = torrent_relative
            .split('/')
            .filter(|s| !s.is_empty())
            .count();
        if depth == 0 {
            return None;
        }
        self.disk_paths.iter().find_map(|disk| {
            let components: Vec<&str> = disk.split('/').collect();
            (components.len() > depth).then(|| components[..depth].join("/"))
        })
    }
}

/// The directory a torrent unpacks into, under the output directory.
///
/// This follows the session's own rule so the two agree on where a torrent
/// lands: a single-file torrent gets no directory of its own, and a multi-file
/// one gets a directory named after the torrent, falling back to the stem of
/// its largest file when the name is missing. The difference is that the name
/// is planned like any other path first, because a torrent called `CON` is a
/// torrent that cannot be written on Windows.
fn subfolder_for(metadata: &TorrentMetadata) -> Option<String> {
    if metadata.file_infos.len() < 2 {
        return None;
    }
    let name = metadata
        .info
        .name()
        .map(|name| name.into_owned())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            let largest = metadata.file_infos.iter().max_by_key(|file| file.len)?;
            Path::new(&largest.relative_filename)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })?;
    Some(crate::paths::plan_one(&name))
}

/// Open one payload file, creating it if it is not there.
fn open(path: &Path, overwrite: bool) -> anyhow::Result<File> {
    if !overwrite && path.exists() {
        return Err(anyhow::anyhow!(
            "{} already exists. Pass --allow-overwrite to write through it, or --continue to resume into it",
            path.display()
        ));
    }
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| anyhow::anyhow!("cannot open {}: {e}", path.display()))
}

/// Join a planned relative path onto the output directory.
///
/// The components are pushed one at a time on purpose. `join` on a whole
/// string would hand the platform's path parser something it might read as a
/// root, and although [`crate::paths::plan`] has already made that impossible,
/// this is the line where it would matter.
fn join_relative(root: &Path, relative: &str) -> PathBuf {
    let mut out = root.to_path_buf();
    for component in relative.split('/').filter(|c| !c.is_empty()) {
        out.push(component);
    }
    out
}

/// Render a path as the `/`-separated form the rest of the crate uses.
fn slash_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(unix)]
fn pread_exact(file: &File, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buf, offset)
}

/// Windows has no `pread`. `seek_read` takes the offset per call and may
/// return a short read, so the loop is what makes it exact. A short read that
/// returns nothing is end of file, and treating that as a read of zeroes is
/// how a hash check passes over a file that is not there.
#[cfg(windows)]
fn pread_exact(file: &File, mut offset: u64, mut buf: &mut [u8]) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match file.seek_read(buf, offset)? {
            0 => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "the file ended before the read was satisfied",
                ));
            }
            read => {
                offset += read as u64;
                buf = &mut buf[read..];
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn pwrite_all(file: &File, offset: u64, buf: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.write_all_at(buf, offset)
}

#[cfg(windows)]
fn pwrite_all(file: &File, mut offset: u64, mut buf: &[u8]) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match file.seek_write(buf, offset)? {
            0 => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "the write made no progress",
                ));
            }
            written => {
                offset += written as u64;
                buf = &buf[written..];
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joining_a_relative_path_stays_under_the_root() {
        let root = Path::new("out");
        assert_eq!(
            join_relative(root, "a/b.bin"),
            Path::new("out").join("a").join("b.bin")
        );
    }

    #[test]
    fn joining_ignores_empty_components() {
        assert_eq!(
            join_relative(Path::new("out"), "a//b.bin"),
            Path::new("out").join("a").join("b.bin")
        );
    }

    #[test]
    fn a_planned_path_can_never_escape_the_root() {
        // Every path the plan produces, joined one component at a time, stays
        // under the root. This is the property the whole module rests on.
        let torrent = [
            "C:/pwned.txt",
            "C:",
            "../../etc/passwd",
            "/abs/x",
            "//server/share/y",
            "CON",
            "a<b.bin",
            "x .",
        ];
        let planned = plan(&torrent.iter().map(|p| (*p).to_string()).collect::<Vec<_>>());
        let root = Path::new("D:").join("out");
        for relative in &planned.disk_paths {
            let full = join_relative(&root, relative);
            assert!(
                full.starts_with(&root),
                "{relative} escaped to {}",
                full.display()
            );
        }
    }

    #[test]
    fn a_windows_style_path_renders_with_forward_slashes() {
        let path: PathBuf = ["disc 1", "a.flac"].iter().collect();
        assert_eq!(slash_path(&path), "disc 1/a.flac");
    }

    #[test]
    fn positioned_writes_and_reads_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("payload.bin");
        let file = open(&path, true).unwrap();
        file.set_len(16).unwrap();

        pwrite_all(&file, 8, b"second").unwrap();
        pwrite_all(&file, 0, b"first").unwrap();

        let mut buf = [0u8; 6];
        pread_exact(&file, 8, &mut buf).unwrap();
        assert_eq!(&buf, b"second");
        let mut buf = [0u8; 5];
        pread_exact(&file, 0, &mut buf).unwrap();
        assert_eq!(&buf, b"first");
    }

    #[test]
    fn reading_past_the_end_is_an_error_not_a_read_of_zeroes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("short.bin");
        let file = open(&path, true).unwrap();
        pwrite_all(&file, 0, b"four").unwrap();

        let mut buf = [0u8; 8];
        let err = pread_exact(&file, 0, &mut buf).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn opening_an_existing_file_without_overwrite_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("there.bin");
        std::fs::write(&path, b"existing").unwrap();

        let err = open(&path, false).unwrap_err().to_string();
        assert!(err.contains("already exists"), "{err}");
        assert!(err.contains("--allow-overwrite"), "{err}");

        // With overwrite the existing bytes are still there: opening does not
        // truncate, which is what makes resuming into a partial file work.
        let file = open(&path, true).unwrap();
        let mut buf = [0u8; 8];
        pread_exact(&file, 0, &mut buf).unwrap();
        assert_eq!(&buf, b"existing");
    }

    #[test]
    fn a_closed_slot_reports_which_file_it_was() {
        let slot = Slot {
            path: PathBuf::from("gone.bin"),
            file: RwLock::new(None),
        };
        let err = slot.with("read", |_| Ok(())).unwrap_err().to_string();
        assert!(err.contains("gone.bin"), "{err}");
        assert!(err.contains("closed"), "{err}");
    }

    #[test]
    fn taking_a_slot_leaves_the_original_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("payload.bin");
        let slot = Slot {
            path: path.clone(),
            file: RwLock::new(Some(open(&path, true).unwrap())),
        };

        let taken = slot.take().unwrap();
        assert!(taken.file.read().unwrap().is_some());
        assert!(slot.file.read().unwrap().is_none());
        assert!(slot.with("read", |_| Ok(())).is_err());
    }

    #[test]
    fn a_directory_is_resolved_through_the_plan_not_the_torrent() {
        let storage = SafeStorage {
            root: PathBuf::from("out"),
            overwrite: true,
            disk_paths: vec!["CON_/a.bin".into(), "CON_/b.bin".into()],
            padding: vec![false, false],
            files: Vec::new(),
        };
        assert_eq!(storage.planned_directory("CON"), Some("CON_".to_string()));
        assert_eq!(storage.planned_directory(""), None);
    }
}
