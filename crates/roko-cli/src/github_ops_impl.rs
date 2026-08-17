//! Live [`GitHubOps`] adapter backed by the blocking GitHub API client.

use std::sync::Arc;

use crate::github_ops::{CiStatus, GitHubOps};
use async_trait::async_trait;
use roko_core::config::GitHubConfig;
use roko_mcp_github::GitHubClient;

/// Production GitHub operations for one configured repository.
#[derive(Debug)]
pub struct LiveGitHubOps {
    client: Arc<GitHubClient>,
    owner: String,
    repo: String,
    config: GitHubConfig,
}

impl LiveGitHubOps {
    /// Construct the live adapter from `[github]` and `GITHUB_TOKEN`.
    pub fn from_config(config: &GitHubConfig) -> Result<Self, String> {
        let owner = config
            .owner
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "github.owner is not configured".to_string())?;
        let repo = config
            .repo
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "github.repo is not configured".to_string())?;
        let client = GitHubClient::from_env().map_err(|error| error.to_string())?;
        Ok(Self {
            client: Arc::new(client),
            owner,
            repo,
            config: config.clone(),
        })
    }

    async fn blocking<T, F>(operation: &'static str, call: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, String> + Send + 'static,
    {
        tokio::task::spawn_blocking(call)
            .await
            .map_err(|error| format!("{operation} worker failed: {error}"))?
    }
}

#[async_trait]
impl GitHubOps for LiveGitHubOps {
    async fn create_plan_branch(&self, plan_id: &str, base_sha: &str) -> Result<String, String> {
        let branch = format!("roko/plan/{plan_id}");
        let client = Arc::clone(&self.client);
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let branch_for_call = branch.clone();
        let base_sha = base_sha.to_string();
        Self::blocking("create GitHub plan branch", move || {
            client
                .create_branch(&owner, &repo, &branch_for_call, &base_sha)
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
        .await?;
        Ok(branch)
    }

    async fn open_pr(&self, branch: &str, title: &str, body: &str) -> Result<u64, String> {
        let client = Arc::clone(&self.client);
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let base = self.config.default_branch.clone();
        let branch = branch.to_string();
        let title = title.to_string();
        let body = body.to_string();
        Self::blocking("open GitHub pull request", move || {
            // Runner-created plan pull requests intentionally start as drafts.
            client
                .create_pr(&owner, &repo, &title, &body, &branch, &base, true)
                .map(|(number, _)| number)
                .map_err(|error| error.to_string())
        })
        .await
    }

    async fn check_ci_status(&self, ref_name: &str) -> Result<CiStatus, String> {
        let client = Arc::clone(&self.client);
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let ref_name = ref_name.to_string();
        Self::blocking("check GitHub CI status", move || {
            let response = client
                .get_actions_status(&owner, &repo, &ref_name)
                .map_err(|error| error.to_string())?;
            match response.get("state").and_then(serde_json::Value::as_str) {
                Some("success") => Ok(CiStatus::Success),
                Some("failure" | "error") => Ok(CiStatus::Failure),
                Some("pending" | "expected") | None => Ok(CiStatus::Pending),
                Some(other) => Err(format!("unrecognized GitHub CI state `{other}`")),
            }
        })
        .await
    }

    async fn merge_pr(&self, pr_number: u64, method: &str) -> Result<(), String> {
        let client = Arc::clone(&self.client);
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let configured_method = self.config.merge_method.as_str().to_string();
        let method = if method.trim().is_empty() {
            configured_method
        } else {
            method.to_string()
        };
        Self::blocking("merge GitHub pull request", move || {
            client
                .merge_pr(&owner, &repo, pr_number, &method)
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
        .await
    }

    async fn create_task_issue(
        &self,
        _task_id: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<u64, String> {
        let client = Arc::clone(&self.client);
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let title = title.to_string();
        let body = body.to_string();
        let labels = labels.to_vec();
        Self::blocking("create GitHub task issue", move || {
            client
                .create_issue(&owner, &repo, &title, &body, &labels)
                .map(|(number, _)| number)
                .map_err(|error| error.to_string())
        })
        .await
    }

    async fn close_issue(&self, number: u64) -> Result<(), String> {
        let client = Arc::clone(&self.client);
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        Self::blocking("close GitHub task issue", move || {
            client
                .close_issue(&owner, &repo, number)
                .map_err(|error| error.to_string())
        })
        .await
    }

    async fn comment_pr(&self, pr_number: u64, body: &str) -> Result<(), String> {
        let client = Arc::clone(&self.client);
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let body = body.to_string();
        Self::blocking("comment on GitHub pull request", move || {
            client
                .comment_pr(&owner, &repo, pr_number, &body)
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_live_config_requires_repository_coordinates_before_token() {
        let error = LiveGitHubOps::from_config(&GitHubConfig::default()).unwrap_err();
        assert_eq!(error, "github.owner is not configured");
    }
}
