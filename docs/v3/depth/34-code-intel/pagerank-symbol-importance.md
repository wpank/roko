# PageRank for Symbol Importance

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 8

---

## The Algorithm

Not all symbols in a codebase are equally important. A core `Signal` struct
imported by 30 modules matters more than a helper function used in one test
file. An agent working on a task needs to know which symbols are structural
pillars and which are peripheral -- this knowledge determines how to
allocate its limited context budget.

PageRank (Page, Brin, Motwani, and Winograd 1999) provides a principled
answer. Originally designed to rank web pages by link structure, the
algorithm transfers naturally to code dependency graphs: symbols imported by
many other symbols receive high scores, and symbols imported by high-scoring
symbols receive even higher scores.

### Standard PageRank

```
PR(v) = (1 - d) / N + d * sum(PR(u) / out_degree(u))
                        for each u that links to v
```

Where:
- `d` = damping factor (typically 0.85)
- `N` = total number of nodes
- `PR(u)` = PageRank of node `u`
- `out_degree(u)` = number of outgoing edges from `u`

### Implementation in roko-index

```rust
pub fn pagerank(
    graph: &SymbolGraph,
    iterations: u32,
    damping: f64,
) -> HashMap<SymbolId, f64>
```

Key implementation details:

1. **Initialization** -- All nodes start with equal rank `1/N`, ensuring
   total rank sums to 1.0.
2. **Iteration** -- Each iteration recomputes all ranks from the previous
   iteration's values. Geometric convergence; 20--30 iterations suffice.
3. **Out-degree floor** -- `max(1)` prevents division by zero for dangling
   nodes (nodes with no outgoing edges).
4. **Reverse edge traversal** -- Uses the pre-computed reverse adjacency
   list for efficient lookup of incoming edges.
5. **`mul_add` precision** -- Fused multiply-add for better floating-point
   precision.

### Convergence properties

| Property | Value |
|----------|-------|
| Initial rank | 1/N for all nodes |
| Damping factor (default) | 0.85 |
| Convergence rate | Geometric with rate d = 0.85 |
| Iterations for < 0.001 error | ~30 |
| Total rank (invariant) | 1.0 (within floating-point precision) |

For the Roko workspace (~5K+ nodes), 30 iterations take under 1ms.

---

## Personalized PageRank

The `personalized_pagerank()` function replaces uniform teleportation with
a biased distribution. Instead of jumping to any random node with
probability `(1-d)/N`, the random surfer teleports to task-relevant seed
nodes with higher probability:

```rust
pub fn personalized_pagerank(
    graph: &SymbolGraph,
    seeds: &[SymbolId],
    iterations: u32,
    damping: f64,
) -> HashMap<SymbolId, f64>
```

The teleportation distribution encodes task context:
- Task-mentioned symbols: `teleport(v) = 0.5 / |task_symbols|`
- All other symbols: `teleport(v) = 0.5 / (N - |task_symbols|)`

PPR can also be understood as a geometric mixture of random walk
distributions:

```
PPR(s, *) = sum_{t=0}^{inf} alpha * (1-alpha)^t * walk_t(s, *)
```

where `alpha` (teleportation probability, typically 0.15--0.20) controls
locality radius.

### Local PPR via push algorithm

For large graphs, the push algorithm (Andersen, Chung, and Lang 2006)
computes approximate PPR locally with running time proportional to the
output set, not the graph size. This is ideal for the `get_context` MCP
tool: given a few task-relevant seed symbols, compute local PPR to find
structurally relevant neighbors without scanning the entire graph.

### Topic-sensitive ranking

Pre-computing one PPR vector per concern (architecture, testing, API
surface, recent activity, current task) and interpolating at query time
gives topic-sensitive ranking without per-query PPR computation.

---

## Weighted PageRank

The `weighted_pagerank()` function incorporates edge weights for task-aware
ranking:

```rust
pub fn weighted_pagerank(
    graph: &SymbolGraph,
    iterations: u32,
    damping: f64,
    weights: &HashMap<SymbolId, f64>,
) -> HashMap<SymbolId, f64>
```

| Condition | Weight | Rationale |
|-----------|--------|-----------|
| Symbol mentioned in task prompt | 10x | Direct task relevance |
| Symbol in currently open file | 50x | Active working context |
| Recently modified symbol | 5x | Recency bias |
| Private/crate-internal symbol | 0.1x | Less cross-module relevance |
| Test file symbol | 0.5x | Tests depend on code, not reverse |
| Default | 1.0x | Baseline |

---

## Interpreting PageRank Scores for Code

### What high PageRank means

| Symbol pattern | Typical PageRank | Why |
|---------------|-----------------|-----|
| Core types (`Signal`, `Error`, `Config`) | Top 1% | Imported everywhere |
| Trait definitions (`Gate`, `Scorer`, `Router`) | Top 5% | Implemented by many types |
| Shared utilities (`parse_source`, `build_graph`) | Top 10% | Called from multiple modules |
| Entry points (`main`, `run`, `execute`) | Top 15% | High out-degree, some in-links |
| Module-internal helpers | Bottom 50% | Few external imports |
| Test utilities | Bottom 20% | Only imported by tests |
| Dead code | Bottom 5% | Zero in-links, only baseline score |

### What PageRank does NOT capture

1. **Task relevance** -- The globally most important symbol may be
   irrelevant to the current task. Personalized PageRank addresses this.
2. **Recency** -- A recently-modified symbol may be more relevant than a
   stable core type. Weighted PageRank addresses this.
3. **Semantic meaning** -- Two symbols with identical graph structure but
   different semantic roles receive the same PageRank. HDC fingerprints
   capture semantic properties that PageRank misses.
4. **Code quality** -- A God object with many imports gets high PageRank
   even though it represents a design problem.

---

## Integration with the Signal Architecture

### PageRank as Signal scoring

| PageRank output | Signal axis | How |
|----------------|-------------|-----|
| Raw PageRank score | `utility` | Higher rank = higher utility for context |
| Normalized rank (0--1) | `salience` | Rank relative to highest-ranked symbol |
| Stability across iterations | `confidence` | Stable ranks = reliably important |

### Budget-aware context allocation

```
token_budget(symbol) = total_budget * (pagerank(symbol) / sum(pagerank(included)))
```

A symbol with twice the PageRank gets twice the token budget -- more of its
surrounding code, documentation, and context is included.

### Dual-process routing

PageRank modulates cognitive investment:

- **T0 (no LLM)** -- Low-PageRank symbol modifications handled by
  pattern-matching heuristics.
- **T1 (fast model)** -- Medium-PageRank symbols get lightweight LLM
  reasoning.
- **T2 (full model)** -- High-PageRank symbols (core types, public API
  surfaces) get full reasoning with extended context.

---

## Performance

| Metric | Value |
|--------|-------|
| Algorithm | Iterative power method |
| Time complexity | O(iterations * (N + E)) |
| Space complexity | O(N) for rank vectors |
| 5K nodes, 30K edges, 30 iterations | < 1ms |
| 50K nodes, 200K edges, 30 iterations | ~10ms (estimated) |

---

## Configuration

```toml
[index.pagerank]
damping_factor = 0.85          # Standard damping. Range: 0.5..0.99.
max_iterations = 30            # For global PageRank. Range: 5..100.
convergence_tolerance = 1e-6   # Early termination threshold.
ppr_alpha = 0.15               # Personalized PageRank teleportation.
```

---

## Verified Behaviors

- Empty graph: no panics, empty rank map
- Star topology: hub gets highest rank
- Cycle topology: all nodes roughly equal (diff < 0.01)
- Personalized PageRank concentrates scores around seed nodes
- Weighted PageRank boosts task-mentioned symbols

---

## Academic Foundations

- **PageRank**: Page, Brin, Motwani, and Winograd (1999). "The PageRank
  Citation Ranking." Stanford InfoLab.
- **Topic-Sensitive PageRank**: Haveliwala (2002). *WWW*. Pre-computed biased
  PageRank per topic.
- **Local Graph Partitioning**: Andersen, Chung, and Lang (2006). *FOCS*.
  Push algorithm for approximate local PPR.
- **Code importance**: Allamanis, Barr, Bird, and Sutton (2014). *FSE*.
  Structural properties correlate with developer attention.
- **Defect prediction**: Zimmermann and Nagappan (2008). *ICSE*. Graph
  centrality predicts defect-prone modules.
- **Meta-Harness**: Lee et al. (2026). arXiv:2603.28052. Better context
  allocation improves agent performance.

---

## Cross-References

- See [dependency-graph.md](./dependency-graph.md) for the graph that
  PageRank operates on
- See [context-assembly-from-code.md](./context-assembly-from-code.md) for
  how scores drive context selection
- See [hdc-fingerprints.md](./hdc-fingerprints.md) for the complementary
  similarity metric
