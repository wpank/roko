# Scorer Examples

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

## Example 1: Using `DefaultScorer`

```rust
// source: crates/roko-core/src/scorer.rs
use roko_core::{Scorer, DefaultScorer, Engram, Score};

let scorer = DefaultScorer::default();
let engram = /* ... */;
let score = scorer.score(&engram, Score::default())?;
println!("confidence={:.2}", score.confidence);
```
<!-- source: crates/roko-core/src/scorer.rs -->

---

## Example 2: A Custom Single-Axis Scorer

```rust
// source: crates/roko-core/src/scorer.rs
use roko_core::{Scorer, ScorerError, Engram, Score};

/// Boosts utility for Engrams tagged with a specific Kind.
pub struct KindUtilityBooster {
    pub target_kind: Kind,
    pub boost: f32,
}

impl Scorer for KindUtilityBooster {
    fn score(&self, engram: &Engram, prior: Score) -> Result<Score, ScorerError> {
        let utility = if engram.kind == self.target_kind {
            (prior.utility + self.boost).clamp(0.0, 1.0)
        } else {
            prior.utility
        };
        Ok(Score { utility, ..prior })
    }
}
```
<!-- source: crates/roko-core/src/scorer.rs -->

---

## Example 3: Stacked Scorers in the Loop

```rust
// source: crates/roko-runtime/src/loop.rs
let scorers: Vec<Box<dyn Scorer>> = vec![
    Box::new(DefaultScorer::default()),
    Box::new(KindUtilityBooster { target_kind: Kind::Fact, boost: 0.2 }),
];

let final_score = scorers.iter()
    .try_fold(Score::default(), |prior, s| s.score(&engram, prior))?;
```
<!-- source: crates/roko-runtime/src/loop.rs -->

---

## See Also

- [Composition Patterns](./09-composition-patterns.md)
- [Trait Surface](./01-trait-surface.md)
