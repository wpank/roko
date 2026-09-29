# 00-ARCH -- Cognitive Cross-Cuts: Neuro, Daimon, Dreams

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Three cognitive cross-cuts -- Neuro (knowledge), Daimon (motivation), and Dreams
> (offline learning) -- are injected into operators and speeds rather than treated as
> steps in the universal loop. Since E44 (Cross-cut Functors, 8/8), each cross-cut is
> realized as a concrete implementation of the `CrossCutFunctor` trait, composing via
> `EnrichedCell` with pre_enrich -> operation -> post_enrich hooks that unwind in
> reverse order (outermost last). This depth file documents the integration points,
> the E44 functor model, the arbitration protocol, the natural transformations, and
> the gate-failure cascade.

---

## 1. Why Cross-Cuts

In a layered architecture, knowledge management, affect modulation, and offline
consolidation all need to be available in more than one place. Forcing them into a
single layer would either create upward dependencies or turn the universal loop into
a pile of special cases.

The solution is cross-cut injection. Neuro implements `Substrate` and provides durable
knowledge to any layer that needs it. Daimon exposes PAD-driven biasing to scoring,
routing, and gating logic. Dreams runs as its own Delta-speed consolidation loop,
reading from and writing to the substrate while also publishing related Pulses.

The cross-cuts are not loop steps. They are load-bearing injections that shape the
seven-step loop from within and alongside it.

Before E44, injection was ad hoc: scattered helper functions queried the knowledge
store, threaded PAD values through context structs, or triggered dream cycles via
external timers. E44 replaced this with a uniform, testable, composable functor model
where each cross-cut is a `CrossCutFunctor` implementation composed through
`EnrichedCell`.

---

## 2. Neuro -- Knowledge Management

`roko-neuro` provides persistent, tier-based knowledge management with HDC encoding
for similarity search.

### 2.1 Six Knowledge Types

| Type | Purpose | Example |
|---|---|---|
| **Insight** | A general observation that proved useful | "This codebase uses builder pattern extensively" |
| **Heuristic** | A procedural rule extracted from experience | "When tests fail with E0599, check trait imports first" |
| **Warning** | A known pitfall or anti-pattern | "Never use --no-verify with this repo's hooks" |
| **CausalLink** | A cause-effect relationship | "Upgrading alloy requires rustc 1.91+" |
| **StrategyFragment** | A reusable strategic approach | "For large refactors, use worktrees for parallel branches" |
| **AntiKnowledge** | Explicitly falsified knowledge | "Hypothesis X was tested and disproved" |

AntiKnowledge prevents the system from re-exploring dead ends. When a hypothesis is
falsified, it is stored so later agents can avoid repeating the same mistake. This is
the knowledge-level equivalent of the `Falsifier` primitive in the learning subsystem.

### 2.2 Four Knowledge Tiers

Knowledge progresses through four tiers with different retention characteristics:

| Tier | Strength Multiplier | Effective Half-Life | Promotion Criteria |
|---|---|---|---|
| **Transient** | 0.1x | Minutes to hours | Created on first observation |
| **Working** | 0.5x | Hours to days | Referenced in 2+ successful ticks |
| **Consolidated** | 1.0x | Days to weeks | Validated by gate verdicts or prediction outcomes |
| **Persistent** | 5.0x | Weeks to months | Repeatedly validated across multiple sessions |

Tier promotion happens during the Dreams Delta loop. Knowledge that proves useful is
promoted; knowledge that fails to prove itself decays naturally via Ebbinghaus
forgetting (see decay-variants-and-tier-matrix.md). Live knowledge tier progression is
wired: successful gate-backed runner ingestion records confirmation/context evidence
and evaluates Transient -> Working progression.

### 2.3 HDC Encoding

Knowledge entries are encoded as 10,240-bit Hyperdimensional Computing (HDC) vectors
(Kanerva 2009, Cognitive Computation 1(2)) for O(1) similarity search:

- **Bind** (XOR): Combines two concepts into a bound pair
- **Bundle** (majority): Combines multiple vectors, preserving similarity to all inputs
- **Similarity** (Hamming distance): Measures overlap between vectors
- **Permute** (rotation): Sequences concepts by position

HDC encoding enables Cross-Domain Insight Resonance: knowledge from one domain can be
retrieved when it is structurally similar to a query from a different domain, even if
the domains share no vocabulary. The HDC vector space is a proper Vector Symbolic
Architecture (Kleyko et al. 2022, Artificial Intelligence Review 56), algebraically
richer than a simple embedding space.

### 2.4 MemoryFunctor Integration (E44)

The `MemoryFunctor` in `roko-compose/src/memory_functor.rs` is the E44 realization of
the Neuro cross-cut. It wraps an `Arc<KnowledgeStore>` and implements `CrossCutFunctor`:

```rust
pub struct MemoryFunctor {
    store: Arc<KnowledgeStore>,
    max_entries: usize,
    included: Mutex<HashMap<(String, String), Vec<String>>>,
    last_query_empty: AtomicBool,
}
```

| Loop step | pre_enrich | post_enrich |
|---|---|---|
| **Sense** | Query store by keyword + HDC similarity; inject matching entries as Insight signals with tier, retrieval method, and demurrage balance metadata | -- |
| **Compose** | Same query, plus add Neuro attention-bidder tags for VCG allocation | -- |
| **React** | -- | Detect gate verdicts; reinforce on pass, weaken on fail |

The `should_short_circuit()` method returns true when the last query was empty or the
store is empty, allowing the `EnrichedCell` to skip the functor entirely. This
optimization avoids unnecessary HDC similarity computations when knowledge context
would add nothing.

### 2.5 Integration Points (Pre-E44 Reference)

| Loop touchpoint | How Neuro Is Injected |
|---|---|
| SENSE | Neuro-backed `Substrate.query` supplies durable context for recall and retrieval |
| COMPOSE | Composer queries NeuroStore for relevant knowledge to enrich prompts under budget |
| VERIFY / REACT | Gate verdicts and outcome records are consumed back into Neuro for consolidation and tier promotion |
| Dreams Delta loop | Dreams reads from and writes to NeuroStore while consolidating Signals |

---

## 3. Daimon -- Motivation and Focus

`roko-daimon` provides the agent's self-model: a PAD (Pleasure-Arousal-Dominance)
vector that biases assessment, action gating, and cadence selection.

### 3.1 PAD Vector

The PAD model (Mehrabian & Russell 1974; Russell & Mehrabian 1977, Journal of
Personality and Social Psychology 35(4)) represents emotional state as three
orthogonal dimensions:

| Dimension | Range | What It Represents |
|---|---|---|
| **Pleasure** (P) | [-1, 1] | Task success vs. failure. Positive when things are going well |
| **Arousal** (A) | [-1, 1] | Urgency and load. High when there is surprise or pressure |
| **Dominance** (D) | [-1, 1] | Confidence and control. High when the agent feels capable |

The PAD vector is not a personality. It is a dynamic state that changes continuously
based on recent outcomes, gate verdicts, prediction accuracy, and task load.

### 3.2 Six Behavioral States

The PAD vector maps to six behavioral states (a simplification of Plutchik's emotion
wheel, Plutchik 2001, American Scientist 89(4)):

| State | PAD Region | Behavior |
|---|---|---|
| **Engaged** | P+, A moderate, D+ | Productive work. Standard Theta cadence |
| **Focused** | P+, A low, D+ | Deep work. Extended Gamma runs, fewer Theta interruptions |
| **Exploring** | P neutral, A+, D neutral | Curious. Higher exploration rate, more T2 escalation |
| **Struggling** | P-, A+, D- | Difficulty. Shortened Theta cadence, more frequent reflection |
| **Coasting** | P neutral, A-, D+ | Easy work. Extended Gamma, T0-heavy |
| **Resting** | P neutral, A-, D neutral | Idle. Delta consolidation mode |

These states are cyclical. There is no terminal state. The agent shifts behavior based
on task outcomes and environmental changes rather than a final endpoint.

### 3.3 Somatic Markers

Damasio's somatic marker hypothesis (Damasio 1994, Descartes' Error, Putnam) proposes
that emotional signals from past experience bias decision-making before conscious
deliberation. The `DaimonFunctor` queries the somatic landscape with
`StrategyCoordinates` (8-dimensional: complexity, risk, novelty, confidence,
time_pressure, scope, reversibility, dependency_depth) and injects the blended
valence/intensity signal into Assess-phase decisions.

### 3.4 DaimonFunctor Integration (E44)

The `DaimonFunctor` in `roko-compose/src/daimon_functor.rs`:

```rust
pub struct DaimonFunctor {
    state: Arc<RwLock<DaimonState>>,
    thresholds: BehavioralStateThresholds,
    somatic_config: SomaticRetrievalConfig,
}
```

| Loop step | pre_enrich | post_enrich |
|---|---|---|
| **Assess** | Inject PAD metadata and somatic-landscape retrieval signals | If high arousal + low dominance, emit tier-escalation signal |
| **Act** | If Struggling and high-risk action detected, emit deferral signal with safety_critical=true | Apply prospect-theory value function to reward; appraise affect event |

The prospect-theory value function (Kahneman & Tversky 1979, Econometrica 47(2)):

```
V(x) = x^alpha           if x >= 0   (alpha = 0.88)
V(x) = -lambda|x|^alpha  if x < 0   (lambda = 2.25)
```

This makes losses loom larger than equivalent gains, matching empirical
decision-making behavior. The functor applies this to the delta between actual and
expected reward on every Act post-enrichment.

### 3.5 Short-Circuit Optimization

`should_short_circuit()` returns true when all three PAD dimensions are near zero
(|P| < 0.1, |A| < 0.1, |D| < 0.1), indicating that the Daimon's affect state is
neutral and enrichment would add no information.

---

## 4. Dreams -- Offline Learning

`roko-dreams` provides offline learning during idle time at Delta frequency. Dreams is
its own Delta-speed loop: it consumes recent Signals and related Pulses, synthesizes
new Signals, and emits follow-up Pulses for later use.

### 4.1 Three-Phase Cycle

| Phase | Inspiration | What Happens |
|---|---|---|
| **NREM Replay** | Slow-wave sleep replay (Mattar & Daw 2018, Nature Neuroscience 21) | Replay recent episodes, weighted by prediction error magnitude. Extract patterns |
| **REM Imagination** | REM sleep creativity (Boden 2004, The Creative Mind) | Generate novel hypotheses via HDC recombination. Counterfactual reasoning via Pearl's SCM (Pearl 2009, Causality). Emotional depotentiation (Walker & van der Helm 2009) |
| **Integration Staging** | Memory consolidation (Lacaux et al. 2021, Science Advances 7(50)) | Validate dream outputs against existing knowledge. Promote to NeuroStore if confidence exceeds threshold |

Resident daemon scheduling is live for adaptive idle, cron, and episode-count triggers
with idle queuing and checkpoint restore. Bus-reactive/intensive backlog controls
remain partial.

### 4.2 Hypnagogia Engine

Four components generate creative hypotheses during the active-to-consolidation
transition:

| Component | Role |
|---|---|
| **Thalamic Gate** | Filters incoming stimuli, allowing only high-novelty signals through |
| **Executive Loosener** | Relaxes constraint satisfaction thresholds, enabling unusual associations |
| **Dali Interrupt** | Captures fleeting insights before they fade (named after Dali's nap technique) |
| **Homuncular Observer** | Coherence filter that evaluates whether the generated hypothesis is worth testing |

This addresses the Alpha Convergence Problem: without creative divergence, an agent's
knowledge converges to a local optimum. The hypnagogia engine provides the random
restart that exploration/exploitation algorithms need, but with structure.

### 4.3 DreamsFunctor and DreamOutputConsumer (E44)

The `DreamsFunctor` is intentionally a per-tick passthrough:

```rust
pub struct DreamsFunctor;

impl CrossCutFunctor for DreamsFunctor {
    fn name(&self) -> &str { "dreams" }
    fn should_short_circuit(&self) -> bool { true }
    // pre_enrich and post_enrich are identity
}
```

Dream work runs at Delta speed, not per-tick Gamma/Theta. The actual cross-cut
publication happens through `DreamOutputConsumer`, which takes a completed
`DreamCycleReport` and publishes its outputs to three live targets:

1. **Knowledge store**: via `eta_DM` (Dreams -> Memory natural transformation)
2. **Daimon state**: via `eta_DN` (Dreams -> Daimon), including depotentiation
3. **Cascade router**: via dream routing advice biasing future model selection

---

## 5. The E44 Functor Composition Model

E44 (Cross-cut Functors, 8/8 complete) replaced ad hoc injection with a formal
composition model based on category theory (Mac Lane 1971, Categories for the
Working Mathematician).

### 5.1 CrossCutFunctor Trait

```rust
#[async_trait]
pub trait CrossCutFunctor<C = CrossCutContext>: Send + Sync + 'static {
    fn name(&self) -> &str;
    async fn pre_enrich(&self, input: Vec<Signal>, ctx: &C)
        -> CrossCutResult<Vec<Signal>>;
    async fn post_enrich(&self, output: Vec<Signal>, ctx: &C)
        -> CrossCutResult<Vec<Signal>>;
    fn should_short_circuit(&self) -> bool;
}
```

Each cross-cut defines an endofunctor F: Eng -> Eng on the Signal category, where F
maps each trait implementation T to an enriched version F(T) and each Signal to an
enriched Signal with additional metadata.

### 5.2 EnrichedCell Composition

`EnrichedCell` wraps an operation with ordered functor application:

```
pre_enrich(safety) -> pre_enrich(memory) -> pre_enrich(daimon) -> operation
    -> post_enrich(daimon) -> post_enrich(memory) -> post_enrich(safety)
```

The first functor in the list is the outermost wrapper. Post-enrichment runs in
reverse order, creating a stack discipline analogous to middleware in web frameworks.
This ensures that safety checks wrap everything and cannot be bypassed by inner
functor logic.

### 5.3 The Four E44 Functors

| Functor | Crate | Purpose | Short-circuits? |
|---|---|---|---|
| `SafetyFunctor` | roko-compose | Capability and contract pre-filter, deny by default | Never |
| `MemoryFunctor` | roko-compose | Knowledge retrieval and gate-feedback reinforcement | When store is empty or last query empty |
| `DaimonFunctor` | roko-compose | Affect injection and prospect-theory valuation | When PAD is near zero |
| `DreamsFunctor` | roko-compose | Per-tick passthrough; real work at Delta speed | Always (per-tick is identity) |

### 5.4 Safety Functor Placement

The `SafetyFunctor` is structurally placed outside all other functors. It does not
participate in VCG arbitration because safety constraints are non-negotiable (E34 8/8
strict):

```rust
impl SafetyFunctor {
    pub fn wrap(
        self: Arc<Self>,
        inner: Vec<Arc<dyn CrossCutFunctor<CrossCutContext>>>,
    ) -> EnrichedCell {
        let mut functors: Vec<Arc<dyn CrossCutFunctor<CrossCutContext>>> = vec![self];
        functors.extend(inner);
        EnrichedCell::new(functors)
    }
}
```

Pre-enrichment filters signals by capability grant (deny by default). Post-enrichment
filters outputs by contract taint level and tool allowlist. The trust-origin lattice
and TaintTracker from E34 integrate through this functor.

### 5.5 Conflict Resolution via VCG

When the legacy greedy composition strategy is replaced by `CompositionStrategy::Auto`
or `CompositionStrategy::Vcg`, the `LearningBidder` in the auction module maintains
per-section Beta-posterior parameters for each subsystem. The E44 functors participate
in VCG through the attention tags they emit during pre-enrichment. Safety is exempt
from VCG -- it always wins. Memory and Daimon compete for context budget via truthful
bidding (winner pays second-highest bid, per the VCG mechanism of Vickrey 1961,
Clarke 1971, Groves 1973).

---

## 6. Natural Transformations

The six natural transformations in `roko-compose/src/natural_transforms.rs` are
structure-preserving maps between Memory, Daimon, and Dreams:

```
eta_MN : Memory -> Daimon    (gate outcomes become affect events)
eta_NM : Daimon -> Memory    (PAD assessments stored as knowledge)
eta_MD : Memory -> Dreams    (knowledge entries become replay inputs)
eta_ND : Daimon -> Dreams    (affect triggers consolidation)
eta_DM : Dreams -> Memory    (consolidated entries published to store)
eta_DN : Dreams -> Daimon    (depotentiation updates PAD)
```

### 6.1 Triangle Commutativity

The composition Daimon -> Memory -> Dreams must equal the direct path Daimon -> Dreams
for the system to stay consistent:

```
eta_MD(eta_NM(assessment)) === eta_ND(assessment)
```

This is enforced by a `debug_assert` in `run_gate_failure_cascade` and verified by
unit tests. Both paths produce identical `DreamConsolidationInput` values: same
episode IDs, same priority, same `trigger_delta` flag.

### 6.2 Gate-Failure Cascade

`run_gate_failure_cascade` fires the synchronous portion of the cascade when a gate
fails. The cascade exercises all six natural transformations:

1. Score prediction utility negatively for all affected knowledge entries
2. Record batch usage failures
3. Apply `eta_MN`: convert memory outcome to affect event, appraise
4. Snapshot Daimon as `DaimonAssessment`
5. Apply `eta_NM`: persist assessment as Working-tier knowledge
6. Apply `eta_MD`: convert knowledge to replay input (memory path)
7. Apply `eta_ND`: convert assessment to replay input (direct path)
8. Assert commutativity: both paths produce identical consolidation inputs

This cascade is the live, non-blocking path wired by E44. It ensures that every gate
failure triggers coordinated updates across all three cross-cuts simultaneously.

### 6.3 VSA Operations as Algebraic Structure

| HDC Operation | Categorical Analog | Knowledge Use |
|---|---|---|
| **Bind** (XOR) | Tensor product | Associating concept pairs: bind(tool, outcome) |
| **Bundle** (majority vote) | Direct sum / coproduct | Combining multiple related concepts |
| **Permute** (rotation) | Cyclic action | Sequencing: permute(step, position) |

---

## 7. Cross-Cut Arbitration Protocol

When two or more cross-cuts produce conflicting signals for the same decision, an
arbitration protocol resolves the conflict.

### 7.1 Priority Hierarchy

| Priority | Cross-cut | Rationale |
|---|---|---|
| 1 (highest) | **Safety** | Capability and contract constraints are non-negotiable |
| 2 | **Daimon** | Behavioral gating overrides learned preferences |
| 3 | **Neuro** | Validated knowledge overrides speculative hypotheses |
| 4 (lowest) | **Dreams** | Dream-generated hypotheses are speculative |

Safety structurally wins because `SafetyFunctor.wrap()` places it as the outermost
functor. Daimon vs. Neuro conflicts use the priority tags emitted by each functor
(Daimon emits `priority_level=1`, Memory emits `priority_level=2`).

### 7.2 Conflict Scenarios

**Scenario 1: Daimon vs. Neuro -- risk tolerance**

Daimon's PAD vector indicates low dominance, so it wants to escalate to slower
reasoning. Neuro has a Persistent heuristic that says this task type usually succeeds
with fast automatic handling. Resolution: Daimon wins. Current state overrides
historical success patterns because PAD reflects the present condition.

```rust
fn resolve_tier_conflict(daimon: TierRecommendation, neuro: TierRecommendation) -> Tier {
    if daimon.safety_critical { return daimon.tier; }
    daimon.tier.max(neuro.tier)
}
```

**Scenario 2: Neuro vs. Dreams -- contradictory knowledge**

Neuro has a Consolidated knowledge entry: "alloy requires rustc 1.91+." Dreams
generated a hypothesis: "alloy might work with rustc 1.85 using feature flags."
Resolution: Neuro wins. The Dreams hypothesis is queued for testing but does not
affect the current task.

**Scenario 3: Daimon vs. Dreams -- consolidation timing**

Daimon is in the Struggling state. Dreams has just completed consolidation and wants
to transition to Resting. Resolution: Daimon wins. Active task performance takes
priority over consolidation scheduling.

### 7.3 VCG Tiebreaker

When the priority hierarchy does not cleanly resolve a conflict, the system falls back
to a VCG (Vickrey-Clarke-Groves) attention auction. Each cross-cut bids its confidence
in its recommendation. The winner pays the second-highest bid, ensuring truthful
reporting.

```rust
fn vcg_tiebreak(bids: &[(CrossCut, f32, Action)]) -> Action {
    let mut sorted = bids.to_vec();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let winner = &sorted[0];
    let second_price = if sorted.len() > 1 { sorted[1].1 } else { 0.0 };
    log_attention_cost(winner.0, second_price);
    winner.2.clone()
}
```

Invoked only when: two cross-cuts are at the same priority level, both have
confidence > 0.5, and the conflict affects a Router or Composer decision (not safety).

---

## 8. Cross-Domain Speed Mapping

The three cognitive speeds (Gamma/Theta/Delta) apply uniformly across domains, but
the cross-cuts modulate how each speed functions:

| Speed | Neuro | Daimon | Dreams |
|---|---|---|---|
| **Gamma** (~5s) | Injects relevant symbols during SENSE/COMPOSE | Biases ASSESS when uncertainty rises | No-op |
| **Theta** (~75s) | Checks for stale heuristics | Tracks confidence trend across ticks | No-op |
| **Delta** (~hours) | Receives consolidated entries | Receives depotentiation | Runs NREM/REM/integration |

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Kanerva 2009, Cognitive Computation 1(2) | HDC: hyperdimensional computing for similarity search |
| Plate 2003, Holographic Reduced Representation | HRR: holographic encoding for knowledge representation |
| Frady et al. 2018 | Neural computation with HDC vectors |
| Kleyko et al. 2022, Artificial Intelligence Review 56 | Survey of HDC applications and VSA algebraic structure |
| Mehrabian & Russell 1974 | PAD model: Pleasure-Arousal-Dominance emotional space |
| Russell & Mehrabian 1977, JPSP 35(4) | Empirical validation of the PAD dimensional model |
| Damasio 1994, Descartes' Error, Putnam | Somatic marker hypothesis: emotion biases decision-making |
| Plutchik 2001, American Scientist 89(4) | Emotion wheel: mapping complex emotions to dimensional space |
| Kahneman & Tversky 1979, Econometrica 47(2) | Prospect theory: loss aversion and diminishing sensitivity |
| McClelland et al. 1995, Psychological Review 102(3) | Complementary Learning Systems theory |
| Mattar & Daw 2018, Nature Neuroscience 21 | Prioritized replay: replay what is most useful for future decisions |
| Walker & van der Helm 2009, ARCP 5 | REM sleep emotional depotentiation |
| Lacaux et al. 2021, Science Advances 7(50) | Hypnagogia: creative insights during sleep onset |
| Boden 2004, The Creative Mind | Computational creativity: exploratory, combinational, transformational |
| Pearl 2009, Causality, CUP | Structural Causal Models for counterfactual reasoning |
| Mac Lane 1971, Categories for the Working Mathematician | Categorical foundations for functor model |
| Vickrey 1961; Clarke 1971; Groves 1973 | VCG mechanism: truthful bidding for resource allocation |

---

## Cross-References

- [33-01 Functor Model](../33-cross-cuts/01-functor-model.md) -- Category-theoretic foundations
- [33-02 Memory Functor](../33-cross-cuts/02-memory-functor.md) -- MemoryFunctor depth
- [33-03 Daimon Functor](../33-cross-cuts/03-daimon-functor.md) -- DaimonFunctor depth
- [33-04 Dreams Functor](../33-cross-cuts/04-dreams-functor.md) -- DreamsFunctor depth
- [33-05 Safety Functor](../33-cross-cuts/05-safety-functor.md) -- SafetyFunctor depth
- [33-06 Arbitration](../33-cross-cuts/06-arbitration.md) -- VCG and priority resolution
- [33-07 Cascade](../33-cross-cuts/07-cascade.md) -- Gate-failure cascade depth
- `crates/roko-compose/src/cross_cut.rs` -- CrossCutFunctor trait
- `crates/roko-compose/src/memory_functor.rs` -- MemoryFunctor implementation
- `crates/roko-compose/src/daimon_functor.rs` -- DaimonFunctor implementation
- `crates/roko-compose/src/natural_transforms.rs` -- Six natural transformations
- `crates/roko-neuro/` -- Knowledge store
- `crates/roko-daimon/` -- Affect engine
- `crates/roko-dreams/` -- Dream consolidation
