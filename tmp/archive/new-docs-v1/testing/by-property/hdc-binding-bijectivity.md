# HDC Binding Bijectivity

> Binding two hypervectors produces a result that is dissimilar to both inputs. Unbinding recovers the original.

**Crate**: `roko-core`
**Test type**: Unit test
**Enforcement**: `HdcVector::bind`, `HdcVector::unbind`
**Last reviewed**: 2026-04-19

---

## Statement

For all hypervectors A, B:
1. `similarity(bind(A, B), A) ≈ 0.5` (bound result is dissimilar to A)
2. `similarity(bind(A, B), B) ≈ 0.5` (bound result is dissimilar to B)
3. `unbind(bind(A, B), A) ≈ B` (unbinding A from the pair recovers B)
4. `unbind(bind(A, B), B) ≈ A` (unbinding B recovers A)

---

## Why It Matters

HDC binding is used to encode key-value associations (e.g., "concept X has property Y"). Bijectivity ensures the encoding is invertible: you can always decode the stored association.

---

## Test

```rust
#[test]
fn hdc_binding_invertible() {
    let mut rng = StdRng::seed_from_u64(42);
    let a = HdcVector::random(&mut rng);
    let b = HdcVector::random(&mut rng);

    let bound = HdcVector::bind(&a, &b);

    // Bound result must be dissimilar to both inputs
    assert!(cosine_similarity(&bound, &a) < 0.6, "Bound must not be similar to A");
    assert!(cosine_similarity(&bound, &b) < 0.6, "Bound must not be similar to B");

    // Unbinding must recover the original
    let recovered_b = HdcVector::unbind(&bound, &a);
    assert!(cosine_similarity(&recovered_b, &b) > 0.9,
        "Unbinding A from bound must recover B");
}
```

---

## See also

- [hdc-bundling-commutativity.md](hdc-bundling-commutativity.md)
- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
- [../by-subsystem/subsystem-neuro.md](../by-subsystem/subsystem-neuro.md)
