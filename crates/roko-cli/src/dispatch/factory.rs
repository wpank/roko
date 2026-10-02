//! Shared agent factory — reuses expensive components across agent dispatches.
//!
//! Per-task agent creation currently rebuilds `ProviderSemaphores`, runs MCP
//! discovery via `block_on` on an OS thread, and reconstructs the
//! `Dispatcher` / `PromptAssembler` / `WarmPool`.  `SharedAgentFactory`
//! creates these once at run start and hands them to every dispatch call.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_learn::error_pattern_store::ErrorPatternStore;

use roko_agent::AgentRuntimeEvent;
use roko_agent::mcp::{McpConfig, McpRuntime, discover_mcp_runtime};
use roko_agent::provider::{LocalToolRuntime, ProviderSemaphores};
use roko_agent::rate_limit::ProviderRateLimiter;
use roko_compose::{AttentionBidder, LearningBidder};
use roko_core::config::schema::RokoConfig;
use roko_core::tool::ToolDef;
use roko_learn::provider_health::ProviderHealthRegistry;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::dispatch_v2::CliPluginMcpConfig;
use crate::dispatch_v2::{
    AgentDispatchRequest, AgentDispatcherV2, AgentResultDispatch, DispatchV2Error,
    ProviderDispatchResolver, ProviderRuntime,
};

use super::plugin_mcp::CliPluginMcpBridge;

use super::{
    Dispatcher, PromptAssembler, PromptCache, ResolvedAgentRuntime, RoutingLadder, WarmPool,
};

/// The error patterns a task's prompt carries (backlog 4210): the rendered
/// block, and the keys of the patterns in it, which the attempt's exposure
/// record can name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorPatternSelection {
    /// The rendered block; empty when no pattern is keyed to the task.
    pub text: String,
    /// The keys of the selected patterns, in display order.
    pub keys: Vec<String>,
}

/// Shared, reusable components for agent dispatch.
///
/// Constructed once at the start of a plan run.  The factory owns:
///
/// - **`ProviderSemaphores`** — concurrency limits per provider (no longer rebuilt per task).
/// - **`mcp_runtime`** — MCP definitions and initialized execution clients discovered once.
/// - **`Dispatcher`** — model routing + prompt assembly + warm pool (stateless, reusable).
/// - **`ProviderDispatchResolver`** — model → provider resolution.
pub struct SharedAgentFactory {
    config: Arc<RokoConfig>,
    semaphores: Arc<ProviderSemaphores>,
    mcp_runtime: Option<Arc<McpRuntime>>,
    /// Declarative plugin definitions paired with their live handlers.
    local_tool_runtime: Option<Arc<LocalToolRuntime>>,
    /// Loopback bridge used only by opaque CLI provider subprocesses.
    cli_plugin_mcp_bridge: Option<CliPluginMcpBridge>,
    /// Startup failure retained so CLI resolution can reject rather than
    /// silently advertising a provider without plugin handler parity.
    cli_plugin_mcp_error: Option<String>,
    dispatcher: Dispatcher,
    resolver: ProviderDispatchResolver,
    /// Runtime-scoped per-provider rate limiter built from `[providers.<name>].limits`.
    ///
    /// Shared across all concurrent agent dispatches for the duration of the run.
    /// Passed to `AgentOptions.rate_limiter` so HTTP-backed provider adapters
    /// call `acquire(provider_id)` before each live LLM request.
    rate_limiter: Arc<ProviderRateLimiter>,
    /// Runtime-scoped provider health registry shared across routing and outcome recording.
    ///
    /// The same `Arc` is used by `AgentDispatcherV2` for bridge calls and by
    /// the runner terminal path for CLI calls. Provider outcomes are recorded
    /// directly at those call sites; the learning event subscriber deliberately
    /// does not mirror them through the event bus.
    pub health_registry: Arc<ProviderHealthRegistry>,
    /// Persistent JSONL tool audit adapter shared across all dispatches.
    tool_audit: Option<Arc<roko_fs::tool_audit::ScrubAuditAdapter>>,
    /// Per-call trace and metrics sinks shared across all dispatches
    /// (find-f489db).
    observability: Option<roko_fs::FsObservabilitySinks>,
    /// The safety provenance sinks of the runs in flight (gap-ff95f5).
    provenance: Option<crate::safety_provenance::ProvenanceSinks>,
    /// Shared in-memory error pattern store. When an agent's gate fails, the
    /// observation is written here immediately so that subsequent agent
    /// dispatches within the same plan run can include the pattern in their
    /// system prompt -- without waiting for a new run to reload from disk.
    ///
    /// Uses `std::sync::RwLock` because `ErrorPatternStore` performs only
    /// brief CPU-bound operations (no I/O under the lock).
    error_pattern_store: Arc<std::sync::RwLock<ErrorPatternStore>>,
    /// What the run's prompt-cache snapshot holds, when the factory's
    /// prompts are built from one (backlog 4214).
    prompt_snapshot: Option<crate::dispatch::prompt_cache::PromptCacheDigest>,
}

/// Bridge task returned only after its worker reaches the provider boundary.
pub struct StartedSharedAgentBridge {
    pub handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedBridgeStartupError {
    Deadline,
    Cancelled,
    WorkerExited,
}

impl std::fmt::Debug for SharedAgentFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedAgentFactory")
            .field("config", &"...")
            .finish()
    }
}

impl SharedAgentFactory {
    /// Create a new factory, performing one-time MCP discovery on the current
    /// tokio runtime (no `block_on`, no OS thread).
    ///
    /// When `prompt_cache` is provided, the factory's `PromptAssembler` will
    /// serve knowledge / episode / playbook / effectiveness data from memory
    /// instead of reading from the filesystem on every task dispatch.
    pub async fn new(
        config: Arc<RokoConfig>,
        mcp_config_path: Option<&PathBuf>,
        cascade_router: Option<Arc<roko_learn::cascade_router::CascadeRouter>>,
        prompt_cache: Option<Arc<PromptCache>>,
    ) -> Self {
        let providers = config.effective_providers();
        let semaphores = Arc::new(ProviderSemaphores::new(&providers));

        let mcp_runtime = match mcp_config_path {
            Some(path) => match McpConfig::load(path) {
                Ok(mcp_config) => match discover_mcp_runtime(&mcp_config).await {
                    Ok(runtime) => {
                        tracing::info!(
                            tool_count = runtime.tools().len(),
                            "factory: MCP tools discovered"
                        );
                        Some(Arc::new(runtime))
                    }
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            "factory: MCP discovery failed; agents will retry per-task"
                        );
                        None
                    }
                },
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "factory: MCP config load failed"
                    );
                    None
                }
            },
            None => None,
        };

        let prompt_snapshot = prompt_cache.as_deref().map(PromptCache::digest);
        let prompt_assembler = match prompt_cache {
            Some(cache) => PromptAssembler::with_cache(cache),
            None => PromptAssembler::new(),
        };
        // Apply [prompt] config knobs from roko.toml.
        let prompt_assembler = prompt_assembler
            .with_composition_strategy(config.prompt.composition_strategy)
            .with_vcg_warmup_observations(config.prompt.vcg_warmup_observations);
        // Default warm-pool capacity: 2 slots per role. Zero-capacity silently
        // discards every pre-spawned agent on insert; using 2 lets the reviewer
        // slot remain warm while the implementer is being cleaned up.
        //
        // Use `available_model_slugs_for_cascade` which filters by credential
        // availability, so cascade router results are constrained to models
        // that actually have a configured, credential-ready provider.
        let configured_models: HashSet<String> = config
            .available_model_slugs_for_cascade()
            .into_iter()
            .collect();
        let model_providers = crate::config_helpers::routing_model_provider_map(&config);
        let warm_pool_size = config.runner.warm_pool_size;
        let dispatcher = Dispatcher::new(
            cascade_router,
            prompt_assembler,
            WarmPool::new(warm_pool_size),
            configured_models.clone(),
        );
        let resolver = ProviderDispatchResolver::new(Arc::clone(&config));

        // Build one shared rate limiter from the provider limits declared in roko.toml.
        // Every concurrent agent dispatch shares this budget so the collective request
        // rate across all spawned agents respects the per-provider RPM/TPM config.
        let rate_limiter = Arc::new(ProviderRateLimiter::from_provider_configs(
            60, // default RPM when no provider-level limit is configured
            config.effective_providers().iter(),
        ));

        // Callers can replace this with a persisted workspace registry via
        // `with_health_registry`. Keeping construction local preserves the
        // factory's use in path-free unit tests.
        let health_registry = Arc::new(ProviderHealthRegistry::new());

        // Wire health into the dispatcher's ModelRouter so cascade routing
        // excludes Open-circuit providers and demotes HalfOpen ones.
        let dispatcher =
            dispatcher.with_provider_health(Arc::clone(&health_registry), model_providers);

        // Wire statically disabled providers from [routing] config so the
        // cascade router never selects models from excluded providers.
        let disabled_providers: HashSet<String> =
            config.routing.disabled_providers.iter().cloned().collect();
        let dispatcher = if disabled_providers.is_empty() {
            dispatcher
        } else {
            tracing::info!(
                disabled = ?config.routing.disabled_providers,
                "routing: statically disabled providers"
            );
            dispatcher.with_disabled_providers(disabled_providers)
        };

        // Wire tool-capability filtering so search-only models (e.g. Perplexity
        // sonar) are never selected for tasks that require tool use.
        let tool_capable: HashSet<String> = config.models_supporting_tools().into_iter().collect();
        let models_without_tools: HashSet<String> = configured_models
            .iter()
            .filter(|slug| !tool_capable.contains(*slug))
            .cloned()
            .collect();
        let dispatcher = if models_without_tools.is_empty() {
            dispatcher
        } else {
            tracing::info!(
                models = ?models_without_tools,
                "routing: models without tool support will be skipped for tool-requiring tasks"
            );
            dispatcher.with_tool_capability_filter(models_without_tools)
        };

        // `[routing.ladder]`: a task's role and tier pick its start rung
        // unless `--model` or its `model_hint` pins one. Rungs this workspace
        // cannot dispatch are skipped and logged here, once per factory.
        let dispatcher = match RoutingLadder::from_config(&config) {
            Some(ladder) => dispatcher.with_routing_ladder(ladder),
            None => dispatcher,
        };

        Self {
            config,
            semaphores,
            mcp_runtime,
            local_tool_runtime: None,
            cli_plugin_mcp_bridge: None,
            cli_plugin_mcp_error: None,
            dispatcher,
            resolver,
            rate_limiter,
            health_registry,
            tool_audit: None,
            observability: None,
            provenance: None,
            // Start with an empty in-memory store. Callers should replace it
            // via `with_error_pattern_store` or `with_error_patterns_from_disk`.
            error_pattern_store: Arc::new(std::sync::RwLock::new(ErrorPatternStore::empty())),
            prompt_snapshot,
        }
    }

    /// What the run's prompt-cache snapshot holds, when this factory's
    /// prompts are built from one: the decision records name it (backlog
    /// 4214).
    pub fn prompt_snapshot(&self) -> Option<&crate::dispatch::prompt_cache::PromptCacheDigest> {
        self.prompt_snapshot.as_ref()
    }

    /// Read-only access to the shared dispatcher (for plan/route without acting).
    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    /// Read-only access to the warm pool held by the shared dispatcher.
    ///
    /// Callers use this to check for a pre-warmed agent slot before dispatch
    /// (recording `was_warm_start = true` in efficiency events) and to return
    /// the slot to the pool after a successful task completes.
    pub fn warm_pool(&self) -> &WarmPool {
        self.dispatcher.warm_pool()
    }

    /// Skip the ladder rungs whose models `failed` their tool-use probe, by
    /// model with the reason (backlog 1121); with no rung left the router
    /// picks.
    #[must_use]
    pub fn skip_failed_rungs(
        mut self,
        failed: &std::collections::BTreeMap<String, String>,
    ) -> Self {
        if failed.is_empty() {
            return self;
        }
        if let Some(ladder) = self.dispatcher.routing_ladder().cloned() {
            self.dispatcher
                .replace_routing_ladder(ladder.without_models(failed));
        }
        self
    }

    /// Use a caller-owned provider health registry for all subsequent
    /// dispatches from this factory.
    ///
    /// Runner v2 supplies the persisted workspace registry here so bridge and
    /// CLI provider outcomes share one circuit-breaker state.
    #[must_use]
    pub fn with_health_registry(mut self, registry: Arc<ProviderHealthRegistry>) -> Self {
        self.health_registry = registry;
        self
    }

    /// Attach a persistent JSONL tool audit adapter.
    ///
    /// When set, every tool call dispatched through agents created by this
    /// factory records scrubbed admit/result lines to disk.
    #[must_use]
    pub fn with_tool_audit(mut self, adapter: Arc<roko_fs::tool_audit::ScrubAuditAdapter>) -> Self {
        self.tool_audit = Some(adapter);
        self
    }

    /// Attach per-call trace and metrics sinks.
    ///
    /// When set, every tool call dispatched through agents created by this
    /// factory leaves a closed trace and a metrics record (find-f489db).
    #[must_use]
    pub fn with_observability_sinks(mut self, sinks: roko_fs::FsObservabilitySinks) -> Self {
        self.observability = Some(sinks);
        self
    }

    /// Record each dispatch's tool calls with the safety provenance sink
    /// `sinks` holds for its run (gap-ff95f5). A run registers its sink there
    /// before its tasks run.
    #[must_use]
    pub fn with_provenance_sinks(
        mut self,
        sinks: crate::safety_provenance::ProvenanceSinks,
    ) -> Self {
        self.provenance = Some(sinks);
        self
    }

    /// Replace the error pattern store with a pre-loaded shared instance.
    #[must_use]
    pub fn with_error_pattern_store(
        mut self,
        store: Arc<std::sync::RwLock<ErrorPatternStore>>,
    ) -> Self {
        self.error_pattern_store = store;
        self
    }

    /// Load the error pattern store from disk at the given workspace root.
    #[must_use]
    pub fn with_error_patterns_from_disk(mut self, workdir: &Path) -> Self {
        let learn_dir = workdir.join(".roko").join("learn");
        // Runner-v2's pattern file is set aside, never read (backlog 4204).
        if let Err(error) =
            roko_learn::error_pattern_store::retire_legacy_discovered_patterns(&learn_dir)
        {
            tracing::warn!(%error, "factory: Runner-v2's pattern file could not be set aside");
        }
        let path = learn_dir.join(roko_learn::error_pattern_store::ERROR_PATTERNS_FILE);
        let store = ErrorPatternStore::load(&path);
        tracing::debug!(
            pattern_count = store.len(),
            "factory: loaded error patterns from disk"
        );
        self.error_pattern_store = Arc::new(std::sync::RwLock::new(store));
        self
    }

    /// Weigh what the durable knowledge store of the workspace at `workdir`
    /// says about each model into the cascade router's pick (reg-ff6e1a).
    #[must_use]
    pub fn with_knowledge_routing(mut self, workdir: &Path) -> Self {
        let store = roko_neuro::KnowledgeStore::for_workdir(workdir);
        self.dispatcher = self.dispatcher.with_knowledge_store(store);
        self
    }

    /// Shared error pattern store for cross-agent pattern sharing.
    pub fn error_pattern_store(&self) -> &Arc<std::sync::RwLock<ErrorPatternStore>> {
        &self.error_pattern_store
    }

    /// The error patterns keyed to task `task_id` of plan `plan_id`, whose
    /// verify steps run `verify_commands`: those its own earlier attempts hit
    /// and those of its verify commands, at most `limit` (backlog 4210).
    /// Empty when none is keyed to the task, or the lock is poisoned
    /// (fail-open: missing context is better than a panic).
    pub fn error_patterns_for_task(
        &self,
        plan_id: &str,
        task_id: &str,
        verify_commands: &[String],
        limit: usize,
    ) -> ErrorPatternSelection {
        let Ok(store) = self.error_pattern_store.read() else {
            tracing::warn!("error pattern store lock poisoned; skipping prompt injection");
            return ErrorPatternSelection::default();
        };
        let query = roko_learn::error_pattern_store::FailurePatternQuery {
            plan_id: Some(plan_id),
            task_id: Some(task_id),
            verify_commands,
            ..Default::default()
        };
        let summary = store.bounded_summary_keyed(query, limit, 2_000);
        ErrorPatternSelection {
            text: summary.format_for_prompt(),
            keys: summary
                .patterns
                .iter()
                .map(|pattern| pattern.key.clone())
                .collect(),
        }
    }

    /// Attach the canonical declarative-plugin runtime to every provider
    /// bridge spawned by this factory.
    #[must_use]
    pub fn with_local_tool_runtime(mut self, runtime: Arc<LocalToolRuntime>) -> Self {
        match CliPluginMcpBridge::start(Arc::clone(&runtime), Arc::clone(&self.config)) {
            Ok(bridge) => {
                self.cli_plugin_mcp_bridge = Some(bridge);
                self.cli_plugin_mcp_error = None;
            }
            Err(error) => {
                tracing::error!(%error, "CLI plugin MCP bridge unavailable");
                self.cli_plugin_mcp_bridge = None;
                self.cli_plugin_mcp_error = Some(error);
            }
        }
        self.local_tool_runtime = Some(runtime);
        self
    }

    /// Mint a contract-scoped MCP bridge configuration for one CLI task.
    #[must_use]
    pub fn cli_plugin_mcp_config(
        &self,
        worktree: &std::path::Path,
        immune_root: &std::path::Path,
        contract: &roko_agent::safety::contract::AgentContract,
    ) -> Option<CliPluginMcpConfig> {
        self.cli_plugin_mcp_bridge
            .as_ref()
            .and_then(|bridge| bridge.session_config(worktree, immune_root, contract))
    }

    /// Set persisted learning bidders on the prompt assembler.
    ///
    /// Called at run startup after loading from `.roko/learn/attention-bidders.json`.
    /// The bidders are passed to `PromptComposer::with_learning_bidders` when the
    /// runner-v2 prompt path is routed through the canonical compose surface.
    pub fn set_learning_bidders(&mut self, bidders: HashMap<AttentionBidder, LearningBidder>) {
        self.dispatcher
            .prompt_assembler()
            .replace_learning_bidders(bidders);
    }

    /// Resolve the runtime for a model key.
    pub fn resolve_runtime(&self, model_key: &str) -> Result<ResolvedAgentRuntime, String> {
        let spec = self.resolver.resolve(model_key);
        match spec.runtime {
            ProviderRuntime::Cli(provider) => {
                if self.local_tool_runtime.is_some()
                    && let Some(error) = &self.cli_plugin_mcp_error
                {
                    return Err(format!(
                        "model `{model_key}` requires CLI plugin handler parity, but the local MCP bridge failed: {error}"
                    ));
                }
                Ok(ResolvedAgentRuntime::Cli {
                    model: spec.model_slug,
                    cli_provider: Some(provider),
                })
            }
            ProviderRuntime::AgentResultBridge { .. } => Ok(ResolvedAgentRuntime::Bridge {
                model: spec.model_slug,
                provider_id: spec.provider_id,
                roko_config: Arc::clone(&self.config),
            }),
            ProviderRuntime::Unsupported(unsupported) => Err(format!(
                "model `{model_key}` resolved to unsupported provider `{}`: {}",
                spec.provider_id, unsupported.detail
            )),
        }
    }

    /// Run one provider dispatch through the factory's shared runtime state.
    ///
    /// This is the synchronous-result counterpart to
    /// [`Self::spawn_shared_agent_bridge`]. It preserves the same semaphores,
    /// rate limits, provider-health registry, pre-discovered MCP runtime, and
    /// declarative plugin handlers for callers such as Graph Activity Cells
    /// that await a concrete output Signal.
    pub async fn run_shared_agent_bridge(
        &self,
        request: AgentDispatchRequest,
    ) -> Result<AgentResultDispatch, DispatchV2Error> {
        self.run_shared_agent_bridge_with_config(request, Arc::clone(&self.config))
            .await
    }

    /// [`Self::run_shared_agent_bridge`], resolving `request.model_key`
    /// against `config` instead of the run's config. Provider failover uses
    /// this to serve a hinted slug on another configured provider that no
    /// `[models.*]` entry names.
    pub async fn run_shared_agent_bridge_with_config(
        &self,
        request: AgentDispatchRequest,
        config: Arc<RokoConfig>,
    ) -> Result<AgentResultDispatch, DispatchV2Error> {
        let local_tool_mcp = request.agent_contract.as_ref().and_then(|contract| {
            self.cli_plugin_mcp_config(
                &request.workdir,
                request.immune_root.as_deref().unwrap_or(&request.workdir),
                contract,
            )
        });
        let local_tool_mcp_bridge_ready =
            self.cli_plugin_mcp_bridge.is_some() && request.agent_contract.is_some();
        let mut dispatcher = AgentDispatcherV2::with_shared(config, Arc::clone(&self.semaphores))
            .with_rate_limiter(Arc::clone(&self.rate_limiter))
            .with_health_registry(Arc::clone(&self.health_registry));
        if let Some(audit) = &self.tool_audit {
            dispatcher = dispatcher.with_tool_audit(Arc::clone(audit));
        }
        if let Some(sinks) = &self.observability {
            dispatcher = dispatcher.with_observability_sinks(sinks.clone());
        }
        if let Some(sinks) = &self.provenance {
            dispatcher = dispatcher.with_provenance_sinks(sinks.clone());
        }

        dispatcher
            .run_agent_result_bridge_with_tools_and_cli_mcp(
                request,
                self.mcp_runtime.clone(),
                self.local_tool_runtime.clone(),
                local_tool_mcp,
                local_tool_mcp_bridge_ready,
            )
            .await
    }

    /// Spawn an API/provider-backed agent using shared semaphores and
    /// pre-discovered MCP tools.
    ///
    /// `cancel_token` is threaded into the `ToolLoopAgent` so runner-level
    /// task cancellation halts in-progress tool execution.
    pub fn spawn_shared_agent_bridge(
        &self,
        request: AgentDispatchRequest,
        event_tx: mpsc::Sender<AgentRuntimeEvent>,
        cancel_token: Option<Arc<dyn roko_core::tool::CancelToken>>,
    ) -> tokio::task::JoinHandle<()> {
        let local_tool_mcp = request.agent_contract.as_ref().and_then(|contract| {
            self.cli_plugin_mcp_config(
                &request.workdir,
                request.immune_root.as_deref().unwrap_or(&request.workdir),
                contract,
            )
        });
        let local_tool_mcp_bridge_ready =
            self.cli_plugin_mcp_bridge.is_some() && request.agent_contract.is_some();
        let config = Arc::clone(&self.config);
        let semaphores = Arc::clone(&self.semaphores);
        let mcp_runtime = self.mcp_runtime.clone();
        let local_tool_runtime = self.local_tool_runtime.clone();
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let health_registry = Arc::clone(&self.health_registry);
        let tool_audit = self.tool_audit.clone();
        let observability = self.observability.clone();
        let provenance = self.provenance.clone();

        tokio::spawn(async move {
            let mut dispatcher = AgentDispatcherV2::with_shared(config, semaphores)
                .with_rate_limiter(rate_limiter)
                .with_health_registry(health_registry);
            if let Some(token) = cancel_token {
                dispatcher = dispatcher.with_cancel_token(token);
            }
            if let Some(audit) = tool_audit {
                dispatcher = dispatcher.with_tool_audit(audit);
            }
            if let Some(sinks) = observability {
                dispatcher = dispatcher.with_observability_sinks(sinks);
            }
            if let Some(sinks) = provenance {
                dispatcher = dispatcher.with_provenance_sinks(sinks);
            }
            match dispatcher
                .run_agent_result_bridge_with_tools_and_cli_mcp(
                    request,
                    mcp_runtime,
                    local_tool_runtime,
                    local_tool_mcp,
                    local_tool_mcp_bridge_ready,
                )
                .await
            {
                Ok(dispatch) => {
                    for event in dispatch.events {
                        if event_tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
                Err(err) => {
                    let _ = event_tx
                        .send(AgentRuntimeEvent::Error {
                            message: err.to_string(),
                        })
                        .await;
                    let _ = event_tx
                        .send(AgentRuntimeEvent::Exited { exit_code: Some(1) })
                        .await;
                }
            }
        })
    }

    /// Spawn a bridge and wait for a typed startup checkpoint. A deadline or
    /// cancellation aborts and joins the worker before returning, proving that
    /// no unowned provider future survives dispatch startup.
    ///
    /// `cancel_token` is threaded into the `ToolLoopAgent` so runner-level
    /// task cancellation halts in-progress tool execution.
    pub async fn spawn_shared_agent_bridge_controlled(
        &self,
        request: AgentDispatchRequest,
        event_tx: mpsc::Sender<AgentRuntimeEvent>,
        deadline: tokio::time::Instant,
        cancel: CancellationToken,
        cancel_token: Option<Arc<dyn roko_core::tool::CancelToken>>,
    ) -> Result<StartedSharedAgentBridge, SharedBridgeStartupError> {
        let local_tool_mcp = request.agent_contract.as_ref().and_then(|contract| {
            self.cli_plugin_mcp_config(
                &request.workdir,
                request.immune_root.as_deref().unwrap_or(&request.workdir),
                contract,
            )
        });
        let local_tool_mcp_bridge_ready =
            self.cli_plugin_mcp_bridge.is_some() && request.agent_contract.is_some();
        let config = Arc::clone(&self.config);
        let semaphores = Arc::clone(&self.semaphores);
        let mcp_runtime = self.mcp_runtime.clone();
        let local_tool_runtime = self.local_tool_runtime.clone();
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let health_registry = Arc::clone(&self.health_registry);
        let tool_audit = self.tool_audit.clone();
        let observability = self.observability.clone();
        let provenance = self.provenance.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();

        let mut handle = tokio::spawn(async move {
            let mut dispatcher = AgentDispatcherV2::with_shared(config, semaphores)
                .with_rate_limiter(rate_limiter)
                .with_health_registry(health_registry);
            if let Some(token) = cancel_token {
                dispatcher = dispatcher.with_cancel_token(token);
            }
            if let Some(audit) = tool_audit {
                dispatcher = dispatcher.with_tool_audit(audit);
            }
            if let Some(sinks) = observability {
                dispatcher = dispatcher.with_observability_sinks(sinks);
            }
            if let Some(sinks) = provenance {
                dispatcher = dispatcher.with_provenance_sinks(sinks);
            }
            if started_tx.send(()).is_err() {
                return;
            }
            match dispatcher
                .run_agent_result_bridge_with_tools_and_cli_mcp(
                    request,
                    mcp_runtime,
                    local_tool_runtime,
                    local_tool_mcp,
                    local_tool_mcp_bridge_ready,
                )
                .await
            {
                Ok(dispatch) => {
                    for event in dispatch.events {
                        if event_tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
                Err(err) => {
                    let _ = event_tx
                        .send(AgentRuntimeEvent::Error {
                            message: err.to_string(),
                        })
                        .await;
                    let _ = event_tx
                        .send(AgentRuntimeEvent::Exited { exit_code: Some(1) })
                        .await;
                }
            }
        });
        let startup = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(SharedBridgeStartupError::Cancelled),
            _ = tokio::time::sleep_until(deadline) => Err(SharedBridgeStartupError::Deadline),
            result = started_rx => result.map_err(|_| SharedBridgeStartupError::WorkerExited),
        };
        if let Err(error) = startup {
            handle.abort();
            let _ = (&mut handle).await;
            return Err(error);
        }
        Ok(StartedSharedAgentBridge { handle })
    }

    /// Pre-discovered MCP tools, if available.
    pub fn mcp_tools(&self) -> Option<&Arc<Vec<ToolDef>>> {
        self.mcp_runtime.as_ref().map(|runtime| runtime.tools())
    }
}
