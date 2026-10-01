//! Worktree manager — creates, removes, lists, and prunes linked Git
//! worktrees so the orchestrator can give each live plan its own isolated
//! working directory (parity §15).
//!
//! Mutating Git processes run with a no-descendant kernel resource profile.
//! Linked-worktree registration uses Git's documented administrative file
//! layout because some Git builds implement `worktree add` by spawning more
//! Git processes, which that containment profile intentionally denies.
//!
//! Cloned-manager mutations acquire an owned async reservation before handing
//! the complete operation to a runtime-independent worker thread. Once
//! acquired, that worker retains the reservation through Git process-tree exit
//! and registry reconciliation even if the public caller future is cancelled
//! or its Tokio runtime shuts down.
//!
//! ## Shipped features
//!
//! - §15.1–§15.2 Create / remove worktrees
//! - §15.3 Ephemeral branch naming ([`format_branch_name`])
//! - §15.4 Extended config: `max_live` budget, `idle_ttl`
//! - §15.5 Health checks ([`WorktreeHealth`])
//! - §15.6 Budget enforcement + idle reclamation
//! - §15.7 Stale lock detection ([`WorktreeManager::clear_stale_locks`])
//! - §15.8 Clean-only removal (via [`WorktreeManager::remove`])
//! - §15.9 Prune stale git metadata ([`WorktreeManager::prune`])

mod acceptance;
pub use acceptance::{ReviewDiff, attempt_review_diff};
mod cleanup;
pub use cleanup::clear_stale_index_lock;
mod creation_journal;
mod git_ops;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
#[cfg(test)]
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

use creation_journal::{
    CreationClaim, CreationMarker, CreationPhase, OperationLifecycle, RuntimeShutdownOwner,
    retain_lock_if_cleanup_unproved,
};
use git_ops::{
    OperationState, await_optional_deadline, await_owned_operation,
    await_owned_operation_controlled, ensure_git_success, isolate_worktree_config,
    reattach_rejected, validate_id, validate_reflex_replay_id,
};

/// Locks older than this are considered stale (§15.7).
///
/// Sourced from [`roko_core::defaults::DEFAULT_STALE_LOCK_SECS`].
pub(super) const STALE_LOCK_SECS: u64 = roko_core::defaults::DEFAULT_STALE_LOCK_SECS;

/// Minimum age (in seconds) before an unowned mutation lock file is
/// considered stuck and eligible for removal by
/// [`WorktreeManager::clear_stuck_mutation_lock`].
pub(super) const STUCK_MUTATION_LOCK_AGE_SECS: u64 = 300;

/// Maximum time caller-runtime shutdown waits for the independent worker.
///
/// Git tree cleanup has a shorter internal bound. If cleanup cannot prove the
/// tree absent, the worker intentionally retains the operation reservation so
/// later mutations fail closed instead of overlapping an unowned process.
pub(super) const RUNTIME_SHUTDOWN_WAIT: Duration = Duration::from_secs(5);

pub(super) const CREATION_MARKER_DIR: &str = ".roko-creation";
// Journal fsyncs provide process/kernel-crash restart convergence on supported
// macOS/Linux targets. Ordinary macOS fsync is not claimed as a power-loss
// persistence boundary (F_FULLFSYNC would be required for that stronger claim).
pub(super) const CREATION_MARKER_SCHEMA: u8 = 2;
pub(super) const CREATION_CLAIM_SUFFIX: &str = ".claim";
pub(super) const REPOSITORY_MUTATION_LOCK: &str = "roko-worktree-mutation.lock";

// Security boundary: atomic mkdir, flock, and fd-relative I/O protect against
// conforming concurrent Roko processes and pathname replacement races. No
// discretionary user-owned filesystem can defend against arbitrary hostile
// code running as the same effective UID; such code can delete any lock or
// claim inode. Root execution, UID mismatch, insecure modes, and foreign entry
// types therefore fail closed instead of being treated as recoverable.
// The permanent flock inode is anchored in the canonical Git common directory,
// so configurable checkout-output roots and linked-repository roots cannot
// partition repository ownership.

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InodeIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct TestPhaseBarrier {
    phase: CreationPhase,
    started: PathBuf,
    release: PathBuf,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TestClaimMutationPoint {
    BeforeBranchCas,
    BeforeTransitionWrite,
    BeforeRemovalCleanup,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(super) struct TestClaimMutationBarrier {
    pub(super) point: TestClaimMutationPoint,
    pub(super) started: PathBuf,
    pub(super) release: PathBuf,
}

/// Configuration handed to [`WorktreeManager::new`].
#[derive(Debug, Clone, Default)]
pub struct WorktreeConfig {
    /// Absolute path to the main repository checkout. `git worktree`
    /// commands are executed with this as their working directory.
    pub repo_root: PathBuf,
    /// Branch used as the starting point when no explicit HEAD is
    /// provided to `git worktree add`. Mirrors Mori's behaviour of
    /// branching from `main` / the plan base branch.
    pub base_branch: String,
    /// Directory under which new worktrees are materialised. Each
    /// worktree lives at `<worktrees_root>/<id>`.
    pub worktrees_root: PathBuf,
    /// Maximum simultaneously live worktrees. `None` = unlimited (§15.6).
    pub max_live: Option<usize>,
    /// After this duration without a [`WorktreeManager::touch`] call,
    /// a worktree becomes a candidate for [`WorktreeManager::reclaim_idle`]
    /// (§15.4).
    pub idle_ttl: Duration,
}

/// A live worktree tracked by the manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeHandle {
    /// Stable caller-assigned identifier (typically the plan id).
    pub id: String,
    /// Absolute path to the worktree's checkout directory.
    pub path: PathBuf,
    /// Branch checked out in the worktree.
    pub branch: String,
    /// Unix epoch milliseconds at which the handle was created.
    pub created_at_ms: i64,
    /// Unix epoch milliseconds of last activity. Updated by
    /// [`WorktreeManager::touch`].
    pub last_active_ms: i64,
}

/// Exact immutable commit accepted from a completed task attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedWorktree {
    /// Exact attempt checkout.
    pub handle: WorktreeHandle,
    /// Commit holding the attempt's work, on the attempt's own branch.
    pub attempt_commit: String,
    /// The plan branch's tip once the attempt was folded in: the base of the
    /// plan's later attempts.
    pub commit_oid: String,
}

/// The run a plan's attempts belong to in this process (bug-056b40).
#[derive(Debug, Clone)]
struct PlanRun {
    run_id: String,
    /// The plan branch's tip when the run continues it: the base of the
    /// plan's attempts until one is accepted in this process.
    continued_tip: Option<String>,
}

/// Why an attempt is accepted, recorded as trailers of the commits
/// [`WorktreeManager::accept_attempt`] writes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttemptAcceptance {
    /// Graph run the attempt belongs to. A plan branch whose tip names
    /// another run is not continued (see [`WorktreeManager::accept_attempt`]).
    pub run_id: String,
    /// Durable attempt key (`run:plan:task:ordinal`).
    pub attempt_key: String,
    /// The attempt's settled verdict (`passed`, `unverified`).
    pub verdict: String,
    /// Task title, for the commit subject.
    pub title: String,
}
/// Health of a tracked worktree (§15.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeHealth {
    /// Path exists and the expected branch is checked out.
    Ok,
    /// Worktree directory is missing from disk.
    Missing,
    /// A stale `.git/index.lock` was found (> 60 seconds old).
    StaleLock,
    /// HEAD is not on the expected branch (detached or switched).
    Detached,
}

/// Serializable registry snapshot for tracked worktrees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeSnapshot {
    /// Active handles known to the manager when the snapshot was taken.
    pub handles: Vec<WorktreeHandle>,
    /// Configured live-worktree budget at snapshot time.
    pub max_live: Option<usize>,
    /// Configured idle TTL in milliseconds.
    pub idle_ttl_ms: u64,
    /// Unix epoch milliseconds when the snapshot was produced.
    pub timestamp_ms: i64,
}

/// Health and isolation metadata for a tracked worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeIsolationStatus {
    /// The tracked worktree handle.
    pub handle: WorktreeHandle,
    /// Filesystem/git health of the worktree.
    pub health: WorktreeHealth,
    /// Milliseconds since the worktree was last touched.
    pub idle_ms: u64,
    /// Whether the handle exceeds the configured idle TTL.
    pub reclaimable: bool,
    /// Whether the worktree path exists on disk.
    pub path_exists: bool,
}

/// Errors returned by [`WorktreeManager`].
///
/// Every git invocation that exits non-zero surfaces its stderr
/// verbatim via [`WorktreeError::GitFailed`] so callers can log the
/// underlying failure without re-running the command.
#[derive(Debug, Error)]
pub enum WorktreeError {
    /// `git` returned a non-zero exit status. `stderr` is captured
    /// verbatim (lossily decoded if it was not valid UTF-8).
    #[error("git command failed: {stderr}")]
    GitFailed {
        /// Captured stderr from the failing git invocation.
        stderr: String,
    },
    /// Caller asked for a worktree that the manager doesn't track.
    #[error("worktree not found: {0}")]
    NotFound(String),
    /// Caller tried to create a worktree whose id is already live.
    #[error("worktree already exists: {0}")]
    AlreadyExists(String),
    /// Supplied identifier failed validation (empty, path separator, …).
    #[error("invalid worktree id: {0}")]
    InvalidId(String),
    /// Creating a new worktree would exceed [`WorktreeConfig::max_live`]
    /// (§15.6).
    #[error("max live worktrees reached ({max})")]
    BudgetExhausted {
        /// The configured cap.
        max: usize,
    },
    /// An accepted attempt's work conflicts with the work already on its
    /// plan branch. The plan branch did not move.
    #[error("attempt conflicts with `{branch}`; conflicted paths: {paths}")]
    Conflict {
        /// The plan branch.
        branch: String,
        /// Conflicted paths, comma-separated.
        paths: String,
    },
    /// Cleanup was requested for a dirty checkout.
    #[error("worktree `{id}` is dirty; preserving owned or unknown changes: {paths}")]
    DirtyWorktree {
        /// Checkout identifier.
        id: String,
        /// Porcelain status retained for recovery.
        paths: String,
    },
    /// Local filesystem error while preparing or removing a worktree.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    /// An existing canonical path could not be proved safe to reattach.
    #[error("cannot safely reattach worktree `{id}`: {reason}")]
    ReattachRejected {
        /// Requested plan/worktree identifier.
        id: String,
        /// Failed identity or metadata invariant.
        reason: String,
    },
    /// The platform, privilege state, executable, or repository configuration
    /// cannot satisfy the no-descendant Git execution contract.
    #[error("unsafe git execution policy: {reason}")]
    UnsafeGitExecution {
        /// Failed containment or extension invariant.
        reason: String,
    },
    /// Worktree mutations are on hold: an earlier operation could not prove
    /// its git process stopped, or another process holds the repository
    /// mutation lock too long. Returned at once instead of waiting forever
    /// (bug-53475e).
    #[error("worktree mutations are on hold: {reason}")]
    OwnershipRetained {
        /// The git processes that may still run, when known.
        pids: Vec<u32>,
        /// What to check or do.
        reason: String,
    },
}

/// Interruption-aware error returned while preparing an attempt worktree.
///
/// The cancellation variants only return after ownership of any started
/// mutation has been transferred to the manager's runtime-independent worker.
/// That worker retains the repository reservation until the mutation has
/// stopped and its journal has converged, so returning at a runner deadline
/// cannot expose an unowned Git writer.
#[derive(Debug, Error)]
pub enum WorktreeOperationError {
    /// The caller's cancellation token fired before preparation completed.
    #[error("worktree preparation cancelled")]
    Cancelled,
    /// The caller's absolute preparation deadline elapsed.
    #[error("worktree preparation deadline elapsed")]
    Deadline,
    /// Ordinary worktree validation or mutation failure.
    #[error(transparent)]
    Worktree(#[from] WorktreeError),
}

/// Derive the canonical branch name for a plan (§15.3).
///
/// Convention: `roko/plan/<plan_id>`. This keeps all roko-managed
/// branches under a single ref namespace, making cleanup and
/// enumeration straightforward.
#[must_use]
pub fn format_branch_name(plan_id: &str) -> String {
    format!("roko/plan/{plan_id}")
}

/// Derive the canonical branch name for a task within its plan.
///
/// Attempt-scoped branches use [`format_attempt_branch_name`], but this
/// compatibility helper remains part of the public worktree API.
#[must_use]
pub fn format_task_branch_name(plan_id: &str, task_id: &str) -> String {
    format!("roko/task/{plan_id}/{task_id}")
}

/// Collision-resistant manager ID for an exact task attempt.
pub fn format_attempt_worktree_id(plan_id: &str, task_id: &str, attempt: u32) -> String {
    let digest = blake3::hash(format!("{plan_id}\0{task_id}\0{attempt}").as_bytes());
    format!("attempt-{}", &digest.to_hex()[..20])
}
/// Format a worktree ID scoped to a namespace prefix.
pub fn format_scoped_attempt_worktree_id(
    namespace: &str,
    plan_id: &str,
    task_id: &str,
    attempt: u32,
) -> String {
    let digest = blake3::hash(format!("{namespace}\0{plan_id}\0{task_id}\0{attempt}").as_bytes());
    format!("attempt-{}", &digest.to_hex()[..20])
}

/// Branch owned by an exact task attempt.
pub fn format_attempt_branch_name(plan_id: &str, task_id: &str, attempt: u32) -> String {
    let id = format_attempt_worktree_id(plan_id, task_id, attempt);
    format!("roko/attempt/{id}")
}

/// Reject special workspace entries using stable no-follow directory handles.
pub fn validate_workspace_file_kinds(workdir: &Path, status: &[u8]) -> std::io::Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let ignored = status
            .split(|byte| *byte == 0)
            .filter_map(|record| record.strip_prefix(b"!! "))
            .map(|path| std::str::from_utf8(path).map(PathBuf::from))
            .collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)?;
        validate_workspace_file_kinds_with(workdir, &ignored, 4096, |_| {})
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (workdir, status);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "stable no-follow workspace traversal is unavailable on this platform",
        ))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn validate_workspace_file_kinds_with(
    workdir: &Path,
    ignored: &[PathBuf],
    max_dirs: usize,
    mut opened: impl FnMut(&Path),
) -> std::io::Result<()> {
    use creation_journal::{inode_identity, open_secure_directory_at};
    use rustix::fs::{AtFlags, FileType, Mode, OFlags};
    use std::io::Error;
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let root = rustix::fs::open(workdir, flags, Mode::empty()).map_err(std::io::Error::from)?;
    let mut pending = vec![(root, None, PathBuf::new())];
    for _ in 0..max_dirs {
        let Some((dir, binding, relative)) = pending.pop() else {
            return Ok(());
        };
        let before = inode_identity(&rustix::fs::fstat(&dir).map_err(std::io::Error::from)?);
        opened(&relative);
        let mut entries = rustix::fs::Dir::read_from(&dir).map_err(std::io::Error::from)?;
        for entry in &mut entries {
            let entry = entry.map_err(std::io::Error::from)?;
            let name = entry.file_name();
            if name.to_bytes() == b"." || name.to_bytes() == b".." {
                continue;
            }
            use std::os::unix::ffi::OsStrExt;
            let child_name = std::ffi::OsStr::from_bytes(name.to_bytes());
            let child = relative.join(child_name);
            if child == Path::new(".git") || ignored.iter().any(|path| child.starts_with(path)) {
                continue;
            }
            match open_secure_directory_at(&dir, name) {
                Ok(fd) => {
                    let parent = open_secure_directory_at(&dir, ".")?;
                    pending.push((fd, Some((parent, child_name.to_owned())), child));
                }
                Err(open_error) => {
                    let mode = rustix::fs::statat(&dir, name, AtFlags::SYMLINK_NOFOLLOW)
                        .map_err(std::io::Error::from)?
                        .st_mode;
                    match FileType::from_raw_mode(mode) {
                        FileType::Directory => return Err(open_error),
                        FileType::RegularFile | FileType::Symlink => {}
                        _ => return Err(Error::other("workspace contains a non-file input")),
                    }
                }
            }
        }
        if let Some((parent, name)) = binding {
            let public = open_secure_directory_at(&parent, name)?;
            let public = inode_identity(&rustix::fs::fstat(&public).map_err(std::io::Error::from)?);
            if before != public {
                return Err(Error::other("workspace directory changed during scan"));
            }
        }
    }
    Err(Error::other(
        "workspace directory count exceeds input limit",
    ))
}

/// Manages the lifecycle of per-plan git worktrees.
///
/// Clones of [`WorktreeManager`] share the same internal registry — the
/// handle is cheap to clone and safe to move across tasks.
#[derive(Clone, Debug)]
pub struct WorktreeManager {
    pub(super) config: Arc<WorktreeConfig>,
    pub(super) active: Arc<Mutex<HashMap<String, WorktreeHandle>>>,
    pub(super) accepted: Arc<Mutex<HashMap<String, AcceptedWorktree>>>,
    /// The run each plan's attempts belong to in this process, set by
    /// [`WorktreeManager::begin_plan_run`] (bug-056b40).
    plan_runs: Arc<Mutex<HashMap<String, PlanRun>>>,
    /// Shared fair reservation transferred into cancellation-independent
    /// tasks, and the ownership an unproved cleanup retained (bug-53475e).
    pub(super) operations: Arc<AsyncMutex<OperationState>>,
    /// Canonical executable selected once and shared by probes and mutations.
    pub(super) resolved_git_executable: Arc<Mutex<Option<PathBuf>>>,
    #[cfg(test)]
    pub(super) git_binary: Arc<Mutex<PathBuf>>,
    #[cfg(test)]
    pub(super) git_probe_environment: Arc<Mutex<Vec<(OsString, OsString)>>>,
    #[cfg(test)]
    phase_barrier: Arc<Mutex<Option<TestPhaseBarrier>>>,
    #[cfg(test)]
    pub(super) claim_mutation_barrier: Arc<Mutex<Option<TestClaimMutationBarrier>>>,
    #[cfg(test)]
    pub(super) force_cleanup_failure: Arc<AtomicBool>,
}

impl WorktreeManager {
    /// Construct a new manager. The caller is responsible for making
    /// sure `config.repo_root` is a real git repository — validation is
    /// lazy: the first git command that runs inside the worktree root
    /// will surface any misconfiguration as [`WorktreeError::GitFailed`].
    #[must_use]
    pub fn new(config: WorktreeConfig) -> Self {
        Self {
            config: Arc::new(config),
            active: Arc::new(Mutex::new(HashMap::new())),
            accepted: Arc::new(Mutex::new(HashMap::new())),
            plan_runs: Arc::new(Mutex::new(HashMap::new())),
            operations: Arc::new(AsyncMutex::new(OperationState::default())),
            resolved_git_executable: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            git_binary: Arc::new(Mutex::new(PathBuf::from("git"))),
            #[cfg(test)]
            git_probe_environment: Arc::new(Mutex::new(Vec::new())),
            #[cfg(test)]
            phase_barrier: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            claim_mutation_barrier: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            force_cleanup_failure: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Restore a manager registry from a snapshot.
    ///
    /// This does not create or remove git worktrees. It only reconstructs the
    /// in-memory registry so callers can validate and reconcile handles with
    /// [`isolation_statuses`](Self::isolation_statuses).
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::InvalidId`] if any snapshot handle contains an
    /// invalid id, or [`WorktreeError::AlreadyExists`] if the snapshot contains
    /// duplicate ids.
    pub fn from_snapshot(
        config: WorktreeConfig,
        snapshot: WorktreeSnapshot,
    ) -> Result<Self, WorktreeError> {
        let mut active = HashMap::with_capacity(snapshot.handles.len());
        for handle in snapshot.handles {
            validate_id(&handle.id)?;
            let id = handle.id.clone();
            if active.insert(id.clone(), handle).is_some() {
                return Err(WorktreeError::AlreadyExists(id));
            }
        }

        Ok(Self {
            config: Arc::new(config),
            active: Arc::new(Mutex::new(active)),
            accepted: Arc::new(Mutex::new(HashMap::new())),
            plan_runs: Arc::new(Mutex::new(HashMap::new())),
            operations: Arc::new(AsyncMutex::new(OperationState::default())),
            resolved_git_executable: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            git_binary: Arc::new(Mutex::new(PathBuf::from("git"))),
            #[cfg(test)]
            git_probe_environment: Arc::new(Mutex::new(Vec::new())),
            #[cfg(test)]
            phase_barrier: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            claim_mutation_barrier: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            force_cleanup_failure: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Return the configured repository root path.
    #[must_use]
    pub fn repo_root(&self) -> &Path {
        &self.config.repo_root
    }

    /// Return the configured base branch name.
    #[must_use]
    pub fn base_branch(&self) -> &str {
        &self.config.base_branch
    }

    /// Compute the path the manager would use for `id`. This is a pure
    /// function of the config and does not touch the filesystem.
    #[must_use]
    pub fn path_for(&self, id: &str) -> PathBuf {
        self.config.worktrees_root.join(id)
    }

    /// Create a worktree for `id` checked out onto `branch`.
    ///
    /// `branch` is created from [`WorktreeConfig::base_branch`] if it
    /// does not already exist. Rejects duplicate ids, invalid id
    /// strings (empty, containing `/`, `\`, NUL, or leading `.`), and
    /// requests that would exceed [`WorktreeConfig::max_live`].
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::InvalidId`] if `id` fails validation,
    /// [`WorktreeError::AlreadyExists`] if the id is already tracked,
    /// [`WorktreeError::BudgetExhausted`] if the live-worktree limit would
    /// be exceeded, [`WorktreeError::GitFailed`] if a contained Git mutation
    /// exits unsuccessfully, or [`WorktreeError::IoError`] if preparing the
    /// linked-worktree metadata or directory fails.
    pub async fn create(&self, id: &str, branch: &str) -> Result<WorktreeHandle, WorktreeError> {
        validate_id(id)?;
        self.reject_legacy_creation_marker(id)?;
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        let id = id.to_string();
        let branch = branch.to_string();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let base = manager.config.base_branch.clone();
            let result = manager.create_locked(&id, &branch, &base, &lifecycle).await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    pub(super) async fn create_locked(
        &self,
        id: &str,
        branch: &str,
        base: &str,
        lifecycle: &OperationLifecycle,
    ) -> Result<WorktreeHandle, WorktreeError> {
        validate_id(id)?;
        self.reject_outstanding_creation_marker(id).await?;
        let _ = self.clear_stale_locks_unlocked();

        // Reserve the slot up-front so racing callers conflict cleanly.
        {
            let guard = self.active.lock();
            if guard.contains_key(id) {
                return Err(WorktreeError::AlreadyExists(id.to_string()));
            }
            // §15.6 — enforce budget before touching git.
            if let Some(max) = self.config.max_live {
                if guard.len() >= max {
                    return Err(WorktreeError::BudgetExhausted { max });
                }
            }
        }

        let path = self.path_for(id);

        self.validate_git_policy(true).await?;
        let common_git_dir = self.common_git_dir().await?;
        let target_oid =
            self.git_ref_oid(base, true)
                .await?
                .ok_or_else(|| WorktreeError::GitFailed {
                    stderr: format!("base ref `{base}` does not resolve"),
                })?;
        let branch_old_oid = self.git_ref_oid(branch, false).await?;
        if branch_old_oid.is_some() {
            let listed = self
                .git_probe_output_at(&self.config.repo_root, &["worktree", "list", "--porcelain"])
                .await?;
            if !listed.status.success() {
                return Err(WorktreeError::GitFailed {
                    stderr: String::from_utf8_lossy(&listed.stderr).into_owned(),
                });
            }
            let branch_ref = format!("refs/heads/{branch}");
            if git_ops::worktree_list_contains_branch(&listed.stdout, &branch_ref) {
                return Err(reattach_rejected(
                    id,
                    format!("branch `{branch}` is already checked out in this repository"),
                ));
            }
        }
        let admin_dir = common_git_dir
            .join("worktrees")
            .join(format!("roko-{}", uuid::Uuid::new_v4().simple()));

        // Make sure `worktrees_root` exists before git tries to write.
        tokio::fs::create_dir_all(&self.config.worktrees_root).await?;
        let marker = CreationMarker {
            schema_version: CREATION_MARKER_SCHEMA,
            claim_id: uuid::Uuid::new_v4().simple().to_string(),
            id: id.to_string(),
            repo_root: self.config.repo_root.clone(),
            common_git_dir: common_git_dir.clone(),
            branch: branch.to_string(),
            branch_old_oid,
            target_oid,
            path: path.clone(),
            admin_dir: admin_dir.clone(),
            phase: CreationPhase::Prepared,
            previous_digest: None,
        };
        let mut claim = self
            .publish_creation_marker(marker)
            .map_err(|error| self.creation_marker_publication_error(id, error))?;

        if let Err(error) = self.create_git_phases(&mut claim, lifecycle).await {
            if let Err(cleanup_error) = self.rollback_incomplete_create(&claim).await {
                lifecycle.mark_cleanup_unproved();
                return Err(WorktreeError::IoError(std::io::Error::new(
                    cleanup_error.kind(),
                    format!(
                        "create failed ({error}); rollback could not be proved: {cleanup_error}"
                    ),
                )));
            }
            return Err(error);
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        let handle = WorktreeHandle {
            id: id.to_string(),
            path,
            branch: branch.to_string(),
            created_at_ms: now_ms,
            last_active_ms: now_ms,
        };

        let conflict = {
            let mut guard = self.active.lock();
            // Double-check in case a concurrent caller inserted first.
            if guard.contains_key(id) {
                true
            } else {
                guard.insert(id.to_string(), handle.clone());
                false
            }
        };

        if conflict {
            // Best-effort rollback of the git side; ignore failures
            // since the real winner owns the worktree now.
            let _ = self.git_remove(&handle.path, lifecycle).await;
            return Err(WorktreeError::AlreadyExists(id.to_string()));
        }

        // G08: Copy isolated config directories into the new worktree so
        // concurrent agents do not contend on shared config files. We copy
        // (not symlink) to prevent contention. Failure is non-fatal.
        isolate_worktree_config(&self.config.repo_root, &handle.path);

        Ok(handle)
    }

    pub(super) async fn create_git_phases(
        &self,
        claim: &mut CreationClaim,
        lifecycle: &OperationLifecycle,
    ) -> Result<(), WorktreeError> {
        let marker = &claim.marker;
        let branch = marker.branch.clone();
        let path = marker.path.clone();
        let admin_dir = marker.admin_dir.clone();
        let old_oid = marker.branch_old_oid.clone().unwrap_or_else(|| {
            // The repository object format is proved by target_oid length;
            // update-ref accepts an all-zero old value as "must not exist".
            "0".repeat(marker.target_oid.len())
        });
        let expected_ref = format!("refs/heads/{branch}");
        #[cfg(test)]
        self.test_claim_mutation_checkpoint(TestClaimMutationPoint::BeforeBranchCas);
        self.verify_live_creation_claim(claim)?;
        let branch_output = self
            .git_mutation_output_at(
                &self.config.repo_root,
                &[
                    "update-ref",
                    "--no-deref",
                    &expected_ref,
                    &marker.target_oid,
                    &old_oid,
                ],
                lifecycle,
                true,
            )
            .await?;
        ensure_git_success(branch_output)?;
        self.verify_live_creation_claim(claim)?;
        self.register_linked_worktree(&path, &admin_dir, &expected_ref)?;
        self.verify_live_creation_claim(claim)?;
        self.transition_creation_marker(claim, CreationPhase::LinkedNoCheckout)?;
        self.test_phase_checkpoint(CreationPhase::LinkedNoCheckout, lifecycle)
            .await?;

        self.verify_live_creation_claim(claim)?;
        let reset = self
            .git_mutation_output_at(
                &path,
                &["reset", "--hard", "--no-recurse-submodules", &expected_ref],
                lifecycle,
                true,
            )
            .await?;
        ensure_git_success(reset)?;
        self.verify_live_creation_claim(claim)?;
        let locked = admin_dir.join("locked");
        self.verify_live_creation_claim(claim)?;
        std::fs::remove_file(&locked)?;
        git_ops::sync_directory(&admin_dir)?;
        self.verify_live_creation_claim(claim)?;
        self.transition_creation_marker(claim, CreationPhase::ResetComplete)?;

        // Once reset succeeded, runtime shutdown must reconcile the completed
        // checkout instead of rolling it back. The checkpoint is test-only;
        // production proceeds synchronously to the durable marker removal.
        let _ = self
            .test_phase_checkpoint(CreationPhase::ResetComplete, lifecycle)
            .await;
        self.remove_completed_creation_marker(claim)?;
        Ok(())
    }

    /// Convenience: create a worktree using the canonical branch naming
    /// convention (`roko/plan/<plan_id>`). §15.3
    ///
    /// # Errors
    ///
    /// Returns the same [`WorktreeError`] variants as
    /// [`WorktreeManager::create`].
    pub async fn create_for_plan(&self, plan_id: &str) -> Result<WorktreeHandle, WorktreeError> {
        let branch = format_branch_name(plan_id);
        self.create(plan_id, &branch).await
    }

    /// Create an attempt checkout from its plan's last accepted immutable tip.
    ///
    /// A checkout of the attempt that an earlier process of the plan's run
    /// kept ([`WorktreeManager::begin_plan_run`]) is re-attached instead
    /// (bug-056b40). One kept by another run is refused, as before.
    pub async fn create_for_attempt(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
    ) -> Result<WorktreeHandle, WorktreeError> {
        let id = format_attempt_worktree_id(plan_id, task_id, attempt);
        let branch = format_attempt_branch_name(plan_id, task_id, attempt);
        let base = self.attempt_base(plan_id);
        let run_id = self
            .plan_runs
            .lock()
            .get(plan_id)
            .map(|run| run.run_id.clone());
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager
                .create_attempt_locked(&id, &branch, &base, run_id.as_deref(), &lifecycle)
                .await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    /// Base of `plan_id`'s next attempt: the plan branch's tip after the
    /// plan's last acceptance in this process, else the tip its resumed run
    /// continues ([`WorktreeManager::begin_plan_run`]), else
    /// [`WorktreeConfig::base_branch`].
    fn attempt_base(&self, plan_id: &str) -> String {
        if let Some(accepted) = self.accepted.lock().get(plan_id) {
            return accepted.commit_oid.clone();
        }
        self.plan_runs
            .lock()
            .get(plan_id)
            .and_then(|run| run.continued_tip.clone())
            .unwrap_or_else(|| self.config.base_branch.clone())
    }

    /// Ensure an exact attempt checkout is tracked, safely reattaching the
    /// canonical attempt branch after process restart when it already exists.
    pub async fn ensure_for_attempt(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
    ) -> Result<WorktreeHandle, WorktreeError> {
        let id = format_attempt_worktree_id(plan_id, task_id, attempt);
        let branch = format_attempt_branch_name(plan_id, task_id, attempt);
        let base = self.attempt_base(plan_id);
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = if manager.active.lock().contains_key(&id) {
                manager
                    .try_reattach_locked_with_branch(&id, &branch)
                    .await?
                    .ok_or_else(|| WorktreeError::NotFound(id.clone()))
            } else if let Some(handle) = manager
                .try_reattach_locked_with_branch(&id, &branch)
                .await?
            {
                Ok(handle)
            } else {
                manager.create_locked(&id, &branch, &base, &lifecycle).await
            };
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    /// Ensure an exact attempt checkout while observing runner cancellation
    /// and an absolute deadline through both reservation and Git preparation.
    ///
    /// If interrupted after the worker starts, cancellation is signalled to
    /// its mutation lifecycle before this future returns. The worker continues
    /// to own the repository reservation until contained Git descendants have
    /// exited and creation-journal reconciliation is complete.
    pub async fn ensure_for_attempt_controlled(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        cancel: &CancellationToken,
        deadline: Option<tokio::time::Instant>,
    ) -> Result<WorktreeHandle, WorktreeOperationError> {
        let id = format_attempt_worktree_id(plan_id, task_id, attempt);
        let branch = format_attempt_branch_name(plan_id, task_id, attempt);
        let base = self.attempt_base(plan_id);
        let operation = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(WorktreeOperationError::Cancelled),
            _ = await_optional_deadline(deadline) => {
                return Err(WorktreeOperationError::Deadline);
            }
            operation = Arc::clone(&self.operations).lock_owned() => operation,
        };
        let manager = self.clone();
        await_owned_operation_controlled(
            operation,
            move |lifecycle| async move {
                let repository_lock = manager.acquire_repository_mutation_lock()?;
                let result = if manager.active.lock().contains_key(&id) {
                    manager
                        .try_reattach_locked_with_branch(&id, &branch)
                        .await?
                        .ok_or_else(|| WorktreeError::NotFound(id.clone()))
                } else if let Some(handle) = manager
                    .try_reattach_locked_with_branch(&id, &branch)
                    .await?
                {
                    Ok(handle)
                } else {
                    manager.create_locked(&id, &branch, &base, &lifecycle).await
                };
                retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
                result
            },
            cancel,
            deadline,
        )
        .await
    }
    /// Return the checkout owned by an exact task attempt.
    pub fn get_attempt(&self, plan: &str, task: &str, attempt: u32) -> Option<WorktreeHandle> {
        self.get(&format_attempt_worktree_id(plan, task, attempt))
    }
    /// Accept an exact attempt (gap-3b5361): commit what its checkout holds
    /// on its attempt branch, fold that commit into the plan branch
    /// ([`format_branch_name`]), and make the plan branch's new tip the base
    /// of the plan's later attempts.
    ///
    /// Only the attempt's own checkout, the object database and the plan
    /// branch change: every step is plumbing, and no other checkout's
    /// branch, index or files are touched. The plan branch moves only by
    /// compare-and-swap: a fast-forward when it has not moved since the
    /// attempt started, else a merge computed with `git merge-tree`. A
    /// conflict ([`WorktreeError::Conflict`]) leaves the plan branch where it
    /// was, and a plan branch checked out anywhere is never moved, since that
    /// checkout would no longer match its HEAD.
    ///
    /// The first acceptance of a plan in this process continues its branch
    /// only when the branch's tip names the same run (a resumed run). A
    /// branch left by another run is kept under
    /// `refs/roko/plan-archive/<plan_id>/<tip>`, and the plan branch starts
    /// afresh from this attempt.
    pub async fn accept_attempt(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        acceptance: &AttemptAcceptance,
    ) -> Result<AcceptedWorktree, WorktreeError> {
        let id = format_attempt_worktree_id(plan_id, task_id, attempt);
        let handle = self
            .get(&id)
            .ok_or_else(|| WorktreeError::NotFound(id.clone()))?;
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        let plan_id = plan_id.to_string();
        let task_id = task_id.to_string();
        let acceptance = acceptance.clone();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager
                .accept_locked(&plan_id, &task_id, handle, &acceptance, &lifecycle)
                .await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }
    /// Last accepted attempt for a plan, used by plan verification and merge.
    pub fn accepted_for_plan(&self, plan_id: &str) -> Option<AcceptedWorktree> {
        self.accepted.lock().get(plan_id).cloned()
    }

    /// Get a tracked worktree handle by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<WorktreeHandle> {
        self.active.lock().get(id).cloned()
    }

    /// Ensure a plan worktree exists and return its handle.
    ///
    /// If the worktree is already tracked, this touches and returns it.
    /// If it exists on disk but isn't tracked (e.g. after resume), it is
    /// re-registered. Otherwise a new canonical plan worktree is created.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::ReattachRejected`] when the canonical path
    /// exists but cannot be proved to be the exact same-repository plan
    /// worktree, or any [`WorktreeError`] that can be produced by
    /// [`WorktreeManager::create_for_plan`]. Existing unsafe paths are never
    /// replaced or removed.
    pub async fn ensure_for_plan(&self, plan_id: &str) -> Result<WorktreeHandle, WorktreeError> {
        validate_id(plan_id)?;
        self.reject_legacy_creation_marker(plan_id)?;
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        let plan_id = plan_id.to_string();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager.ensure_for_plan_locked(&plan_id, &lifecycle).await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    async fn ensure_for_plan_locked(
        &self,
        plan_id: &str,
        lifecycle: &OperationLifecycle,
    ) -> Result<WorktreeHandle, WorktreeError> {
        let already_tracked = self.active.lock().contains_key(plan_id);
        if already_tracked {
            if self.try_reattach_locked(plan_id).await?.is_none() {
                return Err(reattach_rejected(
                    plan_id,
                    "tracked worktree path is missing",
                ));
            }
            let mut guard = self.active.lock();
            if let Some(handle) = guard.get_mut(plan_id) {
                handle.last_active_ms = chrono::Utc::now().timestamp_millis();
                return Ok(handle.clone());
            }
            return Err(WorktreeError::NotFound(plan_id.to_string()));
        }

        // Safety net: if the worktree exists on disk but wasn't tracked
        // (e.g. resume without discover_existing), re-register it instead
        // of trying `git worktree add` which would fail.
        if let Some(handle) = self.try_reattach_locked(plan_id).await? {
            return Ok(handle);
        }

        let branch = format_branch_name(plan_id);
        let base = self.config.base_branch.clone();
        self.create_locked(plan_id, &branch, &base, lifecycle).await
    }

    /// Scan the worktrees root for directories matching `plan_ids` that
    /// exist on disk but are not yet tracked. Valid worktrees are
    /// re-registered in the in-memory map. Invalid identifiers and existing
    /// candidates that fail identity validation are skipped without mutation.
    ///
    /// Returns the list of plan IDs that were successfully re-discovered.
    pub async fn discover_existing(&self, plan_ids: &[&str]) -> Vec<String> {
        if !plan_ids.iter().any(|plan_id| validate_id(plan_id).is_ok()) {
            return Vec::new();
        }
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        let plan_ids = plan_ids
            .iter()
            .map(|plan_id| (*plan_id).to_string())
            .collect::<Vec<_>>();
        match await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = Ok(manager.discover_existing_locked(&plan_ids).await);
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
        {
            Ok(discovered) => discovered,
            Err(error) => {
                tracing::debug!(%error, "owned worktree discovery task failed");
                Vec::new()
            }
        }
    }

    async fn discover_existing_locked(&self, plan_ids: &[String]) -> Vec<String> {
        let mut discovered = Vec::new();
        for plan_id in plan_ids {
            // Already tracked — nothing to do.
            if self.get(plan_id).is_some() {
                continue;
            }
            match self.try_reattach_locked(plan_id).await {
                Ok(Some(_handle)) => discovered.push(plan_id.clone()),
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!(plan_id, error = %error, "skipping unsafe worktree reattachment");
                }
            }
        }
        discovered
    }

    /// Return the active worktree path for `plan_id` if tracked.
    #[must_use]
    pub fn plan_path(&self, plan_id: &str) -> Option<PathBuf> {
        self.get(plan_id).map(|h| h.path)
    }

    /// Remove the worktree tracked under `id`. Errors if `id` isn't
    /// tracked. The underlying git directory is removed via
    /// Refuses to remove a dirty checkout so owned or unknown changes remain
    /// available for attribution and recovery.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::NotFound`] if the id is not tracked,
    /// [`WorktreeError::DirtyWorktree`] if the checkout has changes,
    /// [`WorktreeError::GitFailed`] if `git worktree remove` exits
    /// unsuccessfully, or [`WorktreeError::IoError`] if invoking `git` fails.
    pub async fn remove(&self, id: &str) -> Result<(), WorktreeError> {
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        let id = id.to_string();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager.remove_locked(&id, &lifecycle).await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    /// Force-remove a dirty checkout owned by the reflex replay namespace.
    ///
    /// Ordinary worktrees must use [`Self::remove`], which deliberately
    /// preserves dirty state. A reflex replay is different: its dirty delta
    /// is an ephemeral proof artifact that has already been fingerprinted and
    /// gated. This method fails closed unless both the manager ID and tracked
    /// branch have the exact runner-owned replay shape.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::InvalidId`] for a non-replay identifier,
    /// [`WorktreeError::ReattachRejected`] when the tracked handle is not the
    /// exact replay checkout, or the same Git/I/O errors as [`Self::remove`].
    pub async fn remove_reflex_replay(&self, id: &str) -> Result<(), WorktreeError> {
        validate_reflex_replay_id(id)?;
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        let id = id.to_string();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager.remove_reflex_replay_locked(&id, &lifecycle).await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    async fn remove_reflex_replay_locked(
        &self,
        id: &str,
        lifecycle: &OperationLifecycle,
    ) -> Result<(), WorktreeError> {
        let _ = self.clear_stale_locks_unlocked();
        self.validate_git_policy(false).await?;
        let handle = self
            .active
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| WorktreeError::NotFound(id.to_string()))?;
        let expected_branch = format!("roko/reflex-replay/{id}");
        if handle.branch != expected_branch || handle.path != self.path_for(id) {
            return Err(WorktreeError::ReattachRejected {
                id: id.to_string(),
                reason: "tracked handle is not the exact owned reflex replay".to_string(),
            });
        }
        if let Err(error) = self.git_remove(&handle.path, lifecycle).await {
            if !handle.path.exists() {
                self.active.lock().remove(id);
            }
            return Err(error);
        }
        self.active.lock().remove(id);
        Ok(())
    }

    pub(super) async fn remove_locked(
        &self,
        id: &str,
        lifecycle: &OperationLifecycle,
    ) -> Result<(), WorktreeError> {
        let _ = self.clear_stale_locks_unlocked();
        self.validate_git_policy(false).await?;
        let handle = {
            let guard = self.active.lock();
            guard
                .get(id)
                .cloned()
                .ok_or_else(|| WorktreeError::NotFound(id.to_string()))?
        };

        let args = ["status", "--porcelain", "--untracked-files=all"];
        // A clean `git status --porcelain` succeeds with intentionally empty
        // stdout, so it cannot use `git_probe_stdout_at`, whose contract
        // rejects empty output for identity probes such as `rev-parse`.
        let output = self.git_probe_output_at(&handle.path, &args).await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        let status = String::from_utf8(output.stdout).map_err(|_| {
            WorktreeError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "git status returned non-UTF-8 output",
            ))
        })?;
        if !status.trim().is_empty() {
            return Err(WorktreeError::DirtyWorktree {
                id: id.to_string(),
                paths: status.trim().to_string(),
            });
        }
        if let Err(error) = self.git_remove(&handle.path, lifecycle).await {
            // A runtime-shutdown cancellation may race with Git after it has
            // removed the directory. Reconcile the registry from disk before
            // operation ownership can be released.
            if !handle.path.exists() {
                self.active.lock().remove(id);
            }
            return Err(error);
        }
        self.active.lock().remove(id);

        Ok(())
    }

    /// Snapshot of every worktree currently tracked by the manager.
    /// Does **not** consult `git worktree list` — it reports the
    /// in-memory registry only.
    ///
    /// # Errors
    ///
    /// This function is currently infallible and always returns
    /// `Ok(...)`; the `Result` wrapper is kept for API symmetry with the
    /// rest of the manager surface.
    pub fn list(&self) -> Result<Vec<WorktreeHandle>, WorktreeError> {
        let mut out: Vec<WorktreeHandle> = {
            let guard = self.active.lock();
            guard.values().cloned().collect()
        };
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Snapshot the in-memory worktree registry.
    ///
    /// The snapshot intentionally records the registry, not the result of
    /// `git worktree list`. Use [`isolation_statuses`](Self::isolation_statuses)
    /// after restore to detect missing or unhealthy paths.
    #[must_use]
    pub fn snapshot(&self, timestamp_ms: i64) -> WorktreeSnapshot {
        let mut handles: Vec<_> = self.active.lock().values().cloned().collect();
        handles.sort_by(|a, b| a.id.cmp(&b.id));
        WorktreeSnapshot {
            handles,
            max_live: self.config.max_live,
            idle_ttl_ms: git_ops::duration_millis_u64(self.config.idle_ttl),
            timestamp_ms,
        }
    }

    /// Number of active worktrees currently tracked in memory.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.lock().len()
    }

    /// Bump the last-active timestamp for `id` (§15.4).
    ///
    /// Called by the orchestrator when agents write to a worktree so
    /// idle-time reclamation skips actively-used worktrees.
    pub fn touch(&self, id: &str) {
        let mut guard = self.active.lock();
        if let Some(handle) = guard.get_mut(id) {
            handle.last_active_ms = chrono::Utc::now().timestamp_millis();
        }
    }

    /// Remove all currently tracked worktrees.
    ///
    /// Returns the ids that were successfully removed.
    ///
    /// # Errors
    ///
    /// This function is currently infallible and returns `Ok(...)` after
    /// best-effort removal; per-worktree failures are ignored.
    pub async fn remove_all(&self) -> Result<Vec<String>, WorktreeError> {
        let ids: Vec<String> = self.active.lock().keys().cloned().collect();
        let mut removed = Vec::new();
        for id in ids {
            if self.remove(&id).await.is_ok() {
                removed.push(id);
            }
        }
        removed.sort();
        Ok(removed)
    }

    /// Try to re-register a worktree that exists on disk but is not
    /// tracked. Returns `Some(handle)` only when the candidate belongs to
    /// this manager's repository and has the exact canonical plan branch at
    /// its current tip. Mismatched or detached worktrees fail closed.
    pub(super) async fn try_reattach_locked(
        &self,
        plan_id: &str,
    ) -> Result<Option<WorktreeHandle>, WorktreeError> {
        let expected_branch = format_branch_name(plan_id);
        self.try_reattach_locked_with_branch(plan_id, &expected_branch)
            .await
    }

    pub(super) async fn try_reattach_locked_with_branch(
        &self,
        plan_id: &str,
        expected_branch: &str,
    ) -> Result<Option<WorktreeHandle>, WorktreeError> {
        validate_id(plan_id)?;
        let path = self.path_for(plan_id);
        self.reject_outstanding_creation_marker(plan_id).await?;
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(reattach_rejected(plan_id, error.to_string())),
        };
        if metadata.file_type().is_symlink() {
            return Err(reattach_rejected(plan_id, "candidate path is a symlink"));
        }
        if !metadata.is_dir() {
            return Err(reattach_rejected(
                plan_id,
                "candidate path is not a directory",
            ));
        }

        let canonical_root = std::fs::canonicalize(&self.config.worktrees_root)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        let canonical_path = std::fs::canonicalize(&path)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if canonical_path.parent() != Some(canonical_root.as_path())
            || canonical_path.file_name() != Some(std::ffi::OsStr::new(plan_id))
        {
            return Err(reattach_rejected(
                plan_id,
                "candidate is not the exact canonical child path",
            ));
        }

        // A git worktree has a `.git` *file* (not directory) pointing
        // back to the main repo's worktree metadata.
        let git_file = path.join(".git");
        let git_file_is_regular = std::fs::symlink_metadata(&git_file)
            .is_ok_and(|metadata| metadata.file_type().is_file());
        if !git_file_is_regular {
            return Err(reattach_rejected(
                plan_id,
                "candidate has no regular worktree .git file",
            ));
        }

        let configured_common_dir = self
            .git_probe_common_dir_at(&self.config.repo_root)
            .await
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        let admin_pointer = git_ops::read_gitdir(&path).ok_or_else(|| {
            reattach_rejected(plan_id, "candidate .git file has no valid gitdir pointer")
        })?;
        let admin_metadata = std::fs::symlink_metadata(&admin_pointer)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if admin_metadata.file_type().is_symlink() || !admin_metadata.is_dir() {
            return Err(reattach_rejected(
                plan_id,
                "candidate gitdir pointer is not a regular administrative directory",
            ));
        }
        let admin_dir = std::fs::canonicalize(&admin_pointer)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if admin_dir.parent() != Some(configured_common_dir.join("worktrees").as_path()) {
            return Err(reattach_rejected(
                plan_id,
                "candidate administrative directory is outside the configured common Git directory",
            ));
        }
        let reciprocal_path = admin_dir.join("gitdir");
        let reciprocal_metadata = std::fs::symlink_metadata(&reciprocal_path)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if reciprocal_metadata.file_type().is_symlink() || !reciprocal_metadata.is_file() {
            return Err(reattach_rejected(
                plan_id,
                "candidate administrative gitdir link is not a regular file",
            ));
        }
        let reciprocal_raw = std::fs::read_to_string(&reciprocal_path)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        let reciprocal = PathBuf::from(reciprocal_raw.trim());
        let reciprocal = if reciprocal.is_absolute() {
            reciprocal
        } else {
            admin_dir.join(reciprocal)
        };
        let canonical_git_file = std::fs::canonicalize(&git_file)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        let canonical_reciprocal = std::fs::canonicalize(reciprocal)
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if canonical_reciprocal != canonical_git_file {
            return Err(reattach_rejected(
                plan_id,
                "candidate and administrative gitdir links are not reciprocal",
            ));
        }

        let top_level = self
            .git_probe_canonical_path_at(&path, &["rev-parse", "--show-toplevel"])
            .await
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if top_level != canonical_path {
            return Err(reattach_rejected(
                plan_id,
                "git top-level does not match the canonical candidate path",
            ));
        }

        let candidate_common_dir = self
            .git_probe_common_dir_at(&path)
            .await
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if candidate_common_dir != configured_common_dir {
            return Err(reattach_rejected(
                plan_id,
                "candidate belongs to a different git repository",
            ));
        }

        let branch = self
            .git_probe_stdout_at(&path, &["symbolic-ref", "--quiet", "--short", "HEAD"])
            .await
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if branch != expected_branch {
            return Err(reattach_rejected(
                plan_id,
                format!("expected branch `{expected_branch}`, found `{branch}`"),
            ));
        }

        let head = self
            .git_probe_stdout_at(&path, &["rev-parse", "HEAD"])
            .await
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        let expected_ref = format!("refs/heads/{expected_branch}");
        let branch_head = self
            .git_probe_stdout_at(&path, &["rev-parse", &expected_ref])
            .await
            .map_err(|error| reattach_rejected(plan_id, error.to_string()))?;
        if head != branch_head {
            return Err(reattach_rejected(
                plan_id,
                "candidate HEAD does not match the canonical branch tip",
            ));
        }

        // Use directory mtime as a proxy for creation/activity timestamps.
        let mtime_ms = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            })
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        let now_ms = chrono::Utc::now().timestamp_millis();
        let handle = WorktreeHandle {
            id: plan_id.to_string(),
            path,
            branch,
            created_at_ms: mtime_ms.min(now_ms),
            last_active_ms: now_ms,
        };

        let mut guard = self.active.lock();
        // Double-check: another caller may have inserted concurrently.
        if let Some(existing) = guard.get(plan_id) {
            if existing.id != handle.id
                || existing.path != handle.path
                || existing.branch != handle.branch
            {
                return Err(reattach_rejected(
                    plan_id,
                    "tracked registry handle does not match the canonical candidate",
                ));
            }
            return Ok(Some(existing.clone()));
        }
        guard.insert(plan_id.to_string(), handle.clone());
        Ok(Some(handle))
    }

    pub(super) async fn rollback_incomplete_create(
        &self,
        claim: &CreationClaim,
    ) -> std::io::Result<()> {
        #[cfg(test)]
        if self.force_cleanup_failure.load(Ordering::Acquire) {
            return Err(std::io::Error::other("injected create rollback failure"));
        }

        self.verify_live_creation_claim(claim)?;
        self.verify_creation_marker(&claim.marker.id, &claim.marker)?;
        self.remove_registered_worktree(&claim.marker.path, &claim.marker.admin_dir)?;
        self.remove_creation_claim_if_exact(claim)
    }

    pub(super) fn status_from_handle(
        &self,
        handle: WorktreeHandle,
        health: WorktreeHealth,
    ) -> WorktreeIsolationStatus {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let idle_ms = u64::try_from(now_ms.saturating_sub(handle.last_active_ms)).unwrap_or(0);
        let ttl_ms = git_ops::duration_millis_u64(self.config.idle_ttl);
        let reclaimable = idle_ms > ttl_ms;
        let path_exists = handle.path.exists();
        WorktreeIsolationStatus {
            handle,
            health,
            idle_ms,
            reclaimable,
            path_exists,
        }
    }

    #[cfg(test)]
    pub(super) fn set_test_git_binary(&self, executable: PathBuf) {
        *self.git_binary.lock() = executable;
        *self.resolved_git_executable.lock() = None;
    }

    #[cfg(test)]
    pub(super) fn set_test_git_probe_environment(&self, environment: Vec<(OsString, OsString)>) {
        *self.git_probe_environment.lock() = environment;
    }

    #[cfg(test)]
    fn set_test_phase_barrier(&self, barrier: TestPhaseBarrier) {
        *self.phase_barrier.lock() = Some(barrier);
    }

    #[cfg(test)]
    pub(super) fn set_test_claim_mutation_barrier(&self, barrier: TestClaimMutationBarrier) {
        *self.claim_mutation_barrier.lock() = Some(barrier);
    }

    #[cfg(test)]
    pub(super) fn test_claim_mutation_checkpoint(&self, point: TestClaimMutationPoint) {
        let barrier = self.claim_mutation_barrier.lock().clone();
        let Some(barrier) = barrier.filter(|barrier| barrier.point == point) else {
            return;
        };
        std::fs::write(&barrier.started, b"started").expect("write claim mutation checkpoint");
        while !barrier.release.exists() {
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[cfg(test)]
    pub(super) fn set_test_cleanup_failure(&self) {
        self.force_cleanup_failure.store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(super) fn creation_claim_path(&self, id: &str) -> PathBuf {
        self.config
            .worktrees_root
            .join(CREATION_MARKER_DIR)
            .join(Self::creation_claim_name(id))
    }

    pub(super) fn creation_marker_path(&self, id: &str) -> PathBuf {
        self.config
            .worktrees_root
            .join(CREATION_MARKER_DIR)
            .join(format!("{id}.json"))
    }

    pub(super) fn creation_claim_name(id: &str) -> String {
        format!("{id}{CREATION_CLAIM_SUFFIX}")
    }
}
