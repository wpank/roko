# The Six Operators, Generalized Over Two Mediums

> **TL;DR**: Keep the six existing traits — Scorer, Gate, Router,
> Composer, Policy, plus the two fabric traits Substrate and Bus. Change
> their signatures so they naturally accept either a durable Engram or an
> ephemeral Pulse where it makes sense. No new traits; no merges.

> **For first-time readers**: Roko today has five non-fabric traits — Scorer
> (rate), Gate (verify), Router (choose), Composer (assemble under budget),
> Policy (react to streams) — plus Substrate (storage). This doc adds Bus
> (transport) as a seventh trait and generalizes the five over a new `Datum`
> enum that is either an Engram or a Pulse. The six-operator count stays the
> same; the seventh slot is the Bus we promoted in 03. Read 02 and 03 first
> for the two mediums and two fabrics; this doc is how the verbs align to
> both.

## 1. The revised trait table

| Trait | Role | Input | Output | Medium |
|---|---|---|---|---|
| **Substrate** | Persist / query durable records | `Engram` | `Vec<Engram>` | Engram |
| **Bus** | Publish / subscribe live events | `Pulse`, `TopicFilter` | `BusReceiver<Pulse>` | Pulse |
| **Scorer** | Rate an item along multi-dim axes | `&Datum` (either) | `Score` | Either |
| **Gate** | Verify against external truth | `&Datum` (usually Engram, sometimes Pulse window) | `Verdict` (which is itself an Engram) | Either → Engram |
| **Router** | Choose among candidates | `&[Datum]` | `Option<Selection>` | Either |
| **Composer** | Combine many into one under a budget | `&[Datum]`, `&Budget`, `&dyn Scorer` | `Engram` | Either → Engram |
| **Policy** | React to streams, emit new data | `&[Pulse]` | `PolicyOutputs` ( Pulses + Engrams ) | Pulse → Either |

`Datum` is a small enum:

```rust
/// Either medium — used by operators that work polymorphically.
pub enum Datum<'a> {
    Engram(&'a Engram),
    Pulse(&'a Pulse),
}

impl Datum<'_> {
    pub fn kind(&self) -> &Kind { ... }
    pub fn body(&self) -> &Body { ... }
    pub fn tags(&self) -> Option<&BTreeMap<String, String>> { ... }
    pub fn created_at_ms(&self) -> i64 { ... }
}
```

Each operator decides whether it accepts `&Engram`, `&Pulse`, or
`&Datum`. The trait signatures below describe the recommended choice.

## 2. Substrate — unchanged

`Substrate` stays exactly as it is today. It is the storage fabric for
Engrams. Zero signature changes.

## 3. Bus — new

Defined in detail in `03-bus-as-first-class.md`. It is the transport
fabric for Pulses. It is the seventh trait in the kernel and the
first-class partner of `Substrate`.

## 4. Scorer — minor generalization

Today:

```rust
pub trait Scorer: Send + Sync {
    fn score(&self, signal: &Engram, ctx: &Context) -> Score;
    fn name(&self) -> &'static str;
}
```

Proposed:

```rust
pub trait Scorer: Send + Sync {
    /// Score any datum (Engram or Pulse).
    fn score(&self, datum: Datum<'_>, ctx: &Context) -> Score;
    fn name(&self) -> &'static str;
}
```

Or, if we want to keep separate signatures for monomorphic perf:

```rust
pub trait Scorer: Send + Sync {
    fn score_engram(&self, e: &Engram, ctx: &Context) -> Score;
    fn score_pulse(&self, p: &Pulse, ctx: &Context) -> Score {
        // Default: score as if pulse were a minimal Engram.
        let tmp = p.to_synthetic_engram();
        self.score_engram(&tmp, ctx)
    }
    fn name(&self) -> &'static str;
}
```

The second form is probably better — it lets hot-path scorers stay
zero-allocation on Engrams and only pay conversion cost on Pulses if
they're scored at all.

**Use cases enabled by Pulse-scoring:**

- Scoring live agent stream chunks for an "is this going off the
  rails" detector.
- Scoring conductor-emitted health Pulses to decide which circuit
  breaker is most in distress.
- Scoring incoming HTTP webhook Pulses on `roko-serve` for triage.

## 5. Gate — generalizes to stream verification

Today:

```rust
pub trait Gate: Send + Sync {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}
```

Proposed:

```rust
pub trait Gate: Send + Sync {
    /// Verify a single durable Engram. (Default case — compile, test, clippy.)
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict;

    /// Verify a window of Pulses. Default: collect, graduate, recurse.
    /// Gates that want native stream verification override this.
    async fn verify_stream(
        &self,
        pulses: &[Pulse],
        ctx: &Context,
    ) -> Verdict {
        // default: materialize as a synthetic Engram and verify
        let synthetic = Engram::from_pulses(pulses);
        self.verify(&synthetic, ctx).await
    }

    fn name(&self) -> &str;
}
```

**Use cases enabled by stream-gates:**

- **BudgetGate** that watches `agent.tokens.used` Pulses and trips when
  a plan exceeds `budget.max_plan_usd`. Today this logic is buried in
  orchestrate.rs; as a stream-gate it's composable.
- **SafetyGate** that watches `safety.approval.requested` Pulses and
  ensures no two destructive operations run concurrently.
- **LivenessGate** that watches `agent.msg.chunk` timing and trips if
  an agent goes silent for N seconds.

A Verdict is still always an Engram — the audit DAG is preserved.

## 6. Router — generalizes across both mediums

Today:

```rust
pub trait Router: Send + Sync {
    fn select(&self, candidates: &[Engram], ctx: &Context) -> Option<Selection>;
    fn feedback(&self, outcome: &Outcome);
    fn name(&self) -> &str;
}
```

Proposed:

```rust
pub trait Router: Send + Sync {
    /// Select from Engram candidates (e.g. "which past episode applies").
    fn select_engram(&self, candidates: &[Engram], ctx: &Context) -> Option<Selection>;

    /// Select from Pulse candidates (e.g. "which pending approval to route to whom").
    fn select_pulse(&self, candidates: &[Pulse], ctx: &Context) -> Option<Selection> {
        // default: none — not every router cares about Pulses
        None
    }

    fn feedback(&self, outcome: &Outcome);
    fn name(&self) -> &str;
}
```

**Use cases enabled by Pulse-routing:**

- **ApprovalRouter** that picks which operator/reviewer gets notified
  for an `safety.approval.requested` Pulse.
- **WorkDistributionRouter** that picks which agent receives a newly
  ready `orchestration.task.ready` Pulse.

## 7. Composer — can ingest either

Today:

```rust
pub trait Composer: Send + Sync {
    fn compose(
        &self,
        signals: &[Engram],
        budget: &Budget,
        scorer: &dyn Scorer,
        ctx: &Context,
    ) -> Result<Engram>;
    fn name(&self) -> &str;
}
```

Proposed:

```rust
pub trait Composer: Send + Sync {
    fn compose(
        &self,
        inputs: &[Datum<'_>],
        budget: &Budget,
        scorer: &dyn Scorer,
        ctx: &Context,
    ) -> Result<Engram>;
    fn name(&self) -> &str;
}
```

**Use cases enabled by Pulse-input composers:**

- **LiveContextComposer** that builds an up-to-the-moment context
  window including both stored episodes (Engrams) and the last N
  stream chunks (Pulses).
- **TelemetryRollupComposer** that consolidates a minute of
  `agent.tokens.used` Pulses into a single `TokenUsageSummary` Engram.

## 8. Policy — the big reshape

Today:

```rust
pub trait Policy: Send + Sync {
    fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
    fn name(&self) -> &str;
}
```

Proposed:

```rust
pub trait Policy: Send + Sync {
    fn decide(&self, stream: &[Pulse], ctx: &Context) -> PolicyOutputs;
    fn name(&self) -> &str;
}

pub struct PolicyOutputs {
    /// Pulses to publish on the Bus.
    pub pulses: Vec<Pulse>,
    /// Engrams to persist via Substrate (graduated Pulses, summaries, etc.).
    pub engrams: Vec<Engram>,
}
```

This is the most consequential signature change. Policies are the
reactive layer, and reacting naturally happens over Pulses, not
stored Engrams. Today's implementations that want to react to stored
Engrams (e.g. EpisodePolicy consolidating recent episodes) subscribe
to a `substrate.engram.stored` topic that the Substrate emits when
an Engram lands. That topic is one of the "Substrate emits, Bus
delivers" bridges.

**Existing Policy implementations and their migration:**

| Policy | Today | After |
|---|---|---|
| `EpisodePolicy` | Iterates over episode Engrams | Subscribes to `substrate.engram.stored` filtered to `kind = Episode` |
| `ConductorPolicy` | Watches circuit breaker state | Subscribes to `gate.failure.rate`, `agent.health.*` |
| `PheromonePolicy` (Phase 2) | Watches pheromone Engrams | Subscribes to `mesh.pheromone.deposited` |
| `CircuitBreakerPolicy` | Watches gate history | Subscribes to `gate.verdict.emitted`, computes rolling EMA, publishes `gate.failure.rate` |
| `HeartbeatPolicy` (new) | n/a | Publishes `heartbeat.tick` at Gamma/Theta/Delta rates |
| `MetricPolicy` | `decide(&[], ctx) -> Vec<Engram{Metric}>` | Publishes `metric.*` Pulses; optionally graduates on a cadence |

## 9. Why no new traits

The user's original framing deserves pushback, but the doc-23 audit of
131 trait implementations found no seventh operation. That audit still
holds. What changes is:

- **Signatures generalize** to accept `Datum` or a `&[Pulse]`.
- **Bus joins the kernel** at the same level as Substrate.
- **Policy's input changes** from `&[Engram]` to `&[Pulse]` because
  that's what it was secretly already consuming.

No new traits, no merges, no splits. The six operations plus two
fabrics plus two mediums is the complete kernel grammar.

## 10. The revised one-liner

Replace:

> Roko is one noun (Engram) and six verb traits (Substrate, Scorer,
> Gate, Router, Composer, Policy).

With:

> Roko's kernel is two mediums (durable Engrams, ephemeral Pulses)
> moving through two fabrics (Substrate for storage, Bus for
> transport), acted on by six operators (Scorer, Gate, Router,
> Composer, Policy — and Substrate/Bus count as the storage and
> transport operators themselves). Composition is by trait; layers
> enforce a downward-only dependency rule.

Or, if brevity is the goal:

> Roko is two mediums, two fabrics, six operators, five layers,
> three speeds, three cross-cuts.

That phrase carries the whole architecture in one line — and it's the
one CLAUDE.md, README.md, and `docs/00-architecture/INDEX.md` should
lead with.

## 11. Default method strategy

The proposed generalizations accept a wider input set than any single
operator actually needs. To keep monomorphic performance:

- Every trait method that accepts `Datum<'_>` has a monomorphic
  `_engram` variant and a `_pulse` variant. The `Datum` form is a thin
  dispatcher.
- Default implementations cover the "convert and recurse" path so
  implementations that only care about one medium get the other for
  free at a small cost.
- Performance-critical implementations override both variants directly
  and never pay the dispatch.

Example for the Scorer trait:

```rust
pub trait Scorer: Send + Sync {
    fn score_engram(&self, e: &Engram, ctx: &Context) -> Score;

    fn score_pulse(&self, p: &Pulse, ctx: &Context) -> Score {
        let tmp = Engram::from_pulse_synthetic(p);   // cheap if body shared
        self.score_engram(&tmp, ctx)
    }

    fn score(&self, d: Datum<'_>, ctx: &Context) -> Score {
        match d {
            Datum::Engram(e) => self.score_engram(e, ctx),
            Datum::Pulse(p) => self.score_pulse(p, ctx),
        }
    }

    fn name(&self) -> &'static str;
}
```

A hot-path scorer implements `score_engram` only. A scorer that
natively handles streaming implements both. A scorer that doesn't care
uses `score` via the default and pays one match plus one synthetic
construction on the Pulse path.

## 12. Migration recipe per trait

For anyone working through the refactor, the per-trait steps are small
and mechanical. `06-refactoring-plan.md` §C.2 has the full sequencing;
this section is the per-call-site checklist.

| Trait | Today's sig | After | Per call-site change |
|---|---|---|---|
| Scorer | `score(&Engram, &Context) -> Score` | add `score_pulse`, `score` | old calls still compile; new Pulse calls land as work unblocks them |
| Gate | `verify(&Engram, &Context) -> Verdict` | add `verify_stream(&[Pulse], &Context)` | only stream-gates need change; Engram gates unchanged |
| Router | `select(&[Engram], &Context)` | add `select_pulse` | only Pulse-routing use cases need change |
| Composer | `compose(&[Engram], budget, scorer, ctx)` | `compose(&[Datum], ...)` | callers wrap `&Engram` as `Datum::Engram(e)` — trivial |
| Policy | `decide(&[Engram], ctx)` | `decide(&[Pulse], ctx)` | breaking. Migrate per subsystem via temporary shim that subscribes to `substrate.engram.stored` |

The Policy change is the only breaking one. The phased plan in 06
introduces a shim Policy that subscribes to a `substrate.engram.stored`
topic and re-emits as an Engram-slice Policy call so existing
implementations work during the migration. The shim deletes when the
last consumer is migrated.

## 13. Cross-references

- The two-medium split is in `02-engram-vs-pulse.md`.
- The Bus trait and its methods are in `03-bus-as-first-class.md`.
- The revised seven-step loop that uses these generalized operators is
  in `05-loop-retold.md`.
- The actual Rust code for these trait generalizations is in
  `08-code-sketches.md` §2–§5.
- The phased rollout is in `06-refactoring-plan.md` Phase B (kernel
  addition) and Phase C (subsystem migration).
- Active-inference predictor/outcome Pulses plug into these operators
  in `10-self-learning-cybernetic-loops.md`.
