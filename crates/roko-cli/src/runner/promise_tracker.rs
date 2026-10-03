//! Promise-based early termination tracker for doomed task attempts.
//!
//! Wraps [`ProcessRewardModel`] from `roko-gate` with consecutive-low-promise
//! tracking so the runner can abandon attempts that show no signs of converging.
//!
//! Usage: create one `PromiseTracker` per task attempt. After each gate
//! completion, call [`PromiseTracker::record_and_check`] with a snapshot built
//! from the gate verdicts. When [`PromiseDecision::Terminate`] is returned, the
//! runner should fail the task early.

use roko_core::Verdict;
use roko_gate::{ProcessRewardModel, TurnSnapshot};

/// Decision returned by [`PromiseTracker::record_and_check`].
#[derive(Debug, Clone, PartialEq)]
pub enum PromiseDecision {
    /// The attempt should continue.
    Continue,
    /// The attempt should be terminated early.
    Terminate {
        /// Current promise score at the time of termination.
        promise: f64,
        /// How many consecutive turns the promise was below the threshold.
        consecutive_turns: u32,
    },
}

/// Tracks a [`ProcessRewardModel`] per task attempt and adds consecutive-low-
/// promise counting so the runner can detect and terminate doomed attempts.
pub struct PromiseTracker {
    prm: ProcessRewardModel,
    consecutive_low: u32,
    min_promise: f64,
    consecutive_threshold: u32,
}

impl PromiseTracker {
    /// Create a new tracker with default thresholds (min_promise = 0.2,
    /// consecutive_threshold = 2).
    pub fn new() -> Self {
        Self {
            prm: ProcessRewardModel::new(),
            consecutive_low: 0,
            min_promise: 0.2,
            consecutive_threshold: 2,
        }
    }

    /// Override the minimum promise threshold (default: 0.2). Graph verify
    /// takes M1's B6 knob from here (8125).
    #[must_use]
    pub fn with_min_promise(mut self, min: f64) -> Self {
        self.min_promise = min;
        self
    }

    /// Override the consecutive low-promise turn count required before
    /// termination (default: 2).
    #[must_use]
    pub fn with_consecutive_threshold(mut self, n: u32) -> Self {
        self.consecutive_threshold = n;
        self
    }

    /// Record a turn's gate results and check whether the attempt should be
    /// terminated early.
    ///
    /// Builds a [`TurnSnapshot`] from the gate completion data, feeds it to
    /// the underlying PRM, and checks whether promise has been below
    /// `min_promise` for `consecutive_threshold` consecutive turns.
    pub fn record_and_check(&mut self, snapshot: TurnSnapshot) -> PromiseDecision {
        self.prm.record_turn(snapshot);

        if self.prm.should_terminate(self.min_promise) {
            self.consecutive_low += 1;
        } else {
            self.consecutive_low = 0;
        }

        if self.consecutive_low >= self.consecutive_threshold {
            PromiseDecision::Terminate {
                promise: self.prm.promise(),
                consecutive_turns: self.consecutive_low,
            }
        } else {
            PromiseDecision::Continue
        }
    }

    /// Current promise score (probability of eventual success).
    #[must_use]
    #[allow(dead_code)] // used in tests and Terminate variant; exposed for future instrumentation
    pub fn promise(&self) -> f64 {
        self.prm.promise()
    }

    /// Current progress score (trajectory delta between last two turns).
    #[must_use]
    #[allow(dead_code)] // used in tests; exposed for future runner instrumentation
    pub fn progress(&self) -> f64 {
        self.prm.progress()
    }
}

/// Build a [`TurnSnapshot`] from runner gate completion data.
///
/// Converts the runner's `GateVerdictSummary` list into the PRM's expected
/// `Verdict` format and extracts error counts from verdict summaries.
#[allow(dead_code)] // used only in tests
pub fn snapshot_from_gate_completion(
    rung: u32,
    verdicts: &[super::types::GateVerdictSummary],
    diff_lines: u32,
) -> TurnSnapshot {
    let core_verdicts: Vec<Verdict> = verdicts
        .iter()
        .filter(|v| !v.skipped)
        .map(|v| {
            let mut verdict = if v.passed {
                Verdict::pass(&v.gate_name)
            } else {
                Verdict::fail(&v.gate_name, &v.summary)
            };
            // Preserve the gate name from the summary.
            verdict.gate = v.gate_name.clone();
            verdict
        })
        .collect();

    let error_count = verdicts.iter().filter(|v| !v.skipped && !v.passed).count() as u32;

    TurnSnapshot {
        rung,
        verdicts: core_verdicts,
        error_count,
        diff_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::types::GateVerdictSummary;

    fn make_verdict(name: &str, passed: bool) -> GateVerdictSummary {
        GateVerdictSummary {
            gate_name: name.to_string(),
            passed,
            skipped: false,
            summary: if passed {
                "ok".to_string()
            } else {
                "error[E0308]: mismatched types".to_string()
            },
            error_digest: None,
            failure_kind: None,
            rung_index: None,
        }
    }

    #[test]
    fn new_tracker_starts_with_neutral_promise() {
        let tracker = PromiseTracker::new();
        assert!((tracker.promise() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn single_turn_always_continues() {
        let mut tracker = PromiseTracker::new();
        let snapshot = snapshot_from_gate_completion(
            1,
            &[make_verdict("compile", false), make_verdict("test", false)],
            200,
        );
        let decision = tracker.record_and_check(snapshot);
        assert_eq!(decision, PromiseDecision::Continue);
    }

    #[test]
    fn consecutive_failures_terminate() {
        let mut tracker = PromiseTracker::new()
            .with_min_promise(0.4)
            .with_consecutive_threshold(2);

        // Two turns of all-failing verdicts with no progression.
        let snap1 = snapshot_from_gate_completion(
            0,
            &[make_verdict("compile", false), make_verdict("lint", false)],
            200,
        );
        let snap2 = snapshot_from_gate_completion(
            0,
            &[make_verdict("compile", false), make_verdict("lint", false)],
            300,
        );
        // First all-fail: PRM needs >= 2 turns, so should_terminate is false
        // after first turn. consecutive_low stays 0.
        let d1 = tracker.record_and_check(snap1);
        assert_eq!(d1, PromiseDecision::Continue);

        // Second all-fail: PRM has 2 turns, promise should be low.
        let d2 = tracker.record_and_check(snap2);
        // After 2 turns of all failure with growing diffs, promise < 0.4.
        // But consecutive_low is only 1 here because PRM first becomes active
        // at turn 2. We need a third turn to hit threshold 2.
        if d2 == PromiseDecision::Continue {
            let snap3 = snapshot_from_gate_completion(
                0,
                &[make_verdict("compile", false), make_verdict("lint", false)],
                400,
            );
            let d3 = tracker.record_and_check(snap3);
            assert!(
                matches!(d3, PromiseDecision::Terminate { .. }),
                "expected Terminate after 3 consecutive failing turns, got {d3:?}"
            );
        }
    }

    #[test]
    fn recovery_resets_consecutive_count() {
        let mut tracker = PromiseTracker::new()
            .with_min_promise(0.4)
            .with_consecutive_threshold(2);

        // Two failing turns.
        let snap1 = snapshot_from_gate_completion(0, &[make_verdict("compile", false)], 200);
        let snap2 = snapshot_from_gate_completion(0, &[make_verdict("compile", false)], 300);
        tracker.record_and_check(snap1);
        tracker.record_and_check(snap2);

        // Recovery turn: passing verdicts, rung advancement.
        let snap3 = snapshot_from_gate_completion(
            3,
            &[make_verdict("compile", true), make_verdict("test", true)],
            50,
        );
        let d3 = tracker.record_and_check(snap3);
        assert_eq!(d3, PromiseDecision::Continue);
        // The consecutive counter should have been reset.
        assert!(tracker.promise() > 0.4);
    }

    #[test]
    fn skipped_verdicts_are_excluded() {
        let verdicts = vec![
            GateVerdictSummary {
                gate_name: "compile".to_string(),
                passed: true,
                skipped: false,
                summary: "ok".to_string(),
                error_digest: None,
                failure_kind: None,
                rung_index: None,
            },
            GateVerdictSummary {
                gate_name: "integration".to_string(),
                passed: false,
                skipped: true,
                summary: "not wired".to_string(),
                error_digest: None,
                failure_kind: None,
                rung_index: None,
            },
        ];
        let snapshot = snapshot_from_gate_completion(1, &verdicts, 100);
        // Only the non-skipped compile verdict should be included.
        assert_eq!(snapshot.verdicts.len(), 1);
        assert!(snapshot.verdicts[0].passed);
        assert_eq!(snapshot.error_count, 0);
    }

    #[test]
    fn snapshot_error_count_from_failed_verdicts() {
        let verdicts = vec![
            make_verdict("compile", false),
            make_verdict("lint", false),
            make_verdict("test", true),
        ];
        let snapshot = snapshot_from_gate_completion(2, &verdicts, 150);
        assert_eq!(snapshot.error_count, 2);
        assert_eq!(snapshot.verdicts.len(), 3);
        assert_eq!(snapshot.rung, 2);
        assert_eq!(snapshot.diff_lines, 150);
    }
}
