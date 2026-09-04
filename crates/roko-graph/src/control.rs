//! Executor-neutral approval, control, and cancellation ports (#255).
//!
//! This module defines the graph-local control types, approval lifecycle,
//! durable receipt state machine, and the [`ExecutionControlService`] that
//! translates UI/CLI control commands into scheduling effects.
//!
//! # Extension registration
//!
//! The control port registers `roko.control@1` with the #251 checkpoint
//! extension system. The extension stores pending approvals and command
//! receipts so they survive coordinator restarts.
//!
//! # Approval lifecycle
//!
//! Before every provider process spawn, the scheduler creates an
//! [`ApprovalRequestV1`]. The approval remains pending until it is
//! resolved by an `Approve` or `RejectApproval` command, or until its
//! deadline expires. Deadline expiration is identical to rejection and
//! launches zero work.
//!
//! # Receipt lifecycle
//!
//! Every command yields one [`ControlReceiptV1`]. The service writes a
//! `Received` receipt before applying the command and a terminal receipt
//! (`Applied`, `Rejected`, or `Finalized`) after. Duplicate command IDs
//! return the prior receipt without re-applying.
//!
//! # Scope boundary
//!
//! This module owns approval/control semantics and durable receipts. The
//! outer controller in #256 (plan) or #257 (workflow) alone flushes the
//! final checkpoint, follows #249 release policy, and writes run terminal
//! state. The graph control adapter never independently writes terminal
//! state or releases a #249 lease.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Extension identity
// ---------------------------------------------------------------------------

/// Checkpoint extension name registered with #251.
pub const CONTROL_EXTENSION_NAME: &str = "roko.control";

/// Checkpoint extension version. Bumped on breaking changes to the
/// persisted approval/receipt schema.
pub const CONTROL_EXTENSION_VERSION: u8 = 1;

// ---------------------------------------------------------------------------
// Approval request
// ---------------------------------------------------------------------------

/// A pre-spawn approval gate for a single provider process.
///
/// Created by the scheduler before every provider dispatch. The approval
/// remains pending until explicitly resolved or until `deadline_ms` elapses.
/// Deadline expiration is identical to rejection and launches zero work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestV1 {
    /// Unique approval identifier.
    pub approval_id: String,
    /// Graph execution run ID.
    pub run_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Node identifier within the graph.
    pub node_id: String,
    /// Zero-indexed attempt counter.
    pub attempt: u32,
    /// Human-readable summary of the capabilities/tools the provider will
    /// use if approved.
    pub capability_summary: String,
    /// Human-readable summary of the tool calls requested.
    pub tool_summary: String,
    /// Wall-clock deadline (milliseconds since UNIX epoch). After this
    /// instant, the approval is treated as rejected.
    pub deadline_ms: u64,
    /// Deterministic fingerprint of the request content for tamper detection.
    pub fingerprint: String,
    /// Wall-clock creation timestamp (milliseconds since UNIX epoch).
    pub created_at_ms: u64,
}

impl ApprovalRequestV1 {
    /// Check whether this approval has expired relative to the given wall
    /// clock time (milliseconds since UNIX epoch).
    #[must_use]
    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }

    /// Check whether this approval has expired relative to the current
    /// wall clock.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(now_ms())
    }
}

// ---------------------------------------------------------------------------
// Approval resolution
// ---------------------------------------------------------------------------

/// The resolution of a pending approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "resolution")]
pub enum ApprovalResolution {
    /// The approval was granted by an operator.
    Approved,
    /// The approval was rejected by an operator.
    Rejected {
        /// Human-readable reason for rejection.
        reason: String,
    },
    /// The approval deadline expired without a response.
    Expired,
}

impl ApprovalResolution {
    /// Whether the resolution allows provider work to proceed.
    #[must_use]
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved)
    }
}

impl std::fmt::Display for ApprovalResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approved => write!(f, "approved"),
            Self::Rejected { reason } => write!(f, "rejected: {reason}"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

// ---------------------------------------------------------------------------
// Control receipt
// ---------------------------------------------------------------------------

/// Status progression for a control receipt.
///
/// Advances in order: `Received -> (Applied | Rejected | Finalized)`.
/// Once in a terminal state, no further transitions are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    /// The command has been received but not yet applied.
    Received,
    /// The command was successfully applied.
    Applied,
    /// The command was rejected (e.g. stale ID, expired approval).
    Rejected,
    /// The command resulted in a finalization (cancellation, terminal reset).
    Finalized,
}

impl ReceiptStatus {
    /// Whether this is a terminal status (no further transitions allowed).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Applied | Self::Rejected | Self::Finalized)
    }
}

impl std::fmt::Display for ReceiptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Received => write!(f, "received"),
            Self::Applied => write!(f, "applied"),
            Self::Rejected => write!(f, "rejected"),
            Self::Finalized => write!(f, "finalized"),
        }
    }
}

/// A durable receipt for a single control command.
///
/// Written twice for each command: once at `Received` before applying, and
/// once at the terminal status after. Duplicate command IDs return the
/// stored receipt without re-applying.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlReceiptV1 {
    /// The command ID this receipt acknowledges.
    pub command_id: String,
    /// Correlation ID echoed from the command.
    pub correlation_id: String,
    /// The run ID that processed this command.
    pub run_id: String,
    /// Target plan ID (if the command was plan-scoped).
    pub plan_id: Option<String>,
    /// Target task ID (if the command was task-scoped).
    pub task_id: Option<String>,
    /// The command kind label (e.g. "approve", "cancel", "reset").
    pub command_kind: String,
    /// Current receipt status.
    pub status: ReceiptStatus,
    /// Optional human-readable message.
    pub message: Option<String>,
    /// Wall-clock timestamp (milliseconds since UNIX epoch) when this
    /// receipt was created or last updated.
    pub timestamp_ms: u64,
}

impl ControlReceiptV1 {
    /// Create a new receipt at `Received` status.
    #[must_use]
    pub fn received(
        command_id: impl Into<String>,
        correlation_id: impl Into<String>,
        run_id: impl Into<String>,
        plan_id: Option<String>,
        task_id: Option<String>,
        command_kind: impl Into<String>,
    ) -> Self {
        Self {
            command_id: command_id.into(),
            correlation_id: correlation_id.into(),
            run_id: run_id.into(),
            plan_id,
            task_id,
            command_kind: command_kind.into(),
            status: ReceiptStatus::Received,
            message: None,
            timestamp_ms: now_ms(),
        }
    }

    /// Transition to a terminal status. Returns `None` if the receipt is
    /// already terminal (no-op for idempotency).
    #[must_use]
    pub fn finalize(&self, status: ReceiptStatus, message: Option<String>) -> Option<Self> {
        if self.status.is_terminal() {
            return None;
        }
        let mut receipt = self.clone();
        receipt.status = status;
        receipt.message = message;
        receipt.timestamp_ms = now_ms();
        Some(receipt)
    }
}

// ---------------------------------------------------------------------------
// Control effect
// ---------------------------------------------------------------------------

/// The scheduling effect that the executor should apply after processing
/// a control command.
///
/// The control service translates commands into effects; the executor
/// applies them at safe scheduler boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ControlEffect {
    /// Set paused immediately; allow current provider/gate effects to
    /// finish; schedule no new node.
    Pause,
    /// Clear paused. Only valid when the run ID matches and cancellation
    /// has not begun.
    Resume,
    /// Create a new task attempt/lease after the current attempt is
    /// terminal.
    SoftRetry {
        plan_id: Option<String>,
        task_id: Option<String>,
    },
    /// Reset failed/pending nodes. When `preserve_completed` is false,
    /// reset all non-effect-committed task nodes. Never erases committed
    /// receipts.
    Repair { preserve_completed: bool },
    /// Invoke gates on the existing valid lease; launch no provider.
    ReverifyGates {
        plan_id: Option<String>,
        task_id: Option<String>,
    },
    /// Mark only a not-started task as skipped. Running/terminal tasks
    /// reject.
    Skip {
        plan_id: Option<String>,
        task_id: Option<String>,
    },
    /// Reset eligible graph state using the same receipt-preserving rules
    /// as repair-clean.
    Reset,
    /// Resolve a pending approval.
    ApprovalResolved {
        approval_id: String,
        resolution: ApprovalResolution,
    },
    /// Cancel graph token and every registered provider/gate process
    /// group, await supervisor acknowledgement, then return finalization
    /// intent to the outer controller.
    Cancel {
        plan_id: Option<String>,
        /// The finalization intent to return to the outer controller.
        intent: FinalizationIntent,
    },
    /// The command was rejected (stale run, expired approval, duplicate).
    Rejected {
        reason: String,
        receipt: ControlReceiptV1,
    },
}

// ---------------------------------------------------------------------------
// Finalization intent
// ---------------------------------------------------------------------------

/// Finalization intent returned by the control service after a cancel command.
///
/// The outer controller (#256/#257) uses this to flush the final checkpoint,
/// follow #249 release policy, and write run terminal state.
/// The graph control adapter never independently writes terminal state or
/// releases a #249 lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalizationIntent {
    /// The run ID that was cancelled.
    pub run_id: String,
    /// Plan ID if the cancellation was plan-scoped.
    pub plan_id: Option<String>,
    /// The durable receipt for the cancel command.
    pub receipt: ControlReceiptV1,
    /// Wall-clock timestamp when the finalization intent was created.
    pub created_at_ms: u64,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Manages pending approvals, durable receipts, and command-to-effect
/// translation for graph execution control.
///
/// # Thread safety
///
/// The service uses interior mutability via `parking_lot::Mutex` for the
/// approval and receipt stores. All methods take `&self`.
pub struct ExecutionControlService {
    run_id: String,
    approvals: parking_lot::Mutex<HashMap<String, ApprovalRequestV1>>,
    receipts: parking_lot::Mutex<HashMap<String, ControlReceiptV1>>,
    /// Whether a cancellation has been initiated. Once true, Resume and
    /// new dispatches are rejected.
    cancel_initiated: parking_lot::Mutex<bool>,
}

impl ExecutionControlService {
    /// Create a new control service for the given run.
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            approvals: parking_lot::Mutex::new(HashMap::new()),
            receipts: parking_lot::Mutex::new(HashMap::new()),
            cancel_initiated: parking_lot::Mutex::new(false),
        }
    }

    /// The run ID this service manages.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    // ── Approval management ─────────────────────────────────────────

    /// Register a pending approval request. Returns `true` if the request
    /// was newly inserted, `false` if a duplicate approval_id already existed.
    pub fn register_approval(&self, request: ApprovalRequestV1) -> bool {
        let mut approvals = self.approvals.lock();
        if approvals.contains_key(&request.approval_id) {
            return false;
        }
        approvals.insert(request.approval_id.clone(), request);
        true
    }

    /// Get a pending approval by ID, if one exists.
    #[must_use]
    pub fn get_approval(&self, approval_id: &str) -> Option<ApprovalRequestV1> {
        self.approvals.lock().get(approval_id).cloned()
    }

    /// List all pending approval IDs.
    #[must_use]
    pub fn pending_approval_ids(&self) -> Vec<String> {
        self.approvals.lock().keys().cloned().collect()
    }

    /// Resolve a pending approval. Returns the resolution effect or an
    /// error if the approval_id is not found or already expired.
    pub fn resolve_approval(
        &self,
        approval_id: &str,
        resolution: ApprovalResolution,
    ) -> Result<ApprovalRequestV1, String> {
        let mut approvals = self.approvals.lock();
        let request = approvals
            .remove(approval_id)
            .ok_or_else(|| format!("approval {approval_id} not found or already resolved"))?;

        if request.is_expired() && resolution.is_approved() {
            // Expired approvals cannot be approved -- treat as rejection.
            return Err(format!(
                "approval {approval_id} expired at {}",
                request.deadline_ms
            ));
        }

        Ok(request)
    }

    /// Expire all approvals whose deadlines have passed. Returns the
    /// expired approval IDs.
    pub fn expire_approvals(&self) -> Vec<String> {
        let now = now_ms();
        let mut approvals = self.approvals.lock();
        let expired: Vec<String> = approvals
            .iter()
            .filter(|(_, req)| req.is_expired_at(now))
            .map(|(id, _)| id.clone())
            .collect();
        for id in &expired {
            approvals.remove(id);
        }
        expired
    }

    // ── Receipt management ──────────────────────────────────────────

    /// Record a `Received` receipt for a command. If a receipt with the
    /// same command_id already exists, returns the stored receipt without
    /// modification (idempotency).
    pub fn record_received(
        &self,
        command_id: impl Into<String>,
        correlation_id: impl Into<String>,
        plan_id: Option<String>,
        task_id: Option<String>,
        command_kind: impl Into<String>,
    ) -> ControlReceiptV1 {
        let command_id = command_id.into();
        let mut receipts = self.receipts.lock();
        if let Some(existing) = receipts.get(&command_id) {
            return existing.clone();
        }
        let receipt = ControlReceiptV1::received(
            command_id.clone(),
            correlation_id,
            &self.run_id,
            plan_id,
            task_id,
            command_kind,
        );
        receipts.insert(command_id, receipt.clone());
        receipt
    }

    /// Finalize a receipt to a terminal status. Returns the updated
    /// receipt, or `None` if the receipt was not found or already terminal.
    pub fn finalize_receipt(
        &self,
        command_id: &str,
        status: ReceiptStatus,
        message: Option<String>,
    ) -> Option<ControlReceiptV1> {
        let mut receipts = self.receipts.lock();
        let existing = receipts.get(command_id)?;
        let updated = existing.finalize(status, message)?;
        receipts.insert(command_id.to_string(), updated.clone());
        Some(updated)
    }

    /// Get a receipt by command ID.
    #[must_use]
    pub fn get_receipt(&self, command_id: &str) -> Option<ControlReceiptV1> {
        self.receipts.lock().get(command_id).cloned()
    }

    /// Get all stored receipts, ordered by timestamp.
    #[must_use]
    pub fn all_receipts(&self) -> Vec<ControlReceiptV1> {
        let receipts = self.receipts.lock();
        let mut all: Vec<ControlReceiptV1> = receipts.values().cloned().collect();
        all.sort_by_key(|r| r.timestamp_ms);
        all
    }

    // ── Command processing ──────────────────────────────────────────

    /// Process a control command and return the effect the scheduler
    /// should apply. Also records durable receipts.
    ///
    /// Commands with a stale run ID are rejected. Duplicate command IDs
    /// return the prior receipt.
    #[allow(clippy::too_many_lines)]
    pub fn process_command(
        &self,
        command_id: &str,
        correlation_id: &str,
        run_id: &str,
        plan_id: Option<String>,
        task_id: Option<String>,
        kind: &ControlCommandKind,
    ) -> ControlEffect {
        // ── Run ID validation ───────────────────────────────────────
        if run_id != self.run_id {
            let receipt = ControlReceiptV1::received(
                command_id,
                correlation_id,
                run_id,
                plan_id.clone(),
                task_id.clone(),
                kind.label(),
            );
            let receipt = receipt
                .finalize(ReceiptStatus::Rejected, Some("stale run".into()))
                .unwrap_or(receipt);
            return ControlEffect::Rejected {
                reason: "stale run".into(),
                receipt,
            };
        }

        // ── Duplicate detection ─────────────────────────────────────
        if let Some(existing) = self.get_receipt(command_id)
            && existing.status.is_terminal()
        {
            return ControlEffect::Rejected {
                reason: "duplicate command_id".into(),
                receipt: existing,
            };
        }

        // ── Record Received ─────────────────────────────────────────
        let receipt = self.record_received(
            command_id,
            correlation_id,
            plan_id.clone(),
            task_id.clone(),
            kind.label(),
        );

        // ── Cancellation guard ──────────────────────────────────────
        let cancel_initiated = *self.cancel_initiated.lock();
        if cancel_initiated && matches!(kind, ControlCommandKind::Resume) {
            let receipt = self
                .finalize_receipt(
                    command_id,
                    ReceiptStatus::Rejected,
                    Some("cancellation already initiated".into()),
                )
                .unwrap_or(receipt);
            return ControlEffect::Rejected {
                reason: "cancellation already initiated".into(),
                receipt,
            };
        }

        // ── Dispatch by kind ────────────────────────────────────────
        match kind {
            ControlCommandKind::Pause => {
                self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                ControlEffect::Pause
            }
            ControlCommandKind::Resume => {
                self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                ControlEffect::Resume
            }
            ControlCommandKind::SoftRetry => {
                self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                ControlEffect::SoftRetry { plan_id, task_id }
            }
            ControlCommandKind::Repair { preserve_completed } => {
                self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                ControlEffect::Repair {
                    preserve_completed: *preserve_completed,
                }
            }
            ControlCommandKind::ReverifyGates => {
                self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                ControlEffect::ReverifyGates { plan_id, task_id }
            }
            ControlCommandKind::Skip => {
                self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                ControlEffect::Skip { plan_id, task_id }
            }
            ControlCommandKind::Reset => {
                self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                ControlEffect::Reset
            }
            ControlCommandKind::Approve { approval_id } => {
                match self.resolve_approval(approval_id, ApprovalResolution::Approved) {
                    Ok(_request) => {
                        self.finalize_receipt(command_id, ReceiptStatus::Applied, None);
                        ControlEffect::ApprovalResolved {
                            approval_id: approval_id.clone(),
                            resolution: ApprovalResolution::Approved,
                        }
                    }
                    Err(reason) => {
                        let receipt = self
                            .finalize_receipt(
                                command_id,
                                ReceiptStatus::Rejected,
                                Some(reason.clone()),
                            )
                            .unwrap_or(receipt);
                        ControlEffect::Rejected { reason, receipt }
                    }
                }
            }
            ControlCommandKind::RejectApproval {
                approval_id,
                reason,
            } => {
                let resolution = ApprovalResolution::Rejected {
                    reason: reason.clone(),
                };
                match self.resolve_approval(approval_id, resolution.clone()) {
                    Ok(_request) => {
                        self.finalize_receipt(
                            command_id,
                            ReceiptStatus::Applied,
                            Some(reason.clone()),
                        );
                        ControlEffect::ApprovalResolved {
                            approval_id: approval_id.clone(),
                            resolution,
                        }
                    }
                    Err(err_reason) => {
                        let receipt = self
                            .finalize_receipt(
                                command_id,
                                ReceiptStatus::Rejected,
                                Some(err_reason.clone()),
                            )
                            .unwrap_or(receipt);
                        ControlEffect::Rejected {
                            reason: err_reason,
                            receipt,
                        }
                    }
                }
            }
            ControlCommandKind::Cancel => {
                *self.cancel_initiated.lock() = true;
                let cancel_receipt = self
                    .finalize_receipt(
                        command_id,
                        ReceiptStatus::Finalized,
                        Some("cancellation initiated".into()),
                    )
                    .unwrap_or(receipt);
                let intent = FinalizationIntent {
                    run_id: self.run_id.clone(),
                    plan_id: plan_id.clone(),
                    receipt: cancel_receipt.clone(),
                    created_at_ms: now_ms(),
                };
                ControlEffect::Cancel { plan_id, intent }
            }
        }
    }

    /// Whether a cancellation has been initiated for this run.
    #[must_use]
    pub fn is_cancel_initiated(&self) -> bool {
        *self.cancel_initiated.lock()
    }

    /// Snapshot the current state for checkpoint persistence.
    #[must_use]
    pub fn snapshot(&self) -> ControlSnapshot {
        ControlSnapshot {
            run_id: self.run_id.clone(),
            pending_approvals: self.approvals.lock().clone(),
            receipts: self.receipts.lock().clone(),
            cancel_initiated: *self.cancel_initiated.lock(),
        }
    }

    /// Restore state from a checkpoint snapshot.
    pub fn restore(&self, snapshot: ControlSnapshot) {
        *self.approvals.lock() = snapshot.pending_approvals;
        *self.receipts.lock() = snapshot.receipts;
        *self.cancel_initiated.lock() = snapshot.cancel_initiated;
    }
}

impl std::fmt::Debug for ExecutionControlService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionControlService")
            .field("run_id", &self.run_id)
            .field("pending_approvals", &self.approvals.lock().len())
            .field("receipts", &self.receipts.lock().len())
            .field("cancel_initiated", &*self.cancel_initiated.lock())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Checkpoint snapshot
// ---------------------------------------------------------------------------

/// Serializable snapshot of the control service state for checkpoint
/// persistence via the #251 extension system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSnapshot {
    /// The run ID this snapshot belongs to.
    pub run_id: String,
    /// Pending approvals indexed by approval_id.
    pub pending_approvals: HashMap<String, ApprovalRequestV1>,
    /// Command receipts indexed by command_id.
    pub receipts: HashMap<String, ControlReceiptV1>,
    /// Whether cancellation has been initiated.
    pub cancel_initiated: bool,
}

// ---------------------------------------------------------------------------
// Control command kind (graph-layer view)
// ---------------------------------------------------------------------------

/// The control command discriminant at the graph layer.
///
/// This mirrors `ExecutionCommandKind` from `roko-cli` without creating a
/// dependency from `roko-graph` to `roko-cli`. The CLI control adapter
/// maps between them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ControlCommandKind {
    /// Pause the executor.
    Pause,
    /// Resume the executor.
    Resume,
    /// Retry failed tasks.
    SoftRetry,
    /// Repair a plan.
    Repair { preserve_completed: bool },
    /// Re-run gate checks.
    ReverifyGates,
    /// Skip a task.
    Skip,
    /// Reset eligible graph state.
    Reset,
    /// Approve a pending approval.
    Approve { approval_id: String },
    /// Reject a pending approval.
    RejectApproval { approval_id: String, reason: String },
    /// Cancel the run.
    Cancel,
}

impl ControlCommandKind {
    /// Human-readable label for this command kind.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::SoftRetry => "soft-retry",
            Self::Repair { .. } => "repair",
            Self::ReverifyGates => "reverify-gates",
            Self::Skip => "skip",
            Self::Reset => "reset",
            Self::Approve { .. } => "approve",
            Self::RejectApproval { .. } => "reject-approval",
            Self::Cancel => "cancel",
        }
    }
}

impl std::fmt::Display for ControlCommandKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pause => write!(f, "pause"),
            Self::Resume => write!(f, "resume"),
            Self::SoftRetry => write!(f, "soft-retry"),
            Self::Repair { preserve_completed } => {
                write!(f, "repair(preserve={preserve_completed})")
            }
            Self::ReverifyGates => write!(f, "reverify-gates"),
            Self::Skip => write!(f, "skip"),
            Self::Reset => write!(f, "reset"),
            Self::Approve { approval_id } => write!(f, "approve({approval_id})"),
            Self::RejectApproval {
                approval_id,
                reason,
            } => write!(f, "reject-approval({approval_id}, {reason})"),
            Self::Cancel => write!(f, "cancel"),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Milliseconds since UNIX epoch.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

/// Build an approval request with a default deadline of 5 minutes.
#[must_use]
pub fn build_approval_request(
    approval_id: impl Into<String>,
    run_id: impl Into<String>,
    plan_id: impl Into<String>,
    task_id: impl Into<String>,
    node_id: impl Into<String>,
    attempt: u32,
    capability_summary: impl Into<String>,
    tool_summary: impl Into<String>,
) -> ApprovalRequestV1 {
    let now = now_ms();
    ApprovalRequestV1 {
        approval_id: approval_id.into(),
        run_id: run_id.into(),
        plan_id: plan_id.into(),
        task_id: task_id.into(),
        node_id: node_id.into(),
        attempt,
        capability_summary: capability_summary.into(),
        tool_summary: tool_summary.into(),
        deadline_ms: now + 5 * 60 * 1000, // 5 minutes
        fingerprint: String::new(),
        created_at_ms: now,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> ExecutionControlService {
        ExecutionControlService::new("run-test")
    }

    fn test_approval(id: &str) -> ApprovalRequestV1 {
        let now = now_ms();
        ApprovalRequestV1 {
            approval_id: id.to_string(),
            run_id: "run-test".to_string(),
            plan_id: "plan-1".to_string(),
            task_id: "task-1".to_string(),
            node_id: "node-1".to_string(),
            attempt: 0,
            capability_summary: "file_write, shell_exec".to_string(),
            tool_summary: "write config.rs, run tests".to_string(),
            deadline_ms: now + 300_000, // 5 min from now
            fingerprint: "fp-test".to_string(),
            created_at_ms: now,
        }
    }

    fn expired_approval(id: &str) -> ApprovalRequestV1 {
        ApprovalRequestV1 {
            approval_id: id.to_string(),
            run_id: "run-test".to_string(),
            plan_id: "plan-1".to_string(),
            task_id: "task-1".to_string(),
            node_id: "node-1".to_string(),
            attempt: 0,
            capability_summary: "file_write".to_string(),
            tool_summary: "write file".to_string(),
            deadline_ms: 1, // expired
            fingerprint: "fp-expired".to_string(),
            created_at_ms: 0,
        }
    }

    // ── Approval lifecycle ──────────────────────────────────────────

    #[test]
    fn register_and_resolve_approval() {
        let svc = test_service();
        let req = test_approval("ap-1");
        assert!(svc.register_approval(req.clone()));
        // Duplicate registration returns false.
        assert!(!svc.register_approval(req));
        assert_eq!(svc.pending_approval_ids().len(), 1);

        let resolved = svc
            .resolve_approval("ap-1", ApprovalResolution::Approved)
            .unwrap();
        assert_eq!(resolved.approval_id, "ap-1");
        assert!(svc.pending_approval_ids().is_empty());
    }

    #[test]
    fn resolve_nonexistent_approval_fails() {
        let svc = test_service();
        let result = svc.resolve_approval("ap-missing", ApprovalResolution::Approved);
        assert!(result.is_err());
    }

    #[test]
    fn expired_approval_cannot_be_approved() {
        let svc = test_service();
        svc.register_approval(expired_approval("ap-exp"));
        let result = svc.resolve_approval("ap-exp", ApprovalResolution::Approved);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expired"));
    }

    #[test]
    fn expire_approvals_removes_stale() {
        let svc = test_service();
        svc.register_approval(expired_approval("ap-old"));
        svc.register_approval(test_approval("ap-fresh"));
        let expired = svc.expire_approvals();
        assert_eq!(expired, vec!["ap-old"]);
        assert_eq!(svc.pending_approval_ids(), vec!["ap-fresh"]);
    }

    #[test]
    fn approval_rejection_removes_from_pending() {
        let svc = test_service();
        svc.register_approval(test_approval("ap-rej"));
        let resolved = svc
            .resolve_approval(
                "ap-rej",
                ApprovalResolution::Rejected {
                    reason: "unsafe".into(),
                },
            )
            .unwrap();
        assert_eq!(resolved.approval_id, "ap-rej");
        assert!(svc.pending_approval_ids().is_empty());
    }

    // ── Receipt lifecycle ───────────────────────────────────────────

    #[test]
    fn receipt_received_then_applied() {
        let svc = test_service();
        let r1 = svc.record_received("cmd-1", "cor-1", None, None, "pause");
        assert_eq!(r1.status, ReceiptStatus::Received);

        let r2 = svc
            .finalize_receipt("cmd-1", ReceiptStatus::Applied, None)
            .unwrap();
        assert_eq!(r2.status, ReceiptStatus::Applied);

        // Cannot finalize again (already terminal).
        assert!(
            svc.finalize_receipt("cmd-1", ReceiptStatus::Rejected, None)
                .is_none()
        );
    }

    #[test]
    fn duplicate_command_id_returns_existing_receipt() {
        let svc = test_service();
        let r1 = svc.record_received("cmd-dup", "cor-1", None, None, "pause");
        let r2 = svc.record_received("cmd-dup", "cor-2", None, None, "resume");
        // The second call returns the first receipt unchanged.
        assert_eq!(r1.command_id, r2.command_id);
        assert_eq!(r1.correlation_id, r2.correlation_id);
        assert_eq!(r1.command_kind, r2.command_kind);
    }

    #[test]
    fn all_receipts_ordered_by_timestamp() {
        let svc = test_service();
        svc.record_received("cmd-a", "cor-a", None, None, "pause");
        svc.record_received("cmd-b", "cor-b", None, None, "resume");
        let all = svc.all_receipts();
        assert_eq!(all.len(), 2);
        assert!(all[0].timestamp_ms <= all[1].timestamp_ms);
    }

    // ── Command processing ──────────────────────────────────────────

    #[test]
    fn process_pause_command() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-p1",
            "cor-p1",
            "run-test",
            None,
            None,
            &ControlCommandKind::Pause,
        );
        assert_eq!(effect, ControlEffect::Pause);
        let receipt = svc.get_receipt("cmd-p1").unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Applied);
    }

    #[test]
    fn process_resume_command() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-r1",
            "cor-r1",
            "run-test",
            None,
            None,
            &ControlCommandKind::Resume,
        );
        assert_eq!(effect, ControlEffect::Resume);
    }

    #[test]
    fn stale_run_rejected() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-s1",
            "cor-s1",
            "run-stale",
            None,
            None,
            &ControlCommandKind::Pause,
        );
        assert!(matches!(effect, ControlEffect::Rejected { reason, .. } if reason == "stale run"));
    }

    #[test]
    fn duplicate_command_id_rejected() {
        let svc = test_service();
        svc.process_command(
            "cmd-d1",
            "cor-d1",
            "run-test",
            None,
            None,
            &ControlCommandKind::Pause,
        );
        let effect = svc.process_command(
            "cmd-d1",
            "cor-d1",
            "run-test",
            None,
            None,
            &ControlCommandKind::Resume,
        );
        assert!(
            matches!(effect, ControlEffect::Rejected { reason, .. } if reason == "duplicate command_id")
        );
    }

    #[test]
    fn cancel_sets_flag_and_blocks_resume() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-c1",
            "cor-c1",
            "run-test",
            Some("plan-1".into()),
            None,
            &ControlCommandKind::Cancel,
        );
        assert!(matches!(effect, ControlEffect::Cancel { .. }));
        assert!(svc.is_cancel_initiated());

        // Resume after cancel is rejected.
        let effect = svc.process_command(
            "cmd-r2",
            "cor-r2",
            "run-test",
            None,
            None,
            &ControlCommandKind::Resume,
        );
        assert!(
            matches!(effect, ControlEffect::Rejected { reason, .. } if reason == "cancellation already initiated")
        );
    }

    #[test]
    fn process_approve_with_registered_approval() {
        let svc = test_service();
        svc.register_approval(test_approval("ap-cmd"));
        let effect = svc.process_command(
            "cmd-a1",
            "cor-a1",
            "run-test",
            Some("plan-1".into()),
            Some("task-1".into()),
            &ControlCommandKind::Approve {
                approval_id: "ap-cmd".into(),
            },
        );
        assert!(matches!(
            effect,
            ControlEffect::ApprovalResolved {
                resolution: ApprovalResolution::Approved,
                ..
            }
        ));
    }

    #[test]
    fn process_approve_without_registration_rejected() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-a2",
            "cor-a2",
            "run-test",
            None,
            None,
            &ControlCommandKind::Approve {
                approval_id: "ap-missing".into(),
            },
        );
        assert!(matches!(effect, ControlEffect::Rejected { .. }));
    }

    #[test]
    fn process_reject_approval() {
        let svc = test_service();
        svc.register_approval(test_approval("ap-rej-cmd"));
        let effect = svc.process_command(
            "cmd-rj1",
            "cor-rj1",
            "run-test",
            None,
            None,
            &ControlCommandKind::RejectApproval {
                approval_id: "ap-rej-cmd".into(),
                reason: "dangerous tool".into(),
            },
        );
        assert!(matches!(
            effect,
            ControlEffect::ApprovalResolved {
                resolution: ApprovalResolution::Rejected { .. },
                ..
            }
        ));
    }

    #[test]
    fn process_reset_command() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-rs1",
            "cor-rs1",
            "run-test",
            None,
            None,
            &ControlCommandKind::Reset,
        );
        assert_eq!(effect, ControlEffect::Reset);
    }

    #[test]
    fn process_soft_retry_command() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-sr1",
            "cor-sr1",
            "run-test",
            Some("plan-1".into()),
            Some("task-1".into()),
            &ControlCommandKind::SoftRetry,
        );
        assert_eq!(
            effect,
            ControlEffect::SoftRetry {
                plan_id: Some("plan-1".into()),
                task_id: Some("task-1".into()),
            }
        );
    }

    #[test]
    fn process_repair_command() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-rp1",
            "cor-rp1",
            "run-test",
            Some("plan-1".into()),
            None,
            &ControlCommandKind::Repair {
                preserve_completed: true,
            },
        );
        assert_eq!(
            effect,
            ControlEffect::Repair {
                preserve_completed: true,
            }
        );
    }

    #[test]
    fn process_reverify_gates_command() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-rv1",
            "cor-rv1",
            "run-test",
            Some("plan-1".into()),
            Some("task-1".into()),
            &ControlCommandKind::ReverifyGates,
        );
        assert_eq!(
            effect,
            ControlEffect::ReverifyGates {
                plan_id: Some("plan-1".into()),
                task_id: Some("task-1".into()),
            }
        );
    }

    #[test]
    fn process_skip_command() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-sk1",
            "cor-sk1",
            "run-test",
            Some("plan-1".into()),
            Some("task-1".into()),
            &ControlCommandKind::Skip,
        );
        assert_eq!(
            effect,
            ControlEffect::Skip {
                plan_id: Some("plan-1".into()),
                task_id: Some("task-1".into()),
            }
        );
    }

    // ── Snapshot / restore ──────────────────────────────────────────

    #[test]
    fn snapshot_and_restore() {
        let svc = test_service();
        svc.register_approval(test_approval("ap-snap"));
        svc.process_command(
            "cmd-snap",
            "cor-snap",
            "run-test",
            None,
            None,
            &ControlCommandKind::Pause,
        );

        let snapshot = svc.snapshot();
        assert!(snapshot.pending_approvals.contains_key("ap-snap"));
        assert!(snapshot.receipts.contains_key("cmd-snap"));

        // Restore into a fresh service.
        let svc2 = ExecutionControlService::new("run-test");
        svc2.restore(snapshot.clone());
        assert_eq!(svc2.snapshot(), snapshot);
    }

    // ── ControlCommandKind display ──────────────────────────────────

    #[test]
    fn control_command_kind_display() {
        assert_eq!(ControlCommandKind::Pause.to_string(), "pause");
        assert_eq!(ControlCommandKind::Resume.to_string(), "resume");
        assert_eq!(ControlCommandKind::SoftRetry.to_string(), "soft-retry");
        assert_eq!(
            ControlCommandKind::Repair {
                preserve_completed: true
            }
            .to_string(),
            "repair(preserve=true)"
        );
        assert_eq!(
            ControlCommandKind::ReverifyGates.to_string(),
            "reverify-gates"
        );
        assert_eq!(ControlCommandKind::Skip.to_string(), "skip");
        assert_eq!(ControlCommandKind::Reset.to_string(), "reset");
        assert_eq!(
            ControlCommandKind::Approve {
                approval_id: "ap-1".into()
            }
            .to_string(),
            "approve(ap-1)"
        );
        assert_eq!(
            ControlCommandKind::RejectApproval {
                approval_id: "ap-2".into(),
                reason: "bad".into()
            }
            .to_string(),
            "reject-approval(ap-2, bad)"
        );
        assert_eq!(ControlCommandKind::Cancel.to_string(), "cancel");
    }

    #[test]
    fn control_command_kind_labels() {
        assert_eq!(ControlCommandKind::Pause.label(), "pause");
        assert_eq!(ControlCommandKind::Cancel.label(), "cancel");
        assert_eq!(
            ControlCommandKind::Approve {
                approval_id: "x".into()
            }
            .label(),
            "approve"
        );
        assert_eq!(
            ControlCommandKind::RejectApproval {
                approval_id: "x".into(),
                reason: "y".into()
            }
            .label(),
            "reject-approval"
        );
        assert_eq!(ControlCommandKind::Reset.label(), "reset");
    }

    // ── Receipt status ──────────────────────────────────────────────

    #[test]
    fn receipt_status_is_terminal() {
        assert!(!ReceiptStatus::Received.is_terminal());
        assert!(ReceiptStatus::Applied.is_terminal());
        assert!(ReceiptStatus::Rejected.is_terminal());
        assert!(ReceiptStatus::Finalized.is_terminal());
    }

    #[test]
    fn receipt_status_display() {
        assert_eq!(ReceiptStatus::Received.to_string(), "received");
        assert_eq!(ReceiptStatus::Applied.to_string(), "applied");
        assert_eq!(ReceiptStatus::Rejected.to_string(), "rejected");
        assert_eq!(ReceiptStatus::Finalized.to_string(), "finalized");
    }

    // ── ApprovalResolution ──────────────────────────────────────────

    #[test]
    fn approval_resolution_display() {
        assert_eq!(ApprovalResolution::Approved.to_string(), "approved");
        assert_eq!(
            ApprovalResolution::Rejected {
                reason: "bad tool".into()
            }
            .to_string(),
            "rejected: bad tool"
        );
        assert_eq!(ApprovalResolution::Expired.to_string(), "expired");
    }

    #[test]
    fn approval_resolution_is_approved() {
        assert!(ApprovalResolution::Approved.is_approved());
        assert!(!ApprovalResolution::Rejected { reason: "x".into() }.is_approved());
        assert!(!ApprovalResolution::Expired.is_approved());
    }

    // ── Serde round-trip ────────────────────────────────────────────

    #[test]
    fn approval_request_serde_roundtrip() {
        let req = test_approval("ap-serde");
        let json = serde_json::to_string(&req).unwrap();
        let parsed: ApprovalRequestV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(req, parsed);
    }

    #[test]
    fn control_receipt_serde_roundtrip() {
        let receipt = ControlReceiptV1::received(
            "cmd-serde",
            "cor-serde",
            "run-test",
            Some("plan-1".into()),
            None,
            "approve",
        );
        let json = serde_json::to_string(&receipt).unwrap();
        let parsed: ControlReceiptV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt, parsed);
    }

    #[test]
    fn control_snapshot_serde_roundtrip() {
        let svc = test_service();
        svc.register_approval(test_approval("ap-ss"));
        svc.process_command(
            "cmd-ss",
            "cor-ss",
            "run-test",
            None,
            None,
            &ControlCommandKind::Pause,
        );
        let snapshot = svc.snapshot();
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: ControlSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot, parsed);
    }

    #[test]
    fn finalization_intent_serde_roundtrip() {
        let receipt = ControlReceiptV1::received(
            "cmd-fi",
            "cor-fi",
            "run-test",
            Some("plan-1".into()),
            None,
            "cancel",
        );
        let intent = FinalizationIntent {
            run_id: "run-test".into(),
            plan_id: Some("plan-1".into()),
            receipt,
            created_at_ms: now_ms(),
        };
        let json = serde_json::to_string(&intent).unwrap();
        let parsed: FinalizationIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(intent, parsed);
    }

    // ── Cancel finalization intent ──────────────────────────────────

    #[test]
    fn cancel_produces_finalization_intent() {
        let svc = test_service();
        let effect = svc.process_command(
            "cmd-fin",
            "cor-fin",
            "run-test",
            Some("plan-1".into()),
            None,
            &ControlCommandKind::Cancel,
        );
        match effect {
            ControlEffect::Cancel { plan_id, intent } => {
                assert_eq!(plan_id.as_deref(), Some("plan-1"));
                assert_eq!(intent.run_id, "run-test");
                assert_eq!(intent.plan_id.as_deref(), Some("plan-1"));
                assert_eq!(intent.receipt.status, ReceiptStatus::Finalized);
                assert!(intent.created_at_ms > 0);
            }
            other => panic!("expected Cancel effect, got {other:?}"),
        }
    }

    // ── Debug impl ──────────────────────────────────────────────────

    #[test]
    fn debug_impl_does_not_panic() {
        let svc = test_service();
        let debug = format!("{svc:?}");
        assert!(debug.contains("ExecutionControlService"));
        assert!(debug.contains("run-test"));
    }

    // ── All ten command kinds produce effects ───────────────────────

    #[test]
    fn all_command_kinds_produce_effects() {
        let svc = test_service();
        svc.register_approval(test_approval("ap-all"));

        let kinds = vec![
            ControlCommandKind::Pause,
            ControlCommandKind::Resume,
            ControlCommandKind::SoftRetry,
            ControlCommandKind::Repair {
                preserve_completed: false,
            },
            ControlCommandKind::ReverifyGates,
            ControlCommandKind::Skip,
            ControlCommandKind::Reset,
            ControlCommandKind::Approve {
                approval_id: "ap-all".into(),
            },
            ControlCommandKind::RejectApproval {
                approval_id: "ap-missing-ok".into(),
                reason: "test".into(),
            },
            ControlCommandKind::Cancel,
        ];

        let mut effects = Vec::new();
        for (i, kind) in kinds.iter().enumerate() {
            let cmd_id = format!("cmd-all-{i}");
            let cor_id = format!("cor-all-{i}");
            let effect = svc.process_command(
                &cmd_id,
                &cor_id,
                "run-test",
                Some("p".into()),
                Some("t".into()),
                kind,
            );
            effects.push(effect);
        }

        assert_eq!(effects.len(), 10);
        assert_eq!(effects[0], ControlEffect::Pause);
        assert_eq!(effects[1], ControlEffect::Resume);
        assert!(matches!(effects[2], ControlEffect::SoftRetry { .. }));
        assert!(matches!(effects[3], ControlEffect::Repair { .. }));
        assert!(matches!(effects[4], ControlEffect::ReverifyGates { .. }));
        assert!(matches!(effects[5], ControlEffect::Skip { .. }));
        assert_eq!(effects[6], ControlEffect::Reset);
        assert!(matches!(effects[7], ControlEffect::ApprovalResolved { .. }));
        // ap-missing-ok was never registered, so rejection of the reject
        // command itself is expected.
        assert!(matches!(effects[8], ControlEffect::Rejected { .. }));
        assert!(matches!(effects[9], ControlEffect::Cancel { .. }));
    }
}
