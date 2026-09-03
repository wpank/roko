//! MCP discovery bridge for HTTP-backed tool loops.
//!
//! Claude CLI forwards MCP config directly to the subprocess via
//! `--mcp-config`. HTTP backends cannot do that, so they must discover MCP
//! tools up front, convert them into canonical [`ToolDef`] values, and let the
//! normal translator render them into backend-specific function definitions.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use roko_core::tool::{ToolDef, ToolSource};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, timeout};

use super::{
    McpClient, McpConfig, McpHandlerResolver, McpTransportConfig, StdioTransport, Transport,
    dedup_tools, is_command_on_path, mcp_to_tool_def,
};
use crate::dispatcher::HandlerResolver;
use crate::mcp::client::{McpError, McpRequest, McpResponse};

const MCP_DISCOVERY_TIMEOUT: Duration =
    Duration::from_secs(roko_core::defaults::DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);

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

// ── Diagnostic test report ──────────────────────────────────────────────

/// Per-stage result included in [`McpTestReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStageResult {
    /// Whether the stage succeeded.
    pub ok: bool,
    /// Wall-clock latency in milliseconds.
    pub latency_ms: f64,
    /// Error message if the stage failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Report produced by [`test_mcp_server`].
///
/// Designed for both human-readable rendering and JSON serialization.
/// Secrets and raw stderr are redacted before inclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTestReport {
    /// Path to the MCP config file that was consulted.
    pub config_path: String,
    /// Server name that was tested.
    pub server: String,
    /// Whether the server's command binary was found on PATH.
    pub command_available: bool,
    /// Result of the `initialize` handshake (absent if skipped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize: Option<McpStageResult>,
    /// Result of `tools/list` (absent if skipped due to earlier failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools_list: Option<McpStageResult>,
    /// MCP protocol version returned by the server, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    /// Number of tools discovered.
    pub tool_count: usize,
    /// Tool names (descriptions are excluded for safety).
    pub tool_names: Vec<String>,
    /// Bounded, redacted stderr summary (at most 4 KiB before redaction).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_summary: Option<String>,
    /// Overall result: `"ok"` or `"failed"`.
    pub status: String,
    /// Name of the first stage that failed, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_stage: Option<String>,
}

/// Maximum stderr bytes captured from the diagnostic child.
const MAX_STDERR_BYTES: usize = 4096;

/// Redact values that look like secrets from an MCP server's stderr output.
///
/// Strips:
/// 1. Any configured env values that are non-empty and not env-var references.
/// 2. Assignments matching `KEY=value` where KEY matches sensitive patterns.
///
/// Never returns raw env values or credentials.
pub fn redact_stderr(raw: &str, env: &HashMap<String, String>) -> String {
    let mut redacted = raw.to_string();

    // Redact configured env values (non-reference, non-empty).
    for value in env.values() {
        if !value.is_empty() && !value.starts_with('$') && value.len() >= 4 {
            redacted = redacted.replace(value, "[REDACTED]");
        }
    }

    // Redact KEY=value patterns for sensitive keys.
    let re_pattern = regex::Regex::new(
        r"(?i)(SECRET|TOKEN|KEY|PASSWORD|CREDENTIAL|AUTH|PRIVATE|API_KEY|APIKEY)\s*[=:]\s*\S+",
    )
    .expect("valid regex");
    redacted = re_pattern
        .replace_all(&redacted, |caps: &regex::Captures<'_>| {
            let full = caps.get(0).map_or("", |m| m.as_str());
            if let Some(eq_pos) = full.find(['=', ':']) {
                let (key_part, _) = full.split_at(eq_pos + 1);
                format!("{key_part}[REDACTED]")
            } else {
                "[REDACTED]".to_string()
            }
        })
        .to_string();

    redacted
}

/// Test a single MCP server end-to-end and return a diagnostic report.
///
/// Performs the following stages:
/// 1. Resolve the named server from the config.
/// 2. Check that the command binary exists on PATH.
/// 3. Spawn the server process (capturing stderr).
/// 4. Send `initialize` with the given per-stage timeout.
/// 5. Send `tools/list` with the same per-stage timeout.
/// 6. Cleanly terminate the child process.
pub async fn test_mcp_server(
    config: &McpConfig,
    name: &str,
    stage_timeout: Duration,
    config_path: &Path,
) -> McpTestReport {
    let config_path_str = config_path.display().to_string();
    let fail = |stage: &str| McpTestReport {
        config_path: config_path_str.clone(),
        server: name.to_string(),
        command_available: false,
        initialize: None,
        tools_list: None,
        protocol_version: None,
        tool_count: 0,
        tool_names: Vec::new(),
        stderr_summary: None,
        status: "failed".to_string(),
        failed_stage: Some(stage.to_string()),
    };

    let server = match config.servers.iter().find(|s| s.name == name) {
        Some(s) => s,
        None => return fail("resolve"),
    };

    let command_available = is_command_on_path(&server.command);
    if !command_available {
        return fail("command");
    }

    let transport = match DiagnosticTransport::spawn(&server.command, &server.args, &server.env) {
        Ok(t) => t,
        Err(e) => {
            return McpTestReport {
                config_path: config_path_str,
                server: name.to_string(),
                command_available: true,
                initialize: Some(McpStageResult {
                    ok: false,
                    latency_ms: 0.0,
                    error: Some(format!("spawn failed: {e}")),
                }),
                tools_list: None,
                protocol_version: None,
                tool_count: 0,
                tool_names: Vec::new(),
                stderr_summary: None,
                status: "failed".to_string(),
                failed_stage: Some("spawn".to_string()),
            };
        }
    };

    let transport = Arc::new(transport);
    let client = McpClient::new(Arc::clone(&transport));

    // Stage 1: initialize
    let init_start = Instant::now();
    let init_result = timeout(stage_timeout, client.initialize()).await;
    let init_elapsed = init_start.elapsed();

    let (init_stage, init_caps) = match init_result {
        Ok(Ok(caps)) => (
            McpStageResult {
                ok: true,
                latency_ms: init_elapsed.as_secs_f64() * 1000.0,
                error: None,
            },
            Some(caps),
        ),
        Ok(Err(e)) => {
            let stderr = transport.drain_stderr().await;
            let redacted = redact_stderr(&stderr, &server.env);
            transport.shutdown().await;
            return McpTestReport {
                config_path: config_path_str,
                server: name.to_string(),
                command_available: true,
                initialize: Some(McpStageResult {
                    ok: false,
                    latency_ms: init_elapsed.as_secs_f64() * 1000.0,
                    error: Some(e.to_string()),
                }),
                tools_list: None,
                protocol_version: None,
                tool_count: 0,
                tool_names: Vec::new(),
                stderr_summary: Some(redacted),
                status: "failed".to_string(),
                failed_stage: Some("initialize".to_string()),
            };
        }
        Err(_) => {
            let stderr = transport.drain_stderr().await;
            let redacted = redact_stderr(&stderr, &server.env);
            transport.shutdown().await;
            return McpTestReport {
                config_path: config_path_str,
                server: name.to_string(),
                command_available: true,
                initialize: Some(McpStageResult {
                    ok: false,
                    latency_ms: init_elapsed.as_secs_f64() * 1000.0,
                    error: Some(format!("initialize timed out after {}s", stage_timeout.as_secs())),
                }),
                tools_list: None,
                protocol_version: None,
                tool_count: 0,
                tool_names: Vec::new(),
                stderr_summary: Some(redacted),
                status: "failed".to_string(),
                failed_stage: Some("initialize".to_string()),
            };
        }
    };

    let protocol_version = init_caps
        .as_ref()
        .and_then(|c| c.get("protocolVersion"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Stage 2: tools/list
    let tl_start = Instant::now();
    let tl_result = timeout(stage_timeout, client.list_tools()).await;
    let tl_elapsed = tl_start.elapsed();

    let (tl_stage, tools) = match tl_result {
        Ok(Ok(tools)) => (
            McpStageResult {
                ok: true,
                latency_ms: tl_elapsed.as_secs_f64() * 1000.0,
                error: None,
            },
            tools,
        ),
        Ok(Err(e)) => {
            let stderr = transport.drain_stderr().await;
            let redacted = redact_stderr(&stderr, &server.env);
            transport.shutdown().await;
            return McpTestReport {
                config_path: config_path_str,
                server: name.to_string(),
                command_available: true,
                initialize: Some(init_stage),
                tools_list: Some(McpStageResult {
                    ok: false,
                    latency_ms: tl_elapsed.as_secs_f64() * 1000.0,
                    error: Some(e.to_string()),
                }),
                protocol_version,
                tool_count: 0,
                tool_names: Vec::new(),
                stderr_summary: Some(redacted),
                status: "failed".to_string(),
                failed_stage: Some("tools_list".to_string()),
            };
        }
        Err(_) => {
            let stderr = transport.drain_stderr().await;
            let redacted = redact_stderr(&stderr, &server.env);
            transport.shutdown().await;
            return McpTestReport {
                config_path: config_path_str,
                server: name.to_string(),
                command_available: true,
                initialize: Some(init_stage),
                tools_list: Some(McpStageResult {
                    ok: false,
                    latency_ms: tl_elapsed.as_secs_f64() * 1000.0,
                    error: Some(format!("tools/list timed out after {}s", stage_timeout.as_secs())),
                }),
                protocol_version,
                tool_count: 0,
                tool_names: Vec::new(),
                stderr_summary: Some(redacted),
                status: "failed".to_string(),
                failed_stage: Some("tools_list".to_string()),
            };
        }
    };

    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    let tool_count = tools.len();
    let stderr = transport.drain_stderr().await;
    let redacted = redact_stderr(&stderr, &server.env);
    transport.shutdown().await;

    McpTestReport {
        config_path: config_path_str,
        server: name.to_string(),
        command_available: true,
        initialize: Some(init_stage),
        tools_list: Some(tl_stage),
        protocol_version,
        tool_count,
        tool_names,
        stderr_summary: if redacted.is_empty() { None } else { Some(redacted) },
        status: "ok".to_string(),
        failed_stage: None,
    }
}

// ── Diagnostic transport ────────────────────────────────────────────────

struct DiagnosticTransport {
    stdin: tokio::sync::Mutex<tokio::io::BufWriter<tokio::process::ChildStdin>>,
    stdout: tokio::sync::Mutex<tokio::io::BufReader<tokio::process::ChildStdout>>,
    stderr: tokio::sync::Mutex<tokio::process::ChildStderr>,
    child: tokio::sync::Mutex<tokio::process::Child>,
}

impl DiagnosticTransport {
    fn spawn(command: &str, args: &[String], env: &HashMap<String, String>) -> Result<Self, McpError> {
        let resolved_env: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.clone(), resolve_env_value(v)))
            .collect();

        let mut child = tokio::process::Command::new(command)
            .args(args)
            .envs(resolved_env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| McpError::Transport(format!("failed to spawn {command}: {e}")))?;

        let stdin = child.stdin.take().ok_or_else(|| McpError::Transport("child stdin not available".into()))?;
        let stdout = child.stdout.take().ok_or_else(|| McpError::Transport("child stdout not available".into()))?;
        let stderr = child.stderr.take().ok_or_else(|| McpError::Transport("child stderr not available".into()))?;

        Ok(Self {
            stdin: tokio::sync::Mutex::new(tokio::io::BufWriter::new(stdin)),
            stdout: tokio::sync::Mutex::new(tokio::io::BufReader::new(stdout)),
            stderr: tokio::sync::Mutex::new(stderr),
            child: tokio::sync::Mutex::new(child),
        })
    }

    async fn drain_stderr(&self) -> String {
        use tokio::io::AsyncReadExt;
        let mut stderr = self.stderr.lock().await;
        let mut buf = vec![0u8; MAX_STDERR_BYTES];
        let read_result = timeout(Duration::from_millis(200), async {
            let mut total = 0usize;
            loop {
                if total >= MAX_STDERR_BYTES { break total; }
                match stderr.read(&mut buf[total..]).await {
                    Ok(0) => break total,
                    Ok(n) => total += n,
                    Err(_) => break total,
                }
            }
        }).await;
        let n = read_result.unwrap_or(0);
        String::from_utf8_lossy(&buf[..n]).to_string()
    }

    async fn shutdown(&self) {
        let mut child = self.child.lock().await;
        let wait_result = timeout(Duration::from_secs(2), child.wait()).await;
        if wait_result.is_err() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}

#[async_trait::async_trait]
impl Transport for DiagnosticTransport {
    async fn roundtrip(&self, request: &McpRequest) -> Result<McpResponse, McpError> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

        let mut line = serde_json::to_string(request)?;
        line.push('\n');

        let write_result = timeout(Duration::from_secs(5), async {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(line.as_bytes()).await?;
            stdin.flush().await?;
            Ok::<(), std::io::Error>(())
        }).await;

        match write_result {
            Err(_) => return Err(McpError::Transport("MCP server stdin write timed out after 5s".into())),
            Ok(Err(e)) => return Err(McpError::Transport(format!("write to stdin: {e}"))),
            Ok(Ok(())) => {}
        }

        let read_result = timeout(Duration::from_secs(30), async {
            let mut stdout = self.stdout.lock().await;
            let mut response_line = String::new();
            stdout.read_line(&mut response_line).await?;
            Ok::<String, std::io::Error>(response_line)
        }).await;

        let response_line = match read_result {
            Err(_) => return Err(McpError::Transport("MCP server response timed out after 30s".into())),
            Ok(Err(e)) => return Err(McpError::Transport(format!("read from stdout: {e}"))),
            Ok(Ok(line)) => line,
        };

        if response_line.is_empty() {
            return Err(McpError::Transport("child process closed stdout (EOF)".into()));
        }

        let resp: McpResponse = serde_json::from_str(&response_line)?;
        Ok(resp)
    }
}

fn resolve_env_value(value: &str) -> String {
    let Some(name) = value.strip_prefix("${").and_then(|rest| rest.strip_suffix('}')) else {
        return value.to_string();
    };
    std::env::var(name).unwrap_or_default()
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

    #[test]
    fn mcp_test_report_roundtrips_through_json() {
        let report = McpTestReport {
            config_path: "/tmp/mcp.json".to_string(),
            server: "test-server".to_string(),
            command_available: true,
            initialize: Some(McpStageResult { ok: true, latency_ms: 42.5, error: None }),
            tools_list: Some(McpStageResult { ok: true, latency_ms: 12.3, error: None }),
            protocol_version: Some("2025-11-25".to_string()),
            tool_count: 2,
            tool_names: vec!["read_file".to_string(), "search".to_string()],
            stderr_summary: None,
            status: "ok".to_string(),
            failed_stage: None,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        let parsed: McpTestReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.server, "test-server");
        assert_eq!(parsed.status, "ok");
        assert_eq!(parsed.tool_count, 2);
        assert!(parsed.initialize.as_ref().unwrap().ok);
        assert!(!json.contains("stderr_summary"));
        assert!(!json.contains("failed_stage"));
    }

    #[test]
    fn mcp_test_report_failed_serializes_stage() {
        let report = McpTestReport {
            config_path: "/tmp/mcp.json".to_string(),
            server: "broken".to_string(),
            command_available: false,
            initialize: None, tools_list: None, protocol_version: None,
            tool_count: 0, tool_names: Vec::new(), stderr_summary: None,
            status: "failed".to_string(),
            failed_stage: Some("command".to_string()),
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"failed_stage\":\"command\""));
        assert!(json.contains("\"status\":\"failed\""));
    }

    #[test]
    fn mcp_test_redact_stderr_strips_env_values() {
        let mut env = HashMap::new();
        env.insert("API_KEY".to_string(), "sk-secret-abc123xyz".to_string());
        let raw = "error: failed to auth with sk-secret-abc123xyz on port 8080";
        let redacted = redact_stderr(raw, &env);
        assert!(!redacted.contains("sk-secret-abc123xyz"));
        assert!(redacted.contains("[REDACTED]"));
        assert!(redacted.contains("port 8080"));
    }

    #[test]
    fn mcp_test_redact_stderr_strips_key_value_assignments() {
        let env = HashMap::new();
        let raw = "debug: TOKEN=ghp_verysecrettoken123 loaded";
        let redacted = redact_stderr(raw, &env);
        assert!(!redacted.contains("ghp_verysecrettoken123"));
        assert!(redacted.contains("TOKEN="));
    }

    #[test]
    fn mcp_test_redact_stderr_ignores_short_env_values() {
        let mut env = HashMap::new();
        env.insert("SHORT".to_string(), "ab".to_string());
        let raw = "ab is everywhere: ab ab ab";
        let redacted = redact_stderr(raw, &env);
        assert_eq!(redacted, raw);
    }

    #[test]
    fn mcp_test_redact_stderr_preserves_safe_output() {
        let env = HashMap::new();
        let raw = "MCP server started on port 3000";
        assert_eq!(redact_stderr(raw, &env), raw);
    }

    #[tokio::test]
    async fn mcp_test_server_not_found_in_config() {
        let config = McpConfig {
            servers: vec![McpServerConfig { name: "other".to_string(), ..Default::default() }],
        };
        let report = test_mcp_server(&config, "nonexistent", Duration::from_secs(5), std::path::Path::new("/tmp/mcp.json")).await;
        assert_eq!(report.status, "failed");
        assert_eq!(report.failed_stage.as_deref(), Some("resolve"));
    }

    #[tokio::test]
    async fn mcp_test_server_command_not_found() {
        let config = McpConfig {
            servers: vec![McpServerConfig {
                name: "bad-cmd".to_string(),
                command: "__roko_nonexistent_binary_xyz_9999__".to_string(),
                ..Default::default()
            }],
        };
        let report = test_mcp_server(&config, "bad-cmd", Duration::from_secs(5), std::path::Path::new("/tmp/mcp.json")).await;
        assert_eq!(report.status, "failed");
        assert_eq!(report.failed_stage.as_deref(), Some("command"));
        assert!(!report.command_available);
    }
}
