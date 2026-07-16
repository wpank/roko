//! Worktree manager — wraps `git worktree add/remove/list/prune` so the
//! orchestrator can give each live plan its own isolated working
//! directory (parity §15).
//!
//! The manager shells out to the `git` binary via [`tokio::process::Command`]
//! and tracks active worktrees in an in-memory map guarded by a
//! [`parking_lot::Mutex`]. All git operations are fallible and surface as
//! [`WorktreeError`].
//!
//! ## Shipped features
//!
//! - §15.1–§15.2 Create / remove worktrees
//! - §15.3 Ephemeral branch naming ([`format_branch_name`])
//! - §15.4 Extended config: `max_live` budget, `idle_ttl`
//! - §15.5 Health checks ([`WorktreeHealth`])
//! - §15.6 Budget enforcement + idle reclamation
//! - §15.7 Stale lock detection ([`WorktreeManager::clear_stale_locks`])
//! - §15.8 Force-remove (via [`WorktreeManager::remove`])
//! - §15.9 Prune stale git metadata ([`WorktreeManager::prune`])

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;
use tracing::debug;

/// Locks older than this are considered stale (§15.7).
///
/// Sourced from [`roko_core::defaults::DEFAULT_STALE_LOCK_SECS`].
const STALE_LOCK_SECS: u64 = roko_core::defaults::DEFAULT_STALE_LOCK_SECS;

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
    /// Local filesystem error while preparing or removing a worktree.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
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

/// Derive the canonical branch name for a task attempt.
///
/// Convention: `roko/task/<plan_id>/<task_id>`. Each task gets its own
/// branch forked from the plan branch so diffs are attributable and
/// gates run against an isolated snapshot.
#[must_use]
pub fn format_task_branch_name(plan_id: &str, task_id: &str) -> String {
    format!("roko/task/{plan_id}/{task_id}")
}

/// Manages the lifecycle of per-plan git worktrees.
///
/// Clones of [`WorktreeManager`] share the same internal registry — the
/// handle is cheap to clone and safe to move across tasks.
#[derive(Clone, Debug)]
pub struct WorktreeManager {
    config: Arc<WorktreeConfig>,
    active: Arc<Mutex<HashMap<String, WorktreeHandle>>>,
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
    /// invalid id.
    pub fn from_snapshot(
        config: WorktreeConfig,
        snapshot: WorktreeSnapshot,
    ) -> Result<Self, WorktreeError> {
        let mut active = HashMap::with_capacity(snapshot.handles.len());
        for handle in snapshot.handles {
            validate_id(&handle.id)?;
            active.insert(handle.id.clone(), handle);
        }

        Ok(Self {
            config: Arc::new(config),
            active: Arc::new(Mutex::new(active)),
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
    /// be exceeded, [`WorktreeError::GitFailed`] if `git worktree add`
    /// exits unsuccessfully, or [`WorktreeError::IoError`] if preparing the
    /// worktree directory fails.
    pub async fn create(&self, id: &str, branch: &str) -> Result<WorktreeHandle, WorktreeError> {
        validate_id(id)?;
        let _ = self.clear_stale_locks();

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

        // Make sure `worktrees_root` exists before git tries to write.
        tokio::fs::create_dir_all(&self.config.worktrees_root).await?;

        let path_str = path.to_string_lossy().into_owned();

        // `git worktree add -B <branch> <path> <base>` creates or resets
        // the branch to match the base, then checks it out into a new
        // worktree. Using `-B` (vs `-b`) keeps the call idempotent at
        // the git level if a stray branch is lying around.
        let output = Command::new("git")
            .current_dir(&self.config.repo_root)
            .args([
                "worktree",
                "add",
                "-B",
                branch,
                &path_str,
                &self.config.base_branch,
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
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
            let _ = self.git_remove(&handle.path).await;
            return Err(WorktreeError::AlreadyExists(id.to_string()));
        }

        Ok(handle)
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
    /// Returns [`WorktreeError::NotFound`] if the tracked handle is
    /// removed between the initial lookup and the final fetch, or any
    /// [`WorktreeError`] that can be produced by
    /// [`WorktreeManager::create_for_plan`].
    pub async fn ensure_for_plan(&self, plan_id: &str) -> Result<WorktreeHandle, WorktreeError> {
        {
            let mut guard = self.active.lock();
            if let Some(handle) = guard.get_mut(plan_id) {
                handle.last_active_ms = chrono::Utc::now().timestamp_millis();
                return Ok(handle.clone());
            }
        }

        // Safety net: if the worktree exists on disk but wasn't tracked
        // (e.g. resume without discover_existing), re-register it instead
        // of trying `git worktree add` which would fail.
        if let Some(handle) = self.try_reattach(plan_id).await {
            return Ok(handle);
        }

        self.create_for_plan(plan_id).await
    }

    /// Scan the worktrees root for directories matching `plan_ids` that
    /// exist on disk but are not yet tracked. Valid worktrees are
    /// re-registered in the in-memory map.
    ///
    /// Returns the list of plan IDs that were successfully re-discovered.
    pub async fn discover_existing(&self, plan_ids: &[&str]) -> Vec<String> {
        let mut discovered = Vec::new();
        for &plan_id in plan_ids {
            // Already tracked — nothing to do.
            if self.get(plan_id).is_some() {
                continue;
            }
            if let Some(_handle) = self.try_reattach(plan_id).await {
                discovered.push(plan_id.to_string());
            }
        }
        discovered
    }

    /// Return the active worktree path for `plan_id` if tracked.
    #[must_use]
    pub fn plan_path(&self, plan_id: &str) -> Option<PathBuf> {
        self.get(plan_id).map(|h| h.path)
    }

    /// Create (or reset) a task branch in the plan's worktree and check it
    /// out.  The task branch forks from the current plan branch HEAD so the
    /// agent works on an isolated snapshot.  Returns the worktree path.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::NotFound`] if the plan worktree is not
    /// tracked, or [`WorktreeError::GitFailed`] if the git checkout fails.
    pub async fn checkout_task_branch(
        &self,
        plan_id: &str,
        task_id: &str,
    ) -> Result<PathBuf, WorktreeError> {
        let handle = self
            .get(plan_id)
            .ok_or_else(|| WorktreeError::NotFound(plan_id.to_string()))?;
        let task_branch = format_task_branch_name(plan_id, task_id);
        let plan_branch = format_branch_name(plan_id);
        // Create or reset the task branch from the plan branch tip, then
        // check it out in the existing plan worktree.
        let output = Command::new("git")
            .current_dir(&handle.path)
            .args(["checkout", "-B", &task_branch, &plan_branch])
            .output()
            .await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        self.touch(plan_id);
        Ok(handle.path)
    }

    /// Merge the task branch into the plan branch using `--no-ff` and
    /// return to the plan branch.  This makes the task's diff attributable
    /// as a single merge commit on the plan branch.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::NotFound`] if the plan worktree is not
    /// tracked, or [`WorktreeError::GitFailed`] if git merge/checkout fails.
    pub async fn merge_task_into_plan(
        &self,
        plan_id: &str,
        task_id: &str,
    ) -> Result<(), WorktreeError> {
        let handle = self
            .get(plan_id)
            .ok_or_else(|| WorktreeError::NotFound(plan_id.to_string()))?;
        let task_branch = format_task_branch_name(plan_id, task_id);
        let plan_branch = format_branch_name(plan_id);

        // Switch back to the plan branch.
        let checkout = Command::new("git")
            .current_dir(&handle.path)
            .args(["checkout", &plan_branch])
            .output()
            .await?;
        if !checkout.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&checkout.stderr).into_owned(),
            });
        }

        // Merge the task branch with --no-ff so the merge commit is visible.
        let merge = Command::new("git")
            .current_dir(&handle.path)
            .args([
                "merge",
                "--no-ff",
                "--no-edit",
                "-m",
                &format!("[roko] merge task {task_id} into {plan_id}"),
                &task_branch,
            ])
            .output()
            .await?;
        if !merge.status.success() {
            // Abort the failed merge so the worktree stays clean.
            let _ = Command::new("git")
                .current_dir(&handle.path)
                .args(["merge", "--abort"])
                .output()
                .await;
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&merge.stderr).into_owned(),
            });
        }

        // Clean up the task branch ref — it's been merged.
        let _ = Command::new("git")
            .current_dir(&handle.path)
            .args(["branch", "-d", &task_branch])
            .output()
            .await;

        self.touch(plan_id);
        Ok(())
    }

    /// Discard the task branch and return the plan worktree to the plan
    /// branch.  Used when a task fails gates or the agent errors out.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::NotFound`] if the plan worktree is not
    /// tracked, or [`WorktreeError::GitFailed`] if git operations fail.
    pub async fn discard_task_branch(
        &self,
        plan_id: &str,
        task_id: &str,
    ) -> Result<(), WorktreeError> {
        let handle = self
            .get(plan_id)
            .ok_or_else(|| WorktreeError::NotFound(plan_id.to_string()))?;
        let task_branch = format_task_branch_name(plan_id, task_id);
        let plan_branch = format_branch_name(plan_id);

        // Force-checkout the plan branch (discards any uncommitted task work).
        let checkout = Command::new("git")
            .current_dir(&handle.path)
            .args(["checkout", "-f", &plan_branch])
            .output()
            .await?;
        if !checkout.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&checkout.stderr).into_owned(),
            });
        }

        // Delete the task branch.
        let _ = Command::new("git")
            .current_dir(&handle.path)
            .args(["branch", "-D", &task_branch])
            .output()
            .await;

        self.touch(plan_id);
        Ok(())
    }

    /// Remove the worktree tracked under `id`. Errors if `id` isn't
    /// tracked. The underlying git directory is removed via
    /// `git worktree remove --force` so uncommitted files are cleaned up
    /// along with the metadata.
    ///
    /// # Errors
    ///
    /// Returns [`WorktreeError::NotFound`] if the id is not tracked,
    /// [`WorktreeError::GitFailed`] if `git worktree remove` exits
    /// unsuccessfully, or [`WorktreeError::IoError`] if invoking `git`
    /// fails.
    pub async fn remove(&self, id: &str) -> Result<(), WorktreeError> {
        let _ = self.clear_stale_locks();
        let handle = {
            let mut guard = self.active.lock();
            guard
                .remove(id)
                .ok_or_else(|| WorktreeError::NotFound(id.to_string()))?
        };

        if let Err(err) = self.git_remove(&handle.path).await {
            // Restore the registry entry so callers can retry.
            self.active.lock().insert(id.to_string(), handle);
            return Err(err);
        }

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

        // Verify the expected branch is checked out.
        let output = Command::new("git")
            .current_dir(&handle.path)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
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
        let _ = self.clear_stale_locks();
        let output = Command::new("git")
            .current_dir(&self.config.repo_root)
            .args(["worktree", "prune"])
            .output()
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
    async fn try_reattach(&self, plan_id: &str) -> Option<WorktreeHandle> {
        if let Err(e) = validate_id(plan_id) {
            debug!(plan_id, error = %e, "skipping reattach for invalid plan id");
            return None;
        }

        let path = self.path_for(plan_id);
        // A git worktree has a `.git` *file* (not directory) pointing
        // back to the main repo's worktree metadata.
        if !path.join(".git").exists() {
            return None;
        }

        let candidate_common_dir = git_common_dir(&path).await?;
        let configured_common_dir = git_common_dir(&self.config.repo_root).await?;
        if candidate_common_dir != configured_common_dir {
            return None;
        }

        let branch = git_stdout(&path, &["symbolic-ref", "--quiet", "--short", "HEAD"]).await?;
        let expected_branch = format_branch_name(plan_id);
        if branch != expected_branch {
            return None;
        }

        let head = git_stdout(&path, &["rev-parse", "HEAD"]).await?;
        let branch_head = git_stdout(&path, &["rev-parse", &expected_branch]).await?;
        if head != branch_head {
            return None;
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
            created_at_ms: mtime_ms,
            last_active_ms: now_ms,
        };

        let mut guard = self.active.lock();
        // Double-check: another caller may have inserted concurrently.
        if guard.contains_key(plan_id) {
            return guard.get(plan_id).cloned();
        }
        guard.insert(plan_id.to_string(), handle.clone());
        Some(handle)
    }

    async fn git_remove(&self, path: &Path) -> Result<(), WorktreeError> {
        let path_str = path.to_string_lossy().into_owned();
        let output = Command::new("git")
            .current_dir(&self.config.repo_root)
            .args(["worktree", "remove", "--force", &path_str])
            .output()
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
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

async fn git_stdout(current_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(current_dir)
        .args(args)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

async fn git_common_dir(worktree: &Path) -> Option<PathBuf> {
    let common_dir = git_stdout(worktree, &["rev-parse", "--git-common-dir"]).await?;
    let path = PathBuf::from(common_dir);
    let absolute = if path.is_absolute() {
        path
    } else {
        worktree.join(path)
    };
    std::fs::canonicalize(absolute).ok()
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

#[cfg(test)]
mod tests {
    //! Tests below each spin up a throwaway git repo in a
    //! [`tempfile::TempDir`]. Every test that needs `git` first checks
    //! the binary via [`git_available`]; if git isn't on `$PATH`, the
    //! test returns early so `cargo test` still succeeds on machines
    //! without git (for instance minimal CI images).

    #![allow(clippy::unwrap_used)]

    use super::{
        WorktreeConfig, WorktreeError, WorktreeHealth, WorktreeManager, format_branch_name,
        validate_id,
    };
    use std::path::Path;
    use std::process::Command as StdCommand;
    use std::time::Duration;
    use tempfile::TempDir;

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
        let Some((tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("13-mike", "feature/mike").await.unwrap();

        // Plant a stale lock in the git worktrees metadata dir.
        let repo_root = tmp.path().join("repo");
        let lock_dir = repo_root.join(".git").join("worktrees").join("13-mike");
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
        assert!(cleared.contains(&lock_path), "stale lock should be cleared");
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

    #[tokio::test]
    async fn task_branch_lifecycle_checkout_merge_discard() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };

        // Create a plan worktree first.
        let handle = mgr.create_for_plan("plan-a").await.unwrap();
        let wt = &handle.path;

        // Write a file on the plan branch so it has content.
        std::fs::write(wt.join("base.txt"), "plan baseline").unwrap();
        let status = StdCommand::new("git")
            .current_dir(wt)
            .args(["add", "base.txt"])
            .status()
            .unwrap();
        assert!(status.success());
        let status = StdCommand::new("git")
            .current_dir(wt)
            .args(["commit", "-m", "plan baseline", "--no-verify"])
            .status()
            .unwrap();
        assert!(status.success());

        // Checkout a task branch — should fork from plan HEAD.
        let path = mgr.checkout_task_branch("plan-a", "task-1").await.unwrap();
        assert_eq!(path, *wt);
        let branch = git_stdout_sync(wt, &["symbolic-ref", "--short", "HEAD"]);
        assert_eq!(branch, "roko/task/plan-a/task-1");

        // Make a change on the task branch.
        std::fs::write(wt.join("task.txt"), "task output").unwrap();
        let status = StdCommand::new("git")
            .current_dir(wt)
            .args(["add", "task.txt"])
            .status()
            .unwrap();
        assert!(status.success());
        let status = StdCommand::new("git")
            .current_dir(wt)
            .args(["commit", "-m", "task work", "--no-verify"])
            .status()
            .unwrap();
        assert!(status.success());

        // Merge task → plan.
        mgr.merge_task_into_plan("plan-a", "task-1").await.unwrap();
        let branch = git_stdout_sync(wt, &["symbolic-ref", "--short", "HEAD"]);
        assert_eq!(branch, "roko/plan/plan-a");
        // Task file should be visible on the plan branch.
        assert!(wt.join("task.txt").exists());

        // Now test discard: create another task branch, change, then discard.
        mgr.checkout_task_branch("plan-a", "task-2").await.unwrap();
        std::fs::write(wt.join("discard.txt"), "should be gone").unwrap();
        let status = StdCommand::new("git")
            .current_dir(wt)
            .args(["add", "discard.txt"])
            .status()
            .unwrap();
        assert!(status.success());
        let status = StdCommand::new("git")
            .current_dir(wt)
            .args(["commit", "-m", "bad work", "--no-verify"])
            .status()
            .unwrap();
        assert!(status.success());

        // Discard — should go back to plan branch, task file gone.
        mgr.discard_task_branch("plan-a", "task-2").await.unwrap();
        let branch = git_stdout_sync(wt, &["symbolic-ref", "--short", "HEAD"]);
        assert_eq!(branch, "roko/plan/plan-a");
        assert!(
            !wt.join("discard.txt").exists(),
            "discarded file should not exist on plan branch"
        );
    }

    fn git_stdout_sync(dir: &Path, args: &[&str]) -> String {
        let output = StdCommand::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {:?} failed", args);
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}
