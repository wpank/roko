//! Completion delivery state machine for Graph execution merge/publish lifecycle.
//!
//! This module owns the typed delivery request, the ordered state machine,
//! durable receipts, and the async service trait that host adapters implement.
//!
//! The types here originated from the fire-and-forget `MergeRequest`/`MergeEnqueuer`
//! in `engine.rs`. That pair is preserved for backward compatibility but new
//! callers should use [`CompletionDeliveryService`] for durable merge/regression/
//! publication receipts.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

// ---- Re-export the legacy seam so existing callers keep compiling ----

/// A merge request produced by the graph engine after a successful plan execution.
///
/// This mirrors the live runner's merge request but lives in roko-graph to
/// avoid a circular dependency from the graph layer into CLI orchestration.
/// The orchestrator's runner bridges this to the real `MergeQueue` via the
/// [`MergeEnqueuer`] trait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeRequest {
    /// Plan identifier (typically the graph name).
    pub plan_id: String,
    /// Branch name to merge from.
    pub branch_name: String,
    /// Files changed by this plan execution.
    pub files_changed: Vec<String>,
    /// Merge priority (higher merges first).
    pub priority: u32,
}

/// Trait for enqueueing merge requests after graph execution.
///
/// The graph engine holds an optional `Arc<dyn MergeEnqueuer>`. After a
/// successful graph execution that represents a plan, the engine calls
/// [`MergeEnqueuer::enqueue`] with the plan's changed files.
///
/// Implement this trait to bridge to your merge queue implementation
/// (e.g., the CLI runner's `MergeQueue`).
pub trait MergeEnqueuer: Send + Sync + std::fmt::Debug {
    /// Enqueue a merge request. Returns `true` if the request was accepted.
    fn enqueue(&self, request: MergeRequest) -> bool;
}

// ---- Delivery state machine ----

/// A durable completion delivery request with full merge/publish identity.
///
/// Submitted to [`CompletionDeliveryService::deliver`]. The `delivery_id`
/// is the idempotency key: repeating `deliver` with the same ID returns the
/// stored receipt; a different request fingerprint for that ID fails closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionDeliveryRequest {
    /// Unique delivery identifier (idempotency key).
    pub delivery_id: String,
    /// Graph execution run ID.
    pub run_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Workspace lease ID from the outer controller.
    pub lease_id: String,
    /// Branch name to merge from.
    pub branch: String,
    /// Accepted merge commit OID (hex SHA).
    pub commit_oid: String,
    /// Target branch to merge into (e.g. `main`).
    pub target_branch: String,
    /// Files changed by this plan execution.
    pub changed_files: Vec<String>,
    /// Whether to publish to GitHub after regression passes.
    pub publish: bool,
}

impl CompletionDeliveryRequest {
    /// Compute a deterministic fingerprint for duplicate-detection.
    ///
    /// Two requests with the same `delivery_id` but different fingerprints
    /// indicate a conflict and the second submission must fail closed.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.run_id.hash(&mut hasher);
        self.plan_id.hash(&mut hasher);
        self.lease_id.hash(&mut hasher);
        self.branch.hash(&mut hasher);
        self.commit_oid.hash(&mut hasher);
        self.target_branch.hash(&mut hasher);
        self.changed_files.hash(&mut hasher);
        self.publish.hash(&mut hasher);
        hasher.finish()
    }
}

/// Ordered state machine for the completion delivery lifecycle.
///
/// Advances in exactly this order:
///   `Prepared -> Queued -> Merged -> RegressionPassed -> Published -> Delivered`
///
/// When `publish=false` in the request, the `Published` state is skipped:
///   `... -> RegressionPassed -> Delivered`
///
/// Terminal failure states are `Conflict`, `RegressionFailed`, and `TerminalFailed`.
/// Once in a terminal state, no further transitions are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionDeliveryState {
    /// Request created and validated, not yet enqueued.
    Prepared,
    /// Enqueued in the merge queue, waiting for the merge slot.
    Queued,
    /// Branch successfully merged into the target branch.
    Merged,
    /// Post-merge regression gate passed.
    RegressionPassed,
    /// Branch published to remote (GitHub push + PR/comment).
    Published,
    /// Terminal success: merge, regression, and (optional) publication complete.
    Delivered,
    /// Terminal failure: merge conflict that could not be resolved.
    Conflict,
    /// Terminal failure: post-merge regression gate failed.
    RegressionFailed,
    /// Terminal failure: publication or other irrecoverable error.
    TerminalFailed,
}

impl CompletionDeliveryState {
    /// Returns `true` when no further state transitions are possible.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Delivered | Self::Conflict | Self::RegressionFailed | Self::TerminalFailed
        )
    }

    /// Returns `true` for terminal success.
    #[must_use]
    pub fn is_success(self) -> bool {
        self == Self::Delivered
    }

    /// Returns `true` for any terminal failure.
    #[must_use]
    pub fn is_failed(self) -> bool {
        matches!(
            self,
            Self::Conflict | Self::RegressionFailed | Self::TerminalFailed
        )
    }

    /// Return the next expected state in the happy path.
    ///
    /// `publish` controls whether `Published` is included in the sequence.
    /// Returns `None` from terminal states or from `Delivered`.
    #[must_use]
    pub fn next_happy(self, publish: bool) -> Option<Self> {
        match self {
            Self::Prepared => Some(Self::Queued),
            Self::Queued => Some(Self::Merged),
            Self::Merged => Some(Self::RegressionPassed),
            Self::RegressionPassed => {
                if publish {
                    Some(Self::Published)
                } else {
                    Some(Self::Delivered)
                }
            }
            Self::Published => Some(Self::Delivered),
            _ => None,
        }
    }
}

/// What the outer controller should do with the workspace/worktree after
/// delivery completes.
///
/// Only `#256` (plan terminal) or `#257` (workflow terminal) act on this
/// policy. The delivery service itself never releases or retains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleasePolicy {
    /// Workspace may be deleted (successful delivery).
    Delete,
    /// Workspace must be retained for failure diagnostics.
    RetainForFailure,
    /// Workspace must be retained for manual conflict review.
    RetainForReview,
}

/// Durable receipt returned by every [`CompletionDeliveryService`] operation.
///
/// Contains the full delivery state, optional evidence references, and the
/// release policy that lets the outer controller decide success/failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionDeliveryReceiptV1 {
    /// The original delivery request.
    pub request: CompletionDeliveryRequest,
    /// Current delivery state.
    pub state: CompletionDeliveryState,
    /// Merge commit OID (set after `Merged` state).
    pub merge_commit: Option<String>,
    /// Remote publication reference (PR URL, branch push ref, etc.).
    pub publication_ref: Option<String>,
    /// Reference to regression evidence (e.g. path to gate output log).
    pub regression_evidence_ref: Option<String>,
    /// What the outer controller should do with the workspace.
    pub release_policy: ReleasePolicy,
    /// Error message for failed states.
    pub error: Option<String>,
    /// Fingerprint of the original request for duplicate detection.
    pub request_fingerprint: u64,
    /// Arbitrary metadata from state transition extensions.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl CompletionDeliveryReceiptV1 {
    /// Create the initial receipt in `Prepared` state.
    #[must_use]
    pub fn prepared(request: CompletionDeliveryRequest) -> Self {
        let fingerprint = request.fingerprint();
        Self {
            request,
            state: CompletionDeliveryState::Prepared,
            merge_commit: None,
            publication_ref: None,
            regression_evidence_ref: None,
            release_policy: ReleasePolicy::RetainForReview,
            error: None,
            request_fingerprint: fingerprint,
            extensions: HashMap::new(),
        }
    }

    /// Advance the receipt to a new state, updating the release policy.
    ///
    /// Returns `Err` if the current state is terminal.
    pub fn advance(
        &mut self,
        new_state: CompletionDeliveryState,
    ) -> Result<(), DeliveryTransitionError> {
        if self.state.is_terminal() {
            return Err(DeliveryTransitionError::AlreadyTerminal {
                current: self.state,
                attempted: new_state,
            });
        }
        self.state = new_state;
        self.release_policy = match new_state {
            CompletionDeliveryState::Delivered => ReleasePolicy::Delete,
            CompletionDeliveryState::RegressionFailed
            | CompletionDeliveryState::TerminalFailed => ReleasePolicy::RetainForFailure,
            CompletionDeliveryState::Conflict => ReleasePolicy::RetainForReview,
            _ => ReleasePolicy::RetainForReview,
        };
        Ok(())
    }
}

/// Errors from delivery state transitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeliveryTransitionError {
    /// Attempted to advance a receipt that is already in a terminal state.
    #[error("delivery already terminal ({current:?}), cannot advance to {attempted:?}")]
    AlreadyTerminal {
        /// The current terminal state.
        current: CompletionDeliveryState,
        /// The state the caller tried to advance to.
        attempted: CompletionDeliveryState,
    },
}

/// Error type for delivery service operations.
#[derive(Debug, thiserror::Error)]
pub enum DeliveryError {
    /// A different request was already submitted with the same delivery ID.
    #[error(
        "delivery ID '{delivery_id}' already submitted with a different request fingerprint \
         (stored={stored_fingerprint}, new={new_fingerprint})"
    )]
    FingerprintMismatch {
        /// The delivery ID that conflicted.
        delivery_id: String,
        /// The fingerprint of the previously stored request.
        stored_fingerprint: u64,
        /// The fingerprint of the new conflicting request.
        new_fingerprint: u64,
    },

    /// The merge queue rejected the request (e.g. the plan is already active).
    #[error("merge queue rejected delivery '{delivery_id}': {reason}")]
    QueueRejected {
        /// The delivery ID that was rejected.
        delivery_id: String,
        /// Human-readable reason for rejection.
        reason: String,
    },

    /// The merge resulted in a conflict.
    #[error("merge conflict for delivery '{delivery_id}': {details}")]
    MergeConflict {
        /// The delivery ID.
        delivery_id: String,
        /// Conflict details.
        details: String,
    },

    /// The post-merge regression gate failed.
    #[error("regression failed for delivery '{delivery_id}': {details}")]
    RegressionFailed {
        /// The delivery ID.
        delivery_id: String,
        /// Regression failure details.
        details: String,
    },

    /// Publication to remote failed.
    #[error("publication failed for delivery '{delivery_id}': {details}")]
    PublicationFailed {
        /// The delivery ID.
        delivery_id: String,
        /// Publication error details.
        details: String,
    },

    /// A state transition was invalid.
    #[error("invalid transition: {0}")]
    Transition(#[from] DeliveryTransitionError),

    /// Any other delivery error.
    #[error("delivery error: {0}")]
    Other(String),
}

/// Async trait for durable completion delivery.
///
/// Host adapters implement this to bridge graph completion into the
/// real merge queue, regression gate, and GitHub publication services.
///
/// # Idempotency
///
/// Repeating `deliver` with the same `delivery_id` returns the stored receipt.
/// A different request fingerprint for that `delivery_id` fails closed with
/// [`DeliveryError::FingerprintMismatch`].
///
/// # State machine
///
/// The service must advance through the fixed state sequence and write the
/// `roko.delivery@1` extension before queue submission and after every
/// state transition. The adapter never writes the execution terminal state
/// or releases a workspace lease -- those are outer-controller concerns.
#[async_trait::async_trait]
pub trait CompletionDeliveryService: Send + Sync + std::fmt::Debug {
    /// Submit a delivery request. Returns the terminal or current receipt.
    ///
    /// If the delivery ID has already been submitted with the same fingerprint,
    /// the stored receipt is returned. Otherwise the state machine advances
    /// through merge, regression, and (optional) publication.
    async fn deliver(
        &self,
        request: CompletionDeliveryRequest,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError>;

    /// Resume or query an in-progress delivery.
    ///
    /// On resume, `reconcile` queries the merge queue, git commit, and
    /// publication evidence and continues at the first unproved transition.
    async fn reconcile(
        &self,
        delivery_id: &str,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError>;
}

/// Extension key written before queue submission and after every state transition.
pub const DELIVERY_EXTENSION_KEY: &str = "roko.delivery@1";

/// Construct the extension value for the `roko.delivery@1` key.
#[must_use]
pub fn delivery_extension_value(receipt: &CompletionDeliveryReceiptV1) -> serde_json::Value {
    serde_json::json!({
        "delivery_id": receipt.request.delivery_id,
        "state": receipt.state,
        "plan_id": receipt.request.plan_id,
        "run_id": receipt.request.run_id,
        "merge_commit": receipt.merge_commit,
        "publication_ref": receipt.publication_ref,
        "release_policy": receipt.release_policy,
        "error": receipt.error,
    })
}

// ---- Delivery receipt store (in-memory, for host adapters) ----

/// Thread-safe in-memory store for delivery receipts.
///
/// Host adapters use this to track in-flight and completed deliveries.
/// The store enforces fingerprint-based idempotency.
#[derive(Debug, Default, Clone)]
pub struct DeliveryReceiptStore {
    inner: Arc<parking_lot::Mutex<HashMap<String, CompletionDeliveryReceiptV1>>>,
}

impl DeliveryReceiptStore {
    /// Create a new empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to insert a new delivery. Returns `Ok(receipt)` if the delivery ID
    /// is new, or returns the existing receipt if the fingerprint matches.
    /// Returns `Err` if a different fingerprint was already stored.
    pub fn insert_or_get(
        &self,
        request: &CompletionDeliveryRequest,
    ) -> Result<Option<CompletionDeliveryReceiptV1>, DeliveryError> {
        let mut store = self.inner.lock();
        if let Some(existing) = store.get(&request.delivery_id) {
            if existing.request_fingerprint == request.fingerprint() {
                return Ok(Some(existing.clone()));
            }
            return Err(DeliveryError::FingerprintMismatch {
                delivery_id: request.delivery_id.clone(),
                stored_fingerprint: existing.request_fingerprint,
                new_fingerprint: request.fingerprint(),
            });
        }
        let receipt = CompletionDeliveryReceiptV1::prepared(request.clone());
        store.insert(request.delivery_id.clone(), receipt);
        Ok(None)
    }

    /// Update the stored receipt for a delivery ID.
    pub fn update(&self, receipt: &CompletionDeliveryReceiptV1) {
        let mut store = self.inner.lock();
        store.insert(receipt.request.delivery_id.clone(), receipt.clone());
    }

    /// Get a stored receipt by delivery ID.
    #[must_use]
    pub fn get(&self, delivery_id: &str) -> Option<CompletionDeliveryReceiptV1> {
        self.inner.lock().get(delivery_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn state_machine_happy_path_with_publish() {
        let states = [
            CompletionDeliveryState::Prepared,
            CompletionDeliveryState::Queued,
            CompletionDeliveryState::Merged,
            CompletionDeliveryState::RegressionPassed,
            CompletionDeliveryState::Published,
            CompletionDeliveryState::Delivered,
        ];
        for i in 0..states.len() - 1 {
            assert_eq!(
                states[i].next_happy(true),
                Some(states[i + 1]),
                "expected {:?} -> {:?}",
                states[i],
                states[i + 1]
            );
        }
        assert!(states.last().unwrap().next_happy(true).is_none());
    }

    #[test]
    fn state_machine_skips_published_without_publish() {
        assert_eq!(
            CompletionDeliveryState::RegressionPassed.next_happy(false),
            Some(CompletionDeliveryState::Delivered)
        );
    }

    #[test]
    fn terminal_states_are_terminal() {
        assert!(CompletionDeliveryState::Delivered.is_terminal());
        assert!(CompletionDeliveryState::Conflict.is_terminal());
        assert!(CompletionDeliveryState::RegressionFailed.is_terminal());
        assert!(CompletionDeliveryState::TerminalFailed.is_terminal());
    }

    #[test]
    fn non_terminal_states_are_not_terminal() {
        for state in [
            CompletionDeliveryState::Prepared,
            CompletionDeliveryState::Queued,
            CompletionDeliveryState::Merged,
            CompletionDeliveryState::RegressionPassed,
            CompletionDeliveryState::Published,
        ] {
            assert!(!state.is_terminal(), "{state:?} should not be terminal");
        }
    }

    #[test]
    fn receipt_advance_happy_path() {
        let req = test_request("d1");
        let mut receipt = CompletionDeliveryReceiptV1::prepared(req);
        assert_eq!(receipt.state, CompletionDeliveryState::Prepared);

        receipt
            .advance(CompletionDeliveryState::Queued)
            .expect("advance to queued");
        assert_eq!(receipt.state, CompletionDeliveryState::Queued);

        receipt
            .advance(CompletionDeliveryState::Merged)
            .expect("advance to merged");
        receipt
            .advance(CompletionDeliveryState::RegressionPassed)
            .expect("advance to regression_passed");
        receipt
            .advance(CompletionDeliveryState::Published)
            .expect("advance to published");
        receipt
            .advance(CompletionDeliveryState::Delivered)
            .expect("advance to delivered");

        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
    }

    #[test]
    fn receipt_cannot_advance_past_terminal() {
        let req = test_request("d2");
        let mut receipt = CompletionDeliveryReceiptV1::prepared(req);
        receipt
            .advance(CompletionDeliveryState::Conflict)
            .expect("advance to conflict");
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForReview);

        let err = receipt
            .advance(CompletionDeliveryState::Merged)
            .unwrap_err();
        assert!(matches!(
            err,
            DeliveryTransitionError::AlreadyTerminal { .. }
        ));
    }

    #[test]
    fn regression_failure_sets_retain_for_failure() {
        let req = test_request("d3");
        let mut receipt = CompletionDeliveryReceiptV1::prepared(req);
        receipt
            .advance(CompletionDeliveryState::Queued)
            .expect("queued");
        receipt
            .advance(CompletionDeliveryState::Merged)
            .expect("merged");
        receipt
            .advance(CompletionDeliveryState::RegressionFailed)
            .expect("regression_failed");
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
        assert!(receipt.state.is_failed());
    }

    #[test]
    fn terminal_failed_sets_retain_for_failure() {
        let req = test_request("d4");
        let mut receipt = CompletionDeliveryReceiptV1::prepared(req);
        receipt
            .advance(CompletionDeliveryState::TerminalFailed)
            .expect("terminal_failed");
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let req1 = test_request("d1");
        let req2 = test_request("d1");
        assert_eq!(req1.fingerprint(), req2.fingerprint());
    }

    #[test]
    fn fingerprint_differs_for_different_content() {
        let req1 = test_request("d1");
        let mut req2 = test_request("d1");
        req2.commit_oid = "different".to_string();
        assert_ne!(req1.fingerprint(), req2.fingerprint());
    }

    #[test]
    fn receipt_store_insert_new() {
        let store = DeliveryReceiptStore::new();
        let req = test_request("d1");
        let result = store.insert_or_get(&req).expect("insert succeeds");
        assert!(result.is_none(), "first insert returns None");

        let stored = store.get("d1").expect("stored receipt exists");
        assert_eq!(stored.state, CompletionDeliveryState::Prepared);
    }

    #[test]
    fn receipt_store_returns_existing_on_same_fingerprint() {
        let store = DeliveryReceiptStore::new();
        let req = test_request("d1");
        store.insert_or_get(&req).expect("first insert");

        let result = store
            .insert_or_get(&req)
            .expect("duplicate insert with same fingerprint");
        assert!(
            result.is_some(),
            "should return existing receipt for same fingerprint"
        );
    }

    #[test]
    fn receipt_store_rejects_different_fingerprint() {
        let store = DeliveryReceiptStore::new();
        let req1 = test_request("d1");
        store.insert_or_get(&req1).expect("first insert");

        let mut req2 = test_request("d1");
        req2.commit_oid = "different-oid".to_string();
        let err = store.insert_or_get(&req2).unwrap_err();
        assert!(
            matches!(err, DeliveryError::FingerprintMismatch { .. }),
            "expected fingerprint mismatch, got {err:?}"
        );
    }

    #[test]
    fn delivery_extension_value_structure() {
        let req = test_request("d1");
        let receipt = CompletionDeliveryReceiptV1::prepared(req);
        let value = delivery_extension_value(&receipt);
        assert_eq!(value["delivery_id"], "d1");
        assert_eq!(value["state"], "prepared");
        assert_eq!(value["plan_id"], "plan-a");
        assert!(value["merge_commit"].is_null());
    }

    #[test]
    fn serde_roundtrip_state() {
        for state in [
            CompletionDeliveryState::Prepared,
            CompletionDeliveryState::Queued,
            CompletionDeliveryState::Merged,
            CompletionDeliveryState::RegressionPassed,
            CompletionDeliveryState::Published,
            CompletionDeliveryState::Delivered,
            CompletionDeliveryState::Conflict,
            CompletionDeliveryState::RegressionFailed,
            CompletionDeliveryState::TerminalFailed,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: CompletionDeliveryState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back, "roundtrip for {state:?}");
        }
    }

    #[test]
    fn serde_roundtrip_receipt() {
        let req = test_request("d-rt");
        let mut receipt = CompletionDeliveryReceiptV1::prepared(req);
        receipt.merge_commit = Some("abc123def".to_string());
        receipt.publication_ref = Some("https://github.com/org/repo/pull/42".to_string());
        receipt
            .advance(CompletionDeliveryState::Delivered)
            .unwrap();

        let json = serde_json::to_string(&receipt).unwrap();
        let back: CompletionDeliveryReceiptV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(back.state, CompletionDeliveryState::Delivered);
        assert_eq!(back.merge_commit, receipt.merge_commit);
        assert_eq!(back.release_policy, ReleasePolicy::Delete);
    }

    #[test]
    fn serde_roundtrip_request() {
        let req = test_request("d-req-rt");
        let json = serde_json::to_string(&req).unwrap();
        let back: CompletionDeliveryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn serde_roundtrip_release_policy() {
        for policy in [
            ReleasePolicy::Delete,
            ReleasePolicy::RetainForFailure,
            ReleasePolicy::RetainForReview,
        ] {
            let json = serde_json::to_string(&policy).unwrap();
            let back: ReleasePolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(policy, back);
        }
    }

    #[test]
    fn is_success_and_is_failed() {
        assert!(CompletionDeliveryState::Delivered.is_success());
        assert!(!CompletionDeliveryState::Delivered.is_failed());
        assert!(!CompletionDeliveryState::Prepared.is_success());
        assert!(!CompletionDeliveryState::Prepared.is_failed());
        assert!(!CompletionDeliveryState::Conflict.is_success());
        assert!(CompletionDeliveryState::Conflict.is_failed());
    }
}
