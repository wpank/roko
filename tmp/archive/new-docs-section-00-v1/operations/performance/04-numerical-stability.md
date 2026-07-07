# Numerical Stability

> How Roko handles floating-point arithmetic in score computation and decay, including
> the conventions that prevent accumulation of rounding error across millions of
> operations.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [00-overview.md](00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko uses `f32` for all Score axis values and decay computations. All score values are
clamped to `[0.0, 1.0]` after every arithmetic operation. Decay uses
mathematically stable formulations that avoid underflow near zero. The specific
conventions below are mandatory — deviation causes silent precision loss that
accumulates over thousands of decay steps.

---

## Why Numerical Stability Matters for Roko

The Score type is a 7-axis appraisal of an Engram: `[relevance, confidence, urgency,
novelty, utility, affect, social]`, each a `f32` in `[0.0, 1.0]`. Scores participate
in:

1. **Routing decisions** — CascadeRouter selects tiers based on urgency and confidence.
2. **Gate threshold comparisons** — adaptive thresholds use EMAs of score values.
3. **Decay** — all seven axes decay over time using configurable decay models.
4. **HDC fingerprint weighting** — score components weight the HDC bundle operation.
5. **Substrate GC triggering** — Engrams with all scores below a floor are GC'd.

With millions of Engrams accumulating over months of operation, even a tiny per-step
numerical error compounds. An Engram whose `relevance` score should be 0.003 but rounds
to 0.000 at step 5,000 is GC'd prematurely. An Engram that should be GC'd but rounds
to 0.001 wastes substrate space indefinitely.

---

## Data Types

| Value | Type | Rationale |
|-------|------|-----------|
| Score axis value | `f32` | 7 axes × 4 bytes = 28 bytes; fits in a cache line with other fields |
| Score axis after arithmetic | `f32`, immediately clamped to `[0.0, 1.0]` | Prevents accumulation out of range |
| Decay rate parameter (λ) | `f64` | Higher precision for the per-Engram parameter stored at creation time |
| Decay computation intermediate | `f64` | Downcast to `f32` only at the final assignment |
| HDC vector component | `u8` / `u64` (packed bits) | Binary; no float arithmetic |
| EMA accumulator | `f64` | Running mean must not lose precision over thousands of updates |

---

## Score Arithmetic Conventions

**Rule 1: Clamp after every operation.**

```rust
// CORRECT
let combined = (a + b).clamp(0.0, 1.0);

// WRONG — can produce values > 1.0 with accumulated adds
let combined = a + b;
```

The `Score::add`, `Score::mul`, and `Score::lerp` methods all clamp internally.
Never use raw `f32` arithmetic on score values outside these methods.

**Rule 2: Use `f64` intermediates for decay.**

```rust
// CORRECT
let decayed = (current_f32 as f64 * (-lambda_f64 * dt).exp()) as f32;

// WRONG — precision loss for small lambda × large dt products
let decayed = current_f32 * (-lambda_f32 * dt_f32).exp();
```

When `lambda × dt` is small (slow decay, short time step), `f32` `exp()` can lose
the low-order bits that carry the difference between "barely decayed" and "not decayed
at all".

**Rule 3: Avoid `f32::powi` for decay exponents > 20.**

For decay functions that raise values to a power:

```rust
// CORRECT for high exponents
let decayed = current_f64.powf(exponent_f64);

// ACCEPTABLE for exponents ≤ 20
let decayed = current_f32.powi(exponent_i32);
```

`f32::powi` loses meaningful precision for large integer exponents. `f64::powf` is safer.

---

## Decay Model Stability

Roko supports four decay models. Each has specific numerical stability requirements:

### Exponential Decay

`score(t) = score₀ × e^(−λt)`

- **λ range**: `[1e-6, 1.0]` per time unit. Values below `1e-6` are clamped to `1e-6`
  to prevent underflow in `λt` products.
- **Time step**: real-wall-clock seconds, cast to `f64` before multiplication.
- **Underflow floor**: when `score(t) < 1e-7`, it is set to `0.0` (flushed to zero).
  This prevents subnormal `f32` values that carry no meaningful information but cost CPU.

### Power-Law Decay

`score(t) = score₀ × (1 + t/τ)^(−α)`

- **τ (characteristic time)**: `f64` at creation, downcast to `f32` only for storage.
- **α (exponent)**: `f64` throughout computation.
- **Stability**: for `α × ln(1 + t/τ) > 70` (approximately), the result is < `1e-30`
  and is flushed to `0.0`.

### Step Decay

`score(t) = score₀ × r^floor(t / drop_every)`

- **r (retention)**: `f32` in `[0.0, 1.0]`. Validated at config load time.
- **Exponent**: the `floor(t / drop_every)` is computed as `i64` to avoid `f32` integer
  roundoff for large step counts.

### Asymptotic Decay (Sigmoid to Floor)

`score(t) = floor + (score₀ - floor) × σ(-kt)`

- All computation in `f64`. Downcast to `f32` only at final assignment.
- `floor` must be in `[0.0, 1.0)`; validated at config load.

---

## EMA Accumulator

The adaptive gate threshold EMA uses the update rule:

```
ema_new = α × observation + (1 - α) × ema_old
```

With `α = 0.1` and `f64` accumulators, precision is maintained across > 10^15 updates
before the accumulator degrades. The EMA accumulators are stored as `f64` in the gate
state and downcast to `f32` only when compared against gate thresholds.

---

## Invariants (Enforced by Type)

The `Score` type enforces these invariants at construction and mutation:

1. Every axis value `v` satisfies `0.0 ≤ v ≤ 1.0` after clamp.
2. `NaN` is forbidden. Construction from `NaN` panics in debug builds, clamps to `0.0`
   in release builds.
3. `Inf` is forbidden. Same policy as `NaN`.
4. The zero score `Score::ZERO` has all seven axes at exactly `0.0`.
5. The unit score `Score::ONE` has all seven axes at exactly `1.0`.

The `HdcVector` type has no floating-point state; it is a fixed-width bit array. No
numerical stability concerns apply.

---

## Testing Numerical Properties

The benchmark suite includes stability tests that run decay chains for N steps and
assert the final value is within a tolerance of the analytically expected value:

```bash
cargo test -p roko-core --test numerical_stability -- --nocapture
```

Tests check:
- Exponential decay of 1.0 for 1,000 steps with λ=0.01 produces a value within 0.1%
  of `exp(-10)`.
- Step decay for 10,000 steps with r=0.99 produces a value within 0.1% of `0.99^10000`.
- EMA of 1,000 alternating 0.0/1.0 values converges to ~0.5 within 0.01.
- No subnormal values appear in the decay output (flushed to 0.0 correctly).

---

## See Also

- [reference/10-types/score.md](../../reference/10-types/score.md) — Score type specification
- [reference/10-types/decay.md](../../reference/10-types/decay.md) — decay model specification
- [05-hot-paths.md](05-hot-paths.md) — why these computations must be fast

## Open Questions

- Whether to offer an `f64` Score variant for workloads where precision is more important than cache efficiency is an open question.
- Vectorised (SIMD) decay for batch Engram GC is planned but not yet implemented.
