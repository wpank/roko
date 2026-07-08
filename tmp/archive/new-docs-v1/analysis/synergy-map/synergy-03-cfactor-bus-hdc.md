# S3 — c-factor × Bus × HDC → Diversity-aware routing

> Bus statistics reveal how work is being distributed. HDC signals whether the system's
> representations are converging too tightly. c-factor consumes both and pushes policy toward
> role diversity, model diversity, or pair rotation when the system is becoming too homogeneous.
> This is regulation, not just observability.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P8 c-factor × P3 Bus × P5 HDC  
**Reality check**: P8 c-factor is partially built; P5 HDC fingerprinting is partially built;
P3 Bus (generalized trait with topic statistics) is Scaffold. The regulatory loop that consumes
all three is target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P8 c-factor](../../reference/) | Collective intelligence signal; measures diversity of cognitive contributions; serves as the policy throttle for homogeneity correction |
| [P3 Bus / `EventBus<E>`](../../reference/04-bus/) | Transport surface whose statistics (per-role Pulse volume, per-model Pulse origin) expose work-distribution patterns |
| [P5 HDC fingerprint](../../reference/) | Encodes the semantic content of system outputs; convergence in fingerprint space signals representational monoculture |

---

## What the Synergy Unlocks

### The monoculture problem

A multi-agent system that optimizes locally tends toward homogeneity: the agents, models, and
roles that perform best today get selected more often, creating a positive feedback loop. This
reduces cognitive diversity and makes the system brittle — it becomes very good at problems it
has seen and very bad at problems it has not.

The standard fix is diversity injection via random assignment. That works but wastes capacity.

The synergy's fix: **measure monoculture continuously and correct precisely**.

### How it works

1. The Bus records per-topic, per-origin Pulse statistics: which agents, roles, and models are
   generating output on which topics.
2. The HDC fingerprint index compares the semantic content of recent outputs: if the last N
   outputs from the system cluster tightly in fingerprint space, the system is in representational
   monoculture — it is saying the same thing in many ways.
3. c-factor ingests both signals: low Pulse-origin diversity (few agents generating most traffic)
   AND low HDC diversity (outputs converging semantically) produce a high homogeneity score.
4. When the homogeneity score crosses a threshold, c-factor emits a policy signal: expand the
   role roster, rotate pairs, inject a contrarian model, or widen the topic filter.
5. The policy signal feeds the routing layer, which adjusts assignment for the next N tasks
   without requiring human intervention.

The result: the system watches for monoculture and actively corrects toward broader cognitive
variety. It is a regulatory loop, not a monitoring dashboard.

### Why each primitive is necessary

- Without Bus statistics, c-factor can only measure diversity at batch boundaries (post-hoc).
- Without HDC fingerprints, c-factor can measure who is generating output but not whether the
  outputs are semantically different from each other.
- Without c-factor as the integrating policy signal, both Bus stats and HDC diversity metrics are
  informational but produce no routing action.

---

## What Flows

```
Measurement (continuous):
  Bus.statistics(topic, window) → per-origin Pulse counts
  Substrate.query_similar(recent_outputs, k=all) → fingerprint cluster radius

c-factor computation:
  homogeneity_score = f(origin_gini_coefficient, fingerprint_cluster_radius)
  if homogeneity_score > threshold:
    emit PolicyPulse(kind=DiversityCorrection, payload={rotate_roles, widen_models})

Routing adjustment:
  Routing layer receives PolicyPulse
  → adjust assignment weights for next N tasks
  → log correction event to Substrate for audit
```

Note: Bus statistics API and the PolicyPulse types are target-state. The live `EventBus<E>`
has per-subscriber delivery but no aggregate statistics API.
See [`reference/04-bus/14-today-vs-planned.md`](../../reference/04-bus/14-today-vs-planned.md).

---

## Invariants

1. Diversity corrections are non-punitive. The loop adjusts assignment weights; it does not
   evict or suspend agents.
2. c-factor consumes diversity signals passively. It does not write into the Bus or directly
   modify fingerprint records — it emits policy Pulses that downstream routing chooses to act on.
3. Homogeneity corrections decay. After N tasks, assignment weights return to baseline unless
   the homogeneity signal persists.
4. The diversity correction signal is recorded as an Engram. If corrections become frequent, the
   pattern is discoverable in Substrate history.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Overcorrection thrash | Frequent diversity corrections create instability; no pair is allowed to specialise | Add hysteresis: require homogeneity_score to stay elevated for M consecutive windows before triggering |
| False monoculture signal | HDC fingerprints cluster because the task domain is genuinely narrow, not because of homogeneity | Gate HDC diversity check on task-type label; do not penalise convergence on domain-specific work |
| Bus stats lag | Window too large; homogeneity correction arrives after the pattern has already resolved | Tune window size per deployment; offer configurable short-window alerts |
| c-factor calibration drift | The c-factor model itself becomes miscalibrated over time | S6 (peer-model learning) applies to c-factor's own predictors; see [`synergy-06`](synergy-06-cfactor-heuristics-peer-model.md) |
| Routing layer ignores signal | PolicyPulse is emitted but the routing layer has no subscriber | Routing layer must subscribe to the diversity-correction topic as part of its wiring contract |

---

## Relationship to Other Synergies

- **S1** (Demurrage × HDC): S1 uses HDC to trim semantically redundant *memory*. S3 uses HDC
  to detect semantically redundant *outputs*. Same primitive, different application layer.
- **S6** (c-factor × Heuristics): S3 drives the policy signal that S6 helps calibrate. The
  peer-model predictions in S6 can feed into the c-factor computation that S3 consumes.
- **S9** (HDC × Consensus × Bus): S9 uses HDC for agreement detection. S3 uses HDC for
  diversity detection. Opposite use-case of the same fingerprint primitive.

---

## Today vs. Planned

**Today**: `EventBus<E>` routes typed events. HDC vectors are computed for Engrams. c-factor
exists as a partial metric in code. No aggregate Bus statistics API, no fingerprint-based
diversity signal, no diversity-correction PolicyPulse type.

**Planned**: Bus gains a per-topic statistics API (Pulse origin counts, rate, distribution).
c-factor gains a regulatory emitter that publishes PolicyPulse on a configurable threshold.
Routing layer gains a diversity-correction subscriber.

---

## Cross-References

- [`analysis/architectural-analysis/08-novel-proposals.md`](../architectural-analysis/08-novel-proposals.md) — diversity-aware routing as a novel proposal
- [`analysis/integration-map/learning-x-routing.md`](../integration-map/learning-x-routing.md) — M6: routing-learning integration edge that carries c-factor policy
- [`analysis/readiness-audit/subsystem-neuro.md`](../readiness-audit/subsystem-neuro.md) — HDC readiness
- [`analysis/synergy-map/synergy-06-cfactor-heuristics-peer-model.md`](synergy-06-cfactor-heuristics-peer-model.md) — S6: c-factor calibration
- [`analysis/synergy-map/synergy-09-hdc-consensus-agreement.md`](synergy-09-hdc-consensus-agreement.md) — S9: related HDC use-case (agreement detection)

---

## Open Questions

- What is the right diversity metric — Gini coefficient on Pulse origins, or entropy?
- Should HDC diversity be computed on raw outputs, on the Engrams those outputs produce, or
  on some intermediate representation?
- Can the c-factor regulatory loop itself become a source of monoculture if all correction
  signals push toward the same "diverse" pattern?
