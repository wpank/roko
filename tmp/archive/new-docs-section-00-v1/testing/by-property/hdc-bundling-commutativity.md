# HDC Bundling Commutativity

> Bundling two hypervectors in any order produces the same result: `bundle(a, b) == bundle(b, a)`.

**Crate**: `roko-core`
**Test type**: Property-based (proptest)
**Enforcement**: `HdcVector::bundle`
**Last reviewed**: 2026-04-19

---

## Statement

For all 10,240-bit hypervectors A and B:
`bundle(A, B) == bundle(B, A)`

---

## Why It Matters

HDC bundling is used to aggregate knowledge items: a concept formed from multiple Engrams is represented as the bundle of their individual hypervectors. If bundling were order-dependent, the same set of Engrams assembled in different orders would produce different concept vectors, making knowledge comparison unreliable.

Commutativity ensures:
- Two agents independently assembling the same knowledge set produce the same concept vector.
- Similarity search is consistent: `similarity(bundle(A,B), query) == similarity(bundle(B,A), query)`.

---

## Implementation

`bundle` is implemented as a majority-vote aggregation over bit positions:

```rust
pub fn bundle(a: &HdcVector, b: &HdcVector) -> HdcVector {
    // For 2-vector bundle: majority vote on each bit (with tie-breaking by XOR with noise)
    let mut result = HdcVector::zeros();
    for i in 0..VECTOR_BITS {
        result.set_bit(i, a.bit(i) ^ b.bit(i)); // XOR for 2-vector case
    }
    result
}
```

<!-- source: crates/roko-core/src/hdc.rs -->

XOR is commutative by definition, so the bundle operation inherits commutativity.

---

## Property Test

```rust
proptest! {
    #[test]
    fn hdc_bundle_commutative(
        a in arb_hdc_vector(),
        b in arb_hdc_vector(),
    ) {
        let ab = HdcVector::bundle(&a, &b);
        let ba = HdcVector::bundle(&b, &a);
        prop_assert_eq!(ab, ba, "HDC bundle must be commutative");
    }
}
```

---

## Related Properties

- [hdc-binding-bijectivity.md](hdc-binding-bijectivity.md)

## See also

- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
- [../by-subsystem/subsystem-neuro.md](../by-subsystem/subsystem-neuro.md)
