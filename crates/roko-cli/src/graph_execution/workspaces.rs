//! CLI host adapter that implements [`ExecutionWorkspaceProvider`] using the
//! existing [`WorktreeManager`].
//!
//! This bridges the repo-neutral graph workspace port to the CLI's concrete
//! git-worktree infrastructure. The adapter:
//!
//! - Maps `acquire` to `create_for_attempt` / `ensure_for_attempt`.
//! - Maps `reconcile` to `get_attempt` + `isolation_status`.
//! - Maps `release` with `Delete` to `remove`; `RetainForFailure` and
//!   `RetainForReview` keep the manager entry and return `Retained`.
//! - Rejects any returned path equal to the configured repository root
//!   before returning a lease.
//! - Orphan cleanup delegates to existing manager reconciliation/prune
//!   facilities and never removes an unproved path.

use std::path::PathBuf;

use roko_graph::workspace::{
    ExecutionWorkspaceProvider, WorkspaceAttemptId, WorkspaceError, WorkspaceLease,
    WorkspaceLeaseState, WorkspaceReconcileResult, WorkspaceReleasePolicy,
};

use crate::orchestrator::worktree::{
    WorktreeHealth, WorktreeManager, format_attempt_worktree_id,
};

/// CLI adapter that wraps [`WorktreeManager`] to implement the graph-layer
/// [`ExecutionWorkspaceProvider`] trait.
///
/// # Shared-checkout rejection
///
/// The adapter stores the repository root at construction and rejects any
/// acquired workspace whose path equals it. This prevents the graph engine
/// from accidentally running an attempt in the shared project root.
#[derive(Clone, Debug)]
pub struct WorktreeExecutionWorkspaceProvider {
    /// The underlying worktree manager.
    manager: WorktreeManager,
    /// Absolute path to the repository root (for shared-checkout rejection).
    repo_root: PathBuf,
}

impl WorktreeExecutionWorkspaceProvider {
    /// Create a new adapter wrapping the given manager.
    ///
    /// `repo_root` must be the canonical absolute path to the repository's
    /// main checkout. Any lease whose path equals this root will be rejected
    /// by `acquire`.
    #[must_use]
    pub fn new(manager: WorktreeManager, repo_root: PathBuf) -> Self {
        Self { manager, repo_root }
    }

    /// Access the underlying worktree manager for operations not covered
    /// by the `ExecutionWorkspaceProvider` trait (e.g. orphan cleanup,
    /// prune, idle reclamation).
    #[must_use]
    pub fn manager(&self) -> &WorktreeManager {
        &self.manager
    }

    /// Build a [`WorkspaceLease`] from a manager handle and attempt identity.
    fn lease_from_handle(
        &self,
        handle: &crate::orchestrator::worktree::WorktreeHandle,
        attempt_id: &WorkspaceAttemptId,
        base_revision: &str,
    ) -> WorkspaceLease {
        WorkspaceLease {
            lease_id: handle.id.clone(),
            attempt_id: attempt_id.clone(),
            path: handle.path.clone(),
            branch: handle.branch.clone(),
            base_revision: base_revision.to_string(),
            lease_fingerprint: attempt_id.fingerprint(),
        }
    }

    /// Reject a path that equals the repository root.
    fn reject_shared_checkout(&self, path: &PathBuf) -> Result<(), WorkspaceError> {
        if *path == self.repo_root {
            return Err(WorkspaceError::SharedCheckoutRejected);
        }
        Ok(())
    }

    /// Map a [`crate::orchestrator::worktree::WorktreeError`] to a
    /// [`WorkspaceError`].
    fn map_worktree_error(
        err: crate::orchestrator::worktree::WorktreeError,
    ) -> WorkspaceError {
        use crate::orchestrator::worktree::WorktreeError as WE;
        match err {
            WE::BudgetExhausted { max } => WorkspaceError::BudgetExhausted { max },
            other => WorkspaceError::Io(other.to_string()),
        }
    }
}

#[async_trait::async_trait]
impl ExecutionWorkspaceProvider for WorktreeExecutionWorkspaceProvider {
    async fn acquire(
        &self,
        attempt_id: &WorkspaceAttemptId,
    ) -> Result<WorkspaceLease, WorkspaceError> {
        // Idempotent: if the attempt already has a tracked handle, return it.
        if let Some(handle) = self.manager.get_attempt(
            &attempt_id.plan_id,
            &attempt_id.task_id,
            attempt_id.attempt,
        ) {
            self.reject_shared_checkout(&handle.path)?;
            let base = self
                .manager
                .accepted_for_plan(&attempt_id.plan_id)
                .map_or_else(|| "HEAD".to_string(), |a| a.commit_oid);
            return Ok(self.lease_from_handle(&handle, attempt_id, &base));
        }

        // Create a new attempt worktree via the manager.
        let handle = self
            .manager
            .create_for_attempt(
                &attempt_id.plan_id,
                &attempt_id.task_id,
                attempt_id.attempt,
            )
            .await
            .map_err(Self::map_worktree_error)?;

        self.reject_shared_checkout(&handle.path)?;

        let base = self
            .manager
            .accepted_for_plan(&attempt_id.plan_id)
            .map_or_else(|| "HEAD".to_string(), |a| a.commit_oid);

        Ok(self.lease_from_handle(&handle, attempt_id, &base))
    }

    async fn reconcile(
        &self,
        lease: &WorkspaceLease,
    ) -> Result<WorkspaceReconcileResult, WorkspaceError> {
        let worktree_id = format_attempt_worktree_id(
            &lease.attempt_id.plan_id,
            &lease.attempt_id.task_id,
            lease.attempt_id.attempt,
        );

        // Check if the manager tracks this attempt.
        let handle = match self.manager.get(&worktree_id) {
            Some(h) => h,
            None => {
                // Not tracked -- check if the path exists on disk.
                if lease.path.exists() {
                    return Ok(WorkspaceReconcileResult::Orphaned(lease.clone()));
                }
                return Ok(WorkspaceReconcileResult::AlreadyReleased);
            }
        };

        // Verify the handle matches the lease.
        if handle.path != lease.path || handle.branch != lease.branch {
            return Ok(WorkspaceReconcileResult::Conflict(format!(
                "tracked handle for '{}' has path={:?} branch={}, but lease expects path={:?} branch={}",
                worktree_id,
                handle.path,
                handle.branch,
                lease.path,
                lease.branch,
            )));
        }

        // Probe isolation status for health verification.
        let isolation = self
            .manager
            .isolation_status(&worktree_id)
            .await
            .map_err(Self::map_worktree_error)?;

        match isolation.health {
            WorktreeHealth::Ok => {
                Ok(WorkspaceReconcileResult::Live(lease.clone()))
            }
            WorktreeHealth::Missing => {
                Ok(WorkspaceReconcileResult::AlreadyReleased)
            }
            WorktreeHealth::StaleLock | WorktreeHealth::Detached => {
                Ok(WorkspaceReconcileResult::Conflict(format!(
                    "worktree '{}' health check: {:?}",
                    worktree_id, isolation.health,
                )))
            }
        }
    }

    async fn reset_for_retry(
        &self,
        previous: &WorkspaceLease,
        next_attempt_id: &WorkspaceAttemptId,
    ) -> Result<WorkspaceLease, WorkspaceError> {
        // Release the old lease with RetainForFailure (never delete on retry).
        self.release(previous, WorkspaceReleasePolicy::RetainForFailure)
            .await?;

        // Acquire a fresh checkout for the next attempt.
        self.acquire(next_attempt_id).await
    }

    async fn release(
        &self,
        lease: &WorkspaceLease,
        policy: WorkspaceReleasePolicy,
    ) -> Result<WorkspaceLeaseState, WorkspaceError> {
        let worktree_id = format_attempt_worktree_id(
            &lease.attempt_id.plan_id,
            &lease.attempt_id.task_id,
            lease.attempt_id.attempt,
        );

        match policy {
            WorkspaceReleasePolicy::Delete => {
                // Only attempt removal if the manager tracks this worktree.
                if self.manager.get(&worktree_id).is_some() {
                    self.manager
                        .remove(&worktree_id)
                        .await
                        .map_err(Self::map_worktree_error)?;
                }
                Ok(WorkspaceLeaseState::Released)
            }
            WorkspaceReleasePolicy::RetainForFailure
            | WorkspaceReleasePolicy::RetainForReview => {
                // Keep the manager entry and worktree on disk.
                // The worktree is no longer actively owned by a running attempt
                // but is preserved for inspection.
                Ok(WorkspaceLeaseState::Retained)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_id_matches_manager_convention() {
        // The adapter must produce the same worktree IDs as the manager's
        // format_attempt_worktree_id so get/reconcile/release can find handles.
        let attempt_id = WorkspaceAttemptId {
            plan_id: "my-plan".to_string(),
            task_id: "compile".to_string(),
            attempt: 2,
        };
        let adapter_id = format_attempt_worktree_id(
            &attempt_id.plan_id,
            &attempt_id.task_id,
            attempt_id.attempt,
        );
        let manager_id =
            format_attempt_worktree_id("my-plan", "compile", 2);
        assert_eq!(adapter_id, manager_id);
    }

    #[test]
    fn branch_name_matches_manager_convention() {
        use crate::orchestrator::worktree::format_attempt_branch_name;
        let branch = format_attempt_branch_name("plan-x", "task-y", 0);
        assert!(branch.starts_with("roko/attempt/"));
    }

    #[test]
    fn shared_checkout_rejection() {
        use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};
        use std::time::Duration;

        let config = WorktreeConfig {
            repo_root: PathBuf::from("/project"),
            base_branch: "main".to_string(),
            worktrees_root: PathBuf::from("/project/.worktrees"),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        };
        let manager = WorktreeManager::new(config);
        let provider =
            WorktreeExecutionWorkspaceProvider::new(manager, PathBuf::from("/project"));

        let err = provider
            .reject_shared_checkout(&PathBuf::from("/project"))
            .unwrap_err();
        assert!(
            matches!(err, WorkspaceError::SharedCheckoutRejected),
            "repo root must be rejected"
        );

        // Different paths pass.
        assert!(provider
            .reject_shared_checkout(&PathBuf::from("/project/.worktrees/attempt-abc"))
            .is_ok());
    }

    #[test]
    fn lease_from_handle_produces_correct_fingerprint() {
        use crate::orchestrator::worktree::{WorktreeConfig, WorktreeHandle, WorktreeManager};
        use std::time::Duration;

        let config = WorktreeConfig {
            repo_root: PathBuf::from("/repo"),
            base_branch: "main".to_string(),
            worktrees_root: PathBuf::from("/repo/.worktrees"),
            max_live: None,
            idle_ttl: Duration::from_secs(3600),
        };
        let manager = WorktreeManager::new(config);
        let provider =
            WorktreeExecutionWorkspaceProvider::new(manager, PathBuf::from("/repo"));

        let attempt_id = WorkspaceAttemptId {
            plan_id: "p1".to_string(),
            task_id: "t1".to_string(),
            attempt: 0,
        };
        let handle = WorktreeHandle {
            id: "attempt-abc".to_string(),
            path: PathBuf::from("/repo/.worktrees/attempt-abc"),
            branch: "roko/attempt/attempt-abc".to_string(),
            created_at_ms: 1000,
            last_active_ms: 1000,
        };

        let lease = provider.lease_from_handle(&handle, &attempt_id, "abc123");

        assert_eq!(lease.lease_id, "attempt-abc");
        assert_eq!(lease.attempt_id, attempt_id);
        assert_eq!(lease.path, handle.path);
        assert_eq!(lease.branch, handle.branch);
        assert_eq!(lease.base_revision, "abc123");
        assert_eq!(lease.lease_fingerprint, attempt_id.fingerprint());
    }

    #[test]
    fn map_budget_error() {
        use crate::orchestrator::worktree::WorktreeError;

        let err = WorktreeError::BudgetExhausted { max: 5 };
        let mapped =
            WorktreeExecutionWorkspaceProvider::map_worktree_error(err);
        assert!(matches!(
            mapped,
            WorkspaceError::BudgetExhausted { max: 5 }
        ));
    }

    #[test]
    fn map_generic_error() {
        use crate::orchestrator::worktree::WorktreeError;

        let err = WorktreeError::NotFound("ghost".to_string());
        let mapped =
            WorktreeExecutionWorkspaceProvider::map_worktree_error(err);
        assert!(matches!(mapped, WorkspaceError::Io(_)));
    }
}
