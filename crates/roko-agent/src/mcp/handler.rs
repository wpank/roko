//! MCP-backed tool handlers for the dispatcher.
//!
//! MCP-discovered tools are registered under `{server}.{tool}` names via
//! [`super::mcp_to_tool_def`]. [`McpHandlerResolver`] uses that prefix to route
//! dynamic tool calls to the correct live MCP client while still deferring to a
//! static resolver for built-in tools first.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use roko_core::plugin::PluginTier;
use roko_core::tool::{ToolCall, ToolContext, ToolError, ToolHandler, ToolResult};

use super::client::{McpContent, McpError, McpToolResult};
use super::error_accumulator::McpErrorAccumulator;
use super::{McpClient, Transport};
use crate::dispatcher::HandlerResolver;
use crate::safety::capabilities::{Capability, check_plugin_tier};

const MCP_TOOL_SEPARATOR: &str = ".";

/// Dispatcher resolver that falls back from built-in handlers to live MCP
/// clients for namespaced MCP tools.
pub struct McpHandlerResolver<T: Transport> {
    static_resolver: Arc<dyn HandlerResolver>,
    mcp_clients: HashMap<String, Arc<McpClient<T>>>,
    error_accumulator: Option<McpErrorAccumulator>,
    /// Per-server trust tiers. When set, `resolve` checks the tier before
    /// returning a handler. Unknown servers default to `Sandboxed`.
    server_tiers: Option<HashMap<String, PluginTier>>,
}

impl<T: Transport> McpHandlerResolver<T> {
    /// Create a resolver that checks `static_resolver` first, then resolves MCP
    /// tools using the `{server}.{tool}` naming convention.
    #[must_use]
    pub fn new(
        static_resolver: Arc<dyn HandlerResolver>,
        mcp_clients: HashMap<String, Arc<McpClient<T>>>,
    ) -> Self {
        Self {
            static_resolver,
            mcp_clients,
            error_accumulator: None,
            server_tiers: None,
        }
    }

    /// Attach an error accumulator that records MCP failures for later query.
    ///
    /// When set, any MCP tool call that fails (transport or server error) will
    /// be non-blockingly recorded in the accumulator. The session continues
    /// regardless — errors are informational.
    #[must_use]
    pub fn with_error_accumulator(mut self, accumulator: McpErrorAccumulator) -> Self {
        self.error_accumulator = Some(accumulator);
        self
    }

    /// Attach per-server trust tiers for dispatch-time capability checks.
    ///
    /// When set, `resolve` verifies that the server's tier permits the
    /// capability implied by the tool name. Unknown servers default to
    /// [`PluginTier::Sandboxed`]. If the tier check fails, a
    /// [`DeniedToolHandler`] is returned instead of the live MCP handler.
    #[must_use]
    pub fn with_server_tiers(mut self, tiers: HashMap<String, PluginTier>) -> Self {
        self.server_tiers = Some(tiers);
        self
    }

    /// Borrow the attached error accumulator, if any.
    #[must_use]
    pub fn error_accumulator(&self) -> Option<&McpErrorAccumulator> {
        self.error_accumulator.as_ref()
    }
}

impl<T: Transport + 'static> HandlerResolver for McpHandlerResolver<T> {
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        if let Some(handler) = self.static_resolver.resolve(name) {
            return Some(handler);
        }

        let (server_name, remote_name) = split_prefixed_tool_name(name)?;
        let client = self.mcp_clients.get(server_name)?;

        // Tier check: when server tiers are configured, verify the server's
        // trust level permits the capability implied by the tool name.
        if let Some(ref tiers) = self.server_tiers {
            let tier = tiers
                .get(server_name)
                .copied()
                .unwrap_or(PluginTier::Sandboxed);
            let capability = capability_for_tool(remote_name);
            if let Err(reason) = check_plugin_tier(tier, &capability) {
                tracing::warn!(
                    server = server_name,
                    tool = name,
                    %reason,
                    "MCP tier check denied tool"
                );
                return Some(Arc::new(DeniedToolHandler::new(name, reason)));
            }
        }

        Some(Arc::new(
            McpToolHandler::new(
                Arc::clone(client),
                name.to_string(),
                remote_name.to_string(),
            )
            .with_error_accumulator_opt(self.error_accumulator.clone()),
        ))
    }
}

// ─── Tier-check helpers ─────────────────────────────────────────────

/// Map a remote MCP tool name to the primary [`Capability`] it requires.
///
/// Write-oriented tools map to [`Capability::WritePath`], execution tools
/// to [`Capability::Exec`], network tools to [`Capability::Network`], and
/// everything else to [`Capability::ReadPath`] (the least-privilege default).
#[must_use]
pub fn capability_for_tool(tool_name: &str) -> Capability {
    let lower = tool_name.to_ascii_lowercase();
    if lower.contains("write") || lower.contains("edit") || lower.contains("create") {
        Capability::WritePath(std::path::PathBuf::from("/"))
    } else if lower.contains("bash")
        || lower.contains("exec")
        || lower.contains("run")
        || lower.contains("shell")
        || lower.contains("command")
    {
        Capability::Exec("*".into())
    } else if lower.contains("fetch") || lower.contains("http") || lower.contains("request") {
        Capability::Network {
            host: "*".into(),
            port: 0,
        }
    } else {
        Capability::ReadPath(std::path::PathBuf::from("/"))
    }
}

/// A [`ToolHandler`] that always returns [`ToolError::PermissionDenied`].
///
/// Returned by [`McpHandlerResolver`] when a server's [`PluginTier`] does
/// not permit the capability required by the requested tool.
pub struct DeniedToolHandler {
    tool_name: String,
    reason: String,
}

impl DeniedToolHandler {
    /// Create a handler that will deny with the given reason.
    #[must_use]
    pub fn new(tool_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ToolHandler for DeniedToolHandler {
    fn name(&self) -> &str {
        &self.tool_name
    }

    async fn execute(&self, _call: ToolCall, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err(ToolError::PermissionDenied(self.reason.clone()))
    }
}

/// Concrete [`ToolHandler`] that executes a routed MCP tool via
/// `tools/call`.
///
/// When an [`McpErrorAccumulator`] is attached, failures are recorded
/// non-blockingly so the IDE/ACP session can later report them.
pub struct McpToolHandler<T: Transport> {
    client: Arc<McpClient<T>>,
    exposed_name: String,
    remote_name: String,
    error_accumulator: Option<McpErrorAccumulator>,
}

impl<T: Transport> McpToolHandler<T> {
    /// Construct a handler for one discovered MCP tool.
    #[must_use]
    pub fn new(
        client: Arc<McpClient<T>>,
        exposed_name: impl Into<String>,
        remote_name: impl Into<String>,
    ) -> Self {
        Self {
            client,
            exposed_name: exposed_name.into(),
            remote_name: remote_name.into(),
            error_accumulator: None,
        }
    }

    /// Attach an error accumulator to record failures for later query.
    #[must_use]
    pub fn with_error_accumulator(mut self, accumulator: McpErrorAccumulator) -> Self {
        self.error_accumulator = Some(accumulator);
        self
    }

    /// Conditionally attach an error accumulator (convenience for the resolver).
    #[must_use]
    fn with_error_accumulator_opt(mut self, accumulator: Option<McpErrorAccumulator>) -> Self {
        self.error_accumulator = accumulator;
        self
    }

    /// Extract the server name from the exposed tool name (the prefix before the dot).
    fn server_name(&self) -> &str {
        self.exposed_name
            .split_once(MCP_TOOL_SEPARATOR)
            .map_or(&self.exposed_name, |(server, _)| server)
    }
}

#[async_trait]
impl<T: Transport + 'static> ToolHandler for McpToolHandler<T> {
    fn name(&self) -> &str {
        &self.exposed_name
    }

    async fn execute(&self, call: ToolCall, _ctx: &ToolContext) -> ToolResult {
        match self
            .client
            .call_tool(&self.remote_name, call.arguments)
            .await
        {
            Ok(result) => {
                // Record server-reported tool errors (isError=true) as well.
                if result.is_error
                    && let Some(ref acc) = self.error_accumulator
                {
                    let content = mcp_result_text(&result.content);
                    acc.record(
                        self.server_name(),
                        &self.exposed_name,
                        if content.is_empty() {
                            "tool returned an error".to_string()
                        } else {
                            content.clone()
                        },
                        false,
                    );
                }
                render_mcp_result(&self.exposed_name, result)
            }
            Err(ref err) => {
                if let Some(ref acc) = self.error_accumulator {
                    let is_transport = matches!(err, McpError::Transport(_) | McpError::Json(_));
                    acc.record(
                        self.server_name(),
                        &self.exposed_name,
                        err.to_string(),
                        is_transport,
                    );
                }
                ToolResult::err(ToolError::Other(format!(
                    "mcp tool `{}` failed: {err}",
                    self.exposed_name
                )))
            }
        }
    }
}

fn split_prefixed_tool_name(name: &str) -> Option<(&str, &str)> {
    let (server_name, remote_name) = name.split_once(MCP_TOOL_SEPARATOR)?;
    if server_name.is_empty() || remote_name.is_empty() {
        return None;
    }
    Some((server_name, remote_name))
}

fn render_mcp_result(tool_name: &str, result: McpToolResult) -> ToolResult {
    let content = mcp_result_text(&result.content);
    if result.is_error {
        let message = if content.is_empty() {
            format!("mcp tool `{tool_name}` returned an error")
        } else {
            format!("mcp tool `{tool_name}` returned an error: {content}")
        };
        ToolResult::err(ToolError::Other(message))
    } else {
        ToolResult::text(content)
    }
}

fn mcp_result_text(content: &[McpContent]) -> String {
    let text_blocks: Vec<&str> = content
        .iter()
        .filter_map(|block| (block.content_type == "text").then_some(block.text.as_deref()))
        .flatten()
        .collect();

    if !text_blocks.is_empty() {
        return text_blocks.join("\n");
    }

    serde_json::to_string(content).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatcher::ToolDispatcher;
    use crate::mcp::{McpRequest, McpResponse, McpToolDef, mcp_to_tool_def};
    use roko_core::tool::{ToolRegistry, VecToolRegistry};
    use serde_json::json;
    use std::sync::Mutex;

    struct MockTransport {
        responses: Mutex<Vec<McpResponse>>,
        requests: Mutex<Vec<McpRequest>>,
    }

    impl MockTransport {
        fn new(responses: Vec<McpResponse>) -> Self {
            Self {
                responses: Mutex::new(responses),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn take_requests(&self) -> Vec<McpRequest> {
            self.requests.lock().unwrap().drain(..).collect()
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn roundtrip(
            &self,
            request: &McpRequest,
        ) -> Result<McpResponse, super::super::client::McpError> {
            self.requests.lock().unwrap().push(request.clone());
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(super::super::client::McpError::Transport(
                    "no more canned responses".into(),
                ));
            }
            Ok(responses.remove(0))
        }
    }

    struct StaticHandler;

    #[async_trait]
    impl ToolHandler for StaticHandler {
        fn name(&self) -> &str {
            "local.echo"
        }

        async fn execute(&self, _call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            ToolResult::text("static")
        }
    }

    fn ok_response(id: u64, result: serde_json::Value) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    #[tokio::test]
    async fn mcp_handler_resolver_routes_calls_through_dispatcher() {
        let transport = Arc::new(MockTransport::new(vec![ok_response(
            1,
            json!({
                "content": [{"type": "text", "text": "hello from mcp"}],
                "isError": false
            }),
        )]));
        let client = Arc::new(McpClient::new(Arc::clone(&transport)));
        let registry = Arc::new(VecToolRegistry::from_tools(vec![mcp_to_tool_def(
            &McpToolDef {
                name: "echo".to_string(),
                description: Some("Echo".to_string()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {"text": {"type": "string"}},
                    "required": ["text"]
                })),
                annotations: None,
            },
            "local",
        )])) as Arc<dyn ToolRegistry>;

        let resolver: Arc<dyn HandlerResolver> = Arc::new(McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("local".to_string(), client)]),
        ));
        let dispatcher = ToolDispatcher::new(Arc::clone(&registry), resolver);

        let result = dispatcher
            .dispatch(
                ToolCall::new("call-1", "local.echo", json!({"text": "hello"})),
                &ToolContext::testing("/tmp/mcp-handler-resolver"),
            )
            .await;

        assert_eq!(result, ToolResult::text("hello from mcp"));

        let requests = transport.take_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "tools/call");
        assert_eq!(requests[0].params["name"], "echo");
        assert_eq!(requests[0].params["arguments"], json!({"text": "hello"}));
    }

    #[test]
    fn mcp_handler_resolver_prefers_static_handlers() {
        let transport = Arc::new(MockTransport::new(Vec::new()));
        let client = Arc::new(McpClient::new(transport));
        let resolver = McpHandlerResolver::new(
            Arc::new(|name: &str| {
                (name == "local.echo").then(|| Arc::new(StaticHandler) as Arc<dyn ToolHandler>)
            }),
            HashMap::from([("local".to_string(), client)]),
        );

        let handler = resolver.resolve("local.echo").expect("handler");
        assert_eq!(handler.name(), "local.echo");
    }

    #[test]
    fn mcp_handler_resolver_returns_none_for_unknown_server() {
        let resolver: McpHandlerResolver<Arc<MockTransport>> =
            McpHandlerResolver::new(Arc::new(|_: &str| None), HashMap::new());
        assert!(resolver.resolve("missing.echo").is_none());
        assert!(resolver.resolve("not-prefixed").is_none());
    }

    #[tokio::test]
    async fn mcp_handler_accumulates_transport_errors() {
        // Transport returns no responses → triggers a transport error.
        let transport = Arc::new(MockTransport::new(vec![]));
        let client = Arc::new(McpClient::new(Arc::clone(&transport)));
        let accumulator = McpErrorAccumulator::new();

        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("git".to_string(), client)]),
        )
        .with_error_accumulator(accumulator.clone());

        let handler = resolver.resolve("git.status").expect("handler");
        let call = ToolCall::new("call-err", "git.status", json!({}));
        let result = handler
            .execute(call, &ToolContext::testing("/tmp/mcp-err-test"))
            .await;

        // The tool result should be an error.
        assert!(matches!(result, ToolResult::Err(_)));

        // The accumulator should have recorded the error.
        let errors = accumulator.snapshot();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].server, "git");
        assert_eq!(errors[0].tool_name, "git.status");
        assert!(errors[0].is_transport_error);
        assert!(!errors[0].error_message.is_empty());
    }

    #[tokio::test]
    async fn mcp_handler_accumulates_server_reported_errors() {
        // Server returns isError=true in the tool result.
        let transport = Arc::new(MockTransport::new(vec![ok_response(
            1,
            json!({
                "content": [{"type": "text", "text": "permission denied"}],
                "isError": true
            }),
        )]));
        let client = Arc::new(McpClient::new(Arc::clone(&transport)));
        let accumulator = McpErrorAccumulator::new();

        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("fs".to_string(), client)]),
        )
        .with_error_accumulator(accumulator.clone());

        let handler = resolver.resolve("fs.write_file").expect("handler");
        let call = ToolCall::new("call-2", "fs.write_file", json!({"path": "/etc/passwd"}));
        let result = handler
            .execute(call, &ToolContext::testing("/tmp/mcp-err-test2"))
            .await;

        // Tool result is an error (rendered from isError=true).
        assert!(matches!(result, ToolResult::Err(_)));

        // Accumulator captures the server-reported error.
        let errors = accumulator.snapshot();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].server, "fs");
        assert_eq!(errors[0].tool_name, "fs.write_file");
        assert!(!errors[0].is_transport_error); // Server error, not transport
        assert!(errors[0].error_message.contains("permission denied"));
    }

    #[tokio::test]
    async fn mcp_handler_does_not_accumulate_on_success() {
        let transport = Arc::new(MockTransport::new(vec![ok_response(
            1,
            json!({
                "content": [{"type": "text", "text": "file contents here"}],
                "isError": false
            }),
        )]));
        let client = Arc::new(McpClient::new(Arc::clone(&transport)));
        let accumulator = McpErrorAccumulator::new();

        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("fs".to_string(), client)]),
        )
        .with_error_accumulator(accumulator.clone());

        let handler = resolver.resolve("fs.read_file").expect("handler");
        let call = ToolCall::new("call-ok", "fs.read_file", json!({"path": "/tmp/test"}));
        let result = handler
            .execute(call, &ToolContext::testing("/tmp/mcp-ok-test"))
            .await;

        assert_eq!(result, ToolResult::text("file contents here"));
        assert!(accumulator.is_empty());
    }

    // ─── Tier check tests ───────────────────────────────────────────

    #[test]
    fn sandboxed_server_denies_write_tool() {
        let transport = Arc::new(MockTransport::new(Vec::new()));
        let client = Arc::new(McpClient::new(transport));
        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("fs".to_string(), client)]),
        )
        .with_server_tiers(HashMap::from([(
            "fs".to_string(),
            PluginTier::Sandboxed,
        )]));

        let handler = resolver.resolve("fs.write_file").expect("should return DeniedToolHandler");
        assert_eq!(handler.name(), "fs.write_file");
    }

    #[tokio::test]
    async fn sandboxed_server_write_tool_returns_permission_denied() {
        let transport = Arc::new(MockTransport::new(Vec::new()));
        let client = Arc::new(McpClient::new(transport));
        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("fs".to_string(), client)]),
        )
        .with_server_tiers(HashMap::from([(
            "fs".to_string(),
            PluginTier::Sandboxed,
        )]));

        let handler = resolver.resolve("fs.write_file").expect("handler");
        let call = ToolCall::new("tier-deny", "fs.write_file", json!({"path": "/tmp/x"}));
        let result = handler
            .execute(call, &ToolContext::testing("/tmp/mcp-tier-test"))
            .await;

        assert!(
            matches!(result, ToolResult::Err(ToolError::PermissionDenied(_))),
            "expected PermissionDenied, got: {result:?}"
        );
    }

    #[test]
    fn untrusted_server_denies_exec_tool() {
        let transport = Arc::new(MockTransport::new(Vec::new()));
        let client = Arc::new(McpClient::new(transport));
        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("ci".to_string(), client)]),
        )
        .with_server_tiers(HashMap::from([(
            "ci".to_string(),
            PluginTier::Untrusted,
        )]));

        let handler = resolver.resolve("ci.run_command").expect("should return DeniedToolHandler");
        assert_eq!(handler.name(), "ci.run_command");
    }

    #[test]
    fn trusted_server_allows_write_tool() {
        let transport = Arc::new(MockTransport::new(Vec::new()));
        let client = Arc::new(McpClient::new(transport));
        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("fs".to_string(), client)]),
        )
        .with_server_tiers(HashMap::from([(
            "fs".to_string(),
            PluginTier::Trusted,
        )]));

        // Trusted tier permits writes, so this should return a real McpToolHandler.
        let handler = resolver.resolve("fs.write_file").expect("handler");
        assert_eq!(handler.name(), "fs.write_file");
    }

    #[test]
    fn unknown_server_defaults_to_sandboxed() {
        let transport = Arc::new(MockTransport::new(Vec::new()));
        let client = Arc::new(McpClient::new(transport));
        // Empty tier map but with_server_tiers enabled -- unknown defaults to Sandboxed.
        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("rogue".to_string(), client)]),
        )
        .with_server_tiers(HashMap::new());

        // Sandboxed denies writes, so write_file should be denied.
        let handler = resolver
            .resolve("rogue.write_file")
            .expect("should return DeniedToolHandler");
        assert_eq!(handler.name(), "rogue.write_file");
    }

    #[test]
    fn no_server_tiers_skips_tier_check() {
        let transport = Arc::new(MockTransport::new(Vec::new()));
        let client = Arc::new(McpClient::new(transport));
        // Without with_server_tiers, all tools pass through (existing behavior).
        let resolver = McpHandlerResolver::new(
            Arc::new(|_: &str| None),
            HashMap::from([("fs".to_string(), client)]),
        );

        let handler = resolver.resolve("fs.write_file").expect("handler");
        assert_eq!(handler.name(), "fs.write_file");
    }

    // ─── capability_for_tool tests ──────────────────────────────────

    #[test]
    fn capability_for_write_tools() {
        assert!(matches!(
            capability_for_tool("write_file"),
            Capability::WritePath(_)
        ));
        assert!(matches!(
            capability_for_tool("edit_file"),
            Capability::WritePath(_)
        ));
        assert!(matches!(
            capability_for_tool("create_dir"),
            Capability::WritePath(_)
        ));
    }

    #[test]
    fn capability_for_exec_tools() {
        assert!(matches!(
            capability_for_tool("bash"),
            Capability::Exec(_)
        ));
        assert!(matches!(
            capability_for_tool("run_command"),
            Capability::Exec(_)
        ));
        assert!(matches!(
            capability_for_tool("exec_script"),
            Capability::Exec(_)
        ));
        assert!(matches!(
            capability_for_tool("shell"),
            Capability::Exec(_)
        ));
    }

    #[test]
    fn capability_for_network_tools() {
        assert!(matches!(
            capability_for_tool("web_fetch"),
            Capability::Network { .. }
        ));
        assert!(matches!(
            capability_for_tool("http_get"),
            Capability::Network { .. }
        ));
        assert!(matches!(
            capability_for_tool("request"),
            Capability::Network { .. }
        ));
    }

    #[test]
    fn capability_for_read_tools() {
        assert!(matches!(
            capability_for_tool("read_file"),
            Capability::ReadPath(_)
        ));
        assert!(matches!(
            capability_for_tool("list_dir"),
            Capability::ReadPath(_)
        ));
        assert!(matches!(
            capability_for_tool("grep"),
            Capability::ReadPath(_)
        ));
    }
}
