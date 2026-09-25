# 20-gateway/01 -- Nine Pipeline Stages

> Detailed stage-by-stage walkthrough of the inference gateway pipeline.
> Each stage maps to exactly one kernel protocol and one source module.

**Parent:** [20-GATEWAY](../../20-GATEWAY.md)

**Source:** `crates/roko-gateway/src/gateway.rs` (assembly), per-stage modules below

---

## 1. Stage Architecture

The gateway pipeline is a linear Graph of nine Cells. Each Cell implements exactly one
kernel protocol, enforcing a clean separation between safety checking (Verify), request
transformation (Compose), external I/O (Connect), caching (Route/Store), and read-only
observation (Observe).

| # | Cell | Protocol | Module | Purpose |
|---|------|----------|--------|---------|
| 1 | LoopDetectCell | **Verify** | `loop_detect.rs` | Reject degenerate agent loops |
| 2 | CacheLookupCell | **Route** | `cache.rs` | Short-circuit on exact/semantic cache hit |
| 3 | ToolPruneCell | **Compose** | `tool_prune.rs` | Remove unused tool schemas |
| 4 | OutputBudgetCell | **Compose** | `output_budget.rs` | Cap `max_tokens` from EMA statistics |
| 5 | ThinkingCapCell | **Compose** | `thinking_cap.rs` | Set default thinking budget by model family |
| 6 | ConvergenceCell | **Verify** | `convergence.rs` | Detect repetitive responses |
| 7 | ProviderCallCell | **Connect** | `provider.rs` | Dispatch to LLM provider with fallback |
| 8 | CacheStoreCell | **Store** | `cache.rs` | Write response to L1 + L2 caches |
| 9 | CostTrackCell | **Observe** | `cost_track.rs` | Compute cost, attribute, deduct budget |

---

## 2. Execution Order

The `PipelineStage::ALL` constant in `gateway.rs` defines the canonical execution order
as an array of 9 enum variants. The pipeline assembly code iterates through stages
sequentially, recording each stage in a per-request trace vector.

```rust
pub const ALL: [PipelineStage; 9] = [
    LoopDetect,
    CacheLookup,
    ToolPrune,
    OutputBudget,
    ThinkingCap,
    ConvergenceDetect,
    ProviderCall,
    CacheStore,
    CostTrack,
];
```

Every request generates a trace, stored in a `HashMap<String, Vec<PipelineStage>>`
keyed by session ID. The `last_trace(session_id)` method exposes this for testing and
diagnostics.

---

## 3. Stage Grouping

The nine stages form three logical groups:

### Pre-dispatch (Stages 1-6)

Stages 1-6 run **before** any provider call. They serve two functions:

1. **Guard stages** (Verify protocol): LoopDetect and ConvergenceDetect check for
   degenerate patterns and inject corrective guidance into the system prompt.

2. **Optimization stages** (Compose + Route protocols): CacheLookup returns cached
   responses without a provider call. ToolPrune, OutputBudget, and ThinkingCap
   reduce the token footprint of the request.

### Dispatch (Stage 7)

Stage 7 is the sole external I/O stage. It acquires a backpressure permit, selects a
provider backend, applies a timeout, and handles fallback through the ranked model list.

### Post-dispatch (Stages 8-9)

Stages 8-9 run **after** a successful provider response:

1. **CacheStore** (Store protocol): Writes the response to both cache layers for future
   hits.

2. **CostTrack** (Observe protocol): Computes cost, records it per agent/session/model,
   deducts from the handle budget, updates the output budgeter's EMA, records output
   tokens for drift detection, and records the response SimHash for convergence
   detection.

---

## 4. Guidance Injection Mechanism

Two stages (LoopDetect and ConvergenceDetect) can inject guidance text into the next
request. Both use the same mechanism:

```rust
impl InferenceRequest {
    pub fn prepend_system_guidance(&mut self, guidance: &str) {
        if let Some(system) = self.messages.iter_mut()
            .find(|m| m.role == MessageRole::System)
        {
            system.content = format!("{guidance}\n\n{}", system.content);
        } else {
            self.messages.insert(0, Message {
                role: MessageRole::System,
                content: guidance.to_string(),
            });
        }
    }
}
```

Guidance is consumed via `take_guidance(session_id)`, which returns `Some(text)` exactly
once and then returns `None` until the next detection event. This prevents guidance from
being repeated on every request.

---

## 5. Cache Route Short-Circuit

When stage 2 (CacheLookup) produces a cache hit, the pipeline registers all remaining
stages in the trace (for observability) but skips their actual logic. The cached response
is returned directly. A `cache_hit` counter increments and a `GatewayEvent` is written
with `provider: "cache"`.

This is a Route protocol behavior: the Cell redirects the request to a cached result
rather than continuing through the pipeline.

---

## 6. Budget Preflight

Between the pre-dispatch stages (1-6) and the provider call (stage 7), the gateway
performs a budget preflight check. It estimates the request cost using:

- Input tokens: `semantic_text().chars().count() / 4`
- Output tokens: `max_tokens` or 2048 default

If the estimated cost exceeds the handle's remaining budget, the request is rejected
with `BudgetExceeded` (HTTP 402) before any provider call occurs.

---

## 7. TOML Graph Definition

The checked-in TOML at `crates/roko-gateway/inference-gateway.toml` defines the
pipeline as a standard Graph:

```toml
[graph]
name = "inference-gateway"
description = "Centralized nine-stage inference gateway pipeline"
version = "1.0.0"

[[nodes]]
id = "loop-detect"
cell_type = "roko:gateway/loop-detect"
[nodes.config]
protocol = "Verify"

# ... 8 more nodes, 8 edges
```

The TOML is loaded at compile time via `include_str!` and verified at test time via
`roko_graph::loader::load_from_str`. The loader test asserts 9 nodes and 8 edges --
any structural drift is caught by CI.

---

## 8. Stage Telemetry

Every stage invocation increments an `AtomicU64` counter in `GatewayCounters::stages`.
The `GatewayStats` struct exposes these as `stage_invocations: HashMap<PipelineStage, u64>`,
available at `GET /api/gateway/stats`.

Additional per-stage telemetry:

| Stage | Stats Struct | Key Metrics |
|-------|-------------|-------------|
| LoopDetect | `LoopStats` | `loops_detected`, `loop_injections`, per-type counters |
| CacheLookup | `CacheStats` | `l1_hits`, `l2_hits`, `misses`, `excluded` |
| ToolPrune | `PruneStats` | `tools_pruned`, `tool_tokens_saved` |
| OutputBudget | `OutputBudgetStats` | `output_budgets_applied`, `output_tokens_bounded` |
| ThinkingCap | `ThinkingCapStats` | `thinking_budgets_applied`, `thinking_tokens_capped_estimate` |
| ConvergenceDetect | `ConvergenceStats` | `convergence_detected`, `convergence_injections` |
| Backpressure | `BackpressureStats` | Per-provider queue depths, global in-flight, rejections |
