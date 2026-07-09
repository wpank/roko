# Score — Examples

> Worked examples: scoring Engrams through each Scorer layer.

**Status**: Shipping  
**Crate**: `roko-core`  
**Last reviewed**: 2026-04-19

---

## Example 1: Default Score

```rust
<!-- source: crates/roko-core/examples/score_examples.rs -->

let s = Score::default();
assert_eq!(s.confidence, 0.5);
assert_eq!(s.novelty, 0.5);
assert_eq!(s.utility, 0.5);
assert_eq!(s.reputation, 0.5);
assert_eq!(s.effective(), 0.5);  // 0.35×0.5 + 0.20×0.5 + 0.30×0.5 + 0.15×0.5
```

---

## Example 2: High-Quality Knowledge Entry

An entry from a chain-witnessed source that has proven useful:

```rust
let s = Score {
    confidence: 0.95,
    novelty: 0.40,     // known domain, not particularly novel
    utility: 0.90,     // retrieved frequently and led to success
    reputation: 1.00,  // chain-witnessed
    precision: Some(0.90),
    salience: None,
    coherence: None,
};
// effective = 0.35×0.95 + 0.20×0.40 + 0.30×0.90 + 0.15×1.00
//           = 0.3325 + 0.08 + 0.27 + 0.15 = 0.8325
assert!(s.effective() > 0.80);
assert!(s.passes(0.80));
```

---

## Example 3: Unverified LLM Output

Raw agent output before gate evaluation:

```rust
let s = Score {
    confidence: 0.55,  // LLM is somewhat confident
    novelty: 0.70,     // new information
    utility: 0.50,     // no track record yet
    reputation: 0.25,  // local agent only
    precision: None,
    salience: None,
    coherence: None,
};
// effective = 0.35×0.55 + 0.20×0.70 + 0.30×0.50 + 0.15×0.25
//           = 0.1925 + 0.14 + 0.15 + 0.0375 = 0.52
assert!(s.effective() < 0.65);  // fails default gate threshold
assert!(!s.passes(0.65));
```

---

## Example 4: HDC Active Inference Scorer

Scoring based on Hamming distance to belief vector:

```rust
<!-- source: crates/roko-core/src/scorer/hdc_belief.rs -->

let hamming = belief_vector.hamming_distance(&engram_vector);
let similarity = 1.0 - (hamming as f64 / HdcVector::TOTAL_BITS as f64);

let score = Score {
    confidence: similarity,        // belief alignment = confidence
    novelty: 1.0 - similarity,     // dissimilarity = novelty
    utility: engram.score.utility, // preserve existing utility
    reputation: engram.score.reputation,
    ..Score::default()
};
```

---

## Example 5: Outcome Update After Gate Pass

When a downstream gate passes, update the utility of contributing Engrams:

```rust
<!-- source: crates/roko-core/src/scorer/utility.rs -->

// For each Engram that was in the context window of the passing gate:
for id in &context_assembly.included_ids {
    if let Some(engram) = substrate.get(id) {
        let new_utility = (engram.score.utility + UTILITY_PASS_DELTA).min(UTILITY_CEILING);
        substrate.update_score(id, engram.score.with_utility(new_utility))?;
    }
}
```

---

## Example 6: Custom Weights for Trust-Sensitive Gate

```rust
let weights = ScoreWeights {
    confidence: 0.20,
    novelty: 0.10,
    utility: 0.20,
    reputation: 0.50,
    ..Default::default()
};
let score = Score {
    confidence: 0.80,
    novelty: 0.50,
    utility: 0.70,
    reputation: 0.25,  // LocalAgent — low reputation
    ..Score::default()
};
// With default weights: 0.35×0.80 + 0.20×0.50 + 0.30×0.70 + 0.15×0.25 = 0.6475 → pass 0.65? barely
// With trust-sensitive weights: 0.20×0.80 + 0.10×0.50 + 0.20×0.70 + 0.50×0.25 = 0.16+0.05+0.14+0.125 = 0.475 → fail 0.65
assert!(!score.passes(0.65));
assert!(!score.effective_weighted(&weights) >= 0.65);
```

---

## See Also

- [`01-axes-stable.md`](01-axes-stable.md) — axis semantics
- [`03-arithmetic.md`](03-arithmetic.md) — effective score formula
- [`04-constants.md`](04-constants.md) — weight constants
