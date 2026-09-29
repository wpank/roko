//! Model routing inputs of a Graph task dispatch: the cheap helper model, cross-cut
//! and dream routing bias, the agent contract, and the routing context.

use super::*;

/// Thin `Agent` adapter that forwards a one-shot prompt through the shared
/// factory bridge so `error_enrichment` and `quality_judge` can use the
/// live provider without rebuilding the full dispatch stack.
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

/// Choose the model for best-effort one-shot helper calls (quality judge,
/// error enrichment, gate reflections).
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

pub(super) fn effective_agent_contract(task_role: &str, task: &TaskDef) -> AgentContract {
    let task_allowed_tools = task
        .allowed_tools
        .as_deref()
        .filter(|tools| !tools.is_empty());
    AgentContract::load_for_role_with_mode(task_role, ContractLoadMode::RestrictedFallback)
        .unwrap_or_else(|_| AgentContract::restricted(task_role))
        .with_tool_restrictions(task_allowed_tools, task.denied_tools.as_deref())
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
pub(super) fn build_routing_context(
    role: &str,
    task: &TaskDef,
    daimon_state: &Option<Arc<std::sync::Mutex<roko_daimon::DaimonState>>>,
) -> roko_learn::model_router::RoutingContext {
    use roko_core::agent::AgentRole;
    use roko_core::task::{TaskCategory, TaskComplexityBand};
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

    // Derive task category from the role or task type, defaulting to
    // Implementation for most Graph engine work.
    let task_category = match role_enum {
        AgentRole::Researcher => TaskCategory::Research,
        AgentRole::Auditor => TaskCategory::Verification,
        AgentRole::Refactorer => TaskCategory::Refactor,
        AgentRole::Architect => TaskCategory::Scaffolding,
        _ => TaskCategory::Implementation,
    };

    // Infer complexity from the task tier field, or default to Standard.
    let complexity = match task.tier.trim().to_ascii_lowercase().as_str() {
        "fast" | "t0" | "0" => TaskComplexityBand::Fast,
        "complex" | "t2" | "2" | "premium" => TaskComplexityBand::Complex,
        _ => TaskComplexityBand::Standard,
    };

    // Extract daimon policy if the affect state is loaded.
    let daimon_policy = daimon_state
        .as_ref()
        .and_then(|d| {
            d.lock().ok().map(|state| {
                use roko_daimon::AffectEngine;
                let affect = state.query();
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
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    }
}

#[cfg(test)]
mod tests {
    use roko_core::config::schema::ModelProfile;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{cli_provider, make_bare_dispatcher, model};

    fn config_with_models(models: Vec<(&str, ModelProfile)>) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.models.clear();
        config.routing.fast_task_model = String::new();
        for (key, profile) in models {
            config.models.insert(key.to_string(), profile);
        }
        config
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
