# Predictive Foraging and Active Inference

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 12

---

## The Core Loop

```
1. PREDICT    ->  Oracle.predict(query, ctx) -> Prediction
2. ACT        ->  Agent.execute(action) -> output
3. VERIFY     ->  Gate.verify(output) -> Engram (ground truth)
4. RESOLVE    ->  Oracle.evaluate(prediction, outcome) -> PredictionAccuracy
5. CORRECT    ->  ResidualCorrector.update(model, category, residual)
6. CALIBRATE  ->  CalibrationTracker.update(model, category, accuracy)
7. FEEDBACK   ->  Router.feedback(model, accuracy) -> updated bandit arms
8. LEARN      ->  Neuro.store(pattern) -> knowledge entry
```

Steps 5-8: ~50 nanoseconds total.

## PredictionClaim

```rust
pub struct PredictionClaim {
    pub engram: Engram,
    pub prediction: Prediction,
    pub registered_at_ms: i64,   // registered BEFORE action
    pub status: ClaimStatus,
}

pub enum ClaimStatus {
    Pending,
    Resolved(PredictionAccuracy),
    Expired,
}
```

The claim is stored before action execution, preventing retrodiction.

## ResidualCorrector

```rust
pub struct ResidualCorrector {
    biases: DashMap<(String, String), ExponentialMovingAverage>,
    alpha: f64,  // 0.1
}

impl ResidualCorrector {
    pub fn correct(&self, model: &str, category: &str, raw_value: f64) -> f64 {
        match self.biases.get(&(model.into(), category.into())) {
            Some(ema) => raw_value - ema.current(),
            None => raw_value,
        }
    }

    pub fn update(&self, model: &str, category: &str, residual: f64) {
        self.biases
            .entry((model.into(), category.into()))
            .or_insert_with(|| ExponentialMovingAverage::new(self.alpha))
            .value_mut()
            .update(residual);
    }

    pub fn bias(&self, model: &str, category: &str) -> f64 {
        self.biases.get(&(model.into(), category.into()))
            .map(|ema| ema.current())
            .unwrap_or(0.0)
    }
}
```

Cost breakdown: HashMap lookup ~20ns, EMA update ~10ns, subtraction ~1ns,
cache overhead ~19ns. Total: ~50ns per correction.

## CalibrationTracker

```rust
pub struct CalibrationTracker {
    stats: DashMap<(String, String), CalibrationStats>,
}

pub struct CalibrationStats {
    pub mean_residual: ExponentialMovingAverage,
    pub mean_absolute_error: ExponentialMovingAverage,
    pub interval_coverage: ExponentialMovingAverage,
    pub count: u64,
    pub trend: TrendEstimator,
}

impl CalibrationTracker {
    pub fn calibrated_confidence(&self, model: &str, category: &str) -> f64 {
        self.stats.get(&(model.into(), category.into()))
            .map(|s| 1.0 - s.mean_absolute_error.current())
            .unwrap_or(0.5)
    }

    pub fn accuracy_trend(&self, model: &str, category: &str) -> f64 {
        self.stats.get(&(model.into(), category.into()))
            .map(|s| s.trend.slope())
            .unwrap_or(0.0)
    }
}
```

## Active Inference State Space

### Factorized discrete POMDP

```rust
pub struct ActiveInferenceState {
    pub beliefs: Array3<f64>,   // [6, 5, 3] = 90 states
    pub model: GenerativeModel,
}

pub struct GenerativeModel {
    pub a: Array4<f64>,        // observation likelihood P(o | s)
    pub b: Vec<Array6<f64>>,   // transition dynamics P(s' | s, a)
    pub c: Array1<f64>,        // preferred observations (goal)
    pub d: Array3<f64>,        // initial state prior
}
```

State space factors:
- Task complexity: 6 levels (trivial, simple, moderate, complex, expert, research)
- Information state: 5 levels (blind, partial, adequate, comprehensive, complete)
- Confidence state: 3 levels (low, medium, high)
- Total: 90 states

### Expected Free Energy (EFE)

```rust
pub fn expected_free_energy(
    beliefs: &Array3<f64>,
    action: usize,
    model: &GenerativeModel,
) -> EfeDecomposition {
    let predicted_state = apply_transition(beliefs, action, &model.b[action]);
    let pragmatic = compute_pragmatic_value(&predicted_state, &model.c, &model.a);
    let epistemic = compute_epistemic_value(&predicted_state, beliefs, &model.a);
    let ambiguity = compute_ambiguity(&predicted_state, &model.a);
    EfeDecomposition { total: pragmatic + epistemic - ambiguity, pragmatic, epistemic, ambiguity }
}
```

### EFE as VCG bid

```rust
pub fn efe_to_bid(efe: &EfeDecomposition, pad: &PadState, section_type: &str) -> f64 {
    let urgency = pad.arousal.max(0.1);
    let exploration = 1.0 - pad.dominance;
    let failure_boost = (1.0 - pad.pleasure).max(0.0);
    match section_type {
        "prediction_context" => efe.epistemic * (1.0 + exploration),
        "task_context" => efe.pragmatic * urgency,
        "failure_memory" => efe.pragmatic * failure_boost,
        "knowledge" => efe.epistemic * exploration,
        _ => efe.total,
    }.max(0.0)
}
```

## Context Foraging Stopping Rule

Charnov's MVT (1976):

```rust
pub struct ContextForager {
    gain_rate: ExponentialMovingAverage,
    patches: HashMap<String, ContextPatch>,
    budget_remaining: usize,
}

pub struct ContextPatch {
    pub id: String,
    pub items: Vec<Engram>,
    pub marginal_gain: f64,
    pub item_cost: usize,
    pub retrieved: usize,
}

impl ContextForager {
    pub fn should_continue(&self) -> bool {
        let best_patch = self.patches.values()
            .max_by(|a, b| {
                let r_a = a.marginal_gain / a.item_cost as f64;
                let r_b = b.marginal_gain / b.item_cost as f64;
                r_a.partial_cmp(&r_b).unwrap()
            });
        match best_patch {
            Some(patch) => {
                patch.marginal_gain / patch.item_cost as f64 > self.gain_rate.current()
                    && self.budget_remaining > patch.item_cost
            }
            None => false,
        }
    }
}
```

## Thompson Sampling for Oracle Selection

```rust
pub struct ThompsonArm {
    pub oracle_id: String,
    pub alpha: f64,   // successes + 1
    pub beta: f64,    // failures + 1
}

impl ThompsonArm {
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        Beta::new(self.alpha, self.beta).unwrap().sample(rng)
    }

    pub fn update(&mut self, accuracy: f64, threshold: f64) {
        if accuracy > threshold { self.alpha += 1.0; } else { self.beta += 1.0; }
    }
}
```

For non-stationary environments: f-dsw variant (Raj & Kalyani, 2017) adds
discounting to forget old observations.

## Three Cognitive Speeds

| Speed | Activity |
|---|---|
| **Gamma** | T0 probes evaluate error scalar. No prediction resolution. Cost: us. |
| **Theta** | Predictions resolved. Residuals computed. CalibrationTracker updated. EMA adjusted. |
| **Delta** | Cross-model calibration. Thompson arms updated. Strategy consolidation in Dreams. |

## Academic Foundations

- Friston, K. (2010). "The free-energy principle." *Nature Reviews Neuroscience*, 11(2).
- Charnov, E. L. (1976). "Optimal foraging." *Theoretical Population Biology*, 9.
- Pirolli, P., & Card, S. (1999). "Information foraging." *Psychological Review*, 106(4).
- Thompson, W. R. (1933). *Biometrika*, 25(3-4).
- Raj, V., & Kalyani, S. (2017). arXiv:1707.09727.
- Conant, R. C., & Ashby, W. R. (1970). *International Journal of Systems Science*, 1(2).
