# Task 13: GTM & Integrations

Adapter-first extensibility, gateway adapters, integration chains (GitHub, Linear,
Langfuse, Sentry), plugin system, marketplace foundation, recipe.toml, OTel
observability, and new market verticals.

38 tasks across 7 phases. Each phase is independently valuable.

---

## Overview

Roko's integration surface today is structurally shallow: webhook endpoints for
GitHub and Slack exist (`crates/roko-serve/src/routes/webhooks.rs`), MCP servers
for GitHub/Slack/scripts exist (`crates/roko-mcp-{github,slack,scripts}/`), and
an `IntegrationRegistry` catalog exists (`crates/roko-serve/src/integrations.rs`)
with 6 built-in entries. But none of these are adapter-trait-driven -- they are
hardcoded implementations with no generalization boundary. The webhook handlers
convert payloads to `Engram` signals but do not trigger agent execution or plan
generation. The MCP servers are standalone binaries with no shared trait
interface.

This task plan builds the adapter trait foundation in `roko-core`, implements
Tier 0-1 integrations (OTel, GitHub, Linear, Slack, Sentry) behind those traits,
ships the plugin/recipe composition system, makes the gateway adapter-driven,
and scaffolds marketplace + vertical expansion.

### What Already Exists (Do NOT Reimplement)

| Component | Location | Status |
|---|---|---|
| `IntegrationRegistry` + 6 builtins | `crates/roko-serve/src/integrations.rs` | Catalog only, no dispatch |
| GitHub webhook receiver (HMAC verified) | `crates/roko-serve/src/routes/webhooks.rs` | Engram-only, no agent trigger |
| Slack webhook receiver (HMAC verified) | `crates/roko-serve/src/routes/webhooks.rs` | Engram-only, no agent trigger |
| Generic webhook receiver | `crates/roko-serve/src/routes/webhooks.rs` | Works |
| `roko-mcp-github` (19 tools) | `crates/roko-mcp-github/src/main.rs` | Standalone MCP binary |
| `roko-mcp-slack` (9 tools) | `crates/roko-mcp-slack/src/main.rs` | Standalone MCP binary |
| `ProviderAdapter` trait (6 impls) | `crates/roko-agent/src/provider/mod.rs` | Wired for LLM dispatch |
| `ConnectorRegistry` + routes | `crates/roko-core/src/connector.rs`, `crates/roko-serve/src/routes/connectors.rs` | Legacy, migration-flagged |
| `GateService` (7 rungs) | `crates/roko-gate/src/gate_service.rs` | Wired, hardcoded gate map |
| `SecurityScanGate` (cargo audit) | `crates/roko-gate/src/security_scan_gate.rs` | Built, single scanner |
| `GateRunner` trait | `crates/roko-core/src/foundation.rs` | Foundation trait |
| `Verify` trait | `crates/roko-core/src/traits.rs` | Core gate trait |
| `ModelCallService` | `crates/roko-agent/src/model_call_service.rs` | Wired, no OTel export |
| `EventSource` trait | `crates/roko-plugin/src/lib.rs` | Plugin SDK for events |
| `GithubWebhookConfig` | `crates/roko-core/src/config/serve.rs` | Config struct exists |
| `octocrab` dep in workspace | `Cargo.toml` line 152, `roko-serve`, `roko-cli` | Already declared |
| `SubscriptionConfig` | `crates/roko-core/src/config/subscriptions.rs` | Event subscription wiring |

### Anti-Patterns to Remove

1. **Webhook-to-engram dead end**: `webhooks.rs` converts GitHub/Slack payloads
   to `Engram` signals but never triggers agent dispatch or plan creation.
   Webhooks must flow through the adapter layer into orchestration.

2. **MCP servers without shared trait**: `roko-mcp-github` and `roko-mcp-slack`
   are standalone binaries with no adapter trait. Each implements its own
   `ToolsCallParams` struct. The new adapter traits generalize the interface so
   new integrations implement a trait, not a bespoke binary.

3. **Hardcoded gate rung map**: `GateService::gate_for_name()` has a static
   `match` over 7 names. New gate types (Semgrep, external CI) cannot be added
   without modifying `gate_service.rs`. The adapter pattern makes gates
   pluggable via config.

4. **No observability export**: `ModelCallService` tracks cost and usage
   internally but has no export path to OTel/Langfuse/any backend. Every LLM
   call is invisible to external observability tools.

5. **`ConnectorRegistry` legacy**: `roko-core/src/connector.rs` has a migration
   note saying it will be superseded. New adapter traits should not build on
   this -- they should replace it.

6. **`IntegrationRegistry` is catalog-only**: The registry in
   `integrations.rs` describes integrations but has no `execute` or `dispatch`
   capability. It is documentation, not runtime infrastructure.

---

## Phase 0: Adapter Trait Foundation (6 tasks)

**Goal**: Define core adapter trait interfaces and the `AdapterRegistry` plugin
registration system in `roko-core`. No external integrations yet.

---

### Task 13.1: Define `RokoAdapter` Trait and `AdapterRegistry`

**Files to create**:
- `crates/roko-core/src/adapter.rs`

**Files to modify**:
- `crates/roko-core/src/lib.rs` (re-export)

**What to build**:
Define the foundational adapter trait following the Bevy Plugin pattern:

```rust
pub trait RokoAdapter: Any + Send + Sync {
    fn build(&self, builder: &mut AdapterRegistry);
    fn ready(&self) -> bool { true }
    fn name(&self) -> &str { std::any::type_name::<Self>() }
    fn capabilities(&self) -> AdapterCapabilities { AdapterCapabilities::default() }
}

pub struct AdapterCapabilities {
    pub pull: bool,
    pub push: bool,
    pub write: bool,
    pub enrich: bool,
    pub activate_on: Vec<String>,
}
```

Define `AdapterRegistry` as a typed map keyed by trait type. Support
`register::<T>()`, `get::<T>()`, `get_by_name()`. Provide blanket impl so
any `fn(&mut AdapterRegistry)` is an adapter.

**Existing code to be aware of**:
- `IntegrationRegistry` in `crates/roko-serve/src/integrations.rs` is a
  catalog, not a typed dispatch registry. `AdapterRegistry` replaces its
  runtime role.
- `ConnectorRegistry` in `crates/roko-core/src/connector.rs` is flagged for
  migration. Do not extend it.

**Effort**: ~120 LOC.

**Verification**:
```bash
rg 'pub trait RokoAdapter' crates/roko-core/src/ --type rust | wc -l  # >= 1
rg 'pub struct AdapterRegistry' crates/roko-core/src/ --type rust | wc -l  # >= 1
cargo test -p roko-core -- adapter
```

---

### Task 13.2: Define Core Adapter Trait Interfaces

**Files to create**:
- `crates/roko-core/src/adapters/mod.rs`
- `crates/roko-core/src/adapters/observability.rs`
- `crates/roko-core/src/adapters/vcs.rs`
- `crates/roko-core/src/adapters/work_source.rs`
- `crates/roko-core/src/adapters/ci.rs`
- `crates/roko-core/src/adapters/notification.rs`
- `crates/roko-core/src/adapters/security.rs`

**Files to modify**:
- `crates/roko-core/src/lib.rs` (re-export `adapters` module)

**What to build**:
Six adapter trait interfaces, each with <=5 required methods:

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
    async fn cancel_run(&self, id: RunId) -> Result<()>;
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

Define supporting types (`AgentTurn`, `GateResult`, `CostEvent`, `PullRequest`,
`PrId`, `MergeStrategy`, `WorkItem`, `PipelineConfig`, `RunId`, `RunStatus`,
`Notification`, `ScanTarget`, `Finding`, `Rule`, `OutputFormat`) and their
`Capabilities` structs.

**Existing code to be aware of**:
- `Verify` trait in `crates/roko-core/src/traits.rs` is the gate-level
  verification trait. `SecurityScanner` is a higher-level adapter that wraps
  external tools, not a replacement for `Verify`.
- `FeedbackEvent` in `crates/roko-core/src/foundation.rs` already carries
  some of the fields `AgentTurn` needs. Reuse where possible.
- `GateReport` / `GateVerdict` in `crates/roko-core/src/foundation.rs`
  already exist. `GateResult` in the adapter layer should wrap or alias these.

**Effort**: ~300 LOC across 6 files plus supporting types.

**Verification**:
```bash
rg 'pub trait.*: Send \+ Sync' crates/roko-core/src/adapters/ --type rust | wc -l  # >= 6
cargo check -p roko-core
```

---

### Task 13.3: Define Adapter Config TOML Schema

**Files to modify**:
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-core/src/config/mod.rs`

**What to build**:
Add `[adapters.*]` table to `RokoConfig`:

```toml
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

Parse into per-adapter `AdapterConfig` enum variants. Keep it backward
compatible -- missing `[adapters]` section means no adapters enabled.

**Existing code to be aware of**:
- `GithubWebhookConfig` exists at `crates/roko-core/src/config/serve.rs:350`.
  The new adapter config is broader (token, default repo, PR settings).
  `GithubWebhookConfig.secret` migrates into `adapters.github.webhook_secret`.
- `WebhooksConfig` at `serve.rs:334` wraps `github` field. Deprecate in
  favor of `adapters.github`.

**Effort**: ~150 LOC.

**Verification**:
```bash
rg 'AdapterConfig\|adapters.*github\|adapters.*otel' crates/roko-core/src/config/ --type rust | wc -l  # >= 3
cargo check -p roko-core
```

---

### Task 13.4: Wire `AdapterRegistry` into CLI Startup

**Files to modify**:
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/lib.rs` (if exists)

**What to build**:
At CLI startup, build an `AdapterRegistry`, load enabled adapters from
`roko.toml` config, and thread the registry through command handlers:

```rust
fn build_adapter_registry(config: &RokoConfig) -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    if let Some(otel_cfg) = config.adapters.get("otel") {
        registry.register::<Box<dyn ObservabilityExporter>>(
            Box::new(OtlpExporter::from_config(otel_cfg)?)
        );
    }
    // ... other adapters based on config
    registry
}
```

Thread the registry through `run`, `chat`, `plan run`, and `orchestrate`
entry points. Commands that do not need adapters (e.g., `roko config show`)
skip registry construction.

**Existing code to be aware of**:
- `roko-cli/src/main.rs` already loads `RokoConfig`. The registry should be
  constructed after config load, before command dispatch.
- `orchestrate.rs` already receives many parameters. Consider a context
  struct that bundles `AdapterRegistry` + `RokoConfig` + other runtime state.

**Effort**: ~100 LOC wiring.

**Verification**:
```bash
rg 'AdapterRegistry' crates/roko-cli/src/ --type rust | wc -l  # >= 3
cargo check -p roko-cli
```

---

### Task 13.5: Lazy Activation for Adapters

**Files to modify**:
- `crates/roko-core/src/adapter.rs`

**What to build**:
Add activation-event matching to the registry. Adapters declare activation
events via `AdapterCapabilities::activate_on`. The registry keeps adapters
in a `pending` map until their activation event fires:

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

This avoids initializing the Slack adapter on `roko run "hello"` when only
OTel is needed.

**Effort**: ~60 LOC.

**Verification**:
```bash
rg 'activate_for_event\|activate_on' crates/roko-core/ --type rust | wc -l  # >= 2
cargo test -p roko-core -- adapter_lazy
```

---

### Task 13.6: Adapter Conformance Test Harness

**Files to create**:
- `crates/roko-core/src/adapters/conformance.rs`

**What to build**:
Generic conformance test functions per adapter trait. Each function validates
trait contract invariants:

```rust
pub async fn assert_work_source_conforms<W: WorkSource>(ws: &W) -> Result<()> {
    let caps = ws.capabilities();
    assert!(caps.pull, "WorkSource must support pull");
    let _ = ws.fetch_candidates().await?;
    assert!(ws.update_state("nonexistent-id-000", "done").await.is_err());
    Ok(())
}

pub async fn assert_observability_conforms<O: ObservabilityExporter>(o: &O) -> Result<()> {
    // flush() on empty state does not error
    o.flush().await?;
    Ok(())
}
```

Provide conformance functions for all 6 trait interfaces. These are used by
adapter implementors and required for the "Roko Verified" badge (Task 13.32).

**Effort**: ~200 LOC.

**Verification**:
```bash
rg 'assert_.*_conforms' crates/roko-core/src/adapters/ --type rust | wc -l  # >= 6
cargo test -p roko-core -- conformance
```

---

## Phase 1: OTel Observability (4 tasks)

**Goal**: Ship `gen_ai.*` OTel export. One config knob, six backends (Langfuse,
Honeycomb, Datadog, Grafana, Laminar, Arize Phoenix). Highest-compounding
integration: makes everything measurable.

**Dependency**: Phase 0 (adapter traits).

---

### Task 13.7: Implement `OtlpExporter`

**New crate to create**: `crates/roko-otel/`

**Dependencies**: `opentelemetry = "0.28"`, `opentelemetry-otlp`, `opentelemetry-sdk`

**What to build**:
Implement `ObservabilityExporter` (from Task 13.2) via OTLP `http/protobuf`:

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
        // record duration, end span
    }
}
```

Use `opentelemetry-otlp` directly (NOT `opentelemetry-langfuse` -- bus
factor 1, 3 stars, breaks on every OTel minor bump). Construct basic-auth
header from env vars. Target any OTLP endpoint via `http/protobuf`.

**Config** (parsed from `[adapters.otel]` in roko.toml):
```toml
[adapters.otel]
provider = "langfuse"  # or "phoenix" | "honeycomb" | "grafana" | "laminar" | "otlp-generic"
endpoint = "https://cloud.langfuse.com/api/public/otel/v1/traces"
protocol = "http/protobuf"
auth = "basic"
semconv_opt_in = "gen_ai_latest_experimental"
```

**Effort**: ~200 LOC core exporter + ~80 LOC constants/attributes.

**Verification**:
```bash
cargo check -p roko-otel
rg 'gen_ai\.' crates/roko-otel/ --type rust | wc -l  # >= 6
```

---

### Task 13.8: Define `gen_ai.*` Span Hierarchy

**Files to create**:
- `crates/roko-otel/src/spans.rs`

**What to build**:
Define span hierarchy for agent execution:

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
pub const ROKO_GATEWAY_COST_USD: &str = "roko.gateway.cost_usd";
pub const ROKO_AGENT_CHAIN_ID: &str = "roko.agent.chain_id";
pub const ROKO_AGENT_TRIGGER_SOURCE: &str = "roko.agent.trigger_source";
```

Follow semconv >=1.37 naming for gen_ai attributes.

**Effort**: ~80 LOC.

**Verification**:
```bash
rg 'gen_ai\.\|roko\.' crates/roko-otel/src/spans.rs --type rust | wc -l  # >= 8
```

---

### Task 13.9: Wire `OtlpExporter` into `ModelCallService`

**Files to modify**:
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-cli/src/orchestrate.rs`

**What to build**:
After each `Agent::run()` call in `ModelCallService`, emit a span via
`ObservabilityExporter::export_turn()`. Thread the exporter from CLI startup
through `ModelCallService`:

```rust
let exporter = registry.get::<Box<dyn ObservabilityExporter>>();
let service = ModelCallService::new(model)
    .with_observability(exporter);
```

**Existing code to be aware of**:
- `ModelCallService` at `crates/roko-agent/src/model_call_service.rs` already
  tracks `TokenUsage`, `CostEstimate`, and emits `GatewayEvent`. The OTel
  export adds a span per model call using data already collected.
- Do not duplicate cost tracking -- reuse `CostEstimate` and `TokenUsage`.

**Effort**: ~80 LOC wiring across 2 files.

**Verification**:
```bash
rg 'ObservabilityExporter\|export_turn' crates/roko-agent/ --type rust | wc -l  # >= 2
# Integration test: set OTEL_EXPORTER_OTLP_ENDPOINT to a local collector, run:
# cargo run -p roko-cli -- run "hello world"
# Verify spans appear in collector output
```

---

### Task 13.10: Wire `OtlpExporter` into Gate Pipeline

**Files to modify**:
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-cli/src/orchestrate.rs`

**What to build**:
After each gate rung executes in `GateService`, emit a `roko.gate` span via
`ObservabilityExporter::export_gate_result()`:

```rust
if let Some(exporter) = &self.observability {
    exporter.export_gate_result(&GateResult {
        rung: rung_name,
        passed: verdict.passed,
        duration_ms: elapsed,
        output_lines: verdict.output.len(),
    }).await?;
}
```

Thread the exporter into `GateService` via a builder method:
```rust
let gate_service = GateService::new()
    .with_observability(registry.get::<Box<dyn ObservabilityExporter>>());
```

**Existing code to be aware of**:
- `GateService` at `gate_service.rs:26` already has `adaptive` field. Add
  `observability: Option<Arc<dyn ObservabilityExporter>>` alongside it.

**Effort**: ~60 LOC.

**Verification**:
```bash
rg 'export_gate_result\|ObservabilityExporter' crates/roko-gate/ --type rust | wc -l  # >= 1
cargo check -p roko-gate
```

---

## Phase 2: GitHub Integration (5 tasks)

**Goal**: Full PR lifecycle via `octocrab`. Webhook reception triggers agent
dispatch. GitHub Actions reusable workflow.

**Dependency**: Phase 0 (adapter traits). `octocrab` already in workspace
Cargo.toml at line 152.

---

### Task 13.11: Implement `GitHubAdapter` (VersionControlAdapter)

**New crate to create**: `crates/roko-github/`

**Dependencies**: `octocrab = "0.49"` (already workspace dep), `hmac`, `sha2`

**What to build**:
Implement `VersionControlAdapter` with octocrab:

```rust
pub struct GitHubAdapter {
    client: Octocrab,
    default_repo: (String, String),  // (owner, name)
}

impl VersionControlAdapter for GitHubAdapter {
    async fn create_branch(&self, repo: &str, branch: &str, from: &str) -> Result<()> { ... }
    async fn create_pr(&self, repo: &str, pr: &PullRequest) -> Result<PrId> { ... }
    async fn get_pr_diff(&self, pr: PrId) -> Result<String> { ... }
    async fn merge_pr(&self, pr: PrId, strategy: MergeStrategy) -> Result<()> { ... }
    fn capabilities(&self) -> VcsCapabilities { ... }
}
```

Auth: `OctocrabBuilder::app(app_id, key).build()` for GitHub App, or
`OctocrabBuilder::default().personal_token(token)` for PAT. Config-driven
via `[adapters.github]`.

**Existing code to be aware of**:
- `roko-mcp-github` at `crates/roko-mcp-github/src/main.rs` has its own
  `reqwest::blocking::Client` + raw REST calls to GitHub API. The new
  `GitHubAdapter` uses `octocrab` which is the proper async Rust SDK. Do NOT
  merge or extend `roko-mcp-github` -- it remains an MCP tool server. The
  adapter is used by orchestration code, not by agents.
- `octocrab` is already in workspace `Cargo.toml` and referenced by
  `roko-serve` and `roko-cli`.

**Effort**: ~350 LOC adapter + ~100 LOC auth + ~50 LOC types.

**Verification**:
```bash
cargo check -p roko-github
# Acceptance test (gated by ROKO_ACC=1, needs real token):
ROKO_ACC=1 cargo test -p roko-github -- github_adapter
```

---

### Task 13.12: Extend GitHub Webhook Handler for Agent Dispatch

**Files to modify**:
- `crates/roko-serve/src/routes/webhooks.rs`

**What to build**:
Extend the existing `github_webhook()` handler (which already verifies HMAC
and creates Engrams) to dispatch agent execution when configured:

- `issues` event with `labeled` action and a configured label (e.g.,
  `roko:plan`) -> trigger plan generation pipeline
- `pull_request` event with `opened` action -> trigger review agent
- `check_suite` event with `completed` action -> record CI result

The handler already converts to `Engram` at `webhooks.rs:38`. Add dispatch
logic after the engram is persisted:

```rust
// After engram persistence (existing code):
if let Some(adapter) = state.adapter_registry.get::<GitHubAdapter>() {
    match event_type.as_str() {
        "issues" if action == "labeled" && has_roko_label(&payload) => {
            tokio::spawn(handle_issue_to_plan(state.clone(), payload));
        }
        // ... other event types
    }
}
```

**Existing code to be aware of**:
- The webhook handler at `webhooks.rs:38-60` already does HMAC verification,
  Engram creation, and event bus publish. ADD to this flow, do not rewrite.
- `SubscriptionConfig` at `crates/roko-core/src/config/subscriptions.rs`
  defines event-to-template matching. Wire through this system.

**Effort**: ~200 LOC.

**Verification**:
```bash
cargo test -p roko-serve -- github_webhook_dispatch
```

---

### Task 13.13: Issue-to-Plan Pipeline

**Files to modify**:
- `crates/roko-serve/src/routes/webhooks.rs` (or new `integrations.rs` route module)
- `crates/roko-cli/src/orchestrate.rs`

**What to build**:
When a GitHub issue receives the configured label, trigger:

1. Fetch issue body + comments via `GitHubAdapter`
2. Create a PRD from the issue content (reuse `prd draft` logic)
3. Generate a plan via `prd plan`
4. Create a branch and begin execution
5. Update issue with plan link and status

Wire as an async handler spawned from the webhook endpoint.

**Existing code to be aware of**:
- `prd_publish_subscriber` in `crates/roko-serve/` already triggers
  `prd plan` on publish. The issue-to-plan pipeline is analogous: issue ->
  PRD creation -> plan generation -> execution.
- PRD lifecycle commands exist at `crates/roko-cli/src/commands/prd.rs` and
  `crates/roko-cli/src/prd.rs`. Reuse the draft creation logic.

**Effort**: ~200 LOC.

**Verification**:
```bash
cargo test -p roko-serve -- github_issue_to_plan
```

---

### Task 13.14: PR-from-Plan: Agent Creates PRs

**Files to modify**:
- `crates/roko-cli/src/orchestrate.rs`

**What to build**:
After a task in a plan run produces code changes, create a PR via
`GitHubAdapter::create_pr()`:

```rust
if let Some(github) = registry.get::<GitHubAdapter>() {
    let pr_id = github.create_pr(&repo, &PullRequest {
        title: format!("roko: {}", task.description),
        body: format!("Generated by roko plan `{}`\nTask: {}", plan_name, task.id),
        head: &task_branch,
        base: "main",
    }).await?;
    task_state.pr_url = Some(pr_url);
}
```

This is opt-in via `[adapters.github]` being enabled and `auto_pr = true`.

**Existing code to be aware of**:
- `orchestrate.rs` already tracks task state and branches. The PR creation
  hooks into the post-gate-success path.

**Effort**: ~80 LOC.

**Verification**:
```bash
rg 'create_pr\|GitHubAdapter' crates/roko-cli/src/orchestrate.rs --type rust | wc -l  # >= 2
```

---

### Task 13.15: GitHub Actions Reusable Workflow

**Files to create**:
- `.github/workflows/roko-gates.yml`

**What to build**:
A reusable GitHub Actions workflow that runs roko's gate pipeline on PRs:

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

Users reference via `uses: nunchi/roko-gates@v1`.

**Effort**: ~40 LOC YAML.

**Verification**:
```bash
test -f .github/workflows/roko-gates.yml
```

---

## Phase 3: Linear Integration (5 tasks)

**Goal**: Linear AgentSession integration. The gateway integration that Cursor
cannot ship and Devin validates at $500/month.

**Dependency**: Phase 0 (adapter traits).

---

### Task 13.16: Linear GraphQL Client

**New crate to create**: `crates/roko-linear/`

**Dependencies**: `graphql_client = "0.14"`, `reqwest`, `hmac`, `sha2`

**What to build**:
Typed GraphQL operations from Linear's schema:

```rust
mod scalars {
    pub type DateTime = chrono::DateTime<chrono::Utc>;
    pub type JSON = serde_json::Value;
    pub type UUID = uuid::Uuid;
    pub type TimelessDate = String;
}
```

Operations:
- `IssueQuery.graphql` -- fetch issue by ID
- `AgentActivityCreate.graphql` -- emit agent activity (thought/action/response/error)
- `IssueUpdateState.graphql` -- update issue workflow state
- `CommentCreate.graphql` -- post comment on issue
- `TeamQuery.graphql` -- query team info

No existing Rust Linear SDK on crates.io (v0.0.1 from 2022, abandoned).

**Effort**: ~250 LOC codegen + scalars + 5 operations.

**Verification**:
```bash
cargo check -p roko-linear
rg 'graphql_client' crates/roko-linear/ --type rust | wc -l  # >= 1
```

---

### Task 13.17: Linear Webhook Receiver with HMAC

**Files to modify**:
- `crates/roko-serve/src/routes/webhooks.rs`

**What to build**:
Add webhook receiver at `POST /webhooks/linear`:

```rust
async fn linear_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    // 1. HMAC-SHA256: raw body, hex-encoded, Linear-Signature header
    verify_linear_hmac(&headers, &body, &secret)?;
    // 2. Dedup via Linear-Delivery UUID
    // 3. Parse Linear-Event header
    // 4. Dispatch AgentSessionEvent
}
```

Key protocol details:
- Header: `Linear-Event: AgentSessionEvent`
- Two actions: `created` and `prompted`
- Stop signal: `prompted` with `agentActivity.signal: "stop"`
- HMAC-SHA256 over raw body, hex-encoded, `Linear-Signature` header
- `Linear-Delivery` UUID for dedup

**Existing code to be aware of**:
- `webhooks.rs` already has `github_webhook` and `slack_webhook` handlers.
  Follow the same pattern (HMAC verify -> Engram -> event bus).
- `verify_github_signature()` function exists. Create analogous
  `verify_linear_hmac()`.

**Effort**: ~150 LOC handler + ~120 LOC HMAC verification.

**Verification**:
```bash
rg 'linear_webhook\|Linear-Signature' crates/roko-serve/ --type rust | wc -l  # >= 2
cargo check -p roko-serve
```

---

### Task 13.18: AgentSession Event Handler

**Files to create**:
- `crates/roko-linear/src/agent_session.rs`

**What to build**:
Handle the two AgentSession actions (`created` and `prompted`):

```rust
pub async fn handle_agent_session(state: AppState, payload: AgentSessionPayload) -> Result<StatusCode> {
    match payload.action.as_str() {
        "created" => {
            // HTTP 200 within 5s budget
            // Spawn tokio::task for thought activity within 10s
            tokio::spawn(async move {
                linear_client.agent_activity_create(session_id, "thought", "Analyzing issue...").await?;
                let result = orchestrate_from_issue(&state, &issue).await?;
                linear_client.agent_activity_create(session_id, "response", &result.summary).await?;
            });
            Ok(StatusCode::OK)
        }
        "prompted" => {
            if payload.agent_activity.signal == Some("stop") {
                // Cancel running task
                return Ok(StatusCode::OK);
            }
            // Handle follow-up prompt
        }
    }
}
```

Key: emit-then-async pattern. HTTP 200 within 5s, first mutation within 10s.
5 server-validated activity types: `thought`, `elicitation`, `action`,
`response`, `error`. `promptContext` is XML-formatted markup.

**Effort**: ~200 LOC.

**Verification**:
```bash
cargo test -p roko-linear -- agent_session
```

---

### Task 13.19: Implement `LinearWorkSource` (WorkSource Adapter)

**Files to create**:
- `crates/roko-linear/src/work_source.rs`

**What to build**:
Implement `WorkSource` for Linear:

```rust
impl WorkSource for LinearWorkSource {
    async fn fetch_candidates(&self) -> Result<Vec<WorkItem>> {
        // GraphQL query for active issues in configured team
    }
    async fn update_state(&self, id: &str, state: &str) -> Result<()> {
        // Map state name to Linear workflow state ID via mutation
    }
    fn capabilities(&self) -> WorkSourceCapabilities {
        WorkSourceCapabilities { pull: true, push: true, write: true, enrich: true }
    }
}
```

**Effort**: ~120 LOC.

**Verification**:
```bash
rg 'LinearWorkSource\|WorkSource' crates/roko-linear/ --type rust | wc -l  # >= 2
```

---

### Task 13.20: End-to-End Linear Chain (Chain B)

**Files to modify**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-serve/src/routes/webhooks.rs`

**What to build**:
Wire the full Chain B flow:

```
Linear issue webhook -> fetch issue -> create plan -> execute tasks
  -> create GitHub PR -> run CI -> update Linear issue status
```

This chains `LinearWorkSource` + `GitHubAdapter` + gate pipeline + status
updates:

1. Linear webhook fires `AgentSessionEvent`
2. Handler fetches issue via `LinearWorkSource::fetch_candidates()`
3. Creates a plan from issue description
4. Dispatches agent via orchestrate.rs
5. Agent creates PR via `GitHubAdapter::create_pr()`
6. Gates validate (compile, test, clippy)
7. Updates Linear issue via `LinearWorkSource::update_state("done")`
8. Posts PR link as Linear comment

**Dependency**: Task 13.11 (GitHubAdapter), Task 13.19 (LinearWorkSource).

**Effort**: ~200 LOC integration glue.

**Verification**:
```bash
cargo test -p roko-serve -- linear_chain_b
```

---

## Phase 4: Slack + Sentry (4 tasks)

**Goal**: Ship the killer demo (Chain D: Slack thread -> agent -> trace URL in
reply) and the error-to-fix loop (Chain A: Sentry -> plan -> PR).

**Dependency**: Phase 0, Phase 1 (OTel for trace URLs), Phase 2 (GitHub for PRs).

---

### Task 13.21: Implement `SlackAdapter` (NotificationAdapter)

**New module to create**: `crates/roko-serve/src/adapters/slack.rs`

**Dependencies**: `slack-morphism = "2.18"` with `axum` feature flag

**What to build**:
Implement `NotificationAdapter` for Slack using `slack-morphism`:

```rust
pub struct SlackAdapter {
    client: SlackClient<SlackClientHyperConnector>,
    default_channel: SlackChannelId,
}

impl NotificationAdapter for SlackAdapter {
    async fn send(&self, msg: &Notification) -> Result<()> { ... }
    async fn send_threaded(&self, thread_ts: &str, msg: &Notification) -> Result<()> {
        // Post to thread with trace URL, cost summary, gate results
    }
    fn capabilities(&self) -> NotificationCapabilities { ... }
}
```

Use Socket Mode (no public webhook endpoint required). Render Block Kit
messages with PR links, cost summaries, and gate results.

**Existing code to be aware of**:
- `roko-mcp-slack` at `crates/roko-mcp-slack/src/main.rs` uses
  `reqwest::blocking::Client` with raw Slack Web API calls. The new
  `SlackAdapter` uses `slack-morphism` which provides typed API, Block Kit,
  and Socket Mode. Keep both -- `roko-mcp-slack` is the MCP tool server for
  agents, `SlackAdapter` is the orchestration notification layer.

**Effort**: ~250 LOC adapter + Block Kit rendering.

**Verification**:
```bash
cargo check -p roko-serve
rg 'SlackAdapter\|NotificationAdapter' crates/roko-serve/ --type rust | wc -l  # >= 2
```

---

### Task 13.22: Slack Command Handler (`/roko`)

**Files to modify**:
- `crates/roko-serve/src/routes/webhooks.rs` (or new route module)

**What to build**:
Handle `/roko fix #ENG-123` slash command:

```rust
async fn slack_command(State(state): State<Arc<AppState>>, Form(cmd): Form<SlackSlashCommand>) -> Result<Response> {
    let (action, issue_ref) = parse_roko_command(&cmd.text)?;
    let ack = json!({ "response_type": "in_channel", "text": "On it..." });
    tokio::spawn(async move {
        let issue = resolve_issue(&state, &issue_ref).await?;
        let result = orchestrate_from_issue(&state, &issue).await?;
        state.slack.send_threaded(&cmd.channel_id, &Notification {
            title: format!("Fixed {}", issue_ref),
            body: result.summary,
            trace_url: result.trace_url,
            pr_url: result.pr_url,
            cost: result.total_cost_usd,
        }).await?;
    });
    Ok(Json(ack).into_response())
}
```

Ack within Slack's 3s timeout, then spawn async handler. Reply in thread
with trace URL (from OTel), PR link (from GitHub), cost summary.

**Effort**: ~150 LOC.

**Verification**:
```bash
rg 'slack_command\|SlackSlashCommand' crates/roko-serve/ --type rust | wc -l  # >= 2
```

---

### Task 13.23: Implement `SentryAdapter` via MCP

**Files to create**:
- `crates/roko-serve/src/adapters/sentry.rs`

**Dependencies**: `rmcp` (official Rust MCP crate)

**What to build**:
Use Sentry's MCP server to fetch issue details and resolve issues:

```rust
pub struct SentryAdapter {
    mcp_client: McpClient,
}

impl SentryAdapter {
    pub async fn get_issue_details(&self, issue_id: &str) -> Result<SentryIssue> {
        let result = self.mcp_client.call_tool("getIssueDetails", json!({ "issue_id": issue_id })).await?;
        Ok(serde_json::from_value(result)?)
    }
    pub async fn resolve_issue(&self, issue_id: &str) -> Result<()> {
        self.mcp_client.call_tool("resolveIssue", json!({ "issue_id": issue_id })).await?;
        Ok(())
    }
}
```

**Effort**: ~150 LOC.

**Verification**:
```bash
cargo check -p roko-serve
rg 'SentryAdapter' crates/roko-serve/ --type rust | wc -l  # >= 1
```

---

### Task 13.24: Wire Chain A: Sentry -> Plan -> PR -> Resolve

**Files to modify**:
- `crates/roko-serve/src/routes/webhooks.rs`

**What to build**:
Sentry webhook -> fetch issue details via MCP -> create plan -> agent
generates fix -> PR via GitHubAdapter -> on merge, resolve Sentry issue
+ close Linear issue:

```rust
async fn sentry_webhook(state: AppState, payload: SentryPayload) -> Result<StatusCode> {
    let issue = state.sentry.get_issue_details(&payload.issue_id).await?;
    let plan = create_fix_plan(&issue)?;
    let result = orchestrate_plan(&state, &plan).await?;
    let pr_id = state.github.create_pr(&repo, &pr).await?;
    state.on_pr_merge(pr_id, |state| async {
        state.sentry.resolve_issue(&issue.id).await?;
        if let Some(linear_id) = issue.linear_issue_id {
            state.linear.update_state(&linear_id, "done").await?;
        }
    });
    Ok(StatusCode::OK)
}
```

**Dependency**: Task 13.11 (GitHubAdapter), Task 13.23 (SentryAdapter).

**Effort**: ~200 LOC.

**Verification**:
```bash
cargo test -p roko-serve -- sentry_chain_a
```

---

## Phase 5: Plugin System + Recipe Schema (7 tasks)

**Goal**: Ship `connector.toml` declarative format, `recipe.toml` composition
schema, plugin discovery, and adapter marketplace foundation.

**Dependency**: Phase 0 (adapter traits). Can run in parallel with Phases 2-4.

---

### Task 13.25: Define `connector.toml` Schema

**Files to create**:
- `crates/roko-core/src/connector_manifest.rs`

**What to build**:
Declarative connector format for 80% of REST API integrations:

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
    pub kind: ConnectorProtocol,  // rest | graphql | grpc | stdio
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

**Existing code to be aware of**:
- `ConnectorConfig` at `crates/roko-core/src/connector.rs` is the legacy
  connector struct with `ConnectorKind::Mcp | Api | Database | ...`. The
  new `ConnectorManifest` is a TOML-file schema for declarative connectors,
  NOT a replacement for the runtime `ConnectorConfig`. They serve different
  purposes.

**Effort**: ~200 LOC schema + parser.

**Verification**:
```bash
rg 'ConnectorManifest' crates/roko-core/ --type rust | wc -l  # >= 1
cargo check -p roko-core
```

---

### Task 13.26: Generate `WorkSource` from `connector.toml`

**Files to create**:
- `crates/roko-core/src/connector_runtime.rs`

**What to build**:
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

This means any REST API can become a `WorkSource` with zero Rust code -- just
a `connector.toml` file.

**Effort**: ~200 LOC.

**Verification**:
```bash
cargo test -p roko-core -- declared_work_source
```

---

### Task 13.27: Define `recipe.toml` Schema

**Files to create**:
- `crates/roko-core/src/recipe.rs`

**What to build**:
Composition schema that wires multiple adapters into closed loops:

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
pub struct RecipeStep {
    pub id: String,
    pub adapter: String,
    pub action: String,
    pub inputs: HashMap<String, String>,  // supports ${{ parameters.x }} interpolation
    pub depends_on: Vec<String>,
}

#[derive(Deserialize)]
pub struct AdapterRef {
    pub version: String,
    pub features: Vec<String>,
}
```

Cargo-style `[adapters]` table with SemVer constraints.

**Effort**: ~180 LOC schema.

**Verification**:
```bash
rg 'Recipe\b' crates/roko-core/src/recipe.rs --type rust | wc -l  # >= 3
cargo check -p roko-core
```

---

### Task 13.28: Recipe Executor

**Files to create**:
- `crates/roko-core/src/recipe_executor.rs`

**What to build**:
Execute a recipe by resolving adapter references, interpolating parameters,
and running steps in dependency order:

```rust
pub struct RecipeExecutor {
    registry: AdapterRegistry,
}

impl RecipeExecutor {
    pub async fn run(&self, recipe: &Recipe, params: &HashMap<String, String>) -> Result<RecipeResult> {
        let ordered = topo_sort(&recipe.steps)?;
        let mut outputs: HashMap<String, Value> = HashMap::new();
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

Topological sort by `depends_on`. `${{ parameters.x }}` and
`${{ steps.id.output }}` interpolation.

**Effort**: ~200 LOC.

**Verification**:
```bash
cargo test -p roko-core -- recipe_executor
```

---

### Task 13.29: `roko recipe run` CLI Command

**Files to modify**:
- `crates/roko-cli/src/commands/mod.rs`

**Files to create**:
- `crates/roko-cli/src/commands/recipe.rs`

**What to build**:
CLI commands:
```bash
roko recipe run path/to/recipe.toml --param github_repo=nunchi/roko
roko recipe list     # list available recipes in .roko/recipes/
roko recipe show     # show recipe details
roko recipe validate # validate recipe schema
```

**Effort**: ~120 LOC.

**Verification**:
```bash
cargo run -p roko-cli -- recipe --help
```

---

### Task 13.30: `roko adapter scaffold` CLI Command

**Files to create**:
- `crates/roko-cli/src/commands/adapter.rs`

**What to build**:
Scaffold new adapters:

```bash
roko adapter scaffold --name my-tracker --kind rest --trait WorkSource
# Creates:
#   adapters/my-tracker/connector.toml
#   adapters/my-tracker/src/lib.rs
#   adapters/my-tracker/Cargo.toml

roko adapter scaffold --name my-api --kind rest --declarative
# Creates only connector.toml
```

Template files for each `--trait` variant. Declarative mode produces only
`connector.toml` with example endpoints.

**Effort**: ~150 LOC + templates.

**Verification**:
```bash
cargo run -p roko-cli -- adapter scaffold --help
```

---

### Task 13.31: Adapter Discovery via `roko.toml`

**Files to modify**:
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-cli/src/main.rs`

**What to build**:
`roko.toml` declares required adapters; CLI resolves them at startup:

```toml
[adapters.dependencies]
github = { version = "^1.0", source = "builtin" }
linear = { version = "^0.5", source = "builtin" }
my-tracker = { version = "^0.1", source = "path", path = "./adapters/my-tracker" }
custom-api = { version = "^0.2", source = "connector", path = "./connectors/custom-api.toml" }
```

Resolution:
- `builtin`: compiled into roko binary
- `path`: local crate, loaded via subprocess or dynamic library
- `connector`: declarative `connector.toml`, loaded at runtime

**Effort**: ~150 LOC.

**Verification**:
```bash
rg 'adapter.*dependencies\|AdapterDep' crates/roko-core/ --type rust | wc -l  # >= 2
```

---

## Phase 6: Gateway Adapter Layers (5 tasks)

**Goal**: Make the gateway pipeline's layers adapter-driven. Each layer becomes
a trait boundary that external implementations can plug into.

**Dependency**: Phase 0 (adapter traits). Can run in parallel with Phases 2-4.

---

### Task 13.32: `CacheLayer` Trait + Moka L1 Implementation

**Files to modify**:
- `crates/roko-serve/src/lib.rs` (or new `gateway` module)

**What to build**:

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

**Effort**: ~180 LOC.

**Verification**:
```bash
rg 'CacheLayer\|MokaL1Cache' crates/roko-serve/ --type rust | wc -l  # >= 2
cargo check -p roko-serve
```

---

### Task 13.33: Safety Pipeline Traits

**Files to create**:
- `crates/roko-core/src/adapters/safety.rs`

**What to build**:

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

Ship regex-based default implementations. External ML-based scanners connect
via the same trait.

**Existing code to be aware of**:
- Safety layer exists at `crates/roko-agent/src/safety/`. These traits are
  for the gateway pipeline's request/response safety layer, not the agent
  safety contracts.

**Effort**: ~150 LOC traits + regex defaults.

**Verification**:
```bash
rg 'PiiScanner\|InjectionDetector' crates/roko-core/ --type rust | wc -l  # >= 2
cargo check -p roko-core
```

---

### Task 13.34: `Optimizer` Pipeline Trait

**Files to create**:
- `crates/roko-core/src/adapters/optimizer.rs`

**What to build**:

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
- `OutputBudgetOptimizer`: set `max_tokens` based on task complexity
- `LoopDetectOptimizer`: detect oscillation patterns in agent tool calls

**Effort**: ~250 LOC traits + 3 implementations.

**Verification**:
```bash
rg 'Optimizer\|ToolPrune\|OutputBudget\|LoopDetect' crates/roko-core/ --type rust | wc -l  # >= 4
```

---

### Task 13.35: `BillingProvider` Trait + Stripe Skeleton

**Files to create**:
- `crates/roko-core/src/adapters/billing.rs`

**What to build**:

```rust
#[async_trait]
pub trait BillingProvider: Send + Sync {
    async fn authorize(&self, key: &ApiKey, estimated_cost: f64) -> Result<BillingAuth>;
    async fn record(&self, key: &ApiKey, actual_cost: f64, metadata: &UsageMetadata) -> Result<()>;
    async fn check_budget(&self, key: &ApiKey) -> Result<BudgetStatus>;
}

pub struct StripeBilling {
    client: stripe::Client,
    meter_id: String,
}
```

Stripe skeleton only (full integration is post-MVP). Trait definition is
the deliverable.

**Existing code to be aware of**:
- `BudgetGuardrail` and `CostTable` exist in `roko-agent`. The
  `BillingProvider` is for external billing, not internal budget tracking.

**Effort**: ~120 LOC trait + skeleton.

**Verification**:
```bash
rg 'BillingProvider\|StripeBilling' crates/roko-core/ --type rust | wc -l  # >= 2
```

---

### Task 13.36: Wire Gateway Layers into `roko-serve`

**Files to modify**:
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-serve/src/state.rs`

**What to build**:
Compose all gateway layers via `AdapterRegistry`:

```rust
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

**Existing code to be aware of**:
- `crates/roko-serve/src/routes/gateway.rs` already exists with
  `ModelCallService`. The pipeline wraps it with pre/post processing layers.
- `AppState` at `crates/roko-serve/src/state.rs` holds `ModelCallService`,
  `CascadeRouter`, `ProviderHealthTracker`. The pipeline struct bundles
  these with the new adapter-trait layers.

**Effort**: ~200 LOC pipeline composition.

**Verification**:
```bash
rg 'GatewayPipeline' crates/roko-serve/ --type rust | wc -l  # >= 2
cargo test -p roko-serve -- gateway_pipeline
```

---

## Phase 7: Marketplace Foundation + Verticals (7 tasks)

**Goal**: Adapter registry, verification badges, `roko-contrib` scaffolding,
and vertical-specific role templates for P0 markets.

**Dependency**: Phase 0 (traits) + Phase 5 (plugin system).

---

### Task 13.37: Adapter Registry Protocol

**Files to create**:
- `crates/roko-core/src/adapter_registry_protocol.rs`

**What to build**:

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

**Effort**: ~200 LOC.

**Verification**:
```bash
rg 'AdapterRegistryClient\|AdapterRegistryEntry' crates/roko-core/ --type rust | wc -l  # >= 2
```

---

### Task 13.38: Verification Badge System

**Files to create**:
- `crates/roko-core/src/verification.rs`

**What to build**:

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
    CommunityTested,   // conformance + CI only
    RokoVerified,      // conformance + CI + reviewer
}
```

Earning verification requires: passing conformance tests (from Task 13.6),
CI green in roko-contrib, reviewed by a maintainer.

**Effort**: ~100 LOC.

---

### Task 13.39: `roko adapter list/search/install` CLI Commands

**Files to modify**:
- `crates/roko-cli/src/commands/adapter.rs` (from Task 13.30)

**What to build**:

```bash
roko adapter list                        # list installed adapters
roko adapter search "linear"             # search registry
roko adapter install github@^1.0         # install from registry
roko adapter verify my-adapter           # run conformance tests
roko adapter info github                 # show adapter details + capabilities
```

**Effort**: ~200 LOC.

---

### Task 13.40: Semgrep `SecurityScanner` Implementation

**Files to create**:
- `crates/roko-serve/src/adapters/semgrep.rs`

**What to build**:
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

Wire as a new gate rung (`security-scan`) in the configurable gate pipeline.
SARIF output parsed into `Finding` structs.

**Existing code to be aware of**:
- `SecurityScanGate` at `crates/roko-gate/src/security_scan_gate.rs` runs
  `cargo audit`. The new `SemgrepScanner` is a separate security tool that
  scans source code, not dependencies.

**Effort**: ~150 LOC.

**Verification**:
```bash
rg 'SemgrepScanner\|SecurityScanner' crates/roko-serve/ --type rust | wc -l  # >= 2
```

---

### Task 13.41: `AlertSource` Adapter Trait (DevOps/SRE Vertical)

**Files to create**:
- `crates/roko-core/src/adapters/alert.rs`

**What to build**:

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

Enables PagerDuty/Datadog -> auto-diagnosis -> fix PR -> deploy workflow.

**Effort**: ~100 LOC trait + types.

---

### Task 13.42: `AuditAdapter` Trait (Compliance Vertical)

**Files to create**:
- `crates/roko-core/src/adapters/audit.rs`

**What to build**:

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
    pub gate_results: Vec<GateVerdict>,
    pub timestamp: DateTime<Utc>,
}
```

Wire gate results into audit records in `orchestrate.rs`:

```rust
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

**Effort**: ~180 LOC trait + wiring.

---

### Task 13.43: Vertical Role Templates

**Files to create**:
- `crates/roko-compose/src/templates/sre.rs`
- `crates/roko-compose/src/templates/security.rs`
- `crates/roko-compose/src/templates/compliance.rs`

**What to build**:
Role templates for P0 market verticals:

```rust
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

Similar templates for `security-analyst` and `compliance-auditor`.

**Existing code to be aware of**:
- Role templates exist at `crates/roko-compose/src/templates/`. Follow the
  existing pattern.

**Effort**: ~150 LOC across 3 templates.

---

## Dependencies Between Phases

```
Phase 0 (Adapter Foundation) [Tasks 13.1-13.6]
  |
  +---> Phase 1 (OTel) [13.7-13.10] -- no external deps
  |       |
  +---> Phase 2 (GitHub) [13.11-13.15] -- needs octocrab (already in workspace)
  |       |
  +---> Phase 3 (Linear) [13.16-13.20] -- needs graphql_client
  |       |         |
  |       v         v
  +---> Phase 4 (Slack + Sentry) [13.21-13.24] -- needs Phase 2 for Chain A
  |
  +---> Phase 5 (Plugin System) [13.25-13.31] -- can run parallel with 2-4
  |
  +---> Phase 6 (Gateway Layers) [13.32-13.36] -- can run parallel with 2-4
  |
  +---> Phase 7 (Marketplace + Verticals) [13.37-13.43] -- needs Phase 0 + Phase 5
```

Phase 0 is the critical path. Phases 1-4 proceed in parallel once Phase 0
lands. Phases 5-6 run in parallel with integration work. Phase 7 depends on
traits (Phase 0) and plugin system (Phase 5).

---

## 90-Day Shipping Sequence

| Weeks | Deliverable | Demo |
|---|---|---|
| 1-3 | Phase 0 (traits) + Phase 1 (OTel) | `roko run` produces `gen_ai.*` traces visible in Langfuse |
| 3-6 | Phase 2 (GitHub) | Issue labeled -> agent creates PR -> gates validate |
| 6-9 | Phase 3 (Linear) | Linear issue -> agent session -> PR -> issue closed |
| 9-11 | Phase 4 (Slack + Sentry) | `/roko fix #ENG-123` -> threaded reply with trace URL |
| 11-13 | Phase 5 (Plugin System) | `roko adapter scaffold` + `roko recipe run` |

Phases 6-7 extend beyond 90 days but are independently valuable.

---

## Named Integration Chains (Wired by End of Phase 4)

| Chain | Flow | Phase |
|---|---|---|
| **A** | Sentry issue -> plan -> GitHub PR -> OTel trace -> Linear closed | Phase 4 |
| **B** | Linear webhook -> plan -> GitHub PR -> CI -> Linear status | Phase 3 |
| **C** | GitHub label -> plan -> PR -> Slack approval -> merge | Phase 4 |
| **D** | Slack `/roko` -> agent -> tool use -> Slack reply with trace URL | Phase 4 |
| **E** | `recipe.toml` composition (one import -> 5+ adapters wired) | Phase 5 |

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
| Langfuse re-license (ClickHouse 2027-2028) | Use `opentelemetry-otlp` directly, not Langfuse-specific crate. One env var change repoints to any OTLP endpoint |
| MCP quality crisis (52% abandonment) | Position as quality layer with verification badge. Ship conformance crate |
| Contributor ramp is slow | Start with 10-20 Roko Verified adapters. Declarative connector.toml lowers barrier |
| Gateway-standalone market caps at ~$50M ARR | Gateway is data-acquisition layer for orchestration platform, not standalone product |
| EU AI Act Article 50 timeline shifts | Audit adapter is independently valuable regardless of enforcement date |
