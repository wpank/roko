# Net-New Innovations Catalog

> **TL;DR**: This doc is a flat catalog of primitives, patterns,
> and APIs that (to our knowledge) no other agent framework has.
> Each entry names the primitive, states the problem it solves,
> and names the closest prior art. The catalog is the pitch deck
> for "what does Roko let you do that nothing else does?"

> **For first-time readers**: This is the pitch-deck doc. Each bullet
> is cross-referenced to the refinement doc where it lives in depth.
> If you want the "what's novel" summary in one page, this is it. If
> you want to evaluate novelty honestly, note §4 below — no primitive
> here is unique in isolation; the composition is.

## 1. Primitives

### 1.1 `Pulse` as a first-class type (ephemeral medium)

What: a typed, sequence-numbered, ring-buffered in-flight message
with explicit graduation to durable Engram.
Closest prior art: Kafka messages, ROS topics. Neither has the
graduation pattern or the content-addressed bridge.
Why net-new: the explicit *lifecycle* `Pulse → Engram` and the
symmetric operator traits that work on either medium are unique.

### 1.2 HDC fingerprint on every Engram

What: a 10,240-bit hyperdimensional fingerprint attached at write
time, enabling O(1) similarity, consensus, analogy.
Closest prior art: vector databases (Pinecone, Weaviate). These use
dense float embeddings from an LLM; we use structural HDC with
compositional binding.
Why net-new: *every* Engram in the core substrate gets one, not
just text; fingerprints compose via XOR/permute/bundle without
re-embedding.

### 1.3 Demurrage as the decay primitive

What: economic memory management where balance is taxed over time
and restored by use/citation/surprise.
Closest prior art: LRU, TTL, decay scores in recommender systems.
None encode *reinforcement-by-kind* (cited vs surprised vs retrieved
vs used).
Why net-new: a single primitive that produces self-trimming
playbooks, graceful forgetting, and cold-tier graduation with one
rate law.

### 1.4 Heuristic with explicit falsifier

What: a first-class Engram type that states preconditions,
prediction, and *what would prove it wrong*, with ongoing
calibration.
Closest prior art: rule-based systems (Soar, ACT-R), vector-store
retrieved facts.
Why net-new: the *falsifier* is mandatory, and calibration updates
happen from live Bus traffic without manual evaluation.

### 1.5 Replication ledger

What: per-paper, per-claim ledger of observed vs reported effect,
with confidence intervals and status.
Closest prior art: nothing, honestly, in agent frameworks. In
science, registered replications or OSF.
Why net-new: an agent runtime that *replicates its own
underpinnings continuously*. No prior framework treats its own
research basis as mutable empirical data.

### 1.6 c-factor as a runtime signal

What: the Woolley c-factor computed continuously from Bus and
Substrate statistics, optimized by Policy.
Closest prior art: team-metrics dashboards, software-engineering
health indicators (DORA).
Why net-new: applies c-factor theory to a population of *agents*,
uses it to drive routing decisions, surfaces it in user-facing
dashboards.

### 1.7 Worldview as emergent object

What: a coherent cluster of co-citing heuristics that dominate a
domain-fingerprinted region of situations.
Closest prior art: topic models (LDA), community detection.
Why net-new: worldviews emerge from *citation + outcome* rather
than textual co-occurrence, and they're challenged by active
devil's-advocate and outsider-injection machinery.

### 1.8 Two-fabric operator generalization

What: every kernel operator (Scorer, Gate, Router, Composer,
Policy) operates on either durable Engrams or ephemeral Pulses via
a unified trait.
Closest prior art: stream-processing frameworks (Flink, Dataflow)
distinguish bounded from unbounded but rarely unify them at an
operator-trait level.
Why net-new: lifts an algebraic property (operators are
medium-polymorphic) into a Rust-level trait and a coherent mental
model.

### 1.9 Demurrage-taxed learned parameters

What: every learned parameter has confidence that decays when
unchallenged, enabling graceful relearning.
Closest prior art: Kalman filters have a related uncertainty
inflation; most ML systems do not.
Why net-new: applied to the entire Policy-parameter space, not to
individual estimators.

### 1.10 Prediction markets on heuristics

What: agents stake balance-credits on heuristic outcomes; aggregate
stake is a secondary trust signal.
Closest prior art: Hanson prediction markets, internal enterprise
forecasting platforms.
Why net-new: runs inside a single coding-agent runtime as a
mechanism for *belief price discovery*.

## 2. Patterns

### 2.1 Predict-publish-correct loops

Any operator publishes a prediction Pulse, a later Pulse confirms
or refutes, a learning Policy joins and updates the operator. This
makes the operator *itself* a learner without bespoke training
code.

### 2.2 Stigmergy via Engrams

Agents deposit observations that other agents read later, producing
indirect coordination without handoff. Grassé's model but with
typed content-addressed artifacts.

### 2.3 Chain witnesses for empirical knowledge

Phase 2 primitive: chain-witnessed Engrams carry a signature trail
that increases trust multiplicatively across deployments. Aim is
empirical, not financial — a "proof-of-replication" network.

### 2.4 Dream cycles

Offline consolidation passes that re-read old episodes through
current heuristics, producing retroactive learning. `roko-dreams`
is the crate; pattern is McClelland's complementary-learning-systems
hypothesis made operational.

### 2.5 Role-taking through peer prediction

Each agent predicts what each other agent would say, trains against
reality, and the prediction accuracy *itself* is a c-factor input.
Mead's role-taking as an algorithmic commitment.

### 2.6 Dissonance as a learning signal

When two heuristics disagree on a current situation, that's a
high-information-content event. Surface it, resolve it, update
both. Festinger's cognitive dissonance as a priority in the
scheduler.

### 2.7 Plugin tiers by risk/power

From `17`: five tiers (prompts, profiles, manifests, native, WASM)
each with matched sandboxes. Most frameworks collapse this into
"extensions" with one security model.

## 3. APIs users can write against

### 3.1 `roko heuristic` CLI

Externalized agent beliefs. No other framework exposes this.

### 3.2 `roko dashboard` with c-factor tile

Continuously-updated collective-intelligence metrics.

### 3.3 `roko plugin` CLI with registry

Discoverable extensions with verified metadata.

### 3.4 Bus subscription API

External consumers can subscribe to any topic (with auth). A
monitoring dashboard, a Slack notifier, an audit log — all
subscribers, not integrations.

### 3.5 HDC query API

`substrate.query_similar(fingerprint, k=10)` as a first-class
Substrate method, returning Engrams regardless of type. Cross-cutting
semantic retrieval over the entire knowledge store.

### 3.6 Replication ledger API

`GET /claims/<id>/replication` returns the current status. Third
parties can audit empirical support for any design parameter.

## 4. What this catalog is *not*

Not every primitive here is unique in isolation. HDC exists.
Demurrage exists. c-factor research exists. Prediction markets
exist. *The composition* of all of them, in one coherent Rust
system, wired through a common Substrate and Bus — that composition
is the net-new artifact. Individual primitives are the building
blocks; the moat is the whole.

## 5. Priority for research-style publications

A few of these could reach the "publishable" bar given six to
twelve months of deployment data:

- **c-factor measurement in agent systems** (ICLR / AAAI / NeurIPS
  workshop).
- **Demurrage-based memory management for LLM agents** (ACL / EMNLP
  systems track).
- **Replication ledger as evidence-based engineering** (CHI /
  Empirical SE venues).
- **HDC compositional memory for code agents** (a cog-sci-adjacent
  venue or NeurIPS workshop).

Publishing is itself moat-building (see `18` §2.5). Pick one per
quarter.

## 6. Catalog maintenance

This doc should be updated every time a new primitive lands.
Deleting entries when they're matched by prior art is fine — the
catalog's job is to be honest about what's actually novel, not to
pad the pitch. Honesty in this doc compounds trust with engineers
reading it.

## 7. Cross-doc index for each entry

So each primitive points to its home doc:

| Primitive | Home doc |
|---|---|
| 1.1 Pulse / graduation | 02 |
| 1.2 HDC per Engram | 11 |
| 1.3 Demurrage | 12 |
| 1.4 Heuristic with falsifier | 14 |
| 1.5 Replication ledger | 16 |
| 1.6 c-factor | 13 |
| 1.7 Worldview clusters | 14 §4 |
| 1.8 Two-fabric operator | 04 |
| 1.9 Demurrage-taxed parameters | 12 §5 |
| 1.10 Prediction markets on heuristics | 15 §5.1 |
| 2.1 Predict-publish-correct loops | 10 §2 |
| 2.2 Stigmergy via Engrams | 09 §3 |
| 2.3 Chain-witnessed heuristics | 09 §5, 18 §2.4 |
| 2.4 Dream cycles | 09 §2 |
| 2.5 Peer-prediction role-taking | 13 §3.2, 14 §7 |
| 2.6 Dissonance as learning signal | 14 §8 |
| 2.7 Plugin tiers by risk/power | 17 |
| 3.1 `roko heuristic` CLI | 14 §9 |
| 3.2 `roko dashboard` c-factor tile | 13 §7 |
| 3.3 `roko plugin` CLI / registry | 17 §6–§7 |
| 3.4 Bus subscription API | 27 |
| 3.5 HDC query API | 11 §4 |
| 3.6 Replication ledger API | 16 §5 |

## 8. What's genuinely new vs carefully integrated

Rereading the list with an honest eye, three entries are genuinely
primitive (not seen in any prior agent framework):

- **1.4 Heuristic with explicit falsifier** — most frameworks store
  tips as retrievable text; forcing a falsifier and calibrating
  against lived experience is new.
- **1.5 Replication ledger** — no agent framework has tracked
  whether its own design assumptions hold up empirically over time.
- **1.6 c-factor as a runtime signal** — Woolley's metric has lived
  in HR research; running it on agents continuously is new.

The remaining seven primitives in §1 are integrations: HDC, demurrage,
Pulse, two-fabric operators, prediction markets, peer-prediction, plugin
tiers — each has prior art, and the novelty is their *fit* with the
Roko substrate. That's still a moat (per 18 §2.1) but it's a different
kind of claim. Readers deserve the distinction.

## 9. Candidate additions as the refinements land

Primitives that would join the catalog once their home docs
are implemented:

- **TypedContext + Custody** (25 §8): structured domain vocabulary
  that gates can match against, plus a chain-of-custody record for
  every auditable action.
- **StateHub typed projections** (26): live, filterable, typed
  views over the Bus + Substrate, shared across UIs.
- **Realtime wire protocol** (27): the unified subscribe/query/publish
  vocabulary across WebSocket/SSE/gRPC.
- **Plan revision via `gate.verdict.emitted`** (05 §7): a one-liner
  closure of the self-hosting loop.
- **Demurrage-driven auto-GC of playbooks** (12 §4.1): self-trimming
  agent memory without manual GC.
- **Claim-resolved config parameters** (`claim!` macro, 16 §6):
  engineering decisions that self-audit against empirical evidence.

Each becomes a catalog entry once the home doc ships an
implementation.
