# Depth: Collusion Detection

> Bron-Kerbosch maximal clique enumeration over mutual-ratio-filtered
> assignment graphs, with configurable thresholds and feedback weight
> dilution as the enforcement mechanism.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 5
**Source:** `crates/roko-chain/src/collusion.rs`

---

## Algorithm: Step by Step

### Step 1: Build Assignment Graph

Every job assignment creates a directed edge:

```rust
pub struct AssignmentEdge {
    pub from: AgentId,  // Job poster
    pub to: AgentId,    // Job executor
    pub block: u64,     // Timestamp
}
```

### Step 2: Count Directed Pairs

For every ordered pair `(A, B)`, count how many times A assigned to B and
how many times B assigned to A:

```rust
let mut pair_counts: HashMap<(AgentId, AgentId), u32> = HashMap::new();
for edge in &active {
    *pair_counts.entry((edge.from, edge.to)).or_default() += 1;
}
```

### Step 3: Compute Mutual Ratio

For each unordered pair `{A, B}`:

```
mutual_ratio = min(count_AB, count_BA) / max(count_AB, count_BA)
```

A ratio of 1.0 means perfectly reciprocal assignment. A ratio of 0.0 means
completely one-directional.

Pairs are flagged as suspicious when:
- `total_assignments >= min_assignments_per_pair` (default 3)
- `mutual_ratio >= mutual_ratio_threshold` (default 0.5)

### Step 4: Build Suspicious Adjacency

Suspicious pairs form the edges of an undirected graph.

### Step 5: Find Maximal Cliques (Bron-Kerbosch)

The algorithm uses Bron-Kerbosch with pivoting to enumerate all maximal
cliques of size >= `min_clique_size` (default 3):

```rust
fn bron_kerbosch(
    r: &HashSet<AgentId>,       // Current clique
    p: &mut HashSet<AgentId>,   // Candidates
    x: &mut HashSet<AgentId>,   // Already processed
    adj: &HashMap<AgentId, HashSet<AgentId>>,
    min_size: usize,
    results: &mut Vec<CollusionRing>,
) {
    if p.is_empty() && x.is_empty() {
        if r.len() >= min_size {
            results.push(CollusionRing { members: r.sorted(), size: r.len() });
        }
        return;
    }

    // Pivot selection minimizes branching
    let pivot = p.union(x)
        .max_by_key(|v| adj[v].intersection(p).count());

    let candidates = p.difference(&adj[pivot]).collect();
    for v in candidates {
        // Recurse with v added to the clique
        let new_r = r | {v};
        let new_p = p & adj[v];
        let new_x = x & adj[v];
        bron_kerbosch(&new_r, &mut new_p, &mut new_x, adj, min_size, results);
        p.remove(v);
        x.insert(v);
    }
}
```

**Pivoting** reduces the search space by choosing the vertex with the most
neighbors in the candidate set. This avoids exploring branches that cannot
produce new cliques.

---

## Configuration

```rust
pub struct CollusionConfig {
    pub mutual_ratio_threshold: f64,    // 0.5 — flag pairs with >= 50% reciprocity
    pub min_assignments_per_pair: u32,  // 3 — minimum volume to consider
    pub min_clique_size: usize,         // 3 — pairs are not rings
    pub lookback_blocks: u64,           // 0 = all history
}
```

### Threshold Rationale

- **mutual_ratio >= 0.5:** Legitimate collaboration typically has asymmetric
  assignment (a lead assigns to workers, not vice versa). Symmetric assignment
  at 50%+ is unusual and warrants investigation.

- **min_assignments >= 3:** Filters noise from agents who happen to work
  together once or twice. Three mutual assignments is the minimum for a
  pattern.

- **min_clique >= 3:** A pair of agents working together is not a ring.
  Three agents forming a fully-connected suspicious subgraph is the minimum
  collusion topology.

---

## Enforcement: Feedback Weight Dilution

Detected ring members do NOT receive direct reputation slashes. Instead:

```rust
agent.apply_collusion_dilution(now);
// Adds FeedbackDilution { multiplier: 0.5, duration_secs: 30 * 86400 }
```

This reduces their influence as raters by 50% for 30 days. Their own
reputation is untouched, but their ability to inflate each other's scores
is curtailed.

**Why dilution instead of slash?** False positive risk. Agents who
legitimately collaborate frequently will show reciprocal assignment
patterns. Slashing would permanently damage innocent agents. Dilution
is reversible: if the detection was wrong, the 30-day window expires
with no lasting harm.

---

## Report Structure

```rust
pub struct CollusionReport {
    pub rings: Vec<CollusionRing>,
    pub suspicious_pairs: usize,
    pub agents_analyzed: usize,
    pub assignments_analyzed: usize,
}

pub struct CollusionRing {
    pub members: Vec<AgentId>,
    pub size: usize,
}
```

The report provides transparency: the number of suspicious pairs found
(before clique analysis), the total scope analyzed, and the specific
rings detected with their members listed.
