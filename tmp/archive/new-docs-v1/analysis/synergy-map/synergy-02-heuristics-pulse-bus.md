# S2 — Heuristics × Pulse × Bus → Continuous calibration

> Heuristics carry explicit falsifiers. Pulses on the Bus deliver live evidence of whether each
> heuristic helped, failed, or needs tightening. Calibration becomes a streaming process rather
> than a periodic audit.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P7 Heuristics + falsifiers × P2 Pulse × P3 Bus  
**Reality check**: P7 Heuristics are Scaffold (struct stubs, no runtime calibration loop).
P2 Pulse and the generalized P3 Bus trait are target-state. The live `EventBus<E>` provides a
partial foundation. Full streaming calibration is target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P7 Heuristics + falsifiers](../../subsystems/) | Learned rules of thumb; each heuristic carries a falsifier condition that specifies what evidence would invalidate it |
| [P2 Pulse](../../reference/02-pulse/) | Ephemeral, typed wire event that carries outcome evidence (did the heuristic's prediction hold?) |
| [P3 Bus / `EventBus<E>`](../../reference/04-bus/) | The transport surface that routes outcome Pulses to the calibration listener subscribed to each heuristic's topic |

---

## What the Synergy Unlocks

Without Pulse + Bus, calibration must happen in batch: after a session, an external process
reads the heuristic store, collects outcome data from logs, and adjusts confidence weights.
This is slow, fragile, and requires a separate pipeline.

Without explicit falsifiers on heuristics, outcome Pulses arrive but cannot be routed: the
system does not know which heuristic to update when a particular outcome is observed.

With all three:

1. A heuristic is written as a predicate + confidence + a falsifier condition (e.g., "if
   `outcome.score < threshold` after using this rule, decrement confidence").
2. When an action succeeds or fails, the Cognitive Loop publishes an outcome Pulse on the Bus,
   typed by the heuristic topic it relates to.
3. The calibration listener subscribes to those topics and routes each Pulse to the relevant
   heuristic's falsifier evaluator.
4. Confidence adjusts continuously — the heuristic becomes more trusted if it predicted well,
   less trusted if it predicted poorly, without any manual intervention.

The practical effect is **continuous learning from lived outcomes**. A heuristic is not trusted
because it exists; it is trusted because it has survived repeated contact with relevant Pulses.

### Why this is not just a learning loop

Standard learning loops require labeled data, a training signal, and a separate update pass.
This synergy turns lived experience into calibration material at the moment the experience
occurs. The training signal is implicit in the outcome Pulse; the "training pass" is a
microsecond falsifier evaluation on the Calibration subscriber.

---

## What Flows

```
Prediction phase:
  Agent consults heuristic store → selects matching heuristics → composes action

Outcome phase (continuous):
  Cognitive Loop produces outcome → publish Pulse(topic=heuristic.topic, evidence=outcome)
  Bus routes Pulse to Calibration subscriber

Calibration phase (per-Pulse):
  Calibration subscriber receives Pulse
  → evaluate falsifier condition against evidence
  → if falsifier fires: confidence -= delta
  → if prediction confirmed: confidence += delta
  → write updated confidence to Substrate
```

Note: the live system today has `EventBus<E>` and Substrate writes. The calibration subscriber,
the Pulse typing, and the falsifier-evaluation loop are target-state.
See [`reference/04-bus/07-backend-event-bus.md`](../../reference/04-bus/07-backend-event-bus.md).

---

## Invariants

1. Every heuristic that enters the store carries at least one falsifier condition. A heuristic
   without a falsifier cannot participate in streaming calibration.
2. Confidence values are bounded in `[0.0, 1.0]`. Calibration never drives confidence outside
   this range.
3. Calibration is monotonically responsive: a confirmed prediction always raises confidence; a
   falsifier always lowers it. Cross-directional updates to a single heuristic in the same tick
   are resolved by priority (falsifier wins).
4. The calibration update for heuristic H is only triggered by Pulses on H's topic — not by
   global outcome events.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Confidence collapse | A flawed heuristic receives many falsifier-firing Pulses and drops to zero; never recovers | Add a floor (e.g., 0.05) below which calibration is frozen until manual review |
| Calibration thrash | Opposing Pulses (confirm + falsify) arrive in the same window; confidence oscillates | Smooth confidence updates with a decay factor; batch updates over a window |
| Topic misrouting | Outcome Pulses tagged with the wrong heuristic topic update the wrong rule | Require typed topic enums (not free strings) in the Pulse schema; validate at publish |
| Calibration starvation | A correct heuristic is never exercised; confidence decays via S8 demurrage without any reinforcing Pulses | Treat zero-Pulse heuristics as needing review; surface in `readiness-audit` gaps |
| Bus backpressure | High outcome-Pulse volume causes the calibration subscriber to lag | Calibration subscriber runs in a non-blocking sink; drops if backpressured (outcome data is eventually consistent, not critical-path) |

---

## Relationship to S8 (Graceful Relearning)

S2 and [S8](synergy-08-demurrage-heuristic-relearning.md) share P7 Heuristics but attack
different problems:

- **S2** drives confidence changes from evidence — active incoming Pulses.
- **S8** drives confidence softening from **absence** — when a heuristic is not exercised,
  demurrage on confidence ensures it cannot dominate forever.

Together they form a complete calibration regime: S2 responds to evidence; S8 guards against
stagnation when evidence is sparse.

---

## Today vs. Planned

**Today**: Heuristics can be stored in Substrate as `Signal`/`Engram` variants. `EventBus<E>`
can route typed events. No falsifier-evaluation subscriber exists. Calibration is manual.

**Planned**: The Heuristics subsystem gains a falsifier runtime; the Bus gains typed topic
routing; an always-running Calibration subscriber subscribes to outcome topics and applies
per-Pulse confidence updates.

---

## Cross-References

- [`analysis/architectural-analysis/08-novel-proposals.md`](../architectural-analysis/08-novel-proposals.md) — continuous calibration named as a novel proposal
- [`analysis/readiness-audit/subsystem-learning.md`](../readiness-audit/subsystem-learning.md) — learning subsystem readiness, gaps
- [`analysis/synergy-map/synergy-08-demurrage-heuristic-relearning.md`](synergy-08-demurrage-heuristic-relearning.md) — S8: complementary confidence-softening mechanism
- [`analysis/synergy-map/synergy-06-cfactor-heuristics-peer-model.md`](synergy-06-cfactor-heuristics-peer-model.md) — S6: heuristics applied to peer-model learning
- [`analysis/integration-map/learning-x-composition.md`](../integration-map/learning-x-composition.md) — M4: the integration edge between learning and composition that carries heuristic output

---

## Open Questions

- Should calibration updates be transactional (write confidence atomically with the outcome
  Pulse acknowledgment) or eventually consistent?
- What is the right confidence delta per Pulse — fixed, or proportional to the heuristic's
  current confidence (larger deltas for uncertain rules, smaller for confident ones)?
- How are conflicting heuristics (two rules predicting opposite outcomes, both with high
  confidence) surfaced for human review?
