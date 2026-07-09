# Soulbound Non-Transferability

> A soulbound identity passport cannot be transferred to a different address. The token is bound to its mint address permanently.

**Crate**: `roko-chain`
**Test type**: Unit test
**Enforcement**: `SoulboundPassport::transfer` — always returns `Err(Soulbound)`
**Last reviewed**: 2026-04-19

---

## Statement

For all passport tokens P and all destination addresses D ≠ P.owner:

`P.transfer(D)` returns `Err(SoulboundError::NonTransferable)`

The passport can never be in a state where `P.owner ≠ P.mint_address`.

---

## Why It Matters

The soulbound design (Buterin 2022) prevents identity farming: buying, renting, or stealing reputation accumulated by another agent. Every capability and reputation domain is permanently linked to the minting agent.

---

## Property Test

```rust
proptest! {
    #[test]
    fn soulbound_transfer_always_fails(
        mint_address in arb_address(),
        dest_address in arb_address(),
    ) {
        prop_assume!(mint_address != dest_address);

        let passport = SoulboundPassport::mint(mint_address);
        let result = passport.transfer(dest_address);

        prop_assert!(
            matches!(result, Err(SoulboundError::NonTransferable)),
            "Transfer must always fail for soulbound passport"
        );
        prop_assert_eq!(passport.owner(), mint_address,
            "Owner must remain the mint address");
    }
}
```

---

## See also

- [../by-subsystem/subsystem-chain.md](../by-subsystem/subsystem-chain.md)
