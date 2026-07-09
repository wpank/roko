<!-- Master Plan: Tier 2, Section 2D -->
<!-- Status: Not started -->
<!-- Depends on: Tier 1C (MCP tool registry) -->

# Phase 2: Integration MCP Servers

> **Master Plan Reference**: Tier 2, Section 2D
> **Status**: Not started
> **Depends on**: Tier 1C (MCP tool registry)
> **Blocks**: Agent templates that use MCP
>
> ### What Already Exists in Codebase
> - `crates/roko-agent/src/mcp/` — MCP client, JSON-RPC, tool converter, dedup, config walk-up
> - MCP passthrough via `--mcp-config` flag (wired)
>
> ### Reference Material
> - MCP spec: `crates/roko-agent/src/mcp/protocol.rs`
> - Mori tool system: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/tool-system.md`

> Three standalone MCP server binaries. Each communicates via stdio JSON-RPC.
> Agents connect to them via `--mcp-config`. No changes to roko-agent needed.

---

## 2.1 `roko-mcp-github` — GitHub API as MCP tools

### Crate structure

```
crates/roko-mcp-github/
├── Cargo.toml
├── src/
│   ├── main.rs          # MCP server entry (stdio transport)
│   ├── tools/
│   │   ├── mod.rs       # Tool registry + dispatch
│   │   ├── prs.rs       # PR operations
│   │   ├── issues.rs    # Issue operations
│   │   ├── repos.rs     # File/commit/branch operations
│   │   └── actions.rs   # CI/Actions status
│   ├── auth.rs          # GitHub token management
│   └── rate_limit.rs    # X-RateLimit header tracking
```

### `Cargo.toml`

```toml
[package]
name = "roko-mcp-github"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "MCP server exposing GitHub API as tools for roko agents"

[[bin]]
name = "roko-mcp-github"
path = "src/main.rs"

[dependencies]
octocrab = "0.44"
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
base64 = "0.22"
```

### MCP server entry point (`main.rs`)

```rust
//! GitHub MCP server — exposes GitHub API operations as MCP tools.
//!
//! Communicates via stdio JSON-RPC (MCP protocol).
//! Auth: GITHUB_TOKEN environment variable.

use std::io::{self, BufRead, Write};
use serde_json::{json, Value};

mod auth;
mod rate_limit;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("roko_mcp_github=info")
        .with_writer(io::stderr)
        .init();

    let github = auth::create_client()?;
    let stdin = io::stdin().lock();
    let stdout = io::stdout();

    for line in stdin.lines() {
        let line = line?;
        let request: Value = serde_json::from_str(&line)?;

        let response = match request.get("method").and_then(|m| m.as_str()) {
            Some("initialize") => handle_initialize(),
            Some("tools/list") => tools::list_tools(),
            Some("tools/call") => {
                let params = request.get("params").cloned().unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
                tools::call_tool(&github, tool_name, arguments).await
            }
            _ => json!({"jsonrpc": "2.0", "error": {"code": -32601, "message": "method not found"}}),
        };

        let mut out = stdout.lock();
        serde_json::to_writer(&mut out, &response)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }

    Ok(())
}

fn handle_initialize() -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "roko-mcp-github",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}
```

### Tool definitions

#### PR tools (`tools/prs.rs`)

| Tool | Input | Output | Implementation |
|---|---|---|---|
| `github.list_prs` | `{ owner: str, repo: str, state?: "open"\|"closed"\|"all", head?: str, base?: str, per_page?: u32 }` | `[{ number, title, state, user, created_at, updated_at, head_ref, base_ref, draft, mergeable }]` | `octocrab.pulls(owner, repo).list()` |
| `github.get_pr` | `{ owner: str, repo: str, number: u64, include_diff?: bool }` | `{ number, title, body, state, diff?, files_changed, additions, deletions, commits, reviews_summary }` | `octocrab.pulls(owner, repo).get(number)` + optional `.diff()` |
| `github.create_pr` | `{ owner: str, repo: str, title: str, body: str, head: str, base: str, draft?: bool }` | `{ number, html_url }` | `octocrab.pulls(owner, repo).create(title, head, base)` |
| `github.comment_pr` | `{ owner: str, repo: str, number: u64, body: str }` | `{ id, html_url }` | `octocrab.issues(owner, repo).create_comment(number, body)` |
| `github.review_pr` | `{ owner: str, repo: str, number: u64, event: "APPROVE"\|"REQUEST_CHANGES"\|"COMMENT", body: str, comments?: [{ path: str, line: u64, body: str }] }` | `{ id, state }` | `octocrab.pulls(owner, repo).reviews().create(...)` |
| `github.merge_pr` | `{ owner: str, repo: str, number: u64, merge_method?: "merge"\|"squash"\|"rebase", commit_title?: str }` | `{ sha, merged }` | `octocrab.pulls(owner, repo).merge(number)` |

#### Issue tools (`tools/issues.rs`)

| Tool | Input | Output | Implementation |
|---|---|---|---|
| `github.list_issues` | `{ owner: str, repo: str, state?: str, labels?: [str], assignee?: str, per_page?: u32 }` | `[{ number, title, state, labels, assignee, created_at }]` | `octocrab.issues(owner, repo).list()` |
| `github.create_issue` | `{ owner: str, repo: str, title: str, body: str, labels?: [str], assignees?: [str] }` | `{ number, html_url }` | `octocrab.issues(owner, repo).create(title)` |
| `github.comment_issue` | `{ owner: str, repo: str, number: u64, body: str }` | `{ id, html_url }` | `octocrab.issues(owner, repo).create_comment(number, body)` |
| `github.close_issue` | `{ owner: str, repo: str, number: u64, reason?: "completed"\|"not_planned" }` | `{ number, state }` | `octocrab.issues(owner, repo).update(number).state("closed")` |
| `github.add_labels` | `{ owner: str, repo: str, number: u64, labels: [str] }` | `[{ name, color }]` | `octocrab.issues(owner, repo).add_labels(number, labels)` |
| `github.create_label` | `{ owner: str, repo: str, name: str, color: str, description?: str }` | `{ name, color }` | REST API `POST /repos/{owner}/{repo}/labels` |

#### Repo tools (`tools/repos.rs`)

| Tool | Input | Output | Implementation |
|---|---|---|---|
| `github.get_file` | `{ owner: str, repo: str, path: str, ref?: str }` | `{ content: str, sha: str, size: u64 }` | `octocrab.repos(owner, repo).get_content().path(path)` |
| `github.search_code` | `{ query: str, owner?: str, repo?: str, per_page?: u32 }` | `[{ path, repository, score, text_matches? }]` | `octocrab.search().code(query)` |
| `github.list_commits` | `{ owner: str, repo: str, sha?: str, path?: str, since?: str, until?: str, per_page?: u32 }` | `[{ sha, message, author, date }]` | `octocrab.repos(owner, repo).list_commits()` |
| `github.create_branch` | `{ owner: str, repo: str, branch: str, from_sha: str }` | `{ ref, sha }` | REST API `POST /repos/{owner}/{repo}/git/refs` |
| `github.get_branch` | `{ owner: str, repo: str, branch: str }` | `{ name, sha, protected, ahead_by?, behind_by? }` | `octocrab.repos(owner, repo).get_branch(branch)` |
| `github.compare_branches` | `{ owner: str, repo: str, base: str, head: str }` | `{ ahead_by, behind_by, status, files: [{ filename, status, additions, deletions }] }` | REST API `GET /repos/{owner}/{repo}/compare/{base}...{head}` |

#### CI tools (`tools/actions.rs`)

| Tool | Input | Output | Implementation |
|---|---|---|---|
| `github.get_actions_status` | `{ owner: str, repo: str, ref: str }` | `{ state: "success"\|"failure"\|"pending", statuses: [{ context, state, description }], check_runs: [{ name, status, conclusion }] }` | `octocrab.repos(owner, repo).combined_status(ref)` + check runs |

### Auth (`auth.rs`)

```rust
pub fn create_client() -> anyhow::Result<octocrab::Octocrab> {
    let token = std::env::var("GITHUB_TOKEN")
        .map_err(|_| anyhow::anyhow!("GITHUB_TOKEN env var required"))?;
    let client = octocrab::Octocrab::builder()
        .personal_token(token)
        .build()?;
    Ok(client)
}
```

### Rate limiting (`rate_limit.rs`)

```rust
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct RateLimiter {
    remaining: Arc<AtomicU32>,
    reset_at: Arc<AtomicU32>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            remaining: Arc::new(AtomicU32::new(5000)),
            reset_at: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn update_from_headers(&self, remaining: u32, reset: u32) {
        self.remaining.store(remaining, Ordering::Relaxed);
        self.reset_at.store(reset, Ordering::Relaxed);
    }

    pub async fn wait_if_needed(&self) {
        let remaining = self.remaining.load(Ordering::Relaxed);
        if remaining < 10 {
            let reset = self.reset_at.load(Ordering::Relaxed);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32;
            if reset > now {
                let wait = reset - now;
                tracing::warn!(wait_secs = wait, "rate limit near, waiting");
                tokio::time::sleep(std::time::Duration::from_secs(wait as u64)).await;
            }
        }
    }
}
```

### Checklist

- [ ] Create `crates/roko-mcp-github/` directory structure
- [ ] Write `Cargo.toml` with dependencies
- [ ] Add `"crates/roko-mcp-github"` to workspace `Cargo.toml` members
- [ ] Implement MCP protocol handler (`main.rs` — stdio JSON-RPC loop)
- [ ] Implement `initialize` / `tools/list` / `tools/call` dispatch
- [ ] Implement `auth.rs` — `GITHUB_TOKEN` client creation
- [ ] Implement `rate_limit.rs` — header tracking + backoff
- [ ] Implement `github.list_prs`
- [ ] Implement `github.get_pr` (with `include_diff` flag)
- [ ] Implement `github.create_pr`
- [ ] Implement `github.comment_pr`
- [ ] Implement `github.review_pr` (with inline comments)
- [ ] Implement `github.merge_pr`
- [ ] Implement `github.list_issues`
- [ ] Implement `github.create_issue`
- [ ] Implement `github.comment_issue`
- [ ] Implement `github.close_issue`
- [ ] Implement `github.add_labels`
- [ ] Implement `github.create_label`
- [ ] Implement `github.get_file`
- [ ] Implement `github.search_code`
- [ ] Implement `github.list_commits`
- [ ] Implement `github.create_branch`
- [ ] Implement `github.get_branch`
- [ ] Implement `github.compare_branches`
- [ ] Implement `github.get_actions_status`
- [ ] **Verify:** `cargo build -p roko-mcp-github`
- [ ] **Verify:** `echo '{"method":"initialize","params":{}}' | cargo run -p roko-mcp-github` returns server info
- [ ] **Verify:** `echo '{"method":"tools/list","params":{}}' | cargo run -p roko-mcp-github` lists all tools
- [ ] **Verify:** Test `github.list_prs` against a real repo
- [ ] **Verify:** Test `github.get_pr` with `include_diff: true`
- [ ] **Verify:** Test `github.create_issue` + `github.close_issue` round-trip
- [ ] **Verify:** Connect via MCP inspector, verify all tools callable

---

## 2.2 `roko-mcp-slack` — Slack Web API as MCP tools

### Crate structure

```
crates/roko-mcp-slack/
├── Cargo.toml
├── src/
│   ├── main.rs          # MCP server entry (stdio transport)
│   ├── tools/
│   │   ├── mod.rs       # Tool registry + dispatch
│   │   ├── messages.rs  # Post, reply, update, search
│   │   ├── channels.rs  # List, info
│   │   └── files.rs     # Upload
│   ├── auth.rs          # Bot token management
│   └── blocks.rs        # Block Kit helper builder
```

### `Cargo.toml`

```toml
[package]
name = "roko-mcp-slack"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "MCP server exposing Slack Web API as tools for roko agents"

[[bin]]
name = "roko-mcp-slack"
path = "src/main.rs"

[dependencies]
reqwest = { version = "0.12", features = ["json"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Tool definitions

| Tool | Input | Output | Slack API Method |
|---|---|---|---|
| `slack.post_message` | `{ channel: str, text: str, blocks?: Value, thread_ts?: str, unfurl_links?: bool }` | `{ ok: bool, channel: str, ts: str, message: Value }` | `chat.postMessage` |
| `slack.reply_thread` | `{ channel: str, thread_ts: str, text: str, blocks?: Value }` | `{ ok: bool, ts: str }` | `chat.postMessage` (with `thread_ts`) |
| `slack.update_message` | `{ channel: str, ts: str, text: str, blocks?: Value }` | `{ ok: bool, ts: str }` | `chat.update` |
| `slack.react` | `{ channel: str, timestamp: str, name: str }` | `{ ok: bool }` | `reactions.add` |
| `slack.get_thread` | `{ channel: str, ts: str, limit?: u32 }` | `{ messages: [{ user, text, ts, thread_ts?, reactions? }] }` | `conversations.replies` |
| `slack.list_channels` | `{ types?: str, limit?: u32, cursor?: str }` | `{ channels: [{ id, name, topic, purpose, num_members }], next_cursor?: str }` | `conversations.list` |
| `slack.search_messages` | `{ query: str, count?: u32, sort?: "score"\|"timestamp" }` | `{ messages: { matches: [{ channel, text, ts, user, permalink }] } }` | `search.messages` |
| `slack.upload_file` | `{ channels: str, content: str, filename: str, title?: str, filetype?: str }` | `{ ok: bool, file: { id, permalink } }` | `files.upload` |

### Block Kit helper (`blocks.rs`)

```rust
//! Helpers for building Slack Block Kit structures.

use serde_json::{json, Value};

pub fn section(text: &str) -> Value {
    json!({
        "type": "section",
        "text": { "type": "mrkdwn", "text": text }
    })
}

pub fn header(text: &str) -> Value {
    json!({
        "type": "header",
        "text": { "type": "plain_text", "text": text }
    })
}

pub fn divider() -> Value {
    json!({ "type": "divider" })
}

pub fn context(elements: Vec<&str>) -> Value {
    json!({
        "type": "context",
        "elements": elements.iter().map(|t| json!({
            "type": "mrkdwn",
            "text": t
        })).collect::<Vec<_>>()
    })
}

pub fn actions(buttons: Vec<(&str, &str, &str)>) -> Value {
    json!({
        "type": "actions",
        "elements": buttons.iter().map(|(text, action_id, value)| json!({
            "type": "button",
            "text": { "type": "plain_text", "text": text },
            "action_id": action_id,
            "value": value
        })).collect::<Vec<_>>()
    })
}
```

### Auth (`auth.rs`)

```rust
pub struct SlackClient {
    client: reqwest::Client,
    token: String,
}

impl SlackClient {
    pub fn from_env() -> anyhow::Result<Self> {
        let token = std::env::var("SLACK_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("SLACK_BOT_TOKEN env var required"))?;
        Ok(Self {
            client: reqwest::Client::new(),
            token,
        })
    }

    pub async fn api_call(&self, method: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let url = format!("https://slack.com/api/{method}");
        let resp = self.client
            .post(&url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let error = resp.get("error").and_then(|e| e.as_str()).unwrap_or("unknown");
            anyhow::bail!("Slack API error: {error}");
        }

        Ok(resp)
    }
}
```

### Checklist

- [ ] Create `crates/roko-mcp-slack/` directory structure
- [ ] Write `Cargo.toml`
- [ ] Add to workspace members
- [ ] Implement MCP stdio protocol handler
- [ ] Implement `auth.rs` — `SLACK_BOT_TOKEN` client
- [ ] Implement `blocks.rs` — Block Kit helpers
- [ ] Implement `slack.post_message`
- [ ] Implement `slack.reply_thread`
- [ ] Implement `slack.update_message`
- [ ] Implement `slack.react`
- [ ] Implement `slack.get_thread`
- [ ] Implement `slack.list_channels`
- [ ] Implement `slack.search_messages`
- [ ] Implement `slack.upload_file`
- [ ] **Verify:** `cargo build -p roko-mcp-slack`
- [ ] **Verify:** `tools/list` returns all 8 tools
- [ ] **Verify:** Post a test message to a test channel
- [ ] **Verify:** Reply in thread, verify thread context
- [ ] **Verify:** Add reaction to posted message

---

## 2.3 `roko-mcp-scripts` — Generic script wrapper

> The most important MCP server for extensibility. Any script in any language
> becomes an MCP tool via TOML configuration. Zero code required.

### Crate structure

```
crates/roko-mcp-scripts/
├── Cargo.toml
├── src/
│   ├── main.rs          # MCP server entry
│   ├── config.rs        # Parse .roko/scripts.toml
│   ├── executor.rs      # Script execution + timeout
│   └── discovery.rs     # Auto-discovery from scripts/ dir
```

### `Cargo.toml`

```toml
[package]
name = "roko-mcp-scripts"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "MCP server that wraps arbitrary scripts as tools"

[[bin]]
name = "roko-mcp-scripts"
path = "src/main.rs"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
glob = "0.3"
```

### Config format (`.roko/scripts.toml`)

```toml
# Each [[tool]] entry wraps a script as an MCP tool.
# Scripts receive JSON input on stdin, produce output on stdout.
# Non-zero exit → tool error. stderr captured for diagnostics.

[[tool]]
name = "validate_frontmatter"
description = "Validate YAML frontmatter in markdown files. Returns list of errors."
command = "python3"
args = ["scripts/validate-frontmatter.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 30
input_schema = { type = "object", properties = { path = { type = "string", description = "Path to markdown file or directory" } } }

[[tool]]
name = "gen_digest"
description = "Generate a digest of recent document changes. Returns markdown summary."
command = "python3"
args = ["scripts/gen-digest.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 120
input_schema = { type = "object", properties = { days = { type = "integer", description = "Number of days to look back", default = 7 } } }

[[tool]]
name = "gen_index"
description = "Regenerate the repository index file from current documents."
command = "python3"
args = ["scripts/gen-index.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 60

[[tool]]
name = "extract_actions"
description = "Extract action items from a document or set of documents."
command = "python3"
args = ["scripts/extract-actions.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 60
input_schema = { type = "object", properties = { path = { type = "string" } } }

[[tool]]
name = "detect_conflicts"
description = "Detect contradictions or conflicts between documents."
command = "python3"
args = ["scripts/detect-conflicts.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 120
input_schema = { type = "object", properties = { paths = { type = "array", items = { type = "string" } } } }

[[tool]]
name = "freshness_check"
description = "Check for stale documents that need updating."
command = "python3"
args = ["scripts/freshness-check.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 60

[[tool]]
name = "linear_sync"
description = "Sync Linear issues to/from the repository."
command = "python3"
args = ["scripts/linear-sync.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 120
env = { LINEAR_API_KEY = "${LINEAR_API_KEY}" }

[[tool]]
name = "fireflies_sync"
description = "Sync Fireflies.ai meeting transcripts to call-notes/."
command = "python3"
args = ["scripts/fireflies-sync.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 120
env = { FIREFLIES_API_KEY = "${FIREFLIES_API_KEY}" }

[[tool]]
name = "promote"
description = "Promote a document from draft to proposed or proposed to canonical."
command = "python3"
args = ["scripts/promote.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 30
input_schema = { type = "object", properties = { path = { type = "string" }, target_status = { type = "string", enum = ["proposed", "canonical"] } }, required = ["path", "target_status"] }

[[tool]]
name = "manage_domains"
description = "List, add, or modify domain classifications."
command = "python3"
args = ["scripts/manage-domains.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 30

[[tool]]
name = "backfill_frontmatter"
description = "Add missing YAML frontmatter fields to documents."
command = "python3"
args = ["scripts/backfill-frontmatter.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 60

[[tool]]
name = "update_log"
description = "Update the changelog/log for recent changes."
command = "python3"
args = ["scripts/update-log.py"]
working_dir = "/Users/will/dev/nunchi/collaboration"
timeout_secs = 30
```

### Knowledge-base scripts.toml

```toml
# /Users/will/dev/nunchi/knowledge-base/.roko/scripts.toml

[[tool]]
name = "pm_sync"
description = "Sync GitHub issues and PRs to TOML task files. Bidirectional."
command = "python3"
args = ["scripts/pm-sync.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 120
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }
input_schema = { type = "object", properties = { direction = { type = "string", enum = ["pull", "push", "both"], default = "both" } } }

[[tool]]
name = "pm_views"
description = "Generate PM board views (board, health, people, timeline)."
command = "python3"
args = ["scripts/pm-views.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 60
input_schema = { type = "object", properties = { view = { type = "string", enum = ["board", "health", "people", "timeline", "all"], default = "all" } } }

[[tool]]
name = "pm_board"
description = "Generate the project management board view."
command = "python3"
args = ["scripts/pm-board.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 60

[[tool]]
name = "pm_enrich"
description = "Enrich PM tasks with LLM analysis (priority, complexity, dependencies)."
command = "python3"
args = ["scripts/pm-enrich.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 180
env = { ANTHROPIC_API_KEY = "${ANTHROPIC_API_KEY}" }
input_schema = { type = "object", properties = { task_id = { type = "string", description = "Task ID to enrich, or 'all'" } } }

[[tool]]
name = "pm_validate"
description = "Validate PM TOML files for schema compliance and referential integrity."
command = "python3"
args = ["scripts/pm-validate.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 30

[[tool]]
name = "deep_enrich"
description = "Deep enrichment of documents with cross-repo context and citations."
command = "python3"
args = ["scripts/deep-enrich.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 180
env = { ANTHROPIC_API_KEY = "${ANTHROPIC_API_KEY}" }
input_schema = { type = "object", properties = { path = { type = "string" } } }

[[tool]]
name = "local_enrich"
description = "Enrich documents using only local context (no API calls)."
command = "python3"
args = ["scripts/local-enrich.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 60
input_schema = { type = "object", properties = { path = { type = "string" } } }

[[tool]]
name = "notion_sync"
description = "Sync documents to/from Notion workspace."
command = "python3"
args = ["scripts/notion-sync.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 120
env = { NOTION_TOKEN = "${NOTION_TOKEN}" }

[[tool]]
name = "validate_frontmatter"
description = "Validate YAML frontmatter in markdown files."
command = "python3"
args = ["scripts/validate-frontmatter.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 30

[[tool]]
name = "gen_digest"
description = "Generate a digest of recent changes."
command = "python3"
args = ["scripts/gen-digest.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 120

[[tool]]
name = "gen_index"
description = "Regenerate the repository index."
command = "python3"
args = ["scripts/gen-index.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 60

[[tool]]
name = "extract_actions"
description = "Extract action items from documents."
command = "python3"
args = ["scripts/extract-actions.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 60

[[tool]]
name = "detect_conflicts"
description = "Detect contradictions between documents."
command = "python3"
args = ["scripts/detect-conflicts.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 120

[[tool]]
name = "fireflies_sync"
description = "Sync meeting transcripts from Fireflies.ai."
command = "python3"
args = ["scripts/fireflies-sync.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 120
env = { FIREFLIES_API_KEY = "${FIREFLIES_API_KEY}" }

[[tool]]
name = "linear_sync"
description = "Sync Linear issues to the repository."
command = "python3"
args = ["scripts/linear-sync.py"]
working_dir = "/Users/will/dev/nunchi/knowledge-base"
timeout_secs = 120
env = { LINEAR_API_KEY = "${LINEAR_API_KEY}" }
```

### Script executor (`executor.rs`)

```rust
//! Execute a script as an MCP tool call.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

pub struct ScriptResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub async fn run_script(
    command: &str,
    args: &[String],
    working_dir: &Path,
    env: &[(String, String)],
    input: &serde_json::Value,
    timeout: Duration,
) -> anyhow::Result<ScriptResult> {
    let mut cmd = Command::new(command);
    cmd.args(args)
       .current_dir(working_dir)
       .stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());

    for (key, value) in env {
        let resolved = resolve_env(value);
        cmd.env(key, resolved);
    }

    let mut child = cmd.spawn()?;

    // Write input as JSON to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let input_bytes = serde_json::to_vec(input)?;
        stdin.write_all(&input_bytes).await?;
        drop(stdin);
    }

    // Wait with timeout
    let output = tokio::time::timeout(timeout, child.wait_with_output()).await
        .map_err(|_| anyhow::anyhow!("script timed out after {}s", timeout.as_secs()))??;

    Ok(ScriptResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

fn resolve_env(value: &str) -> String {
    if let Some(var) = value.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        std::env::var(var).unwrap_or_default()
    } else {
        value.to_string()
    }
}
```

### Auto-discovery (`discovery.rs`)

```rust
//! Auto-discover scripts in a directory and generate tool configs.

use std::path::Path;

use crate::config::ToolConfig;

pub fn discover_scripts(scripts_dir: &Path) -> Vec<ToolConfig> {
    let mut tools = Vec::new();

    let entries = match std::fs::read_dir(scripts_dir) {
        Ok(e) => e,
        Err(_) => return tools,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }

        let filename = path.file_name().unwrap().to_string_lossy();

        // Skip non-script files
        let (command, _ext) = if filename.ends_with(".py") {
            ("python3".to_string(), "py")
        } else if filename.ends_with(".sh") {
            ("bash".to_string(), "sh")
        } else if filename.ends_with(".js") {
            ("node".to_string(), "js")
        } else if filename.ends_with(".ts") {
            ("npx".to_string(), "ts")
        } else {
            continue;
        };

        // Skip hooks and utility scripts
        if filename.starts_with("pre-") || filename.starts_with("post-")
            || filename == "setup.sh" || filename == "paths.py"
        {
            continue;
        }

        let name = path.file_stem().unwrap().to_string_lossy()
            .replace('-', "_")
            .replace('.', "_");

        let description = format!("Run {filename}");

        tools.push(ToolConfig {
            name,
            description,
            command,
            args: vec![format!("scripts/{filename}")],
            working_dir: scripts_dir.parent().unwrap().to_path_buf(),
            timeout_secs: 60,
            input_schema: None,
            env: Default::default(),
        });
    }

    tools
}
```

### Checklist

- [ ] Create `crates/roko-mcp-scripts/` directory structure
- [ ] Write `Cargo.toml`
- [ ] Add to workspace members
- [ ] Implement `config.rs` — parse `.roko/scripts.toml`
- [ ] Implement `executor.rs` — script execution with timeout
- [ ] Implement `discovery.rs` — auto-discovery from scripts/ dir
- [ ] Implement MCP stdio protocol handler in `main.rs`
- [ ] Write `collaboration/.roko/scripts.toml` (11 tools)
- [ ] Write `knowledge-base/.roko/scripts.toml` (16 tools)
- [ ] **Verify:** `cargo build -p roko-mcp-scripts`
- [ ] **Verify:** `tools/list` returns all configured tools
- [ ] **Verify:** Call `validate_frontmatter` via MCP → runs script, returns output
- [ ] **Verify:** Call `pm_sync` via MCP → syncs GitHub state
- [ ] **Verify:** Timeout works (script exceeding timeout is killed)
- [ ] **Verify:** Env var interpolation works (`${GITHUB_TOKEN}` → actual value)
- [ ] **Verify:** Auto-discovery mode generates correct config from scripts/ dir

---

## 2.4 MCP server configuration and discovery

### `.roko/mcp-servers.toml`

Agents discover MCP servers via this config file:

```toml
# Well-known MCP servers (shipped with roko)
[servers.github]
command = "roko-mcp-github"
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }

[servers.slack]
command = "roko-mcp-slack"
env = { SLACK_BOT_TOKEN = "${SLACK_BOT_TOKEN}" }

[servers.scripts]
command = "roko-mcp-scripts"
args = ["--config", ".roko/scripts.toml"]

# Custom MCP servers
[servers.custom-db]
command = "/path/to/my-mcp-server"
args = ["--port", "3000"]
env = { DB_URL = "${DATABASE_URL}" }
```

### How templates reference MCP servers

In an agent template:
```toml
mcp_servers = ["github", "slack", "scripts"]
```

When the dispatch loop spawns an agent, it:
1. Looks up each server name in `mcp-servers.toml`
2. Generates a temporary MCP config JSON file
3. Passes it via `--mcp-config /tmp/roko-mcp-{hash}.json` to the agent

### Auto-start behavior

`roko serve` auto-starts MCP servers on first use and keeps them alive.
ProcessSupervisor tracks them. They shut down on server shutdown.

### Checklist

- [ ] Define `MCP server config` schema
- [ ] Implement config loading in AppState
- [ ] Implement dynamic MCP config generation per agent
- [ ] Implement auto-start + lifecycle via ProcessSupervisor
- [ ] Write `.roko/mcp-servers.toml` for each repo
- [ ] **Verify:** Template with `mcp_servers = ["github"]` → agent gets GitHub tools
- [ ] **Verify:** MCP server auto-starts on first agent dispatch
- [ ] **Verify:** MCP server shuts down on `roko serve` shutdown
