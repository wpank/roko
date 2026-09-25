# 08-learning/05 -- HDC Clustering

> Incremental DBSCAN over 10,240-bit hyperdimensional computing vectors
> discovers natural episode groupings at ~50ns per comparison. Codebook
> defragmentation with provenance tracking removes redundant bundled
> vectors while preserving traceability to source episodes.
> Cite: Kanerva, P. (2009).

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 5

**Source:** `crates/roko-learn/src/hdc_clustering.rs` (`HdcCluster`,
`defragment`, `DefragConfig`, `DefragResult`),
`crates/roko-primitives/src/hdc.rs` (`HdcVector`, `text_fingerprint`,
`bundle`)

**Academic basis:** Kanerva, P. (2009). Hyperdimensional Computing: An
Introduction to Computing in Distributed Representation with
High-Dimensional Random Vectors. *Cognitive Computation* 1(2), 139-159.
Ester, M. et al. (1996). A Density-Based Algorithm for Discovering
Clusters in Large Spatial Databases with Noise. *KDD 1996*.
Kanerva, P. (1988). *Sparse Distributed Memory*. MIT Press.

---

## 1. Purpose

HDC clustering groups episodes by semantic similarity using 10,240-bit
binary vectors. This enables pattern discovery without embedding models --
similarity is a Hamming distance computation at ~50ns per comparison,
compared to ~1us for cosine distance on 768-dimensional float embeddings.

The speed advantage is critical: during task dispatch, the system scans
hundreds of historical episodes to find relevant templates and playbook
rules. At 50ns per comparison, scanning 1000 episodes takes ~50us -- well
within the per-task latency budget of < 5ms.

---

## 2. Hyperdimensional Computing Vectors

### 2.1 Representation

Each HDC vector is a 10,240-bit binary vector stored as a `[u128; 80]`
array (80 x 128-bit words = 10,240 bits). The dimensionality is chosen
to be large enough that random vectors are approximately orthogonal with
high probability (Kanerva 2009).

Kanerva's key insight is that in high-dimensional spaces, random points
are approximately equidistant from each other. Two random 10,240-bit
vectors have expected Hamming distance of 5,120 (exactly half the bits
differ). This quasi-orthogonality means that operations like bundling
(superposition) and binding (XOR) produce predictable, well-behaved
results: a bundle of K vectors is similar to each constituent but
dissimilar to non-constituents, with interference bounded by O(1/sqrt(K)).

### 2.2 Operations

| Operation | Definition | Complexity |
|-----------|-----------|------------|
| Similarity | `1.0 - (hamming_distance / dimension)` | O(d/w) where w = word size |
| Bundle | Bitwise majority vote across inputs | O(d * n) |
| Bind | Bitwise XOR | O(d/w) |
| Permute | Rotate bits by k positions | O(d/w) |

Similarity between two vectors is computed as normalized Hamming distance.
Two random 10,240-bit vectors have expected similarity of 0.50 (exactly
half the bits match by chance). Similarity above 0.70 indicates meaningful
shared structure; below 0.55 is noise.

### 2.3 Fingerprinting

Every episode is fingerprinted with two HDC vectors:

1. **Text fingerprint** -- encodes semantic content (task description, gate
   verdicts) via `text_fingerprint`. Character n-grams are mapped to random
   bit vectors and bundled together.

2. **Metadata fingerprint** -- encodes structural identity (agent_id,
   task_id, role, crate name) for structural similarity matching. Each
   metadata field is hashed to a deterministic bit vector and bound (XOR)
   with a position-encoding vector to preserve field ordering.

### 2.4 Why 10,240 Bits?

The dimensionality choice follows Kanerva's analysis of the capacity-
fidelity tradeoff:

| Dimension | Capacity (items before interference) | Storage per vector |
|-----------|-------------------------------------|-------------------|
| 1,024 | ~30 | 128 B |
| 4,096 | ~200 | 512 B |
| **10,240** | **~1,000** | **1.25 KB** |
| 65,536 | ~10,000 | 8 KB |

At 10,240 bits, the codebook can hold ~1,000 distinct bundled vectors
before interference degrades retrieval accuracy below 95%. This is
sufficient for the episode window size (200 episodes) and the number of
active clusters (typically < 100).

---

## 3. Incremental DBSCAN

Standard DBSCAN (Ester et al. 1996) is a batch algorithm that requires
O(n^2) distance computations. For a continuously growing episode log, batch
processing is prohibitive. Roko uses an incremental variant that processes
one episode at a time.

### 3.1 Algorithm

```
On new episode e with HDC fingerprint h:

    1. Compute similarity to each existing cluster's superposition vector:
       sim_c = similarity(h, cluster_c.superposition) for all c

    2. Find best matching cluster:
       best_c = argmax_c(sim_c)

    3. If sim_c > eps_similarity (default 0.72):
        a. Add episode e to cluster best_c
        b. Update cluster superposition:
           cluster.superposition = bundle(cluster.members)
        c. Update cluster statistics:
           cluster.count += 1
           cluster.last_updated = now()

    4. If no cluster matches (all sim_c <= eps_similarity):
        a. Add e to noise buffer
        b. Check noise buffer for newly similar episodes:
           For each pair (e_i, e_j) in noise_buffer:
               if similarity(e_i, e_j) > eps_similarity:
                   found_similar += 1
        c. If noise_buffer has >= min_points (default 3) similar episodes:
           -> Form new cluster from those episodes
           -> Remove them from noise buffer
```

### 3.2 Parameters

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `eps_similarity` | 0.72 | Minimum similarity to join a cluster |
| `min_points` | 3 | Minimum episodes to form a new cluster |
| `TEMPLATE_SUGGESTION_MAX_AGE_DAYS` | 30 | Maximum age for template candidates |
| `TEMPLATE_SUGGESTION_MAX_CANDIDATES` | 256 | Maximum episodes to scan |
| `TEMPLATE_SUGGESTION_MIN_SIMILARITY` | 0.70 | Minimum similarity for template match |

### 3.3 Cluster State

```rust
pub struct HdcCluster {
    /// Cluster identifier.
    pub id: u64,
    /// Superposition vector (bundle of all member fingerprints).
    pub superposition: HdcVector,
    /// Member episode IDs.
    pub members: Vec<String>,
    /// Count of episodes in this cluster.
    pub count: usize,
    /// Centroid metadata (most common role, category, etc.).
    pub metadata: ClusterMetadata,
}
```

The superposition vector is the bitwise majority vote across all member
fingerprints. It represents the "average" of the cluster in HDC space. New
episodes are compared against this superposition rather than against every
individual member, giving O(k) comparison cost per new episode (where k is
the number of clusters, typically << n episodes).

---

## 4. Codebook Defragmentation with Provenance (E25)

### 4.1 Purpose

As episodes accumulate, the HDC codebook grows. Some bundled vectors are
redundant: their constituent episodes all exist independently at higher
tiers (e.g., as confirmed knowledge entries). Defragmentation removes these
redundant bundles while preserving provenance -- every remaining vector
traces back to its source episodes.

### 4.2 Algorithm

```rust
pub fn defragment(
    entries: &[KnowledgeEntry],
    config: &DefragConfig,
) -> DefragResult {
    // 1. Identify bundled vectors (vectors that are superpositions
    //    of other vectors in the codebook)
    // 2. For each bundled vector, check if ALL constituents exist
    //    independently at the same or higher tier
    // 3. If so, mark the bundle as redundant
    // 4. Remove redundant bundles, preserving their provenance chain
}
```

### 4.3 Configuration

```rust
pub struct DefragConfig {
    /// Minimum similarity to consider a vector a constituent of a bundle.
    pub constituent_threshold: f64,  // default: 0.85
    /// Minimum number of constituents before a bundle is considered
    /// for removal.
    pub min_bundle_size: usize,      // default: 2
}
```

### 4.4 Output

```rust
pub struct DefragResult {
    /// Number of redundant bundles removed.
    pub removed: usize,
    /// Number of bundles retained (constituents not fully present).
    pub retained: usize,
    /// Provenance chains for removed bundles (source episode IDs).
    pub provenance: Vec<ProvenanceChain>,
}
```

Provenance tracking is essential for auditability: when a bundle is
removed, the system records which source episodes comprised it. This allows
reconstruction of the bundle if its constituents are later deleted or if
the defragmentation decision is reversed.

---

## 5. Template Suggestion

### 5.1 Purpose

Given a new task context, the system suggests templates from historical
episodes that are semantically similar. This supports playbook rule
matching, skill injection, and cascade router context enrichment.

### 5.2 Algorithm

```
Template suggestion for new task T:

    1. Compute HDC fingerprint h_T from task description + metadata.

    2. Scan recent episodes (within TEMPLATE_SUGGESTION_MAX_AGE_DAYS = 30):
       candidates = episodes.filter(|e| e.timestamp > now - 30d)
                           .take(TEMPLATE_SUGGESTION_MAX_CANDIDATES = 256)

    3. Compute similarity for each candidate:
       scores = candidates.map(|e| (e, similarity(h_T, e.hdc_fingerprint)))

    4. Filter by minimum similarity:
       matches = scores.filter(|(_, sim)| sim > TEMPLATE_SUGGESTION_MIN_SIMILARITY)

    5. Sort by similarity descending.

    6. Return top-k matches (default k = 5).
```

### 5.3 Performance

At 50ns per Hamming distance computation, scanning 256 candidates takes
~13us. This is well within the 5ms per-task latency budget. The age filter
and candidate limit ensure the scan does not grow with the total episode
log size.

---

## 6. Cluster Evolution

### 6.1 Cluster Lifecycle

Clusters evolve through three phases:

1. **Formation** -- min_points similar episodes in the noise buffer form a
   new cluster. The cluster's superposition is initialized from the founding
   members.

2. **Growth** -- new episodes matching the superposition are absorbed. The
   superposition is updated to reflect the expanded membership. As the
   cluster grows, its superposition becomes a better representation of the
   cluster's central tendency.

3. **Decay** -- clusters whose members are all older than
   `TEMPLATE_SUGGESTION_MAX_AGE_DAYS` are aged out of the active candidate
   pool. Their fingerprints remain in the codebook but are not scanned for
   template suggestions.

### 6.2 Cluster Splitting

When a cluster's superposition becomes too diffuse (internal similarity
drops below a threshold), it may need splitting. The superposition of a
diverse cluster is a blur that matches everything weakly but nothing
strongly. Signs of a cluster that needs splitting:

- New episodes match the cluster but with similarity near `eps_similarity`
  (barely above threshold)
- The cluster's internal pairwise similarity variance is high
- The cluster's metadata shows multiple distinct roles or categories

Splitting is not yet implemented in the shipped code -- it is noted as
a target design improvement.

---

## 7. Relationship to Knowledge Tiers

HDC clustering operates at the pattern discovery level (Tier 2 in the
learning stack). Clusters discovered by DBSCAN feed into:

1. **Playbook rules** (Tier 4) -- when a cluster shows consistent
   pass/fail patterns, a rule is extracted.
2. **Skill library** -- when a cluster of successful episodes shares a
   common approach, a skill entry is created.
3. **Cascade router** -- cluster-level statistics (pass rate by model
   within cluster) inform routing decisions.

The tiers flow upward: episodes (Tier 1) are clustered into patterns
(Tier 2), patterns that pass validation become heuristics (Tier 3), and
validated heuristics are compiled into playbook rules (Tier 4).

---

## 8. Comparison with Embedding-Based Clustering

| Property | HDC (10,240-bit) | Float Embeddings (768-dim) |
|----------|-----------------|--------------------------|
| Comparison speed | ~50ns (Hamming distance) | ~1us (cosine distance) |
| Storage per vector | 1.25 KB | 3.0 KB (f32) |
| Incremental update | Bitwise majority (fast) | Mean recomputation (moderate) |
| Semantic quality | Good for structural patterns | Better for semantic nuance |
| External dependency | None (computed locally) | Requires embedding model |
| Offline capability | Full | Requires model access |

HDC is chosen over float embeddings because:
1. **Speed** -- 20x faster per comparison, enabling real-time scan during
   task dispatch.
2. **Independence** -- no external embedding model dependency.
3. **Offline** -- works without network access.
4. **Sufficient quality** -- for pattern discovery (grouping similar tasks),
   structural similarity suffices. Deep semantic understanding is not
   required.

---

## 9. Kanerva's Theoretical Foundation (2009)

### 9.1 Distributed Representation

Kanerva's central thesis is that high-dimensional random binary vectors
provide a computationally efficient substrate for symbolic AI operations.
Traditional symbolic representations (variable-length strings, structured
records) require exact matching. HDC representations support approximate
matching via Hamming distance, enabling graceful degradation: a query that
is 90% similar to a stored pattern retrieves the pattern with high
probability, while a query that is 50% similar (random chance) does not.

### 9.2 Holographic Reduced Representation

HDC builds on Plate's (1995) Holographic Reduced Representations (HRRs),
replacing circular convolution over real-valued vectors with XOR binding
over binary vectors. The binary simplification sacrifices some
representational capacity but gains enormous computational efficiency: XOR
is a single instruction per 64-bit word, while circular convolution is
O(d log d) via FFT.

### 9.3 Capacity and Interference

The capacity of an HDC codebook -- the maximum number of stored patterns
retrievable with high accuracy -- scales as O(d / log d) where d is the
vector dimensionality. For d = 10,240:

```
capacity ~ 10,240 / log2(10,240) ~ 10,240 / 13.3 ~ 770
```

In practice, the system uses ~200 episode fingerprints plus ~50-100 cluster
superpositions, well within the theoretical capacity bound.

---

## References

- Kanerva, P. (2009). Hyperdimensional Computing: An Introduction to
  Computing in Distributed Representation with High-Dimensional Random
  Vectors. *Cognitive Computation* 1(2), 139-159.
- Kanerva, P. (1988). *Sparse Distributed Memory*. MIT Press.
- Ester, M. et al. (1996). A Density-Based Algorithm for Discovering
  Clusters in Large Spatial Databases with Noise. *KDD 1996*.
- Plate, T.A. (1995). Holographic Reduced Representations. *IEEE
  Transactions on Neural Networks* 6(3), 623-641.
