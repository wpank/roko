# 09 — Telemetry: Lens System

> Full observability through the Observe protocol. Lenses focus attention without modifying the subject.

**Depends on**: [01-SIGNAL](01-SIGNAL.md), [02-BLOCK](02-BLOCK.md), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Lens definition)

---

## 1. Overview

Every Block, Graph, Agent, and Space in Roko can be observed without modification. The **Lens** specialization — a Block implementing the Observe protocol — receives read-only lifecycle events and emits observation Signals onto the Bus.

The design follows three principles:

1. **Observation is passive.** A Lens never mutates the subject. Removing all Lenses from a system changes nothing about its behavior — only visibility.
2. **Observation is compositional.** Lenses stack (multiple Lenses on one target), chain (a Lens watches another Lens's output), and scope (Block, Graph, Agent, Space granularity).
3. **Observation uses the same primitives.** Lens output is a Signal. Lens composition is a Graph. Lens configuration is TOML. No special telemetry infrastructure.

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
    SignalDecayed(SignalRef, f64),              // ref + new confidence
    SignalPromoted(SignalRef, Tier, Tier),      // ref + old tier + new tier
    SignalPruned(SignalRef),

    // ── Block lifecycle ──────────────────────────────────────
    BlockStarted { block: BlockRef, run: RunId, input_hash: ContentHash },
    BlockCompleted { block: BlockRef, run: RunId, duration: Duration, cost: Cost },
    BlockFailed { block: BlockRef, run: RunId, error: BlockError },
    BlockRetried { block: BlockRef, run: RunId, attempt: u32, reason: String },
    BlockCancelled { block: BlockRef, run: RunId },

    // ── Graph lifecycle ──────────────────────────────────────
    GraphStarted { graph: GraphRef, run: RunId, input_hash: ContentHash },
    GraphNodeCompleted { graph: GraphRef, run: RunId, node: NodeId, duration: Duration },
    GraphCompleted { graph: GraphRef, run: RunId, duration: Duration, cost: Cost },
    GraphFailed { graph: GraphRef, run: RunId, error: BlockError },
    GraphPaused { graph: GraphRef, run: RunId, reason: PauseReason },
    GraphResumed { graph: GraphRef, run: RunId },

    // ── Agent lifecycle ──────────────────────────────────────
    AgentTick { agent: AgentRef, regime: Regime, prediction_error: f64 },
    AgentRegimeChange { agent: AgentRef, old: Regime, new_regime: Regime },
    AgentBudgetUpdate { agent: AgentRef, spent: Cost, remaining: Cost },
    AgentModeChange { agent: AgentRef, old: AgentMode, new_mode: AgentMode },

    // ── Memory lifecycle ─────────────────────────────────────
    MemoryRetrieved { query: String, results: usize, duration: Duration },
    MemoryStored { signal: SignalRef, tier: Tier },
    MemoryConsolidated { promoted: usize, demoted: usize, pruned: usize },

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
    TriggerLifecycle,
    ExtensionLifecycle,
    All,
}
```

A Lens declaring `observes() = &[ObservableEventKind::BlockLifecycle]` receives only `BlockStarted`, `BlockCompleted`, `BlockFailed`, `BlockRetried`, and `BlockCancelled` events. A Lens declaring `All` receives everything within its scope.

---

## 3. Built-in Lenses

Roko ships 10 built-in Lenses. Each is a Block implementing Observe, packaged with the `roko-core` distribution.

### 3.1 CostLens

Tracks USD and token expenditure across Block executions.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle`, `AgentLifecycle` |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: CostReport }` |

**Observed events**: `BlockCompleted` (extracts `cost`), `GraphCompleted` (aggregates), `AgentBudgetUpdate` (pass-through).

**Emitted Signal payload**:

```rust
pub struct CostReportPayload {
    pub target: String,                // Block/Graph/Agent name
    pub interval: Duration,            // aggregation window
    pub total_usd: f64,                // total USD in interval
    pub total_tokens: u64,             // total tokens (input + output)
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_breakdown: BTreeMap<String, f64>,  // model → USD
    pub cumulative_usd: f64,           // running total since start
    pub budget_remaining: Option<f64>, // if budget set
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 60s | Aggregation window for emitting reports |
| `budget_warn_pct` | f64 | 0.80 | Emit Alert when this % of budget consumed |
| `budget_critical_pct` | f64 | 0.95 | Emit Alert(Critical) at this threshold |
| `include_model_breakdown` | bool | true | Break costs down per model |

---

### 3.2 LatencyLens

Measures execution duration with percentile tracking.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle` |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: Observation }` with latency payload |

**Observed events**: `BlockCompleted` (duration), `BlockFailed` (duration), `GraphNodeCompleted` (per-node duration), `GraphCompleted` (total duration).

**Emitted Signal payload**:

```rust
pub struct LatencyPayload {
    pub target: String,
    pub interval: Duration,
    pub count: u64,                    // number of observations in window
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub mean: Duration,
    pub min: Duration,
    pub max: Duration,
    pub stddev: Duration,
    pub histogram: Vec<HistogramBucket>,  // configurable bucket boundaries
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 30s | Aggregation window |
| `buckets` | Vec<f64> | `[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]` | Histogram bucket boundaries (seconds) |
| `slow_threshold` | Duration | 10s | Emit Alert when p95 exceeds this |

---

### 3.3 QualityLens

Tracks pass/fail rates from Verify-protocol Blocks.

| Property | Value |
|---|---|
| **Observes** | `SignalLifecycle` (specifically `SignalVerified`) |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: Observation }` with quality payload |

**Observed events**: `SignalVerified` (Verdict with pass/fail + confidence).

**Emitted Signal payload**:

```rust
pub struct QualityPayload {
    pub target: String,
    pub interval: Duration,
    pub total_verifications: u64,
    pub passed: u64,
    pub failed: u64,
    pub pass_rate: f64,                // passed / total
    pub avg_confidence: f64,           // mean Verdict.confidence
    pub min_confidence: f64,
    pub findings_by_severity: BTreeMap<Severity, u64>,
    pub rung_breakdown: BTreeMap<String, PassFailCounts>,  // per gate rung
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 60s | Aggregation window |
| `pass_rate_warn` | f64 | 0.70 | Alert when pass rate drops below |
| `pass_rate_critical` | f64 | 0.50 | Alert(Critical) threshold |
| `track_rungs` | bool | true | Break down by gate rung |

---

### 3.4 EfficiencyLens

Measures tokens-per-task and cost-per-quality ratios.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `AgentLifecycle` |
| **Default Scope** | `Agent` |
| **Emits** | `Signal { kind: Observation }` with efficiency payload |

**Observed events**: `BlockCompleted` (cost + duration), `AgentTick` (prediction error), `SignalVerified` (quality).

**Emitted Signal payload**:

```rust
pub struct EfficiencyPayload {
    pub agent: String,
    pub interval: Duration,
    pub tasks_completed: u64,
    pub tokens_per_task: f64,          // avg tokens per completed task
    pub usd_per_task: f64,             // avg USD per completed task
    pub quality_per_usd: f64,          // pass_rate / total_usd
    pub tokens_per_quality: f64,       // tokens / pass_rate
    pub t0_hit_rate: f64,              // % of ticks handled by reflex
    pub t1_hit_rate: f64,              // % handled by lightweight model
    pub t2_hit_rate: f64,              // % requiring full model
    pub avg_prediction_error: f64,
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 120s | Aggregation window |
| `tokens_per_task_warn` | u64 | 5000 | Alert when avg exceeds this |
| `track_tiers` | bool | true | Break down by T0/T1/T2 |

---

### 3.5 ErrorLens

Classifies and aggregates errors across Block executions.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle`, `ExtensionLifecycle` |
| **Default Scope** | `Graph` |
| **Emits** | `Signal { kind: Observation }` with error report payload |

**Observed events**: `BlockFailed`, `BlockRetried`, `GraphFailed`, `ExtensionHookFailed`.

**Emitted Signal payload**:

```rust
pub struct ErrorPayload {
    pub target: String,
    pub interval: Duration,
    pub total_errors: u64,
    pub by_category: BTreeMap<ErrorCategory, u64>,
    pub by_block: BTreeMap<String, u64>,
    pub retry_count: u64,
    pub retry_success_rate: f64,       // retries that eventually succeeded
    pub error_rate: f64,               // errors / total executions
    pub most_common: Vec<ErrorSummary>, // top N error patterns
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

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 60s | Aggregation window |
| `error_rate_warn` | f64 | 0.10 | Alert when error rate exceeds 10% |
| `error_rate_critical` | f64 | 0.30 | Alert(Critical) threshold |
| `top_n` | usize | 5 | Number of most common errors to track |

---

### 3.6 DriftLens

Detects knowledge quality degradation in Memory stores.

| Property | Value |
|---|---|
| **Observes** | `MemoryLifecycle`, `SignalLifecycle` |
| **Default Scope** | `Agent` (specifically the Agent's Memory) |
| **Emits** | `Signal { kind: Observation }` with drift payload |

**Observed events**: `MemoryConsolidated`, `MemoryRetrieved`, `SignalDecayed`, `SignalPromoted`, `SignalPruned`.

**Emitted Signal payload**:

```rust
pub struct DriftPayload {
    pub memory: String,                // Memory Block name
    pub interval: Duration,
    pub total_entries: u64,
    pub tier_distribution: BTreeMap<Tier, u64>,   // count per tier
    pub avg_confidence: f64,           // across all entries
    pub confidence_delta: f64,         // change since last interval
    pub promotion_rate: f64,           // promotions per interval
    pub demotion_rate: f64,            // demotions per interval
    pub pruning_rate: f64,             // prunings per interval
    pub retrieval_hit_rate: f64,       // queries that returned results
    pub staleness_score: f64,          // 0.0 (fresh) to 1.0 (stale)
    pub anti_knowledge_count: u64,     // active AntiKnowledge entries
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 300s | Aggregation window (longer for slow-changing data) |
| `staleness_warn` | f64 | 0.60 | Alert when staleness exceeds threshold |
| `staleness_critical` | f64 | 0.85 | Alert(Critical) threshold |
| `min_confidence` | f64 | 0.30 | Warn when avg confidence drops below |

---

### 3.7 BudgetLens

Monitors budget consumption across Agents and Spaces.

| Property | Value |
|---|---|
| **Observes** | `AgentLifecycle`, `BlockLifecycle` |
| **Default Scope** | `Agent` or `Space` |
| **Emits** | `Signal { kind: Alert }` when thresholds crossed |

**Observed events**: `AgentBudgetUpdate`, `BlockCompleted` (cost).

**Emitted Signal payload**:

```rust
pub struct BudgetAlertPayload {
    pub target: String,                // Agent or Space name
    pub budget_total: f64,             // total allocated USD
    pub budget_spent: f64,             // consumed so far
    pub budget_remaining: f64,
    pub pct_consumed: f64,             // 0.0 to 1.0
    pub projected_exhaustion: Option<DateTime<Utc>>,  // ETA at current burn rate
    pub burn_rate: f64,                // USD per hour
    pub level: AlertLevel,             // Info / Warning / Critical
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `warn_pct` | f64 | 0.75 | Emit Alert(Warning) at this % consumed |
| `critical_pct` | f64 | 0.90 | Emit Alert(Critical) at this % |
| `halt_pct` | f64 | 0.99 | Emit Alert(Critical) + recommend pause |
| `project_exhaustion` | bool | true | Calculate projected exhaustion time |

---

### 3.8 TrendLens

Computes statistical trends over time-series observation data.

| Property | Value |
|---|---|
| **Observes** | Any other Lens's output (chaining Lens) |
| **Default Scope** | `Lens` (wraps another Lens) |
| **Emits** | `Signal { kind: Trend }` |

**Observed events**: Any observation Signal emitted by the chained source Lens.

**Emitted Signal payload**:

```rust
pub struct TrendPayload {
    pub source_lens: String,           // which Lens this trend is computed from
    pub metric: String,                // which field in source payload
    pub window: Duration,              // trend computation window
    pub slope: f64,                    // linear regression slope
    pub slope_pct: f64,               // slope as % of mean
    pub ema: f64,                      // exponential moving average (current)
    pub ema_previous: f64,             // EMA at previous interval
    pub ema_delta: f64,                // ema - ema_previous
    pub direction: TrendDirection,     // Rising, Falling, Stable
    pub r_squared: f64,               // goodness of fit
    pub data_points: usize,           // observations in window
}

pub enum TrendDirection {
    Rising,        // slope > +threshold
    Falling,       // slope < -threshold
    Stable,        // within threshold
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `window` | Duration | 600s | Time window for trend computation |
| `ema_alpha` | f64 | 0.3 | EMA smoothing factor (higher = more responsive) |
| `slope_threshold` | f64 | 0.05 | % change per interval to count as Rising/Falling |
| `metric` | String | (required) | Which numeric field in the source payload to track |
| `min_data_points` | usize | 5 | Minimum observations before emitting trends |

---

### 3.9 AnomalyLens

Detects statistical outliers in observation streams.

| Property | Value |
|---|---|
| **Observes** | Any other Lens's output (chaining Lens) |
| **Default Scope** | `Lens` (wraps another Lens) |
| **Emits** | `Signal { kind: Anomaly }` |

**Observed events**: Any observation Signal emitted by the chained source Lens.

**Emitted Signal payload**:

```rust
pub struct AnomalyPayload {
    pub source_lens: String,
    pub metric: String,
    pub observed_value: f64,
    pub expected_value: f64,           // EMA or mean
    pub deviation: f64,                // in standard deviations (z-score)
    pub direction: AnomalyDirection,
    pub severity: AnomalyLevel,
    pub context: BTreeMap<String, Value>,  // surrounding metric values
}

pub enum AnomalyDirection {
    Above,         // value significantly above expected
    Below,         // value significantly below expected
}

pub enum AnomalyLevel {
    Mild,          // 2-3 sigma
    Moderate,      // 3-4 sigma
    Severe,        // 4+ sigma
}
```

**Detection method**: The AnomalyLens maintains a rolling window of observations, computes mean and standard deviation, and flags values exceeding the configured sigma threshold. For non-Gaussian distributions, it uses the IQR method as a fallback (values beyond `Q1 - 1.5*IQR` or `Q3 + 1.5*IQR`).

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `metric` | String | (required) | Which numeric field to monitor |
| `sigma_mild` | f64 | 2.0 | Standard deviations for Mild |
| `sigma_moderate` | f64 | 3.0 | Standard deviations for Moderate |
| `sigma_severe` | f64 | 4.0 | Standard deviations for Severe |
| `window_size` | usize | 100 | Rolling window for baseline statistics |
| `min_samples` | usize | 10 | Minimum observations before anomaly detection activates |
| `use_iqr_fallback` | bool | true | Fall back to IQR for non-Gaussian data |

---

### 3.10 UsageLens

Tracks usage analytics for marketplace and developer metrics.

| Property | Value |
|---|---|
| **Observes** | `BlockLifecycle`, `GraphLifecycle`, `TriggerLifecycle` |
| **Default Scope** | `Space` or `Global` (marketplace-wide) |
| **Emits** | `Signal { kind: Observation }` with usage payload |

**Observed events**: `BlockStarted` (run count), `GraphStarted` (run count), `TriggerFired` (trigger count).

**Emitted Signal payload**:

```rust
pub struct UsagePayload {
    pub target: String,                // Block, Graph, or Space name
    pub target_kind: UsageTargetKind,  // Block | Graph | Rack | Agent
    pub interval: Duration,
    pub runs: u64,                     // total executions in interval
    pub unique_spaces: u64,            // distinct Spaces that ran it
    pub unique_agents: u64,            // distinct Agents that used it
    pub installs: u64,                 // new installs (marketplace)
    pub forks: u64,                    // forks of published Graph
    pub avg_duration: Duration,        // mean execution time
    pub success_rate: f64,             // completions / total starts
    pub daily_active: u64,             // unique users in 24h (marketplace)
}

pub enum UsageTargetKind {
    Block,
    Graph,
    Rack,
    Agent,
}
```

**Configurable parameters**:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `interval` | Duration | 3600s | Aggregation window (hourly by default) |
| `track_marketplace` | bool | false | Include install/fork/daily_active metrics |
| `track_agents` | bool | true | Track per-agent breakdown |

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
    │CostLens│ ───▶ │TrendLens│ ───▶ │Anomaly   │
    │        │      │(cost $) │      │Lens      │
    └────────┘      └─────────┘      └──────────┘
       emits           emits            emits
     CostReport       Trend            Anomaly
      Signals         Signals          Signals
```

In this chain:
- CostLens observes Block executions, emits CostReport Signals
- TrendLens observes CostLens output, computes slope and EMA over cost
- AnomalyLens observes TrendLens output, flags when cost trends deviate

Chaining uses `LensScope::Lens(ref)`. The engine ensures delivery order: upstream Lens output is delivered to downstream Lens before the next event cycle.

### 4.3 Scoping (Block, Graph, Agent, Space levels)

The same Lens type can be attached at different scope levels:

| Scope | Receives events from | Aggregation |
|---|---|---|
| `Block` | Single Block only | Per-invocation metrics |
| `Graph` | All Blocks within one Graph | Aggregated across nodes |
| `Agent` | All Graphs within one Agent | Cross-pipeline view |
| `Space` | All Agents within one Space | Full workspace view |
| `Global` | Everything in the system | System-wide overview |

A CostLens at Block scope emits per-invocation cost. The same CostLens at Agent scope emits aggregated cost across all the Agent's Graphs.

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
(BlockCompleted,                 │  │ Dashboard (WS)       │         │
 GraphFailed, etc.)              │  │ real-time rendering   │         │
    │                            │  └─────────────────────┘         │
    ▼                            │                                   │
Engine routes to                 │  ┌─────────────────────┐         │
matching Lenses                  │  │ Episode Logger        │         │
    │                            │  │ .roko/episodes.jsonl  │         │
    ▼                            │  └─────────────────────┘         │
Lens.observe(event)              │                                   │
    │                            │  ┌─────────────────────┐         │
    ▼                            │  │ Learning Loops        │         │
Observation Signals  ──► Bus ──►─┤  │ (see doc-10)         │         │
(Observation, Alert,             │  └─────────────────────┘         │
 Trend, Anomaly,                 │                                   │
 CostReport kinds)               │  ┌─────────────────────┐         │
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

1. **Event generation**: A Block completes execution. The engine emits a `BlockCompleted` event as an ephemeral Signal on the Bus topic `block:{id}:events`.

2. **Event routing**: The engine's Lens routing table matches the event's kind and scope to all registered Lenses. Only matching Lenses receive the event.

3. **Lens invocation**: Each matching Lens's `observe()` method is called with the event. The Lens processes the event and returns zero or more observation Signals.

4. **Signal emission**: The returned observation Signals are published to the Bus on the topic `lens:{id}:observations`.

5. **Consumer delivery**: Bus subscribers consume observation Signals:
   - **Dashboard**: WebSocket bridge subscribes to all `lens:*:observations` topics, streams to connected clients.
   - **Episode Logger**: Subscribes to observation Signals tagged `persist`, appends to `.roko/episodes.jsonl`.
   - **Learning Loops**: Loop Graphs subscribe to specific observation types (e.g., CostReport for the cost-optimization Loop).
   - **Store**: A React Block persists selected observation Signals for historical query.
   - **Chained Lenses**: Downstream Lenses subscribed via `LensScope::Lens` receive observation Signals as events.

### Delivery guarantees

| Consumer | Guarantee | Rationale |
|---|---|---|
| Dashboard (WS) | At-most-once | Dropped frames are acceptable for real-time display |
| Episode Logger | At-least-once | Episodes must not be lost |
| Learning Loops | At-least-once | Learning requires complete data |
| Store | At-least-once | Historical queries need completeness |
| Chained Lenses | Exactly-once (within tolerance) | Chain ordering requires delivery |

---

## 6. Dashboard Integration

The dashboard (TUI, web, or visual editor) receives Lens output via WebSocket streaming.

### WebSocket protocol

The control plane (`roko serve` on :6677) exposes a WebSocket endpoint for live telemetry:

```
GET /ws/telemetry
```

The client sends a subscription message:

```json
{
    "subscribe": {
        "lenses": ["cost-lens-main", "latency-lens-main"],
        "scope": "graph:plan-executor",
        "resolution": "1s"
    }
}
```

The server streams observation Signals as they arrive:

```json
{
    "lens": "cost-lens-main",
    "timestamp": "2026-04-25T10:30:00Z",
    "payload": {
        "target": "plan-executor",
        "total_usd": 0.0234,
        "total_tokens": 4521,
        "budget_remaining": 0.9766
    }
}
```

### Resolution and aggregation

Clients specify a `resolution` (minimum interval between updates). The WebSocket bridge aggregates observation Signals within each resolution window:

| Resolution | Use case |
|---|---|
| `100ms` | Real-time TUI sparklines |
| `1s` | Web dashboard charts |
| `10s` | Low-bandwidth monitoring |
| `60s` | Historical review |

The bridge coalesces Signals within the window, sending the latest value for each Lens.

### TUI rendering

The ratatui dashboard (F5: Telemetry tab) renders Lens output directly:

| Widget | Source Lens | Display |
|---|---|---|
| Cost gauge | CostLens | USD spent / budget, projected exhaustion |
| Latency sparkline | LatencyLens | p50/p95 over time |
| Quality bar | QualityLens | Pass rate per rung |
| Efficiency table | EfficiencyLens | Tokens/task, T0/T1/T2 hit rates |
| Error list | ErrorLens | Recent errors by category |
| Drift indicator | DriftLens | Staleness score, tier distribution |
| Budget meter | BudgetLens | Remaining %, burn rate |
| Trend arrows | TrendLens | Direction indicators per metric |
| Anomaly alerts | AnomalyLens | Active anomalies with severity |

### REST API

Historical Lens data is queryable via the control plane:

```
GET /api/telemetry/{lens-id}/history?from=2026-04-25T00:00:00Z&to=2026-04-25T12:00:00Z&resolution=60s
```

Returns time-bucketed aggregates for dashboard charts and reporting.

---

## 7. Developer Analytics

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

### Developer dashboard

Marketplace publishers see a dedicated analytics view:

```
╔══════════════════════════════════════════════╗
║ code-review v1.2.0                          ║
║─────────────────────────────────────────────║
║ Installs: 142   Forks: 23   Active: 67/day ║
║ Runs: 4,521     Success: 94.2%              ║
║ Avg Duration: 12.3s   Avg Cost: $0.034      ║
║─────────────────────────────────────────────║
║ [Runs over time ▁▂▃▅▆▇█▇▆▅]              ║
║ [Success rate   ▆▆▇▇▇▆▇▇▇▆]              ║
╚══════════════════════════════════════════════╝
```

### Privacy

Analytics are aggregated — individual runs are never exposed to publishers. Only the Block/Graph creator and marketplace administrators see usage data. Individual Spaces can opt out of analytics by setting `telemetry.marketplace = false` in their Space config.

---

## 8. TOML Configuration

Lenses are attached to Blocks, Graphs, and Agents via TOML config.

### Attaching Lenses to a Graph

```toml
[graph]
name = "plan-executor"
version = "1.0.0"

# Attach Lenses at Graph scope
[[lenses]]
name = "cost-monitor"
block = "roko:cost-lens@^1.0"
scope = "graph"
[lenses.params]
interval = "60s"
budget_warn_pct = 0.80
budget_critical_pct = 0.95

[[lenses]]
name = "latency-monitor"
block = "roko:latency-lens@^1.0"
scope = "graph"
[lenses.params]
interval = "30s"
slow_threshold = "10s"

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
[agent.lenses.params]
interval = "120s"
track_tiers = true

[[agent.lenses]]
name = "budget"
block = "roko:budget-lens@^1.0"
scope = "agent"
[agent.lenses.params]
warn_pct = 0.75
critical_pct = 0.90
```

### Attaching a Block-level Lens

```toml
[[nodes]]
id = "llm-call"
type = "block"
block = "roko:claude-api@^1.0"

[[nodes.lenses]]
name = "llm-cost"
block = "roko:cost-lens@^1.0"
scope = "block"
[nodes.lenses.params]
interval = "10s"
include_model_breakdown = true
```

### Chaining Lenses

```toml
# First: CostLens observes the Graph
[[lenses]]
name = "cost-monitor"
block = "roko:cost-lens@^1.0"
scope = "graph"

# Second: TrendLens chains on CostLens output
[[lenses]]
name = "cost-trend"
block = "roko:trend-lens@^1.0"
scope = "lens:cost-monitor"        # scope references the upstream Lens
[lenses.params]
metric = "total_usd"
window = "600s"
ema_alpha = 0.3

# Third: AnomalyLens chains on TrendLens output
[[lenses]]
name = "cost-anomaly"
block = "roko:anomaly-lens@^1.0"
scope = "lens:cost-trend"          # chains on the TrendLens
[lenses.params]
metric = "ema"
sigma_moderate = 3.0
window_size = 50
```

### Space-level default Lenses

Spaces can define default Lenses applied to all Graphs within the Space:

```toml
[space]
name = "my-workspace"

[[space.default_lenses]]
block = "roko:error-lens@^1.0"
scope = "graph"                    # applied to every Graph in this Space
[space.default_lenses.params]
error_rate_warn = 0.10

[[space.default_lenses]]
block = "roko:budget-lens@^1.0"
scope = "space"
[space.default_lenses.params]
warn_pct = 0.80
```

Graph-level Lenses override Space defaults when both are present for the same Lens type.

---

## 9. Custom Lenses

Users create custom Lenses by implementing the Observe protocol on a Block.

### Rust implementation

```rust
use roko_core::{Block, Observe, ObservableEvent, ObservableEventKind, LensScope, Signal};

pub struct SlaComplianceLens {
    target_p99: Duration,
    target_error_rate: f64,
    window: RollingWindow,
}

impl Block for SlaComplianceLens {
    fn name(&self) -> &str { "sla-compliance-lens" }
    fn version(&self) -> &Version { &Version::new(1, 0, 0) }
    fn description(&self) -> &str { "Monitors SLA compliance for p99 latency and error rate" }
    fn tags(&self) -> &[&str] { &["lens", "sla", "compliance"] }
    fn input_schema(&self) -> &TypeSchema { &TypeSchema::Signal { kind: None } }
    fn output_schema(&self) -> &TypeSchema { &TypeSchema::Signal { kind: Some(Kind::Observation) } }
    fn capabilities(&self) -> &[Capability] { &[] }  // Lenses need no capabilities
    fn protocols(&self) -> &[Protocol] { &[Protocol::Observe] }

    async fn run(&self, input: BlockInput, ctx: &BlockContext) -> Result<BlockOutput, BlockError> {
        // Lenses are invoked via observe(), not run().
        // run() can be a no-op or delegate to observe().
        Ok(BlockOutput::empty())
    }
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

        // Check SLA compliance
        if self.window.p99() > self.target_p99 {
            signals.push(Signal::new(Kind::Alert {
                level: AlertLevel::Warning,
            }).with_payload(json!({
                "sla": "p99_latency",
                "target": self.target_p99.as_secs_f64(),
                "actual": self.window.p99().as_secs_f64(),
                "breach": true,
            })));
        }

        if self.window.error_rate() > self.target_error_rate {
            signals.push(Signal::new(Kind::Alert {
                level: AlertLevel::Critical,
            }).with_payload(json!({
                "sla": "error_rate",
                "target": self.target_error_rate,
                "actual": self.window.error_rate(),
                "breach": true,
            })));
        }

        Ok(signals)
    }
}
```

### WASM implementation

Custom Lenses can be compiled to WASM for sandboxed, portable distribution:

```rust
// Compiled to .wasm via wasm32-wasi target
// The Block/Observe interface remains identical
// The engine invokes via wasmtime with fuel metering

#[wasm_bindgen]
pub fn observe(event_json: &str) -> String {
    let event: ObservableEvent = serde_json::from_str(event_json).unwrap();
    let signals = my_lens_logic(&event);
    serde_json::to_string(&signals).unwrap()
}
```

### Script implementation

For rapid prototyping, Lenses can be shell scripts:

```toml
# lens-manifest.toml
[block]
name = "my-custom-lens"
version = "0.1.0"
type = "script"
runtime = "python"
entry = "lens.py"
protocols = ["observe"]
observes = ["block-lifecycle"]
```

```python
# lens.py — receives JSON on stdin, emits JSON on stdout
import json, sys

for line in sys.stdin:
    event = json.loads(line)
    if event["type"] == "BlockCompleted":
        duration = event["duration_ms"]
        if duration > 5000:
            print(json.dumps({
                "kind": "Alert",
                "level": "Warning",
                "payload": {"slow_block": event["block"], "duration_ms": duration}
            }))
            sys.stdout.flush()
```

### Publishing custom Lenses

Custom Lenses are published to the marketplace like any Block. They declare `protocols = ["observe"]` in their manifest and include documentation of what events they observe and what Signals they emit.

---

## 10. Performance

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

For high-frequency events (Block executions at 100+ per second), Lenses use sampling:

```toml
[[lenses]]
name = "high-freq-cost"
block = "roko:cost-lens@^1.0"
[lenses.params]
sampling_rate = 0.10              # observe 10% of events
sampling_strategy = "reservoir"   # statistically representative sample
```

Sampling strategies:

| Strategy | Behavior |
|---|---|
| `reservoir` | Reservoir sampling — statistically representative subset |
| `every_nth` | Deterministic: every Nth event |
| `probabilistic` | Random: each event has P(sampling_rate) of being observed |
| `adaptive` | Adjusts rate to stay within overhead budget |

### Async processing

Lenses run asynchronously — they do not block the observed Block's execution path. The engine:

1. Captures the event (cheap — clone or Arc)
2. Dispatches to Lens async task pool
3. Continues Block execution immediately
4. Lens processes event and publishes observation Signals in the background

This ensures that even a slow Lens cannot increase Block latency.

---

## 11. Mapping to Existing Code

| Spec concept | Existing code | Status |
|---|---|---|
| CostLens aggregation | `CFactorSummary` in orchestrate.rs | **Wired** — computes per-task metrics |
| EfficiencyLens | `.roko/learn/efficiency.jsonl` | **Wired** — per-turn efficiency events |
| LatencyLens data | Gate pipeline timing in orchestrate.rs | **Partial** — timing recorded but not aggregated |
| QualityLens data | Gate pass/fail in orchestrate.rs | **Wired** — adaptive gate thresholds track pass rates |
| BudgetLens | Budget tracking in orchestrate.rs | **Wired** — budget warnings emitted |
| DriftLens | `roko-neuro` tier tracking | **Partial** — tier data exists but no drift computation |
| TrendLens | EMA in `gate-thresholds.json` | **Partial** — EMA computed for gates only |
| AnomalyLens | — | **Not built** |
| UsageLens | — | **Not built** |
| Dashboard WS | `roko-serve` SSE routes | **Partial** — SSE exists; WS telemetry endpoint needed |
| Lens TOML config | — | **Not built** — Lenses are currently hardcoded |

---

## 12. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| T-1 | Observe trait compiles with `observe()`, `observes()`, `scope()` | `cargo check` on roko-core |
| T-2 | ObservableEvent covers all lifecycle events in this spec | Enum variant count matches spec |
| T-3 | CostLens emits CostReport Signals per configured interval | Integration test: run Graph, check CostReport on Bus |
| T-4 | LatencyLens computes correct p50/p95/p99 from duration samples | Unit test: feed known durations, verify percentiles |
| T-5 | QualityLens tracks pass rate per rung | Integration test: run gate pipeline, check QualityLens output |
| T-6 | EfficiencyLens computes tokens-per-task correctly | Unit test: feed BlockCompleted events, verify ratio |
| T-7 | ErrorLens classifies errors by category | Unit test: feed mixed BlockFailed events, verify categorization |
| T-8 | DriftLens detects staleness increase | Integration test: decay Signals, verify staleness_score rises |
| T-9 | BudgetLens emits Alert at configured thresholds | Unit test: simulate budget consumption, verify Alert at 75%/90% |
| T-10 | TrendLens computes correct EMA and slope | Unit test: feed linear series, verify slope ≈ true slope |
| T-11 | AnomalyLens flags outliers at configured sigma | Unit test: feed Gaussian + outlier, verify Anomaly |
| T-12 | UsageLens tracks run counts and unique Spaces | Integration test: run Block from 2 Spaces, verify counts |
| T-13 | Stacking: 3 Lenses on same Graph all receive events | Integration test: attach 3 Lenses, verify all emit |
| T-14 | Chaining: TrendLens receives CostLens output | Integration test: chain, verify Trend Signals appear |
| T-15 | Scope isolation: Graph Lens does not see events from other Graphs | Integration test: 2 Graphs, 1 Lens per, verify no cross-talk |
| T-16 | Dashboard WS streams Lens output at configured resolution | Integration test: connect WS, verify streaming |
| T-17 | Circuit breaker disables slow Lens after 10 violations | Unit test: simulate slow Lens, verify disablement |
| T-18 | Sampling reduces event delivery to configured rate | Unit test: 100 events with rate=0.1, verify ~10 observed |
| T-19 | Custom Lens (Rust) compiles and integrates via manifest | Integration test: build custom Lens, attach, verify output |
| T-20 | TOML config for Lenses loads and resolves correctly | Unit test: parse TOML with stacked + chained Lenses |
| T-21 | Lens overhead stays under 1% of Block execution time | Benchmark: attach CostLens to 1ms Block, verify < 10us overhead |
