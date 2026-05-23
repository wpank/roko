//! Tier 3 transport: ACP over stdio via `hermes acp`.
//!
//! [`HermesAcpAgent`] wraps [`AcpStdioClient`] to communicate with Hermes
//! via ACP JSON-RPC 2.0 over stdio. This provides persistent sessions,
//! streaming notifications, and mid-turn cancellation.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::agent::{Agent, AgentResult};
use crate::chat_types::FinishReason;
use crate::harness::acp_client::{
    AcpEvent, AcpNotification, AcpPromptPayload, AcpStdioClient, NewSessionOpts,
};
use crate::harness::{
    CancelMode, HarnessAdapter, HarnessCapabilities, McpMode, OneShotMode, ProbeError,
    SessionResumeMode, StreamingMode, ToolInjection, TransportFlavor,
};
use crate::streaming::StreamChunk;
use crate::usage::Usage;
use roko_core::{Body, Context, Kind, Provenance, Signal};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for the Hermes ACP agent.
#[derive(Clone, Debug)]
pub struct HermesAcpConfig {
    /// Path or name of the Hermes binary.
    pub binary: String,
    /// Working directory for the Hermes subprocess.
    pub cwd: PathBuf,
    /// Optional session key for session reuse.
    pub session_key: Option<String>,
    /// Optional model hint passed to Hermes.
    pub model_hint: Option<String>,
    /// Timeout for ACP operations.
    pub timeout: Duration,
    /// Optional MCP server configuration to pass to the session.
    pub mcp_servers: Option<serde_json::Value>,
}

impl Default for HermesAcpConfig {
    fn default() -> Self {
        Self {
            binary: "hermes".to_string(),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            session_key: None,
            model_hint: None,
            timeout: Duration::from_secs(120),
            mcp_servers: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// Hermes ACP agent -- Tier 3 transport via ACP JSON-RPC 2.0 over stdio.
///
/// Wraps an [`AcpStdioClient`] behind a `tokio::sync::Mutex` to provide
/// the [`Agent`] trait. Each `run()` call creates a session, sends a
/// prompt, collects notification events, and returns the accumulated
/// output.
pub struct HermesAcpAgent {
    client: tokio::sync::Mutex<AcpStdioClient>,
    config: HermesAcpConfig,
    capabilities: HarnessCapabilities,
    name: String,
}

impl HermesAcpAgent {
    /// Create a new Hermes ACP agent from config.
    #[must_use]
    pub fn new(config: HermesAcpConfig) -> Self {
        let client = AcpStdioClient::hermes(&config.binary, config.cwd.clone());
        Self {
            client: tokio::sync::Mutex::new(client),
            capabilities: Self::build_capabilities(),
            name: "hermes-acp".to_string(),
            config,
        }
    }

    /// Create a Hermes ACP agent with an externally-provided client.
    ///
    /// Primarily useful for testing with mock or pre-configured clients.
    #[must_use]
    pub fn with_config(client: AcpStdioClient, config: HermesAcpConfig) -> Self {
        Self {
            client: tokio::sync::Mutex::new(client),
            capabilities: Self::build_capabilities(),
            name: "hermes-acp".to_string(),
            config,
        }
    }

    /// Build the capability set for Hermes ACP transport.
    fn build_capabilities() -> HarnessCapabilities {
        HarnessCapabilities {
            one_shot: OneShotMode::Acp,
            streaming: StreamingMode::NdJson,
            session_resume: SessionResumeMode::Acp,
            mcp_passthrough: McpMode::PerCall,
            tool_injection: ToolInjection::Opaque,
            model_override: false,
            multiplex_safe: false,
            cancel: CancelMode::AcpCancel,
            overhead_p50_ms: 200,
        }
    }

    /// Extract text content from the prompt signal.
    fn extract_prompt(input: &Signal) -> String {
        match input.body.as_text() {
            Ok(s) => s.to_string(),
            Err(_) => serde_json::to_string(&input.body).unwrap_or_default(),
        }
    }

    /// Estimate token usage from character counts.
    ///
    /// Uses 4 chars per token (common approximation for English text).
    /// Returns at least 1 token for each field.
    fn estimate_usage(prompt_chars: usize, output_chars: usize) -> Usage {
        let input_tokens = std::cmp::max(1, (prompt_chars / 4) as u32);
        let output_tokens = std::cmp::max(1, (output_chars / 4) as u32);
        Usage {
            input_tokens,
            output_tokens,
            ..Default::default()
        }
    }

    /// Build a success output signal.
    fn build_output(&self, input: &Signal, content: &str) -> Signal {
        input
            .derive(Kind::AgentOutput, Body::text(content))
            .provenance(Provenance::agent(&self.name))
            .tag("agent", &self.name)
            .tag("transport", "acp_stdio")
            .build()
    }

    /// Build a failure output signal.
    fn build_error_output(&self, input: &Signal, error_msg: &str) -> Signal {
        input
            .derive(Kind::AgentOutput, Body::text(error_msg))
            .provenance(Provenance::agent(&self.name))
            .tag("agent", &self.name)
            .tag("failed", "true")
            .build()
    }
}

// ---------------------------------------------------------------------------
// Notification parsing
// ---------------------------------------------------------------------------

/// Parse an ACP notification into a typed event.
///
/// This interprets the method and params of notifications emitted by the
/// Hermes ACP subprocess and converts them into domain-specific events.
fn parse_notification(notif: &AcpNotification) -> Option<AcpEvent> {
    let params = notif.params.as_ref()?;
    match notif.method.as_str() {
        "session/update" => {
            // Look for text content in priority order: text, delta, content.
            if let Some(text) = params.get("text").and_then(|v| v.as_str()) {
                return Some(AcpEvent::Output {
                    text: text.to_string(),
                });
            }
            if let Some(delta) = params.get("delta").and_then(|v| v.as_str()) {
                return Some(AcpEvent::Output {
                    text: delta.to_string(),
                });
            }
            if let Some(content) = params.get("content").and_then(|v| v.as_str()) {
                return Some(AcpEvent::Output {
                    text: content.to_string(),
                });
            }
            None
        }
        "session/tool_call" => {
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
        "session/tool_call_update" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let progress = params
                .get("progress")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(AcpEvent::ToolCallUpdate { id, progress })
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
// Agent trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Agent for HermesAcpAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        let started = Instant::now();
        let prompt = Self::extract_prompt(input);

        let mut client = self.client.lock().await;

        // Connect if not alive.
        if !client.is_alive() {
            if let Err(e) = client.connect().await {
                let msg = format!("hermes ACP connect failed: {e}");
                tracing::error!("{msg}");
                return AgentResult::fail(self.build_error_output(input, &msg));
            }
        }

        // Create session.
        let session_id = match client
            .new_session(NewSessionOpts {
                session_key: self.config.session_key.clone(),
                cwd: Some(self.config.cwd.clone()),
                mcp_servers: self.config.mcp_servers.clone(),
                reset: false,
                extra_params: None,
            })
            .await
        {
            Ok(sid) => sid,
            Err(e) => {
                let msg = format!("hermes ACP new_session failed: {e}");
                tracing::error!("{msg}");
                return AgentResult::fail(self.build_error_output(input, &msg));
            }
        };

        // Take notification and turn-done receivers.
        let mut notif_rx = client.take_notification_rx();
        let mut turn_done_rx = client.take_turn_done_rx();

        // Send prompt.
        let request_id = match client
            .send_prompt(
                &session_id,
                AcpPromptPayload {
                    text: prompt.clone(),
                    extra_params: None,
                },
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                // Return receivers before bailing.
                if let Some(rx) = notif_rx.take() {
                    client.return_notification_rx(rx);
                }
                if let Some(rx) = turn_done_rx.take() {
                    client.return_turn_done_rx(rx);
                }
                let msg = format!("hermes ACP send_prompt failed: {e}");
                tracing::error!("{msg}");
                let _ = client.close_session(&session_id).await;
                return AgentResult::fail(self.build_error_output(input, &msg));
            }
        };

        // Event loop: collect output from notifications until turn done.
        let mut output_text = String::new();
        let mut usage_input: Option<u64> = None;
        let mut usage_output: Option<u64> = None;
        let timeout = self.config.timeout;

        if let (Some(mut n_rx), Some(mut td_rx)) = (notif_rx.take(), turn_done_rx.take()) {
            loop {
                tokio::select! {
                    notif = n_rx.recv() => {
                        match notif {
                            Some(n) => {
                                if let Some(event) = parse_notification(&n) {
                                    match event {
                                        AcpEvent::Output { text } => {
                                            output_text.push_str(&text);
                                        }
                                        AcpEvent::Usage { input_tokens, output_tokens } => {
                                            usage_input = Some(input_tokens);
                                            usage_output = Some(output_tokens);
                                        }
                                        AcpEvent::ToolCallUpdate { id, progress } => {
                                            tracing::debug!(
                                                tool_call_id = %id,
                                                progress = %progress,
                                                "hermes ACP tool call update"
                                            );
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            None => {
                                // Notification channel closed.
                                break;
                            }
                        }
                    }
                    done = td_rx.recv() => {
                        match done {
                            Some(done_value) => {
                                // Extract any final text from the turn-done payload.
                                if let Some(text) = done_value
                                    .get("result")
                                    .and_then(|r| r.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    if output_text.is_empty() {
                                        output_text.push_str(text);
                                    }
                                }
                                break;
                            }
                            None => {
                                // Turn-done channel closed.
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(timeout) => {
                        tracing::warn!("hermes ACP turn timed out after {:?}", timeout);
                        break;
                    }
                }
            }

            // Return receivers.
            client.return_notification_rx(n_rx);
            client.return_turn_done_rx(td_rx);
        } else {
            // Fallback: no receivers available, use recv_response directly.
            match client.recv_response(request_id).await {
                Ok(resp) => {
                    if let Some(text) = resp
                        .get("result")
                        .and_then(|r| r.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        output_text.push_str(text);
                    }
                }
                Err(e) => {
                    tracing::warn!("hermes ACP recv_response error: {e}");
                }
            }
        }

        // Close session.
        if let Err(e) = client.close_session(&session_id).await {
            tracing::warn!("hermes ACP close_session error: {e}");
        }

        // Compute usage.
        let wall_ms = started.elapsed().as_millis() as u64;
        let usage = match (usage_input, usage_output) {
            (Some(inp), Some(out)) => Usage {
                input_tokens: inp as u32,
                output_tokens: out as u32,
                wall_ms,
                ..Default::default()
            },
            _ => {
                let mut est = Self::estimate_usage(prompt.len(), output_text.len());
                est.wall_ms = wall_ms;
                est
            }
        };

        let output = self.build_output(input, &output_text);
        AgentResult::ok(output).with_usage(usage)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn backend_id(&self) -> &'static str {
        "hermes-acp"
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
        let started = Instant::now();
        let prompt = Self::extract_prompt(input);

        let mut client = self.client.lock().await;

        // Connect if not alive.
        if !client.is_alive() {
            if let Err(e) = client.connect().await {
                let msg = format!("hermes ACP connect failed: {e}");
                tracing::error!("{msg}");
                let _ = event_tx.send(StreamChunk::Error(msg.clone())).await;
                return AgentResult::fail(self.build_error_output(input, &msg));
            }
        }

        // Create session.
        let session_id = match client
            .new_session(NewSessionOpts {
                session_key: self.config.session_key.clone(),
                cwd: Some(self.config.cwd.clone()),
                mcp_servers: self.config.mcp_servers.clone(),
                reset: false,
                extra_params: None,
            })
            .await
        {
            Ok(sid) => sid,
            Err(e) => {
                let msg = format!("hermes ACP new_session failed: {e}");
                tracing::error!("{msg}");
                let _ = event_tx.send(StreamChunk::Error(msg.clone())).await;
                return AgentResult::fail(self.build_error_output(input, &msg));
            }
        };

        // Take receivers.
        let mut notif_rx = client.take_notification_rx();
        let mut turn_done_rx = client.take_turn_done_rx();

        // Send prompt.
        let request_id = match client
            .send_prompt(
                &session_id,
                AcpPromptPayload {
                    text: prompt.clone(),
                    extra_params: None,
                },
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                if let Some(rx) = notif_rx.take() {
                    client.return_notification_rx(rx);
                }
                if let Some(rx) = turn_done_rx.take() {
                    client.return_turn_done_rx(rx);
                }
                let msg = format!("hermes ACP send_prompt failed: {e}");
                tracing::error!("{msg}");
                let _ = client.close_session(&session_id).await;
                let _ = event_tx.send(StreamChunk::Error(msg.clone())).await;
                return AgentResult::fail(self.build_error_output(input, &msg));
            }
        };

        // Streaming event loop.
        let mut output_text = String::new();
        let mut usage_input: Option<u64> = None;
        let mut usage_output: Option<u64> = None;
        let mut tool_call_index: usize = 0;
        let timeout = self.config.timeout;

        if let (Some(mut n_rx), Some(mut td_rx)) = (notif_rx.take(), turn_done_rx.take()) {
            loop {
                tokio::select! {
                    notif = n_rx.recv() => {
                        match notif {
                            Some(n) => {
                                if let Some(event) = parse_notification(&n) {
                                    match event {
                                        AcpEvent::Output { text } => {
                                            output_text.push_str(&text);
                                            let _ = event_tx
                                                .send(StreamChunk::ContentDelta(text))
                                                .await;
                                        }
                                        AcpEvent::ToolCall { id, name, arguments } => {
                                            let args_str = match &arguments {
                                                serde_json::Value::String(s) => s.clone(),
                                                other => other.to_string(),
                                            };
                                            let _ = event_tx
                                                .send(StreamChunk::ToolCallDelta {
                                                    index: tool_call_index,
                                                    id_delta: Some(id),
                                                    name_delta: Some(name),
                                                    arguments_delta: args_str,
                                                })
                                                .await;
                                            tool_call_index += 1;
                                        }
                                        AcpEvent::ToolCallUpdate { id, progress } => {
                                            let _ = event_tx
                                                .send(StreamChunk::ToolProgress {
                                                    tool: id,
                                                    status: progress,
                                                })
                                                .await;
                                        }
                                        AcpEvent::Usage { input_tokens, output_tokens } => {
                                            usage_input = Some(input_tokens);
                                            usage_output = Some(output_tokens);
                                            let u = Usage {
                                                input_tokens: input_tokens as u32,
                                                output_tokens: output_tokens as u32,
                                                ..Default::default()
                                            };
                                            let _ = event_tx
                                                .send(StreamChunk::Usage(u))
                                                .await;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                    done = td_rx.recv() => {
                        match done {
                            Some(done_value) => {
                                if let Some(text) = done_value
                                    .get("result")
                                    .and_then(|r| r.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    if output_text.is_empty() {
                                        output_text.push_str(text);
                                        let _ = event_tx
                                            .send(StreamChunk::ContentDelta(text.to_string()))
                                            .await;
                                    }
                                }
                                let _ = event_tx
                                    .send(StreamChunk::Done(FinishReason::Stop))
                                    .await;
                                break;
                            }
                            None => {
                                let _ = event_tx
                                    .send(StreamChunk::Done(FinishReason::Stop))
                                    .await;
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(timeout) => {
                        tracing::warn!("hermes ACP streaming turn timed out after {:?}", timeout);
                        let _ = event_tx
                            .send(StreamChunk::Done(FinishReason::Stop))
                            .await;
                        break;
                    }
                }
            }

            client.return_notification_rx(n_rx);
            client.return_turn_done_rx(td_rx);
        } else {
            // Fallback: no receivers.
            match client.recv_response(request_id).await {
                Ok(resp) => {
                    if let Some(text) = resp
                        .get("result")
                        .and_then(|r| r.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        output_text.push_str(text);
                        let _ = event_tx
                            .send(StreamChunk::ContentDelta(text.to_string()))
                            .await;
                    }
                }
                Err(e) => {
                    tracing::warn!("hermes ACP recv_response error: {e}");
                }
            }
            let _ = event_tx.send(StreamChunk::Done(FinishReason::Stop)).await;
        }

        // Close session.
        if let Err(e) = client.close_session(&session_id).await {
            tracing::warn!("hermes ACP close_session error: {e}");
        }

        // Compute usage.
        let wall_ms = started.elapsed().as_millis() as u64;
        let usage = match (usage_input, usage_output) {
            (Some(inp), Some(out)) => Usage {
                input_tokens: inp as u32,
                output_tokens: out as u32,
                wall_ms,
                ..Default::default()
            },
            _ => {
                let mut est = Self::estimate_usage(prompt.len(), output_text.len());
                est.wall_ms = wall_ms;
                est
            }
        };

        let output = self.build_output(input, &output_text);
        AgentResult::ok(output).with_usage(usage)
    }
}

// ---------------------------------------------------------------------------
// HarnessAdapter trait
// ---------------------------------------------------------------------------

#[async_trait]
impl HarnessAdapter for HermesAcpAgent {
    fn harness_id(&self) -> &str {
        "hermes"
    }

    fn transport(&self) -> TransportFlavor {
        TransportFlavor::AcpStdio
    }

    fn capabilities(&self) -> &HarnessCapabilities {
        &self.capabilities
    }

    async fn probe(&self) -> Result<(), ProbeError> {
        crate::hermes::probe::probe_hermes(&self.config.binary, None)
            .await
            .map(|_| ())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_config() {
        let config = HermesAcpConfig::default();
        assert_eq!(config.binary, "hermes");
        assert!(config.session_key.is_none());
        assert!(config.model_hint.is_none());
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert!(config.mcp_servers.is_none());
    }

    #[test]
    fn harness_metadata() {
        let config = HermesAcpConfig::default();
        let agent = HermesAcpAgent::new(config);
        assert_eq!(agent.harness_id(), "hermes");
        assert_eq!(agent.transport(), TransportFlavor::AcpStdio);
        assert_eq!(agent.backend_id(), "hermes-acp");
        assert_eq!(agent.name(), "hermes-acp");
        assert!(agent.supports_streaming());
    }

    #[test]
    fn capabilities_streaming() {
        let config = HermesAcpConfig::default();
        let agent = HermesAcpAgent::new(config);
        let caps = agent.capabilities();
        assert!(matches!(caps.streaming, StreamingMode::NdJson));
    }

    #[test]
    fn capabilities_session_resume() {
        let config = HermesAcpConfig::default();
        let agent = HermesAcpAgent::new(config);
        let caps = agent.capabilities();
        assert!(matches!(caps.session_resume, SessionResumeMode::Acp));
    }

    #[test]
    fn capabilities_mcp() {
        let config = HermesAcpConfig::default();
        let agent = HermesAcpAgent::new(config);
        let caps = agent.capabilities();
        assert!(matches!(caps.mcp_passthrough, McpMode::PerCall));
    }

    #[test]
    fn capabilities_cancel() {
        let config = HermesAcpConfig::default();
        let agent = HermesAcpAgent::new(config);
        let caps = agent.capabilities();
        assert!(matches!(caps.cancel, CancelMode::AcpCancel));
    }

    #[test]
    fn parse_notification_output() {
        // Test text field
        let notif = AcpNotification {
            method: "session/update".to_string(),
            params: Some(json!({"text": "hello world"})),
            server_request_id: None,
        };
        let event = parse_notification(&notif).unwrap();
        match event {
            AcpEvent::Output { text } => assert_eq!(text, "hello world"),
            other => panic!("expected Output, got {other:?}"),
        }

        // Test delta field
        let notif_delta = AcpNotification {
            method: "session/update".to_string(),
            params: Some(json!({"delta": "chunk"})),
            server_request_id: None,
        };
        let event_delta = parse_notification(&notif_delta).unwrap();
        match event_delta {
            AcpEvent::Output { text } => assert_eq!(text, "chunk"),
            other => panic!("expected Output, got {other:?}"),
        }

        // Test content field
        let notif_content = AcpNotification {
            method: "session/update".to_string(),
            params: Some(json!({"content": "body text"})),
            server_request_id: None,
        };
        let event_content = parse_notification(&notif_content).unwrap();
        match event_content {
            AcpEvent::Output { text } => assert_eq!(text, "body text"),
            other => panic!("expected Output, got {other:?}"),
        }
    }

    #[test]
    fn parse_notification_tool_call() {
        let notif = AcpNotification {
            method: "session/tool_call".to_string(),
            params: Some(json!({
                "id": "tc-1",
                "name": "read_file",
                "arguments": {"path": "/tmp/foo.txt"}
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
                assert_eq!(id, "tc-1");
                assert_eq!(name, "read_file");
                assert_eq!(arguments["path"], "/tmp/foo.txt");
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn parse_notification_usage() {
        // camelCase variant
        let notif = AcpNotification {
            method: "session/usage".to_string(),
            params: Some(json!({"inputTokens": 100, "outputTokens": 50})),
            server_request_id: None,
        };
        let event = parse_notification(&notif).unwrap();
        match event {
            AcpEvent::Usage {
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(input_tokens, 100);
                assert_eq!(output_tokens, 50);
            }
            other => panic!("expected Usage, got {other:?}"),
        }

        // snake_case variant
        let notif_snake = AcpNotification {
            method: "session/usage".to_string(),
            params: Some(json!({"input_tokens": 200, "output_tokens": 75})),
            server_request_id: None,
        };
        let event_snake = parse_notification(&notif_snake).unwrap();
        match event_snake {
            AcpEvent::Usage {
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(input_tokens, 200);
                assert_eq!(output_tokens, 75);
            }
            other => panic!("expected Usage, got {other:?}"),
        }
    }

    #[test]
    fn parse_notification_unknown_method() {
        let notif = AcpNotification {
            method: "session/unknown_event".to_string(),
            params: Some(json!({"foo": "bar"})),
            server_request_id: None,
        };
        assert!(parse_notification(&notif).is_none());
    }

    #[test]
    fn token_estimation() {
        // 100 chars prompt, 200 chars output
        let usage = HermesAcpAgent::estimate_usage(100, 200);
        assert_eq!(usage.input_tokens, 25);
        assert_eq!(usage.output_tokens, 50);

        // Very small inputs get minimum of 1
        let usage_small = HermesAcpAgent::estimate_usage(1, 1);
        assert_eq!(usage_small.input_tokens, 1);
        assert_eq!(usage_small.output_tokens, 1);

        // Zero-length inputs also get minimum of 1
        let usage_zero = HermesAcpAgent::estimate_usage(0, 0);
        assert_eq!(usage_zero.input_tokens, 1);
        assert_eq!(usage_zero.output_tokens, 1);
    }
}
