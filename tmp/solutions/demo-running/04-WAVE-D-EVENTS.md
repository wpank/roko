# Wave D: Event Coverage — Extend RuntimeEvent + SseAdapter

## Root Cause

The frontend types.ts expects 50+ event types. The backend emits ~20, and only from route
handlers that do inline operations (bench, jobs). The most important source — plan execution
— emits events that never reach SSE clients.

## Fundamental Design Decision (from audit)

The original plan proposed building a `DashboardEvent -> ServerEvent` bridge in StateHub.
**This is wrong.** It inverts the dependency direction and creates unnecessary conversion
layers. StateHub's current crate boundary is also transitional; taskrunner task 104 tracks
moving it out of path-included compatibility wiring.

The CORRECT approach, which already partially exists:

```
RuntimeEvent (roko-core) — THE canonical event type
  │
  ├── SseAdapter (roko-serve/src/adapters.rs)
  │     Already converts RuntimeEvent → SSE JSON format
  │     Just needs more match arms for new variants
  │
  ├── DashboardEventBridge (roko-serve/src/lib.rs:476)
  │     Already converts RuntimeEvent → DashboardEvent for TUI snapshots
  │     Just needs more match arms
  │
  └── JsonlLogger
        Logs to .roko/events.jsonl
```

**What we do**: Extend `RuntimeEvent` with new variants, then extend `SseAdapter`'s match.
**What we DON'T do**: Build a bridge between DashboardEvent and ServerEvent. Eliminate
`ServerEvent` entirely — it's redundant with RuntimeEvent + SseAdapter.

---

## Task D1: Extend RuntimeEvent with High-Value Variants

**Root cause**: RuntimeEvent currently has 12 variants. The demo needs ~20 for rich panels.

**Current variants** (from `crates/roko-core/src/runtime_event.rs`):
- PlanStarted, PlanCompleted
- TaskStarted, TaskCompleted
- AgentSpawned, AgentOutput
- GateResult
- PhaseTransition
- EfficiencyEvent
- Error
- (2 others)

**New variants to add**:

```rust
// In crates/roko-core/src/runtime_event.rs
pub enum RuntimeEvent {
    // ... existing variants ...

    // Inference tracking (critical for cost panels)
    InferenceStarted {
        request_id: String,
        model: String,
        agent_id: String,
        auto_routed: bool,
    },
    InferenceCompleted {
        request_id: String,
        model: String,
        agent_id: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
    },
    InferenceFailed {
        request_id: String,
        model: String,
        agent_id: String,
        error: String,
    },

    // Agent traces (rich debugging)
    AgentTrace {
        agent_id: String,
        turn: u32,
        tool_calls: Vec<ToolCallSummary>,
        reasoning: Option<String>,
        usage: TokenUsage,
    },

    // Lifecycle events
    TaskFailed {
        plan_id: String,
        task_id: String,
        error: String,
        gate_failure: bool,
    },
    RunStarted {
        run_id: String,
        prompt: String,
        complexity: String,
    },
    RunCompleted {
        run_id: String,
        success: bool,
        cost_usd: f64,
        duration_ms: u64,
    },

    // Knowledge events (for memory scenario)
    KnowledgeIngested {
        entry_id: String,
        topic: String,
        source_agent: String,
    },
    KnowledgeConsumed {
        entry_id: String,
        topic: String,
        consuming_agent: String,
    },
}
```

**Verification**: `cargo build --workspace` passes. Unit tests for serialization of all variants.

---

## Task D2: Extend SseAdapter for All Variants

**Root cause**: `SseAdapter` in `crates/roko-serve/src/adapters.rs` matches on RuntimeEvent
variants and produces SSE JSON. It needs match arms for all new D1 variants.

**What already exists** (from audit):
- `SseAdapter` implementing `EventConsumer` trait
- Match arms for existing 12 variants
- SSE format: `event: {type}\ndata: {json}\n\n`

**What to do**:
1. Add match arms for all new D1 variants
2. Each arm produces snake_case event type + JSON data
3. Ensure serialization format matches frontend expectations

```rust
// In SseAdapter's consume() method:
RuntimeEvent::InferenceCompleted { request_id, model, agent_id, input_tokens, output_tokens, cost_usd, duration_ms } => {
    Some(SseEvent {
        event_type: "inference_completed".into(),
        data: json!({
            "request_id": request_id,
            "model": model,
            "agent_id": agent_id,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "cost_usd": cost_usd,
            "duration_ms": duration_ms,
        }),
    })
}
// ... similarly for all new variants
```

**IMPORTANT — Event naming**: The backend emits snake_case (`inference_completed`).
Frontend must subscribe with matching casing. Verify types.ts alignment.

**Verification**:
```bash
cargo run -p roko-cli -- serve &
curl -N http://127.0.0.1:6677/api/events/stream
# Trigger operations, confirm new event types appear in SSE output
```

---

## Task D3: Inference Tracking Emissions

**Root cause**: LLM API calls (the most expensive operation) have zero observability.
InferenceStarted/Completed are defined (D1) but never emitted.

**Design**: Add `InferenceObserver` trait in roko-agent:

```rust
// crates/roko-agent/src/observer.rs
pub trait InferenceObserver: Send + Sync {
    fn on_start(&self, request_id: &str, model: &str, agent_id: &str, auto_routed: bool);
    fn on_complete(&self, request_id: &str, model: &str, agent_id: &str,
                   input_tokens: u64, output_tokens: u64, cost_usd: f64, duration_ms: u64);
    fn on_error(&self, request_id: &str, model: &str, agent_id: &str, error: &str);
}
```

**Wiring** — in dispatcher's LLM call path:
```rust
// In dispatcher/mod.rs, around the backend.call():
observer.on_start(&request_id, &model, &agent_id, auto_routed);
let start = Instant::now();
let result = self.backend.call(&request).await;
match result {
    Ok(response) => {
        observer.on_complete(&request_id, &model, &agent_id,
            response.input_tokens, response.output_tokens,
            response.cost_usd, start.elapsed().as_millis() as u64);
    }
    Err(e) => {
        observer.on_error(&request_id, &model, &agent_id, &e.to_string());
    }
}
```

The concrete implementation (in roko-cli) emits `RuntimeEvent::InferenceStarted/Completed/Failed`
which flows through the EventConsumer pipeline to SSE automatically.

**Verification**:
```bash
curl -N http://127.0.0.1:6677/api/events/stream | grep inference
# Should show inference_started and inference_completed for every LLM call
```

---

## Task D4: AgentTrace Per-Turn Events

**Root cause**: Per-turn agent state (which tools called, reasoning, usage) has zero
observability. `AgentTrace` is now a RuntimeEvent variant but never emitted.

**Where to emit**: In the agent's tool loop, alongside `AgentEvent::TurnCompleted`:

```rust
// After each turn in the agent tool loop:
let trace = RuntimeEvent::AgentTrace {
    agent_id: agent_id.clone(),
    turn: current_turn,
    tool_calls: turn_tool_calls.iter().map(|tc| ToolCallSummary {
        name: tc.name.clone(),
        result_preview: tc.result.chars().take(200).collect(),
    }).collect(),
    reasoning: thinking_content.clone(),
    usage: TokenUsage { input: turn_input, output: turn_output },
};
event_consumer.consume(trace);
```

**Design note**: AgentTrace is separate from AgentOutput. Output = content the agent
produces (text, code). Trace = meta-information (what tools, why, what it cost).

**Verification**: During plan run, SSE shows `agent_trace` events between `agent_output`
events, with tool_calls populated.

---

## Task D5: Remaining High-Value Emissions

These events already have RuntimeEvent variants (from D1). Each just needs an emit site:

| Event | Where to Emit | Trigger |
|-------|--------------|---------|
| TaskFailed | v2 runner, on gate failure or agent error | Gate pipeline returns failure |
| RunStarted/RunCompleted | `roko do` / WorkflowEngine::run() | Run lifecycle |
| KnowledgeIngested | KnowledgeIngestionSink | Knowledge entry added to neuro store |
| KnowledgeConsumed | Knowledge query path | Knowledge entry retrieved for agent |

Each follows the same pattern:
1. Find the code location where the state change happens
2. Add `event_consumer.consume(RuntimeEvent::...)` call
3. SseAdapter already has the match arm (from D2)

**Verification**: For each event individually:
1. Trigger the specific condition
2. Confirm the event appears in `curl -N http://127.0.0.1:6677/api/events/stream`

---

## ServerEvent Elimination (Future Follow-Up)

`ServerEvent` (55 variants in `crates/roko-serve/src/events.rs`) is now redundant.
RuntimeEvent + SseAdapter achieves the same thing without an extra type layer.

**NOT in scope for this wave**: Actually deleting ServerEvent. That's a separate cleanup.
For now, we build the correct architecture (RuntimeEvent → SseAdapter) and let ServerEvent
exist unused. It will be cleaned up when no route handlers reference it.

---

## Dependency Graph

```
D1 (extend RuntimeEvent) → D2 (extend SseAdapter) → D3 (inference emissions)
                                                   → D4 (agent trace emissions)
                                                   → D5 (remaining emissions)
```

D1 and D2 are sequential (D2 needs the variants D1 defines). D3, D4, D5 are parallel
(they just emit events that D2 already handles).

---

## Anti-Patterns to Avoid

1. **Do NOT bridge DashboardEvent → ServerEvent.** That was the old plan. The correct
   approach: RuntimeEvent → EventConsumer pipeline (SseAdapter, DashboardBridge).

2. **Do NOT emit the same event from two places.** If the runner emits RuntimeEvent::TaskStarted,
   and DashboardBridge converts it to DashboardEvent::TaskStarted for TUI, that's fine.
   But don't ALSO manually emit a separate event elsewhere — causes duplicates.

3. **Do NOT add ServerEvent variants.** RuntimeEvent is the canonical type. If you need
   a new event, add it to RuntimeEvent and extend SseAdapter.

4. **Do NOT emit events in tests then claim the feature works.** Verification means
   triggering from a real operation and observing in SSE stream.

---

## What This Wave Achieves

After Wave D, the SSE stream shows:
- Real-time plan execution progress (task start/complete/fail)
- Agent spawning, output, and per-turn traces
- Inference start/complete with token counts and costs
- Gate results as they happen
- Phase transitions
- Knowledge flow (ingested/consumed)
- Run lifecycle (started/completed)
- Errors with context

This is what the demo frontend needs for live-updating sidebar panels.
