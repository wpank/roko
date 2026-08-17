# Inference & Dispatch: Implementation Plan

Phased plan for completing the inference dispatch unification. Each phase has concrete
file paths, acceptance criteria, and grep gates.

---

## Phase 0: Wire CascadeRouter to Live Paths (1-2 days)

**Goal:** CascadeRouter provides adaptive model selection for all major CLI entry points.

### 0.1 Load CascadeRouter at Startup

**File:** `crates/roko-cli/src/model_selection.rs`

Add a function to load or initialize a CascadeRouter from the workspace:

```rust
pub fn load_cascade_router(workdir: &Path, config: &RokoConfig) -> CascadeRouter {
    let path = workdir.join(".roko/learn/cascade-router.json");
    let model_slugs = config.effective_models()
        .values()
        .map(|p| p.slug.clone())
        .collect();
    CascadeRouter::load_or_new(&path, model_slugs)
}
```

Change `resolve_effective_model_key()` to accept `Option<&CascadeRouter>` and pass it
through to `resolve_effective_model()`.

### 0.2 Thread Router Through CLI Commands

**Files:**
- `crates/roko-cli/src/run.rs` -- `roko run` path
- `crates/roko-cli/src/chat_inline.rs` -- `roko chat` path
- `crates/roko-cli/src/run_inline.rs` -- `roko <prompt>` path
- `crates/roko-cli/src/commands/plan.rs` -- `roko plan run` path

Each entry point loads the router and passes it to model selection:
```rust
let router = load_cascade_router(&workdir, &config);
let selection = resolve_effective_model(
    cli_model, task_hint, role,
    Some(&router),
    &config,
)?;
```

### 0.3 Record Observations After Each Call

**File:** `crates/roko-agent/src/model_call_service.rs`

After `Agent::run()` completes in `ModelCallService::call()`, record the observation:
```rust
if let Some(router) = &self.cascade_router_observer {
    router.observe(model_slug, success, latency_ms, cost_usd, context_features);
}
```

Add a `CascadeRouterObserver` trait (or reuse `ForceBackendOverrideRecorder` with
richer interface) and wire it to the router.

### 0.4 Persist on Shutdown

**File:** `crates/roko-cli/src/main.rs` or relevant CLI command handlers

After command completion, persist the router state:
```rust
router.save(&workdir.join(".roko/learn/cascade-router.json"))?;
```

### Acceptance Criteria

```bash
# CascadeRouter is loaded from at least 3 CLI entry points
rg 'load_cascade_router\|CascadeRouter::load' crates/roko-cli/src/ --type rust \
  | grep -v test | wc -l  # >= 3

# resolve_effective_model receives Some(router) from live paths
rg 'resolve_effective_model\(' crates/roko-cli/src/ --type rust \
  | grep 'Some(' | grep -v test | wc -l  # >= 3

# Router is persisted after use
rg 'cascade-router\.json' crates/roko-cli/src/ --type rust \
  | grep 'save\|persist\|write' | wc -l  # >= 1
```

---

## Phase 1: Episode Logging for All Paths (1-2 days)

**Goal:** Every LLM call from any entry point produces a durable episode record.

### 1.1 Add Episode Recording to ModelCallService

**File:** `crates/roko-agent/src/model_call_service.rs`

`ModelCallService` already records `FeedbackEvent::ModelCall` via `FeedbackSink`.
Extend this to also emit a structured episode record:

```rust
// After successful call:
self.emit(RuntimeEvent::Episode {
    run_id: request_id.clone(),
    model: model.to_string(),
    role: req.role.clone(),
    input_tokens: usage.input_tokens,
    output_tokens: usage.output_tokens,
    cost_usd: usage.cost_usd,
    latency_ms,
    success,
    timestamp: Utc::now(),
});
```

### 1.2 Wire Episode Persistence in CLI

**Files:**
- `crates/roko-cli/src/run_inline.rs`
- `crates/roko-cli/src/chat_inline.rs`

Add an `EventConsumer` that appends to `.roko/episodes.jsonl`:
```rust
let episode_logger = JsonlEpisodeLogger::new(workdir.join(".roko/episodes.jsonl"));
let service = ModelCallService::new(model)
    .with_event_consumer(Arc::new(episode_logger))
    .with_feedback_sink(feedback_sink);
```

### 1.3 Unify Episode Format

**File:** `crates/roko-runtime/src/jsonl_logger.rs`

Ensure the episode format matches what `runner/event_loop.rs` already writes:
```json
{
  "timestamp": "2026-04-29T12:00:00Z",
  "run_id": "model-call-service:1:000000000000001",
  "model": "claude-sonnet-4-6",
  "role": "implementer",
  "input_tokens": 1234,
  "output_tokens": 567,
  "cost_usd": 0.012,
  "latency_ms": 3200,
  "success": true,
  "entry_point": "roko_run"
}
```

### Acceptance Criteria

```bash
# episodes.jsonl is written from one-shot paths
cargo run -p roko-cli -- run "echo hello" 2>/dev/null
test -f .roko/episodes.jsonl  # should exist
wc -l .roko/episodes.jsonl  # should be >= 1

# ModelCallService emits episode events
rg 'RuntimeEvent::Episode\|emit.*episode' crates/roko-agent/src/model_call_service.rs \
  | wc -l  # >= 1
```

---

## Phase 2: ACP Provider Migration (1-2 days)

**Goal:** ACP bridge uses provider adapters instead of bare subprocess spawns.

### 2.1 Replace run_claude_cli() in ACP Runner

**File:** `crates/roko-acp/src/runner.rs`

Replace:
```rust
// OLD: bare subprocess
Command::new("claude")
    .args(["--print", "--dangerously-skip-permissions"])
    .arg(&prompt)
```

With:
```rust
// NEW: provider adapter
let agent = create_agent_for_model(&config, "claude-sonnet-4-6", &options)?;
let result = agent.run(&engram, &context).await;
```

### 2.2 Replace run_claude_cognitive_task() in Bridge Events

**File:** `crates/roko-acp/src/bridge_events.rs`

Replace the direct `Command::new("claude")` spawn at line ~545 with
`create_agent_for_model()`. The function already has a model parameter --
just route it through the provider system.

### 2.3 Replace run_openai_compat_cognitive_task()

**File:** `crates/roko-acp/src/bridge_events.rs`

The OpenAI-compat path already uses `resolve_model()` but builds its own HTTP
client. Replace with the `OpenAiCompatAdapter` from the provider system.

### 2.4 Add RokoConfig to ACP Context

**Files:**
- `crates/roko-acp/src/types.rs` (or pipeline context)
- `crates/roko-acp/Cargo.toml` (add roko-agent dependency if missing)

The ACP pipeline needs access to `RokoConfig` for provider resolution. Thread
it through the pipeline context.

### Acceptance Criteria

```bash
# No more Command::new("claude") in ACP crate
rg 'Command::new\("claude"\)' crates/roko-acp/ --type rust | grep -v test  # should be 0

# ACP uses create_agent_for_model
rg 'create_agent_for_model' crates/roko-acp/ --type rust | grep -v test | wc -l  # >= 2
```

---

## Phase 3: Provider Health & Circuit Breaker (2-3 days)

**Goal:** Automatic provider failover when a backend is unhealthy.

### 3.1 Implement ProviderHealthTracker

**New file:** `crates/roko-agent/src/provider/health.rs`

```rust
pub enum ProviderState {
    Healthy,
    Degraded,  // >10% error rate in last 5 minutes
    Open,      // >50% error rate or 5 consecutive failures
    HalfOpen,  // probing after cooldown
}

pub struct ProviderHealthTracker {
    states: HashMap<String, ProviderHealthState>,
    error_windows: HashMap<String, SlidingWindow>,
    cooldown: Duration,
}

impl ProviderHealthTracker {
    pub fn record_success(&self, provider_key: &str);
    pub fn record_failure(&self, provider_key: &str, error: &ProviderError);
    pub fn is_healthy(&self, provider_key: &str) -> bool;
    pub fn state(&self, provider_key: &str) -> ProviderState;
}
```

### 3.2 Wire Health Checks into ModelCallService

**File:** `crates/roko-agent/src/model_call_service.rs`

Before dispatching, check provider health:
```rust
let provider_key = self.provider_for_model(&model);
if let Some(key) = &provider_key {
    if !self.health_tracker.is_healthy(key) {
        // Try fallback models
        for fallback in self.fallback_models_for_request(&model) {
            let fallback_provider = self.provider_for_model(&fallback);
            if fallback_provider.as_deref().map_or(true, |k| self.health_tracker.is_healthy(k)) {
                model = fallback;
                break;
            }
        }
    }
}
```

### 3.3 Expose Health via API

**File:** `crates/roko-serve/src/routes/providers.rs`

Add endpoint `GET /api/providers/health`:
```json
{
  "providers": {
    "anthropic_api": { "state": "healthy", "error_rate": 0.02, "latency_p50_ms": 1200 },
    "claude_cli": { "state": "healthy", "error_rate": 0.0, "latency_p50_ms": 3400 },
    "cerebras_api": { "state": "degraded", "error_rate": 0.15, "latency_p50_ms": 200 }
  }
}
```

### 3.4 Add Retry Logic

**File:** `crates/roko-agent/src/model_call_service.rs`

Add configurable retry policy:
```rust
pub struct RetryPolicy {
    max_retries: u32,              // default: 2
    backoff_base_ms: u64,          // default: 1000
    backoff_max_ms: u64,           // default: 30000
    retryable: HashSet<ErrorKind>, // RateLimit, ServerError, Timeout
}
```

### Acceptance Criteria

```bash
# ProviderHealthTracker exists and is used
rg 'ProviderHealthTracker' crates/roko-agent/ --type rust | wc -l  # >= 3

# Health endpoint exists
rg 'providers/health' crates/roko-serve/ --type rust | wc -l  # >= 1

# RetryPolicy is configurable
rg 'RetryPolicy\|with_retry' crates/roko-agent/src/model_call_service.rs | wc -l  # >= 2
```

---

## Phase 4: Budget Enforcement (1 day)

**Goal:** Mandatory cost budgets with graceful degradation.

### 4.1 Add Budget Configuration to roko.toml

**File:** `crates/roko-core/src/config/schema.rs`

```toml
[budget]
per_turn_usd = 0.50
per_session_usd = 10.00
per_plan_usd = 100.00
warn_at = [0.50, 0.75, 0.90]
on_exceeded = "downgrade"  # "downgrade" | "fail" | "warn"
```

### 4.2 Set Default Budgets in ModelCallService

**File:** `crates/roko-agent/src/model_call_service.rs`

Change `ModelCallService::new()` to set a default budget:
```rust
pub fn new(default_model: String) -> Self {
    Self {
        budget: BudgetCell::new(Some(10.0)), // $10.00 default session budget
        // ...
    }
}
```

### 4.3 Implement Graceful Degradation

When budget is 90%+ consumed, try cheaper model before failing:
```rust
if self.budget.remaining_fraction() < 0.10 {
    // Find cheapest viable model
    let cheapest = self.cheapest_model_for_task(&req);
    if cheapest != model {
        tracing::warn!(budget_remaining = self.budget.remaining(),
            "budget low, switching from {} to {}", model, cheapest);
        model = cheapest;
    }
}
```

### 4.4 Budget Warnings in TUI

**File:** `crates/roko-cli/src/tui/cost_panel.rs` (or similar)

Emit budget warning events at configured thresholds:
```rust
for threshold in &config.budget.warn_at {
    if budget.fraction_used() >= *threshold && !warned.contains(threshold) {
        emit(RuntimeEvent::BudgetWarning { threshold, used, remaining });
        warned.insert(*threshold);
    }
}
```

### Acceptance Criteria

```bash
# Budget config exists in schema
rg 'per_turn_usd\|per_session_usd\|per_plan_usd' crates/roko-core/ --type rust | wc -l  # >= 1

# Default budget is non-None
rg 'BudgetCell::new\(Some' crates/roko-agent/src/model_call_service.rs | wc -l  # >= 1

# Budget warnings are emitted
rg 'BudgetWarning\|budget.*warn' crates/roko-agent/ --type rust | wc -l  # >= 1
```

---

## Phase 5: Stream Parser Consolidation (1 day)

**Goal:** One Claude stream-json parser used everywhere.

### 5.1 Promote Canonical Parser

**File:** `crates/roko-agent/src/provider/claude_cli/stream.rs`

Ensure `parse_stream_line()` returns all needed data:
- Text content deltas
- Tool outputs (name + content)
- Usage info (input/output/cached/thought tokens)
- Session ID
- Model name
- Error messages

### 5.2 Replace Duplicate Parsers

**Files to modify:**
- `crates/roko-agent/src/translate/mod.rs` -- replace `extract_text()` and
  `extract_tool_outputs()` inline parsing with calls to `parse_stream_line()`
- `crates/roko-cli/src/chat.rs` -- replace `extract_clean_text()` with typed
  deserialization using `parse_stream_line()`

### 5.3 Add Shared Truncation Utility

**File:** `crates/roko-agent/src/provider/claude_cli/stream.rs` (or utility module)

```rust
pub fn truncate_tool_output(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }
    let mut end = max_bytes;
    while !content.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...[truncated]", &content[..end])
}
```

Replace all 4 inline copies of the 4096-byte truncation logic.

### Acceptance Criteria

```bash
# Only one truncation constant outside tests
rg '4096' crates/ --type rust | grep -v test | grep -v target | wc -l  # <= 2

# extract_clean_text is removed or deprecated
rg 'extract_clean_text' crates/ --type rust | grep -v test | wc -l  # 0

# parse_stream_line is used from translate/mod.rs
rg 'parse_stream_line' crates/roko-agent/src/translate/ --type rust | wc -l  # >= 1
```

---

## Phase 6: Direct Env Key Elimination (0.5 days)

**Goal:** No code outside provider adapters reads API keys from environment.

### 6.1 Neuro Episode Completion

**File:** `crates/roko-neuro/src/episode_completion.rs`

Replace `std::env::var("ANTHROPIC_API_KEY")` with dependency-injected agent:
```rust
// OLD:
let api_key = std::env::var("ANTHROPIC_API_KEY")?;
let client = reqwest::Client::new();
// ...manual HTTP request...

// NEW:
pub struct EpisodeCompleter {
    agent: Box<dyn Agent>,
}
impl EpisodeCompleter {
    pub fn new(config: &RokoConfig) -> Result<Self> {
        let agent = create_agent_for_model(config, "claude-haiku-4-5", &options)?;
        Ok(Self { agent })
    }
}
```

### 6.2 Web Search Tool

**File:** `crates/roko-std/src/tool/builtin/web_search.rs`

Replace `std::env::var("PERPLEXITY_API_KEY")` with injected configuration:
```rust
// The tool handler should receive API key through ToolContext or a
// pre-configured HTTP client, not by reading env vars directly.
```

### Acceptance Criteria

```bash
# No direct API_KEY env reads outside provider adapters and tests
rg 'std::env::var.*API_KEY' crates/ --type rust \
  | grep -v test | grep -v 'provider/\|adapter' | wc -l  # 0
```

---

## Phase 7: orchestrate.rs Decomposition (3-5 days)

**Goal:** Extract valuable patterns from the 21,577-line God object, delete dead code.

### 7.1 Identify Live Exports

First, identify what is actually imported from `orchestrate.rs`:
```bash
rg 'orchestrate::' crates/ --type rust | grep -v 'orchestrate.rs' | grep -v test
```

Known live exports:
- `save_snapshot_atomic()` -- used in tests
- Some type definitions referenced via `lib.rs`

### 7.2 Extract Gate Failure Replan

**Source:** `orchestrate.rs:build_gate_failure_plan_revision()`
**Target:** `crates/roko-orchestrator/src/replan.rs`

The gate failure replan logic generates a revised plan when gate checks fail.
Extract to a standalone function in `roko-orchestrator`.

### 7.3 Extract Context Bidding

**Source:** `orchestrate.rs` context bidders (`AttentionBidder` variants)
**Target:** `crates/roko-compose/src/context_bidding.rs`

The VCG auction and context bidding system for prompt assembly should live
in `roko-compose` alongside the `SystemPromptBuilder`.

### 7.4 Extract Budget/Anomaly/Conductor

**Source:** `orchestrate.rs` budget guardrails, anomaly detection, conductor
**Target:** `crates/roko-runtime/src/` (already partially there)

### 7.5 Delete Dead PlanRunner

After extracting all valuable patterns, delete:
- `PlanRunner` struct and all methods
- `dispatch_agent_with()` and callers
- `run_task_plans()` and callers
- All private helpers that only serve PlanRunner

### Acceptance Criteria

```bash
# orchestrate.rs is significantly smaller
wc -l crates/roko-cli/src/orchestrate.rs  # should be < 5000

# Extracted modules exist and compile
cargo test -p roko-orchestrator
cargo test -p roko-compose
cargo test -p roko-runtime
```

---

## Phase 8: Novel Dispatch Strategies (2-3 days)

**Goal:** Implement advanced dispatch patterns beyond basic model selection.

### 8.1 Speculative Decoding Pattern

**File:** `crates/roko-agent/src/model_call_service.rs`

For interactive paths, dispatch to a fast model (Haiku/Flash) while simultaneously
starting a slower premium model. If the fast model's output passes a quality check
(e.g., confidence score, length threshold), cancel the slow model and return immediately.

```rust
pub async fn call_speculative(&self, req: ModelCallRequest) -> Result<ModelCallResponse> {
    let fast_model = self.fastest_model_for_task(&req);
    let premium_model = self.resolve_model(&req);

    let (fast_result, premium_handle) = tokio::join!(
        self.call_model(&req, &fast_model),
        tokio::spawn(self.call_model(&req, &premium_model)),
    );

    if fast_result.quality_score > 0.8 {
        premium_handle.abort();
        return fast_result;
    }

    premium_handle.await?
}
```

### 8.2 Ensemble Inference

For high-stakes decisions (architecture, security review), dispatch to multiple
models and aggregate:

```rust
pub async fn call_ensemble(&self, req: ModelCallRequest, models: &[String])
    -> Result<ModelCallResponse>
{
    let results = futures::future::join_all(
        models.iter().map(|m| self.call_model(&req, m))
    ).await;

    // Majority vote for structured outputs
    // Best-of-N for text generation
    // Union for tool calls
    self.aggregate_results(&results)
}
```

### 8.3 Cost-Optimized Batch Routing

For plan execution with many tasks, pre-compute an optimal routing plan:

```rust
pub fn plan_batch_routing(&self, tasks: &[TaskSpec], budget: f64)
    -> Vec<(TaskId, String)>
{
    // Sort tasks by complexity
    // Assign cheapest viable model to each
    // Verify total cost <= budget
    // Escalate models for high-complexity tasks within budget
}
```

### Acceptance Criteria

```bash
# Speculative decoding exists
rg 'call_speculative\|speculative' crates/roko-agent/ --type rust | wc -l  # >= 1

# Ensemble inference exists
rg 'call_ensemble\|ensemble' crates/roko-agent/ --type rust | wc -l  # >= 1
```

---

## Phase 9: Observability & Dashboard (1-2 days)

**Goal:** Inference dispatch decisions visible in TUI and API.

### 9.1 RouterDecision Events

**File:** `crates/roko-agent/src/model_call_service.rs`

Emit structured routing decision events:
```rust
self.emit(RuntimeEvent::RouterDecision {
    policy: "auto-cost-optimized",
    candidates: vec![
        Candidate { model: "haiku", score: 0.94, reason: "trivial task" },
        Candidate { model: "sonnet", score: 0.62, reason: "overkill" },
    ],
    chosen: "haiku",
    estimated_cost: 0.001,
});
```

### 9.2 Cost Summary Stream

Aggregate per-session cost data and expose via SSE:
```rust
GET /api/stream/costs -> SSE {
    turn_cost: 0.003,
    session_cost: 0.147,
    session_budget: 10.00,
    tokens: { input: 12340, output: 5670, cached: 8900, thought: 1200 },
    cost_per_turn: [0.001, 0.003, 0.002, 0.005, ...],  // sparkline data
}
```

### 9.3 TUI Integration

Wire RouterDecision and CostSummary events to existing TUI infrastructure:
- RouterTrace card in the dashboard tab
- CostPanel in the right rail
- Tier confidence bars in the providers tab

---

## Summary: Phase Dependencies

```
Phase 0 (CascadeRouter wiring)
    |
    v
Phase 1 (Episode logging)  <-- can run in parallel with Phase 0
    |
    v
Phase 2 (ACP migration)  <-- independent
    |
    v
Phase 3 (Health + circuit breaker)  <-- needs Phase 0
    |
    v
Phase 4 (Budget enforcement)  <-- needs Phase 0
    |
Phase 5 (Parser consolidation)  <-- independent
Phase 6 (Env key elimination)  <-- independent
    |
    v
Phase 7 (orchestrate.rs decomposition)  <-- after Phases 0-2 extract patterns
    |
    v
Phase 8 (Novel strategies)  <-- needs Phases 0, 3, 4
    |
    v
Phase 9 (Observability)  <-- needs all above
```

**Estimated total: 12-20 days for full completion.**

Phases 0-2 are the critical path for self-hosting quality.
Phases 3-6 improve reliability and correctness.
Phases 7-9 are polish and advanced features.

---

## Sources

| File | Purpose |
|---|---|
| `crates/roko-agent/src/model_call_service.rs` | ModelCallService (2,143 LOC) |
| `crates/roko-agent/src/provider/mod.rs` | Provider adapter registry (1,148 LOC) |
| `crates/roko-agent/src/provider/health.rs` | Proposed: ProviderHealthTracker |
| `crates/roko-cli/src/model_selection.rs` | Model selection precedence (581 LOC) |
| `crates/roko-cli/src/dispatch_v2.rs` | v2 dispatch entry point (946 LOC) |
| `crates/roko-cli/src/orchestrate.rs` | God object to decompose (21,577 LOC) |
| `crates/roko-cli/src/run.rs` | `roko run` universal loop (1,555 LOC) |
| `crates/roko-cli/src/chat_inline.rs` | Chat REPL |
| `crates/roko-cli/src/run_inline.rs` | One-shot inline dispatch |
| `crates/roko-acp/src/runner.rs` | ACP bare subprocess (969 LOC) |
| `crates/roko-acp/src/bridge_events.rs` | ACP event bridge (1,855 LOC) |
| `crates/roko-agent/src/provider/claude_cli/stream.rs` | Canonical stream parser |
| `crates/roko-agent/src/translate/mod.rs` | Response translation (747 LOC) |
| `crates/roko-cli/src/chat.rs` | extract_clean_text() to replace |
| `crates/roko-neuro/src/episode_completion.rs` | Direct env key to fix |
| `crates/roko-std/src/tool/builtin/web_search.rs` | Direct env key to fix |
| `crates/roko-learn/src/cascade_router.rs` | CascadeRouter LinUCB bandit |
| `crates/roko-learn/src/feedback_service.rs` | FeedbackService |
| `crates/roko-runtime/src/jsonl_logger.rs` | Episode JSONL writer |
| `crates/roko-core/src/config/schema.rs` | Config schema (budget section) |
| `crates/roko-orchestrator/src/replan.rs` | Proposed: gate failure replan |
| `crates/roko-compose/src/context_bidding.rs` | Proposed: context bidding |
