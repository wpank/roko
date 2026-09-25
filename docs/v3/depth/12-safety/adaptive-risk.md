# Adaptive Risk Management

> **v3 depth file** -- `/docs/v3/depth/12-safety/adaptive-risk.md`
> Canonical source: v1 `docs/v1/11-safety/09-adaptive-risk.md`
> Status: **Wired**. `SafetyBudget`, `SafetyBudgetTracker`, `OperationalConfidenceTracker`,
> Kelly criterion position sizing, and confidence-modulated effective limits are live in
> `roko-agent/src/safety/risk.rs`. The five-layer adaptive risk model operates as T0
> (deterministic Rust, no LLM calls, zero inference cost per tick).

---

## 1. The Five Risk Layers

Every layer operates as T0 deterministic Rust. The LLM proposes actions; the risk
engine disposes. Layers are ordered by increasing cost and decreasing speed:

| Layer | Name | What it does | Enforcement point |
|---|---|---|---|
| 1 | Hard Shields | Immutable constraints | Pre-execution gate |
| 2 | Position Sizing | Kelly-criterion allocation | Pre-execution adjustment |
| 3 | Adaptive Guardrails | Bayesian trust expansion/contraction | Pre- and post-turn |
| 4 | Health Observation | Anomaly detection, health scoring | Post-turn monitoring |
| 5 | Domain Threat Detection | Domain-specific threats | Pre- and post-execution |

---

## 2. Layer 1: Hard Shields

Hard shields are immutable constraints that cannot be relaxed by any runtime mechanism:

### SafetyBudget

```rust
pub struct SafetyBudget {
    pub irreversibility_limit: f64,       // Total irreversibility score
    pub blast_radius_file_limit: usize,   // Maximum unique files touched
    pub footprint_limit: usize,           // Maximum tool-like external actions
    pub uncertainty_tokens: usize,        // Low-confidence decisions allowed
    pub cost_limit_usd: f64,              // Dollar-cost ceiling
}
```

Defaults: 10.0 irreversibility, 50 files, 500 footprint, 10 uncertainty tokens,
$50.00 cost. These limits apply per session or per task, configurable via `roko.toml`.

### Budget dimensions

Five dimensions are tracked independently:

| Dimension | What it bounds | Default |
|---|---|---|
| Irreversibility | Cumulative score of hard-to-revert changes | 10.0 |
| BlastRadius | Unique files or artifacts touched | 50 |
| Footprint | Total tool calls and external actions | 500 |
| Uncertainty | Low-confidence decisions (< threshold) | 10 |
| Cost | Estimated or actual dollar cost | $50.00 |

The `SafetyBudgetTracker` tracks running usage and provides atomic check-and-consume:

```rust
pub fn check_and_consume(&mut self, action: &ProposedAction) -> BudgetCheckResult {
    let result = self.check(action);
    if matches!(result, BudgetCheckResult::WithinBudget) {
        self.consume(&CompletedAction::from(action));
    }
    result
}
```

If any dimension is exceeded, the tracker returns `BudgetCheckResult::Exceeded(dimension)`
and the action is blocked.

### Irreversibility scoring

Each tool call receives a deterministic irreversibility score:

| Tool | Score | Rationale |
|---|---|---|
| read_file, glob, grep | 0.0 | Pure reads |
| write_file | 0.2 | Overwritable |
| edit_file | 0.3 | Partially reversible via git |
| git_commit | 0.3 | Reversible via git |
| git_push | 0.6 | Externally visible |
| cargo publish | 0.9 | Essentially permanent |
| rm -rf | 0.8 | Destructive |

---

## 3. Layer 2: Kelly Criterion Position Sizing

Position sizing adapts the scope of actions based on the agent's historical win rate.
The Kelly criterion (Kelly, 1956) computes the optimal fraction of available budget to
allocate to a single action:

```
f* = (p * b - q) / b
```

Where:

- `p` = win rate (gate pass rate).
- `q` = 1 - p (failure rate).
- `b` = payoff ratio (value of success / cost of failure).

```rust
pub fn kelly_fraction(win_rate: f64, payoff_ratio: f64) -> f64 {
    if payoff_ratio <= 0.0 {
        return 0.0;
    }
    let p = win_rate.clamp(0.0, 1.0);
    let q = 1.0 - p;
    let fraction = (p * payoff_ratio - q) / payoff_ratio;
    fraction.clamp(0.0, 1.0)
}
```

A negative Kelly fraction (negative edge) returns 0.0: the agent should not act at all.
This provides an automatic circuit-breaker for agents with poor track records.

### Confidence multiplier

The `confidence_multiplier()` function blends the Kelly fraction with a sigmoid baseline
to produce a smooth allocation curve:

```rust
pub fn confidence_multiplier(confidence: f64) -> f64 {
    let sigmoid = 1.0 / (1.0 + (-10.0 * (confidence - 0.5)).exp());
    let baseline = 0.1 + 0.4 * sigmoid;
    let kelly = kelly_fraction(confidence, 2.0);
    baseline.min(kelly.max(0.1))
}
```

At low confidence (0.1), the multiplier is approximately 0.1: only 10% of budget is
available. At high confidence (0.9), it rises to approximately 0.4. The Kelly cap
prevents overallocation when confidence is high but payoff is poor.

### Berkenkamp-inspired safe exploration

The adaptive risk framework draws from Berkenkamp et al. (2017), "Safe Model-based
Reinforcement Learning with Stability Guarantees," which proves that a learner can
safely expand its operating region while maintaining stability guarantees. Roko
instantiates this principle through the confidence tracker: the agent starts with a
pessimistic prior and expands its effective limits only after accumulating evidence
of successful operation.

The key property from Berkenkamp: the region of safe operation grows monotonically with
evidence but never exceeds the hard shield limits. In Roko, the `effective_limit()`
function modulates the hard shield through the Kelly-informed confidence multiplier,
ensuring that the effective limit is always <= the configured hard limit.

---

## 4. Layer 3: Adaptive Guardrails (Bayesian Trust)

### OperationalConfidenceTracker

The `OperationalConfidenceTracker` maintains per-dimension Beta distributions:

```rust
pub struct OperationalConfidenceTracker {
    pub dimensions: HashMap<String, BetaDistribution>,
    pub failure_weight: f64,
}
```

Each dimension starts with a pessimistic prior (Beta(1, 3), mean 0.25) and updates
asymmetrically: successes add 1.0 to alpha, failures add `failure_weight` (default
1.5) to beta. The asymmetry means that failures have more impact than successes,
implementing the precautionary principle.

### Composite confidence

The composite confidence is the geometric mean of lower 95% credible interval bounds:

```rust
pub fn composite_confidence(&self) -> f64 {
    let product: f64 = self.dimensions.values()
        .map(|d| d.lower_95().max(0.001))
        .product();
    product.powf(1.0 / self.dimensions.len() as f64)
}
```

The geometric mean ensures that a single low-confidence dimension drags down the
overall confidence. This prevents an agent from compensating for poor tool execution
with high plan quality -- the weakest dimension dominates.

### Effective limits under confidence

The `effective_limit()` function computes the actual operating limit:

```rust
pub fn effective_limit(
    hard_shield_limit: f64,
    confidence: f64,
    failure_rate: f64,
    task_complexity: f64,
    domain_risk: f64,
) -> f64 {
    let kelly = kelly_fraction(confidence, 2.0);
    let base_multiplier = 0.2 + 0.8 * kelly.max(confidence * 0.5);
    let failure_factor = 1.0 - (failure_rate * 0.5).min(0.8);
    let complexity_factor = 1.0 - (task_complexity * 0.3).min(0.6);
    let risk_factor = 1.0 - (domain_risk * 0.4).min(0.7);
    hard_shield_limit * base_multiplier * failure_factor * complexity_factor * risk_factor
}
```

Four multiplicative factors reduce the hard limit:

1. **Base multiplier**: Kelly-informed scaling with confidence.
2. **Failure factor**: penalizes recent gate failures.
3. **Complexity factor**: reduces budget for complex tasks.
4. **Risk factor**: domain-specific risk discount.

All factors are clamped to prevent the effective limit from reaching zero or exceeding
the hard limit.

---

## 5. Layer 4: Health Observation

Layer 4 monitors agent health trends via efficiency events and the conductor's
diagnosis engine:

- **Efficiency degradation**: compares recent efficiency metrics against baselines.
- **Ghost turn detection**: identifies turns with no meaningful output.
- **Phase stuck detection**: flags agents in the same phase too long.
- **Circuit breaker**: transitions from Closed to Half-Open to Open based on health.

See `loop-detection.md` for the circuit breaker and ghost turn details.

---

## 6. Layer 5: Domain Threat Detection

Layer 5 provides domain-specific threat detection:

- **Code domain**: supply chain analysis, dependency vulnerability scanning, secret
  detection in tool outputs.
- **Chain domain**: MEV detection (see `mev-protection.md`), slippage monitoring,
  gas price anomalies.
- **Research domain**: citation fabrication detection, source reliability scoring.

---

## 7. Integration with Adaptive Gate Thresholds

The adaptive risk system feeds into the gate threshold EMA:

- Gate pass rates update the `OperationalConfidenceTracker` dimensions.
- The confidence tracker's composite score modulates `effective_limit()`.
- Effective limits influence how aggressively the agent operates.
- More aggressive operation produces either more passes (confirming trust) or more
  failures (tightening constraints).

This creates a self-regulating feedback loop: an agent that fails gates gets harder
constraints, which either force better behavior or halt the agent entirely.

---

## Academic References

| Paper | Contribution |
|---|---|
| Kelly (1956), "A New Interpretation of Information Rate" | Optimal position sizing |
| Berkenkamp et al. (2017), "Safe Model-based Reinforcement Learning with Stability Guarantees" | Safe exploration with monotonic region growth |
| Nygard (2018), "Release It!" | Circuit breaker pattern |
| Thompson (1933), "On the Likelihood that One Unknown Probability Exceeds Another" | Beta distribution for Bayesian confidence |

---

## Implementation References

| Component | Location |
|---|---|
| SafetyBudget | `crates/roko-agent/src/safety/risk.rs` |
| SafetyBudgetTracker | `crates/roko-agent/src/safety/risk.rs` |
| OperationalConfidenceTracker | `crates/roko-agent/src/safety/risk.rs` |
| BetaDistribution | `crates/roko-agent/src/safety/risk.rs` |
| kelly_fraction | `crates/roko-agent/src/safety/risk.rs` |
| confidence_multiplier | `crates/roko-agent/src/safety/risk.rs` |
| effective_limit | `crates/roko-agent/src/safety/risk.rs` |
| irreversibility_score | `crates/roko-agent/src/safety/risk.rs` |
| Adaptive gate thresholds | `.roko/learn/gate-thresholds.json` |
