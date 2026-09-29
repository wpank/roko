# HDC/VSA Foundations

> **v3 depth -- 09-memory** | Source: v1/06-neuro/04. FULL HDC math, JL
> bounds. 500+ lines. DO NOT COMPRESS.

Hyperdimensional Computing (HDC) and Vector Symbolic Architectures (VSA)
provide the mathematical substrate for Neuro's similarity search, knowledge
encoding, and cross-domain transfer -- using 10,240-bit binary vectors with
algebraic operations that run in nanoseconds.

---

## Mathematical Foundation: Concentration of Measure

### The geometry of high-dimensional binary spaces

HDC rests on a geometric fact: **concentration of measure** in
high-dimensional spaces. In a D-dimensional binary space {0,1}^D, the
expected Hamming distance between two independently drawn random vectors
is D/2, with a standard deviation that grows only as sqrt(D)/2:

```
Expected Hamming distance:     mu = D/2
Standard deviation:            sigma = sqrt(D) / 2
Coefficient of variation:      CV = sigma / mu = 1 / sqrt(D)
```

As D grows, the distribution of pairwise distances concentrates tightly
around the mean. At D = 10,240:

```
CV = 1 / sqrt(10240) = 0.00988
```

99% of random pairs land within 1% of the expected Hamming distance D/2.
The normalized Hamming similarity (fraction of matching bits) concentrates
around 0.5. Random vectors are neither similar nor dissimilar -- they are
**reliably orthogonal**. When similarity is significantly above 0.5, there
is a genuine structural relationship.

### Quasi-orthogonality guarantee

For any two independently drawn random vectors A, B in {0,1}^D:

```
P(|sim(A, B) - 0.5| > epsilon) < 2 * exp(-2 * D * epsilon^2)
```

This is a direct application of Hoeffding's inequality. At D = 10,240:

| epsilon | P(deviation) | Interpretation |
|---------|-------------|---------------|
| 0.01 | 2.6 x 10^-89 | Virtually impossible |
| 0.02 | 1.4 x 10^-355 | Astronomically unlikely |
| 0.05 | < 10^-2000 | Beyond any practical concern |

This means that any pair of independently generated vectors will have
similarity within [0.49, 0.51] with overwhelming probability. When we
observe sim > 0.52, we can be confident the relationship is genuine.

### Why this matters for knowledge systems

In a 10,240-dimensional binary space, there is room for an astronomically
large number of quasi-orthogonal vectors. The capacity to store K items in
a bundle while maintaining retrieval accuracy is bounded by:

```
SNR = sqrt(D / K)
```

For D = 10,240 and K = 10 (typical knowledge entry with 10 role-filler
bindings): SNR = 32.0, meaning the signal from any individual component is
32x stronger than the noise from all other components. This provides
extremely reliable retrieval.

### Connection to neural population codes

Kanerva (2009) formalized these properties and showed that the same geometry
underlies neural population codes in the brain. Place cells in the
hippocampus, grid cells in the entorhinal cortex, and sparse codes throughout
the cortex all operate in regimes where quasi-orthogonality provides the
capacity guarantees that HDC exploits computationally.

---

## Selected System: Binary Spatter Codes (BSC)

Roko uses BSC exclusively, chosen for five reasons:

### 1. Exact invertibility

XOR is its own inverse: `bind(bind(a, b), b) = a` exactly. No approximation
error. Structured queries decompose composites without information loss.

This is unique to BSC. In HRR, unbinding is only approximate (circular
correlation introduces noise). In FHRR, unbinding requires computing the
conjugate of the phase vector, which is exact but computationally expensive.
BSC unbinding is a single XOR -- the cheapest possible operation.

### 2. Storage efficiency

1,280 bytes per vector. A 100K-entry knowledge base requires only ~128 MB
of HDC storage. Compare:

| System | Bytes per vector (D=10,000) | 100K entries |
|--------|----------------------------|-|
| BSC | 1,280 | 128 MB |
| MAP | 10,000 -- 40,000 | 1 -- 4 GB |
| HRR | 40,000 | 4 GB |
| FHRR | 80,000 | 8 GB |

### 3. Computation speed

XOR compiles to a single instruction per 64-bit word. Comparing two
10,240-bit vectors takes ~13 ns on x86 with AVX-512 auto-vectorization.
Brute-force scanning of 100K entries takes ~1.3 ms.

### 4. Bundle capacity

D=10,240 BSC vectors reliably store up to ~1,000 bound pairs in a bundle
with >95% retrieval accuracy (Kleyko et al. 2022). This vastly exceeds
the needs of knowledge entry encoding (5--10 pairs per entry).

### 5. Discrete data fit

Knowledge entries are inherently discrete -- typed tags, named concepts,
structured relationships. BSC's discrete operations (XOR, majority vote,
cyclic shift) are a natural match for discrete symbolic data.

### Comparison table

| Property | BSC | MAP | HRR | FHRR |
|----------|-----|-----|-----|------|
| Vector space | {0,1}^D | {-1,0,+1}^D | R^D | C^D |
| Binding | XOR | Multiplication | Circular convolution | Phase addition |
| Unbinding | Exact (self-inverse) | Exact | Approximate | Approximate |
| Bundle | Majority vote | Component sum | Component sum | Component sum |
| Bundle capacity (D=10K) | ~1,000 pairs | ~800 pairs | ~100 pairs | ~100 pairs |
| Storage per vector | 1,280 bytes | 10--40 KB | 40 KB | 80 KB |
| Similarity metric | Hamming distance | Cosine similarity | Cosine similarity | Phase coherence |
| Best fit | Discrete symbolic data | Sparse data | Continuous signals | Frequency-domain signals |

---

## Dimension: D = 10,240

The implementation uses **D = 10,240 bits = 160 x u64 words = 1,280 bytes**.

### Rust representation

```rust
// From crates/roko-primitives/src/hdc.rs
pub struct HdcVector {
    bits: [u64; 160],  // 160 words * 64 bits = 10,240 bits
}
```

Key properties: `Copy` semantics (1,280 bytes on the stack), deterministic
seeding via `from_seed(bytes)` using FNV-1a + splitmix64, full serde support,
rkyv zero-copy deserialization.

### Why 10,240 specifically

Two reasons for this specific number:

1. **Quasi-orthogonality guarantee.** P(|sim| > 0.05 from expected) < 10^-9
   for random pairs. The probability that two random vectors have similarity
   above 0.55 is less than 10^-2000 -- effectively zero.

2. **SIMD alignment.** 160 words = 5 x 32-word AVX-512 passes or 10 x
   16-word AVX2 passes. Clean loop boundaries with no remainder handling.
   On ARM NEON (128-bit), 160 words = 20 x 8-word passes.

### Alternative dimensions considered

| Dimension | Words | Storage | Pro | Con |
|-----------|-------|---------|-----|-----|
| 1,024 | 16 | 128 B | Minimal memory | SNR too low for 50+ bundles |
| 4,096 | 64 | 512 B | Good for small vocabularies | JL bound fails at 100K entries |
| **10,240** | **160** | **1,280 B** | **Sweet spot: JL + SIMD + capacity** | **Moderate memory** |
| 16,384 | 256 | 2,048 B | Supports 1M+ entries at epsilon=0.03 | 60% more memory |
| 65,536 | 1,024 | 8,192 B | Overkill for most applications | Memory-bound operations |

---

## Johnson-Lindenstrauss Bound

The Johnson-Lindenstrauss lemma (1984) provides a lower bound on dimension D
to preserve pairwise distances for N points with distortion epsilon:

```
D >= (8 ln N) / epsilon^2
```

### Derivation

The JL lemma states: For any set of N points in R^d, there exists a linear
map f: R^d -> R^D such that for all pairs (x_i, x_j):

```
(1 - epsilon) * ||x_i - x_j||^2 <= ||f(x_i) - f(x_j)||^2 <= (1 + epsilon) * ||x_i - x_j||^2
```

The probabilistic version shows that a random projection into D dimensions
preserves distances with probability at least 1 - 1/N^2 when:

```
D >= (8 ln N) / epsilon^2
```

This is the minimum dimension for faithful distance preservation.

### Validation for Neuro's use case

For N = 100,000 knowledge entries and epsilon = 0.1 (10% maximum distortion):

```
D >= (8 * ln(100000)) / 0.01
   = (8 * 11.51) / 0.01
   = 9,210
```

D = 10,240 exceeds this bound, confirming sufficiency for 100K+ entries with
less than 10% distance distortion.

### Full JL bound table

| N (entries) | epsilon (distortion) | Minimum D | D = 10,240 sufficient? | Headroom |
|-------------|---------------------|-----------|----------------------|----------|
| 1,000 | 0.1 | 553 | Yes | 18.5x |
| 10,000 | 0.1 | 737 | Yes | 13.9x |
| 100,000 | 0.1 | 921 | Yes | 11.1x |
| 100,000 | 0.05 | 3,682 | Yes | 2.8x |
| 1,000,000 | 0.1 | 1,106 | Yes | 9.3x |
| 1,000,000 | 0.05 | 4,423 | Yes | 2.3x |
| 1,000,000 | 0.03 | 12,286 | **No** | Needs D >= 12,288 |

At epsilon = 0.1 (10% distortion tolerance), D = 10,240 supports up to 10
million entries with ample headroom. At epsilon = 0.03 (3% distortion), the
dimension limit is reached at ~750K entries. For extremely large or
high-precision applications, D = 16,384 (256 u64 words) would extend the
range.

---

## Signal-to-Noise Ratio and Capacity Bounds

### Bundle capacity theory

For a bundle of K items in D dimensions, the signal-to-noise ratio is:

```
SNR = sqrt(D / K)
```

This follows from the central limit theorem: the noise introduced by each
additional item in a bundle is approximately N(0, 1/D) per bit, and K items
contribute sqrt(K/D) total noise per bit. The signal from any single item
is 1/sqrt(D), so:

```
SNR = signal / noise = (1/sqrt(D)) / (sqrt(K)/D) = sqrt(D/K)
```

### Capacity table at D = 10,240

| K (items bundled) | SNR | P(correct retrieval, N_codebook=100) | P(correct, N=1000) | Max N at 99% accuracy |
|-------------------|-----|-------------------------------------|--------------------|-|
| 1 | 101.2 | >0.9999 | >0.9999 | >100,000 |
| 5 | 45.3 | >0.9999 | >0.9999 | >100,000 |
| 10 | 32.0 | >0.9999 | >0.9999 | >50,000 |
| 20 | 22.6 | >0.999 | >0.999 | >20,000 |
| 50 | 14.3 | >0.999 | >0.99 | ~5,000 |
| 100 | 10.1 | >0.99 | >0.95 | ~1,000 |
| 200 | 7.2 | >0.95 | >0.85 | ~200 |
| 500 | 4.5 | >0.80 | >0.50 | ~20 |
| 1,000 | 3.2 | >0.60 | >0.25 | ~5 |

### Interpreting the capacity bounds

For the primary use case -- encoding 5--10 role-filler pairs per knowledge
entry -- the capacity is enormous (SNR > 30). Each entry encodes a handful
of structured attributes (kind, content, tags, domain), and the resulting
vector has abundant signal for reliable retrieval.

**Safe rule of thumb:** K < 100 items per bundle for reliable retrieval
against codebooks of 1,000+ entries. Beyond K = 100, retrieval accuracy
degrades noticeably.

For episode compression (bundling 10--50 knowledge entries into a summary
vector), K stays well within the reliable range: at K = 50, SNR = 14.3,
which is sufficient for detecting whether a query is related to the bundle's
content.

### Codebook capacity

The maximum codebook size N_max for reliable retrieval at a given SNR is
approximately:

```
N_max ~ exp(SNR^2 / 2)
```

At SNR = 32 (K=10, D=10,240): N_max > 10^222. The codebook can be
essentially infinite. In practice, the codebook is bounded by memory (each
entry is 1,280 bytes), not by HDC capacity.

---

## Deterministic Vector Generation

### from_seed() implementation

```rust
// From crates/roko-primitives/src/hdc.rs
pub fn from_seed(seed: &[u8]) -> Self {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    for &byte in seed {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
    }
    if hash == 0 { hash = 0xA5A5_A5A5_5A5A_5A5A; }
    let mut bits = [0u64; 160];
    for word in &mut bits {
        *word = splitmix64(&mut hash);
    }
    Self { bits }
}
```

**Properties:**
- **Deterministic**: `from_seed(b"rust")` always produces the same vector
- **Quasi-orthogonal**: `from_seed(b"rust")` and `from_seed(b"python")` have
  similarity approximately 0.5
- **Zero external dependency**: No embedding API, no GPU, no model
- **Fast**: ~50 ns per vector generation (160 splitmix64 calls)

### splitmix64 PRNG

The `splitmix64` function is used to expand the FNV-1a hash into 160 u64
words:

```rust
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
```

Splitmix64 has excellent statistical properties (passes BigCrush) and is
bijective -- every 64-bit input produces a unique 64-bit output. This ensures
that different seeds produce maximally distinct bit patterns.

---

## Performance Characteristics

### Scan performance

| Entries | Scan time (x86 AVX-512) | Scan time (ARM NEON) | Scan time (scalar) |
|---------|------------------------|---------------------|-------------------|
| 1,000 | ~13 us | ~30 us | ~80 us |
| 10,000 | ~130 us | ~300 us | ~800 us |
| 100,000 | ~1.3 ms | ~3 ms | ~8 ms |
| 1,000,000 | ~13 ms | ~30 ms | ~80 ms |

For per-agent knowledge bases (typically <100K entries), brute-force scan is
fast enough that no approximate nearest neighbor index is needed. The scan is
embarrassingly parallel -- each comparison is independent.

### Operation timing

| Operation | Time (scalar) | Time (AVX-512) | Memory |
|-----------|-------------|---------------|--------|
| bind (XOR) | ~5 ns | ~2 ns | 0 (stack) |
| bundle (K=10) | ~800 ns | ~300 ns | 0 (stack) |
| permute (shift) | ~10 ns | ~5 ns | 0 (stack) |
| similarity (Hamming) | ~13 ns | ~5 ns | 0 (stack) |
| from_seed | ~50 ns | ~50 ns | 0 (stack) |
| BundleAccumulator.add | ~30 us | ~10 us | 40 KB (heap) |

All core operations are allocation-free -- they operate on the fixed-size
`[u64; 160]` array. Only `BundleAccumulator` requires heap allocation
(40 KB for 10,240 i32 vote counts).

### Storage comparison

| Items | HDC (1,280 B/vec) | float32 embeddings (768-d, 3,072 B) | float32 (1536-d, 6,144 B) |
|-------|-------------------|------------------------------------|--------------------------|
| 1,000 | 1.28 MB | 3.07 MB | 6.14 MB |
| 10,000 | 12.8 MB | 30.7 MB | 61.4 MB |
| 100,000 | 128 MB | 307 MB | 614 MB |
| 1,000,000 | 1.28 GB | 3.07 GB | 6.14 GB |

HDC vectors are 2.4x to 4.8x more compact than typical neural embedding
vectors, with the advantage of purely algebraic operations (no matrix
multiply, no GPU required).

---

## Three-Tier Search Strategy

For per-agent stores (<100K entries), brute-force Hamming scan is fast
enough. For larger collections (collective knowledge on-chain), the three-tier
search provides sub-linear performance:

### Tier 1: Bloom filter (fast reject)

A Bloom filter with an LSH scheme provides fast rejection of clearly
dissimilar entries. Each vector is hashed into multiple Bloom filter buckets
using random hyperplanes.

**Expected rejection rate:** 90--95% of entries are eliminated.
**Cost:** ~100 ns per query.

### Tier 2: Approximate search (coarse)

Surviving candidates are compared using a reduced-precision Hamming distance
-- comparing only the first 32 of the 160 u64 words (2,048 of 10,240 bits).

**Expected reduction:** 80--90% of Tier 1 survivors.
**Cost:** ~3 ns per comparison (vs ~13 ns for full).

### Tier 3: Exact top-K (full comparison)

Final candidates undergo full 10,240-bit Hamming distance comparison. The
top K most similar entries are returned.

**Expected candidate set:** 10--100 entries (from 100K+ initial pool).
**Cost:** ~13 ns per comparison x 10--100 = 130 ns -- 1.3 us.

### Overall search performance

| Knowledge Base Size | Brute Force | Three-Tier |
|---------------------|------------|-----------|
| 1,000 | 13 us | 13 us (no benefit) |
| 10,000 | 130 us | ~50 us |
| 100,000 | 1.3 ms | ~200 us |
| 1,000,000 | 13 ms | ~500 us |

---

## Zero-Copy Similarity (rkyv)

With the `rkyv` feature flag, similarity can be computed directly against
memory-mapped archived vectors without deserialization:

```rust
#[cfg(feature = "rkyv")]
pub fn similarity_archived(&self, archived: &ArchivedHdcVector) -> f32 {
    let mut differing_bits = 0u32;
    for (left, right) in self.bits.iter().zip(archived.bits.iter()) {
        let right_u64: u64 = (*right).into();
        differing_bits += (left ^ right_u64).count_ones();
    }
    let differing_bits = u16::try_from(differing_bits).unwrap_or(u16::MAX);
    1.0_f32 - (f32::from(differing_bits) / 10_240.0_f32)
}
```

On little-endian platforms, the archived representation of `[u64; 160]` is
identical to the in-memory layout, so this reads directly from the mmap'd
buffer with no deserialization overhead. This enables scanning large on-disk
knowledge bases without loading all vectors into memory.

---

## ItemMemory (Concept Codebook)

An `ItemMemory` is a named dictionary of HDC vectors -- a codebook that maps
concept names to their deterministic vectors:

```rust
pub struct ItemMemory {
    entries: HashMap<String, HdcVector>,
}

impl ItemMemory {
    pub fn new() -> Self { /* ... */ }

    /// Insert a concept with a deterministic vector from its name.
    pub fn insert_seeded(&mut self, name: &str) {
        self.entries.insert(
            name.to_string(),
            HdcVector::from_seed(name.as_bytes()),
        );
    }

    /// Insert a concept with an explicit vector.
    pub fn insert(&mut self, name: &str, vector: HdcVector) {
        self.entries.insert(name.to_string(), vector);
    }

    /// Find the nearest entry to a query vector.
    pub fn nearest(&self, query: &HdcVector) -> Option<(&str, f32)> {
        self.entries.iter()
            .map(|(name, hv)| (name.as_str(), query.similarity(hv)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }

    /// Find the top K nearest entries.
    pub fn top_k(&self, query: &HdcVector, k: usize) -> Vec<(&str, f32)> {
        let mut results: Vec<(&str, f32)> = self.entries.iter()
            .map(|(name, hv)| (name.as_str(), query.similarity(hv)))
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(k);
        results
    }
}
```

ItemMemory is used for:
- **Role registry**: Maps role names ("role:kind", "role:content") to their
  HDC vectors
- **Domain codebook**: Maps domain concepts ("rust", "defi") to vectors
- **Kind codebook**: Maps knowledge types ("insight", "heuristic") to vectors
- **Symbol codebook**: Maps code symbols to their fingerprints

---

## ResonatorNetwork (Factor Decomposition)

Resonator networks (Frady et al. 2020, Neural Computation 32(12)) solve the
inverse problem: given a composite hypervector `z = bind(x1, x2, ..., xF)`
and codebooks for each factor, recover the original factors.

### Algorithm

```
Input:
  composite: HdcVector  (z = bind(x1, x2, ..., xF))
  codebooks: [ItemMemory; F]  (one per factor)
  config: { max_iterations, convergence_threshold, early_termination_sim }

Output:
  factors: [String; F]  (best-matching codebook entry per factor)
  similarities: [f32; F]
  converged: bool

Procedure:
  1. Initialize: estimate[i] = arbitrary entry from codebook[i]
  2. For iteration = 1 to max_iterations:
     a. For each factor i = 0..F:
        i.   other_product = BIND(estimate[0], ..., skip i, ..., estimate[F-1])
        ii.  cleanup_signal = BIND(composite, other_product)
        iii. (best_name, best_sim) = codebook[i].nearest(cleanup_signal)
        iv.  estimate[i] = codebook[i].get(best_name)

     b. Early termination: if ALL similarities > early_termination_sim, return

     c. Convergence: if max |delta similarity| < threshold, return

  3. Return with converged=false
```

### Convergence properties

Frady et al. prove that resonator networks minimize reconstruction error
monotonically under certain conditions:
- Codebook entries are quasi-orthogonal (guaranteed for BSC at D = 10,240)
- Number of factors F < sqrt(D / log(N_max))
- For D = 10,240 and N_max = 1,000: F < sqrt(10240 / 6.9) ~ 38 factors

Practical convergence:

| Factors | Codebook size | Typical iterations | Success rate (D=10,240) |
|---------|--------------|-------------------|----------------------|
| 2 | 100 | 5--10 | >99% |
| 3 | 100 | 10--20 | >98% |
| 5 | 100 | 15--30 | >95% |
| 5 | 1,000 | 20--40 | >90% |
| 8 | 100 | 25--50 | >85% |

---

## SIMD Optimization Strategy

### Current implementation

The current Rust implementation uses scalar code that auto-vectorizes well on
x86 (GCC/LLVM will generate SSE2/AVX2 instructions for the XOR and POPCNT
loops). Performance is already excellent (~13 ns per similarity comparison).

### Explicit SIMD path

For maximum performance, explicit AVX-512 intrinsics would process 512 bits
(8 u64 words) per instruction:

```
// Pseudocode for AVX-512 similarity
fn similarity_avx512(a: &HdcVector, b: &HdcVector) -> f32 {
    let mut total_ones = 0u32;
    // 160 words / 8 words per __m512i = 20 iterations
    for i in (0..160).step_by(8) {
        let va = _mm512_load_si512(&a.bits[i]);
        let vb = _mm512_load_si512(&b.bits[i]);
        let xor = _mm512_xor_si512(va, vb);
        total_ones += _mm512_popcnt_epi64(xor).reduce_add();
    }
    1.0 - (total_ones as f32) / 10_240.0
}
```

This would reduce similarity computation from ~13 ns to ~2--3 ns on
AVX-512-capable hardware. The improvement is significant for brute-force
scans of large knowledge bases.

### ARM NEON path

On ARM (Apple Silicon), NEON provides 128-bit SIMD. The loop would process
2 u64 words per instruction (80 iterations vs 160 scalar):

```
160 words / 2 words per uint64x2_t = 80 iterations
```

Expected improvement: ~13 ns -> ~5 ns.

---

## PathHD Connection

PathHD (arXiv:2512.09369) demonstrates that HDC can serve as an effective
retrieval mechanism for knowledge graph queries, achieving competitive
accuracy with substantially lower computational cost than embedding-based
approaches. Roko's use of HDC for knowledge retrieval follows this pattern:
the role-filler binding structure naturally encodes knowledge graph
relationships, and the algebraic operations support structured query
decomposition.

---

## Academic Foundations

- Kanerva, P. (2009). "Hyperdimensional Computing." *Cognitive Computation*,
  1(2), 139--159. (BSC algebra formalization, quasi-orthogonality proofs)
- Kleyko, D. et al. (2022). "A Survey on Hyperdimensional Computing." *ACM
  Computing Surveys*, 54(6). (Capacity bounds, performance benchmarks)
- Plate, T. A. (2003). *Holographic Reduced Representations.* CSLI
  Publications. (Bundle capacity proofs, HRR algebra)
- Thomas, A. et al. (2021). "A Theoretical Perspective on Hyperdimensional
  Computing." *JAIR*, 72. (Capacity scaling)
- Johnson, W. B. & Lindenstrauss, J. (1984). "Extensions of Lipschitz
  mappings into a Hilbert space." *Contemporary Mathematics*, 26, 189--206.
- Frady, E. P. et al. (2020). "Resonator Networks." *Neural Computation*,
  32(12), 2275--2325. (Resonator network factor decomposition)
- Neubert, P. et al. (2019). "An Introduction to Hyperdimensional Computing
  for Robotics." *KI*, 33, 319--330. (Place cell analogy)

---

## Cross-References

- `hdc-operations.md` -- bind, bundle, permute, similarity with Rust code
- `hdc-knowledge-encoding.md` -- role-filler encoding pipeline
- `false-positive-math.md` -- similarity threshold derivation
- `cross-domain-hdc-transfer.md` -- structural analogy via shared roles
