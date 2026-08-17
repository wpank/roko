//! STATUS: WIRED -- runner-v2 uses [`EfeRouter`] as a cognitive tier signal.
//!
//! Active inference helpers for tier routing.
//!
//! This module keeps the math small but concrete: a belief state over a
//! factorized latent space and an expected-free-energy style tier selector.
//! It is sufficient for routing support and for future integration into the
//! cascade router without introducing a new planning framework.

use std::collections::HashMap;

use roko_core::agent::{ModelTier, TaskRequirements};
use serde::{Deserialize, Serialize};

const STATE_COUNT: usize = 90;
const SKILL_LEVELS: usize = 3;
const CONFIDENCE_LEVELS: usize = 10;

/// Belief distribution over the 90 latent routing states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefState {
    /// Flattened 3 x 3 x 10 probability table.
    pub probabilities: Vec<f64>,
    /// Number of updates incorporated into this belief state.
    pub updates: u64,
}

impl BeliefState {
    /// Uniform prior over all latent states.
    #[must_use]
    pub fn uniform() -> Self {
        Self {
            probabilities: vec![1.0 / STATE_COUNT as f64; STATE_COUNT],
            updates: 0,
        }
    }

    /// Renormalize the distribution after a Bayesian update.
    pub fn normalize(&mut self) {
        let total: f64 = self.probabilities.iter().sum();
        if total <= 0.0 || !total.is_finite() {
            self.probabilities.fill(1.0 / STATE_COUNT as f64);
            return;
        }

        for probability in &mut self.probabilities {
            *probability = (*probability / total).clamp(0.0, 1.0);
        }
    }

    /// Update the belief state after observing an outcome.
    pub fn observe(
        &mut self,
        requirements: &TaskRequirements,
        selected_tier: ModelTier,
        success: bool,
        cost_usd: f64,
        latency_ms: f64,
    ) {
        let task_difficulty = task_difficulty(requirements);
        for idx in 0..self.probabilities.len() {
            let (difficulty, skill, confidence) = decode_state(idx);
            let success_likelihood = success_likelihood(
                selected_tier,
                difficulty,
                skill,
                confidence,
                task_difficulty,
            );
            let cost_penalty = cost_penalty(selected_tier, cost_usd);
            let latency_penalty = latency_penalty(selected_tier, latency_ms);
            let likelihood = if success {
                success_likelihood
            } else {
                1.0 - success_likelihood
            } * (1.0 - cost_penalty)
                * (1.0 - latency_penalty);
            self.probabilities[idx] *= likelihood.clamp(0.01, 1.0);
        }
        self.updates += 1;
        self.normalize();
    }
}

/// Component values contributing to one tier's expected free energy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EfeScore {
    /// Expected information gain from resolving current uncertainty.
    pub epistemic_value: f64,
    /// Belief-weighted probability of completing the task.
    pub pragmatic_value: f64,
    /// Estimated inference cost in USD.
    pub cost: f64,
    /// Regime-dependent penalty for using a more expensive tier.
    pub regime_penalty: f64,
    /// `-epistemic - pragmatic + cost + regime_penalty`.
    pub total: f64,
}

/// Invalid EFE router configuration.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EfeRouterError {
    /// Every model tier requires an explicit price.
    #[error("missing price for model tier {0:?}")]
    MissingTier(ModelTier),
    /// Prices must be finite and non-negative.
    #[error("invalid price for model tier {tier:?}: {price}")]
    InvalidPrice {
        /// Tier whose price was invalid.
        tier: ModelTier,
        /// Rejected price per million tokens.
        price: f64,
    },
}

/// Expected-free-energy router over the three model capability tiers.
#[derive(Debug, Clone, PartialEq)]
pub struct EfeRouter {
    belief: BeliefState,
    cost_table: HashMap<ModelTier, f64>,
}

impl Default for EfeRouter {
    fn default() -> Self {
        let cost_table = HashMap::from([
            (ModelTier::Fast, 0.25),
            (ModelTier::Standard, 3.0),
            (ModelTier::Premium, 15.0),
        ]);
        Self {
            belief: BeliefState::uniform(),
            cost_table,
        }
    }
}

impl EfeRouter {
    /// Construct a router from a belief state and per-million-token prices.
    pub fn new(
        belief: BeliefState,
        cost_table: HashMap<ModelTier, f64>,
    ) -> Result<Self, EfeRouterError> {
        for tier in model_tiers() {
            let Some(price) = cost_table.get(&tier).copied() else {
                return Err(EfeRouterError::MissingTier(tier));
            };
            if !price.is_finite() || price < 0.0 {
                return Err(EfeRouterError::InvalidPrice { tier, price });
            }
        }
        Ok(Self { belief, cost_table })
    }

    /// Borrow the belief state used for pragmatic and epistemic scoring.
    #[must_use]
    pub const fn belief(&self) -> &BeliefState {
        &self.belief
    }

    /// Mutably borrow the belief state so callers can incorporate outcomes.
    pub const fn belief_mut(&mut self) -> &mut BeliefState {
        &mut self.belief
    }

    /// Compute the expected-free-energy components for one tier.
    #[must_use]
    pub fn score(
        &self,
        tier: ModelTier,
        surprise_rate: f32,
        regime: u8,
        task_difficulty: f64,
    ) -> EfeScore {
        let surprise_rate = finite_unit_f32(surprise_rate);
        let task_difficulty = finite_unit(task_difficulty);
        let epistemic_value = epistemic_value(&self.belief, tier, surprise_rate);
        let pragmatic_value = pragmatic_value(&self.belief, tier, task_difficulty);
        let cost = estimated_cost(
            tier,
            task_difficulty,
            self.cost_table.get(&tier).copied().unwrap_or(f64::INFINITY),
        );
        let regime_penalty = regime_penalty(tier, regime);
        EfeScore {
            epistemic_value,
            pragmatic_value,
            cost,
            regime_penalty,
            total: -epistemic_value - pragmatic_value + cost + regime_penalty,
        }
    }

    /// Select the tier with the lowest expected free energy.
    #[must_use]
    pub fn route(&self, surprise_rate: f32, regime: u8, task_difficulty: f64) -> ModelTier {
        model_tiers()
            .into_iter()
            .min_by(|left, right| {
                self.score(*left, surprise_rate, regime, task_difficulty)
                    .total
                    .total_cmp(
                        &self
                            .score(*right, surprise_rate, regime, task_difficulty)
                            .total,
                    )
            })
            .unwrap_or(ModelTier::Fast)
    }
}

const fn model_tiers() -> [ModelTier; 3] {
    [ModelTier::Fast, ModelTier::Standard, ModelTier::Premium]
}

fn epistemic_value(belief: &BeliefState, tier: ModelTier, surprise_rate: f64) -> f64 {
    let entropy = belief_entropy(belief);
    let tier_information_gain = match tier {
        ModelTier::Fast => 0.05,
        ModelTier::Standard => 0.45,
        ModelTier::Premium => 1.0,
        _ => 0.45,
    };
    surprise_rate * entropy * tier_information_gain
}

fn pragmatic_value(belief: &BeliefState, tier: ModelTier, task_difficulty: f64) -> f64 {
    let task_difficulty = (task_difficulty * 2.0).round() as usize;
    belief
        .probabilities
        .iter()
        .copied()
        .enumerate()
        .map(|(index, probability)| {
            let (difficulty, skill, confidence) = decode_state(index);
            probability * success_likelihood(tier, difficulty, skill, confidence, task_difficulty)
        })
        .sum::<f64>()
        .clamp(0.0, 1.0)
}

fn belief_entropy(belief: &BeliefState) -> f64 {
    let entropy = belief
        .probabilities
        .iter()
        .copied()
        .filter(|probability| probability.is_finite() && *probability > 0.0)
        .map(|probability| -probability * probability.ln())
        .sum::<f64>();
    (entropy / (STATE_COUNT as f64).ln()).clamp(0.0, 1.0)
}

fn estimated_cost(tier: ModelTier, task_difficulty: f64, price_per_million: f64) -> f64 {
    let base_tokens = match tier {
        ModelTier::Fast => 256.0,
        ModelTier::Standard => 1_024.0,
        ModelTier::Premium => 4_096.0,
        _ => 1_024.0,
    };
    let token_estimate = base_tokens * (1.0 + task_difficulty * 2.0);
    token_estimate * price_per_million / 1_000_000.0
}

fn regime_penalty(tier: ModelTier, regime: u8) -> f64 {
    let base = match regime {
        0 | 1 => 0.0,
        2 => 0.1,
        _ => 0.5,
    };
    let tier_weight = match tier {
        ModelTier::Fast => 0.0,
        ModelTier::Standard => 0.5,
        ModelTier::Premium => 1.0,
        _ => 0.5,
    };
    base * tier_weight
}

fn finite_unit(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_unit_f32(value: f32) -> f64 {
    if value.is_finite() {
        f64::from(value.clamp(0.0, 1.0))
    } else {
        0.0
    }
}

/// Select the model tier that minimizes expected free energy.
#[must_use]
pub fn select_tier(belief: &BeliefState, requirements: &TaskRequirements) -> ModelTier {
    let task_difficulty = task_difficulty(requirements);
    if task_difficulty >= 2 {
        return ModelTier::Premium;
    }
    let tiers = [ModelTier::Fast, ModelTier::Standard, ModelTier::Premium];
    tiers
        .into_iter()
        .min_by(|left, right| {
            expected_free_energy(belief, *left, task_difficulty)
                .partial_cmp(&expected_free_energy(belief, *right, task_difficulty))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(ModelTier::Standard)
}

fn expected_free_energy(belief: &BeliefState, tier: ModelTier, task_difficulty: usize) -> f64 {
    let mut risk = 0.0;
    let mut ambiguity = 0.0;
    let mut evidence = 0.0;

    for (idx, probability) in belief.probabilities.iter().copied().enumerate() {
        let (difficulty, skill, confidence) = decode_state(idx);
        let success = success_likelihood(tier, difficulty, skill, confidence, task_difficulty);
        risk += probability * (1.0 - success);
        ambiguity += probability * (1.0 - confidence as f64 / (CONFIDENCE_LEVELS as f64 - 1.0));
        evidence += probability * (tier_cost(tier) + tier_latency(tier));
    }

    (risk + 0.20 * ambiguity + 0.10 * evidence).clamp(0.0, 1.5)
}

fn task_difficulty(requirements: &TaskRequirements) -> usize {
    let mut score = 0usize;
    if requirements.needs_web_search {
        score += 1;
    }
    if requirements.needs_code_execution {
        score += 1;
    }
    if requirements.needs_thinking {
        score += 1;
    }
    if requirements.needs_vision {
        score += 1;
    }
    if requirements.needs_structured_output {
        score += 1;
    }
    if requirements.min_context_window >= 120_000 {
        score += 1;
    }

    match score {
        0..=1 => 0,
        2 => 1,
        _ => 2,
    }
}

fn decode_state(index: usize) -> (usize, usize, usize) {
    let difficulty = index / (SKILL_LEVELS * CONFIDENCE_LEVELS);
    let skill = (index / CONFIDENCE_LEVELS) % SKILL_LEVELS;
    let confidence = index % CONFIDENCE_LEVELS;
    (difficulty, skill, confidence)
}

fn success_likelihood(
    tier: ModelTier,
    difficulty: usize,
    skill: usize,
    confidence: usize,
    task_difficulty: usize,
) -> f64 {
    let tier_strength = match tier {
        ModelTier::Fast => 0,
        ModelTier::Standard => 1,
        ModelTier::Premium => 2,
        _ => 1,
    } as isize;
    let skill_strength = skill as isize;
    let difficulty_gap = (task_difficulty as isize - tier_strength).abs() as f64;
    let latent_gap = (difficulty as isize - skill_strength).abs() as f64;
    let confidence_boost = confidence as f64 / (CONFIDENCE_LEVELS as f64 - 1.0);

    (0.8 - difficulty_gap * 0.18 - latent_gap * 0.12 + confidence_boost * 0.2).clamp(0.05, 0.95)
}

fn tier_cost(tier: ModelTier) -> f64 {
    match tier {
        ModelTier::Fast => 0.08,
        ModelTier::Standard => 0.18,
        ModelTier::Premium => 0.28,
        _ => 0.18,
    }
}

fn tier_latency(tier: ModelTier) -> f64 {
    match tier {
        ModelTier::Fast => 0.10,
        ModelTier::Standard => 0.18,
        ModelTier::Premium => 0.26,
        _ => 0.18,
    }
}

fn cost_penalty(tier: ModelTier, cost_usd: f64) -> f64 {
    let budget = match tier {
        ModelTier::Fast => 0.35,
        ModelTier::Standard => 0.65,
        ModelTier::Premium => 1.0,
        _ => 0.65,
    };
    (cost_usd / budget).clamp(0.0, 1.0) * 0.05
}

fn latency_penalty(tier: ModelTier, latency_ms: f64) -> f64 {
    let budget = match tier {
        ModelTier::Fast => 20_000.0,
        ModelTier::Standard => 60_000.0,
        ModelTier::Premium => 120_000.0,
        _ => 60_000.0,
    };
    (latency_ms / budget).clamp(0.0, 1.0) * 0.05
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirements(
        needs_code_execution: bool,
        needs_thinking: bool,
        min_context_window: u64,
    ) -> TaskRequirements {
        TaskRequirements {
            needs_web_search: false,
            needs_code_execution,
            needs_thinking,
            needs_vision: false,
            needs_structured_output: false,
            min_context_window,
            max_cost_output_per_m: None,
            max_latency_ms: None,
        }
    }

    #[test]
    fn easy_requirements_choose_fast_tier() {
        let belief = BeliefState::uniform();
        let tier = select_tier(&belief, &requirements(false, false, 8_000));
        assert_eq!(tier, ModelTier::Fast);
    }

    #[test]
    fn harder_requirements_choose_premium_tier() {
        let belief = BeliefState::uniform();
        let tier = select_tier(&belief, &requirements(true, true, 160_000));
        assert_eq!(tier, ModelTier::Premium);
    }

    #[test]
    fn observation_updates_beliefs() {
        let mut belief = BeliefState::uniform();
        let req = requirements(true, true, 160_000);
        let before = belief.probabilities.clone();
        belief.observe(&req, ModelTier::Premium, true, 0.4, 8_000.0);

        assert_eq!(belief.updates, 1);
        assert_ne!(belief.probabilities, before);
        assert!((belief.probabilities.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn efe_router_chooses_fast_for_low_surprise_routine_work() {
        let router = EfeRouter::default();
        assert_eq!(router.route(0.02, 0, 0.0), ModelTier::Fast);
    }

    #[test]
    fn efe_router_chooses_premium_for_high_uncertainty_hard_work() {
        let router = EfeRouter::default();
        assert_eq!(router.route(1.0, 1, 1.0), ModelTier::Premium);
        let premium = router.score(ModelTier::Premium, 1.0, 1, 1.0);
        let fast = router.score(ModelTier::Fast, 1.0, 1, 1.0);
        assert!(premium.epistemic_value > fast.epistemic_value);
        assert!(premium.pragmatic_value > fast.pragmatic_value);
        assert!(premium.total < fast.total);
    }

    #[test]
    fn efe_router_regime_penalty_discourages_expensive_tiers_in_crisis() {
        let router = EfeRouter::default();
        let calm = router.score(ModelTier::Premium, 0.4, 0, 1.0);
        let crisis = router.score(ModelTier::Premium, 0.4, 3, 1.0);
        assert_eq!(calm.regime_penalty, 0.0);
        assert_eq!(crisis.regime_penalty, 0.5);
        assert!(crisis.total > calm.total);
        assert_ne!(router.route(0.4, 3, 1.0), ModelTier::Premium);
    }

    #[test]
    fn efe_router_validates_prices_and_sanitizes_observations() {
        let missing = HashMap::from([(ModelTier::Fast, 0.25), (ModelTier::Standard, 3.0)]);
        assert_eq!(
            EfeRouter::new(BeliefState::uniform(), missing),
            Err(EfeRouterError::MissingTier(ModelTier::Premium))
        );

        let router = EfeRouter::default();
        assert_eq!(router.route(f32::NAN, u8::MAX, f64::NAN), ModelTier::Fast);
    }
}
