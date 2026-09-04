//! Visual regression assessment pipeline for the TUI.
//!
//! Orchestrates the full cycle of baseline capture, candidate capture, diff
//! comparison, and pass/fail reporting. This module ties together the
//! [`snapshot`](super::snapshot), [`screenshot_diff`](super::screenshot_diff),
//! and [`png_renderer`](super::png_renderer) modules into a single workflow
//! entry point.
//!
//! # Workflow
//!
//! 1. **Baseline capture**: Render the TUI in a known state and save the
//!    output (text or PNG).
//! 2. **Mutation**: Apply a code or state change.
//! 3. **Candidate capture**: Render the TUI again in the new state.
//! 4. **Diff**: Compare baseline and candidate snapshots.
//! 5. **Report**: Evaluate the diff against configurable thresholds and
//!    produce a structured pass/fail verdict.
//!
//! # Feature gate
//!
//! All public types and functions in this module are gated behind
//! `#[cfg(feature = "tui-png")]`.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::Serialize;

use super::screenshot_diff::{ScreenshotDiff, compare, compare_text};
use super::snapshot::{SnapshotConfig, SnapshotResult, capture_snapshots};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Thresholds for pass/fail evaluation of a visual diff.
#[derive(Debug, Clone)]
pub struct AssessmentThresholds {
    /// Maximum allowed diff percentage (0.0-100.0). Any diff above this value
    /// is a failure.
    pub max_diff_percentage: f64,
    /// Maximum number of differing cells before the assessment fails,
    /// regardless of percentage.
    pub max_diff_count: usize,
    /// Maximum number of distinct diff regions before the assessment fails.
    pub max_region_count: usize,
}

impl Default for AssessmentThresholds {
    fn default() -> Self {
        Self {
            max_diff_percentage: 0.0,
            max_diff_count: 0,
            max_region_count: 0,
        }
    }
}

impl AssessmentThresholds {
    /// A strict threshold that requires pixel-perfect identity.
    #[must_use]
    pub fn exact() -> Self {
        Self::default()
    }

    /// A lenient threshold that allows up to the given percentage of diff.
    #[must_use]
    pub fn percentage(max_pct: f64) -> Self {
        Self {
            max_diff_percentage: max_pct,
            max_diff_count: usize::MAX,
            max_region_count: usize::MAX,
        }
    }

    /// Allow up to `n` differing cells.
    #[must_use]
    pub fn max_cells(n: usize) -> Self {
        Self {
            max_diff_percentage: 100.0,
            max_diff_count: n,
            max_region_count: usize::MAX,
        }
    }
}

// ---------------------------------------------------------------------------
// Assessment result
// ---------------------------------------------------------------------------

/// Outcome of a visual regression assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The candidate matches the baseline within the configured thresholds.
    Pass,
    /// The candidate differs from the baseline beyond the configured thresholds.
    Fail,
}

/// Reason a [`Verdict::Fail`] was issued.
#[derive(Debug, Clone, PartialEq)]
pub enum FailureReason {
    /// Diff percentage exceeded the threshold.
    DiffPercentage { actual: f64, max: f64 },
    /// Absolute diff count exceeded the threshold.
    DiffCount { actual: usize, max: usize },
    /// Number of diff regions exceeded the threshold.
    RegionCount { actual: usize, max: usize },
}

/// Full assessment report combining diff data and verdict.
#[derive(Debug, Clone)]
pub struct VisualAssessment {
    /// Pass or fail verdict.
    pub verdict: Verdict,
    /// The underlying diff between baseline and candidate.
    pub diff: ScreenshotDiff,
    /// Thresholds used for evaluation.
    pub thresholds: AssessmentThresholds,
    /// Reasons for failure (empty when verdict is `Pass`).
    pub failure_reasons: Vec<FailureReason>,
    /// Path to the baseline snapshot directory or file.
    pub baseline_path: PathBuf,
    /// Path to the candidate snapshot directory or file.
    pub candidate_path: PathBuf,
}

impl VisualAssessment {
    /// Returns `true` if the assessment passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.verdict == Verdict::Pass
    }
}

// ---------------------------------------------------------------------------
// Assessment engine
// ---------------------------------------------------------------------------

/// Evaluate a pre-computed diff against thresholds.
pub fn assess(
    diff: ScreenshotDiff,
    thresholds: &AssessmentThresholds,
    baseline_path: impl Into<PathBuf>,
    candidate_path: impl Into<PathBuf>,
) -> VisualAssessment {
    let mut reasons = Vec::new();

    if diff.diff_percentage > thresholds.max_diff_percentage {
        reasons.push(FailureReason::DiffPercentage {
            actual: diff.diff_percentage,
            max: thresholds.max_diff_percentage,
        });
    }
    if diff.diff_count > thresholds.max_diff_count {
        reasons.push(FailureReason::DiffCount {
            actual: diff.diff_count,
            max: thresholds.max_diff_count,
        });
    }
    if diff.regions.len() > thresholds.max_region_count {
        reasons.push(FailureReason::RegionCount {
            actual: diff.regions.len(),
            max: thresholds.max_region_count,
        });
    }

    let verdict = if reasons.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Fail
    };

    VisualAssessment {
        verdict,
        diff,
        thresholds: thresholds.clone(),
        failure_reasons: reasons,
        baseline_path: baseline_path.into(),
        candidate_path: candidate_path.into(),
    }
}

/// Compare two snapshot files and assess the diff against thresholds.
///
/// This is the main convenience entry point. It reads both files, runs the
/// appropriate comparison backend, and evaluates the result.
pub fn compare_and_assess(
    baseline: &Path,
    candidate: &Path,
    thresholds: &AssessmentThresholds,
) -> Result<VisualAssessment> {
    let diff = compare(baseline, candidate)?;
    Ok(assess(diff, thresholds, baseline, candidate))
}

// ---------------------------------------------------------------------------
// Full-pipeline helpers
// ---------------------------------------------------------------------------

/// Configuration for a full capture-compare-assess pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Working directory (repository root).
    pub workdir: PathBuf,
    /// Terminal width for snapshot captures.
    pub width: u16,
    /// Terminal height for snapshot captures.
    pub height: u16,
    /// Optional tab filter (lowercase labels or fkey identifiers).
    pub tabs: Option<Vec<String>>,
    /// Thresholds for pass/fail evaluation.
    pub thresholds: AssessmentThresholds,
}

/// Result of a full pipeline run across multiple tabs.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineReport {
    /// Per-tab verdicts.
    pub tab_results: Vec<TabResult>,
    /// Overall verdict: Pass only if all tabs pass.
    pub overall: String,
    /// Total diff cells across all tabs.
    pub total_diff_count: usize,
}

/// Per-tab result within a pipeline report.
#[derive(Debug, Clone, Serialize)]
pub struct TabResult {
    /// Tab label (e.g., "dashboard").
    pub tab: String,
    /// "pass" or "fail".
    pub verdict: String,
    /// Number of differing cells.
    pub diff_count: usize,
    /// Diff percentage.
    pub diff_percentage: f64,
    /// Number of diff regions.
    pub region_count: usize,
    /// Baseline file path.
    pub baseline_file: String,
    /// Candidate file path.
    pub candidate_file: String,
}

/// Run the full capture-compare pipeline.
///
/// 1. Capture baseline snapshots to `baseline_dir`.
/// 2. Capture candidate snapshots to `candidate_dir` (the caller is
///    responsible for applying mutations between steps 1 and 2; this function
///    captures both in sequence for testing purposes).
/// 3. Compare each tab's baseline and candidate text files.
/// 4. Assess each diff against the configured thresholds.
///
/// In a real regression test, the caller would capture baseline separately
/// (perhaps from a known-good commit) and only capture the candidate here.
pub fn run_pipeline(
    config: &PipelineConfig,
    baseline_dir: &Path,
    candidate_dir: &Path,
) -> Result<PipelineReport> {
    // Capture baseline.
    let baseline_config = SnapshotConfig {
        width: config.width,
        height: config.height,
        output_dir: baseline_dir.to_path_buf(),
        tabs: config.tabs.clone(),
        label: Some("baseline".to_string()),
    };
    let baseline_result = capture_snapshots(&config.workdir, &baseline_config)
        .context("capture baseline snapshots")?;

    // Capture candidate (same state for now -- in practice, a mutation
    // happens between these two captures).
    let candidate_config = SnapshotConfig {
        width: config.width,
        height: config.height,
        output_dir: candidate_dir.to_path_buf(),
        tabs: config.tabs.clone(),
        label: Some("candidate".to_string()),
    };
    let candidate_result = capture_snapshots(&config.workdir, &candidate_config)
        .context("capture candidate snapshots")?;

    // Compare each tab file.
    compare_snapshot_results(
        &baseline_result,
        &candidate_result,
        baseline_dir,
        candidate_dir,
        &config.thresholds,
    )
}

/// Compare the files from two snapshot results and produce a pipeline report.
pub fn compare_snapshot_results(
    baseline: &SnapshotResult,
    candidate: &SnapshotResult,
    baseline_dir: &Path,
    candidate_dir: &Path,
    thresholds: &AssessmentThresholds,
) -> Result<PipelineReport> {
    let mut tab_results = Vec::new();
    let mut total_diff_count = 0usize;
    let mut all_pass = true;

    // Read both manifests to discover matching tab files.
    let baseline_manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&baseline.manifest_path).context("read baseline manifest")?,
    )
    .context("parse baseline manifest")?;
    let candidate_manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&candidate.manifest_path).context("read candidate manifest")?,
    )
    .context("parse candidate manifest")?;

    let baseline_tabs = baseline_manifest["tabs"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let candidate_tabs = candidate_manifest["tabs"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    for b_entry in &baseline_tabs {
        let tab_name = b_entry["tab"].as_str().unwrap_or("unknown");
        let b_file = b_entry["file"].as_str().unwrap_or("");

        // Find matching candidate file.
        let c_file = candidate_tabs
            .iter()
            .find(|c| c["tab"].as_str() == Some(tab_name))
            .and_then(|c| c["file"].as_str());

        let Some(c_file) = c_file else {
            tab_results.push(TabResult {
                tab: tab_name.to_string(),
                verdict: "fail".to_string(),
                diff_count: 0,
                diff_percentage: 100.0,
                region_count: 0,
                baseline_file: b_file.to_string(),
                candidate_file: String::new(),
            });
            all_pass = false;
            continue;
        };

        let b_path = baseline_dir.join(b_file);
        let c_path = candidate_dir.join(c_file);

        if !b_path.exists() || !c_path.exists() {
            tab_results.push(TabResult {
                tab: tab_name.to_string(),
                verdict: "fail".to_string(),
                diff_count: 0,
                diff_percentage: 100.0,
                region_count: 0,
                baseline_file: b_file.to_string(),
                candidate_file: c_file.to_string(),
            });
            all_pass = false;
            continue;
        }

        let b_text = std::fs::read_to_string(&b_path)
            .with_context(|| format!("read baseline tab: {}", b_path.display()))?;
        let c_text = std::fs::read_to_string(&c_path)
            .with_context(|| format!("read candidate tab: {}", c_path.display()))?;

        let diff = compare_text(&b_text, &c_text);
        let assessment = assess(diff, thresholds, &b_path, &c_path);

        total_diff_count += assessment.diff.diff_count;
        if !assessment.passed() {
            all_pass = false;
        }

        tab_results.push(TabResult {
            tab: tab_name.to_string(),
            verdict: if assessment.passed() {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            diff_count: assessment.diff.diff_count,
            diff_percentage: assessment.diff.diff_percentage,
            region_count: assessment.diff.regions.len(),
            baseline_file: b_file.to_string(),
            candidate_file: c_file.to_string(),
        });
    }

    Ok(PipelineReport {
        tab_results,
        overall: if all_pass {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        total_diff_count,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::screenshot_diff::DiffMode;

    fn make_diff(diff_count: usize, total: usize, regions: usize) -> ScreenshotDiff {
        ScreenshotDiff {
            mode: DiffMode::Text,
            diff_count,
            total_cells: total,
            diff_percentage: if total == 0 {
                0.0
            } else {
                (diff_count as f64) / (total as f64) * 100.0
            },
            regions: (0..regions)
                .map(|i| super::super::screenshot_diff::DiffRegion {
                    x: i,
                    y: 0,
                    width: 1,
                    height: 1,
                    diff_count: 1,
                })
                .collect(),
            dimensions: (10, 10),
        }
    }

    #[test]
    fn exact_threshold_passes_on_identical() {
        let diff = make_diff(0, 100, 0);
        let result = assess(diff, &AssessmentThresholds::exact(), "a", "b");
        assert!(result.passed());
        assert_eq!(result.verdict, Verdict::Pass);
        assert!(result.failure_reasons.is_empty());
    }

    #[test]
    fn exact_threshold_fails_on_single_diff() {
        let diff = make_diff(1, 100, 1);
        let result = assess(diff, &AssessmentThresholds::exact(), "a", "b");
        assert!(!result.passed());
        assert_eq!(result.verdict, Verdict::Fail);
        assert!(!result.failure_reasons.is_empty());
    }

    #[test]
    fn percentage_threshold_passes_within_budget() {
        let diff = make_diff(5, 100, 2);
        let thresholds = AssessmentThresholds::percentage(6.0);
        let result = assess(diff, &thresholds, "a", "b");
        assert!(result.passed());
    }

    #[test]
    fn percentage_threshold_fails_over_budget() {
        let diff = make_diff(10, 100, 3);
        let thresholds = AssessmentThresholds::percentage(5.0);
        let result = assess(diff, &thresholds, "a", "b");
        assert!(!result.passed());
        assert!(
            result
                .failure_reasons
                .iter()
                .any(|r| matches!(r, FailureReason::DiffPercentage { .. }))
        );
    }

    #[test]
    fn max_cells_threshold() {
        let diff = make_diff(5, 100, 1);
        let pass = assess(diff.clone(), &AssessmentThresholds::max_cells(5), "a", "b");
        assert!(pass.passed());

        let fail = assess(diff, &AssessmentThresholds::max_cells(4), "a", "b");
        assert!(!fail.passed());
        assert!(
            fail.failure_reasons
                .iter()
                .any(|r| matches!(r, FailureReason::DiffCount { .. }))
        );
    }

    #[test]
    fn region_count_threshold() {
        let diff = make_diff(3, 100, 3);
        let thresholds = AssessmentThresholds {
            max_diff_percentage: 100.0,
            max_diff_count: usize::MAX,
            max_region_count: 2,
        };
        let result = assess(diff, &thresholds, "a", "b");
        assert!(!result.passed());
        assert!(
            result
                .failure_reasons
                .iter()
                .any(|r| matches!(r, FailureReason::RegionCount { .. }))
        );
    }

    #[test]
    fn compare_and_assess_with_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "same content").unwrap();
        std::fs::write(&b, "same content").unwrap();
        let result = compare_and_assess(&a, &b, &AssessmentThresholds::exact()).unwrap();
        assert!(result.passed());
    }

    #[test]
    fn pipeline_report_serializes_to_json() {
        let report = PipelineReport {
            tab_results: vec![TabResult {
                tab: "dashboard".to_string(),
                verdict: "pass".to_string(),
                diff_count: 0,
                diff_percentage: 0.0,
                region_count: 0,
                baseline_file: "f01-dashboard.txt".to_string(),
                candidate_file: "f01-dashboard.txt".to_string(),
            }],
            overall: "pass".to_string(),
            total_diff_count: 0,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\"overall\": \"pass\""));
        assert!(json.contains("\"dashboard\""));
    }
}
