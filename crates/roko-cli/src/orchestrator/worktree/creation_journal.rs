//! Durable creation-claim journal and platform-specific filesystem security
//! primitives used by [`super::WorktreeManager`] to ensure crash-consistent
//! worktree creation and removal.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::git_ops::reattach_rejected;
use super::{
    CREATION_MARKER_DIR, CREATION_MARKER_SCHEMA, REPOSITORY_MUTATION_LOCK, RUNTIME_SHUTDOWN_WAIT,
    WorktreeError, WorktreeManager,
};

// ── Types ──

/// Phase of a linked-worktree creation tracked by the durable journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum CreationPhase {
    /// Marker published; branch/metadata may or may not exist yet.
    Prepared,
    /// Administrative directory and .git file registered; checkout not done.
    LinkedNoCheckout,
    /// `git reset --hard` succeeded; the checkout is fully populated.
    ResetComplete,
}

/// Crash-consistent marker written to disk before each creation phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CreationMarker {
    pub(super) schema_version: u8,
    pub(super) claim_id: String,
    pub(super) id: String,
    pub(super) repo_root: PathBuf,
    pub(super) common_git_dir: PathBuf,
    pub(super) branch: String,
    pub(super) branch_old_oid: Option<String>,
    pub(super) target_oid: String,
    pub(super) path: PathBuf,
    pub(super) admin_dir: PathBuf,
    pub(super) phase: CreationPhase,
    pub(super) previous_digest: Option<String>,
}

/// Compact record written once checkout succeeds, used to drive idempotent
/// crash-recovery cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CreationCleanupSafe {
    pub(super) schema_version: u8,
    pub(super) claim_id: String,
    pub(super) id: String,
    pub(super) repo_root: PathBuf,
    pub(super) common_git_dir: PathBuf,
    pub(super) branch: String,
    pub(super) branch_old_oid: Option<String>,
    pub(super) target_oid: String,
    pub(super) path: PathBuf,
    pub(super) admin_dir: PathBuf,
    pub(super) reset_complete_digest: String,
}

/// Held claim binding a creation-marker directory inode to the current process.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(in crate::orchestrator) struct CreationClaim {
    pub(super) marker: CreationMarker,
    pub(super) worktrees_root_fd: std::os::fd::OwnedFd,
    pub(super) marker_root_fd: std::os::fd::OwnedFd,
    pub(super) claim_dir_fd: std::os::fd::OwnedFd,
    pub(super) marker_root_inode: super::InodeIdentity,
    pub(super) claim_dir_inode: super::InodeIdentity,
}

/// Unsupported-platform stub.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(in crate::orchestrator) struct CreationClaim {
    pub(super) marker: CreationMarker,
}

/// Cross-task operation lifecycle used to signal cancellation and track
/// cleanup provenance across the runtime-independent worker boundary.
#[derive(Debug, Default)]
pub(in crate::orchestrator) struct OperationLifecycle {
    cancel_requested: AtomicBool,
    cleanup_unproved: AtomicBool,
    complete: AtomicBool,
    /// Git processes whose exit cleanup could not prove (bug-53475e).
    unproved_pids: parking_lot::Mutex<Vec<u32>>,
    /// The repository lock an operation whose cleanup was unproved keeps,
    /// which its worker hands to the manager's retained ownership.
    retained_lock: parking_lot::Mutex<Option<RepositoryMutationLock>>,
}

impl OperationLifecycle {
    pub(super) fn is_cancel_requested(&self) -> bool {
        self.cancel_requested.load(Ordering::Acquire)
    }

    pub(super) fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::Release);
    }

    pub(super) fn mark_cleanup_unproved(&self) {
        self.cleanup_unproved.store(true, Ordering::Release);
    }

    /// Mark cleanup unproved because git process `pid` may still run; a
    /// later operation proceeds once that process is gone (bug-53475e).
    pub(super) fn mark_cleanup_unproved_for(&self, pid: Option<u32>) {
        if let Some(pid) = pid {
            self.unproved_pids.lock().push(pid);
        }
        self.mark_cleanup_unproved();
    }

    /// The git processes whose exit cleanup could not prove.
    pub(super) fn take_unproved_pids(&self) -> Vec<u32> {
        std::mem::take(&mut *self.unproved_pids.lock())
    }

    /// The repository lock the operation kept after an unproved cleanup.
    pub(super) fn take_retained_lock(&self) -> Option<RepositoryMutationLock> {
        self.retained_lock.lock().take()
    }

    pub(super) fn cleanup_was_unproved(&self) -> bool {
        self.cleanup_unproved.load(Ordering::Acquire)
    }

    pub(super) fn mark_complete(&self) {
        self.complete.store(true, Ordering::Release);
    }

    pub(super) fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Acquire)
    }
}

/// Sentinel that signals cancellation when the owning Tokio runtime shuts
/// down. Disarmed explicitly once the worker completes normally.
pub(super) struct RuntimeShutdownOwner {
    lifecycle: std::sync::Arc<OperationLifecycle>,
    disarmed: bool,
}

impl RuntimeShutdownOwner {
    pub(super) fn new(lifecycle: std::sync::Arc<OperationLifecycle>) -> Self {
        Self {
            lifecycle,
            disarmed: false,
        }
    }

    pub(super) fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for RuntimeShutdownOwner {
    fn drop(&mut self) {
        if !self.disarmed && !self.lifecycle.is_complete() {
            self.lifecycle.request_cancel();
            let deadline = std::time::Instant::now() + RUNTIME_SHUTDOWN_WAIT;
            while !self.lifecycle.is_complete() && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
}

// ── Platform-specific security primitives ──

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn inode_identity(stat: &rustix::fs::Stat) -> super::InodeIdentity {
    super::InodeIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) struct RepositoryMutationLock {
    pub(super) repo_root_fd: std::os::fd::OwnedFd,
    pub(super) git_entry_fd: std::os::fd::OwnedFd,
    pub(super) common_dir_fd: std::os::fd::OwnedFd,
    pub(super) lock_fd: std::os::fd::OwnedFd,
    pub(super) repo_root_inode: super::InodeIdentity,
    pub(super) git_entry_inode: super::InodeIdentity,
    pub(super) git_entry_is_directory: bool,
    pub(super) common_dir_inode: super::InodeIdentity,
    pub(super) lock_inode: super::InodeIdentity,
    pub(super) repo_root: PathBuf,
    pub(super) canonical_common_dir: PathBuf,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl RepositoryMutationLock {
    pub(super) fn verify_binding(&self) -> std::io::Result<()> {
        let held_repo =
            validate_repository_directory(&self.repo_root_fd, "configured repository root")?;
        let held_git_entry = if self.git_entry_is_directory {
            validate_repository_directory(&self.git_entry_fd, "repository .git directory")?
        } else {
            validate_repository_identity_file(&self.git_entry_fd, "repository .git file")?;
            inode_identity(&rustix::fs::fstat(&self.git_entry_fd).map_err(std::io::Error::from)?)
        };
        let held_common =
            validate_repository_directory(&self.common_dir_fd, "Git common directory")?;
        validate_claim_file(&self.lock_fd, "repository mutation lock")?;
        let held_lock =
            inode_identity(&rustix::fs::fstat(&self.lock_fd).map_err(std::io::Error::from)?);
        let public_repo = rustix::fs::statat(
            rustix::fs::CWD,
            &self.repo_root,
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(std::io::Error::from)?;
        let public_git_entry = rustix::fs::statat(
            &self.repo_root_fd,
            ".git",
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(std::io::Error::from)?;
        let canonical_common = rustix::fs::statat(
            rustix::fs::CWD,
            &self.canonical_common_dir,
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(std::io::Error::from)?;
        let public_lock = rustix::fs::statat(
            &self.common_dir_fd,
            REPOSITORY_MUTATION_LOCK,
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(std::io::Error::from)?;
        let rebound = resolve_repository_identity(&self.repo_root_fd, &self.repo_root)?;
        if held_repo != self.repo_root_inode
            || held_common != self.common_dir_inode
            || held_git_entry != self.git_entry_inode
            || held_lock != self.lock_inode
            || inode_identity(&public_repo) != self.repo_root_inode
            || inode_identity(&public_git_entry) != self.git_entry_inode
            || inode_identity(&canonical_common) != self.common_dir_inode
            || inode_identity(&public_lock) != self.lock_inode
            || rebound.git_entry_inode != self.git_entry_inode
            || rebound.git_entry_is_directory != self.git_entry_is_directory
            || rebound.canonical_common_dir != self.canonical_common_dir
        {
            return Err(std::io::Error::other(
                "repository mutation lock no longer binds the canonical Git common directory",
            ));
        }
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl std::fmt::Debug for RepositoryMutationLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepositoryMutationLock")
            .field("canonical_common_dir", &self.canonical_common_dir)
            .finish_non_exhaustive()
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[derive(Debug)]
pub(super) struct RepositoryMutationLock;

pub(super) fn retain_lock_if_cleanup_unproved(
    repository_lock: RepositoryMutationLock,
    lifecycle: &OperationLifecycle,
) {
    if lifecycle.cleanup_was_unproved() {
        // A kernel-released flock is the only cross-process ownership proof.
        // If cleanup cannot be proved, retain it with the local operation
        // reservation so another process cannot overlap mutation, until the
        // manager proves the git processes gone (bug-53475e).
        *lifecycle.retained_lock.lock() = Some(repository_lock);
    }
}

/// How long a worktree mutation waits for another process's repository
/// mutation lock before it fails, instead of blocking forever (bug-53475e).
#[cfg(any(target_os = "macos", target_os = "linux"))]
const REPOSITORY_LOCK_WAIT: Duration = Duration::from_secs(60);

#[cfg(all(not(test), any(target_os = "macos", target_os = "linux")))]
fn repository_lock_wait() -> Duration {
    REPOSITORY_LOCK_WAIT
}

/// Tests shorten the wait for a subprocess through the environment.
#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
fn repository_lock_wait() -> Duration {
    std::env::var("ROKO_TEST_REPOSITORY_LOCK_WAIT_MS")
        .ok()
        .and_then(|millis| millis.parse().ok())
        .map_or(REPOSITORY_LOCK_WAIT, Duration::from_millis)
}

/// Take the exclusive `flock` on the repository mutation lock in
/// `common_dir`, retrying while another process holds it for at most
/// [`REPOSITORY_LOCK_WAIT`], then record this process as its holder.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn lock_repository_within_deadline(
    lock_fd: &std::os::fd::OwnedFd,
    common_dir: &Path,
) -> Result<(), WorktreeError> {
    let wait = repository_lock_wait();
    let deadline = std::time::Instant::now() + wait;
    loop {
        match rustix::fs::flock(
            lock_fd,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        ) {
            Ok(()) => {
                record_lock_holder(lock_fd);
                return Ok(());
            }
            Err(rustix::io::Errno::INTR) => {}
            Err(rustix::io::Errno::WOULDBLOCK) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(rustix::io::Errno::WOULDBLOCK) => {
                let holder = recorded_lock_holder(lock_fd);
                let named = match holder {
                    Some((pid, held_for)) => {
                        format!("pid {pid} (holding it for {}s)", held_for.as_secs())
                    }
                    None => "another process".to_string(),
                };
                return Err(WorktreeError::OwnershipRetained {
                    pids: holder.map(|(pid, _)| pid).into_iter().collect(),
                    reason: format!(
                        "{named} has held the repository mutation lock {} for over {}s; check \
                         for a stuck roko or git process",
                        common_dir.join(REPOSITORY_MUTATION_LOCK).display(),
                        wait.as_secs()
                    ),
                });
            }
            Err(error) => return Err(WorktreeError::IoError(std::io::Error::from(error))),
        }
    }
}

/// Write this process's id and the time into the repository mutation lock
/// it now holds, so a process that times out waiting for the lock can name
/// its holder (bug-53475e). Best effort: the `flock` alone is the lock.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn record_lock_holder(lock_fd: &std::os::fd::OwnedFd) {
    use std::os::unix::fs::FileExt;

    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let holder = format!("{} {since}\n", std::process::id());
    let recorded = lock_fd
        .try_clone()
        .map(std::fs::File::from)
        .and_then(|file| {
            file.set_len(0)?;
            file.write_all_at(holder.as_bytes(), 0)
        });
    if let Err(error) = recorded {
        tracing::debug!(%error, "could not record the repository mutation lock's holder");
    }
}

/// The holder [`record_lock_holder`] wrote, and how long it has held the
/// lock.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn recorded_lock_holder(lock_fd: &std::os::fd::OwnedFd) -> Option<(u32, Duration)> {
    use std::os::unix::fs::FileExt;

    let file = std::fs::File::from(lock_fd.try_clone().ok()?);
    let mut bytes = [0_u8; 64];
    let read = file.read_at(&mut bytes, 0).ok()?;
    let text = std::str::from_utf8(&bytes[..read]).ok()?;
    let mut fields = text.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let since = std::time::UNIX_EPOCH + Duration::from_secs(fields.next()?.parse().ok()?);
    let held_for = std::time::SystemTime::now()
        .duration_since(since)
        .unwrap_or_default();
    Some((pid, held_for))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn open_secure_directory_at(
    parent: &std::os::fd::OwnedFd,
    name: impl rustix::path::Arg,
) -> std::io::Result<std::os::fd::OwnedFd> {
    rustix::fs::openat(
        parent,
        name,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(std::io::Error::from)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_repository_directory(
    fd: &std::os::fd::OwnedFd,
    label: &str,
) -> std::io::Result<super::InodeIdentity> {
    let stat = rustix::fs::fstat(fd).map_err(std::io::Error::from)?;
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::Directory
        || stat.st_uid as u32 != rustix::process::geteuid().as_raw()
        || (stat.st_mode as u32) & 0o022 != 0
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "{label} must be an effective-user-owned directory without group/world write access"
            ),
        ));
    }
    Ok(inode_identity(&stat))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn validate_secure_directory(
    fd: &std::os::fd::OwnedFd,
    label: &str,
) -> std::io::Result<super::InodeIdentity> {
    let stat = rustix::fs::fstat(fd).map_err(std::io::Error::from)?;
    let mode = stat.st_mode as u32;
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::Directory
        || stat.st_uid as u32 != rustix::process::geteuid().as_raw()
        || mode & 0o777 != 0o700
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{label} must be an effective-user-owned 0700 directory"),
        ));
    }
    Ok(inode_identity(&stat))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) struct ResolvedRepositoryIdentity {
    pub(super) git_entry_fd: std::os::fd::OwnedFd,
    pub(super) git_entry_inode: super::InodeIdentity,
    pub(super) git_entry_is_directory: bool,
    pub(super) canonical_common_dir: PathBuf,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn resolve_repository_identity(
    repo_root_fd: &std::os::fd::OwnedFd,
    repo_root: &Path,
) -> std::io::Result<ResolvedRepositoryIdentity> {
    let git_entry_fd = rustix::fs::openat(
        repo_root_fd,
        ".git",
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    let git_entry_stat = rustix::fs::fstat(&git_entry_fd).map_err(std::io::Error::from)?;
    let git_entry_inode = inode_identity(&git_entry_stat);
    let git_entry_is_directory = rustix::fs::FileType::from_raw_mode(git_entry_stat.st_mode)
        == rustix::fs::FileType::Directory;
    let canonical_common_dir = if git_entry_is_directory {
        validate_repository_directory(&git_entry_fd, "repository .git directory")?;
        std::fs::canonicalize(repo_root.join(".git"))?
    } else {
        validate_repository_identity_file(&git_entry_fd, "repository .git file")?;
        let parse_fd = rustix::fs::openat(
            repo_root_fd,
            ".git",
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        let git_dir_reference =
            read_repository_path_file(parse_fd, "repository .git file", Some("gitdir: "))?;
        let git_admin_dir = canonicalize_repository_reference(repo_root, &git_dir_reference)?;
        let git_admin_fd = rustix::fs::open(
            &git_admin_dir,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        validate_repository_directory(&git_admin_fd, "linked-worktree Git admin directory")?;
        match rustix::fs::openat(
            &git_admin_fd,
            "commondir",
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        ) {
            Ok(common_reference_fd) => {
                let common_reference = read_repository_path_file(
                    common_reference_fd,
                    "linked-worktree commondir file",
                    None,
                )?;
                canonicalize_repository_reference(&git_admin_dir, &common_reference)?
            }
            Err(rustix::io::Errno::NOENT) => git_admin_dir,
            Err(error) => return Err(std::io::Error::from(error)),
        }
    };
    Ok(ResolvedRepositoryIdentity {
        git_entry_fd,
        git_entry_inode,
        git_entry_is_directory,
        canonical_common_dir,
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_repository_identity_file(
    fd: &std::os::fd::OwnedFd,
    label: &str,
) -> std::io::Result<super::InodeIdentity> {
    let stat = rustix::fs::fstat(fd).map_err(std::io::Error::from)?;
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::RegularFile
        || stat.st_uid as u32 != rustix::process::geteuid().as_raw()
        || (stat.st_mode as u32) & 0o022 != 0
        || stat.st_nlink != 1
        || stat.st_size <= 0
        || stat.st_size > 64 * 1024
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{label} has unsafe type, owner, mode, link count, or size"),
        ));
    }
    Ok(inode_identity(&stat))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_repository_path_file(
    fd: std::os::fd::OwnedFd,
    label: &str,
    prefix: Option<&str>,
) -> std::io::Result<PathBuf> {
    validate_repository_identity_file(&fd, label)?;
    let mut bytes = Vec::new();
    std::fs::File::from(fd).read_to_end(&mut bytes)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| std::io::Error::other(format!("{label} is not UTF-8")))?
        .trim_end_matches(['\r', '\n']);
    if value.is_empty()
        || value
            .chars()
            .any(|character| matches!(character, '\r' | '\n'))
    {
        return Err(std::io::Error::other(format!(
            "{label} must contain exactly one path record"
        )));
    }
    let path = match prefix {
        Some(prefix) => value
            .strip_prefix(prefix)
            .ok_or_else(|| std::io::Error::other(format!("{label} has invalid framing")))?,
        None => value,
    };
    if path.is_empty() {
        return Err(std::io::Error::other(format!(
            "{label} contains an empty path"
        )));
    }
    Ok(PathBuf::from(path))
}

fn canonicalize_repository_reference(base: &Path, reference: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(if reference.is_absolute() {
        reference.to_path_buf()
    } else {
        base.join(reference)
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_claim_file(fd: &std::os::fd::OwnedFd, label: &str) -> std::io::Result<()> {
    let stat = rustix::fs::fstat(fd).map_err(std::io::Error::from)?;
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::RegularFile
        || stat.st_uid as u32 != rustix::process::geteuid().as_raw()
        || (stat.st_mode as u32) & 0o777 != 0o600
        || stat.st_nlink != 1
        || stat.st_size < 0
        || stat.st_size > 64 * 1024
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{label} has unsafe type, owner, mode, link count, or size"),
        ));
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn write_claim_file(
    claim_dir_fd: &std::os::fd::OwnedFd,
    name: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    use std::io::Write;

    let fd = rustix::fs::openat(
        claim_dir_fd,
        name,
        rustix::fs::OFlags::WRONLY
            | rustix::fs::OFlags::CREATE
            | rustix::fs::OFlags::EXCL
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .map_err(std::io::Error::from)?;
    validate_claim_file(&fd, name)?;
    let mut file = std::fs::File::from(fd);
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_claim_file(claim_dir_fd: &std::os::fd::OwnedFd, name: &str) -> std::io::Result<Vec<u8>> {
    let fd = rustix::fs::openat(
        claim_dir_fd,
        name,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    validate_claim_file(&fd, name)?;
    let mut file = std::fs::File::from(fd);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_optional_claim_file(
    claim_dir_fd: &std::os::fd::OwnedFd,
    name: &str,
) -> std::io::Result<Option<Vec<u8>>> {
    match read_claim_file(claim_dir_fd, name) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(super) fn creation_record_name(claim_id: &str, phase: CreationPhase) -> String {
    let phase = match phase {
        CreationPhase::Prepared => "prepared",
        CreationPhase::LinkedNoCheckout => "linked_no_checkout",
        CreationPhase::ResetComplete => "reset_complete",
    };
    format!("{claim_id}.{phase}.json")
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn serialized_record(marker: &CreationMarker) -> std::io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(marker).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn write_creation_record(
    claim_dir_fd: &std::os::fd::OwnedFd,
    marker: &CreationMarker,
) -> std::io::Result<Vec<u8>> {
    let bytes = serialized_record(marker)?;
    write_claim_file(
        claim_dir_fd,
        &creation_record_name(&marker.claim_id, marker.phase),
        &bytes,
    )?;
    if read_claim_file(
        claim_dir_fd,
        &creation_record_name(&marker.claim_id, marker.phase),
    )? != bytes
    {
        return Err(std::io::Error::other(
            "creation record changed during durable publication",
        ));
    }
    Ok(bytes)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_creation_record_named(
    claim_dir_fd: &std::os::fd::OwnedFd,
    claim_id: &str,
    phase: CreationPhase,
) -> std::io::Result<(CreationMarker, Vec<u8>)> {
    let bytes = read_claim_file(claim_dir_fd, &creation_record_name(claim_id, phase))?;
    let marker = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
    Ok((marker, bytes))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_creation_record(
    claim_dir_fd: &std::os::fd::OwnedFd,
    expected: &CreationMarker,
) -> std::io::Result<(CreationMarker, Vec<u8>)> {
    read_creation_record_named(claim_dir_fd, &expected.claim_id, expected.phase)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_optional_creation_record(
    claim_dir_fd: &std::os::fd::OwnedFd,
    claim_id: &str,
    phase: CreationPhase,
) -> std::io::Result<Option<(CreationMarker, Vec<u8>)>> {
    match read_creation_record_named(claim_dir_fd, claim_id, phase) {
        Ok(record) => Ok(Some(record)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn verify_record_name_and_chain(
    marker: &CreationMarker,
    bytes: &[u8],
    previous: Option<&[u8]>,
) -> std::io::Result<()> {
    if marker.schema_version != CREATION_MARKER_SCHEMA
        || marker.claim_id.len() != 32
        || uuid::Uuid::parse_str(&marker.claim_id).is_err()
        || marker.previous_digest != previous.map(|bytes| blake3::hash(bytes).to_hex().to_string())
        || bytes.last() != Some(&b'\n')
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "creation record schema, UUID, sequence digest, or framing is invalid",
        ));
    }
    Ok(())
}

fn parse_claim_id(bytes: &[u8]) -> Result<String, WorktreeError> {
    let claim_id = std::str::from_utf8(bytes)
        .map_err(|error| reattach_rejected("unknown", error.to_string()))?
        .trim();
    let uuid = uuid::Uuid::parse_str(claim_id)
        .map_err(|_| reattach_rejected("unknown", "creation claim UUID is malformed"))?;
    let canonical = uuid.simple().to_string();
    if canonical != claim_id {
        return Err(reattach_rejected(
            "unknown",
            "creation claim UUID is not canonical",
        ));
    }
    Ok(canonical)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn ensure_cleanup_safe(
    claim_dir_fd: &std::os::fd::OwnedFd,
    marker: &CreationMarker,
) -> std::io::Result<()> {
    let (_, reset_bytes) = read_creation_record(claim_dir_fd, marker)?;
    let cleanup = CreationCleanupSafe {
        schema_version: CREATION_MARKER_SCHEMA,
        claim_id: marker.claim_id.clone(),
        id: marker.id.clone(),
        repo_root: marker.repo_root.clone(),
        common_git_dir: marker.common_git_dir.clone(),
        branch: marker.branch.clone(),
        branch_old_oid: marker.branch_old_oid.clone(),
        target_oid: marker.target_oid.clone(),
        path: marker.path.clone(),
        admin_dir: marker.admin_dir.clone(),
        reset_complete_digest: blake3::hash(&reset_bytes).to_hex().to_string(),
    };
    let mut bytes = serde_json::to_vec(&cleanup).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    match write_claim_file(claim_dir_fd, "cleanup-safe.json", &bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = read_claim_file(claim_dir_fd, "cleanup-safe.json")?;
            if existing == bytes {
                Ok(())
            } else {
                Err(std::io::Error::other(
                    "foreign cleanup-safe record blocks creation claim cleanup",
                ))
            }
        }
        Err(error) => Err(error),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn unlink_claim_file(
    claim_dir_fd: &std::os::fd::OwnedFd,
    name: &str,
) -> std::io::Result<()> {
    match rustix::fs::unlinkat(claim_dir_fd, name, rustix::fs::AtFlags::empty()) {
        Ok(()) => Ok(()),
        Err(rustix::io::Errno::NOENT) => Ok(()),
        Err(error) => Err(std::io::Error::from(error)),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_claim_entries(
    claim_dir_fd: &std::os::fd::OwnedFd,
    marker: &CreationMarker,
    allow_cleanup_safe: bool,
) -> std::io::Result<()> {
    use std::collections::BTreeSet;

    let mut expected = BTreeSet::from(["claim-id".to_string()]);
    expected.insert(creation_record_name(
        &marker.claim_id,
        CreationPhase::Prepared,
    ));
    if matches!(
        marker.phase,
        CreationPhase::LinkedNoCheckout | CreationPhase::ResetComplete
    ) {
        expected.insert(creation_record_name(
            &marker.claim_id,
            CreationPhase::LinkedNoCheckout,
        ));
    }
    if marker.phase == CreationPhase::ResetComplete {
        expected.insert(creation_record_name(
            &marker.claim_id,
            CreationPhase::ResetComplete,
        ));
    }
    if allow_cleanup_safe {
        expected.insert("cleanup-safe.json".to_string());
    }
    let mut actual = BTreeSet::new();
    let mut directory = rustix::fs::Dir::read_from(claim_dir_fd).map_err(std::io::Error::from)?;
    for entry in &mut directory {
        let entry = entry.map_err(std::io::Error::from)?;
        let name = entry
            .file_name()
            .to_str()
            .map_err(|_| std::io::Error::other("creation claim contains a non-UTF-8 entry"))?;
        if name != "." && name != ".." {
            actual.insert(name.to_string());
        }
    }
    if actual != expected {
        return Err(std::io::Error::other(format!(
            "creation claim contains missing, mixed, or unknown entries: {actual:?}"
        )));
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn cleanup_matches_marker(cleanup: &CreationCleanupSafe, marker: &CreationMarker) -> bool {
    marker.schema_version == cleanup.schema_version
        && marker.claim_id == cleanup.claim_id
        && marker.id == cleanup.id
        && marker.repo_root == cleanup.repo_root
        && marker.common_git_dir == cleanup.common_git_dir
        && marker.branch == cleanup.branch
        && marker.branch_old_oid == cleanup.branch_old_oid
        && marker.target_oid == cleanup.target_oid
        && marker.path == cleanup.path
        && marker.admin_dir == cleanup.admin_dir
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_cleanup_entries(
    claim_dir_fd: &std::os::fd::OwnedFd,
    cleanup: &CreationCleanupSafe,
) -> std::io::Result<()> {
    use std::collections::BTreeSet;

    let allowed = BTreeSet::from([
        "claim-id".to_string(),
        creation_record_name(&cleanup.claim_id, CreationPhase::Prepared),
        creation_record_name(&cleanup.claim_id, CreationPhase::LinkedNoCheckout),
        creation_record_name(&cleanup.claim_id, CreationPhase::ResetComplete),
        "cleanup-safe.json".to_string(),
    ]);
    let mut actual = BTreeSet::new();
    let mut directory = rustix::fs::Dir::read_from(claim_dir_fd).map_err(std::io::Error::from)?;
    for entry in &mut directory {
        let entry = entry.map_err(std::io::Error::from)?;
        let name = entry
            .file_name()
            .to_str()
            .map_err(|_| std::io::Error::other("creation claim contains a non-UTF-8 entry"))?;
        if name != "." && name != ".." {
            actual.insert(name.to_string());
        }
    }
    if !actual.contains("cleanup-safe.json") || !actual.is_subset(&allowed) {
        return Err(std::io::Error::other(format!(
            "cleanup-safe claim contains mixed or unknown entries: {actual:?}"
        )));
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_cleanup_records(
    claim_dir_fd: &std::os::fd::OwnedFd,
    cleanup: &CreationCleanupSafe,
) -> std::io::Result<()> {
    if let Some(claim_id) = read_optional_claim_file(claim_dir_fd, "claim-id")? {
        if claim_id != format!("{}\n", cleanup.claim_id).as_bytes() {
            return Err(std::io::Error::other(
                "cleanup-safe claim contains a mixed immutable identity",
            ));
        }
    }
    let prepared =
        read_optional_creation_record(claim_dir_fd, &cleanup.claim_id, CreationPhase::Prepared)?;
    let linked = read_optional_creation_record(
        claim_dir_fd,
        &cleanup.claim_id,
        CreationPhase::LinkedNoCheckout,
    )?;
    let reset = read_optional_creation_record(
        claim_dir_fd,
        &cleanup.claim_id,
        CreationPhase::ResetComplete,
    )?;
    for (record, phase) in [
        (prepared.as_ref(), CreationPhase::Prepared),
        (linked.as_ref(), CreationPhase::LinkedNoCheckout),
        (reset.as_ref(), CreationPhase::ResetComplete),
    ] {
        if let Some((marker, bytes)) = record {
            let digest_is_well_formed = marker.previous_digest.as_ref().is_some_and(|digest| {
                digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            });
            if !cleanup_matches_marker(cleanup, marker)
                || marker.phase != phase
                || bytes.last() != Some(&b'\n')
                || match phase {
                    CreationPhase::Prepared => marker.previous_digest.is_some(),
                    CreationPhase::LinkedNoCheckout | CreationPhase::ResetComplete => {
                        !digest_is_well_formed
                    }
                }
            {
                return Err(std::io::Error::other(
                    "cleanup-safe claim contains a malformed or mixed creation record",
                ));
            }
        }
    }
    if let (Some((_, prepared_bytes)), Some((linked_marker, _))) = (&prepared, &linked) {
        if linked_marker.previous_digest != Some(blake3::hash(prepared_bytes).to_hex().to_string())
        {
            return Err(std::io::Error::other(
                "cleanup-safe claim contains a broken prepared-to-linked digest",
            ));
        }
    }
    if let Some((reset_marker, reset_bytes)) = &reset {
        if cleanup.reset_complete_digest != blake3::hash(reset_bytes).to_hex().to_string() {
            return Err(std::io::Error::other(
                "cleanup-safe record does not bind the remaining reset record",
            ));
        }
        if let Some((_, linked_bytes)) = &linked {
            if reset_marker.previous_digest != Some(blake3::hash(linked_bytes).to_hex().to_string())
            {
                return Err(std::io::Error::other(
                    "cleanup-safe claim contains a broken linked-to-reset digest",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn remove_known_claim_files(
    claim_dir_fd: &std::os::fd::OwnedFd,
    marker: &CreationMarker,
) -> std::io::Result<()> {
    for phase in [
        CreationPhase::Prepared,
        CreationPhase::LinkedNoCheckout,
        CreationPhase::ResetComplete,
    ] {
        unlink_claim_file(claim_dir_fd, &creation_record_name(&marker.claim_id, phase))?;
    }
    unlink_claim_file(claim_dir_fd, "claim-id")?;
    // The self-contained terminal record is deliberately last: restart can
    // distinguish a committed mid-cleanup directory from incomplete state.
    unlink_claim_file(claim_dir_fd, "cleanup-safe.json")?;
    rustix::fs::fsync(claim_dir_fd).map_err(std::io::Error::from)
}

// ── WorktreeManager creation-journal methods ──

impl WorktreeManager {
    pub(super) fn verify_creation_marker(
        &self,
        id: &str,
        marker: &CreationMarker,
    ) -> std::io::Result<()> {
        if marker.schema_version != CREATION_MARKER_SCHEMA
            || marker.claim_id.is_empty()
            || marker.id != id
            || marker.repo_root != self.config.repo_root
            || marker.path != self.path_for(id)
        {
            return Err(std::io::Error::other(
                "creation marker does not match the canonical worktree identity",
            ));
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let common = {
            let repo_root_fd = rustix::fs::open(
                &self.config.repo_root,
                rustix::fs::OFlags::RDONLY
                    | rustix::fs::OFlags::DIRECTORY
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::empty(),
            )
            .map_err(std::io::Error::from)?;
            resolve_repository_identity(&repo_root_fd, &self.config.repo_root)?.canonical_common_dir
        };
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let common = std::fs::canonicalize(self.config.repo_root.join(".git"))?;
        if marker.common_git_dir != common
            || !matches!(marker.target_oid.len(), 40 | 64)
            || !marker
                .target_oid
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || marker.branch_old_oid.as_ref().is_some_and(|oid| {
                oid.len() != marker.target_oid.len()
                    || !oid.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return Err(std::io::Error::other(
                "creation marker repository or ref identity is invalid",
            ));
        }
        let expected_parent = common.join("worktrees");
        if marker.admin_dir.parent() != Some(expected_parent.as_path())
            || !marker
                .admin_dir
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("roko-"))
        {
            return Err(std::io::Error::other(
                "creation marker admin directory is outside the repository worktree registry",
            ));
        }
        Ok(())
    }

    pub(super) async fn reject_outstanding_creation_marker(
        &self,
        id: &str,
    ) -> Result<(), WorktreeError> {
        // R5 markers have no inode-bound update protocol. Preserve every type
        // and every byte (including malformed files and dangling symlinks) and
        // require explicit offline recovery instead of attempting migration.
        self.reject_legacy_creation_marker(id)?;

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        return self.recover_or_reject_creation_claim(id).await;

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        match std::fs::symlink_metadata(self.creation_claim_path(id)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(reattach_rejected(
                id,
                "durable creation claims require supported Unix fd-relative semantics",
            )),
            Err(error) => Err(WorktreeError::IoError(error)),
        }
    }

    pub(super) fn reject_legacy_creation_marker(&self, id: &str) -> Result<(), WorktreeError> {
        match std::fs::symlink_metadata(self.creation_marker_path(id)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => Err(reattach_rejected(
                id,
                "legacy durable creation marker requires explicit offline recovery",
            ))?,
            Err(error) => return Err(WorktreeError::IoError(error)),
        }
        Ok(())
    }

    pub(super) fn creation_marker_publication_error(
        &self,
        id: &str,
        error: std::io::Error,
    ) -> WorktreeError {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            reattach_rejected(
                id,
                "durable creation marker was acquired by another incomplete checkout",
            )
        } else {
            WorktreeError::IoError(error)
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(super) fn publish_creation_marker(
        &self,
        marker: CreationMarker,
    ) -> std::io::Result<CreationClaim> {
        let _ = marker;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "durable creation claims require macOS or Linux fd-relative filesystem semantics",
        ))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn publish_creation_marker(
        &self,
        marker: CreationMarker,
    ) -> std::io::Result<CreationClaim> {
        if marker.phase != CreationPhase::Prepared {
            return Err(std::io::Error::other(
                "initial creation marker must use the Prepared phase",
            ));
        }
        self.verify_creation_marker(&marker.id, &marker)?;
        let (worktrees_root_fd, marker_root_fd, marker_root_inode) =
            self.open_creation_marker_root(true)?;
        let claim_name = Self::creation_claim_name(&marker.id);
        rustix::fs::mkdirat(
            &marker_root_fd,
            claim_name.as_str(),
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR | rustix::fs::Mode::XUSR,
        )
        .map_err(std::io::Error::from)?;
        rustix::fs::fsync(&marker_root_fd).map_err(std::io::Error::from)?;
        let claim_dir_fd = open_secure_directory_at(&marker_root_fd, claim_name.as_str())?;
        let claim_dir_inode = validate_secure_directory(&claim_dir_fd, "creation claim")?;
        let mut claim = CreationClaim {
            marker,
            worktrees_root_fd,
            marker_root_fd,
            claim_dir_fd,
            marker_root_inode,
            claim_dir_inode,
        };
        self.verify_live_creation_claim(&claim)?;
        write_claim_file(
            &claim.claim_dir_fd,
            "claim-id",
            format!("{}\n", claim.marker.claim_id).as_bytes(),
        )?;
        write_creation_record(&claim.claim_dir_fd, &claim.marker)?;
        rustix::fs::fsync(&claim.claim_dir_fd).map_err(std::io::Error::from)?;
        validate_claim_entries(&claim.claim_dir_fd, &claim.marker, false)?;
        self.verify_live_creation_claim(&claim)?;
        // Keep initialization visibly all-or-nothing to callers: an error
        // after mkdir is a durable incomplete claim, never a reusable slot.
        claim.marker.previous_digest = None;
        Ok(claim)
    }

    pub(super) fn transition_creation_marker(
        &self,
        claim: &mut CreationClaim,
        next_phase: CreationPhase,
    ) -> std::io::Result<()> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let marker = &claim.marker;
            let legal = matches!(
                (marker.phase, next_phase),
                (CreationPhase::Prepared, CreationPhase::LinkedNoCheckout)
                    | (
                        CreationPhase::LinkedNoCheckout,
                        CreationPhase::ResetComplete
                    )
            );
            if !legal {
                return Err(std::io::Error::other(format!(
                    "illegal creation marker transition from {:?} to {next_phase:?}",
                    marker.phase
                )));
            }
            self.verify_live_creation_claim(claim)?;
            let (existing, existing_bytes) = read_creation_record(&claim.claim_dir_fd, marker)?;
            if existing != *marker {
                return Err(std::io::Error::other(
                    "creation marker identity or prior phase changed before transition",
                ));
            }
            validate_claim_entries(&claim.claim_dir_fd, marker, false)?;
            let mut next = marker.clone();
            next.phase = next_phase;
            next.previous_digest = Some(blake3::hash(&existing_bytes).to_hex().to_string());
            #[cfg(test)]
            self.test_claim_mutation_checkpoint(
                super::TestClaimMutationPoint::BeforeTransitionWrite,
            );
            write_creation_record(&claim.claim_dir_fd, &next)?;
            rustix::fs::fsync(&claim.claim_dir_fd).map_err(std::io::Error::from)?;
            self.verify_live_creation_claim(claim)?;
            claim.marker = next;
            validate_claim_entries(&claim.claim_dir_fd, &claim.marker, false)?;
            Ok(())
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (claim, next_phase);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "durable creation claims require macOS or Linux",
            ))
        }
    }

    pub(super) fn remove_completed_creation_marker(
        &self,
        claim: &CreationClaim,
    ) -> std::io::Result<()> {
        if claim.marker.phase != CreationPhase::ResetComplete {
            return Err(std::io::Error::other(
                "only a ResetComplete creation marker can be committed",
            ));
        }
        self.remove_creation_claim_if_exact(claim)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(super) fn remove_creation_claim_if_exact(
        &self,
        claim: &CreationClaim,
    ) -> std::io::Result<()> {
        let _ = claim;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "durable creation claims require macOS or Linux",
        ))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn remove_creation_claim_if_exact(
        &self,
        claim: &CreationClaim,
    ) -> std::io::Result<()> {
        self.verify_live_creation_claim(claim)?;
        let (existing, _) = read_creation_record(&claim.claim_dir_fd, &claim.marker)?;
        if existing != claim.marker {
            return Err(std::io::Error::other(
                "creation claim identity or phase changed before removal",
            ));
        }
        validate_claim_entries(&claim.claim_dir_fd, &claim.marker, false)?;
        #[cfg(test)]
        self.test_claim_mutation_checkpoint(super::TestClaimMutationPoint::BeforeRemovalCleanup);
        if claim.marker.phase == CreationPhase::ResetComplete {
            ensure_cleanup_safe(&claim.claim_dir_fd, &claim.marker)?;
            rustix::fs::fsync(&claim.claim_dir_fd).map_err(std::io::Error::from)?;
            validate_claim_entries(&claim.claim_dir_fd, &claim.marker, true)?;
            self.verify_live_creation_claim(claim)?;
        }
        remove_known_claim_files(&claim.claim_dir_fd, &claim.marker)?;
        self.verify_live_creation_claim(claim)?;
        rustix::fs::unlinkat(
            &claim.marker_root_fd,
            Self::creation_claim_name(&claim.marker.id).as_str(),
            rustix::fs::AtFlags::REMOVEDIR,
        )
        .map_err(std::io::Error::from)?;
        rustix::fs::fsync(&claim.marker_root_fd).map_err(std::io::Error::from)
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn acquire_repository_mutation_lock(
        &self,
    ) -> Result<RepositoryMutationLock, WorktreeError> {
        if !self.config.repo_root.is_absolute() {
            return Err(WorktreeError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "repository root must be absolute",
            )));
        }
        let repo_root_fd = rustix::fs::open(
            &self.config.repo_root,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        let repo_root_inode =
            validate_repository_directory(&repo_root_fd, "configured repository root")?;
        let ResolvedRepositoryIdentity {
            git_entry_fd,
            git_entry_inode,
            git_entry_is_directory,
            canonical_common_dir,
        } = resolve_repository_identity(&repo_root_fd, &self.config.repo_root)?;
        let common_dir_fd = rustix::fs::open(
            &canonical_common_dir,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        let common_dir_inode =
            validate_repository_directory(&common_dir_fd, "Git common directory")?;
        let canonical_stat = rustix::fs::statat(
            rustix::fs::CWD,
            &canonical_common_dir,
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(std::io::Error::from)?;
        if inode_identity(&canonical_stat) != common_dir_inode {
            return Err(WorktreeError::IoError(std::io::Error::other(
                "configured repository .git directory is not its canonical common directory",
            )));
        }
        let lock_fd = rustix::fs::openat(
            &common_dir_fd,
            REPOSITORY_MUTATION_LOCK,
            rustix::fs::OFlags::RDWR
                | rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .map_err(std::io::Error::from)?;
        validate_claim_file(&lock_fd, "repository mutation lock")?;
        let lock_inode =
            inode_identity(&rustix::fs::fstat(&lock_fd).map_err(std::io::Error::from)?);
        rustix::fs::fsync(&common_dir_fd).map_err(std::io::Error::from)?;
        lock_repository_within_deadline(&lock_fd, &canonical_common_dir)?;
        let repository_lock = RepositoryMutationLock {
            repo_root_fd,
            git_entry_fd,
            common_dir_fd,
            lock_fd,
            repo_root_inode,
            git_entry_inode,
            git_entry_is_directory,
            common_dir_inode,
            lock_inode,
            repo_root: self.config.repo_root.clone(),
            canonical_common_dir,
        };
        repository_lock.verify_binding()?;
        Ok(repository_lock)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(super) fn acquire_repository_mutation_lock(
        &self,
    ) -> Result<RepositoryMutationLock, WorktreeError> {
        Err(WorktreeError::UnsafeGitExecution {
            reason: "durable worktree mutation locking requires macOS or Linux".to_string(),
        })
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn open_creation_marker_root(
        &self,
        create: bool,
    ) -> std::io::Result<(
        std::os::fd::OwnedFd,
        std::os::fd::OwnedFd,
        super::InodeIdentity,
    )> {
        let worktrees_root_fd = rustix::fs::open(
            &self.config.worktrees_root,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        if create {
            match rustix::fs::mkdirat(
                &worktrees_root_fd,
                CREATION_MARKER_DIR,
                rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR | rustix::fs::Mode::XUSR,
            ) {
                Ok(()) => {
                    rustix::fs::fsync(&worktrees_root_fd).map_err(std::io::Error::from)?;
                }
                Err(rustix::io::Errno::EXIST) => {}
                Err(error) => return Err(std::io::Error::from(error)),
            }
        }
        let marker_root_fd = open_secure_directory_at(&worktrees_root_fd, CREATION_MARKER_DIR)?;
        let marker_root_inode = validate_secure_directory(&marker_root_fd, "creation marker root")?;
        Ok((worktrees_root_fd, marker_root_fd, marker_root_inode))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn verify_live_creation_claim(&self, claim: &CreationClaim) -> std::io::Result<()> {
        let held_root = validate_secure_directory(&claim.marker_root_fd, "creation marker root")?;
        let held_claim = validate_secure_directory(&claim.claim_dir_fd, "creation claim")?;
        if held_root != claim.marker_root_inode || held_claim != claim.claim_dir_inode {
            return Err(std::io::Error::other("held creation claim inode changed"));
        }
        let public_root = rustix::fs::statat(
            &claim.worktrees_root_fd,
            CREATION_MARKER_DIR,
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(std::io::Error::from)?;
        let public_claim = rustix::fs::statat(
            &claim.marker_root_fd,
            Self::creation_claim_name(&claim.marker.id).as_str(),
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(std::io::Error::from)?;
        if inode_identity(&public_root) != claim.marker_root_inode
            || inode_identity(&public_claim) != claim.claim_dir_inode
        {
            return Err(std::io::Error::other(
                "public creation claim pathname no longer names the held inode",
            ));
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(super) fn verify_live_creation_claim(&self, _claim: &CreationClaim) -> std::io::Result<()> {
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) async fn recover_or_reject_creation_claim(
        &self,
        id: &str,
    ) -> Result<(), WorktreeError> {
        let root = match self.open_creation_marker_root(false) {
            Ok(root) => root,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(WorktreeError::IoError(error)),
        };
        let (worktrees_root_fd, marker_root_fd, marker_root_inode) = root;
        let claim_name = Self::creation_claim_name(id);
        let claim_dir_fd = match open_secure_directory_at(&marker_root_fd, claim_name.as_str()) {
            Ok(fd) => fd,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(WorktreeError::IoError(error)),
        };
        let claim_dir_inode = validate_secure_directory(&claim_dir_fd, "creation claim")?;

        if let Some(cleanup_bytes) = read_optional_claim_file(&claim_dir_fd, "cleanup-safe.json")? {
            let cleanup: CreationCleanupSafe = serde_json::from_slice(&cleanup_bytes)
                .map_err(|error| reattach_rejected(id, error.to_string()))?;
            if cleanup.schema_version != CREATION_MARKER_SCHEMA
                || cleanup.id != id
                || cleanup.claim_id.len() != 32
                || !cleanup
                    .claim_id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || uuid::Uuid::parse_str(&cleanup.claim_id).is_err()
                || cleanup.reset_complete_digest.len() != 64
                || !cleanup
                    .reset_complete_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || cleanup_bytes.last() != Some(&b'\n')
            {
                return Err(reattach_rejected(id, "cleanup-safe record is malformed"));
            }
            validate_cleanup_entries(&claim_dir_fd, &cleanup)?;
            validate_cleanup_records(&claim_dir_fd, &cleanup)?;
            let marker = CreationMarker {
                schema_version: cleanup.schema_version,
                claim_id: cleanup.claim_id,
                id: cleanup.id,
                repo_root: cleanup.repo_root,
                common_git_dir: cleanup.common_git_dir,
                branch: cleanup.branch,
                branch_old_oid: cleanup.branch_old_oid,
                target_oid: cleanup.target_oid,
                path: cleanup.path,
                admin_dir: cleanup.admin_dir,
                phase: CreationPhase::ResetComplete,
                previous_digest: None,
            };
            let claim = CreationClaim {
                marker,
                worktrees_root_fd,
                marker_root_fd,
                claim_dir_fd,
                marker_root_inode,
                claim_dir_inode,
            };
            self.verify_live_creation_claim(&claim)?;
            self.verify_reset_complete_recovery(&claim.marker).await?;
            remove_known_claim_files(&claim.claim_dir_fd, &claim.marker)?;
            self.verify_live_creation_claim(&claim)?;
            rustix::fs::unlinkat(
                &claim.marker_root_fd,
                claim_name.as_str(),
                rustix::fs::AtFlags::REMOVEDIR,
            )
            .map_err(std::io::Error::from)?;
            rustix::fs::fsync(&claim.marker_root_fd).map_err(std::io::Error::from)?;
            return Ok(());
        }

        let claim_id = match read_claim_file(&claim_dir_fd, "claim-id") {
            Ok(bytes) => parse_claim_id(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match rustix::fs::unlinkat(
                    &marker_root_fd,
                    claim_name.as_str(),
                    rustix::fs::AtFlags::REMOVEDIR,
                ) {
                    Ok(()) => {
                        rustix::fs::fsync(&marker_root_fd).map_err(std::io::Error::from)?;
                        return Ok(());
                    }
                    Err(_) => {
                        return Err(reattach_rejected(
                            id,
                            "creation claim lacks immutable identity",
                        ));
                    }
                }
            }
            Err(error) => return Err(WorktreeError::IoError(error)),
        };
        let prepared =
            read_creation_record_named(&claim_dir_fd, &claim_id, CreationPhase::Prepared)
                .map_err(WorktreeError::IoError)?;
        self.verify_creation_marker(id, &prepared.0)?;
        verify_record_name_and_chain(&prepared.0, &prepared.1, None)?;
        let linked = read_optional_creation_record(
            &claim_dir_fd,
            &claim_id,
            CreationPhase::LinkedNoCheckout,
        )?;
        let reset =
            read_optional_creation_record(&claim_dir_fd, &claim_id, CreationPhase::ResetComplete)?;
        if let Some((linked_marker, linked_bytes)) = &linked {
            verify_record_name_and_chain(linked_marker, linked_bytes, Some(&prepared.1))?;
        }
        if let Some((reset_marker, reset_bytes)) = &reset {
            let Some((linked_marker, linked_bytes)) = &linked else {
                return Err(reattach_rejected(
                    id,
                    "reset record lacks linked predecessor",
                ));
            };
            if reset_marker.id != linked_marker.id
                || reset_marker.claim_id != linked_marker.claim_id
            {
                return Err(reattach_rejected(id, "mixed creation claim identities"));
            }
            verify_record_name_and_chain(reset_marker, reset_bytes, Some(linked_bytes))?;
            validate_claim_entries(&claim_dir_fd, reset_marker, false)?;
            let claim = CreationClaim {
                marker: reset_marker.clone(),
                worktrees_root_fd,
                marker_root_fd,
                claim_dir_fd,
                marker_root_inode,
                claim_dir_inode,
            };
            self.verify_live_creation_claim(&claim)?;
            self.verify_reset_complete_recovery(&claim.marker).await?;
            self.remove_creation_claim_if_exact(&claim)?;
            return Ok(());
        }
        validate_claim_entries(
            &claim_dir_fd,
            linked.as_ref().map_or(&prepared.0, |record| &record.0),
            false,
        )?;
        Err(reattach_rejected(
            id,
            if linked.is_some() {
                "durable creation claim remains at LinkedNoCheckout"
            } else {
                "durable creation claim remains at Prepared"
            },
        ))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn verify_reset_complete_recovery(
        &self,
        marker: &CreationMarker,
    ) -> Result<(), WorktreeError> {
        use super::git_ops::worktree_list_contains_path;

        self.verify_creation_marker(&marker.id, marker)?;
        let dot_git = std::fs::read_to_string(marker.path.join(".git"))?;
        let admin_gitdir = std::fs::read_to_string(marker.admin_dir.join("gitdir"))?;
        let head = std::fs::read_to_string(marker.admin_dir.join("HEAD"))?;
        if dot_git.trim() != format!("gitdir: {}", marker.admin_dir.display())
            || admin_gitdir.trim() != marker.path.join(".git").to_string_lossy()
            || head.trim() != format!("ref: refs/heads/{}", marker.branch)
            || marker.admin_dir.join("locked").exists()
        {
            return Err(WorktreeError::IoError(std::io::Error::other(
                "ResetComplete claim cannot independently prove reciprocal unlocked worktree identity",
            )));
        }
        let branch_ref = format!("refs/heads/{}", marker.branch);
        let branch_oid =
            self.git_ref_oid(&branch_ref, true)
                .await?
                .ok_or_else(|| WorktreeError::GitFailed {
                    stderr: "completed claim branch disappeared".to_string(),
                })?;
        let worktree_head = self
            .git_probe_output_at(&marker.path, &["rev-parse", "HEAD"])
            .await?;
        if !worktree_head.status.success()
            || String::from_utf8_lossy(&worktree_head.stdout).trim() != marker.target_oid
            || branch_oid != marker.target_oid
        {
            return Err(WorktreeError::IoError(std::io::Error::other(
                "ResetComplete claim branch or worktree tip drifted",
            )));
        }
        let listed = self
            .git_probe_output_at(&self.config.repo_root, &["worktree", "list", "--porcelain"])
            .await?;
        if !listed.status.success() || !worktree_list_contains_path(&listed.stdout, &marker.path) {
            return Err(WorktreeError::IoError(std::io::Error::other(
                "ResetComplete claim is absent from git worktree registry",
            )));
        }
        Ok(())
    }
}
