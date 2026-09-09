//! Worktree cleanup, pruning, stale-lock detection, and health checks.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::creation_journal::{OperationLifecycle, retain_lock_if_cleanup_unproved};
use super::git_ops::{await_owned_operation, is_stale_lock, read_gitdir};
use super::{
    WorktreeError, WorktreeHealth, WorktreeIsolationStatus, WorktreeManager,
    REPOSITORY_MUTATION_LOCK, STUCK_MUTATION_LOCK_AGE_SECS,
};

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
    /// `retain_lock_if_cleanup_unproved` deliberately leaks the
    /// `RepositoryMutationLock` via `std::mem::forget` so the kernel
    /// `flock` stays held for the lifetime of the process. If that
    /// process exits abnormally the kernel releases the flock, but the
    /// lock *file* remains on disk.  Subsequent `acquire_repository_
    /// mutation_lock` calls succeed immediately (the flock is unowned),
    /// yet in long-lived processes where `mem::forget` already ran, all
    /// later worktree operations are permanently blocked.
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
        use std::time::SystemTime;

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
                // Drop the fd first (releases flock), then unlink the path.
                drop(lock_fd);
                if let Err(e) = std::fs::remove_file(&lock_path) {
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
}
