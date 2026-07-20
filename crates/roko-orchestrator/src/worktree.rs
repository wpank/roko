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

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard, oneshot};
use tracing::debug;

/// Locks older than this are considered stale (§15.7).
///
/// Sourced from [`roko_core::defaults::DEFAULT_STALE_LOCK_SECS`].
const STALE_LOCK_SECS: u64 = roko_core::defaults::DEFAULT_STALE_LOCK_SECS;

/// Maximum time caller-runtime shutdown waits for the independent worker.
///
/// Git tree cleanup has a shorter internal bound. If cleanup cannot prove the
/// tree absent, the worker intentionally retains the operation reservation so
/// later mutations fail closed instead of overlapping an unowned process.
const RUNTIME_SHUTDOWN_WAIT: Duration = Duration::from_secs(5);

const CREATION_MARKER_DIR: &str = ".roko-creation";
// Journal fsyncs provide process/kernel-crash restart convergence on supported
// macOS/Linux targets. Ordinary macOS fsync is not claimed as a power-loss
// persistence boundary (F_FULLFSYNC would be required for that stronger claim).
const CREATION_MARKER_SCHEMA: u8 = 2;
const CREATION_CLAIM_SUFFIX: &str = ".claim";
const REPOSITORY_MUTATION_LOCK: &str = "roko-worktree-mutation.lock";

// Security boundary: atomic mkdir, flock, and fd-relative I/O protect against
// conforming concurrent Roko processes and pathname replacement races. No
// discretionary user-owned filesystem can defend against arbitrary hostile
// code running as the same effective UID; such code can delete any lock or
// claim inode. Root execution, UID mismatch, insecure modes, and foreign entry
// types therefore fail closed instead of being treated as recoverable.
// The permanent flock inode is anchored in the canonical Git common directory,
// so configurable checkout-output roots and linked-repository roots cannot
// partition repository ownership.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CreationPhase {
    Prepared,
    LinkedNoCheckout,
    ResetComplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CreationMarker {
    schema_version: u8,
    claim_id: String,
    id: String,
    repo_root: PathBuf,
    common_git_dir: PathBuf,
    branch: String,
    branch_old_oid: Option<String>,
    target_oid: String,
    path: PathBuf,
    admin_dir: PathBuf,
    phase: CreationPhase,
    previous_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CreationCleanupSafe {
    schema_version: u8,
    claim_id: String,
    id: String,
    repo_root: PathBuf,
    common_git_dir: PathBuf,
    branch: String,
    branch_old_oid: Option<String>,
    target_oid: String,
    path: PathBuf,
    admin_dir: PathBuf,
    reset_complete_digest: String,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InodeIdentity {
    device: u64,
    inode: u64,
}

/// Open directory handles bind every journal operation to the originally
/// acquired claim even if a public pathname is concurrently replaced.
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Debug)]
struct CreationClaim {
    marker: CreationMarker,
    worktrees_root_fd: std::os::fd::OwnedFd,
    marker_root_fd: std::os::fd::OwnedFd,
    claim_dir_fd: std::os::fd::OwnedFd,
    marker_root_inode: InodeIdentity,
    claim_dir_inode: InodeIdentity,
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[derive(Debug)]
struct CreationClaim {
    marker: CreationMarker,
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
enum TestClaimMutationPoint {
    BeforeBranchCas,
    BeforeTransitionWrite,
    BeforeRemovalCleanup,
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct TestClaimMutationBarrier {
    point: TestClaimMutationPoint,
    started: PathBuf,
    release: PathBuf,
}

#[derive(Debug, Default)]
struct OperationLifecycle {
    cancel_requested: AtomicBool,
    cleanup_unproved: AtomicBool,
    complete: Mutex<bool>,
    complete_cv: Condvar,
}

impl OperationLifecycle {
    fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::Release);
    }

    fn is_cancel_requested(&self) -> bool {
        self.cancel_requested.load(Ordering::Acquire)
    }

    fn mark_cleanup_unproved(&self) {
        self.cleanup_unproved.store(true, Ordering::Release);
    }

    fn cleanup_was_unproved(&self) -> bool {
        self.cleanup_unproved.load(Ordering::Acquire)
    }

    fn mark_complete(&self) {
        *self.complete.lock() = true;
        self.complete_cv.notify_all();
    }

    fn wait_for_complete(&self, timeout: Duration) -> bool {
        let mut complete = self.complete.lock();
        let started = std::time::Instant::now();
        while !*complete {
            let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
                break;
            };
            if self
                .complete_cv
                .wait_for(&mut complete, remaining)
                .timed_out()
            {
                break;
            }
        }
        *complete
    }
}

struct RuntimeShutdownOwner {
    lifecycle: Arc<OperationLifecycle>,
    armed: bool,
}

impl RuntimeShutdownOwner {
    fn new(lifecycle: Arc<OperationLifecycle>) -> Self {
        Self {
            lifecycle,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RuntimeShutdownOwner {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.lifecycle.request_cancel();
        if !self.lifecycle.wait_for_complete(RUNTIME_SHUTDOWN_WAIT) {
            tracing::error!(
                wait_ms = RUNTIME_SHUTDOWN_WAIT.as_millis(),
                "worktree mutation worker outlived caller-runtime shutdown bound; ownership remains fail-closed"
            );
        }
    }
}

/// Configuration handed to [`WorktreeManager::new`].
#[derive(Debug, Clone)]
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
    /// Accepted full commit ID.
    pub commit_oid: String,
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

/// Collision-resistant manager ID for an exact task attempt.
pub fn format_attempt_worktree_id(plan_id: &str, task_id: &str, attempt: u32) -> String {
    let digest = blake3::hash(format!("{plan_id}\0{task_id}\0{attempt}").as_bytes());
    format!("attempt-{}", &digest.to_hex()[..20])
}
/// Branch owned by an exact task attempt.
pub fn format_attempt_branch_name(plan_id: &str, task_id: &str, attempt: u32) -> String {
    let id = format_attempt_worktree_id(plan_id, task_id, attempt);
    format!("roko/attempt/{id}")
}
/// Manages the lifecycle of per-plan git worktrees.
///
/// Clones of [`WorktreeManager`] share the same internal registry — the
/// handle is cheap to clone and safe to move across tasks.
#[derive(Clone, Debug)]
pub struct WorktreeManager {
    config: Arc<WorktreeConfig>,
    active: Arc<Mutex<HashMap<String, WorktreeHandle>>>,
    accepted: Arc<Mutex<HashMap<String, AcceptedWorktree>>>,
    /// Shared fair reservation transferred into cancellation-independent tasks.
    operations: Arc<AsyncMutex<()>>,
    /// Canonical executable selected once and shared by probes and mutations.
    resolved_git_executable: Arc<Mutex<Option<PathBuf>>>,
    #[cfg(test)]
    git_binary: Arc<Mutex<PathBuf>>,
    #[cfg(test)]
    git_probe_environment: Arc<Mutex<Vec<(OsString, OsString)>>>,
    #[cfg(test)]
    phase_barrier: Arc<Mutex<Option<TestPhaseBarrier>>>,
    #[cfg(test)]
    claim_mutation_barrier: Arc<Mutex<Option<TestClaimMutationBarrier>>>,
    #[cfg(test)]
    force_cleanup_failure: Arc<AtomicBool>,
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
            operations: Arc::new(AsyncMutex::new(())),
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
            operations: Arc::new(AsyncMutex::new(())),
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

    async fn create_locked(
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
            if worktree_list_contains_branch(&listed.stdout, &branch_ref) {
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

        Ok(handle)
    }

    async fn create_git_phases(
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
        sync_directory(&admin_dir)?;
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
    pub async fn create_for_attempt(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
    ) -> Result<WorktreeHandle, WorktreeError> {
        let id = format_attempt_worktree_id(plan_id, task_id, attempt);
        let branch = format_attempt_branch_name(plan_id, task_id, attempt);
        let base = self.accepted.lock().get(plan_id).map_or_else(
            || self.config.base_branch.clone(),
            |accepted| accepted.commit_oid.clone(),
        );
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager.create_locked(&id, &branch, &base, &lifecycle).await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }
    /// Return the checkout owned by an exact task attempt.
    pub fn get_attempt(&self, plan: &str, task: &str, attempt: u32) -> Option<WorktreeHandle> {
        self.get(&format_attempt_worktree_id(plan, task, attempt))
    }
    /// Advance a plan's accepted immutable tip to an exact committed attempt.
    pub async fn accept_attempt(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
    ) -> Result<AcceptedWorktree, WorktreeError> {
        let id = format_attempt_worktree_id(plan_id, task_id, attempt);
        let handle = self
            .get(&id)
            .ok_or_else(|| WorktreeError::NotFound(id.clone()))?;
        let commit_oid = self
            .git_probe_stdout_at(&handle.path, &["rev-parse", "--verify", "HEAD^{commit}"])
            .await?;
        let accepted = AcceptedWorktree { handle, commit_oid };
        let _ = self
            .accepted
            .lock()
            .insert(plan_id.into(), accepted.clone());
        Ok(accepted)
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
                debug!(%error, "owned worktree discovery task failed");
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
                    debug!(plan_id, error = %error, "skipping unsafe worktree reattachment");
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

    async fn remove_locked(
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
        let status = self.git_probe_stdout_at(&handle.path, &args).await?;
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
            idle_ttl_ms: duration_millis_u64(self.config.idle_ttl),
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

    /// Probe the health of the worktree tracked under `id` (§15.5).
    ///
    /// Returns [`WorktreeHealth::Ok`] when the directory exists and the
    /// expected branch is checked out; other variants describe the
    /// specific failure mode.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::NotFound`] if `id` is not tracked,
    /// [`WorktreeError::GitFailed`] if the `git rev-parse` probe exits
    /// unsuccessfully, or [`WorktreeError::IoError`] if the git process
    /// cannot be spawned.
    pub async fn check_health(&self, id: &str) -> Result<WorktreeHealth, WorktreeError> {
        let handle = {
            let guard = self.active.lock();
            guard
                .get(id)
                .cloned()
                .ok_or_else(|| WorktreeError::NotFound(id.to_string()))?
        };

        if !handle.path.exists() {
            return Ok(WorktreeHealth::Missing);
        }

        // Check for a stale index.lock in the worktree's gitdir.
        if let Some(gitdir) = read_gitdir(&handle.path) {
            let lock = gitdir.join("index.lock");
            if lock.exists() && is_stale_lock(&lock) {
                return Ok(WorktreeHealth::StaleLock);
            }
        }

        // Verify the expected branch is checked out using the same executable
        // and environment boundary as every other manager Git invocation.
        let output = self
            .git_probe_output_at(&handle.path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        let current = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if current != handle.branch {
            return Ok(WorktreeHealth::Detached);
        }

        Ok(WorktreeHealth::Ok)
    }

    /// Return full isolation metadata for one tracked worktree.
    ///
    /// This performs the same real git/FS health checks as
    /// [`check_health`](Self::check_health) and adds idle/reclaimability
    /// metadata used by resume/recovery flows.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`check_health`](Self::check_health).
    pub async fn isolation_status(
        &self,
        id: &str,
    ) -> Result<WorktreeIsolationStatus, WorktreeError> {
        let health = self.check_health(id).await?;
        let handle = self
            .get(id)
            .ok_or_else(|| WorktreeError::NotFound(id.to_string()))?;
        Ok(self.status_from_handle(handle, health))
    }

    /// Return full isolation metadata for every tracked worktree.
    ///
    /// Statuses are sorted by worktree id. If a git probe fails for one
    /// worktree, the error is returned so callers can fail closed.
    ///
    /// # Errors
    ///
    /// Returns any [`WorktreeError`] produced while checking individual
    /// worktree health.
    pub async fn isolation_statuses(&self) -> Result<Vec<WorktreeIsolationStatus>, WorktreeError> {
        let ids: Vec<String> = {
            let mut ids: Vec<_> = self.active.lock().keys().cloned().collect();
            ids.sort();
            ids
        };

        let mut statuses = Vec::with_capacity(ids.len());
        for id in ids {
            statuses.push(self.isolation_status(&id).await?);
        }
        Ok(statuses)
    }

    /// Evict worktrees whose `last_active_ms` is older than
    /// [`WorktreeConfig::idle_ttl`], oldest first. Returns the ids
    /// that were successfully reclaimed (§15.6).
    ///
    /// # Errors
    ///
    /// This function is currently infallible and returns `Ok(...)` after
    /// best-effort reclamation; individual removal failures are skipped so
    /// one bad worktree does not block the rest.
    pub async fn reclaim_idle(&self) -> Result<Vec<String>, WorktreeError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let ttl_ms = i64::try_from(self.config.idle_ttl.as_millis()).unwrap_or(i64::MAX);

        let stale_ids: Vec<String> = {
            let mut candidates: Vec<_> = self
                .active
                .lock()
                .values()
                .filter(|h| (now_ms - h.last_active_ms) > ttl_ms)
                .map(|h| (h.id.clone(), h.last_active_ms))
                .collect();
            // Evict oldest first.
            candidates.sort_by_key(|(_, ts)| *ts);
            candidates.into_iter().map(|(id, _)| id).collect()
        };

        let mut removed = Vec::new();
        for id in stale_ids {
            if self.remove(&id).await.is_ok() {
                removed.push(id);
            }
        }

        Ok(removed)
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

    /// Remove stale `.git/index.lock` files (older than 60 seconds)
    /// across the main repo and all git-tracked worktrees (§15.7).
    ///
    /// # Errors
    ///
    /// This function is currently infallible and returns `Ok(...)` after
    /// best-effort cleanup; unreadable or non-stale lock files are simply
    /// skipped.
    pub fn clear_stale_locks(&self) -> Result<Vec<PathBuf>, WorktreeError> {
        let _repository_lock = self.acquire_repository_mutation_lock()?;
        self.clear_stale_locks_unlocked()
    }

    fn clear_stale_locks_unlocked(&self) -> Result<Vec<PathBuf>, WorktreeError> {
        let mut cleared = Vec::new();

        // Main repo lock.
        let main_lock = self.config.repo_root.join(".git").join("index.lock");
        if main_lock.exists()
            && is_stale_lock(&main_lock)
            && std::fs::remove_file(&main_lock).is_ok()
        {
            cleared.push(main_lock);
        }

        // Per-worktree locks stored under .git/worktrees/<name>/index.lock.
        let wt_meta_dir = self.config.repo_root.join(".git").join("worktrees");
        if wt_meta_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&wt_meta_dir) {
                for entry in entries.flatten() {
                    let lock = entry.path().join("index.lock");
                    if lock.exists() && is_stale_lock(&lock) && std::fs::remove_file(&lock).is_ok()
                    {
                        cleared.push(lock);
                    }
                }
            }
        }

        Ok(cleared)
    }

    /// Run `git worktree prune` to clean up stale git worktree metadata
    /// that no longer corresponds to on-disk directories (§15.9).
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::GitFailed`] if `git worktree prune`
    /// exits unsuccessfully or [`WorktreeError::IoError`] if the `git`
    /// process cannot be spawned.
    pub async fn prune(&self) -> Result<String, WorktreeError> {
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager.prune_locked(&lifecycle).await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    async fn prune_locked(&self, lifecycle: &OperationLifecycle) -> Result<String, WorktreeError> {
        let _ = self.clear_stale_locks_unlocked();
        self.validate_git_policy(false).await?;
        let output = self
            .git_mutation_output(&["worktree", "prune"], lifecycle)
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Try to re-register a worktree that exists on disk but is not
    /// tracked. Returns `Some(handle)` only when the candidate belongs to
    /// this manager's repository and has the exact canonical plan branch at
    /// its current tip. Mismatched or detached worktrees fail closed.
    async fn try_reattach_locked(
        &self,
        plan_id: &str,
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
        let admin_pointer = read_gitdir(&path).ok_or_else(|| {
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
        let expected_branch = format_branch_name(plan_id);
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

    async fn git_remove(
        &self,
        path: &Path,
        lifecycle: &OperationLifecycle,
    ) -> Result<(), WorktreeError> {
        let path_str = path.to_string_lossy().into_owned();
        let output = self
            .git_mutation_output(&["worktree", "remove", "--force", &path_str], lifecycle)
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
    }

    async fn rollback_incomplete_create(&self, claim: &CreationClaim) -> std::io::Result<()> {
        #[cfg(test)]
        if self.force_cleanup_failure.load(Ordering::Acquire) {
            return Err(std::io::Error::other("injected create rollback failure"));
        }

        self.verify_live_creation_claim(claim)?;
        self.verify_creation_marker(&claim.marker.id, &claim.marker)?;
        self.remove_registered_worktree(&claim.marker.path, &claim.marker.admin_dir)?;
        self.remove_creation_claim_if_exact(claim)
    }

    async fn common_git_dir(&self) -> Result<PathBuf, WorktreeError> {
        let output = self
            .git_policy_output(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
            .await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        std::fs::canonicalize(raw).map_err(WorktreeError::IoError)
    }

    async fn git_ref_oid(
        &self,
        reference: &str,
        required: bool,
    ) -> Result<Option<String>, WorktreeError> {
        let output = self
            .git_policy_output(&["rev-parse", "--verify", reference])
            .await?;
        if !output.status.success() {
            if required {
                return Err(WorktreeError::GitFailed {
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
            return Ok(None);
        }
        let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(WorktreeError::GitFailed {
                stderr: "git returned a malformed object id".to_string(),
            });
        }
        Ok(Some(oid))
    }

    fn register_linked_worktree(
        &self,
        path: &Path,
        admin_dir: &Path,
        expected_ref: &str,
    ) -> std::io::Result<()> {
        let admin_parent = admin_dir
            .parent()
            .ok_or_else(|| std::io::Error::other("worktree admin directory has no parent"))?;
        std::fs::create_dir_all(admin_parent)?;
        std::fs::create_dir(admin_dir)?;
        write_new_synced_file(&admin_dir.join("locked"), b"roko create in progress\n")?;
        write_new_synced_file(&admin_dir.join("commondir"), b"../..\n")?;
        write_new_synced_file(
            &admin_dir.join("HEAD"),
            format!("ref: {expected_ref}\n").as_bytes(),
        )?;
        write_new_synced_file(
            &admin_dir.join("gitdir"),
            format!("{}\n", path.join(".git").display()).as_bytes(),
        )?;
        sync_directory(admin_dir)?;
        sync_directory(admin_parent)?;

        std::fs::create_dir(path)?;
        write_new_synced_file(
            &path.join(".git"),
            format!("gitdir: {}\n", admin_dir.display()).as_bytes(),
        )?;
        sync_directory(path)?;
        if let Some(parent) = path.parent() {
            sync_directory(parent)?;
        }
        Ok(())
    }

    fn verify_creation_marker(&self, id: &str, marker: &CreationMarker) -> std::io::Result<()> {
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

    fn remove_registered_worktree(&self, path: &Path, admin_dir: &Path) -> std::io::Result<()> {
        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(std::io::Error::other(
                    "owned incomplete worktree path changed type before rollback",
                ));
            }
            let dot_git = std::fs::read_to_string(path.join(".git"))?;
            if dot_git.trim() != format!("gitdir: {}", admin_dir.display()) {
                return Err(std::io::Error::other(
                    "owned incomplete worktree .git link changed before rollback",
                ));
            }
            std::fs::remove_dir_all(path)?;
        }
        if let Ok(metadata) = std::fs::symlink_metadata(admin_dir) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(std::io::Error::other(
                    "owned worktree admin path changed type before rollback",
                ));
            }
            let gitdir = std::fs::read_to_string(admin_dir.join("gitdir"))?;
            if gitdir.trim() != path.join(".git").to_string_lossy() {
                return Err(std::io::Error::other(
                    "owned worktree admin link changed before rollback",
                ));
            }
            std::fs::remove_dir_all(admin_dir)?;
        }
        if path.exists() || admin_dir.exists() {
            return Err(std::io::Error::other(
                "incomplete worktree remains registered after rollback",
            ));
        }
        Ok(())
    }

    fn creation_marker_path(&self, id: &str) -> PathBuf {
        self.config
            .worktrees_root
            .join(CREATION_MARKER_DIR)
            .join(format!("{id}.json"))
    }

    fn creation_claim_name(id: &str) -> String {
        format!("{id}{CREATION_CLAIM_SUFFIX}")
    }

    #[cfg(any(test, not(any(target_os = "macos", target_os = "linux"))))]
    fn creation_claim_path(&self, id: &str) -> PathBuf {
        self.config
            .worktrees_root
            .join(CREATION_MARKER_DIR)
            .join(Self::creation_claim_name(id))
    }

    async fn reject_outstanding_creation_marker(&self, id: &str) -> Result<(), WorktreeError> {
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

    fn reject_legacy_creation_marker(&self, id: &str) -> Result<(), WorktreeError> {
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

    fn creation_marker_publication_error(&self, id: &str, error: std::io::Error) -> WorktreeError {
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
    fn publish_creation_marker(&self, marker: CreationMarker) -> std::io::Result<CreationClaim> {
        let _ = marker;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "durable creation claims require macOS or Linux fd-relative filesystem semantics",
        ))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn publish_creation_marker(&self, marker: CreationMarker) -> std::io::Result<CreationClaim> {
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

    fn transition_creation_marker(
        &self,
        claim: &mut CreationClaim,
        next_phase: CreationPhase,
    ) -> std::io::Result<()> {
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
        self.test_claim_mutation_checkpoint(TestClaimMutationPoint::BeforeTransitionWrite);
        write_creation_record(&claim.claim_dir_fd, &next)?;
        rustix::fs::fsync(&claim.claim_dir_fd).map_err(std::io::Error::from)?;
        self.verify_live_creation_claim(claim)?;
        claim.marker = next;
        validate_claim_entries(&claim.claim_dir_fd, &claim.marker, false)?;
        Ok(())
    }

    fn remove_completed_creation_marker(&self, claim: &CreationClaim) -> std::io::Result<()> {
        if claim.marker.phase != CreationPhase::ResetComplete {
            return Err(std::io::Error::other(
                "only a ResetComplete creation marker can be committed",
            ));
        }
        self.remove_creation_claim_if_exact(claim)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn remove_creation_claim_if_exact(&self, claim: &CreationClaim) -> std::io::Result<()> {
        let _ = claim;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "durable creation claims require macOS or Linux",
        ))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn remove_creation_claim_if_exact(&self, claim: &CreationClaim) -> std::io::Result<()> {
        self.verify_live_creation_claim(claim)?;
        let (existing, _) = read_creation_record(&claim.claim_dir_fd, &claim.marker)?;
        if existing != claim.marker {
            return Err(std::io::Error::other(
                "creation claim identity or phase changed before removal",
            ));
        }
        validate_claim_entries(&claim.claim_dir_fd, &claim.marker, false)?;
        #[cfg(test)]
        self.test_claim_mutation_checkpoint(TestClaimMutationPoint::BeforeRemovalCleanup);
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
    fn acquire_repository_mutation_lock(&self) -> Result<RepositoryMutationLock, WorktreeError> {
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
        rustix::fs::flock(&lock_fd, rustix::fs::FlockOperation::LockExclusive)
            .map_err(std::io::Error::from)?;
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
    fn acquire_repository_mutation_lock(&self) -> Result<RepositoryMutationLock, WorktreeError> {
        Err(WorktreeError::UnsafeGitExecution {
            reason: "durable worktree mutation locking requires macOS or Linux".to_string(),
        })
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn open_creation_marker_root(
        &self,
        create: bool,
    ) -> std::io::Result<(std::os::fd::OwnedFd, std::os::fd::OwnedFd, InodeIdentity)> {
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
    fn verify_live_creation_claim(&self, claim: &CreationClaim) -> std::io::Result<()> {
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

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn recover_or_reject_creation_claim(&self, id: &str) -> Result<(), WorktreeError> {
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
                // Only a truly empty mkdir-publication/cleanup window is
                // recoverable without identity. A non-empty directory is a
                // foreign or corrupt claim and remains untouched.
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

    async fn validate_git_policy(&self, checkout: bool) -> Result<(), WorktreeError> {
        roko_agent::process::validate_no_descendant_context().map_err(|error| {
            WorktreeError::UnsafeGitExecution {
                reason: error.to_string(),
            }
        })?;
        validate_trusted_executable(&self.git_executable()?, &self.config)?;

        if !checkout {
            return Ok(());
        }

        let config = self
            .git_policy_output(&[
                "config",
                "--includes",
                "--get-regexp",
                "^(filter\\..*\\.(process|smudge)|core\\.fsmonitor|extensions\\.partialclone|remote\\..*\\.promisor)$",
            ])
            .await?;
        if config.status.success() {
            for line in String::from_utf8_lossy(&config.stdout).lines() {
                let mut fields = line.splitn(2, char::is_whitespace);
                let key = fields.next().unwrap_or_default().to_ascii_lowercase();
                let value = fields.next().unwrap_or_default().trim();
                let harmless_false = matches!(key.as_str(), "core.fsmonitor")
                    && matches!(value, "false" | "no" | "off" | "0" | "");
                let harmless_promisor = key.ends_with(".promisor")
                    && matches!(value, "false" | "no" | "off" | "0" | "");
                if !harmless_false && !harmless_promisor {
                    return Err(WorktreeError::UnsafeGitExecution {
                        reason: format!("unsupported checkout extension `{key}`"),
                    });
                }
            }
        } else if config.status.code() != Some(1) {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&config.stderr).into_owned(),
            });
        }

        let hook = self
            .git_policy_output(&["rev-parse", "--git-path", "hooks/post-checkout"])
            .await?;
        if !hook.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&hook.stderr).into_owned(),
            });
        }
        let hook_path = PathBuf::from(String::from_utf8_lossy(&hook.stdout).trim());
        if is_executable_file(&hook_path) {
            return Err(WorktreeError::UnsafeGitExecution {
                reason: format!("executable post-checkout hook `{}`", hook_path.display()),
            });
        }
        Ok(())
    }

    async fn git_policy_output(&self, args: &[&str]) -> Result<Output, WorktreeError> {
        self.git_probe_output_at(&self.config.repo_root, args).await
    }

    async fn git_probe_output_at(
        &self,
        current_dir: &Path,
        args: &[&str],
    ) -> Result<Output, WorktreeError> {
        let executable = self.git_executable()?;
        validate_trusted_executable(&executable, &self.config)?;
        let mut command = Command::new(executable);
        command
            .current_dir(current_dir)
            .args(args)
            .stdin(Stdio::null())
            .kill_on_drop(true);
        #[cfg(test)]
        for (key, value) in self.git_probe_environment.lock().iter() {
            command.env(key, value);
        }
        sanitize_git_environment(&mut command);
        roko_agent::process::configure_no_descendant_process(&mut command).map_err(|error| {
            WorktreeError::UnsafeGitExecution {
                reason: error.to_string(),
            }
        })?;
        command.output().await.map_err(WorktreeError::IoError)
    }

    async fn git_probe_stdout_at(
        &self,
        current_dir: &Path,
        args: &[&str],
    ) -> Result<String, WorktreeError> {
        let output = self.git_probe_output_at(current_dir, args).await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        let value = String::from_utf8(output.stdout).map_err(|_| {
            WorktreeError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "git probe returned non-UTF-8 output",
            ))
        })?;
        let value = value.trim().to_string();
        if value.is_empty() {
            Err(WorktreeError::GitFailed {
                stderr: "git probe returned empty output".to_string(),
            })
        } else {
            Ok(value)
        }
    }

    async fn git_probe_canonical_path_at(
        &self,
        current_dir: &Path,
        args: &[&str],
    ) -> Result<PathBuf, WorktreeError> {
        let value = self.git_probe_stdout_at(current_dir, args).await?;
        let path = PathBuf::from(value);
        let absolute = if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        };
        std::fs::canonicalize(absolute).map_err(WorktreeError::IoError)
    }

    async fn git_probe_common_dir_at(&self, current_dir: &Path) -> Result<PathBuf, WorktreeError> {
        self.git_probe_canonical_path_at(current_dir, &["rev-parse", "--git-common-dir"])
            .await
    }

    async fn git_mutation_output(
        &self,
        args: &[&str],
        lifecycle: &OperationLifecycle,
    ) -> std::io::Result<Output> {
        self.git_mutation_output_at(&self.config.repo_root, args, lifecycle, true)
            .await
    }

    async fn git_mutation_output_at(
        &self,
        current_dir: &Path,
        args: &[&str],
        lifecycle: &OperationLifecycle,
        observe_cancellation: bool,
    ) -> std::io::Result<Output> {
        let executable = self.git_executable()?;

        let mut stdout = CommandCapture::new("stdout")?;
        let mut stderr = CommandCapture::new("stderr")?;
        let mut command = Command::new(executable);
        command
            .current_dir(current_dir)
            .arg("--no-pager")
            .args([
                "-c",
                "index.threads=1",
                "-c",
                "checkout.workers=1",
                "-c",
                "core.preloadIndex=false",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "maintenance.auto=false",
                "-c",
                "gc.auto=0",
                "-c",
                "submodule.recurse=false",
            ])
            .args(args)
            .stdin(Stdio::null())
            .stdout(stdout.child_stdio()?)
            .stderr(stderr.child_stdio()?)
            .kill_on_drop(true);
        sanitize_git_environment(&mut command);
        roko_agent::process::configure_no_descendant_process(&mut command)?;
        let mut child = command.spawn()?;

        let status = loop {
            if observe_cancellation && lifecycle.is_cancel_requested() {
                let cleanup = terminate_direct_child(&mut child).await;
                if let Err(error) = cleanup {
                    lifecycle.mark_cleanup_unproved();
                    return Err(error);
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "git mutation cancelled because its caller runtime shut down",
                ));
            }

            let child_status = match child.try_wait() {
                Ok(status) => status,
                Err(wait_error) => {
                    if let Err(cleanup_error) = terminate_direct_child(&mut child).await {
                        lifecycle.mark_cleanup_unproved();
                        return Err(std::io::Error::new(
                            cleanup_error.kind(),
                            format!(
                                "Git wait failed ({wait_error}); direct-child cleanup also failed: {cleanup_error}"
                            ),
                        ));
                    }
                    return Err(wait_error);
                }
            };
            if let Some(status) = child_status {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };

        Ok(Output {
            status,
            stdout: stdout.read_all()?,
            stderr: stderr.read_all()?,
        })
    }

    fn git_executable(&self) -> std::io::Result<PathBuf> {
        if let Some(executable) = self.resolved_git_executable.lock().clone() {
            return Ok(executable);
        }
        #[cfg(test)]
        let requested = self.git_binary.lock().clone();
        #[cfg(not(test))]
        let requested = PathBuf::from("git");
        let executable = resolve_executable(&requested)?;
        *self.resolved_git_executable.lock() = Some(executable.clone());
        Ok(executable)
    }

    #[cfg(test)]
    async fn test_phase_checkpoint(
        &self,
        phase: CreationPhase,
        lifecycle: &OperationLifecycle,
    ) -> std::io::Result<()> {
        let barrier = self.phase_barrier.lock().clone();
        let Some(barrier) = barrier.filter(|barrier| barrier.phase == phase) else {
            return Ok(());
        };
        std::fs::write(&barrier.started, b"started")?;
        while !barrier.release.exists() {
            if lifecycle.is_cancel_requested() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "create phase cancelled by caller-runtime shutdown",
                ));
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok(())
    }

    #[cfg(not(test))]
    async fn test_phase_checkpoint(
        &self,
        _phase: CreationPhase,
        _lifecycle: &OperationLifecycle,
    ) -> std::io::Result<()> {
        Ok(())
    }

    #[cfg(test)]
    fn set_test_git_binary(&self, executable: PathBuf) {
        *self.git_binary.lock() = executable;
        *self.resolved_git_executable.lock() = None;
    }

    #[cfg(test)]
    fn set_test_git_probe_environment(&self, environment: Vec<(OsString, OsString)>) {
        *self.git_probe_environment.lock() = environment;
    }

    #[cfg(test)]
    fn set_test_phase_barrier(&self, barrier: TestPhaseBarrier) {
        *self.phase_barrier.lock() = Some(barrier);
    }

    #[cfg(test)]
    fn set_test_claim_mutation_barrier(&self, barrier: TestClaimMutationBarrier) {
        *self.claim_mutation_barrier.lock() = Some(barrier);
    }

    #[cfg(test)]
    fn test_claim_mutation_checkpoint(&self, point: TestClaimMutationPoint) {
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
    fn set_test_cleanup_failure(&self) {
        self.force_cleanup_failure.store(true, Ordering::Release);
    }

    fn status_from_handle(
        &self,
        handle: WorktreeHandle,
        health: WorktreeHealth,
    ) -> WorktreeIsolationStatus {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let idle_ms = u64::try_from(now_ms.saturating_sub(handle.last_active_ms)).unwrap_or(0);
        let ttl_ms = duration_millis_u64(self.config.idle_ttl);
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
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn inode_identity(stat: &rustix::fs::Stat) -> InodeIdentity {
    InodeIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct RepositoryMutationLock {
    repo_root_fd: std::os::fd::OwnedFd,
    git_entry_fd: std::os::fd::OwnedFd,
    common_dir_fd: std::os::fd::OwnedFd,
    lock_fd: std::os::fd::OwnedFd,
    repo_root_inode: InodeIdentity,
    git_entry_inode: InodeIdentity,
    git_entry_is_directory: bool,
    common_dir_inode: InodeIdentity,
    lock_inode: InodeIdentity,
    repo_root: PathBuf,
    canonical_common_dir: PathBuf,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl RepositoryMutationLock {
    fn verify_binding(&self) -> std::io::Result<()> {
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

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
struct RepositoryMutationLock;

fn retain_lock_if_cleanup_unproved(
    repository_lock: RepositoryMutationLock,
    lifecycle: &OperationLifecycle,
) {
    if lifecycle.cleanup_was_unproved() {
        // A kernel-released flock is the only cross-process ownership proof.
        // If cleanup cannot be proved, deliberately retain it with the local
        // operation reservation so another process cannot overlap mutation.
        std::mem::forget(repository_lock);
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn open_secure_directory_at(
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
) -> std::io::Result<InodeIdentity> {
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
fn validate_secure_directory(
    fd: &std::os::fd::OwnedFd,
    label: &str,
) -> std::io::Result<InodeIdentity> {
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
struct ResolvedRepositoryIdentity {
    git_entry_fd: std::os::fd::OwnedFd,
    git_entry_inode: InodeIdentity,
    git_entry_is_directory: bool,
    canonical_common_dir: PathBuf,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn resolve_repository_identity(
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
) -> std::io::Result<InodeIdentity> {
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

fn creation_record_name(claim_id: &str, phase: CreationPhase) -> String {
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
fn ensure_cleanup_safe(
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
fn unlink_claim_file(claim_dir_fd: &std::os::fd::OwnedFd, name: &str) -> std::io::Result<()> {
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

struct CommandCapture {
    path: PathBuf,
    file: std::fs::File,
}

impl CommandCapture {
    fn new(stream: &str) -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "roko-git-{}-{stream}.capture",
            uuid::Uuid::new_v4()
        ));
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path)?;
        Ok(Self { path, file })
    }

    fn child_stdio(&self) -> std::io::Result<Stdio> {
        self.file.try_clone().map(Stdio::from)
    }

    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        self.file.seek(std::io::SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        self.file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

impl Drop for CommandCapture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

async fn terminate_direct_child(child: &mut tokio::process::Child) -> std::io::Result<()> {
    if child.try_wait()?.is_none() {
        child.start_kill()?;
        let _ = child.wait().await?;
    }
    Ok(())
}

fn ensure_git_success(output: Output) -> Result<(), WorktreeError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(WorktreeError::GitFailed {
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn resolve_executable(requested: &Path) -> std::io::Result<PathBuf> {
    if requested.components().count() > 1 {
        return std::fs::canonicalize(requested);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(requested);
        if candidate.is_file() {
            return std::fs::canonicalize(candidate);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Git executable `{}` not found on PATH", requested.display()),
    ))
}

fn validate_trusted_executable(
    executable: &Path,
    config: &WorktreeConfig,
) -> Result<(), WorktreeError> {
    let metadata = std::fs::symlink_metadata(executable)?;
    if !metadata.file_type().is_file() {
        return Err(WorktreeError::UnsafeGitExecution {
            reason: format!(
                "Git executable `{}` is not a regular file",
                executable.display()
            ),
        });
    }
    if executable.starts_with(&config.repo_root) || executable.starts_with(&config.worktrees_root) {
        return Err(WorktreeError::UnsafeGitExecution {
            reason: "Git executable is inside the managed repository/worktree root".to_string(),
        });
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o6000 != 0 {
            return Err(WorktreeError::UnsafeGitExecution {
                reason: "setuid/setgid Git executables are unsupported".to_string(),
            });
        }
    }
    Ok(())
}

fn sanitize_git_environment(command: &mut Command) {
    let mut git_keys: Vec<OsString> = std::env::vars_os()
        .filter_map(|(key, _)| key.to_string_lossy().starts_with("GIT_").then_some(key))
        .collect();
    git_keys.extend(
        command
            .as_std()
            .get_envs()
            .filter(|(key, _)| key.to_string_lossy().starts_with("GIT_"))
            .map(|(key, _)| key.to_os_string()),
    );
    git_keys.sort();
    git_keys.dedup();
    for key in git_keys {
        command.env_remove(key);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat");
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = metadata;
        false
    }
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}

fn write_new_synced_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

fn canonicalish(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    let Some(parent) = path.parent() else {
        return path.to_path_buf();
    };
    match std::fs::canonicalize(parent) {
        Ok(parent) => path
            .file_name()
            .map_or(parent.clone(), |name| parent.join(name)),
        Err(_) => path.to_path_buf(),
    }
}

fn worktree_list_contains_path(stdout: &[u8], path: &Path) -> bool {
    let expected = canonicalish(path);
    String::from_utf8_lossy(stdout).lines().any(|line| {
        line.strip_prefix("worktree ")
            .is_some_and(|listed| canonicalish(Path::new(listed)) == expected)
    })
}

fn worktree_list_contains_branch(stdout: &[u8], branch_ref: &str) -> bool {
    String::from_utf8_lossy(stdout)
        .lines()
        .any(|line| line.strip_prefix("branch ") == Some(branch_ref))
}

fn reattach_rejected(id: &str, reason: impl Into<String>) -> WorktreeError {
    WorktreeError::ReattachRejected {
        id: id.to_string(),
        reason: reason.into(),
    }
}

async fn await_owned_operation<T, F, Fut>(
    operation: OwnedMutexGuard<()>,
    operation_fn: F,
) -> Result<T, WorktreeError>
where
    T: Send + 'static,
    F: FnOnce(Arc<OperationLifecycle>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, WorktreeError>> + Send + 'static,
{
    let lifecycle = Arc::new(OperationLifecycle::default());
    let worker_lifecycle = Arc::clone(&lifecycle);
    let (result_tx, result_rx) = oneshot::channel();
    let (done_tx, done_rx) = oneshot::channel();

    std::thread::Builder::new()
        .name("roko-worktree-mutation".to_string())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(WorktreeError::IoError)?
                    .block_on(operation_fn(Arc::clone(&worker_lifecycle)))
            }))
            .unwrap_or_else(|_| {
                worker_lifecycle.mark_cleanup_unproved();
                Err(WorktreeError::IoError(std::io::Error::other(
                    "runtime-independent worktree mutation worker panicked",
                )))
            });

            if worker_lifecycle.cleanup_was_unproved() {
                // The process tree could not be proved absent. Permanently
                // retain manager ownership rather than permitting a later Git
                // mutation to overlap an unowned descendant.
                std::mem::forget(operation);
            } else {
                drop(operation);
            }
            worker_lifecycle.mark_complete();
            let _ = done_tx.send(());
            let _ = result_tx.send(result);
        })
        .map_err(WorktreeError::IoError)?;

    let sentinel_lifecycle = Arc::clone(&lifecycle);
    tokio::spawn(async move {
        let mut shutdown_owner = RuntimeShutdownOwner::new(sentinel_lifecycle);
        let _ = done_rx.await;
        shutdown_owner.disarm();
    });

    result_rx.await.map_err(|_| {
        WorktreeError::IoError(std::io::Error::other(
            "runtime-independent worktree mutation worker ended without a result",
        ))
    })?
}

fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn validate_id(id: &str) -> Result<(), WorktreeError> {
    if id.is_empty() {
        return Err(WorktreeError::InvalidId("id is empty".to_string()));
    }
    if id.starts_with('.') || id.starts_with('-') {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` may not start with `.` or `-`"
        )));
    }
    if id.contains("..") {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` may not contain `..`"
        )));
    }
    // Characters that would break either a filesystem path or a git ref:
    // `/`, `\`, NUL, whitespace, and the git-ref-forbidden set
    // (`~`, `^`, `:`, `?`, `*`, `[`, ASCII control). Also reject `@{`.
    for ch in id.chars() {
        let bad = matches!(
            ch,
            '/' | '\\' | '\0' | '~' | '^' | ':' | '?' | '*' | '[' | '@'
        ) || ch.is_whitespace()
            || ch.is_control();
        if bad {
            return Err(WorktreeError::InvalidId(format!(
                "id `{id}` contains forbidden character `{ch}`"
            )));
        }
    }
    Ok(())
}

/// Read the `gitdir:` pointer from a worktree's `.git` file.
///
/// In a worktree `.git` is a regular file (not a directory) containing
/// a single line like `gitdir: /repo/.git/worktrees/<name>`. Returns
/// `None` if the file is missing, is a directory, or is unparseable.
fn read_gitdir(worktree_path: &Path) -> Option<PathBuf> {
    let git_file = worktree_path.join(".git");
    if git_file.is_dir() {
        // Main repo — the gitdir is the .git directory itself.
        return Some(git_file);
    }
    let content = std::fs::read_to_string(&git_file).ok()?;
    let path_str = content.trim().strip_prefix("gitdir: ")?;
    let p = PathBuf::from(path_str);
    if p.is_absolute() {
        Some(p)
    } else {
        Some(worktree_path.join(p))
    }
}

/// A lock file is considered stale when its mtime is ≥ [`STALE_LOCK_SECS`]
/// seconds in the past.
fn is_stale_lock(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    std::time::SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age.as_secs() >= STALE_LOCK_SECS)
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
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "stable no-follow workspace traversal is unavailable on this platform",
    ))
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn validate_workspace_file_kinds_with(
    workdir: &Path,
    ignored: &[PathBuf],
    max_dirs: usize,
    mut opened: impl FnMut(&Path),
) -> std::io::Result<()> {
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
                        FileType::Directory => return Err(open_error.into()),
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
#[cfg(test)]
mod tests {
    //! Tests below each spin up a throwaway git repo in a
    //! [`tempfile::TempDir`]. Every test that needs `git` first checks
    //! the binary via [`git_available`]; if git isn't on `$PATH`, the
    //! test returns early so `cargo test` still succeeds on machines
    //! without git (for instance minimal CI images).

    #![allow(clippy::unwrap_used)]

    use super::{
        CreationMarker, CreationPhase, TestClaimMutationBarrier, TestClaimMutationPoint,
        TestPhaseBarrier, WorktreeConfig, WorktreeError, WorktreeHealth, WorktreeManager,
        format_attempt_branch_name, format_attempt_worktree_id, format_branch_name, validate_id,
        validate_workspace_file_kinds_with,
    };
    use std::path::{Path, PathBuf};
    use std::process::Command as StdCommand;
    use std::time::Duration;
    use tempfile::TempDir;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn workspace_walk_never_traverses_directory_replaced_by_external_symlink() {
        use std::os::unix::fs::symlink;

        let workspace = TempDir::new().unwrap();
        let external = TempDir::new().unwrap();
        std::fs::create_dir(workspace.path().join("queued")).unwrap();
        assert!(
            StdCommand::new("mkfifo")
                .arg(external.path().join("outside.fifo"))
                .status()
                .unwrap()
                .success()
        );
        let mut replaced = false;
        let result = validate_workspace_file_kinds_with(workspace.path(), &[], 8, |relative| {
            if relative == Path::new("queued") && !replaced {
                replaced = true;
                std::fs::rename(
                    workspace.path().join("queued"),
                    workspace.path().join("original"),
                )
                .unwrap();
                symlink(external.path(), workspace.path().join("queued")).unwrap();
            }
        });
        assert!(replaced);
        assert!(
            !result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("non-file input")),
            "walker followed replacement symlink to external FIFO: {result:?}"
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn workspace_walk_rejects_deleted_and_recreated_directory() {
        let workspace = TempDir::new().unwrap();
        std::fs::create_dir(workspace.path().join("queued")).unwrap();
        let mut replaced = false;
        let error = validate_workspace_file_kinds_with(workspace.path(), &[], 8, |relative| {
            if relative == Path::new("queued") && !replaced {
                replaced = true;
                std::fs::remove_dir(workspace.path().join("queued")).unwrap();
                std::fs::create_dir(workspace.path().join("queued")).unwrap();
            }
        })
        .unwrap_err();
        assert!(replaced);
        assert!(error.to_string().contains("directory changed"), "{error}");
    }

    fn git_available() -> bool {
        StdCommand::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn init_repo(dir: &Path) {
        // Keep the default branch name deterministic across git
        // versions and user configs.
        let status = StdCommand::new("git")
            .current_dir(dir)
            .args(["init", "-b", "main"])
            .status()
            .unwrap();
        assert!(status.success(), "git init failed");

        // Scoped identity so the test's commit is reproducible without
        // touching the user's global git config.
        for (k, v) in [
            ("user.email", "roko@example.test"),
            ("user.name", "Roko Test"),
            ("commit.gpgsign", "false"),
        ] {
            let ok = StdCommand::new("git")
                .current_dir(dir)
                .args(["config", k, v])
                .status()
                .unwrap()
                .success();
            assert!(ok, "git config {k} failed");
        }

        let ok = StdCommand::new("git")
            .current_dir(dir)
            .args(["commit", "--allow-empty", "-m", "init"])
            .status()
            .unwrap()
            .success();
        assert!(ok, "git commit --allow-empty failed");
    }

    fn make_manager() -> Option<(TempDir, WorktreeManager)> {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return None;
        }
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        init_repo(&repo_root);
        let worktrees_root = tmp.path().join("worktrees");
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root,
            base_branch: "main".to_string(),
            worktrees_root,
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        Some((tmp, mgr))
    }

    fn make_manager_with_budget(max_live: usize) -> Option<(TempDir, WorktreeManager)> {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return None;
        }
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        init_repo(&repo_root);
        let worktrees_root = tmp.path().join("worktrees");
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root,
            base_branch: "main".to_string(),
            worktrees_root,
            max_live: Some(max_live),
            idle_ttl: Duration::from_secs(3600),
        });
        Some((tmp, mgr))
    }

    fn manager_with_worktrees_root(
        manager: &WorktreeManager,
        worktrees_root: PathBuf,
    ) -> WorktreeManager {
        WorktreeManager::new(WorktreeConfig {
            repo_root: manager.config.repo_root.clone(),
            base_branch: manager.config.base_branch.clone(),
            worktrees_root,
            max_live: manager.config.max_live,
            idle_ttl: manager.config.idle_ttl,
        })
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn repository_lock_path(manager: &WorktreeManager) -> PathBuf {
        let common = String::from_utf8(
            StdCommand::new("git")
                .current_dir(&manager.config.repo_root)
                .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        std::fs::canonicalize(common.trim())
            .unwrap()
            .join(super::REPOSITORY_MUTATION_LOCK)
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn test_creation_marker(manager: &WorktreeManager, id: &str) -> CreationMarker {
        let target_oid = String::from_utf8(
            StdCommand::new("git")
                .current_dir(&manager.config.repo_root)
                .args(["rev-parse", "main"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        let common_git_dir = std::fs::canonicalize(manager.config.repo_root.join(".git")).unwrap();
        CreationMarker {
            schema_version: super::CREATION_MARKER_SCHEMA,
            claim_id: uuid::Uuid::new_v4().simple().to_string(),
            id: id.to_string(),
            repo_root: manager.config.repo_root.clone(),
            common_git_dir: common_git_dir.clone(),
            branch: format!("feature/{id}"),
            branch_old_oid: None,
            target_oid,
            path: manager.path_for(id),
            admin_dir: common_git_dir.join(format!("worktrees/roko-{id}")),
            phase: CreationPhase::Prepared,
            previous_digest: None,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    struct GitProcessBarrier {
        started: PathBuf,
        release: PathBuf,
        invocations: PathBuf,
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn install_git_process_barrier(
        manager: &WorktreeManager,
        tempdir: &TempDir,
    ) -> GitProcessBarrier {
        use std::os::unix::fs::PermissionsExt;

        let script = tempdir.path().join("blocking-git.sh");
        let started = tempdir.path().join("git-started");
        let release = tempdir.path().join("git-release");
        let consumed = tempdir.path().join("git-barrier-consumed");
        let invocations = tempdir.path().join("git-invocations");
        let body = format!(
            "#!/bin/sh\n\
             set -eu\n\
             printf '%s\\n' \"$$\" >> '{}'\n\
             if [ ! -e '{}' ]; then\n\
               : > '{}'\n\
               : > '{}'\n\
               while [ ! -e '{}' ]; do :; done\n\
             fi\n\
             exec git \"$@\"\n",
            invocations.display(),
            consumed.display(),
            consumed.display(),
            started.display(),
            release.display(),
        );
        std::fs::write(&script, body).expect("write blocking git wrapper");
        let mut permissions = std::fs::metadata(&script)
            .expect("wrapper metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&script, permissions).expect("make wrapper executable");
        manager.set_test_git_binary(script);
        GitProcessBarrier {
            started,
            release,
            invocations,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn wait_for_barrier(path: &Path) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !path.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("git process reached barrier");
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn invocation_pids(path: &Path) -> Vec<u32> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(|line| line.parse().expect("wrapper pid"))
            .collect()
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn assert_processes_exited(pids: &[u32]) {
        for pid in pids {
            let status = StdCommand::new("kill")
                .args(["-0", &pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("probe wrapper pid");
            assert!(!status.success(), "git wrapper process {pid} leaked");
        }
    }

    fn assert_no_git_locks(root: &Path) {
        fn visit(path: &Path, locks: &mut Vec<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(path) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, locks);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension == "lock")
                    && path.file_name().is_none_or(|name| {
                        name != std::ffi::OsStr::new(super::REPOSITORY_MUTATION_LOCK)
                    })
                {
                    locks.push(path);
                }
            }
        }

        let mut locks = Vec::new();
        visit(&root.join(".git"), &mut locks);
        assert!(locks.is_empty(), "git lock files leaked: {locks:?}");
    }

    // ── Existing tests (§15.1–§15.2, §15.8) ──

    #[tokio::test]
    async fn create_worktree_materialises_directory() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let handle = mgr.create("01-alpha", "feature/alpha").await.unwrap();
        assert_eq!(handle.id, "01-alpha");
        assert_eq!(handle.branch, "feature/alpha");
        assert!(handle.path.exists(), "worktree dir should exist");
        assert!(handle.path.join(".git").exists(), ".git file expected");
        assert!(handle.created_at_ms > 0);
    }

    #[tokio::test]
    async fn list_returns_every_active_handle_sorted() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("02-bravo", "feature/bravo").await.unwrap();
        mgr.create("01-alpha", "feature/alpha").await.unwrap();
        let listed = mgr.list().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "01-alpha");
        assert_eq!(listed[1].id, "02-bravo");
    }

    #[tokio::test]
    async fn remove_drops_handle_and_worktree() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let handle = mgr.create("03-charlie", "feature/charlie").await.unwrap();
        assert!(handle.path.exists());
        mgr.remove("03-charlie").await.unwrap();
        assert!(mgr.list().unwrap().is_empty());
        assert!(
            !handle.path.exists(),
            "git worktree remove --force should delete the dir"
        );
    }

    #[tokio::test]
    async fn create_remove_roundtrip_allows_reuse() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("04-delta", "feature/delta").await.unwrap();
        mgr.remove("04-delta").await.unwrap();
        // After removal we must be able to use the id again.
        let h2 = mgr.create("04-delta", "feature/delta").await.unwrap();
        assert_eq!(h2.id, "04-delta");
        assert_eq!(mgr.list().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn remove_nonexistent_is_not_found() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let err = mgr.remove("nope").await.unwrap_err();
        assert!(matches!(err, WorktreeError::NotFound(ref id) if id == "nope"));
    }

    #[tokio::test]
    async fn duplicate_id_is_rejected() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("05-echo", "feature/echo").await.unwrap();
        let err = mgr.create("05-echo", "feature/echo-2").await.unwrap_err();
        assert!(matches!(err, WorktreeError::AlreadyExists(ref id) if id == "05-echo"));
        // Original handle is still there, in registry and on disk.
        assert_eq!(mgr.list().unwrap().len(), 1);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_create_retains_ownership_through_registry_commit() {
        let Some((tmp, manager)) = make_manager_with_budget(1) else {
            return;
        };
        let barrier = install_git_process_barrier(&manager, &tmp);
        let cancelled_manager = manager.clone();
        let caller = tokio::spawn(async move {
            cancelled_manager
                .create("cancel-create", "feature/cancel-create")
                .await
        });
        wait_for_barrier(&barrier.started).await;
        assert_eq!(invocation_pids(&barrier.invocations).len(), 1);

        caller.abort();
        assert!(caller.await.expect_err("caller cancelled").is_cancelled());

        let contender_manager = manager.clone();
        let contender = tokio::spawn(async move {
            contender_manager
                .create("after-cancel", "feature/after-cancel")
                .await
        });
        tokio::time::sleep(Duration::from_millis(75)).await;
        assert!(
            !contender.is_finished(),
            "another clone entered while cancelled create still owned Git"
        );
        assert_eq!(
            invocation_pids(&barrier.invocations).len(),
            1,
            "a second Git mutation overlapped the blocked child"
        );

        std::fs::write(&barrier.release, "release").expect("release Git child");
        let error = tokio::time::timeout(Duration::from_secs(5), contender)
            .await
            .expect("contender completed after reconciliation")
            .expect("contender task")
            .expect_err("completed cancelled create consumes the only slot");
        assert!(matches!(error, WorktreeError::BudgetExhausted { max: 1 }));
        assert_eq!(manager.active_count(), 1);
        assert!(manager.path_for("cancel-create").exists());
        assert!(!manager.path_for("after-cancel").exists());
        assert!(manager.operations.try_lock().is_ok());
        let pids = invocation_pids(&barrier.invocations);
        assert_processes_exited(&pids);
        assert_no_git_locks(&manager.config.repo_root);

        manager
            .remove("cancel-create")
            .await
            .expect("free capacity");
        let retry = manager
            .create("after-cancel", "feature/after-cancel")
            .await
            .expect("capacity restored after reconciled removal");
        assert!(retry.path.exists());
        assert_eq!(manager.active_count(), 1);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_remove_retains_handle_until_git_then_allows_ensure() {
        let Some((tmp, manager)) = make_manager_with_budget(1) else {
            return;
        };
        manager
            .create_for_plan("cancel-remove")
            .await
            .expect("seed worktree");
        let barrier = install_git_process_barrier(&manager, &tmp);
        let cancelled_manager = manager.clone();
        let caller = tokio::spawn(async move { cancelled_manager.remove("cancel-remove").await });
        wait_for_barrier(&barrier.started).await;
        assert_eq!(manager.active_count(), 1, "remove hid active handle early");

        caller.abort();
        assert!(caller.await.expect_err("caller cancelled").is_cancelled());
        let ensure_manager = manager.clone();
        let ensure =
            tokio::spawn(async move { ensure_manager.ensure_for_plan("cancel-remove").await });
        tokio::time::sleep(Duration::from_millis(75)).await;
        assert!(
            !ensure.is_finished(),
            "ensure entered while cancelled remove still owned Git"
        );
        assert_eq!(manager.active_count(), 1, "in-flight handle was lost");
        assert_eq!(
            invocation_pids(&barrier.invocations).len(),
            1,
            "a second Git mutation overlapped the blocked remove"
        );

        std::fs::write(&barrier.release, "release").expect("release Git child");
        let ensured = tokio::time::timeout(Duration::from_secs(5), ensure)
            .await
            .expect("ensure completed after removal reconciliation")
            .expect("ensure task")
            .expect("ensure recreated removed worktree");
        assert_eq!(ensured.id, "cancel-remove");
        assert!(ensured.path.exists());
        assert_eq!(manager.active_count(), 1);
        assert!(manager.operations.try_lock().is_ok());
        let pids = invocation_pids(&barrier.invocations);
        assert!(
            pids.len() >= 3,
            "remove and recreated checkout must run Git"
        );
        assert_processes_exited(&pids);
        assert_no_git_locks(&manager.config.repo_root);
    }

    #[test]
    fn runtime_shutdown_during_linked_phase_rolls_back_before_releasing_owner() {
        let Some((tmp, manager)) = make_manager_with_budget(1) else {
            return;
        };
        let started = tmp.path().join("linked-started");
        let release = tmp.path().join("linked-release");
        manager.set_test_phase_barrier(TestPhaseBarrier {
            phase: CreationPhase::LinkedNoCheckout,
            started: started.clone(),
            release,
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build caller runtime");
        let creating_manager = manager.clone();
        runtime.block_on(async {
            tokio::spawn(async move {
                let _ = creating_manager
                    .create_for_plan("runtime-drop-create")
                    .await;
            });
            wait_for_barrier(&started).await;
        });

        let shutdown_started = std::time::Instant::now();
        drop(runtime);
        assert!(
            shutdown_started.elapsed() < super::RUNTIME_SHUTDOWN_WAIT,
            "caller runtime shutdown exceeded its bounded ownership wait"
        );
        assert!(manager.operations.try_lock().is_ok());
        assert!(manager.get("runtime-drop-create").is_none());
        assert!(!manager.path_for("runtime-drop-create").exists());
        assert!(!manager.creation_marker_path("runtime-drop-create").exists());
        assert!(!manager.creation_claim_path("runtime-drop-create").exists());
        let listed = StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .expect("list worktrees");
        assert!(!super::worktree_list_contains_path(
            &listed.stdout,
            &manager.path_for("runtime-drop-create")
        ));
        assert_no_git_locks(&manager.config.repo_root);
    }

    #[test]
    fn runtime_shutdown_after_reset_commits_registry_and_removes_marker() {
        let Some((tmp, manager)) = make_manager_with_budget(1) else {
            return;
        };
        let started = tmp.path().join("reset-started");
        manager.set_test_phase_barrier(TestPhaseBarrier {
            phase: CreationPhase::ResetComplete,
            started: started.clone(),
            release: tmp.path().join("reset-release"),
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build caller runtime");
        let creating_manager = manager.clone();
        runtime.block_on(async {
            tokio::spawn(async move {
                let _ = creating_manager.create_for_plan("reset-complete").await;
            });
            wait_for_barrier(&started).await;
        });
        drop(runtime);

        assert!(manager.operations.try_lock().is_ok());
        let handle = manager
            .get("reset-complete")
            .expect("completed reset reconciled into registry");
        assert!(handle.path.exists());
        assert!(!manager.creation_marker_path("reset-complete").exists());
        assert!(!manager.creation_claim_path("reset-complete").exists());
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn unproved_create_cleanup_permanently_withholds_mutation_owner() {
        let Some((tmp, manager)) = make_manager_with_budget(1) else {
            return;
        };
        let started = tmp.path().join("cleanup-started");
        manager.set_test_phase_barrier(TestPhaseBarrier {
            phase: CreationPhase::LinkedNoCheckout,
            started: started.clone(),
            release: tmp.path().join("cleanup-release"),
        });
        manager.set_test_cleanup_failure();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build caller runtime");
        let creating_manager = manager.clone();
        runtime.block_on(async {
            tokio::spawn(async move {
                let _ = creating_manager.create_for_plan("cleanup-unproved").await;
            });
            wait_for_barrier(&started).await;
        });
        drop(runtime);

        assert!(manager.operations.try_lock().is_err());
        assert!(manager.path_for("cleanup-unproved").exists());
        assert!(manager.creation_claim_path("cleanup-unproved").exists());
        assert_eq!(manager.active_count(), 0);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn mutation_wrapper_cannot_fork_an_escaped_process() {
        use std::os::unix::fs::PermissionsExt;

        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let escaped = tmp.path().join("escaped-descendant");
        let wrapper = tmp.path().join("forking-git-wrapper.sh");
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\n( printf escaped > '{}' ) &\nexec git \"$@\"\n",
                escaped.display()
            ),
        )
        .expect("write adversarial wrapper");
        let mut permissions = std::fs::metadata(&wrapper).unwrap().permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&wrapper, permissions).unwrap();
        manager.set_test_git_binary(wrapper);

        let result = manager.create("no-escape", "feature/no-escape").await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!escaped.exists(), "wrapper fork escaped containment");
        assert!(result.is_err(), "forking wrapper should fail closed");
        assert!(!manager.path_for("no-escape").exists());
        assert!(!manager.creation_marker_path("no-escape").exists());
        assert!(!manager.creation_claim_path("no-escape").exists());
        assert!(manager.operations.try_lock().is_ok());
    }

    #[tokio::test]
    async fn checkout_extension_is_rejected_before_creation_side_effects() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let status = StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["config", "filter.hostile.smudge", "/usr/bin/true"])
            .status()
            .expect("configure hostile filter");
        assert!(status.success());

        let error = manager
            .create("policy-reject", "feature/policy-reject")
            .await
            .expect_err("checkout extension must fail closed");
        assert!(matches!(error, WorktreeError::UnsafeGitExecution { .. }));
        assert!(!manager.path_for("policy-reject").exists());
        assert!(!manager.creation_marker_path("policy-reject").exists());
        assert!(!manager.creation_claim_path("policy-reject").exists());
        assert!(manager.operations.try_lock().is_ok());
    }

    #[tokio::test]
    async fn direct_create_preserves_outstanding_marker_and_owned_objects() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let id = "outstanding-marker";
        let path = manager.path_for(id);
        let admin_dir = manager
            .config
            .repo_root
            .join(".git/worktrees/roko-outstanding-marker");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::create_dir_all(&admin_dir).unwrap();
        std::fs::write(path.join("owned-sentinel"), b"old path object").unwrap();
        std::fs::write(admin_dir.join("owned-sentinel"), b"old admin object").unwrap();
        let marker_path = manager.creation_marker_path(id);
        std::fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
        let original_bytes = format!(
            "{{\n  \"id\": \"{id}\",\n  \"branch\": \"old/branch\",\n  \"path\": \"{}\",\n  \"admin_dir\": \"{}\",\n  \"phase\": \"linked_no_checkout\"\n}}\n",
            path.display(),
            admin_dir.display()
        )
        .into_bytes();
        std::fs::write(&marker_path, &original_bytes).unwrap();

        for result in [
            manager.create(id, "new/branch").await.map(|_| ()),
            manager.create_for_plan(id).await.map(|_| ()),
        ] {
            assert!(matches!(
                result,
                Err(WorktreeError::ReattachRejected { .. })
            ));
            assert_eq!(std::fs::read(&marker_path).unwrap(), original_bytes);
            assert_eq!(
                std::fs::read(path.join("owned-sentinel")).unwrap(),
                b"old path object"
            );
            assert_eq!(
                std::fs::read(admin_dir.join("owned-sentinel")).unwrap(),
                b"old admin object"
            );
        }
        assert!(manager.operations.try_lock().is_ok());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn dangling_creation_marker_rejects_direct_create() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let id = "dangling-marker";
        let marker_path = manager.creation_marker_path(id);
        std::fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
        let missing_target = marker_path.with_extension("missing");
        std::os::unix::fs::symlink(&missing_target, &marker_path).unwrap();

        let error = manager
            .create_for_plan(id)
            .await
            .expect_err("dangling marker must retain the claim");
        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        assert!(
            std::fs::symlink_metadata(&marker_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(!missing_target.exists());
        assert!(!manager.path_for(id).exists());
    }

    #[test]
    fn creation_marker_transitions_require_exact_identity_and_prior_phase() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let target_oid = String::from_utf8(
            StdCommand::new("git")
                .current_dir(&manager.config.repo_root)
                .args(["rev-parse", "main"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        let common_git_dir = std::fs::canonicalize(manager.config.repo_root.join(".git")).unwrap();
        let marker = CreationMarker {
            schema_version: super::CREATION_MARKER_SCHEMA,
            claim_id: uuid::Uuid::new_v4().simple().to_string(),
            id: "phase-identity".to_string(),
            repo_root: manager.config.repo_root.clone(),
            common_git_dir: common_git_dir.clone(),
            branch: "feature/phase-identity".to_string(),
            branch_old_oid: None,
            target_oid,
            path: manager.path_for("phase-identity"),
            admin_dir: common_git_dir.join("worktrees/roko-phase-identity"),
            phase: CreationPhase::Prepared,
            previous_digest: None,
        };
        let mut claim = manager.publish_creation_marker(marker).unwrap();
        let claim_path = manager.creation_claim_path(&claim.marker.id);
        let prepared_path = claim_path.join(super::creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::Prepared,
        ));
        let prepared_bytes = std::fs::read(&prepared_path).unwrap();

        assert!(
            manager
                .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
                .is_err()
        );
        assert_eq!(std::fs::read(&prepared_path).unwrap(), prepared_bytes);

        manager
            .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
            .unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
            .unwrap();
        let linked = std::fs::read(claim_path.join(super::creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::LinkedNoCheckout,
        )))
        .unwrap();
        let reset: CreationMarker = serde_json::from_slice(
            &std::fs::read(claim_path.join(super::creation_record_name(
                &claim.marker.claim_id,
                CreationPhase::ResetComplete,
            )))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            reset.previous_digest,
            Some(blake3::hash(&linked).to_hex().to_string())
        );
        // This unit test does not seed the completed worktree filesystem;
        // rollback cleanup exercises exact fd-relative removal instead.
        manager.remove_creation_claim_if_exact(&claim).unwrap();
        assert!(std::fs::symlink_metadata(claim_path).is_err());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn transition_path_swap_returns_error_and_preserves_foreign_claim_bytes() {
        use std::os::unix::fs::PermissionsExt;

        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "transition-swap"))
            .unwrap();
        let public = manager.creation_claim_path("transition-swap");
        let detached = tmp.path().join("detached-transition-claim");
        let started = tmp.path().join("transition-swap-started");
        let release = tmp.path().join("transition-swap-release");
        manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
            point: TestClaimMutationPoint::BeforeTransitionWrite,
            started: started.clone(),
            release: release.clone(),
        });
        let worker_manager = manager.clone();
        let worker = std::thread::spawn(move || {
            let mut claim = claim;
            worker_manager.transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
        });
        wait_for_file_sync(&started);
        std::fs::rename(&public, &detached).unwrap();
        std::fs::create_dir(&public).unwrap();
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o700)).unwrap();
        let foreign = public.join("foreign-claim.json");
        std::fs::write(&foreign, b"foreign transition claim\n").unwrap();
        std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(0o600)).unwrap();
        let foreign_bytes = std::fs::read(&foreign).unwrap();
        std::fs::write(&release, b"release").unwrap();
        assert!(worker.join().unwrap().is_err());
        assert_eq!(std::fs::read(&foreign).unwrap(), foreign_bytes);
        assert_eq!(std::fs::read_dir(&public).unwrap().count(), 1);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn removal_path_swap_returns_error_and_preserves_foreign_claim_bytes() {
        use std::os::unix::fs::PermissionsExt;

        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let mut claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "removal-swap"))
            .unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
            .unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
            .unwrap();
        let public = manager.creation_claim_path("removal-swap");
        let detached = tmp.path().join("detached-removal-claim");
        let started = tmp.path().join("removal-swap-started");
        let release = tmp.path().join("removal-swap-release");
        manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
            point: TestClaimMutationPoint::BeforeRemovalCleanup,
            started: started.clone(),
            release: release.clone(),
        });
        let worker_manager = manager.clone();
        let worker =
            std::thread::spawn(move || worker_manager.remove_completed_creation_marker(&claim));
        wait_for_file_sync(&started);
        std::fs::rename(&public, &detached).unwrap();
        std::fs::create_dir(&public).unwrap();
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o700)).unwrap();
        let foreign = public.join("foreign-claim.json");
        std::fs::write(&foreign, b"foreign removal claim\n").unwrap();
        std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(0o600)).unwrap();
        let foreign_bytes = std::fs::read(&foreign).unwrap();
        std::fs::write(&release, b"release").unwrap();
        assert!(worker.join().unwrap().is_err());
        assert_eq!(std::fs::read(&foreign).unwrap(), foreign_bytes);
        assert_eq!(std::fs::read_dir(&public).unwrap().count(), 1);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn parent_root_swap_returns_error_and_preserves_foreign_root_bytes() {
        use std::os::unix::fs::PermissionsExt;

        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "root-swap"))
            .unwrap();
        let public_root = manager
            .config
            .worktrees_root
            .join(super::CREATION_MARKER_DIR);
        let detached = tmp.path().join("detached-marker-root");
        let started = tmp.path().join("root-swap-started");
        let release = tmp.path().join("root-swap-release");
        manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
            point: TestClaimMutationPoint::BeforeTransitionWrite,
            started: started.clone(),
            release: release.clone(),
        });
        let worker_manager = manager.clone();
        let worker = std::thread::spawn(move || {
            let mut claim = claim;
            worker_manager.transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
        });
        wait_for_file_sync(&started);
        std::fs::rename(&public_root, &detached).unwrap();
        std::fs::create_dir(&public_root).unwrap();
        std::fs::set_permissions(&public_root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let foreign = public_root.join("foreign-root-record");
        std::fs::write(&foreign, b"foreign root bytes\n").unwrap();
        std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&release, b"release").unwrap();
        assert!(worker.join().unwrap().is_err());
        assert_eq!(std::fs::read(&foreign).unwrap(), b"foreign root bytes\n");
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn wait_for_file_sync(path: &Path) {
        let started = std::time::Instant::now();
        while !path.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "barrier timeout"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn prepared_claim_is_restart_fail_closed_and_byte_preserved() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "prepared-restart"))
            .unwrap();
        let claim_path = manager.creation_claim_path("prepared-restart");
        let prepared = claim_path.join(super::creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::Prepared,
        ));
        let bytes = std::fs::read(&prepared).unwrap();
        drop(claim);

        let error = manager
            .ensure_for_plan("prepared-restart")
            .await
            .expect_err("Prepared restart state must remain fail closed");
        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        assert_eq!(std::fs::read(prepared).unwrap(), bytes);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn empty_claim_publication_window_is_recovered_under_repository_lock() {
        use std::os::unix::fs::PermissionsExt;

        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let _lock = manager.acquire_repository_mutation_lock().unwrap();
        let _root = manager.open_creation_marker_root(true).unwrap();
        let empty = manager.creation_claim_path("empty-restart");
        std::fs::create_dir(&empty).unwrap();
        std::fs::set_permissions(&empty, std::fs::Permissions::from_mode(0o700)).unwrap();
        manager
            .recover_or_reject_creation_claim("empty-restart")
            .await
            .unwrap();
        assert!(std::fs::symlink_metadata(empty).is_err());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn live_owner_prevents_empty_claim_recovery() {
        use std::os::unix::fs::PermissionsExt;

        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let owner = manager.acquire_repository_mutation_lock().unwrap();
        let _root = manager.open_creation_marker_root(true).unwrap();
        let empty = manager.creation_claim_path("live-empty");
        std::fs::create_dir(&empty).unwrap();
        std::fs::set_permissions(&empty, std::fs::Permissions::from_mode(0o700)).unwrap();
        let contender = WorktreeManager::new((*manager.config).clone());
        let task = tokio::spawn(async move { contender.ensure_for_plan("live-empty").await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !task.is_finished(),
            "live owner did not exclude empty recovery"
        );
        assert!(empty.is_dir());
        drop(owner);
        let recovered = task.await.unwrap().unwrap();
        assert!(recovered.path.exists());
        assert!(std::fs::symlink_metadata(empty).is_err());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn live_owner_prevents_reset_complete_recovery() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let handle = manager.create_for_plan("live-reset").await.unwrap();
        let owner = manager.acquire_repository_mutation_lock().unwrap();
        let mut marker = test_creation_marker(&manager, "live-reset");
        marker.branch = handle.branch.clone();
        marker.path = handle.path.clone();
        marker.admin_dir = super::read_gitdir(&handle.path).unwrap();
        marker.branch_old_oid = Some(marker.target_oid.clone());
        let mut claim = manager.publish_creation_marker(marker).unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
            .unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
            .unwrap();
        let claim_path = manager.creation_claim_path("live-reset");
        let contender = WorktreeManager::new((*manager.config).clone());
        let task = tokio::spawn(async move { contender.ensure_for_plan("live-reset").await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !task.is_finished(),
            "live owner did not exclude ResetComplete recovery"
        );
        assert!(claim_path.is_dir());
        drop(claim);
        drop(owner);
        let recovered = task.await.unwrap().unwrap();
        assert_eq!(recovered.path, handle.path);
        assert!(std::fs::symlink_metadata(claim_path).is_err());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn record_symlink_is_rejected_without_following_or_removal() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "record-symlink"))
            .unwrap();
        let record =
            manager
                .creation_claim_path("record-symlink")
                .join(super::creation_record_name(
                    &claim.marker.claim_id,
                    CreationPhase::Prepared,
                ));
        let outside = tmp.path().join("outside-record");
        std::fs::write(&outside, b"outside bytes\n").unwrap();
        let outside_bytes = std::fs::read(&outside).unwrap();
        std::fs::remove_file(&record).unwrap();
        std::os::unix::fs::symlink(&outside, &record).unwrap();
        drop(claim);

        assert!(manager.ensure_for_plan("record-symlink").await.is_err());
        assert!(
            std::fs::symlink_metadata(&record)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read(outside).unwrap(), outside_bytes);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn hard_linked_record_is_rejected_and_preserved() {
        use std::os::unix::fs::MetadataExt;

        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "record-hardlink"))
            .unwrap();
        let record =
            manager
                .creation_claim_path("record-hardlink")
                .join(super::creation_record_name(
                    &claim.marker.claim_id,
                    CreationPhase::Prepared,
                ));
        let outside = tmp.path().join("outside-hardlink");
        std::fs::hard_link(&record, &outside).unwrap();
        let bytes = std::fs::read(&record).unwrap();
        drop(claim);

        assert!(manager.ensure_for_plan("record-hardlink").await.is_err());
        assert_eq!(std::fs::read(&record).unwrap(), bytes);
        assert_eq!(std::fs::read(&outside).unwrap(), bytes);
        assert_eq!(std::fs::metadata(record).unwrap().nlink(), 2);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn mixed_uuid_record_is_rejected_and_preserved() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "mixed-uuid"))
            .unwrap();
        let record = manager
            .creation_claim_path("mixed-uuid")
            .join(super::creation_record_name(
                &claim.marker.claim_id,
                CreationPhase::Prepared,
            ));
        let mut foreign = claim.marker.clone();
        foreign.claim_id = uuid::Uuid::new_v4().simple().to_string();
        let mut foreign_bytes = serde_json::to_vec(&foreign).unwrap();
        foreign_bytes.push(b'\n');
        std::fs::write(&record, &foreign_bytes).unwrap();
        drop(claim);

        assert!(manager.ensure_for_plan("mixed-uuid").await.is_err());
        assert_eq!(std::fs::read(record).unwrap(), foreign_bytes);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn branch_compare_and_swap_rejects_drift_and_preserves_foreign_ref() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let started = tmp.path().join("branch-cas-started");
        let release = tmp.path().join("branch-cas-release");
        manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
            point: TestClaimMutationPoint::BeforeBranchCas,
            started: started.clone(),
            release: release.clone(),
        });
        let creating = manager.clone();
        let task =
            tokio::spawn(async move { creating.create("cas-drift", "feature/cas-drift").await });
        wait_for_barrier(&started).await;
        assert!(
            StdCommand::new("git")
                .current_dir(&manager.config.repo_root)
                .args(["commit", "--allow-empty", "-m", "foreign drift"])
                .status()
                .unwrap()
                .success()
        );
        let foreign_oid = String::from_utf8(
            StdCommand::new("git")
                .current_dir(&manager.config.repo_root)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        assert!(
            StdCommand::new("git")
                .current_dir(&manager.config.repo_root)
                .args(["update-ref", "refs/heads/feature/cas-drift", &foreign_oid,])
                .status()
                .unwrap()
                .success()
        );
        std::fs::write(&release, b"release").unwrap();
        assert!(task.await.unwrap().is_err());
        let actual = String::from_utf8(
            StdCommand::new("git")
                .current_dir(&manager.config.repo_root)
                .args(["rev-parse", "refs/heads/feature/cas-drift"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        assert_eq!(actual.trim(), foreign_oid);
        assert!(!manager.path_for("cas-drift").exists());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn legacy_marker_kinds_and_bytes_never_migrate_automatically() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        for (id, bytes) in [
            ("legacy-prepared", b"{malformed prepared\n".as_slice()),
            (
                "legacy-reset",
                b"{\"phase\":\"reset_complete\"}\n".as_slice(),
            ),
        ] {
            let path = manager.creation_marker_path(id);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, bytes).unwrap();
            let before = std::fs::read(&path).unwrap();
            assert!(manager.ensure_for_plan(id).await.is_err());
            assert_eq!(std::fs::read(&path).unwrap(), before);
            assert!(!manager.creation_claim_path(id).exists());
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn cleanup_safe_restart_reproves_checkout_and_removes_terminal_claim() {
        use std::os::unix::fs::PermissionsExt;

        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let handle = manager
            .create_for_plan("cleanup-restart")
            .await
            .expect("seed completed checkout");
        let mut marker = test_creation_marker(&manager, "cleanup-restart");
        marker.branch = handle.branch.clone();
        marker.path = handle.path.clone();
        marker.admin_dir = super::read_gitdir(&handle.path).unwrap();
        marker.branch_old_oid = Some(marker.target_oid.clone());
        let mut claim = manager.publish_creation_marker(marker).unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
            .unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
            .unwrap();
        super::ensure_cleanup_safe(&claim.claim_dir_fd, &claim.marker).unwrap();
        super::unlink_claim_file(
            &claim.claim_dir_fd,
            &super::creation_record_name(&claim.marker.claim_id, CreationPhase::Prepared),
        )
        .unwrap();
        super::unlink_claim_file(
            &claim.claim_dir_fd,
            &super::creation_record_name(&claim.marker.claim_id, CreationPhase::LinkedNoCheckout),
        )
        .unwrap();
        let claim_path = manager.creation_claim_path("cleanup-restart");
        let reset_path = claim_path.join(super::creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::ResetComplete,
        ));
        let cleanup_path = claim_path.join("cleanup-safe.json");
        let reset_bytes = std::fs::read(&reset_path).unwrap();
        let cleanup_bytes = std::fs::read(&cleanup_path).unwrap();
        let unknown_path = claim_path.join("foreign-record");
        std::fs::write(&unknown_path, b"foreign\n").unwrap();
        std::fs::set_permissions(&unknown_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        drop(claim);

        assert!(manager.ensure_for_plan("cleanup-restart").await.is_err());
        assert_eq!(std::fs::read(&reset_path).unwrap(), reset_bytes);
        assert_eq!(std::fs::read(&cleanup_path).unwrap(), cleanup_bytes);
        assert_eq!(std::fs::read(&unknown_path).unwrap(), b"foreign\n");
        std::fs::remove_file(unknown_path).unwrap();

        let recovered = manager.ensure_for_plan("cleanup-restart").await.unwrap();
        assert_eq!(recovered.path, handle.path);
        assert!(std::fs::symlink_metadata(claim_path).is_err());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn every_cleanup_unlink_crash_prefix_converges() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        for prefix_len in 0..=5 {
            let id = format!("cleanup-prefix-{prefix_len}");
            let handle = manager.create_for_plan(&id).await.unwrap();
            let mut marker = test_creation_marker(&manager, &id);
            marker.branch = handle.branch.clone();
            marker.path = handle.path.clone();
            marker.admin_dir = super::read_gitdir(&handle.path).unwrap();
            marker.branch_old_oid = Some(marker.target_oid.clone());
            let mut claim = manager.publish_creation_marker(marker).unwrap();
            manager
                .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
                .unwrap();
            manager
                .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
                .unwrap();
            super::ensure_cleanup_safe(&claim.claim_dir_fd, &claim.marker).unwrap();
            let cleanup_order = [
                super::creation_record_name(&claim.marker.claim_id, CreationPhase::Prepared),
                super::creation_record_name(
                    &claim.marker.claim_id,
                    CreationPhase::LinkedNoCheckout,
                ),
                super::creation_record_name(&claim.marker.claim_id, CreationPhase::ResetComplete),
                "claim-id".to_string(),
                "cleanup-safe.json".to_string(),
            ];
            for name in cleanup_order.iter().take(prefix_len) {
                super::unlink_claim_file(&claim.claim_dir_fd, name).unwrap();
            }
            rustix::fs::fsync(&claim.claim_dir_fd).unwrap();
            let claim_path = manager.creation_claim_path(&id);
            drop(claim);

            let recovered = manager.ensure_for_plan(&id).await.unwrap();
            assert_eq!(recovered.path, handle.path, "prefix {prefix_len}");
            assert!(
                std::fs::symlink_metadata(&claim_path).is_err(),
                "prefix {prefix_len} left a claim"
            );
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn unknown_journal_entry_fails_closed_without_cleanup() {
        use std::os::unix::fs::PermissionsExt;

        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
        let claim = manager
            .publish_creation_marker(test_creation_marker(&manager, "unknown-entry"))
            .unwrap();
        let unknown = manager
            .creation_claim_path("unknown-entry")
            .join("foreign-record");
        std::fs::write(&unknown, b"foreign\n").unwrap();
        std::fs::set_permissions(&unknown, std::fs::Permissions::from_mode(0o600)).unwrap();
        drop(claim);
        assert!(manager.ensure_for_plan("unknown-entry").await.is_err());
        assert_eq!(std::fs::read(unknown).unwrap(), b"foreign\n");
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn repository_flock_serializes_cross_root_manager_instances() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let owner = manager.acquire_repository_mutation_lock().unwrap();
        let contender =
            manager_with_worktrees_root(&manager, tmp.path().join("alternate-worktrees"));
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let guard = contender.acquire_repository_mutation_lock().unwrap();
            acquired_tx.send(()).unwrap();
            drop(guard);
        });
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "cross-root manager bypassed canonical repository flock"
        );
        drop(owner);
        acquired_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
        assert!(repository_lock_path(&manager).exists());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn linked_repo_root_uses_the_same_canonical_common_directory_lock() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let linked_root = tmp.path().join("linked-repository-root");
        let added = StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args([
                "worktree",
                "add",
                "-b",
                "linked-root",
                linked_root.to_str().unwrap(),
                "main",
            ])
            .status()
            .unwrap();
        assert!(added.success());
        assert!(linked_root.join(".git").is_file());
        let linked_manager = WorktreeManager::new(WorktreeConfig {
            repo_root: linked_root,
            base_branch: "main".to_string(),
            worktrees_root: tmp.path().join("linked-manager-worktrees"),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        assert_eq!(
            repository_lock_path(&manager),
            repository_lock_path(&linked_manager)
        );

        let owner = manager.acquire_repository_mutation_lock().unwrap();
        let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
        let linked_contender = linked_manager.clone();
        let worker = std::thread::spawn(move || {
            let _guard = linked_contender.acquire_repository_mutation_lock().unwrap();
            acquired_tx.send(()).unwrap();
        });
        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "linked repo_root bypassed its canonical common-directory lock"
        );
        drop(owner);
        acquired_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let created = runtime
            .block_on(linked_manager.create_for_plan("linked-root-create"))
            .unwrap();
        assert!(created.path.exists());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cross_root_create_serializes_same_and_different_ids() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let contender =
            manager_with_worktrees_root(&manager, tmp.path().join("alternate-worktrees"));

        let same_started = tmp.path().join("same-create-started");
        let same_release = tmp.path().join("same-create-release");
        manager.set_test_phase_barrier(TestPhaseBarrier {
            phase: CreationPhase::LinkedNoCheckout,
            started: same_started.clone(),
            release: same_release.clone(),
        });
        let owner_manager = manager.clone();
        let owner =
            tokio::spawn(async move { owner_manager.create_for_plan("cross-root-same").await });
        wait_for_barrier(&same_started).await;
        let same_contender = contender.clone();
        let raced =
            tokio::spawn(async move { same_contender.create_for_plan("cross-root-same").await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !raced.is_finished(),
            "same-id create bypassed repository lock"
        );
        std::fs::write(&same_release, b"release").unwrap();
        let first = owner.await.unwrap().unwrap();
        let error = raced.await.unwrap().unwrap_err();
        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        assert!(first.path.exists());
        assert!(!contender.path_for("cross-root-same").exists());

        let different_started = tmp.path().join("different-create-started");
        let different_release = tmp.path().join("different-create-release");
        manager.set_test_phase_barrier(TestPhaseBarrier {
            phase: CreationPhase::LinkedNoCheckout,
            started: different_started.clone(),
            release: different_release.clone(),
        });
        let owner_manager = manager.clone();
        let owner =
            tokio::spawn(async move { owner_manager.create_for_plan("cross-root-owner").await });
        wait_for_barrier(&different_started).await;
        let different_contender = contender.clone();
        let raced = tokio::spawn(async move {
            different_contender
                .create_for_plan("cross-root-contender")
                .await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !raced.is_finished(),
            "different-id create bypassed repository lock"
        );
        std::fs::write(&different_release, b"release").unwrap();
        assert!(owner.await.unwrap().unwrap().path.exists());
        assert!(raced.await.unwrap().unwrap().path.exists());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cross_root_create_serializes_remove() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let remover = manager_with_worktrees_root(&manager, tmp.path().join("remover-worktrees"));
        let removed_path = remover
            .create_for_plan("cross-root-remove")
            .await
            .unwrap()
            .path;
        let started = tmp.path().join("create-remove-started");
        let release = tmp.path().join("create-remove-release");
        manager.set_test_phase_barrier(TestPhaseBarrier {
            phase: CreationPhase::LinkedNoCheckout,
            started: started.clone(),
            release: release.clone(),
        });
        let owner_manager = manager.clone();
        let owner =
            tokio::spawn(async move { owner_manager.create_for_plan("cross-root-create").await });
        wait_for_barrier(&started).await;
        let remove_manager = remover.clone();
        let raced = tokio::spawn(async move { remove_manager.remove("cross-root-remove").await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !raced.is_finished(),
            "remove bypassed cross-root create owner"
        );
        assert!(removed_path.exists());
        std::fs::write(&release, b"release").unwrap();
        assert!(owner.await.unwrap().unwrap().path.exists());
        raced.await.unwrap().unwrap();
        assert!(!removed_path.exists());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn repository_flock_serializes_a_separate_process() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let owner = manager.acquire_repository_mutation_lock().unwrap();
        let started = tmp.path().join("subprocess-lock-started");
        let acquired = tmp.path().join("subprocess-lock-acquired");
        let mut child = StdCommand::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "worktree::tests::repository_lock_process_helper",
                "--nocapture",
            ])
            .env("ROKO_TEST_REPO_ROOT", &manager.config.repo_root)
            .env(
                "ROKO_TEST_WORKTREES_ROOT",
                tmp.path().join("subprocess-worktrees"),
            )
            .env("ROKO_TEST_LOCK_STARTED", &started)
            .env("ROKO_TEST_LOCK_ACQUIRED", &acquired)
            .spawn()
            .unwrap();
        wait_for_file_sync(&started);
        std::thread::sleep(Duration::from_millis(100));
        assert!(!acquired.exists(), "subprocess bypassed repository flock");
        drop(owner);
        wait_for_file_sync(&acquired);
        assert!(child.wait().unwrap().success());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cross_root_create_serializes_a_subprocess_prune() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let started = tmp.path().join("subprocess-prune-started");
        let acquired = tmp.path().join("subprocess-prune-complete");
        let create_started = tmp.path().join("subprocess-create-started");
        let create_release = tmp.path().join("subprocess-create-release");
        manager.set_test_phase_barrier(TestPhaseBarrier {
            phase: CreationPhase::LinkedNoCheckout,
            started: create_started.clone(),
            release: create_release.clone(),
        });
        let owner_manager = manager.clone();
        let owner = tokio::spawn(async move {
            owner_manager
                .create_for_plan("subprocess-prune-owner")
                .await
        });
        wait_for_barrier(&create_started).await;
        let mut child = StdCommand::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "worktree::tests::repository_lock_process_helper",
                "--nocapture",
            ])
            .env("ROKO_TEST_REPO_ROOT", &manager.config.repo_root)
            .env(
                "ROKO_TEST_WORKTREES_ROOT",
                tmp.path().join("subprocess-prune-worktrees"),
            )
            .env("ROKO_TEST_LOCK_ACTION", "prune")
            .env("ROKO_TEST_LOCK_STARTED", &started)
            .env("ROKO_TEST_LOCK_ACQUIRED", &acquired)
            .spawn()
            .unwrap();
        wait_for_file_sync(&started);
        std::thread::sleep(Duration::from_millis(100));
        assert!(!acquired.exists(), "subprocess prune bypassed create owner");
        std::fs::write(&create_release, b"release").unwrap();
        assert!(owner.await.unwrap().unwrap().path.exists());
        wait_for_file_sync(&acquired);
        assert!(child.wait().unwrap().success());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn repository_lock_process_helper() {
        let Ok(repo_root) = std::env::var("ROKO_TEST_REPO_ROOT") else {
            return;
        };
        let worktrees_root = std::env::var("ROKO_TEST_WORKTREES_ROOT").unwrap();
        let started = std::env::var("ROKO_TEST_LOCK_STARTED").unwrap();
        let acquired = std::env::var("ROKO_TEST_LOCK_ACQUIRED").unwrap();
        let manager = WorktreeManager::new(WorktreeConfig {
            repo_root: PathBuf::from(repo_root),
            base_branch: "main".to_string(),
            worktrees_root: PathBuf::from(worktrees_root),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        std::fs::write(started, b"started").unwrap();
        if std::env::var("ROKO_TEST_LOCK_ACTION").as_deref() == Ok("prune") {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(manager.prune())
                .unwrap();
        } else {
            let _owner = manager.acquire_repository_mutation_lock().unwrap();
        }
        std::fs::write(acquired, b"acquired").unwrap();
    }

    #[tokio::test]
    async fn remove_spawn_failure_preserves_registry_for_retry() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let handle = manager
            .create_for_plan("remove-error")
            .await
            .expect("seed worktree");
        manager.set_test_git_binary(PathBuf::from("definitely-not-a-git-binary"));

        let error = manager
            .remove("remove-error")
            .await
            .expect_err("missing Git binary must fail");
        assert!(matches!(error, WorktreeError::IoError(_)));
        assert_eq!(manager.get("remove-error"), Some(handle.clone()));
        assert!(handle.path.exists());
        assert_eq!(manager.active_count(), 1);

        manager.set_test_git_binary(PathBuf::from("git"));
        manager.remove("remove-error").await.expect("retry removal");
        assert_eq!(manager.active_count(), 0);
        assert!(!handle.path.exists());
    }

    #[tokio::test]
    async fn path_for_does_not_touch_disk() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root: tmp.path().join("repo"),
            base_branch: "main".to_string(),
            worktrees_root: tmp.path().join("worktrees"),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        let p = mgr.path_for("06-foxtrot");
        assert_eq!(p, tmp.path().join("worktrees").join("06-foxtrot"));
        assert!(!p.exists(), "path_for should be pure");
        // No git calls were made, so nothing was created.
        assert!(mgr.list().unwrap().is_empty());
    }

    #[tokio::test]
    async fn concurrent_task_attempts_are_file_and_branch_disjoint() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let (first, sibling) = tokio::join!(
            manager.create_for_attempt("plan", "first", 1),
            manager.create_for_attempt("plan", "sibling", 1),
        );
        let first = first.unwrap();
        let sibling = sibling.unwrap();
        assert_ne!(first.id, sibling.id);
        assert_ne!(first.branch, sibling.branch);
        std::fs::write(first.path.join("first-only.txt"), b"owned by first\n").unwrap();
        std::fs::write(sibling.path.join("sibling-only.txt"), b"owned by sibling\n").unwrap();
        assert!(!first.path.join("sibling-only.txt").exists());
        assert!(!sibling.path.join("first-only.txt").exists());
        assert_eq!(first.id, format_attempt_worktree_id("plan", "first", 1));
        assert_eq!(first.branch, format_attempt_branch_name("plan", "first", 1));
    }

    #[tokio::test]
    async fn accepted_attempt_is_the_immutable_base_for_the_next_task() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let first = manager
            .create_for_attempt("plan", "first", 1)
            .await
            .unwrap();
        std::fs::write(first.path.join("accepted.txt"), b"first\n").unwrap();
        for args in [
            vec!["add", "accepted.txt"],
            vec!["commit", "-m", "accepted first attempt"],
        ] {
            assert!(
                StdCommand::new("git")
                    .current_dir(&first.path)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let accepted = manager.accept_attempt("plan", "first", 1).await.unwrap();
        std::fs::write(first.path.join("late.txt"), b"must not propagate\n").unwrap();
        for args in [
            vec!["add", "late.txt"],
            vec!["commit", "-m", "late mutation"],
        ] {
            assert!(
                StdCommand::new("git")
                    .current_dir(&first.path)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }

        let next = manager.create_for_attempt("plan", "next", 1).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(next.path.join("accepted.txt")).unwrap(),
            "first\n"
        );
        assert!(!next.path.join("late.txt").exists());
        let next_oid = StdCommand::new("git")
            .current_dir(&next.path)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&next_oid.stdout).trim(),
            accepted.commit_oid
        );
        assert_eq!(manager.accepted_for_plan("plan"), Some(accepted));
        assert_ne!(first.path, next.path);
    }

    #[tokio::test]
    async fn removal_preserves_dirty_attempt_for_attribution_and_recovery() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let attempt = manager
            .create_for_attempt("plan", "dirty", 1)
            .await
            .unwrap();
        std::fs::write(attempt.path.join("unknown.txt"), b"do not delete\n").unwrap();

        let error = manager.remove(&attempt.id).await.unwrap_err();

        assert!(matches!(error, WorktreeError::DirtyWorktree { .. }));
        assert_eq!(
            std::fs::read_to_string(attempt.path.join("unknown.txt")).unwrap(),
            "do not delete\n"
        );
        assert_eq!(manager.get(&attempt.id), Some(attempt));
    }

    #[tokio::test]
    async fn invalid_ids_are_rejected_before_git_runs() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root: tmp.path().join("repo"),
            base_branch: "main".to_string(),
            worktrees_root: tmp.path().join("worktrees"),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        // Neither repo nor worktrees_root exists — but we never reach
        // git because validate_id rejects these inputs first.
        for bad in [
            "",
            ".hidden",
            "-starts-with-dash",
            "has/slash",
            "has\\back",
            "white space",
            "has..dots",
            "has~tilde",
            "has^caret",
            "has:colon",
            "has?q",
            "has*star",
            "has[bracket",
            "has@at",
            "has\ttab",
        ] {
            let err = mgr.create(bad, "feature/x").await.unwrap_err();
            assert!(
                matches!(err, WorktreeError::InvalidId(_)),
                "expected InvalidId for `{bad}`, got {err:?}"
            );
        }
    }

    #[test]
    fn validate_id_accepts_reasonable_ids() {
        for good in [
            "01-alpha",
            "plan_42",
            "08a-some-thing",
            "nested.branch-name",
        ] {
            validate_id(good).unwrap();
        }
    }

    // ── New tests (§15.3–§15.7, §15.9) ──

    #[test]
    fn format_branch_name_uses_convention() {
        assert_eq!(format_branch_name("01-alpha"), "roko/plan/01-alpha");
        assert_eq!(format_branch_name("fix_typo"), "roko/plan/fix_typo");
    }

    #[tokio::test]
    async fn create_for_plan_uses_canonical_branch() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let handle = mgr.create_for_plan("07-golf").await.unwrap();
        assert_eq!(handle.branch, "roko/plan/07-golf");
        assert_eq!(handle.id, "07-golf");
        assert!(handle.path.exists());
    }

    #[tokio::test]
    async fn get_and_plan_path_return_tracked_handle() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let handle = mgr.create_for_plan("07-golf-2").await.unwrap();
        let fetched = mgr.get("07-golf-2").unwrap();
        assert_eq!(fetched.id, "07-golf-2");
        assert_eq!(mgr.plan_path("07-golf-2"), Some(handle.path));
    }

    #[tokio::test]
    async fn ensure_for_plan_reuses_existing_worktree() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let first = mgr.create_for_plan("07-golf-3").await.unwrap();
        let before_count = mgr.active_count();
        let ensured = mgr.ensure_for_plan("07-golf-3").await.unwrap();
        assert_eq!(before_count, mgr.active_count());
        assert_eq!(ensured.id, first.id);
        assert_eq!(ensured.path, first.path);
        assert_eq!(ensured.branch, first.branch);
    }

    #[tokio::test]
    async fn discover_existing_reattaches_on_disk_worktrees() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        // Create a worktree the normal way.
        let original = mgr.create_for_plan("09-resume").await.unwrap();
        let original_path = original.path.clone();
        let original_branch = original.branch.clone();
        assert!(original_path.exists());

        // Build a *fresh* manager that doesn't know about the worktree.
        let mgr2 = WorktreeManager::new(WorktreeConfig {
            repo_root: mgr.config.repo_root.clone(),
            base_branch: "main".to_string(),
            worktrees_root: original_path.parent().unwrap().to_path_buf(),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        assert_eq!(mgr2.active_count(), 0);

        // discover_existing should find the on-disk worktree.
        let discovered = mgr2.discover_existing(&["09-resume", "nonexistent"]).await;
        assert_eq!(discovered, vec!["09-resume".to_string()]);
        assert_eq!(mgr2.active_count(), 1);

        let handle = mgr2.get("09-resume").unwrap();
        assert_eq!(handle.path, original_path);
        assert_eq!(handle.branch, original_branch);
        assert!(handle.created_at_ms > 0);
        assert!(handle.last_active_ms >= handle.created_at_ms);
    }

    #[tokio::test]
    async fn ensure_for_plan_reattaches_untracked_on_disk_worktree() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        // Create a worktree, then simulate a fresh manager (resume scenario).
        let original = mgr.create_for_plan("09-ensure").await.unwrap();
        let original_path = original.path.clone();

        let mgr2 = WorktreeManager::new(WorktreeConfig {
            repo_root: mgr.config.repo_root.clone(),
            base_branch: "main".to_string(),
            worktrees_root: original_path.parent().unwrap().to_path_buf(),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        assert!(mgr2.get("09-ensure").is_none());

        // ensure_for_plan should reattach instead of trying git worktree add.
        let ensured = mgr2.ensure_for_plan("09-ensure").await.unwrap();
        assert_eq!(ensured.path, original_path);
        assert_eq!(ensured.branch, original.branch);
    }

    #[tokio::test]
    async fn ensure_rejects_stale_snapshot_registry_identity() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let original = mgr.create_for_plan("09-stale-snapshot").await.unwrap();
        let mut snapshot = mgr.snapshot(42);
        snapshot.handles[0].path = original.path.with_file_name("wrong-path");
        let restored = WorktreeManager::from_snapshot((*mgr.config).clone(), snapshot).unwrap();

        let error = restored
            .ensure_for_plan("09-stale-snapshot")
            .await
            .unwrap_err();

        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        assert!(
            original.path.exists(),
            "valid git worktree must be preserved"
        );
    }

    #[tokio::test]
    async fn from_snapshot_rejects_duplicate_registry_ids() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create_for_plan("09-duplicate-snapshot").await.unwrap();
        let mut snapshot = mgr.snapshot(42);
        snapshot.handles.push(snapshot.handles[0].clone());

        let error = WorktreeManager::from_snapshot((*mgr.config).clone(), snapshot).unwrap_err();

        assert!(
            matches!(error, WorktreeError::AlreadyExists(ref id) if id == "09-duplicate-snapshot")
        );
    }

    #[tokio::test]
    async fn concurrent_ensure_for_plan_returns_one_shared_handle() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let first_manager = mgr.clone();
        let second_manager = mgr.clone();

        let (first, second) = tokio::join!(
            first_manager.ensure_for_plan("09-concurrent"),
            second_manager.ensure_for_plan("09-concurrent")
        );

        let first = first.expect("first ensure");
        let second = second.expect("second ensure");
        assert_eq!(first.id, second.id);
        assert_eq!(first.path, second.path);
        assert_eq!(first.branch, second.branch);
        assert_eq!(first.created_at_ms, second.created_at_ms);
        assert_eq!(mgr.active_count(), 1);

        let listed = StdCommand::new("git")
            .current_dir(&mgr.config.repo_root)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .expect("git worktree list");
        assert!(listed.status.success());
        let canonical_path = std::fs::canonicalize(&first.path).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&listed.stdout)
                .lines()
                .filter_map(|line| line.strip_prefix("worktree "))
                .filter_map(|path| std::fs::canonicalize(path).ok())
                .filter(|path| *path == canonical_path)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn concurrent_discover_and_ensure_keep_one_active_handle() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let original = mgr.create_for_plan("09-discover-race").await.unwrap();
        let fresh = WorktreeManager::new(WorktreeConfig {
            repo_root: mgr.config.repo_root.clone(),
            base_branch: "main".to_string(),
            worktrees_root: original.path.parent().unwrap().to_path_buf(),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });
        let ensure_manager = fresh.clone();
        let discover_manager = fresh.clone();

        let (ensured, discovered) = tokio::join!(
            ensure_manager.ensure_for_plan("09-discover-race"),
            discover_manager.discover_existing(&["09-discover-race"])
        );

        let ensured = ensured.expect("ensure should reuse the candidate");
        assert_eq!(ensured.path, original.path);
        assert!(discovered.is_empty() || discovered == ["09-discover-race"]);
        assert_eq!(fresh.active_count(), 1);
        assert_eq!(fresh.get("09-discover-race").unwrap().path, original.path);
    }

    #[tokio::test]
    async fn discover_existing_rejects_wrong_branch() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let original = mgr
            .create("09-wrong-branch", "feature/not-the-plan-branch")
            .await
            .unwrap();
        let fresh = WorktreeManager::new(WorktreeConfig {
            repo_root: mgr.config.repo_root.clone(),
            base_branch: "main".to_string(),
            worktrees_root: original.path.parent().unwrap().to_path_buf(),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });

        assert!(
            fresh
                .discover_existing(&["09-wrong-branch"])
                .await
                .is_empty()
        );
        assert!(fresh.get("09-wrong-branch").is_none());
        let error = fresh.ensure_for_plan("09-wrong-branch").await.unwrap_err();
        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    }

    #[tokio::test]
    async fn discover_existing_rejects_foreign_repository_worktree() {
        let Some((tmp, mgr)) = make_manager() else {
            return;
        };
        let foreign_repo = tmp.path().join("foreign-repo");
        std::fs::create_dir_all(&foreign_repo).unwrap();
        init_repo(&foreign_repo);
        let candidate = mgr.path_for("09-foreign");
        std::fs::create_dir_all(candidate.parent().unwrap()).unwrap();
        let expected_branch = format_branch_name("09-foreign");
        let status = StdCommand::new("git")
            .current_dir(&foreign_repo)
            .args([
                "worktree",
                "add",
                "-b",
                &expected_branch,
                candidate.to_str().unwrap(),
                "main",
            ])
            .status()
            .unwrap();
        assert!(status.success());

        assert!(
            mgr.discover_existing(&["09-foreign"]).await.is_empty(),
            "a canonical-looking worktree from another repository must fail closed"
        );
        assert!(mgr.get("09-foreign").is_none());
        let error = mgr.ensure_for_plan("09-foreign").await.unwrap_err();
        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    }

    #[tokio::test]
    async fn inherited_git_environment_cannot_spoof_reattach_identity() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let id = "09-env-spoof";
        let expected_branch = format_branch_name(id);
        let status = StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["checkout", "-b", &expected_branch])
            .status()
            .unwrap();
        assert!(status.success());
        let candidate = manager.path_for(id);
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::write(candidate.join(".git"), b"not a linked-worktree pointer\n").unwrap();
        manager.set_test_git_probe_environment(vec![
            (
                std::ffi::OsString::from("GIT_DIR"),
                manager.config.repo_root.join(".git").into_os_string(),
            ),
            (
                std::ffi::OsString::from("GIT_WORK_TREE"),
                candidate.clone().into_os_string(),
            ),
        ]);

        assert!(manager.discover_existing(&[id]).await.is_empty());
        assert!(manager.get(id).is_none());
        let error = manager.ensure_for_plan(id).await.unwrap_err();
        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        let listed = StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .args(["worktree", "list", "--porcelain"])
            .output()
            .unwrap();
        assert!(!super::worktree_list_contains_path(
            &listed.stdout,
            &candidate
        ));
    }

    #[tokio::test]
    async fn create_and_health_ignore_command_local_git_environment_spoofing() {
        let Some((tmp, manager)) = make_manager() else {
            return;
        };
        let foreign = tmp.path().join("probe-spoof-repo");
        std::fs::create_dir_all(&foreign).unwrap();
        init_repo(&foreign);
        manager.set_test_git_probe_environment(vec![
            (
                std::ffi::OsString::from("GIT_DIR"),
                foreign.join(".git").into_os_string(),
            ),
            (
                std::ffi::OsString::from("GIT_WORK_TREE"),
                foreign.clone().into_os_string(),
            ),
        ]);

        let handle = manager
            .create("probe-sanitized", "feature/probe-sanitized")
            .await
            .expect("sanitized create");
        let admin_dir = super::read_gitdir(&handle.path).expect("worktree pointer");
        let canonical_admin = std::fs::canonicalize(admin_dir).unwrap();
        let canonical_common = std::fs::canonicalize(manager.config.repo_root.join(".git"))
            .unwrap()
            .join("worktrees");
        assert_eq!(canonical_admin.parent(), Some(canonical_common.as_path()));
        assert_eq!(
            manager.check_health("probe-sanitized").await.unwrap(),
            WorktreeHealth::Ok
        );
    }

    #[tokio::test]
    async fn discover_existing_rejects_nonreciprocal_admin_gitdir_link() {
        let Some((_tmp, manager)) = make_manager() else {
            return;
        };
        let handle = manager.create_for_plan("09-nonreciprocal").await.unwrap();
        let admin_dir = super::read_gitdir(&handle.path).unwrap();
        std::fs::write(
            admin_dir.join("gitdir"),
            format!("{}\n", manager.config.repo_root.join(".git").display()),
        )
        .unwrap();
        let fresh = WorktreeManager::new((*manager.config).clone());

        assert!(
            fresh
                .discover_existing(&["09-nonreciprocal"])
                .await
                .is_empty()
        );
        let error = fresh.ensure_for_plan("09-nonreciprocal").await.unwrap_err();
        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    }

    #[tokio::test]
    async fn ensure_for_plan_rejects_detached_existing_worktree() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let original = mgr.create_for_plan("09-detached").await.unwrap();
        let status = StdCommand::new("git")
            .current_dir(&original.path)
            .args(["checkout", "--detach"])
            .status()
            .unwrap();
        assert!(status.success());
        let tracked_error = mgr.ensure_for_plan("09-detached").await.unwrap_err();
        assert!(matches!(
            tracked_error,
            WorktreeError::ReattachRejected { .. }
        ));
        let fresh = WorktreeManager::new(WorktreeConfig {
            repo_root: mgr.config.repo_root.clone(),
            base_branch: "main".to_string(),
            worktrees_root: original.path.parent().unwrap().to_path_buf(),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });

        let error = fresh.ensure_for_plan("09-detached").await.unwrap_err();

        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        assert!(fresh.get("09-detached").is_none());
    }

    #[tokio::test]
    async fn ensure_for_plan_rejects_missing_worktree_metadata() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let original = mgr.create_for_plan("09-missing-metadata").await.unwrap();
        std::fs::remove_file(original.path.join(".git")).unwrap();
        let fresh = WorktreeManager::new(WorktreeConfig {
            repo_root: mgr.config.repo_root.clone(),
            base_branch: "main".to_string(),
            worktrees_root: original.path.parent().unwrap().to_path_buf(),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        });

        let error = fresh
            .ensure_for_plan("09-missing-metadata")
            .await
            .unwrap_err();

        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        assert!(original.path.exists(), "unsafe candidate must be preserved");
        assert!(fresh.get("09-missing-metadata").is_none());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn ensure_for_plan_rejects_symlink_candidate() {
        let Some((tmp, mgr)) = make_manager() else {
            return;
        };
        let outside = tmp.path().join("outside-worktree");
        let expected_branch = format_branch_name("09-symlink");
        let status = StdCommand::new("git")
            .current_dir(&mgr.config.repo_root)
            .args([
                "worktree",
                "add",
                "-b",
                &expected_branch,
                outside.to_str().unwrap(),
                "main",
            ])
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::create_dir_all(&mgr.config.worktrees_root).unwrap();
        std::os::unix::fs::symlink(&outside, mgr.path_for("09-symlink")).unwrap();

        let error = mgr.ensure_for_plan("09-symlink").await.unwrap_err();

        assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
        assert!(outside.exists(), "target worktree must be preserved");
        assert!(mgr.get("09-symlink").is_none());
    }

    #[tokio::test]
    async fn discover_rejects_invalid_ids_without_creating_paths() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };

        let discovered = mgr.discover_existing(&["../escape", "has/slash"]).await;

        assert!(discovered.is_empty());
        assert_eq!(mgr.active_count(), 0);
        assert!(!mgr.config.worktrees_root.exists());
    }

    #[tokio::test]
    async fn remove_all_clears_every_tracked_worktree() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create_for_plan("07-golf-4").await.unwrap();
        mgr.create_for_plan("07-golf-5").await.unwrap();

        let removed = mgr.remove_all().await.unwrap();
        assert_eq!(
            removed,
            vec!["07-golf-4".to_string(), "07-golf-5".to_string()]
        );
        assert_eq!(mgr.active_count(), 0);
    }

    #[tokio::test]
    async fn budget_exhausted_when_max_live_reached() {
        let Some((_tmp, mgr)) = make_manager_with_budget(1) else {
            return;
        };
        mgr.create("a", "feature/a").await.unwrap();
        let err = mgr.create("b", "feature/b").await.unwrap_err();
        assert!(matches!(err, WorktreeError::BudgetExhausted { max: 1 }));
        // Original still intact.
        assert_eq!(mgr.list().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn concurrent_create_respects_max_live_budget() {
        let Some((_tmp, mgr)) = make_manager_with_budget(1) else {
            return;
        };
        let first_manager = mgr.clone();
        let second_manager = mgr.clone();

        let (first, second) = tokio::join!(
            first_manager.create("budget-a", "feature/budget-a"),
            second_manager.create("budget-b", "feature/budget-b")
        );

        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        let error = first.err().or_else(|| second.err()).expect("one rejection");
        assert!(matches!(error, WorktreeError::BudgetExhausted { max: 1 }));
        assert_eq!(mgr.active_count(), 1);
    }

    #[tokio::test]
    async fn touch_updates_last_active() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let h = mgr.create("08-hotel", "feature/hotel").await.unwrap();
        let before = h.last_active_ms;
        tokio::time::sleep(Duration::from_millis(50)).await;
        mgr.touch("08-hotel");
        let listed = mgr.list().unwrap();
        let after = listed[0].last_active_ms;
        assert!(after > before, "touch should bump last_active_ms");
    }

    #[tokio::test]
    async fn check_health_ok_for_live_worktree() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("09-india", "feature/india").await.unwrap();
        let health = mgr.check_health("09-india").await.unwrap();
        assert_eq!(health, WorktreeHealth::Ok);
    }

    #[tokio::test]
    async fn check_health_missing_when_dir_deleted() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let h = mgr.create("10-juliet", "feature/juliet").await.unwrap();
        std::fs::remove_dir_all(&h.path).unwrap();
        let health = mgr.check_health("10-juliet").await.unwrap();
        assert_eq!(health, WorktreeHealth::Missing);
    }

    #[tokio::test]
    async fn reclaim_idle_evicts_stale_worktrees() {
        if !git_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        init_repo(&repo_root);
        // idle_ttl = 0 so every worktree is immediately reclaimable.
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root,
            base_branch: "main".to_string(),
            worktrees_root: tmp.path().join("worktrees"),
            max_live: None,
            idle_ttl: Duration::ZERO,
        });
        mgr.create("11-kilo", "feature/kilo").await.unwrap();
        mgr.create("12-lima", "feature/lima").await.unwrap();
        // Small sleep so timestamps are in the past.
        tokio::time::sleep(Duration::from_millis(5)).await;
        let evicted = mgr.reclaim_idle().await.unwrap();
        assert_eq!(evicted.len(), 2);
        assert!(mgr.list().unwrap().is_empty());
    }

    #[tokio::test]
    async fn clear_stale_locks_removes_old_lock_files() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("13-mike", "feature/mike").await.unwrap();

        // Plant a stale lock in the git worktrees metadata dir.
        let handle = mgr.get("13-mike").expect("tracked worktree");
        let lock_dir = super::read_gitdir(&handle.path).expect("linked worktree gitdir");
        assert!(
            lock_dir.exists(),
            "git should have created worktree metadata"
        );
        let lock_path = lock_dir.join("index.lock");
        std::fs::write(&lock_path, "").unwrap();

        // Backdate the lock to epoch so it appears stale (> 5 min).
        std::fs::File::options()
            .write(true)
            .open(&lock_path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(std::time::SystemTime::UNIX_EPOCH)
                    .set_modified(std::time::SystemTime::UNIX_EPOCH),
            )
            .unwrap();

        let cleared = mgr.clear_stale_locks().unwrap();
        assert_eq!(cleared.len(), 1, "stale lock should be cleared");
        assert!(!lock_path.exists(), "lock file should be deleted");
    }

    #[tokio::test]
    async fn create_clears_stale_main_repo_lock_before_git() {
        let Some((tmp, mgr)) = make_manager() else {
            return;
        };

        let repo_root = tmp.path().join("repo");
        let lock_path = repo_root.join(".git").join("index.lock");
        std::fs::write(&lock_path, "").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&lock_path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(std::time::SystemTime::UNIX_EPOCH)
                    .set_modified(std::time::SystemTime::UNIX_EPOCH),
            )
            .unwrap();

        let handle = mgr.create("14-november", "feature/november").await.unwrap();
        assert_eq!(handle.id, "14-november");
        assert!(
            !lock_path.exists(),
            "stale main repo lock should be removed"
        );
    }

    #[tokio::test]
    async fn prune_runs_without_error() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let result = mgr.prune().await;
        assert!(result.is_ok());
    }
}
