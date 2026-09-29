# 08-learning/11 -- Pareto Frontier Pruning

> Two-objective dominance check over (pass_rate, cost_per_success), frontier
> evolution, integration with the cascade router bandit arm set, and edge
> case handling for cold start and provider updates.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/pareto.rs`

**Cross-references:** [bandits-ucb-thompson-linucb](bandits-ucb-thompson-linucb.md),
[cascade-router](cascade-router.md),
[cost-normalization](cost-normalization.md)

---

## 1. Purpose

Pareto frontier pruning identifies which models are non-dominated with respect
to two objectives: pass rate and cost per successful task. A model is
Pareto-optimal if no other model has both a higher pass rate and a lower cost
per successful task. Dominated models (worse on both metrics than some other
model) are pruned from the candidate set before presenting arms to the bandit.

This serves two functions:

1. **Reduces exploration waste** -- the bandit does not spend trials on
   clearly inferior models.
2. **Focuses the tradeoff** -- the remaining Pareto-optimal models represent
   genuine cost-quality tradeoffs that the bandit must resolve.

---

## 2. Dominance Definition

Model A dominates model B when:

- A has `pass_rate >= B.pass_rate`, AND
- A has `cost_per_success <= B.cost_per_success`, AND
- At least one inequality is strict.

```
Model A: pass_rate=0.90, cost/success=$10.00
Model B: pass_rate=0.70, cost/success=$12.00
Model C: pass_rate=0.80, cost/success=$9.00

A dominates B (higher pass rate AND lower cost).
Neither A nor C dominates the other:
  A has higher pass rate, but C has lower cost.
  -> Both are Pareto-optimal.
```

---

## 3. Algorithm

```rust
pub fn compute_pareto_frontier(
    stats: &HashMap<String, ModelObservation>
) -> Vec<String> {
    let mut frontier = Vec::new();

    for (slug_a, obs_a) in stats {
        let dominated = stats.iter().any(|(slug_b, obs_b)| {
            slug_b != slug_a
                && obs_b.pass_rate >= obs_a.pass_rate
                && obs_b.cost_per_success <= obs_a.cost_per_success
                && (obs_b.pass_rate > obs_a.pass_rate
                    || obs_b.cost_per_success < obs_a.cost_per_success)
        });

        if !dominated {
            frontier.push(slug_a.clone());
        }
    }

    frontier.sort();
    frontier
}
```

O(n^2) where n is the number of models. With typical counts (3-10), this is
negligible.

### 3.1 ModelObservation

```rust
pub struct ModelObservation {
    pub pass_rate: f64,
    pub cost_per_success: f64,
    pub avg_latency_ms: f64,     // tracked but not used in dominance check
    pub observations: u64,
}
```

`avg_latency_ms` is tracked for future extension to a three-objective frontier.

---

## 4. Visualization

```
Pass Rate ^
    1.0 |         * A (Pareto-optimal)
        |
    0.8 |    * C (Pareto-optimal)
        |
    0.7 |              x B (dominated by A)
        |
    0.6 |
        |
    0.0 +----------------------------------------> Cost/Success
        $0   $5    $9   $10   $12   $15
```

The Pareto frontier is the upper-left boundary. Points below and to the right
of any frontier point are dominated.

---

## 5. Integration with Cascade Router

The cascade router recomputes the Pareto frontier every
`PARETO_RECOMPUTE_INTERVAL = 50` observations:

```
CascadeRouter::update(model, reward, cost)
    |
    +-- Update model stats (trials, successes, costs)
    |
    +-- if observations % 50 == 0:
    |       |
    |       +-- Collect ModelObservation for each model
    |       |     pass_rate = successes / trials
    |       |     cost_per_success = total_cost / successes
    |       |
    |       +-- pareto_frontier = compute_pareto_frontier(observations)
    |
    +-- Store frontier for use in next select() call
```

During `select()`, only models on the Pareto frontier are presented as
candidates. Models that fell off are excluded until the next recomputation.

---

## 6. Multi-Objective Extension

The current implementation uses two objectives. The target extension adds four:

| Objective | Direction | Weight |
|-----------|-----------|--------|
| Quality (pass rate) | Maximize | Configurable |
| Cost per success | Minimize | Configurable |
| Latency (p50) | Minimize | Configurable |
| Reliability (1 - error rate) | Maximize | Configurable |

Uses scalarization: each objective is weighted and combined into a single
score, preserving O(n^2) complexity.

---

## 7. Edge Cases

### 7.1 All Models Dominated

If one model has the highest pass rate AND lowest cost, all others are
dominated. The frontier contains only one model and the bandit has no choice.
This is the expected steady-state for mature systems.

### 7.2 Insufficient Observations

Models with very few observations have noisy statistics. A model that succeeded
on its first 3 trials appears to have 100% pass rate. The cascade router
mitigates this by requiring a minimum observation count before including a model
in Pareto computation. Models below this threshold are always included
(exploration) regardless of dominance.

### 7.3 New Models

When a new model is added, it starts with zero observations and is excluded
from Pareto computation. The bandit gives it maximum exploration priority
(UCB1 selects unpulled arms first).

---

## 8. Frontier Evolution

### 8.1 Cold Start

All models are on the frontier (no data to establish dominance). The bandit
explores uniformly.

### 8.2 Convergence Phase (50-200 Observations)

Dominated models fall off. The set typically converges to 2-3 Pareto-optimal
models representing genuine tradeoffs.

### 8.3 Steady State (200+)

The frontier stabilizes. Changes occur when a provider updates a model, a new
model is added, or the task mix changes.

### 8.4 Provider Updates

When a provider deploys a new model version, the cascade router:

1. Detects the version change (slug comparison).
2. Discounts old observations (partial reset).
3. Re-includes the model with reduced weight.

---

## 9. Practical Example

After 300 observations:

```
Model               Pass Rate   Cost/Success   On Frontier?
-------------------------------------------------------------
claude-haiku-4.5     0.78        $0.12          YES (cheapest)
claude-sonnet-4      0.86        $0.95          YES (mid-range)
claude-opus-4        0.91        $2.40          YES (highest quality)
deepseek-chat        0.72        $0.45          NO (dominated by haiku)
```

After a provider update where deepseek improves to 0.85 pass rate:

```
Model               Pass Rate   Cost/Success   On Frontier?
-------------------------------------------------------------
claude-haiku-4.5     0.78        $0.12          YES (cheapest)
deepseek-chat        0.85        $0.45          YES (new entrant)
claude-sonnet-4      0.86        $0.95          NO (dominated by deepseek!)
claude-opus-4        0.91        $2.40          YES (highest quality)
```

Sonnet is dominated by deepseek (nearly same pass rate at half the cost).
