<!-- Master Plan: Tier 2, Sections 2A-2C -->
<!-- Status: Not started -->
<!-- Depends on: Tier 1 complete -->

# Phase 0: Architectural Preparation

> **Master Plan Reference**: Tier 2, Sections 2A-2C
> **Status**: Not started
> **Depends on**: Tier 1 complete
> **Blocks**: Tier 3 (templates), Tier 4 (daemon)
>
> ### What Already Exists in Codebase
> - `crates/roko-cli/src/serve/` — full HTTP server (21 files)
> - `crates/roko-cli/src/serve/routes/` — all route handlers
> - `crates/roko-cli/src/serve/deploy/` — Railway deployment
> - `crates/roko-cli/src/serve/events.rs` — event system
> - `crates/roko-agent/src/dispatcher/mod.rs` — agent dispatcher
>
> ### Reference Material
> - Mori HTTP server: `/Users/will/dev/uniswap/bardo/apps/mori/src/serve/`
> - Agent spawn reference: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2444-2620`

## 0.1 Extract `roko-serve` crate from `roko-cli`

**Goal:** Make the HTTP server a reusable library so external crates can build on it.

### Files to move

| Source (in `crates/roko-cli/src/serve/`) | Destination (in `crates/roko-serve/src/`) |
|---|---|
| `mod.rs` | `lib.rs` (rename `pub mod` → `pub mod`, expose `run_server`) |
| `state.rs` | `state.rs` |
| `events.rs` | `events.rs` |
| `error.rs` | `error.rs` |
| `templates.rs` | `templates.rs` |
| `routes/mod.rs` | `routes/mod.rs` |
| `routes/status.rs` | `routes/status.rs` |
| `routes/run.rs` | `routes/run.rs` |
| `routes/plans.rs` | `routes/plans.rs` |
| `routes/prds.rs` | `routes/prds.rs` |
| `routes/research.rs` | `routes/research.rs` |
| `routes/templates.rs` | `routes/templates.rs` |
| `routes/agents.rs` | `routes/agents.rs` |
| `routes/learning.rs` | `routes/learning.rs` |
| `routes/config.rs` | `routes/config.rs` |
| `routes/deployments.rs` | `routes/deployments.rs` |
| `routes/ws.rs` | `routes/ws.rs` |
| `deploy/mod.rs` | `deploy/mod.rs` |
| `deploy/manual.rs` | `deploy/manual.rs` |
| `deploy/railway_api.rs` | `deploy/railway_api.rs` |
| `deploy/railway_cli.rs` | `deploy/railway_cli.rs` |

### New `crates/roko-serve/Cargo.toml`

```toml
[package]
name = "roko-serve"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
roko-core = { path = "../roko-core" }
roko-agent = { path = "../roko-agent" }
roko-learn = { path = "../roko-learn" }
roko-gate = { path = "../roko-gate" }
roko-fs = { path = "../roko-fs" }
roko-compose = { path = "../roko-compose" }
roko-orchestrator = { path = "../roko-orchestrator" }
roko-conductor = { path = "../roko-conductor" }
bardo-runtime = { path = "../bardo-runtime" }

axum = { version = "0.8", features = ["ws"] }
tokio = { workspace = true }
tower-http = { version = "0.6", features = ["cors", "trace"] }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
uuid = { version = "1", features = ["v4"] }
```

### Changes to `crates/roko-cli/`

`roko-cli/Cargo.toml` adds `roko-serve = { path = "../roko-serve" }` and removes the direct axum/tower-http deps.

`roko-cli/src/serve/` becomes a thin re-export:
```rust
// crates/roko-cli/src/serve.rs (replaces the directory)
pub use roko_serve::run_server;
```

`main.rs` changes from:
```rust
roko_cli::serve::run_server(wd, bind, port).await?;
```
to:
```rust
roko_serve::run_server(wd, bind, port).await?;
```

### Checklist

- [ ] Create `crates/roko-serve/` directory structure
- [ ] Write `Cargo.toml` with all dependencies
- [ ] Move all 21 files listed above
- [ ] Update `use crate::serve::` → `use crate::` in moved files
- [ ] Update cross-references (e.g., `crate::status::SessionStatus` → add re-export or adjust)
- [ ] Add `"crates/roko-serve"` to workspace `Cargo.toml` members
- [ ] Update `crates/roko-cli/Cargo.toml` to depend on `roko-serve`
- [ ] Replace `crates/roko-cli/src/serve/` with thin re-export
- [ ] **Verify:** `cargo build --workspace` passes
- [ ] **Verify:** `cargo test -p roko-serve` passes
- [ ] **Verify:** `cargo run -p roko-cli -- serve --port 9090` starts and serves `/api/health`

---

## 0.2 Create `roko-plugin` crate — the integration SDK

**Goal:** A tiny, stable crate that external developers implement to build integrations. Minimal dependencies.

### `crates/roko-plugin/Cargo.toml`

```toml
[package]
name = "roko-plugin"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Plugin SDK for building roko integrations"

[dependencies]
roko-core = { path = "../roko-core" }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
tokio = { workspace = true }
```

### `crates/roko-plugin/src/lib.rs`

```rust
//! Plugin SDK for building roko integrations.
//!
//! External developers implement [`Integration`] to provide event sources,
//! feedback collectors, and MCP tool servers. The roko runtime discovers
//! and loads integrations at startup.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use bardo_runtime::cancel::CancelToken;

// Re-export core types that plugins need
pub use roko_core::signal::Signal;
pub use roko_core::kind::Kind;

/// Channel for event sources to emit signals into the runtime.
///
/// Wraps a `tokio::sync::mpsc::UnboundedSender<Signal>`.
/// Event sources call `sender.send(signal)` to inject events.
#[derive(Clone)]
pub struct SignalSender {
    inner: tokio::sync::mpsc::UnboundedSender<Signal>,
}

impl SignalSender {
    pub fn new(inner: tokio::sync::mpsc::UnboundedSender<Signal>) -> Self {
        Self { inner }
    }

    pub fn send(&self, signal: Signal) -> Result<()> {
        self.inner.send(signal).map_err(|e| anyhow::anyhow!("send failed: {e}"))?;
        Ok(())
    }
}

/// An event source produces signals from external systems.
///
/// Implementations run as long-lived async tasks, emitting signals via
/// the provided [`SignalSender`]. The runtime cancels them via [`CancelToken`]
/// on shutdown.
///
/// # Examples
///
/// - GitHub webhook receiver
/// - Cron scheduler
/// - File system watcher
/// - Slack event listener
pub trait EventSource: Send + Sync + 'static {
    /// Human-readable name (e.g., "github-webhooks", "cron-scheduler").
    fn name(&self) -> &str;

    /// Signal kind prefixes this source can produce.
    /// Used for documentation, validation, and subscription routing.
    fn produces(&self) -> Vec<String>;

    /// Start the event source. Runs until cancelled.
    ///
    /// Implementations should:
    /// 1. Set up their listener (HTTP server, file watcher, timer, etc.)
    /// 2. Loop, converting external events to Signals
    /// 3. Send each Signal via `sender.send(signal)`
    /// 4. Return when `cancel` fires
    fn start(
        &self,
        sender: SignalSender,
        cancel: CancelToken,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
}

/// A feedback collector polls external systems for outcomes of past agent actions.
///
/// Unlike event sources (which are push-based), feedback collectors are pull-based:
/// they periodically check for engagement on past agent outputs (PR reviews, Slack
/// reactions, issue state changes, etc.).
pub trait FeedbackCollector: Send + Sync + 'static {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Services this collector monitors (e.g., "github", "slack").
    fn services(&self) -> Vec<String>;

    /// Poll for feedback on recent agent actions.
    ///
    /// `recent_actions` contains external actions from recent episodes.
    /// Returns feedback signals to ingest into the learning loop.
    fn collect(
        &self,
        recent_actions: &[ExternalAction],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<FeedbackSignal>>> + Send>>;
}

/// An integration bundles event sources, feedback collectors, and MCP server config.
///
/// This is the main trait external developers implement. It groups related
/// capabilities and provides configuration validation.
pub trait Integration: Send + Sync + 'static {
    /// Unique integration name (e.g., "github", "slack", "linear").
    fn name(&self) -> &str;

    /// SemVer version string.
    fn version(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Event sources this integration provides.
    fn event_sources(&self) -> Vec<Box<dyn EventSource>>;

    /// Feedback collectors this integration provides.
    fn feedback_collectors(&self) -> Vec<Box<dyn FeedbackCollector>>;

    /// MCP server specification, if this integration provides tools.
    fn mcp_server(&self) -> Option<McpServerSpec>;

    /// JSON Schema for this integration's configuration.
    fn config_schema(&self) -> serde_json::Value;

    /// Validate configuration. Called on startup before any sources/collectors start.
    fn validate_config(&self, config: &serde_json::Value) -> Result<()>;
}

/// Specification for starting an MCP server process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerSpec {
    /// Command to execute (e.g., path to binary).
    pub command: String,
    /// Arguments to pass.
    pub args: Vec<String>,
    /// Environment variables to set.
    pub env: HashMap<String, String>,
    /// Working directory (optional).
    pub working_dir: Option<PathBuf>,
}

/// Record of an external action performed by an agent.
///
/// Used by feedback collectors to know what to poll for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAction {
    /// Service name (e.g., "github", "slack").
    pub service: String,
    /// Action type (e.g., "comment_pr", "post_message", "create_issue").
    pub action_type: String,
    /// Unique resource identifier (e.g., PR URL, Slack channel+ts).
    pub resource_id: String,
    /// Timestamp when the action was performed.
    pub performed_at: DateTime<Utc>,
    /// Additional metadata (varies by service).
    pub metadata: serde_json::Value,
}

/// Feedback signal produced by a feedback collector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackSignal {
    /// The original action this feedback is about.
    pub action: ExternalAction,
    /// Engagement metrics.
    pub engagement: Engagement,
    /// When feedback was collected.
    pub collected_at: DateTime<Utc>,
}

/// Quantified engagement on an external action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engagement {
    /// Overall sentiment: -1.0 (negative) to 1.0 (positive).
    pub sentiment: f64,
    /// Whether the action was acknowledged (reacted to, replied to, etc.).
    pub acknowledged: bool,
    /// Whether the intended outcome was achieved (issue closed, PR merged, etc.).
    pub outcome_achieved: bool,
    /// Service-specific metrics.
    pub details: serde_json::Value,
}

/// Summary of a recent episode, provided to feedback collectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSummary {
    pub episode_id: String,
    pub agent_template: String,
    pub trigger_kind: String,
    pub external_actions: Vec<ExternalAction>,
    pub completed_at: DateTime<Utc>,
    pub success: bool,
}
```

### Checklist

- [ ] Create `crates/roko-plugin/` directory
- [ ] Write `Cargo.toml`
- [ ] Write `src/lib.rs` with all traits and types
- [ ] Add `"crates/roko-plugin"` to workspace members
- [ ] **Verify:** `cargo build -p roko-plugin`
- [ ] **Verify:** `cargo doc -p roko-plugin --open` — docs are clear and complete
- [ ] **Verify:** Create a minimal test: `impl Integration for DummyIntegration` compiles

---

## 0.3 Signal kind constants

**Goal:** Well-known string constants for event kinds, with prefix-matching utility.

### `crates/roko-core/src/kind.rs` — add module

Add at the bottom of the existing `kind.rs`:

```rust
/// Well-known signal kind constants for the subscription system.
///
/// These use the `Kind::Custom(String)` escape hatch, so no enum changes are needed.
/// The dot-separated hierarchy enables prefix matching:
/// subscribing to `"webhook.github"` catches all GitHub events.
pub mod well_known {
    // ── Webhooks ──────────────────────────────────────────────────────
    pub const GITHUB_PUSH: &str = "webhook.github.push";
    pub const GITHUB_PR: &str = "webhook.github.pull_request";
    pub const GITHUB_ISSUE: &str = "webhook.github.issues";
    pub const GITHUB_COMMENT: &str = "webhook.github.issue_comment";
    pub const GITHUB_REVIEW: &str = "webhook.github.pull_request_review";
    pub const GITHUB_ACTIONS: &str = "webhook.github.check_run";
    pub const GITHUB_RELEASE: &str = "webhook.github.release";
    pub const GITHUB_STAR: &str = "webhook.github.star";

    pub const SLACK_MESSAGE: &str = "webhook.slack.message";
    pub const SLACK_COMMAND: &str = "webhook.slack.slash_command";
    pub const SLACK_REACTION: &str = "webhook.slack.reaction_added";
    pub const SLACK_CHANNEL_JOIN: &str = "webhook.slack.member_joined_channel";

    // ── Schedulers ────────────────────────────────────────────────────
    pub const CRON_TICK: &str = "scheduler.cron";

    // ── Watchers ──────────────────────────────────────────────────────
    pub const FS_CHANGE: &str = "watcher.fs_change";

    // ── Feedback ──────────────────────────────────────────────────────
    pub const FEEDBACK_PR: &str = "feedback.github.pr_engagement";
    pub const FEEDBACK_ISSUE: &str = "feedback.github.issue_engagement";
    pub const FEEDBACK_SLACK: &str = "feedback.slack.message_engagement";
    pub const FEEDBACK_TRIAGE: &str = "feedback.github.issue_triage";

    // ── Agent lifecycle ───────────────────────────────────────────────
    pub const AGENT_COMPLETED: &str = "agent.completed";
    pub const AGENT_FAILED: &str = "agent.failed";
    pub const AGENT_GATE_PASSED: &str = "agent.gate_passed";
    pub const AGENT_GATE_FAILED: &str = "agent.gate_failed";

    // ── PRD lifecycle ─────────────────────────────────────────────────
    pub const PRD_PUBLISHED: &str = "prd.published";
    pub const PRD_PLAN_GENERATED: &str = "prd.plan_generated";
    pub const PRD_PLAN_APPROVED: &str = "prd.plan_approved";

    /// Check if a signal kind string matches a subscription pattern.
    ///
    /// Supports exact match and prefix match:
    /// - `matches("webhook.github.push", "webhook.github.push")` → true
    /// - `matches("webhook.github", "webhook.github.push")` → true
    /// - `matches("webhook", "webhook.github.push")` → true
    /// - `matches("webhook.slack", "webhook.github.push")` → false
    pub fn matches(pattern: &str, kind: &str) -> bool {
        kind == pattern || kind.starts_with(&format!("{pattern}."))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn exact_match() {
            assert!(matches(GITHUB_PUSH, "webhook.github.push"));
        }

        #[test]
        fn prefix_match() {
            assert!(matches("webhook.github", "webhook.github.push"));
            assert!(matches("webhook", "webhook.github.push"));
        }

        #[test]
        fn no_match() {
            assert!(!matches("webhook.slack", "webhook.github.push"));
            assert!(!matches("webhook.github.push.extra", "webhook.github.push"));
        }

        #[test]
        fn all_constants_are_dotted() {
            let all = [
                GITHUB_PUSH, GITHUB_PR, GITHUB_ISSUE, GITHUB_COMMENT,
                GITHUB_REVIEW, GITHUB_ACTIONS, SLACK_MESSAGE, SLACK_COMMAND,
                SLACK_REACTION, CRON_TICK, FS_CHANGE, FEEDBACK_PR,
                FEEDBACK_SLACK, AGENT_COMPLETED, AGENT_FAILED,
                PRD_PUBLISHED, PRD_PLAN_GENERATED,
            ];
            for k in all {
                assert!(k.contains('.'), "{k} should contain a dot");
            }
        }
    }
}
```

### Wire into re-exports

In `crates/roko-core/src/lib.rs`, add:
```rust
pub use kind::well_known;
```

### Checklist

- [ ] Add `well_known` module to `crates/roko-core/src/kind.rs`
- [ ] Add re-export in `crates/roko-core/src/lib.rs`
- [ ] **Verify:** `cargo test -p roko-core` — new tests pass
- [ ] **Verify:** `use roko_core::well_known::GITHUB_PUSH;` compiles from another crate

---

# Phase 1: Event Ingress & Webhook Endpoints

## 1.1 Webhook route handlers

**Goal:** GitHub and Slack webhooks flow into roko as Signals.

### Config additions

**File:** `crates/roko-core/src/config/schema.rs`

Add to `RokoConfig`:

```rust
/// Configuration for the HTTP server and webhooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServeConfig {
    /// Address to bind to.
    pub bind: String,
    /// Port number.
    pub port: u16,
    /// Webhook configuration.
    pub webhooks: WebhookConfig,
    /// Additional repos to load subscriptions from.
    pub repos: Vec<PathBuf>,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1".into(),
            port: 9090,
            webhooks: WebhookConfig::default(),
            repos: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookConfig {
    /// GitHub webhook secret (HMAC-SHA256 validation).
    /// Supports env var interpolation: "${GITHUB_WEBHOOK_SECRET}"
    pub github_secret: Option<String>,
    /// Slack signing secret.
    pub slack_signing_secret: Option<String>,
    /// Whether to require signature validation (default: true in production).
    pub require_signatures: bool,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            github_secret: None,
            slack_signing_secret: None,
            require_signatures: true,
        }
    }
}
```

**TOML:**
```toml
[serve]
port = 9090
repos = [
    "/Users/will/dev/nunchi/collaboration",
    "/Users/will/dev/nunchi/knowledge-base",
]

[serve.webhooks]
github_secret = "${GITHUB_WEBHOOK_SECRET}"
slack_signing_secret = "${SLACK_SIGNING_SECRET}"
```

### `routes/hooks.rs`

```rust
//! Webhook ingress endpoints for GitHub and Slack.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{State, Json};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde_json::{json, Value};

use roko_core::kind::well_known;
use roko_core::signal::{Signal, Body};
use roko_core::kind::Kind;

use crate::events::ServerEvent;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/hooks/github", post(github_webhook))
        .route("/hooks/slack", post(slack_webhook))
}

/// POST /api/hooks/github
///
/// 1. Validate X-Hub-Signature-256 header
/// 2. Parse X-GitHub-Event header
/// 3. Construct Signal with Kind::Custom("webhook.github.{event}")
/// 4. Emit to EventBus
/// 5. Return 200 OK
async fn github_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // 1. Validate signature
    if let Some(ref secret) = state.config.serve.webhooks.github_secret {
        let secret = resolve_env_var(secret);
        let signature = headers
            .get("x-hub-signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !validate_github_signature(&secret, &body, signature) {
            return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid signature"}))).into_response();
        }
    } else if state.config.serve.webhooks.require_signatures {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "no webhook secret configured"}))).into_response();
    }

    // 2. Parse event type
    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // 3. Parse body as JSON
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": format!("invalid JSON: {e}")}))).into_response(),
    };

    // 4. Construct signal
    let kind_str = format!("webhook.github.{event_type}");
    let signal = Signal::new(
        Kind::Custom(kind_str.clone()),
        Body::Json(payload.clone()),
    );
    let signal_hash = signal.hash.clone();

    // 5. Persist signal
    if let Err(e) = persist_signal(&state, &signal).await {
        tracing::error!("failed to persist webhook signal: {e}");
    }

    // 6. Emit to EventBus
    state.event_bus.emit(ServerEvent::WebhookReceived {
        kind: kind_str,
        signal_hash: signal_hash.clone(),
        source: "github".into(),
    });

    tracing::info!(event = event_type, hash = %signal_hash, "github webhook received");

    (StatusCode::OK, Json(json!({
        "received": true,
        "kind": format!("webhook.github.{event_type}"),
        "signal_hash": signal_hash,
    }))).into_response()
}

/// POST /api/hooks/slack
///
/// Handles both URL verification challenges and event callbacks.
async fn slack_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // 1. Validate Slack signature
    if let Some(ref secret) = state.config.serve.webhooks.slack_signing_secret {
        let secret = resolve_env_var(secret);
        let timestamp = headers
            .get("x-slack-request-timestamp")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("0");
        let signature = headers
            .get("x-slack-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !validate_slack_signature(&secret, timestamp, &body, signature) {
            return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid signature"}))).into_response();
        }
    } else if state.config.serve.webhooks.require_signatures {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "no slack secret configured"}))).into_response();
    }

    // 2. Parse body
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": format!("invalid JSON: {e}")}))).into_response(),
    };

    // 3. Handle URL verification challenge
    if payload.get("type").and_then(|t| t.as_str()) == Some("url_verification") {
        let challenge = payload.get("challenge").and_then(|c| c.as_str()).unwrap_or("");
        return (StatusCode::OK, Json(json!({"challenge": challenge}))).into_response();
    }

    // 4. Extract event type
    let event_type = payload
        .get("event")
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    // 5. Construct signal
    let kind_str = format!("webhook.slack.{event_type}");
    let signal = Signal::new(
        Kind::Custom(kind_str.clone()),
        Body::Json(payload.clone()),
    );
    let signal_hash = signal.hash.clone();

    // 6. Persist + emit
    if let Err(e) = persist_signal(&state, &signal).await {
        tracing::error!("failed to persist webhook signal: {e}");
    }

    state.event_bus.emit(ServerEvent::WebhookReceived {
        kind: kind_str,
        signal_hash: signal_hash.clone(),
        source: "slack".into(),
    });

    tracing::info!(event = event_type, hash = %signal_hash, "slack webhook received");

    (StatusCode::OK, Json(json!({
        "received": true,
        "kind": format!("webhook.slack.{event_type}"),
        "signal_hash": signal_hash,
    }))).into_response()
}

// ── Helpers ──────────────────────────────────────────────────────

fn validate_github_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let expected = signature.strip_prefix("sha256=").unwrap_or(signature);
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    let result = hex::encode(mac.finalize().into_bytes());
    // Constant-time comparison
    constant_time_eq(result.as_bytes(), expected.as_bytes())
}

fn validate_slack_signature(secret: &str, timestamp: &str, body: &[u8], signature: &str) -> bool {
    let basestring = format!("v0:{timestamp}:{}", String::from_utf8_lossy(body));
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(basestring.as_bytes());
    let expected = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn resolve_env_var(value: &str) -> String {
    if let Some(var_name) = value.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        std::env::var(var_name).unwrap_or_default()
    } else {
        value.to_string()
    }
}

async fn persist_signal(state: &AppState, signal: &Signal) -> anyhow::Result<()> {
    let path = state.workdir.join(".roko").join("signals.jsonl");
    let line = serde_json::to_string(signal)? + "\n";
    tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await?
        .write_all(line.as_bytes())
        .await?;
    Ok(())
}
```

### Wire into routes

In `routes/mod.rs`, add:
```rust
mod hooks;
// ...
.merge(hooks::routes())
```

### Additional Cargo.toml dependencies

```toml
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
```

### Checklist

- [ ] Add `WebhookConfig` and `ServeConfig` to `roko-core/src/config/schema.rs`
- [ ] Create `routes/hooks.rs` with both endpoint handlers
- [ ] Add `hmac`, `sha2`, `hex` to `roko-serve/Cargo.toml`
- [ ] Wire `hooks::routes()` into `routes/mod.rs`
- [ ] **Verify:** `cargo build -p roko-serve`
- [ ] **Verify:** Start server, send GitHub webhook with valid signature → 200
- [ ] **Verify:** Send GitHub webhook with invalid signature → 401
- [ ] **Verify:** Send Slack URL verification challenge → echoes challenge
- [ ] **Verify:** Send Slack event → 200, signal appears in `.roko/signals.jsonl`
- [ ] **Verify:** WebSocket client receives `WebhookReceived` event

---

## 1.2 `WebhookReceived` event variant

**File:** `crates/roko-serve/src/events.rs`

Add to `ServerEvent` enum:

```rust
/// An external webhook was received and converted to a signal.
WebhookReceived {
    /// Signal kind (e.g., "webhook.github.push").
    kind: String,
    /// Hash of the persisted signal.
    signal_hash: String,
    /// Source service ("github" or "slack").
    source: String,
},
```

### Checklist

- [ ] Add variant to `ServerEvent`
- [ ] **Verify:** `cargo build -p roko-serve`
- [ ] **Verify:** WebSocket clients see the event when a webhook arrives

---

## 1.3 Subscription registry

**Goal:** Declarative mapping from signal kinds to agent templates.

### Subscription struct

```rust
/// Maps a signal kind pattern to an agent template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Signal kind pattern. Supports prefix matching.
    /// e.g., "webhook.github.pull_request" or "webhook.github" (all GH events)
    pub pattern: String,

    /// Name of the agent template to invoke.
    pub agent_template: String,

    /// Working directory for the agent (for multi-repo setups).
    /// If not set, uses the repo where the subscription was loaded from.
    #[serde(default)]
    pub repo_context: Option<PathBuf>,

    /// JSON filter on the signal body.
    /// Only triggers if body matches (key-value equality).
    /// Array values mean "any of" (OR).
    #[serde(default)]
    pub filter: Option<serde_json::Value>,

    /// Glob pattern on changed file paths (for push events).
    /// Only triggers if at least one changed file matches.
    #[serde(default)]
    pub path_filter: Option<String>,

    /// Cron schedule expression (only for `scheduler.cron` subscriptions).
    #[serde(default)]
    pub schedule: Option<String>,

    /// Path to watch (only for `watcher.fs_change` subscriptions).
    #[serde(default)]
    pub watch_path: Option<PathBuf>,

    /// Glob pattern for file watcher.
    #[serde(default)]
    pub watch_glob: Option<String>,

    /// Maximum concurrent agent instances for this subscription.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,

    /// Minimum seconds between triggers (debounce).
    #[serde(default)]
    pub cooldown_secs: Option<u64>,

    /// Whether this subscription is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_max_concurrent() -> usize { 3 }
fn default_true() -> bool { true }
```

### `.roko/subscriptions.toml` schema

```toml
[[subscription]]
pattern = "webhook.github.pull_request"
agent_template = "pr-review-agent"
filter = { action = ["opened", "synchronize"] }
max_concurrent = 2

[[subscription]]
pattern = "webhook.github.push"
agent_template = "doc-lifecycle-agent"
filter = { ref = "refs/heads/main" }
path_filter = "docs/**/*.md"
repo_context = "/Users/will/dev/nunchi/collaboration"

[[subscription]]
pattern = "scheduler.cron"
agent_template = "digest-agent"
schedule = "0 9 * * MON"
repo_context = "/Users/will/dev/nunchi/collaboration"
```

### Loading logic

In `AppState::new()` or a dedicated `load_subscriptions()`:

```rust
async fn load_subscriptions(config: &RokoConfig, workdir: &Path) -> Vec<Subscription> {
    let mut subs = Vec::new();

    // Load local subscriptions
    let local_path = workdir.join(".roko/subscriptions.toml");
    if let Ok(content) = tokio::fs::read_to_string(&local_path).await {
        if let Ok(file) = toml::from_str::<SubscriptionsFile>(&content) {
            for mut sub in file.subscription {
                if sub.repo_context.is_none() {
                    sub.repo_context = Some(workdir.to_path_buf());
                }
                subs.push(sub);
            }
        }
    }

    // Load from each configured repo
    for repo in &config.serve.repos {
        let repo_path = repo.join(".roko/subscriptions.toml");
        if let Ok(content) = tokio::fs::read_to_string(&repo_path).await {
            if let Ok(file) = toml::from_str::<SubscriptionsFile>(&content) {
                for mut sub in file.subscription {
                    if sub.repo_context.is_none() {
                        sub.repo_context = Some(repo.clone());
                    }
                    subs.push(sub);
                }
            }
        }
    }

    tracing::info!(count = subs.len(), "loaded subscriptions");
    subs
}

#[derive(Deserialize)]
struct SubscriptionsFile {
    #[serde(default)]
    subscription: Vec<Subscription>,
}
```

### Add to AppState

```rust
pub subscriptions: Arc<RwLock<Vec<Subscription>>>,
```

### Checklist

- [ ] Define `Subscription` struct in `state.rs`
- [ ] Implement `load_subscriptions()` function
- [ ] Add `subscriptions` field to `AppState`
- [ ] Load subscriptions in `AppState::new()` / server startup
- [ ] Expose loaded subscriptions via `GET /api/subscriptions`
- [ ] **Verify:** Create `.roko/subscriptions.toml` with test entries
- [ ] **Verify:** Start server, `curl /api/subscriptions` shows loaded entries
- [ ] **Verify:** Multi-repo loading works (add subscription to collaboration repo)

---

## 1.4 Subscription dispatch loop

**Goal:** When a webhook signal arrives, match it against subscriptions and spawn agents.

### `dispatch.rs`

```rust
//! Event → agent dispatch loop.
//!
//! A background task that subscribes to the EventBus, matches incoming
//! webhook signals against registered subscriptions, and spawns agents
//! for matches.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, Semaphore};
use tracing::{info, warn, error};

use roko_core::kind::well_known;

use crate::events::ServerEvent;
use crate::state::{AppState, Subscription};

/// Per-subscription concurrency limiter and cooldown tracker.
struct SubscriptionState {
    semaphore: Arc<Semaphore>,
    last_triggered: RwLock<Option<Instant>>,
}

/// Start the subscription dispatch loop as a background task.
///
/// Returns a JoinHandle for the spawned task.
pub fn start_dispatch_loop(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut rx = state.event_bus.subscribe();
        let mut sub_states: HashMap<String, SubscriptionState> = HashMap::new();

        // Initialize per-subscription state
        {
            let subs = state.subscriptions.read().await;
            for sub in subs.iter() {
                let key = format!("{}:{}", sub.pattern, sub.agent_template);
                sub_states.insert(key, SubscriptionState {
                    semaphore: Arc::new(Semaphore::new(sub.max_concurrent)),
                    last_triggered: RwLock::new(None),
                });
            }
        }

        loop {
            let envelope = match rx.recv().await {
                Ok(env) => env,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, "dispatch loop lagged, missed events");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("event bus closed, dispatch loop exiting");
                    break;
                }
            };

            // Only process WebhookReceived events
            let (kind, signal_hash, source) = match &envelope.event {
                ServerEvent::WebhookReceived { kind, signal_hash, source } => {
                    (kind.clone(), signal_hash.clone(), source.clone())
                }
                _ => continue,
            };

            // Match against subscriptions
            let subs = state.subscriptions.read().await;
            for sub in subs.iter() {
                if !sub.enabled { continue; }
                if !well_known::matches(&sub.pattern, &kind) { continue; }

                // Apply body filter: if the subscription has a JSON filter, load
                // the signal body and check that it matches (key-value equality,
                // arrays mean "any of").
                if let Some(ref filter) = sub.filter {
                    let signal = load_signal(&state.workdir, &signal_hash).await;
                    if let Ok(sig) = &signal {
                        if let roko_core::signal::Body::Json(ref body) = sig.body {
                            if !body_matches_filter(body, filter) {
                                continue;
                            }
                        }
                    }
                }

                // Apply path filter: for push events, check that at least one
                // changed file matches the subscription's glob pattern.
                if let Some(ref path_pattern) = sub.path_filter {
                    let signal = load_signal(&state.workdir, &signal_hash).await;
                    if let Ok(sig) = &signal {
                        if let roko_core::signal::Body::Json(ref body) = sig.body {
                            if !paths_match_filter(body, path_pattern) {
                                continue;
                            }
                        }
                    }
                }

                let key = format!("{}:{}", sub.pattern, sub.agent_template);

                // Cooldown check
                if let Some(cooldown) = sub.cooldown_secs {
                    if let Some(sub_state) = sub_states.get(&key) {
                        let last = sub_state.last_triggered.read().await;
                        if let Some(t) = *last {
                            if t.elapsed() < Duration::from_secs(cooldown) {
                                info!(template = %sub.agent_template, "cooldown active, skipping");
                                continue;
                            }
                        }
                    }
                }

                // Concurrency check
                let sem = sub_states
                    .get(&key)
                    .map(|s| Arc::clone(&s.semaphore))
                    .unwrap_or_else(|| Arc::new(Semaphore::new(sub.max_concurrent)));

                let permit = match sem.try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        warn!(template = %sub.agent_template, "max concurrent reached, skipping");
                        continue;
                    }
                };

                // Update last triggered
                if let Some(sub_state) = sub_states.get(&key) {
                    *sub_state.last_triggered.write().await = Some(Instant::now());
                }

                // Spawn the agent
                let state_clone = Arc::clone(&state);
                let sub_clone = sub.clone();
                let signal_hash_clone = signal_hash.clone();
                let kind_clone = kind.clone();

                tokio::spawn(async move {
                    let _permit = permit; // Hold until agent completes

                    info!(
                        template = %sub_clone.agent_template,
                        kind = %kind_clone,
                        "dispatching agent for webhook"
                    );

                    if let Err(e) = run_agent_for_subscription(
                        &state_clone,
                        &sub_clone,
                        &signal_hash_clone,
                        &kind_clone,
                    ).await {
                        error!(
                            template = %sub_clone.agent_template,
                            error = %e,
                            "agent dispatch failed"
                        );
                    }
                });
            }
        }
    })
}

async fn run_agent_for_subscription(
    state: &AppState,
    sub: &Subscription,
    signal_hash: &str,
    kind: &str,
) -> anyhow::Result<()> {
    // 1. Load the agent template
    let templates = state.templates.read().await;
    let template = templates
        .get(&sub.agent_template)
        .ok_or_else(|| anyhow::anyhow!("template not found: {}", sub.agent_template))?
        .clone();
    drop(templates);

    // 2. Determine working directory
    let workdir = sub.repo_context
        .as_ref()
        .unwrap_or(&state.workdir);

    // 3. Build system prompt with trigger context
    let prompt = format!(
        "{}\n\n## Trigger\nEvent kind: {}\nSignal hash: {}",
        template.system_prompt, kind, signal_hash
    );

    // 4. Spawn agent process via existing dispatch machinery
    // (Uses ProcessSupervisor for lifecycle management)
    let process_id = state.supervisor.spawn(
        &sub.agent_template,
        &template.model,
        &prompt,
        workdir,
        template.max_turns,
        // MCP config from template
    ).await?;

    // 5. Emit agent started event
    state.event_bus.emit(ServerEvent::AgentStarted {
        agent_id: process_id.to_string(),
        template: sub.agent_template.clone(),
    });

    // 6. Wait for completion (or timeout)
    let result = state.supervisor.wait(&process_id).await?;

    // 7. Emit completed event
    state.event_bus.emit(ServerEvent::AgentCompleted {
        agent_id: process_id.to_string(),
        success: result.success,
    });

    // 8. Log episode (EpisodeLogger)
    // Episode includes: trigger signal, template, agent turns, outcome
    // This feeds into cascade routing and experiment tracking

    Ok(())
}
```

### Filter matching implementation

```rust
/// Check if a signal body matches a subscription filter.
///
/// The filter is a JSON object where each key must match the corresponding
/// key in the body. Array values mean "any of" (OR).
fn body_matches_filter(body: &Value, filter: &Value) -> bool {
    let filter_obj = match filter.as_object() {
        Some(o) => o,
        None => return true,
    };
    let body_obj = match body.as_object() {
        Some(o) => o,
        None => return false,
    };

    for (key, expected) in filter_obj {
        let actual = match body_obj.get(key) {
            Some(v) => v,
            None => return false,
        };
        if let Some(arr) = expected.as_array() {
            // Array filter: actual must match any element
            if !arr.iter().any(|e| e == actual) {
                return false;
            }
        } else if actual != expected {
            return false;
        }
    }
    true
}

/// Check if any changed file path matches a glob pattern.
fn paths_match_filter(body: &Value, pattern: &str) -> bool {
    let glob = glob::Pattern::new(pattern).ok();
    // GitHub push events have commits[].added/modified/removed arrays
    if let Some(commits) = body.get("commits").and_then(|c| c.as_array()) {
        for commit in commits {
            for field in ["added", "modified", "removed"] {
                if let Some(files) = commit.get(field).and_then(|f| f.as_array()) {
                    for file in files {
                        if let Some(path) = file.as_str() {
                            if let Some(ref g) = glob {
                                if g.matches(path) { return true; }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
```

### Checklist

- [ ] Create `dispatch.rs` module
- [ ] Implement `start_dispatch_loop()`
- [ ] Implement `run_agent_for_subscription()`
- [ ] Implement `body_matches_filter()`
- [ ] Implement `paths_match_filter()`
- [ ] Wire dispatch loop into server startup (spawn in `run_server`)
- [ ] Add `glob` crate to dependencies
- [ ] **Verify:** Create subscription for `webhook.github.push` → test template
- [ ] **Verify:** Send push webhook → agent spawns (visible in `GET /api/agents`)
- [ ] **Verify:** Filter matching works (send event that doesn't match filter → no spawn)
- [ ] **Verify:** Path filter works (push with wrong files → no spawn)
- [ ] **Verify:** Concurrency limit works (flood webhooks → only N agents)
- [ ] **Verify:** Cooldown works (rapid webhooks → second is skipped)
- [ ] **Verify:** Episode appears in `.roko/episodes.jsonl` after agent completes
