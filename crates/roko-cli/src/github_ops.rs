//! Trait-based GitHub operations owned by the live CLI runner.
//!
//! [`GitHubOps`] keeps the runner testable without a live GitHub connection.
//! [`NoOpGitHubOps`] is the default when GitHub integration is not configured;
//! [`crate::github_ops_impl::LiveGitHubOps`] provides the production adapter.

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

/// Narrow, object-safe interface for runner-owned GitHub workflow operations.
#[async_trait]
pub trait GitHubOps: Send + Sync {
    /// Create the plan branch from `base_sha` and return its full name.
    async fn create_plan_branch(&self, plan_id: &str, base_sha: &str) -> Result<String, String>;

    /// Open a pull request and return its number.
    async fn open_pr(&self, branch: &str, title: &str, body: &str) -> Result<u64, String>;

    /// Return the combined CI status for a branch or commit.
    async fn check_ci_status(&self, ref_name: &str) -> Result<CiStatus, String>;

    /// Merge the pull request using the configured method.
    async fn merge_pr(&self, pr_number: u64, method: &str) -> Result<(), String>;

    /// Create a terminal task-failure issue and return its number.
    async fn create_task_issue(
        &self,
        task_id: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<u64, String>;

    /// Close an issue.
    async fn close_issue(&self, number: u64) -> Result<(), String>;

    /// Comment on a pull request.
    async fn comment_pr(&self, pr_number: u64, body: &str) -> Result<(), String>;
}

/// No-op GitHub adapter used when integration is unconfigured and by tests.
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
    async fn no_op_adapter_is_object_safe_and_deterministic() {
        let ops: Arc<dyn GitHubOps> = Arc::new(NoOpGitHubOps);
        assert_eq!(
            ops.create_plan_branch("my-plan", "abc123")
                .await
                .expect("create branch"),
            "roko/plan/my-plan"
        );
        assert_eq!(
            ops.check_ci_status("main").await.expect("check CI"),
            CiStatus::Success
        );
        assert_eq!(ops.open_pr("branch", "title", "body").await, Ok(0));
        assert_eq!(
            ops.create_task_issue("T01", "failed", "body", &[]).await,
            Ok(0)
        );
        assert!(ops.merge_pr(0, "squash").await.is_ok());
        assert!(ops.close_issue(0).await.is_ok());
        assert!(ops.comment_pr(0, "comment").await.is_ok());
    }
}
