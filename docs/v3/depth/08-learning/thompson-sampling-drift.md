# 08-learning/13 -- Thompson Sampling with Drift

> Non-stationary bandit selection via discounted Beta posteriors. The discount
> factor gamma down-weights old observations so the posterior tracks changing
> model quality without manual intervention.

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 4.2

**Source:** `crates/roko-learn/src/bandits.rs` (ThompsonArm, Beta posterior),
`crates/roko-learn/src/cascade_router.rs` (integration with stage-2/3
routing)

**Academic basis:** Thompson, W.R. (1933). On the likelihood that one
unknown probability exceeds another in view of the evidence of two samples.
*Biometrika* 25(3-4), 285-294. Garivier, A. & Moulines, E. (2011). On
Upper-Confidence Bound Policies for Switching Bandit Problems. *ALT 2011*,
LNAI 6925, 174-188.

---

## 1. Purpose

Standard bandit algorithms (UCB1, LinUCB) assume a stationary reward
distribution: the expected reward of each arm does not change over time. In
model routing, stationarity is systematically violated by four forces:

| Force | Example | Timescale |
|-------|---------|-----------|
| Provider updates | Anthropic deploys a new Sonnet checkpoint | Days to weeks |
| Codebase evolution | Crate structure changes after refactoring plan | Per-plan |
| Task mix shift | Development phase moves from scaffolding to optimization | Per-session |
| Cache dynamics | Repeated access patterns improve effective latency | Per-episode |

When the reward distribution drifts, stationary algorithms accumulate
misleading historical evidence. A model that was excellent three months ago
may be mediocre after a provider update, but its strong historical record
keeps UCB1 selecting it. Thompson Sampling with a discount factor addresses
this by geometrically down-weighting old observations.

---

## 2. Standard Thompson Sampling

For each arm `a` with binary reward (pass/fail), maintain Beta distribution
parameters `(alpha_a, beta_a)`:

```
Prior:   Beta(1, 1) = Uniform(0, 1)

Selection:
    For each arm a:
        sample theta_a ~ Beta(alpha_a, beta_a)
    Select arm with highest sample.

Update on reward r in {0, 1}:
    alpha_a <- alpha_a + r
    beta_a  <- beta_a  + (1 - r)
```

The Beta distribution is the conjugate prior for Bernoulli observations,
giving a closed-form posterior update:

```
Posterior mean     = alpha / (alpha + beta)
Posterior variance = (alpha * beta) / ((alpha + beta)^2 * (alpha + beta + 1))
```

Arms with few observations have wide posteriors (high variance, frequent
sampling of extreme values), producing natural exploration. Arms with many
observations have narrow posteriors (low variance, samples cluster near the
mean), producing exploitation. No explicit exploration parameter is needed --
the posterior width serves this role automatically.

---

## 3. Adding Drift: Discount Factor

To handle non-stationarity, apply a discount factor gamma in (0, 1) to
existing observations before incorporating the new observation:

```
On update for arm a with reward r:
    alpha_a <- gamma * alpha_a + r
    beta_a  <- gamma * beta_a  + (1 - r)
```

The discount factor shrinks the existing evidence before adding the new
observation. After many updates, the effective observation count stabilizes
at approximately `1 / (1 - gamma)` regardless of actual history length.

### 3.1 Effective Window

The effective window -- the number of past observations that meaningfully
influence the current posterior -- is approximately `1 / (1 - gamma)`. After
`n` observations, the weight of the oldest observation is `gamma^n`. When
this weight drops below 0.01 (1% contribution), the observation is
effectively forgotten:

```
n_effective = ln(0.01) / ln(gamma) = -4.605 / ln(gamma)
```

| gamma | Effective window | n_effective (1% threshold) | Behavior |
|-------|------------------|---------------------------|----------|
| 0.999 | ~1000 | 4603 | Very slow forgetting, near-stationary |
| 0.995 | ~200 | 919 | Moderate forgetting (recommended) |
| 0.99 | ~100 | 460 | Responsive forgetting |
| 0.95 | ~20 | 90 | Fast forgetting |
| 0.90 | ~10 | 44 | Aggressive forgetting |

### 3.2 Parameter Floor

Heavy discounting can drive both alpha and beta toward zero, producing a
degenerate Beta distribution. A floor of 0.01 on both parameters prevents
this:

```rust
fn select(arms: &[ThompsonArm], rng: &mut impl Rng) -> usize {
    arms.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let sample_a = Beta::new(
                a.alpha.max(0.01),
                a.beta.max(0.01),
            ).sample(rng);
            let sample_b = Beta::new(
                b.alpha.max(0.01),
                b.beta.max(0.01),
            ).sample(rng);
            sample_a.partial_cmp(&sample_b).unwrap()
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}
```

The floor ensures that even after extensive discounting, the sampler
produces valid Beta samples rather than NaN or zero-probability draws.

---

## 4. Implementation Design

### 4.1 Per-Arm State

```rust
struct ThompsonArm {
    /// Model slug (e.g. "claude-sonnet-4-20250514").
    model: String,
    /// Beta distribution alpha parameter (discounted successes + prior).
    alpha: f64,
    /// Beta distribution beta parameter (discounted failures + prior).
    beta: f64,
    /// Total observations (not discounted, for diagnostics only).
    total_observations: u64,
}
```

The `total_observations` counter is never discounted. It serves a diagnostic
purpose: comparing `total_observations` against the effective alpha + beta
reveals the degree to which historical evidence has been forgotten.

### 4.2 Update with Discount

```rust
fn update(arm: &mut ThompsonArm, reward: f64, gamma: f64) {
    arm.alpha = gamma * arm.alpha + reward;
    arm.beta = gamma * arm.beta + (1.0 - reward);
    arm.total_observations += 1;
}
```

The reward is typically binary (1.0 for gate pass, 0.0 for gate fail) but
can be continuous in [0, 1] for graded outcomes:

| Outcome | Reward |
|---------|--------|
| Gate pass (first attempt) | 1.0 |
| Gate pass (after retry) | 0.7 |
| Gate fail (recoverable) | 0.2 |
| Gate fail (unrecoverable) | 0.0 |

---

## 5. Recommended Configuration

For model routing in Roko, the recommended discount factor is **gamma =
0.995** (effective window ~200 observations). This balances three concerns:

1. **Responsiveness:** Detects model quality changes within ~50 observations
   of the change. After a provider update degrades one model's performance,
   the degraded model's posterior widens within 50 episodes, allowing
   alternatives to be sampled more frequently.

2. **Stability:** Does not overreact to short-term noise from individual
   task outcomes. A single spurious failure does not dramatically shift the
   posterior because the discount preserves ~200 observations of context.

3. **Cold start:** After 200 observations, the system has effectively
   forgotten its cold-start period (where routing was dominated by the static
   table's conservative defaults) and responds only to post-bootstrap
   performance.

### 5.1 Comparison with UCB1 and LinUCB

| Property | UCB1 | LinUCB | Thompson + Drift |
|----------|------|--------|-----------------|
| Context-dependent | No | Yes (18-dim) | No (per-arm) |
| Non-stationary | No | No | Yes (gamma discount) |
| Exploration strategy | Deterministic (upper bound) | Deterministic (upper bound) | Stochastic (sampling) |
| Regret bound | O(sqrt(T ln T)) | O(d * sqrt(T ln T)) | O(sqrt(T / (1 - gamma))) |
| Cold start | Infinite UCB for unpulled | Static fallback | Wide Beta prior |
| Reproducibility | Deterministic | Deterministic | Stochastic (seed-dependent) |

### 5.2 When to Use Each Algorithm

- **UCB1** -- stationary decisions where the optimal choice does not change:
  tool format selection, retry strategy, fallback ordering.
- **LinUCB** -- decisions where a context vector (task category, role,
  complexity) strongly influences the optimal choice, under approximate
  stationarity.
- **Thompson + Drift** -- decisions where the optimal choice shifts with
  provider updates, codebase evolution, or task mix changes. Preferred when
  non-stationarity dominates context effects.

The cascade router currently uses LinUCB in stage 3. Thompson Sampling with
drift is the recommended alternative for stage 3 in environments with
frequent model provider updates (Garivier & Moulines 2011).

---

## 6. Drift Detection and Reset

Thompson Sampling with discount handles **gradual drift** automatically.
For **abrupt changes** (e.g., a provider deploys a breaking update that
immediately degrades quality), an additional reset mechanism triggers
re-exploration:

```
If recent_pass_rate(last 10) << historical_pass_rate(last 100):
    Reset arm: alpha <- 1, beta <- 1 (uninformative prior)
    -> Full re-exploration for this arm
```

This combines circuit breaker anomaly detection with Thompson arm state:
when `ProviderHealthRegistry` detects a provider degradation (circuit
breaker transitions to Open), the corresponding Thompson arm's posterior
is reset to the uninformative prior Beta(1, 1), forcing the system to
re-evaluate the model from scratch once the circuit breaker transitions
back to HalfOpen.

### 6.1 Reset vs Discount

| Mechanism | Handles | Failure mode |
|-----------|---------|-------------|
| Discount (gamma) | Gradual drift | Slow response to sudden changes |
| Reset (prior reinit) | Sudden change | Unnecessary exploration if the change was temporary |

Both mechanisms are complementary. The discount factor handles the common
case (gradual quality evolution); the reset handles the rare case (sudden
provider update or outage).

---

## 7. Interaction with Stability Mechanisms

Thompson Sampling's stochastic selection naturally provides exploration, but
combined with the cascade router's hysteresis mechanism (10% score delta to
switch models), it can create oscillation between near-equal models:

```
Current model: claude-sonnet-4 (sampled theta = 0.82)
Challenger: claude-opus-4 (sampled theta = 0.85)
Delta: 0.85 - 0.82 = 0.03 < 0.10 (hysteresis threshold)
-> Keep current model (no switch)
```

The hysteresis threshold acts as a damper: even though stochastic sampling
may produce a higher sample for the challenger, the system only switches
when the sampled advantage exceeds 10%. This prevents the stochastic nature
of Thompson Sampling from causing rapid model switching between near-equal
performers.

See [stability-mechanisms.md](stability-mechanisms.md) for the full
hysteresis design and its interaction with frequency separation.

---

## 8. Adaptive Discount Factor

Rather than fixing gamma, the system can adapt it based on observed
non-stationarity:

```
If arm_switching_rate > 0.20:
    gamma <- max(0.90, gamma - 0.01)    // Increase forgetting
If arm_switching_rate < 0.05:
    gamma <- min(0.999, gamma + 0.01)   // Decrease forgetting
```

The arm switching rate is the fraction of consecutive selection decisions
where the chosen arm changes. High switching indicates rapid environmental
change (gamma should decrease to forget faster). Low switching indicates
stability (gamma should increase to retain more history).

This adaptive approach ensures the discount factor tracks the actual rate
of environmental change rather than relying on a fixed prior assumption
about non-stationarity.

---

## 9. Contextual Thompson Sampling

Thompson Sampling can be extended with context features, creating a Bayesian
analogue to LinUCB:

```
Prior:     theta_a ~ N(mu_0, Sigma_0)
Posterior: theta_a | (x_1, r_1), ..., (x_t, r_t)
Selection: sample theta_a from posterior, compute score = theta_a^T * x
```

Contextual Thompson Sampling provides both exploration via posterior
uncertainty (like Thompson) and context-dependent scoring (like LinUCB).

| Criterion | LinUCB | Contextual Thompson |
|-----------|--------|---------------------|
| Stationary environment | Preferred | Either |
| Non-stationary environment | Poor | Preferred (with discount) |
| Deterministic exploration | Yes | No (stochastic) |
| Posterior uncertainty | Point estimate + bound | Full distribution |
| Computational cost | Lower (matrix inverse) | Higher (sampling) |

For Roko's model routing, LinUCB is currently preferred because the 18-dim
context space is well-suited to linear models and deterministic exploration
provides reproducible routing for debugging. Contextual Thompson would be
adopted when non-stationarity creates significant performance degradation
that LinUCB cannot track.

---

## 10. Monitoring and Diagnostics

### 10.1 Drift Monitoring

The system can detect when Thompson Sampling with drift would outperform
LinUCB by monitoring:

1. **Prediction error trend** -- if the cascade router's predictions degrade
   over time, the environment is non-stationary.
2. **Arm switching frequency** -- if the bandit switches arms more than 20%
   of the time, the reward landscape is changing.
3. **Calibration drift** -- if calibration error increases monotonically, the
   model quality distribution is shifting.

### 10.2 Diagnostic Output

```
Model: claude-sonnet-4
  alpha: 45.2  beta: 12.8  mean: 0.779  total_obs: 342
  effective alpha+beta: 58.0  (gamma has discounted 284 observations)

Model: claude-opus-4
  alpha: 38.1  beta: 5.3   mean: 0.878  total_obs: 189
  effective alpha+beta: 43.4  (gamma has discounted 145.6 observations)
```

The gap between `total_observations` and effective `alpha + beta` reveals
how much historical evidence has been forgotten by the discount factor.

---

## References

- Thompson, W.R. (1933). On the likelihood that one unknown probability
  exceeds another in view of the evidence of two samples. *Biometrika*
  25(3-4), 285-294.
- Garivier, A. & Moulines, E. (2011). On Upper-Confidence Bound Policies
  for Switching Bandit Problems. *ALT 2011*, LNAI 6925, 174-188.
- Auer, P., Cesa-Bianchi, N. & Fischer, P. (2002). Finite-time analysis
  of the multiarmed bandit problem. *Machine Learning* 47(2-3), 235-256.
