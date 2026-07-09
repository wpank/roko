# Provenance Chain Integrity

> Each link in a provenance attestation chain covers the previous link's hash. Tampering with any link is detectable.

**Crate**: `roko-core`
**Test type**: Unit test
**Enforcement**: `Attestation::verify_chain`
**Last reviewed**: 2026-04-19

---

## Statement

For all attestation chains C = [a₁, a₂, ..., aₙ]:
`aₖ.hash_of_previous == hash(aₖ₋₁)` for all k ∈ 2..n

And `a₁.hash_of_previous == None` (root attestation has no predecessor).

---

## Why It Matters

Provenance is used to answer "who asserted this Engram and when". Chain integrity ensures the provenance record cannot be retroactively altered without detection.

---

## Test

```rust
#[test]
fn attestation_chain_detects_tampering() {
    let chain = build_attestation_chain(depth = 5);
    assert!(chain.verify().is_ok(), "Valid chain must verify");

    // Tamper with the middle link
    let mut tampered = chain.clone();
    tampered.links[2].content = "tampered".to_string();

    assert!(tampered.verify().is_err(), "Tampered chain must fail verification");
}
```

---

## See also

- [lineage-acyclicity.md](lineage-acyclicity.md)
- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
