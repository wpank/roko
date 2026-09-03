//! Cognitive event to session/update streaming.
//!
//! Bridges Roko's provider system (via `roko-agent`) to ACP
//! `session/update` notifications.
//! All cognitive workflow dispatch now goes through
//! [`crate::runner::run_with_workflow_engine`], which uses `ModelCallService`
//! for provider-agnostic model calls.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    future::poll_fn,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
use roko_agent::mcp::{McpClient, StdioTransport as McpStdioTransport, mcp_to_tool_def};
use roko_agent::rate_limit::{ProviderRateLimitSnapshot, ProviderRateLimiter};
use roko_agent::safety::{DispatchSafetyContext, SafetyLayer, ViolationSeverity};
use roko_agent::streaming::StreamChunk;
use roko_agent::tool_loop::backends::create_openai_compat_backend_with_limiter;
use roko_agent::tool_loop::{StopReason as ToolLoopStopReason, ToolLoop};
use roko_agent::translate::{OpenAiTranslator, StrictOpenAiTranslator, Translator};
use roko_agent::{ModelCallService, ReqwestPoster};
use roko_core::ContentHash;
use roko_core::DaimonPolicy;
use roko_core::agent::{AgentRole, ProviderKind, ResolvedModel, resolve_model};
#[cfg(test)]
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;
use roko_core::config::schema::{ModelProfile, RokoConfig};
#[cfg(test)]
use roko_core::defaults::DEFAULT_CONNECT_TIMEOUT_MS;
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use roko_core::defaults::{DEFAULT_MAX_TOOL_ITERATIONS, DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS};
use roko_core::extension::CamelTaintLevel;
use roko_core::foundation::{
    ChatMessage, MessageRole, ModelCallRequest, ModelCaller, ModelInputBlock, ModelInputMessage,
    ModelStreamEvent, TokenUsage, validate_model_input_messages,
};
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_core::tool::{
    NoopAuditSink, NoopMetricsSink, NoopTraceSink, ToolCall, ToolContext, ToolDef, ToolError,
    ToolHandler, ToolPermission, ToolResult, ToolSource, VecToolRegistry,
};
use roko_dreams::{load_dream_routing_advice, relevant_pattern_summaries};
use roko_learn::{
    cascade_router::CascadeRouter,
    cost_table::CostTable,
    efficiency::AgentEfficiencyEvent,
    episode_logger::{Episode, EpisodeLogger, Usage as EpUsage},
    model_router::RoutingContext,
    playbook::Playbook,
    prompt_experiment::{ExperimentStatus, ExperimentStore},
    provider_health::ProviderHealthRegistry,
};
use roko_neuro::{KnowledgeKind, KnowledgeQueryHit, KnowledgeTier};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncRead, AsyncWrite},
    sync::mpsc,
    task,
};
use tracing::{debug, error, info, warn};

use crate::builtin_tools::{acp_builtin_tools, filter_tools_by_ceiling, tool_permission_request};
use crate::event_forward::AcpEventForwarder;
use crate::knowledge::{DispatchKnowledge, append_context, query_dispatch_knowledge};
use crate::runner::run_with_workflow_engine;
use crate::{
    session::{AcpSession, CancelToken},
    transport::{StdioTransport, TransportError, TransportResult},
    types::{
        ClientCapabilities, ContentBlock, CostInfo, INTERNAL_ERROR, INVALID_PARAMS, JsonRpcMessage,
        McpInitStatus, McpServerStatus, PermissionAction, PermissionDecision, PermissionOptionKind,
        PermissionOutcome, PermissionResponse, PermissionToolCall, PlanEntry,
        RequestPermissionParams, SESSION_BUDGET_EXCEEDED, SESSION_BUSY, SessionCancelParams,
        SessionPromptParams, SessionPromptResult, SessionUpdate, StopReason, ToolCallKind,
        ToolCallStatus, UsageInfo, advertised_prompt_capabilities_for_model,
        unsupported_prompt_content,
    },
};

// ── Error types ──────────────────────────────────────────────────────

/// Errors produced while bridging cognitive events to ACP session updates.
#[derive(Debug, Error)]
pub enum BridgeEventsError {
    /// The target session already has an active prompt in flight.
    #[error("session '{0}' already has an active prompt")]
    SessionBusy(String),
    /// JSON serialization for an outbound session update failed.
    #[error("failed to serialize ACP session update: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Writing to the ACP stdio transport failed.
    #[error("failed to send ACP session update: {0}")]
    Transport(#[from] TransportError),
    /// The spawned cognitive task terminated unexpectedly.
    #[error("ACP cognitive task failed: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    /// A pipeline runner error.
    #[error("ACP pipeline error: {0}")]
    Pipeline(#[from] anyhow::Error),
    /// Prompt content is not supported by the selected model/dispatch path.
    #[error("unsupported ACP prompt content: {0}")]
    UnsupportedPromptContent(String),
    /// The persisted ACP session cost budget has been exhausted.
    #[error(
        "ACP session budget exceeded: spent ${accumulated_cost_usd:.6} of ${cost_budget_usd:.6} USD"
    )]
    BudgetExceeded {
        /// Configured session ceiling in USD.
        cost_budget_usd: f64,
        /// Persisted spend accumulated by completed efficiency events.
        accumulated_cost_usd: f64,
    },
}

impl BridgeEventsError {
    /// Returns a JSON-RPC error tuple when the failure maps to a client-visible ACP error.
    #[must_use]
    pub fn rpc_error(&self) -> Option<(i32, String)> {
        match self {
            Self::SessionBusy(session_id) => Some((
                SESSION_BUSY,
                format!("session '{session_id}' already has an active prompt"),
            )),
            Self::Serialize(e) => Some((INTERNAL_ERROR, format!("serialization error: {e}"))),
            Self::Transport(e) => Some((INTERNAL_ERROR, format!("transport error: {e}"))),
            Self::TaskJoin(e) => Some((INTERNAL_ERROR, format!("task failed: {e}"))),
            Self::Pipeline(e) => Some((INTERNAL_ERROR, format!("pipeline error: {e}"))),
            Self::UnsupportedPromptContent(message) => Some((INVALID_PARAMS, message.clone())),
            Self::BudgetExceeded {
                cost_budget_usd,
                accumulated_cost_usd,
            } => Some((
                SESSION_BUDGET_EXCEEDED,
                format!(
                    "ACP session budget exceeded: spent ${accumulated_cost_usd:.6} of ${cost_budget_usd:.6} USD; increase budget.max_plan_usd or start an unlimited session"
                ),
            )),
        }
    }
}

/// Result alias for ACP event bridge operations.
pub type Result<T> = std::result::Result<T, BridgeEventsError>;

/// Maximum assistant response bytes stored in one history turn.
const MAX_HISTORY_ASSISTANT_BYTES: usize = 10_240;

static CASCADE_ROUTER_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static EXPERIMENT_STORE_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

// ── Cognitive events ─────────────────────────────────────────────────

/// Events emitted by the cognitive loop and mapped to ACP session updates.
#[derive(Debug, Clone)]
pub enum CognitiveEvent {
    /// A streamed agent-visible text chunk.
    TokenChunk(String),
    /// A streamed internal reasoning chunk.
    ThinkingChunk(String),
    /// A tool call has started running.
    ToolCallStart {
        tool_call_id: String,
        title: String,
        kind: ToolCallKind,
        locations: Option<Vec<crate::types::ToolCallLocation>>,
    },
    /// A tool call has finished with rendered content.
    ToolCallComplete {
        tool_call_id: String,
        status: ToolCallStatus,
        content: Vec<ContentBlock>,
    },
    /// A plan update with structured entries (shown as progress in editor).
    PlanUpdate { entries: Vec<PlanEntry> },
    /// MCP server discovery results.
    McpStatus { statuses: Vec<McpServerStatus> },
    /// Prompt execution completed normally.
    Complete {
        stop_reason: StopReason,
        usage: Option<UsageInfo>,
    },
    /// Prompt execution failed before normal completion.
    Failure { message: String },
    /// Prompt execution stopped because the token budget was exhausted.
    MaxTokens,
    /// A spawned tool loop is requesting permission from the parent session.
    ///
    /// The reply channel carries a [`PermissionDecision`]; the parent loop
    /// should call [`PermissionReplyChannel::reply`] exactly once.  If the
    /// channel is dropped without a reply, the tool loop should treat it as
    /// `PermissionDecision::Reject` (fail-closed).
    PermissionRequest {
        /// What the tool loop wants to do and why.
        payload: PermissionRequestPayload,
        /// One-shot reply channel back to the requesting tool loop.
        reply: PermissionReplyChannel,
    },
}

/// Parameters describing what a tool loop wants permission to do.
#[derive(Debug, Clone)]
pub struct PermissionRequestPayload {
    /// The kind of action that needs authorisation (e.g. `FileEdit`, `TerminalCommand`).
    pub action: PermissionAction,
    /// Short human-readable title for the permission dialog (e.g. "Edit src/main.rs").
    pub title: String,
    /// Longer description of why the action is needed.
    pub detail: String,
}

/// Clone-safe wrapper around a `oneshot::Sender<PermissionDecision>`.
///
/// Because `oneshot::Sender` is not `Clone`, the sender is held behind
/// `Arc<Mutex<Option<…>>>`.  The first call to [`reply`](Self::reply)
/// takes the sender and sends the decision.  Subsequent calls (or calls
/// after a clone) return `false`.
#[derive(Debug, Clone)]
pub struct PermissionReplyChannel {
    inner: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<PermissionDecision>>>>,
}

impl PermissionReplyChannel {
    /// Create a new reply channel from a raw oneshot sender.
    pub fn new(sender: tokio::sync::oneshot::Sender<PermissionDecision>) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(Some(sender))),
        }
    }

    /// Send a decision back to the requesting tool loop.
    ///
    /// Returns `true` if the decision was delivered, `false` if the
    /// channel was already consumed or the receiver was dropped.
    pub fn reply(&self, decision: PermissionDecision) -> bool {
        let sender = self
            .inner
            .lock()
            .expect("PermissionReplyChannel mutex poisoned")
            .take();
        match sender {
            Some(tx) => tx.send(decision).is_ok(),
            None => false,
        }
    }

    /// Returns `true` if the reply channel has already been consumed.
    pub fn is_consumed(&self) -> bool {
        self.inner
            .lock()
            .expect("PermissionReplyChannel mutex poisoned")
            .is_none()
    }

    /// Returns `true` when the requesting tool is no longer waiting for a
    /// decision, for example because its dispatcher timeout elapsed.
    #[must_use]
    pub fn receiver_is_closed(&self) -> bool {
        self.inner
            .lock()
            .expect("PermissionReplyChannel mutex poisoned")
            .as_ref()
            .map(tokio::sync::oneshot::Sender::is_closed)
            .unwrap_or(true)
    }
}

// ── Stream events → editor ───────────────────────────────────────────

/// Result of streaming events: the prompt result, accumulated assistant text,
/// and any provider-reported usage.
pub struct StreamResult {
    pub prompt_result: SessionPromptResult,
    /// Accumulated assistant text from TokenChunk events.
    pub assistant_text: String,
    /// Usage reported by the provider, if any.
    pub usage: Option<UsageInfo>,
}

fn pricing_table() -> &'static CostTable {
    static TABLE: OnceLock<CostTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CostTable {
            models: std::collections::HashMap::new(),
        }
        .with_defaults()
    })
}

/// Calculate model cost from token counts.
///
/// Returns `None` when the model slug has no pricing row. Unknown pricing
/// stays unknown instead of collapsing to zero.
pub fn calculate_cost_for_model_slug(
    model_slug: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
) -> Option<f64> {
    let pricing = pricing_table().models.get(model_slug)?;
    Some(
        (input_tokens as f64 * pricing.input_per_m / 1_000_000.0)
            + (output_tokens as f64 * pricing.output_per_m / 1_000_000.0)
            + (cache_read_tokens as f64 * pricing.cache_read_per_m / 1_000_000.0),
    )
}

fn calculate_cost_without_cache_for_model_slug(
    model_slug: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
) -> Option<f64> {
    let pricing = pricing_table().models.get(model_slug)?;
    Some(
        (input_tokens as f64 * pricing.input_per_m / 1_000_000.0)
            + (output_tokens as f64 * pricing.output_per_m / 1_000_000.0)
            + (cache_read_tokens as f64 * pricing.input_per_m / 1_000_000.0),
    )
}

#[allow(clippy::too_many_arguments)]
async fn append_acp_episode(
    roko_config: &RokoConfig,
    workdir: &Path,
    session: &AcpSession,
    model_key: &str,
    prompt_text: &str,
    workflow_config: &str,
    is_pipeline_dispatch: bool,
    dispatch_started: Instant,
    stream_result: Option<&StreamResult>,
    task_error: Option<&str>,
    stream_error: Option<&str>,
    // When provided, overrides the pricing-table cost calculation with the
    // actual cost reported by the provider (e.g. from `WorkflowRunReport.cost`).
    cost_override: Option<f64>,
    // When cascade routing was used, retain both the selected config key and
    // maturity stage so the decision is inspectable via `roko learn episodes`.
    cascade_selection: Option<&AcpCascadeSelection>,
) {
    let resolved = resolve_model(roko_config, model_key);
    let elapsed = dispatch_started.elapsed();
    let input_hash = ContentHash::of(prompt_text.as_bytes()).to_hex();
    let output_source = stream_result
        .map(|sr| sr.assistant_text.as_str())
        .filter(|text| !text.is_empty())
        .or(task_error)
        .or(stream_error)
        .unwrap_or("");
    let output_hash = ContentHash::of(output_source.as_bytes()).to_hex();
    let mode = session.config_state.agent_mode.clone();
    let mut episode = Episode::new(mode.clone(), session.session_id.clone());

    episode.kind = if is_pipeline_dispatch {
        format!("acp-pipeline-{workflow_config}")
    } else {
        "acp-dispatch".to_string()
    };
    episode.agent_template = mode.clone();
    episode.model = resolved.slug.clone();
    episode.backend = resolved.provider_kind.label().to_string();
    episode.trigger_kind = if is_pipeline_dispatch {
        "acp_pipeline".to_string()
    } else {
        "acp_dispatch".to_string()
    };
    episode.trigger_signal_hash = input_hash.clone();
    episode.input_signal_hash = input_hash;
    episode.output_signal_hash = output_hash;
    episode.episode_id = episode.id.clone();
    episode.duration_secs = elapsed.as_secs_f64();
    let stream_usage = stream_result.and_then(|sr| sr.usage.as_ref());
    let mut usage = EpUsage {
        wall_ms: elapsed.as_millis() as u64,
        ..EpUsage::default()
    };
    if let Some(provider_usage) = stream_usage {
        let input_tokens = provider_usage.input_tokens;
        let output_tokens = provider_usage.output_tokens;
        let cached_read_tokens = provider_usage.cached_read_tokens.unwrap_or(0);
        usage.input_tokens = input_tokens;
        usage.output_tokens = output_tokens;
        usage.cache_read_tokens = cached_read_tokens;
        usage.cache_write_tokens = provider_usage.cached_write_tokens.unwrap_or(0);
        usage.cost_usd = cost_override.unwrap_or_else(|| {
            calculate_cost_for_model_slug(
                &resolved.slug,
                input_tokens,
                output_tokens,
                cached_read_tokens,
            )
            .unwrap_or(0.0)
        });
        usage.cost_usd_without_cache = cost_override.unwrap_or_else(|| {
            calculate_cost_without_cache_for_model_slug(
                &resolved.slug,
                input_tokens,
                output_tokens,
                cached_read_tokens,
            )
            .unwrap_or(usage.cost_usd)
        });
    }
    episode.usage = usage;
    episode.tokens_used = stream_usage.map(|usage| usage.total_tokens).unwrap_or(0);
    episode
        .extra
        .insert("entry_point".to_string(), serde_json::json!("acp"));
    episode
        .extra
        .insert("model".to_string(), serde_json::json!(resolved.slug));
    episode
        .extra
        .insert("mode".to_string(), serde_json::json!(mode));
    episode.extra.insert(
        "session_id".to_string(),
        serde_json::json!(session.session_id.clone()),
    );
    episode
        .extra
        .insert("workflow".to_string(), serde_json::json!(workflow_config));
    episode.extra.insert(
        "provider_kind".to_string(),
        serde_json::json!(resolved.provider_kind.label()),
    );
    if let Some(selection) = cascade_selection {
        episode.extra.insert(
            "cascade_selected_model".to_string(),
            serde_json::json!(selection.model_key),
        );
        episode.extra.insert(
            "cascade_stage".to_string(),
            serde_json::json!(selection.stage),
        );
    }

    let success = acp_dispatch_succeeded(stream_result, task_error, stream_error);
    episode.success = success;

    if !success {
        let failure_reason = task_error
            .or(stream_error)
            .map(str::to_string)
            .or_else(|| {
                stream_result.map(|sr| match sr.prompt_result.stop_reason {
                    StopReason::Cancelled => "cancelled".to_string(),
                    StopReason::MaxTokens => "max_tokens".to_string(),
                    StopReason::MaxTurnRequests => "max_turn_requests".to_string(),
                    StopReason::Refusal => "refusal".to_string(),
                    StopReason::EndTurn => "unknown failure".to_string(),
                })
            })
            .unwrap_or_else(|| "unknown failure".to_string());
        episode.failure_reason = Some(failure_reason);
    }

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    if let Some(parent) = episodes_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let logger = EpisodeLogger::new(&episodes_path);
    if let Err(err) = logger.append(&episode).await {
        error!(
            session_id = %session.session_id,
            error = %err,
            "failed to append ACP episode"
        );
    }

    // Spawn background distillation so the knowledge store learns from each ACP interaction.
    let distill_workdir = workdir.to_path_buf();
    let distill_model = roko_config.agent.default_model.clone();
    let distill_caller: Arc<dyn roko_core::foundation::ModelCaller> = Arc::new(
        ModelCallService::new(distill_model)
            .with_config(roko_config.clone())
            .with_working_dir(workdir)
            .with_immune_root(workdir),
    );
    roko_neuro::spawn_episode_distillation(distill_workdir, episode, Some(distill_caller));

    // Auto-dream consolidation: after enough episodes accumulate, spawn a
    // background dream cycle so patterns are extracted into `.roko/dreams/`.
    maybe_spawn_dream_consolidation(workdir, roko_config);
}

/// Default number of episodes that must accumulate since the last dream report
/// before a background dream consolidation is triggered.
const DREAM_EPISODE_THRESHOLD: usize = 10;

/// If the number of episodes since the last dream report exceeds
/// [`DREAM_EPISODE_THRESHOLD`], spawn a background dream consolidation.
/// This is fire-and-forget: failures are logged but never block the caller.
fn maybe_spawn_dream_consolidation(workdir: &Path, config: &RokoConfig) {
    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let dream_dir = workdir.join(".roko").join("dreams");
    let workdir = workdir.to_path_buf();

    // Count episodes since the last dream report.  Both helpers are
    // cheap (file I/O only, no LLM calls) so running them on the
    // current thread is acceptable.
    let last_dream_ts = roko_dreams::runner::load_latest_dream_report(&dream_dir)
        .ok()
        .flatten()
        .map(|r| r.completed_at);

    let text = match std::fs::read_to_string(&episodes_path) {
        Ok(t) => t,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(err) => {
            debug!(?err, "skipping dream check: could not read episode log");
            return;
        }
    };

    let episodes_since_dream = match last_dream_ts {
        Some(ts) => text
            .lines()
            .filter_map(|line| serde_json::from_str::<Episode>(line).ok())
            .filter(|ep| ep.timestamp > ts)
            .count(),
        None => text.lines().filter(|line| !line.trim().is_empty()).count(),
    };

    if episodes_since_dream < DREAM_EPISODE_THRESHOLD {
        return;
    }

    info!(
        episodes_since_dream,
        threshold = DREAM_EPISODE_THRESHOLD,
        "triggering background dream consolidation"
    );

    let dream_config = roko_dreams::DreamLoopConfig {
        auto_dream: true,
        idle_threshold_mins: 0,
        min_episodes_for_dream: 1,
        schedule: roko_dreams::DreamSchedulePolicy::default(),
        agent: roko_dreams::DreamAgentConfig {
            command: config
                .agent
                .command
                .clone()
                .unwrap_or_else(|| "claude".into()),
            args: Vec::new(),
            model: Some(config.agent.default_model.clone()),
            bare_mode: true,
            effort: "medium".to_string(),
            fallback_model: None,
            timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            env: Vec::new(),
        },
    };

    // `consolidate_now` internally calls `block_on`, so it must run on a
    // blocking thread rather than the async runtime.
    tokio::task::spawn_blocking(move || {
        let mut runner = roko_dreams::DreamRunner::new(&workdir, dream_config);
        if let Err(err) = runner.consolidate_now() {
            warn!(?err, "background dream consolidation failed");
        }
    });
}

/// Build the canonical efficiency event used for both learning telemetry and
/// persisted ACP session spend accounting.
fn acp_efficiency_event(
    session_id: &str,
    resolved: &ResolvedModel,
    dispatch_started: Instant,
    stream_result: Option<&StreamResult>,
    succeeded: bool,
    cost_override: Option<f64>,
) -> AgentEfficiencyEvent {
    let elapsed_ms = dispatch_started.elapsed().as_millis() as u64;
    let usage = stream_result.and_then(|sr| sr.usage.as_ref());

    let input_tokens = usage.map_or(0, |u| u.input_tokens);
    let output_tokens = usage.map_or(0, |u| u.output_tokens);
    let cached_read = usage.and_then(|u| u.cached_read_tokens).unwrap_or(0);
    let cached_write = usage.and_then(|u| u.cached_write_tokens).unwrap_or(0);

    let cost_usd = cost_override.unwrap_or_else(|| {
        calculate_cost_for_model_slug(&resolved.slug, input_tokens, output_tokens, cached_read)
            .unwrap_or(0.0)
    });
    let cost_usd_without_cache = cost_override.unwrap_or_else(|| {
        calculate_cost_without_cache_for_model_slug(
            &resolved.slug,
            input_tokens,
            output_tokens,
            cached_read,
        )
        .unwrap_or(cost_usd)
    });

    let outcome = if succeeded { "success" } else { "failure" }.to_string();

    AgentEfficiencyEvent {
        agent_id: session_id.to_string(),
        backend: resolved.provider_kind.label().to_string(),
        model: resolved.slug.clone(),
        model_used: resolved.slug.clone(),
        input_tokens,
        output_tokens,
        cache_read_tokens: cached_read,
        cache_write_tokens: cached_write,
        cost_usd,
        cost_usd_without_cache,
        wall_time_ms: elapsed_ms,
        duration_ms: elapsed_ms,
        outcome,
        timestamp: chrono::Utc::now().to_rfc3339(),
        ..AgentEfficiencyEvent::default()
    }
}

/// Emit an [`AgentEfficiencyEvent`] to `.roko/learn/efficiency.jsonl`.
///
/// This is fire-and-forget: the write is spawned on a blocking thread so it
/// never delays the response stream, and failures are logged but swallowed.
fn emit_acp_efficiency_event(workdir: &Path, event: AgentEfficiencyEvent) {
    let path = workdir.join(".roko").join("learn").join("efficiency.jsonl");

    task::spawn_blocking(move || {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let line = match serde_json::to_string(&event) {
            Ok(json) => json,
            Err(err) => {
                tracing::warn!(error = %err, "failed to serialize efficiency event");
                return;
            }
        };
        use std::io::Write;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path);
        match file {
            Ok(mut f) => {
                if let Err(err) = writeln!(f, "{line}") {
                    tracing::warn!(error = %err, "failed to write efficiency event");
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "failed to open efficiency.jsonl");
            }
        }
    });
}

fn acp_role_for_mode(mode: &str) -> AgentRole {
    match mode {
        "plan" => AgentRole::Strategist,
        "research" => AgentRole::Researcher,
        _ => AgentRole::Implementer,
    }
}

/// Intersect the ACP client's session declarations with the selected role's
/// permission ceiling. Interactive allow/always-allow decisions remain a
/// separate per-call gate in `AcpBuiltinToolHandler`.
fn derive_acp_tool_capabilities(
    mode: &str,
    client: &ClientCapabilities,
    has_session_mcp: bool,
    trusted_actions: &HashSet<PermissionAction>,
) -> ToolPermission {
    let role = acp_role_for_mode(mode).tool_permissions();
    let fs = client.fs.as_ref();
    let mcp = client.mcp_servers == Some(true) && has_session_mcp;
    let write = fs.map_or_else(
        || {
            trusted_actions.contains(&PermissionAction::FileCreate)
                || trusted_actions.contains(&PermissionAction::FileEdit)
        },
        |caps| caps.write_text_file,
    );
    let exec = client
        .terminal
        .unwrap_or_else(|| trusted_actions.contains(&PermissionAction::TerminalCommand));
    ToolPermission {
        read: role.read && (fs.is_some_and(|caps| caps.read_text_file) || mcp),
        write: role.write && write,
        exec: role.exec && exec,
        git: role.git
            && client
                .terminal
                .unwrap_or_else(|| trusted_actions.contains(&PermissionAction::GitOperation)),
        network: role.network && mcp,
    }
}

fn acp_routing_context(mode: &str, prompt: &str, effort: &str, workdir: &Path) -> RoutingContext {
    let _prompt_len = prompt.len();
    let task_category = if mode == "research" {
        TaskCategory::Research
    } else {
        TaskCategory::Implementation
    };

    let role = acp_role_for_mode(mode);

    // T4: Load DaimonState from disk so affect-based routing actually works.
    // Canonical path is .roko/daimon/affect.json; fall back to legacy
    // .roko/state/daimon.json for old workspaces that haven't migrated yet.
    // We read-only — the orchestrator is the sole writer of DaimonState.
    let daimon_policy = {
        let canonical = workdir.join(".roko").join("daimon").join("affect.json");
        let daimon_path = if canonical.exists() {
            canonical
        } else {
            workdir.join(".roko").join("state").join("daimon.json")
        };

        if daimon_path.exists() {
            std::fs::read_to_string(&daimon_path)
                .ok()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .and_then(|v| {
                    let confidence = v.get("state")?.get("confidence")?.as_f64()?;
                    let behavioral_state_str = v.get("state")?.get("behavioral_state")?.as_str()?;
                    use roko_core::BehavioralState;
                    let behavioral_state = match behavioral_state_str {
                        "struggling" => BehavioralState::Struggling,
                        "coasting" => BehavioralState::Coasting,
                        "exploring" => BehavioralState::Exploring,
                        "focused" => BehavioralState::Focused,
                        "resting" => BehavioralState::Resting,
                        _ => BehavioralState::Engaged,
                    };
                    Some(DaimonPolicy::new(confidence, behavioral_state))
                })
                .unwrap_or_default()
        } else {
            DaimonPolicy::default()
        }
    };

    RoutingContext {
        task_category,
        complexity: TaskComplexityBand::Standard,
        iteration: 0,
        role,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy,
        thinking_level: Some(effort.to_owned()).filter(|value| !value.trim().is_empty()),
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
    }
}

fn acp_dispatch_succeeded(
    stream_result: Option<&StreamResult>,
    task_error: Option<&str>,
    stream_error: Option<&str>,
) -> bool {
    task_error.is_none()
        && stream_error.is_none()
        && stream_result
            .map(|sr| matches!(sr.prompt_result.stop_reason, StopReason::EndTurn))
            .unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcpExperimentAssignment {
    experiment_id: String,
    variant_id: String,
    section_name: String,
    content: String,
    model_slug: Option<String>,
}

fn experiment_store_lock() -> std::sync::MutexGuard<'static, ()> {
    EXPERIMENT_STORE_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Select one running experiment deterministically for this ACP role.
///
/// The persisted map is intentionally sorted before selection so HashMap
/// iteration order cannot change which experiment receives an ACP turn.
fn assign_acp_experiment(path: &Path, mode: &str) -> Option<AcpExperimentAssignment> {
    let _guard = experiment_store_lock();
    let store = ExperimentStore::load_or_new(path);
    let role = acp_role_for_mode(mode).label();
    let mut experiments = store
        .experiments()
        .values()
        .filter(|experiment| experiment.status == ExperimentStatus::Running)
        .filter(|experiment| {
            experiment.role.as_deref().is_none_or(|configured| {
                configured.eq_ignore_ascii_case(mode) || configured.eq_ignore_ascii_case(role)
            })
        })
        .collect::<Vec<_>>();
    experiments.sort_by(|left, right| left.experiment_id.cmp(&right.experiment_id));
    let experiment = experiments.first()?;
    let variant = experiment.assign_variant()?;
    Some(AcpExperimentAssignment {
        experiment_id: experiment.experiment_id.clone(),
        variant_id: variant.id.clone(),
        section_name: experiment.section_name.clone(),
        content: variant.content.clone(),
        model_slug: variant.slug.clone().filter(|slug| !slug.trim().is_empty()),
    })
}

fn experiment_model_key(
    config: &RokoConfig,
    assignment: &AcpExperimentAssignment,
) -> Option<String> {
    let requested = assignment.model_slug.as_deref()?.trim();
    let models = config.effective_models();
    if models.contains_key(requested) {
        return Some(requested.to_string());
    }
    let mut matching = models
        .iter()
        .filter(|(_, profile)| profile.slug.trim() == requested)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    matching.sort();
    matching.into_iter().next()
}

fn applicable_acp_experiment(
    config: &RokoConfig,
    current_model_key: &str,
    model_selection_explicit: bool,
    assignment: Option<AcpExperimentAssignment>,
) -> (Option<AcpExperimentAssignment>, Option<String>) {
    let Some(assignment) = assignment else {
        return (None, None);
    };
    if assignment.model_slug.is_none() {
        return (Some(assignment), None);
    }

    let Some(candidate) = experiment_model_key(config, &assignment) else {
        warn!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            model_slug = ?assignment.model_slug,
            "skipping ACP experiment variant with unresolved model"
        );
        return (None, None);
    };
    if model_selection_explicit && resolve_model(config, current_model_key).model_key != candidate {
        debug!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            experiment_model = %candidate,
            selected_model = current_model_key,
            "skipping ACP model experiment because the session model was explicitly selected"
        );
        return (None, None);
    }

    let model_override = (!model_selection_explicit).then_some(candidate);
    (Some(assignment), model_override)
}

fn render_experiment_context(assignment: &AcpExperimentAssignment) -> String {
    format!(
        "ACP experiment `{}` variant `{}` for section `{}`:\n{}",
        assignment.experiment_id,
        assignment.variant_id,
        assignment.section_name,
        assignment.content.trim()
    )
}

fn record_acp_experiment_outcome(
    path: &Path,
    assignment: &AcpExperimentAssignment,
    success: bool,
) -> std::io::Result<()> {
    let _guard = experiment_store_lock();
    ExperimentStore::transaction(path, |store| {
        if !store.record_outcome_for_experiment(
            &assignment.experiment_id,
            &assignment.variant_id,
            success,
        ) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "experiment '{}' variant '{}' disappeared before outcome recording",
                    assignment.experiment_id, assignment.variant_id
                ),
            ));
        }
        store.record_metric(
            &assignment.experiment_id,
            &assignment.variant_id,
            if success { 1.0 } else { 0.0 },
        );
        Ok(())
    })
}

fn cascade_router_model_slugs(roko_config: &RokoConfig, resolved_slug: &str) -> Vec<String> {
    let mut model_slugs = roko_config.models.keys().cloned().collect::<Vec<_>>();
    if model_slugs.is_empty() {
        model_slugs.push(resolved_slug.to_owned());
    }
    model_slugs.sort();
    model_slugs
}

fn acp_model_providers(roko_config: &RokoConfig, model_keys: &[String]) -> HashMap<String, String> {
    let models = roko_config.effective_models();
    model_keys
        .iter()
        .filter_map(|key| {
            models
                .get(key)
                .or_else(|| models.values().find(|profile| profile.slug == *key))
                .map(|profile| (key.clone(), profile.provider.clone()))
        })
        .collect()
}

fn provider_near_rate_limit(snapshot: &ProviderRateLimitSnapshot) -> bool {
    let rpm_pressured = snapshot.rpm_limit > 0
        && snapshot.rpm_used.saturating_mul(5) >= u64::from(snapshot.rpm_limit).saturating_mul(4);
    let tpm_pressured = snapshot.tpm_limit > 0
        && snapshot.tpm_used.saturating_mul(5) >= snapshot.tpm_limit.saturating_mul(4);
    rpm_pressured || tpm_pressured
}

fn rate_aware_model_candidates(
    model_keys: Vec<String>,
    model_providers: &HashMap<String, String>,
    snapshots: &[ProviderRateLimitSnapshot],
) -> (Vec<String>, Vec<String>) {
    let pressured = snapshots
        .iter()
        .filter(|snapshot| provider_near_rate_limit(snapshot))
        .map(|snapshot| snapshot.provider_id.clone())
        .collect::<HashSet<_>>();
    let preferred = model_keys
        .iter()
        .filter(|key| {
            model_providers
                .get(key.as_str())
                .is_none_or(|provider| !pressured.contains(provider))
        })
        .cloned()
        .collect::<Vec<_>>();
    if preferred.is_empty() {
        (model_keys, pressured.into_iter().collect())
    } else {
        (preferred, pressured.into_iter().collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcpCascadeSelection {
    model_key: String,
    stage: String,
}

struct AcpCascadeRequest<'a> {
    workdir: &'a Path,
    roko_config: &'a RokoConfig,
    mode: &'a str,
    prompt: &'a str,
    effort: &'a str,
    resolved_slug: &'a str,
    model_selection_explicit: bool,
    provider_health: &'a ProviderHealthRegistry,
    rate_limiter: &'a ProviderRateLimiter,
}

fn acp_cascade_selection_enabled() -> bool {
    std::env::var_os("ROKO_ACP_CASCADE_SELECT").is_some_and(|value| value == "1")
}

/// Select a model for an ACP session using the cascade router.
///
/// Returns a model key and routing stage when:
/// 1. `ROKO_ACP_CASCADE_SELECT=1` exactly.
/// 2. The cascade router state file exists at `workdir/.roko/learn/cascade-router.json`.
///
/// Returns `None` (leaving model selection to the caller) when the env var is
/// absent, disabled, or the router file does not yet exist (cold start).
fn cascade_select_model(request: AcpCascadeRequest<'_>) -> Option<AcpCascadeSelection> {
    let AcpCascadeRequest {
        workdir,
        roko_config,
        mode,
        prompt,
        effort,
        resolved_slug,
        model_selection_explicit,
        provider_health,
        rate_limiter,
    } = request;
    if model_selection_explicit || !acp_cascade_selection_enabled() {
        return None;
    }

    let router_path = workdir
        .join(".roko")
        .join("learn")
        .join("cascade-router.json");

    if !router_path.exists() {
        return None;
    }

    let model_slugs = cascade_router_model_slugs(roko_config, resolved_slug);
    let initial_candidate_count = model_slugs.len();
    let model_providers = acp_model_providers(roko_config, &model_slugs);
    let (model_slugs, mut pressured_providers) =
        rate_aware_model_candidates(model_slugs, &model_providers, &rate_limiter.snapshot());
    let rate_candidates_filtered = model_slugs.len() < initial_candidate_count;
    pressured_providers.sort();
    let candidate_providers = model_slugs
        .iter()
        .filter_map(|model| model_providers.get(model))
        .cloned()
        .collect::<HashSet<_>>();
    let mut degraded_providers = candidate_providers
        .iter()
        .filter(|provider| !provider_health.is_healthy(provider))
        .cloned()
        .collect::<Vec<_>>();
    degraded_providers.sort();
    let has_healthy_provider = candidate_providers
        .iter()
        .any(|provider| provider_health.is_healthy(provider));
    let router = CascadeRouter::load_or_new(&router_path, model_slugs);
    let ctx = acp_routing_context(mode, prompt, effort, workdir);
    let cascade_model =
        router.route_with_health_scored(&ctx, provider_health, &model_providers, None, None);
    if rate_candidates_filtered {
        info!(
            selected_model = %cascade_model.primary.slug,
            providers = ?pressured_providers,
            reason = "RPM/TPM utilization at or above 80%",
            "ACP adaptive routing deprioritized rate-pressured providers"
        );
    } else if !pressured_providers.is_empty() {
        warn!(
            selected_model = %cascade_model.primary.slug,
            providers = ?pressured_providers,
            reason = "all ACP candidates are near RPM/TPM limits",
            "ACP adaptive routing retained least-bad rate-pressured candidates"
        );
    }
    if !degraded_providers.is_empty() && has_healthy_provider {
        info!(
            selected_model = %cascade_model.primary.slug,
            providers = ?degraded_providers,
            reason = "canonical provider circuit health",
            "ACP adaptive routing deprioritized degraded providers"
        );
    } else if !degraded_providers.is_empty() {
        warn!(
            selected_model = %cascade_model.primary.slug,
            providers = ?degraded_providers,
            reason = "all ACP candidates have open provider circuits",
            "ACP adaptive routing retained least-bad degraded candidates"
        );
    }
    Some(AcpCascadeSelection {
        model_key: cascade_model.primary.slug,
        stage: cascade_model.stage.label().to_owned(),
    })
}

fn resolve_acp_dispatch_model(
    roko_config: &RokoConfig,
    requested_model_key: &str,
    cascade_selection: Option<AcpCascadeSelection>,
) -> (ResolvedModel, String, Option<AcpCascadeSelection>) {
    let requested = resolve_model(roko_config, requested_model_key);
    let requested_dispatch_key = requested.model_key.clone();
    let Some(selection) = cascade_selection else {
        return (requested, requested_dispatch_key, None);
    };

    let selected = resolve_model(roko_config, &selection.model_key);
    if selected.profile.is_none() {
        warn!(
            requested_model = requested_model_key,
            selected_model = %selection.model_key,
            stage = %selection.stage,
            "cascade router selected an unconfigured ACP model; retaining requested model"
        );
        return (requested, requested_dispatch_key, None);
    }

    let dispatch_model_key = selected.model_key.clone();
    (selected, dispatch_model_key, Some(selection))
}

fn compute_acp_reward(success: bool, wall_ms: u64, output_tokens: Option<u64>) -> f64 {
    if !success {
        return 0.0;
    }

    let latency_bonus = if wall_ms < 5_000 {
        0.15
    } else if wall_ms < 15_000 {
        0.05
    } else {
        0.0
    };
    let token_bonus = match output_tokens {
        Some(tokens) if tokens < 2_000 => 0.05,
        Some(tokens) if tokens < 5_000 => 0.02,
        _ => 0.0,
    };

    let score: f64 = 0.8 + latency_bonus + token_bonus;
    score.min(1.0)
}

fn record_cascade_observation(
    router_path: PathBuf,
    model_slug: String,
    routing_ctx: RoutingContext,
    success: bool,
    wall_ms: u64,
    output_tokens: Option<u64>,
    model_slugs: Vec<String>,
) -> task::JoinHandle<()> {
    task::spawn_blocking(move || {
        let _guard = CASCADE_ROUTER_IO_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let router = CascadeRouter::load_or_new(&router_path, model_slugs);

        let Some(model_idx) = router.model_index_for_slug(&model_slug) else {
            debug!(
                model = %model_slug,
                "skipping cascade observation: model not in router arms"
            );
            return;
        };

        let context_vec = routing_ctx.to_features();
        let reward = compute_acp_reward(success, wall_ms, output_tokens);
        router.observe(context_vec, model_idx, reward);

        if let Err(error) = router.save(&router_path) {
            warn!(
                path = %router_path.display(),
                error = %error,
                "failed to persist cascade router after ACP observation"
            );
        }
    })
}

/// Truncate text to a session title: up to `max_len` chars, word-boundary aware.
fn truncate_to_title(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    // Take first line only.
    let first_line = trimmed.lines().next().unwrap_or(trimmed);
    if first_line.len() <= max_len {
        return first_line.to_owned();
    }
    let mut end = max_len;
    // Back up to last word boundary.
    while end > 0 && !first_line.is_char_boundary(end) {
        end -= 1;
    }
    // Try to find a space to break at a word boundary.
    if let Some(space_pos) = first_line[..end].rfind(' ') {
        format!("{}...", &first_line[..space_pos])
    } else {
        format!("{}...", &first_line[..end])
    }
}

fn truncate_assistant_history(text: &str) -> String {
    if text.len() <= MAX_HISTORY_ASSISTANT_BYTES {
        return text.to_owned();
    }

    let mut end = MAX_HISTORY_ASSISTANT_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = String::with_capacity(end + "...[truncated]".len());
    truncated.push_str(&text[..end]);
    truncated.push_str("...[truncated]");
    truncated
}

/// Sends a `session/request_permission` request to the editor and waits for the decision.
///
/// Returns `PermissionDecision::Allow` if the action is already pre-granted.
/// Returns `PermissionDecision::Reject` on timeout or error, which is the safe default.
///
/// If the user chooses `AlwaysAllow`, the decision is remembered on the session and persisted
/// to `.roko/trust/permissions.json`.
pub async fn request_permission<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    workdir: &Path,
    action: PermissionAction,
    title: &str,
    detail: &str,
) -> PermissionDecision
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if session.is_pre_granted(&action) {
        debug!(
            session_id = %session.session_id,
            action = ?action,
            "permission pre-granted (always-allow)"
        );
        return PermissionDecision::Allow;
    }

    debug!(
        session_id = %session.session_id,
        action = ?action,
        title = %title,
        detail = %detail,
        "requesting permission from editor"
    );

    let tool_call_id = format!("perm-{}", uuid::Uuid::new_v4());
    let params = serde_json::to_value(RequestPermissionParams {
        session_id: session.session_id.clone(),
        tool_call: PermissionToolCall {
            tool_call_id: tool_call_id.clone(),
            title: format!("{title}: {detail}"),
        },
        options: PermissionOptionKind::standard_options(),
    })
    .unwrap_or_else(|error| {
        warn!(
            session_id = %session.session_id,
            action = ?action,
            error = %error,
            "failed to serialize permission request; sending null payload"
        );
        serde_json::Value::Null
    });

    let mut request_transport = transport.clone();
    let request_future = request_transport.send_request("session/request_permission", params);
    tokio::pin!(request_future);
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(300));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            biased;
            response = &mut request_future => {
                match response {
                    Ok(json_response) => {
                        if let Some(error) = json_response.error.as_ref() {
                            warn!(
                                session_id = %session.session_id,
                                action = ?action,
                                code = error.code,
                                message = %error.message,
                                "permission request returned an error; defaulting to Reject"
                            );
                            return PermissionDecision::Reject;
                        }

                        let decision = json_response
                            .result
                            .as_ref()
                            .and_then(|value| serde_json::from_value::<PermissionResponse>(value.clone()).ok())
                            .map(|response| match response.outcome {
                                PermissionOutcome::Selected { ref option_id } => {
                                    PermissionOptionKind::decision_from_option_id(option_id)
                                        .unwrap_or(PermissionDecision::Reject)
                                }
                                PermissionOutcome::Cancelled => PermissionDecision::Reject,
                            })
                            .unwrap_or_else(|| {
                                warn!(
                                    session_id = %session.session_id,
                                    action = ?action,
                                    "permission response could not be parsed; defaulting to Reject"
                                );
                                PermissionDecision::Reject
                            });

                        if matches!(decision, PermissionDecision::AlwaysAllow) {
                            session.grant_always_allow(action.clone());
                            match AcpSession::save_workspace_trust(
                                workdir,
                                &session.always_allowed,
                            ) {
                                Ok(()) => info!(
                                    session_id = %session.session_id,
                                    action = ?action,
                                    "permission permanently granted (always-allow persisted)"
                                ),
                                Err(error) => warn!(
                                    session_id = %session.session_id,
                                    action = ?action,
                                    error = %error,
                                    "always-allow retained for this session but workspace persistence failed"
                                ),
                            }
                        }

                        return decision;
                    }
                    Err(error) => {
                        warn!(
                            session_id = %session.session_id,
                            action = ?action,
                            error = %error,
                            "permission request transport error; defaulting to Reject"
                        );
                        return PermissionDecision::Reject;
                    }
                }
            }
            inbound = transport.read_message() => {
                match inbound {
                    Ok(Some(JsonRpcMessage::Response(response))) => {
                        transport.handle_incoming_response(response);
                    }
                    Ok(Some(JsonRpcMessage::Notification(notification))) => {
                        if notification.method == "session/cancel" {
                            match serde_json::from_value::<SessionCancelParams>(
                                notification.params.unwrap_or(serde_json::Value::Null),
                            ) {
                                Ok(params) if params.session_id == session.session_id => {
                                    session.cancel_token.cancel();
                                    warn!(
                                        session_id = %session.session_id,
                                        "permission request cancelled by client; defaulting to Reject"
                                    );
                                    return PermissionDecision::Reject;
                                }
                                Ok(_) => {}
                                Err(error) => {
                                    warn!(
                                        session_id = %session.session_id,
                                        error = %error,
                                        "received malformed session/cancel while waiting for permission"
                                    );
                                }
                            }
                        } else {
                            debug!(
                                session_id = %session.session_id,
                                method = %notification.method,
                                "ignoring notification while waiting for permission"
                            );
                        }
                    }
                    Ok(Some(JsonRpcMessage::Request(request))) => {
                        warn!(
                            session_id = %session.session_id,
                            method = %request.method,
                            "ignoring inbound request while waiting for permission"
                        );
                    }
                    Ok(None) => {
                        warn!(
                            session_id = %session.session_id,
                            "ACP client disconnected while waiting for permission"
                        );
                        return PermissionDecision::Reject;
                    }
                    Err(error) => {
                        warn!(
                            session_id = %session.session_id,
                            error = %error,
                            "failed to read inbound message while waiting for permission; defaulting to Reject"
                        );
                        return PermissionDecision::Reject;
                    }
                }
            }
            _ = &mut timeout => {
                warn!(
                    session_id = %session.session_id,
                    action = ?action,
                    "permission request timed out after 5 minutes; defaulting to Reject"
                );
                return PermissionDecision::Reject;
            }
        }
    }
}

/// Runs the editor round-trip while respecting both the enclosing prompt and
/// tool-handler lifetimes. The handler-side receiver disappears when the tool
/// dispatcher times out, so observing that state prevents the parent stream
/// from waiting for the longer editor timeout after there is nobody to answer.
async fn request_permission_for_event<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    workdir: &Path,
    payload: &PermissionRequestPayload,
    reply: &PermissionReplyChannel,
    cancel_token: &CancelToken,
) -> PermissionDecision
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let session_id = session.session_id.clone();
    let request = request_permission(
        transport,
        session,
        workdir,
        payload.action.clone(),
        &payload.title,
        &payload.detail,
    );
    tokio::pin!(request);

    loop {
        if reply.receiver_is_closed() {
            warn!(
                session_id = %session_id,
                action = ?payload.action,
                "permission requester stopped waiting; abandoning editor request"
            );
            return PermissionDecision::Reject;
        }

        tokio::select! {
            decision = &mut request => return decision,
            _ = cancel_token.cancelled() => {
                warn!(
                    session_id = %session_id,
                    action = ?payload.action,
                    "ACP prompt cancelled while waiting for editor permission"
                );
                return PermissionDecision::Reject;
            }
            () = tokio::time::sleep(Duration::from_millis(25)) => {}
        }
    }
}

/// Maps cognitive events to ACP `session/update` notifications and streams them to the editor.
/// Returns both the prompt result and the accumulated assistant response text.
pub async fn stream_events_to_editor<R, W>(
    transport: &mut StdioTransport<R, W>,
    session_id: &str,
    session: &mut AcpSession,
    workdir: &Path,
    mut events: mpsc::Receiver<CognitiveEvent>,
    cancel_token: &CancelToken,
) -> Result<StreamResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut assistant_text = String::new();
    let event_forwarder = AcpEventForwarder::from_env(session_id);

    loop {
        enum StreamAction {
            Cancelled,
            Event(Option<CognitiveEvent>),
            Inbound(TransportResult<Option<JsonRpcMessage>>),
        }

        let action = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => StreamAction::Cancelled,
            maybe_event = events.recv() => StreamAction::Event(maybe_event),
            inbound = transport.read_message() => StreamAction::Inbound(inbound),
        };

        match action {
            StreamAction::Cancelled => {
                debug!(session_id, "ACP prompt cancelled while streaming events");
                return Ok(StreamResult {
                    prompt_result: SessionPromptResult {
                        stop_reason: StopReason::Cancelled,
                    },
                    assistant_text,
                    usage: None,
                });
            }
            StreamAction::Event(maybe_event) => {
                let Some(event) = maybe_event else {
                    warn!(
                        session_id,
                        "ACP event stream closed without an explicit completion event"
                    );
                    let stop_reason = if cancel_token.is_cancelled() {
                        StopReason::Cancelled
                    } else {
                        StopReason::EndTurn
                    };
                    return Ok(StreamResult {
                        prompt_result: SessionPromptResult { stop_reason },
                        assistant_text,
                        usage: None,
                    });
                };

                if let Some(forwarder) = event_forwarder.as_ref() {
                    forwarder.forward(&event);
                }

                match event {
                    CognitiveEvent::Complete { stop_reason, usage } => {
                        return Ok(StreamResult {
                            prompt_result: SessionPromptResult { stop_reason },
                            assistant_text,
                            usage,
                        });
                    }
                    CognitiveEvent::Failure { message } => {
                        let update = dispatch_failure_update(message);
                        send_session_update(transport, session_id, update).await?;
                        return Ok(StreamResult {
                            prompt_result: SessionPromptResult {
                                stop_reason: StopReason::EndTurn,
                            },
                            assistant_text,
                            usage: None,
                        });
                    }
                    CognitiveEvent::MaxTokens => {
                        return Ok(StreamResult {
                            prompt_result: SessionPromptResult {
                                stop_reason: StopReason::MaxTokens,
                            },
                            assistant_text,
                            usage: None,
                        });
                    }
                    CognitiveEvent::PermissionRequest { payload, reply } => {
                        let decision = request_permission_for_event(
                            transport,
                            session,
                            workdir,
                            &payload,
                            &reply,
                            cancel_token,
                        )
                        .await;
                        if !reply.reply(decision) {
                            warn!(
                                session_id,
                                "permission requester disappeared before receiving the decision"
                            );
                        }
                    }
                    CognitiveEvent::TokenChunk(ref text) => {
                        assistant_text.push_str(text);
                        let update = map_event_to_update(event);
                        send_session_update(transport, session_id, update).await?;
                    }
                    other => {
                        let update = map_event_to_update(other);
                        send_session_update(transport, session_id, update).await?;
                    }
                }
            }
            StreamAction::Inbound(inbound) => match inbound? {
                Some(JsonRpcMessage::Notification(notification))
                    if notification.method == "session/cancel" =>
                {
                    match serde_json::from_value::<SessionCancelParams>(
                        notification.params.unwrap_or(serde_json::Value::Null),
                    ) {
                        Ok(params) if params.session_id == session_id => {
                            cancel_token.cancel();
                        }
                        Ok(_) => {}
                        Err(error) => {
                            warn!(
                                session_id,
                                error = %error,
                                "received malformed session/cancel while prompt was active"
                            );
                        }
                    }
                }
                Some(JsonRpcMessage::Notification(notification)) => {
                    warn!(
                        session_id,
                        method = %notification.method,
                        "ignoring unsupported notification while prompt was active"
                    );
                }
                Some(JsonRpcMessage::Response(response)) => {
                    transport.handle_incoming_response(response);
                }
                Some(JsonRpcMessage::Request(request)) => {
                    warn!(
                        session_id,
                        method = %request.method,
                        "ignoring inbound request while prompt was active"
                    );
                }
                None => {
                    warn!(
                        session_id,
                        "ACP client disconnected while prompt was active"
                    );
                    return Ok(StreamResult {
                        prompt_result: SessionPromptResult {
                            stop_reason: StopReason::Cancelled,
                        },
                        assistant_text,
                        usage: None,
                    });
                }
            },
        }
    }
}

// ── Session prompt entry point ───────────────────────────────────────

/// Handles a `session/prompt` request by running the cognitive task and streaming updates.
pub async fn handle_session_prompt<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    params: SessionPromptParams,
    workdir: &Path,
    roko_config: &RokoConfig,
) -> Result<SessionPromptResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if session.is_busy() {
        return Err(BridgeEventsError::SessionBusy(session.session_id.clone()));
    }

    session.ensure_provider_runtime(workdir, roko_config);
    session.begin_prompt();

    let outcome =
        handle_session_prompt_inner(transport, session, params, workdir, roko_config).await;
    session.finish_prompt();
    outcome
}

async fn handle_session_prompt_inner<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    params: SessionPromptParams,
    workdir: &Path,
    roko_config: &RokoConfig,
) -> Result<SessionPromptResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let prompt_text = extract_prompt_text(&params.prompt);
    let prompt_has_images = params
        .prompt
        .iter()
        .any(|block| matches!(block, ContentBlock::Image { .. }));
    let model_key = session.config_state.model.clone();
    let is_slash_command = prompt_text.trim_start().starts_with('/');
    if !is_slash_command && session.cost_budget_exceeded() {
        return Err(BridgeEventsError::BudgetExceeded {
            cost_budget_usd: session.cost_budget_usd.unwrap_or_default(),
            accumulated_cost_usd: session.accumulated_cost_usd,
        });
    }
    let provider_health = Arc::clone(
        session
            .provider_health_registry
            .as_ref()
            .expect("ACP provider health initialized before prompt"),
    );
    let provider_rate_limiter = Arc::clone(
        session
            .provider_rate_limiter
            .as_ref()
            .expect("ACP provider rate limiter initialized before prompt"),
    );
    let experiment_path = workdir.join(".roko").join("learn").join("experiments.json");
    let experiment_assignment = if is_slash_command {
        None
    } else {
        assign_acp_experiment(&experiment_path, &session.config_state.agent_mode)
    };
    let (experiment_assignment, experiment_model_key) = applicable_acp_experiment(
        roko_config,
        &model_key,
        session.config_state.model_selection_explicit,
        experiment_assignment,
    );
    let routing_model_key = experiment_model_key
        .clone()
        .unwrap_or_else(|| model_key.clone());
    let requested_resolved = resolve_model(roko_config, &routing_model_key);

    // Capture workflow config before model selection: cascade routing owns only
    // direct single-agent prompts. Slash commands and workflow pipelines retain
    // their explicitly selected/configured model behavior.
    let workflow_config = session.config_state.workflow.clone();
    let pipeline_template = if workflow_config == "auto" {
        Some(crate::pipeline::WorkflowTemplate::auto_select(&prompt_text))
    } else {
        crate::pipeline::WorkflowTemplate::from_config(&workflow_config)
    };

    let cascade_candidate = if !is_slash_command
        && pipeline_template.is_none()
        && !prompt_has_images
        && !session.config_state.model_selection_explicit
        && experiment_model_key.is_none()
    {
        cascade_select_model(AcpCascadeRequest {
            workdir,
            roko_config,
            mode: &session.config_state.agent_mode,
            prompt: &prompt_text,
            effort: &session.config_state.effort,
            resolved_slug: &requested_resolved.slug,
            model_selection_explicit: session.config_state.model_selection_explicit,
            provider_health: &provider_health,
            rate_limiter: &provider_rate_limiter,
        })
    } else {
        None
    };
    let (resolved, model_key_for_dispatch, cascade_selection) =
        resolve_acp_dispatch_model(roko_config, &routing_model_key, cascade_candidate);

    if let Some(assignment) = experiment_assignment.as_ref() {
        info!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            section = %assignment.section_name,
            model_override = ?experiment_model_key,
            "assigned ACP experiment variant"
        );
    }

    let pipeline_accepts_images =
        pipeline_template.is_none() || std::env::var_os("ROKO_ACP_LEGACY").is_none();
    let prompt_capabilities = advertised_prompt_capabilities_for_model(
        resolved.provider_kind,
        !is_slash_command
            && pipeline_accepts_images
            && resolved
                .profile
                .as_ref()
                .is_some_and(|profile| profile.supports_vision),
    );
    if let Some(message) = unsupported_prompt_content(&params.prompt, &prompt_capabilities) {
        if let Some(assignment) = experiment_assignment.as_ref()
            && let Err(error) = record_acp_experiment_outcome(&experiment_path, assignment, false)
        {
            warn!(
                experiment_id = %assignment.experiment_id,
                variant_id = %assignment.variant_id,
                error = %error,
                "failed to persist rejected ACP experiment outcome"
            );
        }
        return Err(BridgeEventsError::UnsupportedPromptContent(
            message.to_string(),
        ));
    }

    if prompt_has_images {
        let probe = vec![ModelInputMessage::new(
            MessageRole::User,
            model_input_blocks_from_prompt(&params.prompt),
        )];
        validate_model_input_messages(&probe).map_err(|error| {
            BridgeEventsError::UnsupportedPromptContent(format!("invalid image input: {error}"))
        })?;
    }

    if let Some(selection) = cascade_selection.as_ref() {
        if model_key_for_dispatch == requested_resolved.model_key {
            debug!(
                requested_model = %model_key,
                selected_model = %model_key_for_dispatch,
                stage = %selection.stage,
                "cascade router retained requested model for ACP dispatch"
            );
        } else {
            info!(
                requested_model = %model_key,
                selected_model = %model_key_for_dispatch,
                stage = %selection.stage,
                reason = "adaptive cascade selection",
                "cascade router overriding model for ACP dispatch"
            );
        }
    }

    let resolved_for_logging = resolved.clone();

    debug!(
        session_id = %session.session_id,
        prompt_blocks = params.prompt.len(),
        prompt_chars = prompt_text.chars().count(),
        include_context = params.include_context,
        model_key = %model_key,
        workdir = %workdir.display(),
        "handling ACP session prompt"
    );

    if !is_slash_command {
        session.push_user_turn(prompt_text.clone());
    }

    // Permission is requested per-tool-call by the agent, not preemptively.

    let should_resolve_context = !is_slash_command && pipeline_template.is_none();

    let knowledge = if is_slash_command {
        DispatchKnowledge::default()
    } else {
        query_dispatch_knowledge(workdir, &prompt_text).await
    };
    let knowledge_context = knowledge.context_text();

    // Resolve context only for the single-agent path.
    // Resource blocks always resolve; @-mentions are only resolved when
    // prompt-time context is enabled.
    let file_context = if should_resolve_context {
        if params.include_context {
            resolve_context_items(&params.prompt, workdir).await
        } else {
            let uris = extract_resource_uris(&params.prompt);
            if uris.is_empty() {
                String::new()
            } else {
                read_file_context(&uris, workdir)
            }
        }
    } else {
        String::new()
    };

    // Get system prompt and history context for the single-agent path.
    let system_prompt = if should_resolve_context {
        session.build_system_prompt(workdir, &[], session.cached_conventions.as_deref())
    } else {
        String::new()
    };
    let _history_context = if should_resolve_context {
        session.build_history_context_for_cli()
    } else {
        String::new()
    };
    let messages = if should_resolve_context {
        // Build combined system prompt with resolved context.
        let mut full_system = system_prompt.clone();
        full_system = append_context(&full_system, &file_context);
        full_system = append_context(&full_system, &knowledge_context);
        if let Some(assignment) = experiment_assignment.as_ref() {
            full_system = append_context(&full_system, &render_experiment_context(assignment));
        }
        let mut msgs = session.build_messages_array(&full_system, &prompt_text);
        // If the prompt contains Image blocks, replace the last user message's
        // content with a multi-part content array in the appropriate format.
        inject_image_parts(&mut msgs, &params.prompt, resolved.provider_kind);
        msgs
    } else {
        // Pipeline path: build a minimal user-message array so that image blocks
        // are preserved for any downstream consumer that inspects `messages`.
        let dispatch_prompt = experiment_assignment.as_ref().map_or_else(
            || prompt_text.clone(),
            |assignment| append_context(&prompt_text, &render_experiment_context(assignment)),
        );
        let mut msgs = vec![serde_json::json!({"role": "user", "content": dispatch_prompt})];
        inject_image_parts(&mut msgs, &params.prompt, resolved.provider_kind);
        msgs
    };
    let input_messages = if prompt_has_images {
        model_input_messages_from_wire(&messages).map_err(|error| {
            BridgeEventsError::UnsupportedPromptContent(format!("invalid image input: {error}"))
        })?
    } else {
        Vec::new()
    };

    let (event_sender, event_receiver) = mpsc::channel(64);
    if !is_slash_command {
        emit_knowledge_card(&knowledge, &event_sender).await;
    }
    let provenance = if is_slash_command {
        None
    } else {
        build_provenance(&knowledge.hits, &knowledge.playbooks, &prompt_text, workdir).await
    };
    let provenance_card = provenance.as_ref().map(render_provenance_card);
    if !is_slash_command
        && pipeline_template.is_none()
        && let Some(chain) = provenance.as_ref()
    {
        emit_provenance_card(chain, &event_sender).await;
    }
    let stream_cancel_token = session.cancel_token.clone();
    let cancel_token = stream_cancel_token.clone();
    let stream_session_id = session.session_id.clone();
    let session_id = stream_session_id.clone();
    let worktree_before = crate::runner::WorktreeChangeSnapshot::capture(workdir);
    let workdir = workdir.to_path_buf();
    let workdir_for_logging = workdir.clone();
    let roko_config = roko_config.clone();
    let roko_config_for_logging = roko_config.clone();
    let prompt_text_for_logging = prompt_text.clone();
    let prompt_text_for_dispatch = experiment_assignment.as_ref().map_or_else(
        || prompt_text.clone(),
        |assignment| append_context(&prompt_text, &render_experiment_context(assignment)),
    );
    // The actual dispatched config key must drive provider construction,
    // episode/cost attribution, and the router observation arm.
    let model_key_for_logging = model_key_for_dispatch.clone();
    let cascade_selection_for_logging = cascade_selection.clone();
    let dispatch_started = Instant::now();
    let is_pipeline_dispatch = pipeline_template.is_some();

    let clippy_enabled = session.config_state.clippy_enabled;
    let tests_enabled = session.config_state.tests_enabled;
    let max_iterations = session.config_state.max_iterations;
    let review_strictness = session.config_state.review_strictness.clone();
    let session_mcp_servers = session.mcp_servers.clone();
    let session_mcp_config_path = session.mcp_config_path.clone();
    let session_tools_enabled = session.tools_enabled;
    let session_tool_capabilities = derive_acp_tool_capabilities(
        &session.config_state.agent_mode,
        &session.client_capabilities,
        !session_mcp_servers.is_empty(),
        &session.always_allowed,
    );
    // Effort level from the IDE dropdown (low/medium/high/max). Passed to
    // config_with_session_effort() at dispatch time so the provider backend
    // sees it as `agent.default_effort`. See that function's doc comment for
    // the full effort dispatch flow.
    let session_effort = session.config_state.effort.clone();

    let shared_run = session.shared_run.clone();
    // SP-1: build a restrictive layer per dispatch; missing contracts fall closed.
    let pre_dispatch_violation = {
        let safety =
            SafetyLayer::from_config(&roko_config).with_role(&session.config_state.agent_mode);
        match safety.pre_dispatch_check_with_context(
            &session.session_id,
            "session-prompt",
            &session.config_state.agent_mode,
            &workdir,
            &DispatchSafetyContext::for_local_action(&prompt_text).with_network_requirement(true),
        ) {
            Ok(()) => None,
            Err(violation) => match violation.severity {
                ViolationSeverity::Block => {
                    error!(
                        session_id = %session.session_id,
                        violation = ?violation.violation_type,
                        message = %violation.message,
                        "ACP pre-dispatch safety check BLOCKED dispatch"
                    );
                    Some(violation)
                }
                ViolationSeverity::Warn => {
                    warn!(
                        session_id = %session.session_id,
                        violation = ?violation.violation_type,
                        message = %violation.message,
                        "ACP pre-dispatch safety warning"
                    );
                    None
                }
            },
        }
    };

    // Shared channel for the workflow engine path: the cognitive task writes the
    // WorkflowRunReport's actual cost (which was aggregated from AgentCompleted events)
    // here so that append_acp_episode can use it instead of the pricing-table estimate.
    let prompt_text_for_title = prompt_text.clone();

    let workflow_cost_sink: Arc<Mutex<Option<f64>>> = Arc::new(Mutex::new(None));
    let workflow_cost_sink_task = Arc::clone(&workflow_cost_sink);

    let cognitive_task = tokio::spawn(async move {
        if let Some(violation) = pre_dispatch_violation {
            let message = violation.message;
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "Safety check blocked this action: {}",
                    message
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Err(anyhow::anyhow!("ACP pre-dispatch safety violation: {}", message).into());
        }

        if is_slash_command {
            return run_slash_command(
                &session_id,
                prompt_text_for_dispatch.trim(),
                &workdir,
                model_key_for_dispatch.clone(),
                cancel_token,
                event_sender,
                shared_run,
            )
            .await;
        }

        if let Some(template) = pipeline_template {
            if std::env::var_os("ROKO_ACP_LEGACY").is_some() {
                let legacy_run = shared_run.clone();
                let result = crate::runner::run_workflow_pipeline(
                    &session_id,
                    &prompt_text_for_dispatch,
                    knowledge_context.clone(),
                    provenance_card.clone(),
                    &workdir,
                    crate::runner::PipelineConfig {
                        template,
                        max_iterations,
                        clippy_enabled,
                        tests_enabled,
                        review_strictness,
                        model_slug: resolved.slug.clone(),
                        mcp_config: write_session_mcp_config(&session_mcp_servers, &workdir),
                        sandbox_level: roko_config.runner.sandbox_level,
                    },
                    cancel_token,
                    event_sender,
                    legacy_run.clone(),
                )
                .await;

                result?;

                let final_phase = legacy_run
                    .lock()
                    .await
                    .as_ref()
                    .map(|run| run.pipeline.phase.clone());

                return match final_phase {
                    Some(crate::pipeline::PipelinePhase::Complete) => Ok(()),
                    Some(crate::pipeline::PipelinePhase::Halted { reason }) => {
                        Err(anyhow::anyhow!("workflow pipeline halted: {reason}").into())
                    }
                    Some(crate::pipeline::PipelinePhase::Cancelled) => {
                        Err(anyhow::anyhow!("workflow pipeline cancelled").into())
                    }
                    Some(phase) => Err(anyhow::anyhow!(
                        "workflow pipeline ended in unexpected phase: {phase:?}"
                    )
                    .into()),
                    None => Err(anyhow::anyhow!(
                        "workflow pipeline completed without shared run state"
                    )
                    .into()),
                };
            }

            let mcp_config_path = write_session_mcp_config(&session_mcp_servers, &workdir);
            let report = run_with_workflow_engine(
                &session_id,
                &prompt_text_for_dispatch,
                &workdir,
                workflow_template_name(&template),
                crate::runner::WorkflowEngineOptions {
                    model_key: model_key_for_dispatch,
                    input_messages: input_messages.clone(),
                    mcp_config: mcp_config_path,
                    provenance_card,
                    route: crate::runner::AcpWorkflowRoute::LegacyDefault,
                },
                event_sender,
            )
            .await?;

            // Thread the actual cost from the report back to the main task so
            // append_acp_episode can record it instead of using the pricing-table estimate.
            if let Some(cost) = report.cost
                && let Ok(mut sink) = workflow_cost_sink_task.lock()
            {
                *sink = Some(cost);
            }

            if !report.success {
                return Err(anyhow::anyhow!(
                    "workflow engine reported unsuccessful run: {}",
                    report.output
                )
                .into());
            }

            return Ok(());
        }

        // Default: single-agent dispatch (workflow = "none").
        let provider_kind = resolved.provider_kind;

        info!(
            requested_model = %model_key,
            model_key = %model_key_for_dispatch,
            slug = %resolved.slug,
            provider_kind = ?provider_kind,
            "resolved model for ACP prompt"
        );

        match provider_kind {
            // AnthropicApi uses the dedicated Anthropic model caller path.
            // The provider must be present in explicit RokoConfig; ACP does
            // not synthesize providers from ANTHROPIC_API_KEY.
            ProviderKind::AnthropicApi => {
                run_anthropic_cognitive_task(
                    &session_id,
                    &messages,
                    &model_key_for_dispatch,
                    &resolved.slug,
                    &roko_config,
                    Arc::clone(&provider_health),
                    Arc::clone(&provider_rate_limiter),
                    &workdir,
                    &session_mcp_servers,
                    &session_effort,
                    session_tools_enabled,
                    session_tool_capabilities,
                    cancel_token,
                    event_sender,
                )
                .await
            }
            // All other providers (ClaudeCli, OpenAiCompat, etc.) go through
            // ModelCallService which handles each provider kind natively.
            _ => {
                run_openai_compat_cognitive_task(
                    &session_id,
                    &messages,
                    &model_key_for_dispatch,
                    &roko_config,
                    Arc::clone(&provider_health),
                    Arc::clone(&provider_rate_limiter),
                    &workdir,
                    &session_mcp_servers,
                    session_mcp_config_path.as_deref(),
                    &session_effort,
                    session_tools_enabled,
                    session_tool_capabilities,
                    cancel_token,
                    event_sender,
                )
                .await
            }
        }
    });

    let mut stream_result = stream_events_to_editor(
        transport,
        &stream_session_id,
        session,
        &workdir_for_logging,
        event_receiver,
        &stream_cancel_token,
    )
    .await;

    if !is_slash_command
        && let Ok(ref sr) = stream_result
        && let Some(usage) = sr.usage.as_ref()
    {
        let size = resolved_for_logging
            .profile
            .as_ref()
            .map(|profile| profile.context_window)
            .unwrap_or_else(|| ModelProfile::default().context_window);
        let update = SessionUpdate::UsageUpdate {
            used: usage.total_tokens,
            size,
            cost: calculate_cost_for_model_slug(
                &resolved_for_logging.slug,
                usage.input_tokens,
                usage.output_tokens,
                usage.cached_read_tokens.unwrap_or(0),
            )
            .map(|amount| CostInfo {
                amount,
                currency: "USD".to_string(),
            }),
        };
        if let Err(error) = send_session_update(transport, &session.session_id, update).await {
            warn!(
                session_id = %session.session_id,
                error = %error,
                "failed to send ACP usage update"
            );
        }
    }

    let task_result = cognitive_task.await;
    let (task_error, task_join_error) = match task_result {
        Ok(Ok(())) => (None, None),
        Ok(Err(e)) => {
            let error_text = e.to_string();
            if error_text.starts_with("ACP pre-dispatch safety violation:") {
                warn!(error = %error_text, "cognitive task blocked before dispatch");
            } else {
                error!(error = %error_text, "cognitive task failed");
            }
            (Some(error_text), None)
        }
        Err(join_error) => {
            let error_text = join_error.to_string();
            error!(error = %error_text, "cognitive task failed to join");
            (
                Some(error_text),
                Some(BridgeEventsError::TaskJoin(join_error)),
            )
        }
    };
    let stream_error = stream_result.as_ref().err().map(|err| err.to_string());

    let post_dispatch_block = if let Ok(ref sr) = stream_result
        && !sr.assistant_text.is_empty()
    {
        let changed_files = worktree_before.changed_files(&workdir_for_logging);
        let safety = SafetyLayer::from_config(&roko_config_for_logging)
            .with_role(&session.config_state.agent_mode);
        let violations = safety.post_dispatch_check(
            &session.session_id,
            "session-prompt",
            &session.config_state.agent_mode,
            &sr.assistant_text,
            &changed_files,
        );
        for v in &violations {
            match v.severity {
                ViolationSeverity::Warn | ViolationSeverity::Block => {
                    warn!(
                        session_id = %session.session_id,
                        violation = ?v.violation_type,
                        message = %v.message,
                        "ACP post-dispatch safety violation"
                    );
                }
            }
        }
        let block_messages = violations
            .iter()
            .filter(|violation| violation.severity == ViolationSeverity::Block)
            .map(|violation| format!("{}: {}", violation.violation_type, violation.message))
            .collect::<Vec<_>>();
        (!block_messages.is_empty()).then(|| block_messages.join("; "))
    } else {
        None
    };
    if let Some(block_message) = post_dispatch_block {
        stream_result = Err(BridgeEventsError::Pipeline(anyhow::anyhow!(
            "ACP post-dispatch safety block: {block_message}"
        )));
    }

    if !is_slash_command {
        // For the workflow engine path, the cognitive task wrote the actual provider cost
        // (from WorkflowRunReport) to workflow_cost_sink. Use it to override the
        // pricing-table estimate in append_acp_episode so the episode has accurate cost data.
        let cost_override = workflow_cost_sink.lock().ok().and_then(|g| *g);
        append_acp_episode(
            &roko_config_for_logging,
            &workdir_for_logging,
            session,
            &model_key_for_logging,
            &prompt_text_for_logging,
            &workflow_config,
            is_pipeline_dispatch,
            dispatch_started,
            stream_result.as_ref().ok(),
            task_error.as_deref(),
            stream_error.as_deref(),
            cost_override,
            cascade_selection_for_logging.as_ref(),
        )
        .await;

        let stream_result_ref = stream_result.as_ref().ok();
        let dispatch_succeeded = acp_dispatch_succeeded(
            stream_result_ref,
            task_error.as_deref(),
            stream_error.as_deref(),
        );
        let efficiency_event = acp_efficiency_event(
            &session.session_id,
            &resolved_for_logging,
            dispatch_started,
            stream_result_ref,
            dispatch_succeeded,
            cost_override,
        );
        session.record_efficiency_cost(efficiency_event.cost_usd);
        let budget_status = session.budget_status();
        if let (Some(cost_budget_usd), Some(accumulated_cost_usd), Some(budget_remaining_usd)) = (
            budget_status.cost_budget_usd,
            budget_status.accumulated_cost_usd,
            budget_status.budget_remaining_usd,
        ) && let Err(error) = send_session_update(
            transport,
            &session.session_id,
            SessionUpdate::BudgetStatusUpdate {
                cost_budget_usd,
                accumulated_cost_usd,
                budget_remaining_usd,
            },
        )
        .await
        {
            warn!(
                session_id = %session.session_id,
                error = %error,
                "failed to send ACP budget status update"
            );
        }
        emit_acp_efficiency_event(&workdir_for_logging, efficiency_event);

        if let Some(assignment) = experiment_assignment.as_ref()
            && let Err(error) =
                record_acp_experiment_outcome(&experiment_path, assignment, dispatch_succeeded)
        {
            warn!(
                experiment_id = %assignment.experiment_id,
                variant_id = %assignment.variant_id,
                error = %error,
                "failed to persist ACP experiment outcome"
            );
        }

        if !is_pipeline_dispatch {
            let model_slugs =
                cascade_router_model_slugs(&roko_config_for_logging, &resolved_for_logging.slug);
            let routing_ctx = acp_routing_context(
                &session.config_state.agent_mode,
                &prompt_text_for_logging,
                &session.config_state.effort,
                &workdir_for_logging,
            );
            let output_tokens =
                stream_result_ref.and_then(|sr| sr.usage.as_ref().map(|usage| usage.output_tokens));
            // Observe only direct prompts here. Workflow services own their own
            // provider feedback and recording them again would train the wrong arm.
            drop(record_cascade_observation(
                workdir_for_logging
                    .join(".roko")
                    .join("learn")
                    .join("cascade-router.json"),
                model_key_for_logging.clone(),
                routing_ctx,
                dispatch_succeeded,
                dispatch_started.elapsed().as_millis() as u64,
                output_tokens,
                model_slugs,
            ));
        }
    }

    if let Some(join_error) = task_join_error {
        return Err(join_error);
    }

    // Auto-set session title from first user message.
    if session.session_name.is_none() && !is_slash_command {
        let title = truncate_to_title(&prompt_text_for_title, 60);
        session.session_name = Some(title.clone());
        let title_update = SessionUpdate::SessionInfoUpdate {
            session_id: session.session_id.clone(),
            session_name: Some(title),
        };
        if let Err(error) = send_session_update(transport, &session.session_id, title_update).await
        {
            warn!(
                session_id = %session.session_id,
                error = %error,
                "failed to send ACP session title update"
            );
        }
    }

    // Push assistant turn after streaming completes (skip slash commands).
    // If dispatch failed (empty assistant text), pop the user turn we pushed earlier
    // to prevent a dangling user message with no response in the history.
    match &stream_result {
        Ok(sr) if !is_slash_command && !sr.assistant_text.is_empty() => {
            session.push_assistant_turn(truncate_assistant_history(&sr.assistant_text));
        }
        _ if !is_slash_command => {
            session.conversation_history.pop();
        }
        _ => {}
    }

    stream_result.map(|sr| sr.prompt_result)
}

// ── Anthropic Messages API dispatch ──────────────────────────────────

/// Dispatches a prompt via the Anthropic adapter through the shared model stream contract.
/// Used for explicitly AnthropicApi-configured models in ACP.
#[allow(clippy::too_many_arguments)]
async fn run_anthropic_cognitive_task(
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
    let tools = tools_enabled
        .then(|| filter_tools_by_ceiling(acp_builtin_tools(), &tool_capabilities))
        .unwrap_or_default();
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
async fn run_anthropic_tool_loop(
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
                }),
            );
        }
    }

    if !mcp_servers.is_empty() {
        let (mcp_state, statuses) =
            setup_session_mcp_tools(session_id, mcp_servers, event_sender.clone()).await;
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
    let dispatcher = Arc::new(ToolDispatcher::new(registry, resolver));

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

    let (chunk_sender, chunk_receiver) = mpsc::channel(256);
    let forwarder = tokio::spawn(forward_tool_loop_stream_chunks(
        chunk_receiver,
        event_sender.clone(),
    ));

    let mut tool_context = ToolContext::new(
        workdir,
        Duration::from_secs(120),
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

fn anthropic_model_call_config(
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

fn model_call_request_from_acp_messages(
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

fn model_call_chat_message_from_acp(message: &serde_json::Value) -> Option<ChatMessage> {
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
enum ModelStreamForward {
    Continue,
    Completed,
}

#[derive(Default)]
struct ModelStreamForwardState {
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

async fn forward_model_stream_event(
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

fn usage_info_from_model_usage(usage: &TokenUsage) -> UsageInfo {
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

fn acp_stop_reason_from_model(stop_reason: Option<&str>) -> StopReason {
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
async fn run_openai_compat_cognitive_task(
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
    let tools = tools_enabled
        .then(|| filter_tools_by_ceiling(acp_builtin_tools(), &tool_capabilities))
        .unwrap_or_default();
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
fn config_with_session_effort(roko_config: &RokoConfig, effort: &str) -> RokoConfig {
    let mut config = roko_config.clone();
    let effort = effort.trim();
    if !effort.is_empty() {
        config.agent.default_effort = effort.to_owned();
    }
    config
}

fn openai_compat_tool_loop_supported(provider_kind: ProviderKind) -> bool {
    matches!(
        provider_kind,
        ProviderKind::OpenAiCompat | ProviderKind::PerplexityApi | ProviderKind::CerebrasApi
    )
}

#[allow(clippy::too_many_arguments)]
async fn run_openai_compat_mcp_tool_loop(
    session_id: &str,
    messages: &[serde_json::Value],
    resolved: &ResolvedModel,
    workdir: &Path,
    mcp_servers: &[crate::types::McpServerConfig],
    rate_limiter: Arc<ProviderRateLimiter>,
    tool_capabilities: ToolPermission,
    allowed_tools: Option<Vec<String>>,
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

    let (mcp_state, mcp_statuses) =
        setup_session_mcp_tools(session_id, mcp_servers, event_sender.clone()).await;
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
    let dispatcher = Arc::new(ToolDispatcher::new(registry, resolver));
    let context_limit = usize::try_from(model.context_window).unwrap_or(usize::MAX);
    let tool_loop = ToolLoop::new(translator, dispatcher, backend)
        .with_max_iterations(DEFAULT_MAX_TOOL_ITERATIONS)
        .with_context_token_limit(context_limit);

    let (chunk_sender, chunk_receiver) = mpsc::channel(256);
    let forwarder = tokio::spawn(forward_tool_loop_stream_chunks(
        chunk_receiver,
        event_sender.clone(),
    ));
    let mut tool_context = ToolContext::new(
        workdir,
        Duration::from_secs(120),
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
async fn run_openai_compat_builtin_tool_loop(
    session_id: &str,
    messages: &[serde_json::Value],
    resolved: &ResolvedModel,
    workdir: &Path,
    rate_limiter: Arc<ProviderRateLimiter>,
    tool_capabilities: ToolPermission,
    allowed_tools: Option<Vec<String>>,
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
    let dispatcher = Arc::new(ToolDispatcher::new(registry, resolver));
    let context_limit = usize::try_from(model.context_window).unwrap_or(usize::MAX);
    let tool_loop = ToolLoop::new(translator, dispatcher, backend)
        .with_max_iterations(DEFAULT_MAX_TOOL_ITERATIONS)
        .with_context_token_limit(context_limit);

    let (chunk_sender, chunk_receiver) = mpsc::channel(256);
    let forwarder = tokio::spawn(forward_tool_loop_stream_chunks(
        chunk_receiver,
        event_sender.clone(),
    ));
    let mut tool_context = ToolContext::new(
        workdir,
        Duration::from_secs(120),
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

async fn forward_tool_loop_stream_chunks(
    mut receiver: mpsc::Receiver<StreamChunk>,
    event_sender: mpsc::Sender<CognitiveEvent>,
) {
    while let Some(chunk) = receiver.recv().await {
        match chunk {
            StreamChunk::ContentDelta(text) if !text.is_empty() => {
                send_cognitive_event(&event_sender, CognitiveEvent::TokenChunk(text)).await;
            }
            StreamChunk::ReasoningDelta(text) if !text.is_empty() => {
                send_cognitive_event(&event_sender, CognitiveEvent::ThinkingChunk(text)).await;
            }
            StreamChunk::Error(error) => {
                warn!(error = %error, "ACP MCP tool-loop stream error");
            }
            StreamChunk::ToolCallDelta { .. } | StreamChunk::Usage(_) | StreamChunk::Done(_) => {}
            StreamChunk::ContentDelta(_) | StreamChunk::ReasoningDelta(_) => {}
            StreamChunk::ToolProgress { .. } => {}
        }
    }
}

fn usage_info_from_tool_loop_usage(usage: &roko_core::Usage) -> Option<UsageInfo> {
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

/// Write session MCP server configs to a temporary `.mcp.json` file that
/// the Claude CLI can consume via `--mcp-config`.
///
/// Returns `None` when the input list is empty or all servers use unsupported
/// transports (only stdio is supported for the Claude CLI passthrough).
fn write_session_mcp_config(
    mcp_servers: &[crate::types::McpServerConfig],
    workdir: &Path,
) -> Option<PathBuf> {
    if mcp_servers.is_empty() {
        return None;
    }

    let mut servers = serde_json::Map::new();
    for server in mcp_servers {
        match &server.transport {
            crate::types::McpTransport::Stdio { command, args } => {
                servers.insert(
                    server.name.clone(),
                    serde_json::json!({
                        "command": command,
                        "args": args,
                    }),
                );
            }
            crate::types::McpTransport::Http { .. } => {
                // HTTP transports are not supported by Claude CLI's
                // `--mcp-config` flag; skip them silently.
            }
        }
    }

    if servers.is_empty() {
        return None;
    }

    let config = serde_json::json!({ "mcpServers": servers });
    let roko_dir = workdir.join(".roko");
    let _ = std::fs::create_dir_all(&roko_dir);
    let path = roko_dir.join("session-mcp.json");
    match std::fs::write(
        &path,
        serde_json::to_string_pretty(&config).unwrap_or_default(),
    ) {
        Ok(()) => {
            debug!(path = %path.display(), servers = servers.len(), "wrote session MCP config");
            Some(path)
        }
        Err(error) => {
            warn!(path = %path.display(), error = %error, "failed to write session MCP config");
            None
        }
    }
}

struct SessionMcpRuntime {
    tools: Vec<ToolDef>,
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

async fn setup_session_mcp_tools(
    session_id: &str,
    mcp_servers: &[crate::types::McpServerConfig],
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> (SessionMcpRuntime, Vec<McpServerStatus>) {
    let mut tools = Vec::new();
    let mut handlers: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();
    let mut used_names = HashSet::new();
    let mut statuses = Vec::new();

    for server in mcp_servers {
        let discovery_timeout = server
            .discovery_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS));
        let (command, args) = match &server.transport {
            crate::types::McpTransport::Stdio { command, args } => (command, args),
            crate::types::McpTransport::Http { url } => {
                warn!(
                    session_id,
                    server = %server.name,
                    url = %url,
                    "skipping session MCP server with unsupported HTTP transport"
                );
                statuses.push(McpServerStatus::failed(
                    server.name.clone(),
                    McpInitStatus::TransportUnsupported,
                    format!("HTTP transport is not supported for session MCP ({url})"),
                ));
                continue;
            }
        };

        let transport = match McpStdioTransport::spawn(command, args) {
            Ok(transport) => transport,
            Err(error) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "failed to spawn session MCP server"
                );
                statuses.push(McpServerStatus::failed(
                    server.name.clone(),
                    McpInitStatus::SpawnFailed,
                    error.to_string(),
                ));
                continue;
            }
        };
        let client = Arc::new(McpClient::new(transport));

        match tokio::time::timeout(discovery_timeout, client.initialize()).await {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "session MCP initialize failed"
                );
                statuses.push(McpServerStatus::failed(
                    server.name.clone(),
                    McpInitStatus::InitializeFailed,
                    error.to_string(),
                ));
                continue;
            }
            Err(_) => {
                warn!(
                    session_id,
                    server = %server.name,
                    timeout_ms = discovery_timeout.as_millis(),
                    "session MCP initialize timed out"
                );
                statuses.push(McpServerStatus::failed(
                    server.name.clone(),
                    McpInitStatus::InitializeTimeout,
                    format!(
                        "initialize timed out after {}ms",
                        discovery_timeout.as_millis()
                    ),
                ));
                continue;
            }
        }

        let listed = match tokio::time::timeout(discovery_timeout, client.list_tools()).await {
            Ok(Ok(listed)) => listed,
            Ok(Err(error)) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "session MCP tools/list failed"
                );
                statuses.push(McpServerStatus::failed(
                    server.name.clone(),
                    McpInitStatus::ToolsListFailed,
                    error.to_string(),
                ));
                continue;
            }
            Err(_) => {
                warn!(
                    session_id,
                    server = %server.name,
                    timeout_ms = discovery_timeout.as_millis(),
                    "session MCP tools/list timed out"
                );
                statuses.push(McpServerStatus::failed(
                    server.name.clone(),
                    McpInitStatus::ToolsListTimeout,
                    format!(
                        "tools/list timed out after {}ms",
                        discovery_timeout.as_millis()
                    ),
                ));
                continue;
            }
        };

        info!(
            session_id,
            server = %server.name,
            tool_count = listed.len(),
            "discovered session MCP tools"
        );
        statuses.push(McpServerStatus::ready(server.name.clone(), listed.len()));

        for tool in listed {
            let base_name = format!(
                "{}_{}",
                sanitize_tool_segment(&server.name),
                sanitize_tool_segment(&tool.name)
            );
            let exposed_name = unique_tool_name(&base_name, &mut used_names);
            let mut def = mcp_to_tool_def(&tool, &server.name);
            def.name = exposed_name.clone();
            def.source = ToolSource::Mcp {
                server: server.name.clone(),
            };

            handlers.insert(
                exposed_name.clone(),
                Arc::new(AcpMcpToolHandler {
                    client: Arc::clone(&client),
                    exposed_name,
                    remote_name: tool.name.clone(),
                    event_sender: event_sender.clone(),
                }),
            );
            tools.push(def);
        }
    }

    (SessionMcpRuntime { tools, handlers }, statuses)
}

fn sanitize_tool_segment(input: &str) -> String {
    let mut output = String::with_capacity(input.len().min(28));
    for ch in input.chars().take(28) {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "tool".to_string()
    } else {
        output
    }
}

fn unique_tool_name(base: &str, used: &mut HashSet<String>) -> String {
    let base: String = base.chars().take(64).collect();
    if used.insert(base.clone()) {
        return base;
    }

    for suffix in 2.. {
        let suffix = format!("_{suffix}");
        let max_base_len = 64usize.saturating_sub(suffix.len());
        let mut candidate: String = base.chars().take(max_base_len).collect();
        candidate.push_str(&suffix);
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }

    unreachable!("suffix search should always find a unique tool name")
}

struct AcpMcpHandlerResolver {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl HandlerResolver for AcpMcpHandlerResolver {
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        self.handlers.get(name).cloned()
    }
}

struct AcpMcpToolHandler {
    client: Arc<McpClient<McpStdioTransport>>,
    exposed_name: String,
    remote_name: String,
    event_sender: mpsc::Sender<CognitiveEvent>,
}

#[async_trait]
impl ToolHandler for AcpMcpToolHandler {
    fn name(&self) -> &str {
        &self.exposed_name
    }

    async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
        let tool_call_id = if call.id.is_empty() {
            format!("mcp-{}", uuid::Uuid::new_v4())
        } else {
            call.id.clone()
        };
        send_cognitive_event(
            &self.event_sender,
            CognitiveEvent::ToolCallStart {
                tool_call_id: tool_call_id.clone(),
                title: self.exposed_name.clone(),
                kind: ToolCallKind::Other,
                locations: None,
            },
        )
        .await;

        let result = match tokio::time::timeout(
            ctx.timeout,
            self.client.call_tool(&self.remote_name, call.arguments),
        )
        .await
        {
            Ok(Ok(result)) => tool_result_from_mcp(&self.exposed_name, &result),
            Ok(Err(error)) => ToolResult::err(ToolError::Other(format!(
                "mcp tool `{}` failed: {error}",
                self.exposed_name
            ))),
            Err(_) => ToolResult::err(ToolError::Timeout {
                after_ms: ctx.timeout.as_millis().try_into().unwrap_or(u64::MAX),
            }),
        };

        let (status, text) = tool_result_for_editor(&result);
        send_cognitive_event(
            &self.event_sender,
            CognitiveEvent::ToolCallComplete {
                tool_call_id,
                status,
                content: vec![ContentBlock::Text { text }],
            },
        )
        .await;

        result
    }
}

#[derive(Clone)]
struct AcpToolCancelToken(CancelToken);

impl roko_core::tool::CancelToken for AcpToolCancelToken {
    fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

// ── Builtin tool handler adapter ──────────────────────────────────────

/// Wraps [`execute_acp_builtin_tool`] in the [`ToolHandler`] trait so that
/// the [`ToolLoop`] infrastructure can dispatch builtin tools identically to
/// MCP tools.
struct AcpBuiltinToolHandler {
    tool_name: String,
    session_id: String,
    workdir: PathBuf,
    event_sender: mpsc::Sender<CognitiveEvent>,
}

#[async_trait]
impl ToolHandler for AcpBuiltinToolHandler {
    fn name(&self) -> &str {
        &self.tool_name
    }

    async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
        // Check denied_tools list — if this tool is explicitly denied, reject it.
        if let Some(ref denied) = ctx.denied_tools
            && denied.contains(&self.tool_name)
        {
            warn!(
                tool = %self.tool_name,
                session_id = %self.session_id,
                reason = "denied_tools",
                "ACP tool call denied"
            );
            return ToolResult::err(ToolError::Other(format!(
                "tool '{}' is denied for this command",
                self.tool_name
            )));
        }
        // Check allowed_tools list — if set, only tools in the list are permitted.
        if let Some(ref allowed) = ctx.allowed_tools
            && !allowed.contains(&self.tool_name)
        {
            warn!(
                tool = %self.tool_name,
                session_id = %self.session_id,
                reason = "not_in_allowed_tools",
                "ACP tool call denied"
            );
            return ToolResult::err(ToolError::Other(format!(
                "tool '{}' is not in the allowed set for this command",
                self.tool_name
            )));
        }
        debug!(
            tool = %self.tool_name,
            session_id = %self.session_id,
            "ACP tool call allowed"
        );

        if let Some((action, title, detail)) =
            tool_permission_request(&self.tool_name, &call.arguments)
        {
            let (decision_sender, decision_receiver) = tokio::sync::oneshot::channel();
            let request = CognitiveEvent::PermissionRequest {
                payload: PermissionRequestPayload {
                    action,
                    title,
                    detail,
                },
                reply: PermissionReplyChannel::new(decision_sender),
            };

            if self.event_sender.send(request).await.is_err() {
                warn!(
                    tool = %self.tool_name,
                    session_id = %self.session_id,
                    "ACP permission request could not reach the parent stream; denying tool"
                );
                return ToolResult::err(ToolError::PermissionDenied(format!(
                    "tool '{}' requires editor approval",
                    self.tool_name
                )));
            }

            match decision_receiver.await {
                Ok(PermissionDecision::Allow | PermissionDecision::AlwaysAllow) => {}
                Ok(PermissionDecision::Reject) | Err(_) => {
                    warn!(
                        tool = %self.tool_name,
                        session_id = %self.session_id,
                        "ACP mutation tool denied by editor permission gate"
                    );
                    return ToolResult::err(ToolError::PermissionDenied(format!(
                        "tool '{}' was not approved by the editor",
                        self.tool_name
                    )));
                }
            }
        }

        let output = crate::builtin_tools::execute_acp_builtin_tool(
            &self.tool_name,
            &call.arguments,
            &self.workdir,
            &self.event_sender,
        )
        .await;
        ToolResult::text(output)
    }
}

struct AcpBuiltinHandlerResolver {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl HandlerResolver for AcpBuiltinHandlerResolver {
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        self.handlers.get(name).cloned()
    }
}

fn tool_result_from_mcp(tool_name: &str, result: &roko_agent::mcp::McpToolResult) -> ToolResult {
    let text = mcp_result_text(result);
    if result.is_error {
        let message = if text.is_empty() {
            format!("mcp tool `{tool_name}` returned an error")
        } else {
            format!("mcp tool `{tool_name}` returned an error: {text}")
        };
        ToolResult::err(ToolError::Other(message))
    } else if text.is_empty() {
        ToolResult::text("(empty result)")
    } else {
        ToolResult::text(text)
    }
}

fn mcp_result_text(result: &roko_agent::mcp::McpToolResult) -> String {
    let text_blocks = result
        .content
        .iter()
        .filter(|block| block.content_type == "text")
        .filter_map(|block| block.text.as_deref())
        .collect::<Vec<_>>();
    if !text_blocks.is_empty() {
        return text_blocks.join("\n");
    }
    serde_json::to_string(&result.content).unwrap_or_else(|_| "[]".to_string())
}

fn tool_result_for_editor(result: &ToolResult) -> (ToolCallStatus, String) {
    match result {
        ToolResult::Ok { content, .. } => (ToolCallStatus::Completed, content.clone()),
        ToolResult::Err(error) => (ToolCallStatus::Failed, format!("error: {error}")),
    }
}

async fn emit_knowledge_card(
    knowledge: &DispatchKnowledge,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    let Some(card) = knowledge.card() else {
        return;
    };

    let tool_call_id = "knowledge-query".to_string();
    let _ = event_sender
        .send(CognitiveEvent::ToolCallStart {
            tool_call_id: tool_call_id.clone(),
            title: card.title,
            kind: ToolCallKind::Other,
            locations: None,
        })
        .await;
    let _ = event_sender
        .send(CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Completed,
            content: vec![ContentBlock::Text { text: card.body }],
        })
        .await;
}

/// A chain tracing why Roko chose a particular approach.
#[derive(Debug, Clone, PartialEq)]
struct ProvenanceChain {
    sources: Vec<ProvenanceSource>,
    confidence: f64,
}

/// One source in a decision provenance chain.
#[derive(Debug, Clone, PartialEq)]
enum ProvenanceSource {
    Playbook {
        id: String,
        goal: String,
        total_outcomes: u64,
        success_rate: f64,
    },
    Episode {
        task_id: String,
        success: bool,
        gate_summary: String,
    },
    Knowledge {
        kind: KnowledgeKind,
        tier: KnowledgeTier,
        score: f64,
        summary: String,
    },
    DreamPattern {
        description: String,
        guidance: String,
        confidence: f64,
    },
}

/// Build provenance from already-queried knowledge/playbook results and
/// best-effort episode/dream lookups.
async fn build_provenance(
    knowledge_hits: &[KnowledgeQueryHit],
    playbooks: &[Playbook],
    prompt: &str,
    workdir: &Path,
) -> Option<ProvenanceChain> {
    let mut sources = Vec::new();
    let mut has_playbook_source = false;

    for playbook in playbooks {
        let Some(success_rate) = playbook.success_rate() else {
            continue;
        };

        has_playbook_source = true;
        sources.push(ProvenanceSource::Playbook {
            id: playbook.id.clone(),
            goal: truncate_with_limit(playbook.goal.trim(), 80, "..."),
            total_outcomes: playbook.total_outcomes(),
            success_rate,
        });
    }

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let prompt_keywords = prompt_keywords(prompt);
    let episodes_future = EpisodeLogger::read_all_lossy(&episodes_path);
    let dreams_future = async {
        if prompt_keywords.is_empty() {
            return Vec::new();
        }

        match task::spawn_blocking({
            let workdir = workdir.to_path_buf();
            move || load_dream_routing_advice(&workdir)
        })
        .await
        {
            Ok(Ok(advice)) => {
                let mut seen_signatures = HashSet::new();
                let mut dream_sources = Vec::new();
                for keyword in prompt_keywords {
                    for pattern in relevant_pattern_summaries(&advice, &keyword, 0.5, 2) {
                        if !seen_signatures.insert(pattern.signature) {
                            continue;
                        }

                        dream_sources.push(ProvenanceSource::DreamPattern {
                            description: truncate_with_limit(&pattern.description, 80, "..."),
                            guidance: truncate_with_limit(&pattern.guidance, 80, "..."),
                            confidence: pattern.confidence,
                        });

                        if dream_sources.len() == 2 {
                            return dream_sources;
                        }
                    }
                }

                dream_sources
            }
            Ok(Err(err)) => {
                warn!(
                    workdir = %workdir.display(),
                    error = %err,
                    "dream routing advice load failed"
                );
                Vec::new()
            }
            Err(err) => {
                warn!(
                    workdir = %workdir.display(),
                    error = %err,
                    "dream routing advice task failed"
                );
                Vec::new()
            }
        }
    };

    let (episodes_result, dream_sources) = tokio::join!(episodes_future, dreams_future);

    match episodes_result {
        Ok(episodes) => {
            let matched_ids: HashSet<&str> = playbooks.iter().map(|pb| pb.id.as_str()).collect();
            let mut episode_count = 0usize;
            for episode in episodes.iter().rev().take(100) {
                if !matched_ids.contains(episode.task_id.as_str()) {
                    continue;
                }

                let gate_summary = if episode.gate_verdicts.is_empty() {
                    String::from("no gate verdicts")
                } else {
                    episode
                        .gate_verdicts
                        .iter()
                        .map(|verdict| {
                            format!(
                                "{}:{}",
                                verdict.gate,
                                if verdict.passed { "pass" } else { "fail" }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                sources.push(ProvenanceSource::Episode {
                    task_id: episode.task_id.clone(),
                    success: episode.success,
                    gate_summary,
                });

                episode_count += 1;
                if episode_count == 3 {
                    break;
                }
            }
        }
        Err(err) => {
            warn!(
                workdir = %workdir.display(),
                error = %err,
                "episode log read failed"
            );
        }
    }

    for hit in knowledge_hits.iter().take(3) {
        sources.push(ProvenanceSource::Knowledge {
            kind: hit.entry.kind,
            tier: hit.entry.tier,
            score: hit.total_score,
            summary: truncate_with_limit(hit.entry.content.trim(), 80, "..."),
        });
    }

    for source in dream_sources {
        sources.push(source);
    }

    if sources.is_empty() || (!has_playbook_source && sources.len() < 2) {
        return None;
    }

    let scores = sources
        .iter()
        .map(|source| match source {
            ProvenanceSource::Playbook { success_rate, .. } => *success_rate,
            ProvenanceSource::Episode { success, .. } => {
                if *success {
                    1.0
                } else {
                    0.0
                }
            }
            ProvenanceSource::Knowledge { score, .. } => score_to_confidence(*score),
            ProvenanceSource::DreamPattern { confidence, .. } => *confidence,
        })
        .collect::<Vec<_>>();

    let confidence = if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f64>() / scores.len() as f64
    };

    Some(ProvenanceChain {
        sources,
        confidence,
    })
}

/// Emit a provenance card into ACP updates.
async fn emit_provenance_card(
    chain: &ProvenanceChain,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    let tool_call_id = format!("decision-provenance-{}", uuid::Uuid::new_v4());
    let _ = event_sender
        .send(CognitiveEvent::ToolCallStart {
            tool_call_id: tool_call_id.clone(),
            title: "Decision provenance".to_string(),
            kind: ToolCallKind::Other,
            locations: None,
        })
        .await;
    let _ = event_sender
        .send(CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Completed,
            content: vec![ContentBlock::Text {
                text: render_provenance_card(chain),
            }],
        })
        .await;
}

fn render_provenance_card(chain: &ProvenanceChain) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} source{}, {:.0}% confidence",
        chain.sources.len(),
        if chain.sources.len() == 1 { "" } else { "s" },
        chain.confidence * 100.0
    ));
    lines.push(String::new());

    for source in &chain.sources {
        match source {
            ProvenanceSource::Playbook {
                id,
                goal,
                total_outcomes,
                success_rate,
            } => {
                lines.push(format!(
                    "- Playbook `{id}` ({} runs, {:.0}% success)",
                    total_outcomes,
                    success_rate * 100.0
                ));
                lines.push(format!("  Goal: {}", goal));
            }
            ProvenanceSource::Episode {
                task_id,
                success,
                gate_summary,
            } => {
                lines.push(format!(
                    "- Episode `{task_id}` [{}]",
                    if *success { "pass" } else { "fail" }
                ));
                lines.push(format!(
                    "  Gates: {}",
                    truncate_with_limit(gate_summary, 80, "...")
                ));
            }
            ProvenanceSource::Knowledge {
                kind,
                tier,
                score,
                summary,
            } => {
                lines.push(format!(
                    "- Knowledge [{}/{}] ({:.2})",
                    kind.as_str(),
                    knowledge_tier_label(*tier),
                    score
                ));
                lines.push(format!("  {}", summary));
            }
            ProvenanceSource::DreamPattern {
                description,
                guidance,
                confidence,
            } => {
                lines.push(format!("- Dream pattern ({:.0}%)", confidence * 100.0));
                lines.push(format!("  Description: {}", description));
                lines.push(format!("  Guidance: {}", guidance));
            }
        }
        lines.push(String::new());
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

fn prompt_keywords(prompt: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    for raw in prompt.split(|ch: char| !ch.is_alphanumeric()) {
        let keyword = raw.trim().to_ascii_lowercase();
        if keyword.len() <= 4 || keywords.iter().any(|existing| existing == &keyword) {
            continue;
        }

        keywords.push(keyword);
        if keywords.len() == 5 {
            break;
        }
    }

    keywords
}

fn knowledge_tier_label(tier: KnowledgeTier) -> &'static str {
    match tier {
        KnowledgeTier::Transient => "transient",
        KnowledgeTier::Working => "working",
        KnowledgeTier::Consolidated => "consolidated",
        KnowledgeTier::Persistent => "persistent",
    }
}

fn score_to_confidence(score: f64) -> f64 {
    let score = score.max(0.0);
    score / (1.0 + score)
}

// ── Slash command dispatch ───────────────────────────────────────────

/// Runs a roko CLI slash command and streams the output as ACP updates.
async fn run_slash_command(
    session_id: &str,
    raw_input: &str,
    workdir: &Path,
    model_key: String,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
    shared_run: crate::session::SharedWorkflowRun,
) -> Result<()> {
    let input = raw_input.trim_start_matches('/');
    let (command, args) = match input.split_once(char::is_whitespace) {
        Some((cmd, rest)) => (cmd.trim(), rest.trim()),
        None => (input.trim(), ""),
    };

    // Helper to send a usage hint and return early.
    macro_rules! require_args {
        ($cmd:expr, $hint:expr) => {
            if args.is_empty() {
                let _ = event_sender
                    .send(CognitiveEvent::TokenChunk(format!(
                        "Usage: /{} {}",
                        $cmd, $hint
                    )))
                    .await;
                let _ = event_sender
                    .send(CognitiveEvent::Complete {
                        stop_reason: StopReason::EndTurn,
                        usage: None,
                    })
                    .await;
                return Ok(());
            }
        };
    }

    // Map slash command names to roko CLI args.
    let cli_args: Vec<String> = match command {
        // ── Status & Diagnostics ──
        "status" => vec!["status".into()],
        "doctor" => vec!["doctor".into()],
        "config" => vec!["config".into(), "show".into()],
        "models" => vec!["config".into(), "models".into(), "list".into()],
        "learn" => vec!["learn".into(), "all".into()],

        // ── Research (foraging phase) ──
        "research" => {
            require_args!("research", "<topic>");
            vec![
                "research".into(),
                "topic".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "search" => {
            require_args!("search", "<query>");
            vec!["research".into(), "search".into(), args.into()]
        }
        "enhance-prd" => {
            require_args!("enhance-prd", "<slug>");
            vec![
                "research".into(),
                "enhance-prd".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }

        // ── Specification (PRD lifecycle) ──
        "prd-idea" => {
            require_args!("prd-idea", "<idea text>");
            vec!["prd".into(), "idea".into(), args.into()]
        }
        "prd-draft" => {
            require_args!("prd-draft", "<slug>");
            vec![
                "prd".into(),
                "draft".into(),
                "new".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "prd-list" => vec!["prd".into(), "list".into()],
        "prd-status" => vec!["prd".into(), "status".into()],
        "prd-plan" => {
            require_args!("prd-plan", "<slug>");
            vec![
                "prd".into(),
                "plan".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "prd-consolidate" => vec!["prd".into(), "consolidate".into()],

        // ── Planning ──
        "plan-list" => vec!["plan".into(), "list".into()],
        "plan-generate" => {
            require_args!("plan-generate", "<description>");
            vec![
                "plan".into(),
                "generate".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "plan-regenerate" => {
            require_args!("plan-regenerate", "<description>");
            vec![
                "plan".into(),
                "regenerate".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "plan-validate" => {
            let dir = if args.is_empty() { "plans/" } else { args };
            vec!["plan".into(), "validate".into(), dir.into()]
        }
        "plan-run" => {
            let dir = if args.is_empty() { "plans/" } else { args };
            vec![
                "plan".into(),
                "run".into(),
                dir.into(),
                "--model".into(),
                model_key.clone(),
            ]
        }

        // ── Implementation & Execution ──
        "run" => {
            require_args!("run", "<prompt>");
            vec![
                "run".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "do" => {
            require_args!("do", "<prompt>");
            vec![
                "do".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "develop" => {
            require_args!("develop", "<prompt>");
            vec![
                "develop".into(),
                "--model".into(),
                model_key.clone(),
                "--yes".into(),
                args.into(),
            ]
        }
        "agents" => vec!["agent".into(), "list".into()],
        "agent-chat" => {
            require_args!("agent-chat", "<agent name>");
            vec![
                "agent".into(),
                "chat".into(),
                "--agent".into(),
                args.into(),
                "--model".into(),
                model_key.clone(),
            ]
        }

        // ── Verification & Gates ──
        "build" => {
            return run_shell_command(
                session_id,
                "cargo build --workspace",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "test" => {
            return run_shell_command(
                session_id,
                "cargo test --workspace",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "clippy" => {
            return run_shell_command(
                session_id,
                "cargo clippy --workspace --no-deps -- -D warnings",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "fmt" => {
            return run_shell_command(
                session_id,
                "cargo +nightly fmt --all --check",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "gate" => {
            // Run the full gate pipeline sequentially.
            return run_shell_command(
                session_id,
                "cargo +nightly fmt --all --check && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace",
                workdir,
                cancel_token, event_sender,
            ).await;
        }

        // ── Knowledge & Dreams ──
        "knowledge" => {
            require_args!("knowledge", "<topic>");
            vec!["knowledge".into(), "query".into(), args.into()]
        }
        "knowledge-stats" => vec!["knowledge".into(), "stats".into()],
        "dream" => vec!["knowledge".into(), "dream".into(), "run".into()],

        // ── Code Intelligence ──
        "index" => {
            let sub = if args.is_empty() { "stats" } else { args };
            let parts: Vec<&str> = sub.splitn(2, char::is_whitespace).collect();
            let mut v = vec!["index".into(), parts[0].into()];
            if parts.len() > 1 {
                v.push(parts[1].into());
            }
            v
        }
        "explain" => {
            require_args!("explain", "<topic>");
            vec!["explain".into(), args.into()]
        }
        "replay" => {
            require_args!("replay", "<hash>");
            vec!["replay".into(), args.into()]
        }

        // ── Feedback & Learning ──
        "learn-router" => vec!["learn".into(), "router".into()],
        "learn-episodes" => vec!["learn".into(), "episodes".into()],
        "learn-tune" => {
            let target = if args.is_empty() { "gates" } else { args };
            vec!["learn".into(), "tune".into(), target.into()]
        }

        // ── New commands (plan-show, plan-resume, analyze, review, agent-start/stop, knowledge-gc/backup, audit) ──
        "plan-show" => {
            require_args!("plan-show", "<name>");
            vec!["plan".into(), "show".into(), args.into()]
        }
        "plan-resume" => {
            let path = if args.is_empty() {
                ".roko/state/executor.json"
            } else {
                args
            };
            vec![
                "plan".into(),
                "run".into(),
                "plans/".into(),
                "--resume-plan".into(),
                path.into(),
            ]
        }
        "analyze" => vec!["research".into(), "analyze".into()],
        "review" => {
            let target = if args.is_empty() { "HEAD~1" } else { args };
            return run_shell_command(
                session_id,
                &format!("git diff {target}"),
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "agent-start" => {
            require_args!("agent-start", "<name>");
            vec!["agent".into(), "start".into(), "--name".into(), args.into()]
        }
        "agent-stop" => {
            require_args!("agent-stop", "<name>");
            vec!["agent".into(), "stop".into(), "--name".into(), args.into()]
        }
        "note" => {
            require_args!("note", "<note text>");
            vec!["note".into(), args.into()]
        }
        "knowledge-gc" => vec!["knowledge".into(), "gc".into()],
        "knowledge-backup" => vec!["knowledge".into(), "backup".into()],
        "audit" => vec!["config".into(), "plugins".into(), "audit".into()],

        // ── Workflow ──
        "workflow" => {
            let sub = if args.is_empty() { "list" } else { args };
            match sub {
                "list" | "status" | "cancel" | "resume" => {
                    let msg = match sub {
                        "list" => "\
Workflow pipelines:
  none     — Single agent, no pipeline (current default)
  express  — Implement → gate → commit (fastest)
  standard — Implement → gate → review → commit
  full     — Strategy → implement → gate → multi-review → commit
  auto     — Select pipeline based on task complexity

Use the Workflow dropdown in the status bar to select, or:
  /express <prompt>      Run express pipeline
  /full <prompt>         Run full pipeline
  /review-this           Review current changes
  /pipeline <name>       Run a named pipeline"
                            .to_string(),
                        "status" => {
                            let guard = shared_run.lock().await;
                            match guard.as_ref() {
                                Some(run) => run.status_summary(),
                                None => "No active workflow run. Start one with /express, /full, or select a workflow in the config dropdown.".to_string(),
                            }
                        }
                        "cancel" => "No active workflow to cancel.".to_string(),
                        "resume" => "No halted workflow to resume.".to_string(),
                        _ => "Unknown workflow subcommand. Use: list, status, cancel, resume"
                            .to_string(),
                    };
                    let _ = event_sender.send(CognitiveEvent::TokenChunk(msg)).await;
                    let _ = event_sender
                        .send(CognitiveEvent::Complete {
                            stop_reason: StopReason::EndTurn,
                            usage: None,
                        })
                        .await;
                    return Ok(());
                }
                _ => {
                    let _ = event_sender
                        .send(CognitiveEvent::TokenChunk(format!(
                            "Unknown workflow subcommand: {sub}\n\nUse: /workflow list | status | cancel | resume"
                        )))
                        .await;
                    let _ = event_sender
                        .send(CognitiveEvent::Complete {
                            stop_reason: StopReason::EndTurn,
                            usage: None,
                        })
                        .await;
                    return Ok(());
                }
            }
        }
        "express" => {
            require_args!("express", "<prompt>");
            let knowledge = query_dispatch_knowledge(workdir, args).await;
            emit_knowledge_card(&knowledge, &event_sender).await;
            let provenance_card =
                build_provenance(&knowledge.hits, &knowledge.playbooks, args, workdir)
                    .await
                    .as_ref()
                    .map(render_provenance_card);
            let knowledge_context = knowledge.context_text();
            if std::env::var_os("ROKO_ACP_LEGACY").is_some() {
                return Ok(crate::runner::run_workflow_pipeline(
                    session_id,
                    args,
                    knowledge_context,
                    provenance_card,
                    workdir,
                    crate::runner::PipelineConfig {
                        template: crate::pipeline::WorkflowTemplate::Express,
                        max_iterations: 2,
                        clippy_enabled: true,
                        tests_enabled: true,
                        review_strictness: "standard".to_string(),
                        model_slug: model_key.clone(),
                        mcp_config: None,
                        sandbox_level: roko_core::config::schema::RunnerSandboxLevel::default(),
                    },
                    cancel_token,
                    event_sender,
                    shared_run,
                )
                .await?);
            }

            run_with_workflow_engine(
                session_id,
                args,
                workdir,
                "express",
                crate::runner::WorkflowEngineOptions {
                    model_key,
                    mcp_config: None,
                    provenance_card,
                    input_messages: Vec::new(),
                    route: crate::runner::AcpWorkflowRoute::LegacyDefault,
                },
                event_sender,
            )
            .await?;
            return Ok(());
        }
        "full" => {
            require_args!("full", "<prompt>");
            let knowledge = query_dispatch_knowledge(workdir, args).await;
            emit_knowledge_card(&knowledge, &event_sender).await;
            let provenance_card =
                build_provenance(&knowledge.hits, &knowledge.playbooks, args, workdir)
                    .await
                    .as_ref()
                    .map(render_provenance_card);
            let knowledge_context = knowledge.context_text();
            if std::env::var_os("ROKO_ACP_LEGACY").is_some() {
                return Ok(crate::runner::run_workflow_pipeline(
                    session_id,
                    args,
                    knowledge_context,
                    provenance_card,
                    workdir,
                    crate::runner::PipelineConfig {
                        template: crate::pipeline::WorkflowTemplate::Full,
                        max_iterations: 2,
                        clippy_enabled: true,
                        tests_enabled: true,
                        review_strictness: "standard".to_string(),
                        model_slug: model_key.clone(),
                        mcp_config: None,
                        sandbox_level: roko_core::config::schema::RunnerSandboxLevel::default(),
                    },
                    cancel_token,
                    event_sender,
                    shared_run,
                )
                .await?);
            }

            run_with_workflow_engine(
                session_id,
                args,
                workdir,
                "full",
                crate::runner::WorkflowEngineOptions {
                    model_key,
                    mcp_config: None,
                    provenance_card,
                    input_messages: Vec::new(),
                    route: crate::runner::AcpWorkflowRoute::LegacyDefault,
                },
                event_sender,
            )
            .await?;
            return Ok(());
        }
        "review-this" => {
            return run_shell_command(session_id, "git diff", workdir, cancel_token, event_sender)
                .await;
        }
        "pipeline" => {
            require_args!("pipeline", "<name>");
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "[Pipeline: {args}] Not yet implemented. Available: express, standard, full\n\nUse /workflow list to see all pipelines."
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }

        // ── Help ──
        "help" => {
            let help_text = "\
Available commands (organized by Will's core loop):

  Status & Diagnostics
    /status            Workspace status, signals, agents, runs
    /doctor            Diagnose workspace bootstrap state
    /config            Show roko.toml configuration
    /learn             Learning state overview

  Research (foraging)
    /research <topic>  Deep research with citations (Perplexity)
    /search <query>    Quick web search
    /enhance-prd <slug> Enrich a PRD with web research

  Specification (PRD lifecycle)
    /prd-idea <text>   Capture a work item idea
    /prd-draft <slug>  Draft a new PRD
    /prd-list          List all PRDs
    /prd-status        PRD pipeline coverage report
    /prd-plan <slug>   Generate plan from published PRD
    /prd-consolidate   Scan PRDs for gaps and duplicates

  Planning
    /plan-list         List all plans
    /plan-show <name>  Show a specific plan
    /plan-generate     Generate plan from a prompt
    /plan-validate     Lint tasks.toml without executing
    /plan-run [dir]    Execute a plan (orchestrate→gate→persist)
    /plan-resume [path] Resume an interrupted plan run

  Implementation & Execution
    /run <prompt>      Single prompt → universal loop
    /develop <prompt>  Full pipeline: scope → plan → execute → gate
    /agents            List agents and their status
    /agent-chat <name> Interactive chat with a specific agent
    /agent-start <name> Start a named agent
    /agent-stop <name>  Stop a running agent

  Verification & Gates
    /build             cargo build --workspace
    /test              cargo test --workspace
    /clippy            cargo clippy --workspace
    /fmt               cargo +nightly fmt --all --check
    /gate              Full pipeline: fmt + clippy + test
    /review [target]   git diff of target (default: HEAD~1)

  Research & Analysis
    /research <topic>  Deep research with citations (Perplexity)
    /search <query>    Quick web search
    /enhance-prd <slug> Enrich a PRD with web research
    /analyze           Analyze execution data

  Knowledge & Dreams
    /knowledge <topic> Query durable knowledge store
    /knowledge-stats   Knowledge store statistics
    /knowledge-gc      Garbage collect knowledge store
    /knowledge-backup  Backup knowledge store
    /dream             Dream consolidation (NREM→REM→integration)

  Code Intelligence
    /index [cmd]       Build/search/stats code index
    /explain <topic>   Explain a concept at 3 depth levels
    /replay <hash>     Walk signal DAG by hash

  Feedback & Learning
    /learn-router      Cascade router state and model routing
    /learn-episodes    Recent episode log
    /learn-tune [what] Tune adaptive thresholds

  Workflow Pipelines
    /workflow [sub]    list/status/cancel/resume workflows
    /express <prompt>  Express: implement → gate → commit
    /full <prompt>     Full: strategy → implement → gate → review → commit
    /review-this       Review current uncommitted changes
    /pipeline <name>   Run a named workflow pipeline

  System
    /audit             Plugin security audit

  /help               This message";
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(help_text.into()))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }

        _ => {
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "Unknown command: /{command}\n\nType /help for available commands."
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }
    };

    info!(session_id, command, ?cli_args, "executing slash command");

    // Find the roko binary.
    let roko_bin = std::env::current_exe().unwrap_or_else(|_| "roko".into());

    let mut child = match tokio::process::Command::new(&roko_bin)
        .args(&cli_args)
        .current_dir(workdir)
        .env("ROKO_ACP_PROGRESS", "1")
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let message = format!("Failed to run `roko {}`: {e}", cli_args.join(" "));
            let _ = event_sender
                .send(CognitiveEvent::Failure {
                    message: message.clone(),
                })
                .await;
            return Err(anyhow::anyhow!(message).into());
        }
    };

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let stream_outcome =
        forward_slash_command_streams(session_id, stdout, stderr, &cancel_token, &event_sender)
            .await;

    let SlashCommandStreamOutcome::Completed { had_output } = stream_outcome else {
        if let Err(error) =
            roko_agent::process::kill_tree(&mut child, Duration::from_millis(200)).await
        {
            warn!(session_id, %error, "failed to terminate slash command process tree");
        }
        return Ok(());
    };

    let exit_status = tokio::select! {
        _ = cancel_token.cancelled() => {
            if let Err(error) =
                roko_agent::process::kill_tree(&mut child, Duration::from_millis(200)).await
            {
                warn!(session_id, %error, "failed to terminate slash command process tree");
            }
            return Ok(());
        }
        status = child.wait() => status
    };
    let exit_status = exit_status.map_err(|error| {
        anyhow::anyhow!("failed waiting for `roko {}`: {error}", cli_args.join(" "))
    })?;
    if !exit_status.success() {
        let message = format!(
            "`roko {}` exited with status {}",
            cli_args.join(" "),
            exit_status
        );
        let _ = event_sender
            .send(CognitiveEvent::Failure {
                message: message.clone(),
            })
            .await;
        return Err(anyhow::anyhow!(message).into());
    }
    finish_slash_command_stream(command, had_output, &event_sender).await;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlashCommandStreamOutcome {
    Completed { had_output: bool },
    Cancelled,
}

async fn forward_slash_command_streams<Stdout, Stderr>(
    session_id: &str,
    stdout: Stdout,
    stderr: Stderr,
    cancel_token: &CancelToken,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) -> SlashCommandStreamOutcome
where
    Stdout: AsyncRead + Unpin,
    Stderr: AsyncRead + Unpin,
{
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut had_output = false;
    let mut progress_task_counter: u64 = 0;
    let mut progress_calls: HashMap<String, VecDeque<String>> = HashMap::new();

    loop {
        if cancel_token.is_cancelled() {
            close_progress_calls(&mut progress_calls, "cancelled", event_sender).await;
            return SlashCommandStreamOutcome::Cancelled;
        }
        if stdout_done && stderr_done {
            break;
        }
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                close_progress_calls(
                    &mut progress_calls,
                    "cancelled",
                    event_sender,
                )
                .await;
                return SlashCommandStreamOutcome::Cancelled;
            }
            line = stdout_lines.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        if let Some(json_str) = l.strip_prefix("ROKO_PROGRESS: ") {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                                match value.get("type").and_then(|t| t.as_str()) {
                                    Some("task_started") => {
                                        progress_task_counter += 1;
                                        let title = value.get("title")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("task");
                                        let task_id = value.get("task_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let call_id = format!("progress-{}-{}", task_id, progress_task_counter);
                                        progress_calls
                                            .entry(task_id.to_owned())
                                            .or_default()
                                            .push_back(call_id.clone());
                                        let _ = event_sender.send(CognitiveEvent::ToolCallStart {
                                            tool_call_id: call_id,
                                            title: title.to_string(),
                                            kind: ToolCallKind::Terminal,
                                            locations: None,
                                        }).await;
                                    }
                                    Some("task_completed") => {
                                        let task_id = value.get("task_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let completed = value.get("completed")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        let total = value.get("total")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        let call_id = pop_progress_call(
                                            &mut progress_calls,
                                            task_id,
                                        )
                                            .unwrap_or_else(|| format!("progress-{}-unmatched", task_id));
                                        let _ = event_sender.send(CognitiveEvent::ToolCallComplete {
                                            tool_call_id: call_id,
                                            status: ToolCallStatus::Completed,
                                            content: vec![ContentBlock::Text {
                                                text: format!("{}/{} tasks done", completed, total),
                                            }],
                                        }).await;
                                    }
                                    Some("task_failed") => {
                                        let task_id = value.get("task_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let error = value.get("error")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("task failed");
                                        if let Some(call_id) = pop_progress_call(
                                            &mut progress_calls,
                                            task_id,
                                        ) {
                                            let _ = event_sender.send(CognitiveEvent::ToolCallComplete {
                                                tool_call_id: call_id,
                                                status: ToolCallStatus::Failed,
                                                content: vec![ContentBlock::Text {
                                                    text: error.to_owned(),
                                                }],
                                            }).await;
                                        } else {
                                            let _ = event_sender
                                                .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                                .await;
                                        }
                                    }
                                    Some("agent_started") => {
                                        let provider = value.get("provider")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let model = value.get("model")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let _ = event_sender.send(CognitiveEvent::TokenChunk(
                                            format!("[agent] {} ({})\n", model, provider),
                                        )).await;
                                    }
                                    _ => {
                                        let _ = event_sender
                                            .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                            .await;
                                    }
                                }
                            } else {
                                let _ = event_sender
                                    .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                    .await;
                            }
                        } else {
                            let _ = event_sender
                                .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                .await;
                        }
                    }
                    Ok(None) => stdout_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading slash command stdout");
                        stdout_done = true;
                    }
                }
            }
            line = stderr_lines.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        let _ = event_sender
                            .send(CognitiveEvent::TokenChunk(format!("\x1b[2m{l}\x1b[0m\n")))
                            .await;
                    }
                    Ok(None) => stderr_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading slash command stderr");
                        stderr_done = true;
                    }
                }
            }
        }
    }

    close_progress_calls(
        &mut progress_calls,
        "progress stream ended before task completion",
        event_sender,
    )
    .await;
    SlashCommandStreamOutcome::Completed { had_output }
}

fn pop_progress_call(
    progress_calls: &mut HashMap<String, VecDeque<String>>,
    task_id: &str,
) -> Option<String> {
    let call_id = progress_calls.get_mut(task_id)?.pop_front();
    if progress_calls.get(task_id).is_some_and(VecDeque::is_empty) {
        progress_calls.remove(task_id);
    }
    call_id
}

async fn close_progress_calls(
    progress_calls: &mut HashMap<String, VecDeque<String>>,
    reason: &str,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    let call_ids = progress_calls
        .drain()
        .flat_map(|(_, calls)| calls)
        .collect::<Vec<_>>();
    for tool_call_id in call_ids {
        let _ = event_sender
            .send(CognitiveEvent::ToolCallComplete {
                tool_call_id,
                status: ToolCallStatus::Failed,
                content: vec![ContentBlock::Text {
                    text: reason.to_owned(),
                }],
            })
            .await;
    }
}

async fn finish_slash_command_stream(
    command: &str,
    had_output: bool,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    if !had_output {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(format!(
                "/{command} completed (no output)"
            )))
            .await;
    }
    let _ = event_sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .await;
}

/// Runs a raw shell command (for /build, /test, /clippy) and streams each
/// stdout line as a TokenChunk immediately via tokio::select with cancel_token.
async fn run_shell_command(
    session_id: &str,
    shell_cmd: &str,
    workdir: &Path,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>, // streams each line as TokenChunk
) -> Result<()> {
    info!(session_id, shell_cmd, "executing shell command");

    let mut child = match tokio::process::Command::new("sh")
        .args(["-c", shell_cmd])
        .current_dir(workdir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "Failed to run `{shell_cmd}`: {e}"
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }
    };

    // Interleave stdout and stderr reading.
    let mut stdout_lines =
        tokio::io::BufReader::new(child.stdout.take().expect("stdout was piped")).lines();
    let mut stderr_lines =
        tokio::io::BufReader::new(child.stderr.take().expect("stderr was piped")).lines();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut had_output = false;

    loop {
        if stdout_done && stderr_done {
            break;
        }
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                let _ = child.kill().await;
                return Ok(());
            }
            line = stdout_lines.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        let _ = event_sender
                            .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                            .await;
                    }
                    Ok(None) => stdout_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading shell command stdout");
                        stdout_done = true;
                    }
                }
            }
            line = stderr_lines.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        let _ = event_sender
                            .send(CognitiveEvent::TokenChunk(format!("\x1b[2m{l}\x1b[0m\n")))
                            .await;
                    }
                    Ok(None) => stderr_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading shell command stderr");
                        stderr_done = true;
                    }
                }
            }
        }
    }

    let exit_status = child.wait().await;
    let code = exit_status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    if code != 0 {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(format!(
                "\n\nProcess exited with code {code}"
            )))
            .await;
    }

    if !had_output {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(format!(
                "`{shell_cmd}` completed (no output)"
            )))
            .await;
    }
    let _ = event_sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .await;

    Ok(())
}

/// Maps a Claude tool name to an ACP tool call kind.
#[cfg(test)]
fn tool_name_to_kind(name: &str) -> ToolCallKind {
    match name {
        "Edit" | "MultiEdit" => ToolCallKind::Edit,
        "Write" => ToolCallKind::Create,
        "Bash" | "Terminal" => ToolCallKind::Terminal,
        _ => ToolCallKind::Other,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn map_event_to_update(event: CognitiveEvent) -> SessionUpdate {
    match event {
        CognitiveEvent::TokenChunk(text) => SessionUpdate::AgentMessageChunk {
            content: text_block(text),
            _meta: None,
        },
        CognitiveEvent::ThinkingChunk(text) => SessionUpdate::AgentThoughtChunk {
            content: text_block(text),
        },
        CognitiveEvent::ToolCallStart {
            tool_call_id,
            title,
            kind,
            locations,
        } => SessionUpdate::ToolCall {
            tool_call_id,
            title,
            kind,
            status: ToolCallStatus::InProgress,
            content: Vec::new(),
            locations,
        },
        CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status,
            content,
        } => SessionUpdate::ToolCallUpdate {
            tool_call_id,
            status,
            content,
            locations: None,
        },
        CognitiveEvent::PlanUpdate { entries } => SessionUpdate::Plan { entries },
        CognitiveEvent::McpStatus { statuses } => SessionUpdate::McpStatusUpdate { statuses },
        CognitiveEvent::Complete { .. }
        | CognitiveEvent::Failure { .. }
        | CognitiveEvent::MaxTokens
        | CognitiveEvent::PermissionRequest { .. } => {
            unreachable!("terminal/async cognitive events are handled before update mapping")
        }
    }
}

fn dispatch_failure_update(message: String) -> SessionUpdate {
    SessionUpdate::AgentMessageChunk {
        content: text_block(message),
        _meta: None,
    }
}

/// Pattern-match known error strings and return an actionable user-facing message.
/// The original error is always appended so nothing is suppressed.
fn format_acp_error_for_user(error: &str) -> String {
    // Missing API key – extract the env var name and tell the user how to set it.
    if let Some(rest) = error
        .strip_prefix("Missing API key: env var ")
        .or_else(|| error.strip_prefix("missing API key: env var "))
    {
        let var = rest.split_whitespace().next().unwrap_or(rest);
        return format!(
            "Set {var} in your environment. Run: export {var}=your-key\n\n(Original error: {error})"
        );
    }

    // OpenAI-compat models that reject max_tokens in favour of max_completion_tokens.
    if error.contains("max_tokens is not supported") {
        return format!(
            "This model needs use_max_completion_tokens. Auto-fixing...\n\n(Original error: {error})"
        );
    }

    // Model not found / not configured.
    if error.contains("model not found") || error.contains("model_not_found") {
        return format!(
            "Model isn't configured or doesn't exist. Check your roko.toml [models] section.\n\n(Original error: {error})"
        );
    }

    // Rate limiting (HTTP 429 or explicit rate_limit error code).
    if error.contains("rate_limit") || error.contains("429") {
        return format!("Rate limited. Waiting 30s before retry...\n\n(Original error: {error})");
    }

    // Context length exceeded.
    if error.contains("context_length_exceeded") || error.contains("maximum context length") {
        return format!(
            "Prompt too long for this model. Consider truncating or switching to a model with a larger context window.\n\n(Original error: {error})"
        );
    }

    // No pattern matched – return as-is.
    error.to_string()
}

async fn emit_dispatch_failure(event_sender: &mpsc::Sender<CognitiveEvent>, message: String) {
    let message = format_acp_error_for_user(&message);
    send_cognitive_event(event_sender, CognitiveEvent::Failure { message }).await;
}

async fn send_cognitive_event(event_sender: &mpsc::Sender<CognitiveEvent>, event: CognitiveEvent) {
    if event_sender.send(event).await.is_err() {
        debug!("cognitive event receiver dropped before event could be delivered");
    }
}

async fn send_session_update<R, W>(
    transport: &mut StdioTransport<R, W>,
    session_id: &str,
    update: SessionUpdate,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let update_value = serde_json::to_value(update)?;
    let params = serde_json::json!({
        "sessionId": session_id,
        "update": update_value,
    });
    transport
        .send_notification("session/update", params)
        .await
        .map_err(BridgeEventsError::from)
}

fn extract_prompt_text(prompt: &[ContentBlock]) -> String {
    prompt
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => text.clone(),
            ContentBlock::Resource { .. } => String::new(),
            ContentBlock::Image { mime_type, .. } => format!("[image: {mime_type}]"),
            ContentBlock::Diff { path, diff, .. } => {
                format!("diff {path}:\n{}", diff.as_deref().unwrap_or(""))
            }
            ContentBlock::Unknown => {
                tracing::debug!("skipping unknown content block type in prompt");
                String::new()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn model_input_blocks_from_prompt(prompt: &[ContentBlock]) -> Vec<ModelInputBlock> {
    prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } if !text.is_empty() => {
                Some(ModelInputBlock::text(text.clone()))
            }
            ContentBlock::Image { data, mime_type } => {
                Some(ModelInputBlock::image(mime_type.clone(), data.clone()))
            }
            ContentBlock::Diff { path, diff, .. } => Some(ModelInputBlock::text(format!(
                "diff {path}:\n{}",
                diff.as_deref().unwrap_or("")
            ))),
            ContentBlock::Text { .. } | ContentBlock::Resource { .. } | ContentBlock::Unknown => {
                None
            }
        })
        .collect()
}

/// Builds an OpenAI-compatible content array from prompt blocks, converting
/// `Image` blocks into `image_url` content parts with inline data URIs.
/// Returns `None` when no images are present (caller can use a plain string).
fn build_openai_content_parts(prompt: &[ContentBlock]) -> Option<Vec<serde_json::Value>> {
    let has_image = prompt
        .iter()
        .any(|b| matches!(b, ContentBlock::Image { .. }));
    if !has_image {
        return None;
    }
    let mut parts = Vec::new();
    for block in model_input_blocks_from_prompt(prompt) {
        match block {
            ModelInputBlock::Text { text } => {
                parts.push(serde_json::json!({"type": "text", "text": text}));
            }
            ModelInputBlock::Image { data, media_type } => {
                parts.push(serde_json::json!({
                    "type": "image_url",
                    "image_url": { "url": format!("data:{media_type};base64,{data}") }
                }));
            }
        }
    }
    Some(parts)
}

/// Converts prompt blocks into Anthropic multi-part content (text + base64 image).
/// Returns `None` when there are no image blocks, so the caller can skip replacement.
fn build_anthropic_content_parts(prompt: &[ContentBlock]) -> Option<Vec<serde_json::Value>> {
    let has_image = prompt
        .iter()
        .any(|b| matches!(b, ContentBlock::Image { .. }));
    if !has_image {
        return None;
    }
    let mut parts = Vec::new();
    for block in model_input_blocks_from_prompt(prompt) {
        match block {
            ModelInputBlock::Text { text } => {
                parts.push(serde_json::json!({"type": "text", "text": text}));
            }
            ModelInputBlock::Image { data, media_type } => {
                parts.push(serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": data,
                    }
                }));
            }
        }
    }
    Some(parts)
}

/// Replaces the last user message's `content` field in `msgs` with a multi-part
/// content array when `prompt` contains `Image` blocks.  No-ops when there are
/// no image blocks so text-only prompts are unaffected.
fn inject_image_parts(
    msgs: &mut [serde_json::Value],
    prompt: &[ContentBlock],
    provider_kind: ProviderKind,
) {
    let mut image_parts = if provider_kind == ProviderKind::AnthropicApi {
        build_anthropic_content_parts(prompt)
    } else {
        build_openai_content_parts(prompt)
    };
    if let Some(last) = msgs.last_mut()
        && last.get("role").and_then(|v| v.as_str()) == Some("user")
        && let Some(parts) = image_parts.as_mut()
    {
        let prompt_text = extract_prompt_text(prompt);
        if let Some(existing) = last.get("content").and_then(serde_json::Value::as_str)
            && let Some(suffix) = existing.strip_prefix(&prompt_text)
            && !suffix.is_empty()
        {
            parts.push(serde_json::json!({"type": "text", "text": suffix}));
        }
        last["content"] = serde_json::Value::Array(std::mem::take(parts));
    }
}

fn model_input_messages_from_wire(
    messages: &[serde_json::Value],
) -> std::result::Result<Vec<ModelInputMessage>, String> {
    let mut structured = Vec::with_capacity(messages.len());
    for (message_index, message) in messages.iter().enumerate() {
        let role = match message.get("role").and_then(serde_json::Value::as_str) {
            Some("system") => MessageRole::System,
            Some("user") => MessageRole::User,
            Some("assistant") => MessageRole::Assistant,
            Some(other) => {
                return Err(format!(
                    "message {} has unsupported role {other:?}",
                    message_index + 1
                ));
            }
            None => return Err(format!("message {} has no role", message_index + 1)),
        };
        let content = message
            .get("content")
            .ok_or_else(|| format!("message {} has no content", message_index + 1))?;
        let blocks = if let Some(text) = content.as_str() {
            vec![ModelInputBlock::text(text)]
        } else if let Some(parts) = content.as_array() {
            parts
                .iter()
                .enumerate()
                .map(|(part_index, part)| {
                    match part.get("type").and_then(serde_json::Value::as_str) {
                        Some("text") => part
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .map(|text| ModelInputBlock::text(text.to_string()))
                            .ok_or_else(|| {
                                format!(
                                    "message {} part {} has no text",
                                    message_index + 1,
                                    part_index + 1
                                )
                            }),
                        Some("image") => {
                            let source = part.get("source").ok_or_else(|| {
                                format!(
                                    "message {} image part {} has no source",
                                    message_index + 1,
                                    part_index + 1
                                )
                            })?;
                            let media_type = source
                                .get("media_type")
                                .and_then(serde_json::Value::as_str)
                                .ok_or_else(|| "Anthropic image has no media_type".to_string())?;
                            let data = source
                                .get("data")
                                .and_then(serde_json::Value::as_str)
                                .ok_or_else(|| "Anthropic image has no data".to_string())?;
                            Ok(ModelInputBlock::image(media_type, data))
                        }
                        Some("image_url") => {
                            let uri = part
                                .pointer("/image_url/url")
                                .and_then(serde_json::Value::as_str)
                                .ok_or_else(|| "OpenAI image has no data URI".to_string())?;
                            let encoded = uri.strip_prefix("data:").ok_or_else(|| {
                                "OpenAI image URL is not an inline data URI".to_string()
                            })?;
                            let (media_type, data) =
                                encoded.split_once(";base64,").ok_or_else(|| {
                                    "OpenAI image data URI is not base64 encoded".to_string()
                                })?;
                            Ok(ModelInputBlock::image(media_type, data))
                        }
                        Some(other) => Err(format!(
                            "message {} part {} has unsupported type {other:?}",
                            message_index + 1,
                            part_index + 1
                        )),
                        None => Err(format!(
                            "message {} part {} has no type",
                            message_index + 1,
                            part_index + 1
                        )),
                    }
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            return Err(format!(
                "message {} content is neither text nor an array",
                message_index + 1
            ));
        };
        structured.push(ModelInputMessage::new(role, blocks));
    }
    validate_model_input_messages(&structured)?;
    Ok(structured)
}

/// Extracts `file://` URIs from Resource blocks in the prompt.
fn extract_resource_uris(prompt: &[ContentBlock]) -> Vec<String> {
    use crate::types::ResourceRef;
    prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Resource {
                resource: ResourceRef::File { uri },
            } => Some(uri.clone()),
            _ => None,
        })
        .collect()
}

/// Reads file contents for the given URIs, returning XML-tagged file context.
/// Validates that paths stay within the workdir for security.
fn read_file_context(uris: &[String], workdir: &Path) -> String {
    let mut context = String::new();
    let workdir_canonical = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());

    for uri in uris {
        let path_str = uri.strip_prefix("file://").unwrap_or(uri);
        let path = PathBuf::from(path_str);

        // Security: ensure path is within workdir.
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !canonical.starts_with(&workdir_canonical) {
            warn!(path = %path.display(), "skipping file outside workdir");
            continue;
        }

        match std::fs::read_to_string(&canonical) {
            Ok(contents) => {
                // Cap individual file at 32KB to avoid blowing up context.
                let truncated = truncate_with_limit(&contents, 32_768, "... [truncated at 32KB]");
                let rel_path = canonical
                    .strip_prefix(&workdir_canonical)
                    .unwrap_or(&canonical);
                context.push_str(&format!(
                    "<file path=\"{}\">\n{}\n</file>\n",
                    rel_path.display(),
                    truncated
                ));
            }
            Err(e) => {
                warn!(path = %canonical.display(), error = %e, "failed to read file for context");
            }
        }
    }

    context
}

/// Resolves context annotations in prompt blocks into a single context string.
///
/// Explicit file attachments are resolved as XML-tagged file content. Text
/// blocks are scanned for `@` mentions and each supported mention is resolved
/// to either git context or file content.
pub(crate) async fn resolve_context_items(prompt: &[ContentBlock], workdir: &Path) -> String {
    use crate::types::ResourceRef;

    let mut parts = Vec::new();

    for block in prompt {
        match block {
            ContentBlock::Resource {
                resource: ResourceRef::File { uri },
            } => match resolve_file_uri(uri, workdir).await {
                Ok(content) => parts.push(content),
                Err(error) => {
                    warn!(uri = %uri, error = %error, "failed to resolve file resource URI");
                }
            },
            ContentBlock::Text { text } => {
                for label in extract_at_mentions(text) {
                    match resolve_at_mention(&label, workdir).await {
                        Ok(content) => parts.push(content),
                        Err(error) => {
                            warn!(label = %label, error = %error, "failed to resolve @-mention");
                        }
                    }
                }
            }
            ContentBlock::Image { .. } | ContentBlock::Diff { .. } => {}
            ContentBlock::Unknown => {
                tracing::debug!("skipping unknown content block in context resolution");
            }
        }
    }

    parts.join("\n\n")
}

async fn resolve_file_uri(uri: &str, workdir: &Path) -> anyhow::Result<String> {
    let path_str = uri.strip_prefix("file://").unwrap_or(uri);
    let (rel_path, contents) = resolve_local_file_contents(Path::new(path_str), workdir).await?;
    Ok(format!(
        "<file path=\"{}\">\n{}\n</file>",
        rel_path.display(),
        contents
    ))
}

async fn resolve_at_mention(label: &str, workdir: &Path) -> anyhow::Result<String> {
    match label {
        "branch-diff" | "diff" => {
            let output = tokio::process::Command::new("git")
                .args(["diff"])
                .current_dir(workdir)
                .output()
                .await?;
            ensure_git_output_success(&output, "git diff")?;
            let diff = String::from_utf8_lossy(&output.stdout);
            let truncated = truncate_with_limit(&diff, 10_240, "...\n[truncated]");
            Ok(format!("--- branch diff ---\n{truncated}"))
        }
        "recent-commits" | "git-log" | "log" => {
            let output = tokio::process::Command::new("git")
                .args(["log", "--oneline", "-20"])
                .current_dir(workdir)
                .output()
                .await?;
            ensure_git_output_success(&output, "git log")?;
            let log = String::from_utf8_lossy(&output.stdout);
            let truncated = truncate_with_limit(&log, 10_240, "...\n[truncated]");
            Ok(format!("--- recent commits ---\n{truncated}"))
        }
        "status" | "git-status" => {
            let output = tokio::process::Command::new("git")
                .args(["status", "--short"])
                .current_dir(workdir)
                .output()
                .await?;
            ensure_git_output_success(&output, "git status")?;
            let status = String::from_utf8_lossy(&output.stdout);
            let truncated = truncate_with_limit(&status, 10_240, "...\n[truncated]");
            Ok(format!("--- git status ---\n{truncated}"))
        }
        _ => {
            let (rel_path, contents) =
                resolve_local_file_contents(Path::new(label), workdir).await?;
            Ok(format!("--- {} ---\n{contents}", rel_path.display()))
        }
    }
}

async fn resolve_local_file_contents(
    path: &Path,
    workdir: &Path,
) -> anyhow::Result<(PathBuf, String)> {
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workdir.join(path)
    };

    let workdir_canonical = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let canonical = full_path
        .canonicalize()
        .map_err(|error| anyhow::anyhow!("cannot canonicalize {}: {error}", full_path.display()))?;
    if !canonical.starts_with(&workdir_canonical) {
        return Err(anyhow::anyhow!(
            "path {} is outside workdir",
            canonical.display()
        ));
    }

    let contents = tokio::fs::read_to_string(&canonical).await?;
    let rel_path = canonical
        .strip_prefix(&workdir_canonical)
        .unwrap_or(&canonical)
        .to_path_buf();
    let truncated = truncate_with_limit(&contents, 32_768, "... [truncated at 32KB]");

    Ok((rel_path, truncated))
}

fn extract_at_mentions(text: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut search_start = 0;

    while let Some(relative_at) = text[search_start..].find('@') {
        let at_index = search_start + relative_at;
        let prev = text[..at_index].chars().next_back();
        if matches!(prev, Some(c) if c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
            search_start = at_index + 1;
            continue;
        }

        let mut end = at_index + 1;
        while end < text.len() {
            let ch = text[end..].chars().next().expect("valid char boundary");
            if ch.is_whitespace()
                || ch == '@'
                || matches!(
                    ch,
                    ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '<' | '>'
                )
            {
                break;
            }
            end += ch.len_utf8();
        }

        let label = text[at_index + 1..end].trim_matches(|ch: char| {
            matches!(
                ch,
                ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '<' | '>' | '\'' | '"'
            )
        });
        if !label.is_empty() && !label.starts_with('@') {
            mentions.push(label.to_owned());
        }

        search_start = end;
    }

    mentions
}

fn truncate_with_limit(text: &str, limit: usize, suffix: &str) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }

    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = String::with_capacity(end + suffix.len());
    truncated.push_str(&text[..end]);
    truncated.push_str(suffix);
    truncated
}

fn ensure_git_output_success(output: &std::process::Output, command: &str) -> anyhow::Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        Err(anyhow::anyhow!("{command} failed"))
    } else {
        Err(anyhow::anyhow!("{command} failed: {stderr}"))
    }
}

fn workflow_template_name(template: &crate::pipeline::WorkflowTemplate) -> &'static str {
    match template {
        crate::pipeline::WorkflowTemplate::Express => "express",
        crate::pipeline::WorkflowTemplate::Standard => "standard",
        crate::pipeline::WorkflowTemplate::Full => "full",
    }
}

fn text_block(text: String) -> ContentBlock {
    ContentBlock::Text { text }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex, empty};

    use super::*;
    use crate::{
        session::AcpSession,
        transport::StdioTransport,
        types::{JsonRpcNotification, PermissionDecision, SessionNewParams},
    };

    fn test_session(model: &str, workflow: &str) -> AcpSession {
        let mut session = AcpSession::new(SessionNewParams {
            session_name: None,
            client_capabilities: None,
            model: None,
            provider: None,
            effort: None,
            mcp_servers: Vec::new(),
        });
        session.config_state.model = model.to_string();
        session.config_state.workflow = workflow.to_string();
        session
    }

    async fn reply_to_permission_request<C>(client: C, result: serde_json::Value)
    where
        C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read permission request");
        let request: serde_json::Value = serde_json::from_str(&line).expect("parse request");
        let request_id = request["id"].clone();
        let mut client = reader.into_inner();
        let response = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": result,
        });
        let payload = serde_json::to_vec(&response).expect("serialize response");
        client
            .write_all(&payload)
            .await
            .expect("write response bytes");
        client.write_all(b"\n").await.expect("write newline");
        client.flush().await.expect("flush response");
    }

    #[test]
    fn model_call_request_from_acp_messages_preserves_roles() {
        let request = model_call_request_from_acp_messages(
            "claude-sonnet-4-6",
            &[
                json!({"role": "system", "content": "system text"}),
                json!({"role": "user", "content": "hello"}),
                json!({"role": "assistant", "content": "hi"}),
            ],
            Vec::new(),
        )
        .expect("valid ACP messages");

        assert_eq!(request.model, "claude-sonnet-4-6");
        assert_eq!(request.caller.as_deref(), Some("acp"));
        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, MessageRole::System);
        assert_eq!(request.messages[0].content, "system text");
        assert_eq!(request.messages[1].role, MessageRole::User);
        assert_eq!(request.messages[1].content, "hello");
        assert_eq!(request.messages[2].role, MessageRole::Assistant);
        assert_eq!(request.messages[2].content, "hi");
    }

    #[test]
    fn acp_prompt_conversions_preserve_text_image_diff_order() {
        let prompt = vec![
            ContentBlock::Text {
                text: "before".to_string(),
            },
            ContentBlock::Image {
                data: "aGVsbG8=".to_string(),
                mime_type: "image/png".to_string(),
            },
            ContentBlock::Diff {
                path: "src/lib.rs".to_string(),
                old_text: None,
                new_text: None,
                diff: Some("+added".to_string()),
            },
            ContentBlock::Text {
                text: "after".to_string(),
            },
        ];

        let anthropic = build_anthropic_content_parts(&prompt).expect("Anthropic parts");
        assert_eq!(anthropic[0]["text"], "before");
        assert_eq!(anthropic[1]["source"]["data"], "aGVsbG8=");
        assert_eq!(anthropic[2]["text"], "diff src/lib.rs:\n+added");
        assert_eq!(anthropic[3]["text"], "after");

        let openai = build_openai_content_parts(&prompt).expect("OpenAI parts");
        assert_eq!(openai[0]["text"], "before");
        assert_eq!(
            openai[1]["image_url"]["url"],
            "data:image/png;base64,aGVsbG8="
        );
        assert_eq!(openai[2]["text"], "diff src/lib.rs:\n+added");
        assert_eq!(openai[3]["text"], "after");
    }

    #[test]
    fn acp_wire_round_trip_retains_multimodal_request_and_rejects_invalid_data() {
        let request = model_call_request_from_acp_messages(
            "gpt-4o",
            &[json!({
                "role": "user",
                "content": [
                    {"type": "text", "text": "before"},
                    {"type": "image_url", "image_url": {
                        "url": "data:image/webp;base64,aGVsbG8="
                    }},
                    {"type": "text", "text": "after"}
                ]
            })],
            Vec::new(),
        )
        .expect("valid image request");
        assert_eq!(request.input_messages.len(), 1);
        assert!(matches!(
            &request.input_messages[0].content[0],
            ModelInputBlock::Text { text } if text == "before"
        ));
        assert!(matches!(
            &request.input_messages[0].content[1],
            ModelInputBlock::Image { media_type, data }
                if media_type == "image/webp" && data == "aGVsbG8="
        ));
        assert!(matches!(
            &request.input_messages[0].content[2],
            ModelInputBlock::Text { text } if text == "after"
        ));

        let invalid = model_call_request_from_acp_messages(
            "gpt-4o",
            &[json!({
                "role": "user",
                "content": [{"type": "image_url", "image_url": {
                    "url": "data:image/png;base64,not base64"
                }}]
            })],
            Vec::new(),
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn session_mcp_tool_names_are_provider_safe_and_unique() {
        let mut used = HashSet::new();

        let first = unique_tool_name(
            &format!(
                "{}_{}",
                sanitize_tool_segment("desktop.tools"),
                sanitize_tool_segment("read file")
            ),
            &mut used,
        );
        let second = unique_tool_name(
            &format!(
                "{}_{}",
                sanitize_tool_segment("desktop/tools"),
                sanitize_tool_segment("read:file")
            ),
            &mut used,
        );

        assert_eq!(first, "desktop_tools_read_file");
        assert_eq!(second, "desktop_tools_read_file_2");
        assert!(
            first
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        );
        assert!(first.len() <= 64);
        assert!(second.len() <= 64);
    }

    #[tokio::test]
    async fn anthropic_session_mcp_tools() {
        let server = r#"
            IFS= read -r initialize
            printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
            IFS= read -r list_tools
            printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"Echo input","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":true}}]}}'
        "#;
        let session = AcpSession::new(SessionNewParams {
            session_name: None,
            client_capabilities: Some(ClientCapabilities {
                mcp_servers: Some(true),
                ..Default::default()
            }),
            model: Some("anthropic-test".to_string()),
            provider: Some("anthropic".to_string()),
            effort: None,
            mcp_servers: vec![crate::types::McpServerConfig {
                name: "fixture".to_string(),
                transport: crate::types::McpTransport::Stdio {
                    command: "sh".to_string(),
                    args: vec!["-c".to_string(), server.to_string()],
                },
                discovery_timeout_ms: Some(1_000),
            }],
        });
        let (event_sender, _event_receiver) = mpsc::channel(4);

        let (runtime, statuses) =
            setup_session_mcp_tools(&session.session_id, &session.mcp_servers, event_sender).await;

        assert_eq!(statuses, vec![McpServerStatus::ready("fixture", 1)]);
        assert_eq!(runtime.tools.len(), 1);
        assert_eq!(runtime.tools[0].name, "fixture_echo");
        assert!(runtime.handlers.contains_key("fixture_echo"));
    }

    #[tokio::test]
    async fn capabilities_reflect_session() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let declined = ClientCapabilities {
            fs: Some(crate::types::FsCapabilities {
                read_text_file: true,
                write_text_file: false,
            }),
            terminal: Some(false),
            mcp_servers: Some(false),
        };
        let capabilities = derive_acp_tool_capabilities("code", &declined, false, &HashSet::new());
        assert!(capabilities.read);
        assert!(!capabilities.write);
        assert!(!capabilities.exec);

        let registry = Arc::new(VecToolRegistry::from_tools(acp_builtin_tools()));
        let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpBuiltinHandlerResolver {
            handlers: HashMap::new(),
        });
        let dispatcher = ToolDispatcher::new(registry, resolver);
        let mut context = ToolContext::testing(tmp.path());
        context.capabilities = capabilities;

        for call in [
            ToolCall::new(
                "write-declined",
                "write_file",
                json!({"path": "blocked.txt", "content": "blocked"}),
            ),
            ToolCall::new("exec-declined", "bash", json!({"command": "true"})),
        ] {
            let result = dispatcher.dispatch(call, &context).await;
            assert!(matches!(
                result,
                ToolResult::Err(ToolError::PermissionDenied(message))
                    if message.contains("role grants")
            ));
        }
        assert!(!tmp.path().join("blocked.txt").exists());

        let missing = derive_acp_tool_capabilities(
            "code",
            &ClientCapabilities::default(),
            false,
            &HashSet::new(),
        );
        assert_eq!(missing, ToolPermission::default());

        let elevated_client = ClientCapabilities {
            fs: Some(crate::types::FsCapabilities {
                read_text_file: true,
                write_text_file: true,
            }),
            terminal: Some(true),
            mcp_servers: Some(true),
        };
        let plan = derive_acp_tool_capabilities("plan", &elevated_client, true, &HashSet::new());
        assert!(plan.read);
        assert!(!plan.write);
        assert!(!plan.exec);
    }

    #[tokio::test]
    async fn acp_conformance() {
        use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

        let tmp = tempfile::tempdir().expect("tempdir");

        // Consent: the permission event must arrive before the handler can
        // mutate the worktree, and Reject must leave no side effect.
        let target = tmp.path().join("permission-rejected.txt");
        let (permission_tx, mut permission_rx) = mpsc::channel(4);
        let handler = AcpBuiltinToolHandler {
            tool_name: "write_file".into(),
            session_id: "conformance-session".into(),
            workdir: tmp.path().to_path_buf(),
            event_sender: permission_tx,
        };
        let context = ToolContext::testing(tmp.path());
        let write_task = tokio::spawn(async move {
            handler
                .execute(
                    ToolCall::new(
                        "conformance-write",
                        "write_file",
                        json!({"path": "permission-rejected.txt", "content": "blocked"}),
                    ),
                    &context,
                )
                .await
        });
        let event = permission_rx.recv().await.expect("permission event");
        assert!(!target.exists(), "permission must precede the write");
        match event {
            CognitiveEvent::PermissionRequest { reply, .. } => {
                assert!(reply.reply(PermissionDecision::Reject));
            }
            other => panic!("expected permission request, got {other:?}"),
        }
        assert!(matches!(
            write_task.await.expect("join rejected write"),
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
        assert!(!target.exists(), "Reject must block the write");

        // Experiments: apply both content and model selection, then record the
        // outcome against the assigned experiment even when variant ids overlap.
        let experiment_path = tmp.path().join(".roko/learn/experiments.json");
        std::fs::create_dir_all(experiment_path.parent().expect("experiment parent"))
            .expect("create experiment parent");
        let variant = |content: &str, slug: Option<&str>| PromptVariant {
            id: "shared".into(),
            name: "shared".into(),
            section_name: "constraints".into(),
            content: content.into(),
            slug: slug.map(str::to_string),
            active: true,
        };
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "exp-z",
            "other",
            vec![variant("wrong experiment", None)],
        ));
        store.register(PromptExperiment::new(
            "exp-a",
            "constraints",
            vec![variant("Use the ACP variant.", Some("vision-wire"))],
        ));
        store.save(&experiment_path).expect("save experiments");
        let assignment = assign_acp_experiment(&experiment_path, "code").expect("assignment");
        let mut config = RokoConfig::default();
        config.models.insert(
            "vision-key".into(),
            ModelProfile {
                slug: "vision-wire".into(),
                supports_vision: true,
                ..ModelProfile::default()
            },
        );
        let (assignment, model_override) =
            applicable_acp_experiment(&config, "default", false, Some(assignment));
        let assignment = assignment.expect("applicable assignment");
        assert_eq!(model_override.as_deref(), Some("vision-key"));
        assert!(render_experiment_context(&assignment).contains("Use the ACP variant."));
        record_acp_experiment_outcome(&experiment_path, &assignment, true)
            .expect("record scoped outcome");
        let recorded = ExperimentStore::load_or_new(&experiment_path);
        assert_eq!(
            recorded.get("exp-a").expect("exp-a").stats["shared"].trials,
            1
        );
        assert_eq!(
            recorded.get("exp-z").expect("exp-z").stats["shared"].trials,
            0
        );

        // MCP: an Anthropic-shaped session attachment discovers and exposes
        // the fixture tool without a network service.
        let server = r#"
            IFS= read -r initialize
            printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
            IFS= read -r list_tools
            printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":true}}]}}'
        "#;
        let mcp_session = AcpSession::new(SessionNewParams {
            session_name: None,
            client_capabilities: Some(ClientCapabilities {
                mcp_servers: Some(true),
                ..Default::default()
            }),
            model: Some("anthropic-test".into()),
            provider: Some("anthropic".into()),
            effort: None,
            mcp_servers: vec![crate::types::McpServerConfig {
                name: "fixture".into(),
                transport: crate::types::McpTransport::Stdio {
                    command: "sh".into(),
                    args: vec!["-c".into(), server.into()],
                },
                discovery_timeout_ms: Some(1_000),
            }],
        });
        let (mcp_tx, _mcp_rx) = mpsc::channel(4);
        let (runtime, statuses) =
            setup_session_mcp_tools(&mcp_session.session_id, &mcp_session.mcp_servers, mcp_tx)
                .await;
        assert_eq!(statuses, vec![McpServerStatus::ready("fixture", 1)]);
        assert_eq!(runtime.tools[0].name, "fixture_echo");
        assert!(runtime.handlers.contains_key("fixture_echo"));

        // Capabilities: advertised media support and the ToolContext ceiling
        // must agree with their enforcement helpers.
        let prompt_caps = crate::types::advertised_prompt_capabilities(true);
        let image = vec![ContentBlock::Image {
            data: "aGVsbG8=".into(),
            mime_type: "image/png".into(),
        }];
        let audio = vec![
            serde_json::from_value::<ContentBlock>(json!({
                "type": "audio", "data": "aGVsbG8=", "mimeType": "audio/wav"
            }))
            .expect("deserialize audio fail-closed"),
        ];
        assert!(prompt_caps.image && !prompt_caps.audio);
        assert!(unsupported_prompt_content(&image, &prompt_caps).is_none());
        assert!(build_anthropic_content_parts(&image).is_some());
        assert!(unsupported_prompt_content(&audio, &prompt_caps).is_some());

        let declined = ClientCapabilities {
            fs: Some(crate::types::FsCapabilities {
                read_text_file: true,
                write_text_file: false,
            }),
            terminal: Some(false),
            mcp_servers: Some(false),
        };
        let capabilities = derive_acp_tool_capabilities("code", &declined, false, &HashSet::new());
        let registry = Arc::new(VecToolRegistry::from_tools(acp_builtin_tools()));
        let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpBuiltinHandlerResolver {
            handlers: HashMap::new(),
        });
        let dispatcher = ToolDispatcher::new(registry, resolver);
        let mut denied_context = ToolContext::testing(tmp.path());
        denied_context.capabilities = capabilities;
        let denied = dispatcher
            .dispatch(
                ToolCall::new(
                    "capability-write",
                    "write_file",
                    json!({"path": "capability-blocked.txt", "content": "blocked"}),
                ),
                &denied_context,
            )
            .await;
        assert!(matches!(
            denied,
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
        assert!(!tmp.path().join("capability-blocked.txt").exists());
    }

    #[test]
    fn experiment_assignment_selects_applies_and_records_acp_variant() {
        use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join(".roko/learn/experiments.json");
        std::fs::create_dir_all(path.parent().expect("experiment parent"))
            .expect("create experiment parent");

        let variant = |id: &str, content: &str, slug: Option<&str>| PromptVariant {
            id: id.to_string(),
            name: id.to_string(),
            section_name: "constraints".to_string(),
            content: content.to_string(),
            slug: slug.map(str::to_string),
            active: true,
        };
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "exp-b",
            "style",
            vec![variant("b", "later", None)],
        ));
        store.register(PromptExperiment::new(
            "exp-a",
            "constraints",
            vec![variant(
                "a",
                "Use the selected constraint.",
                Some("vision-model"),
            )],
        ));
        store.save(&path).expect("save experiments");

        let assignment = assign_acp_experiment(&path, "code").expect("active assignment");
        assert_eq!(assignment.experiment_id, "exp-a");
        assert_eq!(assignment.variant_id, "a");
        assert!(render_experiment_context(&assignment).contains("Use the selected constraint."));

        let mut config = RokoConfig::default();
        config.models.insert(
            "configured-vision".to_string(),
            ModelProfile {
                slug: "vision-model".to_string(),
                ..ModelProfile::default()
            },
        );
        assert_eq!(
            experiment_model_key(&config, &assignment).as_deref(),
            Some("configured-vision")
        );

        record_acp_experiment_outcome(&path, &assignment, true).expect("record outcome");
        let recorded = ExperimentStore::load_or_new(&path);
        let stats = &recorded.get("exp-a").expect("experiment").stats["a"];
        assert_eq!(stats.trials, 1);
        assert_eq!(stats.successes, 1);
        let persisted = std::fs::read_to_string(&path).expect("read experiments");
        assert!(persisted.contains("metric_stats"));
    }

    #[test]
    fn concurrent_acp_and_external_experiment_writers_preserve_all_outcomes() {
        use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join(".roko/learn/experiments.json");
        std::fs::create_dir_all(path.parent().expect("experiment parent"))
            .expect("create experiment parent");
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "concurrent-exp",
            "constraints",
            vec![PromptVariant {
                id: "variant".to_string(),
                name: "Variant".to_string(),
                section_name: "constraints".to_string(),
                content: "Concurrent content".to_string(),
                slug: None,
                active: true,
            }],
        ));
        store.save(&path).expect("seed experiments");
        let assignment = AcpExperimentAssignment {
            experiment_id: "concurrent-exp".to_string(),
            variant_id: "variant".to_string(),
            section_name: "constraints".to_string(),
            content: "Concurrent content".to_string(),
            model_slug: None,
        };
        let barrier = Arc::new(std::sync::Barrier::new(3));

        let acp_path = path.clone();
        let acp_assignment = assignment.clone();
        let acp_barrier = barrier.clone();
        let acp_writer = std::thread::spawn(move || {
            acp_barrier.wait();
            for _ in 0..20 {
                record_acp_experiment_outcome(&acp_path, &acp_assignment, true)
                    .expect("record ACP outcome");
            }
        });
        let external_path = path.clone();
        let external_barrier = barrier.clone();
        let external_writer = std::thread::spawn(move || {
            external_barrier.wait();
            for _ in 0..20 {
                ExperimentStore::transaction(&external_path, |store| {
                    if !store.record_outcome_for_experiment("concurrent-exp", "variant", true) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "missing concurrent experiment",
                        ));
                    }
                    Ok(())
                })
                .expect("record external outcome");
            }
        });
        barrier.wait();
        acp_writer.join().expect("ACP writer joins");
        external_writer.join().expect("external writer joins");

        let committed = ExperimentStore::load_or_new(&path);
        let stats = &committed.get("concurrent-exp").expect("experiment").stats["variant"];
        assert_eq!(stats.trials, 40);
        assert_eq!(stats.successes, 40);
    }

    #[test]
    fn anthropic_model_call_config_routes_legacy_claude_to_anthropic_provider() {
        let mut roko_config = RokoConfig::default();
        roko_config.providers.insert(
            "anthropic".to_string(),
            roko_core::config::schema::ProviderConfig {
                kind: ProviderKind::AnthropicApi,
                base_url: Some("https://api.anthropic.com".to_string()),
                api_key_env: Some("TEST_ANTHROPIC_API_KEY".to_string()),
                command: None,
                args: None,
                timeout_ms: Some(DEFAULT_REQUEST_TIMEOUT_MS),
                ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
                connect_timeout_ms: Some(DEFAULT_CONNECT_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
            },
        );

        let config =
            anthropic_model_call_config(&roko_config, "claude-sonnet-4-6", "claude-sonnet-4-6")
                .expect("anthropic provider config");
        let resolved = resolve_model(&config, "claude-sonnet-4-6");

        assert_eq!(resolved.provider_kind, ProviderKind::AnthropicApi);
        assert_eq!(
            resolved
                .profile
                .as_ref()
                .map(|profile| profile.provider.as_str()),
            Some("anthropic")
        );
    }

    #[test]
    fn anthropic_model_call_config_requires_explicit_provider_when_env_values_exist() {
        let mut roko_config = RokoConfig::default();
        roko_config.agent.env = Some(vec![
            ("ANTHROPIC_API_KEY".to_string(), "sk-test".to_string()),
            (
                "ANTHROPIC_BASE_URL".to_string(),
                "https://api.anthropic.com".to_string(),
            ),
        ]);

        assert!(
            !roko_config
                .effective_providers()
                .values()
                .any(|provider| provider.kind == ProviderKind::AnthropicApi)
        );
        assert!(
            anthropic_model_call_config(&roko_config, "claude-sonnet-4-6", "claude-sonnet-4-6")
                .is_none()
        );
    }

    #[tokio::test]
    async fn model_stream_failed_event_emits_failure_event() {
        let (sender, mut receiver) = mpsc::channel(4);
        let mut state = ModelStreamForwardState::default();

        let error = forward_model_stream_event(
            "sess_model_stream",
            &sender,
            &mut state,
            ModelStreamEvent::Failed {
                error: "provider failed".to_string(),
            },
        )
        .await
        .expect_err("failed stream event should error");

        assert!(error.to_string().contains("provider failed"));
        match receiver.recv().await.expect("failure event") {
            CognitiveEvent::Failure { message } => {
                assert_eq!(message, "Error: model stream failed: provider failed");
            }
            other => panic!("expected failure event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn model_stream_usage_and_completion_emit_typed_complete() {
        let (sender, mut receiver) = mpsc::channel(4);
        let mut state = ModelStreamForwardState::default();

        let forwarded = forward_model_stream_event(
            "sess_model_stream",
            &sender,
            &mut state,
            ModelStreamEvent::Usage {
                usage: TokenUsage {
                    input_tokens: 11,
                    output_tokens: 7,
                    total_tokens: 18,
                    cost_usd: 0.0,
                },
            },
        )
        .await
        .expect("usage event");
        assert_eq!(forwarded, ModelStreamForward::Continue);

        let forwarded = forward_model_stream_event(
            "sess_model_stream",
            &sender,
            &mut state,
            ModelStreamEvent::Completed {
                stop_reason: Some("max_tokens".to_string()),
            },
        )
        .await
        .expect("completed event");
        assert_eq!(forwarded, ModelStreamForward::Completed);

        match receiver.recv().await.expect("completion event") {
            CognitiveEvent::Complete { stop_reason, usage } => {
                assert_eq!(stop_reason, StopReason::MaxTokens);
                assert_eq!(
                    usage,
                    Some(UsageInfo {
                        total_tokens: 18,
                        input_tokens: 11,
                        output_tokens: 7,
                        thought_tokens: None,
                        cached_read_tokens: None,
                        cached_write_tokens: None,
                    })
                );
            }
            other => panic!("expected completion event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn model_stream_cancelled_event_emits_cancelled_complete() {
        let (sender, mut receiver) = mpsc::channel(4);
        let mut state = ModelStreamForwardState::default();

        let forwarded = forward_model_stream_event(
            "sess_model_stream",
            &sender,
            &mut state,
            ModelStreamEvent::Cancelled,
        )
        .await
        .expect("cancelled event");
        assert_eq!(forwarded, ModelStreamForward::Completed);

        match receiver.recv().await.expect("completion event") {
            CognitiveEvent::Complete { stop_reason, usage } => {
                assert_eq!(stop_reason, StopReason::Cancelled);
                assert_eq!(usage, None);
            }
            other => panic!("expected completion event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_session_update_emits_wrapped_payload() {
        let (client, server) = duplex(4096);
        let mut transport = StdioTransport::from_io(empty(), server);
        let mut reader = BufReader::new(client);

        send_session_update(
            &mut transport,
            "sess_wrapped",
            SessionUpdate::AgentMessageChunk {
                content: text_block("hello".to_owned()),
                _meta: None,
            },
        )
        .await
        .expect("send session update");

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read notification line");
        let notification: JsonRpcNotification =
            serde_json::from_str(&line).expect("deserialize notification");

        assert_eq!(notification.method, "session/update");
        let params = notification.params.expect("params must be present");
        assert_eq!(params["sessionId"], json!("sess_wrapped"));
        // ACP spec requires updates nested under "update" key.
        let update = &params["update"];
        assert_eq!(update["sessionUpdate"], json!("agent_message_chunk"));
        assert_eq!(
            update["content"],
            json!({ "type": "text", "text": "hello" })
        );
    }

    #[tokio::test]
    async fn stream_events_to_editor_emits_notifications_and_returns_completion() {
        let (client, server) = duplex(4096);
        let mut transport = StdioTransport::from_io(empty(), server);
        let mut reader = BufReader::new(client);
        let cancel_token = CancelToken::new();
        let mut session = test_session("test-model", "none");
        let (sender, receiver) = mpsc::channel(8);

        sender
            .send(CognitiveEvent::TokenChunk("hello".to_owned()))
            .await
            .expect("send token chunk");
        sender
            .send(CognitiveEvent::Complete {
                stop_reason: StopReason::EndTurn,
                usage: Some(UsageInfo {
                    total_tokens: 12,
                    input_tokens: 5,
                    output_tokens: 7,
                    thought_tokens: None,
                    cached_read_tokens: None,
                    cached_write_tokens: None,
                }),
            })
            .await
            .expect("send completion");
        drop(sender);

        let result = stream_events_to_editor(
            &mut transport,
            "sess_test",
            &mut session,
            Path::new("."),
            receiver,
            &cancel_token,
        )
        .await;
        let result = result.expect("stream should succeed");

        assert_eq!(result.prompt_result.stop_reason, StopReason::EndTurn);
        assert_eq!(
            result.usage.as_ref().map(|usage| usage.total_tokens),
            Some(12)
        );
        assert_eq!(
            result.usage.as_ref().map(|usage| usage.input_tokens),
            Some(5)
        );
        assert_eq!(
            result.usage.as_ref().map(|usage| usage.output_tokens),
            Some(7)
        );

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read notification line");
        let notification: JsonRpcNotification =
            serde_json::from_str(&line).expect("deserialize notification");
        assert_eq!(notification.method, "session/update");
        let params = notification.params.expect("params must be present");
        assert_eq!(params["sessionId"], json!("sess_test"));
        let update = &params["update"];
        assert_eq!(update["sessionUpdate"], json!("agent_message_chunk"));
        assert_eq!(
            update["content"],
            json!({ "type": "text", "text": "hello" })
        );
    }

    #[tokio::test]
    async fn stream_events_to_editor_emits_failure_status_without_normal_completion() {
        let (client, server) = duplex(4096);
        let mut transport = StdioTransport::from_io(empty(), server);
        let mut reader = BufReader::new(client);
        let cancel_token = CancelToken::new();
        let mut session = test_session("test-model", "none");
        let (sender, receiver) = mpsc::channel(8);

        sender
            .send(CognitiveEvent::Failure {
                message: "Error: provider returned 401".to_owned(),
            })
            .await
            .expect("send failure");
        sender
            .send(CognitiveEvent::Complete {
                stop_reason: StopReason::EndTurn,
                usage: None,
            })
            .await
            .expect("send normal completion after failure");
        drop(sender);

        let result = stream_events_to_editor(
            &mut transport,
            "sess_failure",
            &mut session,
            Path::new("."),
            receiver,
            &cancel_token,
        )
        .await;
        let result = result.expect("failure should still return a prompt result");

        assert_eq!(result.prompt_result.stop_reason, StopReason::EndTurn);
        assert_eq!(result.usage, None);

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read notification line");
        let notification: JsonRpcNotification =
            serde_json::from_str(&line).expect("deserialize notification");
        assert_eq!(notification.method, "session/update");
        let params = notification.params.expect("params must be present");
        assert_eq!(params["sessionId"], json!("sess_failure"));
        let update = &params["update"];
        assert_eq!(update["sessionUpdate"], json!("agent_message_chunk"));
        assert_eq!(
            update["content"],
            json!({ "type": "text", "text": "Error: provider returned 401" })
        );
    }

    #[tokio::test]
    async fn stream_events_to_editor_returns_cancelled_when_token_is_cancelled() {
        let (_client, server) = duplex(1024);
        let mut transport = StdioTransport::from_io(empty(), server);
        let cancel_token = CancelToken::new();
        let mut session = test_session("test-model", "none");
        let (_sender, receiver) = mpsc::channel(1);

        cancel_token.cancel();

        let result = stream_events_to_editor(
            &mut transport,
            "sess_cancel",
            &mut session,
            Path::new("."),
            receiver,
            &cancel_token,
        )
        .await
        .expect("cancelled prompt should still return a result");

        assert_eq!(result.prompt_result.stop_reason, StopReason::Cancelled);
    }

    #[tokio::test]
    async fn handle_session_prompt_rejects_busy_sessions() {
        let (_client, server) = duplex(1024);
        let mut transport = StdioTransport::from_io(empty(), server);
        let mut session = AcpSession::new(SessionNewParams {
            session_name: None,
            client_capabilities: None,
            model: None,
            provider: None,
            effort: None,
            mcp_servers: Vec::new(),
        });
        let session_id = session.session_id.clone();
        session.begin_prompt();

        let roko_config = RokoConfig::default();
        let error = handle_session_prompt(
            &mut transport,
            &mut session,
            SessionPromptParams {
                session_id: session_id.clone(),
                prompt: vec![ContentBlock::Text {
                    text: "busy".to_owned(),
                }],
                include_context: false,
            },
            Path::new("."),
            &roko_config,
        )
        .await
        .expect_err("busy session should be rejected");

        assert_eq!(
            error.rpc_error(),
            Some((
                SESSION_BUSY,
                format!("session '{session_id}' already has an active prompt")
            ))
        );
    }

    #[tokio::test]
    async fn cost_budget_exhaustion_rejects_before_provider_dispatch() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let mut transport = StdioTransport::from_io(empty(), tokio::io::sink());
        let mut session = test_session("model-a", "none");
        session.cost_budget_usd = Some(1.0);
        session.accumulated_cost_usd = 1.0;
        let session_id = session.session_id.clone();

        let error = handle_session_prompt(
            &mut transport,
            &mut session,
            SessionPromptParams {
                session_id,
                prompt: vec![ContentBlock::Text {
                    text: "this must never dispatch".to_owned(),
                }],
                include_context: false,
            },
            tmp.path(),
            &RokoConfig::default(),
        )
        .await
        .expect_err("exhausted budget must reject the turn");

        let (code, message) = error.rpc_error().expect("structured budget error");
        assert_eq!(code, SESSION_BUDGET_EXCEEDED);
        assert!(message.contains("budget exceeded"));
        assert!(session.conversation_history.is_empty());
        assert!(!session.is_busy());
        assert!(!tmp.path().join(".roko/learn/efficiency.jsonl").exists());
    }

    #[test]
    fn cost_budget_accumulates_exact_efficiency_event_cost() {
        let mut session = test_session("model-a", "none");
        session.cost_budget_usd = Some(1.0);
        let resolved = resolve_model(&RokoConfig::default(), "model-a");
        let event = acp_efficiency_event(
            &session.session_id,
            &resolved,
            Instant::now(),
            None,
            true,
            Some(0.375),
        );

        session.record_efficiency_cost(event.cost_usd);

        assert_eq!(event.cost_usd, 0.375);
        assert_eq!(session.accumulated_cost_usd, 0.375);
        assert_eq!(session.budget_status().budget_remaining_usd, Some(0.625));
    }

    #[tokio::test]
    async fn request_permission_returns_allow_for_pregranted_action() {
        let mut transport = StdioTransport::from_io(empty(), tokio::io::sink());
        let mut session = AcpSession::new(SessionNewParams {
            session_name: Some("perm-test".to_string()),
            client_capabilities: None,
            model: None,
            provider: None,
            effort: None,
            mcp_servers: Vec::new(),
        });
        let action = crate::types::PermissionAction::FileEdit;
        session.grant_always_allow(action.clone());

        let decision = request_permission(
            &mut transport,
            &mut session,
            Path::new("."),
            action,
            "Allow code agent to edit files?",
            "The code agent may read and modify files.",
        )
        .await;

        assert_eq!(decision, PermissionDecision::Allow);
    }

    #[tokio::test]
    async fn request_permission_persists_always_allow_decision() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path().to_path_buf();
        let mut session = AcpSession::new(SessionNewParams {
            session_name: Some("perm-test".to_string()),
            client_capabilities: None,
            model: None,
            provider: None,
            effort: None,
            mcp_servers: Vec::new(),
        });
        let action = crate::types::PermissionAction::FileEdit;

        let (client, server) = duplex(4096);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let ((), decision) = tokio::join!(
            reply_to_permission_request(
                client,
                json!({ "outcome": { "type": "selected", "optionId": "allow_always" } })
            ),
            request_permission(
                &mut transport,
                &mut session,
                &workdir,
                action.clone(),
                "Allow code agent to edit files?",
                "The code agent may read and modify files.",
            ),
        );

        assert_eq!(decision, PermissionDecision::AlwaysAllow);
        assert!(session.always_allowed.contains(&action));
        assert!(AcpSession::load_workspace_trust(&workdir).contains(&action));

        // The persisted session grant suppresses the next equivalent prompt.
        // A transport with no readable client is intentional: reaching it
        // would reject immediately and prove the pre-grant was not consulted.
        let mut disconnected = StdioTransport::from_io(empty(), tokio::io::sink());
        let repeated = request_permission(
            &mut disconnected,
            &mut session,
            &workdir,
            action,
            "Allow code agent to edit files?",
            "The code agent may read and modify files.",
        )
        .await;
        assert_eq!(repeated, PermissionDecision::Allow);
    }

    #[tokio::test]
    async fn stream_events_to_editor_routes_permission_request_to_editor() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path().to_path_buf();
        let mut session = test_session("test-model", "none");
        let session_id = session.session_id.clone();
        let cancel_token = session.cancel_token.clone();
        let (client, server) = duplex(4096);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let (event_sender, event_receiver) = mpsc::channel(4);
        let (decision_sender, decision_receiver) = tokio::sync::oneshot::channel();

        event_sender
            .send(CognitiveEvent::PermissionRequest {
                payload: PermissionRequestPayload {
                    action: PermissionAction::FileEdit,
                    title: "Write result.txt".to_owned(),
                    detail: "Allow this ACP turn to write the requested file?".to_owned(),
                },
                reply: PermissionReplyChannel::new(decision_sender),
            })
            .await
            .expect("queue permission event");
        event_sender
            .send(CognitiveEvent::Complete {
                stop_reason: StopReason::EndTurn,
                usage: None,
            })
            .await
            .expect("queue completion event");
        drop(event_sender);

        let ((), stream_result) = tokio::join!(
            reply_to_permission_request(
                client,
                json!({ "outcome": { "type": "selected", "optionId": "allow_once" } })
            ),
            stream_events_to_editor(
                &mut transport,
                &session_id,
                &mut session,
                &workdir,
                event_receiver,
                &cancel_token,
            ),
        );

        assert_eq!(
            decision_receiver.await.expect("receive editor decision"),
            PermissionDecision::Allow
        );
        assert_eq!(
            stream_result
                .expect("permission stream should complete")
                .prompt_result
                .stop_reason,
            StopReason::EndTurn
        );
    }

    #[tokio::test]
    async fn request_permission_defaults_to_reject_on_malformed_response() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path().to_path_buf();
        let mut session = AcpSession::new(SessionNewParams {
            session_name: Some("perm-test".to_string()),
            client_capabilities: None,
            model: None,
            provider: None,
            effort: None,
            mcp_servers: Vec::new(),
        });
        let action = crate::types::PermissionAction::FileEdit;

        let (client, server) = duplex(4096);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let ((), decision) = tokio::join!(
            reply_to_permission_request(client, json!({ "outcome": { "type": "cancelled" } })),
            request_permission(
                &mut transport,
                &mut session,
                &workdir,
                action.clone(),
                "Allow code agent to edit files?",
                "The code agent may read and modify files.",
            ),
        );

        assert_eq!(decision, PermissionDecision::Reject);
        assert!(!session.always_allowed.contains(&action));
        assert!(AcpSession::load_workspace_trust(&workdir).is_empty());
    }

    #[tokio::test]
    async fn append_acp_episode_records_single_dispatch_episode() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        let session = test_session("claude-sonnet-4-6", "none");
        let roko_config = RokoConfig::default();
        let stream_result = StreamResult {
            prompt_result: SessionPromptResult {
                stop_reason: StopReason::EndTurn,
            },
            assistant_text: "hello from acp".to_string(),
            usage: Some(UsageInfo {
                total_tokens: 12,
                input_tokens: 5,
                output_tokens: 7,
                thought_tokens: None,
                cached_read_tokens: Some(2),
                cached_write_tokens: Some(1),
            }),
        };
        let dispatch_started = Instant::now();
        let cascade_selection = AcpCascadeSelection {
            model_key: "claude-sonnet-4-6".to_owned(),
            stage: "confidence".to_owned(),
        };

        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        append_acp_episode(
            &roko_config,
            workdir,
            &session,
            &session.config_state.model,
            "trim a file",
            &session.config_state.workflow,
            false,
            dispatch_started,
            Some(&stream_result),
            None,
            None,
            None,
            Some(&cascade_selection),
        )
        .await;

        let episodes_path = workdir.join(".roko").join("episodes.jsonl");
        let episodes = EpisodeLogger::read_all(&episodes_path)
            .await
            .expect("read episodes");

        assert_eq!(episodes.len(), 1);
        let episode = &episodes[0];
        assert_eq!(episode.kind, "acp-dispatch");
        assert_eq!(episode.agent_template, "code");
        assert_eq!(episode.task_id, session.session_id);
        assert_eq!(episode.extra.get("entry_point"), Some(&json!("acp")));
        assert_eq!(
            episode.extra.get("session_id"),
            Some(&json!(episode.task_id.clone()))
        );
        assert_eq!(
            episode.extra.get("cascade_selected_model"),
            Some(&json!("claude-sonnet-4-6"))
        );
        assert_eq!(
            episode.extra.get("cascade_stage"),
            Some(&json!("confidence"))
        );
        assert!(!episode.extra.contains_key("routing_mode"));
        assert!(episode.usage.wall_ms > 0);
        assert_eq!(episode.tokens_used, 12);
        assert_eq!(episode.usage.input_tokens, 5);
        assert_eq!(episode.usage.output_tokens, 7);
        assert_eq!(episode.usage.cache_read_tokens, 2);
        assert_eq!(episode.usage.cache_write_tokens, 1);
        assert!(episode.usage.cost_usd > 0.0);
        assert!(episode.usage.cost_usd_without_cache >= episode.usage.cost_usd);
        assert!(episode.success);
        assert_eq!(episode.failure_reason, None);
    }

    #[tokio::test]
    async fn append_acp_episode_records_pipeline_kind() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        let session = test_session("claude-sonnet-4-6", "express");
        let roko_config = RokoConfig::default();
        let stream_result = StreamResult {
            prompt_result: SessionPromptResult {
                stop_reason: StopReason::EndTurn,
            },
            assistant_text: "pipeline complete".to_string(),
            usage: None,
        };
        let dispatch_started = Instant::now();

        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        append_acp_episode(
            &roko_config,
            workdir,
            &session,
            &session.config_state.model,
            "wire ACP logging",
            &session.config_state.workflow,
            true,
            dispatch_started,
            Some(&stream_result),
            None,
            None,
            None,
            None,
        )
        .await;

        let episodes_path = workdir.join(".roko").join("episodes.jsonl");
        let episodes = EpisodeLogger::read_all(&episodes_path)
            .await
            .expect("read episodes");

        assert_eq!(episodes.len(), 1);
        let episode = &episodes[0];
        assert_eq!(episode.kind, "acp-pipeline-express");
        assert_eq!(episode.extra.get("workflow"), Some(&json!("express")));
        assert!(episode.success);
    }

    #[test]
    fn acp_routing_context_maps_modes_to_roles() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();

        let plan = acp_routing_context("plan", "wire router feedback", "high", workdir);
        assert_eq!(plan.task_category, TaskCategory::Implementation);
        assert_eq!(plan.role, AgentRole::Strategist);
        assert_eq!(plan.thinking_level.as_deref(), Some("high"));

        let research =
            acp_routing_context("research", "find the source of truth", "medium", workdir);
        assert_eq!(research.task_category, TaskCategory::Research);
        assert_eq!(research.role, AgentRole::Researcher);

        let code = acp_routing_context("code", "edit file", "low", workdir);
        assert_eq!(code.task_category, TaskCategory::Implementation);
        assert_eq!(code.role, AgentRole::Implementer);
    }

    #[test]
    fn acp_routing_context_loads_canonical_daimon_affect() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let daimon_dir = tmp.path().join(".roko/daimon");
        std::fs::create_dir_all(&daimon_dir).expect("create daimon dir");
        std::fs::write(
            daimon_dir.join("affect.json"),
            serde_json::json!({
                "state": {
                    "confidence": 0.23,
                    "behavioral_state": "struggling"
                }
            })
            .to_string(),
        )
        .expect("write affect state");

        let context = acp_routing_context("code", "repair failure", "high", tmp.path());

        assert!((context.daimon_policy.affect_confidence - 0.23).abs() < f64::EPSILON);
        assert_eq!(
            context.daimon_policy.behavioral_state,
            roko_core::BehavioralState::Struggling
        );
    }

    #[test]
    fn acp_dispatch_reward_distinguishes_success_and_failure() {
        assert_eq!(compute_acp_reward(false, 200, Some(120)), 0.0);
        assert!(compute_acp_reward(true, 1_000, Some(1_000)) > 0.9);
        assert!(compute_acp_reward(true, 20_000, None) >= 0.8);
    }

    #[test]
    fn cascade_router_model_slugs_falls_back_when_config_is_empty() {
        let config = RokoConfig::default();
        let slugs = cascade_router_model_slugs(&config, "fallback-slug");
        assert_eq!(slugs, vec!["fallback-slug".to_string()]);
    }

    #[test]
    fn cascade_router_model_slugs_are_deterministically_sorted() {
        let mut config = RokoConfig::default();
        config
            .models
            .insert("z-model".to_owned(), ModelProfile::default());
        config
            .models
            .insert("a-model".to_owned(), ModelProfile::default());

        assert_eq!(
            cascade_router_model_slugs(&config, "unused"),
            vec!["a-model".to_owned(), "z-model".to_owned()]
        );
    }

    #[test]
    fn resolved_acp_dispatch_uses_the_cascade_config_key() {
        let mut config = RokoConfig::default();
        config.models.insert(
            "requested".to_owned(),
            ModelProfile {
                slug: "wire-requested".to_owned(),
                ..ModelProfile::default()
            },
        );
        config.models.insert(
            "selected".to_owned(),
            ModelProfile {
                slug: "wire-selected".to_owned(),
                ..ModelProfile::default()
            },
        );

        let selection = AcpCascadeSelection {
            model_key: "selected".to_owned(),
            stage: "confidence".to_owned(),
        };
        let (resolved, dispatch_key, retained) =
            resolve_acp_dispatch_model(&config, "requested", Some(selection.clone()));

        assert_eq!(dispatch_key, "selected");
        assert_eq!(resolved.model_key, "selected");
        assert_eq!(resolved.slug, "wire-selected");
        assert_eq!(retained, Some(selection));
    }

    #[test]
    fn resolved_acp_dispatch_rejects_an_unconfigured_cascade_key() {
        let mut config = RokoConfig::default();
        config.models.insert(
            "requested".to_owned(),
            ModelProfile {
                slug: "wire-requested".to_owned(),
                ..ModelProfile::default()
            },
        );

        let (resolved, dispatch_key, retained) = resolve_acp_dispatch_model(
            &config,
            "requested",
            Some(AcpCascadeSelection {
                model_key: "not-configured".to_owned(),
                stage: "ucb".to_owned(),
            }),
        );

        assert_eq!(dispatch_key, "requested");
        assert_eq!(resolved.model_key, "requested");
        assert!(retained.is_none());
    }

    #[test]
    fn calculate_cost_for_model_slug_handles_known_and_unknown_models() {
        let known = calculate_cost_for_model_slug("claude-sonnet-4-6", 1_000, 500, 250)
            .expect("known pricing should exist");
        assert!(known > 0.0);

        assert_eq!(
            calculate_cost_for_model_slug("definitely-not-a-real-model", 1_000, 500, 250),
            None
        );
    }

    #[test]
    fn assistant_history_truncation_caps_bytes_and_preserves_boundaries() {
        let text = "é".repeat(6_000);
        let truncated = truncate_assistant_history(&text);
        let suffix = "...[truncated]";
        let prefix_len = truncated.len() - suffix.len();

        assert!(truncated.ends_with(suffix));
        assert!(truncated.len() <= MAX_HISTORY_ASSISTANT_BYTES + suffix.len());
        assert!(truncated.len() < text.len());
        assert!(truncated[..prefix_len].chars().all(|c| c == 'é'));
    }

    #[test]
    fn tool_name_mapping() {
        assert_eq!(tool_name_to_kind("Edit"), ToolCallKind::Edit);
        assert_eq!(tool_name_to_kind("Write"), ToolCallKind::Create);
        assert_eq!(tool_name_to_kind("Bash"), ToolCallKind::Terminal);
        assert_eq!(tool_name_to_kind("Read"), ToolCallKind::Other);
    }

    #[test]
    fn extract_at_mentions_supports_embedded_mentions() {
        let mentions = extract_at_mentions("fix @src/main.rs and @branch-diff, not foo@bar.com");
        assert_eq!(mentions, vec!["src/main.rs", "branch-diff"]);
    }

    #[tokio::test]
    async fn resolve_context_items_resolves_resource_and_path_mentions() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        let file_path = workdir.join("src/main.rs");
        std::fs::create_dir_all(file_path.parent().expect("parent directory"))
            .expect("create dirs");
        std::fs::write(&file_path, "fn main() {}\n").expect("write file");

        let prompt = vec![
            ContentBlock::Resource {
                resource: crate::types::ResourceRef::File {
                    uri: format!("file://{}", file_path.display()),
                },
            },
            ContentBlock::Text {
                text: "check @src/main.rs".to_owned(),
            },
        ];

        let context = resolve_context_items(&prompt, workdir).await;
        assert!(context.contains("<file path=\"src/main.rs\">"));
        assert!(context.contains("--- src/main.rs ---"));
        assert!(context.contains("fn main() {}"));
    }

    #[test]
    fn truncate_with_limit_is_char_safe() {
        let text = "é".repeat(20_000);
        let truncated = truncate_with_limit(&text, 32_768, "... [truncated]");
        let prefix_len = truncated.len() - "... [truncated]".len();

        assert!(truncated.ends_with("... [truncated]"));
        assert!(truncated.len() < text.len());
        assert!(truncated[..prefix_len].chars().all(|c| c == 'é'));
    }

    #[tokio::test]
    async fn build_provenance_includes_all_source_types() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        std::fs::create_dir_all(workdir.join(".roko").join("learn")).expect("create learn dir");

        let playbook = Playbook {
            id: "dispatch-chain".into(),
            name: "dispatch-chain".into(),
            goal: "Reuse the proven dispatch path for similar tasks".into(),
            when_pattern: Some("dispatch path".into()),
            steps: Vec::new(),
            success_count: 3,
            failure_count: 1,
            created_at_ms: 0,
            last_used_ms: Some(0),
        };

        let mut episode = Episode::new("agent-1", playbook.id.as_str()).succeeded();
        episode.kind = "agent_turn".into();
        episode.gate_verdicts = vec![
            roko_learn::episode_logger::EpisodeGateVerdict::new("compile", true),
            roko_learn::episode_logger::EpisodeGateVerdict::new("test", true),
        ];
        let logger = EpisodeLogger::new(workdir.join(".roko").join("episodes.jsonl"));
        logger.append(&episode).await.expect("append episode");

        let advice = roko_dreams::DreamRoutingAdvice {
            generated_at: chrono::Utc::now(),
            source_dream_report: "dream-report".into(),
            recommendations: Vec::new(),
            pattern_summaries: vec![roko_dreams::PatternSummary {
                description: "dispatch decisions should show the evidence chain".into(),
                applies_to: vec!["dispatch".into()],
                guidance: "surface the chain before strategist work starts".into(),
                confidence: 0.91,
                signature: 42,
            }],
        };
        std::fs::write(
            workdir
                .join(".roko")
                .join("learn")
                .join("dream-routing-advice.json"),
            serde_json::to_string(&advice).expect("serialize dream advice"),
        )
        .expect("write dream advice");

        let knowledge_hits = vec![KnowledgeQueryHit {
            entry: roko_neuro::KnowledgeEntry {
                id: "knowledge-1".into(),
                kind: KnowledgeKind::StrategyFragment,
                content: "Prefer the proven dispatcher path".into(),
                confidence: 0.9,
                tier: KnowledgeTier::Persistent,
                source_episodes: vec![playbook.id.clone()],
                tags: vec!["dispatch".into()],
                ..Default::default()
            },
            total_score: 0.85,
            breakdown: roko_neuro::KnowledgeQueryBreakdown {
                keyword_score: 1.0,
                effective_confidence: 0.9,
                recency_factor: 1.0,
                emotional_boost: 1.0,
                balance_freshness_boost: 0.0,
                hdc_similarity: None,
            },
        }];

        let chain = build_provenance(
            &knowledge_hits,
            &[playbook],
            "dispatch the request",
            workdir,
        )
        .await
        .expect("meaningful provenance");

        assert_eq!(chain.sources.len(), 4);
        assert!(matches!(
            chain.sources[0],
            ProvenanceSource::Playbook { .. }
        ));
        assert!(matches!(chain.sources[1], ProvenanceSource::Episode { .. }));
        assert!(matches!(
            chain.sources[2],
            ProvenanceSource::Knowledge { .. }
        ));
        assert!(matches!(
            chain.sources[3],
            ProvenanceSource::DreamPattern { .. }
        ));
        assert!(chain.confidence > 0.0);

        let card = render_provenance_card(&chain);
        assert!(card.contains("4 sources"));
        assert!(card.contains("Playbook `dispatch-chain`"));
        assert!(card.contains("Episode `dispatch-chain`"));
        assert!(card.contains("Knowledge [strategy_fragment/persistent]"));
        assert!(card.contains("Dream pattern"));
    }

    #[tokio::test]
    async fn build_provenance_suppresses_trivial_knowledge_only_chains() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();

        let knowledge_hits = vec![KnowledgeQueryHit {
            entry: roko_neuro::KnowledgeEntry {
                id: "knowledge-2".into(),
                kind: KnowledgeKind::Insight,
                content: "A lone idea without supporting history".into(),
                confidence: 0.5,
                tier: KnowledgeTier::Working,
                ..Default::default()
            },
            total_score: 0.4,
            breakdown: roko_neuro::KnowledgeQueryBreakdown {
                keyword_score: 1.0,
                effective_confidence: 0.5,
                recency_factor: 1.0,
                emotional_boost: 1.0,
                balance_freshness_boost: 0.0,
                hdc_similarity: None,
            },
        }];

        let chain = build_provenance(&knowledge_hits, &[], "small", workdir).await;
        assert!(chain.is_none());
    }

    #[tokio::test]
    async fn permission_request_event_carries_reply_channel() {
        // Create a oneshot channel for the permission decision.
        let (tx, rx) = tokio::sync::oneshot::channel::<PermissionDecision>();

        // Build the PermissionRequest event.
        let payload = PermissionRequestPayload {
            action: PermissionAction::FileEdit,
            title: "Edit src/main.rs".into(),
            detail: "Replace println with tracing macro".into(),
        };
        let reply = PermissionReplyChannel::new(tx);
        let event = CognitiveEvent::PermissionRequest {
            payload: payload.clone(),
            reply: reply.clone(),
        };

        // Verify the event carries the expected payload fields.
        match &event {
            CognitiveEvent::PermissionRequest {
                payload: p,
                reply: r,
            } => {
                assert_eq!(p.action, PermissionAction::FileEdit);
                assert_eq!(p.title, "Edit src/main.rs");
                assert_eq!(p.detail, "Replace println with tracing macro");
                assert!(!r.is_consumed(), "reply channel should not be consumed yet");
            }
            _ => panic!("expected PermissionRequest variant"),
        }

        // Send the event through an mpsc channel (simulating the real flow).
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<CognitiveEvent>(4);
        event_tx.send(event).await.expect("send event");
        drop(event_tx);

        let received = event_rx.recv().await.expect("receive event");
        match received {
            CognitiveEvent::PermissionRequest { reply, .. } => {
                // Parent loop replies with Allow.
                assert!(reply.reply(PermissionDecision::Allow));
                // Second reply should fail (already consumed).
                assert!(!reply.reply(PermissionDecision::Reject));
                assert!(reply.is_consumed());
            }
            _ => panic!("expected PermissionRequest variant"),
        }

        // The tool loop receives the decision.
        let decision = rx.await.expect("receive decision");
        assert_eq!(decision, PermissionDecision::Allow);
    }

    #[tokio::test]
    async fn permission_reply_channel_dropped_without_reply_gives_recv_error() {
        let (tx, rx) = tokio::sync::oneshot::channel::<PermissionDecision>();
        let reply = PermissionReplyChannel::new(tx);

        // Drop without replying — simulates parent loop crash / timeout.
        drop(reply);

        // The receiver should get an error (fail-closed).
        assert!(
            rx.await.is_err(),
            "dropped reply channel must produce RecvError"
        );
    }

    // ── AcpBuiltinToolHandler permission enforcement ─────────────────────────

    #[tokio::test]
    async fn acp_builtin_tool_handler_respects_denied_tools() {
        let (tx, _rx) = mpsc::channel(16);
        let handler = AcpBuiltinToolHandler {
            tool_name: "bash".into(),
            session_id: "test-session".into(),
            workdir: std::env::temp_dir(),
            event_sender: tx,
        };
        let call = ToolCall {
            id: "t1".into(),
            name: "bash".into(),
            arguments: serde_json::json!({"command": "echo hi"}),
            request_ts_ms: 0,
        };
        let mut ctx = ToolContext::testing(std::env::temp_dir());
        ctx.denied_tools = Some(vec!["bash".into()]);
        let result = handler.execute(call, &ctx).await;
        assert!(
            matches!(result, ToolResult::Err(_)),
            "denied tool must return ToolResult::Err"
        );
    }

    #[tokio::test]
    async fn acp_builtin_tool_handler_respects_allowed_tools() {
        let (tx, _rx) = mpsc::channel(16);
        let handler = AcpBuiltinToolHandler {
            tool_name: "bash".into(),
            session_id: "test-session".into(),
            workdir: std::env::temp_dir(),
            event_sender: tx,
        };
        let call = ToolCall {
            id: "t1".into(),
            name: "bash".into(),
            arguments: serde_json::json!({"command": "echo hi"}),
            request_ts_ms: 0,
        };
        let mut ctx = ToolContext::testing(std::env::temp_dir());
        // bash is not in the allowed set — only read tools are.
        ctx.allowed_tools = Some(vec!["read_file".into(), "glob".into()]);
        let result = handler.execute(call, &ctx).await;
        assert!(
            matches!(result, ToolResult::Err(_)),
            "tool not in allowed set must return ToolResult::Err"
        );
    }

    #[tokio::test]
    async fn permission_prompt_precedes_write() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let target = tmp.path().join("permission-gated.txt");
        let mut session = test_session("test-model", "none");
        let session_id = session.session_id.clone();
        let cancel_token = session.cancel_token.clone();
        let (client, server) = duplex(4096);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let (event_sender, event_receiver) = mpsc::channel(16);

        let handler = AcpBuiltinToolHandler {
            tool_name: "write_file".into(),
            session_id: session_id.clone(),
            workdir: tmp.path().to_path_buf(),
            event_sender: event_sender.clone(),
        };
        let context = ToolContext::testing(tmp.path());
        let handler_task = tokio::spawn(async move {
            handler
                .execute(
                    ToolCall {
                        id: "wire-reject-write".into(),
                        name: "write_file".into(),
                        arguments: json!({
                            "path": "permission-gated.txt",
                            "content": "must not be written"
                        }),
                        request_ts_ms: 0,
                    },
                    &context,
                )
                .await
        });
        let completion_sender = event_sender.clone();
        let (editor_release_sender, editor_release_receiver) = tokio::sync::oneshot::channel();
        let completion_task = tokio::spawn(async move {
            let result = handler_task.await.expect("join permission-gated handler");
            completion_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await
                .expect("queue completion after rejected tool");
            let _ = editor_release_sender.send(());
            result
        });
        drop(event_sender);

        let editor = async {
            let mut reader = BufReader::new(client);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .expect("read outbound permission request");
            let request: serde_json::Value =
                serde_json::from_str(&line).expect("parse permission request");
            assert_eq!(request["method"], json!("session/request_permission"));
            assert!(!target.exists(), "permission must precede the write");

            let response = json!({
                "jsonrpc": "2.0",
                "id": request["id"].clone(),
                "result": {
                    "outcome": { "type": "selected", "optionId": "reject_once" }
                }
            });
            let mut client = reader.into_inner();
            client
                .write_all(
                    serde_json::to_string(&response)
                        .expect("serialize response")
                        .as_bytes(),
                )
                .await
                .expect("write permission rejection");
            client.write_all(b"\n").await.expect("write newline");
            client.flush().await.expect("flush permission rejection");
            let _ = editor_release_receiver.await;
        };

        let ((), stream_result) = tokio::join!(
            editor,
            stream_events_to_editor(
                &mut transport,
                &session_id,
                &mut session,
                tmp.path(),
                event_receiver,
                &cancel_token,
            )
        );
        let tool_result = completion_task.await.expect("join completion task");

        assert!(matches!(
            tool_result,
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
        assert!(
            !target.exists(),
            "wire-level rejection must block the write"
        );
        assert_eq!(
            stream_result
                .expect("permission stream should complete")
                .prompt_result
                .stop_reason,
            StopReason::EndTurn
        );
    }

    #[tokio::test]
    async fn permission_wait_cancellation_is_bounded() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let target = tmp.path().join("cancelled-permission.txt");
        let mut session = test_session("test-model", "none");
        let session_id = session.session_id.clone();
        let cancel_token = session.cancel_token.clone();
        let (client, server) = duplex(4096);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let (event_sender, event_receiver) = mpsc::channel(16);

        let handler = AcpBuiltinToolHandler {
            tool_name: "write_file".into(),
            session_id: session_id.clone(),
            workdir: tmp.path().to_path_buf(),
            event_sender: event_sender.clone(),
        };
        let context = ToolContext::testing(tmp.path());
        let handler_task = tokio::spawn(async move {
            handler
                .execute(
                    ToolCall {
                        id: "cancelled-write".into(),
                        name: "write_file".into(),
                        arguments: json!({
                            "path": "cancelled-permission.txt",
                            "content": "must not be written"
                        }),
                        request_ts_ms: 0,
                    },
                    &context,
                )
                .await
        });
        drop(event_sender);

        let editor_session_id = session_id.clone();
        let editor = async move {
            let mut reader = BufReader::new(client);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .expect("read outbound permission request");
            let request: serde_json::Value =
                serde_json::from_str(&line).expect("parse permission request");
            assert_eq!(request["method"], json!("session/request_permission"));

            let cancel = json!({
                "jsonrpc": "2.0",
                "method": "session/cancel",
                "params": { "sessionId": editor_session_id }
            });
            let mut client = reader.into_inner();
            client
                .write_all(
                    serde_json::to_string(&cancel)
                        .expect("serialize cancel")
                        .as_bytes(),
                )
                .await
                .expect("write session cancel");
            client.write_all(b"\n").await.expect("write newline");
            client.flush().await.expect("flush session cancel");
        };

        let joined = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(
                editor,
                stream_events_to_editor(
                    &mut transport,
                    &session_id,
                    &mut session,
                    tmp.path(),
                    event_receiver,
                    &cancel_token,
                )
            )
        })
        .await
        .expect("permission cancellation must not wait for the editor timeout");
        let ((), stream_result) = joined;
        let tool_result = handler_task.await.expect("join cancelled handler");

        assert!(matches!(
            tool_result,
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
        assert_eq!(
            stream_result
                .expect("cancelled permission stream should return a prompt result")
                .prompt_result
                .stop_reason,
            StopReason::Cancelled
        );
        assert!(cancel_token.is_cancelled());
        assert!(
            !target.exists(),
            "cancelled permission must block the write"
        );
    }

    #[tokio::test]
    async fn permission_wait_abandons_when_requester_times_out() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let mut session = test_session("test-model", "none");
        let session_id = session.session_id.clone();
        let cancel_token = session.cancel_token.clone();
        let (client, server) = duplex(4096);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let (event_sender, event_receiver) = mpsc::channel(4);
        let (decision_sender, decision_receiver) = tokio::sync::oneshot::channel();
        let (request_seen_sender, request_seen_receiver) = tokio::sync::oneshot::channel();

        event_sender
            .send(CognitiveEvent::PermissionRequest {
                payload: PermissionRequestPayload {
                    action: PermissionAction::FileCreate,
                    title: "Write timed-out.txt".to_owned(),
                    detail: "Allow this ACP turn to write the requested file?".to_owned(),
                },
                reply: PermissionReplyChannel::new(decision_sender),
            })
            .await
            .expect("queue permission event");
        event_sender
            .send(CognitiveEvent::Complete {
                stop_reason: StopReason::EndTurn,
                usage: None,
            })
            .await
            .expect("queue completion event");
        drop(event_sender);

        let editor = async move {
            let mut reader = BufReader::new(client);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .expect("read outbound permission request");
            let request: serde_json::Value =
                serde_json::from_str(&line).expect("parse permission request");
            assert_eq!(request["method"], json!("session/request_permission"));
            request_seen_sender
                .send(())
                .expect("signal request observed");
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        let requester_timeout = async move {
            request_seen_receiver
                .await
                .expect("request should be observed");
            drop(decision_receiver);
        };

        let ((), (), stream_result) = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(
                editor,
                requester_timeout,
                stream_events_to_editor(
                    &mut transport,
                    &session_id,
                    &mut session,
                    tmp.path(),
                    event_receiver,
                    &cancel_token,
                )
            )
        })
        .await
        .expect("requester timeout must abandon the longer editor wait");

        assert_eq!(
            stream_result
                .expect("stream should continue after abandoned permission request")
                .prompt_result
                .stop_reason,
            StopReason::EndTurn
        );
    }

    #[tokio::test]
    async fn acp_builtin_permission_decisions_gate_write() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let target = tmp.path().join("permission-gated.txt");
        let call = || ToolCall {
            id: "permission-write".into(),
            name: "write_file".into(),
            arguments: json!({
                "path": "permission-gated.txt",
                "content": "written only after approval"
            }),
            request_ts_ms: 0,
        };

        // Reject: the request is observable before any side effect and the
        // target remains absent after the handler returns.
        let (reject_sender, mut reject_events) = mpsc::channel(16);
        let reject_handler = AcpBuiltinToolHandler {
            tool_name: "write_file".into(),
            session_id: "permission-reject".into(),
            workdir: tmp.path().to_path_buf(),
            event_sender: reject_sender,
        };
        let reject_context = ToolContext::testing(tmp.path());
        let reject_task =
            tokio::spawn(async move { reject_handler.execute(call(), &reject_context).await });
        let reject_reply = match reject_events.recv().await.expect("permission request") {
            CognitiveEvent::PermissionRequest { payload, reply } => {
                assert_eq!(payload.action, PermissionAction::FileCreate);
                assert!(!target.exists(), "permission must precede the write");
                reply
            }
            other => panic!("expected permission request, got {other:?}"),
        };
        assert!(reject_reply.reply(PermissionDecision::Reject));
        let rejected = reject_task.await.expect("join rejected write");
        assert!(matches!(
            rejected,
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
        assert!(!target.exists(), "rejected write must have no side effect");

        // Allow: execution resumes only after the positive decision.
        let (allow_sender, mut allow_events) = mpsc::channel(16);
        let allow_handler = AcpBuiltinToolHandler {
            tool_name: "write_file".into(),
            session_id: "permission-allow".into(),
            workdir: tmp.path().to_path_buf(),
            event_sender: allow_sender,
        };
        let allow_context = ToolContext::testing(tmp.path());
        let allow_task =
            tokio::spawn(async move { allow_handler.execute(call(), &allow_context).await });
        let allow_reply = match allow_events.recv().await.expect("permission request") {
            CognitiveEvent::PermissionRequest { reply, .. } => {
                assert!(!target.exists(), "permission must precede the write");
                reply
            }
            other => panic!("expected permission request, got {other:?}"),
        };
        assert!(allow_reply.reply(PermissionDecision::Allow));
        let allowed = allow_task.await.expect("join allowed write");
        assert!(allowed.is_ok());
        assert_eq!(
            tokio::fs::read_to_string(&target)
                .await
                .expect("allowed write should create target"),
            "written only after approval"
        );

        // A dropped parent reply is a prompt denial, not a hang or fail-open.
        tokio::fs::remove_file(&target)
            .await
            .expect("remove target before dropped-reply case");
        let (drop_sender, mut drop_events) = mpsc::channel(16);
        let drop_handler = AcpBuiltinToolHandler {
            tool_name: "write_file".into(),
            session_id: "permission-dropped".into(),
            workdir: tmp.path().to_path_buf(),
            event_sender: drop_sender,
        };
        let drop_context = ToolContext::testing(tmp.path());
        let drop_task =
            tokio::spawn(async move { drop_handler.execute(call(), &drop_context).await });
        match drop_events.recv().await.expect("permission request") {
            CognitiveEvent::PermissionRequest { reply, .. } => drop(reply),
            other => panic!("expected permission request, got {other:?}"),
        }
        let dropped = tokio::time::timeout(Duration::from_secs(1), drop_task)
            .await
            .expect("dropped reply must not hang")
            .expect("join dropped-reply write");
        assert!(matches!(
            dropped,
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
        assert!(!target.exists(), "dropped reply must not execute the write");
    }

    #[tokio::test]
    async fn slash_command_streaming_forwards_stdout_and_stderr_before_eof() {
        let (mut stdout_writer, stdout_reader) = duplex(1024);
        let (mut stderr_writer, stderr_reader) = duplex(1024);
        let (event_sender, mut event_receiver) = mpsc::channel(8);
        let cancel = CancelToken::new();
        let cancel_for_task = cancel.clone();
        let stream_task = tokio::spawn(async move {
            forward_slash_command_streams(
                "stream-test",
                stdout_reader,
                stderr_reader,
                &cancel_for_task,
                &event_sender,
            )
            .await
        });

        stdout_writer
            .write_all(b"first line\n")
            .await
            .expect("write stdout");
        stdout_writer.flush().await.expect("flush stdout");
        let first = tokio::time::timeout(Duration::from_secs(1), event_receiver.recv())
            .await
            .expect("stdout delivered before eof")
            .expect("stdout event");
        assert!(matches!(first, CognitiveEvent::TokenChunk(text) if text == "first line\n"));

        stderr_writer
            .write_all(b"warning\n")
            .await
            .expect("write stderr");
        stderr_writer.flush().await.expect("flush stderr");
        let second = tokio::time::timeout(Duration::from_secs(1), event_receiver.recv())
            .await
            .expect("stderr delivered before eof")
            .expect("stderr event");
        assert!(matches!(
            second,
            CognitiveEvent::TokenChunk(text) if text == "\x1b[2mwarning\x1b[0m\n"
        ));

        drop(stdout_writer);
        drop(stderr_writer);
        assert_eq!(
            stream_task.await.expect("stream task"),
            SlashCommandStreamOutcome::Completed { had_output: true }
        );
    }

    #[tokio::test]
    async fn slash_command_streaming_preserves_unknown_progress_and_correlates_tasks() {
        let (mut stdout_writer, stdout_reader) = duplex(4096);
        let (stderr_writer, stderr_reader) = duplex(64);
        drop(stderr_writer);
        let (event_sender, mut event_receiver) = mpsc::channel(16);
        let cancel = CancelToken::new();
        let cancel_for_task = cancel.clone();
        let stream_task = tokio::spawn(async move {
            forward_slash_command_streams(
                "progress-test",
                stdout_reader,
                stderr_reader,
                &cancel_for_task,
                &event_sender,
            )
            .await
        });

        stdout_writer
            .write_all(
                b"ROKO_PROGRESS: {not-json}\nROKO_PROGRESS: {\"type\":\"future_event\"}\nROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"alpha\"}\nROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"B\",\"title\":\"beta\"}\nROKO_PROGRESS: {\"type\":\"task_completed\",\"task_id\":\"A\",\"completed\":1,\"total\":2}\n",
            )
            .await
            .expect("write progress lines");
        drop(stdout_writer);

        assert_eq!(
            stream_task.await.expect("stream task"),
            SlashCommandStreamOutcome::Completed { had_output: true }
        );

        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }
        assert!(matches!(
            &events[0],
            CognitiveEvent::TokenChunk(text) if text == "ROKO_PROGRESS: {not-json}\n"
        ));
        assert!(matches!(
            &events[1],
            CognitiveEvent::TokenChunk(text) if text.contains("future_event")
        ));
        assert!(matches!(
            &events[2],
            CognitiveEvent::ToolCallStart { tool_call_id, .. } if tool_call_id == "progress-A-1"
        ));
        assert!(matches!(
            &events[3],
            CognitiveEvent::ToolCallStart { tool_call_id, .. } if tool_call_id == "progress-B-2"
        ));
        assert!(matches!(
            &events[4],
            CognitiveEvent::ToolCallComplete { tool_call_id, .. } if tool_call_id == "progress-A-1"
        ));
        assert!(matches!(
            &events[5],
            CognitiveEvent::ToolCallComplete {
                tool_call_id,
                status: ToolCallStatus::Failed,
                ..
            } if tool_call_id == "progress-B-2"
        ));
    }

    #[tokio::test]
    async fn slash_command_streaming_failure_closes_attempt_before_retry() {
        let (mut stdout_writer, stdout_reader) = duplex(4096);
        let (stderr_writer, stderr_reader) = duplex(64);
        drop(stderr_writer);
        let (event_sender, mut event_receiver) = mpsc::channel(16);
        let cancel = CancelToken::new();
        let cancel_for_task = cancel.clone();
        let stream_task = tokio::spawn(async move {
            forward_slash_command_streams(
                "retry-test",
                stdout_reader,
                stderr_reader,
                &cancel_for_task,
                &event_sender,
            )
            .await
        });

        stdout_writer
            .write_all(
                b"ROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"first\"}\nROKO_PROGRESS: {\"type\":\"task_failed\",\"task_id\":\"A\",\"error\":\"retry me\"}\nROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"retry\"}\nROKO_PROGRESS: {\"type\":\"task_completed\",\"task_id\":\"A\",\"completed\":1,\"total\":1}\n",
            )
            .await
            .expect("write progress lines");
        drop(stdout_writer);

        assert_eq!(
            stream_task.await.expect("stream task"),
            SlashCommandStreamOutcome::Completed { had_output: true }
        );
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }
        assert!(matches!(
            &events[0],
            CognitiveEvent::ToolCallStart { tool_call_id, .. }
                if tool_call_id == "progress-A-1"
        ));
        assert!(matches!(
            &events[1],
            CognitiveEvent::ToolCallComplete {
                tool_call_id,
                status: ToolCallStatus::Failed,
                content,
            } if tool_call_id == "progress-A-1"
                && matches!(content.as_slice(), [ContentBlock::Text { text }] if text == "retry me")
        ));
        assert!(matches!(
            &events[2],
            CognitiveEvent::ToolCallStart { tool_call_id, .. }
                if tool_call_id == "progress-A-2"
        ));
        assert!(matches!(
            &events[3],
            CognitiveEvent::ToolCallComplete {
                tool_call_id,
                status: ToolCallStatus::Completed,
                ..
            } if tool_call_id == "progress-A-2"
        ));
    }

    #[tokio::test]
    async fn slash_command_empty_output_emits_one_fallback_then_completion() {
        let (event_sender, mut event_receiver) = mpsc::channel(4);
        finish_slash_command_stream("plan-run", false, &event_sender).await;

        assert!(matches!(
            event_receiver.recv().await,
            Some(CognitiveEvent::TokenChunk(text)) if text == "/plan-run completed (no output)"
        ));
        assert!(matches!(
            event_receiver.recv().await,
            Some(CognitiveEvent::Complete { .. })
        ));
        assert!(event_receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn slash_command_streaming_cancellation_does_not_emit_completion() {
        let (mut stdout_writer, stdout_reader) = duplex(512);
        let (_stderr_writer, stderr_reader) = duplex(64);
        let (event_sender, mut event_receiver) = mpsc::channel(8);
        let cancel = CancelToken::new();
        let cancel_for_task = cancel.clone();
        let stream_task = tokio::spawn(async move {
            forward_slash_command_streams(
                "cancel-test",
                stdout_reader,
                stderr_reader,
                &cancel_for_task,
                &event_sender,
            )
            .await
        });

        stdout_writer
            .write_all(
                b"ROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"alpha\"}\n",
            )
            .await
            .expect("write task start");
        assert!(matches!(
            event_receiver.recv().await,
            Some(CognitiveEvent::ToolCallStart { tool_call_id, .. })
                if tool_call_id == "progress-A-1"
        ));
        cancel.cancel();
        assert_eq!(
            stream_task.await.expect("stream task"),
            SlashCommandStreamOutcome::Cancelled
        );
        assert!(matches!(
            event_receiver.recv().await,
            Some(CognitiveEvent::ToolCallComplete {
                tool_call_id,
                status: ToolCallStatus::Failed,
                content,
            }) if tool_call_id == "progress-A-1"
                && matches!(content.as_slice(), [ContentBlock::Text { text }] if text == "cancelled")
        ));
        assert!(!matches!(
            event_receiver.try_recv(),
            Ok(CognitiveEvent::Complete { .. })
        ));
    }

    // ── T6: cascade_select_model tests ───────────────────────────────────────
    //
    // Tests that mutate the `ROKO_ACP_CASCADE_SELECT` env var share a module-level
    // Mutex so they run serially and cannot interfere with each other.

    static CASCADE_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    fn cascade_env_lock() -> &'static std::sync::Mutex<()> {
        CASCADE_ENV_LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn test_provider_runtime(
        config: &RokoConfig,
    ) -> (Arc<ProviderHealthRegistry>, Arc<ProviderRateLimiter>) {
        let health = Arc::new(ProviderHealthRegistry::new());
        let health_checker: Arc<dyn roko_agent::rate_limit::ProviderHealthChecker> =
            Arc::clone(&health) as Arc<_>;
        let providers = config.effective_providers();
        let limiter = Arc::new(
            ProviderRateLimiter::from_provider_configs(
                roko_core::defaults::DEFAULT_PROVIDER_RPM,
                providers.iter(),
            )
            .with_health_registry(health_checker),
        );
        (health, limiter)
    }

    /// cascade_select_model returns None when the ROKO_ACP_CASCADE_SELECT env
    /// var is absent, regardless of whether a router file exists.
    #[test]
    fn cascade_select_model_returns_none_without_env_var() {
        let _guard = cascade_env_lock().lock().expect("acquire env lock");
        // SAFETY: serialized by cascade_env_lock; no other test holds the lock
        // and mutates ROKO_ACP_CASCADE_SELECT concurrently.
        unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        let config = RokoConfig::default();
        let (health, limiter) = test_provider_runtime(&config);
        let result = cascade_select_model(AcpCascadeRequest {
            workdir,
            roko_config: &config,
            mode: "code",
            prompt: "fix a bug",
            effort: "medium",
            resolved_slug: "claude-sonnet-4-6",
            model_selection_explicit: false,
            provider_health: &health,
            rate_limiter: &limiter,
        });
        assert!(
            result.is_none(),
            "should return None when env var is not set"
        );
    }

    /// cascade_select_model returns None when the env var is set but the router
    /// state file does not yet exist (cold start).
    #[test]
    fn cascade_select_model_returns_none_without_router_file() {
        let _guard = cascade_env_lock().lock().expect("acquire env lock");
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        // The router path will not exist in the fresh tmpdir.
        let router_path = workdir
            .join(".roko")
            .join("learn")
            .join("cascade-router.json");
        assert!(!router_path.exists(), "router file should not exist yet");

        // SAFETY: serialized by cascade_env_lock; no other test holds the lock
        // and mutates ROKO_ACP_CASCADE_SELECT concurrently.
        unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "1") };
        let config = RokoConfig::default();
        let (health, limiter) = test_provider_runtime(&config);
        let result = cascade_select_model(AcpCascadeRequest {
            workdir,
            roko_config: &config,
            mode: "code",
            prompt: "fix a bug",
            effort: "medium",
            resolved_slug: "claude-sonnet-4-6",
            model_selection_explicit: false,
            provider_health: &health,
            rate_limiter: &limiter,
        });
        unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

        assert!(
            result.is_none(),
            "should return None when router file is absent"
        );
    }

    #[test]
    fn cascade_select_model_requires_exact_opt_in_value() {
        let _guard = cascade_env_lock().lock().expect("acquire env lock");
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let router_dir = tmp.path().join(".roko").join("learn");
        std::fs::create_dir_all(&router_dir).expect("create router dir");
        CascadeRouter::new(vec!["model-a".to_owned()])
            .save(&router_dir.join("cascade-router.json"))
            .expect("save router state");

        // SAFETY: serialized by cascade_env_lock.
        unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "0") };
        let config = RokoConfig::default();
        let (health, limiter) = test_provider_runtime(&config);
        let result = cascade_select_model(AcpCascadeRequest {
            workdir: tmp.path(),
            roko_config: &config,
            mode: "code",
            prompt: "fix a bug",
            effort: "medium",
            resolved_slug: "model-a",
            model_selection_explicit: false,
            provider_health: &health,
            rate_limiter: &limiter,
        });
        unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

        assert!(result.is_none(), "presence alone must not enable routing");
    }

    /// cascade_select_model returns Some when the env var is set and a valid
    /// router state file exists.  The returned slug must be one of the model
    /// keys known to the router.
    #[test]
    fn cascade_select_model_returns_model_with_router() {
        use roko_learn::cascade_router::CascadeRouter;

        let _guard = cascade_env_lock().lock().expect("acquire env lock");
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        let router_dir = workdir.join(".roko").join("learn");
        std::fs::create_dir_all(&router_dir).expect("create router dir");
        let router_path = router_dir.join("cascade-router.json");

        // Build a minimal router with one model slug and persist it.
        let model_key = "claude-sonnet-4-6".to_string();
        let router = CascadeRouter::new(vec![model_key.clone()]);
        router.save(&router_path).expect("save router state");

        // SAFETY: serialized by cascade_env_lock; no other test holds the lock
        // and mutates ROKO_ACP_CASCADE_SELECT concurrently.
        unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "1") };
        let config = RokoConfig::default();
        let (health, limiter) = test_provider_runtime(&config);
        let result = cascade_select_model(AcpCascadeRequest {
            workdir,
            roko_config: &config,
            mode: "code",
            prompt: "add unit tests",
            effort: "medium",
            resolved_slug: &model_key,
            model_selection_explicit: false,
            provider_health: &health,
            rate_limiter: &limiter,
        });
        unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

        assert!(
            result.is_some(),
            "should return Some when router file exists"
        );
        let result = result.expect("selection");
        assert_eq!(result.model_key, model_key);
        assert_eq!(result.stage, "static");
    }

    #[tokio::test]
    async fn rate_limit_provider_selection_prefers_healthy_capacity_and_honors_explicit_model() {
        let config = RokoConfig::from_toml(
            r#"
[agent]
default_model = "model-a"

[providers.provider-a]
kind = "openai_compat"
base_url = "https://a.example/v1"
[providers.provider-a.limits]
rpm = 1
tpm = 1000

[providers.provider-b]
kind = "openai_compat"
base_url = "https://b.example/v1"
[providers.provider-b.limits]
rpm = 100
tpm = 100000

[models.model-a]
provider = "provider-a"
slug = "wire-a"
context_window = 8192

[models.model-b]
provider = "provider-b"
slug = "wire-b"
context_window = 8192
"#,
        )
        .expect("parse provider selection config");
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let router_dir = tmp.path().join(".roko/learn");
        std::fs::create_dir_all(&router_dir).expect("create router dir");
        CascadeRouter::new(vec!["model-a".to_owned(), "model-b".to_owned()])
            .save(&router_dir.join("cascade-router.json"))
            .expect("save router state");
        let (health, limiter) = test_provider_runtime(&config);

        // Consume provider-a's one-RPM configured window. ACP selection reads
        // this canonical limiter snapshot and retains provider-b as capacity.
        limiter.acquire("provider-a").await;

        let _guard = cascade_env_lock().lock().expect("acquire env lock");
        // SAFETY: serialized with every other cascade env mutation in this module.
        unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "1") };
        let automatic = cascade_select_model(AcpCascadeRequest {
            workdir: tmp.path(),
            roko_config: &config,
            mode: "code",
            prompt: "fix a provider issue",
            effort: "medium",
            resolved_slug: "model-a",
            model_selection_explicit: false,
            provider_health: &health,
            rate_limiter: &limiter,
        })
        .expect("automatic adaptive selection");
        let explicit = cascade_select_model(AcpCascadeRequest {
            workdir: tmp.path(),
            roko_config: &config,
            mode: "code",
            prompt: "fix a provider issue",
            effort: "medium",
            resolved_slug: "model-a",
            model_selection_explicit: true,
            provider_health: &health,
            rate_limiter: &limiter,
        });
        let (degraded_health, fresh_limiter) = test_provider_runtime(&config);
        for _ in 0..3 {
            degraded_health.record_failure(
                "provider-a",
                roko_learn::provider_health::ErrorClass::RateLimit,
            );
        }
        let health_aware = cascade_select_model(AcpCascadeRequest {
            workdir: tmp.path(),
            roko_config: &config,
            mode: "code",
            prompt: "fix a provider issue",
            effort: "medium",
            resolved_slug: "model-a",
            model_selection_explicit: false,
            provider_health: &degraded_health,
            rate_limiter: &fresh_limiter,
        })
        .expect("health-aware adaptive selection");
        unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

        assert_eq!(automatic.model_key, "model-b");
        assert_eq!(health_aware.model_key, "model-b");
        assert!(
            explicit.is_none(),
            "explicit model selection must bypass adaptation"
        );
    }

    /// record_cascade_observation uses the config key (model_key_for_logging),
    /// not the wire slug (resolved_for_logging.slug).  This test verifies the
    /// exact-match path works: config keys that were registered with the router
    /// are always found via model_index_for_slug regardless of family aliasing.
    #[tokio::test]
    async fn cascade_observation_updates_the_dispatched_config_key() {
        use roko_learn::cascade_router::CascadeRouter;

        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        let router_dir = workdir.join(".roko").join("learn");
        std::fs::create_dir_all(&router_dir).expect("create router dir");
        let router_path = router_dir.join("cascade-router.json");

        // Use a config key that has no slug_family alias so exact-match is the
        // only way it can be found.  A custom/short key like "my-model" will
        // not match any family heuristic.
        let config_key = "my-custom-model-key".to_string();
        let router = CascadeRouter::new(vec![config_key.clone()]);
        router.save(&router_path).expect("save initial router");

        record_cascade_observation(
            router_path.clone(),
            config_key.clone(),
            RoutingContext::default(),
            true,
            1_000,
            Some(250),
            vec![config_key.clone()],
        )
        .await
        .expect("observation task");

        let router_loaded = CascadeRouter::load_or_new(&router_path, vec![config_key.clone()]);
        let stats = router_loaded.observation_snapshot();
        assert_eq!(stats.get(&config_key).map(|entry| entry.trials), Some(1));
        assert_eq!(stats.get(&config_key).map(|entry| entry.successes), Some(1));
    }
}
