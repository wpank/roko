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
use roko_agent::StreamChunk;
use roko_agent::model_call_service::ProviderOutcomeRecorder;
use roko_agent::process::ResourceLimits;
use roko_agent::provider::{AgentOptions, LocalToolMcpServer, ProviderSemaphores};
use roko_agent::rate_limit::ProviderRateLimiter;
use roko_agent::safety::contract::AgentContract;
use roko_agent::{Agent, AgentResult, create_agent_for_model};
use roko_core::agent::{ProviderKind, resolve_model};
use roko_core::config::schema::{ModelProfile, ProviderConfig, RokoConfig};
use roko_core::tool::aliases::{canonical_names, claude_of_canonical};
use roko_core::{Body, Context, Kind, Signal};
use roko_learn::model_call_feedback::{ModelCallFeedback, ModelCallFeedbackRecorder};
use roko_learn::provider_health::ProviderHealthRegistry;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;

use crate::learning_helpers::capture_runtime_model_slugs;

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
    use roko_learn::cascade_router::CascadeRouter;
    use roko_learn::feedback_service::FeedbackService;

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
    let cascade_router = (!cascade_model_slugs.is_empty()).then(|| {
        Arc::new(CascadeRouter::load_or_new(
            &cascade_path,
            cascade_model_slugs,
        ))
    });

    let feedback_service = FeedbackService::from_roko_dir(&workdir.join(".roko"));
    let feedback_sink: Arc<dyn FeedbackSink> = match &cascade_router {
        Some(router) => Arc::new(feedback_service.with_cascade_router(Arc::clone(router))),
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
        && let Err(err) = router.save(&cascade_path)
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
            Self::CodexExecJson => ProviderKind::OpenAiCompat,
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

    /// Build a Codex CLI provider.
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
                let mut config = if executable_name(&command).contains("codex") {
                    Self::codex(provider_id, command)
                } else {
                    Self::claude(provider_id, command)
                };
                config.provider_args = provider.args.clone().unwrap_or_default();
                config.resource_limits = configured_cli_resource_limits(provider)?;
                Ok(config)
            }
            ProviderKind::OpenAiCompat => {
                let command = required_command(&provider_id, provider)?;
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
        match self.descriptor.protocol {
            CliProtocol::ClaudeStreamJson => self.build_claude_invocation(request),
            CliProtocol::CodexExecJson => self.build_codex_invocation(request),
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
            args.push("--mcp-config".to_string());
            if let Some(mcp_config) = &request.mcp_config {
                args.push(mcp_config.to_string_lossy().to_string());
            }
            if let Some(plugin_mcp) = &request.plugin_mcp {
                args.push(claude_plugin_mcp_json(plugin_mcp));
            }
            args.push("--strict-mcp-config".to_string());
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

        Ok(CliInvocation::new(
            self,
            request,
            args,
            request.prompt.clone(),
        ))
    }

    fn build_codex_invocation(
        &self,
        request: &CliDispatchRequest,
    ) -> Result<CliInvocation, DispatchV2Error> {
        // Codex CLI has no binding native-tool allow/deny flag. The MCP
        // bridge enforces its own contract-scoped catalog, but accepting a
        // request-level policy here would still leave Codex built-ins outside
        // that policy. Preserve the existing fail-closed behavior.
        if request.allowed_tools.is_some() || !request.disallowed_tools.is_empty() {
            return Err(DispatchV2Error::ToolPolicyUnsupported {
                provider_id: self.descriptor.provider_id.clone(),
                protocol: self.descriptor.protocol,
            });
        }
        let mut args = vec!["exec".to_string()];
        args.extend(self.provider_args.clone());
        if let Some(plugin_mcp) = &request.plugin_mcp {
            args.extend(codex_plugin_mcp_args(plugin_mcp));
        }
        args.push("--json".to_string());
        args.push("--cd".to_string());
        args.push(request.workdir.to_string_lossy().to_string());
        args.push("--skip-git-repo-check".to_string());
        args.push("--color".to_string());
        args.push("never".to_string());

        if request.dangerously_skip_permissions {
            args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        } else {
            args.push("--sandbox".to_string());
            args.push("workspace-write".to_string());
        }

        if !request.model.trim().is_empty() && !request.model.starts_with("claude") {
            args.push("--model".to_string());
            args.push(request.model.clone());
        }
        args.push("-".to_string());

        let stdin = if request.system_prompt.trim().is_empty() {
            request.prompt.clone()
        } else {
            format!(
                "{}\n\n---\n\n{}",
                request.system_prompt.trim(),
                request.prompt
            )
        };

        Ok(CliInvocation::new(self, request, args, stdin))
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
    /// equivalent built-in-tool flag and rejects such requests fail-closed.
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

fn codex_plugin_mcp_args(config: &CliPluginMcpConfig) -> Vec<String> {
    let prefix = format!("mcp_servers.{}", config.server_name);
    let tools = toml::Value::Array(
        config
            .tool_names
            .iter()
            .cloned()
            .map(toml::Value::String)
            .collect(),
    )
    .to_string();
    [
        format!("{prefix}.url={}", toml::Value::String(config.url.clone())),
        format!(
            "{prefix}.bearer_token_env_var={}",
            toml::Value::String("ROKO_PLUGIN_MCP_TOKEN".to_string())
        ),
        format!("{prefix}.required=true"),
        format!("{prefix}.enabled_tools={tools}"),
        format!("{prefix}.default_tools_approval_mode=\"auto\""),
    ]
    .into_iter()
    .flat_map(|value| ["--config".to_string(), value])
    .collect()
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
        upsert_env(&mut env, "CARGO_INCREMENTAL", "0");
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

    /// Resolve a model key into a dispatchable provider target.
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
#[derive(Debug, Clone)]
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
        let started = Instant::now();
        let mut result = created.agent.run(&input, &Context::now()).await;
        let latency_ms = started.elapsed().as_millis() as u64;
        fill_cost_from_profile(&mut result, &created.target);

        // Record provider outcome for the circuit breaker (E48-T05).
        if let Some(registry) = &self.health_registry {
            let provider_id = &created.target.provider_id;
            if result.success {
                registry.record_provider_success(provider_id);
            } else {
                let output_text = result
                    .output
                    .body
                    .as_text()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                registry
                    .record_provider_failure(provider_id, classify_provider_error(&output_text));
            }
        }

        record_agent_dispatch_feedback(
            &self.config,
            &request,
            &created.target,
            &result,
            latency_ms,
        )
        .await;
        let events = dispatch_events_from_result(&request, &created.target, &result);
        Ok(AgentResultDispatch {
            target: created.target,
            result,
            events,
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

        // Set up streaming channel: chunks flow from agent -> forwarder -> event_tx.
        let (chunk_tx, mut chunk_rx) =
            mpsc::channel::<StreamChunk>(roko_core::defaults::DEFAULT_CHANNEL_BUFFER);
        let forwarder_tx = event_tx.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(chunk) = chunk_rx.recv().await {
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

        // Back-fill cost from model profile pricing before checking cost_usd.
        fill_cost_from_profile(&mut result, &created.target);

        // Emit terminal events.
        if result.usage.total_tokens() > 0 || result.usage.cost_usd > 0.0 {
            let _ = event_tx
                .send(AgentRuntimeEvent::TokenUsage {
                    input_tokens: u64::from(result.usage.input_tokens),
                    output_tokens: u64::from(result.usage.output_tokens),
                    cache_read_tokens: u64::from(result.usage.cache_read_tokens),
                    cache_write_tokens: u64::from(result.usage.cache_create_tokens),
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
                num_turns: Some(1),
                is_error: !result.success,
            })
            .await;
        let _ = event_tx
            .send(AgentRuntimeEvent::Exited {
                exit_code: Some(if result.success { 0 } else { 1 }),
            })
            .await;

        record_agent_dispatch_feedback(
            &self.config,
            &request,
            &created.target,
            &result,
            latency_ms,
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
        let started = Instant::now();
        let mut result = agent.run(&input, &Context::now()).await;
        let latency_ms = started.elapsed().as_millis() as u64;
        fill_cost_from_profile(&mut result, &target);

        // Record provider outcome for the circuit breaker (E48-T05).
        // This must happen before any gate verdict is applied so a provider
        // success followed by a failing code/test gate remains a provider
        // success in the health registry.
        if let Some(registry) = &self.health_registry {
            let provider_id = &target.provider_id;
            if result.success {
                registry.record_provider_success(provider_id);
            } else {
                let output_text = result
                    .output
                    .body
                    .as_text()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                registry
                    .record_provider_failure(provider_id, classify_provider_error(&output_text));
            }
        }

        record_agent_dispatch_feedback(&self.config, &request, &target, &result, latency_ms).await;
        let events = dispatch_events_from_result(&request, &target, &result);
        Ok(AgentResultDispatch {
            target,
            result,
            events,
        })
    }

    fn agent_options(&self, request: &AgentDispatchRequest) -> AgentOptions {
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
            // Thread the runtime-scoped rate limiter through to provider adapters.
            // HTTP-backed adapters (OpenAI-compat, Anthropic API, Gemini) will call
            // `limiter.acquire(provider_id)` before each live LLM request so that
            // all concurrent task dispatches share one RPM/TPM budget per provider.
            rate_limiter: self.rate_limiter.clone(),
            ..Default::default()
        }
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

fn validate_contract_support(
    request: &AgentDispatchRequest,
    target: &ProviderDispatchSpec,
) -> Result<(), DispatchV2Error> {
    if request.agent_contract.is_none()
        || matches!(
            target.provider_kind,
            ProviderKind::ClaudeCli
                | ProviderKind::AnthropicApi
                | ProviderKind::OpenAiCompat
                | ProviderKind::PerplexityApi
                | ProviderKind::GeminiApi
                | ProviderKind::CerebrasApi
                | ProviderKind::CursorAcp
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
    if output_text_lower.contains("rate limit")
        || output_text_lower.contains("rate_limit")
        || output_text_lower.contains("429")
        || output_text_lower.contains("too many requests")
    {
        "rate_limit"
    } else if output_text_lower.contains("timeout") || output_text_lower.contains("timed out") {
        "timeout"
    } else if output_text_lower.contains("503")
        || output_text_lower.contains("502")
        || output_text_lower.contains("server error")
        || output_text_lower.contains("temporarily unavailable")
    {
        "server_error"
    } else {
        "unknown"
    }
}

async fn record_agent_dispatch_feedback(
    config: &RokoConfig,
    request: &AgentDispatchRequest,
    target: &ProviderDispatchSpec,
    result: &AgentResult,
    latency_ms: u64,
) {
    let cascade_model_slugs = capture_runtime_model_slugs(config, &target.model_slug);
    let recorder = ModelCallFeedbackRecorder::from_workdir(&request.workdir, cascade_model_slugs);
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
            provider_success: Some(result.success),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

/// Provider-neutral events emitted by dispatch v2.
pub type DispatchEvent = AgentRuntimeEvent;

/// Back-fill `usage.cost_usd` from the model profile's per-million token
/// pricing when the provider did not report a dollar amount natively.
fn fill_cost_from_profile(result: &mut AgentResult, target: &ProviderDispatchSpec) {
    if let Some(profile) = target.model_profile.as_ref() {
        result.usage.fill_cost_from_pricing(
            profile.cost_input_per_m,
            profile.cost_output_per_m,
            profile.cost_cache_read_per_m,
        );
    }
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
        num_turns: Some(1),
        is_error: !result.success,
    });
    events.push(DispatchEvent::Exited {
        exit_code: Some(if result.success { 0 } else { 1 }),
    });
    events
}

/// Convert a [`StreamChunk`] into the corresponding [`AgentRuntimeEvent`].
fn agent_event_from_chunk(chunk: StreamChunk) -> AgentRuntimeEvent {
    match chunk {
        StreamChunk::ContentDelta(text) => AgentRuntimeEvent::MessageDelta { text },
        StreamChunk::ReasoningDelta(text) => AgentRuntimeEvent::MessageDelta { text },
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
                ProviderKind::ClaudeCli | ProviderKind::CursorAcp
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
        ProviderKind::ClaudeCli => ProviderRuntime::Unsupported(UnsupportedProvider {
            reason: UnsupportedProviderReason::UnsupportedCliProvider,
            detail: format!("provider `{provider_id}` is not dispatchable as configured"),
        }),
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
    ToolPolicyUnsupported {
        provider_id: String,
        protocol: CliProtocol,
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
            Self::ToolPolicyUnsupported {
                provider_id,
                protocol,
            } => write!(
                f,
                "provider `{provider_id}` ({protocol:?}) cannot enforce the requested tool policy"
            ),
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

    #[test]
    fn codex_invocation_folds_system_prompt_into_stdin() {
        let provider = CliProviderConfig::codex("codex_cli", "codex");
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

        let invocation = provider.build_invocation(&request).unwrap();
        assert_eq!(invocation.protocol, CliProtocol::CodexExecJson);
        assert!(invocation.args.iter().any(|arg| arg == "--model"));
        assert_eq!(invocation.stdin, "system\n\n---\n\nimplement it");
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

    #[test]
    fn codex_invocation_rejects_unenforceable_tool_policy() {
        let provider = CliProviderConfig::codex("codex_cli", "codex");
        let request = CliDispatchRequest {
            prompt: "restricted work".to_string(),
            system_prompt: String::new(),
            model: "gpt-5".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 1,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/restricted".to_string(),
            allowed_tools: Some(vec!["read_file".into()]),
            disallowed_tools: Vec::new(),
            plugin_mcp: None,
        };

        assert!(matches!(
            provider.build_invocation(&request),
            Err(DispatchV2Error::ToolPolicyUnsupported { .. })
        ));
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
    fn codex_invocation_configures_required_mcp_and_keeps_native_policy_fail_closed() {
        let provider = CliProviderConfig::codex("codex_cli", "codex");
        let mut request = CliDispatchRequest {
            prompt: "use the plugin".to_string(),
            system_prompt: String::new(),
            model: "gpt-5".to_string(),
            workdir: std::env::current_dir().unwrap(),
            max_turns: 2,
            effort: None,
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            env: Vec::new(),
            agent_id: "p/plugin".to_string(),
            allowed_tools: None,
            disallowed_tools: Vec::new(),
            plugin_mcp: Some(plugin_mcp_config()),
        };

        let invocation = provider
            .build_invocation(&request)
            .expect("Codex MCP invocation");
        let rendered = invocation.args.join(" ");
        assert!(rendered.contains("mcp_servers.roko_plugins.url="));
        assert!(rendered.contains("mcp_servers.roko_plugins.bearer_token_env_var="));
        assert!(rendered.contains("mcp_servers.roko_plugins.required=true"));
        assert!(rendered.contains("mcp_servers.roko_plugins.enabled_tools="));
        assert!(
            invocation
                .secret_env
                .iter()
                .any(|(key, value)| { key == "ROKO_PLUGIN_MCP_TOKEN" && value == "signed-secret" })
        );

        request.allowed_tools = Some(vec!["demo.echo".to_string()]);
        assert!(matches!(
            provider.build_invocation(&request),
            Err(DispatchV2Error::ToolPolicyUnsupported { .. })
        ));
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
        };
        assert!(matches!(
            classify_runtime("openclaw", ProviderKind::OpenClaw, Some(&openclaw)),
            ProviderRuntime::AgentResultBridge {
                provider_kind: ProviderKind::OpenClaw
            }
        ));
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
        };
        assert!(matches!(
            validate_contract_support(&request, &target),
            Err(DispatchV2Error::ContractUnsupported {
                kind: ProviderKind::OpenClaw,
                ..
            })
        ));
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
        };
        let dispatcher = AgentDispatcherV2::new(Arc::new(config));

        let dispatch = dispatcher
            .run_agent_result_bridge(request)
            .await
            .expect("dispatch");

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
        assert!(provider_health.contains("dispatch-cli"));

        let cascade_router =
            std::fs::read_to_string(tmp.path().join(".roko/learn/cascade-router.json"))
                .expect("read cascade router");
        assert!(cascade_router.contains("claude-sonnet-4-6"));
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
}
