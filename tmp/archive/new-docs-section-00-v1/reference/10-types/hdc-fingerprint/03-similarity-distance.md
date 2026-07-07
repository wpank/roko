# HDC Fingerprint — Similarity and Distance

> Hamming distance as a measure of semantic dissimilarity between HdcVectors.

**Status**: Shipping  
**Crate**: `bardo-primitives`  
**Depends on**: [HdcVector Format](01-hdc-vector.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The distance between two fingerprints is their **Hamming distance** — the number of bit
positions where they differ. Hamming distance is computed in ~16 ns via POPCNT over the
XOR of the two vectors. A normalized similarity score is `1 - (hamming / 10_240.0)`.
Near-duplicate detection uses a threshold of `hamming < NEAR_DUPLICATE_THRESHOLD = 512`
(5% bit difference).

---

## Specification

```rust
<!-- source: crates/bardo-primitives/src/hdc.rs -->

impl HdcVector {
    /// Compute the Hamming distance between two vectors.
    /// Returns the count of bit positions where they differ.
    pub fn hamming_distance(&self, other: &HdcVector) -> u32 {
        self.0.iter().zip(other.0.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum()
    }

    /// Normalized similarity: 1.0 = identical, 0.0 = completely dissimilar.
    pub fn similarity(&self, other: &HdcVector) -> f64 {
        1.0 - (self.hamming_distance(other) as f64 / 10_240.0)
    }

    /// True if two vectors are semantically near-duplicate.
    pub fn is_near_duplicate(&self, other: &HdcVector) -> bool {
        self.hamming_distance(other) < NEAR_DUPLICATE_THRESHOLD
    }
}

/// Hamming distance below which two Engrams are considered near-duplicates.
pub const NEAR_DUPLICATE_THRESHOLD: u32 = 512;

/// Similarity above which two Engrams are considered near-duplicates.
/// 1.0 - 512/10_240 = 0.95
pub const NEAR_DUPLICATE_SIMILARITY: f64 = 0.95;
```

---

## Interpretation Guide

| Hamming distance | Similarity | Semantic interpretation |
|---|---|---|
| 0 | 1.000 | Identical tokens (same Engram or exact duplicate) |
| ≤ 128 | ≥ 0.987 | Near-identical phrasing |
| ≤ 512 | ≥ 0.950 | Near-duplicate; same concept, minor variation |
| 512–2048 | 0.80–0.95 | Related topic; not duplicate |
| 2048–4096 | 0.60–0.80 | Loosely related |
| ~5120 | ~0.50 | Unrelated (expected for two random vectors) |
| > 9000 | < 0.12 | Orthogonal/complementary in HDC sense |

---

## Performance

```
Two vectors: 160 × u64 XOR + POPCNT
On Apple M-series (NEON instructions):
  ≈ 16 ns per pair
  ≈ 62 million comparisons per second
```

This enables brute-force nearest-neighbor search over ~10,000 Engrams in ~160 µs.
For larger Substrates, an HDC index (bucketed by popcount bands) reduces this further.

---

## Near-Duplicate Detection Flow

The Substrate uses fingerprints to detect near-duplicates before insertion:

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

pub fn check_near_duplicate(
    &self,
    candidate: &Engram,
) -> Option<ContentHash> {
    let Some(ref fp) = candidate.fingerprint else {
        return None;
    };
    // Scan warm-tier fingerprints for near-duplicate
    self.fingerprint_index
        .nearest_within(
            &fp.vector,
            fp.encoder_version,
            NEAR_DUPLICATE_THRESHOLD,
        )
        .map(|(id, _)| id)
}
```

If a near-duplicate is found, the caller can:
1. Reject the candidate (strict deduplication).
2. Accept but add a lineage link to the near-duplicate.
3. Accept and update the near-duplicate's score.

The policy is configurable per Substrate instance.

---

## Cross-Version Comparison

Vectors from different `encoder_version` values are **not** comparable. Comparing them
produces a meaningless distance.

```rust
<!-- source: crates/bardo-primitives/src/hdc.rs -->

impl HdcFingerprint {
    /// Compute similarity, returning None if encoder versions differ.
    pub fn similarity_checked(&self, other: &HdcFingerprint) -> Option<f64> {
        if self.encoder_version != other.encoder_version {
            return None;
        }
        Some(self.vector.similarity(&other.vector))
    }
}
```

---

## Invariants

1. `hamming_distance(v, v) == 0` for any vector `v`.
2. `hamming_distance(v, other) == hamming_distance(other, v)` (symmetric).
3. `similarity(v, v) == 1.0`.
4. `similarity(v, zero) ≈ 0.5` for a random vector `v` (expected popcount ≈ 5120).
5. Cross-version comparison returns `None` from `similarity_checked()`.
6. `NEAR_DUPLICATE_THRESHOLD = 512` corresponds to `similarity ≥ 0.95`.

---

## Open Questions

- Should `NEAR_DUPLICATE_THRESHOLD` be configurable per Substrate instance? Currently
  hardcoded.
- Should the fingerprint index use LSH (Locality-Sensitive Hashing) for sub-linear
  nearest neighbor search? Not yet implemented for warm-tier indices.

## See Also

- [`01-hdc-vector.md`](01-hdc-vector.md) — the vector type
- [`04-encoder-versioning.md`](04-encoder-versioning.md) — handling version mismatches
- [`06-examples.md`](06-examples.md) — distance computation examples
