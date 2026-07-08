# Token Demurrage Rate

> KORAI and DAEJI tokens decay at exactly 1% per year. After 365 days, a balance B becomes B × 0.99.

**Crate**: `roko-chain`
**Test type**: Unit test
**Enforcement**: `TokenBalance::apply_demurrage`
**Last reviewed**: 2026-04-19

---

## Statement

For all token balances B in [0, u128::MAX] and elapsed time T in days:

`apply_demurrage(B, T) = B × (0.99)^(T / 365.25)`

The demurrage is applied using fixed-point arithmetic (no floating point) to prevent rounding errors from accumulating.

---

## Why It Matters

Demurrage discourages token hoarding and encourages circulation — a key economic design property of the Korai economy. An incorrect demurrage rate would silently inflate or deflate the token supply over time.

---

## Property Test

```rust
#[test]
fn demurrage_rate_is_one_percent_per_year() {
    let initial = 1_000_000u128; // 1M tokens
    let one_year_seconds: u64 = 365 * 24 * 60 * 60 + 6 * 60 * 60; // 365.25 days

    let after_one_year = apply_demurrage(initial, one_year_seconds);

    // Expected: 1_000_000 × 0.99 = 990_000
    let expected = 990_000u128;
    let tolerance = 100u128; // allow ±100 for fixed-point rounding

    assert!(
        (after_one_year as i128 - expected as i128).abs() <= tolerance as i128,
        "After 1 year, {} tokens should become ~{} (got {})",
        initial, expected, after_one_year
    );
}
```

---

## See also

- [../by-subsystem/subsystem-chain.md](../by-subsystem/subsystem-chain.md)
