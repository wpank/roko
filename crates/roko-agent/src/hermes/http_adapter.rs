//! Tier 1 transport: HTTP via Hermes gateway's OpenAI-compatible API.
//!
//! Thin wrapper around the existing [`crate::OpenAiCompatLlmBackend`].
//! Only adds:
//!
//! - Hermes-specific SSE event inspection (`hermes.tool.progress`).
//! - Default model name (`hermes-agent` or the active profile name).
//! - Optional pointer to a [`HermesGatewayService`] for lifecycle.
//! - Hermes-specific request metadata (`source` tag).
//! - 3-level token accounting fallback.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Map, Value};
use tokio::sync::mpsc;

use crate::agent::{Agent, AgentResult};
use crate::harness::{
    CancelMode, HarnessAdapter, HarnessCapabilities, HarnessService, McpMode, OneShotMode,
    ProbeError, SessionResumeMode, StreamingMode, ToolInjection, TransportFlavor,
};
use crate::http::ReqwestPoster;
use crate::openai_compat_backend::OpenAiCompatLlmBackend;
use crate::streaming::StreamChunk;
use crate::tool_loop::LlmBackend;
use crate::translate::{BackendResponse, RenderedTools, SessionState};
use crate::usage::Usage;
use roko_core::{Body, Context, Kind, Provenance, Signal};

use super::config::HermesConfig;
use super::gateway_service::HermesGatewayService;

/// Hermes HTTP adapter.
///
/// Wraps an `OpenAiCompatLlmBackend` configured for the Hermes gateway
/// API server. The backend handles all OpenAI Chat Completions protocol
/// work; this struct adds:
///
/// 1. Hermes-specific defaults (base URL, model name, source tag).
/// 2. Optional `HermesGatewayService` for daemon lifecycle.
/// 3. `HarnessAdapter` implementation for capability negotiation.
/// 4. 3-level token accounting fallback.
pub struct HermesHttpAgent {
    /// The underlying OpenAI-compat backend, fully configured for Hermes.
    backend: OpenAiCompatLlmBackend,
    /// Optional lifecycle service for the Hermes gateway daemon.
    service: Option<Arc<HermesGatewayService>>,
    /// Config snapshot for probe() and capability queries.
    config: HermesConfig,
    /// Human-readable name (e.g., `"hermes-http"`).
    agent_name: String,
    /// Shared HTTP client for post-turn run lookups (token accounting level 2).
    http: reqwest::Client,
    /// Pre-computed capabilities (constant for the lifetime of the adapter).
    capabilities: HarnessCapabilities,
    /// State directory for probe cache, PID files, etc.
    state_dir: PathBuf,
}

impl HermesHttpAgent {
    /// Create a new Hermes HTTP agent from config.
    ///
    /// Resolves the API key from the environment, configures the backend
    /// with Hermes-specific settings.
    #[must_use]
    pub fn new(config: HermesConfig) -> Self {
        let api_key = config.resolve_api_key().unwrap_or_default();
        let model = config
            .model
            .clone()
            .unwrap_or_else(|| "hermes-agent".to_string());

        // Build extra body params for Hermes-specific metadata.
        let mut extra_body = Map::new();
        extra_body.insert(
            "metadata".to_string(),
            serde_json::json!({
                "source": "roko",
            }),
        );

        // Build base URL: ensure it ends with /v1 for chat completions.
        let base_url = {
            let ep = config.endpoint.trim_end_matches('/');
            if ep.ends_with("/v1") {
                ep.to_string()
            } else {
                format!("{ep}/v1")
            }
        };

        let backend = OpenAiCompatLlmBackend::new(&api_key, &model)
            .with_base_url(&base_url)
            .with_timeout_ms(config.timeout.as_millis() as u64)
            .with_provider_kind(roko_core::agent::ProviderKind::Hermes)
            .with_poster(Box::new(ReqwestPoster::new()))
            .with_extra_body_params(extra_body);

        let state_dir = config.effective_state_dir();

        let capabilities = Self::build_capabilities();

        Self {
            backend,
            service: None,
            http: crate::provider::shared_http_client(),
            state_dir,
            capabilities,
            config,
            agent_name: "hermes-http".to_string(),
        }
    }

    /// Attach a lifecycle service for the Hermes gateway daemon.
    ///
    /// If attached, the adapter reports the service via
    /// `HarnessAdapter::service()` and crash recovery can attempt
    /// a gateway restart on mid-turn disconnects.
    #[must_use]
    pub fn with_service(mut self, service: Arc<HermesGatewayService>) -> Self {
        self.service = Some(service);
        self
    }

    /// Returns `true` if the gateway is managed by this adapter
    /// (i.e., a `HermesGatewayService` is attached).
    #[must_use]
    pub fn is_managed(&self) -> bool {
        self.service.is_some()
    }

    /// Build the Hermes-specific capability set.
    fn build_capabilities() -> HarnessCapabilities {
        HarnessCapabilities {
            one_shot: OneShotMode::HttpJson {
                endpoint: "/v1/chat/completions",
            },
            streaming: StreamingMode::SseChatCompletions,
            session_resume: SessionResumeMode::None,
            mcp_passthrough: McpMode::None,
            tool_injection: ToolInjection::PerCallTools,
            model_override: true,
            multiplex_safe: true,
            cancel: CancelMode::HttpEndpoint("/v1/runs/{id}/stop"),
            overhead_p50_ms: 50,
        }
    }

    /// Extract text content from the prompt signal.
    fn extract_prompt(input: &Signal) -> Result<String, String> {
        match input.body.as_text() {
            Ok(s) => Ok(s.to_string()),
            Err(_) => serde_json::to_string(&input.body).map_err(|e| format!("input error: {e}")),
        }
    }

    /// Extract content text from a backend response.
    fn extract_content(response: &BackendResponse) -> String {
        match response {
            BackendResponse::Json(json) => json
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            BackendResponse::Text(t) => t.clone(),
            BackendResponse::StreamJson(events) => {
                // Concatenate content deltas from the stream events.
                events
                    .iter()
                    .filter_map(|j| {
                        j.pointer("/choices/0/delta/content")
                            .and_then(Value::as_str)
                    })
                    .collect::<String>()
            }
        }
    }

    /// Build a success output signal with standard tags.
    fn build_output(&self, input: &Signal, content: &str) -> Signal {
        let model = self.config.model.as_deref().unwrap_or("hermes-agent");
        input
            .derive(Kind::AgentOutput, Body::text(content))
            .provenance(Provenance::agent(&self.agent_name))
            .tag("agent", &self.agent_name)
            .tag("model", model)
            .build()
    }

    /// Build a failure output signal with standard tags.
    fn build_error_output(&self, input: &Signal, error_msg: &str) -> Signal {
        input
            .derive(Kind::AgentOutput, Body::text(error_msg))
            .provenance(Provenance::agent(&self.agent_name))
            .tag("agent", &self.agent_name)
            .tag("failed", "true")
            .build()
    }

    // -- Token accounting: 3-level fallback --

    /// Level 1: Parse `usage` from the Chat Completions response JSON.
    ///
    /// The `OpenAiCompatLlmBackend` already does this via `parse_sse_line()`
    /// producing `StreamChunk::Usage(Usage)`. This method extracts usage
    /// from a non-streaming JSON response.
    fn extract_usage_from_response(response: &BackendResponse) -> Option<Usage> {
        match response {
            BackendResponse::Json(json) => {
                let usage = json.get("usage")?;
                Some(Usage {
                    input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                    output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                    cache_read_tokens: usage
                        .pointer("/prompt_tokens_details/cached_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }

    /// Level 2: After the stream closes, if no usage was received,
    /// `GET /v1/runs/{run_id}` to fetch the run's final usage.
    ///
    /// Requires capturing the `run_id` from stream metadata (the `id`
    /// field in the first SSE chunk).
    async fn fetch_run_usage(&self, run_id: &str) -> Option<Usage> {
        let base = self.config.endpoint.trim_end_matches('/');
        let base = if base.ends_with("/v1") {
            base.to_string()
        } else {
            format!("{base}/v1")
        };
        let url = format!("{base}/runs/{run_id}");
        let bearer = self.config.resolve_api_key().unwrap_or_default();

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&bearer)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let json: Value = resp.json().await.ok()?;
        let usage = json.get("usage")?;
        Some(Usage {
            input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
            cache_read_tokens: usage
                .pointer("/prompt_tokens_details/cached_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            ..Default::default()
        })
    }

    /// Level 3: Estimate from accumulated content character count.
    ///
    /// Uses 4 chars per token (a common approximation for English text).
    #[must_use]
    pub fn estimate_usage(content_chars: usize, prompt_chars: usize) -> Usage {
        let estimated_input = (prompt_chars as u32) / 4;
        let estimated_output = (content_chars as u32) / 4;
        Usage {
            input_tokens: estimated_input,
            output_tokens: estimated_output,
            ..Default::default()
        }
    }

    /// Resolve usage with 3-level fallback.
    ///
    /// 1. Check the response JSON for inline `usage`.
    /// 2. If missing, try `GET /v1/runs/{run_id}`.
    /// 3. If that also fails, estimate from character counts.
    async fn resolve_usage(
        &self,
        response: &BackendResponse,
        run_id: Option<&str>,
        content_chars: usize,
        prompt_chars: usize,
    ) -> Usage {
        // Level 1: inline usage from response.
        if let Some(usage) = Self::extract_usage_from_response(response) {
            if usage.input_tokens > 0 || usage.output_tokens > 0 {
                return usage;
            }
        }

        // Level 2: post-turn run lookup.
        if let Some(run_id) = run_id {
            if let Some(usage) = self.fetch_run_usage(run_id).await {
                if usage.input_tokens > 0 || usage.output_tokens > 0 {
                    tracing::debug!(run_id, "token accounting: used level-2 (run lookup)");
                    return usage;
                }
            }
        }

        // Level 3: character count estimation.
        tracing::debug!(
            content_chars,
            prompt_chars,
            "token accounting: used level-3 (char estimation)"
        );
        Self::estimate_usage(content_chars, prompt_chars)
    }
}

#[async_trait]
impl Agent for HermesHttpAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        let started = Instant::now();

        let prompt_text = match Self::extract_prompt(input) {
            Ok(s) => s,
            Err(e) => {
                return AgentResult::fail(self.build_error_output(input, &e));
            }
        };

        let messages = vec![serde_json::json!({
            "role": "user",
            "content": prompt_text,
        })];

        let tools = RenderedTools::JsonArray(serde_json::json!([]));
        let session = SessionState::default();

        match self.backend.send_turn(&messages, &tools, &session).await {
            Ok(response) => {
                let wall_ms = started.elapsed().as_millis() as u64;
                let content = Self::extract_content(&response);
                let prompt_chars = prompt_text.len();
                let content_chars = content.len();

                // Resolve usage with 3-level fallback.
                let run_id = match &response {
                    BackendResponse::Json(json) => {
                        json.get("id").and_then(Value::as_str).map(String::from)
                    }
                    _ => None,
                };
                let mut usage = self
                    .resolve_usage(&response, run_id.as_deref(), content_chars, prompt_chars)
                    .await;
                usage.wall_ms = wall_ms;

                let output = self.build_output(input, &content);
                AgentResult::ok(output).with_usage(usage)
            }
            Err(e) => {
                let output = self.build_error_output(input, &format!("hermes error: {e}"));
                AgentResult::fail(output)
            }
        }
    }

    fn name(&self) -> &str {
        &self.agent_name
    }

    fn backend_id(&self) -> &'static str {
        "hermes-http"
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

        let prompt_text = match Self::extract_prompt(input) {
            Ok(s) => s,
            Err(e) => {
                return AgentResult::fail(self.build_error_output(input, &e));
            }
        };

        let messages = vec![serde_json::json!({
            "role": "user",
            "content": prompt_text,
        })];

        let tools = RenderedTools::JsonArray(serde_json::json!([]));
        let session = SessionState::default();

        // Attempt the streaming turn.
        let result = self
            .backend
            .send_turn_streaming(&messages, &tools, &session, event_tx.clone())
            .await;

        match result {
            Ok(response) => {
                let wall_ms = started.elapsed().as_millis() as u64;
                let content = Self::extract_content(&response);
                let prompt_chars = prompt_text.len();
                let content_chars = content.len();

                let run_id = match &response {
                    BackendResponse::Json(json) => {
                        json.get("id").and_then(Value::as_str).map(String::from)
                    }
                    _ => None,
                };
                let mut usage = self
                    .resolve_usage(&response, run_id.as_deref(), content_chars, prompt_chars)
                    .await;
                usage.wall_ms = wall_ms;

                let output = self.build_output(input, &content);
                AgentResult::ok(output).with_usage(usage)
            }
            Err(e) => {
                let output =
                    self.build_error_output(input, &format!("hermes streaming error: {e}"));
                AgentResult::fail(output)
            }
        }
    }
}

#[async_trait]
impl HarnessAdapter for HermesHttpAgent {
    fn harness_id(&self) -> &str {
        "hermes"
    }

    fn transport(&self) -> TransportFlavor {
        TransportFlavor::HttpOpenAi
    }

    fn capabilities(&self) -> &HarnessCapabilities {
        &self.capabilities
    }

    async fn probe(&self) -> Result<(), ProbeError> {
        super::probe::probe_hermes(&self.config.binary, Some(&self.config.endpoint))
            .await
            .map(|_| ())
    }

    fn state_dir(&self) -> Option<&Path> {
        Some(&self.state_dir)
    }

    fn service(&self) -> Option<&dyn HarnessService> {
        self.service
            .as_ref()
            .map(|s| s.as_ref() as &dyn HarnessService)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::parse_sse_line;

    #[test]
    fn basic_sse_fixture_parses_correctly() {
        let fixture = include_str!("../../tests/fixtures/hermes/http/chat_basic.sse");
        let mut content = String::new();
        let mut saw_done = false;

        for line in fixture.lines() {
            if let Some(chunk) = parse_sse_line(line) {
                match chunk {
                    StreamChunk::ContentDelta(delta) => content.push_str(&delta),
                    StreamChunk::Done(_) => saw_done = true,
                    _ => {}
                }
            }
        }

        assert_eq!(content, "Hello! I'm Hermes.");
        assert!(saw_done);
    }

    #[test]
    fn tool_progress_fixture_produces_events() {
        let fixture = include_str!("../../tests/fixtures/hermes/http/chat_with_tool_progress.sse");
        let inspector = super::super::tool_progress_inspector::ToolProgressInspector;
        let mut content = String::new();
        let mut tool_events = Vec::new();
        let mut pending_event: Option<String> = None;

        for line in fixture.lines() {
            if let Some(event_name) = line.strip_prefix("event:").map(str::trim) {
                pending_event = Some(event_name.to_string());
            } else if let Some(data) = line.strip_prefix("data:").map(str::trim) {
                if let Some(event_name) = pending_event.take() {
                    // Non-standard event -- check inspector.
                    if data != "[DONE]" {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(chunk) = inspector.inspect(&event_name, &json) {
                                tool_events.push(chunk);
                            }
                        }
                    }
                } else {
                    // Standard data line.
                    if let Some(chunk) = parse_sse_line(line) {
                        if let StreamChunk::ContentDelta(delta) = chunk {
                            content.push_str(&delta);
                        }
                    }
                }
            }
        }

        assert_eq!(content, "Let me check the files.");
        assert_eq!(tool_events.len(), 2);

        match &tool_events[0] {
            StreamChunk::ToolProgress { tool, status } => {
                assert_eq!(tool, "terminal");
                assert_eq!(status, "start");
            }
            other => panic!("expected ToolProgress, got {other:?}"),
        }

        match &tool_events[1] {
            StreamChunk::ToolProgress { tool, status } => {
                assert_eq!(tool, "terminal");
                assert_eq!(status, "done");
            }
            other => panic!("expected ToolProgress, got {other:?}"),
        }
    }

    #[test]
    fn config_default_values() {
        let config = HermesConfig::default();
        assert_eq!(config.endpoint, "http://localhost:8642");
        assert_eq!(config.binary, "hermes");
        assert!(config.model.is_none());
    }

    #[test]
    fn agent_has_correct_metadata() {
        let config = HermesConfig::default();
        let agent = HermesHttpAgent::new(config);
        assert_eq!(agent.name(), "hermes-http");
        assert_eq!(agent.backend_id(), "hermes-http");
        assert!(agent.supports_streaming());
        assert_eq!(agent.harness_id(), "hermes");
        assert_eq!(agent.transport(), TransportFlavor::HttpOpenAi);
    }

    #[test]
    fn service_is_none_by_default() {
        let config = HermesConfig::default();
        let agent = HermesHttpAgent::new(config);
        assert!(agent.service().is_none());
        assert!(!agent.is_managed());
    }

    #[test]
    fn state_dir_is_available() {
        let config = HermesConfig::default();
        let agent = HermesHttpAgent::new(config);
        assert!(agent.state_dir().is_some());
        let dir = agent.state_dir().unwrap();
        assert!(dir.ends_with("state/hermes"));
    }

    #[test]
    fn token_estimation_at_level_3() {
        // 100 chars / 4 = 25 tokens
        let usage = HermesHttpAgent::estimate_usage(100, 200);
        assert_eq!(usage.output_tokens, 25);
        assert_eq!(usage.input_tokens, 50);
    }

    #[test]
    fn extract_usage_from_json_response() {
        let json = serde_json::json!({
            "id": "chatcmpl-123",
            "choices": [{"message": {"content": "hi"}}],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "prompt_tokens_details": {"cached_tokens": 3}
            }
        });
        let response = BackendResponse::Json(json);
        let usage = HermesHttpAgent::extract_usage_from_response(&response).unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
        assert_eq!(usage.cache_read_tokens, 3);
    }

    #[test]
    fn extract_usage_returns_none_for_text_response() {
        let response = BackendResponse::Text("hello".to_string());
        assert!(HermesHttpAgent::extract_usage_from_response(&response).is_none());
    }

    #[test]
    fn disconnect_fixture_has_no_done_marker() {
        let fixture = include_str!("../../tests/fixtures/hermes/http/chat_stream_disconnect.sse");
        let mut saw_done = false;
        let mut content = String::new();

        for line in fixture.lines() {
            if let Some(chunk) = parse_sse_line(line) {
                match chunk {
                    StreamChunk::ContentDelta(delta) => content.push_str(&delta),
                    StreamChunk::Done(_) => saw_done = true,
                    _ => {}
                }
            }
        }

        // The disconnect fixture should NOT have a [DONE] marker.
        assert!(!saw_done);
        // But it should have partial content.
        assert_eq!(content, "I'm working on your request");
    }

    #[test]
    fn capabilities_are_correct() {
        let config = HermesConfig::default();
        let agent = HermesHttpAgent::new(config);
        let caps = agent.capabilities();
        assert!(matches!(caps.streaming, StreamingMode::SseChatCompletions));
        assert!(matches!(caps.one_shot, OneShotMode::HttpJson {
            endpoint: "/v1/chat/completions"
        }));
        assert!(matches!(caps.tool_injection, ToolInjection::PerCallTools));
        assert!(caps.model_override);
        assert!(caps.multiplex_safe);
        assert!(matches!(
            caps.cancel,
            CancelMode::HttpEndpoint("/v1/runs/{id}/stop")
        ));
        assert_eq!(caps.overhead_p50_ms, 50);
    }
}
