//! Dispatch — the runner's single entry point for agent invocation.
//!
//! ## Architectural role
//!
//! In the unified Roko model the runner is a [`Compose → Route → Act`] loop
//! over `Signal`s. The dispatch module owns the **Route → Act** seam:
//!
//! - **Route**: pick a `ModelSpec` for the task by consulting
//!   [`CascadeRouter`] + any task / config overrides
//!   ([`model_routing`]).
//! - **Compose**: assemble a fully structured prompt
//!   ([`PromptAssembler`] in [`prompt_builder`]).
//! - **Act**: launch the resolved provider through
//!   [`AgentDispatcherV2`] / `roko-agent` and return a normalized
//!   [`AgentOutcome`] ([`outcome`]).
//!
//! This file is intentionally small: it composes the four submodules into
//! a [`Dispatcher`] facade that the runner can call, and a
//! [`DispatchContext`] value object that carries all the per-call inputs.
//!
//! Importantly, no provider-specific logic lives here. Every
//! provider concern (CLI args, stream parsing, http transport) is owned by
//! [`roko_agent::provider`]. The dispatcher only sees a `ModelSpec` and a
//! prompt; the act-step calls into provider code through the shared
//! [`AgentDispatcherV2`] resolver.
//!
//! ## Test seam
//!
//! [`Dispatcher::dispatch`] is async-trait based on a thin
//! [`AgentResultBridge`] that hides the provider for testing. Production
//! callers wire in [`AgentDispatcherV2`]; tests can plug in a stub bridge.

pub mod factory;
pub mod model_routing;
pub mod outcome;
mod plugin_mcp;
pub mod prompt_builder;
pub mod prompt_cache;
pub mod rung_probe;
pub mod warm_pool;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use roko_agent::AgentRuntimeEvent;
use roko_core::agent::ModelSpec;
use roko_core::config::schema::RokoConfig;
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::model_router::RoutingContext;
use roko_learn::provider_health::ProviderHealthRegistry;
use roko_learn::routing_log::RoutingDecisionLog;
use tokio::sync::mpsc;

pub use factory::SharedAgentFactory;
pub use model_routing::{
    FallbackReason, LadderStartRung, ModelChoice, ModelChoiceSource, ModelRouter, RoutingInputs,
    RoutingLadder,
};
pub use outcome::{AgentOutcome, RunnerDispatchError};
pub use prompt_builder::{
    AssembledPrompt, GateFeedback, PromptAssembler, PromptContext, PromptDiagnostics,
    PromptExperimentAssignmentDiagnostic, ScoredSignalDiagnostic,
};
pub use prompt_cache::PromptCache;
pub use warm_pool::{WarmPool, WarmPoolStats};

pub use crate::dispatch_v2::AgentDispatchRequest;
use crate::dispatch_v2::ProviderRuntime;
use crate::dispatch_v2::{AgentDispatcherV2, CliProviderConfig, ProviderDispatchResolver};
use crate::task_parser::TaskDef;

/// Durable prompt-experiment identity and root-workspace store location for
/// one dispatch attempt.
///
/// The path is explicit because [`DispatchContext::workdir`] can point at an
/// attempt worktree while prompt experiment assignments belong to the root
/// workspace's `.roko/learn` store.
#[derive(Debug, Clone)]
pub struct PromptExperimentContext {
    /// Stable attempt identity used for idempotent assignment preparation.
    pub attempt_key: roko_learn::prompt_experiment::PromptAttemptKey,
    /// Absolute or root-resolved path to the durable `experiments.json` store.
    pub store_path: std::path::PathBuf,
}

// ─── Per-call value objects ────────────────────────────────────────────

/// Inputs the runner already has available for a single dispatch call.
///
/// `DispatchContext` is a *value object*: read-only, cheap to construct,
/// and carries the per-task knobs the dispatcher needs without requiring
/// the runner to thread through opaque references.
#[derive(Debug, Clone)]
pub struct DispatchContext {
    /// Plan id this task belongs to.
    pub plan_id: String,
    /// Logical role name (`"implementer"`, `"reviewer"`, ...).
    pub role: String,
    /// Working directory for the agent.
    pub workdir: std::path::PathBuf,
    /// Optional explicit model override from CLI / config (`task.model_hint`).
    /// Lower priority than `force_backend`; used as a soft suggestion from
    /// task authors. The model router consults this after `force_backend` but
    /// before the cascade router.
    pub model_hint: Option<String>,
    /// Highest-priority model slug override (manual operator decision).
    ///
    /// Populated from `RunConfig.cli_model_override`, which in turn comes
    /// from the unified global `--model` CLI flag (aliases: `--force-model`,
    /// `--force-backend`).
    ///
    /// When set, the model router returns this slug immediately with
    /// `ModelChoiceSource::Override`, skipping task hints and the cascade
    /// router entirely. Feedback writers tag the outcome as `forced = true`
    /// so the router's learned policy is not corrupted by operator overrides.
    pub force_backend: Option<String>,
    /// Remaining USD budget for the plan. Routing does not read it: no
    /// routing bias is left (decision 3108).
    pub budget_remaining_usd: f64,
    /// Attempt number for this task (0 = first try, > 0 = retry).
    pub attempt: u32,
    /// Rungs above its start rung on `[routing.ladder]` this attempt
    /// climbs, after the task's agent-blamed failures (gap-460230). `0`
    /// routes on the start rung; pinned models never move.
    pub ladder_step: u32,
    /// Attempt-scoped durable prompt experiment context, when experiments are
    /// enabled for this dispatch.
    pub prompt_experiment: Option<PromptExperimentContext>,
    /// Optional structured feedback from a previous gate failure.
    pub gate_feedback: Option<GateFeedback>,
    /// Routing context for the CascadeRouter. Built at the dispatch site
    /// from task + runner state, threaded through to `RoutingInputs`.
    pub routing_context: Option<RoutingContext>,
    /// Output files from each completed dependency task.
    /// Each entry is `(task_id, files)`. Injected into the system prompt
    /// so the agent knows what its predecessors already produced.
    pub dependency_outputs: Vec<(String, Vec<String>)>,
    /// Pre-rendered error patterns from the shared in-memory store.
    ///
    /// Populated by `GraphTaskDispatcher` from
    /// `SharedAgentFactory::error_patterns_for_task`, the patterns keyed to
    /// the task (backlog 4210), so that agents dispatched later in the same
    /// plan run benefit from error patterns discovered by earlier agents.
    pub error_patterns_context: String,
    /// Pre-computed workspace map (indented crate/src tree).
    ///
    /// When non-empty, `PromptContext::from_task` uses this value instead of
    /// calling `generate_workspace_map` on the Tokio reactor thread.
    /// Populated once per plan run by `GraphTaskDispatcher` via its
    /// `static_prompt_cache` field.
    pub cached_workspace_map: String,
    /// Pre-computed crate descriptions of the workspace context, the part
    /// every checkout of a run shares.
    ///
    /// When non-empty, `PromptContext::from_task` uses this value instead of
    /// reading the Cargo.toml files again, and adds the attempt checkout's
    /// own branch and modified files to it (backlog 3110).
    pub cached_workspace_context: String,
    /// The other plans running in the same working tree now, each with the
    /// areas its tasks write (gap-c09fc7). Empty when the plan runs alone.
    pub concurrent_plans: Vec<(String, Vec<String>)>,
    /// The attempt this dispatch is: the unit of its route's exploration
    /// draw (S02.P1-3). `None` outside Graph dispatch, and then the route
    /// never explores.
    pub attempt_key: Option<roko_learn::telemetry::AttemptKey>,
}

// ─── Dispatcher facade ─────────────────────────────────────────────────

/// Single-entry agent dispatch facade.
///
/// Owns:
/// - a model router (cascade-aware),
/// - a prompt assembler (queries playbooks + neuro store),
/// - a warm pool for fast role transitions.
///
/// The dispatcher does *not* own the agent runtime itself —
/// [`Dispatcher::dispatch`] receives an [`AgentResultBridge`] and calls
/// through to it. Production wiring constructs the bridge from
/// [`AgentDispatcherV2`]; tests use a stub.
#[derive(Debug)]
pub struct Dispatcher {
    router: ModelRouter,
    prompt_assembler: PromptAssembler,
    warm_pool: WarmPool,
}

impl Dispatcher {
    /// Construct a new dispatcher.
    ///
    /// `configured_models` is the set of model slugs that have a configured,
    /// credential-ready provider in the current workspace.  When non-empty,
    /// cascade router results are filtered: a model whose slug is not in
    /// this set is replaced with the default fallback.
    pub fn new(
        cascade: Option<Arc<CascadeRouter>>,
        prompt_assembler: PromptAssembler,
        warm_pool: WarmPool,
        configured_models: HashSet<String>,
    ) -> Self {
        Self {
            router: ModelRouter::new(cascade).with_configured_models(configured_models),
            prompt_assembler,
            warm_pool,
        }
    }

    /// Attach a provider health registry and model-to-provider mapping so
    /// the inner [`ModelRouter`] can exclude unhealthy providers during
    /// cascade routing.
    #[must_use]
    pub fn with_provider_health(
        mut self,
        health: Arc<ProviderHealthRegistry>,
        model_providers: HashMap<String, String>,
    ) -> Self {
        self.router = self.router.with_provider_health(health, model_providers);
        self
    }

    /// Route by `health` from now on ([`ModelRouter::replace_provider_health`]).
    pub fn replace_provider_health(&mut self, health: Arc<ProviderHealthRegistry>) {
        self.router.replace_provider_health(health);
    }

    /// The provider health registry routing reads, if any.
    #[must_use]
    pub fn provider_health(&self) -> Option<&Arc<ProviderHealthRegistry>> {
        self.router.provider_health()
    }

    /// Exclude models whose provider ID is in `providers`.
    ///
    /// Populated from `[routing] disabled_providers` in `roko.toml`.
    #[must_use]
    pub fn with_disabled_providers(mut self, providers: HashSet<String>) -> Self {
        self.router = self.router.with_disabled_providers(providers);
        self
    }

    /// Register model slugs that lack tool-use support.
    ///
    /// When a task requires tool use (implementation, scaffolding, etc.),
    /// cascade router results whose slug is in this set are rejected and
    /// replaced with the default fallback.
    #[must_use]
    pub fn with_tool_capability_filter(mut self, models_without_tools: HashSet<String>) -> Self {
        self.router = self
            .router
            .with_tool_capability_filter(models_without_tools);
        self
    }

    /// Explore with probability `epsilon` on each route the cascade router
    /// decides ([`ModelRouter::with_explore_epsilon`]).
    #[must_use]
    pub fn with_explore_epsilon(mut self, epsilon: f64) -> Self {
        self.router = self.router.with_explore_epsilon(epsilon);
        self
    }

    /// Fall back to `slug` when nothing else decides a route
    /// ([`ModelRouter::with_default_slug`]): a workspace's `[agent]
    /// default_model` (backlog 3107).
    #[must_use]
    pub fn with_default_slug(mut self, slug: impl Into<String>) -> Self {
        self.router = self.router.with_default_slug(slug);
        self
    }

    /// Start tasks without an override or hint on their `[routing.ladder]`
    /// rung ([`ModelRouter::with_routing_ladder`]).
    #[must_use]
    pub fn with_routing_ladder(mut self, ladder: RoutingLadder) -> Self {
        self.router = self.router.with_routing_ladder(ladder);
        self
    }

    /// The routing ladder the inner [`ModelRouter`] uses, if any.
    #[must_use]
    pub fn routing_ladder(&self) -> Option<&RoutingLadder> {
        self.router.routing_ladder()
    }

    /// Route by `ladder` from now on, or by the router when it is `None`
    /// ([`ModelRouter::replace_routing_ladder`]).
    pub fn replace_routing_ladder(&mut self, ladder: Option<RoutingLadder>) {
        self.router.replace_routing_ladder(ladder);
    }

    /// Weigh the durable knowledge `store` into the cascade router's pick
    /// ([`ModelRouter::with_knowledge_store`]).
    #[must_use]
    pub fn with_knowledge_store(mut self, store: roko_neuro::KnowledgeStore) -> Self {
        self.router = self.router.with_knowledge_store(store);
        self
    }

    /// The knowledge store the inner [`ModelRouter`] weighs, if any.
    #[must_use]
    pub fn knowledge_store(&self) -> Option<&roko_neuro::KnowledgeStore> {
        self.router.knowledge_store()
    }

    /// Read-only access to the prompt assembler -- exposed for bidder
    /// persistence and diagnostic endpoints.
    #[must_use]
    pub fn prompt_assembler(&self) -> &PromptAssembler {
        &self.prompt_assembler
    }

    /// Read-only access to the warm pool -- exposed for diagnostics and
    /// admin endpoints (`/agents/warm-pool`) without leaking mutability.
    #[must_use]
    pub fn warm_pool(&self) -> &WarmPool {
        &self.warm_pool
    }

    /// Clone the cascade router `Arc` for use when reconstructing the
    /// dispatcher with an updated prompt assembler.
    #[must_use]
    pub fn cascade_router_arc(&self) -> Option<Arc<CascadeRouter>> {
        self.router.cascade_arc()
    }

    /// Resolve the model + prompt for `task` without dispatching.
    ///
    /// Used by tests, dry-run flows, and the prompt cache to materialize
    /// the dispatch decision without paying for an agent run.
    pub fn plan(
        &self,
        task: &TaskDef,
        ctx: &DispatchContext,
    ) -> Result<RunnerDispatchPlan, RunnerDispatchError> {
        let inputs = RoutingInputs::from_task(task, ctx);
        let (choice, mut decision) = self.router.decide(&inputs)?;
        decision.task_id.clone_from(&task.id);
        let prompt_ctx = PromptContext::from_task(task, ctx);
        let assembled = self.prompt_assembler.assemble(task, &prompt_ctx)?;
        Ok(RunnerDispatchPlan {
            model: choice.model.clone(),
            forced: choice.forced(),
            source: choice.source,
            prompt: assembled,
            route_decision: Some(decision),
        })
    }

    /// Like [`plan`](Self::plan) but emits structured routing decision logs.
    ///
    /// Used by the live event loop where routing visibility matters.
    pub fn plan_logged(
        &self,
        task: &TaskDef,
        ctx: &DispatchContext,
        task_id: &str,
    ) -> Result<RunnerDispatchPlan, RunnerDispatchError> {
        let inputs = RoutingInputs::from_task(task, ctx);
        let (choice, mut decision) = self.router.decide_logged(&inputs, task_id)?;
        decision.task_id = task_id.to_string();
        let prompt_ctx = PromptContext::from_task(task, ctx);
        let assembled = self.prompt_assembler.assemble(task, &prompt_ctx)?;
        Ok(RunnerDispatchPlan {
            model: choice.model.clone(),
            forced: choice.forced(),
            source: choice.source,
            prompt: assembled,
            route_decision: Some(decision),
        })
    }

    /// Dispatch `task` through the supplied bridge and normalize the
    /// outcome.
    ///
    /// The bridge isolates provider state so this method stays pure
    /// orchestration: route + compose + act + normalize.
    pub async fn dispatch<B: AgentResultBridge>(
        &self,
        task: &TaskDef,
        ctx: &DispatchContext,
        bridge: &B,
    ) -> Result<AgentOutcome, RunnerDispatchError> {
        let plan = self.plan(task, ctx)?;
        let result = bridge
            .run_agent(&plan, ctx)
            .await
            .map_err(|err| RunnerDispatchError::SpawnFailed(err.to_string()))?;
        Ok(result)
    }

    /// Launch a streaming CLI agent through the dispatch facade.
    ///
    /// Runner v2 still owns the returned process handle so it can enforce
    /// cancellation and orphan cleanup, but provider invocation construction and
    /// runtime event normalization are below dispatch/roko-agent.
    pub async fn spawn_streaming_cli_agent(
        &self,
        config: &crate::runner::agent_stream::AgentSpawnConfig,
        event_tx: mpsc::Sender<AgentRuntimeEvent>,
    ) -> anyhow::Result<crate::runner::agent_stream::AgentHandle> {
        crate::runner::agent_stream::spawn_agent(config, event_tx).await
    }

    /// Launch a streaming CLI agent with runner-owned startup cancellation.
    pub async fn spawn_streaming_cli_agent_controlled(
        &self,
        config: &crate::runner::agent_stream::AgentSpawnConfig,
        event_tx: mpsc::Sender<AgentRuntimeEvent>,
        control: &crate::runner::agent_stream::AgentStartupControl,
    ) -> std::result::Result<
        crate::runner::agent_stream::AgentHandle,
        crate::runner::agent_stream::AgentStartupError,
    > {
        crate::runner::agent_stream::spawn_agent_controlled(config, event_tx, Some(control)).await
    }
}

/// Materialized runner dispatch plan — what `Dispatcher::plan` resolves to.
#[derive(Debug, Clone)]
pub struct RunnerDispatchPlan {
    /// Selected model + backend.
    pub model: ModelSpec,
    /// `true` if the model came from an operator override (`--model`)
    /// rather than the cascade router. Recorded so feedback writers tag
    /// observations as manual overrides and the router's learned policy
    /// is not corrupted.
    pub forced: bool,
    /// Why the router picked `model`: override, task hint, ladder rung,
    /// cascade router or default.
    pub source: ModelChoiceSource,
    /// Assembled prompt, allowlist, diagnostics.
    pub prompt: AssembledPrompt,
    /// The route decision behind `model` (S01 §5.3), not yet keyed to an
    /// attempt: Graph dispatch writes it to the run's `decisions.jsonl`.
    pub route_decision: Option<RoutingDecisionLog>,
}

// ─── Provider bridge trait (async-trait friendly) ──────────────────────

/// Minimal async trait implemented by provider runtimes.
///
/// Production: a thin shim around `AgentDispatcherV2::run_agent_result_bridge`.
/// Tests: a stub returning canned outcomes.
#[async_trait::async_trait]
pub trait AgentResultBridge: Send + Sync {
    /// Run the agent for `plan` and return a normalized outcome.
    async fn run_agent(
        &self,
        plan: &RunnerDispatchPlan,
        ctx: &DispatchContext,
    ) -> Result<AgentOutcome, anyhow::Error>;
}

// ─── Runtime Launch Facade ─────────────────────────────────────────────

/// Runtime selected for a resolved model.
#[derive(Debug, Clone)]
pub enum ResolvedAgentRuntime {
    /// Streaming CLI subprocess with provider-specific invocation metadata.
    Cli {
        /// Concrete model slug sent to the CLI.
        model: String,
        /// Resolved CLI provider. `None` preserves legacy runner defaults when
        /// no `roko.toml` provider graph has been loaded.
        cli_provider: Option<CliProviderConfig>,
    },
    /// API/provider-backed agent bridged through `AgentDispatcherV2`.
    Bridge {
        /// Concrete model slug sent to the provider.
        model: String,
        /// Provider registry id for diagnostics.
        provider_id: String,
        /// Effective config used to create the provider-backed agent.
        roko_config: Arc<RokoConfig>,
    },
}

/// Resolve the runtime that should execute `requested_model`.
pub fn resolve_agent_runtime(
    roko_config: Option<&Arc<RokoConfig>>,
    requested_model: &str,
) -> Result<ResolvedAgentRuntime, String> {
    let Some(roko_config) = roko_config else {
        return Ok(ResolvedAgentRuntime::Cli {
            model: requested_model.to_string(),
            cli_provider: None,
        });
    };

    let resolver = ProviderDispatchResolver::new(Arc::clone(roko_config));
    let spec = resolver.resolve(requested_model);
    match spec.runtime {
        ProviderRuntime::Cli(provider) => Ok(ResolvedAgentRuntime::Cli {
            model: spec.model_slug,
            cli_provider: Some(provider),
        }),
        ProviderRuntime::AgentResultBridge { .. } => Ok(ResolvedAgentRuntime::Bridge {
            model: spec.model_slug,
            provider_id: spec.provider_id,
            roko_config: Arc::clone(roko_config),
        }),
        ProviderRuntime::Unsupported(unsupported) => Err(format!(
            "model `{requested_model}` resolved to unsupported provider `{}`: {}",
            spec.provider_id, unsupported.detail
        )),
    }
}

/// Spawn an API/provider-backed agent and forward streaming runtime events.
///
/// Events are forwarded in real time as `StreamChunk`s arrive from the agent,
/// rather than batched after the full run completes.
pub fn spawn_agent_result_bridge(
    roko_config: Arc<RokoConfig>,
    request: AgentDispatchRequest,
    event_tx: mpsc::Sender<AgentRuntimeEvent>,
) {
    tokio::spawn(async move {
        let dispatcher = AgentDispatcherV2::new(roko_config);
        if let Err(err) = dispatcher
            .run_agent_streaming(request, event_tx.clone())
            .await
        {
            let _ = event_tx
                .send(AgentRuntimeEvent::Error {
                    message: err.to_string(),
                })
                .await;
            let _ = event_tx
                .send(AgentRuntimeEvent::Exited { exit_code: Some(1) })
                .await;
        }
    });
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_task(id: &str) -> TaskDef {
        TaskDef {
            id: id.into(),
            title: id.into(),
            description: None,
            role: Some("implementer".into()),
            status: "ready".into(),
            tier: "focused".into(),
            frequency: None,
            model_hint: Some("claude-sonnet-4-6".into()),
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
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
            spec: Default::default(),
            hints: Default::default(),
        }
    }

    fn make_ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p1".into(),
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
        }
    }

    struct StubBridge;

    #[async_trait::async_trait]
    impl AgentResultBridge for StubBridge {
        async fn run_agent(
            &self,
            plan: &RunnerDispatchPlan,
            ctx: &DispatchContext,
        ) -> Result<AgentOutcome, anyhow::Error> {
            Ok(AgentOutcome {
                task_id: "t".into(),
                plan_id: ctx.plan_id.clone(),
                model: plan.model.slug.clone(),
                provider: format!("{:?}", plan.model.backend).to_lowercase(),
                output: "ok".into(),
                tokens_in: 10,
                tokens_out: 20,
                cost_usd: 0.001,
                duration_ms: 42,
                exit_code: Some(0),
                is_error: false,
            })
        }
    }

    /// backlog 3107: with `[agent] default_model` set (a `[models.*]` key,
    /// resolved as failover resolves it) and no ladder, a task with no hint
    /// and no routing context routes to that model, as the default.
    #[tokio::test]
    async fn router_default_follows_agent_default_model() {
        let mut config = RokoConfig::default();
        config.routing.ladder.enabled = false;
        config.agent.default_model = "house-model".to_string();
        let profile = roko_core::config::schema::ModelProfile {
            provider: "house-cli".to_string(),
            slug: "glm-5.1".to_string(),
            ..Default::default()
        };
        config.models.insert("house-model".to_string(), profile);
        let factory = SharedAgentFactory::new(Arc::new(config), None, None, None).await;
        let mut task = make_task("t-default");
        task.model_hint = None;
        let workdir = tempfile::tempdir().expect("tempdir");
        let ctx = DispatchContext {
            workdir: workdir.path().to_path_buf(),
            ..make_ctx()
        };

        let plan = factory.dispatcher().plan(&task, &ctx).expect("plan");

        assert_eq!(plan.model.slug, "glm-5.1");
        assert_eq!(plan.source, ModelChoiceSource::Default);
    }

    #[tokio::test]
    async fn dispatcher_plan_and_dispatch_round_trip() {
        let dispatcher = Dispatcher::new(
            None,
            PromptAssembler::minimal(),
            WarmPool::new(0),
            HashSet::new(),
        );
        let task = make_task("t-1");
        let ctx = make_ctx();
        let plan = dispatcher.plan(&task, &ctx).expect("plan");
        assert_eq!(plan.model.slug, "claude-sonnet-4-6");
        assert!(!plan.prompt.system_prompt.is_empty());

        let outcome = dispatcher
            .dispatch(&task, &ctx, &StubBridge)
            .await
            .expect("dispatch");
        assert_eq!(outcome.model, "claude-sonnet-4-6");
        assert_eq!(outcome.tokens_in, 10);
    }

    #[tokio::test]
    async fn force_backend_overrides_router() {
        let dispatcher = Dispatcher::new(
            None,
            PromptAssembler::minimal(),
            WarmPool::new(0),
            HashSet::new(),
        );
        let task = make_task("t-2");
        let ctx = DispatchContext {
            force_backend: Some("gpt-5".into()),
            ..make_ctx()
        };
        let plan = dispatcher.plan(&task, &ctx).expect("plan");
        assert_eq!(plan.model.slug, "gpt-5");
        assert!(plan.forced, "force_backend must mark plan as forced");
    }
}
