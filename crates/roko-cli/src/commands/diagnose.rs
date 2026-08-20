//! `roko diagnose <plan-id>` — structured JSON diagnostic report for plan failures.

use std::path::Path;
use std::process::Command as ProcessCommand;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

use roko_runtime::{STATE_SNAPSHOT_RELATIVE_PATH, load_durable_runner_projection};

/// Run the diagnose command, printing a JSON report to stdout.
pub fn cmd_diagnose(workdir: &Path, plan_id: &str, verbose: bool) -> Result<i32> {
    let report = build_report(workdir, plan_id, verbose)?;
    let json = serde_json::to_string_pretty(&report).context("serializing diagnose report")?;
    println!("{json}");
    Ok(0)
}

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DiagnoseReport {
    pub plan_id: String,
    pub status: String,
    pub phase: Option<String>,
    pub iteration: Option<u32>,
    pub failed_task: Option<FailedTaskInfo>,
    pub gate_results: Vec<GateResultInfo>,
    pub run_state: Option<RunStateSummary>,
    pub git_state: Option<GitStateInfo>,
    pub suggested_recovery: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FailedTaskInfo {
    pub task_id: String,
    pub last_error: Option<String>,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GateResultInfo {
    pub gate_name: String,
    pub rung: u32,
    pub passed: bool,
    pub summary: String,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct RunStateSummary {
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub total_cost_usd: f64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_agent_calls: usize,
    pub completed_tasks: Vec<String>,
    pub failed_tasks: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GitStateInfo {
    pub current_branch: Option<String>,
    pub has_uncommitted_changes: bool,
    pub plan_branch_exists: bool,
}

// ---------------------------------------------------------------------------
// Report builder
// ---------------------------------------------------------------------------

fn build_report(workdir: &Path, plan_id: &str, _verbose: bool) -> Result<DiagnoseReport> {
    let projection = load_durable_runner_projection(workdir).context("reading state snapshot")?;

    let projection = match projection {
        Some(p) => p,
        None => {
            let snapshot_path = workdir.join(STATE_SNAPSHOT_RELATIVE_PATH);
            bail!(
                "No state snapshot found at {}. Run `roko plan run` first.",
                snapshot_path.display()
            );
        }
    };

    // ── Extract executor plan states ────────────────────────────────────
    let plan_states = projection
        .executor
        .get("plan_states")
        .and_then(Value::as_object);

    let plan_state = plan_states.and_then(|states| states.get(plan_id));

    if plan_state.is_none() {
        let available: Vec<String> = plan_states
            .map(|states| states.keys().cloned().collect())
            .unwrap_or_default();
        bail!(
            "Plan '{}' not found in state snapshot. Available plans: [{}]",
            plan_id,
            available.join(", ")
        );
    }
    // SAFETY: we just checked `plan_state.is_none()` above and bailed.
    let plan_state = plan_state.expect("checked above");

    // ── Phase / status ──────────────────────────────────────────────────
    let phase = plan_state
        .get("current_phase")
        .and_then(Value::as_str)
        .map(String::from);

    let iteration = plan_state
        .get("iteration")
        .and_then(Value::as_u64)
        .map(|v| v as u32);

    let last_error = plan_state
        .get("last_error")
        .and_then(Value::as_str)
        .map(String::from);

    let files_changed: Vec<String> = plan_state
        .get("files_changed")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let status = derive_status(phase.as_ref(), last_error.as_ref());

    // ── Gate results ────────────────────────────────────────────────────
    let gate_results: Vec<GateResultInfo> = plan_state
        .get("gate_results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(GateResultInfo {
                        gate_name: v.get("gate_name")?.as_str()?.to_string(),
                        rung: v.get("rung")?.as_u64()? as u32,
                        passed: v.get("passed")?.as_bool()?,
                        summary: v
                            .get("summary")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        duration_ms: v.get("duration_ms").and_then(Value::as_u64).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // ── Failed task info ────────────────────────────────────────────────
    let failed_task = if status == "failed" || last_error.is_some() {
        // Try to identify the failed task from run_state
        let failed_task_id = extract_failed_task_id(projection.run_state.as_ref(), plan_id);
        Some(FailedTaskInfo {
            task_id: failed_task_id.unwrap_or_else(|| "unknown".to_string()),
            last_error,
            files_changed,
        })
    } else {
        None
    };

    // ── Run state summary ───────────────────────────────────────────────
    let run_state = projection.run_state.as_ref().map(|rs| {
        let completed: Vec<String> = rs
            .get("completed_tasks")
            .and_then(Value::as_object)
            .and_then(|m| m.get(plan_id))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let failed: Vec<String> = rs
            .get("failed_tasks")
            .and_then(Value::as_object)
            .and_then(|m| m.get(plan_id))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        RunStateSummary {
            tasks_total: rs.get("tasks_total").and_then(Value::as_u64).unwrap_or(0) as usize,
            tasks_completed: rs
                .get("tasks_completed")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            tasks_failed: rs.get("tasks_failed").and_then(Value::as_u64).unwrap_or(0) as usize,
            total_cost_usd: rs
                .get("total_cost_usd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            total_tokens_in: rs
                .get("total_tokens_in")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_tokens_out: rs
                .get("total_tokens_out")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_agent_calls: rs
                .get("total_agent_calls")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            completed_tasks: completed,
            failed_tasks: failed,
        }
    });

    // ── Git state ───────────────────────────────────────────────────────
    let git_state = collect_git_state(workdir, plan_id);

    // ── Recovery suggestions ────────────────────────────────────────────
    let suggested_recovery =
        build_recovery_suggestions(&status, phase.as_ref(), &gate_results, git_state.as_ref());

    Ok(DiagnoseReport {
        plan_id: plan_id.to_string(),
        status,
        phase,
        iteration,
        failed_task,
        gate_results,
        run_state,
        git_state,
        suggested_recovery,
    })
}

fn derive_status(phase: Option<&String>, last_error: Option<&String>) -> String {
    if let Some(phase) = phase {
        match phase.as_str() {
            "done" | "merged" | "accepted" => "completed".to_string(),
            "failed" | "error" => "failed".to_string(),
            "gating" | "gate" | "verifying" => "gating".to_string(),
            "implementing" | "implement" | "agent" | "coding" => "running".to_string(),
            "queued" | "pending" | "ready" => "pending".to_string(),
            "paused" => "paused".to_string(),
            _ => {
                if last_error.is_some() {
                    "failed".to_string()
                } else {
                    phase.clone()
                }
            }
        }
    } else if last_error.is_some() {
        "failed".to_string()
    } else {
        "unknown".to_string()
    }
}

fn extract_failed_task_id(run_state: Option<&Value>, plan_id: &str) -> Option<String> {
    let rs = run_state?;
    let failed = rs
        .get("failed_tasks")?
        .as_object()?
        .get(plan_id)?
        .as_array()?;
    failed.last()?.as_str().map(String::from)
}

fn collect_git_state(workdir: &Path, plan_id: &str) -> Option<GitStateInfo> {
    let current_branch = run_git(workdir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let has_uncommitted = run_git(workdir, &["status", "--porcelain"])
        .map(|out| !out.trim().is_empty())
        .unwrap_or(false);

    // Check if a branch matching the plan id exists
    let plan_branch_exists = run_git(workdir, &["branch", "--list", &format!("*{plan_id}*")])
        .map(|out| !out.trim().is_empty())
        .unwrap_or(false);

    Some(GitStateInfo {
        current_branch,
        has_uncommitted_changes: has_uncommitted,
        plan_branch_exists,
    })
}

fn run_git(workdir: &Path, args: &[&str]) -> Option<String> {
    ProcessCommand::new("git")
        .args(args)
        .current_dir(workdir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn build_recovery_suggestions(
    status: &str,
    phase: Option<&String>,
    gate_results: &[GateResultInfo],
    git_state: Option<&GitStateInfo>,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    if status == "failed" {
        let failed_gates: Vec<&GateResultInfo> =
            gate_results.iter().filter(|g| !g.passed).collect();

        if failed_gates.iter().any(|g| g.gate_name.contains("compile")) {
            suggestions.push(
                "Compile errors detected. Run `cargo build --workspace` to see full errors."
                    .to_string(),
            );
        }
        if failed_gates.iter().any(|g| g.gate_name.contains("test")) {
            suggestions.push(
                "Test failures detected. Run `cargo test --workspace` to see full output."
                    .to_string(),
            );
        }
        if failed_gates.iter().any(|g| g.gate_name.contains("clippy")) {
            suggestions.push(
                "Clippy warnings detected. Run `cargo clippy --workspace --no-deps -- -D warnings`."
                    .to_string(),
            );
        }

        if failed_gates.is_empty() {
            suggestions.push(
                "No gate failures recorded. Check the last_error field for agent-level failures."
                    .to_string(),
            );
        }

        suggestions
            .push("To retry: `roko plan run plans/ --engine runner-v2 --resume-plan`".to_string());
    }

    if let Some(git) = git_state
        && git.has_uncommitted_changes
    {
        suggestions.push(
            "Uncommitted changes detected. Consider committing or stashing before retry."
                .to_string(),
        );
    }

    if let Some(phase) = phase
        && phase == "paused"
    {
        suggestions.push("Plan is paused. Resume with `roko plan run`.".to_string());
    }

    if suggestions.is_empty() && status == "completed" {
        suggestions.push("Plan completed successfully. No recovery needed.".to_string());
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_status_from_phases() {
        let done: String = "done".into();
        let failed: String = "failed".into();
        let implementing: String = "implementing".into();
        let queued: String = "queued".into();
        let paused: String = "paused".into();
        let err: String = "err".into();
        assert_eq!(derive_status(Some(&done), None), "completed");
        assert_eq!(derive_status(Some(&failed), None), "failed");
        assert_eq!(derive_status(Some(&implementing), None), "running");
        assert_eq!(derive_status(Some(&queued), None), "pending");
        assert_eq!(derive_status(Some(&paused), None), "paused");
        assert_eq!(derive_status(None, Some(&err)), "failed");
        assert_eq!(derive_status(None, None), "unknown");
    }

    #[test]
    fn derive_status_unknown_phase_with_error() {
        let custom: String = "custom".into();
        let oops: String = "oops".into();
        assert_eq!(derive_status(Some(&custom), Some(&oops)), "failed");
    }

    #[test]
    fn recovery_suggestions_compile_failure() {
        let gates = vec![GateResultInfo {
            gate_name: "compile:cargo".into(),
            rung: 1,
            passed: false,
            summary: "3 errors".into(),
            duration_ms: 5000,
        }];
        let suggestions = build_recovery_suggestions("failed", None, &gates, None);
        assert!(suggestions.iter().any(|s| s.contains("cargo build")));
        assert!(suggestions.iter().any(|s| s.contains("resume-plan")));
    }

    #[test]
    fn recovery_suggestions_completed() {
        let suggestions = build_recovery_suggestions("completed", None, &[], None);
        assert!(suggestions.iter().any(|s| s.contains("successfully")));
    }

    #[test]
    fn recovery_suggestions_uncommitted_changes() {
        let git = GitStateInfo {
            current_branch: Some("main".into()),
            has_uncommitted_changes: true,
            plan_branch_exists: false,
        };
        let suggestions = build_recovery_suggestions("failed", None, &[], Some(&git));
        assert!(suggestions.iter().any(|s| s.contains("Uncommitted")));
    }

    #[test]
    fn no_snapshot_gives_helpful_error() {
        let tmp = tempfile::tempdir().unwrap();
        let result = build_report(tmp.path(), "nonexistent", false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No state snapshot found"));
    }
}
