# Predictive Foraging: Marginal Value Theorem for Context Search

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/prompt.rs` -- MultiPatchForager
> v1 source: `docs/v1/03-composition/09-predictive-foraging-mvt.md`
> Primary citations: Charnov (1976), Pirolli and Card (1999)

---

## Overview

Predictive Foraging applies the Marginal Value Theorem (MVT) from behavioral
ecology to the problem of when to stop searching for context. An agent
searching for relevant knowledge faces a diminishing returns curve: each
additional search iteration finds less relevant content than the last. MVT
provides the optimal stopping rule: stop searching when the marginal relevance
of the next result drops below the average relevance gained per unit cost so
far.

---

## 1. The Foraging Problem

Context assembly is an information foraging problem (Pirolli and Card 1999).
The agent must decide how long to search before starting work. Searching longer
finds more context but delays execution and risks including low-quality content
that triggers context rot.

- **Search too little:** Agent misses critical context and fails.
- **Search too much:** Agent drowns in marginal context, wastes budget, may
  perform worse due to the "sufficient context" effect (Joren et al., ICLR 2025).

---

## 2. The MVT Formula

Eric Charnov (1976) formalized the optimal foraging strategy for an animal
exploiting patchy food resources:

### The Stopping Rule

```
Stop when:  relevance(last_result) / cost(last_search)
         <= total_gain / total_cost
```

Where:
- `relevance(last_result)` -- composite score of the most recently retrieved
  context chunk
- `cost(last_search)` -- cost (time and tokens) of the last search operation
- `total_gain` -- cumulative relevance of all retrieved chunks
- `total_cost` -- cumulative cost of all search operations

When the marginal gain-to-cost ratio (left side) drops below the average
gain-to-cost ratio (right side), further searching is suboptimal.

### The Exponential Gain Curve

Context relevance follows diminishing returns modeled as an exponential:

```
g(k) = G_max * (1 - exp(-lambda * k))
```

Where:
- `g(k)` -- cumulative relevance after k search iterations
- `G_max` -- maximum achievable relevance (asymptotic limit)
- `lambda` -- rate parameter (how quickly the curve saturates)
- `k` -- number of search iterations

The marginal gain at step k:

```
g'(k) = G_max * lambda * exp(-lambda * k)
```

Marginal gain decreases exponentially. The first few results are highly
relevant; subsequent results provide rapidly diminishing value.

### Optimal Stopping Point

Setting marginal gain equal to average gain rate:

```
g'(k*) = g(k*) / k*

G_max * lambda * exp(-lambda * k*) = G_max * (1 - exp(-lambda * k*)) / k*
```

This transcendental equation has no closed-form solution but is easily solved
numerically. For typical values (G_max = 1.0, lambda = 0.3), the optimal
stopping point is k* approximately 5-8 iterations.

---

## 3. Multi-Patch Foraging

The basic MVT governs when to stop within a single source. Multi-patch
foraging addresses the higher-level decision: when to switch between sources
and in what order to visit them.

```rust
pub struct MultiPatchForager {
    pub source_params: HashMap<ContextSource, (f64, f64)>,  // (G_max, lambda)
    pub travel_costs: HashMap<ContextSource, f64>,
    pub environment_rate: f64,
}

impl MultiPatchForager {
    pub fn optimal_order(&self) -> Vec<ContextSource> {
        // Sort by expected initial gain: G_max * lambda
        // Visit highest-gain source first
    }

    pub fn should_visit(&self, source: &ContextSource) -> bool {
        let initial_gain = self.expected_initial_gain(source);
        let travel_cost = self.travel_costs[source];
        initial_gain > self.environment_rate * travel_cost
    }

    pub fn optimal_iterations(&self, source: &ContextSource) -> usize {
        // Solve: g'(k*) = environment_rate + travel_cost / k*
        // Binary search for numerical solution
        // Clamp to [1, 10]
    }
}
```

### Source Characteristics

| Source | G_max | lambda | Travel Cost | Typical Iterations |
|---|---|---|---|---|
| Knowledge Store | 0.9 | 0.25 | Low (in-memory) | 5-8 |
| Episode Store | 0.6 | 0.4 | Low (in-memory) | 3-5 |
| File Context | 0.8 | 0.5 | Medium (disk I/O) | 2-4 |
| Signal Log | 0.4 | 0.6 | Low (in-memory) | 1-3 |

Knowledge store has highest G_max (most potential value) but saturates slowly.
Signal log saturates quickly -- first few signals are most relevant.

---

## 4. Per-Category Calibration

MVT parameters are calibrated from historical task outcomes:

```rust
fn calibrate_mvt(episodes: &[Episode], task_category: &str) -> (f64, f64) {
    // For each episode: record (k, cumulative_relevance) pairs
    // Fit exponential curve: g(k) = G_max * (1 - exp(-lambda * k))
    // Return (G_max, lambda)
}
```

Different task categories have different gain curves:
- **Simple rename:** lambda ~= 0.8 (saturates quickly, few results needed)
- **Cross-crate integration:** lambda ~= 0.15 (saturates slowly, many valuable)
- **Bug fix:** lambda ~= 0.4 (moderate saturation)

### Feedback Loop

```
Task outcome -> recorded in episode -> calibration updates (G_max, lambda)
-> next search uses updated parameters
```

Tasks that succeeded with fewer iterations push lambda higher (faster
saturation, earlier stopping). Tasks that failed with too few results push
lambda lower (slower saturation, later stopping).

---

## 5. Social Foraging

In multi-agent execution (parallel plan with 5-20 agents), social foraging
leverages collective retrieval patterns:

### Stigmergic Retrieval Signals

```rust
pub struct RetrievalSignal {
    pub task_category: String,
    pub entry_id: String,
    pub relevance: f64,
    pub gate_passed: bool,
    pub timestamp: Timestamp,
    pub agent_id: String,
}
```

Agents deposit signals after successful completion. Agent B's forager uses
Agent A's successful retrievals as "social information scent" -- boosting
scores of entries useful to similar agents.

Social information helps when resources are heterogeneously distributed
(clustered by topic), which matches knowledge stores. It helps less with
uniform distributions or high agent diversity.

---

## 6. Sufficient Context as Foraging Criterion

The "Sufficient Context" framework (Harel-Canada et al., ICLR 2025) provides
a formal criterion: a retrieved set is **sufficient** if a diligent reader
could answer the question from it alone.

```rust
pub fn should_stop_searching(
    mvt_ratio: f64,
    sufficiency: f64,
    sufficiency_threshold: f64,  // default: 0.85
) -> bool {
    mvt_ratio <= 1.0 || sufficiency >= sufficiency_threshold
}
```

---

## 7. Relation to Active Inference

MVT and active inference are complementary:
- **Active inference** decides WHAT to include (scoring function)
- **MVT** decides WHEN to stop searching (stopping rule)

In the pipeline:
- Stage 1 (Query) uses **MVT** to decide how many candidates to retrieve
- Stage 2 (Score) uses **active inference** to rank them

---

## 8. Academic Foundations

- **Charnov, E. L. (1976).** "Optimal Foraging: The Marginal Value Theorem."
  Theoretical Population Biology, 9(2), 129-136. The foundational paper.

- **Pirolli, P. and Card, S. K. (1999).** "Information Foraging."
  Psychological Review, 106(4), 643-675. Applied foraging to information.

- **Hills, T. T., Jones, M. N., Todd, P. M. (2012).** "Optimal Foraging in
  Semantic Memory." Psychological Review, 119(2), 431-440. Humans follow MVT
  in verbal fluency tasks.

- **Lacosse et al. (2026).** arXiv:2603.01822. LLMs exhibit the same
  convergent/divergent foraging patterns as humans. Foraging is steerable.

- **arXiv:2511.12759 (2025).** Well-organized embedding geometry is sufficient
  for near-optimal foraging without explicit MVT.

- **Harel-Canada et al. (2025).** "Sufficient Context." ICLR 2025. Formalizes
  when a retrieved context set is sufficient.

- **arXiv:2410.05983 (2025).** ICLR 2025. Increasing retrieved passages
  doesn't consistently improve performance -- confirms MVT.

- **Science (2025).** doi:10.1126/science.ady1055. First field-scale
  validation of social MVT in hunter-gatherers.

---

## 9. Implementation Status

| Aspect | Status |
|--------|--------|
| MVT formula specified | **Specified** |
| Exponential gain curve model | **Specified** |
| MultiPatchForager struct | **Shipped** (in PromptComposer) |
| Context assembler gather loop | **Shipped** |
| PF utility in scoring (pf_utility) | **Designed** (defaults to 0) |
| Per-category calibration | **Not yet** |
| Social foraging signals | **Designed** |
| Sufficient context integration | **Designed** |

---

## Cross-References

- [active-inference-context-selection.md](active-inference-context-selection.md) -- WHAT to include
- [5-stage-assembly-pipeline.md](5-stage-assembly-pipeline.md) -- Pipeline
- [vcg-attention-auction.md](vcg-attention-auction.md) -- Alternative allocation
- [token-budget-management.md](token-budget-management.md) -- Budget prediction
- [affect-modulated-retrieval.md](affect-modulated-retrieval.md) -- Affect modulation
- `crates/roko-compose/src/prompt.rs` -- MultiPatchForager implementation
