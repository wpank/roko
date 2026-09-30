# 00-ARCH -- Architectural Coherence Analysis and Improvements

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> A comprehensive analysis of Roko's architecture evaluated against modern research
> in cognitive architectures (SOAR, ACT-R, LIDA), trait-based systems (Scala
> typeclasses, Haskell type classes), category theory (functors, monoids, natural
> transformations), and active inference (Free Energy Principle, VERSES Genius).
> Identifies architectural strengths, coherence gaps, layer violations, and
> improvement paths. Updated for v3 with Graph as the sole engine, 12 protocol
> traits, E44 cross-cut functors, and the full E34 safety layer.

---

## 1. Executive Summary

Roko's architecture is **remarkably coherent** for a system of its scale (~1M LOC, 39
crates, 10,300+ tests). The architecture uses two mediums (durable Signal, ephemeral
Pulse) moving through two fabrics (Substrate for storage, Bus for transport), a
seven-step universal loop, and three cognitive speeds (Gamma/Theta/Delta).

Key findings:

1. **The 12 protocol traits are sufficient.** Analysis of all trait implementations
   shows that boundary operations (transformation, telemetry, batch verification) are
   handled by the existing trait set without needing additional core abstractions.

2. **One historical dependency violation existed and is resolved.** `roko-conductor` ->
   `roko-learn` broke the L3->L2 rule. The Bus-mediated topic routing dissolved this
   coupling.

3. **Category theory provides formal grounding.** The pipeline is a composition of
   morphisms; Score is a monoid; cross-cuts are endofunctors with verified natural
   transformations (E44). These are structural properties, not metaphors.

4. **Active inference reframes the Gate.** The Gate is a prediction-error detector, not
   just pass/fail, enabling continuous model updating through the E25 learning loops.

5. **Three cognitive speeds are a genuine innovation.** The Delta speed (offline
   consolidation via Dreams) has no direct analog in established cognitive architectures.

---

## 2. Analysis A: Are the 12 Protocol Traits Sufficient?

### 2.1 Methodology

Analyzed all trait implementations found across the workspace:

| Trait | Implementations | Representative examples |
|---|---|---|
| Store | 4+ | MemorySubstrate, FileSubstrate, HdcSubstrate, ChainSubstrate |
| Score | 7+ | SumScorer, RelevanceScorer, ReputationScorer, NoOpScorer |
| Verify (Gate) | 19+ | ShellGate, PropertyTestGate, LlmJudgeGate, FactCheckGate, WalletGate |
| Route | 7+ | CascadeRouter, LinUCBRouter, WeightedRouter, RoundRobinRouter |
| Compose | 5+ | PromptComposer, ContextPackComposer, SystemPromptBuilder |
| React (Policy) | 4+ | EpisodePolicy, ConductorPolicy, PheromonPolicy |
| Bus | 2+ | InProcessBus, concrete runtime bus |
| Observe | 3+ | TelemetryObserver, MetricsObserver |
| Connect | 2+ | HTTP relay, supervised client |
| Trigger | 7+ | Cron, file watch, webhook, EVM ABI, chain finality |
| Substrate | 4+ | FileSubstrate, MemorySubstrate |
| ColdStore | 2+ | Cold-tier archival implementations |

No TODO/HACK/FIXME markers were found near core trait usage. All markers are in
UI/API boundary code.

### 2.2 Could Traits Be Merged?

| Candidate Merge | Argument For | Argument Against | Verdict |
|---|---|---|---|
| Scorer + Router | Both evaluate candidates | Router has `feedback()` (stateful); Scorer is stateless and pure | **No merge** |
| Gate + Scorer | Both assess quality | Gate is async (external I/O); Scorer is sync (pure computation). Different output types | **No merge** |
| Policy + Gate | Both examine outputs | Policy is reactive (many->many); Gate is verificatory (one->Verdict). Different cardinalities | **No merge** |
| Bus + Observe | Both handle events | Bus is transport fabric; Observe is passive instrumentation. Different contracts | **No merge** |

All merge candidates fail because the traits differ in at least two of: sync/async,
stateful/stateless, input cardinality, output type, and layer assignment.

### 2.3 Comparison to Other Agent Systems

| System | Core Abstractions | Roko Equivalent |
|---|---|---|
| **CoALA** (Sumers et al. 2023) | 5 stores + 3 action types | 12 traits subsume CoALA's decomposition |
| **LIDA** (Franklin et al. 2016, IEEE Trans. AMD 6(1)) | Codelets (perception, attention, action, learning) | Each codelet type maps to a trait implementation |
| **Google multi-agent patterns** (2025) | 3 execution primitives (sequential, loop, parallel) | Graph engine composes trait calls in these patterns |
| **Agent Design Pattern Catalogue** (Liu et al. 2024, arXiv:2405.10467) | 18 patterns | Patterns compose from trait implementations |

The 12-trait level is the right granularity for a Rust trait system: fine enough for
meaningful composition, coarse enough for human reasoning.

---

## 3. Analysis B: Five-Layer Taxonomy Coherence

### 3.1 Dependency Audit

The five-tier dependency model (T0 Leaf -> T1 Foundation -> T2 Service -> T3
Integration -> T4 Application) is enforced across all 39 workspace members.

**Clean layers (no violations)**:

- **T0** (roko-primitives, roko-mcp-stdio, roko-demo): Zero upward deps
- **T1** (roko-core, roko-lang-*): Depend only on T0
- **T2** (roko-agent, roko-fs, roko-graph, roko-daimon, roko-plugin): Depend on T0/T1
- **T3** (roko-compose, roko-learn, roko-gate, roko-neuro, roko-runtime): Depend on T0-T2
- **T4** (roko-cli, roko-serve, roko-acp): Depend on all layers (expected for entry points)

**Historical violation (resolved)**:

| From | To | Type | Resolution |
|---|---|---|---|
| `roko-conductor` (T3) | `roko-learn` (T3/Cross-cut) | Direct compile-time dep | Bus-mediated topic routing dissolved the coupling |

### 3.2 Layer-VSM Mapping

The five layers map cleanly to Beer's Viable System Model (Beer 1972):

| VSM System | Layer | Function | Clean? |
|---|---|---|---|
| S1 (Operations) | T0/T2 Runtime | Process lifecycle, I/O | Yes |
| S2 (Coordination) | T2 Framework | Prevent conflict between agents | Yes |
| S3 (Control) | T2/T3 Scaffold | Optimize resource allocation | Yes |
| S3* (Audit) | T3 Harness | Verify quality | Yes |
| S4 (Intelligence) | T4 Orchestration | Plan, adapt, look forward | Yes |
| S5 (Policy) | T4 + Cross-cuts | Identity, self-model | Partially: spans L4 and Daimon |

The only imperfect mapping is S5, which spans both orchestration and the Daimon
cross-cut. This is acceptable because Beer's VSM explicitly allows S5 to draw from
multiple subsystems.

---

## 4. Analysis C: Three Cognitive Speeds

### 4.1 Domain Mapping Completeness

| Domain | Gamma (~5s) | Theta (~75s) | Delta (~hours) | Clean? |
|---|---|---|---|---|
| **Coding** | Compile check, quick fix, cached lookup | Summarize progress, check predictions | Dreams replay of failed compilations | Yes |
| **Chain** | Gas check, balance check, price lookup | Portfolio assessment, hedging check | MEV incident analysis | Yes |
| **Research** | Citation lookup, fact check | Research direction assessment | Cross-domain hypothesis generation | Yes |
| **Orchestration** | Task status check, agent health probe | Plan progress summary | Full plan retrospective | Yes |

All domains map cleanly. The key insight: the three speeds are **domain-agnostic**
because they are defined in terms of the universal cognitive loop.

### 4.2 Comparison to Classical Architectures

| Architecture | Speeds | Roko Comparison |
|---|---|---|
| **SOAR** (Laird 2012) | 1 (~50ms decision cycle) | Roughly Gamma; impasses escalate but no explicit offline speed |
| **ACT-R** (Anderson 2007) | 1 (~50ms production fire) | Roughly Gamma; no reflective or consolidation speed |
| **LIDA** (Franklin et al. 2016) | 1 (~260-390ms cognitive cycle) | Roughly Gamma; deliberation is a subphase |
| **SOFAI** (Ollinger et al. 2024) | 2 (Fast/Slow) | Fast ~ Gamma, Slow ~ Theta; no Delta equivalent |
| **Roko** | 3 (Gamma/Theta/Delta) | Extends dual-process with offline consolidation |

The Delta speed (offline consolidation via Dreams) is a genuine architectural
innovation. It is inspired by sleep neuroscience (McClelland et al. 1995, CLS theory)
rather than cognitive architecture tradition.

### 4.3 Speed Interaction Model

```
Gamma ticks produce episodes -> stored in Substrate
    |
    +-- Theta reads recent episodes -> summarizes -> updates Daimon PAD
    |       |
    |       +-- PAD changes may trigger speed escalation or consolidation
    |
    +-- Delta reads accumulated episodes -> Dreams replay -> Neuro promotion
            |
            +-- Promoted knowledge available to next Gamma tick
```

This is a **hierarchical prediction error cascade**: Gamma handles immediate
surprises, Theta handles accumulated pattern changes, and Delta handles deep
structural learning.

---

## 5. Analysis D: Category Theory Perspectives

### 5.1 The Signal Category (Sig)

**Objects**: Types in the pipeline -- `Vec<Signal>`, `Signal`, `Score`, `Selection`,
`Verdict`

**Morphisms**: Trait operations, parameterized by `Context`:

```
query_ctx : 1 -> Vec<Signal>           (Store)
score_ctx : Signal -> Score            (Score)
select_ctx : Vec<Signal> -> Selection  (Route)
compose_ctx : (Vec<Signal>, Budget) -> Signal   (Compose)
verify_ctx : Signal -> Verdict         (Verify)
decide_ctx : Vec<Signal> -> Vec<Signal>         (React)
```

**Identity morphisms**: NoOp implementations (NoOpScorer, NoOpRouter, etc.)

**Composition**: Pipeline steps compose via standard function composition:

```
query >> select >> compose >> verify >> persist >> decide
```

### 5.2 Score as a Commutative Monoid

```
(Score, +, Score::ZERO)     -- additive identity: all axes = 0
(Score, x, Score::NEUTRAL)  -- multiplicative identity
```

Both operations are:
- **Associative**: (a + b) + c = a + (b + c)
- **Commutative**: a + b = b + a
- **Have identity**: a + 0 = a, a x 1 = a

The effective score formula is a monoid homomorphism from (Score, x) to (R+, x):

```
effective = sum(w_i * axis_i)    where sum(w_i) = 1.0
```

### 5.3 Verdict as a Filtered Monoid

Verdicts form a monoid under sequential composition (pipeline of gates):

```
verdict_1 * verdict_2 = {
    passed: verdict_1.passed && verdict_2.passed,
    score: min(verdict_1.score, verdict_2.score),
}
```

This is a filtered monoid: the `passed` field acts as a filter, and once any gate
fails, the pipeline short-circuits. This is the categorical dual of the Maybe monad.

### 5.4 Pipeline as Kleisli Composition

The full pipeline involves effects (async I/O, failure, state) modeled as Kleisli
composition:

```
Pipeline = Store.query >=> Route.select >=> Compose.compose >=> Verify.verify
           >=> Store.put >=> React.decide
```

Where `>=>` is Kleisli composition in the `Result<T, RokoError>` monad.

### 5.5 Cross-Cuts as Endofunctors (E44 Formalization)

Each cross-cut defines an endofunctor F: Sig -> Sig on the Signal category:

| Cross-Cut | Functor F | F(Router) | F(Composer) |
|---|---|---|---|
| **Memory** | Knowledge enrichment | Router with knowledge-informed selection | Composer with knowledge-enriched context |
| **Daimon** | Affect modulation | Router with PAD-biased tier selection | Composer with arousal-adjusted token budget |
| **Dreams** | Consolidation transformation | Router with replay-updated weights | Composer with consolidated knowledge |
| **Safety** | Capability filtering | Router with contract-bounded selection | Composer with taint-filtered context |

The natural transformations between functors (eta_MN, eta_NM, eta_MD, eta_ND, eta_DM,
eta_DN) form a commuting triangle verified by `debug_assert` in the gate-failure
cascade:

```
eta_MD . eta_NM === eta_ND    (Daimon -> Memory -> Dreams === Daimon -> Dreams)
```

### 5.6 VSA/HDC Algebraic Structure

The HDC vectors provide three operations mapping to category theory:

| HDC Operation | Categorical Analog | Knowledge Use |
|---|---|---|
| **Bind** (XOR) | Tensor product | Associating concept pairs |
| **Bundle** (majority vote) | Direct sum / coproduct | Combining related concepts |
| **Permute** (rotation) | Cyclic action | Sequencing by position |

These make the HDC vector space a proper Vector Symbolic Architecture (Kleyko et al.
2022), algebraically richer than a simple embedding space.

---

## 6. Analysis E: Active Inference Perspective

### 6.1 Gate as Prediction-Error Detector

The Free Energy Principle (Friston 2010, Nature Reviews Neuroscience 11) reframes the
Gate from a pass/fail verifier to a prediction-error detector. Each gate verdict
produces a prediction error signal:

```
prediction_error = expected_quality - actual_quality
```

This prediction error feeds back into the learning loops (E25):

- **Positive error** (better than expected): increase model confidence, promote
  knowledge tier
- **Negative error** (worse than expected): decrease model confidence, trigger
  replanning, fire gate-failure cascade
- **Zero error** (as expected): maintain current calibration

### 6.2 Free Energy Minimization in Routing

The CascadeRouter can be reframed as a free energy minimizer. It selects the model
that minimizes the expected free energy:

```
F = E[D_KL(q(o|pi) || p(o|m))] + E[H(o|s, pi)]
```

Where the first term is model complexity (cost) and the second is expected ambiguity
(quality uncertainty). The routing reward function already captures this balance:

```
reward = quality_weight * quality + cost_weight * (1 - cost/budget)
                                  + latency_weight * (1 - latency/sla)
```

### 6.3 E23 Cognitive Autonomy and EFE

E23 (Agent Cognitive Autonomy, 10/10) makes the active inference connection explicit
through Expected Free Energy (EFE) routing. CorticalState energy fields track the
agent's prediction precision, and the GoalTree maintains a hierarchy of predictions
that drive action selection.

---

## 7. Analysis F: Improvement Opportunities

### 7.1 Resolved Improvements

| Improvement | Resolution | Epic |
|---|---|---|
| Ad hoc cross-cut injection | E44 CrossCutFunctor trait with EnrichedCell composition | E44 8/8 |
| Missing safety contracts | E34 trust-origin lattice, TaintTracker, five-head corrigibility | E34 8/8 |
| No self-model for agents | E23 lifecycle type-state, CorticalState, EFE routing | E23 10/10 |
| Flat composition strategy | VCG attention auction with LearningBidder | E25 10/10 |
| No telemetry standardization | E33 Lens runtime with 39 ingress variants | E33 9/9 |
| No config evolution | E42 priority/provenance, migrations, profiles | E42 8/8 |
| Dual execution engines | Graph sole engine (#260 default, #276 retired WorkflowEngine) | Done |

### 7.2 Open Improvement Areas

| Area | Current state | Improvement path |
|---|---|---|
| LIDA-style competitive attention | VCG auction exists but not full competitive coalitions | Multiple Scorers forming competing attention coalitions |
| Formal Effect System | 12 traits approximate 12 effects | If Rust gains effect polymorphism, traits could be reframed as effects |
| Machine-checkable dep graph | Tier invariants documented but not CI-enforced | CI check that fails when dep graph drifts outside tier boundaries |
| Crate splits | roko-std, roko-compose remain combined | Target: roko-defaults + roko-tools, roko-compose-core + roko-templates |
| Full c-factor Bus instrumentation | CFactorSummary exists, full Bus measurement target-state | Wire continuous cohort measurement from live Bus traffic |
| Provider-owned internal screening | Host-visible outputs screened, provider internals not yet | Extend immune Graph to provider-owned calls/results |

---

## 8. Comparison with Agent Data Protocol (ADP)

The Agent Data Protocol (arXiv:2510.24702) addresses the same universal data
representation problem. Comparison with Roko's Signal:

| Dimension | ADP | Roko Signal |
|---|---|---|
| **Universal type** | Trajectory | Signal (backed by Engram struct) |
| **Identity** | Sequential index | Content-addressed (BLAKE3 hash) |
| **Quality assessment** | None | 7-axis Score |
| **Temporal dynamics** | None | Four Decay variants |
| **Trust tracking** | None | Provenance (author, trust, tainted, session) |
| **Composition** | Concatenation | Composer trait with budget constraints |
| **Complexity reduction** | O(D+A) vs O(D*A) | Same: universal type enables O(D+A) integration |

Roko's Signal is strictly richer than ADP's Trajectory: it adds scoring, decay,
provenance, content-addressing, lineage tracking, and HDC fingerprinting. The ADP
paper validates the core insight that a universal type reduces integration complexity
from multiplicative to additive.

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Beer 1972, "Brain of the Firm", Allen Lane | Viable System Model and recursive viability |
| Ashby 1956, "An Introduction to Cybernetics", Chapman & Hall | Law of Requisite Variety |
| Conant & Ashby 1970, IJSS 1(2) | Good Regulator Theorem |
| Friston 2010, Nature Reviews Neuroscience 11 | Free Energy Principle and active inference |
| McClelland et al. 1995, Psychological Review 102(3) | Complementary Learning Systems theory |
| Franklin et al. 2016, IEEE Trans. AMD 6(1) | LIDA: Systems-level Architecture for Cognition |
| Laird 2012, "The Soar Cognitive Architecture", MIT Press | SOAR decision cycle and impasse resolution |
| Anderson 2007, "How Can the Human Mind Occur?", OUP | ACT-R production system |
| Sumers et al. 2023 | CoALA: Cognitive Architectures for Language Agents |
| Liu et al. 2024, arXiv:2405.10467 | Agent Design Pattern Catalogue: 18 patterns |
| Kleyko et al. 2022, Artificial Intelligence Review 56 | Survey of HDC applications |
| Kanerva 2009, Cognitive Computation 1(2) | Hyperdimensional computing |
| Mac Lane 1971, "Categories for the Working Mathematician", Springer | Categorical foundations |
| Parnas 1972, CACM 15(12) | Information hiding |
| Ousterhout 2018, Yaknyam Press | Module depth |
| Plotkin & Power 2001, FoSSaCS, LNCS 2030 | Algebraic effects |
| Kahneman & Tversky 1979, Econometrica 47(2) | Prospect theory |
| Cemri et al. 2025, arXiv:2503.13657 | MAST: multi-agent failure taxonomy |

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- [cognitive-cross-cuts.md](cognitive-cross-cuts.md) -- E44 functor model details
- [design-principles-frontier-summary.md](design-principles-frontier-summary.md) -- Design principles
- [crate-map-and-dependencies.md](crate-map-and-dependencies.md) -- Dependency tier details
- [autocatalytic-and-cybernetics.md](autocatalytic-and-cybernetics.md) -- Compounding feedback loops
- [c-factor-collective-intelligence.md](c-factor-collective-intelligence.md) -- C-factor details
- `crates/roko-core/src/` -- 12 protocol traits
- `crates/roko-compose/src/cross_cut.rs` -- CrossCutFunctor trait
- `crates/roko-graph/` -- Sole execution engine
