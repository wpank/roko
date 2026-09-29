# 08-learning/06 -- Pattern Discovery: Trigram Mining

> Trigram mining over episode gate-verdict sequences, HDC clustering for
> cross-episode consolidation, and the intermediate tier between episodes
> and playbook rules.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/pattern_discovery.rs`,
`crates/roko-learn/src/hdc_clustering.rs`

**Cross-references:** [episode-logger](episode-logger.md),
[playbook-store](playbook-store.md)

---

## 1. Purpose

Pattern discovery mines recurring structural signals from the episode stream.
The core technique is trigram mining: extracting every three-action subsequence
from each episode's gate verdict sequence, counting how often each trigram
appears across episodes, and surfacing those that exceed a support threshold as
recurring patterns. These patterns are the intermediate tier in the three-tier
memory hierarchy (episodes -> patterns -> playbook rules).

The module also provides cross-episode consolidation using HDC clustering:
grouping structurally similar episodes into clusters, then extracting
meta-patterns that describe common traits of each cluster.

---

## 2. EpisodeView Trait

Pattern mining is decoupled from the concrete `Episode` type via a trait:

```rust
pub trait EpisodeView {
    fn actions(&self) -> &[String];
    fn succeeded(&self) -> bool;
}
```

In practice, the `LearningRuntime` wraps each `Episode` in an `EpisodeActions`
adapter that extracts gate names from `gate_verdicts`:

```rust
struct EpisodeActions {
    actions: Vec<String>,   // ["compile", "test", "lint", "diff"]
    success: bool,
}
```

---

## 3. Trigram Mining Algorithm

### 3.1 Ingest

For each episode, the miner extracts all three-action subsequences:

```
Episode actions: ["read", "edit", "compile", "test", "lint"]

Trigrams:
  ("read", "edit", "compile")
  ("edit", "compile", "test")
  ("compile", "test", "lint")
```

Each trigram is hashed to a stable 64-bit signature using FNV-1a. The miner
maintains a `BTreeMap<u64, TrigramStats>` keyed by signature:

```rust
struct TrigramStats {
    trigram: [String; 3],
    signature: u64,
    support: u32,          // distinct episodes containing this trigram
    first_seen_ms: i64,
    last_seen_ms: i64,
}
```

A trigram's support count is the number of **distinct episodes** that contain
it (not the total occurrences). This prevents a single long episode from
inflating support counts.

### 3.2 Discover

After ingesting a batch, `PatternMiner::discover()` returns all trigrams whose
support clears configured thresholds:

```rust
pub struct PatternMiner {
    min_support: u32,       // minimum distinct episodes (default: 2)
    min_confidence: f32,    // minimum support/total ratio (default: 0.5)
}
```

Each qualifying trigram becomes a `Pattern`:

```rust
pub struct Pattern {
    pub id: String,             // "trigram:<signature>"
    pub signature: u64,
    pub description: String,    // "read -> edit -> test"
    pub support_count: u32,
    pub confidence: f32,        // support_count / total_episodes
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
}
```

### 3.3 Promote

Patterns with sufficient support (typically >= 5 episodes) are candidates for
promotion to playbook rules. See [playbook-store](playbook-store.md) for the
promotion criteria and GRASP regression gating.

---

## 4. Why Trigrams?

| N-gram size | Properties |
|-------------|------------|
| Unigrams (1) | Too generic -- "compile" appears in every episode |
| Bigrams (2) | Still generic -- "edit->compile" is nearly universal |
| **Trigrams (3)** | Captures meaningful action patterns |
| 4-grams (4) | Too specific -- insufficient support for extraction |

Trigrams strike the right balance between specificity and support.

---

## 5. HDC Clustering for Cross-Episode Consolidation

### 5.1 k-Medoids Algorithm

The `hdc_clustering` module implements Partitioning Around Medoids (PAM) over
10,240-bit `HdcVector`s:

```rust
pub struct KMedoidsConfig {
    pub k: usize,              // number of clusters (default: 3)
    pub max_iterations: usize, // convergence limit (default: 100)
}
```

Algorithm:

1. **Initialize** -- greedy farthest-first seeding.
2. **Assign** -- each point to the nearest medoid (distance = 1 - HDC
   Hamming similarity).
3. **Update** -- for each cluster, the member minimizing total intra-cluster
   distance becomes the new medoid.
4. Repeat until convergence or `max_iterations`.

### 5.2 Cross-Episode Consolidation

```
Episodes with HDC fingerprints
    |
    v
k-medoids clustering (k=3, HDC similarity)
    |
    v
For each cluster:
    +-- Identify common trigrams across cluster members
    +-- Compute cluster-level pass rate
    +-- Extract distinguishing features (files, roles, categories)
    +-- Produce CrossEpisodeConsolidationReport
```

The consolidation report identifies structural groupings in the episode stream
that may not be visible from individual trigram analysis. For example, a cluster
of episodes involving cross-crate modifications with a high failure rate
suggests a systemic issue, even if no single trigram captures the pattern.

---

## 6. HDC Distance Metric

```
similarity(a, b) = 1 - (hamming_distance(a, b) / 10240)
distance(a, b) = hamming_distance(a, b) / 10240
```

- Identical vectors: distance = 0, similarity = 1.0
- Orthogonal vectors: distance ~ 0.5, similarity ~ 0.5
- Maximally different: distance = 1.0, similarity = 0.0

---

## 7. Operating Frequency

Pattern discovery runs at **every 20 episodes** -- the slowest learning loop
in the system. This frequency separation prevents oscillation: rapid pattern
updates could cause playbook rules to be promoted and demoted on noisy
short-term data.

```
Learning Loop Frequencies:
    +-- Cascade router:       every episode          (highest)
    +-- Gate thresholds:      every 5 episodes
    +-- Pattern discovery:    every 20 episodes       (lowest)
    +-- Cross-episode:        on-demand or periodic
```

---

## 8. Practical Example

### 8.1 Episode Stream

```
Episode 1: actions = ["read", "edit", "compile", "test"]           success=true
Episode 2: actions = ["read", "edit", "compile", "fix", "compile"] success=true
Episode 3: actions = ["edit", "compile", "test"]                   success=true
Episode 4: actions = ["read", "edit", "compile", "test"]           success=true
Episode 5: actions = ["edit", "compile", "lint", "fix", "compile"] success=false
```

### 8.2 Support Counts

```
(read,edit,compile):    support=3  confidence=3/5=0.60  -> PATTERN
(edit,compile,test):    support=3  confidence=3/5=0.60  -> PATTERN
(fix,compile,test):     support=2  confidence=2/5=0.40  -> below threshold
(edit,compile,fix):     support=1  confidence=1/5=0.20  -> below threshold
```

### 8.3 Discovered Patterns

```
Pattern "trigram:0xA1B2C3": read -> edit -> compile
    support: 3 episodes, confidence: 0.60

Pattern "trigram:0xD4E5F6": edit -> compile -> test
    support: 3 episodes, confidence: 0.60
```

These capture the dominant successful sequence: read, edit, compile, test.

---

## 9. Performance

| Operation | Complexity | Typical Time |
|-----------|-----------|--------------|
| Ingest one episode | O(n) where n = action count | < 1us |
| Discover patterns | O(m) where m = unique trigrams | < 100us for 1000 trigrams |
| HDC fingerprint comparison | O(1) bit-parallel Hamming | ~50ns |
| k-medoids clustering | O(k * n * max_iter) | < 10ms for 200 episodes |
