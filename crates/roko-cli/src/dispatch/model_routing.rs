//! Model routing — turn a task + dispatch context into a [`ModelSpec`].
//!
//! ## CLI flag disambiguation
//!
//! Three CLI names exist for model overrides:
//!
//! | Flag | Scope | Field | Notes |
//! |------|-------|-------|-------|
//! | `--model` | Global (before subcommand) | `Cli.model` | Primary name |
//! | `--force-model` | Global (before subcommand) | `Cli.model` | Alias for `--model` |
//! | `--force-backend` | `plan run` subcommand only | `PlanRun.force_backend` | Subcommand convenience; wins over global `--model` |
//!
//! All three ultimately populate `RunConfig.cli_model_override`, which the
//! event loop copies into `DispatchContext.force_backend`. This module reads
//! that field as the highest-priority input.
//!
//! ## Decision pipeline
//!
//! 1. **Manual override**. `force_backend` from CLI (`--model` /
//!    `--force-model` / `--force-backend`) wins unconditionally. This
//!    preserves the operator's ability to pin a model during incidents —
//!    and the choice is recorded so the feedback loop can learn from
//!    operator preferences.
//! 2. **Task hint**. `task_def.model_hint` (if any). Hints are author
//!    intent — not learned policy — and always beat the router.
//! 3. **CascadeRouter**. Only consulted when neither override nor hint
//!    applies. Returns a [`CascadeModel`] whose `primary` slug is used.
//! 4. **Safe default**. With no router and no hint, fall back to the
//!    `RunConfig.model` default. The router will eventually populate
//!    itself from observations.
//!
//! Every choice is wrapped in [`ModelChoice`] which records *why* the
//! model was picked. Feedback writers ([`runtime_feedback`]) use this to
//! tag observations: a forced override is recorded with
//! `forced = true` so the router doesn't conflate operator intent with
//! its own bandit signal.
//!
//! [`runtime_feedback`]: crate::runtime_feedback

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use roko_core::agent::ModelSpec;
use roko_core::task::TaskComplexityBand;
use roko_learn::cascade_router::{CascadeRouter, RoutingBias};
use roko_learn::latency::LatencyRegistry;
use roko_learn::model_router::RoutingContext;
use roko_learn::provider_health::ProviderHealthRegistry;

use super::DispatchContext;
use super::outcome::RunnerDispatchError;
use crate::task_parser::TaskDef;

// ─── Inputs ────────────────────────────────────────────────────────────

/// All inputs the router needs from the runner.
///
/// Constructed from a `TaskDef` + `DispatchContext`. Pre-extracting these
/// fields keeps the router pure and makes it trivial to test without
/// holding live runner state.
#[derive(Debug, Clone)]
pub struct RoutingInputs {
    /// Task domain (`"rust"`, `"docs"`, `"frontend"`, ...). Used by the
    /// router to bias toward domain-strong models.
    pub task_domain: Option<String>,
    /// Task tier (`"focused"`, `"deep"`, ...). Higher tiers can spend
    /// more per call.
    pub task_tier: String,
    /// Author-provided model hint (`task.model_hint`).
    pub task_model_hint: Option<String>,
    /// Operator override from CLI `--model` / `--force-model` / `--force-backend`.
    /// Highest priority: when set, the router returns this slug immediately.
    pub force_backend: Option<String>,
    /// Remaining USD budget for the plan.
    pub budget_remaining_usd: f64,
    /// Attempt number (0 = first try).
    pub attempt: u32,
    /// Role label.
    pub role: String,
    /// Full routing context for the CascadeRouter. When `Some`, the router
    /// calls `CascadeRouter::route()` instead of falling back to the default.
    pub routing_context: Option<RoutingContext>,
    /// Conductor routing bias derived from the live signal stream. When `Some`,
    /// deprioritized models are filtered out and prefer-cheaper scoring is
    /// applied so the cascade router avoids models the conductor flagged.
    pub routing_bias: Option<RoutingBias>,
    /// When `true`, plan spend has crossed the 80% threshold and the router
    /// should bias toward cheaper models. Set by the event loop when
    /// `BudgetAction::RouteToCheaper` fires.
    pub budget_pressure: bool,
}

impl RoutingInputs {
    /// Extract router inputs from a task + per-call context.
    #[must_use]
    pub fn from_task(task: &TaskDef, ctx: &DispatchContext) -> Self {
        Self {
            task_domain: task.domain.as_ref().map(|d| d.label().to_string()),
            task_tier: task.tier.clone(),
            task_model_hint: task.model_hint.clone().or_else(|| ctx.model_hint.clone()),
            force_backend: ctx.force_backend.clone(),
            budget_remaining_usd: ctx.budget_remaining_usd,
            attempt: ctx.attempt,
            role: ctx.role.clone(),
            routing_context: ctx.routing_context.clone(),
            routing_bias: ctx.routing_bias.clone(),
            budget_pressure: false,
        }
    }
}

// ─── Outputs ───────────────────────────────────────────────────────────

/// Why the router picked this model — preserved for feedback writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelChoiceSource {
    /// Operator override via `--model` / `--force-model` / `--force-backend`.
    Override,
    /// Author intent (`task.model_hint`).
    TaskHint,
    /// Returned by [`CascadeRouter`].
    Router,
    /// Fallback when no other signal was available.
    Default,
}

/// A picked model and the reason it was picked.
#[derive(Debug, Clone)]
pub struct ModelChoice {
    /// The resolved model spec (slug + backend + effort).
    pub model: ModelSpec,
    /// Why this model was picked.
    pub source: ModelChoiceSource,
}

impl ModelChoice {
    /// `true` if the choice came from an explicit operator override.
    #[must_use]
    pub fn forced(&self) -> bool {
        matches!(self.source, ModelChoiceSource::Override)
    }
}

// Compatibility shorthand expected by [`super::RunnerDispatchPlan`].
impl ModelChoice {
    /// Reuse the public `forced` flag exposed via the dispatcher facade.
    #[must_use]
    pub fn is_forced(&self) -> bool {
        self.forced()
    }
}

// ─── Router ────────────────────────────────────────────────────────────

/// Thin facade over [`CascadeRouter`] that applies the override / hint /
/// router / default precedence rules.
///
/// Holds an `Option<Arc<CascadeRouter>>` so callers without a configured
/// router (CI, smoke tests) still work. When the router is absent the
/// pipeline degrades to override → hint → default — never panics.
///
/// When a [`ProviderHealthRegistry`] is attached via
/// [`Self::with_provider_health`], the cascade stage uses
/// [`CascadeRouter::route_with_health_scored`] instead of the plain
/// `route` / `route_with_bias` path.  This filters out `Open`-circuit
/// providers and demotes `HalfOpen` ones so the selection automatically
/// avoids degraded backends.
#[derive(Clone)]
pub struct ModelRouter {
    cascade: Option<Arc<CascadeRouter>>,
    /// Default model slug used when override / hint / router all decline.
    /// Configurable so tests can inject a deterministic baseline.
    default_slug: String,
    /// Optional shared health registry.  When present, model selection
    /// filters out `Open` providers and demotes `HalfOpen` ones.
    health: Option<Arc<ProviderHealthRegistry>>,
    /// Map from model slug → provider id (e.g. `"claude-sonnet-4-6"` →
    /// `"anthropic"`).  Required for health-aware routing; without it
    /// health filtering is skipped.
    model_providers: HashMap<String, String>,
    /// Optional latency registry used to demote high-latency providers.
    /// When `latency_threshold_ms` is `None`, no latency demotion occurs.
    latency_registry: Option<Arc<LatencyRegistry>>,
    /// p95 latency ceiling in milliseconds.  Providers whose tracked p95
    /// exceeds this are demoted to secondary candidates during health-aware
    /// routing.  Has no effect when `latency_registry` is `None`.
    latency_threshold_ms: Option<f64>,
    /// Set of model slugs that have a configured, credential-ready provider
    /// in the current workspace.  When non-empty, cascade router results are
    /// filtered: a model whose slug is not in this set is replaced with the
    /// `default_slug` fallback.  Empty means no filtering (backwards compat).
    configured_models: HashSet<String>,
}

impl std::fmt::Debug for ModelRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelRouter")
            .field("cascade", &self.cascade.as_ref().map(|_| ".."))
            .field("default_slug", &self.default_slug)
            .field("health", &self.health.as_ref().map(|_| ".."))
            .field("model_providers", &self.model_providers.len())
            .field(
                "latency_registry",
                &self.latency_registry.as_ref().map(|_| ".."),
            )
            .field("latency_threshold_ms", &self.latency_threshold_ms)
            .field("configured_models", &self.configured_models.len())
            .finish()
    }
}

impl ModelRouter {
    /// Construct a router. Callers without a `CascadeRouter` pass `None`.
    pub fn new(cascade: Option<Arc<CascadeRouter>>) -> Self {
        Self {
            cascade,
            default_slug: roko_core::defaults::MODEL_FOCUSED.to_string(),
            health: None,
            model_providers: HashMap::new(),
            latency_registry: None,
            latency_threshold_ms: None,
            configured_models: HashSet::new(),
        }
    }

    /// Clone the inner cascade router `Arc` (for factory cache swap).
    #[must_use]
    pub fn cascade_arc(&self) -> Option<Arc<CascadeRouter>> {
        self.cascade.clone()
    }

    /// Override the default-fallback slug.
    pub fn with_default_slug(mut self, slug: impl Into<String>) -> Self {
        self.default_slug = slug.into();
        self
    }

    /// Attach a provider health registry and a model->provider map.
    ///
    /// When both are provided, the cascade routing stage calls
    /// [`CascadeRouter::route_with_health_scored`] so `Open`-circuit providers
    /// are filtered and `HalfOpen` ones are demoted.
    #[must_use]
    pub fn with_provider_health(
        mut self,
        health: Arc<ProviderHealthRegistry>,
        model_providers: HashMap<String, String>,
    ) -> Self {
        self.health = Some(health);
        self.model_providers = model_providers;
        self
    }

    /// Attach a latency registry and ceiling.
    ///
    /// Providers whose tracked p95 latency exceeds `threshold_ms` are
    /// treated as secondary candidates when health-aware routing is active.
    /// Has no effect unless [`Self::with_provider_health`] is also called.
    #[must_use]
    pub fn with_latency_demotion(
        mut self,
        registry: Arc<LatencyRegistry>,
        threshold_ms: f64,
    ) -> Self {
        self.latency_registry = Some(registry);
        self.latency_threshold_ms = Some(threshold_ms);
        self
    }

    /// Restrict cascade router results to models that have a configured,
    /// credential-ready provider in the current workspace.
    ///
    /// When non-empty, any cascade router result whose slug is absent from
    /// `models` is replaced with the `default_slug` fallback.  When empty
    /// (the default), no filtering occurs — preserving backwards
    /// compatibility.
    #[must_use]
    pub fn with_configured_models(mut self, models: HashSet<String>) -> Self {
        self.configured_models = models;
        self
    }

    /// Apply the precedence pipeline.
    ///
    /// When a conductor [`RoutingBias`] is supplied through `inputs.routing_bias`,
    /// the bias is applied to the cascade router selection: deprioritized models
    /// are filtered out and `prefer_cheaper` shifts scoring toward cheaper tiers.
    /// The bias is only consulted for router-driven selections -- overrides and
    /// task hints are never affected, preserving operator and author intent.
    ///
    /// When `inputs.budget_pressure` is `true` (plan spend > 80%), the router
    /// merges a `prefer_cheaper` bias into the cascade selection so cheaper
    /// models are favored automatically.
    ///
    /// When a [`ProviderHealthRegistry`] is attached via
    /// [`Self::with_provider_health`], the cascade stage calls
    /// [`CascadeRouter::route_with_health_scored`] which filters `Open`-circuit
    /// providers and demotes `HalfOpen` / high-latency ones, ensuring the
    /// selected model has a healthy provider.
    pub fn route(&self, inputs: &RoutingInputs) -> Result<ModelChoice, RunnerDispatchError> {
        if let Some(slug) = inputs.force_backend.as_ref() {
            return Ok(ModelChoice {
                model: ModelSpec::from_slug(slug),
                source: ModelChoiceSource::Override,
            });
        }
        if let Some(slug) = inputs.task_model_hint.as_ref() {
            return Ok(ModelChoice {
                model: ModelSpec::from_slug(slug),
                source: ModelChoiceSource::TaskHint,
            });
        }
        if let Some(router) = self.cascade.as_ref() {
            if let Some(ctx) = &inputs.routing_context {
                // Merge budget pressure into routing bias when applicable.
                let effective_bias = Self::effective_bias(inputs);

                let cascade_model = if let Some(health) = &self.health {
                    // Health-aware path: filters Open providers, demotes HalfOpen
                    // and optionally high-latency providers.
                    let latency_ref = self.latency_registry.as_deref();
                    router.route_with_health_scored(
                        ctx,
                        health,
                        &self.model_providers,
                        latency_ref,
                        self.latency_threshold_ms,
                    )
                } else if let Some(bias) = &effective_bias {
                    // Conductor / budget bias path (no health data).
                    if bias.deprioritize.is_empty() && !bias.prefer_cheaper {
                        router.route(ctx)
                    } else {
                        router.route_with_bias(ctx, bias)
                    }
                } else {
                    router.route(ctx)
                };
                // Guard: if the workspace has a known set of configured
                // providers, reject models that lack credentials.  This
                // prevents the cascade router from selecting a model whose
                // provider isn't available (e.g. learned state referencing
                // `claude-opus` when no Anthropic key is present).
                if !self.configured_models.is_empty()
                    && !self.configured_models.contains(&cascade_model.primary.slug)
                {
                    tracing::warn!(
                        selected = %cascade_model.primary.slug,
                        fallback = %self.default_slug,
                        "cascade router selected model without a configured provider; \
                         falling back to default"
                    );
                    return Ok(ModelChoice {
                        model: ModelSpec::from_slug(&self.default_slug),
                        source: ModelChoiceSource::Router,
                    });
                }
                return Ok(ModelChoice {
                    model: cascade_model.primary,
                    source: ModelChoiceSource::Router,
                });
            }
            // No RoutingContext → degrade to default (CI, smoke tests).
            return Ok(ModelChoice {
                model: ModelSpec::from_slug(&self.default_slug),
                source: ModelChoiceSource::Default,
            });
        }
        Ok(ModelChoice {
            model: ModelSpec::from_slug(&self.default_slug),
            source: ModelChoiceSource::Default,
        })
    }

    /// Route with structured logging — emits `tracing::info!` for every
    /// decision and `debug!` cascade candidate scores when available.
    pub fn route_logged(
        &self,
        inputs: &RoutingInputs,
        task_id: &str,
    ) -> Result<ModelChoice, RunnerDispatchError> {
        let choice = self.route(inputs)?;
        tracing::info!(
            task_id,
            model = %choice.model.slug,
            source = ?choice.source,
            budget_pressure = inputs.budget_pressure,
            "model routed"
        );
        if choice.source == ModelChoiceSource::Router {
            if let Some(router) = self.cascade.as_ref() {
                if let Some(ctx) = &inputs.routing_context {
                    let explanation = router.explain_route(ctx, None);
                    tracing::debug!(
                        task_id,
                        stage = %explanation.stage,
                        observations = explanation.observations,
                        "routing candidates: {:?}",
                        explanation
                            .candidates
                            .iter()
                            .take(3)
                            .map(|c| (c.slug.as_str(), c.score))
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
        Ok(choice)
    }

    /// Merge `budget_pressure` into the existing `routing_bias` when the
    /// plan budget has crossed the 80% threshold.
    fn effective_bias(inputs: &RoutingInputs) -> Option<RoutingBias> {
        match (&inputs.routing_bias, inputs.budget_pressure) {
            // Budget pressure with existing bias — merge prefer_cheaper.
            (Some(bias), true) => Some(RoutingBias {
                deprioritize: bias.deprioritize.clone(),
                prefer_cheaper: true,
                reason: if bias.reason.is_empty() {
                    "budget >80%".into()
                } else {
                    format!("{}; budget >80%", bias.reason)
                },
            }),
            // Budget pressure without existing bias — new bias.
            (None, true) => Some(RoutingBias {
                deprioritize: vec![],
                prefer_cheaper: true,
                reason: "budget >80%".into(),
            }),
            // Existing bias, no pressure — pass through.
            (Some(bias), false) => Some(bias.clone()),
            // No bias, no pressure.
            (None, false) => None,
        }
    }
}

/// Map a task tier string to a [`TaskComplexityBand`].
pub(crate) fn tier_to_complexity(tier: &str) -> TaskComplexityBand {
    match tier {
        "focused" | "quick" | "trivial" => TaskComplexityBand::Fast,
        "deep" | "architectural" | "complex" => TaskComplexityBand::Complex,
        _ => TaskComplexityBand::Standard,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn task() -> TaskDef {
        TaskDef {
            id: "t".into(),
            title: "t".into(),
            description: None,
            role: Some("implementer".into()),
            status: "ready".into(),
            tier: "focused".into(),
            frequency: None,
            model_hint: None,
            replan_strategy: None,
            max_loc: None,
            files: vec![],
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: vec![],
            depends_on_plan: vec![],
            split_into: None,
            context: None,
            verify: vec![],
            timeout_secs: 60,
            max_retries: 1,
            acceptance: vec![],
            acceptance_contract: None,
            domain: Some(roko_core::task::TaskDomain::Code),
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
        }
    }

    fn ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            prompt_experiment: None,
            gate_feedback: None,
            routing_context: None,
            routing_bias: None,
            dependency_outputs: Vec::new(),
        }
    }

    #[test]
    fn override_wins_over_everything() {
        let mut t = task();
        t.model_hint = Some("claude-sonnet-4-6".into());
        let mut c = ctx();
        c.force_backend = Some("gpt-5".into());
        let inputs = RoutingInputs::from_task(&t, &c);
        let router = ModelRouter::new(None);
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, "gpt-5");
        assert_eq!(choice.source, ModelChoiceSource::Override);
        assert!(choice.forced());
    }

    #[test]
    fn task_hint_beats_router_when_no_override() {
        let mut t = task();
        t.model_hint = Some("claude-haiku-4-5".into());
        let inputs = RoutingInputs::from_task(&t, &ctx());
        let router = ModelRouter::new(None);
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, "claude-haiku-4-5");
        assert_eq!(choice.source, ModelChoiceSource::TaskHint);
        assert!(!choice.forced());
    }

    #[test]
    fn default_fallback_when_router_absent() {
        let inputs = RoutingInputs::from_task(&task(), &ctx());
        let router = ModelRouter::new(None).with_default_slug("custom-default");
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, "custom-default");
        assert_eq!(choice.source, ModelChoiceSource::Default);
    }

    fn routing_context() -> RoutingContext {
        use roko_core::task::TaskCategory;
        use roko_core::{AgentRole, BehavioralState, DaimonPolicy};

        RoutingContext {
            task_category: TaskCategory::Implementation,
            complexity: TaskComplexityBand::Standard,
            iteration: 0,
            role: AgentRole::Implementer,
            crate_familiarity: 0.5,
            has_prior_failure: false,
            conductor_load: 0.0,
            active_agents: 0,
            ready_queue_depth: 0,
            max_queue_wait_hours: 0.0,
            daimon_policy: DaimonPolicy::new(0.5, BehavioralState::Engaged),
            thinking_level: None,
            temperament: None,
            previous_model: None,
            plan_context_tokens: None,
            tier_thresholds: None,
            cfactor: None,
        }
    }

    #[test]
    fn cascade_router_called_when_context_present() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router);
        // CascadeRouter picks from the configured slugs.
        assert!(
            choice.model.slug == "claude-sonnet-4-6" || choice.model.slug == "gpt-5",
            "expected one of the configured slugs, got {:?}",
            choice.model.slug,
        );
    }

    #[test]
    fn no_context_degrades_to_default() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade)).with_default_slug("fallback-model");
        let inputs = RoutingInputs::from_task(&task(), &ctx());
        // routing_context is None via from_task()
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, "fallback-model");
        assert_eq!(choice.source, ModelChoiceSource::Default);
    }

    #[test]
    fn tier_to_complexity_mapping() {
        assert_eq!(tier_to_complexity("focused"), TaskComplexityBand::Fast);
        assert_eq!(tier_to_complexity("quick"), TaskComplexityBand::Fast);
        assert_eq!(tier_to_complexity("trivial"), TaskComplexityBand::Fast);
        assert_eq!(tier_to_complexity("deep"), TaskComplexityBand::Complex);
        assert_eq!(
            tier_to_complexity("architectural"),
            TaskComplexityBand::Complex
        );
        assert_eq!(tier_to_complexity("complex"), TaskComplexityBand::Complex);
        assert_eq!(tier_to_complexity("standard"), TaskComplexityBand::Standard);
        assert_eq!(tier_to_complexity("anything"), TaskComplexityBand::Standard);
    }

    // ── Conductor routing bias tests (E08-T07) ─────────────────────────

    #[test]
    fn conductor_routing_bias_deprioritizes_model() {
        // Two-model router: sonnet and haiku. When sonnet is deprioritized,
        // the router should pick haiku instead.
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        inputs.routing_bias = Some(RoutingBias {
            deprioritize: vec!["claude-sonnet-4-6".into()],
            prefer_cheaper: false,
            reason: "recent failure on claude-sonnet-4-6".into(),
        });
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router);
        // With sonnet deprioritized, the router should avoid it.
        assert_eq!(
            choice.model.slug, "claude-haiku-4-5",
            "deprioritized model should be avoided when alternatives exist"
        );
    }

    #[test]
    fn conductor_routing_bias_neutral_does_not_alter_route() {
        // A neutral bias (no deprioritize, no prefer_cheaper) should behave
        // identically to having no bias at all.
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        // Neutral bias -- should not change routing outcome.
        inputs.routing_bias = Some(RoutingBias {
            deprioritize: vec![],
            prefer_cheaper: false,
            reason: String::new(),
        });
        let with_bias = router.route(&inputs).unwrap();

        // Same inputs without any bias.
        inputs.routing_bias = None;
        let without_bias = router.route(&inputs).unwrap();

        assert_eq!(
            with_bias.model.slug, without_bias.model.slug,
            "neutral routing bias must not alter model selection"
        );
    }

    #[test]
    fn conductor_routing_bias_fallback_when_all_deprioritized() {
        // If all models are deprioritized, the router should gracefully
        // fall back rather than panicking or returning nothing.
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        inputs.routing_bias = Some(RoutingBias {
            deprioritize: vec!["claude-sonnet-4-6".into(), "claude-haiku-4-5".into()],
            prefer_cheaper: false,
            reason: "all models failing".into(),
        });
        // Should not panic -- route_with_bias falls back to unbiased route
        // when filtering removes all candidates.
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router);
        assert!(
            !choice.model.slug.is_empty(),
            "router must return a model even when all are deprioritized"
        );
    }

    #[test]
    fn conductor_routing_bias_does_not_override_force_backend() {
        // Even with conductor bias, force_backend must always win.
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut c = ctx();
        c.force_backend = Some("gpt-5".into());
        let mut inputs = RoutingInputs::from_task(&task(), &c);
        inputs.routing_bias = Some(RoutingBias {
            deprioritize: vec!["gpt-5".into()],
            prefer_cheaper: true,
            reason: "should not matter for forced".into(),
        });
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, "gpt-5");
        assert_eq!(choice.source, ModelChoiceSource::Override);
    }

    #[test]
    fn conductor_routing_bias_does_not_override_task_hint() {
        // Even with conductor bias, task hints must still win.
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut t = task();
        t.model_hint = Some("claude-sonnet-4-6".into());
        let mut inputs = RoutingInputs::from_task(&t, &ctx());
        inputs.routing_bias = Some(RoutingBias {
            deprioritize: vec!["claude-sonnet-4-6".into()],
            prefer_cheaper: true,
            reason: "should not matter for hint".into(),
        });
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, "claude-sonnet-4-6");
        assert_eq!(choice.source, ModelChoiceSource::TaskHint);
    }

    // ── Budget pressure tests ──────────────────────────────────────────

    #[test]
    fn budget_pressure_injects_prefer_cheaper_bias() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        inputs.budget_pressure = true;
        // Should not panic and should produce a valid model.
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router);
        assert!(
            !choice.model.slug.is_empty(),
            "budget pressure must still produce a valid model"
        );
    }

    #[test]
    fn budget_pressure_merges_with_existing_bias() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        inputs.routing_bias = Some(RoutingBias {
            deprioritize: vec!["claude-sonnet-4-6".into()],
            prefer_cheaper: false,
            reason: "conductor signal".into(),
        });
        inputs.budget_pressure = true;
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router);
        // With sonnet deprioritized AND prefer_cheaper, haiku should win.
        assert_eq!(
            choice.model.slug, "claude-haiku-4-5",
            "budget pressure + deprioritize should strongly prefer the cheaper model"
        );
    }

    #[test]
    fn budget_pressure_does_not_override_force_backend() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        let mut c = ctx();
        c.force_backend = Some("gpt-5".into());
        let mut inputs = RoutingInputs::from_task(&task(), &c);
        inputs.budget_pressure = true;
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, "gpt-5");
        assert_eq!(choice.source, ModelChoiceSource::Override);
    }

    // ── Provider-health routing tests (E48-T08) ────────────────────────

    /// When a health registry is attached and a provider is Open,
    /// `ModelRouter::route` must use the health-aware path.
    #[test]
    fn health_aware_route_excludes_open_provider() {
        use roko_learn::provider_health::ErrorClass;

        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gemini-2.5-flash".into(),
        ]));

        let health = Arc::new(ProviderHealthRegistry::new());
        // Trip anthropic to Open.
        health.record_failure("anthropic", ErrorClass::RateLimit);
        health.record_failure("anthropic", ErrorClass::RateLimit);
        health.record_failure("anthropic", ErrorClass::RateLimit);

        let mut model_providers = HashMap::new();
        model_providers.insert("claude-sonnet-4-6".into(), "anthropic".into());
        model_providers.insert("gemini-2.5-flash".into(), "google".into());

        let router = ModelRouter::new(Some(cascade)).with_provider_health(health, model_providers);

        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());

        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router);
        assert_eq!(
            choice.model.slug, "gemini-2.5-flash",
            "Open anthropic must be excluded; gemini must be selected"
        );
    }

    /// Health registry attached but no providers are degraded --
    /// routing behaves normally.
    #[test]
    fn health_aware_route_normal_when_all_healthy() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gemini-2.5-flash".into(),
        ]));

        let health = Arc::new(ProviderHealthRegistry::new());
        health.record_success("anthropic");
        health.record_success("google");

        let mut model_providers = HashMap::new();
        model_providers.insert("claude-sonnet-4-6".into(), "anthropic".into());
        model_providers.insert("gemini-2.5-flash".into(), "google".into());

        let router = ModelRouter::new(Some(cascade)).with_provider_health(health, model_providers);

        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());

        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router);
        assert!(
            choice.model.slug == "claude-sonnet-4-6" || choice.model.slug == "gemini-2.5-flash",
            "must pick from the configured slugs"
        );
    }

    /// Even with provider health attached, force_backend overrides everything.
    #[test]
    fn health_does_not_override_force_backend() {
        use roko_learn::provider_health::ErrorClass;

        let cascade = Arc::new(CascadeRouter::new(vec!["claude-sonnet-4-6".into()]));
        let health = Arc::new(ProviderHealthRegistry::new());
        // Trip the only provider Open.
        for _ in 0..3 {
            health.record_failure("anthropic", ErrorClass::RateLimit);
        }

        let mut model_providers = HashMap::new();
        model_providers.insert("claude-sonnet-4-6".into(), "anthropic".into());

        let router = ModelRouter::new(Some(cascade)).with_provider_health(health, model_providers);

        let mut c = ctx();
        c.force_backend = Some("gpt-5".into());
        let inputs = RoutingInputs::from_task(&task(), &c);
        let choice = router.route(&inputs).unwrap();

        assert_eq!(choice.model.slug, "gpt-5");
        assert_eq!(choice.source, ModelChoiceSource::Override);
    }

    // ── Configured-models filtering tests (dogfood critical) ─────────

    #[test]
    fn cascade_router_falls_back_when_model_not_configured() {
        // Cascade router knows about model-b but the workspace only has
        // model-a configured.  The router must fall back to the default
        // rather than returning a model without credentials.
        let cascade = Arc::new(CascadeRouter::new(vec!["model-b".into()]));
        let configured: HashSet<String> = ["model-a".into()].into_iter().collect();
        let router = ModelRouter::new(Some(cascade))
            .with_default_slug("model-a")
            .with_configured_models(configured);

        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());

        let choice = router.route(&inputs).unwrap();
        assert_eq!(
            choice.model.slug, "model-a",
            "must fall back to default when cascade picks an unconfigured model"
        );
        assert_eq!(
            choice.source,
            ModelChoiceSource::Router,
            "source must remain Router (the router made the decision, just filtered)"
        );
    }

    #[test]
    fn cascade_router_passes_through_when_model_is_configured() {
        // Use two well-known model slugs so the cascade router's internal
        // role/tier logic picks one of them. Both are in the configured set,
        // so whichever the router picks must pass through.
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let configured: HashSet<String> = ["claude-sonnet-4-6".into(), "claude-haiku-4-5".into()]
            .into_iter()
            .collect();
        let router = ModelRouter::new(Some(cascade))
            .with_default_slug("fallback-default")
            .with_configured_models(configured.clone());

        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());

        let choice = router.route(&inputs).unwrap();
        assert!(
            configured.contains(&choice.model.slug),
            "configured model must pass through without fallback, got {:?}",
            choice.model.slug,
        );
        assert_ne!(
            choice.model.slug, "fallback-default",
            "should NOT have fallen back since the cascade model is configured"
        );
        assert_eq!(choice.source, ModelChoiceSource::Router);
    }

    #[test]
    fn empty_configured_models_skips_filtering() {
        // When configured_models is empty, no filtering occurs (backwards compat).
        // Use real model slugs that the cascade router recognises.
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade));
        // configured_models defaults to empty -- no filtering.

        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());

        let choice = router.route(&inputs).unwrap();
        assert!(
            choice.model.slug == "claude-sonnet-4-6" || choice.model.slug == "claude-haiku-4-5",
            "empty configured_models must not filter; got {:?}",
            choice.model.slug,
        );
        assert_eq!(choice.source, ModelChoiceSource::Router);
    }
}
