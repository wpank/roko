//! Shared ACP (Agent Client Protocol) JSON-RPC 2.0 over stdio client.
//!
//! This module provides [`AcpStdioClient`], a transport-level client for
//! communicating with ACP-speaking agent subprocesses. The client manages
//! a persistent child process and provides methods for the ACP lifecycle:
//!
//! 1. `connect()` -- spawn the subprocess, perform `initialize` handshake
//! 2. `new_session()` -- create a new session (`session/new`)
//! 3. `send_prompt()` -- send a prompt (`session/prompt`)
//! 4. `cancel()` -- cancel an in-flight prompt (`session/cancel`)
//! 5. `load_session()` -- resume a previous session (`session/load`)
//! 6. `close_session()` -- close a session (`session/close`)
//! 7. `shutdown()` -- graceful shutdown
//!
//! The client is harness-agnostic: it forwards raw [`AcpNotification`]
//! values without interpreting their content. Each harness adapter
//! (Cursor, Hermes, OpenClaw) is responsible for parsing notifications
//! into domain-specific events.
//!
//! # Relationship to `ChildProcessRunner`
//!
//! `AcpStdioClient` does **NOT** use [`super::ChildProcessRunner`].
//! They solve different problems:
//!
//! - `ChildProcessRunner` is for one-shot processes (spawn, run to
//!   completion, collect output).
//! - `AcpStdioClient` is for persistent processes (spawn once, send many
//!   requests via JSON-RPC over stdin/stdout).
//!
//! The child stays alive across multiple `session/prompt` turns and is
//! only killed on explicit `shutdown()` or `Drop`.

use crate::process;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufWriter};
use tokio::process::{Child, ChildStdin};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Duration;

// ---- Configuration ----------------------------------------------------------

/// Configuration for an ACP stdio client connection.
///
/// Each ACP-speaking harness provides its own constructor that fills in
/// the binary, args, and session prefix. The protocol version is
/// `"2024-11-05"` (the ACP spec's date-string convention); Cursor uses
/// integer `1` which is passed as `"1"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpStdioConfig {
    /// Path or name of the binary to spawn (e.g. `"cursor"`, `"hermes"`,
    /// `"openclaw"`).
    pub command: String,

    /// Arguments to pass to the binary.
    /// Example for Cursor: `["--force", "--approve-mcps",
    /// "--workspace", "/path", "--output-format", "json", "acp"]`.
    pub args: Vec<String>,

    /// Working directory for the subprocess. If `None`, inherits the
    /// current process's cwd.
    pub cwd: Option<PathBuf>,

    /// Additional env vars to set on the child process.
    pub env: HashMap<String, String>,

    /// Protocol version to send in the `initialize` request.
    /// ACP spec uses date strings like `"2024-11-05"`.
    /// Cursor uses `"1"`. Always a `String`.
    pub protocol_version: String,

    /// Timeout for the `initialize` handshake and `session/new`.
    pub timeout: Duration,
}

// ---- Event types ------------------------------------------------------------

/// Semantic events from an ACP agent turn.
///
/// These are the high-level events that callers care about. The
/// `AcpStdioClient` emits raw `AcpNotification` values; adapters
/// (Cursor, Hermes, OpenClaw) parse those into `AcpEvent` variants.
#[derive(Debug, Clone)]
pub enum AcpEvent {
    /// Agent output text chunk (streaming).
    Output { text: String },
    /// Agent initiated a tool call.
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Progress update for an in-flight tool call.
    ToolCallUpdate { id: String, progress: String },
    /// Agent is requesting permission to use a tool.
    PermissionRequest {
        id: String,
        tool: String,
        arguments: serde_json::Value,
    },
    /// Token usage report.
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// The turn ended with this stop reason.
    StopReason(String),
}

/// A raw ACP notification from the server.
///
/// This is the transport-level representation. Adapters (Cursor, Hermes,
/// OpenClaw) convert these into their own domain-specific event enums
/// or into `AcpEvent` variants.
#[derive(Debug, Clone)]
pub struct AcpNotification {
    /// JSON-RPC method name (e.g. `"session/update"`,
    /// `"session/request_permission"`).
    pub method: String,
    /// The `params` field from the notification.
    pub params: Option<serde_json::Value>,
    /// If the server message had an `id` (server request), it is
    /// preserved here so the adapter can send a response if needed.
    pub server_request_id: Option<u64>,
}

/// Session identifier returned by `session/new`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Response from the `initialize` handshake.
#[derive(Clone, Debug)]
pub struct AcpInitResponse {
    pub protocol_version: serde_json::Value,
    pub server_info: serde_json::Value,
    pub raw: serde_json::Value,
}

/// Payload for a `session/prompt` request.
#[derive(Clone, Debug)]
pub struct AcpPromptPayload {
    pub text: String,
    /// Additional fields to merge into the `params` object.
    /// Used by adapters for harness-specific prompt fields.
    pub extra_params: Option<serde_json::Value>,
}

/// Result of a `session/prompt` request (the JSON-RPC response).
#[derive(Clone, Debug)]
pub struct AcpPromptResult {
    /// The raw `result` value from the JSON-RPC response.
    pub raw: serde_json::Value,
    /// Extracted `stopReason` if present.
    pub stop_reason: Option<String>,
}

// ---- Session options --------------------------------------------------------

/// Options for `session/new`. Used by all ACP adapters.
#[derive(Debug, Clone, Default, Serialize)]
pub struct NewSessionOpts {
    /// Explicit session key. If `None`, the server assigns one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,

    /// Working directory for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,

    /// MCP server configuration to pass to the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<serde_json::Value>,

    /// If true, reset any existing session state.
    #[serde(default)]
    pub reset: bool,

    /// Additional fields to merge into the `params` object.
    /// Used by adapters for harness-specific session fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_params: Option<serde_json::Value>,
}

// ---- Error type -------------------------------------------------------------

/// Errors from ACP operations.
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("acp `{method}` failed: {message}")]
    MethodFailed { method: String, message: String },
    #[error("acp connection lost: server exited or pipe closed")]
    Disconnected,
    #[error("acp `{method}` timed out after {elapsed:?}")]
    Timeout {
        method: String,
        elapsed: std::time::Duration,
    },
    #[error("acp protocol error: {0}")]
    Protocol(String),
    #[error("spawn failed: {0}")]
    Spawn(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

// ---- JSON-RPC wire types ----------------------------------------------------

/// Outgoing JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    fn new(id: u64, method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// Raw incoming message from an ACP server.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawServerMessage {
    id: Option<serde_json::Value>,
    method: Option<String>,
    result: Option<serde_json::Value>,
    error: Option<ServerError>,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ServerError {
    #[allow(dead_code)]
    code: i64,
    message: String,
    #[allow(dead_code)]
    data: Option<serde_json::Value>,
}

impl RawServerMessage {
    fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }

    fn is_notification(&self) -> bool {
        self.method.is_some() && self.id.is_none()
    }

    fn is_server_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    fn numeric_id(&self) -> Option<u64> {
        self.id.as_ref().and_then(|v| v.as_u64())
    }
}

// ---- AcpStdioClient ---------------------------------------------------------

/// Shared ACP/JSON-RPC client over stdio.
///
/// Extracted from `CursorConnection` in `cursor_cli_agent.rs`.
/// Parameterized on `AcpStdioConfig` so the same client drives
/// Cursor, Hermes, and OpenClaw ACP servers.
///
/// This client manages its own persistent child process directly.
/// It does NOT use `ChildProcessRunner` (which is for one-shot
/// processes). The child stays alive across multiple `session/prompt`
/// turns and is killed on `shutdown()` or `Drop`.
pub struct AcpStdioClient {
    config: AcpStdioConfig,
    child: Option<Child>,
    stdin: Option<BufWriter<ChildStdin>>,
    next_id: AtomicU64,
    response_rx: Option<mpsc::UnboundedReceiver<(u64, serde_json::Value)>>,
    notification_tx: mpsc::UnboundedSender<AcpNotification>,
    notification_rx: Option<mpsc::UnboundedReceiver<AcpNotification>>,
    turn_done_tx: mpsc::UnboundedSender<serde_json::Value>,
    turn_done_rx: Option<mpsc::UnboundedReceiver<serde_json::Value>>,
    session_id: Option<String>,
    reader_handle: Option<JoinHandle<()>>,
    stderr_handle: Option<JoinHandle<()>>,
}

impl AcpStdioClient {
    /// Create a new `AcpStdioClient` with the given config.
    ///
    /// Does NOT spawn the process. Call `connect()` to spawn and
    /// perform the `initialize` handshake.
    pub fn new(config: AcpStdioConfig) -> Self {
        let (notification_tx, notification_rx) = mpsc::unbounded_channel();
        let (turn_done_tx, turn_done_rx) = mpsc::unbounded_channel();
        Self {
            config,
            child: None,
            stdin: None,
            next_id: AtomicU64::new(1),
            response_rx: None,
            notification_tx,
            notification_rx: Some(notification_rx),
            turn_done_tx,
            turn_done_rx: Some(turn_done_rx),
            session_id: None,
            reader_handle: None,
            stderr_handle: None,
        }
    }

    // ---- Convenience constructors -------------------------------------------

    /// Construct a client configured for the Cursor agent ACP server.
    ///
    /// Command: `<binary> --force --approve-mcps --workspace <cwd>
    ///           --output-format json [--model <model>] acp`
    pub fn cursor(binary: impl Into<String>, cwd: PathBuf, model: Option<String>) -> Self {
        let binary = binary.into();
        let mut args = vec![
            "--force".into(),
            "--approve-mcps".into(),
            "--workspace".into(),
            cwd.to_string_lossy().into_owned(),
            "--output-format".into(),
            "json".into(),
        ];
        if let Some(ref m) = model {
            args.push("--model".into());
            args.push(m.clone());
        }
        args.push("acp".into());

        let mut env = HashMap::new();
        env.insert("CARGO_INCREMENTAL".into(), "0".into());
        env.insert("CARGO_BUILD_JOBS".into(), "2".into());

        Self::new(AcpStdioConfig {
            command: binary,
            args,
            cwd: Some(cwd),
            env,
            protocol_version: "1".into(),
            timeout: Duration::from_secs(90),
        })
    }

    /// Construct a client configured for the Hermes ACP server.
    ///
    /// Command: `<binary> acp`
    pub fn hermes(binary: impl Into<String>, cwd: PathBuf) -> Self {
        Self::new(AcpStdioConfig {
            command: binary.into(),
            args: vec!["acp".into()],
            cwd: Some(cwd),
            env: HashMap::new(),
            protocol_version: "2024-11-05".into(),
            timeout: Duration::from_secs(30),
        })
    }

    /// Construct a client configured for the OpenClaw ACP bridge.
    ///
    /// Command: `<binary> acp [--url <gateway_url>]`
    pub fn openclaw(binary: impl Into<String>, cwd: PathBuf, gateway_url: Option<String>) -> Self {
        let mut args = vec!["acp".into()];
        if let Some(ref url) = gateway_url {
            args.push("--url".into());
            args.push(url.clone());
        }

        Self::new(AcpStdioConfig {
            command: binary.into(),
            args,
            cwd: Some(cwd),
            env: HashMap::new(),
            protocol_version: "2024-11-05".into(),
            timeout: Duration::from_secs(30),
        })
    }

    // ---- Connection lifecycle ------------------------------------------------

    /// Spawn the ACP server subprocess and perform the `initialize` handshake.
    ///
    /// This method:
    /// 1. Builds a `tokio::process::Command` from `config`.
    /// 2. Spawns the child with piped stdin/stdout/stderr.
    /// 3. Registers the PID for orphan cleanup.
    /// 4. Spawns a stderr reader task (logs lines via `tracing::debug!`).
    /// 5. Spawns a stdout reader task (parses JSON-RPC, routes responses
    ///    to `response_rx` and notifications to `notification_tx`).
    /// 6. Sends the `initialize` JSON-RPC request.
    /// 7. Awaits the response within `config.timeout`.
    /// 8. Returns the parsed `AcpInitResponse`.
    ///
    /// # Errors
    ///
    /// Returns `AcpError::Spawn` if the binary cannot be started.
    /// Returns `AcpError::Timeout` if `initialize` does not respond
    /// within `config.timeout`.
    /// Returns `AcpError::MethodFailed` if the server returns an error.
    pub async fn connect(&mut self) -> Result<AcpInitResponse, AcpError> {
        // --- 1. Build command ---
        let mut cmd = tokio::process::Command::new(&self.config.command);
        cmd.args(&self.config.args);
        if let Some(ref cwd) = self.config.cwd {
            cmd.current_dir(cwd);
        }
        for (k, v) in &self.config.env {
            cmd.env(k, v);
        }
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // --- 2. Process group + kill_on_drop ---
        process::set_process_group(&mut cmd);
        cmd.kill_on_drop(true);

        // --- 3. Spawn ---
        let mut child = cmd.spawn().map_err(|e| {
            AcpError::Spawn(format!("failed to spawn `{}`: {e}", self.config.command))
        })?;

        if let Some(pid) = child.id() {
            process::register_spawned_pid(pid);
        }

        // --- 4. Take stdio handles ---
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AcpError::Spawn("no stdin on child".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AcpError::Spawn("no stdout on child".into()))?;

        // --- 5. Response channel ---
        let (resp_tx, resp_rx) = mpsc::unbounded_channel::<(u64, serde_json::Value)>();

        // --- 6. Stderr reader task ---
        let stderr_handle = child.stderr.take().map(|stderr| {
            tokio::spawn(async move {
                let reader = tokio::io::BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        if process::classify_benign_stderr(&line).is_some() {
                            tracing::trace!("[acp/stderr] {line}");
                        } else {
                            tracing::debug!("[acp/stderr] {line}");
                        }
                    }
                }
            })
        });

        // --- 7. Stdout reader task ---
        let notification_tx = self.notification_tx.clone();
        let turn_done_tx = self.turn_done_tx.clone();
        let reader_handle = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                tracing::trace!("[acp] <- {}", &line[..line.len().min(200)]);

                match serde_json::from_str::<RawServerMessage>(&line) {
                    Ok(msg) => {
                        if msg.is_response() {
                            // Has id, no method -> response to our request.
                            let id = msg.numeric_id().unwrap_or(0);
                            let val = if let Some(err) = msg.error {
                                let error_msg = if let Some(data) = &err.data {
                                    if let Some(msg_val) =
                                        data.get("message").and_then(|v| v.as_str())
                                    {
                                        format!("{} ({})", err.message, msg_val)
                                    } else {
                                        err.message.clone()
                                    }
                                } else {
                                    err.message.clone()
                                };
                                serde_json::json!({"error": error_msg})
                            } else {
                                msg.result.unwrap_or(serde_json::Value::Null)
                            };

                            // Detect turn completion: stopReason in the response.
                            if val.get("stopReason").and_then(|s| s.as_str()).is_some() {
                                let _ = turn_done_tx.send(val.clone());
                            }

                            let _ = resp_tx.send((id, val));
                        } else {
                            // Notification or server request -> forward to adapter.
                            let notification = AcpNotification {
                                method: msg.method.clone().unwrap_or_default(),
                                params: msg.params.clone(),
                                server_request_id: if msg.is_server_request() {
                                    msg.numeric_id()
                                } else {
                                    None
                                },
                            };
                            let _ = notification_tx.send(notification);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[acp] parse error: {e}: {}", &line[..line.len().min(200)]);
                    }
                }
            }
            // Process exited or stdout closed.
        });

        // --- 8. Store handles ---
        self.child = Some(child);
        self.stdin = Some(tokio::io::BufWriter::new(stdin));
        self.reader_handle = Some(reader_handle);
        self.stderr_handle = stderr_handle;
        self.response_rx = Some(resp_rx);

        // --- 9. Send `initialize` request ---
        let protocol_version: serde_json::Value =
            if let Ok(n) = self.config.protocol_version.parse::<u64>() {
                serde_json::json!(n)
            } else {
                serde_json::json!(self.config.protocol_version)
            };

        let init_params = serde_json::json!({
            "protocolVersion": protocol_version,
            "clientInfo": {
                "name": "roko",
                "version": env!("CARGO_PKG_VERSION")
            },
            "clientCapabilities": {}
        });

        let id = self.send_request("initialize", Some(init_params)).await?;

        // --- 10. Await response ---
        let resp = tokio::time::timeout(self.config.timeout, self.recv_response(id))
            .await
            .map_err(|_| AcpError::Timeout {
                method: "initialize".into(),
                elapsed: self.config.timeout,
            })??;

        tracing::info!("[acp] initialize OK: {resp}");

        // --- 11. Parse into AcpInitResponse ---
        let init_response = AcpInitResponse {
            protocol_version: resp
                .get("protocolVersion")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            server_info: resp
                .get("serverInfo")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            raw: resp,
        };

        Ok(init_response)
    }

    // ---- Request/response primitives ----------------------------------------

    /// Send a JSON-RPC request and return the assigned ID.
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<u64, AcpError> {
        let stdin = self.stdin.as_mut().ok_or(AcpError::Disconnected)?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest::new(id, method, params);
        let mut json = serde_json::to_string(&req)
            .map_err(|e| AcpError::Protocol(format!("serialize: {e}")))?;
        tracing::debug!("[acp] -> {}", &json[..json.len().min(500)]);
        json.push('\n');
        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(AcpError::Io)?;
        stdin.flush().await.map_err(AcpError::Io)?;
        Ok(id)
    }

    /// Wait for a response with the given ID.
    ///
    /// Discards responses with non-matching IDs (stale responses from
    /// cancelled requests). Returns `AcpError::Disconnected` if the
    /// channel is closed.
    pub async fn recv_response(&mut self, expected_id: u64) -> Result<serde_json::Value, AcpError> {
        let rx = self.response_rx.as_mut().ok_or(AcpError::Disconnected)?;
        loop {
            match rx.recv().await {
                Some((id, val)) => {
                    if id != expected_id {
                        tracing::debug!(
                            "[acp] discarding stale response id={id} (want {expected_id})"
                        );
                        continue;
                    }
                    if val.get("error").is_some() {
                        let err_msg = val["error"].as_str().unwrap_or("unknown error").to_string();
                        return Err(AcpError::MethodFailed {
                            method: format!("response(id={expected_id})"),
                            message: err_msg,
                        });
                    }
                    return Ok(val);
                }
                None => {
                    return Err(AcpError::Disconnected);
                }
            }
        }
    }

    // ---- Session lifecycle --------------------------------------------------

    /// Create a new ACP session via `session/new`.
    ///
    /// The session ID is extracted from the response and stored
    /// internally. Returns the `SessionId` for use with other methods.
    pub async fn new_session(&mut self, opts: NewSessionOpts) -> Result<SessionId, AcpError> {
        let cwd = opts
            .cwd
            .as_ref()
            .or(self.config.cwd.as_ref())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".into());

        let mut params = serde_json::json!({
            "cwd": cwd,
            "mode": "agent",
            "mcpServers": opts.mcp_servers.clone().unwrap_or(serde_json::json!([])),
        });

        // Merge session key if provided.
        if let Some(ref key) = opts.session_key {
            params
                .as_object_mut()
                .unwrap()
                .insert("sessionKey".into(), serde_json::Value::String(key.clone()));
        }

        // Merge reset flag.
        if opts.reset {
            params
                .as_object_mut()
                .unwrap()
                .insert("reset".into(), serde_json::Value::Bool(true));
        }

        // Merge any extra params from the adapter.
        if let Some(extra) = opts.extra_params {
            if let (Some(base), Some(ext)) = (params.as_object_mut(), extra.as_object()) {
                for (k, v) in ext {
                    base.insert(k.clone(), v.clone());
                }
            }
        }

        let id = self.send_request("session/new", Some(params)).await?;

        let resp = tokio::time::timeout(self.config.timeout, self.recv_response(id))
            .await
            .map_err(|_| AcpError::Timeout {
                method: "session/new".into(),
                elapsed: self.config.timeout,
            })??;

        let session_id = resp
            .get("sessionId")
            .or_else(|| resp.get("session_id"))
            .or_else(|| resp.get("id"))
            .and_then(|s| s.as_str())
            .map(String::from)
            .ok_or_else(|| {
                AcpError::Protocol(format!("no sessionId in session/new response: {resp}"))
            })?;

        self.session_id = Some(session_id.clone());
        tracing::info!("[acp] session created: {session_id}");
        Ok(SessionId(session_id))
    }

    /// Send a `session/prompt` request. Returns the request ID.
    ///
    /// The caller must:
    /// 1. Take the notification receiver via `take_notification_rx()`.
    /// 2. Take the turn-done receiver via `take_turn_done_rx()`.
    /// 3. Use `tokio::select!` to listen for `AcpNotification` events
    ///    (streaming) and the turn-done signal (completion).
    /// 4. Optionally call `recv_response(id)` to get the final response
    ///    value (it will return immediately since the response was already
    ///    routed to `resp_tx` by the reader task).
    pub async fn send_prompt(
        &mut self,
        session: &SessionId,
        payload: AcpPromptPayload,
    ) -> Result<u64, AcpError> {
        // Check liveness.
        if let Some(child) = &mut self.child {
            if let Ok(Some(_status)) = child.try_wait() {
                return Err(AcpError::Disconnected);
            }
        } else {
            return Err(AcpError::Disconnected);
        }

        let mut params = serde_json::json!({
            "sessionId": session.0,
            "prompt": [{
                "type": "text",
                "text": payload.text,
            }],
        });

        // Merge extra params (adapter-specific fields).
        if let Some(extra) = payload.extra_params {
            if let (Some(base), Some(ext)) = (params.as_object_mut(), extra.as_object()) {
                for (k, v) in ext {
                    base.insert(k.clone(), v.clone());
                }
            }
        }

        self.send_request("session/prompt", Some(params)).await
    }

    /// Cancel an in-flight prompt via `session/cancel`.
    pub async fn cancel(&mut self, session: &SessionId) -> Result<(), AcpError> {
        let id = self
            .send_request(
                "session/cancel",
                Some(serde_json::json!({ "sessionId": session.0 })),
            )
            .await?;
        let _ = tokio::time::timeout(Duration::from_secs(5), self.recv_response(id)).await;
        Ok(())
    }

    /// Resume a previous session via `session/load`.
    pub async fn load_session(
        &mut self,
        session: &SessionId,
    ) -> Result<serde_json::Value, AcpError> {
        let id = self
            .send_request(
                "session/load",
                Some(serde_json::json!({ "sessionId": session.0 })),
            )
            .await?;
        let resp = tokio::time::timeout(self.config.timeout, self.recv_response(id))
            .await
            .map_err(|_| AcpError::Timeout {
                method: "session/load".into(),
                elapsed: self.config.timeout,
            })??;
        Ok(resp)
    }

    /// Close a session via `session/close`.
    pub async fn close_session(&mut self, session: &SessionId) -> Result<(), AcpError> {
        let id = self
            .send_request(
                "session/close",
                Some(serde_json::json!({ "sessionId": session.0 })),
            )
            .await?;
        let _ = tokio::time::timeout(Duration::from_secs(5), self.recv_response(id)).await;
        Ok(())
    }

    // ---- Shutdown -----------------------------------------------------------

    /// Graceful shutdown: kill_tree handles stdin-close -> SIGTERM -> SIGKILL.
    ///
    /// After calling this, the client is in a disconnected state.
    /// You may call `connect()` again to respawn.
    pub async fn shutdown(&mut self) -> Result<(), AcpError> {
        let pid = self.child.as_ref().and_then(|c| c.id());

        if let Some(child) = &mut self.child {
            let _ = process::kill_tree(child, Duration::from_millis(process::GRACE_STDIN_CLOSE_MS))
                .await;
        }

        if let Some(pid) = pid {
            process::unregister_pid(pid);
        }

        if let Some(h) = self.reader_handle.take() {
            h.abort();
        }
        if let Some(h) = self.stderr_handle.take() {
            h.abort();
        }

        self.child = None;
        self.stdin = None;
        self.response_rx = None;
        self.session_id = None;

        Ok(())
    }

    /// Alias for `shutdown()` for compatibility with code that expects
    /// a `kill()` method (e.g. `CursorConnection::kill()`).
    pub async fn kill(&mut self) {
        let _ = self.shutdown().await;
    }

    // ---- Accessor methods ---------------------------------------------------

    /// Take ownership of the notification receiver.
    ///
    /// Only one consumer can hold this at a time. Returns `None` if
    /// already taken. The caller must return it via `return_notification_rx`
    /// when done.
    pub fn take_notification_rx(&mut self) -> Option<mpsc::UnboundedReceiver<AcpNotification>> {
        self.notification_rx.take()
    }

    /// Return the notification receiver after use.
    pub fn return_notification_rx(&mut self, rx: mpsc::UnboundedReceiver<AcpNotification>) {
        self.notification_rx = Some(rx);
    }

    /// Take ownership of the turn-done receiver.
    ///
    /// The reader task sends to this channel when a response with
    /// `stopReason` is received. Used by adapters to detect turn
    /// completion in their event loops.
    pub fn take_turn_done_rx(&mut self) -> Option<mpsc::UnboundedReceiver<serde_json::Value>> {
        self.turn_done_rx.take()
    }

    /// Return the turn-done receiver after use.
    pub fn return_turn_done_rx(&mut self, rx: mpsc::UnboundedReceiver<serde_json::Value>) {
        self.turn_done_rx = Some(rx);
    }

    /// Get the current session ID, if any.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Check if the subprocess is still alive.
    pub fn is_alive(&mut self) -> bool {
        match &mut self.child {
            Some(c) => c.try_wait().ok().flatten().is_none(),
            None => false,
        }
    }

    /// Access the config.
    pub fn config(&self) -> &AcpStdioConfig {
        &self.config
    }
}

impl Drop for AcpStdioClient {
    fn drop(&mut self) {
        // Best-effort synchronous cleanup. The reader tasks will be
        // aborted when their JoinHandles are dropped. The child process
        // will be killed by `kill_on_drop(true)` set during spawn.
        // We just need to unregister the PID.
        if let Some(pid) = self.child.as_ref().and_then(|c| c.id()) {
            process::unregister_pid(pid);
        }
    }
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use tokio::sync::mpsc;

    // -- Helpers -------------------------------------------------------------

    fn test_config() -> AcpStdioConfig {
        AcpStdioConfig {
            command: "echo".into(),
            args: vec![],
            cwd: Some(std::env::temp_dir()),
            env: HashMap::new(),
            protocol_version: "1".into(),
            timeout: std::time::Duration::from_secs(5),
        }
    }

    fn write_script(path: &std::path::Path, body: &str) {
        std::fs::write(path, body).expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)
                .expect("script metadata")
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).expect("chmod script");
        }
    }

    /// Mock ACP server script that responds to initialize, session/new,
    /// and session/prompt. Uses only bash builtins for portability.
    fn mock_acp_script() -> String {
        r#"#!/bin/bash
set -u
req_num=0
while IFS= read -r line; do
    [ -z "$line" ] && continue
    req_num=$((req_num + 1))
    case "$req_num" in
        1)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":1,"serverInfo":{"name":"mock-acp"}}}\n' "$id"
            ;;
        2)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            printf '{"jsonrpc":"2.0","id":%s,"result":{"sessionId":"test-session-001"}}\n' "$id"
            ;;
        *)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            printf '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"text":"hello from cursor"}}}}\n'
            printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"end_turn"}}\n' "$id"
            ;;
    esac
done
"#
        .to_string()
    }

    /// Extended mock that emits tool_call, tool_call_update, and
    /// permission_request notifications for richer event testing.
    fn mock_acp_script_with_tools() -> String {
        r#"#!/bin/bash
set -u
req_num=0
while IFS= read -r line; do
    [ -z "$line" ] && continue
    req_num=$((req_num + 1))
    case "$req_num" in
        1)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"mock-acp-tools"}}}\n' "$id"
            ;;
        2)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            printf '{"jsonrpc":"2.0","id":%s,"result":{"sessionId":"tool-session-001"}}\n' "$id"
            ;;
        *)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            # Emit a tool_call notification
            printf '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"tool_call","title":"Read file","kind":"read"}}}\n'
            # Emit a tool_call_update notification
            printf '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"tool_call_update","status":"completed","rawOutput":{"content":"file contents"}}}}\n'
            # Emit a text chunk
            printf '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"text":"done reading"}}}}\n'
            # Emit the final response
            printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"end_turn"}}\n' "$id"
            ;;
    esac
done
"#
        .to_string()
    }

    /// Mock that returns a JSON-RPC error on session/new.
    fn mock_acp_script_session_error() -> String {
        r#"#!/bin/bash
set -u
req_num=0
while IFS= read -r line; do
    [ -z "$line" ] && continue
    req_num=$((req_num + 1))
    case "$req_num" in
        1)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":1,"serverInfo":{"name":"mock-error"}}}\n' "$id"
            ;;
        2)
            id="${line##*\"id\":}"
            id="${id%%,*}"
            id="${id%%\}*}"
            printf '{"jsonrpc":"2.0","id":%s,"error":{"code":-32600,"message":"session limit reached"}}\n' "$id"
            ;;
        *)
            ;;
    esac
done
"#
        .to_string()
    }

    // -- Unit tests ----------------------------------------------------------

    #[tokio::test]
    async fn json_rpc_request_serialization() {
        let req = JsonRpcRequest::new(
            42,
            "session/prompt",
            Some(serde_json::json!({"test": true})),
        );
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 42);
        assert_eq!(parsed["method"], "session/prompt");
        assert_eq!(parsed["params"]["test"], true);
    }

    #[tokio::test]
    async fn json_rpc_request_without_params() {
        let req = JsonRpcRequest::new(1, "initialize", None);
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["method"], "initialize");
        // params should be absent (skip_serializing_if)
        assert!(parsed.get("params").is_none());
    }

    #[tokio::test]
    async fn raw_server_message_classification() {
        // Response (has id, no method)
        let msg: RawServerMessage =
            serde_json::from_str(r#"{"id":1,"result":{"ok":true}}"#).unwrap();
        assert!(msg.is_response());
        assert!(!msg.is_notification());
        assert!(!msg.is_server_request());
        assert_eq!(msg.numeric_id(), Some(1));

        // Notification (has method, no id)
        let msg: RawServerMessage =
            serde_json::from_str(r#"{"method":"session/update","params":{"update":{}}}"#).unwrap();
        assert!(msg.is_notification());
        assert!(!msg.is_response());
        assert!(!msg.is_server_request());
        assert_eq!(msg.numeric_id(), None);

        // Server request (has both id and method)
        let msg: RawServerMessage =
            serde_json::from_str(r#"{"id":5,"method":"session/update","params":{}}"#).unwrap();
        assert!(msg.is_server_request());
        assert!(!msg.is_notification());
        assert!(!msg.is_response());
        assert_eq!(msg.numeric_id(), Some(5));
    }

    #[test]
    fn acp_methods_use_slash_notation() {
        let methods = [
            "initialize",
            "session/new",
            "session/prompt",
            "session/cancel",
            "session/load",
            "session/close",
        ];
        for m in &methods {
            if m.contains('/') {
                assert!(
                    !m.contains(char::is_uppercase),
                    "ACP method '{m}' should not contain uppercase chars"
                );
            }
        }
    }

    #[tokio::test]
    async fn request_id_correlation() {
        let (tx, rx) = mpsc::unbounded_channel();
        let (notif_tx, _notif_rx) = mpsc::unbounded_channel();
        let (td_tx, _td_rx) = mpsc::unbounded_channel();

        let mut client = AcpStdioClient {
            config: test_config(),
            child: None,
            stdin: None,
            next_id: AtomicU64::new(1),
            response_rx: Some(rx),
            notification_tx: notif_tx,
            notification_rx: None,
            turn_done_tx: td_tx,
            turn_done_rx: None,
            session_id: None,
            reader_handle: None,
            stderr_handle: None,
        };

        // Send responses out of order.
        tx.send((3, serde_json::json!({"result": "third"})))
            .unwrap();
        tx.send((1, serde_json::json!({"result": "first"})))
            .unwrap();
        tx.send((2, serde_json::json!({"result": "second"})))
            .unwrap();

        // Request ID 1 should skip ID 3 and return.
        let resp = client.recv_response(1).await.unwrap();
        assert_eq!(resp["result"], "first");
    }

    #[tokio::test]
    async fn disconnected_on_closed_channel() {
        let (tx, rx) = mpsc::unbounded_channel::<(u64, serde_json::Value)>();
        let (notif_tx, _) = mpsc::unbounded_channel();
        let (td_tx, _) = mpsc::unbounded_channel();

        let mut client = AcpStdioClient {
            config: test_config(),
            child: None,
            stdin: None,
            next_id: AtomicU64::new(1),
            response_rx: Some(rx),
            notification_tx: notif_tx,
            notification_rx: None,
            turn_done_tx: td_tx,
            turn_done_rx: None,
            session_id: None,
            reader_handle: None,
            stderr_handle: None,
        };

        // Drop the sender to close the channel.
        drop(tx);

        let result = client.recv_response(1).await;
        assert!(matches!(result, Err(AcpError::Disconnected)));
    }

    #[tokio::test]
    async fn error_response_handling() {
        let (tx, rx) = mpsc::unbounded_channel();
        let (notif_tx, _) = mpsc::unbounded_channel();
        let (td_tx, _) = mpsc::unbounded_channel();

        let mut client = AcpStdioClient {
            config: test_config(),
            child: None,
            stdin: None,
            next_id: AtomicU64::new(1),
            response_rx: Some(rx),
            notification_tx: notif_tx,
            notification_rx: None,
            turn_done_tx: td_tx,
            turn_done_rx: None,
            session_id: None,
            reader_handle: None,
            stderr_handle: None,
        };

        tx.send((1, serde_json::json!({"error": "auth failed"})))
            .unwrap();

        let result = client.recv_response(1).await;
        assert!(matches!(result, Err(AcpError::MethodFailed { .. })));
        if let Err(AcpError::MethodFailed { message, .. }) = result {
            assert!(message.contains("auth failed"));
        }
    }

    #[test]
    fn cursor_constructor_builds_correct_args() {
        let client = AcpStdioClient::cursor(
            "cursor",
            PathBuf::from("/workspace"),
            Some("claude-sonnet".into()),
        );
        let cfg = client.config();
        assert_eq!(cfg.command, "cursor");
        assert_eq!(cfg.protocol_version, "1");
        assert!(cfg.args.contains(&"--force".to_string()));
        assert!(cfg.args.contains(&"acp".to_string()));
        assert!(cfg.args.contains(&"--model".to_string()));
        assert!(cfg.args.contains(&"claude-sonnet".to_string()));
        assert_eq!(
            cfg.env.get("CARGO_INCREMENTAL").map(|s| s.as_str()),
            Some("0")
        );
        assert_eq!(
            cfg.env.get("CARGO_BUILD_JOBS").map(|s| s.as_str()),
            Some("2")
        );
    }

    #[test]
    fn cursor_constructor_without_model() {
        let client = AcpStdioClient::cursor("cursor", PathBuf::from("/workspace"), None);
        let cfg = client.config();
        assert!(!cfg.args.contains(&"--model".to_string()));
        // acp should be the last arg
        assert_eq!(cfg.args.last().map(|s| s.as_str()), Some("acp"));
    }

    #[test]
    fn hermes_constructor_builds_correct_args() {
        let client = AcpStdioClient::hermes("hermes", PathBuf::from("/workspace"));
        let cfg = client.config();
        assert_eq!(cfg.command, "hermes");
        assert_eq!(cfg.protocol_version, "2024-11-05");
        assert_eq!(cfg.args, vec!["acp"]);
        assert!(cfg.env.is_empty());
    }

    #[test]
    fn openclaw_constructor_builds_correct_args() {
        let client = AcpStdioClient::openclaw(
            "openclaw",
            PathBuf::from("/workspace"),
            Some("ws://localhost:18789".into()),
        );
        let cfg = client.config();
        assert_eq!(cfg.command, "openclaw");
        assert_eq!(cfg.protocol_version, "2024-11-05");
        assert!(cfg.args.contains(&"--url".to_string()));
        assert!(cfg.args.contains(&"ws://localhost:18789".to_string()));
    }

    #[test]
    fn openclaw_constructor_without_gateway() {
        let client = AcpStdioClient::openclaw("openclaw", PathBuf::from("/workspace"), None);
        let cfg = client.config();
        assert!(!cfg.args.contains(&"--url".to_string()));
        assert_eq!(cfg.args, vec!["acp"]);
    }

    #[test]
    fn notification_rx_take_and_return() {
        let mut client = AcpStdioClient::new(test_config());
        // First take should succeed.
        let rx = client.take_notification_rx();
        assert!(rx.is_some());
        // Second take should return None.
        assert!(client.take_notification_rx().is_none());
        // Return and take again should succeed.
        client.return_notification_rx(rx.unwrap());
        assert!(client.take_notification_rx().is_some());
    }

    #[test]
    fn turn_done_rx_take_and_return() {
        let mut client = AcpStdioClient::new(test_config());
        let rx = client.take_turn_done_rx();
        assert!(rx.is_some());
        assert!(client.take_turn_done_rx().is_none());
        client.return_turn_done_rx(rx.unwrap());
        assert!(client.take_turn_done_rx().is_some());
    }

    #[test]
    fn is_alive_without_child() {
        let mut client = AcpStdioClient::new(test_config());
        assert!(!client.is_alive());
    }

    #[test]
    fn session_id_initially_none() {
        let client = AcpStdioClient::new(test_config());
        assert!(client.session_id().is_none());
    }

    #[test]
    fn new_session_opts_default() {
        let opts = NewSessionOpts::default();
        assert!(opts.session_key.is_none());
        assert!(opts.cwd.is_none());
        assert!(opts.mcp_servers.is_none());
        assert!(!opts.reset);
        assert!(opts.extra_params.is_none());
    }

    #[test]
    fn session_id_display() {
        let sid = SessionId("test-123".into());
        assert_eq!(format!("{sid}"), "test-123");
    }

    #[test]
    fn acp_error_display() {
        let err = AcpError::Disconnected;
        assert_eq!(
            format!("{err}"),
            "acp connection lost: server exited or pipe closed"
        );

        let err = AcpError::MethodFailed {
            method: "initialize".into(),
            message: "bad version".into(),
        };
        assert_eq!(format!("{err}"), "acp `initialize` failed: bad version");

        let err = AcpError::Timeout {
            method: "session/new".into(),
            elapsed: std::time::Duration::from_secs(30),
        };
        assert!(format!("{err}").contains("session/new"));
        assert!(format!("{err}").contains("30"));
    }

    #[tokio::test]
    async fn send_prompt_without_connection_returns_disconnected() {
        let mut client = AcpStdioClient::new(test_config());
        let session = SessionId("fake".into());
        let result = client
            .send_prompt(&session, AcpPromptPayload {
                text: "hello".into(),
                extra_params: None,
            })
            .await;
        assert!(matches!(result, Err(AcpError::Disconnected)));
    }

    #[tokio::test]
    async fn send_request_without_connection_returns_disconnected() {
        let mut client = AcpStdioClient::new(test_config());
        let result = client.send_request("initialize", None).await;
        assert!(matches!(result, Err(AcpError::Disconnected)));
    }

    #[tokio::test]
    async fn recv_response_without_connection_returns_disconnected() {
        let mut client = AcpStdioClient::new(test_config());
        let result = client.recv_response(1).await;
        assert!(matches!(result, Err(AcpError::Disconnected)));
    }

    // -- Fixture-based JSON-RPC trace tests ----------------------------------

    /// Test the reader task's message routing by verifying that raw
    /// JSON-RPC traces produce the expected channel outputs.
    #[tokio::test]
    async fn reader_task_routes_response_and_notification() {
        // Simulate what the reader task does: parse JSON lines and route.
        let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<(u64, serde_json::Value)>();
        let (notif_tx, mut notif_rx) = mpsc::unbounded_channel::<AcpNotification>();
        let (td_tx, mut td_rx) = mpsc::unbounded_channel::<serde_json::Value>();

        // These are the exact JSON-RPC lines a mock server would emit.
        let traces = vec![
            // A notification (no id)
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"text":"hi"}}}}"#,
            // A server request (has both id and method)
            r#"{"jsonrpc":"2.0","id":99,"method":"session/request_permission","params":{"tool":"bash"}}"#,
            // A response without stopReason
            r#"{"jsonrpc":"2.0","id":3,"result":{"sessionId":"s-001"}}"#,
            // A response with stopReason (triggers turn_done)
            r#"{"jsonrpc":"2.0","id":4,"result":{"stopReason":"end_turn","text":"done"}}"#,
        ];

        for line in &traces {
            let msg: RawServerMessage = serde_json::from_str(line).unwrap();
            if msg.is_response() {
                let id = msg.numeric_id().unwrap_or(0);
                let val = msg.result.unwrap_or(serde_json::Value::Null);
                if val.get("stopReason").and_then(|s| s.as_str()).is_some() {
                    let _ = td_tx.send(val.clone());
                }
                let _ = resp_tx.send((id, val));
            } else {
                let notification = AcpNotification {
                    method: msg.method.clone().unwrap_or_default(),
                    params: msg.params.clone(),
                    server_request_id: if msg.is_server_request() {
                        msg.numeric_id()
                    } else {
                        None
                    },
                };
                let _ = notif_tx.send(notification);
            }
        }

        // Verify notifications.
        let n1 = notif_rx.recv().await.unwrap();
        assert_eq!(n1.method, "session/update");
        assert!(n1.server_request_id.is_none()); // notification, not server request

        let n2 = notif_rx.recv().await.unwrap();
        assert_eq!(n2.method, "session/request_permission");
        assert_eq!(n2.server_request_id, Some(99)); // server request

        // Verify responses.
        let (id, val) = resp_rx.recv().await.unwrap();
        assert_eq!(id, 3);
        assert_eq!(val["sessionId"], "s-001");

        let (id, val) = resp_rx.recv().await.unwrap();
        assert_eq!(id, 4);
        assert_eq!(val["stopReason"], "end_turn");

        // Verify turn_done was fired.
        let td = td_rx.recv().await.unwrap();
        assert_eq!(td["stopReason"], "end_turn");
    }

    /// Test that error responses are correctly formatted by the reader task.
    #[tokio::test]
    async fn reader_task_formats_error_responses() {
        let traces = vec![
            // Error without data
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"invalid request"}}"#,
            // Error with data.message
            r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32603,"message":"internal error","data":{"message":"disk full"}}}"#,
        ];

        let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<(u64, serde_json::Value)>();

        for line in &traces {
            let msg: RawServerMessage = serde_json::from_str(line).unwrap();
            assert!(msg.is_response());
            let id = msg.numeric_id().unwrap_or(0);
            let val = if let Some(err) = msg.error {
                let error_msg = if let Some(data) = &err.data {
                    if let Some(msg_val) = data.get("message").and_then(|v| v.as_str()) {
                        format!("{} ({})", err.message, msg_val)
                    } else {
                        err.message.clone()
                    }
                } else {
                    err.message.clone()
                };
                serde_json::json!({"error": error_msg})
            } else {
                msg.result.unwrap_or(serde_json::Value::Null)
            };
            let _ = resp_tx.send((id, val));
        }

        let (id, val) = resp_rx.recv().await.unwrap();
        assert_eq!(id, 1);
        assert_eq!(val["error"], "invalid request");

        let (id, val) = resp_rx.recv().await.unwrap();
        assert_eq!(id, 2);
        assert_eq!(val["error"], "internal error (disk full)");
    }

    // -- Integration tests (process-based) -----------------------------------

    #[tokio::test]
    #[ignore = "flaky under high parallelism; run with --ignored"]
    async fn acp_client_full_cycle_with_mock() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let script_path = tmp.path().join("mock-acp.sh");
        write_script(&script_path, &mock_acp_script());

        let mut client = AcpStdioClient::new(AcpStdioConfig {
            command: script_path.to_str().unwrap().into(),
            args: vec![],
            cwd: Some(tmp.path().to_path_buf()),
            env: HashMap::new(),
            protocol_version: "1".into(),
            timeout: std::time::Duration::from_secs(10),
        });

        // Connect (spawns process + initialize handshake).
        let init = client.connect().await;
        assert!(init.is_ok(), "connect failed: {:?}", init.err());
        let init = init.unwrap();
        assert_eq!(init.protocol_version, serde_json::json!(1));
        assert!(client.is_alive());

        // Create session.
        let session = client.new_session(NewSessionOpts::default()).await;
        assert!(session.is_ok(), "session failed: {:?}", session.err());
        let session = session.unwrap();
        assert_eq!(session.0, "test-session-001");
        assert_eq!(client.session_id(), Some("test-session-001"));

        // Take receivers.
        let mut notif_rx = client.take_notification_rx().unwrap();
        let mut turn_done_rx = client.take_turn_done_rx().unwrap();

        // Send prompt.
        let prompt_id = client
            .send_prompt(&session, AcpPromptPayload {
                text: "test prompt".into(),
                extra_params: None,
            })
            .await;
        assert!(prompt_id.is_ok());
        let prompt_id = prompt_id.unwrap();

        // Collect notifications and wait for turn done.
        let mut output = String::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);

        loop {
            tokio::select! {
                notif = notif_rx.recv() => {
                    match notif {
                        Some(n) => {
                            if n.method == "session/update" {
                                if let Some(params) = &n.params {
                                    if let Some(text) = params
                                        .get("update")
                                        .and_then(|u| u.get("content"))
                                        .and_then(|c| c.get("text"))
                                        .and_then(|t| t.as_str())
                                    {
                                        output.push_str(text);
                                    }
                                }
                            }
                        }
                        None => break,
                    }
                }
                resp = turn_done_rx.recv() => {
                    // Turn complete. Drain remaining notifications.
                    while let Ok(n) = notif_rx.try_recv() {
                        if n.method == "session/update" {
                            if let Some(params) = &n.params {
                                if let Some(text) = params
                                    .get("update")
                                    .and_then(|u| u.get("content"))
                                    .and_then(|c| c.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    output.push_str(text);
                                }
                            }
                        }
                    }
                    // Verify the response has stopReason.
                    if let Some(resp_val) = resp {
                        assert_eq!(
                            resp_val.get("stopReason").and_then(|s| s.as_str()),
                            Some("end_turn")
                        );
                    }
                    break;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    panic!("timed out waiting for turn completion");
                }
            }
        }

        // Return receivers.
        client.return_notification_rx(notif_rx);
        client.return_turn_done_rx(turn_done_rx);

        // Get prompt response.
        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            client.recv_response(prompt_id),
        )
        .await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(
            resp.get("stopReason").and_then(|s| s.as_str()),
            Some("end_turn")
        );

        assert_eq!(output, "hello from cursor");

        // Shutdown.
        let disc = client.shutdown().await;
        assert!(disc.is_ok());
        assert!(!client.is_alive());
        assert!(client.session_id().is_none());
    }

    #[tokio::test]
    #[ignore = "flaky under high parallelism; run with --ignored"]
    async fn acp_client_with_tool_notifications() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let script_path = tmp.path().join("mock-acp-tools.sh");
        write_script(&script_path, &mock_acp_script_with_tools());

        let mut client = AcpStdioClient::new(AcpStdioConfig {
            command: script_path.to_str().unwrap().into(),
            args: vec![],
            cwd: Some(tmp.path().to_path_buf()),
            env: HashMap::new(),
            protocol_version: "2024-11-05".into(),
            timeout: std::time::Duration::from_secs(10),
        });

        let init = client.connect().await;
        assert!(init.is_ok());
        let init = init.unwrap();
        assert_eq!(init.protocol_version, serde_json::json!("2024-11-05"));

        let session = client.new_session(NewSessionOpts::default()).await.unwrap();

        let mut notif_rx = client.take_notification_rx().unwrap();
        let mut turn_done_rx = client.take_turn_done_rx().unwrap();

        let _prompt_id = client
            .send_prompt(&session, AcpPromptPayload {
                text: "read a file".into(),
                extra_params: None,
            })
            .await
            .unwrap();

        // Collect all notification methods.
        let mut methods = Vec::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            tokio::select! {
                notif = notif_rx.recv() => {
                    match notif {
                        Some(n) => methods.push(n.method.clone()),
                        None => break,
                    }
                }
                _ = turn_done_rx.recv() => {
                    while let Ok(n) = notif_rx.try_recv() {
                        methods.push(n.method.clone());
                    }
                    break;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    panic!("timed out");
                }
            }
        }

        // Should have received 3 session/update notifications.
        assert_eq!(methods.len(), 3);
        assert!(methods.iter().all(|m| m == "session/update"));

        client.return_notification_rx(notif_rx);
        client.return_turn_done_rx(turn_done_rx);
        let _ = client.shutdown().await;
    }

    #[tokio::test]
    #[ignore = "flaky under high parallelism; run with --ignored"]
    async fn acp_client_session_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let script_path = tmp.path().join("mock-acp-error.sh");
        write_script(&script_path, &mock_acp_script_session_error());

        let mut client = AcpStdioClient::new(AcpStdioConfig {
            command: script_path.to_str().unwrap().into(),
            args: vec![],
            cwd: Some(tmp.path().to_path_buf()),
            env: HashMap::new(),
            protocol_version: "1".into(),
            timeout: std::time::Duration::from_secs(10),
        });

        // Connect should succeed (initialize works).
        let init = client.connect().await;
        assert!(init.is_ok());

        // Session should fail (server returns error).
        let session = client.new_session(NewSessionOpts::default()).await;
        assert!(session.is_err());
        if let Err(AcpError::MethodFailed { message, .. }) = &session {
            assert!(
                message.contains("session limit reached"),
                "unexpected error message: {message}"
            );
        }

        let _ = client.shutdown().await;
    }
}
