# Bandit Score Monotonicity

> A bandit arm that has received more positive rewards must have a higher estimated value than an arm with fewer positive rewards (all else equal).

**Crate**: `roko-learn`
**Test type**: Unit test
**Enforcement**: `Ucb1::update`, `LinUcb::update`
**Last reviewed**: 2026-04-19

---

## Statement

For UCB1: if arm A has received `n` rewards with mean `r` and arm B has received the same `n` rewards with mean `r' < r`, then `estimate(A) > estimate(B)` (before the UCB exploration bonus is applied).

---

## Why It Matters

The learning loop selects models and routing strategies based on bandit arm scores. Non-monotonic score updates would cause the router to prefer consistently underperforming models.

---

## Property Test

```rust
proptest! {
    #[test]
    fn bandit_higher_reward_higher_estimate(
        n_rounds in 1usize..=50,
        reward_high in 0.5f64..=1.0,
        reward_low in 0.0f64..=0.4,
    ) {
        let mut bandit = Ucb1::new(2);
        // Arm 0 always gets reward_high, arm 1 always gets reward_low
        for _ in 0..n_rounds {
            bandit.update(0, reward_high);
            bandit.update(1, reward_low);
        }

        let est_high = bandit.estimated_value(0);
        let est_low = bandit.estimated_value(1);

        prop_assert!(
            est_high > est_low,
            "Arm with higher rewards ({}) must have higher estimated value than arm with lower rewards ({})",
            reward_high, reward_low
        );
    }
}
```

---

## See also

- [../by-subsystem/subsystem-learn.md](../by-subsystem/subsystem-learn.md)
