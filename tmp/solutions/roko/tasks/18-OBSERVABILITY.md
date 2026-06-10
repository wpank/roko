# 18 -- Observability, Monitoring & Dashboard Tasks

**Source:** `tmp/solutions/roko/impl/18-OBSERVABILITY.md`
**Date:** 2026-04-29
**Crates:** `roko-runtime`, `roko-agent`, `roko-learn`, `roko-gate`, `roko-serve`, `roko-cli`, `roko-core`

---

## Overview

Roko has foundational observability infrastructure -- `RuntimeEvent` (12 variants), `EventBus<E>`, `JsonlLogger`, `GatewayEventWriter`, SSE streaming, Prometheus text endpoint, and TUI widgets -- but critical gaps remain. Model calls lack OTel `gen_ai.*` spans. The `JsonlLogger` flushes per-event (30-50ms overhead per run). No structured events exist for routing decisions, cost ticks, or per-gate progress. Anomaly detection is session-local (`AnomalyDetector` in roko-learn) but not wired into the obs event pipeline. The TUI has no live RouterTrace, CostPanel, or per-gate progress strips.

This task file covers: tracing instrumentation, OTel integration, structured event emission, TUI dashboard widgets, HTTP observability endpoints, anomaly detection wiring, and end-to-end integration testing.

---

## Anti-Patterns to Remove

### AP-1: Per-event flush in JsonlLogger
**Location:** `crates/roko-runtime/src/jsonl_logger.rs:62-86`
**Problem:** `write_event()` calls `w.flush()` after every single event write. For a run with 20-30 events, this adds 60-150ms of synchronous disk I/O.
**Fix:** Interval-based flush (every N events) with explicit `flush_all()` at run completion.

### AP-2: Eager event serialization in EventBus
**Location:** `crates/roko-runtime/src/event_bus.rs` -- `Envelope<E>` wraps events but consumers that only need the typed payload still incur JSON serialization cost when the JSONL sink serializes eagerly.
**Problem:** 20-40ms overhead even when no serialization consumer is listening.
**Fix:** Wrap payload in a `LazyJson<E>` that defers `serde_json::to_string()` until first `.to_json()` call.

### AP-3: No `tracing::instrument` spans on critical paths
**Location:** `crates/roko-agent/src/model_call_service.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/dispatch_v2.rs`
**Problem:** Zero `#[tracing::instrument]` annotations on model call, config load, prompt assembly, or gate pipeline functions. No phase-level latency visibility.
**Fix:** Add spans at each bottleneck boundary from the performance analysis (B02-B13).

### AP-4: Gate results emitted only after pipeline completion
**Location:** `crates/roko-gate/src/gate_service.rs:234-366` -- `run_selected_gate_pipeline()` returns all verdicts at the end.
**Problem:** TUI and SSE consumers see gate results only after the entire pipeline finishes. No live per-gate progress.
**Fix:** Emit `GateStarted`/`GatePassed`/`GateFailed`/`GateSkipped` events per-gate during pipeline execution.

### AP-5: Prometheus endpoint lacks model-level counters
**Location:** `crates/roko-serve/src/routes/status/metrics.rs:140-248`
**Problem:** Only aggregate counters (plans, tasks, gates, episodes). No per-model call counts, token totals, cost totals, or latency percentiles.
**Fix:** Source from `GatewayProjection` and `CascadeRouter` to emit per-model/per-gate Prometheus metrics.

### AP-6: Anomaly detection not wired to obs pipeline
**Location:** `crates/roko-learn/src/anomaly.rs` -- `AnomalyDetector` exists with cost spike, prompt loop, quality degradation detection, but it is session-local and does not emit events to the `EventBus`.
**Problem:** Anomalies detected in `AnomalyDetector` are not surfaced to TUI, SSE, or CLI stderr.
**Fix:** Create `AnomalyMonitor` that subscribes to `ObservabilityEvent` bus and re-emits `Anomaly` events.

---

## Tasks

### Task 18.1: Add `tracing::instrument` spans to critical dispatch paths

**Depends on:** None
**Crates:** `roko-agent`, `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs` -- `ModelCallService` struct, `call()` method
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` -- `cmd_run()`, config load, dispatch
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs` -- V2 dispatch path
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs` -- model routing

**What to do:**
1. Add `#[tracing::instrument(skip_all, fields(phase = "..."))]` to each bottleneck boundary:
   - `config_load` (B02): `load_layered()` / `load_config()` in `run.rs`
   - `learning_init` (B03): `LearningRuntime::open_under()` call sites
   - `agent_construct` (B04/B15): `create_agent_for_model()` in model_call_service.rs
   - `prompt_assembly` (B12): `PromptAssemblyService::assemble()` call sites
   - `model_call`: `ModelCallService::call()` -- record model slug and caller_id
   - `gate_pipeline`: gate execution call sites in dispatch paths
   - `persistence`: substrate writes, episode logging
   - `cascade_routing`: model selection in `model_selection.rs`
2. Each span must capture at minimum: `phase` (string), and where applicable `model` and `caller_id`.
3. Do NOT add OTel dependencies yet -- use `tracing` spans only. OTel layer hooks in later.

**Acceptance criteria:**
- At least 8 `#[tracing::instrument]` annotations across the listed files.
- `cargo check -p roko-agent -p roko-cli` passes.
- `RUST_LOG=roko=trace cargo run -p roko-cli -- status` shows span entries in trace output.

---

### Task 18.2: Add `opentelemetry` workspace dependencies (feature-gated)

**Depends on:** None
**Crates:** workspace root, `roko-runtime`, `roko-agent`, `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` -- `[workspace.dependencies]`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/Cargo.toml`

**What to do:**
1. Add to `[workspace.dependencies]`:
   ```toml
   opentelemetry = { version = "0.28", default-features = false, features = ["trace"] }
   opentelemetry_sdk = { version = "0.28", features = ["rt-tokio"] }
   opentelemetry-otlp = { version = "0.28", features = ["tonic"] }
   tracing-opentelemetry = "0.28"
   ```
2. Add `otel` feature flag to `roko-cli/Cargo.toml`:
   ```toml
   [features]
   otel = ["dep:opentelemetry", "dep:opentelemetry_sdk", "dep:opentelemetry-otlp", "dep:tracing-opentelemetry"]
   ```
3. Add the dependencies as optional in `roko-cli/Cargo.toml` referencing workspace versions.
4. Verify both default build (no OTel) and `--features otel` compile.

**Acceptance criteria:**
- `cargo check -p roko-cli` passes (no OTel in binary).
- `cargo check -p roko-cli --features otel` passes.
- `cargo tree -p roko-cli --no-default-features | grep opentelemetry` returns 0 matches.

---

### Task 18.3: Implement `gen_ai.*` span builder for gateway layer

**Depends on:** 18.2
**Crates:** `roko-agent`, `roko-core`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/otel_spans.rs` (NEW)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lib.rs` -- add `pub mod otel_spans;`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/agent.rs` -- `ProviderKind` enum (line 35)

**What to do:**
1. Create `otel_spans.rs` with a `GenAiSpanBuilder` struct that emits OTel `gen_ai.*` semantic convention attributes (semconv >= 1.37):
   - `gen_ai.system` -- provider name ("anthropic", "openai", "google", "cerebras", "perplexity")
   - `gen_ai.request.model` -- model slug
   - `gen_ai.response.model` -- actual model returned (Empty, filled after response)
   - `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens` -- from `UsageObservation`
   - `roko.gateway.cost_usd`, `roko.gateway.cache_hit`, `roko.gateway.router_decision`
2. Add `fn otel_system_name(&self) -> &str` to `ProviderKind` in `crates/roko-core/src/agent.rs`:
   ```rust
   pub fn otel_system_name(&self) -> &str {
       match self {
           Self::AnthropicApi | Self::ClaudeCli => "anthropic",
           Self::OpenAiCompat => "openai",
           Self::GeminiApi => "google",
           Self::CerebrasApi => "cerebras",
           Self::PerplexityApi => "perplexity",
           Self::CursorAcp => "cursor",
       }
   }
   ```
3. Provide `GenAiSpanBuilder::span()` returning a `tracing::Span` and `record_response()` to fill in response-time attributes from `UsageObservation` (at `crates/roko-agent/src/usage.rs:17`).

**Acceptance criteria:**
- `GenAiSpanBuilder` compiles and `rg 'gen_ai\.system' crates/roko-agent/src/otel_spans.rs` matches.
- `otel_system_name()` exists on `ProviderKind`.

---

### Task 18.4: Wire `gen_ai.*` spans into `ModelCallService::call()`

**Depends on:** 18.3
**Crates:** `roko-agent`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs` -- `ModelCallService` (line 56), `call()` method

**What to do:**
1. In `ModelCallService::call()`, construct a `GenAiSpanBuilder` with the provider kind and model slug.
2. Enter the span before the provider call.
3. After the response, call `GenAiSpanBuilder::record_response()` with the `UsageObservation` from the response.
4. The span wraps the entire model call lifecycle: request construction, network call, response parsing, usage extraction.

**Acceptance criteria:**
- Every `ModelCallService::call()` invocation produces a `gen_ai.request` span.
- `rg 'GenAiSpanBuilder' crates/roko-agent/src/model_call_service.rs` matches at least 2 lines.
- `cargo check -p roko-agent` passes.

---

### Task 18.5: Batch `JsonlLogger` writes and defer flush

**Depends on:** None
**Crates:** `roko-runtime`, `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/jsonl_logger.rs` -- `JsonlLogger` (line 15), `write_event()` (line 62)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs` -- `WorkflowEngine::run()` completion
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` -- run completion path

**What to do:**
1. Add `unflushed: AtomicU64` field to `JsonlLogger`.
2. Add `const FLUSH_INTERVAL: u64 = 16`.
3. In `write_event()`, remove the per-event `w.flush()`. Instead, increment `unflushed` and flush only when `unflushed >= FLUSH_INTERVAL`.
4. Add `pub fn flush_all(&self) -> std::io::Result<()>` that forces a flush and resets the counter.
5. Call `flush_all()` from:
   - `WorkflowEngine::run()` completion path
   - `roko run` shutdown / cleanup path
   - Plan runner event loop shutdown path

**Anti-pattern removed:** AP-1 (per-event flush).
**Performance impact:** Saves 30-50ms per run (B11 from perf analysis).

**Acceptance criteria:**
- `rg 'FLUSH_INTERVAL\|flush_all' crates/roko-runtime/src/jsonl_logger.rs` matches >= 2.
- `rg 'flush_all' crates/roko-cli/src/run.rs crates/roko-runtime/src/workflow_engine.rs` matches >= 1.
- `cargo test -p roko-runtime` passes.

---

### Task 18.6: Add lazy event serialization to EventBus

**Depends on:** None
**Crates:** `roko-runtime`, `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs` -- `Envelope<E>` (line 64), `emit_inner()`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs` -- SSE adapter
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/jsonl_logger.rs` -- JSON serialization in `write_event()`

**What to do:**
1. Create `LazyJson<E>` struct in `event_bus.rs`:
   ```rust
   #[derive(Debug, Clone)]
   pub struct LazyJson<E: Serialize + Clone> {
       inner: E,
       json: OnceLock<String>,
   }
   ```
   - `.payload() -> &E` returns the typed event (zero-cost for TUI consumers).
   - `.to_json() -> &str` serializes once on first call via `OnceLock`.
2. Update `Envelope<E>` to use `LazyJson<E>` for the payload when `E: Serialize`.
3. Update SSE adapter to call `.to_json()` when serializing for the wire.
4. Update `JsonlLogger` to call `.to_json()` instead of re-serializing.
5. TUI bridge continues to use `.payload()` directly -- no serialization overhead.

**Anti-pattern removed:** AP-2 (eager serialization).
**Performance impact:** Saves 20-40ms per run (B13 from perf analysis).

**Acceptance criteria:**
- `LazyJson` type exists in `event_bus.rs`.
- SSE adapter uses `.to_json()`.
- `cargo test -p roko-runtime` passes.

---

### Task 18.7: Define `ObservabilityEvent` enum for all subsystems

**Depends on:** None
**Crates:** `roko-runtime`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/obs_events.rs` (NEW)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs` -- add `pub mod obs_events;`

**What to do:**
1. Create `obs_events.rs` with a unified `ObservabilityEvent` enum covering all observable domains:
   - **Dispatch:** `RouterDecision { request_id, policy_mode, candidates: Vec<CandidateScore>, chosen, reason }`, `EscalationEvent { request_id, from_model, to_model, reason, attempt }`
   - **Cost:** `CostTick { request_id, model, input_tokens, output_tokens, cache_read_tokens, cost_usd, turn_cost_usd, session_cost_usd, turn_budget_usd, session_budget_usd }`, `BudgetWarning { scope, used_usd, limit_usd, pct }`
   - **Gates:** `GateStarted { task_id, gate_name, rung }`, `GatePassed { task_id, gate_name, rung, duration_ms, detail }`, `GateFailed { task_id, gate_name, rung, duration_ms, feedback }`, `GateSkipped { task_id, gate_name, rung, reason }`, `ThresholdUpdated { rung, old, new, reason }`, `PipelineCompleted { task_id, passed, total_duration_ms, gates_run, gates_passed }`
   - **Agent Health:** `AgentHeartbeat { agent_id, model, uptime_secs, turns_completed, last_turn_cost_usd, memory_rss_bytes }`, `AgentStall { agent_id, stall_duration_secs, last_activity }`
   - **Learning:** `TierConfidenceUpdate { model, confidence, observations, avg_cost_per_call }`, `ExperimentUpdate { experiment_id, arm, trials, pass_rate, converged }`
   - **Anomaly:** `Anomaly { kind, severity, message, current_value, threshold }`
2. Add `CandidateScore { model, score, reason }` helper struct.
3. Use `#[serde(tag = "domain", content = "data", rename_all = "snake_case")]` for clean JSON.
4. Register module in `lib.rs`.

**Design note:** This is the single schema contract for all observability consumers (TUI, SSE, WebSocket, Prometheus, anomaly detectors). Downstream tasks emit these events; upstream tasks consume them.

**Acceptance criteria:**
- `ObservabilityEvent` enum exists with all listed variants.
- `rg 'ObservabilityEvent' crates/roko-runtime/src/obs_events.rs` matches >= 2.
- `rg 'mod obs_events' crates/roko-runtime/src/lib.rs` matches >= 1.
- `cargo check -p roko-runtime` passes.

---

### Task 18.8: Emit `RouterDecision` events from CascadeRouter

**Depends on:** 18.7
**Crates:** `roko-learn`, `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` -- `CascadeRouter` (line 82), `select()` (line 296), `select_for_frequency()` (line 309)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs` -- model routing decisions
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` -- escalation on gate failure

**What to do:**
1. After `CascadeRouter::select()` / `select_for_frequency()` returns, emit `ObservabilityEvent::RouterDecision` via `emit_runtime_event()` (from `event_bus.rs:362`). Include candidate scores from LinUCB, chosen model, and policy mode (static/confidence/UCB stage based on `stage_for_observations()`).
2. On gate failure escalation in `orchestrate.rs`, emit `ObservabilityEvent::EscalationEvent` with `from_model`, `to_model`, reason ("gate_failure"), and attempt number.
3. The `CandidateScore` structs should carry the score from `LinUCBRouter` for each candidate arm.

**Acceptance criteria:**
- `rg 'RouterDecision\|router_decision' crates/roko-learn/src/cascade_router.rs` matches >= 1.
- `rg 'EscalationEvent' crates/roko-cli/src/orchestrate.rs` matches >= 1.
- `cargo check -p roko-learn -p roko-cli` passes.

---

### Task 18.9: Emit `CostTick` events from `ModelCallService`

**Depends on:** 18.7
**Crates:** `roko-agent`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs` -- after model call completion, where `UsageObservation` is extracted
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/gateway_events.rs` -- `GatewayEventWriter` (line 46) already writes per-call

**What to do:**
1. After every `ModelCallService::call()` completion, emit `ObservabilityEvent::CostTick` with:
   - Token breakdown from `UsageObservation` (input_tokens, output_tokens, cache_read_tokens from `usage.rs:17`)
   - Per-call cost from `cost_table` computation
   - Cumulative turn/session cost (add `AtomicU64`-backed accumulators to `ModelCallService`)
   - Budget progress from configured `max_turn_usd` / `max_plan_usd`
2. Emit `ObservabilityEvent::BudgetWarning` when cumulative cost crosses 50%, 75%, or 90% of budget. Track which thresholds have been emitted to avoid duplicate warnings.

**Acceptance criteria:**
- `rg 'CostTick\|BudgetWarning' crates/roko-agent/src/model_call_service.rs` matches >= 2.
- `cargo check -p roko-agent` passes.

---

### Task 18.10: Emit granular `GateEvent` variants from `GateService`

**Depends on:** 18.7
**Crates:** `roko-gate`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs` -- `GateService` (line 26), gate loop at line 347, `AdaptiveThresholds::observe()` at line 353

**What to do:**
1. Before each `gate.verify()` call (line 347), emit `ObservabilityEvent::GateStarted { task_id, gate_name, rung }`.
2. After `gate.verify()` returns, emit `GatePassed` or `GateFailed` with duration_ms and detail/feedback.
3. When adaptive thresholds skip a gate (the early-break logic at line 353), emit `GateSkipped` with the reason.
4. When `AdaptiveThresholds::observe()` changes a threshold, emit `ThresholdUpdated` with old/new values.
5. At pipeline end, emit `PipelineCompleted` with total stats.
6. Thread the task_id through the gate service call chain (currently `GateService` does not receive it -- add as parameter or context).

**Anti-pattern removed:** AP-4 (gate results emitted only after pipeline completion).

**Acceptance criteria:**
- `rg 'GateStarted\|GatePassed\|GateSkipped' crates/roko-gate/src/ --type rust | grep -v test` matches >= 3.
- `rg 'ThresholdUpdated' crates/roko-gate/src/` matches >= 1.
- `cargo test -p roko-gate` passes.

---

### Task 18.11: Emit `TierConfidenceUpdate` from CascadeRouter persistence

**Depends on:** 18.7
**Crates:** `roko-learn`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` -- `observe()` (line 1130), `observe_multi_objective()` (line 1135), persistence calls
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs`

**What to do:**
1. After each `observe()` / `observe_multi_objective()` updates the LinUCB state, emit `ObservabilityEvent::TierConfidenceUpdate` for the observed model arm. Include:
   - Model name from the arm index
   - Confidence score (UCB value)
   - Total observations for this arm
   - Average cost per call (compute from cumulative cost / observations -- add tracking if not present)
2. Before persisting the router state to `cascade-router.json`, also persist `avg_cost_per_call` per-model alongside existing reward/observation counts.

**Acceptance criteria:**
- `rg 'TierConfidenceUpdate' crates/roko-learn/src/` matches >= 1.
- `cargo check -p roko-learn` passes.

---

### Task 18.12: Enrich Prometheus text exposition endpoint

**Depends on:** None (can use existing `GatewayProjection`, `CascadeRouter`)
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/metrics.rs` -- `prometheus_metrics()` (line 140)

**What to do:**
1. Expand the existing `prometheus_metrics()` handler to emit per-model counters:
   - `roko_model_calls_total{model,provider,status}` -- from `GatewayProjection::load()` (at `crates/roko-agent/src/gateway_events.rs:98`)
   - `roko_tokens_total{direction,model}` -- input/output/cache_read from gateway events
   - `roko_cost_usd_total{model}` -- cumulative cost per model
2. Add latency summary quantiles:
   - `roko_model_latency_seconds{model,quantile}` -- p50, p90, p99 from gateway event `wall_ms`
3. Add per-gate counters:
   - `roko_gate_results_total{gate,result}` -- pass/fail per gate type
   - `roko_gate_duration_seconds{gate,quantile}` -- p50, p90 from gate durations
4. Add gauge metrics:
   - `roko_active_agents` -- from `ProcessSupervisor` (at `crates/roko-runtime/src/process.rs:839`)
   - `roko_session_cost_usd` -- current session cost
   - `roko_cascade_stage{model}` -- current cascade routing stage
5. Use streaming text assembly (no metrics library). Compute quantiles using sorted arrays over observations from `GatewayProjection`.

**Anti-pattern removed:** AP-5 (incomplete Prometheus metrics).

**Acceptance criteria:**
- `rg 'roko_model_calls_total\|roko_tokens_total\|roko_cost_usd_total' crates/roko-serve/src/routes/status/metrics.rs` matches >= 3.
- `cargo check -p roko-serve` passes.

---

### Task 18.13: Add cost sparkline data to `/api/metrics/summary`

**Depends on:** None
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/metrics.rs`

**What to do:**
1. Add to the existing metrics summary response:
   - `cost_sparkline.turn_costs: Vec<f64>` -- last N turn costs from gateway events
   - `cost_sparkline.trend_pct: f64` -- percent change over recent window
   - `cost_sparkline.total_session_cost_usd: f64`
   - `cost_sparkline.savings_vs_always_premium_pct: f64` -- actual cost vs hypothetical all-opus cost
   - `token_breakdown: { input, output, cache_read, cache_write }` -- from `GatewayProjection`
2. Compute `savings_vs_always_premium_pct` by comparing actual cost against hypothetical cost using the most expensive model for all calls, sourced from `GatewayProjection` events.

**Acceptance criteria:**
- `rg 'cost_sparkline\|savings_vs_always_premium' crates/roko-serve/src/routes/status/metrics.rs` matches >= 2.
- `cargo check -p roko-serve` passes.

---

### Task 18.14: Add `RouterTrace` widget to TUI

**Depends on:** 18.8
**Crates:** `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/router_trace.rs` (NEW)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/mod.rs` -- add `pub mod router_trace;`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` -- `TuiState` (line 999)

**What to do:**
1. Create `router_trace.rs` with a ratatui widget that renders `RouterDecision` events:
   - Policy mode header (auto_cost / auto_learning / manual)
   - Per-candidate horizontal bar with score (0.0 - 1.0), model name, and reason
   - Highlight the chosen model
   - Show escalation chain when `EscalationEvent` occurs
2. Add `router_decisions: VecDeque<RouterDecision>` (capacity 50) to `TuiState`.
3. Use the existing `rosedust` theme from `crates/roko-cli/src/tui/widgets/rosedust.rs` for colors.

**Acceptance criteria:**
- `rg 'RouterTrace\|router_trace' crates/roko-cli/src/tui/widgets/` matches >= 2.
- `cargo check -p roko-cli` passes.

---

### Task 18.15: Add `CostPanel` widget to TUI

**Depends on:** 18.9
**Crates:** `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/cost_panel.rs` (NEW)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs`

**What to do:**
1. Create `cost_panel.rs` with a right-rail widget rendering live cost from `CostTick` events:
   - Turn cost vs budget progress bar
   - Session cost vs budget progress bar
   - Token breakdown (input/output/cache/think) using `fmt_tokens()` from existing `token_sparkline.rs`
   - Cost-per-turn sparkline using existing `braille.rs` renderer
   - Trend percentage and savings-vs-always-opus percentage
2. Progress bars: jade (<50%), amber (50-80%), crimson (>80%) from rosedust semantic colors.
3. Add `cost_history: VecDeque<f64>` (capacity 100) to `TuiState`.

**Acceptance criteria:**
- `rg 'CostPanel\|cost_panel' crates/roko-cli/src/tui/widgets/` matches >= 2.
- `cargo check -p roko-cli` passes.

---

### Task 18.16: Add `GateRow` live strip to TUI operations page

**Depends on:** 18.10
**Crates:** `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/pages/operations.rs` -- existing operations page
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs`

**What to do:**
1. Add `task_gates: HashMap<String, Vec<GateStatus>>` to `TuiState`, where `GateStatus` is a new enum: `Pending | Running | Passed { duration_ms: u64 } | Failed { duration_ms: u64, errors: u32 } | Skipped { reason: String }`.
2. Enhance the operations page to render a gate row strip below each task:
   - PASS: jade background
   - FAIL: crimson background with error count
   - RUN: amber pulsing (frame counter mod for blink)
   - ---: dim gray (pending)
   - SKIP: ghost text
3. Update gate state in real time as `GateStarted`/`GatePassed`/`GateFailed`/`GateSkipped` events arrive via StateHub subscription.

**Acceptance criteria:**
- `rg 'GateStatus\|gate_row\|task_gates' crates/roko-cli/src/tui/` matches >= 3.
- `cargo check -p roko-cli` passes.

---

### Task 18.17: Add `SwarmGates` widget for parallel agent tracking

**Depends on:** 18.10
**Crates:** `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/swarm_gates.rs` (NEW)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/mod.rs`

**What to do:**
1. Create `swarm_gates.rs` widget showing per-agent gate results for parallel/tournament execution:
   - One row per agent: name, model, gate dots (jade/crimson/gray), cost
   - Winner highlight after all agents complete (cheapest passing)
2. Feed from `ObservabilityEvent::GatePassed`/`GateFailed` filtered by agent_id.
3. Add `swarm_state: HashMap<String, Vec<AgentGateState>>` to `TuiState`.

**Acceptance criteria:**
- `rg 'SwarmGates\|swarm_gates' crates/roko-cli/src/tui/widgets/` matches >= 2.
- `cargo check -p roko-cli` passes.

---

### Task 18.18: Integrate new widgets into TUI App and view dispatch

**Depends on:** 18.14, 18.15, 18.16, 18.17
**Crates:** `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/dashboard_view.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/tabs.rs`

**What to do:**
1. **Dashboard view (F1):** Add `CostPanel` to the right rail. Add `RouterTrace` below the existing efficiency section.
2. **Operations page:** `GateRow` strips already integrated per task (18.16). Show `SwarmGates` when parallel execution is active.
3. **State management:** `TuiState` subscribes to `ObservabilityEvent` via the StateHub `watch::Receiver`. Ensure all new fields from 18.14-18.17 are populated.
4. **Event dispatch:** In `App::handle_dashboard_event()`, pattern-match `ObservabilityEvent` variants and update corresponding `TuiState` fields (router_decisions, cost_history, task_gates, swarm_state).

**Acceptance criteria:**
- `rg 'router_decisions\|cost_history\|task_gates\|swarm_state' crates/roko-cli/src/tui/state.rs` matches >= 4.
- `rg 'ObservabilityEvent' crates/roko-cli/src/tui/app.rs` matches >= 1.
- `cargo check -p roko-cli` passes.

---

### Task 18.19: Add agent health heartbeat and stall detection

**Depends on:** 18.7
**Crates:** `roko-runtime`, `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs` -- existing heartbeat module (cognitive clock)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/process.rs` -- `ProcessSupervisor` (line 839)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/agents_view.rs` -- existing agents TUI view

**What to do:**
1. Add an `AgentHealthMonitor` struct (in `heartbeat.rs` or a new file in `roko-runtime`) that:
   - Emits `ObservabilityEvent::AgentHeartbeat` every `heartbeat_interval_secs` (default: 10) for each process in `ProcessSupervisor::list()`.
   - Sources uptime from process start time, turns from usage accumulator, cost from usage.
   - Reads memory RSS via `mach_task_info` on macOS or `/proc/{pid}/status` on Linux (best-effort, `None` if unavailable).
2. Add stall detection: track last `AgentCompleted` / `AgentOutput` timestamp per agent_id. If gap exceeds `stall_timeout_secs` (default: 120), emit `ObservabilityEvent::AgentStall`.
3. Spawn the monitor as a tokio task from `orchestrate.rs` or the plan runner init.
4. Add heartbeat state rendering to `agents_view.rs`: uptime, turns, cost badge, stall warning indicator.

**Acceptance criteria:**
- `rg 'AgentHeartbeat\|AgentStall' crates/roko-runtime/src/` matches >= 2.
- `rg 'stall_timeout\|heartbeat_interval' crates/roko-cli/src/` matches >= 1.
- `cargo check -p roko-runtime -p roko-cli` passes.

---

### Task 18.20: Add WebSocket `ObservabilityEvent` streaming

**Depends on:** 18.7
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` -- existing WebSocket handler (line 28)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`

**What to do:**
1. The WebSocket endpoint at `/ws` already exists and streams `ServerEvent` payloads. Extend `handle_ws()` (line 68) to also stream `ObservabilityEvent` payloads from the obs bus (once Task 18.25 wires it into AppState).
2. Support client-side filter messages: `{"subscribe": ["router", "cost", "gate", "agent", "learning", "anomaly"]}` to limit which domains are streamed. Default: all domains.
3. Add filtering by domain: parse the `ObservabilityEvent` serde tag and match against the subscription list.
4. Use the existing back-pressure mode infrastructure (`BackPressureMode` at line 34) for obs events.

**Acceptance criteria:**
- WebSocket handler processes `ObservabilityEvent` in addition to `ServerEvent`.
- `rg 'ObservabilityEvent\|obs_bus' crates/roko-serve/src/routes/ws.rs` matches >= 1.
- `cargo check -p roko-serve` passes.

---

### Task 18.21: Add `/api/obs/events` SSE endpoint with domain filtering

**Depends on:** 18.7, 18.25
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/obs.rs` (NEW)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` -- add `mod obs;` and merge routes

**What to do:**
1. Create `obs.rs` with a dedicated SSE endpoint: `GET /api/obs/events?domains=router,cost,gate,agent,learning,anomaly`.
2. Subscribe to the `EventBus<ObservabilityEvent>` from `AppState::obs_bus`.
3. Each SSE frame: `id: <seq>`, `event: <domain_tag>`, `data: <JSON payload>`.
4. Domain filtering: parse `?domains=` query param, filter events by serde tag matching.
5. Replay recent events from the bus ring buffer on connect (same pattern as existing SSE at `routes/sse.rs`).
6. Register route in `routes/mod.rs`.

**Acceptance criteria:**
- `rg 'obs_events\|domain.*filter' crates/roko-serve/src/routes/obs.rs` matches >= 2.
- Route registered in `mod.rs`.
- `cargo check -p roko-serve` passes.

---

### Task 18.22: Add `/api/obs/cost` cost dashboard endpoint

**Depends on:** 18.9, 18.13
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/obs.rs`

**What to do:**
1. Add `GET /api/obs/cost?period=24h` REST endpoint returning:
   - `total_cost_usd`, `total_calls`
   - `cost_by_model: Vec<{ model, cost_usd, calls, avg_latency_ms }>`
   - `cost_by_role: Vec<{ role, cost_usd, calls }>` (if role available in gateway events)
   - `token_breakdown: { input, output, cache_read, cache_write }`
   - `savings_vs_premium: { actual_cost_usd, hypothetical_premium_cost_usd, savings_pct }`
   - `sparkline: Vec<f64>` -- per-turn costs
   - `cascade_stage`, `cascade_observations`
2. Source from `GatewayProjection` (at `crates/roko-agent/src/gateway_events.rs:98`) and `CascadeRouter` snapshot.
3. Filter events by `period` query param.

**Acceptance criteria:**
- `rg 'cost_dashboard\|obs/cost\|cost_by_model' crates/roko-serve/src/routes/obs.rs` matches >= 2.
- `cargo check -p roko-serve` passes.

---

### Task 18.23: Add `/api/obs/latency` latency profiling endpoint

**Depends on:** 18.1
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/obs.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` -- `AppState` (line 345)

**What to do:**
1. Create a `LatencyCollector` struct (in `state.rs` or a new module) that accumulates span durations from `tracing` instrumentation (Task 18.1) in a ring buffer of last 1000 observations per phase.
2. Add `GET /api/obs/latency?period=1h` endpoint returning per-phase percentile breakdown:
   - Phases: `config_load`, `learning_init`, `agent_construct`, `prompt_assembly`, `model_call`, `gate_pipeline`, `persistence`
   - Per-phase: `p50_ms`, `p90_ms`, `p99_ms`
   - Total: same percentiles across all phases combined
   - `bottleneck`: name of the phase with highest p50
3. Quantile computation uses sorted arrays (no external library).
4. Add `latency_collector: Arc<LatencyCollector>` to `AppState`.

**Acceptance criteria:**
- `rg 'latency_profile\|obs/latency\|LatencyCollector' crates/roko-serve/src/` matches >= 2.
- `cargo check -p roko-serve` passes.

---

### Task 18.24: Add `/api/obs/health` agent health dashboard endpoint

**Depends on:** 18.19
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/obs.rs`

**What to do:**
1. Add `GET /api/obs/health` endpoint returning per-agent health from heartbeats:
   - `agents: Vec<{ agent_id, model, status, uptime_secs, turns_completed, last_turn_cost_usd, memory_rss_mb, last_heartbeat_secs_ago, stall_duration_secs?, last_activity? }>`
   - `summary: { total, healthy, stalled, total_cost_usd }`
2. Source from `ProcessSupervisor` (at `crates/roko-runtime/src/process.rs:839`) and accumulated heartbeat/stall events in `AppState`.
3. Status: "healthy" if last heartbeat < 30s ago, "stalled" if stall event active, "unknown" otherwise.

**Acceptance criteria:**
- `rg 'agent_health\|obs/health' crates/roko-serve/src/routes/obs.rs` matches >= 2.
- `cargo check -p roko-serve` passes.

---

### Task 18.25: Stream `ObservabilityEvent` through StateHub and AppState

**Depends on:** 18.7
**Crates:** `roko-serve`, `roko-core`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` -- `AppState` (line 345)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` -- server startup

**What to do:**
1. Add `obs_bus: EventBus<ObservabilityEvent>` to `AppState`, initialized with capacity 1024.
2. Connect the obs_bus to:
   - `/api/obs/events` SSE endpoint (Task 18.21)
   - `/ws` WebSocket endpoint (Task 18.20)
   - TUI via `watch::Receiver` bridge (same pattern as existing `SharedStateHub`)
   - `LatencyCollector` for span accumulation (Task 18.23)
   - `AnomalyMonitor` (Task 18.29)
3. All emission points from Phase 2 tasks (18.8-18.11, 18.19) push events to this bus.
4. The bus maintains a replay ring of 1024 events for late-connecting consumers.

**Acceptance criteria:**
- `rg 'obs_bus\|ObservabilityEvent' crates/roko-serve/src/state.rs` matches >= 2.
- `cargo check -p roko-serve` passes.

---

### Task 18.26: Implement cost spike detector

**Depends on:** 18.9
**Crates:** `roko-learn`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/anomaly.rs` -- existing `AnomalyDetector` (line 20), `Anomaly::CostSpike` (line 213)

**What to do:**
1. Add a standalone `CostSpikeDetector` struct (separate from the existing session-local `AnomalyDetector`) designed for the obs pipeline:
   ```rust
   pub struct CostSpikeDetector {
       window: VecDeque<f64>,
       window_size: usize,          // default: 20
       threshold_sigma: f64,        // default: 3.0
   }
   ```
2. `observe(cost_usd) -> Option<ObservabilityEvent::Anomaly>` -- returns an anomaly when cost exceeds `mean + threshold_sigma * stddev`.
3. Severity: "warning" at 3 sigma, "critical" at 5 sigma.
4. This differs from the existing `AnomalyDetector::check_cost()` (line 77) which uses EWMA. The new detector uses a sliding window with explicit sigma thresholds, designed for the obs event pipeline rather than session-local checks.

**Acceptance criteria:**
- `rg 'CostSpikeDetector' crates/roko-learn/src/anomaly.rs` matches >= 2.
- `cargo test -p roko-learn` passes.

---

### Task 18.27: Implement gate pass-rate drop detector

**Depends on:** 18.10
**Crates:** `roko-learn`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/anomaly.rs`

**What to do:**
1. Add `PassRateDropDetector` struct:
   ```rust
   pub struct PassRateDropDetector {
       per_gate: HashMap<String, VecDeque<bool>>,
       window_size: usize,              // default: 30
       min_samples: usize,              // default: 10
       drop_threshold: f64,             // default: 0.2 (20% absolute drop)
   }
   ```
2. `observe(gate, passed) -> Option<ObservabilityEvent::Anomaly>` -- compares first-half vs second-half pass rate in the window. If drop > threshold, emit anomaly.
3. Severity: "warning" at 20% drop, "critical" at 40% drop.
4. This operates at aggregate level across runs, complementing the per-rung `AdaptiveThresholds` (at `crates/roko-gate/src/adaptive_threshold.rs`) which operates within a single pipeline execution.

**Acceptance criteria:**
- `rg 'PassRateDropDetector' crates/roko-learn/src/anomaly.rs` matches >= 2.
- `cargo test -p roko-learn` passes.

---

### Task 18.28: Implement latency surge detector

**Depends on:** 18.23
**Crates:** `roko-learn`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/anomaly.rs`

**What to do:**
1. Add `LatencySurgeDetector` struct:
   ```rust
   pub struct LatencySurgeDetector {
       per_model: HashMap<String, VecDeque<u64>>,  // latency_ms
       window_size: usize,                         // default: 50
       threshold_factor: f64,                      // default: 2.0 (2x median)
   }
   ```
2. `observe(model, latency_ms) -> Option<ObservabilityEvent::Anomaly>` -- computes median of window, triggers at `threshold_factor * median`.
3. Severity: "warning" at 2x median, "critical" at 5x median.
4. Feeds into provider health circuit breaker as an additional signal.

**Acceptance criteria:**
- `rg 'LatencySurgeDetector' crates/roko-learn/src/anomaly.rs` matches >= 2.
- `cargo test -p roko-learn` passes.

---

### Task 18.29: Wire anomaly detectors into the obs event pipeline

**Depends on:** 18.26, 18.27, 18.28, 18.25
**Crates:** `roko-learn`, `roko-cli`, `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/anomaly.rs` -- add `AnomalyMonitor`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` -- spawn monitor task
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` -- spawn monitor task

**What to do:**
1. Create `AnomalyMonitor` struct in `anomaly.rs` holding all three detectors:
   ```rust
   pub struct AnomalyMonitor {
       cost: CostSpikeDetector,
       pass_rate: PassRateDropDetector,
       latency: LatencySurgeDetector,
   }
   ```
2. `process(event: &ObservabilityEvent) -> Vec<ObservabilityEvent>` -- dispatches to appropriate detector based on event variant (CostTick -> cost, GatePassed/GateFailed -> pass_rate, CostTick.wall_ms -> latency), returns anomaly events.
3. Spawn a background `tokio::task` that:
   - Subscribes to the `EventBus<ObservabilityEvent>`
   - Runs `AnomalyMonitor::process()` on each event
   - Re-emits any anomaly events back onto the same bus
4. Start this task from both `roko serve` startup (in `lib.rs`) and `roko plan run` initialization (in `orchestrate.rs` or event_loop).

**Anti-pattern removed:** AP-6 (anomaly detection not wired to obs pipeline).

**Acceptance criteria:**
- `rg 'AnomalyMonitor' crates/roko-learn/src/anomaly.rs crates/roko-cli/src/ crates/roko-serve/src/` matches >= 3.
- `cargo check -p roko-learn -p roko-cli -p roko-serve` passes.

---

### Task 18.30: Add anomaly alert rendering to TUI and CLI

**Depends on:** 18.29
**Crates:** `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/error_digest.rs` -- existing `render_error_digest()` (line 33)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`

**What to do:**
1. **TUI:** Extend `render_error_digest()` to accept and render `Anomaly` events from TuiState:
   - Critical anomalies: crimson background banner, persists until acknowledged.
   - Warning anomalies: amber background, auto-dismiss after 30 seconds.
   - Display kind, message, current_value vs threshold.
2. Add `anomalies: VecDeque<TimestampedAnomaly>` to `TuiState`.
3. **CLI (`roko plan run`):** When an `ObservabilityEvent::Anomaly` arrives in the event loop, print a colored line to stderr:
   ```
   [WARN] cost_spike: Cost $0.142 exceeds 3.2x mean ($0.044)
   [CRIT] pass_rate_drop: test pass rate dropped 40% (90% -> 50%)
   ```
4. SSE/WS: anomaly events automatically stream via the obs bus (no additional work).

**Acceptance criteria:**
- `rg 'Anomaly\|anomaly' crates/roko-cli/src/tui/widgets/error_digest.rs` matches >= 1.
- `rg 'cost_spike\|pass_rate_drop\|latency_surge' crates/roko-cli/src/` matches >= 1.
- `cargo check -p roko-cli` passes.

---

### Task 18.31: Implement OTLP trace exporter initialization

**Depends on:** 18.2, 18.3
**Crates:** `roko-cli`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/otel_init.rs` (NEW)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**What to do:**
1. Create `otel_init.rs` with `init_otel() -> Option<OtelGuard>`:
   - Reads `OTEL_EXPORTER_OTLP_ENDPOINT` (env, required to activate).
   - Reads `OTEL_SERVICE_NAME` (env, default: "roko").
   - Reads `OTEL_EXPORTER_OTLP_HEADERS` (env, for auth -- Langfuse basic-auth, Honeycomb team key, etc.).
   - Falls back to `roko.toml [observability.otel]` section if env vars are unset.
   - Configures `opentelemetry_otlp::SpanExporter` with tonic transport.
   - Creates `SdkTracerProvider` with batch exporter and resource attributes.
   - Composes `tracing_opentelemetry::layer()` with the existing `tracing_subscriber`.
   - Returns `OtelGuard` with Drop impl that calls `provider.shutdown()`.
2. Guard all OTel code behind `#[cfg(feature = "otel")]`.
3. Call `init_otel()` from `main.rs` subscriber setup, before any `tracing` spans are emitted.
4. Do NOT use the `opentelemetry-langfuse` crate (bus factor 1). Standard OTLP with auth headers works with all vendors.

**Acceptance criteria:**
- `rg 'init_otel\|OtelGuard' crates/roko-cli/src/otel_init.rs` matches >= 2.
- `cargo check -p roko-cli --features otel` passes.
- Without `otel` feature, no OTel code is compiled.

---

### Task 18.32: Add `roko config otel` subcommand

**Depends on:** 18.31
**Crates:** `roko-cli`, `roko-core`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/` -- config schema (add `[observability.otel]` section)

**What to do:**
1. Add `roko config otel` subcommands:
   - `set --endpoint <url> --auth-header <header> --service-name <name>` -- writes to `roko.toml [observability.otel]`
   - `show` -- prints current OTel config
   - `test` -- sends a test span to the configured endpoint and reports success/failure
   - `disable` -- sets `enabled = false` in config
2. Add `[observability.otel]` section to config schema:
   ```toml
   [observability.otel]
   endpoint = "https://..."
   auth_header = "Authorization=Basic ..."
   service_name = "roko"
   enabled = true
   ```
3. `init_otel()` (from 18.31) reads both env vars (higher priority) and `roko.toml` (fallback).

**Acceptance criteria:**
- `rg 'otel.*set\|otel.*show\|otel.*test' crates/roko-cli/src/commands/config_cmd.rs` matches >= 3.
- `rg 'otel' crates/roko-core/src/config/` matches >= 1.
- `cargo check -p roko-cli -p roko-core` passes.

---

### Task 18.33: Add end-to-end observability integration test

**Depends on:** 18.21, 18.22, 18.25, 18.29
**Crates:** `roko-serve`
**Files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/tests/obs_integration.rs` (NEW)

**What to do:**
1. Write an integration test verifying the full pipeline:
   - Start `AppState` with a tempdir workdir.
   - Emit `ObservabilityEvent::CostTick` and `GatePassed` events to `state.obs_bus`.
   - Verify events appear on `/api/obs/events` SSE endpoint (parse SSE frames).
   - Verify `/api/obs/cost` REST endpoint returns cost data.
   - Verify Prometheus endpoint includes new `roko_model_calls_total` counter.
   - Emit a cost spike (10x normal) and verify `AnomalyMonitor` produces an `Anomaly` event.
   - Verify the anomaly event appears on SSE.
2. Use `build_router()` and `tower::ServiceExt::oneshot()` following the existing test pattern at `crates/roko-serve/src/routes/mod.rs:252`.
3. Use `build_test_router()` helper or equivalent.

**Acceptance criteria:**
- `rg 'obs_events_flow\|obs_integration' crates/roko-serve/tests/` matches >= 1.
- `cargo test -p roko-serve obs_integration` passes.

---

## Dependency Graph

```
Phase 1: Tracing + OTel Foundation
  18.1  (instrument spans) ---- independent
  18.2  (OTel deps)        ---- independent
  18.5  (batch logger)     ---- independent
  18.6  (lazy serialization) -- independent
  18.2 <- 18.3 (gen_ai spans) <- 18.4 (wire to ModelCallService)

Phase 2: Structured Events
  18.7  (ObservabilityEvent enum) ---- independent
  18.7 <- 18.8  (RouterDecision)
  18.7 <- 18.9  (CostTick)
  18.7 <- 18.10 (GateEvent)
  18.7 <- 18.11 (TierConfidence)
  18.12 (Prometheus) ---- independent
  18.13 (CostSparkline) ---- independent

Phase 3: TUI + Streaming
  18.8  <- 18.14 (RouterTrace widget)
  18.9  <- 18.15 (CostPanel widget)
  18.10 <- 18.16 (GateRow strip)
  18.10 <- 18.17 (SwarmGates widget)
  18.14, 18.15, 18.16, 18.17 <- 18.18 (integrate into App)
  18.7  <- 18.19 (agent heartbeat)
  18.7  <- 18.20 (WebSocket feed)

Phase 4: API Endpoints
  18.7, 18.25 <- 18.21 (SSE obs endpoint)
  18.9, 18.13 <- 18.22 (cost dashboard)
  18.1         <- 18.23 (latency profiling)
  18.19        <- 18.24 (agent health)
  18.7         <- 18.25 (StateHub wiring)

Phase 5: Anomaly Detection
  18.9  <- 18.26 (cost spike)
  18.10 <- 18.27 (pass rate drop)
  18.23 <- 18.28 (latency surge)
  18.26, 18.27, 18.28, 18.25 <- 18.29 (wire into pipeline)
  18.29 <- 18.30 (alert rendering)

Phase 6: OTel Export
  18.2, 18.3        <- 18.31 (OTLP exporter)
  18.31              <- 18.32 (config subcommand)
  18.21, 18.22, 18.29, 18.31 <- 18.33 (integration test)
```

---

## Effort Estimates

| Phase | Tasks | Effort | Cumulative |
|---|---|---|---|
| Phase 1: Tracing + OTel Foundation | 18.1-18.6 | 3-4 days | 3-4 days |
| Phase 2: Structured Events | 18.7-18.13 | 3-4 days | 6-8 days |
| Phase 3: TUI + Streaming | 18.14-18.20 | 4-5 days | 10-13 days |
| Phase 4: API Endpoints | 18.21-18.25 | 2-3 days | 12-16 days |
| Phase 5: Anomaly Detection | 18.26-18.30 | 2-3 days | 14-19 days |
| Phase 6: OTel Export | 18.31-18.33 | 2-3 days | 16-22 days |

---

## Success Criteria

### Must Have (Phase 1-3)
- Every `ModelCallService::call()` emits a `gen_ai.*` span with model, tokens, cost, latency.
- TUI dashboard shows RouterTrace, CostPanel, and GateRow widgets with live data.
- `JsonlLogger` batches writes (measured: < 5ms persistence overhead per run).
- Prometheus endpoint exports `roko_model_calls_total`, `roko_cost_usd_total`, `roko_gate_results_total`.
- Agent heartbeat and stall detection runs for all supervised agents.

### Should Have (Phase 4-5)
- `/api/obs/cost` returns model/role cost breakdown with savings calculation.
- `/api/obs/latency` returns per-phase percentile breakdown.
- Anomaly detection fires on cost spikes, pass-rate drops, and latency surges.
- Anomaly alerts render in TUI error digest and CLI stderr.
- WebSocket endpoint streams filtered events.

### Nice to Have (Phase 6)
- `cargo run -p roko-cli --features otel -- run "hello"` sends spans to configured OTel endpoint.
- `roko config otel set` configures export without env vars.
- End-to-end integration test verifies full pipeline.
