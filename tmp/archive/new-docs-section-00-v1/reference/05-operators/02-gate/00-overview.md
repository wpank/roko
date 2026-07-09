# Gate Overview

> The `Gate` operator produces a `Verdict` — Pass, Fail/Reject, or Abstain — for an
> `Engram` and its `Score`. It is the cognitive loop's filter: nothing proceeds past a Gate
> that returns `Reject`.

**Status**: Shipping
**Crate**: `roko-gate`
**Depends on**: [Score](../../10-types/score.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A `Gate` answers one question: "Should this information proceed?" A `Pass` verdict lets the
`Engram` continue to the Router. A `Reject` verdict stops the loop tick for this `Engram`.
An `Abstain` means the gate has no opinion — let the next gate decide.

---

## Why a Gate?

The Scorer tells you how good an `Engram` is. The Gate decides whether that is good enough.
Scoring and filtering are distinct concerns:

- **Scorer**: "This memory has confidence=0.3 and novelty=0.9."
- **Gate**: "confidence < 0.5 means Reject — do not act on this."

Separating them means you can change the threshold independently of the scoring algorithm.
You can also have multiple gate criteria (safety gate, quality gate, coherence gate) applied
in a pipeline without tangling them.

---

## Three Verdicts (Not Two)

Gate returns one of three verdicts, not simply pass/fail:

| Verdict | Meaning |
|---|---|
| `Pass` | This gate approves the Engram. The pipeline continues. |
| `Reject(reason)` | This gate rejects the Engram. The loop tick ends with `LoopOutcome::Rejected`. |
| `Abstain` | This gate has no opinion. Skip to the next gate. |

`Abstain` is not a failure — it is an explicit "not my decision." A gate that is not
applicable to the current input (e.g., a safety gate triggered only by specific `Kind`
values) returns `Abstain` rather than `Pass` to avoid accidentally approving things it
cannot evaluate.

---

## Where Gate Fits in the Loop

```
SCORE → GATE ← gate.evaluate(engram, score)
           │
      Reject? → LoopOutcome::Rejected
           │
          Pass → ROUTE
```

Gate is step 4 of 7. It runs after scoring and before routing.

---

## Gate Pipelines

Multiple `Gate` implementations are stacked into a pipeline. The loop calls gates in order:
the first `Reject` ends the pipeline; `Abstain` is skipped; `Pass` continues to the next
gate. Only if all gates pass (or abstain) does the loop proceed to routing.

This allows a 7-rung "gauntlet" where an `Engram` must pass: quality, safety, coherence,
freshness, authority, relevance, and domain gates before reaching the Router.

See [Gate Composition](./09-gate-composition.md).

---

## See Also

- [Semantics](./02-semantics.md) — the three-verdict model in depth
- [Gate Composition](./09-gate-composition.md) — pipeline architecture
- [Scorer Overview](../01-scorer/00-overview.md)
