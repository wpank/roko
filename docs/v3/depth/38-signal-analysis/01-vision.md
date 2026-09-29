# Signal Analysis Vision: Generalized Pattern Detection

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 1

---

## Why Generalize Signal Analysis

### The structural analogy argument

Traditional TA works because financial markets have specific properties:
measurable state, time series dynamics, feedback loops, pattern recurrence,
adversarial dynamics, and external verification. Codebases share every one of
these properties:

1. **Measurable state** -- compilation time, test pass rates, cyclomatic complexity, dependency counts
2. **Time series dynamics** -- complexity trends, performance regression trajectories, test coverage drift
3. **Feedback loops** -- tech debt accumulates -> development slows -> more shortcuts -> more debt
4. **Pattern recurrence** -- similar code structures produce similar bug patterns
5. **Adversarial dynamics** -- security vulnerabilities, supply chain attacks
6. **External verification** -- compilers, test suites, benchmarks produce objective outcomes

Research corpora share similar properties:

1. **Measurable state** -- citation counts, publication velocity, contradiction density
2. **Time series dynamics** -- field maturity, paradigm shifts, replication crises
3. **Feedback loops** -- popular papers attract more citations -> more visibility -> more citations
4. **Pattern recurrence** -- similar research methodologies produce similar reliability
5. **Adversarial dynamics** -- p-hacking, selective reporting, predatory publishing
6. **External verification** -- replication studies, meta-analyses, cross-validation

The structural analogy is mathematical, not metaphorical. If TA is defined as
"systematic prediction from structured time series with feedback," then TA
applies to any domain with those properties.

### Cross-domain insight transfer

When a coding agent learns "high-churn modules need more review," it encodes
this as an HDC vector:

```
BIND(high_complexity, more_review)
```

When a research agent learns "high-retraction-rate subfields need more
scrutiny," it encodes:

```
BIND(high_retraction_rate, more_scrutiny)
```

Both encode the same abstract structure:

```
BIND(high_uncertainty, more_verification)
```

The Hamming similarity between these vectors is high because the HDC algebra
preserves structural isomorphism. Cross-domain insight transfer happens
automatically through the Neuro (knowledge) system at nanosecond cost (Kleyko
et al., 2022, *ACM Computing Surveys*).

### The Roko thesis: "the scaffold IS the product"

Roko's architectural thesis is that agent performance varies dramatically based
on the surrounding harness. Meta-Harness (Lee et al., 2026,
arXiv:2603.28052) demonstrated +7.7 points on text classification and +4.7 on
IMO-level math from harness optimization alone, at 4x fewer tokens.

Signal analysis generalization is a direct expression of this thesis. By making
prediction, calibration, and feedback universal primitives -- not
domain-specific add-ons -- every agent benefits from the same self-improving
prediction infrastructure.

## Domain-Agnostic Prediction Architecture

### Where oracles live in the architecture

| Layer | Oracle role | What it does |
|---|---|---|
| **L0 Runtime** | Prediction storage | `PredictionStore` persists predictions and outcomes |
| **L1 Framework** | Router integration | `Router.feedback()` uses prediction accuracy for bandit updates |
| **L2 Scaffold** | Context selection | EFE uses prediction confidence for context bidding |
| **L3 Harness** | Gate calibration | Prediction residuals calibrate adaptive gate thresholds |
| **L4 Orchestration** | Task prioritization | Prediction accuracy informs task scheduling priority |

### The cognitive cross-cut dimension

Oracles are a cognitive cross-cut -- they inject into multiple layers via trait
objects, never hardcoded. The Daimon (motivation engine) modulates oracle
behavior through PAD state:

- **Low Dominance** -> Predictions are made conservatively, wider intervals
- **High Arousal** -> More frequent prediction updates, faster residual correction
- **Low Pleasure** (after prediction failures) -> Automatic model recalibration

The Neuro (knowledge store) accumulates oracle patterns:

- Successful prediction strategies become `StrategyFragment` entries
- Systematic prediction biases become `Warning` entries
- Causal relationships discovered through prediction become `CausalLink` entries

Dreams (offline learning) consolidate oracle performance:

- NREM replay evaluates which prediction patterns were reliable across episodes
- REM imagination generates counterfactual predictions to test robustness
- Integration staging promotes validated strategies to permanent knowledge

## The Seven-Step Loop With Oracles

```
1. SENSE      ->  Substrate.query() + Bus.subscribe()
                  Oracle reads state, prior Signals, and live Pulses
2. ASSESS     ->  Scorer.score() + Router.select()
                  PredictiveScorer weights uncertainty, drift, and likely payoff
3. COMPOSE    ->  Composer.compose()
                  EFE-weighted context includes the smallest prediction-relevant slice
4. ACT        ->  Agent.execute()
                  Agent emits prediction Pulses and final task output
5. VERIFY     ->  Gate.verify()
                  Outcome closes the prediction and emits verdict Signals/Pulses
6. PERSIST    ->  Substrate.put()
                  Prediction, outcome, and calibration artifacts graduate to Signals
   BROADCAST  ->  Bus.publish()
                  Residuals, anomalies, and calibration Pulses feed other agents
7. REACT      ->  Policy.decide()
                  Residual correction, heuristic updates, and routing changes fire
```

## Why Signal Analysis Is a Compounding System

Each loop depends on prediction, verification, or calibration:

| Loop | Signal analysis contribution | What should improve with use |
|---|---|---|
| Demurrage-weighted retrieval | Oracles measure which memories still predict useful outcomes | Fewer wasted tokens, better retrieval precision |
| Heuristic calibration | Prediction/outcome joins tighten confidence intervals | Better priors on similar tasks |
| HDC codebook cleanup | Oracle outcomes add cleaner labels to each HDC fingerprint | Faster cache hits and better analogical matches |
| c-factor feedback | Shared prediction accuracy reveals which cohorts learn well together | Better routing across agents |
| Playbook distillation | Repeated predictions collapse into reusable templates | Lower time-to-solution |

### Anti-metrics

Superlinear capability gains are only credible if resource curves stay bounded:

| Anti-metric | Why it should stay flat or shrink |
|---|---|
| Warm-tier episode count | Demurrage should keep the working set bounded |
| Heuristic count with confirmations below 3 | Weak hypotheses should either be tested quickly or decay away |
| Mean lineage depth per response | Context depth should only grow when it buys quality |

### Evaluation guardrails

The compounding claim is only testable on real workloads with preserved state:

1. Evals must span sessions and days, not reset the Substrate between runs.
2. Each benchmark should be attempted multiple times so the slope of `time_to_solve` is measurable.
3. Commons-on versus commons-off runs should be compared explicitly.
4. Operator dashboards should be read alongside task difficulty buckets.

## Academic Foundations

- Friston, K. (2010). "The free-energy principle." *Nature Reviews Neuroscience*, 11(2), 127-138.
- Conant, R. C., & Ashby, W. R. (1970). "Every good regulator of a system must be a model of that system." *International Journal of Systems Science*, 1(2), 89-97.
- Lee, S., et al. (2026). "Meta-Harness." arXiv:2603.28052.
- Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6).
