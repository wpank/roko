# Depth: TraceRank Algorithm

> PageRank-style power iteration over agent interaction edges with damping,
> convergence guarantees, and normalized rank computation.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 3
**Source:** `crates/roko-chain/src/trace_rank.rs`

---

## Algorithm: Power Iteration

TraceRank uses standard PageRank power iteration with teleportation:

```
rank[B] = (1 - d) / N + d * SUM_over_A(rank[A] * weight(A->B) / out_weight(A))
```

Where:
- `d` = damping factor (0.85)
- `N` = number of agents in the graph
- `weight(A->B)` = edge weight (amount * quality)
- `out_weight(A)` = sum of all outgoing edge weights from A

### Initialization

All agents start with equal rank: `rank[i] = 1/N`.

### Convergence

The algorithm converges when the maximum rank change across all agents falls
below the convergence threshold:

```rust
let final_delta = ranks.iter()
    .zip(new_ranks.iter())
    .map(|(old, new)| (old - new).abs())
    .fold(0.0_f64, f64::max);

if final_delta < self.config.convergence_threshold {
    break;  // Converged
}
```

Default threshold: `1e-6`. Maximum iterations: 100.

**Convergence guarantee:** The matrix is stochastic (rows sum to 1 after
normalization) and aperiodic (teleportation ensures every state is reachable).
These properties guarantee convergence to a unique stationary distribution
(Perron-Frobenius theorem).

### Dangling Nodes

Agents with no outgoing edges (pure receivers) distribute their rank equally
across all agents, equivalent to teleportation:

```rust
if out_weights[from_idx] <= 0.0 {
    let share = damping * ranks[from_idx] / n as f64;
    for r in &mut new_ranks {
        *r += share;
    }
}
```

---

## Edge Construction

Edges are directed and weighted:

```rust
pub struct PaymentEdge {
    pub from: AgentId,   // Assigning agent
    pub to: AgentId,     // Executing agent
    pub amount: f64,     // Interaction value
    pub quality: f64,    // Delivery quality [0.0, 1.0]
    pub block: u64,      // Timestamp
}

impl PaymentEdge {
    pub fn weight(&self) -> f64 {
        self.amount * self.quality
    }
}
```

Edges below `min_edge_weight` (default 0.01) are filtered to exclude dust
interactions that add noise without meaningful trust signal.

### Lookback Window

When `lookback_blocks > 0`, only edges within the window are included:

```rust
let min_block = current_block.saturating_sub(self.config.lookback_blocks);
self.edges.iter().filter(|e| e.block >= min_block).collect()
```

This prevents ancient interactions from permanently anchoring rank and
ensures that the graph reflects recent trust relationships.

---

## Normalized Rank

Raw PageRank scores are probabilities summing to 1.0. For comparison purposes,
ranks are normalized to [0, 1] by dividing by the maximum:

```rust
pub fn normalized_rank(&self, result: &TraceRankResult, agent: AgentId) -> f64 {
    let max_rank = result.ranks.values().copied().fold(0.0_f64, f64::max);
    if max_rank <= 0.0 { return 0.0; }
    result.ranks.get(&agent).copied().unwrap_or(0.0) / max_rank
}
```

The highest-ranked agent always has normalized rank 1.0.

---

## Blend with EMA

TraceRank supplements direct EMA reputation:

```rust
pub fn blend_reputation(&self, ema_score: f64, trace_rank_score: f64) -> f64 {
    let w = self.config.blend_weight.clamp(0.0, 1.0);
    (1.0 - w) * ema_score + w * trace_rank_score
}
```

Default `blend_weight = 0.3`:
- `effective = 0.7 * ema + 0.3 * trace_rank`

**Rationale:** Direct observations (EMA) should dominate because they reflect
actual work quality. Graph structure (TraceRank) supplements by capturing
transitive trust that local observations cannot -- for example, an agent
trusted by many well-reputed principals deserves credit even if the local
observer has not interacted with it directly.

---

## Sybil Resistance Properties

TraceRank provides structural sybil resistance:

1. **No free edges:** Creating fake agents does not help unless they have real
   interaction flow with weight above the dust threshold.
2. **Quality gating:** Low-quality interactions produce low edge weights,
   limiting trust propagation.
3. **Damping limits manipulation:** At most 85% of rank propagates through any
   edge; 15% distributes uniformly. An attacker controlling a subgraph cannot
   concentrate more than `d^k` of the total rank through `k` hops.
4. **Lookback window:** Old manufactured edges expire, requiring continuous
   legitimate activity to maintain rank.
