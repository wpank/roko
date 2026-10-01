//! Cloud execution helpers for `code-implementer`.
//!
//! This module owns the ephemeral clone / branch / commit / push / PR flow
//! used by the deployed code-implementer worker.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Output;

use anyhow::{Context, Result, anyhow, bail};
use roko_agent::mcp::{McpClient, StdioTransport};
use roko_agent::process::apply_credential_scrub;
use roko_core::child_env::CredentialScrub;
use serde::Deserialize;
use serde_json::json;

use crate::serve::deploy::CloudExecutionConfig;
use crate::workspace_paths::relative_plans_dir;

/// Cloud execution parameters for a single plan run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudExecution {
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repo: String,
    /// GitHub token used for clone/push and MCP auth.
    pub github_token: String,
    /// Plan slug used for branch naming.
    pub plan_slug: String,
    /// Base branch to target when opening the implementation PR.
    pub base_branch: String,
    /// Human-readable implementation PR title.
    pub pr_title: String,
    /// Human-readable implementation PR body.
    pub pr_body: String,
    /// Command used to spawn the GitHub MCP server.
    pub github_mcp_command: String,
    /// Extra args passed to the GitHub MCP server command.
    pub github_mcp_args: Vec<String>,
}

impl CloudExecution {
    /// Branch name used for implementation work.
    #[must_use]
    pub fn branch_name(&self) -> String {
        format!("impl/{}", self.plan_slug)
    }

    /// Public repository URL without embedded credentials.
    #[must_use]
    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.repo)
    }

    /// Build the clone URL with embedded token authentication.
    #[must_use]
    pub fn clone_url(&self) -> String {
        rewrite_git_https_url(&self.repo_url(), &self.github_token)
    }

    /// Build the HTTPS push URL with embedded token authentication.
    #[must_use]
    pub fn push_url(&self) -> String {
        rewrite_git_https_url(&self.repo_url(), &self.github_token)
    }
}

/// Parse cloud execution params from a template invocation.
#[derive(Debug, Deserialize)]
pub struct CloudExecutionParams {
    /// Repository owner.
    pub github_owner: String,
    /// Repository name.
    pub github_repo: String,
    /// GitHub token.
    pub github_token: String,
    /// Plan slug / branch slug.
    pub plan_slug: String,
    /// Optional base branch. Defaults to `main`.
    #[serde(default)]
    pub base_branch: Option<String>,
    /// Optional PR title override.
    #[serde(default)]
    pub pr_title: Option<String>,
    /// Optional PR body override.
    #[serde(default)]
    pub pr_body: Option<String>,
    /// Optional plan directory relative to the cloned workspace.
    #[serde(default)]
    pub plan_dir: Option<PathBuf>,
    /// Optional workspace root directory. Defaults to `/tmp/roko-workspace`.
    #[serde(default)]
    pub workspace_dir: Option<PathBuf>,
    /// Optional GitHub MCP command override.
    #[serde(default)]
    pub github_mcp_command: Option<String>,
    /// Optional GitHub MCP args override.
    #[serde(default)]
    pub github_mcp_args: Option<Vec<String>>,
}

impl CloudExecutionParams {
    /// Parse execution parameters from a flat string map.
    pub fn from_map(params: &HashMap<String, String>) -> Result<Self> {
        let (github_owner, github_repo) = match (
            params.get("github_owner").cloned(),
            params.get("github_repo").cloned(),
        ) {
            (Some(owner), Some(repo)) => (owner, repo),
            _ => parse_owner_repo(params.get("repo_url").ok_or_else(|| {
                anyhow!("missing required params: github_owner/github_repo or repo_url")
            })?)?,
        };

        Ok(Self {
            github_owner,
            github_repo,
            github_token: params
                .get("github_token")
                .cloned()
                .or_else(|| std::env::var("GITHUB_TOKEN").ok())
                .ok_or_else(|| anyhow!("missing required param: github_token"))?,
            plan_slug: params
                .get("plan_slug")
                .cloned()
                .ok_or_else(|| anyhow!("missing required param: plan_slug"))?,
            base_branch: params.get("base_branch").cloned(),
            pr_title: params.get("pr_title").cloned(),
            pr_body: params.get("pr_body").cloned(),
            plan_dir: params.get("plan_dir").cloned().map(PathBuf::from),
            workspace_dir: params.get("workspace_dir").cloned().map(PathBuf::from),
            github_mcp_command: params.get("github_mcp_command").cloned(),
            github_mcp_args: params.get("github_mcp_args").map(|args| {
                args.split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            }),
        })
    }

    /// Convert parsed params into a concrete execution configuration.
    #[must_use]
    pub fn into_execution(&self) -> CloudExecution {
        let branch = format!("impl/{}", self.plan_slug);
        CloudExecution {
            owner: self.github_owner.clone(),
            repo: self.github_repo.clone(),
            github_token: self.github_token.clone(),
            plan_slug: self.plan_slug.clone(),
            base_branch: self
                .base_branch
                .clone()
                .unwrap_or_else(|| "main".to_string()),
            pr_title: self
                .pr_title
                .clone()
                .unwrap_or_else(|| format!("Implement {}", self.plan_slug)),
            pr_body: self.pr_body.clone().unwrap_or_else(|| {
                format!(
                    "Automated implementation for plan `{}`.\n\nBranch: `{}`",
                    self.plan_slug, branch
                )
            }),
            github_mcp_command: self
                .github_mcp_command
                .clone()
                .unwrap_or_else(|| "roko-mcp-github".to_string()),
            github_mcp_args: self.github_mcp_args.clone().unwrap_or_default(),
        }
    }

    /// Plan directory relative to the cloned workspace.
    #[must_use]
    pub fn plan_dir(&self) -> PathBuf {
        self.plan_dir
            .clone()
            .unwrap_or_else(|| relative_plans_dir().join(&self.plan_slug))
    }

    /// Resolve the workspace root using the configured default when absent.
    #[must_use]
    pub fn workspace_dir(&self) -> PathBuf {
        self.workspace_dir
            .clone()
            .unwrap_or_else(|| CloudExecutionConfig::default().workspace_dir)
    }
}

fn parse_owner_repo(repo_url: &str) -> Result<(String, String)> {
    let trimmed = repo_url.trim().trim_end_matches(".git");
    if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        let mut parts = rest.splitn(2, '/');
        let owner = parts.next().unwrap_or_default();
        let repo = parts.next().unwrap_or_default();
        if !owner.is_empty() && !repo.is_empty() {
            return Ok((owner.to_string(), repo.to_string()));
        }
    }
    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        let mut parts = rest.splitn(2, '/');
        let owner = parts.next().unwrap_or_default();
        let repo = parts.next().unwrap_or_default();
        if !owner.is_empty() && !repo.is_empty() {
            return Ok((owner.to_string(), repo.to_string()));
        }
    }
    bail!("unable to parse GitHub owner/repo from repo_url: {repo_url}");
}

fn rewrite_git_https_url(url: &str, token: &str) -> String {
    if token.is_empty() {
        return url.to_string();
    }

    if let Some(rest) = url.strip_prefix("https://") {
        let path = rest.split_once('@').map_or(rest, |(_, after_at)| after_at);
        return format!("https://x-access-token:{token}@{path}");
    }

    url.to_string()
}

fn scrub_token(text: &str, token: &str) -> String {
    if token.is_empty() {
        return text.to_string();
    }

    text.replace(token, "***")
}

fn git_error(action: &str, output: &Output, token: Option<&str>) -> anyhow::Error {
    let mut message = String::new();
    if !output.stderr.is_empty() {
        message.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    if !output.stdout.is_empty() {
        if !message.is_empty() {
            message.push_str("\n");
        }
        message.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if message.trim().is_empty() {
        message = format!("git exited with {}", output.status);
    }

    let message = match token {
        Some(token) => scrub_token(&message, token),
        None => message,
    };
    anyhow!("{action} failed: {message}")
}

/// Clone the repository into the target workspace.
pub async fn git_clone(url: &str, workspace: &Path, token: &str) -> Result<()> {
    if let Some(parent) = workspace.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create {}", parent.display()))?;
    }

    let url = rewrite_git_https_url(url, token);
    let output = tokio::process::Command::new("git")
        .args(["clone", "--depth", "1", &url])
        .arg(workspace)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .context("spawn git clone")?;

    if !output.status.success() {
        return Err(git_error("git clone", &output, Some(token)));
    }

    let reset_output = tokio::process::Command::new("git")
        .args(["remote", "set-url", "origin", &url])
        .current_dir(workspace)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .context("spawn git remote set-url origin")?;

    if !reset_output.status.success() {
        return Err(git_error("git remote set-url origin", &reset_output, None));
    }

    Ok(())
}

/// Branches that must never be pushed to directly from a cloud worker.
const PROTECTED_BRANCHES: &[&str] = &["main", "master", "develop", "release", "production"];

/// Validate that a branch name is safe for cloud worker use.
///
/// Rejects protected branch names and names containing shell-unsafe characters.
/// Cloud workers must always work on `impl/` prefixed branches (#373).
fn validate_worker_branch(branch: &str) -> Result<()> {
    if branch.is_empty() {
        bail!("branch name must not be empty");
    }
    if PROTECTED_BRANCHES.contains(&branch) {
        bail!(
            "refusing to use protected branch '{branch}'; \
             cloud workers must use an impl/ prefixed branch"
        );
    }
    // Reject shell-unsafe characters that could be injected through params.
    if branch
        .chars()
        .any(|c| c.is_control() || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
    {
        bail!("branch name '{branch}' contains unsafe characters");
    }
    Ok(())
}

/// Create and switch to the implementation branch.
///
/// Validates the branch name before creating it (#373).
pub async fn git_checkout_new_branch(workspace: &Path, branch: &str) -> Result<()> {
    validate_worker_branch(branch)?;

    let output = tokio::process::Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(workspace)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .context("spawn git checkout -b")?;

    if !output.status.success() {
        return Err(git_error("git checkout -b", &output, None));
    }

    Ok(())
}

/// Path prefixes that must never be staged by a cloud worker (#373).
///
/// Each entry is checked against every discovered changed path. An entry
/// matches when the path equals it exactly or starts with it (directory
/// prefix). Basename matching is used for dot/secret filenames so that
/// `subdir/.env` is also caught.
const EXCLUDED_PATTERNS: &[&str] = &[".env", "credentials.json", "secrets.json", ".roko/"];

/// Returns true if `path` matches any of the [`EXCLUDED_PATTERNS`].
fn is_excluded(path: &str) -> bool {
    EXCLUDED_PATTERNS.iter().any(|pat| {
        // Exact basename match (e.g. ".env", "credentials.json") or
        // prefix match for directory patterns (e.g. ".roko/").
        let basename = path.rsplit('/').next().unwrap_or(path);
        basename == *pat || path.starts_with(pat)
    })
}

/// A `git` command in `workspace` that never prompts.
///
/// The agent could write `.git/hooks` in the workspace, and roko's commit and
/// push run those hooks, so the command inherits roko's environment minus
/// provider keys, `.env`-loaded names and roko's own credentials
/// ([`CredentialScrub`]). The hooks still run; signing and SSH still work.
fn git_command(workspace: &Path) -> tokio::process::Command {
    let mut git = tokio::process::Command::new("git");
    git.current_dir(workspace).env("GIT_TERMINAL_PROMPT", "0");
    apply_credential_scrub(&mut git, &CredentialScrub::default());
    git
}

/// Stage and commit the current workspace state.
///
/// Queries `git diff` for exact changed pathspecs and stages only those,
/// skipping any paths that match [`EXCLUDED_PATTERNS`] (#373).
pub async fn git_commit(workspace: &Path, message: &str) -> Result<()> {
    // Collect modified/deleted tracked files.
    let tracked_output = git_command(workspace)
        .args(["diff", "--name-only", "HEAD"])
        .output()
        .await
        .context("spawn git diff --name-only HEAD")?;

    if !tracked_output.status.success() {
        return Err(git_error(
            "git diff --name-only HEAD",
            &tracked_output,
            None,
        ));
    }

    // Collect untracked files (new files not yet known to git).
    let untracked_output = git_command(workspace)
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()
        .await
        .context("spawn git ls-files --others")?;

    if !untracked_output.status.success() {
        return Err(git_error("git ls-files --others", &untracked_output, None));
    }

    let tracked_paths = String::from_utf8_lossy(&tracked_output.stdout);
    let untracked_paths = String::from_utf8_lossy(&untracked_output.stdout);

    let paths: Vec<&str> = tracked_paths
        .lines()
        .chain(untracked_paths.lines())
        .map(str::trim)
        .filter(|p| !p.is_empty() && !is_excluded(p))
        .collect();

    if paths.is_empty() {
        bail!("nothing to commit (working tree clean)");
    }

    // Stage only the exact pathspecs.
    let mut add_args = vec!["add", "--"];
    add_args.extend(paths);

    let add_output = git_command(workspace)
        .args(&add_args)
        .output()
        .await
        .context("spawn git add -- <paths>")?;

    if !add_output.status.success() {
        return Err(git_error("git add -- <paths>", &add_output, None));
    }

    let diff_output = git_command(workspace)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .await
        .context("spawn git diff --cached")?;

    if diff_output.status.success() {
        bail!("nothing to commit (working tree clean)");
    }

    let output = git_command(workspace)
        .args(["commit", "-m", message])
        .env("GIT_AUTHOR_NAME", "roko")
        .env("GIT_AUTHOR_EMAIL", "roko@nunchi.dev")
        .env("GIT_COMMITTER_NAME", "roko")
        .env("GIT_COMMITTER_EMAIL", "roko@nunchi.dev")
        .output()
        .await
        .context("spawn git commit")?;

    if !output.status.success() {
        return Err(git_error("git commit", &output, None));
    }

    Ok(())
}

/// Push the implementation branch to origin.
///
/// Validates the branch name before pushing (#373). Protected branches
/// (main, master, etc.) are rejected to prevent accidental force-push
/// from cloud workers.
pub async fn git_push(workspace: &Path, branch: &str, token: &str) -> Result<()> {
    validate_worker_branch(branch)?;
    let origin_output = git_command(workspace)
        .args(["remote", "get-url", "origin"])
        .output()
        .await
        .context("spawn git remote get-url origin")?;

    if !origin_output.status.success() {
        return Err(git_error("git remote get-url origin", &origin_output, None));
    }

    let origin = String::from_utf8_lossy(&origin_output.stdout)
        .trim()
        .to_string();
    let push_url = rewrite_git_https_url(&origin, token);
    let output = git_command(workspace)
        .args(["push", &push_url, branch])
        .output()
        .await
        .context("spawn git push")?;

    if !output.status.success() {
        return Err(git_error("git push", &output, Some(token)));
    }

    Ok(())
}

/// Remove the workspace directory after execution.
pub async fn git_cleanup(workspace: &Path) -> Result<()> {
    match tokio::fs::remove_dir_all(workspace).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("remove {}", workspace.display())),
    }
}

/// Call the GitHub MCP `github.create_pr` tool.
pub async fn github_create_pr(workspace: &Path, execution: &CloudExecution) -> Result<String> {
    let transport =
        StdioTransport::spawn(&execution.github_mcp_command, &execution.github_mcp_args)
            .map_err(|err| anyhow!("spawn GitHub MCP server: {err}"))?;
    let client = McpClient::new(transport);
    client
        .initialize()
        .await
        .map_err(|err| anyhow!("initialize GitHub MCP server: {err}"))?;

    let response = client
        .call_tool(
            "github.create_pr",
            json!({
                "owner": execution.owner,
                "repo": execution.repo,
                "title": execution.pr_title,
                "body": execution.pr_body,
                "head": execution.branch_name(),
                "base": execution.base_branch,
            }),
        )
        .await
        .map_err(|err| anyhow!("github.create_pr call failed: {err}"))?;

    let text = response
        .content
        .iter()
        .find_map(|content| content.text.clone())
        .ok_or_else(|| anyhow!("github.create_pr returned no text content"))?;

    tokio::fs::write(
        workspace.join(".roko").join("implementation-pr.json"),
        &text,
    )
    .await
    .ok();

    Ok(text)
}

/// Execute the cloud code-implementer flow end-to-end.
pub async fn run_code_implementer_cloud(
    params: &HashMap<String, String>,
) -> Result<super::handler::TaskResult> {
    let started = std::time::Instant::now();
    let request = CloudExecutionParams::from_map(params)?;
    let execution = request.into_execution();
    let workspace_root = request.workspace_dir();
    let workspace = workspace_root.join(&execution.repo);
    let plan_dir = workspace.join(request.plan_dir());

    if workspace.exists() {
        git_cleanup(&workspace).await.ok();
    }

    let result = async {
        git_clone(&execution.repo_url(), &workspace, &execution.github_token).await?;
        git_checkout_new_branch(&workspace, &execution.branch_name()).await?;

        // Gate verdicts come from the run's own hub, tapped before it starts.
        let state_hub = crate::state_hub::shared_state_hub();
        let gates = crate::graph_execution::EventTap::spawn(
            &state_hub,
            Vec::new(),
            |verdicts: &mut Vec<(String, bool)>, tapped| {
                if let crate::graph_execution::Tapped::Event(envelope) = tapped
                    && let roko_core::DashboardEvent::GateResult { gate, passed, .. } =
                        &envelope.payload
                {
                    verdicts.push((gate.clone(), *passed));
                }
            },
        );
        let exit_code =
            crate::graph_execution::run_graph_plan(crate::graph_execution::GraphPlanRunParams {
                plans_dir: plan_dir.clone(),
                workdir: workspace.clone(),
                quiet: false,
                json: false,
                resume_plan: None,
                fresh: false,
                force_resume: false,
                max_retries: None,
                max_tasks: 0,
                budget_override: None,
                no_budget: false,
                cli_model_override: None,
                dangerously_skip_permissions: false,
                log_file: None,
                worktree_per_task: false,
                rich_topology: false,
                promote: None,
                no_tui: true,
                state_hub: Some(state_hub),
                interrupt: None,
                max_parallel_plans: None,
                fail_fast: false,
                only_plans: None,
                live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
                force_disk_check: false,
                effort: None,
                no_cascade: false,
                metrics: None,
            })
            .await?;
        let success = exit_code == crate::exit_codes::EXIT_SUCCESS;
        let gate_verdicts = gates.finish().await?;

        if success {
            let commit_message = format!("plan: {}", execution.plan_slug);
            git_commit(&workspace, &commit_message).await?;
            git_push(
                &workspace,
                &execution.branch_name(),
                &execution.github_token,
            )
            .await?;
            github_create_pr(&workspace, &execution).await?;
        }

        Ok(super::handler::TaskResult {
            success,
            episode_id: None,
            gate_verdicts,
            error: None,
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        })
    }
    .await;

    git_cleanup(&workspace).await.ok();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_execution_branch_name_is_prefixed() {
        let execution = CloudExecution {
            owner: "nunchi".into(),
            repo: "roko".into(),
            github_token: "token".into(),
            plan_slug: "p07-autofix".into(),
            base_branch: "main".into(),
            pr_title: "title".into(),
            pr_body: "body".into(),
            github_mcp_command: "roko-mcp-github".into(),
            github_mcp_args: Vec::new(),
        };
        assert_eq!(execution.branch_name(), "impl/p07-autofix");
        assert!(
            execution
                .clone_url()
                .contains("x-access-token:token@github.com/nunchi/roko.git")
        );
        assert_eq!(execution.repo_url(), "https://github.com/nunchi/roko.git");
    }

    #[test]
    fn params_default_plan_dir() {
        let params = CloudExecutionParams {
            github_owner: "nunchi".into(),
            github_repo: "roko".into(),
            github_token: "token".into(),
            plan_slug: "slug".into(),
            base_branch: None,
            pr_title: None,
            pr_body: None,
            plan_dir: None,
            workspace_dir: None,
            github_mcp_command: None,
            github_mcp_args: None,
        };
        assert_eq!(params.plan_dir(), relative_plans_dir().join("slug"));
    }

    #[test]
    fn parse_https_repo_url() {
        let (owner, repo) = parse_owner_repo("https://github.com/nunchi/roko.git").unwrap();
        assert_eq!(owner, "nunchi");
        assert_eq!(repo, "roko");
    }

    // ── #373 git safety tests ─────────────────────────────────────────

    #[test]
    fn validate_worker_branch_rejects_protected() {
        for branch in PROTECTED_BRANCHES {
            let err = validate_worker_branch(branch);
            assert!(err.is_err(), "should reject protected branch: {branch}");
            let msg = err.unwrap_err().to_string();
            assert!(
                msg.contains("protected"),
                "error should mention 'protected': {msg}"
            );
        }
    }

    #[test]
    fn validate_worker_branch_accepts_impl_prefix() {
        assert!(validate_worker_branch("impl/p07-autofix").is_ok());
        assert!(validate_worker_branch("impl/feature-xyz").is_ok());
        assert!(validate_worker_branch("feature/my-branch").is_ok());
    }

    #[test]
    fn validate_worker_branch_rejects_empty() {
        assert!(validate_worker_branch("").is_err());
    }

    #[test]
    fn validate_worker_branch_rejects_unsafe_chars() {
        assert!(validate_worker_branch("branch name").is_err());
        assert!(validate_worker_branch("branch~1").is_err());
        assert!(validate_worker_branch("branch^2").is_err());
        assert!(validate_worker_branch("branch:ref").is_err());
        assert!(validate_worker_branch("branch?glob").is_err());
        assert!(validate_worker_branch("branch*star").is_err());
        assert!(validate_worker_branch("branch[0]").is_err());
        assert!(validate_worker_branch("branch\\escape").is_err());
    }

    #[test]
    fn is_excluded_matches_sensitive_paths() {
        // Exact basename matches.
        assert!(is_excluded(".env"));
        assert!(is_excluded("subdir/.env"));
        assert!(is_excluded("credentials.json"));
        assert!(is_excluded("deep/nested/secrets.json"));

        // Directory prefix match -- entire .roko/ tree is excluded (#373).
        assert!(is_excluded(".roko/state/graph/checkpoint.json"));
        assert!(is_excluded(".roko/state/state-snapshot.json"));
        assert!(is_excluded(".roko/episodes.jsonl"));
        assert!(is_excluded(".roko/engrams.jsonl"));

        // Safe paths must not be excluded.
        assert!(!is_excluded("src/main.rs"));
        assert!(!is_excluded("Cargo.toml"));
        assert!(!is_excluded("plans/p07/tasks.toml"));
        assert!(!is_excluded("crates/roko-core/src/lib.rs"));
    }
}
