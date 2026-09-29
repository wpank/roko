# 08-learning/16 -- Collective Calibration (31x Heuristic)

> The 31.6x heuristic provides a measurement framework for collective agent
> performance. The C-Factor composite metric, leave-one-out contributions,
> and calibration regression detection turn a theoretical upper bound into
> a practical diagnostic sensor.

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 7

**Source:** `crates/roko-learn/src/cfactor.rs` (`compute_cfactor`,
`CFactorGovernance`, `detect_cfactor_regression`, `detect_pathologies`,
`VICheck`)

**Academic basis:** Central Limit Theorem (inspiration, not proof);
Woolley et al. 2010 (collective intelligence factor)

---

## 1. The 31.6x Heuristic

### 1.1 Derivation

The heuristic models accuracy as:

```
accuracy(t) = 1 - 1 / sqrt(N * t)
```

where:
- N = number of agents in the collective
- t = number of calibration rounds (episodes)

For N = 10 agents and t = 100 rounds:

```
accuracy = 1 - 1 / sqrt(10 * 100) = 1 - 1 / sqrt(1000) ~ 1 - 0.0316 ~ 0.968
```

The "31.6x" refers to the sqrt(1000) ~ 31.6 factor in the denominator,
which represents the effective sample size advantage of a calibrated
collective over a single agent.

### 1.2 CLT Inspiration

The formula is inspired by the Central Limit Theorem: the standard error
of a sample mean decreases as 1/sqrt(n). If each agent provides an
independent observation and the collective aggregates these observations,
the collective's error decreases as 1/sqrt(N * t).

### 1.3 Explicit Caveats

**This is NOT a theorem.** The following assumptions are required and
frequently violated:

1. **Independence.** Agents' errors must be independent. In practice,
   agents using the same model and similar prompts make correlated errors.
   Correlation reduces the effective N.

2. **Stationarity.** The target distribution must not change during
   calibration. The codebase evolves, model providers update, and task
   distributions shift. Non-stationarity reduces the effective t.

3. **Aggregation mechanism.** The formula assumes optimal aggregation
   (e.g., majority voting or Bayesian averaging). Roko uses sequential
   execution with feedback, not parallel voting. The aggregation mechanism
   affects the constant factor.

4. **Finite-sample effects.** For small N and t, the 1/sqrt(N*t)
   approximation is loose. The CLT is an asymptotic result.

5. **Heterogeneous quality.** Equal-quality agents are assumed. If some
   agents are much worse than others, they add noise rather than signal.

**In practice, expect 3-10x improvement, not 31.6x.** The 31.6x is the
idealized upper bound under perfect conditions.

---

## 2. C-Factor: Composite Capability Metric

The C-Factor (Collective Capability Factor) is the practical implementation
of collective calibration measurement. It combines multiple performance
indicators into a single scalar.

### 2.1 Components

```rust
pub struct CFactorComponents {
    /// % of tasks passing gates on first attempt.
    pub gate_pass_rate: f64,
    /// Inverse of cost per successful task, normalized.
    pub cost_efficiency: f64,
    /// Inverse of time per successful task, normalized.
    pub speed: f64,
    /// Normalized signal throughput.
    pub information_flow_rate: f64,
    /// % of tasks succeeding without re-plan.
    pub first_try_rate: f64,
    /// Rate of new knowledge entries per episode.
    pub knowledge_growth: f64,
    /// Speed of shared insight accumulation.
    pub knowledge_integration_rate: f64,
    /// How strongly templates specialize by category.
    pub task_diversity_coverage: f64,
    /// Speed of convergent conclusions.
    pub convergence_velocity: f64,
    /// Evenness of agent participation.
    pub turn_taking_equality: f64,
    /// Normalized dependency output rate.
    pub social_sensitivity: f64,
}
```

### 2.2 Component Weights

The composite score is a weighted average. Default weights emphasize outcome
metrics over process metrics:

| Component | Weight | Rationale |
|-----------|--------|-----------|
| gate_pass_rate | 0.20 | Primary success metric |
| cost_efficiency | 0.15 | Budget sustainability |
| first_try_rate | 0.15 | Efficiency of approach |
| speed | 0.10 | Throughput |
| knowledge_growth | 0.10 | Learning velocity |
| turn_taking_equality | 0.05 | Collaboration quality |
| Others | 0.25 (distributed) | Secondary indicators |

### 2.3 Normalization

Each component is normalized to [0.0, 1.0] before weighting. Normalization
uses a baseline window: the component value from the first 10 plans serves
as the reference point. Values below baseline map to [0.0, 0.5], values at
baseline map to 0.5, and values above baseline map to [0.5, 1.0].

This relative normalization means the C-Factor measures improvement over
the system's own baseline, not against an absolute standard. A C-Factor of
0.8 means the system is performing significantly better than its initial
configuration.

---

## 3. Five Process Variables (Woolley et al. 2010)

The C-Factor's component selection is informed by Woolley et al.'s
collective intelligence research, which identified process variables that
predict group performance independently of individual member ability.

### 3.1 Variable Mapping

```
c_factor = w_1 * turn_taking_entropy
         + w_2 * peer_prediction_accuracy
         + w_3 * citation_reciprocity
         + w_4 * delivery_rate
         + w_5 * hdc_diversity
         + bias
```

| Variable | Formula | What It Measures |
|----------|---------|------------------|
| Turn-taking entropy | `H = -sum(p_i * ln(p_i)) / ln(N)` | Conversational equality |
| Peer prediction | `1.0 - MSE(predictions, outcomes)` | Social perceptiveness |
| Citation reciprocity | `survived_citations / total_citations` | Trust calibration |
| Delivery rate | `confirmed / (confirmed + dropped)` | Channel openness |
| HDC diversity | `1.0 - mean_pairwise_similarity` | Cognitive diversity |

### 3.2 Learned Weights

Weights are learned online via gradient descent from cohort outcomes, not
declared by fiat:

```
On cohort completion with observed outcome_quality:
    predicted = weights.dot(metrics)
    error = outcome_quality - predicted

    w_i += learning_rate * error * metric_i
```

This means the system discovers which process variables actually predict
quality in its specific context rather than relying on fixed weights from
the Woolley et al. study (which examined human groups, not agent collectives).

### 3.3 Turn-Taking Equality Computation

Turn-taking equality measures whether agent participation is evenly
distributed across a cohort:

```rust
fn compute_turn_taking_equality(episodes: &[&Episode]) -> f64 {
    // Count episodes per agent
    let mut agent_counts: HashMap<&str, u64> = HashMap::new();
    for ep in episodes {
        *agent_counts.entry(&ep.agent_id).or_insert(0) += 1;
    }

    // Compute Gini-based equality from participation distribution
    let counts: Vec<u64> = agent_counts.values().copied().collect();
    turn_taking_equality_for_counts(counts)
}
```

A collective where one agent handles 90% of episodes and others handle 10%
has low turn-taking equality. This matters because it means the collective
is effectively a single agent -- losing the diversity benefit that
motivates collective calibration.

See [c-factor-governance.md](c-factor-governance.md) for the full five-
variable governance system, pathology detection, and non-binding
recommendation generation.

---

## 4. Leave-One-Out Contributions

The C-Factor includes per-agent contribution scores computed via
leave-one-out analysis:

```rust
pub struct AgentCFactorContribution {
    /// Agent identifier.
    pub agent_id: String,
    /// Episodes attributed to this agent.
    pub episode_count: usize,
    /// C-Factor without this agent's episodes.
    pub without_agent_overall: f64,
    /// Full score minus leave-one-out score.
    pub contribution_score: f64,
}
```

If `contribution_score > 0`, the agent raises the collective C-Factor
(positive contributor). If `contribution_score < 0`, the agent drags the
C-Factor down (negative contributor).

### 4.1 Dispatch Bias

Leave-one-out contributions inform routing decisions:

```rust
pub enum AgentDispatchBias {
    /// Agent has negative contribution -> prefer stronger model.
    PreferStronger,
    /// Agent has strong positive contribution -> prefer cheaper model.
    PreferCheaper,
    /// Neutral contribution -> no bias.
    Neutral,
}
```

The cascade router uses this bias during the confidence stage: agents with
consistently negative contributions are routed to stronger (more expensive)
models, while agents with strong positive contributions can be routed to
cheaper models without sacrificing quality.

---

## 5. C-Factor Regression Detection

```rust
pub fn detect_cfactor_regression(
    history: &[CFactor],
    window: Duration,
    regression_threshold: f64,
) -> Option<CFactorRegression> {
    // Compare recent C-Factor against trailing average
    // Trigger regression alert when drop exceeds threshold
}
```

A C-Factor regression is triggered when the current C-Factor drops
significantly below the trailing average. This catches systemic
degradation that individual metrics might miss -- a small drop in pass
rate combined with a small increase in cost and a small decrease in speed
may not trigger any individual threshold, but the C-Factor composite
detects the overall decline.

---

## 6. Variance Inequality Check

The Variance Inequality check determines whether an experiment can still
produce a statistically significant result:

```rust
pub struct VICheck {
    pub can_conclude: bool,
    pub reason: String,
    pub leading_variant: Option<String>,
    pub gap: f64,
    pub required_gap: f64,
}
```

When the variance between the leading variant and the second-best is too
small relative to sample size, the experiment cannot conclude -- more data
would not change the outcome. The VI check terminates such experiments
early, saving execution budget.

---

## 7. Practical Interpretation

| C-Factor | Interpretation | Action |
|----------|---------------|--------|
| < 0.3 | System performing poorly | Investigate regressions, consider manual intervention |
| 0.3 - 0.5 | Below baseline | Check feedback loops, review recent changes |
| 0.5 | At baseline | Normal operation |
| 0.5 - 0.7 | Above baseline, improving | Learning loops are working |
| 0.7 - 0.9 | Well above baseline | System has significantly improved through self-optimization |
| > 0.9 | Near-optimal | Consider lowering cost while maintaining quality |

---

## 8. Computation Schedule

The C-Factor is computed every 50 episodes (the slowest learning frequency):

```
Every 50 episodes:
    |
    +-- 1. Load recent episodes (sliding window of last 200)
    |
    +-- 2. Compute component metrics:
    |       gate_pass_rate, cost_efficiency, speed,
    |       first_try_rate, knowledge_growth, ...
    |
    +-- 3. Compute leave-one-out contributions per agent
    |
    +-- 4. Combine components with weights -> overall score
    |
    +-- 5. Persist to .roko/learn/cfactor.json
```

The 50-episode frequency makes the C-Factor the most stable anchor in the
learning system. It does not react to individual episode outcomes or
short-term routing changes. It reflects long-term trends in collective
performance.

---

## References

- Woolley, A.W. et al. (2010). Evidence for a collective intelligence
  factor in the performance of human groups. *Science* 330(6004), 686-688.
- Surowiecki, J. (2004). *The Wisdom of Crowds*. Doubleday.
