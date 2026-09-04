//! Layered Graph Snapshot v2: versioned envelope, extension ledger, receipt
//! state machine, and reconciliation callback contract.
//!
//! This module defines the graph-layer snapshot types that the engine persists
//! and resumes from. Host layers (CLI, serve, etc.) use the extension ledger
//! to register namespaced data without editing the graph-core schema.
//!
//! # Schema versions
//!
//! - **v1** (implicit): original unversioned `GraphSnapshot` with node statuses,
//!   outputs, tick count, and policy. Missing fields default to zero/empty.
//! - **v2** (current): adds `schema_version`, `graph_fingerprint`, budget
//!   tracking fields (`budget_spent_micro_usd`, `budget_reserved_micro_usd`),
//!   and `last_event_seq` for monotonic event replay.
//!
//! A v1 snapshot on disk deserializes into `GraphSnapshotV2` with serde defaults;
//! no explicit migration code is needed because all new fields have
//! `#[serde(default)]`.
//!
//! # Extension ledger
//!
//! Host features register namespaced extensions via [`CheckpointExtension`].
//! Extension map keys follow the format `<namespace>@<schema_version>`.
//! Unknown required extensions fail on restore; unknown optional extensions
//! round-trip unchanged as opaque `serde_json::Value`.
//!
//! # Receipt ledger
//!
//! The [`ReceiptState`] machine enforces forward-only transitions:
//! `Prepared -> Committed -> Settled`. Repeating the current transition is
//! idempotent success. Reverse or skipped transitions fail closed.
//!
//! # Reconciliation
//!
//! [`ReconcileAction`] is the decision type returned by extension owners when
//! a `Running` Activity is encountered during restore. The engine delegates
//! to the registered owner rather than blindly resetting to `Pending`.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::types::GraphPolicy;

// ---------------------------------------------------------------------------
// Schema version
// ---------------------------------------------------------------------------

/// Current schema version for [`GraphSnapshotV2`].
pub const GRAPH_SNAPSHOT_SCHEMA_VERSION: u8 = 2;

fn default_snapshot_schema_version() -> u8 {
    GRAPH_SNAPSHOT_SCHEMA_VERSION
}

// ---------------------------------------------------------------------------
// Core snapshot
// ---------------------------------------------------------------------------

/// Serializable snapshot of a graph execution in progress or completed (v2).
///
/// Captures per-node status, Activity node outputs, policy, budget state, and
/// a stable graph fingerprint so the engine can be resumed safely. Only
/// Activity node outputs are included -- Workflow node outputs are re-derived
/// on resume.
///
/// V2 adds `schema_version`, `graph_fingerprint`, budget tracking fields, and
/// `last_event_seq` for monotonic event replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshotV2 {
    /// On-disk schema version. Always `2` for this struct.
    #[serde(default = "default_snapshot_schema_version")]
    pub schema_version: u8,
    /// Name of the graph.
    pub graph_name: String,
    /// Graph ID (from metadata).
    pub graph_id: String,
    /// Stable BLAKE3 fingerprint of the execution-relevant graph definition.
    /// Used to reject resume after graph definition or policy drift.
    #[serde(default)]
    pub graph_fingerprint: String,
    /// Per-node execution status at snapshot time.
    pub node_statuses: HashMap<String, SerializableNodeStatus>,
    /// Activity node outputs. Workflow nodes are excluded (re-derived on resume).
    pub node_outputs: HashMap<String, Vec<SerializableSignal>>,
    /// Hot Graph tick count at snapshot time.
    pub tick_count: u64,
    /// Cumulative budget spent in micro-USD (1 USD = 1_000_000).
    #[serde(default)]
    pub budget_spent_micro_usd: u64,
    /// Budget reserved but not yet settled in micro-USD.
    #[serde(default)]
    pub budget_reserved_micro_usd: u64,
    /// Monotonic event sequence number at snapshot time. Replay must not emit
    /// events with sequence numbers at or below this value.
    #[serde(default)]
    pub last_event_seq: u64,
    /// Unix milliseconds when the snapshot was captured.
    pub created_at_ms: i64,
    /// Graph policy preserved for resume.
    pub policy: GraphPolicy,
}

/// Primary snapshot type. Callers use this alias; the underlying versioned
/// struct name is kept for migration clarity.
pub type GraphSnapshot = GraphSnapshotV2;

// ---------------------------------------------------------------------------
// Serializable node status
// ---------------------------------------------------------------------------

/// Serializable node status (mirrors [`NodeStatus`](crate::engine::NodeStatus)
/// but with serde support).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializableNodeStatus {
    /// Not yet started.
    Pending,
    /// Currently executing (delegated to reconciliation owner on resume).
    Running,
    /// Completed successfully.
    Complete,
    /// Failed during execution.
    Failed,
    /// Skipped because an upstream node failed.
    Skipped,
    /// Skipped because no incoming conditional route selected this node.
    ConditionSkipped,
}

/// Lightweight serializable signal reference for snapshots.
///
/// Full [`roko_core::Signal`] is already serde-compatible, but we wrap the
/// JSON representation to keep the snapshot format stable even if Signal
/// internals change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSignal {
    /// JSON-serialized signal.
    pub json: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Reconciliation
// ---------------------------------------------------------------------------

/// Decision returned by an extension owner when a `Running` Activity is
/// encountered during restore.
///
/// Each extension owner registers a reconcile callback for its namespace.
/// On restore, `Running` nodes delegate to the registered owner's callback
/// rather than unconditionally converting to `Pending`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileAction {
    /// The stored state is valid and execution should continue from it.
    Continue,
    /// The stored state is invalid; reset the activity to pending.
    Reset,
    /// The stored state is unrecoverable; fail with the given reason.
    Fail(String),
}

/// Reconcile an ambiguous `Running` status from a restored snapshot.
///
/// Graph callers that do not have a registered reconciliation owner should
/// call this to convert `Running` to `Pending` before resume. This preserves
/// backward compatibility for callers that do not implement owner-based
/// reconciliation.
pub fn reconcile_running_status(status: SerializableNodeStatus) -> ReconcileAction {
    match status {
        SerializableNodeStatus::Running => ReconcileAction::Reset,
        _ => ReconcileAction::Continue,
    }
}

// ---------------------------------------------------------------------------
// Extension ledger
// ---------------------------------------------------------------------------

/// A namespaced, versioned extension stored in a checkpoint envelope.
///
/// Feature packets register their own concrete extension schemas without
/// editing the `GraphSnapshotV2` struct. The extension map key is exactly
/// `<namespace>@<schema_version>`.
///
/// # Required vs optional
///
/// When `required` is `true`, an unknown namespace on restore fails closed
/// with a diagnostic identifying the missing namespace and its expected
/// owner. When `required` is `false`, the extension round-trips unchanged
/// as opaque `serde_json::Value`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointExtension {
    /// Dotted namespace, e.g. `roko.workspace.attempt`.
    pub namespace: String,
    /// Schema version of this extension's value.
    pub schema_version: u32,
    /// Whether this extension is required for a valid restore.
    pub required: bool,
    /// Deterministic fingerprint of the extension value, used to detect
    /// re-registration drift.
    pub fingerprint: String,
    /// Opaque JSON payload owned by the registering feature.
    pub value: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Receipt ledger
// ---------------------------------------------------------------------------

/// Ordered lifecycle states for an idempotent receipt.
///
/// State transitions only move forward: `Prepared -> Committed -> Settled`.
/// Repeating the current transition is a no-op success. Reverse or skipped
/// transitions fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptState {
    /// The receipt has been created but the external side-effect has not
    /// been confirmed.
    Prepared = 0,
    /// The external side-effect has been confirmed. Provider dispatch
    /// reuses the committed evidence rather than re-calling.
    Committed = 1,
    /// The receipt has been fully settled and requires no further work.
    Settled = 2,
}

impl std::fmt::Display for ReceiptState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepared => write!(f, "prepared"),
            Self::Committed => write!(f, "committed"),
            Self::Settled => write!(f, "settled"),
        }
    }
}

/// A single entry in the checkpoint's receipt ledger.
///
/// The `idempotency_key` is the ledger map key. Provider dispatch checks
/// this before calling externally: `Committed` reuses evidence, `Prepared`
/// invokes the owner's reconcile path, and `Settled` performs no work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptLedgerEntry {
    /// Stable key for deduplication across restarts.
    pub idempotency_key: String,
    /// Subsystem or feature that owns this receipt.
    pub owner: String,
    /// Correlation ID linking this receipt to its originating request.
    pub correlation_id: String,
    /// Current lifecycle state.
    pub state: ReceiptState,
    /// Optional reference to external evidence (commit OID, URL, etc.).
    #[serde(default)]
    pub evidence_ref: Option<String>,
    /// Unix milliseconds of the last state transition.
    pub updated_at_ms: u128,
    /// Last error message if a transition failed.
    #[serde(default)]
    pub last_error: Option<String>,
}

// ---------------------------------------------------------------------------
// Extension registry
// ---------------------------------------------------------------------------

/// In-memory registry of known checkpoint extensions and reconciliation
/// callbacks.
///
/// Host layers register their namespaces at startup. On checkpoint restore,
/// the registry validates required extensions and delegates `Running` node
/// reconciliation to the appropriate owner.
#[derive(Default)]
pub struct ExtensionRegistry {
    /// Registered reconciliation callbacks keyed by namespace.
    reconcilers: HashMap<String, Box<dyn Fn(&[u8]) -> ReconcileAction + Send + Sync>>,
    /// Registered extension metadata keyed by `namespace@version`.
    known: BTreeMap<String, ExtensionMeta>,
}

/// Metadata about a registered extension namespace.
#[derive(Debug, Clone)]
struct ExtensionMeta {
    /// Whether the extension is required for a valid restore. Stored for
    /// future owner-based validation; currently the `CheckpointExtension`
    /// carries the authoritative `required` flag.
    #[allow(dead_code)]
    required: bool,
    /// Feature or subsystem that owns this namespace. Stored for diagnostic
    /// messages when unknown required extensions are encountered.
    #[allow(dead_code)]
    owner: String,
}

impl ExtensionRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an extension namespace.
    ///
    /// `namespace_key` must be in `<namespace>@<version>` format.
    pub fn register(
        &mut self,
        namespace_key: &str,
        required: bool,
        owner: &str,
    ) {
        self.known.insert(
            namespace_key.to_string(),
            ExtensionMeta {
                required,
                owner: owner.to_string(),
            },
        );
    }

    /// Register a reconciliation callback for a namespace.
    ///
    /// On restore, `Running` nodes that belong to this namespace will have
    /// their stored state passed to `callback` to determine the action.
    pub fn register_reconciler(
        &mut self,
        namespace: &str,
        callback: impl Fn(&[u8]) -> ReconcileAction + Send + Sync + 'static,
    ) {
        self.reconcilers
            .insert(namespace.to_string(), Box::new(callback));
    }

    /// Look up a reconciler for a namespace.
    pub fn reconciler(&self, namespace: &str) -> Option<&(dyn Fn(&[u8]) -> ReconcileAction + Send + Sync)> {
        self.reconcilers.get(namespace).map(|b| b.as_ref())
    }

    /// Validate that all required extensions in a loaded map are known.
    ///
    /// Returns a list of unknown required namespace keys. If the returned
    /// vec is empty, the restore is valid.
    pub fn validate_extensions(
        &self,
        extensions: &BTreeMap<String, CheckpointExtension>,
    ) -> Vec<String> {
        let mut unknown = Vec::new();
        for (key, ext) in extensions {
            if ext.required && !self.known.contains_key(key) {
                unknown.push(key.clone());
            }
        }
        unknown
    }
}

impl std::fmt::Debug for ExtensionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtensionRegistry")
            .field("known_namespaces", &self.known.keys().collect::<Vec<_>>())
            .field("reconciler_count", &self.reconcilers.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Extension map operations
// ---------------------------------------------------------------------------

/// Register an extension into a `BTreeMap`, enforcing fingerprint idempotency.
///
/// Duplicate registration with the same fingerprint is a no-op success.
/// Duplicate registration with a different fingerprint returns an error.
pub fn register_extension(
    extensions: &mut BTreeMap<String, CheckpointExtension>,
    ext: CheckpointExtension,
) -> Result<(), ExtensionError> {
    let key = format!("{}@{}", ext.namespace, ext.schema_version);
    if let Some(existing) = extensions.get(&key) {
        if existing.fingerprint != ext.fingerprint {
            return Err(ExtensionError::FingerprintMismatch {
                key,
                existing: existing.fingerprint.clone(),
                incoming: ext.fingerprint,
            });
        }
        // Same fingerprint: idempotent success.
        return Ok(());
    }
    extensions.insert(key, ext);
    Ok(())
}

// ---------------------------------------------------------------------------
// Receipt ledger operations
// ---------------------------------------------------------------------------

/// Prepare a new receipt in a ledger map.
///
/// If a receipt with this idempotency key already exists at or past
/// `Prepared`, this is a no-op returning the existing entry.
pub fn prepare_receipt(
    receipts: &mut BTreeMap<String, ReceiptLedgerEntry>,
    idempotency_key: String,
    owner: String,
    correlation_id: String,
    now_ms: u128,
) -> &ReceiptLedgerEntry {
    if receipts.contains_key(&idempotency_key) {
        return receipts.get(&idempotency_key).expect("key exists");
    }
    let entry = ReceiptLedgerEntry {
        idempotency_key: idempotency_key.clone(),
        owner,
        correlation_id,
        state: ReceiptState::Prepared,
        evidence_ref: None,
        updated_at_ms: now_ms,
        last_error: None,
    };
    receipts.insert(idempotency_key.clone(), entry);
    receipts.get(&idempotency_key).expect("just inserted")
}

/// Transition a receipt from `Prepared` to `Committed`.
///
/// Repeating on an already-`Committed` or `Settled` receipt is idempotent
/// success. Returns an error if the receipt does not exist.
pub fn commit_receipt<'a>(
    receipts: &'a mut BTreeMap<String, ReceiptLedgerEntry>,
    idempotency_key: &str,
    evidence_ref: Option<String>,
    now_ms: u128,
) -> Result<&'a ReceiptLedgerEntry, ReceiptError> {
    let entry = receipts
        .get_mut(idempotency_key)
        .ok_or_else(|| ReceiptError::NotFound(idempotency_key.to_string()))?;

    match entry.state {
        ReceiptState::Prepared => {
            entry.state = ReceiptState::Committed;
            entry.evidence_ref = evidence_ref;
            entry.updated_at_ms = now_ms;
            entry.last_error = None;
        }
        ReceiptState::Committed | ReceiptState::Settled => {
            // Already at or past Committed: idempotent success.
        }
    }

    Ok(receipts.get(idempotency_key).expect("entry exists"))
}

/// Transition a receipt from `Committed` to `Settled`.
///
/// Repeating on an already-`Settled` receipt is idempotent. Calling on a
/// `Prepared` receipt (skipping `Committed`) fails closed.
pub fn settle_receipt<'a>(
    receipts: &'a mut BTreeMap<String, ReceiptLedgerEntry>,
    idempotency_key: &str,
    now_ms: u128,
) -> Result<&'a ReceiptLedgerEntry, ReceiptError> {
    let entry = receipts
        .get_mut(idempotency_key)
        .ok_or_else(|| ReceiptError::NotFound(idempotency_key.to_string()))?;

    match entry.state {
        ReceiptState::Prepared => {
            return Err(ReceiptError::SkippedTransition {
                key: idempotency_key.to_string(),
                current: ReceiptState::Prepared,
                target: ReceiptState::Settled,
            });
        }
        ReceiptState::Committed => {
            entry.state = ReceiptState::Settled;
            entry.updated_at_ms = now_ms;
            entry.last_error = None;
        }
        ReceiptState::Settled => {
            // Already settled: idempotent success.
        }
    }

    Ok(receipts.get(idempotency_key).expect("entry exists"))
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error from extension registration.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtensionError {
    /// An extension with the same key but a different fingerprint exists.
    #[error(
        "extension '{key}' already registered with fingerprint '{existing}', \
         cannot re-register with different fingerprint '{incoming}'"
    )]
    FingerprintMismatch {
        /// Extension map key (`namespace@version`).
        key: String,
        /// Fingerprint of the already-registered extension.
        existing: String,
        /// Fingerprint of the incoming conflicting registration.
        incoming: String,
    },
}

/// Error from receipt ledger operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReceiptError {
    /// The idempotency key was not found in the ledger.
    #[error("receipt '{0}' not found in ledger")]
    NotFound(String),
    /// A forward-only transition was violated (e.g. Prepared -> Settled).
    #[error(
        "cannot transition receipt '{key}' from {current} to {target}: \
         must go through intermediate states"
    )]
    SkippedTransition {
        /// Idempotency key of the receipt.
        key: String,
        /// Current state of the receipt.
        current: ReceiptState,
        /// Attempted target state.
        target: ReceiptState,
    },
}

// ---------------------------------------------------------------------------
// Known extension namespaces (frozen registry table from #251 spec)
// ---------------------------------------------------------------------------

/// Extension namespace for graph-core cost tracking.
pub const EXT_COST: &str = "roko.cost@1";
/// Extension namespace for graph-core activity state.
pub const EXT_ACTIVITY: &str = "roko.activity@1";
/// Extension namespace for replan mutation (#252).
pub const EXT_REPLAN: &str = "roko.replan@1";
/// Extension namespace for feedback loop (#253).
pub const EXT_FEEDBACK: &str = "roko.feedback@1";
/// Extension namespace for completion delivery (#254).
pub const EXT_DELIVERY: &str = "roko.delivery@1";
/// Extension namespace for execution control (#255).
pub const EXT_CONTROL: &str = "roko.control@1";
/// Extension namespace for gate verdict history (#250).
pub const EXT_GATE_HISTORY: &str = "roko.gate-history@1";
/// Extension namespace for approval state (#255).
pub const EXT_APPROVAL: &str = "roko.approval@1";
/// Extension namespace for run context (#327).
pub const EXT_RUN_CONTEXT: &str = "roko.run-context@1";
/// Extension namespace for safety provenance (#351).
pub const EXT_SAFETY_PROVENANCE: &str = "roko.safety-provenance@1";
/// Extension namespace for prompt experiments (#359).
pub const EXT_EXPERIMENT: &str = "roko.experiment@1";

/// Populate an [`ExtensionRegistry`] with the frozen namespace table.
///
/// This registers every known namespace with its `required` flag so that
/// [`ExtensionRegistry::validate_extensions`] can reject unknown required
/// extensions on restore.
pub fn register_known_namespaces(registry: &mut ExtensionRegistry) {
    // Required namespaces.
    registry.register(EXT_COST, true, "graph core");
    registry.register(EXT_ACTIVITY, true, "graph core");
    registry.register(EXT_REPLAN, true, "#252");
    registry.register(EXT_FEEDBACK, true, "#253");
    registry.register(EXT_DELIVERY, true, "#254");
    registry.register(EXT_CONTROL, true, "#255");
    registry.register(EXT_RUN_CONTEXT, true, "#327");

    // Optional namespaces.
    registry.register(EXT_GATE_HISTORY, false, "#250");
    registry.register(EXT_APPROVAL, false, "#255");
    registry.register(EXT_SAFETY_PROVENANCE, false, "#351");
    registry.register(EXT_EXPERIMENT, false, "#359");
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─── GraphSnapshotV2 serde ──────────────────────────────────────────

    #[test]
    fn snapshot_v2_serde_roundtrip() {
        let snap = GraphSnapshotV2 {
            schema_version: GRAPH_SNAPSHOT_SCHEMA_VERSION,
            graph_name: "test".into(),
            graph_id: "test".into(),
            graph_fingerprint: "abc123".into(),
            node_statuses: HashMap::from([
                ("a".into(), SerializableNodeStatus::Complete),
                ("b".into(), SerializableNodeStatus::Running),
                ("c".into(), SerializableNodeStatus::Pending),
            ]),
            node_outputs: HashMap::new(),
            tick_count: 5,
            budget_spent_micro_usd: 123_456,
            budget_reserved_micro_usd: 50_000,
            last_event_seq: 42,
            created_at_ms: 1_000_000,
            policy: GraphPolicy::default(),
        };

        let json = serde_json::to_string(&snap).expect("serialize");
        let deserialized: GraphSnapshotV2 =
            serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.schema_version, GRAPH_SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(deserialized.graph_fingerprint, "abc123");
        assert_eq!(deserialized.budget_spent_micro_usd, 123_456);
        assert_eq!(deserialized.budget_reserved_micro_usd, 50_000);
        assert_eq!(deserialized.last_event_seq, 42);
        assert_eq!(deserialized.tick_count, 5);
    }

    #[test]
    fn snapshot_v2_backward_compat_missing_new_fields() {
        let json = serde_json::json!({
            "graph_name": "old",
            "graph_id": "old",
            "node_statuses": {},
            "node_outputs": {},
            "tick_count": 0,
            "created_at_ms": 1000,
            "policy": {
                "mode": "one_shot",
                "failure_strategy": "fail_fast",
                "max_concurrent_nodes": 4
            }
        });

        let snap: GraphSnapshotV2 =
            serde_json::from_value(json).expect("deserialize old format");

        assert_eq!(snap.schema_version, GRAPH_SNAPSHOT_SCHEMA_VERSION);
        assert!(snap.graph_fingerprint.is_empty());
        assert_eq!(snap.budget_spent_micro_usd, 0);
        assert_eq!(snap.budget_reserved_micro_usd, 0);
        assert_eq!(snap.last_event_seq, 0);
    }

    #[test]
    fn snapshot_type_alias_is_v2() {
        fn assert_same_type(_snap: GraphSnapshot) {
            let _v2: GraphSnapshotV2 = _snap;
        }
        let snap = GraphSnapshotV2 {
            schema_version: 2,
            graph_name: "t".into(),
            graph_id: "t".into(),
            graph_fingerprint: String::new(),
            node_statuses: HashMap::new(),
            node_outputs: HashMap::new(),
            tick_count: 0,
            budget_spent_micro_usd: 0,
            budget_reserved_micro_usd: 0,
            last_event_seq: 0,
            created_at_ms: 0,
            policy: GraphPolicy::default(),
        };
        assert_same_type(snap);
    }

    // ─── Reconciliation ─────────────────────────────────────────────────

    #[test]
    fn reconcile_running_returns_reset() {
        assert_eq!(
            reconcile_running_status(SerializableNodeStatus::Running),
            ReconcileAction::Reset
        );
    }

    #[test]
    fn reconcile_complete_returns_continue() {
        assert_eq!(
            reconcile_running_status(SerializableNodeStatus::Complete),
            ReconcileAction::Continue
        );
    }

    #[test]
    fn reconcile_pending_returns_continue() {
        assert_eq!(
            reconcile_running_status(SerializableNodeStatus::Pending),
            ReconcileAction::Continue
        );
    }

    // ─── Extension registration ─────────────────────────────────────────

    #[test]
    fn extension_registration_idempotent_same_fingerprint() {
        let mut exts = BTreeMap::new();
        let ext = CheckpointExtension {
            namespace: "roko.test".into(),
            schema_version: 1,
            required: false,
            fingerprint: "fp1".into(),
            value: serde_json::json!({"key": "value"}),
        };
        register_extension(&mut exts, ext.clone()).expect("first");
        register_extension(&mut exts, ext).expect("idempotent");
        assert_eq!(exts.len(), 1);
    }

    #[test]
    fn extension_registration_rejects_fingerprint_mismatch() {
        let mut exts = BTreeMap::new();
        let ext1 = CheckpointExtension {
            namespace: "roko.test".into(),
            schema_version: 1,
            required: false,
            fingerprint: "fp1".into(),
            value: serde_json::json!({}),
        };
        let ext2 = CheckpointExtension {
            namespace: "roko.test".into(),
            schema_version: 1,
            required: false,
            fingerprint: "fp2".into(),
            value: serde_json::json!({}),
        };
        register_extension(&mut exts, ext1).expect("first");
        let err = register_extension(&mut exts, ext2).unwrap_err();
        assert!(
            matches!(err, ExtensionError::FingerprintMismatch { .. }),
            "expected FingerprintMismatch, got {err:?}"
        );
    }

    #[test]
    fn extension_roundtrip_preserves_opaque_value() {
        let ext = CheckpointExtension {
            namespace: "roko.unknown".into(),
            schema_version: 42,
            required: false,
            fingerprint: "abc".into(),
            value: serde_json::json!({
                "nested": {"deeply": [1, 2, 3]},
                "flag": true
            }),
        };
        let json = serde_json::to_string(&ext).expect("serialize");
        let roundtripped: CheckpointExtension =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtripped.value, ext.value);
    }

    #[test]
    fn new_extension_roundtrips_without_engine_schema_edit() {
        // Prove that a novel namespace can be registered, serialized, and
        // deserialized without any change to GraphSnapshotV2 or engine.rs.
        let mut exts = BTreeMap::new();
        let novel = CheckpointExtension {
            namespace: "third.party.custom".into(),
            schema_version: 7,
            required: false,
            fingerprint: "novel-fp".into(),
            value: serde_json::json!({"custom": "data", "version": 7}),
        };
        register_extension(&mut exts, novel).expect("register");

        let json = serde_json::to_string(&exts).expect("serialize map");
        let restored: BTreeMap<String, CheckpointExtension> =
            serde_json::from_str(&json).expect("deserialize map");

        assert_eq!(restored.len(), 1);
        let key = "third.party.custom@7";
        let restored_ext = restored.get(key).expect("key exists");
        assert_eq!(restored_ext.namespace, "third.party.custom");
        assert_eq!(restored_ext.schema_version, 7);
        assert_eq!(
            restored_ext.value,
            serde_json::json!({"custom": "data", "version": 7})
        );
    }

    // ─── Receipt state machine ──────────────────────────────────────────

    #[test]
    fn receipt_lifecycle_prepared_committed_settled() {
        let mut receipts = BTreeMap::new();
        let key = "task-1::attempt-0".to_string();

        // Prepare
        let entry = prepare_receipt(
            &mut receipts,
            key.clone(),
            "dispatcher".into(),
            "corr-1".into(),
            1000,
        );
        assert_eq!(entry.state, ReceiptState::Prepared);
        assert!(entry.evidence_ref.is_none());

        // Commit
        let entry = commit_receipt(&mut receipts, &key, Some("sha256:abc".into()), 2000)
            .expect("commit");
        assert_eq!(entry.state, ReceiptState::Committed);
        assert_eq!(entry.evidence_ref.as_deref(), Some("sha256:abc"));

        // Settle
        let entry = settle_receipt(&mut receipts, &key, 3000).expect("settle");
        assert_eq!(entry.state, ReceiptState::Settled);
    }

    #[test]
    fn receipt_prepare_idempotent() {
        let mut receipts = BTreeMap::new();
        let key = "dup".to_string();

        prepare_receipt(&mut receipts, key.clone(), "a".into(), "c1".into(), 1000);
        let entry = prepare_receipt(
            &mut receipts,
            key.clone(),
            "b".into(),
            "c2".into(),
            2000,
        );
        // Original owner preserved.
        assert_eq!(entry.owner, "a");
        assert_eq!(entry.correlation_id, "c1");
    }

    #[test]
    fn receipt_commit_idempotent_on_committed() {
        let mut receipts = BTreeMap::new();
        let key = "idem".to_string();

        prepare_receipt(&mut receipts, key.clone(), "o".into(), "c".into(), 1000);
        commit_receipt(&mut receipts, &key, Some("ev1".into()), 2000).expect("first commit");
        let entry = commit_receipt(&mut receipts, &key, Some("ev2".into()), 3000)
            .expect("repeat commit");
        // Evidence from first commit preserved.
        assert_eq!(entry.evidence_ref.as_deref(), Some("ev1"));
    }

    #[test]
    fn receipt_settle_fails_if_still_prepared() {
        let mut receipts = BTreeMap::new();
        let key = "skip".to_string();

        prepare_receipt(&mut receipts, key.clone(), "o".into(), "c".into(), 1000);
        let err = settle_receipt(&mut receipts, &key, 2000).unwrap_err();
        assert!(
            matches!(err, ReceiptError::SkippedTransition { .. }),
            "expected SkippedTransition, got {err:?}"
        );
    }

    #[test]
    fn receipt_commit_not_found() {
        let mut receipts: BTreeMap<String, ReceiptLedgerEntry> = BTreeMap::new();
        let err = commit_receipt(&mut receipts, "ghost", None, 1000).unwrap_err();
        assert!(matches!(err, ReceiptError::NotFound(_)));
    }

    #[test]
    fn receipt_settle_not_found() {
        let mut receipts: BTreeMap<String, ReceiptLedgerEntry> = BTreeMap::new();
        let err = settle_receipt(&mut receipts, "ghost", 1000).unwrap_err();
        assert!(matches!(err, ReceiptError::NotFound(_)));
    }

    #[test]
    fn receipt_settle_idempotent() {
        let mut receipts = BTreeMap::new();
        let key = "settled-twice".to_string();

        prepare_receipt(&mut receipts, key.clone(), "o".into(), "c".into(), 1000);
        commit_receipt(&mut receipts, &key, None, 2000).expect("commit");
        settle_receipt(&mut receipts, &key, 3000).expect("first settle");
        let entry = settle_receipt(&mut receipts, &key, 4000).expect("repeat settle");
        assert_eq!(entry.state, ReceiptState::Settled);
    }

    #[test]
    fn receipt_serde_roundtrip() {
        let entry = ReceiptLedgerEntry {
            idempotency_key: "key-1".into(),
            owner: "dispatcher".into(),
            correlation_id: "corr-1".into(),
            state: ReceiptState::Committed,
            evidence_ref: Some("sha:abc".into()),
            updated_at_ms: 12345,
            last_error: None,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let restored: ReceiptLedgerEntry =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, entry);
    }

    // ─── Extension registry ─────────────────────────────────────────────

    #[test]
    fn registry_validates_required_extensions() {
        let mut registry = ExtensionRegistry::new();
        register_known_namespaces(&mut registry);

        let mut exts = BTreeMap::new();
        // Known required: should pass.
        register_extension(
            &mut exts,
            CheckpointExtension {
                namespace: "roko.cost".into(),
                schema_version: 1,
                required: true,
                fingerprint: "f".into(),
                value: serde_json::json!({}),
            },
        )
        .unwrap();

        // Unknown required: should appear in unknown list.
        register_extension(
            &mut exts,
            CheckpointExtension {
                namespace: "future.feature".into(),
                schema_version: 1,
                required: true,
                fingerprint: "g".into(),
                value: serde_json::json!({}),
            },
        )
        .unwrap();

        let unknown = registry.validate_extensions(&exts);
        assert_eq!(unknown, vec!["future.feature@1"]);
    }

    #[test]
    fn registry_allows_unknown_optional_extensions() {
        let mut registry = ExtensionRegistry::new();
        register_known_namespaces(&mut registry);

        let mut exts = BTreeMap::new();
        register_extension(
            &mut exts,
            CheckpointExtension {
                namespace: "unknown.plugin".into(),
                schema_version: 1,
                required: false,
                fingerprint: "h".into(),
                value: serde_json::json!({"opaque": true}),
            },
        )
        .unwrap();

        let unknown = registry.validate_extensions(&exts);
        assert!(unknown.is_empty(), "optional unknown should not fail");
    }

    #[test]
    fn reconciler_callback_invoked() {
        let mut registry = ExtensionRegistry::new();
        registry.register_reconciler("test.ns", |data| {
            if data == b"valid" {
                ReconcileAction::Continue
            } else {
                ReconcileAction::Reset
            }
        });

        let reconciler = registry.reconciler("test.ns").expect("registered");
        assert_eq!(reconciler(b"valid"), ReconcileAction::Continue);
        assert_eq!(reconciler(b"invalid"), ReconcileAction::Reset);
    }

    #[test]
    fn reconciler_missing_returns_none() {
        let registry = ExtensionRegistry::new();
        assert!(registry.reconciler("unknown").is_none());
    }

    // ─── Receipt state ordering ─────────────────────────────────────────

    #[test]
    fn receipt_state_ordering() {
        assert!(ReceiptState::Prepared < ReceiptState::Committed);
        assert!(ReceiptState::Committed < ReceiptState::Settled);
    }

    // ─── Known namespace constants ──────────────────────────────────────

    #[test]
    fn known_namespace_format() {
        // All constants must follow `<dotted>@<version>` format.
        for ns in [
            EXT_COST,
            EXT_ACTIVITY,
            EXT_REPLAN,
            EXT_FEEDBACK,
            EXT_DELIVERY,
            EXT_CONTROL,
            EXT_GATE_HISTORY,
            EXT_APPROVAL,
            EXT_RUN_CONTEXT,
            EXT_SAFETY_PROVENANCE,
            EXT_EXPERIMENT,
        ] {
            assert!(
                ns.contains('@'),
                "namespace constant '{ns}' must contain '@'"
            );
            let parts: Vec<&str> = ns.split('@').collect();
            assert_eq!(parts.len(), 2, "namespace '{ns}' must have exactly one '@'");
            assert!(
                !parts[0].is_empty(),
                "namespace '{ns}' must have a non-empty name"
            );
            let _version: u32 = parts[1]
                .parse()
                .unwrap_or_else(|_| panic!("namespace '{ns}' version must be u32"));
        }
    }
}
