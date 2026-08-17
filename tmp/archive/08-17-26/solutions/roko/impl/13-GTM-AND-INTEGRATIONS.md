# GTM & Integrations: Implementation Plan

Phased plan for building roko's go-to-market integration surface: adapter-first
extensibility, gateway adapters, integration chains (GitHub, Linear, Langfuse,
Sentry), plugin system, marketplace foundation, recipe.toml, OTel observability,
and new market verticals. Each phase is independently valuable and does not block
later phases. 38 tasks across 7 phases.

Derived from: 21-GTM-MOAT-ANALYSIS, 21-GTM-INTEGRATIONS, 21-GTM-ADAPTER-MAP,
21-GTM-ADAPTER-PHILOSOPHY, 21-GTM-GATEWAY-ADAPTERS, 21-GTM-NEW-MARKETS,
21-GTM-ECOSYSTEM-PATTERNS.

---

## Phase 0: Adapter Trait Foundation (6 tasks, 1-2 weeks)

**Goal**: Define the core adapter trait interfaces and the RokoAdapter plugin
registration system. No external integrations yet -- this phase builds the
surface that all later integrations implement.

### 0.1 Define RokoAdapter Trait (Bevy Plugin Pattern)

**Files to create/modify**:
- `crates/roko-core/src/adapter.rs` (new)
- `crates/roko-core/src/lib.rs` (re-export)

Define the foundational adapter trait following the Bevy Plugin pattern:

```rust
pub trait RokoAdapter: Any + Send + Sync {
    fn build(&self, builder: &mut AdapterRegistry);
    fn ready(&self) -> bool { true }
    fn name(&self) -> &str { std::any::type_name::<Self>() }
    fn capabilities(&self) -> AdapterCapabilities { AdapterCapabilities::default() }
}

// Blanket impl: any fn(&mut AdapterRegistry) is an adapter
impl<F: Fn(&mut AdapterRegistry) + Send + Sync + 'static> RokoAdapter for F {
    fn build(&self, builder: &mut AdapterRegistry) {
        self(builder);
    }
}
```

Define `AdapterRegistry` as a typed map keyed by trait type:

```rust
pub struct AdapterRegistry {
    services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl AdapterRegistry {
    pub fn register<T: 'static + Send + Sync>(&mut self, service: T) { ... }
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> { ... }
}
```

Define `AdapterCapabilities`:

```rust
pub struct AdapterCapabilities {
    pub pull: bool,
    pub push: bool,
    pub write: bool,
    pub enrich: bool,
    pub activate_on: Vec<String>,
}
```

**Effort**: Small. ~120 LOC.

**Verification**:
```bash
rg 'pub trait RokoAdapter' crates/roko-core/src/ --type rust | wc -l  # >= 1
rg 'pub struct AdapterRegistry' crates/roko-core/src/ --type rust | wc -l  # >= 1
cargo test -p roko-core -- adapter
```

### 0.2 Define Core Adapter Trait Interfaces (Tier 2)

**Files to create**:
- `crates/roko-core/src/adapters/mod.rs` (new)
- `crates/roko-core/src/adapters/observability.rs` (new)
- `crates/roko-core/src/adapters/vcs.rs` (new)
- `crates/roko-core/src/adapters/work_source.rs` (new)
- `crates/roko-core/src/adapters/ci.rs` (new)
- `crates/roko-core/src/adapters/notification.rs` (new)
- `crates/roko-core/src/adapters/security.rs` (new)

Each trait follows the <=5-required-methods rule. Traits defined:

```rust
// observability.rs
#[async_trait]
pub trait ObservabilityExporter: Send + Sync {
    async fn export_turn(&self, turn: &AgentTurn) -> Result<()>;
    async fn export_gate_result(&self, result: &GateResult) -> Result<()>;
    async fn export_cost_event(&self, event: &CostEvent) -> Result<()>;
    async fn flush(&self) -> Result<()>;
}

// vcs.rs
#[async_trait]
pub trait VersionControlAdapter: Send + Sync {
    async fn create_branch(&self, repo: &str, branch: &str, from: &str) -> Result<()>;
    async fn create_pr(&self, repo: &str, pr: &PullRequest) -> Result<PrId>;
    async fn get_pr_diff(&self, pr: PrId) -> Result<String>;
    async fn merge_pr(&self, pr: PrId, strategy: MergeStrategy) -> Result<()>;
    fn capabilities(&self) -> VcsCapabilities;
}

// work_source.rs
#[async_trait]
pub trait WorkSource: Send + Sync {
    async fn fetch_candidates(&self) -> Result<Vec<WorkItem>>;
    async fn update_state(&self, id: &str, state: &str) -> Result<()>;
    fn capabilities(&self) -> WorkSourceCapabilities;
}

// ci.rs
#[async_trait]
pub trait CiAdapter: Send + Sync {
    async fn trigger_pipeline(&self, config: &PipelineConfig) -> Result<RunId>;
    async fn get_run_status(&self, id: RunId) -> Result<RunStatus>;
    async fn get_run_logs(&self, id: RunId, step: Option<&str>) -> Result<String>;
    fn capabilities(&self) -> CiCapabilities;
}

// notification.rs
#[async_trait]
pub trait NotificationAdapter: Send + Sync {
    async fn send(&self, msg: &Notification) -> Result<()>;
    async fn send_threaded(&self, thread_id: &str, msg: &Notification) -> Result<()>;
    fn capabilities(&self) -> NotificationCapabilities;
}

// security.rs
#[async_trait]
pub trait SecurityScanner: Send + Sync {
    async fn scan(&self, target: &ScanTarget) -> Result<Vec<Finding>>;
    async fn get_rules(&self) -> Result<Vec<Rule>>;
    fn output_format(&self) -> OutputFormat;
    fn capabilities(&self) -> ScannerCapabilities;
}
```

**Effort**: Medium. ~300 LOC across 6 files plus supporting types.

**Verification**:
```bash
rg 'pub trait.*: Send \+ Sync' crates/roko-core/src/adapters/ --type rust | wc -l  # >= 6
cargo check -p roko-core
```

### 0.3 Define Adapter Config TOML Schema

**Files to modify**:
- `crates/roko-core/src/config/mod.rs`

Add adapter configuration to `RokoConfig`:

```toml
# roko.toml
[adapters.github]
enabled = true
token_env = "GITHUB_TOKEN"
default_repo = "nunchi/roko"

[adapters.linear]
enabled = true
token_env = "LINEAR_API_KEY"
team_id = "TEAM_ID"

[adapters.otel]
enabled = true
provider = "langfuse"
endpoint = "https://cloud.langfuse.com/api/public/otel/v1/traces"
protocol = "http/protobuf"
auth = "basic"

[adapters.slack]
enabled = true
token_env = "SLACK_BOT_TOKEN"
channel = "#roko-notifications"

[adapters.semgrep]
enabled = true
rules = "auto"
```

Parse into `AdapterConfig` enum per adapter type, loaded at CLI startup.

**Effort**: Medium. ~150 LOC config parsing, ~50 LOC TOML schema.

**Verification**:
```bash
rg 'adapters' crates/roko-core/src/config/ --type rust | wc -l  # >= 3
cargo test -p roko-core -- adapter_config
```

### 0.4 Wire AdapterRegistry into CLI Startup

**Files to modify**:
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/lib.rs`

At CLI startup, build an `AdapterRegistry`, load enabled adapters from
`roko.toml`, and pass the registry to command handlers:

```rust
fn build_adapter_registry(config: &RokoConfig) -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    // Register built-in adapters based on config
    if let Some(otel_cfg) = config.adapters.get("otel") {
        registry.register::<Box<dyn ObservabilityExporter>>(
            Box::new(OtlpExporter::from_config(otel_cfg)?)
        );
    }
    // ... other adapters
    registry
}
```

Thread the registry through `run`, `chat`, `plan run`, and `orchestrate`
entry points.

**Effort**: Medium. ~100 LOC wiring.

**Verification**:
```bash
rg 'AdapterRegistry' crates/roko-cli/src/ --type rust | wc -l  # >= 3
```

### 0.5 Lazy Activation for Adapters

**Files to modify**:
- `crates/roko-core/src/adapter.rs`

Add activation-event matching to the registry. Adapters declare activation
events in config (`activate_on`). The registry only initializes an adapter
when its activation event fires:

```rust
impl AdapterRegistry {
    pub fn activate_for_event(&mut self, event: &str) -> Result<()> {
        for (name, adapter) in &self.pending {
            if adapter.capabilities().activate_on.contains(&event.to_string()) {
                adapter.build(self);
                self.active.insert(name.clone());
            }
        }
        Ok(())
    }
}
```

**Effort**: Small. ~60 LOC.

**Verification**:
```bash
rg 'activate_for_event\|activate_on' crates/roko-core/ --type rust | wc -l  # >= 2
```

### 0.6 Adapter Conformance Test Harness

**Files to create**:
- `crates/roko-core/src/adapters/conformance.rs` (new)

Provide a generic conformance test function per adapter trait:

```rust
pub async fn assert_work_source_conforms<W: WorkSource>(ws: &W) -> Result<()> {
    // 1. capabilities() returns valid struct
    let caps = ws.capabilities();
    assert!(caps.pull, "WorkSource must support pull");

    // 2. fetch_candidates() does not panic
    let _ = ws.fetch_candidates().await?;

    // 3. update_state() with invalid ID returns Err, not panic
    assert!(ws.update_state("nonexistent-id-000", "done").await.is_err());

    Ok(())
}
```

Each trait gets a conformance function. These are used by adapter authors
and required for "Roko Verified" badge.

**Effort**: Medium. ~200 LOC across all trait conformance functions.

**Verification**:
```bash
rg 'assert_.*_conforms' crates/roko-core/src/adapters/ --type rust | wc -l  # >= 4
```

---

## Phase 1: OTel Observability (4 tasks, 1-2 weeks)

**Goal**: Ship `gen_ai.*` OTel export. One config knob, six backends (Langfuse,
Honeycomb, Datadog, Grafana, Laminar, Arize Phoenix). This is the
highest-compounding integration: it makes everything measurable.

### 1.1 Implement OtlpExporter

**New crate**: `crates/roko-otel/` (new, or module in roko-runtime)

**Dependencies**: `opentelemetry = "0.28"`, `opentelemetry-otlp`, `opentelemetry-sdk`

Implement `ObservabilityExporter` via OTLP `http/protobuf`:

```rust
pub struct OtlpExporter {
    tracer: BoxedTracer,
    meter: Meter,
    config: OtelConfig,
}

impl ObservabilityExporter for OtlpExporter {
    async fn export_turn(&self, turn: &AgentTurn) -> Result<()> {
        let span = self.tracer.span_builder("gen_ai.chat")
            .with_attributes(vec![
                KeyValue::new("gen_ai.system", turn.provider.as_str()),
                KeyValue::new("gen_ai.request.model", turn.model.as_str()),
                KeyValue::new("gen_ai.usage.input_tokens", turn.input_tokens as i64),
                KeyValue::new("gen_ai.usage.output_tokens", turn.output_tokens as i64),
            ])
            .start(&self.tracer);
        // ...
    }
}
```

Use `opentelemetry-otlp` directly (not `opentelemetry-langfuse` -- bus factor 1,
3 stars). Construct basic-auth header from env vars. Target any OTLP endpoint.

**Effort**: Medium. ~200 LOC core exporter + ~80 LOC constants/attributes.

**Verification**:
```bash
# Exporter compiles
cargo check -p roko-otel

# gen_ai attributes are defined
rg 'gen_ai\.' crates/roko-otel/ --type rust | wc -l  # >= 6
```

### 1.2 Define gen_ai.* Span Hierarchy

**File**: `crates/roko-otel/src/spans.rs` (new)

Define the span hierarchy for agent execution:

```
roko.plan_run (root)
  roko.task (per-task)
    gen_ai.chat (per-model call)
      gen_ai.tool_call (per-tool invocation)
    roko.gate (per-gate rung)
```

Constants for all `gen_ai.*` and `roko.*` span attributes:

```rust
pub const GEN_AI_SYSTEM: &str = "gen_ai.system";
pub const GEN_AI_REQUEST_MODEL: &str = "gen_ai.request.model";
pub const GEN_AI_USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";
pub const GEN_AI_USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";
pub const ROKO_GATEWAY_CACHE_HIT: &str = "roko.gateway.cache_hit";
pub const ROKO_GATEWAY_ROUTER_DECISION: &str = "roko.gateway.router_decision";
pub const ROKO_GATEWAY_COST_USD: &str = "roko.gateway.cost_usd";
pub const ROKO_AGENT_CHAIN_ID: &str = "roko.agent.chain_id";
pub const ROKO_AGENT_TRIGGER_SOURCE: &str = "roko.agent.trigger_source";
```

**Effort**: Small. ~80 LOC.

### 1.3 Wire OtlpExporter into ModelCallService

**Files to modify**:
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-cli/src/run_inline.rs`
- `crates/roko-cli/src/chat_inline.rs`

After each `Agent::run()` call in ModelCallService, emit a span via
`ObservabilityExporter::export_turn()`. Thread the exporter from CLI
startup through model_call_service:

```rust
let exporter = registry.get::<Box<dyn ObservabilityExporter>>();
let service = ModelCallService::new(model)
    .with_observability(exporter);
```

**Effort**: Medium. ~80 LOC wiring across 3 files.

**Verification**:
```bash
# Set OTEL_EXPORTER_OTLP_ENDPOINT to a local collector and run:
cargo run -p roko-cli -- run "hello world"
# Verify spans appear in collector output
```

### 1.4 Wire OtlpExporter into Gate Pipeline

**Files to modify**:
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-cli/src/orchestrate.rs`

After each gate rung executes, emit a `roko.gate` span with attributes:

```rust
// gate_service.rs
if let Some(exporter) = &self.observability {
    exporter.export_gate_result(&GateResult {
        rung: rung_name,
        passed: verdict.passed,
        duration_ms: elapsed,
        output_lines: verdict.output.len(),
    }).await?;
}
```

**Effort**: Small. ~60 LOC.

**Verification**:
```bash
rg 'export_gate_result' crates/roko-gate/ --type rust | wc -l  # >= 1
```

---

## Phase 2: GitHub Integration (5 tasks, 2-3 weeks)

**Goal**: Full PR lifecycle via `octocrab`. Webhook reception for
issue-to-plan trigger. GitHub Actions reusable workflow.

### 2.1 Implement GitHubAdapter (VersionControlAdapter)

**New crate**: `crates/roko-github/` (new)

**Dependencies**: `octocrab = "0.49"`, `hmac`, `sha2`

Implement `VersionControlAdapter` with octocrab:

```rust
pub struct GitHubAdapter {
    client: Octocrab,
    default_repo: (String, String),
}

impl VersionControlAdapter for GitHubAdapter {
    async fn create_branch(&self, repo: &str, branch: &str, from: &str) -> Result<()> {
        let (owner, name) = parse_repo(repo);
        let sha = self.client.repos(&owner, &name)
            .get_ref(&Reference::Branch(from.to_string()))
            .await?
            .object.sha;
        self.client.repos(&owner, &name)
            .create_ref(&Reference::Branch(branch.to_string()), &sha)
            .await?;
        Ok(())
    }
    // ... create_pr, get_pr_diff, merge_pr
}
```

Auth: `OctocrabBuilder::app(app_id, key).build()` for GitHub App, or
`OctocrabBuilder::default().personal_token(token)` for PAT.

**Effort**: Medium. ~350 LOC adapter + ~100 LOC auth + ~50 LOC types.

**Verification**:
```bash
# Gated by ROKO_ACC=1 (acceptance tests need real GitHub token)
ROKO_ACC=1 cargo test -p roko-github -- github_adapter
```

### 2.2 GitHub Webhook Receiver

**Files to modify**:
- `crates/roko-serve/src/routes/integrations.rs` (new or extend existing)

Add webhook receiver route at `POST /webhooks/github`:

```rust
async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    // 1. Verify HMAC-SHA256 signature (X-Hub-Signature-256)
    verify_github_signature(&headers, &body, &state.config.github_webhook_secret)?;

    // 2. Parse event type from X-GitHub-Event header
    let event_type = headers.get("X-GitHub-Event")...;

    // 3. Dispatch:
    //    - "issues" with action "labeled" -> trigger plan generation
    //    - "pull_request" with action "opened" -> trigger review
    //    - "check_suite" with action "completed" -> record CI result
    match event_type {
        "issues" => handle_issue_event(state, payload).await,
        "pull_request" => handle_pr_event(state, payload).await,
        _ => Ok(StatusCode::OK),
    }
}
```

**Effort**: Medium. ~200 LOC handler + HMAC verification.

### 2.3 Issue-to-Plan Pipeline

**Files to modify**:
- `crates/roko-serve/src/routes/integrations.rs`
- `crates/roko-cli/src/orchestrate.rs`

When a GitHub issue receives a configured label (e.g., `roko:plan`), trigger:

1. Fetch issue body and comments via `GitHubAdapter`
2. Create a PRD from the issue content
3. Generate a plan via `prd plan`
4. Create a branch and begin execution
5. Update issue with plan link and status

Wire this as an event handler in roko-serve.

**Effort**: Medium. ~200 LOC pipeline glue.

**Verification**:
```bash
# Integration test: mock webhook -> verify plan created
cargo test -p roko-serve -- github_issue_to_plan
```

### 2.4 PR-from-Plan: Agent Creates PRs

**Files to modify**:
- `crates/roko-cli/src/orchestrate.rs`

After a task in a plan run produces code changes, create a PR via
`GitHubAdapter`:

```rust
if let Some(github) = registry.get::<GitHubAdapter>() {
    let pr_id = github.create_pr(&repo, &PullRequest {
        title: format!("roko: {}", task.description),
        body: format!("Generated by roko plan `{}`\n\nTask: {}", plan_name, task.id),
        head: &task_branch,
        base: "main",
    }).await?;
    // Update task state with PR URL
    task_state.pr_url = Some(pr_url);
}
```

**Effort**: Small. ~80 LOC.

### 2.5 GitHub Actions Reusable Workflow

**Files to create**:
- `.github/workflows/roko-gates.yml` (new, template)

Create a reusable GitHub Actions workflow that runs roko's gate pipeline
on PRs:

```yaml
name: Roko Gates
on:
  workflow_call:
    inputs:
      rungs:
        description: 'Comma-separated gate rungs to run'
        required: false
        default: 'compile,test,clippy'

jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install roko
        run: cargo install roko-cli
      - name: Run gates
        run: roko gate run --rungs ${{ inputs.rungs }}
```

**Effort**: Small. ~40 LOC YAML + documentation.

---

## Phase 3: Linear Integration (5 tasks, 2-3 weeks)

**Goal**: Linear AgentSession integration -- the gateway integration that
Cursor cannot ship and Devin validates at $500/month.

### 3.1 Linear GraphQL Client

**New crate**: `crates/roko-linear/` (new)

**Dependencies**: `graphql_client = "0.14"`, `reqwest`, `hmac`, `sha2`

Generate typed GraphQL operations from Linear's schema:

```rust
// Custom scalars module (required for graphql_client)
mod scalars {
    pub type DateTime = chrono::DateTime<chrono::Utc>;
    pub type JSON = serde_json::Value;
    pub type UUID = uuid::Uuid;
    pub type TimelessDate = String;
}

// Query files:
// - IssueQuery.graphql
// - AgentActivityCreate.graphql
// - IssueUpdateState.graphql
// - CommentCreate.graphql
// - TeamQuery.graphql
```

**Effort**: Medium. ~250 LOC codegen + scalars + 5 operations.

### 3.2 Linear Webhook Receiver with HMAC

**Files to modify**:
- `crates/roko-serve/src/routes/integrations.rs`

Add webhook receiver at `POST /webhooks/linear`:

```rust
async fn linear_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    // 1. HMAC-SHA256 verification: raw body, hex-encoded, Linear-Signature header
    verify_linear_hmac(&headers, &body, &state.config.linear_webhook_secret)?;

    // 2. Dedup via Linear-Delivery UUID
    let delivery_id = headers.get("Linear-Delivery")...;
    if state.seen_deliveries.contains(&delivery_id) {
        return Ok(StatusCode::OK);
    }

    // 3. Parse Linear-Event header
    let event = headers.get("Linear-Event")...;
    match event {
        "AgentSessionEvent" => handle_agent_session(state, payload).await,
        _ => Ok(StatusCode::OK),
    }
}
```

**Effort**: Medium. ~150 LOC handler + ~120 LOC HMAC verification.

### 3.3 AgentSession Event Handler

**Files to modify**:
- `crates/roko-linear/src/agent_session.rs` (new)

Handle the two AgentSession actions (`created` and `prompted`):

```rust
pub async fn handle_agent_session(
    state: AppState,
    payload: AgentSessionPayload,
) -> Result<StatusCode> {
    match payload.action.as_str() {
        "created" => {
            // 1. HTTP 200 within 5s budget
            // 2. Spawn tokio::task for thought activity within 10s
            tokio::spawn(async move {
                // Emit "thought" activity immediately (within 10s budget)
                linear_client.agent_activity_create(
                    session_id, "thought", "Analyzing issue..."
                ).await?;

                // Drive LLM roundtrip async
                let result = orchestrate_from_issue(&state, &issue).await?;

                // Emit "response" with result
                linear_client.agent_activity_create(
                    session_id, "response", &result.summary
                ).await?;
            });
            Ok(StatusCode::OK)
        }
        "prompted" => {
            if payload.agent_activity.signal == Some("stop") {
                // Stop signal -- cancel running task
                return Ok(StatusCode::OK);
            }
            // Handle follow-up prompt
            // ...
        }
    }
}
```

Key: emit-then-async pattern. HTTP 200 within 5s, first mutation within 10s.

**Effort**: Medium. ~200 LOC.

### 3.4 Implement LinearWorkSource (WorkSource Adapter)

**Files to create**:
- `crates/roko-linear/src/work_source.rs` (new)

Implement `WorkSource` for Linear:

```rust
impl WorkSource for LinearWorkSource {
    async fn fetch_candidates(&self) -> Result<Vec<WorkItem>> {
        let query = IssueQuery::build_query(variables);
        let response = self.client.post(&self.endpoint)
            .json(&query)
            .send().await?;
        // Map Linear issues to WorkItems
    }

    async fn update_state(&self, id: &str, state: &str) -> Result<()> {
        // Map state to Linear workflow state ID
        let mutation = IssueUpdateState::build_query(variables);
        self.client.post(&self.endpoint).json(&mutation).send().await?;
        Ok(())
    }
}
```

**Effort**: Small. ~120 LOC.

### 3.5 End-to-End Linear Chain (Chain B)

**Files to modify**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-serve/src/routes/integrations.rs`

Wire the full Chain B flow:

```
Linear issue webhook -> fetch issue -> create plan -> execute tasks
  -> create GitHub PR -> run CI -> update Linear issue status
```

This chains `LinearWorkSource` + `GitHubAdapter` + gate pipeline + status
updates. The complete flow is:

1. Linear webhook fires `AgentSessionEvent`
2. Handler fetches issue via `LinearWorkSource::fetch_candidates()`
3. Creates a plan from issue description
4. Dispatches agent via orchestrate.rs
5. Agent creates PR via `GitHubAdapter::create_pr()`
6. Gates validate (compile, test, clippy)
7. Updates Linear issue via `LinearWorkSource::update_state("done")`
8. Posts PR link as Linear comment

**Effort**: Medium. ~200 LOC integration glue.

**Verification**:
```bash
# Integration test with mock Linear + GitHub servers
cargo test -p roko-serve -- linear_chain_b
```

---

## Phase 4: Slack + Sentry (4 tasks, 1-2 weeks)

**Goal**: Ship the killer demo (Chain D: Slack thread -> agent -> trace URL
in reply) and the error-to-fix loop (Chain A: Sentry -> plan -> PR).

### 4.1 Implement SlackAdapter (NotificationAdapter)

**New module**: `crates/roko-serve/src/adapters/slack.rs` (new, or new crate)

**Dependencies**: `slack-morphism = "2.18"` with `axum` feature flag

Implement `NotificationAdapter` for Slack:

```rust
pub struct SlackAdapter {
    client: SlackClient<SlackClientHyperConnector>,
    default_channel: SlackChannelId,
}

impl NotificationAdapter for SlackAdapter {
    async fn send(&self, msg: &Notification) -> Result<()> {
        let content = SlackMessageContent::new()
            .with_blocks(render_blocks(msg));
        self.client.chat_post_message(
            &SlackApiChatPostMessageRequest::new(
                self.default_channel.clone(),
                content,
            )
        ).await?;
        Ok(())
    }

    async fn send_threaded(&self, thread_ts: &str, msg: &Notification) -> Result<()> {
        // Post to thread with trace URL, cost summary, gate results
    }
}
```

Use Socket Mode (no public webhook endpoint required):

```rust
let socket_listener = SlackClientSocketModeListener::new(
    &SlackClientSocketModeConfig::new(app_token)
);
```

**Effort**: Medium. ~250 LOC adapter + Block Kit rendering.

### 4.2 Slack Command Handler (`/roko`)

**Files to modify**:
- `crates/roko-serve/src/routes/integrations.rs`

Handle `/roko fix #ENG-123` slash command:

```rust
async fn slack_command(
    State(state): State<AppState>,
    Form(cmd): Form<SlackSlashCommand>,
) -> Result<Response> {
    // 1. Parse command: "/roko fix #ENG-123"
    let (action, issue_ref) = parse_roko_command(&cmd.text)?;

    // 2. Ack immediately (Slack 3s timeout)
    let ack = json!({ "response_type": "in_channel", "text": "On it..." });

    // 3. Spawn async handler
    tokio::spawn(async move {
        // Fetch issue from Linear/GitHub
        let issue = resolve_issue(&state, &issue_ref).await?;

        // Run agent
        let result = orchestrate_from_issue(&state, &issue).await?;

        // Reply in thread with trace URL
        state.slack.send_threaded(&cmd.channel_id, &Notification {
            title: format!("Fixed {}", issue_ref),
            body: result.summary,
            trace_url: result.trace_url,  // OTel trace link
            pr_url: result.pr_url,
            cost: result.total_cost_usd,
        }).await?;
    });

    Ok(Json(ack).into_response())
}
```

**Effort**: Medium. ~150 LOC.

### 4.3 Implement SentryAdapter via MCP

**New module**: `crates/roko-serve/src/adapters/sentry.rs` (new)

**Dependencies**: `rmcp` (official Rust MCP crate)

Use Sentry's MCP server to fetch issue details and resolve issues:

```rust
pub struct SentryAdapter {
    mcp_client: McpClient,
}

impl SentryAdapter {
    pub async fn get_issue_details(&self, issue_id: &str) -> Result<SentryIssue> {
        let result = self.mcp_client.call_tool(
            "getIssueDetails",
            json!({ "issue_id": issue_id }),
        ).await?;
        Ok(serde_json::from_value(result)?)
    }

    pub async fn resolve_issue(&self, issue_id: &str) -> Result<()> {
        self.mcp_client.call_tool(
            "resolveIssue",
            json!({ "issue_id": issue_id }),
        ).await?;
        Ok(())
    }
}
```

**Effort**: Small. ~150 LOC.

### 4.4 Wire Chain A: Sentry -> Plan -> PR -> Resolve

**Files to modify**:
- `crates/roko-serve/src/routes/integrations.rs`

Sentry webhook -> fetch issue details via MCP -> create plan -> agent
generates fix -> PR via GitHubAdapter -> on merge, resolve Sentry issue
+ close Linear issue:

```rust
async fn sentry_webhook(state: AppState, payload: SentryPayload) -> Result<StatusCode> {
    let issue = state.sentry.get_issue_details(&payload.issue_id).await?;

    // Create plan from error context
    let plan = create_fix_plan(&issue)?;

    // Execute
    let result = orchestrate_plan(&state, &plan).await?;

    // Create PR
    let pr_id = state.github.create_pr(&repo, &PullRequest {
        title: format!("fix: {}", issue.title),
        body: format!("Auto-fix for Sentry issue {}\n\nRoot cause: {}", issue.id, issue.culprit),
        ..
    }).await?;

    // On merge callback: resolve Sentry issue
    state.on_pr_merge(pr_id, move |state| async move {
        state.sentry.resolve_issue(&issue.id).await?;
        if let Some(linear_id) = issue.linear_issue_id {
            state.linear.update_state(&linear_id, "done").await?;
        }
    });

    Ok(StatusCode::OK)
}
```

**Effort**: Medium. ~200 LOC.

**Verification**:
```bash
# Integration test with mock Sentry MCP server
cargo test -p roko-serve -- sentry_chain_a
```

---

## Phase 5: Plugin System + Recipe Schema (7 tasks, 2-3 weeks)

**Goal**: Ship the connector.toml declarative format, recipe.toml composition
schema, plugin discovery, and the foundation for the adapter marketplace.

### 5.1 Define connector.toml Schema

**Files to create**:
- `crates/roko-core/src/connector.rs` (new)

The declarative connector format for 80% of REST API integrations:

```rust
#[derive(Deserialize)]
pub struct ConnectorManifest {
    pub connector: ConnectorMeta,
    pub auth: ConnectorAuth,
    pub endpoints: HashMap<String, EndpointDef>,
}

#[derive(Deserialize)]
pub struct ConnectorMeta {
    pub name: String,
    pub kind: ConnectorKind,  // rest | graphql | grpc | stdio
    pub version: String,
}

#[derive(Deserialize)]
pub struct ConnectorAuth {
    pub auth_type: AuthType,  // bearer | basic | oauth2 | api_key
    pub token_env: String,
}

#[derive(Deserialize)]
pub struct EndpointDef {
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub response_path: Option<String>,
    pub headers: HashMap<String, String>,
}
```

Example `connector.toml`:
```toml
[connector]
name = "linear-issues"
kind = "graphql"
version = "0.1.0"

[auth]
type = "bearer"
token_env = "LINEAR_API_KEY"

[endpoints.fetch_candidates]
method = "POST"
url = "https://api.linear.app/graphql"
body = '{ "query": "{ issues { nodes { id title } } }" }'
response_path = "data.issues.nodes"
```

**Effort**: Medium. ~200 LOC schema + parser.

### 5.2 Generate WorkSource from connector.toml

**Files to create**:
- `crates/roko-core/src/connector_runtime.rs` (new)

Runtime that generates a `WorkSource` implementation from a parsed
`ConnectorManifest`:

```rust
pub struct DeclaredWorkSource {
    manifest: ConnectorManifest,
    http_client: reqwest::Client,
}

impl WorkSource for DeclaredWorkSource {
    async fn fetch_candidates(&self) -> Result<Vec<WorkItem>> {
        let endpoint = &self.manifest.endpoints["fetch_candidates"];
        let resp = self.http_client
            .request(endpoint.method.parse()?, &endpoint.url)
            .header("Authorization", format!("Bearer {}", self.token()?))
            .body(endpoint.body.clone().unwrap_or_default())
            .send().await?;
        let json: Value = resp.json().await?;
        extract_items(&json, endpoint.response_path.as_deref())
    }
}
```

**Effort**: Medium. ~200 LOC.

**Verification**:
```bash
# connector.toml -> WorkSource -> fetch_candidates() returns items
cargo test -p roko-core -- declared_work_source
```

### 5.3 Define recipe.toml Schema

**Files to create**:
- `crates/roko-core/src/recipe.rs` (new)

The composition schema that wires multiple adapters into closed loops:

```rust
#[derive(Deserialize)]
pub struct Recipe {
    pub recipe: RecipeMeta,
    pub adapters: HashMap<String, AdapterRef>,
    pub parameters: Vec<RecipeParam>,
    pub steps: Vec<RecipeStep>,
    pub telemetry: Option<TelemetryConfig>,
}

#[derive(Deserialize)]
pub struct RecipeMeta {
    pub name: String,
    pub title: String,
    pub version: String,
    pub roko_compat: String,
    pub metadata: RecipeMetadata,
}

#[derive(Deserialize)]
pub struct RecipeMetadata {
    pub capabilities: Vec<String>,
    pub verified: bool,
}

#[derive(Deserialize)]
pub struct AdapterRef {
    pub version: String,
    pub features: Vec<String>,
}

#[derive(Deserialize)]
pub struct RecipeStep {
    pub id: String,
    pub adapter: String,
    pub action: String,
    pub inputs: HashMap<String, String>,  // supports ${{ parameters.x }} interpolation
    pub depends_on: Vec<String>,
}
```

**Effort**: Medium. ~180 LOC schema.

### 5.4 Recipe Executor

**Files to create**:
- `crates/roko-core/src/recipe_executor.rs` (new)

Execute a recipe by resolving adapter references, interpolating parameters,
and running steps in dependency order:

```rust
pub struct RecipeExecutor {
    registry: AdapterRegistry,
}

impl RecipeExecutor {
    pub async fn run(&self, recipe: &Recipe, params: &HashMap<String, String>) -> Result<RecipeResult> {
        let mut outputs: HashMap<String, Value> = HashMap::new();

        // Topological sort of steps by depends_on
        let ordered = topo_sort(&recipe.steps)?;

        for step in ordered {
            let adapter = self.registry.get_by_name(&step.adapter)?;
            let inputs = interpolate(&step.inputs, params, &outputs)?;
            let output = adapter.execute(&step.action, &inputs).await?;
            outputs.insert(step.id.clone(), output);
        }

        Ok(RecipeResult { outputs })
    }
}
```

**Effort**: Medium. ~200 LOC.

**Verification**:
```bash
# Recipe with 3 steps executes in correct order
cargo test -p roko-core -- recipe_executor
```

### 5.5 `roko recipe run` CLI Command

**Files to modify**:
- `crates/roko-cli/src/commands/mod.rs`
- `crates/roko-cli/src/commands/recipe.rs` (new)

Add CLI command to run a recipe:

```bash
roko recipe run path/to/recipe.toml --param github_repo=nunchi/roko
roko recipe list    # list available recipes
roko recipe show    # show recipe details
```

**Effort**: Small. ~120 LOC.

### 5.6 `roko adapter scaffold` CLI Command

**Files to modify**:
- `crates/roko-cli/src/commands/mod.rs`
- `crates/roko-cli/src/commands/adapter.rs` (new)

Scaffold new adapters in <15 minutes:

```bash
roko adapter scaffold --name my-tracker --kind rest --trait WorkSource
# Creates:
#   adapters/my-tracker/connector.toml  (declarative template)
#   adapters/my-tracker/src/lib.rs      (trait impl skeleton)
#   adapters/my-tracker/Cargo.toml      (crate template)
```

For declarative adapters:
```bash
roko adapter scaffold --name my-api --kind rest --declarative
# Creates only connector.toml with example endpoints
```

**Effort**: Small. ~150 LOC + templates.

### 5.7 Adapter Discovery via roko.toml

**Files to modify**:
- `crates/roko-core/src/config/mod.rs`
- `crates/roko-cli/src/main.rs`

`roko.toml` declares required adapters; `roko init` resolves them:

```toml
[adapters.dependencies]
github = { version = "^1.0", source = "builtin" }
linear = { version = "^0.5", source = "builtin" }
my-tracker = { version = "^0.1", source = "path", path = "./adapters/my-tracker" }
custom-api = { version = "^0.2", source = "connector", path = "./connectors/custom-api.toml" }
```

At startup, resolve adapter sources and register them:
- `builtin`: compiled into roko binary
- `path`: local crate, loaded as dynamic library or subprocess
- `connector`: declarative connector.toml, loaded at runtime

**Effort**: Medium. ~150 LOC.

---

## Phase 6: Gateway Adapter Layers (5 tasks, 2-3 weeks)

**Goal**: Make the gateway pipeline's 8 layers adapter-driven. Each layer
becomes a trait boundary that external implementations can plug into.

### 6.1 CacheLayer Trait + Moka L1 Implementation

**Files to modify**:
- `crates/roko-serve/src/lib.rs` (or new gateway module)

Define the cache layer trait and ship a Moka-based L1 implementation:

```rust
#[async_trait]
pub trait CacheLayer: Send + Sync {
    async fn get(&self, key: &CacheKey) -> Option<CachedResponse>;
    async fn put(&self, key: &CacheKey, response: &CachedResponse) -> Result<()>;
    async fn stats(&self) -> CacheStats;
}

pub struct MokaL1Cache {
    cache: Cache<Blake3Hash, CachedResponse>,
}
```

BLAKE3 hash for exact-match cache keys. Moka for LRU eviction.

**Effort**: Medium. ~180 LOC.

### 6.2 Safety Pipeline Traits

**Files to create**:
- `crates/roko-core/src/adapters/safety.rs` (new)

Define safety adapter traits for the gateway:

```rust
#[async_trait]
pub trait PiiScanner: Send + Sync {
    async fn scan(&self, text: &str) -> Result<Vec<PiiMatch>>;
    async fn mask(&self, text: &str) -> Result<String>;
}

#[async_trait]
pub trait InjectionDetector: Send + Sync {
    async fn detect(&self, prompt: &str) -> Result<InjectionScore>;
}
```

Ship regex-based default implementations. External ML-based scanners
connect via the same trait.

**Effort**: Small. ~150 LOC traits + regex defaults.

### 6.3 Optimizer Pipeline Trait

**Files to create**:
- `crates/roko-core/src/adapters/optimizer.rs` (new)

Define the optimizer trait for request/response optimization:

```rust
#[async_trait]
pub trait Optimizer: Send + Sync {
    fn name(&self) -> &str;
    async fn optimize_request(&self, req: &mut LlmRequest) -> Result<()>;
    async fn optimize_response(&self, resp: &mut LlmResponse) -> Result<()>;
    fn priority(&self) -> u32 { 100 }
}
```

Ship 3 starter optimizers:
- `ToolPruneOptimizer`: remove unused tools from request
- `OutputBudgetOptimizer`: set max_tokens based on task complexity
- `LoopDetectOptimizer`: detect oscillation patterns in agent tool calls

**Effort**: Medium. ~250 LOC traits + 3 implementations.

### 6.4 BillingProvider Trait + Stripe Skeleton

**Files to create**:
- `crates/roko-core/src/adapters/billing.rs` (new)

Define the billing trait:

```rust
#[async_trait]
pub trait BillingProvider: Send + Sync {
    async fn authorize(&self, key: &ApiKey, estimated_cost: f64) -> Result<BillingAuth>;
    async fn record(&self, key: &ApiKey, actual_cost: f64, metadata: &UsageMetadata) -> Result<()>;
    async fn check_budget(&self, key: &ApiKey) -> Result<BudgetStatus>;
}
```

Ship a Stripe skeleton implementation that maps to Stripe's March 2026
LLM token billing API:

```rust
pub struct StripeBilling {
    client: stripe::Client,
    meter_id: String,
}
```

Full Stripe integration is Phase 7 (revenue). This phase defines the
trait and skeleton.

**Effort**: Small. ~120 LOC trait + skeleton.

### 6.5 Wire Gateway Layers into roko-serve

**Files to modify**:
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-serve/src/state.rs`

Compose all gateway layers via AdapterRegistry:

```rust
// state.rs
pub struct GatewayPipeline {
    pub auth: Box<dyn AuthAdapter>,
    pub safety: Vec<Box<dyn PiiScanner>>,
    pub cache: Box<dyn CacheLayer>,
    pub router: Box<dyn Router>,
    pub optimizers: Vec<Box<dyn Optimizer>>,
    pub billing: Option<Box<dyn BillingProvider>>,
    pub provider: Box<dyn Provider>,
    pub observability: Option<Box<dyn ObservabilityExporter>>,
}
```

The gateway route processes requests through each layer in order.

**Effort**: Medium. ~200 LOC pipeline composition.

**Verification**:
```bash
# Gateway processes request through all configured layers
cargo test -p roko-serve -- gateway_pipeline
```

---

## Phase 7: Marketplace Foundation + New Market Verticals (7 tasks, 3-4 weeks)

**Goal**: Adapter registry, verification badges, roko-contrib monorepo
scaffolding, and vertical-specific role templates for P0 markets (DevOps/SRE,
Security Ops, Compliance/Audit).

### 7.1 Adapter Registry Protocol

**Files to create**:
- `crates/roko-core/src/registry.rs` (new)

Define the protocol for adapter discovery and installation:

```rust
pub struct AdapterRegistryEntry {
    pub name: String,
    pub version: String,
    pub source: AdapterSource,
    pub verified: bool,
    pub capabilities: AdapterCapabilities,
    pub checksum: String,  // BLAKE3 hash
}

pub enum AdapterSource {
    Builtin,
    CratesIo { crate_name: String },
    Git { url: String, rev: String },
    Path { path: PathBuf },
    Connector { manifest: PathBuf },
}

#[async_trait]
pub trait AdapterRegistryClient: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<AdapterRegistryEntry>>;
    async fn fetch(&self, name: &str, version: &str) -> Result<AdapterPackage>;
    async fn publish(&self, package: &AdapterPackage) -> Result<()>;
}
```

Ship a local-filesystem registry implementation for development.

**Effort**: Medium. ~200 LOC.

### 7.2 Verification Badge System

**Files to create**:
- `crates/roko-core/src/verification.rs` (new)

Binary `verified: true` badge. Earning verification requires:

1. Passing conformance tests (from 0.6)
2. CI green in roko-contrib
3. Reviewed by a maintainer

```rust
pub struct VerificationResult {
    pub adapter_name: String,
    pub version: String,
    pub conformance_passed: bool,
    pub ci_passed: bool,
    pub reviewer: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub badge: VerificationBadge,
}

pub enum VerificationBadge {
    Unverified,
    CommunityTested,  // conformance + CI only
    RokoVerified,     // conformance + CI + reviewer
}
```

**Effort**: Small. ~100 LOC.

### 7.3 `roko adapter list/search/install` CLI Commands

**Files to modify**:
- `crates/roko-cli/src/commands/adapter.rs`

```bash
roko adapter list                      # list installed adapters
roko adapter search "linear"           # search registry
roko adapter install github@^1.0       # install from registry
roko adapter verify my-adapter         # run conformance tests
roko adapter info github               # show adapter details + capabilities
```

**Effort**: Medium. ~200 LOC.

### 7.4 Security Ops Vertical: Semgrep SecurityScanner

**New module**: `crates/roko-serve/src/adapters/semgrep.rs` (new)

Implement `SecurityScanner` for Semgrep:

```rust
pub struct SemgrepScanner {
    config: SemgrepConfig,
}

impl SecurityScanner for SemgrepScanner {
    async fn scan(&self, target: &ScanTarget) -> Result<Vec<Finding>> {
        let output = Command::new("semgrep")
            .args(["scan", "--config", &self.config.rules, "--sarif", &target.path])
            .output().await?;
        parse_sarif(&output.stdout)
    }
}
```

Wire as a new gate rung (`security-scan`) between `test` and `review`:

```toml
[[gates.rungs]]
name = "security-scan"
kind = "external"
command = "semgrep scan --config auto --sarif"
output_format = "sarif"
on_failure = "block"
```

**Effort**: Small. ~150 LOC.

**Verification**:
```bash
# Semgrep scan produces SARIF findings
cargo test -p roko-serve -- semgrep_scanner
```

### 7.5 DevOps/SRE Vertical: AlertSource Adapter Trait

**Files to create**:
- `crates/roko-core/src/adapters/alert.rs` (new)

Define the alert source adapter for SRE self-healing:

```rust
#[async_trait]
pub trait AlertSource: Send + Sync {
    async fn fetch_alerts(&self) -> Result<Vec<Alert>>;
    async fn acknowledge(&self, alert_id: &str) -> Result<()>;
    async fn resolve(&self, alert_id: &str, resolution: &str) -> Result<()>;
    fn capabilities(&self) -> AlertSourceCapabilities;
}

pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub service: String,
    pub triggered_at: DateTime<Utc>,
    pub runbook_url: Option<String>,
}
```

This enables the PagerDuty/Datadog -> auto-diagnosis -> fix PR -> deploy
workflow for P0 DevOps/SRE market.

**Effort**: Small. ~100 LOC trait definition + types.

### 7.6 Compliance Vertical: AuditAdapter Trait

**Files to create**:
- `crates/roko-core/src/adapters/audit.rs` (new)

Define the audit adapter for compliance/regulatory markets:

```rust
#[async_trait]
pub trait AuditAdapter: Send + Sync {
    async fn record_action(&self, action: &AuditableAction) -> Result<AuditReceipt>;
    async fn verify_receipt(&self, receipt: &AuditReceipt) -> Result<bool>;
    async fn export_trail(&self, range: &DateRange) -> Result<Vec<AuditEntry>>;
}

pub struct AuditableAction {
    pub agent_id: String,
    pub action_type: String,
    pub inputs_hash: String,
    pub outputs_hash: String,
    pub gate_results: Vec<GateResult>,
    pub timestamp: DateTime<Utc>,
}
```

Wire gate results into audit records automatically:

```rust
// In orchestrate.rs, after gate run:
if let Some(audit) = registry.get::<Box<dyn AuditAdapter>>() {
    audit.record_action(&AuditableAction {
        agent_id: agent_id.clone(),
        action_type: "gate_run".to_string(),
        inputs_hash: blake3_hash(&task_input),
        outputs_hash: blake3_hash(&gate_report),
        gate_results: gate_report.verdicts.clone(),
        timestamp: Utc::now(),
    }).await?;
}
```

**Effort**: Medium. ~180 LOC trait + wiring.

**Verification**:
```bash
rg 'AuditAdapter\|record_action' crates/ --type rust | wc -l  # >= 3
```

### 7.7 Vertical Role Templates

**Files to create**:
- `crates/roko-compose/src/templates/sre.rs` (new)
- `crates/roko-compose/src/templates/security.rs` (new)
- `crates/roko-compose/src/templates/compliance.rs` (new)

Add role templates for P0 market verticals:

```rust
// sre.rs
pub fn sre_role_template() -> RolePromptTemplate {
    RolePromptTemplate {
        role: "sre-responder",
        identity: "You are an SRE agent that diagnoses and remediates production incidents.",
        rules: vec![
            "Always check runbook URLs before proposing fixes.",
            "Prefer rollback to forward-fix when uncertainty is high.",
            "Never modify infrastructure without gate validation.",
        ],
        tools: vec!["kubectl", "terraform", "datadog-query", "pagerduty-ack"],
        gate_rungs: vec!["syntax-check", "dry-run", "review"],
    }
}
```

Similar templates for `security-analyst` and `compliance-auditor` roles.

**Effort**: Small. ~150 LOC across 3 templates.

---

## Task Summary

| Phase | Tasks | Focus | Effort |
|---|---|---|---|
| 0 | 0.1 - 0.6 | Adapter trait foundation | 1-2 weeks |
| 1 | 1.1 - 1.4 | OTel observability | 1-2 weeks |
| 2 | 2.1 - 2.5 | GitHub integration | 2-3 weeks |
| 3 | 3.1 - 3.5 | Linear integration | 2-3 weeks |
| 4 | 4.1 - 4.4 | Slack + Sentry | 1-2 weeks |
| 5 | 5.1 - 5.7 | Plugin system + recipe.toml | 2-3 weeks |
| 6 | 6.1 - 6.5 | Gateway adapter layers | 2-3 weeks |
| 7 | 7.1 - 7.7 | Marketplace + verticals | 3-4 weeks |
| **Total** | **38 tasks** | | **14-22 weeks** |

---

## Dependencies Between Phases

```
Phase 0 (Adapter Foundation)
  |
  +---> Phase 1 (OTel) -- no external deps
  |       |
  +---> Phase 2 (GitHub) -- needs octocrab
  |       |
  +---> Phase 3 (Linear) -- needs graphql_client
  |       |         |
  |       v         v
  +---> Phase 4 (Slack + Sentry) -- needs Phase 2 (GitHub) for Chain A
  |
  +---> Phase 5 (Plugin System) -- can run in parallel with 2-4
  |
  +---> Phase 6 (Gateway Layers) -- can run in parallel with 2-4
  |
  +---> Phase 7 (Marketplace + Verticals) -- needs Phase 0 (traits) + Phase 5 (registry)
```

Phase 0 is the critical path. Phases 1-4 can proceed in parallel once
Phase 0 lands. Phase 5-6 can run in parallel with integration work.
Phase 7 depends on the trait definitions (Phase 0) and plugin system
(Phase 5) being in place.

---

## 90-Day Shipping Sequence

| Weeks | Deliverable | Demo |
|---|---|---|
| 1-3 | Phase 0 (traits) + Phase 1 (OTel) | `roko run` produces gen_ai.* traces visible in Langfuse |
| 3-6 | Phase 2 (GitHub) | Issue labeled -> agent creates PR -> gates validate |
| 6-9 | Phase 3 (Linear) | Linear issue -> agent session -> PR -> issue closed |
| 9-11 | Phase 4 (Slack + Sentry) | `/roko fix #ENG-123` -> threaded reply with trace URL |
| 11-13 | Phase 5 (Plugin System) | `roko adapter scaffold` + `roko recipe run` |

Phases 6-7 extend beyond the 90-day window but are independently valuable
and do not gate the demo sequence.

---

## Named Integration Chains (Wired by End of Phase 4)

| Chain | Flow | Phase Complete |
|---|---|---|
| **A** | Sentry issue -> plan -> GitHub PR -> OTel trace -> Linear closed | Phase 4 |
| **B** | Linear webhook -> plan -> GitHub PR -> CI -> Linear status update | Phase 3 |
| **C** | GitHub label -> plan -> PR -> Slack approval -> merge | Phase 4 |
| **D** | Slack `/roko` -> agent -> tool use -> Slack reply with trace URL | Phase 4 |
| **E** | recipe.toml composition (one import -> 5+ adapters wired) | Phase 5 |

---

## Acceptance Criteria (Overall)

```bash
# Adapter traits defined and documented
rg 'pub trait.*: Send \+ Sync' crates/roko-core/src/adapters/ --type rust | wc -l  # >= 8

# OTel export works (gen_ai.* spans)
rg 'gen_ai\.' crates/ --type rust | wc -l  # >= 10

# GitHub adapter wired
rg 'GitHubAdapter\|VersionControlAdapter' crates/ --type rust | wc -l  # >= 5

# Linear adapter wired
rg 'LinearWorkSource\|AgentSession' crates/ --type rust | wc -l  # >= 4

# Slack adapter wired
rg 'SlackAdapter\|NotificationAdapter' crates/ --type rust | wc -l  # >= 3

# Recipe schema parsed
rg 'Recipe\b' crates/roko-core/src/recipe.rs --type rust | wc -l  # >= 3

# Gateway layers are trait-driven
rg 'Box<dyn.*>' crates/roko-serve/src/ --type rust | grep -E 'Cache|Safety|Optimizer|Billing' | wc -l  # >= 4

# Conformance tests exist for each trait
rg 'assert_.*_conforms' crates/roko-core/src/adapters/ --type rust | wc -l  # >= 6
```

---

## Risk Register

| Risk | Mitigation |
|---|---|
| Linear AgentSession protocol changes | Pin GraphQL schema version; monitor Linear changelog |
| Langfuse re-license (ClickHouse 2027-2028) | Use `opentelemetry-otlp` directly, not Langfuse-specific crate. One env var change repoints to any OTLP endpoint. |
| MCP quality crisis (52% abandonment) | Position as quality layer with verification badge. Ship conformance crate. |
| Contributor ramp is slow | Start with 10-20 Roko Verified adapters. Declarative connector.toml lowers barrier. |
| Gateway-standalone market caps at ~$50M ARR | Gateway is data-acquisition layer for orchestration platform, not standalone product. |
| EU AI Act Article 50 timeline shifts | Audit adapter is independently valuable regardless of enforcement date. |
