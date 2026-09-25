# HDC Fingerprints for Structural Similarity

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 9

---

## Mathematical Foundations

### Hyperdimensional Computing

HDC (Kanerva 2009) is a computational framework based on the algebraic
properties of high-dimensional random vectors. The core insight: in
sufficiently high-dimensional spaces (thousands of bits), random vectors
are almost certainly near-orthogonal. This means:

1. **Capacity** -- A space of D-bit vectors can represent an exponential
   number of distinct concepts without interference.
2. **Composability** -- Vectors can be combined using algebraic operations
   that preserve the ability to detect component parts.
3. **Robustness** -- Small perturbations do not destroy the overall
   structure because similarity is distributed across many bits.

Three operations form the algebra:

| Operation | Symbol | Implementation | Preserves |
|-----------|--------|---------------|-----------|
| **Bind** | xor | XOR | Associates two concepts (role-filler binding) |
| **Bundle** | maj | Majority vote | Creates a set-like superposition |
| **Permute** | rot | Bit rotation | Creates ordered sequences |

### Why 10,240 bits?

| D | Capacity | Hamming precision | Storage per vector |
|---|----------|-------------------|-------------------|
| 1,024 | ~100 items | +/-3.1% | 128 bytes |
| 4,096 | ~1,000 items | +/-1.6% | 512 bytes |
| **10,240** | **~10,000 items** | **+/-1.0%** | **1,280 bytes** |
| 65,536 | ~100,000 items | +/-0.4% | 8,192 bytes |

The 10,240-bit choice (160 u64 words) balances:

1. **Sufficient capacity** -- ~10,000 distinguishable items for
   workspace-scale indexing.
2. **Precision** -- +/-1.0% Hamming precision distinguishes similar from
   dissimilar with high confidence.
3. **Performance** -- 160 words x 8 bytes = 1,280 bytes per fingerprint.
   XOR + popcount over 160 words completes in ~50ns.

---

## The Encoding Scheme

Each symbol's fingerprint encodes three properties:

```
fingerprint(symbol) = bind(role_vector(kind), bundle(name_vector, context_vector))
```

### Role vectors

Each `SymbolKind` maps to a deterministic base vector:

```rust
fn role_vector(kind: &SymbolKind) -> [u64; WORDS] {
    let seed: &[u8] = match kind {
        SymbolKind::Function => b"roko:role:function",
        SymbolKind::Struct   => b"roko:role:struct",
        SymbolKind::Enum     => b"roko:role:enum",
        SymbolKind::Trait    => b"roko:role:trait",
        SymbolKind::Const    => b"roko:role:const",
        SymbolKind::Type     => b"roko:role:type",
        SymbolKind::Module   => b"roko:role:module",
        SymbolKind::Impl     => b"roko:role:impl",
        _                    => b"roko:role:unknown",
    };
    vector_from_seed(seed)
}
```

`vector_from_seed()` uses FNV-1a hashing to produce a 64-bit seed, then
expands via splitmix64 PRNG. The deterministic PRNG ensures the same seed
always produces the same vector. Different seeds produce near-orthogonal
vectors.

### Name encoding via character trigrams

Symbol names are encoded using overlapping character trigrams:

```rust
fn encode_name(name: &str) -> [u64; WORDS] {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() < 3 {
        return vector_from_seed(name.as_bytes());
    }
    let trigrams: Vec<[u64; WORDS]> = chars
        .windows(3)
        .map(|w| {
            let trigram: String = w.iter().collect();
            vector_from_seed(trigram.as_bytes())
        })
        .collect();
    bundle(&trigrams)
}
```

For `process_input`: trigrams `pro`, `roc`, `oce`, ... `put`. This encoding
has two properties:

1. **Similar names produce similar vectors** -- `process_input` and
   `process_output` share 7 of 11 trigrams, so their name vectors are
   similar.
2. **Order sensitivity** -- Different orderings produce different trigram
   sets: `abc` and `bca` share only partial overlap.

### Context encoding

The context vector captures surrounding source text:

```rust
let ctx_vec = vector_from_seed(context);
```

The context provides a "which file is this in" signal that helps
distinguish identically-named symbols in different files.

### Composition

```rust
pub fn fingerprint_symbol(symbol: &Symbol, context: &[u8]) -> HdcFingerprint {
    let role_vec = role_vector(&symbol.kind);
    let name_vec = encode_name(&symbol.name);
    let ctx_vec = vector_from_seed(context);
    let combined = bundle(&[name_vec, ctx_vec]);
    HdcFingerprint {
        bits: bind(&role_vec, &combined),
    }
}
```

The bundle preserves both name and context (superposition). The bind tags
the result with the symbol kind. This means:
- Two functions with similar names in similar contexts: high similarity
- A function and a struct with the same name: lower similarity
- Two functions with different names but same context: moderate similarity

---

## Core Operations

### Bundle (majority vote)

```rust
fn bundle(vectors: &[[u64; WORDS]]) -> [u64; WORDS]
```

Sets each bit to 1 if more than half of the input vectors have that bit set.
Creates a "consensus" vector similar to all inputs -- the HDC centroid.

### Bind (XOR)

```rust
fn bind(a: &[u64; WORDS], b: &[u64; WORDS]) -> [u64; WORDS]
```

XOR is its own inverse: `bind(bind(a, b), b) = a`. It preserves
dimensionality and distributes over bundle.

### Hamming distance

```rust
fn hamming_distance(a: &[u64; WORDS], b: &[u64; WORDS]) -> u32
```

Counts differing bits. On modern x86 CPUs, `count_ones()` compiles to the
`POPCNT` instruction -- single cycle per word.

### Similarity (normalized Hamming)

```rust
impl HdcFingerprint {
    pub fn similarity(&self, other: &Self) -> f64 {
        let dist = hamming_distance(&self.bits, &other.bits);
        1.0 - (f64::from(dist) / TOTAL_BITS as f64)
    }
}
```

- 1.0 = identical fingerprints (0 differing bits)
- 0.5 = random (expected for unrelated vectors)
- 0.0 = maximally different

---

## File-Level Fingerprints

```rust
pub fn fingerprint_file(source: &SourceFile) -> HdcFingerprint
```

Bundles all symbol fingerprints in a file. Enables file-level similarity
search for finding test file correspondences, duplicate module detection,
and clustering related files.

---

## Code Clone Detection

| Clone Type | Definition | HDC detects | Neural detects |
|-----------|-----------|-------------|----------------|
| Type-1 | Exact copies | Yes (sim ~0.95+) | Yes |
| Type-2 | Renamed identifiers | Partially (trigram overlap) | Yes |
| Type-3 | Near-miss (statements changed) | Weakly | Yes |
| Type-4 | Semantic clones | No | Yes (embeddings) |

The planned hybrid pipeline uses HDC as a fast first pass and neural
embeddings for refinement. PPR re-ranking boosts structurally adjacent
clones above architecturally unrelated matches.

---

## Comparison with Neural Embeddings

| Property | HDC (10,240-bit) | Dense embedding (384-dim float) |
|----------|-----------------|-------------------------------|
| Vector size | 1,280 bytes | 1,536 bytes |
| Computation | ~5us (CPU only) | ~10ms (GPU) or ~100ms (CPU) |
| Similarity op | ~50ns (XOR+POPCNT) | ~500ns (dot product) |
| Structural similarity | Good | Excellent |
| Semantic similarity | Limited | Excellent |
| Model dependency | None | Requires embedding model |
| Incremental update cost | ~5us per symbol | ~10ms per symbol |

HDC excels for structural similarity and is 200x--20,000x faster. The
planned design uses both: HDC for fast structural matching, embeddings for
semantic refinement.

---

## Performance Characteristics

| Operation | Time |
|-----------|------|
| `vector_from_seed()` | ~200ns |
| `encode_name()` (15-char name) | ~3us |
| `fingerprint_symbol()` | ~5us |
| `fingerprint_file()` (10 symbols) | ~50us |
| `similarity()` | ~50ns |

| Metric | Value |
|--------|-------|
| Bytes per fingerprint | 1,280 |
| ~5,000 symbol fingerprints | ~6.25 MB |
| ~122,000 symbol fingerprints | ~150 MB |

---

## Planned Enhancements

### HNSW index

For large indices, HNSW (Hierarchical Navigable Small World) graphs provide
approximate nearest-neighbor search in O(log N) time instead of brute-force
O(N).

### Content-aware fingerprinting

Richer encoding using function body, doc comments, parameter types, and
return type for more discriminating fingerprints.

### Dense embedding integration

The planned embedding layer supports pluggable models (CodeBERT, CodeSage,
Jina Code v2) via the `fastembed` crate for semantic search alongside HDC
structural matching.

---

## Verified Behaviors (10 tests)

| Test | What it verifies |
|------|-----------------|
| `identical_symbols_identical_fingerprints` | Same symbol + context = 1.0 |
| `similar_names_high_similarity` | `process_input` vs `process_output` > 0.5 |
| `different_kinds_lower_similarity` | `Config(Function)` vs `Config(Struct)` < 0.9 |
| `completely_different_symbols_low_similarity` | Unrelated symbols < 0.7 |
| `fingerprint_file_deterministic` | Same file = identical fingerprints |
| `fingerprint_file_empty_symbols` | No symbols = content-based fingerprint |
| `self_similarity_is_one` | Any vs itself = exactly 1.0 |
| `short_name_encoding` | Single-character names produce valid fingerprints |
| `comparison_performance_under_1ms` | 10,000 comparisons < 1ms |

---

## Academic Foundations

- **Hyperdimensional Computing**: Kanerva (2009). *Cognitive Computation*
  1(2). Mathematical foundation for HDC.
- **code2vec**: Alon, Zilberstein, Levy, and Brody (2019). *POPL*.
  Distributed code representations without training.
- **Holographic reduced representations**: Plate (2003). Oxford University
  Press. Binding and bundling operations.
- **CodeBERT**: Feng, Guo, Tang, et al. (2020). *EMNLP*. Pre-trained model
  for code understanding.
- **CodeSage**: Zhang et al. (2024). Contrastive learning for code search.
- **StarCoder**: Li et al. (2023). arXiv:2305.06161. Code embedding model.

---

## Cross-References

- See [symbol-extraction.md](./symbol-extraction.md) for the symbols that
  fingerprints encode
- See [context-assembly-from-code.md](./context-assembly-from-code.md) for
  how HDC similarity drives context retrieval
- See [index-db-scaling.md](./index-db-scaling.md) for persistent
  fingerprint storage
- See [snapshot-optimization.md](./snapshot-optimization.md) for rkyv
  zero-copy fingerprint snapshots
