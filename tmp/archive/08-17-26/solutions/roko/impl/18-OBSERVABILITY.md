# Observability, Monitoring & Dashboard Implementation Plan

**Status:** Draft v1
**Date:** 2026-04-29
**Prerequisites:** RuntimeEvent infrastructure (done), EventBus (done), SSE streaming (done), StateHub (done), GatewayEventWriter (done), MetricRecorder (done), Prometheus text endpoint (done)
**Sources:** 14-GATE-VIZ-07 (dashboard integration), 19-DISPATCH-GOALS (UX data feeds), 18-LEARN-GOALS (learning data feeds), 20-GATE-GOALS (gate events), 13-PERF-BOTTLENECK-ANALYSIS (measurement methodology), 21-GTM-GATEWAY-ADAPTERS (OTel)

---

## 0. Current State and Gaps

### What Exists

| Component | Location | State |
|---|---|---|
| `RuntimeEvent` enum | `crates/roko-core/src/runtime_event.rs` | 12 variants (lifecycle, agent, gate, feedback) |
| `EventBus<E>` broadcast + replay ring | `crates/roko-runtime/src/event_bus.rs` | Wired, bounded, sequenced |
| `JsonlLogger` (RuntimeEvent sink) | `crates/roko-runtime/src/jsonl_logger.rs` | Wired, per-event flush |
| `MetricRecorder` (generic JSONL) | `crates/roko-runtime/src/metrics.rs` | Wired, append-only |
| `GatewayEventWriter` (per-call log) | `crates/roko-agent/src/gateway_events.rs` | Wired, writes to `.roko/learn/gateway.jsonl` |
| `UsageObservation` (canonical telemetry) | `crates/roko-agent/src/usage.rs` | Wired, 8 fields |
| SSE endpoint `/api/events` | `crates/roko-serve/src/routes/sse.rs` | Wired, replay + live stream |
| `ServerEvent` enum | `crates/roko-serve/src/events.rs` | ~15 variants (plan, agent, gate, inference) |
| Prometheus text exposition | `crates/roko-serve/src/routes/status/metrics.rs` | Basic counters (uptime, agents, plans, episodes) |
| 12 metrics endpoints | `crates/roko-serve/src/routes/status/mod.rs` | Summary, success rate, c-factor, gate rate, etc. |
| TUI token sparkline | `crates/roko-cli/src/tui/widgets/token_sparkline.rs` | Renders efficiency snapshot |
| TUI verdicts aggregator | `crates/roko-cli/src/tui/verdicts.rs` | Rolling 24h gate stats from substrate |
| TUI operations page | `crates/roko-cli/src/tui/pages/operations.rs` | Task execution with inline gate results |
| TUI efficiency page | `crates/roko-cli/src/tui/pages/efficiency.rs` | Efficiency metrics and cost tracking |

### What Is Missing

| Gap | Impact | Phase |
|---|---|---|
| No OTel spans on model calls | Cannot export to Datadog/Honeycomb/Langfuse | Phase 1 |
| No `gen_ai.*` semantic convention attributes | Missing from observability vendor dashboards | Phase 1 |
| No RouterTrace data feed | TUI/web cannot show cascade routing decisions | Phase 2 |
| No CostPanel data feed | No real-time cost progress bars | Phase 2 |
| No GateRow live streaming | Gate results not streamed per-criterion | Phase 3 |
| No SwarmGates per-agent feed | Parallel agent gates not independently tracked | Phase 3 |
| No latency profiling spans | Cannot identify bottleneck phases in runs | Phase 1 |
| No anomaly alerting | Cost spikes and pass-rate drops go unnoticed | Phase 4 |
| No agent health monitoring | No heartbeat/liveness for long-running agents | Phase 3 |
| Prometheus metrics incomplete | Missing token counts, cost, latency percentiles | Phase 2 |
| No WebSocket event feed | Only SSE available; no bidirectional channel | Phase 3 |
| `JsonlLogger` flushes per-event | 20-40ms overhead from sync I/O (B11 from PERF analysis) | Phase 1 |
| Event bus serializes eagerly | 20-40ms overhead even without consumers (B13) | Phase 1 |

---

## Phase 1: Tracing Instrumentation and OTel Foundation (6 tasks)

### Task 1: Add `tracing` instrument spans to critical dispatch paths

**Files:**
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/dispatch_v2.rs`

Add `#[tracing::instrument]` spans at each bottleneck boundary identified in the performance analysis. Each span captures the phase name, model, and wall-clock duration.

```rust
#[tracing::instrument(skip_all, fields(
    phase = "model_call",
    model = %model_slug,
    caller = %caller_id,
))]
async fn call_model(&self, req: &ModelCallRequest) -> Result<ModelCallResponse> {
    // existing body
}
```

Instrument these phases:
- `config_load` (B02): `load_layered()` and `load_config()`
- `learning_init` (B03): `LearningRuntime::open_under()`
- `agent_construct` (B04/B15): `create_agent_for_model()`
- `prompt_assembly` (B12): `PromptAssemblyService::assemble()`
- `model_call`: `ModelCallService::call()` (the network-bound call)
- `gate_pipeline`: `run_gate_pipeline()` and `run_selected_gate_pipeline()`
- `persistence`: substrate writes, episode logging, feedback flush
- `cascade_routing`: `resolve_effective_model()` with efficiency signal load

**Acceptance criteria:**
```bash
# At least 8 instrumented spans in the dispatch/runtime path
rg '#\[tracing::instrument' crates/roko-agent/src/model_call_service.rs \
  crates/roko-cli/src/run.rs crates/roko-cli/src/orchestrate.rs \
  crates/roko-cli/src/dispatch_v2.rs --type rust | wc -l  # >= 8
```

---

### Task 2: Add `opentelemetry` and `opentelemetry-otlp` dependencies

**Files:**
- `Cargo.toml` (workspace root)
- `crates/roko-runtime/Cargo.toml`
- `crates/roko-agent/Cargo.toml`
- `crates/roko-cli/Cargo.toml`

Add workspace-level dependencies:

```toml
# Workspace Cargo.toml [workspace.dependencies]
opentelemetry = { version = "0.28", default-features = false, features = ["trace"] }
opentelemetry_sdk = { version = "0.28", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.28", features = ["tonic"] }
tracing-opentelemetry = "0.28"
```

Wire the `tracing-opentelemetry` layer into the CLI subscriber setup in `crates/roko-cli/src/main.rs`. Use feature-gated compilation so OTel is opt-in:

```toml
# crates/roko-cli/Cargo.toml
[features]
otel = ["opentelemetry", "opentelemetry_sdk", "opentelemetry-otlp", "tracing-opentelemetry"]
```

The OTel exporter is configured via standard env vars (`OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`). When the `otel` feature is disabled or no endpoint is set, no exporter is initialized and tracing falls back to the existing `tracing_subscriber` setup.

**Acceptance criteria:**
```bash
# Feature compiles
cargo check -p roko-cli --features otel

# Without the feature, no OTel dependency in the binary
cargo tree -p roko-cli --no-default-features | grep -c opentelemetry  # 0
```

---

### Task 3: Implement `gen_ai.*` span builder for gateway layer

**File:** `crates/roko-agent/src/otel_spans.rs` (new)

Create a span builder that emits `gen_ai.*` semantic convention attributes on every model call. These attributes are consumed by Datadog (March 2026+), Honeycomb, Langfuse, and Grafana.

```rust
use tracing::Span;

/// Attributes following OTel gen_ai semantic conventions (semconv >= 1.37).
pub struct GenAiSpanBuilder {
    pub system: String,           // "anthropic", "openai", "google"
    pub request_model: String,    // "claude-sonnet-4-6"
    pub response_model: Option<String>,
}

impl GenAiSpanBuilder {
    /// Create a tracing span with gen_ai.* attributes.
    pub fn span(&self) -> Span {
        tracing::info_span!(
            "gen_ai.request",
            gen_ai.system = %self.system,
            gen_ai.request.model = %self.request_model,
            gen_ai.response.model = tracing::field::Empty,
            gen_ai.usage.input_tokens = tracing::field::Empty,
            gen_ai.usage.output_tokens = tracing::field::Empty,
            roko.gateway.cache_hit = tracing::field::Empty,
            roko.gateway.router_decision = tracing::field::Empty,
            roko.gateway.cost_usd = tracing::field::Empty,
            roko.gateway.safety_flags = tracing::field::Empty,
            roko.agent.chain_id = tracing::field::Empty,
        )
    }

    /// Record response-time attributes after the call completes.
    pub fn record_response(
        span: &Span,
        usage: &UsageObservation,
        router_decision: &str,
        cache_hit: bool,
    ) {
        if let Some(input) = usage.input_tokens {
            span.record("gen_ai.usage.input_tokens", input);
        }
        if let Some(output) = usage.output_tokens {
            span.record("gen_ai.usage.output_tokens", output);
        }
        if let Some(cost) = usage.cost_usd {
            span.record("roko.gateway.cost_usd", cost);
        }
        span.record("roko.gateway.cache_hit", cache_hit);
        span.record("roko.gateway.router_decision", router_decision);
    }
}
```

Gateway-specific attributes beyond the standard `gen_ai.*` namespace:

| Attribute | Source | Type |
|---|---|---|
| `gen_ai.system` | Provider kind | string |
| `gen_ai.request.model` | Router selection | string |
| `gen_ai.usage.input_tokens` | Provider response | i64 |
| `gen_ai.usage.output_tokens` | Provider response | i64 |
| `roko.gateway.cache_hit` | Cache layer | string (L1/L2/miss) |
| `roko.gateway.router_decision` | CascadeRouter arm | string |
| `roko.gateway.cost_usd` | Cost computation | f64 |
| `roko.gateway.safety_flags` | Safety pipeline | string |

**Acceptance criteria:**
```bash
# gen_ai span builder exists and compiles
rg 'gen_ai\.system' crates/roko-agent/src/otel_spans.rs --type rust | wc -l  # >= 1

# ModelCallService uses the span builder
rg 'GenAiSpanBuilder' crates/roko-agent/src/model_call_service.rs --type rust | wc -l  # >= 1
```

---

### Task 4: Wire `gen_ai.*` spans into `ModelCallService::call()`

**File:** `crates/roko-agent/src/model_call_service.rs`

Wrap each model call in a `gen_ai.*` span. The span starts before the provider call and records usage attributes after completion.

```rust
// In ModelCallService::call():
let gen_ai = GenAiSpanBuilder {
    system: provider_kind.otel_system_name().to_string(),
    request_model: model_slug.to_string(),
    response_model: None,
};
let span = gen_ai.span();
let _guard = span.enter();

// ... existing call logic ...

// After response:
GenAiSpanBuilder::record_response(
    &span,
    &usage_observation,
    &router_decision_label,
    cache_hit,
);
```

Add `fn otel_system_name(&self) -> &str` to `ProviderKind` in `crates/roko-core/src/agent.rs`:

```rust
impl ProviderKind {
    pub fn otel_system_name(&self) -> &str {
        match self {
            Self::Anthropic | Self::ClaudeCli => "anthropic",
            Self::OpenAi => "openai",
            Self::Google => "google",
            Self::Cerebras => "cerebras",
            Self::Perplexity => "perplexity",
            _ => "unknown",
        }
    }
}
```

**Acceptance criteria:**
```bash
# ModelCallService wraps calls in gen_ai spans
rg 'gen_ai.*span\|GenAiSpanBuilder' crates/roko-agent/src/model_call_service.rs \
  --type rust | wc -l  # >= 2

# ProviderKind has OTel name mapping
rg 'otel_system_name' crates/roko-core/src/ --type rust | wc -l  # >= 1
```

---

### Task 5: Batch `JsonlLogger` writes and defer flush

**File:** `crates/roko-runtime/src/jsonl_logger.rs`

Replace per-event `w.flush()` with buffered writes and periodic/shutdown flush. This addresses bottleneck B11 from the performance analysis (30-50ms savings per run).

```rust
impl JsonlLogger {
    /// Write an event to the buffer. Does NOT flush immediately.
    fn write_event(&self, event: &RuntimeEvent) -> std::io::Result<()> {
        self.ensure_writer()?;
        let envelope = RuntimeEventEnvelope::new(
            event.run_id(),
            self.seq.fetch_add(1, Ordering::Relaxed),
            "jsonl_logger",
            event.clone(),
        );
        let json = serde_json::to_string(&envelope)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        let mut writer = self.writer.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref mut w) = *writer {
            writeln!(w, "{json}")?;
            // Flush only every N events or on explicit flush_all()
            let count = self.unflushed.fetch_add(1, Ordering::Relaxed);
            if count >= Self::FLUSH_INTERVAL {
                w.flush()?;
                self.unflushed.store(0, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    /// Flush all buffered events to disk. Call at run completion.
    pub fn flush_all(&self) -> std::io::Result<()> {
        let mut writer = self.writer.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref mut w) = *writer {
            w.flush()?;
            self.unflushed.store(0, Ordering::Relaxed);
        }
        Ok(())
    }
}
```

Add `unflushed: AtomicU64` field and `const FLUSH_INTERVAL: u64 = 16`.

Ensure `flush_all()` is called from `WorkflowEngine::run()` completion and `roko run` shutdown.

**Acceptance criteria:**
```bash
# Per-event flush replaced with interval flush
rg 'FLUSH_INTERVAL\|flush_all' crates/roko-runtime/src/jsonl_logger.rs --type rust | wc -l  # >= 2

# flush_all called from run completion paths
rg 'flush_all' crates/roko-cli/src/run.rs crates/roko-runtime/src/workflow_engine.rs \
  --type rust | wc -l  # >= 1
```

---

### Task 6: Add lazy event serialization to EventBus

**File:** `crates/roko-runtime/src/event_bus.rs`

Address bottleneck B13: event payloads are serialized to JSON on every `emit()`, even when no consumer will serialize them. Add lazy serialization so the JSON representation is computed once, only when first requested.

```rust
/// Lazy-serialized event wrapper.
#[derive(Debug, Clone)]
pub struct LazyJson<E: Serialize + Clone> {
    inner: E,
    json: OnceLock<String>,
}

impl<E: Serialize + Clone> LazyJson<E> {
    pub fn new(inner: E) -> Self {
        Self { inner, json: OnceLock::new() }
    }

    pub fn payload(&self) -> &E { &self.inner }

    pub fn to_json(&self) -> &str {
        self.json.get_or_init(|| {
            serde_json::to_string(&self.inner).unwrap_or_default()
        })
    }
}
```

Update `Envelope<E>` to use `LazyJson<E>` for the payload field. The SSE adapter and JSONL logger call `.to_json()` when they need the serialized form; the TUI bridge accesses `.payload()` directly without serialization overhead.

**Acceptance criteria:**
```bash
# LazyJson type exists
rg 'LazyJson' crates/roko-runtime/src/event_bus.rs --type rust | wc -l  # >= 2

# SSE adapter uses to_json()
rg 'to_json\(\)' crates/roko-serve/src/routes/sse.rs --type rust | wc -l  # >= 1
```

---

## Phase 2: Structured Event Emission and Data Feeds (7 tasks)

### Task 7: Define `ObservabilityEvent` enum for all subsystems

**File:** `crates/roko-runtime/src/obs_events.rs` (new)

Create a unified observability event enum that all subsystems emit through the EventBus. This consolidates the currently scattered event types (`RuntimeEvent`, `ServerEvent`, `ExecutionEvent`, `GatewayEvent`) into a single schema for consumers.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "domain", content = "data", rename_all = "snake_case")]
pub enum ObservabilityEvent {
    // -- Dispatch --
    RouterDecision {
        request_id: String,
        policy_mode: String,          // "auto_cost", "auto_learning", "manual"
        candidates: Vec<CandidateScore>,
        chosen: String,               // model slug
        reason: String,
    },
    EscalationEvent {
        request_id: String,
        from_model: String,
        to_model: String,
        reason: String,               // "gate_failure", "budget_exceeded"
        attempt: u32,
    },

    // -- Cost --
    CostTick {
        request_id: String,
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cost_usd: f64,
        turn_cost_usd: f64,
        session_cost_usd: f64,
        turn_budget_usd: f64,
        session_budget_usd: f64,
    },
    BudgetWarning {
        scope: String,                // "turn", "session", "plan"
        used_usd: f64,
        limit_usd: f64,
        pct: f64,                     // 0.5, 0.75, 0.9
    },

    // -- Gates --
    GateStarted { task_id: String, gate_name: String, rung: u8 },
    GatePassed { task_id: String, gate_name: String, rung: u8, duration_ms: u64, detail: String },
    GateFailed { task_id: String, gate_name: String, rung: u8, duration_ms: u64, feedback: String },
    GateSkipped { task_id: String, gate_name: String, rung: u8, reason: String },
    ThresholdUpdated { rung: u8, old: f64, new: f64, reason: String },
    PipelineCompleted {
        task_id: String,
        passed: bool,
        total_duration_ms: u64,
        gates_run: u8,
        gates_passed: u8,
    },

    // -- Agent Health --
    AgentHeartbeat {
        agent_id: String,
        model: String,
        uptime_secs: u64,
        turns_completed: u64,
        last_turn_cost_usd: f64,
        memory_rss_bytes: Option<u64>,
    },
    AgentStall {
        agent_id: String,
        stall_duration_secs: u64,
        last_activity: String,
    },

    // -- Learning --
    TierConfidenceUpdate {
        model: String,
        confidence: f64,
        observations: u64,
        avg_cost_per_call: f64,
    },
    ExperimentUpdate {
        experiment_id: String,
        arm: String,
        trials: u64,
        pass_rate: f64,
        converged: bool,
    },

    // -- Anomaly --
    Anomaly {
        kind: String,                 // "cost_spike", "pass_rate_drop", "latency_surge"
        severity: String,             // "warning", "critical"
        message: String,
        current_value: f64,
        threshold: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateScore {
    pub model: String,
    pub score: f64,
    pub reason: String,
}
```

Register the new module in `crates/roko-runtime/src/lib.rs`.

**Acceptance criteria:**
```bash
rg 'ObservabilityEvent' crates/roko-runtime/src/obs_events.rs --type rust | wc -l  # >= 2
rg 'mod obs_events' crates/roko-runtime/src/lib.rs --type rust | wc -l  # >= 1
```

---

### Task 8: Emit `RouterDecision` events from CascadeRouter

**Files:**
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-cli/src/orchestrate.rs`

After `CascadeRouter::select()` returns a model, emit a `RouterDecision` event through the EventBus with candidate scores, chosen model, and policy mode (static/confidence/UCB). Also emit `EscalationEvent` when a gate failure triggers model escalation.

The data feed shape matches the RouterTrace card from 19-DISPATCH-GOALS section 4A:
- Policy mode: `auto_cost` / `auto_learning` / `post_replay`
- Candidate list with score bars
- Active chosen candidate
- Escalation state

**Acceptance criteria:**
```bash
rg 'RouterDecision\|router_decision' crates/roko-learn/src/cascade_router.rs \
  crates/roko-cli/src/orchestrate.rs --type rust | wc -l  # >= 2
rg 'EscalationEvent' crates/roko-cli/src/orchestrate.rs --type rust | wc -l  # >= 1
```

---

### Task 9: Emit `CostTick` events from `ModelCallService`

**Files:**
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-agent/src/gateway_events.rs`

After every model call completes, emit a `CostTick` event with cumulative turn/session costs and budget progress. The data feed matches the CostPanel from 19-DISPATCH-GOALS section 4B:
- This-turn cost vs turn budget (progress bar data)
- Session cost vs session budget
- 4-cell token breakdown (input/output/cached/thought)

Wire through `BudgetCell` to get cumulative cost tracking. Emit `BudgetWarning` events at 50%, 75%, 90% thresholds.

**Acceptance criteria:**
```bash
rg 'CostTick\|BudgetWarning' crates/roko-agent/src/model_call_service.rs --type rust | wc -l  # >= 2
```

---

### Task 10: Emit granular `GateEvent` variants from `GateService`

**Files:**
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-gate/src/gate_pipeline.rs`
- `crates/roko-cli/src/orchestrate.rs`

Emit `GateStarted`, `GatePassed`/`GateFailed`/`GateSkipped`, `ThresholdUpdated`, and `PipelineCompleted` events from within the gate pipeline execution. Currently, gate results are emitted only as `RuntimeEvent::GatePassed`/`GateFailed` from orchestrate.rs after the entire pipeline completes. The new events fire per-gate as each rung executes, enabling live GateRow rendering in both TUI and web dashboard.

This matches the GateRow data feed from 20-GATE-GOALS section 9:
- Per-gate: name, status (pending/running/passed/failed/skipped), detail, duration
- Pulsing amber dot when running

Also emit `ThresholdUpdated` when `AdaptiveThresholds::observe()` changes a rung threshold.

**Acceptance criteria:**
```bash
rg 'GateStarted\|GatePassed\|GateSkipped' crates/roko-gate/src/ --type rust \
  | grep -v test | wc -l  # >= 3
rg 'ThresholdUpdated' crates/roko-gate/src/ --type rust | wc -l  # >= 1
```

---

### Task 11: Emit `TierConfidenceUpdate` from CascadeRouter persistence

**Files:**
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/model_router.rs`

After each routing observation and before persisting the updated router state, emit `TierConfidenceUpdate` events for each model arm. This data feed matches the Tier Confidence Panel from 19-DISPATCH-GOALS section 4C:
- Per-model confidence bars with cost
- Model name, confidence percentage, average cost per call

Also persist the per-model `avg_cost_per_call` alongside the existing reward/observation counts in `cascade-router.json`.

**Acceptance criteria:**
```bash
rg 'TierConfidenceUpdate' crates/roko-learn/src/ --type rust | wc -l  # >= 1
```

---

### Task 12: Enrich Prometheus text exposition endpoint

**File:** `crates/roko-serve/src/routes/status/metrics.rs`

Expand the `GET /api/metrics/prometheus` endpoint with counters and histograms for:

```
# TYPE roko_model_calls_total counter
roko_model_calls_total{model="claude-sonnet-4-6",provider="anthropic",status="success"} 142
roko_model_calls_total{model="claude-haiku-4-5",provider="anthropic",status="success"} 87
roko_model_calls_total{model="claude-sonnet-4-6",provider="anthropic",status="error"} 3

# TYPE roko_tokens_total counter
roko_tokens_total{direction="input",model="claude-sonnet-4-6"} 1452000
roko_tokens_total{direction="output",model="claude-sonnet-4-6"} 312000
roko_tokens_total{direction="cache_read",model="claude-sonnet-4-6"} 890000

# TYPE roko_cost_usd_total counter
roko_cost_usd_total{model="claude-sonnet-4-6"} 12.34
roko_cost_usd_total{model="claude-haiku-4-5"} 1.87

# TYPE roko_model_latency_seconds summary
roko_model_latency_seconds{model="claude-sonnet-4-6",quantile="0.5"} 1.2
roko_model_latency_seconds{model="claude-sonnet-4-6",quantile="0.9"} 2.8
roko_model_latency_seconds{model="claude-sonnet-4-6",quantile="0.99"} 5.1

# TYPE roko_gate_results_total counter
roko_gate_results_total{gate="compile",result="pass"} 130
roko_gate_results_total{gate="compile",result="fail"} 12
roko_gate_results_total{gate="test",result="pass"} 118
roko_gate_results_total{gate="test",result="fail"} 24

# TYPE roko_gate_duration_seconds summary
roko_gate_duration_seconds{gate="compile",quantile="0.5"} 0.52
roko_gate_duration_seconds{gate="test",quantile="0.5"} 2.1

# TYPE roko_cascade_stage gauge
roko_cascade_stage{model="claude-sonnet-4-6"} 2

# TYPE roko_active_agents gauge
roko_active_agents 3

# TYPE roko_session_cost_usd gauge
roko_session_cost_usd 4.56
```

Source data from: `GatewayProjection` (model calls, tokens, cost, latency), gate history JSONL, `CascadeRouter` snapshot, and `ProcessSupervisor` counts.

Use streaming text assembly (not a metrics library) to keep dependencies minimal. Quantile computation uses sorted arrays over the last 1000 observations.

**Acceptance criteria:**
```bash
rg 'roko_model_calls_total\|roko_tokens_total\|roko_cost_usd_total' \
  crates/roko-serve/src/routes/status/metrics.rs --type rust | wc -l  # >= 3
```

---

### Task 13: Add `CostSparkline` data to `/api/metrics/summary`

**File:** `crates/roko-serve/src/routes/status/metrics.rs`

Add cost-per-turn sparkline data to the metrics summary response. This surfaces the "cumulative savings vs always-opus" metric from 18-LEARN-GOALS section 4.4.

Response shape addition:
```json
{
  "cost_sparkline": {
    "turn_costs": [0.012, 0.008, 0.042, 0.003, ...],
    "trend_pct": -12.5,
    "total_session_cost_usd": 4.56,
    "savings_vs_always_premium_pct": 87.2
  },
  "token_breakdown": {
    "input": 1452000,
    "output": 312000,
    "cache_read": 890000,
    "cache_write": 42000
  }
}
```

Compute `savings_vs_always_premium_pct` by comparing actual cost against hypothetical cost using the most expensive model for all calls. Source from `GatewayProjection` events.

**Acceptance criteria:**
```bash
rg 'cost_sparkline\|savings_vs_always_premium' crates/roko-serve/src/routes/status/metrics.rs \
  --type rust | wc -l  # >= 2
```

---

## Phase 3: TUI Data Streams and Dashboard Widgets (7 tasks)

### Task 14: Add `RouterTrace` widget to TUI

**Files:**
- `crates/roko-cli/src/tui/widgets/router_trace.rs` (new)
- `crates/roko-cli/src/tui/widgets/mod.rs`
- `crates/roko-cli/src/tui/state.rs`

Create a ratatui widget that renders `RouterDecision` events as a compact card showing cascade routing decisions:

```
Router: auto-cost (UCB stage, 247 obs)
  haiku-4.5    0.94  ████████████████████  trivial task
  sonnet-4.6   0.62  ████████████░░░░░░░░  overkill
  opus-4.6     0.31  ██████░░░░░░░░░░░░░░  wasteful
  > chose: haiku-4.5 (cost: $0.001)
```

When an escalation occurs, show the escalation chain:
```
  haiku-4.5    0.32  ██████░░░░░░░░░░░░░░  gate fail -> escalated
  sonnet-4.6   0.91  ██████████████████░░  escalated to
```

Store the last N `RouterDecision` events in `TuiState` for display. Subscribe to `ObservabilityEvent::RouterDecision` via the EventBus/StateHub.

**Acceptance criteria:**
```bash
rg 'RouterTrace\|router_trace' crates/roko-cli/src/tui/widgets/ --type rust | wc -l  # >= 2
```

---

### Task 15: Add `CostPanel` widget to TUI

**Files:**
- `crates/roko-cli/src/tui/widgets/cost_panel.rs` (new)
- `crates/roko-cli/src/tui/widgets/mod.rs`
- `crates/roko-cli/src/tui/state.rs`

Create a right-rail panel widget that renders live cost tracking from `CostTick` events:

```
Cost
  Turn:     $0.008 / $0.50  [==.......] 1.6%
  Session:  $4.56 / $10.00  [========.] 45.6%

  Tokens:
    in: 12.4k  out: 3.2k  cache: 8.9k  think: 1.1k

  $/turn sparkline: ..._,-'^-,_..^'
  Trend: -12.5% vs last 10 turns
  Savings vs always-opus: 87%
```

Progress bars use ROSEDUST semantic colors (jade for <50%, amber for 50-80%, crimson for >80%). Token counts use the `fmt_tokens()` helper from the existing `token_sparkline.rs`. The sparkline uses the existing braille renderer.

Feed data from `CostTick` events via StateHub subscription. Accumulate in `TuiState::cost_history: VecDeque<f64>` (last 100 turns).

**Acceptance criteria:**
```bash
rg 'CostPanel\|cost_panel' crates/roko-cli/src/tui/widgets/ --type rust | wc -l  # >= 2
```

---

### Task 16: Add `GateRow` live strip to TUI operations page

**Files:**
- `crates/roko-cli/src/tui/pages/operations.rs`
- `crates/roko-cli/src/tui/state.rs`

Enhance the existing operations page to show live gate result strips below each task message. As `GateStarted`/`GatePassed`/`GateFailed`/`GateSkipped` events arrive, update the per-task gate row in real time:

```
Task: implement-login-form (wave 2, slot 3)
  Agent: claude-sonnet-4 (turn 4/8)
  Gates: [compile PASS 0.12s] [clippy PASS 0.34s] [test RUN...] [diff ---] [fmt ---]
```

Each gate cell shows:
- PASS: jade background
- FAIL: crimson background with error count
- RUN: amber pulsing (use frame counter mod for blink)
- ---: dim gray (pending)
- SKIP: ghost text with reason on hover

Store per-task gate state in `TuiState::task_gates: HashMap<String, Vec<GateStatus>>`.

**Acceptance criteria:**
```bash
rg 'GateStatus\|gate_row\|task_gates' crates/roko-cli/src/tui/ --type rust | wc -l  # >= 3
```

---

### Task 17: Add `SwarmGates` widget for parallel agent tracking

**Files:**
- `crates/roko-cli/src/tui/widgets/swarm_gates.rs` (new)
- `crates/roko-cli/src/tui/widgets/mod.rs`

Create a widget that shows per-agent gate results for parallel (tournament) execution, matching the SwarmGates data feed from 20-GATE-GOALS section 9:

```
Swarm: wave-3 (3 agents, tournament mode)
  agent-a (sonnet): [compile OK] [test OK] [clippy OK]  $0.042
  agent-b (haiku):  [compile OK] [test FAIL] [---]       $0.008
  agent-c (opus):   [compile OK] [test OK] [clippy OK]  $0.084
  Winner: agent-a (cheapest passing)
```

Each agent row tracks gates independently. Gate dots are 3-color (jade/crimson/gray). The winner is highlighted after all agents complete.

Feed from `ObservabilityEvent::GatePassed`/`GateFailed` events filtered by agent_id.

**Acceptance criteria:**
```bash
rg 'SwarmGates\|swarm_gates' crates/roko-cli/src/tui/widgets/ --type rust | wc -l  # >= 2
```

---

### Task 18: Integrate new widgets into TUI App and view dispatch

**Files:**
- `crates/roko-cli/src/tui/app.rs`
- `crates/roko-cli/src/tui/views/dashboard_view.rs`
- `crates/roko-cli/src/tui/state.rs`
- `crates/roko-cli/src/tui/tabs.rs`

Wire the new widgets (RouterTrace, CostPanel, GateRow, SwarmGates) into the TUI layout:

1. **Dashboard view (F1):** Add CostPanel to the right rail. Add RouterTrace below the existing efficiency section.
2. **Operations page:** GateRow strips already integrated per task (Task 16). SwarmGates shown when parallel execution is active.
3. **State management:** `TuiState` subscribes to `ObservabilityEvent` via the StateHub `watch::Receiver`. New fields:
   - `router_decisions: VecDeque<RouterDecision>` (last 50)
   - `cost_history: VecDeque<CostTick>` (last 100)
   - `task_gates: HashMap<String, Vec<GateStatus>>`
   - `swarm_state: HashMap<String, Vec<AgentGateState>>`

4. **Event dispatch:** In `App::handle_dashboard_event()`, pattern-match `ObservabilityEvent` variants and update the corresponding `TuiState` fields.

**Acceptance criteria:**
```bash
rg 'router_decisions\|cost_history\|task_gates\|swarm_state' \
  crates/roko-cli/src/tui/state.rs --type rust | wc -l  # >= 4
rg 'ObservabilityEvent' crates/roko-cli/src/tui/app.rs --type rust | wc -l  # >= 1
```

---

### Task 19: Add agent health heartbeat and stall detection

**Files:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/process.rs`
- `crates/roko-cli/src/orchestrate.rs`

Wire agent health monitoring that emits `AgentHeartbeat` events at a configurable interval (default: 10s) for each supervised agent, and `AgentStall` events when an agent exceeds a stall timeout (default: 120s without a turn completion).

The heartbeat data sources:
- `ProcessSupervisor::list()` for uptime and process state
- `UsageObservation` accumulator for cost and turn counts
- `/proc/{pid}/status` (Linux) or `mach_task_info` (macOS) for RSS (best-effort, `None` if unavailable)

Stall detection: track last `AgentCompleted` or `AgentOutput` timestamp per agent. If the gap exceeds `stall_timeout_secs`, emit `AgentStall` with the duration and last known activity.

The TUI agents view (`views/agents_view.rs`) already exists. Add heartbeat state rendering: uptime, turns, cost, and a stall warning badge when stalled.

**Acceptance criteria:**
```bash
rg 'AgentHeartbeat\|AgentStall' crates/roko-runtime/src/ --type rust | wc -l  # >= 2
rg 'stall_timeout\|heartbeat_interval' crates/roko-cli/src/orchestrate.rs --type rust | wc -l  # >= 1
```

---

### Task 20: Add WebSocket event feed alongside SSE

**Files:**
- `crates/roko-serve/src/routes/ws.rs` (new)
- `crates/roko-serve/src/routes/mod.rs`

Add a WebSocket endpoint at `/api/ws` that streams the same `DashboardEvent` / `ObservabilityEvent` payloads as the SSE endpoint, plus supports bidirectional messaging for future interactive controls.

```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.state_hub.subscribe_events();
    // Replay recent events
    for envelope in state.state_hub.replay_from(0) {
        let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
        if socket.send(Message::Text(data.into())).await.is_err() {
            return;
        }
    }
    // Stream live events
    loop {
        tokio::select! {
            Ok(envelope) = rx.recv() => {
                let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
                if socket.send(Message::Text(data.into())).await.is_err() {
                    break;
                }
            }
            Some(Ok(msg)) = socket.recv() => {
                // Handle client messages (subscriptions, filters)
                handle_client_message(msg, &state).await;
            }
            else => break,
        }
    }
}
```

Register the route in `routes/mod.rs`. Support an optional `?filter=gate,cost,router` query parameter that limits which event domains are streamed.

**Acceptance criteria:**
```bash
rg 'ws_handler\|WebSocket' crates/roko-serve/src/routes/ws.rs --type rust | wc -l  # >= 2
rg 'ws::routes' crates/roko-serve/src/routes/mod.rs --type rust | wc -l  # >= 1
```

---

## Phase 4: SSE/API Event Feeds and Cost Dashboard (5 tasks)

### Task 21: Add `/api/obs/events` endpoint with domain filtering

**Files:**
- `crates/roko-serve/src/routes/obs.rs` (new)
- `crates/roko-serve/src/routes/mod.rs`

Create a dedicated SSE endpoint for `ObservabilityEvent` streaming with domain-level filtering, separate from the general `/api/events` endpoint:

```
GET /api/obs/events?domains=router,cost,gate,agent,learning,anomaly
```

Each SSE frame carries:
```
id: 42
event: router_decision
data: {"domain":"router_decision","data":{...}}

id: 43
event: cost_tick
data: {"domain":"cost_tick","data":{...}}
```

Domain filtering reduces bandwidth for consumers that only need a subset of events (e.g., the cost dashboard only subscribes to `cost` and `anomaly`).

Register in `routes/mod.rs` alongside the existing SSE routes.

**Acceptance criteria:**
```bash
rg 'obs_events\|domain.*filter' crates/roko-serve/src/routes/obs.rs --type rust | wc -l  # >= 2
```

---

### Task 22: Add `/api/obs/cost` cost dashboard endpoint

**Files:**
- `crates/roko-serve/src/routes/obs.rs`

Add a REST endpoint that returns the current cost dashboard state:

```
GET /api/obs/cost?period=24h
```

Response:
```json
{
  "period": "24h",
  "total_cost_usd": 12.34,
  "total_calls": 142,
  "cost_by_model": [
    {"model": "claude-sonnet-4-6", "cost_usd": 8.42, "calls": 87, "avg_latency_ms": 1200},
    {"model": "claude-haiku-4-5", "cost_usd": 1.87, "calls": 42, "avg_latency_ms": 400}
  ],
  "cost_by_role": [
    {"role": "implementer", "cost_usd": 6.20, "calls": 52},
    {"role": "reviewer", "cost_usd": 2.10, "calls": 38}
  ],
  "token_breakdown": {
    "input": 1452000, "output": 312000,
    "cache_read": 890000, "cache_write": 42000
  },
  "savings_vs_premium": {
    "actual_cost_usd": 12.34,
    "hypothetical_premium_cost_usd": 94.50,
    "savings_pct": 86.9
  },
  "sparkline": [0.012, 0.008, 0.042, ...],
  "cascade_stage": "ucb",
  "cascade_observations": 247
}
```

Source from `GatewayProjection` and `CascadeRouter` snapshot.

**Acceptance criteria:**
```bash
rg 'cost_dashboard\|obs/cost\|cost_by_model' crates/roko-serve/src/routes/obs.rs \
  --type rust | wc -l  # >= 2
```

---

### Task 23: Add `/api/obs/latency` latency profiling endpoint

**Files:**
- `crates/roko-serve/src/routes/obs.rs`

Add an endpoint that returns latency breakdown by phase, matching the performance bottleneck analysis methodology:

```
GET /api/obs/latency?period=1h
```

Response:
```json
{
  "period": "1h",
  "sample_count": 42,
  "phases": {
    "config_load": {"p50_ms": 10, "p90_ms": 15, "p99_ms": 40},
    "learning_init": {"p50_ms": 70, "p90_ms": 100, "p99_ms": 200},
    "agent_construct": {"p50_ms": 10, "p90_ms": 30, "p99_ms": 50},
    "prompt_assembly": {"p50_ms": 50, "p90_ms": 80, "p99_ms": 200},
    "model_call": {"p50_ms": 1200, "p90_ms": 2800, "p99_ms": 5100},
    "gate_pipeline": {"p50_ms": 520, "p90_ms": 1500, "p99_ms": 2000},
    "persistence": {"p50_ms": 20, "p90_ms": 50, "p99_ms": 100}
  },
  "total": {"p50_ms": 1880, "p90_ms": 4575, "p99_ms": 7690},
  "bottleneck": "model_call"
}
```

Source from `tracing` span durations. Collect the last 1000 span observations in a ring buffer in `AppState` (or a dedicated `LatencyCollector` struct). Quantile computation uses sorted arrays.

**Acceptance criteria:**
```bash
rg 'latency_profile\|obs/latency\|LatencyCollector' crates/roko-serve/src/ --type rust | wc -l  # >= 2
```

---

### Task 24: Add `/api/obs/health` agent health dashboard endpoint

**Files:**
- `crates/roko-serve/src/routes/obs.rs`

Add an endpoint that aggregates agent health from heartbeats and stall detection:

```
GET /api/obs/health
```

Response:
```json
{
  "agents": [
    {
      "agent_id": "impl-01",
      "model": "claude-sonnet-4-6",
      "status": "healthy",
      "uptime_secs": 3600,
      "turns_completed": 42,
      "last_turn_cost_usd": 0.012,
      "memory_rss_mb": 128,
      "last_heartbeat_secs_ago": 5
    },
    {
      "agent_id": "review-01",
      "model": "claude-haiku-4-5",
      "status": "stalled",
      "uptime_secs": 1800,
      "turns_completed": 8,
      "stall_duration_secs": 145,
      "last_activity": "AgentOutput at 2026-04-29T12:34:00Z"
    }
  ],
  "summary": {
    "total": 3,
    "healthy": 2,
    "stalled": 1,
    "total_cost_usd": 4.56
  }
}
```

Source from `ProcessSupervisor` and the heartbeat/stall events accumulated in `AppState`.

**Acceptance criteria:**
```bash
rg 'agent_health\|obs/health' crates/roko-serve/src/routes/obs.rs --type rust | wc -l  # >= 2
```

---

### Task 25: Stream `ObservabilityEvent` through existing StateHub

**Files:**
- `crates/roko-serve/src/state.rs`
- `crates/roko-serve/src/lib.rs`

Wire `ObservabilityEvent` into the existing `StateHub` push-based pattern. The StateHub already uses `tokio::sync::watch::Sender<DashboardEvent>` for TUI/SSE/WS consumers.

Add an `EventBus<ObservabilityEvent>` to `AppState` alongside the existing `event_bus: EventBus<ServerEvent>`:

```rust
pub struct AppState {
    // ... existing fields ...
    pub obs_bus: EventBus<ObservabilityEvent>,
}
```

Connect the obs_bus to:
1. The new `/api/obs/events` SSE endpoint (Task 21)
2. The new `/api/ws` WebSocket endpoint (Task 20)
3. The TUI via a `watch::Receiver` bridge
4. The `LatencyCollector` for span-duration accumulation (Task 23)
5. The anomaly detector (Task 28)

All emission points from Phase 2 (Tasks 8-11, 19) push to this bus. The bus has a replay ring of 1024 events.

**Acceptance criteria:**
```bash
rg 'obs_bus\|ObservabilityEvent' crates/roko-serve/src/state.rs --type rust | wc -l  # >= 2
```

---

## Phase 5: Anomaly Detection and Alerting (5 tasks)

### Task 26: Implement cost spike detector

**Files:**
- `crates/roko-learn/src/anomaly.rs` (new)
- `crates/roko-learn/src/lib.rs`

Create a cost spike detector that monitors `CostTick` events and emits `Anomaly` events when cost per turn exceeds a configurable threshold above the rolling mean:

```rust
pub struct CostSpikeDetector {
    window: VecDeque<f64>,       // last N turn costs
    window_size: usize,          // default: 20
    threshold_sigma: f64,        // default: 3.0 (3 standard deviations)
}

impl CostSpikeDetector {
    /// Returns Some(Anomaly) if the new cost exceeds mean + threshold_sigma * stddev.
    pub fn observe(&mut self, cost_usd: f64) -> Option<Anomaly> {
        self.window.push_back(cost_usd);
        if self.window.len() > self.window_size {
            self.window.pop_front();
        }
        if self.window.len() < 5 { return None; }

        let mean = self.window.iter().sum::<f64>() / self.window.len() as f64;
        let variance = self.window.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.window.len() as f64;
        let stddev = variance.sqrt();
        let threshold = mean + self.threshold_sigma * stddev;

        if cost_usd > threshold {
            Some(Anomaly {
                kind: "cost_spike".to_string(),
                severity: if cost_usd > mean + 5.0 * stddev { "critical" } else { "warning" }.to_string(),
                message: format!("Cost ${cost_usd:.4} exceeds {:.1}x mean (${mean:.4})", cost_usd / mean),
                current_value: cost_usd,
                threshold,
            })
        } else {
            None
        }
    }
}
```

**Acceptance criteria:**
```bash
rg 'CostSpikeDetector' crates/roko-learn/src/anomaly.rs --type rust | wc -l  # >= 2
```

---

### Task 27: Implement gate pass-rate drop detector

**File:** `crates/roko-learn/src/anomaly.rs`

Add a gate pass-rate drop detector that monitors the per-gate pass rate over a sliding window and emits an anomaly when the rate drops below a configurable threshold or exhibits a statistically significant decline.

```rust
pub struct PassRateDropDetector {
    per_gate: HashMap<String, VecDeque<bool>>,
    window_size: usize,              // default: 30
    min_samples: usize,              // default: 10
    drop_threshold: f64,             // default: 0.2 (20% absolute drop)
}

impl PassRateDropDetector {
    pub fn observe(&mut self, gate: &str, passed: bool) -> Option<Anomaly> {
        let window = self.per_gate.entry(gate.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.window_size));
        window.push_back(passed);
        if window.len() > self.window_size {
            window.pop_front();
        }
        if window.len() < self.min_samples { return None; }

        let total = window.len() as f64;
        let pass_count = window.iter().filter(|&&p| p).count() as f64;
        let current_rate = pass_count / total;

        // Compare first half vs second half for trend detection
        let half = window.len() / 2;
        let first_half_rate = window.iter().take(half).filter(|&&p| p).count() as f64 / half as f64;
        let second_half_rate = window.iter().skip(half).filter(|&&p| p).count() as f64
            / (window.len() - half) as f64;
        let drop = first_half_rate - second_half_rate;

        if drop > self.drop_threshold {
            Some(Anomaly {
                kind: "pass_rate_drop".to_string(),
                severity: if drop > 0.4 { "critical" } else { "warning" }.to_string(),
                message: format!("{gate} pass rate dropped {drop:.0}% ({first_half_rate:.0}% -> {second_half_rate:.0}%)"),
                current_value: current_rate,
                threshold: first_half_rate - self.drop_threshold,
            })
        } else { None }
    }
}
```

This integrates with the existing SPC ensemble in `crates/roko-gate/src/` but operates at a higher level -- monitoring aggregate trends across runs rather than per-rung adaptive thresholds.

**Acceptance criteria:**
```bash
rg 'PassRateDropDetector' crates/roko-learn/src/anomaly.rs --type rust | wc -l  # >= 2
```

---

### Task 28: Implement latency surge detector

**File:** `crates/roko-learn/src/anomaly.rs`

Add a latency surge detector that monitors per-model latency and emits an anomaly when latency exceeds historical norms, which may indicate provider degradation.

```rust
pub struct LatencySurgeDetector {
    per_model: HashMap<String, VecDeque<u64>>,  // latency_ms
    window_size: usize,                         // default: 50
    threshold_factor: f64,                      // default: 2.0 (2x median)
}
```

The detector emits with severity `warning` at 2x median and `critical` at 5x median. This feeds into the provider health circuit breaker (from 19-DISPATCH-GOALS section 1D) as an additional signal.

**Acceptance criteria:**
```bash
rg 'LatencySurgeDetector' crates/roko-learn/src/anomaly.rs --type rust | wc -l  # >= 2
```

---

### Task 29: Wire anomaly detectors into the obs event pipeline

**Files:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-serve/src/lib.rs`

Create an `AnomalyMonitor` that holds all three detectors (cost spike, pass-rate drop, latency surge) and subscribes to the `ObservabilityEvent` bus. When an anomaly is detected, it emits an `ObservabilityEvent::Anomaly` back onto the bus.

```rust
pub struct AnomalyMonitor {
    cost: CostSpikeDetector,
    pass_rate: PassRateDropDetector,
    latency: LatencySurgeDetector,
}

impl AnomalyMonitor {
    pub fn process(&mut self, event: &ObservabilityEvent) -> Vec<ObservabilityEvent> {
        let mut anomalies = Vec::new();
        match event {
            ObservabilityEvent::CostTick { cost_usd, .. } => {
                if let Some(a) = self.cost.observe(*cost_usd) {
                    anomalies.push(ObservabilityEvent::Anomaly(a));
                }
            }
            ObservabilityEvent::GatePassed { gate_name, .. } => {
                if let Some(a) = self.pass_rate.observe(gate_name, true) {
                    anomalies.push(ObservabilityEvent::Anomaly(a));
                }
            }
            ObservabilityEvent::GateFailed { gate_name, .. } => {
                if let Some(a) = self.pass_rate.observe(gate_name, false) {
                    anomalies.push(ObservabilityEvent::Anomaly(a));
                }
            }
            // latency from CostTick.wall_ms or dedicated event
            _ => {}
        }
        anomalies
    }
}
```

Spawn a background `tokio::task` that reads from the obs bus, runs the monitor, and re-emits anomaly events. The task is started from `roko serve` startup and from `roko plan run` initialization.

**Acceptance criteria:**
```bash
rg 'AnomalyMonitor' crates/roko-cli/src/orchestrate.rs crates/roko-serve/src/ \
  --type rust | wc -l  # >= 2
```

---

### Task 30: Add anomaly alert rendering to TUI and CLI

**Files:**
- `crates/roko-cli/src/tui/widgets/error_digest.rs`
- `crates/roko-cli/src/tui/app.rs`
- `crates/roko-cli/src/orchestrate.rs`

When an `ObservabilityEvent::Anomaly` event arrives:

1. **TUI:** Show a notification banner in the error digest widget. Critical anomalies use crimson background. Warning anomalies use amber. The banner auto-dismisses after 30 seconds for warnings, persists until acknowledged for critical.

2. **CLI (`roko plan run`):** Print a colored warning line to stderr:
   ```
   [WARN] cost_spike: Cost $0.142 exceeds 3.2x mean ($0.044)
   [CRIT] pass_rate_drop: test pass rate dropped 40% (90% -> 50%)
   ```

3. **SSE/WS:** Anomaly events are automatically streamed via the obs bus to connected clients.

**Acceptance criteria:**
```bash
rg 'Anomaly\|anomaly' crates/roko-cli/src/tui/widgets/error_digest.rs --type rust | wc -l  # >= 1
rg 'cost_spike\|pass_rate_drop\|latency_surge' crates/roko-cli/src/orchestrate.rs \
  --type rust | wc -l  # >= 1
```

---

## Phase 6: OTel Export and Vendor Integration (3 tasks)

### Task 31: Implement OTLP trace exporter initialization

**Files:**
- `crates/roko-cli/src/otel_init.rs` (new)
- `crates/roko-cli/src/main.rs`

Create an OTel initialization module that configures the OTLP exporter and `tracing-opentelemetry` layer. Controlled by environment variables following the OTel spec:

```rust
/// Initialize OTel exporter. Returns None if OTEL_EXPORTER_OTLP_ENDPOINT is unset.
pub fn init_otel() -> Option<OtelGuard> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "roko".to_string());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .build()
        .ok()?;

    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name),
            opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ]))
        .build();

    let tracer = tracer_provider.tracer("roko");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Compose with existing tracing subscriber
    // ...

    Some(OtelGuard { provider: tracer_provider })
}

pub struct OtelGuard {
    provider: opentelemetry_sdk::trace::SdkTracerProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        let _ = self.provider.shutdown();
    }
}
```

For Langfuse specifically (per 21-GTM-GATEWAY-ADAPTERS): use `opentelemetry-otlp` directly with basic-auth headers. Do NOT use the `opentelemetry-langfuse` crate (bus factor 1). Same code repoints at any vendor by changing env vars:

```bash
# Langfuse
export OTEL_EXPORTER_OTLP_ENDPOINT=https://us.cloud.langfuse.com/api/public/otel
export OTEL_EXPORTER_OTLP_HEADERS="Authorization=Basic $(echo -n pk:sk | base64)"

# Honeycomb
export OTEL_EXPORTER_OTLP_ENDPOINT=https://api.honeycomb.io
export OTEL_EXPORTER_OTLP_HEADERS="x-honeycomb-team=YOUR_KEY"

# Datadog
export OTEL_EXPORTER_OTLP_ENDPOINT=https://intake.datadoghq.com/v1
export DD_API_KEY=YOUR_KEY
```

**Acceptance criteria:**
```bash
rg 'init_otel\|OtelGuard' crates/roko-cli/src/otel_init.rs --type rust | wc -l  # >= 2
# Feature-gated compilation succeeds
cargo check -p roko-cli --features otel
```

---

### Task 32: Add `roko config otel` subcommand for OTel configuration

**Files:**
- `crates/roko-cli/src/commands/config_cmd.rs`
- `crates/roko-core/src/config/serve.rs`

Add a convenience subcommand to configure OTel export without manually setting env vars:

```bash
roko config otel set --endpoint https://us.cloud.langfuse.com/api/public/otel \
  --auth-header "Authorization=Basic ..." \
  --service-name "roko-prod"

roko config otel show     # Display current config
roko config otel test     # Send a test span to verify connectivity
roko config otel disable  # Clear configuration
```

Store the OTel config in `roko.toml`:

```toml
[observability.otel]
endpoint = "https://us.cloud.langfuse.com/api/public/otel"
auth_header = "Authorization=Basic ..."
service_name = "roko-prod"
enabled = true
```

The `init_otel()` function from Task 31 reads from both env vars (higher priority) and `roko.toml` (fallback).

**Acceptance criteria:**
```bash
rg 'otel.*set\|otel.*show\|otel.*test' crates/roko-cli/src/commands/config_cmd.rs \
  --type rust | wc -l  # >= 3
rg 'otel' crates/roko-core/src/config/ --type rust | wc -l  # >= 1
```

---

### Task 33: Add end-to-end observability integration test

**Files:**
- `crates/roko-serve/tests/obs_integration.rs` (new)

Write an integration test that verifies the full observability pipeline:

1. Start an `AppState` with a mock workdir
2. Emit `ObservabilityEvent::CostTick` and `GatePassed` events to the obs bus
3. Verify they appear on the `/api/obs/events` SSE endpoint
4. Verify they appear on the `/api/obs/cost` REST endpoint
5. Verify the Prometheus endpoint includes the new counters
6. Verify anomaly detection triggers on a cost spike
7. Verify the WebSocket endpoint delivers the anomaly event

```rust
#[tokio::test]
async fn obs_events_flow_through_sse_and_rest() {
    let (dir, state) = test_state();
    let app = build_router(Arc::clone(&state), &[], ServeAuthConfig::default());

    // Emit a cost tick
    state.obs_bus.emit(ObservabilityEvent::CostTick { ... });

    // Verify SSE delivers the event
    let response = app.clone().oneshot(
        Request::builder().uri("/api/obs/events?domains=cost").body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(response.status(), 200);

    // Verify REST cost endpoint reflects the data
    let response = app.oneshot(
        Request::builder().uri("/api/obs/cost").body(Body::empty()).unwrap()
    ).await.unwrap();
    let body: Value = parse_body(response).await;
    assert!(body["total_cost_usd"].as_f64().unwrap() > 0.0);
}
```

**Acceptance criteria:**
```bash
rg 'obs_events_flow\|obs_integration' crates/roko-serve/tests/ --type rust | wc -l  # >= 1
cargo test -p roko-serve obs_integration  # passes
```

---

## Dependency Graph

```
Phase 1: Tracing + OTel Foundation
  T1 (instrument spans)
  T2 (OTel deps) ← T3 (gen_ai spans) ← T4 (wire to ModelCallService)
  T5 (batch logger) -- independent
  T6 (lazy serialization) -- independent

Phase 2: Structured Events
  T7 (ObservabilityEvent enum) ← T8 (RouterDecision)
                                ← T9 (CostTick)
                                ← T10 (GateEvent)
                                ← T11 (TierConfidence)
  T12 (Prometheus) -- independent, uses GatewayProjection
  T13 (CostSparkline) -- independent, uses GatewayProjection

Phase 3: TUI + Streaming
  T8 ← T14 (RouterTrace widget)
  T9 ← T15 (CostPanel widget)
  T10 ← T16 (GateRow strip)
  T10 ← T17 (SwarmGates widget)
  T14, T15, T16, T17 ← T18 (integrate into App)
  T7 ← T19 (agent heartbeat)
  T7 ← T20 (WebSocket feed)

Phase 4: API Endpoints
  T7 ← T21 (SSE obs endpoint)
  T9, T13 ← T22 (cost dashboard)
  T1 ← T23 (latency profiling)
  T19 ← T24 (agent health)
  T7 ← T25 (StateHub wiring)

Phase 5: Anomaly Detection
  T9 ← T26 (cost spike)
  T10 ← T27 (pass rate drop)
  T23 ← T28 (latency surge)
  T26, T27, T28 ← T29 (wire into pipeline)
  T29 ← T30 (alert rendering)

Phase 6: OTel Export
  T2, T3 ← T31 (OTLP exporter)
  T31 ← T32 (config subcommand)
  T21, T22, T29, T31 ← T33 (integration test)
```

---

## Effort Estimates

| Phase | Tasks | Effort | Cumulative |
|---|---|---|---|
| Phase 1: Tracing + OTel | T1-T6 | 3-4 days | 3-4 days |
| Phase 2: Structured Events | T7-T13 | 3-4 days | 6-8 days |
| Phase 3: TUI + Streaming | T14-T20 | 4-5 days | 10-13 days |
| Phase 4: API Endpoints | T21-T25 | 2-3 days | 12-16 days |
| Phase 5: Anomaly Detection | T26-T30 | 2-3 days | 14-19 days |
| Phase 6: OTel Export | T31-T33 | 2-3 days | 16-22 days |

---

## Success Criteria

### Must Have (Phase 1-3)

- Every `ModelCallService::call()` invocation emits a `gen_ai.*` span with model, tokens, cost, and latency
- TUI dashboard shows RouterTrace, CostPanel, and GateRow widgets with live data
- `JsonlLogger` batches writes (measured: < 5ms persistence overhead per run)
- Prometheus endpoint exports `roko_model_calls_total`, `roko_cost_usd_total`, and `roko_gate_results_total`
- Agent heartbeat and stall detection runs for all supervised agents

### Should Have (Phase 4-5)

- `/api/obs/cost` returns model/role cost breakdown with savings calculation
- `/api/obs/latency` returns per-phase percentile breakdown
- Anomaly detection fires on cost spikes, pass-rate drops, and latency surges
- Anomaly alerts render in TUI error digest and CLI stderr
- WebSocket endpoint streams filtered events

### Nice to Have (Phase 6)

- `cargo run -p roko-cli --features otel -- run "hello"` sends spans to configured OTel endpoint
- `roko config otel set` configures export without env vars
- End-to-end integration test verifies full pipeline

---

## Sources

| Document | Used For |
|---|---|
| `14-GATE-VIZ-07-Dashboard-Integration.md` | TUI architecture, SSE event types, data flow patterns |
| `19-DISPATCH-GOALS.md` | RouterTrace, CostPanel, TierConfidence data feed shapes |
| `18-LEARN-GOALS.md` | CostSparkline, EpisodeScrubber, savings metrics |
| `20-GATE-GOALS.md` | GateRow, SwarmGates, GateEvent enum, threshold updates |
| `13-PERF-BOTTLENECK-ANALYSIS.md` | Bottleneck IDs (B11, B13), measurement methodology, instrumentation points |
| `21-GTM-GATEWAY-ADAPTERS.md` | OTel `gen_ai.*` attributes, vendor landscape, Langfuse guidance |
| `crates/roko-runtime/src/event_bus.rs` | Existing EventBus architecture |
| `crates/roko-runtime/src/jsonl_logger.rs` | Current per-event flush pattern |
| `crates/roko-agent/src/gateway_events.rs` | GatewayEvent and GatewayEventWriter |
| `crates/roko-agent/src/usage.rs` | UsageObservation canonical shape |
| `crates/roko-serve/src/routes/sse.rs` | Existing SSE streaming pattern |
| `crates/roko-serve/src/routes/status/metrics.rs` | Existing Prometheus endpoint |
| `crates/roko-serve/src/events.rs` | ServerEvent enum (15 variants) |
| `crates/roko-core/src/runtime_event.rs` | RuntimeEvent enum (12 variants) |
