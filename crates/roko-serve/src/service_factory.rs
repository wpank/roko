//! Shared workflow service construction for CLI, server, and ACP.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use roko_agent::{GatewayEventWriter, InferenceObserver, ModelCallService, ProviderRateLimiter};
use roko_compose::prompt_assembly_service::PromptAssemblyService;
use roko_core::agent::resolve_model;
use roko_core::config::schema::{RokoConfig, ToolsConfig};
use roko_core::foundation::{
    AffectPolicy, FeedbackEvent, FeedbackSink, GateRunner, ModelCaller, PromptAssembler,
};
use roko_core::{AgentRole, Result, RokoError, RuntimeEvent};
use roko_daimon::policy::DaimonPolicy;
use roko_gate::gate_service::GateService;
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::feedback_service::FeedbackService;
use roko_learn::model_router::RoutingContext;
use roko_learn::playbook::PlaybookStore;
use roko_learn::provider_health::ProviderHealthRegistry;
use roko_learn::section_effect::SectionEffectivenessRegistry;
use roko_neuro::knowledge_store::KnowledgeStore;
use roko_runtime::{JsonlLogger, effect_driver::EffectServices};

#[derive(Debug, Default)]
struct RuntimeBusInferenceObserver;

impl InferenceObserver for RuntimeBusInferenceObserver {
    fn on_runtime_event_with_cursor(&self, event: &RuntimeEvent, cursor: Option<u64>) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(event.clone(), cursor);
    }

    fn on_start(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        auto_routed: bool,
    ) {
        self.on_start_with_cursor(run_id, request_id, model, agent_id, auto_routed, None);
    }

    fn on_start_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        auto_routed: bool,
        cursor: Option<u64>,
    ) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(
            RuntimeEvent::InferenceStarted {
                run_id: run_id.to_string(),
                request_id: request_id.to_string(),
                model: model.to_string(),
                agent_id: agent_id.to_string(),
                auto_routed,
            },
            cursor,
        );
    }

    fn on_complete(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
    ) {
        self.on_complete_with_cursor(
            run_id,
            request_id,
            model,
            agent_id,
            input_tokens,
            output_tokens,
            cost_usd,
            duration_ms,
            None,
        );
    }

    fn on_complete_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
        cursor: Option<u64>,
    ) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(
            RuntimeEvent::InferenceCompleted {
                run_id: run_id.to_string(),
                request_id: request_id.to_string(),
                model: model.to_string(),
                agent_id: agent_id.to_string(),
                input_tokens,
                output_tokens,
                cost_usd,
                duration_ms,
            },
            cursor,
        );
    }

    fn on_error(&self, run_id: &str, request_id: &str, model: &str, agent_id: &str, error: &str) {
        self.on_error_with_cursor(run_id, request_id, model, agent_id, error, None);
    }

    fn on_error_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        error: &str,
        cursor: Option<u64>,
    ) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(
            RuntimeEvent::InferenceFailed {
                run_id: run_id.to_string(),
                request_id: request_id.to_string(),
                model: model.to_string(),
                agent_id: agent_id.to_string(),
                error: error.to_string(),
            },
            cursor,
        );
    }
}

/// Input settings for constructing shared workflow services.
#[derive(Clone)]
pub struct ServiceConfig {
    /// Workspace root used by service implementations.
    pub workdir: PathBuf,
    /// `.roko` directory used for persistent service state.
    pub roko_dir: PathBuf,
    /// Runtime workspace configuration for model/provider dispatch.
    pub workspace_config: RokoConfig,
    /// Optional model key or slug overriding `workspace_config.agent.default_model`.
    pub model_key: Option<String>,
    /// Optional MCP config passed into model provider construction.
    pub mcp_config: Option<PathBuf>,
    /// Whether feedback should persist through `FeedbackService`.
    pub feedback_enabled: bool,
    /// Whether affect modulation should be backed by Daimon state.
    pub affect_enabled: bool,
    /// Whether cascade routing and cascade learning should be active.
    pub cascade_enabled: bool,
    /// Stable run id used by service-level event and feedback records.
    pub run_id: Option<String>,
    /// Optional inference observer for RuntimeEvent emission around model calls.
    pub inference_observer: Option<Arc<dyn InferenceObserver>>,
    /// Optional metric registry for emitting LLM call/error/token/cost metrics.
    pub metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
}

impl ServiceConfig {
    /// Build a production service config from a workspace root and Roko config.
    #[must_use]
    pub fn production(workdir: impl Into<PathBuf>, workspace_config: RokoConfig) -> Self {
        let workdir = workdir.into();
        Self {
            roko_dir: workdir.join(".roko"),
            workdir,
            workspace_config,
            model_key: None,
            mcp_config: None,
            feedback_enabled: true,
            affect_enabled: true,
            cascade_enabled: true,
            run_id: None,
            inference_observer: Some(Arc::new(RuntimeBusInferenceObserver)),
            metrics: None,
        }
    }
}

/// Concrete service bundle shared by all runtime entry points.
pub struct ServiceBundle {
    /// Resolved default model slug.
    pub model: String,
    /// Concrete model-call gateway used by HTTP inference and workflow effects.
    pub model_call_service: Arc<ModelCallService>,
    /// Canonical persisted circuit-breaker state shared by routing and APIs.
    pub provider_health_registry: Arc<ProviderHealthRegistry>,
    /// Prompt assembly service exposed as the foundation trait.
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    /// Feedback service exposed as the foundation trait.
    pub feedback_sink: Arc<dyn FeedbackSink>,
    /// Gate execution service exposed as the foundation trait.
    pub gate_runner: Arc<dyn GateRunner>,
    /// Optional affect policy shared with the effect driver.
    pub affect_policy: Option<Arc<tokio::sync::Mutex<dyn AffectPolicy>>>,
}

impl ServiceBundle {
    /// Build the `EffectServices` value consumed by `WorkflowEngine`.
    #[must_use]
    pub fn effect_services(&self) -> EffectServices {
        let model_caller: Arc<dyn ModelCaller> = self.model_call_service.clone();
        EffectServices {
            default_model: self.model.clone(),
            model_caller,
            prompt_assembler: Arc::clone(&self.prompt_assembler),
            feedback_sink: Arc::clone(&self.feedback_sink),
            gate_runner: Arc::clone(&self.gate_runner),
            affect_policy: self.affect_policy.clone(),
        }
    }
}

/// Factory for constructing the shared service bundle.
pub struct ServiceFactory;

impl ServiceFactory {
    /// Construct all workflow services through the canonical path.
    pub fn build(config: ServiceConfig) -> Result<ServiceBundle> {
        let mut workspace_config = config.workspace_config;
        let model_key = config
            .model_key
            .clone()
            .unwrap_or_else(|| workspace_config.agent.default_model.clone());
        if model_key.trim().is_empty() {
            return Err(RokoError::invalid(
                "model is not configured for service factory",
            ));
        }
        let resolved_model = resolve_model(&workspace_config, &model_key);
        let model_context_window_tokens = context_window_tokens_from_resolved(&resolved_model);
        let model = resolved_model.slug;
        if model.trim().is_empty() {
            return Err(RokoError::invalid(format!(
                "model key {model_key:?} resolved to an empty model slug"
            )));
        }
        workspace_config.agent.default_model = model.clone();
        let prompt_token_budget = workspace_config.budget.prompt_token_budget;
        let tool_instructions = tool_instructions_for_config(&workspace_config.tools);
        let cascade_router = if config.cascade_enabled {
            let cascade_model_slugs = model_slugs_for_config(&workspace_config, &model);
            Some(Arc::new(CascadeRouter::load_or_new(
                &config.roko_dir.join("learn").join("cascade-router.json"),
                cascade_model_slugs,
            )))
        } else {
            None
        };
        let knowledge_store = Arc::new(KnowledgeStore::for_roko_dir(&config.roko_dir));

        let feedback_sink: Arc<dyn FeedbackSink> = if config.feedback_enabled {
            let feedback_service = FeedbackService::from_roko_dir_with_episodes(&config.roko_dir);
            match &cascade_router {
                Some(router) => Arc::new(feedback_service.with_cascade_router(Arc::clone(router))),
                None => Arc::new(feedback_service),
            }
        } else {
            Arc::new(MemoryFeedbackSink::default())
        };

        // knowledge_store implements KnowledgeQuery via the concrete impl
        // in roko-neuro (KnowledgeStore -> KnowledgeQuery).
        let gateway_knowledge_query: Arc<dyn roko_core::KnowledgeQuery> =
            knowledge_store.clone() as Arc<dyn roko_core::KnowledgeQuery>;
        let cost_table = roko_agent::CostTable::from_config_with_defaults(&workspace_config.models);
        let provider_health_registry = Arc::new(ProviderHealthRegistry::load_or_new(
            &config.roko_dir.join("learn").join("provider-health.json"),
        ));
        let health_checker: Arc<dyn roko_agent::ProviderHealthChecker> =
            provider_health_registry.clone();
        let rate_limiter = Arc::new(
            ProviderRateLimiter::from_provider_configs(
                60,
                workspace_config.effective_providers().iter(),
            )
            .with_health_registry(health_checker),
        );
        let routing_config = workspace_config.clone();
        let routing_health_registry = Arc::clone(&provider_health_registry);
        let prompt_model_router = cascade_router.clone();
        let prompt_routing_config = workspace_config.clone();
        let prompt_health_registry = Arc::clone(&provider_health_registry);
        let mut model_call_service = ModelCallService::new(model.clone())
            .with_config(workspace_config)
            .with_working_dir(&config.workdir)
            .with_immune_root(&config.workdir)
            .with_cost_table(cost_table)
            .with_feedback_sink(Arc::clone(&feedback_sink))
            .with_gateway_event_writer(Arc::new(GatewayEventWriter::for_workdir(&config.workdir)))
            .with_event_consumer(Arc::new(JsonlLogger::from_roko_dir(&config.roko_dir)))
            .with_knowledge_store(gateway_knowledge_query)
            .with_provider_outcome_recorder(Arc::clone(&provider_health_registry))
            .with_rate_limiter(rate_limiter)
            .with_run_id(config.run_id.unwrap_or_else(default_run_id));
        if let Some(cascade_router) = cascade_router {
            let model_router = Some(Arc::clone(&cascade_router));
            model_call_service = model_call_service
                .with_cascade_router(cascade_router)
                .with_model_router(move |role| {
                    routed_model_for_role(
                        &routing_config,
                        model_router.as_ref(),
                        Some(routing_health_registry.as_ref()),
                        agent_role_from_label(role.unwrap_or("implementer")),
                    )
                });
        }
        if let Some(observer) = config.inference_observer {
            model_call_service = model_call_service.with_inference_observer(observer);
        }
        let has_mcp = config.mcp_config.is_some();
        if let Some(mcp_config) = config.mcp_config {
            model_call_service = model_call_service.with_mcp_config(mcp_config);
        }
        if let Some(metrics) = config.metrics {
            model_call_service = model_call_service.with_metrics(metrics);
        }
        let model_call_service = Arc::new(model_call_service);

        let playbook_store = Arc::new(PlaybookStore::new(
            config.roko_dir.join("learn").join("playbooks"),
        ));
        let section_effectiveness = SectionEffectivenessRegistry::load_or_new(
            &config.roko_dir.join("learn").join("section-effects.json"),
        )
        .lift_weights();

        let mut prompt_service = PromptAssemblyService::new()
            .with_model_context_window(model_context_window_tokens)
            .with_model_context_window_resolver(move |role| {
                let selected_model = routed_model_for_role(
                    &prompt_routing_config,
                    prompt_model_router.as_ref(),
                    Some(prompt_health_registry.as_ref()),
                    role,
                );
                context_window_tokens_for_model(&prompt_routing_config, &selected_model)
            })
            .with_knowledge_store(knowledge_store)
            .with_episodes(config.roko_dir.join("episodes.jsonl"))
            .with_playbooks(playbook_store);
        if prompt_token_budget > 0 {
            prompt_service = prompt_service.with_token_budget(prompt_token_budget);
        }
        if let Some(tools) = tool_instructions {
            prompt_service = prompt_service.with_tool_instructions(tools);
        }
        if has_mcp {
            prompt_service = prompt_service.with_mcp_tools();
        }
        if !section_effectiveness.is_empty() {
            prompt_service = prompt_service.with_section_effectiveness(section_effectiveness);
        }

        let prompt_assembler: Arc<dyn PromptAssembler> = Arc::new(prompt_service);
        let gate_runner: Arc<dyn GateRunner> = Arc::new(GateService::new());
        let affect_policy = config.affect_enabled.then(|| {
            // Canonical path: .roko/daimon/affect.json (matches serve).
            // Fall back to legacy .roko/state/daimon.json for old workspaces
            // that haven't migrated yet.
            let canonical = config.roko_dir.join("daimon").join("affect.json");
            let state_path = if canonical.exists() {
                canonical
            } else {
                let legacy = config.roko_dir.join("state").join("daimon.json");
                if legacy.exists() {
                    // Migrate: copy legacy to canonical location.
                    if let Some(parent) = canonical.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::copy(&legacy, &canonical);
                    canonical
                } else {
                    canonical
                }
            };
            Arc::new(tokio::sync::Mutex::new(DaimonPolicy::new(state_path)))
                as Arc<tokio::sync::Mutex<dyn AffectPolicy>>
        });

        Ok(ServiceBundle {
            model,
            model_call_service,
            provider_health_registry,
            prompt_assembler,
            feedback_sink,
            gate_runner,
            affect_policy,
        })
    }
}

fn routed_model_for_role(
    config: &RokoConfig,
    router: Option<&Arc<CascadeRouter>>,
    health: Option<&ProviderHealthRegistry>,
    role: AgentRole,
) -> String {
    let Some(router) = router else {
        return resolve_model(config, &config.agent.default_model).slug;
    };
    let ctx = RoutingContext {
        role,
        ..Default::default()
    };
    let mut candidates = config.available_model_slugs_for_cascade();
    if candidates.is_empty() {
        candidates = config.model_slugs_for_cascade();
    }
    if let Some(health) = health {
        let model_providers = model_provider_map(config, &candidates);
        candidates = router.filter_unhealthy(&candidates, health, &model_providers);
    }
    router.explain_routing(&ctx, &candidates).selected_model
}

fn model_provider_map(config: &RokoConfig, candidates: &[String]) -> HashMap<String, String> {
    let profiles = config.effective_models();
    candidates
        .iter()
        .filter_map(|slug| {
            profiles
                .values()
                .find(|profile| profile.slug == *slug)
                .map(|profile| (slug.clone(), profile.provider.clone()))
        })
        .collect()
}

fn context_window_tokens_for_model(config: &RokoConfig, model_key: &str) -> usize {
    let resolved = resolve_model(config, model_key);
    context_window_tokens_from_resolved(&resolved)
}

fn context_window_tokens_from_resolved(resolved: &roko_core::agent::ResolvedModel) -> usize {
    resolved
        .profile
        .as_ref()
        .and_then(|profile| usize::try_from(profile.context_window).ok())
        .unwrap_or(128_000)
}

fn tool_instructions_for_config(tools: &ToolsConfig) -> Option<String> {
    if tools.allow.is_empty() && tools.deny.is_empty() && tools.profiles.is_empty() {
        return None;
    }

    let mut lines = Vec::new();
    if !tools.allow.is_empty() {
        lines.push(format!("Allowed tools: {}", tools.allow.join(", ")));
    }
    if !tools.deny.is_empty() {
        lines.push(format!("Denied tools: {}", tools.deny.join(", ")));
    }
    let mut profiles = tools.profiles.iter().collect::<Vec<_>>();
    profiles.sort_by_key(|(left, _)| *left);
    for (name, profile) in profiles {
        let mut parts = Vec::new();
        if !profile.extra_tools.is_empty() {
            parts.push(format!("extra: {}", profile.extra_tools.join(", ")));
        }
        if !profile.excluded_tools.is_empty() {
            parts.push(format!("excluded: {}", profile.excluded_tools.join(", ")));
        }
        if !parts.is_empty() {
            lines.push(format!("{name} profile tools: {}", parts.join("; ")));
        }
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn model_slugs_for_config(config: &RokoConfig, default_model: &str) -> Vec<String> {
    let mut slugs = Vec::new();
    push_model_slug(config, &mut slugs, default_model);

    if let Some(fallback) = config.agent.fallback_model.as_deref() {
        push_model_slug(config, &mut slugs, fallback);
    }

    let mut tier_models = config.agent.tier_models.values().collect::<Vec<_>>();
    tier_models.sort();
    for tier_model in tier_models {
        push_model_slug(config, &mut slugs, tier_model);
    }

    let mut configured_models = config
        .effective_models()
        .into_values()
        .map(|profile| profile.slug)
        .collect::<Vec<_>>();
    configured_models.sort();
    for slug in configured_models {
        push_model_slug(config, &mut slugs, &slug);
    }

    slugs
}

fn push_model_slug(config: &RokoConfig, slugs: &mut Vec<String>, model_key: &str) {
    let slug = resolve_model(config, model_key).slug;
    if !slug.trim().is_empty() && !slugs.contains(&slug) {
        slugs.push(slug);
    }
}

fn agent_role_from_label(label: &str) -> AgentRole {
    let normalized = label.trim().to_ascii_lowercase();
    if normalized == AgentRole::Conductor.label() {
        return AgentRole::Conductor;
    }
    AgentRole::ALL_AGENTS
        .iter()
        .copied()
        .find(|role| role.label() == normalized)
        .unwrap_or(AgentRole::Implementer)
}

#[derive(Default)]
struct MemoryFeedbackSink {
    events: tokio::sync::Mutex<Vec<FeedbackEvent>>,
}

#[async_trait]
impl FeedbackSink for MemoryFeedbackSink {
    async fn record(&self, event: FeedbackEvent) -> Result<()> {
        self.events.lock().await.push(event);
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.events.lock().await.clear();
        Ok(())
    }
}

fn default_run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    format!("service_factory_{millis}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::agent::ProviderKind;
    use roko_core::config::provider::ProviderConfig;
    use roko_core::config::schema::ModelProfile;
    use roko_core::foundation::{CachePolicy, ChatMessage, MessageRole, ModelCallRequest};
    use roko_learn::provider_health::ErrorClass;
    use tempfile::TempDir;

    fn write_provider_script(tmp: &TempDir, name: &str, body: &str) -> PathBuf {
        let path = tmp.path().join(name);
        std::fs::write(&path, body).expect("write provider script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = std::fs::metadata(&path)
                .expect("provider script metadata")
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).expect("chmod provider script");
        }
        path
    }

    fn add_cli_model(
        config: &mut RokoConfig,
        provider_id: &str,
        model_key: &str,
        model_slug: &str,
        command: PathBuf,
    ) {
        config.providers.insert(
            provider_id.to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(command.display().to_string()),
                args: None,
                timeout_ms: Some(5_000),
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            model_key.to_string(),
            ModelProfile {
                provider: provider_id.to_string(),
                slug: model_slug.to_string(),
                ..Default::default()
            },
        );
    }

    fn service_config(tmp: &TempDir, workspace_config: RokoConfig) -> ServiceConfig {
        let mut config = ServiceConfig::production(tmp.path(), workspace_config);
        config.affect_enabled = false;
        config.cascade_enabled = false;
        config.feedback_enabled = false;
        config
    }

    #[test]
    fn production_services_publish_inference_lifecycle() {
        let tmp = TempDir::new().expect("tempdir");
        let config = ServiceConfig::production(tmp.path(), RokoConfig::default());

        assert!(config.inference_observer.is_some());
    }

    fn request() -> ModelCallRequest {
        ModelCallRequest {
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "exercise provider health wiring".to_string(),
            }],
            cache_policy: CachePolicy::Bypass,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn live_model_call_records_success_under_configured_provider_identity() {
        let tmp = TempDir::new().expect("tempdir");
        let script = write_provider_script(
            &tmp,
            "success-provider.sh",
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"provider-ok"}}'
"#,
        );

        let mut workspace_config = RokoConfig::default();
        workspace_config.providers.clear();
        workspace_config.models.clear();
        workspace_config.agent.default_model = "health-model".to_string();
        workspace_config.agent.fallback_model = None;
        workspace_config.agent.tier_models.clear();
        add_cli_model(
            &mut workspace_config,
            "health-provider",
            "health-model",
            "health-model-v1",
            script,
        );

        let bundle = ServiceFactory::build(service_config(&tmp, workspace_config))
            .expect("build live workflow services");
        assert_eq!(bundle.model, "health-model-v1");

        let response = bundle
            .model_call_service
            .call(request())
            .await
            .expect("live provider call succeeds");
        assert_eq!(response.model, "health-model-v1");
        assert_eq!(response.content, "provider-ok");

        let snapshot = bundle.provider_health_registry.snapshot();
        assert_eq!(snapshot.len(), 1, "one live attempt must produce one key");
        let health = snapshot
            .get("health-provider")
            .expect("configured provider identity recorded");
        assert_eq!(
            health.total_requests, 1,
            "outcome must not be double-counted"
        );
        assert_eq!(health.total_failures, 0);
        assert!(!snapshot.contains_key("health-model-v1"));
    }

    #[tokio::test]
    async fn live_fallback_records_failure_and_success_for_exact_attempt_identities() {
        let tmp = TempDir::new().expect("tempdir");
        let primary = write_provider_script(
            &tmp,
            "primary-provider.sh",
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' 'temporarily unavailable' >&2
exit 1
"#,
        );
        let fallback = write_provider_script(
            &tmp,
            "fallback-provider.sh",
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"fallback-ok"}}'
"#,
        );

        let mut workspace_config = RokoConfig::default();
        workspace_config.providers.clear();
        workspace_config.models.clear();
        workspace_config.agent.default_model = "primary-model".to_string();
        workspace_config.agent.fallback_model = Some("fallback-model".to_string());
        workspace_config.agent.tier_models.clear();
        add_cli_model(
            &mut workspace_config,
            "primary-provider",
            "primary-model",
            "primary-model-v1",
            primary,
        );
        add_cli_model(
            &mut workspace_config,
            "fallback-provider",
            "fallback-model",
            "fallback-model-v1",
            fallback,
        );

        let bundle = ServiceFactory::build(service_config(&tmp, workspace_config))
            .expect("build live workflow services");
        let response = bundle
            .model_call_service
            .call(request())
            .await
            .expect("fallback provider succeeds");
        assert_eq!(response.model, "fallback-model-v1");
        assert_eq!(response.content, "fallback-ok");

        let snapshot = bundle.provider_health_registry.snapshot();
        assert_eq!(snapshot.len(), 2);
        let primary_health = snapshot
            .get("primary-provider")
            .expect("failed primary identity recorded");
        assert_eq!(primary_health.total_requests, 1);
        assert_eq!(primary_health.total_failures, 1);
        assert_eq!(
            primary_health
                .failure_window
                .back()
                .map(|failure| failure.error_class),
            Some(ErrorClass::ServerError)
        );
        let fallback_health = snapshot
            .get("fallback-provider")
            .expect("successful fallback identity recorded");
        assert_eq!(fallback_health.total_requests, 1);
        assert_eq!(fallback_health.total_failures, 0);
        assert!(!snapshot.contains_key("primary-model-v1"));
        assert!(!snapshot.contains_key("fallback-model-v1"));
    }

    #[test]
    fn role_routing_excludes_open_providers() {
        let mut config = RokoConfig::default();
        config.models.clear();
        config.models.insert(
            "primary".to_string(),
            ModelProfile {
                provider: "unavailable-provider".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..Default::default()
            },
        );
        config.models.insert(
            "fallback".to_string(),
            ModelProfile {
                provider: "healthy-provider".to_string(),
                slug: "gemini-2.5-flash".to_string(),
                ..Default::default()
            },
        );
        config.agent.default_model = "primary".to_string();
        config.agent.fallback_model = Some("fallback".to_string());

        let router = Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".to_string(),
            "gemini-2.5-flash".to_string(),
        ]));
        let health = ProviderHealthRegistry::new();
        for _ in 0..3 {
            health.record_failure("unavailable-provider", ErrorClass::AuthFailure);
        }

        assert_eq!(
            routed_model_for_role(
                &config,
                Some(&router),
                Some(&health),
                AgentRole::Implementer,
            ),
            "gemini-2.5-flash"
        );
    }
}
