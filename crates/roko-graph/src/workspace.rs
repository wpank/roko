//! Workspace and attempt lifecycle port for graph execution.
//!
//! This module defines the repo-neutral types and async trait that graph
//! execution uses to acquire, reconcile, and release per-attempt worktrees.
//! The trait is implemented by host adapters (e.g. the CLI
//! `WorktreeExecutionWorkspaceProvider`) and by the in-memory fake for tests.
//!
//! All mutating methods return the resulting serializable lease/state so
//! downstream checkpoint consumers (#251) can persist it. `acquire` is
//! idempotent for the same exact attempt ID and fingerprint; a different
//! active attempt never receives the same path or branch.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Uniquely identifies a single execution attempt within a plan task.
///
/// The triple `(plan_id, task_id, attempt)` is collision-free across
/// concurrent graph executions. Downstream checkpoint and lease consumers
/// use this identity to bind workspace resources to exactly one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceAttemptId {
    /// Plan that owns the task.
    pub plan_id: String,
    /// Task within the plan.
    pub task_id: String,
    /// Zero-indexed attempt counter for retries.
    pub attempt: u32,
}

impl WorkspaceAttemptId {
    /// Compute a stable BLAKE3-based fingerprint of this attempt identity.
    ///
    /// The fingerprint is used for idempotent `acquire` calls — if the same
    /// attempt ID produces the same fingerprint, the provider returns the
    /// existing lease instead of allocating a new checkout.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let input = format!("{}\0{}\0{}", self.plan_id, self.task_id, self.attempt);
        roko_core::ContentHash::of(input.as_bytes()).to_hex()
    }
}

impl std::fmt::Display for WorkspaceAttemptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/attempt-{}",
            self.plan_id, self.task_id, self.attempt
        )
    }
}

// ---------------------------------------------------------------------------
// Lease
// ---------------------------------------------------------------------------

/// A live or historical workspace lease binding an attempt to a checkout.
///
/// Every field is serializable so #251 can persist leases as host checkpoint
/// extensions. The `lease_fingerprint` is derived from the `attempt_id` and
/// is used by `acquire` for idempotency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceLease {
    /// Opaque lease identifier assigned by the provider.
    pub lease_id: String,
    /// The attempt this lease is bound to.
    pub attempt_id: WorkspaceAttemptId,
    /// Absolute filesystem path to the checkout directory.
    pub path: PathBuf,
    /// Branch checked out in the workspace.
    pub branch: String,
    /// Git revision the branch was created from.
    pub base_revision: String,
    /// BLAKE3-based fingerprint of the attempt identity, used for
    /// idempotent `acquire` and conflict detection.
    pub lease_fingerprint: String,
}

// ---------------------------------------------------------------------------
// Lease state machine
// ---------------------------------------------------------------------------

/// Terminal and intermediate states a workspace lease can be in.
///
/// The state machine is: `Acquired` -> (`Accepted` | `Released`), with
/// `Retained` as a side state for failure/review preservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceLeaseState {
    /// The workspace is checked out and owned by the attempt.
    Acquired,
    /// The attempt's work was accepted (e.g. gate passed, commit merged).
    Accepted,
    /// The workspace was kept after failure or for review but is no
    /// longer actively owned by a running attempt.
    Retained,
    /// The workspace has been fully released and cleaned up.
    Released,
}

impl WorkspaceLeaseState {
    /// Whether this state represents an active (non-terminal) lease.
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Acquired)
    }

    /// Whether this state represents a terminal (completed) lease.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Accepted | Self::Released)
    }
}

impl std::fmt::Display for WorkspaceLeaseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Acquired => write!(f, "acquired"),
            Self::Accepted => write!(f, "accepted"),
            Self::Retained => write!(f, "retained"),
            Self::Released => write!(f, "released"),
        }
    }
}

impl std::fmt::Display for WorkspaceLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "lease {} for {} at {}",
            self.lease_id,
            self.attempt_id,
            self.path.display()
        )
    }
}

// ---------------------------------------------------------------------------
// Release policy
// ---------------------------------------------------------------------------

/// Policy governing how a workspace is released after an attempt completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceReleasePolicy {
    /// Delete the checkout and branch. Used for successful attempts after
    /// their commits have been merged.
    Delete,
    /// Keep the checkout on disk for post-mortem inspection after a
    /// failed attempt.
    RetainForFailure,
    /// Keep the checkout on disk for human review (e.g. manual approval
    /// required before merge).
    RetainForReview,
}

impl std::fmt::Display for WorkspaceReleasePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delete => write!(f, "delete"),
            Self::RetainForFailure => write!(f, "retain_for_failure"),
            Self::RetainForReview => write!(f, "retain_for_review"),
        }
    }
}

// ---------------------------------------------------------------------------
// Reconciliation
// ---------------------------------------------------------------------------

/// Result of reconciling a lease against the actual workspace state.
///
/// Used by resume/recovery flows to determine whether a previously
/// checkpointed lease is still usable. Serializable so #251 can persist
/// reconciliation results in host checkpoint extensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "payload")]
pub enum WorkspaceReconcileResult {
    /// The workspace exists and matches the lease — ready to use.
    Live(WorkspaceLease),
    /// The workspace was already released (no longer on disk).
    AlreadyReleased,
    /// The workspace exists but its state conflicts with the lease
    /// (e.g. different branch, detached HEAD).
    Conflict(String),
    /// The workspace exists on disk but is not tracked by the provider.
    /// The lease is returned so the caller can decide whether to adopt
    /// or clean it up.
    Orphaned(WorkspaceLease),
}

// ---------------------------------------------------------------------------
// Workspace errors
// ---------------------------------------------------------------------------

/// Errors returned by [`ExecutionWorkspaceProvider`] operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// The requested path is the repository root, which must never be
    /// issued as an attempt lease.
    #[error("workspace path equals repository root; shared checkout rejected")]
    SharedCheckoutRejected,

    /// The provider's live-worktree budget is exhausted.
    #[error("workspace budget exhausted (max: {max})")]
    BudgetExhausted {
        /// Configured maximum.
        max: usize,
    },

    /// The requested attempt already has an active lease with a different
    /// fingerprint, indicating a concurrent ownership conflict.
    #[error("ownership conflict for {attempt_id}: existing fingerprint {existing}, requested {requested}")]
    OwnershipConflict {
        /// The attempt that owns the conflicting lease.
        attempt_id: WorkspaceAttemptId,
        /// Fingerprint of the existing lease.
        existing: String,
        /// Fingerprint the caller tried to acquire with.
        requested: String,
    },

    /// The lease was not found in the provider's registry.
    #[error("lease not found: {0}")]
    LeaseNotFound(String),

    /// An underlying I/O or git error.
    #[error("workspace I/O error: {0}")]
    Io(String),
}

// ---------------------------------------------------------------------------
// Port trait
// ---------------------------------------------------------------------------

/// Async trait for workspace lifecycle management during graph execution.
///
/// Host adapters implement this trait to provide isolated worktrees for
/// each task attempt. The graph engine calls `acquire` before dispatching
/// a cell and `release` after the attempt completes (with appropriate
/// policy). `reconcile` is used during resume to verify checkpointed leases.
///
/// # Idempotency
///
/// `acquire` is idempotent for the same exact attempt ID and fingerprint:
/// calling it twice with the same `WorkspaceAttemptId` returns the existing
/// lease. A different active attempt never receives the same path or branch.
///
/// # Safety
///
/// `reset_for_retry` always releases the old lease with `RetainForFailure`
/// and acquires a new exact-attempt lease. It never hard-resets and reuses
/// a writable checkout.
#[async_trait::async_trait]
pub trait ExecutionWorkspaceProvider: Send + Sync {
    /// Acquire a workspace for the given attempt.
    ///
    /// Returns an existing lease if one with the same attempt ID and
    /// fingerprint is already active. Returns `WorkspaceError::OwnershipConflict`
    /// if a lease exists with a different fingerprint.
    ///
    /// The returned path must never equal the repository root.
    async fn acquire(
        &self,
        attempt_id: &WorkspaceAttemptId,
    ) -> Result<WorkspaceLease, WorkspaceError>;

    /// Reconcile a previously checkpointed lease against the live workspace
    /// state.
    ///
    /// Used during resume to verify that a workspace from a previous session
    /// is still valid and usable.
    async fn reconcile(
        &self,
        lease: &WorkspaceLease,
    ) -> Result<WorkspaceReconcileResult, WorkspaceError>;

    /// Release an old attempt lease with `RetainForFailure` and acquire a
    /// new lease for the next attempt.
    ///
    /// This method never hard-resets and reuses a writable checkout. The
    /// old lease is retained for post-mortem, and a completely new checkout
    /// is created for the next attempt.
    async fn reset_for_retry(
        &self,
        previous: &WorkspaceLease,
        next_attempt_id: &WorkspaceAttemptId,
    ) -> Result<WorkspaceLease, WorkspaceError>;

    /// Release a workspace according to the given policy.
    ///
    /// - `Delete`: remove the checkout and branch.
    /// - `RetainForFailure`/`RetainForReview`: keep the checkout, mark as
    ///   retained.
    ///
    /// Returns the terminal lease state.
    async fn release(
        &self,
        lease: &WorkspaceLease,
        policy: WorkspaceReleasePolicy,
    ) -> Result<WorkspaceLeaseState, WorkspaceError>;
}

// ---------------------------------------------------------------------------
// In-memory fake provider (for testing)
// ---------------------------------------------------------------------------

#[cfg(any(test, feature = "test-support"))]
pub mod fake {
    //! In-memory [`ExecutionWorkspaceProvider`] for unit and integration tests.
    //!
    //! The fake provider tracks leases in memory without touching the filesystem.
    //! It enforces the same ownership, idempotency, and shared-checkout
    //! rejection contracts as the real CLI adapter.

    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use parking_lot::Mutex;

    use super::*;

    /// In-memory workspace provider that tracks leases without filesystem
    /// operations. Useful for testing graph execution workspace lifecycle.
    #[derive(Debug)]
    pub struct InMemoryWorkspaceProvider {
        /// Simulated repository root (used for shared-checkout rejection).
        repo_root: PathBuf,
        /// Simulated worktree output directory.
        worktrees_root: PathBuf,
        /// Simulated base branch name.
        base_branch: String,
        /// Active leases indexed by attempt fingerprint.
        leases: Mutex<HashMap<String, (WorkspaceLease, WorkspaceLeaseState)>>,
        /// Monotonic lease ID counter.
        next_id: AtomicU64,
        /// Optional budget cap (None = unlimited).
        max_live: Option<usize>,
    }

    impl InMemoryWorkspaceProvider {
        /// Create a new in-memory provider.
        #[must_use]
        pub fn new(repo_root: PathBuf, worktrees_root: PathBuf) -> Self {
            Self {
                repo_root,
                worktrees_root,
                base_branch: "main".to_string(),
                leases: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
                max_live: None,
            }
        }

        /// Set a maximum number of concurrent active leases.
        #[must_use]
        pub fn with_max_live(mut self, max: usize) -> Self {
            self.max_live = Some(max);
            self
        }

        /// Set the simulated base branch.
        #[must_use]
        pub fn with_base_branch(mut self, branch: impl Into<String>) -> Self {
            self.base_branch = branch.into();
            self
        }

        /// Return all currently active leases.
        pub fn active_leases(&self) -> Vec<WorkspaceLease> {
            self.leases
                .lock()
                .values()
                .filter(|(_, state)| state.is_active())
                .map(|(lease, _)| lease.clone())
                .collect()
        }

        /// Return total number of tracked leases (including retained/released).
        pub fn total_lease_count(&self) -> usize {
            self.leases.lock().len()
        }

        fn allocate_lease_id(&self) -> String {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            format!("fake-lease-{id}")
        }

        fn build_path(&self, attempt_id: &WorkspaceAttemptId) -> PathBuf {
            let dir_name = format!(
                "attempt-{}-{}-{}",
                attempt_id.plan_id, attempt_id.task_id, attempt_id.attempt
            );
            self.worktrees_root.join(dir_name)
        }

        fn build_branch(&self, attempt_id: &WorkspaceAttemptId) -> String {
            format!(
                "roko/attempt/{}/{}/{}",
                attempt_id.plan_id, attempt_id.task_id, attempt_id.attempt
            )
        }

        fn active_count(&self) -> usize {
            self.leases
                .lock()
                .values()
                .filter(|(_, state)| state.is_active())
                .count()
        }

        /// Reconcile all tracked leases and return results keyed by fingerprint.
        ///
        /// This is the fake equivalent of the CLI manager's batch reconciliation
        /// and prune facilities. It never removes an unproved path.
        pub async fn reconcile_all(
            &self,
        ) -> Vec<(WorkspaceLease, WorkspaceReconcileResult)> {
            let snapshot: Vec<WorkspaceLease> = self
                .leases
                .lock()
                .values()
                .map(|(lease, _)| lease.clone())
                .collect();

            let mut results = Vec::with_capacity(snapshot.len());
            for lease in snapshot {
                if let Ok(result) = self.reconcile(&lease).await {
                    results.push((lease, result));
                }
            }
            results
        }

        /// Release all orphaned (Retained) leases, transitioning them to Released.
        ///
        /// Returns the leases that were cleaned up. This never removes an
        /// unproved path -- only leases the provider already tracks as Retained
        /// are eligible.
        pub async fn cleanup_orphans(&self) -> Vec<WorkspaceLease> {
            let orphans: Vec<WorkspaceLease> = self
                .leases
                .lock()
                .values()
                .filter(|(_, state)| *state == WorkspaceLeaseState::Retained)
                .map(|(lease, _)| lease.clone())
                .collect();

            let mut cleaned = Vec::new();
            for lease in orphans {
                if self
                    .release(&lease, WorkspaceReleasePolicy::Delete)
                    .await
                    .is_ok()
                {
                    cleaned.push(lease);
                }
            }
            cleaned
        }
    }

    #[async_trait::async_trait]
    impl ExecutionWorkspaceProvider for InMemoryWorkspaceProvider {
        async fn acquire(
            &self,
            attempt_id: &WorkspaceAttemptId,
        ) -> Result<WorkspaceLease, WorkspaceError> {
            let fingerprint = attempt_id.fingerprint();
            let mut guard = self.leases.lock();

            // Idempotent: return existing lease if fingerprint matches.
            if let Some((existing, state)) = guard.get(&fingerprint) {
                if state.is_active() {
                    if existing.lease_fingerprint == fingerprint {
                        return Ok(existing.clone());
                    }
                    return Err(WorkspaceError::OwnershipConflict {
                        attempt_id: attempt_id.clone(),
                        existing: existing.lease_fingerprint.clone(),
                        requested: fingerprint,
                    });
                }
            }

            // Budget check.
            if let Some(max) = self.max_live {
                let active = guard
                    .values()
                    .filter(|(_, state)| state.is_active())
                    .count();
                if active >= max {
                    return Err(WorkspaceError::BudgetExhausted { max });
                }
            }

            let path = self.build_path(attempt_id);

            // Shared-checkout rejection.
            if path == self.repo_root {
                return Err(WorkspaceError::SharedCheckoutRejected);
            }

            // Verify no other active lease has the same path.
            for (existing, state) in guard.values() {
                if state.is_active() && existing.path == path {
                    return Err(WorkspaceError::OwnershipConflict {
                        attempt_id: attempt_id.clone(),
                        existing: existing.lease_fingerprint.clone(),
                        requested: fingerprint,
                    });
                }
            }

            let lease = WorkspaceLease {
                lease_id: self.allocate_lease_id(),
                attempt_id: attempt_id.clone(),
                path,
                branch: self.build_branch(attempt_id),
                base_revision: self.base_branch.clone(),
                lease_fingerprint: fingerprint.clone(),
            };

            guard.insert(fingerprint, (lease.clone(), WorkspaceLeaseState::Acquired));
            Ok(lease)
        }

        async fn reconcile(
            &self,
            lease: &WorkspaceLease,
        ) -> Result<WorkspaceReconcileResult, WorkspaceError> {
            let guard = self.leases.lock();

            match guard.get(&lease.lease_fingerprint) {
                Some((existing, state)) => match state {
                    WorkspaceLeaseState::Acquired | WorkspaceLeaseState::Accepted => {
                        if existing.lease_id == lease.lease_id
                            && existing.branch == lease.branch
                            && existing.path == lease.path
                        {
                            Ok(WorkspaceReconcileResult::Live(existing.clone()))
                        } else {
                            Ok(WorkspaceReconcileResult::Conflict(format!(
                                "lease {} does not match tracked state",
                                lease.lease_id
                            )))
                        }
                    }
                    WorkspaceLeaseState::Retained => {
                        Ok(WorkspaceReconcileResult::Orphaned(existing.clone()))
                    }
                    WorkspaceLeaseState::Released => {
                        Ok(WorkspaceReconcileResult::AlreadyReleased)
                    }
                },
                None => Ok(WorkspaceReconcileResult::Orphaned(lease.clone())),
            }
        }

        async fn reset_for_retry(
            &self,
            previous: &WorkspaceLease,
            next_attempt_id: &WorkspaceAttemptId,
        ) -> Result<WorkspaceLease, WorkspaceError> {
            // Release the old lease with RetainForFailure.
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
            let mut guard = self.leases.lock();
            let entry = guard.get_mut(&lease.lease_fingerprint).ok_or_else(|| {
                WorkspaceError::LeaseNotFound(lease.lease_id.clone())
            })?;

            let terminal_state = match policy {
                WorkspaceReleasePolicy::Delete => WorkspaceLeaseState::Released,
                WorkspaceReleasePolicy::RetainForFailure
                | WorkspaceReleasePolicy::RetainForReview => WorkspaceLeaseState::Retained,
            };

            entry.1 = terminal_state;
            Ok(terminal_state)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::fake::InMemoryWorkspaceProvider;

    fn test_provider() -> InMemoryWorkspaceProvider {
        InMemoryWorkspaceProvider::new(
            PathBuf::from("/repo"),
            PathBuf::from("/repo/.worktrees"),
        )
    }

    fn attempt(plan: &str, task: &str, n: u32) -> WorkspaceAttemptId {
        WorkspaceAttemptId {
            plan_id: plan.to_string(),
            task_id: task.to_string(),
            attempt: n,
        }
    }

    // -- WorkspaceAttemptId ---------------------------------------------------

    #[test]
    fn attempt_id_fingerprint_is_stable() {
        let id = attempt("plan-1", "task-a", 0);
        let fp1 = id.fingerprint();
        let fp2 = id.fingerprint();
        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 64); // BLAKE3 hex
    }

    #[test]
    fn attempt_id_fingerprint_differs_by_attempt() {
        let a = attempt("plan-1", "task-a", 0);
        let b = attempt("plan-1", "task-a", 1);
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn attempt_id_display() {
        let id = attempt("my-plan", "compile", 3);
        assert_eq!(id.to_string(), "my-plan/compile/attempt-3");
    }

    // -- Lease state machine --------------------------------------------------

    #[test]
    fn lease_state_active_and_terminal() {
        assert!(WorkspaceLeaseState::Acquired.is_active());
        assert!(!WorkspaceLeaseState::Acquired.is_terminal());

        assert!(!WorkspaceLeaseState::Accepted.is_active());
        assert!(WorkspaceLeaseState::Accepted.is_terminal());

        assert!(!WorkspaceLeaseState::Retained.is_active());
        assert!(!WorkspaceLeaseState::Retained.is_terminal());

        assert!(!WorkspaceLeaseState::Released.is_active());
        assert!(WorkspaceLeaseState::Released.is_terminal());
    }

    // -- Serde round-trip -----------------------------------------------------

    #[test]
    fn attempt_id_serde_roundtrip() {
        let id = attempt("plan-x", "task-y", 7);
        let json = serde_json::to_string(&id).unwrap();
        let back: WorkspaceAttemptId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn lease_serde_roundtrip() {
        let lease = WorkspaceLease {
            lease_id: "lease-42".to_string(),
            attempt_id: attempt("p", "t", 1),
            path: PathBuf::from("/worktrees/attempt-p-t-1"),
            branch: "roko/attempt/p/t/1".to_string(),
            base_revision: "main".to_string(),
            lease_fingerprint: attempt("p", "t", 1).fingerprint(),
        };
        let json = serde_json::to_string(&lease).unwrap();
        let back: WorkspaceLease = serde_json::from_str(&json).unwrap();
        assert_eq!(lease, back);
    }

    #[test]
    fn lease_state_serde_roundtrip() {
        for state in [
            WorkspaceLeaseState::Acquired,
            WorkspaceLeaseState::Accepted,
            WorkspaceLeaseState::Retained,
            WorkspaceLeaseState::Released,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: WorkspaceLeaseState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn release_policy_serde_roundtrip() {
        for policy in [
            WorkspaceReleasePolicy::Delete,
            WorkspaceReleasePolicy::RetainForFailure,
            WorkspaceReleasePolicy::RetainForReview,
        ] {
            let json = serde_json::to_string(&policy).unwrap();
            let back: WorkspaceReleasePolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(policy, back);
        }
    }

    // -- InMemoryWorkspaceProvider: acquire ------------------------------------

    #[tokio::test]
    async fn acquire_returns_isolated_checkout() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);
        let lease = provider.acquire(&id).await.unwrap();

        assert!(!lease.path.as_os_str().is_empty());
        assert_ne!(lease.path, PathBuf::from("/repo"));
        assert!(!lease.branch.is_empty());
        assert_eq!(lease.attempt_id, id);
        assert_eq!(lease.lease_fingerprint, id.fingerprint());
    }

    #[tokio::test]
    async fn acquire_idempotent_same_attempt() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);

        let first = provider.acquire(&id).await.unwrap();
        let second = provider.acquire(&id).await.unwrap();

        assert_eq!(first.lease_id, second.lease_id);
        assert_eq!(first.path, second.path);
    }

    #[tokio::test]
    async fn acquire_different_attempts_get_different_paths() {
        let provider = test_provider();
        let a = attempt("plan-1", "task-a", 0);
        let b = attempt("plan-1", "task-a", 1);

        let lease_a = provider.acquire(&a).await.unwrap();
        let lease_b = provider.acquire(&b).await.unwrap();

        assert_ne!(lease_a.path, lease_b.path);
        assert_ne!(lease_a.branch, lease_b.branch);
        assert_ne!(lease_a.lease_id, lease_b.lease_id);
    }

    #[tokio::test]
    async fn acquire_budget_exhausted() {
        let provider = test_provider().with_max_live(1);
        let a = attempt("plan-1", "task-a", 0);
        let b = attempt("plan-1", "task-b", 0);

        provider.acquire(&a).await.unwrap();
        let err = provider.acquire(&b).await.unwrap_err();

        assert!(matches!(err, WorkspaceError::BudgetExhausted { max: 1 }));
    }

    // -- Concurrent checkout rejection ----------------------------------------

    #[tokio::test]
    async fn concurrent_requests_never_share_checkout() {
        let provider = test_provider();
        let mut leases = Vec::new();

        for i in 0..10 {
            let id = attempt("plan-1", &format!("task-{i}"), 0);
            leases.push(provider.acquire(&id).await.unwrap());
        }

        // Verify all paths are unique.
        let paths: std::collections::HashSet<_> =
            leases.iter().map(|l| l.path.clone()).collect();
        assert_eq!(paths.len(), 10);

        // Verify all branches are unique.
        let branches: std::collections::HashSet<_> =
            leases.iter().map(|l| l.branch.clone()).collect();
        assert_eq!(branches.len(), 10);
    }

    // -- Reconcile ------------------------------------------------------------

    #[tokio::test]
    async fn reconcile_live_lease() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let result = provider.reconcile(&lease).await.unwrap();
        assert!(matches!(result, WorkspaceReconcileResult::Live(_)));
    }

    #[tokio::test]
    async fn reconcile_released_lease() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);
        let lease = provider.acquire(&id).await.unwrap();

        provider
            .release(&lease, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap();

        let result = provider.reconcile(&lease).await.unwrap();
        assert!(matches!(result, WorkspaceReconcileResult::AlreadyReleased));
    }

    #[tokio::test]
    async fn reconcile_retained_shows_orphaned() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);
        let lease = provider.acquire(&id).await.unwrap();

        provider
            .release(&lease, WorkspaceReleasePolicy::RetainForFailure)
            .await
            .unwrap();

        let result = provider.reconcile(&lease).await.unwrap();
        assert!(matches!(result, WorkspaceReconcileResult::Orphaned(_)));
    }

    #[tokio::test]
    async fn reconcile_unknown_lease_shows_orphaned() {
        let provider = test_provider();
        let fake_lease = WorkspaceLease {
            lease_id: "nonexistent".to_string(),
            attempt_id: attempt("ghost", "task", 0),
            path: PathBuf::from("/repo/.worktrees/ghost"),
            branch: "roko/attempt/ghost".to_string(),
            base_revision: "main".to_string(),
            lease_fingerprint: attempt("ghost", "task", 0).fingerprint(),
        };

        let result = provider.reconcile(&fake_lease).await.unwrap();
        assert!(matches!(result, WorkspaceReconcileResult::Orphaned(_)));
    }

    // -- Release --------------------------------------------------------------

    #[tokio::test]
    async fn release_delete_transitions_to_released() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let state = provider
            .release(&lease, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap();
        assert_eq!(state, WorkspaceLeaseState::Released);
    }

    #[tokio::test]
    async fn release_retain_for_failure_transitions_to_retained() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let state = provider
            .release(&lease, WorkspaceReleasePolicy::RetainForFailure)
            .await
            .unwrap();
        assert_eq!(state, WorkspaceLeaseState::Retained);
    }

    #[tokio::test]
    async fn release_retain_for_review_transitions_to_retained() {
        let provider = test_provider();
        let id = attempt("plan-1", "task-a", 0);
        let lease = provider.acquire(&id).await.unwrap();

        let state = provider
            .release(&lease, WorkspaceReleasePolicy::RetainForReview)
            .await
            .unwrap();
        assert_eq!(state, WorkspaceLeaseState::Retained);
    }

    #[tokio::test]
    async fn release_nonexistent_lease_errors() {
        let provider = test_provider();
        let fake_lease = WorkspaceLease {
            lease_id: "nonexistent".to_string(),
            attempt_id: attempt("ghost", "task", 0),
            path: PathBuf::from("/worktrees/ghost"),
            branch: "roko/attempt/ghost".to_string(),
            base_revision: "main".to_string(),
            lease_fingerprint: "deadbeef".to_string(),
        };

        let err = provider
            .release(&fake_lease, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::LeaseNotFound(_)));
    }

    // -- Reset for retry ------------------------------------------------------

    #[tokio::test]
    async fn reset_for_retry_retains_old_and_acquires_new() {
        let provider = test_provider();
        let first_id = attempt("plan-1", "task-a", 0);
        let next_id = attempt("plan-1", "task-a", 1);

        let first_lease = provider.acquire(&first_id).await.unwrap();
        let new_lease = provider
            .reset_for_retry(&first_lease, &next_id)
            .await
            .unwrap();

        // Old lease should be retained (not deleted).
        let old_result = provider.reconcile(&first_lease).await.unwrap();
        assert!(
            matches!(old_result, WorkspaceReconcileResult::Orphaned(_)),
            "old lease should be retained for failure"
        );

        // New lease should be active.
        let new_result = provider.reconcile(&new_lease).await.unwrap();
        assert!(matches!(new_result, WorkspaceReconcileResult::Live(_)));

        // Paths must differ.
        assert_ne!(first_lease.path, new_lease.path);
        assert_ne!(first_lease.branch, new_lease.branch);
    }

    #[tokio::test]
    async fn reset_for_retry_never_reuses_path() {
        let provider = test_provider();
        let id0 = attempt("plan-1", "task-a", 0);
        let id1 = attempt("plan-1", "task-a", 1);
        let id2 = attempt("plan-1", "task-a", 2);

        let lease0 = provider.acquire(&id0).await.unwrap();
        let lease1 = provider.reset_for_retry(&lease0, &id1).await.unwrap();
        let lease2 = provider.reset_for_retry(&lease1, &id2).await.unwrap();

        let all_paths: std::collections::HashSet<_> =
            [&lease0.path, &lease1.path, &lease2.path]
                .into_iter()
                .collect();
        assert_eq!(all_paths.len(), 3, "every retry must get a unique path");
    }

    // -- Budget after release -------------------------------------------------

    #[tokio::test]
    async fn released_lease_frees_budget_slot() {
        let provider = test_provider().with_max_live(1);
        let a = attempt("plan-1", "task-a", 0);
        let b = attempt("plan-1", "task-b", 0);

        let lease_a = provider.acquire(&a).await.unwrap();

        // Budget full — second acquire fails.
        assert!(provider.acquire(&b).await.is_err());

        // Release frees the slot.
        provider
            .release(&lease_a, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap();

        // Now acquire succeeds.
        let lease_b = provider.acquire(&b).await.unwrap();
        assert_ne!(lease_a.path, lease_b.path);
    }

    // -- Truly concurrent acquire (multi-task) --------------------------------

    #[tokio::test]
    async fn concurrent_spawned_acquires_never_share_checkout() {
        use std::sync::Arc;

        let provider = Arc::new(test_provider());

        // Spawn 20 concurrent acquire tasks for distinct attempts.
        let mut handles = Vec::new();
        for i in 0..20 {
            let p = Arc::clone(&provider);
            handles.push(tokio::spawn(async move {
                let id = WorkspaceAttemptId {
                    plan_id: "concurrent-plan".to_string(),
                    task_id: format!("task-{i}"),
                    attempt: 0,
                };
                p.acquire(&id).await.unwrap()
            }));
        }

        let leases: Vec<WorkspaceLease> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All 20 leases must have unique paths.
        let paths: std::collections::HashSet<_> =
            leases.iter().map(|l| l.path.clone()).collect();
        assert_eq!(paths.len(), 20, "concurrent acquires must yield unique paths");

        // All 20 leases must have unique branches.
        let branches: std::collections::HashSet<_> =
            leases.iter().map(|l| l.branch.clone()).collect();
        assert_eq!(
            branches.len(),
            20,
            "concurrent acquires must yield unique branches"
        );

        // All 20 leases must have unique lease IDs.
        let ids: std::collections::HashSet<_> =
            leases.iter().map(|l| l.lease_id.clone()).collect();
        assert_eq!(ids.len(), 20, "concurrent acquires must yield unique lease IDs");
    }

    // -- WorkspaceReconcileResult serde ---------------------------------------

    #[test]
    fn reconcile_result_serde_roundtrip() {
        let lease = WorkspaceLease {
            lease_id: "lease-99".to_string(),
            attempt_id: attempt("p", "t", 0),
            path: PathBuf::from("/worktrees/attempt-p-t-0"),
            branch: "roko/attempt/p/t/0".to_string(),
            base_revision: "main".to_string(),
            lease_fingerprint: attempt("p", "t", 0).fingerprint(),
        };

        let cases: Vec<WorkspaceReconcileResult> = vec![
            WorkspaceReconcileResult::Live(lease.clone()),
            WorkspaceReconcileResult::AlreadyReleased,
            WorkspaceReconcileResult::Conflict("detached HEAD".to_string()),
            WorkspaceReconcileResult::Orphaned(lease),
        ];

        for original in &cases {
            let json = serde_json::to_string(original).unwrap();
            let back: WorkspaceReconcileResult = serde_json::from_str(&json).unwrap();
            assert_eq!(*original, back, "round-trip failed for {json}");
        }
    }

    // -- WorkspaceReleasePolicy Display ---------------------------------------

    #[test]
    fn release_policy_display() {
        assert_eq!(WorkspaceReleasePolicy::Delete.to_string(), "delete");
        assert_eq!(
            WorkspaceReleasePolicy::RetainForFailure.to_string(),
            "retain_for_failure"
        );
        assert_eq!(
            WorkspaceReleasePolicy::RetainForReview.to_string(),
            "retain_for_review"
        );
    }

    // -- reconcile_all --------------------------------------------------------

    #[tokio::test]
    async fn reconcile_all_returns_all_tracked_leases() {
        let provider = test_provider();

        // Acquire three leases.
        let a = attempt("plan-1", "task-a", 0);
        let b = attempt("plan-1", "task-b", 0);
        let c = attempt("plan-1", "task-c", 0);

        provider.acquire(&a).await.unwrap();
        let lease_b = provider.acquire(&b).await.unwrap();
        provider.acquire(&c).await.unwrap();

        // Release one with Delete, one with RetainForFailure.
        provider
            .release(&lease_b, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap();

        let results = provider.reconcile_all().await;
        assert_eq!(results.len(), 3, "reconcile_all must return all tracked leases");

        // Verify the mix of states.
        let live_count = results
            .iter()
            .filter(|(_, r)| matches!(r, WorkspaceReconcileResult::Live(_)))
            .count();
        let released_count = results
            .iter()
            .filter(|(_, r)| matches!(r, WorkspaceReconcileResult::AlreadyReleased))
            .count();

        assert_eq!(live_count, 2, "two leases should be live");
        assert_eq!(released_count, 1, "one lease should be released");
    }

    // -- cleanup_orphans ------------------------------------------------------

    #[tokio::test]
    async fn cleanup_orphans_releases_retained_leases() {
        let provider = test_provider();

        let a = attempt("plan-1", "task-a", 0);
        let b = attempt("plan-1", "task-b", 0);
        let c = attempt("plan-1", "task-c", 0);

        let lease_a = provider.acquire(&a).await.unwrap();
        provider.acquire(&b).await.unwrap(); // stays active
        let lease_c = provider.acquire(&c).await.unwrap();

        // Retain two leases (simulating failures).
        provider
            .release(&lease_a, WorkspaceReleasePolicy::RetainForFailure)
            .await
            .unwrap();
        provider
            .release(&lease_c, WorkspaceReleasePolicy::RetainForReview)
            .await
            .unwrap();

        // cleanup_orphans should release the two retained leases.
        let cleaned = provider.cleanup_orphans().await;
        assert_eq!(cleaned.len(), 2, "two retained leases should be cleaned up");

        // After cleanup, only one active lease should remain.
        let active = provider.active_leases();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].attempt_id, b);
    }

    #[tokio::test]
    async fn cleanup_orphans_skips_active_and_released() {
        let provider = test_provider();

        let a = attempt("plan-1", "task-a", 0);
        let b = attempt("plan-1", "task-b", 0);

        provider.acquire(&a).await.unwrap(); // stays active
        let lease_b = provider.acquire(&b).await.unwrap();
        provider
            .release(&lease_b, WorkspaceReleasePolicy::Delete)
            .await
            .unwrap(); // already released

        // No retained leases exist, so cleanup should return empty.
        let cleaned = provider.cleanup_orphans().await;
        assert!(
            cleaned.is_empty(),
            "cleanup_orphans must not touch active or released leases"
        );
    }
}
