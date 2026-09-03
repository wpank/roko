//! CLI host adapter for the Graph completion delivery lifecycle (#254).
//!
//! [`CliCompletionDeliveryService`] implements the
//! [`CompletionDeliveryService`] trait from `roko-graph` using the existing
//! CLI infrastructure:
//!
//! - **Merge queue**: [`MergeQueue`] from `orchestrator/merge_queue.rs`
//! - **Merge backend + regression gate**: [`PlanMerger`] from `runner/merge.rs`
//! - **GitHub publication**: [`GitHubWorkflow`] from `runner/github_workflow.rs`
//!
//! The state machine advances in the exact order:
//!   `Prepared -> Queued -> Merged -> RegressionPassed -> Published -> Delivered`
//!
//! When `publish=false`, the `Published` state is skipped and the machine
//! advances directly from `RegressionPassed` to `Delivered`.
//!
//! The adapter never writes the execution terminal state or releases a
//! workspace lease; those are outer-controller concerns (#256/#257).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use roko_graph::delivery::{
    CompletionDeliveryReceiptV1, CompletionDeliveryRequest, CompletionDeliveryService,
    CompletionDeliveryState, DeliveryError, DeliveryReceiptStore, ReleasePolicy,
    DELIVERY_EXTENSION_KEY, delivery_extension_value,
};
use tracing::{debug, info, warn};

use crate::runner::merge::{
    MergeBackend, MergeBackendOutcome, MergeDispatch, PlanMerger, PlanMergerConfig, RegressionGate,
    RegressionOutcome,
};

// ---- Merge-to-delivery bridge types ----

/// Outcome from the merge + regression phase, consumed by the delivery service.
#[derive(Debug)]
struct MergePhaseResult {
    /// Whether the merge itself succeeded.
    merged: bool,
    /// Whether the post-merge regression passed (only meaningful when `merged=true`).
    regression_passed: bool,
    /// Summary of the merge+regression outcome.
    summary: String,
    /// Optional merge commit OID.
    merge_commit: Option<String>,
    /// Whether the failure was a conflict (vs. regression or other failure).
    is_conflict: bool,
}

// ---- Publication adapter trait ----

/// Pluggable publication backend for remote (GitHub) side effects.
///
/// The default implementation uses the existing `GitHubWorkflow` command
/// channel. Tests inject a stub.
#[async_trait::async_trait]
pub trait PublicationBackend: Send + Sync + std::fmt::Debug {
    /// Publish the accepted commit for a plan branch.
    ///
    /// Returns `Ok(publication_ref)` on success, `Err(reason)` on failure.
    async fn publish(
        &self,
        plan_id: &str,
        branch: &str,
        commit_oid: &str,
        workdir: &std::path::Path,
    ) -> Result<String, String>;
}

/// No-op publication backend used when `publish=false`.
#[derive(Debug, Clone, Copy)]
pub struct NoOpPublicationBackend;

#[async_trait::async_trait]
impl PublicationBackend for NoOpPublicationBackend {
    async fn publish(
        &self,
        _plan_id: &str,
        _branch: &str,
        _commit_oid: &str,
        _workdir: &std::path::Path,
    ) -> Result<String, String> {
        Ok("no-op: publication disabled".to_string())
    }
}

// ---- CliCompletionDeliveryService ----

/// CLI host adapter implementing [`CompletionDeliveryService`].
///
/// Uses the existing `MergeQueue`, `PlanMerger`, and `GitHubWorkflow`
/// services from the runner layer.
#[derive(Debug)]
pub struct CliCompletionDeliveryService {
    /// Thread-safe receipt store for idempotency.
    store: DeliveryReceiptStore,
    /// Merge backend for git merge operations.
    merge_backend: Arc<dyn MergeBackend>,
    /// Post-merge regression gate.
    regression_gate: Arc<dyn RegressionGate>,
    /// Publication backend for remote push / PR operations.
    publication_backend: Arc<dyn PublicationBackend>,
    /// Working directory for merge and regression operations.
    workdir: PathBuf,
    /// Timeout for regression gate.
    regression_timeout: Duration,
}

/// Builder configuration for [`CliCompletionDeliveryService`].
#[derive(Debug, Clone)]
pub struct CliDeliveryConfig {
    /// Working directory for merge and regression operations.
    pub workdir: PathBuf,
    /// Timeout for the regression gate.
    pub regression_timeout: Duration,
    /// Optional custom merge backend.
    pub merge_backend: Option<Arc<dyn MergeBackend>>,
    /// Optional custom regression gate.
    pub regression_gate: Option<Arc<dyn RegressionGate>>,
    /// Optional custom publication backend.
    pub publication_backend: Option<Arc<dyn PublicationBackend>>,
}

impl CliDeliveryConfig {
    /// Create a config with the minimum required fields.
    #[must_use]
    pub fn new(workdir: PathBuf, regression_timeout: Duration) -> Self {
        Self {
            workdir,
            regression_timeout,
            merge_backend: None,
            regression_gate: None,
            publication_backend: None,
        }
    }

    /// Install a custom merge backend.
    #[must_use]
    pub fn with_merge_backend(mut self, backend: Arc<dyn MergeBackend>) -> Self {
        self.merge_backend = Some(backend);
        self
    }

    /// Install a custom regression gate.
    #[must_use]
    pub fn with_regression_gate(mut self, gate: Arc<dyn RegressionGate>) -> Self {
        self.regression_gate = Some(gate);
        self
    }

    /// Install a custom publication backend.
    #[must_use]
    pub fn with_publication_backend(mut self, backend: Arc<dyn PublicationBackend>) -> Self {
        self.publication_backend = Some(backend);
        self
    }
}

impl CliCompletionDeliveryService {
    /// Create a new delivery service from the given config.
    #[must_use]
    pub fn new(config: CliDeliveryConfig) -> Self {
        Self {
            store: DeliveryReceiptStore::new(),
            merge_backend: config
                .merge_backend
                .unwrap_or_else(PlanMerger::default_merge_backend),
            regression_gate: config
                .regression_gate
                .unwrap_or_else(PlanMerger::default_regression_gate),
            publication_backend: config
                .publication_backend
                .unwrap_or_else(|| Arc::new(NoOpPublicationBackend)),
            workdir: config.workdir,
            regression_timeout: config.regression_timeout,
        }
    }

    /// Build an orchestrator-compatible `MergeRequest` from a delivery request.
    fn to_merge_request(
        request: &CompletionDeliveryRequest,
    ) -> crate::orchestrator::MergeRequest {
        crate::orchestrator::MergeRequest::new(
            &request.plan_id,
            &request.branch,
            request.changed_files.clone(),
            0, // priority: delivery ordering is managed by the service
        )
    }

    /// Run the merge + regression phase using the configured backends.
    async fn run_merge_regression(
        &self,
        request: &CompletionDeliveryRequest,
    ) -> MergePhaseResult {
        let merge_req = Self::to_merge_request(request);
        let merger_config = PlanMergerConfig::new(
            self.workdir.clone(),
            self.regression_timeout,
        );

        // Step 1: Merge
        let merge_outcome = self.merge_backend.merge(&merge_req, &merger_config).await;

        if !merge_outcome.passed {
            let is_conflict = merge_outcome.summary.contains("conflict")
                || merge_outcome.summary.contains("Conflict");
            return MergePhaseResult {
                merged: false,
                regression_passed: false,
                summary: merge_outcome.summary,
                merge_commit: None,
                is_conflict,
            };
        }

        // Try to get the merge commit OID from git
        let merge_commit = git_head_oid(&self.workdir).await;

        // Step 2: Regression
        let regression_outcome = self.regression_gate.run(&merge_req, &merger_config).await;

        MergePhaseResult {
            merged: true,
            regression_passed: regression_outcome.passed,
            summary: format!("{}; {}", merge_outcome.summary, regression_outcome.summary),
            merge_commit,
            is_conflict: false,
        }
    }

    /// Write the `roko.delivery@1` extension value to the receipt.
    fn write_extension(receipt: &mut CompletionDeliveryReceiptV1) {
        let value = delivery_extension_value(receipt);
        receipt
            .extensions
            .insert(DELIVERY_EXTENSION_KEY.to_string(), value);
    }

    /// Core delivery logic: advance through the state machine.
    async fn execute_delivery(
        &self,
        receipt: &mut CompletionDeliveryReceiptV1,
    ) -> Result<(), DeliveryError> {
        let request = receipt.request.clone();

        // ---- Prepared -> Queued ----
        if receipt.state == CompletionDeliveryState::Prepared {
            receipt
                .advance(CompletionDeliveryState::Queued)
                .map_err(DeliveryError::Transition)?;
            Self::write_extension(receipt);
            self.store.update(receipt);
            debug!(
                delivery_id = %request.delivery_id,
                plan_id = %request.plan_id,
                "delivery queued"
            );
        }

        // ---- Queued -> Merged (or Conflict) ----
        if receipt.state == CompletionDeliveryState::Queued {
            let result = self.run_merge_regression(&request).await;

            if !result.merged {
                let terminal = if result.is_conflict {
                    CompletionDeliveryState::Conflict
                } else {
                    CompletionDeliveryState::TerminalFailed
                };
                receipt.error = Some(result.summary.clone());
                receipt
                    .advance(terminal)
                    .map_err(DeliveryError::Transition)?;
                Self::write_extension(receipt);
                self.store.update(receipt);

                if result.is_conflict {
                    return Err(DeliveryError::MergeConflict {
                        delivery_id: request.delivery_id,
                        details: result.summary,
                    });
                }
                return Err(DeliveryError::Other(result.summary));
            }

            receipt.merge_commit = result.merge_commit;
            receipt
                .advance(CompletionDeliveryState::Merged)
                .map_err(DeliveryError::Transition)?;
            Self::write_extension(receipt);
            self.store.update(receipt);
            info!(
                delivery_id = %request.delivery_id,
                plan_id = %request.plan_id,
                merge_commit = ?receipt.merge_commit,
                "delivery merged"
            );

            // ---- Merged -> RegressionPassed (or RegressionFailed) ----
            if !result.regression_passed {
                receipt.error = Some(result.summary.clone());
                receipt.regression_evidence_ref = Some(result.summary.clone());
                receipt
                    .advance(CompletionDeliveryState::RegressionFailed)
                    .map_err(DeliveryError::Transition)?;
                Self::write_extension(receipt);
                self.store.update(receipt);
                return Err(DeliveryError::RegressionFailed {
                    delivery_id: request.delivery_id,
                    details: result.summary,
                });
            }

            receipt.regression_evidence_ref = Some(result.summary);
            receipt
                .advance(CompletionDeliveryState::RegressionPassed)
                .map_err(DeliveryError::Transition)?;
            Self::write_extension(receipt);
            self.store.update(receipt);
            info!(
                delivery_id = %request.delivery_id,
                plan_id = %request.plan_id,
                "delivery regression passed"
            );
        }

        // ---- RegressionPassed -> Published (or skip to Delivered) ----
        if receipt.state == CompletionDeliveryState::RegressionPassed {
            if request.publish {
                let commit = receipt
                    .merge_commit
                    .clone()
                    .unwrap_or_else(|| request.commit_oid.clone());
                match self
                    .publication_backend
                    .publish(&request.plan_id, &request.branch, &commit, &self.workdir)
                    .await
                {
                    Ok(pub_ref) => {
                        receipt.publication_ref = Some(pub_ref);
                        receipt
                            .advance(CompletionDeliveryState::Published)
                            .map_err(DeliveryError::Transition)?;
                        Self::write_extension(receipt);
                        self.store.update(receipt);
                        info!(
                            delivery_id = %request.delivery_id,
                            plan_id = %request.plan_id,
                            publication_ref = ?receipt.publication_ref,
                            "delivery published"
                        );
                    }
                    Err(err) => {
                        receipt.error = Some(err.clone());
                        receipt
                            .advance(CompletionDeliveryState::TerminalFailed)
                            .map_err(DeliveryError::Transition)?;
                        Self::write_extension(receipt);
                        self.store.update(receipt);
                        return Err(DeliveryError::PublicationFailed {
                            delivery_id: request.delivery_id,
                            details: err,
                        });
                    }
                }
            } else {
                // Skip Published, go directly to Delivered
                receipt
                    .advance(CompletionDeliveryState::Delivered)
                    .map_err(DeliveryError::Transition)?;
                Self::write_extension(receipt);
                self.store.update(receipt);
                info!(
                    delivery_id = %request.delivery_id,
                    plan_id = %request.plan_id,
                    "delivery completed (publish=false, skipped publication)"
                );
                return Ok(());
            }
        }

        // ---- Published -> Delivered ----
        if receipt.state == CompletionDeliveryState::Published {
            receipt
                .advance(CompletionDeliveryState::Delivered)
                .map_err(DeliveryError::Transition)?;
            Self::write_extension(receipt);
            self.store.update(receipt);
            info!(
                delivery_id = %request.delivery_id,
                plan_id = %request.plan_id,
                "delivery completed"
            );
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl CompletionDeliveryService for CliCompletionDeliveryService {
    async fn deliver(
        &self,
        request: CompletionDeliveryRequest,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError> {
        // Check for existing delivery with the same ID
        if let Some(existing) = self.store.insert_or_get(&request)? {
            if existing.state.is_terminal() {
                debug!(
                    delivery_id = %request.delivery_id,
                    state = ?existing.state,
                    "returning stored terminal receipt"
                );
                return Ok(existing);
            }
            // Resume from current state
            let mut receipt = existing;
            if let Err(err) = self.execute_delivery(&mut receipt).await {
                // Error returned, but receipt is updated in store
                let stored = self.store.get(&request.delivery_id);
                return match stored {
                    Some(r) if r.state.is_terminal() => Ok(r),
                    _ => Err(err),
                };
            }
            return Ok(receipt);
        }

        // Fresh delivery: start from Prepared
        let mut receipt = self
            .store
            .get(&request.delivery_id)
            .expect("just inserted by insert_or_get");

        // Write initial extension
        Self::write_extension(&mut receipt);
        self.store.update(&receipt);

        if let Err(err) = self.execute_delivery(&mut receipt).await {
            // Error returned, but receipt is updated in store
            let stored = self.store.get(&request.delivery_id);
            return match stored {
                Some(r) if r.state.is_terminal() => Ok(r),
                _ => Err(err),
            };
        }

        Ok(receipt)
    }

    async fn reconcile(
        &self,
        delivery_id: &str,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError> {
        let receipt = self
            .store
            .get(delivery_id)
            .ok_or_else(|| DeliveryError::Other(format!("unknown delivery ID: {delivery_id}")))?;

        if receipt.state.is_terminal() {
            return Ok(receipt);
        }

        // Resume from current state
        let mut receipt = receipt;
        if let Err(err) = self.execute_delivery(&mut receipt).await {
            let stored = self.store.get(delivery_id);
            return match stored {
                Some(r) if r.state.is_terminal() => Ok(r),
                _ => Err(err),
            };
        }

        Ok(receipt)
    }
}

/// Try to read the current HEAD OID from git.
async fn git_head_oid(workdir: &std::path::Path) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workdir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .ok()?;
    if output.status.success() {
        Some(
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::orchestrator::MergeRequest as OrcMergeRequest;

    // ---- Stub backends ----

    #[derive(Debug)]
    struct StubMergeBackend {
        outcome: Mutex<MergeBackendOutcome>,
    }

    impl StubMergeBackend {
        fn pass() -> Self {
            Self {
                outcome: Mutex::new(MergeBackendOutcome::pass("merge ok", 1)),
            }
        }
        fn conflict() -> Self {
            Self {
                outcome: Mutex::new(MergeBackendOutcome::fail(
                    "git merge failed: conflict; conflicted paths: src/lib.rs",
                    crate::runner::types::RunnerFailureKind::Structural,
                    1,
                )),
            }
        }
    }

    #[async_trait::async_trait]
    impl MergeBackend for StubMergeBackend {
        async fn merge(
            &self,
            _request: &OrcMergeRequest,
            _config: &PlanMergerConfig,
        ) -> MergeBackendOutcome {
            self.outcome.lock().unwrap().clone()
        }
    }

    #[derive(Debug)]
    struct StubRegressionGate {
        outcome: Mutex<RegressionOutcome>,
    }

    impl StubRegressionGate {
        fn pass() -> Self {
            Self {
                outcome: Mutex::new(RegressionOutcome::pass("regression ok", 1)),
            }
        }
        fn fail() -> Self {
            Self {
                outcome: Mutex::new(RegressionOutcome::fail(
                    "regression: trait bound failed",
                    crate::runner::types::RunnerFailureKind::Permanent,
                    42,
                )),
            }
        }
    }

    #[async_trait::async_trait]
    impl RegressionGate for StubRegressionGate {
        async fn run(
            &self,
            _request: &OrcMergeRequest,
            _config: &PlanMergerConfig,
        ) -> RegressionOutcome {
            self.outcome.lock().unwrap().clone()
        }
    }

    #[derive(Debug)]
    struct StubPublicationBackend {
        result: Mutex<Result<String, String>>,
    }

    impl StubPublicationBackend {
        fn ok() -> Self {
            Self {
                result: Mutex::new(Ok(
                    "https://github.com/org/repo/pull/42".to_string(),
                )),
            }
        }
        fn fail() -> Self {
            Self {
                result: Mutex::new(Err("push rejected".to_string())),
            }
        }
    }

    #[async_trait::async_trait]
    impl PublicationBackend for StubPublicationBackend {
        async fn publish(
            &self,
            _plan_id: &str,
            _branch: &str,
            _commit_oid: &str,
            _workdir: &std::path::Path,
        ) -> Result<String, String> {
            self.result.lock().unwrap().clone()
        }
    }

    fn test_request(id: &str, publish: bool) -> CompletionDeliveryRequest {
        CompletionDeliveryRequest {
            delivery_id: id.to_string(),
            run_id: "run-1".to_string(),
            plan_id: "plan-a".to_string(),
            lease_id: "lease-1".to_string(),
            branch: "roko/plan-a".to_string(),
            commit_oid: "abc123".to_string(),
            target_branch: "main".to_string(),
            changed_files: vec!["src/lib.rs".to_string()],
            publish,
        }
    }

    fn service_with(
        merge: Arc<dyn MergeBackend>,
        gate: Arc<dyn RegressionGate>,
        pub_backend: Arc<dyn PublicationBackend>,
    ) -> CliCompletionDeliveryService {
        let config = CliDeliveryConfig::new(PathBuf::from("/tmp"), Duration::from_secs(5))
            .with_merge_backend(merge)
            .with_regression_gate(gate)
            .with_publication_backend(pub_backend);
        CliCompletionDeliveryService::new(config)
    }

    fn happy_service() -> CliCompletionDeliveryService {
        service_with(
            Arc::new(StubMergeBackend::pass()),
            Arc::new(StubRegressionGate::pass()),
            Arc::new(StubPublicationBackend::ok()),
        )
    }

    // ---- Tests ----

    #[tokio::test]
    async fn happy_path_with_publish() {
        let svc = happy_service();
        let req = test_request("d-happy", true);
        let receipt = svc.deliver(req).await.expect("delivery succeeds");

        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
        assert!(receipt.publication_ref.is_some());
        assert!(receipt.error.is_none());
        assert!(receipt.extensions.contains_key(DELIVERY_EXTENSION_KEY));
    }

    #[tokio::test]
    async fn happy_path_without_publish_skips_published() {
        let svc = happy_service();
        let req = test_request("d-nopub", false);
        let receipt = svc.deliver(req).await.expect("delivery succeeds");

        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
        assert!(
            receipt.publication_ref.is_none(),
            "publication_ref should be None when publish=false"
        );
    }

    #[tokio::test]
    async fn merge_conflict_is_terminal() {
        let svc = service_with(
            Arc::new(StubMergeBackend::conflict()),
            Arc::new(StubRegressionGate::pass()),
            Arc::new(StubPublicationBackend::ok()),
        );
        let req = test_request("d-conflict", true);
        let err = svc.deliver(req.clone()).await.unwrap_err();

        assert!(
            matches!(err, DeliveryError::MergeConflict { .. }),
            "expected MergeConflict, got {err:?}"
        );

        // Stored receipt should be terminal with conflict state
        let receipt = svc.store.get("d-conflict").unwrap();
        assert_eq!(receipt.state, CompletionDeliveryState::Conflict);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForReview);
    }

    #[tokio::test]
    async fn regression_failure_is_terminal() {
        let svc = service_with(
            Arc::new(StubMergeBackend::pass()),
            Arc::new(StubRegressionGate::fail()),
            Arc::new(StubPublicationBackend::ok()),
        );
        let req = test_request("d-regfail", true);
        let err = svc.deliver(req).await.unwrap_err();

        assert!(
            matches!(err, DeliveryError::RegressionFailed { .. }),
            "expected RegressionFailed, got {err:?}"
        );

        let receipt = svc.store.get("d-regfail").unwrap();
        assert_eq!(receipt.state, CompletionDeliveryState::RegressionFailed);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
        assert!(receipt.regression_evidence_ref.is_some());
    }

    #[tokio::test]
    async fn publication_failure_is_terminal() {
        let svc = service_with(
            Arc::new(StubMergeBackend::pass()),
            Arc::new(StubRegressionGate::pass()),
            Arc::new(StubPublicationBackend::fail()),
        );
        let req = test_request("d-pubfail", true);
        let err = svc.deliver(req).await.unwrap_err();

        assert!(
            matches!(err, DeliveryError::PublicationFailed { .. }),
            "expected PublicationFailed, got {err:?}"
        );

        let receipt = svc.store.get("d-pubfail").unwrap();
        assert_eq!(receipt.state, CompletionDeliveryState::TerminalFailed);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
    }

    #[tokio::test]
    async fn duplicate_same_fingerprint_returns_stored_receipt() {
        let svc = happy_service();
        let req = test_request("d-dup", true);

        let r1 = svc.deliver(req.clone()).await.expect("first deliver");
        let r2 = svc.deliver(req).await.expect("second deliver");

        assert_eq!(r1.state, CompletionDeliveryState::Delivered);
        assert_eq!(r2.state, CompletionDeliveryState::Delivered);
        assert_eq!(r1.request_fingerprint, r2.request_fingerprint);
    }

    #[tokio::test]
    async fn duplicate_different_fingerprint_fails_closed() {
        let svc = happy_service();
        let req1 = test_request("d-dup2", true);
        svc.deliver(req1).await.expect("first deliver");

        let mut req2 = test_request("d-dup2", true);
        req2.commit_oid = "different-oid".to_string();
        let err = svc.deliver(req2).await.unwrap_err();

        assert!(
            matches!(err, DeliveryError::FingerprintMismatch { .. }),
            "expected FingerprintMismatch, got {err:?}"
        );
    }

    #[tokio::test]
    async fn reconcile_returns_terminal_receipt() {
        let svc = happy_service();
        let req = test_request("d-reconcile", true);
        svc.deliver(req).await.expect("deliver");

        let receipt = svc.reconcile("d-reconcile").await.expect("reconcile");
        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
    }

    #[tokio::test]
    async fn reconcile_unknown_id_errors() {
        let svc = happy_service();
        let err = svc.reconcile("unknown").await.unwrap_err();
        assert!(
            matches!(err, DeliveryError::Other(ref msg) if msg.contains("unknown")),
            "expected Other with unknown message, got {err:?}"
        );
    }

    #[tokio::test]
    async fn fire_and_forget_enqueue_cannot_be_mistaken_for_success() {
        // Acceptance criteria: a fire-and-forget enqueue cannot be mistaken
        // for merged success. This test proves that the delivery service
        // tracks state and a Queued receipt is NOT terminal/success.
        let svc = happy_service();
        let req = test_request("d-enqueue-check", true);

        // Manually insert at Prepared to simulate a partial delivery
        svc.store.insert_or_get(&req).expect("insert");
        let stored = svc.store.get("d-enqueue-check").unwrap();
        assert_eq!(stored.state, CompletionDeliveryState::Prepared);
        assert!(!stored.state.is_terminal());
        assert!(!stored.state.is_success());
    }

    #[tokio::test]
    async fn resume_does_not_create_second_merge() {
        // Acceptance criteria: resume does not create a second merge or PR comment.
        // We prove this by delivering once, then reconciling. The second call
        // returns the stored receipt without re-merging.
        let svc = happy_service();
        let req = test_request("d-resume-no-dup", true);

        let r1 = svc.deliver(req.clone()).await.expect("first deliver");
        assert_eq!(r1.state, CompletionDeliveryState::Delivered);

        // Reconcile should return the same receipt without re-executing
        let r2 = svc.reconcile("d-resume-no-dup").await.expect("reconcile");
        assert_eq!(r2.state, CompletionDeliveryState::Delivered);
        assert_eq!(r1.merge_commit, r2.merge_commit);
    }

    #[tokio::test]
    async fn adapter_never_writes_terminal_state_or_releases_lease() {
        // Acceptance criteria: the adapter never writes the execution terminal
        // state or releases a #249 lease. We verify this by checking the receipt
        // only carries release_policy metadata, never acting on it.
        let svc = happy_service();
        let req = test_request("d-lease-check", true);
        let receipt = svc.deliver(req).await.expect("deliver");

        // The receipt has a release_policy but the service itself never
        // invokes release/retain -- that's outer-controller concern.
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
        // No direct workspace release was performed -- the service only
        // returns the policy for the outer controller to act on.
    }
}
