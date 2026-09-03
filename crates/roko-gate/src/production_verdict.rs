//! Production gate pipeline verdict types.
//!
//! [`ProductionGateRungVerdict`] records the outcome of a single rung.
//! [`ProductionGateVerdictV1`] is the versioned aggregate verdict for the
//! entire pipeline run — replay-safe, serializable, and sufficient for
//! downstream consumers (#251 persistence, #253 feedback, #275 runner parity).

use std::time::Duration;

use roko_core::TestCount;
use serde::{Deserialize, Serialize};

use crate::adaptive_threshold::AdaptiveThresholds;
use crate::compile_errors::GateFailureClassification;
use crate::rung_selector::Rung;

/// Maximum raw output bytes retained inline per rung verdict.
///
/// Full output beyond this limit is written to the generated-artifact store
/// and referenced by [`EvidenceRef`].
pub const MAX_RAW_OUTPUT_BYTES: usize = 64 * 1024;

// ────────────────────────────────────────────────────────────────────────────
// Evidence reference
// ────────────────────────────────────────────────────────────────────────────

/// Durable reference to gate evidence stored outside the verdict.
///
/// Full output exceeding [`MAX_RAW_OUTPUT_BYTES`] is written through the
/// existing artifact store and referenced here by content hash or file path.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Content-addressable hash of the full output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// Relative path within the artifact store.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
}

impl EvidenceRef {
    /// Whether this reference points to stored evidence.
    #[must_use]
    pub fn is_populated(&self) -> bool {
        self.content_hash.is_some() || self.artifact_path.is_some()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Per-rung verdict
// ────────────────────────────────────────────────────────────────────────────

/// State of a single rung after pipeline execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RungState {
    /// The rung was selected and passed.
    Passed,
    /// The rung was selected and failed.
    Failed,
    /// The rung was not executed (adaptive skip, max-rung cap, etc.).
    Skipped,
}

/// Verdict for a single rung of the production gate pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionGateRungVerdict {
    /// Canonical rung that was executed.
    pub rung: Rung,
    /// Concrete gate name within the rung (e.g. `"compile:cargo"`).
    pub gate_name: String,
    /// Outcome of this rung.
    pub state: RungState,
    /// Structured failure classification (populated on failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_classification: Option<GateFailureClassification>,
    /// Bounded diagnostic output (capped at [`MAX_RAW_OUTPUT_BYTES`]).
    #[serde(default)]
    pub diagnostic: String,
    /// Reference to full durable evidence when output exceeds the cap.
    #[serde(default)]
    pub evidence: EvidenceRef,
    /// Wall-clock duration for this rung.
    pub duration: Duration,
    /// Test counts when the rung is a test gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_counts: Option<TestCount>,
    /// Content-hash fingerprint of the inputs to this rung.
    #[serde(default)]
    pub input_fingerprint: String,
    /// Reason the rung was skipped (only meaningful when `state == Skipped`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

impl ProductionGateRungVerdict {
    /// Convenience: did this rung pass?
    #[must_use]
    pub const fn passed(&self) -> bool {
        matches!(self.state, RungState::Passed)
    }

    /// Convenience: was this rung skipped?
    #[must_use]
    pub const fn skipped(&self) -> bool {
        matches!(self.state, RungState::Skipped)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Aggregate pipeline verdict
// ────────────────────────────────────────────────────────────────────────────

/// Overall pipeline outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineOutcome {
    /// All selected rungs passed.
    Passed,
    /// At least one selected rung failed.
    Failed,
    /// The pipeline was cancelled before completing.
    Cancelled,
    /// The pipeline timed out before completing.
    TimedOut,
}

/// Schema version for the production gate verdict.
pub const VERDICT_SCHEMA_VERSION: u32 = 1;

/// Versioned aggregate verdict for a full production gate pipeline run.
///
/// This is the canonical output of [`ProductionGateRunner::run`] and the
/// type that downstream consumers (#275 runner, #251 persistence) depend on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionGateVerdictV1 {
    /// Schema version — always [`VERDICT_SCHEMA_VERSION`] (1).
    pub schema_version: u32,

    // ── Identity fingerprints ─────────────────────────────────────────
    /// Fingerprint from the request (echoed for correlation).
    pub request_fingerprint: String,
    /// Workspace fingerprint at the time of execution.
    pub workspace_fingerprint: String,

    // ── Rung verdicts ─────────────────────────────────────────────────
    /// Ordered per-rung verdicts (in execution order = CANONICAL_ORDER).
    pub rung_verdicts: Vec<ProductionGateRungVerdict>,

    // ── Aggregate state ───────────────────────────────────────────────
    /// Overall pipeline outcome.
    pub outcome: PipelineOutcome,
    /// `true` when the only failures are in non-required rungs and the
    /// core compile+test rungs passed (the "mostly passing" policy).
    pub mostly_passing: bool,
    /// Total wall-clock duration for the entire pipeline.
    pub total_duration: Duration,

    // ── Adaptive state ────────────────────────────────────────────────
    /// Adaptive threshold snapshot *after* this pipeline run.
    ///
    /// The service updates skip/observe decisions but does not persist or
    /// settle the thresholds -- that is #251/#253 scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adaptive_snapshot: Option<AdaptiveThresholds>,
}

impl ProductionGateVerdictV1 {
    /// Convenience: did the pipeline pass?
    #[must_use]
    pub const fn passed(&self) -> bool {
        matches!(self.outcome, PipelineOutcome::Passed)
    }

    /// Count of rungs that were actually executed (not skipped).
    #[must_use]
    pub fn executed_rung_count(&self) -> usize {
        self.rung_verdicts.iter().filter(|v| !v.skipped()).count()
    }

    /// Count of rungs that failed.
    #[must_use]
    pub fn failed_rung_count(&self) -> usize {
        self.rung_verdicts
            .iter()
            .filter(|v| matches!(v.state, RungState::Failed))
            .count()
    }

    /// Names of the rungs that failed.
    #[must_use]
    pub fn failed_rung_names(&self) -> Vec<String> {
        self.rung_verdicts
            .iter()
            .filter(|v| matches!(v.state, RungState::Failed))
            .map(|v| v.gate_name.clone())
            .collect()
    }

    /// Aggregate test counts across all rungs that reported them.
    #[must_use]
    pub fn aggregate_test_counts(&self) -> Option<TestCount> {
        let mut acc: Option<TestCount> = None;
        for rv in &self.rung_verdicts {
            if let Some(tc) = rv.test_counts {
                acc = Some(match acc {
                    None => tc,
                    Some(a) => TestCount::new(
                        a.passed.saturating_add(tc.passed),
                        a.failed.saturating_add(tc.failed),
                        a.ignored.saturating_add(tc.ignored),
                    ),
                });
            }
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rung_verdict(rung: Rung, state: RungState) -> ProductionGateRungVerdict {
        ProductionGateRungVerdict {
            rung,
            gate_name: rung.label().to_string(),
            state,
            failure_classification: None,
            diagnostic: String::new(),
            evidence: EvidenceRef::default(),
            duration: Duration::from_millis(100),
            test_counts: None,
            input_fingerprint: String::new(),
            skip_reason: None,
        }
    }

    #[test]
    fn verdict_v1_round_trip() {
        let verdict = ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: "req-fp".into(),
            workspace_fingerprint: "ws-fp".into(),
            rung_verdicts: vec![
                make_rung_verdict(Rung::Compile, RungState::Passed),
                make_rung_verdict(Rung::Lint, RungState::Passed),
                make_rung_verdict(Rung::Test, RungState::Failed),
            ],
            outcome: PipelineOutcome::Failed,
            mostly_passing: false,
            total_duration: Duration::from_secs(42),
            adaptive_snapshot: None,
        };
        assert!(!verdict.passed());
        assert_eq!(verdict.executed_rung_count(), 3);
        assert_eq!(verdict.failed_rung_count(), 1);
        assert_eq!(verdict.failed_rung_names(), vec!["test"]);

        let json = serde_json::to_string(&verdict).expect("serialize");
        let back: ProductionGateVerdictV1 = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.schema_version, VERDICT_SCHEMA_VERSION);
        assert_eq!(back.outcome, PipelineOutcome::Failed);
        assert_eq!(back.rung_verdicts.len(), 3);
    }

    #[test]
    fn passed_verdict() {
        let verdict = ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: String::new(),
            workspace_fingerprint: String::new(),
            rung_verdicts: vec![
                make_rung_verdict(Rung::Compile, RungState::Passed),
                make_rung_verdict(Rung::Lint, RungState::Skipped),
            ],
            outcome: PipelineOutcome::Passed,
            mostly_passing: false,
            total_duration: Duration::from_millis(500),
            adaptive_snapshot: None,
        };
        assert!(verdict.passed());
        assert_eq!(verdict.executed_rung_count(), 1);
        assert_eq!(verdict.failed_rung_count(), 0);
    }

    #[test]
    fn aggregate_test_counts_combines() {
        let mut rv1 = make_rung_verdict(Rung::Test, RungState::Passed);
        rv1.test_counts = Some(TestCount::new(10, 0, 1));
        let mut rv2 = make_rung_verdict(Rung::PropertyTest, RungState::Passed);
        rv2.test_counts = Some(TestCount::new(5, 1, 0));

        let verdict = ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: String::new(),
            workspace_fingerprint: String::new(),
            rung_verdicts: vec![rv1, rv2],
            outcome: PipelineOutcome::Passed,
            mostly_passing: false,
            total_duration: Duration::ZERO,
            adaptive_snapshot: None,
        };

        let tc = verdict.aggregate_test_counts().expect("has test counts");
        assert_eq!(tc.passed, 15);
        assert_eq!(tc.failed, 1);
        assert_eq!(tc.ignored, 1);
    }

    #[test]
    fn aggregate_test_counts_none_when_no_rungs_report() {
        let verdict = ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: String::new(),
            workspace_fingerprint: String::new(),
            rung_verdicts: vec![make_rung_verdict(Rung::Compile, RungState::Passed)],
            outcome: PipelineOutcome::Passed,
            mostly_passing: false,
            total_duration: Duration::ZERO,
            adaptive_snapshot: None,
        };
        assert!(verdict.aggregate_test_counts().is_none());
    }

    #[test]
    fn evidence_ref_populated_check() {
        let empty = EvidenceRef::default();
        assert!(!empty.is_populated());

        let with_hash = EvidenceRef {
            content_hash: Some("sha256:abc".into()),
            artifact_path: None,
        };
        assert!(with_hash.is_populated());

        let with_path = EvidenceRef {
            content_hash: None,
            artifact_path: Some("artifacts/gate-output.txt".into()),
        };
        assert!(with_path.is_populated());
    }

    #[test]
    fn rung_verdict_state_helpers() {
        let passed = make_rung_verdict(Rung::Compile, RungState::Passed);
        assert!(passed.passed());
        assert!(!passed.skipped());

        let skipped = make_rung_verdict(Rung::Lint, RungState::Skipped);
        assert!(!skipped.passed());
        assert!(skipped.skipped());

        let failed = make_rung_verdict(Rung::Test, RungState::Failed);
        assert!(!failed.passed());
        assert!(!failed.skipped());
    }

    #[test]
    fn pipeline_outcome_variants_serialize() {
        for outcome in [
            PipelineOutcome::Passed,
            PipelineOutcome::Failed,
            PipelineOutcome::Cancelled,
            PipelineOutcome::TimedOut,
        ] {
            let json = serde_json::to_string(&outcome).expect("serialize");
            let back: PipelineOutcome = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, outcome);
        }
    }
}
