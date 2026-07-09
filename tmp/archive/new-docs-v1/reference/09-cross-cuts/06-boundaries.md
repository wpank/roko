# Cross-Cut Boundaries

> What a cross-cut may and may not do.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## TL;DR

Cross-cuts have two hard limits: they may not call external models or tools (ACT is
the only stage that crosses the agent boundary), and they may not write to the
Substrate outside of their designated injection points (PERSIST and Delta).

---

## What a Cross-Cut MAY Do

| Capability | Notes |
|---|---|
| Read from the Substrate | At any injection point |
| Write to the Substrate | At PERSIST stage and Delta only |
| Publish Pulses | At any injection point, via `Bus` ref |
| Maintain in-memory state | Must be reconstructable from Substrate on restart |
| Modify `TickContext` sub-fields | Only the fields it owns (see [Injection Model](04-injection-model.md)) |
| Call other L1 trait objects | Yes; e.g., Neuro may call `substrate.query()` |
| Fail gracefully | Cross-cut errors are non-fatal; the tick continues without the cross-cut |

---

## What a Cross-Cut MAY NOT Do

| Prohibition | Reason |
|---|---|
| Call a model API | That is ACT's job; cross-cuts must not create hidden costs |
| Call an external tool | Same reason; side effects must be audited through ACT |
| Write to the Substrate outside PERSIST / Delta | Would bypass VERIFY |
| Import from a higher layer (L3+) | Layer rule violation |
| Hold state that cannot be flushed to the Substrate | Prevents crash recovery |
| Block indefinitely | Cross-cut trait calls must complete within a stage's time budget |
| Change the routing decision directly | Only the Router trait may do this |
| Override a HardFail verdict | Only the Gate pipeline may do this |

---

## Graceful Degradation

When a cross-cut fails (returns an error from any trait method), `TickContext`
treats the failure as if the cross-cut were absent:

- Neuro failure: SCORE uses `utility = 0.5` (neutral default); QUERY uses basic
  substrate lookup (no HDC search).
- Daimon failure: SCORE uses `valence = 0.0`; ROUTE uses base thresholds.
- Dreams failure: Delta consolidation proceeds without replay/imagination.

The failure is logged and a `cross_cut.error` Pulse is published. The orchestrator
monitors this Pulse and may restart the cross-cut.

---

## See also

- [Injection Model](04-injection-model.md) — where cross-cuts touch the loop
- [Composition](05-composition.md) — how cross-cuts interact with each other
- [Dependency Rules](../08-layers/06-dependency-rules.md) — layer-level restrictions
