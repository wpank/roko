//! MCP discovery bridge for HTTP-backed tool loops.
//!
//! Claude CLI forwards MCP config directly to the subprocess via
//! `--mcp-config`. HTTP backends cannot do that, so they must discover MCP
//! tools up front, convert them into canonical [`ToolDef`] values, and let the
//! normal translator render them into backend-specific function definitions.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use roko_core::tool::{ToolDef, ToolSource};
use tokio::time::{Duration, timeout};

use super::{
    McpClient, McpConfig, McpHandlerResolver, McpTransportConfig, StdioTransport, Transport,
    dedup_tools, mcp_to_tool_def,
};
use crate::dispatcher::HandlerResolver;
use crate::mcp::client::McpError;

const MCP_DISCOVERY_TIMEOUT: Duration =
    Duration::from_secs(roko_core::defaults::DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);

/// Type-erased transport retained by an [`McpRuntime`].
pub type McpRuntimeTransport = Arc<dyn Transport>;
/// Initialized client retained for one configured MCP server.
pub type McpRuntimeClient = Arc<McpClient<McpRuntimeTransport>>;

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

        match timeout(MCP_DISCOVERY_TIMEOUT, client.initialize()).await {
            Ok(Ok(_)) => {}
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
        }

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

        let defs = mcp_tools
            .iter()
            .map(|tool| mcp_to_tool_def(tool, &server.name))
            .collect();
        all_server_tools.push((server.name.clone(), defs));
        clients.insert(server.name.clone(), client);
    }

    Ok(McpRuntime::from_clients(
        dedup_tools(all_server_tools),
        clients,
    ))
}

const MCP_TOOL_SEPARATOR: char = '.';

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
}
