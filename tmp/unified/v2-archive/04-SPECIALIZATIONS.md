# 04 — Specializations

> 10 patterns built on the 3 fundamentals (Signal, Cell, Graph) and 9 protocols. Specializations are conventions, not new primitives. Every specialization composes from what already exists. No special machinery.

**Design Principle 7**: Elegance through composition. An Agent is Space + Extensions + Memory + clock. A dream cycle is a Loop. Adaptive thresholds are a Loop. The cascade router is a Loop. 36 old concepts collapsed to 12 without losing expressiveness.

---

## 1. Overview

A Specialization is a named pattern that constrains how Cells, Graphs, and Signals are used together. Specializations introduce **no new kernel types** — they are conventions enforced by validation and runtime behavior, built from the same Cells, Graphs, Signals, and Pulses defined in [doc-01](01-SIGNAL.md), [doc-02](02-CELL.md), and [doc-03](03-GRAPH.md).

The purpose of naming them is discoverability: developers learn 10 patterns, not 100 ad-hoc compositions.

---

## 2. Flow

A **Flow** is a Graph at runtime: a Graph definition + a `RunId` + execution state + snapshots. Every Graph execution creates a Flow. Flows are the unit of execution managed by the engine ([doc-05](05-EXECUTION-ENGINE.md)).

```rust
pub struct Flow {
    /// Unique run identity.
    pub run_id: RunId,

    /// The Graph being executed.
    pub graph: Graph,

    /// Current lifecycle state.
    pub state: FlowState,

    /// Per-node execution state.
    pub node_states: BTreeMap<NodeId, NodeState>,

    /// Snapshot for resumability.
    pub snapshot: Option<FlowSnapshot>,

    /// Cost accounting.
    pub cost: CostLedger,

    /// Start time.
    pub started_at: DateTime<Utc>,

    /// Agent context (if running inside an Agent).
    pub agent: Option<AgentId>,
}

pub enum FlowState {
    Created,
    Running,
    Paused { reason: String },
    Completed { outputs: Vec<Signal> },
    Failed { error: CellError },
    Cancelled { reason: String },
}
```

**Protocols used**: none directly (Flow is a runtime wrapper, not a Cell).

### Hot Flow variant

A **Hot Flow** is a Flow whose underlying Graph has `policy.hot = true` and a `ClockBinding`. Instead of running once and completing, a Hot Flow stays resident in memory, re-firing on each tick of its bound clock. Between ticks, node outputs are retained. State persists in memory (flushed to disk on Agent teardown or periodic checkpoint).

Hot Flows power Agent processing pipelines (the 9-step pipeline in [doc-03](03-GRAPH.md) S8) and any long-lived reactive computation.

```toml
# A Hot Flow defined in TOML
[graph]
name = "market-monitor"
version = "0.1.0"
hot = true
clock = { kind = "custom", period_ms = 5000, name = "market-tick" }
```

---

## 3. Rack

A **Rack** is a Graph with **Macros** (knobs) and **Slots** (jacks). Inspired by modular synthesis: a Rack is a preconfigured board where the performer adjusts parameters (Macros) and patches cables (Slots) without redesigning the circuit.

```rust
pub struct Rack {
    /// The underlying Graph.
    pub graph: Graph,

    /// Named parameters exposed to the user. Changing a Macro
    /// adjusts Cell params without editing the Graph.
    pub macros: Vec<RackMacro>,

    /// Late-bound Slots that must be filled before execution.
    pub slots: Vec<RackSlot>,
}

pub struct RackMacro {
    /// Human-readable name (shown in UI).
    pub name: String,

    /// Description of what this knob controls.
    pub description: String,

    /// The parameter path(s) this Macro affects.
    /// Example: "nodes.score.params.model" maps to the `model` param
    /// of the `score` node.
    pub targets: Vec<MacroTarget>,

    /// Default value.
    pub default: Value,

    /// Allowed range or enum of values.
    pub constraint: MacroConstraint,
}

pub struct MacroTarget {
    pub node_id: NodeId,
    pub param_path: String,
}

pub enum MacroConstraint {
    Range { min: f64, max: f64 },
    Enum(Vec<Value>),
    FreeText,
    Boolean,
}

pub struct RackSlot {
    pub name: String,
    pub node_id: NodeId,
    pub expected_schema: Option<TypeSchema>,
    pub expected_protocols: Vec<ProtocolId>,
    pub description: String,
}
```

```toml
# A Rack with macros and slots
[rack]
name = "research-and-review"
version = "0.1.0"
graph = "plans/research-review.toml"

[[rack.macros]]
name = "model"
description = "LLM model for research and review"
default = "claude-sonnet-4-20250514"
constraint = { kind = "enum", values = ["claude-sonnet-4-20250514", "claude-opus-4-20250514"] }

[[rack.macros.targets]]
node_id = "research"
param_path = "model"

[[rack.macros.targets]]
node_id = "review"
param_path = "model"

[[rack.slots]]
name = "custom-gate"
node_id = "verification"
expected_protocols = ["verify"]
description = "Plug in your verification gate"
```

**Protocols used**: inherits from the underlying Graph's Cells.

---

## 4. Trigger

A **Trigger** is a Cell conforming to the Trigger protocol ([doc-02](02-CELL.md) S2.9) that listens for events and fires Graphs. Fully specified in [doc-06](06-TRIGGER-SYSTEM.md).

```rust
pub struct TriggerSpec {
    pub name: String,
    pub kind: TriggerKind,
    pub graph: GraphRef,
    pub binding: TriggerBinding,
    pub concurrency: ConcurrencyPolicy,
    pub filter: Option<TriggerFilter>,
}
```

**Protocols used**: Trigger.

---

## 5. Lens

A **Lens** is a Cell conforming to the Observe protocol ([doc-02](02-CELL.md) S2.7). Lenses are **read-only**: they observe but never mutate. Stacking Lenses gives different views of the same data.

```rust
pub struct Lens {
    /// The underlying Observe Cell.
    pub block: Arc<dyn ObserveProtocol>,

    /// What this Lens observes.
    pub focus: LensFocus,

    /// Output projections for different surfaces.
    pub projections: Vec<Projection>,
}

pub enum LensFocus {
    /// Observe a specific Agent's state.
    Agent(AgentId),

    /// Observe a specific Flow's execution.
    Flow(RunId),

    /// Observe the entire Bus.
    Bus { filter: TopicFilter },

    /// Observe Store contents.
    Store { query: StoreQuery },

    /// Observe system-wide metrics.
    System,
}

/// A projection maps observation Signals to a specific surface format.
pub struct Projection {
    pub surface: SurfaceName,
    pub transform: ProjectionTransform,
}

pub enum SurfaceName {
    Tui,
    Web,
    Slack,
    Audit,
    Api,
    Custom(String),
}
```

### Key Lenses

| Lens | What it observes | Surface |
|---|---|---|
| `CostLens` | Real-time cost telemetry per Cell, Flow, Agent | TUI, API |
| `CollectiveIntelligenceLens` | c-factor (Woolley et al. 2010): turn-taking entropy, peer prediction accuracy, citation reciprocity | TUI, API |
| `VitalityLens` | Per-Agent vitality, behavioral phase | TUI, API |
| `GateLens` | Gate verdict stream with pass rates, trends | TUI, API |
| `BusLens` | Topic throughput, backpressure, missed Pulses | TUI, API |

**Protocols used**: Observe.

---

## 6. Loop

A **Loop** is a Graph with a feedback edge — output from exit nodes feeds back to entry nodes. All learning loops in Roko are Loops:

| Loop | What it learns | Feedback Signal |
|---|---|---|
| **T0 reflex loop** | Pattern-match reflexes (no LLM needed for ~80% of ticks) | Gate verdicts on reflex accuracy |
| **T1/T2 cascade loop** | Model routing (which model for which task) | Reward from Verify verdicts via EFE update |
| **Gate threshold loop** | Adaptive pass/fail thresholds per gate rung | EMA of gate scores |
| **Dream consolidation loop** | Knowledge distillation during low-activity periods | Surprise delta pre/post consolidation |
| **Heuristic calibration loop** | Brier scores on heuristic predictions | Episode outcomes matched to predictions |

```rust
/// A Loop is just a Graph with `policy.hot = true` and a feedback edge
/// from an exit node back to an entry node. No new type required.
///
/// The Loop node kind in Graph (doc-03 S2) encapsulates this pattern
/// with an explicit `condition` and `max_iterations`.
```

```toml
# Gate threshold learning loop
[graph]
name = "gate-threshold-loop"
version = "0.1.0"

[[nodes]]
id = "observe-verdicts"
label = "Observe recent gate verdicts"
kind = "block"
block = "builtin://gate-verdict-observer"

[[nodes]]
id = "compute-ema"
label = "Compute EMA of pass rates"
kind = "block"
block = "builtin://ema-computer"
[nodes.params]
alpha = 0.1

[[nodes]]
id = "update-thresholds"
label = "Update gate thresholds"
kind = "block"
block = "builtin://threshold-updater"

[[nodes]]
id = "check-convergence"
label = "Has the threshold stabilized?"
kind = "loop"
body = "inline"
condition = "abs(payload.delta) < 0.001"
max_iterations = 100

[[edges]]
from = "observe-verdicts"
to = "compute-ema"

[[edges]]
from = "compute-ema"
to = "update-thresholds"

[[edges]]
from = "update-thresholds"
to = "check-convergence"
```

**Protocols used**: inherits from contained Cells.

---

## 7. Memory

A **Memory** is a Store Cell with demurrage ([doc-01](01-SIGNAL.md) S6) and dream consolidation. Memory is the knowledge specialization: it manages Signals that decay unless actively used, promoting valuable knowledge and pruning stale entries.

```rust
pub struct Memory {
    /// The underlying Store Cell.
    pub store: Arc<dyn StoreProtocol>,

    /// Demurrage configuration for this Memory.
    pub demurrage_config: DemurrageConfig,

    /// Dream schedule (when to run consolidation).
    pub dream_schedule: Option<DreamSchedule>,

    /// AntiKnowledge thresholds.
    pub anti_knowledge: AntiKnowledgeConfig,
}

pub struct DemurrageConfig {
    /// Per-Kind rate overrides (see doc-01 S6).
    pub kind_rates: BTreeMap<Kind, DemurrageRate>,

    /// Default flat tax per day.
    pub default_flat_tax: f64,

    /// Default exponential decay rate per day.
    pub default_exp_decay: f64,

    /// Cold threshold (below this balance, archive to cold storage).
    pub cold_threshold: f64,

    /// Reinforcement bonuses per interaction kind.
    pub bonuses: BTreeMap<ReinforceKind, f64>,
}

pub struct DemurrageRate {
    pub flat_tax: f64,
    pub exp_decay: f64,
}

pub struct DreamSchedule {
    /// Trigger condition for dream consolidation.
    pub trigger: DreamTrigger,

    /// Maximum duration for a dream cycle.
    pub max_duration: Duration,
}

pub enum DreamTrigger {
    /// Run when agent activity drops below threshold.
    LowActivity { idle_seconds: u64 },

    /// Run on a cron schedule.
    Cron(String),

    /// Run after N new episodes.
    EpisodeCount(u32),

    /// Manual trigger only.
    Manual,
}
```

**Protocols used**: Store (directly), React (subscribes to Bus for reinforcement events).

---

## 8. Space

A **Space** is an isolation boundary with capability grants. Spaces separate agents, teams, and workspaces. Every Agent runs inside a Space; Spaces can be nested for hierarchical isolation.

```rust
pub struct Space {
    pub id: SpaceId,
    pub name: String,

    /// Capability grants for this Space.
    /// Cells running inside this Space have their capabilities
    /// intersected with these grants (three-layer intersection,
    /// doc-02 S3.2).
    pub grants: CapabilitySet,

    /// Store scoped to this Space.
    pub store: Arc<dyn StoreProtocol>,

    /// Bus scoped to this Space (Pulses don't leak across Spaces
    /// unless explicitly bridged).
    pub bus: Arc<dyn Bus>,

    /// Parent Space (for nesting).
    pub parent: Option<SpaceId>,

    /// Child Spaces.
    pub children: Vec<SpaceId>,

    /// Cross-space knowledge sharing policy.
    pub sharing: SharingPolicy,
}

pub enum SharingPolicy {
    /// No knowledge crosses the Space boundary.
    Isolated,

    /// Read-only access to parent Space's Store.
    ReadParent,

    /// Bidirectional sharing with specific Spaces.
    SharedWith {
        spaces: Vec<SpaceId>,
        filter: Option<StoreQuery>,
    },

    /// Full mesh sharing (all Spaces can read each other's Store).
    Open,
}
```

**Protocols used**: Store (scoped), Bus (scoped).

---

## 9. Extension

An **Extension** is a Cell that intercepts another Cell's pipeline. Extensions provide 8 layers with 22 hooks for modifying behavior without changing the intercepted Cell's code. CaMeL IFC (capability-tagged information flow control) prevents capability laundering through extensions.

```rust
pub struct Extension {
    /// The interceptor Cell.
    pub block: Arc<dyn Cell>,

    /// Which layer this Extension operates at.
    pub layer: ExtensionLayer,

    /// Which hooks this Extension implements.
    pub hooks: Vec<ExtensionHook>,

    /// Priority (lower = runs first). Extensions at the same layer
    /// execute in priority order.
    pub priority: u32,

    /// CaMeL IFC tags: what information flow this Extension is
    /// permitted to perform.
    pub camel_tags: CamelTags,
}

pub enum ExtensionLayer {
    /// L0: Signal preprocessing (before Cell sees input).
    Input,
    /// L1: Prompt assembly (for LLM-backed Cells).
    Prompt,
    /// L2: Model selection override.
    Routing,
    /// L3: Tool filtering (restrict available tools).
    Tools,
    /// L4: Output postprocessing (after Cell produces output).
    Output,
    /// L5: Verification amendment (add criteria to Verify).
    Verify,
    /// L6: Cost accounting override.
    Cost,
    /// L7: Lifecycle events (start, stop, error).
    Lifecycle,
}

/// CaMeL information flow control tags.
/// Prevents capability laundering: an Extension cannot grant
/// capabilities beyond what its own CaMeL tags permit.
pub struct CamelTags {
    /// What information this Extension may read.
    pub read: BTreeSet<InformationLabel>,

    /// What information this Extension may write.
    pub write: BTreeSet<InformationLabel>,

    /// What information this Extension may declassify.
    pub declassify: BTreeSet<InformationLabel>,
}

pub enum InformationLabel {
    Public,
    AgentInternal,
    UserData,
    Secret,
    SafetyCritical,
    Custom(String),
}
```

Full extension system specification is in [doc-08](08-EXTENSION-SYSTEM.md).

**Protocols used**: any (Extensions wrap other Cells' protocols).

---

## 10. Agent

An **Agent** is the richest specialization: Space + Extensions + Memory + clock + vitality. Agents have a type-state lifecycle, behavioral phases driven by vitality, a CorticalState for lock-free concurrent perception, multi-slot state management, EFE-gated routing, somatic markers for affective modulation, and a CognitiveWorkspace for learnable context assembly.

```rust
pub struct Agent<S: AgentLifecycleState> {
    pub id: AgentId,
    pub name: String,

    /// Type-state lifecycle marker (compile-time enforced transitions).
    pub state: PhantomData<S>,

    /// Isolation boundary.
    pub space: Space,

    /// Extensions pipeline.
    pub extensions: Vec<Extension>,

    /// Knowledge store with demurrage.
    pub memory: Memory,

    /// Clock bindings for the 3 cognitive timescales.
    pub clocks: AgentClocks,

    /// Vitality: remaining_budget / initial_budget.
    /// Drives behavioral phases (Thriving -> Terminal).
    pub vitality: Vitality,

    /// Lock-free atomic shared perception surface.
    pub cortical_state: Arc<CorticalState>,

    /// Named concurrent execution slots.
    pub slots: SlotManager,

    /// Affective state for dispatch modulation.
    pub somatic: SomaticState,

    /// Learnable context assembly.
    pub workspace: CognitiveWorkspace,

    /// The agent's 9-step processing pipeline (a Hot Graph).
    pub pipeline: HotFlow,
}
```

### Type-State Lifecycle

The Agent lifecycle is enforced at compile time using Rust's type-state pattern. Invalid transitions (e.g., `Terminal -> Active`) produce compile errors.

```rust
/// Lifecycle states. Implemented as zero-sized types for type-state.
pub struct Provisioning;
pub struct Active;
pub struct Dreaming;
pub struct Terminal;

/// Valid transitions (sealed trait pattern).
pub trait AgentLifecycleState: sealed::Sealed {}
impl AgentLifecycleState for Provisioning {}
impl AgentLifecycleState for Active {}
impl AgentLifecycleState for Dreaming {}
impl AgentLifecycleState for Terminal {}

impl Agent<Provisioning> {
    /// Provision -> Active: agent is ready to process.
    pub fn activate(self) -> Agent<Active> { /* ... */ }
}

impl Agent<Active> {
    /// Active -> Dreaming: enter dream consolidation.
    pub fn dream(self) -> Agent<Dreaming> { /* ... */ }

    /// Active -> Terminal: begin shutdown.
    pub fn terminate(self) -> Agent<Terminal> { /* ... */ }
}

impl Agent<Dreaming> {
    /// Dreaming -> Active: wake from consolidation.
    pub fn wake(self) -> Agent<Active> { /* ... */ }

    /// Dreaming -> Terminal: shutdown during dream.
    pub fn terminate(self) -> Agent<Terminal> { /* ... */ }
}
```

### Vitality and Behavioral Phases

Vitality is `remaining_budget / initial_budget` — a scalar from 0.0 to 1.0 that creates behavioral phases through economic pressure. An Agent that has never faced resource pressure has never learned to prioritize (Jonas 1966, mortality as precondition for value).

```rust
pub struct Vitality {
    pub remaining_budget: Cost,
    pub initial_budget: Cost,
}

impl Vitality {
    pub fn ratio(&self) -> f64 {
        self.remaining_budget.0 as f64 / self.initial_budget.0 as f64
    }

    pub fn phase(&self) -> BehavioralPhase {
        match self.ratio() {
            r if r >= 0.7 => BehavioralPhase::Thriving,
            r if r >= 0.4 => BehavioralPhase::Stable,
            r if r >= 0.2 => BehavioralPhase::Conservation,
            r if r >= 0.05 => BehavioralPhase::Declining,
            _ => BehavioralPhase::Terminal,
        }
    }
}

pub enum BehavioralPhase {
    /// 1.0-0.7: Explore freely, take risks, pursue novel strategies.
    Thriving,
    /// 0.7-0.4: Normal operation, balanced exploration/exploitation.
    Stable,
    /// 0.4-0.2: Reduce exploration, prioritize high-value tasks, begin
    /// knowledge transfer to peers.
    Conservation,
    /// 0.2-0.05: Pure exploitation, transfer remaining knowledge,
    /// complete in-progress work only.
    Declining,
    /// <0.05: Flush state, publish final knowledge Signals, shut down.
    Terminal,
}
```

### CorticalState

Lock-free atomic shared perception surface. Enables sub-microsecond concurrent reads from multiple Hot Graphs and Lenses without locking.

```rust
use std::sync::atomic::{AtomicU64, AtomicI64, Ordering};

/// Lock-free atomic shared perception surface.
/// All fields use atomic operations for concurrent access.
pub struct CorticalState {
    /// Current regime (encoded as u64).
    pub regime: AtomicU64,

    /// Vitality ratio (encoded as fixed-point i64).
    pub vitality_fp: AtomicI64,

    /// Number of active slots.
    pub active_slots: AtomicU64,

    /// Total cost spent (microcents as u64).
    pub cost_spent: AtomicU64,

    /// Bus sequence number of last processed Pulse.
    pub last_pulse_seq: AtomicU64,

    /// Pleasure-Arousal-Dominance (PAD) somatic state
    /// (3 x fixed-point i64).
    pub pleasure_fp: AtomicI64,
    pub arousal_fp: AtomicI64,
    pub dominance_fp: AtomicI64,

    /// Timestamp of last cortical update (Unix ms).
    pub updated_at_ms: AtomicI64,
}

impl CorticalState {
    /// Read current vitality as f64 (sub-microsecond).
    pub fn vitality(&self) -> f64 {
        self.vitality_fp.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    /// Read current regime.
    pub fn regime(&self) -> Regime {
        match self.regime.load(Ordering::Relaxed) {
            0 => Regime::Calm,
            1 => Regime::Normal,
            2 => Regime::Volatile,
            3 => Regime::Crisis,
            _ => Regime::Normal,
        }
    }
}
```

### Multi-Slot State

An Agent manages N concurrent named slots, each running a Flow. Slots share the Agent's global budget and CorticalState but execute independently.

```rust
pub struct SlotManager {
    /// Named active slots.
    pub slots: BTreeMap<String, Slot>,

    /// Maximum concurrent slots.
    pub max_slots: usize,

    /// Shared budget across all slots.
    pub budget: Arc<Vitality>,
}

pub struct Slot {
    pub name: String,
    pub flow: Flow,
    pub started_at: DateTime<Utc>,
    pub priority: u32,
}
```

### Somatic Markers

Affective state for dispatch modulation. Based on the **PAD model** (Pleasure-Arousal-Dominance, Mehrabian & Russell 1974) with **prospect theory** loss aversion (Kahneman & Tversky 1979, loss aversion coefficient lambda = 2.2). Somatic markers modulate routing decisions: high arousal + low dominance = conservative routing; high pleasure + high dominance = exploratory routing.

```rust
pub struct SomaticState {
    /// Pleasure-Arousal-Dominance triplet, each in [-1.0, 1.0].
    pub pad: PadVector,

    /// Recent events that shaped the current affective state.
    pub recent_markers: Vec<SomaticMarker>,

    /// k-d tree for fast nearest-neighbor lookup of similar
    /// affective states in episode history (<100 microseconds).
    pub marker_index: KdTree<SomaticMarker>,
}

pub struct PadVector {
    pub pleasure: f64,    // -1.0 (pain) to 1.0 (pleasure)
    pub arousal: f64,     // -1.0 (calm) to 1.0 (excited)
    pub dominance: f64,   // -1.0 (submissive) to 1.0 (dominant)
}

pub struct SomaticMarker {
    /// The affective state when this marker was created.
    pub pad: PadVector,

    /// What caused this marker (gate pass, failure, cost spike, etc.).
    pub cause: MarkerCause,

    /// Outcome valence (positive or negative).
    /// Losses weighted lambda = 2.2 vs gains (Kahneman-Tversky).
    pub valence: f64,

    /// When this marker was created.
    pub timestamp: DateTime<Utc>,

    /// Episode reference for provenance.
    pub episode: Option<SignalRef>,
}

pub enum MarkerCause {
    GatePass { reward: f64 },
    GateFail { penalty: f64 },
    CostSpike { delta: Cost },
    BudgetLow { vitality: f64 },
    SuccessStreak { length: u32 },
    FailureStreak { length: u32 },
    NovelDiscovery { surprise: f64 },
    PeerFeedback { sentiment: f64 },
}
```

### CognitiveWorkspace

Learnable context assembly using VCG auction ([doc-02](02-CELL.md) S2.5) with section effect tracking. The CognitiveWorkspace is where the Agent assembles the prompt for each LLM call, using bidders that learn which context sections lead to successful gate outcomes.

```rust
pub struct CognitiveWorkspace {
    /// Registered context bidders.
    pub bidders: Vec<Arc<dyn ContextBidder>>,

    /// Section effect posteriors: per-section beta distributions
    /// tracking correlation with downstream gate success.
    pub section_effects: BTreeMap<String, BetaPosterior>,

    /// Composition history for novelty attenuation.
    pub section_frequencies: BTreeMap<String, u64>,
}

#[async_trait]
pub trait ContextBidder: Send + Sync {
    fn bidder_id(&self) -> BidderId;
    fn name(&self) -> &str;

    /// Generate bids for inclusion in the next prompt.
    async fn bid(
        &self,
        task: &Signal,
        budget: &ComposeBudget,
        ctx: &BidContext,
    ) -> Result<Vec<ComposeBid>>;
}

pub struct BidContext {
    pub agent_id: AgentId,
    pub vitality: f64,
    pub recent_episodes: Vec<Signal>,
    pub active_heuristics: Vec<Signal>,
}
```

### EFE Gating

The Agent uses EFE (Expected Free Energy, Friston 2006) from the Route protocol to gate every significant decision: which model to call, whether to use a cached result, whether to explore a new tool. Each cognitive timescale (gamma/theta/delta) operates at a different free-energy lower bound — fast reflexes tolerate higher free energy than slow deliberation.

**Protocols used**: Store, Score, Verify, Route, Compose, React, Observe, Connect (potentially all 9 — Agent is the universal composition).

---

## 11. Connector

A **Connector** is a Cell conforming to the Connect protocol ([doc-02](02-CELL.md) S2.8) with lifecycle management. Connectors handle external system I/O: databases, APIs, file systems, message queues.

```rust
pub struct Connector {
    /// The underlying Connect Cell.
    pub block: Arc<dyn ConnectProtocol>,

    /// Connection configuration.
    pub config: ConnectConfig,

    /// Current connection state.
    pub state: ConnectorState,

    /// Health check interval.
    pub health_interval: Duration,

    /// Reconnection policy.
    pub reconnect: ReconnectPolicy,
}

pub struct ConnectConfig {
    pub protocol: String,
    pub endpoint: String,
    pub auth: Option<AuthConfig>,
    pub timeout: Duration,
    pub max_retries: u32,
}

pub enum ConnectorState {
    Disconnected,
    Connecting,
    Connected(ConnectionHandle),
    Reconnecting { attempt: u32 },
    Failed { error: String },
}

pub enum ReconnectPolicy {
    /// No automatic reconnection.
    None,
    /// Reconnect with exponential backoff.
    Backoff { base: Duration, max: Duration, max_attempts: u32 },
    /// Reconnect immediately, up to N times.
    Immediate { max_attempts: u32 },
}
```

**Protocols used**: Connect.

---

## 12. Summary Table

| Specialization | Built From | Protocols Used | TOML Defined? | New Type? |
|---|---|---|---|---|
| **Flow** | Graph + RunId + state + snapshots | (runtime wrapper) | Yes (`[graph]`) | `Flow` (runtime only) |
| **Hot Flow** | Flow + `hot=true` + ClockBinding | (runtime wrapper) | Yes (`hot = true`) | (variant of Flow) |
| **Rack** | Graph + Macros + Slots | (inherits from Graph) | Yes (`[rack]`) | `Rack` |
| **Trigger** | Cell + Trigger protocol | Trigger | Yes (`[trigger]`) | `TriggerSpec` |
| **Lens** | Cell + Observe protocol | Observe | Yes (`[lens]`) | `Lens` |
| **Loop** | Graph + feedback edge | (inherits from Graph) | Yes (`kind = "loop"`) | (Graph convention) |
| **Memory** | Store Cell + demurrage + dreams | Store, React | Yes (`[memory]`) | `Memory` |
| **Space** | Isolation boundary + capability grants | Store (scoped), Bus (scoped) | Yes (`[space]`) | `Space` |
| **Extension** | Cell intercepting pipeline | (wraps any) | Yes (`[extension]`) | `Extension` |
| **Agent** | Space + Extensions + Memory + clock + vitality | All 9 (potentially) | Yes (`[agent]`) | `Agent<S>` |
| **Connector** | Cell + Connect + lifecycle | Connect | Yes (`[connector]`) | `Connector` |

---

## 13. Citations

| Concept | Citation |
|---|---|
| Mortality as precondition for value | Jonas, H. (1966). *The Phenomenon of Life: Toward a Philosophical Biology*. |
| PAD affect model | Mehrabian, A., & Russell, J. A. (1974). *An Approach to Environmental Psychology*. MIT Press. |
| Prospect theory, loss aversion (lambda = 2.2) | Kahneman, D., & Tversky, A. (1979). Prospect theory: An analysis of decision under risk. *Econometrica*, 47(2), 263-292. |
| c-factor (collective intelligence) | Woolley, A. W. et al. (2010). Evidence for a collective intelligence factor in the performance of human groups. *Science*, 330(6004), 686-688. |
| Active inference, EFE | Friston, K. (2006). A free energy principle for the brain. *Journal of Physiology-Paris*, 100(1-3), 70-87. |
| VCG auction mechanism | Vickrey (1961), Clarke (1971), Groves (1973). See [doc-02](02-CELL.md) S4. |
| CaMeL IFC | See [doc-08](08-EXTENSION-SYSTEM.md) and [doc-17](17-SECURITY-MODEL.md). |
| Stigmergic coordination | Dorigo, M. (1992). *Optimization, learning, and natural algorithms*. PhD thesis, Politecnico di Milano. |
| Gesell demurrage | Gesell, S. (1916). *The Natural Economic Order*. |

---

## 14. Acceptance Criteria

| Criterion | Verification |
|---|---|
| All 10 specializations are expressible from Cell + Graph + Signal (no kernel changes) | Design review |
| `Flow` wraps `Graph` with `RunId` and `FlowState` | Compile check |
| Hot Flow retains state between clock ticks | Integration test |
| `Rack` exposes `macros` that adjust Cell params | Unit test: change macro, verify param changed |
| Rack `Slot` rejection when unfilled | Unit test |
| `Trigger` arms, fires, and disarms correctly | Integration test (see [doc-06](06-TRIGGER-SYSTEM.md)) |
| `Lens` is read-only: attempting Store write through Lens fails | Unit test |
| `Loop` terminates when condition met or max_iterations reached | Unit test |
| `Memory` applies demurrage: balance decreases without interaction | Unit test |
| `Memory` dream consolidation runs on schedule | Integration test with mock clock |
| `Space` capability intersection: denied in Space -> denied for Cell | Unit test |
| `Space` bus isolation: Pulse in Space A not visible in Space B | Integration test |
| `Extension` CaMeL tags prevent capability escalation | Unit test: Extension with `read: [Public]` cannot read `Secret` |
| Agent type-state: `Terminal -> Active` does not compile | Compile check (negative test) |
| Agent vitality phases: 0.3 -> Conservation | Unit test |
| CorticalState concurrent reads: 4 threads, no data race | Integration test with `loom` or `std::thread` |
| Somatic markers: gate failure creates negative marker | Unit test |
| Somatic k-d tree query returns nearest marker in <100 microseconds | Benchmark |
| CognitiveWorkspace VCG auction selects budget-feasible sections | Unit test |
| Section effect posteriors update on gate verdict | Unit test: alpha increments on pass |
| Connector reconnects on failure with backoff | Integration test with mock connection |
| All 11 specializations listed in summary table | Review |
