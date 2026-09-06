//! Arena flywheel — trace collection (Stage 1) and auto-grading (Stage 2).
//!
//! The remaining five stages (Preference-Mine, Failure-Cluster, Curriculum-Gen,
//! Pattern-Extract, Preference-Bootstrap) are specified in
//! `tmp/evals-audit/arena-flywheel-spec.md` and reserved for a future batch.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::arena::AttemptTrace;

// ────────────────────────────────────────────────────────────────────────────
// Stage 1 — Trace Collection
// ────────────────────────────────────────────────────────────────────────────

/// Durable append-only store of [`AttemptTrace`] records for one workspace.
///
/// Each arena's traces land in a dedicated subdirectory so they can be
/// collected independently without listing the whole store.
///
/// Layout:
/// ```text
/// .roko/arenas/<arena_hex>/traces/<attempt_hex>.jsonl
/// ```
pub struct TraceCollector {
    /// Root of the trace store — typically `<workspace>/.roko/arenas`.
    traces_dir: PathBuf,
}

impl TraceCollector {
    /// Create a collector rooted at `traces_dir`.
    ///
    /// The directory is created lazily on first write; it need not exist yet.
    #[must_use]
    pub fn new(traces_dir: PathBuf) -> Self {
        Self { traces_dir }
    }

    /// Convenience constructor that derives the store path from the workspace root.
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use roko_chain::arena_flywheel::TraceCollector;
    /// let collector = TraceCollector::from_workspace(Path::new("."));
    /// ```
    #[must_use]
    pub fn from_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".roko").join("arenas"))
    }

    fn arena_dir(&self, arena_id: &[u8; 32]) -> PathBuf {
        self.traces_dir
            .join(format_hex(arena_id))
            .join("traces")
    }

    fn trace_path(&self, arena_id: &[u8; 32], attempt_id: &[u8; 32]) -> PathBuf {
        self.arena_dir(arena_id)
            .join(format!("{}.jsonl", format_hex(attempt_id)))
    }

    /// Persist `trace` for the given arena/attempt pair.
    ///
    /// The arena traces directory is created if it does not exist.  Writing is
    /// idempotent: a second call for the same attempt_id overwrites the file
    /// (traces are deterministic given the same settlement inputs).
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if directory creation or file writing fails.
    pub fn collect_trace(
        &self,
        arena_id: &[u8; 32],
        attempt_id: &[u8; 32],
        trace: &AttemptTrace,
    ) -> io::Result<()> {
        let dir = self.arena_dir(arena_id);
        fs::create_dir_all(&dir)?;
        let path = self.trace_path(arena_id, attempt_id);
        let line =
            serde_json::to_string(trace).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        fs::write(path, format!("{line}\n"))
    }

    /// Read all persisted traces for `arena_id`.
    ///
    /// Files that fail to deserialize are silently skipped so a corrupt entry
    /// does not block access to the rest of the store.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] only if the traces directory cannot be listed.
    /// Individual file read or parse failures are logged and skipped.
    pub fn read_traces(&self, arena_id: &[u8; 32]) -> io::Result<Vec<AttemptTrace>> {
        let dir = self.arena_dir(arena_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut traces = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    tracing::warn!(%error, "arena trace: could not read directory entry");
                    continue;
                }
            };
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            let raw = match fs::read_to_string(&path) {
                Ok(raw) => raw,
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "arena trace: could not read file");
                    continue;
                }
            };
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<AttemptTrace>(line) {
                    Ok(trace) => traces.push(trace),
                    Err(error) => {
                        tracing::warn!(
                            %error,
                            path = %path.display(),
                            "arena trace: could not deserialize entry"
                        );
                    }
                }
            }
        }
        Ok(traces)
    }

    /// Count persisted traces for `arena_id` without deserializing them.
    ///
    /// Returns 0 if the arena directory does not yet exist.
    #[must_use]
    pub fn trace_count(&self, arena_id: &[u8; 32]) -> usize {
        let dir = self.arena_dir(arena_id);
        if !dir.exists() {
            return 0;
        }
        fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            == Some("jsonl")
                    })
                    .count()
            })
            .unwrap_or(0)
    }
}

fn format_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

// ────────────────────────────────────────────────────────────────────────────
// Stage 2 — Auto-Grading
// ────────────────────────────────────────────────────────────────────────────

/// Grading result produced by [`AutoGrader::grade`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradeResult {
    /// Normalized score in `[0, 1]`.
    pub score: f64,
    /// Letter grade: `"A"`, `"B"`, `"C"`, `"D"`, or `"F"`.
    pub grade_label: String,
    /// Human-readable explanation of how the grade was derived.
    pub reasoning: String,
}

impl GradeResult {
    /// Map a raw pass-rate in `[0, 1]` to a [`GradeResult`].
    fn from_pass_rate(pass_rate: f64, gate_count: usize, dimension_count: usize) -> Self {
        let (grade_label, reasoning) = if pass_rate >= 0.90 {
            (
                "A".to_string(),
                format!(
                    "Pass rate {:.0}% across {} gate(s) and {} scoring dimension(s).",
                    pass_rate * 100.0,
                    gate_count,
                    dimension_count
                ),
            )
        } else if pass_rate >= 0.75 {
            (
                "B".to_string(),
                format!(
                    "Pass rate {:.0}% — strong but a minority of gates failed.",
                    pass_rate * 100.0
                ),
            )
        } else if pass_rate >= 0.55 {
            (
                "C".to_string(),
                format!(
                    "Pass rate {:.0}% — marginal; roughly half the gates passed.",
                    pass_rate * 100.0
                ),
            )
        } else if pass_rate >= 0.30 {
            (
                "D".to_string(),
                format!(
                    "Pass rate {:.0}% — most gates failed.",
                    pass_rate * 100.0
                ),
            )
        } else {
            (
                "F".to_string(),
                format!(
                    "Pass rate {:.0}% — attempt did not meet minimum gate requirements.",
                    pass_rate * 100.0
                ),
            )
        };
        Self {
            score: pass_rate,
            grade_label,
            reasoning,
        }
    }
}

/// Grades an [`AttemptTrace`] from its gate verdicts and scoring dimensions
/// without requiring an external settlement.
///
/// The grader is stateless; construct one per call site or share a single
/// instance across threads — it holds no mutable state.
#[derive(Debug, Default, Clone)]
pub struct AutoGrader;

impl AutoGrader {
    /// Create a new auto-grader.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compute a [`GradeResult`] for `trace`.
    ///
    /// The algorithm:
    ///
    /// 1. Compute the gate pass-rate from [`AttemptTrace::gate_verdicts`].
    /// 2. If scoring dimensions are present, compute their normalized mean and
    ///    blend it with the gate pass-rate (equal weight).
    /// 3. Map the blended score to a letter grade via fixed thresholds:
    ///    - A ≥ 0.90, B ≥ 0.75, C ≥ 0.55, D ≥ 0.30, F < 0.30.
    ///
    /// If no gate verdicts and no scoring dimensions are present the result is
    /// an F with score 0.0.
    #[must_use]
    pub fn grade(&self, trace: &AttemptTrace) -> GradeResult {
        let gate_count = trace.gate_verdicts.len();
        let dimension_count = trace.scoring_dimensions.len();

        // Pass-rate component (NaN-safe).
        let gate_score = if gate_count == 0 {
            None
        } else {
            let passed = trace.gate_verdicts.iter().filter(|&&v| v).count();
            Some(passed as f64 / gate_count as f64)
        };

        // Scoring-dimension component: mean of all dimension values clamped to
        // [0, 1].  Dimensions that are already normalised (Pareto) are presumed
        // to live in this range; unclamped values are clamped defensively.
        let dim_score = if dimension_count == 0 {
            None
        } else {
            let sum: f64 = trace
                .scoring_dimensions
                .iter()
                .map(|(_, value)| value.clamp(0.0, 1.0))
                .sum();
            Some(sum / dimension_count as f64)
        };

        let pass_rate = match (gate_score, dim_score) {
            (Some(g), Some(d)) => (g + d) / 2.0,
            (Some(g), None) => g,
            (None, Some(d)) => d,
            (None, None) => {
                return GradeResult {
                    score: 0.0,
                    grade_label: "F".to_string(),
                    reasoning: "No gate verdicts or scoring dimensions present.".to_string(),
                };
            }
        };

        GradeResult::from_pass_rate(pass_rate, gate_count, dimension_count)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trace(attempt_id: u8, verdicts: Vec<bool>, dims: Vec<f64>) -> AttemptTrace {
        AttemptTrace {
            attempt_id: [attempt_id; 32],
            episode_id: None,
            gate_verdicts: verdicts,
            scoring_dimensions: dims.into_iter().enumerate().map(|(i, v)| (format!("dim_{i}"), v)).collect(),
            hdc_fingerprint: None,
        }
    }

    // ── TraceCollector ────────────────────────────────────────────────────

    #[test]
    fn collect_and_read_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let collector = TraceCollector::from_workspace(dir.path());
        let arena_id = [1_u8; 32];
        let trace = sample_trace(42, vec![true, false, true], vec![0.8, 0.6]);
        collector
            .collect_trace(&arena_id, &trace.attempt_id, &trace)
            .unwrap();
        let traces = collector.read_traces(&arena_id).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].attempt_id, trace.attempt_id);
        assert_eq!(traces[0].gate_verdicts, trace.gate_verdicts);
    }

    #[test]
    fn read_empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let collector = TraceCollector::from_workspace(dir.path());
        let arena_id = [99_u8; 32];
        let traces = collector.read_traces(&arena_id).unwrap();
        assert!(traces.is_empty());
    }

    #[test]
    fn trace_count_matches_writes() {
        let dir = tempfile::tempdir().unwrap();
        let collector = TraceCollector::from_workspace(dir.path());
        let arena_id = [2_u8; 32];
        assert_eq!(collector.trace_count(&arena_id), 0);
        for i in 0_u8..5 {
            let trace = sample_trace(i, vec![true], vec![]);
            collector
                .collect_trace(&arena_id, &trace.attempt_id, &trace)
                .unwrap();
        }
        assert_eq!(collector.trace_count(&arena_id), 5);
    }

    #[test]
    fn overwrite_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let collector = TraceCollector::from_workspace(dir.path());
        let arena_id = [3_u8; 32];
        let trace = sample_trace(7, vec![true], vec![1.0]);
        collector
            .collect_trace(&arena_id, &trace.attempt_id, &trace)
            .unwrap();
        collector
            .collect_trace(&arena_id, &trace.attempt_id, &trace)
            .unwrap();
        assert_eq!(collector.trace_count(&arena_id), 1);
    }

    // ── AutoGrader ────────────────────────────────────────────────────────

    #[test]
    fn all_pass_verdicts_scores_a() {
        let grader = AutoGrader::new();
        let trace = sample_trace(1, vec![true, true, true, true], vec![]);
        let result = grader.grade(&trace);
        assert_eq!(result.grade_label, "A");
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn all_fail_verdicts_scores_f() {
        let grader = AutoGrader::new();
        let trace = sample_trace(2, vec![false, false, false], vec![]);
        let result = grader.grade(&trace);
        assert_eq!(result.grade_label, "F");
        assert!(result.score < 0.30);
    }

    #[test]
    fn empty_trace_scores_f() {
        let grader = AutoGrader::new();
        let trace = sample_trace(3, vec![], vec![]);
        let result = grader.grade(&trace);
        assert_eq!(result.grade_label, "F");
        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn blended_gate_and_dimension_scores() {
        let grader = AutoGrader::new();
        // 2 of 4 gates pass (0.50) + dimensions mean 1.0 → blended = 0.75 → B
        let trace = sample_trace(4, vec![true, false, true, false], vec![1.0]);
        let result = grader.grade(&trace);
        assert_eq!(result.grade_label, "B");
    }

    #[test]
    fn boundary_d_grade() {
        let grader = AutoGrader::new();
        // 1 of 3 gates pass ≈ 0.333 → D
        let trace = sample_trace(5, vec![true, false, false], vec![]);
        let result = grader.grade(&trace);
        assert_eq!(result.grade_label, "D");
    }

    #[test]
    fn c_grade_range() {
        let grader = AutoGrader::new();
        // 3 of 5 gates pass = 0.60 → C
        let trace = sample_trace(6, vec![true, true, true, false, false], vec![]);
        let result = grader.grade(&trace);
        assert_eq!(result.grade_label, "C");
    }
}
