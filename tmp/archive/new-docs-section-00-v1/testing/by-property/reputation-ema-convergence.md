# Reputation EMA Convergence

> After sufficient rounds, the EMA reputation score for each domain converges to the true mean of the received rewards.

**Crate**: `roko-chain`
**Test type**: Unit test
**Enforcement**: `ReputationDomain::update`
**Last reviewed**: 2026-04-19

---

## Statement

For all EMA smoothing factors α in (0, 1) and all true means μ in [0, 1]:

After N rounds where each round receives reward r ~ Bernoulli(μ):
`|EMA_after_N - μ| → 0 as N → ∞`

Practically tested at N=1000 with tolerance 0.05.

---

## Property Test

```rust
#[test]
fn reputation_ema_converges_to_true_mean() {
    let mut rep = ReputationDomain::new(alpha = 0.1);
    let true_mean = 0.7;
    let mut rng = StdRng::seed_from_u64(42);

    for _ in 0..1000 {
        let reward = if rng.gen::<f64>() < true_mean { 1.0 } else { 0.0 };
        rep.update(reward);
    }

    assert!(
        (rep.score() - true_mean).abs() < 0.05,
        "EMA score {} must be within 0.05 of true mean {}",
        rep.score(), true_mean
    );
}
```

---

## Related Properties

- [bandit-score-monotonicity.md](bandit-score-monotonicity.md)

## See also

- [../by-subsystem/subsystem-chain.md](../by-subsystem/subsystem-chain.md)
