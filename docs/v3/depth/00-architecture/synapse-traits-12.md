# The 12 Kernel Traits

> **v3 depth file** -- `/docs/v3/depth/00-architecture/synapse-traits-12.md`
> Canonical source: v1 `docs/v1/00-architecture/06-synapse-traits.md`
> Implementation: `crates/roko-core/src/traits.rs`
> Status: **Shipping** -- all 12 traits are defined and implemented. The trait names in code
> are Store, ColdStore, Score, Verify, Route, Compose, React, Bus, Observe, Connect, Trigger,
> and Substrate (blanket alias for Store).

---

## 1. The Composition Model

Roko's kernel is built from a small, composable operator vocabulary. The complete grammar is:

- **Two mediums**: durable **Signal** (backed by the `Engram` struct) and ephemeral **Pulse**
- **Two fabrics**: storage-oriented **Store** and transport-oriented **Bus**
- **Six operators**: Score, Verify, Route, Compose, React, plus the observation/connection
  protocols
- **Three protocol traits**: Observe, Connect, Trigger

The kernel story is two mediums, two fabrics, and twelve traits that compose to express every
capability in the system.

### 1.1 Why 12 Traits?

The original "one noun, six verbs" mnemonic was useful shorthand but understated the real
surface. The runtime has:

1. **Store** and **ColdStore** for hot and cold durable storage
2. **Bus** for ephemeral transport
3. **Score**, **Verify**, **Route**, **Compose**, **React** for the five non-fabric operators
4. **Observe**, **Connect**, **Trigger** for external integration protocols
5. **Substrate** as a blanket alias for backward compatibility

### 1.2 What Still Holds from v1

1. The operator set is compact and composable.
2. Strict downward layering is the correct dependency rule.
3. Gamma, Theta, and Delta are still the three-speed model.
4. Neuro, Daimon, and Dreams are cross-cuts, not loop steps.
5. All kernel traits extend the `Cell` trait (where applicable), making them composable
   units in the Graph execution engine.

---

## 2. Complete Trait Table

| # | Trait | Rust name | Core job | Primary layer | Input | Output |
|---|---|---|---|---|---|---|
| 1 | Store | `Store` | Persist and query durable Signals | L0 Runtime | Signal, Query, ContentHash | ContentHash, Option<Signal>, Vec<Signal> |
| 2 | ColdStore | `ColdStore` | Archive aged-out Signals | L0 Runtime | Signal, ContentHash | ContentHash, Option<Signal> |
| 3 | Score | `Score` | Rate along multi-axis criteria | L1-L2 | Signal or Datum, Context | ScoreValue |
| 4 | Verify | `Verify` | Verify against external reality | L3 Harness | Signal or Pulse window, Context | Verdict |
| 5 | Route | `Route` | Choose among candidates | L1 Framework | Slice of Signals or Pulses, Context | Option<Selection> |
| 6 | Compose | `Compose` | Combine under Budget | L2 Scaffold | Slice of Signals or Datums, Budget, Scorer, Context | Signal |
| 7 | React | `React` | React to streams and outcomes | L3-L4 | Signal stream and/or Pulse stream, Context | Vec<Signal> or PolicyOutputs |
| 8 | Bus | `Bus` | Publish and subscribe Pulses | L0 Runtime | Pulse, TopicFilter | u64 (seq), Receiver |
| 9 | Observe | `Observe` | Passive data collection | L1 | (environment) | Vec<Signal> |
| 10 | Connect | `Connect` | Manage external connections | L1 | () | Result<()>, bool |
| 11 | Trigger | `Trigger` | Armed conditions that fire | L1 | () | Result<()> |
| 12 | Substrate | `Substrate` | Legacy alias for Store | L0 | (same as Store) | (same as Store) |

---

## 3. Store -- Durable Storage (Trait #1)

```rust
#[async_trait]
pub trait Store: Send + Sync {
    /// Store a signal. Returns its content hash. Idempotent on content.
    async fn put(&self, signal: Signal) -> Result<ContentHash>;

    /// Retrieve a signal by content hash. Does not apply decay.
    async fn get(&self, id: &ContentHash) -> Result<Option<Signal>>;

    /// Query for signals matching the given filter.
    async fn query(&self, q: &Query, ctx: &Context) -> Result<Vec<Signal>>;

    /// Query by HDC similarity against a fingerprint.
    async fn query_similar(
        &self, fp: &HdcVector, radius: f32, limit: usize, ctx: &Context,
    ) -> Result<Vec<(ContentHash, f32)>> { Ok(Vec::new()) }

    /// Remove signals whose effective weight has fallen below threshold.
    async fn prune(&self, threshold: f32, ctx: &Context) -> Result<usize>;

    /// Total count of stored signals.
    async fn len(&self) -> Result<usize> { Ok(0) }

    /// Is the store empty?
    async fn is_empty(&self) -> Result<bool> { Ok(self.len().await? == 0) }

    /// Human-readable name.
    fn name(&self) -> &'static str { "unnamed_store" }
}
```

**Implementors**: `MemorySubstrate` (in-memory HashMap for testing), `FileSubstrate`
(JSONL persistence in `roko-fs`).

**Key properties**:
- `put()` is idempotent on content hash.
- `query_similar()` provides native HDC similarity search (10,240-bit HdcVector fingerprints).
- `prune()` removes Signals whose `score.effective() * decay.apply(age)` falls below threshold.
- All implementations are `Send + Sync`.

---

## 4. ColdStore -- Archival Storage (Trait #2)

```rust
#[async_trait]
pub trait ColdStore: Send + Sync {
    /// Archive a signal into cold storage.
    async fn archive(&self, signal: Signal) -> Result<ContentHash>;

    /// Archive a batch of signals.
    async fn archive_batch(&self, signals: Vec<Signal>) -> Result<usize>;

    /// Retrieve a signal from cold storage (potentially slow).
    async fn thaw(&self, id: &ContentHash) -> Result<Option<Signal>>;

    /// Check existence without full load.
    async fn contains(&self, id: &ContentHash) -> Result<bool>;

    /// Total count of archived signals.
    async fn archived_count(&self) -> Result<usize> { Ok(0) }

    /// Approximate storage size in bytes.
    async fn storage_bytes(&self) -> Result<u64> { Ok(0) }

    /// Purge signals older than epoch.
    async fn purge_before(&self, epoch_ms: i64) -> Result<usize> { Ok(0) }

    fn name(&self) -> &'static str { "unnamed_cold_store" }
}
```

**Implementors**: `ArchiveColdSubstrate` in `roko-fs` (compressed JSONL archives).

**Migration flow**: `Store (hot) --archive()--> ColdStore (cold) --thaw()--> Store (hot)`

---

## 5. Score -- Assessment (Trait #3)

```rust
pub trait Score: Cell + Send + Sync {
    /// Score a persisted signal in context.
    fn score(&self, signal: &Signal, ctx: &Context) -> ScoreValue;

    /// Alias for score -- explicit about input type.
    fn score_signal(&self, signal: &Signal, ctx: &Context) -> ScoreValue {
        self.score(signal, ctx)
    }

    /// Score an ephemeral pulse by promoting to synthetic signal.
    fn score_pulse(&self, p: &Pulse, ctx: &Context) -> ScoreValue {
        let synthetic = Signal::from_pulse_synthetic(p);
        self.score(&synthetic, ctx)
    }

    /// Score either medium via Datum dispatch.
    fn score_datum(&self, datum: Datum<'_>, ctx: &Context) -> ScoreValue {
        match datum {
            Datum::Signal(e) => self.score(e, ctx),
            Datum::Pulse(p) => self.score_pulse(p, ctx),
        }
    }

    fn name(&self) -> &'static str { "unnamed_scorer" }
}
```

**Implementors**: `RelevanceScorer`, `RecencyScorer`, `ReputationScorer`, `CatalyticScorer`,
`CompositeScorer` (in `roko-std`).

Scorers are pure functions of `(Signal, Context) -> ScoreValue`. They compose freely via
addition (aggregation) and multiplication (scaling).

---

## 6. Verify -- Gate Verification (Trait #4)

```rust
#[async_trait]
pub trait Verify: Cell + Send + Sync {
    /// Verify a signal and return a verdict.
    async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict;

    /// Verify a batch of ephemeral pulses.
    async fn verify_stream(&self, pulses: &[Pulse], ctx: &Context) -> Verdict {
        let synthetic = Signal::from_pulses(pulses);
        self.verify(&synthetic, ctx).await
    }

    fn name(&self) -> &str;
}
```

**Implementors**: 19 gates in `roko-gate` organized in a 7-rung pipeline. Examples:
`CompileGate`, `TestGate`, `ClippyGate`, `DiffGate`, `FormatGate`, `SchemaGate`,
`LlmJudgeGate`.

**Key invariant**: A Verdict is always a durable audit artifact. Even when a gate verifies
a live Pulse window, the result persists as a GateVerdict Signal.

**Stream gates** exist for temporal criteria: `BudgetGate` (token usage), `SafetyGate`
(approval sequencing), `LivenessGate` (silence detection).

---

## 7. Route -- Selection (Trait #5)

```rust
pub trait Route: Cell + Send + Sync {
    /// Select one signal from candidates.
    fn select(&self, candidates: &[Signal], ctx: &Context) -> Option<Selection>;

    /// Alias for select -- explicit about input type.
    fn select_signal(&self, candidates: &[Signal], ctx: &Context) -> Option<Selection> {
        self.select(candidates, ctx)
    }

    /// Select from ephemeral pulse candidates.
    fn select_pulse(&self, _candidates: &[Pulse], _ctx: &Context) -> Option<Selection> {
        None
    }

    /// Learn from a selection's actual outcome.
    fn feedback(&self, outcome: &Outcome);

    fn name(&self) -> &str;
}
```

**Implementors**: `StaticRouter` (config-driven), `LinUCBRouter` (contextual bandit),
`CascadeRouter` (multi-stage confidence + UCB), `WeightedRouter` (softmax).

**Key property**: Routers learn via `feedback()`, improving selection quality over time.
The CascadeRouter persists its state to `.roko/learn/cascade-router.json`.

---

## 8. Compose -- Bounded Assembly (Trait #6)

```rust
pub trait Compose: Cell + Send + Sync {
    /// Combine input signals into a new composed signal under budget.
    fn compose(
        &self,
        signals: &[Signal],
        budget: &Budget,
        scorer: &dyn Score,
        ctx: &Context,
    ) -> Result<Signal>;

    /// Compose from a polymorphic mix of signals and pulses.
    fn compose_datums(
        &self,
        datums: &[Datum<'_>],
        budget: &Budget,
        scorer: &dyn Score,
        ctx: &Context,
    ) -> Result<Signal> {
        let signals: Vec<Signal> = datums.iter().map(|d| match d {
            Datum::Signal(e) => (*e).clone(),
            Datum::Pulse(p) => Signal::from_pulse_synthetic(p),
        }).collect();
        self.compose(&signals, budget, scorer, ctx)
    }

    fn name(&self) -> &str;
}
```

**Implementors**: `SystemPromptBuilder` (9-layer prompt assembly with role templates),
`ContextComposer`, `PlanComposer`.

**Key property**: The output is always a durable Signal. The input can contain either medium
via the Datum-polymorphic `compose_datums()` method.

---

## 9. React -- Reactive Policy (Trait #7)

```rust
pub trait React: Cell + Send + Sync {
    /// Examine the recent signal stream and produce new signals.
    fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;

    /// Examine both persisted signals and ephemeral pulses.
    fn decide_with_pulses(
        &self,
        signals: &[Signal],
        _pulses: &[Pulse],
        ctx: &Context,
    ) -> PolicyOutputs {
        let out_signals = self.decide(signals, ctx);
        PolicyOutputs {
            signals: out_signals,
            pulses: Vec::new(),
        }
    }

    fn name(&self) -> &str;
}
```

**Implementors**: `CircuitBreakerPolicy`, `ConductorPolicy`, `EpisodePolicy`,
`HeartbeatPolicy`, `MetricPolicy`, `PlanPhasePolicy`.

**PolicyOutputs** separates the two mediums:
- `signals: Vec<Signal>` -- durable records to persist
- `pulses: Vec<Pulse>` -- ephemeral messages to publish on the Bus

---

## 10. Bus -- Transport Fabric (Trait #8)

```rust
pub trait Bus: Send + Sync {
    type Receiver: Send;

    /// Publish a pulse. Returns the assigned sequence number.
    fn publish(&self, pulse: Pulse) -> Result<u64>;

    /// Subscribe to pulses matching the given topic filter.
    fn subscribe(&self, filter: TopicFilter) -> Result<Self::Receiver>;
}
```

**Implementors**: `PulseBus` in `roko-runtime` (wraps `EventBus<Pulse>` with topic
filtering).

See `bus-transport-fabric.md` for full specification.

---

## 11. Observe -- Passive Data Collection (Trait #9)

```rust
pub trait Observe: Cell {
    /// Collect observations from the environment.
    fn observe(&self) -> Vec<Signal>;
}
```

Used for passive data collection from external sources without requiring active connections.

---

## 12. Connect -- External Connections (Trait #10)

```rust
pub trait Connect: Cell {
    /// Establish the connection.
    fn connect(&self) -> Result<()>;
    /// Check if the connection is healthy.
    fn health(&self) -> bool;
    /// Tear down the connection.
    fn disconnect(&self) -> Result<()>;
}
```

Manages lifecycle of connections to external systems (LLMs, APIs, tools).

---

## 13. Trigger -- Armed Conditions (Trait #11)

```rust
pub trait Trigger: Cell {
    /// Arm the trigger to begin watching.
    fn arm(&self) -> Result<()>;
    /// Disarm the trigger, stopping all watches.
    fn disarm(&self) -> Result<()>;
}
```

All seven trigger sources are implemented (E31 8/8): cron with IANA/DST support, event
pattern matching, threshold conditions, and EVM ABI-aware chain watchers.

---

## 14. Substrate -- Legacy Alias (Trait #12)

```rust
/// Substrate -- legacy alias for the `Store` trait.
pub trait Substrate: Store {}
impl<T: Store + ?Sized> Substrate for T {}
```

A blanket implementation ensures that anything implementing `Store` automatically implements
`Substrate`. This provides backward compatibility while `Store` is the canonical trait name.

---

## 15. The Cell Super-Trait

All non-fabric operator traits (Score, Verify, Route, Compose, React, Observe, Connect,
Trigger) extend the `Cell` trait:

```rust
pub trait Cell: Send + Sync {
    // Cell identity, metadata, and lifecycle methods
}
```

This makes every operator a composable unit in the Graph execution engine. Cells are the
nodes in the DAG; edges define data flow and execution dependencies.

---

## 16. Trait x Layer Map

```text
Layer 4: Orchestration  -> React (plan reactions, scheduling responses)
Layer 3: Harness        -> Verify (gate pipeline), React (watchers, breakers)
Layer 2: Scaffold       -> Score (context relevance), Compose (prompt assembly)
Layer 1: Framework      -> Route (model/tool selection), Score (dispatch relevance)
Layer 0: Runtime        -> Store (persistence), ColdStore (archival), Bus (transport)
```

The Observe, Connect, and Trigger protocols span L0-L1 depending on implementation.

---

## 17. Sufficiency Analysis

The 12-trait vocabulary is sufficient because:

1. Every capability in the system maps to one of these traits.
2. No boundary case requires a 13th trait -- awkward cases are medium mismatches resolved
   by the Datum polymorphism, not missing operator categories.
3. The Cell super-trait makes all operators composable in the Graph engine.
4. The two-fabric split (Store/Bus) eliminates the need to force transport through storage.

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Sumers et al. 2023 (arXiv:2309.02427) | CoALA: cognitive decomposition supports compact operator vocabulary |
| Franklin et al. 2016, LIDA | Concurrent cognitive roles map to composable operators |
| Chen et al. 2023 (arXiv:2305.05176) | FrugalGPT cascades justify Router and Composer separation |
| Friston 2010 | Verification and reaction sit inside an active-inference loop |
| Milewski 2014 | Small composable interfaces are easier to reason about |

---

## Cross-References

- `substrate-trait.md` -- Store trait deep dive
- `bus-transport-fabric.md` -- Bus trait deep dive
- `scorer-gate-router-composer-policy.md` -- Five operational traits in detail
- `five-layer-taxonomy.md` -- Layer assignments
- `vision-and-thesis.md` -- Architectural context
