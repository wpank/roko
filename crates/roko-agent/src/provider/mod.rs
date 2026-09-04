//! Provider routing and agent construction.
//!
//! This module is the migration layer between the older "pick a concrete
//! [`Agent`](crate::Agent) and construct it directly" style and the newer
//! provider-aware factory flow.
//!
//! ## Relationship to `Agent`
//!
//! The concrete backends in this crate, such as `ClaudeCliAgent`,
//! `CodexAgent`, `CursorAgent`, and `OllamaAgent`, still implement
//! [`Agent`](crate::Agent). The provider layer does not replace that trait.
//! Instead, it chooses and configures one of those implementations from
//! `RokoConfig` plus a model key, then returns `Box<dyn Agent>` to the
//! existing runtime.
//!
//! ## When to use `create_agent_for_model`
//!
//! Use [`create_agent_for_model`] when you want config-driven resolution:
//! the caller has a `RokoConfig`, a model key, and wants Roko to resolve the
//! provider, model slug, timeout, and other provider settings in one place.
//! This is the right entry point for CLI/runtime code that should follow the
//! configured routing rules.
//!
//! Construct a concrete agent directly when you already know the exact
//! backend you want, such as in a test, a focused integration, or a
//! backend-specific utility that intentionally bypasses model resolution.
//!
//! ## Adding a new provider
//!
//! There are two supported paths:
//!
//! - If the provider needs new runtime behavior, implement
//!   [`ProviderAdapter`] for it and register the adapter in
//!   [`adapter_for_kind`].
//! - If the provider is already covered by an existing adapter, add a
//!   matching `providers.*` entry and point one or more `models.*` entries at
//!   it in `roko.toml`.
//!
//! In both cases, the goal is the same: keep provider-specific wiring out of
//! the call sites and centralize it in this module.

use crate::SafetyLayer;
use crate::dispatcher::{HandlerResolver, ToolDispatcher};
use crate::gemini::GeminiAdapter;
use crate::immune_boundary::{safe_provider_agent_identity, wrap_provider_agent};
use crate::mcp::McpRuntime;
use crate::mock::MockAgent;
use crate::process::ResourceLimits;
use crate::rate_limit::ProviderRateLimiter;
use crate::safety::contract::AgentContract;
use crate::{Agent, ExecAgent};
use indexmap::IndexMap;
use roko_core::agent::{ProviderKind, resolve_model};
#[cfg(test)]
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;
use roko_core::config::schema::RokoConfig;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::defaults::{DEFAULT_MAX_TOOL_ITERATIONS, DEFAULT_REQUEST_TIMEOUT_MS};
use roko_core::tool::{ToolDef, ToolRegistry};
use roko_core::{ModelInputMessage, Temperament};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub mod anthropic_api;
pub mod cerebras;
pub mod claude_cli;
pub mod codex_cli;
pub mod cursor_acp;
pub mod cursor_cli;
pub mod error_classify;
pub mod gemini_cli;
pub mod hermes;
pub mod openai_compat;
pub mod openclaw;
pub mod openrouter_meta;
pub mod pre_flight;

pub use anthropic_api::AnthropicApiAdapter;
pub use cerebras::CerebrasAdapter;
pub use claude_cli::{ClaudeCliAdapter, CodexCliAdapter};
pub use cursor_acp::CursorAcpAdapter;
pub use cursor_cli::CursorCliAdapter;
pub use gemini_cli::GeminiCliAdapter;
pub use hermes::HermesProviderAdapter;
pub use openai_compat::OpenAiCompatAdapter;
pub use openclaw::OpenClawProviderAdapter;
pub use openrouter_meta::fetch_model_metadata;
pub use pre_flight::{ProviderReadinessIssue, check_provider_readiness, report_readiness_issues};

use crate::perplexity::{PerplexityAdapter, SearchOptions};

static ANTHROPIC_API_ADAPTER: AnthropicApiAdapter = AnthropicApiAdapter;
static CEREBRAS_ADAPTER: CerebrasAdapter = CerebrasAdapter;
static CLAUDE_CLI_ADAPTER: ClaudeCliAdapter = ClaudeCliAdapter;
static CODEX_CLI_ADAPTER: CodexCliAdapter = CodexCliAdapter;
static CURSOR_ACP_ADAPTER: CursorAcpAdapter = CursorAcpAdapter;
static CURSOR_CLI_ADAPTER: CursorCliAdapter = CursorCliAdapter;
static GEMINI_CLI_ADAPTER: GeminiCliAdapter = GeminiCliAdapter;
static HERMES_ADAPTER: HermesProviderAdapter = HermesProviderAdapter;
static OPENAI_COMPAT_ADAPTER: OpenAiCompatAdapter = OpenAiCompatAdapter;
static OPENCLAW_ADAPTER: OpenClawProviderAdapter = OpenClawProviderAdapter;
static PERPLEXITY_ADAPTER: PerplexityAdapter = PerplexityAdapter;
static GEMINI_ADAPTER: GeminiAdapter = GeminiAdapter;
const DEFAULT_PROVIDER_MAX_CONCURRENT: usize = roko_core::defaults::DEFAULT_PROVIDER_MAX_CONCURRENT;
pub const PERPLEXITY_SEARCH_OPTIONS_ARG_PREFIX: &str = "pplx.search_options=";

/// Process-wide shared HTTP client with pooled connections.
///
/// A single `reqwest::Client` keeps TCP and TLS connections warm across all
/// provider adapters, avoiding redundant handshakes when new backends are
/// constructed for the same process.
static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    match reqwest::Client::builder()
        // Keep at most 10 idle connections per host so a burst of parallel
        // agent requests does not accumulate unbounded sockets.
        .pool_max_idle_per_host(10)
        // Evict idle connections after 90 s to avoid holding sockets open
        // against provider endpoints that enforce shorter server-side timeouts.
        .pool_idle_timeout(Duration::from_secs(90))
        // Send TCP keep-alive probes every 30 s so NAT/firewall state is
        // refreshed between long streaming responses.
        .tcp_keepalive(Duration::from_secs(30))
        // Fail fast on unreachable hosts instead of blocking an agent task
        // indefinitely while waiting for a three-way handshake.
        .connect_timeout(Duration::from_secs(10))
        // Identify outbound requests so provider logs and rate-limit headers
        // can be correlated back to the roko-agent version in use.
        .user_agent(concat!("roko-agent/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            tracing::warn!(
                "failed to build shared HTTP client with pool settings: {e}; using default client"
            );
            reqwest::Client::new()
        }
    }
});

/// Return the process-wide shared HTTP client.
///
/// All production HTTP callers should use this client so requests can reuse
/// pooled connections instead of paying a fresh TLS handshake per backend.
///
/// Pool settings (configured in [`SHARED_HTTP_CLIENT`]):
///
/// | Setting | Value | Rationale |
/// |---|---|---|
/// | `pool_max_idle_per_host` | 10 | Caps idle sockets per provider endpoint |
/// | `pool_idle_timeout` | 90 s | Evicts before typical server-side timeouts |
/// | `tcp_keepalive` | 30 s | Keeps NAT state alive during long streams |
/// | `connect_timeout` | 10 s | Fails fast on unreachable hosts |
/// | `user_agent` | `roko-agent/<version>` | Correlates provider logs to this build |
#[must_use]
pub fn shared_http_client() -> reqwest::Client {
    SHARED_HTTP_CLIENT.clone()
}

thread_local! {
    static ACTIVE_SAFETY_LAYER: RefCell<Option<SafetyLayer>> = const { RefCell::new(None) };
    static ACTIVE_TEMPERAMENT: RefCell<Option<Temperament>> = const { RefCell::new(None) };
}

/// Return the static adapter for a provider kind.
#[must_use]
pub fn adapter_for_kind(kind: ProviderKind) -> &'static dyn ProviderAdapter {
    match kind {
        ProviderKind::OpenAiCompat => &OPENAI_COMPAT_ADAPTER,
        ProviderKind::ClaudeCli => &CLAUDE_CLI_ADAPTER,
        ProviderKind::AnthropicApi => &ANTHROPIC_API_ADAPTER,
        ProviderKind::CursorAcp => &CURSOR_ACP_ADAPTER,
        ProviderKind::CursorCli => &CURSOR_CLI_ADAPTER,
        ProviderKind::PerplexityApi => &PERPLEXITY_ADAPTER,
        ProviderKind::GeminiApi => &GEMINI_ADAPTER,
        ProviderKind::GeminiCli => &GEMINI_CLI_ADAPTER,
        ProviderKind::CerebrasApi => &CEREBRAS_ADAPTER,
        ProviderKind::Hermes => &HERMES_ADAPTER,
        ProviderKind::OpenClaw => &OPENCLAW_ADAPTER,
        ProviderKind::CodexCli => &CODEX_CLI_ADAPTER,
    }
}

/// Resolve a model key and create a configured agent for it.
///
/// This is the unified entrypoint for provider-aware agent construction.
#[must_use]
pub fn create_agent_for_model(
    config: &RokoConfig,
    model_key: &str,
    options: AgentOptions,
) -> Result<Box<dyn Agent>, AgentCreationError> {
    let mut options = options;
    // Preserve the requested value only until the outer boundary can record
    // whether it was valid. Provider adapters and their inner agents receive
    // only the safe identity, so even an infallible construction path cannot
    // retain or later expose a rejected secret-shaped/control-bearing value.
    //
    // An empty name means "let the adapter choose" — skip the scrub so the
    // adapter's default naming (e.g. `gemini-compat:{slug}`) is preserved.
    let requested_agent_id = options.name.clone();
    if !requested_agent_id.is_empty() {
        options.name = safe_provider_agent_identity(&requested_agent_id).0;
    }
    roko_core::validate_model_input_messages(&options.input_messages)
        .map_err(AgentCreationError::InvalidImageInput)?;
    let has_images = crate::multimodal::contains_images(&options.input_messages);
    let mut mock_agent = mock_agent_from_env(&options)?;
    if !has_images && let Some(mock_agent) = mock_agent.take() {
        return Ok(wrap_provider_agent(
            mock_agent,
            &requested_agent_id,
            options.effective_immune_root(),
        ));
    }
    let safety_layer = options
        .safety_layer
        .clone()
        .unwrap_or_else(|| safety_layer_for_options(config, &options));
    let effective_temperament = options
        .temperament
        .unwrap_or_else(|| config.agent.temperament_for_role(&safety_layer.role));
    // Populate canonical fields so adapters can read them directly.
    options.safety_layer = Some(safety_layer.clone());
    options.temperament = Some(effective_temperament);
    let resolved = resolve_model(config, model_key);
    let profile = resolved
        .profile
        .or_else(|| config.effective_models().get(model_key).cloned());
    let provider_config = profile.as_ref().and_then(|profile| {
        resolved
            .provider_config
            .clone()
            .or_else(|| config.effective_providers().get(&profile.provider).cloned())
    });
    let legacy_command = options
        .command
        .as_deref()
        .or(config.agent.command.as_deref());

    let (provider_config, profile) = match (provider_config, profile) {
        (Some(pc), Some(mp)) => (pc, mp),
        (None, Some(mp)) => {
            tracing::warn!(
                model_key = model_key,
                provider = %mp.provider,
                "configured model references missing provider"
            );
            return Err(AgentCreationError::MissingConfig(format!(
                "model `{model_key}` references provider `{}` but that provider is not configured",
                mp.provider
            )));
        }
        _ if legacy_command.is_some_and(is_known_protocol_command) => {
            let command = legacy_command.unwrap_or("unknown");
            tracing::warn!(
                model_key = model_key,
                command = command,
                "known protocol command requires explicit provider/model config"
            );
            return Err(AgentCreationError::MissingConfig(format!(
                "explicit [providers] and [models] entries are required for protocol command `{command}` and model `{model_key}`"
            )));
        }
        _ => {
            if has_images {
                return Err(AgentCreationError::ImageInputUnsupported {
                    model: model_key.to_string(),
                    provider: resolved.provider_kind,
                });
            }
            tracing::warn!(
                model_key = model_key,
                command = %legacy_command.unwrap_or("unknown"),
                "no provider found — falling back to ExecAgent (no tool support)"
            );

            let mut agent = ExecAgent::new(
                legacy_command.unwrap_or("cat"),
                options.extra_args.clone(),
                safety_layer,
            )
            .with_timeout_ms(options.effective_timeout_ms(None));
            if !options.name.is_empty() {
                agent = agent.with_name(options.name.clone());
            }
            if !options.env.is_empty() {
                agent = agent.with_env(options.env.clone());
            }
            return Ok(wrap_provider_agent(
                Box::new(agent) as Box<dyn Agent>,
                &requested_agent_id,
                options.effective_immune_root(),
            ));
        }
    };

    tracing::info!(
        model_key = model_key,
        slug = %resolved.slug,
        provider = %provider_config.kind,
        base_url = ?provider_config.base_url,
        "creating agent via provider adapter"
    );

    if has_images && (!profile.supports_vision || !provider_config.kind.supports_inline_images()) {
        return Err(AgentCreationError::ImageInputUnsupported {
            model: model_key.to_string(),
            provider: provider_config.kind,
        });
    }

    if let Some(mock_agent) = mock_agent.take() {
        return Ok(wrap_provider_agent(
            mock_agent,
            &requested_agent_id,
            options.effective_immune_root(),
        ));
    }

    let adapter = adapter_for_kind(provider_config.kind);

    if options
        .pre_discovered_local_tools
        .as_ref()
        .is_some_and(|runtime| !runtime.tools().is_empty())
        && !adapter.supports_local_tool_runtime()
    {
        return Err(AgentCreationError::LocalToolsUnsupported(
            provider_config.kind,
        ));
    }
    if options
        .local_tool_mcp_servers
        .as_ref()
        .is_some_and(|servers| !servers.is_empty())
        && !adapter.supports_per_call_local_mcp(&provider_config)
    {
        return Err(AgentCreationError::LocalToolsUnsupported(
            provider_config.kind,
        ));
    }

    if options.provider_semaphores.is_none() {
        let providers = config.effective_providers();
        options.provider_semaphores = Some(Arc::new(ProviderSemaphores::new(&providers)));
    }

    // Forward Gemini safety settings from config so they reach the native API body.
    if provider_config.kind == ProviderKind::GeminiApi && options.gemini_safety_settings.is_empty()
    {
        options.gemini_safety_settings = config.gemini.safety_settings.clone();
    }
    let agent = with_temperament(Some(effective_temperament), || {
        with_safety_layer(Some(safety_layer), || {
            adapter.create_agent(&provider_config, &profile, &options)
        })
    })?;
    // When no explicit agent ID was requested, use the adapter-chosen name
    // (e.g. `gemini-compat:{slug}`) so the immune boundary inherits a valid
    // identity instead of rejecting an empty string.
    let effective_agent_id = if requested_agent_id.is_empty() {
        agent.name().to_string()
    } else {
        requested_agent_id
    };
    Ok(wrap_provider_agent(
        agent,
        &effective_agent_id,
        options.effective_immune_root(),
    ))
}

fn safety_layer_for_options(config: &RokoConfig, options: &AgentOptions) -> SafetyLayer {
    let mut safety_layer = options
        .safety_layer
        .clone()
        .or_else(current_safety_layer)
        .unwrap_or_else(|| SafetyLayer::from_config(config));
    if let Some(contract) = options.agent_contract.clone() {
        safety_layer = safety_layer.with_contract(contract);
    }
    safety_layer
}

fn mock_agent_from_env(
    options: &AgentOptions,
) -> Result<Option<Box<dyn Agent>>, AgentCreationError> {
    let Ok(dispatcher) = env::var("ROKO_DISPATCHER") else {
        return Ok(None);
    };
    let fixture = match dispatcher.strip_prefix("mock-") {
        Some(fixture) if !fixture.trim().is_empty() => fixture,
        _ => return Ok(None),
    };

    let mut agent = MockAgent::scripted_from_fixture(fixture)
        .map_err(|err| AgentCreationError::FixtureLoad(err.to_string()))?;
    if let Some(working_dir) = options.working_dir.clone() {
        agent = agent.with_working_dir(working_dir);
    }
    if let Ok(state_path) = env::var("ROKO_MOCK_STATE_PATH") {
        let state_path = state_path.trim();
        if !state_path.is_empty() {
            agent = agent.with_state_path(state_path);
        }
    }
    if !options.name.is_empty() {
        agent = agent.with_name(options.name.clone());
    }
    Ok(Some(Box::new(agent)))
}

/// Run `f` with an optional safety layer attached to provider-backed agent construction.
///
/// This is intentionally scoped to synchronous construction so the thread-local
/// state cannot leak across async suspension points.
#[must_use]
pub fn with_safety_layer<R>(layer: Option<SafetyLayer>, f: impl FnOnce() -> R) -> R {
    let scope = set_active_safety_layer(layer);
    let result = f();
    drop(scope);
    result
}

/// Run `f` with the current safety layer, or default to [`SafetyLayer::with_defaults()`].
///
/// This is the common case for direct agent construction paths that want the same
/// safety scope behavior as orchestrated runs without having to duplicate the fallback.
#[must_use]
pub fn with_scoped_safety_layer<R>(f: impl FnOnce() -> R) -> R {
    let layer = current_safety_layer().or_else(|| Some(SafetyLayer::with_defaults()));
    with_safety_layer(layer, f)
}

/// Build a `ToolDispatcher` and attach the active safety layer if one is present.
#[must_use]
pub fn build_tool_dispatcher(
    registry: Arc<dyn ToolRegistry>,
    resolver: Arc<dyn HandlerResolver>,
) -> Arc<ToolDispatcher> {
    build_tool_dispatcher_with_audit(registry, resolver, None)
}

/// Build a `ToolDispatcher` with an optional persistent JSONL file audit adapter.
#[must_use]
pub fn build_tool_dispatcher_with_audit(
    registry: Arc<dyn ToolRegistry>,
    resolver: Arc<dyn HandlerResolver>,
    file_audit: Option<Arc<roko_fs::tool_audit::ScrubAuditAdapter>>,
) -> Arc<ToolDispatcher> {
    let layer = current_safety_layer().unwrap_or_else(SafetyLayer::with_defaults);
    let mut dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);
    if let Some(audit) = file_audit {
        dispatcher = dispatcher.with_file_audit(audit);
    }
    Arc::new(dispatcher)
}

/// Return the safety layer currently scoped to provider-backed construction, if any.
#[must_use]
pub fn current_safety_layer() -> Option<SafetyLayer> {
    ACTIVE_SAFETY_LAYER.with(|slot| slot.borrow().clone())
}

#[must_use]
pub(crate) fn with_temperament<R>(temperament: Option<Temperament>, f: impl FnOnce() -> R) -> R {
    let scope = set_active_temperament(temperament);
    let result = f();
    drop(scope);
    result
}

#[must_use]
pub(crate) fn current_temperament() -> Option<Temperament> {
    ACTIVE_TEMPERAMENT.with(|slot| *slot.borrow())
}

struct SafetyLayerScope {
    previous: Option<SafetyLayer>,
}

impl Drop for SafetyLayerScope {
    fn drop(&mut self) {
        ACTIVE_SAFETY_LAYER.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

fn set_active_safety_layer(layer: Option<SafetyLayer>) -> SafetyLayerScope {
    let previous = ACTIVE_SAFETY_LAYER.with(|slot| slot.replace(layer));
    SafetyLayerScope { previous }
}

struct TemperamentScope {
    previous: Option<Temperament>,
}

impl Drop for TemperamentScope {
    fn drop(&mut self) {
        ACTIVE_TEMPERAMENT.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

fn set_active_temperament(temperament: Option<Temperament>) -> TemperamentScope {
    let previous = ACTIVE_TEMPERAMENT.with(|slot| slot.replace(temperament));
    TemperamentScope { previous }
}

#[must_use]
pub(crate) fn tool_limit_for_temperament(limit: usize) -> usize {
    let adjusted = match current_temperament().unwrap_or_default() {
        Temperament::Conservative => limit.saturating_mul(3) / 4,
        Temperament::Balanced | Temperament::Exploratory => limit,
        Temperament::Aggressive => limit / 2,
    };
    adjusted.max(1)
}

/// Apply the temperament adjustment to a base iteration cap.
///
/// - Balanced: no change
/// - Conservative: +10
/// - Aggressive: -15 (floor at 10)
/// - Exploratory: +20
fn apply_temperament_to_iteration_cap(base: usize) -> usize {
    match current_temperament().unwrap_or_default() {
        Temperament::Conservative => base.saturating_add(10),
        Temperament::Balanced => base,
        Temperament::Aggressive => base.saturating_sub(15).max(10),
        Temperament::Exploratory => base.saturating_add(20),
    }
}

#[must_use]
pub(crate) fn tool_loop_max_iterations() -> usize {
    tool_loop_max_iterations_for_profile(None)
}

/// Per-model iteration cap with temperament adjustment.
///
/// When a `ModelProfile` is provided and has `max_tool_iterations` set,
/// that value is used as the base before the temperament adjustment.
/// Otherwise falls back to `DEFAULT_MAX_TOOL_ITERATIONS`.
#[must_use]
pub(crate) fn tool_loop_max_iterations_for_profile(profile: Option<&ModelProfile>) -> usize {
    let base = profile
        .and_then(|p| p.max_tool_iterations)
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_MAX_TOOL_ITERATIONS);
    apply_temperament_to_iteration_cap(base)
}

#[must_use]
pub fn is_known_protocol_command(command: &str) -> bool {
    provider_kind_for_known_protocol_command(command).is_some()
}

#[must_use]
fn provider_kind_for_known_protocol_command(command: &str) -> Option<ProviderKind> {
    let executable = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command);

    match executable {
        "claude" => Some(ProviderKind::ClaudeCli),
        "codex" => Some(ProviderKind::OpenAiCompat),
        "cursor-agent" | "cursor_agent" => Some(ProviderKind::CursorAcp),
        _ => None,
    }
}

/// Find the first configured provider matching a given kind.
fn provider_for_kind(
    providers: &IndexMap<String, ProviderConfig>,
    kind: ProviderKind,
) -> Option<(String, &ProviderConfig)> {
    let exact_key = kind.label();
    if let Some(provider) = providers.get(exact_key)
        && provider.kind == kind
    {
        return Some((exact_key.to_string(), provider));
    }
    providers
        .iter()
        .find(|(_, p)| p.kind == kind)
        .map(|(k, p)| (k.clone(), p))
}

/// Shared semaphores that cap in-flight requests per provider.
#[derive(Debug)]
pub struct ProviderSemaphores {
    semaphores: HashMap<String, Arc<Semaphore>>,
    default_permits: usize,
}

impl ProviderSemaphores {
    #[must_use]
    pub fn new(configs: &IndexMap<String, ProviderConfig>) -> Self {
        let mut semaphores = HashMap::with_capacity(configs.len());
        for (id, config) in configs {
            let permits = config
                .max_concurrent
                .map_or(DEFAULT_PROVIDER_MAX_CONCURRENT, |n| n.max(1) as usize);
            semaphores.insert(id.clone(), Arc::new(Semaphore::new(permits)));
        }

        Self {
            semaphores,
            default_permits: DEFAULT_PROVIDER_MAX_CONCURRENT,
        }
    }

    pub async fn acquire(&self, provider_id: &str) -> Result<OwnedSemaphorePermit, ProviderError> {
        let semaphore = self
            .semaphores
            .get(provider_id)
            .cloned()
            .unwrap_or_else(|| Arc::new(Semaphore::new(self.default_permits)));

        semaphore.acquire_owned().await.map_err(|_| {
            ProviderError::Other(format!("provider semaphore for '{provider_id}' closed"))
        })
    }
}

/// Adapter for a protocol family. Creates Agent instances configured for a
/// specific provider and model.
pub trait ProviderAdapter: Send + Sync {
    /// Which protocol family this adapter handles.
    fn kind(&self) -> ProviderKind;

    /// Create an Agent instance from provider config and model profile.
    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError>;

    /// Classify an error response into a canonical error type.
    /// Used by health tracking to decide retry vs cooldown vs skip.
    fn classify_error(&self, status: u16, body: &Value) -> ProviderError;

    /// Whether this adapter's in-process loop can consume `LocalToolRuntime`.
    ///
    /// Opaque CLI/ACP harnesses must use an explicit transport bridge instead
    /// of receiving a runtime they cannot execute. Defaults to `false` so that
    /// newly added adapters fail closed rather than silently advertising
    /// definition-only plugin tools.
    fn supports_local_tool_runtime(&self) -> bool {
        false
    }

    /// Whether this adapter supports per-call local MCP server injection.
    fn supports_per_call_local_mcp(&self, _provider: &ProviderConfig) -> bool {
        false
    }
}

pub(crate) fn configured_resource_limits(
    provider: &ProviderConfig,
) -> Result<Option<ResourceLimits>, AgentCreationError> {
    let limits = ResourceLimits::from_provider_config(provider);
    if let Some(limits) = &limits {
        limits
            .validate_for_current_platform()
            .map_err(|error| AgentCreationError::ResourceLimitEnforcement(error.to_string()))?;
    }
    Ok(limits)
}

/// Model-visible local tool definitions paired with executable handlers.
///
/// This is the dependency-neutral handoff used by embedding surfaces such as
/// the CLI plugin loader. A provider must retain both halves for the lifetime
/// of its tool loop: definitions without handlers would advertise tools that
/// fail only after a model selects them.
#[derive(Clone)]
pub struct LocalToolRuntime {
    tools: Arc<Vec<ToolDef>>,
    resolver: Arc<dyn HandlerResolver>,
}

impl fmt::Debug for LocalToolRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalToolRuntime")
            .field("tool_count", &self.tools.len())
            .finish_non_exhaustive()
    }
}

impl LocalToolRuntime {
    /// Construct a runtime from definitions and the resolver that executes
    /// them. Provider construction validates handler parity before use.
    #[must_use]
    pub fn new(tools: Vec<ToolDef>, resolver: Arc<dyn HandlerResolver>) -> Self {
        Self {
            tools: Arc::new(tools),
            resolver,
        }
    }

    /// Canonical local definitions exposed to provider tool loops.
    #[must_use]
    pub fn tools(&self) -> &Arc<Vec<ToolDef>> {
        &self.tools
    }

    /// Definitions that do not resolve to an executable handler.
    #[must_use]
    pub fn missing_handlers(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter(|tool| self.resolver.resolve(&tool.name).is_none())
            .map(|tool| tool.name.clone())
            .collect()
    }

    /// Compose these local handlers over a fallback resolver. Local handlers
    /// take precedence, matching the local registry's definition override
    /// semantics.
    #[must_use]
    pub fn resolver(&self, fallback: Arc<dyn HandlerResolver>) -> Arc<dyn HandlerResolver> {
        let local = Arc::clone(&self.resolver);
        Arc::new(move |name: &str| local.resolve(name).or_else(|| fallback.resolve(name)))
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Default)]
pub struct AgentOptions {
    /// Resolved safety layer for this agent construction.
    ///
    /// When set, `create_agent_for_model` uses this instead of looking up a
    /// thread-local. Prefer setting this field over calling `with_safety_layer`.
    pub safety_layer: Option<SafetyLayer>,
    /// Resolved temperament for this agent construction.
    ///
    /// When set, `create_agent_for_model` uses this instead of looking up a
    /// thread-local. Prefer setting this field over calling `with_temperament`.
    pub temperament: Option<Temperament>,
    pub command: Option<String>,
    pub timeout_ms: Option<u64>,
    pub system_prompt: Option<String>,
    /// Validated provider-neutral structured messages for a multimodal turn.
    /// HTTP adapters translate these at their final wire boundary. Adapters
    /// without an inline-image protocol must reject a non-empty list.
    pub input_messages: Vec<ModelInputMessage>,
    pub cached_content: Option<String>,
    pub tools: Option<String>,
    /// Role contract resolved by the orchestration boundary.
    ///
    /// Provider construction scopes this contract into every in-process tool
    /// dispatcher. This is structured rather than encoded in `tools` so an
    /// empty allowlist remains a binding deny-all.
    pub agent_contract: Option<AgentContract>,
    pub mcp_config: Option<PathBuf>,
    /// Canonical workspace root for durable immune authority and evidence.
    ///
    /// Unlike `working_dir`, this path must not point at a disposable attempt
    /// worktree. Standalone callers may omit it and inherit `working_dir`.
    pub immune_root: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub provider_semaphores: Option<Arc<ProviderSemaphores>>,
    pub env: Vec<(String, String)>,
    pub extra_args: Vec<String>,
    pub effort: Option<String>,
    pub bare_mode: bool,
    /// Whether to skip Claude's permission system for this agent.
    ///
    /// Security audit inventory (PE_01):
    /// - NEEDS FIX (hardcoded `true`):
    ///   - crates/roko-agent/src/claude_cli_agent.rs:128
    ///   - crates/roko-cli/src/runner/types.rs:1337
    ///   - crates/roko-cli/src/runner/types.rs:1382
    ///   - crates/roko-cli/src/agent_exec.rs:145
    ///   - crates/roko-cli/src/serve_runtime.rs:547
    ///   - crates/roko-cli/src/commands/plan.rs:394
    ///   - crates/roko-serve/src/dispatch.rs:1824
    ///   - crates/roko-acp/src/runner.rs:1763
    /// - OK (hardcoded `false` / test-only):
    ///   - crates/roko-agent/src/provider/claude_cli.rs:241
    ///   - crates/roko-agent/src/claude_cli_agent.rs:1109
    ///   - crates/roko-cli/src/agent_serve.rs:429
    ///   - crates/roko-cli/src/commands/research.rs:66,264,377
    ///   - crates/roko-cli/src/dispatch_v2.rs:1037
    ///   - crates/roko-cli/src/runner/event_loop.rs (runner-v2; orchestrate.rs deleted in E12-T07)
    ///   - crates/roko-cli/src/run.rs:2054,2096
    ///   - crates/roko-cli/tests/smoke.rs:260
    ///   - crates/roko-dreams/src/runner.rs:169
    /// - DIVERGENCE:
    ///   - crates/roko-cli/src/run.rs:2784 (unknown roles default `true`; includes `network`)
    ///   (orchestrate.rs deleted in E12-T07; typed AgentRole now in runner-v2)
    /// - PROPAGATION:
    ///   - crates/roko-agent/src/provider/claude_cli.rs:54
    ///   - crates/roko-agent/src/claude_cli_agent.rs:328
    ///   - crates/roko-cli/src/runner/agent_stream.rs:66,138
    ///   - crates/roko-cli/src/runner/event_loop.rs:2043
    ///   - crates/roko-cli/src/agent_spawn.rs:59
    ///   - crates/roko-cli/src/dispatch_v2.rs:320
    ///   - crates/roko-cli/src/dispatch_v2.rs:357
    ///   - crates/roko-cli/src/dispatch_v2.rs:728
    ///   - crates/roko-cli/src/runner/event_loop.rs (runner-v2; orchestrate.rs deleted in E12-T07)
    ///   - crates/roko-cli/src/run.rs:1923,2003
    /// Default MUST be `false`. PE_02 will flip all NEEDS FIX sites.
    pub dangerously_skip_permissions: bool,
    pub name: String,
    /// Pre-supplied tool definitions that do not require a dynamic execution
    /// client. MCP-sourced definitions are rejected unless
    /// `pre_discovered_mcp_runtime` is also supplied.
    pub pre_discovered_mcp_tools: Option<Arc<Vec<ToolDef>>>,
    /// Pre-discovered MCP definitions plus their initialized execution
    /// clients. HTTP-provider tool loops use this instead of advertising
    /// definition-only MCP tools.
    pub pre_discovered_mcp_runtime: Option<Arc<McpRuntime>>,
    /// Non-MCP local tools, such as declarative plugin commands, paired with
    /// the handlers that execute them. Providers reject the entire runtime if
    /// any definition lacks a handler.
    pub pre_discovered_local_tools: Option<Arc<LocalToolRuntime>>,
    /// Task-scoped loopback MCP servers that expose local tools to an ACP
    /// subprocess. This is separate from `pre_discovered_local_tools`: ACP
    /// adapters receive transport configuration, never in-process handlers.
    pub local_tool_mcp_servers: Option<Arc<Vec<LocalToolMcpServer>>>,
    /// Runtime-scoped per-provider rate limiter shared across concurrent dispatches.
    ///
    /// When set, provider adapters that create HTTP-backed LLM backends (OpenAI-compat,
    /// Anthropic API, Gemini) should call `acquire(provider_id)` before each I/O request
    /// to enforce configured RPM/TPM budgets from `[providers.<name>].limits` in roko.toml.
    pub rate_limiter: Option<Arc<ProviderRateLimiter>>,
    /// Gemini-specific per-category safety thresholds from `[gemini].safety_settings`.
    /// Forwarded verbatim as `safetySettings` in `GenerateContentRequest`.
    pub gemini_safety_settings: Vec<roko_core::config::schema::SafetySetting>,
    /// Cancellation token propagated from the runner to the tool loop.
    ///
    /// When set, provider adapters that construct a `ToolLoopAgent` wire this
    /// token via [`ToolLoopAgent::with_cancel_token`] so that runner-level task
    /// cancellation (Skip/Cancel) immediately halts in-progress tool execution
    /// rather than waiting for the current LLM turn to complete.
    pub cancel_token: Option<Arc<dyn roko_core::tool::CancelToken>>,
    /// Persistent JSONL tool audit adapter.
    ///
    /// When set, the tool dispatcher records scrubbed admit/result lines
    /// to `.roko/tool_audit.jsonl` for every executed tool call.
    pub tool_audit: Option<Arc<roko_fs::tool_audit::ScrubAuditAdapter>>,
}

impl std::fmt::Debug for AgentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentOptions")
            .field("safety_layer", &self.safety_layer)
            .field("temperament", &self.temperament)
            .field("command", &self.command)
            .field("timeout_ms", &self.timeout_ms)
            .field(
                "system_prompt",
                &self
                    .system_prompt
                    .as_ref()
                    .map(|s| format!("{}...", &s[..s.len().min(40)])),
            )
            .field("input_messages", &self.input_messages.len())
            .field("bare_mode", &self.bare_mode)
            .field(
                "dangerously_skip_permissions",
                &self.dangerously_skip_permissions,
            )
            .field("name", &self.name)
            .field("cancel_token", &self.cancel_token.is_some())
            .field("tool_audit", &self.tool_audit.is_some())
            .finish_non_exhaustive()
    }
}

/// Authenticated per-call MCP endpoint for an ACP provider subprocess.
///
/// The bearer token is intentionally redacted from `Debug` output. Adapters
/// materialize the standard ACP HTTP server object only at `session/new`.
#[derive(Clone, PartialEq, Eq)]
pub struct LocalToolMcpServer {
    pub name: String,
    pub url: String,
    pub bearer_token: String,
}

impl LocalToolMcpServer {
    /// Serialize to ACP v1's flat HTTP MCP server shape.
    #[must_use]
    pub fn to_acp_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "http",
            "name": self.name,
            "url": self.url,
            "headers": [{
                "name": "Authorization",
                "value": format!("Bearer {}", self.bearer_token),
            }],
        })
    }
}

impl fmt::Debug for LocalToolMcpServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalToolMcpServer")
            .field("name", &self.name)
            .field("url", &self.url)
            .field("bearer_token", &"[REDACTED]")
            .finish()
    }
}

impl AgentOptions {
    /// Root the agent subprocess in the given working directory.
    #[must_use]
    pub fn with_working_dir(mut self, working_dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    /// Root durable immune controls independently from the effect worktree.
    #[must_use]
    pub fn with_immune_root(mut self, immune_root: impl Into<PathBuf>) -> Self {
        let immune_root = immune_root.into();
        self.immune_root = Some(immune_root.canonicalize().unwrap_or(immune_root));
        self
    }

    /// Resolve the explicit immune root, falling back for standalone callers.
    #[must_use]
    pub fn effective_immune_root(&self) -> Option<&Path> {
        self.immune_root.as_deref().or(self.working_dir.as_deref())
    }

    /// Resolve timeout: agent option > provider default > global default.
    #[must_use]
    pub fn effective_timeout_ms(&self, provider_default: Option<u64>) -> u64 {
        self.timeout_ms
            .or(provider_default)
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS)
    }

    /// Append Perplexity search options as a structured `extra_args` payload.
    #[must_use]
    pub fn with_perplexity_search_options(mut self, search_options: SearchOptions) -> Self {
        match serde_json::to_string(&search_options) {
            Ok(encoded) => {
                self.extra_args
                    .push(format!("{PERPLEXITY_SEARCH_OPTIONS_ARG_PREFIX}{encoded}"));
            }
            Err(e) => {
                tracing::warn!("failed to serialize Perplexity search options: {e}; skipping");
            }
        }
        self
    }
}

// ─── Human-readable provider error mapping ──────────────────────────────

/// Map a raw provider error string into a human-readable message with
/// actionable instructions.
///
/// This function inspects the error text for known patterns using simple
/// case-insensitive string matching. It does NOT depend on reqwest or std
/// internal types.
///
/// # Arguments
/// - `kind`: The provider kind (for context in the message).
/// - `provider_name`: The config key for this provider (e.g. "anthropic").
/// - `api_key_env`: The environment variable name for the API key, if any.
/// - `base_url`: The provider base URL, if known.
/// - `err`: The error to inspect (any displayable error).
///
/// # Returns
/// A human-readable error message. If no known pattern is matched, returns
/// a generic message wrapping the original error text.
#[must_use]
pub fn map_provider_error(
    kind: ProviderKind,
    provider_name: &str,
    api_key_env: Option<&str>,
    base_url: Option<&str>,
    err: &dyn std::fmt::Display,
) -> String {
    let err_text = err.to_string();
    let err_lower = err_text.to_lowercase();

    let env_var = api_key_env.unwrap_or("(none)");
    let url = base_url.unwrap_or("(unknown)");

    if err_lower.contains("401")
        || err_lower.contains("authentication_error")
        || err_lower.contains("unauthorized")
    {
        return format!(
            "API key invalid for provider '{}' (HTTP 401). Check ${} or roko.toml [providers.{}].",
            provider_name, env_var, provider_name
        );
    }

    if err_lower.contains("429")
        || err_lower.contains("rate_limit")
        || err_lower.contains("too many requests")
    {
        return format!(
            "Rate limited by provider '{}' (HTTP 429). Wait and retry, or switch providers.",
            provider_name
        );
    }

    if err_lower.contains("404")
        || err_lower.contains("model_not_found")
        || err_lower.contains("model not found")
    {
        return format!(
            "Model not found on provider '{}' (HTTP 404). Verify the slug in roko.toml [models.*].",
            provider_name
        );
    }

    if err_lower.contains("connection refused")
        || err_lower.contains("connecterror")
        || err_lower.contains("tcp connect error")
    {
        return format!(
            "Cannot reach provider '{}' at {}. Is the server running?",
            provider_name, url
        );
    }

    if err_lower.contains("no such file or directory")
        || err_lower.contains("program not found")
        || err_lower.contains("enoent")
    {
        return format!(
            "Provider binary not found on PATH for '{}'. Install it or configure a different provider in roko.toml.",
            provider_name
        );
    }

    // No known pattern matched — return a generic wrapper.
    format!(
        "Provider '{}' ({}) error: {}",
        provider_name,
        kind.label(),
        err_text
    )
}

#[derive(Debug, Clone)]
pub enum ProviderError {
    RateLimit { retry_after_ms: Option<u64> },
    AuthFailure,
    Timeout,
    ServerError(u16),
    ContentPolicy,
    ContextOverflow,
    ModelNotFound,
    Other(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimit { retry_after_ms } => match retry_after_ms {
                Some(ms) => write!(f, "rate limited; retry after {ms} ms"),
                None => f.write_str("rate limited"),
            },
            Self::AuthFailure => f.write_str("authentication failed"),
            Self::Timeout => f.write_str("request timed out"),
            Self::ServerError(status) => write!(f, "server error {status}"),
            Self::ContentPolicy => f.write_str("content policy violation"),
            Self::ContextOverflow => f.write_str("context overflow"),
            Self::ModelNotFound => f.write_str("model not found"),
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ProviderError {}

/// Retry decision for a classified provider error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAction {
    /// Wait for the specified delay, then retry the same provider.
    WaitAndRetry { delay_ms: u64 },
    /// Try a different provider or backend.
    TryFallback,
    /// Retry with a smaller or shorter context.
    TryWithSmallerContext,
    /// Do not retry this error.
    Skip,
}

/// Map a provider error class to a retry action.
#[must_use]
pub fn should_retry(error: &ProviderError) -> RetryAction {
    match error {
        ProviderError::RateLimit { retry_after_ms } => RetryAction::WaitAndRetry {
            delay_ms: retry_after_ms.unwrap_or(5_000),
        },
        ProviderError::AuthFailure => RetryAction::Skip,
        ProviderError::Timeout => RetryAction::TryFallback,
        ProviderError::ServerError(_) => RetryAction::TryFallback,
        ProviderError::ContentPolicy => RetryAction::Skip,
        ProviderError::ContextOverflow => RetryAction::TryWithSmallerContext,
        _ => RetryAction::TryFallback,
    }
}

impl ProviderError {
    #[must_use]
    pub const fn retry_after_ms(&self) -> Option<u64> {
        match self {
            Self::RateLimit { retry_after_ms } => *retry_after_ms,
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentCreationError {
    #[error("Missing API key: env var {0} not set")]
    MissingApiKey(String),
    #[error("Missing required config field: {0}")]
    MissingConfig(String),
    #[error("Invalid provider kind: {0:?}")]
    InvalidKind(ProviderKind),
    #[error("Failed to load mock fixture: {0}")]
    FixtureLoad(String),
    #[error("Binary not found on PATH: {0}")]
    BinaryNotFound(String),
    #[error("Provider process resource-limit enforcement failed: {0}")]
    ResourceLimitEnforcement(String),
    #[error("Invalid inline image input: {0}")]
    InvalidImageInput(String),
    #[error("Model {model:?} on provider {provider:?} cannot accept inline image input")]
    ImageInputUnsupported {
        model: String,
        provider: ProviderKind,
    },
    #[error(
        "Provider {0:?} cannot execute in-process local tools; use an authenticated MCP bridge or a provider-native tool loop"
    )]
    LocalToolsUnsupported(ProviderKind),
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::schema::{ModelProfile, ProviderConfig, RokoConfig};
    use roko_core::{Body, Context, Kind, Signal};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::time::timeout;

    fn prompt(text: &str) -> Signal {
        Signal::builder(Kind::Prompt).body(Body::text(text)).build()
    }

    #[test]
    fn local_tool_mcp_server_uses_authenticated_acp_http_shape_and_redacts_debug() {
        let server = LocalToolMcpServer {
            name: "roko_plugins".to_string(),
            url: "http://127.0.0.1:1234/mcp".to_string(),
            bearer_token: "secret-token".to_string(),
        };
        assert_eq!(
            server.to_acp_json(),
            serde_json::json!({
                "type": "http",
                "name": "roko_plugins",
                "url": "http://127.0.0.1:1234/mcp",
                "headers": [{"name": "Authorization", "value": "Bearer secret-token"}],
            })
        );
        let debug = format!("{server:?}");
        assert!(!debug.contains("secret-token"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn only_cursor_cli_and_hermes_acp_accept_per_call_local_mcp() {
        let provider = |kind| ProviderConfig {
            kind,
            base_url: None,
            api_key_env: None,
            command: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let cursor = provider(ProviderKind::CursorCli);
        assert!(adapter_for_kind(ProviderKind::CursorCli).supports_per_call_local_mcp(&cursor));

        let hermes_acp = ProviderConfig {
            command: Some("hermes".to_string()),
            args: Some(vec!["acp".to_string()]),
            ..provider(ProviderKind::Hermes)
        };
        assert!(adapter_for_kind(ProviderKind::Hermes).supports_per_call_local_mcp(&hermes_acp));

        let hermes_oneshot = ProviderConfig {
            command: Some("hermes".to_string()),
            ..provider(ProviderKind::Hermes)
        };
        assert!(
            !adapter_for_kind(ProviderKind::Hermes).supports_per_call_local_mcp(&hermes_oneshot)
        );
        let openclaw = provider(ProviderKind::OpenClaw);
        assert!(!adapter_for_kind(ProviderKind::OpenClaw).supports_per_call_local_mcp(&openclaw));
    }

    fn write_script(path: &std::path::Path, body: &str) {
        fs::write(path, body).expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).expect("script metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).expect("chmod script");
        }
    }

    fn spawn_chat_server(
        response: String,
    ) -> (String, Arc<Mutex<Option<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let captured = Arc::new(Mutex::new(None));
        let captured_request = Arc::clone(&captured);

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");

            let mut buf = Vec::new();
            let mut header_end = None;
            let mut content_length = None;

            loop {
                let mut chunk = [0_u8; 1024];
                let n = stream.read(&mut chunk).expect("read request");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);

                if header_end.is_none()
                    && let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    header_end = Some(pos + 4);
                    let headers = String::from_utf8_lossy(&buf[..pos + 4]);
                    content_length = headers.lines().find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    });
                }

                if let (Some(header_end), Some(content_length)) = (header_end, content_length)
                    && buf.len() >= header_end + content_length
                {
                    break;
                }
            }

            let header_end = header_end.expect("request headers");
            let content_length = content_length.expect("content length");
            let request = String::from_utf8_lossy(&buf[..header_end + content_length]).to_string();
            *captured_request.lock().expect("capture lock") = Some(request);

            let response_bytes = response.as_bytes();
            let wire = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_bytes.len(),
                response
            );
            stream.write_all(wire.as_bytes()).expect("write response");
            stream.flush().expect("flush response");
        });

        (format!("http://{}", addr), captured, handle)
    }

    fn test_config(base_url: String) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.providers.insert(
            "zai".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some(base_url),
                api_key_env: Some("PATH".to_string()),
                command: None,
                args: None,
                timeout_ms: Some(1_500),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "glm-5-1".to_string(),
            ModelProfile {
                provider: "zai".to_string(),
                slug: "glm-5.1".to_string(),
                context_window: 200_000,
                max_output: Some(1_024),
                supports_tools: true,
                supports_thinking: true,
                supports_vision: false,
                supports_web_search: false,
                supports_mcp_tools: false,
                supports_partial: false,
                supports_grounding: false,
                supports_code_execution: false,
                supports_caching: false,
                provider_routing: None,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: None,
                cost_output_per_m: None,
                cost_input_per_m_high: None,
                cost_output_per_m_high: None,
                cost_cache_read_per_m: None,
                cost_cache_write_per_m: None,
                thinking_level: None,
                max_tools: None,
                tokenizer_ratio: None,
                ..Default::default()
            },
        );
        config
    }

    fn perplexity_config(
        base_url: Option<String>,
        model_slug: &str,
        supports_async: bool,
    ) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.providers.insert(
            "perplexity".to_string(),
            ProviderConfig {
                kind: ProviderKind::PerplexityApi,
                base_url,
                api_key_env: Some("PATH".to_string()),
                command: None,
                args: None,
                timeout_ms: Some(300_000),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            model_slug.to_string(),
            ModelProfile {
                provider: "perplexity".to_string(),
                slug: model_slug.to_string(),
                context_window: 127_072,
                max_output: Some(8_192),
                supports_tools: false,
                supports_thinking: false,
                supports_vision: false,
                supports_web_search: true,
                supports_mcp_tools: false,
                supports_partial: false,
                supports_grounding: false,
                supports_code_execution: false,
                supports_caching: false,
                provider_routing: None,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: None,
                cost_output_per_m: None,
                cost_input_per_m_high: None,
                cost_output_per_m_high: None,
                cost_cache_read_per_m: None,
                cost_cache_write_per_m: None,
                thinking_level: None,
                max_tools: None,
                max_tool_iterations: None,
                tokenizer_ratio: None,
                supports_search: true,
                supports_citations: true,
                supports_async,
                is_embedding_model: false,
                search_context_size: Some("medium".to_string()),
                cost_per_request: None,
                use_max_completion_tokens: false,
                tier: None,
            },
        );
        config
    }

    #[test]
    fn tool_loop_iterations_derive_from_workspace_default() {
        assert_eq!(tool_loop_max_iterations(), DEFAULT_MAX_TOOL_ITERATIONS);
        assert_eq!(
            with_temperament(Some(Temperament::Conservative), tool_loop_max_iterations),
            DEFAULT_MAX_TOOL_ITERATIONS + 10
        );
        assert_eq!(
            with_temperament(Some(Temperament::Aggressive), tool_loop_max_iterations),
            DEFAULT_MAX_TOOL_ITERATIONS - 15
        );
        assert_eq!(
            with_temperament(Some(Temperament::Exploratory), tool_loop_max_iterations),
            DEFAULT_MAX_TOOL_ITERATIONS + 20
        );
    }

    #[test]
    fn tool_loop_iterations_respect_per_model_override() {
        let profile = ModelProfile {
            max_tool_iterations: Some(20),
            ..Default::default()
        };

        // Balanced: base unchanged → 20
        assert_eq!(
            with_temperament(Some(Temperament::Balanced), || {
                tool_loop_max_iterations_for_profile(Some(&profile))
            }),
            20
        );
        // Conservative: base + 10 → 30
        assert_eq!(
            with_temperament(Some(Temperament::Conservative), || {
                tool_loop_max_iterations_for_profile(Some(&profile))
            }),
            30
        );
        // Aggressive: max(10, base - 15) → max(10, 5) = 10
        assert_eq!(
            with_temperament(Some(Temperament::Aggressive), || {
                tool_loop_max_iterations_for_profile(Some(&profile))
            }),
            10
        );
        // Exploratory: base + 20 → 40
        assert_eq!(
            with_temperament(Some(Temperament::Exploratory), || {
                tool_loop_max_iterations_for_profile(Some(&profile))
            }),
            40
        );

        // None profile falls back to DEFAULT_MAX_TOOL_ITERATIONS
        assert_eq!(
            tool_loop_max_iterations_for_profile(None),
            DEFAULT_MAX_TOOL_ITERATIONS
        );

        // Profile without max_tool_iterations also falls back
        let profile_none = ModelProfile {
            max_tool_iterations: None,
            ..Default::default()
        };
        assert_eq!(
            tool_loop_max_iterations_for_profile(Some(&profile_none)),
            DEFAULT_MAX_TOOL_ITERATIONS
        );
    }

    #[test]
    fn adapter_for_kind_returns_expected_adapter() {
        assert_eq!(
            adapter_for_kind(ProviderKind::OpenAiCompat).kind(),
            ProviderKind::OpenAiCompat
        );
        assert_eq!(
            adapter_for_kind(ProviderKind::ClaudeCli).kind(),
            ProviderKind::ClaudeCli
        );
        assert_eq!(
            adapter_for_kind(ProviderKind::AnthropicApi).kind(),
            ProviderKind::AnthropicApi
        );
        assert_eq!(
            adapter_for_kind(ProviderKind::CursorAcp).kind(),
            ProviderKind::CursorAcp
        );
        assert_eq!(
            adapter_for_kind(ProviderKind::PerplexityApi).kind(),
            ProviderKind::PerplexityApi
        );
        assert_eq!(
            adapter_for_kind(ProviderKind::GeminiApi).kind(),
            ProviderKind::GeminiApi
        );
        assert_eq!(
            adapter_for_kind(ProviderKind::CerebrasApi).kind(),
            ProviderKind::CerebrasApi
        );
    }

    #[test]
    fn adapter_for_kind_hermes() {
        assert_eq!(
            adapter_for_kind(ProviderKind::Hermes).kind(),
            ProviderKind::Hermes
        );
    }

    #[test]
    fn adapter_for_kind_openclaw() {
        assert_eq!(
            adapter_for_kind(ProviderKind::OpenClaw).kind(),
            ProviderKind::OpenClaw
        );
    }

    #[test]
    fn image_input_fails_closed_without_a_vision_http_transport() {
        let image_options = || AgentOptions {
            input_messages: vec![roko_core::ModelInputMessage::new(
                roko_core::MessageRole::User,
                vec![roko_core::ModelInputBlock::image("image/png", "aGVsbG8=")],
            )],
            ..Default::default()
        };

        let no_config = create_agent_for_model(
            &RokoConfig::default(),
            "unconfigured-model",
            image_options(),
        );
        assert!(matches!(
            no_config,
            Err(AgentCreationError::ImageInputUnsupported { .. })
        ));

        let config = test_config("https://example.invalid/v1".to_string());
        let text_only_model = create_agent_for_model(&config, "glm-5-1", image_options());
        assert!(matches!(
            text_only_model,
            Err(AgentCreationError::ImageInputUnsupported {
                provider: ProviderKind::OpenAiCompat,
                ..
            })
        ));
    }

    #[test]
    fn build_tool_dispatcher_attaches_scoped_safety_layer() {
        fn no_handler(_: &str) -> Option<Arc<dyn roko_core::tool::ToolHandler>> {
            None
        }

        let registry: Arc<dyn roko_core::tool::ToolRegistry> =
            Arc::new(roko_core::tool::VecToolRegistry::from_tools(Vec::new()));
        let resolver: Arc<dyn HandlerResolver> = Arc::new(no_handler);

        let dispatcher = with_safety_layer(Some(SafetyLayer::with_defaults()), || {
            build_tool_dispatcher(registry, resolver)
        });

        // Safety is always present; verify it was constructed.
        let _ = dispatcher.safety();
    }

    #[test]
    fn agent_options_contract_is_the_live_provider_safety_contract() {
        use crate::safety::contract::{AgentContract, GovernanceRule};
        use roko_core::tool::{ToolCall, ToolContext};

        let known = AgentOptions {
            agent_contract: Some(AgentContract {
                role: "known-role".into(),
                allowed_tools: Some(vec!["read_file".into(), "bash".into()]),
                governance: vec![GovernanceRule::ForbiddenTools(vec!["bash".into()])],
                ..AgentContract::default()
            }),
            ..AgentOptions::default()
        };
        let layer = safety_layer_for_options(&RokoConfig::default(), &known);
        let ctx = ToolContext::testing("/tmp/provider-contract");
        assert!(
            layer
                .check_pre_execution(
                    &ToolCall::new("read", "read_file", serde_json::json!({})),
                    &ctx,
                )
                .is_ok()
        );
        assert!(
            layer
                .check_pre_execution(
                    &ToolCall::new("bash", "bash", serde_json::json!({"command": "echo ok"})),
                    &ctx,
                )
                .is_err()
        );

        let unknown = AgentOptions {
            agent_contract: Some(AgentContract::restricted("unknown-role")),
            ..AgentOptions::default()
        };
        let layer = safety_layer_for_options(&RokoConfig::default(), &unknown);
        assert!(
            layer
                .check_pre_execution(
                    &ToolCall::new("read", "read_file", serde_json::json!({})),
                    &ctx,
                )
                .is_err(),
            "provider dispatcher must preserve RestrictedFallback deny-all"
        );
    }

    #[test]
    fn with_scoped_safety_layer_defaults_when_unscoped() {
        let layer = with_scoped_safety_layer(current_safety_layer);
        assert!(layer.is_some());
    }

    #[test]
    fn with_scoped_safety_layer_preserves_existing_scope() {
        let observed = with_safety_layer(Some(SafetyLayer::with_defaults()), || {
            with_scoped_safety_layer(current_safety_layer)
        });
        assert!(observed.is_some());
    }

    #[tokio::test]
    async fn create_agent_for_model_returns_configured_agent() {
        let response = serde_json::json!({
            "id": "chatcmpl-test",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "factory-ok"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18
            }
        })
        .to_string();
        let (base_url, captured, handle) = spawn_chat_server(response);
        let config = test_config(format!("{base_url}/v4"));
        let options = AgentOptions {
            timeout_ms: Some(2_500),
            name: "factory-agent".to_string(),
            ..Default::default()
        };

        let agent =
            create_agent_for_model(&config, "glm-5-1", options).expect("create agent for model");
        assert_eq!(agent.name(), "factory-agent");

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );
        assert_eq!(result.output.body.as_text().unwrap_or(""), "factory-ok");

        let request = captured
            .lock()
            .expect("capture lock")
            .take()
            .expect("captured request");
        assert!(
            request.starts_with("POST /v4/chat/completions HTTP/1.1"),
            "unexpected request line: {}",
            request.lines().next().unwrap_or("")
        );
        let body = request.split("\r\n\r\n").nth(1).expect("request body");
        let parsed: serde_json::Value = serde_json::from_str(body).expect("json request body");
        assert_eq!(parsed["model"], "glm-5.1");
        assert_eq!(parsed["max_tokens"], 1024);
        assert_eq!(parsed["messages"][1]["content"], "hello");

        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn create_agent_for_model_routes_perplexity_search_grounded_chat() {
        let response = serde_json::json!({
            "id": "chatcmpl-pplx",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "research-ok"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 21,
                "completion_tokens": 9,
                "total_tokens": 30
            }
        })
        .to_string();
        let (base_url, captured, handle) = spawn_chat_server(response);
        let config = perplexity_config(Some(base_url.clone()), "sonar-pro", false);
        let options = AgentOptions {
            timeout_ms: Some(45_000),
            name: "research-agent".to_string(),
            ..Default::default()
        }
        .with_perplexity_search_options(SearchOptions {
            search_domain_filter: Some(vec!["arxiv.org".to_string(), "nature.com".to_string()]),
            search_recency_filter: Some("week".to_string()),
            search_context_size: Some("high".to_string()),
            search_mode: Some("academic".to_string()),
            return_images: Some(false),
            ..Default::default()
        });

        let agent = create_agent_for_model(&config, "sonar-pro", options)
            .expect("create perplexity chat agent");
        assert_eq!(agent.name(), "research-agent");

        let result = agent.run(&prompt("research"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap_or(""), "research-ok");

        let request = captured
            .lock()
            .expect("capture lock")
            .take()
            .expect("captured request");
        assert!(request.starts_with("POST /chat/completions HTTP/1.1"));
        let body = request.split("\r\n\r\n").nth(1).expect("request body");
        let parsed: serde_json::Value = serde_json::from_str(body).expect("json request body");
        assert_eq!(parsed["search_domain_filter"][0], "arxiv.org");
        assert_eq!(parsed["search_recency_filter"], "week");
        assert_eq!(parsed["search_mode"], "academic");
        assert_eq!(parsed["web_search_options"]["search_context_size"], "high");
        assert_eq!(parsed["return_images"], false);

        handle.join().expect("server thread");
    }

    #[test]
    fn create_agent_for_model_routes_perplexity_async_models_to_deep_research() {
        let config = perplexity_config(None, "sonar-deep-research", true);
        let agent = create_agent_for_model(
            &config,
            "sonar-deep-research",
            AgentOptions {
                name: "deep-research-agent".to_string(),
                ..Default::default()
            },
        )
        .expect("create deep research agent");
        assert_eq!(agent.name(), "deep-research-agent");
    }

    #[test]
    fn retry_policy_maps_error_classes() {
        assert_eq!(
            should_retry(&ProviderError::RateLimit {
                retry_after_ms: Some(1_250),
            }),
            RetryAction::WaitAndRetry { delay_ms: 1_250 }
        );
        assert_eq!(
            should_retry(&ProviderError::RateLimit {
                retry_after_ms: None,
            }),
            RetryAction::WaitAndRetry { delay_ms: 5_000 }
        );
        assert_eq!(should_retry(&ProviderError::AuthFailure), RetryAction::Skip);
        assert_eq!(
            should_retry(&ProviderError::Timeout),
            RetryAction::TryFallback
        );
        assert_eq!(
            should_retry(&ProviderError::ServerError(503)),
            RetryAction::TryFallback
        );
        assert_eq!(
            should_retry(&ProviderError::ContentPolicy),
            RetryAction::Skip
        );
        assert_eq!(
            should_retry(&ProviderError::ContextOverflow),
            RetryAction::TryWithSmallerContext
        );
        assert_eq!(
            should_retry(&ProviderError::Other("x".to_string())),
            RetryAction::TryFallback
        );
    }

    #[tokio::test]
    async fn exec_agent_fallback_for_unknown_model_key() {
        let mut config = RokoConfig::default();
        config.agent.command = Some("cat".to_string());

        let agent = create_agent_for_model(
            &config,
            "mystery-model",
            AgentOptions {
                timeout_ms: Some(250),
                name: "fallback-agent".to_string(),
                ..Default::default()
            },
        )
        .expect("fallback exec agent");

        assert_eq!(agent.name(), "fallback-agent");

        let result = agent.run(&prompt("fallback-ok"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap_or(""), "fallback-ok");
    }

    #[test]
    fn exec_agent_fallback_defaults_safety_layer_when_unscoped() {
        let mut config = RokoConfig::default();
        config.agent.command = Some("sh".to_string());

        let agent = create_agent_for_model(
            &config,
            "mystery-model",
            AgentOptions {
                timeout_ms: Some(250),
                name: "fallback-agent".to_string(),
                extra_args: vec!["-c".to_string(), "rm -rf /".to_string()],
                ..Default::default()
            },
        )
        .expect("fallback exec agent");
        assert_eq!(agent.name(), "fallback-agent");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let result = runtime.block_on(async { agent.run(&prompt(""), &Context::now()).await });
        assert!(!result.success);
        assert!(
            result
                .output
                .body
                .as_text()
                .unwrap_or("")
                .contains("blocked by safety layer")
        );
    }

    #[test]
    fn exec_agent_fallback_uses_scoped_safety_layer_when_active() {
        let mut config = RokoConfig::default();
        config.agent.command = Some("sh".to_string());

        let agent = with_safety_layer(Some(SafetyLayer::with_defaults()), || {
            create_agent_for_model(
                &config,
                "mystery-model",
                AgentOptions {
                    timeout_ms: Some(250),
                    name: "fallback-agent".to_string(),
                    extra_args: vec!["-c".to_string(), "rm -rf /".to_string()],
                    ..Default::default()
                },
            )
        })
        .expect("fallback exec agent");
        assert_eq!(agent.name(), "fallback-agent");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let result = runtime.block_on(async { agent.run(&prompt(""), &Context::now()).await });
        assert!(!result.success);
        assert!(
            result
                .output
                .body
                .as_text()
                .unwrap_or("")
                .contains("blocked by safety layer")
        );
    }

    #[test]
    fn known_protocol_command_detection_handles_paths() {
        assert!(is_known_protocol_command("claude"));
        assert!(is_known_protocol_command("/tmp/cursor-agent"));
        assert!(is_known_protocol_command("bin/cursor_agent"));
        assert!(is_known_protocol_command("/usr/local/bin/codex"));
        assert!(!is_known_protocol_command("cat"));
    }

    #[tokio::test]
    async fn create_agent_for_model_uses_effective_claude_provider_for_configured_model() {
        let tmp = tempdir().expect("tempdir");
        let script = tmp.path().join("claude");
        let prompt_file = tmp.path().join("prompt.txt");
        let response = r#"{"type":"content_block_delta","delta":{"text":"factory-claude-ok"}}"#;
        let script_body = format!(
            "#!/bin/sh\nset -eu\ncat > \"{}\"\nprintf '%s\\n' '{}'\n",
            prompt_file.display(),
            response,
        );
        write_script(&script, &script_body);
        let mut config = RokoConfig::default();
        config.agent.command = Some(script.display().to_string());
        // The protocol-command guard requires explicit provider+model config
        // when the command binary is a known protocol (e.g. "claude").
        config.providers.insert(
            "test-claude".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(5_000),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "claude-sonnet-4-6".to_string(),
            ModelProfile {
                provider: "test-claude".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                context_window: 200_000,
                max_output: Some(16_000),
                supports_tools: true,
                supports_thinking: true,
                ..Default::default()
            },
        );

        let agent = create_agent_for_model(
            &config,
            "claude-sonnet-4-6",
            AgentOptions {
                timeout_ms: Some(5_000),
                name: "factory-claude".to_string(),
                ..Default::default()
            },
        )
        .expect("create configured claude agent");

        assert_eq!(agent.name(), "factory-claude");

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(
            result.output.body.as_text().unwrap_or(""),
            "factory-claude-ok"
        );
        assert_eq!(
            fs::read_to_string(prompt_file).expect("read prompt file"),
            "hello"
        );
    }

    #[test]
    fn create_agent_for_model_rejects_protocol_command_without_model_config() {
        let result = create_agent_for_model(
            &RokoConfig::default(),
            "claude",
            AgentOptions {
                command: Some("claude".to_string()),
                timeout_ms: Some(5_000),
                name: "factory-claude".to_string(),
                ..Default::default()
            },
        );

        let Err(error) = result else {
            panic!("expected missing config error");
        };
        let AgentCreationError::MissingConfig(message) = error else {
            panic!("expected missing config error, got {error}");
        };
        assert!(message.contains("explicit [providers] and [models]"));
        assert!(message.contains("claude"));
    }

    #[test]
    fn opaque_cli_provider_rejects_unconsumed_local_tool_runtime() {
        let mut config = RokoConfig::default();
        config.providers.insert(
            "test-claude".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some("/bin/sh".to_string()),
                args: None,
                timeout_ms: Some(5_000),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "test-model".to_string(),
            ModelProfile {
                provider: "test-claude".to_string(),
                slug: "test-model".to_string(),
                supports_tools: true,
                ..Default::default()
            },
        );
        let mut plugin_tool = ToolDef::new(
            "demo.run",
            "test plugin tool",
            roko_core::tool::ToolCategory::Exec,
            roko_core::tool::ToolPermission::executes(),
        );
        plugin_tool.source = roko_core::tool::ToolSource::Plugin {
            name: "demo".to_string(),
        };
        let resolver: Arc<dyn HandlerResolver> = Arc::new(|_: &str| None);
        let result = create_agent_for_model(
            &config,
            "test-model",
            AgentOptions {
                pre_discovered_local_tools: Some(Arc::new(LocalToolRuntime::new(
                    vec![plugin_tool],
                    resolver,
                ))),
                ..Default::default()
            },
        );

        let Err(AgentCreationError::LocalToolsUnsupported(kind)) = result else {
            panic!("opaque provider must reject an unconsumed local tool runtime");
        };
        assert_eq!(kind, ProviderKind::ClaudeCli);
    }

    #[test]
    fn create_agent_for_model_rejects_model_profile_with_missing_provider() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.models.insert(
            "custom-model".to_string(),
            ModelProfile {
                provider: "missing-provider".to_string(),
                slug: "custom-slug".to_string(),
                ..Default::default()
            },
        );

        let result = create_agent_for_model(&config, "custom-model", AgentOptions::default());

        let Err(error) = result else {
            panic!("expected missing provider config error");
        };
        let AgentCreationError::MissingConfig(message) = error else {
            panic!("expected missing config error, got {error}");
        };
        assert!(message.contains("custom-model"));
        assert!(message.contains("missing-provider"));
    }

    #[tokio::test(start_paused = true)]
    async fn provider_semaphore_blocks_fourth_request_when_limit_is_three() {
        let mut configs = indexmap::IndexMap::new();
        configs.insert(
            "zai".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some("https://api.z.ai/api/paas/v4".to_string()),
                api_key_env: Some("ZAI_API_KEY".to_string()),
                command: None,
                args: None,
                timeout_ms: Some(1_500),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: Some(3),
                limits: None,
                require_confirmation: false,
            },
        );

        let semaphores = Arc::new(ProviderSemaphores::new(&configs));
        let permit_one = semaphores.acquire("zai").await;
        let permit_two = semaphores.acquire("zai").await;
        let permit_three = semaphores.acquire("zai").await;

        let blocked_semaphores = Arc::clone(&semaphores);
        let blocked = tokio::spawn(async move {
            timeout(Duration::from_millis(50), blocked_semaphores.acquire("zai")).await
        });
        tokio::time::advance(Duration::from_millis(50)).await;
        assert!(
            blocked
                .await
                .expect("blocked acquisition task should join")
                .is_err(),
            "fourth request should block while all permits are held"
        );

        drop(permit_one);

        let permit_four = semaphores.acquire("zai").await;

        drop(permit_two);
        drop(permit_three);
        drop(permit_four);
    }

    // ─── map_provider_error tests ─────────────────────────────────────

    #[test]
    fn map_provider_error_401_produces_api_key_message() {
        let msg = map_provider_error(
            ProviderKind::AnthropicApi,
            "anthropic",
            Some("ANTHROPIC_API_KEY"),
            Some("https://api.anthropic.com/v1"),
            &"status 401: authentication_error",
        );
        assert!(msg.contains("API key invalid"), "got: {msg}");
        assert!(msg.contains("anthropic"), "got: {msg}");
        assert!(msg.contains("ANTHROPIC_API_KEY"), "got: {msg}");
    }

    #[test]
    fn map_provider_error_429_produces_rate_limit_message() {
        let msg = map_provider_error(
            ProviderKind::OpenAiCompat,
            "openai",
            Some("OPENAI_API_KEY"),
            Some("https://api.openai.com/v1"),
            &"429 Too Many Requests",
        );
        assert!(msg.contains("Rate limited"), "got: {msg}");
        assert!(msg.contains("openai"), "got: {msg}");
    }

    #[test]
    fn map_provider_error_404_produces_model_not_found_message() {
        let msg = map_provider_error(
            ProviderKind::AnthropicApi,
            "anthropic",
            Some("ANTHROPIC_API_KEY"),
            None,
            &"404: model_not_found",
        );
        assert!(msg.contains("Model not found"), "got: {msg}");
        assert!(msg.contains("anthropic"), "got: {msg}");
    }

    #[test]
    fn map_provider_error_connection_refused_produces_server_message() {
        let msg = map_provider_error(
            ProviderKind::OpenAiCompat,
            "local-llm",
            None,
            Some("http://localhost:8080"),
            &"tcp connect error: connection refused",
        );
        assert!(msg.contains("Cannot reach"), "got: {msg}");
        assert!(msg.contains("local-llm"), "got: {msg}");
        assert!(msg.contains("http://localhost:8080"), "got: {msg}");
    }

    #[test]
    fn map_provider_error_enoent_produces_binary_not_found_message() {
        let msg = map_provider_error(
            ProviderKind::ClaudeCli,
            "claude",
            None,
            None,
            &"No such file or directory (os error 2)",
        );
        assert!(msg.contains("binary not found"), "got: {msg}");
        assert!(msg.contains("claude"), "got: {msg}");
    }

    #[test]
    fn map_provider_error_unknown_pattern_wraps_original() {
        let msg = map_provider_error(
            ProviderKind::GeminiApi,
            "gemini",
            Some("GEMINI_API_KEY"),
            None,
            &"some unknown error happened",
        );
        assert!(msg.contains("some unknown error happened"), "got: {msg}");
        assert!(msg.contains("gemini"), "got: {msg}");
    }

    // ─── Cross-PR harness adapter pipeline integration tests ─────────

    #[test]
    fn hermes_adapter_pipeline_end_to_end() {
        // 1. adapter_for_kind returns a HermesAdapter
        let adapter = adapter_for_kind(ProviderKind::Hermes);
        assert_eq!(adapter.kind(), ProviderKind::Hermes);

        // 2. create_agent with minimal config produces a Box<dyn Agent>
        let provider = ProviderConfig {
            kind: ProviderKind::Hermes,
            base_url: None,
            api_key_env: None,
            command: Some("hermes".to_string()),
            args: None,
            timeout_ms: Some(5_000),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "hermes".to_string(),
            slug: "hermes-3-llama-70b".to_string(),
            ..Default::default()
        };
        let options = AgentOptions {
            name: "hermes-pipeline-test".to_string(),
            ..Default::default()
        };
        let agent = adapter
            .create_agent(&provider, &model, &options)
            .expect("hermes adapter pipeline: create_agent must not panic");

        // 3. backend_id contains "hermes"
        assert!(
            agent.backend_id().contains("hermes"),
            "expected backend_id to contain 'hermes', got '{}'",
            agent.backend_id()
        );
    }

    #[test]
    fn openclaw_adapter_pipeline_end_to_end() {
        // 1. adapter_for_kind returns an OpenClawAdapter
        let adapter = adapter_for_kind(ProviderKind::OpenClaw);
        assert_eq!(adapter.kind(), ProviderKind::OpenClaw);

        // 2. create_agent with minimal config produces a Box<dyn Agent>
        let provider = ProviderConfig {
            kind: ProviderKind::OpenClaw,
            base_url: None,
            api_key_env: None,
            command: Some("openclaw".to_string()),
            args: None,
            timeout_ms: Some(5_000),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "openclaw".to_string(),
            slug: "openai/gpt-5.5".to_string(),
            ..Default::default()
        };
        let options = AgentOptions {
            name: "openclaw-pipeline-test".to_string(),
            ..Default::default()
        };
        let agent = adapter
            .create_agent(&provider, &model, &options)
            .expect("openclaw adapter pipeline: create_agent must not panic");

        // 3. backend_id contains "openclaw"
        assert!(
            agent.backend_id().contains("openclaw"),
            "expected backend_id to contain 'openclaw', got '{}'",
            agent.backend_id()
        );
    }
}
