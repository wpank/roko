//! `TaskAttemptReceiptV1` -- the canonical feedback receipt for one task attempt.
//!
//! Every field is specified by the backlog #253 contract. The receipt captures
//! the immutable facts of a completed provider attempt so that downstream
//! settlement sinks never need to reach back into the live runner or provider.
//!
//! # Secret safety
//!
//! `prompt_ref` and `evidence_ref` carry *references* (file paths, content
//! hashes), never raw prompt text or secret-bearing payloads. The settler
//! populates these from the runner's already-scrubbed artifacts.

use serde::{Deserialize, Serialize};

/// Schema version for [`TaskAttemptReceiptV1`]. Bump on breaking changes.
pub const RECEIPT_SCHEMA_VERSION: u8 = 1;

/// Terminal status of one task attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptTerminalStatus {
    /// The attempt succeeded and passed all gates.
    Succeeded,
    /// The attempt ran to completion but failed gate validation.
    GateFailed,
    /// The attempt itself failed (provider error, timeout, crash).
    AttemptFailed,
    /// The attempt was cancelled before completion.
    Cancelled,
}

impl AttemptTerminalStatus {
    /// Whether this status represents a successful outcome.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

/// How the provider/model selection was made.
///
/// Learning sinks must not misattribute routing choices from manual overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceSource {
    /// Selected by the cascade router / bandit.
    Router,
    /// Manually overridden by the operator (e.g. `force_backend`).
    ManualOverride,
    /// Selected by an A/B experiment assignment.
    Experiment,
    /// Fallback / default selection.
    Default,
}

/// Canonical receipt for one completed task attempt.
///
/// This is the single input to the 12-row settlement pipeline. Every sink
/// reads fields from this receipt; no sink parses graph events directly.
///
/// # Idempotency
///
/// The `idempotency_key` uniquely identifies this receipt. Repeating
/// settlement with the same key is a no-op for already-settled rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAttemptReceiptV1 {
    // ── Identity ─────────────────────────────────────────────────────
    /// Schema version (always [`RECEIPT_SCHEMA_VERSION`]).
    pub schema_version: u8,
    /// Idempotency key. Format: `{run_id}:{plan_id}:{task_id}:{attempt}`.
    pub idempotency_key: String,
    /// Execution run identifier.
    pub run_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Graph node identifier (may differ from task_id in graph execution).
    pub node_id: String,
    /// Zero-based attempt number.
    pub attempt: u32,

    // ── Request fingerprint ──────────────────────────────────────────
    /// Deterministic fingerprint of the dispatch request (prompt + config).
    pub request_fingerprint: String,

    // ── Provider resolution ──────────────────────────────────────────
    /// Resolved provider name (e.g. "claude_cli", "anthropic_api").
    pub resolved_provider: String,
    /// Resolved model identifier.
    pub resolved_model: String,
    /// How the provider/model was selected.
    pub choice_source: ChoiceSource,

    // ── Prompt and evidence references ───────────────────────────────
    /// Reference to the composed prompt (path or content hash, never raw text).
    pub prompt_ref: String,
    /// Reference to the evidence artifacts (path or content hash).
    pub evidence_ref: String,

    // ── Timing ───────────────────────────────────────────────────────
    /// Unix milliseconds when the attempt started.
    pub start_time_ms: i64,
    /// Unix milliseconds when the attempt ended.
    pub end_time_ms: i64,

    // ── Outcome ──────────────────────────────────────────────────────
    /// Terminal status of the attempt.
    pub terminal_status: AttemptTerminalStatus,
    /// Error message for failed/cancelled attempts. Empty for success.
    #[serde(default)]
    pub error: String,

    // ── Token usage ──────────────────────────────────────────────────
    /// Input tokens consumed.
    pub tokens_in: u64,
    /// Output tokens produced.
    pub tokens_out: u64,

    // ── Cost ─────────────────────────────────────────────────────────
    /// Actual cost in micro-USD (1 USD = 1_000_000 micro-USD).
    pub actual_cost_micro_usd: u64,

    // ── Changed files ────────────────────────────────────────────────
    /// Files changed by this attempt.
    #[serde(default)]
    pub changed_files: Vec<String>,
    /// Deterministic fingerprint of the diff (e.g. BLAKE3 of patch).
    #[serde(default)]
    pub diff_fingerprint: String,

    // ── Gate verdict ─────────────────────────────────────────────────
    /// Reference to the gate verdict (path or content hash).
    #[serde(default)]
    pub gate_verdict_ref: String,

    // ── Experiment ───────────────────────────────────────────────────
    /// Experiment assignment ID, if this attempt was part of an A/B test.
    #[serde(default)]
    pub experiment_assignment: Option<String>,

    // ── Workspace ────────────────────────────────────────────────────
    /// Workspace lease reference from the outer controller.
    #[serde(default)]
    pub workspace_lease_ref: String,

    // ── Provider-attempt receipt ──────────────────────────────────────
    /// Opaque provider-attempt receipt from #247 (provider-specific metadata).
    #[serde(default)]
    pub provider_attempt_receipt: Option<serde_json::Value>,
}

impl TaskAttemptReceiptV1 {
    /// Construct a new receipt with the required identity fields.
    ///
    /// All other fields default to their zero/empty values. Callers should
    /// populate them before passing to the settler.
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        node_id: impl Into<String>,
        attempt: u32,
    ) -> Self {
        let run_id = run_id.into();
        let plan_id = plan_id.into();
        let task_id = task_id.into();
        let node_id = node_id.into();
        let idempotency_key = format!("{run_id}:{plan_id}:{task_id}:{attempt}");
        Self {
            schema_version: RECEIPT_SCHEMA_VERSION,
            idempotency_key,
            run_id,
            plan_id,
            task_id,
            node_id,
            attempt,
            request_fingerprint: String::new(),
            resolved_provider: String::new(),
            resolved_model: String::new(),
            choice_source: ChoiceSource::Default,
            prompt_ref: String::new(),
            evidence_ref: String::new(),
            start_time_ms: 0,
            end_time_ms: 0,
            terminal_status: AttemptTerminalStatus::AttemptFailed,
            error: String::new(),
            tokens_in: 0,
            tokens_out: 0,
            actual_cost_micro_usd: 0,
            changed_files: Vec::new(),
            diff_fingerprint: String::new(),
            gate_verdict_ref: String::new(),
            experiment_assignment: None,
            workspace_lease_ref: String::new(),
            provider_attempt_receipt: None,
        }
    }

    /// Duration of the attempt in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        (self.end_time_ms - self.start_time_ms).max(0) as u64
    }

    /// Whether this receipt represents a successful attempt.
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.terminal_status.is_success()
    }

    /// Cost in USD (floating point, for display / learning sinks).
    #[must_use]
    pub fn cost_usd(&self) -> f64 {
        self.actual_cost_micro_usd as f64 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_key_format() {
        let r = TaskAttemptReceiptV1::new("run-1", "plan-a", "task-b", "node-b", 2);
        assert_eq!(r.idempotency_key, "run-1:plan-a:task-b:2");
    }

    #[test]
    fn duration_ms_computation() {
        let mut r = TaskAttemptReceiptV1::new("r", "p", "t", "n", 0);
        r.start_time_ms = 1000;
        r.end_time_ms = 2500;
        assert_eq!(r.duration_ms(), 1500);
    }

    #[test]
    fn duration_ms_negative_clamped() {
        let mut r = TaskAttemptReceiptV1::new("r", "p", "t", "n", 0);
        r.start_time_ms = 3000;
        r.end_time_ms = 1000;
        assert_eq!(r.duration_ms(), 0);
    }

    #[test]
    fn cost_usd_conversion() {
        let mut r = TaskAttemptReceiptV1::new("r", "p", "t", "n", 0);
        r.actual_cost_micro_usd = 3_500_000;
        assert!((r.cost_usd() - 3.5).abs() < f64::EPSILON);
    }

    #[test]
    fn succeeded_mirrors_terminal_status() {
        let mut r = TaskAttemptReceiptV1::new("r", "p", "t", "n", 0);
        assert!(!r.succeeded()); // default is AttemptFailed
        r.terminal_status = AttemptTerminalStatus::Succeeded;
        assert!(r.succeeded());
        r.terminal_status = AttemptTerminalStatus::GateFailed;
        assert!(!r.succeeded());
        r.terminal_status = AttemptTerminalStatus::Cancelled;
        assert!(!r.succeeded());
    }

    #[test]
    fn serde_roundtrip() {
        let mut r = TaskAttemptReceiptV1::new("run-1", "plan-a", "task-b", "node-b", 0);
        r.resolved_provider = "claude_cli".into();
        r.resolved_model = "claude-sonnet-4-6".into();
        r.choice_source = ChoiceSource::Router;
        r.terminal_status = AttemptTerminalStatus::Succeeded;
        r.tokens_in = 200;
        r.tokens_out = 80;
        r.actual_cost_micro_usd = 3000;
        r.experiment_assignment = Some("exp-42".into());
        r.provider_attempt_receipt =
            Some(serde_json::json!({"request_id": "req-abc", "region": "us-west-2"}));

        let json = serde_json::to_string(&r).expect("serialize");
        let back: TaskAttemptReceiptV1 = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.schema_version, RECEIPT_SCHEMA_VERSION);
        assert_eq!(back.idempotency_key, "run-1:plan-a:task-b:0");
        assert_eq!(back.resolved_provider, "claude_cli");
        assert_eq!(back.resolved_model, "claude-sonnet-4-6");
        assert_eq!(back.choice_source, ChoiceSource::Router);
        assert_eq!(back.terminal_status, AttemptTerminalStatus::Succeeded);
        assert_eq!(back.tokens_in, 200);
        assert_eq!(back.tokens_out, 80);
        assert_eq!(back.actual_cost_micro_usd, 3000);
        assert_eq!(back.experiment_assignment, Some("exp-42".into()));
        assert!(back.provider_attempt_receipt.is_some());
    }

    #[test]
    fn choice_source_serde() {
        for source in [
            ChoiceSource::Router,
            ChoiceSource::ManualOverride,
            ChoiceSource::Experiment,
            ChoiceSource::Default,
        ] {
            let json = serde_json::to_string(&source).expect("serialize");
            let back: ChoiceSource = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(source, back, "roundtrip for {source:?}");
        }
    }

    #[test]
    fn terminal_status_serde() {
        for status in [
            AttemptTerminalStatus::Succeeded,
            AttemptTerminalStatus::GateFailed,
            AttemptTerminalStatus::AttemptFailed,
            AttemptTerminalStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let back: AttemptTerminalStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, back, "roundtrip for {status:?}");
        }
    }
}
