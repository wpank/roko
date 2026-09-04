//! Report adapter: map workflow controller state to [`WorkflowRunReport`].
//!
//! This module bridges the [`WorkflowGraphController`] terminal state into
//! the existing [`WorkflowRunReport`] structure from `roko-runtime` without
//! changing the report's text or JSON output format.

use std::time::Instant;

use roko_core::runtime_event::RuntimeEventEnvelope;
use roko_runtime::workflow_engine::{GateOutcome, WorkflowRunReport};

use super::controller::{WorkflowGraphController, WorkflowTermination};

// ---------------------------------------------------------------------------
// Report builder
// ---------------------------------------------------------------------------

/// Build a [`WorkflowRunReport`] from a terminated controller and execution
/// metadata.
///
/// The caller provides the start instant, gate outcomes, and any events
/// collected during execution so the report can be constructed without
/// the controller needing direct access to timing or event infrastructure.
#[must_use]
pub fn build_report(
    controller: &WorkflowGraphController,
    started_at: Instant,
    model: String,
    provider: Option<String>,
    agent_output: String,
    agent_turns: u32,
    token_usage: u64,
    cost: Option<f64>,
    gates: Vec<GateOutcome>,
    events: Vec<RuntimeEventEnvelope>,
    checkpoint_path: Option<String>,
) -> WorkflowRunReport {
    let duration = started_at.elapsed();
    let prompt_summary = truncate_prompt(&controller.workflow_id, 80);

    let (success, output) = match &controller.termination {
        Some(WorkflowTermination::Success { commit_hash }) => {
            let out = if let Some(hash) = commit_hash {
                format!("Committed: {hash}")
            } else {
                "Completed (no commit)".to_string()
            };
            (true, out)
        }
        Some(WorkflowTermination::NoChanges) => (true, "No changes produced".to_string()),
        Some(WorkflowTermination::GateExhausted { last_failure }) => {
            (false, format!("Gate exhausted: {last_failure}"))
        }
        Some(WorkflowTermination::ReviewExhausted { findings }) => (
            false,
            format!("Review exhausted ({} findings)", findings.len()),
        ),
        Some(WorkflowTermination::Cancelled) => (false, "Cancelled".to_string()),
        Some(WorkflowTermination::Failed { reason }) => (false, format!("Failed: {reason}")),
        Some(WorkflowTermination::Skipped { reason }) => (false, format!("Skipped: {reason}")),
        None => (false, agent_output.clone()),
    };

    WorkflowRunReport {
        run_id: controller.run_id.clone(),
        success,
        model,
        provider,
        prompt_summary,
        output,
        agent_turns,
        token_usage,
        cost,
        duration_secs: duration.as_secs_f64(),
        gates,
        events,
        checkpoint_path,
    }
}

/// Truncate a string to `max_len` characters, appending "..." if needed.
fn truncate_prompt(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut result = s[..max_len].to_string();
        result.push_str("...");
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::templates::WorkflowTemplateDescriptor;

    fn make_terminated_controller(termination: WorkflowTermination) -> WorkflowGraphController {
        let mut ctrl = WorkflowGraphController::new(
            "run-001".to_string(),
            WorkflowTemplateDescriptor::mechanical(),
            "wf-001".to_string(),
        );
        ctrl.termination = Some(termination);
        ctrl
    }

    #[test]
    fn success_report() {
        let ctrl = make_terminated_controller(WorkflowTermination::Success {
            commit_hash: Some("abc123".to_string()),
        });
        let report = build_report(
            &ctrl,
            Instant::now(),
            "claude-opus-4-20250514".to_string(),
            Some("anthropic".to_string()),
            "done".to_string(),
            3,
            1500,
            Some(0.05),
            vec![],
            vec![],
            None,
        );
        assert!(report.success);
        assert!(report.output.contains("abc123"));
        assert_eq!(report.run_id, "run-001");
        assert_eq!(report.agent_turns, 3);
    }

    #[test]
    fn gate_exhausted_report() {
        let ctrl = make_terminated_controller(WorkflowTermination::GateExhausted {
            last_failure: "compile error".to_string(),
        });
        let report = build_report(
            &ctrl,
            Instant::now(),
            "model".to_string(),
            None,
            "".to_string(),
            1,
            500,
            None,
            vec![GateOutcome {
                name: "compile".to_string(),
                passed: false,
                output: Some("error[E0308]".to_string()),
                duration_ms: 100,
            }],
            vec![],
            None,
        );
        assert!(!report.success);
        assert!(report.output.contains("Gate exhausted"));
        assert_eq!(report.gates.len(), 1);
    }

    #[test]
    fn cancelled_report() {
        let ctrl = make_terminated_controller(WorkflowTermination::Cancelled);
        let report = build_report(
            &ctrl,
            Instant::now(),
            "model".to_string(),
            None,
            "".to_string(),
            0,
            0,
            None,
            vec![],
            vec![],
            None,
        );
        assert!(!report.success);
        assert_eq!(report.output, "Cancelled");
    }

    #[test]
    fn no_changes_report() {
        let ctrl = make_terminated_controller(WorkflowTermination::NoChanges);
        let report = build_report(
            &ctrl,
            Instant::now(),
            "model".to_string(),
            None,
            "".to_string(),
            1,
            200,
            None,
            vec![],
            vec![],
            None,
        );
        assert!(report.success);
        assert!(report.output.contains("No changes"));
    }

    #[test]
    fn prompt_summary_truncation() {
        let long_prompt = "a".repeat(200);
        let truncated = truncate_prompt(&long_prompt, 80);
        assert_eq!(truncated.len(), 83); // 80 + "..."
        assert!(truncated.ends_with("..."));
    }
}
