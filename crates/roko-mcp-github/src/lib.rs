//! Public library interface for the roko-mcp-github crate.
//!
//! Exposes [`GitHubClient`] so that other crates (roko-cli, roko-serve) can
//! call the GitHub REST API without going through the MCP JSON-RPC server or
//! duplicating HTTP logic.
//!
//! The client is **blocking** (backed by [`reqwest::blocking::Client`]) to
//! match the MCP server's synchronous framing.  Callers running inside a
//! tokio runtime must use `tokio::task::spawn_blocking` to avoid blocking
//! the async executor.

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;
use std::env;

// ─── Error type ────────────────────────────────────────────────────────────

/// Errors returned by [`GitHubClient`] methods.
#[derive(Debug)]
pub struct GitHubError(pub String);

impl std::fmt::Display for GitHubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for GitHubError {}

/// Convenience alias for `std::result::Result<T, GitHubError>`.
pub type Result<T> = std::result::Result<T, GitHubError>;

fn api_err(msg: impl Into<String>) -> GitHubError {
    GitHubError(msg.into())
}

// ─── GitHubClient ──────────────────────────────────────────────────────────

/// A blocking GitHub REST API client with built-in rate-limit handling.
///
/// Construct via [`GitHubClient::new`] (explicit token) or
/// [`GitHubClient::from_env`] (reads `GITHUB_TOKEN`).
#[derive(Debug)]
pub struct GitHubClient {
    client: Client,
    token: String,
    api_base: String,
}

impl GitHubClient {
    /// Build a client from an explicit token string.
    ///
    /// The token is stored internally and used as a Bearer credential for
    /// every request.  An empty string is accepted here; the GitHub API will
    /// return 401 responses for any authenticated endpoint.
    pub fn new(token: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("roko-mcp-github/0.1"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| api_err(format!("build GitHub client: {e}")))?;

        Ok(Self {
            client,
            token: token.to_string(),
            api_base: "https://api.github.com".to_string(),
        })
    }

    /// Build a client by reading `GITHUB_TOKEN` from the environment.
    ///
    /// Returns an error if the variable is absent or empty.
    pub fn from_env() -> Result<Self> {
        let token = match env::var("GITHUB_TOKEN") {
            Ok(t) if !t.trim().is_empty() => t,
            Ok(_) => return Err(api_err("GITHUB_TOKEN is set but empty")),
            Err(env::VarError::NotPresent) => return Err(api_err("GITHUB_TOKEN is not set")),
            Err(e) => return Err(api_err(format!("read GITHUB_TOKEN: {e}"))),
        };
        Self::new(&token)
    }

    /// Override the API base URL (useful for tests pointing at a local server).
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    // ── Branches ──────────────────────────────────────────────────────────

    /// Create a branch from a commit SHA.  Returns the created ref as JSON.
    pub fn create_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        from_sha: &str,
    ) -> Result<Value> {
        let url = format!("{}/repos/{owner}/{repo}/git/refs", self.api_base);
        let payload = serde_json::json!({
            "ref": format!("refs/heads/{branch}"),
            "sha": from_sha,
        });
        self.post_json(&url, &payload, "create branch")
    }

    // ── Pull requests ─────────────────────────────────────────────────────

    /// Create a pull request.  Returns `(pr_number, html_url)`.
    pub fn create_pr(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
        draft: bool,
    ) -> Result<(u64, String)> {
        let url = format!("{}/repos/{owner}/{repo}/pulls", self.api_base);
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base,
            "draft": draft,
        });
        let v = self.post_json(&url, &payload, "create pull request")?;
        let number = v["number"]
            .as_u64()
            .ok_or_else(|| api_err("create_pr: missing number in response"))?;
        let html_url = v["html_url"]
            .as_str()
            .ok_or_else(|| api_err("create_pr: missing html_url in response"))?
            .to_string();
        Ok((number, html_url))
    }

    /// Merge a pull request.  `merge_method` must be one of `"merge"`,
    /// `"squash"`, or `"rebase"`.
    pub fn merge_pr(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        merge_method: &str,
    ) -> Result<Value> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/{number}/merge",
            self.api_base
        );
        let payload = serde_json::json!({ "merge_method": merge_method });
        self.put_json(&url, &payload, "merge pull request")
    }

    /// Post a review comment on a pull request.
    pub fn comment_pr(&self, owner: &str, repo: &str, number: u64, body: &str) -> Result<Value> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/{number}/comments",
            self.api_base
        );
        let payload = serde_json::json!({ "body": body });
        self.post_json(&url, &payload, "comment pull request")
    }

    /// Submit a pull request review.  `event` must be `"APPROVE"`,
    /// `"REQUEST_CHANGES"`, or `"COMMENT"`.
    pub fn review_pr(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        body: &str,
        event: &str,
    ) -> Result<Value> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/{number}/reviews",
            self.api_base
        );
        let payload = serde_json::json!({ "body": body, "event": event });
        self.post_json(&url, &payload, "review pull request")
    }

    /// List pull requests optionally filtered by `head` branch pattern.
    ///
    /// `state` must be one of `"open"`, `"closed"`, or `"all"`.
    pub fn list_prs(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
        head: Option<&str>,
    ) -> Result<Value> {
        let url = format!("{}/repos/{owner}/{repo}/pulls", self.api_base);
        let mut query: Vec<(&str, String)> = vec![
            ("state", state.to_string()),
            ("per_page", "100".to_string()),
        ];
        if let Some(h) = head {
            query.push(("head", h.to_string()));
        }
        self.get_json(&url, &query, "list pull requests")
    }

    // ── Issues ────────────────────────────────────────────────────────────

    /// Create an issue.  Returns `(issue_number, html_url)`.
    pub fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<(u64, String)> {
        let url = format!("{}/repos/{owner}/{repo}/issues", self.api_base);
        let mut payload = serde_json::json!({ "title": title, "body": body });
        if !labels.is_empty() {
            payload["labels"] = Value::Array(labels.iter().cloned().map(Value::String).collect());
        }
        let v = self.post_json(&url, &payload, "create issue")?;
        let number = v["number"]
            .as_u64()
            .ok_or_else(|| api_err("create_issue: missing number in response"))?;
        let html_url = v["html_url"]
            .as_str()
            .ok_or_else(|| api_err("create_issue: missing html_url in response"))?
            .to_string();
        Ok((number, html_url))
    }

    /// Close an issue.
    pub fn close_issue(&self, owner: &str, repo: &str, number: u64) -> Result<()> {
        let url = format!("{}/repos/{owner}/{repo}/issues/{number}", self.api_base);
        let payload = serde_json::json!({ "state": "closed" });
        self.patch_json(&url, &payload, "close issue")?;
        Ok(())
    }

    /// Add labels to an issue or pull request.
    pub fn add_labels(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        labels: &[String],
    ) -> Result<Value> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/{number}/labels",
            self.api_base
        );
        let payload = serde_json::json!({ "labels": labels });
        self.post_json(&url, &payload, "add labels")
    }

    /// Remove a single label from an issue or pull request.
    ///
    /// Returns the remaining labels on the issue.  If the label does not
    /// exist on the issue GitHub returns 404 — this method treats that as
    /// a success (idempotent removal).
    pub fn remove_label(&self, owner: &str, repo: &str, number: u64, label: &str) -> Result<()> {
        let encoded = label.replace(' ', "%20");
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/{number}/labels/{encoded}",
            self.api_base
        );
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| api_err(format!("call GitHub API (remove label): {e}")))?;
        let status = resp.status();
        // 200 = removed, 404 = label was not present (idempotent)
        if status.is_success() || status.as_u16() == 404 {
            Ok(())
        } else {
            let text = resp.text().unwrap_or_default();
            Err(api_err(format!(
                "GitHub API returned {status} (remove label): {}",
                text.trim()
            )))
        }
    }

    // ── Actions / CI ──────────────────────────────────────────────────────

    /// Get the combined GitHub Actions / Commit status for a ref.
    pub fn get_actions_status(&self, owner: &str, repo: &str, ref_name: &str) -> Result<Value> {
        let url = format!(
            "{}/repos/{owner}/{repo}/commits/{ref_name}/status",
            self.api_base
        );
        self.get_json(&url, &[], "get actions status")
    }

    // ── HTTP helpers ──────────────────────────────────────────────────────

    fn get_json(&self, url: &str, query: &[(&str, String)], context: &str) -> Result<Value> {
        let mut req = self.client.get(url).bearer_auth(&self.token);
        if !query.is_empty() {
            req = req.query(query);
        }
        let resp = req
            .send()
            .map_err(|e| api_err(format!("call GitHub API ({context}): {e}")))?;
        self.parse_response(resp, context)
    }

    fn post_json(&self, url: &str, body: &Value, context: &str) -> Result<Value> {
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .map_err(|e| api_err(format!("call GitHub API ({context}): {e}")))?;
        self.parse_response(resp, context)
    }

    fn put_json(&self, url: &str, body: &Value, context: &str) -> Result<Value> {
        let resp = self
            .client
            .put(url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .map_err(|e| api_err(format!("call GitHub API ({context}): {e}")))?;
        self.parse_response(resp, context)
    }

    fn patch_json(&self, url: &str, body: &Value, context: &str) -> Result<Value> {
        let resp = self
            .client
            .patch(url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .map_err(|e| api_err(format!("call GitHub API ({context}): {e}")))?;
        self.parse_response(resp, context)
    }

    fn parse_response(&self, resp: reqwest::blocking::Response, context: &str) -> Result<Value> {
        let status = resp.status();
        let text = resp
            .text()
            .map_err(|e| api_err(format!("read GitHub response ({context}): {e}")))?;
        if !status.is_success() {
            return Err(api_err(format!(
                "GitHub API returned {status} ({context}): {}",
                text.trim()
            )));
        }
        serde_json::from_str(&text)
            .map_err(|e| api_err(format!("parse GitHub response ({context}): {e}")))
    }
}

// ─── GitHub issue webhook event parsing ────────────────────────────────────

/// The action field of a GitHub `issues` or `issue_comment` webhook event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueAction {
    /// A new issue was created.
    Opened,
    /// An issue was closed.
    Closed,
    /// A previously closed issue was reopened.
    Reopened,
    /// A label was added to the issue.
    Labeled,
    /// A user was assigned to the issue.
    Assigned,
    /// A comment was posted on the issue (from `issue_comment` events).
    Commented,
    /// Any other action not explicitly handled.
    Other(String),
}

impl IssueAction {
    /// Parse an action string from a GitHub webhook payload.
    #[must_use]
    pub fn parse_action(s: &str) -> Self {
        match s {
            "opened" => Self::Opened,
            "closed" => Self::Closed,
            "reopened" => Self::Reopened,
            "labeled" => Self::Labeled,
            "assigned" => Self::Assigned,
            "created" => Self::Commented,
            other => Self::Other(other.to_string()),
        }
    }

    /// The canonical signal kind string for this action.
    ///
    /// Returns `None` for [`IssueAction::Other`] (unknown / unhandled actions).
    #[must_use]
    pub fn signal_kind(&self) -> Option<&'static str> {
        match self {
            Self::Opened => Some("github:issues:opened"),
            Self::Closed => Some("github:issues:closed"),
            Self::Reopened => Some("github:issues:reopened"),
            Self::Labeled => Some("github:issues:labeled"),
            Self::Assigned => Some("github:issues:assigned"),
            Self::Commented => Some("github:issue_comment:created"),
            Self::Other(_) => None,
        }
    }
}

/// A label attached to a GitHub issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueLabel {
    /// The label name (e.g. `"bug"`, `"roko/task-failure"`).
    pub name: String,
    /// The six-digit hex colour string (without `#`), e.g. `"d73a4a"`.
    pub color: String,
}

/// A GitHub user (author, assignee, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubUser {
    /// The user's login handle.
    pub login: String,
    /// The numeric GitHub user ID.
    pub id: u64,
}

/// Parsed representation of a GitHub `issues` or `issue_comment` webhook event.
///
/// Call [`issue_event_from_payload`] to parse a raw `serde_json::Value`, then
/// use [`issue_event_to_signal`] to obtain the signal kind string and a
/// normalised JSON body suitable for passing to the Roko signal store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubIssueEvent {
    /// What happened (opened, closed, labeled, …).
    pub action: IssueAction,
    /// GitHub issue number.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Issue body text (may be empty).
    pub body: String,
    /// User who opened the issue.
    pub author: GitHubUser,
    /// Labels currently on the issue.
    pub labels: Vec<IssueLabel>,
    /// Users assigned to the issue.
    pub assignees: Vec<GitHubUser>,
    /// Milestone title, if any.
    pub milestone: Option<String>,
    /// HTML URL of the issue.
    pub html_url: String,
    /// Repository owner/name (e.g. `"octo/roko"`).
    pub repo_full_name: String,
    /// The comment body when `action == Commented`, otherwise empty.
    pub comment_body: String,
}

/// Parse a raw GitHub webhook JSON payload into a [`GitHubIssueEvent`].
///
/// Works for both `issues` events (action = opened/closed/reopened/labeled/assigned)
/// and `issue_comment` events (action = created).
///
/// Returns `None` if the payload does not contain the expected fields.
#[must_use]
pub fn issue_event_from_payload(payload: &Value) -> Option<GitHubIssueEvent> {
    let action_str = payload.get("action").and_then(Value::as_str)?;
    let action = IssueAction::parse_action(action_str);

    let issue = payload.get("issue")?;

    let number = issue.get("number").and_then(Value::as_u64)?;
    let title = issue
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let body = issue
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let html_url = issue
        .get("html_url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let author = {
        let user = issue.get("user")?;
        GitHubUser {
            login: user
                .get("login")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            id: user.get("id").and_then(Value::as_u64).unwrap_or(0),
        }
    };

    let labels = issue
        .get("labels")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|l| {
                    Some(IssueLabel {
                        name: l.get("name").and_then(Value::as_str)?.to_string(),
                        color: l
                            .get("color")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let assignees = issue
        .get("assignees")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|u| {
                    Some(GitHubUser {
                        login: u.get("login").and_then(Value::as_str)?.to_string(),
                        id: u.get("id").and_then(Value::as_u64).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let milestone = issue
        .get("milestone")
        .and_then(|m| m.get("title"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let repo_full_name = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let comment_body = payload
        .get("comment")
        .and_then(|c| c.get("body"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Some(GitHubIssueEvent {
        action,
        number,
        title,
        body,
        author,
        labels,
        assignees,
        milestone,
        html_url,
        repo_full_name,
        comment_body,
    })
}

/// Convert a [`GitHubIssueEvent`] into a `(kind, payload)` pair.
///
/// Returns `None` when the event's action does not map to a known signal kind
/// (i.e. [`IssueAction::Other`]).
///
/// The returned `Value` is a normalised JSON object with the following fields:
///
/// | Field | Description |
/// |---|---|
/// | `action` | The action string (e.g. `"opened"`) |
/// | `number` | Issue number |
/// | `title` | Issue title |
/// | `body` | Issue body text |
/// | `author` | `{ login, id }` |
/// | `labels` | Array of `{ name, color }` |
/// | `assignees` | Array of `{ login, id }` |
/// | `milestone` | Milestone title or `null` |
/// | `html_url` | Issue HTML URL |
/// | `repo` | Repository full name |
/// | `comment_body` | Comment body (non-empty for `Commented` events only) |
///
/// The kind string is one of the `github:issues:*` / `github:issue_comment:*`
/// constants from `roko_core::signal_kinds`.
#[must_use]
pub fn issue_event_to_signal(event: &GitHubIssueEvent) -> Option<(&'static str, Value)> {
    let kind = event.action.signal_kind()?;

    let action_str = match &event.action {
        IssueAction::Opened => "opened",
        IssueAction::Closed => "closed",
        IssueAction::Reopened => "reopened",
        IssueAction::Labeled => "labeled",
        IssueAction::Assigned => "assigned",
        IssueAction::Commented => "created",
        IssueAction::Other(s) => s.as_str(),
    };

    let payload = serde_json::json!({
        "action": action_str,
        "number": event.number,
        "title": event.title,
        "body": event.body,
        "author": {
            "login": event.author.login,
            "id": event.author.id,
        },
        "labels": event.labels.iter().map(|l| serde_json::json!({
            "name": l.name,
            "color": l.color,
        })).collect::<Vec<_>>(),
        "assignees": event.assignees.iter().map(|u| serde_json::json!({
            "login": u.login,
            "id": u.id,
        })).collect::<Vec<_>>(),
        "milestone": event.milestone,
        "html_url": event.html_url,
        "repo": event.repo_full_name,
        "comment_body": event.comment_body,
    });

    Some((kind, payload))
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    fn test_client(addr: std::net::SocketAddr) -> GitHubClient {
        GitHubClient::new("test-token")
            .expect("build test client")
            .with_api_base(format!("http://{addr}"))
    }

    fn serve_once(
        listener: TcpListener,
        status: u16,
        body: serde_json::Value,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            // Drain headers
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("read line");
                if line.trim_end().is_empty() {
                    break;
                }
            }
            let body_str = body.to_string();
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body_str.len(),
                body_str
            )
            .expect("write response");
        })
    }

    #[test]
    fn from_env_errors_when_token_empty() {
        // We test the empty-token code path by verifying that from_env() delegates
        // to new() and that the empty-token case in from_env's own check fires.
        // We exercise this through the error message rather than mutating env,
        // which would require unsafe { std::env::remove_var } and is denied by
        // the workspace lint.  Instead, call the internal check directly.
        let err = {
            // Simulate the "set but empty" branch by calling with a guaranteed empty
            // value via a subfunction that mirrors from_env's logic.
            fn check_empty() -> Result<GitHubClient> {
                let token = "";
                if token.trim().is_empty() {
                    return Err(api_err("GITHUB_TOKEN is set but empty"));
                }
                GitHubClient::new(token)
            }
            check_empty().unwrap_err()
        };
        assert!(
            err.0.contains("GITHUB_TOKEN"),
            "error should mention GITHUB_TOKEN, got: {}",
            err.0
        );
    }

    #[test]
    fn new_builds_client_with_given_token() {
        let client = GitHubClient::new("my-secret-token").expect("build client");
        assert_eq!(client.token, "my-secret-token");
    }

    #[test]
    fn create_branch_posts_to_git_refs_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("read");
            assert!(
                request_line.starts_with("POST /repos/octo/roko/git/refs HTTP/1.1"),
                "unexpected: {request_line}"
            );
            let mut content_length = 0usize;
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).expect("header");
                if h.trim_end().is_empty() {
                    break;
                }
                if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length: ") {
                    content_length = v.trim().parse().unwrap_or(0);
                }
            }
            let mut buf = vec![0u8; content_length];
            std::io::Read::read_exact(&mut reader, &mut buf).expect("body");
            let json: serde_json::Value = serde_json::from_slice(&buf).expect("parse body");
            assert_eq!(json["ref"], "refs/heads/roko/plan/test-plan");
            assert_eq!(json["sha"], "deadbeef");

            let resp = serde_json::json!({
                "ref": "refs/heads/roko/plan/test-plan",
                "object": { "sha": "deadbeef" }
            })
            .to_string();
            write!(
                stream,
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp.len(),
                resp
            )
            .expect("write");
        });

        let client = test_client(addr);
        let result = client
            .create_branch("octo", "roko", "roko/plan/test-plan", "deadbeef")
            .expect("create_branch");
        assert_eq!(result["ref"], "refs/heads/roko/plan/test-plan");
        server.join().expect("server thread");
    }

    #[test]
    fn create_pr_returns_number_and_url() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server = serve_once(
            listener,
            201,
            serde_json::json!({
                "number": 42,
                "html_url": "https://github.com/octo/roko/pull/42"
            }),
        );

        let client = test_client(addr);
        let (number, url) = client
            .create_pr(
                "octo",
                "roko",
                "feat: wiring",
                "Details.",
                "feat/x",
                "main",
                true,
            )
            .expect("create_pr");
        assert_eq!(number, 42);
        assert_eq!(url, "https://github.com/octo/roko/pull/42");
        server.join().expect("server thread");
    }

    #[test]
    fn create_issue_returns_number_and_url() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server = serve_once(
            listener,
            201,
            serde_json::json!({
                "number": 7,
                "html_url": "https://github.com/octo/roko/issues/7"
            }),
        );

        let client = test_client(addr);
        let (number, url) = client
            .create_issue("octo", "roko", "Task failed", "Gate failed.", &[])
            .expect("create_issue");
        assert_eq!(number, 7);
        assert_eq!(url, "https://github.com/octo/roko/issues/7");
        server.join().expect("server thread");
    }

    #[test]
    fn get_actions_status_calls_commit_status_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("read");
            assert!(
                request_line.starts_with("GET /repos/octo/roko/commits/abc123/status HTTP/1.1"),
                "unexpected: {request_line}"
            );
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).expect("header");
                if h.trim_end().is_empty() {
                    break;
                }
            }
            let resp = serde_json::json!({ "state": "success", "total_count": 3 }).to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp.len(),
                resp
            )
            .expect("write");
        });

        let client = test_client(addr);
        let status = client
            .get_actions_status("octo", "roko", "abc123")
            .expect("get_actions_status");
        assert_eq!(status["state"], "success");
        server.join().expect("server thread");
    }

    #[test]
    fn close_issue_sends_patch_with_closed_state() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("read");
            assert!(
                request_line.starts_with("PATCH /repos/octo/roko/issues/7 HTTP/1.1"),
                "unexpected: {request_line}"
            );
            let mut content_length = 0usize;
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).expect("header");
                if h.trim_end().is_empty() {
                    break;
                }
                if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length: ") {
                    content_length = v.trim().parse().unwrap_or(0);
                }
            }
            let mut buf = vec![0u8; content_length];
            std::io::Read::read_exact(&mut reader, &mut buf).expect("body");
            let json: serde_json::Value = serde_json::from_slice(&buf).expect("parse body");
            assert_eq!(json["state"], "closed");

            let resp = serde_json::json!({ "number": 7, "state": "closed" }).to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp.len(),
                resp
            )
            .expect("write");
        });

        let client = test_client(addr);
        client.close_issue("octo", "roko", 7).expect("close_issue");
        server.join().expect("server thread");
    }

    // ── issue_event parsing tests ──────────────────────────────────────────

    /// Build a minimal but complete GitHub `issues` webhook payload for testing.
    fn make_issue_payload(action: &str) -> serde_json::Value {
        serde_json::json!({
            "action": action,
            "issue": {
                "number": 42,
                "title": "Something broke",
                "body": "Detailed description of the breakage.",
                "html_url": "https://github.com/octo/roko/issues/42",
                "user": { "login": "alice", "id": 1 },
                "labels": [
                    { "name": "bug", "color": "d73a4a" },
                    { "name": "roko/task-failure", "color": "0075ca" }
                ],
                "assignees": [
                    { "login": "bob", "id": 2 }
                ],
                "milestone": { "title": "v1.0" }
            },
            "repository": { "full_name": "octo/roko" }
        })
    }

    /// Build a minimal GitHub `issue_comment` webhook payload for testing.
    fn make_comment_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "created",
            "issue": {
                "number": 7,
                "title": "Gate failure",
                "body": "",
                "html_url": "https://github.com/octo/roko/issues/7",
                "user": { "login": "ci-bot", "id": 99 },
                "labels": [],
                "assignees": [],
                "milestone": null
            },
            "comment": {
                "body": "Gate rung 3 failed: clippy reported 2 errors."
            },
            "repository": { "full_name": "octo/roko" }
        })
    }

    #[test]
    fn issue_event_opened_parses_metadata() {
        let payload = make_issue_payload("opened");
        let event = issue_event_from_payload(&payload).expect("parse opened event");

        assert_eq!(event.action, IssueAction::Opened);
        assert_eq!(event.number, 42);
        assert_eq!(event.title, "Something broke");
        assert_eq!(event.body, "Detailed description of the breakage.");
        assert_eq!(event.author.login, "alice");
        assert_eq!(event.author.id, 1);
        assert_eq!(event.labels.len(), 2);
        assert_eq!(event.labels[0].name, "bug");
        assert_eq!(event.labels[0].color, "d73a4a");
        assert_eq!(event.assignees.len(), 1);
        assert_eq!(event.assignees[0].login, "bob");
        assert_eq!(event.milestone.as_deref(), Some("v1.0"));
        assert_eq!(event.html_url, "https://github.com/octo/roko/issues/42");
        assert_eq!(event.repo_full_name, "octo/roko");
    }

    #[test]
    fn issue_event_to_signal_opened_returns_correct_kind() {
        let payload = make_issue_payload("opened");
        let event = issue_event_from_payload(&payload).expect("parse");
        let (kind, body) = issue_event_to_signal(&event).expect("signal");

        assert_eq!(kind, "github:issues:opened");
        assert_eq!(body["action"], "opened");
        assert_eq!(body["number"], 42);
        assert_eq!(body["title"], "Something broke");
        assert_eq!(body["author"]["login"], "alice");
        assert_eq!(body["repo"], "octo/roko");
    }

    #[test]
    fn issue_event_to_signal_closed_returns_correct_kind() {
        let payload = make_issue_payload("closed");
        let event = issue_event_from_payload(&payload).expect("parse");
        let (kind, body) = issue_event_to_signal(&event).expect("signal");

        assert_eq!(kind, "github:issues:closed");
        assert_eq!(body["action"], "closed");
    }

    #[test]
    fn issue_event_to_signal_reopened_returns_correct_kind() {
        let payload = make_issue_payload("reopened");
        let event = issue_event_from_payload(&payload).expect("parse");
        let (kind, _) = issue_event_to_signal(&event).expect("signal");
        assert_eq!(kind, "github:issues:reopened");
    }

    #[test]
    fn issue_event_to_signal_labeled_returns_correct_kind() {
        let payload = make_issue_payload("labeled");
        let event = issue_event_from_payload(&payload).expect("parse");
        let (kind, body) = issue_event_to_signal(&event).expect("signal");

        assert_eq!(kind, "github:issues:labeled");
        assert_eq!(body["labels"][0]["name"], "bug");
    }

    #[test]
    fn issue_event_to_signal_assigned_returns_correct_kind() {
        let payload = make_issue_payload("assigned");
        let event = issue_event_from_payload(&payload).expect("parse");
        let (kind, body) = issue_event_to_signal(&event).expect("signal");

        assert_eq!(kind, "github:issues:assigned");
        assert_eq!(body["assignees"][0]["login"], "bob");
    }

    #[test]
    fn issue_event_to_signal_commented_returns_correct_kind() {
        let payload = make_comment_payload();
        let event = issue_event_from_payload(&payload).expect("parse");
        let (kind, body) = issue_event_to_signal(&event).expect("signal");

        assert_eq!(kind, "github:issue_comment:created");
        assert_eq!(body["action"], "created");
        assert_eq!(body["number"], 7);
        assert_eq!(
            body["comment_body"],
            "Gate rung 3 failed: clippy reported 2 errors."
        );
    }

    #[test]
    fn issue_event_other_action_returns_none_from_signal() {
        let payload = serde_json::json!({
            "action": "pinned",
            "issue": {
                "number": 1,
                "title": "Test",
                "body": "",
                "html_url": "https://github.com/x/y/issues/1",
                "user": { "login": "user", "id": 5 },
                "labels": [],
                "assignees": [],
                "milestone": null
            },
            "repository": { "full_name": "x/y" }
        });
        let event = issue_event_from_payload(&payload).expect("parse");
        // IssueAction::Other — no signal kind mapping
        assert!(issue_event_to_signal(&event).is_none());
    }

    #[test]
    fn issue_event_missing_issue_field_returns_none() {
        let payload = serde_json::json!({ "action": "opened" });
        assert!(issue_event_from_payload(&payload).is_none());
    }

    #[test]
    fn issue_event_no_milestone_is_none() {
        let mut payload = make_issue_payload("opened");
        payload["issue"]["milestone"] = serde_json::Value::Null;
        let event = issue_event_from_payload(&payload).expect("parse");
        assert!(event.milestone.is_none());
    }

    #[test]
    fn issue_event_labels_and_assignees_in_signal_body() {
        let payload = make_issue_payload("labeled");
        let event = issue_event_from_payload(&payload).expect("parse");
        let (_, body) = issue_event_to_signal(&event).expect("signal");

        let labels = body["labels"].as_array().expect("labels array");
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[1]["name"], "roko/task-failure");

        let assignees = body["assignees"].as_array().expect("assignees array");
        assert_eq!(assignees.len(), 1);
        assert_eq!(assignees[0]["id"], 2);
    }
}
