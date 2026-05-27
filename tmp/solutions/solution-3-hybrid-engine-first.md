# Solution 3 — Hybrid: Thin Engine + Immediate Services (Recommended)

**Philosophy**: Build a thin Cell/Graph engine (~2K LOC), then immediately express the
three services (Gateway, Session, PromptAssembly) as Graphs of Cells. You get the
architectural durability of Solution 2 with the time-to-visible-result of Solution 1.
The trick: the engine is small enough to build in ~15h, and wrapping existing code as
Cells is mechanical.

**Total estimate**: ~80-100 hours
**What it addresses**: Everything from all 6 subsystem audits + aligned with unified spec.
**Risk**: Medium — engine must be correct but is deliberately minimal.

---

## Why This Is the Right Answer

The user's constraints:
1. "I want the ideal best version" → needs the unified spec's architecture
2. "So I don't have to touch or change things again" → needs Cell/Graph composability
3. "What if I want to use a remote inference gateway?" → needs Connect protocol
4. "Get this to where it is as effective as mori" → needs to work, not just be designed
5. "Consider the newer designs in unified/unified-depth" → needs to implement the spec

Solution 1 (Service Triad) satisfies 3-4 but not 1-2. Solution 2 (Full Engine) satisfies
1-2 but takes too long for 4. Solution 3 threads the needle.

**The insight**: The unified engine doesn't need to be big. The kernel is ~2K LOC:
- `Signal` struct + `Store` trait: ~200 LOC
- `Pulse` struct + `Bus` trait: ~150 LOC
- `Cell` trait + `CellContext`: ~100 LOC
- `Graph` struct + executor: ~400 LOC
- 9 protocol traits: ~200 LOC
- Pipeline/Loop/Space patterns: ~400 LOC
- Config/TOML graph loading: ~300 LOC
- Error types + utilities: ~200 LOC

That's the whole engine. Everything else is Cells (which wrap existing crate code) and
Graphs (which compose Cells via TOML).

---

## Phase 0: Security Hardening (3h)

Same as all solutions. Non-negotiable.

- Auth enabled by default
- Terminal routes behind auth
- CORS restricted to localhost
- Private gists, LogScrubber on payloads
- PTY session limits
- `dangerously_skip_permissions` removed

---

## Phase 1: Engine Kernel (`roko-engine`) (12-15h)

### What gets built

A new crate: `roko-engine/` (~2K LOC)

```
roko-engine/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── signal.rs          // Signal struct, ContentHash, Kind, Score, demurrage
    ├── pulse.rs           // Pulse struct, topic parsing
    ├── store.rs           // Store trait (put/get/query/prune)
    ├── bus.rs             // Bus trait (publish/subscribe), ring buffer impl
    ├── cell.rs            // Cell trait, CellContext, Capabilities
    ├── graph.rs           // Graph struct, Edge, Node, executor (topo-sort + parallel)
    ├── protocol.rs        // 9 protocol trait definitions
    ├── patterns/
    │   ├── pipeline.rs    // Linear Graph of Cells with early exit
    │   ├── loop_pattern.rs // Graph with feedback edge + convergence
    │   └── space.rs       // Bus partition + Store partition + membership
    ├── config.rs          // TOML graph loading
    └── error.rs
```

### Design constraints

- **No async runtime**: Engine is runtime-agnostic. Cells are `async fn`, executor is generic.
- **No framework**: Engine is a library. Callers compose, engine doesn't dictate.
- **Wrap, don't port**: Existing crate code stays. Cells are thin wrappers (~50-100 LOC each).
- **TOML-defined Graphs**: Graphs are declarative, not code. Easy to modify, version, diff.

### The Graph executor

```rust
pub async fn execute_graph(
    graph: &Graph,
    input: Signal,
    ctx: &GraphContext,
) -> Result<Signal> {
    // 1. Topological sort of nodes
    let order = topo_sort(&graph.nodes, &graph.edges);

    // 2. Execute in dependency order, parallelizing independent nodes
    let mut results: HashMap<NodeId, Signal> = HashMap::new();

    for batch in order.parallel_batches() {
        let futures = batch.iter().map(|node| {
            let cell = ctx.resolve_cell(node.cell_id)?;
            let input = gather_inputs(node, &graph.edges, &results)?;
            cell.execute(input, &ctx.cell_context(node))
        });

        let batch_results = join_all(futures).await;
        for (node, result) in batch.iter().zip(batch_results) {
            results.insert(node.id, result?);
        }
    }

    // 3. Gather exit node outputs
    gather_outputs(&graph.exit, &results)
}
```

This replaces:
- PlanRunner DAG executor in orchestrate.rs
- runner/event_loop.rs task scheduling
- ACP FSM pipeline

With one Graph executor that handles all of them.

---

## Phase 2: Wrap Existing Crates as Cells (10-12h)

Each Cell is a thin wrapper (~50-100 LOC) around existing, working code:

### Verify Cells (wrap roko-gate)
```rust
pub struct CompileGateCell;
impl Cell for CompileGateCell {
    async fn execute(&self, input: Signal, ctx: &CellContext) -> Result<Signal> {
        let workspace = ctx.workspace_root();
        let result = roko_gate::compile::check(workspace).await?;
        Ok(Signal::verdict(result.passed, result.diagnostics))
    }
}
```

Similarly: `TestGateCell`, `ClippyGateCell`, `DiffGateCell`, `LlmJudgeCell`

### Route Cells (wrap roko-learn)
```rust
pub struct CascadeRouterCell {
    router: Arc<CascadeRouter>,
}
impl Cell for CascadeRouterCell {
    async fn execute(&self, input: Signal, ctx: &CellContext) -> Result<Signal> {
        let routing_ctx = RoutingContext::from_signal(&input);
        let decision = self.router.select_model(&routing_ctx);
        Ok(Signal::routing_decision(decision))
    }
}
```

### Compose Cells (wrap roko-compose)
```rust
pub struct PromptComposerCell;
impl Cell for PromptComposerCell {
    async fn execute(&self, input: Signal, ctx: &CellContext) -> Result<Signal> {
        let task_context = TaskContext::from_signal(&input);
        let assembly = ctx.prompt_assembly_service()
            .assemble(&task_context)
            .await?;
        Ok(Signal::system_prompt(assembly.prompt, assembly.manifest))
    }
}
```

### Connect Cells (wrap roko-agent backends)
```rust
pub struct AnthropicProviderCell {
    client: reqwest::Client,
    config: ProviderConfig,
}
impl Cell for AnthropicProviderCell {
    async fn execute(&self, input: Signal, ctx: &CellContext) -> Result<Signal> {
        let request = ProxyRequest::from_signal(&input);
        let response = self.forward_anthropic(&request).await?;
        // Publish cost Pulse on Bus
        ctx.bus().publish(Pulse::cost(response.usage));
        Ok(Signal::model_response(response))
    }
}
```

### React Cells (new, ~100 LOC each)
```rust
pub struct CalibrationPolicyCell;
impl Cell for CalibrationPolicyCell {
    async fn execute(&self, input: Signal, ctx: &CellContext) -> Result<Signal> {
        // Join prediction + outcome
        let prediction = input.get_prediction()?;
        let outcome = input.get_outcome()?;
        let error = prediction.error_from(&outcome);

        // Update belief (this is the "correct" step)
        ctx.store().update_score(prediction.id, error)?;

        // Publish calibration Pulse
        ctx.bus().publish(Pulse::calibration(prediction.id, error));

        Ok(Signal::calibration_result(error))
    }
}
```

### Store Cells (wrap existing persistence)
```rust
pub struct EpisodeStoreCell;
impl Cell for EpisodeStoreCell {
    async fn execute(&self, input: Signal, ctx: &CellContext) -> Result<Signal> {
        let episode = Episode::from_signal(&input);
        ctx.store().put(Signal::episode(episode)).await?;

        // Auto-compaction (demurrage handles this naturally)
        if ctx.store().count("episode").await? > 200 {
            ctx.store().prune(0.05).await?; // prune below 5% balance
        }

        Ok(Signal::ack())
    }
}
```

### Observe Cells (Lenses — wrap StateHub)
```rust
pub struct CostLensCell;
impl Cell for CostLensCell {
    async fn execute(&self, input: Signal, _ctx: &CellContext) -> Result<Signal> {
        // Read-only projection
        let cost_snapshot = CostSnapshot::from_signal(&input);
        Ok(Signal::observation(cost_snapshot))
    }
}
```

---

## Phase 3: Gateway as Graph (12-15h)

The InferenceGateway is now a Graph of Cells, not a monolithic service:

```toml
# roko-gateway/gateway-pipeline.toml
[graph]
id = "inference_gateway"
name = "Inference Gateway Pipeline"

[[nodes]]
id = "format_detect"
cell = "FormatDetectCell"

[[nodes]]
id = "cache_lookup"
cell = "CacheLookupCell"

[[nodes]]
id = "route"
cell = "CascadeRouterCell"

[[nodes]]
id = "tool_prune"
cell = "ToolPruneCell"

[[nodes]]
id = "output_budget"
cell = "OutputBudgetCell"

[[nodes]]
id = "provider_call"
cell = "ProviderCallCell"

[[nodes]]
id = "cache_store"
cell = "CacheStoreCell"

[[nodes]]
id = "cost_track"
cell = "CostTrackCell"

[[edges]]
from = "format_detect"
to = "cache_lookup"

[[edges]]
from = "cache_lookup"
to = "route"
condition = "!cache_hit"

# ... linear pipeline
```

### Streaming

The `ProviderCallCell` produces a stream of Pulses on the Bus:
```
topic: "gateway.{request_id}.token_delta"
topic: "gateway.{request_id}.tool_call"
topic: "gateway.{request_id}.complete"
```

Consumers (chat UI, ACP, TUI) subscribe to these topics. Streaming is not a
special case — it's just Bus pub/sub.

### Remote deployment

The gateway Graph runs inside an HTTP server (axum). Requests come in as HTTP,
get converted to Signals, flow through the Graph, responses come back as HTTP.

```rust
// Embedded mode: Graph runs in-process
let gateway = GraphEngine::load("gateway-pipeline.toml")?;
let response = gateway.execute(request_signal).await?;

// Remote mode: HTTP call to standalone gateway
let response = reqwest::Client::new()
    .post("https://gateway.myteam.dev/v1/messages")
    .json(&request)
    .send().await?;
```

Same Gateway Graph, two deployment modes.

### What this replaces

- `dispatch_direct.rs` (500 LOC) → deleted
- 4 copies of stream-json parser → one Cell
- Hardcoded model/URL/version → config in Graph TOML
- CascadeRouter dead code → Route Cell in pipeline
- Per-request `reqwest::Client::new()` → shared client in ProviderCallCell

---

## Phase 4: Session as Space (15-18h)

A Session is a Space (Bus partition + Store partition) with an Agent cognitive loop:

### Session Space
```toml
[space]
id = "session_{uuid}"
bus_partition = "session.{id}"
store_partition = ".roko/sessions/{id}/"

[space.config]
model = "auto"                    # CascadeRouter selects
effort = "medium"
mode = "code"
gates = ["compile", "test", "clippy"]
```

### Cognitive Loop (Hot Graph)
```toml
[graph]
id = "cognitive_loop"
kind = "hot_flow"

[[nodes]]
id = "receive_input"
cell = "InputCell"

[[nodes]]
id = "assemble_prompt"
cell = "PromptComposerCell"

[[nodes]]
id = "call_model"
cell = "GatewayForwardCell"    # sends to gateway Graph (local or remote)

[[nodes]]
id = "handle_tools"
cell = "ToolDispatchCell"

[[nodes]]
id = "verify_output"
cell = "SafetyVerifyCell"

[[nodes]]
id = "record_episode"
cell = "EpisodeStoreCell"

[[nodes]]
id = "update_history"
cell = "HistoryCell"

# Tool loop: if tool_use, dispatch tool and re-send
[[edges]]
from = "call_model"
to = "handle_tools"
condition = "has_tool_use"

[[edges]]
from = "handle_tools"
to = "call_model"            # feedback edge (Loop pattern)

# Normal path
[[edges]]
from = "call_model"
to = "verify_output"
condition = "!has_tool_use"
```

### What the Session Space owns

| Component | Cell type | What it does |
|---|---|---|
| Conversation history | HistoryCell (Store) | Accumulates messages, context-windows |
| Tool registry | ToolRegistryCell (Store) | Builtin + MCP + workspace tools |
| Workspace context | WorkspaceCell (Observe) | Path, git branch, file listing |
| Prompt assembly | PromptComposerCell (Compose) | 9-layer builder, VCG auction |
| Gateway handle | GatewayForwardCell (Connect) | Channel to gateway (local or remote) |
| Feedback | EpisodeStoreCell (Store) | Episodes, efficiency, routing events |
| Cost tracking | CostLensCell (Observe) | Per-session cumulative cost |

### How entry points use it

```rust
// roko (chat)
let session = Space::new("chat", config)?;
let loop_graph = Graph::load("cognitive_loop.toml")?;
session.run_hot(loop_graph).await?; // persistent cognitive loop

// roko "fix the bug" (oneshot)
let session = Space::new("oneshot", config)?;
let loop_graph = Graph::load("cognitive_loop.toml")?;
session.run_once(loop_graph, input_signal).await?; // single iteration

// roko plan run (plan execution)
let session = Space::new("plan", config)?;
let plan_graph = Graph::load_plan("plans/tasks.toml")?; // DAG of task nodes
for task in plan_graph.topological_order() {
    session.run_once(cognitive_loop, task_signal).await?;
}

// ACP (editor integration)
let session = Space::new("acp", config)?;
let pipeline = Graph::load(match complexity {
    Low => "acp_express.toml",
    Medium => "acp_standard.toml",
    High => "acp_full.toml",
})?;
session.run_hot(pipeline).await?;
```

### Slash commands mutate the Space

```rust
// /system "You are a pirate"
session.store().put(Signal::config("system_prompt", "You are a pirate"))?;
// Next PromptComposerCell iteration reads from Store → takes effect

// /model claude-opus-4-6
session.store().put(Signal::config("model", "claude-opus-4-6"))?;
// Next CascadeRouterCell reads from Store → model pinned

// /effort max
session.store().put(Signal::config("effort", "max"))?;
// Next OutputBudgetCell reads from Store → tokens increased
```

Commands mutate Store. Cells read Store. Changes take effect on next iteration.
No special wiring needed per command.

---

## Phase 5: Learning as Loops (8-10h)

Every learning loop is an instance of the Loop pattern:

### Routing Loop
```toml
[graph]
id = "routing_loop"
kind = "loop"

[[nodes]]
id = "predict"
cell = "CascadeRouterCell"        # predicts best model

[[nodes]]
id = "observe"
cell = "GateResultObserverCell"   # observes gate pass/fail

[[nodes]]
id = "correct"
cell = "CalibrationPolicyCell"    # updates router weights

[[edges]]
from = "predict"
to = "observe"

[[edges]]
from = "observe"
to = "correct"

[[edges]]
from = "correct"
to = "predict"                    # feedback edge
```

### Gate Threshold Loop
```toml
# Same pattern, different Cells
predict = "AdaptiveThresholdCell"
observe = "GateVerdictCell"
correct = "ThresholdCalibrationCell"
```

### Composition Effectiveness Loop
```toml
predict = "PromptComposerCell"        # predicts section value
observe = "TaskSuccessObserverCell"   # observes task outcome
correct = "SectionEffectivenessCell"  # updates bidder weights
```

**Three learning loops, one pattern.** Currently these are three separate, inconsistently-wired
subsystems. With the engine, they're three instances of the same Loop TOML.

### LinUCB weight persistence (S8.3)

The `CascadeRouterCell` uses the Store protocol:
```rust
// On shutdown
store.put(Signal::routing_weights(self.router.serialize_weights())).await?;

// On startup
let weights = store.query(Query::kind("routing_weights").latest()).await?;
self.router.restore_weights(weights.payload)?;
```

Store handles persistence. Demurrage handles cleanup. No special code needed.

---

## Phase 6: Cleanup & Delete (5-8h)

With Graphs handling everything, delete the dead and duplicate code:

| File | LOC | Reason |
|---|---|---|
| `dispatch_direct.rs` | 500 | Replaced by ProviderCallCell |
| `orchestrate.rs` (PlanRunner parts) | ~15K of 21K | Replaced by Plan Graph |
| `chat.rs` (legacy) | 658 | Replaced by Session Space |
| 4 copies of stream-json parser | ~600 | Replaced by one Cell |
| `runner/event_loop.rs` scheduling | ~500 | Replaced by Graph executor |

Estimated deletion: **~17K LOC**. Net new code: ~5-6K LOC (engine + cells + graphs).

---

## Full Audit Cross-Reference

### Binary Issues (MASTER-INDEX S1-S11)

| S# | How it's solved | Phase |
|---|---|---|
| S1 (thin pipe) | Session Space + Cognitive Loop Graph | 4 |
| S2 (throwaway clients) | Shared client in ProviderCallCell | 3 |
| S3 (commands lie) | Commands mutate Store, Cells read Store | 4 |
| S4 (error swallowing) | Cell execute returns Result, Graph propagates | 1 |
| S5 (security) | Security hardening | 0 |
| S6 (no streaming) | Bus pub/sub for all streaming | 3-4 |
| S7 (hardcoded values) | Config in Graph TOML | 3 |
| S8 (phantom features) | Cells wrap existing code, Graphs wire them | 2-5 |
| S9 (subprocess management) | Cell lifecycle + Graph shutdown | 1-2 |
| S10 (duplicate code) | One Graph executor, delete duplicates | 6 |
| S11 (mutex/unwrap) | Cell isolation, no shared mutable state | 1 |

### Subsystem Audits

| Audit | Status | How |
|---|---|---|
| gateway | ✅ | Gateway Pipeline Graph (Phase 3) |
| inference-dispatch | ✅ | ProviderCallCell + CascadeRouterCell (Phase 2-3) |
| prompt-assembly | ✅ | PromptComposerCell in every Graph (Phase 2) |
| ux | ✅ | Lens Cells on Bus for all surfaces (Phase 4) |
| acp-protocol | ✅ | ACP Pipeline Graph (express/standard/full) (Phase 4) |

### Mori-Diffs (12 GAP clusters)

| GAP | Status | How |
|---|---|---|
| GAP-01 (dispatch parity) | ✅ | All dispatch through Gateway Graph |
| GAP-02 (streaming) | ✅ | Bus pub/sub |
| GAP-03 (tool system) | ✅ | ToolDispatchCell in Cognitive Loop |
| GAP-04 (session state) | ✅ | Session Space |
| GAP-05 (cost tracking) | ✅ | CostTrackCell in Gateway |
| GAP-06 (two runtimes) | ✅ | One Graph executor |
| GAP-07 (permissions) | ✅ | Security hardening + SafetyVerifyCell |
| GAP-08 (knowledge) | ✅ | NeuroStore Cell in Cognitive Loop |
| GAP-09 (learning) | ✅ | Learning Loops (Phase 5) |
| GAP-10 (config) | ✅ | Graph TOML config |
| GAP-11 (security) | ✅ | Phase 0 |
| GAP-12 (observability) | ✅ | Lens Cells on Bus |

---

## Why This Is the Recommended Approach

1. **Engine is tiny** (~2K LOC) — not a framework, just traits + executor
2. **Cells wrap existing code** — no rewrite, just ~50-100 LOC wrappers
3. **Graphs are TOML** — declarative, versionable, diffable, modifiable without recompilation
4. **Remote-native** — Connect Cells abstract location; gateway deploys anywhere
5. **Learning is structural** — Loop pattern, not per-subsystem wiring
6. **~17K LOC deleted, ~5-6K LOC added** — net reduction in complexity
7. **Aligned with unified spec** — this IS the spec's architecture, just built incrementally
8. **Every phase ships** — Phase 0 (security), Phase 3 (gateway works), Phase 4 (chat works)

### Time comparison

| Approach | Hours | Mori parity? | Spec alignment? | Remote gateway? |
|---|---|---|---|---|
| Solution 1 (Service Triad) | 70-90h | ~80% | Partial | Yes |
| Solution 2 (Full Engine) | 120-150h | ~95% | Full | Yes |
| **Solution 3 (Hybrid)** | **80-100h** | **~90%** | **Full** | **Yes** |
| Previous Solution C | 45-55h | ~60% | None | No |

Solution 3 gets ~90% of Solution 2's architectural quality at ~65% of the cost, and
ships visible results at Phase 3 (~30h in) vs Phase 3 of Solution 2 (~75h in).
