//! MCP tool resolution, builtin tool handler adapter, and tool helpers.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use roko_agent::dispatcher::HandlerResolver;
use roko_agent::mcp::handler::capability_for_tool;
use roko_agent::mcp::{McpClient, StdioTransport as McpStdioTransport, mcp_to_tool_def};
use roko_agent::safety::capabilities::{PluginTier, check_plugin_tier};
use roko_agent::safety::contract::{AgentContract, ContractLoadMode};
use roko_core::defaults::DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS;
use roko_core::tool::{
    ToolCall, ToolContext, ToolDef, ToolError, ToolHandler, ToolResult, ToolSource,
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::builtin_tools::tool_permission_request;
use crate::session::CancelToken;
use crate::types::{
    ContentBlock, McpInitStatus, McpServerStatus, PermissionDecision, ToolCallKind, ToolCallStatus,
};

use super::{
    CognitiveEvent, PermissionReplyChannel, PermissionRequestPayload, send_cognitive_event,
};

pub(crate) fn write_session_mcp_config(
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

pub(crate) struct SessionMcpRuntime {
    pub(crate) tools: Vec<ToolDef>,
    pub(crate) handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

/// Set up MCP tool handlers for a session.
///
/// `plugin_tier` is the trust tier applied to every discovered tool in this
/// session's MCP servers. ACP sessions default to [`PluginTier::Sandboxed`]
/// because MCP servers connect over stdio and may be third-party. Callers that
/// have verified server provenance may pass a higher tier.
pub(crate) async fn setup_session_mcp_tools(
    session_id: &str,
    mcp_servers: &[crate::types::McpServerConfig],
    plugin_tier: PluginTier,
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
                    plugin_tier,
                }),
            );
            tools.push(def);
        }
    }

    (SessionMcpRuntime { tools, handlers }, statuses)
}

pub(crate) fn sanitize_tool_segment(input: &str) -> String {
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

/// Maximum number of suffix attempts before giving up on unique name generation.
const MAX_UNIQUE_TOOL_NAME_SUFFIX: u32 = 10_000;

pub(crate) fn unique_tool_name(base: &str, used: &mut HashSet<String>) -> String {
    let base: String = base.chars().take(64).collect();
    if used.insert(base.clone()) {
        return base;
    }

    for suffix_num in 2..=MAX_UNIQUE_TOOL_NAME_SUFFIX {
        let suffix = format!("_{suffix_num}");
        let max_base_len = 64usize.saturating_sub(suffix.len());
        let mut candidate: String = base.chars().take(max_base_len).collect();
        candidate.push_str(&suffix);
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }

    // Fallback: generate a UUID-based name instead of panicking.
    let fallback = format!("{}_fallback_{}", &base[..base.len().min(32)], used.len());
    warn!(
        base_name = %base,
        attempts = MAX_UNIQUE_TOOL_NAME_SUFFIX,
        "could not find unique tool name after max suffix attempts; using fallback"
    );
    used.insert(fallback.clone());
    fallback
}

pub(crate) struct AcpMcpHandlerResolver {
    pub(crate) handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl HandlerResolver for AcpMcpHandlerResolver {
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        self.handlers.get(name).cloned()
    }
}

pub(crate) struct AcpMcpToolHandler {
    client: Arc<McpClient<McpStdioTransport>>,
    exposed_name: String,
    remote_name: String,
    event_sender: mpsc::Sender<CognitiveEvent>,
    /// Plugin trust tier for this MCP server. Controls which capabilities the
    /// tool may exercise: `Sandboxed` allows only reads, `Standard` and above
    /// permit writes/exec/network according to the `check_plugin_tier` policy.
    pub(crate) plugin_tier: PluginTier,
}

#[async_trait]
impl ToolHandler for AcpMcpToolHandler {
    fn name(&self) -> &str {
        &self.exposed_name
    }

    async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
        // Plugin-tier gate: check whether this server's tier permits the
        // capability implied by the remote tool name.
        let required_capability = capability_for_tool(&self.remote_name);
        if let Err(reason) = check_plugin_tier(self.plugin_tier, &required_capability) {
            warn!(
                tool = %self.exposed_name,
                remote_tool = %self.remote_name,
                tier = ?self.plugin_tier,
                reason = %reason,
                "ACP MCP tool call denied by plugin tier"
            );
            return ToolResult::err(ToolError::PermissionDenied(format!(
                "MCP tool '{}' denied: {reason}",
                self.exposed_name
            )));
        }

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
pub(crate) struct AcpToolCancelToken(pub(crate) CancelToken);

impl roko_core::tool::CancelToken for AcpToolCancelToken {
    fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

// ── Builtin tool handler adapter ──────────────────────────────────────

/// Wraps [`execute_acp_builtin_tool`] in the [`ToolHandler`] trait so that
/// the [`ToolLoop`] infrastructure can dispatch builtin tools identically to
/// MCP tools.
pub(crate) struct AcpBuiltinToolHandler {
    pub(crate) tool_name: String,
    pub(crate) session_id: String,
    pub(crate) workdir: PathBuf,
    pub(crate) event_sender: mpsc::Sender<CognitiveEvent>,
    /// Agent role used to load the `AgentContract` for capability checking.
    /// When empty, the default role contract is used.
    pub(crate) role: String,
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

        // AgentContract gate: verify the role's behavioral contract permits
        // this tool. Unknown roles fall back to a deny-everything restricted
        // contract so that unrecognised modes fail closed rather than open.
        let role = if self.role.trim().is_empty() {
            "default"
        } else {
            self.role.trim()
        };
        let contract =
            AgentContract::load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)
                .unwrap_or_else(|_| AgentContract::restricted(role));
        if !contract.permits_tool(&self.tool_name) {
            warn!(
                tool = %self.tool_name,
                session_id = %self.session_id,
                role = %role,
                reason = "contract_denied",
                "ACP tool call denied by role contract"
            );
            return ToolResult::err(ToolError::PermissionDenied(format!(
                "tool '{}' is not permitted for role '{role}'",
                self.tool_name
            )));
        }

        debug!(
            tool = %self.tool_name,
            session_id = %self.session_id,
            role = %role,
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
            &ctx.env_passthrough,
            &self.event_sender,
        )
        .await;
        ToolResult::text(output)
    }
}

pub(crate) struct AcpBuiltinHandlerResolver {
    pub(crate) handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl HandlerResolver for AcpBuiltinHandlerResolver {
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        self.handlers.get(name).cloned()
    }
}

pub(crate) fn tool_result_from_mcp(
    tool_name: &str,
    result: &roko_agent::mcp::McpToolResult,
) -> ToolResult {
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

pub(crate) fn mcp_result_text(result: &roko_agent::mcp::McpToolResult) -> String {
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

pub(crate) fn tool_result_for_editor(result: &ToolResult) -> (ToolCallStatus, String) {
    match result {
        ToolResult::Ok { .. } => (ToolCallStatus::Completed, result.text_content()),
        ToolResult::Err(error) => (ToolCallStatus::Failed, format!("error: {error}")),
    }
}
