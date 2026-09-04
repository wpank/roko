//! Three-stage cascade router for model selection (section 13.8-13.11).
//!
//! The cascade combines three routing strategies, automatically transitioning
//! as observation count grows:
//!
//! | Stage | Name | Observations | Strategy |
//! |-------|------|-------------|----------|
//! | 1 | Static | < 50 | Hardcoded role -> model table |
//! | 2 | Confidence | 50 - 200 | Empirical pass rates + confidence interval |
//! | 3 | UCB1 | > 200 | Full `LinUCB` contextual bandit |
//!
//! # [`CascadeModel`]
//!
//! The router returns a [`CascadeModel`] containing a primary model,
//! an ordered fallback chain, an optional context-overflow fallback,
//! and a latency SLA in milliseconds.
//!
//! # Thread safety
//!
//! The cascade wraps a [`LinUCBRouter`] and an additional
//! [`parking_lot::Mutex`] for confidence-stage statistics.

use chrono::Utc;
use indexmap::IndexMap;
use parking_lot::Mutex;
use roko_agent::AgentResult;
use roko_core::DaimonPolicy;
use roko_core::OperatingFrequency;
use roko_core::agent::TaskRequirements;
use roko_core::agent::{AgentRole, ModelSpec, ModelTier};
use roko_core::config::schema::RewardWeights;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::active_inference::{BeliefState, select_tier as select_tier_with_belief};
// Re-export public types from cascade submodules so that
// `crate::cascade_router::CascadeRouter` etc. still works for downstream crates.
pub use crate::cascade::helpers::slug_family;
pub use crate::cascade::types::{
    CascadeCandidateScore, CascadeModel, CascadeObservationStats, CascadeRouteExplanation,
    CascadeRoutingCandidate, CascadeRoutingExplanation, CascadeSelection, CascadeStage,
    DEFAULT_MAX_FALLBACK_DEPTH, FallbackChain, FallbackEvent, GeminiContextTier, GeminiObservation,
    KnowledgeHint, KnowledgeRoutingAdvice, PerplexityObservation, RoutingBias, ShadowModelRunner,
    StageTransition,
};

use crate::cascade::helpers::{
    ProviderHealthSnapshotKey, ThinkingPreference, apply_cache_affinity,
    behavioral_state_tier_shift, conductor_load_tier_shift, context_overflow_fallback_for_model,
    cost_pressure_factor, default_latency_sla, default_role_model_table, estimate_total_cost_usd,
    fallback_chain_for_model, infer_shadow_routing_context, is_free_tier_gemini_model,
    low_confidence_tier_bonus, model_tier_rank, pareto_adjusted_alpha, pareto_cost_proxy,
    pareto_latency_proxy, parse_agent_role, pick_available_static_slug, pick_tier_extreme,
    routing_tier_bias_factor, select_with_hysteresis, shadow_quality_score, slug_to_tier,
    slugs_match, stage_for_observations, target_tier_rank, temperament_exploration_multiplier,
    temperament_tier_shift, thinking_filtered_candidates, thinking_preference,
};
use crate::cascade::persistence::{
    CascadeSnapshot, PersistedModelStats, detect_version_changes, migrated_confidence_stats,
    remap_role_table_entry,
};
use crate::cascade::types::{
    CATEGORY_CONFIDENCE_WEIGHT, CATEGORY_MIN_TRIALS, CategoryModelStats, GeminiObservationTotals,
    HIGH_CFACTOR_THRESHOLD, LOW_AFFECT_CONFIDENCE_THRESHOLD, LOW_CFACTOR_THRESHOLD, ModelStats,
    OVERRIDE_LEARNING_RATE, PARETO_RECOMPUTE_INTERVAL, ParetoFrontierState,
    PerplexityObservationTotals, StageTracking,
};
use crate::cfactor::{AgentDispatchBias, CFactor};
use crate::latency::LatencyTracker;
use crate::model_experiment::ModelExperimentStore;
use crate::model_router::{
    CONTEXT_DIM, CandidateArmScore, LinUCBRouter, RoutingContext, compute_routing_reward_v2,
};
use crate::pareto::{ModelObservation, compute_pareto_frontier};
use crate::provider_health::ProviderHealthRegistry;
use crate::routing_log::{CandidateEntry, RoutingDecisionLog, RoutingDecisionMeta, RoutingLogger};

// ─── CascadeRouter ──────────────────────────────────────────────────────────

/// Three-stage cascade router: Static -> Confidence -> UCB.
///
/// Thread-safe: wrap in `Arc` for shared access.
pub struct CascadeRouter {
    /// The `LinUCB` router used for stage 3 (and observations from all stages).
    linucb: LinUCBRouter,
    /// Per-model statistics for the confidence stage.
    confidence_stats: Mutex<HashMap<String, ModelStats>>,
    /// Cached Pareto frontier used to down-weight dominated models during UCB.
    pareto_frontier: Mutex<ParetoFrontierState>,
    /// Static role -> model table for stage 1.
    role_table: Mutex<HashMap<AgentRole, String>>,
    /// Ordered list of model slugs tracked by the router.
    model_slugs: Vec<String>,
    /// Config-sourced tier map: slug → ModelTier from `roko.toml`.
    ///
    /// Used by [`Self::tier_for_slug`] to resolve tiers from config before
    /// falling back to substring heuristics. Populated from `ModelProfile.tier`
    /// fields at construction time.
    tier_map: HashMap<String, ModelTier>,
    /// Active stage and recorded stage-transition history.
    stage_tracking: Mutex<StageTracking>,
    /// Per-(model, category) pass-rate statistics for category-aware
    /// confidence scoring (audit #84).
    category_stats: Mutex<HashMap<(String, TaskCategory), CategoryModelStats>>,
    /// Optional free-tier Gemini runner used for shadow evaluation.
    free_tier_shadow_runner: Option<Arc<dyn ShadowModelRunner>>,
}

impl std::fmt::Debug for CascadeRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CascadeRouter")
            .field("model_slugs", &self.model_slugs)
            .finish_non_exhaustive()
    }
}

impl roko_core::Cell for CascadeRouter {
    fn cell_id(&self) -> &str {
        "cascade-router"
    }
    fn cell_name(&self) -> &str {
        "CascadeRouter"
    }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::Route]
    }
}

impl Default for RoutingContext {
    fn default() -> Self {
        Self {
            task_category: TaskCategory::Implementation,
            complexity: TaskComplexityBand::Standard,
            iteration: 1,
            role: AgentRole::Implementer,
            crate_familiarity: 0.5,
            has_prior_failure: false,
            conductor_load: 0.0,
            active_agents: 0,
            ready_queue_depth: 0,
            max_queue_wait_hours: 0.0,
            daimon_policy: DaimonPolicy::default(),
            thinking_level: None,
            temperament: None,
            previous_model: None,
            plan_context_tokens: None,
            tier_thresholds: None,
            cfactor: None,
        }
    }
}

impl roko_agent::model_call_service::ForceBackendOverrideRecorder for CascadeRouter {
    fn record_override_outcome(&self, model_slug: &str, success: bool) -> bool {
        // Use the full multi-objective path with a default routing context
        // so that LinUCB observations are recorded even when the caller
        // lacks a real RoutingContext (e.g. from ModelCallService).
        let ctx = RoutingContext::default();
        CascadeRouter::record_override_outcome(self, model_slug, &ctx, success, None)
    }
}

impl CascadeRouter {
    /// Create a cascade router with the given model slugs.
    ///
    /// # Panics
    ///
    /// Panics if `model_slugs` is empty.
    pub fn new(model_slugs: Vec<String>) -> Self {
        assert!(
            !model_slugs.is_empty(),
            "CascadeRouter: need at least one model"
        );
        Self {
            linucb: LinUCBRouter::new(model_slugs.clone()),
            confidence_stats: Mutex::new(HashMap::new()),
            pareto_frontier: Mutex::new(ParetoFrontierState::default()),
            role_table: Mutex::new(default_role_model_table(&model_slugs)),
            tier_map: HashMap::new(),
            model_slugs,
            stage_tracking: Mutex::new(StageTracking {
                current: CascadeStage::Static,
                transitions: Vec::new(),
            }),
            category_stats: Mutex::new(HashMap::new()),
            free_tier_shadow_runner: None,
        }
    }

    /// Set config-sourced tier assignments (builder pattern).
    ///
    /// Pass the `models` map from `RokoConfig` so the router uses explicit
    /// `tier` fields instead of substring heuristics.
    #[must_use]
    pub fn with_model_tiers(
        mut self,
        models: &IndexMap<String, roko_core::config::ModelProfile>,
    ) -> Self {
        self.tier_map = models
            .values()
            .filter_map(|p| p.tier.map(|t| (p.slug.clone(), t)))
            .collect();
        self
    }

    /// Set config-sourced tier assignments (mutable version).
    ///
    /// Use this when the router is already constructed (e.g. after
    /// `LearningRuntime::open`) and you want to inject config tiers.
    pub fn set_model_tiers(&mut self, models: &IndexMap<String, roko_core::config::ModelProfile>) {
        self.tier_map = models
            .values()
            .filter_map(|p| p.tier.map(|t| (p.slug.clone(), t)))
            .collect();
    }

    /// Resolve a model's tier, preferring config over heuristic.
    pub fn tier_for_slug(&self, slug: &str) -> ModelTier {
        slug_to_tier(slug, &self.tier_map)
    }

    /// Override the static role table (builder pattern).
    #[must_use]
    pub fn with_role_table(mut self, table: HashMap<AgentRole, String>) -> Self {
        self.role_table = Mutex::new(table);
        self
    }

    /// Update the Stage 1 static role-to-model mapping for one role.
    pub fn set_static_role_model(&mut self, role: AgentRole, model_slug: impl Into<String>) {
        self.role_table.lock().insert(role, model_slug.into());
    }

    /// Update the static role -> model table used during the cold-start stage.
    pub fn update_static_table(&self, role: AgentRole, model_slug: impl Into<String>) -> bool {
        let model_slug = model_slug.into();
        let mut role_table = self.role_table.lock();
        if role_table
            .get(&role)
            .is_some_and(|current| current == &model_slug)
        {
            return false;
        }
        role_table.insert(role, model_slug);
        true
    }

    /// Override the `LinUCB` router (builder pattern, for injecting pre-trained state).
    #[must_use]
    pub fn with_linucb(mut self, linucb: LinUCBRouter) -> Self {
        self.linucb = linucb;
        self
    }

    /// Enable free-tier Gemini shadow evaluation with the provided runner.
    #[must_use]
    pub fn with_free_tier_shadow_runner(mut self, runner: Arc<dyn ShadowModelRunner>) -> Self {
        self.free_tier_shadow_runner = Some(runner);
        self
    }

    /// Determine the current cascade stage based on total observations.
    #[must_use]
    pub fn current_stage(&self) -> CascadeStage {
        self.stage_tracking.lock().current
    }

    /// Read the ordered model slug history retained by the router.
    #[must_use]
    pub fn model_slugs(&self) -> &[String] {
        &self.model_slugs
    }

    /// Return tracked slugs paired with display availability.
    ///
    /// A slug is considered available when its wire slug appears in
    /// `configured_slugs` or it has at least one successful observation.
    /// `ModelStats` does not retain timestamps, so `successes > 0` is the
    /// best available proxy for "used successfully" without deleting history.
    #[must_use]
    pub fn model_slugs_with_availability(
        &self,
        configured_slugs: &[String],
    ) -> Vec<(String, bool)> {
        let configured: std::collections::HashSet<&str> =
            configured_slugs.iter().map(String::as_str).collect();
        let stats = self.confidence_stats.lock();

        self.model_slugs
            .iter()
            .map(|slug| {
                let available = configured.contains(slug.as_str())
                    || stats.get(slug).map_or(false, |entry| entry.successes > 0);
                (slug.clone(), available)
            })
            .collect()
    }

    /// Total observations recorded across all stages.
    #[must_use]
    pub fn total_observations(&self) -> u64 {
        self.linucb.total_observations()
    }

    /// Record a stage transition when the observation count crosses a stage boundary.
    pub fn check_stage_transition(&self) -> Option<StageTransition> {
        let obs = self.total_observations();
        let next = stage_for_observations(obs);

        let transition = {
            let mut tracking = self.stage_tracking.lock();
            if next == tracking.current {
                return None;
            }

            let transition = StageTransition {
                from: tracking.current,
                to: next,
                observations: obs,
                timestamp: Utc::now(),
            };
            tracking.current = next;
            tracking.transitions.push(transition.clone());
            transition
        };

        tracing::info!(
            from = %transition.from,
            to = %transition.to,
            observations = transition.observations,
            timestamp = %transition.timestamp.to_rfc3339(),
            "cascade router stage transition"
        );

        Some(transition)
    }

    /// Return the recorded stage-transition history.
    #[must_use]
    pub fn stage_transitions(&self) -> Vec<StageTransition> {
        self.stage_tracking.lock().transitions.clone()
    }

    /// Select a model from a raw context vector.
    #[must_use]
    pub fn select(&self, context_vec: Vec<f64>) -> CascadeSelection {
        let observations = self.total_observations();
        let stage = stage_for_observations(observations);
        let model = self.linucb.select_features(&context_vec);
        CascadeSelection {
            model,
            observations,
            stage,
        }
    }

    /// Select a model for a given operating frequency.
    #[must_use]
    pub fn select_for_frequency(
        &self,
        frequency: OperatingFrequency,
        ctx: Option<&RoutingContext>,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> Option<ModelSpec> {
        match frequency {
            OperatingFrequency::Gamma => None,
            OperatingFrequency::Theta => {
                ctx.map(|ctx| self.route_with_cfactor(ctx, cfactor, agent_id).primary)
            }
            OperatingFrequency::Delta => {
                Some(self.bias_model_for_cfactor(self.strongest_model(), cfactor, agent_id))
            }
        }
    }

    /// Select a model for a given operating frequency from a candidate subset.
    #[must_use]
    pub fn select_for_frequency_among(
        &self,
        frequency: OperatingFrequency,
        ctx: Option<&RoutingContext>,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
        candidates: &[String],
    ) -> Option<ModelSpec> {
        let candidates = if candidates.is_empty() {
            &self.model_slugs
        } else {
            candidates
        };

        match frequency {
            OperatingFrequency::Gamma => None,
            OperatingFrequency::Theta => ctx.map(|ctx| {
                self.route_with_cfactor_among(ctx, candidates, cfactor, agent_id)
                    .primary
            }),
            OperatingFrequency::Delta => Some(self.bias_model_for_cfactor_among(
                self.strongest_model_among(candidates),
                cfactor,
                agent_id,
                candidates,
            )),
        }
    }

    /// Select a model for a given operating frequency from a candidate subset,
    /// incorporating knowledge-informed routing advice into the selection.
    ///
    /// This is the knowledge-aware variant of [`Self::select_for_frequency_among`].
    /// When knowledge advice is provided and the frequency is `Theta`, the router
    /// uses [`Self::route_with_knowledge_among`] so that neuro knowledge store
    /// hints bias candidate scores **during** selection rather than only in
    /// post-hoc explanation enrichment.
    #[must_use]
    pub fn select_for_frequency_among_with_knowledge(
        &self,
        frequency: OperatingFrequency,
        ctx: Option<&RoutingContext>,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
        candidates: &[String],
        knowledge: Option<&KnowledgeRoutingAdvice>,
    ) -> Option<ModelSpec> {
        let candidates = if candidates.is_empty() {
            &self.model_slugs
        } else {
            candidates
        };

        match frequency {
            OperatingFrequency::Gamma => None,
            OperatingFrequency::Theta => ctx.map(|ctx| {
                self.route_with_knowledge_among(ctx, candidates, cfactor, agent_id, knowledge)
                    .primary
            }),
            OperatingFrequency::Delta => Some(self.bias_model_for_cfactor_among(
                self.strongest_model_among(candidates),
                cfactor,
                agent_id,
                candidates,
            )),
        }
    }

    /// Select a tier using the active-inference belief state.
    #[must_use]
    pub fn select_tier_with_active_inference(
        &self,
        belief: &BeliefState,
        requirements: &TaskRequirements,
    ) -> ModelTier {
        let _ = self;
        select_tier_with_belief(belief, requirements)
    }

    /// Return the strongest model currently available to the router.
    #[must_use]
    pub fn strongest_model(&self) -> ModelSpec {
        let mut best_slug = self
            .model_slugs
            .first()
            .cloned()
            .expect("CascadeRouter: need at least one model");
        let mut best_rank = model_tier_rank(slug_to_tier(&best_slug, &self.tier_map));

        for slug in self.model_slugs.iter().skip(1) {
            let rank = model_tier_rank(slug_to_tier(slug, &self.tier_map));
            if rank > best_rank {
                best_rank = rank;
                best_slug.clone_from(slug);
            }
        }

        ModelSpec::from_slug(best_slug)
    }

    /// Return the cheapest model currently available to the router.
    #[must_use]
    pub fn cheapest_model(&self) -> ModelSpec {
        let mut best_slug = self
            .model_slugs
            .first()
            .cloned()
            .expect("CascadeRouter: need at least one model");
        let mut best_rank = model_tier_rank(slug_to_tier(&best_slug, &self.tier_map));

        for slug in self.model_slugs.iter().skip(1) {
            let rank = model_tier_rank(slug_to_tier(slug, &self.tier_map));
            if rank < best_rank {
                best_rank = rank;
                best_slug.clone_from(slug);
            }
        }

        ModelSpec::from_slug(best_slug)
    }

    /// Return the strongest model from `candidates`.
    #[must_use]
    pub fn strongest_model_among(&self, candidates: &[String]) -> ModelSpec {
        let mut best_slug = candidates
            .first()
            .cloned()
            .unwrap_or_else(|| self.strongest_model().slug);
        let mut best_rank = model_tier_rank(slug_to_tier(&best_slug, &self.tier_map));

        for slug in candidates.iter().skip(1) {
            let rank = model_tier_rank(slug_to_tier(slug, &self.tier_map));
            if rank > best_rank {
                best_rank = rank;
                best_slug.clone_from(slug);
            }
        }

        ModelSpec::from_slug(best_slug)
    }

    /// Return the cheapest model from `candidates`.
    #[must_use]
    pub fn cheapest_model_among(&self, candidates: &[String]) -> ModelSpec {
        let mut best_slug = candidates
            .first()
            .cloned()
            .unwrap_or_else(|| self.cheapest_model().slug);
        let mut best_rank = model_tier_rank(slug_to_tier(&best_slug, &self.tier_map));

        for slug in candidates.iter().skip(1) {
            let rank = model_tier_rank(slug_to_tier(slug, &self.tier_map));
            if rank < best_rank {
                best_rank = rank;
                best_slug.clone_from(slug);
            }
        }

        ModelSpec::from_slug(best_slug)
    }

    fn retarget_route_primary(
        &self,
        mut route: CascadeModel,
        candidates: &[String],
        primary: ModelSpec,
    ) -> CascadeModel {
        let tier = slug_to_tier(&primary.slug, &self.tier_map);
        route.fallback_chain = fallback_chain_for_model(candidates, &primary.slug, &self.tier_map);
        route.context_overflow_fallback =
            context_overflow_fallback_for_model(candidates, &primary.slug, &self.tier_map);
        route.latency_sla_ms = default_latency_sla(tier);
        route.primary = primary;
        route
    }

    fn bias_model_for_behavioral_state_among(
        &self,
        model: ModelSpec,
        ctx: &RoutingContext,
        candidates: &[String],
    ) -> ModelSpec {
        let shift = behavioral_state_tier_shift(ctx);
        if shift == 0 {
            return model;
        }

        let current_rank = model_tier_rank(slug_to_tier(&model.slug, &self.tier_map));
        let target_rank = target_tier_rank(current_rank, shift);
        candidates
            .iter()
            .find(|slug| model_tier_rank(slug_to_tier(slug, &self.tier_map)) == target_rank)
            .map(ModelSpec::from_slug)
            .unwrap_or(model)
    }

    fn bias_model_for_conductor_load_among(
        &self,
        model: ModelSpec,
        ctx: &RoutingContext,
        candidates: &[String],
    ) -> ModelSpec {
        let shift = conductor_load_tier_shift(ctx);
        if shift == 0 {
            return model;
        }

        let current_rank = model_tier_rank(slug_to_tier(&model.slug, &self.tier_map));
        let target_rank = target_tier_rank(current_rank, shift);
        candidates
            .iter()
            .find(|slug| model_tier_rank(slug_to_tier(slug, &self.tier_map)) == target_rank)
            .map(ModelSpec::from_slug)
            .unwrap_or(model)
    }

    fn bias_model_for_temperament_among(
        &self,
        model: ModelSpec,
        ctx: &RoutingContext,
        candidates: &[String],
    ) -> ModelSpec {
        let shift = temperament_tier_shift(ctx);
        if shift == 0 {
            return model;
        }

        let current_rank = model_tier_rank(slug_to_tier(&model.slug, &self.tier_map));
        let target_rank = target_tier_rank(current_rank, shift);
        candidates
            .iter()
            .find(|slug| model_tier_rank(slug_to_tier(slug, &self.tier_map)) == target_rank)
            .map(ModelSpec::from_slug)
            .unwrap_or(model)
    }

    fn apply_context_biases_among(
        &self,
        route: CascadeModel,
        ctx: &RoutingContext,
        candidates: &[String],
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        let primary =
            self.bias_model_for_behavioral_state_among(route.primary.clone(), ctx, candidates);
        let route = self.retarget_route_primary(route, candidates, primary);
        let primary = self.bias_model_for_temperament_among(route.primary.clone(), ctx, candidates);
        let route = self.retarget_route_primary(route, candidates, primary);
        let primary =
            self.bias_model_for_cfactor_among(route.primary.clone(), cfactor, agent_id, candidates);
        let route = self.retarget_route_primary(route, candidates, primary);
        let primary =
            self.bias_model_for_conductor_load_among(route.primary.clone(), ctx, candidates);
        self.retarget_route_primary(route, candidates, primary)
    }

    fn apply_knowledge_to_route(
        &self,
        ctx: &RoutingContext,
        route: CascadeModel,
        candidates: &[String],
        knowledge: Option<&KnowledgeRoutingAdvice>,
    ) -> CascadeModel {
        let Some(knowledge) = knowledge else {
            return route;
        };
        if !knowledge.has_signal {
            return route;
        }

        let _primary_hint = knowledge.hint_for(&route.primary.slug);
        let frontier = self.current_pareto_frontier();
        let linucb_scores: HashMap<_, _> = self
            .ucb_scores(ctx, candidates, frontier.as_deref())
            .into_iter()
            .collect();

        let adjusted_score = |slug: &str| {
            let base = linucb_scores
                .get(slug)
                .copied()
                .filter(|score| score.is_finite())
                .unwrap_or(0.0);
            base + knowledge.score_for(slug)
        };

        let current_score = adjusted_score(&route.primary.slug);
        let best = candidates
            .iter()
            .map(|slug| (slug, adjusted_score(slug)))
            .max_by(|(left_slug, left_score), (right_slug, right_score)| {
                left_score
                    .total_cmp(right_score)
                    .then_with(|| right_slug.cmp(left_slug))
            });

        let mut swapped = false;
        let route = if let Some((best_slug, best_score)) = best {
            if !slugs_match(best_slug, &route.primary.slug) && best_score - current_score > 0.1 {
                swapped = true;
                self.retarget_route_primary(route, candidates, ModelSpec::from_slug(best_slug))
            } else {
                route
            }
        } else {
            route
        };

        tracing::debug!(
            primary = %route.primary.slug,
            knowledge_hints = ?knowledge.hints.len(),
            swapped = %swapped,
            "knowledge-aware routing applied"
        );

        route
    }

    /// Return the index of `slug` in the router's model list.
    #[must_use]
    pub fn model_index_for_slug(&self, slug: &str) -> Option<usize> {
        self.linucb.model_index(slug)
    }

    /// Route a context through the cascade, returning a recommendation.
    pub fn route(&self, ctx: &RoutingContext) -> CascadeModel {
        self.route_with_cfactor(ctx, None, None)
    }

    /// Route a context through the cascade and append a routing decision log.
    pub fn route_logged(
        &self,
        ctx: &RoutingContext,
        log: &RoutingLogger,
        meta: &RoutingDecisionMeta,
    ) -> std::io::Result<(CascadeModel, RoutingDecisionLog)> {
        let selected = self.route(ctx);
        let explanation = self.explain_route(ctx, None);
        let record =
            self.append_routing_log(log, meta, &selected.primary.slug, Some(&explanation))?;
        Ok((selected, record))
    }

    /// Route a context, overriding selection when a model experiment is active.
    pub fn route_with_experiments(
        &self,
        ctx: &RoutingContext,
        experiments: &ModelExperimentStore,
    ) -> CascadeModel {
        if let Some(variant) =
            experiments.assign_model(ctx.role.label(), ctx.task_category.label(), "")
        {
            return CascadeModel {
                primary: ModelSpec::from_slug(&variant.slug),
                fallback_chain: Vec::new(),
                context_overflow_fallback: None,
                latency_sla_ms: 30_000,
                stage: CascadeStage::Static,
            };
        }

        self.route(ctx)
    }

    /// Route a context, excluding models whose provider is currently unavailable.
    pub fn route_with_health(
        &self,
        ctx: &RoutingContext,
        health: &ProviderHealthRegistry,
        model_providers: &HashMap<String, String>,
    ) -> CascadeModel {
        let available: Vec<String> = self
            .model_slugs
            .iter()
            .filter(|slug| {
                model_providers
                    .get(slug.as_str())
                    .map(|provider_id| health.is_available(provider_id))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        if available.is_empty() {
            return self.route(ctx);
        }

        match self.current_stage() {
            CascadeStage::Static => self.route_static_filtered(ctx, &available),
            CascadeStage::Confidence => self.route_confidence_filtered(ctx, &available),
            CascadeStage::Ucb => self.route_ucb_filtered(ctx, &available),
        }
    }

    /// Route a context, excluding unavailable providers and demoting degraded ones.
    ///
    /// Providers in `Open` state (circuit tripped) are fully excluded from
    /// candidate selection. Providers in `HalfOpen` state (recovering, one probe
    /// allowed) are retained but demoted: their latency SLA is doubled to
    /// signal reduced confidence. When every provider is unavailable the router
    /// falls back to the plain unfiltered route so dispatch never hard-fails
    /// due to health data alone.
    ///
    /// `latency_threshold_ms` is an optional per-provider p95 latency ceiling.
    /// When a provider's tracked p95 latency exceeds this threshold its models
    /// are demoted to the next-cheaper tier before selection.  Pass `None` to
    /// skip latency-based demotion.
    pub fn route_with_health_scored(
        &self,
        ctx: &RoutingContext,
        health: &ProviderHealthRegistry,
        model_providers: &HashMap<String, String>,
        latency_registry: Option<&crate::latency::LatencyRegistry>,
        latency_threshold_ms: Option<f64>,
    ) -> CascadeModel {
        // Partition candidates into available (Closed/HalfOpen) and
        // unavailable (Open / hard-down).
        let available: Vec<String> = self
            .model_slugs
            .iter()
            .filter(|slug| {
                model_providers
                    .get(slug.as_str())
                    .map(|provider_id| health.is_available(provider_id))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        if available.is_empty() {
            // All providers are circuit-open — route anyway so we don't stall.
            tracing::warn!("all known providers are circuit-open; routing without health filter");
            return self.route(ctx);
        }

        // Apply latency-based demotion: collect slugs whose provider p95
        // exceeds the threshold so we can filter them out of the preferred
        // set and fall back to the full available list.
        let latency_degraded: std::collections::HashSet<String> =
            if let (Some(registry), Some(threshold)) = (latency_registry, latency_threshold_ms) {
                available
                    .iter()
                    .filter(|slug| {
                        let provider = model_providers
                            .get(slug.as_str())
                            .map_or(slug.as_str(), String::as_str);
                        let stats = registry.get_all_for_provider(provider);
                        stats.observations >= 5 && stats.p95_ms() > threshold
                    })
                    .cloned()
                    .collect()
            } else {
                std::collections::HashSet::new()
            };

        // Prefer candidates whose provider is fast; fall back to all available
        // when latency demotion would remove everyone.
        let preferred: Vec<String> = available
            .iter()
            .filter(|slug| !latency_degraded.contains(slug.as_str()))
            .cloned()
            .collect();

        let candidates = if preferred.is_empty() {
            tracing::debug!(
                demoted = latency_degraded.len(),
                "all available providers exceed latency threshold; using full available set"
            );
            &available
        } else {
            &preferred
        };

        // Route within the healthy candidate set.
        let mut route = match self.current_stage() {
            CascadeStage::Static => self.route_static_filtered(ctx, candidates),
            CascadeStage::Confidence => self.route_confidence_filtered(ctx, candidates),
            CascadeStage::Ucb => self.route_ucb_filtered(ctx, candidates),
        };

        // When the chosen provider is HalfOpen (recovering), widen the latency
        // SLA to signal reduced confidence to the caller.
        let primary_provider = model_providers
            .get(route.primary.slug.as_str())
            .map_or(route.primary.slug.as_str(), String::as_str);
        let primary_health = health.get(primary_provider);
        if primary_health.state == crate::provider_health::CircuitState::HalfOpen {
            route.latency_sla_ms = route.latency_sla_ms.saturating_mul(2);
            tracing::debug!(
                provider = primary_provider,
                model = %route.primary.slug,
                latency_sla_ms = route.latency_sla_ms,
                "selected HalfOpen provider — latency SLA doubled as recovery signal"
            );
        }

        // Extend the fallback chain with any still-available candidates that
        // weren't in the preferred set (latency-degraded but reachable).
        let existing_slugs: std::collections::HashSet<&str> = route
            .fallback_chain
            .iter()
            .map(|m| m.slug.as_str())
            .collect();
        let extra_fallbacks: Vec<roko_core::agent::ModelSpec> = latency_degraded
            .into_iter()
            .filter(|slug| !existing_slugs.contains(slug.as_str()))
            .map(|slug| roko_core::agent::ModelSpec::from_slug(&slug))
            .collect();
        route.fallback_chain.extend(extra_fallbacks);

        route
    }

    /// Remove candidates whose provider is currently unhealthy.
    #[must_use]
    pub fn filter_unhealthy(
        &self,
        candidates: &[String],
        health: &ProviderHealthRegistry,
        model_providers: &HashMap<String, String>,
    ) -> Vec<String> {
        if candidates.is_empty() {
            return Vec::new();
        }

        let available: Vec<String> = candidates
            .iter()
            .filter(|slug| {
                let provider = model_providers
                    .get(slug.as_str())
                    .map_or(slug.as_str(), String::as_str);
                health.is_healthy(provider)
            })
            .cloned()
            .collect();
        if !available.is_empty() {
            return available;
        }

        let snapshot = health.snapshot();
        let mut ranked: Vec<(String, ProviderHealthSnapshotKey)> = candidates
            .iter()
            .map(|slug| {
                let provider = model_providers
                    .get(slug.as_str())
                    .map_or(slug.as_str(), String::as_str);
                let health_record = snapshot
                    .get(provider)
                    .cloned()
                    .unwrap_or_else(|| health.get(provider));
                (
                    slug.clone(),
                    ProviderHealthSnapshotKey::from(&health_record),
                )
            })
            .collect();

        ranked.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
        ranked
            .first()
            .map(|(slug, _)| vec![slug.clone()])
            .unwrap_or_default()
    }

    /// Apply a routing bias to scored candidates.
    pub fn apply_bias(&self, candidates: &mut [(String, f64)], bias: &RoutingBias) {
        if bias.deprioritize.is_empty() && !bias.prefer_cheaper {
            return;
        }

        for (slug, score) in candidates.iter_mut() {
            if bias
                .deprioritize
                .iter()
                .any(|blocked| slugs_match(slug, blocked))
            {
                *score *= 0.5;
            }

            if bias.prefer_cheaper {
                *score *= routing_tier_bias_factor(slug_to_tier(slug, &self.tier_map));
            }
        }
    }

    /// Apply cost pressure to scored candidates.
    pub fn apply_cost_pressure(&self, candidates: &mut [(String, f64)], spike: bool) {
        if !spike {
            return;
        }

        for (slug, score) in candidates.iter_mut() {
            *score *= cost_pressure_factor(slug_to_tier(slug, &self.tier_map));
        }
    }

    /// Route a context, applying conductor routing bias directly.
    pub fn route_with_bias(&self, ctx: &RoutingContext, bias: &RoutingBias) -> CascadeModel {
        let candidates: Vec<String> = self
            .model_slugs
            .iter()
            .filter(|slug| !bias.deprioritize.iter().any(|d| slugs_match(slug, d)))
            .cloned()
            .collect();

        if candidates.is_empty() {
            return self.route(ctx);
        }

        self.route_with_cfactor_among(ctx, &candidates, None, None)
    }

    /// Load static routing overrides from a JSON map of role labels to model slugs.
    pub fn load_static_overrides(&self, path: &Path) -> Result<usize, crate::error::LearnError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(err) => {
                return Err(crate::error::LearnError::Io {
                    path: path.display().to_string(),
                    source: err,
                });
            }
        };
        let overrides =
            serde_json::from_str::<HashMap<String, String>>(&contents).map_err(|err| {
                crate::error::LearnError::Corrupt {
                    path: path.display().to_string(),
                    reason: err.to_string(),
                }
            })?;

        let mut applied = 0usize;
        for (parameter, winning_value) in overrides {
            if let Some(role) = parse_agent_role(&parameter) {
                if self.update_static_table(role, winning_value) {
                    applied += 1;
                }
            }
        }
        Ok(applied)
    }

    /// Compute a latency penalty using actual wall-clock latency.
    #[must_use]
    pub fn latency_penalty(actual_ms: f64, expected_ms: f64) -> f64 {
        if expected_ms <= 0.0 {
            return 0.0;
        }
        0.1 * (actual_ms / expected_ms - 1.0).max(0.0)
    }

    /// Convert a quality score into a latency-adjusted reward.
    #[must_use]
    pub fn reward_with_latency(
        quality_score: f64,
        actual_ms: Option<f64>,
        expected_ms: f64,
    ) -> f64 {
        let penalty = actual_ms
            .map(|actual_ms| Self::latency_penalty(actual_ms, expected_ms))
            .unwrap_or(0.0);
        (quality_score - penalty).max(0.0)
    }

    /// Convert a tracker-backed latency observation into a reward signal.
    #[must_use]
    pub fn reward_with_tracker_latency(
        &self,
        quality_score: f64,
        model: &str,
        tracker: &LatencyTracker,
        expected_ms: f64,
    ) -> f64 {
        let actual_ms = tracker
            .mean_latency(model)
            .or_else(|| tracker.p95_latency(model));
        Self::reward_with_latency(quality_score, actual_ms, expected_ms)
    }

    /// Route a context through the cascade, optionally biasing by C-Factor.
    pub fn route_with_cfactor(
        &self,
        ctx: &RoutingContext,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        match self.current_stage() {
            CascadeStage::Static => self.route_static(ctx, cfactor, agent_id),
            CascadeStage::Confidence => self.route_confidence(ctx, cfactor, agent_id),
            CascadeStage::Ucb => self.route_ucb(ctx, cfactor, agent_id),
        }
    }

    /// Route a context through the cascade, optionally biasing by C-Factor and knowledge hints.
    pub fn route_with_knowledge(
        &self,
        ctx: &RoutingContext,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
        knowledge: Option<&KnowledgeRoutingAdvice>,
    ) -> CascadeModel {
        let route = self.route_with_cfactor(ctx, cfactor, agent_id);
        self.apply_knowledge_to_route(ctx, route, &self.model_slugs, knowledge)
    }

    /// Route a context through the cascade over a candidate subset.
    pub fn route_with_cfactor_among(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        let candidates = if candidates.is_empty() {
            &self.model_slugs
        } else {
            candidates
        };

        match self.current_stage() {
            CascadeStage::Static => self.route_static_among(ctx, candidates, cfactor, agent_id),
            CascadeStage::Confidence => {
                self.route_confidence_among(ctx, candidates, cfactor, agent_id)
            }
            CascadeStage::Ucb => self.route_ucb_among(ctx, candidates, cfactor, agent_id),
        }
    }

    /// Route a context through the cascade over a candidate subset with knowledge hints.
    pub fn route_with_knowledge_among(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
        knowledge: Option<&KnowledgeRoutingAdvice>,
    ) -> CascadeModel {
        let route = self.route_with_cfactor_among(ctx, candidates, cfactor, agent_id);
        let candidates = if candidates.is_empty() {
            &self.model_slugs
        } else {
            candidates
        };
        self.apply_knowledge_to_route(ctx, route, candidates, knowledge)
    }

    /// Explain a routing decision over the supplied candidate set.
    #[must_use]
    pub fn explain_routing(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
    ) -> CascadeRoutingExplanation {
        let candidates = if candidates.is_empty() {
            self.model_slugs.clone()
        } else {
            candidates.to_vec()
        };

        let route = match self.current_stage() {
            CascadeStage::Static => self.route_static_filtered(ctx, &candidates),
            CascadeStage::Confidence => self.route_confidence_filtered(ctx, &candidates),
            CascadeStage::Ucb => self.route_ucb_filtered(ctx, &candidates),
        };

        let frontier = self.current_pareto_frontier();
        let scores = self.stage_scores(ctx, &candidates, frontier.as_deref());
        let score_map: HashMap<_, _> = scores.into_iter().collect();

        let mut explained: Vec<_> = candidates
            .into_iter()
            .map(|slug| CascadeRoutingCandidate {
                cache_affinity: ctx.previous_model.as_deref() == Some(slug.as_str()),
                pareto_optimal: frontier
                    .as_ref()
                    .map(|frontier| frontier.iter().any(|frontier_slug| frontier_slug == &slug)),
                score: score_map.get(&slug).copied().unwrap_or(0.0),
                selected: slugs_match(&slug, &route.primary.slug),
                model: slug,
            })
            .collect();

        explained.sort_by(|a, b| {
            b.selected
                .cmp(&a.selected)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.model.cmp(&b.model))
        });

        CascadeRoutingExplanation {
            stage: route.stage,
            selected_model: route.primary.slug,
            fallback_model: route.fallback_chain.first().map(|model| model.slug.clone()),
            latency_sla_ms: route.latency_sla_ms,
            candidates: explained,
        }
    }

    /// Record an observation (updates both confidence stats and `LinUCB`).
    pub fn record_observation(
        &self,
        ctx: &RoutingContext,
        model_slug: &str,
        reward: f64,
        success: bool,
    ) {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            return;
        };
        self.observe_internal(&ctx.to_features(), model_idx, reward, success, None, None);
    }

    /// Apply a WAL-replayed observation. Does NOT write a WAL entry.
    ///
    /// Validates that `model_idx` still maps to `model_slug`. If the config
    /// has changed since the WAL was written (e.g. model list reordered),
    /// the entry is silently skipped with a warning.
    pub fn replay_observation(
        &self,
        model_slug: &str,
        context_features: &[f64],
        model_idx: usize,
        reward: f64,
        success: bool,
    ) {
        // Validate the slug/index pair is still valid after potential config changes.
        let current_idx = self.model_index_for_slug(model_slug);
        let effective_idx = match current_idx {
            Some(idx) => idx,
            None => {
                tracing::warn!(
                    slug = %model_slug,
                    model_idx,
                    "[wal] replay: unknown model slug -- skipping observation"
                );
                return;
            }
        };
        if effective_idx != model_idx {
            tracing::warn!(
                slug = %model_slug,
                wal_idx = model_idx,
                current_idx = effective_idx,
                "[wal] replay: model index changed -- using current index"
            );
        }
        self.observe_internal(context_features, effective_idx, reward, success, None, None);
    }

    /// Record an observation enriched with Perplexity search metadata.
    pub fn record_perplexity_observation(
        &self,
        ctx: &RoutingContext,
        model_slug: &str,
        reward: f64,
        success: bool,
        observation: PerplexityObservation,
    ) -> bool {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            return false;
        };

        let perplexity = PerplexityObservationTotals {
            citation_count: observation.citation_count,
            search_latency_ms: observation.search_latency_ms,
            total_cost_usd: estimate_total_cost_usd(
                model_slug,
                observation.input_tokens,
                observation.output_tokens,
            ),
        };
        self.observe_internal(
            &ctx.to_features(),
            model_idx,
            reward,
            success,
            Some(perplexity),
            None,
        );
        true
    }

    /// Record an observation enriched with Gemini-native metadata.
    pub fn record_gemini_observation(
        &self,
        ctx: &RoutingContext,
        model_slug: &str,
        reward: f64,
        success: bool,
        observation: GeminiObservation,
    ) -> bool {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            return false;
        };

        let gemini = GeminiObservationTotals {
            thinking_tokens: observation.thinking_tokens.unwrap_or(0),
            cached_tokens: observation.cached_tokens.unwrap_or(0),
            grounding_query_count: observation.grounding_query_count,
            code_execution_success_count: observation.code_execution_success_count,
            code_execution_failure_count: observation.code_execution_failure_count,
            context_tier: observation.context_tier,
        };
        self.observe_internal(
            &ctx.to_features(),
            model_idx,
            reward,
            success,
            None,
            Some(gemini),
        );
        true
    }

    /// Record a binary confidence-only outcome for `model_slug`.
    ///
    /// This path intentionally does not update the contextual `LinUCB` bandit
    /// because the caller has not supplied a real [`RoutingContext`].
    pub fn record_confidence_outcome(&self, model_slug: &str, success: bool) -> bool {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            tracing::warn!(
                slug = %model_slug,
                success,
                "cascade router: unknown model slug -- outcome dropped. \
                 Add this model to [models] in roko.toml or provider config \
                 so the router can track it."
            );
            return false;
        };

        let Some(slug) = self.model_slugs.get(model_idx) else {
            return false;
        };

        let mut stats = self.confidence_stats.lock();
        let entry = stats.entry(slug.clone()).or_default();
        entry.trials += 1;
        if success {
            entry.successes += 1;
        }
        true
    }

    /// Apply a calibration correction to adjust confidence stats for a model.
    ///
    /// When the `CalibrationPolicy` detects systematic bias (overconfidence or
    /// underconfidence), it produces a `CalibrationCorrection`. This method
    /// adjusts the model's empirical pass rate by injecting synthetic
    /// observations that counteract the detected bias.
    ///
    /// For an overconfident model (positive `mean_bias`): inject virtual failures
    /// to lower the empirical pass rate.
    /// For an underconfident model (negative `mean_bias`): inject virtual successes
    /// to raise the empirical pass rate.
    ///
    /// The number of synthetic observations is proportional to both the magnitude
    /// of the correction and the existing sample count, capped to avoid
    /// overwhelming real data.
    pub fn apply_calibration_correction(
        &self,
        correction: &crate::calibration_policy::CalibrationCorrection,
    ) {
        let mut stats = self.confidence_stats.lock();
        let entry = stats.entry(correction.model.clone()).or_default();

        // Determine how many synthetic trials to inject. We use a fraction of
        // the existing trial count scaled by the correction magnitude. Cap at
        // 10% of existing trials to avoid overwhelming real observations.
        let existing_trials = entry.trials.max(1) as f64;
        let magnitude = correction.mean_bias.abs().clamp(0.0, 1.0);
        let synthetic_count = (existing_trials * magnitude * 0.1).ceil() as u64;

        if synthetic_count == 0 {
            return;
        }

        if correction.mean_bias > 0.0 {
            // Overconfident: inject synthetic failures (add trials but not successes).
            entry.trials += synthetic_count;
        } else {
            // Underconfident: inject synthetic successes (add both trials and successes).
            entry.trials += synthetic_count;
            entry.successes += synthetic_count;
        }

        tracing::debug!(
            model = %correction.model,
            bias = correction.mean_bias,
            synthetic_trials = synthetic_count,
            new_pass_rate = entry.pass_rate(),
            "calibration correction applied to confidence stats"
        );
    }

    /// Record a manual model override outcome for learning (UX34).
    ///
    /// Called when the operator used `--model` / `--force-model` /
    /// `--force-backend` to bypass the cascade router. Uses the full
    /// multi-objective observation path so that LinUCB context is updated
    /// alongside confidence stats.  The `dampening` factor (0.0--1.0)
    /// scales the quality signal to prevent a single user override from
    /// dominating the bandit policy.  Pass `None` to use the built-in
    /// default (`OVERRIDE_LEARNING_RATE`, currently 0.5).
    pub fn record_override_outcome(
        &self,
        model_slug: &str,
        ctx: &RoutingContext,
        success: bool,
        dampening: Option<f64>,
    ) -> bool {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            return false;
        };
        let damp = dampening.unwrap_or(OVERRIDE_LEARNING_RATE).clamp(0.0, 1.0);
        let raw_quality = if success { 1.0 } else { 0.0 };
        let dampened_quality = raw_quality * damp;
        // For overrides we have no cost/latency telemetry, so use neutral
        // values (0.0) and let only the dampened quality signal drive learning.
        let weights = RewardWeights::default();
        self.observe_multi_objective(
            ctx.to_features(),
            model_idx,
            dampened_quality,
            0.0,
            0.0,
            &weights,
        );
        true
    }

    /// Record a per-category observation for the given model (audit #84).
    ///
    /// Called alongside the main observation path so the router can
    /// adjust confidence scores based on how a model performs on a
    /// specific task category (e.g. research, implementation, refactor).
    pub fn record_category_outcome(&self, model_slug: &str, category: TaskCategory, success: bool) {
        let mut cat = self.category_stats.lock();
        let entry = cat.entry((model_slug.to_string(), category)).or_default();
        entry.trials += 1;
        if success {
            entry.successes += 1;
        }
    }

    /// Look up the category-specific pass-rate deviation for a model.
    ///
    /// Returns the delta between the category pass rate and the global
    /// pass rate, or 0.0 if there are too few category observations.
    fn category_pass_rate_delta(
        &self,
        slug: &str,
        category: TaskCategory,
        global_pass_rate: f64,
    ) -> f64 {
        let cat = self.category_stats.lock();
        let Some(entry) = cat.get(&(slug.to_string(), category)) else {
            return 0.0;
        };
        if entry.trials < CATEGORY_MIN_TRIALS {
            return 0.0;
        }
        entry.pass_rate() - global_pass_rate
    }

    /// Record the outcome of a rate-limit fallback attempt for cascade learning.
    ///
    /// Call this after every dispatch that was triggered by a [`FallbackEvent`]
    /// (i.e. a dispatch to the fallback model selected by [`FallbackChain::advance`]).
    ///
    /// Two observations are written:
    ///
    /// 1. A **failure** for the rate-limited model (`event.rate_limited_slug`),
    ///    recording that the rate limit is an adverse outcome for that model.
    /// 2. A **success or failure** for the fallback model (`event.fallback_slug`),
    ///    depending on whether the fallback dispatch succeeded.
    ///
    /// Both observations use the confidence-stage path so they accumulate
    /// toward the empirical pass-rate table used by Stage 2 and LinUCB.
    ///
    /// Returns `(rate_limited_recorded, fallback_recorded)` — each `true` when
    /// the corresponding slug was known to the router.
    pub fn record_fallback_outcome(
        &self,
        event: &FallbackEvent,
        fallback_succeeded: bool,
    ) -> (bool, bool) {
        tracing::warn!(
            rate_limited = %event.rate_limited_slug,
            fallback = %event.fallback_slug,
            depth = event.depth,
            success = fallback_succeeded,
            "rate-limited on {}, falling back to {} (depth={})",
            event.rate_limited_slug,
            event.fallback_slug,
            event.depth,
        );

        // Record the rate-limited model as a failure so the router learns to
        // prefer alternatives when this provider is under quota pressure.
        let rate_limited_recorded = self.record_confidence_outcome(&event.rate_limited_slug, false);

        // Record the fallback model with the actual outcome.
        let fallback_recorded =
            self.record_confidence_outcome(&event.fallback_slug, fallback_succeeded);

        (rate_limited_recorded, fallback_recorded)
    }

    /// Run a shadow evaluation against a free-tier Gemini model.
    pub async fn shadow_evaluate(
        &mut self,
        prompt: &str,
        primary_result: &AgentResult,
        free_model: &str,
    ) {
        if !is_free_tier_gemini_model(free_model) {
            return;
        }

        let Some(model_idx) = self.model_index_for_slug(free_model) else {
            return;
        };
        let Some(runner) = self.free_tier_shadow_runner.clone() else {
            return;
        };

        let prompt = prompt.trim();
        if prompt.is_empty() {
            return;
        }

        let shadow_result = runner.run_shadow(prompt, free_model).await;
        let quality = shadow_quality_score(prompt, primary_result, &shadow_result);
        let passed = quality >= 0.65;
        let ctx = infer_shadow_routing_context(prompt, primary_result);
        let reward = if passed {
            compute_routing_reward_v2(
                quality,
                0.0,
                shadow_result.usage.wall_ms as f64,
                default_latency_sla(slug_to_tier(free_model, &self.tier_map)) as f64,
            )
        } else {
            0.0
        };

        self.observe_internal(
            &ctx.to_features_for_model(Some(free_model)),
            model_idx,
            reward,
            passed,
            None,
            None,
        );
    }

    /// Record a successful observation from a raw 18-dim context vector.
    pub fn observe(&self, context_vec: Vec<f64>, model_idx: usize, reward: f64) {
        self.observe_internal(&context_vec, model_idx, reward, true, None, None);
    }

    /// Record a successful multi-objective observation from a raw context vector.
    pub fn observe_multi_objective(
        &self,
        context_vec: Vec<f64>,
        model_idx: usize,
        quality: f64,
        normalized_cost: f64,
        normalized_latency: f64,
        weights: &RewardWeights,
    ) {
        let Some(slug) = self.model_slugs.get(model_idx) else {
            return;
        };

        let mut stats = self.confidence_stats.lock();
        let entry = stats.entry(slug.clone()).or_default();
        entry.trials += 1;
        entry.successes += 1;
        drop(stats);

        self.linucb.update_features_multi_objective(
            &context_vec,
            model_idx,
            quality,
            normalized_cost,
            normalized_latency,
            weights,
        );
    }

    fn observe_internal(
        &self,
        context_vec: &[f64],
        model_idx: usize,
        reward: f64,
        success: bool,
        perplexity: Option<PerplexityObservationTotals>,
        gemini: Option<GeminiObservationTotals>,
    ) {
        let Some(slug) = self.model_slugs.get(model_idx) else {
            return;
        };

        // Phase 1: Update confidence stats (single lock, dropped before next).
        {
            let mut stats = self.confidence_stats.lock();
            let entry = stats.entry(slug.clone()).or_default();
            entry.trials += 1;
            if success {
                entry.successes += 1;
            }
            if let Some(perplexity) = perplexity {
                entry.total_citations += perplexity.citation_count;
                entry.total_search_latency_ms += perplexity.search_latency_ms;
                entry.total_cost_usd += perplexity.total_cost_usd;
                entry.perplexity_requests += 1;
            }
            if let Some(gemini) = gemini {
                entry.total_gemini_thinking_tokens += gemini.thinking_tokens;
                entry.total_gemini_cached_tokens += gemini.cached_tokens;
                entry.total_gemini_grounding_queries += gemini.grounding_query_count;
                entry.gemini_code_execution_successes += gemini.code_execution_success_count;
                entry.gemini_code_execution_failures += gemini.code_execution_failure_count;
                entry.gemini_requests += 1;
                match gemini.context_tier {
                    GeminiContextTier::UpTo200k => {
                        entry.gemini_context_window_le_200k_requests += 1;
                    }
                    GeminiContextTier::Over200k => {
                        entry.gemini_context_window_gt_200k_requests += 1;
                    }
                }
            }
        } // stats lock dropped

        // Phase 2: Update LinUCB (internal lock, not nested with ours).
        self.linucb.update_features(context_vec, model_idx, reward);

        // Refresh Pareto frontier if the observation count crossed a bucket boundary.
        self.refresh_pareto_frontier_if_needed();

        // Phase 3: Check stage transition (single lock, dropped before log).
        let obs = self.linucb.total_observations();
        let next = stage_for_observations(obs);
        let transition = {
            let mut stage_tracking = self.stage_tracking.lock();
            if next != stage_tracking.current {
                let t = StageTransition {
                    from: stage_tracking.current,
                    to: next,
                    observations: obs,
                    timestamp: Utc::now(),
                };
                stage_tracking.current = next;
                stage_tracking.transitions.push(t.clone());
                Some(t)
            } else {
                None
            }
        }; // stage_tracking lock dropped

        if let Some(transition) = transition {
            tracing::info!(
                from = %transition.from,
                to = %transition.to,
                observations = transition.observations,
                timestamp = %transition.timestamp.to_rfc3339(),
                "cascade router stage transition"
            );
        }
    }

    /// Access the underlying `LinUCB` router (for introspection / persistence).
    pub const fn linucb(&self) -> &LinUCBRouter {
        &self.linucb
    }

    /// Snapshot of confidence-stage statistics.
    pub fn confidence_snapshot(&self) -> HashMap<String, (u64, u64)> {
        self.confidence_stats
            .lock()
            .iter()
            .map(|(k, v)| (k.clone(), (v.trials, v.successes)))
            .collect()
    }

    /// Explain the current routing decision for `ctx`.
    pub fn explain_route(
        &self,
        ctx: &RoutingContext,
        candidates: Option<&[String]>,
    ) -> CascadeRouteExplanation {
        let candidates = candidates
            .filter(|candidates| !candidates.is_empty())
            .unwrap_or(&self.model_slugs);
        let stage = self.current_stage();
        let observations = self.total_observations();
        let pareto_frontier = self.recompute_pareto_frontier();

        match stage {
            CascadeStage::Static => {
                let selected = if std::ptr::eq(candidates, self.model_slugs.as_slice()) {
                    self.route_static(ctx, None, None).primary.slug
                } else {
                    self.route_static_filtered(ctx, candidates).primary.slug
                };
                let mut scored = candidates
                    .iter()
                    .map(|slug| CascadeCandidateScore {
                        slug: slug.clone(),
                        score: if slugs_match(slug, &selected) {
                            1.0
                        } else {
                            0.0
                        },
                        selected: slugs_match(slug, &selected),
                        on_pareto_frontier: pareto_frontier.iter().any(|entry| entry == slug),
                        exploitation: None,
                        exploration: None,
                    })
                    .collect::<Vec<_>>();
                scored.sort_by(|a, b| {
                    b.score
                        .total_cmp(&a.score)
                        .then_with(|| a.slug.cmp(&b.slug))
                });

                CascadeRouteExplanation {
                    stage,
                    observations,
                    alpha: None,
                    selected_slug: selected,
                    candidates: scored,
                    pareto_frontier,
                    knowledge_advice: None,
                }
            }
            CascadeStage::Confidence => {
                let scores = self.confidence_scores(candidates, ctx);
                let selected = scores
                    .iter()
                    .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                    .map(|(slug, _)| slug.clone())
                    .unwrap_or_else(|| candidates[0].clone());
                let mut scored_candidates = scores
                    .into_iter()
                    .map(|(slug, score)| CascadeCandidateScore {
                        selected: slugs_match(&slug, &selected),
                        on_pareto_frontier: pareto_frontier.iter().any(|entry| entry == &slug),
                        slug,
                        score,
                        exploitation: None,
                        exploration: None,
                    })
                    .collect::<Vec<_>>();
                scored_candidates.sort_by(|a, b| {
                    b.score
                        .total_cmp(&a.score)
                        .then_with(|| a.slug.cmp(&b.slug))
                });

                CascadeRouteExplanation {
                    stage,
                    observations,
                    alpha: None,
                    selected_slug: selected,
                    candidates: scored_candidates,
                    pareto_frontier,
                    knowledge_advice: None,
                }
            }
            CascadeStage::Ucb => {
                self.refresh_pareto_frontier_if_needed();
                let frontier = {
                    let frontier_state = self.pareto_frontier.lock();
                    if frontier_state.bucket == 0 || frontier_state.frontier.is_empty() {
                        pareto_frontier.clone()
                    } else {
                        frontier_state.frontier.clone()
                    }
                };
                let base_alpha = self.linucb.current_alpha();
                let arm_scores: Vec<CandidateArmScore> = self
                    .linucb
                    .score_candidates_with_alpha_adjuster(ctx, candidates, |slug| {
                        pareto_adjusted_alpha(base_alpha, slug, &frontier)
                    });

                let selected = arm_scores
                    .iter()
                    .max_by(|left, right| {
                        left.score
                            .total_cmp(&right.score)
                            .then_with(|| right.slug.cmp(&left.slug))
                    })
                    .map(|score| score.slug.clone())
                    .unwrap_or_else(|| candidates[0].clone());

                let mut scored = arm_scores
                    .into_iter()
                    .map(|candidate| CascadeCandidateScore {
                        selected: slugs_match(&candidate.slug, &selected),
                        on_pareto_frontier: frontier.iter().any(|entry| entry == &candidate.slug),
                        slug: candidate.slug,
                        score: candidate.score,
                        exploitation: Some(candidate.exploitation),
                        exploration: Some(candidate.exploration),
                    })
                    .collect::<Vec<_>>();
                scored.sort_by(|a, b| {
                    b.score
                        .total_cmp(&a.score)
                        .then_with(|| a.slug.cmp(&b.slug))
                });

                CascadeRouteExplanation {
                    stage,
                    observations,
                    alpha: Some(base_alpha),
                    selected_slug: selected,
                    candidates: scored,
                    pareto_frontier: frontier,
                    knowledge_advice: None,
                }
            }
        }
    }

    /// Apply knowledge-informed routing advice to a routing explanation.
    pub fn apply_knowledge_advice(
        &self,
        explanation: &mut CascadeRouteExplanation,
        advice: KnowledgeRoutingAdvice,
    ) {
        if !advice.has_signal || advice.hints.is_empty() {
            explanation.knowledge_advice = Some(advice);
            return;
        }
        for candidate in &mut explanation.candidates {
            if let Some(hint) = advice.hint_for(&candidate.slug) {
                let adjustment = hint.score.clamp(-0.20, 0.20);
                candidate.score += adjustment;
            }
        }
        explanation.candidates.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.slug.cmp(&b.slug))
        });
        if let Some(new_top) = explanation.candidates.first() {
            if !slugs_match(&new_top.slug, &explanation.selected_slug) {
                tracing::info!(
                    previous = %explanation.selected_slug,
                    new = %new_top.slug,
                    "knowledge advice changed routing selection"
                );
            }
            let new_top_slug = new_top.slug.clone();
            for c in &mut explanation.candidates {
                c.selected = slugs_match(&c.slug, &new_top_slug);
            }
            explanation.selected_slug = new_top_slug;
        }
        explanation.knowledge_advice = Some(advice);
    }

    /// Append a routing decision log entry for a selected model.
    pub fn append_routing_log(
        &self,
        log: &RoutingLogger,
        meta: &RoutingDecisionMeta,
        selected_model: &str,
        explanation: Option<&CascadeRouteExplanation>,
    ) -> std::io::Result<RoutingDecisionLog> {
        let mut candidates = explanation
            .map(|explanation| {
                explanation
                    .candidates
                    .iter()
                    .map(|candidate| CandidateEntry {
                        model: candidate.slug.clone(),
                        provider: log.provider_for_model(&candidate.slug),
                        score: candidate.score,
                        disqualified: log.disqualified_reason(&candidate.slug),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !candidates
            .iter()
            .any(|candidate| candidate.model == selected_model)
        {
            candidates.insert(
                0,
                CandidateEntry {
                    model: selected_model.to_string(),
                    provider: log.provider_for_model(selected_model),
                    score: 1.0,
                    disqualified: log.disqualified_reason(selected_model),
                },
            );
        }
        if candidates.is_empty() {
            candidates.push(CandidateEntry {
                model: selected_model.to_string(),
                provider: log.provider_for_model(selected_model),
                score: 1.0,
                disqualified: log.disqualified_reason(selected_model),
            });
        }

        let record = RoutingDecisionLog {
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: meta.trace_id.clone(),
            task_id: meta.task_id.clone(),
            requested_model: meta.requested_model.clone(),
            role: meta.role.clone(),
            task_complexity: meta.task_complexity.clone(),
            task_category: meta.task_category.clone(),
            selected_provider: log.provider_for_model(selected_model),
            selected_model: selected_model.to_string(),
            routing_stage: meta.routing_stage.clone(),
            routing_reason: meta.routing_reason.clone(),
            candidates,
            outcome_success: None,
            outcome_cost_usd: None,
            outcome_latency_ms: None,
        };
        log.append(&record)?;
        Ok(record)
    }

    /// Snapshot of richer per-model observations used by learning loops.
    pub fn observation_snapshot(&self) -> HashMap<String, CascadeObservationStats> {
        self.confidence_stats
            .lock()
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    CascadeObservationStats {
                        trials: v.trials,
                        successes: v.successes,
                        total_citations: v.total_citations,
                        avg_citations_per_response: v.avg_citations_per_response(),
                        total_search_latency_ms: v.total_search_latency_ms,
                        avg_search_latency_ms: v.avg_search_latency_ms(),
                        total_cost_usd: v.total_cost_usd,
                        avg_cost_usd: v.avg_cost_usd(),
                        perplexity_requests: v.perplexity_requests,
                        total_gemini_thinking_tokens: v.total_gemini_thinking_tokens,
                        avg_gemini_thinking_tokens_per_response: v
                            .avg_gemini_thinking_tokens_per_response(),
                        total_gemini_cached_tokens: v.total_gemini_cached_tokens,
                        avg_gemini_cached_tokens_per_response: v
                            .avg_gemini_cached_tokens_per_response(),
                        total_gemini_grounding_queries: v.total_gemini_grounding_queries,
                        avg_gemini_grounding_queries_per_response: v
                            .avg_gemini_grounding_queries_per_response(),
                        gemini_code_execution_successes: v.gemini_code_execution_successes,
                        gemini_code_execution_failures: v.gemini_code_execution_failures,
                        gemini_code_execution_success_rate: v.gemini_code_execution_success_rate(),
                        gemini_requests: v.gemini_requests,
                        gemini_context_window_le_200k_requests: v
                            .gemini_context_window_le_200k_requests,
                        gemini_context_window_gt_200k_requests: v
                            .gemini_context_window_gt_200k_requests,
                    },
                )
            })
            .collect()
    }

    /// Build a JSON snapshot of the current router state (same format as `save()`).
    pub fn snapshot_json(&self) -> String {
        let stage_transitions = self.stage_tracking.lock().transitions.clone();
        let snapshot = CascadeSnapshot {
            model_slugs: self.model_slugs.clone(),
            role_table: self.role_table.lock().clone(),
            confidence_stats: self
                .confidence_stats
                .lock()
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        PersistedModelStats {
                            trials: v.trials,
                            successes: v.successes,
                            total_citations: v.total_citations,
                            total_search_latency_ms: v.total_search_latency_ms,
                            total_cost_usd: v.total_cost_usd,
                            perplexity_requests: v.perplexity_requests,
                            total_gemini_thinking_tokens: v.total_gemini_thinking_tokens,
                            total_gemini_cached_tokens: v.total_gemini_cached_tokens,
                            total_gemini_grounding_queries: v.total_gemini_grounding_queries,
                            gemini_code_execution_successes: v.gemini_code_execution_successes,
                            gemini_code_execution_failures: v.gemini_code_execution_failures,
                            gemini_context_window_le_200k_requests: v
                                .gemini_context_window_le_200k_requests,
                            gemini_context_window_gt_200k_requests: v
                                .gemini_context_window_gt_200k_requests,
                            gemini_requests: v.gemini_requests,
                        },
                    )
                })
                .collect(),
            total_observations: self.linucb.total_observations(),
            stage_transitions,
            linucb_state: Some(self.linucb.export_linucb_snapshot()),
        };
        tracing::debug!(
            total_observations = snapshot.total_observations,
            linucb_persisted = snapshot.linucb_state.is_some(),
            "cascade router snapshot built"
        );
        serde_json::to_string_pretty(&snapshot).unwrap_or_default()
    }

    /// Save confidence stats, model slugs, and total observation count to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), crate::error::LearnError> {
        let json = self.snapshot_json();
        if json.is_empty() {
            return Err(crate::error::LearnError::Io {
                path: path.display().to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "failed to serialize cascade snapshot",
                ),
            });
        }
        roko_core::io::atomic_write_str(path, &json).map_err(|source| {
            crate::error::LearnError::Io {
                path: path.display().to_string(),
                source,
            }
        })
    }

    fn from_snapshot(snapshot: CascadeSnapshot, model_slugs: Vec<String>) -> Self {
        let CascadeSnapshot {
            model_slugs: persisted_model_slugs,
            confidence_stats,
            total_observations,
            role_table,
            stage_transitions,
            linucb_state,
        } = snapshot;

        let slugs = if model_slugs.is_empty() {
            persisted_model_slugs.clone()
        } else {
            model_slugs
        };
        assert!(!slugs.is_empty(), "CascadeRouter: need at least one model");

        let version_changes = detect_version_changes(&persisted_model_slugs, &slugs);
        let migrated_stats = migrated_confidence_stats(&confidence_stats, &version_changes, &slugs);
        let router = Self::new(slugs);

        // Restore confidence stats.
        let mut stats = router.confidence_stats.lock();
        for (model, persisted) in &migrated_stats {
            stats.insert(
                model.clone(),
                ModelStats {
                    trials: persisted.trials,
                    successes: persisted.successes,
                    total_citations: persisted.total_citations,
                    total_search_latency_ms: persisted.total_search_latency_ms,
                    total_cost_usd: persisted.total_cost_usd,
                    perplexity_requests: persisted.perplexity_requests,
                    total_gemini_thinking_tokens: persisted.total_gemini_thinking_tokens,
                    total_gemini_cached_tokens: persisted.total_gemini_cached_tokens,
                    total_gemini_grounding_queries: persisted.total_gemini_grounding_queries,
                    gemini_code_execution_successes: persisted.gemini_code_execution_successes,
                    gemini_code_execution_failures: persisted.gemini_code_execution_failures,
                    gemini_context_window_le_200k_requests: persisted
                        .gemini_context_window_le_200k_requests,
                    gemini_context_window_gt_200k_requests: persisted
                        .gemini_context_window_gt_200k_requests,
                    gemini_requests: persisted.gemini_requests,
                },
            );
        }
        drop(stats);

        let total = if total_observations > 0 {
            total_observations
        } else {
            confidence_stats.values().map(|s| s.trials).sum()
        };
        router.linucb.set_total_observations(total);
        if let Some(state) = linucb_state {
            router.linucb.import_linucb_snapshot(&state);
        }
        if !role_table.is_empty() {
            let mut rt = router.role_table.lock();
            for (role, slug) in role_table {
                rt.insert(role, remap_role_table_entry(slug, &version_changes));
            }
        }
        {
            let mut stage_tracking = router.stage_tracking.lock();
            stage_tracking.current = stage_for_observations(total);
            stage_tracking.transitions = stage_transitions;
        }

        router
    }

    /// Build a router from an in-memory snapshot JSON string.
    pub fn from_snapshot_json(
        json: &str,
        model_slugs: Vec<String>,
    ) -> Result<Self, crate::error::LearnError> {
        let snapshot: CascadeSnapshot = serde_json::from_str(json)?;
        Ok(Self::from_snapshot(snapshot, model_slugs))
    }

    /// Load a cascade router from a persisted JSON file, or create a new one.
    ///
    /// When the file exists but cannot be parsed, the corrupted file is
    /// backed up to `<path>.corrupted` and a warning is emitted before
    /// creating a fresh router.
    pub fn load_or_new(path: &Path, model_slugs: Vec<String>) -> Self {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Self::new(model_slugs),
        };

        match serde_json::from_str::<CascadeSnapshot>(&raw) {
            Ok(snapshot) => Self::from_snapshot(snapshot, model_slugs),
            Err(err) => {
                let backup = path.with_extension("json.corrupted");
                tracing::warn!(
                    path = %path.display(),
                    backup = %backup.display(),
                    %err,
                    "cascade router state corrupted — backing up and resetting"
                );
                if let Err(backup_err) = std::fs::copy(path, &backup) {
                    tracing::error!(
                        %backup_err,
                        "failed to backup corrupted cascade router file"
                    );
                }
                Self::new(model_slugs)
            }
        }
    }

    // ── Test-visible accessors for Pareto frontier ──────────────────

    /// Return the current Pareto frontier bucket (for tests).
    #[cfg(test)]
    pub(crate) fn pareto_frontier_bucket(&self) -> u64 {
        self.pareto_frontier.lock().bucket
    }

    /// Return the current Pareto frontier slugs (for tests).
    #[cfg(test)]
    pub(crate) fn pareto_frontier_slugs(&self) -> Vec<String> {
        self.pareto_frontier.lock().frontier.clone()
    }

    // ── Internal routing per stage ──────────────────────────────────────

    fn route_static(
        &self,
        ctx: &RoutingContext,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        if let Some(thinking_selected) = match thinking_preference(ctx) {
            ThinkingPreference::PreferThinking => pick_tier_extreme(
                &thinking_filtered_candidates(&self.model_slugs, ctx),
                true,
                &self.tier_map,
            ),
            ThinkingPreference::PreferNonThinking => pick_tier_extreme(
                &thinking_filtered_candidates(&self.model_slugs, ctx),
                false,
                &self.tier_map,
            ),
            ThinkingPreference::Neutral => None,
        } {
            let tier = slug_to_tier(&thinking_selected, &self.tier_map);
            let route = CascadeModel {
                primary: ModelSpec::from_slug(thinking_selected),
                fallback_chain: Vec::new(),
                context_overflow_fallback: None,
                latency_sla_ms: default_latency_sla(tier),
                stage: CascadeStage::Static,
            };
            return self.apply_context_biases_among(
                route,
                ctx,
                &self.model_slugs,
                cfactor,
                agent_id,
            );
        }

        let default_slug = self
            .model_slugs
            .first()
            .cloned()
            .unwrap_or_else(|| "claude-sonnet-4-5".to_string());
        let slug = if ctx.task_category == TaskCategory::Research {
            self.model_slugs
                .iter()
                .find(|s| slug_family(s) == Some("sonar"))
                .cloned()
                .unwrap_or_else(|| {
                    self.role_table
                        .lock()
                        .get(&ctx.role)
                        .cloned()
                        .unwrap_or_else(|| default_slug.clone())
                })
        } else {
            self.role_table
                .lock()
                .get(&ctx.role)
                .cloned()
                .unwrap_or(default_slug)
        };
        let tier = slug_to_tier(&slug, &self.tier_map);

        let route = CascadeModel {
            primary: ModelSpec::from_slug(&slug),
            fallback_chain: fallback_chain_for_model(&self.model_slugs, &slug, &self.tier_map),
            context_overflow_fallback: context_overflow_fallback_for_model(
                &self.model_slugs,
                &slug,
                &self.tier_map,
            ),
            latency_sla_ms: default_latency_sla(tier),
            stage: CascadeStage::Static,
        };
        self.apply_context_biases_among(route, ctx, &self.model_slugs, cfactor, agent_id)
    }

    fn route_static_filtered(&self, ctx: &RoutingContext, candidates: &[String]) -> CascadeModel {
        if let Some(thinking_selected) = match thinking_preference(ctx) {
            ThinkingPreference::PreferThinking => pick_tier_extreme(
                &thinking_filtered_candidates(candidates, ctx),
                true,
                &self.tier_map,
            ),
            ThinkingPreference::PreferNonThinking => pick_tier_extreme(
                &thinking_filtered_candidates(candidates, ctx),
                false,
                &self.tier_map,
            ),
            ThinkingPreference::Neutral => None,
        } {
            let selected = ModelSpec::from_slug(thinking_selected);
            let tier = slug_to_tier(&selected.slug, &self.tier_map);

            return CascadeModel {
                primary: selected,
                fallback_chain: Vec::new(),
                context_overflow_fallback: None,
                latency_sla_ms: default_latency_sla(tier),
                stage: CascadeStage::Static,
            };
        }

        let slug = self
            .role_table
            .lock()
            .get(&ctx.role)
            .cloned()
            .unwrap_or_else(|| "claude-sonnet-4-5".to_string());

        let selected_slug = if candidates
            .iter()
            .any(|candidate| slugs_match(candidate, &slug))
        {
            slug
        } else {
            let tier_candidates: &[&str] = match ctx.role.model_tier() {
                ModelTier::Fast => &["gemini-2.5-flash-lite", "claude-haiku-4-5"],
                ModelTier::Premium => &[
                    "claude-opus-4-6",
                    "gemini-3.1-pro-preview",
                    "gemini-2.5-pro",
                ],
                _ => &[
                    "gemini-2.5-flash",
                    "gemini-2.5-pro",
                    "kimi-k2.5",
                    "kimi-k2-thinking",
                    "claude-sonnet-4-6",
                    "claude-sonnet-4-5",
                ],
            };
            pick_available_static_slug(candidates, tier_candidates)
        };
        let selected = ModelSpec::from_slug(selected_slug);
        let tier = slug_to_tier(&selected.slug, &self.tier_map);
        let fallback_chain = fallback_chain_for_model(candidates, &selected.slug, &self.tier_map);
        let context_overflow_fallback =
            context_overflow_fallback_for_model(candidates, &selected.slug, &self.tier_map);

        CascadeModel {
            primary: selected,
            fallback_chain,
            context_overflow_fallback,
            latency_sla_ms: default_latency_sla(tier),
            stage: CascadeStage::Static,
        }
    }

    fn route_static_among(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        let route = self.route_static_filtered(ctx, candidates);
        self.apply_context_biases_among(route, ctx, candidates, cfactor, agent_id)
    }

    fn route_confidence(
        &self,
        ctx: &RoutingContext,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        let thinking_candidates = thinking_filtered_candidates(&self.model_slugs, ctx);
        let scores = self.confidence_scores(&thinking_candidates, ctx);
        let best_slug = select_with_hysteresis(&scores, ctx.previous_model.as_deref());
        let tier = slug_to_tier(&best_slug, &self.tier_map);

        let route = CascadeModel {
            primary: ModelSpec::from_slug(&best_slug),
            fallback_chain: fallback_chain_for_model(&self.model_slugs, &best_slug, &self.tier_map),
            context_overflow_fallback: context_overflow_fallback_for_model(
                &self.model_slugs,
                &best_slug,
                &self.tier_map,
            ),
            latency_sla_ms: default_latency_sla(tier),
            stage: CascadeStage::Confidence,
        };
        self.apply_context_biases_among(route, ctx, &self.model_slugs, cfactor, agent_id)
    }

    fn route_confidence_filtered(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
    ) -> CascadeModel {
        let thinking_candidates = thinking_filtered_candidates(candidates, ctx);
        let scores = self.confidence_scores(&thinking_candidates, ctx);
        let best_slug = select_with_hysteresis(&scores, ctx.previous_model.as_deref());

        let selected = ModelSpec::from_slug(best_slug);
        let tier = slug_to_tier(&selected.slug, &self.tier_map);
        let fallback_chain = fallback_chain_for_model(candidates, &selected.slug, &self.tier_map);
        let context_overflow_fallback =
            context_overflow_fallback_for_model(candidates, &selected.slug, &self.tier_map);

        CascadeModel {
            primary: selected,
            fallback_chain,
            context_overflow_fallback,
            latency_sla_ms: default_latency_sla(tier),
            stage: CascadeStage::Confidence,
        }
    }

    fn route_confidence_among(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        let route = self.route_confidence_filtered(ctx, candidates);
        self.apply_context_biases_among(route, ctx, candidates, cfactor, agent_id)
    }

    fn route_ucb(
        &self,
        ctx: &RoutingContext,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        let thinking_candidates = thinking_filtered_candidates(&self.model_slugs, ctx);
        let model = self.select_ucb_model(ctx, &thinking_candidates);
        let tier = slug_to_tier(&model.slug, &self.tier_map);
        let route = CascadeModel {
            primary: model.clone(),
            fallback_chain: fallback_chain_for_model(
                &self.model_slugs,
                &model.slug,
                &self.tier_map,
            ),
            context_overflow_fallback: context_overflow_fallback_for_model(
                &self.model_slugs,
                &model.slug,
                &self.tier_map,
            ),
            latency_sla_ms: default_latency_sla(tier),
            stage: CascadeStage::Ucb,
        };
        self.apply_context_biases_among(route, ctx, &self.model_slugs, cfactor, agent_id)
    }

    fn route_ucb_filtered(&self, ctx: &RoutingContext, candidates: &[String]) -> CascadeModel {
        let thinking_candidates = thinking_filtered_candidates(candidates, ctx);
        let model = self.select_ucb_model(ctx, &thinking_candidates);
        let selected = model;
        let tier = slug_to_tier(&selected.slug, &self.tier_map);
        let fallback_chain = fallback_chain_for_model(candidates, &selected.slug, &self.tier_map);
        let context_overflow_fallback =
            context_overflow_fallback_for_model(candidates, &selected.slug, &self.tier_map);

        CascadeModel {
            primary: selected,
            fallback_chain,
            context_overflow_fallback,
            latency_sla_ms: default_latency_sla(tier),
            stage: CascadeStage::Ucb,
        }
    }

    fn route_ucb_among(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> CascadeModel {
        let route = self.route_ucb_filtered(ctx, candidates);
        self.apply_context_biases_among(route, ctx, candidates, cfactor, agent_id)
    }

    fn bias_model_for_cfactor(
        &self,
        model: ModelSpec,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
    ) -> ModelSpec {
        let Some(cfactor) = cfactor else {
            return model;
        };

        if let Some(agent_id) = agent_id {
            match cfactor.dispatch_bias_for_agent(agent_id) {
                AgentDispatchBias::PreferStronger => return self.strongest_model(),
                AgentDispatchBias::PreferCheaper => return self.cheapest_model(),
                AgentDispatchBias::Neutral => {}
            }
        }

        if cfactor.overall > HIGH_CFACTOR_THRESHOLD {
            self.cheapest_model()
        } else if cfactor.overall < LOW_CFACTOR_THRESHOLD {
            self.strongest_model()
        } else {
            model
        }
    }

    fn bias_model_for_cfactor_among(
        &self,
        model: ModelSpec,
        cfactor: Option<&CFactor>,
        agent_id: Option<&str>,
        candidates: &[String],
    ) -> ModelSpec {
        let Some(cfactor) = cfactor else {
            return model;
        };

        if let Some(agent_id) = agent_id {
            match cfactor.dispatch_bias_for_agent(agent_id) {
                AgentDispatchBias::PreferStronger => return self.strongest_model_among(candidates),
                AgentDispatchBias::PreferCheaper => return self.cheapest_model_among(candidates),
                AgentDispatchBias::Neutral => {}
            }
        }

        if cfactor.overall > HIGH_CFACTOR_THRESHOLD {
            self.cheapest_model_among(candidates)
        } else if cfactor.overall < LOW_CFACTOR_THRESHOLD {
            self.strongest_model_among(candidates)
        } else {
            model
        }
    }

    fn confidence_scores(&self, candidates: &[String], ctx: &RoutingContext) -> Vec<(String, f64)> {
        let stats = self.confidence_stats.lock();

        let prediction_error = 1.0 - ctx.daimon_policy.affect_confidence.clamp(0.0, 1.0);
        let low_confidence = ctx
            .tier_thresholds
            .as_ref()
            .map(|th| prediction_error > th.t0_ceiling)
            .unwrap_or(ctx.daimon_policy.affect_confidence < LOW_AFFECT_CONFIDENCE_THRESHOLD);

        // Pre-compute global pass rates while holding the lock so we can
        // derive per-category deltas below without re-locking.
        let global_rates: HashMap<&str, f64> = candidates
            .iter()
            .filter_map(|slug| stats.get(slug).map(|s| (slug.as_str(), s.pass_rate())))
            .collect();

        let mut scores: Vec<(String, f64)> = candidates
            .iter()
            .map(|slug| {
                let s = stats.get(slug).cloned().unwrap_or_default();
                // Audit #80: cold-start penalty — untried models get 0.2 instead
                // of the neutral 0.5 so they are not preferred over models with
                // proven observations.  Once a model records its first trial the
                // empirical upper-bound takes over.
                let base_score = if s.trials == 0 { 0.2 } else { s.upper_bound() };
                let tier_bonus = if low_confidence {
                    low_confidence_tier_bonus(slug_to_tier(slug, &self.tier_map))
                } else {
                    0.0
                };
                (slug.clone(), base_score + tier_bonus)
            })
            .collect();
        drop(stats);

        // Audit #84: apply category-aware adjustment.  If the task
        // carries a category and we have enough per-category data,
        // nudge the score toward models that historically do well on
        // this category (and away from those that do poorly).
        for (slug, score) in &mut scores {
            let global_pr = global_rates.get(slug.as_str()).copied().unwrap_or(0.5);
            let delta = self.category_pass_rate_delta(slug, ctx.task_category, global_pr);
            *score += CATEGORY_CONFIDENCE_WEIGHT * delta;
        }

        apply_cache_affinity(&mut scores, ctx.previous_model.as_deref());
        scores
    }

    fn select_ucb_model(&self, ctx: &RoutingContext, candidates: &[String]) -> ModelSpec {
        let frontier = self.current_pareto_frontier();
        let scores = self.ucb_scores(ctx, candidates, frontier.as_deref());
        let best_slug = select_with_hysteresis(&scores, ctx.previous_model.as_deref());
        ModelSpec::from_slug(best_slug)
    }

    fn refresh_pareto_frontier_if_needed(&self) {
        let total = self.total_observations();
        if total < PARETO_RECOMPUTE_INTERVAL {
            return;
        }

        let bucket = total / PARETO_RECOMPUTE_INTERVAL;
        let needs_refresh = {
            let state = self.pareto_frontier.lock();
            state.bucket < bucket || state.frontier.is_empty()
        };

        if !needs_refresh {
            return;
        }

        let frontier = self.recompute_pareto_frontier();
        let mut state = self.pareto_frontier.lock();
        if state.bucket < bucket || state.frontier.is_empty() {
            state.frontier = frontier;
            state.bucket = bucket;
        }
    }

    fn current_pareto_frontier(&self) -> Option<Vec<String>> {
        self.refresh_pareto_frontier_if_needed();
        let state = self.pareto_frontier.lock();
        if state.bucket == 0 || state.frontier.is_empty() {
            None
        } else {
            Some(state.frontier.clone())
        }
    }

    fn stage_scores(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
        frontier: Option<&[String]>,
    ) -> Vec<(String, f64)> {
        match self.current_stage() {
            CascadeStage::Static => {
                let selected = self.route_static_filtered(ctx, candidates).primary.slug;
                candidates
                    .iter()
                    .map(|slug| {
                        (
                            slug.clone(),
                            if slugs_match(slug, &selected) {
                                1.0
                            } else {
                                0.0
                            },
                        )
                    })
                    .collect()
            }
            CascadeStage::Confidence => {
                let thinking_candidates = thinking_filtered_candidates(candidates, ctx);
                let score_map: HashMap<_, _> = self
                    .confidence_scores(&thinking_candidates, ctx)
                    .into_iter()
                    .collect();
                candidates
                    .iter()
                    .map(|slug| (slug.clone(), score_map.get(slug).copied().unwrap_or(0.0)))
                    .collect()
            }
            CascadeStage::Ucb => {
                let thinking_candidates = thinking_filtered_candidates(candidates, ctx);
                let score_map: HashMap<_, _> = self
                    .ucb_scores(ctx, &thinking_candidates, frontier)
                    .into_iter()
                    .collect();
                candidates
                    .iter()
                    .map(|slug| (slug.clone(), score_map.get(slug).copied().unwrap_or(0.0)))
                    .collect()
            }
        }
    }

    fn ucb_scores(
        &self,
        ctx: &RoutingContext,
        candidates: &[String],
        frontier: Option<&[String]>,
    ) -> Vec<(String, f64)> {
        let base_alpha = self.linucb.current_alpha() * temperament_exploration_multiplier(ctx);

        self.linucb
            .score_features_from_candidates_with_alpha_adjuster(ctx, candidates, |slug| {
                frontier.map_or(base_alpha, |frontier| {
                    pareto_adjusted_alpha(base_alpha, slug, frontier)
                })
            })
    }

    fn recompute_pareto_frontier(&self) -> Vec<String> {
        let stats = self.confidence_stats.lock();
        let mut observations = HashMap::new();
        let mut unobserved = Vec::new();

        for slug in &self.model_slugs {
            match stats.get(slug) {
                Some(model_stats) if model_stats.trials > 0 => {
                    observations.insert(
                        slug.clone(),
                        ModelObservation {
                            pass_rate: model_stats.pass_rate(),
                            cost_per_success: model_stats.cost_per_success().unwrap_or_else(|| {
                                pareto_cost_proxy(slug, &self.tier_map)
                                    / model_stats.pass_rate().max(0.01)
                            }),
                            avg_latency_ms: if model_stats.perplexity_requests > 0 {
                                model_stats.avg_search_latency_ms()
                            } else {
                                pareto_latency_proxy(slug, &self.tier_map)
                            },
                            reliability: if model_stats.trials > 0 {
                                model_stats.pass_rate().max(0.5)
                            } else {
                                0.5
                            },
                            observations: model_stats.trials,
                        },
                    );
                }
                _ => unobserved.push(slug.clone()),
            }
        }
        drop(stats);

        let mut frontier = if observations.is_empty() {
            Vec::new()
        } else {
            compute_pareto_frontier(&observations)
        };
        frontier.extend(unobserved);
        frontier.sort();
        frontier.dedup();
        frontier
    }

    /// Feed prediction residuals back into the router after task completion.
    pub fn feedback(
        &self,
        model_slug: &str,
        _predicted_success: f64,
        actual_success: bool,
        residual: f64,
    ) {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            return;
        };

        let calibration_bonus = 1.0 - residual.abs().min(1.0);
        let base_reward = if actual_success { 1.0 } else { 0.0 };
        let adjusted_reward = (base_reward * 0.7 + calibration_bonus * 0.3).clamp(0.0, 1.0);

        let context_vec = vec![0.0; CONTEXT_DIM];
        self.observe_internal(
            &context_vec,
            model_idx,
            adjusted_reward,
            actual_success,
            None,
            None,
        );

        self.check_stage_transition();
    }

    /// Feed prediction residuals from a calibration tracker summary.
    pub fn feedback_from_prediction(
        &self,
        model_slug: &str,
        predicted_success: f64,
        actual_success: bool,
    ) {
        let actual_value = if actual_success { 1.0 } else { 0.0 };
        let residual = predicted_success - actual_value;
        self.feedback(model_slug, predicted_success, actual_success, residual);
    }

    /// Route a context, applying a provider pass-rate multiplier to model scores
    /// in stages 2 (Confidence) and 3 (UCB).
    ///
    /// `provider_metrics` maps provider id → [`ProviderMetrics`].
    /// `model_providers` maps model slug → provider id.
    /// `pass_rate_weight` controls how strongly the pass rate influences the
    /// score: 0.0 disables the bias, 1.0 applies the full pass-rate multiplier.
    ///
    /// This is a soft signal — providers with low pass rates are down-weighted
    /// but never completely excluded.
    pub fn route_with_provider_metrics(
        &self,
        ctx: &RoutingContext,
        provider_metrics: &HashMap<String, ProviderMetrics>,
        model_providers: &HashMap<String, String>,
        pass_rate_weight: f64,
    ) -> CascadeModel {
        match self.current_stage() {
            CascadeStage::Static => self.route_static(ctx, None, None),
            CascadeStage::Confidence => {
                let thinking_candidates = thinking_filtered_candidates(&self.model_slugs, ctx);
                let mut scores = self.confidence_scores(&thinking_candidates, ctx);
                apply_provider_pass_rate(
                    &mut scores,
                    provider_metrics,
                    model_providers,
                    pass_rate_weight,
                );
                let best_slug = select_with_hysteresis(&scores, ctx.previous_model.as_deref());
                let tier = slug_to_tier(&best_slug, &self.tier_map);
                CascadeModel {
                    primary: ModelSpec::from_slug(&best_slug),
                    fallback_chain: fallback_chain_for_model(
                        &self.model_slugs,
                        &best_slug,
                        &self.tier_map,
                    ),
                    context_overflow_fallback: context_overflow_fallback_for_model(
                        &self.model_slugs,
                        &best_slug,
                        &self.tier_map,
                    ),
                    latency_sla_ms: default_latency_sla(tier),
                    stage: CascadeStage::Confidence,
                }
            }
            CascadeStage::Ucb => {
                let thinking_candidates = thinking_filtered_candidates(&self.model_slugs, ctx);
                let frontier = self.current_pareto_frontier();
                let mut scores = self.ucb_scores(ctx, &thinking_candidates, frontier.as_deref());
                apply_provider_pass_rate(
                    &mut scores,
                    provider_metrics,
                    model_providers,
                    pass_rate_weight,
                );
                let best_slug = select_with_hysteresis(&scores, ctx.previous_model.as_deref());
                let tier = slug_to_tier(&best_slug, &self.tier_map);
                CascadeModel {
                    primary: ModelSpec::from_slug(&best_slug),
                    fallback_chain: fallback_chain_for_model(
                        &self.model_slugs,
                        &best_slug,
                        &self.tier_map,
                    ),
                    context_overflow_fallback: context_overflow_fallback_for_model(
                        &self.model_slugs,
                        &best_slug,
                        &self.tier_map,
                    ),
                    latency_sla_ms: default_latency_sla(tier),
                    stage: CascadeStage::Ucb,
                }
            }
        }
    }
}

// ─── Provider pass-rate metrics ──────────────────────────────────────────────

/// Per-provider historical pass-rate metrics derived from lifetime request/failure counts.
///
/// Used by [`CascadeRouter::route_with_provider_metrics`] to soft-bias model scoring
/// toward providers with better historical reliability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMetrics {
    /// Total requests recorded for this provider.
    pub total_calls: u64,
    /// Requests that completed successfully (total_calls - failures).
    pub successful_calls: u64,
}

impl ProviderMetrics {
    /// Historical pass rate in `[0.0, 1.0]`.
    ///
    /// Returns `1.0` when no calls have been recorded so that unknown providers
    /// are treated optimistically rather than penalised.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 1.0;
        }
        (self.successful_calls as f64 / self.total_calls as f64).clamp(0.0, 1.0)
    }
}

/// Compute per-provider pass-rate metrics from a [`ProviderHealthRegistry`] snapshot.
///
/// Providers with fewer than `min_calls` requests are excluded from the returned
/// map so that routing is not biased by statistically unreliable samples.
///
/// # Example
///
/// ```rust,ignore
/// let metrics = compute_provider_metrics(&health_registry, 5);
/// let route = router.route_with_provider_metrics(ctx, &metrics, &model_providers, 0.3);
/// ```
#[must_use]
pub fn compute_provider_metrics(
    health: &ProviderHealthRegistry,
    min_calls: u64,
) -> HashMap<String, ProviderMetrics> {
    health
        .snapshot()
        .into_iter()
        .filter_map(|(provider_id, h)| {
            if h.total_requests < min_calls {
                return None;
            }
            let successful_calls = h.total_requests.saturating_sub(h.total_failures);
            Some((
                provider_id,
                ProviderMetrics {
                    total_calls: h.total_requests,
                    successful_calls,
                },
            ))
        })
        .collect()
}

/// Apply provider pass-rate as a soft multiplier to a score vector in-place.
///
/// For each `(slug, score)` pair the effective score becomes:
///
/// ```text
/// adjusted = score * lerp(1.0, pass_rate, weight)
///          = score * (1.0 - weight + weight * pass_rate)
/// ```
///
/// When `weight == 0.0` the scores are unchanged. When `weight == 1.0` the
/// scores are multiplied directly by the provider's pass rate.
///
/// Slugs whose provider is not present in `provider_metrics` (i.e. insufficient
/// data) are left unchanged.
fn apply_provider_pass_rate(
    scores: &mut [(String, f64)],
    provider_metrics: &HashMap<String, ProviderMetrics>,
    model_providers: &HashMap<String, String>,
    weight: f64,
) {
    if weight <= 0.0 || provider_metrics.is_empty() {
        return;
    }
    let weight = weight.clamp(0.0, 1.0);
    for (slug, score) in scores.iter_mut() {
        let provider_id = model_providers
            .get(slug.as_str())
            .map_or(slug.as_str(), String::as_str);
        if let Some(metrics) = provider_metrics.get(provider_id) {
            let pass_rate = metrics.pass_rate();
            // lerp(1.0, pass_rate, weight): no change when weight==0, full
            // multiplier when weight==1.
            let multiplier = 1.0 - weight + weight * pass_rate;
            *score *= multiplier;
        }
    }
}

#[cfg(test)]
mod cascade_router_tests {
    use super::*;

    #[test]
    fn replay_observation_increments_confidence_stats() {
        let router =
            CascadeRouter::new(vec!["claude-sonnet-4-5".into(), "claude-haiku-4-5".into()]);
        let context_features = vec![0.0; crate::model_router::CONTEXT_DIM];

        // Replay 10 observations for the first model.
        for _ in 0..10 {
            router.replay_observation("claude-sonnet-4-5", &context_features, 0, 0.85, true);
        }

        let stats = router.confidence_stats.lock();
        let entry = stats.get("claude-sonnet-4-5").expect("stats should exist");
        assert_eq!(entry.trials, 10);
        assert_eq!(entry.successes, 10);
    }

    #[test]
    fn replay_observation_skips_unknown_slug() {
        let router = CascadeRouter::new(vec!["claude-sonnet-4-5".into()]);
        let context_features = vec![0.0; crate::model_router::CONTEXT_DIM];

        // This should log a warning and not panic.
        router.replay_observation("nonexistent-model", &context_features, 0, 1.0, true);

        let stats = router.confidence_stats.lock();
        assert!(stats.get("nonexistent-model").is_none());
    }

    #[test]
    fn replay_observation_uses_current_index_on_mismatch() {
        let router =
            CascadeRouter::new(vec!["claude-haiku-4-5".into(), "claude-sonnet-4-5".into()]);
        let context_features = vec![0.0; crate::model_router::CONTEXT_DIM];

        // WAL says model_idx=0 but the slug maps to current index 1.
        // replay_observation should use the current index (1) not the stale WAL index.
        router.replay_observation(
            "claude-sonnet-4-5",
            &context_features,
            0, // stale WAL index
            0.9,
            true,
        );

        let stats = router.confidence_stats.lock();
        let entry = stats.get("claude-sonnet-4-5").expect("stats should exist");
        assert_eq!(entry.trials, 1);
        assert_eq!(entry.successes, 1);
    }

    // ── Cross-restart LinUCB persistence ────────────────────────────────
    //
    // Proves that LinUCB A/b matrix weights survive a save → load cycle.
    // After training past CONFIDENCE_TO_UCB_THRESHOLD (200 obs) the router
    // enters UCB stage and the A matrices are no longer identity.  Saving
    // and then loading must restore those weights so that:
    //   (a) the reloaded router is still in UCB stage, and
    //   (b) a representative arm's A[0][0] differs from the identity value 1.0,
    //   (c) routing the same context vector yields the same primary slug.

    #[test]
    fn linucb_persists_across_restart() {
        use crate::cascade::types::{CONFIDENCE_TO_UCB_THRESHOLD, CascadeStage};
        use tempfile::tempdir;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("cascade-router.json");

        let slugs = vec![
            "claude-sonnet-4-5".to_string(),
            "claude-haiku-4-5".to_string(),
            "claude-opus-4-6".to_string(),
        ];

        // ── Phase 1: train a fresh router past the UCB threshold ────────
        let router = CascadeRouter::new(slugs.clone());
        // Construct a context vector that mirrors a real RoutingContext so that
        // A matrix elements actually change during training.
        // Layout (CONTEXT_DIM=18):
        //   [0..8]  = task category one-hot (Implementation → x[1]=1)
        //   [8]     = complexity scalar (Standard → 0.5)
        //   [9]     = iteration (0 → 0.0)
        //   [10..14]= role hash (4 floats; use 0.25 each)
        //   [14]    = crate familiarity (0.5)
        //   [15]    = has_prior_failure (0.0)
        //   [16]    = bias (1.0)
        //   [17]    = cache affinity (0.0)
        let ctx_vec: Vec<f64> = {
            let mut v = vec![0.0; crate::model_router::CONTEXT_DIM];
            v[1] = 1.0; // Implementation one-hot
            v[8] = 0.5; // Standard complexity
            v[10] = 0.25;
            v[11] = 0.25;
            v[12] = 0.25;
            v[13] = 0.25;
            v[14] = 0.5; // crate familiarity
            v[16] = 1.0; // bias term
            v
        };

        // Feed one more observation than required to cross into UCB stage.
        // Alternate rewards so the A matrix accumulates a non-trivial value.
        let n = CONFIDENCE_TO_UCB_THRESHOLD as usize + 1;
        for i in 0..n {
            let model_idx = i % slugs.len();
            let reward = if model_idx == 0 { 1.0 } else { 0.2 };
            router.observe(ctx_vec.clone(), model_idx, reward);
        }

        assert_eq!(
            router.current_stage(),
            CascadeStage::Ucb,
            "router must enter UCB stage after {} observations",
            n
        );

        // Capture the A[1][1] value for the first arm before saving.
        // Index 1 is the Implementation one-hot feature (x[1]=1.0 in ctx_vec),
        // so A[1][1] accumulates the number of times arm 0 was updated.
        let pre_a11 = router.linucb().arm_stats()[0].a_matrix[1][1];
        assert!(
            (pre_a11 - 1.0).abs() > 1e-6,
            "after training, A[1][1] should differ from the identity value 1.0 (got {pre_a11})"
        );

        // Route with the training context before saving.
        let pre_slug = router.select(ctx_vec.clone()).model.slug;

        // ── Phase 2: persist ────────────────────────────────────────────
        router.save(&path).expect("save should succeed");

        // ── Phase 3: reload (simulated restart) ─────────────────────────
        let reloaded = CascadeRouter::load_or_new(&path, slugs.clone());

        assert_eq!(
            reloaded.current_stage(),
            CascadeStage::Ucb,
            "reloaded router must still be in UCB stage"
        );

        // Verify A[1][1] survived the round-trip.
        let post_a11 = reloaded.linucb().arm_stats()[0].a_matrix[1][1];
        assert!(
            (post_a11 - pre_a11).abs() < 1e-9,
            "A[1][1] must be identical after reload (pre={pre_a11}, post={post_a11})"
        );
        assert!(
            (post_a11 - 1.0).abs() > 1e-6,
            "reloaded A[1][1] must still differ from identity (got {post_a11})"
        );

        // Verify routing is stable across the restart.
        let post_slug = reloaded.select(ctx_vec).model.slug;
        assert_eq!(
            pre_slug, post_slug,
            "routing the same context must yield the same primary slug after reload"
        );
    }

    // ── Provider-health integration tests (E48-T08) ─────────────────────
    //
    // These tests prove that route_with_health_scored and filter_unhealthy
    // integrate correctly with ProviderHealthRegistry:
    //   1. Open-circuit providers are excluded from selection.
    //   2. All-Open fallback routes without error.
    //   3. filter_unhealthy returns healthy subset when available.
    //   4. filter_unhealthy returns best-ranked candidate when all are unhealthy.
    //   5. Auto-recovery: provider moves Open → Closed after success.

    fn health_routing_ctx() -> RoutingContext {
        RoutingContext::default()
    }

    fn two_provider_map() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("claude-sonnet-4-5".into(), "anthropic".into());
        m.insert("claude-haiku-4-5".into(), "anthropic".into());
        m.insert("gemini-2.5-flash".into(), "google".into());
        m
    }

    /// Open-circuit provider: its models must be excluded from the candidate
    /// list so the router selects the other provider.
    #[test]
    fn cascade_health_excludes_open_circuit_provider() {
        use crate::provider_health::ErrorClass;

        let slugs = vec!["claude-sonnet-4-5".into(), "gemini-2.5-flash".into()];
        let router = CascadeRouter::new(slugs);
        let ctx = health_routing_ctx();

        let health = crate::provider_health::ProviderHealthRegistry::new();
        // Drive anthropic into Open state (3 consecutive failures).
        health.record_failure("anthropic", ErrorClass::RateLimit);
        health.record_failure("anthropic", ErrorClass::RateLimit);
        health.record_failure("anthropic", ErrorClass::RateLimit);

        let model_providers = two_provider_map();
        let route = router.route_with_health_scored(&ctx, &health, &model_providers, None, None);

        assert_eq!(
            route.primary.slug, "gemini-2.5-flash",
            "Open anthropic circuit must be excluded; gemini must be selected"
        );
    }

    /// When all providers are Open the router must fall back to the unfiltered
    /// route rather than returning an error.
    #[test]
    fn cascade_health_all_open_falls_back_gracefully() {
        use crate::provider_health::ErrorClass;

        let slugs = vec!["claude-sonnet-4-5".into(), "gemini-2.5-flash".into()];
        let router = CascadeRouter::new(slugs);
        let ctx = health_routing_ctx();

        let health = crate::provider_health::ProviderHealthRegistry::new();
        for _ in 0..3 {
            health.record_failure("anthropic", ErrorClass::ServerError);
            health.record_failure("google", ErrorClass::ServerError);
        }

        let model_providers = two_provider_map();
        // Should not panic and must return a valid slug.
        let route = router.route_with_health_scored(&ctx, &health, &model_providers, None, None);
        assert!(
            !route.primary.slug.is_empty(),
            "fallback route must return a non-empty slug"
        );
    }

    /// filter_unhealthy returns healthy candidates when available, and falls
    /// back to the least-degraded candidate when all are unhealthy.
    #[test]
    fn filter_unhealthy_returns_healthy_subset() {
        use crate::provider_health::ErrorClass;

        let slugs = vec![
            "claude-sonnet-4-5".into(),
            "gemini-2.5-flash".into(),
            "claude-haiku-4-5".into(),
        ];
        let router = CascadeRouter::new(slugs);

        let health = crate::provider_health::ProviderHealthRegistry::new();
        // anthropic: Open (3 failures)
        for _ in 0..3 {
            health.record_failure("anthropic", ErrorClass::RateLimit);
        }
        // google: Closed (healthy)
        health.record_success("google");

        let model_providers = two_provider_map();
        let candidates = vec![
            "claude-sonnet-4-5".into(),
            "gemini-2.5-flash".into(),
            "claude-haiku-4-5".into(),
        ];
        let filtered = router.filter_unhealthy(&candidates, &health, &model_providers);

        // Only gemini maps to healthy google provider.
        assert!(
            filtered.contains(&"gemini-2.5-flash".to_string()),
            "healthy google provider must be included"
        );
        assert!(
            !filtered.contains(&"claude-sonnet-4-5".to_string()),
            "Open anthropic must be excluded"
        );
        assert!(
            !filtered.contains(&"claude-haiku-4-5".to_string()),
            "Open anthropic (haiku) must be excluded"
        );
    }

    /// filter_unhealthy returns the least-degraded candidate when all are
    /// unhealthy, rather than an empty vec.
    #[test]
    fn filter_unhealthy_falls_back_to_best_candidate_when_all_unhealthy() {
        use crate::provider_health::ErrorClass;

        let slugs = vec!["claude-sonnet-4-5".into(), "claude-haiku-4-5".into()];
        let router = CascadeRouter::new(slugs);

        let health = crate::provider_health::ProviderHealthRegistry::new();
        // Tip both into Open.
        for _ in 0..3 {
            health.record_failure("anthropic", ErrorClass::ServerError);
        }

        let model_providers = {
            let mut m = HashMap::new();
            m.insert("claude-sonnet-4-5".into(), "anthropic".into());
            m.insert("claude-haiku-4-5".into(), "anthropic".into());
            m
        };
        let candidates = vec!["claude-sonnet-4-5".into(), "claude-haiku-4-5".into()];
        let filtered = router.filter_unhealthy(&candidates, &health, &model_providers);

        assert_eq!(
            filtered.len(),
            1,
            "must return exactly one candidate when all are unhealthy"
        );
        // The returned candidate is the least-degraded one (ranked by
        // ProviderHealthSnapshotKey ordering).
        assert!(
            candidates.contains(&filtered[0]),
            "returned candidate must be from the original list"
        );
    }

    /// Auto-recovery: a provider that was Open returns to Closed after a
    /// successful request is recorded, and subsequent routing includes it again.
    #[test]
    fn health_auto_recovery_after_success() {
        use crate::provider_health::ErrorClass;

        let slugs = vec!["claude-sonnet-4-5".into(), "gemini-2.5-flash".into()];
        let router = CascadeRouter::new(slugs);
        let ctx = health_routing_ctx();
        let model_providers = two_provider_map();

        let health = crate::provider_health::ProviderHealthRegistry::new();

        // Step 1: trip anthropic to Open.
        for _ in 0..3 {
            health.record_failure("anthropic", ErrorClass::RateLimit);
        }
        assert!(
            !health.is_healthy("anthropic"),
            "must be Open after 3 failures"
        );

        // Verify gemini is chosen when anthropic is Open.
        let route_during_outage =
            router.route_with_health_scored(&ctx, &health, &model_providers, None, None);
        assert_eq!(
            route_during_outage.primary.slug, "gemini-2.5-flash",
            "gemini must be selected while anthropic is Open"
        );

        // Step 2: record a success — closes the circuit.
        health.record_success("anthropic");
        assert!(
            health.is_healthy("anthropic"),
            "anthropic must be Closed again after success"
        );

        // Step 3: anthropic models are candidates again.
        let available = router.filter_unhealthy(
            &["claude-sonnet-4-5".into(), "gemini-2.5-flash".into()],
            &health,
            &model_providers,
        );
        assert!(
            available.contains(&"claude-sonnet-4-5".to_string()),
            "claude-sonnet-4-5 must be available again after recovery"
        );
    }

    // ── Provider pass-rate scoring tests ────────────────────────────────
    //
    // These tests prove that provider pass-rate metrics are computed and
    // applied correctly to model scores:
    //   1. compute_provider_metrics excludes providers below min_calls.
    //   2. ProviderMetrics::pass_rate returns 1.0 for zero-call providers.
    //   3. apply_provider_pass_rate applies the lerp multiplier correctly.
    //   4. apply_provider_pass_rate with weight=0.0 is a no-op.
    //   5. route_with_provider_metrics in Confidence stage selects the model
    //      backed by the higher-pass-rate provider.

    /// compute_provider_metrics must exclude providers with fewer than min_calls
    /// requests and include those at or above the threshold.
    #[test]
    fn compute_provider_metrics_excludes_below_min_calls() {
        let health = ProviderHealthRegistry::new();

        // "anthropic": 3 requests (below min_calls=5 → excluded).
        health.record_success("anthropic");
        health.record_success("anthropic");
        health.record_failure("anthropic", crate::provider_health::ErrorClass::ServerError);

        // "google": 6 requests, 1 failure → pass_rate = 5/6 ≈ 0.833.
        for _ in 0..5 {
            health.record_success("google");
        }
        health.record_failure("google", crate::provider_health::ErrorClass::RateLimit);

        let metrics = compute_provider_metrics(&health, 5);

        assert!(
            !metrics.contains_key("anthropic"),
            "anthropic (3 requests) must be excluded when min_calls=5"
        );
        assert!(
            metrics.contains_key("google"),
            "google (6 requests) must be included when min_calls=5"
        );

        let google = &metrics["google"];
        assert_eq!(google.total_calls, 6, "total_calls must be 6");
        assert_eq!(google.successful_calls, 5, "successful_calls must be 5");

        let rate = google.pass_rate();
        assert!(
            (rate - 5.0 / 6.0).abs() < 1e-9,
            "pass_rate must be 5/6 (got {rate})"
        );
    }

    /// When a provider has no recorded calls, pass_rate returns 1.0 (optimistic default).
    #[test]
    fn provider_metrics_pass_rate_returns_one_when_no_calls() {
        let m = ProviderMetrics {
            total_calls: 0,
            successful_calls: 0,
        };
        assert_eq!(
            m.pass_rate(),
            1.0,
            "unknown provider must be treated optimistically (pass_rate=1.0)"
        );
    }

    /// apply_provider_pass_rate must scale score by lerp(1.0, pass_rate, weight).
    /// With weight=1.0 the score is multiplied directly by pass_rate.
    /// Slugs with no matching provider entry must be left unchanged.
    #[test]
    fn apply_provider_pass_rate_multiplies_with_lerp() {
        // Build a minimal provider_metrics map: "google" has pass_rate=0.6.
        let mut provider_metrics = HashMap::new();
        provider_metrics.insert(
            "google".to_string(),
            ProviderMetrics {
                total_calls: 10,
                successful_calls: 6,
            },
        );

        // model_providers: gemini → google, sonnet → anthropic (not in metrics).
        let mut model_providers = HashMap::new();
        model_providers.insert("gemini-2.5-flash".to_string(), "google".to_string());
        model_providers.insert("claude-sonnet-4-5".to_string(), "anthropic".to_string());

        let mut scores = vec![
            ("gemini-2.5-flash".to_string(), 1.0_f64),
            ("claude-sonnet-4-5".to_string(), 1.0_f64),
        ];

        // weight = 1.0 → multiplier = pass_rate directly.
        apply_provider_pass_rate(&mut scores, &provider_metrics, &model_providers, 1.0);

        let gemini_score = scores
            .iter()
            .find(|(s, _)| s == "gemini-2.5-flash")
            .map(|(_, s)| *s)
            .expect("gemini must be present");
        let sonnet_score = scores
            .iter()
            .find(|(s, _)| s == "claude-sonnet-4-5")
            .map(|(_, s)| *s)
            .expect("sonnet must be present");

        // gemini: 1.0 * (1 - 1.0 + 1.0 * 0.6) = 0.6
        assert!(
            (gemini_score - 0.6).abs() < 1e-9,
            "gemini score must be 0.6 with weight=1.0 and pass_rate=0.6 (got {gemini_score})"
        );
        // sonnet: provider not in metrics → unchanged at 1.0
        assert!(
            (sonnet_score - 1.0).abs() < 1e-9,
            "sonnet score must be unchanged (no anthropic metrics) (got {sonnet_score})"
        );
    }

    /// apply_provider_pass_rate with weight=0.0 must leave all scores unchanged.
    #[test]
    fn apply_provider_pass_rate_zero_weight_is_noop() {
        let mut provider_metrics = HashMap::new();
        provider_metrics.insert(
            "anthropic".to_string(),
            ProviderMetrics {
                total_calls: 10,
                successful_calls: 3,
            },
        );
        let mut model_providers = HashMap::new();
        model_providers.insert("claude-sonnet-4-5".to_string(), "anthropic".to_string());

        let mut scores = vec![("claude-sonnet-4-5".to_string(), 0.9_f64)];
        apply_provider_pass_rate(&mut scores, &provider_metrics, &model_providers, 0.0);

        assert!(
            (scores[0].1 - 0.9).abs() < 1e-9,
            "weight=0 must leave score unchanged (got {})",
            scores[0].1
        );
    }

    /// route_with_provider_metrics must prefer the model backed by the higher-pass-rate
    /// provider when both models have equal confidence-stage base scores.
    ///
    /// Setup: two models with equal observation count (30 each, all successful).
    ///   - "gemini-2.5-flash" → provider "google" with pass_rate=0.9 (9/10 calls)
    ///   - "claude-sonnet-4-5" → provider "anthropic" with pass_rate=0.5 (5/10 calls)
    ///
    /// With weight=1.0, google's multiplier (0.9) greatly exceeds anthropic's (0.5),
    /// so route_with_provider_metrics must select gemini.
    #[test]
    fn route_with_provider_metrics_prefers_higher_pass_rate_provider() {
        let slugs = vec!["claude-sonnet-4-5".into(), "gemini-2.5-flash".into()];
        let router = CascadeRouter::new(slugs);

        // Inject 60 observations (30 each) so both models have equal base scores
        // and the router is in the Confidence stage (>= 50 observations).
        let ctx_vec = vec![0.0; crate::model_router::CONTEXT_DIM];
        for i in 0..60_usize {
            router.replay_observation(
                if i % 2 == 0 {
                    "claude-sonnet-4-5"
                } else {
                    "gemini-2.5-flash"
                },
                &ctx_vec,
                i % 2,
                0.8,
                true,
            );
        }
        assert_eq!(
            router.current_stage(),
            CascadeStage::Confidence,
            "router must be in Confidence stage for this test"
        );

        // google: 9/10 calls pass (0.9); anthropic: 5/10 calls pass (0.5).
        let mut provider_metrics = HashMap::new();
        provider_metrics.insert(
            "google".to_string(),
            ProviderMetrics {
                total_calls: 10,
                successful_calls: 9,
            },
        );
        provider_metrics.insert(
            "anthropic".to_string(),
            ProviderMetrics {
                total_calls: 10,
                successful_calls: 5,
            },
        );

        let mut model_providers = HashMap::new();
        model_providers.insert("gemini-2.5-flash".to_string(), "google".to_string());
        model_providers.insert("claude-sonnet-4-5".to_string(), "anthropic".to_string());

        let ctx = RoutingContext::default();
        // weight=1.0 → full pass-rate multiplier applied.
        let route =
            router.route_with_provider_metrics(&ctx, &provider_metrics, &model_providers, 1.0);

        assert_eq!(
            route.primary.slug, "gemini-2.5-flash",
            "higher-pass-rate google provider must be preferred over anthropic; got {}",
            route.primary.slug
        );
    }
}

// ─── FallbackChain tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod fallback_tests {
    use super::*;
    use roko_core::agent::ModelSpec;

    /// claude-* slugs map to AgentBackend::Claude via heuristic.
    fn anthropic(slug: &str) -> ModelSpec {
        ModelSpec::from_slug(slug)
    }

    /// gemini-* slugs map to AgentBackend::Codex (OpenAI-compat) via heuristic.
    /// Either way they differ from Claude backend, satisfying the different-backend test.
    fn google(slug: &str) -> ModelSpec {
        ModelSpec::from_slug(slug)
    }

    fn make_cascade(primary: ModelSpec, fallbacks: Vec<ModelSpec>) -> CascadeModel {
        CascadeModel {
            primary,
            fallback_chain: fallbacks,
            context_overflow_fallback: None,
            latency_sla_ms: 30_000,
            stage: CascadeStage::Static,
        }
    }

    // ── FallbackChain::new ───────────────────────────────────────────────────

    #[test]
    fn fallback_chain_empty_when_no_fallbacks() {
        let cascade = make_cascade(anthropic("claude-sonnet-4-6"), vec![]);
        let chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", None);
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert_eq!(chain.remaining(), 0);
        assert!(chain.is_exhausted());
    }

    #[test]
    fn fallback_chain_different_backend_first() {
        // Primary is anthropic (Claude backend); fallbacks include one same-backend
        // Claude model then a Codex-backend Gemini model.
        // The different-backend (Gemini) model should appear first.
        let cascade = make_cascade(
            anthropic("claude-sonnet-4-6"),
            vec![
                anthropic("claude-haiku-4-5"), // same backend (Claude)
                google("gemini-2.5-flash"),    // different backend (Codex) — should come first
            ],
        );
        let mut chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", None);
        assert_eq!(chain.len(), 2);

        let (first, event) = chain.advance().expect("first fallback must exist");
        assert_eq!(
            first.slug, "gemini-2.5-flash",
            "different-backend model must come first"
        );
        assert_eq!(event.fallback_slug, "gemini-2.5-flash");
        assert_eq!(event.rate_limited_slug, "claude-sonnet-4-6");
        assert_eq!(event.depth, 1);
    }

    #[test]
    fn fallback_chain_respects_max_depth() {
        let cascade = make_cascade(
            anthropic("claude-sonnet-4-6"),
            vec![
                google("gemini-2.5-flash"),
                google("gemini-2.5-pro"),
                google("gemini-2.5-flash-lite"),
                anthropic("claude-haiku-4-5"),
            ],
        );
        // Limit to depth 2.
        let chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", Some(2));
        assert_eq!(chain.len(), 2, "depth cap must limit to 2 candidates");
        assert_eq!(chain.max_fallback_depth, 2);
    }

    #[test]
    fn fallback_chain_default_max_depth_is_three() {
        let cascade = make_cascade(
            anthropic("claude-sonnet-4-6"),
            vec![
                google("gemini-2.5-flash"),
                google("gemini-2.5-pro"),
                google("gemini-2.5-flash-lite"),
                anthropic("claude-haiku-4-5"),
            ],
        );
        let chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", None);
        assert_eq!(
            chain.len(),
            3,
            "default depth must cap at DEFAULT_MAX_FALLBACK_DEPTH=3"
        );
        assert_eq!(chain.max_fallback_depth, DEFAULT_MAX_FALLBACK_DEPTH);
    }

    #[test]
    fn fallback_chain_excludes_rate_limited_slug_from_candidates() {
        // The rate-limited model also appears in the fallback list (unusual but
        // possible if the config is strange). It must be excluded.
        let cascade = make_cascade(
            anthropic("claude-sonnet-4-6"),
            vec![
                anthropic("claude-sonnet-4-6"), // same as primary — must be excluded
                google("gemini-2.5-flash"),
            ],
        );
        let chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", None);
        assert_eq!(
            chain.len(),
            1,
            "rate-limited slug must be excluded from candidates"
        );
        let slugs: Vec<_> = {
            let mut c = chain;
            let mut out = vec![];
            while let Some((m, _)) = c.advance() {
                out.push(m.slug.clone());
            }
            out
        };
        assert!(!slugs.contains(&"claude-sonnet-4-6".to_string()));
    }

    // ── FallbackChain::advance ───────────────────────────────────────────────

    #[test]
    fn fallback_chain_advance_returns_none_when_exhausted() {
        let cascade = make_cascade(
            anthropic("claude-sonnet-4-6"),
            vec![google("gemini-2.5-flash")],
        );
        let mut chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", None);
        assert!(
            chain.advance().is_some(),
            "first advance must return a model"
        );
        assert!(
            chain.is_exhausted(),
            "chain must be exhausted after last model"
        );
        assert!(
            chain.advance().is_none(),
            "advance past exhaustion must return None"
        );
    }

    #[test]
    fn fallback_chain_depth_increments_per_advance() {
        let cascade = make_cascade(
            anthropic("claude-sonnet-4-6"),
            vec![google("gemini-2.5-flash"), google("gemini-2.5-pro")],
        );
        let mut chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", None);

        let (_, ev1) = chain.advance().unwrap();
        assert_eq!(ev1.depth, 1);

        let (_, ev2) = chain.advance().unwrap();
        assert_eq!(ev2.depth, 2);
    }

    #[test]
    fn fallback_chain_remaining_decrements_on_advance() {
        let cascade = make_cascade(
            anthropic("claude-sonnet-4-6"),
            vec![google("gemini-2.5-flash"), google("gemini-2.5-pro")],
        );
        let mut chain = FallbackChain::new(&cascade, "claude-sonnet-4-6", None);
        assert_eq!(chain.remaining(), 2);
        let _ = chain.advance();
        assert_eq!(chain.remaining(), 1);
        let _ = chain.advance();
        assert_eq!(chain.remaining(), 0);
    }

    // ── CascadeRouter::record_fallback_outcome ───────────────────────────────

    #[test]
    fn record_fallback_outcome_records_rate_limited_model_as_failure() {
        let router = CascadeRouter::new(vec![
            "claude-sonnet-4-6".to_string(),
            "gemini-2.5-flash".to_string(),
        ]);

        let event = FallbackEvent {
            rate_limited_slug: "claude-sonnet-4-6".to_string(),
            fallback_slug: "gemini-2.5-flash".to_string(),
            depth: 1,
        };
        let (rl_recorded, fb_recorded) = router.record_fallback_outcome(&event, true);
        assert!(rl_recorded, "rate-limited model must be known to router");
        assert!(fb_recorded, "fallback model must be known to router");

        let snap = router.confidence_snapshot();

        // Rate-limited model: 1 trial, 0 successes.
        let (rl_trials, rl_successes) = snap["claude-sonnet-4-6"];
        assert_eq!(rl_trials, 1, "rate-limited model must have 1 trial");
        assert_eq!(
            rl_successes, 0,
            "rate-limited model must have 0 successes (it failed)"
        );

        // Fallback model: 1 trial, 1 success (fallback_succeeded=true).
        let (fb_trials, fb_successes) = snap["gemini-2.5-flash"];
        assert_eq!(fb_trials, 1, "fallback model must have 1 trial");
        assert_eq!(fb_successes, 1, "fallback model must have 1 success");
    }

    #[test]
    fn record_fallback_outcome_records_failure_when_fallback_also_fails() {
        let router = CascadeRouter::new(vec![
            "claude-sonnet-4-6".to_string(),
            "gemini-2.5-flash".to_string(),
        ]);

        let event = FallbackEvent {
            rate_limited_slug: "claude-sonnet-4-6".to_string(),
            fallback_slug: "gemini-2.5-flash".to_string(),
            depth: 1,
        };
        // fallback_succeeded = false
        router.record_fallback_outcome(&event, false);

        let snap = router.confidence_snapshot();
        let (fb_trials, fb_successes) = snap["gemini-2.5-flash"];
        assert_eq!(fb_trials, 1);
        assert_eq!(
            fb_successes, 0,
            "fallback failure must not increment successes"
        );
    }

    #[test]
    fn record_fallback_outcome_returns_false_for_unknown_slug() {
        let router = CascadeRouter::new(vec!["claude-sonnet-4-6".to_string()]);

        let event = FallbackEvent {
            rate_limited_slug: "nonexistent-model".to_string(),
            fallback_slug: "also-nonexistent".to_string(),
            depth: 1,
        };
        let (rl_recorded, fb_recorded) = router.record_fallback_outcome(&event, true);
        assert!(!rl_recorded, "unknown rate-limited model must return false");
        assert!(!fb_recorded, "unknown fallback model must return false");
    }

    #[test]
    fn record_fallback_outcome_accumulates_across_multiple_events() {
        let router = CascadeRouter::new(vec![
            "claude-sonnet-4-6".to_string(),
            "gemini-2.5-flash".to_string(),
        ]);

        // Two fallback events: first succeeds, second fails.
        let event1 = FallbackEvent {
            rate_limited_slug: "claude-sonnet-4-6".to_string(),
            fallback_slug: "gemini-2.5-flash".to_string(),
            depth: 1,
        };
        let event2 = FallbackEvent {
            rate_limited_slug: "claude-sonnet-4-6".to_string(),
            fallback_slug: "gemini-2.5-flash".to_string(),
            depth: 2,
        };

        router.record_fallback_outcome(&event1, true);
        router.record_fallback_outcome(&event2, false);

        let snap = router.confidence_snapshot();
        let (rl_trials, rl_successes) = snap["claude-sonnet-4-6"];
        assert_eq!(
            rl_trials, 2,
            "two rate-limit failures = 2 trials for primary"
        );
        assert_eq!(rl_successes, 0);

        let (fb_trials, fb_successes) = snap["gemini-2.5-flash"];
        assert_eq!(
            fb_trials, 2,
            "two fallback attempts = 2 trials for fallback model"
        );
        assert_eq!(fb_successes, 1, "only first fallback succeeded");
    }
}

// ─── Cost control integration tests (E48-T12) ──────────────────────────────

#[cfg(test)]
mod cost_control_integration_tests {
    use super::*;
    use crate::provider_health::{ErrorClass, ProviderHealthRegistry};

    /// Helper: build a three-model cascade router spanning two providers.
    fn three_model_router() -> CascadeRouter {
        CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
            "gemini-2.5-flash".into(),
        ])
    }

    fn two_provider_map() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("claude-sonnet-4-6".into(), "anthropic".into());
        m.insert("claude-haiku-4-5".into(), "anthropic".into());
        m.insert("gemini-2.5-flash".into(), "google".into());
        m
    }

    // ── 1. FallbackChain + ProviderHealthRegistry end-to-end ────────────

    /// Simulate rate limit on anthropic, build a FallbackChain, advance to a
    /// healthy alternative, record the outcome, and verify the router learned
    /// from the event while the health registry reflects provider state.
    #[test]
    fn fallback_chain_plus_health_registry_end_to_end() {
        let router = three_model_router();
        let health = ProviderHealthRegistry::new();
        let model_providers = two_provider_map();
        let ctx = RoutingContext::default();

        // Step 1: anthropic hits rate limit — mark unhealthy.
        health.record_failure("anthropic", ErrorClass::RateLimit);
        health.record_failure("anthropic", ErrorClass::RateLimit);
        health.record_failure("anthropic", ErrorClass::RateLimit);
        assert!(
            !health.is_healthy("anthropic"),
            "anthropic must be Open after 3 failures"
        );

        // Step 2: route with health — should pick gemini (the only healthy model).
        let route = router.route_with_health_scored(&ctx, &health, &model_providers, None, None);
        assert_eq!(
            route.primary.slug, "gemini-2.5-flash",
            "must route to the healthy provider"
        );

        // Step 3: build a FallbackChain from the original route (as if sonnet
        // was the primary but got rate-limited at dispatch time).
        let original_route = router.route(&ctx);
        let mut chain = FallbackChain::new(&original_route, &original_route.primary.slug, None);

        // Step 4: advance the chain — get the next fallback.
        let (fallback_model, fallback_event) = chain
            .advance()
            .expect("chain must have at least one fallback");
        assert_ne!(
            fallback_model.slug, original_route.primary.slug,
            "fallback must differ from the rate-limited primary"
        );

        // Step 5: record the fallback outcome in the cascade router.
        let (rl_ok, fb_ok) = router.record_fallback_outcome(&fallback_event, true);
        assert!(rl_ok, "rate-limited model must be known");
        assert!(fb_ok, "fallback model must be known");

        // Step 6: verify the cascade learned — primary has a failure, fallback
        // has a success.
        let snap = router.confidence_snapshot();
        let (rl_trials, rl_successes) = snap[&fallback_event.rate_limited_slug];
        assert_eq!(rl_trials, 1);
        assert_eq!(
            rl_successes, 0,
            "rate-limited primary must be recorded as failure"
        );

        let (fb_trials, fb_successes) = snap[&fallback_event.fallback_slug];
        assert_eq!(fb_trials, 1);
        assert_eq!(fb_successes, 1, "successful fallback must be recorded");

        // Step 7: recover anthropic and confirm routing includes it again.
        health.record_success("anthropic");
        assert!(health.is_healthy("anthropic"), "anthropic must recover");

        let recovered_available =
            router.filter_unhealthy(&["claude-sonnet-4-6".into()], &health, &model_providers);
        assert!(
            recovered_available.contains(&"claude-sonnet-4-6".to_string()),
            "sonnet must be available again after anthropic recovery"
        );
    }

    // ── 2. Budget tracking with model cost accumulation ─────────────────

    /// BudgetGuardrail tracks cost across multiple record_cost calls and
    /// transitions through all action levels.
    #[test]
    fn budget_tracking_with_model_cost_accumulation() {
        use crate::budget::{BudgetAction, BudgetGuardrail};

        let mut budget = BudgetGuardrail::new(
            10.0,  // per-task
            50.0,  // per-session
            100.0, // per-day
            0.70,  // warn at 70%
        );

        // Accumulate cost in small increments at the task level.
        assert_eq!(budget.record_cost(3.0, "task"), BudgetAction::Ok);
        assert_eq!(budget.record_cost(2.0, "task"), BudgetAction::Ok);
        // At $5.0 / $10.0 = 50% — still below warn threshold.

        // Push past 70% warning.
        match budget.record_cost(2.5, "task") {
            BudgetAction::Warn { percent_used, .. } => {
                assert!(
                    (percent_used - 0.75).abs() < 1e-9,
                    "percent must be 75% (got {percent_used})"
                );
            }
            other => panic!("expected Warn at 75%, got {other:?}"),
        }

        // Push past 80% → RouteToCheaper.
        assert_eq!(
            budget.record_cost(1.0, "task"),
            BudgetAction::RouteToCheaper,
            "85% usage must trigger RouteToCheaper"
        );

        // Push past 95% → BlockNewSessions.
        assert_eq!(
            budget.record_cost(1.0, "task"),
            BudgetAction::BlockNewSessions,
            "95% usage must trigger BlockNewSessions"
        );

        // Push to 100% → Block.
        assert_eq!(
            budget.record_cost(0.5, "task"),
            BudgetAction::Block,
            "100%+ usage must trigger Block"
        );
    }

    // ── 3. Rate-limit detection → fallback → recording flow ─────────────

    /// Full flow: detect rate limit, advance FallbackChain, record outcome,
    /// verify confidence stats reflect the outcome for both models.
    #[test]
    fn rate_limit_fallback_recording_flow() {
        let router = CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gemini-2.5-flash".into(),
            "gemini-2.5-pro".into(),
        ]);

        // Build a route, then simulate rate limit on primary.
        let route = router.route(&RoutingContext::default());
        let primary_slug = route.primary.slug.clone();

        let mut chain = FallbackChain::new(&route, &primary_slug, None);

        // Advance through two fallbacks: first succeeds, second fails.
        let mut outcomes = vec![];
        while let Some((model, event)) = chain.advance() {
            let success = outcomes.is_empty(); // first = success, second = failure
            router.record_fallback_outcome(&event, success);
            outcomes.push((model.slug.clone(), success));
        }

        assert!(
            !outcomes.is_empty(),
            "must have advanced at least one fallback"
        );

        // Verify the rate-limited primary accumulated as many failures as
        // fallback hops.
        let snap = router.confidence_snapshot();
        if let Some(&(trials, successes)) = snap.get(&primary_slug) {
            assert_eq!(
                trials,
                outcomes.len() as u64,
                "rate-limited primary must record one failure per fallback hop"
            );
            assert_eq!(successes, 0, "all primary entries must be failures");
        }

        // Verify the first fallback was recorded as success.
        let first_fb = &outcomes[0].0;
        if let Some(&(trials, successes)) = snap.get(first_fb) {
            assert!(trials >= 1);
            assert!(successes >= 1, "first fallback must be recorded as success");
        }
    }

    // ── 4. Cascade router respects provider health state ────────────────

    /// When a provider is unhealthy the cascade router must exclude its models
    /// from selection. When the provider recovers, its models must be candidates
    /// again.
    #[test]
    fn cascade_router_respects_provider_health_state() {
        let router = three_model_router();
        let health = ProviderHealthRegistry::new();
        let model_providers = two_provider_map();
        let ctx = RoutingContext::default();

        // Baseline: all healthy → route may pick any model.
        let baseline = router.route_with_health_scored(&ctx, &health, &model_providers, None, None);
        assert!(
            !baseline.primary.slug.is_empty(),
            "baseline route must return a valid slug"
        );

        // Trip google → Open.
        for _ in 0..3 {
            health.record_failure("google", ErrorClass::ServerError);
        }
        assert!(!health.is_healthy("google"));

        // Route must now exclude gemini (the only google model).
        let route_no_google =
            router.route_with_health_scored(&ctx, &health, &model_providers, None, None);
        assert_ne!(
            route_no_google.primary.slug, "gemini-2.5-flash",
            "gemini must be excluded when google is Open"
        );

        // Recover google.
        health.record_success("google");
        assert!(health.is_healthy("google"));

        // Route should consider gemini again — filter_unhealthy confirms.
        let all_slugs: Vec<String> = vec![
            "claude-sonnet-4-6".into(),
            "claude-haiku-4-5".into(),
            "gemini-2.5-flash".into(),
        ];
        let filtered = router.filter_unhealthy(&all_slugs, &health, &model_providers);
        assert!(
            filtered.contains(&"gemini-2.5-flash".to_string()),
            "gemini must be available after google recovery"
        );
    }

    // ── 5. Cost pressure penalizes premium models under spike ───────────

    /// apply_cost_pressure must zero-out premium models when spike=true,
    /// boost fast models, and leave scores unchanged when spike=false.
    #[test]
    fn cost_pressure_penalizes_premium_under_spike() {
        let router = CascadeRouter::new(vec![
            "claude-opus-4-6".into(),   // Premium
            "claude-sonnet-4-6".into(), // Standard
            "claude-haiku-4-5".into(),  // Fast
        ]);

        let mut candidates = vec![
            ("claude-opus-4-6".to_string(), 1.0),
            ("claude-sonnet-4-6".to_string(), 1.0),
            ("claude-haiku-4-5".to_string(), 1.0),
        ];

        // No spike → scores unchanged.
        router.apply_cost_pressure(&mut candidates, false);
        assert!(
            (candidates[0].1 - 1.0).abs() < f64::EPSILON,
            "no spike: premium score must be unchanged"
        );

        // Spike → premium zeroed, standard reduced, fast boosted.
        router.apply_cost_pressure(&mut candidates, true);

        let opus_score = candidates[0].1;
        let sonnet_score = candidates[1].1;
        let haiku_score = candidates[2].1;

        assert!(
            opus_score < 0.01,
            "premium model must be zeroed under cost spike (got {opus_score})"
        );
        assert!(
            sonnet_score < 1.0,
            "standard model must be reduced under cost spike (got {sonnet_score})"
        );
        assert!(
            haiku_score >= 1.0,
            "fast model must be boosted under cost spike (got {haiku_score})"
        );
    }

    // ── 6. Budget guardrail resets are independent per scope ─────────────

    /// Resetting task spend must not affect session spend and vice versa.
    #[test]
    fn budget_guardrail_resets_are_independent_per_scope() {
        use crate::budget::{BudgetAction, BudgetGuardrail};

        let mut budget = BudgetGuardrail::new(10.0, 20.0, 100.0, 0.70);

        // Accumulate $8 at task level and $15 at session level.
        budget.record_cost(8.0, "task");
        budget.record_cost(15.0, "session");

        // Reset task.
        budget.reset_task();

        // Task must be clean (new $5 at task level → 50% → Ok).
        assert_eq!(
            budget.record_cost(5.0, "task"),
            BudgetAction::Ok,
            "task budget must be reset to 0"
        );

        // Session must still be at $15 + $5 = $20 → 100% → Block.
        assert_eq!(
            budget.record_cost(5.0, "session"),
            BudgetAction::Block,
            "session spend must not be affected by task reset"
        );
    }

    // ── 7. Budget zero limit means unlimited ────────────────────────────

    /// A per_task_limit_usd of 0.0 must never trigger any budget action.
    #[test]
    fn budget_zero_limit_means_unlimited() {
        use crate::budget::{BudgetAction, BudgetGuardrail};

        let mut budget = BudgetGuardrail::new(0.0, 0.0, 0.0, 0.80);

        // Record absurdly large cost — must always be Ok.
        assert_eq!(budget.record_cost(1_000_000.0, "task"), BudgetAction::Ok);
        assert_eq!(budget.record_cost(1_000_000.0, "session"), BudgetAction::Ok);
        assert_eq!(budget.record_cost(1_000_000.0, "day"), BudgetAction::Ok);
    }
}
