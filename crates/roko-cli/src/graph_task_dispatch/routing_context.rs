//! Model routing inputs of a Graph task dispatch: the cheap helper model, cross-cut
//! and dream routing bias, the agent contract, and the routing context.

use roko_core::TaskDomain;
use roko_core::tool::ToolRegistry;
use roko_std::StaticToolRegistry;
use roko_std::roles::domain_profile;

use super::*;

/// Thin `Agent` adapter that forwards a one-shot prompt through the shared
/// factory bridge so `error_enrichment`, the gate reflection and the LLM
/// judge can use the live provider without rebuilding the full dispatch
/// stack.
///
/// The adapter is intentionally lightweight: it constructs a minimal
/// `AgentDispatchRequest` with no tools, no MCP, and no contract, targeting
/// the model chosen by [`select_cheap_model_key`] and bounded by
/// `timeouts.llm_call_secs`. On dispatch failure it returns an
/// unsuccessful `AgentResult` so callers' built-in fallbacks activate.
pub(super) struct CheapFactoryAgent {
    pub(super) factory: Arc<SharedAgentFactory>,
    pub(super) model_key: String,
    pub(super) workdir: PathBuf,
    pub(super) timeout_ms: u64,
}

#[async_trait::async_trait]
impl roko_agent::Agent for CheapFactoryAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> roko_agent::AgentResult {
        let prompt = match input.body.as_text() {
            Ok(text) => text.to_string(),
            Err(_) => {
                return roko_agent::AgentResult::fail(
                    Signal::builder(Kind::AgentOutput)
                        .body(Body::text("cheap-factory-agent: non-text input"))
                        .build(),
                );
            }
        };
        let request = AgentDispatchRequest {
            model_key: self.model_key.clone(),
            prompt,
            system_prompt: String::new(),
            workdir: self.workdir.clone(),
            immune_root: None,
            agent_id: "cheap-factory-agent".to_string(),
            command: None,
            timeout_ms: Some(self.timeout_ms),
            mcp_config: None,
            env: vec![],
            extra_args: vec![],
            effort: None,
            tools: None,
            agent_contract: None,
            bare_mode: false,
            dangerously_skip_permissions: false,
            max_turns: None,
            live_output: None,
            attempt_key: None,
        };
        match self.factory.run_shared_agent_bridge(request).await {
            Ok(dispatch) => dispatch.result,
            Err(_) => roko_agent::AgentResult::fail(
                Signal::builder(Kind::AgentOutput)
                    .body(Body::text("cheap-factory-agent: dispatch failed"))
                    .build(),
            ),
        }
    }

    fn name(&self) -> &str {
        "cheap-factory-agent"
    }
}

/// Choose the model for best-effort one-shot helper calls (error
/// enrichment, gate reflections, the opt-in LLM judge).
///
/// Prefers `routing.fast_task_model` when it names (by `[models.*]` key or
/// slug) a dispatchable model. Otherwise picks the cheapest dispatchable
/// model by input price, then output price, using `[models.*]` prices with
/// the built-in pricing table as fallback; unpriced models rank last.
/// Embedding models, tool-less (search-only) models, and models of
/// `routing.disabled_providers` are never chosen. Returns the `[models.*]`
/// key so dispatch resolves exactly that profile.
pub(super) fn select_cheap_model_key(config: &RokoConfig) -> Option<String> {
    select_cheap_model_key_with(config, |key| config.provider_available_for_model_key(key))
}

/// [`select_cheap_model_key`] with an injectable provider-availability check.
fn select_cheap_model_key_with(
    config: &RokoConfig,
    available: impl Fn(&str) -> bool,
) -> Option<String> {
    let models = config.effective_models();
    let candidates: Vec<(&String, &roko_core::config::schema::ModelProfile)> = models
        .iter()
        .filter(|(key, profile)| {
            !profile.is_embedding_model
                && profile.supports_tools
                && !profile.slug.trim().is_empty()
                && !config
                    .routing
                    .disabled_providers
                    .contains(&profile.provider)
                && available(key)
        })
        .collect();

    let fast = config.routing.fast_task_model.trim();
    if let Some((key, _)) = candidates
        .iter()
        .find(|(key, profile)| !fast.is_empty() && (key.as_str() == fast || profile.slug == fast))
    {
        return Some((*key).clone());
    }

    let price = |profile: &roko_core::config::schema::ModelProfile| {
        let builtin = roko_core::config::model_registry::builtin_pricing(&profile.slug);
        (
            profile
                .cost_input_per_m
                .or(builtin.map(|pricing| pricing.input_per_m)),
            profile
                .cost_output_per_m
                .or(builtin.map(|pricing| pricing.output_per_m)),
        )
    };
    candidates
        .into_iter()
        .min_by(|(a_key, a_profile), (b_key, b_profile)| {
            let (a_input, a_output) = price(a_profile);
            let (b_input, b_output) = price(b_profile);
            cmp_price(a_input, b_input)
                .then(cmp_price(a_output, b_output))
                .then_with(|| a_key.cmp(b_key))
        })
        .map(|(key, _)| key.clone())
}

/// The model to dispatch for the routed `model` (a `[models.*]` key or slug)
/// of a task whose `preferred_provider` is `provider`, a `[providers.*]` id:
/// the key of that provider's entry for the same slug when it has a usable
/// one, else `model` as routed.
fn preferred_provider_model(config: &RokoConfig, model: &str, provider: Option<&str>) -> String {
    preferred_provider_model_with(config, model, provider, |key| {
        config.provider_available_for_model_key(key)
    })
}

impl GraphTaskDispatcher {
    /// The model an attempt of `task` dispatches for `dispatch_plan`, on
    /// both dispatch paths: the routed model, run by the entry of the task's
    /// `preferred_provider` when it has one ([`preferred_provider_model`]).
    /// `--model` and express mode keep theirs.
    pub(super) fn dispatch_model_key(
        &self,
        dispatch_plan: &crate::dispatch::RunnerDispatchPlan,
        task: &TaskDef,
    ) -> String {
        if dispatch_plan.forced {
            return dispatch_plan.model.slug.clone();
        }
        preferred_provider_model(
            &self.config,
            &dispatch_plan.model.slug,
            task.hints.preferred_provider.as_deref(),
        )
    }
}

/// [`preferred_provider_model`] with an injectable provider-availability
/// check on a `[models.*]` key.
fn preferred_provider_model_with(
    config: &RokoConfig,
    model: &str,
    provider: Option<&str>,
    available: impl Fn(&str) -> bool,
) -> String {
    let Some(provider) = provider
        .map(str::trim)
        .filter(|provider| !provider.is_empty())
    else {
        return model.to_string();
    };
    let models = config.effective_models();
    let slug = models
        .get(model)
        .map_or(model, |profile| profile.slug.as_str());
    models
        .iter()
        .find(|(key, profile)| {
            profile.slug == slug
                && profile.provider == provider
                && profile.supports_tools
                && !config
                    .routing
                    .disabled_providers
                    .contains(&profile.provider)
                && available(key)
        })
        .map_or_else(|| model.to_string(), |(key, _)| key.clone())
}

/// Order prices ascending, with unknown or non-finite prices last.
fn cmp_price(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    let rank = |price: Option<f64>| {
        price
            .filter(|value| value.is_finite())
            .unwrap_or(f64::INFINITY)
    };
    rank(a).total_cmp(&rank(b))
}

/// P1-16: Resolve cross-cut functor conflicts at routing time.
///
/// When Memory, Daimon, and Dreams all propose routing recommendations on
/// the same signal set, the arbitrator applies priority resolution (safety-
/// critical Daimon wins, consolidated Memory beats speculative Dreams) and
/// falls back to VCG second-price arbitration for same-level ties.
///
/// Returns an `Option<RoutingBias>` derived from the winning recommendation
/// so the cascade router can incorporate the cross-cut consensus.
///
/// `dream_advice` is the persisted Dreams advice, loaded once per dispatch
/// and shared with [`dream_routing_bias`].
pub(super) fn arbitrate_cross_cut_routing_bias(
    feedback: &GraphFeedbackContext,
    dream_advice: Option<&roko_dreams::DreamRoutingAdvice>,
    task_category: &str,
) -> Option<roko_learn::cascade_router::RoutingBias> {
    use roko_compose::auction::{
        CrossCutArbitrationResult, CrossCutDecisionKind, CrossCutRecommendation,
    };

    // Collect recommendations from persisted cross-cut state.
    let mut recommendations = Vec::new();

    // Dreams routing advice (persisted by DreamOutputConsumer or delta dream).
    if let Some(advice) = dream_advice {
        for rec in &advice.recommendations {
            if rec.confidence < 0.5 {
                continue;
            }
            recommendations.push(CrossCutRecommendation {
                source: roko_compose::auction::CrossCutId::Dreams,
                decision_key: format!("route:{task_category}"),
                decision_kind: CrossCutDecisionKind::Route,
                value: rec.recommended_model.clone(),
                confidence: rec.confidence,
                priority_level: 2,
                safety_critical: false,
                knowledge_tier: None,
            });
        }
    }

    // Daimon safety override: if the daimon is Struggling, emit a safety-
    // critical recommendation to prefer a conservative model.
    if let Some(daimon) = &feedback.daimon_state {
        if let Ok(daimon) = daimon.lock() {
            let affect = daimon.query_state();
            if affect.behavioral_state == roko_core::BehavioralState::Struggling {
                recommendations.push(CrossCutRecommendation {
                    source: roko_compose::auction::CrossCutId::Daimon,
                    decision_key: format!("route:{task_category}"),
                    decision_kind: CrossCutDecisionKind::Route,
                    value: "conservative".to_string(),
                    confidence: 0.9,
                    priority_level: 1,
                    safety_critical: true,
                    knowledge_tier: None,
                });
            }
        }
    }

    if recommendations.is_empty() {
        return None;
    }

    // Run priority-then-VCG arbitration.
    let result = roko_compose::auction::resolve_by_priority(&recommendations)
        .unwrap_or_else(|| roko_compose::auction::resolve_by_vcg(&recommendations));

    match result {
        CrossCutArbitrationResult::Resolved {
            winner,
            ref recommendation,
            attention_cost,
            mechanism,
            ..
        } => {
            tracing::debug!(
                ?winner,
                value = %recommendation.value,
                attention_cost,
                ?mechanism,
                "cross-cut arbitration resolved routing recommendation"
            );
            // If the winning recommendation names a specific model to prefer,
            // deprioritize everything else. For safety-critical "conservative"
            // recommendations, signal budget pressure instead.
            if recommendation.safety_critical {
                Some(roko_learn::cascade_router::RoutingBias {
                    deprioritize: Vec::new(),
                    prefer_cheaper: true,
                    reason: format!("cross-cut safety arbitration: {}", recommendation.value),
                })
            } else {
                None // Prefer normal dream routing advice path (P1-18)
            }
        }
        CrossCutArbitrationResult::NoConflict => None,
    }
}

/// The agent contract of a Graph task: its role's contract, narrowed by the
/// task's `allowed_tools` and `denied_tools` and by its domain
/// ([`task_denied_tools`]). A task that names no `domain` takes
/// `project.default_domain` from `config`.
pub(super) fn effective_agent_contract(
    task_role: &str,
    task: &TaskDef,
    config: &RokoConfig,
) -> AgentContract {
    let task_allowed_tools = task
        .allowed_tools
        .as_deref()
        .filter(|tools| !tools.is_empty());
    let domain = task.effective_domain(config.project.default_domain.as_ref());
    let denied = task_denied_tools(task, domain.as_ref(), task_allowed_tools);
    AgentContract::load_for_role_with_mode(task_role, ContractLoadMode::RestrictedFallback)
        .unwrap_or_else(|_| AgentContract::restricted(task_role))
        .with_tool_restrictions(task_allowed_tools, Some(denied.as_slice()))
}

/// The tools a task in `domain` is denied: its own `denied_tools`, and the
/// built-in tools that belong to another domain
/// ([`roko_std::roles::DomainToolProfile::offers`]) unless it names them in
/// `allowed_tools`. Without a domain it counts as coding, so it is not
/// offered `chain.transfer`, `chain.swap` or any other `chain.*` tool.
fn task_denied_tools(
    task: &TaskDef,
    domain: Option<&TaskDomain>,
    task_allowed: Option<&[String]>,
) -> Vec<String> {
    let profile = domain_profile(domain.map_or("coding", TaskDomain::label));
    let named = task_allowed.unwrap_or_default();
    let registry = StaticToolRegistry::new();
    let other_domains = registry
        .all()
        .iter()
        .map(|tool| tool.name.as_str())
        .filter(|&tool| !profile.offers(tool) && !named.iter().any(|name| name == tool))
        .map(str::to_string);
    task.denied_tools
        .iter()
        .flatten()
        .cloned()
        .chain(other_domains)
        .collect()
}

pub(super) fn upstream_outputs(input: &[Signal]) -> Vec<(String, Vec<String>)> {
    input
        .iter()
        .enumerate()
        .filter_map(|(index, signal)| {
            signal
                .body
                .as_text()
                .ok()
                .map(|text| (format!("graph-upstream-{index}"), vec![text.to_string()]))
        })
        .collect()
}

/// P1-18: Convert persisted dream routing advice to a `RoutingBias` for the
/// cascade router. Returns `None` when no advice was loaded (missing or
/// stale file) or no recommendations match the task category.
pub(super) fn dream_routing_bias(
    advice: Option<&roko_dreams::DreamRoutingAdvice>,
    task_category: &str,
    routing_ctx: &roko_learn::model_router::RoutingContext,
) -> Option<roko_learn::cascade_router::RoutingBias> {
    let advice = advice?;
    if advice.recommendations.is_empty() {
        return None;
    }
    let complexity_band = routing_ctx.complexity.label();
    let bias = roko_dreams::dream_advice_to_routing_bias(advice, task_category, complexity_band);
    if bias.deprioritize.is_empty() {
        return None;
    }
    tracing::debug!(
        deprioritize = ?bias.deprioritize,
        reason = %bias.reason,
        "loaded dream routing bias for graph task dispatch"
    );
    Some(bias)
}

/// Whether a plan run dreams, so that dream routing advice may steer its
/// dispatches: `[learning] dream_on_completion` and
/// `dreams.trigger_on_plan_complete` are both on. Dreams are held
/// (dec-e70592), and the first is off by default (backlog 4207).
pub(super) const fn dreams_feed_plan_routing(learning: &roko_core::config::LearningConfig) -> bool {
    learning.dream_on_completion && learning.dreams.trigger_on_plan_complete
}

/// The dream routing advice a dispatch reads: none while plan runs do not
/// dream ([`dreams_feed_plan_routing`]), else the advice on disk, which
/// [`roko_dreams::load_dream_routing_advice`] empties once it is older than
/// [`roko_dreams::ROUTING_ADVICE_DEFAULT_TTL`], one hour.
pub(super) fn plan_dream_routing_advice(
    learning: &roko_core::config::LearningConfig,
    workdir: &Path,
) -> Option<roko_dreams::DreamRoutingAdvice> {
    if !dreams_feed_plan_routing(learning) {
        return None;
    }
    roko_dreams::load_dream_routing_advice(workdir).ok()
}

/// RAG-11: assign the retrieval-strategy arm from the experiment store.
///
/// Blocking file I/O: call it from `spawn_blocking`. Assignment is a pure
/// read of the persisted arm statistics. The store is written only to
/// register the experiment, and then under its lock, so the prompt
/// treatments parallel attempts record in the same file are never lost.
pub(super) fn assign_retrieval_strategy_arm(exp_path: &Path) -> String {
    use roko_learn::prompt_experiment::ExperimentStore;

    let mut store = ExperimentStore::load_or_new(exp_path);
    if store
        .get(ExperimentStore::RETRIEVAL_STRATEGY_EXPERIMENT_ID)
        .is_none()
    {
        store.ensure_retrieval_strategy_experiment();
        if let Err(error) = ExperimentStore::transaction(exp_path, |locked| {
            locked.ensure_retrieval_strategy_experiment();
            Ok(())
        }) {
            tracing::debug!(
                %error,
                "RAG-11: persisting the retrieval-strategy experiment failed (best-effort)"
            );
        }
    }
    store
        .assign_retrieval_strategy()
        .unwrap_or_else(|| roko_learn::retrieval_outcome::STRATEGY_KEYWORD.to_string())
}

/// Build a reasonable `RoutingContext` for Graph task dispatch.
///
/// This provides the cascade router with actionable task signals without
/// requiring the full runner-v2 internal state. The Graph engine has less
/// runtime state than the event loop, so fields like `active_agents` and
/// `ready_queue_depth` are set to sane defaults.
///
/// The task's authored `category`, `complexity_band` and `reasoning_level`
/// win over what its role and tier suggest. The context is a first
/// attempt's until [`mark_attempt`] says otherwise.
pub(super) fn build_routing_context(
    role: &str,
    task: &TaskDef,
    daimon_state: &Option<Arc<std::sync::Mutex<roko_daimon::DaimonState>>>,
) -> roko_learn::model_router::RoutingContext {
    use roko_core::agent::AgentRole;
    use roko_core::task::TaskCategory;
    use roko_learn::model_router::RoutingContext;

    let role_enum = match role.trim().to_ascii_lowercase().as_str() {
        "conductor" => AgentRole::Conductor,
        "strategist" => AgentRole::Strategist,
        "architect" => AgentRole::Architect,
        "researcher" => AgentRole::Researcher,
        "auditor" | "reviewer" => AgentRole::Auditor,
        "refactorer" => AgentRole::Refactorer,
        _ => AgentRole::Implementer,
    };

    // An authored category wins; otherwise derive it from the role,
    // defaulting to Implementation for most Graph engine work.
    let task_category = task.hints.category.unwrap_or(match role_enum {
        AgentRole::Researcher => TaskCategory::Research,
        AgentRole::Auditor => TaskCategory::Verification,
        AgentRole::Refactorer => TaskCategory::Refactor,
        AgentRole::Architect => TaskCategory::Scaffolding,
        _ => TaskCategory::Implementation,
    });

    // An authored band wins; otherwise the tier's band: mechanical is Fast,
    // focused (and any unknown tier) Standard, integrative and architectural
    // Complex.
    let complexity = task
        .hints
        .complexity_band
        .unwrap_or_else(|| task.tier_class().complexity_band());

    // Extract daimon policy if the affect state is loaded (`[daimon] enabled`,
    // 1211). It shifts the routing tier, and this log line is its record.
    let daimon_policy = daimon_state
        .as_ref()
        .and_then(|d| {
            d.lock().ok().map(|state| {
                use roko_daimon::AffectEngine;
                let affect = state.query();
                tracing::info!(
                    behavioral_state = ?affect.behavioral_state,
                    confidence = affect.confidence,
                    "affect shapes this dispatch's routing tier ([daimon] enabled)"
                );
                roko_core::DaimonPolicy::new(affect.confidence, affect.behavioral_state)
            })
        })
        .unwrap_or_default();

    RoutingContext {
        task_category,
        complexity,
        iteration: 0,
        role: role_enum,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 1,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy,
        thinking_level: task
            .hints
            .reasoning_level
            .map(|level| level.label().to_string()),
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    }
}

/// Make `routing` the context of `attempt` of `task` (0 is the first try).
///
/// The Graph engine retries a task only after an attempt failed, so a retry
/// has a prior failure. A task with `escalate_on_retry = true` asks the
/// router for the next complexity band on its retries.
pub(super) fn mark_attempt(
    routing: &mut roko_learn::model_router::RoutingContext,
    task: &TaskDef,
    attempt: u32,
) {
    routing.iteration = attempt;
    routing.has_prior_failure = attempt > 0;
    if attempt > 0 && task.hints.escalate_on_retry == Some(true) {
        routing.complexity = routing.complexity.escalate();
    }
}

#[cfg(test)]
mod tests {
    use roko_core::config::schema::ModelProfile;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        batch_ctx, cli_provider, make_bare_dispatcher, make_batch_dispatcher, make_spec,
        make_task_def, model,
    };

    /// The built-in tools `task`'s agent contract under `config` lets its
    /// agent use.
    fn offered_tools(task: &TaskDef, config: &RokoConfig) -> Vec<String> {
        let contract = effective_agent_contract("implementer", task, config);
        let registry = StaticToolRegistry::new();
        registry
            .all()
            .iter()
            .map(|tool| tool.name.clone())
            .filter(|name| contract.permits_tool(name))
            .collect()
    }

    /// Write dream routing advice, generated at `generated_at`, that
    /// deprioritises `model-x` for a focused implementer task.
    fn save_dream_advice(workdir: &Path, generated_at: chrono::DateTime<chrono::Utc>) {
        let routing = build_routing_context("implementer", &make_task_def("focused"), &None);
        let advice = roko_dreams::DreamRoutingAdvice {
            generated_at,
            recommendations: vec![roko_dreams::RoutingRecommendation {
                task_category: "implementation".to_string(),
                complexity_band: routing.complexity.label().to_string(),
                recommended_model: "model-y".to_string(),
                deprioritize: vec!["model-x".to_string()],
                confidence: 0.9,
                supporting_episodes: 5,
                recommended_model_success_rate: 0.8,
                pattern_signature: 1,
            }],
            ..roko_dreams::DreamRoutingAdvice::default()
        };
        roko_dreams::save_dream_routing_advice(workdir, &advice).expect("save dream advice");
    }

    /// The dream routing bias a focused implementer task's dispatch gets
    /// under `learning` in `workdir`.
    fn dream_bias(
        learning: &roko_core::config::LearningConfig,
        workdir: &Path,
    ) -> Option<roko_learn::cascade_router::RoutingBias> {
        let routing = build_routing_context("implementer", &make_task_def("focused"), &None);
        let advice = plan_dream_routing_advice(learning, workdir);
        dream_routing_bias(advice.as_ref(), "implementation", &routing)
    }

    /// A learning config under which plan runs dream.
    fn dreaming() -> roko_core::config::LearningConfig {
        let mut learning = roko_core::config::LearningConfig::default();
        learning.dream_on_completion = true;
        learning.dreams.trigger_on_plan_complete = true;
        learning
    }

    /// backlog 4207: fresh advice on disk steers no dispatch while plan runs
    /// do not dream, which is the default; a dreaming run reads it.
    #[test]
    fn dream_advice_ignored_while_dreams_are_held() {
        let temp = tempdir().expect("tempdir");
        save_dream_advice(temp.path(), chrono::Utc::now());
        let held = roko_core::config::LearningConfig::default();
        assert!(!dreams_feed_plan_routing(&held));
        assert!(dream_bias(&held, temp.path()).is_none());
        assert!(
            dream_bias(&dreaming(), temp.path()).is_some(),
            "fresh advice steers a dreaming run"
        );
    }

    /// backlog 4207: with dreams on, advice past its one-hour TTL yields no
    /// routing bias.
    #[test]
    fn stale_dream_advice_yields_no_routing_bias() {
        let temp = tempdir().expect("tempdir");
        save_dream_advice(temp.path(), chrono::Utc::now() - chrono::Duration::hours(2));
        assert!(dream_bias(&dreaming(), temp.path()).is_none());
    }

    /// gap-585bd2: the task's domain decides whether the `chain.*` tools,
    /// `chain.transfer` and `chain.swap` among them, are offered.
    #[test]
    fn a_coding_task_is_offered_no_chain_tools() {
        let config = RokoConfig::default();
        let mut task = make_task_def("focused");
        for domain in [None, Some(TaskDomain::Code), Some(TaskDomain::Research)] {
            task.domain = domain;
            let tools = offered_tools(&task, &config);
            let chain: Vec<&String> = tools
                .iter()
                .filter(|name| name.starts_with("chain."))
                .collect();
            assert!(chain.is_empty(), "{:?} is offered {chain:?}", task.domain);
            assert!(tools.iter().any(|name| name == "read_file"), "{tools:?}");
        }

        // A chain-domain task keeps them, and a coding task gets the one it
        // names. The catalog holds them when roko-std's `chain` feature is on.
        if cfg!(feature = "chain") {
            task.domain = Some(TaskDomain::Chain);
            let tools = offered_tools(&task, &config);
            for tool in ["chain.balance", "chain.get_pool_info", "chain.transfer"] {
                assert!(
                    tools.iter().any(|name| name == tool),
                    "{tool} missing: {tools:?}"
                );
            }

            // A task that names no domain takes `project.default_domain`.
            let mut chain_project = RokoConfig::default();
            chain_project.project.default_domain = Some(TaskDomain::Chain);
            task.domain = None;
            let tools = offered_tools(&task, &chain_project);
            let offers_balance = tools.iter().any(|name| name == "chain.balance");
            assert!(offers_balance, "{tools:?}");

            task.domain = Some(TaskDomain::Code);
            task.allowed_tools = Some(vec!["read_file".to_string(), "chain.balance".to_string()]);
            let mut tools = offered_tools(&task, &config);
            tools.sort();
            assert_eq!(tools, ["chain.balance", "read_file"]);
        }
    }

    /// gap-9cbf35: a plan task without a `model_hint` runs on the model of
    /// its `[routing.ladder]` start rung.
    #[tokio::test]
    async fn an_unhinted_task_runs_on_its_ladder_start_rung() {
        use roko_core::config::routing::LadderRung;

        let rung = |name: &str, model: &str| LadderRung {
            name: name.to_string(),
            model: model.to_string(),
        };
        for (tier, slug) in [
            ("mechanical", "claude-haiku-4-5"),
            ("architectural", "claude-sonnet-4-6"),
        ] {
            let temp = tempdir().expect("tempdir");
            let (dispatcher, mut task) = make_batch_dispatcher(&temp, 0.01, |config| {
                config.models.insert(
                    "cheap-model".to_string(),
                    model("batch-cli", "claude-haiku-4-5", None),
                );
                config.routing.ladder.rungs =
                    vec![rung("cheap", "cheap-model"), rung("top", "batch-model")];
            })
            .await;
            task.model_hint = None;
            task.tier = tier.to_string();
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
                .await
                .expect("dispatch");
            let args = std::fs::read_to_string(temp.path().join("provider-args"))
                .expect("the provider recorded its arguments");
            assert!(
                args.contains(&format!("--model {slug}")),
                "{tier}: provider args: {args}"
            );
        }
    }

    /// gap-8c0a20: every consumer reads a plan tier through `TaskTier`, so
    /// for each spelling of a tier the routing band, budget multiplier, turn
    /// cap and express eligibility agree.
    #[test]
    fn plan_tiers_reach_router_budget_and_turn_caps() {
        use roko_core::task::{TaskComplexityBand, TaskTier};

        let mut config = RokoConfig::default();
        config.conductor.express_mode = true;
        config.budget.max_task_usd = 1.0;
        config.budget.max_task_retry_usd = 0.0;
        let expected = [
            (
                &["mechanical", "trivial", "fast", " T0 "][..],
                TaskTier::Mechanical,
                TaskComplexityBand::Fast,
                0.2,
                40,
                true,
            ),
            (
                &["focused", "standard", "t1"][..],
                TaskTier::Focused,
                TaskComplexityBand::Standard,
                1.0,
                60,
                false,
            ),
            (
                &["integrative", "Complex", "2"][..],
                TaskTier::Integrative,
                TaskComplexityBand::Complex,
                3.0,
                90,
                false,
            ),
            (
                &["architectural", "premium", "deep"][..],
                TaskTier::Architectural,
                TaskComplexityBand::Complex,
                5.0,
                120,
                false,
            ),
            // A missing or misspelt tier reads as focused everywhere.
            (
                &["", "mechancial"][..],
                TaskTier::Focused,
                TaskComplexityBand::Standard,
                1.0,
                60,
                false,
            ),
        ];
        for (spellings, tier, band, budget_multiplier, turn_cap, express) in expected {
            for spelling in spellings {
                let task = make_task_def(spelling);
                assert_eq!(task.tier_class(), tier, "{spelling:?}");
                let routing = build_routing_context("implementer", &task, &None);
                assert_eq!(routing.complexity, band, "router band of {spelling:?}");
                let ceiling = task_budget_ceiling_usd(&config.budget, &task);
                assert!(
                    (ceiling - budget_multiplier).abs() < 1e-6,
                    "budget of {spelling:?}: {ceiling}"
                );
                assert_eq!(
                    task_turn_limit(&config, &task, false),
                    turn_cap,
                    "turn cap of {spelling:?}"
                );
                assert_eq!(
                    is_express_task(&config, &task),
                    express,
                    "express eligibility of {spelling:?}"
                );
            }
        }
    }

    fn parse_task(extra_keys: &str) -> TaskDef {
        crate::task_parser::TasksFile::parse_str(&format!(
            "[meta]\nplan = \"p\"\n\n[[task]]\nid = \"T01\"\ntitle = \"Check the parser\"\n\
             role = \"implementer\"\n{extra_keys}"
        ))
        .expect("parse the task")
        .tasks
        .remove(0)
    }

    /// gap-0f3980: a task's authored category, complexity band and reasoning
    /// level beat what its role and tier suggest.
    #[test]
    fn routing_context_uses_authored_task_metadata() {
        use roko_core::task::{TaskCategory, TaskComplexityBand};

        let hinted = parse_task(
            "category = \"verification\"\ncomplexity_band = \"complex\"\n\
             reasoning_level = \"high\"\n",
        );
        let routing = build_routing_context("implementer", &hinted, &None);
        assert_eq!(routing.task_category, TaskCategory::Verification);
        assert_eq!(routing.complexity, TaskComplexityBand::Complex);
        assert_eq!(routing.thinking_level.as_deref(), Some("high"));

        // Without hints the role and the (focused) tier decide, as before.
        let routing = build_routing_context("implementer", &parse_task(""), &None);
        assert_eq!(routing.task_category, TaskCategory::Implementation);
        assert_eq!(routing.complexity, TaskComplexityBand::Standard);
        assert_eq!(routing.thinking_level, None);
    }

    /// gap-b62e95: a retry's context says so, and `escalate_on_retry` asks
    /// for the next complexity band on retries only.
    #[test]
    fn mark_attempt_flags_retries_and_escalates_on_request() {
        use roko_core::task::TaskComplexityBand;

        let task = parse_task("tier = \"mechanical\"\n");
        let mut routing = build_routing_context("implementer", &task, &None);
        mark_attempt(&mut routing, &task, 0);
        assert_eq!((routing.iteration, routing.has_prior_failure), (0, false));
        mark_attempt(&mut routing, &task, 2);
        assert_eq!((routing.iteration, routing.has_prior_failure), (2, true));
        assert_eq!(routing.complexity, TaskComplexityBand::Fast);

        let escalating = parse_task("tier = \"mechanical\"\nescalate_on_retry = true\n");
        let first = {
            let mut routing = build_routing_context("implementer", &escalating, &None);
            mark_attempt(&mut routing, &escalating, 0);
            routing.complexity
        };
        let retry = {
            let mut routing = build_routing_context("implementer", &escalating, &None);
            mark_attempt(&mut routing, &escalating, 1);
            routing.complexity
        };
        assert_eq!(
            (first, retry),
            (TaskComplexityBand::Fast, TaskComplexityBand::Standard)
        );
    }

    fn config_with_models(models: Vec<(&str, ModelProfile)>) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.models.clear();
        config.routing.fast_task_model = String::new();
        for (key, profile) in models {
            config.models.insert(key.to_string(), profile);
        }
        config
    }

    /// gap-0f3980: `preferred_provider` picks which provider's entry runs the
    /// routed model, when it has a usable one.
    #[test]
    fn preferred_provider_picks_that_providers_entry_for_the_model() {
        let mut config = config_with_models(vec![
            ("sonnet-cli", model("claude_cli", "claude-sonnet-4-6", None)),
            ("sonnet-api", model("anthropic", "claude-sonnet-4-6", None)),
            ("mini", model("openai", "gpt-4o-mini", None)),
        ]);
        let pick = |config: &RokoConfig, routed: &str, provider: Option<&str>| {
            preferred_provider_model_with(config, routed, provider, |key| key != "offline")
        };
        // By slug or by key, the preferred provider's entry runs the model.
        assert_eq!(
            pick(&config, "claude-sonnet-4-6", Some("anthropic")),
            "sonnet-api"
        );
        assert_eq!(pick(&config, "sonnet-cli", Some("anthropic")), "sonnet-api");
        // No preference, or a provider without that model: as routed.
        assert_eq!(
            pick(&config, "claude-sonnet-4-6", None),
            "claude-sonnet-4-6"
        );
        assert_eq!(
            pick(&config, "gpt-4o-mini", Some("anthropic")),
            "gpt-4o-mini"
        );
        // An unusable entry is never picked.
        config.routing.disabled_providers = vec!["anthropic".to_string()];
        assert_eq!(pick(&config, "sonnet-cli", Some("anthropic")), "sonnet-cli");
        config.routing.disabled_providers.clear();
        config
            .models
            .get_mut("sonnet-api")
            .expect("entry")
            .supports_tools = false;
        assert_eq!(pick(&config, "sonnet-cli", Some("anthropic")), "sonnet-cli");
        config
            .models
            .get_mut("sonnet-api")
            .expect("entry")
            .supports_tools = true;
        assert_eq!(
            preferred_provider_model_with(&config, "sonnet-cli", Some("anthropic"), |_| false),
            "sonnet-cli"
        );
    }

    #[test]
    fn cheap_model_prefers_a_dispatchable_fast_task_model() {
        let mut config = config_with_models(vec![
            ("mini", model("openai", "gpt-4o-mini", Some((0.15, 0.6)))),
            ("haiku", model("claude_cli", "claude-haiku-4-5", None)),
        ]);
        config.routing.fast_task_model = "claude-haiku-4-5".to_string();
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("haiku"),
            "fast_task_model matches by slug"
        );
        config.routing.fast_task_model = "haiku".to_string();
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("haiku"),
            "fast_task_model matches by key"
        );
        assert_eq!(
            select_cheap_model_key_with(&config, |key| key != "haiku").as_deref(),
            Some("mini"),
            "an undispatchable fast_task_model falls back to the cheapest model"
        );
        config.routing.fast_task_model = "not-configured".to_string();
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("mini")
        );
    }

    #[test]
    fn cheap_model_ranks_by_input_then_output_price_not_by_name() {
        let config = config_with_models(vec![
            ("a-expensive", model("p", "aaa-large", Some((3.0, 15.0)))),
            ("b-cheap-input", model("p", "bbb", Some((0.10, 2.0)))),
            ("c-cheapest", model("p", "ccc", Some((0.10, 0.40)))),
            ("d-unpriced", model("p", "zz-unknown-model", None)),
        ]);
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("c-cheapest")
        );
    }

    #[test]
    fn cheap_model_prices_unpriced_claude_models_from_the_builtin_table() {
        // The dogfood shape: Claude CLI models with no `cost_*` keys. The old
        // alphabetical pick returned the first key, not the cheapest model.
        let config = config_with_models(vec![
            ("claude-opus", model("claude_cli", "claude-opus-4-6", None)),
            (
                "claude-sonnet",
                model("claude_cli", "claude-sonnet-4-6", None),
            ),
            ("zeta-haiku", model("claude_cli", "claude-haiku-4-5", None)),
        ]);
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("zeta-haiku")
        );
    }

    #[test]
    fn cheap_model_skips_embedding_search_disabled_and_offline_models() {
        let mut embedding = model("p", "text-embedding-3-small", Some((0.02, 0.0)));
        embedding.is_embedding_model = true;
        let mut search = model("perplexity", "sonar", Some((0.01, 0.01)));
        search.supports_tools = false;
        let mut config = config_with_models(vec![
            ("embed", embedding),
            ("sonar", search),
            (
                "disabled",
                model("groq", "llama-3.3-70b", Some((0.05, 0.05))),
            ),
            (
                "offline",
                model("p", "cheap-but-offline", Some((0.01, 0.01))),
            ),
            ("ok", model("p", "gpt-4o-mini", Some((0.15, 0.6)))),
        ]);
        config.routing.disabled_providers = vec!["groq".to_string()];
        assert_eq!(
            select_cheap_model_key_with(&config, |key| key != "offline").as_deref(),
            Some("ok")
        );
        assert_eq!(select_cheap_model_key_with(&config, |_| false), None);
    }

    #[tokio::test]
    async fn cheap_agent_uses_llm_call_timeout_and_the_selected_model_key() {
        let temp = tempdir().expect("tempdir");
        let mut config = config_with_models(vec![(
            "cli-sonnet",
            model("cli", "claude-sonnet-4-6", None),
        )]);
        config.providers.clear();
        config
            .providers
            .insert("cli".to_string(), cli_provider("/bin/sh"));
        config.timeouts.llm_call_secs = 7;
        let dispatcher = make_bare_dispatcher(config, temp.path()).await;

        let agent = dispatcher.cheap_agent().expect("a dispatchable model");
        assert_eq!(agent.model_key, "cli-sonnet");
        assert_eq!(agent.timeout_ms, 7_000);
    }

    #[test]
    fn retrieval_strategy_assignment_rewrites_the_store_only_on_registration() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("experiments.json");
        let arm = assign_retrieval_strategy_arm(&path);
        assert!(path.is_file(), "registration persists the experiment");

        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open store")
            .set_modified(old)
            .expect("backdate store");
        assert_eq!(assign_retrieval_strategy_arm(&path), arm);
        assert_eq!(
            std::fs::metadata(&path)
                .expect("store metadata")
                .modified()
                .expect("mtime"),
            old,
            "a steady-state dispatch must not rewrite the store"
        );
    }
}
