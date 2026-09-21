//! Graph caller adapter for workflow execution (#258).
//!
//! After #276 retired `WorkflowEngine`, all workflow execution goes through
//! the graph-based `WorkflowGraphController` from `roko-execution` (#257).
//! This module provides:
//!
//! - The canary comparison function for running workflows and comparing
//!   results across template configurations.
//! - The frozen legacy facade marker.
//! - Tests for the graph workflow path.

use roko_core::foundation::ShellGateCommand as CoreShellGateCommand;
use roko_runtime::workflow_contract::WorkflowRunReport;

use crate::run::CliOverrides;
use crate::state_hub::StateHub;

// ---------------------------------------------------------------------------
// Canary comparison
// ---------------------------------------------------------------------------

/// Report from running a workflow with two different template configurations
/// for comparison.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CanaryComparisonReport {
    /// The primary (authoritative) report.
    pub primary_report: Option<WorkflowRunReport>,
    /// The comparison report.
    pub comparison_report: Option<WorkflowRunReport>,
    /// Differences detected between the two runs.
    pub differences: Vec<String>,
    /// Which run is authoritative.
    pub authoritative: CanaryAuthoritative,
}

/// Which run result is authoritative in canary mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanaryAuthoritative {
    /// The primary run is authoritative.
    Primary,
    /// The comparison run is authoritative.
    Comparison,
}

impl CanaryComparisonReport {
    /// Return the authoritative report, if available.
    #[must_use]
    pub fn authoritative_report(&self) -> Option<&WorkflowRunReport> {
        match self.authoritative {
            CanaryAuthoritative::Primary => self.primary_report.as_ref(),
            CanaryAuthoritative::Comparison => self.comparison_report.as_ref(),
        }
    }
}

/// Run the same prompt through two different templates and compare results.
///
/// Returns a [`CanaryComparisonReport`] with the primary result as
/// authoritative. The comparison result is for observability only.
pub async fn run_canary_comparison(
    prompt: &str,
    workdir: &std::path::Path,
    primary_template: &str,
    comparison_template: &str,
    enabled_gates: Vec<String>,
    shell_gates: Vec<CoreShellGateCommand>,
    external_hub: Option<&StateHub>,
    overrides: &CliOverrides,
) -> anyhow::Result<CanaryComparisonReport> {
    let primary_result = crate::run::run_workflow_report(
        prompt,
        workdir,
        primary_template,
        enabled_gates.clone(),
        shell_gates.clone(),
        external_hub,
        overrides,
    )
    .await;

    let comparison_result = crate::run::run_workflow_report(
        prompt,
        workdir,
        comparison_template,
        enabled_gates,
        shell_gates,
        external_hub,
        overrides,
    )
    .await;

    let mut differences = Vec::new();

    match (&primary_result, &comparison_result) {
        (Ok(primary), Ok(comparison)) => {
            if primary.success != comparison.success {
                differences.push(format!(
                    "success mismatch: {}={}, {}={}",
                    primary_template, primary.success, comparison_template, comparison.success
                ));
            }

            let primary_gate_count = primary.gates.len();
            let comparison_gate_count = comparison.gates.len();
            if primary_gate_count != comparison_gate_count {
                differences.push(format!(
                    "gate count mismatch: {primary_template}={primary_gate_count}, \
                     {comparison_template}={comparison_gate_count}"
                ));
            }

            for (pg, cg) in primary.gates.iter().zip(comparison.gates.iter()) {
                if pg.name != cg.name || pg.passed != cg.passed {
                    differences.push(format!(
                        "gate '{}' mismatch: {primary_template}={}({}), {comparison_template}={}({})",
                        pg.name,
                        pg.name,
                        if pg.passed { "PASS" } else { "FAIL" },
                        cg.name,
                        if cg.passed { "PASS" } else { "FAIL" },
                    ));
                }
            }
        }
        (Ok(_), Err(e)) => {
            differences.push(format!("{comparison_template} failed: {e}"));
        }
        (Err(_), Ok(_)) => {
            differences.push(format!(
                "{primary_template} failed but {comparison_template} succeeded"
            ));
        }
        (Err(pe), Err(ce)) => {
            differences.push(format!(
                "both templates failed: {primary_template}={pe}, {comparison_template}={ce}"
            ));
        }
    }

    if differences.is_empty() {
        tracing::info!(
            primary = primary_template,
            comparison = comparison_template,
            "canary comparison: no differences detected"
        );
    } else {
        for diff in &differences {
            tracing::warn!(
                primary = primary_template,
                comparison = comparison_template,
                diff,
                "canary comparison: difference detected"
            );
        }
    }

    Ok(CanaryComparisonReport {
        primary_report: primary_result.ok(),
        comparison_report: comparison_result.ok(),
        differences,
        authoritative: CanaryAuthoritative::Primary,
    })
}

// ---------------------------------------------------------------------------
// Frozen legacy facade marker (#258)
// ---------------------------------------------------------------------------

/// Marker documenting that the legacy WorkflowEngine execution path has been
/// retired by #276. All workflow execution now uses graph templates via
/// `WorkflowGraphController`.
pub const LEGACY_WORKFLOW_ENGINE_FROZEN: &str =
    "frozen by #258, retired by #276; all execution uses graph templates";

#[cfg(test)]
mod tests {
    use super::*;

    // ── Engine flag resolution ───────────────────────────────────────

    #[test]
    fn resolve_engine_flag_none_is_graph() {
        let result = crate::run::resolve_engine_flag(None);
        assert_eq!(result, "graph");
    }

    #[test]
    fn resolve_engine_flag_graph_is_graph() {
        let result = crate::run::resolve_engine_flag(Some("graph"));
        assert_eq!(result, "graph");
    }

    #[test]
    fn resolve_engine_flag_graph_canary_is_graph() {
        let result = crate::run::resolve_engine_flag(Some("graph_canary"));
        assert_eq!(result, "graph");
    }

    #[test]
    fn resolve_engine_flag_unknown_falls_back_to_graph() {
        let result = crate::run::resolve_engine_flag(Some("bogus"));
        assert_eq!(result, "graph");
    }

    // ── Canary comparison types ──────────────────────────────────────

    #[test]
    fn canary_authoritative_primary() {
        let report = CanaryComparisonReport {
            primary_report: Some(WorkflowRunReport {
                run_id: "test".to_string(),
                success: true,
                model: "model".to_string(),
                provider: None,
                prompt_summary: "test".to_string(),
                output: "done".to_string(),
                agent_turns: 1,
                token_usage: 100,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cost: None,
                duration_secs: 1.0,
                gates: vec![],
                events: vec![],
                checkpoint_path: None,
            }),
            comparison_report: None,
            differences: vec![],
            authoritative: CanaryAuthoritative::Primary,
        };
        assert!(report.authoritative_report().is_some());
        assert!(report.authoritative_report().unwrap().success);
    }

    #[test]
    fn canary_differences_reported() {
        let report = CanaryComparisonReport {
            primary_report: None,
            comparison_report: None,
            differences: vec!["test difference".to_string()],
            authoritative: CanaryAuthoritative::Primary,
        };
        assert_eq!(report.differences.len(), 1);
        assert!(report.authoritative_report().is_none());
    }

    // ── Frozen legacy marker ─────────────────────────────────────────

    #[test]
    fn frozen_marker_exists() {
        assert!(LEGACY_WORKFLOW_ENGINE_FROZEN.contains("frozen"));
        assert!(LEGACY_WORKFLOW_ENGINE_FROZEN.contains("#258"));
    }
}
