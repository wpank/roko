# 08-learning/05 -- Cascade Router

> Three-stage model selection system: Static (< 50 obs), Confidence
> (50-200 obs), and LinUCB contextual bandit (> 200 obs), with provider
> health filtering, Pareto pruning, C-Factor bias, and calibration.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/cascade_router.rs`

**Persistence:** `.roko/learn/cascade-router.json`

**Cross-references:** [bandits-ucb-thompson-linucb](bandits-ucb-thompson-linucb.md),
[provider-health-circuit-breaker](provider-health-circuit-breaker.md),
[pareto-frontier-pruning](pareto-frontier-pruning.md),
[cost-normalization](cost-normalization.md)

---

## 1. Purpose

The cascade router is Roko's central model selection system. It answers the
question: "Given a task with these features (category, complexity, role,
iteration, crate familiarity), which LLM model should run it?" The answer
evolves as the system accumulates observations, transitioning through three
stages of increasing sophistication.

The cascade design is inspired by production routing systems (RouteLLM, Ong
et al. ICLR 2025; FrugalGPT, Chen et al. arXiv:2305.05176; AutoMix, NeurIPS
2024) but adapted for a self-hosted development tool where the reward signal
(gate pass/fail) is deterministic and the decision space is small enough for
contextual bandits rather than neural routers.

---

## 2. Three-Stage Cascade

```
+-----------------------------+-----------------------------------------+
|  Stage 1: Static            |  < 50 observations                      |
|  Hardcoded role->model      |  No learning, safe defaults              |
|  table                      |                                          |
+-----------------------------+-----------------------------------------+
|  Stage 2: Confidence        |  50 -- 200 observations                  |
|  Empirical pass rates +     |  Simple statistics, wide confidence      |
|  confidence intervals       |  intervals shrink with data              |
+-----------------------------+-----------------------------------------+
|  Stage 3: UCB               |  > 200 observations                      |
|  Full LinUCB contextual     |  Context-dependent routing with          |
|  bandit                     |  learned feature weights                  |
+-----------------------------+-----------------------------------------+
```

### 2.1 Why Three Stages?

A single bandit algorithm works poorly at all scales:

- **Too few observations for UCB**: LinUCB with 18 context dimensions needs
  ~50 observations per arm to begin producing meaningful weights. With 5
  models, that is 250+ observations before the bandit is useful. During cold
  start, random exploration wastes money.
- **Too crude for static forever**: A hardcoded table cannot adapt to
  crate-specific patterns, role-specific model preferences, or changes in
  model capabilities after a provider update.
- **Confidence stage bridges the gap**: Between 50 and 200 observations,
  simple pass-rate statistics with confidence intervals provide reasonable
  routing without the sample complexity of a 18-dimensional linear model.

---

## 3. Stage 1: Static Routing (< 50 Observations)

Hardcoded mapping from `ModelTier` to model slug:

```rust
fn static_route(tier: ModelTier) -> ModelSpec {
    match tier {
        ModelTier::Fast    => ModelSpec::new("claude-haiku-4-5-20251001"),
        ModelTier::Standard => ModelSpec::new("claude-sonnet-4-20250514"),
        ModelTier::Complex => ModelSpec::new("claude-opus-4-20250514"),
    }
}
```

The mapping is deliberately conservative: it over-routes to stronger models to
avoid gate failures during cold-start, accepting higher cost in exchange for
higher pass rates while the system builds its observation base.

---

## 4. Stage 2: Confidence Routing (50-200 Observations)

### 4.1 Per-Model Statistics

```rust
struct ModelStats {
    trials: u64,
    successes: u64,
}
```

### 4.2 Selection Algorithm

For each candidate model:

```
score(model) = pass_rate(model) - cost_penalty(model) + affinity_bonus(model)
```

where:

- `cost_penalty` = normalized cost relative to the cheapest available model
- `affinity_bonus` = `CACHE_AFFINITY_BONUS` (0.15) if the model matches the
  previous task's model

Additional biases from C-Factor and affect system:

- **Low affect confidence** (< 0.3): bias toward stronger models.
- **High C-Factor** (> 0.8): bias toward cheaper models (system performing
  well, can save).
- **Low C-Factor** (< 0.4): bias toward stronger models (system struggling,
  need quality).

### 4.3 Transition Threshold

`CONFIDENCE_TO_UCB_THRESHOLD = 200` observations triggers transition to stage
3. With 5 models, this gives ~40 per model -- LinUCB needs ~2x the dimension
count (36+) per arm for stable weights.

---

## 5. Stage 3: UCB Routing (> 200 Observations)

At 200+ observations, the full `LinUCBRouter` contextual bandit takes over.
See [bandits-ucb-thompson-linucb](bandits-ucb-thompson-linucb.md) for the
algorithm. The LinUCB stage uses the 18-dimensional `RoutingContext` to make
context-dependent decisions. The router can learn patterns such as:

- "For `roko-core` crate with high familiarity, haiku is sufficient."
- "For cross-crate refactoring on retry (iteration > 0), use opus."
- "When the previous model was sonnet and it failed, escalate to opus."

---

## 6. CascadeModel Output

```rust
pub struct CascadeModel {
    pub primary: ModelSpec,
    pub fallback: Option<ModelSpec>,
    pub latency_sla_ms: u64,
    pub stage: CascadeStage,
}
```

The `fallback` field provides a pre-computed escalation target. If the primary
fails, the orchestrator can immediately retry with the fallback without
re-querying the router.

---

## 7. Provider Health Integration

```
CascadeRouter::select(context)
    |
    +-- 1. Compute candidate scores (per stage algorithm)
    |
    +-- 2. Filter: ProviderHealthRegistry::is_available(model.provider)
    |       -> Remove models whose provider circuit breaker is Open
    |
    +-- 3. Filter: Pareto frontier pruning
    |       -> Remove dominated models (worse on both cost and quality)
    |
    +-- 4. Select highest-scoring non-filtered model
```

See [provider-health-circuit-breaker](provider-health-circuit-breaker.md) and
[pareto-frontier-pruning](pareto-frontier-pruning.md).

---

## 8. C-Factor Integration

```rust
pub enum AgentDispatchBias {
    PreferStronger,   // C-Factor < 0.4 -- system struggling
    PreferCheaper,    // C-Factor > 0.8 -- system performing well
    Neutral,          // C-Factor 0.4-0.8 -- no bias
}
```

The C-Factor is computed from a composite of gate pass rate, cost efficiency,
speed, first-try rate, knowledge growth, and turn-taking equality across recent
episodes.

---

## 9. Router Calibration

### 9.1 Expected Calibration Error (ECE)

```
ECE = sum_{b=1}^{B} (n_b / N) * |accuracy_b - confidence_b|
```

| ECE Range | Interpretation | Action |
|-----------|---------------|--------|
| < 0.05 | Well calibrated | No action |
| 0.05-0.10 | Slightly miscalibrated | Monitor |
| 0.10-0.20 | Miscalibrated | Apply Platt scaling |
| > 0.20 | Severely miscalibrated | Investigate distribution shift |

### 9.2 Platt Scaling

```
calibrated_probability = sigmoid(a * raw_score + b)
```

Parameters `a` and `b` are fit by minimizing log-loss on a held-out validation
set. Fast (O(n) fitting), requires ~50 samples.

### 9.3 Auto-Recalibration

```
Every 100 routing decisions:
    1. Collect last 200 (predicted, actual) pairs
    2. Compute ECE
    3. If ECE > 0.10:
       a. Fit Platt scaling via gradient descent
       b. Validate on held-out 20%
       c. Apply if improved, else fit isotonic regression
    4. Log to .roko/learn/calibration.jsonl
```

---

## 10. Persistence

State is persisted to `.roko/learn/cascade-router.json`:

```json
{
  "observations": 347,
  "stage": "ucb",
  "model_stats": {
    "claude-haiku-4-5-20251001": { "trials": 89, "successes": 71 },
    "claude-sonnet-4-20250514": { "trials": 156, "successes": 108 },
    "claude-opus-4-20250514": { "trials": 102, "successes": 89 }
  },
  "linucb_state": { "arms": { "..." }, "observation_count": 347 },
  "pareto_frontier": ["claude-haiku-4-5-20251001", "claude-opus-4-20250514"]
}
```

Loaded on startup, saved after each routing update. Atomic tempfile+rename.

---

## 11. Operating Frequency

The cascade router operates at **per-episode frequency** -- every agent turn
produces one routing update. This is the highest-frequency learning loop in
the system.
