# Scorer — Trait Surface

> The exact Rust `Scorer` trait signature with every parameter and return type annotated.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Overview](./00-overview.md)
**Last reviewed**: 2026-04-19

---

## The Trait

```rust
// source: crates/roko-core/src/scorer.rs

/// Assigns a [`Score`] to an [`Engram`].
///
/// Scorers are applied in a chain: each scorer receives the accumulated
/// [`Score`] from the previous scorer as `prior`, and returns an updated
/// `Score`. The first scorer in the chain receives `Score::default()`.
///
/// # Object safety
/// This trait is object-safe. Hold implementations as `Box<dyn Scorer>`.
pub trait Scorer: Send + Sync {
    /// Score an `Engram`.
    ///
    /// Implementations MUST return a valid `Score` (all axes in [0.0, 1.0]).
    /// NaN or infinite values are a bug.
    ///
    /// # Parameters
    /// - `engram`: the `Engram` being appraised.
    /// - `prior`: the `Score` accumulated by earlier scorers in the chain.
    ///
    /// # Returns
    /// An updated `Score`. Implementations SHOULD preserve axes they do not
    /// intentionally modify (i.e., return `prior` fields unchanged for axes
    /// outside their responsibility).
    fn score(&self, engram: &Engram, prior: Score) -> Result<Score, ScorerError>;
}
```
<!-- source: crates/roko-core/src/scorer.rs -->

---

## `ScorerError`

```rust
// source: crates/roko-core/src/scorer.rs

#[derive(Debug, thiserror::Error)]
pub enum ScorerError {
    #[error("scoring computation failed: {0}")]
    Computation(String),

    #[error("invalid score value (NaN or out of range): axis={axis}, value={value}")]
    InvalidValue { axis: &'static str, value: f32 },
}
```
<!-- source: crates/roko-core/src/scorer.rs -->

---

## Method Summary

| Method | Signature | Mut? | Returns |
|---|---|---|---|
| `score` | `score(&self, engram: &Engram, prior: Score) -> Result<Score, ScorerError>` | No | Score or error |

---

## See Also

- [Semantics](./02-semantics.md) — what each axis means and how accumulators work
- [Invariants](./05-invariants.md)
- [Failure Modes](./06-failure-modes.md)
