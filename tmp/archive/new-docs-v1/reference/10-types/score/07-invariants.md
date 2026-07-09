# Score — Invariants

> What must always be true about a Score value.

**Status**: Shipping  
**Crate**: `roko-core`  
**Last reviewed**: 2026-04-19

---

## Invariants

| # | Invariant | Enforced by |
|---|-----------|-------------|
| S1 | `confidence ∈ [0.0, 1.0]` | `Score::validate()`, `Substrate::update_score()` |
| S2 | `novelty ∈ [0.0, 1.0]` | `Score::validate()`, `Substrate::update_score()` |
| S3 | `utility ∈ [0.0, 1.0]` | `Score::validate()`, `Substrate::update_score()` |
| S4 | `reputation ∈ [0.0, 1.0]` | `Score::validate()`, `Substrate::update_score()` |
| S5 | `precision.map(|v| v ∈ [0.0, 1.0])` | `Score::validate()` |
| S6 | `salience.map(|v| v ∈ [0.0, 1.0])` | `Score::validate()` |
| S7 | `coherence.map(|v| v ∈ [0.0, 1.0])` | `Score::validate()` |
| S8 | `effective() ∈ [0.0, 1.0]` | Follows from S1–S4 and weight sum = 1.0 |
| S9 | Default weights sum to 1.0: `W_CONFIDENCE + W_NOVELTY + W_UTILITY + W_REPUTATION = 1.0` | Compile-time assertion |
| S10 | `utility` is monotonically non-decreasing while the Engram is being retrieved successfully | Convention; not technically enforced |

---

## Monotonicity Note

`utility` is expected to increase as an Engram proves useful and decrease when it
contributes to failures. It is not strictly monotonic — it can go both up and down.
The invariant is that it is clamped to [0.0, 1.0] and updated by fixed deltas.

---

## See Also

- [`03-arithmetic.md`](03-arithmetic.md) — effective score formula
- [`05-api-reference.md`](05-api-reference.md) — `validate()` method
