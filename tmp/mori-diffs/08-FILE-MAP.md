# File Map: Every File Created or Modified

## Summary

| Category | New Files | Modified Files | Deleted Files |
|----------|-----------|---------------|---------------|
| `dispatch/` (new module) | 5 | 0 | 0 |
| `runner/` (rewritten) | 0 | 8 | 0 |
| CLI integration | 0 | 2 | 0 |
| External crates | 0 | 0 | 0 |
| **Total** | **5** | **10** | **0** |

## New Files: `crates/roko-cli/src/dispatch/`

### `dispatch/mod.rs` (~120 lines)

**Purpose**: Central agent dispatch - replaces hardcoded Claude CLI spawning.

```rust
pub struct AgentDispatcher {
    router: CascadeRouter,
    prompt_assembler: PromptAssembler,
    warm_pool: WarmPool,
    provider_semaphores: ProviderSemaphores,
}

impl AgentDispatcher {
    pub fn new(router: CascadeRouter, config: &RunConfig) -> Self;
    pub async fn dispatch(&mut self, task: &TaskDef, plan_id: &str, ctx: &DispatchContext) -> Result<AgentOutcome, DispatchError>;
    pub fn pre_validate(&self, task: &TaskDef, config: &RunConfig) -> Result<(), DispatchError>;
}
```

**Depends on**: `roko-agent/provider`, `roko-learn/cascade_router`, `dispatch/prompt_builder`, `dispatch/warm_pool`

**Audit paths**: 1 (dispatch), 5 (prompts)

---

### `dispatch/model_routing.rs` (~80 lines)

**Purpose**: CascadeRouter integration - adaptive model selection at dispatch time.

```rust
pub struct RoutingContext {
    pub task_domain: String,
    pub task_complexity: f64,
    pub budget_remaining: f64,
    pub force_backend: Option<String>,
    pub history: Vec<RoutingObservation>,
}

impl RoutingContext {
    pub fn from_task(task: &TaskDef, config: &RunConfig, state: &RunState) -> Self;
}

pub fn select_model(router: &CascadeRouter, ctx: &RoutingContext) -> String;
pub fn record_observation(router: &mut CascadeRouter, task_id: &str, model: &str, outcome: &AgentOutcome);
```

**Depends on**: `roko-learn/cascade_router`

**Audit paths**: 1 (dispatch), 4 (learning)

---

### `dispatch/prompt_builder.rs` (~150 lines)

**Purpose**: 9-layer system prompt assembly using `RoleSystemPromptSpec`.

```rust
pub struct PromptAssembler {
    playbook_store: Option<PlaybookStore>,
    neuro_store: Option<NeuroStore>,
}

impl PromptAssembler {
    pub fn new(playbook_store: Option<PlaybookStore>, neuro_store: Option<NeuroStore>) -> Self;
    pub fn assemble(&self, task: &TaskDef, plan_id: &str, ctx: &PromptContext) -> Result<AssembledPrompt>;
}

pub struct PromptContext {
    pub role: String,
    pub gate_feedback: Option<GateFeedback>,
    pub files_in_scope: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub verify_commands: Vec<String>,
}

pub struct GateFeedback {
    pub compile_errors: Vec<CompileError>,
    pub test_failures: Vec<TestFailure>,
    pub clippy_warnings: Vec<ClippyWarning>,
    pub raw_output: String,
}

pub struct AssembledPrompt {
    pub system_prompt: String,
    pub user_prompt: String,
    pub tool_allowlist: Option<Vec<String>>,
}
```

**Depends on**: `roko-compose/system_prompt_builder`, `roko-learn/playbook`, `roko-neuro`

**Audit paths**: 5 (prompts)

---

### `dispatch/outcome.rs` (~60 lines)

**Purpose**: Result and error types for agent dispatch.

```rust
pub struct AgentOutcome {
    pub task_id: String,
    pub model: String,
    pub provider: String,
    pub output: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub is_error: bool,
}

pub enum DispatchError {
    BudgetExceeded { spent: f64, limit: f64 },
    NoModelAvailable { reason: String },
    SpawnFailed { inner: anyhow::Error },
    PreValidationFailed { reason: String },
    Timeout { after_secs: u64 },
    Cancelled,
}
```

**Audit paths**: 1 (dispatch), 2 (execution)

---

### `dispatch/warm_pool.rs` (~100 lines)

**Purpose**: Pre-spawned agent pool for fast transitions (for example, gate to reviewer).

```rust
pub struct WarmPool {
    pool: HashMap<String, Vec<WarmAgent>>,
    max_per_role: usize,
}

struct WarmAgent {
    agent: Box<dyn Agent>,
    model: String,
    spawned_at: Instant,
    ttl: Duration,
}

impl WarmPool {
    pub fn new(max_per_role: usize) -> Self;
    pub fn pre_spawn(&mut self, role: &str, model: &str, config: &RunConfig) -> Result<()>;
    pub fn take(&mut self, role: &str) -> Option<Box<dyn Agent>>;
    pub fn take_or_spawn(&mut self, role: &str, model: &str, config: &RunConfig) -> Result<Box<dyn Agent>>;
    pub fn evict_expired(&mut self);
}
```

**Audit paths**: 1 (dispatch)

---

## Modified Files: `crates/roko-cli/src/runner/`

### `runner/mod.rs` (33 lines -> ~40 lines)

**Changes**: Add re-export for new `dispatch` module integration types.

```diff
+ pub use event_loop::DispatchConfig;
```

---

### `runner/event_loop.rs` (736 lines -> ~600 lines)

**Major rewrite**. The core `tokio::select!` loop structure stays, but dispatch logic moves to `dispatch/`.

**Changes by section**:

| Section | Lines (v2) | Change | Lines (v3) |
|---------|------------|--------|------------|
| `RunContext` struct | 68-79 | Add `dispatcher: &mut AgentDispatcher` | +3 |
| `run()` entry point | 84-310 | Load CascadeRouter, gate thresholds on startup | +20 |
| `SpawnAgent` handler | 402-519 | **Replace** with `ctx.dispatcher.dispatch()` call | -80 |
| `RunGate` handler | 522-543 | Add timeout + semaphore | +15 |
| `RunVerify` handler | 545-548 | **Replace stub** with warm-pool reviewer dispatch | +30 |
| `MergeBranch` handler | 566-569 | Add sequential merge queue | +10 |
| `emit_episode` | 580-638 | Use `AgentOutcome` for all fields | ~same |
| `emit_efficiency_event` | 641-687 | Use `AgentOutcome` for all fields | ~same |

**Key deletions**:
- Sentinel resolution (`"next"`, `"fix"`, `"regen-verify"`) - replaced by real DAG in executor
- Manual prompt building - replaced by `dispatch/prompt_builder.rs`
- Direct `agent_stream::spawn_agent()` call - replaced by `dispatch/mod.rs`

---

### `runner/agent_events.rs` (100 lines -> ~160 lines)

**Changes**: Publish all event types to StateHub, not just MessageDelta.

```diff
  AgentEvent::SystemInit { session_id, model } => {
      state.agent_active = true;
      state.agent_model = model.clone();
+     tui.agent_spawned(&agent_id, &state.current_role, model);
  }

  AgentEvent::ToolCall { id, name } => {
      let marker = format!("\n[tool: {name}]\n");
      state.agent_output.push_str(&marker);
+     tui.tool_started(&agent_id, &id, &name);
  }

  AgentEvent::ToolOutput { id, output } => {
      state.agent_output.push_str(truncated);
+     tui.tool_completed(&agent_id, &id);
  }

  AgentEvent::TokenUsage { .. } => {
      state.tokens_in += input_tokens;
+     tui.token_usage(&agent_id, *input_tokens, *output_tokens);
+     tui.efficiency_event(&state.plan_id, &state.current_task, "tokens", (input + output) as f64);
  }

  AgentEvent::TurnCompleted { .. } => {
+     tui.cost_update(&agent_id, state.cost_usd);
  }
```

---

### `runner/gate_dispatch.rs` (80 lines -> ~130 lines)

**Changes**: Add timeout, cancellation token, and semaphore.

```diff
  pub fn spawn_gate(
      plan_id: String,
      task_id: String,
      rung: u32,
      workdir: PathBuf,
      gate_tx: mpsc::Sender<GateCompletion>,
+     cancel: CancellationToken,
+     gate_semaphore: Arc<Semaphore>,
+     timeout_secs: u64,
  ) {
      tokio::spawn(async move {
+         let _permit = gate_semaphore.acquire().await.map_err(|_| ...)?;
+         let result = tokio::time::timeout(
+             Duration::from_secs(timeout_secs),
+             run_rung(&signal, &ctx, rung, &inputs, &config)
+         ).await;
+
+         match result {
+             Ok(verdicts) => { /* existing logic */ }
+             Err(_timeout) => {
+                 gate_tx.send(GateCompletion { passed: false, output: "timeout".into(), .. }).await;
+             }
+         }
      });
  }
```

---

### `runner/state.rs` (207 lines -> ~280 lines)

**Changes**: Per-plan completed task tracking, routing decisions, retry metadata.

```diff
  pub struct RunState {
      // existing fields...
+
+     // Routing
+     /// Model selected for the current task (from CascadeRouter or override).
+     pub routing_model: String,
+     /// Provider used for the current task.
+     pub routing_provider: String,
+     /// Whether the current model was a manual override.
+     pub routing_forced: bool,
+
+     // Retry
+     /// Number of retries for the current task.
+     pub retry_count: u32,
+     /// Backoff delay for the next retry (seconds).
+     pub next_backoff_secs: u64,
+     /// Failure classification for the current task.
+     pub failure_class: Option<FailureClass>,
+
+     // Enhanced per-plan tracking
+     /// Routing decisions per plan (for persistence).
+     pub routing_decisions: HashMap<String, Vec<RoutingDecision>>,
  }

+ pub enum FailureClass {
+     Transient,  // network timeout, OOM, rate limit
+     Permanent,  // compile error not fixable, wrong model
+ }
+
+ pub struct RoutingDecision {
+     pub task_id: String,
+     pub model: String,
+     pub provider: String,
+     pub forced: bool,
+ }
```

---

### `runner/tui_bridge.rs` (114 lines -> ~200 lines)

**Changes**: Add methods for all new event types.

```diff
  impl TuiBridge {
      // existing methods...
+
+     pub fn tool_started(&self, agent_id: &str, tool_id: &str, name: &str) {
+         self.sender.publish(DashboardEvent::ToolStarted { .. });
+     }
+
+     pub fn tool_completed(&self, agent_id: &str, tool_id: &str) {
+         self.sender.publish(DashboardEvent::ToolCompleted { .. });
+     }
+
+     pub fn token_usage(&self, agent_id: &str, input: u64, output: u64) {
+         self.sender.publish(DashboardEvent::TokenUsage { .. });
+     }
+
+     pub fn cost_update(&self, agent_id: &str, total_cost: f64) {
+         self.sender.publish(DashboardEvent::CostUpdate { .. });
+     }
+
+     pub fn task_output(&self, plan_id: &str, task_id: &str, content: &str) {
+         self.sender.publish(DashboardEvent::TaskOutputAppended { .. });
+     }
+
+     pub fn retry_started(&self, plan_id: &str, task_id: &str, attempt: u32, backoff_secs: u64) {
+         self.sender.publish(DashboardEvent::RetryStarted { .. });
+     }
  }
```

---

### `runner/persist.rs` (164 lines -> ~250 lines)

**Changes**: 5-file snapshot, version field, incremental save.

```diff
  pub struct PersistPaths {
      pub executor_json: PathBuf,
      pub episodes_jsonl: PathBuf,
      pub efficiency_jsonl: PathBuf,
      pub agent_pids_json: PathBuf,
      pub events_json: PathBuf,
+     pub cascade_router_json: PathBuf,
+     pub gate_thresholds_json: PathBuf,
+     pub daimon_json: PathBuf,
  }

+ pub struct FullSnapshot {
+     pub version: u32,
+     pub executor: ExecutorSnapshot,
+     pub cascade_router: Option<serde_json::Value>,
+     pub gate_thresholds: Option<serde_json::Value>,
+     pub daimon: Option<serde_json::Value>,
+     pub completed_tasks: HashMap<String, Vec<String>>,
+     pub routing_decisions: HashMap<String, Vec<RoutingDecision>>,
+ }

+ pub fn save_full_snapshot(paths: &PersistPaths, snapshot: &FullSnapshot) -> Result<()>;
+ pub fn load_full_snapshot(paths: &PersistPaths) -> Result<Option<FullSnapshot>>;
```

---

### `runner/types.rs` (240 lines -> ~200 lines)

**Changes**: Remove Claude-specific stream types. AgentEvent becomes provider-agnostic.

**Deletions** (~100 lines):
- `ClaudeStreamEvent` enum
- `ClaudeSystemEvent` struct
- `ClaudeAssistantEvent` struct
- `ClaudeMessage` struct
- `ClaudeContentBlock` enum
- `ClaudeToolEvent` struct
- `ClaudeResultEvent` struct
- `ClaudeUsage` struct

These move to `roko-agent`'s Claude CLI backend where they belong.

**Additions** (~60 lines):
```rust
/// Provider-agnostic agent event.
pub enum AgentEvent {
    Initialized { session_id: String, model: String, provider: String },
    MessageDelta { text: String },
    ToolCall { id: String, name: String },
    ToolOutput { id: String, output: String },
    TokenUsage { input_tokens: u64, output_tokens: u64, cache_read_tokens: u64, cache_write_tokens: u64 },
    TurnCompleted { session_id: Option<String>, total_cost_usd: Option<f64>, num_turns: Option<u32>, is_error: bool },
    Error { message: String },
    Exited { exit_code: Option<i32> },
}
```

**Changes to `RunConfig`**:
```diff
  pub struct RunConfig {
      // existing...
-     pub claude_program: PathBuf,
+     pub default_provider: String,
+     pub gate_timeout_secs: u64,
+     pub gate_concurrency: usize,
+     pub merge_queue_enabled: bool,
+     pub plan_timeout_secs: u64,
  }
```

---

## Modified Files: CLI Integration

### `crates/roko-cli/src/main.rs` (~2 line change)

```diff
+ mod dispatch;
```

---

### `crates/roko-cli/src/run.rs` (~20 line change)

Wire `dispatch/` into the `plan run` command path:

```diff
+ use crate::dispatch::AgentDispatcher;
+ use roko_learn::cascade_router::CascadeRouter;

  // In plan_run():
+ let router = CascadeRouter::load_or_default(&paths.cascade_router_json);
+ let dispatcher = AgentDispatcher::new(router, &config);
  let report = runner::run(plans, &config, &state_hub, cancel).await?;
```

---

## Files NOT Modified (Reused As-Is)

| File | Why Unchanged |
|------|--------------|
| `crates/roko-agent/src/provider/mod.rs` | Already has `create_agent_for_model()` + `AgentOptions` |
| `crates/roko-agent/src/process.rs` | Already has `kill_tree()` + PID registration |
| `crates/roko-cli/src/agent_spawn.rs` | Already has `SpawnAgentSpec` + `spawn_agent_with_layer()` |
| `crates/roko-compose/src/system_prompt_builder.rs` | Already has `RoleSystemPromptSpec` 9-layer builder |
| `crates/roko-learn/src/cascade_router.rs` | Already has `CascadeRouter::select_model()` |
| `crates/roko-learn/src/episode_logger.rs` | Already has `Episode` + logging |
| `crates/roko-gate/src/lib.rs` | Already has `run_rung()` pipeline |
| `crates/roko-core/src/dashboard_snapshot.rs` | Already has 25+ `DashboardEvent` variants |
| `crates/roko-core/src/state_hub.rs` | Already has pub/sub |
| `crates/roko-cli/src/task_parser.rs` | Already has `TaskDef` with DAG + deps |
| `crates/roko-cli/src/runner/plan_loader.rs` | TOML parsing unchanged |

## Line Count Summary

| File | v2 Lines | v3 Lines | Delta |
|------|----------|----------|-------|
| **New files** | | | |
| `dispatch/mod.rs` | 0 | ~120 | +120 |
| `dispatch/model_routing.rs` | 0 | ~80 | +80 |
| `dispatch/prompt_builder.rs` | 0 | ~150 | +150 |
| `dispatch/outcome.rs` | 0 | ~60 | +60 |
| `dispatch/warm_pool.rs` | 0 | ~100 | +100 |
| **Modified files** | | | |
| `runner/mod.rs` | 33 | ~40 | +7 |
| `runner/event_loop.rs` | 736 | ~600 | -136 |
| `runner/agent_events.rs` | 100 | ~160 | +60 |
| `runner/gate_dispatch.rs` | 80 | ~130 | +50 |
| `runner/state.rs` | 207 | ~280 | +73 |
| `runner/tui_bridge.rs` | 114 | ~200 | +86 |
| `runner/persist.rs` | 164 | ~250 | +86 |
| `runner/types.rs` | 240 | ~200 | -40 |
| `main.rs` | n/a | n/a | +2 |
| `run.rs` | n/a | n/a | +20 |
| **Total** | 1674 | ~2370 | **+696** |

Net: +510 new lines in `dispatch/`, +186 net in `runner/` modifications. The v3 runner is ~2370 lines total - significantly smaller than the 21K line `orchestrate.rs` it replaces for runtime execution.

## Implementation Packet

This file is the concrete file ownership map. When implementing, update this file whenever a new module is added or a responsibility moves.

### Required Additions Beyond The Original Runner v3 Map

Source-corrected status as of 2026-04-27:

- [x] `crates/roko-agent/src/runtime_events.rs` for normalized agent events.
- [x] `crates/roko-cli/src/runtime_feedback/mod.rs` for learning/knowledge/conductor/dream sinks.
- [x] `crates/roko-cli/src/runtime_feedback/episodes.rs`.
- [x] `crates/roko-cli/src/runtime_feedback/routing.rs`.
- [x] `crates/roko-cli/src/runtime_feedback/knowledge.rs`.
- [x] `crates/roko-cli/src/runtime_feedback/conductor.rs`.
- [x] `crates/roko-cli/src/runtime_feedback/dreams.rs`.
- [x] `crates/roko-cli/src/projection/mod.rs`.
- [x] `crates/roko-cli/src/projection/dashboard.rs`.
- [x] `crates/roko-cli/src/projection/cli_progress.rs`.
- [x] `crates/roko-cli/src/runner/task_dag.rs` if DAG resolution does not stay in orchestrator.
- [x] `crates/roko-cli/src/runner/merge.rs` for merge queue dispatch.

### Ownership Checklist

- [ ] Every provider-specific parser belongs in `roko-agent`, not `runner/`.
- [ ] Every prompt/context decision belongs in `roko-compose` or `dispatch/prompt_builder.rs`.
- [ ] Every state write belongs in `runner/persist.rs` or a persistence helper owned by the relevant crate.
- [ ] Every dashboard mutation starts from a normalized runtime event.
- [ ] Every learning update goes through `runtime_feedback/`.

### File Review Checklist

- [ ] `runner/event_loop.rs` remains under 800 lines after migration.
- [ ] No new module exceeds 500 lines without a clear reason.
- [ ] `orchestrate.rs` line count decreases or stays frozen after each phase.
- [ ] New files include module-level comments explaining ownership.
- [ ] Tests are listed next to each file they validate.

## Worker 9 Actual File Map Delta (2026-04-26)

Files and APIs that exist now, even when they differ from this target map:

- [x] `crates/roko-cli/src/dispatch_v2.rs` exists and is the current dispatch abstraction; the planned `crates/roko-cli/src/dispatch/` directory does not exist.
- [x] `crates/roko-cli/src/runner/event_loop.rs` owns active runner orchestration, direct agent spawning, gate completion handling, retry decisions, snapshot saves, and merge auto-advance.
- [x] `crates/roko-cli/src/runner/agent_stream.rs` owns live CLI process launch, Claude-shaped stream parsing, fallback JSON parsing, stderr event creation, and prompt construction calls.
- [x] `crates/roko-cli/src/runner/gate_dispatch.rs` owns active gate execution, rung timeout, semaphore, `GatePayload`, and `task.verify` shell gates.
- [x] `crates/roko-cli/src/runner/persist.rs` owns active runner persistence paths and atomic writes.
- [x] `crates/roko-compose/src/strategy.rs` and `crates/roko-compose/src/cost_attribution.rs` exist for composition strategy selection and per-section cost attribution.
- [x] `crates/roko-dreams/src/routing_advice.rs`, `crates/roko-dreams/src/cycle.rs`, and `crates/roko-dreams/src/runner.rs` exist for dream consolidation and routing advice.
- [x] `crates/roko-neuro/src/admission.rs`, `crates/roko-neuro/src/knowledge_store.rs`, and `crates/roko-neuro/src/lifecycle.rs` exist for admission, reinforcement, and lifecycle observations.

Files from the target map that were missing in the 2026-04-26 evidence pass but now exist:

- [x] `crates/roko-agent/src/runtime_events.rs`
- [x] `crates/roko-cli/src/dispatch/mod.rs`
- [x] `crates/roko-cli/src/dispatch/model_routing.rs`
- [x] `crates/roko-cli/src/dispatch/prompt_builder.rs`
- [x] `crates/roko-cli/src/dispatch/outcome.rs`
- [x] `crates/roko-cli/src/dispatch/warm_pool.rs`
- [x] `crates/roko-cli/src/runtime_feedback/mod.rs`
- [x] `crates/roko-cli/src/runtime_feedback/episodes.rs`
- [x] `crates/roko-cli/src/runtime_feedback/routing.rs`
- [x] `crates/roko-cli/src/runtime_feedback/knowledge.rs`
- [x] `crates/roko-cli/src/runtime_feedback/conductor.rs`
- [x] `crates/roko-cli/src/runtime_feedback/dreams.rs`
- [x] `crates/roko-cli/src/projection/mod.rs`
- [x] `crates/roko-cli/src/runner/projection.rs`
- [x] `crates/roko-cli/src/runner/task_dag.rs`
- [x] `crates/roko-cli/src/runner/merge.rs`

Still missing or unresolved from the target map:

- [ ] A generated file-ownership proof proving these modules are source-wired and active-path exercised.
- [ ] A generated dead-surface inventory for `dispatch_v2.rs`, `dispatch_direct.rs`, and legacy helper call sites.

## 9. 2026-04-27 Deepening Pass - Current Ownership Map

Self-grade for this pass:

- Initial rating: 9.90 / 10.
- Reasoning: this file now corrects the stale missing-file inventory, names actual module sizes, and converts the file map into an ownership/proof checklist. The score is not higher because the map still needs generated evidence proving each module is active-path exercised, not merely present.

### 9.1 Current Source Inventory

Verified current files and approximate line counts:

- [x] `crates/roko-agent/src/runtime_events.rs`: provider-neutral runtime event vocabulary, 219 lines.
- [x] `crates/roko-cli/src/dispatch/mod.rs`: dispatch facade, 405 lines.
- [x] `crates/roko-cli/src/dispatch/model_routing.rs`: model choice precedence and router facade, 273 lines.
- [x] `crates/roko-cli/src/dispatch/outcome.rs`: normalized dispatch outcome and errors, 173 lines.
- [x] `crates/roko-cli/src/dispatch/prompt_builder.rs`: active runner prompt assembler, 993 lines.
- [x] `crates/roko-cli/src/dispatch/warm_pool.rs`: typed warm-pool container, 250 lines.
- [x] `crates/roko-cli/src/runtime_feedback/mod.rs`: feedback facade and event vocabulary, 400 lines.
- [x] `crates/roko-cli/src/runtime_feedback/conductor.rs`: conductor observation sink, 204 lines.
- [x] `crates/roko-cli/src/runtime_feedback/dreams.rs`: dream trigger sink, 250 lines.
- [x] `crates/roko-cli/src/runtime_feedback/episodes.rs`: episode sink, 149 lines.
- [x] `crates/roko-cli/src/runtime_feedback/knowledge.rs`: knowledge candidate sink, 250 lines.
- [x] `crates/roko-cli/src/runtime_feedback/routing.rs`: router observation sink, 153 lines.
- [x] `crates/roko-cli/src/runner/merge.rs`: PlanMerger/GitMergeBackend/regression gate, 776 lines.
- [x] `crates/roko-cli/src/projection/dashboard.rs`: dashboard projection adapter, 288 lines.
- [x] `crates/roko-cli/src/projection/cli_progress.rs`: CLI progress projection adapter, 341 lines.
- [x] `crates/roko-cli/src/runner/projection.rs`: runner projection broadcaster/reducer, 554 lines.
- [x] `crates/roko-cli/src/runner/task_dag.rs`: task DAG utilities, 554 lines.
- [x] `crates/roko-cli/src/lib.rs` exports `dispatch`, `projection`, and `runtime_feedback`.
- [x] `crates/roko-cli/src/runner/mod.rs` exports `merge`, `projection`, and `task_dag`.

### 9.2 Current Ownership Rules

- [ ] `roko-agent` owns provider wire parsing and normalized agent runtime events.
- [ ] `dispatch/` owns model selection, prompt assembly, provider runtime resolution, and normalized dispatch outcomes.
- [ ] `runner/` owns state-machine execution, event persistence, merge dispatch, resume, projection emission, and task scheduling.
- [ ] `runtime_feedback/` owns non-blocking fan-out to episodes, routing, knowledge, conductor, and dreams.
- [ ] `projection/` and `runner/projection.rs` own read-model updates and query/stream surfaces.
- [ ] `roko-compose` owns advanced prompt composition, VCG, cost attribution, templates, and section-effect support.
- [ ] `roko-learn` owns routing learning, section effectiveness, efficiency, and model/provider outcome summaries.
- [ ] `roko-neuro` owns durable knowledge admission, reinforcement, lifecycle, and heuristic falsifiers.
- [ ] `roko-dreams` owns dream consolidation, routing advice, and dream reports.
- [ ] `roko-serve` owns HTTP adapter wiring and long-running serve/daemon loops.

### 9.3 Remaining File-Map Risks

- [ ] `dispatch_v2.rs` still exists beside `dispatch/`; classify every remaining caller as legacy, adapter, or active.
- [ ] `dispatch_direct.rs` still exists; classify whether it is test utility, legacy provider path, or active bypass.
- [ ] `dispatch/prompt_builder.rs` is 993 lines and may need splitting into section sources, diagnostics, budget enforcement, and renderers.
- [ ] `runner/merge.rs` is 776 lines and may need splitting if merge backend, regression gate, and queue orchestration continue growing.
- [ ] `runner/projection.rs` and top-level `projection/mod.rs` overlap; document exact ownership or consolidate.
- [ ] `runtime_feedback/knowledge.rs` writes candidates but does not prove durable neuro lifecycle ingestion.
- [ ] `runtime_feedback/dreams.rs` writes triggers but does not prove consolidation.
- [ ] `dispatch/warm_pool.rs` is a container, not real provider pre-spawn.
- [ ] `dispatch/model_routing.rs` currently has a cascade branch placeholder/default behavior that must be fixed before routing parity.

### 9.4 Implementation Batches

#### FM-01: Generate Ownership Inventory

- [ ] Write or run a script that lists every Rust file under `crates/`.
- [ ] Classify each file by owner domain: `agent`, `dispatch`, `runner`, `feedback`, `projection`, `serve`, `compose`, `learn`, `neuro`, `dreams`, `gate`, `core`, `legacy`, or `test`.
- [ ] Record line counts and public symbols for every file.
- [ ] Record whether each file has active-path call sites.
- [ ] Store output in `tmp/mori-diffs/generated/file-ownership-inventory.json`.

#### FM-02: Classify Legacy/Transition Files

- [ ] Search for `dispatch_v2`.
- [ ] Search for `dispatch_direct`.
- [ ] Search for `orchestrate`.
- [ ] Search for direct `agent_stream::spawn_agent`.
- [ ] Search for direct provider protocol parsing outside `roko-agent`.
- [ ] Classify each hit as `active_adapter`, `legacy_donor`, `test_only`, `production_gap`, or `remove`.
- [ ] Store output in `tmp/mori-diffs/generated/legacy-transition-file-map.json`.

#### FM-03: Active-Path Proof Per Module

- [ ] Prove `dispatch/mod.rs` is exercised by active `plan run`.
- [ ] Prove `dispatch/model_routing.rs` returns correct `ModelChoiceSource` for override, task hint, router, and default.
- [ ] Prove `dispatch/prompt_builder.rs` emits prompt diagnostics.
- [ ] Prove `runtime_feedback/*` sinks receive events from active runner.
- [ ] Prove `runner/merge.rs` handles merge success and merge failure.
- [ ] Prove `runner/projection.rs` emits queryable projection updates.
- [ ] Store output in `tmp/mori-diffs/generated/active-module-proof-report.json`.

#### FM-04: Split Or Cap Oversized Modules

- [ ] Decide whether `dispatch/prompt_builder.rs` should split now or after prompt/VCG convergence.
- [ ] If splitting, separate section sources from renderer and diagnostics.
- [ ] Decide whether `runner/merge.rs` should split merge backend, regression gate, and coordinator.
- [ ] Add module-level ownership comments to every split file.
- [ ] Update this file with the new map.

### 9.5 Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/file-map-proof-report.json`:

```json
{
  "schema": "mori-diffs.file-map-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "inventories": {
    "file_ownership_inventory": false,
    "legacy_transition_file_map": false,
    "active_module_proof_report": false
  },
  "active_modules": {
    "dispatch": false,
    "model_routing": false,
    "prompt_builder": false,
    "runtime_feedback": false,
    "merge": false,
    "projection": false
  },
  "legacy_surfaces": [],
  "oversized_modules": [],
  "remaining_gaps": []
}
```

### 9.6 No-Context Handoff Checklist

- [ ] Run `rg --files crates | sort`.
- [ ] Run `wc -l` for all files listed in section 9.1.
- [ ] Run `rg -n "dispatch_v2|dispatch_direct|orchestrate|agent_stream::spawn_agent|ClaudeStreamEvent|PlanRunner::from_plans_dir|PromptAssembler::minimal" crates`.
- [ ] Implement FM-01 before changing file ownership.
- [ ] Implement FM-02 before deleting or renaming legacy files.
- [ ] Implement FM-03 before claiming module completion.
- [ ] Implement FM-04 only after proof shows a module is oversized and still active.
- [ ] Generate `tmp/mori-diffs/generated/file-map-proof-report.json`.
- [ ] Update [README.md](README.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), and [31-REPOSITORY-WIDE-ARCHITECTURE-SCAN.md](31-REPOSITORY-WIDE-ARCHITECTURE-SCAN.md).

### 9.7 Archive Gate

- [ ] All stale missing-file rows are corrected.
- [ ] File ownership inventory exists.
- [ ] Legacy/transition file map exists.
- [ ] Active-module proof report exists.
- [ ] Oversized active modules have split plans or explicit keep decisions.
