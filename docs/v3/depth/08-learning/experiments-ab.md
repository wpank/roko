# 08-learning/21 -- Prompt Experiments (A/B Testing)

> Bandit-driven A/B testing for prompt section variants. Each experiment
> assigns variants using UCB1, evaluates against gate pass rate, and
> concludes via chi-square significance testing with Wilson score confidence
> intervals. Variance Inequality checks enable early stopping.

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 8

**Source:** `crates/roko-learn/src/prompt_experiment.rs` (`ExperimentStore`,
`PromptVariant`, `VariantStats`, `ExperimentArchive`),
`crates/roko-learn/src/experiment_receipt.rs`,
`crates/roko-cli/src/runner/prompt_experiments.rs`

---

## 1. Purpose

The prompt experiment system treats prompt optimization as an online
learning problem rather than a static compilation problem. Where DSPy
(Khattab et al. 2024) generates many variants and evaluates them on a
test set to find a static winner, Roko's experiments run during live
execution: variants are assigned to real tasks using bandit selection,
and outcomes are evaluated through the real gate pipeline.

This online approach has three advantages over static prompt optimization:

1. **No separate test set.** Every production task contributes evaluation
   data, eliminating the need for a held-out benchmark.
2. **Non-stationarity tolerance.** The best prompt variant may change as
   the codebase, task mix, or model provider evolves. Online experiments
   track these shifts automatically.
3. **Sample efficiency.** Bandit selection (UCB1) concentrates samples on
   promising variants, requiring fewer total trials than uniform random
   assignment.

---

## 2. Experiment Structure

### 2.1 PromptVariant

```rust
pub struct PromptVariant {
    /// Unique identifier (e.g. "concise-v2").
    pub id: String,
    /// Human-readable label.
    pub name: String,
    /// Prompt section this replaces (e.g. "constraints").
    pub section_name: String,
    /// Actual prompt text content.
    pub content: String,
    /// Optional model slug for model experiments.
    pub slug: Option<String>,
    /// Whether this variant is still eligible.
    pub active: bool,
}
```

Each variant replaces a specific prompt section. When assigned to a task,
the variant's `content` replaces the default content of the named section
in the composed system prompt. The original section content serves as the
implicit baseline variant.

### 2.2 VariantStats

```rust
pub struct VariantStats {
    /// Total assignments.
    pub trials: u64,
    /// Successful outcomes (gate pass).
    pub successes: u64,
}
```

The per-variant outcome tracker is deliberately minimal: trial count and
success count. All derived statistics (success rate, UCB score, confidence
intervals) are computed from these two counters.

---

## 3. Variant Selection: UCB1

Variant selection uses UCB1 (Auer, Cesa-Bianchi & Fischer 2002) to balance
exploration and exploitation:

```rust
fn ucb_score(&self, total_trials: u64) -> f64 {
    if self.trials == 0 {
        return f64::MAX; // Explore unsampled arms first.
    }
    let mean = self.successes as f64 / self.trials as f64;
    let exploration = (2.0 * (total_trials as f64).ln() / self.trials as f64).sqrt();
    mean + exploration
}
```

### 3.1 Selection Algorithm

```
On new task assignment for experiment E:
    total = sum of all variant trials
    For each active variant V:
        score_V = V.success_rate() + sqrt(2 * ln(total) / V.trials)
    Select variant with highest UCB score
    Record reservation: (task_id, experiment_id, variant_id)
```

Unsampled variants receive score `f64::MAX`, ensuring they are tried at
least once before any exploitation occurs. After the initial exploration
round, UCB1 concentrates samples on the best-performing variant while
maintaining logarithmic exploration of alternatives.

### 3.2 Reservation Tracking

Each variant assignment creates a reservation that persists until the
task completes:

```
reservation = (task_id, experiment_id, variant_id, assigned_at)
```

Reservations prevent double-counting: if a task is retried (due to gate
failure and replan), the reservation ensures the same variant is assigned
on retry. The experiment receives exactly one outcome per reservation,
regardless of how many retries the task requires.

---

## 4. Conclusion Criteria: Chi-Square Test

An experiment concludes when the chi-square test rejects the null
hypothesis that all variants have equal success rates.

### 4.1 Chi-Square Statistic

For k variants with observed successes (s_i) and failures (f_i = n_i - s_i):

```
Expected successes: E_i = n_i * p_pooled
    where p_pooled = sum(s_i) / sum(n_i)

Chi-square = sum((s_i - E_i)^2 / E_i + (f_i - (n_i - E_i))^2 / (n_i - E_i))

Degrees of freedom = k - 1
```

### 4.2 Conclusion Thresholds

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `min_trials_per_variant` | 20 | Minimum data before conclusion |
| `min_total_trials` | 50 | Global minimum across all variants |
| `max_p_value` | 0.05 | Significance threshold |
| `min_effect_size` | 0.05 | Minimum absolute success rate difference |

An experiment concludes when all four conditions are met:

```
1. Every active variant has >= 20 trials
2. Total trials >= 50
3. Chi-square p-value < 0.05
4. |best_rate - second_best_rate| >= 0.05 (5% absolute difference)
```

### 4.3 Why Chi-Square (Not Fisher's Exact or Z-Test)

- **Fisher's exact test** is computationally expensive for k > 2 variants
  and large sample sizes. Chi-square is asymptotically equivalent and O(k).
- **Z-test** (two-proportion) only compares two variants at a time. Chi-
  square handles k >= 2 variants in a single test.
- **Chi-square** is the standard omnibus test for contingency tables. It
  detects whether *any* variant differs from the pool, and the effect size
  requirement ensures the difference is practically meaningful, not merely
  statistically significant.

---

## 5. Confidence Intervals: Wilson Score

The Wilson score interval provides confidence bounds on the estimated
success rate of each variant:

```rust
fn confidence_interval_95(&self) -> (f64, f64) {
    if self.trials == 0 {
        return (0.0, 0.0);
    }
    let n = self.trials as f64;
    let p = self.success_rate();
    let z = 1.96;
    let z_sq = z * z;
    let denom = 1.0 + z_sq / n;
    let center = (p + z_sq / (2.0 * n)) / denom;
    let margin = (z / denom)
        * ((p * (1.0 - p) / n + z_sq / (4.0 * n * n)).sqrt());
    (
        (center - margin).clamp(0.0, 1.0),
        (center + margin).clamp(0.0, 1.0),
    )
}
```

### 5.1 Why Wilson (Not Wald)

The standard "Wald" confidence interval (`p +/- z * sqrt(p*(1-p)/n)`) has
two problems:

1. **Boundary bias.** When p is near 0 or 1, the Wald interval can extend
   below 0 or above 1, producing meaningless bounds.
2. **Poor coverage.** For small n, the Wald interval has actual coverage
   significantly below the nominal 95%.

The Wilson score interval corrects both problems by solving a different
equation: instead of `|p - p_hat| < z * SE`, it solves the quadratic
`(p - p_hat)^2 < z^2 * p*(1-p)/n`. The result is an interval that:
- Never extends below 0 or above 1
- Has near-nominal coverage for all n >= 5
- Shrinks toward the center when n is small (a Bayesian-flavored correction)

### 5.2 Use in Diagnostics

Wilson intervals are reported in `roko learn experiments`:

```
Experiment: system-prompt-v2
    Variant "concise":  72/100 = 72.0% [62.3%, 80.0%]
    Variant "verbose":  58/95  = 61.1% [50.7%, 70.6%]
    Variant "baseline": 63/98  = 64.3% [54.1%, 73.4%]

    Chi-square p-value: 0.032 (significant at alpha=0.05)
    Winner: "concise" with 72.0% pass rate
```

The non-overlapping Wilson intervals between "concise" and the other
variants provide additional visual confirmation of the chi-square result.

---

## 6. Variance Inequality Early Stopping

When the variance between the leading and second-best variant is too small
relative to sample size, the experiment is futile -- more data cannot
produce a significant result.

```rust
pub struct VICheck {
    pub can_conclude: bool,
    pub reason: String,
    pub leading_variant: Option<String>,
    pub gap: f64,
    pub required_gap: f64,
}
```

### 6.1 Mechanism

```
For each active variant pair (i, j):
    gap = |rate_i - rate_j|
    se = sqrt(rate_i * (1 - rate_i) / n_i + rate_j * (1 - rate_j) / n_j)
    required_gap = z_alpha * se + min_effect_size

If gap < required_gap for the leading pair AND n > min_trials:
    -> Experiment cannot conclude
    -> Recommend archiving as inconclusive
```

The VI check prevents the experiment from running indefinitely when variants
perform similarly. Without early stopping, an experiment with two 70% pass
rate variants would need tens of thousands of trials to achieve statistical
significance -- a waste of execution budget.

---

## 7. Experiment Lifecycle

### 7.1 States

```rust
pub enum ExperimentStatus {
    Running,     // Actively assigning variants
    Concluded,   // Winner identified
}
```

### 7.2 Full Lifecycle

```
1. CREATE
    roko config experiments add --section constraints --variants concise,verbose
    -> Creates experiment with status=Running
    -> Registers variant prompt texts from template files

2. ASSIGN (automatic, per-task)
    On task dispatch where experiment section matches:
        -> UCB1 selects variant
        -> Variant text replaces section in composed prompt
        -> Reservation recorded

3. RECORD (automatic, per-episode)
    On episode completion:
        -> Match episode to reservation
        -> Increment variant.trials and variant.successes (if passed)

4. CHECK (automatic, every 50 episodes)
    Run chi-square test:
        If significant AND effect_size >= min_effect_size:
            -> Status = Concluded
            -> Archive final stats (ExperimentArchive)
            -> Winner persisted to static overrides
        If VI check says futile:
            -> Archive as inconclusive

5. SETTLE (automatic or manual)
    Concluded winner becomes static default:
        -> Written to .roko/learn/static-overrides.json
        -> Cascade router's static table updated
        -> Loop 8 (Experiments -> Static) closes
```

### 7.3 ExperimentArchive

```rust
pub struct ExperimentArchive {
    pub concluded_at: DateTime<Utc>,
    pub final_stats: HashMap<String, VariantStats>,
    pub p_value: f64,
    pub effect_size: f64,
}
```

The archive is an immutable snapshot of the experiment's final state at
conclusion time. It preserves the exact per-variant trial counts, success
counts, chi-square p-value, and effect size for auditability. Archives are
never modified after creation.

---

## 8. Static Overrides (Loop 8)

When an experiment concludes, its winner is persisted to the static
overrides file:

```json
{
    "overrides": [
        {
            "experiment_id": "system-prompt-v2",
            "parameter": "constraints",
            "winning_value": "concise",
            "confidence": 0.95
        }
    ]
}
```

File: `.roko/learn/static-overrides.json`

On subsequent task dispatches, the experiment store returns the winning
variant without running the bandit -- the experiment is settled. This
closes Loop 8 (Experiments -> Static) from the eight cybernetic feedback
loops: experiment results become durable defaults.

---

## 9. Runner Integration

### 9.1 ExperimentReceipt

The runner's prompt experiment integration uses a receipt protocol to
ensure exactly-once settlement:

```
1. Pre-dispatch: ExperimentStore assigns variant, creates reservation
2. Runner builds prompt with variant content
3. Agent executes task
4. Gate pipeline evaluates
5. Post-dispatch: ExperimentReceipt settles reservation with outcome
6. ExperimentStore updates VariantStats
```

The receipt protocol ensures that each task contributes exactly one
outcome to exactly one experiment, even in the presence of retries,
concurrent execution, and process restarts.

### 9.2 ACP Integration

ACP dispatches also participate in experiments when the ACP session's
context matches an active experiment's section. However, ACP experiment
assignment currently uses context injection rather than the full
canonical-section/receipt protocol. This is tracked as a gap in
`.roko/GAPS.md`.

---

## 10. Diagnostics

```
$ roko learn experiments

Active experiments:
    system-prompt-v2:
        Status: Running (45 total trials)
        Variants:
            concise:  18/25 = 72.0% [51.5%, 86.5%]  (UCB: 1.24)
            verbose:  10/20 = 50.0% [29.0%, 71.0%]  (UCB: 1.02)
        Chi-square p = 0.124 (not significant)
        VI check: can still conclude (gap = 22.0%, required = 18.5%)

Concluded experiments:
    format-instructions-v1:
        Winner: "structured" (78.5% vs 64.2% baseline)
        p = 0.008, n = 180
        Concluded at: 2026-09-10T14:32:00Z

Archived (inconclusive):
    role-preamble-v3:
        Reason: VI check -- variants within 2.1%, required gap 8.3%
        Total trials: 312
```

---

## References

- Auer, P., Cesa-Bianchi, N. & Fischer, P. (2002). Finite-time analysis
  of the multiarmed bandit problem. *Machine Learning* 47(2-3), 235-256.
- Khattab, O. et al. (2024). DSPy: Compiling Declarative Language Model
  Calls into Self-Improving Pipelines. *ICLR 2024*.
- Wilson, E.B. (1927). Probable inference, the law of succession, and
  statistical inference. *Journal of the American Statistical Association*
  22(158), 209-212.
- Pearson, K. (1900). On the criterion that a given system of deviations
  from the probable in the case of a correlated system of variables is
  such that it can be reasonably supposed to have arisen from random
  sampling. *Philosophical Magazine* 50(302), 157-175.
