# S8 — Demurrage × Heuristics × calibration → Graceful relearning

> Confidence is not frozen. A heuristic that is not challenged has its confidence softened by
> demurrage so that fresh contradictory evidence can move it without a manual reset. The
> anti-stagnation mechanism prevents long-stable rules from dominating forever.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P6 Demurrage × P7 Heuristics × calibration loop (from S2)  
**Reality check**: P7 Heuristics are Scaffold; P6 Demurrage is Specified. The confidence-decay
mechanism is target-state. The calibration loop (from S2) is also target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P6 Demurrage](../../reference/) | Imposes a holding cost on confidence — a heuristic that is not exercised or challenged slowly loses confidence weight over time |
| [P7 Heuristics + falsifiers](../../subsystems/) | Learned rules with confidence weights; the target of the confidence-decay pressure |
| Calibration loop (S2) | The streaming evidence mechanism that can raise confidence when a heuristic performs well; demurrage is its complement for the case when evidence is sparse or absent |

---

## What the Synergy Unlocks

### The stagnation problem

A heuristic that is correct becomes highly confident over time. A highly confident heuristic
resists updating. If the world changes — the environment shifts, domain constraints evolve,
or a previously reliable pattern breaks — the highly confident heuristic may continue
dominating decisions even as its usefulness decays.

Without a confidence-softening mechanism, the only path to recovery is manual reset. A human
must identify which heuristic has stagnated, reduce its confidence, and wait for the calibration
loop to rebuild it under current conditions. This is expensive, slow, and error-prone.

### How it works

1. Each heuristic carries a confidence weight and a last-exercised timestamp.
2. Demurrage applies a small, continuous decay to confidence as a function of elapsed time since
   the heuristic was last exercised (used in a prediction and evaluated against an outcome).
3. The decay is bounded: confidence does not fall below a configurable floor (e.g., 0.1). The
   heuristic never becomes fully inactive; it merely requires less force to update.
4. When fresh evidence arrives — confirming or contradicting — the calibration loop (S2) can
   move the confidence more easily because demurrage has lowered the inertia.
5. If a heuristic is stagnant for long enough, its confidence softens to the floor and it may
   be surfaced for review (optional: a Bus event notifying the oversight system).

The result: confidence is never frozen. Long-stable rules can be moved by fresh evidence without
a manual reset. The system gracefully forgets what it no longer needs to be sure about.

### Why this is not the same as S2

S2 raises or lowers confidence in response to **active incoming evidence** (outcome Pulses).
S8 lowers confidence in response to **absence of evidence** (elapsed time without exercise).

S2 is reactive; S8 is preventive. Together they form a complete confidence management system:
- S2 ensures that confidence accurately tracks current performance.
- S8 ensures that confidence does not survive unchallenged long enough to become immovable.

### The economic intuition

Demurrage on Engrams prevents memory from accumulating indefinitely (S1). Demurrage on heuristic
confidence prevents *certainty* from accumulating indefinitely. The same economic primitive
applies at two different levels of the architecture: the storage level (what records to keep)
and the knowledge level (how confident to remain in a given rule).

---

## What Flows

```
Continuous decay (background process):
  for each heuristic in store:
    elapsed = now - heuristic.last_exercised
    confidence_decay = base_decay_rate × elapsed
    heuristic.confidence = max(floor, heuristic.confidence - confidence_decay)
    if heuristic.confidence ≤ floor + epsilon:
      optionally emit Pulse(topic=stagnant_heuristic, id=heuristic.id)

Evidence update (via S2):
  outcome Pulse arrives → calibration subscriber
  → evaluate falsifier
  → confidence += delta (bounded [floor, 1.0])
  → update heuristic.last_exercised = now
```

Note: the decay background process and the stagnant-heuristic Pulse are target-state.
The calibration subscriber is also target-state (see S2).

---

## Invariants

1. Confidence decay is monotonic over time when no evidence arrives: it never increases through
   demurrage alone.
2. The confidence floor is configurable per heuristic kind. Safety-critical heuristics may have
   a higher floor (they should never become too uncertain to act as guardrails).
3. Decay is additive with calibration: a heuristic that is both decaying and being calibrated
   has both forces applied. Evidence typically dominates because calibration deltas are larger
   than decay increments per unit time.
4. `last_exercised` is updated only when the heuristic is used **and** evaluated, not when it
   is merely consulted. A heuristic that is retrieved but not acted on does not reset its decay
   clock.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Decay too aggressive | Correct heuristics lose confidence faster than they can be reinforced by sparse evidence | Tune decay rate to be much smaller than typical calibration delta; or gate decay on exercise frequency |
| Confidence floor too low | Critical safety heuristics decay below a useful threshold in a low-evidence environment | Set per-kind floors; safety-kind heuristics have a higher floor |
| Decay-calibration race | Rapid decay and rapid calibration create an oscillating confidence value | Smooth calibration updates (as in S2); ensure calibration delta > decay increment per cycle |
| Stagnant heuristic flood | Many heuristics simultaneously reach the floor in a quiet period; Bus flooded with stagnant-heuristic events | Rate-limit stagnant events; batch them into a daily or session-level digest |

---

## Relationship to Other Synergies

- **S2** (Heuristics × Pulse × Bus): S8 is the complement of S2. S2 responds to evidence; S8
  responds to its absence. Both act on the same heuristic confidence weight.
- **S1** (Demurrage × HDC): S1 applies demurrage to Engram persistence; S8 applies demurrage to
  heuristic confidence. Same primitive, different target.
- **S4** (Replication ledger × Heuristics × paper Engram): A falsified claim in the ledger
  lowers heuristic confidence via the ledger propagation mechanism. S8's decay serves the same
  direction but through a different pathway (temporal vs. evidence-driven).
- **S6** (c-factor × Heuristics): Peer-model heuristics are subject to the same decay as world-
  model heuristics. An agent that does not interact with a peer for a long time should become less
  certain of its peer model.

---

## Today vs. Planned

**Today**: Heuristics are stored in Substrate with confidence weights. No decay background
process exists. No stagnant-heuristic Bus event exists. Confidence is manually managed.

**Planned**: A background decay process runs on the Heuristics store. Demurrage rates are
configurable per heuristic kind. The decay integrates with the calibration loop so that the
two forces are applied consistently on the same confidence value.

---

## Cross-References

- [`analysis/synergy-map/synergy-02-heuristics-pulse-bus.md`](synergy-02-heuristics-pulse-bus.md) — S2: the evidence-driven complement of S8
- [`analysis/synergy-map/synergy-01-demurrage-x-hdc.md`](synergy-01-demurrage-x-hdc.md) — S1: demurrage on Engrams (parallel application of the same primitive)
- [`analysis/synergy-map/synergy-04-replication-living-research.md`](synergy-04-replication-living-research.md) — S4: ledger-driven confidence reduction (compare with temporal decay)
- [`analysis/readiness-audit/subsystem-learning.md`](../readiness-audit/subsystem-learning.md) — learning gaps relevant to confidence management
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- Should decay rate be linear in elapsed time, or sub-linear (so that very old heuristics
  decay more slowly, approaching the floor asymptotically)?
- Can the system detect when it has entered a "low-evidence regime" and temporarily pause
  confidence decay to avoid unnecessary softening during quiet periods?
- Should the stagnant-heuristic notification include a suggested action (e.g., "design a test
  to exercise this heuristic") or just flag it for review?
