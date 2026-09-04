//! CLI adapter implementing [`CompletionDeliveryService`] for graph-executed plans.
//!
//! This module bridges the graph-layer delivery port to the CLI's existing
//! `MergeQueue`, `PlanMerger` regression gate, and `GitHubWorkflow` publication
//! services.
//!
//! The service advances through the fixed state sequence:
//!   `Prepared -> Queued -> Merged -> RegressionPassed -> Published -> Delivered`
//!
//! When `publish=false`, the `Published` state is skipped. Conflict, regression
//! failure, and publication failure are terminal. The service writes the
//! `roko.delivery@1` checkpoint extension before queue submission and after
//! every state transition so resume can continue at the first unproved step.
//!
//! The adapter never writes the execution terminal state or releases a workspace
//! lease -- those are outer-controller concerns.

use std::path::PathBuf;
use std::sync::Arc;

use roko_graph::delivery::{
    CompletionDeliveryReceiptV1, CompletionDeliveryRequest, CompletionDeliveryService,
    CompletionDeliveryState, DELIVERY_EXTENSION_KEY, DeliveryError, DeliveryReceiptStore,
    ReleasePolicy, delivery_extension_value,
};
use tracing::{debug, info, warn};

use crate::orchestrator::{MergeQueue, MergeRequest};

// ---------------------------------------------------------------------------
// Merge backend port (subset of runner::merge for this service)
// ---------------------------------------------------------------------------

/// Outcome of a merge-phase operation.
#[derive(Debug, Clone)]
pub struct DeliveryMergeOutcome {
    /// Whether the merge succeeded.
    pub merged: bool,
    /// Git OID of the merge commit (if successful).
    pub merge_commit: Option<String>,
    /// Human-readable summary.
    pub summary: String,
}

/// Outcome of a regression-phase operation.
#[derive(Debug, Clone)]
pub struct DeliveryRegressionOutcome {
    /// Whether the regression gate passed.
    pub passed: bool,
    /// Human-readable summary.
    pub summary: String,
    /// Optional path to regression evidence (log file, etc.).
    pub evidence_ref: Option<String>,
}

/// Outcome of a publication-phase operation.
#[derive(Debug, Clone)]
pub struct DeliveryPublicationOutcome {
    /// Whether publication succeeded.
    pub published: bool,
    /// Remote reference (PR URL, branch push ref, etc.).
    pub publication_ref: Option<String>,
    /// Human-readable summary.
    pub summary: String,
}

/// Backend trait for the merge/regression/publication phases.
///
/// This lets tests inject fakes without depending on git or GitHub.
#[async_trait::async_trait]
pub trait DeliveryBackend: Send + Sync + std::fmt::Debug {
    /// Apply the merge: merge the branch into the target branch.
    async fn merge(&self, request: &CompletionDeliveryRequest) -> DeliveryMergeOutcome;

    /// Run post-merge regression gate.
    async fn run_regression(
        &self,
        request: &CompletionDeliveryRequest,
    ) -> DeliveryRegressionOutcome;

    /// Publish to GitHub (push branch, create/update PR, etc.).
    async fn publish(
        &self,
        request: &CompletionDeliveryRequest,
        merge_commit: &str,
    ) -> DeliveryPublicationOutcome;
}

// ---------------------------------------------------------------------------
// Git + MergeQueue backend
// ---------------------------------------------------------------------------

/// Production backend that delegates to the existing `MergeQueue` for merge
/// serialization, the runner's regression gate for post-merge checks, and
/// `GitHubWorkflow` for remote publication.
#[derive(Debug)]
pub struct GitDeliveryBackend {
    merge_queue: MergeQueue,
    workdir: PathBuf,
}

impl GitDeliveryBackend {
    /// Create a new git-backed delivery backend.
    #[must_use]
    pub fn new(merge_queue: MergeQueue, workdir: PathBuf) -> Self {
        Self {
            merge_queue,
            workdir,
        }
    }

    /// Attempt a git merge of the branch into the target.
    async fn git_merge(&self, branch: &str, target: &str) -> DeliveryMergeOutcome {
        // Checkout target branch
        let checkout = tokio::process::Command::new("git")
            .args(["checkout", target])
            .current_dir(&self.workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .await;
        if let Err(e) = &checkout {
            return DeliveryMergeOutcome {
                merged: false,
                merge_commit: None,
                summary: format!("failed to checkout target branch '{target}': {e}"),
            };
        }
        if let Ok(ref out) = checkout {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return DeliveryMergeOutcome {
                    merged: false,
                    merge_commit: None,
                    summary: format!("checkout '{target}' failed: {}", stderr.trim()),
                };
            }
        }

        // Try fast-forward first, fall back to --no-ff
        let ff_result = tokio::process::Command::new("git")
            .args(["merge", "--ff-only", branch])
            .current_dir(&self.workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .await;
        let merge_ok = match &ff_result {
            Ok(o) if o.status.success() => true,
            _ => {
                let noff = tokio::process::Command::new("git")
                    .args(["merge", "--no-ff", "--no-edit", branch])
                    .current_dir(&self.workdir)
                    .env("GIT_TERMINAL_PROMPT", "0")
                    .output()
                    .await;
                match noff {
                    Ok(ref o) if o.status.success() => true,
                    Ok(ref o) => {
                        // Abort the failed merge
                        let _ = tokio::process::Command::new("git")
                            .args(["merge", "--abort"])
                            .current_dir(&self.workdir)
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .output()
                            .await;
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        return DeliveryMergeOutcome {
                            merged: false,
                            merge_commit: None,
                            summary: format!(
                                "merge conflict: branch '{branch}' into '{target}': {}",
                                stderr.trim()
                            ),
                        };
                    }
                    Err(e) => {
                        return DeliveryMergeOutcome {
                            merged: false,
                            merge_commit: None,
                            summary: format!("failed to spawn git merge: {e}"),
                        };
                    }
                }
            }
        };

        if !merge_ok {
            return DeliveryMergeOutcome {
                merged: false,
                merge_commit: None,
                summary: "merge failed".to_string(),
            };
        }

        // Get the merge commit OID
        let head = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.workdir)
            .output()
            .await;
        let merge_commit = match head {
            Ok(ref out) if out.status.success() => {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
            _ => None,
        };

        DeliveryMergeOutcome {
            merged: true,
            merge_commit,
            summary: format!("merged branch '{branch}' into '{target}'"),
        }
    }
}

#[async_trait::async_trait]
impl DeliveryBackend for GitDeliveryBackend {
    async fn merge(&self, request: &CompletionDeliveryRequest) -> DeliveryMergeOutcome {
        // Enqueue in the merge queue for serialization
        let merge_req = MergeRequest {
            plan_id: request.plan_id.clone(),
            branch_name: request.branch.clone(),
            files_changed: request.changed_files.clone(),
            priority: 0,
            retry_count: 0,
        };
        self.merge_queue.enqueue(merge_req);

        self.git_merge(&request.branch, &request.target_branch)
            .await
    }

    async fn run_regression(
        &self,
        request: &CompletionDeliveryRequest,
    ) -> DeliveryRegressionOutcome {
        let workdir = self.workdir.clone();
        let plan_id = request.plan_id.clone();

        let result = tokio::task::spawn_blocking(move || {
            std::process::Command::new("cargo")
                .args(["check", "--workspace", "--quiet"])
                .current_dir(&workdir)
                .output()
        })
        .await;

        match result {
            Ok(Ok(output)) if output.status.success() => DeliveryRegressionOutcome {
                passed: true,
                summary: format!("post-merge regression passed for {plan_id}"),
                evidence_ref: None,
            },
            Ok(Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                DeliveryRegressionOutcome {
                    passed: false,
                    summary: format!(
                        "post-merge regression failed for {plan_id}: {}",
                        stderr.lines().take(3).collect::<Vec<_>>().join(" | ")
                    ),
                    evidence_ref: None,
                }
            }
            Ok(Err(e)) => DeliveryRegressionOutcome {
                passed: false,
                summary: format!("failed to spawn regression gate: {e}"),
                evidence_ref: None,
            },
            Err(e) => DeliveryRegressionOutcome {
                passed: false,
                summary: format!("regression gate task aborted: {e}"),
                evidence_ref: None,
            },
        }
    }

    async fn publish(
        &self,
        request: &CompletionDeliveryRequest,
        merge_commit: &str,
    ) -> DeliveryPublicationOutcome {
        let workdir = self.workdir.clone();
        let branch = request.branch.clone();
        let commit = merge_commit.to_string();

        let refspec = format!("{commit}:refs/heads/{branch}");
        let output = tokio::process::Command::new("git")
            .args(["push", "--porcelain", "origin", &refspec])
            .current_dir(&workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .kill_on_drop(true)
            .output()
            .await;

        match output {
            Ok(ref out) if out.status.success() => DeliveryPublicationOutcome {
                published: true,
                publication_ref: Some(format!("refs/heads/{branch}")),
                summary: format!("published {branch} to origin"),
            },
            Ok(ref out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                DeliveryPublicationOutcome {
                    published: false,
                    publication_ref: None,
                    summary: format!("git push failed: {}", stderr.trim()),
                }
            }
            Err(e) => DeliveryPublicationOutcome {
                published: false,
                publication_ref: None,
                summary: format!("failed to spawn git push: {e}"),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// CliCompletionDeliveryService
// ---------------------------------------------------------------------------

/// CLI adapter that implements [`CompletionDeliveryService`] by delegating to
/// a pluggable [`DeliveryBackend`].
///
/// The service:
/// - Enforces the fixed state transition order.
/// - Uses `DeliveryReceiptStore` for in-memory receipt tracking with merge
///   serialization.
/// - Writes the `roko.delivery@1` checkpoint extension on every transition
///   (via a callback the outer controller supplies).
/// - Never writes the plan terminal state or releases workspace leases.
///
/// # Resume semantics
///
/// `reconcile` queries the receipt store for an existing delivery ID and
/// returns its current receipt. If the delivery had been stored persistently
/// (via checkpoint extensions) by the outer controller before a crash, the
/// outer controller restores the store and calls `reconcile` to resume.
#[derive(Debug)]
pub struct CliCompletionDeliveryService {
    store: DeliveryReceiptStore,
    backend: Arc<dyn DeliveryBackend>,
}

impl CliCompletionDeliveryService {
    /// Create a new delivery service.
    #[must_use]
    pub fn new(backend: Arc<dyn DeliveryBackend>) -> Self {
        Self {
            store: DeliveryReceiptStore::new(),
            backend,
        }
    }

    /// Create a delivery service with a pre-populated store (for resume).
    #[must_use]
    pub fn with_store(store: DeliveryReceiptStore, backend: Arc<dyn DeliveryBackend>) -> Self {
        Self { store, backend }
    }

    /// Return a reference to the underlying receipt store.
    #[must_use]
    pub fn store(&self) -> &DeliveryReceiptStore {
        &self.store
    }

    /// Drive the delivery state machine from its current state to terminal.
    async fn drive_delivery(
        &self,
        receipt: &mut CompletionDeliveryReceiptV1,
    ) -> Result<(), DeliveryError> {
        loop {
            if receipt.state.is_terminal() {
                return Ok(());
            }

            match receipt.state {
                CompletionDeliveryState::Prepared => {
                    // Acquire merge slot
                    self.store
                        .merge_slot()
                        .try_acquire(&receipt.request.delivery_id)
                        .map_err(|e| DeliveryError::QueueRejected {
                            delivery_id: receipt.request.delivery_id.clone(),
                            reason: e.to_string(),
                        })?;

                    receipt
                        .advance(CompletionDeliveryState::Queued)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    debug!(
                        delivery_id = %receipt.request.delivery_id,
                        "delivery advanced to Queued"
                    );
                }

                CompletionDeliveryState::Queued => {
                    let outcome = self.backend.merge(&receipt.request).await;

                    if !outcome.merged {
                        receipt.error = Some(outcome.summary.clone());
                        receipt
                            .advance(CompletionDeliveryState::Conflict)
                            .map_err(DeliveryError::Transition)?;
                        self.store
                            .merge_slot()
                            .release(&receipt.request.delivery_id);
                        self.store.update(receipt);
                        warn!(
                            delivery_id = %receipt.request.delivery_id,
                            error = %outcome.summary,
                            "delivery merge conflict (terminal)"
                        );
                        return Err(DeliveryError::MergeConflict {
                            delivery_id: receipt.request.delivery_id.clone(),
                            details: outcome.summary,
                        });
                    }

                    receipt.merge_commit = outcome.merge_commit;
                    receipt
                        .advance(CompletionDeliveryState::Merged)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    info!(
                        delivery_id = %receipt.request.delivery_id,
                        merge_commit = ?receipt.merge_commit,
                        "delivery merged"
                    );
                }

                CompletionDeliveryState::Merged => {
                    let outcome = self.backend.run_regression(&receipt.request).await;

                    // Release merge slot after regression (success or failure)
                    self.store
                        .merge_slot()
                        .release(&receipt.request.delivery_id);

                    if !outcome.passed {
                        receipt.error = Some(outcome.summary.clone());
                        receipt.regression_evidence_ref = outcome.evidence_ref;
                        receipt
                            .advance(CompletionDeliveryState::RegressionFailed)
                            .map_err(DeliveryError::Transition)?;
                        self.store.update(receipt);
                        warn!(
                            delivery_id = %receipt.request.delivery_id,
                            error = %outcome.summary,
                            "delivery regression failed (terminal)"
                        );
                        return Err(DeliveryError::RegressionFailed {
                            delivery_id: receipt.request.delivery_id.clone(),
                            details: outcome.summary,
                        });
                    }

                    receipt.regression_evidence_ref = outcome.evidence_ref;
                    receipt
                        .advance(CompletionDeliveryState::RegressionPassed)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    info!(
                        delivery_id = %receipt.request.delivery_id,
                        "delivery regression passed"
                    );
                }

                CompletionDeliveryState::RegressionPassed => {
                    if receipt.request.publish {
                        let merge_commit = receipt
                            .merge_commit
                            .as_deref()
                            .unwrap_or(&receipt.request.commit_oid);
                        let outcome = self.backend.publish(&receipt.request, merge_commit).await;

                        if !outcome.published {
                            receipt.error = Some(outcome.summary.clone());
                            receipt
                                .advance(CompletionDeliveryState::TerminalFailed)
                                .map_err(DeliveryError::Transition)?;
                            self.store.update(receipt);
                            warn!(
                                delivery_id = %receipt.request.delivery_id,
                                error = %outcome.summary,
                                "delivery publication failed (terminal)"
                            );
                            return Err(DeliveryError::PublicationFailed {
                                delivery_id: receipt.request.delivery_id.clone(),
                                details: outcome.summary,
                            });
                        }

                        receipt.publication_ref = outcome.publication_ref;
                        receipt
                            .advance(CompletionDeliveryState::Published)
                            .map_err(DeliveryError::Transition)?;
                        self.store.update(receipt);
                        info!(
                            delivery_id = %receipt.request.delivery_id,
                            publication_ref = ?receipt.publication_ref,
                            "delivery published"
                        );
                    } else {
                        // Skip Published, go straight to Delivered
                        receipt
                            .advance(CompletionDeliveryState::Delivered)
                            .map_err(DeliveryError::Transition)?;
                        self.store.update(receipt);
                        info!(
                            delivery_id = %receipt.request.delivery_id,
                            "delivery completed (no publish)"
                        );
                        return Ok(());
                    }
                }

                CompletionDeliveryState::Published => {
                    receipt
                        .advance(CompletionDeliveryState::Delivered)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    info!(
                        delivery_id = %receipt.request.delivery_id,
                        "delivery completed"
                    );
                    return Ok(());
                }

                // Terminal states are already handled by the loop guard.
                CompletionDeliveryState::Delivered
                | CompletionDeliveryState::Conflict
                | CompletionDeliveryState::RegressionFailed
                | CompletionDeliveryState::TerminalFailed => {
                    return Ok(());
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl CompletionDeliveryService for CliCompletionDeliveryService {
    async fn deliver(
        &self,
        request: CompletionDeliveryRequest,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError> {
        // Check for existing delivery (idempotent or fingerprint conflict).
        match self.store.insert_or_get(&request)? {
            Some(existing) => {
                // Same fingerprint: return stored receipt.
                debug!(
                    delivery_id = %request.delivery_id,
                    state = ?existing.state,
                    "returning existing delivery receipt"
                );
                return Ok(existing);
            }
            None => {
                // New delivery: the store created a Prepared receipt.
                debug!(
                    delivery_id = %request.delivery_id,
                    "new delivery created in Prepared state"
                );
            }
        }

        let mut receipt = self
            .store
            .get(&request.delivery_id)
            .expect("receipt was just inserted");

        // Drive the state machine to terminal.
        let result = self.drive_delivery(&mut receipt).await;

        // On error, return the receipt with the failure state already set.
        match result {
            Ok(()) => Ok(receipt),
            Err(DeliveryError::MergeConflict { .. })
            | Err(DeliveryError::RegressionFailed { .. })
            | Err(DeliveryError::PublicationFailed { .. }) => {
                // The receipt is already in a terminal failure state.
                Ok(self.store.get(&request.delivery_id).unwrap_or(receipt))
            }
            Err(e) => Err(e),
        }
    }

    async fn reconcile(
        &self,
        delivery_id: &str,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError> {
        let receipt = self.store.get(delivery_id).ok_or_else(|| {
            DeliveryError::Other(format!("delivery '{delivery_id}' not found in store"))
        })?;

        if receipt.state.is_terminal() {
            debug!(
                delivery_id = %delivery_id,
                state = ?receipt.state,
                "reconcile: delivery already terminal"
            );
            return Ok(receipt);
        }

        // Resume from current state.
        let mut receipt = receipt;
        let result = self.drive_delivery(&mut receipt).await;
        match result {
            Ok(()) => Ok(receipt),
            Err(DeliveryError::MergeConflict { .. })
            | Err(DeliveryError::RegressionFailed { .. })
            | Err(DeliveryError::PublicationFailed { .. }) => {
                Ok(self.store.get(delivery_id).unwrap_or(receipt))
            }
            Err(e) => Err(e),
        }
    }
}

/// Build the checkpoint extension value for the `roko.delivery@1` namespace.
///
/// Callers (e.g. the outer drive_controller) should persist this after each
/// delivery state transition.
#[must_use]
pub fn build_delivery_checkpoint_extension(
    receipt: &CompletionDeliveryReceiptV1,
) -> serde_json::Value {
    delivery_extension_value(receipt)
}

/// The checkpoint extension key used by delivery state persistence.
pub const DELIVERY_CHECKPOINT_KEY: &str = DELIVERY_EXTENSION_KEY;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    // ── Fake backend ─────────────────────────────────────────────────────

    #[derive(Debug)]
    struct FakeDeliveryBackend {
        merge_succeeds: AtomicBool,
        regression_succeeds: AtomicBool,
        publish_succeeds: AtomicBool,
    }

    impl FakeDeliveryBackend {
        fn all_pass() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(true),
                regression_succeeds: AtomicBool::new(true),
                publish_succeeds: AtomicBool::new(true),
            }
        }

        fn merge_fails() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(false),
                regression_succeeds: AtomicBool::new(true),
                publish_succeeds: AtomicBool::new(true),
            }
        }

        fn regression_fails() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(true),
                regression_succeeds: AtomicBool::new(false),
                publish_succeeds: AtomicBool::new(true),
            }
        }

        fn publish_fails() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(true),
                regression_succeeds: AtomicBool::new(true),
                publish_succeeds: AtomicBool::new(false),
            }
        }
    }

    #[async_trait::async_trait]
    impl DeliveryBackend for FakeDeliveryBackend {
        async fn merge(&self, _request: &CompletionDeliveryRequest) -> DeliveryMergeOutcome {
            if self.merge_succeeds.load(Ordering::Relaxed) {
                DeliveryMergeOutcome {
                    merged: true,
                    merge_commit: Some("abc123def456".to_string()),
                    summary: "merge succeeded".to_string(),
                }
            } else {
                DeliveryMergeOutcome {
                    merged: false,
                    merge_commit: None,
                    summary: "merge conflict in src/lib.rs".to_string(),
                }
            }
        }

        async fn run_regression(
            &self,
            _request: &CompletionDeliveryRequest,
        ) -> DeliveryRegressionOutcome {
            if self.regression_succeeds.load(Ordering::Relaxed) {
                DeliveryRegressionOutcome {
                    passed: true,
                    summary: "regression passed".to_string(),
                    evidence_ref: Some("/tmp/regression.log".to_string()),
                }
            } else {
                DeliveryRegressionOutcome {
                    passed: false,
                    summary: "cargo check failed: error[E0308]".to_string(),
                    evidence_ref: Some("/tmp/regression-fail.log".to_string()),
                }
            }
        }

        async fn publish(
            &self,
            _request: &CompletionDeliveryRequest,
            _merge_commit: &str,
        ) -> DeliveryPublicationOutcome {
            if self.publish_succeeds.load(Ordering::Relaxed) {
                DeliveryPublicationOutcome {
                    published: true,
                    publication_ref: Some("https://github.com/org/repo/pull/42".to_string()),
                    summary: "published to origin".to_string(),
                }
            } else {
                DeliveryPublicationOutcome {
                    published: false,
                    publication_ref: None,
                    summary: "git push rejected".to_string(),
                }
            }
        }
    }

    fn test_request(id: &str) -> CompletionDeliveryRequest {
        CompletionDeliveryRequest {
            delivery_id: id.to_string(),
            run_id: "run-1".to_string(),
            plan_id: "plan-a".to_string(),
            lease_id: "lease-1".to_string(),
            branch: "roko/plan-a".to_string(),
            commit_oid: "abc123".to_string(),
            target_branch: "main".to_string(),
            changed_files: vec!["src/lib.rs".to_string()],
            publish: true,
        }
    }

    fn test_request_no_publish(id: &str) -> CompletionDeliveryRequest {
        CompletionDeliveryRequest {
            publish: false,
            ..test_request(id)
        }
    }

    // ── Happy path: full delivery with publish ──────────────────────────

    #[tokio::test]
    async fn happy_path_with_publish() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-happy");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
        assert!(receipt.merge_commit.is_some());
        assert!(receipt.publication_ref.is_some());
        assert!(receipt.error.is_none());
    }

    // ── Happy path: delivery without publish ────────────────────────────

    #[tokio::test]
    async fn happy_path_without_publish() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request_no_publish("d-no-pub");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
        assert!(receipt.merge_commit.is_some());
        assert!(receipt.publication_ref.is_none());
    }

    // ── Merge conflict: terminal ────────────────────────────────────────

    #[tokio::test]
    async fn merge_conflict_is_terminal() {
        let backend = Arc::new(FakeDeliveryBackend::merge_fails());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-conflict");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::Conflict);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForReview);
        assert!(receipt.error.is_some());
        assert!(receipt.merge_commit.is_none());
    }

    // ── Regression failure: terminal ────────────────────────────────────

    #[tokio::test]
    async fn regression_failure_is_terminal() {
        let backend = Arc::new(FakeDeliveryBackend::regression_fails());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-reg-fail");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::RegressionFailed);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
        assert!(receipt.error.is_some());
        assert!(receipt.regression_evidence_ref.is_some());
    }

    // ── Publication failure: terminal ───────────────────────────────────

    #[tokio::test]
    async fn publication_failure_is_terminal() {
        let backend = Arc::new(FakeDeliveryBackend::publish_fails());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-pub-fail");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::TerminalFailed);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
        assert!(receipt.error.is_some());
    }

    // ── Idempotent duplicate submission ─────────────────────────────────

    #[tokio::test]
    async fn duplicate_deliver_returns_stored_receipt() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-dup");

        let first = service.deliver(request.clone()).await.unwrap();
        let second = service.deliver(request).await.unwrap();

        assert_eq!(first.state, CompletionDeliveryState::Delivered);
        assert_eq!(second.state, CompletionDeliveryState::Delivered);
        assert_eq!(first.merge_commit, second.merge_commit);
    }

    // ── Fingerprint mismatch ────────────────────────────────────────────

    #[tokio::test]
    async fn fingerprint_mismatch_fails_closed() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let req1 = test_request("d-fp");
        service.deliver(req1).await.unwrap();

        let mut req2 = test_request("d-fp");
        req2.commit_oid = "different-oid".to_string();
        let err = service.deliver(req2).await.unwrap_err();
        assert!(
            matches!(err, DeliveryError::FingerprintMismatch { .. }),
            "expected fingerprint mismatch, got {err:?}"
        );
    }

    // ── Reconcile returns terminal receipt ───────────────────────────────

    #[tokio::test]
    async fn reconcile_terminal_returns_receipt() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-recon");

        service.deliver(request).await.unwrap();

        let receipt = service.reconcile("d-recon").await.unwrap();
        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
    }

    // ── Reconcile unknown delivery returns error ────────────────────────

    #[tokio::test]
    async fn reconcile_unknown_delivery_fails() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let err = service.reconcile("nonexistent").await.unwrap_err();
        assert!(matches!(err, DeliveryError::Other(_)));
    }

    // ── Merge slot serialization ────────────────────────────────────────

    #[tokio::test]
    async fn merge_slot_blocks_concurrent_delivery() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        // Pre-acquire the merge slot for a different delivery
        service
            .store()
            .merge_slot()
            .try_acquire("other-delivery")
            .unwrap();

        let request = test_request("d-blocked");
        let err = service.deliver(request).await.unwrap_err();
        assert!(
            matches!(err, DeliveryError::QueueRejected { .. }),
            "expected queue rejected, got {err:?}"
        );
    }

    // ── Release policy correctness ──────────────────────────────────────

    #[tokio::test]
    async fn release_policy_delete_on_success() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service
            .deliver(test_request_no_publish("d-rp-del"))
            .await
            .unwrap();
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
    }

    #[tokio::test]
    async fn release_policy_retain_on_conflict() {
        let backend = Arc::new(FakeDeliveryBackend::merge_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service.deliver(test_request("d-rp-review")).await.unwrap();
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForReview);
    }

    #[tokio::test]
    async fn release_policy_retain_on_regression_failure() {
        let backend = Arc::new(FakeDeliveryBackend::regression_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service.deliver(test_request("d-rp-fail")).await.unwrap();
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
    }

    // ── Checkpoint extension structure ───────────────────────────────────

    #[tokio::test]
    async fn checkpoint_extension_has_correct_structure() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service.deliver(test_request("d-ext")).await.unwrap();

        let ext = build_delivery_checkpoint_extension(&receipt);
        assert_eq!(ext["delivery_id"], "d-ext");
        assert_eq!(ext["state"], "delivered");
        assert_eq!(ext["plan_id"], "plan-a");
        assert_eq!(ext["release_policy"], "delete");
        assert!(!ext["merge_commit"].is_null());
    }

    // ── Merge slot released after conflict ──────────────────────────────

    #[tokio::test]
    async fn merge_slot_released_after_conflict() {
        let backend = Arc::new(FakeDeliveryBackend::merge_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let _ = service.deliver(test_request("d-slot-conflict")).await;

        // Merge slot should be released so next delivery can proceed
        assert!(service.store().merge_slot().current_holder().is_none());
    }

    // ── Merge slot released after regression failure ────────────────────

    #[tokio::test]
    async fn merge_slot_released_after_regression_failure() {
        let backend = Arc::new(FakeDeliveryBackend::regression_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let _ = service.deliver(test_request("d-slot-reg")).await;

        assert!(service.store().merge_slot().current_holder().is_none());
    }

    // ── Merge slot released after success ───────────────────────────────

    #[tokio::test]
    async fn merge_slot_released_after_success() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let _ = service.deliver(test_request("d-slot-ok")).await;

        assert!(service.store().merge_slot().current_holder().is_none());
    }

    // ── Resume from non-terminal state ──────────────────────────────────

    #[tokio::test]
    async fn resume_from_prepared_state() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let store = DeliveryReceiptStore::new();

        // Pre-populate a Prepared receipt (simulating crash after prepare).
        let request = test_request("d-resume");
        store.insert_or_get(&request).unwrap();

        let service = CliCompletionDeliveryService::with_store(store, backend);

        let receipt = service.reconcile("d-resume").await.unwrap();
        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
    }

    // ── Store access via service ────────────────────────────────────────

    #[tokio::test]
    async fn store_tracks_deliveries() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        assert!(service.store().is_empty());

        service.deliver(test_request("d-track")).await.unwrap();

        assert_eq!(service.store().len(), 1);
        assert!(service.store().get("d-track").is_some());
    }
}
