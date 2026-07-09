# Scorer Overview

> The `Scorer` operator assigns a [`Score`](../../10-types/score.md) to an `Engram`. It is
> the appraisal step — the mechanism by which the cognitive loop decides what is worth
> keeping, acting on, and learning from.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Engram](../../01-engram/README.md), [Score](../../10-types/score.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Scorer::score(engram, prior_score) -> Score` takes an `Engram` and the accumulated score
from earlier scorers in the chain, and returns an updated `Score`. Multiple scorers are
stacked; the last scorer's output is the final score for the loop tick.

---

## What Scoring Does

Before an agent can act on a piece of information, it needs to know how much to trust it
(confidence), how new it is (novelty), whether it is useful for the current task (utility),
and who produced it (reputation). These four stable axes plus three extended axes form the
7-axis `Score`. The Scorer's job is to populate those axes.

Without scoring, all memories look equally important. With scoring, the cognitive loop can
prioritise high-confidence, high-utility memories in recall, prefer novel information for
learning, and distrust low-reputation sources.

---

## The 7 Axes

| Axis | Type | Range | Stable? |
|---|---|---|---|
| `confidence` | `f32` | [0.0, 1.0] | Stable |
| `novelty` | `f32` | [0.0, 1.0] | Stable |
| `utility` | `f32` | [0.0, 1.0] | Stable |
| `reputation` | `f32` | [0.0, 1.0] | Stable |
| `precision` | `f32` | [0.0, 1.0] | Extended |
| `salience` | `f32` | [0.0, 1.0] | Extended |
| `coherence` | `f32` | [0.0, 1.0] | Extended |

The four stable axes are required for every `Score`. The three extended axes default to `0.5`
(neutral) if not set. See [Score Type](../../10-types/score.md) for the full definition.

---

## Where Scoring Fits in the Loop

```
SENSE → RECALL → SCORE ← scorer.score(engram, prior)
                   ↓
                  GATE → gate.evaluate(engram, score)
                   ↓
                 ROUTE → router.route(engram, score)
```

Scoring is step 3 of 7. It runs after recall (the agent has retrieved relevant memories)
and before gating (the agent decides whether to proceed).

---

## Stacking

When multiple `Scorer` implementations are registered, they are applied in order. Each
scorer receives the `prior` score from the previous scorer:

```
Engram → Scorer A (returns score_A)
               → Scorer B (receives score_A as prior, returns score_B)
               → Scorer C (receives score_B as prior, returns score_C)
               → final score = score_C
```

This means scorers can build on each other: a `RecencyScorer` sets `novelty`; a
`ConfidenceScorer` sets `confidence`; a `UtilityScorer` sets `utility`. None interfere.

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Semantics](./02-semantics.md)
- [Composition Patterns](./09-composition-patterns.md)
- [Gate Overview](../02-gate/00-overview.md) — what happens after scoring
