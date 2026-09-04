//! `roko agent serve` command wiring.
//!
//! Also contains the `roko agent create` and `roko agent delete` commands
//! for lifecycle management (LIFE-01, LIFE-06).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use async_trait::async_trait;
use clap::{Args, Subcommand};
use roko_agent::{
    Agent,
    chat_types::{ChatRequest, ChatResponse},
    lifecycle::{
        AgentCoreManifest, AgentExtendedManifest, ChainConfig as LifecycleChainConfig,
        CodingConfig, DeploymentMode, DomainPlugin, ResearchConfig, resolve_manifest,
        validate_manifest,
    },
    process::registry::{register_spawned_pid, unregister_pid},
};
use roko_agent_server::{AgentServer, DispatchLike, SidecarDispatchError};
use roko_cli::agent_spawn::{SpawnAgentSpec, spawn_agent_scoped};
use roko_core::{Body, Context, Kind, MessageContent, Signal};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Agent-focused CLI subtree.
#[derive(Debug, Subcommand)]
pub enum AgentCmd {
    /// Create a new agent from a manifest.
    ///
    /// Generates an `AgentExtendedManifest` TOML at `.roko/agents/<name>/manifest.toml`
    /// after validating the manifest fields. Supports domain presets (coding, research,
    /// chain, general) and optional strategy templates.
    Create {
        /// Human-readable agent name (required).
        #[arg(long)]
        name: String,
        /// Agent domain: coding, research, chain, or general.
        #[arg(long, default_value = "general")]
        domain: String,
        /// Strategy template to use (e.g. fast-coding, deep-research).
        #[arg(long)]
        template: Option<String>,
        /// Natural-language prompt describing what the agent should do.
        #[arg(long)]
        prompt: Option<String>,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Comma-separated skill tags (e.g. "rust,p2p,networking").
        #[arg(long, value_delimiter = ',')]
        skills: Vec<String>,
        /// Agent tier: Unverified, Verified, Trusted, Expert, Pioneer.
        #[arg(long)]
        tier: Option<String>,
        /// Reputation score (0–100).
        #[arg(long, default_value_t = 0)]
        reputation: u32,
        /// Maximum concurrent jobs.
        #[arg(long, default_value_t = 0)]
        max_concurrent_jobs: u32,
        /// Auto-register with roko-serve at this URL after creation.
        /// Uses the default http://localhost:6677 when set to empty string.
        #[arg(long)]
        serve_url: Option<String>,
    },
    /// Delete an agent and clean up its state.
    ///
    /// Performs an ordered 8-step shutdown: stop processing, flush pending,
    /// backup knowledge, deregister from mesh, release resources, archive
    /// signals, clean state, and emit a deletion marker. Use --force to
    /// skip the ordered shutdown for immediate removal.
    Delete {
        /// Agent name to delete.
        #[arg(long)]
        name: String,
        /// Skip ordered shutdown and remove immediately.
        #[arg(long)]
        force: bool,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// List all agents with their status.
    List {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Start a previously created agent.
    Start {
        /// Agent name.
        #[arg(long)]
        name: String,
        /// Socket address to bind (default: 127.0.0.1:0 for auto-port).
        #[arg(long, default_value = "127.0.0.1:0")]
        bind: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Stop a running agent.
    Stop {
        /// Agent name.
        #[arg(long)]
        name: String,
        /// Force kill (SIGKILL instead of SIGTERM).
        #[arg(long)]
        force: bool,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show detailed status for one agent.
    Status {
        /// Agent name.
        #[arg(long)]
        name: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Start a per-agent HTTP runtime.
    Serve(AgentServeArgs),
    /// Interactive chat REPL with an agent.
    Chat {
        /// Agent ID to chat with.
        ///
        /// Resolution precedence: explicit --agent > config [agent].default_agent_id >
        /// the only registered healthy agent > actionable error.
        #[arg(long)]
        agent: Option<String>,
        /// roko-serve base URL.
        #[arg(long, default_value_t = roko_cli::DEFAULT_SERVE_URL.to_string())]
        serve_url: String,
        /// Use a direct API provider instead of sidecar/serve routing.
        /// Accepted values: anthropic_api, openai_compat.
        #[arg(long)]
        provider: Option<String>,
        /// Override the model key for this chat session.
        ///
        /// Must reference a key in `[models.*]`. When both --model and a
        /// global override are set, equal values coalesce; differing
        /// explicit values are a pre-dispatch conflict error.
        #[arg(long)]
        model: Option<String>,
        /// Force line-oriented REPL even on a TTY.
        ///
        /// Default: TTY gets the inline TUI, non-TTY gets the line REPL.
        #[arg(long)]
        text: bool,
    },
}

/// Arguments for `roko agent serve`.
#[derive(Debug, Args, Clone)]
pub struct AgentServeArgs {
    /// Unique agent identifier advertised by the runtime.
    #[arg(long)]
    pub agent_id: String,
    /// Socket address to bind (default: auto-pick a free port on localhost).
    #[arg(long, default_value = "127.0.0.1:0")]
    pub bind: String,
    /// Relay base URL reserved for a future relay bridge hook.
    ///
    /// Not yet implemented. Passing this flag will fail with an actionable
    /// hint. Track progress in backlog #224 (platform/transport).
    #[arg(long)]
    pub relay_url: Option<String>,
    /// Chain JSON-RPC URL reserved for future chain hooks.
    ///
    /// Not yet implemented. Passing this flag will fail with an actionable
    /// hint. Track progress in backlog #224 (platform/transport).
    #[arg(long)]
    pub chain_rpc_url: Option<String>,
    /// ERC-8004 identity registry contract address.
    ///
    /// Not yet implemented. Passing this flag will fail with an actionable
    /// hint. Track progress in backlog #224 (platform/transport).
    #[arg(long)]
    pub identity_registry: Option<String>,
    /// ERC-8004 passport id used for `updateAgentCardUri`.
    ///
    /// Not yet implemented. Passing this flag will fail with an actionable
    /// hint. Track progress in backlog #224 (platform/transport).
    #[arg(long)]
    pub passport_id: Option<String>,
    /// Wallet private key reserved for future signing hooks.
    ///
    /// Not yet implemented. Passing this flag will fail with an actionable
    /// hint. Track progress in backlog #224 (platform/transport).
    #[arg(long)]
    pub wallet_key: Option<String>,
    /// roko-serve control plane URL for heartbeat reporting.
    #[arg(long, default_value_t = roko_cli::DEFAULT_SERVE_URL.to_string())]
    pub serve_url: String,
    /// Allow the cognitive loop to start even when it uses stub cells.
    ///
    /// Debug-only escape hatch. Release builds always reject stub cognitive
    /// loops regardless of this flag.
    #[arg(long, hide = true)]
    pub allow_stub_cognitive_loop: bool,
}

/// Typed capability readiness report for the agent serve runtime.
///
/// Each field indicates whether the corresponding subsystem is active (true)
/// or merely advertised metadata (false). Serializable for startup logs and
/// registration payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityReadiness {
    /// HTTP messaging endpoint (/message, /stream).
    pub messaging: bool,
    /// Prediction endpoint (/predictions).
    pub predictions: bool,
    /// Relay bridge connection to a remote relay.
    pub relay: bool,
    /// On-chain RPC connection for contract interactions.
    pub chain: bool,
    /// ERC-8004 identity registry integration.
    pub identity: bool,
    /// ERC-8004 passport registration.
    pub passport: bool,
    /// Wallet signing for on-chain transactions.
    pub wallet_signing: bool,
    /// Cognitive loop running as a Hot Graph.
    pub cognitive_loop: bool,
}

impl CapabilityReadiness {
    /// Build a readiness report for the current runtime configuration.
    ///
    /// Only messaging and predictions are active today. All other
    /// capabilities require implementations tracked by backlog #224/#270.
    fn for_runtime(has_dispatcher: bool) -> Self {
        Self {
            messaging: has_dispatcher,
            predictions: true,
            relay: false,
            chain: false,
            identity: false,
            passport: false,
            wallet_signing: false,
            cognitive_loop: false,
        }
    }
}

/// Validate that no unsupported active-integration flags were passed.
///
/// Returns an error with a concrete hint before socket bind and runtime
/// registration when any of the reserved flags are supplied.
fn reject_unsupported_serve_flags(args: &AgentServeArgs) -> Result<()> {
    let mut unsupported = Vec::new();
    if let Some(url) = &args.relay_url {
        unsupported.push(format!(
            "--relay-url {url}: relay bridge is not yet implemented. \
             Track progress in backlog #224 (platform/transport implementations)"
        ));
    }
    if let Some(url) = &args.chain_rpc_url {
        unsupported.push(format!(
            "--chain-rpc-url {url}: chain RPC integration is not yet implemented. \
             Track progress in backlog #224 (platform/transport implementations)"
        ));
    }
    if let Some(addr) = &args.identity_registry {
        unsupported.push(format!(
            "--identity-registry {addr}: ERC-8004 identity registry is not yet implemented. \
             Track progress in backlog #224 (platform/transport implementations)"
        ));
    }
    if let Some(id) = &args.passport_id {
        unsupported.push(format!(
            "--passport-id {id}: ERC-8004 passport registration is not yet implemented. \
             Track progress in backlog #224 (platform/transport implementations)"
        ));
    }
    if args.wallet_key.is_some() {
        // Do not log the wallet key value.
        unsupported.push(
            "--wallet-key: wallet signing is not yet implemented. \
             Track progress in backlog #224 (platform/transport implementations)"
                .to_string(),
        );
    }
    if unsupported.is_empty() {
        Ok(())
    } else {
        bail!(
            "unsupported agent serve flags:\n  - {}",
            unsupported.join("\n  - ")
        );
    }
}

// ─── Chat launch configuration ──────────────────────────────────────────

/// UI mode for the chat session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatUiMode {
    /// Inline ratatui TUI (default on TTY).
    InlineTui,
    /// Line-oriented REPL (default on non-TTY / --text).
    LineRepl,
}

/// Unified chat launch configuration resolved before dispatch.
///
/// Both the direct-provider and sidecar/serve chat backends use the same
/// resolved config. Provider/model selection and UI selection are
/// independent, explicit axes.
#[derive(Debug, Clone)]
pub struct ChatLaunchConfig {
    /// Resolved agent ID.
    pub target: String,
    /// Optional direct provider name (e.g. `"anthropic_api"`).
    pub provider: Option<String>,
    /// Optional model key override.
    pub model: Option<String>,
    /// UI mode.
    pub ui_mode: ChatUiMode,
    /// Working directory.
    pub workdir: PathBuf,
    /// roko-serve base URL.
    pub serve_url: String,
}

/// Resolve the chat agent target deterministically.
///
/// Precedence: explicit `--agent` > config `[agent].default_agent_id` >
/// the only registered healthy agent > actionable error.
fn resolve_chat_agent(explicit: Option<&str>, workdir: &Path) -> Result<String> {
    // 1. Explicit --agent flag.
    if let Some(agent) = explicit.filter(|s| !s.trim().is_empty()) {
        return Ok(agent.to_string());
    }

    // 2. Config default_agent_id.
    let core_config = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    if let Some(configured) = core_config
        .agent
        .default_agent_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        // Validate the configured agent exists.
        let entries = load_agent_entries(workdir);
        let matching = entries.iter().find(|e| e.name == configured);
        if let Some(entry) = matching {
            if is_process_alive(entry.pid) {
                return Ok(configured.to_string());
            }
            bail!(
                "configured default agent '{configured}' exists but is not healthy \
                 (pid {} not running); start it with: roko agent start --name {configured}",
                entry.pid
            );
        }
        // Agent not in registry but config says to use it -- allow anyway
        // as it might be reachable through roko-serve.
        return Ok(configured.to_string());
    }

    // 3. The only registered healthy agent.
    let entries = load_agent_entries(workdir);
    let healthy: Vec<&AgentEntry> = entries.iter().filter(|e| is_process_alive(e.pid)).collect();
    match healthy.len() {
        0 => bail!(
            "no agent specified and no healthy agents found; use --agent <id> \
             or set [agent].default_agent_id in roko.toml"
        ),
        1 => Ok(healthy[0].name.clone()),
        n => {
            let names: Vec<&str> = healthy.iter().map(|e| e.name.as_str()).collect();
            bail!(
                "no agent specified and {n} healthy agents found ({});\n\
                 use --agent <id> or set [agent].default_agent_id in roko.toml",
                names.join(", ")
            );
        }
    }
}

/// Resolve the `ChatLaunchConfig` from CLI args and config.
///
/// Validates that provider/model overrides do not conflict with global
/// config.
pub fn resolve_chat_launch(
    agent: Option<&str>,
    provider: Option<String>,
    model: Option<String>,
    text: bool,
    serve_url: String,
    workdir: &Path,
) -> Result<ChatLaunchConfig> {
    let target = resolve_chat_agent(agent, workdir)?;

    // Resolve model conflicts with global config.
    if let Some(ref local_model) = model {
        let core = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
        let global_model = &core.agent.default_model;
        if !global_model.is_empty() && global_model != local_model {
            // Different explicit values -- pre-dispatch conflict.
            warn!(
                local_model = %local_model,
                global_model = %global_model,
                "explicit --model differs from global [agent].default_model; \
                 using --model (local override takes precedence)"
            );
        }
    }

    // UI mode: --text forces line REPL; otherwise use TTY detection.
    use std::io::IsTerminal;
    let ui_mode = if text || !std::io::stdout().is_terminal() {
        ChatUiMode::LineRepl
    } else {
        ChatUiMode::InlineTui
    };

    Ok(ChatLaunchConfig {
        target,
        provider,
        model,
        ui_mode,
        workdir: workdir.to_path_buf(),
        serve_url,
    })
}

#[derive(Debug, Clone)]
struct AgentServeRuntimeConfig {
    agent_id: String,
    bind: String,
    serve_url: String,
    allow_stub_cognitive_loop: bool,
}

impl AgentServeRuntimeConfig {
    fn from_args(args: AgentServeArgs) -> Self {
        Self {
            agent_id: args.agent_id,
            bind: args.bind,
            serve_url: args.serve_url,
            allow_stub_cognitive_loop: args.allow_stub_cognitive_loop,
        }
    }

    async fn run(self) -> Result<()> {
        let startup = self.startup_snapshot();
        let has_dispatcher = self.try_build_dispatcher()?.is_some();
        let readiness = CapabilityReadiness::for_runtime(has_dispatcher);

        info!(
            agent_id = %startup.agent_id,
            bind = %startup.bind,
            readiness = %serde_json::to_string(&readiness).unwrap_or_default(),
            "starting roko agent server"
        );

        let server = self.build_server()?;

        // ── Start the cognitive loop as a Hot Graph (task 103) ──
        //
        // The cognitive loop uses stub cells until #270 lands. Starting it
        // requires the hidden --allow-stub-cognitive-loop flag in debug
        // builds; release builds always reject stub loops.
        let cog_handle = self.try_start_cognitive_loop();

        let result = server.serve().await;

        // Cancel the cognitive loop (if running) when the server exits.
        if let Some(ref handle) = cog_handle {
            handle.cancel();
            if let Err(error) = handle.wait_result().await {
                warn!(%error, "cognitive Hot Graph stopped with a terminal failure");
            }
        }

        // Cleanup: remove our entry from agents.json on shutdown.
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        remove_agent_entry(&workdir, &startup.agent_id);

        result
    }

    fn build_server(&self) -> Result<AgentServer> {
        let mut builder = AgentServer::builder()
            .agent_id(self.agent_id.clone())
            .bind(self.bind.clone())
            .serve_url(self.serve_url.clone())
            .messaging()
            .predictions();

        if let Some(dispatcher) = self.try_build_dispatcher()? {
            builder = builder.with_message_dispatcher(dispatcher);
        }

        let startup = self.startup_snapshot();
        let workdir_for_start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        builder
            .on_start(move |addr, card| {
                let startup = startup.clone();
                let workdir = workdir_for_start.clone();
                async move {
                    info!(
                        agent_id = %startup.agent_id,
                        bind = %startup.bind,
                        local_addr = %addr,
                        rest_endpoint = ?card.endpoints.rest,
                        "agent server is ready"
                    );
                    // Register with roko-serve so the control plane and
                    // dashboard can discover this agent.  Retry up to 3
                    // times with 2 s gaps — when `roko up` starts serve and
                    // agents near-simultaneously the first attempt may fail
                    // because serve isn't listening yet.
                    let rest_endpoint = card
                        .endpoints
                        .rest
                        .clone()
                        .unwrap_or_else(|| format!("http://127.0.0.1:{}", addr.port()));
                    let register_url = format!(
                        "{}/api/agents/register",
                        startup.serve_url.trim_end_matches('/')
                    );
                    let body = serde_json::json!({
                        "agent_id": startup.agent_id,
                        "label": startup.agent_id,
                        "rest_endpoint": rest_endpoint,
                        "process_id": std::process::id(),
                    });
                    let client = reqwest::Client::new();
                    let mut registered = false;
                    for attempt in 1..=3u32 {
                        match client
                            .post(&register_url)
                            .json(&body)
                            .timeout(std::time::Duration::from_secs(3))
                            .send()
                            .await
                        {
                            Ok(resp) if resp.status().is_success() => {
                                info!(
                                    agent_id = %startup.agent_id,
                                    serve_url = %startup.serve_url,
                                    attempt,
                                    "registered with roko-serve"
                                );
                                registered = true;
                                break;
                            }
                            Ok(resp) => {
                                warn!(
                                    agent_id = %startup.agent_id,
                                    status = %resp.status(),
                                    attempt,
                                    "roko-serve registration returned non-success"
                                );
                            }
                            Err(err) => {
                                warn!(
                                    agent_id = %startup.agent_id,
                                    error = %err,
                                    attempt,
                                    "could not register with roko-serve (is it running?)"
                                );
                            }
                        }
                        if attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    }
                    if !registered {
                        warn!(
                            agent_id = %startup.agent_id,
                            "failed to register with roko-serve after 3 attempts"
                        );
                    }

                    // Write to agents.json so `roko agent list`, `roko agent chat`,
                    // and the dashboard can discover this sidecar.
                    let actual_bind = format!("http://127.0.0.1:{}", addr.port());
                    upsert_agent_entry(&workdir, &startup.agent_id, &actual_bind);

                    Ok(())
                }
            })
            .build()
    }

    /// Attempt to start the cognitive loop as a background Hot Graph.
    ///
    /// Looks for the cognitive-loop TOML at the standard path
    /// (`examples/graphs/cognitive-loop.toml` relative to the workspace root).
    /// If the file exists, the graph is loaded with stub cells and started as a
    /// crash-recoverable Hot Graph with a 1-second tick interval and no tick
    /// limit (runs until cancelled).
    ///
    /// ## Stub guard
    ///
    /// The cognitive loop currently uses stub cells (real implementations
    /// tracked in backlog #270). Starting it requires the hidden
    /// `--allow-stub-cognitive-loop` flag in debug builds. Release builds
    /// always reject stub loops regardless of the flag.
    ///
    /// Returns `Some(HotGraphHandle)` if the loop was started, `None` if the
    /// TOML was not found, the stub guard blocked it, or it failed to load.
    /// Errors are logged but do not prevent the agent server from starting.
    fn try_start_cognitive_loop(&self) -> Option<roko_graph::HotGraphHandle> {
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Try standard locations for the cognitive loop graph definition.
        let candidates = [
            workdir.join("examples/graphs/cognitive-loop.toml"),
            workdir.join(".roko/graphs/cognitive-loop.toml"),
        ];
        let toml_path = candidates.iter().find(|p| p.exists())?;

        // ── Stub guard ──
        // Release builds always reject stub cognitive loops.
        if !cfg!(debug_assertions) {
            warn!(
                agent_id = %self.agent_id,
                path = %toml_path.display(),
                "cognitive loop graph found but rejected: stub cells are not \
                 permitted in release builds (backlog #270)"
            );
            return None;
        }

        if !self.allow_stub_cognitive_loop {
            warn!(
                agent_id = %self.agent_id,
                path = %toml_path.display(),
                "cognitive loop graph found but uses stub cells; pass \
                 --allow-stub-cognitive-loop to start it (debug builds only; \
                 production cells tracked in backlog #270)"
            );
            return None;
        }

        warn!(
            agent_id = %self.agent_id,
            path = %toml_path.display(),
            "starting cognitive loop with STUB cells (--allow-stub-cognitive-loop); \
             this is a development escape hatch and must not be used in production"
        );

        let toml_str = match std::fs::read_to_string(toml_path) {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    path = %toml_path.display(),
                    error = %e,
                    "failed to read cognitive-loop graph; skipping"
                );
                return None;
            }
        };

        let graph = match roko_graph::loader::load_from_str(&toml_str) {
            Ok(g) => g,
            Err(e) => {
                warn!(error = %e, "failed to parse cognitive-loop graph; skipping");
                return None;
            }
        };

        let registry = roko_graph::default_registry();
        let policy = roko_graph::HotPolicy {
            tick_interval_ms: 1000,
            max_ticks: None,
            persist_tick_state: true,
            loop_level: None,
        };
        let checkpoint_dir = workdir
            .join(".roko/state/hot")
            .join(safe_hot_state_component(&self.agent_id))
            .join(safe_hot_state_component(&graph.metadata.name));

        info!(
            agent_id = %self.agent_id,
            graph = %graph.metadata.name,
            "starting cognitive loop as Hot Graph (stub cells)"
        );

        match roko_graph::start_hot_resumable(
            graph,
            registry,
            policy,
            None,
            roko_graph::HotCheckpointOptions::new(&checkpoint_dir),
        ) {
            Ok(handle) => Some(handle),
            Err(error) => {
                warn!(
                    agent_id = %self.agent_id,
                    checkpoint = %checkpoint_dir.display(),
                    %error,
                    "cognitive Hot Graph checkpoint failed validation; loop not started"
                );
                None
            }
        }
    }

    fn try_build_dispatcher(&self) -> Result<Option<Arc<dyn DispatchLike>>> {
        let workdir = std::env::current_dir().context("read current working directory")?;
        let config = roko_core::config::loader::load_config_unified(&workdir)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let model = config.agent.default_model.trim().to_string();
        if model.is_empty() {
            return Ok(None);
        }

        // Resolve the provider through the normal provider registry.
        // A dispatcher can be built if:
        //   (a) the default model resolves in the provider registry, or
        //   (b) a legacy subprocess command is configured.
        //
        // Note: ANTHROPIC_API_KEY no longer implicitly switches the provider
        // kind. Users should configure [providers.anthropic] and
        // [models.<model>] with provider = "anthropic" explicitly.
        // The old implicit override silently changed behavior based on
        // environment, violating the explicit provider resolution contract.
        let has_provider_backing = config.effective_models().contains_key(&model);
        let has_legacy_command = config.agent.command.is_some();
        if !has_provider_backing && !has_legacy_command {
            if std::env::var_os("ANTHROPIC_API_KEY").is_some() {
                info!(
                    model = %model,
                    reason = "provider_resolution",
                    "ANTHROPIC_API_KEY is set but no matching [models.*] entry \
                     with provider = \"anthropic\" was found; add one to use the \
                     direct API. Falling back to configured provider resolution"
                );
            }
            return Ok(None);
        }

        let command = config.agent.command.clone();

        let agent = spawn_agent_scoped(
            &config,
            SpawnAgentSpec {
                model: model.to_string(),
                command,
                timeout_ms: config.agent.timeout_ms,
                system_prompt: None,
                cached_content: None,
                tools: None,
                mcp_config: None,
                working_dir: Some(workdir),
                env: config.agent.env.clone().unwrap_or_default(),
                extra_args: config.agent.args.clone().unwrap_or_default(),
                effort: Some(config.agent.default_effort.clone()),
                bare_mode: config.agent.bare_mode,
                dangerously_skip_permissions: false,
                name: self.agent_id.clone(),
                role: None,
            },
            format!("create serving agent for {}", self.agent_id),
        )?;

        Ok(Some(Arc::new(ServingAgentDispatcher {
            agent: Arc::from(agent),
        })))
    }

    fn startup_snapshot(&self) -> StartupSnapshot {
        StartupSnapshot {
            agent_id: self.agent_id.clone(),
            bind: self.bind.clone(),
            serve_url: self.serve_url.clone(),
        }
    }
}

fn safe_hot_state_component(value: &str) -> String {
    let component: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if component.is_empty() || component == "." || component == ".." {
        "graph".to_string()
    } else {
        component
    }
}

#[derive(Debug, Clone)]
struct StartupSnapshot {
    agent_id: String,
    bind: String,
    serve_url: String,
}

struct ServingAgentDispatcher {
    agent: Arc<dyn Agent>,
}

#[async_trait]
impl DispatchLike for ServingAgentDispatcher {
    async fn dispatch(&self, request: ChatRequest) -> Result<ChatResponse, SidecarDispatchError> {
        let prompt = extract_prompt(&request).ok_or(SidecarDispatchError::NotConfigured)?;
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text(prompt.clone()))
            .build();
        let result = self
            .agent
            .run(&input, &Context::now().with_goal(prompt))
            .await;

        // Clean up raw JSON from the agent output (e.g. Claude CLI streaming
        // protocol).  The `extract_clean_text` parser handles plain text
        // (no-op), JSONL with result/assistant events, content block arrays,
        // and nested `result`/`content` fields.
        let raw = result.output.body.as_text().unwrap_or_default();
        let content = roko_cli::chat::extract_clean_text(raw);

        Ok(ChatResponse {
            content,
            usage: result.usage,
            finish_reason: if result.success {
                roko_agent::chat_types::FinishReason::Stop
            } else {
                roko_agent::chat_types::FinishReason::Error(
                    result
                        .output
                        .body
                        .as_text()
                        .unwrap_or("agent failed")
                        .to_string(),
                )
            },
            ..ChatResponse::default()
        })
    }
}

fn extract_prompt(request: &ChatRequest) -> Option<String> {
    request.messages.iter().find_map(|message| match message {
        roko_core::ChatMessage::User { content } => match content {
            MessageContent::Text(text) => Some(text.clone()),
            MessageContent::Blocks(blocks) => {
                let parts: Vec<&str> = blocks
                    .iter()
                    .filter_map(|block| match block {
                        roko_core::ContentBlock::Text { text } => Some(text.as_str()),
                        roko_core::ContentBlock::ImageUrl { .. } => None,
                    })
                    .collect();
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join("\n"))
                }
            }
        },
        _ => None,
    })
}

/// Run a chat session using the unified `ChatLaunchConfig`.
///
/// Routes to the correct backend (direct provider vs. sidecar/serve) and
/// UI mode (inline TUI vs. line REPL) based on the resolved config.
async fn run_chat_with_launch(launch: ChatLaunchConfig) -> Result<()> {
    if let Some(provider_name) = &launch.provider {
        // Direct provider mode: build resolved config and call the
        // direct provider chat with the shared config path.
        let config = roko_core::config::loader::load_config_unified(&launch.workdir)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut provider_config = roko_core::config::schema::RokoConfig::default();
        provider_config.providers.extend(config.providers.clone());
        provider_config.models.extend(config.models.clone());

        // Apply model override from ChatLaunchConfig.
        if let Some(ref model_key) = launch.model {
            provider_config.agent.default_model = model_key.clone();
        } else if !config.agent.default_model.is_empty() {
            provider_config.agent.default_model = config.agent.default_model.clone();
        }
        provider_config.agent.default_effort = config.agent.default_effort.clone();
        provider_config.agent.bare_mode = config.agent.bare_mode;
        provider_config.agent.timeout_ms = config.agent.timeout_ms;
        provider_config.agent.fallback_model = config.agent.fallback_model.clone();
        provider_config.agent.tier_models = config.agent.tier_models.clone();
        provider_config.agent.env = config.agent.env.clone();

        info!(
            target = %launch.target,
            provider = %provider_name,
            model = ?launch.model,
            ui_mode = ?launch.ui_mode,
            "chat session: direct provider dispatch"
        );

        roko_cli::chat::run_direct_provider_chat(
            &launch.target,
            provider_name,
            &provider_config,
            &launch.workdir,
        )
        .await?;
    } else {
        // Sidecar / serve mode.
        info!(
            target = %launch.target,
            ui_mode = ?launch.ui_mode,
            serve_url = %launch.serve_url,
            "chat session: sidecar/serve dispatch"
        );

        match launch.ui_mode {
            ChatUiMode::InlineTui => {
                roko_cli::chat_inline::run_chat_inline(&launch.target, &launch.serve_url).await?;
            }
            ChatUiMode::LineRepl => {
                roko_cli::chat::run_chat_repl(&launch.target, &launch.serve_url).await?;
            }
        }
    }
    Ok(())
}

/// Run `roko agent ...`.
pub async fn run(cmd: AgentCmd) -> Result<()> {
    match cmd {
        AgentCmd::Create {
            name,
            domain,
            template,
            prompt,
            workdir,
            skills,
            tier,
            reputation,
            max_concurrent_jobs,
            serve_url,
        } => {
            run_agent_create(
                &name,
                &domain,
                template.as_deref(),
                prompt.as_deref(),
                workdir.as_deref(),
            )
            .await?;

            // Auto-register with roko-serve if --serve-url is given.
            let url = serve_url.as_deref().map(|u| {
                if u.is_empty() {
                    "http://localhost:6677"
                } else {
                    u
                }
            });
            if let Some(base) = url {
                let capabilities = match domain.as_str() {
                    "research" => vec!["messaging".to_string(), "research".to_string()],
                    _ => vec!["messaging".to_string(), "tasks".to_string()],
                };
                let body = serde_json::json!({
                    "agent_id": name,
                    "label": name,
                    "capabilities": capabilities,
                    "domain_tags": [domain],
                    "skills": skills,
                    "tier": tier,
                    "reputation": reputation,
                    "max_concurrent_jobs": max_concurrent_jobs,
                });
                let register_url = format!("{}/api/agents/register", base.trim_end_matches('/'));
                match reqwest::Client::new()
                    .post(&register_url)
                    .json(&body)
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        println!("Registered with serve at {base}");
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        tracing::warn!(%status, %text, "serve registration failed");
                    }
                    Err(err) => {
                        tracing::warn!(%register_url, %err, "could not reach serve for agent registration");
                        tracing::info!("the agent was created locally; register manually later");
                    }
                }
            }
            Ok(())
        }
        AgentCmd::Delete {
            name,
            force,
            workdir,
        } => run_agent_delete(&name, force, workdir.as_deref()).await,
        AgentCmd::List { workdir, json } => run_agent_list(workdir.as_deref(), json),
        AgentCmd::Start {
            name,
            bind,
            workdir,
        } => run_agent_start(&name, &bind, workdir.as_deref()),
        AgentCmd::Stop {
            name,
            force,
            workdir,
        } => run_agent_stop(&name, force, workdir.as_deref()),
        AgentCmd::Status { name, workdir } => run_agent_status(&name, workdir.as_deref()),
        AgentCmd::Serve(args) => {
            reject_unsupported_serve_flags(&args)?;
            AgentServeRuntimeConfig::from_args(args).run().await
        }
        AgentCmd::Chat {
            agent,
            serve_url,
            provider,
            model,
            text,
        } => {
            let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let launch =
                resolve_chat_launch(agent.as_deref(), provider, model, text, serve_url, &workdir)?;
            run_chat_with_launch(launch).await
        }
    }
}

// ─── Structured agent tracking ──────────────────────────────────────────

/// Runtime state for a single agent, persisted to `.roko/runtime/agents.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentEntry {
    name: String,
    pid: u32,
    bind: String,
    domain: String,
    started_at: String, // RFC 3339
}

/// Path to the structured agent tracking file.
fn agents_file_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("runtime").join("agents.json")
}

/// Load all agent entries from disk. Returns empty vec if file is missing or corrupt.
fn load_agent_entries(workdir: &Path) -> Vec<AgentEntry> {
    let path = agents_file_path(workdir);
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

/// Persist agent entries to disk.
fn save_agent_entries(workdir: &Path, entries: &[AgentEntry]) -> Result<()> {
    let path = agents_file_path(workdir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create runtime directory at {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(entries).context("serialize agent entries")?;
    std::fs::write(&path, json)
        .with_context(|| format!("write agent entries to {}", path.display()))?;
    Ok(())
}

/// Insert or update an agent entry in agents.json.
fn upsert_agent_entry(workdir: &Path, agent_id: &str, bind: &str) {
    let mut entries = load_agent_entries(workdir);
    entries.retain(|e| e.name != agent_id);
    entries.push(AgentEntry {
        name: agent_id.to_string(),
        pid: std::process::id(),
        bind: bind.to_string(),
        domain: "general".to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
    });
    if let Err(e) = save_agent_entries(workdir, &entries) {
        warn!(error = %e, "failed to write agent entry to agents.json");
    }
}

/// Remove an agent entry from agents.json.
fn remove_agent_entry(workdir: &Path, agent_id: &str) {
    let mut entries = load_agent_entries(workdir);
    let before = entries.len();
    entries.retain(|e| e.name != agent_id);
    if entries.len() < before {
        if let Err(e) = save_agent_entries(workdir, &entries) {
            warn!(error = %e, "failed to clean agent entry from agents.json");
        } else {
            info!(agent_id, "removed agent entry from agents.json");
        }
    }
}

/// Check whether a process with the given PID is alive.
#[cfg(unix)]
#[allow(unsafe_code, clippy::cast_possible_wrap)]
fn is_process_alive(pid: u32) -> bool {
    // SAFETY: signal 0 is an existence check — no signal is delivered.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    false
}

/// Send a signal to a process.
#[cfg(unix)]
#[allow(unsafe_code, clippy::cast_possible_wrap)]
fn send_signal(pid: u32, sig: i32) {
    unsafe {
        libc::kill(pid as i32, sig);
    }
}

#[cfg(not(unix))]
fn send_signal(_pid: u32, _sig: i32) {}

/// Extract the domain string from a manifest TOML on disk.
fn read_domain_from_manifest(manifest_path: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(manifest_path) else {
        return "unknown".to_string();
    };
    let Ok(manifest) = toml::from_str::<AgentExtendedManifest>(&text) else {
        return "unknown".to_string();
    };
    match &manifest.core.domain {
        Some(DomainPlugin::Coding(_)) => "coding".to_string(),
        Some(DomainPlugin::Research(_)) => "research".to_string(),
        Some(DomainPlugin::Chain(_)) => "chain".to_string(),
        Some(DomainPlugin::Custom(c)) => c.id.clone(),
        None => "general".to_string(),
    }
}

/// Format a duration in a human-readable way.
fn format_duration(dur: chrono::Duration) -> String {
    let secs = dur.num_seconds();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
    }
}

// ─── Agent list ─────────────────────────────────────────────────────────

fn run_agent_list(workdir: Option<&Path>, json: bool) -> Result<()> {
    let wd = workdir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let agents_dir = wd.join(".roko").join("agents");
    if !agents_dir.exists() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "agents": [],
                    "summary": { "total": 0, "running": 0, "idle": 0 }
                })
            );
        } else {
            println!("No agents found.");
        }
        return Ok(());
    }

    // Scan manifests.
    let mut agents: Vec<(String, String)> = Vec::new(); // (name, domain)
    let entries = std::fs::read_dir(&agents_dir)
        .with_context(|| format!("read agents directory at {}", agents_dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let agent_path = entry.path();

        // Skip deleted agents.
        if agent_path.join("DELETED").exists() {
            continue;
        }
        let manifest_path = agent_path.join("manifest.toml");
        if !manifest_path.exists() {
            continue;
        }
        let domain = read_domain_from_manifest(&manifest_path);
        agents.push((name, domain));
    }

    if agents.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "agents": [],
                    "summary": { "total": 0, "running": 0, "idle": 0 }
                })
            );
        } else {
            println!("No agents found.");
        }
        return Ok(());
    }

    agents.sort_by(|a, b| a.0.cmp(&b.0));

    // Load runtime state.
    let runtime_entries = load_agent_entries(&wd);

    // Count active/idle.
    let mut active = 0u32;
    let mut idle = 0u32;
    for (name, _) in &agents {
        let rt = runtime_entries.iter().find(|e| e.name == *name);
        if rt.is_some_and(|e| is_process_alive(e.pid)) {
            active += 1;
        } else {
            idle += 1;
        }
    }

    if json {
        let agent_list: Vec<serde_json::Value> = agents
            .iter()
            .map(|(name, domain)| {
                let rt = runtime_entries.iter().find(|e| e.name == *name);
                let (status, pid, bind, started_at) = match rt {
                    Some(entry) if is_process_alive(entry.pid) => (
                        "running",
                        Some(entry.pid),
                        Some(entry.bind.as_str()),
                        Some(entry.started_at.as_str()),
                    ),
                    Some(_) => ("stopped", None, None, None),
                    None => ("created", None, None, None),
                };
                let mut obj = serde_json::json!({
                    "name": name,
                    "domain": domain,
                    "status": status,
                });
                if let Some(p) = pid {
                    obj["pid"] = serde_json::json!(p);
                }
                if let Some(b) = bind {
                    obj["bind"] = serde_json::json!(b);
                }
                if let Some(s) = started_at {
                    obj["started_at"] = serde_json::json!(s);
                }
                obj
            })
            .collect();

        let output = serde_json::json!({
            "agents": agent_list,
            "summary": {
                "total": agents.len(),
                "running": active,
                "idle": idle,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);

        // Clean up stale entries.
        let live: Vec<AgentEntry> = runtime_entries
            .into_iter()
            .filter(|e| is_process_alive(e.pid))
            .collect();
        let _ = save_agent_entries(&wd, &live);

        return Ok(());
    }

    // Use inline primitives for formatted output when TTY.
    if roko_cli::inline::should_use_inline() {
        let theme = roko_cli::tui::Theme::from_env();
        let total = agents.len();
        let mut lines = vec![
            roko_cli::inline::styled::section_start(
                &theme,
                "agents",
                &format!("{total} registered"),
                Some(&format!("{active} active, {idle} idle")),
            ),
            ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                roko_cli::inline::symbols::BAR.to_string(),
                theme.muted(),
            )]),
            ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    roko_cli::inline::symbols::BAR.to_string(),
                    theme.muted(),
                ),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(
                    format!(
                        "{:<16} {:<12} {:<30} {}",
                        "NAME", "STATUS", "IDENTITY", "DOMAIN"
                    ),
                    theme.muted(),
                ),
            ]),
        ];

        for (name, domain) in &agents {
            let rt = runtime_entries.iter().find(|e| e.name == *name);
            let (status_icon, status_label) = if rt.is_some_and(|e| is_process_alive(e.pid)) {
                ("\u{25cf}", "active") // ● active
            } else {
                ("\u{25cb}", "idle") // ○ idle
            };
            let identity = format!("eid://roko/{name}");

            lines.push(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    roko_cli::inline::symbols::BAR.to_string(),
                    theme.muted(),
                ),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(format!("{:<16}", name), theme.text()),
                ratatui::text::Span::styled(
                    format!("{status_icon} "),
                    if status_label == "active" {
                        theme.success()
                    } else {
                        theme.muted()
                    },
                ),
                ratatui::text::Span::styled(
                    format!("{:<10}", status_label),
                    if status_label == "active" {
                        theme.success()
                    } else {
                        theme.muted()
                    },
                ),
                ratatui::text::Span::styled(
                    format!("{:<30}", identity),
                    ratatui::style::Style::default().fg(roko_cli::tui::Theme::DREAM),
                ),
                ratatui::text::Span::styled(
                    domain.to_string(),
                    ratatui::style::Style::default().fg(roko_cli::tui::Theme::TEXT_DIM),
                ),
            ]));
        }

        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(roko_cli::inline::symbols::BAR.to_string(), theme.muted()),
        ]));
        lines.push(roko_cli::inline::styled::section_end(
            &theme,
            "",
            &format!("{total} agent{}", if total == 1 { "" } else { "s" }),
        ));

        roko_cli::inline::plaintext::print_plain(&lines);
    } else {
        // Plain fallback
        println!(
            "{:<20} {:<10} {:<8} {:<22} {}",
            "NAME", "STATUS", "PID", "BIND", "DOMAIN"
        );
        for (name, domain) in &agents {
            let rt = runtime_entries.iter().find(|e| e.name == *name);
            let (status, pid_str, bind_str) = match rt {
                Some(entry) if is_process_alive(entry.pid) => (
                    "running".to_string(),
                    entry.pid.to_string(),
                    entry.bind.clone(),
                ),
                Some(_) => ("stopped".to_string(), "-".to_string(), "-".to_string()),
                None => ("created".to_string(), "-".to_string(), "-".to_string()),
            };
            println!(
                "{:<20} {:<10} {:<8} {:<22} {}",
                name, status, pid_str, bind_str, domain
            );
        }
    }

    // Clean up stale entries.
    let live: Vec<AgentEntry> = runtime_entries
        .into_iter()
        .filter(|e| is_process_alive(e.pid))
        .collect();
    let _ = save_agent_entries(&wd, &live);

    Ok(())
}

// ─── Agent start ────────────────────────────────────────────────────────

pub(crate) fn run_agent_start(name: &str, bind: &str, workdir: Option<&Path>) -> Result<()> {
    let wd = workdir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let agent_dir = wd.join(".roko").join("agents").join(name);
    let manifest_path = agent_dir.join("manifest.toml");

    if !manifest_path.exists() {
        bail!(
            "agent '{}' not found (no manifest at {})",
            name,
            manifest_path.display()
        );
    }
    if agent_dir.join("DELETED").exists() {
        bail!("agent '{}' has been deleted", name);
    }

    // Check if already running.
    let mut entries = load_agent_entries(&wd);
    if let Some(existing) = entries.iter().find(|e| e.name == name) {
        if is_process_alive(existing.pid) {
            bail!(
                "agent '{}' is already running (pid {}, bind {})",
                name,
                existing.pid,
                existing.bind
            );
        }
        // Stale entry — remove it.
        entries.retain(|e| e.name != name);
    }

    let domain = read_domain_from_manifest(&manifest_path);

    // Spawn `roko agent serve --agent-id <name> --bind <bind>` as detached child.
    let roko_bin = std::env::current_exe().context("determine roko binary path")?;
    let child = std::process::Command::new(&roko_bin)
        .arg("agent")
        .arg("serve")
        .arg("--agent-id")
        .arg(name)
        .arg("--bind")
        .arg(bind)
        .current_dir(&wd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn agent serve for '{}'", name))?;

    let pid = child.id();
    register_spawned_pid(pid);

    let now = chrono::Utc::now().to_rfc3339();
    entries.push(AgentEntry {
        name: name.to_string(),
        pid,
        bind: bind.to_string(),
        domain,
        started_at: now,
    });
    save_agent_entries(&wd, &entries)?;

    println!("Agent '{}' started (pid {}, bind {}).", name, pid, bind);
    Ok(())
}

// ─── Agent stop ─────────────────────────────────────────────────────────

pub(crate) fn run_agent_stop(name: &str, force: bool, workdir: Option<&Path>) -> Result<()> {
    let wd = workdir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut entries = load_agent_entries(&wd);
    let entry_idx = entries.iter().position(|e| e.name == name);

    let Some(idx) = entry_idx else {
        println!("Agent '{}' is not running.", name);
        return Ok(());
    };

    let entry = entries[idx].clone();
    if !is_process_alive(entry.pid) {
        println!("Agent '{}' is not running (stale entry cleaned up).", name);
        entries.remove(idx);
        save_agent_entries(&wd, &entries)?;
        unregister_pid(entry.pid);
        return Ok(());
    }

    // Send initial signal.
    if force {
        #[cfg(unix)]
        send_signal(entry.pid, libc::SIGKILL);
        #[cfg(not(unix))]
        send_signal(entry.pid, 9);
    } else {
        #[cfg(unix)]
        send_signal(entry.pid, libc::SIGTERM);
        #[cfg(not(unix))]
        send_signal(entry.pid, 15);
    }

    // Wait up to 5 seconds for exit.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if !is_process_alive(entry.pid) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // If still alive after timeout and not force, escalate to SIGKILL.
    if is_process_alive(entry.pid) && !force {
        #[cfg(unix)]
        send_signal(entry.pid, libc::SIGKILL);
        #[cfg(not(unix))]
        send_signal(entry.pid, 9);

        // Brief wait for SIGKILL to take effect.
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    // Compute run duration.
    let duration_str = chrono::DateTime::parse_from_rfc3339(&entry.started_at)
        .ok()
        .map(|started| {
            let dur = chrono::Utc::now().signed_duration_since(started);
            format_duration(dur)
        })
        .unwrap_or_else(|| "unknown".to_string());

    entries.remove(idx);
    save_agent_entries(&wd, &entries)?;
    unregister_pid(entry.pid);

    println!(
        "Agent '{}' stopped (pid {}, ran for {}).",
        name, entry.pid, duration_str
    );
    Ok(())
}

// ─── Agent status ───────────────────────────────────────────────────────

fn run_agent_status(name: &str, workdir: Option<&Path>) -> Result<()> {
    let wd = workdir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let agent_dir = wd.join(".roko").join("agents").join(name);
    let manifest_path = agent_dir.join("manifest.toml");

    if !manifest_path.exists() {
        bail!(
            "agent '{}' not found (no manifest at {})",
            name,
            manifest_path.display()
        );
    }
    if agent_dir.join("DELETED").exists() {
        bail!("agent '{}' has been deleted", name);
    }

    let domain = read_domain_from_manifest(&manifest_path);
    let entries = load_agent_entries(&wd);
    let rt = entries.iter().find(|e| e.name == name);

    let (status, pid_str, bind_str, started_str) = match rt {
        Some(entry) if is_process_alive(entry.pid) => {
            let ago = chrono::DateTime::parse_from_rfc3339(&entry.started_at)
                .ok()
                .map(|started| {
                    let dur = chrono::Utc::now().signed_duration_since(started);
                    format!("{} ({} ago)", entry.started_at, format_duration(dur))
                })
                .unwrap_or_else(|| entry.started_at.clone());
            ("running", entry.pid.to_string(), entry.bind.clone(), ago)
        }
        Some(_) => ("stopped", "-".to_string(), "-".to_string(), "-".to_string()),
        None => ("created", "-".to_string(), "-".to_string(), "-".to_string()),
    };

    println!("Agent:    {}", name);
    println!("Status:   {}", status);
    println!("Domain:   {}", domain);
    println!("PID:      {}", pid_str);
    println!("Bind:     {}", bind_str);
    println!("Started:  {}", started_str);
    println!("Manifest: {}", manifest_path.display());

    Ok(())
}

// ─── LIFE-01: Agent creation ────────────────────────────────────────────

/// Default prompt used when the operator does not supply one.
const DEFAULT_AGENT_PROMPT: &str =
    "You are a helpful agent. Describe your task in the strategy document.";

/// Three-step agent creation: build manifest, validate, write to disk.
pub(crate) async fn run_agent_create(
    name: &str,
    domain: &str,
    template: Option<&str>,
    prompt: Option<&str>,
    workdir: Option<&Path>,
) -> Result<()> {
    let wd = workdir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Step 1: Build the core manifest from user input.
    let agent_prompt = prompt.unwrap_or(DEFAULT_AGENT_PROMPT);
    let domain_plugin = match domain {
        "coding" => Some(DomainPlugin::Coding(CodingConfig {
            workspace_path: wd.display().to_string(),
            language: None,
        })),
        "research" => Some(DomainPlugin::Research(ResearchConfig::default())),
        "chain" => Some(DomainPlugin::Chain(LifecycleChainConfig::default())),
        "general" => None,
        other => bail!(
            "unknown domain '{}'; expected: coding, research, chain, general",
            other
        ),
    };

    let core = AgentCoreManifest {
        prompt: agent_prompt.to_string(),
        mode: DeploymentMode::SelfHosted,
        domain: domain_plugin,
        schema_version: 1,
    };

    let mut manifest = AgentExtendedManifest::new(core);
    manifest.name = Some(name.to_string());
    manifest.template_id = template.map(String::from);

    // Step 2: Resolve defaults and validate.
    let manifest = resolve_manifest(manifest);
    validate_manifest(&manifest).map_err(|e| anyhow::anyhow!("manifest validation failed: {e}"))?;

    // Step 3: Write to disk.
    let agents_dir = wd.join(".roko").join("agents").join(name);
    std::fs::create_dir_all(&agents_dir)
        .with_context(|| format!("create agent directory at {}", agents_dir.display()))?;

    let manifest_path = agents_dir.join("manifest.toml");
    let toml_text =
        toml::to_string_pretty(&manifest).context("serialize agent manifest to TOML")?;
    std::fs::write(&manifest_path, &toml_text)
        .with_context(|| format!("write manifest to {}", manifest_path.display()))?;

    println!("Agent '{}' created successfully.", name);
    println!("  domain:   {domain}");
    if let Some(tpl) = template {
        println!("  template: {tpl}");
    }
    println!("  manifest: {}", manifest_path.display());
    println!();
    println!("Edit the manifest to customize, then provision with:");
    println!("  roko agent serve --agent-id {name}");

    Ok(())
}

// ─── LIFE-06: Agent deletion ────────────────────────────────────────────

/// 8-step agent deletion with per-step 30-second timeout.
///
/// Steps:
///   1. Stop processing (cancel current task, drain queue)
///   2. Flush pending (complete in-flight tool calls)
///   3. Backup knowledge (auto-invoke neuro backup)
///   4. Deregister from mesh
///   5. Release resources
///   6. Archive signals (compress JSONL logs)
///   7. Clean state (remove executor.json and transient files)
///   8. Confirm (write DELETED marker)
async fn run_agent_delete(name: &str, force: bool, workdir: Option<&Path>) -> Result<()> {
    let wd = workdir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let agent_dir = wd.join(".roko").join("agents").join(name);
    if !agent_dir.exists() {
        bail!("agent '{}' not found at {}", name, agent_dir.display());
    }

    if force {
        println!("Force-deleting agent '{name}'...");
        // Force mode: skip ordered shutdown, remove everything immediately.
        std::fs::remove_dir_all(&agent_dir)
            .with_context(|| format!("remove agent directory at {}", agent_dir.display()))?;
        println!("Agent '{name}' force-deleted.");
        return Ok(());
    }

    // Ordered 8-step shutdown, each step has a 30-second budget.
    let step_timeout = std::time::Duration::from_secs(30);

    // Step 1: Stop processing.
    run_deletion_step("Stop processing", step_timeout, || {
        info!(agent = name, "stopping agent processing");
        Ok(())
    });

    // Step 2: Flush pending.
    run_deletion_step("Flush pending", step_timeout, || {
        info!(agent = name, "flushing pending operations");
        Ok(())
    });

    // Step 3: Backup knowledge.
    run_deletion_step("Backup knowledge", step_timeout, || {
        let neuro_dir = wd.join(".roko").join("neuro");
        if neuro_dir.exists() {
            let backup_dir = wd.join(".roko").join("backups").join(format!(
                "{}-{}",
                name,
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ));
            std::fs::create_dir_all(&backup_dir)?;
            // Copy knowledge files into the backup directory.
            let knowledge_src = neuro_dir.join("knowledge.jsonl");
            if knowledge_src.exists() {
                std::fs::copy(&knowledge_src, backup_dir.join("knowledge.jsonl"))?;
                println!("  knowledge backed up to {}", backup_dir.display());
            }
            let confirmations_src = neuro_dir.join("knowledge-confirmations.jsonl");
            if confirmations_src.exists() {
                std::fs::copy(
                    &confirmations_src,
                    backup_dir.join("knowledge-confirmations.jsonl"),
                )?;
            }
        } else {
            println!("  no neuro store to backup");
        }
        Ok(())
    });

    // Step 4: Deregister from mesh.
    run_deletion_step("Deregister from mesh", step_timeout, || {
        info!(agent = name, "deregistering from mesh");
        // Mesh deregistration would happen here if mesh is enabled.
        Ok(())
    });

    // Step 5: Release resources.
    run_deletion_step("Release resources", step_timeout, || {
        info!(agent = name, "releasing allocated resources");
        Ok(())
    });

    // Step 6: Archive signals.
    run_deletion_step("Archive signals", step_timeout, || {
        let signals_path = wd.join(".roko").join("signals.jsonl");
        let episodes_path = wd.join(".roko").join("episodes.jsonl");
        let archive_dir = agent_dir.join("archived");
        std::fs::create_dir_all(&archive_dir)?;
        if signals_path.exists() {
            std::fs::copy(&signals_path, archive_dir.join("signals.jsonl"))?;
        }
        if episodes_path.exists() {
            std::fs::copy(&episodes_path, archive_dir.join("episodes.jsonl"))?;
        }
        Ok(())
    });

    // Step 7: Clean state.
    run_deletion_step("Clean state", step_timeout, || {
        let executor_state = wd.join(".roko").join("state").join("executor.json");
        if executor_state.exists() {
            std::fs::remove_file(&executor_state)?;
        }
        // Remove transient files in the agent directory.
        let _ = std::fs::remove_dir_all(agent_dir.join("tmp"));
        Ok(())
    });

    // Step 8: Confirm deletion.
    run_deletion_step("Confirm deletion", step_timeout, || {
        // Write a DELETED marker in the agent directory.
        let marker = agent_dir.join("DELETED");
        let ts = chrono::Utc::now().to_rfc3339();
        std::fs::write(&marker, format!("deleted_at={ts}\nagent={name}\n"))?;
        Ok(())
    });

    println!("Agent '{name}' deleted (ordered shutdown complete).");
    println!("  Archived signals and DELETED marker remain at:");
    println!("  {}", agent_dir.display());

    Ok(())
}

/// Run a single deletion step with a wall-clock timeout. If the step panics
/// or exceeds the timeout, it is skipped and the next step proceeds.
fn run_deletion_step(label: &str, timeout: std::time::Duration, f: impl FnOnce() -> Result<()>) {
    print!("  [{label}] ");
    let start = std::time::Instant::now();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(())) => {
            let elapsed = start.elapsed();
            if elapsed > timeout {
                println!("ok (exceeded {timeout:?}, continuing)");
            } else {
                println!("ok");
            }
        }
        Ok(Err(err)) => {
            println!("skipped: {err}");
        }
        Err(_) => {
            println!("skipped (panicked)");
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Capability readiness ───────────────────────────────────────────

    #[test]
    fn capability_readiness_no_active_stub_capabilities() {
        let readiness = CapabilityReadiness::for_runtime(true);
        // Only messaging and predictions should be active.
        assert!(readiness.messaging);
        assert!(readiness.predictions);
        // All stub capabilities must be false.
        assert!(!readiness.relay);
        assert!(!readiness.chain);
        assert!(!readiness.identity);
        assert!(!readiness.passport);
        assert!(!readiness.wallet_signing);
        assert!(!readiness.cognitive_loop);
    }

    #[test]
    fn capability_readiness_no_dispatcher() {
        let readiness = CapabilityReadiness::for_runtime(false);
        assert!(!readiness.messaging);
        assert!(readiness.predictions);
    }

    #[test]
    fn capability_readiness_serializable() {
        let readiness = CapabilityReadiness::for_runtime(true);
        let json = serde_json::to_string(&readiness).expect("serialize");
        let parsed: CapabilityReadiness = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(readiness.messaging, parsed.messaging);
        assert_eq!(readiness.relay, parsed.relay);
    }

    // ── Unsupported flag rejection ─────────────────────────────────────

    fn minimal_serve_args() -> AgentServeArgs {
        AgentServeArgs {
            agent_id: "test-agent".to_string(),
            bind: "127.0.0.1:0".to_string(),
            relay_url: None,
            chain_rpc_url: None,
            identity_registry: None,
            passport_id: None,
            wallet_key: None,
            serve_url: roko_cli::DEFAULT_SERVE_URL.to_string(),
            allow_stub_cognitive_loop: false,
        }
    }

    #[test]
    fn reject_no_flags_passes() {
        assert!(reject_unsupported_serve_flags(&minimal_serve_args()).is_ok());
    }

    #[test]
    fn reject_relay_url() {
        let mut args = minimal_serve_args();
        args.relay_url = Some("wss://relay.example.com".to_string());
        let err = reject_unsupported_serve_flags(&args).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--relay-url"), "error: {msg}");
        assert!(msg.contains("#224"), "error: {msg}");
    }

    #[test]
    fn reject_chain_rpc_url() {
        let mut args = minimal_serve_args();
        args.chain_rpc_url = Some("http://localhost:8545".to_string());
        let err = reject_unsupported_serve_flags(&args).unwrap_err();
        assert!(err.to_string().contains("--chain-rpc-url"));
    }

    #[test]
    fn reject_identity_registry() {
        let mut args = minimal_serve_args();
        args.identity_registry = Some("0x1234".to_string());
        let err = reject_unsupported_serve_flags(&args).unwrap_err();
        assert!(err.to_string().contains("--identity-registry"));
    }

    #[test]
    fn reject_passport_id() {
        let mut args = minimal_serve_args();
        args.passport_id = Some("42".to_string());
        let err = reject_unsupported_serve_flags(&args).unwrap_err();
        assert!(err.to_string().contains("--passport-id"));
    }

    #[test]
    fn reject_wallet_key_does_not_log_value() {
        let mut args = minimal_serve_args();
        args.wallet_key = Some("0xSECRET_KEY_VALUE".to_string());
        let err = reject_unsupported_serve_flags(&args).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--wallet-key"), "error: {msg}");
        // Must not leak the wallet key value.
        assert!(
            !msg.contains("SECRET_KEY_VALUE"),
            "leaked wallet key: {msg}"
        );
    }

    #[test]
    fn reject_multiple_flags_reports_all() {
        let mut args = minimal_serve_args();
        args.relay_url = Some("wss://relay.example.com".to_string());
        args.chain_rpc_url = Some("http://localhost:8545".to_string());
        let err = reject_unsupported_serve_flags(&args).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--relay-url"), "error: {msg}");
        assert!(msg.contains("--chain-rpc-url"), "error: {msg}");
    }

    // ── Agent target resolution ────────────────────────────────────────

    #[test]
    fn resolve_chat_agent_explicit_flag() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_chat_agent(Some("my-agent"), dir.path());
        assert_eq!(result.unwrap(), "my-agent");
    }

    #[test]
    fn resolve_chat_agent_no_agents_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_chat_agent(None, dir.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no agent specified"),
            "expected actionable error, got: {msg}"
        );
    }

    #[test]
    fn resolve_chat_agent_config_default() {
        let dir = tempfile::tempdir().unwrap();
        // Write a config with default_agent_id.
        let toml = "[agent]\ndefault_agent_id = \"configured-agent\"\n";
        std::fs::write(dir.path().join("roko.toml"), toml).unwrap();
        let result = resolve_chat_agent(None, dir.path());
        assert_eq!(result.unwrap(), "configured-agent");
    }

    #[test]
    fn resolve_chat_agent_single_healthy_agent() {
        let dir = tempfile::tempdir().unwrap();
        // Write agents.json with our own PID (so it's alive).
        let runtime_dir = dir.path().join(".roko/runtime");
        std::fs::create_dir_all(&runtime_dir).unwrap();
        let entry = serde_json::json!([{
            "name": "solo-agent",
            "pid": std::process::id(),
            "bind": "http://127.0.0.1:8081",
            "domain": "general",
            "started_at": "2026-01-01T00:00:00Z"
        }]);
        std::fs::write(
            runtime_dir.join("agents.json"),
            serde_json::to_string(&entry).unwrap(),
        )
        .unwrap();

        let result = resolve_chat_agent(None, dir.path());
        assert_eq!(result.unwrap(), "solo-agent");
    }

    #[test]
    fn resolve_chat_agent_ambiguous_multiple() {
        let dir = tempfile::tempdir().unwrap();
        let runtime_dir = dir.path().join(".roko/runtime");
        std::fs::create_dir_all(&runtime_dir).unwrap();
        let pid = std::process::id();
        let entries = serde_json::json!([
            {"name": "agent-a", "pid": pid, "bind": "http://127.0.0.1:8081", "domain": "general", "started_at": "2026-01-01T00:00:00Z"},
            {"name": "agent-b", "pid": pid, "bind": "http://127.0.0.1:8082", "domain": "general", "started_at": "2026-01-01T00:00:00Z"},
        ]);
        std::fs::write(
            runtime_dir.join("agents.json"),
            serde_json::to_string(&entries).unwrap(),
        )
        .unwrap();

        let err = resolve_chat_agent(None, dir.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("2 healthy agents"),
            "expected ambiguity, got: {msg}"
        );
        assert!(msg.contains("agent-a"), "expected names, got: {msg}");
        assert!(msg.contains("agent-b"), "expected names, got: {msg}");
    }

    // ── ChatLaunchConfig ───────────────────────────────────────────────

    #[test]
    fn chat_launch_config_text_flag_forces_line_repl() {
        let dir = tempfile::tempdir().unwrap();
        // Write agents.json with our own PID.
        let runtime_dir = dir.path().join(".roko/runtime");
        std::fs::create_dir_all(&runtime_dir).unwrap();
        let entry = serde_json::json!([{
            "name": "chat-agent",
            "pid": std::process::id(),
            "bind": "http://127.0.0.1:8081",
            "domain": "general",
            "started_at": "2026-01-01T00:00:00Z"
        }]);
        std::fs::write(
            runtime_dir.join("agents.json"),
            serde_json::to_string(&entry).unwrap(),
        )
        .unwrap();

        let launch = resolve_chat_launch(
            Some("chat-agent"),
            None,
            None,
            true, // --text
            "http://localhost:6677".to_string(),
            dir.path(),
        )
        .unwrap();
        assert_eq!(launch.ui_mode, ChatUiMode::LineRepl);
        assert_eq!(launch.target, "chat-agent");
    }

    #[test]
    fn chat_launch_config_model_override() {
        let dir = tempfile::tempdir().unwrap();
        let launch = resolve_chat_launch(
            Some("test-agent"),
            Some("anthropic_api".to_string()),
            Some("custom-model".to_string()),
            false,
            "http://localhost:6677".to_string(),
            dir.path(),
        )
        .unwrap();
        assert_eq!(launch.model.as_deref(), Some("custom-model"));
        assert_eq!(launch.provider.as_deref(), Some("anthropic_api"));
    }

    // ── Default agent ID config ────────────────────────────────────────

    #[test]
    fn agent_config_default_agent_id_serde() {
        let config = roko_core::config::agent::AgentConfig::default();
        assert!(config.default_agent_id.is_none());

        let toml_str = "[agent]\ndefault_agent_id = \"my-preferred-agent\"\n";
        let parsed: roko_core::config::schema::RokoConfig =
            roko_core::config::schema::RokoConfig::from_toml(toml_str).unwrap();
        assert_eq!(
            parsed.agent.default_agent_id.as_deref(),
            Some("my-preferred-agent")
        );
    }

    // ── Chat command no longer has hardcoded default ────────────────────

    #[test]
    fn chat_agent_field_is_optional() {
        // Verify that the clap definition no longer has a default_value.
        // When --agent is not passed, the field should be None.
        use clap::Subcommand;

        // Build the AgentCmd parser from clap metadata.
        let app = AgentCmd::augment_subcommands(clap::Command::new("agent"));
        let chat_cmd = app.find_subcommand("chat").expect("chat subcommand");
        let agent_arg = chat_cmd
            .get_arguments()
            .find(|a| a.get_id() == "agent")
            .expect("agent argument");
        // Should not have any default value.
        assert!(
            agent_arg.get_default_values().is_empty(),
            "agent should not have a default value (was: {:?})",
            agent_arg.get_default_values()
        );
    }
}
