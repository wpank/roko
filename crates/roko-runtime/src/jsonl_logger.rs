//! JsonlLogger -- persists RuntimeEvents to a JSONL file.
//!
//! Each event is serialized as a single JSON line with a timestamp, enabling
//! replay and state reconstruction.

use roko_core::RuntimeEvent;
pub use roko_core::foundation::EventConsumer;
use roko_core::runtime_event::RuntimeEventEnvelope;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// Logger that writes RuntimeEvents as JSONL (one JSON object per line).
pub struct JsonlLogger {
    path: PathBuf,
    shared: Arc<SharedWriterState>,
}

const MAX_OPEN_RUN_WRITERS: usize = 32;

struct SharedWriterState {
    path: PathBuf,
    io: Mutex<WriterState>,
}

struct WriterState {
    seq: u64,
    writer: Option<std::io::BufWriter<std::fs::File>>,
    run_writers: HashMap<PathBuf, std::io::BufWriter<std::fs::File>>,
    // Declared after every buffered writer so field-drop order flushes/closes
    // those handles before releasing the cross-process maintenance lease.
    _lease: Option<std::fs::File>,
}

type WriterRegistry = HashMap<PathBuf, Weak<SharedWriterState>>;

static WRITER_REGISTRY: OnceLock<Mutex<WriterRegistry>> = OnceLock::new();

fn shared_writer(path: &Path) -> Arc<SharedWriterState> {
    let registry = WRITER_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry = registry
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(shared) = registry.get(path).and_then(Weak::upgrade) {
        return shared;
    }
    registry.retain(|_, shared| shared.strong_count() != 0);
    let shared = Arc::new(SharedWriterState {
        path: path.to_path_buf(),
        io: Mutex::new(WriterState {
            seq: 0,
            writer: None,
            run_writers: HashMap::new(),
            _lease: None,
        }),
    });
    registry.insert(path.to_path_buf(), Arc::downgrade(&shared));
    shared
}

impl JsonlLogger {
    /// Create a new JsonlLogger writing to the given path.
    pub fn new(path: PathBuf) -> Self {
        let path = absolute_path(path);
        let shared = shared_writer(&path);
        Self { path, shared }
    }

    /// Create from the standard .roko directory.
    pub fn from_roko_dir(roko_dir: &Path) -> Self {
        Self::new(roko_dir.join("runtime-events.jsonl"))
    }

    /// Path to the log file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Resolve the safe per-run index written alongside the global log.
    ///
    /// The run identifier is validated, then SHA-256 hashed; it is never used
    /// as a path component. Readers can therefore resolve the same file without
    /// maintaining a second identifier mapping.
    pub fn run_path(&self, run_id: &str) -> Result<PathBuf, &'static str> {
        run_index_path(&self.path, run_id)
    }

    fn shared_run_path(&self, run_id: &str) -> Result<PathBuf, &'static str> {
        run_index_path(&self.shared.path, run_id)
    }

    /// Flush buffered derived records for one run before a read-side query.
    ///
    /// The global compatibility log is already flushed independently. A
    /// missing writer is normal for an inactive or not-yet-observed run.
    pub fn flush_run(&self, run_id: &str) -> std::io::Result<()> {
        let Ok(run_path) = self.shared_run_path(run_id) else {
            return Ok(());
        };
        let mut state = self
            .shared
            .io
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(writer) = state.run_writers.get_mut(&run_path) {
            writer.flush()?;
        }
        Ok(())
    }

    fn ensure_writer(&self, state: &mut WriterState) -> std::io::Result<()> {
        if state.writer.is_none() {
            if let Some(parent) = self.shared.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let lease_path = lock_path(&self.shared.path)?;
            let lease = open_secure_lock_file(&lease_path)?;
            // All in-process instances for this path share `WriterState`.
            // Across processes, one exclusive lifetime lease prevents
            // independent buffers and sequence counters from interleaving.
            fs2::FileExt::try_lock_exclusive(&lease).map_err(|error| {
                std::io::Error::new(
                    error.kind(),
                    format!(
                        "runtime event writer lease is unavailable at {}: {error}",
                        lease_path.display()
                    ),
                )
            })?;
            let file = open_secure_append_file(&self.shared.path)?;
            state._lease = Some(lease);
            state.writer = Some(std::io::BufWriter::new(file));
        }
        Ok(())
    }

    /// Persist one event and return its exact post-append byte cursor in the
    /// derived per-run index.
    ///
    /// The cursor is measured while the per-run writer mutex is still held, so
    /// another producer using this logger cannot append between this event and
    /// the offset observation. The derived writer is flushed, but not fsynced;
    /// the global compatibility log remains the authoritative durable record.
    pub fn consume_with_run_cursor(&self, event: &RuntimeEvent) -> Option<u64> {
        match self.write_event(event, true) {
            Ok(cursor) => cursor,
            Err(error) => {
                tracing::warn!(
                    run_id = event.run_id(),
                    %error,
                    "failed to persist runtime event before live publication",
                );
                None
            }
        }
    }

    fn write_event(
        &self,
        event: &RuntimeEvent,
        require_run_cursor: bool,
    ) -> std::io::Result<Option<u64>> {
        let mut state = self
            .shared
            .io
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.ensure_writer(&mut state)?;

        let envelope = RuntimeEventEnvelope::new(
            event.run_id(),
            state.seq,
            "jsonl_logger",
            event.clone(),
        );
        state.seq = state.seq.saturating_add(1);

        let mut json = serde_json::to_string(&envelope)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        json.push('\n');

        if let Some(ref mut w) = state.writer {
            w.write_all(json.as_bytes())?;
            // High-frequency deltas remain buffered. Lifecycle, gate,
            // failure, and cursor-bearing publications flush both the global
            // authority and the derived index, so terminal truth stays
            // observable without a syscall on every streamed token chunk.
            if require_run_cursor || runtime_index_flush_boundary(event) {
                w.flush()?;
            }
        }

        // Keep a bounded set of per-run append handles. Clearing the cache
        // closes and flushes old writers; it never deletes an index. The global
        // log remains the durable compatibility source if this secondary write
        // encounters an error.
        let run_cursor = match self.write_run_index(
            &mut state,
            event,
            json.as_bytes(),
            require_run_cursor,
        ) {
            Ok(cursor) => cursor,
            Err(error) => {
                tracing::warn!(
                    run_id = event.run_id(),
                    %error,
                    "failed to append derived per-run runtime event index",
                );
                None
            }
        };

        Ok(run_cursor)
    }

    fn write_run_index(
        &self,
        state: &mut WriterState,
        event: &RuntimeEvent,
        json: &[u8],
        require_cursor: bool,
    ) -> std::io::Result<Option<u64>> {
        let Ok(run_path) = self.shared_run_path(event.run_id()) else {
            return Ok(None);
        };
        if !state.run_writers.contains_key(&run_path)
            && state.run_writers.len() >= MAX_OPEN_RUN_WRITERS
        {
            state.run_writers.clear();
        }
        if !state.run_writers.contains_key(&run_path) {
            let file = open_secure_run_index_file(&run_path)?;
            state
                .run_writers
                .insert(run_path.clone(), std::io::BufWriter::new(file));
        }
        if let Some(run_writer) = state.run_writers.get_mut(&run_path) {
            run_writer.write_all(json)?;
            if require_cursor || runtime_index_flush_boundary(event) {
                run_writer.flush()?;
            }
            if require_cursor {
                // The mutex is deliberately held across flush + metadata so a
                // concurrent producer cannot make this event inherit a later
                // event's cursor.
                return Ok(Some(run_writer.get_ref().metadata()?.len()));
            }
        }
        Ok(None)
    }
}

fn run_index_path(global_path: &Path, run_id: &str) -> Result<PathBuf, &'static str> {
    if run_id.is_empty() || run_id.len() > 128 {
        return Err("run id must contain 1..=128 bytes");
    }
    if !run_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
        || run_id.contains("..")
    {
        return Err("run id contains unsupported characters");
    }

    let stem = global_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or("global log has no UTF-8 file stem")?;
    let parent = global_path.parent().ok_or("global log has no parent")?;
    let digest = Sha256::digest(run_id.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    Ok(parent
        .join(format!("{stem}-by-run"))
        .join(format!("{encoded}.jsonl")))
}

fn absolute_path(path: PathBuf) -> PathBuf {
    let absolute = if path.is_absolute() {
        path
    } else if let Ok(current) = std::env::current_dir() {
        current.join(path)
    } else {
        return path;
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                // The input is absolute whenever cwd was available, so an
                // excess parent stays clamped at its filesystem root.
                let _ = normalized.pop();
            }
            std::path::Component::Prefix(_)
            | std::path::Component::RootDir
            | std::path::Component::Normal(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn lock_path(path: &Path) -> std::io::Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "log has no name"))?;
    let mut lock_name = name.to_os_string();
    lock_name.push(".lock");
    Ok(path.with_file_name(lock_name))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn open_secure_lock_file(path: &Path) -> std::io::Result<std::fs::File> {
    open_secure_file(path, false)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn open_secure_append_file(path: &Path) -> std::io::Result<std::fs::File> {
    open_secure_file(path, true)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn open_secure_file(path: &Path, append: bool) -> std::io::Result<std::fs::File> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "log has no parent")
    })?;
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "log has no filename")
    })?;
    let parent_fd = rustix::fs::open(
        parent,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    validate_secure_directory(&parent_fd, "runtime event log directory")?;
    let mut flags = rustix::fs::OFlags::RDWR
        | rustix::fs::OFlags::CREATE
        | rustix::fs::OFlags::NOFOLLOW
        | rustix::fs::OFlags::CLOEXEC;
    if append {
        flags |= rustix::fs::OFlags::APPEND;
    }
    let fd = rustix::fs::openat(
        &parent_fd,
        name,
        flags,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .map_err(std::io::Error::from)?;
    validate_secure_regular_file(&fd, "runtime event log or writer lease")?;
    Ok(std::fs::File::from(fd))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn open_secure_run_index_file(path: &Path) -> std::io::Result<std::fs::File> {
    let index_dir = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "run index has no parent")
    })?;
    let root = index_dir.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "run index directory has no parent",
        )
    })?;
    let directory_name = index_dir.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "run index directory has no name",
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "run index has no filename")
    })?;
    let root_fd = rustix::fs::open(
        root,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    validate_secure_directory(&root_fd, "runtime event state directory")?;
    match rustix::fs::mkdirat(
        &root_fd,
        directory_name,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR | rustix::fs::Mode::XUSR,
    ) {
        Ok(()) | Err(rustix::io::Errno::EXIST) => {}
        Err(error) => return Err(std::io::Error::from(error)),
    }
    let index_dir_fd = rustix::fs::openat(
        &root_fd,
        directory_name,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    validate_secure_directory(&index_dir_fd, "runtime per-run index directory")?;
    let fd = rustix::fs::openat(
        &index_dir_fd,
        file_name,
        rustix::fs::OFlags::WRONLY
            | rustix::fs::OFlags::CREATE
            | rustix::fs::OFlags::APPEND
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .map_err(std::io::Error::from)?;
    validate_secure_regular_file(&fd, "runtime per-run index")?;
    Ok(std::fs::File::from(fd))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_secure_directory(
    fd: &std::os::fd::OwnedFd,
    label: &str,
) -> std::io::Result<()> {
    let stat = rustix::fs::fstat(fd).map_err(std::io::Error::from)?;
    let mode = stat.st_mode as u32;
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::Directory
        || stat.st_uid as u32 != rustix::process::geteuid().as_raw()
        || mode & 0o022 != 0
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{label} must be user-owned and not group/world writable"),
        ));
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_secure_regular_file(
    fd: &std::os::fd::OwnedFd,
    label: &str,
) -> std::io::Result<()> {
    let stat = rustix::fs::fstat(fd).map_err(std::io::Error::from)?;
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::RegularFile
        || stat.st_uid as u32 != rustix::process::geteuid().as_raw()
        || stat.st_nlink != 1
        || (stat.st_mode as u32) & 0o022 != 0
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "{label} must be a single-link, user-owned regular file without group/world write access"
            ),
        ));
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn open_secure_lock_file(path: &Path) -> std::io::Result<std::fs::File> {
    open_checked_file(path, false)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn open_secure_append_file(path: &Path) -> std::io::Result<std::fs::File> {
    open_checked_file(path, true)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn open_secure_run_index_file(path: &Path) -> std::io::Result<std::fs::File> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "run index has no parent")
    })?;
    match std::fs::create_dir(parent) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    reject_symlink_or_special(parent, true)?;
    open_checked_file(path, true)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn open_checked_file(path: &Path, append: bool) -> std::io::Result<std::fs::File> {
    if let Some(parent) = path.parent() {
        reject_symlink_or_special(parent, true)?;
    }
    match std::fs::symlink_metadata(path) {
        Ok(_) => reject_symlink_or_special(path, false)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).read(true).write(true).append(append);
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("expected a regular file at {}", path.display()),
        ));
    }
    reject_symlink_or_special(path, false)?;
    Ok(file)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn reject_symlink_or_special(path: &Path, directory: bool) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsafe runtime event path: {}", path.display()),
        ));
    }
    Ok(())
}

fn runtime_index_flush_boundary(event: &RuntimeEvent) -> bool {
    matches!(
        event,
        RuntimeEvent::WorkflowCompleted { .. }
            | RuntimeEvent::RunCompleted { .. }
            | RuntimeEvent::TaskCompleted { .. }
            | RuntimeEvent::TaskFailed { .. }
            | RuntimeEvent::AgentCompleted { .. }
            | RuntimeEvent::AgentFailed { .. }
            | RuntimeEvent::GatePassed { .. }
            | RuntimeEvent::GateFailed { .. }
            | RuntimeEvent::InferenceFailed { .. }
    )
}

impl EventConsumer for JsonlLogger {
    fn consume(&self, event: &RuntimeEvent) {
        let _ = self.write_event(event, false);
    }

    fn consume_with_cursor(&self, event: &RuntimeEvent) -> Option<u64> {
        self.consume_with_run_cursor(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_events_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let logger = JsonlLogger::new(path.clone());

        logger.consume(&RuntimeEvent::AgentSpawned {
            run_id: "r1".into(),
            agent_id: "a1".into(),
            role: "implementer".into(),
            model: "model".into(),
        });

        logger.consume(&RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            duration_ms: 100,
        });

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: RuntimeEventEnvelope = serde_json::from_str(lines[0]).unwrap();
        let second: RuntimeEventEnvelope = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first.payload.kind(), "agent_spawned");
        assert_eq!(second.payload.kind(), "gate_passed");

        let run_content = std::fs::read_to_string(logger.run_path("r1").unwrap()).unwrap();
        assert_eq!(run_content.lines().count(), 2);
        assert!(!logger.run_path("r1").unwrap().to_string_lossy().contains("r1"));
    }

    #[test]
    fn run_path_rejects_traversal() {
        let logger = JsonlLogger::new(PathBuf::from("/tmp/events.jsonl"));
        assert!(logger.run_path("../../outside").is_err());
        assert!(logger.run_path("with/slash").is_err());
    }

    #[test]
    fn live_cursors_are_exact_monotonic_run_offsets() {
        let dir = tempfile::tempdir().unwrap();
        let logger = JsonlLogger::new(dir.path().join("events.jsonl"));
        let first = RuntimeEvent::AgentSpawned {
            run_id: "r1".into(),
            agent_id: "a1".into(),
            role: "implementer".into(),
            model: "model".into(),
        };
        let second = RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            duration_ms: 100,
        };

        let first_cursor = logger
            .consume_with_run_cursor(&first)
            .expect("first cursor");
        let second_cursor = logger
            .consume_with_run_cursor(&second)
            .expect("second cursor");

        assert!(first_cursor < second_cursor);
        assert_eq!(
            std::fs::metadata(logger.run_path("r1").unwrap())
                .unwrap()
                .len(),
            second_cursor,
        );
    }

    #[test]
    fn same_path_loggers_share_ordering_and_writers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let first_logger = JsonlLogger::new(path.clone());
        let second_logger = JsonlLogger::new(path.clone());
        let first = RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            duration_ms: 10,
        };
        let second = RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "test".into(),
            duration_ms: 20,
        };

        let first_cursor = first_logger
            .consume_with_run_cursor(&first)
            .expect("first cursor");
        let second_cursor = second_logger
            .consume_with_run_cursor(&second)
            .expect("second cursor");
        assert!(first_cursor < second_cursor);

        let envelopes = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<RuntimeEventEnvelope>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(envelopes.len(), 2);
        assert_eq!(envelopes[0].seq, 0);
        assert_eq!(envelopes[1].seq, 1);
    }

    #[test]
    fn lexical_path_aliases_share_the_same_writer_state() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = dir.path().join("events.jsonl");
        let aliased = dir.path().join("unused").join("..").join("events.jsonl");

        let first = JsonlLogger::new(canonical.clone());
        let second = JsonlLogger::new(aliased);

        assert_eq!(first.path(), canonical);
        assert_eq!(second.path(), canonical);
        assert!(Arc::ptr_eq(&first.shared, &second.shared));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn per_run_index_refuses_symlink_directory() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        symlink(outside.path(), dir.path().join("events-by-run")).unwrap();
        let logger = JsonlLogger::new(path);
        let event = RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            duration_ms: 10,
        };

        assert!(logger.consume_with_run_cursor(&event).is_none());
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }
}
