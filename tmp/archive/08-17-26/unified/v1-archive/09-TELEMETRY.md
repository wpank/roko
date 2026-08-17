# 09 — Telemetry: Lens System and StateHub

> Full observability through the Observe protocol. Lenses focus attention without modifying the subject. StateHub projects Lens output into typed projections consumed by every surface.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality), [02-BLOCK](02-BLOCK.md) (Observe protocol, predict-publish-correct), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Lens definition), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (CorticalState, vitality)

---

## 1. Overview

Every Block, Graph, Agent, and Space in Roko can be observed without modification. The **Lens** specialization — a Block implementing the Observe protocol — receives read-only lifecycle events and emits observation Signals onto the Bus.

The telemetry system has two layers:

1. **Lenses** — the raw observation machinery. Blocks that watch events and emit structured observation Signals.
2. **StateHub** — the projection layer. Consumes Lens output and produces typed, versioned projections that every surface (TUI, web, Slack, audit) can subscribe to.

The design follows four principles:

1. **Observation is passive.** A Lens never mutates the subject. Removing all Lenses from a system changes nothing about its behavior — only visibility.
2. **Observation is compositional.** Lenses stack (multiple Lenses on one target), chain (a Lens watches another Lens's output), and scope (Block, Graph, Agent, Space granularity).
3. **Observation uses the same primitives.** Lens output is a Signal. Lens composition is a Graph. Lens configuration is TOML. No special telemetry infrastructure.
4. **Projections are the data contracts.** StateHub projections are the typed interfaces between the telemetry system and the five named surfaces in [doc-16](16-SURFACES.md). Surfaces never read raw Lens output — they subscribe to projections.

---

## 2. The Observe Protocol

The Observe protocol is the seventh of nine Block protocols. It is the only protocol that is strictly read-only — calling `observe()` never modifies the event source.

```rust
pub trait Observe: Block {
    /// Observe a single event. Emit zero or more observation Signals.
    /// The event is immutable — Lenses cannot modify what they observe.
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>>;

    /// Declare which event types this Lens observes.
    /// The engine only routes matching events to this Lens.
    fn observes(&self) -> &[ObservableEventKind];

    /// The scope this Lens is attached to.
    fn scope(&self) -> LensScope;
}
```

### LensScope

```rust
pub enum LensScope {
    /// Observe a single Block's events.
    Block(BlockRef),

    /// Observe all Blocks within a Graph.
    Graph(GraphRef),

    /// Observe an Agent's full pipeline (all internal Graphs).
    Agent(AgentRef),

    /// Observe everything within a Space (all Agents, Graphs, Blocks).
    Space(SpaceId),

    /// Observe a specific other Lens's output (chaining).
    Lens(LensRef),

    /// Global — observe all events system-wide.
    Global,
}
```

The engine uses `observes()` and `scope()` together to build a routing table at Graph-load time. Events matching both the kind filter and the scope filter are delivered to the Lens. Events outside scope are never delivered — the Lens does not pay for events it ignores.

### ObservableEvent

```rust
pub enum ObservableEvent {
    // ── Signal lifecycle ─────────────────────────────────────
    SignalCreated(Signal),
    SignalScored(SignalRef, ScoreResult),
    SignalRouted(SignalRef, RouteResult),
    SignalVerified(SignalRef, Verdict),
    SignalComposed(Vec<SignalRef>, Signal),
    SignalDemurrageApplied(SignalRef, f64),   // ref + new balance
    SignalPromoted(SignalRef, Tier, Tier),     // ref + old tier + new tier
    SignalPruned(SignalRef),

    // ── Block lifecycle ──────────────────────────────────────
    BlockStarted { block: BlockRef, run: RunId, input_hash: ContentHash },
    BlockCompleted { block: BlockRef, run: RunId, duration: Duration, cost: Cost },
    BlockFailed { block: BlockRef, run: RunId, error: BlockError },
    BlockRetried { block: BlockRef, run: RunId, attempt: u32, reason: String },
    BlockCancelled { block: BlockRef, run: RunId },
    BlockPredictionPublished { block: BlockRef, prediction: Pulse },
    BlockCalibrationReceived { block: BlockRef, error: f64 },

    // ── Graph lifecycle ──────────────────────────────────────
    GraphStarted { graph: GraphRef, run: RunId, input_hash: ContentHash },
    GraphNodeCompleted { graph: GraphRef, run: RunId, node: NodeId, duration: Duration },
    GraphCompleted { graph: GraphRef, run: RunId, duration: Duration, cost: Cost },
    GraphFailed { graph: GraphRef, run: RunId, error: BlockError },
    GraphPaused { graph: GraphRef, run: RunId, reason: PauseReason },
    GraphResumed { graph: GraphRef, run: RunId },

    // ── Agent lifecycle ──────────────────────────────────────
    AgentTick { agent: AgentRef, regime: Regime, prediction_error: f64, vitality: f64 },
    AgentRegimeChange { agent: AgentRef, old: Regime, new_regime: Regime },
    AgentBudgetUpdate { agent: AgentRef, spent: Cost, remaining: Cost, vitality: f64 },
    AgentModeChange { agent: AgentRef, old: AgentMode, new_mode: AgentMode },
    AgentPhaseChange { agent: AgentRef, old: VitalityPhase, new_phase: VitalityPhase },
    AgentStateTransition { agent: AgentRef, old: TypeState, new_state: TypeState },
    AgentSlotUpdate { agent: AgentRef, slot: SlotName, state: SlotState },

    // ── Memory lifecycle ─────────────────────────────────────
    MemoryRetrieved { query: String, results: usize, duration: Duration },
    MemoryStored { signal: SignalRef, tier: Tier },
    MemoryConsolidated { promoted: usize, demoted: usize, pruned: usize },
    DemurrageApplied { count: usize, total_balance_lost: f64 },

    // ── Verify lifecycle ─────────────────────────────────────
    VerifyPreResult { block: BlockRef, verdict: Verdict, evidence: Vec<EvidenceKind> },
    VerifyPostResult { block: BlockRef, verdict: Verdict, reward: f64, evidence: Vec<EvidenceKind> },

    // ── Trigger lifecycle ────────────────────────────────────
    TriggerFired { trigger: TriggerRef, graph: GraphRef },
    TriggerArmed { trigger: TriggerRef },
    TriggerDisarmed { trigger: TriggerRef },

    // ── Extension lifecycle ──────────────────────────────────
    ExtensionHookCalled { extension: String, hook: String, layer: u8, duration: Duration },
    ExtensionHookFailed { extension: String, hook: String, error: String },
}
```

### ObservableEventKind

```rust
/// Used by Lens::observes() to declare interest.
/// The engine matches events against these kinds for routing.
pub enum ObservableEventKind {
    SignalLifecycle,
    BlockLifecycle,
    GraphLifecycle,
    AgentLifecycle,
    MemoryLifecycle,
    VerifyLifecycle,
    TriggerLifecycle,
    ExtensionLifecycle,
    All,
}
```

A Lens declaring `observes() = &[ObservableEventKind::BlockLifecycle]` receives only `BlockStarted`, `BlockCompleted`, `BlockFailed`, `BlockRetried`, and `BlockCancelled` events. A Lens declaring `All` receives everything within its scope.

---

## 3. Built-in Lenses

Roko ships 11 built-in Lenses. Each is a Block implementing Observe, packaged with the `roko-core` distribution.

### 3.1 CostLens

Tracks USD and token expenditure across Block executions.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle`, `AgentLifecycle` |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: CostReport }` |

**Emitted Signal payload**:

```rust
pub struct CostReportPayload {
    pub target: String,
    pub interval: Duration,
    pub total_usd: f64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_breakdown: BTreeMap<String, f64>,
    pub cumulative_usd: f64,
    pub budget_remaining: Option<f64>,
    pub vitality: Option<f64>,
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 60s | Aggregation window for emitting reports |
| `budget_warn_pct` | f64 | 0.80 | Emit Alert when this % of budget consumed |
| `budget_critical_pct` | f64 | 0.95 | Emit Alert(Critical) at this threshold |
| `include_model_breakdown` | bool | true | Break costs down per model |

### 3.2 LatencyLens

Measures execution duration with percentile tracking.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle` |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: Observation }` with latency payload |

**Emitted Signal payload**:

```rust
pub struct LatencyPayload {
    pub target: String,
    pub interval: Duration,
    pub count: u64,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub mean: Duration,
    pub min: Duration,
    pub max: Duration,
    pub stddev: Duration,
    pub histogram: Vec<HistogramBucket>,
}
```

### 3.3 QualityLens

Tracks pass/fail rates from Verify-protocol Blocks, including the redesigned pre/post verification.

| Property | Value |
|---|---|
| **Observes** | `VerifyLifecycle`, `SignalLifecycle` |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: Observation }` with quality payload |

**Emitted Signal payload**:

```rust
pub struct QualityPayload {
    pub target: String,
    pub interval: Duration,
    pub total_verifications: u64,
    pub pre_verify_vetoes: u64,         // verify_pre rejections
    pub post_verify_passed: u64,
    pub post_verify_failed: u64,
    pub pass_rate: f64,
    pub avg_reward: f64,                // mean Verdict.reward (continuous)
    pub min_reward: f64,
    pub evidence_type_breakdown: BTreeMap<String, u64>,
    pub hard_criteria_failures: u64,    // conjunctive hard fails
    pub rung_breakdown: BTreeMap<String, PassFailCounts>,
}
```

### 3.4 EfficiencyLens

Measures tokens-per-task and cost-per-quality ratios.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `AgentLifecycle` |
| **Default Scope** | `Agent` |
| **Emits** | `Signal { kind: Observation }` with efficiency payload |

**Emitted Signal payload**:

```rust
pub struct EfficiencyPayload {
    pub agent: String,
    pub interval: Duration,
    pub tasks_completed: u64,
    pub tokens_per_task: f64,
    pub usd_per_task: f64,
    pub quality_per_usd: f64,
    pub tokens_per_quality: f64,
    pub t0_hit_rate: f64,
    pub t1_hit_rate: f64,
    pub t2_hit_rate: f64,
    pub avg_prediction_error: f64,
    pub vitality: f64,
    pub vitality_phase: VitalityPhase,
}
```

### 3.5 ErrorLens

Classifies and aggregates errors across Block executions.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle`, `ExtensionLifecycle` |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: Observation }` with error report payload |

**Emitted Signal payload**:

```rust
pub struct ErrorPayload {
    pub target: String,
    pub interval: Duration,
    pub total_errors: u64,
    pub by_category: BTreeMap<ErrorCategory, u64>,
    pub by_block: BTreeMap<String, u64>,
    pub retry_count: u64,
    pub retry_success_rate: f64,
    pub error_rate: f64,
    pub most_common: Vec<ErrorSummary>,
}

pub enum ErrorCategory {
    Timeout,
    CapabilityDenied,
    External,
    LogicError,
    InvalidInput,
    Cancelled,
}
```

### 3.6 DriftLens

Detects knowledge quality degradation in Memory stores. Now tracks demurrage-driven balance changes rather than time-based decay.

| Property | Value |
|---|---|
| **Observes** | `MemoryLifecycle`, `SignalLifecycle` |
| **Default Scope** | `Agent` |
| **Emits** | `Signal { kind: Observation }` with drift payload |

**Emitted Signal payload**:

```rust
pub struct DriftPayload {
    pub memory: String,
    pub interval: Duration,
    pub total_entries: u64,
    pub tier_distribution: BTreeMap<Tier, u64>,
    pub avg_balance: f64,               // mean balance across entries (demurrage)
    pub balance_delta: f64,             // change since last interval
    pub promotion_rate: f64,
    pub demotion_rate: f64,
    pub pruning_rate: f64,
    pub retrieval_hit_rate: f64,
    pub cold_entries: u64,              // below cold_threshold, candidates for archival
    pub anti_knowledge_count: u64,
    pub heuristic_calibration_avg: f64, // mean heuristic calibration score
}
```

### 3.7 BudgetLens

Monitors budget consumption and vitality across Agents and Spaces.

| Property | Value |
|---|---|
| **Observes** | `AgentLifecycle`, `BlockLifecycle` |
| **Default Scope** | `Agent` or `Space` |
| **Emits** | `Signal { kind: Alert }` when thresholds crossed |

**Emitted Signal payload**:

```rust
pub struct BudgetAlertPayload {
    pub target: String,
    pub budget_total: f64,
    pub budget_spent: f64,
    pub budget_remaining: f64,
    pub pct_consumed: f64,
    pub vitality: f64,
    pub vitality_phase: VitalityPhase,
    pub projected_exhaustion: Option<DateTime<Utc>>,
    pub burn_rate: f64,
    pub level: AlertLevel,
}
```

### 3.8 TrendLens

Computes statistical trends over time-series observation data.

| Property | Value |
|---|---|
| **Observes** | Any other Lens's output (chaining Lens) |
| **Default Scope** | `Lens` (wraps another Lens) |
| **Emits** | `Signal { kind: Trend }` |

**Emitted Signal payload**:

```rust
pub struct TrendPayload {
    pub source_lens: String,
    pub metric: String,
    pub window: Duration,
    pub slope: f64,
    pub slope_pct: f64,
    pub ema: f64,
    pub ema_previous: f64,
    pub ema_delta: f64,
    pub direction: TrendDirection,
    pub r_squared: f64,
    pub data_points: usize,
}

pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
}
```

### 3.9 AnomalyLens

Detects statistical outliers in observation streams.

| Property | Value |
|---|---|
| **Observes** | Any other Lens's output (chaining Lens) |
| **Default Scope** | `Lens` (wraps another Lens) |
| **Emits** | `Signal { kind: Anomaly }` |

**Emitted Signal payload**:

```rust
pub struct AnomalyPayload {
    pub source_lens: String,
    pub metric: String,
    pub observed_value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub direction: AnomalyDirection,
    pub severity: AnomalyLevel,
    pub context: BTreeMap<String, Value>,
}
```

Detection uses rolling window z-score with IQR fallback for non-Gaussian distributions.

### 3.10 UsageLens

Tracks usage analytics for marketplace and developer metrics.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle`, `TriggerLifecycle` |
| **Default Scope** | `Space` or `Global` |
| **Emits** | `Signal { kind: Observation }` with usage payload |

### 3.11 CollectiveIntelligenceLens

Computes the **c-factor** — collective intelligence as a runtime observable. The c-factor is derived from four components measured across a cohort of agents:

| Property | Value |
|---|---|
| **Observes** | `AgentLifecycle`, `SignalLifecycle`, `MemoryLifecycle` |
| **Default Scope** | `Space` (measures across all agents in a Space) |
| **Emits** | `Signal { kind: CFactorReport }` |

**Emitted Signal payload**:

```rust
pub struct CFactorPayload {
    pub space: String,
    pub interval: Duration,
    pub c_factor: f64,                       // composite score (0.0..=1.0)

    // ── Components ───────────────────────────────────────────
    pub turn_taking_entropy: f64,            // how evenly distributed are agent turns
    pub peer_prediction_accuracy: f64,       // how well agents predict each other's output
    pub citation_reciprocity: f64,           // knowledge attribution flow balance
    pub hdc_diversity: f64,                  // spread of HDC fingerprints (avoid collapse)

    // ── Diagnostics ──────────────────────────────────────────
    pub agent_count: usize,
    pub active_agents: usize,
    pub dominant_agent_share: f64,           // % of turns by most active agent
    pub knowledge_flow_edges: usize,         // citation graph edges in interval
    pub avg_agent_vitality: f64,
}
```

### c-factor components

| Component | What it measures | Why it matters |
|---|---|---|
| **Turn-taking entropy** | Shannon entropy of agent turn distribution | High entropy = balanced participation. Low entropy = one agent dominates, others idle. |
| **Peer prediction accuracy** | How well agents predict what other agents will produce | High accuracy = shared mental model. Measured via predict-publish-correct on inter-agent Signals. |
| **Citation reciprocity** | Balance of knowledge attribution flow | Reciprocal citation = genuine knowledge exchange. One-way flow = parasitic consumption. |
| **HDC diversity** | Spread of episode fingerprints across agents | High diversity = agents explore different regions of solution space. Low diversity = redundant work. |

### c-factor as gate for L4

The c-factor is not just a metric — it is a gate. L4 structural adaptation ([doc-10 §5](10-LEARNING-LOOPS.md)) only evolves configurations that increase genuine collective intelligence. A configuration that improves individual metrics but decreases c-factor is rejected. This prevents optimization pressure from collapsing agent diversity.

---

## 4. Lens Composition

Lenses compose in three ways: stacking, chaining, and scoping. All three are configured via TOML and resolved at Graph-load time.

### 4.1 Stacking (multiple Lenses on same target)

Multiple Lenses can observe the same Block, Graph, Agent, or Space simultaneously. Each receives the same events independently.

```
         ┌──────────┐
         │  Target   │
         │  (Block)  │
         └─────┬─────┘
               │ events
         ┌─────┼─────┐
         ▼     ▼     ▼
    ┌────────┐ ┌────────┐ ┌────────┐
    │CostLens│ │Latency │ │Quality │
    │        │ │ Lens   │ │ Lens   │
    └───┬────┘ └───┬────┘ └───┬────┘
        │          │          │
        ▼          ▼          ▼
      Cost       Latency    Quality
      Signal     Signal     Signal
        │          │          │
        └──────────┼──────────┘
                   ▼
                  Bus
```

Stacking is the default composition. Attaching three Lenses to a Graph means all three independently observe the same event stream. There is no ordering dependency between stacked Lenses.

### 4.2 Chaining (Lens observes another Lens's output)

A Lens can observe another Lens's output Signals rather than raw lifecycle events. This enables derived metrics.

```
    ┌────────┐      ┌─────────┐      ┌──────────┐
    │CostLens│ ───► │TrendLens│ ───► │Anomaly   │
    │        │      │(cost $) │      │Lens      │
    └────────┘      └─────────┘      └──────────┘
       emits           emits            emits
     CostReport       Trend            Anomaly
      Signals         Signals          Signals
```

Chaining uses `LensScope::Lens(ref)`. The engine ensures delivery order: upstream Lens output is delivered to downstream Lens before the next event cycle.

### 4.3 Scoping (Block, Graph, Agent, Space levels)

| Scope | Receives events from | Aggregation |
|---|---|---|
| `Block` | Single Block only | Per-invocation metrics |
| `Graph` | All Blocks within one Graph | Aggregated across nodes |
| `Agent` | All Graphs within one Agent | Cross-pipeline view |
| `Space` | All Agents within one Space | Full workspace view |
| `Global` | Everything in the system | System-wide overview |

### Composition rules

1. **Independence**: Stacked Lenses do not affect each other. Failure in one does not interrupt others.
2. **Ordering**: Chained Lenses are topologically sorted. Cycles are rejected at Graph-load time.
3. **Scope narrowing**: A Lens at Graph scope does not receive events from Blocks in other Graphs. Scope is strict.
4. **Scope widening**: A Lens at Space scope receives events from all contained Agents/Graphs/Blocks. Wider scope means more events.
5. **Cross-scope chaining**: A Graph-scoped CostLens can chain into a Space-scoped TrendLens. The TrendLens receives CostReport Signals from all Graphs in the Space.

---

## 5. Data Flow

Observation Signals follow a defined path from event source to consumers.

```
Block execution                  ┌───────────────────────────────────┐
    │                            │           Consumers               │
    ▼                            │                                   │
Lifecycle event                  │  ┌─────────────────────┐         │
(BlockCompleted,                 │  │ StateHub              │         │
 GraphFailed, etc.)              │  │ typed projections     │         │
    │                            │  └────────┬────────────┘         │
    ▼                            │           │                      │
Engine routes to                 │     ┌─────┼──────────┐          │
matching Lenses                  │     ▼     ▼          ▼          │
    │                            │  ┌──────┐ ┌──────┐ ┌──────┐    │
    ▼                            │  │ TUI  │ │ Web  │ │Slack │    │
Lens.observe(event)              │  └──────┘ └──────┘ └──────┘    │
    │                            │                                   │
    ▼                            │  ┌─────────────────────┐         │
Observation Signals  ──► Bus ──►─┤  │ Episode Logger        │         │
(Observation, Alert,             │  │ .roko/episodes.jsonl  │         │
 Trend, Anomaly,                 │  └─────────────────────┘         │
 CostReport,                     │                                   │
 CFactorReport kinds)            │  ┌─────────────────────┐         │
                                 │  │ Learning Loops        │         │
                                 │  │ (see doc-10)         │         │
                                 │  └─────────────────────┘         │
                                 │                                   │
                                 │  ┌─────────────────────┐         │
                                 │  │ Store (historical)    │         │
                                 │  │ query-able via API    │         │
                                 │  └─────────────────────┘         │
                                 │                                   │
                                 │  ┌─────────────────────┐         │
                                 │  │ React Blocks          │         │
                                 │  │ (policy enforcement)  │         │
                                 │  └─────────────────────┘         │
                                 └───────────────────────────────────┘
```

### Step-by-step flow

1. **Event generation**: A Block completes execution. The engine emits a `BlockCompleted` event as a Pulse on the Bus topic `block:{id}:events`.

2. **Event routing**: The engine's Lens routing table matches the event's kind and scope to all registered Lenses. Only matching Lenses receive the event.

3. **Lens invocation**: Each matching Lens's `observe()` method is called with the event. The Lens processes the event and returns zero or more observation Signals.

4. **Signal emission**: The returned observation Signals are published to the Bus on the topic `lens:{id}:observations`.

5. **StateHub consumption**: StateHub subscribes to all `lens:*:observations` topics, updates its projections, and notifies subscribed surfaces.

6. **Consumer delivery**: Additional Bus subscribers consume observation Signals:
   - **Episode Logger**: Subscribes to observation Signals tagged `persist`, appends to `.roko/episodes.jsonl`.
   - **Learning Loops**: Loop Graphs subscribe to specific observation types (e.g., CostReport for the cost-optimization Loop).
   - **Store**: A React Block persists selected observation Signals for historical query.
   - **Chained Lenses**: Downstream Lenses subscribed via `LensScope::Lens` receive observation Signals as events.

### Delivery guarantees

| Consumer | Guarantee | Rationale |
|---|---|---|
| StateHub | At-least-once | Projections must reflect all Lens output |
| Episode Logger | At-least-once | Episodes must not be lost |
| Learning Loops | At-least-once | Learning requires complete data |
| Store | At-least-once | Historical queries need completeness |
| Chained Lenses | Exactly-once (within tolerance) | Chain ordering requires delivery |
| Surface clients (WS/SSE) | At-most-once | Dropped frames acceptable for real-time display |

---

## 6. StateHub — The Projection Layer

StateHub is the universal projection layer between Lens output and all consumer surfaces. It subscribes to Lens observation Signals, maintains typed projection state, and publishes versioned snapshots that surfaces subscribe to.

### Why StateHub

Without StateHub, every surface (TUI, web dashboard, Slack bot, audit trail) would independently parse raw Lens output, compute its own aggregations, and maintain its own state. This leads to inconsistency (surfaces disagree on numbers) and duplication (same computation done N times for N surfaces).

StateHub centralizes the computation once and projects the result to all consumers through typed, versioned contracts.

### Core design

```rust
pub struct StateHub {
    pub projections: BTreeMap<ProjectionId, ProjectionState>,
    pub subscribers: Vec<SurfaceSubscription>,
    pub bus: BusHandle,
}

pub struct ProjectionState {
    pub id: ProjectionId,
    pub schema: TypeSchema,
    pub version: u64,                    // monotonic, incremented on update
    pub data: Value,                     // typed payload matching schema
    pub updated_at: DateTime<Utc>,
    pub source_lenses: Vec<LensRef>,     // which Lenses feed this projection
}
```

### 7 Core Projections

StateHub maintains these typed projections. Each projection is a data contract consumed by one or more surfaces.

| Projection | Type | Source Lenses | Consumers |
|---|---|---|---|
| `cohort_health` | `CohortHealthProjection` | EfficiencyLens, ErrorLens, BudgetLens | TUI (F1), Web dashboard |
| `active_tasks` | `ActiveTasksProjection` | QualityLens, LatencyLens | TUI (F2: Tasks), Workbench surface |
| `gate_pipeline` | `GatePipelineProjection` | QualityLens | TUI (F3: Gates), Audit trail |
| `cost_meter` | `CostMeterProjection` | CostLens, BudgetLens | TUI (F5: Telemetry), Web |
| `knowledge_health` | `KnowledgeHealthProjection` | DriftLens | TUI (F6: Knowledge), Web |
| `c_factor` | `CFactorProjection` | CollectiveIntelligenceLens | TUI (F7), L4 gate, Web |
| `agent_vitality` | `AgentVitalityProjection` | BudgetLens, EfficiencyLens | TUI (F4: Agents), Slack alerts |

### Projection schemas

```rust
pub struct CohortHealthProjection {
    pub agent_count: usize,
    pub active_count: usize,
    pub avg_vitality: f64,
    pub avg_pass_rate: f64,
    pub total_spend_usd: f64,
    pub error_rate: f64,
    pub t0_hit_rate: f64,
    pub regime_distribution: BTreeMap<Regime, usize>,
}

pub struct ActiveTasksProjection {
    pub tasks: Vec<TaskSnapshot>,        // in-progress tasks with agent, progress, ETA
    pub queued: usize,
    pub completed_last_hour: usize,
    pub avg_task_duration: Duration,
}

pub struct GatePipelineProjection {
    pub rungs: Vec<RungSnapshot>,        // per-rung pass rate, avg duration, last run
    pub overall_pass_rate: f64,
    pub avg_reward: f64,                 // continuous reward from Verify
    pub hard_criteria_fail_rate: f64,
}

pub struct CostMeterProjection {
    pub total_usd: f64,
    pub budget_remaining: f64,
    pub burn_rate_usd_per_hour: f64,
    pub projected_exhaustion: Option<DateTime<Utc>>,
    pub model_breakdown: BTreeMap<String, f64>,
    pub cost_trend: TrendDirection,
}

pub struct KnowledgeHealthProjection {
    pub total_entries: u64,
    pub tier_distribution: BTreeMap<Tier, u64>,
    pub avg_balance: f64,
    pub cold_entries: u64,
    pub heuristic_count: u64,
    pub heuristic_avg_calibration: f64,
    pub anti_knowledge_count: u64,
}

pub struct CFactorProjection {
    pub c_factor: f64,
    pub components: CFactorComponents,
    pub trend: TrendDirection,
    pub agent_diversity: f64,
}

pub struct AgentVitalityProjection {
    pub agents: Vec<AgentVitalitySnapshot>,
}

pub struct AgentVitalitySnapshot {
    pub name: String,
    pub vitality: f64,
    pub phase: VitalityPhase,
    pub regime: Regime,
    pub slots_active: usize,
    pub slots_total: usize,
    pub tasks_completed: u64,
    pub current_task: Option<String>,
}
```

### Projection updates

StateHub updates projections on every Lens observation Signal. Each update:

1. Applies the observation to the relevant projection(s).
2. Increments the projection `version`.
3. Publishes a Pulse on `statehub:{projection_id}:updated` with the new version number.
4. Subscribed surfaces receive the Pulse and fetch the updated projection.

Surfaces can subscribe at different resolutions. A TUI subscribing at 100ms resolution receives every update. A Slack bot subscribing at 60s resolution receives only the latest state per minute.

### StateHub as data contracts for surfaces

The five named surfaces in [doc-16](16-SURFACES.md) consume StateHub projections:

| Surface | Projections consumed |
|---|---|
| **Workbench** | `active_tasks`, `gate_pipeline`, `cost_meter` |
| **Agent Inbox** | `agent_vitality`, `cohort_health` |
| **Generative Canvas** | `active_tasks`, `gate_pipeline` |
| **Stigmergy Minimap** | `c_factor`, `cohort_health`, `knowledge_health` |
| **Autonomy Slider** | `agent_vitality`, `c_factor` |

Surfaces never read raw Lens output. The projection schemas are the stable API between telemetry and UX.

---

## 7. Dashboard Integration

The dashboard (TUI, web, or visual editor) receives Lens output via StateHub projections, streamed over WebSocket or SSE.

### WebSocket protocol

The control plane (`roko serve` on :6677) exposes a WebSocket endpoint for live telemetry:

```
GET /ws/telemetry
```

The client sends a subscription message:

```json
{
    "subscribe": {
        "projections": ["cohort_health", "cost_meter", "c_factor"],
        "resolution": "1s"
    }
}
```

The server streams StateHub projection updates as they arrive:

```json
{
    "projection": "cost_meter",
    "version": 1423,
    "timestamp": "2026-04-25T10:30:00Z",
    "data": {
        "total_usd": 0.0234,
        "budget_remaining": 0.9766,
        "burn_rate_usd_per_hour": 0.047,
        "cost_trend": "stable"
    }
}
```

### Resolution and aggregation

| Resolution | Use case |
|---|---|
| `100ms` | Real-time TUI sparklines |
| `1s` | Web dashboard charts |
| `10s` | Low-bandwidth monitoring |
| `60s` | Slack notifications, historical review |

### TUI rendering

The ratatui dashboard (F5: Telemetry tab) renders StateHub projections directly:

| Widget | Source Projection | Display |
|---|---|---|
| Cost gauge | `cost_meter` | USD spent / budget, projected exhaustion |
| Latency sparkline | (direct Lens) | p50/p95 over time |
| Quality bar | `gate_pipeline` | Pass rate per rung, avg reward |
| Efficiency table | `cohort_health` | Tokens/task, T0/T1/T2 hit rates |
| Error list | (direct Lens) | Recent errors by category |
| Knowledge health | `knowledge_health` | Balance distribution, heuristic calibration |
| Budget/vitality meter | `agent_vitality` | Per-agent vitality phase, burn rate |
| Trend arrows | (direct Lens) | Direction indicators per metric |
| Anomaly alerts | (direct Lens) | Active anomalies with severity |
| c-factor gauge | `c_factor` | Score + component breakdown + trend |

### REST API

Historical Lens data and StateHub projections are queryable via the control plane:

```
GET /api/telemetry/{lens-id}/history?from=...&to=...&resolution=60s
GET /api/statehub/{projection-id}
GET /api/statehub/{projection-id}/history?from=...&to=...&resolution=60s
```

---

## 8. Developer Analytics

For marketplace creators, the UsageLens provides analytics on published Blocks, Graphs, and Racks.

### Tracked metrics

| Metric | Description | Source |
|---|---|---|
| `runs` | Total executions | BlockStarted / GraphStarted events |
| `unique_spaces` | Distinct Spaces using the Block | Event context extraction |
| `installs` | New installs from marketplace | Marketplace install event |
| `forks` | Forks of published Graphs | Marketplace fork event |
| `daily_active` | Unique users in 24h | Deduplication by Space ID |
| `success_rate` | Completions / total starts | Block/Graph lifecycle tracking |
| `avg_duration` | Mean execution time | Duration from lifecycle events |

### Privacy

Analytics are aggregated — individual runs are never exposed to publishers. Only the Block/Graph creator and marketplace administrators see usage data. Individual Spaces can opt out of analytics by setting `telemetry.marketplace = false` in their Space config.

---

## 9. TOML Configuration

### Attaching Lenses to a Graph

```toml
[graph]
name = "plan-executor"
version = "1.0.0"

[[lenses]]
name = "cost-monitor"
block = "roko:cost-lens@^1.0"
scope = "graph"
[lenses.params]
interval = "60s"
budget_warn_pct = 0.80

[[lenses]]
name = "quality-monitor"
block = "roko:quality-lens@^1.0"
scope = "graph"
[lenses.params]
pass_rate_warn = 0.70
track_rungs = true
```

### Attaching Lenses to an Agent

```toml
[agent]
name = "code-agent"
profile = "coding"

[[agent.lenses]]
name = "efficiency"
block = "roko:efficiency-lens@^1.0"
scope = "agent"

[[agent.lenses]]
name = "budget"
block = "roko:budget-lens@^1.0"
scope = "agent"
[agent.lenses.params]
warn_pct = 0.75
```

### Chaining Lenses

```toml
[[lenses]]
name = "cost-monitor"
block = "roko:cost-lens@^1.0"
scope = "graph"

[[lenses]]
name = "cost-trend"
block = "roko:trend-lens@^1.0"
scope = "lens:cost-monitor"
[lenses.params]
metric = "total_usd"
window = "600s"

[[lenses]]
name = "cost-anomaly"
block = "roko:anomaly-lens@^1.0"
scope = "lens:cost-trend"
[lenses.params]
metric = "ema"
sigma_moderate = 3.0
```

### Space-level c-factor Lens

```toml
[space]
name = "my-workspace"

[[space.default_lenses]]
block = "roko:collective-intelligence-lens@^1.0"
scope = "space"
[space.default_lenses.params]
interval = "300s"
min_agents = 2
```

### StateHub configuration

```toml
[statehub]
enabled = true
projection_interval = "1s"            # minimum time between projection updates
history_retention = "7d"               # how long to keep historical projections

[[statehub.projections]]
id = "cohort_health"
sources = ["efficiency-lens", "error-lens", "budget-lens"]

[[statehub.projections]]
id = "c_factor"
sources = ["collective-intelligence-lens"]
```

---

## 10. Custom Lenses

Users create custom Lenses by implementing the Observe protocol on a Block.

### Rust implementation

```rust
use roko_core::{Block, Observe, ObservableEvent, ObservableEventKind, LensScope, Signal};

pub struct SlaComplianceLens {
    target_p99: Duration,
    target_error_rate: f64,
    window: RollingWindow,
}

impl Observe for SlaComplianceLens {
    fn observes(&self) -> &[ObservableEventKind] {
        &[ObservableEventKind::BlockLifecycle]
    }

    fn scope(&self) -> LensScope {
        LensScope::Graph(self.target_graph.clone())
    }

    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        match event {
            ObservableEvent::BlockCompleted { duration, .. } => {
                self.window.record_latency(*duration);
            }
            ObservableEvent::BlockFailed { .. } => {
                self.window.record_error();
            }
            _ => return Ok(vec![]),
        }

        let mut signals = vec![];
        if self.window.p99() > self.target_p99 {
            signals.push(Signal::alert(AlertLevel::Warning, json!({
                "sla": "p99_latency",
                "target": self.target_p99.as_secs_f64(),
                "actual": self.window.p99().as_secs_f64(),
            })));
        }
        Ok(signals)
    }
}
```

Custom Lenses are published to the marketplace like any Block. They declare `protocols = ["observe"]` in their manifest.

---

## 11. Performance

Lenses must not degrade the observed system. The overhead budget is strict.

### Overhead budget

| Constraint | Limit | Enforcement |
|---|---|---|
| Per-event Lens invocation | < 1% of observed Block's execution time | Runtime circuit breaker |
| Total Lens overhead per Block | < 5% (all stacked Lenses combined) | Aggregated timing check |
| Memory per Lens | < 10 MB rolling window | OOM guard per Lens |
| Bus backpressure | Drop-oldest for observation topics | Prevents slow Lens from blocking |

### Circuit breaker

If a Lens consistently exceeds its overhead budget:

1. **First violation**: Log warning, continue.
2. **3 consecutive violations**: Reduce invocation rate (sample 50% of events).
3. **10 consecutive violations**: Disable Lens, emit Alert(Critical) on system Bus, notify operator.
4. **Recovery**: Operator re-enables via config or API. Lens starts in sampled mode.

### Sampling

For high-frequency events, Lenses use sampling:

```toml
[[lenses]]
name = "high-freq-cost"
block = "roko:cost-lens@^1.0"
[lenses.params]
sampling_rate = 0.10
sampling_strategy = "reservoir"
```

Strategies: `reservoir` (statistically representative), `every_nth` (deterministic), `probabilistic` (random), `adaptive` (adjusts to stay within overhead budget).

### Async processing

Lenses run asynchronously — they do not block the observed Block's execution path. The engine captures the event (cheap clone or Arc), dispatches to the Lens async task pool, and continues Block execution immediately.

---

## 12. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| T-1 | Observe trait compiles with `observe()`, `observes()`, `scope()` | `cargo check` on roko-core |
| T-2 | ObservableEvent covers all lifecycle events in this spec | Enum variant count matches spec |
| T-3 | CostLens emits CostReport Signals per configured interval | Integration test |
| T-4 | LatencyLens computes correct p50/p95/p99 from duration samples | Unit test |
| T-5 | QualityLens tracks pass rate per rung and avg continuous reward | Integration test |
| T-6 | EfficiencyLens computes tokens-per-task with vitality phase | Unit test |
| T-7 | ErrorLens classifies errors by category | Unit test |
| T-8 | DriftLens detects balance decrease from demurrage | Integration test |
| T-9 | BudgetLens emits Alert at configured thresholds with vitality | Unit test |
| T-10 | TrendLens computes correct EMA and slope | Unit test |
| T-11 | AnomalyLens flags outliers at configured sigma | Unit test |
| T-12 | CollectiveIntelligenceLens computes c-factor from 4 components | Integration test |
| T-13 | c-factor decreases when one agent dominates turns | Unit test |
| T-14 | Stacking: 3 Lenses on same Graph all receive events | Integration test |
| T-15 | Chaining: TrendLens receives CostLens output | Integration test |
| T-16 | Scope isolation: Graph Lens does not see events from other Graphs | Integration test |
| T-17 | StateHub updates projection on Lens observation Signal | Integration test |
| T-18 | StateHub projection version increments monotonically | Unit test |
| T-19 | Surface subscription at 1s resolution receives coalesced updates | Integration test |
| T-20 | StateHub projections match typed schemas | Schema validation test |
| T-21 | Dashboard WS streams StateHub projections at configured resolution | Integration test |
| T-22 | Circuit breaker disables slow Lens after 10 violations | Unit test |
| T-23 | Sampling reduces event delivery to configured rate | Unit test |
| T-24 | Lens overhead stays under 1% of Block execution time | Benchmark |

---

## 13. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality | [doc-01](01-SIGNAL.md) | §1-3 |
| Observe protocol definition | [doc-02](02-BLOCK.md) | §3.7 |
| Predict-publish-correct | [doc-02](02-BLOCK.md) | §3.10 |
| Verify redesign (continuous reward) | [doc-02](02-BLOCK.md) | §3.3 |
| Vitality and behavioral phases | [doc-07](07-AGENT-RUNTIME.md) | §3 |
| CorticalState | [doc-07](07-AGENT-RUNTIME.md) | §4 |
| L4 c-factor gate | [doc-10](10-LEARNING-LOOPS.md) | §5 |
| Demurrage model | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §3 |
| Five named surfaces | [doc-16](16-SURFACES.md) | — |
