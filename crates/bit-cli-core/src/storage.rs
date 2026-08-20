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
//!
//! Files are opened when they are first touched, not when the torrent is
//! added. Two things follow from that. A torrent added with a file selection
//! never creates the files it was not asked for, because nothing ever touches
//! them. And a torrent with more files than the handle cap allows keeps only
//! the cap open, closing the least recently opened when it needs another, so
//! one process seeding many large torrents does not run out of descriptors.

use std::collections::{BTreeSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::IoSlice;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use librqbit::storage::{BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage};
use librqbit::{ManagedTorrentShared, TorrentMetadata};
use librqbit_core::lengths::ValidPieceIndex;

use crate::alloc::Allocation;
use crate::paths::{PathPlan, plan};

/// How many payload files stay open when no cap is given.
///
/// Chosen to sit well under the 512 stream limit a Windows CRT allows by
/// default and far under a typical Linux `RLIMIT_NOFILE` of 1024, so the
/// default never runs the process out on its own. `--max-open-files` raises or
/// lowers it.
pub const DEFAULT_MAX_OPEN_FILES: usize = 128;

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
    allocation: Allocation,
    max_open_files: usize,
    plan: Arc<OnceLock<PathPlan>>,
    notes: Arc<Mutex<Vec<String>>>,
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
            allocation: Allocation::default(),
            max_open_files: DEFAULT_MAX_OPEN_FILES,
            plan: Arc::new(OnceLock::new()),
            notes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// How space is reserved for each file.
    pub fn with_allocation(mut self, allocation: Allocation) -> Self {
        self.allocation = allocation;
        self
    }

    /// How many payload files stay open at once. Zero means the default.
    pub fn with_max_open_files(mut self, max: usize) -> Self {
        self.max_open_files = match max {
            0 => DEFAULT_MAX_OPEN_FILES,
            max => max,
        };
        self
    }

    /// Anything storage needs the caller to know: an allocation method that
    /// could not be used, and what ran instead.
    ///
    /// Reported once per distinct message rather than once per file, because a
    /// torrent with twenty thousand files would otherwise say the same thing
    /// twenty thousand times.
    pub fn notes(&self) -> Vec<String> {
        self.notes.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// A shared handle on the notes, so a caller holding it can read them
    /// after the factory has been given away to the session.
    pub fn notes_handle(&self) -> Arc<Mutex<Vec<String>>> {
        self.notes.clone()
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
            allocation: self.allocation,
            disk_paths: planned.disk_paths,
            padding: metadata
                .file_infos
                .iter()
                .map(|file| file.attrs.padding)
                .collect(),
            files: Vec::new(),
            open: Mutex::new(OpenSet::new(self.max_open_files)),
            notes: self.notes.clone(),
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
    allocation: Allocation,
    /// Relative, `/`-separated, one per file, from the plan.
    disk_paths: Vec<String>,
    /// BEP 47 padding files hold no data and are never opened.
    padding: Vec<bool>,
    files: Vec<Slot>,
    open: Mutex<OpenSet>,
    notes: Arc<Mutex<Vec<String>>>,
}

/// Which files are open, in the order they were opened.
///
/// The order is by open rather than by access. Recording an access would mean
/// taking this lock on every read and write, which costs more than it saves:
/// the expensive event is opening a handle, and the least recently opened file
/// is the one least recently needed both for a download walking pieces and for
/// a seeder answering requests.
struct OpenSet {
    order: VecDeque<usize>,
    cap: usize,
}

impl OpenSet {
    fn new(cap: usize) -> Self {
        Self {
            order: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    /// Record that `file_id` is now open, and name whatever has to close to
    /// stay inside the cap.
    fn opened(&mut self, file_id: usize) -> Vec<usize> {
        self.order.retain(|id| *id != file_id);
        self.order.push_back(file_id);
        let mut evict = Vec::new();
        while self.order.len() > self.cap {
            if let Some(id) = self.order.pop_front() {
                evict.push(id);
            }
        }
        evict
    }

    fn closed(&mut self, file_id: usize) {
        self.order.retain(|id| *id != file_id);
    }

    fn len(&self) -> usize {
        self.order.len()
    }
}

/// Why a file is being opened.
///
/// A write creates the file; a read does not. The hash check reads every
/// piece of every file to learn what is on disk, and if that created files
/// then a torrent added with a selection would still get all of them. See
/// `TODO/disk-io.md`, T-013.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Read,
    Write,
}

/// One payload file, opened when it is first touched.
///
/// The lock guards the handle, not the file position: every read and write is
/// positioned, so several can be in flight against one file at once. Taking
/// the read half here is correct because the only writer is the handle swap,
/// and a positioned write is safe against another positioned write at a
/// different offset. See `TODO/disk-io.md`, T-010.
#[derive(Default)]
struct Slot {
    path: PathBuf,
    file: RwLock<Option<File>>,
}

impl Slot {
    /// Run `f` against the open handle, or report that the slot is closed.
    fn try_with<T>(
        &self,
        what: &str,
        f: impl FnOnce(&File) -> std::io::Result<T>,
    ) -> anyhow::Result<Option<T>> {
        let guard = self
            .file
            .read()
            .map_err(|_| anyhow::anyhow!("the lock on {} is poisoned", self.path.display()))?;
        let Some(file) = guard.as_ref() else {
            return Ok(None);
        };
        f(file)
            .map(Some)
            .map_err(|e| anyhow::anyhow!("cannot {what} {}: {e}", self.path.display()))
    }

    /// Close the handle, if one is open.
    fn close(&self) -> anyhow::Result<()> {
        let mut guard = self
            .file
            .write()
            .map_err(|_| anyhow::anyhow!("the lock on {} is poisoned", self.path.display()))?;
        drop(guard.take());
        Ok(())
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

    fn is_open(&self) -> bool {
        self.file
            .read()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }
}

impl SafeStorage {
    fn slot(&self, file_id: usize) -> anyhow::Result<&Slot> {
        self.files
            .get(file_id)
            .ok_or_else(|| anyhow::anyhow!("no file with index {file_id} in this torrent"))
    }

    /// Run `f` against a file, opening it first if it is not open.
    ///
    /// No slot guard is ever held while another slot's guard is taken. The
    /// eviction happens between the failed read and the open, so two threads
    /// evicting each other's file cannot deadlock.
    fn with<T>(
        &self,
        file_id: usize,
        intent: Intent,
        what: &str,
        f: impl FnOnce(&File) -> std::io::Result<T>,
    ) -> anyhow::Result<T> {
        let slot = self.slot(file_id)?;
        if slot.path.as_os_str().is_empty() {
            anyhow::bail!("file index {file_id} holds no data, so it cannot be {what}");
        }
        if !self.ensure_open(file_id, intent)? {
            // A read of a file that is not there. The hash check asks for
            // every piece of every file to learn what is already on disk, and
            // a file nothing has written is exactly the answer "nothing".
            // Reporting it as end of file rather than creating the file is
            // what keeps an unselected file off the disk.
            return Err(anyhow::anyhow!(
                "cannot {what} {}: the file has not been created yet",
                slot.path.display()
            ));
        }
        let slot = self.slot(file_id)?;
        slot.try_with(what, f)?.ok_or_else(|| {
            // The handle was evicted between the open and the call. One more
            // attempt is enough: only a cap smaller than the number of files
            // in flight can produce this, and that is worth reporting rather
            // than looping on.
            anyhow::anyhow!(
                "{} was closed while being {what}: --max-open-files is smaller than the number of files in flight",
                slot.path.display()
            )
        })
    }

    /// Make sure one file is open, closing others if the cap says so.
    ///
    /// Returns whether the file is open afterwards. A read of a file that does
    /// not exist answers `false` rather than creating it.
    fn ensure_open(&self, file_id: usize, intent: Intent) -> anyhow::Result<bool> {
        let slot = self.slot(file_id)?;
        if slot.is_open() {
            return Ok(true);
        }
        if intent == Intent::Read && !slot.path.exists() {
            return Ok(false);
        }

        let evict = {
            let mut open = self.open.lock().unwrap_or_else(|e| e.into_inner());
            open.opened(file_id)
        };
        for victim in evict {
            if victim != file_id {
                self.slot(victim)?.close()?;
            }
        }

        let mut guard = slot
            .file
            .write()
            .map_err(|_| anyhow::anyhow!("the lock on {} is poisoned", slot.path.display()))?;
        if guard.is_some() {
            // Another thread opened it while this one was evicting.
            return Ok(true);
        }
        if let Some(parent) = slot.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!("cannot create the directory {}: {e}", parent.display())
            })?;
        }
        *guard = Some(open(&slot.path, self.overwrite)?);
        Ok(true)
    }

    /// Record something the caller should be told, once.
    fn note(&self, message: String) {
        let mut notes = self.notes.lock().unwrap_or_else(|e| e.into_inner());
        if !notes.contains(&message) {
            notes.push(message);
        }
    }

    /// How many payload files are open right now. For tests and for the
    /// handle accounting a soak run reads.
    pub fn open_files(&self) -> usize {
        self.open.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

impl TorrentStorage for SafeStorage {
    fn init(
        &mut self,
        _shared: &ManagedTorrentShared,
        _metadata: &TorrentMetadata,
    ) -> anyhow::Result<()> {
        // Paths are planned here and files are opened on first use. A torrent
        // added with a file selection never touches the files it was not asked
        // for, so this is what keeps them off the disk entirely rather than
        // creating them empty. See `TODO/disk-io.md`, T-013.
        //
        // Directories are still created here, and once each rather than once
        // per file: a torrent with many files in one directory is the common
        // case, and `create_dir_all` per file is a syscall per file for no
        // reason. An empty directory a selection did not fill is cheap and
        // visible, which an empty file pretending to be payload is not.
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
            // Refusing an existing file has to happen before anything is
            // written, not on first touch, or a run without --allow-overwrite
            // would fail halfway through instead of at the start.
            if !self.overwrite && path.exists() {
                return Err(refuse_existing(&path));
            }
            files.push(Slot {
                path,
                file: RwLock::new(None),
            });
        }

        self.files = files;
        Ok(())
    }

    fn pread_exact(&self, file_id: usize, offset: u64, buf: &mut [u8]) -> anyhow::Result<()> {
        self.with(file_id, Intent::Read, "read", |file| {
            pread_exact(file, offset, buf)
        })
    }

    fn pwrite_all(&self, file_id: usize, offset: u64, buf: &[u8]) -> anyhow::Result<()> {
        self.with(file_id, Intent::Write, "write to", |file| {
            pwrite_all(file, offset, buf)
        })
    }

    fn pwrite_all_vectored(
        &self,
        file_id: usize,
        offset: u64,
        bufs: [IoSlice<'_>; 2],
    ) -> anyhow::Result<usize> {
        self.with(file_id, Intent::Write, "write to", |file| {
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
        slot.close()?;
        self.open
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .closed(file_id);
        // A file that was never touched was never created, and removing what
        // is not there is not a failure: this is cleanup.
        match std::fs::remove_file(&slot.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(anyhow::anyhow!(
                "cannot remove {}: {e}",
                slot.path.display()
            )),
        }
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

    /// Reserve space for one file, under the configured strategy.
    ///
    /// This is the call that creates a file, because it is the first thing the
    /// session does to a file it intends to use. A file the session never asks
    /// to size is a file the caller did not select, and it stays off the disk.
    fn ensure_file_length(&self, file_id: usize, length: u64) -> anyhow::Result<()> {
        if self.padding.get(file_id).copied().unwrap_or(false) {
            return Ok(());
        }
        let outcome = self.with(file_id, Intent::Write, "reserve space for", |file| {
            crate::alloc::reserve(file, length, self.allocation)
        })?;
        if !outcome.as_asked() {
            self.note(format!(
                "--file-allocation {} is not available here, so {} was used instead: {}",
                outcome.requested.as_str(),
                outcome.used.as_str(),
                outcome.note.as_deref().unwrap_or("no reason given")
            ));
        }
        Ok(())
    }

    fn take(&self) -> anyhow::Result<Box<dyn TorrentStorage>> {
        let files = self
            .files
            .iter()
            .map(Slot::take)
            .collect::<anyhow::Result<Vec<_>>>()?;
        let open = {
            let mut guard = self.open.lock().unwrap_or_else(|e| e.into_inner());
            let cap = guard.cap;
            std::mem::replace(&mut *guard, OpenSet::new(cap))
        };
        Ok(Box::new(Self {
            root: self.root.clone(),
            overwrite: self.overwrite,
            allocation: self.allocation,
            disk_paths: self.disk_paths.clone(),
            padding: self.padding.clone(),
            files,
            open: Mutex::new(open),
            notes: self.notes.clone(),
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

/// The error for a file that is already there and was not asked to be.
fn refuse_existing(path: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "{} already exists. Pass --allow-overwrite to write through it, or --continue to resume into it",
        path.display()
    )
}

/// Open one payload file, creating it if it is not there.
fn open(path: &Path, overwrite: bool) -> anyhow::Result<File> {
    if !overwrite && path.exists() {
        return Err(refuse_existing(path));
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
    fn a_closed_slot_answers_that_it_is_closed_rather_than_failing() {
        let slot = Slot {
            path: PathBuf::from("gone.bin"),
            file: RwLock::new(None),
        };
        assert_eq!(slot.try_with("read", |_| Ok(())).unwrap(), None);
        assert!(!slot.is_open());
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
        assert!(taken.is_open());
        assert!(!slot.is_open());
        assert_eq!(slot.try_with("read", |_| Ok(())).unwrap(), None);
    }

    /// A storage with no files, for the pure path logic.
    fn empty_storage(disk_paths: Vec<String>) -> SafeStorage {
        let padding = vec![false; disk_paths.len()];
        SafeStorage {
            root: PathBuf::from("out"),
            overwrite: true,
            allocation: Allocation::None,
            disk_paths,
            padding,
            files: Vec::new(),
            open: Mutex::new(OpenSet::new(DEFAULT_MAX_OPEN_FILES)),
            notes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[test]
    fn a_directory_is_resolved_through_the_plan_not_the_torrent() {
        let storage = empty_storage(vec!["CON_/a.bin".into(), "CON_/b.bin".into()]);
        assert_eq!(storage.planned_directory("CON"), Some("CON_".to_string()));
        assert_eq!(storage.planned_directory(""), None);
    }

    /// A storage rooted in a real directory, ready to be initialised.
    fn storage_at(root: &Path, paths: &[&str], cap: usize) -> SafeStorage {
        let disk_paths: Vec<String> = paths.iter().map(|p| (*p).to_string()).collect();
        let padding = vec![false; disk_paths.len()];
        let mut storage = SafeStorage {
            root: root.to_path_buf(),
            overwrite: true,
            allocation: Allocation::None,
            disk_paths: disk_paths.clone(),
            padding,
            files: Vec::new(),
            open: Mutex::new(OpenSet::new(cap)),
            notes: Arc::new(Mutex::new(Vec::new())),
        };
        storage.files = disk_paths
            .iter()
            .map(|relative| Slot {
                path: join_relative(root, relative),
                file: RwLock::new(None),
            })
            .collect();
        storage
    }

    #[test]
    fn a_file_is_created_when_it_is_first_touched_and_not_before() {
        let dir = tempfile::tempdir().unwrap();
        let storage = storage_at(dir.path(), &["a.bin", "b.bin"], 8);
        assert!(!dir.path().join("a.bin").exists());
        assert!(!dir.path().join("b.bin").exists());

        storage.pwrite_all(0, 0, b"hello").unwrap();
        assert!(dir.path().join("a.bin").exists());
        assert!(
            !dir.path().join("b.bin").exists(),
            "a file nothing touched is a file nothing creates"
        );
        assert_eq!(std::fs::read(dir.path().join("a.bin")).unwrap(), b"hello");
    }

    #[test]
    fn the_handle_cap_closes_the_least_recently_opened_file() {
        let dir = tempfile::tempdir().unwrap();
        let names: Vec<String> = (0..8).map(|i| format!("f{i}.bin")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let storage = storage_at(dir.path(), &refs, 3);

        for index in 0..8 {
            storage.pwrite_all(index, 0, b"x").unwrap();
            assert!(
                storage.open_files() <= 3,
                "{} files open with a cap of 3 after touching {index}",
                storage.open_files()
            );
        }
        // Every file was written, whatever the cap did to the handles.
        for name in &names {
            assert_eq!(std::fs::read(dir.path().join(name)).unwrap(), b"x");
        }
        assert_eq!(storage.open_files(), 3);
        // The three still open are the three most recently opened.
        for index in 0..5 {
            assert!(!storage.files[index].is_open(), "f{index} should be closed");
        }
        for index in 5..8 {
            assert!(storage.files[index].is_open(), "f{index} should be open");
        }
    }

    #[test]
    fn a_reopened_file_reads_back_what_was_written_before_it_was_closed() {
        let dir = tempfile::tempdir().unwrap();
        let storage = storage_at(dir.path(), &["a.bin", "b.bin", "c.bin"], 1);
        storage.pwrite_all(0, 0, b"first").unwrap();
        storage.pwrite_all(1, 0, b"second").unwrap();
        storage.pwrite_all(2, 0, b"third").unwrap();
        assert_eq!(storage.open_files(), 1);

        let mut buf = [0u8; 5];
        storage.pread_exact(0, 0, &mut buf).unwrap();
        assert_eq!(&buf, b"first");
    }

    /// The lock over the handle is taken by the read half on every read and
    /// write, and by the write half only when the handle is swapped. Two
    /// positioned writes at different offsets are safe against each other, so
    /// the read half is the right one. `TODO/disk-io.md`, T-010.
    #[test]
    fn concurrent_positioned_writes_to_one_file_do_not_interleave() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(storage_at(dir.path(), &["big.bin"], 4));
        const WRITERS: usize = 8;
        const CHUNK: usize = 64 * 1024;

        std::thread::scope(|scope| {
            for writer in 0..WRITERS {
                let storage = storage.clone();
                scope.spawn(move || {
                    let payload = vec![writer as u8; CHUNK];
                    for round in 0..8 {
                        let offset = ((round * WRITERS + writer) * CHUNK) as u64;
                        storage.pwrite_all(0, offset, &payload).unwrap();
                    }
                });
            }
        });

        let written = std::fs::read(dir.path().join("big.bin")).unwrap();
        assert_eq!(written.len(), 8 * WRITERS * CHUNK);
        for round in 0..8 {
            for writer in 0..WRITERS {
                let at = (round * WRITERS + writer) * CHUNK;
                let block = &written[at..at + CHUNK];
                assert!(
                    block.iter().all(|b| *b == writer as u8),
                    "round {round}, writer {writer}: a write landed inside another one"
                );
            }
        }
    }

    #[test]
    fn removing_a_file_that_was_never_touched_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let storage = storage_at(dir.path(), &["never.bin"], 4);
        storage
            .remove_file(0, Path::new("never.bin"))
            .expect("removing what is not there is cleanup, not failure");
    }

    #[test]
    fn removing_a_file_closes_its_handle_first() {
        let dir = tempfile::tempdir().unwrap();
        let storage = storage_at(dir.path(), &["gone.bin"], 4);
        storage.pwrite_all(0, 0, b"data").unwrap();
        assert_eq!(storage.open_files(), 1);
        storage.remove_file(0, Path::new("gone.bin")).unwrap();
        assert_eq!(storage.open_files(), 0);
        assert!(!dir.path().join("gone.bin").exists());
    }

    #[test]
    fn reserving_space_creates_the_file_at_the_right_length() {
        let dir = tempfile::tempdir().unwrap();
        let storage = storage_at(dir.path(), &["sized.bin"], 4);
        storage.ensure_file_length(0, 4096).unwrap();
        assert_eq!(
            std::fs::metadata(dir.path().join("sized.bin"))
                .unwrap()
                .len(),
            4096
        );
    }

    #[test]
    fn the_open_set_evicts_in_the_order_files_were_opened() {
        let mut open = OpenSet::new(2);
        assert!(open.opened(0).is_empty());
        assert!(open.opened(1).is_empty());
        assert_eq!(open.opened(2), vec![0]);
        assert_eq!(open.len(), 2);
        // Opening one that is already open moves it to the back rather than
        // counting it twice.
        assert!(open.opened(1).is_empty());
        assert_eq!(open.len(), 2);
        assert_eq!(open.opened(3), vec![2]);
    }

    #[test]
    fn a_cap_of_zero_is_read_as_one_rather_than_as_none() {
        let mut open = OpenSet::new(0);
        assert!(open.opened(0).is_empty());
        assert_eq!(open.opened(1), vec![0]);
        assert_eq!(open.len(), 1);
    }

    #[test]
    fn closing_a_file_takes_it_out_of_the_open_set() {
        let mut open = OpenSet::new(4);
        open.opened(0);
        open.opened(1);
        open.closed(0);
        assert_eq!(open.len(), 1);
        open.closed(9);
        assert_eq!(open.len(), 1, "closing one that was never open is a no-op");
    }
}
