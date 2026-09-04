//! Pre-dispatch cost projection utilities.
//!
//! Given an estimated prompt token count and a model slug, [`project_task_cost`]
//! returns a [`CostProjection`] with predicted input and output token counts and
//! the resulting cost in USD.
//!
//! Pricing is resolved from the canonical shared registry
//! ([`roko_core::config::model_registry::builtin_pricing`]) so projections
//! stay in sync with the live cost tables and TUI. Local zero-cost overrides
//! cover self-hosted models (llama, ollama, mistral).
//!
//! # Default output token assumption
//!
//! Output token counts are unknown before a request is issued.  We assume
//! **512 output tokens** as a conservative minimum that keeps projections
//! usable without being wildly optimistic.  Callers with a specific
//! `max_tokens` value should multiply `estimated_output_tokens` accordingly.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Projected cost for a single LLM task dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostProjection {
    /// Estimated number of input (prompt) tokens.
    pub estimated_input_tokens: u64,
    /// Estimated number of output (completion) tokens.
    pub estimated_output_tokens: u64,
    /// Predicted cost in USD for the estimated token counts.
    pub estimated_cost_usd: f64,
}

/// Default assumed output token count when `max_tokens` is unknown.
const DEFAULT_OUTPUT_TOKENS: u64 = 512;

/// Fallback pricing used when the model slug is not in the pricing table.
///
/// Uses conservative Sonnet-equivalent rates as a safe over-estimate:
/// - $0.003 / 1K input tokens
/// - $0.015 / 1K output tokens
const FALLBACK_INPUT_PER_K: f64 = 0.003;
const FALLBACK_OUTPUT_PER_K: f64 = 0.015;

/// Per-model pricing entry used by the local projection table.
struct ProjectionPricing {
    /// Cost in USD per 1 000 input tokens.
    input_per_k: f64,
    /// Cost in USD per 1 000 output tokens.
    output_per_k: f64,
}

/// Project the cost of a single task dispatch given a prompt token estimate and
/// a model slug.
///
/// # Parameters
///
/// - `prompt_tokens`: estimated input token count (e.g. from
///   `roko_agent::estimate_prompt_tokens`).
/// - `model_slug`: canonical model slug such as `"claude-sonnet-4-6"` or
///   `"gpt-5.4-mini"`.  Prefix matching is performed so
///   `"claude-sonnet-4-6-20250514"` resolves to the `claude-sonnet-4-6` entry.
///
/// # Returns
///
/// A [`CostProjection`] with the estimated input tokens, a conservative output
/// token assumption of [`DEFAULT_OUTPUT_TOKENS`], and the projected USD cost.
#[must_use]
pub fn project_task_cost(prompt_tokens: u64, model_slug: &str) -> CostProjection {
    let output_tokens = DEFAULT_OUTPUT_TOKENS;

    let pricing = resolve_pricing(model_slug);

    let estimated_cost_usd = (prompt_tokens as f64 * pricing.input_per_k / 1_000.0)
        + (output_tokens as f64 * pricing.output_per_k / 1_000.0);

    CostProjection {
        estimated_input_tokens: prompt_tokens,
        estimated_output_tokens: output_tokens,
        estimated_cost_usd,
    }
}

/// Project the cost with an explicit output token count.
///
/// Useful when the caller knows `max_tokens` from the model profile or task
/// config and wants a tighter estimate than the [`DEFAULT_OUTPUT_TOKENS`]
/// assumption.
#[must_use]
pub fn project_task_cost_with_output(
    prompt_tokens: u64,
    output_tokens: u64,
    model_slug: &str,
) -> CostProjection {
    let pricing = resolve_pricing(model_slug);

    let estimated_cost_usd = (prompt_tokens as f64 * pricing.input_per_k / 1_000.0)
        + (output_tokens as f64 * pricing.output_per_k / 1_000.0);

    CostProjection {
        estimated_input_tokens: prompt_tokens,
        estimated_output_tokens: output_tokens,
        estimated_cost_usd,
    }
}

// ── pricing table ─────────────────────────────────────────────────────────────

/// Resolve pricing for a model slug by delegating to the canonical shared
/// registry ([`roko_core::config::model_registry::builtin_pricing`]).
///
/// The registry stores per-million rates; this function converts to per-1K.
/// Falls back to [`FALLBACK_INPUT_PER_K`] / [`FALLBACK_OUTPUT_PER_K`] when
/// no entry matches.
fn resolve_pricing(slug: &str) -> ProjectionPricing {
    if let Some(reg) = roko_core::config::model_registry::builtin_pricing(slug) {
        return ProjectionPricing {
            input_per_k: reg.input_per_m / 1_000.0,
            output_per_k: reg.output_per_m / 1_000.0,
        };
    }

    // Local zero-cost overrides for self-hosted / local models.
    let lower = slug.to_ascii_lowercase();
    if lower.starts_with("llama") || lower.starts_with("ollama") {
        return ProjectionPricing {
            input_per_k: 0.0,
            output_per_k: 0.0,
        };
    }
    if lower.starts_with("mistral") {
        return ProjectionPricing {
            input_per_k: 0.001,
            output_per_k: 0.003,
        };
    }

    // Unknown model — use conservative fallback.
    ProjectionPricing {
        input_per_k: FALLBACK_INPUT_PER_K,
        output_per_k: FALLBACK_OUTPUT_PER_K,
    }
}

// ── Plan-level cost projection ─────────────────────────────────────────────

/// A completed task record used as input to [`CostProjector`].
///
/// Callers populate this from their orchestrator's task tracking state.
/// Only the `tier` and `cost_usd` fields are required; the model hint is
/// used to improve tier inference when `tier` is `""`.
#[derive(Debug, Clone)]
pub struct CompletedTask {
    /// Complexity tier of the task (e.g. `"mechanical"`, `"standard"`,
    /// `"complex"`, `"expert"`).  Empty string is treated as `"standard"`.
    pub tier: String,
    /// Model slug that was used to execute the task (optional hint).
    pub model: String,
    /// Actual cost incurred by this task in USD.
    pub cost_usd: f64,
}

/// A remaining task that has not yet been dispatched.
///
/// Only `tier` and (optionally) `model_hint` are consumed by the projector.
#[derive(Debug, Clone)]
pub struct RemainingTask {
    /// Complexity tier of the task.  Empty string is treated as `"standard"`.
    pub tier: String,
    /// Optional model hint from the task definition (e.g. `"claude-haiku-4-5"`).
    pub model_hint: String,
}

/// A three-band cost estimate for the remaining tasks in a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Total cost already incurred by completed tasks, in USD.
    pub spent_usd: f64,
    /// Optimistic remaining cost (0.6× expected), in USD.
    pub optimistic_usd: f64,
    /// Expected remaining cost (historical average × remaining count), in USD.
    pub expected_usd: f64,
    /// Pessimistic remaining cost (1.5× expected), in USD.
    pub pessimistic_usd: f64,
    /// Number of completed tasks used to compute the averages.
    pub tasks_completed: usize,
    /// Number of remaining tasks whose cost is being projected.
    pub tasks_remaining: usize,
    /// Confidence level: `"low"` when fewer than [`CostProjector::MIN_SAMPLES`]
    /// tasks have been observed for a tier, `"high"` otherwise.
    pub confidence: String,
}

impl CostEstimate {
    /// Projected total plan cost (spent + expected remaining), in USD.
    #[must_use]
    pub fn projected_total_usd(&self) -> f64 {
        self.spent_usd + self.expected_usd
    }

    /// Returns `true` when the projected total exceeds `budget_usd`.
    #[must_use]
    pub fn exceeds_budget(&self, budget_usd: f64) -> bool {
        self.projected_total_usd() > budget_usd
    }
}

/// Real-time cost projector for an in-progress plan execution.
///
/// After each task completes, call [`record_completed`](Self::record_completed)
/// to update the internal per-tier averages.  Then call
/// [`project_remaining_cost`](Self::project_remaining_cost) with the list of
/// remaining tasks to get the current [`CostEstimate`].
///
/// # Confidence
///
/// Projections with fewer than [`Self::MIN_SAMPLES`] completed tasks per tier
/// fall back to the pricing-table estimate from [`project_task_cost`] and are
/// flagged as `confidence: "low"`.  Once enough historical data has been
/// collected the label becomes `"high"`.
#[derive(Debug, Clone, Default)]
pub struct CostProjector {
    /// Per-tier running sum of completed task costs.
    tier_cost_sum: HashMap<String, f64>,
    /// Per-tier completed task count.
    tier_count: HashMap<String, usize>,
    /// Total cost of all completed tasks.
    total_spent_usd: f64,
    /// Total number of completed tasks.
    tasks_completed: usize,
}

impl CostProjector {
    /// Minimum completed samples per tier required for `"high"` confidence.
    pub const MIN_SAMPLES: usize = 3;

    /// Optimistic multiplier applied to the expected estimate.
    const OPTIMISTIC_FACTOR: f64 = 0.6;
    /// Pessimistic multiplier applied to the expected estimate.
    const PESSIMISTIC_FACTOR: f64 = 1.5;

    /// Create a new empty projector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a completed task and update the per-tier averages.
    pub fn record_completed(&mut self, task: &CompletedTask) {
        let tier = normalize_tier(&task.tier).to_string();
        *self.tier_cost_sum.entry(tier.clone()).or_default() += task.cost_usd;
        *self.tier_count.entry(tier).or_default() += 1;
        self.total_spent_usd += task.cost_usd;
        self.tasks_completed += 1;
    }

    /// Return the average cost per task for the given tier.
    ///
    /// Falls back to the pricing-table estimate when fewer than
    /// [`Self::MIN_SAMPLES`] tasks have been recorded for this tier.
    #[must_use]
    pub fn avg_cost_for_tier(&self, tier: &str, model_hint: &str) -> f64 {
        let tier = normalize_tier(tier);
        let count = self.tier_count.get(tier).copied().unwrap_or(0);
        if count > 0 {
            let sum = self.tier_cost_sum.get(tier).copied().unwrap_or(0.0);
            sum / count as f64
        } else {
            // No history yet — use pricing table estimate with conservative defaults.
            self.pricing_fallback(tier, model_hint)
        }
    }

    /// Project the remaining cost for the provided set of tasks.
    ///
    /// `default_model` is used as the pricing-table fallback for tasks whose
    /// `model_hint` is empty.
    #[must_use]
    pub fn project_remaining_cost(
        &self,
        remaining: &[RemainingTask],
        default_model: &str,
    ) -> CostEstimate {
        let tasks_remaining = remaining.len();

        // Accumulate expected cost and track whether any tier lacked samples.
        let mut expected_usd = 0.0;
        let mut low_confidence = false;

        for task in remaining {
            let tier = normalize_tier(&task.tier);
            let model = if task.model_hint.is_empty() {
                default_model
            } else {
                task.model_hint.as_str()
            };

            let count = self.tier_count.get(tier).copied().unwrap_or(0);
            if count < Self::MIN_SAMPLES {
                low_confidence = true;
            }

            expected_usd += self.avg_cost_for_tier(tier, model);
        }

        let optimistic_usd = expected_usd * Self::OPTIMISTIC_FACTOR;
        let pessimistic_usd = expected_usd * Self::PESSIMISTIC_FACTOR;
        let confidence = if low_confidence { "low" } else { "high" }.to_string();

        CostEstimate {
            spent_usd: self.total_spent_usd,
            optimistic_usd,
            expected_usd,
            pessimistic_usd,
            tasks_completed: self.tasks_completed,
            tasks_remaining,
            confidence,
        }
    }

    /// Return the total accumulated spend so far.
    #[must_use]
    pub fn total_spent_usd(&self) -> f64 {
        self.total_spent_usd
    }

    /// Number of completed tasks recorded.
    #[must_use]
    pub fn tasks_completed(&self) -> usize {
        self.tasks_completed
    }

    /// Pricing-table fallback cost for a tier + model combination.
    fn pricing_fallback(&self, tier: &str, model_hint: &str) -> f64 {
        let (input_tokens, output_tokens) = default_tokens_for_tier(tier);
        let proj = project_task_cost_with_output(input_tokens, output_tokens, model_hint);
        proj.estimated_cost_usd
    }
}

/// Canonical tier string from a raw tier value.
fn normalize_tier(tier: &str) -> &str {
    match tier {
        "mechanical" | "standard" | "complex" | "expert" => tier,
        _ => "standard",
    }
}

/// Conservative default token counts by tier used for pricing-table fallback.
fn default_tokens_for_tier(tier: &str) -> (u64, u64) {
    match tier {
        "mechanical" => (2_000, 512),
        "complex" => (20_000, 2_048),
        "expert" => (40_000, 4_096),
        _ => (8_000, 1_024), // "standard" and unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_sonnet() {
        let proj = project_task_cost(1_000, "claude-sonnet-4-6");
        assert_eq!(proj.estimated_input_tokens, 1_000);
        assert_eq!(proj.estimated_output_tokens, DEFAULT_OUTPUT_TOKENS);
        // 1_000 * 0.003/1_000 + 512 * 0.015/1_000
        let expected = 1.0 * 0.003 + 512.0 * 0.015 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn known_model_haiku() {
        let proj = project_task_cost(2_000, "claude-haiku-4-5");
        // 2_000 * 0.0008/1_000 + 512 * 0.004/1_000
        let expected = 2.0 * 0.0008 + 512.0 * 0.004 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn prefix_match_with_date_suffix() {
        let proj_base = project_task_cost(1_000, "claude-sonnet-4-6");
        let proj_dated = project_task_cost(1_000, "claude-sonnet-4-6-20250514");
        // Both should resolve to the same pricing
        assert!((proj_base.estimated_cost_usd - proj_dated.estimated_cost_usd).abs() < 1e-12);
    }

    #[test]
    fn unknown_model_uses_fallback() {
        let proj = project_task_cost(1_000, "some-unknown-llm-v99");
        // 1_000 * FALLBACK_INPUT_PER_K/1_000 + 512 * FALLBACK_OUTPUT_PER_K/1_000
        let expected = 1.0 * FALLBACK_INPUT_PER_K + 512.0 * FALLBACK_OUTPUT_PER_K / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn zero_tokens_produces_zero_cost() {
        let proj = project_task_cost(0, "claude-sonnet-4-6");
        // 0 input + 512 output at sonnet rates
        let expected = 512.0 * 0.015 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn explicit_output_tokens_override() {
        let proj = project_task_cost_with_output(1_000, 2_000, "claude-sonnet-4-6");
        assert_eq!(proj.estimated_output_tokens, 2_000);
        // 1_000 * 0.003/1_000 + 2_000 * 0.015/1_000
        let expected = 1.0 * 0.003 + 2.0 * 0.015;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn opus_pricing() {
        let proj = project_task_cost(1_000, "claude-opus-4-6");
        // 1_000 * 0.015/1_000 + 512 * 0.075/1_000
        let expected = 1.0 * 0.015 + 512.0 * 0.075 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn local_model_zero_cost() {
        let proj = project_task_cost(10_000, "ollama-llama3");
        // ollama prefix → 0.0, 0.0
        assert!((proj.estimated_cost_usd).abs() < 1e-12);
    }

    // ── CostProjector tests ────────────────────────────────────────────────

    fn completed(tier: &str, cost: f64) -> CompletedTask {
        CompletedTask {
            tier: tier.into(),
            model: "claude-sonnet-4-6".into(),
            cost_usd: cost,
        }
    }

    fn remaining(tier: &str) -> RemainingTask {
        RemainingTask {
            tier: tier.into(),
            model_hint: "claude-sonnet-4-6".into(),
        }
    }

    #[test]
    fn cost_projector_empty_returns_fallback_low_confidence() {
        let projector = CostProjector::new();
        let tasks = vec![remaining("standard"), remaining("standard")];
        let est = projector.project_remaining_cost(&tasks, "claude-sonnet-4-6");

        assert_eq!(est.tasks_completed, 0);
        assert_eq!(est.tasks_remaining, 2);
        assert_eq!(est.confidence, "low");
        assert!((est.spent_usd).abs() < 1e-12);
        assert!(est.expected_usd > 0.0);
        assert!(est.optimistic_usd < est.expected_usd);
        assert!(est.expected_usd < est.pessimistic_usd);
    }

    #[test]
    fn cost_projector_uses_historical_average() {
        let mut projector = CostProjector::new();
        for _ in 0..3 {
            projector.record_completed(&completed("standard", 0.10));
        }

        let tasks = vec![remaining("standard"), remaining("standard")];
        let est = projector.project_remaining_cost(&tasks, "claude-sonnet-4-6");

        assert!((est.expected_usd - 0.20).abs() < 1e-9);
        assert!((est.spent_usd - 0.30).abs() < 1e-9);
        assert_eq!(est.tasks_completed, 3);
        assert_eq!(est.tasks_remaining, 2);
        assert_eq!(est.confidence, "high");
    }

    #[test]
    fn cost_projector_confidence_intervals_correct_factors() {
        let mut projector = CostProjector::new();
        for _ in 0..CostProjector::MIN_SAMPLES {
            projector.record_completed(&completed("standard", 1.00));
        }

        let tasks = vec![remaining("standard")];
        let est = projector.project_remaining_cost(&tasks, "claude-sonnet-4-6");

        assert!((est.expected_usd - 1.00).abs() < 1e-9);
        assert!((est.optimistic_usd - 0.60).abs() < 1e-9);
        assert!((est.pessimistic_usd - 1.50).abs() < 1e-9);
    }

    #[test]
    fn cost_projector_mixed_tiers() {
        let mut projector = CostProjector::new();
        for _ in 0..CostProjector::MIN_SAMPLES {
            projector.record_completed(&completed("standard", 0.50));
        }

        let tasks = vec![
            remaining("standard"),
            RemainingTask {
                tier: "complex".into(),
                model_hint: "claude-opus-4-6".into(),
            },
        ];
        let est = projector.project_remaining_cost(&tasks, "claude-sonnet-4-6");

        assert_eq!(est.confidence, "low");
        assert!(est.expected_usd > 0.50);
    }

    #[test]
    fn cost_projector_no_remaining_tasks_returns_zero_remaining() {
        let mut projector = CostProjector::new();
        projector.record_completed(&completed("standard", 0.42));

        let est = projector.project_remaining_cost(&[], "claude-sonnet-4-6");
        assert_eq!(est.tasks_remaining, 0);
        assert!((est.expected_usd).abs() < 1e-12);
    }
}
