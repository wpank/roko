//! `roko diagnose <plan-id>` — structured JSON diagnostic report for plan failures.

use std::io::BufRead;
use std::path::Path;
use std::process::Command as ProcessCommand;

use anyhow::{Context, Result, bail};
use regex::Regex;
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
    /// Top-level plan cost derived from `.roko/learn/efficiency.jsonl`.
    /// `None` when no efficiency data exists for this plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct FailedTaskInfo {
    pub task_id: String,
    pub last_error: Option<String>,
    pub files_changed: Vec<String>,
    /// Episode IDs from `.roko/episodes.jsonl` linked to this failed task.
    /// At most 5, ordered most-recent-last.
    #[serde(default)]
    pub episode_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GateResultInfo {
    pub gate_name: String,
    pub rung: u32,
    pub passed: bool,
    pub summary: String,
    pub duration_ms: u64,
    /// Classified errors extracted from the gate summary / output.
    #[serde(default)]
    pub classified_errors: Vec<ClassifiedError>,
}

/// One classified error extracted from gate output via regex pattern matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClassifiedError {
    /// Category of the error.
    pub error_class: ErrorClass,
    /// Source file, if identifiable from the error output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Line number in the source file, if identifiable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// One-line human-readable error summary.
    pub error_summary: String,
    /// Suggested remediation action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Error classification categories for gate failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    /// Rust compiler error (e.g. `error[E0308]: mismatched types`).
    CompileError,
    /// Test failure (e.g. `test result: FAILED`).
    TestFailure,
    /// Clippy or other lint warning/error.
    LintWarning,
    /// Linker error (e.g. `ld: symbol(s) not found`).
    LinkError,
    /// Runtime panic (e.g. `thread 'main' panicked at`).
    RuntimePanic,
    /// Timeout or process killed.
    Timeout,
    /// Unable to classify the error.
    Unknown,
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
                    let gate_name = v.get("gate_name")?.as_str()?.to_string();
                    let passed = v.get("passed")?.as_bool()?;
                    let summary = v
                        .get("summary")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();

                    // Classify errors from gate output when the gate failed.
                    let output = v.get("output").and_then(Value::as_str).unwrap_or("");
                    let classified_errors = if !passed {
                        classify_gate_errors(&gate_name, &summary, output)
                    } else {
                        Vec::new()
                    };

                    Some(GateResultInfo {
                        gate_name,
                        rung: v.get("rung")?.as_u64()? as u32,
                        passed,
                        summary,
                        duration_ms: v.get("duration_ms").and_then(Value::as_u64).unwrap_or(0),
                        classified_errors,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // ── Failed task info ────────────────────────────────────────────────
    let failed_task = if status == "failed" || last_error.is_some() {
        // Try to identify the failed task from run_state
        let failed_task_id = extract_failed_task_id(projection.run_state.as_ref(), plan_id);
        let task_id_str = failed_task_id.unwrap_or_else(|| "unknown".to_string());

        // Look up episode IDs from .roko/episodes.jsonl for the failed task.
        let episode_ids = collect_episode_ids(workdir, &task_id_str);

        Some(FailedTaskInfo {
            task_id: task_id_str,
            last_error,
            files_changed,
            episode_ids,
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

    // ── Total cost from efficiency events ────────────────────────────────
    let total_cost_usd = collect_total_cost_usd(workdir, plan_id);

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
        total_cost_usd,
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

// ---------------------------------------------------------------------------
// Error classification
// ---------------------------------------------------------------------------

/// Classify errors from gate output using regex-based pattern matching.
///
/// Inspects the gate name, summary, and raw output to produce structured
/// `ClassifiedError` entries. When no raw output is available, falls back
/// to classifying the summary line itself.
fn classify_gate_errors(gate_name: &str, summary: &str, output: &str) -> Vec<ClassifiedError> {
    let mut errors = Vec::new();

    // If there is raw output, parse it line-by-line.
    if !output.is_empty() {
        errors.extend(classify_output_lines(output));
    }

    // If the raw output produced nothing, fall back to classifying the
    // summary + gate name combination.
    if errors.is_empty() && !summary.is_empty() {
        let class = infer_class_from_gate_name(gate_name, summary);
        errors.push(ClassifiedError {
            error_class: class,
            file: None,
            line: None,
            error_summary: truncate(summary, 200),
            suggestion: suggestion_for(class),
        });
    }

    errors
}

/// Walk raw gate output and extract classified errors from compiler/test
/// diagnostics.
fn classify_output_lines(output: &str) -> Vec<ClassifiedError> {
    // Regex: `error[E0308]: mismatched types` or `error: ...`
    let re_compile = Regex::new(r"^error(\[E\d+\])?: (.+)").expect("valid regex");
    // Regex: `warning: ...` (clippy / lint)
    let re_lint = Regex::new(r"^warning: (.+)").expect("valid regex");
    // Regex: `  --> path/to/file.rs:42:10`
    let re_location = Regex::new(r"^\s*--> (.+):(\d+):\d+").expect("valid regex");
    // Regex: `test foo::bar ... FAILED`
    let re_test = Regex::new(r"^test .+ \.\.\. FAILED").expect("valid regex");
    // Regex: `thread '...' panicked at '...'`
    let re_panic = Regex::new(r"thread '.+' panicked at").expect("valid regex");
    // Regex: linker error patterns
    let re_link = Regex::new(r"(?i)(linker|ld|undefined (reference|symbol)|symbol\(s\) not found)")
        .expect("valid regex");

    let mut errors: Vec<ClassifiedError> = Vec::new();

    // Track the most recent file/line from `-->` markers so that
    // subsequent error lines can inherit the location.
    let mut pending_file: Option<String> = None;
    let mut pending_line: Option<u32> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim();

        // Update pending location from `-->` lines.
        if let Some(caps) = re_location.captures(line) {
            pending_file = caps.get(1).map(|m| m.as_str().to_string());
            pending_line = caps.get(2).and_then(|m| m.as_str().parse().ok());
            continue;
        }

        if let Some(caps) = re_compile.captures(line) {
            let msg = caps.get(2).map_or("", |m| m.as_str());
            errors.push(ClassifiedError {
                error_class: ErrorClass::CompileError,
                file: pending_file.take(),
                line: pending_line.take(),
                error_summary: truncate(msg, 200),
                suggestion: suggestion_for(ErrorClass::CompileError),
            });
        } else if re_test.is_match(line) {
            errors.push(ClassifiedError {
                error_class: ErrorClass::TestFailure,
                file: None,
                line: None,
                error_summary: truncate(line, 200),
                suggestion: suggestion_for(ErrorClass::TestFailure),
            });
        } else if re_panic.is_match(line) {
            errors.push(ClassifiedError {
                error_class: ErrorClass::RuntimePanic,
                file: pending_file.take(),
                line: pending_line.take(),
                error_summary: truncate(line, 200),
                suggestion: suggestion_for(ErrorClass::RuntimePanic),
            });
        } else if re_link.is_match(line) {
            errors.push(ClassifiedError {
                error_class: ErrorClass::LinkError,
                file: None,
                line: None,
                error_summary: truncate(line, 200),
                suggestion: suggestion_for(ErrorClass::LinkError),
            });
        } else if let Some(caps) = re_lint.captures(line) {
            let msg = caps.get(1).map_or("", |m| m.as_str());
            errors.push(ClassifiedError {
                error_class: ErrorClass::LintWarning,
                file: pending_file.take(),
                line: pending_line.take(),
                error_summary: truncate(msg, 200),
                suggestion: suggestion_for(ErrorClass::LintWarning),
            });
        }
    }

    errors
}

/// Infer an error class from the gate name when no raw output is available.
fn infer_class_from_gate_name(gate_name: &str, summary: &str) -> ErrorClass {
    let combined = format!("{gate_name} {summary}").to_lowercase();
    if combined.contains("compile") || combined.contains("build") {
        ErrorClass::CompileError
    } else if combined.contains("test") {
        ErrorClass::TestFailure
    } else if combined.contains("clippy") || combined.contains("lint") {
        ErrorClass::LintWarning
    } else if combined.contains("link") {
        ErrorClass::LinkError
    } else if combined.contains("panic") {
        ErrorClass::RuntimePanic
    } else if combined.contains("timeout") || combined.contains("timed out") {
        ErrorClass::Timeout
    } else {
        ErrorClass::Unknown
    }
}

/// Return a human-friendly suggestion for the given error class.
fn suggestion_for(class: ErrorClass) -> Option<String> {
    Some(
        match class {
            ErrorClass::CompileError => {
                "Fix the type/syntax error and rerun `cargo build --workspace`."
            }
            ErrorClass::TestFailure => {
                "Inspect the failing test assertion and rerun `cargo test --workspace`."
            }
            ErrorClass::LintWarning => {
                "Address the clippy/lint issue and rerun `cargo clippy --workspace --no-deps -- -D warnings`."
            }
            ErrorClass::LinkError => {
                "Check for missing native libraries or duplicate symbol definitions."
            }
            ErrorClass::RuntimePanic => {
                "Examine the panic message and add appropriate error handling."
            }
            ErrorClass::Timeout => "Consider increasing timeout or optimizing the operation.",
            ErrorClass::Unknown => return None,
        }
        .to_string(),
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

// ---------------------------------------------------------------------------
// Episode ID collection
// ---------------------------------------------------------------------------

/// Read `.roko/episodes.jsonl` and return up to 5 episode IDs whose
/// `task_id` matches the given failed task. Returns most-recent-last.
fn collect_episode_ids(workdir: &Path, task_id: &str) -> Vec<String> {
    let path = workdir.join(".roko").join("episodes.jsonl");
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = std::io::BufReader::new(file);

    let mut ids: Vec<String> = Vec::new();
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        // Lightweight: parse only the fields we need.
        let val: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ep_task = val.get("task_id").and_then(Value::as_str).unwrap_or("");
        if ep_task == task_id {
            // Prefer `id`; fall back to deprecated `episode_id`.
            let ep_id = val
                .get("id")
                .or_else(|| val.get("episode_id"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if !ep_id.is_empty() {
                ids.push(ep_id);
            }
        }
    }

    // Keep only the last 5 (most recent).
    let start = ids.len().saturating_sub(5);
    ids[start..].to_vec()
}

// ---------------------------------------------------------------------------
// Cost collection from efficiency events
// ---------------------------------------------------------------------------

/// Sum `cost_usd` from `.roko/learn/efficiency.jsonl` for entries whose
/// `plan_id` matches. Returns `None` when no matching records exist or
/// the file is absent.
fn collect_total_cost_usd(workdir: &Path, plan_id: &str) -> Option<f64> {
    let path = workdir.join(".roko").join("learn").join("efficiency.jsonl");
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return None,
    };
    let reader = std::io::BufReader::new(file);

    let mut total: f64 = 0.0;
    let mut found = false;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let val: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ep_plan = val.get("plan_id").and_then(Value::as_str).unwrap_or("");
        if ep_plan == plan_id
            && let Some(cost) = val.get("cost_usd").and_then(Value::as_f64)
        {
            total += cost;
            found = true;
        }
    }

    if found { Some(total) } else { None }
}

// ---------------------------------------------------------------------------
// Recovery suggestions
// ---------------------------------------------------------------------------

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
            classified_errors: Vec::new(),
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

    // ── ClassifiedError tests ────────────────────────────────────────

    #[test]
    fn classify_compile_error_from_output() {
        let output = "\
  --> crates/roko-cli/src/main.rs:42:10
error[E0308]: mismatched types
   |
42 |     foo(bar)
   |         ^^^ expected `u32`, found `&str`";
        let errors = classify_gate_errors("compile:cargo", "", output);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].error_class, ErrorClass::CompileError);
        assert!(errors[0].error_summary.contains("mismatched types"));
        assert_eq!(
            errors[0].file.as_deref(),
            Some("crates/roko-cli/src/main.rs")
        );
        assert_eq!(errors[0].line, Some(42));
        assert!(errors[0].suggestion.is_some());
    }

    #[test]
    fn classify_test_failure_from_output() {
        let output = "test commands::diagnose::tests::my_test ... FAILED";
        let errors = classify_gate_errors("test:cargo", "", output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::TestFailure);
    }

    #[test]
    fn classify_panic_from_output() {
        let output = "thread 'main' panicked at 'index out of bounds'";
        let errors = classify_gate_errors("test:cargo", "", output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::RuntimePanic);
    }

    #[test]
    fn classify_lint_from_output() {
        let output = "\
  --> crates/roko-cli/src/foo.rs:10:9
warning: unused variable: `x`";
        let errors = classify_gate_errors("clippy:cargo", "", output);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].error_class, ErrorClass::LintWarning);
        assert_eq!(
            errors[0].file.as_deref(),
            Some("crates/roko-cli/src/foo.rs")
        );
        assert_eq!(errors[0].line, Some(10));
    }

    #[test]
    fn classify_link_error_from_output() {
        let output = "ld: symbol(s) not found for architecture arm64";
        let errors = classify_gate_errors("compile:cargo", "", output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::LinkError);
    }

    #[test]
    fn classify_fallback_from_summary_when_no_output() {
        let errors = classify_gate_errors("compile:cargo", "build failed with 3 errors", "");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::CompileError);
        assert!(errors[0].error_summary.contains("build failed"));
    }

    #[test]
    fn classify_unknown_gate() {
        let errors = classify_gate_errors("custom:gate", "something went wrong", "");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::Unknown);
        assert!(errors[0].suggestion.is_none());
    }

    #[test]
    fn classify_empty_summary_and_output_produces_nothing() {
        let errors = classify_gate_errors("test:cargo", "", "");
        assert!(errors.is_empty());
    }

    #[test]
    fn classify_passed_gate_produces_nothing() {
        // classify_output_lines returns nothing for benign output.
        let errors =
            classify_output_lines("Compiling roko-cli v0.1.0\nFinished dev [unoptimized]");
        assert!(errors.is_empty());
    }

    // ── Episode ID tests ─────────────────────────────────────────────

    #[test]
    fn collect_episode_ids_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert!(ids.is_empty());
    }

    #[test]
    fn collect_episode_ids_filters_by_task() {
        let tmp = tempfile::tempdir().unwrap();
        let roko = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko).unwrap();
        let content = [
            r#"{"id":"ep-1","task_id":"task-1","agent_id":"a1"}"#,
            r#"{"id":"ep-2","task_id":"task-2","agent_id":"a2"}"#,
            r#"{"id":"ep-3","task_id":"task-1","agent_id":"a1"}"#,
        ]
        .join("\n");
        std::fs::write(roko.join("episodes.jsonl"), content).unwrap();

        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert_eq!(ids, vec!["ep-1", "ep-3"]);
    }

    #[test]
    fn collect_episode_ids_caps_at_five() {
        let tmp = tempfile::tempdir().unwrap();
        let roko = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko).unwrap();
        let lines: Vec<String> = (0..8)
            .map(|i| format!(r#"{{"id":"ep-{i}","task_id":"task-1","agent_id":"a"}}"#))
            .collect();
        std::fs::write(roko.join("episodes.jsonl"), lines.join("\n")).unwrap();

        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert_eq!(ids.len(), 5);
        assert_eq!(ids, vec!["ep-3", "ep-4", "ep-5", "ep-6", "ep-7"]);
    }

    #[test]
    fn collect_episode_ids_falls_back_to_episode_id_field() {
        let tmp = tempfile::tempdir().unwrap();
        let roko = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko).unwrap();
        let content = r#"{"episode_id":"legacy-1","task_id":"task-1","agent_id":"a1"}"#;
        std::fs::write(roko.join("episodes.jsonl"), content).unwrap();

        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert_eq!(ids, vec!["legacy-1"]);
    }

    // ── Cost collection tests ────────────────────────────────────────

    #[test]
    fn collect_total_cost_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_none());
    }

    #[test]
    fn collect_total_cost_sums_matching_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let learn = tmp.path().join(".roko").join("learn");
        std::fs::create_dir_all(&learn).unwrap();
        let content = [
            r#"{"plan_id":"plan-1","cost_usd":0.05,"task_id":"t1"}"#,
            r#"{"plan_id":"plan-2","cost_usd":0.10,"task_id":"t2"}"#,
            r#"{"plan_id":"plan-1","cost_usd":0.03,"task_id":"t3"}"#,
        ]
        .join("\n");
        std::fs::write(learn.join("efficiency.jsonl"), content).unwrap();

        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_some());
        let total = cost.unwrap();
        assert!((total - 0.08).abs() < 1e-9);
    }

    #[test]
    fn collect_total_cost_none_when_no_matching_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let learn = tmp.path().join(".roko").join("learn");
        std::fs::create_dir_all(&learn).unwrap();
        let content = r#"{"plan_id":"other","cost_usd":0.10,"task_id":"t1"}"#;
        std::fs::write(learn.join("efficiency.jsonl"), content).unwrap();

        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_none());
    }

    #[test]
    fn collect_total_cost_tolerates_malformed_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let learn = tmp.path().join(".roko").join("learn");
        std::fs::create_dir_all(&learn).unwrap();
        let content = [
            r#"{"plan_id":"plan-1","cost_usd":0.02,"task_id":"t1"}"#,
            "this is not json",
            r#"{"plan_id":"plan-1","cost_usd":0.01,"task_id":"t2"}"#,
        ]
        .join("\n");
        std::fs::write(learn.join("efficiency.jsonl"), content).unwrap();

        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_some());
        assert!((cost.unwrap() - 0.03).abs() < 1e-9);
    }

    // ── Serialization tests ──────────────────────────────────────────

    #[test]
    fn report_serializes_with_new_fields() {
        let report = DiagnoseReport {
            plan_id: "test-plan".into(),
            status: "failed".into(),
            phase: Some("failed".into()),
            iteration: Some(1),
            failed_task: Some(FailedTaskInfo {
                task_id: "task-1".into(),
                last_error: Some("compile error".into()),
                files_changed: vec!["src/main.rs".into()],
                episode_ids: vec!["ep-1".into(), "ep-2".into()],
            }),
            gate_results: vec![GateResultInfo {
                gate_name: "compile:cargo".into(),
                rung: 1,
                passed: false,
                summary: "3 errors".into(),
                duration_ms: 5000,
                classified_errors: vec![ClassifiedError {
                    error_class: ErrorClass::CompileError,
                    file: Some("src/main.rs".into()),
                    line: Some(42),
                    error_summary: "mismatched types".into(),
                    suggestion: Some("Fix the type error".into()),
                }],
            }],
            run_state: None,
            git_state: None,
            suggested_recovery: vec![],
            total_cost_usd: Some(0.42),
        };

        let json = serde_json::to_string_pretty(&report).expect("serialize");
        let parsed: Value = serde_json::from_str(&json).expect("parse");

        // Verify total_cost_usd at top level.
        assert_eq!(parsed["total_cost_usd"].as_f64(), Some(0.42));

        // Verify episode_ids on failed_task.
        let ep_ids = parsed["failed_task"]["episode_ids"]
            .as_array()
            .expect("episode_ids array");
        assert_eq!(ep_ids.len(), 2);
        assert_eq!(ep_ids[0].as_str(), Some("ep-1"));

        // Verify classified_errors on gate_results[0].
        let classified = parsed["gate_results"][0]["classified_errors"]
            .as_array()
            .expect("classified_errors array");
        assert_eq!(classified.len(), 1);
        assert_eq!(
            classified[0]["error_class"].as_str(),
            Some("compile_error")
        );
        assert_eq!(classified[0]["file"].as_str(), Some("src/main.rs"));
        assert_eq!(classified[0]["line"].as_u64(), Some(42));
    }

    #[test]
    fn report_omits_null_total_cost() {
        let report = DiagnoseReport {
            plan_id: "test-plan".into(),
            status: "completed".into(),
            phase: Some("done".into()),
            iteration: Some(1),
            failed_task: None,
            gate_results: vec![],
            run_state: None,
            git_state: None,
            suggested_recovery: vec![],
            total_cost_usd: None,
        };
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(!json.contains("total_cost_usd"));
    }
}
