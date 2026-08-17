# Solution 2 — Cell/Graph Engine (Build the Platform)

**Philosophy**: The unified spec (`tmp/unified/`, 29 docs) and depth layer (`tmp/unified-depth/`,
155 docs) describe a complete platform where everything — gateway, routing, prompts, tools,
learning, safety — is expressed as Cells in Graphs using 9 Protocols. Build the engine first,
then express the existing subsystems as Cell compositions. This is the "never change it again"
approach.

**Total estimate**: ~120-150 hours
**What it addresses**: Everything. By definition — the spec is designed to be the final form.
**Risk**: High upfront investment before anything visible works. The engine must be correct
before subsystems can be expressed on top of it.

---

## Why Build the Engine

The unified spec isn't aspirational — it's the design that resolves every tension in the
codebase simultaneously:

| Tension | Current | With Cell/Graph engine |
|---|---|---|
| 5 entry points, 5 dispatch paths | Each reimplements dispatch | Each instantiates a Graph |
| Learning bolted on | Separate subsystem, inconsistently wired | Predict-publish-correct structural via Bus |
| Safety as afterthought | Permissive fallback, post-dispatch warnings | Verify Cells in pipeline, fail-closed |
| Knowledge decays forever | RAG appends indefinitely | Demurrage: balance decays unless used |
| God objects (21K LOC) | orchestrate.rs does everything | Max ~500 LOC per Cell, compose via Graph |
| Two execution engines | Runner v2 + PlanRunner coexist | One Graph engine runs everything |
| Remote inference impossible | Tight local coupling | Connect Cells abstract location |

### The key insight from the spec

The 5 primitives (Signal, Pulse, Cell, Graph, Protocol) + 9 protocols create a
**combinatorial multiplier**: adding a new Cell multiplies capabilities across all
existing Graphs. In the current architecture, adding a feature requires wiring it
into each of 5+ entry points independently.

---

## What Gets Built

### Layer 0: Kernel (~25-30h)

#### Signal + Store fabric
```rust
pub struct Signal {
    pub id: ContentHash,           // SHA-256 of payload
    pub kind: Kind,                // Text, Code, Insight, Heuristic, etc.
    pub payload: Value,
    pub score: Score,              // relevance, quality, confidence, novelty, utility
    pub balance: f64,              // demurrage: decays unless actively used
    pub lineage: Vec<SignalRef>,
    pub hdc_fingerprint: HdcVector,
}

pub trait Store: Send + Sync {
    async fn put(&self, signal: Signal) -> Result<ContentHash>;
    async fn get(&self, id: &ContentHash) -> Result<Option<Signal>>;
    async fn query(&self, predicate: &Query) -> Result<Vec<Signal>>;
    async fn query_similar(&self, fingerprint: &HdcVector, k: usize) -> Result<Vec<Signal>>;
    async fn prune(&self, below_balance: f64) -> Result<usize>;
}
```

This replaces: `.roko/episodes.jsonl`, `.roko/signals.jsonl`, knowledge store, playbook
store, cascade-router.json — all become Signal Store operations.

#### Pulse + Bus fabric
```rust
pub struct Pulse {
    pub seq: u64,
    pub topic: String,             // "agent:{id}.{phase}", "prediction.{op}"
    pub ts: Instant,
    pub payload: Value,
}

pub trait Bus: Send + Sync {
    fn publish(&self, pulse: Pulse);
    fn subscribe(&self, topic_pattern: &str) -> PulseStream;
}
```

This replaces: EventBus, StateHub, RuntimeEvents, TuiBridge — all become Bus pub/sub.

#### Cell trait + Graph engine
```rust
pub trait Cell: Send + Sync + 'static {
    fn id(&self) -> CellId;
    fn protocols(&self) -> &[ProtocolId];
    fn capabilities(&self) -> &Capabilities;
    async fn execute(&self, input: Signal, ctx: &CellContext) -> Result<Signal>;
}

pub struct Graph {
    pub nodes: Vec<Node>,          // Cell instantiations
    pub edges: Vec<Edge>,          // typed connections with routing conditions
    pub entry: Vec<NodeId>,
    pub exit: Vec<NodeId>,
}

// Graphs are Cells (fractal composition)
impl Cell for Graph { ... }
```

This replaces: PlanRunner DAG executor, runner/event_loop.rs, orchestrate.rs task dispatch
— all become Graph execution.

### Layer 1: Protocol Cells (~20-25h)

Implement the 9 protocols as Cell implementations:

| Protocol | Key Cells | Replaces |
|---|---|---|
| Store | FileStore, NeuroStore, EpisodeStore | `.roko/` file I/O scattered across codebase |
| Score | LlmScorer, RuleScorer, HdcScorer | Scoring in orchestrate.rs |
| Verify | CompileGate, TestGate, ClippyGate, DiffGate, LlmJudge | roko-gate pipeline |
| Route | CascadeRouter, CostRouter, RuleRouter | CascadeRouter (currently dead code) |
| Compose | PromptComposer, VcgComposer, GreedyComposer | PromptAssemblyService |
| React | SafetyReactor, BudgetReactor, CalibrationPolicy | Safety layer, budget enforcement |
| Observe | AgentLens, GateLens, CostLens, HealthLens | StateHub projections |
| Connect | AnthropicProvider, OpenAIProvider, ClaudeCliProvider, McpBridge | roko-agent backends |
| Trigger | CronTrigger, WebhookTrigger, FileWatchTrigger, BusTrigger | Event subscriptions |

### Layer 2: Compositions (~25-30h)

Express the application as Graphs of Cells:

#### Inference Gateway Pipeline (9 Cells)
```toml
[[cells]]
id = "loop_detect"
protocol = "Verify"

[[cells]]
id = "cache_lookup"
protocol = "Route"

[[cells]]
id = "tool_prune"
protocol = "Compose"

[[cells]]
id = "output_budget"
protocol = "Compose"

[[cells]]
id = "provider_call"
protocol = "Connect"

[[cells]]
id = "cache_store"
protocol = "Store"

[[cells]]
id = "cost_track"
protocol = "Observe"

[[edges]]
from = "loop_detect"
to = "cache_lookup"
```

#### Agent Cognitive Loop (Hot Graph)
```toml
[graph]
kind = "hot_flow"
subscribe_topic = "heartbeat.gamma.tick"

[[cells]]
id = "sense"
protocol = "Store"
description = "Read observations, drain Bus topics"

[[cells]]
id = "assess"
protocol = "Score"
description = "Score candidates, select tier"

[[cells]]
id = "compose"
protocol = "Compose"
description = "Assemble context under budget (VCG)"

[[cells]]
id = "act"
protocol = "Connect"
description = "Call LLM via gateway pipeline"

[[cells]]
id = "verify"
protocol = "Verify"
description = "Check output against gates"

[[cells]]
id = "persist"
protocol = "Store"
description = "Write episode, update knowledge"

[[cells]]
id = "react"
protocol = "React"
description = "Update routing, trigger replan if needed"
```

#### Session as Space
```toml
[space]
name = "chat_session"
bus_partition = "session:{id}"
store_partition = ".roko/sessions/{id}/"

[space.extensions]
layers = ["perception", "memory", "cognition", "action", "meta"]

[space.memory]
demurrage_rate = 0.01
tier_multipliers = { transient = 0.1, working = 0.5, consolidated = 1.0, persistent = 5.0 }
```

### Layer 3: Surfaces (~15-20h)

Wire the Graphs to user-facing entry points:

| Entry point | What it instantiates |
|---|---|
| `roko` (chat) | Session Space + Cognitive Loop (hot) |
| `roko "prompt"` | Session Space + single Cognitive Loop iteration |
| `roko run` | Session Space + single iteration with gates |
| `roko plan run` | Session Space per-task + Plan Graph (DAG of tasks) |
| ACP | Session Space + FSM pipeline Graph (express/standard/full) |
| `roko serve` | Gateway Pipeline + HTTP routes as Connect Cells |
| Dashboard/TUI | Lens Cells subscribing to Bus topics |

---

## How This Addresses the Subsystem Audits

### gateway audit
- Gateway = Graph of 9 Cells → ✅
- Routing = Route Cell (CascadeRouter) → ✅
- Caching = Store Cell (L1/L2/L3) → ✅
- Streaming = Connect Cell with async tap → ✅
- Remote = Connect Cell abstracts location → ✅
- Learning = React Cell (CalibrationPolicy) → ✅

### inference-dispatch audit
- 13 call sites → all go through Connect Cell → ✅
- 4 duplicate parsers → one Cell implementation → ✅
- CascadeRouter called → Route Cell in gateway → ✅
- Episode logging → Store Cell (EpisodeStore) → ✅
- Credential centralization → Connect Cell owns keys → ✅

### prompt-assembly audit
- 9-layer builder → Compose Cell (PromptComposer) → ✅
- VCG auction → Compose Cell variant → ✅
- All entry points use it → Compose Cell in every Graph → ✅
- Section effectiveness → React Cell (CalibrationPolicy) → ✅

### ux audit
- Board/Epic/Task → Graph Cells with Store protocol → ✅
- Streaming to surfaces → Lens Cells on Bus → ✅
- TrackerAdapter → Connect Cells (GitHub, Linear, Sentry) → ✅
- Five surfaces → all subscribe to same Bus topics → ✅

### acp-protocol audit
- FSM pipeline → Graph (express/standard/full templates) → ✅
- Learning → React Cells in pipeline → ✅
- Safety → Verify Cells in pipeline → ✅
- Multi-backend → Connect Cells via gateway → ✅
- Session persistence → Space with Store partition → ✅

---

## The Predict-Publish-Correct Pattern

This is the single most valuable design pattern from the unified spec. Every learning loop
follows the same pattern:

1. **Predict**: Cell makes decision based on current belief
2. **Publish**: Action outcome as Pulse on Bus topic `outcome.{op}`
3. **Correct**: CalibrationPolicy Cell joins prediction + outcome, updates belief

Applied to:
- **Routing**: predict model quality → observe gate result → update CascadeRouter
- **Gate thresholds**: predict pass rate → observe actual → update EMA
- **Composition**: predict section effectiveness → observe task success → update bidder weights
- **Cost**: predict cost → observe actual → update pricing model

Currently each of these is a separate, inconsistently-wired subsystem. With the engine,
they're all instances of the same Loop pattern — write once, apply everywhere.

---

## The Demurrage Pattern

Knowledge that isn't used decays. This solves the RAG accumulation problem:

- **Facts**: 50% decay in 20h
- **Procedures**: 50% decay in 5d
- **Heuristics**: 50% decay in 30d

Reinforcement (retrieval, citation, gate-pass) restores balance. Novelty-weighted to prevent
popular-but-mediocre knowledge from dominating.

This replaces: manual episode compaction (S8.1), unbounded JSONL growth (EP1), stale
knowledge in neuro store. The Store fabric handles it uniformly.

---

## Migration Path

The engine doesn't require rewriting everything at once. Migration is progressive:

### Step 1: Build kernel (Signal, Pulse, Cell, Graph, Store, Bus)
- New crate: `roko-engine`
- Pure abstractions, no existing code depends on it yet

### Step 2: Wrap existing crates as Cells
- `CompileGate` Cell wraps `roko-gate::compile`
- `CascadeRouter` Cell wraps `roko-learn::cascade_router`
- `PromptComposer` Cell wraps `roko-compose::prompt_assembly_service`
- `AnthropicProvider` Cell wraps `roko-agent::anthropic_backend`
- Each wrapper is ~50-100 LOC

### Step 3: Compose Graphs
- Gateway Pipeline Graph
- Cognitive Loop Graph
- Plan Execution Graph (DAG of task Graphs)
- ACP Pipeline Graph (express/standard/full)

### Step 4: Wire entry points
- `roko` instantiates Session Space + Cognitive Loop
- `roko serve` instantiates Gateway Pipeline
- `roko plan run` instantiates Plan Execution Graph
- `roko acp` instantiates ACP Pipeline Graph

### Step 5: Remove wrappers
- Once Graphs are working, migrate Cell implementations from wrappers to native
- Delete `orchestrate.rs` (21K LOC), `dispatch_direct.rs`, legacy chat paths
- Delete dead PlanRunner code

---

## Trade-offs

### Advantages
- **Never change it again**: The Cell/Graph model is the final form from the spec
- **Combinatorial value**: New Cells multiply across all Graphs
- **Learning is structural**: Predict-publish-correct is topology, not wiring
- **Remote-native**: Connect Cells abstract location transparently
- **Testable**: Each Cell is independently testable; Graphs are testable as compositions
- **Observable**: Lens Cells provide uniform observability

### Disadvantages
- **~120-150h before visible results**: Engine must be correct first
- **Abstraction overhead**: Simple things (like "call an API") go through Cell/Graph machinery
- **Spec is ambitious**: 155 depth docs describe a system that doesn't fully exist yet
- **Risk of over-engineering**: Building an engine before the use cases are settled
- **Team onboarding**: Contributors must understand Cell/Graph/Protocol vocabulary

### Mitigation
- Build kernel as a thin layer (~2K LOC), not a framework
- Wrap existing code as Cells immediately (don't rewrite)
- Ship each Graph as it's composed (don't wait for everything)
- Use the engine for new features; migrate old features progressively

---

## Comparison to Solution 1

| Dimension | Solution 1 (Triad) | Solution 2 (Engine) |
|---|---|---|
| Time to first visible result | ~25h (Phase 1 gateway) | ~50h (kernel + first Graph) |
| Total effort | ~70-90h | ~120-150h |
| Architecture durability | Good (conventional services) | Excellent (spec's final form) |
| Remote inference | Yes (gateway as HTTP proxy) | Yes (Connect Cells abstract location) |
| Learning integration | Manual wiring per subsystem | Structural (predict-publish-correct) |
| New feature cost | Wire into 3 services | Add a Cell, compose into Graph |
| Testability | Service-level integration tests | Cell-level unit + Graph-level integration |
| Spec alignment | Partial (services, not Cells) | Full (implements unified spec) |
