# Scorer Invariants

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

## Invariants

**I1 — Axis Range**: Every axis in the returned `Score` must be in [0.0, 1.0]. NaN and
infinity are bugs.

**I2 — Prior Preservation**: Axes not modified by a scorer must be returned with the same
value they had in `prior`. A scorer that only sets `confidence` must not change `novelty`,
`utility`, or other axes.

**I3 — Determinism**: Given the same `Engram` and the same `prior`, a scorer must return
the same `Score` (pure function, no hidden mutable state).

**I4 — No Side Effects on Substrate**: A scorer must not call `substrate.put()` or
`substrate.prune()`. Scorers are read-only observers.

**I5 — No Panic**: Scorers must return `Err(ScorerError)` rather than panicking.

---

## See Also

- [Failure Modes](./06-failure-modes.md)
- [Semantics](./02-semantics.md)
