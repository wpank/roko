# Depth: Sybil Detection

> PersonalizedPageRank trust propagation, SybilRank random walks from
> trusted seeds, and graph-based collusion ring detection via internal
> edge density.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 6
**Source:** `crates/roko-chain/src/identity_economy_identity.rs`

---

## PersonalizedPageRank

Unlike standard PageRank (where teleportation distributes uniformly across all
nodes), Personalized PageRank concentrates teleportation on a known-good seed
set. Agents far from trusted seeds in graph distance receive negligible trust.

### Parameters

```rust
pub struct PersonalizedPageRank {
    pub alpha: f64,             // Teleport probability (default 0.15)
    pub seed_set: Vec<AgentId>, // Known-good agents
    pub max_iterations: u32,    // Iteration limit
    pub epsilon: f64,           // Convergence threshold
}
```

### Algorithm

```
For each iteration:
  For each node v:
    seed_component = alpha * (1 / |seed_set|)  if v is in seed_set, else 0
    neighbor_sum = SUM over in-edges (u -> v):
        score[u] * weight(u, v) / out_weight(u)
    new_score[v] = seed_component + (1 - alpha) * neighbor_sum

  Check convergence: max|new_score[v] - score[v]| < epsilon
```

### Properties

- **Seed-biased:** Only seed nodes receive teleportation mass. Non-seed agents
  must earn trust transitively through connections to seeds.
- **Distance decay:** Trust decays exponentially with graph distance from seeds.
  At `alpha = 0.15`, the probability of reaching a node `k` hops away decreases
  as `(1 - alpha)^k = 0.85^k`:
  - 1 hop: 0.85
  - 2 hops: 0.72
  - 3 hops: 0.61
  - 5 hops: 0.44
  - 10 hops: 0.20

- **Attack cost:** A sybil attacker must create many high-quality edges from
  sybil nodes to real trusted nodes. Low-quality edges provide negligible
  trust propagation due to quality-weighted edge weights.

---

## SybilRank

SybilRank propagates a fixed trust budget from trusted seeds for a bounded
number of random walk steps, then flags agents below a threshold.

### Parameters

```rust
pub struct SybilRankDetector {
    pub walk_length: u32,       // Recommended: O(log n)
    pub trust_seed: Vec<AgentId>,
    pub threshold: f64,
}
```

### Algorithm

1. **Initialize:** Seed nodes receive `1 / |seed_set|` trust. All other nodes
   receive 0 trust.

2. **Propagate:** For `walk_length` steps, propagate trust along weighted edges:
   ```
   new_trust[to] += trust[from] * weight(from, to) / out_weight(from)
   ```

3. **Flag:** Nodes whose final trust is below `threshold` are flagged as
   potential sybils.

### Difference from PersonalizedPageRank

| Property | PersonalizedPageRank | SybilRank |
|---|---|---|
| Teleportation | Continuous (every iteration) | None after initialization |
| Walk length | Until convergence | Fixed (`walk_length` steps) |
| Steady state | Converges to unique distribution | Final trust depends on walk length |
| Trust budget | Renews via teleportation | Conserved (distributes but does not create) |

SybilRank is simpler and faster (bounded iterations) but less precise.
PersonalizedPageRank produces a more stable ranking but requires convergence.
Using both provides defense in depth.

---

## Cluster-Based Collusion Detection

After identifying suspicious clusters via SybilRank, a structural check
distinguishes coordinated sybil groups from legitimate communities:

```rust
pub struct SybilCluster {
    pub members: Vec<AgentId>,
    pub internal_edge_density: f64,
    pub external_edge_count: u32,
    pub estimated_sybil_probability: f64,
}

pub fn detect_collusion_rings(clusters: &[SybilCluster]) -> Vec<&SybilCluster> {
    clusters.iter()
        .filter(|c| c.internal_edge_density > 0.5
                  && (c.external_edge_count as usize) < c.members.len() * 2)
        .collect()
}
```

### Detection Criteria

A cluster is flagged as a collusion ring when:

1. **High internal density** (`> 0.5`): More than half of all possible
   internal edges exist. Members interact with each other much more than
   chance would predict.

2. **Low external connectivity** (`< 2 * members`): The cluster has fewer
   external edges than twice its member count. Legitimate communities have
   extensive external connections; sybil groups are insular.

### False Positive Mitigation

Both criteria must hold simultaneously. A legitimate team might have high
internal density (they work together) but also high external connectivity
(they work with many others). The dual criterion reduces false positives.

---

## Interaction Graph

Both algorithms operate over the same graph:

```rust
pub struct InteractionGraph {
    pub nodes: Vec<AgentId>,
    pub edges: Vec<(AgentId, AgentId, f64)>,
}
```

Edge weights aggregate multiple interaction types:
- Job assignments (primary signal)
- Knowledge validations
- Group memberships
- Delegation relationships

Higher-weight edges indicate stronger, more frequent interaction and
propagate more trust.
