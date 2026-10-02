# 00-ARCH -- C-Factor: Collective Intelligence

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Woolley et al. (2010, Science 330(6004):686-688) showed that group performance
> across varied tasks loads onto a single collective factor, c. This depth file
> documents the c-factor doctrine, the full formula derivation, the five process
> variables, the learned weight model, and how Roko instruments them from Bus and
> Substrate traffic.

---

## 1. The Research Foundation

### 1.1 Woolley et al. and the c-factor result

Woolley, A. W., Chabris, C. F., Pentland, A., Hashmi, N., & Malone, T. W. (2010).
"Evidence for a Collective Intelligence Factor in the Performance of Human Groups."
Science, 330(6004), 686-688.

The study administered diverse tasks to 699 people in groups of 2-5 and performed a
factor analysis on group performance scores. The key findings:

1. A single statistical factor (c) predicted group performance across tasks, analogous
   to g (general intelligence) for individuals.
2. c was **not** significantly correlated with the average IQ of group members
   (r = 0.15, p > 0.05).
3. c **was** significantly correlated with:
   - Equal distribution of conversational turn-taking (r = 0.41, p < 0.01)
   - Average social sensitivity of group members (r = 0.26, p < 0.01), measured by
     the "Reading the Mind in the Eyes" test (Baron-Cohen et al. 2001)
   - Proportion of females in the group (r = 0.23, p < 0.05), partially mediated by
     social sensitivity

The important result is not that larger groups are automatically better. The important
result is that **process matters**: how turns are shared, how well members predict one
another, how often they cite one another correctly, how open the channel is, and how
diverse the working set is.

### 1.2 Why the Bus and Substrate are enough

The Bus records who spoke, when, to whom, and whether the turn was delivered. The
Substrate records what became durable, what was cited, what was reused, and what
survived verification. In practice that is enough to compute the observable parts
of collective intelligence:

- Turn-taking from Pulse authorship and delivery timing
- Social perceptiveness from peer-prediction error
- Trust calibration from citation reciprocity and later gate survival
- Channel openness from delivery confirmation and subscription reach
- Cognitive diversity from HDC distance across cohort artifacts

No separate telemetry plane is required. The measurement falls out of the architecture
if the Bus and Substrate are already authoritative.

### 1.3 The cohort unit

Define a cohort as a set of agents working on a shared plan, task family, or
parent episode during a bounded window. Cohorts are the unit of measurement because
c-factor is about group process, not isolated agent skill.

That matters for two reasons:

- It keeps the metric local enough to drive policy
- It keeps the metric comparable across domains by normalizing to a cohort window

---

## 2. The C-Factor Formula

### 2.1 The five process variables

| Variable | Symbol | Agent analog | Measured from |
|---|---|---|---|
| Turn-taking equality | T | How evenly the cohort shares turns | Pulse authorship entropy and sender share on the Bus |
| Social perceptiveness | S | How well members predict each other's outputs | `peer.prediction` vs `peer.outcome` residuals |
| Trust calibration | R | How often citations are useful and verified | Citation reciprocity and downstream gate survival in the Substrate |
| Channel openness | O | How much intended traffic is actually delivered | Bus delivery confirmation and subscriber reach |
| Cognitive diversity | D | How different the cohort's working set is | HDC distance across cohort Signals |

### 2.2 Individual variable formulas

**Turn-taking equality (T)**:

```
H = -sum_{i=1}^{n} p_i * log_2(p_i)    for each sender i with share p_i
T = H / log_2(n)                         normalized by max entropy for n senders
```

When T = 1.0, all senders share the floor equally. When T approaches 0.0, one sender
dominates. The normalization by log_2(n) ensures T is independent of cohort size.

**Social perceptiveness (S)**:

```
S = 1 - mean(|prediction_i - outcome_i|)    for all prediction-outcome pairs
```

Where prediction_i is an agent's forecast of a peer's output (via `peer.prediction`
Pulses) and outcome_i is the actual output (via `peer.outcome` Pulses).

**Trust calibration (R)**:

```
R = |{citations surviving gate verification}| / |{total citations}|
```

A citation is a lineage reference from one Signal to another. If the cited Signal
later fails verification, R decreases for the citing agent's topic.

**Channel openness (O)**:

```
O = |{confirmed deliveries}| / |{intended deliveries}|
```

Measured from Bus delivery confirmation and subscriber reach.

**Cognitive diversity (D)**:

```
D = mean(hamming(v_i, v_j)) / d    for all pairs (i, j), d = dimensionality (10,240)
```

Where v_i, v_j are the HDC fingerprint vectors of the cohort members' working sets.

### 2.3 CohortMetrics

```rust
pub struct CohortMetrics {
    /// Normalized by cohort size. T in [0, 1].
    pub turn_taking_entropy: f64,
    /// Mean prediction accuracy across peer pairs. S in [0, 1].
    pub peer_prediction_accuracy: f64,
    /// Fraction of citations that survive verification. R in [0, 1].
    pub citation_reciprocity: f64,
    /// Fraction of intended deliveries confirmed. O in [0, 1].
    pub delivery_rate: f64,
    /// Mean pairwise HDC Hamming distance, normalized. D in [0, 1].
    pub hdc_diversity: f64,
}
```

### 2.4 The full c-factor formula

The c-factor for a cohort over a window is a weighted linear combination of the five
process variables:

```
c_factor(cohort, window) = w_T * T + w_S * S + w_R * R + w_O * O + w_D * D + bias
```

Where the weights (w_T, w_S, w_R, w_O, w_D) and bias are learned online from observed
cohort outcomes:

```rust
pub struct CohortWeights {
    pub turn_taking_entropy: f64,       // w_T
    pub peer_prediction_accuracy: f64,  // w_S
    pub citation_reciprocity: f64,      // w_R
    pub delivery_rate: f64,             // w_O
    pub hdc_diversity: f64,             // w_D
    pub bias: f64,
}

pub fn c_factor(m: &CohortMetrics, w: &CohortWeights) -> f64 {
    w.turn_taking_entropy * m.turn_taking_entropy
        + w.peer_prediction_accuracy * m.peer_prediction_accuracy
        + w.citation_reciprocity * m.citation_reciprocity
        + w.delivery_rate * m.delivery_rate
        + w.hdc_diversity * m.hdc_diversity
        + w.bias
}
```

### 2.5 CohortWeightsLearner

The weights are not static. They are fitted online from cohort outcomes:

```rust
pub struct CohortWeightsLearner<B: Bus> {
    pub bus: Arc<B>,
    pub weights: parking_lot::RwLock<CohortWeights>,
    pub learning_rate: f64,
}
```

The learner subscribes to cohort-completion Pulses, joins them with Substrate outcomes,
and updates weights by online gradient step. The score is learned from evidence, not
declared by fiat.

The gradient update rule:

```
w_{t+1} = w_t + eta * (outcome_quality - predicted_quality) * x_t
```

Where x_t is the vector of process variable values for the cohort, eta is the learning
rate, and outcome_quality is the normalized task success rate for the cohort window.

### 2.6 The metric is continuous

c-factor is tracked on a rolling cadence:

1. Collect Bus and Substrate events for the cohort window
2. Derive process variables from those events
3. Fit or refresh the learned scalar
4. Publish the current c-factor value
5. Compare with task outcomes and Policy interventions

That continuous loop matters because group process changes faster than coarse reporting
cycles. A cohort can drift into echo-chamber behavior in minutes, not quarters.

---

## 3. C-Score vs. C-Factor

The operational split between the reporting metric and the optimization metric:

| Concept | Role |
|---|---|
| `CohortMetrics` | **Explain** -- the raw vector of process variables |
| `CohortWeights` | **Adapt** -- learned mapping from metrics to outcomes |
| c-factor | **Report** -- the published scalar for dashboards and alerts |
| c-score | **Predict** -- the fitted model behind the scalar |

Near-term, dashboards, alerts, and operator review should lead. Automatic policy
actuation belongs behind explicit evidence that the signal is stable enough to govern
runtime behavior.

---

## 4. Four Diagnostic Signals

### 4.1 Turn-taking equality

If one sender dominates the cohort floor, the group is not really collective. The Bus
makes this visible through sender-share concentration and authorship entropy.

Symptoms of low turn-taking equality:

- One agent dominates Pulses
- Other agents contribute only after repeated prompting
- Cohort outcome quality tracks a single voice rather than a shared process

### 4.2 Social perceptiveness

How well members predict each other's outputs. This is the clearest place for
heuristic calibration. Heuristic models of teammates are first-class knowledge
objects, and the system should learn from their misses instead of hiding them.

### 4.3 Trust calibration

Trust is not a social vibe; it is a measurable citation relation. If an agent cites a
Signal that later fails verification, that citation should reduce trust on the relevant
topic. If the citation survives and helps a later task, trust should increase.

That gives the system a structural way to distinguish:

- Useful reuse
- Speculative reuse
- Cargo-cult reuse

### 4.4 Cognitive diversity

Measured as pairwise distance across cohort HDC clouds. If every agent's working set
converges to the same region, the cohort is over-coupled and likely brittle.

HDC diversity is the clearest structural hedge against monoculture because it measures
whether the cohort is drawing from genuinely different semantic neighborhoods.

---

## 5. Collective Calibration

### 5.1 WisdomGate

Before a consensus artifact is finalized, it should pass a WisdomGate:

```rust
pub struct WisdomGate {
    pub min_hdc_diversity: f64,
    pub max_lineage_overlap: f64,
    pub max_sender_share: f64,
    pub aggregator: Box<dyn Aggregator>,
}
```

The four classical conditions (Surowiecki 2004, The Wisdom of Crowds) map cleanly:

| Condition | Roko analog |
|---|---|
| Diversity of opinion | HDC diversity |
| Independence | Low lineage overlap |
| Decentralization | Low sender concentration |
| Aggregation | Chosen aggregation method |

### 5.2 Aggregation methods

| Method | When to use |
|---|---|
| Bundle (majority vote) | Cohort is already well aligned |
| Bind (XOR) | Provenance matters |
| Weighted bundle | Reliable trust priors exist |
| Cleanup to codebook | Output must land in existing vocabulary |

### 5.3 Anti-groupthink primitives

Three structural countermeasures keep the collective honest:

1. **Devil's-advocate Pulse**: emit an explicit opposing view on consensus topics
2. **Outsider injection**: route some work to an agent with zero lineage overlap
3. **Minority report preservation**: retain dissenting artifacts longer, with softer
   demurrage on the Substrate

These are not rhetorical devices. They are runtime policies that keep the cohort from
collapsing into self-confirmation.

### 5.4 Policy levers

Policy should not optimize c-factor blindly. It should use c-factor as a covariate and
only intervene when low c-factor coincides with poor outcomes:

1. Turn-taking temperature
2. Peer-prediction calibration
3. Trust update rates
4. Delivery and retry policy
5. Diversity pressure

The point is to adjust process, not to sandbag the metric.

---

## 6. C-Factor in the Current Architecture

### 6.1 CFactorSummary

The runner computes per-task `CFactorSummary` metrics (E25 advanced learning loops,
10/10). The summary includes turn-taking entropy, peer-prediction accuracy, and a
cohort-level scalar. This is wired into the runner dispatch/completion path.

### 6.2 CFactorPolicy

`CFactorPolicy` exists in `roko-core` and is wired into the routing stack as a live
signal. The broader continuous-measurement doctrine described in this chapter is
target-state for the full Bus/Substrate instrumentation.

### 6.3 Surfacing

c-factor appears in:

- TUI and dashboard tiles (via StateHub projections)
- HTTP API routes (roko-serve; counts in `tools/http_route_inventory.snapshot.json`)
- Metrics export: `roko.c_factor` for observability stacks
- E33 telemetry Lens (9/9, 39/39 ingress variants)

### 6.4 C-factor as a diagnostic

The central constraint: c-factor is a **diagnostic covariate, not the direct target**.

**Good use**: c-factor falls and task outcomes fall, so Policy intervenes.

**Bad use**: c-factor rises because the policy routes only easy work.

The metric should help the system see process quality, not hide it.

---

## 7. Implementation Phases

| Phase | Status |
|---|---|
| 1. Metrics-only | `CFactorSummary` wired in runner (E25 10/10) |
| 2. Dashboard tile and alerts | StateHub projection available |
| 3. Passive optimization | Target-state |
| 4. Conditional Policy actuation | Target-state |
| 5. Cross-cohort scaling | Target-state |

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Woolley, A. W. et al. 2010, Science 330(6004):686-688 | Collective intelligence factor in human groups; c predicts group performance across tasks independent of mean individual IQ |
| Baron-Cohen, S. et al. 2001, JCPP 42(2) | "Reading the Mind in the Eyes" test; social sensitivity measure used in Woolley study |
| Surowiecki, J. 2004, The Wisdom of Crowds, Doubleday | Diversity, independence, decentralization, aggregation as conditions for wise crowds |
| Metcalfe, B. 2013, Computer 46(12) | Network value scaling intuition |
| Grasse, P. P. 1959, Insectes Sociaux 6(1) | Stigmergy through environmental modification |
| Parunak, H. V. D. 2006, Engineering Self-Organising Systems | Digital stigmergy in multi-agent systems |
| Dorigo, M. et al. 2000, Artificial Life 5(3) | Emergent coordination under local rules |
| Bonabeau, E. et al. 1999, Swarm Intelligence, OUP | Self-organization and specialization |
| Beer, S. 1972, Brain of the Firm, Allen Lane | Recursive organizational intelligence |
| Vickrey, W. 1961; Clarke, E. 1971; Groves, T. 1973 | VCG mechanism: truthful bidding for resource allocation |

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- [cognitive-cross-cuts.md](cognitive-cross-cuts.md) -- Cross-cut integration and VCG arbitration
- [autocatalytic-and-cybernetics.md](autocatalytic-and-cybernetics.md) -- c-factor feedback loop
- `crates/roko-learn/src/` -- CFactorSummary computation
- `crates/roko-core/` -- CFactorPolicy
