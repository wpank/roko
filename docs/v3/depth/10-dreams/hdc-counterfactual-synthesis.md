# HDC Counterfactual Synthesis

> **v3 depth file** -- `/docs/v3/depth/10-dreams/hdc-counterfactual-synthesis.md`
> Canonical source: v1 `docs/v1/10-dreams/06-hdc-counterfactual-synthesis.md`
> Implementation: `crates/roko-primitives/src/hdc.rs`, `crates/roko-learn/src/hdc_clustering.rs`,
> `crates/roko-dreams/src/cycle.rs`
> Status: **Wired** -- `HdcVector`, `text_fingerprint`, K-medoids clustering, and
> counterfactual record generation are live in cycle.rs

---

## 1. What HDC Counterfactual Synthesis Is

Hyperdimensional Computing (HDC), also known as Vector Symbolic Architectures
(VSA), provides the mathematical substrate for Roko's dream counterfactual
operations. Where LLM-based counterfactuals operate on natural language (slow,
expensive, creative), HDC counterfactuals operate on 10,240-bit Binary Spatter
Code (BSC) vectors (fast, free, mechanical):

- **LLM counterfactuals** (REM imagination): ~$0.01 per counterfactual
- **HDC counterfactuals**: ~1,000 counterfactuals per millisecond, zero cost

HDC counterfactual synthesis scans the agent's knowledge space for unexplored
regions, identifies potential connections, and generates candidate directions
for LLM-based reasoning to explore.

---

## 2. HDC Fundamentals

### 2.1 Binary Spatter Codes (BSC)

Roko uses 10,240-bit BSC vectors. Each vector is a binary string where each
bit is independently set to 0 or 1. The high dimensionality provides:

- **Quasi-orthogonality**: Random vectors are nearly orthogonal (Hamming
  similarity ~0.50) with high probability
- **Distributed representation**: Information is spread across all bits
- **Noise tolerance**: Corrupting a small fraction of bits barely changes
  similarity scores

### 2.2 Core Operations

| Operation | Definition | Dream Use |
|-----------|-----------|-----------|
| **Binding** (XOR) | `A XOR B` | Associate two concepts |
| **Bundling** (majority) | `MAJ(A, B, C, ...)` | Create composite concepts |
| **Permutation** (cyclic shift) | `SHIFT(A, k)` | Create role/filler bindings |
| **Similarity** (Hamming) | `1 - hamming(A, B) / D` | Measure concept relatedness |

### 2.3 Text Fingerprinting

Episodes are converted to HDC vectors via `text_fingerprint`:

```rust
let episode_vector = text_fingerprint(&episode_text);
```

The fingerprint encodes character n-grams using position-dependent hashing,
producing vectors where semantically similar text yields high Hamming similarity.

---

## 3. Counterfactual Generation

### 3.1 Semantic Axis Perturbation

The dream cycle generates counterfactuals by perturbing episodes along semantic
axes (model, gate, task-type, outcome). Each perturbation:

1. Takes the episode's HDC vector as a base
2. Selects a semantic axis to perturb
3. Generates a replacement value from the episode corpus
4. Creates a counterfactual variant

The counterfactual record captures the perturbation:

```rust
struct DreamCounterfactualRecord {
    generated_at: DateTime<Utc>,
    cluster_key: DreamClusterKey,
    focus_axis: String,
    original_value: String,
    replacement_value: String,
    replacement_source: String,
    hypothesis: String,
    permutation: usize,
    base_signature: u64,
    counterfactual_signature: u64,
    similarity: f32,
}
```

### 3.2 Neighborhood Exploration

HDC permutation explores the neighborhood of existing knowledge. A cyclic
bit-shift by k positions on a 10,240-bit vector produces a new vector that is:

- Related to the original (shares structural features)
- Distinct from the original (different positional encoding)
- Deterministic (same shift always produces the same result)

Nearest neighbors of the permuted vector in NeuroStore identify potentially
relevant knowledge entries that the agent has not yet connected.

---

## 4. K-Medoids Clustering

The `CrossEpisodeConsolidator` clusters episode vectors using K-medoids:

```rust
pub fn consolidate(
    &self,
    episode_vectors: &[(usize, HdcVector)],
) -> Vec<CrossEpisodeMetaPattern> {
    let config = KMedoidsConfig {
        k: self.target_clusters,
        max_iterations: self.max_iterations,
    };
    let result = k_medoids(&vectors, &config);
    // convert clusters to meta-patterns with coherence scores
}
```

K-medoids is preferred over K-means because BSC vectors are binary -- arithmetic
mean is undefined for binary representations.

---

## 5. Similarity Thresholds

| Similarity Range | Interpretation | Dream Action |
|-----------------|----------------|-------------|
| > 0.85 | Near-duplicate | Skip (already covered) |
| 0.60--0.85 | Related | Potential consolidation candidate |
| 0.40--0.60 | Weakly related | Combinational creativity target |
| < 0.40 | Unrelated | Hypnagogia anti-correlation target |

---

## 6. Academic Citations

| Paper | Contribution |
|-------|-------------|
| Kanerva (2009), Cognitive Computation 1(2) | Hyperdimensional Computing fundamentals |
| Gayler (2003) | Vector Symbolic Architectures |
| Plate (2003) | Holographic Reduced Representations |
| Rahimi et al. (2019) | HDC for language understanding |

---

## 7. Cross-References

| Document | Relevance |
|----------|-----------|
| [nrem-replay.md](nrem-replay.md) | HDC clustering during NREM |
| [rem-imagination.md](rem-imagination.md) | LLM counterfactuals complement HDC |
| [dream-evolution.md](dream-evolution.md) | HDC permutation for knowledge recombination |
| [hypnagogia-engine.md](hypnagogia-engine.md) | Anti-correlated HDC retrieval |
