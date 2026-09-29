# Scorer, Gate, Router, Composer, Policy -- The Five Operational Traits

> **v3 depth file** -- `/docs/v3/depth/00-architecture/scorer-gate-router-composer-policy.md`
> Canonical source: v1 `docs/v1/00-architecture/08-scorer-gate-router-composer-policy.md`
> Implementation: `crates/roko-core/src/traits.rs`
> Status: **Shipping** -- all five operational traits are implemented with both Signal-first
> and Pulse/Datum-polymorphic entry points. The trait names in code are Score, Verify, Route,
> Compose, and React.

---

## Kernel Framing

Roko's kernel is two mediums, two fabrics, and twelve traits. The five non-fabric operators
documented here are:

| Trait | Code name | Role | Input | Output | Medium |
|---|---|---|---|---|---|
| Scorer | `Score` | Rate along multi-axis criteria | `Datum<'_>` or `&Signal` / `&Pulse` | `ScoreValue` | Either |
| Gate | `Verify` | Verify against external reality | `&Signal` or `&[Pulse]` | `Verdict` (persisted as Signal) | Either -> Signal |
| Router | `Route` | Choose among candidates | `&[Signal]` or `&[Pulse]` | `Option<Selection>` | Either |
| Composer | `Compose` | Combine under budget | `&[Datum<'_>]` or `&[Signal]`, Budget, Scorer | `Signal` | Either -> Signal |
| Policy | `React` | React to streams and outcomes | `&[Signal]` and/or `&[Pulse]` | `Vec<Signal>` or `PolicyOutputs` | Either |

The fabric siblings (Store, ColdStore, Bus) are documented in their own depth files.

---

## Datum -- Shared Surface for Either Medium

Operators that work polymorphically across Signal and Pulse use `Datum`:

```rust
pub enum Datum<'a> {
    Signal(&'a Signal),
    Pulse(&'a Pulse),
}

impl Datum<'_> {
    pub fn kind(&self) -> &Kind { /* ... */ }
    pub fn body(&self) -> &Body { /* ... */ }
    pub fn tags(&self) -> Option<&BTreeMap<String, String>> { /* ... */ }
    pub fn created_at_ms(&self) -> i64 { /* ... */ }
}
```

Practical rule:
- Use `&Signal` when only the durable path makes sense
- Use `&Pulse` or `&[Pulse]` when reacting to live traffic
- Use `Datum` when a single operator needs to accept either medium

---

## 1. Scorer (Score Trait) -- Rate Signals

Scorers rate what the runtime should care about. Assessment happens against both durable and
live inputs: retrieved Signals during context selection, candidate actions before execution,
and live runtime signals that may later graduate.

```rust
pub trait Score: Cell + Send + Sync {
    /// Score a persisted signal -- the implementor hook.
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

**Shape preserves the fast path**:
- Signal-oriented scorers implement `score()` only.
- Pulse-aware scorers override `score_pulse()` for transport-native behavior.
- Callers that want one entry point use `score_datum()`.

**Pulse-aware uses**: scoring stream chunks for drift, conductor health Pulses for distress
prioritization, webhook Pulses for triage before graduation.

**Key implementors**:
- `RelevanceScorer` -- how well the Signal matches the current goal
- `RecencyScorer` -- how recent is the Signal
- `ReputationScorer` -- author's track record
- `CatalyticScorer` -- how many downstream Signals this one has catalyzed
- `CompositeScorer` -- composes multiple scorers via arithmetic

**Score output**: 7-axis `ScoreValue` (see `score-7-axis-appraisal.md` for full specification).

---

## 2. Gate (Verify Trait) -- Verify Against Ground Truth

Gates connect Roko to external reality. They compile code, run tests, simulate transactions,
validate schemas, check balances, and emit verdicts about whether a claim survives contact
with the world.

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

**Key invariant**: A Verdict is always a durable audit artifact. Even when a Gate verifies a
live Pulse window, the result persists as a GateVerdict Signal so the audit DAG remains
durable.

**Stream-gates** exist for temporal criteria:
- `BudgetGate` -- watches `agent.tokens.used` over a window
- `SafetyGate` -- watches `safety.approval.requested` for sequencing violations
- `LivenessGate` -- watches `agent.msg.chunk` timing and trips on silence

**Key implementors** (19 gates in 7-rung pipeline):

| Rung | Gates | Purpose |
|---|---|---|
| 0 | Compile | Does it build? |
| 1 | Format, Clippy | Does it meet style standards? |
| 2 | Test (unit) | Do unit tests pass? |
| 3 | Test (integration) | Do integration tests pass? |
| 4 | Diff, Schema | Is the diff reasonable? Does it match the schema? |
| 5 | LlmJudge | Does an LLM reviewer approve? |
| 6 | Simulation | Does the simulated execution succeed? |

Adaptive gate thresholds (EMA-based) are persisted in `.roko/learn/gate-thresholds.json`
and updated based on observed pass rates.

---

## 3. Router (Route Trait) -- Select Among Alternatives

Routers choose among alternatives: which model to call, which backend to use, which tool to
run, which plan branch to pursue, or which candidate artifact to advance.

```rust
pub trait Route: Cell + Send + Sync {
    /// Select one signal from candidates -- the implementor hook.
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

**Durable selection**: "which episode or exemplar applies"
**Pulse routing**: "which reviewer should receive this approval request"

The default `None` on `select_pulse` keeps durable-only routers unchanged until a subsystem
has a real live-routing need.

**Key implementors**:
- `StaticRouter` -- deterministic, config-driven
- `LinUCBRouter` -- contextual bandit with upper confidence bound
- `CascadeRouter` -- multi-stage confidence + UCB, persists to `.roko/learn/cascade-router.json`
- `WeightedRouter` -- softmax over scorers

**Feedback loop**: Routers learn from outcomes via `feedback()`. The CascadeRouter implements
a contextual bandit that improves model selection quality with experience.

---

## 4. Composer (Compose Trait) -- Combine Under Budget

Composer is the bounded assembly operator: it turns multiple ingredients into a bounded
output under token, byte, time, or structural budgets.

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

**Architectural boundary**:
- Composition may need stored episodes plus the last N stream chunks
- Scoring and budget logic apply across the whole candidate set
- The result remains a Signal because composed artifacts are durable records

**Key implementors**:
- `SystemPromptBuilder` -- 9-layer prompt assembly with 11 role templates
- `ContextComposer` -- assembles context from Store retrieval and Bus traffic
- `PlanComposer` -- assembles plans from task descriptions
- `TelemetryRollupComposer` -- consolidates transport Pulses into summary Signals

**Budget struct**:
```rust
pub struct Budget {
    pub max_tokens: Option<usize>,
    pub max_bytes: Option<usize>,
    pub max_signals: Option<usize>,
    pub max_wall_ms: Option<u64>,
}
```

---

## 5. Policy (React Trait) -- React to Streams

Policy is where reactive logic lives. Policies watch ongoing activity and decide whether to
emit interventions, summaries, alerts, pauses, promotions, or other follow-on work.

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

pub struct PolicyOutputs {
    pub signals: Vec<Signal>,
    pub pulses: Vec<Pulse>,
}
```

**PolicyOutputs** makes the reaction step explicit:
- Publish new Pulses on the Bus for immediate downstream reactions
- Persist Signals for summaries, graduations, metrics, or durable decisions

**Key implementors**:
- `EpisodePolicy` -- subscribes to stored episodes and emits summaries
- `CircuitBreakerPolicy` -- watches gate verdicts and publishes failure-rate Pulses
- `HeartbeatPolicy` -- publishes heartbeat tick Pulses at Gamma/Theta/Delta cadence
- `MetricPolicy` -- publishes metric Pulses and graduates summaries on a cadence
- `PlanPhasePolicy` -- manages plan state machine transitions

---

## 6. Trait x Layer Map

| Trait | L0 Runtime | L1 Framework | L2 Scaffold | L3 Harness | L4 Orchestration |
|---|---|---|---|---|---|
| Score | -- | ToolRelevanceScorer | RelevanceScorer, RecencyScorer | -- | -- |
| Verify | -- | -- | -- | CompileGate, TestGate, etc. | -- |
| Route | -- | CascadeRouter, LinUCBRouter | -- | -- | -- |
| Compose | -- | -- | SystemPromptBuilder, ContextComposer | -- | PlanComposer |
| React | -- | -- | -- | CircuitBreakerPolicy | PlanPhasePolicy |

---

## 7. Migration Strategy

The trait evolution is deliberately conservative:

| Trait | Before | After | Shape |
|---|---|---|---|
| Score | `score(&Signal, &Context)` | add `score_pulse`, `score_datum` | additive |
| Verify | `verify(&Signal, &Context)` | add `verify_stream(&[Pulse], &Context)` | additive |
| Route | `select(&[Signal], &Context)` | add `select_pulse(&[Pulse], ...)` | additive |
| Compose | `compose(&[Signal], ...)` | add `compose_datums(&[Datum], ...)` | additive wrapper |
| React | `decide(&[Signal], ctx)` | add `decide_with_pulses(signals, pulses, ctx)` | additive |

All five traits preserve backward compatibility: existing Signal-first implementations
work unchanged. The Pulse/Datum extensions are additive with sensible defaults.

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Sumers et al. 2023 (arXiv:2309.02427) | CoALA: cognitive decomposition supports compact operator vocabulary |
| Chen et al. 2023 (arXiv:2305.05176) | FrugalGPT cascades justify Router and Composer separation |
| Friston 2010 | Verification and reaction sit inside an active-inference loop |
| Franklin et al. 2016, LIDA | Concurrent cognitive roles map to composable operators |

---

## Cross-References

- `synapse-traits-12.md` -- Full 12-trait overview
- `substrate-trait.md` -- Store trait (durable storage fabric)
- `bus-transport-fabric.md` -- Bus trait (transport fabric)
- `score-7-axis-appraisal.md` -- ScoreValue specification
- `five-layer-taxonomy.md` -- Layer assignments for each trait
