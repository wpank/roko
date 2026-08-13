//! PR auto-update on gate results.
//!
//! When `github.auto_update_prs = true` in `roko.toml`, the runner posts a
//! comment to the associated PR on every gate completion and toggles
//! `gate-passed` / `gate-failed` labels.
//!
//! This is a **best-effort** feature: failures are logged and never propagate
//! to the caller. Missing config, missing `GITHUB_TOKEN`, or API errors all
//! result in a warning log line and nothing more.

use roko_core::config::GitHubConfig;
use tracing::{debug, warn};

use super::types::GateCompletion;

// ─── Public entry point ───────────────────────────────────────────────────

/// Post a comment and update labels on a PR based on a gate result.
///
/// The function is fire-and-forget: all errors are logged and swallowed.
/// It runs the blocking GitHub client on `spawn_blocking` so it never
/// blocks the async event loop.
///
/// `pr_number` is the PR to update (0 or absent means "skip").
pub async fn update_pr_on_gate_result(
    github_cfg: &GitHubConfig,
    task_name: &str,
    completion: &GateCompletion,
    pr_number: u64,
) {
    if pr_number == 0 {
        debug!("pr_gate_update: no PR number, skipping");
        return;
    }
    let Some(owner) = github_cfg.owner.as_deref() else {
        debug!("pr_gate_update: github.owner not configured, skipping");
        return;
    };
    let Some(repo) = github_cfg.repo.as_deref() else {
        debug!("pr_gate_update: github.repo not configured, skipping");
        return;
    };

    let owner = owner.to_owned();
    let repo = repo.to_owned();
    let label_prefix = github_cfg.label_prefix.clone();
    let comment_body = build_gate_comment(task_name, completion);
    let passed = completion.passed;

    // Run blocking HTTP calls off the async executor.
    let result = tokio::task::spawn_blocking(move || {
        do_update_pr(
            &owner,
            &repo,
            &label_prefix,
            pr_number,
            passed,
            &comment_body,
        )
    })
    .await;

    match result {
        Ok(Ok(())) => {
            debug!(pr = pr_number, passed, "pr_gate_update: PR updated");
        }
        Ok(Err(msg)) => {
            warn!(pr = pr_number, %msg, "pr_gate_update: failed to update PR");
        }
        Err(err) => {
            warn!(pr = pr_number, %err, "pr_gate_update: spawn_blocking panicked");
        }
    }
}

// ─── Internals ────────────────────────────────────────────────────────────

/// Build the Markdown comment body for a gate result.
pub(crate) fn build_gate_comment(task_name: &str, completion: &GateCompletion) -> String {
    let icon = if completion.passed {
        "white_check_mark"
    } else {
        "x"
    };
    let status = if completion.passed {
        "PASSED"
    } else {
        "FAILED"
    };

    let mut body = format!(
        "### :{icon}: Gate {status}\n\n\
         | Field | Value |\n\
         |---|---|\n\
         | **Task** | `{task_name}` |\n\
         | **Rung** | {} |\n\
         | **Duration** | {}ms |\n",
        completion.rung, completion.duration_ms,
    );

    if !completion.verdicts.is_empty() {
        body.push_str("\n#### Verdicts\n\n| Gate | Result | Summary |\n|---|---|---|\n");
        for v in &completion.verdicts {
            let v_icon = if v.passed { "white_check_mark" } else { "x" };
            let summary_escaped = v.summary.replace('|', "\\|");
            let summary_trimmed = if summary_escaped.len() > 200 {
                format!("{}...", &summary_escaped[..200])
            } else {
                summary_escaped
            };
            body.push_str(&format!(
                "| `{}` | :{v_icon}: | {summary_trimmed} |\n",
                v.gate_name,
            ));
        }
    }

    body
}

/// Compute the label names for passed / failed states.
pub(crate) fn gate_labels(prefix: &str) -> (String, String) {
    let passed = format!("{prefix}gate-passed");
    let failed = format!("{prefix}gate-failed");
    (passed, failed)
}

/// Blocking helper that performs the actual GitHub API calls.
fn do_update_pr(
    owner: &str,
    repo: &str,
    label_prefix: &str,
    pr_number: u64,
    passed: bool,
    comment_body: &str,
) -> Result<(), String> {
    let client = roko_mcp_github::GitHubClient::from_env()
        .map_err(|e| format!("GitHubClient::from_env: {e}"))?;

    // 1. Post the comment.
    client
        .comment_pr(owner, repo, pr_number, comment_body)
        .map_err(|e| format!("comment_pr: {e}"))?;

    // 2. Toggle labels: add the current, remove the opposite.
    let (passed_label, failed_label) = gate_labels(label_prefix);
    let (add, remove) = if passed {
        (passed_label, failed_label)
    } else {
        (failed_label, passed_label)
    };

    client
        .add_labels(owner, repo, pr_number, &[add])
        .map_err(|e| format!("add_labels: {e}"))?;

    // remove_label is idempotent (404 = already absent).
    client
        .remove_label(owner, repo, pr_number, &remove)
        .map_err(|e| format!("remove_label: {e}"))?;

    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::types::{GateCompletionKind, GateVerdictSummary};

    fn sample_completion(passed: bool) -> GateCompletion {
        GateCompletion {
            kind: GateCompletionKind::Gate,
            attempt: None,
            effect: None,
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            rung: 2,
            passed,
            failure_kind: None,
            verdicts: vec![
                GateVerdictSummary {
                    gate_name: "compile".into(),
                    passed: true,
                    summary: "cargo check ok".into(),
                    skipped: false,
                    error_digest: None,
                    failure_kind: None,
                    rung_index: Some(0),
                },
                GateVerdictSummary {
                    gate_name: "clippy".into(),
                    passed,
                    summary: if passed {
                        "no warnings".into()
                    } else {
                        "2 warnings".into()
                    },
                    skipped: false,
                    error_digest: None,
                    failure_kind: None,
                    rung_index: Some(1),
                },
            ],
            output: String::new(),
            duration_ms: 4500,
            selected_rungs: vec!["compile".into(), "clippy".into()],
        }
    }

    #[test]
    fn build_gate_comment_passed_contains_checkmark() {
        let completion = sample_completion(true);
        let body = build_gate_comment("E46-T09-wire-pr-updates", &completion);
        assert!(body.contains("PASSED"), "should contain PASSED");
        assert!(
            body.contains(":white_check_mark:"),
            "should contain checkmark"
        );
        assert!(
            body.contains("E46-T09-wire-pr-updates"),
            "should contain task name"
        );
        assert!(body.contains("4500ms"), "should contain duration");
        assert!(
            body.contains("| `compile` |"),
            "should list compile verdict"
        );
        assert!(body.contains("| `clippy` |"), "should list clippy verdict");
    }

    #[test]
    fn build_gate_comment_failed_contains_x() {
        let completion = sample_completion(false);
        let body = build_gate_comment("failing-task", &completion);
        assert!(body.contains("FAILED"), "should contain FAILED");
        assert!(body.contains(":x:"), "should contain x emoji");
        assert!(
            body.contains("2 warnings"),
            "should contain failure summary"
        );
    }

    #[test]
    fn build_gate_comment_no_verdicts_omits_table() {
        let completion = GateCompletion {
            kind: GateCompletionKind::Gate,
            attempt: None,
            effect: None,
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            rung: 0,
            passed: true,
            failure_kind: None,
            verdicts: vec![],
            output: String::new(),
            duration_ms: 100,
            selected_rungs: vec![],
        };
        let body = build_gate_comment("empty-task", &completion);
        assert!(
            !body.contains("Verdicts"),
            "should not contain Verdicts header"
        );
    }

    #[test]
    fn gate_labels_uses_prefix() {
        let (passed, failed) = gate_labels("roko/");
        assert_eq!(passed, "roko/gate-passed");
        assert_eq!(failed, "roko/gate-failed");
    }

    #[test]
    fn gate_labels_custom_prefix() {
        let (passed, failed) = gate_labels("myapp/");
        assert_eq!(passed, "myapp/gate-passed");
        assert_eq!(failed, "myapp/gate-failed");
    }

    #[test]
    fn build_gate_comment_truncates_long_summary() {
        let long_summary = "x".repeat(300);
        let completion = GateCompletion {
            kind: GateCompletionKind::Gate,
            attempt: None,
            effect: None,
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            rung: 0,
            passed: false,
            failure_kind: None,
            verdicts: vec![GateVerdictSummary {
                gate_name: "test".into(),
                passed: false,
                summary: long_summary,
                skipped: false,
                error_digest: None,
                failure_kind: None,
                rung_index: Some(0),
            }],
            output: String::new(),
            duration_ms: 100,
            selected_rungs: vec![],
        };
        let body = build_gate_comment("long-task", &completion);
        // The truncated summary should end with "..."
        assert!(body.contains("..."), "should truncate long summaries");
        // The full 300-char string should NOT be present
        assert!(
            !body.contains(&"x".repeat(300)),
            "should not contain the full long string"
        );
    }

    #[tokio::test]
    async fn update_pr_skips_when_pr_number_is_zero() {
        // Should return without error when pr_number is 0.
        let cfg = GitHubConfig::default();
        let completion = sample_completion(true);
        // No panic, no error — just returns.
        update_pr_on_gate_result(&cfg, "test-task", &completion, 0).await;
    }

    #[tokio::test]
    async fn update_pr_skips_when_owner_missing() {
        let cfg = GitHubConfig {
            repo: Some("roko".into()),
            ..Default::default()
        };
        let completion = sample_completion(true);
        update_pr_on_gate_result(&cfg, "test-task", &completion, 42).await;
    }

    #[tokio::test]
    async fn update_pr_skips_when_repo_missing() {
        let cfg = GitHubConfig {
            owner: Some("nunchi".into()),
            ..Default::default()
        };
        let completion = sample_completion(true);
        update_pr_on_gate_result(&cfg, "test-task", &completion, 42).await;
    }
}
