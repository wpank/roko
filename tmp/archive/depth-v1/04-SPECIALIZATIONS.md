# 04 — Specializations

> Ten patterns built on the three fundamentals. Conventions, not new primitives.

---

## 1. Overview

Every specialization is a well-known configuration of Signal, Block, and Graph. None introduces a new fundamental type. A developer who understands the three fundamentals and nine protocols can derive any specialization.

| # | Specialization | One-liner | Built from |
|---|---|---|---|
| 1 | **Flow** | Graph at runtime | Graph + RunId + snapshot + events |
| 2 | **Rack** | Graph with knobs and jacks | Graph + Macros + Slots |
| 3 | **Trigger** | Block that fires Graphs | Block + Trigger protocol |
| 4 | **Lens** | Block that observes without modifying | Block + Observe protocol |
| 5 | **Loop** | Graph that feeds back | Graph + self-referential edge |
| 6 | **Memory** | Store with decay and dreams | Block + Store protocol + decay model |
| 7 | **Space** | Isolation boundary | Graph + capability grants |
| 8 | **Extension** | Block that intercepts | Block + interception metadata + layer |
| 9 | **Agent** | Space + Extensions + Memory + clock | Space + Extension[] + Memory + AdaptiveClock |
| 10 | **Connector** | Block with external I/O lifecycle | Block + Connect protocol + lifecycle |

---

## 2. Flow

A **Flow** is a Graph at runtime. It has a RunId, produces snapshots, emits lifecycle events, and can be paused/resumed/cancelled.

```rust
pub struct Flow {
    pub run_id: RunId,
    pub graph: ResolvedGraph,          // pinned Block versions
    pub input: Value,                  // validated against graph.schema.input
    pub macros: MacroBindings,         // resolved (if Graph is a Rack)
    pub slots: SlotBindings,           // resolved (if Graph is a Rack)
    pub trigger: Option<TriggerRef>,   // what started this Flow
    pub policy: GraphPolicy,           // overrides graph.policy if set
    pub resume_from: Option<RunSnapshot>, // for --resume
}
```

### Lifecycle

```
Created → Running → Completed | Failed | Cancelled
                ↘ Paused (human input, budget) → Resumed → Running
```

### Snapshots

Every node completion produces a checkpoint. Flows can be resumed from any checkpoint:

```bash
roko run <graph> --resume <run-id>
```

In-flight Blocks at snapshot time are restarted (Blocks must be idempotent or carry their own checkpointing).

Full execution semantics in [doc-05 (Execution Engine)](05-EXECUTION-ENGINE.md).

---

## 3. Rack

A **Rack** is a Graph with **Macros** (promoted parameters — knobs) and **Slots** (typed empty positions — jacks) exposed to consumers.

The name comes from modular synthesis: a rack holds modules and exposes macro knobs and patch jacks to the performer, hiding internal wiring.

### Macros (knobs)

```rust
pub struct MacroDef {
    pub name: String,
    pub label: String,                 // shown in UI
    pub description: String,
    pub kind: MacroKind,
    pub default: Value,
    pub bindings: Vec<MacroBinding>,   // which internal Block params it sets
}

pub struct MacroBinding {
    pub target_node: NodeId,
    pub target_param: String,          // dotted path into Block params
    pub transform: Option<Expr>,       // optional value transformation
}
```

A single Macro can fan out across multiple internal Blocks. Setting `macro.strictness = "high"` might bind to `auditor.threshold = 0.9`, `synthesizer.temperature = 0.3`, and `reviewer.iterations = 3` simultaneously.

```rust
pub enum MacroKind {
    Boolean,
    Enum { variants: Vec<String> },
    Integer { min: i64, max: i64, step: i64 },
    Float { min: f64, max: f64, step: f64 },
    Text { pattern: Option<String> },
    Money { currency: String, max: f64 },
    ModelRef,
    AgentRef,
    SlotRef,                           // the Macro IS the Slot's filling
}
```

### Slots (jacks)

```rust
pub struct SlotDef {
    pub name: String,
    pub label: String,
    pub description: String,
    pub accepts: SlotKind,
    pub input_schema: TypeSchema,
    pub output_schema: TypeSchema,
    pub default_filling: Option<SlotFilling>,
    pub required: bool,
}

pub enum SlotKind {
    AnyBlock,
    AnyGraph,
    AnyVerification,                   // verification Rack producing Verdict
    SpecificTag { tag: String },
    Capability { capability: Capability },
}

pub enum SlotFilling {
    Block { block: BlockRef, params: Value },
    Graph { graph: GraphRef, params: Value },
    Inline { graph: Graph },           // ad-hoc fill
}
```

Slots are the composability hinge. A `research-pipeline` Rack has slots for "Researcher" and "Verifier" — consumers plug in any Block whose types match, without forking the parent.

### TOML authoring

```toml
[graph]
name = "code-review"
version = "1.0.0"

[[macros]]
name = "strictness"
label = "Review Strictness"
kind = { type = "enum", variants = ["low", "medium", "high"] }
default = "medium"

[[macros.bindings]]
target_node = "linter"
target_param = "threshold"
transform = "strictness == 'high' ? 0.9 : strictness == 'medium' ? 0.7 : 0.5"

[[slots]]
name = "reviewer"
label = "Code Reviewer"
accepts = "any-block"
input_schema = { type = "object", fields = { code = "string", language = "string" } }
output_schema = { type = "object", fields = { findings = { type = "array", items = "Finding" } } }
required = true
```

---

## 4. Trigger

A **Trigger** is a Block implementing the Trigger protocol. It listens for events and fires Graphs.

```rust
pub struct TriggerBinding {
    pub trigger: TriggerRef,           // which Trigger Block
    pub graph: GraphRef,               // which Graph to fire
    pub input_map: Vec<Mapping>,       // map trigger event → Graph input
    pub filter: Option<Expr>,          // optional filter on trigger events
    pub concurrency: ConcurrencyPolicy,
    pub enabled: bool,
}

pub enum ConcurrencyPolicy {
    Queue,                             // new runs queue behind running ones
    Skip,                              // skip if already running
    CancelRunning,                     // cancel running, start new
    Parallel { max: u32 },             // allow N concurrent runs
}
```

### Built-in Trigger kinds

| Kind | Fires when |
|---|---|
| Cron | Schedule expression matches (`0 */5 * * *`) |
| Webhook | HTTP request arrives at registered path |
| FileWatch | File/directory changes detected |
| Bus | Specific Signal kind appears on a Bus topic |
| ChainEvent | Smart contract event emitted on-chain |
| Manual | User invokes via CLI, TUI, or dashboard |
| SignalPattern | HDC-similar Signal appears above threshold |

Full trigger system spec in [doc-06 (Trigger System)](06-TRIGGER-SYSTEM.md).

---

## 5. Lens

A **Lens** is a Block implementing the Observe protocol. It receives read-only ObservableEvents and emits observation Signals onto the Bus. Lenses never modify what they observe.

### Observable events

```rust
pub enum ObservableEvent {
    // Signal lifecycle
    SignalCreated(Signal),
    SignalScored(SignalRef, ScoreResult),
    SignalRouted(SignalRef, RouteResult),
    SignalVerified(SignalRef, Verdict),
    SignalComposed(Vec<SignalRef>, Signal),

    // Block lifecycle
    BlockStarted { block: BlockRef, run: RunId },
    BlockCompleted { block: BlockRef, run: RunId, duration: Duration, cost: Cost },
    BlockFailed { block: BlockRef, run: RunId, error: BlockError },

    // Graph lifecycle
    GraphStarted { graph: GraphRef, run: RunId },
    GraphCompleted { graph: GraphRef, run: RunId },
    GraphFailed { graph: GraphRef, run: RunId },

    // Agent lifecycle
    AgentTick { agent: AgentRef, regime: Regime, prediction_error: f64 },
    MemoryEvent { kind: MemoryEventKind },
    TriggerFired { trigger: TriggerRef, graph: GraphRef },
}
```

### Lens composition

- **Stacking**: Multiple Lenses observe the same target (cost + latency + quality simultaneously)
- **Chaining**: A Lens observes another Lens's output (TrendLens watches CostLens)
- **Scoping**: Lenses attach at Block, Graph, Agent, or Space level

### Built-in Lenses

| Lens | Emits | Scope |
|---|---|---|
| CostLens | CostReport Signals per interval | Block / Graph / Agent |
| LatencyLens | p50/p95/p99 Signals | Block / Graph |
| QualityLens | Pass rate from Verify Blocks | Graph |
| EfficiencyLens | Tokens-per-task ratio | Agent |
| ErrorLens | Classified error report Signals | Block / Graph / Agent |
| DriftLens | Knowledge quality degradation Signals | Memory |
| BudgetLens | Threshold alert Signals | Agent / Space |
| TrendLens | Slope, EMA, derivative Signals | Any other Lens |
| AnomalyLens | Statistical outlier alert Signals | Any other Lens |
| UsageLens | Install/run/fork count Signals | Space / Marketplace |

Full Lens system spec in [doc-09 (Telemetry)](09-TELEMETRY.md).

---

## 6. Loop

A **Loop** is a Graph that feeds output back to input. The output of the Graph's exit node routes back to its entry node through a feedback edge.

```toml
[graph]
name = "adaptive-gate-threshold"
loop = true                           # marks this Graph as a Loop

[[nodes]]
id = "observe"
type = "block"
block = "gate-outcome-collector@^1.0"

[[nodes]]
id = "update"
type = "block"
block = "ema-threshold-updater@^1.0"

[[edges]]
from = "observe"
to = "update"

# Feedback edge: output → input
[[edges]]
from = "update"
to = "observe"
condition = "NOT converged"
```

### Timescales

Loops operate at different timescales:

| Loop | Timescale | Period | Example |
|---|---|---|---|
| Parameter tuning | Gamma | Per-tick | Temperature adjustment, gate threshold EMA |
| Strategy routing | Theta | Per-task | Model selection, failure strategy |
| Knowledge consolidation | Delta | Per-session | Dream cycle (NREM → REM → Integration) |
| Structural adaptation | Manual | Per-approval | Gate pipeline changes, Graph revisions |

Full learning loop spec in [doc-10 (Learning Loops)](10-LEARNING-LOOPS.md).

---

## 7. Memory

A **Memory** is a Store-protocol Block with decay, tier progression, dream consolidation, and HDC-based retrieval.

```rust
pub struct MemoryConfig {
    pub store_path: PathBuf,
    pub max_entries: usize,
    pub default_half_life: Duration,
    pub tier_config: TierConfig,
    pub anti_knowledge: AntiKnowledgeConfig,
    pub dream_config: DreamConfig,
}
```

Memory Blocks manage the knowledge lifecycle:

1. **Ingest**: New Signals enter at Transient tier
2. **Retrieve**: HDC similarity search + scoring (40% HDC, 30% keyword, 20% utility, 10% freshness, +15% cross-domain)
3. **Decay**: Ebbinghaus curve with tier multipliers
4. **Promote/Demote**: Based on validation (gate passes/failures)
5. **Consolidate**: Dream cycles (NREM replay → REM imagination → Integration)
6. **Prune**: Below 1% threshold → cold storage

Full knowledge system spec in [doc-11 (Memory and Knowledge)](11-MEMORY-AND-KNOWLEDGE.md).

---

## 8. Space

A **Space** is a Graph isolation boundary with capability grants. Every execution happens within a Space. Spaces control what Blocks can do.

```rust
pub struct Space {
    pub id: SpaceId,
    pub name: String,
    pub grants: Vec<CapabilityGrant>,
    pub config: SpaceConfig,
    pub store: StoreRef,               // default Store for this Space
    pub bus: BusRef,                   // default Bus for this Space
}

pub struct CapabilityGrant {
    pub capability: Capability,
    pub granted_to: GrantScope,        // all Blocks, specific Blocks, specific Graphs
    pub granted_by: String,            // user who authorized
    pub expires: Option<DateTime<Utc>>,
}
```

### Capability intersection

A Block may run only when all three layers permit:

```
Block declaration ∩ Graph allow-list ∩ Space grant = effective capabilities
```

Missing at any layer = denied. The system fails closed.

---

## 9. Extension

An **Extension** is a Block that intercepts another Block's pipeline. Extensions modify behavior through hooks, organized into 8 layers.

```rust
pub struct ExtensionManifest {
    pub name: String,
    pub layer: ExtensionLayer,
    pub depends_on: Vec<String>,       // other Extensions this one requires
    pub optional: bool,                // agent continues if this fails to load
}
```

### 8 Layers

| Layer | # | Hooks | Purpose |
|---|---|---|---|
| Foundation | L0 | `on_init`, `on_shutdown` | Lifecycle setup/teardown |
| Perception | L1 | `on_observe`, `filter_input` | Input filtering and observation |
| Memory | L2 | `on_retrieve`, `on_store` | Knowledge access interception |
| Cognition | L3 | `pre_inference`, `post_inference`, `on_gate` | LLM call modification |
| Action | L4 | `pre_action`, `post_action`, `on_tool_call` | Tool/action interception |
| Social | L5 | `on_message_send`, `on_message_receive` | Communication interception |
| Meta | L6 | `on_reflect`, `on_cost_update` | Self-monitoring |
| Recovery | L7 | `on_error`, `on_budget_exceeded` | Error handling |

Extensions fire in layer order (L0 → L7). Within a layer, in config order. Dependencies within a layer are topologically sorted.

**Fault isolation**: If one Extension hook errors, the runtime logs and continues to the next. A buggy optional Extension cannot take down the Agent.

Full extension system spec in [doc-08 (Extension System)](08-EXTENSION-SYSTEM.md).

---

## 10. Agent

An **Agent** is the most complex specialization: Space + Extensions + Memory + adaptive clock. Every agent runs the same core loop.

```rust
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub profile: DomainProfile,        // user-defined string (e.g., "coding", "research")
    pub mode: AgentMode,               // Ephemeral | Persistent | Reactive
    pub space: Space,                  // isolation boundary
    pub extensions: Vec<Extension>,    // interceptor chain
    pub memory: Memory,                // knowledge store with decay
    pub clock: AdaptiveClock,          // tick frequency control
    pub pipeline: NineStepGraph,       // the 9-step pipeline as a Graph
    pub cortical: CorticalState,       // working memory, goals, beliefs, attention
}
```

### 9-step pipeline

The Agent's internal pipeline is a Graph with 9 nodes:

```
1. Observe  → Read inbox, check triggers, scan environment
2. Retrieve → Query Memory, load relevant context
3. Analyze  → Score observations, compute prediction error
4. Gate     → T0/T1/T2 decision (PE threshold)
5. Simulate → Generate candidate actions, evaluate outcomes
6. Validate → Safety checks, capability verification, budget guard
7. Execute  → Dispatch action (LLM call, tool use, message)
8. Verify   → Check result against predictions
9. Reflect  → Update cortical state, log episode, adjust clock
```

### Three modes

| Mode | Behavior | Use case |
|---|---|---|
| Ephemeral | Runs until task completes, then stops | Coding tasks, one-off research |
| Persistent | Runs tick loop indefinitely | Chain monitoring, CI watchers |
| Reactive | Sleeps until trigger fires, works, sleeps | PR reviewer, scheduled jobs |

### Adaptive clock

Three timescales with regime-based adjustment:

| Timescale | Frequency | Purpose |
|---|---|---|
| Gamma | 100ms – 2s | Fast perception, heartbeat |
| Theta | 750ms – 16s | Reflective planning |
| Delta | 60s – 10m | Deep consolidation |

Regimes: Calm (4×), Normal (1×), Volatile (0.5×), Crisis (0.25×). 3-tick hysteresis prevents oscillation.

### T0/T1/T2 gating

| Tier | Condition | Cost | Action |
|---|---|---|---|
| T0 (reflex) | PE < 0.15, no urgency | ~0 tokens | Execute cached reflex rule |
| T1 (reflective) | PE 0.15–0.40 | ~500 tokens | Lightweight model (Haiku) |
| T2 (deliberate) | PE > 0.40 or novel | ~2000–8000 tokens | Full model (Sonnet/Opus) |
| Sleepwalk | Budget exhausted | 0 tokens | Observe + reflect only |

Full agent runtime spec in [doc-07 (Agent Runtime)](07-AGENT-RUNTIME.md).

---

## 11. Connector

A **Connector** is a Block implementing the Connect protocol with lifecycle management. Connectors wrap external system I/O behind a universal interface.

```rust
pub enum ConnectorKind {
    ChainRpc,       // Ethereum, Solana
    Exchange,       // Hyperliquid, Binance
    McpServer,      // MCP tool servers
    Database,       // Postgres, SQLite
    Webhook,        // External HTTP endpoints
    Api,            // Generic REST/gRPC
}
```

### Discovery

Connectors are discovered from three sources:
1. **Config**: `connectors = ["postgres", "hyperliquid"]` in agent config
2. **MCP auto-register**: MCP servers in `agent.mcp_config` auto-register as McpConnectors
3. **Extension-provided**: Extensions can register Connectors in their `on_init()` hook

### Distinction from Extension

Extensions modify agent behavior through hooks (intercept, filter, transform). Connectors provide bidirectional I/O with external systems. An agent *loads* Extensions but *uses* Connectors. An Extension can *wrap* a Connector to add retry logic or rate limiting.

Full connectivity spec in [doc-12 (Connectivity)](12-CONNECTIVITY.md).

---

## 12. Summary Table

| Specialization | Protocols Used | TOML Defined? | New Type? |
|---|---|---|---|
| Flow | (runtime concept) | N/A | `Flow` struct wraps `Graph` |
| Rack | (structural convention) | Yes | No (Graph + Macros + Slots) |
| Trigger | Trigger | Yes | No (Block + Trigger protocol) |
| Lens | Observe | Yes | No (Block + Observe protocol) |
| Loop | (structural convention) | Yes | No (Graph + feedback edge) |
| Memory | Store | Yes | No (Block + Store + decay) |
| Space | (isolation boundary) | Yes | `Space` struct |
| Extension | (interception) | Yes (manifest) | `ExtensionManifest` |
| Agent | All (via pipeline) | Yes | `Agent` struct |
| Connector | Connect | Yes | No (Block + Connect protocol) |

No specialization requires understanding beyond the three fundamentals and nine protocols. Every one is a discoverable pattern, documented here and in its detailed spec.
