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
//! 2. **Task hint**. `task_def.model_hint`, else the task's `preferred_model`
//!    (if any). Hints are author intent — not learned policy — and always
//!    beat the router.
//! 3. **Ladder**. With a [`RoutingLadder`] attached (`[routing.ladder]`, on
//!    by default), the task's role and tier pick its start rung, unless its
//!    `rung` hint names one. The cascade router's pick is only logged beside
//!    it (shadow).
//! 4. **CascadeRouter**. Only consulted when no override, hint or ladder
//!    rung applies. Returns a [`CascadeModel`] whose `primary` slug is used.
//!    The guards mask the models that cannot run here before its argmax
//!    (S02.P1-2); when none can, a guard replaces the pick with the default
//!    and labels the choice [`ModelChoiceSource::Fallback`]. An attempt's
//!    route explores (`[routing] explore_epsilon`, S02.P1-3): with
//!    probability ε it runs a model drawn among the eligible ones, labelled
//!    [`ModelChoiceSource::Explore`].
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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use indexmap::IndexMap;
use roko_core::agent::ModelSpec;
use roko_core::config::routing::LadderConfig;
use roko_core::config::schema::{ModelProfile, RokoConfig};
use roko_core::task::{TaskCategory, TaskTier};
use roko_learn::cascade_router::{CascadeModel, CascadeRouter, ExploredRoute, explore_route};
use roko_learn::latency::LatencyRegistry;
use roko_learn::loop_audit::arm_set::ArmSet;
use roko_learn::loop_audit::assign::{NestedPick, RouteDecision, RouteDraw, route_propensity};
use roko_learn::loop_audit::faults::{self, FaultKind};
use roko_learn::model_router::RoutingContext;
use roko_learn::provider_health::ProviderHealthRegistry;
use roko_learn::routing_log::{
    CandidateEntry, DecisionState, ROUTE_DECISION_POINT, RouteInfluence, RouteProposals,
    RoutingDecisionLog,
};
use roko_learn::telemetry::records::{AuditFields, DecisionAssignment, DecisionOpportunity};
use roko_learn::telemetry::{AssignmentUnit, AttemptKey, DecisionSource, LayerSpec, assign};

use super::DispatchContext;
use super::outcome::RunnerDispatchError;
use crate::task_parser::TaskDef;

/// `run_seed` of the exploration draws (`telemetry::assign`): 0 until runs
/// record an experiment seed (S01 `experiment.seed`). The attempt key the
/// draws use names the run.
const EXPLORE_SEED: u64 = 0;

/// The route decision's layer (S03 §4.3).
const ROUTE_LAYER: &str = "route";

/// The loop whose layer the route decision draws on.
const ROUTE_LOOP: &str = "L-route";

/// L-route's opportunity reason when nothing pins the route (S01 §5.3).
const ROUTE_OPPORTUNITY: &str = "no_override_no_hint_ge2_eligible";

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
    /// Author-provided model hint (`task.model_hint`, else the task's
    /// `preferred_model`).
    pub task_model_hint: Option<String>,
    /// Author-provided `[routing.ladder]` rung the task starts on
    /// (`rung = "strong"`).
    pub task_rung: Option<String>,
    /// Operator override from the unified CLI `--model` flag.
    /// Highest priority: when set, the router returns this slug immediately.
    pub force_backend: Option<String>,
    /// Remaining USD budget for the plan.
    pub budget_remaining_usd: f64,
    /// Attempt number (0 = first try).
    pub attempt: u32,
    /// Rungs above its start rung the task climbs after agent-blamed
    /// failures (gap-460230); `0` routes on the start rung.
    pub ladder_step: u32,
    /// Role label.
    pub role: String,
    /// Full routing context for the CascadeRouter. When `Some`, the router
    /// calls `CascadeRouter::route()` instead of falling back to the default.
    pub routing_context: Option<RoutingContext>,
    /// The attempt the route is for: the unit of its exploration draw
    /// (S02.P1-3). Without one the route never explores.
    pub attempt_key: Option<AttemptKey>,
    /// The arms of the attempt's chain (S02.P1-14). A draw on the route
    /// layer that holds the chain out runs π⁰.
    pub arm_set: Option<Arc<ArmSet>>,
    /// The start rung an active self-model chose, an index on the role's
    /// ladder (S04, 6130); [`ModelRouter::decide`] draws it through S03's
    /// route table.
    pub self_model_rung: Option<usize>,
}

impl RoutingInputs {
    /// Extract router inputs from a task + per-call context.
    ///
    /// A task's `model_hint` beats its `preferred_model`. Its
    /// `speed_priority` routes nothing: no routing bias is left (decision
    /// 3108), and `plan validate` says so (PLAN_047).
    #[must_use]
    pub fn from_task(task: &TaskDef, ctx: &DispatchContext) -> Self {
        let hints = &task.hints;
        Self {
            task_domain: task.domain.as_ref().map(|d| d.label().to_string()),
            task_tier: task.tier_class(),
            task_model_hint: task
                .model_hint
                .clone()
                .or_else(|| hints.preferred_model.clone())
                .or_else(|| ctx.model_hint.clone()),
            task_rung: hints.rung.clone(),
            force_backend: ctx.force_backend.clone(),
            budget_remaining_usd: ctx.budget_remaining_usd,
            attempt: ctx.attempt,
            ladder_step: ctx.ladder_step,
            role: ctx.role.clone(),
            routing_context: ctx.routing_context.clone(),
            attempt_key: ctx.attempt_key.clone(),
            arm_set: ctx.arm_set.clone(),
            self_model_rung: ctx.self_model_rung,
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
    /// Its outcome teaches the cascade router nothing (decision 4111): the
    /// router learns only from its own picks.
    Ladder {
        /// Index of the rung among the task's rungs, cheapest first.
        rung: usize,
    },
    /// Returned by [`CascadeRouter`].
    Router,
    /// A guard replaced the cascade router's pick with the default: the
    /// pick had no configured provider, a disabled one, or not the tool use
    /// the task needs (G55).
    Fallback {
        /// Why the guard rejected the pick.
        reason: FallbackReason,
    },
    /// Fallback when no other signal was available.
    Default,
    /// The exploration draw of the cascade router's ε-greedy route (S02.P1-3)
    /// replaced its argmax with a model drawn among the eligible ones.
    Explore,
    /// The start rung an active self-model chose on `[routing.ladder]`, drawn
    /// through S03's route table (S04, 6130). Like a ladder rung, it teaches
    /// the cascade router nothing.
    SelfModel {
        /// Index of the rung among the task's rungs, cheapest first.
        rung: usize,
    },
}

impl ModelChoiceSource {
    /// The route decision row's `source` for a choice of this kind (S01
    /// §5.3).
    #[must_use]
    pub const fn decision_source(self) -> DecisionSource {
        match self {
            Self::Override => DecisionSource::Override,
            Self::TaskHint => DecisionSource::TaskHint,
            Self::Ladder { .. } => DecisionSource::Ladder,
            Self::Router => DecisionSource::Router,
            Self::Fallback { .. } => DecisionSource::Fallback,
            Self::Default => DecisionSource::Default,
            Self::Explore => DecisionSource::Explore,
            Self::SelfModel { .. } => DecisionSource::SelfModel,
        }
    }
}

/// Why a guard in [`ModelRouter::route`] replaces the cascade router's pick
/// with the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// No configured, credential-ready provider serves the model.
    ProviderUnconfigured,
    /// The model's provider is in `[routing] disabled_providers`.
    ProviderDisabled,
    /// The task needs tool use, which the model lacks.
    NoToolSupport,
}

impl FallbackReason {
    /// The reason as route decision rows spell it (`fallback_reason`, and a
    /// candidate's `ineligible_reason`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProviderUnconfigured => "provider_unconfigured",
            Self::ProviderDisabled => "provider_disabled",
            Self::NoToolSupport => "no_tool_support",
        }
    }
}

/// A candidate the guards reject because its provider is not available in
/// the health registry. Only candidates carry it: the health-aware pick
/// avoids such providers itself.
const PROVIDER_UNHEALTHY: &str = "provider_unhealthy";

/// A candidate DP4 leaves out because audits keep finding false greens in
/// its passes (S05 §4.6, 7133).
const AUDIT_TRUST: &str = "audit_trust";

/// The cascade router's counts a learned-state digest was computed at: its
/// observations, and the trials and successes of its confidence stats (a
/// hindsight retraction lowers successes without a new observation).
type LearnedStateCounts = (u64, u64, u64);

/// The last learned-state digest, with the counts it was computed at.
type StateDigestMemo = Option<(
    LearnedStateCounts,
    roko_learn::cascade_router::RouterStateDigest,
)>;

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
/// `route` path.  This filters out `Open`-circuit
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
    /// in the current workspace.  When non-empty, the cascade router picks
    /// among these models (S02.P1-2), and its pick falls back to
    /// `default_slug` only when none of its models is here.  Empty means no
    /// filtering (backwards compat).
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
    /// The workspace's durable knowledge store. When present, what it says
    /// about each model weighs into the cascade router's pick (reg-ff6e1a).
    knowledge: Option<roko_neuro::KnowledgeStore>,
    /// The cascade router's last learned-state digest and the counts it was
    /// computed at, so a decision digests the state again only after the
    /// router learned (S01 P0-10).
    state_digest: Arc<parking_lot::Mutex<StateDigestMemo>>,
    /// ε of the exploration draw of a route the cascade router decides
    /// (S02.P1-3, `[routing] explore_epsilon`); 0 takes its argmax.
    explore_epsilon: f64,
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
            .field("knowledge", &self.knowledge.is_some())
            .field("explore_epsilon", &self.explore_epsilon)
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
            knowledge: None,
            state_digest: Arc::default(),
            explore_epsilon: 0.0,
        }
    }

    /// Explore with probability `epsilon` among the models the guards
    /// accept, on each route the cascade router decides that has an attempt
    /// key to draw for (S02.P1-3, decision 2203). The caller caps it
    /// (`RoutingConfig::effective_explore_epsilon`).
    #[must_use]
    pub fn with_explore_epsilon(mut self, epsilon: f64) -> Self {
        self.explore_epsilon = epsilon;
        self
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

    /// Route by `ladder` from now on, or by the router when it is `None`.
    pub fn replace_routing_ladder(&mut self, ladder: Option<RoutingLadder>) {
        self.ladder = ladder;
    }

    /// Clone the inner cascade router `Arc` (for factory cache swap).
    #[must_use]
    pub fn cascade_arc(&self) -> Option<Arc<CascadeRouter>> {
        self.cascade.clone()
    }

    /// Weigh what the durable knowledge `store` says about each model into
    /// the cascade router's pick (reg-ff6e1a): a model the knowledge vouches
    /// for can replace one it warns about. Overrides, hints and the ladder
    /// are unaffected.
    #[must_use]
    pub fn with_knowledge_store(mut self, store: roko_neuro::KnowledgeStore) -> Self {
        self.knowledge = Some(store);
        self
    }

    /// The knowledge store the cascade pick weighs, if any.
    #[must_use]
    pub fn knowledge_store(&self) -> Option<&roko_neuro::KnowledgeStore> {
        self.knowledge.as_ref()
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

    /// Route by `health` from now on, keeping the model-to-provider map
    /// [`Self::with_provider_health`] gave: a caller that loads a persisted
    /// registry after the router was built swaps it in (bug-cf1cf7).
    pub fn replace_provider_health(&mut self, health: Arc<ProviderHealthRegistry>) {
        self.health = Some(health);
    }

    /// The provider health registry routing reads, if any.
    #[must_use]
    pub fn provider_health(&self) -> Option<&Arc<ProviderHealthRegistry>> {
        self.health.as_ref()
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
    /// When non-empty, the cascade router picks among `models` (S02.P1-2);
    /// when it knows none of them, its pick is replaced with the
    /// `default_slug` fallback.  When empty (the default), no filtering
    /// occurs — preserving backwards compatibility.
    #[must_use]
    pub fn with_configured_models(mut self, models: HashSet<String>) -> Self {
        self.configured_models = models;
        self
    }

    /// Exclude models whose provider ID appears in `providers`.
    ///
    /// Populated from `[routing] disabled_providers` in `roko.toml`.
    /// Models backed by a disabled provider are masked like unconfigured
    /// models, before the cascade router selects (S02.P1-2).
    #[must_use]
    pub fn with_disabled_providers(mut self, providers: HashSet<String>) -> Self {
        self.disabled_providers = providers;
        self
    }

    /// Register model slugs that lack tool-use support.
    ///
    /// When a task requires tool use (implementation, scaffolding, integration,
    /// verification, refactoring, infrastructure), the cascade router picks
    /// among the models outside this set (S02.P1-2).  Research and
    /// documentation tasks are unaffected.
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
    /// When a [`ProviderHealthRegistry`] is attached via
    /// [`Self::with_provider_health`], the cascade stage calls
    /// [`CascadeRouter::route_with_health_scored`] which filters `Open`-circuit
    /// providers and demotes `HalfOpen` / high-latency ones, ensuring the
    /// selected model has a healthy provider.
    ///
    /// [`Self::decide`] returns the same choice with its decision row.
    pub fn route(&self, inputs: &RoutingInputs) -> Result<ModelChoice, RunnerDispatchError> {
        self.decide(inputs).map(|(choice, _)| choice)
    }

    /// Route with structured logging — emits `tracing::info!` for every
    /// decision and `debug!` cascade candidate scores when available.
    pub fn route_logged(
        &self,
        inputs: &RoutingInputs,
        task_id: &str,
    ) -> Result<ModelChoice, RunnerDispatchError> {
        self.decide_logged(inputs, task_id)
            .map(|(choice, _)| choice)
    }

    /// [`Self::route`], with the route decision row it made (S01 §5.3, P0-8):
    /// the candidates the cascade router scored, each with its eligibility
    /// under the guards and its probability under the policy, the router's
    /// own pick (beside a ladder rung, its shadow pick), the default, the
    /// ladder rung and the source of the choice. The row has no attempt key:
    /// Graph dispatch keys it when it writes it.
    pub fn decide(
        &self,
        inputs: &RoutingInputs,
    ) -> Result<(ModelChoice, RoutingDecisionLog), RunnerDispatchError> {
        let assigned_at = chrono::Utc::now().timestamp_millis();
        // M3 (6130): an active self-model's start rung, through the route table.
        if let Some(route) = self.self_model_route(inputs) {
            return Ok(self.decide_self_model(inputs, &route, assigned_at));
        }
        let (choice, learned) = self.choose(inputs);
        let (choice, explored) = self.explore(inputs, choice);
        let mut decision = self.decision_row(inputs, &choice, learned, explored.as_ref());
        // A decision made in its draw's millisecond still follows the draw.
        let decided_at = chrono::Utc::now().timestamp_millis().max(assigned_at + 1);
        decision.audit = route_audit_fields(inputs, &decision, assigned_at, decided_at);
        self.compose_propensities(&mut decision, explored.as_ref());
        Ok((choice, decision))
    }

    /// Recompute `row`'s probabilities over the route layer's all-off and
    /// holdout draws (S03 §4.3, R.S03-9's table) when the chain's draw holds
    /// anything out and the route was L-route's opportunity. π⁰'s model
    /// joins the candidates, so the probabilities still sum to 1.
    fn compose_propensities(&self, row: &mut RoutingDecisionLog, explored: Option<&ExploredRoute>) {
        let Some(assignment) = &row.audit.assignment else {
            return;
        };
        let (g, h) = (assignment.draw.g, assignment.draw.h);
        let eligible_route = row
            .audit
            .opportunity
            .as_ref()
            .is_some_and(|opportunity| opportunity.eligible);
        let Some(learned) = row.proposals.learned.clone() else {
            return;
        };
        if (g <= 0.0 && h <= 0.0) || !eligible_route {
            return;
        }
        let eligible: Vec<String> = explored
            .iter()
            .flat_map(|route| &route.propensities)
            .map(|(model, _)| model.clone())
            .collect();
        let eps = if eligible.is_empty() {
            0.0
        } else {
            self.explore_epsilon
        };
        let route = RouteDecision::Drawn(RouteDraw {
            default: self.default_slug.clone(),
            eligible,
            nested: vec![NestedPick {
                pick: learned,
                probability: 1.0,
            }],
        });
        let listed = row
            .candidates
            .iter()
            .any(|candidate| candidate.model == self.default_slug);
        if !listed {
            let provider = self.provider_of(&self.default_slug);
            let default = CandidateEntry::new(self.default_slug.clone(), provider, 0.0, None);
            row.candidates.push(default);
        }
        for candidate in &mut row.candidates {
            candidate.p = Some(route_propensity(g, h, eps, &route, &candidate.model));
        }
        row.propensity = Some(route_propensity(g, h, eps, &route, &row.selected_model));
    }

    /// [`Self::decide`] with [`Self::route_logged`]'s logging.
    pub fn decide_logged(
        &self,
        inputs: &RoutingInputs,
        task_id: &str,
    ) -> Result<(ModelChoice, RoutingDecisionLog), RunnerDispatchError> {
        let (choice, decision) = self.decide(inputs)?;
        tracing::info!(
            task_id,
            model = %choice.model.slug,
            source = ?choice.source,
            "model routed"
        );
        if matches!(
            choice.source,
            ModelChoiceSource::Router | ModelChoiceSource::Fallback { .. }
        ) {
            tracing::debug!(
                task_id,
                stage = %decision.routing_stage,
                "routing candidates: {:?}",
                decision
                    .candidates
                    .iter()
                    .take(3)
                    .map(|c| (c.model.as_str(), c.score))
                    .collect::<Vec<_>>()
            );
        }
        Ok((choice, decision))
    }

    /// The precedence pipeline of [`Self::route`]: the choice, and the
    /// cascade router's own pick before the guards when it made one (beside
    /// a ladder rung, its shadow pick).
    fn choose(&self, inputs: &RoutingInputs) -> (ModelChoice, Option<String>) {
        if let Some(slug) = inputs.force_backend.as_ref() {
            let choice = ModelChoice {
                model: ModelSpec::from_slug(slug),
                source: ModelChoiceSource::Override,
            };
            return (choice, None);
        }
        if let Some(slug) = inputs.task_model_hint.as_ref() {
            let choice = ModelChoice {
                model: ModelSpec::from_slug(slug),
                source: ModelChoiceSource::TaskHint,
            };
            return (choice, None);
        }
        if let Some(routed) = self.ladder_choice(inputs) {
            return routed;
        }
        // No router, or no RoutingContext (CI, smoke tests): the default.
        let Some((router, ctx)) = self.cascade.as_ref().zip(inputs.routing_context.as_ref()) else {
            let choice = ModelChoice {
                model: ModelSpec::from_slug(&self.default_slug),
                source: ModelChoiceSource::Default,
            };
            return (choice, None);
        };
        let pick = self.cascade_pick(router, ctx).primary;
        let learned = Some(pick.slug.clone());
        // A chain held out on the route layer runs π⁰ (S03 §4.3), and the
        // cascade's pick stays its proposal.
        let held_out = inputs
            .arm_set
            .as_deref()
            .is_some_and(|arms| arms.takes_default(ROUTE_LAYER));
        if held_out {
            let choice = ModelChoice {
                model: ModelSpec::from_slug(&self.default_slug),
                source: ModelChoiceSource::Default,
            };
            return (choice, learned);
        }
        // Guards: the pick must have a configured, credential-ready provider
        // (learned state may name `claude-opus` when no Anthropic key is
        // present), its provider must not be in `[routing]
        // disabled_providers`, and it must support tool use when the task
        // category requires tools. The cascade picked among the models they
        // accept, so they reject its pick only when they reject every model.
        if let Some(reason) = self.guard_rejection(&pick.slug, needs_tool_use(ctx.task_category)) {
            tracing::warn!(
                selected = %pick.slug,
                fallback = %self.default_slug,
                reason = reason.as_str(),
                task_category = ?ctx.task_category,
                "cascade router selected a model that cannot run this task; \
                 falling back to default"
            );
            let choice = ModelChoice {
                model: ModelSpec::from_slug(&self.default_slug),
                source: ModelChoiceSource::Fallback { reason },
            };
            return (choice, learned);
        }
        let choice = ModelChoice {
            model: self.faulted_pick(router, ctx, pick),
            source: ModelChoiceSource::Router,
        };
        (choice, learned)
    }

    /// The cascade's `pick` under a fault flag on L-route (S03 §4.9;
    /// fault-injection builds only): MASK runs the default under the
    /// router's label, HARMFUL the cheapest eligible model.
    fn faulted_pick(
        &self,
        router: &CascadeRouter,
        ctx: &RoutingContext,
        pick: ModelSpec,
    ) -> ModelSpec {
        match faults::active(ROUTE_LOOP) {
            Some(FaultKind::Mask) => ModelSpec::from_slug(&self.default_slug),
            Some(FaultKind::Harmful) => {
                let eligible = self.eligible_models(router, ctx);
                router.cheapest_model_among(&eligible)
            }
            _ => pick,
        }
    }

    /// `choice` made ε-greedy (S02.P1-3): when the cascade router decided it,
    /// exploration is on and the route has an attempt key, the attempt's
    /// draw may replace the router's pick with a model drawn among the ones
    /// the guards accept, as an [`ModelChoiceSource::Explore`] choice.
    /// Returns the choice and, for an ε-greedy route, its draw.
    fn explore(
        &self,
        inputs: &RoutingInputs,
        choice: ModelChoice,
    ) -> (ModelChoice, Option<ExploredRoute>) {
        let route = if choice.source == ModelChoiceSource::Router {
            self.explored_route(inputs, &choice.model.slug)
        } else {
            None
        };
        match route {
            Some(route) if route.explored => {
                let explored = ModelChoice {
                    model: ModelSpec::from_slug(&route.chosen),
                    source: ModelChoiceSource::Explore,
                };
                (explored, Some(route))
            }
            route => (choice, route),
        }
    }

    /// The attempt's ε-greedy route around `argmax` among the models the
    /// guards accept; `None` when exploration is off, the route has no
    /// attempt key or routing context, or the guards accept `argmax` alone
    /// or reject it.
    fn explored_route(&self, inputs: &RoutingInputs, argmax: &str) -> Option<ExploredRoute> {
        let key = inputs.attempt_key.as_ref()?;
        let router = self.cascade.as_ref()?;
        let ctx = inputs.routing_context.as_ref()?;
        if self.explore_epsilon <= 0.0 {
            return None;
        }
        let eligible = self.knowledge_candidates(router, ctx);
        if eligible.len() < 2 || !eligible.iter().any(|model| model == argmax) {
            return None;
        }
        let epoch = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let epsilon = self.explore_epsilon;
        let route = explore_route(&eligible, argmax, epsilon, EXPLORE_SEED, &epoch, key);
        Some(route)
    }

    /// `choice` as a route decision row, with `learned` the cascade router's
    /// own pick and `explored` the draw of its ε-greedy route.
    fn decision_row(
        &self,
        inputs: &RoutingInputs,
        choice: &ModelChoice,
        learned: Option<String>,
        explored: Option<&ExploredRoute>,
    ) -> RoutingDecisionLog {
        let ctx = inputs.routing_context.as_ref();
        let chosen = choice.model.slug.as_str();
        let explanation = self
            .cascade
            .as_ref()
            .zip(ctx)
            .map(|(router, ctx)| router.explain_route(ctx, None));
        let needs_tools = ctx.is_some_and(|ctx| needs_tool_use(ctx.task_category));
        let mut candidates: Vec<CandidateEntry> = explanation
            .iter()
            .flat_map(|explanation| &explanation.candidates)
            .map(|candidate| {
                CandidateEntry::new(
                    candidate.slug.clone(),
                    self.provider_of(&candidate.slug),
                    candidate.score,
                    self.ineligible_reason(&candidate.slug, needs_tools)
                        .or_else(|| self.trust_reason(&candidate.slug, ctx, chosen))
                        .map(str::to_string),
                )
            })
            .collect();
        // A pin, a rung or the default the router never scored is still a
        // candidate, so the probabilities sum to 1; its score is 0.
        if !candidates.iter().any(|candidate| candidate.model == chosen) {
            candidates.push(CandidateEntry::new(
                chosen,
                self.provider_of(chosen),
                0.0,
                None,
            ));
        }
        // Every model the router's ε-greedy route may run is a candidate.
        for (model, _) in explored.iter().flat_map(|route| &route.propensities) {
            if !candidates.iter().any(|candidate| candidate.model == *model) {
                candidates.push(CandidateEntry::new(
                    model.clone(),
                    self.provider_of(model),
                    0.0,
                    None,
                ));
            }
        }
        // The router's ε-greedy route gives each model the guards accept its
        // probability (S02.P1-3); any other route chose with certainty.
        for candidate in &mut candidates {
            candidate.p = Some(match explored {
                Some(route) => route.propensity(&candidate.model),
                None if candidate.model == chosen => 1.0,
                None => 0.0,
            });
        }
        let propensity = explored.map_or(1.0, |route| route.propensity(chosen));
        let (ladder, fallback_reason) = match choice.source {
            ModelChoiceSource::Ladder { .. } | ModelChoiceSource::SelfModel { .. } => {
                (Some(chosen.to_string()), None)
            }
            ModelChoiceSource::Fallback { reason } => (None, Some(reason.as_str().to_string())),
            _ => (None, None),
        };
        // Knowledge weighting and provider health move the cascade pick when
        // they are attached.
        let cascade_picked = learned.is_some();
        let influences = vec![
            RouteInfluence {
                kind: "knowledge_weighting".to_string(),
                applied: cascade_picked && self.knowledge.is_some(),
            },
            RouteInfluence {
                kind: "provider_health".to_string(),
                applied: cascade_picked && self.health.is_some(),
            },
        ];
        let source = choice.source.decision_source();
        RoutingDecisionLog {
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: String::new(),
            task_id: String::new(),
            requested_model: inputs.task_model_hint.clone().unwrap_or_default(),
            role: inputs.role.clone(),
            task_complexity: ctx.map_or_else(
                || inputs.task_tier.to_string(),
                |ctx| ctx.complexity.label().to_string(),
            ),
            task_category: ctx
                .map(|ctx| ctx.task_category.label().to_string())
                .unwrap_or_default(),
            selected_provider: self.provider_of(chosen),
            selected_model: chosen.to_string(),
            routing_stage: explanation
                .as_ref()
                .map(|explanation| explanation.stage.to_string())
                .unwrap_or_default(),
            routing_reason: serde_json::to_value(source)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_default(),
            candidates,
            outcome_success: None,
            outcome_cost_usd: None,
            outcome_latency_ms: None,
            attempt_key: None,
            source: Some(source),
            default_model: Some(self.default_slug.clone()),
            propensity: Some(propensity),
            decision_point: ROUTE_DECISION_POINT.to_string(),
            proposals: RouteProposals {
                learned,
                default: Some(self.default_slug.clone()),
                ladder,
                aa: explored.map(|route| route.aa.clone()),
            },
            fallback_reason,
            influences,
            state: self.learned_state(),
            arm_set: None,
            audit: Default::default(),
        }
    }

    /// The cascade router's learned state, for a decision row: its digest,
    /// recomputed only after the router's counts moved, and whether it had
    /// learned anything. Every route reads it, pinned ones included, since
    /// the router's own pick came from it. `None` without a router.
    fn learned_state(&self) -> Option<DecisionState> {
        let router = self.cascade.as_ref()?;
        let confidence = router.confidence_snapshot();
        let trials: u64 = confidence.values().map(|(trials, _)| trials).sum();
        let successes: u64 = confidence.values().map(|(_, successes)| successes).sum();
        let counts = (router.total_observations(), trials, successes);
        let mut memo = self.state_digest.lock();
        let digest = match &*memo {
            Some((at, digest)) if *at == counts => digest.clone(),
            _ => router.snapshot_digest(),
        };
        *memo = Some((counts, digest.clone()));
        Some(DecisionState {
            read: digest.n_obs > 0,
            version: digest.version,
            digest: digest.digest,
            age_s: digest.age_s,
            n_obs: digest.n_obs,
        })
    }

    /// The task's start rung on the attached `[routing.ladder]`, with the
    /// cascade router's own pick beside it (shadow), which is logged. `None`
    /// without a ladder, or when no rung of the task's ladder can run.
    fn ladder_choice(&self, inputs: &RoutingInputs) -> Option<(ModelChoice, Option<String>)> {
        let start = self.ladder.as_ref()?.start(
            &inputs.role,
            inputs.task_tier,
            inputs.task_rung.as_deref(),
        )?;
        let shadow = self
            .cascade
            .as_ref()
            .zip(inputs.routing_context.as_ref())
            .map(|(router, ctx)| self.cascade_pick(router, ctx).primary.slug);
        // A task that failed on its rung climbs from its start (gap-460230).
        let rung = self
            .ladder
            .as_ref()?
            .climb(&inputs.role, start, inputs.ladder_step);
        tracing::info!(
            role = %inputs.role,
            tier = %inputs.task_tier,
            rung = %rung.name,
            step = inputs.ladder_step,
            model = %rung.model,
            router_pick = shadow.as_deref().unwrap_or("none"),
            "model routed by the ladder"
        );
        let choice = ModelChoice {
            model: ModelSpec::from_slug(rung.model),
            source: ModelChoiceSource::Ladder { rung: rung.index },
        };
        Some((choice, shadow))
    }

    /// The cascade router's pick for `ctx`, among the models the provider
    /// guards accept when they accept some ([`Self::eligible_models`]).
    fn cascade_pick(&self, router: &CascadeRouter, ctx: &RoutingContext) -> CascadeModel {
        // The L-route canary's preference, read inside its canary scope only
        // (S03 §4.7).
        if let Some(canary) = router.canary_route() {
            return canary;
        }
        // S02.P1-2: the guards mask the models that cannot run before the
        // cascade's argmax, so it picks the best one that can.
        let eligible = self.eligible_models(router, ctx);

        let route = if let Some(health) = &self.health {
            // Health-aware path: filters Open providers, demotes HalfOpen
            // and optionally high-latency providers.
            let latency_ref = self.latency_registry.as_deref();
            router.route_with_health_scored_among(
                ctx,
                &eligible,
                health,
                &self.model_providers,
                latency_ref,
                self.latency_threshold_ms,
            )
        } else if !eligible.is_empty() {
            router.route_with_cfactor_among(ctx, &eligible, None, None)
        } else {
            router.route(ctx)
        };
        self.weigh_knowledge(router, ctx, route)
    }

    /// `route`, re-ranked with what the knowledge store says about the models
    /// it may land on (reg-ff6e1a); `route` itself without a store.
    fn weigh_knowledge(
        &self,
        router: &CascadeRouter,
        ctx: &RoutingContext,
        route: CascadeModel,
    ) -> CascadeModel {
        let Some(store) = &self.knowledge else {
            return route;
        };
        let candidates = self.knowledge_candidates(router, ctx);
        let advice = crate::knowledge_helpers::build_knowledge_routing_advice(
            store,
            &candidates,
            ctx.role,
            ctx.task_category.label(),
        );
        router.apply_knowledge_among(ctx, route, &candidates, Some(&advice))
    }

    /// The cascade router's models the guards in [`Self::route`] accept, when
    /// they reject some of them and accept at least one (S02.P1-2): the
    /// cascade picks among these. Empty otherwise, and the cascade picks among
    /// every model; when the guards reject them all, [`Self::route`] replaces
    /// the pick with the default.
    fn eligible_models(&self, router: &CascadeRouter, ctx: &RoutingContext) -> Vec<String> {
        let needs_tools = needs_tool_use(ctx.task_category);
        let models = router.model_slugs();
        let eligible: Vec<String> = models
            .iter()
            .filter(|slug| self.guard_rejection(slug, needs_tools).is_none())
            .cloned()
            .collect();
        if eligible.len() == models.len() {
            Vec::new()
        } else {
            eligible
        }
    }

    /// The cascade router's models a knowledge-weighed pick or an
    /// exploration draw may land on: those the guards in [`Self::route`]
    /// accept.
    fn knowledge_candidates(&self, router: &CascadeRouter, ctx: &RoutingContext) -> Vec<String> {
        let needs_tools = needs_tool_use(ctx.task_category);
        router
            .model_slugs()
            .iter()
            .filter(|slug| self.guards_accept(slug, needs_tools))
            .cloned()
            .collect()
    }

    /// Whether the guards in [`Self::route`] accept `slug`: a configured
    /// model on an enabled and available provider, able to use tools when
    /// the task needs them.
    fn guards_accept(&self, slug: &str, needs_tools: bool) -> bool {
        self.ineligible_reason(slug, needs_tools).is_none()
    }

    /// Why the guards reject `slug` as a candidate, or `None` when they
    /// accept it ([`Self::guards_accept`]): a [`FallbackReason`], or a
    /// provider the health registry reports unavailable.
    fn ineligible_reason(&self, slug: &str, needs_tools: bool) -> Option<&'static str> {
        if let Some(reason) = self.guard_rejection(slug, needs_tools) {
            return Some(reason.as_str());
        }
        let provider = self.model_providers.get(slug)?;
        let health = self.health.as_ref()?;
        (!health.is_available(provider)).then_some(PROVIDER_UNHEALTHY)
    }

    /// [`AUDIT_TRUST`] when DP4 (S05 §4.6) leaves `slug` out of `ctx`'s task,
    /// unless it is `chosen`, as one route in twenty lets it be.
    fn trust_reason(
        &self,
        slug: &str,
        ctx: Option<&RoutingContext>,
        chosen: &str,
    ) -> Option<&'static str> {
        let router = self.cascade.as_ref()?;
        let excluded = router.trust_excludes(slug, ctx?.complexity).is_some();
        (excluded && slug != chosen).then_some(AUDIT_TRUST)
    }

    /// Why the guards in [`Self::route`] replace the cascade router's pick
    /// `slug` with the default, or `None` when it may run.
    fn guard_rejection(&self, slug: &str, needs_tools: bool) -> Option<FallbackReason> {
        if !self.configured_models.is_empty() && !self.configured_models.contains(slug) {
            return Some(FallbackReason::ProviderUnconfigured);
        }
        let disabled = self
            .model_providers
            .get(slug)
            .is_some_and(|provider| self.disabled_providers.contains(provider));
        if disabled {
            return Some(FallbackReason::ProviderDisabled);
        }
        (needs_tools && self.models_without_tools.contains(slug))
            .then_some(FallbackReason::NoToolSupport)
    }

    /// The provider `slug` runs on, as far as this router knows (empty
    /// without a provider map).
    fn provider_of(&self, slug: &str) -> String {
        self.model_providers.get(slug).cloned().unwrap_or_default()
    }
}

/// S03's fields of route decision `row` for `inputs` (A-DEC, S01 §5.3;
/// backlog 5124): L-route's layer, whether the route was its opportunity,
/// the chain's draw on the route layer with when it was made, and when the
/// decision was. The chain's arm set holds the draw once the route layer is
/// randomised; until then the draw holds nothing out (h = g = 0). The
/// receipt is the attempt verdict's `executed.model_reported`, which the
/// census joins, so the row carries none. Pure, for E1 (5130).
#[must_use]
pub fn route_audit_fields(
    inputs: &RoutingInputs,
    row: &RoutingDecisionLog,
    assigned_at: i64,
    decided_at: i64,
) -> AuditFields {
    // LABEL_ONLY on L-route (S03 §4.9; fault-injection builds only) draws
    // the arm at the decision, not before it.
    let label_only = faults::active(ROUTE_LOOP) == Some(FaultKind::LabelOnly);
    let assigned_at = if label_only { decided_at } else { assigned_at };
    AuditFields {
        loop_id: Some(ROUTE_LOOP.to_string()),
        loop_ids: vec![ROUTE_LOOP.to_string()],
        layer: Some(ROUTE_LAYER.to_string()),
        opportunity: Some(route_opportunity(inputs, row)),
        assignment: route_assignment(inputs, assigned_at),
        decided_at: Some(decided_at),
        receipt: None,
    }
}

/// Whether the route was L-route's opportunity (S03 §4.2): no pin (an
/// override, a task hint, a ladder rung), a cascade pick, and at least two
/// eligible candidates.
fn route_opportunity(inputs: &RoutingInputs, row: &RoutingDecisionLog) -> DecisionOpportunity {
    let eligible = row
        .candidates
        .iter()
        .filter(|candidate| candidate.eligible)
        .count();
    let reason = if inputs.force_backend.is_some() {
        "pinned_override"
    } else if inputs.task_model_hint.is_some() {
        "pinned_task_hint"
    } else if row.source == Some(DecisionSource::Ladder) {
        "ladder_rung"
    } else if row.proposals.learned.is_none() {
        "no_router"
    } else if eligible < 2 {
        "fewer_than_two_eligible"
    } else {
        ROUTE_OPPORTUNITY
    };
    DecisionOpportunity {
        eligible: reason == ROUTE_OPPORTUNITY,
        reason: reason.to_string(),
    }
}

/// The chain's draw on the route layer at `assigned_at`: its arm set's,
/// else one that holds nothing out. `None` for a route outside an attempt.
fn route_assignment(inputs: &RoutingInputs, assigned_at: i64) -> Option<DecisionAssignment> {
    let key = inputs.attempt_key.as_ref()?;
    let drawn = inputs
        .arm_set
        .as_deref()
        .and_then(|arms| arms.get(ROUTE_LAYER))
        .cloned();
    let draw = drawn.unwrap_or_else(|| {
        let spec = LayerSpec {
            run_seed: EXPLORE_SEED,
            layer: ROUTE_LAYER.to_string(),
            epoch: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            unit: AssignmentUnit::Chain,
            h: 0.0,
            g: 0.0,
        };
        assign(&spec, key)
    });
    Some(DecisionAssignment::new(draw, key, assigned_at))
}

// ─── M3: the self-model's start rung (6130) ────────────────────────────

/// The loop whose epochs let the self-model choose on the route layer (S03
/// §4.3): it shares the layer with L-route, rotating by epoch.
const SELF_MODEL_LOOP: &str = "L-M3";

/// L-M3's opportunity reason: an active self-model proposed the start rung.
const SELF_MODEL_OPPORTUNITY: &str = "self_model_start";

/// An active self-model's route for one attempt (6130).
struct SelfModelRoute {
    /// a⁰, π⁰'s pick: the rung the ladder routes the attempt to by itself.
    default: LadderStartRung,
    /// a^L: the rung the self-model's start rung puts the attempt on.
    pick: LadderStartRung,
    /// The rung the attempt runs on.
    chosen: LadderStartRung,
    /// E: every runnable rung of the task's role, cheapest first.
    eligible: Vec<LadderStartRung>,
    /// The chain's route-layer draw holds it out, so it runs a⁰.
    held_out: bool,
    /// The ε draw among the runnable rungs, when one was made.
    explored: Option<ExploredRoute>,
}

impl ModelRouter {
    /// The route an active self-model's start rung gives `inputs` (6130),
    /// drawn through S03's route table: a chain the route layer holds out
    /// runs the ladder's own rung, and the ε draw may explore another
    /// runnable rung; the ladder climbs from the chosen start as from its
    /// own. `None` without a proposal or a ladder, for a pinned attempt, and
    /// when the proposed rung cannot run here: a guard-rejected rung is never
    /// chosen.
    fn self_model_route(&self, inputs: &RoutingInputs) -> Option<SelfModelRoute> {
        let proposed = inputs.self_model_rung?;
        if inputs.force_backend.is_some() || inputs.task_model_hint.is_some() {
            return None;
        }
        let ladder = self.ladder.as_ref()?;
        let (role, tier) = (inputs.role.as_str(), inputs.task_tier);
        let own = ladder.start(role, tier, inputs.task_rung.as_deref())?;
        let learned = ladder
            .start(role, tier, Some(ladder.rung_name(role, proposed)?))
            .filter(|rung| rung.index == proposed)?;
        let lowest = ladder.start(role, tier, Some(ladder.rung_name(role, 0)?))?;
        let above = ladder.rung_models_above(role, lowest.index);
        let eligible: Vec<LadderStartRung> = std::iter::once(lowest).chain(above).collect();
        let default = ladder.climb(role, own, inputs.ladder_step);
        let pick = ladder.climb(role, learned, inputs.ladder_step);
        let held_out = inputs
            .arm_set
            .as_deref()
            .is_some_and(|arms| arms.takes_default(ROUTE_LAYER));
        let explored = if held_out {
            None
        } else {
            self.explored_rung(inputs, &eligible, &pick.model)
        };
        let drawn = explored
            .as_ref()
            .filter(|route| route.explored)
            .and_then(|route| eligible.iter().find(|rung| rung.model == route.chosen));
        let chosen = if held_out {
            default.clone()
        } else {
            drawn.unwrap_or(&pick).clone()
        };
        Some(SelfModelRoute {
            default,
            pick,
            chosen,
            eligible,
            held_out,
            explored,
        })
    }

    /// The ε draw among the runnable `rungs` around `pick` (S02.P1-3, the
    /// route table's last row); `None` when exploration is off, the attempt
    /// has no key, or one rung can run.
    fn explored_rung(
        &self,
        inputs: &RoutingInputs,
        rungs: &[LadderStartRung],
        pick: &str,
    ) -> Option<ExploredRoute> {
        let key = inputs.attempt_key.as_ref()?;
        if self.explore_epsilon <= 0.0 || rungs.len() < 2 {
            return None;
        }
        let models: Vec<String> = rungs.iter().map(|rung| rung.model.clone()).collect();
        let epoch = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let epsilon = self.explore_epsilon;
        Some(explore_route(
            &models,
            pick,
            epsilon,
            EXPLORE_SEED,
            &epoch,
            key,
        ))
    }

    /// [`Self::decide`] for an active self-model's `route`: the choice, and a
    /// decision row with L-M3's opportunity, a⁰ and a^L as its default and
    /// learned proposals, and every runnable rung's composed propensity
    /// (S03 §4.3). A held-out chain's row has the ladder as its source, an
    /// explored one ε's.
    fn decide_self_model(
        &self,
        inputs: &RoutingInputs,
        route: &SelfModelRoute,
        assigned_at: i64,
    ) -> (ModelChoice, RoutingDecisionLog) {
        let rung = route.chosen.index;
        let source = if route.held_out {
            ModelChoiceSource::Ladder { rung }
        } else {
            ModelChoiceSource::SelfModel { rung }
        };
        let choice = ModelChoice {
            model: ModelSpec::from_slug(route.chosen.model.clone()),
            source,
        };
        let learned = Some(route.pick.model.clone());
        let mut row = self.decision_row(inputs, &choice, learned, None);
        let explored = route.explored.as_ref().filter(|explored| explored.explored);
        let row_source = match (route.held_out, explored) {
            (true, _) => DecisionSource::Ladder,
            (false, Some(_)) => DecisionSource::Explore,
            (false, None) => DecisionSource::SelfModel,
        };
        row.source = Some(row_source);
        row.routing_reason = serde_json::to_value(row_source)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default();
        row.default_model = Some(route.default.model.clone());
        row.proposals.default = Some(route.default.model.clone());
        row.proposals.aa = route.explored.as_ref().map(|explored| explored.aa.clone());
        for rung in &route.eligible {
            let listed = row
                .candidates
                .iter()
                .any(|candidate| candidate.model == rung.model);
            if !listed {
                let provider = self.provider_of(&rung.model);
                let candidate = CandidateEntry::new(rung.model.clone(), provider, 0.0, None);
                row.candidates.push(candidate);
            }
        }
        let decided_at = chrono::Utc::now().timestamp_millis().max(assigned_at + 1);
        row.audit = route_audit_fields(inputs, &row, assigned_at, decided_at);
        row.audit.loop_id = Some(SELF_MODEL_LOOP.to_string());
        row.audit.loop_ids = vec![SELF_MODEL_LOOP.to_string()];
        row.audit.opportunity = Some(DecisionOpportunity {
            eligible: true,
            reason: SELF_MODEL_OPPORTUNITY.to_string(),
        });
        let (g, h) = match &row.audit.assignment {
            Some(assignment) => (assignment.draw.g, assignment.draw.h),
            None => (0.0, 0.0),
        };
        let (eps, eligible) = match &route.explored {
            Some(_) => {
                let models = route.eligible.iter().map(|rung| rung.model.clone());
                (self.explore_epsilon, models.collect())
            }
            None => (0.0, Vec::new()),
        };
        let decision = RouteDecision::Drawn(RouteDraw {
            default: route.default.model.clone(),
            eligible,
            nested: vec![NestedPick {
                pick: route.pick.model.clone(),
                probability: 1.0,
            }],
        });
        for candidate in &mut row.candidates {
            candidate.p = Some(route_propensity(g, h, eps, &decision, &candidate.model));
        }
        let chosen = &route.chosen.model;
        row.propensity = Some(route_propensity(g, h, eps, &decision, chosen));
        (choice, row)
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
    /// of its ladder can run. A `rung_hint` naming one of the task's rungs
    /// replaces the tier's start rung.
    #[must_use]
    pub fn start(
        &self,
        role: &str,
        tier: TaskTier,
        rung_hint: Option<&str>,
    ) -> Option<LadderStartRung> {
        let resolved = self.config.resolve(role, tier, rung_hint, |model| {
            self.runnable.contains_key(model)
        })?;
        let rung = resolved.start_rung();
        Some(LadderStartRung {
            index: resolved.start,
            name: rung.name.clone(),
            model: self.runnable.get(&rung.model)?.clone(),
        })
    }
}

impl RoutingLadder {
    /// The rung `steps` runnable rungs above `start` on `role`'s ladder,
    /// capped at the highest runnable rung; `start` itself when `steps` is
    /// `0` or no rung above it can run (gap-460230).
    #[must_use]
    pub fn climb(&self, role: &str, start: LadderStartRung, steps: u32) -> LadderStartRung {
        let steps = usize::try_from(steps).unwrap_or(usize::MAX);
        let mut above = self.rung_models_above(role, start.index);
        above.truncate(steps);
        above.pop().unwrap_or(start)
    }

    /// The runnable rungs above rung `index` on `role`'s ladder, cheapest
    /// first, each with the model dispatch runs for it. Failover of a task
    /// the ladder routed moves up these and never down (decision 1119,
    /// backlog 1120).
    #[must_use]
    pub fn rung_models_above(&self, role: &str, index: usize) -> Vec<LadderStartRung> {
        let rungs = self.config.role_rungs(role);
        rungs
            .iter()
            .enumerate()
            .skip(index.saturating_add(1))
            .filter_map(|(at, rung)| {
                Some(LadderStartRung {
                    index: at,
                    name: rung.name.clone(),
                    model: self.runnable.get(&rung.model)?.clone(),
                })
            })
            .collect()
    }

    /// The models dispatch runs for the ladder's runnable rungs, each once,
    /// sorted.
    #[must_use]
    pub fn rung_models(&self) -> Vec<String> {
        let models: BTreeSet<&String> = self.runnable.values().collect();
        models.into_iter().cloned().collect()
    }

    /// This ladder without the rungs whose model `failed` names, each logged
    /// with the reason it gives: rungs whose agent-work probe failed (backlog
    /// 1121). `None` when no rung is left, which leaves routing to the
    /// router.
    #[must_use]
    pub fn without_models(mut self, failed: &BTreeMap<String, String>) -> Option<Self> {
        self.runnable.retain(|rung_model, model| {
            let Some(reason) = failed.get(model.as_str()) else {
                return true;
            };
            tracing::warn!(
                model = %rung_model,
                reason = %reason,
                "routing ladder: skipping a rung whose model failed its tool-use probe"
            );
            false
        });
        if self.runnable.is_empty() {
            tracing::warn!("routing ladder: no rung passed its probe; the router picks");
            return None;
        }
        Some(self)
    }

    /// How many runnable rungs sit above rung `index` on `role`'s ladder.
    #[must_use]
    pub fn runnable_above(&self, role: &str, index: usize) -> usize {
        let rungs = self.config.role_rungs(role);
        rungs
            .iter()
            .skip(index.saturating_add(1))
            .filter(|rung| self.runnable.contains_key(&rung.model))
            .count()
    }

    /// Name of rung `index` on `role`'s ladder.
    #[must_use]
    pub fn rung_name(&self, role: &str, index: usize) -> Option<&str> {
        let rungs = self.config.role_rungs(role);
        rungs.get(index).map(|rung| rung.name.as_str())
    }

    /// Whether `name` names a rung on `role`'s ladder, as a task's `rung`
    /// hint must to move its start rung.
    #[must_use]
    pub fn has_rung(&self, role: &str, name: &str) -> bool {
        let rungs = self.config.role_rungs(role);
        rungs.iter().any(|rung| rung.name == name)
    }

    /// Index of the rung named `name` on `role`'s ladder, cheapest first,
    /// runnable or not: where M1's tier floor and cap sit (8124).
    #[must_use]
    pub fn rung_index(&self, role: &str, name: &str) -> Option<usize> {
        let rungs = self.config.role_rungs(role);
        rungs.iter().position(|rung| rung.name == name)
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
            spec: Default::default(),
            hints: Default::default(),
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
            ladder_step: 0,
            prompt_experiment: None,
            gate_feedback: None,
            routing_context: None,
            dependency_outputs: Vec::new(),
            error_patterns_context: String::new(),
            cached_workspace_map: String::new(),
            cached_workspace_context: String::new(),
            concurrent_plans: Vec::new(),
            attempt_key: None,
            arm_set: None,
            self_model_rung: None,
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

    /// reg-ff6e1a: with the workspace's knowledge store attached, what it
    /// says about each model moves the cascade router's pick. An empty store
    /// leaves the pick alone; once the store warns about the pick and vouches
    /// for the other model, the other model wins.
    #[test]
    fn knowledge_routing_moves_the_cascade_pick() {
        use roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeStore};

        let models = vec!["model-alpha".to_string(), "model-beta".to_string()];
        let cascade = Arc::new(CascadeRouter::new(models.clone()));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        let baseline = ModelRouter::new(Some(Arc::clone(&cascade)))
            .route(&inputs)
            .unwrap()
            .model
            .slug;
        let other = models
            .iter()
            .find(|slug| **slug != baseline)
            .expect("a second model")
            .clone();

        let workdir = tempfile::tempdir().expect("tempdir");
        let store = KnowledgeStore::for_workdir(workdir.path());
        let router = ModelRouter::new(Some(cascade)).with_knowledge_store(store.clone());
        assert_eq!(router.route(&inputs).unwrap().model.slug, baseline);

        let warning =
            format!("implementer implementation routing model {baseline} repeatedly fails");
        store
            .add(KnowledgeEntry {
                id: "warns".into(),
                kind: KnowledgeKind::AntiKnowledge,
                content: warning,
                confidence: 0.8,
                source_model: Some(baseline.clone()),
                ..KnowledgeEntry::default()
            })
            .expect("persist the warning");
        let endorsement =
            format!("implementer implementation routing model {other} succeeds reliably");
        store
            .add(KnowledgeEntry {
                id: "vouches".into(),
                kind: KnowledgeKind::Heuristic,
                content: endorsement,
                confidence: 0.9,
                source_model: Some(other.clone()),
                ..KnowledgeEntry::default()
            })
            .expect("persist the endorsement");
        let choice = router.route(&inputs).unwrap();
        assert_eq!(choice.model.slug, other);
        assert_eq!(choice.source, ModelChoiceSource::Router);
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

    /// gap-0f3980: a task's `preferred_model` is its model hint unless it
    /// sets `model_hint`.
    #[test]
    fn routing_inputs_read_the_task_hints() {
        let mut t = task();
        t.hints.preferred_model = Some("claude-opus-4-1".into());
        let inputs = RoutingInputs::from_task(&t, &ctx());
        assert_eq!(inputs.task_model_hint.as_deref(), Some("claude-opus-4-1"));
        let choice = ModelRouter::new(None).route(&inputs).unwrap();
        assert_eq!(
            routed(&choice),
            ("claude-opus-4-1", ModelChoiceSource::TaskHint)
        );
        t.model_hint = Some("claude-haiku-4-5".into());
        assert_eq!(
            RoutingInputs::from_task(&t, &ctx())
                .task_model_hint
                .as_deref(),
            Some("claude-haiku-4-5")
        );
    }

    /// backlog 3109 (decision 3108): with the routing bias gone, a plan task
    /// routes as it did when the health-aware pick dropped the bias: a task
    /// that asks for `speed_priority = "latency"` lands where any other does.
    #[test]
    fn routing_unchanged_without_bias() {
        use roko_core::task::TaskSpeedPriority;

        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
        ]));
        let health = Arc::new(ProviderHealthRegistry::new());
        let router = ModelRouter::new(Some(cascade)).with_provider_health(health, HashMap::new());
        let route = |t: &TaskDef| {
            let mut inputs = RoutingInputs::from_task(t, &ctx());
            inputs.routing_context = Some(routing_context());
            router.route(&inputs).unwrap()
        };
        let mut t = task();
        let plain = route(&t);
        t.hints.speed_priority = Some(TaskSpeedPriority::Latency);
        let latency = route(&t);

        assert_eq!(plain.source, ModelChoiceSource::Router);
        assert!(
            ["claude-sonnet-4-6", "claude-haiku-4-5"].contains(&plain.model.slug.as_str()),
            "{plain:?}"
        );
        assert_eq!(routed(&latency), routed(&plain));
    }

    /// gap-dbf2a6: a task with `rung = "strong"` and no `model_hint` starts
    /// on that rung, moving up past it when it cannot run; `model_hint`
    /// still wins.
    #[test]
    fn rung_hint_starts_the_task_on_that_rung() {
        let everywhere = ladder(&ladder_config(), |_| true);
        let mut t = task();
        t.tier = "mechanical".into();
        t.hints.rung = Some("strong".into());
        let choice = ladder_route(everywhere.clone(), &t, &ctx());
        assert_eq!(
            routed(&choice),
            ("gpt-5.4-mini", ModelChoiceSource::Ladder { rung: 2 })
        );
        let no_mini = ladder(&ladder_config(), |key| key != "gpt-5-4-mini");
        let choice = ladder_route(no_mini, &t, &ctx());
        assert_eq!(
            routed(&choice),
            ("claude-sonnet-4-6", ModelChoiceSource::Ladder { rung: 3 })
        );
        t.model_hint = Some("claude-haiku-4-5".into());
        let choice = ladder_route(everywhere, &t, &ctx());
        assert_eq!(
            routed(&choice),
            ("claude-haiku-4-5", ModelChoiceSource::TaskHint)
        );
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

    /// gap-460230: each ladder step climbs one runnable rung above the
    /// task's start rung, skipping rungs that cannot run, and stops at the
    /// top. A pinned model never climbs.
    #[test]
    fn ladder_steps_climb_runnable_rungs_to_the_top() {
        let no_glm = ladder(&ladder_config(), |key| key != "glm-4-7");
        let mut t = task();
        t.tier = "mechanical".into();
        for (step, slug, rung) in [
            (0, "gpt-oss-120b", 0),
            (1, "gpt-5.4-mini", 2),
            (2, "claude-sonnet-4-6", 3),
            (3, "claude-sonnet-4-6", 3),
        ] {
            let mut climbing = ctx();
            climbing.ladder_step = step;
            let choice = ladder_route(no_glm.clone(), &t, &climbing);
            assert_eq!(
                routed(&choice),
                (slug, ModelChoiceSource::Ladder { rung }),
                "step {step}"
            );
        }
        assert_eq!(no_glm.runnable_above("implementer", 0), 2);
        assert_eq!(no_glm.runnable_above("implementer", 3), 0);
        assert_eq!(no_glm.rung_name("implementer", 2), Some("strong"));

        t.model_hint = Some("claude-haiku-4-5".into());
        let mut climbing = ctx();
        climbing.ladder_step = 2;
        let choice = ladder_route(no_glm, &t, &climbing);
        assert_eq!(
            routed(&choice),
            ("claude-haiku-4-5", ModelChoiceSource::TaskHint)
        );
    }

    /// backlog 1120: the rungs failover may move a task the ladder routed to
    /// are the runnable ones above its rung, cheapest first.
    #[test]
    fn rung_models_above_lists_runnable_rungs_in_order() {
        let no_glm = ladder(&ladder_config(), |key| key != "glm-4-7");
        let above: Vec<(usize, String, String)> = no_glm
            .rung_models_above("implementer", 0)
            .into_iter()
            .map(|rung| (rung.index, rung.name, rung.model))
            .collect();
        assert_eq!(
            above,
            [
                (2, "strong".to_string(), "gpt-5.4-mini".to_string()),
                (3, "top".to_string(), "claude-sonnet-4-6".to_string()),
            ]
        );
        assert!(no_glm.rung_models_above("implementer", 3).is_empty());
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

    /// bug-cf1cf7: the router a factory builds reads the registry that
    /// `with_health_registry` supplies afterwards, as a plan run's persisted
    /// one, so a provider whose circuit that registry holds open gets no task
    /// from the new run's first route.
    #[tokio::test]
    async fn with_health_registry_rewires_the_router() {
        use roko_core::agent::ProviderKind;
        use roko_core::config::schema::ProviderConfig;
        use roko_learn::provider_health::ErrorClass;

        use crate::dispatch::SharedAgentFactory;

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        // The cascade router picks, not the ladder.
        config.routing.ladder.enabled = false;
        for (provider, kind, model) in [
            ("anthropic", ProviderKind::AnthropicApi, "claude-sonnet-4-6"),
            ("google", ProviderKind::GeminiApi, "gemini-2.5-flash"),
        ] {
            // `PATH` is always set, standing in for a key.
            let provider_config = ProviderConfig {
                kind,
                api_key_env: Some("PATH".to_string()),
                ..ProviderConfig::default()
            };
            config
                .providers
                .insert(provider.to_string(), provider_config);
            let profile = ModelProfile {
                provider: provider.to_string(),
                slug: model.to_string(),
                supports_tools: true,
                ..ModelProfile::default()
            };
            config.models.insert(model.to_string(), profile);
        }
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gemini-2.5-flash".into(),
        ]));
        let health = Arc::new(ProviderHealthRegistry::new());
        for _ in 0..3 {
            health.record_failure("anthropic", ErrorClass::RateLimit);
        }

        let factory = SharedAgentFactory::new(Arc::new(config), None, Some(cascade), None)
            .await
            .with_health_registry(Arc::clone(&health));
        let routed = factory
            .dispatcher()
            .provider_health()
            .expect("the router reads provider health");
        assert!(Arc::ptr_eq(routed, &health));
        let mut planned = ctx();
        planned.routing_context = Some(routing_context());
        for _ in 0..4 {
            let plan = factory
                .dispatcher()
                .plan(&task(), &planned)
                .expect("the factory plans the task");
            assert_eq!(plan.model.slug, "gemini-2.5-flash");
        }
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
        let reason = FallbackReason::ProviderUnconfigured;
        assert_eq!(
            choice.source,
            ModelChoiceSource::Fallback { reason },
            "a guard's replacement of the router's pick is a fallback, not the router's (G55)"
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

    // ── Route decision rows (S01 P0-8) ─────────────────────────────────

    /// The choice and decision row `router` makes for `inputs`, checked for
    /// what every route row carries: the source of the choice, the default,
    /// and candidate probabilities that sum to 1 with the chosen model at 1.
    fn decided(router: &ModelRouter, inputs: &RoutingInputs) -> (ModelChoice, RoutingDecisionLog) {
        let (choice, row) = router.decide_logged(inputs, "t").unwrap();
        let routed = router.route_logged(inputs, "t").unwrap();
        assert_eq!(routed.model.slug, choice.model.slug);
        assert_eq!(row.decision_point, ROUTE_DECISION_POINT);
        assert_eq!(row.source, Some(choice.source.decision_source()));
        assert_eq!(row.selected_model, choice.model.slug);
        assert_eq!(row.propensity, Some(1.0));
        assert_eq!(row.proposals.default.as_deref(), Some("default-model"));
        assert_eq!(row.default_model.as_deref(), Some("default-model"));
        let total: f64 = row.candidates.iter().filter_map(|c| c.p).sum();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "{:?}: p sums to {total}",
            choice.source
        );
        let chosen: Vec<&str> = row
            .candidates
            .iter()
            .filter(|c| c.p == Some(1.0))
            .map(|c| c.model.as_str())
            .collect();
        assert_eq!(chosen, [choice.model.slug.as_str()]);
        (choice, row)
    }

    /// S01 P0-8: router, ladder, hint, override and default routes each
    /// return a decision row with candidates, their propensities, the
    /// default and the cascade router's own pick wherever it made one.
    #[test]
    fn route_logged_writes_candidates_propensity_and_default() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]));
        let cascade_pick = cascade.route(&routing_context()).primary.slug;
        let router = ModelRouter::new(Some(cascade)).with_default_slug("default-model");
        let mut routed = RoutingInputs::from_task(&task(), &ctx());
        routed.routing_context = Some(routing_context());

        // The cascade router's own pick runs.
        let (choice, row) = decided(&router, &routed);
        assert_eq!(choice.source, ModelChoiceSource::Router);
        assert_eq!(
            row.proposals.learned.as_deref(),
            Some(cascade_pick.as_str())
        );
        assert_eq!(row.candidates.len(), 2, "{:?}", row.candidates);
        assert!(row.candidates.iter().all(|c| c.eligible));

        // Beside a ladder rung, the cascade router's pick is the shadow
        // proposal, and the rung joins the candidates.
        let everywhere = ladder(&ladder_config(), |_| true);
        let laddered = router.clone().with_routing_ladder(everywhere);
        let mut t = task();
        t.tier = "mechanical".into();
        let mut on_ladder = RoutingInputs::from_task(&t, &ctx());
        on_ladder.routing_context = Some(routing_context());
        let (choice, row) = decided(&laddered, &on_ladder);
        assert_eq!(choice.source, ModelChoiceSource::Ladder { rung: 0 });
        assert_eq!(row.proposals.ladder.as_deref(), Some("gpt-oss-120b"));
        assert_eq!(
            row.proposals.learned.as_deref(),
            Some(cascade_pick.as_str())
        );
        assert_eq!(row.candidates.len(), 3, "{:?}", row.candidates);

        // A task hint or an override pins the model: no cascade pick.
        let mut hinted = routed.clone();
        hinted.task_model_hint = Some("claude-haiku-4-5".into());
        let (choice, row) = decided(&router, &hinted);
        assert_eq!(choice.source, ModelChoiceSource::TaskHint);
        assert_eq!(row.proposals.learned, None);
        assert_eq!(row.requested_model, "claude-haiku-4-5");
        let mut forced = routed;
        forced.force_backend = Some("gpt-5".into());
        let (choice, row) = decided(&router, &forced);
        assert_eq!(choice.source, ModelChoiceSource::Override);
        assert_eq!(row.proposals.learned, None);
        assert_eq!(row.candidates.len(), 2, "gpt-5 is a router model");

        // Without a routing context the default runs, alone.
        let unrouted = RoutingInputs::from_task(&task(), &ctx());
        let (choice, row) = decided(&router, &unrouted);
        assert_eq!(choice.source, ModelChoiceSource::Default);
        assert_eq!(row.proposals.learned, None);
        assert_eq!(row.proposals.ladder, None);
        assert_eq!(row.candidates.len(), 1);
    }

    /// G55: each guard that replaces the cascade router's pick with the
    /// default labels the choice a fallback with its reason, and so does
    /// the decision row; the pick stays the row's learned proposal. A guard
    /// replaces the pick only when it rejects every model: otherwise the
    /// cascade picks among the others (S02.P1-2).
    #[test]
    fn unconfigured_cascade_pick_is_not_labelled_router() {
        let models = vec!["claude-sonnet-4-6".to_string(), "gpt-5".to_string()];
        let cascade = Arc::new(CascadeRouter::new(models.clone()));
        let pick = cascade.route(&routing_context()).primary.slug;
        let base = ModelRouter::new(Some(cascade)).with_default_slug("model-a");
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());

        let only_a = HashSet::from(["model-a".to_string()]);
        let unconfigured = base.clone().with_configured_models(only_a);
        // Every model the router may land on runs on one provider, disabled.
        let providers: HashMap<String, String> = models
            .iter()
            .chain([&pick])
            .map(|slug| (slug.clone(), "provider-x".to_string()))
            .collect();
        let disabled = base
            .clone()
            .with_provider_health(Arc::new(ProviderHealthRegistry::new()), providers)
            .with_disabled_providers(HashSet::from(["provider-x".to_string()]));
        let no_tools: HashSet<String> = models.iter().cloned().collect();
        let toolless = base.clone().with_tool_capability_filter(no_tools);
        for (router, reason) in [
            (unconfigured, FallbackReason::ProviderUnconfigured),
            (disabled, FallbackReason::ProviderDisabled),
            (toolless, FallbackReason::NoToolSupport),
        ] {
            let (choice, row) = router.decide(&inputs).unwrap();
            let fallback = ModelChoiceSource::Fallback { reason };
            assert_eq!(routed(&choice), ("model-a", fallback), "{reason:?}");
            assert!(!choice.forced());
            assert_eq!(row.source, Some(DecisionSource::Fallback));
            assert_eq!(row.fallback_reason.as_deref(), Some(reason.as_str()));
            assert_eq!(row.selected_model, "model-a");
            let learned = row.proposals.learned.as_deref().expect("a pick");
            assert_ne!(learned, "model-a", "{reason:?}");
        }

        // Without a guard, the pick runs as the router's own.
        let (choice, row) = base.decide(&inputs).unwrap();
        assert_eq!(routed(&choice), (pick.as_str(), ModelChoiceSource::Router));
        assert_eq!(row.source, Some(DecisionSource::Router));
        assert_eq!(row.fallback_reason, None);
    }

    /// S02.P1-2: the guards mask a model that cannot run before the cascade
    /// router's argmax. When the router's best arm is unconfigured, on a
    /// disabled provider or without the tool use the task needs, it picks
    /// the best arm that can run, as its own pick, not the default. The
    /// masked arm stays in the decision row as an ineligible candidate.
    #[test]
    fn ineligible_pick_is_masked_before_argmax() {
        let models = vec!["claude-sonnet-4-6".to_string(), "gpt-5".to_string()];
        let cascade = Arc::new(CascadeRouter::new(models.clone()));
        let best = cascade.route(&routing_context()).primary.slug;
        let other = models
            .iter()
            .find(|slug| **slug != best)
            .expect("a second arm")
            .clone();
        let base = ModelRouter::new(Some(cascade)).with_default_slug("default-model");
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());

        let only_other = HashSet::from([other.clone()]);
        let unconfigured = base.clone().with_configured_models(only_other);
        let providers = HashMap::from([
            (best.clone(), "provider-x".to_string()),
            (other.clone(), "provider-y".to_string()),
        ]);
        let disabled = base
            .clone()
            .with_provider_health(Arc::new(ProviderHealthRegistry::new()), providers)
            .with_disabled_providers(HashSet::from(["provider-x".to_string()]));
        let no_tools = HashSet::from([best.clone()]);
        let toolless = base.with_tool_capability_filter(no_tools);
        let runnable = (other.as_str(), ModelChoiceSource::Router);
        for (router, reason) in [
            (unconfigured, FallbackReason::ProviderUnconfigured),
            (disabled, FallbackReason::ProviderDisabled),
            (toolless, FallbackReason::NoToolSupport),
        ] {
            let (choice, row) = decided(&router, &inputs);
            assert_eq!(routed(&choice), runnable, "{reason:?}");
            assert_eq!(row.proposals.learned.as_deref(), Some(other.as_str()));
            assert_eq!(row.fallback_reason, None);
            let masked = row
                .candidates
                .iter()
                .find(|candidate| candidate.model == best)
                .expect("the masked arm is a candidate");
            assert!(!masked.eligible, "{reason:?}");
            assert_eq!(masked.ineligible_reason.as_deref(), Some(reason.as_str()));
        }
    }

    /// S02.P1-3: a route the cascade router decides for an attempt explores
    /// with probability ε among the models the guards accept. Its decision
    /// row gives each its ε-greedy probability and names the A/A draw; a
    /// route without an attempt key takes the argmax.
    #[test]
    fn explored_route_logs_every_eligible_propensity() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]));
        let router = ModelRouter::new(Some(cascade))
            .with_default_slug("default-model")
            .with_explore_epsilon(1.0);
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        let (choice, row) = router.decide(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Router, "no attempt key");
        assert_eq!(row.propensity, Some(1.0));
        assert_eq!(row.proposals.aa, None);

        inputs.attempt_key = Some(AttemptKey::new("run", "p", "t", 1));
        let (choice, row) = router.decide(&inputs).unwrap();
        assert_eq!(choice.source, ModelChoiceSource::Explore);
        assert_eq!(row.source, Some(DecisionSource::Explore));
        assert_eq!(row.propensity, Some(0.5));
        let p: Vec<Option<f64>> = row.candidates.iter().map(|c| c.p).collect();
        assert_eq!(p, [Some(0.5), Some(0.5)]);
        assert!(row.proposals.aa.is_some());
    }

    /// The learned state `router`'s decision for `inputs` names.
    fn decided_state(router: &ModelRouter, inputs: &RoutingInputs) -> DecisionState {
        let (_, row) = router.decide(inputs).unwrap();
        row.state.expect("the router's learned state")
    }

    /// S01 P0-10: every route decision names the cascade router's learned
    /// state, whatever routed it, and the digest moves only when the router
    /// learns.
    #[test]
    fn decision_records_carry_learned_state_digest() {
        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]));
        let router = ModelRouter::new(Some(Arc::clone(&cascade)));
        let everywhere = ladder(&ladder_config(), |_| true);
        let laddered = router.clone().with_routing_ladder(everywhere);
        let mut routed = RoutingInputs::from_task(&task(), &ctx());
        routed.routing_context = Some(routing_context());
        let mut t = task();
        t.tier = "mechanical".into();
        let mut on_ladder = RoutingInputs::from_task(&t, &ctx());
        on_ladder.routing_context = Some(routing_context());
        let mut hinted = routed.clone();
        hinted.task_model_hint = Some("claude-haiku-4-5".into());

        // Two decisions with no learning between them name one state, and
        // a ladder or pinned route names the state the router's pick read.
        let before = decided_state(&router, &routed);
        assert_eq!(decided_state(&router, &routed), before);
        assert!(before.digest.starts_with("b3:"), "{before:?}");
        assert_eq!((before.n_obs, before.read), (0, false));
        assert_eq!(decided_state(&laddered, &on_ladder), before);
        assert_eq!(decided_state(&router, &hinted), before);

        // One observation moves it.
        cascade.record_observation(&routing_context(), "gpt-5", 0.9, true);
        let after = decided_state(&router, &routed);
        assert_ne!(after.digest, before.digest);
        assert_eq!((after.n_obs, after.read), (1, true));
        assert_eq!(after.version, "cr:obs=1");
        assert_eq!(decided_state(&laddered, &on_ladder), after);

        // Without a router there is no learned state to name.
        let (_, row) = ModelRouter::new(None).decide(&routed).unwrap();
        assert_eq!(row.state, None);
    }

    /// S03 §5 A-DEC (backlog 5124): a route decision carries L-route's layer,
    /// its opportunity and the chain's draw, made before the decision, and
    /// both proposals on both arms, with probabilities that sum to 1. A
    /// guard's fallback is labelled `fallback`, and the census reads the
    /// executed model that the attempt's verdict reports as the receipt.
    #[test]
    fn route_decision_logs_arm_before_plan_and_executed_model() {
        use roko_learn::loop_audit::census::measure;
        use roko_learn::telemetry::records::{
            AttemptIdentity, AttemptOutcome, AttemptVerdictRecord, DECISION_SCHEMA, Stamped,
            VERDICT_SCHEMA,
        };
        use roko_learn::telemetry::report::RunRecords;
        use roko_learn::telemetry::{Arm, Assignment};

        let cascade = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]));
        cascade.record_observation(&routing_context(), "gpt-5", 0.9, true);
        let pick = cascade.route(&routing_context()).primary.slug;
        let router = ModelRouter::new(Some(cascade)).with_default_slug("default-model");
        let key = AttemptKey::new("gr-route", "p", "t", 1);
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.routing_context = Some(routing_context());
        inputs.attempt_key = Some(key.clone());

        // The learned arm: the route layer holds nothing out yet.
        let (choice, learned_row) = router.decide(&inputs).unwrap();
        assert_eq!(routed(&choice), (pick.as_str(), ModelChoiceSource::Router));
        let audit = &learned_row.audit;
        assert_eq!(audit.layer.as_deref(), Some("route"));
        assert_eq!(audit.loop_id.as_deref(), Some("L-route"));
        let opportunity = audit.opportunity.as_ref().expect("the opportunity");
        assert!(opportunity.eligible, "{opportunity:?}");
        let assignment = audit.assignment.as_ref().expect("the chain's draw");
        assert_eq!(
            (assignment.draw.arm, assignment.draw.h),
            (Arm::Learned, 0.0)
        );
        assert_eq!(assignment.unit_key, key.chain_key());
        assert!(assignment.assigned_at < audit.decided_at.expect("decided_at"));
        let proposals = &learned_row.proposals;
        assert_eq!(proposals.learned.as_deref(), Some(pick.as_str()));
        assert_eq!(proposals.default.as_deref(), Some("default-model"));

        // The default arm: a chain held out on the route layer runs π⁰, and
        // the cascade's pick stays its proposal.
        let held_out = Assignment {
            unit: AssignmentUnit::Chain,
            layer: "route".to_string(),
            salt_id: "route@2026-10-03".to_string(),
            u: 0.1,
            h: 0.2,
            g: 0.0,
            arm: Arm::Default,
            propensity: 0.2,
        };
        inputs.arm_set = Some(Arc::new(ArmSet {
            chain_key: key.chain_key(),
            arms: [("route".to_string(), held_out)].into(),
            condition_id: "normal".to_string(),
        }));
        let (choice, row) = router.decide(&inputs).unwrap();
        let default = ("default-model", ModelChoiceSource::Default);
        assert_eq!(routed(&choice), default);
        assert_eq!(row.proposals.learned.as_deref(), Some(pick.as_str()));
        assert_eq!(row.proposals.default.as_deref(), Some("default-model"));
        let assignment = row.audit.assignment.as_ref().expect("the held-out draw");
        assert_eq!(assignment.draw.arm, Arm::Default);
        assert_eq!(assignment.audit_epoch, "2026-10-03");
        let total: f64 = row.candidates.iter().filter_map(|c| c.p).sum();
        assert!((total - 1.0).abs() < 1e-9, "p sums to {total}");
        assert_eq!(row.propensity, Some(0.2));
        inputs.arm_set = None;

        // A guard's replacement of the pick is labelled `fallback`, and with
        // one eligible model the route is no opportunity.
        let only_default = HashSet::from(["default-model".to_string()]);
        let guarded = router.clone().with_configured_models(only_default);
        let (_, fallback) = guarded.decide(&inputs).unwrap();
        assert_eq!(fallback.source, Some(DecisionSource::Fallback));
        let reason = fallback.fallback_reason.as_deref();
        assert_eq!(reason, Some("provider_unconfigured"));
        let opportunity = fallback.audit.opportunity.expect("the opportunity");
        assert_eq!(opportunity.reason, "fewer_than_two_eligible");

        // The census reads the executed model the verdict reports as the
        // receipt of the learned route.
        let mut row = learned_row;
        row.attempt_key = Some(key.attempt_key());
        let identity = AttemptIdentity::new(&key);
        let mut verdict = AttemptVerdictRecord::settle(identity, AttemptOutcome::Passed, true);
        verdict.executed.model_reported = Some(pick.clone());
        let run = RunRecords {
            run_id: "gr-route".to_string(),
            decisions: vec![Stamped {
                schema_version: DECISION_SCHEMA.to_string(),
                record_id: "b3:decision".to_string(),
                seq: 1,
                ts: "2026-10-03T09:00:00Z".to_string(),
                record: row,
            }],
            verdicts: vec![Stamped {
                schema_version: VERDICT_SCHEMA.to_string(),
                record_id: "b3:verdict".to_string(),
                seq: 2,
                ts: "2026-10-03T09:00:01Z".to_string(),
                record: verdict,
            }],
            ..RunRecords::default()
        };
        let measured = measure(&[run]);
        let route = &measured["L-route"];
        assert_eq!(route.n_learned, 1);
        assert_eq!((route.eps.receipt, route.eps.honest), (1.0, 1.0));
        assert_eq!(route.eps.est, 1.0);
    }

    /// 6130: an active self-model's start rung routes the task through S03's route table, and
    /// the decision row names L-M3, a⁰ and a^L with every rung's composed propensity; a chain the
    /// route layer holds out runs the ladder's own rung; dispatch proposes nothing when the
    /// breaker trips, under a pin or outside active mode; and a pin beats a proposal.
    #[test]
    fn active_self_model_picks_start_rung_and_logs_propensity() {
        use crate::graph_task_dispatch::self_model::active_start;
        use roko_core::config::self_model::{SelfModelConfig, SelfModelMode};
        use roko_learn::self_model::PredictorVersion;
        use roko_learn::self_model::gate::GateReport;
        use roko_learn::telemetry::{Arm, Assignment};

        let gate = |eligible, breaker_tripped| GateReport {
            predictor_version: PredictorVersion("m3-l1-test".to_string()),
            n: 100,
            ece: Some(0.03),
            cal_in_large: Some(0.01),
            bss: Some(0.2),
            auroc: Some(0.8),
            route_pass: Some(0.82),
            eligible,
            reasons: Vec::new(),
            breaker_tripped,
        };
        let acts = |settings: &SelfModelConfig, eligible, tripped, pinned, choice| {
            active_start(settings, &gate(eligible, tripped), pinned, Some(choice), 1)
        };
        let active = SelfModelConfig {
            mode: SelfModelMode::Active,
            ..SelfModelConfig::default()
        };
        let shadow = SelfModelConfig {
            mode: SelfModelMode::Shadow,
            ..SelfModelConfig::default()
        };
        let upward = SelfModelConfig {
            allow_downward_start: false,
            ..active.clone()
        };
        assert_eq!(acts(&active, true, false, false, 3), Some(3));
        assert_eq!(
            acts(&active, false, true, false, 3),
            None,
            "a tripped breaker"
        );
        assert_eq!(acts(&active, true, false, true, 3), None, "a pin");
        assert_eq!(acts(&shadow, true, false, false, 3), None, "shadow mode");
        assert_eq!(acts(&active, true, false, false, 0), Some(0), "B1");
        assert_eq!(acts(&upward, true, false, false, 0), None, "without B1");

        // Without a proposal, the ladder's own start rung routes the task.
        let config = ladder_config();
        let router = ModelRouter::new(None).with_routing_ladder(ladder(&config, |_| true));
        let mut inputs = RoutingInputs::from_task(&task(), &ctx());
        inputs.attempt_key = Some(AttemptKey::new("gr-m3", "p", "t", 1));
        let (own, _) = router.decide(&inputs).unwrap();
        let ModelChoiceSource::Ladder { rung: start } = own.source else {
            panic!("the ladder routes an unpinned task: {:?}", own.source);
        };
        let own_model = own.model.slug.as_str();

        // Active and eligible: the proposed top rung routes, as the self-model's choice.
        let top = 3;
        assert_ne!(start, top);
        inputs.self_model_rung = Some(top);
        let (choice, row) = router.decide(&inputs).unwrap();
        let expected = ModelChoiceSource::SelfModel { rung: top };
        assert_eq!(routed(&choice), ("claude-sonnet-4-6", expected));
        assert_eq!(row.source, Some(DecisionSource::SelfModel));
        assert_eq!(row.propensity, Some(1.0));
        assert_eq!(row.proposals.learned.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(row.proposals.default.as_deref(), Some(own_model));
        assert_eq!(row.default_model.as_deref(), Some(own_model));
        assert_eq!(row.audit.loop_id.as_deref(), Some("L-M3"));
        let opportunity = row.audit.opportunity.as_ref().expect("L-M3's opportunity");
        assert!(opportunity.eligible, "{opportunity:?}");

        // With ε the propensities compose over the four runnable rungs and sum to 1.
        let p = |row: &RoutingDecisionLog, model: &str| {
            let candidate = row.candidates.iter().find(|c| c.model == model);
            candidate.and_then(|c| c.p)
        };
        let exploring = router.clone().with_explore_epsilon(0.25);
        let (choice, row) = exploring.decide(&inputs).unwrap();
        assert_eq!(p(&row, "claude-sonnet-4-6"), Some(0.8125));
        assert_eq!(p(&row, "gpt-oss-120b"), Some(0.0625));
        let total: f64 = row.candidates.iter().filter_map(|c| c.p).sum();
        assert!((total - 1.0).abs() < 1e-9, "p sums to {total}");
        assert_eq!(row.propensity, p(&row, &choice.model.slug));
        let source = if choice.model.slug == "claude-sonnet-4-6" {
            DecisionSource::SelfModel
        } else {
            DecisionSource::Explore
        };
        assert_eq!(row.source, Some(source));

        // A chain the route layer holds out runs the ladder's own rung, a⁰, at P = h.
        let held_out = Assignment {
            unit: AssignmentUnit::Chain,
            layer: "route".to_string(),
            salt_id: "route@2026-10-03".to_string(),
            u: 0.1,
            h: 0.2,
            g: 0.0,
            arm: Arm::Default,
            propensity: 0.2,
        };
        inputs.arm_set = Some(Arc::new(ArmSet {
            chain_key: "gr-m3:p:t".to_string(),
            arms: [("route".to_string(), held_out)].into(),
            condition_id: "normal".to_string(),
        }));
        let (choice, row) = router.decide(&inputs).unwrap();
        let ladder_source = ModelChoiceSource::Ladder { rung: start };
        assert_eq!(routed(&choice), (own_model, ladder_source));
        assert_eq!(row.source, Some(DecisionSource::Ladder));
        assert_eq!(row.propensity, Some(0.2));
        assert_eq!(row.proposals.learned.as_deref(), Some("claude-sonnet-4-6"));
        inputs.arm_set = None;

        // A pin beats a proposal.
        inputs.force_backend = Some("glm-4.7".to_string());
        let (choice, _) = router.decide(&inputs).unwrap();
        assert_eq!(routed(&choice), ("glm-4.7", ModelChoiceSource::Override));
    }
}
