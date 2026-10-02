//! Worktree cleanup, pruning, stale-lock detection, and health checks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Serialize;

use super::acceptance::CHECKOUT_RUN_FILE;
use super::creation_journal::{OperationLifecycle, retain_lock_if_cleanup_unproved};
use super::git_ops::{await_owned_operation, is_stale_lock, read_gitdir};
use super::{
    REPOSITORY_MUTATION_LOCK, STUCK_MUTATION_LOCK_AGE_SECS, WorktreeError, WorktreeHealth,
    WorktreeIsolationStatus, WorktreeManager,
};
use crate::graph_checkpoint::{GraphCheckpointStatus, recorded_checkpoint_runs};

impl WorktreeManager {
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

        // After evicting stale worktrees, run `git worktree prune` to clean up
        // git's internal metadata for worktrees whose on-disk directories no
        // longer exist (G01 audit).  Best-effort: failures are logged but never
        // block the caller.
        if let Err(err) = self.prune().await {
            tracing::warn!(error = %err, "git worktree prune during reclaim_idle failed (non-fatal)");
        }

        Ok(removed)
    }

    /// Remove a stuck repository mutation lock file left behind by a
    /// crashed or killed process.
    ///
    /// After an unproved cleanup, `retain_lock_if_cleanup_unproved` keeps
    /// the `RepositoryMutationLock`, so the kernel `flock` stays held until
    /// the manager proves the git processes gone (bug-53475e). If the
    /// process exits abnormally the kernel releases the flock, but the lock
    /// *file* remains on disk. Later `acquire_repository_mutation_lock`
    /// calls succeed immediately (the flock is unowned), so the file only
    /// costs disk hygiene; it never blocks anything.
    ///
    /// This helper performs a non-blocking exclusive `flock` attempt:
    ///
    /// * **flock succeeds** -- no other process holds the lock. If the
    ///   file is also older than [`STUCK_MUTATION_LOCK_AGE_SECS`], it
    ///   is removed. The age guard prevents racing with a concurrent
    ///   process that just created the file but has not yet acquired
    ///   the flock.
    /// * **flock fails (EWOULDBLOCK)** -- another live process still
    ///   owns the lock. The file is left alone.
    ///
    /// Returns `Ok(true)` when a stuck lock was removed, `Ok(false)`
    /// when no action was taken. Any I/O error is surfaced so callers
    /// can log and continue.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn clear_stuck_mutation_lock(&self) -> Result<bool, WorktreeError> {
        use super::creation_journal::resolve_repository_identity;

        // Resolve the canonical Git common directory so the lock path
        // matches the one used by `acquire_repository_mutation_lock`.
        let repo_root_fd = rustix::fs::open(
            &self.config.repo_root,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        let identity = resolve_repository_identity(&repo_root_fd, &self.config.repo_root)?;
        let lock_path = identity.canonical_common_dir.join(REPOSITORY_MUTATION_LOCK);

        // If the file does not exist there is nothing to clean up.
        let metadata = match std::fs::metadata(&lock_path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(WorktreeError::IoError(e)),
        };

        // Age guard: only consider files older than the threshold so we
        // never race with a concurrent process that is still starting up.
        let age = metadata
            .modified()
            .ok()
            .and_then(|mtime| SystemTime::now().duration_since(mtime).ok())
            .unwrap_or(Duration::ZERO);
        if age.as_secs() < STUCK_MUTATION_LOCK_AGE_SECS {
            return Ok(false);
        }

        // Try a non-blocking exclusive flock.  If another process still
        // holds the lock the call returns EWOULDBLOCK and we leave the
        // file alone.
        let lock_fd = rustix::fs::open(
            &lock_path,
            rustix::fs::OFlags::RDWR | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;

        match rustix::fs::flock(
            &lock_fd,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        ) {
            Ok(()) => {
                // We own the lock now. The file is old and unowned -- remove it.
                // Unlink the path while still holding the flock, then drop the
                // fd: a process that opened the old file and takes its flock
                // afterwards then fails its binding check instead of mutating
                // beside a holder of the new file.
                let removed = std::fs::remove_file(&lock_path);
                drop(lock_fd);
                if let Err(e) = removed {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Err(WorktreeError::IoError(e));
                    }
                }
                tracing::warn!(
                    path = %lock_path.display(),
                    age_secs = age.as_secs(),
                    "removed stuck worktree mutation lock (no owning process)"
                );
                Ok(true)
            }
            Err(rustix::io::Errno::WOULDBLOCK) => {
                // Another process legitimately holds the lock.
                Ok(false)
            }
            Err(e) => Err(WorktreeError::IoError(std::io::Error::from(e))),
        }
    }

    /// Non-Unix stub: mutation lock cleanup is not supported on this platform.
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub fn clear_stuck_mutation_lock(&self) -> Result<bool, WorktreeError> {
        Ok(false)
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

    pub(super) fn clear_stale_locks_unlocked(&self) -> Result<Vec<PathBuf>, WorktreeError> {
        let mut cleared = Vec::new();
        // The repository root may itself be a linked worktree or a
        // submodule, whose `.git` is a `gitdir:` file (bug-109b5a).
        let Some(git_dir) = read_gitdir(&self.config.repo_root) else {
            return Ok(cleared);
        };

        // Main repo lock.
        let main_lock = git_dir.join("index.lock");
        if main_lock.exists()
            && is_stale_lock(&main_lock)
            && std::fs::remove_file(&main_lock).is_ok()
        {
            cleared.push(main_lock);
        }

        // Per-worktree locks stored under <common dir>/worktrees/<name>/index.lock.
        let wt_meta_dir = common_git_dir(&git_dir).join("worktrees");
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
}

/// How long a leftover attempt checkout must sit untouched before
/// `roko doctor disk --fix` may remove it (gap-f67a72).
pub const LEFTOVER_CHECKOUT_MIN_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// What [`WorktreeManager::remove_leftover_checkouts`] did with one checkout
/// under the worktrees root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeftoverCheckout {
    /// The checkout directory.
    pub path: PathBuf,
    /// Why it was kept; `None` when it was removed.
    pub kept: Option<String>,
}

impl WorktreeManager {
    /// Remove the leftover attempt checkouts under the worktrees root that no
    /// run will use again, for `roko doctor disk --fix` (gap-f67a72). A
    /// checkout goes only when all of these hold, and is kept, with the
    /// reason, otherwise:
    ///
    /// - its `roko-run` record names the run that made it, and a plan's Graph
    ///   checkpoint, current or archived, names that run;
    /// - the current checkpoint of each such plan succeeded, failed or was
    ///   cancelled. A running, interrupted or unverified run can resume, and
    ///   a resumed run re-attaches the checkouts it made;
    /// - nothing touched it for `min_age`;
    /// - it has no uncommitted changes, and no `git worktree lock` holds it.
    ///
    /// Branches are kept. The caller holds the runner lock, so no run of the
    /// workspace is live.
    pub async fn remove_leftover_checkouts(&self, min_age: Duration) -> Vec<LeftoverCheckout> {
        let runs = CheckpointRuns::read(&self.config.repo_root);
        let mut outcomes = Vec::new();
        for path in checkout_dirs(&self.config.worktrees_root) {
            let mut kept = leftover_retention(&path, &runs, min_age);
            if kept.is_none() {
                kept = match self.remove_leftover(&path).await {
                    Ok(()) => None,
                    Err(WorktreeError::DirtyWorktree { .. }) => {
                        Some("it has uncommitted changes".to_string())
                    }
                    Err(error) => Some(format!("removing it failed: {error}")),
                };
            }
            outcomes.push(LeftoverCheckout { path, kept });
        }
        outcomes
    }

    /// `git worktree remove` the checkout at `path`, which this manager does
    /// not track. Like [`WorktreeManager::remove`], it keeps a checkout that
    /// has changes; unlike it, it passes no `--force`, so git also keeps one
    /// that a `git worktree lock` holds. The branch stays.
    async fn remove_leftover(&self, path: &Path) -> Result<(), WorktreeError> {
        let operation = Arc::clone(&self.operations).lock_owned().await;
        let manager = self.clone();
        let path = path.to_path_buf();
        await_owned_operation(operation, move |lifecycle| async move {
            let repository_lock = manager.acquire_repository_mutation_lock()?;
            let result = manager.remove_leftover_locked(&path, &lifecycle).await;
            retain_lock_if_cleanup_unproved(repository_lock, &lifecycle);
            result
        })
        .await
    }

    async fn remove_leftover_locked(
        &self,
        path: &Path,
        lifecycle: &OperationLifecycle,
    ) -> Result<(), WorktreeError> {
        self.validate_git_policy(false).await?;
        let args = ["status", "--porcelain", "--untracked-files=all"];
        let probe = self.git_probe_output_at(path, &args).await?;
        if !probe.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&probe.stderr).trim().to_string(),
            });
        }
        if !probe.stdout.trim_ascii().is_empty() {
            return Err(WorktreeError::DirtyWorktree {
                id: path.display().to_string(),
                paths: String::from_utf8_lossy(&probe.stdout).trim().to_string(),
            });
        }
        let path = path.to_string_lossy().into_owned();
        let output = self
            .git_mutation_output(&["worktree", "remove", &path], lifecycle)
            .await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }
}

/// What the Graph checkpoints under the workspace's `.roko/state/graph/` say
/// about each run, for [`WorktreeManager::remove_leftover_checkouts`].
#[derive(Debug, Default)]
struct CheckpointRuns {
    /// The plans whose checkpoints, current or archived, name each run.
    plans: HashMap<String, Vec<String>>,
    /// The status of each plan's current checkpoint.
    current: HashMap<String, GraphCheckpointStatus>,
}

impl CheckpointRuns {
    fn read(workdir: &Path) -> Self {
        let mut runs = Self::default();
        for recorded in recorded_checkpoint_runs(workdir) {
            let plan = recorded.plan_dir;
            if recorded.current {
                runs.current.insert(plan.clone(), recorded.status);
            }
            let plans = runs.plans.entry(recorded.run_id).or_default();
            if !plans.contains(&plan) {
                plans.push(plan);
            }
        }
        runs
    }
}

/// Why the leftover checkout at `path` must stay, or `None` when it may go,
/// by the rule of [`WorktreeManager::remove_leftover_checkouts`] save its
/// last condition, which the removal itself checks.
fn leftover_retention(path: &Path, runs: &CheckpointRuns, min_age: Duration) -> Option<String> {
    let Some(admin) = read_gitdir(path) else {
        return Some("it is not a git checkout".to_string());
    };
    let run = std::fs::read_to_string(admin.join(CHECKOUT_RUN_FILE)).unwrap_or_default();
    let run = run.trim();
    if run.is_empty() {
        return Some("no run is recorded for it".to_string());
    }
    let Some(plans) = runs.plans.get(run) else {
        return Some(format!("no plan checkpoint names its run {run}"));
    };
    for plan in plans {
        match runs.current.get(plan) {
            Some(status) if run_has_ended(*status) => {}
            Some(status) => {
                return Some(format!("plan {plan}'s checkpoint is {}", status.as_str()));
            }
            None => return Some(format!("plan {plan} has no current checkpoint")),
        }
    }
    let (index, head) = (admin.join("index"), admin.join("HEAD"));
    let candidates: [&Path; 4] = [path, &admin, &index, &head];
    let Some(touched) = candidates.into_iter().filter_map(modified).max() else {
        return Some("when it was last touched is unknown".to_string());
    };
    let age = touched.elapsed().unwrap_or_default();
    if age < min_age {
        return Some(format!(
            "it was touched {} day(s) ago",
            age.as_secs() / 86_400
        ));
    }
    None
}

/// Whether a run whose checkpoint reads `status` has ended for good: it
/// succeeded, failed or was cancelled. A running, interrupted or unverified
/// run can resume.
const fn run_has_ended(status: GraphCheckpointStatus) -> bool {
    matches!(
        status,
        GraphCheckpointStatus::Succeeded
            | GraphCheckpointStatus::Failed
            | GraphCheckpointStatus::Cancelled
    )
}

/// The checkout directories under `root`, sorted: its subdirectories, but
/// not symlinks, nor the dot-directories in which the manager keeps its own
/// records, such as its creation markers.
fn checkout_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| entry.path())
        .collect();
    dirs.sort();
    dirs
}

/// When `path` was last modified, if that can be read.
fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// Remove a stale `index.lock` from the git directory that serves `workdir`,
/// following a `.git` file's `gitdir:` indirection (linked worktrees,
/// submodules), so that the next agent dispatched there can use git
/// (bug-109b5a). A git process killed mid-command leaves the lock behind, and
/// every index-writing git command then fails.
///
/// A lock younger than `stale_after`, and never one younger than
/// `STALE_LOCK_SECS` (60 s), may belong to a live git process and is left
/// alone. Returns the lock it removed.
pub fn clear_stale_index_lock(workdir: &Path, stale_after: Duration) -> Option<PathBuf> {
    let git_dir = workdir.ancestors().find_map(read_gitdir)?;
    let lock = git_dir.join("index.lock");
    let age = std::fs::symlink_metadata(&lock)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())?;
    if age < stale_after.max(Duration::from_secs(super::STALE_LOCK_SECS)) {
        tracing::debug!(
            path = %lock.display(),
            age_secs = age.as_secs(),
            "leaving a git index.lock that a live git process may hold"
        );
        return None;
    }
    match std::fs::remove_file(&lock) {
        Ok(()) => {
            tracing::warn!(
                path = %lock.display(),
                age_secs = age.as_secs(),
                "removed a stale git index.lock left by an earlier git process"
            );
            Some(lock)
        }
        Err(error) => {
            tracing::warn!(
                path = %lock.display(),
                %error,
                "could not remove a stale git index.lock"
            );
            None
        }
    }
}

/// The common git directory behind `git_dir`: a linked worktree's git
/// directory names it in its `commondir` file; otherwise it is `git_dir`.
fn common_git_dir(git_dir: &Path) -> PathBuf {
    match std::fs::read_to_string(git_dir.join("commondir")) {
        Ok(common) => git_dir.join(common.trim()),
        Err(_) => git_dir.to_path_buf(),
    }
}
