# Scorer Failure Modes

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

<!-- ADDED: expanded from thin source section -->

## Failure Catalogue

### F1 — NaN Propagation

**Scenario**: A computation (e.g., HDC fingerprint distance from a zero-length vector)
produces NaN.

**Behaviour**: The scorer must detect NaN before returning and return
`ScorerError::InvalidValue { axis: "novelty", value: NaN }` (or clamp to a fallback value
like 0.5 with a warning log).

**Recovery**: The cognitive loop catches `ScorerError::InvalidValue`, logs the error, and
uses `Score::default()` (all axes = 0.5) for the current tick.

---

### F2 — Missing Substrate Context

**Scenario**: A scorer that computes `novelty` via fingerprint distance cannot access the
substrate (e.g., substrate is locked by another writer).

**Behaviour**: The scorer returns `ScorerError::Computation("substrate unavailable")`.

**Recovery**: The loop falls back to `RecencyScorer` output if the primary scorer fails.

---

### F3 — Score Overflow / Underflow

**Scenario**: Arithmetic produces a value > 1.0 or < 0.0.

**Behaviour**: The scorer calls `.clamp(0.0, 1.0)` before returning — not an error.

---

## See Also

- [Invariants](./05-invariants.md)
- [Trait Surface](./01-trait-surface.md)
