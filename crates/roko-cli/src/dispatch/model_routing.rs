//! Model routing — turn a task + dispatch context into a [`ModelSpec`].
//!
//! ## CLI flag
//!
//! A single global `--model` flag (with hidden aliases `--force-model` and
//! `--force-backend` for backward compatibility) populates
//! `RunConfig.cli_model_override`, which the event loop copies into
//! `DispatchContext.force_backend`. This module reads that field as the
//! highest-priority input.
//!
//! ## Decision pipeline
//!
//! 1. **Manual override**. `force_backend` from CLI (`--model`) wins
//!    unconditionally. This
//!    preserves the operator's ability to pin a model during incidents —
//!    and the choice is recorded so the feedback loop can learn from
//!    operator preferences.
//! 2. **Task hint**. `task_def.model_hint` (if any). Hints are author
//!    intent — not learned policy — and always beat the router.
//! 3. **Ladder**. With a [`RoutingLadder`] attached (`[routing.ladder]`, on
//!    by default), the task's role and tier pick its start rung. The
//!    cascade router's pick is only logged beside it (shadow).
//! 4. **CascadeRouter**. Only consulted when no override, hint or ladder
//!    rung applies. Returns a [`CascadeModel`] whose `primary` slug is used.
//! 5. **Safe default**. With no router and no hint, fall back to the
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

use indexmap::IndexMap;
use roko_core::agent::ModelSpec;
use roko_core::config::routing::LadderConfig;
use roko_core::config::schema::{ModelProfile, RokoConfig};
use roko_core::task::{TaskCategory, TaskTier};
use roko_learn::cascade_router::{CascadeModel, CascadeRouter, RoutingBias};
use roko_learn::latency::LatencyRegistry;
use roko_learn::model_router::RoutingContext;
use roko_learn::provider_health::ProviderHealthRegistry;

use super::DispatchContext;
use super::outcome::RunnerDispatchError;
use crate::task_parser::TaskDef;

/// Returns `true` when the task category requires tool use.
///
/// Implementation, scaffolding, integration, verification, refactoring, and
/// infrastructure tasks all need the model to support tool calls.  Only
/// research and documentation tasks can proceed without tools.
fn needs_tool_use(cat: TaskCategory) -> bool {
    !matches!(cat, TaskCategory::Research | TaskCategory::Docs)
}

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
    /// Task tier, read by [`TaskDef::tier_class`] (an unknown tier is
    /// focused).
    pub task_tier: TaskTier,
    /// Author-provided model hint (`task.model_hint`).
    pub task_model_hint: Option<String>,
    /// Operator override from the unified CLI `--model` flag.
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
            task_tier: task.tier_class(),
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
    /// Operator override via the unified `--model` flag.
    Override,
    /// Author intent (`task.model_hint`).
    TaskHint,
    /// The task's start rung on `[routing.ladder]` ([`RoutingLadder`]).
    /// Feedback records its outcome like a router pick, so the learner
    /// sees every rung.
    Ladder {
        /// Index of the rung among the task's rungs, cheapest first.
        rung: usize,
    },
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
    /// Provider IDs that the operator has statically disabled via
    /// `[routing] disabled_providers`.  Models backed by a disabled provider
    /// are rejected in the same way as models without credentials.
    disabled_providers: HashSet<String>,
    /// Model slugs whose `ModelProfile.supports_tools` is `false`.
    ///
    /// When a task requires tool use (determined by [`needs_tool_use`]),
    /// cascade router results that land in this set are rejected and replaced
    /// with the `default_slug` fallback.
    models_without_tools: HashSet<String>,
    /// `[routing.ladder]` bound to this workspace. When present, it picks
    /// the model of every task without an override or hint.
    ladder: Option<RoutingLadder>,
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
            .field("disabled_providers", &self.disabled_providers.len())
            .field("models_without_tools", &self.models_without_tools.len())
            .field("ladder", &self.ladder)
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
            disabled_providers: HashSet::new(),
            models_without_tools: HashSet::new(),
            ladder: None,
        }
    }

    /// Start every task without an override or hint on its
    /// `[routing.ladder]` rung. The cascade router's pick is then only
    /// logged.
    #[must_use]
    pub fn with_routing_ladder(mut self, ladder: RoutingLadder) -> Self {
        self.ladder = Some(ladder);
        self
    }

    /// The attached routing ladder.
    #[must_use]
    pub fn routing_ladder(&self) -> Option<&RoutingLadder> {
        self.ladder.as_ref()
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

    /// Exclude models whose provider ID appears in `providers`.
    ///
    /// Populated from `[routing] disabled_providers` in `roko.toml`.
    /// Models backed by a disabled provider are rejected at the same
    /// precedence level as unconfigured models (after the cascade router
    /// selects, before returning the choice).
    #[must_use]
    pub fn with_disabled_providers(mut self, providers: HashSet<String>) -> Self {
        self.disabled_providers = providers;
        self
    }

    /// Register model slugs that lack tool-use support.
    ///
    /// When a task requires tool use (implementation, scaffolding, integration,
    /// verification, refactoring, infrastructure), cascade router results whose
    /// slug is in this set are rejected and replaced with the `default_slug`
    /// fallback.  Research and documentation tasks are unaffected.
    #[must_use]
    pub fn with_tool_capability_filter(mut self, models_without_tools: HashSet<String>) -> Self {
        self.models_without_tools = models_without_tools;
        self
    }

    /// Apply the precedence pipeline.
    ///
    /// Override, then task hint, then the `[routing.ladder]` start rung
    /// (when a ladder is attached and a rung of the task's ladder can run),
    /// then the cascade router, then the default.
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
        if let Some(choice) = self.ladder_choice(inputs) {
            return Ok(choice);
        }
        if let Some(router) = self.cascade.as_ref() {
            if let Some(ctx) = &inputs.routing_context {
                let cascade_model = self.cascade_pick(router, ctx, inputs);
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
                // Guard: reject models whose provider is statically disabled
                // via `[routing] disabled_providers`.
                if !self.disabled_providers.is_empty() {
                    if let Some(provider_id) = self.model_providers.get(&cascade_model.primary.slug)
                    {
                        if self.disabled_providers.contains(provider_id) {
                            tracing::info!(
                                selected = %cascade_model.primary.slug,
                                provider = %provider_id,
                                fallback = %self.default_slug,
                                "provider is statically disabled; falling back to default"
                            );
                            return Ok(ModelChoice {
                                model: ModelSpec::from_slug(&self.default_slug),
                                source: ModelChoiceSource::Router,
                            });
                        }
                    }
                }
                // Guard: reject models that lack tool-use support when the
                // task category requires tools (implementation, scaffolding,
                // integration, verification, refactoring, infrastructure).
                if !self.models_without_tools.is_empty()
                    && needs_tool_use(ctx.task_category)
                    && self
                        .models_without_tools
                        .contains(&cascade_model.primary.slug)
                {
                    tracing::warn!(
                        selected = %cascade_model.primary.slug,
                        fallback = %self.default_slug,
                        task_category = ?ctx.task_category,
                        "model does not support tool use required by task category; \
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

    /// The task's start rung on the attached `[routing.ladder]`, logged
    /// beside the cascade router's own pick (shadow). `None` without a
    /// ladder, or when no rung of the task's ladder can run.
    fn ladder_choice(&self, inputs: &RoutingInputs) -> Option<ModelChoice> {
        let start = self
            .ladder
            .as_ref()?
            .start(&inputs.role, inputs.task_tier)?;
        let shadow = self
            .cascade
            .as_ref()
            .zip(inputs.routing_context.as_ref())
            .map(|(router, ctx)| self.cascade_pick(router, ctx, inputs).primary.slug);
        tracing::info!(
            role = %inputs.role,
            tier = %inputs.task_tier,
            rung = %start.name,
            model = %start.model,
            router_pick = shadow.as_deref().unwrap_or("none"),
            "model routed by the ladder"
        );
        Some(ModelChoice {
            model: ModelSpec::from_slug(start.model),
            source: ModelChoiceSource::Ladder { rung: start.index },
        })
    }

    /// The cascade router's pick for `ctx`, before the provider guards.
    fn cascade_pick(
        &self,
        router: &CascadeRouter,
        ctx: &RoutingContext,
        inputs: &RoutingInputs,
    ) -> CascadeModel {
        // Merge budget pressure into routing bias when applicable.
        let effective_bias = Self::effective_bias(inputs);

        if let Some(health) = &self.health {
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
        }
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

// ─── Ladder ────────────────────────────────────────────────────────────

/// `[routing.ladder]` bound to the models this workspace can dispatch.
///
/// Built once per run by [`Self::from_config`], which logs each rung it
/// skips; [`Self::start`] then picks each task's start rung.
#[derive(Debug, Clone)]
pub struct RoutingLadder {
    config: LadderConfig,
    /// Rung model as configured → the model name dispatch runs, for every
    /// rung model this workspace can dispatch.
    runnable: HashMap<String, String>,
}

/// The rung a task starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderStartRung {
    /// Index of the rung among the task's rungs, cheapest first.
    pub index: usize,
    /// Rung name (`"cheap"`).
    pub name: String,
    /// Model to dispatch: the `[models.*]` entry's slug, or its key when the
    /// slug alone would not select that entry.
    pub model: String,
}

impl RoutingLadder {
    /// Bind `config.routing.ladder` to this workspace. `None` when the
    /// ladder is off or none of its rungs can run here, which leaves routing
    /// as it was.
    #[must_use]
    pub fn from_config(config: &RokoConfig) -> Option<Self> {
        Self::from_config_with(config, |key| config.provider_available_for_model_key(key))
    }

    /// [`Self::from_config`] with an injectable provider-availability check
    /// on a `[models.*]` key.
    pub(crate) fn from_config_with(
        config: &RokoConfig,
        available: impl Fn(&str) -> bool,
    ) -> Option<Self> {
        let ladder = &config.routing.ladder;
        if !ladder.enabled {
            return None;
        }
        for issue in ladder.issues() {
            tracing::warn!(%issue, "routing ladder is misconfigured");
        }
        let models = config.effective_models();
        let mut runnable = HashMap::new();
        let mut seen = HashSet::new();
        for rung in ladder.all_rungs() {
            if !seen.insert(rung.model.as_str()) {
                continue;
            }
            match rung_dispatch_model(config, &models, &rung.model, &available) {
                Ok(model) => {
                    runnable.insert(rung.model.clone(), model);
                }
                Err(reason) => tracing::info!(
                    rung = %rung.name,
                    model = %rung.model,
                    reason,
                    "routing ladder: skipping a rung this workspace cannot run"
                ),
            }
        }
        if runnable.is_empty() {
            tracing::info!("routing ladder: no rung can run in this workspace; the router picks");
            return None;
        }
        Some(Self {
            config: ladder.clone(),
            runnable,
        })
    }

    /// The rung a task of `tier` in `role` starts on, or `None` when no rung
    /// of its ladder can run.
    #[must_use]
    pub fn start(&self, role: &str, tier: TaskTier) -> Option<LadderStartRung> {
        let resolved = self
            .config
            .resolve(role, tier, |model| self.runnable.contains_key(model))?;
        let rung = resolved.start_rung();
        Some(LadderStartRung {
            index: resolved.start,
            name: rung.name.clone(),
            model: self.runnable.get(&rung.model)?.clone(),
        })
    }
}

/// The model name dispatch runs for a rung's model, or why the rung cannot
/// run. The model must name a `[models.*]` entry (by key, else by slug)
/// that can call tools, whose provider is not in
/// `routing.disabled_providers` and is `available`.
fn rung_dispatch_model(
    config: &RokoConfig,
    models: &IndexMap<String, ModelProfile>,
    model: &str,
    available: impl Fn(&str) -> bool,
) -> Result<String, &'static str> {
    let model = model.trim();
    let (key, profile) = models
        .get_key_value(model)
        .or_else(|| models.iter().find(|(_, profile)| profile.slug == model))
        .ok_or("no [models.*] entry has this key or slug")?;
    if profile.is_embedding_model || !profile.supports_tools {
        return Err("the model cannot call tools");
    }
    if config
        .routing
        .disabled_providers
        .contains(&profile.provider)
    {
        return Err("its provider is in routing.disabled_providers");
    }
    if !available(key) {
        return Err("its provider has no credentials or command");
    }
    // Dispatch reads a model name as a `[models.*]` key before a slug, so the
    // slug selects this entry only when no other entry uses it as a key or
    // a slug.
    let slug = profile.slug.trim();
    let slug_is_unique = !slug.is_empty()
        && models
            .iter()
            .filter(|(other_key, _)| *other_key != key)
            .all(|(other_key, other)| other_key.as_str() != slug && other.slug.trim() != slug);
    Ok(if slug_is_unique { slug } else { key.as_str() }.to_string())
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::task::TaskComplexityBand;
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
            accept: None,
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
            error_patterns_context: String::new(),
            cached_workspace_map: String::new(),
            cached_workspace_context: String::new(),
            cached_cfactor_context: String::new(),
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
    fn routing_inputs_read_the_tier_through_task_tier() {
        let mut t = task();
        for (tier, expected) in [
            ("mechanical", TaskTier::Mechanical),
            ("Trivial", TaskTier::Mechanical),
            ("integrative", TaskTier::Integrative),
            ("deep", TaskTier::Architectural),
            ("mechancial", TaskTier::Focused),
        ] {
            t.tier = tier.into();
            assert_eq!(RoutingInputs::from_task(&t, &ctx()).task_tier, expected);
        }
    }

    // ── Routing ladder (gap-9cbf35) ────────────────────────────────────

    /// A workspace with D11's three executor models and Sonnet.
    fn ladder_config() -> RokoConfig {
        let mut config = RokoConfig::default();
        config.models.clear();
        for (key, provider, slug) in [
            ("cerebras-gptoss", "cerebras", "gpt-oss-120b"),
            ("glm-4-7", "zai", "glm-4.7"),
            ("gpt-5-4-mini", "openai", "gpt-5.4-mini"),
            ("claude-sonnet", "claude_cli", "claude-sonnet-4-6"),
        ] {
            config.models.insert(
                key.to_string(),
                ModelProfile {
                    provider: provider.to_string(),
                    slug: slug.to_string(),
                    supports_tools: true,
                    ..ModelProfile::default()
                },
            );
        }
        config
    }

    fn ladder(config: &RokoConfig, available: impl Fn(&str) -> bool) -> RoutingLadder {
        RoutingLadder::from_config_with(config, available).expect("a rung can run")
    }

    /// Route `task` for `ctx` through a router with a cascade and `ladder`.
    fn ladder_route(ladder: RoutingLadder, task: &TaskDef, ctx: &DispatchContext) -> ModelChoice {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade)).with_routing_ladder(ladder);
        let mut inputs = RoutingInputs::from_task(task, ctx);
        inputs.routing_context = Some(routing_context());
        router.route(&inputs).unwrap()
    }

    fn routed(choice: &ModelChoice) -> (&str, ModelChoiceSource) {
        (choice.model.slug.as_str(), choice.source)
    }

    #[test]
    fn ladder_routes_by_role_and_tier() {
        use roko_core::config::routing::{LadderRoleConfig, LadderStart};

        let everywhere = ladder(&ladder_config(), |_| true);
        let mut t = task();

        // With no hint, a mechanical implementer task starts on the first
        // rung, and the choice says the ladder made it.
        t.tier = "mechanical".into();
        let choice = ladder_route(everywhere.clone(), &t, &ctx());
        assert_eq!(
            routed(&choice),
            ("gpt-oss-120b", ModelChoiceSource::Ladder { rung: 0 })
        );
        assert!(!choice.forced());
        for (tier, slug, rung) in [
            ("focused", "gpt-oss-120b", 0),
            ("integrative", "glm-4.7", 1),
            ("architectural", "claude-sonnet-4-6", 3),
        ] {
            t.tier = tier.into();
            let choice = ladder_route(everywhere.clone(), &t, &ctx());
            assert_eq!(
                routed(&choice),
                (slug, ModelChoiceSource::Ladder { rung }),
                "{tier}"
            );
        }
        t.tier = "mechanical".into();

        // roko.toml names rungs by `[models.*]` key; dispatch gets the slug.
        let mut by_key = ladder_config();
        let keys = [
            "cerebras-gptoss",
            "glm-4-7",
            "gpt-5-4-mini",
            "claude-sonnet",
        ];
        for (rung, key) in by_key.routing.ladder.rungs.iter_mut().zip(keys) {
            rung.model = key.to_string();
        }
        let choice = ladder_route(ladder(&by_key, |_| true), &t, &ctx());
        assert_eq!(choice.model.slug, "gpt-oss-120b");

        // A role override moves that role's start rung only.
        let mut config = ladder_config();
        config.routing.ladder.roles.push(LadderRoleConfig {
            role: "reviewer".to_string(),
            start: LadderStart {
                mechanical: Some("strong".to_string()),
                ..LadderStart::default()
            },
            ..LadderRoleConfig::default()
        });
        let with_reviewer = ladder(&config, |_| true);
        let mut reviewer = ctx();
        reviewer.role = "reviewer".into();
        let choice = ladder_route(with_reviewer.clone(), &t, &reviewer);
        assert_eq!(
            routed(&choice),
            ("gpt-5.4-mini", ModelChoiceSource::Ladder { rung: 2 })
        );
        let choice = ladder_route(with_reviewer, &t, &ctx());
        assert_eq!(choice.model.slug, "gpt-oss-120b");

        // A rung whose provider has no credentials is skipped: the task
        // starts one rung up. So is a disabled provider or a tool-less model.
        let no_cerebras = ladder(&ladder_config(), |key| key != "cerebras-gptoss");
        let choice = ladder_route(no_cerebras, &t, &ctx());
        assert_eq!(
            routed(&choice),
            ("glm-4.7", ModelChoiceSource::Ladder { rung: 1 })
        );
        let mut config = ladder_config();
        config.routing.disabled_providers = vec!["cerebras".to_string()];
        config
            .models
            .get_mut("glm-4-7")
            .expect("glm")
            .supports_tools = false;
        let choice = ladder_route(ladder(&config, |_| true), &t, &ctx());
        assert_eq!(choice.model.slug, "gpt-5.4-mini");

        // A slug that another entry shares would not select the rung's
        // entry, so dispatch gets its key.
        let mut config = ladder_config();
        config.models.insert(
            "groq-gptoss".to_string(),
            ModelProfile {
                provider: "groq".to_string(),
                slug: "gpt-oss-120b".to_string(),
                supports_tools: true,
                ..ModelProfile::default()
            },
        );
        config.routing.ladder.rungs[0].model = "cerebras-gptoss".to_string();
        let choice = ladder_route(ladder(&config, |_| true), &t, &ctx());
        assert_eq!(choice.model.slug, "cerebras-gptoss");

        // A workspace with only Claude routes every tier to Sonnet, as the
        // router's default did.
        let claude_only = ladder(&ladder_config(), |key| key == "claude-sonnet");
        for tier in TaskTier::ALL {
            t.tier = tier.label().into();
            let choice = ladder_route(claude_only.clone(), &t, &ctx());
            assert_eq!(choice.model.slug, "claude-sonnet-4-6", "{tier}");
        }
        t.tier = "mechanical".into();

        // No rung that can run, no ladder model configured, or the ladder
        // turned off: no ladder, and the router decides as before.
        assert!(RoutingLadder::from_config_with(&ladder_config(), |_| false).is_none());
        assert!(RoutingLadder::from_config_with(&RokoConfig::default(), |_| true).is_none());
        let mut off = ladder_config();
        off.routing.ladder.enabled = false;
        assert!(RoutingLadder::from_config_with(&off, |_| true).is_none());

        // A task hint and `--model` still win over the ladder.
        t.model_hint = Some("claude-haiku-4-5".into());
        let choice = ladder_route(everywhere.clone(), &t, &ctx());
        assert_eq!(
            routed(&choice),
            ("claude-haiku-4-5", ModelChoiceSource::TaskHint)
        );
        t.model_hint = None;
        let mut forced = ctx();
        forced.force_backend = Some("gpt-5".into());
        let choice = ladder_route(everywhere, &t, &forced);
        assert_eq!(routed(&choice), ("gpt-5", ModelChoiceSource::Override));
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
