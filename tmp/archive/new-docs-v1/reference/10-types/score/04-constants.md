# Score — Constants

> All named scoring constants with values and rationale.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Arithmetic](03-arithmetic.md)  
**Last reviewed**: 2026-04-19

---

## Weight Constants

```rust
<!-- source: crates/roko-core/src/score.rs -->

/// Default weight for the confidence axis.
/// Correctness is the primary quality signal.
pub const W_CONFIDENCE: f64 = 0.35;

/// Default weight for the novelty axis.
/// New information is valuable but secondary to correctness.
pub const W_NOVELTY: f64 = 0.20;

/// Default weight for the utility axis.
/// Proven usefulness is the second most important signal.
pub const W_UTILITY: f64 = 0.30;

/// Default weight for the reputation axis.
/// Source trust matters but is bounded by direct evidence.
pub const W_REPUTATION: f64 = 0.15;
```

Rationale for each:

| Constant | Value | Rationale |
|----------|-------|-----------|
| `W_CONFIDENCE` | 0.35 | Correctness is the primary gate concern; wrong information is actively harmful |
| `W_UTILITY` | 0.30 | Proven usefulness is the best predictor of future value; evidence-based |
| `W_NOVELTY` | 0.20 | New information is valuable but must not outrank correctness or utility |
| `W_REPUTATION` | 0.15 | Trust matters but is a prior, not direct evidence; lowest stable weight |

---

## Trust-to-Reputation Mapping

```rust
<!-- source: crates/roko-core/src/scorer/reputation.rs -->

pub const REPUTATION_LOCAL_AGENT: f64   = 0.25;
pub const REPUTATION_SELF_VERIFIED: f64 = 0.50;
pub const REPUTATION_PEER_VERIFIED: f64 = 0.75;
pub const REPUTATION_CHAIN_WITNESS: f64 = 1.00;
pub const REPUTATION_TAINTED: f64       = 0.00;
```

---

## Utility Update Deltas

```rust
<!-- source: crates/roko-core/src/scorer/utility.rs -->

/// Utility boost when an Engram contributes to a passed gate verdict.
pub const UTILITY_PASS_DELTA: f64 = 0.05;

/// Utility penalty when an Engram was retrieved but led to a failed gate verdict.
pub const UTILITY_FAIL_DELTA: f64 = 0.03;

/// Minimum achievable utility (floor).
pub const UTILITY_FLOOR: f64 = 0.0;

/// Maximum achievable utility (ceiling).
pub const UTILITY_CEILING: f64 = 1.0;
```

---

## Gate Threshold Defaults

```rust
<!-- source: crates/roko-core/src/gate.rs -->

/// Default minimum effective score to pass a standard quality gate.
pub const GATE_DEFAULT_THRESHOLD: f64 = 0.65;

/// Minimum effective score for a "high-confidence" gate.
pub const GATE_HIGH_CONFIDENCE_THRESHOLD: f64 = 0.80;

/// Minimum effective score for insertion into the long-term Neuro substrate.
pub const NEURO_ADMISSION_THRESHOLD: f64 = 0.70;
```

---

## Open Questions

- Should weight constants be runtime-configurable (via `configuration.toml`) or compile-time only?
- Are the current weight values empirically validated, or are they reasonable priors?

<!-- ADDED: open questions about empirical validation — this question is not addressed in source docs -->

---

## See Also

- [`03-arithmetic.md`](03-arithmetic.md) — how constants are used in the formula
- [`08-rationale.md`](08-rationale.md) — design rationale for the weight choices
