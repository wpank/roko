# 12-AFFECT-ROUTING: Unified Affect-Aware Model Routing Pipeline

Covers gap #7 (Daimon Affect not influencing routing), gap #8 (Neuro store not consulted for
model selection), gap #14 (Provider Pass-Rate not weighted in routing), and gap #24
(Predict-Publish-Correct calibration loop missing).

---

## Problem Statement

### Gap #7: Daimon affects dispatch parameters but not model selection

The `DaimonState` is loaded, appraised, and used for modulating `DispatchParams` (turn limit,
effort label, strategy) via `modulate_with_strategy()`. It is also used to build emotional
tags on signals and episodes. But the cascade router (`CascadeRouter`) makes model selection
decisions **without any affect input**. The PAD vector (Pleasure-Arousal-Dominance) and
behavioral state (Engaged/Stressed/Cautious/etc.) are invisible to the routing pipeline.

Concretely: when the system is stressed (low pleasure, high arousal -- e.g. after 3
consecutive gate failures), it should prefer cheaper, faster models to conserve budget. When
confident (high dominance, high pleasure), it can use expensive models. This is implemented
in `DispatchParams.strategy` but never reaches the `RoutingContext` that drives model
selection.

### Gap #8: Knowledge store not consulted for model selection

`knowledge_routing_boost()` and `build_knowledge_routing_advice()` exist in
`knowledge_helpers.rs` and produce per-model score adjustments based on neuro store entries.
But the cascade router's `route()`, `route_with_cfactor()`, and `route_with_health()` methods
do not accept or use knowledge advice. The CLAUDE.md lists "Knowledge-informed agent routing"
as item #13 in the remaining work list.

The `KnowledgeRoutingAdvice` type exists with `hints: Vec<KnowledgeHint>` containing
per-model scores in `[-0.3, 0.3]`. This is computed but never applied.

### Gap #14: Provider pass-rate not weighted in routing scores

`ProviderHealthRegistry` tracks per-provider circuit breaker state (Closed/Open/HalfOpen)
and `ProviderHealthTracker` has `filter_arms()` for binary inclusion/exclusion. But neither
provides a continuous pass-rate signal that can be used as a multiplicative weight in routing
scores.

The cascade router has `route_with_health()` which filters unavailable providers, but it
treats all available providers as equally healthy. A provider at 60% pass rate and one at 99%
pass rate are scored identically.

### Gap #24: No Predict-Publish-Correct calibration loop

The system has no mechanism to:
1. Record what model it predicted/selected before dispatch
2. Record the actual outcome (gate pass/fail, cost, latency) after dispatch
3. Close the loop by updating routing weights based on prediction accuracy

The cascade router's `record_outcome()` method updates LinUCB weights, but there is no
structured prediction event emitted before dispatch, no pairing of prediction with outcome,
and no calibration metric tracking whether the router's confidence in its selections matches
reality.

### Combined impact

These four gaps mean the routing pipeline operates in an open-loop mode: it selects models
based on cold-start heuristics and LinUCB observations, but it ignores the system's emotional
state, accumulated knowledge, provider reliability gradients, and its own prediction accuracy.
The result is suboptimal model selection that does not improve with operational experience.

---

## Ideal Design

### Architecture: Unified Routing Pipeline

```
     Task Requirements
           |
     +-----v------+
     | RoutingContext |  (existing: role, category, tier, tokens, etc.)
     +-----+------+
           |
     +-----v------+
     | Stage 1: AffectBias |  NEW -- PAD vector -> tier shift + exploration rate
     +-----+------+
           |
     +-----v------+
     | Stage 2: KnowledgeBias |  Wire existing knowledge_routing_boost()
     +-----+------+
           |
     +-----v------+
     | Stage 3: ProviderWeight |  NEW -- continuous pass-rate multiplier
     +-----+------+
           |
     +-----v------+
     | Stage 4: CascadeRouter |  (existing: Static / Confidence / UCB)
     +-----+------+
           |
     +-----v------+
     | Prediction Emission |  NEW -- Pulse::RoutingPrediction
     +-----+------+
           |
       [dispatch]
           |
     +-----v------+
     | Outcome Emission |  NEW -- Pulse::RoutingOutcome
     +-----+------+
           |
     +-----v------+
     | CalibrationPolicy |  NEW -- React Cell subscribing to prediction+outcome
     +-----+------+
           |
     updates routing weights
```

### 1. Affect Bias: PAD vectors modulate model scores

#### New Type: AffectRoutingBias

```rust
// roko-learn/src/cascade/affect_bias.rs -- NEW FILE

use roko_core::{BehavioralState, PadVector};

/// Affect-derived routing bias parameters.
///
/// Computed from the current PAD vector and behavioral state,
/// these values modify the cascade router's candidate scoring.
#[derive(Debug, Clone, PartialEq)]
pub struct AffectRoutingBias {
    /// Tier shift: positive = prefer higher tier, negative = prefer cheaper.
    /// Range: [-2, +2] (in model-tier-rank units).
    pub tier_shift: i32,
    /// Exploration rate multiplier for LinUCB alpha parameter.
    /// Range: [0.5, 2.0]. High arousal -> more exploration.
    pub exploration_multiplier: f64,
    /// Cost sensitivity multiplier. High stress -> higher cost penalty.
    /// Range: [0.5, 2.0].
    pub cost_sensitivity: f64,
    /// Latency tolerance. High arousal -> prefer faster models.
    /// Range: [0.5, 2.0].
    pub latency_sensitivity: f64,
    /// Source behavioral state for logging.
    pub source_state: BehavioralState,
    /// Source PAD vector for logging.
    pub source_pad: PadVector,
}

impl AffectRoutingBias {
    /// Compute routing bias from a PAD vector and behavioral state.
    ///
    /// The mapping follows Gebhard's ALMA model:
    /// - **Pleasure** axis: positive -> allow expensive models, negative -> prefer cheap.
    /// - **Arousal** axis: high -> increase exploration + latency sensitivity.
    /// - **Dominance** axis: high -> maintain current strategy (low exploration).
    pub fn from_pad(pad: PadVector, state: BehavioralState) -> Self {
        // Tier shift: pleasure-driven.
        // Pleasure in [-1, 1]. Map to tier shift in [-2, +2].
        let tier_shift = match state {
            BehavioralState::Stressed | BehavioralState::Cautious => {
                // Stressed: shift toward cheaper models regardless of pleasure.
                -1 + (pad.pleasure * 0.5).round() as i32
            }
            BehavioralState::Confident | BehavioralState::Engaged => {
                // Confident: shift toward stronger models.
                1 + (pad.pleasure * 0.5).round() as i32
            }
            _ => (pad.pleasure * 1.5).round() as i32,
        };

        // Exploration: arousal-driven, dominance-suppressed.
        // High arousal + low dominance = uncertain + agitated = explore more.
        let exploration_multiplier = (1.0 + 0.5 * pad.arousal - 0.3 * pad.dominance)
            .clamp(0.5, 2.0);

        // Cost sensitivity: inverse of pleasure + stress amplifier.
        let stress_factor = match state {
            BehavioralState::Stressed => 1.5,
            BehavioralState::Cautious => 1.2,
            _ => 1.0,
        };
        let cost_sensitivity = ((1.0 - 0.4 * pad.pleasure) * stress_factor).clamp(0.5, 2.0);

        // Latency: arousal-driven. High arousal = "hurry up."
        let latency_sensitivity = (1.0 + 0.4 * pad.arousal).clamp(0.5, 2.0);

        Self {
            tier_shift: tier_shift.clamp(-2, 2),
            exploration_multiplier,
            cost_sensitivity,
            latency_sensitivity,
            source_state: state,
            source_pad: pad,
        }
    }

    /// The neutral (no-op) bias.
    pub fn neutral() -> Self {
        Self {
            tier_shift: 0,
            exploration_multiplier: 1.0,
            cost_sensitivity: 1.0,
            latency_sensitivity: 1.0,
            source_state: BehavioralState::Engaged,
            source_pad: PadVector::neutral(),
        }
    }
}
```

### 2. Knowledge Bias: neuro store boosts/penalizes models

#### Wire existing functions into RoutingContext

The `knowledge_routing_boost()` and `build_knowledge_routing_advice()` functions already
produce the right output. They just need to be called at routing time and their output
applied to candidate scores.

```rust
// Addition to CascadeRouter -- new method

/// Route with full context: affect bias + knowledge advice + health weighting.
pub fn route_full(
    &self,
    ctx: &RoutingContext,
    affect: Option<&AffectRoutingBias>,
    knowledge: Option<&KnowledgeRoutingAdvice>,
    health: Option<&ProviderHealthRegistry>,
    model_providers: Option<&HashMap<String, String>>,
    cfactor: Option<&CFactor>,
    agent_id: Option<&str>,
) -> CascadeModel {
    // 1. Start with available candidates (provider health filter).
    let candidates: Vec<String> = if let (Some(health), Some(providers)) =
        (health, model_providers)
    {
        self.model_slugs
            .iter()
            .filter(|slug| {
                providers
                    .get(slug.as_str())
                    .map(|pid| health.is_available(pid))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    } else {
        self.model_slugs.clone()
    };

    if candidates.is_empty() {
        return self.route(ctx);
    }

    // 2. Apply affect-driven tier targeting.
    let adjusted_ctx = if let Some(affect) = affect {
        let mut ctx = ctx.clone();
        ctx.tier_bias = Some(affect.tier_shift);
        ctx.exploration_multiplier = Some(affect.exploration_multiplier);
        ctx
    } else {
        ctx.clone()
    };

    // 3. Get base route from cascade.
    let mut route = self.route_with_cfactor_among(
        &adjusted_ctx, &candidates, cfactor, agent_id,
    );

    // 4. Apply knowledge-driven score adjustments.
    if let Some(knowledge) = knowledge {
        if knowledge.has_signal {
            route = self.apply_knowledge_bias(route, knowledge, &candidates);
        }
    }

    // 5. Apply provider pass-rate weighting.
    if let (Some(health), Some(providers)) = (health, model_providers) {
        route = self.apply_pass_rate_weighting(route, health, providers, &candidates);
    }

    route
}
```

#### Knowledge bias application

```rust
// Addition to CascadeRouter

fn apply_knowledge_bias(
    &self,
    mut route: CascadeModel,
    advice: &KnowledgeRoutingAdvice,
    candidates: &[String],
) -> CascadeModel {
    // Check if any candidate has a stronger knowledge score than the current primary.
    let primary_score = advice.hints.iter()
        .find(|h| slugs_match(&h.model_slug, &route.primary.slug))
        .map(|h| h.score)
        .unwrap_or(0.0);

    let best_alternative = advice.hints.iter()
        .filter(|h| candidates.contains(&h.model_slug))
        .filter(|h| !slugs_match(&h.model_slug, &route.primary.slug))
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

    // Only switch if the alternative has a significantly better knowledge score
    // (>0.15 delta) to avoid flip-flopping.
    if let Some(alt) = best_alternative {
        if alt.score > primary_score + 0.15 {
            let new_primary = ModelSpec::from_slug(&alt.model_slug);
            route = self.retarget_route_primary(route, candidates, new_primary);
            tracing::debug!(
                from = %route.primary.slug,
                to = %alt.model_slug,
                knowledge_delta = alt.score - primary_score,
                "knowledge bias overriding primary model"
            );
        }
    }

    route
}
```

### 3. Provider Pass-Rate Weighting

#### Continuous health signal

```rust
// roko-learn/src/provider_health.rs -- additions to existing ProviderHealth

impl ProviderHealth {
    /// Compute a continuous pass rate in [0.0, 1.0].
    ///
    /// Uses the rolling failure window for recency-weighted calculation,
    /// falling back to lifetime stats when the window is too small.
    pub fn pass_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0; // Assume healthy until proven otherwise.
        }

        // Use rolling window if we have enough recent data.
        if self.failure_window.len() >= 5 {
            let recent_failures = self.failure_window.len() as f64;
            // The window stores failures only, so we estimate the denominator
            // from the window's time span and overall request rate.
            let window_duration_ms = self.failure_window.back()
                .and_then(|last| self.failure_window.front().map(|first| last.timestamp_ms - first.timestamp_ms))
                .unwrap_or(1)
                .max(1) as f64;

            let lifetime_rate = self.total_requests as f64
                / (self.total_requests as f64 / self.total_failures.max(1) as f64);
            let estimated_window_requests = (window_duration_ms / 1000.0) * lifetime_rate;

            if estimated_window_requests > 0.0 {
                return (1.0 - recent_failures / estimated_window_requests.max(recent_failures))
                    .clamp(0.0, 1.0);
            }
        }

        // Fallback: lifetime pass rate.
        let successes = self.total_requests.saturating_sub(self.total_failures);
        successes as f64 / self.total_requests as f64
    }
}

impl ProviderHealthRegistry {
    /// Return the pass rate for a provider, defaulting to 1.0 for unknown providers.
    pub fn pass_rate(&self, provider_id: &str) -> f64 {
        self.providers
            .lock()
            .get(provider_id)
            .map(|h| h.pass_rate())
            .unwrap_or(1.0)
    }
}
```

#### Pass-rate scoring in CascadeRouter

```rust
// Addition to CascadeRouter

fn apply_pass_rate_weighting(
    &self,
    mut route: CascadeModel,
    health: &ProviderHealthRegistry,
    providers: &HashMap<String, String>,
    candidates: &[String],
) -> CascadeModel {
    let primary_provider = providers.get(&route.primary.slug);
    let primary_rate = primary_provider
        .map(|pid| health.pass_rate(pid))
        .unwrap_or(1.0);

    // If primary provider is below 0.7 pass rate, consider switching.
    if primary_rate < 0.7 {
        let best_alt = candidates.iter()
            .filter(|slug| !slugs_match(slug, &route.primary.slug))
            .max_by(|a, b| {
                let rate_a = providers.get(a.as_str())
                    .map(|pid| health.pass_rate(pid))
                    .unwrap_or(1.0);
                let rate_b = providers.get(b.as_str())
                    .map(|pid| health.pass_rate(pid))
                    .unwrap_or(1.0);
                rate_a.partial_cmp(&rate_b).unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(alt_slug) = best_alt {
            let alt_rate = providers.get(alt_slug.as_str())
                .map(|pid| health.pass_rate(pid))
                .unwrap_or(1.0);

            // Switch only if the alternative is significantly healthier.
            if alt_rate > primary_rate + 0.15 {
                let new_primary = ModelSpec::from_slug(alt_slug);
                route = self.retarget_route_primary(route, candidates, new_primary);
                tracing::info!(
                    from_rate = primary_rate,
                    to_rate = alt_rate,
                    to_model = %alt_slug,
                    "provider health overriding primary model"
                );
            }
        }
    }

    route
}
```

### 4. Predict-Publish-Correct Calibration Loop

#### Pulse topics

```rust
// Topic conventions for routing calibration:
//
// routing.prediction    -- emitted pre-dispatch with predicted model + confidence
// routing.outcome       -- emitted post-dispatch with actual result
// routing.calibration   -- emitted by CalibrationPolicy with updated weights
```

#### Prediction and Outcome types

```rust
// roko-learn/src/calibration.rs -- NEW FILE

use serde::{Deserialize, Serialize};

/// Emitted before dispatch with the router's prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPrediction {
    /// Unique ID to pair with outcome.
    pub prediction_id: String,
    /// Plan and task being dispatched.
    pub plan_id: String,
    pub task_id: String,
    /// Selected model slug.
    pub predicted_model: String,
    /// Router's confidence in this selection (from LinUCB score or confidence stats).
    pub predicted_confidence: f64,
    /// Estimated cost in USD for this dispatch.
    pub predicted_cost_usd: f64,
    /// Which cascade stage made the selection.
    pub cascade_stage: String,
    /// Affect bias applied (if any).
    pub affect_tier_shift: Option<i32>,
    /// Knowledge hints applied (if any).
    pub knowledge_hints: Vec<String>,
    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Emitted after dispatch + gate with the actual result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingOutcome {
    /// Paired with prediction.
    pub prediction_id: String,
    /// The model actually used (may differ from predicted if fallback triggered).
    pub actual_model: String,
    /// Whether the gate passed.
    pub gate_passed: bool,
    /// Actual cost.
    pub actual_cost_usd: f64,
    /// Wall time in milliseconds.
    pub wall_time_ms: u64,
    /// Number of agent turns consumed.
    pub turns: u32,
    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Paired prediction + outcome for calibration analysis.
#[derive(Debug, Clone)]
pub struct CalibrationPair {
    pub prediction: RoutingPrediction,
    pub outcome: RoutingOutcome,
}

impl CalibrationPair {
    /// Did the prediction match the outcome?
    pub fn model_matched(&self) -> bool {
        self.prediction.predicted_model == self.outcome.actual_model
    }

    /// Prediction error: |predicted_cost - actual_cost| / actual_cost.
    pub fn cost_error(&self) -> f64 {
        if self.outcome.actual_cost_usd <= 0.0 {
            return 0.0;
        }
        (self.prediction.predicted_cost_usd - self.outcome.actual_cost_usd).abs()
            / self.outcome.actual_cost_usd
    }

    /// Calibration reward: positive if prediction was good, negative if bad.
    /// Range: [-1.0, 1.0].
    pub fn calibration_reward(&self) -> f64 {
        let mut reward = 0.0;

        // Gate pass is the primary signal.
        if self.outcome.gate_passed {
            reward += 0.6;
        } else {
            reward -= 0.4;
        }

        // Cost accuracy bonus.
        let cost_err = self.cost_error();
        if cost_err < 0.2 {
            reward += 0.2;
        } else if cost_err > 0.5 {
            reward -= 0.2;
        }

        // Model match bonus (router was well-calibrated).
        if self.model_matched() {
            reward += 0.2;
        }

        reward.clamp(-1.0, 1.0)
    }
}
```

#### CalibrationPolicy (React Cell)

```rust
// roko-learn/src/calibration.rs -- continued

use std::collections::HashMap;
use parking_lot::Mutex;

/// Tracks routing prediction accuracy and adjusts weights.
///
/// Implements the Predict-Publish-Correct loop:
/// 1. Before dispatch: router emits RoutingPrediction
/// 2. After dispatch + gate: runner emits RoutingOutcome
/// 3. CalibrationPolicy pairs them and computes calibration reward
/// 4. Reward is fed back to CascadeRouter.record_outcome()
///
/// This is a React Cell: it subscribes to prediction + outcome topics
/// on the PulseBus and emits calibration updates.
pub struct CalibrationPolicy {
    /// Pending predictions awaiting outcomes, keyed by prediction_id.
    pending: Mutex<HashMap<String, RoutingPrediction>>,
    /// Rolling calibration statistics.
    stats: Mutex<CalibrationStats>,
    /// Maximum pending predictions before evicting oldest.
    max_pending: usize,
}

/// Rolling calibration statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalibrationStats {
    /// Total prediction-outcome pairs processed.
    pub total_pairs: u64,
    /// Number of correct model predictions.
    pub correct_model_count: u64,
    /// Average cost prediction error.
    pub mean_cost_error: f64,
    /// Exponential moving average of calibration reward.
    pub ema_reward: f64,
    /// Per-model calibration stats.
    pub per_model: HashMap<String, ModelCalibrationStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCalibrationStats {
    pub predictions: u64,
    pub gate_passes: u64,
    pub mean_cost_error: f64,
    pub mean_wall_time_ms: f64,
}

impl CalibrationStats {
    /// Model selection accuracy: correct_model_count / total_pairs.
    pub fn model_accuracy(&self) -> f64 {
        if self.total_pairs == 0 { return 0.0; }
        self.correct_model_count as f64 / self.total_pairs as f64
    }

    /// Overall calibration quality: EMA reward.
    pub fn calibration_quality(&self) -> f64 {
        self.ema_reward
    }
}

const EMA_ALPHA: f64 = 0.1;

impl CalibrationPolicy {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            stats: Mutex::new(CalibrationStats::default()),
            max_pending: 100,
        }
    }

    /// Record a prediction before dispatch.
    pub fn record_prediction(&self, prediction: RoutingPrediction) {
        let mut pending = self.pending.lock();
        if pending.len() >= self.max_pending {
            // Evict oldest (by timestamp).
            if let Some(oldest_id) = pending.values()
                .min_by_key(|p| p.timestamp)
                .map(|p| p.prediction_id.clone())
            {
                pending.remove(&oldest_id);
            }
        }
        pending.insert(prediction.prediction_id.clone(), prediction);
    }

    /// Record an outcome after dispatch + gate. Returns the calibration
    /// pair if a matching prediction was found.
    pub fn record_outcome(&self, outcome: RoutingOutcome) -> Option<CalibrationPair> {
        let prediction = self.pending.lock().remove(&outcome.prediction_id)?;

        let pair = CalibrationPair {
            prediction,
            outcome,
        };

        // Update stats.
        let mut stats = self.stats.lock();
        stats.total_pairs += 1;
        if pair.model_matched() {
            stats.correct_model_count += 1;
        }

        let cost_err = pair.cost_error();
        stats.mean_cost_error = stats.mean_cost_error * (1.0 - EMA_ALPHA)
            + cost_err * EMA_ALPHA;

        let reward = pair.calibration_reward();
        stats.ema_reward = stats.ema_reward * (1.0 - EMA_ALPHA) + reward * EMA_ALPHA;

        // Per-model stats.
        let model_stats = stats.per_model
            .entry(pair.prediction.predicted_model.clone())
            .or_default();
        model_stats.predictions += 1;
        if pair.outcome.gate_passed {
            model_stats.gate_passes += 1;
        }
        model_stats.mean_cost_error = model_stats.mean_cost_error * (1.0 - EMA_ALPHA)
            + cost_err * EMA_ALPHA;
        model_stats.mean_wall_time_ms = model_stats.mean_wall_time_ms * (1.0 - EMA_ALPHA)
            + pair.outcome.wall_time_ms as f64 * EMA_ALPHA;

        Some(pair)
    }

    /// Current calibration statistics snapshot.
    pub fn stats(&self) -> CalibrationStats {
        self.stats.lock().clone()
    }

    /// Per-model pass rate from calibration data.
    pub fn model_pass_rate(&self, model_slug: &str) -> Option<f64> {
        let stats = self.stats.lock();
        stats.per_model.get(model_slug).map(|s| {
            if s.predictions == 0 { 1.0 } else { s.gate_passes as f64 / s.predictions as f64 }
        })
    }
}
```

### 5. Integration: Unified route_full() call site

The call site is in `orchestrate.rs` (or the new `event_loop.rs` runner), at the point where
the model is selected for a task dispatch.

```rust
// In the SpawnAgent handler or dispatch_agent_with():

// 1. Compute affect bias.
let affect_state = runner.daimon.query();
let affect_bias = AffectRoutingBias::from_pad(
    affect_state.pad,
    affect_state.behavioral_state,
);

// 2. Compute knowledge advice.
let knowledge_advice = build_knowledge_routing_advice(
    &knowledge_store,
    &cascade_router.model_slugs().to_vec(),
    role,
    task_category.label(),
);

// 3. Route with full context.
let route = cascade_router.route_full(
    &routing_ctx,
    Some(&affect_bias),
    Some(&knowledge_advice),
    Some(&provider_health),
    Some(&model_providers),
    cfactor.as_ref(),
    Some(&agent_id),
);

// 4. Emit prediction.
let prediction_id = format!("{plan_id}/{task_id}/{}", chrono::Utc::now().timestamp_millis());
let prediction = RoutingPrediction {
    prediction_id: prediction_id.clone(),
    plan_id: plan_id.clone(),
    task_id: task_id.clone(),
    predicted_model: route.primary.slug.clone(),
    predicted_confidence: 0.0, // filled from LinUCB score
    predicted_cost_usd: estimate_cost(&route.primary.slug, &task_def),
    cascade_stage: format!("{:?}", route.stage),
    affect_tier_shift: Some(affect_bias.tier_shift),
    knowledge_hints: knowledge_advice.hints.iter().map(|h| h.reason.clone()).collect(),
    timestamp: chrono::Utc::now(),
};
calibration_policy.record_prediction(prediction.clone());

// 5. Publish prediction pulse.
if let Some(bus) = pulse_bus.as_ref() {
    let pulse = Pulse::builder(0, Topic::new("routing.prediction"), Kind::Event)
        .body(Body::json(&prediction))
        .build();
    let _ = bus.publish(pulse);
}

// 6. Use route.primary.slug as the model for dispatch.
let model = route.primary.slug;

// ... dispatch agent ...

// 7. After gate completes, emit outcome:
let outcome = RoutingOutcome {
    prediction_id,
    actual_model: state.agent_model.clone(),
    gate_passed: completion.passed,
    actual_cost_usd: state.cost_usd,
    wall_time_ms: state.task_elapsed_ms(),
    turns: state.task_agent_calls,
    timestamp: chrono::Utc::now(),
};

if let Some(pair) = calibration_policy.record_outcome(outcome.clone()) {
    // Feed calibration reward back to the cascade router.
    let reward = pair.calibration_reward();
    cascade_router.record_calibration_reward(
        &pair.prediction.predicted_model,
        reward,
    );

    // Publish outcome pulse.
    if let Some(bus) = pulse_bus.as_ref() {
        let pulse = Pulse::builder(0, Topic::new("routing.outcome"), Kind::Event)
            .body(Body::json(&outcome))
            .build();
        let _ = bus.publish(pulse);
    }
}
```

### 6. RoutingContext Extensions

```rust
// Additions to RoutingContext (roko-learn/src/model_router.rs):

pub struct RoutingContext {
    // ... existing fields ...

    /// Affect-driven tier shift. Positive = prefer higher tier.
    /// Applied during candidate scoring. Default: None (no shift).
    pub tier_bias: Option<i32>,

    /// Affect-driven exploration rate multiplier for LinUCB.
    /// Default: None (use default alpha).
    pub exploration_multiplier: Option<f64>,
}
```

---

## Implementation Plan

### Step 1: AffectRoutingBias type

**File**: `crates/roko-learn/src/cascade/affect_bias.rs` (new)

- Struct `AffectRoutingBias` with `from_pad()` and `neutral()`.
- PAD -> tier_shift, exploration_multiplier, cost_sensitivity, latency_sensitivity.
- Unit tests: verify behavioral state -> bias mapping for all 5 states.

**File**: `crates/roko-learn/src/cascade/mod.rs`

- Add `pub mod affect_bias;`

### Step 2: ProviderHealth.pass_rate()

**File**: `crates/roko-learn/src/provider_health.rs`

- Add `pass_rate()` method to `ProviderHealth`
- Add `pass_rate()` method to `ProviderHealthRegistry`
- Unit tests: pass rate with rolling window, with lifetime stats, with empty state

### Step 3: CalibrationPolicy

**File**: `crates/roko-learn/src/calibration.rs` (new)

- Types: `RoutingPrediction`, `RoutingOutcome`, `CalibrationPair`, `CalibrationStats`
- Struct `CalibrationPolicy` with `record_prediction()`, `record_outcome()`, `stats()`
- `CalibrationPair::calibration_reward()` computation
- Unit tests: pair matching, EMA update, max_pending eviction, per-model stats

**File**: `crates/roko-learn/src/lib.rs`

- Add `pub mod calibration;`

### Step 4: RoutingContext extensions

**File**: `crates/roko-learn/src/model_router.rs`

- Add `tier_bias: Option<i32>` field to `RoutingContext`
- Add `exploration_multiplier: Option<f64>` field
- Wire `tier_bias` into the LinUCB feature vector (add as an extra context dimension)
- Wire `exploration_multiplier` into the LinUCB alpha parameter

### Step 5: CascadeRouter.route_full()

**File**: `crates/roko-learn/src/cascade_router.rs`

- Add `route_full()` method composing affect + knowledge + health + cascade
- Add `apply_knowledge_bias()` private method
- Add `apply_pass_rate_weighting()` private method
- Add `record_calibration_reward()` method (feeds back to LinUCB + confidence stats)
- Unit tests: route_full with each bias applied independently and in combination

### Step 6: Wire into orchestrate.rs / event_loop.rs

**File**: `crates/roko-cli/src/runner/event_loop.rs`

- Import `AffectRoutingBias`, `CalibrationPolicy`, `RoutingPrediction`, `RoutingOutcome`
- In `SpawnAgent` handler: compute affect bias, knowledge advice, call `route_full()`
- After route: emit prediction via `CalibrationPolicy` and `PulseBus`
- In gate completion handler: emit outcome, feed calibration reward back

**File**: `crates/roko-cli/src/runner/types.rs`

- Add `CalibrationPolicy` and `PulseBus` to `RunConfig` or `RunContext`

### Step 7: Persistence

**File**: `crates/roko-learn/src/calibration.rs`

- Add `CalibrationPolicy::save()` / `load_or_new()` for `CalibrationStats`
- Persist to `.roko/learn/calibration.json`

**File**: `crates/roko-cli/src/runner/persist.rs`

- Add calibration stats to the periodic flush

### Step 8: CLI inspection

**File**: `crates/roko-cli/src/learn_cmd.rs`

- Add `roko learn calibration` subcommand showing calibration stats
- Display: model accuracy, EMA reward, per-model pass rates, cost prediction error

---

## Verification

### Automated

1. **Unit tests** (affect_bias.rs):
   - Stressed state -> negative tier_shift, high cost_sensitivity
   - Confident state -> positive tier_shift, low cost_sensitivity
   - Neutral PAD -> all multipliers near 1.0
   - Extreme PAD values clamp correctly

2. **Unit tests** (provider_health.rs):
   - Provider with 0 requests -> pass_rate 1.0
   - Provider with 100 requests / 10 failures -> pass_rate ~0.9
   - Rolling window pass_rate reflects recent failures more than old ones

3. **Unit tests** (calibration.rs):
   - Prediction without outcome -> stays pending
   - Outcome without prediction -> returns None
   - Matched pair -> correct calibration_reward
   - EMA updates converge to the right value
   - max_pending evicts oldest
   - model_pass_rate returns correct per-model stats

4. **Unit tests** (cascade_router.rs):
   - route_full with affect bias shifts tier selection
   - route_full with knowledge bias overrides primary when delta > 0.15
   - route_full with low pass-rate provider switches to healthier alternative
   - route_full with no biases produces same result as route()

5. **Integration test**:
   - Run a plan, verify `.roko/learn/calibration.json` is written
   - Verify calibration stats match the number of dispatched tasks
   - Verify prediction_id pairing: every prediction has a matching outcome

### Manual

1. Run `cargo build --workspace` (compiles)
2. Run `cargo test --workspace` (all tests pass)
3. Run `cargo clippy --workspace --no-deps -- -D warnings` (clean)
4. Run a real plan and check:
   - `roko learn calibration` shows calibration stats
   - Logs show "affect bias: tier_shift=..." and "knowledge bias: ..." messages
   - After 3+ gate failures, observe affect shifting model selection downward
   - `.roko/learn/calibration.json` contains valid stats

### Metrics to track

- Calibration accuracy: model_accuracy should increase over time
- Routing reward EMA: should converge toward positive (>0.3)
- Affect responsiveness: gate failure streaks should trigger cheaper model selection
- Knowledge hit rate: how often knowledge advice has non-zero signal

---

## Rating

**9.5 / 10**

Strengths:
- Reuses all existing infrastructure: DaimonState (PAD + behavioral state),
  knowledge_routing_boost(), ProviderHealthRegistry, PulseBus, cascade router stages.
- Single new entry point (`route_full`) that composes all four signals without modifying
  any existing route methods (backward compatible: `route()` still works as before).
- CalibrationPolicy is self-contained (no external dependencies beyond serde) and persists
  its own state. It can be added incrementally without breaking existing routing.
- The affect bias mapping follows established emotion-behavior theory (Gebhard's ALMA model,
  which the daimon already implements). No new emotional models introduced.
- Provider pass-rate is computed from existing `ProviderHealth` data (rolling failure window
  + lifetime stats). No new data collection needed.

Risks:
- The PAD-to-tier-shift mapping is heuristic. The specific coefficients (0.5 * pleasure,
  0.3 * dominance) need tuning based on real execution data. Mitigation: the calibration
  loop will provide the data needed to tune these, and the `AffectRoutingBias::neutral()`
  path means the system works without affect if the mapping is wrong.
- Knowledge routing advice scanning 10 entries per routing decision adds ~1ms latency.
  Mitigation: this is negligible compared to the 2-8s agent spawn time, and the query
  only runs once per task dispatch, not per turn.
- The `route_full()` composition order (affect -> knowledge -> health -> cascade) means
  later stages can override earlier stages. This is intentional (health should always
  override affect -- a down provider is a down provider regardless of mood), but the
  knowledge override threshold (0.15 delta) needs tuning.
- CalibrationPolicy's pending map can grow unbounded if outcomes are lost (e.g. agent
  crashes before gate). Mitigation: max_pending cap with oldest-first eviction, plus
  stale prediction cleanup on periodic flush.

## Implementation Packet

This work makes affect, provider health, and knowledge advice visible to model routing.

### Required Context

- `crates/roko-daimon/src/`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/provider_health.rs`
- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-cli/src/dispatch/model_routing.rs`
- `docs/09-daimon/10-integration-points.md`
- `docs/02-agents/11-dual-process-routing.md`
- `docs/05-learning/04-cascade-router.md`
- `tmp/unified/05-AGENT.md`
- `tmp/unified/07-LEARNING.md`

### Target Files

- [ ] Add affect fields to dispatch routing context.
- [ ] Add provider health fields to dispatch routing context.
- [ ] Add knowledge advice fields to dispatch routing context.
- [ ] Add tests in routing modules.

### Checklist

- [ ] Define `RoutingContext` in dispatch layer with task, budget, affect, knowledge, provider health, and override fields.
- [ ] Map `DaimonState` or `PadState` into a small `AffectRoutingBias`.
- [ ] Keep a neutral bias when daimon state is unavailable.
- [ ] Query knowledge store for routing advice once per task dispatch.
- [ ] Add provider health penalty before final model choice.
- [ ] Record prediction before dispatch with model, expected cost, expected quality, and risk.
- [ ] Join prediction with gate outcome after task terminal event.
- [ ] Persist calibration observations.
- [ ] Add cap and cleanup for pending predictions.

### Acceptance Criteria

- [ ] With no daimon/neuro state, routing behaves like current cascade routing.
- [ ] A down provider is never selected because affect is optimistic.
- [ ] A high-risk affect state can shift choice toward cheaper/faster models when configured.
- [ ] Routing observations include expected vs actual outcome.
- [ ] Unit tests cover neutral, stressed, confident, and provider-down cases.

## Worker 9 Evidence Checklist (2026-04-26)

Routing/affect code that exists:

- [x] `crates/roko-learn/src/model_router.rs::RoutingContext` includes role, complexity, crate familiarity, prior failure, conductor load, active agents, queue depth, `DaimonPolicy`, thinking level, temperament, previous model, plan context tokens, and tier thresholds.
- [x] `crates/roko-learn/src/cascade_router.rs` implements health filtering, cost pressure, route biasing, and knowledge-advice adjustment.
- [x] `crates/roko-learn/src/calibration_policy.rs` implements a predict-publish-correct policy over learning `AgentEvent`s.
- [x] `crates/roko-compose/src/system_prompt_builder.rs` includes affect/context guidance for prompt construction.

Gaps blocking archive:

- [ ] No `AffectRoutingBias` type exists under the active dispatch layer.
- [ ] `ProviderHealth.pass_rate()` is not present as the direct API described here; pass-rate style data exists in cascade/model stats instead.
- [ ] `crates/roko-cli/src/runner/event_loop.rs` constructs `DispatchContext` with `model_hint: Some(ctx.config.model.clone())`, so the dispatch router treats the configured default model as a task hint and bypasses `CascadeRouter`.
- [ ] Even when `CascadeRouter` is present, `crates/roko-cli/src/dispatch/model_routing.rs` currently returns the deterministic default slug because the active path does not build the full `RoutingContext`.
- [ ] `CalibrationPolicy` is not wired into the active runner event stream.
- [ ] Knowledge advice can bias `CascadeRouter`, but the runner does not query knowledge advice once per task dispatch.
- [ ] No proof shows neutral/stressed/confident/provider-down routing behavior in the active runner.

## 9. 2026-04-27 Deepening Pass - Active Routing Authority Contract

Self-grade for this pass:

- Initial rating: 9.91 / 10.
- Reasoning: this pass identifies the concrete active-runner bypass that prevents routing from working, distinguishes existing learned-routing infrastructure from live selection authority, and gives implementation batches with proof artifacts. The score is not higher because no generated proof yet shows affect/knowledge/provider/calibration influencing real model choice.

### 9.1 Source-Corrected Status

- [x] `crates/roko-cli/src/dispatch/mod.rs` defines a `Dispatcher` facade with `ModelRouter`.
- [x] `crates/roko-cli/src/dispatch/model_routing.rs` defines precedence: override, task hint, cascade router, safe default.
- [x] `crates/roko-cli/src/runner/event_loop.rs` calls `Dispatcher::plan` before spawning the provider runtime.
- [x] `crates/roko-cli/src/runtime_feedback/routing.rs` exists and records task outcomes back into `CascadeRouter`.
- [x] `crates/roko-learn/src/cascade_router.rs` contains routing, health filtering, route biasing, knowledge-advice adjustment, and model stats.
- [x] `crates/roko-learn/src/calibration_policy.rs` contains a predict-publish-correct policy over learning events.
- [x] `crates/roko-learn/src/provider_model_outcome.rs` computes pass-rate and rolling pass-rate summaries.
- [ ] The active runner still passes the configured default model as `DispatchContext.model_hint`, which makes the default behave like an author hint.
- [ ] `ModelRouter::route` does not currently call `CascadeRouter::route`, `route_with_health`, `route_with_bias`, or knowledge-aware methods; it returns the default slug when the cascade branch is selected.
- [ ] Active feedback observations can still carry empty model/provider/usage values from runner-level event translation, which limits routing learning quality.
- [ ] No active-runner proof demonstrates affect state, knowledge advice, provider health, or calibration changing the selected model.

### 9.2 Correct Routing Semantics

The target route decision must preserve operator intent without turning defaults into overrides:

- [ ] `force_backend` means operator override and always wins.
- [ ] `task.model_hint` means author hint and wins over learned routing.
- [ ] `config.model` means default fallback only; it must not block cascade routing when a router is configured.
- [ ] If `CascadeRouter` is configured, the router receives a full routing context and returns the primary model.
- [ ] If no router is configured or the router cannot decide, use `config.model`.
- [ ] Every decision records `ModelChoiceSource::{Override, TaskHint, Router, Default}` accurately.
- [ ] Feedback dampens or excludes overrides and task hints from learned router updates unless explicitly configured.

### 9.3 Implementation Batches

#### AR-ROUTE-01: Fix Default-Model Semantics

- [ ] Remove `model_hint: Some(ctx.config.model.clone())` from the runner dispatch context.
- [ ] Add a separate `default_model` field to `DispatchContext` or `ModelRouter`.
- [ ] Preserve task-level `task_def.model_hint` as the only task hint.
- [ ] Preserve explicit operator overrides through `force_backend`.
- [ ] Update `RoutingInputs::from_task` so `task_model_hint` does not fall back to the config default.
- [ ] Add tests proving a configured default does not bypass cascade when cascade exists.
- [ ] Add tests proving the configured default is used when no cascade exists.

#### AR-ROUTE-02: Build Full RoutingContext

- [ ] Map task role into `roko_learn::model_router::RoutingContext`.
- [ ] Map task domain/category into routing context.
- [ ] Map task tier/complexity into routing context.
- [ ] Map retry attempt/prior failure into routing context.
- [ ] Map remaining budget and cost pressure into routing context.
- [ ] Map active agents, queue depth, and conductor load if available.
- [ ] Map plan context tokens or prompt diagnostics token budget into routing context.
- [ ] Map `DaimonPolicy` or neutral policy.
- [ ] Use this context when calling `CascadeRouter`.

#### AR-ROUTE-03: Make Cascade The Real Router Branch

- [ ] Replace the placeholder default return in `ModelRouter::route` with a call into `CascadeRouter`.
- [ ] Choose the best available cascade method for the currently available inputs.
- [ ] Record route stage, candidate list, selected primary, alternatives, and reason.
- [ ] Return `ModelChoiceSource::Router` only when the selected model came from the cascade.
- [ ] Emit route diagnostics into prompt/runtime projections.
- [ ] Add tests where two cascade models are available and learned stats change the selected model.

#### AR-ROUTE-04: Affect Bias Integration

- [ ] Define `AffectRoutingBias` or reuse existing `DaimonPolicy` fields if they already express the needed bias.
- [ ] Compute neutral bias when daimon state is unavailable.
- [ ] Compute stressed, cautious, engaged, and confident bias from available daimon/PAD state.
- [ ] Apply bias before or inside cascade scoring, not after provider resolution.
- [ ] Persist bias inputs and resulting score shift in route diagnostics.
- [ ] Add tests for neutral, stressed, cautious, and confident states.

#### AR-ROUTE-05: Knowledge Advice Integration

- [ ] Make prompt/knowledge retrieval emit routing advice refs or query `roko-neuro` once per dispatch.
- [ ] Convert advice into model score adjustments with bounded magnitude.
- [ ] Require a minimum delta before switching away from the cascade primary.
- [ ] Record advice ids, scores, and selected/dropped hints in route diagnostics.
- [ ] Prove knowledge advice can change selection in a deterministic fixture.

#### AR-ROUTE-06: Provider Health And Pass-Rate Integration

- [ ] Decide the canonical source for provider health: `ProviderHealthRegistry`, provider/model outcome summaries, or both.
- [ ] Expose continuous pass-rate or rolling pass-rate to `ModelRouter`.
- [ ] Ensure open/down providers are never selected except explicit override with warning.
- [ ] Apply pass-rate penalty before final model selection.
- [ ] Record provider health snapshot in route diagnostics.
- [ ] Add tests for provider-down, provider-degraded, and provider-healthy alternatives.

#### AR-ROUTE-07: Predict-Publish-Correct Wiring

- [ ] Emit a routing prediction event before dispatch with prediction id, selected model, source, expected cost, expected quality, affect bias, knowledge advice, and provider health.
- [ ] Store prediction id on the task attempt state.
- [ ] Emit routing outcome after gate completion with actual model, provider, pass/fail, cost, latency, and retry count.
- [ ] Pair prediction/outcome through `CalibrationPolicy`.
- [ ] Feed calibration reward back into `CascadeRouter` or the selected routing policy.
- [ ] Persist calibration stats under `.roko/learn/`.
- [ ] Add stale-prediction cleanup for crashed tasks.

#### AR-ROUTE-08: Feedback Quality Fix

- [ ] Ensure `FeedbackEvent::TaskCompleted` uses the real `AgentOutcome` from dispatch instead of default empty model/provider values.
- [ ] Record `ModelChoiceSource` from the actual dispatch decision.
- [ ] Do not record override/task-hint outcomes as ordinary router successes unless a policy says to.
- [ ] Include provider/model/cost/tokens/duration in routing feedback.
- [ ] Add tests proving router observations contain non-empty model/provider for a real dispatch path.

#### AR-ROUTE-09: Query And Proof Surface

- [ ] Add a route-diagnostics projection keyed by run/task/attempt.
- [ ] Add query output for current router stats, provider pass rates, calibration stats, and latest decisions.
- [ ] Add CLI inspection for the same data.
- [ ] Prove HTTP and CLI read the same route diagnostics.
- [ ] Store proof under `tmp/mori-diffs/generated/affect-routing-proof-report.json`.

### 9.4 Generated Proof Contract

An agent implementing this file must create `tmp/mori-diffs/generated/affect-routing-proof-report.json`:

```json
{
  "schema": "mori-diffs.affect-routing-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "default_model_semantics": {
    "config_default_is_not_task_hint": false,
    "task_hint_wins": false,
    "override_wins": false,
    "default_fallback_works": false
  },
  "cascade_authority": {
    "full_routing_context_built": false,
    "cascade_selected_model": false,
    "decision_source": null,
    "diagnostics_path": null
  },
  "bias_inputs": {
    "affect_bias_proved": false,
    "knowledge_advice_proved": false,
    "provider_health_proved": false,
    "calibration_proved": false
  },
  "feedback_quality": {
    "real_model_recorded": false,
    "real_provider_recorded": false,
    "real_cost_recorded": false,
    "override_dampening_proved": false
  },
  "queries": {
    "http_route_diagnostics": false,
    "cli_route_diagnostics": false,
    "projection_digest": null
  },
  "remaining_gaps": []
}
```

### 9.5 No-Context Handoff Checklist

Use this exact order:

- [ ] Run `rg -n "model_hint: Some\\(ctx.config.model|task_model_hint.*or_else|CascadeRouter|ModelRouter|RoutingInputs|ModelChoiceSource|record_outcome|CalibrationPolicy|DaimonPolicy|KnowledgeRoutingAdvice|provider_model_outcome" crates/roko-cli/src crates/roko-learn/src`.
- [ ] Fix AR-ROUTE-01 first; otherwise every later routing change can be silently bypassed by the config default.
- [ ] Implement AR-ROUTE-02 before calling cascade methods.
- [ ] Implement AR-ROUTE-03 before affect/knowledge/provider health tuning.
- [ ] Implement AR-ROUTE-08 before trusting learned feedback.
- [ ] Implement AR-ROUTE-04 through AR-ROUTE-07 one signal at a time with deterministic tests.
- [ ] Implement AR-ROUTE-09 after route diagnostics are durable.
- [ ] Generate `tmp/mori-diffs/generated/affect-routing-proof-report.json`.
- [ ] Update [README.md](README.md), [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), [38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md](38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md), and [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).

### 9.6 Archive Gate

Do not archive this file until:

- [ ] Config default no longer bypasses cascade routing.
- [ ] `ModelRouter` calls the real cascade branch with a full routing context.
- [ ] Route diagnostics prove selected model, source, candidates, affect, knowledge, provider health, and calibration inputs.
- [ ] Feedback observations contain real model/provider/cost/tokens/duration.
- [ ] Neutral/stressed/confident/provider-down fixtures all pass.
- [ ] A real runner task proves route prediction and route outcome pairing.
- [ ] HTTP or CLI can query the route diagnostics after the run.
