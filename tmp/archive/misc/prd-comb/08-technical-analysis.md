
---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/00-vision-ta-generalized.md

# Technical Analysis as Universal Oracle Primitives

> TA is NOT chain-only. It is a general-purpose prediction framework with domain-specific instances.


> **Implementation**: Mixed

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [00-architecture](../00-architecture/INDEX.md) for Synapse Architecture, [06-neuro](../06-neuro/INDEX.md) for HDC knowledge encoding
**Key sources**: `refactoring-prd/03-cognitive-subsystems.md` §4, legacy source `bardo-backup/prd/23-ta/00-witness-as-technical-analyst.md`, `refactoring-prd/09-innovations.md`

---

## Abstract

Technical analysis (TA) originated as a financial discipline — chart patterns, moving averages, momentum oscillators applied to price data. In Roko, TA is generalized far beyond its financial origins into a set of **universal oracle primitives**: prediction, evaluation, calibration, and feedback loops that operate identically across any domain where an agent interacts with a verifiable external system.

The core insight: code, markets, research, and operations all share the same structural properties that make TA useful. They are structured systems with measurable state variables, feedback loops, non-stationary dynamics, and adversarial participants. A build time trend is structurally analogous to a price trend. A test failure probability is structurally analogous to a risk assessment. A dependency vulnerability score is structurally analogous to portfolio risk. The mathematics is the same; the domain vocabulary changes. That same measurement discipline is what lets TA distinguish a real moat from a feature list.

This document establishes the vision: TA as a domain-agnostic cognitive capability that any Roko agent can use, regardless of whether it operates on blockchains, codebases, research corpora, or any other structured domain. Under the two-mediums/two-fabrics framing in the [glossary](../00-architecture/01-naming-and-glossary.md), TA is one of the places where Roko's compounding and superlinear product claim becomes measurable: each prediction, correction, and replay cycle should make the next cycle cheaper, faster, and better. It is also the measurement and feedback layer for the structural moat described in [tmp/refinements/18-competitive-moat.md](../../tmp/refinements/18-competitive-moat.md): TA shows whether architectural coherence, heuristic commons, plugin ecosystem, replication ledger, and Rust-level correctness are compounding together or merely existing as separate features. REF19 adds the honesty test on top of that: which oracle-side primitives are genuinely net-new, which are carefully integrated from prior art, and which claims have enough evidence to support publication. See also [tmp/refinements/15-exponential-scaling.md](../../tmp/refinements/15-exponential-scaling.md) and [tmp/refinements/19-net-new-innovations.md](../../tmp/refinements/19-net-new-innovations.md).

> **Reality check**: Under the 10-primitive framing used in this chapter, 2
> primitives exist fully today (`Engram`, `Substrate`), 2 exist partially or in
> a narrower form than described here (HDC, c-factor), and 6 remain target-state
> (`Pulse`, a kernel `Bus` trait, demurrage, heuristic commons, replication
> ledger, plugin SPI). The moat framing in this chapter is aspirational.
>
> **Actual edge today**: the live product advantage is a working Rust agent
> orchestrator with multi-backend LLM dispatch, a 7-rung gate pipeline, HDC
> support in the learning/neuro stack, episode logging with feedback loops, and
> an interactive TUI.

## Moat telemetry

REF18 frames the target-state moat as five interacting components, not a checklist. TA is the
instrumentation that would tell us whether those components are reinforcing one another across
deployments and over time. The shared vocabulary for Engram, Pulse, Bus, Substrate, HDC
fingerprint, demurrage, and c-factor lives in the
[glossary](../00-architecture/01-naming-and-glossary.md).

| Moat component | What TA measures | Healthy curve | Drift curve |
|---|---|---|---|
| Architectural coherence | Whether Substrate, Bus, HDC fingerprinting, demurrage, heuristic calibration, and c-factor move together as one stack | Joint slopes improve together; no layer is winning while another stalls | Local wins that do not improve end-to-end reuse or coordination |
| Heuristic commons | Whether cross-deployment heuristics actually improve outcomes for later deployments | Reused heuristics tighten calibration and reduce trial count | More heuristics with no downstream lift, or noisy imports overwhelming the commons |
| Plugin ecosystem | Whether plugins create durable switching costs and broaden the set of measurable tasks | Adoption, retention, and breadth of domains increase together | Plugins exist but do not change workflows or stick after the first install |
| Replication ledger | Whether claims remain scientifically defensible across contexts and reruns | Replication rate and effect-size stability improve over time | Claims multiply faster than confirmed replications |
| Rust-level correctness | Whether the kernel keeps safety and performance guarantees at the boundary where competitors tend to rely on wrappers | Compile-time guarantees, latency, and failure rates stay tight | Glue code hides type mismatches, regressions, or safety leaks |

This is why TA matters here: not because it adds another surface to the product, but because it makes the moat observable. A deployment that looks impressive on one row and flat on the others is still easy to copy.

---

## Why generalize TA

### The structural analogy argument

Traditional TA works because financial markets have specific properties:

1. **Measurable state** — prices, volumes, open interest, volatility
2. **Time series dynamics** — trends, mean reversion, regime changes
3. **Feedback loops** — momentum traders amplify trends, mean-reversion traders dampen them
4. **Pattern recurrence** — similar market structures produce similar outcomes
5. **Adversarial dynamics** — participants adapt to each other's strategies
6. **External verification** — prices are observable, outcomes are deterministic

Codebases share every one of these properties:

1. **Measurable state** — compilation time, test pass rates, cyclomatic complexity, dependency counts
2. **Time series dynamics** — complexity trends, performance regression trajectories, test coverage drift
3. **Feedback loops** — tech debt accumulates → development slows → more shortcuts → more debt
4. **Pattern recurrence** — similar code structures produce similar bug patterns
5. **Adversarial dynamics** — security vulnerabilities, supply chain attacks
6. **External verification** — compilers, test suites, benchmarks produce objective outcomes

Research corpora share similar properties:

1. **Measurable state** — citation counts, publication velocity, contradiction density
2. **Time series dynamics** — field maturity, paradigm shifts, replication crises
3. **Feedback loops** — popular papers attract more citations → more visibility → more citations
4. **Pattern recurrence** — similar research methodologies produce similar reliability
5. **Adversarial dynamics** — p-hacking, selective reporting, predatory publishing
6. **External verification** — replication studies, meta-analyses, cross-validation

The structural analogy is not a metaphor — it is a mathematical fact. If we define TA as "systematic prediction from structured time series with feedback," then TA applies to any domain with those properties. Markets are one instance. Code is another. Research is a third. Roko treats all three identically at the trait level, specializing only in the domain-specific implementations.

### Cross-domain insight transfer

When a coding agent learns "high-churn modules need more review," it encodes this as an HDC vector:

```
BIND(high_complexity, more_review)
```

When a chain agent learns "high-volatility assets need more caution," it encodes:

```
BIND(high_volatility, more_caution)
```

Both encode the same abstract structure:

```
BIND(high_uncertainty, more_verification)
```

The Hamming similarity between these vectors is high because the HDC algebra preserves structural isomorphism. This means cross-domain insight transfer happens automatically through the Neuro (knowledge) system — a pattern learned in one domain can be detected as relevant in another.

This is the deepest justification for generalizing TA. If each domain used its own bespoke prediction framework, cross-domain transfer would require explicit translation. With universal oracle primitives, transfer happens through the HDC similarity space at nanosecond cost (Kleyko et al., 2022, ACM Computing Surveys).

### The Roko thesis: "the scaffold IS the product"

Roko's architectural thesis is that agent performance varies dramatically based on the surrounding harness — context engineering, verification, learning loops, cognitive architecture. Meta-Harness (Lee et al., 2026, arXiv:2603.28052) demonstrated +7.7 points on text classification and +4.7 on IMO-level math from harness optimization alone, at 4× fewer tokens.

TA generalization is a direct expression of this thesis. By making prediction, calibration, and feedback universal primitives — not domain-specific add-ons — every agent benefits from the same self-improving prediction infrastructure. The harness (oracle + calibration tracker + residual corrector) works identically whether the agent is predicting build times, gas prices, source reliability, or whether a moat component is actually compounding instead of merely present.

---

## Domain-agnostic prediction architecture

### Where oracles live in the Synapse Architecture

The Oracle subsystem operates across multiple layers of Roko's five-layer architecture:

| Layer | Oracle role | What it does |
|---|---|---|
| **L0 Runtime** | Prediction storage | `PredictionStore` persists predictions and outcomes |
| **L1 Framework** | Router integration | `Router.feedback()` uses prediction accuracy for bandit updates |
| **L2 Scaffold** | Context selection | EFE (Expected Free Energy) uses prediction confidence for context bidding |
| **L3 Harness** | Gate calibration | Prediction residuals calibrate adaptive gate thresholds |
| **L4 Orchestration** | Task prioritization | Prediction accuracy informs task scheduling priority |

The Oracle connects to all six Synapse traits:

- **Substrate** — `PredictionStore` persists predictions as Engrams via `Substrate.put()`
- **Scorer** — `PredictiveScorer` uses oracle accuracy to weight Engram relevance
- **Gate** — Prediction residuals feed adaptive gate thresholds (EMA per rung)
- **Router** — Accurate oracle predictions increase routing weight via `Router.feedback()`
- **Composer** — EFE-based context selection uses prediction confidence for VCG auction bids
- **Policy** — `PredictionPolicy` tracks accuracy, feeds back to all other traits

### The cognitive cross-cut dimension

Oracles are a cognitive cross-cut — they inject into multiple layers via trait objects, never hardcoded. The Daimon (motivation engine) modulates oracle behavior through PAD (Pleasure-Arousal-Dominance) state:

- **Low confidence (Dominance)** → Predictions are made conservatively, wider prediction intervals
- **High arousal** → More frequent prediction updates, faster residual correction
- **Low pleasure (after prediction failures)** → Automatic model recalibration

The Neuro (knowledge store) accumulates oracle patterns:

- Successful prediction strategies become `StrategyFragment` knowledge entries
- Systematic prediction biases become `Warning` entries
- Causal relationships discovered through prediction analysis become `CausalLink` entries

Dreams (offline learning) consolidate oracle performance:

- NREM replay evaluates which prediction patterns were reliable across episodes
- REM imagination generates counterfactual predictions to test oracle robustness
- Integration staging promotes validated prediction strategies to permanent knowledge

---

## The seven-step loop with oracles

The seven-step loop is where TA turns usage into compounding returns:

```
1. SENSE      → Substrate.query() + Bus.subscribe()
               Oracle reads state, prior Engrams, and live Pulses
2. ASSESS     → Scorer.score() + Router.select()
               PredictiveScorer weights uncertainty, drift, and likely payoff
3. COMPOSE    → Composer.compose()
               EFE-weighted context includes the smallest prediction-relevant slice
4. ACT        → Agent.execute()
               Agent emits prediction Pulses and final task output
5. VERIFY     → Gate.verify()
               Outcome closes the prediction and emits verdict Engrams/Pulses
6. PERSIST    → Substrate.put()
               Prediction, outcome, and calibration artifacts graduate to Engrams
   BROADCAST  → Bus.publish()
               Residuals, anomalies, and calibration Pulses feed other agents
7. REACT      → Policy.decide()
               Residual correction, heuristic updates, and routing changes fire
```

This mapping keeps TA aligned with the current architecture story in [00-architecture](../00-architecture/INDEX.md) and the moat framing in [tmp/refinements/18-competitive-moat.md](../../tmp/refinements/18-competitive-moat.md): Engrams carry durable prediction history in the Substrate, Pulses carry ephemeral prediction and outcome traffic on the Bus, and Policy closes the learning loop. See also [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for the detailed prediction-resolution-calibration path.

---

## Three cognitive speeds for oracles

Oracles operate at all three of Roko's cognitive timescales:

| Speed | Period | Oracle activity |
|---|---|---|
| **Gamma** (~5-15s) | Real-time | T0 probes evaluate prediction error scalar. No LLM. Zero cost. |
| **Theta** (~75s) | Reflection | Pending predictions resolved. Residuals computed. CalibrationTracker updated. |
| **Delta** (hours) | Consolidation | Cross-domain prediction patterns consolidated. Routing tables updated. |

At Gamma frequency, the 16 T0 probes (FrugalGPT-inspired; Chen et al., 2023, arXiv:2305.05176) compute a prediction error scalar that drives T0/T1/T2 cognitive tier routing:

```
error < 0.2  → T0 (suppress, no LLM)     ~80% of ticks
error < 0.6  → T1 (fast model, shallow)   ~15% of ticks
error ≥ 0.6  → T2 (full model, deep)      ~5% of ticks
```

This means 80% of cognitive cycles cost nothing — the oracle decides no action is needed based on pure Rust probes running in microseconds.

---

## Why TA is a compounding system

REF15 makes an explicit product claim: Roko should improve superlinearly on real workloads because multiple feedback loops reinforce each other. REF18 makes the defensibility claim: the system should become harder to copy because its moat components reinforce each other. Technical analysis is the instrumentation layer for both claims because each loop depends on prediction, verification, or calibration:

| REF15 loop | TA contribution | What should improve with use |
|---|---|---|
| Architectural coherence | TA checks whether the Substrate + Bus + HDC + demurrage + heuristic + c-factor stack is improving as one system | Lower copyability, fewer isolated wins |
| Demurrage-weighted retrieval | Oracles measure which memories still predict useful outcomes | Fewer wasted tokens, better retrieval precision |
| Heuristic calibration | Prediction/outcome joins tighten confidence intervals | Better priors on similar tasks |
| HDC codebook cleanup | Oracle outcomes add cleaner labels to each HDC fingerprint | Faster cache hits and better analogical matches |
| c-factor feedback | Shared prediction accuracy reveals which cohorts learn well together | Better routing across agents and domains |
| Playbook distillation | Repeated predictions collapse into reusable strategy templates | Lower time-to-solution for recurring task classes |
| Cross-deployment heuristic commons | Portable calibration data bootstraps new deployments | Better first-week performance on fresh installs |
| Plugin ecosystem | Each plugin adds new measurable state and new verification surfaces | More domains that feed the same learning machinery |

The point is not just that these loops exist. The point is that they are observable. If TA cannot show improving slopes, then the broader compounding claim has not actually landed. The same is true for the moat: if the five components do not improve together, there is no defensible structural advantage.

### Scaling dashboards

The technical-analysis chapter is the right home for the operator dashboards that REF15 calls out:

| Dashboard line | Interpretation | Failure signal |
|---|---|---|
| Retrieval quality vs. episode count | Demurrage and HDC are improving effective recall | Flat slope means retrieval is not learning |
| Mean calibration CI width per heuristic | Heuristic calibration is tightening with trials | Flat or rising width means the Calibrator lacks fresh trials |
| c-factor trend on sampled cohorts | Coordination is turning better predictions into better group output | Rising c with flat gate outcomes suggests reward hacking |
| HDC cleanup hit rate | The codebook is helping later retrieval and composition | Falling hit rate implies noisy fingerprints or poor cleanup |
| Mean time to first successful PR on a new codebase | Headline composite KPI across all loops | Flat curve means the product claim is not compounding |

### Anti-metrics

Superlinear capability gains are only credible if a few resource curves stay bounded:

| Anti-metric | Why it should stay flat or shrink |
|---|---|
| Warm-tier episode count | Demurrage should keep the working set bounded |
| Heuristic count with confirmations below 3 | Weak hypotheses should either be tested quickly or decay away |
| Mean lineage depth per response | Context depth should only grow when it buys quality |

If these anti-metrics blow out while the headline metrics improve, the system is probably cheating by hoarding context or retaining low-value memory.

### Evaluation guardrails

The compounding claim is only testable on real workloads with preserved state:

1. Evals must span sessions and days, not reset the Substrate between runs.
2. Each benchmark should be attempted multiple times so the slope of `time_to_solve` is measurable.
3. Commons-on versus commons-off runs should be compared explicitly to isolate the value of shared heuristics.
4. Operator dashboards should be read alongside task difficulty buckets so easier task selection cannot masquerade as progress.

Without those guardrails, TA can still report local prediction quality, but it cannot justify the stronger superlinear product claim.

---

## Net-New Innovation Lens For TA

REF19 reframes novelty claims as a flat catalog with three levels: primitive, pattern, and API. The technical-analysis chapter is where several of those claims become measurable rather than rhetorical, because this topic owns prediction, correction, calibration, and replay loops. See [tmp/refinements/19-net-new-innovations.md](../../tmp/refinements/19-net-new-innovations.md) for the canonical catalog.

### Primitive vs integration claims

Not every TA-facing capability is net-new in isolation. HDC fingerprinting, demurrage, prediction markets, and active inference all have prior art. The honest claim is that Roko turns them into one coherent oracle stack by running them through the same two mediums, two fabrics, and seven-step loop.

| TA-facing claim | Category | Why it matters here | Closest prior art |
|---|---|---|---|
| **HDC fingerprint** on every Engram used for oracle-side similarity and analogy | Integrated primitive | Lets coding, chain, and research oracles share one structural retrieval surface | Vector databases and HDC/VSA systems |
| **Demurrage** on durable prediction knowledge | Integrated primitive | Keeps stale calibration, weak hypotheses, and idle playbooks from dominating retrieval | TTL/LRU caches and recommender decay |
| **Heuristic with explicit falsifier** | Genuinely new primitive | Turns prediction guidance into something the oracle loop can actually disconfirm and recalibrate | Rule engines and retrieved tips without mandatory falsifiers |
| **Replication ledger** for design claims and oracle assumptions | Genuinely new primitive | Makes the runtime's own research basis auditable across deployments | Replication registries in science, not agent runtimes |
| **c-factor** as a runtime routing and dashboard signal | Genuinely new primitive | Measures whether cohort prediction quality is producing better group outcomes | Team dashboards and organizational-health metrics |
| **Predict-publish-correct loop** over Bus + Substrate | Integrated pattern | Makes every prediction a first-class learning cycle across agents and sessions | Forecasting pipelines and stream-processing systems |
| **Bus subscription API** for predictions and outcomes | Integrated API | Lets dashboards, audits, and external tools subscribe to the same live calibration traffic | Message-bus subscribers and monitoring feeds |

The target-state moat claim stays the same across those rows: a competitor can copy a local
feature, but copying the whole TA stack would mean reproducing the alignment among Engram,
Pulse, Bus, Substrate, HDC fingerprint, demurrage, heuristics with falsifiers, c-factor, and
the replication ledger. Today that alignment is only partially built, so this section should
be read as a research-hypothesis and instrumentation roadmap rather than a shipping-claims
table.

### TA's contribution to the net-new catalog

TA is the chapter where several REF19 entries either become visible to users or become empirically defensible:

| REF19 entry | TA chapter contribution | Home doc |
|---|---|---|
| Predict-publish-correct loops | Prediction registration, outcome resolution, residual correction, and calibration flow | [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) |
| HDC query API | Cross-domain similarity search over oracle artifacts and structural analogies | [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) |
| c-factor runtime signal | Shared oracle accuracy becomes a cohort health input instead of a vanity dashboard number | [../00-architecture/14-c-factor-collective-intelligence.md](../00-architecture/14-c-factor-collective-intelligence.md) |
| Replication ledger API | Research-oracle outputs can be joined to claims and rerun histories | [../05-learning/20-research-to-runtime.md](../05-learning/20-research-to-runtime.md) |
| Demurrage-taxed learned parameters | Calibration priors decay when unchallenged, enabling graceful relearning | [../00-architecture/04-decay-variants.md](../00-architecture/04-decay-variants.md) |

### Publishable claims ladder

REF19 identifies four claims that could plausibly clear a publication bar after enough deployment history. The TA topic is where the evidence plan for those claims lives because it owns the measurement discipline.

| Candidate paper | TA evidence needed | Likely adjacent doc |
|---|---|---|
| **c-factor measurement in agent systems** | Longitudinal cohort metrics showing that higher c-factor predicts better verified outcomes, not just more chatter | [../00-architecture/14-c-factor-collective-intelligence.md](../00-architecture/14-c-factor-collective-intelligence.md) |
| **Demurrage-based memory management for LLM agents** | Retrieval-quality gains with bounded warm-tier size and lower stale-memory interference | [../00-architecture/04-decay-variants.md](../00-architecture/04-decay-variants.md) |
| **Replication ledger as evidence-based engineering** | Per-claim history of reported effect vs observed deployment effect, with confidence intervals and reversals logged | [../05-learning/20-research-to-runtime.md](../05-learning/20-research-to-runtime.md) |
| **HDC compositional memory for code agents** | Cross-codebase similarity, analogy transfer, and latency measurements over real coding workloads | [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) |

Those are intentionally stronger than "interesting feature" claims. A publishable claim here needs live deployment evidence, preserved historical state, and explicit falsifiers for the claim being tested.

### Honest novelty rule

The chapter should keep making one distinction explicit:

- A **net-new primitive** is rare and should be claimed carefully.
- An **innovation by integration** is still valuable, but its novelty comes from fit, not from pretending the ingredients never existed.
- TA is the instrumentation layer that tells us when either claim has become true in practice.

---

## Why domain-specific instances matter

Universal primitives provide the architecture. Domain-specific instances provide the value. Each domain has unique:

- **State variables** to predict (prices vs. build times vs. citation counts)
- **Verification mechanisms** (blockchain finality vs. compiler output vs. replication studies)
- **Temporal dynamics** (block-time vs. CI pipeline cadence vs. publication cycles)
- **Adversarial threats** (MEV vs. supply chain attacks vs. p-hacking)
- **Feedback loop structure** (market impact vs. tech debt vs. citation cascades)

The Oracle trait abstracts over these differences. Domain-specific implementations handle the details. New domains are added by implementing the Oracle trait — not modifying the kernel.

---

## Academic foundations

- Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127-138. — Active inference framework underlying EFE-based context selection.
- Conant, R. C., & Ashby, W. R. (1970). "Every good regulator of a system must be a model of that system." *International Journal of Systems Science*, 1(2), 89-97. — Good Regulator Theorem motivating agent self-modeling via prediction.
- Sumers, T. R., et al. (2023). "Cognitive Architectures for Language Agents." arXiv:2309.02427. — CoALA framework that the Synapse Loop implements.
- Lee, S., et al. (2026). "Meta-Harness: Optimizing Harness, Not Model." arXiv:2603.28052. — +7.7 pts from scaffold optimization alone.
- Chen, L., et al. (2023). "FrugalGPT: How to Use Large Language Models While Reducing Cost and Improving Performance." arXiv:2305.05176. — Cascade architectures for cost reduction.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6), 1-51. — HDC foundations for cross-domain transfer.
- Kanerva, P. (2009). "Hyperdimensional Computing: An Introduction to Computing in Distributed Representation with High-Dimensional Random Vectors." *Cognitive Computation*, 1(2), 139-159. — Binary spatter codes for pattern encoding.
- Pirolli, P., & Card, S. (1999). "Information foraging." *Psychological Review*, 106(4), 643-675. — Optimal foraging theory applied to information retrieval, inspiring predictive foraging.
- Charnov, E. L. (1976). "Optimal foraging: the marginal value theorem." *Theoretical Population Biology*, 9, 129-136. — Stopping rule for context retrieval.

---

## Current status and gaps

**What exists today**:
- `CascadeRouter` in `roko-learn` implements tier-based model routing with UCB1 and LinUCB
- Adaptive gate thresholds with EMA per rung in `.roko/learn/gate-thresholds.json`
- Efficiency events logged per turn in `.roko/learn/efficiency.jsonl`
- Prompt experiments via `ExperimentStore` in `.roko/learn/experiments.json`

**What's scaffold/planned**:
- Full `Oracle` trait implementation (defined in `refactoring-prd/03-cognitive-subsystems.md`)
- `PredictionStore` with on-chain and off-chain variants
- `CalibrationTracker` per (model, task_category) pair
- Domain-specific oracle implementations (chain, coding, research)
- Active inference state space (factorized POMDP, 90 states)
- EFE-based context selection with pragmatic + epistemic - ambiguity decomposition
- Cross-session scaling dashboards for compounding, anti-metrics, and commons-on/off comparisons

See `tmp/implementation-plans/modelrouting/12-advanced-patterns.md` for the Thompson Sampling and predictive foraging calibration implementation plan. See also [tmp/refinements/15-exponential-scaling.md](../../tmp/refinements/15-exponential-scaling.md) for the canonical superlinear-scaling framing this chapter now measures.

---

## Cross-References

- See [01-oracle-trait.md](./01-oracle-trait.md) for the full Rust trait signature and Prediction struct
- See [02-chain-oracles.md](./02-chain-oracles.md) for chain-specific TA primitives
- See [03-coding-oracles.md](./03-coding-oracles.md) for coding equivalents of TA primitives
- See [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for the full prediction loop
- See [../00-architecture/01-naming-and-glossary.md](../00-architecture/01-naming-and-glossary.md) for the current two-mediums/two-fabrics vocabulary
- See topic [00-architecture](../00-architecture/INDEX.md) for the Synapse Architecture overview
- See topic [05-learning](../05-learning/INDEX.md) for the cybernetic feedback loops
- See topic [06-neuro](../06-neuro/INDEX.md) for HDC cross-domain transfer
- See [../../tmp/refinements/18-competitive-moat.md](../../tmp/refinements/18-competitive-moat.md) for the structural moat synthesis this chapter measures
- See [../../tmp/refinements/15-exponential-scaling.md](../../tmp/refinements/15-exponential-scaling.md) for the canonical REF15 proposal
- See [../../tmp/refinements/19-net-new-innovations.md](../../tmp/refinements/19-net-new-innovations.md) for the net-new innovation catalog this chapter helps validate


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/01-oracle-trait.md

# The Oracle Trait — Universal Prediction Interface

> Every domain-specific prediction system in Roko implements a single trait. This document specifies the full Rust signature, supporting types, and integration points.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [00-vision-ta-generalized](./00-vision-ta-generalized.md) for motivation, [00-architecture](../00-architecture/INDEX.md) for Synapse traits and Engram
**Key sources**: `refactoring-prd/03-cognitive-subsystems.md` §4, `refactoring-prd/01-synapse-architecture.md`

---

## The Oracle trait

The `Oracle` trait is the single interface through which all prediction capabilities are expressed. It is async, object-safe (`Send + Sync`), and designed for composition with the six Synapse traits:

```rust
/// Universal prediction interface for any domain.
///
/// Chain oracles predict prices, gas, liquidity depth, MEV opportunities.
/// Coding oracles predict build times, test failures, complexity drift.
/// Research oracles predict source reliability, contradiction density.
/// Custom domains implement the same trait with domain-specific queries.
pub trait Oracle: Send + Sync {
    /// Make a prediction about future state.
    ///
    /// The query encodes WHAT to predict. The context encodes the agent's
    /// current cognitive state (PAD vector, active knowledge, recent history).
    /// The returned Prediction includes confidence bounds and a time horizon
    /// by which the prediction should be resolved.
    async fn predict(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<Prediction>;

    /// Evaluate a past prediction against the actual outcome.
    ///
    /// The outcome is an Engram produced by external verification —
    /// a compiler result, a blockchain state, a test suite output, a
    /// replication study. The returned PredictionAccuracy drives feedback
    /// into Router, Daimon, Neuro, and Gate subsystems.
    async fn evaluate(
        &self,
        prediction: &Prediction,
        outcome: &Engram,
    ) -> Result<PredictionAccuracy>;
}
```

Both methods are async because predictions may require I/O — querying a blockchain node, running a compilation probe, or fetching citation data. The trait is `Send + Sync` to allow concurrent prediction evaluation across multiple oracle instances.

### Design rationale

The Oracle trait deliberately does **not** include:

- **`subscribe()`** — Real-time streams are handled by `Substrate.query()` with watch semantics. Oracles predict; Substrates observe.
- **`calibrate()`** — Calibration is a `Policy` concern. The `CalibrationTracker` (see [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md)) wraps any Oracle and adds calibration as a separate layer, following the Synapse Architecture's composition principle.
- **`batch_predict()`** — Batch semantics are provided by the caller iterating over queries. Oracle implementations may internally batch for efficiency (e.g., a chain oracle batching RPC calls), but this is an implementation detail, not a trait concern.

This follows Ousterhout's "deep module" principle (Ousterhout, 2018, *A Philosophy of Software Design*) — the interface is narrow (2 methods), but the implementation depth is substantial.

---

## OracleQuery — What to predict

The `OracleQuery` struct encodes the prediction request. It is domain-agnostic at the top level, with domain-specific payloads carried in the `domain` field:

```rust
/// A request for a prediction.
///
/// The query specifies WHAT to predict (domain-specific payload),
/// at what confidence level, over what time horizon.
pub struct OracleQuery {
    /// Unique identifier for this query (content-addressed, BLAKE3).
    pub id: ContentHash,

    /// What domain this prediction belongs to.
    pub domain: OracleDomain,

    /// The specific question being asked, as a domain-specific payload.
    /// Chain: "What will ETH price be in 5 blocks?"
    /// Coding: "Will this change break the test suite?"
    /// Research: "Is this source reliable for this claim?"
    pub payload: QueryPayload,

    /// How far into the future the prediction should cover.
    pub horizon: Duration,

    /// Minimum acceptable confidence for the prediction to be useful.
    /// Below this threshold, the oracle should return `Err(LowConfidence)`.
    pub min_confidence: f64,

    /// Tags for categorization (used by CalibrationTracker to track
    /// per-category accuracy).
    pub tags: BTreeMap<String, String>,

    /// Timestamp of query creation.
    pub created_at_ms: i64,
}
```

### OracleDomain — Domain classification

```rust
/// The domain a prediction belongs to.
///
/// Used by CalibrationTracker to maintain per-(model, domain) accuracy
/// statistics, and by the Router to select appropriate oracle implementations.
#[non_exhaustive]
pub enum OracleDomain {
    /// On-chain TA: price, gas, liquidity, MEV, protocol health.
    Chain,

    /// Software engineering: build time, test failure, complexity, dependency risk.
    Coding,

    /// Research and information analysis: source reliability, completeness, contradiction.
    Research,

    /// Operations: deployment success, infrastructure health, latency prediction.
    Operations,

    /// User-defined domain with a string identifier.
    Custom(String),
}
```

The `#[non_exhaustive]` attribute ensures new domains can be added without breaking existing code. The `Custom(String)` variant allows users to define domains not anticipated by the framework.

### QueryPayload — Domain-specific prediction targets

```rust
/// The specific prediction target.
///
/// Each variant carries domain-specific fields that the corresponding
/// Oracle implementation knows how to interpret.
pub enum QueryPayload {
    /// Chain domain predictions.
    Chain(ChainQueryPayload),

    /// Coding domain predictions.
    Coding(CodingQueryPayload),

    /// Research domain predictions.
    Research(ResearchQueryPayload),

    /// Operations domain predictions.
    Operations(OperationsQueryPayload),

    /// Arbitrary JSON payload for custom domains.
    Custom(serde_json::Value),
}

/// Chain-specific prediction targets.
pub struct ChainQueryPayload {
    /// The asset or protocol to predict about.
    pub target: ChainTarget,

    /// The metric to predict (price, gas, tvl, liquidity_depth, mev_opportunity).
    pub metric: ChainMetric,

    /// Optional: specific conditions to check (e.g., "if ETH > $3000").
    pub conditions: Vec<ChainCondition>,
}

/// Coding-specific prediction targets.
pub struct CodingQueryPayload {
    /// The scope of the prediction (file, module, crate, workspace).
    pub scope: CodingScope,

    /// The metric to predict (build_time, test_pass_rate, complexity_delta,
    /// dependency_risk, perf_regression).
    pub metric: CodingMetric,

    /// The change set that triggers this prediction (if applicable).
    pub change_context: Option<ChangeContext>,
}

/// Research-specific prediction targets.
pub struct ResearchQueryPayload {
    /// The source being evaluated.
    pub source: SourceReference,

    /// The metric to predict (reliability, completeness, contradiction_risk,
    /// replication_probability).
    pub metric: ResearchMetric,

    /// The claim or topic being assessed.
    pub claim_context: Option<String>,
}
```

---

## Prediction — The output

The `Prediction` struct is what an Oracle returns. It is designed to be stored as an Engram (via `Substrate.put()`) and later resolved against actual outcomes:

```rust
/// A prediction about future state.
///
/// Predictions are stored as Engrams with `kind: Kind::Prediction`.
/// They are resolved when the time horizon elapses or when external
/// verification produces an outcome.
pub struct Prediction {
    /// Content-addressed ID (BLAKE3 of query_id + predicted_value + confidence + horizon).
    pub id: ContentHash,

    /// The query this prediction answers.
    pub query_id: ContentHash,

    /// The predicted value. Domain-specific interpretation.
    ///
    /// Chain: PredictedValue::Numeric(3245.50)  // ETH price
    /// Coding: PredictedValue::Probability(0.85)  // test pass rate
    /// Research: PredictedValue::Ordinal(Reliability::High)  // source reliability
    pub value: PredictedValue,

    /// Confidence in this prediction, [0.0, 1.0].
    ///
    /// Maps to the Engram Score.confidence axis.
    /// Fed into the VCG auction as a bid weight for prediction context.
    pub confidence: f64,

    /// Prediction interval — the range within which the actual value
    /// is expected to fall with the stated confidence.
    ///
    /// For Probability predictions, this is a credible interval.
    /// For Numeric predictions, this is a prediction interval.
    pub interval: Option<PredictionInterval>,

    /// When this prediction was made.
    pub created_at_ms: i64,

    /// When this prediction should be resolved.
    /// After this time, the PredictionStore marks it for evaluation.
    pub resolve_by_ms: i64,

    /// The model and oracle that produced this prediction.
    /// Used by CalibrationTracker for per-model accuracy tracking.
    pub provenance: PredictionProvenance,

    /// Lineage — which Engrams informed this prediction.
    /// Enables causal replay: "why did the oracle predict X?"
    pub lineage: Vec<ContentHash>,

    /// Resolution state. None until resolved.
    pub outcome: Option<PredictionOutcome>,
}
```

### PredictedValue — Domain-polymorphic values

```rust
/// The value being predicted.
///
/// Supports numeric (prices, times), probability (pass rates, risk scores),
/// categorical (reliability levels), and compound (multiple related values).
pub enum PredictedValue {
    /// A numeric value (price, time, count).
    Numeric(f64),

    /// A probability [0.0, 1.0].
    Probability(f64),

    /// An ordinal category with associated numeric rank.
    Ordinal { label: String, rank: u32 },

    /// A boolean prediction (will it happen or not).
    Binary(bool),

    /// Multiple related predictions (e.g., price + volume + volatility).
    Compound(BTreeMap<String, PredictedValue>),
}
```

### PredictionInterval — Uncertainty quantification

```rust
/// Prediction interval bounding the expected outcome range.
///
/// The Oracle should produce intervals that are well-calibrated:
/// a 90% interval should contain the actual value 90% of the time.
/// CalibrationTracker measures this and adjusts via residual correction.
pub struct PredictionInterval {
    /// Lower bound of the prediction interval.
    pub lower: f64,

    /// Upper bound of the prediction interval.
    pub upper: f64,

    /// The coverage probability this interval targets (e.g., 0.90 for 90%).
    pub coverage: f64,
}
```

---

## PredictionAccuracy — The feedback signal

When a prediction resolves, the Oracle's `evaluate()` method returns a `PredictionAccuracy` that drives feedback into every Synapse subsystem:

```rust
/// The accuracy of a resolved prediction.
///
/// This is the primary feedback signal for the entire predictive foraging
/// loop. It feeds into:
/// - Router: accurate oracles get higher routing weight
/// - Daimon: prediction errors update Dominance (confidence)
/// - Neuro: prediction patterns become knowledge entries
/// - Gate: calibrate adaptive thresholds via EMA
/// - CalibrationTracker: update per-(model, category) bias estimates
pub struct PredictionAccuracy {
    /// The prediction being evaluated.
    pub prediction_id: ContentHash,

    /// The actual outcome Engram.
    pub outcome_id: ContentHash,

    /// Scalar accuracy [0.0, 1.0].
    /// 1.0 = perfect prediction. 0.0 = maximally wrong.
    pub accuracy: f64,

    /// Signed residual: predicted_value - actual_value.
    /// Positive = overestimated. Negative = underestimated.
    /// Used by ResidualCorrector for bias correction.
    pub residual: f64,

    /// Whether the prediction interval contained the actual value.
    /// Used by CalibrationTracker to measure interval calibration.
    pub interval_hit: Option<bool>,

    /// Time between prediction and resolution.
    /// Used to evaluate prediction quality at different horizons.
    pub resolution_lag_ms: i64,

    /// The domain and category for per-category tracking.
    pub domain: OracleDomain,
    pub category: String,
}
```

### PredictionOutcome — Resolution state

```rust
/// The resolution of a prediction.
pub struct PredictionOutcome {
    /// The actual value observed.
    pub actual: PredictedValue,

    /// The Engram that constitutes the evidence (compiler output, block data, etc.).
    pub evidence_id: ContentHash,

    /// When the outcome was observed.
    pub resolved_at_ms: i64,

    /// The accuracy assessment.
    pub accuracy: PredictionAccuracy,
}
```

---

## Integration with the Synapse traits

The Oracle trait is not a seventh Synapse trait — it is a **cognitive cross-cut** that integrates with all six traits through well-defined injection points:

### Substrate integration

Predictions and outcomes are persisted as Engrams:

```rust
// Store a new prediction
let prediction = oracle.predict(&query, &ctx).await?;
let engram = Engram::builder()
    .kind(Kind::Prediction)
    .body(Body::Json(serde_json::to_value(&prediction)?))
    .tag("domain", prediction.provenance.domain.as_str())
    .tag("horizon_ms", prediction.resolve_by_ms.to_string())
    .score(Score {
        confidence: prediction.confidence,
        novelty: 0.5,  // predictions start at baseline novelty
        utility: 0.0,  // utility accumulates after resolution
        reputation: prediction.provenance.model_reputation,
        ..Default::default()
    })
    .lineage(prediction.lineage.clone())
    .build();
substrate.put(engram).await?;
```

### Scorer integration — PredictiveScorer

The `PredictiveScorer` uses oracle accuracy history to weight Engram relevance:

```rust
/// Scores Engrams based on how well the oracle that produced them
/// has been performing recently.
pub struct PredictiveScorer {
    calibration: Arc<CalibrationTracker>,
}

impl Scorer for PredictiveScorer {
    fn score(&self, engram: &Engram) -> Score {
        let model = engram.provenance.model_id();
        let category = engram.tag("task_category").unwrap_or("unknown");

        // Oracle accuracy history modulates confidence
        let calibration = self.calibration.get_accuracy(model, category);
        let mut score = engram.score.clone();
        score.confidence *= calibration.recent_accuracy;
        score
    }
}
```

### Router integration

Prediction accuracy feeds into `Router.feedback()`, updating bandit arms for model selection:

```rust
// After prediction resolution
let accuracy = oracle.evaluate(&prediction, &outcome).await?;
router.feedback(
    &prediction.provenance.model_id,
    accuracy.accuracy,  // reward signal
)?;
```

This is how the CascadeRouter (with LinUCB + Thompson Sampling, see `roko-learn`) learns which models are best at predicting in which domains — the same bandit mechanism that routes LLM inference also routes oracle predictions.

### Gate integration

Prediction residuals calibrate adaptive gate thresholds:

```rust
// Residual correction updates gate thresholds
let residual = accuracy.residual;
gate_thresholds.update_ema(
    accuracy.category.as_str(),
    residual.abs(),
    alpha: 0.1,  // EMA smoothing factor
);
```

This creates a direct feedback loop: if an oracle consistently overestimates test pass rates, the gate threshold for that category tightens automatically.

### Composer integration — EFE bidding

Oracle predictions participate in the VCG attention auction (Vickrey 1961, Clarke 1971, Groves 1973) through Expected Free Energy (EFE) decomposition:

```rust
// Prediction context bids for attention budget
let efe = pragmatic_value + epistemic_value - ambiguity;
let bid = efe * urgency * affect_weight;
composer.bid("oracle_predictions", bid, prediction_context);
```

High-uncertainty predictions bid more aggressively because resolving them has high epistemic value (Friston, 2010, *Nature Reviews Neuroscience*).

### Policy integration — PredictionPolicy

The `PredictionPolicy` observes prediction streams and emits new Engrams based on patterns:

```rust
/// Watches prediction accuracy streams and generates meta-predictions,
/// warnings, and routing recommendations.
pub struct PredictionPolicy {
    tracker: Arc<CalibrationTracker>,
    neuro: Arc<dyn Substrate>,
}

impl Policy for PredictionPolicy {
    fn decide(&self, engrams: &[Engram]) -> Vec<Engram> {
        let mut outputs = Vec::new();

        // Detect systematic bias
        let bias = self.tracker.mean_residual("coding", "test_prediction");
        if bias.abs() > 0.15 {
            outputs.push(Engram::warning(
                format!("Systematic prediction bias detected: {:.2} in coding/test_prediction", bias),
            ));
        }

        // Detect accuracy degradation
        let trend = self.tracker.accuracy_trend("chain", "price");
        if trend < -0.05 {  // accuracy dropping
            outputs.push(Engram::insight(
                "Chain price prediction accuracy declining — possible regime change",
            ));
        }

        outputs
    }
}
```

---

## PredictionStore — Persistence layer

The `PredictionStore` manages the lifecycle of predictions from creation to resolution:

```rust
/// Manages prediction lifecycle: register → track → resolve → feedback.
///
/// Built on top of Substrate for persistence.
/// Provides efficient querying by domain, horizon, and resolution status.
pub struct PredictionStore {
    substrate: Arc<dyn Substrate>,
    pending: DashMap<ContentHash, Prediction>,
    resolved: DashMap<ContentHash, PredictionOutcome>,
}

impl PredictionStore {
    /// Register a new prediction for tracking.
    pub async fn register(&self, prediction: Prediction) -> Result<()>;

    /// Get all predictions that should have resolved by now.
    pub async fn pending_resolutions(&self) -> Vec<Prediction>;

    /// Resolve a prediction with an observed outcome.
    pub async fn resolve(
        &self,
        prediction_id: &ContentHash,
        outcome: &Engram,
        oracle: &dyn Oracle,
    ) -> Result<PredictionAccuracy>;

    /// Get accuracy statistics for a given domain and category.
    pub async fn accuracy_stats(
        &self,
        domain: &OracleDomain,
        category: &str,
    ) -> AccuracyStats;

    /// Get all unresolved predictions for a given domain.
    pub async fn pending_for_domain(
        &self,
        domain: &OracleDomain,
    ) -> Vec<Prediction>;
}
```

The `PredictionStore` has both off-chain (JSONL via `roko-fs`) and on-chain (Korai smart contract) variants. The on-chain variant enables collective calibration — all agents in the mesh share prediction outcomes, achieving up to 31.6× faster calibration for new agents (see [00-vision-ta-generalized.md](./00-vision-ta-generalized.md); collective calibration math in `refactoring-prd/09-innovations.md` §VI with explicit caveats about the independence assumption).

---

## ResidualCorrector — Bias elimination

The `ResidualCorrector` is a lightweight arithmetic layer that adjusts oracle predictions based on historical bias:

```rust
/// Corrects oracle predictions by subtracting the estimated systematic bias.
///
/// Cost: ~50 nanoseconds per correction (pure arithmetic, no LLM).
/// This is the mechanism that makes predictive foraging cost-effective —
/// the correction is free, but the learning is real.
pub struct ResidualCorrector {
    /// Mean bias per (model, category) pair.
    biases: DashMap<(String, String), ExponentialMovingAverage>,
}

impl ResidualCorrector {
    /// Apply bias correction to a raw prediction.
    pub fn correct(&self, prediction: &mut Prediction) {
        let key = (
            prediction.provenance.model_id.clone(),
            prediction.query_id_category(),
        );
        if let Some(bias) = self.biases.get(&key) {
            if let PredictedValue::Numeric(ref mut v) = prediction.value {
                *v -= bias.current();  // subtract estimated bias
            }
        }
    }

    /// Update bias estimate with a new residual.
    pub fn update(&self, model: &str, category: &str, residual: f64) {
        let key = (model.to_string(), category.to_string());
        self.biases
            .entry(key)
            .or_insert_with(|| ExponentialMovingAverage::new(0.1))
            .value_mut()
            .update(residual);
    }
}
```

The cost profile is critical: 50 nanoseconds per correction means 1,000 corrections per day per agent costs effectively nothing. This is what makes the predictive foraging loop viable at Gamma frequency (~5-15s) — corrections happen in microseconds, not milliseconds.

---

## CalibrationTracker — Per-model accuracy tracking

The `CalibrationTracker` aggregates prediction accuracy across (model, task_category) pairs, enabling bias-aware routing:

```rust
/// Tracks prediction calibration per (model, task_category) pair.
///
/// The key insight: different models have different biases on different
/// task categories. GPT-4 might overestimate test pass rates but
/// underestimate build times. Claude might do the reverse. The
/// CalibrationTracker learns these patterns and feeds them to the
/// ResidualCorrector and CascadeRouter.
pub struct CalibrationTracker {
    /// Per-(model, category) accuracy statistics.
    stats: DashMap<(String, String), CalibrationStats>,
}

pub struct CalibrationStats {
    /// Exponential moving average of residuals (bias estimate).
    pub mean_residual: ExponentialMovingAverage,

    /// Exponential moving average of absolute residuals (accuracy estimate).
    pub mean_absolute_error: ExponentialMovingAverage,

    /// Count of resolved predictions in this category.
    pub count: u64,

    /// Recent accuracy trend (positive = improving, negative = degrading).
    pub trend: f64,

    /// Interval calibration: fraction of outcomes within prediction intervals.
    pub interval_coverage: ExponentialMovingAverage,
}
```

On-chain (Korai), the CalibrationTracker is shared across all agents. A new agent importing the collective calibration starts with pre-learned biases for every model-category pair the collective has encountered. This is the concrete mechanism behind the 31.6× faster calibration heuristic (see `refactoring-prd/09-innovations.md` §VI).

---

## Implementing a custom Oracle

Adding prediction capability for a new domain requires implementing the `Oracle` trait:

```rust
/// Example: a deployment oracle that predicts deployment success probability.
pub struct DeploymentOracle {
    history: Arc<PredictionStore>,
    corrector: Arc<ResidualCorrector>,
}

#[async_trait]
impl Oracle for DeploymentOracle {
    async fn predict(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<Prediction> {
        let payload = query.payload.as_operations()?;

        // Gather features: recent deployment history, change size, time of day
        let features = self.extract_features(payload, ctx).await?;

        // Base prediction from historical success rate
        let mut prediction = Prediction {
            id: ContentHash::compute(&query, &features),
            query_id: query.id,
            value: PredictedValue::Probability(features.base_success_rate),
            confidence: features.sample_confidence,
            interval: Some(PredictionInterval {
                lower: features.base_success_rate - features.std_dev,
                upper: (features.base_success_rate + features.std_dev).min(1.0),
                coverage: 0.68,  // 1-sigma interval
            }),
            created_at_ms: now_ms(),
            resolve_by_ms: now_ms() + payload.expected_duration_ms,
            provenance: PredictionProvenance::local("deployment-oracle"),
            lineage: features.source_engrams,
            outcome: None,
        };

        // Apply bias correction from CalibrationTracker
        self.corrector.correct(&mut prediction);

        Ok(prediction)
    }

    async fn evaluate(
        &self,
        prediction: &Prediction,
        outcome: &Engram,
    ) -> Result<PredictionAccuracy> {
        let actual_success = outcome.tag("deployment_success")
            .map(|v| v == "true")
            .unwrap_or(false);

        let predicted_prob = prediction.value.as_probability()?;
        let actual_value = if actual_success { 1.0 } else { 0.0 };

        let accuracy = PredictionAccuracy {
            prediction_id: prediction.id,
            outcome_id: outcome.id,
            accuracy: 1.0 - (predicted_prob - actual_value).abs(),
            residual: predicted_prob - actual_value,
            interval_hit: prediction.interval.as_ref().map(|i| {
                actual_value >= i.lower && actual_value <= i.upper
            }),
            resolution_lag_ms: outcome.created_at_ms - prediction.created_at_ms,
            domain: OracleDomain::Operations,
            category: "deployment".to_string(),
        };

        // Update ResidualCorrector
        self.corrector.update(
            &prediction.provenance.model_id,
            "deployment",
            accuracy.residual,
        );

        Ok(accuracy)
    }
}
```

This pattern — predict, resolve, correct, repeat — is the same regardless of domain. The Oracle trait's simplicity (2 methods) hides substantial implementation depth in the supporting types (`PredictionStore`, `ResidualCorrector`, `CalibrationTracker`), following Ousterhout's deep module principle.

---

## Oracle Calibration and Composition Theory

The Oracle trait defines prediction and evaluation. This section addresses the next layer: how to **compose** multiple oracles into calibrated ensembles, how to provide **finite-sample coverage guarantees** via conformal prediction, and how to **diagnose calibration quality** through Brier score decomposition.

These techniques are essential for production oracle systems where no single predictor is uniformly best. Composition extracts value from diverse oracles; conformal prediction provides distribution-free uncertainty quantification; Brier decomposition separates calibration from discrimination, enabling targeted improvements.

---

### Oracle Composition — Combining Multiple Oracles

```rust
/// Compose multiple oracles into a single calibrated meta-oracle.
///
/// Oracle composition follows the theory of expert aggregation
/// (Cesa-Bianchi & Lugosi, 2006, "Prediction, Learning, and Games")
/// and conformal prediction (Vovk et al., 2005).
///
/// Three composition strategies:
/// 1. Weighted ensemble: predictions weighted by calibration quality
/// 2. Conformal aggregation: produce prediction sets with coverage guarantees
/// 3. Stacking: meta-learner over oracle predictions
pub struct OracleComposer {
    /// Individual oracle instances.
    pub oracles: Vec<Arc<dyn Oracle>>,
    /// Per-oracle calibration quality (Brier score decomposition).
    pub calibration_quality: Vec<CalibrationQuality>,
    /// Composition strategy.
    pub strategy: CompositionStrategy,
}

pub struct CalibrationQuality {
    /// Brier score decomposition (Murphy, 1973).
    /// Brier = Reliability - Resolution + Uncertainty
    pub reliability: f64,     // lower is better (0 = perfect calibration)
    pub resolution: f64,      // higher is better (distinguishes outcomes)
    pub uncertainty: f64,     // base rate uncertainty (irreducible)
    pub brier_score: f64,     // overall score = REL - RES + UNC
    /// Expected Calibration Error (Naeini et al., 2015).
    /// ECE = Σ (|B_m|/n) |acc(B_m) - conf(B_m)|
    pub ece: f64,
    /// Maximum Calibration Error.
    pub mce: f64,
    /// Calibration sharpness (resolution of prediction intervals).
    pub sharpness: f64,
}

pub enum CompositionStrategy {
    /// Weighted average by inverse Brier score.
    WeightedEnsemble,
    /// Conformal prediction: output prediction set with coverage α.
    /// Vovk, Gammerman & Shafer (2005), "Algorithmic Learning in a Random World"
    Conformal { target_coverage: f64 },
    /// Isotonic regression recalibration (Zadrozny & Elkan, 2002).
    IsotonicRecalibration,
    /// Platt scaling (Platt, 1999) — logistic recalibration.
    PlattScaling,
    /// Temperature scaling (Guo et al., 2017) — single parameter.
    TemperatureScaling { temperature: f64 },
}
```

The `OracleComposer` holds a vector of oracle instances alongside their calibration diagnostics. The `CompositionStrategy` enum captures the principal approaches from the calibration literature:

- **WeightedEnsemble** uses inverse Brier scores as weights, so better-calibrated oracles contribute more. This is the simplest strategy and works well when oracles are approximately independent.
- **Conformal** wraps the ensemble output in a prediction set with a coverage guarantee. The `target_coverage` parameter (e.g., 0.90) controls the width of the prediction set.
- **IsotonicRecalibration** and **PlattScaling** are post-hoc recalibration methods that transform raw oracle outputs to better-calibrated probabilities.
- **TemperatureScaling** applies a single learned parameter to soften or sharpen the prediction distribution, following Guo et al. (2017).

---

### Conformal Prediction for Oracle Uncertainty

```rust
/// Conformal prediction wrapper for any Oracle implementation.
///
/// Produces prediction SETS with finite-sample validity guarantees:
/// P(y_new ∈ C(x_new)) ≥ 1 - α for any distribution, any sample size.
///
/// No distributional assumptions required — only exchangeability.
/// (Vovk et al., 2005; Angelopoulos & Bates, 2023, arXiv:2107.07511)
pub struct ConformalOracle {
    base_oracle: Arc<dyn Oracle>,
    /// Nonconformity scores from calibration set.
    calibration_scores: Vec<f64>,
    /// Target miscoverage rate α (e.g., 0.10 for 90% coverage).
    alpha: f64,
}

impl ConformalOracle {
    /// Compute prediction set for a new query.
    ///
    /// The prediction set C(x) = {y : s(x,y) ≤ q̂}
    /// where q̂ = ⌈(1-α)(n+1)⌉/n quantile of calibration scores.
    pub async fn predict_set(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<PredictionSet> {
        let base = self.base_oracle.predict(query, ctx).await?;
        let n = self.calibration_scores.len();
        let quantile_idx = ((1.0 - self.alpha) * (n + 1) as f64).ceil() as usize;
        let threshold = self.calibration_scores
            .get(quantile_idx.min(n - 1))
            .copied()
            .unwrap_or(f64::MAX);

        Ok(PredictionSet {
            point_prediction: base,
            coverage_guarantee: 1.0 - self.alpha,
            prediction_interval: PredictionInterval {
                lower: base.value.as_numeric()? - threshold,
                upper: base.value.as_numeric()? + threshold,
                coverage: 1.0 - self.alpha,
            },
            n_calibration: n,
        })
    }
}
```

The key property of conformal prediction is **distribution-free finite-sample validity**: the coverage guarantee `P(y_new in C(x_new)) >= 1 - alpha` holds for any underlying distribution, requiring only that the calibration and test data are exchangeable. This is strictly stronger than asymptotic guarantees from parametric models.

The `calibration_scores` vector stores nonconformity scores computed on a held-out calibration set. The quantile computation `ceil((1-alpha)(n+1))/n` is the split conformal prediction threshold from Vovk et al. (2005). Larger calibration sets produce tighter prediction intervals without sacrificing coverage.

In the Roko context, each oracle maintains its own calibration set of recent (prediction, outcome) pairs. The `ConformalOracle` wrapper can be applied to any `Oracle` implementation — chain, coding, research, or custom — to add rigorous uncertainty quantification at negligible computational cost.

---

### Brier Score Decomposition — Calibration Diagnostics

```rust
/// Murphy (1973) decomposition of the Brier score.
///
/// Brier = (1/N) Σ (f_i - o_i)² = REL - RES + UNC
///
/// REL (reliability): (1/N) Σ n_k (f̄_k - ō_k)²
///   Measures calibration: do predicted probabilities match observed frequencies?
///
/// RES (resolution): (1/N) Σ n_k (ō_k - ō)²
///   Measures discrimination: do different forecasts correspond to different outcomes?
///
/// UNC (uncertainty): ō(1 - ō)
///   Base rate uncertainty (irreducible).
pub fn brier_decomposition(
    predictions: &[(f64, bool)], // (predicted_probability, actual_outcome)
    n_bins: usize,               // default: 10
) -> CalibrationQuality {
    // ... implementation
}
```

The Brier score decomposition separates three orthogonal components:

- **Reliability (REL)** measures pure calibration error. An oracle that says "80% chance" should be right 80% of the time. REL = 0 means perfect calibration. This is what `ResidualCorrector` targets — subtracting systematic bias drives REL toward zero.
- **Resolution (RES)** measures discrimination ability. An oracle that always predicts the base rate has RES = 0 (no skill). High RES means the oracle's predictions actually distinguish between different outcomes. This is intrinsic to the oracle's predictive power and cannot be improved by recalibration alone.
- **Uncertainty (UNC)** is the base rate entropy, a property of the prediction task itself. It is irreducible — no oracle can reduce it.

The decomposition identity `Brier = REL - RES + UNC` means an oracle improves by either reducing REL (better calibration) or increasing RES (better discrimination). The `CalibrationTracker` in Roko tracks both components separately, enabling targeted diagnostics: if REL is high, apply `ResidualCorrector` or `PlattScaling`; if RES is low, the oracle's features or model need improvement.

The Expected Calibration Error (ECE) from Naeini et al. (2015) provides an alternative calibration metric based on binned accuracy-confidence gaps. ECE is more interpretable than REL for practitioners but provides less diagnostic information.

---

### Calibration and composition citations

- Murphy, A. H. (1973). "A New Vector Partition of the Probability Score." *J. Applied Meteorology*, 12(4), 595-600.
- Vovk, V., Gammerman, A., & Shafer, G. (2005). *Algorithmic Learning in a Random World*. Springer.
- Angelopoulos, A. N., & Bates, S. (2023). "Conformal Prediction: A Gentle Introduction." *Foundations and Trends in ML*. arXiv:2107.07511.
- Naeini, M. P., Cooper, G., & Hauskrecht, M. (2015). "Obtaining Well Calibrated Probabilities Using Bayesian Binning." *AAAI 2015*.
- Guo, C., Pleiss, G., Sun, Y., & Weinberger, K. Q. (2017). "On Calibration of Modern Neural Networks." *ICML 2017*.
- Cesa-Bianchi, N., & Lugosi, G. (2006). *Prediction, Learning, and Games*. Cambridge University Press.

### Test criteria

- **Conformal coverage**: Over 1000 test samples, empirical coverage >= (1-alpha) - 0.01.
- **Brier decomposition additivity**: REL - RES + UNC = Brier score within f64 epsilon.
- **Composition monotonicity**: Adding a well-calibrated oracle to the ensemble does not increase Brier score.
- **Temperature scaling idempotence**: Applying temperature scaling twice with T=1.0 produces identical predictions.

---

## Academic foundations

- Ousterhout, J. (2018). *A Philosophy of Software Design*. Yaknyam Press. — Deep module design principle motivating the 2-method Oracle trait.
- Vickrey, W. (1961). "Counterspeculation, Auctions, and Competitive Sealed Tenders." *Journal of Finance*, 16(1), 8-37. — VCG auction mechanism used for context allocation.
- Clarke, E. H. (1971). "Multipart Pricing of Public Goods." *Public Choice*, 11(1), 17-33. — VCG mechanism design.
- Groves, T. (1973). "Incentives in Teams." *Econometrica*, 41(4), 617-631. — VCG incentive compatibility.
- Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127-138. — EFE decomposition for context bidding.
- Conant, R. C., & Ashby, W. R. (1970). "Every good regulator of a system must be a model of that system." *International Journal of Systems Science*, 1(2), 89-97. — Good Regulator Theorem.
- Sumers, T. R., et al. (2023). "Cognitive Architectures for Language Agents." arXiv:2309.02427. — CoALA cognitive architecture.
- Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176. — Cascade routing for cost reduction.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6). — HDC for cross-domain pattern matching.

---

## Cross-References

- See [00-vision-ta-generalized.md](./00-vision-ta-generalized.md) for why TA is generalized across domains
- See [02-chain-oracles.md](./02-chain-oracles.md) for `ChainOracle` implementation
- See [03-coding-oracles.md](./03-coding-oracles.md) for `CodingOracle` implementation
- See [04-research-oracles.md](./04-research-oracles.md) for `ResearchOracle` implementation
- See [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for the full prediction loop with active inference
- See topic [05-learning](../05-learning/INDEX.md) for CascadeRouter bandit integration


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/02-chain-oracles.md

# Chain Oracles — On-Chain Technical Analysis Primitives

> The chain domain is where TA originated. Chain oracles implement the universal Oracle trait with blockchain-specific state variables, verification mechanisms, and adversarial threat models.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [01-oracle-trait](./01-oracle-trait.md) for the Oracle trait, [00-vision-ta-generalized](./00-vision-ta-generalized.md) for generalization rationale
**Key sources**: `refactoring-prd/03-cognitive-subsystems.md` §4, `bardo-backup/prd/23-ta/00-witness-as-technical-analyst.md`, `bardo-backup/prd/23-ta/07-defi-native-technical-analysis.md`

---

## ChainOracle — Implementation overview

The `ChainOracle` is the first and most mature Oracle implementation. It wraps traditional financial TA primitives (moving averages, RSI, Bollinger bands) alongside DeFi-native indicators (concentrated liquidity shape analysis, funding rates, yield term structures) into the universal Oracle trait interface:

```rust
pub struct ChainOracle {
    /// Connection to chain data (via roko-chain ChainClient).
    client: Arc<dyn ChainClient>,

    /// Historical price/volume/liquidity data cache.
    market_data: Arc<MarketDataCache>,

    /// DeFi-native indicator engine.
    defi_indicators: Arc<DeFiIndicatorEngine>,

    /// Prediction persistence and tracking.
    prediction_store: Arc<PredictionStore>,

    /// Bias correction from collective calibration.
    corrector: Arc<ResidualCorrector>,

    /// Per-(model, category) accuracy tracking.
    calibration: Arc<CalibrationTracker>,
}

#[async_trait]
impl Oracle for ChainOracle {
    async fn predict(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<Prediction> {
        let chain_payload = query.payload.as_chain()?;

        match chain_payload.metric {
            ChainMetric::Price => self.predict_price(chain_payload, ctx).await,
            ChainMetric::Gas => self.predict_gas(chain_payload, ctx).await,
            ChainMetric::Volatility => self.predict_volatility(chain_payload, ctx).await,
            ChainMetric::LiquidityDepth => self.predict_liquidity(chain_payload, ctx).await,
            ChainMetric::MevOpportunity => self.predict_mev(chain_payload, ctx).await,
            ChainMetric::ProtocolHealth => self.predict_protocol_health(chain_payload, ctx).await,
            ChainMetric::FundingRate => self.predict_funding(chain_payload, ctx).await,
            ChainMetric::YieldSpread => self.predict_yield(chain_payload, ctx).await,
        }
    }

    async fn evaluate(
        &self,
        prediction: &Prediction,
        outcome: &Engram,
    ) -> Result<PredictionAccuracy> {
        // Chain outcomes are deterministic — the blockchain state IS the ground truth.
        // Extract actual value from the outcome Engram (block data, DEX state, etc.)
        let actual = self.extract_chain_outcome(outcome)?;
        let accuracy = self.compute_accuracy(prediction, &actual);

        // Feed back to ResidualCorrector
        self.corrector.update(
            &prediction.provenance.model_id,
            &accuracy.category,
            accuracy.residual,
        );

        Ok(accuracy)
    }
}
```

### Verification mechanism: blockchain finality

Chain oracles have the strongest verification mechanism of any domain — the blockchain itself provides deterministic, tamper-proof ground truth. When a prediction about ETH price is made, the actual price at the predicted block height is an indisputable fact. This makes chain oracles ideal for calibrating the entire prediction system, since the feedback signal has zero noise.

---

## Traditional TA primitives

These are the financial TA indicators adapted for on-chain data. Each operates as a sub-predictor within the `ChainOracle`:

### Price prediction

```rust
/// Moving average family: SMA, EMA, WMA, DEMA, TEMA.
pub struct MovingAveragePredictor {
    /// Window sizes (e.g., [7, 25, 99] for short/medium/long term).
    windows: Vec<usize>,
    /// Type of moving average.
    ma_type: MovingAverageType,
}

/// Bollinger Bands: mean ± k*σ for dynamic support/resistance.
/// Standard: 20-period SMA with k=2 (captures 95% of price action).
pub struct BollingerBandPredictor {
    period: usize,
    num_std_dev: f64,
}

/// Relative Strength Index (Wilder, 1978).
/// RSI > 70 → overbought → predict mean reversion.
/// RSI < 30 → oversold → predict recovery.
/// In crypto, thresholds are often shifted (80/20) due to stronger trends.
pub struct RsiPredictor {
    period: usize,
    overbought_threshold: f64,
    oversold_threshold: f64,
}

/// MACD (Moving Average Convergence/Divergence).
/// Signal line crossovers predict trend changes.
/// Histogram divergence from price predicts momentum shifts.
pub struct MacdPredictor {
    fast_period: usize,   // typically 12
    slow_period: usize,   // typically 26
    signal_period: usize, // typically 9
}
```

### Volatility estimation

```rust
/// Realized volatility from historical price data.
/// Uses Garman-Klass estimator (more efficient than close-to-close)
/// when OHLC data is available.
pub struct VolatilityPredictor {
    /// Lookback window for volatility estimation.
    window: usize,
    /// Estimator type.
    estimator: VolatilityEstimator,
}

pub enum VolatilityEstimator {
    /// Close-to-close: σ = std(ln(P_t/P_{t-1}))
    CloseToClose,
    /// Garman-Klass (1980): uses OHLC for 5-8x efficiency gain.
    GarmanKlass,
    /// Parkinson (1980): uses high-low range.
    Parkinson,
    /// Yang-Zhang (2000): combines overnight and trading hour volatility.
    YangZhang,
}
```

### Gas price forecasting

```rust
/// Gas prediction using block-level fee data (EIP-1559 base fee dynamics).
///
/// The base fee follows a deterministic formula:
///   base_fee_next = base_fee * (1 + 0.125 * (gas_used - gas_target) / gas_target)
///
/// But the PRIORITY fee is market-driven and requires prediction.
/// We use exponential smoothing + day-of-week/hour-of-day seasonality.
pub struct GasPredictor {
    /// Base fee model (deterministic from block data).
    base_fee_model: BaseFeeModel,
    /// Priority fee model (statistical, requires prediction).
    priority_fee_model: PriorityFeeModel,
    /// Seasonal adjustment factors.
    seasonality: SeasonalityModel,
}
```

Gas prediction is a T0 probe — it runs at Gamma frequency with no LLM cost. The base fee is deterministic from EIP-1559 mechanics; only the priority fee requires statistical prediction.

---

## DeFi-native indicators

These indicators have no traditional finance equivalent. They arise from the unique mechanics of decentralized protocols:

### Concentrated liquidity shape analysis (Uniswap v3+)

```rust
/// Analyzes the distribution of liquidity across price ticks.
/// Concentrated liquidity creates a "liquidity landscape" that reveals
/// market maker expectations about future price ranges.
pub struct ConcentratedLiquidityAnalyzer {
    /// Pool address and chain.
    pool: PoolAddress,

    /// Indicators computed from tick-level data.
    indicators: ConcentratedLiquidityIndicators,
}

pub struct ConcentratedLiquidityIndicators {
    /// Tick asymmetry: ratio of liquidity above vs. below current price.
    /// High asymmetry → market expects directional move.
    /// Computed as: sum(liquidity_above) / sum(liquidity_below).
    pub tick_asymmetry: f64,

    /// Migration velocity: rate at which LPs are repositioning their ranges.
    /// High velocity → market makers expect imminent price action.
    /// Computed as: Δ(center_of_mass) / Δt.
    pub migration_velocity: f64,

    /// Density gaps: contiguous tick ranges with zero liquidity.
    /// Gaps indicate "air pockets" where price can move rapidly.
    /// Each gap is a (lower_tick, upper_tick) range.
    pub density_gaps: Vec<(i32, i32)>,

    /// Herfindahl-Hirschman Index of liquidity concentration.
    /// Low HHI → diffuse liquidity (high resilience).
    /// High HHI → concentrated liquidity (fragile, LP-dependent).
    pub hhi: f64,

    /// JIT (Just-In-Time) liquidity fraction: percentage of liquidity
    /// added and removed within the same block.
    /// High JIT → sophisticated MEV activity, higher execution risk.
    pub jit_fraction: f64,
}
```

These indicators are unique to DeFi and have no TradFi equivalent. They provide structural information about execution costs that traditional order book analysis cannot capture.

### Lending market indicators

```rust
/// Indicators derived from lending protocol state (Aave, Compound, etc.).
pub struct LendingIndicators {
    /// Utilization rate: borrowed / total_supplied.
    /// Above optimal (typically 80%) → interest rates spike nonlinearly.
    pub utilization_rate: f64,

    /// Liquidation proximity: distribution of borrower health factors.
    /// Concentration near 1.0 → cascade risk.
    pub liquidation_proximity: LiquidationDistribution,

    /// Supply/borrow rate spread: lender yield vs. borrower cost.
    /// Narrowing spread → protocol stress.
    pub rate_spread: f64,

    /// Flash loan volume trend: elevated flash loans often precede
    /// governance attacks or liquidation cascades.
    pub flash_loan_trend: f64,
}
```

### Perpetual funding rates

```rust
/// Funding rate indicators for perpetual futures (dYdX, GMX, etc.).
pub struct FundingRateIndicators {
    /// Current funding rate (annualized).
    /// Positive → longs pay shorts → market is long-biased.
    /// Negative → shorts pay longs → market is short-biased.
    pub current_rate: f64,

    /// Funding rate vs. 30-day moving average.
    /// Extreme deviation → mean reversion likely.
    pub deviation_from_mean: f64,

    /// Open interest trend: rising OI + positive funding → leveraged long squeeze risk.
    pub open_interest_trend: f64,

    /// Basis: spot price - perpetual price.
    /// Persistent negative basis → market structure stress.
    pub basis: f64,
}
```

### Yield term structure

```rust
/// Yield curves across DeFi lending protocols.
/// An inverted yield curve (short rates > long rates) signals stress,
/// analogous to inverted yield curves in TradFi bond markets.
pub struct YieldTermStructure {
    /// Rates at standard maturities.
    pub rates: BTreeMap<Duration, f64>,

    /// Slope: long_rate - short_rate.
    /// Positive slope → normal (compensation for duration risk).
    /// Negative slope → inverted (stress signal).
    pub slope: f64,

    /// Curvature: 2 * medium_rate - short_rate - long_rate.
    /// High curvature → convexity opportunity.
    pub curvature: f64,

    /// Rate of slope change: Δslope / Δt.
    /// Rapid flattening → potential regime change.
    pub slope_velocity: f64,
}
```

### On-chain options indicators

```rust
/// Indicators from on-chain options protocols (Lyra, Hegic, etc.).
pub struct OnChainOptionsIndicators {
    /// Implied volatility surface: IV across strikes and expirations.
    pub iv_surface: VolatilitySurface,

    /// Put/call ratio: elevated put buying → hedging demand → bearish signal.
    pub put_call_ratio: f64,

    /// Skew: difference between OTM put IV and OTM call IV.
    /// High skew → market pricing in downside risk.
    pub skew: f64,

    /// Term structure of IV: short-dated vs. long-dated implied vol.
    /// Inverted term structure → imminent event expected.
    pub iv_term_structure_slope: f64,
}
```

---

## MEV opportunity detection

Maximal Extractable Value (MEV) is an adversarial dynamic unique to blockchains. The chain oracle detects MEV exposure as a risk factor:

```rust
/// MEV analysis for execution risk assessment.
pub struct MevAnalyzer {
    /// Sandwich attack risk: probability of being sandwiched on a given pool.
    /// Estimated from historical mempool + block builder data.
    pub sandwich_risk: f64,

    /// Backrun opportunity: value available from transaction ordering.
    pub backrun_value: f64,

    /// Block builder concentration: if one builder dominates,
    /// MEV extraction is more predictable.
    pub builder_hhi: f64,

    /// Private transaction fraction: percentage of transactions
    /// submitted through private mempools (Flashbots, etc.).
    /// High fraction → less public mempool data for MEV prediction.
    pub private_tx_fraction: f64,
}
```

MEV detection is the chain oracle's adversarial threat model — analogous to the coding oracle's supply chain attack detection or the research oracle's p-hacking detection. The Oracle trait's generalization maps these domain-specific adversarial dynamics to a common pattern: "detect when the environment is actively working against you."

---

## The 8 T0 chain probes

At Gamma frequency (~5-15s), 8 chain-specific probes run with zero LLM cost (FrugalGPT-inspired; Chen et al., 2023, arXiv:2305.05176):

```rust
/// The 8 chain-domain T0 probes.
/// Each is a pure function: fn(state) -> f32.
/// Combined via weighted sum into a prediction error scalar.
pub fn chain_probes() -> Vec<Box<dyn Probe>> {
    vec![
        // 1. Price delta — has the price moved more than expected?
        Box::new(PriceDeltaProbe::new(threshold: 0.02)),

        // 2. TVL delta — has total value locked shifted significantly?
        Box::new(TvlDeltaProbe::new(threshold: 0.05)),

        // 3. Position health — is any position approaching liquidation?
        Box::new(PositionHealthProbe::new(min_health_factor: 1.2)),

        // 4. Gas spike — has base fee jumped more than 2x?
        Box::new(GasSpikeProbe::new(spike_multiplier: 2.0)),

        // 5. Credit balance — is KORAI balance below operating threshold?
        Box::new(CreditBalanceProbe::new(min_balance: 100.0)),

        // 6. RSI — is RSI in extreme territory (>80 or <20)?
        Box::new(RsiProbe::new(period: 14, overbought: 80.0, oversold: 20.0)),

        // 7. MACD — has MACD crossed the signal line?
        Box::new(MacdCrossProbe::new(fast: 12, slow: 26, signal: 9)),

        // 8. Circuit breaker — has any monitored exchange halted trading?
        Box::new(CircuitBreakerProbe::new()),
    ]
}
```

These probes cost microseconds each. When the weighted sum of all 16 probes (8 chain + 6 coding + 2 universal) produces an error scalar below 0.2, the agent suppresses cognitive activity — no LLM call, no cost. This happens ~80% of ticks, making the chain agent dramatically cheaper to run than naive polling-based agents.

---

## ChainOracle integration with the witness crate

The chain oracle integrates with `roko-chain`'s witness infrastructure (formerly the "Witness crate" in legacy documents):

```rust
/// The witness pipeline feeds data to the chain oracle.
///
/// Data flow:
///   ChainClient → raw block/tx data
///   → TriagePipeline → filtered, classified events
///   → MarketDataCache → indexed price/volume/liquidity history
///   → ChainOracle → predictions
///   → PredictionStore → tracked predictions
///   → ResidualCorrector → calibrated predictions
///
/// At each step, data is an Engram flowing through Synapse traits.
pub struct ChainWitnessPipeline {
    client: Arc<dyn ChainClient>,
    triage: TriagePipeline,
    cache: Arc<MarketDataCache>,
    oracle: Arc<ChainOracle>,
    store: Arc<PredictionStore>,
}
```

The triage pipeline uses MIDAS-R (Massively Irregular Data Aggregation using Streaming) for real-time anomaly detection and DDSketch (Masson et al., 2019) for percentile estimation on streaming data. Both are O(1) memory and sub-microsecond per update.

### CorticalState — The shared signal bus

The `CorticalState` (formerly `TaCorticalExtension` in legacy documents) is the shared state that all chain TA subsystems read and write:

```rust
/// Shared state for chain technical analysis.
///
/// All chain TA subsystems read from and write to this state.
/// Atomic operations ensure consistency at Gamma frequency.
/// This is the chain oracle's "working memory."
pub struct CorticalState {
    /// 8 atomic signal values, updated by probes.
    pub signals: [AtomicF64; 8],

    /// Current prediction error scalar (drives T0/T1/T2 routing).
    pub prediction_error: AtomicF64,

    /// Current behavioral state from Daimon.
    pub behavioral_state: AtomicU8,

    /// Timestamp of last update.
    pub last_update_ms: AtomicI64,
}
```

---

## Mirage-rs integration — Simulation-backed predictions

Chain oracles can validate predictions against `mirage-rs`, Roko's in-process EVM simulator (141 tests). Before executing a trade, the oracle can simulate the transaction:

```rust
/// Simulate a trade in mirage-rs to validate oracle predictions.
///
/// This creates a fork of the current chain state, executes the
/// proposed transaction, and compares the simulated outcome against
/// the oracle's prediction. If the simulation contradicts the
/// prediction, the confidence is reduced.
pub async fn validate_with_simulation(
    oracle: &ChainOracle,
    prediction: &Prediction,
    mirage: &MirageSimulator,
) -> ValidationResult {
    let simulated = mirage.simulate_trade(&prediction.trade_params).await?;
    let predicted_value = prediction.value.as_numeric()?;
    let simulated_value = simulated.execution_price;

    let divergence = (predicted_value - simulated_value).abs() / simulated_value;

    if divergence > 0.05 {
        ValidationResult::Divergent {
            predicted: predicted_value,
            simulated: simulated_value,
            divergence,
            recommendation: "Reduce confidence or re-predict with updated state",
        }
    } else {
        ValidationResult::Consistent { divergence }
    }
}
```

Mirage-rs enables the chain oracle's dream cycle to run counterfactual simulations — "what would have happened if gas was 5x higher?" or "what if liquidity was withdrawn from this pool?" — without risking real assets. This is the concrete implementation of Pearl's do-operator (Pearl, 2009, *Causality*) in the chain domain: simulate interventions on the causal model.

---

## On-chain prediction infrastructure

Predictions are published on-chain (Korai) for collective calibration:

```rust
/// On-chain prediction registry on Korai.
///
/// Each prediction is a `PredictionClaim` Engram posted to the
/// Intersubjective Fact Registry (ISFR). When resolved, the
/// resolution is also posted, and the agent's calibration score
/// is updated on-chain.
///
/// This enables:
/// 1. Collective calibration — all agents share prediction outcomes
/// 2. Reputation building — accurate predictors earn higher reputation
/// 3. Knowledge futures — predictions can be staked with KORAI tokens
pub struct OnChainPredictionStore {
    /// ISFR contract address on Korai.
    registry: Address,

    /// Agent's wallet for posting predictions.
    wallet: Arc<dyn ChainWallet>,

    /// Local cache of on-chain predictions.
    cache: Arc<PredictionStore>,
}
```

The on-chain prediction infrastructure connects to the Knowledge Futures Market (see `refactoring-prd/09-innovations.md` §XVI), where agents can stake KORAI tokens on their predictions. Accurate predictors earn rewards; inaccurate ones lose stake. This creates an economic incentive for oracle quality that compounds with the technical calibration loop.

---

## Academic foundations

- Wilder, J. W. (1978). *New Concepts in Technical Trading Systems*. — RSI, ADX, parabolic SAR.
- Bollinger, J. (2001). *Bollinger on Bollinger Bands*. — Dynamic support/resistance via standard deviation bands.
- Garman, M. B., & Klass, M. J. (1980). "On the Estimation of Security Price Volatilities from Historical Data." *Journal of Business*, 53(1), 67-78. — Efficient volatility estimation from OHLC data.
- Pearl, J. (2009). *Causality: Models, Reasoning, and Inference*. 2nd ed. Cambridge University Press. — Structural causal models, do-operator for counterfactual simulation.
- Masson, C., et al. (2019). "DDSketch: A Fast and Fully-Mergeable Quantile Sketch with Relative-Error Guarantees." *PVLDB*, 12(12), 2195-2205. — Streaming percentile estimation.
- Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176. — Cascade architectures for T0/T1/T2 probe system.
- Vickrey, W. (1961). "Counterspeculation, Auctions, and Competitive Sealed Tenders." *Journal of Finance*, 16(1), 8-37. — VCG auction for context allocation.

---

## Cross-References

- See [01-oracle-trait.md](./01-oracle-trait.md) for the Oracle trait signature
- See [03-coding-oracles.md](./03-coding-oracles.md) for coding domain equivalents of these indicators
- See [07-spectral-liquidity-manifolds.md](./07-spectral-liquidity-manifolds.md) for Riemannian geometry over liquidity landscapes
- See [11-adversarial-signal-robustness.md](./11-adversarial-signal-robustness.md) for MEV defense via adversarial robustness
- See [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for the full prediction loop


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/03-coding-oracles.md

# Coding Oracles — TA Equivalents for Software Engineering

> Every financial TA primitive has a structural equivalent in software engineering. Build time trends are price trends. Test failure probability is risk assessment. Dependency vulnerability scoring is portfolio risk. The mathematics is identical; the vocabulary changes.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [01-oracle-trait](./01-oracle-trait.md) for Oracle trait, [02-chain-oracles](./02-chain-oracles.md) for chain-domain comparison
**Key sources**: `refactoring-prd/03-cognitive-subsystems.md` §4, `refactoring-prd/09-innovations.md` §I (coding probes)

---

## CodingOracle — Implementation overview

The `CodingOracle` implements the universal Oracle trait for software engineering prediction. It wraps coding-specific indicators — build time, test failure probability, complexity drift, dependency risk, performance regression — into the same predict/evaluate interface used by chain oracles:

```rust
pub struct CodingOracle {
    /// Workspace analysis engine (uses roko-index for code intelligence).
    workspace: Arc<WorkspaceAnalyzer>,

    /// Historical build/test/complexity data cache.
    metrics_cache: Arc<CodingMetricsCache>,

    /// Dependency vulnerability scanner integration.
    vuln_scanner: Arc<VulnerabilityScanner>,

    /// Prediction persistence and tracking.
    prediction_store: Arc<PredictionStore>,

    /// Bias correction from collective calibration.
    corrector: Arc<ResidualCorrector>,

    /// Per-(model, category) accuracy tracking.
    calibration: Arc<CalibrationTracker>,
}

#[async_trait]
impl Oracle for CodingOracle {
    async fn predict(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<Prediction> {
        let coding_payload = query.payload.as_coding()?;

        match coding_payload.metric {
            CodingMetric::BuildTime => self.predict_build_time(coding_payload, ctx).await,
            CodingMetric::TestPassRate => self.predict_test_pass_rate(coding_payload, ctx).await,
            CodingMetric::ComplexityDelta => self.predict_complexity(coding_payload, ctx).await,
            CodingMetric::DependencyRisk => self.predict_dep_risk(coding_payload, ctx).await,
            CodingMetric::PerfRegression => self.predict_perf_regression(coding_payload, ctx).await,
            CodingMetric::CoverageImpact => self.predict_coverage(coding_payload, ctx).await,
        }
    }

    async fn evaluate(
        &self,
        prediction: &Prediction,
        outcome: &Engram,
    ) -> Result<PredictionAccuracy> {
        // Coding outcomes come from external verifiers:
        // compilers, test suites, benchmarks, coverage tools.
        let actual = self.extract_coding_outcome(outcome)?;
        let accuracy = self.compute_accuracy(prediction, &actual);

        self.corrector.update(
            &prediction.provenance.model_id,
            &accuracy.category,
            accuracy.residual,
        );

        Ok(accuracy)
    }
}
```

### Verification mechanisms

Coding oracles use three classes of external verifiers — all produce deterministic, reproducible outcomes:

| Verifier | What it produces | Prediction it resolves |
|---|---|---|
| **Compiler** (rustc, gcc, tsc) | Success/failure + error count + compile time | Build time, compilation success |
| **Test suite** (cargo test, pytest, jest) | Pass/fail per test, total pass rate | Test failure probability |
| **Linter/analyzer** (clippy, eslint, mypy) | Warning/error counts, complexity metrics | Complexity drift |
| **Benchmark** (criterion, hyperfine) | Throughput, latency distributions | Performance regression |
| **Coverage tool** (tarpaulin, llvm-cov) | Line/branch coverage percentage | Coverage impact |
| **Vulnerability scanner** (cargo audit, npm audit) | CVE counts, severity scores | Dependency risk |

These verifiers are the coding domain's equivalent of blockchain finality — they provide ground truth that the oracle's predictions are measured against.

---

## The structural analogy table

Each chain TA primitive has a coding equivalent. The math is the same; the domain vocabulary differs:

| Chain TA Primitive | Coding Equivalent | Shared Math |
|---|---|---|
| **Price prediction** (MA, Bollinger, regression) | **Build time prediction** (SMA of compile times, trend regression) | Time series forecasting |
| **Volatility estimation** (Garman-Klass, realized vol) | **Build time variance** (variance in compile times across runs) | Variance estimation |
| **RSI** (overbought/oversold momentum) | **Test pass rate momentum** (improving vs. degrading test suites) | Bounded oscillator |
| **MACD** (trend change detection) | **Complexity trend change** (accelerating vs. decelerating complexity growth) | Moving average crossover |
| **Gas price forecasting** (base fee + priority fee) | **CI pipeline time forecasting** (fixed overhead + variable test time) | Two-component prediction |
| **Liquidity depth analysis** | **Test coverage depth analysis** (which code paths are tested) | Distribution analysis |
| **MEV detection** (adversarial execution risk) | **Supply chain attack detection** (malicious dependencies) | Adversarial threat analysis |
| **TVL trends** (total value locked trajectory) | **Dependency count trends** (growing dependency graph) | Growth rate analysis |
| **Funding rate** (long/short sentiment) | **Error rate direction** (improving vs. degrading code health) | Directional bias indicator |
| **Liquidation proximity** | **Breakage proximity** (how close code is to failing) | Threshold distance metric |

---

## Coding-specific prediction targets

### Build time prediction

```rust
/// Predict compilation time for a given change set.
///
/// Uses historical compile time data + change scope analysis.
/// Features: number of files changed, number of crates affected,
/// dependency depth, incremental vs. full rebuild.
///
/// Analogous to price prediction: both are time series with
/// trend, seasonality, and external shocks.
pub struct BuildTimePredictor {
    /// Historical compile time observations.
    history: Vec<BuildTimeObservation>,

    /// Exponential moving average of recent compile times.
    ema: ExponentialMovingAverage,

    /// Per-crate compile time model.
    crate_models: HashMap<String, CrateCompileModel>,
}

pub struct BuildTimeObservation {
    pub timestamp_ms: i64,
    pub files_changed: usize,
    pub crates_affected: Vec<String>,
    pub incremental: bool,
    pub compile_time_ms: u64,
    pub success: bool,
}

impl BuildTimePredictor {
    /// Predict compile time given a change context.
    pub fn predict(&self, change: &ChangeContext) -> (f64, f64) {
        // Base: EMA of recent compile times
        let base = self.ema.current();

        // Adjust for change scope
        let scope_factor = self.scope_adjustment(change);

        // Adjust for affected crates (some crates are slower to compile)
        let crate_factor = self.crate_adjustment(&change.affected_crates);

        // Adjust for incremental vs. full
        let incr_factor = if change.incremental { 1.0 } else { 3.5 };

        let predicted = base * scope_factor * crate_factor * incr_factor;
        let confidence = self.confidence_from_history(change);

        (predicted, confidence)
    }
}
```

### Test failure probability

```rust
/// Predict which tests will fail given a change set.
///
/// Uses file-to-test mapping (from roko-index) + historical
/// failure rates per test. Tests that cover changed code are
/// more likely to fail. Tests with high historical flakiness
/// are discounted.
///
/// Analogous to risk assessment in finance: both estimate
/// the probability of an adverse event given current conditions.
pub struct TestFailurePredictor {
    /// File → test mapping (from symbol graph).
    file_test_map: Arc<FileTestMap>,

    /// Per-test historical failure rate.
    test_histories: HashMap<String, TestHistory>,

    /// Flakiness estimator: tests that fail randomly
    /// are weighted lower in the prediction.
    flakiness: HashMap<String, f64>,
}

pub struct TestHistory {
    /// Total runs.
    pub total_runs: u64,
    /// Failed runs.
    pub failures: u64,
    /// Recent failure rate (EMA with α=0.1).
    pub recent_rate: ExponentialMovingAverage,
    /// Last N results (ring buffer).
    pub recent_results: VecDeque<bool>,
}

impl TestFailurePredictor {
    /// Predict aggregate test pass rate for a change set.
    pub fn predict_pass_rate(&self, change: &ChangeContext) -> (f64, f64) {
        let affected_tests = self.file_test_map.tests_for_files(&change.files);

        if affected_tests.is_empty() {
            return (1.0, 0.9);  // no affected tests → high pass rate, high confidence
        }

        let mut expected_failures = 0.0;
        let total = affected_tests.len() as f64;

        for test in &affected_tests {
            if let Some(history) = self.test_histories.get(test) {
                let base_rate = history.recent_rate.current();
                let flakiness = self.flakiness.get(test).copied().unwrap_or(0.0);

                // Adjusted failure probability:
                // Higher if the test covers changed code AND has historical failures.
                // Discounted by flakiness (flaky tests are less informative).
                let adj_rate = base_rate * (1.0 - flakiness);
                expected_failures += adj_rate;
            } else {
                // Unknown test: conservative assumption (10% failure rate).
                expected_failures += 0.1;
            }
        }

        let predicted_pass_rate = 1.0 - (expected_failures / total);
        let confidence = self.confidence_from_sample_size(total as u64);

        (predicted_pass_rate, confidence)
    }
}
```

### Complexity drift detection

```rust
/// Track cyclomatic complexity trends at module/crate/workspace level.
///
/// Uses moving averages and trend regression to detect when
/// complexity is accelerating (danger) or decelerating (healthy).
///
/// Analogous to MACD in finance: both detect changes in the
/// rate of change of a metric.
pub struct ComplexityDriftDetector {
    /// Per-module complexity history.
    module_histories: HashMap<String, Vec<ComplexityObservation>>,

    /// Short-term EMA (5 commits).
    short_ema: ExponentialMovingAverage,

    /// Long-term EMA (25 commits).
    long_ema: ExponentialMovingAverage,
}

pub struct ComplexityObservation {
    pub commit_hash: String,
    pub timestamp_ms: i64,
    pub cyclomatic_complexity: f64,
    pub cognitive_complexity: f64,
    pub lines_of_code: usize,
    pub function_count: usize,
}

impl ComplexityDriftDetector {
    /// Detect complexity trend direction and acceleration.
    ///
    /// Returns (trend_direction, acceleration, confidence):
    /// - trend_direction > 0: complexity increasing
    /// - acceleration > 0: complexity growth accelerating (red flag)
    pub fn detect(&self) -> ComplexityTrend {
        let short = self.short_ema.current();
        let long = self.long_ema.current();

        // MACD equivalent: difference between short and long EMA
        let macd = short - long;

        // Signal line: EMA of MACD
        let signal = self.macd_signal_ema.current();

        ComplexityTrend {
            direction: macd.signum(),
            magnitude: macd.abs(),
            acceleration: macd - signal,  // histogram equivalent
            confidence: self.confidence_from_history(),
        }
    }
}
```

### Dependency risk scoring

```rust
/// Score the risk of dependency updates and additions.
///
/// Combines multiple signals: known CVEs, maintenance activity,
/// dependency depth, license compatibility, download trends.
///
/// Analogous to portfolio risk in finance: both aggregate
/// multiple risk factors into a single score with decomposition.
pub struct DependencyRiskScorer {
    /// Known vulnerability database.
    vuln_db: Arc<VulnerabilityDatabase>,

    /// Package registry metadata.
    registry: Arc<PackageRegistry>,

    /// Per-dependency risk history.
    histories: HashMap<String, DependencyRiskHistory>,
}

pub struct DependencyRisk {
    /// Overall risk score [0.0, 1.0].
    pub score: f64,

    /// Risk decomposition (for explainability).
    pub factors: DependencyRiskFactors,

    /// Confidence in the risk assessment.
    pub confidence: f64,
}

pub struct DependencyRiskFactors {
    /// Known CVE risk (number × severity weighting).
    pub cve_risk: f64,

    /// Maintenance risk (time since last commit, bus factor).
    pub maintenance_risk: f64,

    /// Depth risk (how deep in the dependency tree — deeper = harder to fix).
    pub depth_risk: f64,

    /// License risk (compatibility with project license).
    pub license_risk: f64,

    /// Popularity risk (very popular = well-tested; unpopular = less scrutiny).
    pub popularity_risk: f64,
}
```

### Performance regression forecasting

```rust
/// Predict performance impact of code changes.
///
/// Uses historical benchmark data + change scope analysis.
/// Analogous to volatility estimation in finance: both predict
/// the magnitude of future deviations from baseline.
pub struct PerfRegressionPredictor {
    /// Historical benchmark results per test.
    benchmarks: HashMap<String, Vec<BenchmarkResult>>,

    /// File-to-benchmark mapping.
    file_bench_map: Arc<FileBenchMap>,

    /// Baseline performance per benchmark.
    baselines: HashMap<String, BenchmarkBaseline>,
}

pub struct BenchmarkBaseline {
    /// Median throughput or latency.
    pub median: f64,
    /// Interquartile range (robust spread estimate).
    pub iqr: f64,
    /// Number of observations.
    pub n: u64,
}

impl PerfRegressionPredictor {
    /// Predict whether a change will cause a performance regression.
    /// Returns (probability_of_regression, expected_magnitude, confidence).
    pub fn predict(&self, change: &ChangeContext) -> PerfPrediction {
        let affected_benches = self.file_bench_map.benches_for_files(&change.files);

        let mut regression_prob = 0.0;
        let mut expected_magnitude = 0.0;
        let count = affected_benches.len() as f64;

        for bench in &affected_benches {
            if let Some(baseline) = self.baselines.get(bench) {
                // Historical regression rate for this benchmark
                let hist_rate = self.historical_regression_rate(bench);

                // Scale by change size (larger changes → more likely to regress)
                let adj_rate = hist_rate * self.change_size_factor(change);

                regression_prob += adj_rate;
                expected_magnitude += baseline.iqr * adj_rate;
            }
        }

        if count > 0.0 {
            regression_prob /= count;
            expected_magnitude /= count;
        }

        PerfPrediction {
            regression_probability: regression_prob,
            expected_magnitude,
            confidence: self.confidence_from_sample_size(count as u64),
            affected_benchmarks: affected_benches,
        }
    }
}
```

---

## The 6 T0 coding probes

At Gamma frequency, 6 coding-specific probes run with zero LLM cost:

```rust
/// The 6 coding-domain T0 probes.
/// These are the coding equivalents of the 8 chain probes.
pub fn coding_probes() -> Vec<Box<dyn Probe>> {
    vec![
        // 9. Build health — did the last compile succeed?
        //    Is the success rate trending down?
        //    error = 0.0 if last build passed and trend stable
        //    error = 1.0 if last build failed and trend declining
        Box::new(BuildHealthProbe::new()),

        // 10. Test regression — have any tests started failing
        //     since the last run? Delta of passing test count.
        //     error = 0.0 if no change, scales with delta
        Box::new(TestRegressionProbe::new()),

        // 11. Complexity drift — is cyclomatic complexity moving
        //     average accelerating? (MACD-equivalent probe)
        //     error = 0.0 if stable, scales with acceleration
        Box::new(ComplexityDriftProbe::new()),

        // 12. Dependency risk — have any new vulnerabilities
        //     appeared in the dependency tree?
        //     error = 0.0 if clean, scales with CVE severity
        Box::new(DependencyRiskProbe::new()),

        // 13. Coverage delta — has test coverage dropped?
        //     error = 0.0 if stable or increasing
        //     error scales with coverage decrease magnitude
        Box::new(CoverageDeltaProbe::new()),

        // 14. Error rate — is the gate failure trend over the
        //     last N tasks increasing?
        //     error = 0.0 if improving, scales with failure trend
        Box::new(ErrorRateProbe::new()),
    ]
}
```

Combined with the 8 chain probes and 2 universal probes, these form the 16 T0 probes that drive ~80% of cognitive cycles to zero LLM cost. For a pure coding agent (no chain domain), the chain probes are disabled and only the 6 coding + 2 universal probes run.

---

## Tech debt as a feedback loop

The coding oracle detects and quantifies tech debt accumulation, which creates the same kind of feedback loop that chain oracles track in DeFi:

```
Tech debt accumulates
  → Development slows (increasing build times, more test failures)
  → Engineers take more shortcuts (increasing complexity)
  → More tech debt accumulates
  → Eventually: system becomes unmaintainable (analogous to protocol insolvency)
```

The coding oracle breaks this loop by making it visible. When complexity drift acceleration exceeds a threshold, the oracle emits a Warning knowledge entry via the Neuro subsystem:

```rust
// Complexity drift exceeds threshold → emit Warning
if complexity_trend.acceleration > 0.05 {
    neuro.store(KnowledgeEntry {
        kind: KnowledgeType::Warning,
        content: format!(
            "Complexity growth accelerating in module {}: Δ²C = {:.3}. \
             Historical pattern: modules with this acceleration rate \
             reach unmaintainability within {} commits.",
            module, complexity_trend.acceleration,
            self.estimated_commits_to_crisis(complexity_trend),
        ),
        confidence: complexity_trend.confidence,
        tier: KnowledgeTier::Working,
        ..Default::default()
    }).await?;
}
```

---

## Integration with roko-index

The coding oracle relies on `roko-index` for code intelligence — symbol graphs, dependency analysis, file-to-test mappings, and HDC fingerprints of code structure:

```rust
/// roko-index provides the code intelligence layer.
/// The coding oracle queries it for:
/// - File → symbol graph (function signatures, type definitions)
/// - File → test mapping (which tests cover which files)
/// - File → dependency mapping (which crates/modules depend on which)
/// - Module → complexity metrics (cyclomatic, cognitive, LOC)
/// - Workspace → HDC fingerprint (10,240-bit structural hash)
pub struct CodeIntelligenceIntegration {
    index: Arc<RokoIndex>,
}
```

HDC fingerprints from `roko-index` enable structural similarity search across codebases. When the coding oracle detects a pattern (e.g., "high-churn modules with low coverage tend to produce production bugs"), it encodes this as an HDC vector. If the same structural pattern appears in a different crate or even a different project, the Neuro subsystem's cross-domain similarity search detects the resonance.

---

## Academic foundations

- McCabe, T. J. (1976). "A Complexity Measure." *IEEE Transactions on Software Engineering*, SE-2(4), 308-320. — Cyclomatic complexity metric.
- Lehman, M. M. (1980). "Programs, Life Cycles, and Laws of Software Evolution." *Proceedings of the IEEE*, 68(9), 1060-1076. — Software evolution laws (increasing complexity, declining quality).
- Nagappan, N., & Ball, T. (2005). "Use of Relative Code Churn Measures to Predict System Defect Density." *ICSE 2005*. — Code churn as defect predictor.
- Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176. — Cascade architecture for T0 probe system.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6). — HDC fingerprints for code structure.
- Ousterhout, J. (2018). *A Philosophy of Software Design*. — Complexity management principles.

---

## Cross-References

- See [01-oracle-trait.md](./01-oracle-trait.md) for the Oracle trait these implement
- See [02-chain-oracles.md](./02-chain-oracles.md) for the chain equivalents
- See [04-research-oracles.md](./04-research-oracles.md) for the research equivalents
- See [05-witness-as-ta-generalized.md](./05-witness-as-ta-generalized.md) for the generalized witness pipeline
- See [09-causal-microstructure-discovery.md](./09-causal-microstructure-discovery.md) for causal analysis of code change patterns


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/04-research-oracles.md

# Research Oracles — Prediction for Information Analysis

> Research oracles predict source reliability, information completeness, and contradiction risk. The same TA framework that tracks price trends tracks citation momentum. The same adversarial detection that identifies MEV identifies p-hacking.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [01-oracle-trait](./01-oracle-trait.md) for Oracle trait, [02-chain-oracles](./02-chain-oracles.md) and [03-coding-oracles](./03-coding-oracles.md) for domain comparisons
**Key sources**: `refactoring-prd/03-cognitive-subsystems.md` §4

---

## ResearchOracle — Implementation overview

The `ResearchOracle` implements the universal Oracle trait for research and information analysis tasks. It evaluates sources, detects contradictions, estimates completeness, and predicts replication probability:

```rust
pub struct ResearchOracle {
    /// Source evaluation engine.
    evaluator: Arc<SourceEvaluator>,

    /// Citation graph analyzer.
    citation_graph: Arc<CitationGraphAnalyzer>,

    /// Contradiction detection engine.
    contradiction_detector: Arc<ContradictionDetector>,

    /// Prediction persistence and tracking.
    prediction_store: Arc<PredictionStore>,

    /// Bias correction from collective calibration.
    corrector: Arc<ResidualCorrector>,

    /// Per-(model, category) accuracy tracking.
    calibration: Arc<CalibrationTracker>,
}

#[async_trait]
impl Oracle for ResearchOracle {
    async fn predict(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<Prediction> {
        let research_payload = query.payload.as_research()?;

        match research_payload.metric {
            ResearchMetric::Reliability => self.predict_reliability(research_payload, ctx).await,
            ResearchMetric::Completeness => self.predict_completeness(research_payload, ctx).await,
            ResearchMetric::ContradictionRisk => self.predict_contradiction(research_payload, ctx).await,
            ResearchMetric::ReplicationProbability => self.predict_replication(research_payload, ctx).await,
            ResearchMetric::CitationMomentum => self.predict_citation_momentum(research_payload, ctx).await,
        }
    }

    async fn evaluate(
        &self,
        prediction: &Prediction,
        outcome: &Engram,
    ) -> Result<PredictionAccuracy> {
        // Research outcomes are softer than chain/coding outcomes.
        // Verification comes from: cross-validation with other sources,
        // replication studies, meta-analyses, expert review.
        let actual = self.extract_research_outcome(outcome)?;
        let accuracy = self.compute_accuracy(prediction, &actual);

        self.corrector.update(
            &prediction.provenance.model_id,
            &accuracy.category,
            accuracy.residual,
        );

        Ok(accuracy)
    }
}
```

### Verification mechanisms

Research verification is inherently weaker than chain or coding verification. There is no compiler to produce a deterministic pass/fail. Instead, research oracles use probabilistic verification:

| Verification method | Strength | Latency | What it resolves |
|---|---|---|---|
| **Cross-source agreement** | Moderate | Immediate | If 5 independent sources agree, reliability is likely high |
| **Citation analysis** | Moderate | Immediate | High-citation papers are more likely reliable (imperfect signal) |
| **Replication study** | Strong | Months/years | Direct test of whether findings reproduce |
| **Meta-analysis** | Strong | Months/years | Statistical aggregation of multiple studies |
| **Expert review** | Moderate | Days/weeks | Human expert assessment of claims |
| **Logical consistency** | Moderate | Immediate | Internal contradictions indicate unreliability |

Because verification is softer, research oracle predictions carry wider confidence intervals than chain or coding predictions. The CalibrationTracker learns these domain-specific accuracy profiles automatically.

---

## The structural analogy table

| Chain TA | Coding TA | Research TA | Shared Math |
|---|---|---|---|
| Price prediction | Build time prediction | **Citation count prediction** | Time series forecasting |
| Volatility estimation | Build time variance | **Citation velocity variance** | Variance estimation |
| RSI (momentum) | Test pass rate momentum | **Field maturity oscillator** | Bounded oscillator |
| MACD (trend change) | Complexity trend change | **Paradigm shift detection** | Moving average crossover |
| Liquidity depth | Test coverage depth | **Information completeness depth** | Distribution analysis |
| MEV detection | Supply chain attacks | **p-hacking detection** | Adversarial threat analysis |
| TVL trends | Dependency count trends | **Publication volume trends** | Growth rate analysis |
| Funding rate | Error rate direction | **Contradiction density direction** | Directional bias |
| Liquidation proximity | Breakage proximity | **Replication crisis proximity** | Threshold distance |

---

## Research-specific prediction targets

### Source reliability estimation

```rust
/// Estimate the reliability of a source for a specific claim.
///
/// Uses multiple signals: publication venue, citation count,
/// author track record, methodology quality, internal consistency,
/// and cross-source agreement.
///
/// Analogous to credit rating in finance: both aggregate multiple
/// risk factors into a single reliability score.
pub struct SourceReliabilityEstimator {
    /// Venue quality scores (preprint, peer-reviewed, top-tier journal).
    venue_scores: HashMap<String, f64>,

    /// Author track record (historical reliability of predictions
    /// based on this author's work).
    author_scores: HashMap<String, AuthorReliability>,

    /// Cross-source agreement scores.
    agreement_cache: Arc<AgreementCache>,
}

pub struct SourceReliability {
    /// Overall reliability score [0.0, 1.0].
    pub score: f64,

    /// Decomposition for explainability.
    pub factors: ReliabilityFactors,

    /// Confidence in this reliability assessment.
    pub confidence: f64,
}

pub struct ReliabilityFactors {
    /// Venue quality (top-tier journal = high, preprint = lower).
    pub venue_quality: f64,

    /// Citation momentum (increasing citations = positive signal).
    pub citation_momentum: f64,

    /// Author track record (based on historical accuracy).
    pub author_reliability: f64,

    /// Methodology quality (sample size, statistical rigor, preregistration).
    pub methodology_quality: f64,

    /// Internal consistency (no contradictions within the source).
    pub internal_consistency: f64,

    /// Cross-source agreement (other sources confirm these claims).
    pub cross_source_agreement: f64,
}
```

### Information completeness assessment

```rust
/// Assess whether the agent has enough information about a topic
/// to make reliable decisions.
///
/// Completeness is measured against a topic model: which subtopics
/// have been covered, which are missing, and how critical each is.
///
/// Analogous to portfolio coverage analysis in finance: are all
/// risk factors accounted for?
pub struct CompletenessAssessor {
    /// Topic model: expected subtopics for a given research area.
    topic_models: HashMap<String, TopicModel>,

    /// Current coverage state.
    coverage: HashMap<String, TopicCoverage>,
}

pub struct TopicCoverage {
    /// Fraction of expected subtopics covered [0.0, 1.0].
    pub completeness: f64,

    /// List of covered subtopics with confidence per subtopic.
    pub covered: Vec<(String, f64)>,

    /// List of missing subtopics with criticality score.
    pub missing: Vec<(String, f64)>,

    /// Shannon entropy of the coverage distribution.
    /// Low entropy → concentrated coverage (some subtopics deep, others absent).
    /// High entropy → even coverage (breadth without depth).
    pub coverage_entropy: f64,
}

impl CompletenessAssessor {
    /// Predict whether additional research will meaningfully improve
    /// the agent's understanding.
    ///
    /// Uses Charnov's marginal value theorem (1976): stop foraging
    /// when the marginal information gain drops below the cost.
    pub fn should_continue_research(&self, topic: &str, cost_per_query: f64) -> bool {
        let coverage = self.coverage.get(topic);
        let marginal_gain = self.estimated_marginal_gain(coverage);
        marginal_gain > cost_per_query
    }
}
```

The stopping rule uses Charnov's marginal value theorem (Charnov, 1976, *Theoretical Population Biology*) — the same optimal foraging framework used by the predictive foraging system (see [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md)). An agent stops researching when the expected information gain per additional query drops below the cost of that query.

### Contradiction detection across sources

```rust
/// Detect contradictions between sources on the same topic.
///
/// Contradictions are the research domain's equivalent of arbitrage
/// in finance: two prices for the same asset indicate market
/// inefficiency. Two contradictory claims about the same phenomenon
/// indicate at least one source is wrong.
pub struct ContradictionDetector {
    /// Claim extraction engine.
    claim_extractor: Arc<ClaimExtractor>,

    /// Semantic similarity engine (HDC-based).
    similarity: Arc<HdcSimilarity>,

    /// Known contradictions with resolution status.
    known_contradictions: Vec<Contradiction>,
}

pub struct Contradiction {
    /// The two claims that contradict each other.
    pub claim_a: ContentHash,
    pub claim_b: ContentHash,

    /// Semantic similarity of the claims (high similarity + different conclusions = contradiction).
    pub claim_similarity: f64,

    /// How confident we are that these truly contradict.
    pub confidence: f64,

    /// Resolution status: which claim is more likely correct?
    pub resolution: Option<ContradictionResolution>,
}

pub struct ContradictionResolution {
    /// Which claim is favored after analysis.
    pub favored: ContentHash,

    /// Why (cross-source agreement, recency, methodology quality).
    pub reason: String,

    /// Confidence in the resolution.
    pub confidence: f64,
}
```

HDC encoding (Kleyko et al., 2022, *ACM Computing Surveys*) enables nanosecond contradiction detection: encode each claim as a 10,240-bit vector, compute Hamming similarity between claim pairs, and flag pairs with high semantic similarity but opposite conclusions. This runs at Gamma frequency without LLM cost.

### Replication probability estimation

```rust
/// Estimate the probability that a study's findings would replicate.
///
/// Based on the Open Science Collaboration's (2015) replication crisis
/// research: only 36% of psychology studies replicated. Signals that
/// predict replication failure include small sample size, novel claims
/// without preregistration, p-values near 0.05, and single-author studies.
///
/// Analogous to default probability estimation in credit risk.
pub struct ReplicationEstimator {
    /// Features that predict replication.
    feature_weights: ReplicationFeatures,

    /// Historical replication outcomes (training data).
    history: Vec<ReplicationOutcome>,
}

pub struct ReplicationFeatures {
    /// Sample size relative to effect size (power analysis).
    pub statistical_power: f64,

    /// Whether the study was preregistered.
    pub preregistered: bool,

    /// p-value proximity to 0.05 (p = 0.049 is suspicious).
    pub p_value_proximity: f64,

    /// Number of dependent variables tested (multiple comparisons risk).
    pub n_comparisons: usize,

    /// Effect size magnitude (implausibly large effects are suspicious).
    pub effect_size: f64,

    /// Field replication rate (psychology ≈ 36%, economics ≈ 61%).
    pub field_base_rate: f64,
}
```

### Citation momentum analysis

```rust
/// Track citation trends over time.
///
/// Analogous to price momentum in finance: papers with accelerating
/// citations are gaining influence. Papers with decelerating citations
/// may be superseded or found incorrect.
pub struct CitationMomentumAnalyzer {
    /// Citation time series per paper.
    citation_series: HashMap<String, Vec<(i64, u64)>>,

    /// Short-term citation EMA (6 months).
    short_ema: ExponentialMovingAverage,

    /// Long-term citation EMA (3 years).
    long_ema: ExponentialMovingAverage,
}

impl CitationMomentumAnalyzer {
    /// Compute citation MACD for a paper.
    ///
    /// Positive MACD → accelerating citations → growing influence.
    /// Negative MACD → decelerating citations → declining relevance.
    /// MACD crossover → paradigm shift signal.
    pub fn compute_macd(&self, paper_id: &str) -> Option<CitationMacd> {
        let series = self.citation_series.get(paper_id)?;
        let short = self.short_ema.compute(series);
        let long = self.long_ema.compute(series);

        Some(CitationMacd {
            value: short - long,
            signal: self.signal_ema.compute_from(short - long),
            histogram: (short - long) - self.signal_ema.current(),
        })
    }
}
```

---

## Adversarial dynamics: p-hacking detection

The research domain's adversarial threat model centers on publication bias, p-hacking, and selective reporting — researchers who game their methodology to produce publishable results:

```rust
/// Detect potential p-hacking in research sources.
///
/// Analogous to MEV detection in chain oracles: both identify
/// when participants are gaming the system.
///
/// Signals of p-hacking (Simmons et al., 2011):
/// - p-values clustered just below 0.05
/// - Effect sizes that don't decrease with larger samples
/// - Multiple unreported comparisons
/// - Post-hoc hypothesis refinement
pub struct PHackingDetector {
    /// p-value distribution analysis.
    p_value_analyzer: PValueAnalyzer,

    /// Effect size consistency checker.
    effect_checker: EffectSizeChecker,

    /// Known p-hacking patterns.
    patterns: Vec<PHackingPattern>,
}

pub struct PHackingAssessment {
    /// Overall p-hacking risk [0.0, 1.0].
    pub risk: f64,

    /// Specific red flags detected.
    pub red_flags: Vec<PHackingRedFlag>,

    /// Confidence in this assessment.
    pub confidence: f64,
}

pub enum PHackingRedFlag {
    /// p-value clustering below 0.05.
    PValueClustering { count: usize, expected_by_chance: f64 },

    /// Effect size inconsistent with sample size.
    EffectSizeAnomaly { reported: f64, expected_range: (f64, f64) },

    /// Multiple comparisons without correction.
    MultipleComparisons { reported_tests: usize, likely_tests: usize },

    /// Selective reporting (outcomes mentioned in methods but not results).
    SelectiveReporting { missing_outcomes: Vec<String> },
}
```

---

## Research oracle as VCG auction bidder

Research predictions participate in the VCG attention auction when the agent is composing context for a research task:

```rust
// Research oracle predictions bid for context inclusion.
// High-contradiction areas bid aggressively (high epistemic value).
// High-confidence areas bid modestly (low information gain).

let contradiction_bid = contradiction_detector.risk_score(topic)
    * urgency
    * affect_weight;

let completeness_bid = (1.0 - completeness_assessor.score(topic))
    * urgency
    * 0.8;  // completeness context is valuable but less urgent

let reliability_bid = reliability_estimator.uncertainty(source)
    * urgency
    * affect_weight;

composer.bid("contradiction_context", contradiction_bid, contradiction_engrams);
composer.bid("completeness_gaps", completeness_bid, gap_engrams);
composer.bid("reliability_warnings", reliability_bid, warning_engrams);
```

---

## Collective research calibration

On the Korai mesh, research oracles share their source reliability assessments. When 100 agents have all evaluated the same source, the collective reliability estimate converges faster than any individual agent's assessment. This is the research domain's version of collective calibration:

```
Agent A rates Source X at reliability 0.7
Agent B rates Source X at reliability 0.8  (used it successfully)
Agent C rates Source X at reliability 0.5  (found a contradiction)
...
Collective estimate: weighted average by agent reputation = 0.68
New agent importing this: starts at 0.68, not at 0.5 (prior)
```

This directly implements the 31.6× faster calibration heuristic from `refactoring-prd/09-innovations.md` §VI, adapted for the research domain where "verification" is probabilistic rather than deterministic.

---

## Academic foundations

- Open Science Collaboration. (2015). "Estimating the reproducibility of psychological science." *Science*, 349(6251), aac4716. — Replication crisis data (36% replication rate).
- Simmons, J. P., Nelson, L. D., & Simonsohn, U. (2011). "False-Positive Psychology." *Psychological Science*, 22(11), 1359-1366. — p-hacking mechanisms and detection.
- Ioannidis, J. P. A. (2005). "Why Most Published Research Findings Are False." *PLoS Medicine*, 2(8), e124. — Base rate of false positives in research.
- Charnov, E. L. (1976). "Optimal foraging: the marginal value theorem." *Theoretical Population Biology*, 9, 129-136. — Stopping rule for information foraging.
- Pirolli, P., & Card, S. (1999). "Information foraging." *Psychological Review*, 106(4), 643-675. — Optimal foraging applied to information retrieval.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6). — HDC for contradiction detection via similarity.

---

## Cross-References

- See [01-oracle-trait.md](./01-oracle-trait.md) for the Oracle trait interface
- See [02-chain-oracles.md](./02-chain-oracles.md) for chain domain comparison
- See [03-coding-oracles.md](./03-coding-oracles.md) for coding domain comparison
- See [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) for HDC-based contradiction detection details
- See [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for the Charnov stopping rule


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/05-witness-as-ta-generalized.md

# The Witness Pipeline — Generalized Data Ingestion for TA

> The witness is the perception layer of technical analysis. Originally designed for blockchain observation, it generalizes to any structured data stream. Every oracle needs a witness to feed it data.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [01-oracle-trait](./01-oracle-trait.md) for Oracle trait, [00-architecture](../00-architecture/INDEX.md) for Synapse Architecture
**Key sources**: `bardo-backup/prd/23-ta/00-witness-as-technical-analyst.md`, `refactoring-prd/03-cognitive-subsystems.md`

---

## The witness concept

In the original chain-centric architecture, the "witness" was a module in `roko-chain` that observed blockchain state and translated it into signals for the TA subsystem. In the generalized Roko architecture, the witness is a **domain-agnostic data ingestion pipeline** that feeds structured observations to any Oracle implementation.

The witness maps to Step 1 (PERCEIVE) of the universal cognitive loop:

```
1. PERCEIVE → Substrate.query() → Witness reads current state
```

Every domain has its own witness:

| Domain | Witness source | Data type | Cadence |
|---|---|---|---|
| **Chain** | RPC nodes, indexers, mempools | Block, transaction, event data | Per-block (~12s on Ethereum) |
| **Coding** | File system, CI/CD, Git, test runners | Build results, test outcomes, code metrics | Per-commit or continuous |
| **Research** | APIs, databases, citation indices | Papers, citations, claims | On-demand or periodic |
| **Operations** | Metrics systems, log aggregators | Latency, error rates, throughput | Continuous (sub-second) |

---

## Generalized witness trait

```rust
/// Universal data ingestion interface.
///
/// A Witness observes a structured domain and produces Engrams
/// that feed into Oracles, Scorers, and the rest of the Synapse pipeline.
pub trait Witness: Send + Sync {
    /// Observe the current state of the domain.
    /// Returns a batch of Engrams representing new observations.
    async fn observe(&self, since: i64) -> Result<Vec<Engram>>;

    /// Subscribe to a real-time stream of observations.
    /// Returns a receiver that emits Engrams as events occur.
    async fn subscribe(&self) -> Result<mpsc::Receiver<Engram>>;

    /// Get the witness's current health status.
    fn health(&self) -> WitnessHealth;
}

pub struct WitnessHealth {
    /// Is the data source reachable?
    pub connected: bool,
    /// How far behind is the witness? (0 = real-time)
    pub lag_ms: i64,
    /// Number of observations since last health check.
    pub observations_since_last: u64,
    /// Error count since last health check.
    pub errors_since_last: u64,
}
```

### Chain witness implementation

```rust
/// Blockchain witness: observes chain state via RPC.
pub struct ChainWitness {
    client: Arc<dyn ChainClient>,
    filters: Vec<ChainFilter>,
    last_block: AtomicU64,
}

#[async_trait]
impl Witness for ChainWitness {
    async fn observe(&self, since: i64) -> Result<Vec<Engram>> {
        let current_block = self.client.block_number().await?;
        let last = self.last_block.load(Ordering::Relaxed);

        let mut engrams = Vec::new();
        for block_num in last..=current_block {
            let block = self.client.block(block_num).await?;
            for filter in &self.filters {
                let filtered = filter.apply(&block)?;
                engrams.extend(filtered.into_iter().map(|data| {
                    Engram::builder()
                        .kind(Kind::Observation)
                        .body(Body::Json(data))
                        .tag("domain", "chain")
                        .tag("block", block_num.to_string())
                        .build()
                }));
            }
        }

        self.last_block.store(current_block, Ordering::Relaxed);
        Ok(engrams)
    }

    async fn subscribe(&self) -> Result<mpsc::Receiver<Engram>> {
        let (tx, rx) = mpsc::channel(1024);
        let client = self.client.clone();
        let filters = self.filters.clone();

        tokio::spawn(async move {
            let mut stream = client.subscribe_blocks().await.unwrap();
            while let Some(block) = stream.next().await {
                for filter in &filters {
                    if let Ok(filtered) = filter.apply(&block) {
                        for data in filtered {
                            let engram = Engram::builder()
                                .kind(Kind::Observation)
                                .body(Body::Json(data))
                                .tag("domain", "chain")
                                .build();
                            let _ = tx.send(engram).await;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    fn health(&self) -> WitnessHealth {
        WitnessHealth {
            connected: self.client.is_connected(),
            lag_ms: self.compute_lag(),
            observations_since_last: self.observation_count.swap(0, Ordering::Relaxed),
            errors_since_last: self.error_count.swap(0, Ordering::Relaxed),
        }
    }
}
```

### Coding witness implementation

```rust
/// Coding workspace witness: observes build results, test outcomes,
/// code metrics, and Git activity.
pub struct CodingWitness {
    /// File system watcher for code changes.
    fs_watcher: Arc<FsWatcher>,

    /// Git repository interface.
    git: Arc<GitRepository>,

    /// CI/CD pipeline interface.
    ci: Arc<dyn CiPipeline>,

    /// Code metrics calculator (via roko-index).
    metrics: Arc<CodeMetrics>,
}

#[async_trait]
impl Witness for CodingWitness {
    async fn observe(&self, since: i64) -> Result<Vec<Engram>> {
        let mut engrams = Vec::new();

        // Git changes since last observation
        let commits = self.git.commits_since(since).await?;
        for commit in commits {
            engrams.push(Engram::builder()
                .kind(Kind::Observation)
                .body(Body::Json(serde_json::to_value(&commit)?))
                .tag("domain", "coding")
                .tag("event", "commit")
                .tag("hash", &commit.hash)
                .build());
        }

        // Latest build result
        if let Some(build) = self.ci.latest_build().await? {
            engrams.push(Engram::builder()
                .kind(Kind::Observation)
                .body(Body::Json(serde_json::to_value(&build)?))
                .tag("domain", "coding")
                .tag("event", "build")
                .tag("status", if build.success { "pass" } else { "fail" })
                .build());
        }

        // Latest test results
        if let Some(tests) = self.ci.latest_test_results().await? {
            engrams.push(Engram::builder()
                .kind(Kind::Observation)
                .body(Body::Json(serde_json::to_value(&tests)?))
                .tag("domain", "coding")
                .tag("event", "tests")
                .tag("pass_rate", format!("{:.2}", tests.pass_rate))
                .build());
        }

        // Complexity metrics snapshot
        let complexity = self.metrics.workspace_complexity().await?;
        engrams.push(Engram::builder()
            .kind(Kind::Observation)
            .body(Body::Json(serde_json::to_value(&complexity)?))
            .tag("domain", "coding")
            .tag("event", "complexity")
            .build());

        Ok(engrams)
    }

    async fn subscribe(&self) -> Result<mpsc::Receiver<Engram>> {
        let (tx, rx) = mpsc::channel(1024);
        let fs_watcher = self.fs_watcher.clone();

        tokio::spawn(async move {
            let mut events = fs_watcher.watch().await.unwrap();
            while let Some(event) = events.next().await {
                let engram = Engram::builder()
                    .kind(Kind::Observation)
                    .body(Body::Json(serde_json::to_value(&event).unwrap()))
                    .tag("domain", "coding")
                    .tag("event", "fs_change")
                    .build();
                let _ = tx.send(engram).await;
            }
        });

        Ok(rx)
    }

    fn health(&self) -> WitnessHealth {
        WitnessHealth {
            connected: self.fs_watcher.is_watching() && self.ci.is_connected(),
            lag_ms: 0,  // file system events are real-time
            observations_since_last: self.observation_count.swap(0, Ordering::Relaxed),
            errors_since_last: self.error_count.swap(0, Ordering::Relaxed),
        }
    }
}
```

---

## Triage pipeline — Filtering and classification

Not every observation deserves attention. The triage pipeline filters and classifies incoming data before it reaches the oracle:

```rust
/// Triage pipeline: filter, classify, and prioritize observations.
///
/// Uses streaming anomaly detection (MIDAS-R) and percentile
/// estimation (DDSketch) to identify significant events without
/// storing the full data stream.
pub struct TriagePipeline {
    /// MIDAS-R anomaly detector: identifies sudden changes in
    /// streaming data using count-min sketch structures.
    /// O(1) memory, sub-microsecond per update.
    anomaly_detector: MidasR,

    /// DDSketch percentile estimator: tracks percentiles of
    /// streaming numeric data with relative-error guarantees.
    /// O(1) memory per sketch.
    percentile_tracker: DdSketch,

    /// Classification rules: map observations to categories.
    classifiers: Vec<Box<dyn ObservationClassifier>>,

    /// Priority scoring: determines which observations are
    /// worth routing to T1/T2 cognition.
    priority_scorer: PriorityScorer,
}

impl TriagePipeline {
    /// Process a batch of observations.
    /// Returns only the observations that warrant further analysis.
    pub fn triage(&mut self, observations: &[Engram]) -> Vec<TriagedObservation> {
        observations.iter().filter_map(|obs| {
            // Step 1: anomaly detection
            let anomaly_score = self.anomaly_detector.score(obs);

            // Step 2: percentile context
            let percentile = self.percentile_tracker.rank(obs.numeric_value()?);

            // Step 3: classification
            let category = self.classify(obs);

            // Step 4: priority scoring
            let priority = self.priority_scorer.score(anomaly_score, percentile, &category);

            if priority > 0.2 {  // threshold for attention
                Some(TriagedObservation {
                    observation: obs.clone(),
                    anomaly_score,
                    percentile,
                    category,
                    priority,
                })
            } else {
                None
            }
        }).collect()
    }
}
```

MIDAS-R (Bhatia et al., 2020, *AAAI*) provides streaming anomaly detection with O(1) memory — it identifies sudden changes in data streams without storing history. DDSketch (Masson et al., 2019, *PVLDB*) provides streaming percentile estimation with relative-error guarantees. Together, they allow the triage pipeline to process millions of observations per second while maintaining constant memory usage.

---

## CorticalState — The shared signal bus

The `CorticalState` is the working memory for the witness pipeline. All TA subsystems read from and write to this shared state:

```rust
/// Shared state for technical analysis across all domains.
///
/// The CorticalState is the "blackboard" that all TA components
/// read from and write to. It is domain-parameterized: chain agents
/// have chain-specific signals, coding agents have coding-specific
/// signals, but the structure is identical.
pub struct CorticalState<const N: usize> {
    /// N atomic signal values, updated by T0 probes.
    /// Chain: 8 signals (price, tvl, position, gas, credit, rsi, macd, circuit)
    /// Coding: 6 signals (build, test, complexity, deps, coverage, error_rate)
    pub signals: [AtomicF64; N],

    /// Current prediction error scalar (drives T0/T1/T2 routing).
    pub prediction_error: AtomicF64,

    /// Probe weights (used to combine signals into prediction error).
    pub weights: [AtomicF64; N],

    /// Current behavioral state from Daimon.
    pub behavioral_state: AtomicU8,

    /// Timestamp of last update.
    pub last_update_ms: AtomicI64,
}

/// Chain CorticalState with 8 signals.
pub type ChainCorticalState = CorticalState<8>;

/// Coding CorticalState with 6 signals.
pub type CodingCorticalState = CorticalState<6>;
```

The CorticalState is updated at Gamma frequency by the T0 probes. All operations are atomic — no locking, no allocation, sub-microsecond latency. This is what enables the "80% of ticks cost nothing" property: the probes update atomic values, combine them into a prediction error scalar, and the tier router reads the scalar to decide whether to invoke an LLM.

---

## Three cognitive speeds in the witness

The witness pipeline operates at all three cognitive speeds:

### Gamma (~5-15s) — Real-time observation

```
Witness.observe()
  → New observations
  → Triage pipeline (MIDAS-R + DDSketch)
  → CorticalState update (atomic signals)
  → T0 probes compute prediction error scalar
  → T0/T1/T2 routing decision
```

At Gamma, only the triage pipeline and T0 probes run. No LLM. Cost: microseconds.

### Theta (~75s) — Reflective analysis

```
Witness.observe() accumulates since last Theta tick
  → Pending predictions resolved against observations
  → Residuals computed, CalibrationTracker updated
  → Oracle re-predicts for next horizon
  → Significant observations stored to Neuro as knowledge
```

At Theta, the oracle makes explicit predictions and resolves pending ones. The LLM may be involved (T1 or T2) if the prediction error scalar warrants it.

### Delta (hours) — Consolidation

```
Dreams process accumulated observations
  → NREM replay of significant observation episodes
  → REM counterfactual: "what if this observation pattern recurred?"
  → Cross-domain pattern consolidation
  → Routing table updates based on observation patterns
```

At Delta, the Dreams subsystem consolidates observation patterns into permanent knowledge. This is where the witness's observations become long-term learning.

---

## Witness pipeline integration with VCG auction

Witness observations compete for context window space through the VCG attention auction:

```rust
/// Witness observations bid for context inclusion.
///
/// High-anomaly observations bid aggressively (high surprise value).
/// Routine observations bid modestly (low information content).
/// The VCG mechanism ensures truthful bidding.
pub fn observation_bid(obs: &TriagedObservation, ctx: &AuctionContext) -> f64 {
    let surprise_value = obs.anomaly_score;
    let relevance = ctx.task_relevance(&obs.category);
    let urgency = ctx.daimon_arousal;

    surprise_value * relevance * urgency
}
```

---

## Memory architecture — Three timescales

The witness integrates with Roko's three-timescale memory architecture, mirroring the Complementary Learning Systems (CLS) theory (McClelland, 1995):

| Timescale | Memory type | What it stores | Decay |
|---|---|---|---|
| **Gamma** (seconds) | CorticalState | Current signal values, prediction error | Overwritten each tick |
| **Theta** (minutes) | Working Engrams | Recent observations, pending predictions | Hours (Ebbinghaus) |
| **Delta** (hours) | Neuro knowledge | Validated patterns, calibration data | Days to months (tier-dependent) |

Fast episodic memory (Gamma/Theta) captures details. Slow semantic memory (Delta/Neuro) captures patterns. Dreams consolidate fast to slow, replicating the hippocampal-cortical memory consolidation that CLS theory describes.

---

## Academic foundations

- Bhatia, S., Hooi, B., Yoon, M., Shin, K., & Faloutsos, C. (2020). "MIDAS: Microcluster-Based Detector of Anomalies in Edge Streams." *AAAI 2020*. — Streaming anomaly detection.
- Masson, C., et al. (2019). "DDSketch: A Fast and Fully-Mergeable Quantile Sketch." *PVLDB*, 12(12), 2195-2205. — Streaming percentile estimation.
- McClelland, J. L., McNaughton, B. L., & O'Reilly, R. C. (1995). "Why there are complementary learning systems in the hippocampus and neocortex." *Psychological Review*, 102(3), 419-457. — CLS theory.
- Charnov, E. L. (1976). "Optimal foraging: the marginal value theorem." *Theoretical Population Biology*, 9, 129-136. — Stopping rule for observation foraging.
- Friston, K. (2010). "The free-energy principle." *Nature Reviews Neuroscience*, 11(2), 127-138. — Active inference driving observation priority.
- Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176. — T0 probe architecture.

---

## Cross-References

- See [01-oracle-trait.md](./01-oracle-trait.md) for the Oracle trait that witnesses feed
- See [02-chain-oracles.md](./02-chain-oracles.md) for chain witness integration
- See [03-coding-oracles.md](./03-coding-oracles.md) for coding witness integration
- See [08-adaptive-signal-metabolism.md](./08-adaptive-signal-metabolism.md) for how signals evolve in the witness pipeline
- See [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for the full prediction loop


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/06-hyperdimensional-ta.md

# Hyperdimensional Technical Analysis

> HDC encodes TA patterns as 10,240-bit vectors. Pattern algebra (bind, bundle, permute) enables nanosecond cross-domain similarity search, temporal composition, and shift-invariant pattern matching.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [06-neuro](../06-neuro/INDEX.md) for HDC basics, [01-oracle-trait](./01-oracle-trait.md) for Oracle integration
**Key sources**: `bardo-backup/prd/23-ta/01-hyperdimensional-technical-analysis.md`, `refactoring-prd/09-innovations.md` §XIII

---

## Why HDC for technical analysis

Traditional TA relies on numerical time series operations — moving averages, statistical tests, regression models. These are computationally efficient but fragile: they require exact feature alignment, cannot handle structural similarity across domains, and fail at compositional pattern matching.

Hyperdimensional Computing (HDC) solves these problems by encoding patterns as high-dimensional binary vectors (10,240 bits = 1,280 bytes) and performing pattern algebra via bitwise operations:

| Operation | HDC | Cost | What it does |
|---|---|---|---|
| **Bind** (XOR) | `A ⊕ B` | ~2ns | Associate two concepts: "price" bound with "rising" |
| **Bundle** (majority) | `[A, B, C]` | ~10ns | Merge patterns: composite of multiple observations |
| **Permute** (rotate) | `π(A)` | ~1ns | Encode position/sequence: "first observation" vs. "second" |
| **Similarity** (Hamming) | `d(A, B)` | ~13ns | Compare patterns: how similar are these two vectors? |

With AVX-512 SIMD on modern x86, each operation processes the full 10,240-bit vector in one pass (XOR 160 u64 words + popcount). ARM NEON is roughly 2-3x slower. Performance numbers from Kleyko et al. (2022, *ACM Computing Surveys*) and Kanerva (2009, *Cognitive Computation*).

The critical advantage: **cross-domain pattern matching at nanosecond cost**. A pattern learned in the chain domain (e.g., "volatility spike precedes mean reversion") can be detected as structurally similar to a pattern in the coding domain (e.g., "error rate spike precedes test stabilization") without explicit cross-domain translation.

---

## Pattern algebra for TA

### Role-filler composition

TA patterns are encoded as role-filler pairs — a "role" (what kind of observation) bound to a "filler" (the specific value or state):

```rust
/// Encode a TA observation as a role-filler HDC vector.
///
/// Role = what kind of observation (price, volume, rsi, build_time, test_rate)
/// Filler = the specific value quantized into an HDC codebook
///
/// The binding preserves both pieces: given the composite,
/// unbinding with the role recovers the filler (approximately).
pub fn encode_observation(role: &HdcVector, filler: &HdcVector) -> HdcVector {
    role.xor(filler)  // BIND operation
}

/// Encode a complete TA state as a bundle of role-filler pairs.
///
/// Example (chain): BUNDLE(
///     BIND(price_role, price_filler),
///     BIND(volume_role, volume_filler),
///     BIND(rsi_role, rsi_filler),
///     BIND(macd_role, macd_filler),
/// )
pub fn encode_ta_state(observations: &[(HdcVector, HdcVector)]) -> HdcVector {
    let bound: Vec<HdcVector> = observations.iter()
        .map(|(role, filler)| role.xor(filler))
        .collect();
    HdcVector::bundle(&bound)  // majority vote across all bound pairs
}
```

### Temporal composition

Time series patterns are encoded using permutation to represent sequence:

```rust
/// Encode a temporal pattern: a sequence of observations over time.
///
/// Uses permutation (bit rotation) to mark temporal position:
///   π^0(obs_0) ⊕ π^1(obs_1) ⊕ π^2(obs_2) ⊕ ...
///
/// This creates a single vector that encodes the SEQUENCE of
/// observations, not just their aggregate.
///
/// Example: "RSI rose from 30 to 50 to 70 over 3 ticks"
///   = BIND(PERM(rsi_30, 0), PERM(rsi_50, 1), PERM(rsi_70, 2))
pub fn encode_temporal_pattern(observations: &[HdcVector]) -> HdcVector {
    let permuted: Vec<HdcVector> = observations.iter()
        .enumerate()
        .map(|(i, obs)| obs.permute(i as u32))
        .collect();
    HdcVector::bundle(&permuted)
}
```

### Shift-invariant pattern matching

Temporal patterns should be recognizable regardless of when they start. Shift invariance is achieved by checking similarity at all offsets:

```rust
/// Check if a pattern exists anywhere in a longer sequence.
///
/// Slides the pattern template across the sequence and returns
/// the maximum similarity at any offset.
///
/// This is how TA patterns like "head and shoulders" are detected
/// regardless of when they occurred in the time series.
pub fn shift_invariant_match(
    pattern: &HdcVector,
    sequence: &[HdcVector],
    pattern_len: usize,
) -> (f64, usize) {
    let mut best_similarity = 0.0;
    let mut best_offset = 0;

    for offset in 0..=(sequence.len() - pattern_len) {
        let window = &sequence[offset..offset + pattern_len];
        let window_encoded = encode_temporal_pattern(window);
        let similarity = pattern.hamming_similarity(&window_encoded);

        if similarity > best_similarity {
            best_similarity = similarity;
            best_offset = offset;
        }
    }

    (best_similarity, best_offset)
}
```

---

## DeFi primitive encoding

The chain domain defines HDC codebooks for DeFi primitives. Each primitive type gets a unique role vector, and specific instances are encoded as fillers:

```rust
/// DeFi primitive HDC codebook.
///
/// Each primitive type is a randomly generated 10,240-bit vector.
/// These are fixed at initialization and shared across all agents
/// (deterministic from seed).
pub struct DeFiCodebook {
    // Transaction type roles
    pub swap: HdcVector,
    pub liquidity_provision: HdcVector,
    pub lending: HdcVector,
    pub borrowing: HdcVector,
    pub vault_deposit: HdcVector,
    pub staking: HdcVector,
    pub restaking: HdcVector,
    pub perpetual: HdcVector,
    pub options: HdcVector,
    pub yield_farming: HdcVector,
    pub streaming_payment: HdcVector,
    pub gas_token: HdcVector,
    pub intent: HdcVector,
    pub rwa: HdcVector,
    pub cross_chain: HdcVector,
    pub account_abstraction: HdcVector,
    pub prediction_market: HdcVector,

    // Parameter roles
    pub amount: HdcVector,
    pub price: HdcVector,
    pub slippage: HdcVector,
    pub gas_cost: HdcVector,
    pub protocol: HdcVector,
    pub chain: HdcVector,
    pub pool: HdcVector,
    pub token_pair: HdcVector,

    // Numeric codebooks (quantized value ranges)
    pub amount_codebook: QuantizedCodebook,
    pub price_codebook: QuantizedCodebook,
    pub percentage_codebook: QuantizedCodebook,
}
```

### Quantized numeric encoding

Continuous values are quantized into discrete HDC vectors using thermometer encoding:

```rust
/// Quantized codebook for encoding continuous values as HDC vectors.
///
/// Uses thermometer encoding: for value in range [min, max] with N levels,
/// the encoded vector is a blend of the level vectors weighted by proximity.
///
/// This preserves ordinal relationships: encode(3.0) is more similar to
/// encode(4.0) than to encode(100.0).
pub struct QuantizedCodebook {
    /// Level vectors, one per quantization level.
    levels: Vec<HdcVector>,
    /// Value range.
    min: f64,
    max: f64,
    /// Number of quantization levels.
    n_levels: usize,
}

impl QuantizedCodebook {
    /// Encode a continuous value as an HDC vector.
    pub fn encode(&self, value: f64) -> HdcVector {
        let normalized = (value - self.min) / (self.max - self.min);
        let level = (normalized * self.n_levels as f64).clamp(0.0, (self.n_levels - 1) as f64);
        let lower = level.floor() as usize;
        let upper = (lower + 1).min(self.n_levels - 1);
        let weight = level - lower as f64;

        // Interpolate between adjacent level vectors
        self.levels[lower].weighted_bundle(&self.levels[upper], 1.0 - weight, weight)
    }
}
```

### Pattern composition queries

Complex TA patterns are composed from primitive encodings:

```rust
/// Example: encode "a large ETH swap on Uniswap with high slippage"
///
/// This creates a single 10,240-bit vector that captures the
/// full semantic content of the pattern.
pub fn encode_swap_pattern(
    codebook: &DeFiCodebook,
    token_pair: &str,
    amount: f64,
    slippage: f64,
    protocol: &str,
) -> HdcVector {
    let type_binding = codebook.swap.clone();
    let pair_binding = codebook.token_pair.xor(&codebook.encode_string(token_pair));
    let amount_binding = codebook.amount.xor(&codebook.amount_codebook.encode(amount));
    let slip_binding = codebook.slippage.xor(&codebook.percentage_codebook.encode(slippage));
    let proto_binding = codebook.protocol.xor(&codebook.encode_string(protocol));

    HdcVector::bundle(&[type_binding, pair_binding, amount_binding, slip_binding, proto_binding])
}

/// Query: "find all patterns similar to large swaps with high slippage"
pub fn query_similar_patterns(
    pattern: &HdcVector,
    memory: &[HdcVector],
    threshold: f64,  // typically 0.526 per refactoring-prd/09-innovations.md §XIII
) -> Vec<(usize, f64)> {
    memory.iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let sim = pattern.hamming_similarity(m);
            if sim > threshold { Some((i, sim)) } else { None }
        })
        .collect()
}
```

---

## Cross-domain pattern matching

The deepest value of HDC for TA is cross-domain insight resonance (see `refactoring-prd/09-innovations.md` §XIII). When a coding oracle encodes "high churn in auth module" and a chain oracle encodes "high volatility in ETH/USDC," the HDC vectors are structurally similar because both encode `BIND(high_uncertainty, critical_subsystem)`:

```rust
/// Cross-domain insight resonance detection.
///
/// Continuously cross-correlate new Engrams against the HDC knowledge
/// base across ALL domains. When similarity exceeds threshold (0.526),
/// emit a cross-domain insight.
pub fn detect_cross_domain_resonance(
    new_engram: &Engram,
    all_domain_knowledge: &[Engram],
    threshold: f64,
) -> Vec<CrossDomainInsight> {
    let new_hv = new_engram.hdc_vector();
    let new_domain = new_engram.domain();

    all_domain_knowledge.iter()
        .filter(|k| k.domain() != new_domain)  // cross-domain only
        .filter_map(|k| {
            let sim = new_hv.hamming_similarity(&k.hdc_vector());
            if sim > threshold {
                Some(CrossDomainInsight {
                    source_engram: new_engram.id,
                    target_engram: k.id,
                    source_domain: new_domain.clone(),
                    target_domain: k.domain().clone(),
                    similarity: sim,
                    description: format!(
                        "Pattern in {} domain has structural similarity ({:.3}) to pattern in {} domain",
                        new_domain, sim, k.domain()
                    ),
                })
            } else {
                None
            }
        })
        .collect()
}
```

The threshold of 0.526 comes from information-theoretic analysis: with 10,240-bit vectors, random vectors have expected Hamming similarity of 0.500 with standard deviation of ~0.005. A threshold of 0.526 (5σ above chance) ensures that detected similarities are statistically significant.

---

## Coding domain HDC codebook

The coding domain has its own HDC codebook, structurally parallel to the DeFi codebook:

```rust
pub struct CodingCodebook {
    // Event type roles
    pub commit: HdcVector,
    pub build: HdcVector,
    pub test_run: HdcVector,
    pub lint: HdcVector,
    pub benchmark: HdcVector,
    pub deploy: HdcVector,
    pub review: HdcVector,
    pub merge: HdcVector,

    // Metric roles
    pub complexity: HdcVector,
    pub coverage: HdcVector,
    pub pass_rate: HdcVector,
    pub build_time: HdcVector,
    pub error_count: HdcVector,
    pub churn_rate: HdcVector,

    // Scope roles
    pub file: HdcVector,
    pub module: HdcVector,
    pub crate_scope: HdcVector,
    pub workspace: HdcVector,

    // Numeric codebooks
    pub count_codebook: QuantizedCodebook,
    pub rate_codebook: QuantizedCodebook,
    pub duration_codebook: QuantizedCodebook,
}
```

Because both codebooks use the same HDC algebra (10,240-bit BSC, XOR bind, majority bundle), patterns from either domain can be compared directly via Hamming similarity. This is the mechanism that enables cross-domain insight transfer at nanosecond cost.

---

## HDC pattern memory — The pattern store

```rust
/// HDC pattern memory for TA.
///
/// Stores encoded TA patterns as a searchable vector space.
/// Queries return the most similar patterns, enabling:
/// - "Have I seen this pattern before?" (recall)
/// - "What patterns are similar to this?" (analogy)
/// - "What patterns from other domains match?" (transfer)
pub struct PatternStore {
    /// All stored patterns indexed by domain.
    patterns: HashMap<OracleDomain, Vec<StoredPattern>>,

    /// Cross-domain index for resonance detection.
    cross_domain_index: Vec<(OracleDomain, HdcVector, ContentHash)>,
}

pub struct StoredPattern {
    /// The encoded HDC vector.
    pub vector: HdcVector,

    /// The Engram this pattern was derived from.
    pub source_engram: ContentHash,

    /// The outcome when this pattern last occurred.
    pub outcome: Option<PredictionOutcome>,

    /// How often this pattern has been observed.
    pub frequency: u64,

    /// Reliability: how often did this pattern's predicted outcome match actual?
    pub reliability: f64,
}

impl PatternStore {
    /// Find the K most similar patterns to a query.
    ///
    /// Cost: O(N) with N = total patterns, ~13ns per comparison.
    /// For 100K patterns: ~1.3ms. For 1M: ~13ms.
    pub fn find_similar(
        &self,
        query: &HdcVector,
        domain: Option<&OracleDomain>,
        k: usize,
        threshold: f64,
    ) -> Vec<(f64, &StoredPattern)> {
        let candidates = match domain {
            Some(d) => self.patterns.get(d).map(|v| v.as_slice()).unwrap_or(&[]),
            None => &self.cross_domain_index.iter()
                .map(|(_, v, _)| v)
                .collect::<Vec<_>>(),  // simplified
        };

        let mut results: Vec<_> = candidates.iter()
            .map(|p| (query.hamming_similarity(&p.vector), p))
            .filter(|(sim, _)| *sim > threshold)
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        results.truncate(k);
        results
    }
}
```

---

## Integration with Dreams — Pattern consolidation

During Delta-frequency consolidation (Dreams), the HDC pattern store undergoes three operations:

1. **NREM replay**: High-value patterns are replayed and their reliability scores updated based on accumulated outcomes.
2. **REM recombination**: Novel pattern compositions are generated by bundling existing patterns with random perturbation (mutation via XOR with noise vector).
3. **Pruning**: Patterns with reliability below 0.3 after 10+ observations are removed.

```rust
/// Dream consolidation for HDC pattern store.
pub fn dream_consolidation(store: &mut PatternStore) {
    // NREM: replay high-value patterns
    for pattern in store.high_value_patterns() {
        let updated_reliability = pattern.recompute_reliability();
        pattern.reliability = updated_reliability;
    }

    // REM: generate novel compositions
    let existing: Vec<&HdcVector> = store.all_vectors().collect();
    for _ in 0..10 {
        let a = existing.choose(&mut rng).unwrap();
        let b = existing.choose(&mut rng).unwrap();
        let novel = a.xor(b).permute(1);  // recombine + shift
        store.add_hypothetical(novel, confidence: 0.2);
    }

    // Prune: remove unreliable patterns
    store.prune(|p| p.frequency >= 10 && p.reliability < 0.3);
}
```

---

## Implementation details

### Codebook generation algorithm

Codebooks are generated deterministically from a domain-specific seed. This ensures that all agents sharing a seed share the same vector space, enabling direct cross-agent pattern comparison without alignment.

```rust
/// Generate a domain-specific HDC codebook deterministically.
///
/// The seed derives from the domain name via SHA-256. Each role vector
/// is drawn from the resulting CSPRNG stream. Because the seed is
/// deterministic, every agent in the same domain produces identical
/// codebooks without coordination.
pub struct CodebookGenerator {
    /// Domain seed (SHA-256 of domain name).
    seed: [u8; 32],
    /// Dimensionality of generated vectors (default: 10_240).
    dim: usize,
}

impl CodebookGenerator {
    pub fn new(domain: &str, dim: usize) -> Self {
        let seed = sha256(domain.as_bytes());
        Self { seed, dim }
    }

    /// Generate a role vector at a given index.
    ///
    /// Uses ChaCha20 seeded from `self.seed ++ index.to_le_bytes()`.
    /// Each bit is drawn with P(1) = 0.5 (dense binary).
    pub fn generate_role(&self, index: u32) -> HdcVector {
        let mut key = self.seed.to_vec();
        key.extend_from_slice(&index.to_le_bytes());
        let mut rng = ChaCha20Rng::from_seed(sha256(&key));
        HdcVector::random(&mut rng, self.dim)
    }

    /// Generate a QuantizedCodebook for a value range.
    ///
    /// Level vectors use thermometer construction:
    ///   level_0 = random base vector
    ///   level_k = level_{k-1} with `flip_count` random bits flipped
    ///
    /// `flip_count = dim / (2 * n_levels)` ensures adjacent levels
    /// have Hamming similarity ~= 1 - 1/(2*n_levels).
    pub fn generate_quantized(
        &self,
        codebook_index: u32,
        n_levels: usize,
        min: f64,
        max: f64,
    ) -> QuantizedCodebook {
        let flip_count = self.dim / (2 * n_levels);
        let base = self.generate_role(codebook_index);
        let mut levels = vec![base];

        for k in 1..n_levels {
            let prev = &levels[k - 1];
            let mut rng = ChaCha20Rng::from_seed(
                sha256(&[&self.seed[..], &(codebook_index + k as u32).to_le_bytes()].concat())
            );
            let flipped = prev.flip_random_bits(flip_count, &mut rng);
            levels.push(flipped);
        }

        QuantizedCodebook { levels, min, max, n_levels }
    }
}
```

**Configuration parameters**:

| Parameter | Default | Range | Notes |
|---|---|---|---|
| `dim` | 10,240 | 1,024 - 65,536 | Must be multiple of 64 for SIMD alignment. 10,240 = 160 u64 words. |
| `n_levels` (QuantizedCodebook) | 64 | 8 - 256 | More levels = finer granularity, more memory. 64 gives ~1.5% resolution. |
| `flip_count` | `dim / (2 * n_levels)` | derived | Controls similarity between adjacent levels. |

### QuantizedCodebook::encode() interpolation

The `encode()` method interpolates between adjacent level vectors via a weighted bundle. The procedure:

1. Normalize the input value to `[0.0, 1.0]` within the codebook's range.
2. Map to a fractional level index: `level_f = normalized * (n_levels - 1)`.
3. Identify the two bracketing levels: `lower = floor(level_f)`, `upper = lower + 1`.
4. Compute interpolation weight: `w = level_f - lower`.
5. Return `weighted_bundle(levels[lower], levels[upper], 1.0 - w, w)`.

The weighted bundle for two vectors uses probabilistic bit selection: for each bit position, select from `levels[upper]` with probability `w`, else from `levels[lower]`. This produces a vector whose Hamming similarity to each level is proportional to the interpolation weight.

**Error handling**: Values outside `[min, max]` are clamped. If `n_levels` is 1, return the single level vector regardless of input. If the codebook is empty (zero levels), return a zero vector and log a warning.

### Pattern store serialization (CBOR)

The PatternStore serializes to CBOR (RFC 8949) for compact, schema-flexible persistence:

```rust
/// CBOR schema for PatternStore persistence.
///
/// Top-level: CBOR map {
///   "version": u32,              // schema version (currently 1)
///   "domains": map {             // keyed by OracleDomain string
///     "<domain>": array [        // array of StoredPattern
///       {
///         "v": bytes(1280),      // HDC vector (10,240 bits = 1,280 bytes)
///         "src": bytes(32),      // source engram ContentHash
///         "out": ?i8,            // outcome: -1 (loss), 0 (neutral), 1 (profit), null
///         "freq": u64,           // observation count
///         "rel": f32,            // reliability [0.0, 1.0]
///       },
///       ...
///     ]
///   },
///   "cross_index": array [       // cross-domain index entries
///     { "d": string, "v": bytes(1280), "h": bytes(32) },
///     ...
///   ]
/// }
///
/// File size estimate: 1,280 bytes per pattern + 45 bytes metadata.
/// 100K patterns ~= 130 MB. 1M patterns ~= 1.3 GB.
pub fn serialize_pattern_store(store: &PatternStore) -> Vec<u8> {
    let mut encoder = CborEncoder::new();
    encoder.map(3);
    encoder.text("version").unsigned(1);
    encoder.text("domains");
    encoder.map(store.patterns.len());
    for (domain, patterns) in &store.patterns {
        encoder.text(&domain.to_string());
        encoder.array(patterns.len());
        for p in patterns {
            encoder.map(5);
            encoder.text("v").bytes(&p.vector.as_bytes());
            encoder.text("src").bytes(&p.source_engram.as_bytes());
            encoder.text("out").optional_i8(p.outcome.map(|o| o as i8));
            encoder.text("freq").unsigned(p.frequency);
            encoder.text("rel").float32(p.reliability as f32);
        }
    }
    // ... cross_index similarly
    encoder.finish()
}
```

### Similarity threshold calibration

The default threshold of 0.526 derives from the information-theoretic properties of 10,240-bit BSC vectors. Calibrate it in practice with this procedure:

1. **Generate null distribution**: Create 10,000 random vector pairs. Compute their Hamming similarities. The distribution should be approximately Gaussian with mean 0.500 and stddev ~0.00494.
2. **Choose significance level**: The default 0.526 corresponds to 5.26 sigma (p < 1e-7). For applications tolerating more false positives, use 0.515 (3 sigma, p < 0.0013).
3. **Validate on held-out data**: Take known-similar pattern pairs from the domain. Compute their similarity distribution. The threshold should separate the null distribution from the true-positive distribution with <1% overlap.
4. **Adjust per domain**: If a domain has noisier encodings (fewer role-filler pairs per pattern), increase the threshold. Rule of thumb: add 0.005 per missing role-filler pair below 5.

```rust
/// Calibrate similarity threshold for a given vector dimensionality.
///
/// Returns (mean, stddev, suggested_threshold) based on the null distribution.
pub fn calibrate_threshold(dim: usize, sigma_level: f64, n_samples: usize) -> (f64, f64, f64) {
    let mut rng = thread_rng();
    let mut similarities = Vec::with_capacity(n_samples);
    for _ in 0..n_samples {
        let a = HdcVector::random(&mut rng, dim);
        let b = HdcVector::random(&mut rng, dim);
        similarities.push(a.hamming_similarity(&b));
    }
    let mean = similarities.iter().sum::<f64>() / n_samples as f64;
    let variance = similarities.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n_samples as f64;
    let stddev = variance.sqrt();
    (mean, stddev, mean + sigma_level * stddev)
}
```

### Cross-domain routing protocol

When oracles from different domains want to exchange patterns, the routing protocol works as follows:

```
1. Source oracle encodes a pattern using its domain codebook.
2. Source sends (domain_id, pattern_hv, metadata) to the PatternStore.
3. PatternStore inserts the pattern into the cross_domain_index.
4. Any oracle can query the cross_domain_index with a pattern vector.
5. Matches above threshold are returned with their source domain.
6. The querying oracle decides whether to incorporate the cross-domain match.
```

No codebook translation is needed because all codebooks share the same vector space (10,240-bit BSC with XOR bind and majority bundle). The cross-domain similarity is structural, not lexical.

**Integration wiring**: `PatternStore::find_similar()` with `domain: None` searches the cross-domain index. The oracle calls this during its Theta-frequency analysis pass.

### Pruning rules

When the pattern count exceeds memory limits, prune according to these rules (applied in order):

1. **Unreliable patterns**: Remove patterns where `reliability < 0.3` and `frequency >= 10`. These have enough observations to confirm they do not predict well.
2. **Stale patterns**: Remove patterns not matched in the last `max_staleness` duration (default: 72 hours). These are no longer relevant to current market/code conditions.
3. **Redundant patterns**: For patterns with Hamming similarity > 0.95 to each other, keep only the one with higher reliability. This deduplicates near-identical encodings.
4. **LRU eviction**: If the store still exceeds `max_patterns`, remove the least-recently-matched patterns until within budget.

```rust
/// Configuration for pattern store pruning.
pub struct PruneConfig {
    /// Maximum patterns per domain before pruning triggers.
    pub max_patterns_per_domain: usize,  // default: 100_000
    /// Minimum reliability to survive pruning (with sufficient observations).
    pub min_reliability: f64,            // default: 0.3
    /// Minimum observations before reliability-based pruning applies.
    pub min_frequency: u64,              // default: 10
    /// Maximum time since last match before staleness pruning.
    pub max_staleness: Duration,         // default: 72 hours
    /// Similarity threshold for deduplication.
    pub dedup_threshold: f64,            // default: 0.95
}
```

### Connection to Dreams: reliability updates

During Delta-frequency dream consolidation, the `StoredPattern.reliability` field is updated from dream outcomes:

```
1. Dreams NREM phase replays high-frequency patterns.
2. For each replayed pattern, Dreams evaluates: "If this pattern
   activated now, would the predicted outcome hold?"
3. The evaluation runs the pattern through the current causal model
   (from causal microstructure discovery) and compares the predicted
   outcome to the pattern's stored outcome.
4. If they agree: reliability += 0.05 (capped at 1.0).
5. If they disagree: reliability -= 0.10 (floored at 0.0).
6. During REM recombination, newly generated hypothetical patterns
   start with reliability = 0.2.
```

The asymmetric update (slower increase, faster decrease) follows the principle that trust is hard to earn and easy to lose. A pattern must consistently agree with the causal model across multiple dream cycles to reach high reliability.

### Test criteria

- **Codebook determinism**: Two `CodebookGenerator` instances with the same domain and dim produce identical role vectors.
- **Quantized encoding monotonicity**: For values v1 < v2, `hamming_similarity(encode(v1), encode(v2))` decreases as `|v2 - v1|` increases.
- **Threshold calibration**: The null distribution mean is within 0.001 of 0.500 for dim = 10,240.
- **Cross-domain routing**: A pattern stored by one oracle is retrievable by another oracle via `find_similar(domain: None)`.
- **CBOR round-trip**: `deserialize(serialize(store))` produces an identical PatternStore.
- **Pruning correctness**: After pruning, no pattern violates the configured thresholds.
- **Dream reliability update**: After 10 agreeing dream cycles, reliability reaches >= 0.7. After 5 disagreeing cycles from reliability 0.7, reliability drops to <= 0.2.

---

## Academic foundations

- Kanerva, P. (2009). "Hyperdimensional Computing: An Introduction." *Cognitive Computation*, 1(2), 139-159. — Binary Spatter Code (BSC) foundations.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6), 1-51. — Comprehensive HDC survey including performance benchmarks.
- Plate, T. A. (1995). "Holographic Reduced Representations." *IEEE Transactions on Neural Networks*, 6(3), 623-641. — Distributed representations for structured data.
- Frady, E. P., Kleyko, D., & Sommer, F. T. (2018). "A Theory of Sequence Indexing and Working Memory in Recurrent Neural Networks." *Neural Computation*, 30(6), 1449-1513. — Temporal encoding via permutation.
- Rachkovskij, D. A. (2001). "Representation and Processing of Structures with Binary Sparse Distributed Codes." *IEEE Transactions on Knowledge and Data Engineering*, 13(2), 261-276. — Sparse distributed memory for pattern matching.
- Lacaux, C., et al. (2021). "Sleep onset is a creative sweet spot." *Science Advances*, 7(50). — Creativity during N1 sleep, motivating hypnagogia-based pattern generation.

---

## Cross-References

- See [06-neuro](../06-neuro/INDEX.md) for HDC fundamentals and the Neuro knowledge store
- See [08-adaptive-signal-metabolism.md](./08-adaptive-signal-metabolism.md) for evolutionary dynamics of HDC-encoded signals
- See [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) for TDA + HDC pattern composition
- See [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) for somatic markers as HDC bindings


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/07-spectral-liquidity-manifolds.md

# Spectral Liquidity Manifolds

> Riemannian geometry applied to DeFi execution costs. Liquidity pools form a curved manifold where geodesics are optimal execution paths and curvature indicates structural risk.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [02-chain-oracles](./02-chain-oracles.md) for chain TA primitives, [06-hyperdimensional-ta](./06-hyperdimensional-ta.md) for pattern encoding
**Key sources**: `bardo-backup/prd/23-ta/02-spectral-liquidity-manifolds.md`

---

## Abstract

DeFi execution is not a simple price lookup. Every trade traverses a **liquidity landscape** where costs depend on pool depth, gas fees, timing, and opportunity costs. These costs vary non-linearly with trade size, time, and market conditions. The spectral liquidity manifold framework models this landscape using Riemannian geometry — the mathematical framework for curved spaces.

The core insight: a liquidity landscape is a curved space. The metric tensor encodes execution costs at each point. Geodesics (shortest paths on the manifold) are the optimal execution routes. Curvature measures structural stability — positive curvature (like a sphere) means the market self-corrects; negative curvature (like a saddle) means small perturbations amplify.

While this framework is natively chain-specific (DeFi liquidity is the domain), the mathematical structure generalizes. Any domain with spatially varying costs — CI/CD pipeline routing, research strategy selection, resource allocation — can be modeled as a manifold with a cost metric.

---

## The state manifold

The liquidity manifold is a smooth differentiable manifold M where each point represents a DeFi portfolio state:

```rust
/// A point on the liquidity manifold.
///
/// Coordinates represent the portfolio state:
/// (asset_0_balance, asset_1_balance, ..., asset_n_balance, liquidity_position_params)
///
/// The manifold dimension equals the number of independent state variables.
pub struct ManifoldPoint {
    /// Portfolio state coordinates.
    pub coordinates: Vec<f64>,

    /// Which protocol/pool this point belongs to.
    pub protocol: ProtocolId,

    /// Timestamp of the state observation.
    pub timestamp_ms: i64,
}

/// A tangent vector at a point on the manifold.
///
/// Tangent vectors represent infinitesimal trades — small changes
/// in portfolio state. The metric tensor measures the "cost" of
/// moving in each direction.
pub struct TangentVector {
    /// Components of the tangent vector in local coordinates.
    pub components: Vec<f64>,

    /// The point where this tangent vector is attached.
    pub base_point: ManifoldPoint,
}
```

### The metric tensor

The metric tensor g_ij defines the cost of moving from one state to another. It encodes four types of execution cost:

```rust
/// The metric tensor at a point on the liquidity manifold.
///
/// g_ij = slippage_ij + gas_ij + time_ij + opportunity_ij
///
/// Each component captures a different execution cost:
/// - slippage: price impact of the trade
/// - gas: transaction fee on the blockchain
/// - time: cost of waiting for confirmation
/// - opportunity: cost of capital locked during execution
pub struct MetricTensor {
    /// The n×n matrix of metric components.
    pub components: Vec<Vec<f64>>,

    /// Dimension of the manifold.
    pub dim: usize,
}

impl MetricTensor {
    /// Compute the metric tensor at a given point.
    ///
    /// This requires querying the current liquidity state of the
    /// underlying pools and computing the cost gradient in each direction.
    pub fn compute(point: &ManifoldPoint, pools: &[PoolState]) -> Self {
        let dim = point.coordinates.len();
        let mut g = vec![vec![0.0; dim]; dim];

        for i in 0..dim {
            for j in 0..dim {
                // Slippage component: d²(price_impact) / d(x_i)d(x_j)
                g[i][j] += slippage_metric(point, i, j, pools);

                // Gas component: constant per transaction, amortized
                g[i][j] += gas_metric(point, i, j);

                // Time component: confirmation time scaled by urgency
                g[i][j] += time_metric(point, i, j);

                // Opportunity component: capital lockup cost
                g[i][j] += opportunity_metric(point, i, j);
            }
        }

        MetricTensor { components: g, dim }
    }

    /// Inner product of two tangent vectors using this metric.
    /// This gives the "cost squared" of moving in direction v.
    pub fn inner_product(&self, v: &TangentVector, w: &TangentVector) -> f64 {
        let mut result = 0.0;
        for i in 0..self.dim {
            for j in 0..self.dim {
                result += self.components[i][j] * v.components[i] * w.components[j];
            }
        }
        result
    }

    /// Length of a tangent vector: the "cost" of an infinitesimal trade.
    pub fn norm(&self, v: &TangentVector) -> f64 {
        self.inner_product(v, v).sqrt()
    }
}
```

---

## Christoffel symbols — How the manifold curves

The Christoffel symbols Γ^k_ij describe how the coordinate system curves — they are the "gravitational field" of the liquidity manifold:

```rust
/// Christoffel symbols of the second kind: Γ^k_ij.
///
/// These describe how parallel transport along the manifold
/// rotates tangent vectors. In financial terms: they describe
/// how the cost of a trade changes as you move through the
/// liquidity landscape.
///
/// Γ^k_ij = (1/2) g^{kl} (∂_i g_{jl} + ∂_j g_{il} - ∂_l g_{ij})
pub struct ChristoffelSymbols {
    /// Γ^k_ij stored as [k][i][j].
    pub components: Vec<Vec<Vec<f64>>>,
    pub dim: usize,
}

impl ChristoffelSymbols {
    /// Compute Christoffel symbols from the metric tensor.
    ///
    /// Requires: metric tensor and its first derivatives at the point.
    /// Uses finite differences for derivatives when analytical forms
    /// are not available.
    pub fn compute(
        metric: &MetricTensor,
        metric_derivatives: &[MetricTensor],  // ∂_l g_{ij} for each l
    ) -> Self {
        let dim = metric.dim;
        let g_inv = metric.inverse();
        let mut gamma = vec![vec![vec![0.0; dim]; dim]; dim];

        for k in 0..dim {
            for i in 0..dim {
                for j in 0..dim {
                    for l in 0..dim {
                        gamma[k][i][j] += 0.5 * g_inv.components[k][l] * (
                            metric_derivatives[i].components[j][l] +
                            metric_derivatives[j].components[i][l] -
                            metric_derivatives[l].components[i][j]
                        );
                    }
                }
            }
        }

        ChristoffelSymbols { components: gamma, dim }
    }
}
```

---

## Geodesics — Optimal execution paths

A geodesic on the liquidity manifold is the path that minimizes total execution cost. Finding the optimal route for a DeFi trade is equivalent to solving the geodesic equation:

```rust
/// Compute the geodesic (optimal execution path) between two portfolio states.
///
/// The geodesic equation:
///   d²x^k/dt² + Γ^k_ij (dx^i/dt)(dx^j/dt) = 0
///
/// Solved numerically using 4th-order Runge-Kutta integration.
///
/// The resulting path minimizes total execution cost (slippage + gas + time + opportunity).
pub fn compute_geodesic(
    start: &ManifoldPoint,
    end: &ManifoldPoint,
    manifold: &LiquidityManifold,
    n_steps: usize,
) -> Vec<ManifoldPoint> {
    let dt = 1.0 / n_steps as f64;
    let mut path = vec![start.clone()];
    let mut velocity = initial_velocity(start, end, manifold);

    for _ in 0..n_steps {
        let point = path.last().unwrap();
        let christoffel = manifold.christoffel_at(point);

        // Geodesic equation: acceleration = -Γ^k_ij v^i v^j
        let mut acceleration = vec![0.0; manifold.dim];
        for k in 0..manifold.dim {
            for i in 0..manifold.dim {
                for j in 0..manifold.dim {
                    acceleration[k] -= christoffel.components[k][i][j]
                        * velocity.components[i]
                        * velocity.components[j];
                }
            }
        }

        // RK4 integration step
        let (new_point, new_velocity) = rk4_step(point, &velocity, &acceleration, dt);
        velocity = new_velocity;
        path.push(new_point);
    }

    path
}
```

### Geodesic interpretation

| Geodesic property | Financial meaning |
|---|---|
| **Geodesic length** | Total execution cost of the optimal path |
| **Geodesic curvature** | How far the optimal path deviates from a "straight" trade |
| **Conjugate points** | Points where alternative optimal paths exist (arbitrage opportunities) |
| **Geodesic incompleteness** | Regions where no optimal path exists (illiquid, fragmented markets) |

---

## Curvature — Structural risk

The Riemann curvature tensor and its contractions reveal structural properties of the liquidity landscape:

### Riemann curvature tensor

```rust
/// Riemann curvature tensor: R^l_{ijk}.
///
/// Measures the failure of parallel transport around an infinitesimal loop.
/// In financial terms: how much does the cost structure change as you
/// move around in the liquidity landscape?
pub struct RiemannTensor {
    pub components: Vec<Vec<Vec<Vec<f64>>>>,  // R^l_{ijk}
    pub dim: usize,
}
```

### Ricci scalar — Market stability indicator

```rust
/// Ricci scalar: R = g^{ij} R_{ij} where R_{ij} = R^k_{ikj}.
///
/// A single number that summarizes the overall curvature at a point.
///
/// R > 0 (positive curvature, sphere-like):
///   Market self-corrects. Perturbations damp out.
///   Liquidity is resilient. Safe to execute.
///
/// R = 0 (flat):
///   Execution costs are uniform. No structural effects.
///
/// R < 0 (negative curvature, saddle-like):
///   Perturbations amplify. Small trades can have outsized impact.
///   Liquidity is fragile. Exercise caution.
pub fn ricci_scalar(
    riemann: &RiemannTensor,
    metric: &MetricTensor,
) -> f64 {
    let dim = riemann.dim;
    let g_inv = metric.inverse();
    let mut scalar = 0.0;

    // Contract R^k_{ikj} to get Ricci tensor R_{ij}
    // Then contract with g^{ij} to get scalar
    for i in 0..dim {
        for j in 0..dim {
            let mut ricci_ij = 0.0;
            for k in 0..dim {
                ricci_ij += riemann.components[k][i][k][j];
            }
            scalar += g_inv.components[i][j] * ricci_ij;
        }
    }

    scalar
}
```

The Ricci scalar acts as a chain oracle signal — when it turns negative, the chain oracle increases its prediction uncertainty and the Daimon raises arousal (urgency).

---

## Parallel transport — Cross-protocol pattern transfer

Parallel transport moves a tangent vector (a trading strategy) along a path on the manifold without "rotating" it. This is how TA patterns transfer between protocols:

```rust
/// Parallel transport a vector from one point to another along a geodesic.
///
/// Financial interpretation: take a trading strategy that works on
/// Protocol A and transport it to Protocol B, adjusting for the
/// different cost structure.
///
/// d(v^k)/dt + Γ^k_ij v^i (dx^j/dt) = 0
pub fn parallel_transport(
    vector: &TangentVector,
    along_path: &[ManifoldPoint],
    manifold: &LiquidityManifold,
) -> TangentVector {
    let mut transported = vector.clone();
    let n = along_path.len();

    for step in 0..n - 1 {
        let point = &along_path[step];
        let next = &along_path[step + 1];
        let christoffel = manifold.christoffel_at(point);

        let dx: Vec<f64> = next.coordinates.iter()
            .zip(point.coordinates.iter())
            .map(|(a, b)| a - b)
            .collect();

        // Update each component: dv^k = -Γ^k_ij v^i dx^j
        let mut new_components = transported.components.clone();
        for k in 0..manifold.dim {
            let mut delta = 0.0;
            for i in 0..manifold.dim {
                for j in 0..manifold.dim {
                    delta -= christoffel.components[k][i][j]
                        * transported.components[i]
                        * dx[j];
                }
            }
            new_components[k] += delta;
        }

        transported.components = new_components;
        transported.base_point = next.clone();
    }

    transported
}
```

---

## Exponential and logarithmic maps

These maps connect the manifold to its tangent spaces, enabling local linear approximation:

```rust
/// Exponential map: project from tangent space to manifold.
///
/// Given a point p and a tangent vector v, exp_p(v) follows the
/// geodesic starting at p in direction v for unit time.
///
/// Financial interpretation: "if I execute a trade of size v
/// starting from portfolio state p, where do I end up?"
pub fn exponential_map(
    point: &ManifoldPoint,
    vector: &TangentVector,
    manifold: &LiquidityManifold,
) -> ManifoldPoint {
    // Follow geodesic from point in direction vector for t=1
    let path = compute_geodesic_from_velocity(point, vector, manifold, 100);
    path.last().cloned().unwrap()
}

/// Logarithmic map: project from manifold to tangent space.
///
/// Given two points p and q, log_p(q) is the tangent vector at p
/// that points toward q along the geodesic.
///
/// Financial interpretation: "what trade gets me from portfolio p to portfolio q
/// via the optimal (geodesic) route?"
pub fn logarithmic_map(
    from: &ManifoldPoint,
    to: &ManifoldPoint,
    manifold: &LiquidityManifold,
) -> TangentVector {
    // Solve the boundary value problem: find v such that exp_from(v) = to
    // Uses shooting method with Newton iteration
    shooting_method(from, to, manifold, max_iter: 20)
}
```

### Fréchet mean — Consensus portfolio state

```rust
/// Fréchet mean: the point on the manifold that minimizes
/// the sum of squared geodesic distances to a set of points.
///
/// Financial interpretation: the "average" portfolio state
/// that is closest to all observed states. Used to compute
/// consensus positions across a collective of agents.
///
/// Computed iteratively via the Karcher mean algorithm.
pub fn frechet_mean(
    points: &[ManifoldPoint],
    manifold: &LiquidityManifold,
    max_iter: usize,
) -> ManifoldPoint {
    let mut mean = points[0].clone();

    for _ in 0..max_iter {
        // Compute mean tangent vector
        let tangent_sum: Vec<f64> = points.iter()
            .map(|p| logarithmic_map(&mean, p, manifold))
            .fold(vec![0.0; manifold.dim], |acc, v| {
                acc.iter().zip(v.components.iter())
                    .map(|(a, b)| a + b)
                    .collect()
            });

        let mean_tangent = TangentVector {
            components: tangent_sum.iter().map(|v| v / points.len() as f64).collect(),
            base_point: mean.clone(),
        };

        // Step toward mean tangent
        let step_size = 0.5;  // damping for convergence
        let scaled = TangentVector {
            components: mean_tangent.components.iter().map(|v| v * step_size).collect(),
            base_point: mean.clone(),
        };

        mean = exponential_map(&mean, &scaled, manifold);
    }

    mean
}
```

---

## Spectral decomposition — Eigenvalue analysis

The metric tensor's eigenvalues reveal the principal directions of cost and their magnitudes:

```rust
/// Spectral decomposition of the metric tensor.
///
/// Eigenvalues: the cost magnitude in each principal direction.
///   Large eigenvalue → expensive to move in that direction.
///   Small eigenvalue → cheap to move in that direction.
///
/// Eigenvectors: the principal directions.
///   The cheapest direction is the eigenvector with smallest eigenvalue.
///   The most expensive direction has the largest eigenvalue.
///
/// Condition number (λ_max / λ_min):
///   High condition number → highly anisotropic cost structure.
///   The market strongly favors some trades over others.
pub struct SpectralDecomposition {
    pub eigenvalues: Vec<f64>,
    pub eigenvectors: Vec<Vec<f64>>,
    pub condition_number: f64,
}

impl MetricTensor {
    pub fn spectral_decomposition(&self) -> SpectralDecomposition {
        let (eigenvalues, eigenvectors) = symmetric_eigendecomposition(&self.components);
        let condition_number = eigenvalues.last().unwrap() / eigenvalues.first().unwrap();

        SpectralDecomposition {
            eigenvalues,
            eigenvectors,
            condition_number,
        }
    }
}
```

---

## Implementation details

### Metric tensor computation: Hessian of price impact

The metric tensor `g_ij` at a point is the Hessian of the total execution cost function with respect to portfolio state variables. Since analytical Hessians are unavailable for arbitrary pool types, the implementation uses central finite differences:

```rust
/// Compute the metric tensor via numerical differentiation.
///
/// Uses central finite differences on the execution cost function:
///   g_ij = d²C / dx_i dx_j
///        ≈ [C(x+ε_i+ε_j) - C(x+ε_i-ε_j) - C(x-ε_i+ε_j) + C(x-ε_i-ε_j)] / (4ε²)
///
/// The step size ε is adaptive: ε = max(|x_i| * relative_eps, absolute_eps).
pub struct MetricTensorComputer {
    /// Relative step size for finite differences.
    pub relative_eps: f64,   // default: 1e-4
    /// Absolute step size floor (prevents division by near-zero).
    pub absolute_eps: f64,   // default: 1e-8
    /// The execution cost function C(x) for a given pool configuration.
    pub cost_fn: Box<dyn Fn(&[f64], &[PoolState]) -> f64 + Send + Sync>,
}

impl MetricTensorComputer {
    /// Compute g_ij at a point using central finite differences.
    pub fn compute(&self, point: &ManifoldPoint, pools: &[PoolState]) -> MetricTensor {
        let dim = point.coordinates.len();
        let mut g = vec![vec![0.0; dim]; dim];
        let x = &point.coordinates;

        for i in 0..dim {
            let eps_i = (x[i].abs() * self.relative_eps).max(self.absolute_eps);
            for j in i..dim {
                let eps_j = (x[j].abs() * self.relative_eps).max(self.absolute_eps);

                let mut x_pp = x.clone(); x_pp[i] += eps_i; x_pp[j] += eps_j;
                let mut x_pm = x.clone(); x_pm[i] += eps_i; x_pm[j] -= eps_j;
                let mut x_mp = x.clone(); x_mp[i] -= eps_i; x_mp[j] += eps_j;
                let mut x_mm = x.clone(); x_mm[i] -= eps_i; x_mm[j] -= eps_j;

                let c_pp = (self.cost_fn)(&x_pp, pools);
                let c_pm = (self.cost_fn)(&x_pm, pools);
                let c_mp = (self.cost_fn)(&x_mp, pools);
                let c_mm = (self.cost_fn)(&x_mm, pools);

                g[i][j] = (c_pp - c_pm - c_mp + c_mm) / (4.0 * eps_i * eps_j);
                g[j][i] = g[i][j]; // symmetric
            }
        }

        MetricTensor { components: g, dim }
    }
}
```

**Configuration parameters**:

| Parameter | Default | Range | Notes |
|---|---|---|---|
| `relative_eps` | 1e-4 | 1e-6 - 1e-2 | Smaller = more accurate but noisier. 1e-4 balances accuracy and numerical noise for f64. |
| `absolute_eps` | 1e-8 | 1e-12 - 1e-4 | Floor for coordinates near zero. |

### Christoffel symbol finite difference parameters

The Christoffel symbols require first derivatives of the metric tensor. These are also computed via central finite differences:

```rust
/// Compute ∂_l g_{ij} via central finite differences on the metric.
///
///   ∂_l g_{ij} ≈ [g_{ij}(x + ε_l) - g_{ij}(x - ε_l)] / (2ε_l)
///
/// This requires 2*dim metric tensor evaluations (each itself O(dim²) cost evaluations).
/// Total cost: O(dim³) cost function evaluations per Christoffel computation.
pub fn metric_derivatives(
    computer: &MetricTensorComputer,
    point: &ManifoldPoint,
    pools: &[PoolState],
) -> Vec<MetricTensor> {
    let dim = point.coordinates.len();
    let mut derivs = Vec::with_capacity(dim);

    for l in 0..dim {
        let eps_l = (point.coordinates[l].abs() * computer.relative_eps)
            .max(computer.absolute_eps);

        let mut x_plus = point.clone();
        x_plus.coordinates[l] += eps_l;
        let g_plus = computer.compute(&x_plus, pools);

        let mut x_minus = point.clone();
        x_minus.coordinates[l] -= eps_l;
        let g_minus = computer.compute(&x_minus, pools);

        let mut dg = vec![vec![0.0; dim]; dim];
        for i in 0..dim {
            for j in 0..dim {
                dg[i][j] = (g_plus.components[i][j] - g_minus.components[i][j])
                    / (2.0 * eps_l);
            }
        }

        derivs.push(MetricTensor { components: dg, dim });
    }

    derivs
}
```

The step size for Christoffel computation should match the metric tensor step size. Using a different scale introduces inconsistency between the metric and its derivatives.

### Geodesic solver: dynamic step count and error tolerance

The geodesic solver uses adaptive 4th-order Runge-Kutta (RK4) with dynamic step count:

```rust
/// Adaptive geodesic solver with error-controlled step sizing.
///
/// Starts with `n_steps` uniform steps. After initial solve,
/// estimates local truncation error by comparing RK4 with RK2.
/// Doubles step count in regions where error exceeds tolerance.
pub struct GeodesicSolverConfig {
    /// Initial step count.
    pub initial_n_steps: usize,     // default: 100
    /// Maximum step count (prevents runaway refinement).
    pub max_n_steps: usize,         // default: 10_000
    /// Local truncation error tolerance per step.
    pub error_tolerance: f64,       // default: 1e-6
    /// Maximum geodesic parameter length (prevents infinite geodesics).
    pub max_parameter: f64,         // default: 10.0
    /// Singular point detection threshold (eigenvalue ratio).
    pub singularity_threshold: f64, // default: 1e-10
}

impl GeodesicSolverConfig {
    /// Detect singular points where the metric degenerates.
    ///
    /// A point is singular if the metric tensor's condition number
    /// exceeds 1/singularity_threshold, or if any eigenvalue is
    /// negative (the metric is no longer positive-definite).
    pub fn is_singular(&self, metric: &MetricTensor) -> bool {
        let spectral = metric.spectral_decomposition();
        let min_eigenvalue = spectral.eigenvalues.first().copied().unwrap_or(0.0);
        min_eigenvalue < self.singularity_threshold
            || spectral.condition_number > 1.0 / self.singularity_threshold
    }
}
```

**Singular point handling**: When the solver encounters a singular point (degenerate metric), it:

1. Halves the step size and retries.
2. If still singular after 3 retries, records a `GeodesicIncomplete` result with the last valid point.
3. Logs the singular location for manifold diagnostics.

### Exponential and logarithmic map parameters

```rust
/// Exponential map configuration.
pub struct ExpMapConfig {
    /// Number of geodesic integration steps.
    pub n_steps: usize,       // default: 100
    /// Error tolerance for integration.
    pub tolerance: f64,        // default: 1e-6
}

/// Logarithmic map configuration (shooting method).
///
/// The shooting method solves: find v such that exp_p(v) = q.
/// It iterates by adjusting v based on the error exp_p(v) - q.
pub struct LogMapConfig {
    /// Maximum Newton iterations for the shooting method.
    pub max_iterations: usize,     // default: 20
    /// Convergence tolerance: ||exp_p(v) - q|| < tolerance.
    pub convergence_tolerance: f64, // default: 1e-6
    /// Line search parameters (backtracking Armijo).
    pub armijo_c: f64,              // default: 1e-4
    pub armijo_tau: f64,            // default: 0.5
    /// Initial step size for Newton line search.
    pub initial_step: f64,          // default: 1.0
    /// Minimum step size before declaring failure.
    pub min_step: f64,              // default: 1e-10
}
```

The Newton line search in the logarithmic map uses backtracking Armijo conditions: accept a step if `f(x + alpha*d) <= f(x) + c*alpha*grad_f . d`, where `c = 1e-4` (sufficient decrease) and `alpha` is halved each backtrack attempt (`tau = 0.5`). Maximum backtracks: `ceil(log2(initial_step / min_step))`.

### Ricci scalar thresholds for market fragility

| Ricci scalar range | Interpretation | Agent response |
|---|---|---|
| R > 1.0 | Strongly self-correcting. Trades have predictable costs. | Execute normally. |
| 0.0 < R <= 1.0 | Mildly stable. Some cost variation. | Execute with wider slippage tolerance. |
| -0.5 <= R <= 0.0 | Neutral to mildly fragile. | Reduce position sizes by 50%. |
| -2.0 <= R < -0.5 | Fragile. Small trades amplify. | Reduce position sizes by 80%. Alert Daimon (raise Arousal). |
| R < -2.0 | Critically fragile. Market structure unstable. | Suppress all execution. Escalate to T2. |

These thresholds are configurable per protocol. Concentrated liquidity AMMs (Uniswap V3) tend toward higher curvature magnitude than constant-product AMMs (Uniswap V2), so adjust accordingly.

### Failure modes

1. **Degenerate manifold**: The metric tensor has zero or negative eigenvalues. Cause: a liquidity pool is empty or nearly so. Mitigation: skip the degenerate dimension (project out the null eigenspace) or mark the pool as unavailable.

2. **Disconnected components**: The manifold splits into disconnected regions (e.g., two isolated liquidity pools with no bridge). Geodesics between disconnected components do not exist. The solver returns `GeodesicIncomplete` with the reason `DisconnectedComponents`.

3. **Numerical instability in Christoffel symbols**: When the metric changes rapidly (high curvature), finite differences amplify truncation error. Mitigation: reduce `eps` by 10x in high-curvature regions (detected when eigenvalue ratio > 100).

4. **Ill-conditioned metric inverse**: Required for Christoffel computation. When condition number > 1e8, use pseudoinverse (SVD with eigenvalue floor at 1e-10).

5. **Geodesic divergence**: RK4 integration can diverge near singular points. The adaptive solver detects divergence when `||velocity|| > 1e6` and terminates early.

### Integration wiring

The spectral liquidity manifold integrates into the chain oracle prediction pipeline:

```
ChainOracle::predict()
  -> query on-chain pool states (via alloy provider)
  -> construct ManifoldPoint from portfolio state
  -> MetricTensorComputer::compute() at current point
  -> SpectralDecomposition for eigenvalue analysis
  -> ricci_scalar() for fragility assessment
  -> if R > threshold: compute_geodesic() for optimal execution path
  -> if R < threshold: suppress execution, raise Daimon arousal
  -> encode manifold features as HDC vector (via DeFiCodebook)
  -> emit as Engram to the witness pipeline
```

### Test criteria

- **Metric symmetry**: `g[i][j] == g[j][i]` for all i, j (within f64 epsilon).
- **Metric positive-definiteness**: All eigenvalues of a well-formed metric are positive.
- **Geodesic consistency**: `exp_p(log_p(q)) == q` within convergence tolerance.
- **Christoffel symmetry**: `Gamma[k][i][j] == Gamma[k][j][i]` (lower indices are symmetric).
- **Ricci scalar sign**: For a known constant-product AMM with deep liquidity, R > 0. For a pool at 99% depletion, R < 0.
- **Adaptive step refinement**: Halving error tolerance halves the integration error (4th-order convergence).
- **Singular point detection**: A pool with zero liquidity triggers `is_singular() == true`.

---

## Information Geometry and Statistical Manifold Extensions

The spectral liquidity manifold described above uses Riemannian geometry to model execution costs. This section extends the framework into **information geometry** -- the Riemannian geometry of probability distributions. When oracles produce probabilistic predictions, the space of those prediction distributions is itself a manifold with rich geometric structure. Information geometry provides coordinate-free tools for measuring distances between distributions, optimizing oracle parameters, and detecting distribution drift.

The key bridge: the liquidity manifold's metric tensor measures execution cost; the Fisher-Rao metric measures **statistical distinguishability**. Combining both gives a product manifold where geodesics jointly optimize execution cost and prediction accuracy.

### Fisher-Rao Metric on Oracle Distributions

```rust
/// Information-geometric extension of the liquidity manifold.
///
/// The Fisher-Rao metric is the UNIQUE Riemannian metric invariant
/// under sufficient statistics (Cencov's theorem, 1982).
///
/// When oracle predictions form a parametric family p(y|theta),
/// the Fisher information matrix defines a natural Riemannian metric:
///   G_ij(theta) = E[d(log p(y|theta))/d(theta_i) * d(log p(y|theta))/d(theta_j)]
///
/// This metric captures the "distinguishability" of nearby parameter values.
/// Two parameter values theta and theta + d(theta) are "far apart" if the
/// distributions p(y|theta) and p(y|theta + d(theta)) are easy to distinguish
/// from samples. The Fisher-Rao distance is the geodesic distance under
/// this metric.
///
/// For the oracle prediction pipeline, this means:
/// - Parameters that strongly affect predictions are "far" from each other.
/// - Parameters with redundant effects are "close" (the manifold collapses
///   in those directions, reflected by small eigenvalues of G).
/// - The volume element sqrt(det(G)) gives the Jeffreys prior --
///   the uninformative prior that is invariant under reparameterization.
///
/// (Amari & Nagaoka, 2000, "Methods of Information Geometry")
pub struct FisherRaoManifold {
    /// Dimension of the parameter space.
    pub dim: usize,
    /// Fisher information matrix at a point.
    /// Given parameter vector theta, returns the dim x dim Fisher matrix G(theta).
    pub fisher_matrix: Box<dyn Fn(&[f64]) -> Vec<Vec<f64>> + Send + Sync>,
    /// Alpha-connection parameter (alpha in [-1, 1]).
    /// alpha = 0: Levi-Civita connection (Riemannian, metric-compatible)
    /// alpha = 1: e-connection (exponential family natural parameters)
    /// alpha = -1: m-connection (mixture family natural parameters)
    /// Other values interpolate between these extremes.
    pub alpha: f64,
}

impl FisherRaoManifold {
    /// Compute the Fisher-Rao distance between two parameter values.
    ///
    /// This is the geodesic distance under the Fisher metric.
    /// For univariate Gaussians N(mu, sigma^2), the Fisher-Rao distance
    /// has a known closed form; for general families, we integrate
    /// numerically along the geodesic.
    pub fn distance(&self, theta_1: &[f64], theta_2: &[f64]) -> f64 {
        // Construct geodesic between theta_1 and theta_2 on the
        // Fisher-Rao manifold and integrate sqrt(g_ij dx^i dx^j).
        let geodesic = self.compute_geodesic(theta_1, theta_2, 200);
        let mut length = 0.0;
        for step in 0..geodesic.len() - 1 {
            let p = &geodesic[step];
            let q = &geodesic[step + 1];
            let g = (self.fisher_matrix)(p);
            let dp: Vec<f64> = q.iter().zip(p.iter()).map(|(a, b)| a - b).collect();
            let mut local_sq = 0.0;
            for i in 0..self.dim {
                for j in 0..self.dim {
                    local_sq += g[i][j] * dp[i] * dp[j];
                }
            }
            length += local_sq.max(0.0).sqrt();
        }
        length
    }

    /// Spectral decomposition of the Fisher matrix at a point.
    ///
    /// Eigenvalues reveal which parameter directions are most "informative":
    /// - Large eigenvalue: small changes in that direction produce distinguishable distributions.
    /// - Small eigenvalue: parameter is poorly identified (sloppy direction).
    ///
    /// This directly informs oracle parameter pruning:
    /// parameters along sloppy directions can be fixed without loss of predictive power.
    pub fn fisher_spectrum(&self, theta: &[f64]) -> (Vec<f64>, Vec<Vec<f64>>) {
        let g = (self.fisher_matrix)(theta);
        symmetric_eigendecomposition(&g)
    }
}
```

The Fisher-Rao manifold connects to the liquidity manifold through a product structure: the full state space is M_liq x M_stat, where M_liq carries the execution cost metric and M_stat carries the Fisher-Rao metric. Geodesics on the product manifold optimize both execution cost and parameter estimation simultaneously.

### Natural Gradient for Oracle Parameter Updates

```rust
/// Natural gradient descent on the oracle parameter manifold.
///
/// Standard gradient descent ignores the geometry of parameter space.
/// In Euclidean gradient descent, the update direction depends on how
/// the parameters are chosen (e.g., mu vs. mu^2). This is a fundamental
/// flaw: the optimizer's behavior changes under reparameterization.
///
/// Natural gradient (Amari, 1998) corrects for curvature:
///   theta_{t+1} = theta_t - eta * G(theta_t)^{-1} * grad(L(theta_t))
///
/// where G(theta) is the Fisher information matrix.
///
/// Key properties:
/// - Equivalent to steepest descent in the KL-divergence metric.
/// - Convergence: O(1/t) regardless of parameterization (coordinate-free).
/// - For exponential families: natural gradient in natural parameters
///   reduces to standard gradient in expectation parameters (and vice versa).
/// - Achieves the Cramer-Rao bound asymptotically: no other first-order
///   method converges faster in the information-geometric sense.
///
/// (Amari, 1998, "Natural Gradient Works Efficiently in Learning")
pub struct NaturalGradientOptimizer {
    pub learning_rate: f64,
    /// Tikhonov damping to regularize Fisher matrix inversion.
    /// G_reg = G + lambda * I (prevents singularity for flat directions).
    /// Interpretation: adds a small isotropic component to the metric,
    /// ensuring all directions have at least lambda curvature.
    /// Larger lambda -> more regularization -> closer to standard gradient.
    pub damping: f64,  // default: 1e-4
    /// Fisher matrix estimation method.
    pub fisher_method: FisherEstimation,
}

impl NaturalGradientOptimizer {
    /// Compute one natural gradient step.
    ///
    /// Returns the parameter update: delta_theta = -eta * G_reg^{-1} * grad_L.
    pub fn step(
        &self,
        theta: &[f64],
        gradient: &[f64],
        fisher_fn: &dyn Fn(&[f64]) -> Vec<Vec<f64>>,
    ) -> Vec<f64> {
        let dim = theta.len();
        let mut g = (fisher_fn)(theta);

        // Apply Tikhonov damping: G_reg = G + lambda * I
        for i in 0..dim {
            g[i][i] += self.damping;
        }

        // Solve G_reg * delta = gradient via Cholesky decomposition.
        // G_reg is symmetric positive-definite (given damping > 0),
        // so Cholesky is numerically stable and O(d^3 / 3).
        let natural_grad = cholesky_solve(&g, gradient);

        // Scale by learning rate
        natural_grad.iter().map(|&ng| -self.learning_rate * ng).collect()
    }
}

pub enum FisherEstimation {
    /// Exact: compute full Fisher matrix (O(d^2) per sample).
    /// Requires access to the log-likelihood's analytical Hessian.
    Exact,
    /// Empirical: Monte Carlo estimate from samples.
    /// G_hat = (1/N) sum_{n=1}^{N} grad(log p(y_n|theta)) * grad(log p(y_n|theta))^T
    /// Unbiased but high variance for small N.
    Empirical { n_samples: usize },
    /// Kronecker-factored (K-FAC, Martens & Grosse, 2015).
    /// Approximates G as Kronecker product of two smaller matrices: G ~= A (x) B.
    /// For neural network layers with d_in inputs and d_out outputs:
    ///   Full Fisher: d_in*d_out x d_in*d_out (huge)
    ///   K-FAC: d_in x d_in and d_out x d_out (manageable)
    /// Reduces inversion cost from O((d_in*d_out)^3) to O(d_in^3 + d_out^3).
    KroneckerFactored,
    /// Diagonal approximation (cheapest, O(d) per sample).
    /// Keeps only the diagonal of the Fisher matrix.
    /// Equivalent to adaptive learning rates (like Adam without momentum).
    /// Loses off-diagonal structure (parameter correlations).
    Diagonal,
}
```

**Configuration parameters**:

| Parameter | Default | Range | Notes |
|---|---|---|---|
| `learning_rate` | 0.01 | 1e-4 - 1.0 | Standard learning rate. Natural gradient is less sensitive to this than standard gradient. |
| `damping` | 1e-4 | 1e-8 - 1.0 | Too small: G may be singular. Too large: reverts to standard gradient. |
| `n_samples` (empirical) | 100 | 10 - 10000 | Variance of Fisher estimate scales as O(1/N). |

### Alpha-Connections and Dually Flat Structure

```rust
/// Alpha-connection family on the statistical manifold.
///
/// The alpha-connection nabla^(alpha) interpolates between the exponential (alpha=1)
/// and mixture (alpha=-1) connections:
///   nabla^(alpha) = (1+alpha)/2 * nabla^(e) + (1-alpha)/2 * nabla^(m)
///
/// When alpha = 0, this recovers the Levi-Civita (metric-compatible) connection.
///
/// For exponential families, the (1,-1)-connections make the manifold
/// DUALLY FLAT: both natural parameters theta and expectation parameters eta
/// yield flat coordinate systems. The canonical divergence is the
/// Bregman divergence (= KL divergence for exponential families).
///
/// Why this matters for oracle predictions:
/// - The natural parameter theta represents the "raw" oracle model parameters.
/// - The expectation parameter eta = E[T(y)] represents the sufficient statistics.
/// - The Legendre transform F(theta) <-> F*(eta) converts between them.
/// - The Bregman divergence D_F(p, q) = KL(p || q) measures prediction error
///   in a way that respects the manifold geometry.
///
/// (Amari, 2016, "Information Geometry and Its Applications", Springer)
pub struct DuallyFlatManifold {
    /// Natural (theta) coordinate system.
    /// For a Gaussian: theta = (mu/sigma^2, -1/(2*sigma^2)).
    pub theta_coords: Vec<f64>,
    /// Expectation (eta) coordinate system.
    /// For a Gaussian: eta = (mu, mu^2 + sigma^2).
    pub eta_coords: Vec<f64>,
    /// Legendre transform: F(theta) -> F*(eta) where eta = grad(F(theta)).
    /// F(theta) = log-partition function (log normalizer) for exponential families.
    pub potential: Box<dyn Fn(&[f64]) -> f64 + Send + Sync>,
    /// Conjugate potential: F*(eta) = theta . eta - F(theta).
    /// F*(eta) = negative entropy for exponential families.
    pub conjugate_potential: Box<dyn Fn(&[f64]) -> f64 + Send + Sync>,
}

impl DuallyFlatManifold {
    /// Bregman divergence (= canonical divergence on dually flat manifold).
    ///
    /// D_F(p, q) = F(theta_p) - F(theta_q) - grad(F(theta_q)) . (theta_p - theta_q)
    ///
    /// Properties:
    /// - D_F(p, q) >= 0 with equality iff p = q.
    /// - D_F(p, q) != D_F(q, p) in general (asymmetric).
    /// - For exponential families: D_F(p, q) = KL(q || p).
    ///   Note the argument swap: Bregman in theta coords = reverse KL.
    /// - The symmetrized Bregman (D_F(p,q) + D_F(q,p))/2 = (theta_p - theta_q) . (eta_p - eta_q).
    pub fn bregman_divergence(&self, p: &[f64], q: &[f64]) -> f64 {
        let f_p = (self.potential)(p);
        let f_q = (self.potential)(q);
        let grad_q = numerical_gradient(self.potential.as_ref(), q);
        f_p - f_q - dot(&grad_q, &subtract(p, q))
    }

    /// Convert natural parameters to expectation parameters.
    /// eta = grad(F(theta))
    pub fn theta_to_eta(&self, theta: &[f64]) -> Vec<f64> {
        numerical_gradient(self.potential.as_ref(), theta)
    }

    /// Convert expectation parameters to natural parameters.
    /// theta = grad(F*(eta))
    pub fn eta_to_theta(&self, eta: &[f64]) -> Vec<f64> {
        numerical_gradient(self.conjugate_potential.as_ref(), eta)
    }

    /// m-geodesic: straight line in eta (expectation) coordinates.
    ///
    /// The mixture geodesic is flat in eta space:
    ///   eta(t) = (1 - t) * eta_p + t * eta_q
    ///
    /// This corresponds to mixing distributions:
    ///   p_t = (1-t) * p + t * q  (in the mixture sense)
    pub fn m_geodesic(
        &self,
        eta_p: &[f64],
        eta_q: &[f64],
        n_steps: usize,
    ) -> Vec<Vec<f64>> {
        (0..=n_steps)
            .map(|step| {
                let t = step as f64 / n_steps as f64;
                eta_p.iter()
                    .zip(eta_q.iter())
                    .map(|(&a, &b)| (1.0 - t) * a + t * b)
                    .collect()
            })
            .collect()
    }

    /// e-geodesic: straight line in theta (natural) coordinates.
    ///
    /// The exponential geodesic is flat in theta space:
    ///   theta(t) = (1 - t) * theta_p + t * theta_q
    ///
    /// This corresponds to exponential tilting of distributions.
    pub fn e_geodesic(
        &self,
        theta_p: &[f64],
        theta_q: &[f64],
        n_steps: usize,
    ) -> Vec<Vec<f64>> {
        (0..=n_steps)
            .map(|step| {
                let t = step as f64 / n_steps as f64;
                theta_p.iter()
                    .zip(theta_q.iter())
                    .map(|(&a, &b)| (1.0 - t) * a + t * b)
                    .collect()
            })
            .collect()
    }
}
```

The dually flat structure provides two complementary views of the same oracle prediction manifold. The e-geodesic interpolates in log-likelihood space (exponential tilting), while the m-geodesic interpolates in moment space (mixture averaging). The Pythagorean theorem holds: for any triple (p, q, r) where the e-geodesic from p to r is orthogonal to the m-geodesic from q to r, we have D_F(p, q) = D_F(p, r) + D_F(r, q). This generalizes the Euclidean Pythagorean theorem to information geometry.

### Wasserstein Geometry for Distribution Transport

```rust
/// Wasserstein-2 geometry on the space of oracle prediction distributions.
///
/// The W_2 metric measures the cost of optimally transporting one distribution
/// to another. Unlike KL divergence, W_2 is a true metric (symmetric,
/// satisfies triangle inequality) and is defined even when distributions
/// have non-overlapping support.
///
/// For Gaussian predictions N(mu_1, Sigma_1) and N(mu_2, Sigma_2):
///
///   W_2^2 = ||mu_1 - mu_2||^2 + tr(Sigma_1 + Sigma_2 - 2*(Sigma_1^{1/2} Sigma_2 Sigma_1^{1/2})^{1/2})
///
/// The Bures metric (infinitesimal W_2 on Gaussians) defines a Riemannian
/// metric on the space of Gaussian distributions. This is NOT the Fisher-Rao
/// metric -- it captures different geometric structure:
/// - Fisher-Rao: statistical distinguishability (hypothesis testing).
/// - Wasserstein: physical transport cost (mass movement).
///
/// The W_2 metric captures distribution shift -- when oracle prediction
/// distributions change over time, the Wasserstein distance quantifies
/// how much the predictions have drifted. Unlike KL divergence, this
/// drift measure is bounded and interpretable as a physical distance.
///
/// (Villani, 2009, "Optimal Transport: Old and New", Springer)
pub struct WassersteinDistanceComputer {
    /// Distribution representation.
    pub representation: DistributionRepr,
}

pub enum DistributionRepr {
    /// Gaussian: track mean and covariance.
    /// W_2 has closed form for Gaussians (Bures distance).
    Gaussian { mean: Vec<f64>, cov: Vec<Vec<f64>> },
    /// Empirical: track samples.
    /// W_2 computed via linear programming (exact) or Sinkhorn (approximate).
    Empirical { samples: Vec<f64> },
    /// Histogram: discretized distribution.
    /// W_2 computed via the transportation simplex or Sinkhorn regularization.
    Histogram { bins: Vec<f64>, counts: Vec<f64> },
}

impl WassersteinDistanceComputer {
    /// Compute W_2 distance between two Gaussian distributions.
    ///
    /// Uses the closed-form Bures distance formula.
    /// Requires matrix square root computation (via Schur decomposition).
    pub fn gaussian_w2(
        mean_1: &[f64],
        cov_1: &[Vec<f64>],
        mean_2: &[f64],
        cov_2: &[Vec<f64>],
    ) -> f64 {
        let mean_diff_sq: f64 = mean_1.iter()
            .zip(mean_2.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum();

        // tr(Sigma_1 + Sigma_2 - 2 * (Sigma_1^{1/2} Sigma_2 Sigma_1^{1/2})^{1/2})
        let sqrt_1 = matrix_sqrt(cov_1);
        let product = matrix_multiply(&matrix_multiply(&sqrt_1, cov_2), &sqrt_1);
        let sqrt_product = matrix_sqrt(&product);
        let trace_term = trace(cov_1) + trace(cov_2) - 2.0 * trace(&sqrt_product);

        (mean_diff_sq + trace_term).max(0.0).sqrt()
    }

    /// Compute W_2 distance between empirical distributions via Sinkhorn.
    ///
    /// Sinkhorn divergence is an entropy-regularized approximation to W_2:
    ///   W_2^epsilon = min_P <P, C> + epsilon * KL(P || a (x) b)
    ///
    /// where C is the cost matrix, P is the transport plan, and epsilon
    /// controls regularization (epsilon -> 0 recovers exact W_2).
    ///
    /// Complexity: O(n^2 / epsilon^2) vs O(n^3 log n) for exact W_2.
    pub fn sinkhorn_w2(
        samples_1: &[f64],
        samples_2: &[f64],
        epsilon: f64,    // regularization, default: 0.01
        max_iter: usize, // default: 100
    ) -> f64 {
        let n = samples_1.len();
        let m = samples_2.len();

        // Cost matrix: C_ij = |x_i - y_j|^2
        let cost: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..m).map(|j| (samples_1[i] - samples_2[j]).powi(2)).collect())
            .collect();

        // Gibbs kernel: K_ij = exp(-C_ij / epsilon)
        let kernel: Vec<Vec<f64>> = cost.iter()
            .map(|row| row.iter().map(|&c| (-c / epsilon).exp()).collect())
            .collect();

        // Sinkhorn iterations: alternating row/column normalization
        let mut u = vec![1.0 / n as f64; n];
        let mut v = vec![1.0 / m as f64; m];

        for _ in 0..max_iter {
            // u = a ./ (K * v)
            for i in 0..n {
                let kv: f64 = (0..m).map(|j| kernel[i][j] * v[j]).sum();
                u[i] = (1.0 / n as f64) / kv.max(1e-30);
            }
            // v = b ./ (K^T * u)
            for j in 0..m {
                let ku: f64 = (0..n).map(|i| kernel[i][j] * u[i]).sum();
                v[j] = (1.0 / m as f64) / ku.max(1e-30);
            }
        }

        // Transport cost: sum_ij u_i K_ij v_j C_ij
        let mut total = 0.0;
        for i in 0..n {
            for j in 0..m {
                total += u[i] * kernel[i][j] * v[j] * cost[i][j];
            }
        }
        total.max(0.0).sqrt()
    }
}
```

The Wasserstein distance connects to the liquidity manifold through **distribution drift detection**. When the chain oracle's prediction distribution shifts (measured by W_2), the manifold metric must be recomputed. The rate of Wasserstein drift serves as a signal for metric staleness: if W_2(p_t, p_{t-1}) exceeds a threshold, the cached metric tensor and Christoffel symbols are invalidated and recomputed from fresh pool states.

### Test criteria

- **Fisher-Rao positive definiteness**: For any exponential family, the Fisher matrix has all positive eigenvalues. Verify by constructing the Fisher matrix for a Gaussian family at multiple parameter values and checking that all eigenvalues exceed zero.
- **Natural gradient coordinate invariance**: The natural gradient produces the same update (in distribution space) regardless of parameterization. Verify by computing the natural gradient update in both natural and mean parameterizations of a Gaussian and confirming that the resulting distributions match within numerical tolerance.
- **Bregman divergence non-negativity**: D_F(p, q) >= 0 with equality iff p = q. Verify for the Gaussian log-partition potential with random parameter pairs.
- **Wasserstein triangle inequality**: W_2(p, r) <= W_2(p, q) + W_2(q, r). Verify for triples of Gaussian distributions using the closed-form Bures distance.

### Information geometry references

- Amari, S. (1998). "Natural Gradient Works Efficiently in Learning." *Neural Computation*, 10(2), 251-276.
- Amari, S. (2016). *Information Geometry and Its Applications*. Springer.
- Cencov, N. N. (1982). *Statistical Decision Rules and Optimal Inference*. AMS.
- Villani, C. (2009). *Optimal Transport: Old and New*. Springer.
- Martens, J., & Grosse, R. (2015). "Optimizing Neural Networks with Kronecker-Factored Approximate Curvature." *ICML 2015*.

---

## Academic foundations

- Amari, S., & Nagaoka, H. (2000). *Methods of Information Geometry*. AMS/Oxford. — Riemannian geometry for statistical manifolds.
- do Carmo, M. P. (1992). *Riemannian Geometry*. Birkhäuser. — Standard reference for geodesics, curvature, parallel transport.
- Pennec, X. (2006). "Intrinsic Statistics on Riemannian Manifolds." *Journal of Mathematical Imaging and Vision*, 25(1), 127-154. — Fréchet mean and Karcher iteration.
- Adams, R. P., & Stegle, O. (2012). "Gaussian Process Product Models." *ICML 2012*. — GP-based metric tensor estimation.
- Bronstein, M. M., et al. (2017). "Geometric Deep Learning." *IEEE Signal Processing Magazine*, 34(6), 18-42. — Geometric methods for learning on manifolds.

---

## Cross-References

- See [02-chain-oracles.md](./02-chain-oracles.md) for chain oracle integration
- See [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) for HDC encoding of manifold features
- See [09-causal-microstructure-discovery.md](./09-causal-microstructure-discovery.md) for causal analysis of manifold dynamics
- See [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) for TDA on manifold topology


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/08-adaptive-signal-metabolism.md

# Adaptive Signal Metabolism

> Signals are living organisms. They compete for attention, reproduce when useful, die when obsolete, and evolve through mutation and selection. The TA subsystem is an ecological system governed by Hebbian learning and replicator dynamics.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [06-hyperdimensional-ta](./06-hyperdimensional-ta.md) for HDC encoding, [05-witness-as-ta-generalized](./05-witness-as-ta-generalized.md) for the witness pipeline
**Key sources**: `bardo-backup/prd/23-ta/03-adaptive-signal-metabolism.md`

---

## Signals as organisms

In the adaptive signal metabolism framework, every TA signal is treated as a living organism that exists within an ecological system. Signals compete for the limited resource of agent attention. Useful signals grow stronger (reproduce). Obsolete signals weaken (die). Novel mutations emerge from HDC recombination. The system self-organizes into an optimal signal ensemble without manual curation.

This is not metaphor — it is a direct implementation of replicator dynamics (Taylor & Jonker, 1978) and evolutionary game theory applied to signal selection.

### The signal as a 5-tuple

```rust
/// A signal organism in the adaptive metabolism framework.
///
/// Each signal is a 5-tuple: (f, C, H, W, ctx)
/// - f: the signal function (computation)
/// - C: confidence (self-assessed reliability)
/// - H: HDC vector (pattern identity)
/// - W: weight (fitness in the attention economy)
/// - ctx: context (domain-specific parameters)
pub struct AdaptiveSignal {
    /// Unique identifier.
    pub id: SignalId,

    /// The computation that produces this signal's value.
    /// In Rust: a closure or function pointer.
    pub function: Box<dyn Fn(&EngineState) -> f64 + Send + Sync>,

    /// Self-assessed confidence [0.0, 1.0].
    /// Updated via Hebbian learning after each prediction cycle.
    pub confidence: f64,

    /// HDC vector encoding this signal's identity.
    /// Used for similarity search and cross-domain matching.
    pub hdc_vector: HdcVector,

    /// Weight (fitness) in the attention economy.
    /// Signals with higher weight get more attention budget.
    /// Updated via replicator dynamics.
    pub weight: f64,

    /// Domain-specific context for this signal.
    pub context: SignalContext,

    /// Lineage: parent signals this was derived from (for evolution tracking).
    pub lineage: Vec<SignalId>,

    /// Generation counter (how many mutation/selection cycles).
    pub generation: u64,

    /// Birth timestamp.
    pub created_at_ms: i64,

    /// Number of times this signal has been evaluated.
    pub evaluation_count: u64,

    /// Running accuracy statistics.
    pub accuracy: ExponentialMovingAverage,
}
```

---

## Hebbian learning — "Neurons that fire together wire together"

Signal confidence is updated via Oja's rule, a normalized variant of Hebbian learning that prevents runaway weight growth:

```rust
/// Hebbian update for signal confidence.
///
/// When a signal's prediction correlates with the actual outcome,
/// confidence increases. When it anti-correlates, confidence decreases.
///
/// Uses Oja's rule (Oja, 1982) for stability:
///   Δw = η × y × (x - y × w)
///
/// where:
///   w = current confidence
///   x = signal value (prediction)
///   y = outcome (actual value)
///   η = learning rate (typically 0.01-0.05)
///
/// The (- y × w) term prevents weights from growing without bound.
pub fn hebbian_update(
    signal: &mut AdaptiveSignal,
    prediction: f64,
    outcome: f64,
    learning_rate: f64,
) {
    let delta = learning_rate * outcome * (prediction - outcome * signal.confidence);
    signal.confidence = (signal.confidence + delta).clamp(0.0, 1.0);
}

/// Batch Hebbian update across all signals in the registry.
///
/// After each prediction resolution, all signals that contributed
/// to the prediction have their confidence updated.
pub fn batch_hebbian_update(
    registry: &mut SignalRegistry,
    predictions: &[(SignalId, f64)],
    outcome: f64,
    learning_rate: f64,
) {
    for (signal_id, prediction) in predictions {
        if let Some(signal) = registry.get_mut(signal_id) {
            hebbian_update(signal, *prediction, outcome, learning_rate);
            signal.evaluation_count += 1;
            signal.accuracy.update((prediction - outcome).abs());
        }
    }
}
```

---

## Replicator dynamics — Fitness-proportionate selection

Signal weights evolve according to replicator dynamics (Taylor & Jonker, 1978): signals with above-average fitness gain weight; below-average signals lose weight. This creates a self-organizing ensemble without manual threshold tuning:

```rust
/// Replicator dynamics update for signal weights.
///
/// The replicator equation:
///   dw_i/dt = w_i × (f_i - f̄)
///
/// where:
///   w_i = weight of signal i
///   f_i = fitness of signal i
///   f̄ = average fitness across all signals
///
/// Fitness is the accuracy of the signal's predictions.
/// Signals that predict better than average grow.
/// Signals that predict worse than average shrink.
pub fn replicator_update(registry: &mut SignalRegistry, dt: f64) {
    let signals: Vec<(SignalId, f64, f64)> = registry.iter()
        .map(|s| (s.id, s.weight, s.fitness()))
        .collect();

    let total_weight: f64 = signals.iter().map(|(_, w, _)| w).sum();
    let avg_fitness: f64 = signals.iter()
        .map(|(_, w, f)| w * f / total_weight)
        .sum();

    for (id, weight, fitness) in &signals {
        let delta = weight * (fitness - avg_fitness) * dt;
        if let Some(signal) = registry.get_mut(id) {
            signal.weight = (signal.weight + delta).max(0.001);  // floor to prevent extinction
        }
    }

    // Normalize weights to sum to 1.0
    registry.normalize_weights();
}
```

### Fisher's fundamental theorem

Fisher's fundamental theorem of natural selection (Fisher, 1930) applies: the rate of increase in mean fitness equals the genetic variance in fitness. In signal terms: **the rate at which the signal ensemble improves equals the diversity of signal quality**. This has a practical implication — if all signals have similar accuracy, improvement stalls. The system must maintain diversity to keep improving.

```rust
/// Compute Fisher's variance (rate of improvement potential).
///
/// V(fitness) = Σ w_i × (f_i - f̄)²
///
/// When V is high: the ensemble is rapidly improving.
/// When V approaches 0: the ensemble has converged (may need mutation injection).
pub fn fisher_variance(registry: &SignalRegistry) -> f64 {
    let avg_fitness = registry.mean_fitness();
    registry.iter()
        .map(|s| s.weight * (s.fitness() - avg_fitness).powi(2))
        .sum()
}
```

---

## Speciation — Signal evolution

New signals emerge through mutation of successful parent signals:

```rust
/// Signal speciation: create a new signal by mutating a parent.
///
/// The mutation operator perturbs the parent's HDC vector
/// (XOR with a random noise vector of controlled density)
/// and adjusts the signal function parameters.
///
/// New signals start with low weight (0.01) and must prove
/// themselves through replicator dynamics before gaining
/// significant attention budget.
pub fn speciate(
    parent: &AdaptiveSignal,
    mutation_rate: f64,
    rng: &mut impl Rng,
) -> AdaptiveSignal {
    // Mutate HDC vector: flip bits with probability = mutation_rate
    let noise = HdcVector::random_with_density(mutation_rate, rng);
    let mutated_hv = parent.hdc_vector.xor(&noise);

    // Mutate signal function parameters
    let mutated_function = parent.function.mutate(mutation_rate, rng);

    AdaptiveSignal {
        id: SignalId::new(),
        function: mutated_function,
        confidence: 0.5,  // start neutral
        hdc_vector: mutated_hv,
        weight: 0.01,  // start with minimal weight
        context: parent.context.clone(),
        lineage: {
            let mut l = parent.lineage.clone();
            l.push(parent.id);
            l
        },
        generation: parent.generation + 1,
        created_at_ms: now_ms(),
        evaluation_count: 0,
        accuracy: ExponentialMovingAverage::new(0.1),
    }
}
```

### Fitness landscape (Sewall Wright, 1932)

The ensemble of signals exists on a fitness landscape — a surface where each point is a signal configuration and the height is its fitness. The replicator dynamics push signals uphill (toward higher fitness), but speciation (mutation) allows escape from local optima:

```rust
/// The fitness landscape for a signal ensemble.
///
/// Each signal's position is its HDC vector (in 10,240-bit space).
/// The height at each position is the signal's fitness (accuracy).
///
/// Properties:
/// - Rugged: many local optima (similar signals with different accuracy)
/// - Dynamic: the landscape shifts as market/code/research conditions change
/// - High-dimensional: 10,240 dimensions, many escape routes from local optima
///
/// Navigation via: replicator dynamics (hill climbing) + speciation (exploration)
pub struct FitnessLandscape {
    /// Current signal positions and heights.
    pub signals: Vec<(HdcVector, f64)>,

    /// Landscape roughness (variance of fitness across neighbors).
    pub roughness: f64,

    /// Landscape shift rate (how fast the landscape changes).
    pub shift_rate: f64,
}
```

### Red Queen dynamic

Following Van Valen's Red Queen hypothesis (1973): in adversarial environments, signals must continuously evolve just to maintain their fitness, because the environment (adversary) co-evolves. In the chain domain, MEV searchers adapt to agent strategies. In the coding domain, codebase structure evolves. Signals that stop evolving become obsolete:

```rust
/// Red Queen pressure: signals must evolve to maintain fitness.
///
/// Implemented as a constant downward pressure on all signal weights:
///   w_i(t+1) = w_i(t) × (1 - decay_rate) + replicator_delta
///
/// Without improvement, signals decay toward zero.
/// Only signals that continuously outperform survive.
pub fn apply_red_queen_pressure(registry: &mut SignalRegistry, decay_rate: f64) {
    for signal in registry.iter_mut() {
        signal.weight *= 1.0 - decay_rate;
    }
}
```

---

## SignalRegistry — The ecosystem container

```rust
/// The signal registry: manages the full signal ecosystem.
///
/// Contains all living signals, tracks their fitness over time,
/// manages speciation and extinction events.
pub struct SignalRegistry {
    /// All active signals.
    signals: HashMap<SignalId, AdaptiveSignal>,

    /// Maximum population (attention budget constraint).
    max_population: usize,

    /// Speciation rate (probability of mutation per generation).
    speciation_rate: f64,

    /// Extinction threshold (minimum weight before removal).
    extinction_threshold: f64,

    /// Generation counter.
    generation: u64,
}

impl SignalRegistry {
    /// Run one evolutionary step.
    ///
    /// 1. Evaluate all signals against recent data
    /// 2. Hebbian update of confidence
    /// 3. Replicator dynamics update of weights
    /// 4. Speciate: create mutations of top performers
    /// 5. Extinction: remove signals below threshold
    /// 6. Red Queen: apply constant decay pressure
    pub fn evolve_step(&mut self, data: &[Engram], outcomes: &[Engram]) {
        // 1. Evaluate
        let predictions = self.evaluate_all(data);

        // 2. Hebbian update
        for (pred, outcome) in predictions.iter().zip(outcomes) {
            batch_hebbian_update(self, &pred.signal_contributions, outcome.numeric_value(), 0.02);
        }

        // 3. Replicator dynamics
        replicator_update(self, dt: 1.0);

        // 4. Speciation
        let top_signals: Vec<_> = self.top_k(5);
        for parent in &top_signals {
            if rand::random::<f64>() < self.speciation_rate {
                let child = speciate(parent, mutation_rate: 0.05, &mut rng);
                self.insert(child);
            }
        }

        // 5. Extinction
        self.remove_below_threshold(self.extinction_threshold);

        // 6. Red Queen
        apply_red_queen_pressure(self, decay_rate: 0.001);

        // 7. Enforce population cap
        while self.signals.len() > self.max_population {
            self.remove_weakest();
        }

        self.generation += 1;
    }
}
```

---

## Heartbeat integration

The signal metabolism operates at all three cognitive speeds:

| Speed | Signal metabolism activity |
|---|---|
| **Gamma** (~5-15s) | Signals evaluate against current data. No learning. Cost: microseconds. |
| **Theta** (~75s) | Hebbian update + replicator dynamics step. Predictions resolve. |
| **Delta** (hours) | Full evolutionary step: speciation, extinction, Red Queen. Landscape analysis. |

At Gamma frequency, the signal registry is read-only — probes read signal values but don't update weights. This ensures the T0 probe system (80% of ticks costing nothing) is not disrupted by evolutionary computation.

At Theta frequency, learning happens — confidence updates and weight adjustments based on resolved predictions.

At Delta frequency, the full evolutionary cycle runs — new signals are born, old ones die, and the fitness landscape is analyzed for stagnation.

---

## Domain-specific signal contexts

```rust
/// Domain-specific context for signal metabolism.
pub enum SignalContext {
    /// Chain signals: DeFi-specific parameters.
    Chain(ChainSignalContext),

    /// Coding signals: software engineering parameters.
    Coding(CodingSignalContext),

    /// Research signals: information analysis parameters.
    Research(ResearchSignalContext),

    /// Custom domain.
    Custom(serde_json::Value),
}

pub struct ChainSignalContext {
    /// Which protocols this signal monitors.
    pub protocols: Vec<ProtocolId>,
    /// Which assets this signal tracks.
    pub assets: Vec<AssetId>,
    /// Time granularity (block-level, minute, hourly).
    pub granularity: Duration,
}

pub struct CodingSignalContext {
    /// Which crates/modules this signal monitors.
    pub scope: CodingScope,
    /// Which metrics this signal tracks.
    pub metrics: Vec<CodingMetric>,
    /// Event granularity (commit-level, CI run, daily).
    pub granularity: Duration,
}

pub struct ResearchSignalContext {
    /// Which topics this signal covers.
    pub topics: Vec<String>,
    /// Which source types this signal evaluates.
    pub source_types: Vec<SourceType>,
    /// Evaluation cadence.
    pub granularity: Duration,
}
```

---

## Implementation details

### Replicator dynamics: dt semantics and numerical stability

The `dt` parameter in `replicator_update()` represents the elapsed time in arbitrary units since the last update. In practice:

- At **Theta frequency** (~75s): `dt = 1.0` (one Theta tick = one evolutionary time unit).
- At **Delta frequency** (hours): `dt` accumulates missed Theta ticks if replicator was not called at Theta, but this is not recommended. Run replicator at every Theta tick.

**Numerical stability**: The replicator equation `dw_i/dt = w_i * (f_i - f_bar)` is solved via forward Euler. This is adequate when `dt * max(|f_i - f_bar|) < 1.0`. If this condition fails, weights can go negative.

```rust
/// Safe replicator update with stability check.
///
/// Forward Euler is stable when dt * max_fitness_deviation < 1.0.
/// If this condition fails, subdivide the step.
pub fn replicator_update_safe(registry: &mut SignalRegistry, dt: f64) {
    let signals: Vec<(SignalId, f64, f64)> = registry.iter()
        .map(|s| (s.id, s.weight, s.fitness()))
        .collect();

    let total_weight: f64 = signals.iter().map(|(_, w, _)| w).sum();
    let avg_fitness: f64 = signals.iter()
        .map(|(_, w, f)| w * f / total_weight)
        .sum();

    let max_deviation = signals.iter()
        .map(|(_, _, f)| (f - avg_fitness).abs())
        .fold(0.0f64, f64::max);

    // Subdivide if Euler would be unstable
    let n_substeps = ((dt * max_deviation).ceil() as usize).max(1);
    let sub_dt = dt / n_substeps as f64;

    for _ in 0..n_substeps {
        for (id, weight, fitness) in &signals {
            let delta = weight * (fitness - avg_fitness) * sub_dt;
            if let Some(signal) = registry.get_mut(id) {
                signal.weight = (signal.weight + delta).max(0.001);
            }
        }
    }

    registry.normalize_weights();
}
```

RK4 is an alternative but provides negligible benefit here because the replicator dynamics are evaluated at coarse (Theta) intervals and the forward Euler error is dominated by the discretization of the fitness landscape, not the integrator.

### Speciation: adaptive mutation rate

The mutation rate adapts based on Red Queen pressure and Fisher's variance:

```rust
/// Compute adaptive mutation rate.
///
/// When Fisher's variance is low (ensemble converged), increase mutation
/// to inject diversity. When variance is high (actively evolving),
/// reduce mutation to let selection operate.
///
/// The formula incorporates Red Queen pressure: in adversarial domains
/// (chain), mutation rate has a higher floor.
///
///   mutation_rate = base_rate * (1.0 + rq_pressure) / (1.0 + fisher_v / fisher_scale)
///
/// where:
///   base_rate:    0.05 (default)
///   rq_pressure:  0.0 (coding) to 1.0 (chain, adversarial)
///   fisher_v:     current Fisher's variance
///   fisher_scale: 0.1 (normalizing constant)
pub fn adaptive_mutation_rate(
    base_rate: f64,
    red_queen_pressure: f64,
    fisher_variance: f64,
) -> f64 {
    let fisher_scale = 0.1;
    let rate = base_rate * (1.0 + red_queen_pressure) / (1.0 + fisher_variance / fisher_scale);
    rate.clamp(0.01, 0.3) // never below 1% or above 30%
}
```

### HdcVector::random_with_density() distribution

`random_with_density(density, rng)` generates a 10,240-bit vector where each bit is independently set to 1 with probability `density`:

- `density = 0.5`: standard dense random vector (used for codebook generation).
- `density = 0.05`: sparse noise vector (used for mutation). On average, 512 bits are flipped.
- `density = 0.001`: very sparse noise (used for fine-tuning). On average, ~10 bits are flipped.

The distribution is Bernoulli per bit. Implementation uses `rng.gen::<f64>() < density` per bit (slow) or batch generation via geometric distribution of inter-bit gaps (fast, O(density * dim) expected operations).

### Fitness computation

`s.fitness()` returns a composite score combining prediction accuracy and information value:

```rust
impl AdaptiveSignal {
    /// Compute fitness for replicator dynamics.
    ///
    /// fitness = accuracy_ema * (1.0 + information_ratio)
    ///
    /// accuracy_ema:      exponential moving average of |prediction - outcome|,
    ///                    inverted so higher accuracy = higher fitness.
    ///                    Specifically: 1.0 - accuracy.value() where accuracy
    ///                    tracks mean absolute error.
    ///
    /// information_ratio: how much unique information this signal provides
    ///                    beyond what other signals already cover.
    ///                    Computed as 1.0 - max_correlation_with_other_signals.
    ///                    Range: [0.0, 1.0]. Higher = more unique.
    pub fn fitness(&self) -> f64 {
        let accuracy_score = 1.0 - self.accuracy.value().min(1.0);
        let info_ratio = self.information_ratio.unwrap_or(0.5);
        accuracy_score * (1.0 + info_ratio)
    }
}
```

**Range**: `[0.0, 2.0]`. A signal with perfect accuracy and completely unique information scores 2.0. A signal with zero accuracy scores 0.0 regardless of uniqueness.

### Heartbeat integration state machine

Signal metabolism integrates with the heartbeat via a three-state machine:

```
State: GAMMA (read-only)
  Entry: heartbeat tick at Gamma frequency (~5-15s)
  Action: evaluate all signals, collect predictions. No weight updates.
  Transition: on Theta tick -> THETA

State: THETA (learning)
  Entry: heartbeat tick at Theta frequency (~75s)
  Action:
    1. Resolve predictions from last Theta cycle.
    2. Hebbian update of signal confidence.
    3. Replicator dynamics update of signal weights.
    4. Update Fisher's variance.
  Transition: on Delta tick -> DELTA
               on Gamma tick -> GAMMA

State: DELTA (evolution)
  Entry: heartbeat tick at Delta frequency (hours)
  Action:
    1. Run full replicator update with accumulated dt.
    2. Speciate: mutate top-k signals (k = 5, mutation_rate from adaptive formula).
    3. Extinction: remove signals with weight < extinction_threshold (default: 0.001).
    4. Red Queen pressure: decay all weights by decay_rate (default: 0.001).
    5. Enforce population cap (default: 500).
    6. Analyze fitness landscape for stagnation.
    7. If Fisher's variance < 0.001: inject 10 random signals to restore diversity.
  Transition: on Gamma tick -> GAMMA
```

### Oja's rule learning rate calibration

The learning rate `eta` for Oja's rule should be calibrated per domain:

| Domain | Recommended eta | Rationale |
|---|---|---|
| Chain (DeFi) | 0.01 | High noise, slow learning prevents overfit to flash events. |
| Coding | 0.05 | Lower noise, faster adaptation to codebase changes. |
| Research | 0.02 | Moderate noise, medium adaptation speed. |

The learning rate can be made adaptive: `eta = base_eta / (1.0 + 0.01 * evaluation_count)`. This annealing schedule reduces learning rate as the signal accumulates more observations, following the Robbins-Monro conditions for stochastic approximation convergence.

### normalize_weights() semantics

`normalize_weights()` rescales all signal weights so they sum to 1.0:

```rust
impl SignalRegistry {
    /// Normalize weights so they sum to 1.0.
    ///
    /// Preserves relative proportions. Does NOT preserve absolute magnitudes.
    /// After normalization, each weight represents the signal's share of the
    /// total attention budget.
    ///
    /// If total weight is zero (all signals extinct), distributes weight
    /// uniformly: each signal gets 1.0 / n.
    pub fn normalize_weights(&mut self) {
        let total: f64 = self.signals.values().map(|s| s.weight).sum();
        if total < 1e-12 {
            // All weights near zero: reset to uniform
            let uniform = 1.0 / self.signals.len() as f64;
            for s in self.signals.values_mut() {
                s.weight = uniform;
            }
        } else {
            for s in self.signals.values_mut() {
                s.weight /= total;
            }
        }
    }
}
```

The sum-to-1.0 convention means weights are interpretable as probability distributions over signals. This is consistent with the replicator dynamics formulation where weights are population shares.

### Error handling

- **Division by zero in replicator**: If `total_weight == 0`, skip the replicator step and log a warning. This can only happen if all signals were externally removed.
- **NaN in Hebbian update**: If prediction or outcome is NaN, skip the update for that signal.
- **Population collapse**: If signal count drops below `min_population` (default: 10) after extinction, inject `min_population - current` random signals.
- **Infinite fitness**: Clamp fitness to `[0.0, 10.0]` to prevent a single signal from dominating.

### Test criteria

- **Replicator conservation**: After `replicator_update()`, total weight is unchanged (before normalization).
- **Replicator stability**: With `dt = 1.0` and fitness deviations < 1.0, no weight goes negative.
- **Hebbian convergence**: A signal that always predicts correctly converges to confidence ~1.0 within 100 updates.
- **Speciation diversity**: After speciation, `hamming_similarity(parent, child)` is between 0.9 and 0.99 for mutation_rate = 0.05.
- **Extinction threshold**: After `evolve_step()`, no signal has weight < `extinction_threshold` (they are removed).
- **Fisher's variance monotonicity**: When all signals have identical fitness, Fisher's variance is 0.0.
- **Normalize idempotence**: Calling `normalize_weights()` twice produces the same result.

---

## Academic foundations

- Taylor, P. D., & Jonker, L. B. (1978). "Evolutionary Stable Strategies and Game Dynamics." *Mathematical Biosciences*, 40(1-2), 145-156. — Replicator dynamics.
- Fisher, R. A. (1930). *The Genetical Theory of Natural Selection*. Clarendon Press. — Fisher's fundamental theorem.
- Wright, S. (1932). "The Roles of Mutation, Inbreeding, Crossbreeding, and Selection in Evolution." *Proceedings of the Sixth International Congress of Genetics*, 1, 356-366. — Fitness landscapes.
- Van Valen, L. (1973). "A New Evolutionary Law." *Evolutionary Theory*, 1, 1-30. — Red Queen hypothesis.
- Oja, E. (1982). "Simplified neuron model as a principal component analyzer." *Journal of Mathematical Biology*, 15(3), 267-273. — Oja's learning rule.
- Hebb, D. O. (1949). *The Organization of Behavior*. Wiley. — Hebbian learning.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6). — HDC operations for signal encoding.

---

## Cross-References

- See [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) for HDC encoding fundamentals
- See [05-witness-as-ta-generalized.md](./05-witness-as-ta-generalized.md) for the data pipeline that feeds signals
- See [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) for resonant pattern ecosystems
- See [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) for emergent intelligence from signal interactions


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/09-causal-microstructure-discovery.md

# Causal Microstructure Discovery

> Correlation is not causation. The causal discovery subsystem uses Pearl's structural causal models, Granger causality, and interventional experiments (via mirage-rs simulation) to discover genuine causal relationships in structured domains.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [01-oracle-trait](./01-oracle-trait.md) for prediction integration, [06-hyperdimensional-ta](./06-hyperdimensional-ta.md) for pattern encoding
**Key sources**: `bardo-backup/prd/23-ta/04-causal-microstructure-discovery.md`

---

## Pearl's causal hierarchy in Roko

Judea Pearl's causal hierarchy (Pearl, 2009, *Causality*) defines three levels of causal reasoning. Roko implements all three:

| Level | Question | Roko implementation |
|---|---|---|
| **L1: Association** (seeing) | "What is the probability of Y given X?" | Standard TA indicators (correlation, regression) |
| **L2: Intervention** (doing) | "What happens to Y if I do X?" | Mirage-rs simulation (do-operator) |
| **L3: Counterfactual** (imagining) | "Would Y have occurred if X hadn't happened?" | Dreams counterfactual engine (REM phase) |

Most TA systems operate exclusively at Level 1. They detect correlations — "when RSI drops below 30, price tends to rise." But correlation is not causation. The RSI drop and the price rise might both be caused by a third variable (e.g., a whale liquidation). Acting on correlation without understanding causation leads to fragile strategies that fail when the causal structure changes.

Roko's causal discovery subsystem moves the agent up Pearl's hierarchy, enabling genuinely causal predictions that survive regime changes.

---

## Structural Causal Model (SCM)

```rust
/// A structural causal model (SCM) following Pearl's formalism.
///
/// An SCM is a tuple (U, V, F) where:
/// - U: exogenous (external, unobserved) variables
/// - V: endogenous (internal, observed) variables
/// - F: structural equations V_i = f_i(Pa(V_i), U_i)
///   where Pa(V_i) are the parents of V_i in the causal graph
pub struct StructuralCausalModel {
    /// Exogenous variables (external factors).
    pub exogenous: Vec<Variable>,

    /// Endogenous variables (observed state).
    pub endogenous: Vec<Variable>,

    /// Structural equations: each variable is a function of its parents.
    pub equations: HashMap<VariableId, StructuralEquation>,

    /// The causal DAG (directed acyclic graph).
    pub graph: CausalGraph,
}

pub struct Variable {
    pub id: VariableId,
    pub name: String,
    pub domain: VariableDomain,
}

pub enum VariableDomain {
    Continuous { min: f64, max: f64 },
    Discrete(Vec<String>),
    Binary,
}

/// A structural equation: V_i = f(Pa(V_i), U_i).
pub struct StructuralEquation {
    /// The variable this equation defines.
    pub target: VariableId,

    /// Parent variables (causal inputs).
    pub parents: Vec<VariableId>,

    /// The functional form.
    pub function: Box<dyn Fn(&HashMap<VariableId, f64>) -> f64 + Send + Sync>,

    /// Exogenous noise distribution.
    pub noise: NoiseDistribution,
}
```

### The do-operator

Pearl's do-operator `do(X = x)` intervenes on the model by setting variable X to value x and removing all incoming edges to X. This breaks the causal mechanism that normally determines X, allowing us to measure the pure causal effect of X on downstream variables:

```rust
/// Apply the do-operator to an SCM.
///
/// do(X = x):
///   1. Set X = x (override its structural equation)
///   2. Remove all incoming edges to X in the causal graph
///   3. Propagate through remaining structural equations
///
/// Returns the distribution of downstream variables under intervention.
pub fn do_intervention(
    scm: &StructuralCausalModel,
    variable: VariableId,
    value: f64,
) -> InterventionalDistribution {
    // Create a modified SCM with X fixed
    let mut modified = scm.clone();

    // Replace X's equation with a constant
    modified.equations.insert(variable, StructuralEquation {
        target: variable,
        parents: vec![],  // no parents — intervention severs incoming arrows
        function: Box::new(move |_| value),
        noise: NoiseDistribution::Constant(0.0),
    });

    // Remove incoming edges to X in the graph
    modified.graph.remove_incoming_edges(variable);

    // Propagate through the modified model
    modified.propagate()
}
```

---

## Causal discovery algorithms

### PC Algorithm (Spirtes, Glymour, Scheines, 2000)

The PC algorithm discovers the causal graph structure from observational data:

```rust
/// The PC (Peter-Clark) algorithm for causal graph discovery.
///
/// Spirtes, Glymour, & Scheines (2000), "Causation, Prediction, and Search"
///
/// 1. Start with a complete undirected graph
/// 2. Remove edges based on conditional independence tests
/// 3. Orient edges based on v-structures (colliders)
/// 4. Apply Meek's orientation rules
///
/// Output: a Partially Directed Acyclic Graph (PDAG)
pub fn pc_algorithm(
    data: &DataFrame,
    alpha: f64,  // significance level for independence tests (typically 0.05)
    max_conditioning_set: usize,
) -> CausalGraph {
    let variables: Vec<VariableId> = data.columns().collect();
    let mut graph = CausalGraph::complete_undirected(&variables);

    // Phase I: Edge removal via conditional independence
    for conditioning_size in 0..=max_conditioning_set {
        for (x, y) in graph.edges() {
            let neighbors = graph.neighbors(x);
            for conditioning_set in neighbors.combinations(conditioning_size) {
                if conditional_independence_test(data, x, y, &conditioning_set, alpha) {
                    graph.remove_edge(x, y);
                    graph.add_separation_set(x, y, conditioning_set);
                    break;
                }
            }
        }
    }

    // Phase II: Orient v-structures
    for (x, z) in graph.undirected_edges() {
        for y in graph.common_neighbors(x, z) {
            if !graph.separation_set(x, z).contains(&y) {
                // x → y ← z is a v-structure (collider)
                graph.orient(x, y);
                graph.orient(z, y);
            }
        }
    }

    // Phase III: Meek's orientation rules
    graph.apply_meek_rules();

    graph
}
```

### Granger causality with DeFi extensions

Granger causality (Granger, 1969) tests whether past values of X help predict Y beyond Y's own past values. Four extensions adapt it to DeFi:

```rust
/// Granger causality test with DeFi-specific extensions.
///
/// Base test: does X_{t-k} Granger-cause Y_t?
/// H0: past values of X add no predictive power for Y
/// H1: past values of X improve Y prediction
pub struct GrangerCausalityTest {
    /// Maximum lag order to test.
    pub max_lag: usize,
    /// Significance level.
    pub alpha: f64,
}

impl GrangerCausalityTest {
    /// Extension 1: Block-aware Granger causality.
    ///
    /// Standard Granger assumes uniform time steps.
    /// Blockchain data has variable block times and MEV-induced
    /// ordering effects. This extension uses block number as the
    /// time index and accounts for intra-block ordering.
    pub fn block_aware(&self, x: &TimeSeries, y: &TimeSeries, blocks: &[Block]) -> GrangerResult;

    /// Extension 2: Cross-protocol Granger causality.
    ///
    /// Tests whether events on Protocol A Granger-cause events on
    /// Protocol B. Accounts for different time granularities across
    /// protocols (Uniswap has per-swap data, Aave has per-block updates).
    pub fn cross_protocol(&self, x: &ProtocolSeries, y: &ProtocolSeries) -> GrangerResult;

    /// Extension 3: Multi-chain Granger causality.
    ///
    /// Tests whether events on Chain A Granger-cause events on Chain B.
    /// Accounts for bridge latency and cross-chain message propagation.
    pub fn multi_chain(&self, x: &ChainSeries, y: &ChainSeries) -> GrangerResult;

    /// Extension 4: MEV-adjusted Granger causality.
    ///
    /// Removes MEV-induced spurious correlations (sandwich attacks
    /// create artificial dependencies between transactions that are
    /// not genuinely causal).
    pub fn mev_adjusted(&self, x: &TimeSeries, y: &TimeSeries, mev_labels: &[bool]) -> GrangerResult;
}
```

---

## Interventional discovery via mirage-rs

The deepest causal reasoning requires interventions — actively changing variables and observing effects. In the chain domain, mirage-rs enables this without risking real assets:

```rust
/// Interventional causal discovery using mirage-rs simulation.
///
/// The agent constructs causal hypotheses from observational data,
/// then tests them by simulating interventions:
///
/// 1. Observe: "When pool TVL drops, gas spikes."
/// 2. Hypothesize: "TVL drop → liquidation cascade → gas spike" (causal)
///    vs. "Both caused by external event (whale movement)" (confounded)
/// 3. Intervene: In mirage-rs, force TVL to drop while holding
///    external factors constant.
/// 4. Observe: If gas spikes in the simulation, the causal hypothesis
///    is supported. If not, it's confounded.
pub struct InterventionalDiscovery {
    /// The simulation environment.
    mirage: Arc<MirageSimulator>,

    /// The current causal model.
    scm: StructuralCausalModel,

    /// Hypotheses to test.
    hypotheses: Vec<CausalHypothesis>,
}

pub struct CausalHypothesis {
    /// Hypothesized cause.
    pub cause: VariableId,

    /// Hypothesized effect.
    pub effect: VariableId,

    /// Hypothesized mechanism (intermediate variables).
    pub mechanism: Vec<VariableId>,

    /// Confidence in this hypothesis.
    pub confidence: f64,

    /// Test results (from interventional experiments).
    pub test_results: Vec<InterventionResult>,
}

pub struct InterventionResult {
    /// The intervention applied.
    pub intervention: (VariableId, f64),

    /// The observed effect.
    pub observed_effect: f64,

    /// The predicted effect (from the causal model).
    pub predicted_effect: f64,

    /// Whether the hypothesis was supported.
    pub supported: bool,
}

impl InterventionalDiscovery {
    /// Run an interventional experiment.
    pub async fn test_hypothesis(
        &self,
        hypothesis: &CausalHypothesis,
    ) -> InterventionResult {
        // Fork the current chain state in mirage-rs
        let fork = self.mirage.fork_current_state().await;

        // Apply the intervention (set cause variable to a specific value)
        fork.set_variable(hypothesis.cause, 0.5).await;  // e.g., reduce TVL by 50%

        // Advance the simulation for the hypothesized propagation time
        fork.advance_blocks(10).await;

        // Observe the effect variable
        let observed = fork.get_variable(hypothesis.effect).await;
        let predicted = do_intervention(&self.scm, hypothesis.cause, 0.5)
            .mean(hypothesis.effect);

        InterventionResult {
            intervention: (hypothesis.cause, 0.5),
            observed_effect: observed,
            predicted_effect: predicted,
            supported: (observed - predicted).abs() < 0.1,
        }
    }
}
```

### Coding domain causal discovery

In the coding domain, interventional experiments use the workspace itself as the simulation environment:

```rust
/// Coding causal discovery: test whether code change X causes test failure Y.
///
/// Example hypothesis: "Modifying auth.rs causes security_tests to fail"
///
/// Interventional test:
///   1. Create a workspace snapshot (git stash or worktree)
///   2. Apply the change to auth.rs
///   3. Run security_tests
///   4. If they fail: hypothesis supported
///   5. If they pass: hypothesis not supported (the failure was confounded)
pub async fn test_coding_hypothesis(
    hypothesis: &CodingCausalHypothesis,
    workspace: &Workspace,
) -> InterventionResult {
    let snapshot = workspace.create_snapshot().await?;

    // Apply the change (intervention)
    workspace.apply_change(&hypothesis.change).await?;

    // Run the tests (observe effect)
    let test_result = workspace.run_tests(&hypothesis.affected_tests).await?;

    // Restore snapshot
    workspace.restore_snapshot(&snapshot).await?;

    InterventionResult {
        intervention: hypothesis.change.clone(),
        observed_effect: test_result.pass_rate,
        predicted_effect: hypothesis.predicted_pass_rate,
        supported: (test_result.pass_rate - hypothesis.predicted_pass_rate).abs() < 0.1,
    }
}
```

---

## Backdoor criterion — Controlling for confounders

```rust
/// The backdoor criterion (Pearl, 2009).
///
/// A set Z satisfies the backdoor criterion relative to (X, Y) if:
/// 1. No node in Z is a descendant of X
/// 2. Z blocks every path between X and Y that contains an arrow INTO X
///
/// If Z satisfies the backdoor criterion, the causal effect of X on Y
/// can be computed from observational data:
///
/// P(Y | do(X)) = Σ_z P(Y | X, Z=z) × P(Z=z)
///
/// This is the "adjustment formula" — it converts observational
/// probabilities into interventional ones without running experiments.
pub fn backdoor_adjustment(
    graph: &CausalGraph,
    x: VariableId,
    y: VariableId,
    z: &[VariableId],
    data: &DataFrame,
) -> Option<f64> {
    // Verify backdoor criterion
    if !graph.satisfies_backdoor(x, y, z) {
        return None;
    }

    // Compute adjustment formula
    let mut causal_effect = 0.0;
    for z_values in data.unique_values(z) {
        let p_y_given_xz = data.conditional_probability(y, &[(x, 1.0)], z, &z_values);
        let p_z = data.marginal_probability(z, &z_values);
        causal_effect += p_y_given_xz * p_z;
    }

    Some(causal_effect)
}
```

---

## Dream-based counterfactual discovery

During REM Dreams, the agent generates counterfactual scenarios using the causal model:

```rust
/// Counterfactual generation during REM phase.
///
/// "What would have happened if X had been different?"
///
/// Pearl's three-step counterfactual:
/// 1. Abduction: Use evidence to determine exogenous variables U
/// 2. Action: Modify the SCM with do(X = x')
/// 3. Prediction: Propagate through modified model
pub fn generate_counterfactual(
    scm: &StructuralCausalModel,
    evidence: &HashMap<VariableId, f64>,
    intervention: (VariableId, f64),
) -> HashMap<VariableId, f64> {
    // Step 1: Abduction — infer exogenous variables from evidence
    let exogenous = scm.abduct(evidence);

    // Step 2: Action — apply intervention to modified model
    let modified = scm.intervene(intervention.0, intervention.1);

    // Step 3: Prediction — propagate with inferred exogenous values
    modified.propagate_with_exogenous(&exogenous)
}
```

Counterfactual discovery is unique to Level 3 of Pearl's hierarchy. No competitor agent framework operates at this level. Combined with HDC encoding, discovered causal relationships are stored as `CausalLink` knowledge entries in the Neuro subsystem:

```rust
// Store discovered causal link
neuro.store(KnowledgeEntry {
    kind: KnowledgeType::CausalLink,
    content: format!(
        "Causal link discovered: {} → {} (effect size: {:.3}, confidence: {:.2})",
        cause_name, effect_name, effect_size, confidence
    ),
    hdc_vector: HdcVector::bind(&cause_hv, &effect_hv),  // HDC encoding
    confidence,
    tier: KnowledgeTier::Working,
    ..Default::default()
}).await?;
```

---

## Implementation details

### PC algorithm: conditional independence test

The PC algorithm uses the **partial correlation test** as its conditional independence test. For continuous variables X, Y conditioned on set Z:

```rust
/// Conditional independence test via partial correlation.
///
/// Tests H0: X ⊥ Y | Z (X is independent of Y given Z).
///
/// Method: Compute partial correlation r_{XY|Z} from the
/// correlation matrix using recursive formula (Baba et al., 2004).
/// Convert to a test statistic via Fisher's z-transform:
///   z = 0.5 * ln((1+r)/(1-r)) * sqrt(n - |Z| - 3)
///
/// Under H0, z ~ N(0,1). Reject H0 if |z| > z_{alpha/2}.
pub fn conditional_independence_test(
    data: &DataFrame,
    x: VariableId,
    y: VariableId,
    z: &[VariableId],
    alpha: f64,
) -> bool {
    let n = data.n_rows();
    let r_xy_z = partial_correlation(data, x, y, z);
    let z_stat = 0.5 * ((1.0 + r_xy_z) / (1.0 - r_xy_z)).ln()
        * ((n as f64 - z.len() as f64 - 3.0).max(1.0)).sqrt();
    let critical = normal_quantile(1.0 - alpha / 2.0); // two-sided
    z_stat.abs() < critical // true = independent
}
```

### Significance level adaptation

The significance level `alpha` adapts based on the number of variables and available data:

| n_variables | n_observations | Recommended alpha |
|---|---|---|
| < 10 | > 1000 | 0.05 (standard) |
| 10 - 50 | > 1000 | 0.01 (Bonferroni-style correction) |
| > 50 | > 1000 | 0.001 |
| any | < 100 | 0.10 (relaxed, low power) |

```rust
/// Adapt alpha based on problem scale.
///
/// Uses Bonferroni-like correction: alpha_adj = base_alpha / n_tests_estimate.
/// The n_tests estimate is O(p^2) where p = number of variables.
pub fn adaptive_alpha(n_variables: usize, n_observations: usize, base_alpha: f64) -> f64 {
    let n_tests = n_variables * (n_variables - 1) / 2; // upper bound on pairwise tests
    let bonferroni = base_alpha / n_tests.max(1) as f64;
    // Floor at 1e-6 to prevent test from never rejecting
    let alpha = bonferroni.max(1e-6);
    // Relax if sample size is too small for the correction
    if n_observations < 10 * n_variables {
        (alpha * 10.0).min(0.1)
    } else {
        alpha
    }
}
```

### Maximum conditioning set formula

The PC algorithm tests conditional independence with conditioning sets of increasing size. The maximum conditioning set size controls the computational cost:

```
max_conditioning_set = min(
    max_neighbors - 1,         // can't condition on more than the neighbor count
    floor(log2(n_observations)) - 1,  // statistical power limit
    user_max                   // user override (default: 5)
)
```

**Rationale**: Conditioning on k variables requires estimating a (k+2)-dimensional distribution. With n observations, you need roughly `2^(k+2)` samples for reliable estimation. So `k < log2(n) - 2` is the practical limit.

### Meek's orientation rules

After v-structure orientation, Meek's four rules orient remaining undirected edges:

```
Rule 1 (Acyclicity): If X → Y — Z and X and Z are not adjacent, orient Y → Z.
Rule 2 (Directed path): If X → Y → Z and X — Z, orient X → Z.
Rule 3 (Two directed paths): If X — Y, X → Z, X → W, Y — Z, Y — W,
        and Z and W are not adjacent, orient X → Y.
Rule 4 (Transitive closure): If X — Y, Y → Z, X — Z, orient X → Z.
```

Apply rules repeatedly until no new orientations are produced. This converges in at most O(p^2) iterations where p is the number of variables.

### Granger causality extensions

#### Time alignment for cross-protocol tests

DeFi protocols produce observations at different cadences. Before running Granger tests, time-align the series:

```rust
/// Align two time series to a common time grid.
///
/// Method: snap each observation to the nearest grid point.
/// Grid spacing = max(cadence_x, cadence_y).
/// Missing values are forward-filled (last observation carried forward).
pub fn time_align(
    x: &TimeSeries,
    y: &TimeSeries,
    grid_spacing: Duration,
) -> (Vec<f64>, Vec<f64>) {
    let start = x.start().min(y.start());
    let end = x.end().max(y.end());
    let n_points = ((end - start).as_secs_f64() / grid_spacing.as_secs_f64()).ceil() as usize;

    let mut aligned_x = Vec::with_capacity(n_points);
    let mut aligned_y = Vec::with_capacity(n_points);

    for i in 0..n_points {
        let t = start + grid_spacing * i as u32;
        aligned_x.push(x.value_at_or_before(t).unwrap_or(f64::NAN));
        aligned_y.push(y.value_at_or_before(t).unwrap_or(f64::NAN));
    }

    (aligned_x, aligned_y)
}
```

#### MEV label generation

The MEV-adjusted Granger test requires labels identifying which transactions are MEV-related:

```rust
/// Heuristic MEV labeling for transaction sequences.
///
/// Labels a transaction as MEV if any of:
/// 1. It is part of a sandwich bundle (buy-victim-sell in same block).
/// 2. It is a backrun (immediately follows a large swap in the same block).
/// 3. It interacts with a known MEV relay (Flashbots, MEV-Boost builder).
/// 4. Its gas price is >3x the block median (priority fee bidding).
pub fn label_mev_transactions(txs: &[Transaction], block: &Block) -> Vec<bool> {
    let median_gas = median_gas_price(txs);
    txs.iter().map(|tx| {
        is_sandwich_component(tx, txs)
            || is_backrun(tx, txs)
            || is_known_mev_relay(&tx.from)
            || tx.gas_price > median_gas * 3.0
    }).collect()
}
```

#### Bridge latency model for multi-chain tests

Cross-chain Granger tests must account for message propagation delay:

| Bridge type | Typical latency | Model |
|---|---|---|
| Native bridge (L1 -> L2) | 1 - 15 minutes | Fixed lag = 10 minutes |
| Third-party bridge (LayerZero, Wormhole) | 2 - 30 minutes | Fixed lag = 15 minutes |
| Optimistic rollup -> L1 | 7 days (dispute period) | Fixed lag = 7 days |
| ZK rollup -> L1 | 1 - 4 hours (proof generation) | Fixed lag = 2 hours |

The Granger test lag order `max_lag` is set to `ceil(bridge_latency / grid_spacing) + 2` to account for the bridge latency plus a buffer.

### Do-operator on code: supported change formats

The coding-domain do-operator supports these intervention types:

```rust
/// Supported code intervention formats for causal experiments.
pub enum CodeIntervention {
    /// Modify a function body.
    FunctionBody {
        file: PathBuf,
        function_name: String,
        new_body: String,
    },
    /// Add/remove a dependency.
    DependencyChange {
        crate_name: String,
        action: DepAction, // Add, Remove, ChangeVersion
    },
    /// Modify a configuration value.
    ConfigChange {
        key: String,
        old_value: String,
        new_value: String,
    },
    /// Apply a diff patch.
    Patch {
        diff: String, // unified diff format
    },
}

/// Observable timing model for coding experiments.
///
/// After applying a code change, measure these observables:
pub struct CodingObservables {
    pub compile_time_ms: u64,
    pub test_pass_rate: f64,
    pub test_duration_ms: u64,
    pub clippy_warning_count: usize,
    pub binary_size_bytes: u64,
}
```

**Snapshot/restore**: Uses `git stash` for lightweight snapshots. For heavier experiments (dependency changes), creates a temporary git worktree. Restore is `git stash pop` or worktree deletion.

### Backdoor adjustment: handling high-cardinality Z

When the conditioning set Z contains high-cardinality variables (many unique values), direct enumeration of Z values is infeasible. Two mitigation strategies:

1. **Binning**: For continuous Z variables, bin into quantiles (default: 10 bins). This trades precision for tractability.

2. **Propensity score**: Replace Z with a 1-dimensional propensity score `e(Z) = P(X=1|Z)`. The backdoor adjustment becomes `sum over e(Z) bins of P(Y|X, e(Z)) * P(e(Z))`.

```rust
/// Backdoor adjustment with propensity score dimensionality reduction.
///
/// When |Z| > max_cardinality, collapses Z into a propensity score.
pub fn backdoor_adjustment_propensity(
    graph: &CausalGraph,
    x: VariableId,
    y: VariableId,
    z: &[VariableId],
    data: &DataFrame,
    max_cardinality: usize,  // default: 100
    n_bins: usize,           // default: 10
) -> Option<f64> {
    if !graph.satisfies_backdoor(x, y, z) {
        return None;
    }

    let z_cardinality: usize = z.iter()
        .map(|v| data.n_unique(v))
        .product();

    if z_cardinality <= max_cardinality {
        // Direct adjustment
        return backdoor_adjustment(graph, x, y, z, data);
    }

    // Propensity score collapse
    let propensity = logistic_regression(data, x, z);
    let binned_propensity = quantile_bin(&propensity, n_bins);
    backdoor_adjustment(graph, x, y, &[binned_propensity], data)
}
```

### Adequacy detection for backdoor sets

Automatically find a valid backdoor set (if one exists):

```rust
/// Find a minimal valid backdoor adjustment set.
///
/// Algorithm: start with all non-descendants of X. Test the backdoor
/// criterion. If valid, greedily remove variables while maintaining
/// validity. Returns None if no valid backdoor set exists.
pub fn find_backdoor_set(
    graph: &CausalGraph,
    x: VariableId,
    y: VariableId,
) -> Option<Vec<VariableId>> {
    let descendants_x = graph.descendants(x);
    let candidates: Vec<VariableId> = graph.all_variables()
        .filter(|v| *v != x && *v != y && !descendants_x.contains(v))
        .collect();

    // Start with full candidate set and prune
    let mut z = candidates.clone();
    if !graph.satisfies_backdoor(x, y, &z) {
        return None; // no valid backdoor set exists
    }

    // Greedy minimization
    for candidate in &candidates {
        let reduced: Vec<_> = z.iter().filter(|v| *v != candidate).cloned().collect();
        if graph.satisfies_backdoor(x, y, &reduced) {
            z = reduced;
        }
    }

    Some(z)
}
```

### Intervention hypothesis confidence threshold

Hypotheses are accepted when the observed effect matches the predicted effect within a tolerance:

```
|observed_effect - predicted_effect| < tolerance

tolerance = max(0.1, 0.2 * |predicted_effect|)
```

The 0.1 absolute floor prevents rejection of hypotheses with small predicted effects due to noise. The 0.2 relative component accounts for model imprecision scaling with effect size.

Confidence updates after each test:

```
if supported:
    confidence = confidence + 0.1 * (1.0 - confidence)   // diminishing increase
if not supported:
    confidence = confidence * 0.7                         // 30% penalty
```

A hypothesis is promoted to "confirmed" at confidence >= 0.8 (requires ~5 supporting tests from a neutral prior of 0.5). It is demoted to "rejected" at confidence < 0.1 (requires ~3 consecutive failures from 0.5).

### Test criteria

- **PC algorithm on known DAG**: Given data generated from X -> Y -> Z, the PC algorithm recovers the correct structure.
- **Conditional independence calibration**: On independent data, the test rejects at rate <= alpha.
- **Granger test on known causal series**: When X(t) = X(t-1) + noise, Y(t) = 0.5*X(t-1) + Y(t-1) + noise, the test detects X -> Y.
- **Do-operator correctness**: In a confounded model (X <- Z -> Y, X -> Y), `do(X)` gives a different result than conditioning on X.
- **Backdoor adjustment**: On synthetic data with known causal effect, the adjusted estimate is within 10% of the true effect.
- **Coding intervention round-trip**: After applying and restoring a CodeIntervention, the workspace is in its original state.
- **MEV label accuracy**: On a set of labeled Flashbots bundles, the heuristic achieves >90% recall.

---

## Continuous Optimization for DAG Learning

The PC algorithm and Granger causality above are constraint-based methods: they use statistical tests to prune edges from a candidate graph. A fundamentally different approach reformulates DAG structure learning as a continuous optimization problem. Instead of testing conditional independence for each pair of variables, these methods optimize a score function over the space of weighted adjacency matrices, subject to a differentiable acyclicity constraint.

This is a significant shift. The combinatorial search over DAG structures is NP-hard (the number of DAGs on d nodes is super-exponential). Continuous relaxation converts this into a smooth optimization problem solvable with gradient descent, at the cost of requiring a tractable acyclicity characterization.

### NOTEARS -- Continuous Acyclicity Constraint

The breakthrough insight of NOTEARS (Zheng et al., 2018) is that acyclicity can be expressed as a smooth equality constraint on the weighted adjacency matrix, eliminating the need for combinatorial search entirely.

```rust
/// NOTEARS: Non-combinatorial Optimization via Trace Exponential
/// and Augmented lagRangian for Structure learning.
///
/// Key insight (Zheng et al., 2018, NeurIPS):
/// A weighted adjacency matrix W encodes a DAG if and only if:
///   h(W) = tr(e^{W ∘ W}) - d = 0
///
/// where ∘ is element-wise (Hadamard) product and d = number of variables.
/// This converts the NP-hard combinatorial DAG constraint into a smooth,
/// differentiable equality constraint solvable via augmented Lagrangian.
///
/// Complexity: O(d³) per iteration (matrix exponential) vs exponential for PC.
/// Recent improvement SDCD (Nazaret et al., 2024, ICML) replaces trace-exponential
/// with spectral constraint h(W) = λ_max(|W|) < 1, which is numerically
/// more stable and scales to thousands of variables.
pub struct NotearsSolver {
    /// Maximum number of augmented Lagrangian iterations.
    pub max_outer_iter: usize,      // default: 10
    /// Maximum inner optimization iterations per outer step.
    pub max_inner_iter: usize,      // default: 100
    /// Augmented Lagrangian penalty parameter.
    pub rho: f64,                   // default: 1.0, doubles each outer iter
    /// Lagrangian multiplier growth factor.
    pub rho_max: f64,               // default: 1e16
    /// Convergence tolerance for acyclicity constraint.
    pub h_tol: f64,                 // default: 1e-8
    /// L1 regularization for sparsity.
    pub lambda_l1: f64,             // default: 0.1
    /// Acyclicity constraint type.
    pub acyclicity: AcyclicityConstraint,
}

pub enum AcyclicityConstraint {
    /// Original NOTEARS: h(W) = tr(e^{W∘W}) - d = 0.
    /// Numerically unstable for large d (matrix exponential overflow).
    TraceExponential,
    /// SDCD spectral constraint: h(W) = λ_max(|W|).
    /// (Nazaret et al., 2024, ICML 2024)
    /// Numerically stable, differentiable via eigenvector gradients.
    /// Scales to thousands of variables.
    Spectral,
    /// DAGMA log-determinant: h(W) = -log det(sI - W∘W) + d·log(s).
    /// (Bello et al., 2022) Avoids matrix exponential entirely.
    LogDeterminant { s: f64 },  // default s: 1.0
}
```

The optimization proceeds via augmented Lagrangian method. The unconstrained subproblem at each outer iteration is:

```
min_W  F(W) + alpha * h(W) + (rho / 2) * h(W)^2

where:
  F(W)   = (1/2n) ||X - XW||_F^2 + lambda * ||W||_1   (penalized least squares)
  h(W)   = tr(e^{W∘W}) - d                              (acyclicity constraint)
  alpha  = Lagrange multiplier (updated each outer iter)
  rho    = penalty parameter (doubled each outer iter)
```

The inner optimization uses L-BFGS (limited-memory BFGS) since both F and h have closed-form gradients. The gradient of h with respect to W is:

```
∇h(W) = (e^{W∘W})^T ∘ 2W
```

This is computable in O(d^3) via the matrix exponential. The augmented Lagrangian doubles rho each outer iteration until h(W) < h_tol, guaranteeing convergence to a DAG.

**Limitation**: The trace-exponential h(W) = tr(e^{W∘W}) - d suffers from numerical overflow when d > 200. The matrix exponential produces entries of magnitude e^{d}, which exceeds float64 range. This motivated both the DAGMA and SDCD improvements below.

### DAG-GNN -- Neural Causal Discovery

Where NOTEARS assumes linear structural equations (Y = WX + noise), DAG-GNN extends continuous DAG learning to nonlinear relationships using graph neural networks.

```rust
/// DAG-GNN: Structure learning via Graph Neural Networks.
///
/// Yu et al. (2019, ICML): Uses a variational autoencoder with GNN
/// encoder/decoder to learn the DAG structure alongside functional
/// relationships. The adjacency matrix is treated as a learnable
/// parameter, with the acyclicity constraint integrated into the loss.
///
/// Advantages over PC/NOTEARS:
/// - Captures nonlinear causal relationships via neural expressiveness
/// - Handles mixed variable types (continuous + discrete)
/// - End-to-end differentiable (gradient-based optimization)
pub struct DagGnnConfig {
    /// GNN encoder hidden dimension.
    pub encoder_hidden: usize,     // default: 64
    /// Number of GNN message-passing layers.
    pub n_layers: usize,            // default: 2
    /// VAE latent dimension.
    pub latent_dim: usize,          // default: 16
    /// Edge existence temperature (Gumbel-Softmax for discrete edges).
    pub temperature: f64,           // default: 0.5
    /// KL divergence weight in ELBO.
    pub kl_weight: f64,             // default: 1.0
    /// Acyclicity penalty weight (grows during training).
    pub acyclicity_weight: f64,     // default: 1.0
}
```

The architecture has two components:

1. **Encoder**: A GNN that maps observed variables X to a latent representation Z. The adjacency matrix A is a learnable parameter that defines the message-passing structure. Edges are sampled via Gumbel-Softmax (Jang et al., 2017) to maintain differentiability while producing discrete edge decisions.

2. **Decoder**: Another GNN that reconstructs X from Z using the same adjacency matrix A. The reconstruction loss trains the model to learn both the graph structure (A) and the functional relationships (GNN weights).

The loss function combines reconstruction, KL divergence, and acyclicity:

```
L = -ELBO + acyclicity_weight * h(A)
  = E_q[log p(X|Z,A)] - kl_weight * KL(q(Z|X,A) || p(Z)) + acyclicity_weight * h(A)
```

The acyclicity term h(A) uses the same trace-exponential constraint as NOTEARS. During training, `acyclicity_weight` increases on a schedule (typically doubling every 100 epochs) to gradually enforce the DAG constraint, allowing the model to first learn approximate relationships before being forced into acyclicity.

**Trade-off**: DAG-GNN captures nonlinear relationships that NOTEARS misses, but requires substantially more data (thousands of samples vs. hundreds for NOTEARS) and is sensitive to hyperparameters (temperature, KL weight, training schedule). For the linear case, NOTEARS is preferred.

### SDCD -- Stable Differentiable Causal Discovery

SDCD (Nazaret et al., 2024) addresses the core numerical instability of NOTEARS via a two-stage approach that separates edge pruning from DAG enforcement.

```rust
/// SDCD: Two-stage stable causal discovery.
///
/// Nazaret et al. (2024, ICML 2024, PMLR 235:37413-37445)
/// Addresses numerical instability in NOTEARS via two-stage optimization:
///
/// Stage 1 (Pruning): Optimize edge weights WITHOUT acyclicity constraint.
///   Uses L1 regularization to identify likely edges.
///   Much faster — no expensive matrix exponential.
///
/// Stage 2 (DAG Learning): Apply spectral acyclicity constraint
///   h(A) = λ_max(|A|) on the pruned graph.
///   Spectral constraint: gradient = right_eigvec · left_eigvec^T.
///   10-100x faster convergence than NOTEARS.
pub struct SdcdSolver {
    /// Stage 1: edge pruning parameters.
    pub pruning_l1_weight: f64,     // default: 0.1
    pub pruning_epochs: usize,      // default: 100
    pub pruning_threshold: f64,     // default: 0.01 (edges below this removed)
    /// Stage 2: DAG learning parameters.
    pub dag_learning_rate: f64,     // default: 1e-3
    pub dag_epochs: usize,          // default: 200
    /// Spectral constraint gradient method.
    pub eigvec_method: EigvecMethod,
}

pub enum EigvecMethod {
    /// Full eigendecomposition (via LAPACK dsyev).
    Full,
    /// Power iteration (faster for single dominant eigenvalue).
    PowerIteration { max_iter: usize, tol: f64 },
    /// Lanczos (for sparse adjacency matrices).
    Lanczos { n_krylov: usize },
}
```

The key innovation is the **spectral acyclicity constraint**. Instead of h(W) = tr(e^{W∘W}) - d, SDCD uses:

```
h(W) = lambda_max(|W|)

where lambda_max is the largest eigenvalue of the element-wise absolute value of W.
A matrix W encodes a DAG if and only if lambda_max(|W|) < 1.
```

This constraint is numerically stable (eigenvalues are bounded, no exponential blowup) and its gradient is computed via the eigenvector:

```
∇h(W) = sign(W) ∘ (v_right · v_left^T)

where v_right, v_left are the right and left eigenvectors
corresponding to lambda_max(|W|).
```

The two-stage approach provides additional speedup. Stage 1 runs unconstrained L1-penalized optimization to identify the sparse set of candidate edges. Stage 2 operates only on this pruned graph, which is typically much smaller than the full d x d adjacency matrix. On benchmarks, SDCD achieves 10-100x faster convergence than NOTEARS while producing equal or better structural accuracy.

### DAGMA -- Log-Determinant Acyclicity

DAGMA (Bello et al., 2022) provides a third acyclicity characterization that avoids both the matrix exponential (NOTEARS) and eigenvalue computation (SDCD):

```rust
/// DAGMA: DAG learning via M-matrices and log-determinant.
///
/// Bello et al. (2022, NeurIPS): Uses M-matrix theory to characterize
/// DAGs via log-determinant:
///
///   h(W) = -log det(sI - W∘W) + d·log(s)
///
/// where s > 0 is a hyperparameter (default: 1.0).
/// h(W) = 0 if and only if W encodes a DAG.
///
/// Advantages:
/// - No matrix exponential (avoids NOTEARS overflow)
/// - Gradient is (sI - W∘W)^{-1}, a matrix inverse (O(d³), stable)
/// - The log-det is a barrier function: it goes to +infinity as W
///   approaches a cycle, preventing the optimizer from crossing into
///   cyclic territory. This self-correcting property eliminates the
///   need for the augmented Lagrangian outer loop.
pub struct DagmaSolver {
    /// Hyperparameter s for the log-det constraint.
    /// Larger s makes the constraint more permissive initially.
    pub s: f64,                     // default: 1.0
    /// Learning rate for gradient descent.
    pub learning_rate: f64,         // default: 3e-2
    /// Maximum optimization iterations.
    pub max_iter: usize,            // default: 5000
    /// L1 regularization for sparsity.
    pub lambda_l1: f64,             // default: 0.02
    /// Convergence tolerance.
    pub tol: f64,                   // default: 1e-6
    /// Schedule for decreasing s (annealing toward strict acyclicity).
    pub s_schedule: Vec<f64>,       // default: [1.0, 0.9, 0.8, 0.7]
}
```

The gradient of the DAGMA constraint has a particularly clean form:

```
∇h(W) = -2W ∘ (sI - W∘W)^{-1}
```

This requires only a matrix inverse, which is O(d^3) and numerically stable via LU decomposition. Compared to NOTEARS (matrix exponential, overflow-prone) and SDCD (eigendecomposition, requires iterative methods for large d), the matrix inverse is the most numerically well-conditioned operation of the three.

The s-annealing schedule starts with a permissive constraint (large s) that allows the optimizer to explore the space of weighted graphs, then gradually tightens (decreasing s) to enforce strict acyclicity. This eliminates the augmented Lagrangian outer loop entirely, simplifying the optimization to a single-level problem.

### Comparison of acyclicity constraints

| Method | Constraint h(W) | Gradient cost | Numerical stability | Scales to |
|---|---|---|---|---|
| NOTEARS (2018) | tr(e^{W∘W}) - d | O(d³) matrix exp | Poor (overflow at d>200) | ~200 variables |
| DAGMA (2022) | -log det(sI - W∘W) + d·log(s) | O(d³) matrix inverse | Good (LU decomposition) | ~500 variables |
| SDCD (2024) | lambda_max(\|W\|) | O(d²) power iteration | Good (bounded eigenvalues) | ~2000 variables |
| DAG-GNN (2019) | tr(e^{A∘A}) - d (on learned A) | O(d³) + backprop | Poor (same as NOTEARS) | ~100 variables (GPU) |

### Critical analysis: known limitations

Ng, Ghassami, & Zhang (2024, CLR 2024) provide a sobering empirical analysis of continuous optimization methods for DAG learning. Key findings relevant to Roko:

1. **Thresholding sensitivity**: All continuous methods produce dense weighted matrices that require post-hoc thresholding to obtain a DAG. The choice of threshold dramatically affects the recovered structure. A threshold too low retains spurious edges; too high removes genuine ones.

2. **Nonlinear settings**: NOTEARS (linear) and DAGMA (linear) degrade significantly on nonlinear data. DAG-GNN handles nonlinearity but at much higher sample and computational cost. For Roko's domains (code dependency graphs, protocol interactions), relationships are often nonlinear.

3. **Equal variance assumption**: NOTEARS assumes equal noise variance across variables. Violation of this assumption (common in real data) biases edge orientation. SDCD partially addresses this via its two-stage approach.

4. **Faithfulness violations**: All continuous methods assume faithfulness (no perfect cancellation of causal effects). In engineered systems like software, faithfulness violations are common (e.g., two bugs that cancel each other's effects).

Zhou, Wang, et al. (2025, NeurIPS 2025) introduce differentiable constraint-based methods that combine the statistical rigor of PC-style independence testing with the scalability of continuous optimization, partially addressing limitation (4).

### Integration with Existing Causal Discovery

Roko combines constraint-based discovery (PC algorithm, described above) with continuous optimization (SDCD) in a hybrid pipeline that leverages the strengths of both approaches.

```rust
/// Hybrid causal discovery pipeline for Roko.
///
/// Combines constraint-based (PC) with continuous optimization (SDCD):
///
/// 1. PC algorithm for skeleton discovery (fast, handles small p)
/// 2. SDCD for edge weight estimation on the PC skeleton
/// 3. Interventional validation via mirage-rs (ground truth)
/// 4. Dream-based counterfactual refinement
///
/// This hybrid (PC-NOTEARS; Kraskov et al., 2024, Bioinformatics)
/// achieves the best aggregate performance across structural
/// and effect size metrics.
pub struct HybridCausalDiscovery {
    /// Phase 1: constraint-based skeleton.
    pub pc_config: PcAlgorithmConfig,
    /// Phase 2: continuous optimization on skeleton.
    pub sdcd_config: SdcdSolver,
    /// Phase 3: interventional validation.
    pub intervention_config: InterventionalDiscovery,
    /// Minimum edge weight to retain in final graph.
    pub final_threshold: f64,       // default: 0.05
}
```

The hybrid approach works in four phases:

**Phase 1 (PC skeleton)**: Run the PC algorithm to discover the undirected skeleton. This is fast for small variable counts (d < 50) and produces a sparse graph by removing conditionally independent pairs. The skeleton serves as a mask for the continuous optimization -- SDCD only needs to estimate weights for edges that survive PC's independence tests.

**Phase 2 (SDCD weight estimation)**: Run SDCD on the PC skeleton. Stage 1 (pruning) is skipped since PC already performed edge pruning. Stage 2 applies the spectral acyclicity constraint to orient edges and estimate weights. Operating on the PC skeleton rather than the full d x d matrix dramatically reduces the search space.

**Phase 3 (Interventional validation)**: For high-confidence edges (weight > final_threshold), run interventional experiments via mirage-rs (chain domain) or workspace snapshots (coding domain) to confirm causal direction and measure effect size. This moves from Level 1 (association) to Level 2 (intervention) of Pearl's hierarchy.

**Phase 4 (Counterfactual refinement)**: During REM Dreams, generate counterfactual scenarios from the validated causal model. Counterfactuals that match historical data increase confidence; those that diverge trigger re-examination of the causal structure.

The hybrid pipeline addresses the thresholding sensitivity problem (limitation 1 from Ng et al.) by using PC's independence tests as a principled pruning criterion rather than relying on arbitrary weight thresholds. It addresses the faithfulness problem (limitation 4) by validating with interventional experiments rather than relying solely on observational statistics.

### Domain-specific considerations

**Code dependency graphs**: Function call graphs and import dependencies provide a known partial DAG structure. The hybrid pipeline can incorporate this as a structural prior, constraining SDCD to only discover edges consistent with the known dependency structure. For example, if module A does not import module B, the optimizer is constrained to set W[A,B] = 0.

**Protocol interaction graphs**: Cross-protocol causal relationships (e.g., Uniswap price changes causing Aave liquidations) operate on different time scales. The hybrid pipeline runs separate PC+SDCD passes at each time scale, then merges the results. Edges that appear at multiple time scales receive higher confidence.

### Test criteria for continuous optimization

- **NOTEARS acyclicity**: After optimization with h_tol=1e-8, the acyclicity constraint satisfies h(W) < 1e-8. Verify on random graphs with d=10, 20, 50.
- **SDCD spectral convergence**: After stage 2, lambda_max(|W|) < 1.0 on all test instances. Verify that the spectral radius monotonically decreases during optimization.
- **Known DAG recovery**: On synthetic data generated from a known DAG X -> Y -> Z with linear structural equations, each solver (NOTEARS, SDCD, DAGMA) recovers the correct structure with edge weights within 10% of ground truth.
- **Hybrid consistency**: The PC skeleton is a supergraph of the final SDCD result. That is, every edge in the SDCD output also appears in the PC skeleton. If this invariant is violated, the SDCD solver introduced a spurious edge outside the PC mask.
- **Nonlinear recovery (DAG-GNN)**: On data generated from Y = sin(X) + noise, Z = X^2 + noise, DAG-GNN recovers X -> Y and X -> Z while NOTEARS (linear) fails to orient X -> Z correctly.
- **Numerical stability**: SDCD and DAGMA complete without NaN or Inf on graphs with d=500. NOTEARS is expected to overflow and is excluded from this test.
- **Threshold sensitivity**: For each method, vary the post-optimization threshold from 0.01 to 0.5 and measure structural Hamming distance (SHD). Report the threshold range where SHD < 5 for the ground-truth graph.

### Citations for continuous optimization methods

- Zheng, X., Aragam, B., Ravikumar, P., & Xing, E. P. (2018). "DAGs with NO TEARS: Continuous Optimization for Structure Learning." *NeurIPS 2018*. -- Original continuous acyclicity constraint via trace exponential.
- Yu, Y., Chen, J., Gao, T., & Yu, M. (2019). "DAG-GNN: DAG Structure Learning with Graph Neural Networks." *ICML 2019*. -- Neural causal discovery with VAE + GNN architecture.
- Bello, K., Aragam, B., & Ravikumar, P. (2022). "DAGMA: Learning DAGs via M-matrices and a Log-Determinant Acyclicity Characterization." *NeurIPS 2022*. -- Log-determinant acyclicity, eliminates augmented Lagrangian.
- Nazaret, A., Hoffman, M., et al. (2024). "Stable Differentiable Causal Discovery." *ICML 2024*, PMLR 235:37413-37445. -- Two-stage SDCD with spectral acyclicity constraint.
- Ng, I., Ghassami, A., & Zhang, K. (2024). "Structure Learning with Continuous Optimization: A Sober Look." *CLR 2024*. -- Critical empirical analysis of continuous DAG learning methods.
- Zhou, J., Wang, M., et al. (2025). "Differentiable Constraint-Based Causal Discovery." *NeurIPS 2025*. -- Hybrid differentiable + constraint-based approach.

---

## Academic foundations

- Pearl, J. (2009). *Causality: Models, Reasoning, and Inference*. 2nd ed. Cambridge University Press. — SCM formalism, do-calculus, backdoor criterion.
- Spirtes, P., Glymour, C., & Scheines, R. (2000). *Causation, Prediction, and Search*. 2nd ed. MIT Press. — PC algorithm.
- Granger, C. W. J. (1969). "Investigating Causal Relations by Econometric Models and Cross-spectral Methods." *Econometrica*, 37(3), 424-438. — Granger causality.
- Pearl, J. (2019). "The seven tools of causal inference." *Communications of the ACM*, 62(3), 54-60. — Accessible overview of the causal hierarchy.
- Peters, J., Janzing, D., & Schölkopf, B. (2017). *Elements of Causal Inference*. MIT Press. — Modern causal discovery algorithms.

---

## Cross-References

- See [01-oracle-trait.md](./01-oracle-trait.md) for how causal models feed oracle predictions
- See [02-chain-oracles.md](./02-chain-oracles.md) for mirage-rs simulation integration
- See [03-coding-oracles.md](./03-coding-oracles.md) for coding-domain causal discovery
- See [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) for topological constraints on causal graphs
- See [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for how causal models inform active inference


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/10-predictive-geometry-and-resonant-patterns.md

# Predictive Geometry and Resonant Pattern Ecosystems

> Topological Data Analysis (TDA) extracts shape from time series. Persistence landscapes provide a Banach space for pattern comparison. Resonant patterns are living organisms with HDC genomes that compete for attention via VCG auction.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [06-hyperdimensional-ta](./06-hyperdimensional-ta.md) for HDC encoding, [08-adaptive-signal-metabolism](./08-adaptive-signal-metabolism.md) for evolutionary dynamics
**Key sources**: `bardo-backup/prd/23-ta/05-predictive-geometry.md`, `bardo-backup/prd/23-ta/06-resonant-pattern-ecosystem.md`

---

## Part I: Predictive Geometry via TDA

### Why topology for time series

Standard TA reduces time series to statistics (means, variances, correlations). These statistics capture numerical properties but miss **shape** — the topological structure of the data. A time series with two peaks and a valley has different topology than one with a gradual rise, even if both have the same mean and variance.

Topological Data Analysis (TDA) extracts shape features that are:

- **Coordinate-free**: invariant to scaling, translation, and monotone transformations
- **Multi-scale**: captures structure at every resolution simultaneously
- **Robust**: small perturbations in data produce small changes in topology
- **Composable**: topological features from different domains can be compared

### Persistence diagrams

A persistence diagram tracks the birth and death of topological features (connected components, loops, voids) across a filtration of the data:

```rust
/// A persistence diagram: the set of (birth, death) pairs for
/// topological features at a given dimension.
///
/// Each point (b, d) represents a feature that appears at scale b
/// and disappears at scale d. Long-lived features (d - b is large)
/// represent genuine structure. Short-lived features are noise.
pub struct PersistenceDiagram {
    /// The topological dimension (0 = components, 1 = loops, 2 = voids).
    pub dimension: usize,

    /// The (birth, death) pairs.
    pub points: Vec<(f64, f64)>,
}

impl PersistenceDiagram {
    /// Compute persistence from a time series using Rips filtration.
    pub fn from_time_series(series: &[f64], dimension: usize) -> Self {
        // Embed time series as point cloud using delay embedding
        // (Takens' theorem guarantees topological equivalence)
        let point_cloud = delay_embedding(series, embedding_dim: 3, delay: 1);

        // Build Rips complex filtration
        let filtration = rips_filtration(&point_cloud, max_scale: f64::MAX);

        // Compute persistent homology
        let diagram = compute_persistence(&filtration, dimension);

        diagram
    }

    /// Lifetime of a feature: death - birth.
    pub fn lifetimes(&self) -> Vec<f64> {
        self.points.iter().map(|(b, d)| d - b).collect()
    }

    /// The persistence of the longest-lived feature.
    pub fn max_persistence(&self) -> f64 {
        self.lifetimes().iter().cloned().fold(0.0, f64::max)
    }
}
```

### Persistence landscapes (Bubenik, 2015)

Persistence landscapes transform persistence diagrams into functions that live in a Banach space — enabling arithmetic operations (addition, subtraction, scaling) on topological features:

```rust
/// A persistence landscape: a sequence of piecewise-linear functions
/// derived from a persistence diagram.
///
/// Bubenik (2015): persistence landscapes form a Banach space,
/// enabling statistical operations (mean, variance, hypothesis testing)
/// on topological features.
///
/// Key property: the landscape is a FUNCTION, not a set of points.
/// Functions can be added, subtracted, scaled, and integrated —
/// operations that are not well-defined on persistence diagrams directly.
pub struct PersistenceLandscape {
    /// The landscape functions λ_k(t) for k = 1, 2, 3, ...
    /// λ_1 is the outermost envelope, λ_2 the next, etc.
    pub layers: Vec<PiecewiseLinearFunction>,

    /// The dimension of the underlying persistence diagram.
    pub dimension: usize,
}

pub struct PiecewiseLinearFunction {
    /// Breakpoints (t_i, f(t_i)).
    pub points: Vec<(f64, f64)>,
}

impl PersistenceLandscape {
    /// Convert a persistence diagram to a landscape.
    pub fn from_diagram(diagram: &PersistenceDiagram) -> Self {
        let mut tent_functions: Vec<PiecewiseLinearFunction> = diagram.points.iter()
            .map(|(b, d)| {
                let mid = (b + d) / 2.0;
                let height = (d - b) / 2.0;
                PiecewiseLinearFunction {
                    points: vec![(*b, 0.0), (mid, height), (*d, 0.0)],
                }
            })
            .collect();

        // Sort by peak height (descending) to get layers
        tent_functions.sort_by(|a, b| {
            b.max_value().partial_cmp(&a.max_value()).unwrap()
        });

        // Build layers by taking the k-th largest value at each t
        let layers = build_layers(&tent_functions);

        PersistenceLandscape { layers, dimension: diagram.dimension }
    }

    /// Add two landscapes (point-wise).
    pub fn add(&self, other: &PersistenceLandscape) -> PersistenceLandscape {
        // Point-wise addition of corresponding layers
        let layers = self.layers.iter()
            .zip_longest(other.layers.iter())
            .map(|pair| match pair {
                Both(a, b) => a.pointwise_add(b),
                Left(a) => a.clone(),
                Right(b) => b.clone(),
            })
            .collect();

        PersistenceLandscape { layers, dimension: self.dimension }
    }

    /// Scale a landscape by a constant.
    pub fn scale(&self, factor: f64) -> PersistenceLandscape {
        let layers = self.layers.iter()
            .map(|l| l.pointwise_scale(factor))
            .collect();

        PersistenceLandscape { layers, dimension: self.dimension }
    }

    /// L^p norm of the landscape (measure of total topological complexity).
    pub fn lp_norm(&self, p: f64) -> f64 {
        self.layers.iter()
            .map(|l| l.lp_integral(p))
            .sum::<f64>()
            .powf(1.0 / p)
    }
}
```

### Topology-to-trajectory mapping

The key application: use topological features to constrain trajectory predictions:

```rust
/// Map topological features to trajectory forecasts.
///
/// The persistence landscape provides topological constraints
/// on future price/metric trajectories. For example:
///
/// β_0 (component count):
///   - If the current time series has 2 connected components,
///     any predicted trajectory must eventually reduce to 1
///     (convergence) or increase to 3+ (divergence).
///   - This constrains the set of possible futures.
///
/// β_1 (loop count):
///   - If the time series has a persistent 1-cycle (loop),
///     the predicted trajectory should account for periodic behavior.
///
/// The mapping uses kernel regression: given a topological feature,
/// predict the trajectory parameters.
pub struct TopologyToTrajectory {
    /// Trained kernel regression model.
    kernel: KernelRegression,

    /// Historical (topology, trajectory) pairs for training.
    training_data: Vec<(PersistenceLandscape, Vec<f64>)>,
}

impl TopologyToTrajectory {
    /// Predict future trajectory from current topology.
    pub fn predict(&self, current_topology: &PersistenceLandscape) -> TrajectoryPrediction {
        let weights = self.kernel.compute_weights(current_topology);
        let predicted = self.training_data.iter()
            .zip(weights.iter())
            .map(|((_, traj), w)| traj.iter().map(|v| v * w).collect::<Vec<_>>())
            .fold(vec![0.0; self.training_data[0].1.len()], |acc, t| {
                acc.iter().zip(t.iter()).map(|(a, b)| a + b).collect()
            });

        TrajectoryPrediction {
            values: predicted,
            topological_constraints: self.extract_constraints(current_topology),
        }
    }
}

pub struct TrajectoryPrediction {
    /// Predicted future values.
    pub values: Vec<f64>,

    /// Topological constraints on the prediction.
    pub topological_constraints: Vec<TopologicalConstraint>,
}

pub enum TopologicalConstraint {
    /// β_0 constraint: the trajectory must have this many components.
    ComponentCount { expected: usize, tolerance: usize },

    /// β_1 constraint: the trajectory exhibits periodic behavior.
    PeriodicBehavior { period_estimate: f64, confidence: f64 },

    /// Persistence constraint: features with lifetime > threshold
    /// will likely persist in the future.
    FeaturePersistence { min_lifetime: f64, count: usize },
}
```

---

## Part II: Resonant Pattern Ecosystems

### Patterns as organisms

Resonant patterns extend the adaptive signal metabolism framework (see [08-adaptive-signal-metabolism.md](./08-adaptive-signal-metabolism.md)) by treating multi-signal patterns as organisms with HDC genomes:

```rust
/// A resonant pattern: a multi-signal pattern that acts as an organism
/// in the pattern ecosystem.
///
/// Each pattern has:
/// - An HDC "genome" (its encoded structure)
/// - A weight (fitness in the attention economy)
/// - A niche (the environmental conditions where it activates)
/// - A lineage (evolutionary history)
pub struct ResonantPattern {
    /// HDC genome: the pattern's identity vector.
    pub genome: HdcVector,

    /// Weight/fitness in the attention economy.
    pub weight: f64,

    /// The conditions under which this pattern activates.
    /// Encoded as a region in the state space.
    pub niche: PatternNiche,

    /// The signals that compose this pattern.
    pub signals: Vec<SignalId>,

    /// Evolutionary lineage (parent patterns).
    pub lineage: Vec<PatternId>,

    /// Generation counter.
    pub generation: u64,

    /// Historical accuracy when this pattern activated.
    pub accuracy_history: Vec<f64>,

    /// Topological fingerprint (persistence landscape summary).
    pub topo_fingerprint: Option<PersistenceLandscape>,
}

pub struct PatternNiche {
    /// Center of the niche in state space.
    pub center: Vec<f64>,

    /// Radius of activation.
    pub radius: f64,

    /// Niche specificity: narrow (specialist) vs. broad (generalist).
    pub specificity: f64,
}
```

### Reproductive algebra in HDC space

Patterns reproduce by combining parent genomes via HDC operations:

```rust
/// Pattern reproduction: combine two parent patterns into an offspring.
///
/// The reproductive algebra uses HDC operations:
/// - Bundle (majority vote): inherit traits from both parents
/// - Bind (XOR): create new associations
/// - Permute (rotate): shift temporal relationships
///
/// The offspring inherits structure from both parents but is
/// distinct — like biological sexual reproduction.
pub fn reproduce(
    parent_a: &ResonantPattern,
    parent_b: &ResonantPattern,
    mutation_rate: f64,
    rng: &mut impl Rng,
) -> ResonantPattern {
    // Crossover: bundle both genomes (majority vote preserves shared structure)
    let offspring_genome = parent_a.genome.bundle_with(&parent_b.genome);

    // Mutation: XOR with random noise
    let noise = HdcVector::random_with_density(mutation_rate, rng);
    let mutated = offspring_genome.xor(&noise);

    // Niche: interpolate between parent niches
    let niche = PatternNiche {
        center: parent_a.niche.center.iter()
            .zip(parent_b.niche.center.iter())
            .map(|(a, b)| (a + b) / 2.0)
            .collect(),
        radius: (parent_a.niche.radius + parent_b.niche.radius) / 2.0,
        specificity: (parent_a.niche.specificity + parent_b.niche.specificity) / 2.0,
    };

    ResonantPattern {
        genome: mutated,
        weight: 0.01,  // start with minimal weight
        niche,
        signals: merge_signal_sets(&parent_a.signals, &parent_b.signals),
        lineage: vec![parent_a.id(), parent_b.id()],
        generation: parent_a.generation.max(parent_b.generation) + 1,
        accuracy_history: vec![],
        topo_fingerprint: None,
    }
}
```

### VCG auction competition

Patterns compete for the limited attention budget through the VCG auction (Vickrey 1961, Clarke 1971, Groves 1973):

```rust
/// Pattern competition via VCG auction.
///
/// When multiple patterns activate simultaneously, they bid for
/// inclusion in the agent's cognitive context.
///
/// Bid = pattern_weight × niche_match × daimon_urgency
///
/// VCG truthfulness: each winner pays the second-highest bid,
/// preventing bid inflation.
pub fn pattern_auction(
    active_patterns: &[ResonantPattern],
    state: &EngineState,
    budget: usize,
) -> Vec<(PatternId, f64)> {
    let bids: Vec<(PatternId, f64)> = active_patterns.iter()
        .map(|p| {
            let niche_match = p.niche.match_score(state);
            let bid = p.weight * niche_match * state.daimon_urgency();
            (p.id(), bid)
        })
        .collect();

    // Sort by bid, take top `budget` patterns
    let mut sorted = bids.clone();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // VCG payment: each winner pays the second-highest excluded bid
    let winners: Vec<_> = sorted.iter().take(budget).collect();
    winners.iter().map(|(id, bid)| {
        let payment = sorted.get(budget).map(|(_, b)| *b).unwrap_or(0.0);
        (*id, *bid - payment)  // surplus = bid - payment
    }).collect()
}
```

### Lotka-Volterra predator-prey dynamics

Patterns that deplete the same market opportunity (edge) interact via predator-prey dynamics:

```rust
/// Lotka-Volterra dynamics for patterns competing for the same edge.
///
/// When multiple patterns exploit the same alpha source,
/// the resource (edge) depletes. This creates predator-prey
/// oscillations: patterns grow → edge depletes → patterns shrink →
/// edge recovers → patterns grow again.
///
/// dx/dt = αx - βxy  (prey = edge opportunity)
/// dy/dt = δxy - γy  (predator = pattern exploiting the edge)
pub fn lotka_volterra_update(
    patterns: &mut [(ResonantPattern, f64)],  // (pattern, current_exploitation)
    edge_resources: &mut HashMap<String, f64>,
    dt: f64,
) {
    for (pattern, exploitation) in patterns.iter_mut() {
        for edge_id in pattern.exploited_edges() {
            if let Some(resource) = edge_resources.get_mut(edge_id) {
                let alpha = 0.1;  // resource growth rate
                let beta = 0.02;  // exploitation impact
                let delta = 0.01;  // benefit from exploitation
                let gamma = 0.05; // natural decay of exploitation

                // Prey (resource): grows naturally, depleted by exploitation
                let d_resource = alpha * *resource - beta * *resource * *exploitation;

                // Predator (exploitation): grows from resource, decays naturally
                let d_exploit = delta * *resource * *exploitation - gamma * *exploitation;

                *resource += d_resource * dt;
                *exploitation += d_exploit * dt;
            }
        }
    }
}
```

### Price equation — Partitioning evolutionary change

The Price equation (Price, 1970) partitions evolutionary change in the pattern ecosystem into selection and transmission components:

```rust
/// Price equation for pattern ecosystem analysis.
///
/// Δ(z̄) = Cov(w, z) / w̄ + E(w × Δz) / w̄
///
/// where:
///   z̄ = mean trait value (e.g., accuracy)
///   w = fitness (weight)
///   Cov(w, z) = selection component (fitter patterns have higher z)
///   E(w × Δz) = transmission component (mutation/drift)
///
/// This tells us: how much of the improvement in the pattern ensemble
/// is due to selection (bad patterns dying) vs. mutation (new patterns
/// being better than their parents)?
pub fn price_equation(
    patterns: &[ResonantPattern],
    trait_fn: impl Fn(&ResonantPattern) -> f64,
) -> PriceDecomposition {
    let fitnesses: Vec<f64> = patterns.iter().map(|p| p.weight).collect();
    let traits: Vec<f64> = patterns.iter().map(&trait_fn).collect();

    let mean_fitness: f64 = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;
    let mean_trait: f64 = traits.iter().sum::<f64>() / traits.len() as f64;

    // Covariance(fitness, trait) = selection pressure
    let covariance: f64 = fitnesses.iter().zip(traits.iter())
        .map(|(w, z)| (w - mean_fitness) * (z - mean_trait))
        .sum::<f64>() / fitnesses.len() as f64;

    let selection = covariance / mean_fitness;

    PriceDecomposition {
        total_change: 0.0,  // computed from generation-over-generation comparison
        selection_component: selection,
        transmission_component: 0.0,  // requires parent-offspring comparison
    }
}
```

---

## Implementation details

### Rips filtration: point cloud sizing and memory

The Rips filtration builds a simplicial complex from a point cloud. The critical cost constraint is the O(n^2) distance matrix:

```rust
/// Rips filtration configuration.
pub struct RipsFiltrationConfig {
    /// Maximum number of points in the point cloud.
    /// Memory cost: O(n^2) for the distance matrix.
    ///   n = 1,000:  ~8 MB (f64 distances)
    ///   n = 5,000:  ~200 MB
    ///   n = 10,000: ~800 MB
    /// Default: 2,000 (keeps memory under 32 MB).
    pub max_points: usize,

    /// Distance metric for the point cloud.
    pub distance_metric: DistanceMetric,

    /// Maximum filtration scale (distances beyond this are ignored).
    /// Default: f64::MAX (no cutoff). Set lower to reduce computation.
    pub max_scale: f64,

    /// Maximum homological dimension to compute.
    /// 0 = connected components only. 1 = + loops. 2 = + voids.
    /// Default: 1 (components and loops). Higher dimensions are
    /// exponentially more expensive.
    pub max_dimension: usize,
}

pub enum DistanceMetric {
    /// Euclidean distance: sqrt(sum((x_i - y_i)^2)).
    /// Standard choice for delay-embedded time series.
    Euclidean,
    /// Maximum norm: max(|x_i - y_i|).
    /// Cheaper to compute, produces similar persistence diagrams.
    Chebyshev,
    /// Correlation distance: 1 - pearson_correlation(x, y).
    /// Use when magnitudes are uninformative (scale-invariant).
    Correlation,
}
```

For point clouds exceeding `max_points`, subsample uniformly at random. The persistence diagram is stable under subsampling: the bottleneck distance between the full and subsampled diagrams is bounded by 2 * the Hausdorff distance of the subsample (stability theorem, Cohen-Steiner et al., 2007).

### Persistence computation: algorithm selection

```rust
/// Persistence algorithm configuration.
pub struct PersistenceConfig {
    /// Algorithm choice.
    pub algorithm: PersistenceAlgorithm,
    /// Rust library for computation.
    /// Recommended: `ripser` crate (Rust port of Ripser).
    /// Fallback: `gudhi-rs` bindings if available.
    pub backend: PersistenceBackend,
}

pub enum PersistenceAlgorithm {
    /// Ripser (Bauer, 2021): optimized for Rips complexes.
    /// Uses implicit representations to avoid storing the full complex.
    /// Memory: O(n^2) for the distance matrix only.
    /// Speed: fastest known algorithm for Rips persistence.
    Ripser,
    /// Standard persistence via matrix reduction.
    /// Memory: O(m) where m = number of simplices (can be huge).
    /// Use only for non-Rips filtrations.
    MatrixReduction,
    /// Cohomology-based algorithm (de Silva et al., 2011).
    /// Faster than matrix reduction for high-dimensional features.
    /// Good choice when max_dimension >= 2.
    Cohomology,
}

pub enum PersistenceBackend {
    /// Pure Rust Ripser port.
    RipserRs,
    /// GUDHI bindings (requires C++ library).
    GudhiBindings,
}
```

**Recommendation**: Use `Ripser` + `RipserRs` for all standard use cases. Switch to `Cohomology` only when computing dimension >= 2 persistence on large point clouds.

### Delay embedding: dynamic parameter selection

Takens' embedding theorem requires choosing `embedding_dim` and `delay`. These are selected dynamically from the data:

```rust
/// Select delay embedding parameters from the time series.
///
/// delay: first minimum of the average mutual information (AMI).
///   AMI measures nonlinear dependence between x(t) and x(t+tau).
///   The first minimum gives the smallest tau where the lagged values
///   provide maximally independent information.
///
/// embedding_dim: smallest d where the false nearest neighbors (FNN)
///   fraction drops below 1%. FNN counts how many "close" points in
///   d dimensions are no longer close in d+1 dimensions.
pub struct DelayEmbeddingSelector {
    /// Maximum lag to test for AMI minimum.
    pub max_delay: usize,          // default: 50
    /// Maximum dimension to test for FNN.
    pub max_dim: usize,            // default: 10
    /// FNN threshold: stop when FNN fraction < this.
    pub fnn_threshold: f64,        // default: 0.01
}

impl DelayEmbeddingSelector {
    pub fn select(&self, series: &[f64]) -> (usize, usize) {
        let delay = self.first_ami_minimum(series);
        let dim = self.fnn_dimension(series, delay);
        (dim, delay)
    }

    fn first_ami_minimum(&self, series: &[f64]) -> usize {
        let mut prev_ami = f64::MAX;
        for tau in 1..=self.max_delay.min(series.len() / 4) {
            let ami = average_mutual_information(series, tau);
            if ami > prev_ami {
                return tau - 1; // previous tau was the minimum
            }
            prev_ami = ami;
        }
        self.max_delay // no minimum found, use max
    }

    fn fnn_dimension(&self, series: &[f64], delay: usize) -> usize {
        for d in 1..=self.max_dim {
            let fnn_frac = false_nearest_neighbors(series, d, delay);
            if fnn_frac < self.fnn_threshold {
                return d;
            }
        }
        self.max_dim // no clean embedding, use max
    }
}
```

**Detrending**: Before delay embedding, remove trends to prevent non-stationarity from dominating the topology. Apply first-order differencing: `x'(t) = x(t) - x(t-1)`. For strongly trending series, apply second-order differencing.

### Persistence landscape: discretization and construction

The persistence landscape is discretized on a uniform grid for practical computation:

```rust
/// Discretize a persistence landscape on a uniform grid.
///
/// The grid spans [t_min, t_max] with n_grid points.
/// At each grid point, evaluate the k-th layer function.
pub struct LandscapeDiscretization {
    /// Grid resolution (number of points).
    pub n_grid: usize,           // default: 500
    /// Number of landscape layers to compute.
    pub n_layers: usize,         // default: 5
    /// Grid range (auto-detected from diagram if not specified).
    pub t_min: Option<f64>,
    pub t_max: Option<f64>,
}

/// Construct discretized landscape from a persistence diagram.
///
/// For each (b, d) pair in the diagram, create a tent function:
///   f(t) = t - b        for b <= t <= (b+d)/2
///   f(t) = d - t        for (b+d)/2 <= t <= d
///   f(t) = 0            otherwise
///
/// Layer k at grid point t is the k-th largest tent function value at t.
pub fn discretize_landscape(
    diagram: &PersistenceDiagram,
    config: &LandscapeDiscretization,
) -> Vec<Vec<f64>> {
    let (t_min, t_max) = match (config.t_min, config.t_max) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            let births: Vec<f64> = diagram.points.iter().map(|(b, _)| *b).collect();
            let deaths: Vec<f64> = diagram.points.iter().map(|(_, d)| *d).collect();
            (*births.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&0.0),
             *deaths.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&1.0))
        }
    };

    let step = (t_max - t_min) / config.n_grid as f64;
    let mut layers = vec![vec![0.0; config.n_grid]; config.n_layers];

    for grid_idx in 0..config.n_grid {
        let t = t_min + grid_idx as f64 * step;
        let mut values: Vec<f64> = diagram.points.iter()
            .map(|(b, d)| {
                let mid = (b + d) / 2.0;
                if t >= *b && t <= mid {
                    t - b
                } else if t > mid && t <= *d {
                    d - t
                } else {
                    0.0
                }
            })
            .collect();
        values.sort_by(|a, b| b.partial_cmp(a).unwrap());

        for k in 0..config.n_layers.min(values.len()) {
            layers[k][grid_idx] = values[k];
        }
    }

    layers
}
```

### Pattern niche: center and specificity

The niche center is computed as the weighted mean of recent activation states. Specificity measures how narrow the niche is:

```rust
/// Compute niche from activation history.
///
/// center = weighted mean of states where the pattern activated,
///          weighted by activation strength.
///
/// specificity = 1.0 / (1.0 + normalized_variance_of_activation_states).
///   specificity near 1.0: narrow specialist (activates in similar conditions).
///   specificity near 0.0: broad generalist (activates everywhere).
pub fn compute_niche(activation_history: &[(Vec<f64>, f64)]) -> PatternNiche {
    let total_weight: f64 = activation_history.iter().map(|(_, w)| w).sum();
    let dim = activation_history[0].0.len();

    let center: Vec<f64> = (0..dim).map(|d| {
        activation_history.iter()
            .map(|(state, w)| state[d] * w / total_weight)
            .sum()
    }).collect();

    let variance: f64 = activation_history.iter()
        .map(|(state, w)| {
            let dist_sq: f64 = state.iter().zip(&center)
                .map(|(s, c)| (s - c).powi(2))
                .sum();
            dist_sq * w / total_weight
        })
        .sum();

    let radius = variance.sqrt();
    let specificity = 1.0 / (1.0 + variance / dim as f64);

    PatternNiche { center, radius, specificity }
}
```

### VCG auction: auctioneer, payment, zero-bid prevention

The auctioneer is the heartbeat's Theta-frequency tick. At each Theta tick, active patterns bid for inclusion in the cognitive context (limited to `budget` slots).

**Payment mechanism**: Each winning pattern pays the externality it imposes -- the decrease in total welfare that others experience because this pattern occupies a slot. In practice, this equals the bid of the highest-ranked excluded pattern:

```rust
/// VCG payment computation.
///
/// For winner i with bid b_i, payment = optimal welfare without i minus
/// welfare of others when i wins.
///
/// With single-item-per-slot allocation, this simplifies to:
/// payment_i = bid of the (budget+1)-th ranked pattern.
///
/// Zero-bid prevention: patterns with weight < min_bid are excluded
/// from the auction entirely.
pub struct AuctionConfig {
    /// Maximum patterns in the cognitive context.
    pub budget: usize,           // default: 10
    /// Minimum bid to participate.
    pub min_bid: f64,            // default: 0.001
}
```

If fewer than `budget` patterns have bids above `min_bid`, all qualifying patterns win and pay zero (no competition).

### Lotka-Volterra: sensitivity analysis and domain calibration

The four Lotka-Volterra parameters have domain-specific interpretations:

| Parameter | Symbol | Chain domain | Coding domain | Default |
|---|---|---|---|---|
| Resource growth rate | alpha | How fast arbitrage opportunity regenerates | How fast new code surfaces bugs | 0.1 |
| Exploitation impact | beta | How much trading depletes the opportunity | How much testing reveals bugs | 0.02 |
| Benefit from exploitation | delta | Profit per unit of opportunity exploited | Information gain per bug found | 0.01 |
| Natural decay | gamma | Strategy obsolescence rate (MEV competition) | Bug fix rate (resolves the opportunity) | 0.05 |

**Sensitivity analysis**: The system has a stable equilibrium at:
- `resource* = gamma / delta`
- `exploitation* = alpha / beta`

Small perturbations around equilibrium oscillate with period `T = 2*pi / sqrt(alpha * gamma)`. With defaults: `T = 2*pi / sqrt(0.005) ~= 89` time units.

If `alpha * gamma` is too small, oscillations are slow and the system appears static. If `beta * delta` is too large relative to `alpha * gamma`, the system collapses (exploitation exceeds recovery).

**Calibration procedure**: Observe real resource recovery rates and exploitation impact over 20+ Theta cycles. Fit alpha, beta, delta, gamma via least-squares on the observed trajectories.

### Error handling

- **Empty persistence diagram**: If the point cloud produces no persistent features, return an empty PersistenceLandscape with zero layers.
- **Auction with zero patterns**: Return empty winners list.
- **Lotka-Volterra negative values**: Clamp resource and exploitation to `[0.0, max_resource]`. Log a warning if clamping occurs.
- **Delay embedding on short series**: If `series.len() < embedding_dim * delay`, fall back to `embedding_dim = 2, delay = 1`.
- **NaN in niche computation**: If all activation weights are zero, return a niche with center at the origin and radius = infinity (universal generalist).

### Integration wiring

```
Oracle::predict()
  -> collect recent time series (last 2000 points)
  -> DelayEmbeddingSelector::select() for dynamic dim/delay
  -> delay_embedding() to produce point cloud
  -> RipsFiltration with Ripser backend
  -> PersistenceDiagram -> PersistenceLandscape
  -> TopologyToTrajectory::predict() for topological constraints
  -> ResonantPattern ecosystem:
       -> pattern_auction() at Theta frequency
       -> lotka_volterra_update() for resource dynamics
       -> reproduce() for top patterns at Delta frequency
  -> encode landscape features as HDC vector
  -> emit as Engram
```

### Test criteria

- **Takens embedding**: Delay-embedding a sine wave with period P recovers a circle-like point cloud. The H1 persistence diagram has one dominant point with lifetime proportional to the amplitude.
- **Persistence stability**: Adding Gaussian noise with stddev sigma shifts the bottleneck distance by at most O(sigma).
- **Landscape linearity**: `landscape(A + B) == landscape(A).add(landscape(B))` for diagrams A, B.
- **VCG truthfulness**: No pattern benefits from bidding other than its true value.
- **Lotka-Volterra equilibrium**: Starting from equilibrium with default parameters, the system stays within 1% of equilibrium for 1000 steps with dt=0.1.
- **Niche convergence**: After 100 activations in similar states, the niche center is within 5% of the true activation centroid.
- **Memory budget**: 2000-point Rips filtration completes within 32 MB memory.

---

## Advanced TDA: Persistence Images, Vectorization, and Computation

### Persistence Images — ML-Compatible TDA Features

Persistence diagrams are sets of points — not vectors. This makes them incompatible with standard ML algorithms (SVMs, random forests, neural networks) that require fixed-dimensional inputs. Persistence images solve this by transforming diagrams into stable, finite-dimensional vector representations.

```rust
/// Persistence images: stable vector representation of persistent homology.
///
/// Adams et al. (2017, JMLR 18(8), 1-35): Transforms persistence diagrams
/// into finite-dimensional vectors compatible with standard ML algorithms.
///
/// Algorithm:
/// 1. Transform (birth, death) → (birth, persistence) where persistence = death - birth
/// 2. Apply weighting function w(b,p) (e.g., linear ramp w = p, or Gaussian)
/// 3. Sum weighted Gaussians centered at each (b,p) point
/// 4. Discretize on n×n grid → flatten to n²-dimensional vector
///
/// Stability: If d_W1(D, D') < ε, then ||PI(D) - PI(D')|| < C·ε
/// for Lipschitz weighting functions.
pub struct PersistenceImageConfig {
    /// Grid resolution per axis.
    pub resolution: usize,           // default: 50 (produces 2500-dim vector)
    /// Gaussian bandwidth for kernel density estimation.
    pub sigma: f64,                  // default: auto (median persistence / 5)
    /// Weighting function for persistence points.
    pub weight: PersistenceWeight,
    /// Birth range for the grid.
    pub birth_range: Option<(f64, f64)>,
    /// Persistence range for the grid.
    pub persistence_range: Option<(f64, f64)>,
}

pub enum PersistenceWeight {
    /// Linear ramp: w(b,p) = p (long-lived features weighted more).
    Linear,
    /// Gaussian: w(b,p) = 1 - exp(-p²/2σ²).
    Gaussian { sigma: f64 },
    /// Uniform: w(b,p) = 1 (all features equal).
    Uniform,
    /// Custom function.
    Custom(Box<dyn Fn(f64, f64) -> f64 + Send + Sync>),
}
```

The key design choice is the weighting function. Linear weighting (`w(b,p) = p`) emphasizes long-lived topological features and suppresses noise near the diagonal, which is the right default for time series analysis where persistent structure matters more than transient features. Gaussian weighting provides a softer transition and is preferred when even short-lived features carry signal (e.g., detecting brief microstructure anomalies). Uniform weighting treats all features equally and should only be used when the persistence distribution itself is the signal of interest.

The bandwidth parameter `sigma` controls smoothing. Too small and the image is a collection of spikes (overfitting to specific diagram points). Too large and distinct features blur together. The default heuristic — median persistence divided by 5 — provides a reasonable starting point that can be tuned via cross-validation on downstream task performance.

**Computational cost**: For a diagram with `m` points on an `n x n` grid, computing a persistence image costs `O(m * n²)`. With typical values (m ~ 100, n = 50), this is ~250K multiplications — negligible compared to the persistence computation itself.

**Integration with HDC**: Persistence image vectors can be binarized (threshold at the median value) to produce HDC-compatible binary vectors. This enables combining topological features with the existing HDC signal encoding pipeline via BIND and BUNDLE operations.

### Computational Backends: Ripser and Alpha Complexes

The choice of computational backend determines both the speed and the class of input data that can be processed. The three main complex types — Vietoris-Rips, Alpha, and Cubical — cover different data geometries:

```rust
/// TDA computation backend selection.
///
/// Ripser (Bauer, 2021, JACT 5(1), 91-119): Fastest known algorithm
/// for Vietoris-Rips persistence. Uses persistent cohomology, apparent
/// pairs optimization (eliminates ~99% of pairs before main algorithm),
/// and implicit coboundary representation.
///
/// Alpha complexes (Edelsbrunner & Harer, 2010): Subset of Delaunay
/// triangulation. Sparser than Rips, faster in low dimensions (d ≤ 3),
/// but exponential in ambient dimension.
pub struct TdaBackendConfig {
    /// Which complex to build.
    pub complex_type: ComplexType,
    /// Maximum homological dimension.
    pub max_dim: usize,              // default: 1
    /// Maximum filtration value.
    pub max_filtration: f64,         // default: f64::MAX
    /// Point cloud subsampling (for large datasets).
    pub max_points: usize,           // default: 2000
    /// Vectorization method for ML integration.
    pub vectorization: VectorizationMethod,
}

pub enum ComplexType {
    /// Vietoris-Rips: works in any dimension, O(n²) distance matrix.
    /// Use for d > 3 or abstract metric spaces.
    VietorisRips,
    /// Alpha complex: subset of Delaunay, O(n^{d/2}) construction.
    /// Use for d ≤ 3 with Euclidean data.
    Alpha,
    /// Cubical: for gridded data (images, volumes). O(n) construction.
    Cubical,
}

pub enum VectorizationMethod {
    /// Persistence images (Adams et al., 2017).
    PersistenceImage(PersistenceImageConfig),
    /// Persistence landscapes (Bubenik, 2015).
    PersistenceLandscape { n_layers: usize, n_grid: usize },
    /// Betti curve: β_k(t) = count of features alive at scale t.
    BettiCurve { n_grid: usize },
    /// Persistence entropy: H = -Σ (l_i/L) log(l_i/L) where l_i = lifetime.
    Entropy,
    /// Persistence silhouette (weighted Betti curve).
    Silhouette { power: f64 },
}
```

**Backend selection guide**:

| Data type | Recommended complex | Reason |
|---|---|---|
| Delay-embedded time series (d = 3) | Alpha | Sparser complex, exact Delaunay geometry |
| Delay-embedded time series (d > 3) | Vietoris-Rips | Alpha is exponential in d |
| Abstract metric space (non-Euclidean) | Vietoris-Rips | Only option for non-Euclidean |
| Gridded data (heatmaps, images) | Cubical | O(n) construction, natural fit |
| Large point cloud (n > 5000) | Vietoris-Rips + subsampling | Ripser handles large n with max_filtration cutoff |

**Ripser optimizations**: Ripser achieves its speed through three key techniques. First, persistent cohomology computes the same barcode as homology but with a more cache-friendly access pattern. Second, the apparent pairs optimization identifies simplex pairs that must be paired together without running the full reduction — this eliminates approximately 99% of pairs in typical datasets. Third, the implicit coboundary representation avoids materializing the full boundary matrix, keeping memory at O(n^2) for the distance matrix alone.

**GPU acceleration**: Zhang et al. (2020) demonstrate GPU-accelerated Vietoris-Rips computation achieving 20-50x speedup over CPU Ripser for point clouds with n > 10,000. This is relevant for batch processing of historical time series but not for real-time computation where n is typically capped at 2,000.

### Vectorization Method Comparison

Each vectorization method trades off different properties:

| Method | Output dimension | Stability | Interpretability | Cost |
|---|---|---|---|---|
| Persistence image | n² (e.g., 2500) | W1-stable | Moderate (heatmap) | O(m * n²) |
| Persistence landscape | k * n_grid | Lp-stable | High (layer functions) | O(m * n_grid) |
| Betti curve | n_grid | Not stable | High (feature count) | O(m * n_grid) |
| Entropy | 1 | Stable | High (single scalar) | O(m) |
| Silhouette | n_grid | Stable | Moderate | O(m * n_grid) |

**Persistence landscapes** (already detailed in Part I) live in a Banach space and support arithmetic. They are the right choice when you need to compute means, perform hypothesis tests, or do regression on topological features.

**Betti curves** count the number of topological features alive at each scale. They are the simplest vectorization and the most interpretable, but they are not stable under perturbations (a single noisy point can create a spurious feature that shifts the curve). Use Betti curves for visualization and exploratory analysis, not for ML features.

**Persistence entropy** compresses the entire diagram into a single scalar measuring disorder. High entropy means many features with similar lifetimes (uniform structure). Low entropy means a few dominant features (concentrated structure). This is useful as a regime indicator: high entropy regimes are noisy/chaotic, low entropy regimes have clear structure.

**Persistence silhouettes** are weighted Betti curves where longer-lived features contribute more. The `power` parameter controls the weighting: power = 1 gives equal weight, power > 1 emphasizes long-lived features, power < 1 emphasizes short-lived features. Silhouettes are stable and provide a middle ground between Betti curves (unstable but interpretable) and persistence images (stable but less interpretable).

### Composition with Existing Pipeline

The vectorization methods integrate into the existing `Oracle::predict()` pipeline at the point where persistence diagrams are converted to features:

```
Oracle::predict()
  -> ... (existing pipeline up to PersistenceDiagram)
  -> TdaBackendConfig selects complex type and vectorization
  -> VectorizationMethod produces fixed-dimensional vector
  -> if PersistenceImage: binarize → HdcVector for BIND/BUNDLE
  -> if PersistenceLandscape: use existing Banach space arithmetic
  -> if Entropy: feed as scalar feature to CascadeRouter
  -> encode as Engram via existing HDC pipeline
```

### Citations

- Adams, H., et al. (2017). "Persistence Images: A Stable Vector Representation of Persistent Homology." *JMLR*, 18(8), 1-35. — Persistence images: stable vectorization of persistence diagrams for ML.
- Bauer, U. (2021). "Ripser: Efficient Computation of Vietoris-Rips Persistence Barcodes." *JACT*, 5(1), 91-119. — Fastest known algorithm for Rips persistence.
- Zhang, S., Xiao, M., & Wang, H. (2020). "GPU-Accelerated Computation of Vietoris-Rips Persistence Barcodes." *SoCG 2020*. — GPU parallelization of Ripser.
- Cohen-Steiner, D., Edelsbrunner, H., & Harer, J. (2007). "Stability of Persistence Diagrams." *DCG*, 37(1), 103-120. — Foundational stability theorem for persistence.
- Luchinsky, A., & Islambekov, U. (2025). "TDAvec: Vectorization of Persistence Diagrams." *JOSS*, 10(114). — Unified vectorization library and comparative analysis.

### Test criteria

- **PI stability**: Adding noise with stddev sigma to a point cloud shifts the PI vector by at most O(sigma). Verify by computing PI on 100 noisy copies of a reference point cloud and checking that the maximum L2 deviation scales linearly with sigma.
- **Ripser vs Alpha agreement**: For 2D Euclidean data, both Vietoris-Rips and Alpha complex backends produce identical persistence diagrams (up to floating-point tolerance of 1e-10).
- **Vectorization determinism**: Same diagram produces identical vectors across runs. Verify by computing each vectorization method 10 times on the same diagram and asserting bitwise equality.
- **Binarization round-trip**: Binarizing a persistence image and computing Hamming similarity between two diagrams preserves the rank ordering of W1 distances (Spearman correlation > 0.9).
- **Entropy monotonicity**: A diagram with one dominant feature has lower entropy than a diagram with many equal-lifetime features. Verify on synthetic diagrams.

---

## Academic foundations

- Bubenik, P. (2015). "Statistical Topological Data Analysis using Persistence Landscapes." *JMLR*, 16(3), 77-102. — Persistence landscapes as Banach space elements.
- Takens, F. (1981). "Detecting strange attractors in turbulence." *Lecture Notes in Mathematics*, 898, 366-381. — Delay embedding theorem for time series topology.
- Price, G. R. (1970). "Selection and Covariance." *Nature*, 227, 520-521. — The Price equation for partitioning evolutionary change.
- Vickrey, W. (1961). "Counterspeculation, Auctions, and Competitive Sealed Tenders." *Journal of Finance*, 16(1). — VCG auction for pattern competition.
- Lotka, A. J. (1925). *Elements of Physical Biology*. Williams & Wilkins. — Predator-prey dynamics.
- Volterra, V. (1926). "Fluctuations in the Abundance of a Species considered Mathematically." *Nature*, 118, 558-560. — Population dynamics equations.
- Carlsson, G. (2009). "Topology and Data." *Bulletin of the AMS*, 46(2), 255-308. — TDA foundations.

---

## Cross-References

- See [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) for HDC genome encoding
- See [07-spectral-liquidity-manifolds.md](./07-spectral-liquidity-manifolds.md) for Riemannian geometry (complementary to TDA)
- See [08-adaptive-signal-metabolism.md](./08-adaptive-signal-metabolism.md) for signal-level evolution (patterns are composed of signals)
- See [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) for emergent intelligence from pattern interactions


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/11-adversarial-signal-robustness.md

# Adversarial Signal Robustness

> Every domain has adversaries who manipulate signals. MEV searchers manipulate prices. Attackers manipulate supply chains. p-hackers manipulate statistics. The adversarial robustness subsystem defends predictions through HDC prototype matching, robust statistics, and red-team dreaming.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [06-hyperdimensional-ta](./06-hyperdimensional-ta.md) for HDC encoding, [09-causal-microstructure-discovery](./09-causal-microstructure-discovery.md) for causal analysis
**Key sources**: `bardo-backup/prd/23-ta/08-adversarial-signal-robustness.md`

---

## Adversarial signal decomposition

Every observed signal is a mixture of genuine information and adversarial manipulation. The first defense is decomposition — separating the signal into components and identifying which parts are trustworthy:

```rust
/// Decompose a signal into genuine and adversarial components.
///
/// The decomposition model:
///   observed = genuine + adversarial + noise
///
/// Genuine: reflects actual state (market fundamentals, code quality)
/// Adversarial: intentional manipulation (MEV, supply chain, p-hacking)
/// Noise: random, zero-mean disturbance
///
/// Identification uses multiple methods:
/// - Statistical outlier detection (robust statistics)
/// - Causal consistency (does this signal fit the causal model?)
/// - HDC prototype matching (does this match a known attack pattern?)
/// - Cross-source verification (do independent sources agree?)
pub struct AdversarialDecomposer {
    /// Robust statistics engine.
    robust_stats: RobustStatistics,

    /// Causal model for consistency checking.
    causal_model: Arc<StructuralCausalModel>,

    /// HDC prototypes of known attack patterns.
    attack_prototypes: Vec<HdcVector>,

    /// Cross-source verifier.
    cross_verifier: CrossSourceVerifier,
}

pub struct SignalDecomposition {
    /// Estimated genuine component.
    pub genuine: f64,

    /// Estimated adversarial component.
    pub adversarial: f64,

    /// Estimated noise component.
    pub noise: f64,

    /// Confidence in the decomposition.
    pub confidence: f64,

    /// If adversarial component is significant: which attack pattern matched?
    pub attack_match: Option<AttackPatternMatch>,
}
```

---

## HDC prototype matching — Nanosecond attack detection

Known adversarial patterns are encoded as HDC prototype vectors. Incoming signals are compared against all prototypes via Hamming similarity:

```rust
/// HDC prototype matching for adversarial pattern detection.
///
/// Cost: ~10ns per prototype comparison (XOR + popcount on 10,240 bits).
/// For 1,000 known attack patterns: ~10µs total.
///
/// This is fast enough to run at Gamma frequency on every observation.
pub struct PrototypeMatcher {
    /// Known adversarial pattern prototypes.
    prototypes: Vec<PrototypeEntry>,

    /// Similarity threshold for match detection (typically 0.6).
    threshold: f64,
}

pub struct PrototypeEntry {
    /// The HDC prototype vector.
    pub vector: HdcVector,

    /// Human-readable name of the attack pattern.
    pub name: String,

    /// Domain this prototype belongs to.
    pub domain: OracleDomain,

    /// Severity if this pattern is detected.
    pub severity: f64,

    /// Recommended response.
    pub response: AdversarialResponse,
}

pub enum AdversarialResponse {
    /// Widen prediction intervals (increase uncertainty).
    WidenIntervals(f64),

    /// Suppress action (wait for the adversarial activity to pass).
    SuppressAction(Duration),

    /// Escalate to T2 (deep reasoning) for manual analysis.
    EscalateToT2,

    /// Emit a Warning to Neuro.
    EmitWarning(String),
}

impl PrototypeMatcher {
    /// Match an observation against all prototypes.
    ///
    /// Returns all prototypes with similarity above threshold,
    /// sorted by similarity (most similar first).
    pub fn match_prototypes(&self, observation: &HdcVector) -> Vec<(f64, &PrototypeEntry)> {
        self.prototypes.iter()
            .filter_map(|proto| {
                let sim = observation.hamming_similarity(&proto.vector);
                if sim > self.threshold {
                    Some((sim, proto))
                } else {
                    None
                }
            })
            .sorted_by(|a, b| b.0.partial_cmp(&a.0).unwrap())
            .collect()
    }
}
```

### Domain-specific attack prototypes

```rust
/// Chain domain attack prototypes.
pub fn chain_attack_prototypes(codebook: &DeFiCodebook) -> Vec<PrototypeEntry> {
    vec![
        // Sandwich attack: buy before victim, sell after
        PrototypeEntry {
            vector: encode_sandwich_pattern(codebook),
            name: "sandwich_attack".into(),
            domain: OracleDomain::Chain,
            severity: 0.8,
            response: AdversarialResponse::SuppressAction(Duration::from_secs(12)),
        },

        // Oracle manipulation: flash loan → manipulate price feed → profit
        PrototypeEntry {
            vector: encode_oracle_manipulation_pattern(codebook),
            name: "oracle_manipulation".into(),
            domain: OracleDomain::Chain,
            severity: 0.9,
            response: AdversarialResponse::EscalateToT2,
        },

        // Governance attack: flash loan → vote → profit
        PrototypeEntry {
            vector: encode_governance_attack_pattern(codebook),
            name: "governance_attack".into(),
            domain: OracleDomain::Chain,
            severity: 0.95,
            response: AdversarialResponse::EmitWarning("Potential governance attack detected".into()),
        },

        // JIT liquidity sniping
        PrototypeEntry {
            vector: encode_jit_sniping_pattern(codebook),
            name: "jit_sniping".into(),
            domain: OracleDomain::Chain,
            severity: 0.5,
            response: AdversarialResponse::WidenIntervals(0.2),
        },
    ]
}

/// Coding domain attack prototypes.
pub fn coding_attack_prototypes(codebook: &CodingCodebook) -> Vec<PrototypeEntry> {
    vec![
        // Dependency confusion: malicious package name squatting
        PrototypeEntry {
            vector: encode_dep_confusion_pattern(codebook),
            name: "dependency_confusion".into(),
            domain: OracleDomain::Coding,
            severity: 0.9,
            response: AdversarialResponse::EscalateToT2,
        },

        // Typosquatting: similar package name with malicious payload
        PrototypeEntry {
            vector: encode_typosquatting_pattern(codebook),
            name: "typosquatting".into(),
            domain: OracleDomain::Coding,
            severity: 0.85,
            response: AdversarialResponse::EmitWarning("Potential typosquatting detected".into()),
        },

        // Build artifact tampering
        PrototypeEntry {
            vector: encode_build_tampering_pattern(codebook),
            name: "build_tampering".into(),
            domain: OracleDomain::Coding,
            severity: 0.9,
            response: AdversarialResponse::EscalateToT2,
        },
    ]
}
```

---

## Robust statistics — Defending numerical estimates

When adversarial manipulation is suspected, standard statistics (mean, variance) are unreliable. Robust estimators resist contamination:

```rust
/// Robust statistics for adversarial-contaminated data.
///
/// Each estimator resists a fraction of contaminated data points
/// (the "breakdown point"). Standard mean has breakdown point 0
/// (one outlier can destroy it). These estimators have breakdown
/// points of 25-50%.
pub struct RobustStatistics;

impl RobustStatistics {
    /// Trimmed mean: discard the top and bottom α fraction before averaging.
    ///
    /// Breakdown point: α (typically 0.1-0.25).
    /// Removes extreme values that may be adversarial.
    pub fn trimmed_mean(data: &[f64], alpha: f64) -> f64 {
        let n = data.len();
        let trim = (n as f64 * alpha) as usize;
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let trimmed = &sorted[trim..n - trim];
        trimmed.iter().sum::<f64>() / trimmed.len() as f64
    }

    /// Hodges-Lehmann estimator: median of all pairwise averages.
    ///
    /// Breakdown point: ~29%.
    /// More efficient than trimmed mean for symmetric distributions.
    pub fn hodges_lehmann(data: &[f64]) -> f64 {
        let n = data.len();
        let mut pairwise_means = Vec::with_capacity(n * (n + 1) / 2);

        for i in 0..n {
            for j in i..n {
                pairwise_means.push((data[i] + data[j]) / 2.0);
            }
        }

        pairwise_means.sort_by(|a, b| a.partial_cmp(b).unwrap());
        pairwise_means[pairwise_means.len() / 2]
    }

    /// Winsorized variance: clip extreme values before computing variance.
    ///
    /// Breakdown point: α.
    /// More stable than standard variance under contamination.
    pub fn winsorized_variance(data: &[f64], alpha: f64) -> f64 {
        let n = data.len();
        let trim = (n as f64 * alpha) as usize;
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let lower = sorted[trim];
        let upper = sorted[n - trim - 1];

        let winsorized: Vec<f64> = data.iter()
            .map(|&x| x.clamp(lower, upper))
            .collect();

        let mean = winsorized.iter().sum::<f64>() / n as f64;
        winsorized.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64
    }

    /// Median Absolute Deviation (MAD): robust scale estimator.
    ///
    /// Breakdown point: 50% (highest possible).
    /// MAD = median(|x_i - median(x)|)
    /// Scaled MAD: 1.4826 * MAD ≈ standard deviation for Gaussian data.
    pub fn mad(data: &[f64]) -> f64 {
        let median = Self::median(data);
        let deviations: Vec<f64> = data.iter().map(|x| (x - median).abs()).collect();
        Self::median(&deviations)
    }

    /// Rank-order transformation: replace values with ranks.
    ///
    /// Completely eliminates magnitude-based manipulation.
    /// Preserves ordinal relationships but discards scale information.
    pub fn rank_transform(data: &[f64]) -> Vec<f64> {
        let n = data.len() as f64;
        let mut indexed: Vec<_> = data.iter().enumerate().collect();
        indexed.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

        let mut ranks = vec![0.0; data.len()];
        for (rank, (original_idx, _)) in indexed.iter().enumerate() {
            ranks[*original_idx] = rank as f64 / n;
        }
        ranks
    }

    fn median(data: &[f64]) -> f64 {
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();
        if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        }
    }
}
```

---

## Signal cross-validation

Multiple independent signals predicting the same outcome should agree. Disagreement indicates either adversarial manipulation or model error:

```rust
/// Cross-validate signals predicting the same outcome.
///
/// If multiple independent signals (e.g., different data sources
/// predicting the same price) disagree significantly, at least one
/// is compromised. The cross-validator identifies outlier signals.
pub struct SignalCrossValidator {
    /// Maximum acceptable disagreement (MAD-based).
    max_disagreement: f64,
}

impl SignalCrossValidator {
    /// Validate a set of predictions for the same outcome.
    pub fn validate(&self, predictions: &[(SignalId, f64)]) -> CrossValidationResult {
        let values: Vec<f64> = predictions.iter().map(|(_, v)| *v).collect();
        let median = RobustStatistics::median(&values);
        let mad = RobustStatistics::mad(&values);

        let outliers: Vec<SignalId> = predictions.iter()
            .filter(|(_, v)| (*v - median).abs() > self.max_disagreement * mad * 1.4826)
            .map(|(id, _)| *id)
            .collect();

        CrossValidationResult {
            consensus: median,
            spread: mad * 1.4826,  // scaled MAD ≈ std dev
            outlier_signals: outliers,
            is_consistent: outliers.is_empty(),
        }
    }
}
```

---

## Red-team dreaming — Adversarial simulation

During Delta-frequency Dreams, the agent runs adversarial simulations against its own strategies:

```rust
/// Red-team dreaming: the agent attacks its own strategies.
///
/// During REM Dreams, the agent generates adversarial scenarios
/// and tests whether its current predictions and strategies survive.
///
/// Algorithm:
/// 1. Select the agent's top N active strategies
/// 2. For each strategy, generate K adversarial perturbations
/// 3. Simulate each perturbation (via mirage-rs or workspace snapshot)
/// 4. If the strategy fails under perturbation:
///    a. Demote the strategy's confidence
///    b. Store the adversarial scenario as a Warning in Neuro
///    c. Generate a defensive modification
///
/// This is how agents develop adversarial robustness WITHOUT
/// encountering real attacks.
pub struct RedTeamDreaming {
    /// The agent's current active strategies.
    strategies: Vec<StrategyFragment>,

    /// Adversarial perturbation generators per domain.
    perturbation_generators: HashMap<OracleDomain, Box<dyn PerturbationGenerator>>,

    /// Simulation environment.
    simulator: Arc<dyn Simulator>,
}

pub trait PerturbationGenerator: Send + Sync {
    /// Generate adversarial perturbations for a strategy.
    fn generate(
        &self,
        strategy: &StrategyFragment,
        n_perturbations: usize,
    ) -> Vec<AdversarialPerturbation>;
}

pub struct AdversarialPerturbation {
    /// What was changed (e.g., "2x slippage", "5x gas", "correlation breakdown").
    pub description: String,

    /// The perturbation as a state modification.
    pub modification: StateModification,

    /// Severity of the adversarial scenario.
    pub severity: f64,
}

impl RedTeamDreaming {
    /// Run one red-team dreaming cycle.
    pub async fn dream_cycle(&self) -> Vec<RedTeamResult> {
        let mut results = Vec::new();

        for strategy in &self.strategies {
            let domain = strategy.domain();
            if let Some(generator) = self.perturbation_generators.get(&domain) {
                let perturbations = generator.generate(strategy, 5);

                for perturbation in perturbations {
                    let outcome = self.simulator
                        .simulate_with_perturbation(strategy, &perturbation)
                        .await;

                    let survived = outcome.success_rate > 0.5;

                    results.push(RedTeamResult {
                        strategy_id: strategy.id,
                        perturbation: perturbation.description.clone(),
                        survived,
                        outcome_detail: outcome,
                    });

                    if !survived {
                        // Strategy failed — this is a discovered vulnerability
                        // Store as Warning in Neuro during dream integration phase
                    }
                }
            }
        }

        results
    }
}
```

### Chain-domain adversarial perturbations

```rust
/// Chain-specific adversarial perturbations for red-team dreaming.
pub struct ChainPerturbationGenerator;

impl PerturbationGenerator for ChainPerturbationGenerator {
    fn generate(
        &self,
        strategy: &StrategyFragment,
        n: usize,
    ) -> Vec<AdversarialPerturbation> {
        vec![
            // What if slippage is 2x higher than expected?
            AdversarialPerturbation {
                description: "2x slippage spike".into(),
                modification: StateModification::ScaleVariable("slippage", 2.0),
                severity: 0.6,
            },
            // What if gas is 5x higher?
            AdversarialPerturbation {
                description: "5x gas spike".into(),
                modification: StateModification::ScaleVariable("gas_price", 5.0),
                severity: 0.7,
            },
            // What if correlations break down?
            AdversarialPerturbation {
                description: "Correlation breakdown: ETH/BTC decorrelation".into(),
                modification: StateModification::BreakCorrelation("eth", "btc"),
                severity: 0.8,
            },
            // What if a pool is drained (rug pull)?
            AdversarialPerturbation {
                description: "Pool drain: 90% liquidity removal".into(),
                modification: StateModification::ScaleVariable("pool_liquidity", 0.1),
                severity: 0.95,
            },
            // What if a sandwich attack targets the strategy?
            AdversarialPerturbation {
                description: "Sandwich attack on primary swap".into(),
                modification: StateModification::InjectSandwich("primary_swap"),
                severity: 0.7,
            },
        ].into_iter().take(n).collect()
    }
}
```

---

## Integration with the Daimon

Adversarial detection feeds directly into the Daimon PAD vector:

- **Detection of adversarial activity** → increases Arousal (urgency)
- **Failed red-team defense** → decreases Dominance (confidence)
- **Successful defense** → increases Pleasure (positive outcome)

This creates a feedback loop: adversarial pressure raises arousal, which routes more cycles to T2 (deep reasoning), which enables more thorough analysis.

---

## Implementation details

### Attack prototype HDC encoding

Each attack prototype is encoded as an HDC vector that captures the structural signature of the attack pattern. The encoding method varies by domain:

```rust
/// Encode a chain attack prototype as an HDC vector.
///
/// The encoding captures the temporal and structural fingerprint of the attack.
/// For a sandwich attack:
///   TEMPORAL_PATTERN(
///     BIND(swap_role, large_buy),       // step 0: frontrun
///     BIND(swap_role, victim_trade),     // step 1: victim
///     BIND(swap_role, large_sell),       // step 2: backrun
///   )
/// bundled with:
///   BIND(timing_role, same_block),       // timing constraint
///   BIND(profit_role, positive),         // profit indicator
///
/// The resulting vector matches any structurally similar sandwich pattern
/// regardless of specific tokens, amounts, or protocols.
pub fn encode_attack_prototype(
    pattern: &AttackPatternDef,
    codebook: &DeFiCodebook,
) -> HdcVector {
    // Encode the temporal sequence of actions
    let temporal = encode_temporal_pattern(
        &pattern.steps.iter()
            .map(|step| encode_ta_state(&step.role_filler_pairs(codebook)))
            .collect::<Vec<_>>()
    );

    // Encode structural constraints (timing, profit, etc.)
    let constraints: Vec<HdcVector> = pattern.constraints.iter()
        .map(|c| c.encode(codebook))
        .collect();

    // Bundle temporal pattern with constraints
    let mut all = vec![temporal];
    all.extend(constraints);
    HdcVector::bundle(&all)
}

/// For coding domain attacks, encode the supply chain signature:
///   BIND(package_role, name_similarity_vector),
///   BIND(action_role, install_hook | postinstall_script),
///   BIND(timing_role, recent_publish),
pub fn encode_coding_attack_prototype(
    pattern: &CodingAttackDef,
    codebook: &CodingCodebook,
) -> HdcVector {
    let components: Vec<HdcVector> = pattern.indicators.iter()
        .map(|indicator| {
            let role = codebook.role_for(indicator.kind);
            let filler = codebook.encode_indicator_value(&indicator.value);
            role.xor(&filler)
        })
        .collect();
    HdcVector::bundle(&components)
}
```

### Prototype selection: count and update procedure

| Domain | Initial prototype count | Source | Update cadence |
|---|---|---|---|
| Chain | 20-50 | Known MEV patterns from Flashbots data, historical exploits | Delta frequency (daily) |
| Coding | 15-30 | Known supply chain attacks from OSV/advisories | Weekly or on new advisory |
| Research | 5-10 | Known p-hacking patterns from replication crisis literature | Monthly |

**Update procedure**:

1. When a new attack is confirmed (by red-team dreaming or external report), encode it as a prototype.
2. Compute similarity to existing prototypes. If max similarity > 0.8, update the existing prototype via bundle (strengthens shared structure).
3. If max similarity <= 0.8, add as a new prototype.
4. Prune prototypes with zero matches in the last 30 days, unless they represent critical attack classes (severity >= 0.9).

```rust
/// Update prototypes with a newly confirmed attack pattern.
pub fn update_prototypes(
    prototypes: &mut Vec<PrototypeEntry>,
    new_attack: &HdcVector,
    metadata: PrototypeMetadata,
    merge_threshold: f64,  // default: 0.8
) {
    let best_match = prototypes.iter_mut()
        .map(|p| (p, new_attack.hamming_similarity(&p.vector)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    match best_match {
        Some((existing, sim)) if sim > merge_threshold => {
            // Merge: bundle existing with new to strengthen shared structure
            existing.vector = existing.vector.bundle_with(new_attack);
            existing.last_matched = now_ms();
        }
        _ => {
            // Add as new prototype
            prototypes.push(PrototypeEntry {
                vector: new_attack.clone(),
                name: metadata.name,
                domain: metadata.domain,
                severity: metadata.severity,
                response: metadata.response,
            });
        }
    }
}
```

### Robust statistics: adaptive trim fraction

The trim fraction alpha for the trimmed mean should adapt based on the suspected contamination rate:

```rust
/// Adaptive trim fraction selection.
///
/// If adversarial activity is detected (prototype match or cross-source
/// disagreement), increase alpha to resist heavier contamination.
///
/// Default: alpha = 0.10 (handles up to 10% contamination).
/// Under adversarial pressure: alpha = min(0.25, 2 * estimated_contamination).
/// Maximum useful alpha: 0.25 (trimming more than 25% from each tail
/// discards too much genuine data).
pub fn adaptive_trim_alpha(
    adversarial_detected: bool,
    estimated_contamination: Option<f64>,
) -> f64 {
    match (adversarial_detected, estimated_contamination) {
        (false, _) => 0.10,                                    // baseline
        (true, Some(rate)) => (2.0 * rate).clamp(0.10, 0.25),  // adaptive
        (true, None) => 0.20,                                   // conservative default
    }
}
```

### Hodges-Lehmann caching for n > 1000

The Hodges-Lehmann estimator computes the median of all `n*(n+1)/2` pairwise averages. For n = 1000, this is ~500K pairs (fast). For n > 1000, the O(n^2) cost becomes significant:

```rust
/// Hodges-Lehmann estimator with subsampling for large n.
///
/// For n <= 1000: exact computation (500K pairs, ~1ms).
/// For n > 1000: subsample to 1000 points, compute exactly on the subsample.
///   Error bound: O(1/sqrt(1000)) = ~3% relative to exact.
///   The subsample is drawn without replacement using reservoir sampling.
///
/// Alternative for very large n: use the Johnson-Ethier approximation
/// (Hodges-Lehmann ~ median + O(1/n)), but this loses robustness.
pub fn hodges_lehmann_cached(data: &[f64], max_exact_n: usize) -> f64 {
    if data.len() <= max_exact_n {
        return RobustStatistics::hodges_lehmann(data);
    }

    // Subsample
    let sample = reservoir_sample(data, max_exact_n);
    RobustStatistics::hodges_lehmann(&sample)
}
```

### MAD scaling constant

The MAD is scaled by 1.4826 to estimate the standard deviation under a Gaussian distribution. This constant equals `1 / Phi_inv(3/4)` where `Phi_inv` is the inverse normal CDF. For non-Gaussian distributions, the constant differs:

| Distribution | Correct scaling constant | When to use |
|---|---|---|
| Gaussian | 1.4826 | Default assumption |
| Laplace (heavy-tailed) | 1.0 | When data has fat tails (common in DeFi) |
| Uniform | 1.1547 | When data is bounded |
| Unknown | 1.4826 | Safe default (overestimates for fat tails, conservative) |

### Cross-source verification

```rust
/// Cross-source verification configuration.
pub struct CrossSourceConfig {
    /// Minimum number of independent sources required for verification.
    /// Default: 3. With fewer sources, mark the signal as unverified.
    pub min_independent_sources: usize,

    /// Maximum acceptable MAD-normalized disagreement between sources.
    /// Default: 3.0 (sources within 3 MAD-scaled deviations agree).
    pub max_disagreement_mad: f64,

    /// Reliability weights per source (higher = more trusted).
    /// Sources with reliability < 0.3 are excluded from consensus.
    pub source_weights: HashMap<SourceId, f64>,
}

impl CrossSourceConfig {
    /// Compute reliability-weighted consensus.
    ///
    /// Each source contributes proportionally to its reliability weight.
    /// The consensus is the weighted median (robust to a single bad source).
    pub fn weighted_consensus(&self, predictions: &[(SourceId, f64)]) -> f64 {
        let filtered: Vec<(f64, f64)> = predictions.iter()
            .filter_map(|(id, v)| {
                let w = self.source_weights.get(id).copied().unwrap_or(0.5);
                if w >= 0.3 { Some((*v, w)) } else { None }
            })
            .collect();
        weighted_median(&filtered)
    }
}
```

### Red-team dreaming: strategy selection and severity

Red-team dreaming selects strategies to attack based on exposure and novelty:

```rust
/// Select strategies for red-team dreaming.
///
/// Priority order:
/// 1. Strategies with highest current exposure (most capital/attention at risk).
/// 2. Strategies that have not been red-teamed in the last 7 days.
/// 3. Strategies that survived all previous red-teams (they may have
///    undiscovered vulnerabilities).
pub fn select_red_team_targets(
    strategies: &[StrategyFragment],
    red_team_history: &HashMap<StrategyId, DateTime>,
    max_targets: usize,
) -> Vec<&StrategyFragment> {
    let mut scored: Vec<_> = strategies.iter()
        .map(|s| {
            let exposure_score = s.exposure_value();
            let staleness = red_team_history.get(&s.id)
                .map(|last| (now() - *last).as_secs_f64() / 86400.0)
                .unwrap_or(30.0); // never tested = 30 days stale
            let survived_all = s.red_team_failures == 0;
            let priority = exposure_score * staleness * if survived_all { 2.0 } else { 1.0 };
            (s, priority)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scored.into_iter().take(max_targets).map(|(s, _)| s).collect()
}
```

**Perturbation severity levels**:

| Severity | Description | Example perturbations |
|---|---|---|
| 0.0 - 0.3 | Mild | 10% price change, 2x gas, minor slippage |
| 0.3 - 0.6 | Moderate | 30% price change, 5x gas, correlation weakening |
| 0.6 - 0.8 | Severe | 50% price change, pool 50% drained, correlation breakdown |
| 0.8 - 1.0 | Extreme | 90% price crash, pool fully drained, sandwich + frontrun combo |

**Success/failure criteria**: A strategy "survives" a perturbation if its simulated PnL remains above -10% (chain domain) or its test pass rate remains above 80% (coding domain). Failure triggers confidence demotion and a Warning stored in Neuro.

### Somatic marker formation from adversarial events

When adversarial activity triggers an `AdversarialResponse`, the event feeds into the Daimon PAD vector and forms a somatic marker:

```rust
/// Map AdversarialResponse to Daimon PAD changes.
pub fn adversarial_response_to_pad(response: &AdversarialResponse) -> PadDelta {
    match response {
        AdversarialResponse::WidenIntervals(amount) => PadDelta {
            pleasure: -0.1,                // mild negative valence
            arousal: 0.2 * amount,         // proportional urgency
            dominance: -0.1,               // slight loss of control
        },
        AdversarialResponse::SuppressAction(duration) => PadDelta {
            pleasure: -0.2,
            arousal: 0.4,                  // significant urgency
            dominance: -0.3,               // loss of agency (can't act)
        },
        AdversarialResponse::EscalateToT2 => PadDelta {
            pleasure: -0.3,
            arousal: 0.6,                  // high urgency
            dominance: -0.5,               // significant loss of control
        },
        AdversarialResponse::EmitWarning(_) => PadDelta {
            pleasure: -0.15,
            arousal: 0.3,
            dominance: -0.2,
        },
    }
}

/// Form a somatic marker for an adversarial event.
///
/// The marker binds the attack pattern vector with the PAD response,
/// enabling future fast retrieval: "I've seen something like this before,
/// and it felt bad."
pub fn form_adversarial_marker(
    attack_vector: &HdcVector,
    response: &AdversarialResponse,
    codebook: &AffectCodebook,
) -> SomaticMarker {
    let pad = adversarial_response_to_pad(response);
    let affect_hv = encode_pad(pad.pleasure, pad.arousal, pad.dominance, codebook);
    let marker_hv = attack_vector.xor(&affect_hv);

    SomaticMarker {
        marker_hv,
        pattern_hv: attack_vector.clone(),
        affect_hv,
        pleasure: pad.pleasure,
        arousal: pad.arousal,
        dominance: pad.dominance,
        strength: 1.5, // adversarial markers start stronger (high-salience events)
        episode_sources: vec![],
        created_at_ms: now_ms(),
    }
}
```

### Test criteria

- **Prototype matching recall**: Against a test set of 100 known sandwich attacks, the PrototypeMatcher detects >= 90% with threshold = 0.6.
- **Prototype matching precision**: Against 1000 random (non-attack) observations, false positive rate < 5%.
- **Robust statistics breakdown**: The trimmed mean with alpha = 0.25 survives 25% contamination (estimate within 10% of true mean).
- **Hodges-Lehmann accuracy**: Subsampled estimate (n=1000 from n=10000) is within 5% of exact estimate.
- **Cross-source consensus**: When 2 of 3 sources agree and 1 is adversarial, the consensus matches the honest majority.
- **Red-team coverage**: After 10 dream cycles, every strategy with exposure > 0 has been red-teamed at least once.
- **Somatic marker formation**: After an adversarial event, a somatic marker exists with correct PAD sign (negative pleasure, positive arousal).

---

## Certified Adversarial Robustness

The methods above — prototype matching, robust statistics, red-team dreaming — are empirical defenses. They work well in practice but offer no formal guarantees. Certified adversarial robustness provides provable bounds: given a perturbation budget, the system can guarantee that its predictions will not change. This section covers three complementary certification approaches.

### Randomized Smoothing for Signal Certification

Randomized smoothing converts any base predictor into a certifiably robust one by averaging over random noise. The key insight: if a prediction is stable under random perturbations, it must also be stable under adversarial perturbations of bounded magnitude.

```rust
/// Certified robustness via randomized smoothing.
///
/// Cohen et al. (2019, ICML): Given a base classifier f, construct
/// a smoothed classifier g(x) = argmax_c P[f(x + δ) = c] where δ ~ N(0, σ²I).
///
/// Certification guarantee: If g(x) = c with probability p̄_A, then
/// g is certifiably robust within L₂ radius R = σ · Φ⁻¹(p̄_A).
///
/// For oracle predictions: this guarantees prediction stability
/// under bounded signal perturbations (market noise, sensor drift).
pub struct RandomizedSmoothing {
    /// Noise standard deviation (controls robustness-accuracy tradeoff).
    pub sigma: f64,                  // default: 0.25
    /// Number of samples for probability estimation.
    pub n_samples: usize,            // default: 1000
    /// Confidence level for Clopper-Pearson bound.
    pub confidence: f64,             // default: 0.999
}

impl RandomizedSmoothing {
    /// Certify a prediction's robustness radius.
    pub fn certify(
        &self,
        base_oracle: &dyn Oracle,
        query: &OracleQuery,
        ctx: &Context,
    ) -> CertificationResult {
        // Sample n predictions with Gaussian noise
        let mut predictions = Vec::with_capacity(self.n_samples);
        for _ in 0..self.n_samples {
            let noisy_query = add_gaussian_noise(query, self.sigma);
            predictions.push(base_oracle.predict(&noisy_query, ctx));
        }

        // Compute top class probability via Clopper-Pearson
        let (top_class, count) = most_common(&predictions);
        let p_lower = clopper_pearson_lower(count, self.n_samples, 1.0 - self.confidence);

        // Certification radius
        let radius = self.sigma * normal_quantile_inv(p_lower);

        CertificationResult {
            prediction: top_class,
            certified_radius: radius,
            confidence: self.confidence,
            n_samples: self.n_samples,
        }
    }
}
```

**Robustness-accuracy tradeoff**: Larger sigma yields larger certified radii but degrades base prediction accuracy (the noise blurs fine-grained signal features). The optimal sigma depends on the domain:

| Domain | Recommended sigma | Rationale |
|---|---|---|
| Chain (price signals) | 0.1 - 0.3 | Prices are noisy; moderate smoothing preserves trend direction |
| Coding (quality metrics) | 0.05 - 0.15 | Code metrics are more stable; less smoothing needed |
| Research (citation signals) | 0.2 - 0.5 | Citation data is sparse; heavier smoothing appropriate |

**Computational cost**: Certification requires `n_samples` forward passes through the base oracle. At 1000 samples, this is feasible for T2 (deep reasoning) frequency but too expensive for Gamma (real-time) frequency. Use randomized smoothing for high-stakes predictions where certification justifies the cost.

**Clopper-Pearson bound**: The certification uses a one-sided Clopper-Pearson confidence interval (not the normal approximation) to compute a rigorous lower bound on the top-class probability. This is critical — using a normal approximation would yield invalid certificates for small `p_lower` values.

### Lipschitz-Bounded Oracle Certification

When the oracle's prediction function has a known Lipschitz constant, certification is free at inference time — no sampling required.

```rust
/// Lipschitz-based deterministic robustness certification.
///
/// If an oracle's prediction function f has Lipschitz constant L,
/// then for any perturbation δ with ||δ|| ≤ ε:
///   ||f(x+δ) - f(x)|| ≤ L · ε
///
/// This provides zero-overhead certification at inference time
/// (no sampling required, unlike randomized smoothing).
///
/// ECLipsE (NeurIPS 2024): compositional Lipschitz estimation
/// achieves 1000x speedup over global SDP methods.
pub struct LipschitzCertifier {
    /// Estimated Lipschitz constant of the oracle.
    pub lipschitz_constant: f64,
    /// Estimation method.
    pub method: LipschitzMethod,
}

pub enum LipschitzMethod {
    /// Spectral norm: L = ∏ σ_max(W_i) for each layer.
    /// Fast but loose bound.
    SpectralNorm,
    /// ECLipsE: per-layer SDP, grows linearly with depth.
    /// (NeurIPS 2024) Tight and fast.
    Eclipse { per_layer_sdp_size: usize },
    /// Empirical: sample-based estimate (not a guarantee).
    Empirical { n_samples: usize },
}

impl LipschitzCertifier {
    /// Certify maximum output change under ε-bounded perturbation.
    pub fn certify(&self, epsilon: f64) -> f64 {
        self.lipschitz_constant * epsilon
    }

    /// Certify minimum perturbation needed to change prediction.
    pub fn minimum_adversarial_perturbation(&self, margin: f64) -> f64 {
        margin / self.lipschitz_constant
    }
}
```

**Spectral norm estimation** computes the product of the largest singular values across all layers. This is fast (one SVD per layer) but produces loose bounds because it assumes worst-case alignment across layers. In practice, the true Lipschitz constant is often 10-100x smaller than the spectral norm product.

**ECLipsE** (NeurIPS 2024) improves on spectral norm by solving a small semidefinite program (SDP) per layer and composing the results. The key insight is that compositional estimation grows linearly with network depth (not exponentially), achieving bounds that are 10-50x tighter than spectral norm at 1000x less cost than global SDP methods.

**Empirical estimation** samples random perturbations and measures the maximum observed output change. This is not a formal guarantee (it is a lower bound on L, not an upper bound), but it is useful for calibrating expectations and detecting when formal bounds are excessively loose.

**Application to oracle pipelines**: For the roko oracle pipeline, the Lipschitz constant can be estimated per stage:
- HDC encoding: L = 1 (Hamming distance is bounded by dimension)
- Scorer functions: estimate L empirically or via spectral norm if differentiable
- Gate thresholds: L = 0 (discontinuous, but the prediction itself is continuous)

The overall pipeline Lipschitz constant is the product of per-stage constants, modulo non-differentiable components which must be handled separately.

### Interval Bound Propagation for Signal Pipelines

IBP provides an alternative to Lipschitz certification that works for non-differentiable pipelines. Instead of bounding the gradient, it propagates interval bounds through each processing stage.

```rust
/// Interval bound propagation (IBP) for verifying signal processing pipelines.
///
/// IBP propagates interval bounds through each processing stage:
///   [x - ε, x + ε] → layer → [y_min, y_max]
///
/// If the output interval is unambiguous (same prediction class
/// for all values in [y_min, y_max]), the pipeline is certifiably robust.
///
/// For sequential signals: O(T · L · n²) where T = sequence length.
/// (Gowal et al., 2018; Mao et al., 2024, ICLR)
pub struct IntervalBoundPropagation {
    /// Input perturbation radius per dimension.
    pub epsilon: Vec<f64>,
    /// Propagation through each pipeline stage.
    pub stages: Vec<Box<dyn BoundPropagator + Send + Sync>>,
}

pub trait BoundPropagator: Send + Sync {
    /// Propagate interval bounds through this stage.
    fn propagate(&self, lower: &[f64], upper: &[f64]) -> (Vec<f64>, Vec<f64>);
}
```

**IBP for common operations**:

| Operation | Bound propagation rule |
|---|---|
| Linear: y = Wx + b | y_min = W⁺x_min + W⁻x_max + b, y_max = W⁺x_max + W⁻x_min + b |
| ReLU: y = max(0, x) | y_min = max(0, x_min), y_max = max(0, x_max) |
| Multiplication: y = x₁ * x₂ | y_min = min(products of all corner combinations) |
| Normalization: y = x / ||x|| | Requires interval arithmetic on the norm |

Where W⁺ = max(W, 0) and W⁻ = min(W, 0) denotes the positive and negative parts of the weight matrix.

**Tightness vs. speed**: IBP produces bounds that are fast to compute (single forward pass) but can be loose — especially for deep pipelines where interval overestimation compounds multiplicatively. For a pipeline with L stages and overestimation factor r per stage, the final interval width is O(r^L) times the true range. Mao et al. (2024) show that certified training (training specifically to minimize IBP bounds) produces tighter bounds at the cost of some nominal accuracy.

**Application to signal pipelines**: The roko signal pipeline is a natural fit for IBP because it consists of discrete stages (encoding, scoring, routing, composition) that can each implement `BoundPropagator`. The epsilon vector can be set per signal dimension based on the expected noise or adversarial perturbation budget for that dimension.

### Combining Certification Methods

The three certification methods are complementary:

| Method | Cost | Tightness | Applicability |
|---|---|---|---|
| Randomized smoothing | High (n_samples forward passes) | Tight for L₂ | Any base predictor |
| Lipschitz certification | Zero (at inference) | Depends on estimation method | Differentiable pipelines |
| IBP | Low (single forward pass) | Loose for deep pipelines | Staged pipelines with interval arithmetic |

**Recommended strategy**: Use Lipschitz certification at Gamma frequency (free, always-on monitoring of prediction stability). Use IBP at Theta frequency (cheap, periodic verification of pipeline robustness). Use randomized smoothing at Delta frequency (expensive, thorough certification of high-stakes predictions).

When methods disagree — e.g., Lipschitz says a prediction is robust but IBP finds the interval ambiguous — take the conservative answer (not robust). The disagreement itself is a signal worth logging: it may indicate that the Lipschitz bound is too loose for the specific input region.

### Citations

- Cohen, J. M., Rosenfeld, E., & Kolter, Z. (2019). "Certified Adversarial Robustness via Randomized Smoothing." *ICML 2019*. — Foundational randomized smoothing certification.
- Gowal, S., et al. (2018). "On the Effectiveness of Interval Bound Propagation for Training Verifiably Robust Models." arXiv:1810.12715. — IBP for certified training.
- Mao, Z., et al. (2024). "Understanding Certified Training with Interval Bound Propagation." *ICLR 2024*. — Analysis of IBP tightness and certified training dynamics.
- NeurIPS 2024. "ECLipsE: Efficient Compositional Lipschitz Constant Estimation for Deep Neural Networks." — Compositional Lipschitz estimation with 1000x speedup.
- Steinhardt, G., Koh, P. W., & Liang, P. S. (2017). "Certified Defenses for Data Poisoning Attacks." *NeurIPS 2017*. — Certified defenses against training-time attacks.

### Test criteria

- **Smoothing coverage**: Over 1000 certified predictions, the empirical attack success rate within the certified radius is 0. Verify by running projected gradient descent (PGD) attacks with L₂ budget equal to the certified radius and confirming zero successful attacks.
- **Lipschitz bound soundness**: No observed output change exceeds L times epsilon for any test input. Verify by sampling 10,000 random perturbations with ||delta|| = epsilon and checking that max observed output change is less than L times epsilon.
- **IBP soundness**: For all inputs in [x-epsilon, x+epsilon], the actual output falls within [y_min, y_max]. Verify by grid-sampling the input interval at 100 points per dimension (for low-dimensional inputs) and confirming containment.
- **Certification consistency**: When all three methods certify a prediction as robust at radius R, no attack within radius R succeeds. When methods disagree, the conservative (smallest radius) answer is correct.
- **Sigma sensitivity**: Increasing smoothing sigma by 2x increases the certified radius by approximately 2x (for predictions with high top-class probability). Verify on 100 test queries.

---

## Academic foundations

- Huber, P. J. (1964). "Robust Estimation of a Location Parameter." *Annals of Mathematical Statistics*, 35(1), 73-101. — Robust statistics foundations.
- Hodges, J. L., & Lehmann, E. L. (1963). "Estimates of Location Based on Rank Tests." *Annals of Mathematical Statistics*, 34(2), 598-611. — Hodges-Lehmann estimator.
- Hampel, F. R. (1974). "The Influence Curve and its Role in Robust Estimation." *JASA*, 69(346), 383-393. — Influence functions and breakdown points.
- Pearl, J. (2009). *Causality*. Cambridge University Press. — Causal consistency checking.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6). — HDC prototype matching performance.

---

## Cross-References

- See [02-chain-oracles.md](./02-chain-oracles.md) for MEV detection context
- See [04-research-oracles.md](./04-research-oracles.md) for p-hacking detection
- See [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) for HDC prototype encoding
- See [09-causal-microstructure-discovery.md](./09-causal-microstructure-discovery.md) for causal consistency checks
- See [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) for Daimon integration


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/12-somatic-ta-and-emergent-multiscale.md

# Somatic Technical Analysis and Emergent Multiscale Intelligence

> Somatic TA uses Damasio's somatic marker hypothesis to create "gut feelings" about TA patterns. Emergent multiscale intelligence measures integrated information (IIT Phi) across the TA subsystems, detecting when the whole is greater than the sum of its parts.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [06-hyperdimensional-ta](./06-hyperdimensional-ta.md) for HDC encoding, [08-adaptive-signal-metabolism](./08-adaptive-signal-metabolism.md) for signal ecosystem
**Key sources**: `bardo-backup/prd/23-ta/09-somatic-technical-analysis.md`, `bardo-backup/prd/23-ta/10-emergent-multiscale-intelligence.md`

---

## Part I: Somatic Technical Analysis

### Damasio's somatic marker hypothesis

Antonio Damasio's somatic marker hypothesis (Damasio, 1994, *Descartes' Error*) proposes that emotions are not irrational noise but fast heuristics for decision-making. When a person encounters a situation similar to one that previously had a strong outcome (good or bad), they experience a "gut feeling" — a somatic marker — that biases their decision before conscious analysis completes.

Roko implements somatic markers for TA patterns: when the agent encounters a pattern similar to one that previously led to profit or loss, it retrieves an HDC-encoded "feeling" that biases the prediction. This is System 1 cognition (Kahneman, 2011) for agents — fast, pre-analytical, and often correct.

### Somatic markers as HDC bindings

Each somatic marker binds a TA pattern vector to an affect (PAD) vector:

```rust
/// A somatic marker: an HDC binding between a pattern and an affect.
///
/// marker_hv = BIND(pattern_hv, affect_hv)
///
/// When the agent encounters a new pattern, it retrieves somatic
/// markers with high similarity to BIND(new_pattern, ?) — i.e.,
/// it finds patterns that are similar AND checks what affect they
/// are associated with.
pub struct SomaticMarker {
    /// The combined HDC vector: BIND(pattern, affect).
    pub marker_hv: HdcVector,

    /// The pattern this marker was formed from.
    pub pattern_hv: HdcVector,

    /// The affect vector (PAD encoding).
    pub affect_hv: HdcVector,

    /// PAD values for interpretability.
    pub pleasure: f64,
    pub arousal: f64,
    pub dominance: f64,

    /// Strength of the marker (decays over time, strengthened by re-experience).
    pub strength: f64,

    /// Which episodes formed this marker.
    pub episode_sources: Vec<ContentHash>,

    /// Creation timestamp.
    pub created_at_ms: i64,
}
```

### PAD encoding in HDC

The PAD (Pleasure-Arousal-Dominance) vector is encoded as an HDC vector using the same quantized codebook approach as numeric values (see [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md)):

```rust
/// Encode PAD state as an HDC vector.
///
/// Uses role-filler composition:
///   affect_hv = BUNDLE(
///       BIND(pleasure_role, pleasure_value),
///       BIND(arousal_role, arousal_value),
///       BIND(dominance_role, dominance_value),
///   )
///
/// The encoding preserves the continuous nature of PAD values
/// while enabling HDC similarity operations.
pub fn encode_pad(
    pleasure: f64,
    arousal: f64,
    dominance: f64,
    codebook: &AffectCodebook,
) -> HdcVector {
    let p_binding = codebook.pleasure_role.xor(
        &codebook.value_codebook.encode(pleasure)
    );
    let a_binding = codebook.arousal_role.xor(
        &codebook.value_codebook.encode(arousal)
    );
    let d_binding = codebook.dominance_role.xor(
        &codebook.value_codebook.encode(dominance)
    );

    HdcVector::bundle(&[p_binding, a_binding, d_binding])
}
```

The affect codebook is compatible with the Mehrabian & Russell (1974) PAD model, which provides the dimensional framework for Roko's Daimon subsystem.

### Somatic retrieval — Pre-analytical "gut feeling"

Before making a prediction, the oracle queries the somatic landscape for emotional valence of similar patterns:

```rust
/// Somatic retrieval: query "what does this pattern feel like?"
///
/// Given a new TA pattern, find somatic markers with similar patterns
/// and aggregate their affect vectors.
///
/// Cost: ~63ns per marker comparison (BIND + Hamming similarity).
/// For 1,000 markers: ~63µs.
///
/// This runs BEFORE analytical prediction — it's a fast System 1
/// heuristic that biases the subsequent System 2 analysis.
pub fn somatic_retrieval(
    pattern: &HdcVector,
    somatic_map: &[SomaticMarker],
    threshold: f64,
    contrarian_fraction: f64,  // typically 0.15 per Bower (1981)
) -> SomaticAssessment {
    // Find all markers where the pattern component is similar
    let mut matches: Vec<(f64, &SomaticMarker)> = somatic_map.iter()
        .filter_map(|marker| {
            // Unbind the affect to compare just the pattern component
            let pattern_component = marker.marker_hv.xor(&marker.affect_hv);
            let similarity = pattern.hamming_similarity(&pattern_component);
            if similarity > threshold {
                Some((similarity, marker))
            } else {
                None
            }
        })
        .collect();

    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    // Aggregate affect, weighted by similarity and marker strength
    let total_weight: f64 = matches.iter()
        .map(|(sim, m)| sim * m.strength)
        .sum();

    let avg_pleasure = matches.iter()
        .map(|(sim, m)| m.pleasure * sim * m.strength / total_weight)
        .sum::<f64>();
    let avg_arousal = matches.iter()
        .map(|(sim, m)| m.arousal * sim * m.strength / total_weight)
        .sum::<f64>();
    let avg_dominance = matches.iter()
        .map(|(sim, m)| m.dominance * sim * m.strength / total_weight)
        .sum::<f64>();

    // Mandatory 15% contrarian retrieval (Bower, 1981)
    // Retrieve markers with OPPOSITE valence to prevent echo chambers
    let contrarian_count = (matches.len() as f64 * contrarian_fraction).ceil() as usize;
    let contrarian_markers = find_contrarian_markers(
        pattern, somatic_map, avg_pleasure, contrarian_count
    );

    SomaticAssessment {
        valence: avg_pleasure,
        arousal: avg_arousal,
        dominance: avg_dominance,
        confidence: total_weight / matches.len().max(1) as f64,
        n_matching_markers: matches.len(),
        contrarian_markers,
    }
}
```

The mandatory 15% contrarian retrieval is critical. Without it, the somatic system would create an emotional echo chamber — if the agent has positive associations with a pattern, it would always retrieve positive markers, reinforcing the bias. The contrarian retrieval ensures the agent considers counterarguments, following Bower's (1981) research on mood-congruent memory bias.

### Somatic marker formation

Markers form after prediction resolution — when the oracle knows whether a pattern led to a good or bad outcome:

```rust
/// Create a somatic marker from a resolved prediction.
///
/// When a prediction resolves, the oracle knows:
/// - The pattern that was observed (HDC vector)
/// - The outcome (good/bad, encoded as PAD)
///
/// The somatic marker binds these together for future retrieval.
pub fn form_somatic_marker(
    pattern: &HdcVector,
    outcome: &PredictionAccuracy,
    current_pad: &PadState,
    codebook: &AffectCodebook,
) -> SomaticMarker {
    // Encode the affect: outcome quality modulates PAD
    let pleasure = if outcome.accuracy > 0.7 { 0.8 } else { -0.6 };
    let arousal = outcome.residual.abs();  // larger errors = more arousal
    let dominance = outcome.accuracy;  // higher accuracy = more confidence

    let affect_hv = encode_pad(pleasure, arousal, dominance, codebook);
    let marker_hv = pattern.xor(&affect_hv);  // BIND

    SomaticMarker {
        marker_hv,
        pattern_hv: pattern.clone(),
        affect_hv,
        pleasure,
        arousal,
        dominance,
        strength: 1.0,
        episode_sources: vec![outcome.prediction_id],
        created_at_ms: now_ms(),
    }
}
```

### Somatic marker decay and reinforcement

Markers weaken over time (Ebbinghaus decay) but are strengthened by re-experience:

```rust
/// Update somatic marker strength.
///
/// Decay: strength *= exp(-λt) where λ depends on marker type.
/// Reinforcement: when a similar pattern is re-encountered with
/// similar affect, strength increases.
pub fn update_marker_strength(
    marker: &mut SomaticMarker,
    elapsed_ms: i64,
    reinforcement: Option<f64>,
) {
    // Decay
    let lambda = 0.001;  // half-life ~700 ms (fast for working memory)
    let decay_factor = (-lambda * elapsed_ms as f64).exp();
    marker.strength *= decay_factor;

    // Reinforcement
    if let Some(reinforcement_strength) = reinforcement {
        marker.strength = (marker.strength + reinforcement_strength).min(5.0);
    }
}
```

---

## Part II: Emergent Multiscale Intelligence

### Integrated Information Theory (IIT) for TA

Giulio Tononi's Integrated Information Theory (Tononi, 2004; Tononi et al., 2016) proposes that consciousness arises from systems with high integrated information — measured as Phi (Φ). In Roko, we apply IIT not to measure consciousness but to measure **emergent intelligence** in the TA subsystem: when the 9 TA subsystems working together produce more insight than the sum of their individual contributions.

### The 9 TA subsystems

| # | Subsystem | What it contributes |
|---|---|---|
| 1 | HDC pattern algebra | Structural pattern encoding and cross-domain matching |
| 2 | Spectral liquidity manifolds | Riemannian geometry for execution cost modeling |
| 3 | Adaptive signal metabolism | Evolutionary signal selection and speciation |
| 4 | Causal microstructure discovery | Causal reasoning (Pearl's 3 levels) |
| 5 | Predictive geometry (TDA) | Topological constraints on trajectories |
| 6 | Resonant pattern ecosystem | Multi-signal pattern competition and evolution |
| 7 | Adversarial signal robustness | Defense against manipulation |
| 8 | Somatic technical analysis | Pre-analytical "gut feelings" |
| 9 | Predictive foraging + active inference | Prediction-resolution-calibration loop |

### Phi computation over TA subsystems

```rust
/// Compute Phi (integrated information) across the 9 TA subsystems.
///
/// Phi measures the degree to which the whole system generates more
/// information than the sum of its parts when partitioned.
///
/// For 9 subsystems, there are 2^9 - 2 = 510 possible partitions
/// (excluding the trivial empty and full partitions).
///
/// For each partition, we compute:
///   ΔI = I(whole) - I(part_A) - I(part_B)
///
/// Phi = minimum ΔI across all partitions.
///
/// This is the Minimum Information Bipartition (MIB).
pub struct PhiComputer {
    /// Current state of each TA subsystem.
    subsystem_states: [SubsystemState; 9],

    /// Information flow matrix: how much information flows
    /// from subsystem i to subsystem j.
    flow_matrix: [[f64; 9]; 9],
}

pub struct SubsystemState {
    /// Entropy of the subsystem's output distribution.
    pub entropy: f64,

    /// Mutual information with each other subsystem.
    pub mutual_info: [f64; 9],

    /// The subsystem's current prediction accuracy.
    pub accuracy: f64,
}

impl PhiComputer {
    /// Compute Phi across all 510 bipartitions.
    pub fn compute_phi(&self) -> PhiResult {
        let n = 9;
        let mut min_phi = f64::MAX;
        let mut min_partition = (0u16, 0u16);

        // Enumerate all non-trivial bipartitions
        for mask in 1..(1u16 << n) - 1 {
            let complement = ((1u16 << n) - 1) ^ mask;

            let part_a: Vec<usize> = (0..n).filter(|i| mask & (1 << i) != 0).collect();
            let part_b: Vec<usize> = (0..n).filter(|i| complement & (1 << i) != 0).collect();

            // Information generated by the whole
            let i_whole = self.integrated_information_whole();

            // Information generated by each part independently
            let i_a = self.integrated_information_part(&part_a);
            let i_b = self.integrated_information_part(&part_b);

            // Information lost by partitioning
            let delta_i = i_whole - i_a - i_b;

            if delta_i < min_phi {
                min_phi = delta_i;
                min_partition = (mask, complement);
            }
        }

        PhiResult {
            phi: min_phi,
            mib_partition: min_partition,
            interpretation: self.interpret_phi(min_phi),
        }
    }

    fn interpret_phi(&self, phi: f64) -> PhiInterpretation {
        if phi < 0.1 {
            PhiInterpretation::Modular
            // Subsystems operate independently — no emergent intelligence
        } else if phi < 0.5 {
            PhiInterpretation::WeaklyIntegrated
            // Some cross-subsystem synergy
        } else {
            PhiInterpretation::StronglyIntegrated
            // The TA system is generating insights that no subsystem could alone
        }
    }
}
```

### Minimum Information Bipartition (MIB) as diagnostic

The MIB reveals the system's weakest link — the partition that causes the least information loss:

```rust
/// The MIB diagnostic: which bipartition is the weakest link?
///
/// If the MIB separates {HDC, TDA, Somatic} from {Causal, Manifold, Adversarial, ...},
/// this tells us that the first group operates somewhat independently
/// from the second. Strengthening the connections between these
/// groups would increase Phi.
///
/// Actionable: add more cross-subsystem information flows at the MIB boundary.
pub fn diagnose_mib(phi_result: &PhiResult, subsystem_names: &[&str; 9]) -> MibDiagnosis {
    let (mask_a, mask_b) = phi_result.mib_partition;

    let group_a: Vec<String> = (0..9)
        .filter(|i| mask_a & (1 << i) != 0)
        .map(|i| subsystem_names[i].to_string())
        .collect();

    let group_b: Vec<String> = (0..9)
        .filter(|i| mask_b & (1 << i) != 0)
        .map(|i| subsystem_names[i].to_string())
        .collect();

    MibDiagnosis {
        group_a,
        group_b,
        phi: phi_result.phi,
        recommendation: format!(
            "Increase information flow between groups to raise Phi. \
             Current weakest link: Phi = {:.3}.",
            phi_result.phi
        ),
    }
}
```

### Partial Information Decomposition (PID)

PID (Williams & Beer, 2010) decomposes the information provided by multiple TA subsystems about a target variable into four components:

```rust
/// Partial Information Decomposition for TA subsystem analysis.
///
/// Given two TA subsystems S1 and S2 predicting a target T:
///
/// I(S1, S2 ; T) = Redundancy + Unique_S1 + Unique_S2 + Synergy
///
/// Redundancy: what both S1 and S2 independently know about T
/// Unique_S1: what only S1 knows about T
/// Unique_S2: what only S2 knows about T
/// Synergy: what S1 and S2 together know that neither knows alone
///
/// Synergy is the emergent intelligence: information that only
/// exists in the interaction between subsystems.
pub struct PidAnalysis {
    pub redundancy: f64,
    pub unique_s1: f64,
    pub unique_s2: f64,
    pub synergy: f64,
}

impl PidAnalysis {
    /// Compute PID for two TA subsystems predicting a target.
    pub fn compute(
        s1_predictions: &[f64],
        s2_predictions: &[f64],
        target: &[f64],
    ) -> Self {
        let i_s1_t = mutual_information(s1_predictions, target);
        let i_s2_t = mutual_information(s2_predictions, target);
        let i_s1s2_t = joint_mutual_information(s1_predictions, s2_predictions, target);

        // Williams & Beer (2010) minimum mutual information
        let redundancy = i_s1_t.min(i_s2_t);
        let unique_s1 = i_s1_t - redundancy;
        let unique_s2 = i_s2_t - redundancy;
        let synergy = i_s1s2_t - i_s1_t - i_s2_t + redundancy;

        PidAnalysis { redundancy, unique_s1, unique_s2, synergy }
    }

    /// Is there significant synergy between these subsystems?
    pub fn has_synergy(&self) -> bool {
        self.synergy > 0.05  // threshold for meaningful synergy
    }
}
```

### Synergy detection across all TA subsystem pairs

```rust
/// Detect synergistic pairs across all TA subsystems.
///
/// For each pair of subsystems, compute PID and flag pairs
/// with high synergy — these are producing emergent insights.
pub fn detect_synergies(
    subsystem_outputs: &[[f64; N]; 9],
    target: &[f64; N],
) -> Vec<SynergyPair> {
    let mut pairs = Vec::new();

    for i in 0..9 {
        for j in (i + 1)..9 {
            let pid = PidAnalysis::compute(
                &subsystem_outputs[i],
                &subsystem_outputs[j],
                target,
            );

            if pid.has_synergy() {
                pairs.push(SynergyPair {
                    subsystem_a: i,
                    subsystem_b: j,
                    synergy: pid.synergy,
                    redundancy: pid.redundancy,
                });
            }
        }
    }

    pairs.sort_by(|a, b| b.synergy.partial_cmp(&a.synergy).unwrap());
    pairs
}
```

---

## Integration: Somatic + Multiscale

Somatic markers and multiscale intelligence interact bidirectionally:

1. **Somatic → Multiscale**: Somatic markers provide fast pre-analytical biases that increase the speed of the overall TA system, reducing the information processing burden and potentially increasing Phi (by enabling faster cross-subsystem communication via affect).

2. **Multiscale → Somatic**: When the Phi computation reveals high synergy between two subsystems, somatic markers form at the boundary — encoding the "feeling" of their joint activation. This creates a fast path for future detection of similar synergistic conditions.

3. **Daimon integration**: Both somatic assessment and Phi computation feed into the Daimon PAD vector:
   - Somatic valence → Pleasure dimension
   - Phi value → Dominance dimension (high integration = high confidence)
   - Synergy detection → Arousal dimension (novel synergy = surprise)

---

## Implementation details

### PAD encoding: AffectCodebook generation

The AffectCodebook uses the same deterministic generation as other HDC codebooks (see [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md)), seeded with domain = "affect":

```rust
/// AffectCodebook for PAD encoding in HDC space.
///
/// Generated deterministically from seed "affect".
/// Three role vectors for the three PAD dimensions.
/// One shared QuantizedCodebook for value encoding (range [-1.0, 1.0]).
pub struct AffectCodebook {
    /// Role vector for the Pleasure dimension.
    pub pleasure_role: HdcVector,
    /// Role vector for the Arousal dimension.
    pub arousal_role: HdcVector,
    /// Role vector for the Dominance dimension.
    pub dominance_role: HdcVector,
    /// Shared quantized codebook for PAD values.
    /// Range: [-1.0, 1.0], n_levels: 32.
    pub value_codebook: QuantizedCodebook,
}

impl AffectCodebook {
    pub fn new(dim: usize) -> Self {
        let gen = CodebookGenerator::new("affect", dim);
        Self {
            pleasure_role: gen.generate_role(0),
            arousal_role: gen.generate_role(1),
            dominance_role: gen.generate_role(2),
            value_codebook: gen.generate_quantized(100, 32, -1.0, 1.0),
        }
    }
}
```

**Configuration parameters**:

| Parameter | Default | Range | Notes |
|---|---|---|---|
| `dim` | 10,240 | Must match global HDC dimensionality | Shared with all other codebooks. |
| `n_levels` | 32 | 16 - 64 | 32 gives ~6.25% resolution per PAD dimension. Sufficient for affect encoding. |
| `value_range` | [-1.0, 1.0] | Fixed | Matches Mehrabian-Russell PAD range. |

The quantization levels are generated via thermometer construction: each adjacent level differs by `dim / (2 * 32) = 160` bits. Two PAD states differing by 0.1 on one dimension have similarity ~0.975 on that dimension's component.

### Somatic retrieval: k-d tree in 8D

The somatic map can contain thousands of markers. Linear scan is adequate for < 5,000 markers (~315 microseconds at 63ns/comparison). For larger collections, use a k-d tree on the PAD+pattern summary space:

```rust
/// Somatic marker index for fast retrieval.
///
/// Each marker is projected into an 8-dimensional space:
///   [pleasure, arousal, dominance,         // 3 PAD dims
///    pattern_pca_0, ..., pattern_pca_4]    // 5 PCA dims of pattern vector
///
/// The PCA projection compresses the 10,240-bit pattern vector into
/// 5 f64 dimensions that capture the most variance. This loses some
/// information but enables tree-based spatial indexing.
///
/// Build cost: O(n * log(n)) where n = marker count.
/// Query cost: O(log(n)) average, O(n) worst case (high-dimensional curse).
/// For n = 10,000 markers: ~10x faster than linear scan.
pub struct SomaticIndex {
    /// k-d tree over the 8D projection space.
    tree: KdTree<f64, usize, 8>,

    /// PCA projection matrix (10,240 -> 5 dimensions).
    pca_matrix: [[f64; 5]; 10_240],

    /// The underlying markers (indexed by position in this vec).
    markers: Vec<SomaticMarker>,
}

impl SomaticIndex {
    /// Build the index from a collection of markers.
    pub fn build(markers: Vec<SomaticMarker>) -> Self {
        let pca_matrix = compute_pca_projection(&markers, 5);
        let mut tree = KdTree::new(8);

        for (idx, marker) in markers.iter().enumerate() {
            let point = Self::project(marker, &pca_matrix);
            tree.add(point, idx).unwrap();
        }

        Self { tree, pca_matrix, markers }
    }

    /// Query: find k nearest markers to a pattern + PAD query.
    pub fn query_nearest(
        &self,
        pattern: &HdcVector,
        pad_hint: (f64, f64, f64),
        k: usize,
    ) -> Vec<(f64, &SomaticMarker)> {
        let query_point = self.project_query(pattern, pad_hint);
        self.tree.nearest(&query_point, k, &squared_euclidean)
            .unwrap()
            .into_iter()
            .map(|(dist, &idx)| (dist.sqrt(), &self.markers[idx]))
            .collect()
    }
}
```

**Distance metric**: Squared Euclidean in the 8D projection space. The PAD dimensions and PCA dimensions are on different scales, so normalize each dimension to unit variance before building the tree.

**Unbinding operation**: To compare just the pattern component of a somatic marker (ignoring affect), unbind by XORing the marker vector with its affect vector: `pattern_component = marker_hv XOR affect_hv`. This recovers the approximate pattern vector (XOR is its own inverse in BSC).

### Phi computation: information flow matrix

The 9x9 information flow matrix `flow_matrix[i][j]` measures how much information flows from subsystem i to subsystem j:

```rust
/// Compute the information flow matrix across TA subsystems.
///
/// Method: for each pair (i, j), compute the transfer entropy
/// from subsystem i's output time series to subsystem j's output
/// time series over the last window_size observations.
///
/// Transfer entropy T(i -> j) measures the reduction in uncertainty
/// about j's next state when knowing i's past states, beyond what
/// j's own past provides.
///
///   T(i->j) = H(j_t | j_{t-1..t-k}) - H(j_t | j_{t-1..t-k}, i_{t-1..t-k})
///
/// where H is conditional entropy and k is the lag order.
pub struct FlowMatrixComputer {
    /// Number of lag steps for transfer entropy.
    pub lag_order: usize,          // default: 3
    /// Observation window size.
    pub window_size: usize,        // default: 100
    /// Number of histogram bins for entropy estimation.
    pub n_bins: usize,             // default: 10
}

impl FlowMatrixComputer {
    pub fn compute(
        &self,
        subsystem_outputs: &[[f64]; 9],
    ) -> [[f64; 9]; 9] {
        let mut flow = [[0.0; 9]; 9];
        for i in 0..9 {
            for j in 0..9 {
                if i != j {
                    flow[i][j] = transfer_entropy(
                        &subsystem_outputs[i],
                        &subsystem_outputs[j],
                        self.lag_order,
                        self.n_bins,
                    );
                }
            }
        }
        flow
    }
}
```

**Temporal lag model**: Transfer entropy uses lag_order = 3 by default (looks 3 time steps back). At Theta frequency (~75s), this covers ~225s of history. For subsystems that communicate at different speeds (e.g., HDC is instantaneous, TDA requires batch computation), the lag order should be adjusted per pair.

### Minimum information bipartition: algorithm for n = 9

With 9 subsystems, there are `2^9 - 2 = 510` non-trivial bipartitions. This is small enough for exhaustive enumeration:

```rust
/// Enumerate all 510 bipartitions and find the MIB.
///
/// For each bipartition (A, B):
///   1. Compute I(whole) = sum of all transfer entropies in the flow matrix.
///   2. Compute I(A) = sum of transfer entropies within subsystems in A.
///   3. Compute I(B) = sum of transfer entropies within subsystems in B.
///   4. delta_I = I(whole) - I(A) - I(B).
///   5. Track the bipartition with minimum delta_I.
///
/// Cost: 510 iterations, each O(81) operations on the flow matrix.
/// Total: ~41K arithmetic operations. Negligible (< 1ms).
pub fn find_mib(flow_matrix: &[[f64; 9]; 9]) -> (u16, u16, f64) {
    let n = 9;
    let i_whole: f64 = flow_matrix.iter().flat_map(|row| row.iter()).sum();
    let mut min_delta = f64::MAX;
    let mut min_mask = (0u16, 0u16);

    for mask in 1u16..(1 << n) - 1 {
        let complement = ((1u16 << n) - 1) ^ mask;

        let i_a: f64 = (0..n).flat_map(|i| (0..n).map(move |j| (i, j)))
            .filter(|(i, j)| mask & (1 << i) != 0 && mask & (1 << j) != 0)
            .map(|(i, j)| flow_matrix[i][j])
            .sum();

        let i_b: f64 = (0..n).flat_map(|i| (0..n).map(move |j| (i, j)))
            .filter(|(i, j)| complement & (1 << i) != 0 && complement & (1 << j) != 0)
            .map(|(i, j)| flow_matrix[i][j])
            .sum();

        let delta = i_whole - i_a - i_b;
        if delta < min_delta {
            min_delta = delta;
            min_mask = (mask, complement);
        }
    }

    (min_mask.0, min_mask.1, min_delta)
}
```

**Scalability note**: For n = 9, exhaustive enumeration is trivial. For n > 20, the number of bipartitions exceeds 10^6 and heuristic search (e.g., spectral bisection on the flow matrix) becomes necessary. This is not an issue for the current 9-subsystem architecture.

### Partial information decomposition: algorithm and bias correction

The PID implementation uses the Williams-Beer I_min (minimum specific information) approach:

```rust
/// PID computation using Williams-Beer I_min.
///
/// For two sources S1, S2 and target T:
///   Redundancy = I_min(S1; T) where I_min is the minimum specific info.
///   I_min is computed over all realizations t of T:
///     I_min(S1, S2; T) = sum_t p(t) * min(I_spec(S1; t), I_spec(S2; t))
///   where I_spec(S; t) = sum_s p(s|t) * log(p(s|t) / p(s)).
///
/// Sample size requirement: at least 5 * n_bins^2 observations
/// to avoid severe estimation bias.
pub struct PidConfig {
    /// Number of histogram bins per variable.
    pub n_bins: usize,         // default: 5
    /// Minimum sample size: 5 * n_bins^2 = 125 with default.
    pub min_samples: usize,    // derived: 5 * n_bins * n_bins
    /// Bias correction method.
    pub bias_correction: BiasCorrection,
}

pub enum BiasCorrection {
    /// No correction (raw plugin estimator).
    None,
    /// Miller-Madow correction: subtract (|alphabet| - 1) / (2 * n).
    MillerMadow,
    /// Jackknife resampling: leave-one-out estimate of bias.
    /// More accurate but O(n) times more expensive.
    Jackknife,
}
```

**Recommended settings**: Use `n_bins = 5` and `MillerMadow` bias correction for routine monitoring. Switch to `Jackknife` for publication-quality Phi/PID estimates. The Miller-Madow correction subtracts `(k - 1) / (2n)` from each entropy estimate, where k is the number of non-empty bins and n is the sample size.

### State machine: Phi/somatic markers feeding Daimon updates

The Phi computation and somatic assessment update the Daimon on a schedule:

```
THETA TICK (every ~75s):
  1. Evaluate somatic assessment for the current TA state.
     -> If somatic valence is strong (|pleasure| > 0.5):
        Update Daimon.pleasure += 0.3 * somatic_pleasure.
  2. No Phi computation (too expensive for Theta frequency).

DELTA TICK (every few hours):
  1. Compute the 9x9 information flow matrix from recent Theta outputs.
  2. Find MIB and compute Phi.
  3. Compute PID for all 36 subsystem pairs.
  4. Update Daimon:
     - Phi > 0.5 -> Daimon.dominance += 0.2 (high integration = high confidence)
     - New synergy detected (PID synergy > 0.1 for a pair that was
       previously < 0.05) -> Daimon.arousal += 0.3 (surprise)
  5. Form somatic markers at synergistic boundaries:
     - For each high-synergy pair (i, j), encode the joint activation
       pattern and bind it with the positive affect of discovery.
     - These markers enable fast future detection of similar synergistic
       conditions at Theta frequency (avoiding the expensive Phi computation).
  6. Log Phi value and MIB partition to .roko/learn/phi.jsonl.
```

**Computation frequency**: Phi is computed at Delta frequency only. At Theta, somatic markers serve as fast proxies for the Phi-derived state. This two-speed design keeps Theta ticks cheap while still incorporating multiscale intelligence insights.

### Error handling

- **Empty somatic map**: If no markers exist, somatic retrieval returns a neutral assessment (pleasure = 0.0, arousal = 0.0, dominance = 0.0, confidence = 0.0).
- **All markers expired**: Same as empty map. Log a warning suggesting that either the system is too new or the decay rate is too aggressive.
- **Zero weight in somatic aggregation**: If total weight is zero (all matched markers have zero strength), return neutral assessment.
- **Degenerate flow matrix**: If all transfer entropies are zero (no inter-subsystem communication), Phi = 0 and MIB is arbitrary. This indicates the subsystems are operating independently.
- **Insufficient data for PID**: If sample count < `min_samples`, skip PID computation and report `synergy = NaN` with a warning.
- **PCA failure in somatic index**: If the marker set has fewer than 5 unique patterns, reduce PCA dimensions to match. If fewer than 2, fall back to linear scan.

### Test criteria

- **PAD encoding round-trip**: Encode PAD (0.5, -0.3, 0.8) as HDC vector, then decode by unbinding each role. The decoded values should be within 0.1 of the originals (limited by quantization).
- **Somatic retrieval correctness**: Store a marker for pattern A with positive pleasure. Query with a pattern similar to A. The assessment should have positive valence.
- **Contrarian retrieval**: Somatic retrieval always returns at least `ceil(n_matches * 0.15)` contrarian markers when available.
- **Phi monotonicity**: Adding a strong inter-subsystem connection (increasing one flow_matrix entry by 1.0) does not decrease Phi.
- **MIB exhaustiveness**: For n = 9, exactly 510 bipartitions are evaluated.
- **PID non-negativity**: Redundancy, unique_s1, unique_s2 are all >= 0. Synergy can be negative (indicates suppression).
- **State machine scheduling**: Phi is never computed at Theta frequency. Somatic assessment is computed at every Theta tick.

---

## Academic foundations

- Damasio, A. R. (1994). *Descartes' Error: Emotion, Reason, and the Human Brain*. Putnam. — Somatic marker hypothesis.
- Mehrabian, A., & Russell, J. A. (1974). *An Approach to Environmental Psychology*. MIT Press. — PAD model.
- Bower, G. H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129-148. — Mood-congruent memory (15% contrarian retrieval).
- Kahneman, D. (2011). *Thinking, Fast and Slow*. Farrar, Straus and Giroux. — System 1/System 2 dual-process theory.
- Tononi, G. (2004). "An information integration theory of consciousness." *BMC Neuroscience*, 5(42). — IIT Phi.
- Tononi, G., Boly, M., Massimini, M., & Koch, C. (2016). "Integrated information theory." *Nature Reviews Neuroscience*, 17(7), 450-461. — IIT 3.0.
- Williams, P. L., & Beer, R. D. (2010). "Nonnegative decomposition of multivariate information." arXiv:1004.2515. — PID framework.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6). — HDC for somatic marker encoding.

---

## Cross-References

- See [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) for HDC encoding of somatic markers
- See [08-adaptive-signal-metabolism.md](./08-adaptive-signal-metabolism.md) for the signal ecosystem that somatic markers modulate
- See [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) for resonant patterns interacting with somatic markers
- See [11-adversarial-signal-robustness.md](./11-adversarial-signal-robustness.md) for adversarial robustness feeding somatic markers


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/13-predictive-foraging-and-active-inference.md

# Predictive Foraging and Active Inference

> Every knowledge retrieval is a falsifiable prediction. The CalibrationTracker corrects biases at ~50ns per correction. Active inference (factorized discrete POMDP with 90 states) drives context selection via Expected Free Energy. This is the complete prediction-resolution-calibration loop.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [01-oracle-trait](./01-oracle-trait.md) for Oracle trait, [00-vision-ta-generalized](./00-vision-ta-generalized.md) for the universal prediction vision
**Key sources**: `refactoring-prd/09-innovations.md` §VII, §XIX.A-C, `bardo-backup/tmp/agent-chain/10-predictive-foraging.md`, `tmp/implementation-plans/modelrouting/12-advanced-patterns.md`

---

## Predictive foraging — The core loop

Predictive foraging transforms every agent action into a learning opportunity. Before acting, the agent makes a falsifiable prediction about the outcome. After acting, the prediction is compared to reality. The difference (residual) feeds an arithmetic corrector that improves future predictions. This loop costs ~50 nanoseconds per correction — pure arithmetic, no LLM.

```
1. PREDICT    → Oracle.predict(query, ctx) → Prediction
2. ACT        → Agent.execute(action) → output
3. VERIFY     → Gate.verify(output) → Engram (ground truth)
4. RESOLVE    → Oracle.evaluate(prediction, outcome) → PredictionAccuracy
5. CORRECT    → ResidualCorrector.update(model, category, residual) → adjusted bias
6. CALIBRATE  → CalibrationTracker.update(model, category, accuracy) → updated stats
7. FEEDBACK   → Router.feedback(model, accuracy) → updated bandit arms
8. LEARN      → Neuro.store(pattern) → knowledge entry
```

Steps 1-4 are the prediction lifecycle (see [01-oracle-trait.md](./01-oracle-trait.md)). Steps 5-8 are the learning loop that makes predictions improve over time.

### PredictionClaim — The falsifiable commitment

```rust
/// A PredictionClaim is an Engram that commits the agent to a
/// specific prediction about a specific outcome.
///
/// The claim is stored BEFORE action execution. This makes it
/// impossible for the agent to "retrodict" — to claim after the
/// fact that it predicted the right outcome.
///
/// The claim structure:
///   "I predict that [metric] will be [value] with [confidence]
///    in [horizon], and this prediction is based on [lineage]."
pub struct PredictionClaim {
    /// The Engram that stores this claim.
    pub engram: Engram,

    /// The prediction (from Oracle.predict()).
    pub prediction: Prediction,

    /// When the claim was registered (before action execution).
    pub registered_at_ms: i64,

    /// Status: Pending | Resolved(accuracy) | Expired.
    pub status: ClaimStatus,
}

pub enum ClaimStatus {
    Pending,
    Resolved(PredictionAccuracy),
    Expired,
}
```

### ResidualCorrector — ~50ns per correction

The ResidualCorrector is the workhorse of predictive foraging. It maintains per-(model, task_category) bias estimates and corrects raw predictions:

```rust
/// ResidualCorrector: fast bias elimination.
///
/// For each (model, category) pair, tracks the exponential moving
/// average of prediction residuals (predicted - actual).
///
/// Correction: adjusted = raw - mean_bias(model, category)
///
/// Cost: ~50 nanoseconds per correction.
///   - HashMap lookup: ~20ns
///   - EMA update: ~10ns
///   - Subtraction: ~1ns
///   - Cache overhead: ~19ns
///
/// At 1,000 predictions/day/agent: 50µs total daily cost for corrections.
pub struct ResidualCorrector {
    biases: DashMap<(String, String), ExponentialMovingAverage>,
    alpha: f64,  // EMA smoothing factor (typically 0.1)
}

impl ResidualCorrector {
    pub fn new(alpha: f64) -> Self {
        Self {
            biases: DashMap::new(),
            alpha,
        }
    }

    /// Correct a raw prediction by subtracting estimated bias.
    pub fn correct(&self, model: &str, category: &str, raw_value: f64) -> f64 {
        let key = (model.to_string(), category.to_string());
        match self.biases.get(&key) {
            Some(ema) => raw_value - ema.current(),
            None => raw_value,  // no correction data yet
        }
    }

    /// Update bias estimate with a new residual observation.
    pub fn update(&self, model: &str, category: &str, residual: f64) {
        let key = (model.to_string(), category.to_string());
        self.biases
            .entry(key)
            .or_insert_with(|| ExponentialMovingAverage::new(self.alpha))
            .value_mut()
            .update(residual);
    }

    /// Get the current bias estimate for a (model, category) pair.
    pub fn bias(&self, model: &str, category: &str) -> f64 {
        let key = (model.to_string(), category.to_string());
        self.biases.get(&key)
            .map(|ema| ema.current())
            .unwrap_or(0.0)
    }
}
```

### CalibrationTracker — Per-(model, category) accuracy

```rust
/// CalibrationTracker aggregates prediction accuracy statistics.
///
/// Tracks per-(model, task_category):
/// - Mean residual (bias)
/// - Mean absolute error (accuracy)
/// - Interval calibration (fraction of outcomes within prediction intervals)
/// - Accuracy trend (improving or degrading)
///
/// On-chain (Korai): shared across all agents in the collective.
/// A new agent importing the collective calibration starts with
/// pre-learned biases — this is the mechanism behind the 31.6x
/// faster calibration heuristic.
pub struct CalibrationTracker {
    stats: DashMap<(String, String), CalibrationStats>,
}

pub struct CalibrationStats {
    pub mean_residual: ExponentialMovingAverage,
    pub mean_absolute_error: ExponentialMovingAverage,
    pub interval_coverage: ExponentialMovingAverage,
    pub count: u64,
    pub trend: TrendEstimator,
}

impl CalibrationTracker {
    /// Update calibration with a resolved prediction.
    pub fn update(&self, accuracy: &PredictionAccuracy) {
        let key = (
            accuracy.domain.to_string(),
            accuracy.category.clone(),
        );

        self.stats.entry(key)
            .or_insert_with(|| CalibrationStats::new(alpha: 0.1))
            .value_mut()
            .update(accuracy);
    }

    /// Get calibrated confidence for a (model, category) pair.
    /// This is used to adjust raw Oracle confidence values.
    pub fn calibrated_confidence(&self, model: &str, category: &str) -> f64 {
        let key = (model.to_string(), category.to_string());
        self.stats.get(&key)
            .map(|s| 1.0 - s.mean_absolute_error.current())
            .unwrap_or(0.5)  // prior: 50% confidence when no data
    }

    /// Get the accuracy trend (positive = improving).
    pub fn accuracy_trend(&self, model: &str, category: &str) -> f64 {
        let key = (model.to_string(), category.to_string());
        self.stats.get(&key)
            .map(|s| s.trend.slope())
            .unwrap_or(0.0)
    }

    /// Export calibration data for on-chain publishing (Korai).
    pub fn export(&self) -> CollectiveCalibration {
        let entries: Vec<_> = self.stats.iter()
            .map(|entry| {
                let ((model, category), stats) = entry.pair();
                CalibrationEntry {
                    model: model.clone(),
                    category: category.clone(),
                    mean_bias: stats.mean_residual.current(),
                    mean_absolute_error: stats.mean_absolute_error.current(),
                    interval_coverage: stats.interval_coverage.current(),
                    count: stats.count,
                }
            })
            .collect();

        CollectiveCalibration { entries }
    }

    /// Import collective calibration from Korai.
    pub fn import(&self, collective: &CollectiveCalibration) {
        for entry in &collective.entries {
            let key = (entry.model.clone(), entry.category.clone());
            self.stats.entry(key)
                .or_insert_with(|| CalibrationStats::from_collective(entry));
        }
    }
}
```

---

## Active inference — Expected Free Energy

Active inference (Friston, 2010, *Nature Reviews Neuroscience*) provides the theoretical foundation for how agents select actions (including what context to retrieve). The agent maintains an internal generative model of the world and selects actions that minimize Expected Free Energy (EFE).

### Factorized discrete POMDP — The state space

The agent's state space is a factorized discrete Partially Observable Markov Decision Process with 6 × 5 × 3 = 90 states:

```rust
/// Factorized discrete POMDP for active inference.
///
/// The state space factors into three independent dimensions:
///   - Task complexity (6 levels): trivial, simple, moderate, complex, expert, research
///   - Information state (5 levels): blind, partial, adequate, comprehensive, complete
///   - Confidence state (3 levels): low, medium, high
///
/// Total: 6 × 5 × 3 = 90 discrete states.
///
/// This factorization reduces the state space exponentially compared
/// to a flat representation (which would need to enumerate all
/// possible combinations of continuous variables).
pub struct ActiveInferenceState {
    /// Current beliefs about the state (probability distribution over 90 states).
    pub beliefs: Array3<f64>,  // shape: [6, 5, 3]

    /// Generative model matrices (see below).
    pub model: GenerativeModel,
}

/// The four matrices of the generative model.
pub struct GenerativeModel {
    /// A matrix: observation likelihood P(o | s).
    /// "Given state s, what observations would I expect?"
    /// Shape: [n_observations, 6, 5, 3]
    pub a: Array4<f64>,

    /// B matrix: transition dynamics P(s' | s, a).
    /// "Given state s and action a, what state will I be in next?"
    /// One matrix per action.
    /// Shape: [n_actions][6, 5, 3, 6, 5, 3]
    pub b: Vec<Array6<f64>>,

    /// C matrix: preferred observations (goal).
    /// "What observations do I want to see?"
    /// Encodes: high task success, high confidence, complete information.
    /// Shape: [n_observations]
    pub c: Array1<f64>,

    /// D matrix: initial state prior.
    /// "What state do I believe I start in?"
    /// Shape: [6, 5, 3]
    pub d: Array3<f64>,
}
```

### EFE decomposition for context selection

Expected Free Energy decomposes into three terms that drive context selection in the VCG attention auction:

```rust
/// Expected Free Energy (EFE) for action evaluation.
///
/// G(π) = pragmatic_value + epistemic_value - ambiguity
///
/// where:
///   pragmatic_value = E[ln P(o_desired | s_π)] — expected goal achievement
///   epistemic_value = E[H(s | o_π) - H(s | o_π, θ)] — expected information gain
///   ambiguity = H(o | s_π) — expected observation noise
///
/// Lower EFE → better action.
/// The agent selects actions (including context retrieval actions)
/// that minimize EFE.
pub fn expected_free_energy(
    beliefs: &Array3<f64>,
    action: usize,
    model: &GenerativeModel,
) -> EfeDecomposition {
    // Predicted state after action
    let predicted_state = apply_transition(beliefs, action, &model.b[action]);

    // Pragmatic value: how much does this action achieve the goal?
    let pragmatic = compute_pragmatic_value(&predicted_state, &model.c, &model.a);

    // Epistemic value: how much information does this action provide?
    let epistemic = compute_epistemic_value(&predicted_state, beliefs, &model.a);

    // Ambiguity: how noisy are the expected observations?
    let ambiguity = compute_ambiguity(&predicted_state, &model.a);

    EfeDecomposition {
        total: pragmatic + epistemic - ambiguity,
        pragmatic,
        epistemic,
        ambiguity,
    }
}

pub struct EfeDecomposition {
    /// Total EFE (lower = better).
    pub total: f64,
    /// Goal achievement (higher = more goal-directed).
    pub pragmatic: f64,
    /// Information gain (higher = more exploratory).
    pub epistemic: f64,
    /// Observation noise (lower = less ambiguous).
    pub ambiguity: f64,
}
```

### Context foraging stopping rule — Charnov's MVT

The agent forages for context (retrieves Engrams to fill the context window) and must decide when to stop. Charnov's marginal value theorem (Charnov, 1976, *Theoretical Population Biology*) provides the optimal stopping rule:

```rust
/// Context foraging stopping rule based on Charnov's MVT.
///
/// Stop retrieving context when the marginal information gain
/// of the next retrieval drops below the average gain rate
/// across all context patches (domains/topics).
///
/// gain_rate = total_information_gained / total_tokens_spent
///
/// Retrieve next item if:
///   marginal_gain(next_item) > gain_rate × marginal_cost(next_item)
///
/// This naturally balances breadth (exploring many topics) vs.
/// depth (going deep on one topic) based on the current
/// information landscape.
pub struct ContextForager {
    /// Current information gain rate (running average).
    gain_rate: ExponentialMovingAverage,

    /// Per-domain context patches.
    patches: HashMap<String, ContextPatch>,

    /// Token budget remaining.
    budget_remaining: usize,
}

pub struct ContextPatch {
    /// Domain/topic identifier.
    pub id: String,
    /// Items available in this patch.
    pub items: Vec<Engram>,
    /// Estimated information gain per item (decreasing as more items are retrieved).
    pub marginal_gain: f64,
    /// Cost per item (tokens).
    pub item_cost: usize,
    /// Items already retrieved from this patch.
    pub retrieved: usize,
}

impl ContextForager {
    /// Decide whether to continue foraging or stop.
    pub fn should_continue(&self) -> bool {
        // Find the best next item across all patches
        let best_patch = self.patches.values()
            .max_by(|a, b| {
                let ratio_a = a.marginal_gain / a.item_cost as f64;
                let ratio_b = b.marginal_gain / b.item_cost as f64;
                ratio_a.partial_cmp(&ratio_b).unwrap()
            });

        match best_patch {
            Some(patch) => {
                let marginal_ratio = patch.marginal_gain / patch.item_cost as f64;
                marginal_ratio > self.gain_rate.current()
                    && self.budget_remaining > patch.item_cost
            }
            None => false,
        }
    }

    /// Select the next item to retrieve.
    pub fn select_next(&mut self) -> Option<(String, Engram)> {
        if !self.should_continue() {
            return None;
        }

        // Select from the patch with highest marginal gain/cost ratio
        let best_id = self.patches.values()
            .max_by(|a, b| {
                let ratio_a = a.marginal_gain / a.item_cost as f64;
                let ratio_b = b.marginal_gain / b.item_cost as f64;
                ratio_a.partial_cmp(&ratio_b).unwrap()
            })
            .map(|p| p.id.clone())?;

        let patch = self.patches.get_mut(&best_id)?;
        let item = patch.items.get(patch.retrieved)?.clone();

        // Update state
        patch.retrieved += 1;
        patch.marginal_gain *= 0.8;  // diminishing returns
        self.budget_remaining -= patch.item_cost;
        self.gain_rate.update(patch.marginal_gain);

        Some((best_id, item))
    }
}
```

### EFE as VCG bid

The EFE decomposition feeds directly into the VCG attention auction:

```rust
/// Convert EFE score into a VCG auction bid.
///
/// Higher epistemic value → higher bid (the agent WANTS to know this)
/// Higher pragmatic value → higher bid (this helps achieve the goal)
/// Higher ambiguity → lower bid (noisy information is less valuable)
///
/// Modulated by Daimon PAD state:
/// - High arousal → urgency multiplier on pragmatic bids
/// - Low dominance → boost epistemic bids (need more information)
/// - Low pleasure → boost iteration memory bids (learn from failures)
pub fn efe_to_bid(
    efe: &EfeDecomposition,
    pad: &PadState,
    section_type: &str,
) -> f64 {
    let urgency = pad.arousal.max(0.1);
    let exploration = 1.0 - pad.dominance;
    let failure_boost = (1.0 - pad.pleasure).max(0.0);

    let base_bid = match section_type {
        "prediction_context" => efe.epistemic * (1.0 + exploration),
        "task_context" => efe.pragmatic * urgency,
        "failure_memory" => efe.pragmatic * failure_boost,
        "knowledge" => efe.epistemic * exploration,
        _ => efe.total,
    };

    base_bid.max(0.0)
}
```

---

## Thompson Sampling for oracle selection

When multiple oracle implementations are available for the same query, Thompson Sampling (Thompson, 1933) selects which oracle to use:

```rust
/// Thompson Sampling for oracle selection.
///
/// Each oracle maintains a Beta distribution modeling its accuracy:
///   Beta(α_success, β_failure)
///
/// To select an oracle:
///   1. Sample from each oracle's Beta distribution
///   2. Select the oracle with the highest sample
///
/// This naturally balances exploration (trying less-used oracles)
/// with exploitation (preferring proven oracles).
pub struct ThompsonOracleSelector {
    arms: HashMap<String, ThompsonArm>,
}

pub struct ThompsonArm {
    /// Oracle identifier.
    pub oracle_id: String,

    /// Success count (predictions with accuracy > threshold).
    pub alpha: f64,

    /// Failure count (predictions with accuracy <= threshold).
    pub beta: f64,
}

impl ThompsonArm {
    pub fn new(oracle_id: String) -> Self {
        Self {
            oracle_id,
            alpha: 1.0,  // prior: Beta(1,1) = uniform
            beta: 1.0,
        }
    }

    /// Sample from the Beta distribution.
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        Beta::new(self.alpha, self.beta)
            .unwrap()
            .sample(rng)
    }

    /// Update after observing a prediction outcome.
    pub fn update(&mut self, accuracy: f64, threshold: f64) {
        if accuracy > threshold {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }
}

impl ThompsonOracleSelector {
    /// Select the best oracle for a given query.
    pub fn select(&self, rng: &mut impl Rng) -> &str {
        self.arms.values()
            .max_by(|a, b| {
                a.sample(rng).partial_cmp(&b.sample(rng)).unwrap()
            })
            .map(|arm| arm.oracle_id.as_str())
            .unwrap()
    }
}
```

For non-stationary environments (where oracle quality changes over time), the f-dsw (fixed-share with discounting) variant of Thompson Sampling (Raj & Kalyani, 2017) is used. This adds a discount factor that gradually forgets old observations, allowing the selector to track changing oracle quality.

---

## Collective calibration on Korai

The full predictive foraging loop extends to the collective via Korai:

```
Individual agent:
  Predict → Act → Verify → Correct → Calibrate

Collective:
  Agent A publishes calibration → Korai ISFR
  Agent B imports calibration → starts with pre-learned biases
  Agent B's corrections refine the collective calibration
  → Published back to Korai
  → Next agent starts even better
```

The collective calibration heuristic (1/sqrt(N×t), see `refactoring-prd/09-innovations.md` §VI) projects that with N=1,000 agents, a new agent reaches ~82% accuracy in 3 days instead of 3 months. This is a theoretical upper bound under the independence assumption — actual speedup depends on agent correlation and domain shift.

---

## Three cognitive speeds in the prediction loop

| Speed | Prediction activity |
|---|---|
| **Gamma** (~5-15s) | T0 probes evaluate prediction error scalar. No prediction resolution. Cost: µs. |
| **Theta** (~75s) | Pending predictions resolved. Residuals computed. CalibrationTracker updated. EMA thresholds adjusted. |
| **Delta** (hours) | Cross-model calibration analysis. Thompson Sampling arms updated. Collective calibration published to Korai. Predictive strategy fragments consolidated in Dreams. |

---

## Academic foundations

- Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127-138. — Active inference framework.
- Charnov, E. L. (1976). "Optimal foraging: the marginal value theorem." *Theoretical Population Biology*, 9, 129-136. — Context foraging stopping rule.
- Pirolli, P., & Card, S. (1999). "Information foraging." *Psychological Review*, 106(4), 643-675. — Information foraging theory.
- Thompson, W. R. (1933). "On the Likelihood that One Unknown Probability Exceeds Another." *Biometrika*, 25(3-4), 285-294. — Thompson Sampling.
- Raj, V., & Kalyani, S. (2017). "Taming Non-stationary Bandits: A Bayesian Approach." arXiv:1707.09727. — f-dsw Thompson Sampling for non-stationary environments.
- Conant, R. C., & Ashby, W. R. (1970). "Every good regulator of a system must be a model of that system." *International Journal of Systems Science*, 1(2), 89-97. — Good Regulator Theorem.
- Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176. — Cascade routing for cost-effective prediction.
- Lee, S., et al. (2026). "Meta-Harness." arXiv:2603.28052. — Harness optimization.

---

## Cross-References

- See [01-oracle-trait.md](./01-oracle-trait.md) for the Oracle trait that predictions use
- See [00-vision-ta-generalized.md](./00-vision-ta-generalized.md) for the vision of universal oracle primitives
- See [02-chain-oracles.md](./02-chain-oracles.md) and [03-coding-oracles.md](./03-coding-oracles.md) for domain-specific prediction examples
- See topic [05-learning](../05-learning/INDEX.md) for CascadeRouter and bandit integration
- See topic [06-neuro](../06-neuro/INDEX.md) for knowledge tier progression from prediction outcomes


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/14-sheaf-tropical-geometry.md

# Sheaf-Theoretic Consistency and Tropical Decision Geometry

> Sheaf theory provides local-to-global consistency guarantees across distributed oracle subsystems. Tropical geometry reveals the piecewise-linear decision boundaries of oracle policies and connects symbolic planning (dynamic programming) with neural computation via the max-plus semiring.


> **Implementation**: Specified

**Topic**: [Technical Analysis](./INDEX.md)
**Prerequisites**: [06-hyperdimensional-ta](./06-hyperdimensional-ta.md) for HDC encoding, [07-spectral-liquidity-manifolds](./07-spectral-liquidity-manifolds.md) for Riemannian geometry, [12-somatic-ta-and-emergent-multiscale](./12-somatic-ta-and-emergent-multiscale.md) for IIT Phi
**Key sources**: Hansen & Ghrist (2019), Bodnar et al. (2022), Zhang et al. (2018, ICML)

---

## Part I: Sheaf Theory for Oracle Consistency

### Why sheaves for distributed oracles

Roko's 9 TA subsystems (HDC patterns, spectral manifolds, causal discovery, TDA, signal metabolism, adversarial robustness, somatic markers, active inference, resonant patterns) each produce predictions that must be **locally consistent** — the chain oracle's price prediction should cohere with the liquidity manifold's execution cost estimate, which should cohere with the causal model's structural equations.

Sheaf theory (Bredon, 1997; Curry, 2014) provides the mathematical framework for exactly this problem: ensuring local consistency implies global consistency. A cellular sheaf assigns a vector space to each subsystem (its "prediction space") and linear maps between adjacent subsystems (their "consistency constraints"). When the sheaf has vanishing cohomology, local consistency implies global consistency — the system's predictions are guaranteed to be mutually compatible.

This is the mathematical formalization of what IIT Phi (doc 12) measures empirically: the degree to which subsystem predictions form a coherent whole. Sheaf cohomology replaces the brute-force 510-bipartition enumeration with a principled algebraic characterization.

### Cellular sheaves on the oracle graph

```rust
/// A cellular sheaf over the oracle subsystem graph.
///
/// The graph G has:
/// - Vertices v_i: the 9 TA subsystems (each producing predictions)
/// - Edges e_{ij}: pairs of subsystems that must be consistent
///
/// The sheaf assigns:
/// - F(v_i) = ℝ^{d_i}: prediction space of subsystem i
/// - F(e_{ij}): comparison space for consistency between i and j
/// - ρ_{v_i, e_{ij}}: F(v_i) → F(e_{ij}): restriction map
///   (projects subsystem i's prediction into the comparison space)
///
/// (Hansen & Ghrist, 2019, "Toward a Spectral Theory of Cellular Sheaves",
///  Journal of Applied and Computational Topology, 3, 315-358)
pub struct CellularSheaf {
    /// Number of vertices (subsystems).
    pub n_vertices: usize,
    /// Vertex stalks: dimension of each subsystem's prediction space.
    pub vertex_dims: Vec<usize>,
    /// Edges: pairs of vertices that share consistency constraints.
    pub edges: Vec<(usize, usize)>,
    /// Edge stalks: dimension of each comparison space.
    pub edge_dims: Vec<usize>,
    /// Restriction maps: linear maps from vertex stalk to edge stalk.
    /// restriction_maps[e] = (matrix_from_v1, matrix_from_v2)
    pub restriction_maps: Vec<(Vec<Vec<f64>>, Vec<Vec<f64>>)>,
}

/// A section of the sheaf: a choice of prediction from each subsystem.
pub struct SheafSection {
    /// Per-vertex prediction vectors.
    pub vertex_values: Vec<Vec<f64>>,
}

/// The coboundary operator δ: C^0(G, F) → C^1(G, F).
///
/// For a section s ∈ C^0, the coboundary measures inconsistency:
///   (δs)(e_{ij}) = ρ_{v_j, e_{ij}}(s(v_j)) - ρ_{v_i, e_{ij}}(s(v_i))
///
/// A section is CONSISTENT (a global section) iff δs = 0.
/// ||δs||² measures the total inconsistency across all edges.
impl CellularSheaf {
    /// Compute the coboundary of a section.
    pub fn coboundary(&self, section: &SheafSection) -> Vec<Vec<f64>> {
        self.edges.iter().enumerate().map(|(e_idx, &(vi, vj))| {
            let (rho_i, rho_j) = &self.restriction_maps[e_idx];
            let projected_i = mat_vec_mul(rho_i, &section.vertex_values[vi]);
            let projected_j = mat_vec_mul(rho_j, &section.vertex_values[vj]);
            subtract_vec(&projected_j, &projected_i)
        }).collect()
    }

    /// Total inconsistency: ||δs||².
    pub fn inconsistency(&self, section: &SheafSection) -> f64 {
        self.coboundary(section).iter()
            .flat_map(|v| v.iter())
            .map(|x| x * x)
            .sum()
    }
}
```

### Sheaf Laplacian — Diffusion with consistency

```rust
/// The sheaf Laplacian L_F = δ^T δ.
///
/// The sheaf Laplacian generalizes the graph Laplacian by incorporating
/// the restriction maps. Where the graph Laplacian diffuses scalar values,
/// the sheaf Laplacian diffuses VECTOR values while preserving
/// consistency structure.
///
/// Spectral properties (Hansen & Ghrist, 2019):
/// - ker(L_F) = H^0(G, F) = space of globally consistent sections
/// - dim(ker(L_F)) = β_0(F) = number of independent consistent predictions
/// - Smallest nonzero eigenvalue λ_1 = "consistency gap" (how far from
///   consistency the best non-trivial section is)
/// - Fiedler-like bound: λ_1 ≥ h²(F)/2 where h(F) is the sheaf Cheeger constant
pub struct SheafLaplacian {
    /// The Laplacian matrix L_F (block matrix, total dim = Σ d_i).
    pub matrix: Vec<Vec<f64>>,
    /// Total dimension.
    pub total_dim: usize,
}

impl CellularSheaf {
    /// Construct the sheaf Laplacian L_F = δ^T δ.
    pub fn laplacian(&self) -> SheafLaplacian {
        let total_dim: usize = self.vertex_dims.iter().sum();
        let mut matrix = vec![vec![0.0; total_dim]; total_dim];

        for (e_idx, &(vi, vj)) in self.edges.iter().enumerate() {
            let (rho_i, rho_j) = &self.restriction_maps[e_idx];
            let offset_i = self.vertex_offset(vi);
            let offset_j = self.vertex_offset(vj);

            // L_F has block structure:
            // L_F[vi,vi] += ρ_i^T ρ_i
            // L_F[vj,vj] += ρ_j^T ρ_j
            // L_F[vi,vj] -= ρ_i^T ρ_j
            // L_F[vj,vi] -= ρ_j^T ρ_i
            add_block(&mut matrix, offset_i, offset_i, &mat_mul_transpose(rho_i, rho_i));
            add_block(&mut matrix, offset_j, offset_j, &mat_mul_transpose(rho_j, rho_j));
            sub_block(&mut matrix, offset_i, offset_j, &mat_mul_transpose(rho_i, rho_j));
            sub_block(&mut matrix, offset_j, offset_i, &mat_mul_transpose(rho_j, rho_i));
        }

        SheafLaplacian { matrix, total_dim }
    }
}
```

### Sheaf cohomology for inconsistency detection

```rust
/// Sheaf cohomology H^k(G, F) detects global inconsistencies.
///
/// H^0(G, F) = ker(δ₀) = globally consistent sections
///   dim(H^0) > 0 means consistent global predictions exist.
///
/// H^1(G, F) = ker(δ₁) / im(δ₀) = obstructions to consistency
///   dim(H^1) > 0 means there are inconsistencies that cannot be
///   resolved by adjusting individual subsystem predictions.
///   These indicate STRUCTURAL contradictions in the oracle architecture.
///
/// (Curry, 2014, "Sheaves, Cosheaves and Applications", arXiv:1303.3255)
///
/// Connection to IIT Phi (doc 12):
/// - dim(H^0) large: high integration (subsystems agree)
/// - dim(H^1) large: low integration (structural disagreements)
/// - Sheaf cohomology provides the ALGEBRAIC explanation for
///   why certain bipartitions in the Phi computation lose less
///   information — they correspond to sheaf subcomplexes with
///   low H^1.
pub struct SheafCohomology {
    /// Betti numbers β_k = dim(H^k).
    pub betti_numbers: Vec<usize>,
    /// Basis of H^0 (globally consistent sections).
    pub global_sections: Vec<SheafSection>,
    /// Representatives of H^1 (inconsistency witnesses).
    pub obstruction_cocycles: Vec<Vec<Vec<f64>>>,
}

impl CellularSheaf {
    /// Compute sheaf cohomology via Smith normal form.
    pub fn cohomology(&self) -> SheafCohomology {
        let coboundary_matrix = self.build_coboundary_matrix();
        let snf = smith_normal_form(&coboundary_matrix);

        let beta_0 = snf.null_space_dim();
        let global_sections = snf.null_space_basis()
            .into_iter()
            .map(|v| self.vector_to_section(&v))
            .collect();

        SheafCohomology {
            betti_numbers: vec![beta_0, snf.cokernel_dim()],
            global_sections,
            obstruction_cocycles: snf.cokernel_basis(),
        }
    }
}
```

### Sheaf neural networks for learned consistency

```rust
/// Sheaf neural networks: learn restriction maps from data.
///
/// Instead of hand-coding consistency constraints between TA subsystems,
/// learn them from prediction-outcome pairs.
///
/// Architecture (Bodnar et al., 2022, "Neural Sheaf Diffusion",
///  arXiv:2202.04579):
/// 1. Input: subsystem predictions at vertices
/// 2. Sheaf diffusion: x_{t+1} = x_t - σ · L_F · x_t
///    where L_F is the sheaf Laplacian with LEARNED restriction maps
/// 3. Output: diffused predictions (consistent, denoised)
///
/// The restriction maps ρ_{v,e} are parameterized as small neural networks
/// or linear maps learned via backpropagation.
pub struct SheafNeuralNetwork {
    /// Number of diffusion steps.
    pub n_diffusion_steps: usize,   // default: 5
    /// Diffusion step size.
    pub sigma: f64,                  // default: 0.1
    /// Whether restriction maps are learned or fixed.
    pub learn_restrictions: bool,    // default: true
    /// Hidden dimension for restriction map networks.
    pub restriction_hidden_dim: usize, // default: 32
}
```

---

## Part II: Tropical Geometry for Decision Boundaries

### The max-plus semiring and oracle decisions

Every oracle prediction that selects among discrete outcomes computes a maximum over score functions — this is inherently tropical arithmetic.

```rust
/// Tropical semiring: (ℝ ∪ {-∞}, ⊕, ⊗) where:
///   a ⊕ b = max(a, b)     (tropical addition)
///   a ⊗ b = a + b          (tropical multiplication)
///
/// Key insight (Zhang et al., 2018, ICML):
/// A ReLU neural network computes a tropical rational function.
/// The decision boundary of max(f₁(x), f₂(x)) is a TROPICAL
/// HYPERSURFACE — a piecewise-linear codimension-1 set in input space.
///
/// For oracle policies:
///   prediction = argmax_k score_k(observation)
///   The boundary where score_i = score_j is a tropical hyperplane.
///   The arrangement of ALL boundaries forms a tropical polytope
///   whose combinatorial type characterizes the oracle's behavior.
pub struct TropicalPolynomial {
    /// Coefficients c_i and exponent vectors a_i.
    /// f(x) = max_i (c_i + a_i · x)
    pub terms: Vec<TropicalTerm>,
}

pub struct TropicalTerm {
    /// Constant coefficient.
    pub coefficient: f64,
    /// Exponent vector (linear coefficients in max-plus).
    pub exponents: Vec<f64>,
}

impl TropicalPolynomial {
    /// Evaluate the tropical polynomial at a point.
    /// f(x) = max_i (c_i + a_i · x)
    pub fn evaluate(&self, x: &[f64]) -> f64 {
        self.terms.iter()
            .map(|t| t.coefficient + dot(&t.exponents, x))
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Find the tropical hypersurface (decision boundary).
    /// The hypersurface is the set of points where the maximum
    /// is achieved by at least two terms simultaneously.
    pub fn hypersurface_test(&self, x: &[f64]) -> bool {
        let values: Vec<f64> = self.terms.iter()
            .map(|t| t.coefficient + dot(&t.exponents, x))
            .collect();
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let count = values.iter().filter(|&&v| (v - max_val).abs() < 1e-10).count();
        count >= 2  // on the hypersurface iff 2+ terms tie for maximum
    }
}
```

### Tropical convexity for prediction regions

```rust
/// Tropical convex hull of oracle prediction prototypes.
///
/// The tropical convex hull of points p₁, ..., pₙ is:
///   tconv(p₁,...,pₙ) = { max_i(c_i + p_i) : c_i ∈ ℝ, ⊕_i c_i = max(c_i) = 0 }
///
/// Tropical convexity has properties different from classical convexity:
/// - Tropical line segments are piecewise-linear paths
/// - Tropical convex sets can be non-convex in the classical sense
/// - The tropical convex hull of d+1 points in ℝ^d is a tropical polytope
///   whose combinatorial type classifies the point configuration
///
/// Application: each oracle's prediction space is tropically convex.
/// The tropical convex hull of successful prediction prototypes defines
/// the oracle's "competence region" in a way that respects the
/// max-plus structure of the prediction mechanism.
///
/// (Develin & Sturmfels, 2004; developments survey: arXiv:2405.17005)
pub struct TropicalConvexHull {
    /// Generator points.
    pub generators: Vec<Vec<f64>>,
    /// Dimension of the ambient space.
    pub dim: usize,
}

impl TropicalConvexHull {
    /// Test membership in the tropical convex hull.
    ///
    /// A point x is in tconv(p₁,...,pₙ) iff there exist c₁,...,cₙ ∈ ℝ
    /// with max(c_i) = 0 such that x_j = max_i(c_i + p_{i,j}) for all j.
    ///
    /// Solvable as a linear feasibility problem in O(n·d) variables.
    pub fn contains(&self, x: &[f64]) -> bool {
        // Formulate as: find c s.t. max(c) = 0 and
        // for all j: x_j = max_i(c_i + generators[i][j])
        // This is a tropical linear system.
        tropical_feasibility(&self.generators, x)
    }
}
```

### Tropical attention for symbolic-neural fusion

```rust
/// Tropical attention: attention mechanism native to max-plus semiring.
///
/// Standard attention: softmax(QK^T / √d) V
/// Tropical attention: max-plus(Q ⊗ K^T) ⊗ V
///
/// Where ⊗ is tropical matrix multiplication:
///   (A ⊗ B)_{ij} = max_k(A_{ik} + B_{kj})
///
/// Key property (arXiv:2505.17190, 2025):
/// Tropical attention directly approximates dynamic programming
/// algorithms (shortest paths, Viterbi, CKY parsing).
/// This creates a principled bridge between:
/// - Symbolic planning (DAG executor in roko-orchestrator)
/// - Neural scoring (oracle prediction networks)
///
/// Application to Roko:
/// When the plan DAG executor chooses which task to execute next,
/// the scoring function is naturally a tropical polynomial.
/// Tropical attention learns optimal task selection from execution history.
pub struct TropicalAttention {
    /// Query projection dimension.
    pub d_k: usize,                 // default: 64
    /// Value projection dimension.
    pub d_v: usize,                 // default: 64
    /// Number of tropical attention heads.
    pub n_heads: usize,             // default: 4
    /// Temperature for soft-max approximation (→0 recovers exact max).
    pub temperature: f64,            // default: 0.1
}

impl TropicalAttention {
    /// Tropical attention forward pass.
    ///
    /// Given queries Q, keys K, values V:
    ///   Attention(Q,K,V) = softmax(Q ⊗ K^T / τ) · V
    ///
    /// where ⊗ is tropical matmul and τ → 0 recovers exact max-plus.
    pub fn forward(
        &self,
        queries: &[Vec<f64>],
        keys: &[Vec<f64>],
        values: &[Vec<f64>],
    ) -> Vec<Vec<f64>> {
        // Tropical matmul: (Q ⊗ K^T)_{ij} = max_k(Q_{ik} + K_{jk})
        let scores: Vec<Vec<f64>> = queries.iter().map(|q| {
            keys.iter().map(|k| {
                q.iter().zip(k.iter())
                    .map(|(qi, ki)| qi + ki)
                    .fold(f64::NEG_INFINITY, f64::max)
            }).collect()
        }).collect();

        // Soft-max approximation with temperature
        let weights = softmax_2d(&scores, self.temperature);

        // Weighted sum of values
        mat_mul(&weights, values)
    }
}
```

### Tropical robustness analysis

```rust
/// Tropical geometry reveals adversarial vulnerability structure.
///
/// Zhang et al. (2018, ICML) and subsequent work (arXiv:2402.00576, 2024)
/// show that:
/// 1. Decision boundaries of ReLU-based oracles are tropical hypersurfaces
/// 2. Adversarial examples live on or near these hypersurfaces
/// 3. The NUMBER of linear regions in a tropical polynomial
///    correlates with adversarial robustness (more regions → more robust)
///
/// This connects to certified robustness (doc 11):
/// - Lipschitz constant L of a tropical polynomial is the maximum
///   slope across all linear regions
/// - Certification radius R = margin / L
/// - Tropical geometry provides an exact characterization of L
///   (not just an upper bound as in spectral norm methods)
pub struct TropicalRobustnessAnalyzer {
    /// The oracle's prediction function as a tropical polynomial.
    pub policy: TropicalPolynomial,
}

impl TropicalRobustnessAnalyzer {
    /// Count the number of linear regions (combinatorial complexity).
    pub fn count_linear_regions(&self) -> usize {
        // The number of linear regions equals the number of cells
        // in the tropical hyperplane arrangement.
        self.policy.terms.len()  // upper bound; exact count requires arrangement computation
    }

    /// Compute exact Lipschitz constant from the tropical polynomial.
    ///
    /// L = max over all linear regions of ||gradient||
    /// For a tropical polynomial f(x) = max_i(c_i + a_i·x),
    /// the gradient in region i is a_i, so L = max_i ||a_i||.
    pub fn exact_lipschitz(&self) -> f64 {
        self.policy.terms.iter()
            .map(|t| norm(&t.exponents))
            .fold(0.0f64, f64::max)
    }

    /// Find the minimum distance from a point to the tropical hypersurface.
    /// This is the EXACT adversarial perturbation distance
    /// (not a bound — the true minimum distance to a decision boundary).
    pub fn distance_to_boundary(&self, x: &[f64]) -> f64 {
        let values: Vec<f64> = self.policy.terms.iter()
            .map(|t| t.coefficient + dot(&t.exponents, x))
            .collect();
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());

        if sorted.len() < 2 {
            return f64::MAX;
        }

        // Distance to boundary ≈ (best score - second best) / ||gradient_diff||
        let margin = sorted[0] - sorted[1];
        let best_idx = values.iter().position(|&v| (v - sorted[0]).abs() < 1e-12).unwrap();
        let second_idx = values.iter().position(|&v| (v - sorted[1]).abs() < 1e-12).unwrap();
        let grad_diff = subtract_vec(
            &self.policy.terms[best_idx].exponents,
            &self.policy.terms[second_idx].exponents,
        );
        let grad_norm = norm(&grad_diff);
        if grad_norm < 1e-12 { f64::MAX } else { margin / grad_norm }
    }
}
```

### Tropical VCG auction theory

```rust
/// Tropical geometry in the VCG attention auction.
///
/// The VCG auction used by the Composer (doc 00, doc 10) for context
/// allocation is a product-mix auction. Recent work shows that
/// product-mix auctions are fundamentally tropical-geometric objects:
///
/// - Bidder valuations form tropical polynomials
/// - Competitive equilibrium prices lie on tropical hypersurfaces
/// - The set of Walrasian equilibria is a tropical polytope
///
/// (Baldwin & Klemperer, 2019, "Understanding Preferences: 'Demand Types',
///  and the Existence of Equilibrium with Indivisibilities";
///  Tran & Yu, 2019, "Product-Mix Auctions and Tropical Geometry", MOR)
///
/// This means Roko's VCG context auction has a tropical structure:
/// the equilibrium prices for context window slots are solutions
/// to a tropical linear system. Computing them via tropical methods
/// is O(n·k) where n = bidders and k = slots, faster than general
/// VCG computation.
pub struct TropicalAuction {
    /// Bidder valuations as tropical polynomials.
    pub bidder_valuations: Vec<TropicalPolynomial>,
    /// Number of available slots.
    pub n_slots: usize,
}

impl TropicalAuction {
    /// Find competitive equilibrium prices via tropical linear algebra.
    pub fn equilibrium_prices(&self) -> Vec<f64> {
        // Solve the tropical linear system for equilibrium
        // Uses tropical Cramer's rule (Richter-Gebert et al., 2005)
        tropical_linear_solve(&self.bidder_valuations, self.n_slots)
    }
}
```

---

## Integration: Sheaf + Tropical + Existing Architecture

### Connecting sheaf consistency to IIT Phi

The sheaf Laplacian eigenvalues provide a principled replacement for the brute-force Phi computation (doc 12):

```
IIT Phi (doc 12):           Sheaf cohomology (this doc):
510 bipartitions enumerated  →  dim(H^1(G, F)) computed via Smith normal form
O(2^9) cost                  →  O(d^3) where d = total stalk dimension
Phi = min(ΔI) over partitions →  β_1 = number of independent obstructions
```

When β_1 = 0 (no obstructions), the TA subsystems are guaranteed to have a globally consistent prediction — a stronger statement than "Phi is high."

### Connecting tropical geometry to adversarial robustness

Tropical analysis (this doc) complements the certified robustness methods (doc 11):

```
Certified robustness (doc 11):        Tropical analysis (this doc):
Randomized smoothing: statistical     →  Exact boundary distance: deterministic
Lipschitz bounds: upper bound on L    →  Exact L from tropical polynomial structure
IBP: interval over-approximation      →  Exact linear region enumeration
```

Tropical methods provide EXACT adversarial distances (not bounds), but only for piecewise-linear oracle functions. For neural oracles, the tropical analysis applies to the last layer (softmax/argmax) exactly.

---

## Configuration parameters

| Parameter | Default | Range | Notes |
|---|---|---|---|
| Sheaf: `n_diffusion_steps` | 5 | 1-20 | More steps = smoother but may over-smooth |
| Sheaf: `sigma` (diffusion rate) | 0.1 | 0.01-1.0 | Higher = faster convergence, risk of instability |
| Sheaf: `restriction_hidden_dim` | 32 | 8-128 | Capacity of learned restriction maps |
| Tropical: `temperature` | 0.1 | 0.01-1.0 | Lower = closer to exact max-plus |
| Tropical: `n_heads` | 4 | 1-8 | Tropical attention heads |

---

## Test criteria

- **Sheaf consistency**: For a section with δs = 0, `inconsistency()` returns 0.0 within f64 epsilon.
- **Laplacian positive semidefiniteness**: All eigenvalues of L_F are ≥ 0.
- **Cohomology dimension**: For a connected graph with trivial sheaf (all ρ = identity), β_0 = 1.
- **Tropical evaluation**: For f(x) = max(3 + 2x, 1 + 4x), f(1) = max(5, 5) = 5.
- **Tropical hypersurface detection**: At x=1 in the above example, `hypersurface_test` returns true.
- **Exact Lipschitz**: For f(x) = max(2x, 4x), exact_lipschitz() = 4.0.
- **Tropical attention**: With temperature → 0, output converges to the value vector with highest tropical score.
- **Sheaf-IIT agreement**: When β_1 = 0, Phi > 0 (global consistency implies integration).

---

## Academic foundations

- Hansen, J., & Ghrist, R. (2019). "Toward a Spectral Theory of Cellular Sheaves." *Journal of Applied and Computational Topology*, 3, 315-358. — Sheaf Laplacian spectral theory.
- Bodnar, C., et al. (2022). "Neural Sheaf Diffusion: A Topological Perspective on Heterophily and Oversmoothing in GNNs." arXiv:2202.04579. — Learned sheaf neural networks.
- Curry, J. (2014). "Sheaves, Cosheaves and Applications." arXiv:1303.3255. — Computational sheaf cohomology.
- Bredon, G. E. (1997). *Sheaf Theory*. 2nd ed. Springer. — Standard reference.
- Robinson, M. (2014). *Topological Signal Processing*. Springer. — Sheaves for signal processing.
- Gebhart, T., Schrater, P., & Hylton, A. (2023). "Knowledge Sheaves: A Sheaf-Theoretic Framework for Knowledge Graph Embedding." *PMLR 206*. — Knowledge representation via sheaves.
- Zhang, L., Naitzat, G., & Lim, L.-H. (2018). "Tropical Geometry of Deep Neural Networks." *ICML 2018*. — Tropical decision boundaries.
- Alfarra, M., et al. (2024). "Tropical Decision Boundaries for Neural Networks Are Robust Against Adversarial Attacks." arXiv:2402.00576. — Tropical adversarial robustness.
- Tran, N. M., & Yu, J. (2019). "Product-Mix Auctions and Tropical Geometry." *Mathematics of Operations Research*, 44(4). — Tropical auction theory.
- Maragos, P. (2024). "Tropical Geometry for Machine Learning and Optimization." *ICASSP 2024 Tutorial*. — Comprehensive tropical ML survey.
- arXiv:2505.17190 (2025). "Tropical Attention: Neural Algorithmic Reasoning for Combinatorial Algorithms." — Tropical attention mechanism.
- Develin, M., & Sturmfels, B. (2004). "Tropical Convexity." *Documenta Mathematica*, 9, 1-27. — Tropical convex hull foundations.

---

## Cross-References

- See [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) for HDC pattern encoding that sheaf sections encode
- See [07-spectral-liquidity-manifolds.md](./07-spectral-liquidity-manifolds.md) for Riemannian geometry complementing information geometry
- See [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) for TDA (topology from different angle than sheaves)
- See [11-adversarial-signal-robustness.md](./11-adversarial-signal-robustness.md) for certified robustness methods complemented by tropical analysis
- See [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) for IIT Phi replaced by sheaf cohomology
- See [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) for VCG auction with tropical structure


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/INDEX.md

# Topic 20: Technical Analysis — Universal Oracle Primitives

> TA is NOT chain-only. It is a general-purpose prediction framework with domain-specific instances.

**Prerequisites**: [00-architecture](../00-architecture/INDEX.md) for Synapse Architecture (Engram, 6 traits, 5 layers), [05-learning](../05-learning/INDEX.md) for cybernetic feedback loops and CascadeRouter, [06-neuro](../06-neuro/INDEX.md) for HDC knowledge encoding and tier progression

---

## Overview

Technical analysis (TA) originated as a financial discipline — chart patterns, moving averages, momentum oscillators applied to price data. In Roko, TA is generalized into a set of **universal oracle primitives**: prediction, evaluation, calibration, and feedback loops that operate identically across any domain where an agent interacts with a verifiable external system.

The core insight: code, markets, research, and operations all share the same structural properties that make TA useful — measurable state variables, time series dynamics, feedback loops, pattern recurrence, adversarial dynamics, and external verification. A build time trend is structurally analogous to a price trend. A test failure probability is structurally analogous to a risk assessment. The mathematics is identical; the domain vocabulary changes.

The `Oracle` trait provides the universal interface. Domain-specific implementations (chain, coding, research, custom) handle the details. New domains are added by implementing the Oracle trait — not modifying the kernel. This topic is also where REF15's compounding, superlinear, and exponential product claims become measurable, where REF18's moat becomes observable, and where REF19's net-new innovation claims become testable rather than rhetorical: the oracle stack is intended to supply the KPIs, anti-metrics, calibration traces, and replay evidence that can separate a genuinely new primitive from a careful integration. See [tmp/refinements/15-exponential-scaling.md](../../tmp/refinements/15-exponential-scaling.md), [tmp/refinements/18-competitive-moat.md](../../tmp/refinements/18-competitive-moat.md), [tmp/refinements/19-net-new-innovations.md](../../tmp/refinements/19-net-new-innovations.md), and the shared [glossary](../00-architecture/01-naming-and-glossary.md).

> **Status framing**: This topic mixes shipping building blocks with research
> hypotheses. Shipping today: the Rust orchestration/runtime stack, multi-backend
> dispatch, the gate pipeline, HDC primitives, episode logging/feedback loops,
> and the interactive TUI. Target-state or research-hypothesis material here
> includes the universal Oracle surface, moat telemetry over unbuilt primitives,
> demurrage dashboards, replication-ledger-backed claims, and much of the
> REF19 innovation catalog.

---

## Sub-documents

| # | File | Title | Lines | Summary |
|---|---|---|---|---|
| 00 | [00-vision-ta-generalized.md](./00-vision-ta-generalized.md) | TA as Universal Oracle Primitives | ~280 | Vision document. Why generalize TA. Structural analogy argument. Cross-domain HDC transfer. Includes target-state moat telemetry and an honesty split between shipping evidence, research hypotheses, and integrated prior art. |
| 01 | [01-oracle-trait.md](./01-oracle-trait.md) | The Oracle Trait | ~380 | Full Rust trait signature. `predict()`, `evaluate()`. OracleQuery, Prediction, PredictionAccuracy structs. PredictionStore, ResidualCorrector, CalibrationTracker. Integration with all 6 Synapse traits. |
| 02 | [02-chain-oracles.md](./02-chain-oracles.md) | Chain Oracles | ~300 | ChainOracle implementation. Traditional TA (MA, RSI, Bollinger, MACD). DeFi-native indicators (concentrated liquidity, lending, funding rates, yield curves, on-chain options). MEV detection. 8 T0 chain probes. Mirage-rs integration. |
| 03 | [03-coding-oracles.md](./03-coding-oracles.md) | Coding Oracles | ~320 | CodingOracle implementation. Build time prediction, test failure probability, complexity drift, dependency risk, performance regression. 6 T0 coding probes. Tech debt feedback loops. roko-index integration. |
| 04 | [04-research-oracles.md](./04-research-oracles.md) | Research Oracles | ~280 | ResearchOracle implementation. Source reliability, completeness assessment, contradiction detection, replication probability, citation momentum. p-hacking detection. Charnov stopping rule for research. |
| 05 | [05-witness-as-ta-generalized.md](./05-witness-as-ta-generalized.md) | The Witness Pipeline | ~280 | Generalized witness trait. Chain, coding, research witness implementations. Triage pipeline (MIDAS-R, DDSketch). CorticalState shared signal bus. Three cognitive speeds in the witness. |
| 06 | [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) | Hyperdimensional TA | ~280 | HDC pattern algebra for TA. Role-filler composition. Temporal encoding via permutation. Shift-invariant matching. DeFi and coding codebooks. Cross-domain resonance detection (threshold 0.526). Pattern store with Dreams consolidation. |
| 07 | [07-spectral-liquidity-manifolds.md](./07-spectral-liquidity-manifolds.md) | Spectral Liquidity Manifolds | ~300 | Riemannian geometry for DeFi execution costs. Metric tensor (slippage + gas + time + opportunity). Christoffel symbols. Geodesics as optimal execution paths. Ricci scalar as market stability indicator. Parallel transport for cross-protocol pattern transfer. Fréchet mean. Spectral decomposition. |
| 08 | [08-adaptive-signal-metabolism.md](./08-adaptive-signal-metabolism.md) | Adaptive Signal Metabolism | ~280 | Signals as living organisms. Hebbian learning (Oja's rule). Replicator dynamics (Taylor & Jonker). Fisher's fundamental theorem. Speciation, fitness landscapes (Sewall Wright). Red Queen dynamic. SignalRegistry ecosystem. |
| 09 | [09-causal-microstructure-discovery.md](./09-causal-microstructure-discovery.md) | Causal Microstructure Discovery | ~280 | Pearl's causal hierarchy (3 levels). Structural causal models. do-operator. PC algorithm (Spirtes/Glymour/Scheines). Granger causality with 4 DeFi extensions. Interventional discovery via mirage-rs. Dream-based counterfactuals. Backdoor criterion. |
| 10 | [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) | Predictive Geometry & Resonant Patterns | ~320 | TDA persistence diagrams and landscapes (Bubenik). Topology-to-trajectory mapping. Resonant patterns as organisms with HDC genomes. Reproductive algebra. VCG auction competition. Lotka-Volterra dynamics. Price equation. |
| 11 | [11-adversarial-signal-robustness.md](./11-adversarial-signal-robustness.md) | Adversarial Signal Robustness | ~300 | Adversarial signal decomposition. HDC prototype matching (~10ns). Robust statistics (trimmed mean, Hodges-Lehmann, MAD, rank transform). Signal cross-validation. Red-team dreaming algorithm. Domain-specific attack prototypes. |
| 12 | [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) | Somatic TA & Emergent Multiscale Intelligence | ~320 | Somatic markers (Damasio) as HDC bindings. PAD encoding. Somatic retrieval (~63ns). 15% contrarian retrieval (Bower). IIT Phi over 9 TA subsystems (510 bipartitions). MIB diagnostic. PID synergy detection (Williams & Beer). |
| 13 | [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) | Predictive Foraging & Active Inference | ~330 | The complete prediction-resolution-calibration loop. PredictionClaim, ResidualCorrector (~50ns), CalibrationTracker. Active inference POMDP (90 states). EFE decomposition (pragmatic + epistemic - ambiguity). Charnov MVT stopping rule. Thompson Sampling for oracle selection. Collective calibration on Korai and the data needed for REF15 scaling KPIs. |
| 14 | [14-sheaf-tropical-geometry.md](./14-sheaf-tropical-geometry.md) | Sheaf-Theoretic Consistency & Tropical Decision Geometry | ~450 | Cellular sheaves for oracle consistency (Hansen & Ghrist). Sheaf Laplacian, cohomology for inconsistency detection. Sheaf neural networks (Bodnar et al.). Tropical semiring (max-plus). Tropical polynomials as oracle decisions. Tropical convexity. Tropical attention (symbolic-neural fusion). Tropical robustness (exact adversarial distances). Tropical VCG auctions. |

---

## Key concepts

| Concept | Description | Where defined |
|---|---|---|
| **Oracle trait** | Universal prediction interface: `predict()` + `evaluate()` | [01-oracle-trait.md](./01-oracle-trait.md) |
| **Prediction** | Value + confidence + interval + horizon + lineage | [01-oracle-trait.md](./01-oracle-trait.md) |
| **PredictionStore** | Lifecycle management: register → track → resolve → feedback | [01-oracle-trait.md](./01-oracle-trait.md) |
| **ResidualCorrector** | Bias elimination at ~50ns per correction | [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) |
| **CalibrationTracker** | Per-(model, category) accuracy statistics | [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) |
| **Scaling dashboard** | Target-state cross-session compounding KPIs, moat telemetry, and anti-metrics for testing superlinear claims | [00-vision-ta-generalized.md](./00-vision-ta-generalized.md) |
| **Net-new innovation lens** | Distinguishes shipping innovations from research hypotheses and integrated prior art | [00-vision-ta-generalized.md](./00-vision-ta-generalized.md) |
| **CorticalState** | Shared atomic signal bus for T0 probes | [05-witness-as-ta-generalized.md](./05-witness-as-ta-generalized.md) |
| **T0 Probes** | 16 zero-LLM probes (8 chain + 6 coding + 2 universal) | [02-chain-oracles.md](./02-chain-oracles.md), [03-coding-oracles.md](./03-coding-oracles.md) |
| **HDC pattern algebra** | 10,240-bit BSC vectors: bind (XOR), bundle (majority), permute (rotate) | [06-hyperdimensional-ta.md](./06-hyperdimensional-ta.md) |
| **Somatic markers** | HDC bindings between patterns and PAD affect states | [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) |
| **Spectral manifold** | Riemannian geometry over liquidity cost landscape | [07-spectral-liquidity-manifolds.md](./07-spectral-liquidity-manifolds.md) |
| **Active inference** | Factorized POMDP (90 states), EFE for context selection | [13-predictive-foraging-and-active-inference.md](./13-predictive-foraging-and-active-inference.md) |
| **Replicator dynamics** | Fitness-proportionate signal evolution | [08-adaptive-signal-metabolism.md](./08-adaptive-signal-metabolism.md) |
| **Causal discovery** | Pearl's SCM + PC algorithm + Granger + mirage-rs interventions | [09-causal-microstructure-discovery.md](./09-causal-microstructure-discovery.md) |
| **Persistence landscapes** | Banach-space elements from TDA for trajectory prediction | [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) |
| **Red-team dreaming** | Adversarial self-simulation during Delta Dreams | [11-adversarial-signal-robustness.md](./11-adversarial-signal-robustness.md) |
| **IIT Phi** | Integrated information metric over 9 TA subsystems | [12-somatic-ta-and-emergent-multiscale.md](./12-somatic-ta-and-emergent-multiscale.md) |
| **Conformal prediction** | Distribution-free prediction sets with coverage guarantees | [01-oracle-trait.md](./01-oracle-trait.md) |
| **Oracle composition** | Weighted ensemble, conformal aggregation, recalibration | [01-oracle-trait.md](./01-oracle-trait.md) |
| **Fisher-Rao metric** | Information-geometric Riemannian metric on oracle parameter space | [07-spectral-liquidity-manifolds.md](./07-spectral-liquidity-manifolds.md) |
| **Natural gradient** | Coordinate-free optimization on statistical manifolds | [07-spectral-liquidity-manifolds.md](./07-spectral-liquidity-manifolds.md) |
| **NOTEARS/SDCD** | Continuous optimization for DAG structure learning | [09-causal-microstructure-discovery.md](./09-causal-microstructure-discovery.md) |
| **DAG-GNN** | Neural causal discovery with GNN encoder | [09-causal-microstructure-discovery.md](./09-causal-microstructure-discovery.md) |
| **Persistence images** | Stable vector representation of persistent homology | [10-predictive-geometry-and-resonant-patterns.md](./10-predictive-geometry-and-resonant-patterns.md) |
| **Certified robustness** | Randomized smoothing, Lipschitz bounds, IBP | [11-adversarial-signal-robustness.md](./11-adversarial-signal-robustness.md) |
| **Cellular sheaves** | Local-to-global consistency via sheaf Laplacian and cohomology | [14-sheaf-tropical-geometry.md](./14-sheaf-tropical-geometry.md) |
| **Tropical polynomials** | Max-plus algebra for piecewise-linear oracle decisions | [14-sheaf-tropical-geometry.md](./14-sheaf-tropical-geometry.md) |
| **Tropical attention** | Symbolic-neural fusion via max-plus attention mechanism | [14-sheaf-tropical-geometry.md](./14-sheaf-tropical-geometry.md) |

---

## Citation index

All academic citations used across this topic's sub-documents:

| Citation | Used in |
|---|---|
| Friston, K. (2010). "The free-energy principle." *Nature Reviews Neuroscience*, 11(2), 127-138. | 00, 01, 05, 13 |
| Conant, R. C., & Ashby, W. R. (1970). "Every good regulator of a system." *IJSS*, 1(2), 89-97. | 00, 01, 13 |
| Sumers, T. R., et al. (2023). "Cognitive Architectures for Language Agents." arXiv:2309.02427. | 00, 01 |
| Lee, S., et al. (2026). "Meta-Harness." arXiv:2603.28052. | 00, 13 |
| Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176. | 00, 01, 02, 03, 05, 13 |
| Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6). | 00, 01, 03, 04, 06, 08, 11, 12 |
| Kanerva, P. (2009). "Hyperdimensional Computing." *Cognitive Computation*, 1(2), 139-159. | 00, 06 |
| Pirolli, P., & Card, S. (1999). "Information foraging." *Psychological Review*, 106(4), 643-675. | 00, 04, 13 |
| Charnov, E. L. (1976). "Optimal foraging: the marginal value theorem." *TPB*, 9, 129-136. | 00, 04, 05, 13 |
| Ousterhout, J. (2018). *A Philosophy of Software Design*. | 01, 03 |
| Vickrey, W. (1961). "Counterspeculation, Auctions." *Journal of Finance*, 16(1). | 01, 02, 10 |
| Pearl, J. (2009). *Causality*. 2nd ed. Cambridge University Press. | 02, 09, 11 |
| Masson, C., et al. (2019). "DDSketch." *PVLDB*, 12(12), 2195-2205. | 02, 05 |
| Wilder, J. W. (1978). *New Concepts in Technical Trading Systems*. | 02 |
| Garman, M. B., & Klass, M. J. (1980). "Estimation of Security Price Volatilities." *JoB*, 53(1). | 02 |
| McCabe, T. J. (1976). "A Complexity Measure." *IEEE TSE*, SE-2(4). | 03 |
| Lehman, M. M. (1980). "Programs, Life Cycles, and Laws of Software Evolution." *IEEE*, 68(9). | 03 |
| Nagappan, N., & Ball, T. (2005). "Relative Code Churn Measures." *ICSE 2005*. | 03 |
| Open Science Collaboration. (2015). "Reproducibility of psychological science." *Science*, 349(6251). | 04 |
| Simmons, J. P., et al. (2011). "False-Positive Psychology." *Psychological Science*, 22(11). | 04 |
| Ioannidis, J. P. A. (2005). "Why Most Published Research Findings Are False." *PLoS Medicine*, 2(8). | 04 |
| Bhatia, S., et al. (2020). "MIDAS." *AAAI 2020*. | 05 |
| McClelland, J. L., et al. (1995). "Complementary learning systems." *Psychological Review*, 102(3). | 05 |
| Plate, T. A. (1995). "Holographic Reduced Representations." *IEEE TNN*, 6(3). | 06 |
| Frady, E. P., et al. (2018). "Sequence Indexing." *Neural Computation*, 30(6). | 06 |
| Rachkovskij, D. A. (2001). "Binary Sparse Distributed Codes." *IEEE TKDE*, 13(2). | 06 |
| Lacaux, C., et al. (2021). "Sleep onset is a creative sweet spot." *Science Advances*, 7(50). | 06 |
| Amari, S., & Nagaoka, H. (2000). *Methods of Information Geometry*. | 07 |
| do Carmo, M. P. (1992). *Riemannian Geometry*. | 07 |
| Pennec, X. (2006). "Intrinsic Statistics on Riemannian Manifolds." *JMIV*, 25(1). | 07 |
| Taylor, P. D., & Jonker, L. B. (1978). "Evolutionary Stable Strategies." *Math. Biosci.*, 40(1-2). | 08 |
| Fisher, R. A. (1930). *The Genetical Theory of Natural Selection*. | 08 |
| Wright, S. (1932). "Roles of Mutation, Inbreeding, Crossbreeding, and Selection." | 08 |
| Van Valen, L. (1973). "A New Evolutionary Law." *Evolutionary Theory*, 1. | 08 |
| Oja, E. (1982). "Simplified neuron model." *J. Math. Biol.*, 15(3). | 08 |
| Hebb, D. O. (1949). *The Organization of Behavior*. | 08 |
| Spirtes, P., et al. (2000). *Causation, Prediction, and Search*. 2nd ed. MIT Press. | 09 |
| Granger, C. W. J. (1969). "Causal Relations by Econometric Models." *Econometrica*, 37(3). | 09 |
| Pearl, J. (2019). "Seven tools of causal inference." *CACM*, 62(3). | 09 |
| Bubenik, P. (2015). "Statistical TDA using Persistence Landscapes." *JMLR*, 16(3). | 10 |
| Takens, F. (1981). "Detecting strange attractors in turbulence." *LNM*, 898. | 10 |
| Price, G. R. (1970). "Selection and Covariance." *Nature*, 227. | 10 |
| Lotka, A. J. (1925). *Elements of Physical Biology*. | 10 |
| Carlsson, G. (2009). "Topology and Data." *Bulletin of the AMS*, 46(2). | 10 |
| Huber, P. J. (1964). "Robust Estimation." *Annals of Math. Stat.*, 35(1). | 11 |
| Hodges, J. L., & Lehmann, E. L. (1963). "Estimates of Location Based on Rank Tests." | 11 |
| Hampel, F. R. (1974). "The Influence Curve." *JASA*, 69(346). | 11 |
| Damasio, A. R. (1994). *Descartes' Error*. Putnam. | 12 |
| Mehrabian, A., & Russell, J. A. (1974). *An Approach to Environmental Psychology*. | 12 |
| Bower, G. H. (1981). "Mood and Memory." *American Psychologist*, 36(2). | 12 |
| Kahneman, D. (2011). *Thinking, Fast and Slow*. | 12 |
| Tononi, G. (2004). "Information integration theory." *BMC Neuroscience*, 5(42). | 12 |
| Williams, P. L., & Beer, R. D. (2010). "Nonnegative decomposition." arXiv:1004.2515. | 12 |
| Thompson, W. R. (1933). "On the Likelihood." *Biometrika*, 25(3-4). | 13 |
| Raj, V., & Kalyani, S. (2017). "Taming Non-stationary Bandits." arXiv:1707.09727. | 13 |
| Murphy, A. H. (1973). "A New Vector Partition of the Probability Score." *J. Applied Meteorology*, 12(4). | 01 |
| Vovk, V., Gammerman, A., & Shafer, G. (2005). *Algorithmic Learning in a Random World*. Springer. | 01 |
| Angelopoulos, A. N., & Bates, S. (2023). "Conformal Prediction: A Gentle Introduction." arXiv:2107.07511. | 01 |
| Naeini, M. P., et al. (2015). "Obtaining Well Calibrated Probabilities Using Bayesian Binning." *AAAI 2015*. | 01 |
| Guo, C., et al. (2017). "On Calibration of Modern Neural Networks." *ICML 2017*. | 01 |
| Cesa-Bianchi, N., & Lugosi, G. (2006). *Prediction, Learning, and Games*. Cambridge University Press. | 01 |
| Amari, S. (1998). "Natural Gradient Works Efficiently in Learning." *Neural Computation*, 10(2). | 07 |
| Amari, S. (2016). *Information Geometry and Its Applications*. Springer. | 07 |
| Čencov, N. N. (1982). *Statistical Decision Rules and Optimal Inference*. AMS. | 07 |
| Villani, C. (2009). *Optimal Transport: Old and New*. Springer. | 07 |
| Martens, J., & Grosse, R. (2015). "Optimizing Neural Networks with K-FAC." *ICML 2015*. | 07 |
| Zheng, X., et al. (2018). "DAGs with NO TEARS." *NeurIPS 2018*. | 09 |
| Yu, Y., et al. (2019). "DAG-GNN: Structure Learning with Graph Neural Networks." *ICML 2019*. | 09 |
| Nazaret, A., et al. (2024). "Stable Differentiable Causal Discovery." *ICML 2024*, PMLR 235. | 09 |
| Bello, K., et al. (2022). "DAGMA: Learning DAGs via M-matrices." *NeurIPS 2022*. | 09 |
| Adams, H., et al. (2017). "Persistence Images." *JMLR*, 18(8), 1-35. | 10 |
| Bauer, U. (2021). "Ripser: Efficient Computation of Vietoris-Rips Persistence Barcodes." *JACT*, 5(1). | 10 |
| Cohen-Steiner, D., et al. (2007). "Stability of Persistence Diagrams." *DCG*, 37(1). | 10 |
| Cohen, J. M., et al. (2019). "Certified Adversarial Robustness via Randomized Smoothing." *ICML 2019*. | 11 |
| Gowal, S., et al. (2018). "Interval Bound Propagation." arXiv:1810.12715. | 11 |
| Steinhardt, G., et al. (2017). "Certified Defenses for Data Poisoning Attacks." *NeurIPS 2017*. | 11 |
| Hansen, J., & Ghrist, R. (2019). "Toward a Spectral Theory of Cellular Sheaves." *JACT*, 3. | 14 |
| Bodnar, C., et al. (2022). "Neural Sheaf Diffusion." arXiv:2202.04579. | 14 |
| Zhang, L., Naitzat, G., & Lim, L.-H. (2018). "Tropical Geometry of Deep Neural Networks." *ICML 2018*. | 14 |
| Tran, N. M., & Yu, J. (2019). "Product-Mix Auctions and Tropical Geometry." *MOR*, 44(4). | 14 |
| Alfarra, M., et al. (2024). "Tropical Decision Boundaries Are Robust." arXiv:2402.00576. | 14 |
| Gebhart, T., et al. (2023). "Knowledge Sheaves." *PMLR 206*. | 14 |

---

## Cross-topic references

| Topic | Relationship |
|---|---|
| [00-architecture](../00-architecture/INDEX.md) | Synapse Architecture, Engram struct, 6 traits, 5 layers, Universal Cognitive Loop |
| [05-learning](../05-learning/INDEX.md) | CascadeRouter (LinUCB, Thompson Sampling), adaptive gate thresholds, cybernetic feedback loops |
| [06-neuro](../06-neuro/INDEX.md) | HDC encoding (10,240-bit BSC), knowledge tier progression, cross-domain transfer |
| [07-daimon](../09-daimon/INDEX.md) | PAD vector, behavioral states, somatic landscape, affect modulation of oracle behavior |
| [08-dreams](../10-dreams/INDEX.md) | NREM replay, REM counterfactuals, hypnagogia, offline consolidation of prediction patterns |
| [09-innovations](../20-technical-analysis/INDEX.md) | T0 probes, VCG auction, somatic landscape, collective calibration, predictive foraging |

---

## Generation notes

- **Generated by**: Claude Opus 4.6, PRD migration batch
- **Source material**: `refactoring-prd/03-cognitive-subsystems.md` §4, `refactoring-prd/09-innovations.md` §I/II/III/VII/XIX, `refactoring-prd/01-synapse-architecture.md`, legacy sources `bardo-backup/prd/23-ta/*` (11 legacy files) and `bardo-backup/tmp/agent-chain/10-predictive-foraging.md`, plus `tmp/implementation-plans/modelrouting/12-advanced-patterns.md`
- **Legacy naming map applied**: golem→agent, grimoire→neuro, bardo→roko, Signal→Engram, GNOS→KORAI, clade→collective, Styx→Agent Mesh
- **Legacy reframe rules applied**: mortality→resource management, death clocks→budget limits, succession→backup/restore
- **Citation count**: 76 unique academic citations across 15 sub-documents
- **Total lines**: ~11,000 across 16 files
- **2025-04-13 enhancement**: Deep research pass adding oracle calibration/composition (conformal prediction, Brier decomposition), information geometry (Fisher-Rao, natural gradient, α-connections, Wasserstein), continuous DAG learning (NOTEARS, DAG-GNN, SDCD, DAGMA), advanced TDA (persistence images, Ripser, vectorization methods), certified adversarial robustness (randomized smoothing, Lipschitz bounds, IBP), sheaf theory (cellular sheaves, Laplacian, cohomology), and tropical geometry (max-plus semiring, tropical attention, tropical VCG auctions). 28 new citations from 2017-2025 research.

