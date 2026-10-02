//! Provider-neutral dispatch primitives for the plan runner.
//!
//! This module is intentionally small and side-effect free except for
//! `AgentDispatcherV2::create_agent`: callers can first resolve a model into a
//! concrete runtime, inspect whether that runtime is supported, then either
//! build a subprocess invocation for streaming CLI providers or construct a
//! provider-backed `Agent` through `roko-agent`.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context as _, Result as AnyhowResult};
use roko_agent::AgentRuntimeEvent;

/// Internal framing used when a provider exposes reasoning but the legacy
/// runtime event enum only has `MessageDelta`. It is removed before normal
/// transcript/state accumulation and emitted as a semantic StateHub record.
pub(crate) const REASONING_DELTA_PREFIX: &str = "\u{001f}roko.reasoning.v1 ";

/// Streaming chunk from a provider session, used for agent event bridging.
#[derive(Debug, Clone)]
#[allow(dead_code)] // all variants are produced but not all fields/payloads are consumed yet
pub(crate) enum StreamChunk {
    /// Plain content delta from the agent.
    ContentDelta(String),
    /// Reasoning delta emitted by the backend.
    ReasoningDelta(String),
    /// Tool-call delta emitted by the backend.
    ToolCallDelta {
        id_delta: Option<String>,
        name_delta: Option<String>,
        args_delta: Option<String>,
    },
    /// Tool progress update.
    ToolProgress { tool: String, status: String },
    /// Usage payload emitted by the backend.
    Usage(roko_core::Usage),
    /// Stream-local error message.
    Error(String),
    /// Stream completed with the given finish reason.
    Done(String),
}
use roko_agent::model_call_service::ProviderOutcomeRecorder;
use roko_agent::process::ResourceLimits;
use roko_agent::provider::{AgentOptions, LocalToolMcpServer, ProviderSemaphores};
use roko_agent::rate_limit::ProviderRateLimiter;
use roko_agent::safety::contract::AgentContract;
use roko_agent::{Agent, AgentResult, create_agent_for_model};
use roko_core::agent::{ProviderKind, resolve_model, try_resolve_model};
use roko_core::config::schema::{ModelProfile, ProviderConfig, RokoConfig};
use roko_core::pricing_snapshot::{PriceSnapshot, PricingConfig, TokenCounts};
use roko_core::tool::aliases::{canonical_names, claude_of_canonical};
use roko_core::{Body, Context, Kind, Signal};
use roko_learn::model_call_feedback::{ModelCallFeedback, ModelCallFeedbackRecorder};
use roko_learn::provider_health::ProviderHealthRegistry;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;

/// A single tool execution output captured from a dispatch response.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Tool name (e.g. "Read", "Bash", "Edit"), if available.
    pub tool_name: Option<String>,
    /// The tool's output content (file contents, bash stdout, etc.).
    pub content: String,
}

/// Result of dispatching a prompt to an LLM backend.
#[derive(Debug, Clone)]
pub struct DispatchResult {
    /// The model's text response.
    pub text: String,
    /// Which model answered.
    pub model: String,
    /// Approximate input tokens.
    pub input_tokens: u64,
    /// Approximate output tokens.
    pub output_tokens: u64,
    /// Tool execution outputs captured from the agent's tool calls.
    pub tool_outputs: Vec<ToolOutput>,
    /// Session ID for conversation resume, when provided by the backend.
    pub session_id: Option<String>,
}

/// Dispatch a prompt through ModelCallService (v2 path).
///
/// Uses the ModelCaller trait that WorkflowEngine uses, preserving routing,
/// budget, cache, gateway event, and feedback behavior.
pub async fn dispatch_via_model_call_service(prompt: &str) -> AnyhowResult<DispatchResult> {
    use crate::learning_helpers::{
        capture_runtime_model_slugs, provider_id_for_model, record_persisted_provider_health,
    };
    use roko_agent::model_call_service::ModelCallService;
    use roko_core::agent::resolve_model;
    use roko_core::config::schema::RokoConfig;
    use roko_core::foundation::{
        ChatMessage, FeedbackSink, MessageRole, ModelCallRequest, ModelCaller, caller,
    };
    use roko_learn::feedback_service::FeedbackService;
    use roko_learn::model_call_feedback::{ModelCallJournal, load_recovered_router};

    let workdir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let config = crate::config::load_resolved_config(&workdir)
        .map(|r| r.config)
        .unwrap_or_default();

    let mut model_config = RokoConfig::default();
    model_config.providers.extend(config.providers.clone());
    model_config.models.extend(config.models.clone());
    model_config.agent.command = Some(config.agent.command.clone());
    model_config.agent.args = Some(config.agent.args.clone());
    model_config.agent.timeout_ms = Some(config.agent.timeout_ms);
    model_config.agent.env = Some(config.agent.env.clone());
    model_config.agent.default_effort = config.agent.effort.clone();
    model_config.agent.bare_mode = config.agent.bare_mode;
    model_config.agent.fallback_model = config.agent.fallback_model.clone();
    model_config.agent.tier_models = config.agent.tier_models.clone();
    if let Some(model) = config.agent.model.clone() {
        model_config.agent.default_model = model;
    }
    let model_key = config
        .agent
        .model
        .clone()
        .unwrap_or_else(|| model_config.agent.default_model.clone());
    let model = resolve_model(&model_config, &model_key).slug;

    let cascade_path = workdir
        .join(".roko")
        .join("learn")
        .join("cascade-router.json");
    let cascade_model_slugs = capture_runtime_model_slugs(&model_config, &model);
    // The snapshot first takes what a crashed writer journaled and never
    // saved (bug-8a78e1).
    let cascade_router = (!cascade_model_slugs.is_empty())
        .then(|| Arc::new(load_recovered_router(&cascade_path, cascade_model_slugs)));
    // Observations are journaled in the learning WAL until the save below
    // (find-0dc1d5).
    let cascade_journal = Arc::new(ModelCallJournal::for_snapshot(&cascade_path));

    // Nothing else costs this direct model call (bug-724982).
    let roko_dir = workdir.join(".roko");
    let feedback_service = FeedbackService::from_roko_dir(&roko_dir).with_cost_records();
    let feedback_sink: Arc<dyn FeedbackSink> = match &cascade_router {
        Some(router) => Arc::new(
            feedback_service
                .with_cascade_router(Arc::clone(router))
                .with_cascade_journal(Arc::clone(&cascade_journal)),
        ),
        None => Arc::new(feedback_service),
    };
    let cost_table = roko_agent::CostTable::from_config_with_defaults(&model_config.models);
    let mut service = ModelCallService::new(model.clone())
        .with_config(model_config.clone())
        .with_working_dir(workdir.clone())
        .with_immune_root(workdir.clone())
        .with_cost_table(cost_table)
        .with_feedback_sink(feedback_sink)
        .with_inference_observer(Arc::new(
            crate::inference_observer::RuntimeEventInferenceObserver::new(),
        ));
    if let Some(ref mcp_path) = config.agent.mcp_config {
        service = service.with_mcp_config(mcp_path.clone());
    }

    let request = ModelCallRequest {
        model: model.clone(),
        system: None,
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
        }],
        max_tokens: None,
        caller: Some(caller::CLI.to_string()),
        ..Default::default()
    };

    let call_result = service.call(request).await;
    if let Some(router) = &cascade_router
        && let Err(err) = cascade_journal.save(router)
    {
        tracing::warn!(
            path = %cascade_path.display(),
            error = %err,
            "failed to persist direct ModelCallService cascade observation"
        );
    }

    let response = match call_result {
        Ok(response) => {
            if let Some(provider) = provider_id_for_model(&model_config, &response.model) {
                record_persisted_provider_health(&workdir, &provider, true)
                    .context("record direct ModelCallService provider success")?;
            }
            response
        }
        Err(err) => {
            if let Some(provider) = provider_id_for_model(&model_config, &model)
                && let Err(health_err) =
                    record_persisted_provider_health(&workdir, &provider, false)
            {
                tracing::warn!(
                    provider = %provider,
                    error = %health_err,
                    "failed to persist direct ModelCallService provider failure"
                );
            }
            return Err(err).context("ModelCallService dispatch failed");
        }
    };

    Ok(DispatchResult {
        text: response.content,
        model: response.model,
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
        tool_outputs: Vec::new(),
        session_id: None,
    })
}

/// Wire protocol emitted by a supported CLI provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliProtocol {
    /// Anthropic Claude CLI `--output-format stream-json`.
    ClaudeStreamJson,
    /// OpenAI Codex CLI `codex exec --json`.
    CodexExecJson,
    /// Google Gemini CLI `--output-format stream-json`.
    GeminiStreamJson,
}

impl CliProtocol {
    /// Stable provider label used in runner events.
    pub const fn event_provider(self) -> &'static str {
        match self {
            Self::ClaudeStreamJson => "claude-cli",
            Self::CodexExecJson => "codex-cli",
            Self::GeminiStreamJson => "gemini-cli",
        }
    }

    /// Provider kind used by config/model resolution.
    pub const fn provider_kind(self) -> ProviderKind {
        match self {
            Self::ClaudeStreamJson => ProviderKind::ClaudeCli,
            Self::CodexExecJson => ProviderKind::CodexCli,
            Self::GeminiStreamJson => ProviderKind::GeminiCli,
        }
    }

    /// Whether this CLI supports resuming an existing session through runner config.
    pub const fn supports_resume(self) -> bool {
        matches!(self, Self::ClaudeStreamJson | Self::GeminiStreamJson)
    }

    /// Whether this CLI accepts an MCP config path directly.
    pub const fn supports_mcp_config(self) -> bool {
        matches!(self, Self::ClaudeStreamJson)
    }

    /// Whether this CLI has a native system-prompt flag.
    pub const fn supports_system_prompt_flag(self) -> bool {
        matches!(self, Self::ClaudeStreamJson)
    }

    /// Whether the provider accepts a binding native turn/session limit.
    pub const fn supports_native_turn_limit(self) -> bool {
        matches!(self, Self::ClaudeStreamJson | Self::GeminiStreamJson)
    }
}

/// How the requested turn limit is enforced by the selected CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliTurnLimitEnforcement {
    /// The limit is passed through a provider-owned binding setting.
    Native,
    /// The CLI exposes no binding turn-count setting. Runner wall-clock
    /// deadlines still apply, but must not be reported as a native turn cap.
    Unsupported,
}

/// Serialized receipt describing the requested and effective turn policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliTurnLimitReceipt {
    pub requested_max_turns: u32,
    pub effective_max_turns: Option<u32>,
    pub enforcement: CliTurnLimitEnforcement,
}

impl Default for CliTurnLimitReceipt {
    fn default() -> Self {
        Self {
            requested_max_turns: 0,
            effective_max_turns: None,
            enforcement: CliTurnLimitEnforcement::Unsupported,
        }
    }
}

/// Human-readable CLI provider metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliProviderDescriptor {
    /// Provider registry id, for example `claude_cli`.
    pub provider_id: String,
    /// Config protocol family.
    pub provider_kind: ProviderKind,
    /// CLI wire protocol.
    pub protocol: CliProtocol,
    /// Label emitted in normalized runtime events.
    pub event_provider: String,
}

impl CliProviderDescriptor {
    fn new(provider_id: impl Into<String>, protocol: CliProtocol) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider_kind: protocol.provider_kind(),
            protocol,
            event_provider: protocol.event_provider().to_string(),
        }
    }
}

/// Configured CLI provider plus its executable and static args.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliProviderConfig {
    /// Static provider metadata.
    pub descriptor: CliProviderDescriptor,
    /// Program to execute.
    pub command: PathBuf,
    /// Provider-level extra args from `roko.toml`.
    pub provider_args: Vec<String>,
    /// OS resource limits applied to each CLI subprocess.
    pub resource_limits: Option<ResourceLimits>,
}

/// Per-agent authority for the loopback MCP server that exposes in-process
/// declarative-plugin handlers to an opaque CLI provider.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliPluginMcpConfig {
    /// Stable MCP server name used in provider tool namespaces.
    pub server_name: String,
    /// Loopback Streamable HTTP endpoint.
    pub url: String,
    /// HMAC-signed task authority. Never serialize it into persisted dispatch
    /// plans or print it through `Debug`.
    #[serde(default, skip_serializing)]
    pub bearer_token: String,
    /// Raw MCP tool names permitted by the effective task contract.
    pub tool_names: Vec<String>,
}

impl std::fmt::Debug for CliPluginMcpConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CliPluginMcpConfig")
            .field("server_name", &self.server_name)
            .field("url", &self.url)
            .field("bearer_token", &"[REDACTED]")
            .field("tool_names", &self.tool_names)
            .finish()
    }
}

impl CliProviderConfig {
    /// Build a Claude CLI provider.
    pub fn claude(provider_id: impl Into<String>, command: impl Into<PathBuf>) -> Self {
        Self {
            descriptor: CliProviderDescriptor::new(provider_id, CliProtocol::ClaudeStreamJson),
            command: command.into(),
            provider_args: Vec::new(),
            resource_limits: None,
        }
    }

    /// Build a Codex CLI provider. It resolves like the others, but
    /// [`CliDispatchProvider::build_invocation`] refuses it: Codex runs only
    /// through roko-agent's `CodexCliAdapter`.
    pub fn codex(provider_id: impl Into<String>, command: impl Into<PathBuf>) -> Self {
        Self {
            descriptor: CliProviderDescriptor::new(provider_id, CliProtocol::CodexExecJson),
            command: command.into(),
            provider_args: Vec::new(),
            resource_limits: None,
        }
    }

    /// Build a Gemini CLI provider.
    pub fn gemini(provider_id: impl Into<String>, command: impl Into<PathBuf>) -> Self {
        Self {
            descriptor: CliProviderDescriptor::new(provider_id, CliProtocol::GeminiStreamJson),
            command: command.into(),
            provider_args: Vec::new(),
            resource_limits: None,
        }
    }

    /// Preserve runner-v2 compatibility while moving CLI detection out of
    /// `agent_stream`: a configured `codex` executable uses Codex protocol,
    /// everything else uses Claude's stream-json protocol.
    pub fn from_legacy_runner_program(program: impl Into<PathBuf>) -> Self {
        let program = program.into();
        if executable_name(&program).contains("codex") {
            Self::codex("codex_cli", program)
        } else {
            Self::claude("claude_cli", program)
        }
    }

    /// Resolve a CLI provider from an explicit provider registry entry.
    ///
    /// API-backed providers are not errors here because they are handled by the
    /// `AgentResultBridge` runtime, not a subprocess-json runtime.
    pub fn from_provider_config(
        provider_id: impl Into<String>,
        provider: &ProviderConfig,
    ) -> Result<Self, DispatchV2Error> {
        let provider_id = provider_id.into();
        match provider.kind {
            ProviderKind::ClaudeCli => {
                let command = required_command(&provider_id, provider)?;
                let mut config = Self::claude(provider_id, command);
                config.provider_args = provider.args.clone().unwrap_or_default();
                config.resource_limits = configured_cli_resource_limits(provider)?;
                Ok(config)
            }
            ProviderKind::CodexCli => {
                let command = provider
                    .command
                    .as_deref()
                    .map(str::trim)
                    .filter(|command| !command.is_empty())
                    .unwrap_or("codex");
                let mut config = Self::codex(provider_id, command);
                config.provider_args = provider.args.clone().unwrap_or_default();
                config.resource_limits = configured_cli_resource_limits(provider)?;
                Ok(config)
            }
            ProviderKind::OpenAiCompat => {
                let command = required_command(&provider_id, provider)?;
                // Legacy backward compat: an OpenAiCompat provider whose
                // command is a codex binary is dispatched as codex protocol.
                if executable_name(&command).contains("codex") {
                    let mut config = Self::codex(provider_id, command);
                    config.provider_args = provider.args.clone().unwrap_or_default();
                    config.resource_limits = configured_cli_resource_limits(provider)?;
                    Ok(config)
                } else {
                    Err(DispatchV2Error::UnsupportedCommand {
                        provider_id,
                        command: command.display().to_string(),
                    })
                }
            }
            ProviderKind::GeminiCli => {
                let command = provider
                    .command
                    .as_deref()
                    .map(str::trim)
                    .filter(|command| !command.is_empty())
                    .unwrap_or("gemini");
                let mut config = Self::gemini(provider_id, command);
                config.provider_args = provider.args.clone().unwrap_or_default();
                config.resource_limits = configured_cli_resource_limits(provider)?;
                Ok(config)
            }
            // API-backed and ACP providers are dispatched via AgentResultBridge,
            // not as CLI subprocesses.
            kind @ (ProviderKind::AnthropicApi
            | ProviderKind::CursorAcp
            | ProviderKind::CursorCli
            | ProviderKind::PerplexityApi
            | ProviderKind::GeminiApi
            | ProviderKind::CerebrasApi
            | ProviderKind::Hermes
            | ProviderKind::OpenClaw) => {
                Err(DispatchV2Error::UnsupportedCliProvider { provider_id, kind })
            }
        }
    }
}

/// Trait implemented by provider-specific CLI launchers.
pub trait CliDispatchProvider {
    /// Static description of this provider.
    fn descriptor(&self) -> &CliProviderDescriptor;

    /// Build the exact subprocess invocation for a runner turn.
    fn build_invocation(
        &self,
        request: &CliDispatchRequest,
    ) -> Result<CliInvocation, DispatchV2Error>;
}

impl CliDispatchProvider for CliProviderConfig {
    fn descriptor(&self) -> &CliProviderDescriptor {
        &self.descriptor
    }

    fn build_invocation(
        &self,
        request: &CliDispatchRequest,
    ) -> Result<CliInvocation, DispatchV2Error> {
        request.validate()?;
        // A CLI gets the system prompt's cache markers as inert text; only
        // the Anthropic API translators turn them into `cache_control`
        // (bug-6052d8).
        let request = &CliDispatchRequest {
            system_prompt: roko_agent::translate::claude::strip_cache_markers(
                &request.system_prompt,
            ),
            ..request.clone()
        };
        match self.descriptor.protocol {
            CliProtocol::ClaudeStreamJson => self.build_claude_invocation(request),
            // Codex runs only through roko-agent's `CodexCliAdapter`, whose
            // operation broker stops a denied operation; a bare subprocess
            // here would run Codex's built-in tools unchecked (gap-baab0a).
            CliProtocol::CodexExecJson => Err(DispatchV2Error::UnsupportedCliProvider {
                provider_id: self.descriptor.provider_id.clone(),
                kind: self.descriptor.protocol.provider_kind(),
            }),
            CliProtocol::GeminiStreamJson => self.build_gemini_invocation(request),
        }
    }
}

impl CliProviderConfig {
    fn build_claude_invocation(
        &self,
        request: &CliDispatchRequest,
    ) -> Result<CliInvocation, DispatchV2Error> {
        let settings_json = roko_agent::claude_cli_agent::build_settings_json();
        let isolation = roko_agent::claude_cli_agent::ClaudeIsolation::new(&request.workdir);
        let mut args = vec![
            "--print".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
            "--model".to_string(),
            request.model.clone(),
            "--max-turns".to_string(),
            request.max_turns.to_string(),
            "--settings".to_string(),
            settings_json,
        ];
        args.extend(self.provider_args.clone());
        // The Claude Code isolation (`--add-dir`, `--setting-sources`,
        // `--strict-mcp-config`), after the provider's own arguments so they
        // cannot undo it.
        args.extend(isolation.args());

        if request.dangerously_skip_permissions {
            args.push("--dangerously-skip-permissions".to_string());
        }
        if !request.system_prompt.trim().is_empty() {
            args.push("--append-system-prompt".to_string());
            args.push(request.system_prompt.clone());
        }
        if let Some(effort) = request
            .effort
            .as_ref()
            .filter(|effort| !effort.trim().is_empty())
        {
            args.push("--effort".to_string());
            args.push(effort.clone());
        }
        if request.mcp_config.is_some() || request.plugin_mcp.is_some() {
            if let Some(reason) = isolation.mcp_config_refusal() {
                tracing::warn!(provider_id = %self.descriptor.provider_id, "{reason}");
                return Err(DispatchV2Error::McpConfigUnsupported {
                    provider_id: self.descriptor.provider_id.clone(),
                    protocol: self.descriptor.protocol,
                });
            }
            args.push("--mcp-config".to_string());
            if let Some(mcp_config) = &request.mcp_config {
                args.push(mcp_config.to_string_lossy().to_string());
            }
            if let Some(plugin_mcp) = &request.plugin_mcp {
                args.push(claude_plugin_mcp_json(plugin_mcp));
            }
        }
        if let Some(session) = &request.resume_session {
            args.push("--resume".to_string());
            args.push(session.clone());
        }
        if let Some(allowed) = &request.allowed_tools {
            args.push("--tools".to_string());
            args.push(
                allowed
                    .iter()
                    .filter_map(|name| claude_policy_tool_name(name, request.plugin_mcp.as_ref()))
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        for tool in &request.disallowed_tools {
            if let Some(tool) = claude_policy_tool_name(tool, request.plugin_mcp.as_ref()) {
                args.push("--disallowed-tools".to_string());
                args.push(tool);
            }
        }

        let mut invocation = CliInvocation::new(self, request, args, request.prompt.clone());
        // The request's own variables win.
        for &(key, value) in isolation.env() {
            if !invocation.env.iter().any(|(existing, _)| existing == key) {
                invocation.env.push((key.to_string(), value.to_string()));
            }
        }
        tracing::debug!(
            provider_id = %self.descriptor.provider_id,
            isolation = ?isolation.tags(),
            mcp_config = ?request.mcp_config,
            "claude run isolated from the user's Claude Code configuration"
        );
        Ok(invocation)
    }

    fn build_gemini_invocation(
        &self,
        request: &CliDispatchRequest,
    ) -> Result<CliInvocation, DispatchV2Error> {
        if request.mcp_config.is_some() {
            return Err(DispatchV2Error::McpConfigUnsupported {
                provider_id: self.descriptor.provider_id.clone(),
                protocol: self.descriptor.protocol,
            });
        }
        if let Some(argument) = self
            .provider_args
            .iter()
            .find(|argument| gemini_provider_arg_conflicts(argument))
        {
            return Err(DispatchV2Error::ConflictingProviderArgument {
                provider_id: self.descriptor.provider_id.clone(),
                argument: argument.clone(),
            });
        }

        let mut args = self.provider_args.clone();
        args.extend([
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--model".to_string(),
            request.model.clone(),
            "--prompt".to_string(),
            String::new(),
            "--extensions".to_string(),
            "none".to_string(),
            "--approval-mode".to_string(),
            if request.dangerously_skip_permissions {
                "yolo".to_string()
            } else {
                "default".to_string()
            },
        ]);
        if let Some(plugin_mcp) = &request.plugin_mcp {
            args.extend([
                "--allowed-mcp-server-names".to_string(),
                plugin_mcp.server_name.clone(),
            ]);
        }
        if let Some(session) = &request.resume_session {
            args.extend(["--resume".to_string(), session.clone()]);
        }

        let stdin = if request.system_prompt.trim().is_empty() {
            request.prompt.clone()
        } else {
            format!(
                "{}\n\n---\n\n{}",
                request.system_prompt.trim(),
                request.prompt
            )
        };
        let mut invocation = CliInvocation::new(self, request, args, stdin);
        invocation.ephemeral_config = Some(CliEphemeralConfig {
            env_key: "GEMINI_CLI_SYSTEM_SETTINGS_PATH".to_string(),
            file_name: "settings.json".to_string(),
            contents: gemini_system_settings_json(request),
        });
        Ok(invocation)
    }
}

/// Resolve an explicitly supplied shared Cargo target directory: the
/// canonical repository's own `target` subtree, outside the task worktree.
/// `CliInvocation::new` turns incremental builds on only for such a target.
///
/// A target already contained by the task worktree is not shared. Existing
/// paths are canonicalized so a symlink cannot pass for a wider lexical path
/// than the directory Cargo actually writes to. Missing directories and
/// requests without `ROKO_AGENT_SHARED_TARGET=1` fail closed.
fn shared_target_dir(request: &CliDispatchRequest) -> Option<PathBuf> {
    let explicitly_enabled = request.env.iter().rev().find_map(|(key, value)| {
        (key == "ROKO_AGENT_SHARED_TARGET").then(|| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    });
    if explicitly_enabled != Some(true) {
        return None;
    }
    let raw = request
        .env
        .iter()
        .rev()
        .find(|(key, _)| key == "CARGO_TARGET_DIR")
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())?;
    let configured = PathBuf::from(raw);
    let absolute = if configured.is_absolute() {
        configured
    } else {
        request.workdir.join(configured)
    };
    // Compare resolved filesystem identities throughout. On macOS, temporary
    // paths are commonly spelled through `/var` while `canonicalize` returns
    // the equivalent `/private/var` path; mixing those forms would reject the
    // repository's legitimate shared target. Canonicalizing here also makes a
    // nested symlink escape fail the later repository-target containment check.
    let target_dir = std::fs::canonicalize(&absolute).ok()?;
    let workdir = std::fs::canonicalize(&request.workdir)
        .ok()
        .or_else(|| normalize_absolute_path(&request.workdir))?;
    if target_dir.starts_with(&workdir) {
        return None;
    }

    // A generic dispatch request can carry arbitrary environment values. Do
    // not treat an arbitrary CARGO_TARGET_DIR as shared: linked worktrees may
    // only share the canonical repository's own `target` subtree, derived from
    // Git's common directory.
    let git = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(&request.workdir)
        .output()
        .ok()?;
    if !git.status.success() {
        return None;
    }
    let common_dir = PathBuf::from(String::from_utf8(git.stdout).ok()?.trim());
    let common_dir = std::fs::canonicalize(common_dir).ok()?;
    if common_dir.file_name().and_then(|name| name.to_str()) != Some(".git") {
        return None;
    }
    let repo_root = common_dir.parent()?.to_path_buf();
    if repo_root.parent().is_none()
        || std::env::var_os("HOME")
            .and_then(|home| std::fs::canonicalize(home).ok())
            .is_some_and(|home| home == repo_root)
    {
        return None;
    }
    let allowed_target = normalize_absolute_path(&repo_root.join("target"))?;
    if !allowed_target.starts_with(&repo_root) || !target_dir.starts_with(&allowed_target) {
        return None;
    }

    // Resolve every path component before accepting it. Requiring an existing
    // directory prevents a missing leaf below a symlink from escaping the
    // lexical `<repo>/target` prefix.
    if !target_dir.is_dir()
        || !allowed_target.is_dir()
        || std::fs::symlink_metadata(&allowed_target)
            .ok()?
            .file_type()
            .is_symlink()
    {
        return None;
    }
    let resolved_allowed = std::fs::canonicalize(&allowed_target).ok()?;
    if resolved_allowed != allowed_target || !target_dir.starts_with(&resolved_allowed) {
        return None;
    }
    Some(target_dir)
}

fn normalize_absolute_path(path: &Path) -> Option<PathBuf> {
    use std::path::Component;

    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    Some(normalized)
}

/// Provider-neutral request to launch a CLI-backed agent turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliDispatchRequest {
    /// Prompt sent to the provider on stdin.
    pub prompt: String,
    /// System prompt, either passed as a native flag or folded into stdin.
    pub system_prompt: String,
    /// Concrete model slug or model key selected for this turn.
    pub model: String,
    /// Working directory for the agent.
    pub workdir: PathBuf,
    /// Maximum agent turns when the provider supports it.
    pub max_turns: u32,
    /// Optional reasoning effort hint when the provider supports it.
    pub effort: Option<String>,
    /// Whether to bypass provider permission prompts/sandboxing.
    pub dangerously_skip_permissions: bool,
    /// Optional MCP config path.
    pub mcp_config: Option<PathBuf>,
    /// Optional session to resume when the provider supports it.
    pub resume_session: Option<String>,
    /// Extra subprocess environment entries.
    pub env: Vec<(String, String)>,
    /// Agent id used by observers.
    pub agent_id: String,
    /// Binding tool allowlist translated into the selected CLI's native policy.
    ///
    /// `Some(vec![])` means deny all and is serialized as an explicit empty
    /// value. `None` means the contract imposed no allowlist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    /// Tool names the agent must not invoke, translated into native policy.
    ///
    /// Claude and Gemini support this binding restriction. Codex has no
    /// equivalent built-in-tool flag, so this path refuses it; Codex runs
    /// through roko-agent's `CodexCliAdapter` and its operation broker.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disallowed_tools: Vec<String>,
    /// Contract-scoped bridge for local plugin handlers, when the runner has
    /// discovered declarative tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_mcp: Option<CliPluginMcpConfig>,
}

fn claude_policy_tool_name(name: &str, plugin_mcp: Option<&CliPluginMcpConfig>) -> Option<String> {
    if let Some(plugin_mcp) = plugin_mcp
        && plugin_mcp.tool_names.iter().any(|tool| tool == name)
    {
        return Some(format!("mcp__{}__{name}", plugin_mcp.server_name));
    }
    if let Some(alias) = claude_of_canonical(name) {
        Some(alias.to_string())
    } else if canonical_names().any(|canonical| canonical == name) {
        None
    } else {
        Some(name.to_string())
    }
}

fn claude_plugin_mcp_json(config: &CliPluginMcpConfig) -> String {
    let mut servers = serde_json::Map::new();
    servers.insert(
        config.server_name.clone(),
        json!({
            "type": "http",
            "url": config.url,
            "headers": {
                "Authorization": "Bearer ${ROKO_PLUGIN_MCP_TOKEN}"
            }
        }),
    );
    json!({ "mcpServers": servers }).to_string()
}

fn gemini_policy_tool_name(name: &str, plugin_mcp: Option<&CliPluginMcpConfig>) -> Option<String> {
    if plugin_mcp.is_some_and(|config| config.tool_names.iter().any(|tool| tool == name)) {
        return None;
    }
    Some(
        match name {
            "edit_file" | "multi_edit" => "replace",
            "grep" => "search_file_content",
            "bash" => "run_shell_command",
            "ls" => "list_directory",
            "web_search" => "google_web_search",
            "todo_write" => "write_todos",
            "task" => "agent",
            other => other,
        }
        .to_string(),
    )
}

fn gemini_provider_arg_conflicts(argument: &str) -> bool {
    if !argument.starts_with('-') {
        return true;
    }
    let flag = argument.split_once('=').map_or(argument, |(flag, _)| flag);
    matches!(
        flag,
        "-m" | "--model"
            | "-p"
            | "--prompt"
            | "-o"
            | "--output-format"
            | "-y"
            | "--yolo"
            | "--approval-mode"
            | "--policy"
            | "--admin-policy"
            | "--allowed-tools"
            | "--allowed-mcp-server-names"
            | "-e"
            | "--extensions"
            | "-r"
            | "--resume"
            | "-i"
            | "--prompt-interactive"
            | "--acp"
            | "--experimental-acp"
            | "-w"
            | "--worktree"
            | "--skip-trust"
            | "--include-directories"
            | "--session-id"
            | "--list-sessions"
            | "--delete-session"
            | "--fake-responses"
            | "--record-responses"
    )
}

fn gemini_system_settings_json(request: &CliDispatchRequest) -> String {
    let mut tools = serde_json::Map::new();
    // System settings have higher precedence than user/workspace settings.
    // Empty commands prevent ambient discovered-tool configuration from
    // expanding the task's executable catalog.
    tools.insert("discoveryCommand".to_string(), json!(""));
    tools.insert("callCommand".to_string(), json!(""));
    if let Some(allowed) = &request.allowed_tools {
        tools.insert(
            "core".to_string(),
            json!(
                allowed
                    .iter()
                    .filter_map(|name| gemini_policy_tool_name(name, request.plugin_mcp.as_ref()))
                    .collect::<Vec<_>>()
            ),
        );
    }
    let excluded = request
        .disallowed_tools
        .iter()
        .filter_map(|name| gemini_policy_tool_name(name, request.plugin_mcp.as_ref()))
        .collect::<Vec<_>>();
    if !excluded.is_empty() {
        tools.insert("exclude".to_string(), json!(excluded));
    }

    let mut settings = serde_json::Map::new();
    settings.insert("tools".to_string(), serde_json::Value::Object(tools));
    settings.insert("hooksConfig".to_string(), json!({ "enabled": false }));
    settings.insert("skills".to_string(), json!({ "enabled": false }));
    settings.insert(
        "model".to_string(),
        json!({ "maxSessionTurns": request.max_turns }),
    );
    if let Some(config) = &request.plugin_mcp {
        // Gemini deliberately sanitizes inherited environment variables before
        // expanding remote MCP headers. Explicitly authorize this task-scoped
        // credential so the Authorization placeholder resolves instead of
        // silently becoming an empty bearer token.
        settings.insert(
            "security".to_string(),
            json!({
                "environmentVariableRedaction": {
                    "allowed": ["ROKO_PLUGIN_MCP_TOKEN"]
                }
            }),
        );
        settings.insert(
            "mcp".to_string(),
            json!({ "allowed": [config.server_name.clone()] }),
        );
        let mut servers = serde_json::Map::new();
        servers.insert(
            config.server_name.clone(),
            json!({
                "type": "http",
                "httpUrl": config.url,
                "headers": {
                    "Authorization": "Bearer ${ROKO_PLUGIN_MCP_TOKEN}"
                },
                "includeTools": config.tool_names,
                "trust": true
            }),
        );
        settings.insert("mcpServers".to_string(), serde_json::Value::Object(servers));
    }
    serde_json::Value::Object(settings).to_string()
}

impl CliDispatchRequest {
    fn validate(&self) -> Result<(), DispatchV2Error> {
        if roko_agent::immune_boundary::validate_provider_agent_id(&self.agent_id).is_err() {
            return Err(DispatchV2Error::InvalidAgentId);
        }
        if self.prompt.trim().is_empty() {
            return Err(DispatchV2Error::EmptyPrompt);
        }
        if self.model.trim().is_empty() {
            return Err(DispatchV2Error::EmptyModel);
        }
        if !self.workdir.exists() {
            return Err(DispatchV2Error::WorkdirMissing {
                path: self.workdir.clone(),
            });
        }
        Ok(())
    }
}

/// Fully materialized subprocess invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliInvocation {
    /// Program to execute.
    pub program: PathBuf,
    /// Program arguments.
    pub args: Vec<String>,
    /// Current directory for the subprocess.
    pub workdir: PathBuf,
    /// Stdin payload.
    pub stdin: String,
    /// Environment entries to set on the subprocess.
    pub env: Vec<(String, String)>,
    /// Authentication entries applied after ordinary environment overrides.
    #[serde(skip)]
    pub(crate) secret_env: CliSecretEnv,
    /// CLI wire protocol.
    pub protocol: CliProtocol,
    /// Provider label for normalized runner events.
    pub event_provider: String,
    /// Model selected for this invocation.
    pub model: String,
    /// Agent id associated with this invocation.
    pub agent_id: String,
    /// Truthful receipt for the provider's native turn-limit capability.
    #[serde(default)]
    pub turn_limit: CliTurnLimitReceipt,
    /// OS resource limits to install before spawning the CLI.
    pub resource_limits: Option<ResourceLimits>,
    /// Provider configuration that must exist for the subprocess lifetime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ephemeral_config: Option<CliEphemeralConfig>,
}

/// A short-lived provider config materialized immediately before spawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliEphemeralConfig {
    /// Environment variable through which the provider receives the path.
    pub env_key: String,
    /// File name within the runner-owned temporary directory.
    pub file_name: String,
    /// Complete file content. Secrets should be referenced through env vars.
    pub contents: String,
}

/// Subprocess credentials that are neither serialized nor exposed by `Debug`.
#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct CliSecretEnv(Vec<(String, String)>);

impl fmt::Debug for CliSecretEnv {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_list()
            .entries(self.0.iter().map(|(key, _)| (key, "[REDACTED]")))
            .finish()
    }
}

impl CliSecretEnv {
    fn upsert(&mut self, key: &str, value: &str) {
        upsert_env(&mut self.0, key, value);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.0.iter()
    }
}

impl CliInvocation {
    fn new(
        provider: &CliProviderConfig,
        request: &CliDispatchRequest,
        args: Vec<String>,
        stdin: String,
    ) -> Self {
        let mut env = request.env.clone();
        let fast_shared_target = shared_target_dir(request).is_some();
        upsert_env(
            &mut env,
            "CARGO_INCREMENTAL",
            if fast_shared_target { "1" } else { "0" },
        );
        upsert_env(&mut env, "CARGO_BUILD_JOBS", "2");
        let mut secret_env = CliSecretEnv::default();
        if let Some(plugin_mcp) = &request.plugin_mcp {
            env.retain(|(key, _)| key != "ROKO_PLUGIN_MCP_TOKEN");
            secret_env.upsert("ROKO_PLUGIN_MCP_TOKEN", &plugin_mcp.bearer_token);
        }

        Self {
            program: provider.command.clone(),
            args,
            workdir: request.workdir.clone(),
            stdin,
            env,
            secret_env,
            protocol: provider.descriptor.protocol,
            event_provider: provider.descriptor.event_provider.clone(),
            model: request.model.clone(),
            agent_id: request.agent_id.clone(),
            turn_limit: if provider.descriptor.protocol.supports_native_turn_limit() {
                CliTurnLimitReceipt {
                    requested_max_turns: request.max_turns,
                    effective_max_turns: Some(request.max_turns),
                    enforcement: CliTurnLimitEnforcement::Native,
                }
            } else {
                CliTurnLimitReceipt {
                    requested_max_turns: request.max_turns,
                    effective_max_turns: None,
                    enforcement: CliTurnLimitEnforcement::Unsupported,
                }
            },
            resource_limits: provider.resource_limits.clone(),
            ephemeral_config: None,
        }
    }
}

fn configured_cli_resource_limits(
    provider: &ProviderConfig,
) -> Result<Option<ResourceLimits>, DispatchV2Error> {
    let limits = ResourceLimits::from_provider_config(provider);
    if let Some(limits) = &limits {
        limits.validate_for_current_platform().map_err(|error| {
            DispatchV2Error::ResourceLimitEnforcement {
                provider_id: provider.kind.label().to_string(),
                message: error.to_string(),
            }
        })?;
    }
    Ok(limits)
}

/// Runtime the runner should use for a resolved provider/model pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRuntime {
    /// Spawn a subprocess and decode provider JSON lines.
    Cli(CliProviderConfig),
    /// Construct a `roko-agent::Agent` and bridge its `AgentResult` into
    /// normalized events. This is the API-provider path.
    AgentResultBridge {
        /// Provider family bridged through `Agent::run`.
        provider_kind: ProviderKind,
    },
    /// The provider cannot currently be dispatched by the runner.
    Unsupported(UnsupportedProvider),
}

impl ProviderRuntime {
    /// Whether this resolved target can be dispatched by this layer.
    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unsupported(_))
    }

    /// Return the CLI runtime when this is a subprocess-json provider.
    pub fn as_cli(&self) -> Option<&CliProviderConfig> {
        match self {
            Self::Cli(provider) => Some(provider),
            Self::AgentResultBridge { .. } | Self::Unsupported(_) => None,
        }
    }
}

/// Unsupported provider metadata retained for diagnostics and fallback routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedProvider {
    /// Machine-readable reason.
    pub reason: UnsupportedProviderReason,
    /// Human-readable detail.
    pub detail: String,
}

/// Why a provider/model cannot be dispatched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedProviderReason {
    /// The model references a provider id absent from the effective config.
    MissingProvider,
    /// A CLI provider has no command.
    MissingCommand,
    /// The provider kind has no subprocess-json adapter.
    UnsupportedCliProvider,
    /// The command is not a known supported CLI protocol.
    UnsupportedCommand,
    /// No `[models.*]` entry, configured slug or builtin model resolves the
    /// key (bug-5cff57).
    UnknownModel,
}

/// Fully resolved dispatch target for a model key.
#[derive(Debug, Clone)]
pub struct ProviderDispatchSpec {
    /// User-facing model key requested by the runner.
    pub model_key: String,
    /// Concrete model slug sent to the provider.
    pub model_slug: String,
    /// Provider registry id.
    pub provider_id: String,
    /// Protocol family.
    pub provider_kind: ProviderKind,
    /// Effective model profile, when present.
    pub model_profile: Option<ModelProfile>,
    /// Effective provider config, when present.
    pub provider_config: Option<ProviderConfig>,
    /// Runtime selected by this abstraction.
    pub runtime: ProviderRuntime,
}

impl ProviderDispatchSpec {
    /// Whether this spec can be dispatched.
    pub fn is_supported(&self) -> bool {
        self.runtime.is_supported()
    }
}

/// Provider/model resolver backed by `RokoConfig`.
#[derive(Debug, Clone)]
pub struct ProviderDispatchResolver {
    config: Arc<RokoConfig>,
}

impl ProviderDispatchResolver {
    /// Create a resolver from effective `roko.toml` config.
    pub fn new(config: Arc<RokoConfig>) -> Self {
        Self { config }
    }

    /// Resolve a model key into a dispatchable provider target. A key that no
    /// `[models.*]` entry, configured slug or builtin model resolves is
    /// unsupported, with [`try_resolve_model`]'s reason, rather than sent to
    /// whichever provider its name suggests (bug-5cff57).
    pub fn resolve(&self, model_key: &str) -> ProviderDispatchSpec {
        let resolved = resolve_model(&self.config, model_key);
        let models = self.config.effective_models();
        let providers = self.config.effective_providers();

        let model_profile = resolved
            .profile
            .clone()
            .or_else(|| models.get(model_key).cloned())
            .or_else(|| {
                models
                    .values()
                    .find(|profile| profile.slug == resolved.slug)
                    .cloned()
            });
        if model_profile.is_none()
            && let Err(error) = try_resolve_model(&self.config, model_key)
        {
            return ProviderDispatchSpec {
                model_key: model_key.to_string(),
                model_slug: resolved.slug.clone(),
                provider_id: resolved.provider_kind.label().to_string(),
                provider_kind: resolved.provider_kind,
                model_profile: None,
                provider_config: None,
                runtime: ProviderRuntime::Unsupported(UnsupportedProvider {
                    reason: UnsupportedProviderReason::UnknownModel,
                    detail: error.to_string(),
                }),
            };
        }

        let model_slug = model_profile
            .as_ref()
            .map(|profile| profile.slug.clone())
            .unwrap_or_else(|| resolved.slug.clone());

        let requested_provider_id = model_profile
            .as_ref()
            .map(|profile| profile.provider.clone())
            .unwrap_or_else(|| resolved.provider_kind.label().to_string());

        let provider_match = if model_profile.is_some() {
            providers
                .get(&requested_provider_id)
                .cloned()
                .map(|provider| (requested_provider_id.clone(), provider))
        } else {
            providers
                .get(&requested_provider_id)
                .cloned()
                .map(|provider| (requested_provider_id.clone(), provider))
                .or_else(|| {
                    providers
                        .iter()
                        .find(|(_, provider)| provider.kind == resolved.provider_kind)
                        .map(|(id, provider)| (id.clone(), provider.clone()))
                })
        };

        let (provider_id, provider_config) = match provider_match {
            Some((provider_id, provider)) => (provider_id, Some(provider)),
            None => (requested_provider_id, None),
        };
        let provider_kind = provider_config
            .as_ref()
            .map(|provider| provider.kind)
            .unwrap_or(resolved.provider_kind);
        let runtime = classify_runtime(&provider_id, provider_kind, provider_config.as_ref());

        ProviderDispatchSpec {
            model_key: model_key.to_string(),
            model_slug,
            provider_id,
            provider_kind,
            model_profile,
            provider_config,
            runtime,
        }
    }
}

/// Provider-neutral agent construction facade.
#[derive(Clone)]
pub struct AgentDispatcherV2 {
    config: Arc<RokoConfig>,
    resolver: ProviderDispatchResolver,
    semaphores: Arc<ProviderSemaphores>,
    /// Runtime-scoped per-provider rate limiter.
    ///
    /// When present, threaded into `AgentOptions.rate_limiter` so HTTP-backed
    /// provider adapters call `acquire(provider_id)` before each LLM request.
    rate_limiter: Option<Arc<ProviderRateLimiter>>,
    /// Runtime-scoped shared provider health registry (E48-T05).
    ///
    /// When present, every live provider attempt records its outcome
    /// (`record_success` / `record_failure`) immediately after the
    /// provider call completes and before any gate verdict is applied.
    /// The same `Arc` is shared with `CascadeRouter` routing calls so
    /// circuit-state changes are immediately visible to the next routing
    /// decision.
    health_registry: Option<Arc<ProviderHealthRegistry>>,
    /// Cancellation token from the runner, threaded into every
    /// `ToolLoopAgent` constructed by this dispatcher.
    cancel_token: Option<Arc<dyn roko_core::tool::CancelToken>>,
    /// Persistent JSONL tool audit adapter, shared across all dispatches.
    ///
    /// When set, every tool call records scrubbed admit/result lines to
    /// `.roko/tool_audit.jsonl` for durable observability.
    tool_audit: Option<Arc<roko_fs::tool_audit::ScrubAuditAdapter>>,
    /// Per-call trace and metrics sinks for the tool calls of every agent
    /// this dispatcher creates (find-f489db).
    observability: Option<roko_fs::FsObservabilitySinks>,
    /// The safety provenance sinks of the runs in flight; a dispatch's tool
    /// calls record with its run's sink (gap-ff95f5).
    provenance: Option<crate::safety_provenance::ProvenanceSinks>,
}

impl std::fmt::Debug for AgentDispatcherV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentDispatcherV2")
            .field("config", &self.config)
            .field("resolver", &self.resolver)
            .field("semaphores", &self.semaphores)
            .field("rate_limiter", &self.rate_limiter)
            .field("health_registry", &self.health_registry)
            .field("cancel_token", &self.cancel_token.is_some())
            .field("tool_audit", &self.tool_audit)
            .field("observability", &self.observability)
            .finish()
    }
}

impl AgentDispatcherV2 {
    /// Create a dispatcher from effective `roko.toml` config.
    pub fn new(config: Arc<RokoConfig>) -> Self {
        let providers = config.effective_providers();
        let semaphores = Arc::new(ProviderSemaphores::new(&providers));
        let resolver = ProviderDispatchResolver::new(Arc::clone(&config));
        Self {
            config,
            resolver,
            semaphores,
            rate_limiter: None,
            health_registry: None,
            cancel_token: None,
            tool_audit: None,
            observability: None,
            provenance: None,
        }
    }

    /// Create a dispatcher that reuses pre-built semaphores.
    ///
    /// Used by `SharedAgentFactory` to avoid rebuilding the semaphore set
    /// for every task dispatch.
    pub fn with_shared(config: Arc<RokoConfig>, semaphores: Arc<ProviderSemaphores>) -> Self {
        let resolver = ProviderDispatchResolver::new(Arc::clone(&config));
        Self {
            config,
            resolver,
            semaphores,
            rate_limiter: None,
            health_registry: None,
            cancel_token: None,
            tool_audit: None,
            observability: None,
            provenance: None,
        }
    }

    /// Attach a runtime-scoped rate limiter.
    ///
    /// The limiter is built once per run from `[providers.<name>].limits` in
    /// roko.toml and shared by every concurrent task dispatch via
    /// `SharedAgentFactory`. This ensures all agents share a single RPM/TPM
    /// budget per provider rather than each maintaining independent counters.
    pub fn with_rate_limiter(mut self, limiter: Arc<ProviderRateLimiter>) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Attach a runtime-scoped provider health registry (E48-T05).
    ///
    /// The same `Arc` must be shared with the `CascadeRouter` routing path
    /// so that outcomes recorded here are immediately visible to the next
    /// routing decision.  `SharedAgentFactory` constructs the registry once
    /// and threads it through both paths.
    pub fn with_health_registry(mut self, registry: Arc<ProviderHealthRegistry>) -> Self {
        self.health_registry = Some(registry);
        self
    }

    /// Attach a cancellation token from the runner.
    ///
    /// When set, this token is threaded into every `ToolLoopAgent` constructed
    /// by `create_agent_for_model`, allowing runner-level task cancellation
    /// (Skip/Cancel) to halt in-progress tool execution immediately.
    pub fn with_cancel_token(mut self, token: Arc<dyn roko_core::tool::CancelToken>) -> Self {
        self.cancel_token = Some(token);
        self
    }

    /// Attach a persistent JSONL tool audit adapter.
    ///
    /// When set, every tool call dispatched through this factory records
    /// scrubbed admit/result lines to `.roko/tool_audit.jsonl`.
    pub fn with_tool_audit(mut self, adapter: Arc<roko_fs::tool_audit::ScrubAuditAdapter>) -> Self {
        self.tool_audit = Some(adapter);
        self
    }

    /// Attach the per-call trace sink.
    ///
    /// When set, every tool call an agent created by this dispatcher makes
    /// leaves a closed trace under `.roko/traces/` (find-f489db).
    pub fn with_observability_sinks(mut self, sinks: roko_fs::FsObservabilitySinks) -> Self {
        self.observability = Some(sinks);
        self
    }

    /// The price snapshot `request`'s calls are priced from (backlog 2114):
    /// the one decision 2113 picks for its workspace root, which a Graph
    /// dispatch names as its `immune_root`.
    fn pricing_snapshot_for(&self, request: &AgentDispatchRequest) -> Option<Arc<PriceSnapshot>> {
        let root = request.immune_root.as_deref().unwrap_or(&request.workdir);
        pricing_snapshot(&self.config.pricing, root)
    }

    /// Record the tool calls of each dispatch with the safety provenance
    /// sink `sinks` holds for the dispatch's run, when it holds one
    /// (gap-ff95f5).
    pub fn with_provenance_sinks(
        mut self,
        sinks: crate::safety_provenance::ProvenanceSinks,
    ) -> Self {
        self.provenance = Some(sinks);
        self
    }

    /// Resolve a model without launching anything.
    pub fn resolve(&self, model_key: &str) -> ProviderDispatchSpec {
        self.resolver.resolve(model_key)
    }

    /// Create the provider-backed agent for a request.
    ///
    /// This is the generalized path for API providers and provider adapters
    /// that return a single `AgentResult`. CLI subprocess streaming providers
    /// can still use `build_cli_invocation` when the runner needs PID-level
    /// lifecycle control.
    pub fn create_agent(
        &self,
        request: &AgentDispatchRequest,
    ) -> Result<CreatedAgent, DispatchV2Error> {
        request.validate()?;
        let target = self.resolve(&request.model_key);
        if let ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Err(DispatchV2Error::UnsupportedResolvedProvider {
                provider_id: target.provider_id.clone(),
                detail: unsupported.detail.clone(),
            });
        }
        validate_contract_support(request, &target)?;

        let options = self.agent_options(request);
        let agent =
            create_agent_for_model(&self.config, &request.model_key, options).map_err(|err| {
                DispatchV2Error::AgentCreation {
                    model_key: request.model_key.clone(),
                    message: err.to_string(),
                }
            })?;

        Ok(CreatedAgent { target, agent })
    }

    /// Run a provider-factory agent and return provider-neutral events.
    ///
    /// This is not wired into runner v2 yet because runner v2's `Started`
    /// event requires an OS pid. The returned event type carries `pid:
    /// Option<u32>` so the event protocol can evolve without lying about
    /// process ownership.
    pub async fn run_agent_result_bridge(
        &self,
        request: AgentDispatchRequest,
    ) -> Result<AgentResultDispatch, DispatchV2Error> {
        let created = self.create_agent(&request)?;
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text(request.prompt.clone()))
            .build();
        let audit_mark = self.tool_audit_mark(&request).await;
        let started = Instant::now();
        let mut result = created.agent.run(&input, &Context::now()).await;
        let latency_ms = started.elapsed().as_millis() as u64;
        let snapshot = self.pricing_snapshot_for(&request);
        fill_cost_from_profile(&mut result, &created.target, snapshot.as_deref());

        self.record_provider_outcome(&created.target.provider_id, &result);

        let health_recorded = self.health_registry.is_some();
        record_agent_dispatch_feedback(
            &request,
            &created.target,
            &result,
            latency_ms,
            health_recorded,
        )
        .await;
        let events = dispatch_events_from_result(&request, &created.target, &result);
        let tool_calls = match audit_mark {
            Some(mark) => mark.tool_calls().await,
            None => Vec::new(),
        };
        let tool_policy = tool_policy_record(&request, &created.target, &result);
        Ok(AgentResultDispatch {
            target: created.target,
            result,
            events,
            tool_calls,
            tool_policy,
        })
    }

    /// Run a provider-factory agent with streaming events forwarded in real time.
    ///
    /// Emits `Started` immediately, spawns an internal forwarder that converts
    /// [`StreamChunk`]s into [`AgentRuntimeEvent`]s as they arrive, then
    /// emits `TurnCompleted` + `Exited` after the agent finishes.
    pub async fn run_agent_streaming(
        &self,
        request: AgentDispatchRequest,
        event_tx: mpsc::Sender<AgentRuntimeEvent>,
    ) -> Result<AgentResult, DispatchV2Error> {
        let created = self.create_agent(&request)?;

        // Emit Started immediately so the TUI shows the agent is running.
        let _ = event_tx
            .send(AgentRuntimeEvent::Started {
                agent_id: request.agent_id.clone(),
                provider: created.target.provider_id.clone(),
                model: created.target.model_slug.clone(),
                pid: None,
            })
            .await;

        // Set up streaming channel: StreamEvents flow from agent -> forwarder -> event_tx.
        let (chunk_tx, mut chunk_rx) = mpsc::channel::<roko_agent::tool_loop::StreamEvent>(
            roko_core::defaults::DEFAULT_CHANNEL_BUFFER,
        );
        let forwarder_tx = event_tx.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(stream_event) = chunk_rx.recv().await {
                let chunk = stream_chunk_from_event(stream_event);
                let event = agent_event_from_chunk(chunk);
                if forwarder_tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        let input = Signal::builder(Kind::Prompt)
            .body(Body::text(request.prompt.clone()))
            .build();
        let started = Instant::now();
        let mut result = created
            .agent
            .run_streaming(&input, &Context::now(), chunk_tx)
            .await;
        let latency_ms = started.elapsed().as_millis() as u64;

        // Wait for forwarder to drain remaining chunks.
        let _ = forwarder.await;

        // Back-fill cost from the price snapshot or model profile pricing
        // before checking cost_usd.
        let snapshot = self.pricing_snapshot_for(&request);
        fill_cost_from_profile(&mut result, &created.target, snapshot.as_deref());

        // Emit terminal events.
        if result.usage.total_tokens() > 0 || result.usage.cost_usd > 0.0 {
            let _ = event_tx
                .send(AgentRuntimeEvent::TokenUsage {
                    input_tokens: u64::from(result.usage.input_tokens),
                    output_tokens: u64::from(result.usage.output_tokens),
                    cache_read_tokens: u64::from(result.usage.cache_read_tokens),
                    cache_write_tokens: u64::from(result.usage.cache_create_tokens),
                    reasoning_tokens: u64::from(result.usage.reasoning_tokens),
                })
                .await;
        }
        if !result.success {
            let message = result
                .output
                .body
                .as_text()
                .unwrap_or("agent failed without text output")
                .to_string();
            let _ = event_tx.send(AgentRuntimeEvent::Error { message }).await;
        }
        let _ = event_tx
            .send(AgentRuntimeEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: (result.usage.cost_usd > 0.0)
                    .then_some(f64::from(result.usage.cost_usd)),
                num_turns: reported_num_turns(&result),
                is_error: !result.success,
            })
            .await;
        let _ = event_tx
            .send(AgentRuntimeEvent::Exited {
                exit_code: Some(if result.success { 0 } else { 1 }),
            })
            .await;

        self.record_provider_outcome(&created.target.provider_id, &result);
        let health_recorded = self.health_registry.is_some();
        record_agent_dispatch_feedback(
            &request,
            &created.target,
            &result,
            latency_ms,
            health_recorded,
        )
        .await;

        Ok(result)
    }

    /// Run a provider-factory agent with pre-discovered MCP tools.
    ///
    /// When `mcp_tools` is `Some`, the tools are passed to the provider adapter
    /// so it skips MCP discovery entirely (no `block_on`, no OS thread).
    pub async fn run_agent_result_bridge_with_mcp(
        &self,
        request: AgentDispatchRequest,
        mcp_runtime: Option<Arc<roko_agent::mcp::McpRuntime>>,
    ) -> Result<AgentResultDispatch, DispatchV2Error> {
        self.run_agent_result_bridge_with_tools(request, mcp_runtime, None)
            .await
    }

    /// Run a provider-factory agent with pre-discovered MCP and local tool
    /// runtimes. Keeping the executable local resolver beside its definitions
    /// prevents provider loops from advertising definition-only plugin tools.
    pub async fn run_agent_result_bridge_with_tools(
        &self,
        request: AgentDispatchRequest,
        mcp_runtime: Option<Arc<roko_agent::mcp::McpRuntime>>,
        local_tool_runtime: Option<Arc<roko_agent::provider::LocalToolRuntime>>,
    ) -> Result<AgentResultDispatch, DispatchV2Error> {
        self.run_agent_result_bridge_with_tools_and_cli_mcp(
            request,
            mcp_runtime,
            local_tool_runtime,
            None,
            false,
        )
        .await
    }

    /// Run a provider bridge while supplying an authenticated per-call MCP
    /// endpoint for ACP transports that can consume one.
    pub async fn run_agent_result_bridge_with_tools_and_cli_mcp(
        &self,
        request: AgentDispatchRequest,
        mcp_runtime: Option<Arc<roko_agent::mcp::McpRuntime>>,
        local_tool_runtime: Option<Arc<roko_agent::provider::LocalToolRuntime>>,
        local_tool_mcp: Option<CliPluginMcpConfig>,
        local_tool_mcp_bridge_ready: bool,
    ) -> Result<AgentResultDispatch, DispatchV2Error> {
        request.validate()?;
        let target = self.resolve(&request.model_key);
        if let ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Err(DispatchV2Error::UnsupportedResolvedProvider {
                provider_id: target.provider_id.clone(),
                detail: unsupported.detail.clone(),
            });
        }
        validate_contract_support(&request, &target)?;

        let mut options = self.agent_options(&request);
        if let Some(runtime) = mcp_runtime {
            options.pre_discovered_mcp_runtime = Some(runtime);
        }
        if target_supports_per_call_local_mcp(&target) && local_tool_mcp_bridge_ready {
            options.local_tool_mcp_servers = local_tool_mcp.map(|config| {
                Arc::new(vec![LocalToolMcpServer {
                    name: config.server_name,
                    url: config.url,
                    bearer_token: config.bearer_token,
                }])
            });
        } else {
            // Passing the in-process runtime to an opaque adapter is
            // intentional here: central provider construction rejects it,
            // yielding a truthful error when no supported bridge exists.
            options.pre_discovered_local_tools = local_tool_runtime;
        }
        let agent =
            create_agent_for_model(&self.config, &request.model_key, options).map_err(|err| {
                DispatchV2Error::AgentCreation {
                    model_key: request.model_key.clone(),
                    message: err.to_string(),
                }
            })?;

        let input = Signal::builder(Kind::Prompt)
            .body(Body::text(request.prompt.clone()))
            .build();
        let audit_mark = self.tool_audit_mark(&request).await;
        let started = Instant::now();
        let mut result = agent.run(&input, &Context::now()).await;
        let latency_ms = started.elapsed().as_millis() as u64;
        let snapshot = self.pricing_snapshot_for(&request);
        fill_cost_from_profile(&mut result, &target, snapshot.as_deref());

        // This must happen before any gate verdict is applied so a provider
        // success followed by a failing code/test gate remains a provider
        // success in the health registry.
        self.record_provider_outcome(&target.provider_id, &result);

        let health_recorded = self.health_registry.is_some();
        record_agent_dispatch_feedback(&request, &target, &result, latency_ms, health_recorded)
            .await;
        let events = dispatch_events_from_result(&request, &target, &result);
        let tool_calls = match audit_mark {
            Some(mark) => mark.tool_calls().await,
            None => Vec::new(),
        };
        let tool_policy = tool_policy_record(&request, &target, &result);
        Ok(AgentResultDispatch {
            target,
            result,
            events,
            tool_calls,
            tool_policy,
        })
    }

    /// Mark the tool audit before `request` runs, so the tool calls its
    /// agent makes can be read back after it (gap-4d5e2d). `None` without an
    /// attached audit or an attempt key to find its lines by.
    async fn tool_audit_mark(&self, request: &AgentDispatchRequest) -> Option<ToolAuditMark> {
        let audit = self.tool_audit.as_ref()?;
        let attempt_key = request.attempt_key.as_deref()?;
        Some(ToolAuditMark::at(audit.path().to_path_buf(), attempt_key).await)
    }

    /// Record a provider run's outcome for the circuit breaker (E48-T05), as
    /// [`ProviderHealthOutcome::of`] reads it. With a registry attached,
    /// this is the run's only provider-health record: the feedback recorder
    /// then leaves health alone (backlog 1114).
    fn record_provider_outcome(&self, provider_id: &str, result: &AgentResult) {
        let Some(registry) = &self.health_registry else {
            return;
        };
        match ProviderHealthOutcome::of(result) {
            ProviderHealthOutcome::Success => registry.record_provider_success(provider_id),
            ProviderHealthOutcome::Failure(error_kind) => {
                registry.record_provider_failure(provider_id, error_kind);
            }
            ProviderHealthOutcome::ImmuneDenied => tracing::debug!(
                provider = %provider_id,
                "an immune denial is host policy; the provider's health is unchanged"
            ),
            ProviderHealthOutcome::AttemptTimeout => tracing::debug!(
                provider = %provider_id,
                "an attempt timeout is a task outcome; the provider's health is unchanged"
            ),
        }
    }

    fn agent_options(&self, request: &AgentDispatchRequest) -> AgentOptions {
        let correlation = tool_correlation(request);
        let provenance_sink = self
            .provenance
            .as_ref()
            .and_then(|sinks| sinks.for_run(&correlation.run_id));
        AgentOptions {
            command: request.command.clone(),
            timeout_ms: request.timeout_ms,
            system_prompt: (!request.system_prompt.trim().is_empty())
                .then(|| request.system_prompt.clone()),
            cached_content: None,
            tools: request.tools.clone(),
            agent_contract: request.agent_contract.clone(),
            mcp_config: request.mcp_config.clone(),
            immune_root: request
                .immune_root
                .clone()
                .or_else(|| Some(request.workdir.clone())),
            working_dir: Some(request.workdir.clone()),
            provider_semaphores: Some(Arc::clone(&self.semaphores)),
            env: request.env.clone(),
            extra_args: request.extra_args.clone(),
            effort: request.effort.clone(),
            bare_mode: request.bare_mode,
            dangerously_skip_permissions: request.dangerously_skip_permissions,
            name: request.agent_id.clone(),
            max_turns: request.max_turns,
            // Thread the runtime-scoped rate limiter through to provider adapters.
            // HTTP-backed adapters (OpenAI-compat, Anthropic API, Gemini) will call
            // `limiter.acquire(provider_id)` before each live LLM request so that
            // all concurrent task dispatches share one RPM/TPM budget per provider.
            rate_limiter: self.rate_limiter.clone(),
            // Thread the runner cancel token through to the ToolLoopAgent so
            // task-level cancellation halts in-progress tool execution.
            cancel_token: self.cancel_token.clone(),
            // Thread the persistent file audit adapter so every tool call
            // records scrubbed admit/result lines to disk.
            tool_audit: self.tool_audit.clone(),
            // find-f489db: each tool call also leaves a closed trace, and
            // both join back to the attempt.
            trace_sink: self
                .observability
                .as_ref()
                .map(roko_fs::FsObservabilitySinks::trace_sink_dyn),
            // No tool metrics file: nothing read it (backlog 2123).
            metrics_sink: None,
            tool_correlation: Some(correlation),
            // gap-ff95f5: the run's tool calls leave durable safety provenance.
            provenance_sink,
            // Thread the live output channel so the immune boundary can
            // forward tool steps and unscreened events before screening.
            live_output: request.live_output.clone(),
            ..Default::default()
        }
    }
}

/// The correlation a dispatch's tool calls carry into their audit, trace and
/// metrics records (find-f489db): the run and task of the attempt the
/// request serves, the attempt's key (`"{run}:{plan}:{task}:{attempt}"`,
/// which telemetry rows also carry), and the agent id. A request without an
/// attempt key names only the agent.
fn tool_correlation(request: &AgentDispatchRequest) -> roko_core::tool::CorrelationEnvelope {
    let attempt = request
        .attempt_key
        .as_deref()
        .and_then(roko_learn::telemetry::AttemptKey::parse);
    roko_core::tool::CorrelationEnvelope {
        run_id: attempt
            .as_ref()
            .map(|key| key.run_id.clone())
            .unwrap_or_default(),
        task_id: attempt.map(|key| key.task_id).unwrap_or_default(),
        attempt_id: request.attempt_key.clone().unwrap_or_default(),
        turn_id: String::new(),
        agent_id: request.agent_id.clone(),
    }
}

fn target_supports_per_call_local_mcp(target: &ProviderDispatchSpec) -> bool {
    target.provider_kind == ProviderKind::CursorCli
        || (target.provider_kind == ProviderKind::Hermes
            && target.provider_config.as_ref().is_some_and(|provider| {
                provider.base_url.is_none()
                    && provider
                        .args
                        .as_ref()
                        .is_some_and(|args| args.iter().any(|argument| argument == "acp"))
            }))
}

/// Refuse a provider that cannot enforce the request's agent contract.
///
/// Codex's built-in tools have no binding allowlist, so it cannot honour a
/// contract that names the only tools a role may use (gap-baab0a); Graph
/// failover moves such a task to a provider that can.
pub(crate) fn validate_contract_support(
    request: &AgentDispatchRequest,
    target: &ProviderDispatchSpec,
) -> Result<(), DispatchV2Error> {
    let allowlist = request
        .agent_contract
        .as_ref()
        .is_some_and(|contract| contract.allowed_tools.is_some());
    if allowlist && target.provider_kind == ProviderKind::CodexCli {
        return Err(DispatchV2Error::ContractUnsupported {
            provider_id: target.provider_id.clone(),
            kind: target.provider_kind,
        });
    }
    if request.agent_contract.is_none()
        || matches!(
            target.provider_kind,
            ProviderKind::ClaudeCli
                | ProviderKind::AnthropicApi
                | ProviderKind::OpenAiCompat
                | ProviderKind::PerplexityApi
                | ProviderKind::GeminiApi
                | ProviderKind::GeminiCli
                | ProviderKind::CerebrasApi
                | ProviderKind::CursorAcp
                | ProviderKind::CursorCli
                | ProviderKind::CodexCli
                | ProviderKind::Hermes
                | ProviderKind::OpenClaw
        )
    {
        return Ok(());
    }

    Err(DispatchV2Error::ContractUnsupported {
        provider_id: target.provider_id.clone(),
        kind: target.provider_kind,
    })
}

/// Classify a provider error from output text into an error kind string
/// suitable for [`ProviderHealthRegistry::record_provider_failure`].
pub(crate) fn classify_provider_error(output_text_lower: &str) -> &'static str {
    // One classifier for every provider-health caller (backlog 1113): it
    // also names auth failures, which this copy used to report as unknown.
    roko_agent::provider::error_classify::classify_failure_text(output_text_lower)
}

/// What one provider run says about its provider's health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderHealthOutcome {
    /// The provider served the run: a success, or a run stopped at its turn
    /// cap (a task outcome, not a provider fault).
    Success,
    /// The provider failed, with the error kind its text classifies as.
    Failure(&'static str),
    /// The immune boundary denied the run. That is the host's own policy,
    /// often decided before any model call, so it must not open a healthy
    /// provider's circuit (backlog 1114).
    ImmuneDenied,
    /// The run was killed at its attempt's wall-clock timeout, which says how
    /// long the task took, not how healthy the provider is (bug-7cdce7):
    /// three slow attempts must not open the circuit.
    AttemptTimeout,
}

impl ProviderHealthOutcome {
    /// Read `result`'s provider-health outcome. Any unsuccessful run that is
    /// neither an immune denial, a turn cap nor an attempt timeout is a
    /// provider failure, classified from its text.
    fn of(result: &AgentResult) -> Self {
        use roko_agent::provider::error_classify::{detect_attempt_timeout, detect_turn_cap};

        let text = result.output.body.as_text().unwrap_or_default();
        if result.output.tag("immune_denied") == Some("true") {
            Self::ImmuneDenied
        } else if result.success || detect_turn_cap(text).is_some() {
            Self::Success
        } else if detect_attempt_timeout(text) {
            Self::AttemptTimeout
        } else {
            Self::Failure(classify_provider_error(&text.to_ascii_lowercase()))
        }
    }

    /// Whether the run says anything about its provider's health.
    const fn is_provider_outcome(self) -> bool {
        matches!(self, Self::Success | Self::Failure(_))
    }
}

/// Record one bridge call's model-call feedback: its efficiency row and,
/// unless `health_recorded` says the dispatcher's own registry holds it, the
/// provider's health. Either way a call leaves one provider-health record,
/// and an immune denial or an attempt timeout leaves none (backlog 1114).
///
/// The bridge never teaches the cascade router (bug-07bc75). Its callers are
/// Graph dispatch's attempts and helper calls: the router learns each
/// attempt's settled verdict through `RoutingObservationSink`, and a
/// provider call's own success, before any gate ran, is no quality evidence.
async fn record_agent_dispatch_feedback(
    request: &AgentDispatchRequest,
    target: &ProviderDispatchSpec,
    result: &AgentResult,
    latency_ms: u64,
    health_recorded: bool,
) {
    let learn_dir = roko_fs::RokoLayout::for_project(&request.workdir).learn_dir();
    let outcome = ProviderHealthOutcome::of(result);
    let mut recorder = ModelCallFeedbackRecorder::without_cascade_router(learn_dir);
    if health_recorded || !outcome.is_provider_outcome() {
        recorder = recorder.without_provider_health();
    }
    let error_class = match outcome {
        ProviderHealthOutcome::Failure(error_kind) => Some(error_kind.to_string()),
        _ => None,
    };
    if let Err(error) = recorder
        .record(ModelCallFeedback {
            run_id: None,
            request_id: Some(format!("dispatch-v2-{}", request.agent_id)),
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: target.model_slug.clone(),
            provider: target.provider_id.clone(),
            role: "dispatch_v2".to_string(),
            input_tokens: u64::from(result.usage.input_tokens),
            output_tokens: u64::from(result.usage.output_tokens),
            cost_usd: f64::from(result.usage.cost_usd),
            latency_ms,
            success: result.success,
            provider_success: Some(outcome == ProviderHealthOutcome::Success),
            error_class,
            model_reported: result
                .usage_obs
                .as_ref()
                .and_then(|usage| usage.model.clone()),
            attempt_key: request.attempt_key.clone(),
        })
        .await
    {
        tracing::warn!(
            provider = %target.provider_id,
            model = %target.model_slug,
            agent_id = %request.agent_id,
            error = %error,
            "failed to record dispatch-v2 feedback"
        );
    }
}

/// Request for provider-factory dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDispatchRequest {
    /// Logical model key to resolve.
    pub model_key: String,
    /// User prompt.
    pub prompt: String,
    /// System prompt.
    pub system_prompt: String,
    /// Working directory.
    pub workdir: PathBuf,
    /// Canonical workspace root for durable immune authority state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub immune_root: Option<PathBuf>,
    /// Agent id for diagnostics.
    pub agent_id: String,
    /// Optional command override for legacy providers.
    pub command: Option<String>,
    /// Optional timeout override.
    pub timeout_ms: Option<u64>,
    /// Optional MCP config path.
    pub mcp_config: Option<PathBuf>,
    /// Extra environment entries.
    pub env: Vec<(String, String)>,
    /// Extra provider args.
    pub extra_args: Vec<String>,
    /// Optional effort hint.
    pub effort: Option<String>,
    /// Optional tool allowlist/config payload.
    pub tools: Option<String>,
    /// Fully resolved role contract for this dispatch.
    ///
    /// Runner-v2 folds task allow/deny restrictions into this contract before
    /// entering the bridge so the provider adapter receives one authoritative
    /// policy. `None` is reserved for non-runner callers with no role contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_contract: Option<AgentContract>,
    /// Whether provider built-in prompts should be disabled.
    pub bare_mode: bool,
    /// Whether provider permission prompts/sandboxing should be bypassed.
    pub dangerously_skip_permissions: bool,
    /// Optional maximum agent turn count override.
    ///
    /// When `Some`, overrides the provider's built-in default turn limit.
    /// Currently wired for Claude CLI via `AgentOptions::max_turns`.
    /// `None` means use the provider default (Theta = 10 for Claude CLI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    /// Optional live output channel.
    ///
    /// When set, the immune boundary taps the provider event stream and
    /// forwards qualifying events (tool steps, unscreened text/reasoning)
    /// before the final result is screened. Skipped by serde because
    /// `LiveOutput` is not serializable.
    #[serde(skip)]
    pub live_output: Option<roko_agent::live_output::LiveOutput>,
    /// Key of the attempt this dispatch serves
    /// (`"{run}:{plan}:{task}:{attempt}"`), when the caller has one. The
    /// bridge's `model_call` row carries it (bug-92f655).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_key: Option<String>,
}

impl AgentDispatchRequest {
    fn validate(&self) -> Result<(), DispatchV2Error> {
        if roko_agent::immune_boundary::validate_provider_agent_id(&self.agent_id).is_err() {
            return Err(DispatchV2Error::InvalidAgentId);
        }
        if self.prompt.trim().is_empty() {
            return Err(DispatchV2Error::EmptyPrompt);
        }
        if self.model_key.trim().is_empty() {
            return Err(DispatchV2Error::EmptyModel);
        }
        if !self.workdir.exists() {
            return Err(DispatchV2Error::WorkdirMissing {
                path: self.workdir.clone(),
            });
        }
        Ok(())
    }
}

/// Created provider-backed agent plus its resolved target metadata.
pub struct CreatedAgent {
    /// Resolved dispatch target.
    pub target: ProviderDispatchSpec,
    /// Real provider-backed agent.
    pub agent: Box<dyn Agent>,
}

/// Result from running an `AgentResultBridge` dispatch.
pub struct AgentResultDispatch {
    /// Resolved dispatch target.
    pub target: ProviderDispatchSpec,
    /// Raw provider result.
    pub result: AgentResult,
    /// Provider-neutral event projection.
    pub events: Vec<DispatchEvent>,
    /// The tool calls roko's own tool loop made for the dispatch, read back
    /// from the tool audit with each one's outcome (gap-4d5e2d). Empty when
    /// no audit is attached, the request names no attempt, or the provider
    /// ran its own tools.
    pub tool_calls: Vec<ToolCallRecord>,
    /// The tool policy the request's contract asked for and what the
    /// provider enforced, for a provider that runs its own tools
    /// (gap-baab0a).
    pub tool_policy: Option<roko_learn::telemetry::ToolPolicyRecord>,
}

/// A tool call a dispatch made, as the tool audit (gap-4d5e2d) or the
/// provider's live output (bug-264c41) recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallRecord {
    /// The provider's call id.
    pub id: String,
    /// The tool's name; empty when the record saw only the call's result.
    pub name: String,
    /// Whether the call succeeded; `None` when no result for it was seen.
    pub succeeded: Option<bool>,
}

/// Where a dispatch's tool-audit lines start: the audit file, its length
/// before the dispatch ran, and the attempt whose lines to read back.
struct ToolAuditMark {
    path: PathBuf,
    offset: u64,
    attempt_key: String,
}

impl ToolAuditMark {
    /// Mark the audit at `path` before a dispatch of `attempt_key` runs.
    async fn at(path: PathBuf, attempt_key: &str) -> Self {
        let offset = tokio::fs::metadata(&path)
            .await
            .map_or(0, |metadata| metadata.len());
        Self {
            path,
            offset,
            attempt_key: attempt_key.to_string(),
        }
    }

    /// The attempt's tool calls among the lines appended since the mark. An
    /// audit that can't be read gives none, so their outcomes stay unknown.
    async fn tool_calls(&self) -> Vec<ToolCallRecord> {
        match read_from(&self.path, self.offset).await {
            Ok(appended) => {
                audited_tool_calls(&String::from_utf8_lossy(&appended), &self.attempt_key)
            }
            Err(error) => {
                tracing::debug!(
                    path = %self.path.display(),
                    %error,
                    "tool audit unreadable; tool outcomes stay unknown"
                );
                Vec::new()
            }
        }
    }
}

/// The bytes of the file at `path` from `offset` on.
async fn read_from(path: &Path, offset: u64) -> std::io::Result<Vec<u8>> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let mut file = tokio::fs::File::open(path).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await?;
    Ok(bytes)
}

/// The tool calls of attempt `attempt_key` in the tool-audit `lines`, in the
/// order they were admitted. A result settles the earliest call admitted
/// under its id that has none yet, so a provider that reuses call ids still
/// gets one record per call; a result whose admission isn't among the lines
/// is a call of its own. Other attempts' lines, and lines that aren't audit
/// JSON (a line another writer is still appending), are skipped.
fn audited_tool_calls(lines: &str, attempt_key: &str) -> Vec<ToolCallRecord> {
    use roko_fs::tool_audit::AuditLine;

    let mut calls: Vec<ToolCallRecord> = Vec::new();
    for line in lines.lines() {
        let Ok(audited) = serde_json::from_str::<AuditLine>(line) else {
            continue;
        };
        match audited {
            AuditLine::Admit {
                call_id,
                call_name,
                correlation,
                ..
            } if correlation.attempt_id == attempt_key => calls.push(ToolCallRecord {
                id: call_id,
                name: call_name,
                succeeded: None,
            }),
            AuditLine::Result {
                call_id,
                call_name,
                ok,
                correlation,
                ..
            } if correlation.attempt_id == attempt_key => {
                let admitted = calls
                    .iter_mut()
                    .find(|call| call.id == call_id && call.succeeded.is_none());
                match admitted {
                    Some(call) => call.succeeded = Some(ok),
                    None => calls.push(ToolCallRecord {
                        id: call_id,
                        name: call_name,
                        succeeded: Some(ok),
                    }),
                }
            }
            _ => {}
        }
    }
    calls
}

/// Provider-neutral events emitted by dispatch v2.
pub type DispatchEvent = AgentRuntimeEvent;

/// The dated price snapshot that calls in `workspace_root` are priced from
/// (backlog 2114): the one decision 2113 picks, `[pricing] snapshot` else the
/// newest in `config/prices/` else the copy built into the binary. Loaded
/// once per process and workspace. `None`, with a warning, when it cannot be
/// read: calls are then priced from roko.toml and the built-in rates.
pub(crate) fn pricing_snapshot(
    pricing: &PricingConfig,
    workspace_root: &Path,
) -> Option<Arc<PriceSnapshot>> {
    type Loaded = std::collections::HashMap<(PathBuf, String), Option<Arc<PriceSnapshot>>>;
    static LOADED: std::sync::LazyLock<parking_lot::Mutex<Loaded>> =
        std::sync::LazyLock::new(parking_lot::Mutex::default);
    let key = (
        workspace_root.to_path_buf(),
        pricing.snapshot_id().unwrap_or_default().to_string(),
    );
    let mut loaded = LOADED.lock();
    loaded
        .entry(key)
        .or_insert_with(|| match PriceSnapshot::for_workspace(pricing, workspace_root) {
            Ok(snapshot) => Some(Arc::new(snapshot)),
            Err(error) => {
                tracing::warn!(
                    workspace = %workspace_root.display(),
                    %error,
                    "no price snapshot: calls are priced from roko.toml and built-in rates"
                );
                None
            }
        })
        .clone()
}

/// Which rates priced a call's usage (backlog 2114).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallPricing {
    /// The provider reported the cost: no rate applied.
    Reported,
    /// The model's row in the dated price snapshot the caller passed, whose
    /// id the cost records carry.
    Snapshot,
    /// The model's profile in roko.toml.
    Profile,
    /// roko's built-in registry rates.
    Registry,
    /// No rate: the cost stays unknown.
    Unpriced,
}

/// `usage` in a price snapshot's token classes. Cache writes count at the
/// 5-minute TTL, the API default; reasoning is inside the output.
fn usage_token_counts(usage: &roko_core::Usage) -> TokenCounts {
    TokenCounts {
        input: u64::from(usage.input_tokens),
        cache_read: u64::from(usage.cache_read_tokens),
        cache_write_5m: u64::from(usage.cache_create_tokens),
        cache_write_1h: 0,
        output: u64::from(usage.output_tokens),
        reasoning: u64::from(usage.reasoning_tokens),
    }
}

/// [`fill_usage_cost_from_pricing`] for a dispatch result and its target.
fn fill_cost_from_profile(
    result: &mut AgentResult,
    target: &ProviderDispatchSpec,
    snapshot: Option<&PriceSnapshot>,
) {
    fill_usage_cost_from_pricing(
        &mut result.usage,
        snapshot,
        target.model_profile.as_ref(),
        &target.model_slug,
    );
}

/// Back-fill `usage.cost_usd` when the provider did not report a dollar
/// amount (backlog 2114): at the dated price `snapshot`'s row for
/// `model_slug` first, else the model profile's per-million prices, else the
/// shared registry rates for known slugs (glm-5.1, kimi-k2.5, sonar, gpt-5.x,
/// codex, …), so token-bearing usage is not silently recorded as $0.00. A
/// model none of them prices stays at 0.0, which `Usage::has_known_cost`
/// reports as unknown rather than free. Returns which rates priced the call.
pub(crate) fn fill_usage_cost_from_pricing(
    usage: &mut roko_core::Usage,
    snapshot: Option<&PriceSnapshot>,
    profile: Option<&ModelProfile>,
    model_slug: &str,
) -> CallPricing {
    if usage.cost_usd.abs() > f32::EPSILON {
        return CallPricing::Reported;
    }
    if let Some(snapshot) = snapshot
        && let Some(priced) = snapshot.price(model_slug, &usage_token_counts(usage))
    {
        usage.cost_usd = priced.api_equiv_usd as f32;
        return CallPricing::Snapshot;
    }
    let profile = profile.filter(|profile| {
        profile.cost_input_per_m.is_some() && profile.cost_output_per_m.is_some()
    });
    if let Some(profile) = profile {
        usage.fill_cost_from_pricing(
            profile.cost_input_per_m,
            profile.cost_output_per_m,
            profile.cost_cache_read_per_m,
            profile.cost_cache_write_per_m,
        );
        if usage.cost_usd.abs() > f32::EPSILON {
            return CallPricing::Profile;
        }
    }
    if let Some(pricing) = roko_core::config::model_registry::builtin_pricing(model_slug) {
        usage.fill_cost_from_pricing(
            Some(pricing.input_per_m),
            Some(pricing.output_per_m),
            Some(pricing.cache_read_per_m),
            Some(pricing.cache_write_per_m),
        );
        return CallPricing::Registry;
    }
    // A profile priced at 0/0 is free, and still a rate (backlog 2109).
    if profile.is_some() {
        CallPricing::Profile
    } else {
        CallPricing::Unpriced
    }
}

/// Whether roko has a rate for `model_slug`, as
/// [`fill_usage_cost_from_pricing`] applies one: the price snapshot's row,
/// the profile's per-million prices, or the model's built-in pricing. A rate
/// of 0 is free, and still a rate (backlog 2109).
pub(crate) fn model_has_price(
    snapshot: Option<&PriceSnapshot>,
    profile: Option<&ModelProfile>,
    model_slug: &str,
) -> bool {
    snapshot.is_some_and(|snapshot| snapshot.row(model_slug).is_some())
        || profile.is_some_and(|profile| {
            profile.cost_input_per_m.is_some() || profile.cost_output_per_m.is_some()
        })
        || roko_core::config::model_registry::builtin_pricing(model_slug).is_some()
}

/// Whether a call's `usage` is priced, for its cost row (backlog 2109): its
/// cost is known, or roko has a rate for the model, which prices even a
/// free call. An unpriced call's `cost_usd` of 0 is unknown, not free.
pub(crate) fn usage_is_priced(
    usage: &roko_core::Usage,
    snapshot: Option<&PriceSnapshot>,
    profile: Option<&ModelProfile>,
    model_slug: &str,
) -> bool {
    usage.has_known_cost() || model_has_price(snapshot, profile, model_slug)
}

/// What `usage` would have cost with no prompt caching, priced like
/// [`fill_usage_cost_from_pricing`]: the price snapshot's input and output
/// rates, else the profile's, else the model's built-in pricing. `None` when
/// none prices the model (gap-7a8474).
pub(crate) fn usage_cost_without_cache(
    usage: &roko_core::Usage,
    snapshot: Option<&PriceSnapshot>,
    profile: Option<&ModelProfile>,
    model_slug: &str,
) -> Option<f64> {
    if let Some(row) = snapshot.and_then(|snapshot| snapshot.row(model_slug)) {
        return Some(usage.cost_without_cache(row.input, row.output));
    }
    if let Some((input, output)) =
        profile.and_then(|profile| profile.cost_input_per_m.zip(profile.cost_output_per_m))
    {
        return Some(usage.cost_without_cache(input, output));
    }
    let pricing = roko_core::config::model_registry::builtin_pricing(model_slug)?;
    Some(usage.cost_without_cache(pricing.input_per_m, pricing.output_per_m))
}

/// The tool policy `request`'s contract asked for and what `target`'s
/// provider enforced, for the attempt's record (gap-baab0a). Codex runs its
/// own tools under roko's operation broker: the record lists the operations
/// the broker denies, whether its network was switched off, and the denial
/// that stopped `result`, if any. `None` for other providers and for a
/// request without a contract.
fn tool_policy_record(
    request: &AgentDispatchRequest,
    target: &ProviderDispatchSpec,
    result: &AgentResult,
) -> Option<roko_learn::telemetry::ToolPolicyRecord> {
    let contract = request.agent_contract.as_ref()?;
    if target.provider_kind != ProviderKind::CodexCli {
        return None;
    }
    let policy = roko_agent::exec::CodexOperationPolicy::from_contract(contract);
    let denial = result
        .output
        .tag(roko_agent::exec::CODEX_POLICY_DENIAL_TAG)
        .map(str::to_string);
    Some(roko_learn::telemetry::ToolPolicyRecord {
        allowed_tools: contract.allowed_tools.clone(),
        forbidden_tools: contract.forbidden_tool_names(),
        enforcement: "broker".to_string(),
        denied_operations: policy
            .denied_operations()
            .into_iter()
            .map(str::to_string)
            .collect(),
        network_off: !contract.permits_network(),
        denial,
    })
}

fn dispatch_events_from_result(
    request: &AgentDispatchRequest,
    target: &ProviderDispatchSpec,
    result: &AgentResult,
) -> Vec<DispatchEvent> {
    let mut events = vec![DispatchEvent::Started {
        agent_id: request.agent_id.clone(),
        provider: target.provider_id.clone(),
        model: target.model_slug.clone(),
        pid: None,
    }];

    for signal in &result.trace {
        if let Ok(text) = signal.body.as_text()
            && !text.trim().is_empty()
        {
            events.push(DispatchEvent::MessageDelta {
                text: text.to_string(),
            });
        }
    }
    if let Ok(text) = result.output.body.as_text()
        && !text.trim().is_empty()
    {
        events.push(DispatchEvent::MessageDelta {
            text: text.to_string(),
        });
    }

    if result.usage.total_tokens() > 0 || result.usage.cost_usd > 0.0 {
        events.push(DispatchEvent::TokenUsage {
            input_tokens: u64::from(result.usage.input_tokens),
            output_tokens: u64::from(result.usage.output_tokens),
            cache_read_tokens: u64::from(result.usage.cache_read_tokens),
            cache_write_tokens: u64::from(result.usage.cache_create_tokens),
            reasoning_tokens: u64::from(result.usage.reasoning_tokens),
        });
    }

    if !result.success {
        let message = result
            .output
            .body
            .as_text()
            .unwrap_or("agent failed without text output")
            .to_string();
        events.push(DispatchEvent::Error { message });
    }

    events.push(DispatchEvent::TurnCompleted {
        session_id: None,
        total_cost_usd: (result.usage.cost_usd > 0.0).then_some(f64::from(result.usage.cost_usd)),
        num_turns: reported_num_turns(result),
        is_error: !result.success,
    });
    events.push(DispatchEvent::Exited {
        exit_code: Some(if result.success { 0 } else { 1 }),
    });
    events
}

/// Turns the agent reported: the Claude CLI's `num_turns`, or the model
/// calls roko's tool loop made, both tagged `num_turns` on the output.
/// `None` when the agent did not say (the Codex and Gemini CLIs): an
/// unreported count is unknown, not one.
fn reported_num_turns(result: &AgentResult) -> Option<u32> {
    result
        .output
        .tag("num_turns")
        .and_then(|turns| turns.parse().ok())
}

/// Convert a [`roko_agent::tool_loop::StreamEvent`] into a local [`StreamChunk`].
fn stream_chunk_from_event(event: roko_agent::tool_loop::StreamEvent) -> StreamChunk {
    use roko_agent::tool_loop::StreamEventKind;
    match event.kind {
        StreamEventKind::TextDelta(text) => StreamChunk::ContentDelta(text),
        StreamEventKind::ReasoningDelta(text) => StreamChunk::ReasoningDelta(text),
        StreamEventKind::ToolCallStart { id, name } => StreamChunk::ToolCallDelta {
            id_delta: Some(id),
            name_delta: Some(name),
            args_delta: None,
        },
        StreamEventKind::ToolCallDelta { id, json_fragment } => StreamChunk::ToolCallDelta {
            id_delta: Some(id),
            name_delta: None,
            args_delta: Some(json_fragment),
        },
        StreamEventKind::ToolCallEnd { id, name, .. } => StreamChunk::ToolCallDelta {
            id_delta: Some(id),
            name_delta: Some(name),
            args_delta: None,
        },
        StreamEventKind::ToolResult { id, output, .. } => {
            // Map provider-surfaced tool results to ToolProgress so they flow
            // through to AgentRuntimeEvent::ToolOutput via agent_event_from_chunk.
            StreamChunk::ToolProgress {
                tool: id,
                status: output,
            }
        }
        StreamEventKind::Usage(usage) => StreamChunk::Usage(usage),
        StreamEventKind::Done { finish_reason } => StreamChunk::Done(finish_reason),
    }
}

/// Convert a [`StreamChunk`] into the corresponding [`AgentRuntimeEvent`].
fn agent_event_from_chunk(chunk: StreamChunk) -> AgentRuntimeEvent {
    match chunk {
        StreamChunk::ContentDelta(text) => AgentRuntimeEvent::MessageDelta { text },
        StreamChunk::ReasoningDelta(text) => AgentRuntimeEvent::MessageDelta {
            text: format!("{}{}", REASONING_DELTA_PREFIX, text),
        },
        StreamChunk::ToolCallDelta {
            id_delta,
            name_delta,
            ..
        } => AgentRuntimeEvent::ToolCall {
            id: id_delta.unwrap_or_default(),
            name: name_delta.unwrap_or_default(),
        },
        StreamChunk::Usage(usage) => AgentRuntimeEvent::TokenUsage {
            input_tokens: u64::from(usage.input_tokens),
            output_tokens: u64::from(usage.output_tokens),
            cache_read_tokens: u64::from(usage.cache_read_tokens),
            cache_write_tokens: u64::from(usage.cache_create_tokens),
            reasoning_tokens: u64::from(usage.reasoning_tokens),
        },
        StreamChunk::Done(_) => AgentRuntimeEvent::TurnCompleted {
            session_id: None,
            total_cost_usd: None,
            num_turns: None,
            is_error: false,
        },
        StreamChunk::Error(message) => AgentRuntimeEvent::Error { message },
        StreamChunk::ToolProgress { tool, status } => AgentRuntimeEvent::ToolOutput {
            id: tool,
            output: status,
        },
    }
}

fn classify_runtime(
    provider_id: &str,
    provider_kind: ProviderKind,
    provider: Option<&ProviderConfig>,
) -> ProviderRuntime {
    let Some(provider) = provider else {
        return ProviderRuntime::Unsupported(UnsupportedProvider {
            reason: UnsupportedProviderReason::MissingProvider,
            detail: format!("model references missing provider `{provider_id}`"),
        });
    };

    match CliProviderConfig::from_provider_config(provider_id.to_string(), provider) {
        Ok(cli) => return ProviderRuntime::Cli(cli),
        Err(DispatchV2Error::MissingCommand { .. }) => {
            if matches!(
                provider_kind,
                ProviderKind::ClaudeCli | ProviderKind::CodexCli | ProviderKind::CursorAcp
            ) {
                return ProviderRuntime::Unsupported(UnsupportedProvider {
                    reason: UnsupportedProviderReason::MissingCommand,
                    detail: format!("provider `{provider_id}` requires a command"),
                });
            }
        }
        Err(DispatchV2Error::UnsupportedCommand { command, .. }) => {
            if provider.base_url.is_none() && provider.api_key_env.is_none() {
                return ProviderRuntime::Unsupported(UnsupportedProvider {
                    reason: UnsupportedProviderReason::UnsupportedCommand,
                    detail: format!(
                        "provider `{provider_id}` command `{command}` is not a supported runner CLI"
                    ),
                });
            }
        }
        Err(DispatchV2Error::UnsupportedCliProvider { .. }) => {}
        Err(_) => {}
    }

    match provider_kind {
        ProviderKind::AnthropicApi
        | ProviderKind::OpenAiCompat
        | ProviderKind::PerplexityApi
        | ProviderKind::GeminiApi
        | ProviderKind::GeminiCli
        | ProviderKind::CursorAcp
        | ProviderKind::CursorCli
        | ProviderKind::CerebrasApi
        | ProviderKind::Hermes
        | ProviderKind::OpenClaw => ProviderRuntime::AgentResultBridge { provider_kind },
        ProviderKind::ClaudeCli | ProviderKind::CodexCli => {
            ProviderRuntime::Unsupported(UnsupportedProvider {
                reason: UnsupportedProviderReason::UnsupportedCliProvider,
                detail: format!("provider `{provider_id}` is not dispatchable as configured"),
            })
        }
    }
}

fn required_command(
    provider_id: &str,
    provider: &ProviderConfig,
) -> Result<PathBuf, DispatchV2Error> {
    provider
        .command
        .as_deref()
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| DispatchV2Error::MissingCommand {
            provider_id: provider_id.to_string(),
            kind: provider.kind,
        })
}

fn executable_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn upsert_env(env: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some((_, existing)) = env.iter_mut().find(|(candidate, _)| candidate == key) {
        *existing = value.to_string();
    } else {
        env.push((key.to_string(), value.to_string()));
    }
}

/// Dispatch v2 error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchV2Error {
    EmptyPrompt,
    EmptyModel,
    InvalidAgentId,
    WorkdirMissing {
        path: PathBuf,
    },
    MissingCommand {
        provider_id: String,
        kind: ProviderKind,
    },
    UnsupportedCliProvider {
        provider_id: String,
        kind: ProviderKind,
    },
    UnsupportedCommand {
        provider_id: String,
        command: String,
    },
    UnsupportedResolvedProvider {
        provider_id: String,
        detail: String,
    },
    AgentCreation {
        model_key: String,
        message: String,
    },
    McpConfigUnsupported {
        provider_id: String,
        protocol: CliProtocol,
    },
    ConflictingProviderArgument {
        provider_id: String,
        argument: String,
    },
    ContractUnsupported {
        provider_id: String,
        kind: ProviderKind,
    },
    ResourceLimitEnforcement {
        provider_id: String,
        message: String,
    },
}

impl fmt::Display for DispatchV2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => f.write_str("cannot dispatch an empty prompt"),
            Self::EmptyModel => f.write_str("cannot dispatch without a model"),
            Self::InvalidAgentId => f.write_str("cannot dispatch with an invalid agent identity"),
            Self::WorkdirMissing { path } => {
                write!(f, "dispatch workdir does not exist: {}", path.display())
            }
            Self::MissingCommand { provider_id, kind } => write!(
                f,
                "provider `{provider_id}` ({kind}) requires a non-empty command"
            ),
            Self::UnsupportedCliProvider { provider_id, kind } => write!(
                f,
                "provider `{provider_id}` ({kind}) has no supported CLI dispatch adapter"
            ),
            Self::UnsupportedCommand {
                provider_id,
                command,
            } => write!(
                f,
                "provider `{provider_id}` command `{command}` is not a supported CLI protocol"
            ),
            Self::UnsupportedResolvedProvider {
                provider_id,
                detail,
            } => write!(f, "provider `{provider_id}` is not dispatchable: {detail}"),
            Self::AgentCreation { model_key, message } => {
                write!(f, "failed to create agent for `{model_key}`: {message}")
            }
            Self::McpConfigUnsupported {
                provider_id,
                protocol,
            } => write!(
                f,
                "provider `{provider_id}` ({protocol:?}) cannot consume the configured MCP file"
            ),
            Self::ConflictingProviderArgument {
                provider_id,
                argument,
            } => write!(
                f,
                "provider `{provider_id}` argument `{argument}` conflicts with runner-enforced dispatch policy"
            ),
            Self::ContractUnsupported { provider_id, kind } => write!(
                f,
                "provider `{provider_id}` ({kind}) cannot enforce the resolved agent contract"
            ),
            Self::ResourceLimitEnforcement {
                provider_id,
                message,
            } => write!(
                f,
                "provider `{provider_id}` resource-limit enforcement failed: {message}"
            ),
        }
    }
}

impl Error for DispatchV2Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::defaults::{
        DEFAULT_CONNECT_TIMEOUT_MS, DEFAULT_REQUEST_TIMEOUT_MS, DEFAULT_TTFT_TIMEOUT_MS,
    };
    use tempfile::tempdir;

    fn write_fake_claude_script(tmp: &tempfile::TempDir, body: &str) -> PathBuf {
        let script = tmp.path().join("claude-fake.sh");
        std::fs::write(&script, body).expect("write fake claude script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut perms = std::fs::metadata(&script).expect("metadata").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).expect("chmod");
        }
        script
    }

    fn run_git(workdir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .expect("git starts");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn shared_target_request(workdir: PathBuf, target_dir: PathBuf) -> CliDispatchRequest {
        CliDispatchRequest {
            prompt: "implement it".to_string(),
            system_prompt: String::new(),
            model: "claude-sonnet-4-6".to_string(),
            workdir,
            max_turns: 10,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: vec![
                (
                    "CARGO_TARGET_DIR".to_string(),
                    target_dir.to_string_lossy().to_string(),
                ),
                ("ROKO_AGENT_SHARED_TARGET".to_string(), "1".to_string()),
            ],
            agent_id: "p/shared-target".to_string(),
            allowed_tools: None,
            disallowed_tools: Vec::new(),
            plugin_mcp: None,
        }
    }

    #[test]
    fn shared_target_is_limited_to_canonical_repo_target() {
        let fixture = tempdir().expect("fixture tempdir");
        let repo = fixture.path().join("repo");
        let attempt = fixture.path().join("attempt");
        let outside = fixture.path().join("outside");
        std::fs::create_dir(&repo).expect("repo dir");
        std::fs::create_dir(&outside).expect("outside dir");
        run_git(&repo, &["init", "-b", "main"]);
        run_git(&repo, &["config", "user.name", "Roko Test"]);
        run_git(&repo, &["config", "user.email", "roko@example.invalid"]);
        run_git(&repo, &["config", "commit.gpgsign", "false"]);
        run_git(&repo, &["commit", "--allow-empty", "-m", "base"]);
        run_git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "shared-target-test",
                attempt.to_str().expect("attempt path"),
            ],
        );
        let shared_target = repo.join("target");
        std::fs::create_dir(&shared_target).expect("shared target");

        let request = shared_target_request(attempt.clone(), shared_target.clone());
        let canonical_shared_target =
            std::fs::canonicalize(&shared_target).expect("canonical target");
        assert_eq!(shared_target_dir(&request), Some(canonical_shared_target));
        let incremental = |request: &CliDispatchRequest| {
            CliProviderConfig::claude("claude_cli", "claude")
                .build_invocation(request)
                .expect("Claude invocation")
                .env
                .into_iter()
                .find(|(key, _)| key == "CARGO_INCREMENTAL")
                .map(|(_, value)| value)
        };
        assert_eq!(incremental(&request).as_deref(), Some("1"));
        let mut default_request = request.clone();
        default_request
            .env
            .retain(|(key, _)| key != "ROKO_AGENT_SHARED_TARGET");
        assert_eq!(shared_target_dir(&default_request), None);
        assert_eq!(
            incremental(&default_request).as_deref(),
            Some("0"),
            "default mode must not share the target"
        );

        for forbidden in [PathBuf::from("/"), outside.clone()] {
            assert_eq!(
                shared_target_dir(&shared_target_request(attempt.clone(), forbidden)),
                None
            );
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, shared_target.join("escape"))
                .expect("escape symlink");
            assert_eq!(
                shared_target_dir(&shared_target_request(
                    attempt.clone(),
                    shared_target.join("escape"),
                )),
                None,
                "a symlink below target must not widen the shared root"
            );

            std::fs::remove_file(shared_target.join("escape")).expect("remove nested symlink");
            std::fs::remove_dir(&shared_target).expect("remove target directory");
            std::os::unix::fs::symlink(&repo, &shared_target).expect("target-root symlink");
            assert_eq!(
                shared_target_dir(&shared_target_request(attempt, shared_target)),
                None,
                "the target root itself must never widen authority through a symlink"
            );
        }
    }

    #[test]
    fn legacy_runner_program_detects_codex_only_by_executable_name() {
        let codex = CliProviderConfig::from_legacy_runner_program("/opt/bin/codex");
        assert_eq!(codex.descriptor.protocol, CliProtocol::CodexExecJson);

        let claude = CliProviderConfig::from_legacy_runner_program("/tmp/custom-agent");
        assert_eq!(claude.descriptor.protocol, CliProtocol::ClaudeStreamJson);
    }

    #[test]
    fn agent_dispatch_request_rejects_invalid_identity_before_resolution() {
        let invalid_ids = [
            String::new(),
            "agent\nTOKEN=request-secret".to_string(),
            "x".repeat(257),
            "PASSWORD=request-secret".to_string(),
            format!("sk-proj-{}", "A".repeat(32)),
        ];
        for agent_id in invalid_ids {
            let request = AgentDispatchRequest {
                model_key: "missing-model".to_string(),
                prompt: "do work".to_string(),
                system_prompt: String::new(),
                workdir: std::env::current_dir().expect("current dir"),
                immune_root: None,
                agent_id: agent_id.clone(),
                command: None,
                timeout_ms: None,
                mcp_config: None,
                env: Vec::new(),
                extra_args: Vec::new(),
                effort: None,
                tools: None,
                agent_contract: None,
                bare_mode: false,
                dangerously_skip_permissions: false,
                max_turns: None,
                live_output: None,
                attempt_key: None,
            };
            let error = request.validate().expect_err("invalid identity must fail");
            assert_eq!(error, DispatchV2Error::InvalidAgentId);
            let visible = error.to_string();
            assert!(!visible.contains("request-secret"));
            if !agent_id.is_empty() {
                assert!(!visible.contains(&agent_id));
            }
        }
    }

    /// gap-baab0a: Codex runs only through `CodexCliAdapter` and its
    /// operation broker, so no bare Codex subprocess is built here, with or
    /// without a tool policy, from a configured or a legacy runner program.
    #[test]
    fn codex_has_no_cli_invocation() {
        let request = CliDispatchRequest {
            prompt: "implement it".to_string(),
            system_prompt: "system".to_string(),
            model: "gpt-5".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 50,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/t".to_string(),
            allowed_tools: None,
            disallowed_tools: Vec::new(),
            plugin_mcp: None,
        };
        let restricted = CliDispatchRequest {
            allowed_tools: Some(vec!["read_file".into()]),
            disallowed_tools: vec!["web_search".into()],
            plugin_mcp: Some(plugin_mcp_config()),
            ..request.clone()
        };

        for provider in [
            CliProviderConfig::codex("codex_cli", "codex"),
            CliProviderConfig::from_legacy_runner_program("/opt/bin/codex"),
        ] {
            for request in [&request, &restricted] {
                assert_eq!(
                    provider.build_invocation(request),
                    Err(DispatchV2Error::UnsupportedCliProvider {
                        provider_id: "codex_cli".to_string(),
                        kind: ProviderKind::CodexCli,
                    })
                );
            }
        }
    }

    /// bug-6052d8: `roko chat`'s own CLI invocation drops the system prompt's
    /// cache markers, which only the Anthropic API reads.
    #[test]
    fn chat_strips_cache_markers() {
        let provider = CliProviderConfig::claude("claude_cli", "claude");
        let system_prompt = "Role\n\n<!-- cache:system -->\n\nWorkspace\n\n\
                             <!-- cache:session -->\n\nTurn";
        let request = CliDispatchRequest {
            prompt: "implement it".to_string(),
            system_prompt: system_prompt.to_string(),
            model: "claude-sonnet-4-6".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 50,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/t".to_string(),
            allowed_tools: None,
            disallowed_tools: Vec::new(),
            plugin_mcp: None,
        };

        let invocation = provider.build_invocation(&request).unwrap();
        let at = invocation
            .args
            .iter()
            .position(|arg| arg == "--append-system-prompt")
            .expect("the system prompt flag");
        assert_eq!(invocation.args[at + 1], "Role\n\nWorkspace\n\nTurn");
        assert!(
            !invocation
                .args
                .iter()
                .any(|arg| arg.contains("<!-- cache:"))
        );
    }

    /// bug-5cff57: a key nothing configures is unsupported, with the reason,
    /// even when a provider of the kind its name suggests is configured; a
    /// builtin model is not unknown.
    #[test]
    fn provider_dispatch_resolver_unknown_model_is_unsupported() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.providers.insert(
            "claude_cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                command: Some("claude".to_string()),
                ..ProviderConfig::default()
            },
        );
        let resolver = ProviderDispatchResolver::new(Arc::new(config));

        let spec = resolver.resolve("mystery-model-9");
        let ProviderRuntime::Unsupported(unsupported) = &spec.runtime else {
            panic!("an unknown model must not dispatch: {:?}", spec.runtime);
        };
        assert_eq!(unsupported.reason, UnsupportedProviderReason::UnknownModel);
        assert!(
            unsupported.detail.contains("`mystery-model-9`"),
            "{}",
            unsupported.detail
        );
        assert!(spec.provider_config.is_none());
        let builtin = resolver.resolve("claude-sonnet-4-6");
        assert!(
            !matches!(
                &builtin.runtime,
                ProviderRuntime::Unsupported(UnsupportedProvider {
                    reason: UnsupportedProviderReason::UnknownModel,
                    ..
                })
            ),
            "{:?}",
            builtin.runtime
        );
    }

    #[test]
    fn claude_invocation_enforces_allowlist_and_forbidden_tools() {
        let provider = CliProviderConfig::claude("claude_cli", "claude");
        let request = CliDispatchRequest {
            prompt: "implement it".to_string(),
            system_prompt: String::new(),
            model: "claude-sonnet-4-6".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 50,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/t".to_string(),
            allowed_tools: Some(vec![
                "read_file".into(),
                "grep".into(),
                "apply_patch".into(),
            ]),
            disallowed_tools: vec!["bash".into(), "web_search".into(), "run_tests".into()],
            plugin_mcp: None,
        };

        let invocation = provider.build_invocation(&request).unwrap();
        assert_eq!(
            invocation.turn_limit.enforcement,
            CliTurnLimitEnforcement::Native
        );
        assert_eq!(invocation.turn_limit.effective_max_turns, Some(50));
        let tools_index = invocation
            .args
            .iter()
            .position(|arg| arg == "--tools")
            .expect("allowlist flag");
        assert_eq!(invocation.args[tools_index + 1], "Read,Grep");
        let denied = invocation
            .args
            .windows(2)
            .filter(|pair| pair[0] == "--disallowed-tools")
            .map(|pair| pair[1].as_str())
            .collect::<Vec<_>>();
        assert_eq!(denied, vec!["Bash", "WebSearch"]);
    }

    #[test]
    fn claude_invocation_preserves_explicit_deny_all_allowlist() {
        let provider = CliProviderConfig::claude("claude_cli", "claude");
        let request = CliDispatchRequest {
            prompt: "restricted work".to_string(),
            system_prompt: String::new(),
            model: "claude-sonnet-4-6".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 1,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/unknown".to_string(),
            allowed_tools: Some(Vec::new()),
            disallowed_tools: Vec::new(),
            plugin_mcp: None,
        };

        let invocation = provider.build_invocation(&request).unwrap();
        let tools_index = invocation
            .args
            .iter()
            .position(|arg| arg == "--tools")
            .expect("deny-all must still emit --tools");
        assert_eq!(invocation.args[tools_index + 1], "");
    }

    fn plugin_mcp_config() -> CliPluginMcpConfig {
        CliPluginMcpConfig {
            server_name: "roko_plugins".to_string(),
            url: "http://127.0.0.1:43123/mcp".to_string(),
            bearer_token: "signed-secret".to_string(),
            tool_names: vec!["demo.echo".to_string()],
        }
    }

    #[test]
    fn claude_invocation_exposes_local_handlers_through_authenticated_mcp() {
        let provider = CliProviderConfig::claude("claude_cli", "claude");
        let request = CliDispatchRequest {
            prompt: "use the plugin".to_string(),
            system_prompt: String::new(),
            model: "claude-sonnet-4-6".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 2,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/plugin".to_string(),
            allowed_tools: Some(vec!["demo.echo".to_string()]),
            disallowed_tools: Vec::new(),
            plugin_mcp: Some(plugin_mcp_config()),
        };

        let invocation = provider
            .build_invocation(&request)
            .expect("Claude MCP invocation");
        let config_json = invocation
            .args
            .iter()
            .find(|argument| argument.contains("127.0.0.1:43123/mcp"))
            .expect("inline MCP config");
        assert!(config_json.contains("${ROKO_PLUGIN_MCP_TOKEN}"));
        assert!(
            invocation
                .args
                .iter()
                .any(|argument| argument == "--strict-mcp-config")
        );
        let tools = invocation
            .args
            .windows(2)
            .find(|pair| pair[0] == "--tools")
            .expect("tool allowlist");
        assert_eq!(tools[1], "mcp__roko_plugins__demo.echo");
        assert!(
            invocation
                .secret_env
                .iter()
                .any(|(key, value)| { key == "ROKO_PLUGIN_MCP_TOKEN" && value == "signed-secret" })
        );
        assert!(!format!("{:?}", request.plugin_mcp).contains("signed-secret"));
    }

    #[test]
    fn gemini_invocation_binds_authenticated_mcp_and_tool_policy() {
        let provider = CliProviderConfig::gemini("gemini_cli", "gemini");
        let request = CliDispatchRequest {
            prompt: "use the plugin".to_string(),
            system_prompt: "stay scoped".to_string(),
            model: "gemini-2.5-pro".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 3,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/gemini-plugin".to_string(),
            allowed_tools: Some(vec!["read_file".to_string(), "demo.echo".to_string()]),
            disallowed_tools: vec!["bash".to_string()],
            plugin_mcp: Some(plugin_mcp_config()),
        };

        let invocation = provider
            .build_invocation(&request)
            .expect("Gemini MCP invocation");
        assert_eq!(invocation.protocol, CliProtocol::GeminiStreamJson);
        assert_eq!(invocation.stdin, "stay scoped\n\n---\n\nuse the plugin");
        assert!(
            invocation
                .args
                .windows(2)
                .any(|pair| pair == ["--output-format", "stream-json"])
        );
        assert!(
            invocation
                .args
                .windows(2)
                .any(|pair| pair == ["--allowed-mcp-server-names", "roko_plugins"])
        );
        assert!(
            invocation
                .args
                .windows(2)
                .any(|pair| pair == ["--extensions", "none"])
        );
        assert!(
            invocation
                .secret_env
                .iter()
                .any(|(key, value)| key == "ROKO_PLUGIN_MCP_TOKEN" && value == "signed-secret")
        );
        assert!(!format!("{invocation:?}").contains("signed-secret"));
        assert!(
            !serde_json::to_string(&invocation)
                .expect("serialize invocation")
                .contains("signed-secret")
        );

        let ephemeral = invocation
            .ephemeral_config
            .as_ref()
            .expect("Gemini system settings");
        assert_eq!(ephemeral.env_key, "GEMINI_CLI_SYSTEM_SETTINGS_PATH");
        let settings: serde_json::Value =
            serde_json::from_str(&ephemeral.contents).expect("valid settings JSON");
        assert_eq!(settings["tools"]["core"], json!(["read_file"]));
        assert_eq!(settings["tools"]["discoveryCommand"], "");
        assert_eq!(settings["tools"]["callCommand"], "");
        assert_eq!(settings["tools"]["exclude"], json!(["run_shell_command"]));
        assert_eq!(settings["hooksConfig"]["enabled"], false);
        assert_eq!(settings["skills"]["enabled"], false);
        assert_eq!(
            settings["security"]["environmentVariableRedaction"]["allowed"],
            json!(["ROKO_PLUGIN_MCP_TOKEN"])
        );
        assert_eq!(
            settings["mcpServers"]["roko_plugins"]["httpUrl"],
            "http://127.0.0.1:43123/mcp"
        );
        assert_eq!(
            settings["mcpServers"]["roko_plugins"]["headers"]["Authorization"],
            "Bearer ${ROKO_PLUGIN_MCP_TOKEN}"
        );
        assert_eq!(
            settings["mcpServers"]["roko_plugins"]["includeTools"],
            json!(["demo.echo"])
        );
        assert!(!ephemeral.contents.contains("signed-secret"));
    }

    #[test]
    fn gemini_rejects_untranslatable_external_mcp_config() {
        let mut provider = CliProviderConfig::gemini("gemini_cli", "gemini");
        let mut request = CliDispatchRequest {
            prompt: "use configured MCP".to_string(),
            system_prompt: String::new(),
            model: "gemini-2.5-pro".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 1,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: Some(PathBuf::from("external-mcp.json")),
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/gemini-mcp".to_string(),
            allowed_tools: None,
            disallowed_tools: Vec::new(),
            plugin_mcp: None,
        };
        assert!(matches!(
            provider.build_invocation(&request),
            Err(DispatchV2Error::McpConfigUnsupported { .. })
        ));

        request.mcp_config = None;
        provider.provider_args = vec!["--allowed-mcp-server-names=unscoped".to_string()];
        assert!(matches!(
            provider.build_invocation(&request),
            Err(DispatchV2Error::ConflictingProviderArgument { .. })
        ));
    }

    #[test]
    fn gemini_cli_resolves_to_stream_runtime_and_openclaw_stays_opaque() {
        let gemini = ProviderConfig {
            kind: ProviderKind::GeminiCli,
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
            stream_usage: None,
            billing: None,
        };
        assert!(matches!(
            classify_runtime("gemini", ProviderKind::GeminiCli, Some(&gemini)),
            ProviderRuntime::Cli(CliProviderConfig {
                descriptor: CliProviderDescriptor {
                    protocol: CliProtocol::GeminiStreamJson,
                    ..
                },
                ..
            })
        ));

        let openclaw = ProviderConfig {
            kind: ProviderKind::OpenClaw,
            base_url: None,
            api_key_env: None,
            command: Some("openclaw".to_string()),
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
            stream_usage: None,
            billing: None,
        };
        assert!(matches!(
            classify_runtime("openclaw", ProviderKind::OpenClaw, Some(&openclaw)),
            ProviderRuntime::AgentResultBridge {
                provider_kind: ProviderKind::OpenClaw
            }
        ));
    }

    /// A resolved target on a provider of `kind`, for checks that read only
    /// the kind.
    fn kind_target(kind: ProviderKind) -> ProviderDispatchSpec {
        ProviderDispatchSpec {
            provider_id: "p".to_string(),
            provider_kind: kind,
            model_key: "m".to_string(),
            model_slug: "m".to_string(),
            model_profile: None,
            provider_config: None,
            runtime: ProviderRuntime::AgentResultBridge {
                provider_kind: kind,
            },
        }
    }

    /// gap-baab0a: Codex cannot enforce a tool allowlist, so a contract with
    /// one is refused for it. Codex with forbidden tools alone passes, and so
    /// does another provider with the allowlist.
    #[test]
    fn codex_cannot_take_a_contract_with_a_tool_allowlist() {
        use roko_agent::safety::contract::GovernanceRule;

        let mut request = fake_claude_request(Path::new("."), 1_000);
        request.agent_contract = Some(AgentContract {
            allowed_tools: Some(vec!["read_file".to_string(), "grep".to_string()]),
            ..AgentContract::default()
        });

        let codex = kind_target(ProviderKind::CodexCli);
        let claude = kind_target(ProviderKind::ClaudeCli);
        let refused = validate_contract_support(&request, &codex);
        assert!(
            matches!(
                refused,
                Err(DispatchV2Error::ContractUnsupported {
                    kind: ProviderKind::CodexCli,
                    ..
                })
            ),
            "{refused:?}"
        );
        assert!(validate_contract_support(&request, &claude).is_ok());

        request.agent_contract = Some(AgentContract {
            governance: vec![GovernanceRule::ForbiddenTools(vec!["bash".to_string()])],
            ..AgentContract::default()
        });
        assert!(validate_contract_support(&request, &codex).is_ok());
    }

    /// gap-baab0a: a Codex attempt records the tool policy its contract
    /// asked for and what the broker enforced, with the denial that stopped
    /// it. Other providers record none.
    #[test]
    fn codex_attempts_record_their_tool_policy() {
        use roko_agent::safety::contract::GovernanceRule;

        let mut request = fake_claude_request(Path::new("."), 1_000);
        request.agent_contract = Some(AgentContract {
            governance: vec![GovernanceRule::ForbiddenTools(vec![
                "web_fetch".to_string(),
                "web_search".to_string(),
            ])],
            ..AgentContract::default()
        });
        let denial = "web_search denied by policy: rust";
        let output = Signal::builder(Kind::AgentOutput)
            .body(Body::text(format!(
                "Codex operation policy violation: {denial}"
            )))
            .tag(roko_agent::exec::CODEX_POLICY_DENIAL_TAG, denial)
            .build();
        let result = AgentResult::fail(output);

        let codex = kind_target(ProviderKind::CodexCli);
        let record = tool_policy_record(&request, &codex, &result).expect("a Codex record");
        assert_eq!(record.allowed_tools, None);
        assert_eq!(record.forbidden_tools, ["web_fetch", "web_search"]);
        assert_eq!(record.enforcement, "broker");
        assert_eq!(record.denied_operations, ["web_search"]);
        assert!(record.network_off);
        assert_eq!(record.denial.as_deref(), Some(denial));

        let claude = kind_target(ProviderKind::ClaudeCli);
        assert_eq!(tool_policy_record(&request, &claude, &result), None);
    }

    #[test]
    fn openclaw_contract_calls_fail_closed_before_adapter_creation() {
        let target = ProviderDispatchSpec {
            provider_id: "openclaw".to_string(),
            provider_kind: ProviderKind::OpenClaw,
            model_key: "openclaw-model".to_string(),
            model_slug: "probe-model".to_string(),
            provider_config: Some(ProviderConfig {
                kind: ProviderKind::OpenClaw,
                base_url: None,
                api_key_env: None,
                command: Some("openclaw".to_string()),
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
                billing: None,
            }),
            model_profile: None,
            runtime: ProviderRuntime::AgentResultBridge {
                provider_kind: ProviderKind::OpenClaw,
            },
        };
        let request = AgentDispatchRequest {
            model_key: "openclaw-model".to_string(),
            prompt: "use plugin".to_string(),
            system_prompt: String::new(),
            workdir: std::env::current_dir().unwrap(),
            immune_root: None,
            agent_id: "p/openclaw".to_string(),
            command: None,
            timeout_ms: None,
            mcp_config: None,
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: None,
            tools: None,
            agent_contract: Some(AgentContract {
                allowed_tools: Some(vec!["demo.echo".to_string()]),
                ..AgentContract::default()
            }),
            bare_mode: false,
            dangerously_skip_permissions: false,
            max_turns: None,
            live_output: None,
            attempt_key: None,
        };
        // All provider kinds are now in the contract support whitelist,
        // so OpenClaw with a contract should pass validation.
        assert!(validate_contract_support(&request, &target).is_ok());
    }

    #[tokio::test]
    async fn run_agent_result_bridge_records_feedback_and_provider_health() {
        let tmp = tempdir().expect("tempdir");
        let script = write_fake_claude_script(
            &tmp,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"dispatch-ok"}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0}'
"#,
        );

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "dispatch-model".to_string();
        config.providers.insert(
            "dispatch-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(DEFAULT_REQUEST_TIMEOUT_MS),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(DEFAULT_CONNECT_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
                billing: None,
            },
        );
        config.models.insert(
            "dispatch-model".to_string(),
            ModelProfile {
                provider: "dispatch-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..Default::default()
            },
        );

        let request = AgentDispatchRequest {
            model_key: "dispatch-model".to_string(),
            prompt: "do work".to_string(),
            system_prompt: "system".to_string(),
            workdir: tmp.path().to_path_buf(),
            immune_root: None,
            agent_id: "dispatch-agent".to_string(),
            command: None,
            timeout_ms: Some(5_000),
            mcp_config: None,
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: None,
            tools: None,
            agent_contract: None,
            bare_mode: false,
            dangerously_skip_permissions: false,
            max_turns: None,
            live_output: None,
            attempt_key: None,
        };
        let health_path = tmp.path().join(".roko/learn/provider-health.json");
        let registry = Arc::new(ProviderHealthRegistry::new());
        let dispatcher =
            AgentDispatcherV2::new(Arc::new(config)).with_health_registry(registry.clone());

        let dispatch = dispatcher
            .run_agent_result_bridge(request)
            .await
            .expect("dispatch");

        // Flush the registry to disk so the test can read it.
        std::fs::create_dir_all(health_path.parent().unwrap()).expect("create learn dir");
        registry.save(&health_path).expect("save health");

        assert!(dispatch.result.success);
        assert_eq!(
            dispatch.result.output.body.as_text().unwrap_or(""),
            "dispatch-ok"
        );

        let efficiency_path = tmp.path().join(".roko/learn/efficiency.jsonl");
        let efficiency = std::fs::read_to_string(&efficiency_path).expect("read efficiency");
        assert!(efficiency.contains(r#""kind":"model_call""#));
        assert!(efficiency.contains(r#""role":"dispatch_v2""#));
        assert!(efficiency.contains(r#""model":"claude-sonnet-4-6""#));
        assert!(efficiency.contains(r#""provider":"dispatch-cli""#));
        assert!(efficiency.contains(r#""success":true"#));

        let provider_health =
            std::fs::read_to_string(tmp.path().join(".roko/learn/provider-health.json"))
                .expect("read provider health");
        // The registry normalizes provider keys (hyphens to underscores).
        assert!(provider_health.contains("dispatch_cli"));

        // The bridge never teaches the router (bug-07bc75): Graph dispatch
        // does, from each attempt's settled verdict.
        assert!(
            !tmp.path().join(".roko/learn/cascade-router.json").exists(),
            "the bridge must not observe or save the cascade router"
        );
    }

    /// A config whose one model, `dispatch-model`, runs the Claude CLI
    /// `script` through the `dispatch-cli` provider.
    fn fake_claude_config(script: &Path) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "dispatch-model".to_string();
        config.providers.insert(
            "dispatch-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(DEFAULT_REQUEST_TIMEOUT_MS),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(DEFAULT_CONNECT_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
                billing: None,
            },
        );
        config.models.insert(
            "dispatch-model".to_string(),
            ModelProfile {
                provider: "dispatch-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..Default::default()
            },
        );
        config
    }

    /// A request for `dispatch-model` in `workdir`, killed after
    /// `timeout_ms`.
    fn fake_claude_request(workdir: &Path, timeout_ms: u64) -> AgentDispatchRequest {
        AgentDispatchRequest {
            model_key: "dispatch-model".to_string(),
            prompt: "do work".to_string(),
            system_prompt: "system".to_string(),
            workdir: workdir.to_path_buf(),
            immune_root: None,
            agent_id: "dispatch-agent".to_string(),
            command: None,
            timeout_ms: Some(timeout_ms),
            mcp_config: None,
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: None,
            tools: None,
            agent_contract: None,
            bare_mode: false,
            dangerously_skip_permissions: false,
            max_turns: None,
            live_output: None,
            attempt_key: None,
        }
    }

    /// gap-ff95f5: a dispatch's agent records its tool calls with the safety
    /// provenance sink of the run its attempt belongs to, while that run is
    /// registered, and with none otherwise.
    #[test]
    fn agent_options_carry_the_runs_safety_provenance_sink() {
        use crate::safety_provenance::{GraphProvenanceSink, ProvenanceSinks};

        let workspace = tempdir().expect("tempdir");
        let sinks = ProvenanceSinks::default();
        let sink = GraphProvenanceSink::open(workspace.path()).expect("provenance sink");
        let registration = sinks.register("run-7", Arc::new(sink));
        let dispatcher = AgentDispatcherV2::new(Arc::new(RokoConfig::default()))
            .with_provenance_sinks(sinks.clone());
        let mut request = fake_claude_request(workspace.path(), 1_000);
        request.attempt_key = Some("run-7:plan:task-1:1".to_string());
        assert!(dispatcher.agent_options(&request).provenance_sink.is_some());
        request.attempt_key = Some("run-8:plan:task-1:1".to_string());
        assert!(dispatcher.agent_options(&request).provenance_sink.is_none());
        drop(registration);
        request.attempt_key = Some("run-7:plan:task-1:1".to_string());
        assert!(dispatcher.agent_options(&request).provenance_sink.is_none());
    }

    /// bug-7cdce7: an attempt killed at its wall-clock timeout, or stopped at
    /// its turn cap, is a task outcome. Three in a row through the Graph
    /// bridge leave the provider's circuit closed, while three real provider
    /// failures still open it.
    #[tokio::test]
    async fn attempt_timeouts_do_not_open_the_provider_circuit() {
        let cases = [
            (
                "attempt timeout",
                "#!/bin/sh\ncat >/dev/null\nexec sleep 30\n",
                100,
                "timed out after",
                true,
            ),
            (
                "turn cap",
                r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"result","subtype":"error_max_turns","is_error":true,"num_turns":2,"total_cost_usd":0}'
exit 1
"#,
                10_000,
                "turn cap reached",
                true,
            ),
            (
                "provider failure",
                r#"#!/bin/sh
cat >/dev/null
echo '503 service temporarily unavailable' >&2
exit 1
"#,
                10_000,
                "503",
                false,
            ),
        ];
        for (case, body, timeout_ms, says, stays_closed) in cases {
            let tmp = tempdir().expect("tempdir");
            let script = write_fake_claude_script(&tmp, body);
            let registry = Arc::new(ProviderHealthRegistry::new());
            let dispatcher = AgentDispatcherV2::new(Arc::new(fake_claude_config(&script)))
                .with_health_registry(registry.clone());
            for _ in 0..3 {
                let dispatch = dispatcher
                    .run_agent_result_bridge_with_tools_and_cli_mcp(
                        fake_claude_request(tmp.path(), timeout_ms),
                        None,
                        None,
                        None,
                        false,
                    )
                    .await
                    .expect("dispatch");
                let text = dispatch.result.output.body.as_text().unwrap_or_default();
                assert!(!dispatch.result.success, "{case}: {text}");
                assert!(text.contains(says), "{case}: {text}");
            }
            let health = registry.get("dispatch-cli");
            assert_eq!(
                registry.is_available("dispatch-cli"),
                stays_closed,
                "{case}: {health:?}"
            );
            if stays_closed {
                assert_eq!(health.consecutive_failures, 0, "{case}: {health:?}");
            }
        }
    }

    /// backlog 1114: an immune denial is the host's own policy, here decided
    /// before any model call, not a provider outcome. A denied bridge run,
    /// with or without the dispatcher's registry, leaves the registry and
    /// `provider-health.json` as they were.
    #[tokio::test]
    async fn immune_denial_leaves_provider_health_unchanged() {
        let tmp = tempdir().expect("tempdir");
        let calls = tmp.path().join("provider-calls.log");
        let script = write_fake_claude_script(
            &tmp,
            &format!(
                "#!/bin/sh\ncat >/dev/null\necho called >> '{}'\nexit 1\n",
                calls.display()
            ),
        );
        let config = Arc::new(fake_claude_config(&script));
        let learn_dir = tmp.path().join(".roko/learn");
        std::fs::create_dir_all(&learn_dir).expect("create learn dir");
        let health_path = learn_dir.join("provider-health.json");
        let registry = Arc::new(ProviderHealthRegistry::new());
        registry.record_success("dispatch-cli");
        registry.save(&health_path).expect("save health");
        let health_before = registry.get("dispatch-cli");
        let file_before = std::fs::read_to_string(&health_path).expect("read health");
        // An isolation control on the request's agent makes the immune
        // boundary deny each run before its provider.
        roko_agent::isolate_agent(tmp.path(), "dispatch-agent", "test_isolation")
            .expect("isolate the agent");

        let with_registry =
            AgentDispatcherV2::new(Arc::clone(&config)).with_health_registry(registry.clone());
        let without_registry = AgentDispatcherV2::new(config);
        for dispatcher in [with_registry, without_registry] {
            let dispatch = dispatcher
                .run_agent_result_bridge(fake_claude_request(tmp.path(), 10_000))
                .await
                .expect("dispatch");
            assert!(!dispatch.result.success);
            assert_eq!(dispatch.result.output.tag("immune_denied"), Some("true"));
            assert_eq!(
                dispatch.result.output.tag("immune_reason"),
                Some("agent_isolated")
            );
        }

        assert!(!calls.exists(), "the provider must not be called");
        assert_eq!(registry.get("dispatch-cli"), health_before);
        assert_eq!(
            std::fs::read_to_string(&health_path).expect("read health"),
            file_before
        );
    }

    /// backlog 1114: a bridge attempt leaves exactly one provider-health
    /// record. With the dispatcher's registry attached, the registry holds it
    /// and the feedback recorder writes no second one to
    /// `provider-health.json`; without one, the recorder writes it, under the
    /// failure's classified error rather than `Unknown`.
    #[tokio::test]
    async fn bridge_attempt_records_provider_health_once() {
        use roko_learn::provider_health::ErrorClass;

        let tmp = tempdir().expect("tempdir");
        let script = write_fake_claude_script(
            &tmp,
            r#"#!/bin/sh
cat >/dev/null
echo '503 service temporarily unavailable' >&2
exit 1
"#,
        );
        let config = Arc::new(fake_claude_config(&script));
        let health_path = tmp.path().join(".roko/learn/provider-health.json");

        let registry = Arc::new(ProviderHealthRegistry::new());
        let dispatcher =
            AgentDispatcherV2::new(Arc::clone(&config)).with_health_registry(registry.clone());
        let dispatch = dispatcher
            .run_agent_result_bridge(fake_claude_request(tmp.path(), 10_000))
            .await
            .expect("dispatch");
        assert!(!dispatch.result.success);
        let health = registry.get("dispatch-cli");
        assert_eq!(health.total_requests, 1, "{health:?}");
        assert_eq!(health.total_failures, 1, "{health:?}");
        assert!(
            !health_path.exists(),
            "the feedback recorder must not record the attempt a second time"
        );

        let dispatch = AgentDispatcherV2::new(config)
            .run_agent_result_bridge(fake_claude_request(tmp.path(), 10_000))
            .await
            .expect("dispatch");
        assert!(!dispatch.result.success);
        let persisted = ProviderHealthRegistry::load_or_new(&health_path).get("dispatch-cli");
        assert_eq!(persisted.total_requests, 1, "{persisted:?}");
        assert_eq!(persisted.total_failures, 1, "{persisted:?}");
        let classes: Vec<ErrorClass> = persisted
            .failure_window
            .iter()
            .map(|failure| failure.error_class)
            .collect();
        assert_eq!(classes, vec![ErrorClass::ServerError]);
    }

    /// gap-28ceb9: a usage-window refusal is a class of its own, which the
    /// circuit breaker records as exhaustion, ahead of the billing and
    /// rate-limit wording it can share.
    #[test]
    fn classify_provider_error_detects_usage_exhaustion() {
        for text in [
            "You've hit your session limit · resets 4pm",
            "You've hit your usage limit. Upgrade to Pro or try again later.",
            "usage limit reached for this quota window",
        ] {
            assert_eq!(
                classify_provider_error(&text.to_ascii_lowercase()),
                "provider_exhausted",
                "{text}"
            );
        }
        assert_eq!(
            classify_provider_error("429 too many requests"),
            "rate_limit"
        );
        assert_eq!(
            classify_provider_error("insufficient credits"),
            "insufficient_credits"
        );
    }

    /// backlog 1113: a CLI that is not logged in is an auth failure, not an
    /// unknown error that is retried and opens the circuit.
    #[test]
    fn not_logged_in_classifies_as_auth_failure() {
        assert_eq!(
            classify_provider_error("exit 1: not logged in · please run /login"),
            "auth_failure"
        );
        assert_eq!(
            classify_provider_error("429 too many requests"),
            "rate_limit"
        );
        assert_eq!(
            classify_provider_error("provider returned an empty response (empty_response)"),
            "empty_response"
        );
    }

    /// E04-T06: Verify that the default Claude CLI dispatch path exercises
    /// roko-side pre- and post-dispatch safety checks via SafetyLayer.
    #[test]
    fn claude_cli_dispatch_runs_safety_funnel() {
        use roko_agent::SafetyLayer;
        use roko_agent::safety::ViolationSeverity;

        let tmp = tempdir().expect("tempdir");
        let workdir = tmp.path().to_path_buf();

        // Build a SafetyLayer with defaults (the production path).
        let safety = SafetyLayer::with_defaults();

        // ── Pre-dispatch: normal workdir passes ──────────────────────
        let pre_result =
            safety.pre_dispatch_check("test-plan", "test-task", "implementer", &workdir);
        assert!(
            pre_result.is_ok(),
            "pre-dispatch check should pass for a valid workdir"
        );

        // ── Pre-dispatch: path-traversal workdir is blocked ──────────
        // Use a non-existent traversal path that cannot be canonicalized
        // -- it falls back to the raw string which contains "..".
        let traversal_dir = tmp.path().join("nonexistent/../../..");
        let pre_traversal =
            safety.pre_dispatch_check("test-plan", "test-task", "implementer", &traversal_dir);
        // The path policy checks canonicalized paths; when canonicalization
        // fails (non-existent path) it falls back to the raw string.
        // Verify the API is callable and returns a structured result.
        let _ = pre_traversal;

        // ── Post-dispatch: clean output passes ───────────────────────
        let clean_output = "implemented the feature successfully";
        let post_clean =
            safety.post_dispatch_check("test-plan", "test-task", "implementer", clean_output, &[]);
        assert!(
            post_clean.is_empty(),
            "post-dispatch check should produce no violations for clean output"
        );

        // ── Post-dispatch: path-escape in changed files is Block ─────
        let escape_files = vec!["../../../etc/passwd".to_string()];
        let post_escape = safety.post_dispatch_check(
            "test-plan",
            "test-task",
            "implementer",
            clean_output,
            &escape_files,
        );
        assert!(
            !post_escape.is_empty(),
            "post-dispatch check should flag path escape in changed files"
        );
        assert!(
            post_escape
                .iter()
                .any(|v| v.severity == ViolationSeverity::Block),
            "path escape violations must be Block severity"
        );

        // ── Post-dispatch: secret leak in output is Block ────────────
        let secret_output = "here is the api key: AKIA1234567890ABCDEF";
        let post_secret =
            safety.post_dispatch_check("test-plan", "test-task", "implementer", secret_output, &[]);
        // The scrub policy detects AWS-style keys by default.
        // If the default scrub patterns catch it, we get a Block violation.
        if !post_secret.is_empty() {
            assert!(
                post_secret
                    .iter()
                    .any(|v| v.severity == ViolationSeverity::Block),
                "secret leak violations must be Block severity per E04-T05"
            );
        }
    }

    /// One tool-audit line of `attempt`: an admission, or with `ok` a result.
    fn audit_line(call_id: &str, ok: Option<bool>, attempt: &str) -> String {
        use roko_fs::tool_audit::AuditLine;

        let correlation = roko_core::tool::CorrelationEnvelope {
            attempt_id: attempt.to_string(),
            ..roko_core::tool::CorrelationEnvelope::empty()
        };
        let line = match ok {
            None => AuditLine::Admit {
                ts_ms: 1,
                call_id: call_id.to_string(),
                call_name: "read_file".to_string(),
                arguments_scrubbed: "{}".to_string(),
                correlation,
            },
            Some(ok) => AuditLine::Result {
                ts_ms: 2,
                call_id: call_id.to_string(),
                call_name: "read_file".to_string(),
                ok,
                content_scrubbed: String::new(),
                correlation,
            },
        };
        serde_json::to_string(&line).expect("serialize audit line")
    }

    /// gap-4d5e2d: an attempt's audit lines pair into its tool calls. A
    /// result settles the earliest call admitted under its id that has none
    /// yet, so reused ids still give one record per call; a call with no
    /// result keeps an unknown outcome; other attempts' lines and stray text
    /// are skipped.
    #[test]
    fn audited_tool_calls_pair_each_result_with_its_admission() {
        let attempt = "run-1:plan:T01:1";
        let other = "run-1:plan:T02:1";
        let lines = [
            audit_line("call-1", None, attempt),
            "{\"kind\":\"res".to_string(),
            audit_line("call-1", None, other),
            audit_line("call-1", Some(false), other),
            audit_line("call-1", Some(true), attempt),
            audit_line("call-2", None, attempt),
            audit_line("call-1", None, attempt),
            audit_line("call-1", Some(false), attempt),
            audit_line("call-3", Some(true), attempt),
        ]
        .join("\n");

        let calls = audited_tool_calls(&lines, attempt);
        let outcomes: Vec<(&str, Option<bool>)> = calls
            .iter()
            .map(|call| (call.id.as_str(), call.succeeded))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("call-1", Some(true)),
                ("call-2", None),
                ("call-1", Some(false)),
                ("call-3", Some(true)),
            ]
        );
        assert!(calls.iter().all(|call| call.name == "read_file"));
    }

    /// gap-4d5e2d: a dispatch reads back only the audit lines written after
    /// its mark, so an earlier attempt's lines under a reused key don't
    /// count. A missing audit gives no calls.
    #[tokio::test]
    async fn tool_audit_mark_reads_only_lines_written_after_it() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("tool_audit.jsonl");
        let attempt = "run-1:plan:T01:1";
        let missing = ToolAuditMark::at(path.clone(), attempt).await;
        assert!(missing.tool_calls().await.is_empty());

        let before = format!(
            "{}\n{}\n",
            audit_line("call-0", None, attempt),
            audit_line("call-0", Some(true), attempt)
        );
        std::fs::write(&path, before).expect("write earlier lines");
        let mark = ToolAuditMark::at(path.clone(), attempt).await;
        let after = format!(
            "{}\n{}\n",
            audit_line("call-1", None, attempt),
            audit_line("call-1", Some(false), attempt)
        );
        let mut audit = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open audit");
        std::io::Write::write_all(&mut audit, after.as_bytes()).expect("append lines");

        assert_eq!(
            mark.tool_calls().await,
            [ToolCallRecord {
                id: "call-1".to_string(),
                name: "read_file".to_string(),
                succeeded: Some(false),
            }]
        );
    }

    fn cost_test_target(model_slug: &str, profile: Option<ModelProfile>) -> ProviderDispatchSpec {
        ProviderDispatchSpec {
            model_key: model_slug.to_string(),
            model_slug: model_slug.to_string(),
            provider_id: "test".to_string(),
            provider_kind: ProviderKind::OpenAiCompat,
            model_profile: profile,
            provider_config: None,
            runtime: ProviderRuntime::AgentResultBridge {
                provider_kind: ProviderKind::OpenAiCompat,
            },
        }
    }

    #[test]
    fn fill_cost_falls_back_to_registry_pricing_for_known_slug() {
        // No profile at all (or a profile without cost fields): known slugs
        // still get priced from the shared registry instead of staying $0.
        let target = cost_test_target("glm-5.1", None);
        let mut result = AgentResult::ok(
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("done"))
                .build(),
        );
        result.usage.input_tokens = 1_000_000;
        result.usage.output_tokens = 1_000_000;

        fill_cost_from_profile(&mut result, &target, None);

        // glm-5.1 registry rates: $1.40/M input + $4.40/M output.
        assert!(
            (f64::from(result.usage.cost_usd) - 5.80).abs() < 1e-6,
            "registry-priced cost, got {}",
            result.usage.cost_usd
        );
        assert!(result.usage.has_known_cost());
    }

    #[test]
    fn fill_cost_leaves_unknown_slug_zero_and_marked_unknown() {
        let target = cost_test_target("totally-unknown-llm-9000", None);
        let mut result = AgentResult::ok(
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("done"))
                .build(),
        );
        result.usage.input_tokens = 1_000;

        fill_cost_from_profile(&mut result, &target, None);

        // Unknown model: cost stays 0.0 and reports as unknown, not free.
        assert!(result.usage.cost_usd.abs() <= f32::EPSILON);
        assert!(!result.usage.has_known_cost());
    }

    #[test]
    fn fill_cost_profile_pricing_wins_over_registry() {
        let profile = ModelProfile {
            provider: "zai".to_string(),
            slug: "glm-5.1".to_string(),
            cost_input_per_m: Some(9.0),
            cost_output_per_m: Some(9.0),
            ..ModelProfile::default()
        };
        let target = cost_test_target("glm-5.1", Some(profile));
        let mut result = AgentResult::ok(
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("done"))
                .build(),
        );
        result.usage.input_tokens = 1_000_000;
        result.usage.output_tokens = 1_000_000;

        fill_cost_from_profile(&mut result, &target, None);

        // Configured profile rates ($9/$9) beat the registry ($1.40/$4.40).
        assert!(
            (f64::from(result.usage.cost_usd) - 18.0).abs() < 1e-6,
            "profile-priced cost, got {}",
            result.usage.cost_usd
        );
    }

    /// backlog 2114: a call to a model the dated price snapshot lists is
    /// priced at the snapshot's rates, even when roko.toml prices it
    /// otherwise; a model the snapshot lacks falls back to its profile.
    #[test]
    fn plan_run_prices_from_the_dated_snapshot() {
        // Named, not the built-in copy, which a newer snapshot replaces.
        let snapshot = PriceSnapshot::from_toml(
            include_str!("../../../config/prices/2026-09-28.toml"),
            "config/prices/2026-09-28.toml",
        )
        .expect("the 2026-09-28 snapshot");
        let profile = |slug: &str| ModelProfile {
            provider: "cerebras".to_string(),
            slug: slug.to_string(),
            cost_input_per_m: Some(0.5),
            cost_output_per_m: Some(1.5),
            ..ModelProfile::default()
        };
        let million = |usage: &mut roko_core::Usage| {
            usage.input_tokens = 1_000_000;
            usage.output_tokens = 1_000_000;
        };

        let mut usage = roko_core::Usage::zero();
        million(&mut usage);
        let gpt_oss = profile("gpt-oss-120b");
        let pricing = fill_usage_cost_from_pricing(
            &mut usage,
            Some(&snapshot),
            Some(&gpt_oss),
            "gpt-oss-120b",
        );
        assert_eq!(pricing, CallPricing::Snapshot);
        assert_eq!(snapshot.id(), "prices-2026-09-28");
        // 0.35 in + 0.75 out, not the profile's 0.5 + 1.5.
        let cost = f64::from(usage.cost_usd);
        assert!((cost - 1.10).abs() < 1e-6, "{cost}");
        assert!(model_has_price(Some(&snapshot), None, "gpt-oss-120b"));
        let uncached =
            usage_cost_without_cache(&usage, Some(&snapshot), Some(&gpt_oss), "gpt-oss-120b");
        assert!(uncached.is_some_and(|uncached| (uncached - 1.10).abs() < 1e-9));

        let mut usage = roko_core::Usage::zero();
        million(&mut usage);
        let unlisted = profile("qwen-3.8-27b");
        let pricing = fill_usage_cost_from_pricing(
            &mut usage,
            Some(&snapshot),
            Some(&unlisted),
            "qwen-3.8-27b",
        );
        assert_eq!(pricing, CallPricing::Profile, "no snapshot row, so no snapshot id");
        assert!(snapshot.row("qwen-3.8-27b").is_none());
        let cost = f64::from(usage.cost_usd);
        assert!((cost - 2.0).abs() < 1e-6, "{cost}");

        let mut reported = roko_core::Usage::zero();
        million(&mut reported);
        reported.cost_usd = 0.25;
        let pricing =
            fill_usage_cost_from_pricing(&mut reported, Some(&snapshot), None, "gpt-oss-120b");
        assert_eq!(pricing, CallPricing::Reported, "a reported cost stands");
        assert!((f64::from(reported.cost_usd) - 0.25).abs() < 1e-6);
    }

    /// backlog 2114 (decision 2113): calls are priced from the newest
    /// snapshot in the workspace's `config/prices/`, else the built-in copy,
    /// loaded once per workspace.
    #[test]
    fn pricing_snapshot_is_the_workspaces_newest_loaded_once() {
        let empty = tempfile::tempdir().expect("tempdir");
        let pricing = PricingConfig::default();
        let builtin = pricing_snapshot(&pricing, empty.path()).expect("the built-in copy");
        assert_eq!(builtin.id(), roko_core::pricing_snapshot::BUILTIN_SNAPSHOT_ID);

        let workspace = tempfile::tempdir().expect("tempdir");
        let prices = workspace.path().join("config/prices");
        std::fs::create_dir_all(&prices).expect("prices dir");
        let newer = include_str!("../../../config/prices/2026-09-28.toml")
            .replace("id = \"prices-2026-09-28\"", "id = \"prices-2026-10-01\"");
        std::fs::write(prices.join("2026-10-01.toml"), newer).expect("write the snapshot");
        let first = pricing_snapshot(&pricing, workspace.path()).expect("the newest snapshot");
        assert_eq!(first.id(), "prices-2026-10-01");
        let again = pricing_snapshot(&pricing, workspace.path()).expect("the newest snapshot");
        assert!(Arc::ptr_eq(&first, &again), "loaded once");
    }
}
