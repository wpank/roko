//! `run_tests` with `build: "checks"` (gap-770cb5): run the task's own checks
//! mid-try.
//!
//! The verify steps that judge a try ran only once the agent handed off, so a
//! compile error in a C++ task came back as a whole new try. Dispatch leaves
//! the attempt's checks under the workspace's `.roko/state/task-checks/`,
//! outside the checkout the agent writes, and this mode runs them in the
//! checkout, in order, as the gate's environment would, stopping at the
//! first that fails and returning its output's tail.
//!
//! Each run of an attempt's checks, the agent's and the hand-off check's
//! (bug-cbbaa8), is recorded beside its checks file with the tree it judged
//! (gap-56f761), so dispatch can judge a try on the best tree its checks saw
//! when its last edit made the tree worse.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use roko_core::child_env;
use roko_core::tool::{ToolContext, ToolError, ToolResult};
use serde::{Deserialize, Serialize};

use super::child_group;

/// The lines of a failing check's output the agent gets back.
pub(super) const TAIL_LINES: usize = 60;

/// The most bytes of them.
pub(super) const TAIL_BYTES: usize = 8 * 1024;

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

/// Where the runs of the attempt `attempt_key`'s checks are recorded, one
/// [`CheckRun`] per line, beside its checks file ([`task_checks_path`]).
#[must_use]
pub fn task_check_runs_path(workspace: &Path, attempt_key: &str) -> PathBuf {
    task_checks_path(workspace, attempt_key).with_extension("runs.jsonl")
}

/// One run of an attempt's checks, as [`run_checks`] records it
/// (gap-56f761): the tree the checks judged, the checks that passed, and the
/// one that did not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRun {
    /// When the checks ran, in Unix milliseconds.
    pub at_ms: i64,
    /// The git tree id of the checkout they ran in.
    pub tree: String,
    /// The labels of the checks that passed, in order.
    pub passed: Vec<String>,
    /// The check that did not pass, when one did not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<FailedCheck>,
}

/// The check of a [`CheckRun`] that did not pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedCheck {
    /// Its label.
    pub label: String,
    /// The tail of what it printed on stdout, then on stderr.
    pub output: String,
}

/// The runs recorded at `path` ([`task_check_runs_path`]), in order; lines
/// that do not parse are skipped.
#[must_use]
pub fn read_check_runs(path: &Path) -> Vec<CheckRun> {
    std::fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// What running an attempt's checks came to ([`run_checks`]).
#[derive(Debug, Clone, PartialEq)]
pub enum ChecksOutcome {
    /// Every check passed: each one's label and seconds, in order.
    Passed(Vec<(String, f64)>),
    /// A check failed.
    Failed {
        /// Its index among the checks.
        index: usize,
        /// Its exit code; `None` when a signal ended it.
        code: Option<i32>,
        /// How long it ran, in seconds.
        secs: f64,
        /// What it printed on stdout.
        stdout: String,
        /// What it printed on stderr.
        stderr: String,
        /// The checks that passed before it: each one's label and seconds.
        passed: Vec<(String, f64)>,
    },
    /// A check did not finish in time.
    TimedOut {
        /// Its index among the checks.
        index: usize,
        /// The time it had.
        limit: Duration,
    },
    /// The checks did not run, as `run_tests` says why.
    NotRun(String),
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
    match run_checks(&checks, ctx, budget, parent_env).await {
        ChecksOutcome::Passed(ran) => ToolResult::structured(
            serde_json::json!({ "build": "checks", "status": "ok", "checks": checks_json(&ran) })
                .to_string(),
        ),
        ChecksOutcome::Failed {
            index,
            code,
            secs,
            stdout,
            stderr,
            passed,
        } => {
            let exit = code.map_or_else(|| "a signal".to_string(), |code| format!("exit {code}"));
            // bug-1f8db7: a failing check is the tool's answer, not a tool
            // failure: as an error it would count toward a contract's
            // consecutive-failure limit while the agent fixes its work.
            ToolResult::structured(
                serde_json::json!({
                    "build": "checks",
                    "status": "failed",
                    "summary": format!(
                        "check {} of {} ({}) failed ({exit}) after {secs} s",
                        index + 1,
                        checks.len(),
                        checks[index].label
                    ),
                    "checks": checks_json(&passed),
                    "output": tail(&format!("{stdout}{stderr}")),
                })
                .to_string(),
            )
        }
        ChecksOutcome::TimedOut { index, limit } => ToolResult::Err(ToolError::Other(format!(
            "run_tests: check {} ({}) did not finish within {} s; pass a larger timeout_ms (at \
             most {} s) for a long build",
            index + 1,
            checks[index].label,
            limit.as_secs(),
            ctx.max_timeout.as_secs()
        ))),
        ChecksOutcome::NotRun(why) => ToolResult::Err(ToolError::Other(why)),
    }
}

/// Run `checks` in `ctx`'s checkout, in order, stopping at the first that
/// does not pass.
///
/// They run as the gate's environment would, within `budget` in all and each
/// within its own time limit. Its tasks' agents run them with `run_tests`
/// (gap-770cb5), and roko runs them when a plan attempt's agent hands back
/// (bug-cbbaa8). `parent_env` stands in for roko's own environment when
/// given. A run in an attempt is recorded with the tree it judged
/// ([`CheckRun`], gap-56f761).
pub async fn run_checks(
    checks: &[TaskCheck],
    ctx: &ToolContext,
    budget: Duration,
    parent_env: Option<Vec<(String, OsString)>>,
) -> ChecksOutcome {
    let key = &ctx.correlation.attempt_id;
    let tree = if key.is_empty() {
        None
    } else {
        super::checkout_tree::snapshot_tree(ctx.worktree()).await
    };
    let outcome = run_each(checks, ctx, budget, parent_env).await;
    if let Some(tree) = tree {
        record_run(
            &task_check_runs_path(&ctx.immune_root_path, key),
            checks,
            &outcome,
            tree,
        );
    }
    outcome
}

/// Append the run of `checks` that came to `outcome` on the tree `tree` to
/// the record at `path`; a run that did not run its checks is not one.
fn record_run(path: &Path, checks: &[TaskCheck], outcome: &ChecksOutcome, tree: String) {
    use std::io::Write;

    let labels = |ran: &[(String, f64)]| -> Vec<String> {
        ran.iter().map(|(label, _)| label.clone()).collect()
    };
    let (passed, failed) = match outcome {
        ChecksOutcome::Passed(ran) => (labels(ran), None),
        ChecksOutcome::Failed {
            index,
            stdout,
            stderr,
            passed,
            ..
        } => (
            labels(passed),
            Some(FailedCheck {
                label: checks[*index].label.clone(),
                output: format!("{}\n{}", tail(stdout), tail(stderr)),
            }),
        ),
        ChecksOutcome::TimedOut { index, limit } => (
            checks[..*index]
                .iter()
                .map(|check| check.label.clone())
                .collect(),
            Some(FailedCheck {
                label: checks[*index].label.clone(),
                output: format!("did not finish within {} s", limit.as_secs()),
            }),
        ),
        ChecksOutcome::NotRun(_) => return,
    };
    let at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        });
    let run = CheckRun {
        at_ms,
        tree,
        passed,
        failed,
    };
    let written = serde_json::to_string(&run)
        .map_err(std::io::Error::other)
        .and_then(|line| {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            writeln!(file, "{line}")
        });
    if let Err(error) = written {
        tracing::debug!(%error, "the checks' run was not recorded");
    }
}

/// [`run_checks`] without the record.
async fn run_each(
    checks: &[TaskCheck],
    ctx: &ToolContext,
    budget: Duration,
    parent_env: Option<Vec<(String, OsString)>>,
) -> ChecksOutcome {
    let parent_env = parent_env.unwrap_or_else(child_env::process_env);
    let started = Instant::now();
    let mut ran = Vec::new();
    for (index, check) in checks.iter().enumerate() {
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
            return ChecksOutcome::NotRun(format!(
                "run_tests: the checks did not run: their environment could not be set up: {error}"
            ));
        }
        let check_started = Instant::now();
        let output = match child_group::output_within(cmd, limit).await {
            Ok(Ok(output)) => output,
            Ok(Err(error)) => {
                return ChecksOutcome::NotRun(format!(
                    "run_tests: check {} ({}) could not start: {error}",
                    index + 1,
                    check.label
                ));
            }
            Err(_) => return ChecksOutcome::TimedOut { index, limit },
        };
        let secs = (check_started.elapsed().as_secs_f64() * 10.0).round() / 10.0;
        if !output.status.success() {
            return ChecksOutcome::Failed {
                index,
                code: output.status.code(),
                secs,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                passed: ran,
            };
        }
        ran.push((check.label.clone(), secs));
    }
    ChecksOutcome::Passed(ran)
}

/// Checks that passed, each with its label and seconds, as `run_tests`
/// lists them.
fn checks_json(passed: &[(String, f64)]) -> Vec<serde_json::Value> {
    passed
        .iter()
        .map(|(label, secs)| serde_json::json!({ "check": label, "passed": true, "secs": secs }))
        .collect()
}

/// The last [`TAIL_LINES`] lines of `text`, at most [`TAIL_BYTES`] of them.
fn tail(text: &str) -> String {
    tail_of(text, TAIL_LINES, TAIL_BYTES)
}

/// The last `max_lines` lines of `text`, at most `max_bytes` of them.
pub(super) fn tail_of(text: &str, max_lines: usize, max_bytes: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let kept = lines[lines.len().saturating_sub(max_lines)..].join("\n");
    if kept.len() <= max_bytes {
        return kept;
    }
    let mut start = kept.len() - max_bytes;
    while !kept.is_char_boundary(start) {
        start += 1;
    }
    kept[start..].to_string()
}
