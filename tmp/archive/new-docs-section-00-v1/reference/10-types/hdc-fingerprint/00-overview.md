# HDC Fingerprint — Overview

> A 10,240-bit binary vector that encodes semantic content for fast approximate similarity search, distinct from the identity hash.

**Status**: Shipping  
**Crate**: `bardo-primitives`  
**Depends on**: [Engram](../../01-engram/00-overview.md)  
**Used by**: [Substrate](../../../subsystems/substrate/), [Gate](../../../subsystems/gate/)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Each Engram has an optional `fingerprint: Option<HdcFingerprint>`. When present, the
fingerprint is a 10,240-bit binary vector (`[u64; 160]`) produced by an HDC encoder from
the Engram's `Body`. Hamming distance between two fingerprints is a fast proxy for
semantic dissimilarity. Fingerprints are used for near-duplicate detection, novelty
filtering, and context assembly. They are **not** part of the ContentHash, so encoder
upgrades are transparent to Engram identity.

---

## The Idea

The ContentHash answers "is this exactly the same Engram?" Fingerprint answers "is this
Engram semantically similar to that one?" They serve orthogonal purposes:

- ContentHash: exact identity, used for deduplication and lineage.
- Fingerprint: semantic proximity, used for retrieval, novelty scoring, and clustering.

HDC (Hyperdimensional Computing) encodes meaning into a high-dimensional binary vector
using a **Binary Sparse Code (BSC)**. The 10,240-bit space gives sufficient resolution for
semantic distinctions while keeping distance computation fast: comparing two fingerprints
is a single POPCNT over 160 u64 values, taking ~16 nanoseconds on modern hardware.

---

## Structure

```rust
<!-- source: crates/bardo-primitives/src/hdc.rs -->

/// A 10,240-bit Binary Sparse Code fingerprint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdcFingerprint {
    /// The binary vector. 160 × 64 = 10,240 bits.
    pub vector: HdcVector,

    /// Which version of the encoder produced this vector.
    /// Vectors from different encoder versions are not directly comparable.
    pub encoder_version: u32,
}

/// Newtype wrapping [u64; 160] — the 10,240-bit BSC.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdcVector(pub [u64; 160]);
```

---

## Why Not an Embedding?

Floating-point embeddings (e.g., `Vec<f32>` at 768 or 1536 dimensions) are the common
choice for semantic similarity in LLM-adjacent systems. Roko uses binary HDC vectors
instead for three reasons:

1. **Speed**: Hamming distance via POPCNT is faster than cosine similarity via
   floating-point dot product for the same bitwidth.
2. **Composability**: HDC vectors can be combined (XOR for binding, majority vote for
   bundling) to build composite fingerprints for multi-body Engrams.
3. **Robustness**: Binary vectors are less sensitive to quantization errors and are
   trivially serialized/deserialized without precision loss.

The tradeoff is lower precision at extreme similarity thresholds, which is acceptable
for Roko's use of fingerprints as a coarse filter before exact scoring.

<!-- ADDED: rationale section — from architectural inference and cross-pollination docs -->

---

## Exclusion from ContentHash

`fingerprint` is excluded from `canonical_encode()` because:
- Fingerprints are produced by an external encoder that can be upgraded.
- Encoder v2 might produce different vectors for the same content than v1.
- If fingerprints were in the hash, every encoder upgrade would require re-creating
  all Engrams, breaking lineage.

The `encoder_version` field handles compatibility: the Substrate will not compare
fingerprints from different encoder versions without explicit re-encoding.

---

## Optional Presence

`fingerprint` is `Option<HdcFingerprint>`:
- `None` = fingerprint not yet computed or not applicable (e.g., `Body::Binary`).
- `Some(f)` = fingerprint computed by encoder version `f.encoder_version`.

The builder can compute the fingerprint eagerly or leave it as `None` for later computation.

---

## Open Questions

- Should 10,240 bits be the fixed size, or configurable per deployment? Currently fixed.
- Should Engrams with `fingerprint = None` participate in similarity search (with a
  lower-priority score) or be excluded? Currently excluded.

## See Also

- [`01-hdc-vector.md`](01-hdc-vector.md) — the `[u64; 160]` format
- [`02-encoding-pipeline.md`](02-encoding-pipeline.md) — how Body → HdcVector
- [`03-similarity-distance.md`](03-similarity-distance.md) — Hamming distance and thresholds
