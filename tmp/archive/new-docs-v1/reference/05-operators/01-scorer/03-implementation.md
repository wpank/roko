# Scorer Implementations

> The shipping `Scorer` implementations in the current codebase.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## Shipping Implementations

### `DefaultScorer`

The standard, all-axes scorer. Computes all four stable axes using heuristics derived from
the `Engram`'s fields, `Provenance`, and substrate context:

- `confidence` — from `Provenance::attestation_level` and `Score` history.
- `novelty` — from HDC fingerprint distance to nearest substrate records.
- `utility` — from task-context similarity (cosine of fingerprints).
- `reputation` — from `Provenance::source_id` track record.

```rust
// source: crates/roko-core/src/scorer.rs
pub struct DefaultScorer {
    pub confidence_weight: f32,
    pub novelty_weight: f32,
    pub utility_weight: f32,
    pub reputation_weight: f32,
}

impl Default for DefaultScorer {
    fn default() -> Self {
        Self {
            confidence_weight: 0.4,
            novelty_weight: 0.2,
            utility_weight: 0.3,
            reputation_weight: 0.1,
        }
    }
}
```
<!-- source: crates/roko-core/src/scorer.rs -->

### `ConstantScorer`

Returns a fixed `Score` regardless of input. Useful as a baseline in tests or for
stub operators in partial pipelines:

```rust
// source: crates/roko-core/src/scorer.rs
pub struct ConstantScorer(pub Score);

impl Scorer for ConstantScorer {
    fn score(&self, _engram: &Engram, _prior: Score) -> Result<Score, ScorerError> {
        Ok(self.0.clone())
    }
}
```
<!-- source: crates/roko-core/src/scorer.rs -->

### `RecencyScorer`

Sets `novelty` based on the `Engram`'s `created_at` timestamp — more recent engrams score
higher novelty. Does not modify other axes.

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Composition Patterns](./09-composition-patterns.md)
