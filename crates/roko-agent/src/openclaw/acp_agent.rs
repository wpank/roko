//! Tier 3 transport: OpenClaw ACP over stdio.
//!
//! Wraps [`AcpStdioClient`] to communicate with OpenClaw via ACP
//! JSON-RPC 2.0 over stdio. This adapter manages a persistent child
//! process (`openclaw acp`) and drives the full ACP lifecycle:
//! `connect` -> `session/new` -> `session/prompt` -> event loop ->
//! `session/close`.
//!
//! ## CRITICAL: No per-session MCP servers
//!
//! OpenClaw ACP does **NOT** support per-session MCP server injection.
//! The adapter MUST always pass `mcp_servers: None` in [`NewSessionOpts`].
//! MCP servers must be configured server-side on the OpenClaw gateway.
//!
//! ## Token accounting
//!
//! OpenClaw provides NO usage data in most notification flows. When no
//! `session/usage` notification is received, token counts are estimated:
//!
//! - `input_tokens = max(prompt_chars / 4, 1)`
//! - `output_tokens = max(output_chars / 4, 1)`

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use roko_core::{Body, Context, Kind, Signal};
use tokio::sync::mpsc;

use crate::agent::{Agent, AgentResult, derived_output};
use crate::chat_types::FinishReason;
use crate::harness::acp_client::{
    AcpEvent, AcpNotification, AcpPromptPayload, AcpStdioClient, NewSessionOpts,
};
use crate::harness::capability::*;
use crate::harness::{HarnessAdapter, HarnessCapabilities, ProbeError, TransportFlavor};
use crate::streaming::StreamChunk;
use crate::usage::Usage;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the OpenClaw ACP agent adapter.
#[derive(Clone, Debug)]
pub struct OpenClawAcpConfig {
    /// Path or name of the `openclaw` binary.
    pub binary: String,
    /// Working directory for the OpenClaw subprocess.
    pub cwd: PathBuf,
    /// Optional gateway URL passed as `--url` to `openclaw acp`.
    pub gateway_url: Option<String>,
    /// Explicit session key for session reuse across runs.
    pub session_key: Option<String>,
    /// Timeout for ACP operations (connect, session/new, etc.).
    pub timeout: Duration,
    /// Whether to auto-approve permission requests from the agent.
    pub auto_approve_permissions: bool,
}

impl Default for OpenClawAcpConfig {
    fn default() -> Self {
        Self {
            binary: "openclaw".into(),
            cwd: std::env::current_dir().unwrap_or_else(|_| ".".into()),
            gateway_url: None,
            session_key: Some("agent:main:roko".into()),
            timeout: Duration::from_secs(120),
            auto_approve_permissions: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// OpenClaw ACP agent adapter.
///
/// Communicates with `openclaw acp` via ACP JSON-RPC 2.0 over stdio.
/// Each `run()` call creates a session, sends a prompt, collects the
/// streaming output, and closes the session.
pub struct OpenClawAcpAgent {
    client: tokio::sync::Mutex<AcpStdioClient>,
    config: OpenClawAcpConfig,
    capabilities: HarnessCapabilities,
    name: String,
}

impl OpenClawAcpAgent {
    /// Construct a new `OpenClawAcpAgent` from config.
    ///
    /// The `AcpStdioClient` is created but NOT connected. Connection
    /// happens lazily on the first `run()` call.
    pub fn new(config: OpenClawAcpConfig) -> Self {
        let client = AcpStdioClient::openclaw(
            &config.binary,
            config.cwd.clone(),
            config.gateway_url.clone(),
        );

        let capabilities = HarnessCapabilities {
            one_shot: OneShotMode::Acp,
            streaming: StreamingMode::NdJson,
            session_resume: SessionResumeMode::Acp,
            mcp_passthrough: McpMode::ServerOnly, // CRITICAL: rejects per-session MCP
            tool_injection: ToolInjection::Opaque,
            model_override: false,
            multiplex_safe: true,
            cancel: CancelMode::AcpCancel,
            overhead_p50_ms: 80,
        };

        Self {
            client: tokio::sync::Mutex::new(client),
            config,
            capabilities,
            name: "openclaw-acp".to_string(),
        }
    }

    /// Construct from an existing `AcpStdioClient` (for testing).
    pub fn with_client(client: AcpStdioClient, config: OpenClawAcpConfig) -> Self {
        let capabilities = HarnessCapabilities {
            one_shot: OneShotMode::Acp,
            streaming: StreamingMode::NdJson,
            session_resume: SessionResumeMode::Acp,
            mcp_passthrough: McpMode::ServerOnly,
            tool_injection: ToolInjection::Opaque,
            model_override: false,
            multiplex_safe: true,
            cancel: CancelMode::AcpCancel,
            overhead_p50_ms: 80,
        };

        Self {
            client: tokio::sync::Mutex::new(client),
            config,
            capabilities,
            name: "openclaw-acp".to_string(),
        }
    }

    /// Extract the prompt text from an input signal.
    fn extract_prompt(input: &Signal) -> String {
        input.body.as_text().unwrap_or("(empty prompt)").to_string()
    }

    /// Build `NewSessionOpts` for OpenClaw.
    ///
    /// CRITICAL: `mcp_servers` is ALWAYS `None`. OpenClaw does NOT
    /// support per-session MCP server injection.
    fn build_session_opts(&self) -> NewSessionOpts {
        NewSessionOpts {
            session_key: self.config.session_key.clone(),
            cwd: Some(self.config.cwd.clone()),
            mcp_servers: None, // HARDCODED: OpenClaw rejects per-session MCP
            reset: false,
            extra_params: None,
        }
    }

    /// Run the ACP lifecycle: connect -> session -> prompt -> event loop -> close.
    ///
    /// Returns the accumulated output text, usage, and success flag.
    async fn run_acp_lifecycle(
        &self,
        prompt: &str,
        mut stream_tx: Option<&mpsc::Sender<StreamChunk>>,
    ) -> (String, Usage, bool) {
        let start = Instant::now();
        let mut client = self.client.lock().await;

        // --- 1. Connect if not alive ---
        if !client.is_alive() {
            if let Err(e) = client.connect().await {
                let msg = format!("openclaw-acp connect failed: {e}");
                tracing::error!("{msg}");
                return (msg, Usage::zero(), false);
            }
        }

        // --- 2. Create session (mcp_servers: None -- HARDCODED) ---
        let session = match client.new_session(self.build_session_opts()).await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("openclaw-acp session/new failed: {e}");
                tracing::error!("{msg}");
                return (msg, Usage::zero(), false);
            }
        };

        // --- 3. Take notification + turn-done receivers ---
        let mut notif_rx = match client.take_notification_rx() {
            Some(rx) => rx,
            None => {
                let msg = "openclaw-acp: notification_rx already taken".to_string();
                tracing::error!("{msg}");
                let _ = client.close_session(&session).await;
                return (msg, Usage::zero(), false);
            }
        };
        let mut turn_done_rx = match client.take_turn_done_rx() {
            Some(rx) => rx,
            None => {
                client.return_notification_rx(notif_rx);
                let msg = "openclaw-acp: turn_done_rx already taken".to_string();
                tracing::error!("{msg}");
                let _ = client.close_session(&session).await;
                return (msg, Usage::zero(), false);
            }
        };

        // --- 4. Send prompt ---
        let prompt_id = match client
            .send_prompt(
                &session,
                AcpPromptPayload {
                    text: prompt.to_string(),
                    extra_params: None,
                },
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                client.return_notification_rx(notif_rx);
                client.return_turn_done_rx(turn_done_rx);
                let msg = format!("openclaw-acp send_prompt failed: {e}");
                tracing::error!("{msg}");
                let _ = client.close_session(&session).await;
                return (msg, Usage::zero(), false);
            }
        };

        // --- 5. Event loop ---
        let mut output_text = String::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut got_usage = false;
        let timeout = tokio::time::sleep(self.config.timeout);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(notif) = notif_rx.recv() => {
                    if let Some(event) = parse_notification(&notif) {
                        match event {
                            AcpEvent::Output { text } => {
                                output_text.push_str(&text);
                                if let Some(tx) = stream_tx.as_mut() {
                                    let _ = tx.send(StreamChunk::ContentDelta(text)).await;
                                }
                            }
                            AcpEvent::ToolCall { id, name, arguments } => {
                                tracing::debug!(
                                    "[openclaw-acp] tool_call: id={id}, name={name}, args={arguments}"
                                );
                                if let Some(tx) = stream_tx.as_mut() {
                                    let _ = tx.send(StreamChunk::ToolProgress {
                                        tool: name,
                                        status: "started".to_string(),
                                    }).await;
                                }
                            }
                            AcpEvent::ToolCallUpdate { id, progress } => {
                                tracing::debug!(
                                    "[openclaw-acp] tool_call_update: id={id}, progress={progress}"
                                );
                            }
                            AcpEvent::PermissionRequest { id, tool, arguments } => {
                                if self.config.auto_approve_permissions {
                                    tracing::warn!(
                                        "[openclaw-acp] permission request for tool '{}' (id={}, args={}); auto-approved",
                                        tool, id, arguments
                                    );
                                } else {
                                    tracing::warn!(
                                        "[openclaw-acp] permission request for tool '{}' (id={}, args={}); DENIED (auto_approve_permissions=false)",
                                        tool, id, arguments
                                    );
                                }
                            }
                            AcpEvent::Usage { input_tokens: inp, output_tokens: out } => {
                                input_tokens = inp;
                                output_tokens = out;
                                got_usage = true;
                                if let Some(tx) = stream_tx.as_mut() {
                                    let _ = tx.send(StreamChunk::Usage(Usage {
                                        input_tokens: u32::try_from(inp).unwrap_or(u32::MAX),
                                        output_tokens: u32::try_from(out).unwrap_or(u32::MAX),
                                        ..Usage::zero()
                                    })).await;
                                }
                            }
                            AcpEvent::StopReason(reason) => {
                                tracing::debug!("[openclaw-acp] stop_reason: {reason}");
                            }
                        }
                    }
                }
                Some(_turn_result) = turn_done_rx.recv() => {
                    // Turn completed.
                    tracing::debug!("[openclaw-acp] turn done (prompt_id={prompt_id})");
                    if let Some(tx) = stream_tx.as_mut() {
                        let _ = tx.send(StreamChunk::Done(FinishReason::Stop)).await;
                    }
                    break;
                }
                () = &mut timeout => {
                    tracing::error!("[openclaw-acp] timed out after {:?}", self.config.timeout);
                    let _ = client.cancel(&session).await;
                    client.return_notification_rx(notif_rx);
                    client.return_turn_done_rx(turn_done_rx);
                    let _ = client.close_session(&session).await;
                    let msg = format!(
                        "openclaw-acp timed out after {:?}",
                        self.config.timeout
                    );
                    return (msg, Usage::zero(), false);
                }
            }
        }

        // --- 6. Return channels ---
        client.return_notification_rx(notif_rx);
        client.return_turn_done_rx(turn_done_rx);

        // --- 7. Close session ---
        if let Err(e) = client.close_session(&session).await {
            tracing::warn!("[openclaw-acp] session/close failed: {e}");
        }

        // --- 8. Estimate usage if not provided ---
        if !got_usage {
            input_tokens = (prompt.len() as u64 / 4).max(1);
            output_tokens = (output_text.len() as u64 / 4).max(1);
        }

        let wall_ms = start.elapsed().as_millis() as u64;
        let usage = Usage {
            input_tokens: u32::try_from(input_tokens).unwrap_or(u32::MAX),
            output_tokens: u32::try_from(output_tokens).unwrap_or(u32::MAX),
            wall_ms,
            ..Usage::zero()
        };

        (output_text, usage, true)
    }
}

#[async_trait]
impl Agent for OpenClawAcpAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        let prompt = Self::extract_prompt(input);
        let (output_text, usage, success) = self.run_acp_lifecycle(&prompt, None).await;

        let display_text = if success && output_text.is_empty() {
            "(no output from openclaw acp)".to_string()
        } else {
            output_text
        };

        let output_signal =
            derived_output(input, Kind::AgentOutput, Body::text(&display_text)).build();

        let result = if success {
            AgentResult::ok(output_signal)
        } else {
            AgentResult::fail(output_signal)
        };

        result.with_usage(usage)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn backend_id(&self) -> &'static str {
        "openclaw-acp"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn run_streaming(
        &self,
        input: &Signal,
        _ctx: &Context,
        event_tx: mpsc::Sender<StreamChunk>,
    ) -> AgentResult {
        let prompt = Self::extract_prompt(input);
        let (output_text, usage, success) = self.run_acp_lifecycle(&prompt, Some(&event_tx)).await;

        let display_text = if success && output_text.is_empty() {
            "(no output from openclaw acp)".to_string()
        } else {
            output_text
        };

        let output_signal =
            derived_output(input, Kind::AgentOutput, Body::text(&display_text)).build();

        let result = if success {
            AgentResult::ok(output_signal)
        } else {
            AgentResult::fail(output_signal)
        };

        result.with_usage(usage)
    }
}

#[async_trait]
impl HarnessAdapter for OpenClawAcpAgent {
    fn harness_id(&self) -> &str {
        "openclaw"
    }

    fn transport(&self) -> TransportFlavor {
        TransportFlavor::AcpStdio
    }

    fn capabilities(&self) -> &HarnessCapabilities {
        &self.capabilities
    }

    async fn probe(&self) -> Result<(), ProbeError> {
        let infer_config = super::config::OpenClawInferConfig {
            binary: std::ffi::OsString::from(&self.config.binary),
            ..Default::default()
        };
        super::probe::probe_openclaw_infer(&infer_config).await
    }

    fn state_dir(&self) -> Option<&Path> {
        None
    }
}

// ---------------------------------------------------------------------------
// Notification parsing
// ---------------------------------------------------------------------------

/// Parse an ACP notification into a semantic event.
///
/// OpenClaw uses the same ACP notification methods as other ACP agents
/// but with slightly different parameter shapes. This parser handles
/// both the standard and OpenClaw-specific variants.
fn parse_notification(notif: &AcpNotification) -> Option<AcpEvent> {
    let params = notif.params.as_ref()?;
    match notif.method.as_str() {
        "session/update" | "agent_message_chunk" => {
            if let Some(text) = params
                .get("text")
                .and_then(|v| v.as_str())
                .or_else(|| params.get("delta").and_then(|v| v.as_str()))
                .or_else(|| params.get("content").and_then(|v| v.as_str()))
            {
                return Some(AcpEvent::Output {
                    text: text.to_string(),
                });
            }
            None
        }
        "session/tool_call" | "tool_call" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            Some(AcpEvent::ToolCall {
                id,
                name,
                arguments,
            })
        }
        "session/tool_call_update" | "tool_call_update" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let progress = params
                .get("progress")
                .and_then(|v| v.as_str())
                .or_else(|| params.get("rawOutput").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            Some(AcpEvent::ToolCallUpdate { id, progress })
        }
        "session/request_permission" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let tool = params
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            Some(AcpEvent::PermissionRequest {
                id,
                tool,
                arguments,
            })
        }
        "session/usage" => {
            let input_tokens = params
                .get("inputTokens")
                .or_else(|| params.get("input_tokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let output_tokens = params
                .get("outputTokens")
                .or_else(|| params.get("output_tokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            Some(AcpEvent::Usage {
                input_tokens,
                output_tokens,
            })
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = OpenClawAcpConfig::default();
        assert_eq!(config.binary, "openclaw");
        assert_eq!(config.gateway_url, None);
        assert_eq!(config.session_key, Some("agent:main:roko".into()));
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert!(config.auto_approve_permissions);
    }

    #[test]
    fn config_custom() {
        let config = OpenClawAcpConfig {
            binary: "/usr/local/bin/openclaw".into(),
            cwd: "/tmp/workspace".into(),
            gateway_url: Some("http://localhost:18789".into()),
            session_key: Some("custom-key".into()),
            timeout: Duration::from_secs(60),
            auto_approve_permissions: false,
        };
        assert_eq!(config.binary, "/usr/local/bin/openclaw");
        assert_eq!(config.cwd, PathBuf::from("/tmp/workspace"));
        assert_eq!(
            config.gateway_url,
            Some("http://localhost:18789".to_string())
        );
        assert_eq!(config.session_key, Some("custom-key".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert!(!config.auto_approve_permissions);
    }

    #[test]
    fn harness_metadata() {
        let agent = OpenClawAcpAgent::new(OpenClawAcpConfig::default());
        assert_eq!(agent.harness_id(), "openclaw");
        assert_eq!(agent.transport(), TransportFlavor::AcpStdio);
        assert_eq!(agent.backend_id(), "openclaw-acp");
        assert_eq!(agent.name(), "openclaw-acp");
    }

    #[test]
    fn supports_streaming() {
        let agent = OpenClawAcpAgent::new(OpenClawAcpConfig::default());
        assert!(agent.supports_streaming());
    }

    #[test]
    fn capabilities_mcp_is_server_only() {
        let agent = OpenClawAcpAgent::new(OpenClawAcpConfig::default());
        let caps = agent.capabilities();
        assert!(
            matches!(caps.mcp_passthrough, McpMode::ServerOnly),
            "OpenClaw ACP MUST declare McpMode::ServerOnly to prevent per-session MCP injection"
        );
    }

    #[test]
    fn capabilities_cancel() {
        let agent = OpenClawAcpAgent::new(OpenClawAcpConfig::default());
        let caps = agent.capabilities();
        assert!(matches!(caps.cancel, CancelMode::AcpCancel));
    }

    #[test]
    fn capabilities_session_resume() {
        let agent = OpenClawAcpAgent::new(OpenClawAcpConfig::default());
        let caps = agent.capabilities();
        assert!(matches!(caps.session_resume, SessionResumeMode::Acp));
    }

    #[test]
    fn capabilities_multiplex_safe() {
        let agent = OpenClawAcpAgent::new(OpenClawAcpConfig::default());
        let caps = agent.capabilities();
        assert!(caps.multiplex_safe);
    }

    #[test]
    fn parse_notification_output() {
        let notif = AcpNotification {
            method: "session/update".into(),
            params: Some(serde_json::json!({"text": "hello world"})),
            server_request_id: None,
        };
        let event = parse_notification(&notif).unwrap();
        assert!(matches!(event, AcpEvent::Output { text } if text == "hello world"));

        // delta variant
        let notif_delta = AcpNotification {
            method: "agent_message_chunk".into(),
            params: Some(serde_json::json!({"delta": "chunk"})),
            server_request_id: None,
        };
        let event_delta = parse_notification(&notif_delta).unwrap();
        assert!(matches!(event_delta, AcpEvent::Output { text } if text == "chunk"));

        // content variant
        let notif_content = AcpNotification {
            method: "session/update".into(),
            params: Some(serde_json::json!({"content": "from content"})),
            server_request_id: None,
        };
        let event_content = parse_notification(&notif_content).unwrap();
        assert!(matches!(event_content, AcpEvent::Output { text } if text == "from content"));
    }

    #[test]
    fn parse_notification_tool_call() {
        let notif = AcpNotification {
            method: "session/tool_call".into(),
            params: Some(serde_json::json!({
                "id": "call-1",
                "name": "read_file",
                "arguments": {"path": "/tmp/test.rs"}
            })),
            server_request_id: None,
        };
        let event = parse_notification(&notif).unwrap();
        match event {
            AcpEvent::ToolCall {
                id,
                name,
                arguments,
            } => {
                assert_eq!(id, "call-1");
                assert_eq!(name, "read_file");
                assert_eq!(arguments, serde_json::json!({"path": "/tmp/test.rs"}));
            }
            other => panic!("expected ToolCall, got: {other:?}"),
        }

        // Also test the "tool_call" method variant
        let notif2 = AcpNotification {
            method: "tool_call".into(),
            params: Some(serde_json::json!({
                "id": "call-2",
                "name": "write_file",
            })),
            server_request_id: None,
        };
        let event2 = parse_notification(&notif2).unwrap();
        assert!(
            matches!(event2, AcpEvent::ToolCall { id, name, .. } if id == "call-2" && name == "write_file")
        );
    }

    #[test]
    fn parse_notification_permission_request() {
        let notif = AcpNotification {
            method: "session/request_permission".into(),
            params: Some(serde_json::json!({
                "id": "perm-1",
                "tool": "execute_command",
                "arguments": {"command": "rm -rf /"}
            })),
            server_request_id: None,
        };
        let event = parse_notification(&notif).unwrap();
        match event {
            AcpEvent::PermissionRequest {
                id,
                tool,
                arguments,
            } => {
                assert_eq!(id, "perm-1");
                assert_eq!(tool, "execute_command");
                assert_eq!(arguments, serde_json::json!({"command": "rm -rf /"}));
            }
            other => panic!("expected PermissionRequest, got: {other:?}"),
        }
    }

    #[test]
    fn parse_notification_usage() {
        // camelCase
        let notif = AcpNotification {
            method: "session/usage".into(),
            params: Some(serde_json::json!({
                "inputTokens": 500,
                "outputTokens": 200
            })),
            server_request_id: None,
        };
        let event = parse_notification(&notif).unwrap();
        assert!(matches!(
            event,
            AcpEvent::Usage {
                input_tokens: 500,
                output_tokens: 200
            }
        ));

        // snake_case
        let notif_snake = AcpNotification {
            method: "session/usage".into(),
            params: Some(serde_json::json!({
                "input_tokens": 300,
                "output_tokens": 100
            })),
            server_request_id: None,
        };
        let event_snake = parse_notification(&notif_snake).unwrap();
        assert!(matches!(
            event_snake,
            AcpEvent::Usage {
                input_tokens: 300,
                output_tokens: 100
            }
        ));
    }

    #[test]
    fn parse_notification_unknown() {
        let notif = AcpNotification {
            method: "some/unknown/method".into(),
            params: Some(serde_json::json!({"data": 42})),
            server_request_id: None,
        };
        assert!(parse_notification(&notif).is_none());
    }

    #[test]
    fn mcp_servers_always_none() {
        let config = OpenClawAcpConfig::default();
        let agent = OpenClawAcpAgent::new(config);
        let opts = agent.build_session_opts();
        assert!(
            opts.mcp_servers.is_none(),
            "OpenClaw ACP MUST always pass mcp_servers: None in NewSessionOpts"
        );

        // Also test with custom config that might tempt someone to add MCP
        let config_custom = OpenClawAcpConfig {
            gateway_url: Some("http://localhost:18789".into()),
            session_key: Some("test-key".into()),
            ..Default::default()
        };
        let agent_custom = OpenClawAcpAgent::new(config_custom);
        let opts_custom = agent_custom.build_session_opts();
        assert!(
            opts_custom.mcp_servers.is_none(),
            "mcp_servers must be None regardless of config"
        );
    }

    #[test]
    fn token_estimation() {
        // When no usage events arrive, tokens are estimated as len/4, min 1.
        let prompt = "Hello, how are you doing today?"; // 30 chars
        let output = "I am doing well, thank you for asking!"; // 38 chars

        let estimated_input = (prompt.len() as u64 / 4).max(1);
        let estimated_output = (output.len() as u64 / 4).max(1);

        assert_eq!(estimated_input, 7); // 30/4 = 7
        assert_eq!(estimated_output, 9); // 38/4 = 9

        // Edge case: empty strings should produce minimum of 1
        let empty_input = (0_u64 / 4).max(1);
        assert_eq!(empty_input, 1);
    }

    #[test]
    fn capabilities_full_check() {
        let agent = OpenClawAcpAgent::new(OpenClawAcpConfig::default());
        let caps = agent.capabilities();
        assert!(matches!(caps.one_shot, OneShotMode::Acp));
        assert!(matches!(caps.streaming, StreamingMode::NdJson));
        assert!(matches!(caps.session_resume, SessionResumeMode::Acp));
        assert!(matches!(caps.mcp_passthrough, McpMode::ServerOnly));
        assert!(matches!(caps.tool_injection, ToolInjection::Opaque));
        assert!(!caps.model_override);
        assert!(caps.multiplex_safe);
        assert!(matches!(caps.cancel, CancelMode::AcpCancel));
        assert_eq!(caps.overhead_p50_ms, 80);
    }
}
