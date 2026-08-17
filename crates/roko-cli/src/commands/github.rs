//! Standalone GitHub integration diagnostics.

use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Subcommand;
use roko_core::config::GitHubConfig;
use roko_core::config::loader::{LoadOptions, load_config_file, load_config_unified};
use roko_mcp_github::GitHubClient;
use serde::Serialize;
use serde_json::Value;

use crate::{Cli, EXIT_SUCCESS, resolve_workdir};

#[derive(Debug, Subcommand)]
pub(crate) enum GithubCmd {
    /// Show configuration, authentication, plan PRs, CI, and failure issues.
    Status {
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) async fn cmd_github(cli: &Cli, command: GithubCmd) -> Result<i32> {
    match command {
        GithubCmd::Status { workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let config = load_github_config(&workdir, cli.config.as_deref())?;
            let token_state = token_state_from_env();
            let report = if token_state == TokenState::Present {
                match GitHubClient::from_env() {
                    Ok(client) => {
                        let worker_config = config.clone();
                        tokio::task::spawn_blocking(move || {
                            collect_status(&worker_config, token_state, &client)
                        })
                        .await
                        .unwrap_or_else(|error| {
                            GitHubStatusReport::without_remote(
                                &config,
                                token_state,
                                format!("GitHub status worker failed: {error}"),
                            )
                        })
                    }
                    Err(error) => {
                        GitHubStatusReport::without_remote(&config, token_state, error.to_string())
                    }
                }
            } else {
                GitHubStatusReport::without_remote(
                    &config,
                    token_state,
                    token_state.help().to_string(),
                )
            };

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", report.render_human());
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

fn load_github_config(workdir: &Path, override_path: Option<&Path>) -> Result<GitHubConfig> {
    let options = LoadOptions::default();
    let config = match override_path {
        Some(path) => load_config_file(path, &options)
            .with_context(|| format!("load GitHub config from {}", path.display()))?,
        None => load_config_unified(workdir)
            .with_context(|| format!("load GitHub config for {}", workdir.display()))?,
    };
    Ok(config.github)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum TokenState {
    Present,
    Missing,
    Empty,
    Unreadable,
}

impl TokenState {
    const fn help(self) -> &'static str {
        match self {
            Self::Present => "GITHUB_TOKEN is set",
            Self::Missing => "GITHUB_TOKEN is not set; export it to query GitHub",
            Self::Empty => "GITHUB_TOKEN is empty; set it to a GitHub token",
            Self::Unreadable => "GITHUB_TOKEN could not be read",
        }
    }
}

fn token_state_from_env() -> TokenState {
    match env::var("GITHUB_TOKEN") {
        Ok(token) if token.trim().is_empty() => TokenState::Empty,
        Ok(_) => TokenState::Present,
        Err(env::VarError::NotPresent) => TokenState::Missing,
        Err(env::VarError::NotUnicode(_)) => TokenState::Unreadable,
    }
}

trait GithubStatusClient {
    fn authenticated_user(&self) -> std::result::Result<Value, String>;
    fn list_open_prs(&self, owner: &str, repo: &str) -> std::result::Result<Value, String>;
    fn ci_status(
        &self,
        owner: &str,
        repo: &str,
        reference: &str,
    ) -> std::result::Result<Value, String>;
    fn list_failure_issues(
        &self,
        owner: &str,
        repo: &str,
        label: &str,
    ) -> std::result::Result<Value, String>;
}

impl GithubStatusClient for GitHubClient {
    fn authenticated_user(&self) -> std::result::Result<Value, String> {
        self.authenticated_user().map_err(|error| error.to_string())
    }

    fn list_open_prs(&self, owner: &str, repo: &str) -> std::result::Result<Value, String> {
        self.list_prs(owner, repo, "open", None)
            .map_err(|error| error.to_string())
    }

    fn ci_status(
        &self,
        owner: &str,
        repo: &str,
        reference: &str,
    ) -> std::result::Result<Value, String> {
        self.get_actions_status(owner, repo, reference)
            .map_err(|error| error.to_string())
    }

    fn list_failure_issues(
        &self,
        owner: &str,
        repo: &str,
        label: &str,
    ) -> std::result::Result<Value, String> {
        self.list_issues(owner, repo, "open", &[label.to_string()], None, 100)
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Serialize)]
struct GitHubStatusReport {
    config: ConfigStatus,
    auth: AuthStatus,
    plan_pull_requests: PullRequestStatus,
    task_failure_issues: IssueStatus,
}

#[derive(Debug, Serialize)]
struct ConfigStatus {
    valid: bool,
    owner: Option<String>,
    repo: Option<String>,
    auto_pr: bool,
    merge_method: String,
    label_prefix: String,
}

#[derive(Debug, Serialize)]
struct AuthStatus {
    token: TokenState,
    authenticated: bool,
    user: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RemoteState {
    Ok,
    Skipped,
    Error,
}

#[derive(Debug, Serialize)]
struct PullRequestStatus {
    state: RemoteState,
    items: Vec<PlanPullRequest>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct PlanPullRequest {
    number: u64,
    title: String,
    branch: String,
    url: Option<String>,
    ci_state: String,
    ci_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct IssueStatus {
    state: RemoteState,
    label: String,
    items: Vec<TaskFailureIssue>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct TaskFailureIssue {
    number: u64,
    title: String,
    url: Option<String>,
}

impl GitHubStatusReport {
    fn base(config: &GitHubConfig, token: TokenState) -> Self {
        let owner = non_empty(config.owner.as_deref());
        let repo = non_empty(config.repo.as_deref());
        let failure_label = format!("{}task-failure", config.label_prefix);
        Self {
            config: ConfigStatus {
                valid: owner.is_some() && repo.is_some(),
                owner,
                repo,
                auto_pr: config.auto_pr,
                merge_method: config.merge_method.to_string(),
                label_prefix: config.label_prefix.clone(),
            },
            auth: AuthStatus {
                token,
                authenticated: false,
                user: None,
                error: None,
            },
            plan_pull_requests: PullRequestStatus {
                state: RemoteState::Skipped,
                items: Vec::new(),
                error: None,
            },
            task_failure_issues: IssueStatus {
                state: RemoteState::Skipped,
                label: failure_label,
                items: Vec::new(),
                error: None,
            },
        }
    }

    fn without_remote(config: &GitHubConfig, token: TokenState, error: String) -> Self {
        let mut report = Self::base(config, token);
        report.auth.error = Some(error.clone());
        report.plan_pull_requests.error = Some(error.clone());
        report.task_failure_issues.error = Some(error);
        report
    }

    fn render_human(&self) -> String {
        let mut output = String::from("GitHub status\n");
        let repository = match (&self.config.owner, &self.config.repo) {
            (Some(owner), Some(repo)) => format!("{owner}/{repo}"),
            _ => "not configured".to_string(),
        };
        let _ = writeln!(output, "  Repository: {repository}");
        let _ = writeln!(
            output,
            "  Config: {}",
            if self.config.valid {
                "valid"
            } else {
                "incomplete"
            }
        );
        let _ = writeln!(
            output,
            "  Auto PR: {}",
            if self.config.auto_pr {
                "enabled"
            } else {
                "disabled"
            }
        );
        let _ = writeln!(output, "  Merge method: {}", self.config.merge_method);
        let _ = writeln!(output, "  Label prefix: {}", self.config.label_prefix);

        output.push_str("\nAuthentication\n");
        let _ = writeln!(output, "  Token: {}", token_label(self.auth.token));
        match (&self.auth.user, &self.auth.error) {
            (Some(user), _) => {
                let _ = writeln!(output, "  User: @{user}");
            }
            (_, Some(error)) => {
                let _ = writeln!(output, "  User: unavailable ({error})");
            }
            _ => output.push_str("  User: unavailable\n"),
        }

        let _ = writeln!(
            output,
            "\nRoko plan PRs ({})",
            self.plan_pull_requests.items.len()
        );
        if let Some(error) = &self.plan_pull_requests.error {
            let _ = writeln!(
                output,
                "  {}: {error}",
                remote_label(self.plan_pull_requests.state)
            );
        }
        for pr in &self.plan_pull_requests.items {
            let _ = writeln!(
                output,
                "  #{} [{}] {} ({})",
                pr.number, pr.ci_state, pr.title, pr.branch
            );
            if let Some(error) = &pr.ci_error {
                let _ = writeln!(output, "    CI unavailable: {error}");
            }
        }

        let _ = writeln!(
            output,
            "\nTask-failure issues ({}, label {})",
            self.task_failure_issues.items.len(),
            self.task_failure_issues.label
        );
        if let Some(error) = &self.task_failure_issues.error {
            let _ = writeln!(
                output,
                "  {}: {error}",
                remote_label(self.task_failure_issues.state)
            );
        }
        for issue in &self.task_failure_issues.items {
            let _ = writeln!(output, "  #{} {}", issue.number, issue.title);
        }
        output
    }
}

fn collect_status(
    config: &GitHubConfig,
    token: TokenState,
    client: &dyn GithubStatusClient,
) -> GitHubStatusReport {
    let mut report = GitHubStatusReport::base(config, token);
    match client.authenticated_user() {
        Ok(value) => match value.get("login").and_then(Value::as_str) {
            Some(login) => {
                report.auth.authenticated = true;
                report.auth.user = Some(login.to_string());
            }
            None => report.auth.error = Some("authenticated user response missing login".into()),
        },
        Err(error) => report.auth.error = Some(error),
    }

    let (Some(owner), Some(repo)) = (&report.config.owner, &report.config.repo) else {
        let error = "github.owner and github.repo are required for repository status".to_string();
        report.plan_pull_requests.error = Some(error.clone());
        report.task_failure_issues.error = Some(error);
        return report;
    };

    match client.list_open_prs(owner, repo) {
        Ok(value) => match parse_plan_prs(&value) {
            Ok(raw_prs) => {
                report.plan_pull_requests.state = RemoteState::Ok;
                report.plan_pull_requests.items = raw_prs
                    .into_iter()
                    .map(|raw| {
                        let reference = raw.sha.as_deref().unwrap_or(&raw.branch);
                        match client.ci_status(owner, repo, reference) {
                            Ok(status) => PlanPullRequest {
                                number: raw.number,
                                title: raw.title,
                                branch: raw.branch,
                                url: raw.url,
                                ci_state: status
                                    .get("state")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unknown")
                                    .to_string(),
                                ci_error: None,
                            },
                            Err(error) => PlanPullRequest {
                                number: raw.number,
                                title: raw.title,
                                branch: raw.branch,
                                url: raw.url,
                                ci_state: "unavailable".to_string(),
                                ci_error: Some(error),
                            },
                        }
                    })
                    .collect();
            }
            Err(error) => {
                report.plan_pull_requests.state = RemoteState::Error;
                report.plan_pull_requests.error = Some(error);
            }
        },
        Err(error) => {
            report.plan_pull_requests.state = RemoteState::Error;
            report.plan_pull_requests.error = Some(error);
        }
    }

    match client.list_failure_issues(owner, repo, &report.task_failure_issues.label) {
        Ok(value) => match parse_issues(&value) {
            Ok(issues) => {
                report.task_failure_issues.state = RemoteState::Ok;
                report.task_failure_issues.items = issues;
            }
            Err(error) => {
                report.task_failure_issues.state = RemoteState::Error;
                report.task_failure_issues.error = Some(error);
            }
        },
        Err(error) => {
            report.task_failure_issues.state = RemoteState::Error;
            report.task_failure_issues.error = Some(error);
        }
    }
    report
}

struct RawPlanPr {
    number: u64,
    title: String,
    branch: String,
    sha: Option<String>,
    url: Option<String>,
}

fn parse_plan_prs(value: &Value) -> std::result::Result<Vec<RawPlanPr>, String> {
    let entries = value
        .as_array()
        .ok_or_else(|| "GitHub pull-request response was not an array".to_string())?;
    let mut prs = Vec::new();
    for entry in entries {
        let Some(branch) = entry.pointer("/head/ref").and_then(Value::as_str) else {
            continue;
        };
        if !branch.starts_with("roko/plan/") {
            continue;
        }
        let number = entry
            .get("number")
            .and_then(Value::as_u64)
            .ok_or_else(|| "plan PR response missing number".to_string())?;
        let title = entry
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("plan PR #{number} response missing title"))?;
        prs.push(RawPlanPr {
            number,
            title: title.to_string(),
            branch: branch.to_string(),
            sha: entry
                .pointer("/head/sha")
                .and_then(Value::as_str)
                .map(str::to_string),
            url: entry
                .get("html_url")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    Ok(prs)
}

fn parse_issues(value: &Value) -> std::result::Result<Vec<TaskFailureIssue>, String> {
    let entries = value
        .as_array()
        .ok_or_else(|| "GitHub issue response was not an array".to_string())?;
    entries
        .iter()
        .filter(|entry| entry.get("pull_request").is_none())
        .map(|entry| {
            let number = entry
                .get("number")
                .and_then(Value::as_u64)
                .ok_or_else(|| "task-failure issue response missing number".to_string())?;
            let title = entry
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("task-failure issue #{number} response missing title"))?;
            Ok(TaskFailureIssue {
                number,
                title: title.to_string(),
                url: entry
                    .get("html_url")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect()
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

const fn token_label(state: TokenState) -> &'static str {
    match state {
        TokenState::Present => "present",
        TokenState::Missing => "missing",
        TokenState::Empty => "empty",
        TokenState::Unreadable => "unreadable",
    }
}

const fn remote_label(state: RemoteState) -> &'static str {
    match state {
        RemoteState::Ok => "ok",
        RemoteState::Skipped => "skipped",
        RemoteState::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct MockClient {
        user: std::result::Result<Value, String>,
        prs: std::result::Result<Value, String>,
        ci: std::result::Result<Value, String>,
        issues: std::result::Result<Value, String>,
        calls: Mutex<Vec<String>>,
    }

    impl GithubStatusClient for MockClient {
        fn authenticated_user(&self) -> std::result::Result<Value, String> {
            self.calls.lock().unwrap().push("user".into());
            self.user.clone()
        }

        fn list_open_prs(&self, owner: &str, repo: &str) -> std::result::Result<Value, String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("prs:{owner}/{repo}"));
            self.prs.clone()
        }

        fn ci_status(
            &self,
            owner: &str,
            repo: &str,
            reference: &str,
        ) -> std::result::Result<Value, String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("ci:{owner}/{repo}:{reference}"));
            self.ci.clone()
        }

        fn list_failure_issues(
            &self,
            owner: &str,
            repo: &str,
            label: &str,
        ) -> std::result::Result<Value, String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("issues:{owner}/{repo}:{label}"));
            self.issues.clone()
        }
    }

    fn config() -> GitHubConfig {
        GitHubConfig {
            owner: Some("octo".into()),
            repo: Some("roko".into()),
            auto_pr: true,
            ..GitHubConfig::default()
        }
    }

    #[test]
    fn status_uses_mock_boundary_and_filters_roko_plan_prs_and_pull_issues() {
        let client = MockClient {
            user: Ok(serde_json::json!({"login": "octocat"})),
            prs: Ok(serde_json::json!([
                {"number": 7, "title": "Plan work", "html_url": "https://example/pr/7", "head": {"ref": "roko/plan/E46", "sha": "abc"}},
                {"number": 8, "title": "Human PR", "head": {"ref": "feature/manual", "sha": "def"}}
            ])),
            ci: Ok(serde_json::json!({"state": "success"})),
            issues: Ok(serde_json::json!([
                {"number": 9, "title": "Task failed", "html_url": "https://example/issues/9"},
                {"number": 10, "title": "PR-shaped issue", "pull_request": {"url": "x"}}
            ])),
            calls: Mutex::new(Vec::new()),
        };

        let report = collect_status(&config(), TokenState::Present, &client);

        assert!(report.config.valid);
        assert_eq!(report.config.merge_method, "squash");
        assert!(report.auth.authenticated);
        assert_eq!(report.auth.user.as_deref(), Some("octocat"));
        assert_eq!(report.plan_pull_requests.items.len(), 1);
        assert_eq!(report.plan_pull_requests.items[0].ci_state, "success");
        assert_eq!(report.task_failure_issues.items.len(), 1);
        assert_eq!(
            *client.calls.lock().unwrap(),
            [
                "user",
                "prs:octo/roko",
                "ci:octo/roko:abc",
                "issues:octo/roko:roko/task-failure"
            ]
        );
    }

    #[test]
    fn missing_token_is_structured_and_human_readable_without_panicking() {
        let report = GitHubStatusReport::without_remote(
            &config(),
            TokenState::Missing,
            TokenState::Missing.help().into(),
        );
        let human = report.render_human();

        assert_eq!(report.auth.token, TokenState::Missing);
        assert_eq!(report.plan_pull_requests.state, RemoteState::Skipped);
        assert!(human.contains("Token: missing"));
        assert!(human.contains("export it to query GitHub"));
    }

    #[test]
    fn api_failures_are_reported_per_section_and_keep_config_visible() {
        let client = MockClient {
            user: Err("401 bad credentials".into()),
            prs: Err("503 pulls unavailable".into()),
            ci: Ok(serde_json::json!({"state": "pending"})),
            issues: Err("rate limited".into()),
            calls: Mutex::new(Vec::new()),
        };

        let report = collect_status(&config(), TokenState::Present, &client);
        let human = report.render_human();

        assert_eq!(report.auth.error.as_deref(), Some("401 bad credentials"));
        assert_eq!(report.plan_pull_requests.state, RemoteState::Error);
        assert_eq!(report.task_failure_issues.state, RemoteState::Error);
        assert!(human.contains("Repository: octo/roko"));
        assert!(human.contains("503 pulls unavailable"));
        assert!(human.contains("rate limited"));
    }

    #[test]
    fn incomplete_repository_config_skips_repo_calls_but_checks_auth() {
        let client = MockClient {
            user: Ok(serde_json::json!({"login": "octocat"})),
            prs: Ok(Value::Array(Vec::new())),
            ci: Ok(serde_json::json!({"state": "pending"})),
            issues: Ok(Value::Array(Vec::new())),
            calls: Mutex::new(Vec::new()),
        };
        let report = collect_status(&GitHubConfig::default(), TokenState::Present, &client);

        assert!(!report.config.valid);
        assert!(report.auth.authenticated);
        assert_eq!(*client.calls.lock().unwrap(), ["user"]);
        assert!(
            report
                .plan_pull_requests
                .error
                .as_deref()
                .unwrap()
                .contains("github.owner")
        );
    }
}
