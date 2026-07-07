# Scorer API Reference

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

## Trait

```rust
// source: crates/roko-core/src/scorer.rs
pub trait Scorer: Send + Sync {
    fn score(&self, engram: &Engram, prior: Score) -> Result<Score, ScorerError>;
}
```
<!-- source: crates/roko-core/src/scorer.rs -->

## `ScorerError` Variants

| Variant | Meaning |
|---|---|
| `Computation(String)` | Internal computation failed |
| `InvalidValue { axis, value }` | NaN or out-of-range axis value |

## Shipping Implementations

| Type | Crate | Notes |
|---|---|---|
| `DefaultScorer` | `roko-core` | All-axes scorer |
| `ConstantScorer(Score)` | `roko-core` | Returns fixed score |
| `RecencyScorer` | `roko-core` | Sets `novelty` by recency |
