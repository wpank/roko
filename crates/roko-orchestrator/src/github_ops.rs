//! Trait-based interface for GitHub operations used by the runner.
//!
//! [`GitHubOps`] abstracts over GitHub API calls so the runner can be tested
//! without a real GitHub connection. [`NoOpGitHubOps`] is the default
//! implementation — all methods succeed silently. `LiveGitHubOps` (in
//! `roko-cli`) bridges to the real API client.

use async_trait::async_trait;

/// CI check status returned by [`GitHubOps::check_ci_status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiStatus {
    /// Checks are still running.
    Pending,
    /// All checks passed.
    Success,
    /// One or more checks failed.
    Failure,
}

/// Async trait for runner-level GitHub operations.
///
/// All methods are object-safe (`dyn GitHubOps` works) and return
/// `Result<_, String>` so implementations can surface API errors without
/// pulling in any particular error crate.
///
/// The trait is intentionally narrow — it covers exactly what the plan
/// runner needs to integrate with GitHub and nothing more. Broad GitHub
/// API access belongs in the MCP server (`roko-mcp-github`).
#[async_trait]
pub trait GitHubOps: Send + Sync {
    /// Create a branch named `"roko/plan/{plan_id}"` from `base_sha`.
    ///
    /// Returns the full branch name on success.
    async fn create_plan_branch(&self, plan_id: &str, base_sha: &str) -> Result<String, String>;

    /// Open a pull request from `branch` onto the default base branch.
    ///
    /// Returns the PR number on success.
    async fn open_pr(&self, branch: &str, title: &str, body: &str) -> Result<u64, String>;

    /// Return the combined CI status for the given ref (branch name or SHA).
    async fn check_ci_status(&self, ref_name: &str) -> Result<CiStatus, String>;

    /// Merge pull request `pr_number` using the configured merge method.
    async fn merge_pr(&self, pr_number: u64, method: &str) -> Result<(), String>;

    /// Create a GitHub issue to track a terminal task failure.
    ///
    /// Returns the issue number on success.
    async fn create_task_issue(
        &self,
        task_id: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<u64, String>;

    /// Close a previously opened issue.
    async fn close_issue(&self, number: u64) -> Result<(), String>;

    /// Post a comment on a pull request.
    async fn comment_pr(&self, pr_number: u64, body: &str) -> Result<(), String>;
}

/// No-op implementation of [`GitHubOps`].
///
/// All methods return `Ok` with dummy values. Used when:
/// - `[github]` config is missing `owner`/`repo`, or
/// - Tests do not need real GitHub side-effects.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpGitHubOps;

#[async_trait]
impl GitHubOps for NoOpGitHubOps {
    async fn create_plan_branch(&self, plan_id: &str, _base_sha: &str) -> Result<String, String> {
        Ok(format!("roko/plan/{plan_id}"))
    }

    async fn open_pr(&self, _branch: &str, _title: &str, _body: &str) -> Result<u64, String> {
        Ok(0)
    }

    async fn check_ci_status(&self, _ref_name: &str) -> Result<CiStatus, String> {
        Ok(CiStatus::Success)
    }

    async fn merge_pr(&self, _pr_number: u64, _method: &str) -> Result<(), String> {
        Ok(())
    }

    async fn create_task_issue(
        &self,
        _task_id: &str,
        _title: &str,
        _body: &str,
        _labels: &[String],
    ) -> Result<u64, String> {
        Ok(0)
    }

    async fn close_issue(&self, _number: u64) -> Result<(), String> {
        Ok(())
    }

    async fn comment_pr(&self, _pr_number: u64, _body: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn noop_create_plan_branch_returns_formatted_name() {
        let ops = NoOpGitHubOps;
        let branch = ops.create_plan_branch("my-plan", "abc123").await.unwrap();
        assert_eq!(branch, "roko/plan/my-plan");
    }

    #[tokio::test]
    async fn noop_open_pr_returns_zero() {
        let ops = NoOpGitHubOps;
        let pr = ops.open_pr("roko/plan/x", "Test PR", "body").await.unwrap();
        assert_eq!(pr, 0);
    }

    #[tokio::test]
    async fn noop_check_ci_status_returns_success() {
        let ops = NoOpGitHubOps;
        let status = ops.check_ci_status("main").await.unwrap();
        assert_eq!(status, CiStatus::Success);
    }

    #[tokio::test]
    async fn noop_merge_pr_returns_ok() {
        let ops = NoOpGitHubOps;
        ops.merge_pr(42, "squash").await.unwrap();
    }

    #[tokio::test]
    async fn noop_create_task_issue_returns_zero() {
        let ops = NoOpGitHubOps;
        let issue = ops
            .create_task_issue("T01", "task failed", "details", &[])
            .await
            .unwrap();
        assert_eq!(issue, 0);
    }

    #[tokio::test]
    async fn noop_close_issue_returns_ok() {
        let ops = NoOpGitHubOps;
        ops.close_issue(1).await.unwrap();
    }

    #[tokio::test]
    async fn noop_comment_pr_returns_ok() {
        let ops = NoOpGitHubOps;
        ops.comment_pr(7, "nice work").await.unwrap();
    }

    #[tokio::test]
    async fn noop_is_object_safe_via_arc() {
        let ops: Arc<dyn GitHubOps> = Arc::new(NoOpGitHubOps);
        ops.merge_pr(0, "squash").await.unwrap();
    }
}
