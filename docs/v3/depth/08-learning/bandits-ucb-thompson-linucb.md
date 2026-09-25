# 08-learning/04 -- Bandits: UCB1, Thompson Sampling, LinUCB

> Full mathematical treatment of the three bandit algorithms used for repeated
> decision-making throughout the learning system, plus Track-and-Stop for
> best-arm identification, bandit ensembles, and visualization diagnostics.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/bandits.rs`,
`crates/roko-learn/src/model_router.rs`

**Persistence:** `.roko/learn/cascade-router.json` (LinUCB state), per-bandit
JSON files

**Academic basis:** Auer, Cesa-Bianchi & Fischer (2002) -- UCB1;
Thompson (1933) -- Thompson Sampling; Li et al. (2010) -- LinUCB;
Garivier & Kaufmann (2016) -- Track-and-Stop

**Cross-references:** [cascade-router](cascade-router.md),
[pareto-frontier-pruning](pareto-frontier-pruning.md),
[cost-normalization](cost-normalization.md),
[provider-health-circuit-breaker](provider-health-circuit-breaker.md)

---

## 1. Purpose

Roko uses multi-armed bandit algorithms for every repeated decision in the
system: which model to route a task to, which prompt section to include, which
tool format to use, which backend to prefer. Bandits provide a principled
framework for balancing exploration (trying less-tested options) against
exploitation (using the best-known option), with formal regret bounds that
guarantee convergence to optimal choices.

The `roko-learn` crate provides four bandit implementations, each suited to a
different decision structure:

| Bandit | Algorithm | Use Case | Key Property |
|--------|-----------|----------|--------------|
| `UcbBandit` | UCB1 (Auer et al. 2002) | Context-free repeated decisions | O(sqrt(T ln T)) cumulative regret |
| `LinUCBRouter` | LinUCB (Li et al. 2010) | Context-dependent model routing | Handles 18-dim context vectors |
| `TrackAndStopBandit` | Track-and-Stop (Garivier & Kaufmann 2016) | Best-arm identification | Stops when confident |
| `BanditBank` | Collection of UCB1 instances | Keyed decision spaces | One bandit per context key |

---

## 2. UCB1: Upper Confidence Bound

### 2.1 UCB1 Formula

The `UcbBandit` implements the classic UCB1 algorithm for context-free
multi-armed bandits. For K arms and a total of T pulls, at each decision point
the algorithm selects the arm that maximizes the upper confidence bound.

For each arm `a` with `n_a` observations and empirical mean reward
`mu_hat_a`:

```
UCB(a) = mu_hat_a + C * sqrt(ln(T) / n_a)
```

where:

- `mu_hat_a = (sum of rewards for arm a) / n_a` is the empirical mean
  reward for arm a.
- `T = sum of n_a across all arms` is the total number of pulls so far.
- `C` is the exploration constant. The theoretical optimum is `sqrt(2)` for
  rewards in [0, 1] (Auer et al. 2002, Theorem 1). Roko uses C = sqrt(2)
  by default.
- `ln(T)` is the natural logarithm of total pulls.

**Selection rule:** Select arm `a* = argmax_a UCB(a)`.

**Initialization:** Arms with `n_a = 0` receive UCB = +infinity and are always
selected before any pulled arm. Tiebreaking is deterministic by insertion
order.

### 2.2 Regret Bound

**Theorem (Auer et al. 2002).** For K arms with expected rewards
mu_1 >= mu_2 >= ... >= mu_K where mu_1 is the best arm, the expected
cumulative regret of UCB1 after T rounds is bounded by:

```
E[R_T] <= sum_{a: mu_a < mu_1} [ (8 * ln(T)) / Delta_a ] + (1 + pi^2/3) * sum_{a=1}^{K} Delta_a
```

where `Delta_a = mu_1 - mu_a` is the gap between the best arm and arm a.

This gives O(sqrt(K * T * ln(T))) cumulative regret in the worst case, which
is within a logarithmic factor of the theoretical minimum O(sqrt(K * T)).

**Interpretation for Roko:** With 5 model arms and 200 observations, the
expected cumulative regret is bounded by approximately:

```
R_200 <= 8 * ln(200) / Delta_min * (K - 1)

For Delta_min = 0.1 (10% gap between best and second-best model):
R_200 <= 8 * 5.3 / 0.1 * 4 = 1,696 regret units
```

In practice, Roko observes much lower regret because the arm gaps are
typically larger than 10%.

### 2.3 Reward Scaling

UCB1 regret bounds assume rewards in [0, 1]. Callers must normalize:

| Outcome | Reward |
|---------|--------|
| Gate pass (first attempt) | 1.0 |
| Gate pass (after retry) | 0.7 |
| Gate fail (recoverable) | 0.2 |
| Gate fail (unrecoverable) | 0.0 |
| Cost efficiency bonus | 1.0 - (cost / max_cost) |

### 2.4 Schema

```rust
pub struct BanditArm {
    /// Human-readable name (e.g. "claude", "codex").
    pub name: String,
    /// Number of times this arm has been pulled.
    pub pulls: u64,
    /// Cumulative reward received across all pulls.
    pub total_reward: f64,
}

pub struct UcbBandit {
    arms: RwLock<Vec<BanditArm>>,
    total_pulls: AtomicU64,
    /// UCB exploration constant (default: sqrt(2)).
    exploration_c: f64,
    /// Persistence path (optional).
    persist_path: Option<PathBuf>,
}
```

### 2.5 Thread Safety

`UcbBandit` uses `parking_lot::RwLock` for arm stats and `AtomicU64` for the
pull counter. `select()` acquires only a shared read lock while `update()`
acquires an exclusive write lock. Concurrent `select()` calls never block each
other -- only an in-progress `update()` causes contention.

### 2.6 Use Cases

- **Backend selection**: which LLM provider to route a request to.
- **Retry strategy**: immediate retry vs. escalate model vs. re-plan.
- **Context-size buckets**: how much context to include in the prompt.
- **Prompt experiment variant selection**: which variant of a prompt section.

---

## 3. Thompson Sampling with Beta Posteriors

### 3.1 Algorithm

Thompson Sampling (Thompson 1933) maintains a Bayesian posterior for each arm
and samples from it to make decisions. For binary rewards (success/failure),
the natural posterior is the Beta distribution.

For each arm `a` with `alpha_a` successes and `beta_a` failures:

```
Prior:     Beta(1, 1)        (uniform prior)
Posterior: Beta(alpha_a, beta_a)
           where alpha_a = 1 + successes_a
                 beta_a  = 1 + failures_a
```

**Selection rule:**

```
1. For each arm a, sample theta_a ~ Beta(alpha_a, beta_a)
2. Select arm a* = argmax_a theta_a
```

### 3.2 Posterior Update

On observing reward r_t in {0, 1} for arm a:

```
alpha_a <- alpha_a + r_t
beta_a  <- beta_a  + (1 - r_t)
```

### 3.3 Expected Value and Variance

The posterior mean and variance for arm a are:

```
E[theta_a]   = alpha_a / (alpha_a + beta_a)
Var[theta_a] = (alpha_a * beta_a) / ((alpha_a + beta_a)^2 * (alpha_a + beta_a + 1))
```

As observations accumulate, the variance shrinks and the posterior concentrates
around the true success probability. This provides natural exploration:
well-observed arms have tight posteriors and are rarely sampled far from their
mean, while under-observed arms have wide posteriors and occasionally produce
high samples that trigger exploration.

### 3.4 Bayesian Regret Bound

**Theorem (Agrawal & Goyal 2012).** For K arms with Bernoulli rewards,
Thompson Sampling with Beta(1,1) priors achieves expected Bayesian regret:

```
E[R_T] = O(sqrt(K * T * ln(T)))
```

This matches UCB1's regret order while being empirically superior in many
practical settings due to more aggressive exploitation of posterior information.

### 3.5 Thompson Sampling with Discount Factor (Drift Handling)

When arm reward distributions change over time (non-stationarity), standard
Thompson Sampling converges on stale beliefs. The discounted variant
(Kocsis & Szepesvari 2006, adapted) applies a discount factor gamma in
(0, 1) to past observations:

```
alpha_a <- gamma * alpha_a + r_t
beta_a  <- gamma * beta_a  + (1 - r_t)
```

With gamma = 0.95 (Roko default), the effective memory window is
approximately 1/(1 - gamma) = 20 recent observations. Older observations
are exponentially downweighted, allowing the posterior to track distributional
shifts such as provider model updates.

**Effective sample size:**

```
N_eff = alpha_a + beta_a
      = sum_{t=0}^{T} gamma^(T-t)
      = (1 - gamma^T) / (1 - gamma)
      -> 1 / (1 - gamma) as T -> infinity
```

For gamma = 0.95, N_eff converges to 20, meaning the bandit "remembers" the
equivalent of 20 observations regardless of how many total observations it has
seen.

---

## 4. LinUCB: Contextual Bandit Router

### 4.1 LinUCB Formula

The `LinUCBRouter` implements the LinUCB algorithm (Li et al. 2010) for
context-dependent model selection. Unlike UCB1, which treats each arm
independently, LinUCB models the expected reward as a linear function of a
context vector, allowing the router to generalize across similar contexts.

For each arm `a` with context vector `x` in R^d:

```
score(a) = theta_a^T * x + alpha * sqrt(x^T * A_a^{-1} * x)
```

where:

- `theta_a = A_a^{-1} * b_a` is the ridge regression weight vector for arm a.
- `A_a = I_d + sum_{t: a_t = a} x_t * x_t^T` is the d x d design matrix
  (initialized to the identity matrix I_d).
- `b_a = sum_{t: a_t = a} r_t * x_t` is the d x 1 response vector
  (initialized to zero).
- `alpha` is the exploration parameter, controlling the width of the
  confidence set.

**Selection rule:** Select arm `a* = argmax_a score(a)`.

### 4.2 Update Rule

On observing context `x_t`, chosen arm `a_t`, and reward `r_t`:

```
A_{a_t} <- A_{a_t} + x_t * x_t^T
b_{a_t} <- b_{a_t} + r_t * x_t
theta_{a_t} <- A_{a_t}^{-1} * b_{a_t}
```

The matrix inverse `A_a^{-1}` can be maintained incrementally via the
Sherman-Morrison formula:

```
A^{-1}_{new} = A^{-1}_{old} - (A^{-1}_{old} * x * x^T * A^{-1}_{old}) /
               (1 + x^T * A^{-1}_{old} * x)
```

This avoids the O(d^3) cost of a full matrix inversion on each update, reducing
it to O(d^2).

### 4.3 Confidence Ellipsoid

The term `sqrt(x^T * A_a^{-1} * x)` is the width of the confidence ellipsoid
in the direction of context x. It is large when x lies in a direction where
few observations have been made (high uncertainty) and small when many
observations have been made in that direction (low uncertainty).

**Geometric interpretation:** The set of plausible weight vectors for arm a
forms an ellipsoid:

```
{ theta : (theta - theta_hat_a)^T * A_a * (theta - theta_hat_a) <= alpha^2 }
```

The UCB score is the maximum predicted reward over this ellipsoid:

```
max_{theta in ellipsoid} theta^T * x = theta_hat_a^T * x + alpha * sqrt(x^T * A_a^{-1} * x)
```

### 4.4 Context Vector (18 Dimensions)

The `RoutingContext` encodes task features into a fixed-length vector:

| Dimension(s) | Feature | Encoding |
|--------------|---------|----------|
| 0-7 | Task category | One-hot (8 `TaskCategory` variants) |
| 8 | Complexity band | Scalar: 0.0 (Fast) / 0.5 (Standard) / 1.0 (Complex) |
| 9 | Iteration | Normalized: iteration / 10, capped at 1.0 |
| 10-13 | Agent role | 4-dim float vector (hashed from role string) |
| 14 | Crate familiarity | success_count / total_count, clamped to [0, 1] |
| 15 | Has prior failure | Binary: 0.0 or 1.0 |
| 16 | Bias term | Always 1.0 |
| 17 | Cache affinity | 1.0 when candidate matches previous model, else 0.0 |

Total dimension: `CONTEXT_DIM = 18`.

### 4.5 Alpha Decay

The exploration parameter alpha decays exponentially from 1.0 to 0.05 over
200 observations:

```
alpha = 0.05 + 0.95 * exp(-observations / 60)
```

| Observations | alpha | Behavior |
|-------------|-------|----------|
| 0 | 1.00 | Maximum exploration |
| 50 | 0.47 | Balanced |
| 100 | 0.23 | Mostly exploitation |
| 200 | 0.08 | Near-pure exploitation |

The decay constant tau = 60 was chosen so that exp(-200/60) ~ 0.036, giving
effective convergence by 200 observations.

### 4.6 Regret Bound

**Theorem (Li et al. 2010, Abbasi-Yadkori et al. 2011).** For K arms,
d-dimensional contexts, and T rounds, the expected regret of LinUCB is:

```
E[R_T] = O(d * sqrt(T * ln(K * T * L / delta)))
```

where L is a bound on the norm of the weight vectors and delta is the
confidence parameter.

For Roko with d = 18, K = 5, T = 200:

```
R_200 ~ 18 * sqrt(200 * ln(5 * 200 * L / delta))
      ~ 18 * sqrt(200 * 10)
      ~ 18 * 44.7
      ~ 805 regret units
```

This is substantially lower than UCB1's context-free bound because LinUCB
exploits the structure in the context vector.

### 4.7 Cold Start

When observation count is below `COLD_START_THRESHOLD = 50`, the router falls
back to a static mapping from `ModelTier` to a default model slug. This
prevents LinUCB from making poorly-informed decisions with insufficient data
to estimate the 18-dimensional weight vectors.

### 4.8 Cache Affinity

Dimension 17 encodes cache affinity: 1.0 when the candidate model matches the
model used for the previous task in the same plan. This encodes the observation
that consecutive tasks in a plan often share similar context, and reusing the
same model allows the provider's KV cache to serve prefix tokens at reduced
cost.

The `CACHE_AFFINITY_BONUS = 0.15` in the cascade router provides an additional
static bonus for cache-consistent routing during the confidence stage.

---

## 5. Track-and-Stop: Best-Arm Identification

### 5.1 Algorithm

The `TrackAndStopBandit` implements the Track-and-Stop algorithm (Garivier &
Kaufmann 2016) for best-arm identification with anytime-valid stopping. Unlike
UCB1 which minimizes cumulative regret, Track-and-Stop minimizes the number of
samples needed to identify the best arm with probability >= 1 - delta.

```
Phase 1: Round-robin
    Pull each arm at least once.

Phase 2: D-tracking
    Compute target allocation proportions from gap estimates.
    Pull the arm most under-sampled relative to its target.
    Forced exploration: no arm falls below sqrt(t) - K/2 pulls.

Phase 3: Stopping
    When GLR statistic > beta(t, delta), declare winner.
    Stop exploring permanently for this key.
```

### 5.2 GLR Stopping Criterion

The Generalized Likelihood Ratio statistic for Bernoulli arms:

```
GLR(t) = t * KL(mu_hat_1, mu_hat_2)
```

where `mu_hat_1` and `mu_hat_2` are the empirical means of the top-2 arms,
and KL is the Kullback-Leibler divergence:

```
KL(p, q) = p * ln(p / q) + (1 - p) * ln((1 - p) / (1 - q))
```

**Stopping rule:** When `GLR(t) > beta(t, delta)` where:

```
beta(t, delta) = ln((ln(t) + 1) / delta)
```

the best arm is declared with confidence >= 1 - delta.

### 5.3 Sample Complexity

**Theorem (Garivier & Kaufmann 2016).** The expected number of samples for
Track-and-Stop to identify the best arm is:

```
E[tau] ~ T*(mu) * ln(1 / delta)
```

where `T*(mu)` is the characteristic time of the problem, defined by the
inverse of the KL-divergence gap between the best and second-best arms.

### 5.4 Use Case: Tool Format Selection

The `TrackAndStopBandit` implements the `FormatBandit` trait for adaptive
tool-format selection. For each `(model, role, tool_count, complexity)` key,
the bandit identifies the best tool format (JSON, XML, native function calling)
with high confidence, then stops exploring permanently for that key.

```rust
pub trait FormatBandit: Send + Sync {
    fn select_format(&self, key: &BanditKey) -> ToolFormat;
    fn update_format(&self, key: &BanditKey, format: ToolFormat, outcome: &ToolOutcome);
}
```

### 5.5 Why Track-and-Stop Instead of UCB1?

UCB1 never stops exploring. For decisions where:

1. The optimal choice is fixed (the best tool format for a given model).
2. Exploration has a cost (suboptimal formats waste tokens and cause parse
   errors).
3. High confidence is required, not just low cumulative regret.

Track-and-Stop explores only as much as needed, then commits permanently.

---

## 6. BanditBank: Keyed Collections

The `BanditBank` manages a collection of independent `UcbBandit` instances
keyed by context string:

```
BanditBank {
    "implementer:rust:standard" -> UcbBandit { arms: [claude, codex, gemini] }
    "reviewer:rust:complex"     -> UcbBandit { arms: [claude, codex, gemini] }
    "planner:python:fast"       -> UcbBandit { arms: [claude, codex, gemini] }
}
```

Bandits are created lazily: when a `select(key, ...)` call arrives for a key
that does not exist, a new `UcbBandit` is initialized with all available arms
and zero observations. This ensures that new context keys start with full
exploration before converging.

### 6.1 Persistence

The entire bank is serialized to a single JSON file. Each bandit's arm stats
are included, so the system resumes with full history on restart.

---

## 7. Reward Scaling Across Bandits

All bandit implementations assume rewards in [0, 1]:

| Signal | Reward Value |
|--------|-------------|
| Gate pass (first attempt) | 1.0 |
| Gate pass (after retry) | 0.7 |
| Gate fail (recoverable) | 0.2 |
| Gate fail (unrecoverable) | 0.0 |
| Cost efficiency | 1.0 - (cost / max_cost) |

For the cascade router, rewards are typically binary (1.0 for gate pass, 0.0
for fail) with a cost adjustment that penalizes expensive successes.

Track-and-Stop assumes sub-Gaussian rewards with parameter sigma = 0.5. The
GLR stopping criterion uses this assumption for threshold calibration.

---

## 8. Neural Contextual Bandits (Target Extension)

Linear contextual bandits (LinUCB) assume a linear relationship between
context features and reward. When the true reward function is nonlinear --
e.g., interaction effects between task complexity and crate familiarity --
LinUCB's regret grows. Neural contextual bandits replace the linear model with
a neural network.

### 8.1 NeuralUCB (Zhou et al. 2020)

NeuralUCB extends LinUCB by replacing the linear predictor with a neural
network and deriving an exploration bonus from the network's gradient:

```
For each arm a:
    predicted_reward = f(context; theta)       -- neural network forward pass
    gradient = nabla_theta f(context; theta)   -- backprop to get gradient
    exploration_bonus = nu * sqrt(gradient^T * Z_a^{-1} * gradient)
    score(a) = predicted_reward + exploration_bonus

where Z_a = sum_t g_t * g_t^T + lambda * I  (gradient covariance)
```

### 8.2 When to Use Neural vs Linear

| Criterion | LinUCB | NeuralUCB |
|-----------|--------|-----------|
| Context dimension | Low (<= 20) | Any |
| Reward structure | Approximately linear | Nonlinear interactions |
| Sample efficiency | Higher (fewer params) | Lower (needs ~500+ obs) |
| Computational cost | O(d^2) per update | O(network_size) per update |
| Interpretability | High (weight per feature) | Low (black box) |
| Cold start | Better (fewer params) | Worse (needs more data) |

**Roko recommendation:** Use LinUCB (current) until 500+ observations
accumulate and prediction residuals show nonlinear structure.

---

## 9. Bandit Ensembles and Meta-Selection

When multiple bandit algorithms are available, the question arises: which
bandit should we use? Meta-bandits solve this by treating the choice of
algorithm as itself a bandit problem.

### 9.1 Meta-Selection Algorithm

```
On each routing decision:
    1. meta_bandit.select() -> choose strategy_i
    2. arm = strategy_i.select(context)
    3. Execute arm, observe reward
    4. strategy_i.update(arm, reward)
    5. meta_bandit.update(strategy_i, reward)

The meta-bandit learns which strategy works best:
    - Stationary environment     -> UCB1 or LinUCB dominate
    - Non-stationary environment -> Thompson+drift dominates
    - High-dimensional context   -> NeuralUCB dominates
    - Low data regime            -> UCB1 dominates
```

### 9.2 Adaptive Strategy Switching

```
Every 50 decisions:
    for each strategy:
        regret_estimate = optimal_arm_reward * selections - cumulative_reward
        regret_rate = regret_estimate / selections
    if current_strategy.regret_rate > regret_threshold:
        switch to strategy with lowest regret_rate
```

### 9.3 Correlated Arms and Diversification

When strategies are correlated (they tend to select the same arm), the
ensemble provides little benefit. The correlation matrix tracks per-pair
agreement rates:

```
correlation(strategy_i, strategy_j) =
    count(both_select_same_arm) / count(both_queried)
```

If correlation > 0.9, the strategies are redundant. If correlation < 0.3, the
strategies provide genuine diversity.

---

## 10. Visualization and Debugging

### 10.1 Arm Performance Dashboard

```
+-------------------------------------------------------------+
| Cascade Router -- Stage 3 (LinUCB, 347 observations)         |
+-------------------------------------------------------------+
| Arm                 Pulls  Reward  UCB Score  Pass%  $/task  |
| claude-haiku-4.5      89   71.2    0.837      80%   $0.12   |
| claude-sonnet-4      156  108.0    0.812      69%   $0.95   |
| claude-opus-4        102   89.0    0.891      87%   $2.40   |
|                                                               |
| Exploration rate: 12% (target: 10-15%)                        |
| Hysteresis blocks: 23 (since last switch)                     |
| Current best: claude-opus-4 (score: 0.891)                    |
| Pareto frontier: [haiku, opus] (sonnet dominated)             |
+-------------------------------------------------------------+
```

### 10.2 Regret Trajectory

```
Cumulative Regret
    |
 40 |                                              / theoretical sqrt(T ln T)
    |                                           /
 30 |                                        /
    |                                ////
 20 |                          ////
    |                   /////    <- actual regret
 10 |            /////
    |     /////
  0 +--------------------------------------------> Decisions
    0      50     100     150     200     250
```

If actual regret exceeds the theoretical bound, the bandit is misconfigured.

### 10.3 Context Feature Importance (LinUCB)

The learned weight vector theta_a reveals which context features matter most:

```
LinUCB Feature Importance (averaged across arms):
    complexity_band:    ####################  0.42 (most important)
    has_prior_failure:  ##############        0.28
    crate_familiarity:  ###########           0.23
    iteration:          ########              0.17
    cache_affinity:     ######                0.12
    task_category[3]:   ####                  0.08
    bias_term:          ###                   0.06
```

Features with near-zero importance across all arms are candidates for removal.

### 10.4 Anomaly Detection

```rust
pub enum BanditAnomaly {
    /// One arm selected > 80% of the time.
    ArmLockIn { arm: String, selection_rate: f64 },
    /// Exploration rate dropped below 5% before convergence.
    PrematureExploitation { exploration_rate: f64, observations: u64 },
    /// Regret growing faster than theoretical bound.
    SuperlinearRegret { actual: f64, bound: f64 },
    /// Arm performance suddenly changed (provider update).
    ArmPerformanceShift { arm: String, old_rate: f64, new_rate: f64 },
    /// All arms have similar performance.
    IndistinguishableArms { max_gap: f64 },
}
```

These anomalies are surfaced in the TUI dashboard and can trigger automatic
corrective actions (e.g., resetting an arm on `ArmPerformanceShift`, increasing
exploration on `PrematureExploitation`).

---

## 11. Persistence

| Component | Format | Path |
|-----------|--------|------|
| `UcbBandit` | JSON (arm stats) | Per-bandit file |
| `BanditBank` | JSON (all bandits) | Single file |
| `LinUCBRouter` | JSON (A matrices, b vectors, obs count) | `.roko/learn/cascade-router.json` |
| `TrackAndStopBandit` | JSON (per-key state) | Per-instance file |

All persistence uses the atomic tempfile+rename pattern for crash safety.

---

## 12. Mathematical Summary

### 12.1 UCB1 Selection

```
a* = argmax_a [ mu_hat_a + sqrt(2) * sqrt(ln(T) / n_a) ]

mu_hat_a = (1/n_a) * sum_{t: a_t = a} r_t
```

### 12.2 Thompson Beta Posterior

```
theta_a ~ Beta(alpha_a, beta_a)

alpha_a = 1 + sum_{t: a_t = a} r_t
beta_a  = 1 + sum_{t: a_t = a} (1 - r_t)

E[theta_a] = alpha_a / (alpha_a + beta_a)
Var[theta_a] = (alpha_a * beta_a) / ((alpha_a + beta_a)^2 * (alpha_a + beta_a + 1))
```

### 12.3 Thompson with Discount

```
alpha_a <- gamma * alpha_a + r_t
beta_a  <- gamma * beta_a  + (1 - r_t)

N_eff = (1 - gamma^T) / (1 - gamma)  ->  1 / (1 - gamma)  as T -> inf
```

### 12.4 LinUCB Selection

```
a* = argmax_a [ theta_a^T * x + alpha * sqrt(x^T * A_a^{-1} * x) ]

theta_a = A_a^{-1} * b_a
A_a = I_d + sum_{t: a_t = a} x_t * x_t^T
b_a = sum_{t: a_t = a} r_t * x_t
```

### 12.5 LinUCB Incremental Update (Sherman-Morrison)

```
A^{-1}_{new} = A^{-1}_{old}
             - (A^{-1}_{old} * x * x^T * A^{-1}_{old})
               / (1 + x^T * A^{-1}_{old} * x)
```

### 12.6 Track-and-Stop GLR

```
GLR(t) = t * KL(mu_hat_1, mu_hat_2)

KL(p, q) = p * ln(p/q) + (1-p) * ln((1-p)/(1-q))

Stop when: GLR(t) > ln((ln(t) + 1) / delta)
```

### 12.7 Alpha Decay (LinUCB)

```
alpha(t) = 0.05 + 0.95 * exp(-t / 60)
```

### 12.8 Regret Bounds Summary

| Algorithm | Regret Bound | Tightness |
|-----------|-------------|-----------|
| UCB1 | O(sqrt(K * T * ln T)) | Near-optimal (log factor) |
| Thompson Beta | O(sqrt(K * T * ln T)) | Empirically tighter |
| LinUCB | O(d * sqrt(T * ln(KTL/delta))) | Exploits context structure |
| Track-and-Stop | O(T*(mu) * ln(1/delta)) sample complexity | Optimal for identification |
