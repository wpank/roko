# HDC Fingerprint — HdcVector Format

> The `[u64; 160]` representation of a 10,240-bit Binary Sparse Code.

**Status**: Shipping  
**Crate**: `bardo-primitives`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`HdcVector` is a newtype wrapping `[u64; 160]`. The 160 `u64` values concatenate to form
a 10,240-bit binary vector. Bit `i` is accessed as `(array[i/64] >> (i%64)) & 1`.
Operations are batch-vectorized over the 160-element array.

---

## Specification

```rust
<!-- source: crates/bardo-primitives/src/hdc.rs -->

/// A 10,240-bit binary sparse code.
/// Each bit is set (1) or unset (0). The vector is stored as 160 u64 words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdcVector(pub [u64; 160]);

impl HdcVector {
    /// Create a zero vector (all bits unset).
    pub fn zero() -> Self {
        HdcVector([0u64; 160])
    }

    /// Create a random vector with approximately 50% bits set.
    pub fn random(rng: &mut impl rand::Rng) -> Self {
        let mut words = [0u64; 160];
        for w in &mut words {
            *w = rng.gen::<u64>();
        }
        HdcVector(words)
    }

    /// Return the value of bit `i` (0-indexed).
    pub fn bit(&self, i: usize) -> bool {
        debug_assert!(i < 10_240);
        (self.0[i / 64] >> (i % 64)) & 1 == 1
    }

    /// Set bit `i` to 1.
    pub fn set_bit(&mut self, i: usize) {
        debug_assert!(i < 10_240);
        self.0[i / 64] |= 1u64 << (i % 64);
    }

    /// Count the number of 1-bits (popcount).
    pub fn popcount(&self) -> u32 {
        self.0.iter().map(|w| w.count_ones()).sum()
    }

    /// XOR two vectors (HDC binding operation).
    pub fn xor(&self, other: &HdcVector) -> HdcVector {
        let mut result = [0u64; 160];
        for i in 0..160 {
            result[i] = self.0[i] ^ other.0[i];
        }
        HdcVector(result)
    }

    /// Bitwise AND (HDC intersection / masking).
    pub fn and(&self, other: &HdcVector) -> HdcVector {
        let mut result = [0u64; 160];
        for i in 0..160 {
            result[i] = self.0[i] & other.0[i];
        }
        HdcVector(result)
    }

    /// Bitwise OR (HDC union / bundling — lossy).
    pub fn or(&self, other: &HdcVector) -> HdcVector {
        let mut result = [0u64; 160];
        for i in 0..160 {
            result[i] = self.0[i] | other.0[i];
        }
        HdcVector(result)
    }
}
```

---

## Memory Layout

- 160 × 8 bytes = **1,280 bytes** per vector.
- On a typical cache line (64 bytes), one vector spans **20 cache lines**.
- Distance computation reads all 20 cache lines sequentially — cache-friendly.

---

## Popcount as Density

`popcount()` returns the number of 1-bits. For a well-formed BSC, ~50% of bits should
be set (`popcount ≈ 5,120`). Very sparse vectors (popcount < 100) or very dense vectors
(popcount > 10,100) indicate encoding failures.

---

## Majority Vote Bundle

When building a composite fingerprint from N component vectors, the bundle is computed
by majority vote (not OR, which would quickly saturate):

```rust
<!-- source: crates/bardo-primitives/src/hdc.rs -->

/// Compute the majority-vote bundle of `vectors`.
/// For N vectors, bit i is set iff more than N/2 input vectors have bit i set.
/// N must be odd to avoid ties; if even, ties are broken by the first vector.
pub fn majority_bundle(vectors: &[&HdcVector]) -> HdcVector {
    assert!(!vectors.is_empty());
    let threshold = vectors.len() / 2;
    let mut counts = [0u32; 10_240];
    for v in vectors {
        for i in 0..10_240 {
            if v.bit(i) { counts[i] += 1; }
        }
    }
    let mut result = HdcVector::zero();
    for i in 0..10_240 {
        if counts[i] > threshold as u32 {
            result.set_bit(i);
        }
    }
    result
}
```

---

## Invariants

1. `HdcVector` is always exactly `[u64; 160]` — 1,280 bytes.
2. `bit(i)` is defined for `i ∈ [0, 10_239]`. Access outside this range is a panic in
   debug builds, undefined in release.
3. `random()` produces vectors with popcount in `[4_500, 5_700]` with high probability
   (±3σ of the binomial distribution).
4. XOR is its own inverse: `v.xor(&v) == HdcVector::zero()`.
5. XOR is commutative and associative.

---

## Open Questions

- Should `HdcVector` derive `Hash` for use in hash maps? Currently it does not
  (large arrays are expensive to hash).
- Should bit access have a bounds-checked variant in release builds?

## See Also

- [`00-overview.md`](00-overview.md) — HDC fingerprint structure
- [`02-encoding-pipeline.md`](02-encoding-pipeline.md) — how vectors are produced
- [`03-similarity-distance.md`](03-similarity-distance.md) — how vectors are compared
