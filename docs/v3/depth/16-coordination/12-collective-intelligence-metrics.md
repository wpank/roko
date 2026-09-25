# Depth: Collective Intelligence Metrics

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 12

---

## Overview

This document operationalizes the c-factor for coordination measurement.
The Bus is the conversation floor, Pulses are the turns, and Signals are the
durable artifacts. The goal is not to count activity for its own sake, but to
measure whether a cohort is coordinating in a way that improves outcomes over
time.

The central idea is Woolley et al.'s collective intelligence factor [Woolley,
A.W. et al. "Evidence for a Collective Intelligence Factor in the Performance
of Human Groups." *Science*, 330(6004):686-688, 2010], adapted to Roko's
runtime: measure the process, fit a cohort-level score, and let Policy
intervene only when the score and the task outcome move together. The c-factor
is therefore a diagnostic covariate, not a blind optimization target.

---

## Cohort Windows and Observation Units

A **cohort** is the smallest coordination unit: a set of agents working
together on the same plan, PRD, parent episode, or other shared objective.

A **cohort window** is the time-bounded slice used for measurement. The
default window is the smallest interval containing the cohort's active work,
closing on:

1. `cohort.completed` -- task or plan completion
2. A timeout set by Policy or orchestration
3. A handoff to a new cohort window with the same task lineage

Observation happens at three levels:

1. **Pulse turn**: One authored Pulse on a cohort Topic
2. **Artifact turn**: One Signal created, cited, or revised inside the window
3. **Outcome label**: One completion or evaluation label marking the window

The join key is a shared cohort identity on Bus and Substrate records:
Pulses carry `cohort_id`, `topic`, `author`, and sequence number; Signals
carry lineage, provenance, and the same cohort identity. This makes the
metrics computable from instrumentation rather than manual annotation.

---

## Five-Axis CohortMetrics

The c-factor is computed from five normalized axes. Each axis derives from
Bus or Substrate instrumentation.

| Axis | Meaning | Primary Source |
|------|---------|----------------|
| `turn_taking_entropy` | How evenly turns distribute across agents | Bus Pulse authorship stats |
| `peer_prediction_accuracy` | How well agents predict each other's outputs | Bus peer.prediction/outcome Pulses |
| `citation_reciprocity` | How often citations flow both ways across Signals | Substrate provenance edges |
| `delivery_rate` | How much intended traffic reaches subscribers | Bus publish/deliver/ack/drop stats |
| `hdc_diversity` | How far apart cohort artifact fingerprints are in HDC space | Substrate HDC similarity queries |

### Turn-Taking Entropy

Normalized Shannon entropy of Pulse authorship within a window:

```
H = -sum_i (p_i x log_2(p_i)) / log_2(N)
```

Where `p_i` = fraction of turns authored by agent i, N = number of agents.

- H = 1.0: Perfectly even distribution -- all agents participate equally
- H = 0.0: Single agent monopolizes the floor
- Target: H > 0.7 for healthy cohorts

High entropy means no single agent monopolizes the floor. Low entropy means
the cohort is over-concentrated around one speaker.

### Peer Prediction Accuracy

Each agent can emit a `peer.prediction` Pulse encoding what it expects
another agent to say, ship, or conclude. The corresponding `peer.outcome`
Pulse records what actually happened. Accuracy is the fraction of predictions
within accepted tolerance for the cohort's task type.

This axis measures social calibration -- how well agents model each other's
behavior. High accuracy indicates that agents have internalized the
collective's working patterns.

### Citation Reciprocity

Substrate gives each Signal a provenance chain. Citation reciprocity measures
whether the cohort uses that chain as a working memory network instead of a
one-way broadcast.

```
R = (2 x reciprocal_citations) / total_citations
```

Reciprocity rises when Agent A cites Agent B and Agent B later cites or
builds on the same artifact in a way that survives validation.

### Delivery Rate

Proportion of intended Bus deliveries that arrive at subscribed targets:

```
D = successful_deliveries / attempted_deliveries
```

Drops from backpressure, auth failure, filter mismatch, or transport failure
lower the rate. Delivery rate is a direct measure of communication health,
not just throughput.

### HDC Diversity

Measures how spread out cohort Signal fingerprints are in HDC space:

```
diversity = 1 - mean(pairwise_cosine_similarity)
```

Cohorts with fingerprints collapsing to the same region tend to lose
perspective and converge too early. The point is useful diversity, not raw
novelty.

---

## CohortMetrics Shape

```rust
/// Five-axis cohort process metrics.
///
/// Each axis is normalized to [0, 1]. The c-factor is computed as a
/// weighted linear combination of these axes.
///
/// # References
/// Woolley, A.W. et al. "Evidence for a Collective Intelligence Factor."
/// Science, 330(6004):686-688, 2010.
pub struct CohortMetrics {
    /// Normalized Shannon entropy of Pulse authorship.
    pub turn_taking_entropy: f64,
    /// Fraction of peer predictions within tolerance.
    pub peer_prediction_accuracy: f64,
    /// Bidirectional citation fraction.
    pub citation_reciprocity: f64,
    /// Successful/attempted Bus delivery ratio.
    pub delivery_rate: f64,
    /// 1 - mean pairwise HDC cosine similarity.
    pub hdc_diversity: f64,
}
```

---

## Computing the c-factor

The c-factor is a weighted score over the five axes. In the simplest form it
is a linear model with a learned bias:

```rust
pub struct CohortWeights {
    pub turn_taking_entropy: f64,
    pub peer_prediction_accuracy: f64,
    pub citation_reciprocity: f64,
    pub delivery_rate: f64,
    pub hdc_diversity: f64,
    pub bias: f64,
}

/// Compute the c-factor for a cohort window.
///
/// Returns a scalar in approximately [0, 1] representing the cohort's
/// coordination quality. The weights are learned per task family.
pub fn c_factor(m: &CohortMetrics, w: &CohortWeights) -> f64 {
    w.turn_taking_entropy * m.turn_taking_entropy
        + w.peer_prediction_accuracy * m.peer_prediction_accuracy
        + w.citation_reciprocity * m.citation_reciprocity
        + w.delivery_rate * m.delivery_rate
        + w.hdc_diversity * m.hdc_diversity
        + w.bias
}
```

The weights are learned from cohort outcomes, not hardcoded as universal
constants. Different task families can legitimately induce different weight
shapes.

---

## CohortWeightsLearner

The learner subscribes to Bus topics labeling completed windows. The main
input is `cohort.completed`, carrying measured metrics plus observed outcome
score. The learner updates weights online using stochastic gradient descent:

```rust
pub struct CohortObservation {
    pub metrics: CohortMetrics,
    pub outcome_score: f64,
}

pub struct CohortWeightsLearner<B: Bus> {
    pub bus: std::sync::Arc<B>,
    pub weights: parking_lot::RwLock<CohortWeights>,
    pub learning_rate: f64,
}

impl<B: Bus> CohortWeightsLearner<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let filter = TopicFilter::Exact(Topic::new("cohort.completed"));
        let mut rx = self.bus.subscribe(filter).await.unwrap();

        while let Some(pulse) = rx.recv().await {
            let Some(obs) = parse_cohort_observation(&pulse) else {
                continue;
            };
            let prediction = c_factor(&obs.metrics, &self.weights.read());
            let error = obs.outcome_score - prediction;

            let mut w = self.weights.write();
            let lr = self.learning_rate;
            w.turn_taking_entropy += lr * error * obs.metrics.turn_taking_entropy;
            w.peer_prediction_accuracy += lr * error * obs.metrics.peer_prediction_accuracy;
            w.citation_reciprocity += lr * error * obs.metrics.citation_reciprocity;
            w.delivery_rate += lr * error * obs.metrics.delivery_rate;
            w.hdc_diversity += lr * error * obs.metrics.hdc_diversity;
            w.bias += lr * error;

            let _ = self.bus.publish(emit_weights_update(&*w)).await;
        }
    }
}
```

The update rule is the LMS (Least Mean Squares) algorithm. Each observation
adjusts the weight vector in the direction that reduces prediction error,
scaled by the learning rate and the corresponding metric value.

---

## Instrumentation Sources

The five metrics are derivable from existing Bus and Substrate signals:

| Metric | Bus Instrumentation | Substrate Instrumentation |
|--------|--------------------|-----------------------------|
| turn_taking_entropy | Pulse sender counts, per-topic turn order | Cohort metadata only |
| peer_prediction_accuracy | peer.prediction and peer.outcome Pulses | Matching cohort labels and outcome Signals |
| citation_reciprocity | Citation Pulses or citation tags | Signal provenance, lineage, and citation edges |
| delivery_rate | publish, deliver, ack, retry, drop, backpressure | None beyond transport metadata |
| hdc_diversity | Fingerprint summaries on published artifacts | HDC fingerprint similarity over cohort Signals |

The join logic: Bus tells us what was attempted and what was seen; Substrate
tells us what persisted and how artifacts relate. Together they suffice to
compute all five axes without special-case data collection.

---

## Policy, WisdomGate, and Interventions

Policy should not optimize the c-factor directly. It should treat c-factor as
a measurement that becomes actionable only when the downstream outcome is also
worsening.

### Operational Rule

1. Observe c-factor and outcome together
2. If c-factor drops **and** outcomes also drop, intervene on process
3. If c-factor drops but outcomes stay stable, log and do not perturb

### WisdomGate Inputs

Before a consensus Signal is finalized, the WisdomGate checks whether inputs
are broad enough to deserve aggregation:

```rust
pub struct WisdomGate {
    /// Minimum turn-taking entropy for consensus acceptance. Default: 0.5.
    pub min_turn_taking_entropy: f64,
    /// Minimum peer prediction accuracy. Default: 0.3.
    pub min_peer_prediction_accuracy: f64,
    /// Minimum citation reciprocity. Default: 0.2.
    pub min_citation_reciprocity: f64,
    /// Minimum HDC diversity. Default: 0.3.
    pub min_hdc_diversity: f64,
    /// Maximum lineage overlap fraction. Default: 0.8.
    pub max_lineage_overlap: f64,
    /// Maximum fraction of turns from a single sender. Default: 0.6.
    pub max_sender_share: f64,
}
```

### Conditional Interventions

| Condition | Policy Response |
|-----------|----------------|
| Low turn-taking entropy | Soften top-1 routing, throttle dominant senders, widen speaker set |
| Low peer prediction accuracy | Route agents into calibration pairs, emit more peer.prediction Pulses |
| Low citation reciprocity | Require stronger citation trails before consensus acceptance |
| Low delivery rate | Inspect Bus backpressure, auth failures, and filter mismatches |
| Low HDC diversity | Diversify prompts, tools, or agent selection to prevent premature convergence |

---

## Groupthink Countermeasures

Optimizing for collective intelligence can produce groupthink if the cohort
converges too quickly or too uniformly. Structural countermeasures:

1. **Devil's-advocate Pulse**: Policy spawns a deliberately opposing Pulse on
   key decisions when diversity or reciprocity falls below threshold.
2. **Outsider injection**: Route the task through an agent with low lineage
   overlap so the cohort sees an outside view before consensus locks.
3. **Minority report preservation**: Keep dissenting Signals alive longer
   than the majority trail so alternative hypotheses survive.
4. **WisdomGate refusal**: If inputs are too narrow, refuse to finalize the
   consensus Signal and ask the cohort to widen its evidence base.

These are structural controls, not social theater. They prevent a
high-agreement but low-information cohort from looking healthy on the surface.

---

## Surfacing and Rollout

### Dashboard Tile

`roko dashboard` shows a cohort intelligence tile with the headline c-factor,
the five axes, and the current weakest link.

### API

```
GET /api/cohorts/{cohort_id}/metrics
GET /api/cohorts/{cohort_id}/c-factor
GET /api/cohorts/{cohort_id}/weights
```

### Phased Rollout

1. **Metrics-only**: Compute CohortMetrics from Bus and Substrate, log values
2. **Dashboard and alerts**: Surface the tile, API, and telemetry
3. **Passive optimization**: Fit CohortWeights online, keep Policy read-only
4. **Active optimization**: Allow Policy to apply conditional interventions

---

## Implementation Status

The c-factor computation infrastructure exists in `roko-learn`:

| Component | Status | Source |
|-----------|--------|--------|
| CFactorSummary struct | Wired | `roko-learn/src/quality_judge.rs` |
| Per-dispatch c-factor computation | Wired | Runner dispatch path |
| CohortWeightsLearner | Specified | Target design |
| WisdomGate | Specified | Target design |
| Dashboard tile | Partial | Named surface projection exists |
| API routes | Wired | `roko-serve` cohort routes |

The primary gap is the CohortWeightsLearner: the online weight learning loop
that fits weights from completed cohort windows is specified but not yet wired
end-to-end.

---

## Cross-References

- `11-exponential-flywheel.md` -- c-factor as Loop 4 of the flywheel
- `01-stigmergy-theory.md` -- Coordination model feeding the metrics
- [08-LEARNING](../../08-LEARNING.md) -- Playbook distillation, c-factor
  integration in learning
- [13-TELEMETRY](../../13-TELEMETRY.md) -- Lens executors for observation
- [17-GROUPS](../../17-GROUPS.md) -- Agent group membership, coordination
  modes

---

## References

- [Woolley et al. 2010] Evidence for a Collective Intelligence Factor in
  the Performance of Human Groups, *Science*, 330(6004):686-688
- [Shannon 1948] A Mathematical Theory of Communication, *Bell System
  Technical Journal*, 27(3):379-423
- [Bonabeau, Theraulaz & Deneubourg 1998] Fixed Response Thresholds and
  the Regulation of Division of Labor in Insect Societies, *Bulletin of
  Mathematical Biology*, 60(4):753-807
