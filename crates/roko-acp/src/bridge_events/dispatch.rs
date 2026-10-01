//! Provider dispatch: Anthropic Messages API and OpenAI-compatible backends.

use std::{
    collections::HashMap,
    future::poll_fn,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use roko_agent::ModelCallService;
use roko_agent::ReqwestPoster;
use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
use roko_agent::rate_limit::ProviderRateLimiter;
use roko_agent::tool_loop::backends::create_openai_compat_backend_with_limiter;
use roko_agent::tool_loop::{StopReason as ToolLoopStopReason, ToolLoop};
use roko_agent::translate::{OpenAiTranslator, StrictOpenAiTranslator, Translator};
use roko_core::agent::{ProviderKind, ResolvedModel, resolve_model};
use roko_core::config::DEFAULT_ACP_REQUEST_MS;
use roko_core::config::schema::{ModelProfile, RokoConfig};
use roko_core::defaults::DEFAULT_MAX_TOOL_ITERATIONS;
use roko_core::extension::CamelTaintLevel;
use roko_core::foundation::{
    ChatMessage, MessageRole, ModelCallRequest, ModelCaller, ModelInputBlock, ModelStreamEvent,
    TokenUsage,
};
use roko_core::tool::{
    NoopAuditSink, NoopMetricsSink, NoopTraceSink, ToolContext, ToolDef, ToolHandler,
    ToolPermission, VecToolRegistry,
};
use roko_learn::provider_health::ProviderHealthRegistry;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::builtin_tools::{acp_builtin_tools, filter_tools_by_ceiling};
use crate::session::CancelToken;
use crate::types::{StopReason, UsageInfo};

use super::{
    BridgeEventsError, CognitiveEvent, Result,
    context::model_input_messages_from_wire,
    emit_dispatch_failure, send_cognitive_event,
    tools::{
        AcpBuiltinHandlerResolver, AcpBuiltinToolHandler, AcpMcpHandlerResolver,
        AcpToolCancelToken, setup_session_mcp_tools, write_session_mcp_config,
    },
};

use roko_agent::safety::SafetyLayer;
use roko_agent::safety::capabilities::PluginTier;

// ── Anthropic Messages API dispatch ──────────────────────────────────

/// Dispatches a prompt via the Anthropic adapter through the shared model stream contract.
/// Used for explicitly AnthropicApi-configured models in ACP.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_anthropic_cognitive_task(
    session_id: &str,
    messages: &[serde_json::Value],
    model_key: &str,
    slug: &str,
    roko_config: &RokoConfig,
    provider_health: Arc<ProviderHealthRegistry>,
    rate_limiter: Arc<ProviderRateLimiter>,
    workdir: &Path,
    mcp_servers: &[crate::types::McpServerConfig],
    effort: &str,
    tools_enabled: bool,
    tool_capabilities: ToolPermission,
    // Agent role used to load the AgentContract for builtin tool permission
    // checks. Passed verbatim to run_anthropic_tool_loop.
    role: &str,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> Result<()> {
    let config = config_with_session_effort(roko_config, effort);
    let Some(model_call_config) = anthropic_model_call_config(&config, model_key, slug) else {
        emit_dispatch_failure(
            &event_sender,
            "Error: Anthropic provider is not configured for ACP dispatch.".to_string(),
        )
        .await;
        return Err(anyhow::anyhow!("Anthropic provider is not configured").into());
    };

    info!(
        session_id,
        model_key,
        slug,
        message_count = messages.len(),
        "dispatching prompt via ModelCaller stream"
    );

    if cancel_token.is_cancelled() {
        return Ok(());
    }

    // Anthropic tool-loop path: register both enabled builtins and any tools
    // attached through the ACP session's MCP servers.
    if (tools_enabled || !mcp_servers.is_empty())
        && run_anthropic_tool_loop(
            session_id,
            messages,
            model_key,
            slug,
            &config,
            workdir,
            mcp_servers,
            tools_enabled,
            tool_capabilities,
            None, // single-agent chat path: all tools allowed
            role,
            Arc::clone(&provider_health),
            Arc::clone(&rate_limiter),
            cancel_token.clone(),
            event_sender.clone(),
        )
        .await?
        .unwrap_or(false)
    {
        return Ok(());
    }

    // Fallback: plain streaming with no tool execution loop.
    let caller = ModelCallService::new(model_key.to_string())
        .with_config(model_call_config.clone())
        .with_working_dir(workdir)
        .with_immune_root(workdir)
        .with_provider_outcome_recorder(provider_health)
        .with_rate_limiter(rate_limiter);
    let tools = if tools_enabled {
        filter_tools_by_ceiling(acp_builtin_tools(), &tool_capabilities)
    } else {
        Vec::new()
    };
    let request = model_call_request_from_acp_messages(model_key, messages, tools)
        .map_err(BridgeEventsError::UnsupportedPromptContent)?;
    stream_model_call_to_cognitive_events(session_id, &caller, request, cancel_token, event_sender)
        .await
}

/// Anthropic-native builtin and session-MCP tool loop.
///
/// Uses the Anthropic Messages API tool format (`tool_use` / `tool_result` content
/// blocks) and the shared [`ToolLoop`] infrastructure. When the model emits
/// `tool_use` blocks the loop executes them via [`execute_acp_builtin_tool`],
/// appends `tool_result` blocks, and re-calls the model until it produces a
/// text-only response (or hits the 25-iteration cap).
///
/// Returns `Ok(Some(true))` if the loop handled the request, `Ok(Some(false))`
/// if the caller should fall through to the plain streaming path, and `Ok(None)`
/// if the Anthropic API key is not available.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_anthropic_tool_loop(
    session_id: &str,
    messages: &[serde_json::Value],
    model_key: &str,
    slug: &str,
    roko_config: &RokoConfig,
    workdir: &Path,
    mcp_servers: &[crate::types::McpServerConfig],
    tools_enabled: bool,
    tool_capabilities: ToolPermission,
    allowed_tools: Option<Vec<String>>,
    // Agent role for AgentContract tool permission checks in builtin handlers.
    role: &str,
    provider_health: Arc<ProviderHealthRegistry>,
    rate_limiter: Arc<ProviderRateLimiter>,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> Result<Option<bool>> {
    let model_profile = roko_config
        .effective_models()
        .get(model_key)
        .cloned()
        .unwrap_or_else(|| ModelProfile {
            slug: slug.to_string(),
            context_window: 200_000,
            ..Default::default()
        });
    let configured_provider_id =
        (!model_profile.provider.trim().is_empty()).then_some(model_profile.provider.as_str());
    let provider_entry = configured_provider_id
        .and_then(|provider_id| {
            roko_config
                .providers
                .get(provider_id)
                .filter(|provider| provider.kind == ProviderKind::AnthropicApi)
                .map(|provider| (provider_id, provider))
        })
        .or_else(|| {
            roko_config
                .providers
                .iter()
                .find(|(_, provider)| provider.kind == ProviderKind::AnthropicApi)
                .map(|(provider_id, provider)| (provider_id.as_str(), provider))
        });
    let provider_id = provider_entry
        .map(|(provider_id, _)| provider_id)
        .unwrap_or(model_key);

    // Resolve the API key from the exact provider named by the model profile
    // whenever possible, matching the ID used for limiter/outcome accounting.
    let api_key = provider_entry.and_then(|(_, provider)| provider.resolve_api_key());

    let Some(api_key) = api_key else {
        debug!(
            session_id,
            model_key, "Anthropic builtin tool loop skipped: no API key"
        );
        return Ok(None);
    };

    let timeout_ms = provider_entry
        .and_then(|(_, provider)| provider.timeout_ms)
        .unwrap_or(roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS);

    let mut tools = Vec::new();
    let mut handlers: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();
    if tools_enabled {
        tools = filter_tools_by_ceiling(acp_builtin_tools(), &tool_capabilities);
        for tool in &tools {
            handlers.insert(
                tool.name.clone(),
                Arc::new(AcpBuiltinToolHandler {
                    tool_name: tool.name.clone(),
                    session_id: session_id.to_string(),
                    workdir: workdir.to_path_buf(),
                    event_sender: event_sender.clone(),
                    role: role.to_string(),
                }),
            );
        }
    }

    if !mcp_servers.is_empty() {
        // ACP sessions use Sandboxed tier by default for external MCP servers.
        let (mcp_state, statuses) = setup_session_mcp_tools(
            session_id,
            mcp_servers,
            PluginTier::Sandboxed,
            role,
            event_sender.clone(),
        )
        .await;
        if !statuses.is_empty() {
            send_cognitive_event(&event_sender, CognitiveEvent::McpStatus { statuses }).await;
        }
        tools.extend(mcp_state.tools);
        handlers.extend(mcp_state.handlers);
    }

    if tools.is_empty() {
        return Ok(Some(false));
    }

    let registry = Arc::new(VecToolRegistry::from_tools(tools.clone()));
    let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpMcpHandlerResolver { handlers });
    let safety = acp_tool_safety(roko_config, role);
    let dispatcher = Arc::new(acp_tool_dispatcher(registry, resolver, safety));

    let (backend, translator) =
        roko_agent::provider::anthropic_api::tool_loop::create_anthropic_backend_with_runtime(
            api_key,
            slug,
            provider_id,
            timeout_ms,
            rate_limiter,
            provider_health,
        );
    let context_limit = usize::try_from(model_profile.context_window).unwrap_or(usize::MAX);

    let tool_loop = ToolLoop::new(translator, dispatcher, backend)
        .with_max_iterations(DEFAULT_MAX_TOOL_ITERATIONS)
        .with_context_token_limit(context_limit);

    let (chunk_sender, chunk_receiver) = mpsc::channel::<roko_agent::tool_loop::StreamEvent>(256);
    let forwarder = tokio::spawn(forward_tool_loop_stream_chunks(
        chunk_receiver,
        event_sender.clone(),
    ));

    let mut tool_context = ToolContext::new(
        workdir,
        Duration::from_millis(DEFAULT_ACP_REQUEST_MS),
        tool_capabilities,
        Arc::new(NoopAuditSink),
        Arc::new(NoopTraceSink),
        Arc::new(NoopMetricsSink),
        Arc::new(AcpToolCancelToken(cancel_token.clone())),
    )
    .with_immune_root(workdir)
    .with_taint_level(CamelTaintLevel::External)
    .with_env_passthrough(roko_config.agent.env_passthrough.clone());
    tool_context.allowed_tools = allowed_tools;

    let output = tool_loop
        .run_messages_streaming(messages.to_vec(), &tools, &tool_context, chunk_sender)
        .await;
    let _ = forwarder.await;

    let usage = usage_info_from_tool_loop_usage(&output.total_usage);
    match output.stop_reason {
        ToolLoopStopReason::Stop => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::MaxIterations => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::TokenChunk(format!(
                    "\n[stopped after {} tool rounds because the model kept requesting tools]",
                    DEFAULT_MAX_TOOL_ITERATIONS
                )),
            )
            .await;
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::MaxTokens,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::Cancelled => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::Cancelled,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::BudgetExhausted => {
            emit_dispatch_failure(
                &event_sender,
                "Error: Anthropic builtin tool loop stopped because the model-call budget was exhausted."
                    .to_string(),
            )
            .await;
            return Err(anyhow::anyhow!("ACP Anthropic builtin tool loop budget exhausted").into());
        }
        ToolLoopStopReason::BackendError(error) => {
            warn!(
                session_id,
                error = %error,
                "Anthropic builtin tool loop backend error, falling through to plain streaming"
            );
            return Ok(Some(false));
        }
    }

    Ok(Some(true))
}

pub(crate) fn anthropic_model_call_config(
    roko_config: &RokoConfig,
    model_key: &str,
    slug: &str,
) -> Option<RokoConfig> {
    let mut config = roko_config.clone();
    config.providers = roko_config.providers.clone();
    config.models = roko_config.effective_models();

    // 1. Prefer an existing AnthropicApi provider (NOT ClaudeCli — ACP IS the CLI subprocess).
    let anthropic_provider_id = config.providers.iter().find_map(|(id, provider)| {
        (provider.kind == ProviderKind::AnthropicApi).then(|| id.clone())
    })?;

    let mut profile = config
        .models
        .get(model_key)
        .or_else(|| config.models.values().find(|profile| profile.slug == slug))
        .cloned()
        .unwrap_or_else(|| ModelProfile {
            provider: anthropic_provider_id.clone(),
            slug: slug.to_string(),
            context_window: 200_000,
            tool_format: "anthropic_blocks".to_string(),
            ..Default::default()
        });
    profile.provider = anthropic_provider_id;
    profile.slug = slug.to_string();
    if profile.tool_format.trim().is_empty() {
        profile.tool_format = "anthropic_blocks".to_string();
    }

    config.models.insert(model_key.to_string(), profile.clone());
    config.models.entry(slug.to_string()).or_insert(profile);
    Some(config)
}

pub(crate) fn model_call_request_from_acp_messages(
    model_key: &str,
    messages: &[serde_json::Value],
    tools: Vec<ToolDef>,
) -> std::result::Result<ModelCallRequest, String> {
    let structured = model_input_messages_from_wire(messages)?;
    let input_messages = if structured.iter().any(|message| {
        message
            .content
            .iter()
            .any(|block| matches!(block, ModelInputBlock::Image { .. }))
    }) {
        structured
    } else {
        Vec::new()
    };
    Ok(ModelCallRequest {
        model: model_key.to_string(),
        messages: messages
            .iter()
            .filter_map(model_call_chat_message_from_acp)
            .collect(),
        input_messages,
        caller: Some("acp".to_string()),
        tools,
        ..Default::default()
    })
}

pub(crate) fn model_call_chat_message_from_acp(message: &serde_json::Value) -> Option<ChatMessage> {
    let role = match message.get("role").and_then(serde_json::Value::as_str)? {
        "system" => MessageRole::System,
        "assistant" => MessageRole::Assistant,
        "user" => MessageRole::User,
        _ => return None,
    };
    let content_val = message.get("content")?;
    let content = if let Some(s) = content_val.as_str() {
        s.to_string()
    } else if content_val.is_array() {
        // Multi-part content (e.g. text + image_url). Extract text parts for
        // the string-only ChatMessage; the original JSON messages array
        // preserves the full structure for backends that consume it directly.
        content_val
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| {
                        if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                            p.get("text").and_then(|t| t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    Some(ChatMessage { role, content })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ModelStreamForward {
    Continue,
    Completed,
}

#[derive(Default)]
pub(crate) struct ModelStreamForwardState {
    usage: Option<UsageInfo>,
}

async fn stream_model_call_to_cognitive_events<C>(
    session_id: &str,
    caller: &C,
    request: ModelCallRequest,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> Result<()>
where
    C: ModelCaller + ?Sized,
{
    let stream_result = tokio::select! {
        biased;
        _ = cancel_token.cancelled() => return Ok(()),
        result = caller.stream(request) => result,
    };

    let mut stream = match stream_result {
        Ok(stream) => stream,
        Err(error) => {
            emit_dispatch_failure(
                &event_sender,
                format!("Error: model stream failed: {error}"),
            )
            .await;
            return Err(anyhow::anyhow!("model stream failed: {error}").into());
        }
    };
    let mut state = ModelStreamForwardState::default();

    loop {
        let event = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => return Ok(()),
            event = poll_fn(|cx| stream.as_mut().poll_next(cx)) => event,
        };

        let Some(event) = event else {
            break;
        };

        if forward_model_stream_event(session_id, &event_sender, &mut state, event).await?
            == ModelStreamForward::Completed
        {
            return Ok(());
        }
    }

    send_cognitive_event(
        &event_sender,
        CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: state.usage,
        },
    )
    .await;
    Ok(())
}

pub(crate) async fn forward_model_stream_event(
    session_id: &str,
    event_sender: &mpsc::Sender<CognitiveEvent>,
    state: &mut ModelStreamForwardState,
    event: ModelStreamEvent,
) -> Result<ModelStreamForward> {
    match event {
        ModelStreamEvent::Started { model } => {
            debug!(session_id, model, "model stream started");
            Ok(ModelStreamForward::Continue)
        }
        ModelStreamEvent::ContentDelta { text } => {
            if !text.is_empty() {
                send_cognitive_event(event_sender, CognitiveEvent::TokenChunk(text)).await;
            }
            Ok(ModelStreamForward::Continue)
        }
        ModelStreamEvent::Usage { usage } => {
            state.usage = Some(usage_info_from_model_usage(&usage));
            Ok(ModelStreamForward::Continue)
        }
        ModelStreamEvent::Completed { stop_reason } => {
            send_cognitive_event(
                event_sender,
                CognitiveEvent::Complete {
                    stop_reason: acp_stop_reason_from_model(stop_reason.as_deref()),
                    usage: state.usage.clone(),
                },
            )
            .await;
            Ok(ModelStreamForward::Completed)
        }
        ModelStreamEvent::Failed { error } => {
            emit_dispatch_failure(event_sender, format!("Error: model stream failed: {error}"))
                .await;
            Err(anyhow::anyhow!("model stream failed: {error}").into())
        }
        ModelStreamEvent::Cancelled => {
            send_cognitive_event(
                event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::Cancelled,
                    usage: state.usage.clone(),
                },
            )
            .await;
            Ok(ModelStreamForward::Completed)
        }
        ModelStreamEvent::AttemptFailed { model, error } => {
            warn!(
                session_id,
                model,
                error = %error,
                "model stream attempt failed"
            );
            Ok(ModelStreamForward::Continue)
        }
    }
}

pub(crate) fn usage_info_from_model_usage(usage: &TokenUsage) -> UsageInfo {
    UsageInfo {
        total_tokens: if usage.total_tokens > 0 {
            usage.total_tokens
        } else {
            usage.input_tokens + usage.output_tokens
        },
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        thought_tokens: None,
        cached_read_tokens: None,
        cached_write_tokens: None,
    }
}

pub(crate) fn acp_stop_reason_from_model(stop_reason: Option<&str>) -> StopReason {
    match stop_reason
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "max_tokens" | "length" => StopReason::MaxTokens,
        "cancelled" | "canceled" => StopReason::Cancelled,
        "refusal" | "content_filter" => StopReason::Refusal,
        _ => StopReason::EndTurn,
    }
}

// ── OpenAI-compatible provider dispatch ──────────────────────────────

/// Dispatches a prompt through the shared model stream contract for
/// OpenAI-compatible provider selections (zhipu/GLM, moonshot/Kimi, OpenAI,
/// Perplexity, Ollama, etc.). Accepts a pre-built messages array with system
/// prompt and history.
///
/// `mcp_config_path` is the auto-discovered `.mcp.json` path resolved during
/// session creation (see [`crate::session::AcpSession::mcp_config_path`]). When
/// the session has no explicitly-attached MCP servers but a workspace `.mcp.json`
/// was found, this path is forwarded to the `ModelCallService` so Claude CLI
/// backends can load it via the `--mcp-config` flag.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_openai_compat_cognitive_task(
    session_id: &str,
    messages: &[serde_json::Value],
    model_key: &str,
    roko_config: &RokoConfig,
    provider_health: Arc<ProviderHealthRegistry>,
    rate_limiter: Arc<ProviderRateLimiter>,
    workdir: &Path,
    mcp_servers: &[crate::types::McpServerConfig],
    mcp_config_path: Option<&Path>,
    effort: &str,
    tools_enabled: bool,
    tool_capabilities: ToolPermission,
    // Agent role for AgentContract builtin tool permission checks.
    role: &str,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> Result<()> {
    let roko_config = config_with_session_effort(roko_config, effort);
    let resolved = resolve_model(&roko_config, model_key);

    info!(
        session_id,
        model_key,
        slug = %resolved.slug,
        provider_kind = ?resolved.provider_kind,
        message_count = messages.len(),
        "dispatching prompt via ModelCaller stream"
    );

    if cancel_token.is_cancelled() {
        return Ok(());
    }

    // Both tool loops check every call against the session role's contract.
    let tool_safety = acp_tool_safety(&roko_config, role);

    // MCP tool-loop path (OpenAI-compatible providers with MCP servers).
    if !mcp_servers.is_empty()
        && openai_compat_tool_loop_supported(resolved.provider_kind)
        && run_openai_compat_mcp_tool_loop(
            session_id,
            messages,
            &resolved,
            workdir,
            mcp_servers,
            Arc::clone(&rate_limiter),
            tool_capabilities,
            None, // single-agent chat path: all tools allowed
            role,
            tool_safety.clone(),
            cancel_token.clone(),
            event_sender.clone(),
        )
        .await?
    {
        return Ok(());
    }

    // Builtin tool-loop path: when tools are enabled and the provider supports
    // tool calls, run the ToolLoop with ACP builtin tools so the model can
    // invoke read_file, bash, etc. and receive results in a loop.
    if tools_enabled
        && openai_compat_tool_loop_supported(resolved.provider_kind)
        && run_openai_compat_builtin_tool_loop(
            session_id,
            messages,
            &resolved,
            workdir,
            Arc::clone(&rate_limiter),
            tool_capabilities,
            None, // single-agent chat path: all tools allowed
            role,
            tool_safety,
            &roko_config.agent.env_passthrough,
            cancel_token.clone(),
            event_sender.clone(),
        )
        .await?
    {
        return Ok(());
    }

    // Fallback: plain streaming with no tool execution loop.
    // Thread session MCP servers as a --mcp-config file for Claude CLI dispatch.
    // If no session-attached servers produced a written config, fall back to the
    // auto-discovered workspace `.mcp.json` path resolved at session creation time.
    let mut caller = ModelCallService::new(model_key.to_string())
        .with_config(roko_config.clone())
        .with_working_dir(workdir)
        .with_immune_root(workdir)
        .with_provider_outcome_recorder(provider_health)
        .with_rate_limiter(rate_limiter);
    let resolved_mcp_path = write_session_mcp_config(mcp_servers, workdir)
        .or_else(|| mcp_config_path.map(PathBuf::from));
    if let Some(mcp_path) = resolved_mcp_path {
        caller = caller.with_mcp_config(mcp_path);
    }
    let tools = if tools_enabled {
        filter_tools_by_ceiling(acp_builtin_tools(), &tool_capabilities)
    } else {
        Vec::new()
    };
    let request = model_call_request_from_acp_messages(model_key, messages, tools)
        .map_err(BridgeEventsError::UnsupportedPromptContent)?;
    stream_model_call_to_cognitive_events(session_id, &caller, request, cancel_token, event_sender)
        .await
}

/// Clone the workspace config with the session's effort level applied.
///
/// ## Effort dispatch flow
///
/// The ACP effort selection flows through the system as follows:
///
/// 1. **IDE -> ACP**: User picks effort in the status-bar dropdown (low/medium/high/max).
///    Stored in `SessionConfigState.effort`.
///
/// 2. **ACP -> config**: This function stamps the session effort onto
///    `RokoConfig.agent.default_effort` so that provider backends see it.
///
/// 3. **Config -> provider**: Each provider backend reads `agent.default_effort` to
///    decide how to pass effort/thinking to the upstream API:
///    - **Anthropic API**: maps to `thinking.budget_tokens` via the Anthropic model
///      caller (see `roko-agent/src/model_call/anthropic.rs`).
///    - **OpenAI-compat**: maps to `reasoning_effort` request field when the
///      provider supports it.
///    - **Claude CLI**: maps to the `--thinking` flag.
///
/// ## Known gap
///
/// Effort is passed through `agent.default_effort` as a string. Providers that
/// do not yet read this field will silently ignore it. See `.roko/GAPS.md` for
/// the tracking entry on per-provider effort wiring completeness.
pub(crate) fn config_with_session_effort(roko_config: &RokoConfig, effort: &str) -> RokoConfig {
    let mut config = roko_config.clone();
    let effort = effort.trim();
    if !effort.is_empty() {
        config.agent.default_effort = effort.to_owned();
    }
    config
}

/// The safety layer for an ACP tool loop: the configured policies plus the
/// `AgentContract` of the session's contract role (`acp_contract_role_for_mode`).
/// A role without a bundled contract gets the deny-all restricted fallback.
pub(crate) fn acp_tool_safety(roko_config: &RokoConfig, role: &str) -> SafetyLayer {
    SafetyLayer::from_config(roko_config).with_role(role)
}

/// The dispatcher for an ACP tool loop, which checks every builtin and MCP tool
/// call against `safety`. `ToolDispatcher::new` alone keeps the default layer,
/// which carries no role contract and whose empty allow-list denies every tool.
pub(crate) fn acp_tool_dispatcher(
    registry: Arc<VecToolRegistry>,
    resolver: Arc<dyn HandlerResolver>,
    safety: SafetyLayer,
) -> ToolDispatcher {
    ToolDispatcher::new(registry, resolver).with_safety(safety)
}

pub(crate) fn openai_compat_tool_loop_supported(provider_kind: ProviderKind) -> bool {
    matches!(
        provider_kind,
        ProviderKind::OpenAiCompat | ProviderKind::PerplexityApi | ProviderKind::CerebrasApi
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_openai_compat_mcp_tool_loop(
    session_id: &str,
    messages: &[serde_json::Value],
    resolved: &ResolvedModel,
    workdir: &Path,
    mcp_servers: &[crate::types::McpServerConfig],
    rate_limiter: Arc<ProviderRateLimiter>,
    tool_capabilities: ToolPermission,
    allowed_tools: Option<Vec<String>>,
    // The session's contract role, for MCP tools whose own name it forbids.
    role: &str,
    // Safety layer carrying the session role's contract (`acp_tool_safety`).
    safety: SafetyLayer,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> Result<bool> {
    let Some(provider) = resolved.provider_config.as_ref() else {
        emit_dispatch_failure(
            &event_sender,
            format!(
                "Error: session MCP tools require an explicitly configured provider for model '{}'.",
                resolved.model_key
            ),
        )
        .await;
        return Err(anyhow::anyhow!(
            "session MCP tools require explicit provider config for {}",
            resolved.model_key
        )
        .into());
    };
    let Some(model) = resolved.profile.as_ref() else {
        emit_dispatch_failure(
            &event_sender,
            format!(
                "Error: session MCP tools require an explicitly configured model profile for '{}'.",
                resolved.model_key
            ),
        )
        .await;
        return Err(anyhow::anyhow!(
            "session MCP tools require explicit model profile for {}",
            resolved.model_key
        )
        .into());
    };

    let (mcp_state, mcp_statuses) = setup_session_mcp_tools(
        session_id,
        mcp_servers,
        PluginTier::Sandboxed,
        role,
        event_sender.clone(),
    )
    .await;
    if !mcp_statuses.is_empty() {
        send_cognitive_event(
            &event_sender,
            CognitiveEvent::McpStatus {
                statuses: mcp_statuses,
            },
        )
        .await;
    }
    if mcp_state.tools.is_empty() {
        send_cognitive_event(
            &event_sender,
            CognitiveEvent::TokenChunk(
                "No MCP tools were discovered for this session; continuing without them.\n"
                    .to_string(),
            ),
        )
        .await;
        return Ok(false);
    }

    let translator: Arc<dyn Translator> = if provider.kind == ProviderKind::CerebrasApi {
        Arc::new(StrictOpenAiTranslator)
    } else {
        Arc::new(OpenAiTranslator)
    };
    let backend = create_openai_compat_backend_with_limiter(
        provider,
        model,
        Arc::new(ReqwestPoster::new()),
        rate_limiter,
    )
    .map_err(|error| anyhow::anyhow!("create ACP MCP tool-loop backend: {error}"))?;
    let registry = Arc::new(VecToolRegistry::from_tools(mcp_state.tools.clone()));
    let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpMcpHandlerResolver {
        handlers: mcp_state.handlers,
    });
    let dispatcher = Arc::new(acp_tool_dispatcher(registry, resolver, safety));
    let context_limit = usize::try_from(model.context_window).unwrap_or(usize::MAX);
    let tool_loop = ToolLoop::new(translator, dispatcher, backend)
        .with_max_iterations(DEFAULT_MAX_TOOL_ITERATIONS)
        .with_context_token_limit(context_limit);

    let (chunk_sender, chunk_receiver) = mpsc::channel::<roko_agent::tool_loop::StreamEvent>(256);
    let forwarder = tokio::spawn(forward_tool_loop_stream_chunks(
        chunk_receiver,
        event_sender.clone(),
    ));
    let mut tool_context = ToolContext::new(
        workdir,
        Duration::from_millis(DEFAULT_ACP_REQUEST_MS),
        tool_capabilities,
        Arc::new(NoopAuditSink),
        Arc::new(NoopTraceSink),
        Arc::new(NoopMetricsSink),
        Arc::new(AcpToolCancelToken(cancel_token.clone())),
    )
    .with_immune_root(workdir)
    .with_taint_level(CamelTaintLevel::External);
    tool_context.allowed_tools = allowed_tools;

    let output = tool_loop
        .run_messages_streaming(
            messages.to_vec(),
            &mcp_state.tools,
            &tool_context,
            chunk_sender,
        )
        .await;
    let _ = forwarder.await;

    let usage = usage_info_from_tool_loop_usage(&output.total_usage);
    match output.stop_reason {
        ToolLoopStopReason::Stop => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::MaxIterations => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::TokenChunk(format!(
                    "\n[stopped after {} tool rounds because the model kept requesting tools]",
                    DEFAULT_MAX_TOOL_ITERATIONS
                )),
            )
            .await;
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::MaxTokens,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::Cancelled => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::Cancelled,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::BudgetExhausted => {
            emit_dispatch_failure(
                &event_sender,
                "Error: MCP tool loop stopped because the model-call budget was exhausted."
                    .to_string(),
            )
            .await;
            return Err(anyhow::anyhow!("ACP MCP tool loop budget exhausted").into());
        }
        ToolLoopStopReason::BackendError(error) => {
            emit_dispatch_failure(
                &event_sender,
                format!("Error: MCP tool loop failed: {error}"),
            )
            .await;
            return Err(anyhow::anyhow!("ACP MCP tool loop failed: {error}").into());
        }
    }

    Ok(true)
}

/// Builtin-tool loop for OpenAI-compatible providers.
///
/// Mirrors [`run_openai_compat_mcp_tool_loop`] but wires the 8 ACP builtin tools
/// (read_file, write_file, edit_file, glob, grep, bash, ls, web_fetch) through the
/// same [`ToolLoop`] infrastructure. When the model emits `tool_use` blocks the loop
/// executes them via [`execute_acp_builtin_tool`], appends the results, and re-calls
/// the model until it produces a text-only response (or hits the 25-iteration cap).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_openai_compat_builtin_tool_loop(
    session_id: &str,
    messages: &[serde_json::Value],
    resolved: &ResolvedModel,
    workdir: &Path,
    rate_limiter: Arc<ProviderRateLimiter>,
    tool_capabilities: ToolPermission,
    allowed_tools: Option<Vec<String>>,
    // Agent role for AgentContract builtin tool permission checks.
    role: &str,
    // Safety layer carrying the session role's contract (`acp_tool_safety`).
    safety: SafetyLayer,
    // `[agent] env_passthrough`: variables the `bash` tool may inherit.
    env_passthrough: &[String],
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> Result<bool> {
    let Some(provider) = resolved.provider_config.as_ref() else {
        debug!(
            session_id,
            model_key = %resolved.model_key,
            "builtin tool loop skipped: no explicit provider config"
        );
        return Ok(false);
    };
    let Some(model) = resolved.profile.as_ref() else {
        debug!(
            session_id,
            model_key = %resolved.model_key,
            "builtin tool loop skipped: no explicit model profile"
        );
        return Ok(false);
    };

    // Build builtin tool definitions and handler map, filtered by the
    // session's capability ceiling so restricted modes (e.g. research)
    // cannot invoke write or exec tools.
    let tools = filter_tools_by_ceiling(acp_builtin_tools(), &tool_capabilities);
    if tools.is_empty() {
        return Ok(false);
    }

    let mut handlers: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();
    for tool in &tools {
        handlers.insert(
            tool.name.clone(),
            Arc::new(AcpBuiltinToolHandler {
                tool_name: tool.name.clone(),
                session_id: session_id.to_string(),
                workdir: workdir.to_path_buf(),
                event_sender: event_sender.clone(),
                role: role.to_string(),
            }),
        );
    }

    let translator: Arc<dyn Translator> = if provider.kind == ProviderKind::CerebrasApi {
        Arc::new(StrictOpenAiTranslator)
    } else {
        Arc::new(OpenAiTranslator)
    };
    let backend = create_openai_compat_backend_with_limiter(
        provider,
        model,
        Arc::new(ReqwestPoster::new()),
        rate_limiter,
    )
    .map_err(|error| anyhow::anyhow!("create ACP builtin tool-loop backend: {error}"))?;
    let registry = Arc::new(VecToolRegistry::from_tools(tools.clone()));
    let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpBuiltinHandlerResolver { handlers });
    let dispatcher = Arc::new(acp_tool_dispatcher(registry, resolver, safety));
    let context_limit = usize::try_from(model.context_window).unwrap_or(usize::MAX);
    let tool_loop = ToolLoop::new(translator, dispatcher, backend)
        .with_max_iterations(DEFAULT_MAX_TOOL_ITERATIONS)
        .with_context_token_limit(context_limit);

    let (chunk_sender, chunk_receiver) = mpsc::channel::<roko_agent::tool_loop::StreamEvent>(256);
    let forwarder = tokio::spawn(forward_tool_loop_stream_chunks(
        chunk_receiver,
        event_sender.clone(),
    ));
    let mut tool_context = ToolContext::new(
        workdir,
        Duration::from_millis(DEFAULT_ACP_REQUEST_MS),
        tool_capabilities,
        Arc::new(NoopAuditSink),
        Arc::new(NoopTraceSink),
        Arc::new(NoopMetricsSink),
        Arc::new(AcpToolCancelToken(cancel_token.clone())),
    )
    .with_immune_root(workdir)
    .with_taint_level(CamelTaintLevel::External)
    .with_env_passthrough(env_passthrough.to_vec());
    tool_context.allowed_tools = allowed_tools;

    let output = tool_loop
        .run_messages_streaming(messages.to_vec(), &tools, &tool_context, chunk_sender)
        .await;
    let _ = forwarder.await;

    let usage = usage_info_from_tool_loop_usage(&output.total_usage);
    match output.stop_reason {
        ToolLoopStopReason::Stop => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::MaxIterations => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::TokenChunk(format!(
                    "\n[stopped after {} tool rounds because the model kept requesting tools]",
                    DEFAULT_MAX_TOOL_ITERATIONS
                )),
            )
            .await;
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::MaxTokens,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::Cancelled => {
            send_cognitive_event(
                &event_sender,
                CognitiveEvent::Complete {
                    stop_reason: StopReason::Cancelled,
                    usage,
                },
            )
            .await;
        }
        ToolLoopStopReason::BudgetExhausted => {
            emit_dispatch_failure(
                &event_sender,
                "Error: builtin tool loop stopped because the model-call budget was exhausted."
                    .to_string(),
            )
            .await;
            return Err(anyhow::anyhow!("ACP builtin tool loop budget exhausted").into());
        }
        ToolLoopStopReason::BackendError(error) => {
            // Fall through to the plain streaming path so that providers that
            // don't fully support streaming tool loops (or mock servers in
            // tests) still work.
            warn!(
                session_id,
                error = %error,
                "builtin tool loop backend error, falling through to plain streaming"
            );
            return Ok(false);
        }
    }

    Ok(true)
}

pub(crate) async fn forward_tool_loop_stream_chunks(
    mut receiver: mpsc::Receiver<roko_agent::tool_loop::StreamEvent>,
    event_sender: mpsc::Sender<CognitiveEvent>,
) {
    use roko_agent::tool_loop::StreamEventKind;
    while let Some(event) = receiver.recv().await {
        match event.kind {
            StreamEventKind::TextDelta(text) if !text.is_empty() => {
                send_cognitive_event(&event_sender, CognitiveEvent::TokenChunk(text)).await;
            }
            StreamEventKind::ReasoningDelta(text) if !text.is_empty() => {
                send_cognitive_event(&event_sender, CognitiveEvent::ThinkingChunk(text)).await;
            }
            _ => {}
        }
    }
}

pub(crate) fn usage_info_from_tool_loop_usage(usage: &roko_core::Usage) -> Option<UsageInfo> {
    let input_tokens = u64::from(usage.input_tokens);
    let output_tokens = u64::from(usage.output_tokens);
    let cached_read_tokens = u64::from(usage.cache_read_tokens);
    let cached_write_tokens = u64::from(usage.cache_create_tokens);
    let total_tokens = u64::from(usage.total_tokens());
    (total_tokens > 0 || cached_read_tokens > 0).then_some(UsageInfo {
        total_tokens,
        input_tokens,
        output_tokens,
        thought_tokens: None,
        cached_read_tokens: (cached_read_tokens > 0).then_some(cached_read_tokens),
        cached_write_tokens: (cached_write_tokens > 0).then_some(cached_write_tokens),
    })
}
