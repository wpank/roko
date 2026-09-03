//! Production gate pipeline request types.
//!
//! [`ProductionGateRequest`] carries all the context needed to execute the full
//! configured gate pipeline for a single task attempt. [`VerifyStepSpec`] is a
//! neutral, CLI-independent copy of the authored verify-step data that the
//! runner currently carries in `task_parser::VerifyStep`.

use std::path::PathBuf;

use roko_core::config::GatesConfig;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::adaptive_threshold::AdaptiveThresholds;

// ────────────────────────────────────────────────────────────────────────────
// VerifyStepSpec
// ────────────────────────────────────────────────────────────────────────────

/// Default timeout for a verify step (180 s).
const DEFAULT_VERIFY_STEP_TIMEOUT_MS: u64 = 180_000;

/// Neutral copy of the CLI `task_parser::VerifyStep`.
///
/// Conversion from the CLI type stays in `roko-cli` until #275 replaces the
/// runner gate path with `ProductionGateService`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyStepSpec {
    /// Phase label (e.g. `"structural"`, `"compile"`, `"test"`).
    #[serde(default)]
    pub phase: String,
    /// Shell command to run. Exit 0 = pass.
    pub command: String,
    /// Human-readable message shown on failure.
    #[serde(default)]
    pub fail_msg: Option<String>,
    /// Per-step timeout in milliseconds.
    #[serde(default = "default_verify_step_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_verify_step_timeout_ms() -> u64 {
    DEFAULT_VERIFY_STEP_TIMEOUT_MS
}

impl VerifyStepSpec {
    /// Create a verify step from a command string with defaults.
    #[must_use]
    pub fn from_command(command: impl Into<String>) -> Self {
        Self {
            phase: String::new(),
            command: command.into(),
            fail_msg: None,
            timeout_ms: DEFAULT_VERIFY_STEP_TIMEOUT_MS,
        }
    }

    /// Builder: set the phase label.
    #[must_use]
    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = phase.into();
        self
    }

    /// Builder: set the failure message.
    #[must_use]
    pub fn with_fail_msg(mut self, msg: impl Into<String>) -> Self {
        self.fail_msg = Some(msg.into());
        self
    }

    /// Builder: set the per-step timeout.
    #[must_use]
    pub const fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
}

// ────────────────────────────────────────────────────────────────────────────
// TaskContext
// ────────────────────────────────────────────────────────────────────────────

/// Task metadata carried into the gate pipeline for diagnostic/evidence use.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GateTaskContextSpec {
    /// Human-readable task title.
    #[serde(default)]
    pub title: String,
    /// Optional longer description.
    #[serde(default)]
    pub description: Option<String>,
    /// Relevant symbols for the symbol gate.
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Acceptance criteria sentences.
    #[serde(default)]
    pub acceptance: Vec<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// ProductionGateRequest
// ────────────────────────────────────────────────────────────────────────────

/// Complete request for a production gate pipeline execution.
///
/// This is the single entry-point contract for `ProductionGateRunner::run`.
/// It carries everything the service needs without reaching back into CLI
/// types or global state.
#[derive(Clone, Debug)]
pub struct ProductionGateRequest {
    // ── Identity ──────────────────────────────────────────────────────
    /// Run identifier (unique per plan execution).
    pub run_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Attempt number (0-based).
    pub attempt: u32,

    // ── Workspace ─────────────────────────────────────────────────────
    /// Root workspace path where gates execute.
    pub workspace: PathBuf,
    /// Content-hash fingerprint of the workspace state before gating.
    pub workspace_fingerprint: String,
    /// Files changed by the task (relative to workspace root).
    pub changed_files: Vec<String>,

    // ── Verify steps ──────────────────────────────────────────────────
    /// Authored per-task verify steps. Run after canonical rungs and
    /// before the structured-review rung.
    pub verify_steps: Vec<VerifyStepSpec>,

    // ── Configuration ─────────────────────────────────────────────────
    /// Full gate configuration from `roko.toml`.
    pub gates_config: GatesConfig,
    /// Task-level context for diagnostic enrichment.
    pub task_context: GateTaskContextSpec,

    // ── Timing ────────────────────────────────────────────────────────
    /// Overall pipeline timeout in seconds.
    pub timeout_secs: u64,
    /// Cooperative cancellation token.
    pub cancel: CancellationToken,

    // ── Baseline ──────────────────────────────────────────────────────
    /// Content-hash fingerprint from the pre-existing workspace baseline.
    /// When set, gates that failed identically before the current attempt
    /// can be filtered out (pre-existing failure policy).
    pub baseline_fingerprint: Option<String>,

    // ── Adaptive state ────────────────────────────────────────────────
    /// Snapshot of the adaptive thresholds *before* this pipeline run.
    /// The service reads skip/observe decisions from it but does not
    /// persist or settle it -- that is #251/#253 scope.
    pub adaptive_thresholds: Option<AdaptiveThresholds>,
}

impl ProductionGateRequest {
    /// Compute a composite request identity fingerprint for correlation.
    ///
    /// This is echoed in the verdict's `request_fingerprint` field so
    /// downstream consumers can match verdicts to requests.
    #[must_use]
    pub fn request_fingerprint(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.run_id, self.plan_id, self.task_id, self.attempt
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_step_spec_round_trip() {
        let step = VerifyStepSpec::from_command("cargo test")
            .with_phase("test")
            .with_fail_msg("tests failed")
            .with_timeout_ms(60_000);

        let json = serde_json::to_string(&step).expect("serialize");
        let back: VerifyStepSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.phase, "test");
        assert_eq!(back.command, "cargo test");
        assert_eq!(back.fail_msg.as_deref(), Some("tests failed"));
        assert_eq!(back.timeout_ms, 60_000);
    }

    #[test]
    fn verify_step_spec_defaults() {
        let step = VerifyStepSpec::from_command("true");
        assert!(step.phase.is_empty());
        assert!(step.fail_msg.is_none());
        assert_eq!(step.timeout_ms, DEFAULT_VERIFY_STEP_TIMEOUT_MS);
    }

    #[test]
    fn gate_task_context_spec_default() {
        let ctx = GateTaskContextSpec::default();
        assert!(ctx.title.is_empty());
        assert!(ctx.description.is_none());
        assert!(ctx.symbols.is_empty());
        assert!(ctx.acceptance.is_empty());
    }

    #[test]
    fn production_gate_request_can_be_constructed() {
        let req = ProductionGateRequest {
            run_id: "run-1".into(),
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            attempt: 0,
            workspace: PathBuf::from("/tmp/ws"),
            workspace_fingerprint: "abc123".into(),
            changed_files: vec!["src/lib.rs".into()],
            verify_steps: vec![VerifyStepSpec::from_command("cargo test")],
            gates_config: GatesConfig::default(),
            task_context: GateTaskContextSpec::default(),
            timeout_secs: 600,
            cancel: CancellationToken::new(),
            baseline_fingerprint: None,
            adaptive_thresholds: None,
        };
        assert_eq!(req.run_id, "run-1");
        assert_eq!(req.attempt, 0);
        assert_eq!(req.changed_files.len(), 1);
        assert_eq!(req.verify_steps.len(), 1);
    }

    #[test]
    fn request_fingerprint_format() {
        let req = ProductionGateRequest {
            run_id: "run-42".into(),
            plan_id: "plan-x".into(),
            task_id: "task-y".into(),
            attempt: 3,
            workspace: PathBuf::from("/tmp/ws"),
            workspace_fingerprint: "fp".into(),
            changed_files: Vec::new(),
            verify_steps: Vec::new(),
            gates_config: GatesConfig::default(),
            task_context: GateTaskContextSpec::default(),
            timeout_secs: 60,
            cancel: CancellationToken::new(),
            baseline_fingerprint: None,
            adaptive_thresholds: None,
        };
        assert_eq!(req.request_fingerprint(), "run-42:plan-x:task-y:3");
    }
}
