# Engram — HDC Fingerprint

> The HDC fingerprint is a 10,240-bit semantic vector that enables similarity search alongside exact ContentHash lookup.

**Status**: Shipping  
**Crate**: `roko-core`, `bardo-primitives` (HDC ops)  
**Depends on**: [HdcFingerprint type](../10-types/hdc-fingerprint/00-overview.md)  
**Used by**: Substrate similarity queries, Neuro knowledge cross-cut  
**Last reviewed**: 2026-04-19

---

## TL;DR

Every Engram carries an optional `HdcFingerprint` — a 10,240-bit Binary Spatter Code (BSC)
vector encoding the semantic meaning of the Engram's content. The fingerprint enables
queries like "find me Engrams similar to this one" at ~50 ns per comparison via POPCNT.
The `encoder_version` field ensures fingerprints from different encoder generations are
not cross-compared incorrectly.

---

## The Idea

ContentHash gives exact-match retrieval: "give me Engram with id X." But agents often need
semantic retrieval: "give me Engrams related to Python async patterns." ContentHash cannot
help — only the HDC fingerprint can.

Hyperdimensional computing (HDC) represents concepts as very high-dimensional binary
vectors. In 10,240 dimensions, nearly-random vectors are nearly orthogonal: the expected
cosine similarity between two random vectors is 0 ± ε. When two vectors are close (high
Hamming similarity), the underlying concepts are semantically related.

Key properties that make HDC valuable here:

- **O(1) similarity**: Hamming distance is a single POPCNT instruction on 160 64-bit words.
- **Composable**: vectors for compound concepts are computed by binding (XOR) and bundling
  (majority-vote). The composition is associative and reversible.
- **Fault-tolerant**: a vector that is 10–20% corrupted still represents approximately the
  same concept. This matches how noisy real-world content should behave.
- **Encoder-independent identity**: the ContentHash does not include the fingerprint,
  so encoding algorithm upgrades are transparent to identity.

---

## Specification

### The HdcFingerprint Struct

```rust
<!-- source: crates/roko-core/src/engram.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HdcFingerprint {
    /// The 10,240-bit binary spatter code (BSC) vector.
    /// Stored as 160 u64 words (10,240 / 64 = 160).
    pub vector: HdcVector,

    /// Registry version of the encoder that produced this vector.
    /// Comparisons between fingerprints with different encoder_version
    /// values are semantically undefined and should not be performed.
    pub encoder_version: u32,
}
```

### The HdcVector Type

```rust
<!-- source: crates/bardo-primitives/src/hdc.rs -->

/// A 10,240-bit binary spatter code vector.
/// Stored as [u64; 160].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HdcVector(pub [u64; 160]);

impl HdcVector {
    pub const TOTAL_BITS: usize = 10_240;
    pub const WORDS: usize = 160;

    /// Hamming distance: number of differing bits.
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        self.0.iter().zip(other.0.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum()
    }

    /// Normalized similarity: 1.0 = identical, 0.0 = maximally different.
    pub fn similarity(&self, other: &Self) -> f32 {
        1.0 - (self.hamming_distance(other) as f32 / Self::TOTAL_BITS as f32)
    }

    /// Binding (XOR): combines two vectors into a new concept.
    pub fn bind(&self, other: &Self) -> Self { /* XOR */ }

    /// Bundling (majority vote): merges a set of vectors into a superposition.
    pub fn bundle(vectors: &[&Self]) -> Self { /* majority vote per bit */ }
}
```

---

## Semantics

### When Is the Fingerprint Computed?

`EngramBuilder::build()` computes the fingerprint immediately after computing the
ContentHash:

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

fn compute_fingerprint(kind: &Kind, body: &Body) -> Option<HdcFingerprint> {
    let encoder = HDC_ENCODER_REGISTRY.get_current()?;
    let vector = encoder.encode(kind, body);
    Some(HdcFingerprint {
        vector,
        encoder_version: encoder.version(),
    })
}
```

The encoder is looked up from the global registry. If no encoder is registered (test
environments), `fingerprint` is `None`.

### Encoding Process

The encoder converts an Engram's `kind` and `body` into a 10,240-bit vector:

1. Encode the Kind discriminant as a role vector (deterministic from the Kind tag).
2. Encode the Body content into a content vector (tokenize + embed + project to BSC).
3. Bind the role vector and content vector with XOR.
4. The result is the Engram's fingerprint vector.

For the full encoding algorithm, see
[`../10-types/hdc-fingerprint/04-encoding-scheme.md`](../10-types/hdc-fingerprint/04-encoding-scheme.md).

### Similarity Queries

Substrate implementations support similarity queries:

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

trait Substrate {
    /// Return Engrams whose fingerprint similarity exceeds `threshold`.
    fn find_similar(
        &self,
        query: &HdcFingerprint,
        threshold: f32,
        limit: usize,
    ) -> Vec<Engram>;
}
```

Typical thresholds: 0.85 for "same topic", 0.70 for "related topic".

### Cross-Version Compatibility

Fingerprints generated by different encoder versions are not comparable:

```rust
<!-- source: crates/roko-core/src/hdc.rs -->

fn versions_compatible(a: &HdcFingerprint, b: &HdcFingerprint) -> bool {
    a.encoder_version == b.encoder_version
}
```

When comparing Engrams across encoder versions, the system re-encodes the older Engram
using the current encoder. See
[`../10-types/hdc-fingerprint/03-encoder-version.md`](../10-types/hdc-fingerprint/03-encoder-version.md).

---

## Performance

- **Encoding**: ~10–50 µs per Engram (body tokenization dominates)
- **Hamming distance**: ~50 ns (160 POPCNT operations on 64-bit words)
- **Substrate scan** (1M Engrams): ~50 ms without indexing; ~500 µs with an HDC index structure

For the full performance analysis, see
[`../10-types/hdc-fingerprint/05-performance.md`](../10-types/hdc-fingerprint/05-performance.md).

---

## Invariants

1. If `fingerprint.is_some()`, `encoder_version` is a registered version
2. Fingerprint is deterministic for the same (kind, body, encoder_version) triple
3. `similarity(v, v) == 1.0` for any vector `v`
4. `hamming_distance(v, v) == 0` for any vector `v`

---

## Failure Modes

| Failure | Trigger | Recovery |
|---------|---------|----------|
| `fingerprint` is None in production | HDC encoder not registered | Substrate logs warning; similarity search disabled; hash lookup still works |
| Cross-version comparison | Comparing fingerprints with different `encoder_version` | Detected and rejected; re-encode if cross-version similarity needed |
| Encoding failure | Body contains content the encoder cannot process | Encoder returns `None`; `fingerprint` is set to `None` for that Engram |

---

## See Also

- [`../10-types/hdc-fingerprint/00-overview.md`](../10-types/hdc-fingerprint/00-overview.md)
- [`../10-types/hdc-fingerprint/01-binding-bundling.md`](../10-types/hdc-fingerprint/01-binding-bundling.md)
- [`../10-types/hdc-fingerprint/02-similarity-metric.md`](../10-types/hdc-fingerprint/02-similarity-metric.md)
- [`02-content-hash.md`](02-content-hash.md) — exact identity (vs. semantic similarity)
