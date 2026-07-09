# Decay — Invariants

> The complete, authoritative list of invariants that all Decay variants must satisfy, and where each is enforced.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Used by**: [Engram Invariants](../../01-engram/12-invariants.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Invariants are properties that must hold **always** — at construction, after every mutation,
and after deserialization from any storage backend. This page collects all decay invariants
in one place for auditing and testing. Each invariant names the variant it applies to and
the layer responsible for enforcement.

---

## Universal Invariants (all variants)

These hold regardless of which `Decay` variant is active.

| # | Invariant | Enforcement layer |
|---|---|---|
| U1 | `weight_at(t)` returns a value in `[0.0, 1.0]` for any `t ≥ created_at` | `weight_at()` has a `.clamp(0.0, 1.0)` call |
| U2 | `weight_at(t)` is non-negative | `max(0.0)` floor in all implementations |
| U3 | `weight_at(t2) ≤ weight_at(t1)` for `t2 > t1` between mutations (monotonically non-increasing) | Enforced by the math of each model |
| U4 | Decay is excluded from the Engram's `ContentHash` | `canonical_encode()` skips the `decay` field |
| U5 | Decay mutation (reinforce, apply_elapsed) never changes the Engram's `ContentHash` | Follows from U4 |
| U6 | The `Decay` variant may not change for the lifetime of an Engram | Enforced by Substrate; a stored `Decay::Exponential` cannot become `Decay::Demurrage` |
| U7 | Serialized `Decay` must deserialize to the same variant | `serde` round-trip test in CI |

---

## Demurrage-Specific Invariants

| # | Invariant | Enforcement layer |
|---|---|---|
| D1 | `balance ∈ [0.0, 1.0]` | `min(1.0)` in `reinforce()`, `max(0.0)` in `apply_idle_tax()` |
| D2 | `idle_tax_per_day ∈ (0.0, 1.0)` | Validated at construction; rejected with `DecayError::InvalidParam` |
| D3 | `reinforcement_per_use ∈ (0.0, 1.0]` | Validated at construction |
| D4 | `reinforce()` is idempotent when `balance = 1.0` | `min(1.0)` enforces this |
| D5 | `apply_idle_tax(0.0)` is a no-op | `(1 - tax).powf(0.0) = 1.0` by definition |

---

## Exponential-Specific Invariants

| # | Invariant | Enforcement layer |
|---|---|---|
| E1 | `half_life_secs > 0` | Validated at construction |
| E2 | `weight_at(t)` approaches 0 asymptotically but never reaches it | Model property; GC uses explicit floor `GC_FLOOR = 0.001` |
| E3 | `weight_at(created_at) = 1.0` | Mathematical identity: `e^0 = 1` |
| E4 | `weight_at(created_at + half_life_secs*1000) ≈ 0.5` | Verified in unit tests |

---

## Step-Specific Invariants

| # | Invariant | Enforcement layer |
|---|---|---|
| S1 | `balance ∈ [0.0, 1.0]` | `max(0.0)` in `apply_epochs()` |
| S2 | `epoch_secs > 0` | Validated at construction |
| S3 | `step_multiplier ∈ (0.0, 1.0)` exclusive | Validated at construction; 1.0 is immortal (rejected), 0.0 is instant-zero (rejected) |
| S4 | `weight_at(t)` is constant within an epoch | Mathematical property of floor division |
| S5 | `apply_epochs(0)` is a no-op | `step_multiplier.powi(0) = 1.0` |

---

## Linear-Specific Invariants

| # | Invariant | Enforcement layer |
|---|---|---|
| L1 | `balance ∈ [0.0, 1.0]` | `max(0.0)` in `apply_elapsed()` |
| L2 | `rate_per_sec > 0.0` | Validated at construction |
| L3 | `weight_at(t)` reaches exactly 0.0 at the expiry time | Mathematical property |
| L4 | `weight_at(t) = 0.0` for all `t ≥ expiry` | `max(0.0)` enforces this |
| L5 | `remaining_secs()` is non-negative | `max(0.0)` in `remaining_secs()` |

---

## Custom-Specific Invariants

| # | Invariant | Enforcement layer |
|---|---|---|
| C1 | `name` is non-empty | Validated at construction |
| C2 | Handler `weight_at()` return value is clamped to `[0.0, 1.0]` | Substrate clamps after dispatch |
| C3 | Unregistered handler returns `1.0` (not panic, not 0.0) | Substrate dispatch path |

---

## Cold-Tier Invariants

| # | Invariant | Enforcement layer |
|---|---|---|
| T1 | Cold-tier Engrams are not subject to idle decay while cold | Compaction skips cold tier |
| T2 | Thawed Engrams have `balance = THAW_RESTORE_BALANCE` (for balance-tracking models) | `thaw()` method |
| T3 | An Engram cannot be in both warm and cold storage simultaneously | Substrate move-then-delete protocol |
| T4 | `frozen_at_ms ≤ now_ms` at all times | Set at freeze, never updated |

---

## Test Coverage

<!-- ADDED: test coverage table — not in source docs; inferred from invariant definitions -->

```rust
<!-- source: crates/roko-core/tests/decay_invariants.rs -->

#[test]
fn demurrage_balance_never_exceeds_one() {
    let mut p = DemurrageParams::default();
    for _ in 0..1000 {
        p.reinforce();
    }
    assert!(p.balance <= 1.0);
}

#[test]
fn exponential_half_life_correct() {
    let p = ExponentialDecayParams { half_life_secs: 3600 };
    let w = p.weight_at(3_600_000, 0); // exactly one half-life
    assert!((w - 0.5).abs() < 1e-9, "w = {}", w);
}

#[test]
fn step_constant_within_epoch() {
    let p = StepDecayParams { balance: 1.0, epoch_secs: 3600, step_multiplier: 0.5 };
    let w0 = p.weight_at(500_000, 0);    // 500 s in — still epoch 0
    let w1 = p.weight_at(3_599_000, 0); // 3599 s in — still epoch 0
    assert_eq!(w0, w1);
}

#[test]
fn linear_reaches_zero() {
    let p = LinearDecayParams { balance: 1.0, rate_per_sec: 1.0 / 3600.0 };
    let w = p.weight_at(3_601_000, 0); // 1 second past expiry
    assert_eq!(w, 0.0);
}

#[test]
fn weight_excluded_from_content_hash() {
    let e1 = make_engram_with_decay(Decay::Demurrage(DemurrageParams {
        balance: 0.5, ..Default::default()
    }));
    let e2 = make_engram_with_decay(Decay::Demurrage(DemurrageParams {
        balance: 0.9, ..Default::default()
    }));
    // Same identity, different balance — hashes must be equal.
    assert_eq!(e1.id, e2.id);
}
```

---

## Failure Modes for Invariant Violations

| Violation | Symptom | Root cause |
|---|---|---|
| `balance > 1.0` | Score inflation, incorrect ranking | Missing `min(1.0)` cap after deserialization from legacy data |
| `weight_at` returns negative | GC errors, sort panics | Missing `max(0.0)` floor; only on buggy custom handlers |
| Decay variant changed for existing Engram | Semantically wrong decay curve | Substrate must reject variant changes on update |
| Hash includes decay | Different Engrams for same knowledge at different balance | `canonical_encode()` must be audited to exclude `decay` field |

---

## Open Questions

- Should invariant U6 (variant immutability) be relaxed to allow a deliberate
  "decay upgrade" operation (e.g., upgrading Exponential to Demurrage for a
  heavily-accessed Engram)? This would require a new Substrate operation with
  a migration audit trail.
- Should invariants be checked at deserialization time with a dedicated `validate()`
  function rather than relying on construction-time checks? Currently no runtime
  validation on load.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants
- [`../../01-engram/12-invariants.md`](../../01-engram/12-invariants.md) — Engram-level invariants
- [`10-api-reference.md`](10-api-reference.md) — API signatures for all decay operations
