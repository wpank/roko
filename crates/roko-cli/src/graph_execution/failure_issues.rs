//! GitHub issues for the tasks a plan run leaves failed (gap-cd51b7).
//!
//! When `[github]` turns the runner's GitHub workflow on (`auto_pr = true`,
//! with the repository named and `GITHUB_TOKEN` set), the run's
//! [`RunConfig::github_ops`](crate::runner::RunConfig::github_ops) is the
//! live adapter, and each task that a plan's run leaves failed gets one issue
//! there, labelled `<label_prefix>task-failure`: the label `roko github
//! status` lists. A run that was interrupted or cancelled files none, and
//! without that configuration nothing is filed.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::sync::Arc;

use roko_core::obs::LogScrubber;
use roko_graph::GraphOutput;

use crate::github_ops::GitHubOps;

/// Longest excerpt of a task's error that an issue quotes, in characters.
const MAX_ERROR_CHARS: usize = 2_000;

/// Files the issues for a run's failed tasks through a [`GitHubOps`] adapter.
pub struct FailureIssues {
    ops: Arc<dyn GitHubOps>,
    label: String,
}

impl FailureIssues {
    /// Issues filed through `ops`, labelled `<label_prefix>task-failure`.
    pub fn new(ops: Arc<dyn GitHubOps>, label_prefix: &str) -> Self {
        Self {
            ops,
            label: format!("{label_prefix}task-failure"),
        }
    }

    /// File one issue for each task in `failed`, which run `run_id` of plan
    /// `plan_id` left failed, quoting the error its node in `output` settled
    /// with. Best effort: a call that fails is logged, and the other tasks'
    /// issues are still filed. Returns the numbers of the issues filed.
    pub async fn file(
        &self,
        plan_id: &str,
        run_id: &str,
        output: &GraphOutput,
        failed: &BTreeSet<String>,
        node_titles: &HashMap<String, String>,
    ) -> Vec<u64> {
        let labels = [self.label.clone()];
        let mut filed = Vec::new();
        for result in output
            .node_results
            .iter()
            .filter(|result| failed.contains(&result.node_id))
        {
            let task_id = result.node_id.as_str();
            let task_title = node_titles.get(task_id).map_or("", String::as_str);
            let (title, body) = failure_issue(
                plan_id,
                run_id,
                task_id,
                task_title,
                result.error.as_deref(),
            );
            match self
                .ops
                .create_task_issue(task_id, &title, &body, &labels)
                .await
            {
                Ok(number) => {
                    tracing::info!(
                        plan_id,
                        task_id,
                        issue = number,
                        "filed a GitHub issue for a failed task"
                    );
                    filed.push(number);
                }
                Err(error) => tracing::warn!(
                    plan_id,
                    task_id,
                    %error,
                    "could not file a GitHub issue for a failed task"
                ),
            }
        }
        filed
    }
}

/// The title and body of the issue for task `task_id`, titled `task_title`,
/// which run `run_id` of plan `plan_id` left failed with `error`. The body
/// quotes the task's details and its error in indented code blocks, so
/// nothing from the plan or the error renders or mentions anyone. The error
/// is quoted with the process's known secrets and anything shaped like a key
/// redacted, then cut to [`MAX_ERROR_CHARS`] characters.
fn failure_issue(
    plan_id: &str,
    run_id: &str,
    task_id: &str,
    task_title: &str,
    error: Option<&str>,
) -> (String, String) {
    let title = format!("Task {task_id} failed in plan {plan_id}");
    let mut details = format!("plan: {plan_id}\nrun: {run_id}\ntask: {task_id}\n");
    if !task_title.is_empty() {
        let _ = writeln!(details, "title: {task_title}");
    }
    let mut body = String::from("A Roko plan run left this task failed.\n\n");
    push_code_block(&mut body, &details);
    if let Some(error) = error.map(str::trim).filter(|error| !error.is_empty()) {
        let redacted = LogScrubber::new().scrub(&roko_core::obs::scrub_secrets(error));
        let mut excerpt = redacted.chars().take(MAX_ERROR_CHARS).collect::<String>();
        if excerpt.len() < redacted.len() {
            excerpt.push_str("\n[...]");
        }
        body.push_str("\nError:\n\n");
        push_code_block(&mut body, &excerpt);
    }
    (title, body)
}

/// Append `text` to `body` as the lines of an indented code block.
fn push_code_block(body: &mut String, text: &str) {
    for line in text.lines() {
        let _ = writeln!(body, "    {line}");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::time::Duration;

    use async_trait::async_trait;
    use roko_graph::engine::NodeStatus;

    use super::*;
    use crate::github_ops::CiStatus;

    /// One `create_task_issue` call: task id, title, body and labels.
    type IssueCall = (String, String, String, Vec<String>);

    /// Records the issues it is asked to file; refuses those of `refuse`.
    #[derive(Default)]
    struct RecordingGitHubOps {
        issues: Mutex<Vec<IssueCall>>,
        refuse: Option<&'static str>,
    }

    impl RecordingGitHubOps {
        fn issues(&self) -> Vec<IssueCall> {
            self.issues.lock().expect("issues lock").clone()
        }
    }

    #[async_trait]
    impl GitHubOps for RecordingGitHubOps {
        async fn create_plan_branch(&self, plan_id: &str, _base: &str) -> Result<String, String> {
            Ok(format!("roko/plan/{plan_id}"))
        }

        async fn open_pr(&self, _branch: &str, _title: &str, _body: &str) -> Result<u64, String> {
            Err("not expected".to_string())
        }

        async fn check_ci_status(&self, _ref_name: &str) -> Result<CiStatus, String> {
            Ok(CiStatus::Pending)
        }

        async fn merge_pr(&self, _pr_number: u64, _method: &str) -> Result<(), String> {
            Err("not expected".to_string())
        }

        async fn create_task_issue(
            &self,
            task_id: &str,
            title: &str,
            body: &str,
            labels: &[String],
        ) -> Result<u64, String> {
            if self.refuse == Some(task_id) {
                return Err("issues are disabled".to_string());
            }
            let mut issues = self.issues.lock().expect("issues lock");
            issues.push((
                task_id.to_string(),
                title.to_string(),
                body.to_string(),
                labels.to_vec(),
            ));
            Ok(100 + issues.len() as u64)
        }

        async fn close_issue(&self, _number: u64) -> Result<(), String> {
            Err("not expected".to_string())
        }

        async fn comment_pr(&self, _pr_number: u64, _body: &str) -> Result<(), String> {
            Err("not expected".to_string())
        }
    }

    fn node(id: &str, status: NodeStatus, error: Option<&str>) -> roko_graph::NodeResult {
        roko_graph::NodeResult {
            node_id: id.to_string(),
            cell_type: "task-executor".to_string(),
            status,
            duration: Duration::ZERO,
            error: error.map(str::to_string),
            output_count: 0,
            is_stub: false,
            blocked_by: None,
            timing: roko_graph::NodeTiming::default(),
        }
    }

    fn output(node_results: Vec<roko_graph::NodeResult>) -> GraphOutput {
        GraphOutput {
            graph_name: "plan-a".to_string(),
            success: false,
            node_results,
            total_duration: Duration::ZERO,
            gate_verdicts: BTreeMap::new(),
        }
    }

    fn titles() -> HashMap<String, String> {
        HashMap::from([
            ("T1".to_string(), "Add the parser".to_string()),
            ("T2".to_string(), "Wire the parser in".to_string()),
            ("T3".to_string(), "Document the parser".to_string()),
        ])
    }

    #[tokio::test]
    async fn failed_tasks_are_filed_through_github_ops_with_the_failure_label() {
        let ops = Arc::new(RecordingGitHubOps::default());
        let issues = FailureIssues::new(ops.clone(), "roko/");
        let output = output(vec![
            node("T1", NodeStatus::Complete, None),
            node(
                "T2",
                NodeStatus::Failed,
                Some("verify step `cargo test` failed"),
            ),
            node("T3", NodeStatus::Failed, None),
        ]);
        let failed = BTreeSet::from(["T2".to_string(), "T3".to_string()]);

        let filed = issues
            .file("plan-a", "run-7", &output, &failed, &titles())
            .await;

        assert_eq!(filed, [101, 102]);
        let calls = ops.issues();
        let filed_tasks = calls
            .iter()
            .map(|(task, title, _, labels)| (task.as_str(), title.as_str(), labels.join(",")))
            .collect::<Vec<_>>();
        assert_eq!(
            filed_tasks,
            [
                (
                    "T2",
                    "Task T2 failed in plan plan-a",
                    "roko/task-failure".to_string()
                ),
                (
                    "T3",
                    "Task T3 failed in plan plan-a",
                    "roko/task-failure".to_string()
                ),
            ]
        );
        let body = &calls[0].2;
        assert!(body.contains("    run: run-7\n"), "{body}");
        assert!(body.contains("    title: Wire the parser in\n"), "{body}");
        assert!(
            body.contains("Error:\n\n    verify step `cargo test` failed\n"),
            "{body}"
        );
        assert!(!calls[1].2.contains("Error:"), "{}", calls[1].2);
    }

    #[tokio::test]
    async fn a_refused_issue_leaves_the_other_github_ops_issues_filed() {
        let ops = Arc::new(RecordingGitHubOps {
            refuse: Some("T2"),
            ..RecordingGitHubOps::default()
        });
        let issues = FailureIssues::new(ops.clone(), "automation/");
        let output = output(vec![
            node("T2", NodeStatus::Failed, Some("timed out")),
            node("T3", NodeStatus::Failed, Some("gate failed")),
        ]);
        let failed = BTreeSet::from(["T2".to_string(), "T3".to_string()]);

        let filed = issues
            .file("plan-a", "run-8", &output, &failed, &titles())
            .await;

        assert_eq!(filed, [101]);
        let calls = ops.issues();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "T3");
        assert_eq!(calls[0].3, ["automation/task-failure".to_string()]);
    }

    #[test]
    fn github_ops_failure_issue_quotes_a_redacted_error_in_code_blocks() {
        let key = format!("sk-{}", "a1b2c3d4e5".repeat(4));
        let error = format!("provider said: invalid key {key}\n@octocat look\n# Heading");

        let (_, body) = failure_issue("plan-a", "run-9", "T1", "", Some(&error));

        assert!(!body.contains(&key), "{body}");
        assert!(body.contains("[REDACTED"), "{body}");
        assert!(body.contains("\n    @octocat look\n"), "{body}");
        assert!(body.contains("\n    # Heading\n"), "{body}");
        assert!(!body.contains("title:"), "{body}");
        for line in body.lines().skip(1).filter(|line| !line.is_empty()) {
            assert!(line.starts_with("    ") || line == "Error:", "{line:?}");
        }
    }

    #[test]
    fn github_ops_failure_issue_cuts_a_long_error() {
        let error = "é".repeat(MAX_ERROR_CHARS + 10);

        let (_, body) = failure_issue("plan-a", "run-9", "T1", "", Some(&error));

        let quoted = body.split("Error:\n\n    ").nth(1).expect("quoted error");
        assert!(quoted.starts_with(&"é".repeat(MAX_ERROR_CHARS)));
        assert!(quoted.ends_with("\n    [...]\n"), "{quoted}");
        assert_eq!(quoted.matches('é').count(), MAX_ERROR_CHARS);
    }
}
