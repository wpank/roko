//! Non-blocking, ordered GitHub lifecycle side effects for the plan runner.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::github_ops::{CiStatus, GitHubOps};
use roko_core::config::GitHubConfig;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

const COMMENT_OUTPUT_LIMIT: usize = 2_000;

#[derive(Debug, Clone, Copy)]
struct CiRetryPolicy {
    interval: Duration,
    max_retries: u8,
}

impl Default for CiRetryPolicy {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            max_retries: 5,
        }
    }
}

#[async_trait::async_trait]
trait BranchPublisher: Send + Sync {
    async fn publish(
        &self,
        workdir: &Path,
        branch: &str,
        accepted_commit: &str,
    ) -> Result<(), String>;
}

#[derive(Debug, Default)]
struct GitBranchPublisher;

#[async_trait::async_trait]
impl BranchPublisher for GitBranchPublisher {
    async fn publish(
        &self,
        workdir: &Path,
        branch: &str,
        accepted_commit: &str,
    ) -> Result<(), String> {
        validate_branch(branch)?;
        validate_commit_oid(accepted_commit)?;
        let refspec = format!("{accepted_commit}:refs/heads/{branch}");
        let output = tokio::process::Command::new("git")
            .args(["push", "--porcelain", "origin", &refspec])
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|error| format!("start git push: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "git push exited with {}: {}",
                output.status,
                truncate_chars(stderr.trim(), 1_000)
            ))
        }
    }
}

#[derive(Clone)]
pub(crate) struct GitHubWorkflow {
    tx: mpsc::UnboundedSender<GitHubCommand>,
}

impl GitHubWorkflow {
    pub(crate) fn start(config: GitHubConfig, enabled: bool) -> Self {
        Self::start_with_policy(
            config,
            enabled,
            CiRetryPolicy::default(),
            Arc::new(GitBranchPublisher),
        )
    }

    fn start_with_policy(
        config: GitHubConfig,
        enabled: bool,
        ci_policy: CiRetryPolicy,
        publisher: Arc<dyn BranchPublisher>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(run_worker(rx, config, enabled, ci_policy, publisher));
        Self { tx }
    }

    pub(crate) fn plan_started(&self, ops: Arc<dyn GitHubOps>, plan_id: String, base_sha: String) {
        self.send(GitHubCommand::PlanStarted {
            ops,
            plan_id,
            base_sha,
        });
    }

    pub(crate) fn task_finished(&self, ops: Arc<dyn GitHubOps>, result: TaskGitHubResult) {
        self.send(GitHubCommand::TaskFinished { ops, result });
    }

    pub(crate) fn plan_finished(&self, ops: Arc<dyn GitHubOps>, summary: PlanGitHubSummary) {
        self.send(GitHubCommand::PlanFinished { ops, summary });
    }

    /// Queue the remote CI/merge sequence only after local merge regression passed.
    pub(crate) fn merge_regression_passed(
        &self,
        ops: Arc<dyn GitHubOps>,
        plan_id: String,
        workdir: PathBuf,
        accepted_commit: String,
    ) {
        self.send(GitHubCommand::MergeRegressionPassed {
            ops,
            plan_id,
            workdir,
            accepted_commit,
        });
    }

    fn send(&self, command: GitHubCommand) {
        if self.tx.send(command).is_err() {
            warn!("GitHub workflow worker stopped; side effect skipped");
        }
    }

    /// Wait until every previously queued GitHub side effect has settled.
    /// Called only after the runner event loop is terminal, so CI timers never
    /// stall task scheduling or other select branches.
    pub(crate) async fn flush(&self) {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(GitHubCommand::Barrier(tx)).is_err() {
            warn!("GitHub workflow worker stopped before flush");
            return;
        }
        if rx.await.is_err() {
            warn!("GitHub workflow worker stopped during flush");
        }
    }

    #[cfg(test)]
    async fn barrier(&self) {
        self.flush().await;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TaskGitHubResult {
    pub plan_id: String,
    pub task_id: String,
    pub task_title: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct PlanGitHubSummary {
    pub plan_id: String,
    pub succeeded: bool,
    pub tasks_passed: usize,
    pub tasks_failed: usize,
    pub duration_ms: u64,
    pub cost_usd: f64,
}

enum GitHubCommand {
    PlanStarted {
        ops: Arc<dyn GitHubOps>,
        plan_id: String,
        base_sha: String,
    },
    TaskFinished {
        ops: Arc<dyn GitHubOps>,
        result: TaskGitHubResult,
    },
    PlanFinished {
        ops: Arc<dyn GitHubOps>,
        summary: PlanGitHubSummary,
    },
    MergeRegressionPassed {
        ops: Arc<dyn GitHubOps>,
        plan_id: String,
        workdir: PathBuf,
        accepted_commit: String,
    },
    Barrier(oneshot::Sender<()>),
}

#[derive(Debug, Clone)]
struct PlanGitHubState {
    branch: String,
    pr_number: u64,
}

async fn run_worker(
    mut rx: mpsc::UnboundedReceiver<GitHubCommand>,
    config: GitHubConfig,
    enabled: bool,
    ci_policy: CiRetryPolicy,
    publisher: Arc<dyn BranchPublisher>,
) {
    let mut plans = HashMap::<String, PlanGitHubState>::new();
    let mut task_issues = HashMap::<(String, String), u64>::new();

    while let Some(command) = rx.recv().await {
        match command {
            GitHubCommand::PlanStarted {
                ops,
                plan_id,
                base_sha,
            } => {
                if !enabled || !config.auto_pr || plans.contains_key(&plan_id) {
                    continue;
                }
                let branch = match ops.create_plan_branch(&plan_id, &base_sha).await {
                    Ok(branch) => branch,
                    Err(error) => {
                        warn!(%plan_id, %error, "could not create GitHub plan branch (non-fatal)");
                        continue;
                    }
                };
                let title = format!("[roko] Plan {plan_id}");
                let body = format!(
                    "Automated draft pull request for roko plan `{plan_id}`.\n\n\
                     Roko will report terminal task gates here and merge only after local regression and GitHub CI pass."
                );
                match ops.open_pr(&branch, &title, &body).await {
                    Ok(pr_number) => {
                        info!(%plan_id, %branch, pr_number, draft = true, "opened GitHub plan PR");
                        plans.insert(plan_id, PlanGitHubState { branch, pr_number });
                    }
                    Err(error) => {
                        warn!(%plan_id, %error, "could not open GitHub plan PR (non-fatal)");
                    }
                }
            }
            GitHubCommand::TaskFinished { ops, result } => {
                if !enabled {
                    continue;
                }
                let issue_key = (result.plan_id.clone(), result.task_id.clone());
                if result.passed {
                    if let Some(number) = task_issues.remove(&issue_key)
                        && let Err(error) = ops.close_issue(number).await
                    {
                        warn!(number, %error, "could not close recovered task issue (non-fatal)");
                        task_issues.insert(issue_key, number);
                    }
                } else if !task_issues.contains_key(&issue_key) {
                    let title = format!(
                        "[roko] Task {} failed: {}",
                        result.task_id, result.task_title
                    );
                    let body = format!(
                        "Plan: `{}`\n\nTerminal gate output:\n\n```text\n{}\n```",
                        result.plan_id,
                        truncate_chars(&result.output, COMMENT_OUTPUT_LIMIT),
                    );
                    let labels = vec![
                        format!("{}task-failure", config.label_prefix),
                        format!("{}plan/{}", config.label_prefix, result.plan_id),
                    ];
                    match ops
                        .create_task_issue(&result.task_id, &title, &body, &labels)
                        .await
                    {
                        Ok(number) => {
                            task_issues.insert(issue_key, number);
                        }
                        Err(error) => {
                            warn!(plan_id = %result.plan_id, task_id = %result.task_id, %error,
                                "could not create terminal task issue (non-fatal)");
                        }
                    }
                }

                if let Some(plan) = plans.get(&result.plan_id) {
                    let verdict = if result.passed { "passed" } else { "failed" };
                    let body = format!(
                        "### Task `{}`: {}\n\n- Final gate: **{}**\n- Duration: {} ms\n\n```text\n{}\n```",
                        result.task_id,
                        result.task_title,
                        verdict,
                        result.duration_ms,
                        truncate_chars(&result.output, COMMENT_OUTPUT_LIMIT),
                    );
                    if let Err(error) = ops.comment_pr(plan.pr_number, &body).await {
                        warn!(plan_id = %result.plan_id, pr_number = plan.pr_number, %error,
                            "could not post terminal task gate comment (non-fatal)");
                    }
                }
            }
            GitHubCommand::PlanFinished { ops, summary } => {
                let Some(plan) = plans.get(&summary.plan_id) else {
                    continue;
                };
                let outcome = if summary.succeeded {
                    "succeeded"
                } else {
                    "failed"
                };
                let body = format!(
                    "## Roko plan {}\n\n- Tasks passed: {}\n- Tasks failed: {}\n- Duration: {} ms\n- Cost: ${:.4}",
                    outcome,
                    summary.tasks_passed,
                    summary.tasks_failed,
                    summary.duration_ms,
                    summary.cost_usd,
                );
                if let Err(error) = ops.comment_pr(plan.pr_number, &body).await {
                    warn!(plan_id = %summary.plan_id, pr_number = plan.pr_number, %error,
                        "could not post final plan summary (non-fatal)");
                }
            }
            GitHubCommand::MergeRegressionPassed {
                ops,
                plan_id,
                workdir,
                accepted_commit,
            } => {
                let Some(plan) = plans.get(&plan_id).cloned() else {
                    continue;
                };
                check_ci_then_merge(
                    ops,
                    &config,
                    &plan_id,
                    &plan,
                    &workdir,
                    &accepted_commit,
                    ci_policy,
                    Arc::clone(&publisher),
                )
                .await;
            }
            GitHubCommand::Barrier(done) => {
                let _ = done.send(());
            }
        }
    }
}

async fn check_ci_then_merge(
    ops: Arc<dyn GitHubOps>,
    config: &GitHubConfig,
    plan_id: &str,
    plan: &PlanGitHubState,
    workdir: &Path,
    accepted_commit: &str,
    policy: CiRetryPolicy,
    publisher: Arc<dyn BranchPublisher>,
) {
    if let Err(error) = publisher
        .publish(workdir, &plan.branch, accepted_commit)
        .await
    {
        error!(%plan_id, branch = %plan.branch, %accepted_commit, %error,
            "accepted commit publication failed; PR left open");
        let body = format!(
            "## Branch publication failed\n\nRoko could not publish accepted commit `{accepted_commit}` to branch `{}`. CI was not polled and the PR was left open.\n\n```text\n{}\n```",
            plan.branch,
            truncate_chars(&error, 1_000),
        );
        if let Err(comment_error) = ops.comment_pr(plan.pr_number, &body).await {
            warn!(%plan_id, %comment_error, "could not post branch publication failure comment (non-fatal)");
        }
        return;
    }

    let mut ci_check_retries = 0_u8;
    loop {
        match ops.check_ci_status(&plan.branch).await {
            Ok(CiStatus::Success) => {
                if let Err(error) = ops
                    .merge_pr(plan.pr_number, config.merge_method.as_str())
                    .await
                {
                    warn!(%plan_id, pr_number = plan.pr_number, %error,
                        "GitHub PR merge failed after CI passed (non-fatal)");
                }
                return;
            }
            Ok(CiStatus::Failure) => {
                error!(%plan_id, branch = %plan.branch, "GitHub CI failed; PR left open");
                let body = format!(
                    "## CI failed\n\nGitHub CI for branch `{}` failed after local regression passed. The PR was not merged.",
                    plan.branch
                );
                if let Err(error) = ops.comment_pr(plan.pr_number, &body).await {
                    warn!(%plan_id, %error, "could not post CI failure comment (non-fatal)");
                }
                return;
            }
            Ok(CiStatus::Pending) if ci_check_retries < policy.max_retries => {
                ci_check_retries += 1;
                info!(%plan_id, ci_check_retries, max_retries = policy.max_retries,
                    "GitHub CI pending; scheduled re-check");
                tokio::time::sleep(policy.interval).await;
            }
            Ok(CiStatus::Pending) => {
                let body = format!(
                    "## CI still pending\n\nGitHub CI for branch `{}` remained pending after {} retries. The PR was left open.",
                    plan.branch, policy.max_retries
                );
                warn!(%plan_id, retries = policy.max_retries, "GitHub CI retry limit reached; PR left open");
                if let Err(error) = ops.comment_pr(plan.pr_number, &body).await {
                    warn!(%plan_id, %error, "could not post pending-CI comment (non-fatal)");
                }
                return;
            }
            Err(error) => {
                warn!(%plan_id, %error, "could not check GitHub CI; PR left open (non-fatal)");
                return;
            }
        }
    }
}

fn validate_branch(branch: &str) -> Result<(), String> {
    let invalid_char = branch.chars().any(|character| {
        character.is_control()
            || character.is_whitespace()
            || matches!(character, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    });
    if !branch.starts_with("roko/plan/")
        || branch.len() <= "roko/plan/".len()
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch.ends_with('.')
        || branch.contains("..")
        || branch.contains("@{")
        || branch.contains("//")
        || invalid_char
    {
        Err(format!("invalid GitHub plan branch `{branch}`"))
    } else {
        Ok(())
    }
}

fn validate_commit_oid(commit: &str) -> Result<(), String> {
    if matches!(commit.len(), 40 | 64) && commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("accepted commit must be a full hexadecimal Git object ID".to_string())
    }
}

fn truncate_chars(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let truncated = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}\n… (truncated)")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use crate::github_ops::NoOpGitHubOps;
    use async_trait::async_trait;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum GitHubCall {
        CreatePlanBranch {
            plan_id: String,
            base_sha: String,
        },
        OpenPr {
            branch: String,
            draft: bool,
        },
        PublishBranch {
            branch: String,
            accepted_commit: String,
        },
        CheckCi {
            branch: String,
        },
        MergePr {
            number: u64,
            method: String,
        },
        CreateTaskIssue {
            task_id: String,
            labels: Vec<String>,
        },
        CloseIssue {
            number: u64,
        },
        CommentPr {
            number: u64,
            body: String,
        },
    }

    #[derive(Default)]
    struct MockGitHubOps {
        calls: Arc<Mutex<Vec<GitHubCall>>>,
        ci: Mutex<VecDeque<CiStatus>>,
    }

    struct MockBranchPublisher {
        calls: Arc<Mutex<Vec<GitHubCall>>>,
        error: Option<String>,
    }

    #[async_trait]
    impl BranchPublisher for MockBranchPublisher {
        async fn publish(
            &self,
            _workdir: &Path,
            branch: &str,
            accepted_commit: &str,
        ) -> Result<(), String> {
            self.calls.lock().unwrap().push(GitHubCall::PublishBranch {
                branch: branch.into(),
                accepted_commit: accepted_commit.into(),
            });
            self.error.clone().map_or(Ok(()), Err)
        }
    }

    fn publisher(mock: &MockGitHubOps) -> Arc<dyn BranchPublisher> {
        Arc::new(MockBranchPublisher {
            calls: Arc::clone(&mock.calls),
            error: None,
        })
    }

    fn failing_publisher(mock: &MockGitHubOps) -> Arc<dyn BranchPublisher> {
        Arc::new(MockBranchPublisher {
            calls: Arc::clone(&mock.calls),
            error: Some("non-fast-forward".into()),
        })
    }

    impl MockGitHubOps {
        fn with_ci(statuses: impl IntoIterator<Item = CiStatus>) -> Self {
            Self {
                ci: Mutex::new(statuses.into_iter().collect()),
                ..Self::default()
            }
        }

        fn calls(&self) -> Vec<GitHubCall> {
            self.calls.lock().unwrap().clone()
        }

        fn record(&self, call: GitHubCall) {
            self.calls.lock().unwrap().push(call);
        }
    }

    #[async_trait]
    impl GitHubOps for MockGitHubOps {
        async fn create_plan_branch(
            &self,
            plan_id: &str,
            base_sha: &str,
        ) -> Result<String, String> {
            self.record(GitHubCall::CreatePlanBranch {
                plan_id: plan_id.into(),
                base_sha: base_sha.into(),
            });
            Ok(format!("roko/plan/{plan_id}"))
        }

        async fn open_pr(&self, branch: &str, _title: &str, _body: &str) -> Result<u64, String> {
            self.record(GitHubCall::OpenPr {
                branch: branch.into(),
                draft: true,
            });
            Ok(17)
        }

        async fn check_ci_status(&self, ref_name: &str) -> Result<CiStatus, String> {
            self.record(GitHubCall::CheckCi {
                branch: ref_name.into(),
            });
            Ok(self
                .ci
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(CiStatus::Success))
        }

        async fn merge_pr(&self, pr_number: u64, method: &str) -> Result<(), String> {
            self.record(GitHubCall::MergePr {
                number: pr_number,
                method: method.into(),
            });
            Ok(())
        }

        async fn create_task_issue(
            &self,
            task_id: &str,
            _title: &str,
            _body: &str,
            labels: &[String],
        ) -> Result<u64, String> {
            self.record(GitHubCall::CreateTaskIssue {
                task_id: task_id.into(),
                labels: labels.to_vec(),
            });
            Ok(23)
        }

        async fn close_issue(&self, number: u64) -> Result<(), String> {
            self.record(GitHubCall::CloseIssue { number });
            Ok(())
        }

        async fn comment_pr(&self, pr_number: u64, body: &str) -> Result<(), String> {
            self.record(GitHubCall::CommentPr {
                number: pr_number,
                body: body.into(),
            });
            Ok(())
        }
    }

    fn github_config() -> GitHubConfig {
        GitHubConfig {
            owner: Some("acme".into()),
            repo: Some("widget".into()),
            auto_pr: true,
            ..GitHubConfig::default()
        }
    }

    fn task(plan_id: &str, task_id: &str, passed: bool) -> TaskGitHubResult {
        TaskGitHubResult {
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            task_title: format!("Task {task_id}"),
            passed,
            output: "gate output".into(),
            duration_ms: 12,
        }
    }

    const ACCEPTED_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn merge_passed(workflow: &GitHubWorkflow, ops: Arc<dyn GitHubOps>) {
        workflow.merge_regression_passed(
            ops,
            "P1".into(),
            PathBuf::from("/fixture/repo"),
            ACCEPTED_COMMIT.into(),
        );
    }

    #[tokio::test]
    async fn github_two_task_success_sequence_opens_draft_comments_checks_ci_then_merges() {
        let mock = Arc::new(MockGitHubOps::with_ci([CiStatus::Success]));
        let ops: Arc<dyn GitHubOps> = mock.clone();
        let workflow = GitHubWorkflow::start_with_policy(
            github_config(),
            true,
            CiRetryPolicy {
                interval: Duration::ZERO,
                max_retries: 5,
            },
            publisher(&mock),
        );

        workflow.plan_started(ops.clone(), "P1".into(), "abc123".into());
        workflow.task_finished(ops.clone(), task("P1", "T1", true));
        workflow.task_finished(ops.clone(), task("P1", "T2", true));
        workflow.plan_finished(
            ops.clone(),
            PlanGitHubSummary {
                plan_id: "P1".into(),
                succeeded: true,
                tasks_passed: 2,
                tasks_failed: 0,
                duration_ms: 50,
                cost_usd: 0.25,
            },
        );
        // This event models successful GateCompletionKind::Merge handling.
        merge_passed(&workflow, ops);
        workflow.barrier().await;

        let calls = mock.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, GitHubCall::CreatePlanBranch { .. }))
                .count(),
            1
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, GitHubCall::OpenPr { draft: true, .. }))
                .count(),
            1
        );
        assert_eq!(
            calls.iter().filter(|call| matches!(call, GitHubCall::CommentPr { body, .. } if body.starts_with("### Task"))).count(),
            2
        );
        let ci = calls
            .iter()
            .position(|call| matches!(call, GitHubCall::CheckCi { .. }))
            .unwrap();
        let publish = calls
            .iter()
            .position(|call| {
                matches!(
                    call,
                    GitHubCall::PublishBranch {
                        branch,
                        accepted_commit,
                    } if branch == "roko/plan/P1" && accepted_commit == ACCEPTED_COMMIT
                )
            })
            .unwrap();
        let merge = calls
            .iter()
            .position(|call| matches!(call, GitHubCall::MergePr { .. }))
            .unwrap();
        assert!(
            publish < ci,
            "the exact accepted commit must be published before CI is polled"
        );
        assert!(ci < merge, "CI must be checked before the GitHub merge");
    }

    #[tokio::test]
    async fn github_publication_failure_comments_without_ci_or_merge() {
        let mock = Arc::new(MockGitHubOps::with_ci([CiStatus::Success]));
        let ops: Arc<dyn GitHubOps> = mock.clone();
        let workflow = GitHubWorkflow::start_with_policy(
            github_config(),
            true,
            CiRetryPolicy::default(),
            failing_publisher(&mock),
        );
        workflow.plan_started(ops.clone(), "P1".into(), "abc".into());
        merge_passed(&workflow, ops);
        workflow.barrier().await;

        let calls = mock.calls();
        assert!(
            calls
                .iter()
                .any(|call| matches!(call, GitHubCall::PublishBranch { .. }))
        );
        assert!(calls.iter().any(
            |call| matches!(call, GitHubCall::CommentPr { body, .. } if body.contains("publication failed"))
        ));
        assert!(!calls.iter().any(|call| matches!(
            call,
            GitHubCall::CheckCi { .. } | GitHubCall::MergePr { .. }
        )));
    }

    #[tokio::test]
    async fn github_terminal_failure_creates_issue_and_later_success_closes_it() {
        let mock = Arc::new(MockGitHubOps::default());
        let ops: Arc<dyn GitHubOps> = mock.clone();
        let workflow = GitHubWorkflow::start_with_policy(
            github_config(),
            true,
            CiRetryPolicy::default(),
            publisher(&mock),
        );

        workflow.task_finished(ops.clone(), task("P1", "T1", false));
        workflow.task_finished(ops, task("P1", "T1", true));
        workflow.barrier().await;

        let calls = mock.calls();
        assert!(calls.iter().any(
            |call| matches!(call, GitHubCall::CreateTaskIssue { task_id, .. } if task_id == "T1")
        ));
        assert!(
            calls
                .iter()
                .any(|call| matches!(call, GitHubCall::CloseIssue { number: 23 }))
        );
    }

    #[tokio::test]
    async fn github_pending_retries_without_merge_then_success_merges() {
        let mock = Arc::new(MockGitHubOps::with_ci([
            CiStatus::Pending,
            CiStatus::Pending,
            CiStatus::Success,
        ]));
        let ops: Arc<dyn GitHubOps> = mock.clone();
        let workflow = GitHubWorkflow::start_with_policy(
            github_config(),
            true,
            CiRetryPolicy {
                interval: Duration::ZERO,
                max_retries: 5,
            },
            publisher(&mock),
        );
        workflow.plan_started(ops.clone(), "P1".into(), "abc".into());
        merge_passed(&workflow, ops);
        workflow.barrier().await;

        let calls = mock.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, GitHubCall::CheckCi { .. }))
                .count(),
            3
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, GitHubCall::MergePr { .. }))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn github_failed_ci_comments_and_never_merges() {
        let mock = Arc::new(MockGitHubOps::with_ci([CiStatus::Failure]));
        let ops: Arc<dyn GitHubOps> = mock.clone();
        let workflow = GitHubWorkflow::start_with_policy(
            github_config(),
            true,
            CiRetryPolicy::default(),
            publisher(&mock),
        );
        workflow.plan_started(ops.clone(), "P1".into(), "abc".into());
        merge_passed(&workflow, ops);
        workflow.barrier().await;

        let calls = mock.calls();
        assert!(calls.iter().any(
            |call| matches!(call, GitHubCall::CommentPr { body, .. } if body.contains("CI failed"))
        ));
        assert!(
            !calls
                .iter()
                .any(|call| matches!(call, GitHubCall::MergePr { .. }))
        );
    }

    #[tokio::test]
    async fn github_pending_ci_stops_after_five_retries_without_merging() {
        let mock = Arc::new(MockGitHubOps::with_ci([CiStatus::Pending; 6]));
        let ops: Arc<dyn GitHubOps> = mock.clone();
        let workflow = GitHubWorkflow::start_with_policy(
            github_config(),
            true,
            CiRetryPolicy {
                interval: Duration::ZERO,
                max_retries: 5,
            },
            publisher(&mock),
        );
        workflow.plan_started(ops.clone(), "P1".into(), "abc".into());
        merge_passed(&workflow, ops);
        workflow.barrier().await;

        let calls = mock.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, GitHubCall::CheckCi { .. }))
                .count(),
            6,
            "the initial check plus at most five scheduled retries must run"
        );
        assert!(calls.iter().any(
            |call| matches!(call, GitHubCall::CommentPr { body, .. } if body.contains("still pending"))
        ));
        assert!(
            !calls
                .iter()
                .any(|call| matches!(call, GitHubCall::MergePr { .. }))
        );
    }

    #[tokio::test]
    async fn github_unconfigured_noop_has_no_external_workflow_calls() {
        let workflow = GitHubWorkflow::start(GitHubConfig::default(), false);
        let ops: Arc<dyn GitHubOps> = Arc::new(NoOpGitHubOps);
        workflow.plan_started(ops.clone(), "P1".into(), "abc".into());
        workflow.task_finished(ops.clone(), task("P1", "T1", false));
        workflow.merge_regression_passed(
            ops,
            "P1".into(),
            PathBuf::from("/fixture/repo"),
            ACCEPTED_COMMIT.into(),
        );
        workflow.barrier().await;
        // The unconfigured worker discarded every command; NoOp has no side effects.
    }

    #[tokio::test]
    async fn github_live_construction_failure_cannot_create_fake_pr_state() {
        let mock = Arc::new(MockGitHubOps::default());
        let ops: Arc<dyn GitHubOps> = mock.clone();
        // Runtime passes enabled=false when LiveGitHubOps construction fails,
        // for example because GITHUB_TOKEN is absent.
        let workflow = GitHubWorkflow::start(github_config(), false);
        workflow.plan_started(ops.clone(), "P1".into(), "abc".into());
        workflow.task_finished(ops.clone(), task("P1", "T1", true));
        workflow.merge_regression_passed(
            ops,
            "P1".into(),
            PathBuf::from("/fixture/repo"),
            ACCEPTED_COMMIT.into(),
        );
        workflow.barrier().await;

        assert!(
            mock.calls().is_empty(),
            "disabled fallback must not retain a fake PR #0"
        );
    }

    #[test]
    fn github_comment_output_is_unicode_safe_and_bounded() {
        let output = "🦀".repeat(COMMENT_OUTPUT_LIMIT + 1);
        let truncated = truncate_chars(&output, COMMENT_OUTPUT_LIMIT);
        assert!(truncated.ends_with("… (truncated)"));
        assert_eq!(truncated.matches('🦀').count(), COMMENT_OUTPUT_LIMIT);
    }

    #[test]
    fn github_publication_ref_validation_rejects_unsafe_inputs() {
        assert!(validate_branch("roko/plan/E46-safe").is_ok());
        assert!(validate_branch("roko/plan/../../main").is_err());
        assert!(validate_branch("main").is_err());
        assert!(validate_commit_oid(ACCEPTED_COMMIT).is_ok());
        assert!(validate_commit_oid("HEAD").is_err());
    }
}
