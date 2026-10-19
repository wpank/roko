//! `run_tests` with `build: "checks"` (gap-770cb5): run the task's own checks
//! mid-try.
//!
//! The verify steps that judge a try ran only once the agent handed off, so a
//! compile error in a C++ task came back as a whole new try. Dispatch leaves
//! the attempt's checks under the workspace's `.roko/state/task-checks/`,
//! outside the checkout the agent writes, and this mode runs them in the
//! checkout, in order, as the gate's environment would, stopping at the
//! first that fails and returning its output's tail.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use roko_core::child_env;
use roko_core::tool::{ToolCall, ToolContext, ToolError, ToolResult};
use serde::{Deserialize, Serialize};

use super::child_group;

/// The lines of a failing check's output the agent gets back.
const TAIL_LINES: usize = 60;

/// The most bytes of them.
const TAIL_BYTES: usize = 8 * 1024;

/// One of an attempt's checks, as dispatch leaves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCheck {
    /// The name its gate result has, such as `verify[0:build]`.
    pub label: String,
    /// The shell command.
    pub command: String,
    /// Its own time limit, when the task gives one.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Where dispatch leaves the checks of the attempt `attempt_key` in the
/// workspace at `workspace`: one file per attempt, named by its key.
#[must_use]
pub fn task_checks_path(workspace: &Path, attempt_key: &str) -> PathBuf {
    let name: String = attempt_key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    workspace
        .join(".roko/state/task-checks")
        .join(format!("{name}.json"))
}

/// Run the checks of the attempt `ctx` belongs to, within `budget` in all.
pub(super) async fn run(
    ctx: &ToolContext,
    budget: Duration,
    parent_env: Option<Vec<(String, OsString)>>,
) -> ToolResult {
    let key = &ctx.correlation.attempt_id;
    if key.is_empty() {
        return ToolResult::Err(ToolError::Other(
            "run_tests: this call belongs to no task, so it has no checks to run".into(),
        ));
    }
    let path = task_checks_path(&ctx.immune_root_path, key);
    let checks: Vec<TaskCheck> = match std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
    {
        Some(checks) => checks,
        None => {
            return ToolResult::Err(ToolError::Other(
                "run_tests: this task has no checks to run".into(),
            ));
        }
    };
    let parent_env = parent_env.unwrap_or_else(child_env::process_env);
    let started = Instant::now();
    let mut ran = Vec::new();
    for (number, check) in (1_usize..).zip(&checks) {
        let left = budget.saturating_sub(started.elapsed());
        let limit = check
            .timeout_ms
            .map_or(left, |ms| Duration::from_millis(ms).min(left));
        let mut cmd = tokio::process::Command::new("bash");
        cmd.args(["-o", "pipefail", "-c", check.command.as_str()])
            .current_dir(ctx.worktree());
        if let Err(error) =
            child_env::apply_check_env(cmd.as_std_mut(), parent_env.clone(), &ctx.env_passthrough)
        {
            return ToolResult::Err(ToolError::Other(format!(
                "run_tests: the checks did not run: their environment could not be set up: {error}"
            )));
        }
        let check_started = Instant::now();
        let output = match child_group::output_within(cmd, limit).await {
            Ok(Ok(output)) => output,
            Ok(Err(error)) => {
                return ToolResult::Err(ToolError::Other(format!(
                    "run_tests: check {number} ({}) could not start: {error}",
                    check.label
                )));
            }
            Err(_) => {
                return ToolResult::Err(ToolError::Other(format!(
                    "run_tests: check {number} ({}) did not finish within {} s; pass a larger \
                     timeout_ms (at most {} s) for a long build",
                    check.label,
                    limit.as_secs(),
                    ctx.max_timeout.as_secs()
                )));
            }
        };
        let secs = (check_started.elapsed().as_secs_f64() * 10.0).round() / 10.0;
        if !output.status.success() {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let exit = output
                .status
                .code()
                .map_or_else(|| "a signal".to_string(), |code| format!("exit {code}"));
            return ToolResult::Err(ToolError::Other(format!(
                "run_tests: check {number} of {} ({}) failed ({exit}) after {secs} s; the \
                 output's end:\n{}",
                checks.len(),
                check.label,
                tail(&text)
            )));
        }
        ran.push(serde_json::json!({ "check": check.label, "passed": true, "secs": secs }));
    }
    ToolResult::structured(
        serde_json::json!({ "build": "checks", "status": "ok", "checks": ran }).to_string(),
    )
}

/// The last [`TAIL_LINES`] lines of `text`, at most [`TAIL_BYTES`] of them.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let kept = lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n");
    if kept.len() <= TAIL_BYTES {
        return kept;
    }
    let mut start = kept.len() - TAIL_BYTES;
    while !kept.is_char_boundary(start) {
        start += 1;
    }
    kept[start..].to_string()
}
