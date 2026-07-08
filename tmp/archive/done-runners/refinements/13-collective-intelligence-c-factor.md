# Collective Intelligence & the c-factor

> **TL;DR**: Woolley et al. (2010, *Science*) showed that group
> performance across diverse tasks loads onto a single factor — "c"
> — analogous to the g-factor for individuals. c is predicted by
> social perceptiveness, turn-taking equality, and (notably) not by
> mean IQ of members. Roko's multi-agent runtime is a laboratory for
> measuring, optimizing, and mechanizing c. This doc proposes a
> concrete operationalization: the *Bus* is the conversation floor,
> Engrams are the artifacts, Pulses are the turns, and c-factor
> becomes a metric computed from their statistics. Improving it
> becomes an objective the Policy layer can optimize directly.

> **For first-time readers**: The c-factor is to groups what the g-factor
> is to individuals: one number that correlates with performance across
> a wide variety of tasks. Woolley 2010 found that mean IQ of group
> members didn't predict c; turn-taking equality and social
> perceptiveness did. Roko's runtime has observable analogs of all those
> process variables (Pulse authorship entropy, peer-prediction accuracy,
> citation reciprocity, delivery rate, HDC cloud diversity). This doc
> wires them into one metric, exposes it, and lets Policy optimize it.
> Read 10 (self-learning) and 11 (HDC) first; they provide the
> instrumentation this doc consumes.

## 1. The Woolley result and why it matters here

Three findings from the original paper and follow-ups:

1. **c exists**: across a battery of tasks, 40%+ of variance in group
   performance loads onto one factor.
2. **c is not mean IQ**: correlation between c and average member IQ
   is weak (r ≈ 0.15).
3. **c is driven by process**: turn-taking equality, social
   perceptiveness (measured via "Reading the Mind in the Eyes"),
   and proportion of women (partially mediated by social
   perceptiveness) all correlate strongly.

The mechanistic reading: **groups are intelligent to the extent that
information flows with low loss and reasonable equality between
members**. Bottlenecks (one dominant voice), silos (no turn-taking),
or poor perspective-taking (low social perceptiveness) all crush c.

Roko has all three failure modes available to it right now. A
dispatcher that always picks the fastest agent concentrates
turn-taking; a Router with no cross-lineage visibility creates silos;
agents that can't read each other's episodes lack social
perceptiveness. **We can do better because our "social" layer is
observable.**

## 2. Operationalizing c for Roko

### 2.1 The unit of observation

Define a *cohort*: a set of agents working on a related task (same
plan, same PRD, same parent episode). Over a time window, measure
their group output quality against an objective criterion (gates
passed, tests green, PRD reviewer score).

### 2.2 The process variables to measure

From the Woolley framework, adapted for agents:

| Human variable | Agent analog | How we measure |
|---|---|---|
| Turn-taking equality | Pulse authorship entropy per topic | `shannon(distinct_senders)` |
| Social perceptiveness | Ability to predict other agents' outputs | active-inference error on `peer.prediction` |
| Trust calibration | How often one agent cites another | citation-graph statistics |
| Channel openness | % of Pulses that reach all intended subscribers | delivery confirmation on the Bus |
| Cognitive diversity | HDC distance between agents' episode clouds | pairwise fingerprint distances |

All five are computable from the Bus + Substrate. **No new data
collection needed — just a stats layer.**

### 2.3 The c-score

Combine the five into a scalar via a small learned regression:

```rust
pub struct CohortMetrics {
    pub turn_taking_entropy: f64,     // [0, log2(n_agents)]
    pub peer_prediction_accuracy: f64, // [0, 1]
    pub citation_reciprocity: f64,     // [0, 1]
    pub delivery_rate: f64,            // [0, 1]
    pub hdc_diversity: f64,            // [0, 1]
}

pub fn c_factor(m: &CohortMetrics, w: &CohortWeights) -> f64 {
    w.a * m.turn_taking_entropy +
    w.b * m.peer_prediction_accuracy +
    w.c * m.citation_reciprocity +
    w.d * m.delivery_rate +
    w.e * m.hdc_diversity
}
```

Weights are fit by regressing `c_factor` against realized cohort
outcomes (gate-pass rate, task success). The regression itself runs as
an active-inference loop — see `10-self-learning-cybernetic-loops.md`.

## 3. Improving c

Once c is measured, the Policy layer can *optimize* it. Five levers:

### 3.1 Turn-taking

If entropy is low, the dispatcher's Router is too greedy. Add a
*temperature* that softens top-1 selection in proportion to recent
entropy deficit. Gesell's demurrage (see `12`) applied to *agent
balance* — an agent that has spoken recently pays a tax on its next
bid.

### 3.2 Social perceptiveness

Each agent publishes `peer.prediction` Pulses: "I think agent X would
say Y." Reality arrives on `peer.outcome`. Prediction error feeds the
agent's own learning. Agents that model each other well get routed to
collaborative tasks; those that can't get routed to parallelizable
(non-collaborative) ones. This is **cognitive specialization by
measured empathy.**

### 3.3 Trust calibration

Citation reciprocity and citation quality. An agent that cites another
agent's Engram which later fails a gate loses trust-credit. Accrues
trust-credit for citations that pass. Trust is per-directed-pair and
per-topic (agent A might trust B on Rust syntax, not on
database-schema design).

### 3.4 Channel openness

The Bus already tracks delivery; expose metrics. Subscribers that drop
messages due to backpressure, circuit breakers, or auth failures
*reduce c directly*. Ops dashboards should plot c alongside CPU; they
move together.

### 3.5 Cognitive diversity

Use HDC cloud distance. If all agents' episode fingerprints collapse
to the same region, diversity is low — and diversity is part of c.
Policy can inject diversity pressure: spawn agents with deliberately
different system prompts, tool sets, or model families when cloud
distance drops below threshold. **c-factor as a regularizer against
cognitive monoculture.**

## 4. The Surowiecki conditions as gates

James Surowiecki's *Wisdom of Crowds* (2004) lists four conditions:
diversity of opinion, independence, decentralization, aggregation.
These become explicit gates in Roko's pipeline:

```rust
pub struct WisdomGate {
    min_hdc_diversity: f64,       // diversity of opinion
    max_lineage_overlap: f64,     // independence
    max_sender_share: f64,        // decentralization
    aggregator: Box<dyn Aggregator>, // aggregation method
}
```

Before a consensus Engram is finalized, its inputs must pass the
WisdomGate. If 80% of inputs share a lineage ancestor, that's not a
wisdom-of-crowds consensus — it's an echo chamber. **We can detect
and refuse echo chambers structurally.**

## 5. Aggregation methods as first-class

The fourth Surowiecki condition — aggregation — is where most systems
cheat by averaging. HDC gives us four genuinely different options:

1. **Bundle (majority vote)**: XOR-add fingerprints, binarize. Classic
   wisdom-of-crowds.
2. **Bind (structured)**: tag each agent's contribution with their
   identity fingerprint, bind, then bundle. Preserves *who said what*
   while still collapsing.
3. **Weighted bundle**: each fingerprint multiplied by agent's trust
   score for the topic. Bayesian flavor.
4. **Cleanup to codebook**: bundle, then snap to nearest known
   Engram. Forces output to be *expressible* in existing vocabulary.

Each has different properties under different team compositions.
Policy picks. Aggregation is no longer a hardcoded `mean()` but an
operator chosen based on c-factor stats.

## 6. Anti-groupthink primitives

A system optimizing for c can overshoot into groupthink if not
careful. Three countermeasures:

### 6.1 Devil's-advocate role

A canonical role-prompt whose job is to generate a *maximally-opposed*
Pulse on every consensus topic. Policy spawns one when HDC diversity
drops below threshold. This is the "red team" made structural.

### 6.2 Outsider-injection

Periodically, Policy routes a task to an agent with *zero lineage
overlap* with the active cohort. Its output is published but
labeled: downstream consumers know this is a deliberate outsider
perspective and weight accordingly.

### 6.3 Minority report preservation

Demurrage on *dissenting* Engrams is softer than on consenting
ones — we explicitly subsidize minority positions for longer. The
Bus carries a `consensus_distance` tag; high-distance Engrams get a
demurrage discount. This prevents the majority from simply starving
minority views of attention-credit.

## 7. c-factor as a dashboard tile

```
┌─ Cohort Intelligence (last 24h) ──────────────────────┐
│ c-factor: 0.72 (↑ 0.08 from last window)              │
│ turn-taking entropy:      2.31 / 3.00 (7 agents)      │
│ peer prediction accuracy: 61%                         │
│ citation reciprocity:     0.54                        │
│ delivery rate:            99.1%                       │
│ HDC diversity:            0.68                        │
│                                                       │
│ Weakest link: peer prediction (consider: rotate pairs)│
│ Groupthink risk:  LOW (min_pair_distance = 0.41)      │
└───────────────────────────────────────────────────────┘
```

This becomes a first-class tab on `roko dashboard`, and an API route
on `roko serve`.

## 8. Why most multi-agent frameworks can't measure this

Frameworks like LangGraph, AutoGen, CrewAI have agents, and some have
shared state. None have:

1. A **content-addressed substrate with lineage** — needed for
   citation-reciprocity and lineage-overlap metrics.
2. A **first-class Bus with delivery confirmation** — needed for
   channel-openness and turn-taking metrics.
3. An **HDC fingerprint on every artifact** — needed for diversity.
4. A **demurrage-driven attention economy** — needed to prevent
   echo-chamber drift without manual intervention.

Roko has or can easily add all four. c-factor measurement *falls out*
of the architecture rather than being bolted on. This is the
genuine moat: measuring collective intelligence is trivial when your
substrate is already designed for it.

## 9. Cross-cohort c: coalitions of coalitions

Once c is measured per cohort, the same math applies *between*
cohorts. A team-of-teams c-factor. This is the primitive for Phase
2+ chain architecture: chains of agents coordinating via
witness-signed Pulses can have their inter-cohort c measured and
optimized exactly the same way.

Cohorts with low c get merged (break silos). Cohorts with too-high c
and low diversity get split (break monocultures). The org-chart of
agent teams becomes self-tuning.

## 10. Implementation phases

1. **Metrics-only**: compute CohortMetrics from existing Bus and
   Substrate data. Log to `.roko/learn/c-factor.jsonl`. No behavior
   change. Two days of work.
2. **Dashboard tile + alerts**: expose in TUI and HTTP. One day.
3. **Passive optimization**: CohortWeights fit via active-inference;
   Policy reports c as a signal but doesn't act on it. One week.
4. **Active optimization**: Policy acts on c — temperature bumps,
   devil's-advocate spawning, outsider injection. Two weeks.
5. **Cross-cohort c and auto-org**: Phase 2. Open-ended.

Steps 1–2 are risk-free wins that surface information the team
already has. Steps 3+ get structurally interesting.

## 11. The net-new claim

There is published work measuring c in human teams, and there is
published work on multi-agent coordination. There is (to our
knowledge) no system that *measures c continuously in a running
agent runtime and closes the loop on it*. This combination is
specific to Roko's architecture, publishable, and a genuine
differentiator.

## 12. Fitting CohortWeights against outcomes

The regression from §2.3 isn't abstract; it's a small online learner
that subscribes to the Bus. Pseudo-Rust:

```rust
pub struct CohortWeights {
    pub a: f64,   // turn-taking entropy
    pub b: f64,   // peer prediction accuracy
    pub c: f64,   // citation reciprocity
    pub d: f64,   // delivery rate
    pub e: f64,   // HDC diversity
    pub bias: f64,
}

/// Policy that subscribes to cohort-completion Pulses and fits the
/// CohortWeights via online stochastic gradient on squared error vs
/// the observed outcome (gate-pass rate).
pub struct CohortWeightsLearner<B: Bus> {
    pub bus: Arc<B>,
    pub weights: parking_lot::RwLock<CohortWeights>,
    pub learning_rate: f64,         // e.g. 1e-3
}

impl<B: Bus> CohortWeightsLearner<B> {
    pub async fn run(self: Arc<Self>) {
        let filter = TopicFilter::Exact(Topic::new("cohort.completed"));
        let mut rx = self.bus.subscribe(filter).await.unwrap();
        while let Some(pulse) = rx.recv().await {
            let Some(obs) = parse_observation(&pulse) else { continue };
            let prediction = predict(&*self.weights.read(), &obs.metrics);
            let err = obs.outcome - prediction;
            let mut w = self.weights.write();
            w.a += self.learning_rate * err * obs.metrics.turn_taking_entropy;
            w.b += self.learning_rate * err * obs.metrics.peer_prediction_accuracy;
            w.c += self.learning_rate * err * obs.metrics.citation_reciprocity;
            w.d += self.learning_rate * err * obs.metrics.delivery_rate;
            w.e += self.learning_rate * err * obs.metrics.hdc_diversity;
            w.bias += self.learning_rate * err;
            drop(w);
            let _ = self.bus.publish(emit_weights_update(&*self.weights.read())).await;
        }
    }
}
```

The weights drift toward whatever configuration best explains observed
outcomes. Teams with different task distributions naturally end up
with different weights — Woolley's c isn't one number for everyone,
it's a shape of the regression learned in context. Confidence intervals
on the weights themselves are maintained via the same active-inference
machinery from `10-self-learning-cybernetic-loops.md`.

## 13. c-factor is a covariate, not an objective

Critical caveat (also in `15-exponential-scaling.md` §4.2): **the
Policy layer should not optimize c directly**. Optimizing c can be
trivially gamed by routing all work to easy tasks. The correct use:

1. **c is a measurement.** Publish it. Inspect it. Correlate it with
   outcomes.
2. **c is a diagnostic.** A drop in c alongside a drop in outcomes
   is a signal to intervene. A drop in c alongside stable outcomes is
   noise.
3. **c-optimization levers are applied conditionally.** Turn-taking
   temperature, devil's advocate, outsider injection — these fire
   when c is low *and* outcomes are suffering. Never when only c is
   low.

The loop is: observe c and outcome, compute correlation, intervene
on *process* variables when correlation predicts intervention will
help, measure again. This is standard regulatory control, not
reward hacking.

## 14. Cross-synergies

- **Demurrage (12)** §5 — agents accruing balance too fast
  (dominant voices) get agent-level demurrage that taxes their bid
  in the next turn. This directly lowers `max_sender_share` in the
  WisdomGate.
- **Heuristics (14)** §7 — peer-heuristic models are the
  operationalization of `peer_prediction_accuracy`. An agent that
  models its teammates' priors well scores high on that metric.
- **Worldview (14)** §5 — multiple active worldviews is the
  structural answer to the `hdc_diversity` axis.
- **Replication ledger (16)** — every agreed-upon claim gets
  measured twice, once by the cohort's aggregate and once by any
  individual. High cross-cohort c correlates with stable
  replications.
- **UX (23 §10, 30 §2.8)** — the c-factor and its components
  surface in the TUI and web UI as a tile. Operators can *see*
  what the group is doing well or poorly.
- **Deployment UX (24)** §5 — `roko.c_factor` is a first-class
  Prometheus metric; alerts on it fire like any other SLI.
