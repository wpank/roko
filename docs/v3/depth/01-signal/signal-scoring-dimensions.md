# Signal Scoring Dimensions

> Deep dive into the 7-axis Score type: primary axes, extended axes, the
> effective scalar formula, composition operators, and design rationale.

**Source**: `crates/roko-core/src/score.rs`

---

## 1. The Score Struct

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub confidence: f32,   // [0..1]
    pub novelty:    f32,   // [0..1]
    pub utility:    f32,   // [0..inf)
    pub reputation: f32,   // [0..inf)
    pub precision:  f32,   // [0..1]   (serde default = 0)
    pub salience:   f32,   // [0..1]   (serde default = 0)
    pub coherence:  f32,   // [0..1]   (serde default = 0)
}
```

Every axis is clamped to its valid range by the constructors. Non-finite
values (NaN, Infinity) are scrubbed to `0.0`.

---

## 2. The Four Primary Axes

These are the required axes. Every scoring mechanism in the Roko design
corpus (confidence, novelty, utility, reputation, fitness, pfUtility,
catalytic score) collapses into one of these four.

### 2.1 Confidence `[0..1]`

How sure are we that this Signal is correct or valid?

- `0.0` = no evidence of correctness
- `0.5` = neutral (default for unscored Signals)
- `1.0` = verified ground truth

**Critical property**: zero confidence produces zero effective score.
A Signal with `confidence = 0.0` scores zero regardless of how novel,
useful, or reputable it is. This prevents false positives from inflating
rankings.

### 2.2 Novelty `[0..1]`

How new or surprising is this Signal compared to prior Signals?

- `0.0` = completely expected (redundant)
- `1.0` = maximally novel (never seen before)

Acts as a multiplicative bonus: `(1 + novelty)`. A fully novel Signal
doubles its base score. A zero-novelty Signal gets no bonus (multiplier
of `1.0`).

### 2.3 Utility `[0..infinity)`

How useful has this Signal proven historically? Unbounded because utility
accumulates over time -- a Signal referenced by many successful outcomes
has higher utility than one referenced once.

- `0.0` = no historical evidence of usefulness
- `> 0.0` = proportional to observed benefit

Also acts as a multiplicative bonus: `(1 + utility)`.

### 2.4 Reputation `[0..infinity)`

The trustworthiness of the Signal's author at the time of emission.
Unbounded to allow compound reputation from multiple successful
contributions.

- `0.0` = untrusted (zero reputation kills the score)
- `1.0` = neutral trust (default for `Provenance::trusted`)
- `> 1.0` = elevated reputation from track record

Reputation directly scales the result. A Signal from an author with
`reputation = 2.0` scores twice as high as one with `reputation = 1.0`,
all else equal.

---

## 3. The Three Extended Axes

These are optional shaping axes that default to `0.0`. When zero, they
have no effect on the scalar score (backward-compatible).

### 3.1 Precision `[0..1]`

How exact or narrowly applicable is this Signal?

- `0.0` = broadly applicable (or unspecified)
- `1.0` = very specific to a narrow context

**Design decision**: Precision is **excluded** from the effective scalar.
It describes applicability narrowness, not quality. Routers that need
specificity ranking consume precision separately. Including it in the
scalar would penalize broad signals inappropriately.

### 3.2 Salience `[0..1]`

How much extra ranking weight should this Signal receive? A boosting
factor for signals that should stand out during selection.

- `0.0` = no boost (factor = 1.0, no effect)
- `1.0` = maximum boost (factor = 1.0)

When non-zero, the salience factor is `0.5 + 0.5 * salience`. This
provides a soft range from `0.5` to `1.0` -- salience can halve the
score at worst, never zero it out.

### 3.3 Coherence `[0..1]`

How internally consistent is the supporting evidence for this Signal?

- `0.0` = no coherence assessment (factor = 1.0, no effect)
- `1.0` = maximally consistent evidence (factor = 1.0)

Same formula as salience: when non-zero, `0.5 + 0.5 * coherence`. Soft
damping for incoherent evidence, no effect when unassessed.

---

## 4. The Effective Scalar Formula

`Score::effective()` combines six of the seven axes into a single `f32`:

```
effective = confidence
          * (1 + novelty)
          * (1 + utility)
          * reputation
          * salience_factor
          * coherence_factor
```

where:

```
salience_factor  = if salience  == 0.0 { 1.0 } else { 0.5 + 0.5 * salience }
coherence_factor = if coherence == 0.0 { 1.0 } else { 0.5 + 0.5 * coherence }
```

### Properties

1. **Zero confidence kills everything**: `confidence = 0.0` -> `effective = 0.0`
2. **Zero reputation kills everything**: `reputation = 0.0` -> `effective = 0.0`
3. **Novelty and utility are additive bonuses**: `(1 + x)` means zero gives
   no penalty, positive values boost proportionally
4. **Salience and coherence are opt-in soft dampers**: zero means "not
   assessed" (factor 1.0), non-zero provides `[0.5, 1.0]` range
5. **Precision is excluded**: consumed separately by routers
6. **Non-finite scores return zero**: guards against NaN/Infinity propagation
7. **Backward compatible**: when extended axes are zero (the default from
   `Score::new()`), the formula reduces to the original 4-factor spec:
   `confidence * (1 + novelty) * (1 + utility) * reputation`

### Worked Examples

**Minimal trusted signal**:
```
confidence=1.0, novelty=0.0, utility=0.0, reputation=1.0
effective = 1.0 * 1.0 * 1.0 * 1.0 * 1.0 * 1.0 = 1.0
```

**Highly novel, useful, reputable, salient, coherent**:
```
confidence=0.5, novelty=1.0, utility=1.0, reputation=2.0, salience=1.0, coherence=1.0
effective = 0.5 * 2.0 * 2.0 * 2.0 * 1.0 * 1.0 = 4.0
```

**Low salience damping**:
```
confidence=0.8, novelty=0.0, utility=0.0, reputation=1.0, salience=0.2
salience_factor = 0.5 + 0.5 * 0.2 = 0.6
effective = 0.8 * 1.0 * 1.0 * 1.0 * 0.6 * 1.0 = 0.48
```

---

## 5. Constants

```rust
pub const ZERO: Score = Score {
    confidence: 0.0, novelty: 0.0, utility: 0.0, reputation: 0.0,
    precision: 0.0, salience: 0.0, coherence: 0.0,
};

pub const NEUTRAL: Score = Score {
    confidence: 0.5, novelty: 0.0, utility: 0.0, reputation: 1.0,
    precision: 0.0, salience: 0.0, coherence: 0.0,
};
```

`Score::default()` returns `NEUTRAL`. This is the score assigned to
Signals when no scorer has evaluated them.

`Score::from_confidence(c)` creates a score with only confidence set
and neutral reputation (`1.0`).

---

## 6. Composition Operators

### Multiplication (`Score * Score`)

Element-wise scaling of each axis. Useful for combining a base score
with a per-axis modifier. Unit-interval axes are clamped to `[0, 1]`;
unbounded axes (`utility`, `reputation`) are clamped to `[0, inf)`.

```rust
let product = base_score * modifier;
// product.confidence = clamp(base.confidence * modifier.confidence, 0, 1)
// product.utility    = clamp(base.utility * modifier.utility, 0, inf)
```

### Addition (`Score + Score`)

Element-wise aggregation of evidence from multiple scorers. Unit-interval
axes saturate at `1.0`; unbounded axes accumulate.

```rust
let aggregate = score_a + score_b;
// aggregate.confidence = clamp(a.confidence + b.confidence, 0, 1)
// aggregate.utility    = a.utility + b.utility  (unbounded)
```

---

## 7. Threshold Checks

```rust
pub fn exceeds(&self, threshold: f32) -> bool {
    threshold.is_finite() && self.effective() > threshold
}
```

Returns `false` for non-finite thresholds (NaN, Infinity), providing
a defensive default against misconfigured thresholds.

---

## 8. Finite Safety

All constructors scrub non-finite values:
- NaN -> `0.0`
- Infinity -> `0.0` (for non-negative axes) or clamped to `1.0` (for unit axes)

`Score::is_finite()` checks all seven axes. `effective()` returns `0.0`
if any axis is non-finite.

---

## 9. Integration Points

| Consumer | How it uses Score |
|---|---|
| `Store::query()` | Ranks results by `weight_at(now)` = `effective() * decay.apply(age)` |
| `Store::prune()` | Removes Signals where weight < threshold |
| `CascadeRouter` | Routes to models based on effective score of prior outcomes |
| `GraduationCell` | Checks `effective() >= min_score` for Pulse promotion |
| `promote_to_working()` | Compares `effective()` against min_score parameter |
| `QualityJudge` | Produces 5-dimensional assessments mapped to Score axes |
| `BayesianConfidence` | Updates confidence based on evidence accumulation |

---

## 10. Verification Commands

```bash
# Run Score tests
cargo test -p roko-core score -- --nocapture

# Verify 7 axes exist
grep 'pub.*f32' crates/roko-core/src/score.rs | head -7
```

---

## 11. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/score.rs` | Score struct, effective formula, operators, constants |
| `crates/roko-std/src/scorer.rs` | Standard scorer implementations |
| `crates/roko-learn/src/quality_judge.rs` | QualityJudge multi-dimensional scorer |
| `crates/roko-learn/src/bayesian_confidence.rs` | BayesianConfidence evidence updater |
