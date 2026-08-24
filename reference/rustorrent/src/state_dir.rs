use std::ffi::OsStr;
use std::path::Path;
#[cfg(any(unix, windows))]
use std::path::PathBuf;

const STATE_DIRECTORY_NAME: &str = ".rustorrent";

pub(crate) fn is_state_directory_name(name: &OsStr) -> bool {
    #[cfg(any(target_os = "macos", windows))]
    {
        name.to_str()
            .is_some_and(|name| name.eq_ignore_ascii_case(STATE_DIRECTORY_NAME))
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        name == OsStr::new(STATE_DIRECTORY_NAME)
    }
}

pub(crate) fn is_state_file_path(path: &Path) -> bool {
    path.file_name().is_some()
        && path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(is_state_directory_name)
}

#[cfg(any(unix, windows))]
fn state_path_parts(path: &Path) -> Option<(PathBuf, &OsStr)> {
    let parent = path.parent()?;
    if !parent.file_name().is_some_and(is_state_directory_name) {
        return None;
    }
    let mut root = parent.parent()?.to_path_buf();
    if root.as_os_str().is_empty() {
        root.push(".");
    }
    Some((root, path.file_name()?))
}

#[cfg(unix)]
mod unix {
    use std::collections::HashMap;
    use std::ffi::{CStr, CString, OsStr, OsString};
    use std::fs;
    use std::io::{self, Read, Write};
    use std::mem::MaybeUninit;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};

    use super::{state_path_parts, STATE_DIRECTORY_NAME};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    static BINDINGS: OnceLock<Mutex<BindingRegistry>> = OnceLock::new();
    const MAX_OPEN_BINDINGS: usize = 16;
    const MAX_KNOWN_BINDINGS: usize = 4_096;
    const MAX_DIRECTORY_ENTRIES: usize = 16_384;
    const MAX_DIRECTORY_NAME_BYTES: usize = 16 * 1024 * 1024;

    pub(super) struct OpenStateDirectory {
        root_path: PathBuf,
        root: fs::File,
        directory: fs::File,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BindingIdentity {
        root_device: u64,
        root_inode: u64,
        state_device: u64,
        state_inode: u64,
    }

    struct BindingRecord {
        identity: BindingIdentity,
        directory: Option<Arc<OpenStateDirectory>>,
        last_used: u64,
    }

    #[derive(Default)]
    struct BindingRegistry {
        entries: HashMap<PathBuf, BindingRecord>,
        clock: u64,
    }

    impl BindingRegistry {
        fn next_tick(&mut self) -> u64 {
            self.clock = self.clock.wrapping_add(1);
            self.clock
        }

        fn trim_open_directories(&mut self, current: &Path) {
            while self
                .entries
                .values()
                .filter(|record| record.directory.is_some())
                .count()
                > MAX_OPEN_BINDINGS
            {
                let candidate = self
                    .entries
                    .iter()
                    .filter(|(path, record)| {
                        path.as_path() != current && record.directory.is_some()
                    })
                    .min_by_key(|(_, record)| record.last_used)
                    .map(|(path, _)| path.clone());
                let Some(candidate) = candidate else {
                    break;
                };
                if let Some(record) = self.entries.get_mut(&candidate) {
                    // In-flight callers retain their own Arc. Dropping only
                    // the registry's reference safely closes idle descriptors.
                    record.directory = None;
                }
            }
        }
    }

    fn component_name(name: &OsStr) -> io::Result<CString> {
        let bytes = name.as_bytes();
        if bytes.is_empty() || bytes == b"." || bytes == b".." || bytes.contains(&b'/') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid state-file name",
            ));
        }
        CString::new(bytes).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "state-file name contains NUL")
        })
    }

    fn open_directory(path: &Path) -> io::Result<fs::File> {
        let mut options = fs::OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        );
        options.open(path)
    }

    fn same_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
        left.dev() == right.dev() && left.ino() == right.ino()
    }

    fn open_download_root(root: &Path, create: bool) -> io::Result<fs::File> {
        if create {
            fs::create_dir_all(root)?;
        }
        let directory = open_directory(root)?;
        let opened = directory.metadata()?;
        if !opened.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "download root is not a directory",
            ));
        }

        // Resolve and inspect only for the identity check. All subsequent
        // operations stay attached to `directory`, so renaming any textual
        // ancestor after this point cannot redirect them.
        let canonical = fs::canonicalize(root)?;
        let resolved = fs::metadata(&canonical)?;
        if !resolved.is_dir() || !same_identity(&opened, &resolved) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "download root changed while opening",
            ));
        }
        Ok(directory)
    }

    fn stat_at(directory: &fs::File, name: &OsStr) -> io::Result<libc::stat> {
        let name = component_name(name)?;
        let mut stat = MaybeUninit::<libc::stat>::zeroed();
        // SAFETY: the directory descriptor and NUL-terminated component are
        // live for the call, and `stat` points to writable storage.
        let result = unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result == 0 {
            // SAFETY: fstatat initialized the output on success.
            Ok(unsafe { stat.assume_init() })
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn is_directory(stat: &libc::stat) -> bool {
        stat.st_mode & libc::S_IFMT == libc::S_IFDIR
    }

    fn is_regular_file(stat: &libc::stat) -> bool {
        stat.st_mode & libc::S_IFMT == libc::S_IFREG
    }

    fn current_euid() -> libc::uid_t {
        // SAFETY: geteuid has no preconditions and does not dereference
        // caller-provided memory.
        unsafe { libc::geteuid() }
    }

    fn verify_regular_entry(stat: &libc::stat) -> io::Result<()> {
        if !is_regular_file(stat) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "state path is not a regular file",
            ));
        }
        if stat.st_nlink != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "state file must not be hard-linked",
            ));
        }
        // A permissive or foreign-owned state file could have been forged
        // before startup. Reject it before parsing any session fields.
        if stat.st_uid != current_euid() || stat.st_mode & 0o022 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "state file is not privately owned",
            ));
        }
        Ok(())
    }

    fn open_state_child(root: &fs::File, create: bool) -> io::Result<(fs::File, bool)> {
        let state_name = CString::new(STATE_DIRECTORY_NAME).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "state-directory name contains NUL",
            )
        })?;
        let mut created = false;
        if create {
            // SAFETY: root is a live directory descriptor and the state name
            // is a fixed single component. EEXIST is expected under races.
            let result = unsafe { libc::mkdirat(root.as_raw_fd(), state_name.as_ptr(), 0o700) };
            if result == 0 {
                created = true;
            } else {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::AlreadyExists {
                    return Err(error);
                }
            }
        }
        // SAFETY: root is live and the fixed component is NUL terminated.
        let fd = unsafe {
            libc::openat(
                root.as_raw_fd(),
                state_name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat returned a new owned descriptor.
        Ok((unsafe { fs::File::from_raw_fd(fd) }, created))
    }

    impl OpenStateDirectory {
        fn open(root_path: &Path, create: bool) -> io::Result<Self> {
            let root = open_download_root(root_path, create)?;
            let (directory, _created) = open_state_child(&root, create)?;
            let opened = directory.metadata()?;
            let linked = stat_at(&root, OsStr::new(STATE_DIRECTORY_NAME))?;
            if !opened.is_dir()
                || !is_directory(&linked)
                || opened.dev() != linked.st_dev as u64
                || opened.ino() != linked.st_ino as u64
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "state directory changed while opening",
                ));
            }
            // Validate even after mkdirat reported creation: on a permissive
            // download root another principal could have replaced the entry
            // before openat pinned it.
            if opened.uid() != current_euid() || opened.mode() & 0o022 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "existing state directory is not privately owned",
                ));
            }
            directory.set_permissions(fs::Permissions::from_mode(0o700))?;
            Ok(Self {
                root_path: root_path.to_path_buf(),
                root,
                directory,
            })
        }

        fn verify_binding(&self) -> io::Result<()> {
            let opened_root = self.root.metadata()?;
            let canonical = fs::canonicalize(&self.root_path)?;
            let resolved_root = fs::metadata(canonical)?;
            if !resolved_root.is_dir() || !same_identity(&opened_root, &resolved_root) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "download root changed after state-directory binding",
                ));
            }
            let opened_state = self.directory.metadata()?;
            let linked_state = stat_at(&self.root, OsStr::new(STATE_DIRECTORY_NAME))?;
            if !opened_state.is_dir()
                || !is_directory(&linked_state)
                || opened_state.dev() != linked_state.st_dev as u64
                || opened_state.ino() != linked_state.st_ino as u64
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "state directory changed after binding",
                ));
            }
            Ok(())
        }

        fn identity(&self) -> io::Result<BindingIdentity> {
            self.verify_binding()?;
            let root = self.root.metadata()?;
            let state = self.directory.metadata()?;
            Ok(BindingIdentity {
                root_device: root.dev(),
                root_inode: root.ino(),
                state_device: state.dev(),
                state_inode: state.ino(),
            })
        }

        fn for_path(path: &Path, create: bool) -> io::Result<(Arc<Self>, OsString)> {
            let (root, name) = state_path_parts(path).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "path is not a state file")
            })?;
            component_name(name)?;
            let directory = binding(&root, create)?;
            directory.verify_binding()?;
            Ok((directory, name.to_os_string()))
        }

        fn open_regular(&self, name: &OsStr) -> io::Result<(fs::File, libc::stat)> {
            let before = stat_at(&self.directory, name)?;
            verify_regular_entry(&before)?;
            let name = component_name(name)?;
            // SAFETY: the directory is live and the component is validated.
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: openat returned a new owned descriptor.
            let file = unsafe { fs::File::from_raw_fd(fd) };
            let opened = file.metadata()?;
            if !opened.is_file()
                || opened.nlink() != 1
                || opened.dev() != before.st_dev as u64
                || opened.ino() != before.st_ino as u64
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "state file changed while opening",
                ));
            }
            Ok((file, before))
        }

        fn read_limited(&self, name: &OsStr, limit: usize) -> io::Result<Vec<u8>> {
            let (file, stat) = self.open_regular(name)?;
            if stat.st_size < 0 || stat.st_size as u64 > limit as u64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("file exceeds {limit} byte limit"),
                ));
            }
            let mut data = Vec::with_capacity((stat.st_size as usize).min(limit));
            file.take((limit + 1) as u64).read_to_end(&mut data)?;
            if data.len() > limit {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("file exceeds {limit} byte limit"),
                ));
            }
            Ok(data)
        }

        fn existing_entry_is_safe(&self, name: &OsStr) -> io::Result<bool> {
            match stat_at(&self.directory, name) {
                Ok(stat) => {
                    verify_regular_entry(&stat)?;
                    Ok(true)
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            }
        }

        fn unlink(&self, name: &OsStr) -> io::Result<()> {
            let name = component_name(name)?;
            // SAFETY: the directory is live and the component is validated.
            // unlinkat with flags 0 removes the entry itself and never follows
            // a final symlink.
            let result = unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), 0) };
            if result == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }

        fn rename(&self, source: &OsStr, destination: &OsStr) -> io::Result<()> {
            let source = component_name(source)?;
            let destination = component_name(destination)?;
            // SAFETY: both names are validated components relative to the same
            // live directory descriptor.
            let result = unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    source.as_ptr(),
                    self.directory.as_raw_fd(),
                    destination.as_ptr(),
                )
            };
            if result == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }

        fn write_entry_atomic(&self, name: &OsStr, data: &[u8], mode: u32) -> io::Result<()> {
            self.existing_entry_is_safe(name)?;
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let mut temporary = name.to_os_string();
            temporary.push(format!(".tmp.{}.{}", std::process::id(), sequence));
            let temporary_name = component_name(&temporary)?;
            // SAFETY: the directory is live, the temporary name is a validated
            // component, and create/exclusive prevents replacing an entry.
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    temporary_name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    mode as libc::mode_t as libc::c_uint,
                )
            };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: openat returned a new owned descriptor.
            let mut file = unsafe { fs::File::from_raw_fd(fd) };
            let result = (|| -> io::Result<()> {
                file.write_all(data)?;
                file.sync_all()?;
                drop(file);
                self.rename(&temporary, name)?;
                self.directory.sync_all()
            })();
            if result.is_err() {
                let _ = self.unlink(&temporary);
            }
            result
        }

        fn entry_names(&self) -> io::Result<Vec<OsString>> {
            struct DirectoryStream(*mut libc::DIR);

            impl Drop for DirectoryStream {
                fn drop(&mut self) {
                    // SAFETY: the pointer was returned by fdopendir and this
                    // guard owns it until exactly one close here.
                    unsafe { libc::closedir(self.0) };
                }
            }

            // fdopendir takes ownership of its descriptor, so duplicate the
            // state directory descriptor before handing it to libc.
            // SAFETY: the directory descriptor is live.
            let duplicate = unsafe { libc::dup(self.directory.as_raw_fd()) };
            if duplicate < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: duplicate is a newly owned directory descriptor.
            let stream = unsafe { libc::fdopendir(duplicate) };
            if stream.is_null() {
                let error = io::Error::last_os_error();
                // SAFETY: fdopendir failed and therefore did not consume it.
                unsafe { libc::close(duplicate) };
                return Err(error);
            }
            let stream = DirectoryStream(stream);
            let mut names = Vec::new();
            let mut name_bytes = 0usize;
            loop {
                // errno is not portable to reset directly, but readdir errors
                // are exceptionally rare; a null pointer is treated as EOF.
                // SAFETY: stream remains live until closed below.
                let entry = unsafe { libc::readdir(stream.0) };
                if entry.is_null() {
                    break;
                }
                // SAFETY: d_name is a NUL-terminated array owned by stream.
                let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                if bytes != b"." && bytes != b".." {
                    name_bytes = name_bytes.saturating_add(bytes.len());
                    if names.len() >= MAX_DIRECTORY_ENTRIES || name_bytes > MAX_DIRECTORY_NAME_BYTES
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "state directory contains too many entries",
                        ));
                    }
                    names.push(OsString::from_vec(bytes.to_vec()));
                }
            }
            Ok(names)
        }
    }

    fn binding_key(root: &Path) -> io::Result<PathBuf> {
        if root.is_absolute() {
            Ok(root.to_path_buf())
        } else {
            Ok(std::env::current_dir()?.join(root))
        }
    }

    fn binding(root: &Path, create: bool) -> io::Result<Arc<OpenStateDirectory>> {
        let key = binding_key(root)?;
        let bindings = BINDINGS.get_or_init(|| Mutex::new(BindingRegistry::default()));
        let mut registry = bindings
            .lock()
            .map_err(|_| io::Error::other("state-directory binding registry is poisoned"))?;
        let tick = registry.next_tick();
        let directory = if let Some(record) = registry.entries.get(&key) {
            if let Some(directory) = record.directory.as_ref() {
                directory.verify_binding()?;
                Arc::clone(directory)
            } else {
                let expected_identity = record.identity;
                let directory = Arc::new(OpenStateDirectory::open(&key, false)?);
                if directory.identity()? != expected_identity {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "state directory identity changed after descriptor eviction",
                    ));
                }
                directory
            }
        } else {
            if registry.entries.len() >= MAX_KNOWN_BINDINGS {
                return Err(io::Error::other(
                    "too many distinct state-directory bindings",
                ));
            }
            Arc::new(OpenStateDirectory::open(&key, create)?)
        };
        let identity = directory.identity()?;
        registry.entries.insert(
            key.clone(),
            BindingRecord {
                identity,
                directory: Some(Arc::clone(&directory)),
                last_used: tick,
            },
        );
        registry.trim_open_directories(&key);
        Ok(directory)
    }

    #[cfg(test)]
    pub(super) fn open_binding_count() -> usize {
        BINDINGS
            .get_or_init(|| Mutex::new(BindingRegistry::default()))
            .lock()
            .map(|registry| {
                registry
                    .entries
                    .values()
                    .filter(|record| record.directory.is_some())
                    .count()
            })
            .unwrap_or(usize::MAX)
    }

    #[cfg(test)]
    pub(super) fn open_binding_limit() -> usize {
        MAX_OPEN_BINDINGS
    }

    pub(super) fn ensure(root: &Path) -> io::Result<()> {
        binding(root, true).map(|_| ())
    }

    pub(super) fn open_lock_directory(root: &Path) -> io::Result<fs::File> {
        let directory = binding(root, true)?;
        directory.verify_binding()?;
        // Use a fresh open-file description. Locking a dup of the cached
        // descriptor could keep the lock alive after the session guard drops.
        let lock_directory = open_state_child(&directory.root, false)?.0;
        let expected = directory.directory.metadata()?;
        let opened = lock_directory.metadata()?;
        if !opened.is_dir() || !same_identity(&expected, &opened) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "state directory changed while opening its session lock",
            ));
        }
        Ok(lock_directory)
    }

    pub(super) fn read_limited(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
        let (directory, name) = OpenStateDirectory::for_path(path, false)?;
        directory.read_limited(&name, limit)
    }

    pub(super) fn write_atomic(
        path: &Path,
        data: &[u8],
        keep_backup: bool,
        mode: u32,
        backup_limit: usize,
    ) -> io::Result<()> {
        let (directory, name) = OpenStateDirectory::for_path(path, true)?;
        if keep_backup {
            match directory.read_limited(&name, backup_limit) {
                Ok(existing) => {
                    let mut backup = name.clone();
                    backup.push(".bak");
                    directory.write_entry_atomic(&backup, &existing, mode)?;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        directory.write_entry_atomic(&name, data, mode)
    }

    pub(super) fn exists(path: &Path) -> io::Result<bool> {
        let (directory, name) = match OpenStateDirectory::for_path(path, false) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        directory.existing_entry_is_safe(&name)
    }

    pub(super) fn remove_file(path: &Path) -> io::Result<()> {
        let (directory, name) = OpenStateDirectory::for_path(path, false)?;
        match directory.existing_entry_is_safe(&name) {
            Ok(true) => directory.unlink(&name),
            Ok(false) => Err(io::Error::from(io::ErrorKind::NotFound)),
            Err(error) => Err(error),
        }
    }

    pub(super) fn remove_resume_artifacts(path: &Path) -> io::Result<()> {
        let (directory, name) = match OpenStateDirectory::for_path(path, false) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let mut exact = vec![name.clone()];
        for suffix in [".bak", ".tmp"] {
            let mut candidate = name.clone();
            candidate.push(suffix);
            exact.push(candidate);
        }
        let mut prefix = name.as_bytes().to_vec();
        prefix.extend_from_slice(b".tmp.");
        let mut failures = Vec::new();
        for candidate in exact {
            match directory.existing_entry_is_safe(&candidate) {
                Ok(true) => {
                    if let Err(error) = directory.unlink(&candidate) {
                        if error.kind() != io::ErrorKind::NotFound {
                            failures.push(format!("{}: {error}", candidate.to_string_lossy()));
                        }
                    }
                }
                Ok(false) => {}
                Err(error) => failures.push(format!("{}: {error}", candidate.to_string_lossy())),
            }
        }
        for candidate in directory.entry_names()? {
            if candidate.as_bytes().starts_with(&prefix) {
                match directory.existing_entry_is_safe(&candidate) {
                    Ok(true) => {
                        if let Err(error) = directory.unlink(&candidate) {
                            if error.kind() != io::ErrorKind::NotFound {
                                failures.push(format!("{}: {error}", candidate.to_string_lossy()));
                            }
                        }
                    }
                    Ok(false) => {}
                    Err(error) => {
                        failures.push(format!("{}: {error}", candidate.to_string_lossy()))
                    }
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(failures.join("; ")))
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::collections::HashMap;
    use std::ffi::{c_void, OsStr, OsString};
    use std::fs;
    use std::io::{self, Read, Write};
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::{Path, PathBuf};
    use std::ptr;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::windows_fs::{self, CreateDisposition, FileIdentity, OpenedFile, PinnedDir};

    use super::{state_path_parts, STATE_DIRECTORY_NAME};

    const SE_FILE_OBJECT: u32 = 1;
    const OWNER_SECURITY_INFORMATION: u32 = 0x0000_0001;
    const DACL_SECURITY_INFORMATION: u32 = 0x0000_0004;
    const PROTECTED_DACL_SECURITY_INFORMATION: u32 = 0x8000_0000;
    const SE_DACL_PROTECTED: u16 = 0x1000;
    const TOKEN_QUERY: u32 = 0x0000_0008;
    const TOKEN_USER_CLASS: u32 = 1;
    const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
    const ACL_SIZE_INFORMATION_CLASS: u32 = 2;
    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
    const ACCESS_ALLOWED_COMPOUND_ACE_TYPE: u8 = 4;
    const ACCESS_ALLOWED_OBJECT_ACE_TYPE: u8 = 5;
    const ACCESS_ALLOWED_CALLBACK_ACE_TYPE: u8 = 9;
    const ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE: u8 = 11;
    const INHERIT_ONLY_ACE: u8 = 0x08;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const GENERIC_ALL: u32 = 0x1000_0000;
    const DELETE: u32 = 0x0001_0000;
    const WRITE_DAC: u32 = 0x0004_0000;
    const WRITE_OWNER: u32 = 0x0008_0000;
    const FILE_WRITE_DATA: u32 = 0x0000_0002;
    const FILE_APPEND_DATA: u32 = 0x0000_0004;
    const FILE_WRITE_EA: u32 = 0x0000_0010;
    const FILE_DELETE_CHILD: u32 = 0x0000_0040;
    const FILE_WRITE_ATTRIBUTES: u32 = 0x0000_0100;
    const WRITE_ACCESS_MASK: u32 = GENERIC_WRITE
        | GENERIC_ALL
        | DELETE
        | WRITE_DAC
        | WRITE_OWNER
        | FILE_WRITE_DATA
        | FILE_APPEND_DATA
        | FILE_WRITE_EA
        | FILE_DELETE_CHILD
        | FILE_WRITE_ATTRIBUTES;
    const MAX_SECURITY_DESCRIPTOR_BYTES: usize = 64 * 1024;
    const MAX_ACES: usize = 4_096;
    const MAX_OPEN_BINDINGS: usize = 16;
    const MAX_KNOWN_BINDINGS: usize = 4_096;
    const MAX_DIRECTORY_ENTRIES: usize = 16_384;
    const MAX_DIRECTORY_NAME_BYTES: usize = 16 * 1024 * 1024;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    static BINDINGS: OnceLock<Mutex<BindingRegistry>> = OnceLock::new();

    #[repr(C)]
    struct SidAndAttributes {
        sid: *mut c_void,
        _attributes: u32,
    }

    #[repr(C)]
    struct TokenUser {
        user: SidAndAttributes,
    }

    #[repr(C)]
    struct Acl {
        _revision: u8,
        _sbz1: u8,
        _acl_size: u16,
        _ace_count: u16,
        _sbz2: u16,
    }

    #[repr(C)]
    struct AclSizeInformation {
        ace_count: u32,
        _acl_bytes_in_use: u32,
        _acl_bytes_free: u32,
    }

    #[repr(C)]
    struct AceHeader {
        ace_type: u8,
        ace_flags: u8,
        ace_size: u16,
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
        fn GetTokenInformation(
            token: *mut c_void,
            class: u32,
            information: *mut c_void,
            information_length: u32,
            return_length: *mut u32,
        ) -> i32;
        fn ConvertSidToStringSidW(sid: *mut c_void, string_sid: *mut *mut u16) -> i32;
        fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
            string: *const u16,
            revision: u32,
            descriptor: *mut *mut c_void,
            descriptor_size: *mut u32,
        ) -> i32;
        fn GetSecurityDescriptorDacl(
            descriptor: *mut c_void,
            present: *mut i32,
            dacl: *mut *mut Acl,
            defaulted: *mut i32,
        ) -> i32;
        fn GetSecurityDescriptorControl(
            descriptor: *mut c_void,
            control: *mut u16,
            revision: *mut u32,
        ) -> i32;
        fn SetSecurityInfo(
            handle: *mut c_void,
            object_type: u32,
            security_information: u32,
            owner: *mut c_void,
            group: *mut c_void,
            dacl: *mut Acl,
            sacl: *mut Acl,
        ) -> u32;
        fn GetSecurityInfo(
            handle: *mut c_void,
            object_type: u32,
            security_information: u32,
            owner: *mut *mut c_void,
            group: *mut *mut c_void,
            dacl: *mut *mut Acl,
            sacl: *mut *mut Acl,
            descriptor: *mut *mut c_void,
        ) -> u32;
        fn GetAclInformation(
            acl: *mut Acl,
            information: *mut c_void,
            information_length: u32,
            information_class: u32,
        ) -> i32;
        fn GetAce(acl: *mut Acl, ace_index: u32, ace: *mut *mut c_void) -> i32;
        fn EqualSid(first: *mut c_void, second: *mut c_void) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
    }

    struct OwnedHandle(*mut c_void);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: this guard owns the non-null token handle returned by
            // OpenProcessToken and closes it exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }

    struct LocalAllocation(*mut c_void);

    impl Drop for LocalAllocation {
        fn drop(&mut self) {
            // SAFETY: the Windows conversion/security APIs allocate this
            // buffer with LocalAlloc and transfer ownership to the caller.
            unsafe { LocalFree(self.0) };
        }
    }

    struct SecurityDescriptor {
        storage: Vec<usize>,
        len: usize,
    }

    impl SecurityDescriptor {
        fn as_bytes(&self) -> &[u8] {
            // SAFETY: storage remains live, is at least `len` bytes long, and
            // the descriptor was copied byte-for-byte into its aligned memory.
            unsafe { std::slice::from_raw_parts(self.storage.as_ptr().cast(), self.len) }
        }

        fn as_ptr(&self) -> *mut c_void {
            self.storage.as_ptr().cast_mut().cast()
        }
    }

    struct OpenStateDirectory {
        root_path: PathBuf,
        root: PinnedDir,
        directory: PinnedDir,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BindingIdentity {
        root: FileIdentity,
        state: FileIdentity,
    }

    struct BindingRecord {
        identity: BindingIdentity,
        directory: Option<Arc<OpenStateDirectory>>,
        last_used: u64,
    }

    #[derive(Default)]
    struct BindingRegistry {
        entries: HashMap<PathBuf, BindingRecord>,
        clock: u64,
    }

    impl BindingRegistry {
        fn next_tick(&mut self) -> u64 {
            self.clock = self.clock.wrapping_add(1);
            self.clock
        }

        fn trim_open_directories(&mut self, current: &Path) {
            while self
                .entries
                .values()
                .filter(|record| record.directory.is_some())
                .count()
                > MAX_OPEN_BINDINGS
            {
                let candidate = self
                    .entries
                    .iter()
                    .filter(|(path, record)| {
                        path.as_path() != current && record.directory.is_some()
                    })
                    .min_by_key(|(_, record)| record.last_used)
                    .map(|(path, _)| path.clone());
                let Some(candidate) = candidate else {
                    break;
                };
                if let Some(record) = self.entries.get_mut(&candidate) {
                    record.directory = None;
                }
            }
        }
    }

    pub(super) struct SessionLock {
        _binding: Arc<OpenStateDirectory>,
        _file: fs::File,
    }

    fn windows_error(code: u32) -> io::Error {
        io::Error::from_raw_os_error(code as i32)
    }

    fn with_current_user_sid<T>(
        action: impl FnOnce(*mut c_void) -> io::Result<T>,
    ) -> io::Result<T> {
        let mut token = ptr::null_mut();
        // SAFETY: GetCurrentProcess returns a process pseudo-handle and token
        // points to writable storage for the newly owned token handle.
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
        if opened == 0 {
            return Err(io::Error::last_os_error());
        }
        if token.is_null() {
            return Err(io::Error::other("Windows returned an empty process token"));
        }
        let token = OwnedHandle(token);
        let mut required = 0u32;
        // SAFETY: the null-buffer probe is the documented way to obtain the
        // TOKEN_USER buffer size.
        unsafe {
            GetTokenInformation(token.0, TOKEN_USER_CLASS, ptr::null_mut(), 0, &mut required)
        };
        if required == 0 || required as usize > MAX_SECURITY_DESCRIPTOR_BYTES {
            return Err(io::Error::other("invalid Windows token-user size"));
        }
        let words = (required as usize).div_ceil(std::mem::size_of::<usize>());
        let mut buffer = vec![0usize; words];
        // SAFETY: buffer is aligned and exposes at least `required` bytes;
        // GetTokenInformation initializes a TOKEN_USER on success.
        let read = unsafe {
            GetTokenInformation(
                token.0,
                TOKEN_USER_CLASS,
                buffer.as_mut_ptr().cast(),
                required,
                &mut required,
            )
        };
        if read == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the successful call initialized TOKEN_USER at the beginning
        // of the suitably aligned buffer.
        let user = unsafe { &*buffer.as_ptr().cast::<TokenUser>() };
        if user.user.sid.is_null() {
            return Err(io::Error::other("Windows token has no user SID"));
        }
        action(user.user.sid)
    }

    fn sid_string(sid: *mut c_void) -> io::Result<String> {
        let mut wide = ptr::null_mut();
        // SAFETY: sid was obtained from a Windows security descriptor/token
        // and wide points to writable storage for a LocalAlloc result.
        if unsafe { ConvertSidToStringSidW(sid, &mut wide) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if wide.is_null() {
            return Err(io::Error::other("Windows returned an empty SID string"));
        }
        let allocation = LocalAllocation(wide.cast());
        let mut len = 0usize;
        // SAFETY: ConvertSidToStringSidW returns a NUL-terminated UTF-16
        // string. SID strings are short; the explicit cap fails closed.
        while len < 512 && unsafe { *wide.add(len) } != 0 {
            len += 1;
        }
        if len == 512 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Windows SID string is unreasonably long",
            ));
        }
        // SAFETY: the preceding scan established `len` initialized code units.
        let text = String::from_utf16(unsafe { std::slice::from_raw_parts(wide, len) })
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid SID string"))?;
        drop(allocation);
        Ok(text)
    }

    fn descriptor_from_sddl(sddl: &str) -> io::Result<SecurityDescriptor> {
        let mut wide = OsStr::new(sddl).encode_wide().collect::<Vec<_>>();
        wide.push(0);
        let mut descriptor = ptr::null_mut();
        let mut length = 0u32;
        // SAFETY: wide is a NUL-terminated SDDL string and output pointers are
        // writable. Windows owns the returned LocalAlloc buffer until copied.
        let converted = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                SECURITY_DESCRIPTOR_REVISION,
                &mut descriptor,
                &mut length,
            )
        };
        if converted == 0 {
            return Err(io::Error::last_os_error());
        }
        if descriptor.is_null() || length == 0 || length as usize > MAX_SECURITY_DESCRIPTOR_BYTES {
            if !descriptor.is_null() {
                // SAFETY: a successful conversion returned LocalAlloc memory.
                unsafe { LocalFree(descriptor) };
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Windows security descriptor",
            ));
        }
        let allocation = LocalAllocation(descriptor);
        let len = length as usize;
        let words = len.div_ceil(std::mem::size_of::<usize>());
        let mut storage = vec![0usize; words];
        // SAFETY: both buffers are valid for `len` bytes and do not overlap.
        unsafe {
            ptr::copy_nonoverlapping(
                descriptor.cast::<u8>(),
                storage.as_mut_ptr().cast::<u8>(),
                len,
            )
        };
        drop(allocation);
        Ok(SecurityDescriptor { storage, len })
    }

    fn private_security_descriptor() -> io::Result<SecurityDescriptor> {
        with_current_user_sid(|sid| {
            let sid = sid_string(sid)?;
            descriptor_from_sddl(&format!(
                "O:{sid}D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
            ))
        })
    }

    fn apply_descriptor_dacl(file: &fs::File, descriptor: &SecurityDescriptor) -> io::Result<()> {
        let mut present = 0;
        let mut defaulted = 0;
        let mut dacl = ptr::null_mut();
        // SAFETY: descriptor is a valid self-relative descriptor produced by
        // the Windows SDDL parser, and output pointers are writable.
        if unsafe {
            GetSecurityDescriptorDacl(descriptor.as_ptr(), &mut present, &mut dacl, &mut defaulted)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if present == 0 || dacl.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "private state ACL is missing",
            ));
        }
        // SAFETY: file is open with WRITE_DAC, dacl remains live for the call,
        // and no null DACL is ever passed.
        let status = unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                dacl,
                ptr::null_mut(),
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(windows_error(status))
        }
    }

    fn validate_private_security(
        file: &fs::File,
        require_protected: bool,
        label: &str,
    ) -> io::Result<()> {
        with_current_user_sid(|current_sid| {
            let mut owner = ptr::null_mut();
            let mut dacl = ptr::null_mut();
            let mut descriptor = ptr::null_mut();
            // SAFETY: file is open with READ_CONTROL and all output pointers
            // are writable. GetSecurityInfo allocates descriptor with LocalAlloc.
            let status = unsafe {
                GetSecurityInfo(
                    file.as_raw_handle(),
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                    &mut owner,
                    ptr::null_mut(),
                    &mut dacl,
                    ptr::null_mut(),
                    &mut descriptor,
                )
            };
            if status != 0 {
                return Err(windows_error(status));
            }
            if descriptor.is_null() {
                return Err(io::Error::other("Windows returned no security descriptor"));
            }
            let allocation = LocalAllocation(descriptor);
            if owner.is_null() || unsafe { EqualSid(owner, current_sid) } == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{label} is not owned by the current user"),
                ));
            }
            if dacl.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{label} has a null DACL"),
                ));
            }
            if require_protected {
                let mut control = 0u16;
                let mut revision = 0u32;
                // SAFETY: descriptor is live and the output words are writable.
                if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) }
                    == 0
                {
                    return Err(io::Error::last_os_error());
                }
                if control & SE_DACL_PROTECTED == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("{label} inherits an unsafe DACL"),
                    ));
                }
            }

            let mut information = MaybeUninit::<AclSizeInformation>::zeroed();
            // SAFETY: dacl points inside the live descriptor and information
            // exposes the complete documented output structure.
            if unsafe {
                GetAclInformation(
                    dacl,
                    information.as_mut_ptr().cast(),
                    std::mem::size_of::<AclSizeInformation>() as u32,
                    ACL_SIZE_INFORMATION_CLASS,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: GetAclInformation initialized the structure on success.
            let information = unsafe { information.assume_init() };
            if information.ace_count as usize > MAX_ACES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{label} ACL has too many entries"),
                ));
            }
            let mut current_user_can_write = false;
            for index in 0..information.ace_count {
                let mut ace = ptr::null_mut();
                // SAFETY: index is within the count reported for this live ACL.
                if unsafe { GetAce(dacl, index, &mut ace) } == 0 {
                    return Err(io::Error::last_os_error());
                }
                if ace.is_null() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{label} ACL contains an empty entry"),
                    ));
                }
                // SAFETY: every ACE starts with an ACE_HEADER.
                let header = unsafe { &*ace.cast::<AceHeader>() };
                if usize::from(header.ace_size) < 8 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{label} ACL contains a malformed entry"),
                    ));
                }
                let is_allow = matches!(
                    header.ace_type,
                    ACCESS_ALLOWED_ACE_TYPE
                        | ACCESS_ALLOWED_COMPOUND_ACE_TYPE
                        | ACCESS_ALLOWED_OBJECT_ACE_TYPE
                        | ACCESS_ALLOWED_CALLBACK_ACE_TYPE
                        | ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE
                );
                if !is_allow || header.ace_flags & INHERIT_ONLY_ACE != 0 {
                    continue;
                }
                // SAFETY: the access mask follows the four-byte ACE header;
                // read_unaligned covers the native variable-size allocation.
                let mask = unsafe { ptr::read_unaligned(ace.cast::<u8>().add(4).cast::<u32>()) };
                if mask & WRITE_ACCESS_MASK == 0 {
                    continue;
                }
                if header.ace_type != ACCESS_ALLOWED_ACE_TYPE || usize::from(header.ace_size) < 12 {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("{label} has an unsupported write-capable ACL entry"),
                    ));
                }
                // ACCESS_ALLOWED_ACE stores the variable-length SID after its
                // 4-byte header and 4-byte access mask.
                let ace_sid = unsafe { ace.cast::<u8>().add(8).cast::<c_void>() };
                if unsafe { EqualSid(ace_sid, current_sid) } != 0 {
                    current_user_can_write = true;
                    continue;
                }
                let trustee = sid_string(ace_sid)?;
                if trustee != "S-1-5-18" && trustee != "S-1-5-32-544" {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("{label} is writable by an untrusted principal"),
                    ));
                }
            }
            drop(allocation);
            if !current_user_can_write {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{label} is not writable by the current user"),
                ));
            }
            Ok(())
        })
    }

    impl OpenStateDirectory {
        fn open(root_path: &Path, create: bool) -> io::Result<Self> {
            if create {
                fs::create_dir_all(root_path)?;
            }
            let root = PinnedDir::open(root_path)?;
            let descriptor = private_security_descriptor()?;
            let (directory, created) = root.open_relative_dir_secure(
                Path::new(STATE_DIRECTORY_NAME),
                create,
                Some(descriptor.as_bytes()),
            )?;
            if created {
                validate_private_security(directory.as_file(), true, "new state directory")?;
            } else {
                // Validate provenance before changing permissions or consuming
                // any pre-existing state file.
                validate_private_security(directory.as_file(), false, "existing state directory")?;
                apply_descriptor_dacl(directory.as_file(), &descriptor)?;
                validate_private_security(directory.as_file(), true, "state directory")?;
            }
            Ok(Self {
                root_path: root_path.to_path_buf(),
                root,
                directory,
            })
        }

        fn verify_binding(&self) -> io::Result<()> {
            self.root.verify_path(&self.root_path)?;
            let current = self
                .root
                .open_relative_dir_secure(Path::new(STATE_DIRECTORY_NAME), false, None)?
                .0;
            if current.identity() != self.directory.identity() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "state directory changed after binding",
                ));
            }
            validate_private_security(current.as_file(), true, "state directory")
        }

        fn identity(&self) -> BindingIdentity {
            BindingIdentity {
                root: self.root.identity(),
                state: self.directory.identity(),
            }
        }

        fn for_path(path: &Path, create: bool) -> io::Result<(Arc<Self>, OsString)> {
            let (root, name) = state_path_parts(path).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "path is not a state file")
            })?;
            validate_component(name)?;
            let directory = binding(&root, create)?;
            Ok((directory, name.to_os_string()))
        }

        fn open_regular(&self, name: &OsStr) -> io::Result<OpenedFile> {
            validate_component(name)?;
            let opened = self.directory.open_regular_with_read_control(
                Path::new(name),
                CreateDisposition::OpenExisting,
                false,
            )?;
            validate_private_security(&opened.file, false, "state file")?;
            Ok(opened)
        }

        fn read_limited(&self, name: &OsStr, limit: usize) -> io::Result<Vec<u8>> {
            let opened = self.open_regular(name)?;
            if opened.info.length > limit as u64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("file exceeds {limit} byte limit"),
                ));
            }
            let mut data = Vec::with_capacity((opened.info.length as usize).min(limit));
            opened
                .file
                .take((limit + 1) as u64)
                .read_to_end(&mut data)?;
            if data.len() > limit {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("file exceeds {limit} byte limit"),
                ));
            }
            Ok(data)
        }

        fn existing_entry_is_safe(&self, name: &OsStr) -> io::Result<bool> {
            match self.open_regular(name) {
                Ok(_) => Ok(true),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            }
        }

        fn unlink(&self, name: &OsStr) -> io::Result<()> {
            let opened = self.open_regular(name)?;
            windows_fs::mark_delete(&opened.file)
        }

        fn write_entry_atomic(&self, name: &OsStr, data: &[u8]) -> io::Result<()> {
            self.existing_entry_is_safe(name)?;
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let mut temporary = name.to_os_string();
            temporary.push(format!(".tmp.{}.{}", std::process::id(), sequence));
            validate_component(&temporary)?;
            let descriptor = private_security_descriptor()?;
            let mut opened = self.directory.open_regular_secure(
                Path::new(&temporary),
                CreateDisposition::CreateNew,
                false,
                Some(descriptor.as_bytes()),
            )?;
            validate_private_security(&opened.file, true, "temporary state file")?;
            let result = (|| -> io::Result<()> {
                opened.file.write_all(data)?;
                opened.file.sync_all()?;
                self.existing_entry_is_safe(name)?;
                self.directory
                    .rename_open_file_here(&opened.file, name, true)
            })();
            if result.is_err() {
                let _ = windows_fs::mark_delete(&opened.file);
            }
            result
        }

        fn entry_names(&self) -> io::Result<Vec<OsString>> {
            let mut names = Vec::new();
            let mut name_bytes = 0usize;
            // The state and download-root handles omit FILE_SHARE_DELETE, so
            // this resolved path cannot be redirected while the binding lives.
            for entry in fs::read_dir(self.directory.final_path())? {
                let name = entry?.file_name();
                name_bytes = name_bytes.saturating_add(name.encode_wide().count() * 2);
                if names.len() >= MAX_DIRECTORY_ENTRIES || name_bytes > MAX_DIRECTORY_NAME_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "state directory contains too many entries",
                    ));
                }
                names.push(name);
            }
            Ok(names)
        }
    }

    fn validate_component(name: &OsStr) -> io::Result<()> {
        let wide = name.encode_wide().collect::<Vec<_>>();
        if wide.is_empty()
            || wide == [b'.' as u16]
            || wide == [b'.' as u16, b'.' as u16]
            || wide.contains(&0)
            || wide.contains(&(b'/' as u16))
            || wide.contains(&(b'\\' as u16))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid state-file name",
            ));
        }
        Ok(())
    }

    fn binding(root: &Path, create: bool) -> io::Result<Arc<OpenStateDirectory>> {
        let root = if root.is_absolute() {
            root.to_path_buf()
        } else {
            std::env::current_dir()?.join(root)
        };
        // Open before consulting the registry. Its stable final path is the
        // case-normalized key, and its identities prove whether an evicted
        // textual binding still names the same directories.
        let candidate = Arc::new(OpenStateDirectory::open(&root, create)?);
        let key = candidate.root.final_path().to_path_buf();
        let candidate_identity = candidate.identity();
        let bindings = BINDINGS.get_or_init(|| Mutex::new(BindingRegistry::default()));
        let mut registry = bindings
            .lock()
            .map_err(|_| io::Error::other("state-directory binding registry is poisoned"))?;
        let tick = registry.next_tick();
        let directory = if let Some(record) = registry.entries.get(&key) {
            if record.identity != candidate_identity {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "state directory identity changed after binding",
                ));
            }
            record
                .directory
                .as_ref()
                .map(Arc::clone)
                .unwrap_or(candidate)
        } else {
            if registry.entries.len() >= MAX_KNOWN_BINDINGS {
                return Err(io::Error::other(
                    "too many distinct state-directory bindings",
                ));
            }
            candidate
        };
        directory.verify_binding()?;
        registry.entries.insert(
            key.clone(),
            BindingRecord {
                identity: candidate_identity,
                directory: Some(Arc::clone(&directory)),
                last_used: tick,
            },
        );
        registry.trim_open_directories(&key);
        Ok(directory)
    }

    pub(super) fn ensure(root: &Path) -> io::Result<()> {
        binding(root, true).map(|_| ())
    }

    pub(super) fn acquire_session_lock(root: &Path) -> io::Result<SessionLock> {
        let binding = binding(root, true)?;
        let descriptor = private_security_descriptor()?;
        let opened = binding.root.open_regular_secure(
            Path::new(".rustorrent.lock"),
            CreateDisposition::OpenOrCreate,
            false,
            Some(descriptor.as_bytes()),
        )?;
        if opened.created {
            validate_private_security(&opened.file, true, "new session lock")?;
        } else {
            validate_private_security(&opened.file, false, "existing session lock")?;
            apply_descriptor_dacl(&opened.file, &descriptor)?;
            validate_private_security(&opened.file, true, "session lock")?;
        }
        opened.file.try_lock()?;
        Ok(SessionLock {
            _binding: binding,
            _file: opened.file,
        })
    }

    pub(super) fn read_limited(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
        let (directory, name) = OpenStateDirectory::for_path(path, false)?;
        directory.read_limited(&name, limit)
    }

    pub(super) fn write_atomic(
        path: &Path,
        data: &[u8],
        keep_backup: bool,
        _mode: u32,
        backup_limit: usize,
    ) -> io::Result<()> {
        let (directory, name) = OpenStateDirectory::for_path(path, true)?;
        if keep_backup {
            match directory.read_limited(&name, backup_limit) {
                Ok(existing) => {
                    let mut backup = name.clone();
                    backup.push(".bak");
                    directory.write_entry_atomic(&backup, &existing)?;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        directory.write_entry_atomic(&name, data)
    }

    pub(super) fn exists(path: &Path) -> io::Result<bool> {
        let (directory, name) = match OpenStateDirectory::for_path(path, false) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        directory.existing_entry_is_safe(&name)
    }

    pub(super) fn remove_file(path: &Path) -> io::Result<()> {
        let (directory, name) = OpenStateDirectory::for_path(path, false)?;
        directory.unlink(&name)
    }

    pub(super) fn remove_resume_artifacts(path: &Path) -> io::Result<()> {
        let (directory, name) = match OpenStateDirectory::for_path(path, false) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let mut exact = vec![name.clone()];
        for suffix in [".bak", ".tmp"] {
            let mut candidate = name.clone();
            candidate.push(suffix);
            exact.push(candidate);
        }
        let mut prefix = name.to_string_lossy().into_owned();
        prefix.push_str(".tmp.");
        let mut failures = Vec::new();
        for candidate in exact {
            match directory.unlink(&candidate) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => failures.push(format!("{}: {error}", candidate.to_string_lossy())),
            }
        }
        for candidate in directory.entry_names()? {
            if candidate.to_string_lossy().starts_with(&prefix) {
                match directory.unlink(&candidate) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => {
                        failures.push(format!("{}: {error}", candidate.to_string_lossy()))
                    }
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(failures.join("; ")))
        }
    }

    #[cfg(test)]
    pub(super) fn open_binding_count() -> usize {
        BINDINGS
            .get_or_init(|| Mutex::new(BindingRegistry::default()))
            .lock()
            .map(|registry| {
                registry
                    .entries
                    .values()
                    .filter(|record| record.directory.is_some())
                    .count()
            })
            .unwrap_or(usize::MAX)
    }

    #[cfg(test)]
    pub(super) fn open_binding_limit() -> usize {
        MAX_OPEN_BINDINGS
    }

    #[cfg(test)]
    mod tests {
        use std::fs;
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicU64, Ordering};

        use super::{
            apply_descriptor_dacl, binding, descriptor_from_sddl, ensure,
            private_security_descriptor, read_limited, validate_private_security,
            with_current_user_sid, BINDINGS,
        };
        use crate::windows_fs::{CreateDisposition, PinnedDir};

        static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

        fn temp_path(label: &str) -> PathBuf {
            std::env::temp_dir().join(format!(
                "rustorrent-windows-state-{label}-{}-{}",
                std::process::id(),
                NEXT_TEST.fetch_add(1, Ordering::Relaxed)
            ))
        }

        fn permissive_descriptor() -> std::io::Result<super::SecurityDescriptor> {
            with_current_user_sid(|sid| {
                let sid = super::sid_string(sid)?;
                descriptor_from_sddl(&format!("O:{sid}D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;WD)"))
            })
        }

        fn forget_binding(root: &Path) {
            let Ok(key) = fs::canonicalize(root) else {
                return;
            };
            if let Some(bindings) = BINDINGS.get() {
                if let Ok(mut bindings) = bindings.lock() {
                    bindings.entries.remove(&key);
                }
            }
        }

        #[test]
        fn preexisting_broadly_writable_state_directory_is_rejected() {
            let root = temp_path("permissive-directory");
            fs::create_dir_all(&root).unwrap();
            let root_handle = PinnedDir::open(&root).unwrap();
            let descriptor = permissive_descriptor().unwrap();
            let (state, created) = root_handle
                .open_relative_dir_secure(
                    Path::new(super::STATE_DIRECTORY_NAME),
                    true,
                    Some(descriptor.as_bytes()),
                )
                .unwrap();
            assert!(created);
            assert!(validate_private_security(state.as_file(), true, "test state").is_err());
            drop(state);
            drop(root_handle);

            assert!(ensure(&root).is_err());
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn broadly_writable_state_file_is_rejected_before_reading() {
            let root = temp_path("permissive-file");
            ensure(&root).unwrap();
            let binding = binding(&root, false).unwrap();
            let private = private_security_descriptor().unwrap();
            let mut opened = binding
                .directory
                .open_regular_secure(
                    Path::new("session.benc"),
                    CreateDisposition::CreateNew,
                    false,
                    Some(private.as_bytes()),
                )
                .unwrap();
            use std::io::Write;
            opened.file.write_all(b"forged").unwrap();
            opened.file.sync_all().unwrap();
            let permissive = permissive_descriptor().unwrap();
            apply_descriptor_dacl(&opened.file, &permissive).unwrap();
            drop(opened);
            drop(binding);

            let path = root.join(".rustorrent/session.benc");
            assert!(read_limited(&path, 1024).is_err());
            forget_binding(&root);
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn session_lock_remains_exclusive_for_its_full_lifetime() {
            let root = temp_path("session-lock");
            let first = super::acquire_session_lock(&root).unwrap();
            assert!(super::acquire_session_lock(&root).is_err());
            drop(first);
            let second = super::acquire_session_lock(&root).unwrap();
            drop(second);
            forget_binding(&root);
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn multiply_linked_state_file_is_rejected() {
            let root = temp_path("hard-link");
            super::write_atomic(
                &root.join(".rustorrent/session.benc"),
                b"state",
                false,
                0o600,
                1024,
            )
            .unwrap();
            fs::hard_link(
                root.join(".rustorrent/session.benc"),
                root.join("outside-link"),
            )
            .unwrap();
            assert!(read_limited(&root.join(".rustorrent/session.benc"), 1024).is_err());
            forget_binding(&root);
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn idle_state_handle_registry_is_bounded() {
            let limit = super::open_binding_limit();
            let mut roots = Vec::new();
            for index in 0..(limit + 4) {
                let root = temp_path(&format!("bounded-{index}"));
                ensure(&root).unwrap();
                roots.push(root);
            }
            assert!(super::open_binding_count() <= limit);
            for root in &roots {
                forget_binding(root);
            }
            for root in roots {
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
}

#[cfg(unix)]
pub(crate) fn ensure(root: &Path) -> std::io::Result<()> {
    unix::ensure(root)
}

#[cfg(windows)]
pub(crate) fn ensure(root: &Path) -> std::io::Result<()> {
    windows::ensure(root)
}

#[cfg(windows)]
pub(crate) struct SessionLock {
    _inner: windows::SessionLock,
}

#[cfg(windows)]
pub(crate) fn acquire_session_lock(root: &Path) -> std::io::Result<SessionLock> {
    windows::acquire_session_lock(root).map(|inner| SessionLock { _inner: inner })
}

#[cfg(unix)]
pub(crate) fn open_lock_directory(root: &Path) -> std::io::Result<std::fs::File> {
    unix::open_lock_directory(root)
}

#[cfg(unix)]
pub(crate) fn read_limited(path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
    unix::read_limited(path, limit)
}

#[cfg(windows)]
pub(crate) fn read_limited(path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
    windows::read_limited(path, limit)
}

#[cfg(unix)]
pub(crate) fn write_atomic(
    path: &Path,
    data: &[u8],
    keep_backup: bool,
    mode: u32,
    backup_limit: usize,
) -> std::io::Result<()> {
    unix::write_atomic(path, data, keep_backup, mode, backup_limit)
}

#[cfg(windows)]
pub(crate) fn write_atomic(
    path: &Path,
    data: &[u8],
    keep_backup: bool,
    mode: u32,
    backup_limit: usize,
) -> std::io::Result<()> {
    windows::write_atomic(path, data, keep_backup, mode, backup_limit)
}

#[cfg(unix)]
pub(crate) fn exists(path: &Path) -> std::io::Result<bool> {
    unix::exists(path)
}

#[cfg(windows)]
pub(crate) fn exists(path: &Path) -> std::io::Result<bool> {
    windows::exists(path)
}

#[cfg(unix)]
pub(crate) fn remove_file(path: &Path) -> std::io::Result<()> {
    unix::remove_file(path)
}

#[cfg(windows)]
pub(crate) fn remove_file(path: &Path) -> std::io::Result<()> {
    windows::remove_file(path)
}

#[cfg(unix)]
pub(crate) fn remove_resume_artifacts(path: &Path) -> std::io::Result<()> {
    unix::remove_resume_artifacts(path)
}

#[cfg(windows)]
pub(crate) fn remove_resume_artifacts(path: &Path) -> std::io::Result<()> {
    windows::remove_resume_artifacts(path)
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{ensure, write_atomic};

    static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rustorrent-state-dir-{label}-{}-{}",
            std::process::id(),
            NEXT_TEST.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn bound_state_directory_rejects_a_replacement() {
        let root = temp_path("state-swap");
        fs::create_dir_all(&root).unwrap();
        ensure(&root).unwrap();
        let original = root.join(".rustorrent");
        let moved = root.join("original-state");
        fs::rename(&original, &moved).unwrap();
        fs::create_dir(&original).unwrap();

        let target = original.join("session.benc");
        assert!(write_atomic(&target, b"replacement", false, 0o600, 1024).is_err());
        assert!(!target.exists());
        assert!(!moved.join("session.benc").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bound_download_root_rejects_an_ancestor_replacement() {
        let root = temp_path("root-swap");
        fs::create_dir_all(&root).unwrap();
        ensure(&root).unwrap();
        let moved = root.with_extension("original");
        fs::rename(&root, &moved).unwrap();
        fs::create_dir_all(root.join(".rustorrent")).unwrap();

        let target = root.join(".rustorrent/session.benc");
        assert!(write_atomic(&target, b"replacement", false, 0o600, 1024).is_err());
        assert!(!target.exists());
        assert!(!moved.join(".rustorrent/session.benc").exists());

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&moved);
    }

    #[test]
    fn preexisting_permissive_state_directory_is_rejected_without_adoption() {
        let root = temp_path("permissive-directory");
        let state = root.join(".rustorrent");
        fs::create_dir_all(&state).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o777)).unwrap();

        assert!(ensure(&root).is_err());
        assert_eq!(fs::metadata(&state).unwrap().mode() & 0o777, 0o777);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn preexisting_permissive_state_file_is_rejected_without_consumption() {
        let root = temp_path("permissive-file");
        fs::create_dir_all(&root).unwrap();
        ensure(&root).unwrap();
        let path = root.join(".rustorrent/session.benc");
        fs::write(&path, b"forged").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();

        assert!(super::read_limited(&path, 1024).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"forged");
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o666);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn idle_descriptor_cache_is_bounded() {
        let limit = super::unix::open_binding_limit();
        let mut roots = Vec::new();
        for index in 0..(limit + 4) {
            let root = temp_path(&format!("bounded-{index}"));
            fs::create_dir_all(&root).unwrap();
            ensure(&root).unwrap();
            roots.push(root);
        }
        assert!(super::unix::open_binding_count() <= limit);
        let first = &roots[0];
        let state = first.join(".rustorrent");
        let moved = first.join("original-state");
        fs::rename(&state, &moved).unwrap();
        fs::create_dir(&state).unwrap();
        let target = state.join("session.benc");
        assert!(write_atomic(&target, b"replacement", false, 0o600, 1024).is_err());
        assert!(!target.exists());
        for root in roots {
            let _ = fs::remove_dir_all(root);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_recognizes_state_directory_case_variants() {
        use std::ffi::OsStr;

        assert!(super::is_state_directory_name(OsStr::new(".RUSTORRENT")));
        assert!(super::is_state_file_path(std::path::Path::new(
            "/tmp/.Rustorrent/session.benc"
        )));
    }
}
