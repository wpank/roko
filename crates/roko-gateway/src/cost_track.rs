//! Actual-versus-naive inference cost accounting.

use std::collections::HashMap;
use std::sync::RwLock;

use roko_learn::cost_table::{CostTable, ModelPricing};
use serde::{Deserialize, Serialize};

use crate::TokenUsage;

const PER_MILLION: f64 = 1_000_000.0;
const BATCH_DISCOUNT: f64 = 0.5;

/// Itemized cost components in USD.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Non-cached input.
    pub fresh_input: f64,
    /// Provider cache reads.
    pub cached_input: f64,
    /// Provider cache creation.
    pub cache_write: f64,
    /// Non-reasoning generated output.
    pub regular_output: f64,
    /// Provider-reported reasoning tokens.
    pub reasoning: f64,
    /// Provider-reported extended-thinking tokens.
    pub thinking: f64,
}

impl CostBreakdown {
    fn total(self) -> f64 {
        self.fresh_input
            + self.cached_input
            + self.cache_write
            + self.regular_output
            + self.reasoning
            + self.thinking
    }
}

/// Actual and counterfactual cost for one request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CostResult {
    /// Provider-aware cost after caching and optional batch discount.
    pub actual_cost: f64,
    /// Cost if all input/output used regular rates.
    pub naive_cost: f64,
    /// Difference between naive and actual cost.
    pub savings: f64,
    /// Pre-discount itemization.
    pub breakdown: CostBreakdown,
}

/// Fully attributed cost record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostRecord {
    /// Calling agent.
    pub agent_id: String,
    /// Calling session.
    pub session_id: String,
    /// Served model.
    pub model: String,
    /// Cost result.
    pub cost: CostResult,
}

/// Aggregate attributed totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CostAggregate {
    /// Completed calls.
    pub requests: u64,
    /// Actual cost.
    pub actual_cost: f64,
    /// Naive cost.
    pub naive_cost: f64,
    /// Mechanical savings.
    pub savings: f64,
}

/// Pricing-backed cost calculator and attribution index.
pub struct CostTracker {
    cost_table: CostTable,
    by_agent: RwLock<HashMap<String, CostAggregate>>,
    by_session: RwLock<HashMap<String, CostAggregate>>,
    by_model: RwLock<HashMap<String, CostAggregate>>,
}

impl CostTracker {
    /// Construct from the canonical learning cost table.
    #[must_use]
    pub fn new(cost_table: CostTable) -> Self {
        Self {
            cost_table,
            by_agent: RwLock::new(HashMap::new()),
            by_session: RwLock::new(HashMap::new()),
            by_model: RwLock::new(HashMap::new()),
        }
    }

    /// Compute one request's cost using canonical model pricing.
    #[must_use]
    pub fn compute_cost(&self, usage: &TokenUsage, model: &str, is_batch: bool) -> CostResult {
        let fallback;
        let pricing = if let Some(pricing) = self.cost_table.lookup(model) {
            pricing
        } else {
            fallback = sonnet_fallback();
            &fallback
        };
        let fresh_tokens = usage
            .input_tokens
            .saturating_sub(usage.cache_read_input_tokens);
        let regular_output_tokens = usage.output_tokens.saturating_sub(usage.reasoning_tokens);
        let breakdown = CostBreakdown {
            fresh_input: fresh_tokens as f64 * pricing.input_per_m / PER_MILLION,
            cached_input: usage.cache_read_input_tokens as f64 * pricing.cache_read_per_m
                / PER_MILLION,
            cache_write: usage.cache_creation_input_tokens as f64 * pricing.input_per_m * 1.25
                / PER_MILLION,
            regular_output: regular_output_tokens as f64 * pricing.output_per_m / PER_MILLION,
            // The canonical ModelPricing has no separate reasoning rate today;
            // provider reasoning and thinking therefore use the output rate.
            reasoning: usage.reasoning_tokens as f64 * pricing.output_per_m / PER_MILLION,
            thinking: usage.thinking_tokens as f64 * pricing.output_per_m / PER_MILLION,
        };
        let undiscounted = breakdown.total();
        let actual_cost = if is_batch {
            undiscounted * BATCH_DISCOUNT
        } else {
            undiscounted
        };
        let naive_cost = usage.input_tokens as f64 * pricing.input_per_m / PER_MILLION
            + usage.output_tokens as f64 * pricing.output_per_m / PER_MILLION;
        CostResult {
            actual_cost,
            naive_cost,
            savings: naive_cost - actual_cost,
            breakdown,
        }
    }

    /// Compute and record a fully attributed request.
    pub fn record(
        &self,
        usage: &TokenUsage,
        model: &str,
        is_batch: bool,
        agent_id: &str,
        session_id: &str,
    ) -> CostRecord {
        let cost = self.compute_cost(usage, model, is_batch);
        update(&self.by_agent, agent_id, cost);
        update(&self.by_session, session_id, cost);
        update(&self.by_model, model, cost);
        CostRecord {
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
            model: model.to_string(),
            cost,
        }
    }

    /// Per-agent aggregate.
    #[must_use]
    pub fn agent_total(&self, agent_id: &str) -> CostAggregate {
        get(&self.by_agent, agent_id)
    }

    /// Per-session aggregate.
    #[must_use]
    pub fn session_total(&self, session_id: &str) -> CostAggregate {
        get(&self.by_session, session_id)
    }

    /// Per-model aggregate.
    #[must_use]
    pub fn model_total(&self, model: &str) -> CostAggregate {
        get(&self.by_model, model)
    }
}

fn update(store: &RwLock<HashMap<String, CostAggregate>>, key: &str, cost: CostResult) {
    if let Ok(mut store) = store.write() {
        let aggregate = store.entry(key.to_string()).or_default();
        aggregate.requests = aggregate.requests.saturating_add(1);
        aggregate.actual_cost += cost.actual_cost;
        aggregate.naive_cost += cost.naive_cost;
        aggregate.savings += cost.savings;
    }
}

fn get(store: &RwLock<HashMap<String, CostAggregate>>, key: &str) -> CostAggregate {
    store
        .read()
        .ok()
        .and_then(|store| store.get(key).copied())
        .unwrap_or_default()
}

fn sonnet_fallback() -> ModelPricing {
    ModelPricing {
        input_per_m: 3.0,
        output_per_m: 15.0,
        cache_read_per_m: 0.30,
        cache_write_per_m: 3.75,
        tokenizer_ratio: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn tracker() -> CostTracker {
        CostTracker::new(CostTable {
            models: HashMap::from([(
                "model".into(),
                ModelPricing {
                    input_per_m: 2.0,
                    output_per_m: 10.0,
                    cache_read_per_m: 0.2,
                    cache_write_per_m: 2.5,
                    tokenizer_ratio: 1.0,
                },
            )]),
        })
    }

    #[test]
    fn cost_track_formula_and_naive_savings_match_contract() {
        let usage = TokenUsage {
            input_tokens: 1_000,
            output_tokens: 200,
            cache_read_input_tokens: 400,
            cache_creation_input_tokens: 100,
            thinking_tokens: 20,
            reasoning_tokens: 50,
        };
        let result = tracker().compute_cost(&usage, "model", false);
        let expected = (600.0 * 2.0
            + 400.0 * 0.2
            + 100.0 * 2.0 * 1.25
            + 150.0 * 10.0
            + 50.0 * 10.0
            + 20.0 * 10.0)
            / 1_000_000.0;
        assert!((result.actual_cost - expected).abs() < 1e-12);
        assert!((result.naive_cost - 0.004).abs() < 1e-12);
        assert!((result.savings - (result.naive_cost - result.actual_cost)).abs() < 1e-12);
    }

    #[test]
    fn cost_track_batch_discount_and_attribution() {
        let tracker = tracker();
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 100,
            ..TokenUsage::default()
        };
        let regular = tracker.compute_cost(&usage, "model", false);
        let batch = tracker.record(&usage, "model", true, "agent", "session");
        assert!((batch.cost.actual_cost - regular.actual_cost * 0.5).abs() < 1e-12);
        assert_eq!(batch.agent_id, "agent");
        assert_eq!(batch.session_id, "session");
        assert_eq!(tracker.agent_total("agent").requests, 1);
        assert_eq!(tracker.session_total("session").requests, 1);
    }

    #[test]
    fn cost_track_unknown_model_uses_sonnet_fallback() {
        let tracker = CostTracker::new(CostTable {
            models: HashMap::new(),
        });
        let result = tracker.compute_cost(
            &TokenUsage {
                input_tokens: 1_000_000,
                ..TokenUsage::default()
            },
            "unknown",
            false,
        );
        assert!((result.actual_cost - 3.0).abs() < 1e-12);
    }
}
