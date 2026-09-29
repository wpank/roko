# 8-Dimensional Strategy Space

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/08-8-dimensional-strategy-space.md`

---

## Overview

The somatic landscape (see `somatic-markers-damasio.md`) is a k-d tree over an
8-dimensional strategy space. The 8 dimensions are **domain-configurable axes**
that characterize a strategy attempt in terms meaningful to the agent's domain.
A coding agent's dimensions (Complexity, Risk, Novelty, Confidence, Time Pressure,
Scope, Reversibility, Dependency Depth) differ from a chain agent's dimensions
(Volatility, Liquidity, Correlation, Leverage, Time Horizon, Concentration,
Counterparty Risk, Regulatory Exposure).

The 8D strategy space serves three purposes:

1. **Somatic marker storage**: every marker in the k-d tree has coordinates in this
   space, enabling nearest-neighbor queries that find emotionally similar past
   strategies.
2. **Strategy classification**: the coordinates provide a compact feature vector
   for classifying strategy types, enabling the agent to recognize familiar vs.
   unfamiliar territory.
3. **Cross-agent transfer**: agents with the same domain configuration can share
   somatic markers because their coordinates are commensurable.

---

## Why 8 Dimensions?

### Dimensionality Trade-offs

The choice of 8 dimensions balances three constraints:

**Expressiveness**: Too few dimensions conflate distinct strategy types. With 3
dimensions, "high-complexity, low-risk, novel" and "high-complexity, low-risk,
familiar" would be indistinguishable -- but they require very different approaches.

**Computational efficiency**: k-d tree performance degrades with dimensionality.
For D dimensions and N points, nearest-neighbor search is efficient when N >> 2^D.
At D=8, this requires N >> 256 markers -- easily achievable. At D=16, N >> 65,536
would be needed, which is impractical for early-stage agents.

**Human interpretability**: An operator examining a somatic marker at coordinates
[0.8, 0.3, 0.9, 0.2, 0.7, 0.4, 0.1, 0.6] should understand what kind of strategy
it represents. With 8 named dimensions, each coordinate is meaningful. With 32
dimensions, interpretation becomes impractical.

### Alternative Dimensionalities

| Dimensions | Pros | Cons | Decision |
|---|---|---|---|
| 3 (PAD only) | Fast, simple | Cannot distinguish same-emotion strategies | Rejected |
| 5 (Big Five analog) | Psychology-backed | Not enough for domain features | Rejected |
| **8** | **Good k-d tree performance, interpretable** | **Requires domain config** | **Selected** |
| 16 | Very expressive | k-d tree degradation, hard to populate | Rejected |
| Arbitrary (HDC) | Maximum flexibility | No spatial structure for k-d tree | Used elsewhere |

---

## Coding Agent Dimensions

For agents working on software engineering tasks, the 8 dimensions are:

### Dimension 1: Complexity [0, 1]

Captures how structurally difficult the change is.

| Low (0.0) | High (1.0) |
|---|---|
| Simple change (rename, formatting) | Multi-file refactor, architecture change |

**Measurement source**: Cyclomatic complexity of affected code, file count.

**Extraction algorithm**:

```rust
fn compute_complexity(&self, action: &Action, context: &Context) -> f64 {
    let cc = if let Some(index) = context.code_index() {
        let fns = index.functions_in_files(&action.affected_files);
        fns.iter().map(|f| f.cyclomatic_complexity as f64)
            .sum::<f64>() / fns.len().max(1) as f64
    } else {
        estimate_complexity_from_description(&action.description)
    };
    let file_count = action.affected_files.len() as f64;
    let cc_norm = sigmoid_normalize(cc, 5.0, 2.0);
    let files_norm = sigmoid_normalize(file_count, 5.0, 1.5);
    0.6 * cc_norm + 0.4 * files_norm
}
```

The sigmoid normalization function maps [0, infinity) to [0, 1] with a configurable
midpoint:

```rust
fn sigmoid_normalize(x: f64, midpoint: f64, steepness: f64) -> f64 {
    1.0 / (1.0 + (-steepness * (x - midpoint)).exp())
}
```

**Heuristic fallback** (when no code index is available):

| Keyword pattern | Estimated CC |
|---|---|
| "rename", "typo", "comment" | 1-2 |
| "add test", "add field" | 3-5 |
| "refactor", "wire module" | 6-10 |
| "rewrite", "migrate", "new crate" | 10-20 |

### Dimension 2: Risk [0, 1]

Combines test coverage, gate strictness, and dependency criticality.

| Low (0.0) | High (1.0) |
|---|---|
| Test-covered, well-understood code | Untested, critical path, safety-sensitive |

**Extraction algorithm**:

```rust
fn compute_risk(&self, action: &Action, context: &Context) -> f64 {
    let coverage_risk = 1.0 - context.test_coverage_for_files(&action.affected_files)
        .unwrap_or(0.0);
    let rung_risk = context.gate_rung_for_task(&action.task_id) as f64 / 5.0;
    let dep_risk = sigmoid_normalize(
        context.reverse_dependency_count(&action.affected_files) as f64, 5.0, 1.0);
    0.50 * coverage_risk + 0.25 * rung_risk + 0.25 * dep_risk
}
```

Gate rung quantization: the 6-rung gate pipeline (rungs 0-5) maps linearly to
[0, 1]. Rung 0 (advisory) produces risk 0.0. Rung 5 (strict) produces risk 1.0.

### Dimension 3: Novelty [0, 1]

The inverse of similarity to known patterns.

| Low (0.0) | High (1.0) |
|---|---|
| Familiar crate/module, repeat task | New crate, new API, first encounter |

**Extraction algorithm**:

```rust
fn compute_novelty(&self, action: &Action, context: &Context) -> f64 {
    let embedding = context.embed_task_description(&action.description);
    match context.neuro_store().nearest_playbook(&embedding) {
        Some(m) => 1.0 - m.similarity,
        None => 1.0,  // no playbooks -- maximum novelty
    }
}
```

The query searches the entire NeuroStore history. A task matching a playbook from
1,000 episodes ago has low novelty even if nothing similar has occurred recently.
This is intentional -- somatic markers from that old playbook should still fire.

### Dimension 4: Confidence [0, 1]

Read directly from the Daimon's `AffectState.confidence` field, which aggregates
gate outcomes and task results.

| Low (0.0) | High (1.0) |
|---|---|
| Low Daimon confidence, unfamiliar territory | High confidence, proven approach |

When per-crate confidence is available (see `coding-agent-integration.md`), the
crate-specific value is used instead of the global value.

### Dimension 5: Time Pressure [0, 1]

| Low (0.0) | High (1.0) |
|---|---|
| No deadline, background task | Imminent deadline, blocking other tasks |

Computed from deadline proximity, blocker count, and queue wait time.

### Dimension 6: Scope [0, 1]

| Low (0.0) | High (1.0) |
|---|---|
| Single function, localized change | System-wide change, public API modification |

Estimated from lines changed, file count, and symbol graph extent.

### Dimension 7: Reversibility [0, 1]

Analyzes the diff to determine how easily a change can be undone.

| Low (0.0) | High (1.0) |
|---|---|
| Hard to revert (schema migration, data loss) | Easily reverted (new file, additive) |

**Extraction algorithm**:

```rust
fn compute_reversibility(&self, action: &Action, context: &Context) -> f64 {
    let diff = context.estimated_diff(action);
    let mut reversibility = 1.0;
    reversibility -= 0.3 * (diff.deleted_files.len() as f64).min(1.0);
    if diff.touches_migration_files { reversibility -= 0.4; }
    let pub_changes = diff.public_api_changes.len() as f64;
    reversibility -= 0.1 * pub_changes.min(3.0);
    if diff.is_purely_additive() { reversibility = reversibility.max(0.85); }
    reversibility.clamp(0.0, 1.0)
}
```

Reversibility is computed from the *estimated* diff (before the task executes), not
the actual diff.

### Dimension 8: Dependency Depth [0, 1]

| Low (0.0) | High (1.0) |
|---|---|
| Leaf module, no dependents | Core library, many reverse deps |

Queries the dependency graph for reverse dependency count.

---

## Example Strategy Coordinates (Coding)

| Task | Cplx | Risk | Nov | Conf | Time | Scope | Rev | Deps |
|---|---|---|---|---|---|---|---|---|
| Fix typo in comment | 0.05 | 0.01 | 0.02 | 0.95 | 0.10 | 0.02 | 0.99 | 0.01 |
| Add unit test | 0.15 | 0.10 | 0.20 | 0.80 | 0.20 | 0.10 | 0.95 | 0.05 |
| Wire existing module | 0.40 | 0.30 | 0.40 | 0.60 | 0.50 | 0.35 | 0.70 | 0.40 |
| Refactor error handling | 0.70 | 0.60 | 0.30 | 0.50 | 0.30 | 0.65 | 0.40 | 0.70 |
| New crate from scratch | 0.80 | 0.40 | 0.90 | 0.35 | 0.40 | 0.80 | 0.80 | 0.20 |
| Migrate database schema | 0.85 | 0.90 | 0.50 | 0.30 | 0.70 | 0.90 | 0.05 | 0.85 |

The Euclidean distance between "Fix typo" and "Migrate database schema" is
approximately 2.1 -- far apart. A somatic marker from a migration would not fire
for a typo fix. But "Wire module" and "Refactor error handling" are closer
(distance ~0.8), so somatic markers can transfer.

---

## Chain Agent Dimensions

For agents working in DeFi/blockchain domains:

| # | Dimension | Low (0.0) | High (1.0) | Source |
|---|---|---|---|---|
| 1 | **Volatility** | Stable market | Rapid price changes | Price delta |
| 2 | **Liquidity** | Deep pools | Thin markets, high slippage | Pool depth |
| 3 | **Correlation** | Assets independent | High cross-asset correlation | Correlation matrix |
| 4 | **Leverage** | No leverage, spot | High leverage, liquidation risk | Collateral ratio |
| 5 | **Time Horizon** | Short-term (< 1h) | Long-term (> 1 week) | Position duration |
| 6 | **Concentration** | Diversified | Single asset/pool | Herfindahl index |
| 7 | **Counterparty Risk** | Trustless protocol | Bridge, CEX dependency | Audit status |
| 8 | **Regulatory Exposure** | Clearly unregulated | Potentially regulated | Jurisdiction analysis |

---

## Domain Registration

New domains register dimension definitions at configuration time:

```toml
[daimon.strategy_space]
domain = "coding"
dimensions = [
    { name = "complexity", source = "task_analysis" },
    { name = "risk", source = "coverage_and_gates" },
    { name = "novelty", source = "neuro_similarity" },
    { name = "confidence", source = "daimon_state" },
    { name = "time_pressure", source = "scheduler" },
    { name = "scope", source = "diff_analysis" },
    { name = "reversibility", source = "diff_analysis" },
    { name = "dependency_depth", source = "dep_graph" },
]
```

```rust
pub struct DomainRegistration {
    pub name: String,
    pub dimensions: [DimensionDef; 8],
}

pub struct DimensionDef {
    pub name: String,
    pub source: DimensionSource,
    pub weight: f64,  // default: 1.0
}

pub enum DimensionSource {
    TaskAnalysis,
    CoverageAndGates,
    NeuroSimilarity,
    DaimonState,
    Scheduler,
    DiffAnalysis,
    DepGraph,
    Custom(String),  // named extraction function
}
```

---

## Dimension Weighting

Each dimension carries a different weight for distance calculations:

```rust
pub struct DimensionWeights {
    pub weights: [f64; 8],  // default: [1.0; 8]
}

impl DimensionWeights {
    pub fn apply(&self, coords: &[f64; 8]) -> [f64; 8] {
        let mut weighted = [0.0; 8];
        for i in 0..8 {
            weighted[i] = coords[i] * self.weights[i].sqrt();
        }
        weighted
    }
}
```

The `sqrt` on the weight is necessary because the distance function squares the
coordinate difference. Applying `sqrt(w)` to each coordinate before squaring
produces `w * (a_i - b_i)^2` in the final distance.

```toml
[daimon.strategy_space]
dimension_weights = [1.0, 1.5, 1.2, 1.0, 0.8, 1.0, 1.3, 0.7]
# Risk (1.5) and Reversibility (1.3) weighted higher
# Time Pressure (0.8) and Dependency Depth (0.7) weighted lower
```

**Why weighted sum, not max?** The max operator would make a single extreme
dimension dominate, ignoring all others. A task that is high-risk but
low-complexity would look identical to a high-risk high-complexity task. The
weighted sum preserves every dimension's contribution.

---

## Resource Pressure Scalar

When budget is low, strategy coordinates are compressed toward the conservative
region:

```rust
pub struct ResourcePressure {
    pub token_budget_remaining: f64,   // [0, 1]
    pub time_budget_remaining: f64,    // [0, 1]
}

impl ResourcePressure {
    pub fn scalar(&self) -> f64 {
        self.token_budget_remaining.min(self.time_budget_remaining).sqrt().clamp(0.0, 1.0)
    }

    pub fn apply(&self, coords: &[f64; 8]) -> [f64; 8] {
        let s = self.scalar();
        let mut compressed = [0.0; 8];
        for i in 0..8 {
            compressed[i] = s * coords[i] + (1.0 - s) * 0.5;
        }
        compressed
    }
}
```

At 100% budget: scalar = 1.0 (no compression). At 25% budget: scalar = 0.5. At
6.25% budget: scalar = 0.25. At 0%: maximum compression toward the midpoint.

**Effect on somatic lookup**: under pressure, all coordinates compress toward the
center, causing k-d tree queries to hit markers from cautious, well-understood
tasks. The agent reverts to proven approaches when budget is low.

**Processing pipeline**:

```
compute_coords(action, context)     // raw 8D coordinates
    -> DimensionWeights::apply()    // weighted coordinates
    -> ResourcePressure::apply()    // compressed if under budget
    -> SomaticLandscape::query()    // k-d tree lookup
```

---

## Cross-Domain Transfer

### Structural Analogy via Dimension Mapping

Agents from different domains have incompatible dimension semantics (coding's
"Complexity" is not chain's "Volatility"). But structural patterns transfer through
role mapping:

```
Coding:  high_complexity + high_risk + low_confidence   -> cautious approach
Chain:   high_volatility + high_leverage + low_confidence -> cautious approach

Both map to: high_dim_1 + high_dim_2 + low_dim_4
```

The `StrategyTransferMapper` maps dimensions by behavioral role:

| Role | Coding Dimension | Chain Dimension |
|---|---|---|
| difficulty | Complexity | Volatility |
| danger | Risk | Leverage |
| familiarity | Novelty | Correlation |
| self_assessment | Confidence | Confidence |
| urgency | Time Pressure | Time Horizon |
| breadth | Scope | Concentration |
| recoverability | Reversibility | Counterparty Risk |
| coupling | Dependency Depth | Regulatory Exposure |

```rust
pub struct StrategyTransferMapper {
    dimension_map: [(usize, usize); 8],
}

impl StrategyTransferMapper {
    pub fn transfer(&self, source_coords: &[f64; 8]) -> [f64; 8] {
        let mut target = [0.5; 8]; // default midpoint
        for &(src, tgt) in &self.dimension_map {
            target[tgt] = source_coords[src];
        }
        target
    }
}
```

The mapping preserves the shape of the strategy profile (which dimensions are
high/low) without preserving specific meanings. This enables cross-domain somatic
transfer at a coarse level.

---

## Error Handling

| Error | Cause | Response |
|---|---|---|
| Code index unavailable | `roko-index` not built | Keyword heuristic for Complexity |
| Test coverage missing | No coverage report | Assume 0% (maximum risk) |
| NeuroStore empty | New agent | Novelty = 1.0 for all tasks |
| Dependency graph unavailable | Workspace not resolved | Dependency Depth = 0.5 |
| Budget data missing | No budget configured | Pressure scalar = 1.0 |
| NaN in dimension | Buggy extractor | Clamp to 0.5, log warning |

---

## Implementation Status

`roko-daimon` owns the persisted `StrategySpaceDefinition` and the coordinate
projection layer. `roko.toml` parses `[daimon.strategy_space]`, `DaimonState`
persists the selected definition, and a `StrategySpaceComputer` abstraction
projects normalized task observations into the 8D space. The built-in coding
extractor is used for live orchestration and dream replay. Non-coding domains use
a role-aware projection over their configured labels.

Remaining work: dedicated native extractors for non-coding domains, richer
cross-domain mapping logic, and tighter integration with the VCG market.

---

## Academic Foundations

- Damasio, A.R. (1994). *Descartes' Error*. Putnam.
- Bechara, A. et al. (1997). "Deciding advantageously before knowing the
  advantageous strategy." *Science*, 275(5304), 1293-1295.
- Kleyko, D. et al. (2022). "Vector Symbolic Architectures as a Computing
  Framework." *ACM Computing Surveys*, 55(13s), 1-55.

---

## Cross-References

- `somatic-markers-damasio.md` -- k-d tree query over strategy space
- `15-percent-contrarian-retrieval.md` -- contrarian retrieval in strategy space
- `coding-agent-integration.md` -- coding-specific dimension computation
- `integration-points.md` -- strategy space as integration point
