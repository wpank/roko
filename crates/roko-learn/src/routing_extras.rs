//! Routing extras for lookahead support.
//!
//! These types capture the documented lookahead router scaffolding that sits
//! around the core cascade router. The calibration table that used to live
//! here had no caller and was deleted (backlog 6111): forecasts are scored by
//! [`crate::self_model::metrics`].

#![allow(dead_code)]

use crate::cascade_router::{CascadeModel, CascadeRouter};
use crate::model_router::RoutingContext;
use roko_core::agent::ModelSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Placeholder task dependency graph used by the lookahead router.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDag {}

impl TaskDag {
    /// Create an empty task DAG shell.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Whether the shell currently contains any task data.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        true
    }
}

/// Cache reuse statistics keyed by `(model, role)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheReuseModel {
    /// Per-(model, role) estimated cache hit rate when reusing the same model.
    cache_hit_rates: HashMap<(String, String), f64>,
    /// Average input tokens saved per cache hit.
    avg_tokens_saved_per_hit: u64,
    /// Cost per 1M tokens for cache reads vs fresh input.
    cache_read_discount: f64,
}

impl CacheReuseModel {
    /// Create a cache reuse model with conservative defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache_hit_rates: HashMap::new(),
            avg_tokens_saved_per_hit: 0,
            cache_read_discount: 1.0,
        }
    }

    /// Override the cache-read discount.
    #[must_use]
    pub fn with_cache_read_discount(mut self, discount: f64) -> Self {
        self.cache_read_discount = if discount.is_finite() && discount >= 0.0 {
            discount
        } else {
            1.0
        };
        self
    }

    /// Set the average input-token savings per cache hit.
    #[must_use]
    pub fn with_avg_tokens_saved_per_hit(mut self, tokens: u64) -> Self {
        self.avg_tokens_saved_per_hit = tokens;
        self
    }

    /// Record the estimated cache-hit rate for a `(model, role)` pair.
    pub fn record_hit_rate(
        &mut self,
        model: impl Into<String>,
        role: impl Into<String>,
        hit_rate: f64,
    ) {
        self.cache_hit_rates
            .insert((model.into(), role.into()), hit_rate.clamp(0.0, 1.0));
    }

    /// Read the estimated cache-hit rate for a `(model, role)` pair.
    #[must_use]
    pub fn cache_hit_rate(&self, model: &str, role: &str) -> f64 {
        self.cache_hit_rates
            .get(&(model.to_string(), role.to_string()))
            .copied()
            .unwrap_or(0.0)
    }

    /// Estimate tokens saved for a candidate model and role.
    #[must_use]
    pub fn estimated_tokens_saved(&self, model: &str, role: &str) -> f64 {
        self.cache_hit_rate(model, role) * self.avg_tokens_saved_per_hit as f64
    }

    /// Estimate cost saved for a candidate model and role.
    #[must_use]
    pub fn estimated_cost_saved(&self, model: &str, role: &str) -> f64 {
        self.estimated_tokens_saved(model, role) * self.cache_read_discount / 1_000_000.0
    }
}

/// Lookahead wrapper around the core cascade router.
pub struct LookaheadRouter {
    /// Base cascade router for individual decisions.
    inner: CascadeRouter,
    /// Task dependency graph for lookahead.
    task_graph: TaskDag,
    /// Lookahead horizon (default: 3 tasks ahead).
    pub horizon: usize,
    /// Discount factor for future savings (default: 0.9).
    pub gamma: f64,
    /// KV cache reuse probability model.
    cache_model: CacheReuseModel,
}

impl LookaheadRouter {
    /// Create a lookahead router wrapper.
    #[must_use]
    pub fn new(inner: CascadeRouter, task_graph: TaskDag) -> Self {
        Self {
            inner,
            task_graph,
            horizon: 3,
            gamma: 0.9,
            cache_model: CacheReuseModel::new(),
        }
    }

    /// Override the lookahead horizon.
    #[must_use]
    pub fn with_horizon(mut self, horizon: usize) -> Self {
        self.horizon = horizon.max(1);
        self
    }

    /// Override the future-savings discount factor.
    #[must_use]
    pub fn with_gamma(mut self, gamma: f64) -> Self {
        self.gamma = if gamma.is_finite() {
            gamma.clamp(0.0, 1.0)
        } else {
            0.9
        };
        self
    }

    /// Access the wrapped cascade router.
    #[must_use]
    pub const fn inner(&self) -> &CascadeRouter {
        &self.inner
    }

    /// Access the task graph used for lookahead planning.
    #[must_use]
    pub const fn task_graph(&self) -> &TaskDag {
        &self.task_graph
    }

    /// Access the cache reuse model.
    #[must_use]
    pub const fn cache_model(&self) -> &CacheReuseModel {
        &self.cache_model
    }

    /// Mutably access the cache reuse model.
    #[must_use]
    pub fn cache_model_mut(&mut self) -> &mut CacheReuseModel {
        &mut self.cache_model
    }

    /// Route with tier-downgrade lookahead.
    ///
    /// Before committing to the cascade-selected model, checks whether a
    /// cheaper tier has sufficiently high estimated success probability. If
    /// `P(success | cheaper_tier) > threshold`, the cheaper model is returned
    /// instead, saving cost without meaningful quality loss.
    /// `success_probability` gives a model's estimate, when it has one.
    #[must_use]
    pub fn route_with_lookahead(
        &self,
        ctx: &RoutingContext,
        success_probability: impl Fn(&str) -> Option<f64>,
        threshold: f64,
    ) -> CascadeModel {
        let baseline = self.inner.route(ctx);
        let baseline_tier = tier_rank_for_slug(&baseline.primary.slug);

        // Only attempt downgrade if we're at Standard or Premium.
        if baseline_tier == 0 {
            return baseline;
        }

        // Check each cheaper tier from cheapest up.
        let candidates = self.inner.model_slugs();
        for candidate_slug in candidates {
            let candidate_tier = tier_rank_for_slug(candidate_slug);
            if candidate_tier >= baseline_tier {
                continue;
            }

            if success_probability(candidate_slug).is_some_and(|success| success > threshold) {
                // Cheaper model is likely to succeed — use it.
                let mut downgraded = baseline.clone();
                downgraded.primary = ModelSpec::from_slug(candidate_slug);
                return downgraded;
            }
        }

        baseline
    }
}

/// Map a model slug to a tier rank (0 = fast/cheap, 1 = standard, 2 = premium).
fn tier_rank_for_slug(slug: &str) -> u8 {
    if slug.contains("gemini-2.5-flash-lite")
        || slug.contains("gemini-3.1-flash-lite-preview")
        || slug.contains("haiku")
    {
        0
    } else if slug.contains("opus")
        || slug.contains("premium")
        || slug.contains("gemini-3.1-pro-preview")
    {
        2
    } else {
        1
    }
}

// ---------------------------------------------------------------------------
// Public utility re-exports for runner-v2 wiring
// ---------------------------------------------------------------------------

/// Map a model slug to a tier rank (0 = fast/cheap, 1 = standard, 2 = premium).
///
/// This is the public wrapper around the internal `tier_rank_for_slug`
/// used by the runner-v2 event loop for inline lookahead logic.
#[must_use]
pub fn tier_rank(slug: &str) -> u8 {
    tier_rank_for_slug(slug)
}
