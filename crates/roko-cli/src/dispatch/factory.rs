//! Shared agent factory — reuses expensive components across agent dispatches.
//!
//! Per-task agent creation currently rebuilds `ProviderSemaphores`, runs MCP
//! discovery via `block_on` on an OS thread, and reconstructs the
//! `Dispatcher` / `PromptAssembler` / `WarmPool`.  `SharedAgentFactory`
//! creates these once at run start and hands them to every dispatch call.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use roko_agent::AgentRuntimeEvent;
use roko_agent::mcp::{McpConfig, discover_mcp_tools};
use roko_agent::provider::ProviderSemaphores;
use roko_compose::{AttentionBidder, LearningBidder};
use roko_core::config::schema::RokoConfig;
use roko_core::tool::ToolDef;
use tokio::sync::mpsc;

use crate::dispatch_v2::{
    AgentDispatchRequest, AgentDispatcherV2, ProviderDispatchResolver, ProviderRuntime,
};

use super::{
    AgentResultBridge, Dispatcher, PromptAssembler, PromptCache, ResolvedAgentRuntime, WarmPool,
};

/// Shared, reusable components for agent dispatch.
///
/// Constructed once at the start of a plan run.  The factory owns:
///
/// - **`ProviderSemaphores`** — concurrency limits per provider (no longer rebuilt per task).
/// - **`mcp_tools`** — MCP tool definitions discovered once via async (no `block_on` / OS thread).
/// - **`Dispatcher`** — model routing + prompt assembly + warm pool (stateless, reusable).
/// - **`ProviderDispatchResolver`** — model → provider resolution.
#[derive(Debug)]
pub struct SharedAgentFactory {
    config: Arc<RokoConfig>,
    semaphores: Arc<ProviderSemaphores>,
    mcp_tools: Option<Arc<Vec<ToolDef>>>,
    dispatcher: Dispatcher,
    resolver: ProviderDispatchResolver,
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

        let mcp_tools = match mcp_config_path {
            Some(path) => match McpConfig::load(path) {
                Ok(mcp_config) => match discover_mcp_tools(&mcp_config).await {
                    Ok(tools) => {
                        tracing::info!(tool_count = tools.len(), "factory: MCP tools discovered");
                        Some(Arc::new(tools))
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

        let prompt_assembler = match prompt_cache {
            Some(cache) => PromptAssembler::with_cache(cache),
            None => PromptAssembler::new(),
        };
        // Apply [prompt] config knobs from roko.toml.
        let prompt_assembler = prompt_assembler
            .with_composition_strategy(config.prompt.composition_strategy)
            .with_vcg_warmup_observations(config.prompt.vcg_warmup_observations);
        let dispatcher = Dispatcher::new(cascade_router, prompt_assembler, WarmPool::new(0));
        let resolver = ProviderDispatchResolver::new(Arc::clone(&config));

        Self {
            config,
            semaphores,
            mcp_tools,
            dispatcher,
            resolver,
        }
    }

    /// Read-only access to the shared dispatcher (for plan/route without acting).
    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    /// Swap the prompt assembler's cache without rebuilding expensive factory
    /// components (semaphores, MCP tools, resolver).
    ///
    /// Called after gate failures or when the periodic staleness check fires.
    pub fn update_prompt_cache(&mut self, cache: Arc<PromptCache>) {
        let assembler = PromptAssembler::with_cache(cache)
            .with_composition_strategy(self.config.prompt.composition_strategy)
            .with_vcg_warmup_observations(self.config.prompt.vcg_warmup_observations);
        self.dispatcher = Dispatcher::new(
            self.dispatcher.cascade_router_arc(),
            assembler,
            WarmPool::new(0),
        );
    }

    /// Set persisted learning bidders on the prompt assembler.
    ///
    /// Called at run startup after loading from `.roko/learn/attention-bidders.json`.
    /// The bidders are passed to `PromptComposer::with_learning_bidders` when the
    /// runner-v2 prompt path is routed through the canonical compose surface.
    pub fn set_learning_bidders(&mut self, bidders: HashMap<AttentionBidder, LearningBidder>) {
        let cascade = self.dispatcher.cascade_router_arc();
        let assembler = PromptAssembler::new()
            .with_composition_strategy(self.config.prompt.composition_strategy)
            .with_vcg_warmup_observations(self.config.prompt.vcg_warmup_observations)
            .with_learning_bidders(bidders);
        self.dispatcher = Dispatcher::new(cascade, assembler, WarmPool::new(0));
    }

    /// Resolve the runtime for a model key.
    pub fn resolve_runtime(&self, model_key: &str) -> Result<ResolvedAgentRuntime, String> {
        let spec = self.resolver.resolve(model_key);
        match spec.runtime {
            ProviderRuntime::Cli(provider) => Ok(ResolvedAgentRuntime::Cli {
                model: spec.model_slug,
                cli_provider: Some(provider),
            }),
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

    /// Spawn an API/provider-backed agent using shared semaphores and
    /// pre-discovered MCP tools.
    pub fn spawn_shared_agent_bridge(
        &self,
        request: AgentDispatchRequest,
        event_tx: mpsc::Sender<AgentRuntimeEvent>,
    ) -> tokio::task::JoinHandle<()> {
        let config = Arc::clone(&self.config);
        let semaphores = Arc::clone(&self.semaphores);
        let mcp_tools = self.mcp_tools.clone();

        tokio::spawn(async move {
            let dispatcher = AgentDispatcherV2::with_shared(config, semaphores);
            match dispatcher
                .run_agent_result_bridge_with_mcp(request, mcp_tools)
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

    /// Pre-discovered MCP tools, if available.
    pub fn mcp_tools(&self) -> Option<&Arc<Vec<ToolDef>>> {
        self.mcp_tools.as_ref()
    }
}
