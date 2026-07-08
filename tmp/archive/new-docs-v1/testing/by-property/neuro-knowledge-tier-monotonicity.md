# Neuro Knowledge Tier Monotonicity

> A knowledge item's validation tier can only increase over its lifetime. Promotion is irreversible: Transient → Working → Consolidated → Persistent.

**Crate**: `roko-neuro`
**Test type**: Unit test
**Enforcement**: `KnowledgeItem::promote`
**Last reviewed**: 2026-04-19

---

## Statement

For all knowledge items K and all time points t₁ < t₂:
`tier(K, t₁) ≤ tier(K, t₂)` where `Transient < Working < Consolidated < Persistent`

No demotion path exists in the system.

---

## Why It Matters

The tier progression represents increasing confidence in a knowledge claim. Allowing demotion would mean that previously validated knowledge could become unvalidated — undermining the trust hierarchy that the Neuro subsystem uses for retrieval prioritization.

---

## Property Test

```rust
proptest! {
    #[test]
    fn neuro_tier_only_increases(
        initial_tier in arb_tier(),
        promotion_attempts in proptest::collection::vec(arb_tier(), 0..5),
    ) {
        let mut item = KnowledgeItem::new_at_tier(initial_tier);
        let mut current_tier = initial_tier;

        for target_tier in &promotion_attempts {
            let result = item.promote_to(*target_tier);
            let new_tier = item.tier();

            prop_assert!(
                new_tier >= current_tier,
                "Tier must not decrease: was {:?}, now {:?}",
                current_tier, new_tier
            );

            if *target_tier < current_tier {
                prop_assert!(result.is_err(), "Demotion must return an error");
            }

            current_tier = new_tier;
        }
    }
}
```

---

## See also

- [../by-subsystem/subsystem-neuro.md](../by-subsystem/subsystem-neuro.md)
