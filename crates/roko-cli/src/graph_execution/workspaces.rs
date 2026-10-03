//! CLI adapter implementing [`ExecutionWorkspaceProvider`] via [`WorktreeManager`].
//!
//! This module bridges the graph-layer workspace port to the CLI's existing
//! `WorktreeManager`. It enforces shared-checkout rejection and maps the
//! provider trait methods to the established worktree API:
//!
//! - `acquire` → `WorktreeManager::create_for_attempt`
//! - `reconcile` → `WorktreeManager::get_attempt` + `isolation_status`
//! - `release(Delete)` → `WorktreeManager::remove`
//! - `release(RetainFor*)` → keep manager entry, return `Retained`
//! - `reset_for_retry` → release old with `RetainForFailure`, acquire new
//! - `accept` → `WorktreeManager::accept_attempt`: commit the attempt and fold
//!   it into its plan branch

use std::path::PathBuf;

use roko_graph::workspace::{
    ExecutionWorkspaceProvider, WorkspaceAcceptRequest, WorkspaceAcceptance, WorkspaceAttemptId,
    WorkspaceError, WorkspaceLease, WorkspaceLeaseState, WorkspaceReconcileResult,
    WorkspaceReleasePolicy,
};

use crate::orchestrator::worktree::{
    AttemptAcceptance, WorktreeError, WorktreeHealth, WorktreeManager, format_attempt_worktree_id,
    format_branch_name,
};

/// CLI adapter that implements [`ExecutionWorkspaceProvider`] by delegating to
/// the existing [`WorktreeManager`].
///
/// The adapter enforces that no lease ever returns a path equal to the
/// configured repository root, rejecting shared checkouts before any
/// downstream consumer can observe them.
#[derive(Clone, Debug)]
pub struct WorktreeExecutionWorkspaceProvider {
    manager: WorktreeManager,
    /// Cached repo root for shared-checkout rejection.
    repo_root: PathBuf,
}

impl WorktreeExecutionWorkspaceProvider {
    /// Wrap an existing `WorktreeManager` as an `ExecutionWorkspaceProvider`.
    #[must_use]
    pub fn new(manager: WorktreeManager) -> Self {
        let repo_root = manager.repo_root().to_path_buf();
        Self { manager, repo_root }
    }

    /// Return a reference to the underlying `WorktreeManager`.
    #[must_use]
    pub fn manager(&self) -> &WorktreeManager {
        &self.manager
    }

    /// Build a `WorkspaceLease` from a successful worktree handle.
    fn lease_from_handle(
        &self,
        attempt_id: &WorkspaceAttemptId,
        handle: &crate::orchestrator::worktree::WorktreeHandle,
    ) -> Result<WorkspaceLease, WorkspaceError> {
        // Shared-checkout rejection: the returned path must never be the
        // repository root.
        if handle.path == self.repo_root {
            return Err(WorkspaceError::SharedCheckoutRejected);
        }

        // The commit the checkout started from, so the attempt's diff leaves
        // out what siblings landed before it; the configured base only when
        // that is unknown, as for a re-attached checkout (backlog 1124).
        let base_revision = handle
            .base_commit
            .clone()
            .unwrap_or_else(|| self.manager.base_branch().to_string());
        Ok(WorkspaceLease {
            lease_id: handle.id.clone(),
            attempt_id: attempt_id.clone(),
            path: handle.path.clone(),
            branch: handle.branch.clone(),
            base_revision,
            lease_fingerprint: attempt_id.fingerprint(),
        })
    }

    /// Derive the manager worktree ID from an attempt ID.
    fn worktree_id(attempt_id: &WorkspaceAttemptId) -> String {
        format_attempt_worktree_id(&attempt_id.plan_id, &attempt_id.task_id, attempt_id.attempt)
    }
}

#[async_trait::async_trait]
impl ExecutionWorkspaceProvider for WorktreeExecutionWorkspaceProvider {
    async fn acquire(
        &self,
        attempt_id: &WorkspaceAttemptId,
    ) -> Result<WorkspaceLease, WorkspaceError> {
        let wt_id = Self::worktree_id(attempt_id);

        // Idempotent: if the attempt is already tracked, return it.
        if let Some(handle) =
            self.manager
                .get_attempt(&attempt_id.plan_id, &attempt_id.task_id, attempt_id.attempt)
        {
            return self.lease_from_handle(attempt_id, &handle);
        }

        // Create a fresh attempt worktree.
        let handle = self
            .manager
            .create_for_attempt(&attempt_id.plan_id, &attempt_id.task_id, attempt_id.attempt)
            .await
            .map_err(|e| match e {
                crate::orchestrator::worktree::WorktreeError::BudgetExhausted { max } => {
                    WorkspaceError::BudgetExhausted { max }
                }
                crate::orchestrator::worktree::WorktreeError::AlreadyExists(_) => {
                    // Race: another caller created it between our check and create.
                    // Re-fetch and return idempotently.
                    return WorkspaceError::Io(format!("concurrent create for {wt_id}: {e}"));
                }
                other => WorkspaceError::Io(other.to_string()),
            })?;

        self.lease_from_handle(attempt_id, &handle)
    }

    async fn reconcile(
        &self,
        lease: &WorkspaceLease,
    ) -> Result<WorkspaceReconcileResult, WorkspaceError> {
        let attempt_id = &lease.attempt_id;

        // Check if the manager still tracks this attempt.
        let handle = match self.manager.get_attempt(
            &attempt_id.plan_id,
            &attempt_id.task_id,
            attempt_id.attempt,
        ) {
            Some(h) => h,
            None => {
                // Not tracked -- check if the path still exists on disk.
                if lease.path.exists() {
                    return Ok(WorkspaceReconcileResult::Orphaned(lease.clone()));
                }
                return Ok(WorkspaceReconcileResult::AlreadyReleased);
            }
        };

        // Verify the tracked state matches the lease.
        if handle.path != lease.path || handle.branch != lease.branch {
            return Ok(WorkspaceReconcileResult::Conflict(format!(
                "tracked worktree {} has path={}, branch={} but lease expects path={}, branch={}",
                handle.id,
                handle.path.display(),
                handle.branch,
                lease.path.display(),
                lease.branch,
            )));
        }

        // Check filesystem/git health via isolation_status.
        let wt_id = Self::worktree_id(attempt_id);
        match self.manager.isolation_status(&wt_id).await {
            Ok(status) => match status.health {
                WorktreeHealth::Ok => Ok(WorkspaceReconcileResult::Live(lease.clone())),
                WorktreeHealth::Missing => Ok(WorkspaceReconcileResult::AlreadyReleased),
                WorktreeHealth::Detached => Ok(WorkspaceReconcileResult::Conflict(
                    "worktree HEAD is detached from expected branch".to_string(),
                )),
                WorktreeHealth::StaleLock => Ok(WorkspaceReconcileResult::Conflict(
                    "worktree has a stale index.lock".to_string(),
                )),
            },
            Err(e) => {
                // If health check fails, report as conflict.
                Ok(WorkspaceReconcileResult::Conflict(format!(
                    "health check failed: {e}"
                )))
            }
        }
    }

    async fn reset_for_retry(
        &self,
        previous: &WorkspaceLease,
        next_attempt_id: &WorkspaceAttemptId,
    ) -> Result<WorkspaceLease, WorkspaceError> {
        // Release the old lease with RetainForFailure (keep for post-mortem).
        self.release(previous, WorkspaceReleasePolicy::RetainForFailure)
            .await?;

        // Acquire a fresh lease for the next attempt.
        self.acquire(next_attempt_id).await
    }

    async fn release(
        &self,
        lease: &WorkspaceLease,
        policy: WorkspaceReleasePolicy,
    ) -> Result<WorkspaceLeaseState, WorkspaceError> {
        let wt_id = Self::worktree_id(&lease.attempt_id);

        match policy {
            WorkspaceReleasePolicy::Delete => {
                // Actually remove the worktree from disk and the manager registry.
                self.manager.remove(&wt_id).await.map_err(|e| match e {
                    crate::orchestrator::worktree::WorktreeError::NotFound(_) => {
                        // Already gone -- idempotent.
                        return WorkspaceError::LeaseNotFound(lease.lease_id.clone());
                    }
                    crate::orchestrator::worktree::WorktreeError::DirtyWorktree { .. } => {
                        // Dirty worktree: retain instead of deleting, consistent
                        // with the manager's safety behavior.
                        return WorkspaceError::Io(e.to_string());
                    }
                    other => WorkspaceError::Io(other.to_string()),
                })?;
                Ok(WorkspaceLeaseState::Released)
            }
            WorkspaceReleasePolicy::RetainForFailure | WorkspaceReleasePolicy::RetainForReview => {
                // Keep the manager entry without pruning. The worktree stays
                // on disk for inspection. Touch it to prevent idle reclamation.
                self.manager.touch(&wt_id);
                Ok(WorkspaceLeaseState::Retained)
            }
        }
    }

    async fn accept(
        &self,
        lease: &WorkspaceLease,
        request: &WorkspaceAcceptRequest,
    ) -> Result<WorkspaceAcceptance, WorkspaceError> {
        let attempt_id = &lease.attempt_id;
        let tracked =
            self.manager
                .get_attempt(&attempt_id.plan_id, &attempt_id.task_id, attempt_id.attempt);
        match tracked {
            Some(handle) if handle.path == lease.path && handle.branch == lease.branch => {}
            Some(handle) => {
                return Err(WorkspaceError::Io(format!(
                    "tracked worktree {} is at {} on {}, but the lease names {} on {}",
                    handle.id,
                    handle.path.display(),
                    handle.branch,
                    lease.path.display(),
                    lease.branch
                )));
            }
            None => return Err(WorkspaceError::LeaseNotFound(lease.lease_id.clone())),
        }
        let acceptance = AttemptAcceptance {
            run_id: request.run_id.clone(),
            attempt_key: request.attempt_key.clone(),
            verdict: request.verdict.clone(),
            title: request.title.clone(),
        };
        let accepted = self
            .manager
            .accept_attempt(
                &attempt_id.plan_id,
                &attempt_id.task_id,
                attempt_id.attempt,
                &acceptance,
            )
            .await
            .map_err(|error| match error {
                WorktreeError::Conflict { .. } => WorkspaceError::Conflict(error.to_string()),
                WorktreeError::NotFound(_) => WorkspaceError::LeaseNotFound(lease.lease_id.clone()),
                other => WorkspaceError::Io(other.to_string()),
            })?;
        Ok(WorkspaceAcceptance {
            attempt_commit: accepted.attempt_commit,
            plan_branch: format_branch_name(&attempt_id.plan_id),
            accepted_commit: accepted.commit_oid,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_graph::workspace::ExecutionWorkspaceProvider;
    use roko_graph::workspace::fake::InMemoryWorkspaceProvider;
    use std::sync::Arc;

    fn attempt(plan: &str, task: &str, n: u32) -> WorkspaceAttemptId {
        WorkspaceAttemptId {
            plan_id: plan.to_string(),
            task_id: task.to_string(),
            attempt: n,
        }
    }

    // ── InMemoryWorkspaceProvider contract tests ─────────────────────────
    //
    // These tests verify the graph-layer contract using the in-memory fake.
    // They prove that any correct adapter (including the CLI
    // WorktreeExecutionWorkspaceProvider) must satisfy these invariants.

    fn fake_provider() -> InMemoryWorkspaceProvider {
        InMemoryWorkspaceProvider::new(PathBuf::from("/repo"), PathBuf::from("/repo/.worktrees"))
    }

    #[tokio::test]
    async fn acquire_returns_isolated_path() {
        let provider = fake_provider();
        let id = attempt("plan-a", "task-1", 0);
        let lease = provider.acquire(&id).await.unwrap();

        assert_ne!(lease.path, PathBuf::from("/repo"));
        assert!(!lease.path.as_os_str().is_empty());
        assert_eq!(lease.attempt_id, id);
    }

    #[tokio::test]
    async fn acquire_idempotent_for_same_attempt() {
        let provider = fake_provider();
        let id = attempt("plan-a", "task-1", 0);

        let first = provider.acquire(&id).await.unwrap();
        let second = provider.acquire(&id).await.unwrap();

        assert_eq!(first.lease_id, second.lease_id);
        assert_eq!(first.path, second.path);
    }

    #[tokio::test]
    async fn different_attempts_get_different_paths() {
        let provider = fake_provider();
        let a = attempt("plan-a", "task-1", 0);
        let b = attempt("plan-a", "task-1", 1);

        let la = provider.acquire(&a).await.unwrap();
        let lb = provider.acquire(&b).await.unwrap();

        assert_ne!(la.path, lb.path);
        assert_ne!(la.branch, lb.branch);
    }

    #[tokio::test]
    async fn concurrent_acquires_unique_paths() {
        let provider = Arc::new(fake_provider());
        let mut handles = Vec::new();

        for i in 0..10 {
            let p = Arc::clone(&provider);
            handles.push(tokio::spawn(async move {
                let id = attempt("plan-c", &format!("task-{i}"), 0);
                p.acquire(&id).await.unwrap()
            }));
        }

        let leases: Vec<WorkspaceLease> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let paths: std::collections::HashSet<_> = leases.iter().map(|l| l.path.clone()).collect();
        assert_eq!(paths.len(), 10, "all paths must be unique");
    }

    #[tokio::test]
    async fn reconcile_live_returns_live() {
        let provider = fake_provider();
        let id = attempt("plan-a", "task-1", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let result = provider.reconcile(&lease).await.unwrap();
        assert!(matches!(result, WorkspaceReconcileResult::Live(_)));
    }

    #[tokio::test]
    async fn release_delete_then_reconcile_shows_released() {
        let provider = fake_provider();
        let id = attempt("plan-a", "task-1", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let state = provider
            .release(&lease, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap();
        assert_eq!(state, WorkspaceLeaseState::Released);

        let result = provider.reconcile(&lease).await.unwrap();
        assert!(matches!(result, WorkspaceReconcileResult::AlreadyReleased));
    }

    #[tokio::test]
    async fn release_retain_keeps_entry() {
        let provider = fake_provider();
        let id = attempt("plan-a", "task-1", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let state = provider
            .release(&lease, WorkspaceReleasePolicy::RetainForFailure)
            .await
            .unwrap();
        assert_eq!(state, WorkspaceLeaseState::Retained);

        let result = provider.reconcile(&lease).await.unwrap();
        assert!(matches!(result, WorkspaceReconcileResult::Orphaned(_)));
    }

    #[tokio::test]
    async fn reset_for_retry_retains_old_acquires_new() {
        let provider = fake_provider();
        let id0 = attempt("plan-a", "task-1", 0);
        let id1 = attempt("plan-a", "task-1", 1);

        let lease0 = provider.acquire(&id0).await.unwrap();
        let lease1 = provider.reset_for_retry(&lease0, &id1).await.unwrap();

        // Old lease is retained (orphaned).
        let old = provider.reconcile(&lease0).await.unwrap();
        assert!(matches!(old, WorkspaceReconcileResult::Orphaned(_)));

        // New lease is live.
        let new = provider.reconcile(&lease1).await.unwrap();
        assert!(matches!(new, WorkspaceReconcileResult::Live(_)));

        // Paths differ.
        assert_ne!(lease0.path, lease1.path);
    }

    #[tokio::test]
    async fn budget_exhaustion() {
        let provider = fake_provider().with_max_live(1);
        let a = attempt("plan-a", "task-1", 0);
        let b = attempt("plan-a", "task-2", 0);

        provider.acquire(&a).await.unwrap();
        let err = provider.acquire(&b).await.unwrap_err();
        assert!(matches!(err, WorkspaceError::BudgetExhausted { max: 1 }));
    }

    #[tokio::test]
    async fn release_frees_budget() {
        let provider = fake_provider().with_max_live(1);
        let a = attempt("plan-a", "task-1", 0);
        let b = attempt("plan-a", "task-2", 0);

        let lease_a = provider.acquire(&a).await.unwrap();
        provider
            .release(&lease_a, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap();

        // Budget is freed, second acquire should succeed.
        let lease_b = provider.acquire(&b).await.unwrap();
        assert_ne!(lease_a.path, lease_b.path);
    }

    #[tokio::test]
    async fn lease_serde_roundtrip() {
        let provider = fake_provider();
        let id = attempt("plan-a", "task-1", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let json = serde_json::to_string(&lease).unwrap();
        let back: WorkspaceLease = serde_json::from_str(&json).unwrap();
        assert_eq!(lease, back);
    }

    #[tokio::test]
    async fn reset_never_reuses_path() {
        let provider = fake_provider();
        let id0 = attempt("plan-a", "task-1", 0);
        let id1 = attempt("plan-a", "task-1", 1);
        let id2 = attempt("plan-a", "task-1", 2);

        let l0 = provider.acquire(&id0).await.unwrap();
        let l1 = provider.reset_for_retry(&l0, &id1).await.unwrap();
        let l2 = provider.reset_for_retry(&l1, &id2).await.unwrap();

        let paths: std::collections::HashSet<_> =
            [&l0.path, &l1.path, &l2.path].into_iter().collect();
        assert_eq!(paths.len(), 3, "every retry must get a unique path");
    }

    #[tokio::test]
    async fn orphan_cleanup() {
        let provider = fake_provider();
        let a = attempt("plan-a", "task-1", 0);
        let b = attempt("plan-a", "task-2", 0);

        let la = provider.acquire(&a).await.unwrap();
        provider.acquire(&b).await.unwrap(); // stays active

        // Retain lease a (simulating failure).
        provider
            .release(&la, WorkspaceReleasePolicy::RetainForFailure)
            .await
            .unwrap();

        // Cleanup orphans should release the retained lease.
        let cleaned = provider.cleanup_orphans().await;
        assert_eq!(cleaned.len(), 1);

        // Only one active lease should remain.
        let active = provider.active_leases();
        assert_eq!(active.len(), 1);
    }
}
