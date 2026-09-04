//! Typed cell payload contracts for graph-level inter-cell communication.
//!
//! Each payload struct is the canonical serialized form for a specific cell
//! kind's output Signal. Cells serialize via `Signal::with_typed_payload()`
//! and consumers deserialize via `signal.typed_payload::<T>()`.

use serde::{Deserialize, Serialize};

// ────────────────────────────────────────────────────────────────────────────
// Gate payloads
// ────────────────────────────────────────────────────────────────────────────

/// Typed output payload for [`GatePipelineCell`].
///
/// Maps from the internal `ProductionGateVerdictV1` service contract to a
/// graph-consumable form. Consumers can pattern-match on `passed` without
/// importing the full gate crate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateResult {
    /// Whether the overall gate pipeline passed.
    pub passed: bool,
    /// Per-rung results in execution order.
    pub rung_results: Vec<RungResult>,
    /// Aggregate score (0.0 = all failed, 1.0 = all passed).
    pub overall_score: f64,
}

/// Result for a single rung within the gate pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RungResult {
    /// Canonical rung name (e.g. "compile", "lint", "test").
    pub rung_name: String,
    /// Whether this rung passed.
    pub passed: bool,
    /// Score for this rung (0.0 = failed, 1.0 = passed, intermediate for partial).
    pub score: f64,
    /// Diagnostic evidence (bounded output).
    pub evidence: Option<String>,
}

impl GateResult {
    /// Create a passed result with no rung details.
    #[must_use]
    pub fn passed() -> Self {
        Self {
            passed: true,
            rung_results: Vec::new(),
            overall_score: 1.0,
        }
    }

    /// Create a failed result with no rung details.
    #[must_use]
    pub fn failed() -> Self {
        Self {
            passed: false,
            rung_results: Vec::new(),
            overall_score: 0.0,
        }
    }

    /// Names of the rungs that failed.
    #[must_use]
    pub fn failed_rung_names(&self) -> Vec<&str> {
        self.rung_results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| r.rung_name.as_str())
            .collect()
    }

    /// Count of rungs that passed.
    #[must_use]
    pub fn passed_count(&self) -> usize {
        self.rung_results.iter().filter(|r| r.passed).count()
    }

    /// Count of rungs that failed.
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.rung_results.iter().filter(|r| !r.passed).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_result_round_trip() {
        let result = GateResult {
            passed: false,
            rung_results: vec![
                RungResult {
                    rung_name: "compile".into(),
                    passed: true,
                    score: 1.0,
                    evidence: None,
                },
                RungResult {
                    rung_name: "test".into(),
                    passed: false,
                    score: 0.0,
                    evidence: Some("3 failures".into()),
                },
            ],
            overall_score: 0.5,
        };

        let json = serde_json::to_string(&result).expect("serialize");
        let back: GateResult = serde_json::from_str(&json).expect("deserialize");
        assert!(!back.passed);
        assert_eq!(back.rung_results.len(), 2);
        assert_eq!(back.overall_score, 0.5);
    }

    #[test]
    fn gate_result_helpers() {
        let result = GateResult {
            passed: false,
            rung_results: vec![
                RungResult {
                    rung_name: "compile".into(),
                    passed: true,
                    score: 1.0,
                    evidence: None,
                },
                RungResult {
                    rung_name: "test".into(),
                    passed: false,
                    score: 0.0,
                    evidence: Some("error".into()),
                },
            ],
            overall_score: 0.5,
        };

        assert_eq!(result.passed_count(), 1);
        assert_eq!(result.failed_count(), 1);
        assert_eq!(result.failed_rung_names(), vec!["test"]);
    }

    #[test]
    fn gate_result_convenience_constructors() {
        let p = GateResult::passed();
        assert!(p.passed);
        assert_eq!(p.overall_score, 1.0);
        assert!(p.rung_results.is_empty());

        let f = GateResult::failed();
        assert!(!f.passed);
        assert_eq!(f.overall_score, 0.0);
    }
}
