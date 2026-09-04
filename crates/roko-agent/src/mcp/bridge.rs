//! MCP discovery bridge for HTTP-backed tool loops.
//!
//! Claude CLI forwards MCP config directly to the subprocess via
//! `--mcp-config`. HTTP backends cannot do that, so they must discover MCP
//! tools up front, convert them into canonical [`ToolDef`] values, and let the
//! normal translator render them into backend-specific function definitions.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use roko_core::tool::{ToolDef, ToolSource};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, timeout};

use super::{
    MCP_TOOL_SEPARATOR, McpClient, McpConfig, McpHandlerResolver, McpTransportConfig,
    StdioTransport, Transport, dedup_tools, mcp_to_tool_def,
};
use crate::dispatcher::HandlerResolver;
use crate::mcp::client::McpError;

const MCP_DISCOVERY_TIMEOUT: Duration =
    Duration::from_secs(roko_core::defaults::DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);

// ── MCP test report ─────────────────────────────────────────────────────

/// Overall status of an MCP server test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTestStatus {
    /// Both initialize and tools/list succeeded.
    Ok,
    /// At least one stage failed.
    Failed,
}

impl fmt::Display for McpTestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Per-stage outcome from an MCP test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTestStageResult {
    /// Name of the stage (`"initialize"` or `"tools_list"`).
    pub stage: String,
    /// Whether the stage succeeded.
    pub success: bool,
    /// Latency in milliseconds (omitted when the stage was not reached).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Error message, if the stage failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Structured report returned by `test_mcp_server`.
///
/// Designed for both human-readable text and JSON `--json` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTestReport {
    /// Path to the MCP config file used.
    pub config_path: PathBuf,
    /// Name of the server tested.
    pub server: String,
    /// Whether the command binary was found on the system.
    pub command_available: bool,
    /// Per-stage results (initialize, tools/list).
    pub stages: Vec<McpTestStageResult>,
    /// Protocol version returned by the server (if initialize succeeded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    /// Number of tools discovered.
    pub tool_count: usize,
    /// Tool names (descriptions omitted for security).
    pub tool_names: Vec<String>,
    /// Redacted stderr summary (up to 4 KiB, secrets stripped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_summary: Option<String>,
    /// Overall status.
    pub status: McpTestStatus,
}

/// Type-erased transport retained by an [`McpRuntime`].
pub type McpRuntimeTransport = Arc<dyn Transport>;
/// Initialized client retained for one configured MCP server.
pub type McpRuntimeClient = Arc<McpClient<McpRuntimeTransport>>;

/// Observable lifecycle state for a single MCP server connection (T013).
///
/// Surfaced through [`McpRuntime::lifecycle_state`] so dashboards, health
/// checks, and the parity matrix can inspect MCP state without owning
/// the transport.
#[derive(Debug, Clone)]
pub struct McpLifecycleState {
    /// Name of the MCP server (matches [`McpServerConfig::name`]).
    pub server_name: String,
    /// When the server was last successfully health-checked (initialize + tools/list).
    pub last_health_check: Option<Instant>,
    /// Last error encountered during initialization or tool listing.
    pub last_error: Option<String>,
    /// Capabilities returned by the server's initialize response.
    pub negotiated_capabilities: Option<serde_json::Value>,
    /// Names of tools discovered from this server.
    pub available_tools: Vec<String>,
}

/// Discovered MCP definitions together with the initialized clients that can
/// execute them.
///
/// HTTP-provider tool loops must retain this value for their lifetime. Keeping
/// definitions alone would advertise tools whose stdio child and `tools/call`
/// channel had already been dropped.
#[derive(Clone)]
pub struct McpRuntime {
    tools: Arc<Vec<ToolDef>>,
    clients: Arc<HashMap<String, McpRuntimeClient>>,
    owner: Option<Arc<dyn Send + Sync>>,
    lifecycle: Arc<Vec<McpLifecycleState>>,
}

struct McpRuntimeResolver {
    inner: McpHandlerResolver<McpRuntimeTransport>,
    // Retains the initialized clients and, for synchronous provider
    // construction, the Tokio runtime that owns their stdio I/O drivers.
    _runtime: McpRuntime,
}

impl HandlerResolver for McpRuntimeResolver {
    fn resolve(&self, name: &str) -> Option<Arc<dyn roko_core::tool::ToolHandler>> {
        self.inner.resolve(name)
    }
}

impl fmt::Debug for McpRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpRuntime")
            .field("tool_count", &self.tools.len())
            .field("servers", &self.clients.keys().collect::<Vec<_>>())
            .field("has_runtime_owner", &self.owner.is_some())
            .field("lifecycle_entries", &self.lifecycle.len())
            .finish()
    }
}

impl McpRuntime {
    /// Build a runtime from already initialized clients.
    ///
    /// This constructor is public so embedding applications and tests that own
    /// a non-stdio transport can use the same provider resolver path.
    #[must_use]
    pub fn from_clients(tools: Vec<ToolDef>, clients: HashMap<String, McpRuntimeClient>) -> Self {
        Self {
            tools: Arc::new(tools),
            clients: Arc::new(clients),
            owner: None,
            lifecycle: Arc::new(Vec::new()),
        }
    }

    /// Build a runtime with lifecycle state from discovery.
    #[must_use]
    pub fn from_clients_with_lifecycle(
        tools: Vec<ToolDef>,
        clients: HashMap<String, McpRuntimeClient>,
        lifecycle: Vec<McpLifecycleState>,
    ) -> Self {
        Self {
            tools: Arc::new(tools),
            clients: Arc::new(clients),
            owner: None,
            lifecycle: Arc::new(lifecycle),
        }
    }

    /// Retain an opaque owner required by the clients (for example the Tokio
    /// runtime that owns stdio process I/O registered during synchronous
    /// provider construction).
    #[must_use]
    pub(crate) fn with_owner(mut self, owner: Arc<dyn Send + Sync>) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Canonical definitions exposed to the model.
    #[must_use]
    pub fn tools(&self) -> &Arc<Vec<ToolDef>> {
        &self.tools
    }

    /// Definitions that cannot be routed to one of this runtime's clients.
    ///
    /// Runtime tool names must use the same `{server}.{tool}` namespace as
    /// their [`ToolSource::Mcp`] provenance. Treat malformed or non-MCP
    /// definitions as unexecutable too so provider construction can fail
    /// before advertising them to a model.
    #[must_use]
    pub fn unexecutable_tools(&self) -> Vec<String> {
        let mut missing = self
            .tools
            .iter()
            .filter(|tool| match &tool.source {
                ToolSource::Mcp { server } => {
                    let expected_prefix = format!("{server}.");
                    server.trim().is_empty()
                        || server.contains(MCP_TOOL_SEPARATOR)
                        || !self.clients.contains_key(server)
                        || tool
                            .name
                            .strip_prefix(&expected_prefix)
                            .is_none_or(str::is_empty)
                }
                _ => true,
            })
            .map(|tool| tool.name.clone())
            .collect::<Vec<_>>();
        missing.sort();
        missing.dedup();
        missing
    }

    /// Compose built-in handlers with the retained MCP clients.
    ///
    /// The static resolver wins on collisions, matching the dynamic registry's
    /// default built-in-first policy.
    #[must_use]
    pub fn resolver(&self, static_resolver: Arc<dyn HandlerResolver>) -> Arc<dyn HandlerResolver> {
        Arc::new(McpRuntimeResolver {
            inner: McpHandlerResolver::new(static_resolver, self.clients.as_ref().clone()),
            _runtime: self.clone(),
        })
    }

    /// Observable lifecycle state for each MCP server (T013).
    ///
    /// Returns lifecycle information captured during discovery. Dashboards
    /// and health checks use this to display MCP connection status without
    /// owning the transport or performing additional I/O.
    #[must_use]
    pub fn lifecycle_state(&self) -> &[McpLifecycleState] {
        &self.lifecycle
    }

    /// Number of connected MCP servers.
    #[must_use]
    pub fn server_count(&self) -> usize {
        self.clients.len()
    }
}

/// Errors raised while discovering MCP tools for HTTP backends.
#[derive(Debug, thiserror::Error)]
pub enum McpBridgeError {
    #[error(
        "MCP server name '{server}' is invalid; names must be non-empty and cannot contain '.'"
    )]
    InvalidServerName { server: String },
    #[error("MCP server name '{server}' is configured more than once")]
    DuplicateServerName { server: String },
    #[error("failed to spawn MCP server '{server}': {source}")]
    Spawn { server: String, source: McpError },
    #[error("MCP server '{server}' uses unsupported transport '{transport}'")]
    UnsupportedTransport { server: String, transport: String },
    #[error("MCP server '{server}' initialize timed out after {timeout_secs}s")]
    InitializeTimeout { server: String, timeout_secs: u64 },
    #[error("MCP server '{server}' initialize failed: {source}")]
    Initialize { server: String, source: McpError },
    #[error("MCP server '{server}' tools/list timed out after {timeout_secs}s")]
    ListToolsTimeout { server: String, timeout_secs: u64 },
    #[error("MCP server '{server}' tools/list failed: {source}")]
    ListTools { server: String, source: McpError },
}

/// Discover MCP tools and retain their initialized clients for later
/// `tools/call` execution by HTTP-provider tool loops.
pub async fn discover_mcp_runtime(config: &McpConfig) -> Result<McpRuntime, McpBridgeError> {
    let mut server_names = HashSet::new();
    for server in &config.servers {
        if server.name.trim().is_empty() || server.name.contains(MCP_TOOL_SEPARATOR) {
            return Err(McpBridgeError::InvalidServerName {
                server: server.name.clone(),
            });
        }
        if !server_names.insert(server.name.clone()) {
            return Err(McpBridgeError::DuplicateServerName {
                server: server.name.clone(),
            });
        }
    }

    let mut all_server_tools = Vec::new();
    let mut clients = HashMap::new();
    let mut lifecycle_states = Vec::new();

    for server in &config.servers {
        if server.transport != McpTransportConfig::Stdio {
            return Err(McpBridgeError::UnsupportedTransport {
                server: server.name.clone(),
                transport: format!("{:?}", server.transport).to_ascii_lowercase(),
            });
        }

        let transport = StdioTransport::spawn_with_env(&server.command, &server.args, &server.env)
            .map_err(|source| McpBridgeError::Spawn {
                server: server.name.clone(),
                source,
            })?;
        let transport: McpRuntimeTransport = Arc::new(transport);
        let client = Arc::new(McpClient::new(transport));

        let negotiated_capabilities =
            match timeout(MCP_DISCOVERY_TIMEOUT, client.initialize()).await {
                Ok(Ok(caps)) => Some(caps),
                Ok(Err(source)) => {
                    return Err(McpBridgeError::Initialize {
                        server: server.name.clone(),
                        source,
                    });
                }
                Err(_) => {
                    return Err(McpBridgeError::InitializeTimeout {
                        server: server.name.clone(),
                        timeout_secs: MCP_DISCOVERY_TIMEOUT.as_secs(),
                    });
                }
            };

        let mcp_tools = match timeout(MCP_DISCOVERY_TIMEOUT, client.list_tools()).await {
            Ok(Ok(tools)) => tools,
            Ok(Err(source)) => {
                return Err(McpBridgeError::ListTools {
                    server: server.name.clone(),
                    source,
                });
            }
            Err(_) => {
                return Err(McpBridgeError::ListToolsTimeout {
                    server: server.name.clone(),
                    timeout_secs: MCP_DISCOVERY_TIMEOUT.as_secs(),
                });
            }
        };

        let tool_names: Vec<String> = mcp_tools.iter().map(|t| t.name.clone()).collect();
        let defs = mcp_tools
            .iter()
            .map(|tool| mcp_to_tool_def(tool, &server.name))
            .collect();
        all_server_tools.push((server.name.clone(), defs));
        clients.insert(server.name.clone(), client);

        lifecycle_states.push(McpLifecycleState {
            server_name: server.name.clone(),
            last_health_check: Some(Instant::now()),
            last_error: None,
            negotiated_capabilities,
            available_tools: tool_names,
        });
    }

    Ok(McpRuntime::from_clients_with_lifecycle(
        dedup_tools(all_server_tools),
        clients,
        lifecycle_states,
    ))
}

/// Discover and convert MCP tools without retaining their execution clients.
///
/// Prefer [`discover_mcp_runtime`] for any tool loop. This definition-only
/// helper remains for callers that render or inspect schemas but never execute
/// tool calls.
pub async fn discover_mcp_tools(config: &McpConfig) -> Result<Vec<ToolDef>, McpBridgeError> {
    discover_mcp_runtime(config)
        .await
        .map(|runtime| runtime.tools().as_ref().clone())
}

/// Test a single configured MCP server by running initialize + tools/list
/// in diagnostic mode.
///
/// This function spawns the server process with stderr captured (up to 4 KiB),
/// runs the two protocol stages with the configured timeout, and shuts down
/// the child cleanly. The resulting [`McpTestReport`] can be rendered as text
/// or JSON for `roko config mcp test`.
///
/// `timeout` overrides the default [`MCP_DISCOVERY_TIMEOUT`] when `Some`.
pub async fn test_mcp_server(
    server: &super::McpServerConfig,
    config_path: PathBuf,
    custom_timeout: Option<Duration>,
) -> McpTestReport {
    let discovery_timeout = custom_timeout.unwrap_or(MCP_DISCOVERY_TIMEOUT);
    let command_available = super::is_command_on_path(&server.command);

    // Spawn in diagnostic mode so we can capture stderr.
    let transport =
        match StdioTransport::spawn_diagnostic(&server.command, &server.args, &server.env) {
            Ok(t) => t,
            Err(err) => {
                return McpTestReport {
                    config_path,
                    server: server.name.clone(),
                    command_available,
                    stages: vec![McpTestStageResult {
                        stage: "spawn".to_string(),
                        success: false,
                        latency_ms: None,
                        error: Some(err.to_string()),
                    }],
                    protocol_version: None,
                    tool_count: 0,
                    tool_names: vec![],
                    stderr_summary: None,
                    status: McpTestStatus::Failed,
                };
            }
        };

    let env_values: Vec<String> = server.env.values().cloned().collect();
    let transport = Arc::new(transport);
    let client = McpClient::new(Arc::clone(&transport) as McpRuntimeTransport);
    let mut stages = Vec::new();
    let mut protocol_version = None;
    let mut tool_names = Vec::new();
    let mut overall_ok = true;

    // Stage 1: initialize
    let init_start = Instant::now();
    match timeout(discovery_timeout, client.initialize()).await {
        Ok(Ok(caps)) => {
            stages.push(McpTestStageResult {
                stage: "initialize".to_string(),
                success: true,
                latency_ms: Some(init_start.elapsed().as_millis() as u64),
                error: None,
            });
            protocol_version = caps
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .map(String::from);
        }
        Ok(Err(err)) => {
            stages.push(McpTestStageResult {
                stage: "initialize".to_string(),
                success: false,
                latency_ms: Some(init_start.elapsed().as_millis() as u64),
                error: Some(err.to_string()),
            });
            overall_ok = false;
        }
        Err(_) => {
            stages.push(McpTestStageResult {
                stage: "initialize".to_string(),
                success: false,
                latency_ms: Some(discovery_timeout.as_millis() as u64),
                error: Some(format!("timed out after {}s", discovery_timeout.as_secs())),
            });
            overall_ok = false;
        }
    }

    // Stage 2: tools/list (only if initialize succeeded)
    if overall_ok {
        let list_start = Instant::now();
        match timeout(discovery_timeout, client.list_tools()).await {
            Ok(Ok(tools)) => {
                tool_names = tools.iter().map(|t| t.name.clone()).collect();
                stages.push(McpTestStageResult {
                    stage: "tools_list".to_string(),
                    success: true,
                    latency_ms: Some(list_start.elapsed().as_millis() as u64),
                    error: None,
                });
            }
            Ok(Err(err)) => {
                stages.push(McpTestStageResult {
                    stage: "tools_list".to_string(),
                    success: false,
                    latency_ms: Some(list_start.elapsed().as_millis() as u64),
                    error: Some(err.to_string()),
                });
                overall_ok = false;
            }
            Err(_) => {
                stages.push(McpTestStageResult {
                    stage: "tools_list".to_string(),
                    success: false,
                    latency_ms: Some(discovery_timeout.as_millis() as u64),
                    error: Some(format!("timed out after {}s", discovery_timeout.as_secs())),
                });
                overall_ok = false;
            }
        }
    }

    // Capture stderr before shutdown (Arc derefs to StdioTransport).
    let stderr_summary = transport.drain_stderr(&env_values).await;

    // Clean shutdown.
    transport.shutdown().await;

    let tool_count = tool_names.len();
    McpTestReport {
        config_path,
        server: server.name.clone(),
        command_available,
        stages,
        protocol_version,
        tool_count,
        tool_names,
        stderr_summary,
        status: if overall_ok {
            McpTestStatus::Ok
        } else {
            McpTestStatus::Failed
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::McpServerConfig;

    #[tokio::test]
    async fn runtime_rejects_ambiguous_server_names_before_spawning() {
        for name in ["", "nested.server"] {
            let error = discover_mcp_runtime(&McpConfig {
                servers: vec![McpServerConfig {
                    name: name.to_string(),
                    ..Default::default()
                }],
            })
            .await
            .expect_err("invalid name");
            assert!(matches!(error, McpBridgeError::InvalidServerName { .. }));
        }

        let error = discover_mcp_runtime(&McpConfig {
            servers: vec![
                McpServerConfig {
                    name: "local".to_string(),
                    ..Default::default()
                },
                McpServerConfig {
                    name: "local".to_string(),
                    ..Default::default()
                },
            ],
        })
        .await
        .expect_err("duplicate name");
        assert!(matches!(error, McpBridgeError::DuplicateServerName { .. }));
    }

    // ── In-memory transport for bridge discovery tests (#356) ──────────

    use crate::mcp::client::McpError;
    use crate::mcp::{McpToolDef, mcp_to_tool_def};
    use async_trait::async_trait;
    use roko_core::tool::{ToolCall, ToolContext};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct FakeTransport {
        responses: std::sync::Mutex<Vec<Result<crate::mcp::McpResponse, McpError>>>,
        requests: std::sync::Mutex<Vec<crate::mcp::McpRequest>>,
        dropped: Arc<AtomicBool>,
    }

    impl FakeTransport {
        fn new(
            responses: Vec<Result<crate::mcp::McpResponse, McpError>>,
        ) -> (Arc<Self>, Arc<AtomicBool>) {
            let dropped = Arc::new(AtomicBool::new(false));
            let transport = Arc::new(Self {
                responses: std::sync::Mutex::new(responses),
                requests: std::sync::Mutex::new(Vec::new()),
                dropped: Arc::clone(&dropped),
            });
            (transport, dropped)
        }

        fn ok(responses: Vec<crate::mcp::McpResponse>) -> (Arc<Self>, Arc<AtomicBool>) {
            Self::new(responses.into_iter().map(Ok).collect())
        }

        fn take_requests(&self) -> Vec<crate::mcp::McpRequest> {
            self.requests.lock().unwrap().drain(..).collect()
        }
    }

    impl Drop for FakeTransport {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl Transport for FakeTransport {
        async fn roundtrip(
            &self,
            request: &crate::mcp::McpRequest,
        ) -> Result<crate::mcp::McpResponse, McpError> {
            self.requests.lock().unwrap().push(request.clone());
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(McpError::Transport("no more canned responses".into()));
            }
            responses.remove(0)
        }
    }

    struct HangingTransport;

    #[async_trait]
    impl Transport for HangingTransport {
        async fn roundtrip(
            &self,
            _request: &crate::mcp::McpRequest,
        ) -> Result<crate::mcp::McpResponse, McpError> {
            std::future::pending().await
        }
    }

    fn ok_resp(id: u64, result: serde_json::Value) -> crate::mcp::McpResponse {
        crate::mcp::McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn err_resp(id: u64, code: i64, msg: &str) -> crate::mcp::McpResponse {
        crate::mcp::McpResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(crate::mcp::client::JsonRpcError {
                code,
                message: msg.to_string(),
                data: None,
            }),
            id,
        }
    }

    fn fake_mcp_tool(name: &str) -> McpToolDef {
        McpToolDef {
            name: name.to_string(),
            description: Some(format!("Tool: {name}")),
            input_schema: Some(serde_json::json!({"type": "object"})),
            annotations: None,
        }
    }

    #[tokio::test]
    async fn runtime_rejects_http_transport() {
        let error = discover_mcp_runtime(&McpConfig {
            servers: vec![McpServerConfig {
                name: "remote".to_string(),
                transport: McpTransportConfig::Http,
                endpoint: Some("https://example.com/mcp".to_string()),
                ..Default::default()
            }],
        })
        .await
        .expect_err("http transport");
        assert!(matches!(error, McpBridgeError::UnsupportedTransport { .. }));
    }

    #[tokio::test]
    async fn runtime_empty_config_succeeds() {
        let runtime = discover_mcp_runtime(&McpConfig { servers: vec![] })
            .await
            .expect("empty config");
        assert_eq!(runtime.server_count(), 0);
        assert!(runtime.tools().is_empty());
        assert!(runtime.lifecycle_state().is_empty());
    }

    #[tokio::test]
    async fn runtime_rejects_whitespace_only_name() {
        let error = discover_mcp_runtime(&McpConfig {
            servers: vec![McpServerConfig {
                name: "   ".to_string(),
                ..Default::default()
            }],
        })
        .await
        .expect_err("whitespace name");
        assert!(matches!(error, McpBridgeError::InvalidServerName { .. }));
    }

    #[test]
    fn runtime_from_clients_two_servers() {
        let (t1, _) = FakeTransport::ok(vec![]);
        let (t2, _) = FakeTransport::ok(vec![]);
        let tools = vec![
            mcp_to_tool_def(&fake_mcp_tool("read_file"), "fs"),
            mcp_to_tool_def(&fake_mcp_tool("write_file"), "fs"),
            mcp_to_tool_def(&fake_mcp_tool("status"), "git"),
        ];
        let clients: HashMap<String, McpRuntimeClient> = HashMap::from([
            (
                "fs".to_string(),
                Arc::new(McpClient::new(t1 as McpRuntimeTransport)),
            ),
            (
                "git".to_string(),
                Arc::new(McpClient::new(t2 as McpRuntimeTransport)),
            ),
        ]);
        let runtime = McpRuntime::from_clients(tools, clients);
        assert_eq!(runtime.server_count(), 2);
        assert_eq!(runtime.tools().len(), 3);
        assert_eq!(runtime.tools()[0].name, "fs.read_file");
        assert_eq!(runtime.tools()[2].name, "git.status");
    }

    #[test]
    fn runtime_from_clients_with_lifecycle_state() {
        let (t, _) = FakeTransport::ok(vec![]);
        let tools = vec![mcp_to_tool_def(&fake_mcp_tool("echo"), "srv")];
        let clients = HashMap::from([(
            "srv".to_string(),
            Arc::new(McpClient::new(t as McpRuntimeTransport)),
        )]);
        let lifecycle = vec![McpLifecycleState {
            server_name: "srv".to_string(),
            last_health_check: Some(Instant::now()),
            last_error: None,
            negotiated_capabilities: Some(serde_json::json!({"tools": {}})),
            available_tools: vec!["echo".to_string()],
        }];
        let runtime = McpRuntime::from_clients_with_lifecycle(tools, clients, lifecycle);
        assert_eq!(runtime.lifecycle_state().len(), 1);
        assert_eq!(runtime.lifecycle_state()[0].server_name, "srv");
        assert!(runtime.lifecycle_state()[0].last_health_check.is_some());
    }

    #[test]
    fn runtime_dedup_last_writer_wins() {
        let all = vec![
            (
                "a".to_string(),
                vec![mcp_to_tool_def(&fake_mcp_tool("search"), "shared")],
            ),
            (
                "b".to_string(),
                vec![mcp_to_tool_def(
                    &McpToolDef {
                        name: "search".into(),
                        description: Some("v2".into()),
                        input_schema: None,
                        annotations: None,
                    },
                    "shared",
                )],
            ),
        ];
        let deduped = dedup_tools(all);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].description, "v2");
    }

    #[test]
    fn runtime_detects_unexecutable_tools() {
        let (t, _) = FakeTransport::ok(vec![]);
        let bad = mcp_to_tool_def(&fake_mcp_tool("read"), "missing_server");
        let builtin = roko_core::tool::ToolDef::new(
            "builtin.echo",
            "echo",
            roko_core::tool::ToolCategory::Meta,
            roko_core::tool::ToolPermission::read_only(),
        );
        let mut empty_srv = mcp_to_tool_def(&fake_mcp_tool("empty_read"), "valid");
        empty_srv.source = roko_core::tool::ToolSource::Mcp { server: "".into() };
        let mut no_suffix = mcp_to_tool_def(&fake_mcp_tool("read"), "valid");
        no_suffix.name = "valid.".to_string();

        let tools = vec![
            mcp_to_tool_def(&fake_mcp_tool("read"), "valid"),
            bad.clone(),
            builtin.clone(),
            empty_srv.clone(),
            no_suffix.clone(),
        ];
        let clients = HashMap::from([(
            "valid".to_string(),
            Arc::new(McpClient::new(t as McpRuntimeTransport)),
        )]);
        let runtime = McpRuntime::from_clients(tools, clients);
        let un = runtime.unexecutable_tools();
        assert!(!un.contains(&"valid.read".to_string()));
        assert!(un.contains(&bad.name));
        assert!(un.contains(&builtin.name));
        assert!(un.contains(&empty_srv.name));
        assert!(un.contains(&no_suffix.name));
    }

    #[test]
    fn runtime_no_unexecutable_when_all_valid() {
        let (t, _) = FakeTransport::ok(vec![]);
        let tools = vec![mcp_to_tool_def(&fake_mcp_tool("status"), "git")];
        let clients = HashMap::from([(
            "git".to_string(),
            Arc::new(McpClient::new(t as McpRuntimeTransport)),
        )]);
        assert!(
            McpRuntime::from_clients(tools, clients)
                .unexecutable_tools()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn runtime_resolver_routes_call_to_correct_client() {
        let (transport, _) = FakeTransport::ok(vec![ok_resp(
            1,
            serde_json::json!({
                "content": [{"type": "text", "text": "file data"}], "isError": false
            }),
        )]);
        let tref = Arc::clone(&transport);
        let tools = vec![mcp_to_tool_def(&fake_mcp_tool("read_file"), "fs")];
        let clients = HashMap::from([(
            "fs".to_string(),
            Arc::new(McpClient::new(transport as McpRuntimeTransport)),
        )]);
        let runtime = McpRuntime::from_clients(tools, clients);
        let resolver = runtime.resolver(Arc::new(|_: &str| None));
        let handler = resolver.resolve("fs.read_file").expect("handler");
        let result = handler
            .execute(
                ToolCall::new("c1", "fs.read_file", serde_json::json!({"path": "/tmp"})),
                &ToolContext::testing("/tmp/bridge-test"),
            )
            .await;
        assert_eq!(result, roko_core::tool::ToolResult::text("file data"));
        let reqs = tref.take_requests();
        assert_eq!(reqs[0].method, "tools/call");
        assert_eq!(reqs[0].params["name"], "read_file");
    }

    #[test]
    fn runtime_resolver_returns_none_for_missing_tool() {
        let (t, _) = FakeTransport::ok(vec![]);
        let tools = vec![mcp_to_tool_def(&fake_mcp_tool("echo"), "srv")];
        let clients = HashMap::from([(
            "srv".to_string(),
            Arc::new(McpClient::new(t as McpRuntimeTransport)),
        )]);
        let resolver = McpRuntime::from_clients(tools, clients).resolver(Arc::new(|_: &str| None));
        assert!(resolver.resolve("unknown.tool").is_none());
        assert!(resolver.resolve("unprefixed").is_none());
    }

    #[tokio::test]
    async fn handler_call_tool_error_flag() {
        let (transport, _) = FakeTransport::ok(vec![ok_resp(
            1,
            serde_json::json!({
                "content": [{"type": "text", "text": "denied"}], "isError": true
            }),
        )]);
        let tools = vec![mcp_to_tool_def(&fake_mcp_tool("write_file"), "fs")];
        let clients = HashMap::from([(
            "fs".to_string(),
            Arc::new(McpClient::new(transport as McpRuntimeTransport)),
        )]);
        let resolver = McpRuntime::from_clients(tools, clients).resolver(Arc::new(|_: &str| None));
        let result = resolver
            .resolve("fs.write_file")
            .unwrap()
            .execute(
                ToolCall::new("c1", "fs.write_file", serde_json::json!({})),
                &ToolContext::testing("/tmp/bridge-err"),
            )
            .await;
        assert!(matches!(result, roko_core::tool::ToolResult::Err(_)));
    }

    #[test]
    fn transport_dropped_when_runtime_is_dropped() {
        let (transport, dropped) = FakeTransport::ok(vec![]);
        assert!(!dropped.load(Ordering::SeqCst));
        {
            let tools = vec![mcp_to_tool_def(&fake_mcp_tool("echo"), "srv")];
            let clients = HashMap::from([(
                "srv".to_string(),
                Arc::new(McpClient::new(transport as McpRuntimeTransport)),
            )]);
            let _rt = McpRuntime::from_clients(tools, clients);
            assert!(!dropped.load(Ordering::SeqCst));
        }
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn client_initialize_failure_propagates() {
        let (t, _) = FakeTransport::new(vec![Ok(err_resp(1, -32600, "init refused"))]);
        let err = McpClient::new(t as McpRuntimeTransport)
            .initialize()
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::Server { code: -32600, .. }));
    }

    #[tokio::test]
    async fn client_list_tools_failure_propagates() {
        let (t, _) = FakeTransport::new(vec![Ok(err_resp(1, -32601, "not found"))]);
        let err = McpClient::new(t as McpRuntimeTransport)
            .list_tools()
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::Server { code: -32601, .. }));
    }

    #[tokio::test]
    async fn client_transport_error_propagates() {
        let (t, _) = FakeTransport::new(vec![Err(McpError::Transport("lost".into()))]);
        let err = McpClient::new(t as McpRuntimeTransport)
            .initialize()
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::Transport(_)));
    }

    #[tokio::test(start_paused = true)]
    async fn initialize_timeout_fires() {
        let t: McpRuntimeTransport = Arc::new(HangingTransport);
        assert!(
            timeout(MCP_DISCOVERY_TIMEOUT, McpClient::new(t).initialize())
                .await
                .is_err()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn list_tools_timeout_fires() {
        let t: McpRuntimeTransport = Arc::new(HangingTransport);
        assert!(
            timeout(MCP_DISCOVERY_TIMEOUT, McpClient::new(t).list_tools())
                .await
                .is_err()
        );
    }

    #[test]
    fn multi_server_discovery_simulation() {
        let (t1, _) = FakeTransport::ok(vec![]);
        let (t2, _) = FakeTransport::ok(vec![]);
        let all = vec![
            (
                "fs".to_string(),
                vec![
                    mcp_to_tool_def(&fake_mcp_tool("read_file"), "fs"),
                    mcp_to_tool_def(&fake_mcp_tool("write_file"), "fs"),
                ],
            ),
            (
                "git".to_string(),
                vec![
                    mcp_to_tool_def(&fake_mcp_tool("status"), "git"),
                    mcp_to_tool_def(&fake_mcp_tool("diff"), "git"),
                ],
            ),
        ];
        let deduped = dedup_tools(all);
        let clients: HashMap<String, McpRuntimeClient> = HashMap::from([
            (
                "fs".to_string(),
                Arc::new(McpClient::new(t1 as McpRuntimeTransport)),
            ),
            (
                "git".to_string(),
                Arc::new(McpClient::new(t2 as McpRuntimeTransport)),
            ),
        ]);
        let lifecycle = vec![
            McpLifecycleState {
                server_name: "fs".into(),
                last_health_check: Some(Instant::now()),
                last_error: None,
                negotiated_capabilities: Some(serde_json::json!({})),
                available_tools: vec!["read_file".into(), "write_file".into()],
            },
            McpLifecycleState {
                server_name: "git".into(),
                last_health_check: Some(Instant::now()),
                last_error: None,
                negotiated_capabilities: Some(serde_json::json!({})),
                available_tools: vec!["status".into(), "diff".into()],
            },
        ];
        let runtime = McpRuntime::from_clients_with_lifecycle(deduped, clients, lifecycle);
        assert_eq!(runtime.server_count(), 2);
        assert_eq!(runtime.tools().len(), 4);
        assert!(runtime.unexecutable_tools().is_empty());
        assert_eq!(runtime.lifecycle_state().len(), 2);
    }

    #[test]
    fn runtime_debug_does_not_panic() {
        let (t, _) = FakeTransport::ok(vec![]);
        let tools = vec![mcp_to_tool_def(&fake_mcp_tool("echo"), "srv")];
        let clients = HashMap::from([(
            "srv".to_string(),
            Arc::new(McpClient::new(t as McpRuntimeTransport)),
        )]);
        let debug = format!("{:?}", McpRuntime::from_clients(tools, clients));
        assert!(debug.contains("McpRuntime"));
    }

    #[test]
    fn runtime_with_owner_retains_data() {
        let (t, _) = FakeTransport::ok(vec![]);
        let counter = Arc::new(AtomicUsize::new(0));
        let tools = vec![mcp_to_tool_def(&fake_mcp_tool("echo"), "srv")];
        let clients = HashMap::from([(
            "srv".to_string(),
            Arc::new(McpClient::new(t as McpRuntimeTransport)),
        )]);
        let runtime = McpRuntime::from_clients(tools, clients)
            .with_owner(Arc::clone(&counter) as Arc<dyn Send + Sync>);
        assert!(Arc::strong_count(&counter) >= 2);
        assert!(format!("{runtime:?}").contains("has_runtime_owner: true"));
    }

    // ── test_mcp_server tests (#356) ─────────────────────────────────────

    #[tokio::test]
    async fn test_mcp_server_spawn_failure_returns_failed_report() {
        let server = McpServerConfig {
            name: "broken".to_string(),
            command: "__roko_nonexistent_binary_xyz__".to_string(),
            ..Default::default()
        };
        let report = test_mcp_server(&server, PathBuf::from("/tmp/test.mcp.json"), None).await;
        assert_eq!(report.status, McpTestStatus::Failed);
        assert_eq!(report.server, "broken");
        assert!(!report.command_available);
        assert_eq!(report.tool_count, 0);
        assert!(report.tool_names.is_empty());
        assert!(report.protocol_version.is_none());
        assert!(!report.stages.is_empty());
        assert!(!report.stages[0].success);
        assert!(report.stages[0].error.is_some());
    }

    #[test]
    fn test_report_serde_roundtrip() {
        let report = McpTestReport {
            config_path: PathBuf::from("/home/user/.mcp.json"),
            server: "test-server".to_string(),
            command_available: true,
            stages: vec![
                McpTestStageResult {
                    stage: "initialize".to_string(),
                    success: true,
                    latency_ms: Some(42),
                    error: None,
                },
                McpTestStageResult {
                    stage: "tools_list".to_string(),
                    success: true,
                    latency_ms: Some(15),
                    error: None,
                },
            ],
            protocol_version: Some("2025-11-25".to_string()),
            tool_count: 3,
            tool_names: vec!["read_file".into(), "write_file".into(), "search".into()],
            stderr_summary: None,
            status: McpTestStatus::Ok,
        };

        let json = serde_json::to_string(&report).expect("serialize");
        let parsed: McpTestReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.server, "test-server");
        assert_eq!(parsed.status, McpTestStatus::Ok);
        assert_eq!(parsed.tool_count, 3);
        assert_eq!(parsed.stages.len(), 2);
        assert!(parsed.stages[0].success);
        assert_eq!(parsed.protocol_version.as_deref(), Some("2025-11-25"));
    }

    #[test]
    fn test_report_serde_skips_none_fields() {
        let report = McpTestReport {
            config_path: PathBuf::from("/test"),
            server: "s".to_string(),
            command_available: false,
            stages: vec![McpTestStageResult {
                stage: "spawn".to_string(),
                success: false,
                latency_ms: None,
                error: Some("not found".to_string()),
            }],
            protocol_version: None,
            tool_count: 0,
            tool_names: vec![],
            stderr_summary: None,
            status: McpTestStatus::Failed,
        };

        let json = serde_json::to_string(&report).expect("serialize");
        // Optional None fields should be skipped.
        assert!(!json.contains("\"protocol_version\""));
        assert!(!json.contains("\"stderr_summary\""));
        assert!(!json.contains("\"latency_ms\""));
    }

    #[test]
    fn test_status_display() {
        assert_eq!(McpTestStatus::Ok.to_string(), "ok");
        assert_eq!(McpTestStatus::Failed.to_string(), "failed");
    }
}
