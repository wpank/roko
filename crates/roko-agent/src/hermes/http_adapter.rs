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
use futures::StreamExt as _;
use serde_json::{Map, Value};
use tokio::sync::mpsc;

use crate::agent::{Agent, AgentResult};
use crate::harness::{
    CancelMode, HarnessAdapter, HarnessCapabilities, HarnessService, McpMode, OneShotMode,
    ProbeError, SessionResumeMode, StreamingMode, ToolInjection, TransportFlavor,
};
use crate::http::ReqwestPoster;
use crate::openai_compat_backend::OpenAiCompatLlmBackend;
use crate::tool_loop::{LlmBackend, StreamEvent, TurnConfig, collect_stream_to_response};
use crate::translate::{BackendResponse, RenderedTools, SessionState};
use crate::usage::Usage;
use roko_core::{Body, Context, Kind, Provenance, Signal};

use super::config::HermesConfig;

/// Why a Hermes turn whose reply stopped at the output token limit fails:
/// a cut-off reply is not an answer (gap-fd0c0b).
const TRUNCATED_REPLY: &str =
    "hermes: the model hit its output token limit (finish_reason=length); its reply was cut off";
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
    /// Optional system prompt injected as a system message.
    system_prompt: Option<String>,
    /// Shared HTTP client for post-turn run lookups (token accounting level 2).
    http: reqwest::Client,
    /// Pre-computed capabilities (constant for the lifetime of the adapter).
    capabilities: HarnessCapabilities,
    /// State directory for probe cache, PID files, etc.
    state_dir: PathBuf,
    /// Safety layer for output scrubbing (secret leak prevention).
    safety: crate::safety::SafetyLayer,
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
            system_prompt: None,
            safety: crate::safety::SafetyLayer::with_defaults(),
        }
    }

    /// Set an optional system prompt included in every request.
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = Some(prompt);
        self
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

    /// Build the messages array, optionally prepending a system message.
    fn build_messages(&self, prompt_text: &str) -> Vec<Value> {
        let mut messages = Vec::new();
        if let Some(sp) = &self.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sp,
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt_text,
        }));
        messages
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
    /// producing `StreamEvent::Usage(Usage)`. This method extracts usage
    /// from a non-streaming JSON response with the same OpenAI usage parser,
    /// which counts cached prompt tokens once, as cache reads (bug-b72a37).
    fn extract_usage_from_response(response: &BackendResponse) -> Option<Usage> {
        match response {
            BackendResponse::Json(json) if json.get("usage").is_some() => {
                Some(crate::translate::openai::parse_usage(json))
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
        json.get("usage")?;
        Some(crate::translate::openai::parse_usage(&json))
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
        if let Some(usage) = Self::extract_usage_from_response(response)
            && (usage.input_tokens > 0 || usage.output_tokens > 0)
        {
            return usage;
        }

        // Level 2: post-turn run lookup.
        if let Some(run_id) = run_id
            && let Some(usage) = self.fetch_run_usage(run_id).await
            && (usage.input_tokens > 0 || usage.output_tokens > 0)
        {
            tracing::debug!(run_id, "token accounting: used level-2 (run lookup)");
            return usage;
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

        let messages = self.build_messages(&prompt_text);

        let tools = RenderedTools::JsonArray(serde_json::json!([]));
        let session = SessionState::default();

        match self.backend.send_turn(&messages, &tools, &session).await {
            Ok(response) => {
                let wall_ms = started.elapsed().as_millis() as u64;
                let content = self.safety.scrub_text(&Self::extract_content(&response));
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

                if response.hit_length_limit() {
                    let output = self.build_error_output(input, TRUNCATED_REPLY);
                    return AgentResult::fail(output).with_usage(usage);
                }
                let output = self.build_output(input, &content);
                AgentResult::ok(output).with_usage(usage)
            }
            Err(e) => {
                let mapped = crate::provider::map_provider_error(
                    roko_core::agent::ProviderKind::Hermes,
                    "hermes",
                    self.config.api_key_env.as_deref(),
                    Some(self.config.endpoint.as_str()),
                    &e,
                );
                tracing::debug!(error = %e, "hermes send_turn failed");
                let output = self.build_error_output(input, &mapped.to_string());
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
        event_tx: mpsc::Sender<StreamEvent>,
    ) -> AgentResult {
        let started = Instant::now();

        let prompt_text = match Self::extract_prompt(input) {
            Ok(s) => s,
            Err(e) => {
                return AgentResult::fail(self.build_error_output(input, &e));
            }
        };

        let messages = self.build_messages(&prompt_text);

        let tools = RenderedTools::JsonArray(serde_json::json!([]));
        let session = SessionState::default();
        let config = TurnConfig::default();

        // Attempt the streaming turn via `stream_turn`, then collect.
        let stream = match self
            .backend
            .stream_turn(&messages, &tools, &session, &config)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                let output =
                    self.build_error_output(input, &format!("hermes streaming error: {e}"));
                return AgentResult::fail(output);
            }
        };

        // Hand each event to the caller as it arrives (bug-e139f9), then
        // collect the turn's response from the same events.
        let stream = stream
            .then(move |event| {
                let event_tx = event_tx.clone();
                async move {
                    if let Some(forwarded) = event.as_ref().ok().cloned() {
                        // A caller that stopped listening does not stop the turn.
                        let _ = event_tx.send(forwarded).await;
                    }
                    event
                }
            })
            .boxed();
        let result = collect_stream_to_response(stream, started).await;
        match result {
            Ok(response) => {
                let wall_ms = started.elapsed().as_millis() as u64;
                let content = self.safety.scrub_text(&Self::extract_content(&response));
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

                if response.hit_length_limit() {
                    let output = self.build_error_output(input, TRUNCATED_REPLY);
                    return AgentResult::fail(output).with_usage(usage);
                }
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
        super::probe::probe_hermes_with_limits(
            &self.config.binary,
            Some(&self.config.endpoint),
            self.config.resource_limits.as_ref(),
            self.config.timeout,
        )
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
    use crate::tool_loop::StreamEventKind;
    use roko_core::sse::parse_sse_text;

    /// bug-e139f9: a streaming Hermes HTTP turn hands each event to the
    /// caller as it arrives, and still returns the collected answer.
    #[tokio::test]
    async fn hermes_http_streaming_forwards_each_event() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let sse = include_str!("../../tests/fixtures/hermes/http/chat_basic.sse");
        let response = ResponseTemplate::new(200).set_body_raw(sse, "text/event-stream");
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(response)
            .mount(&server)
            .await;
        let agent = HermesHttpAgent::new(HermesConfig {
            endpoint: server.uri(),
            ..HermesConfig::default()
        });
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text("hello"))
            .build();
        let ctx = Context::at(0);
        let (event_tx, mut event_rx) = mpsc::channel(256);

        let result = agent.run_streaming(&input, &ctx, event_tx).await;

        assert!(result.success);
        let (mut text, mut done) = (String::new(), false);
        while let Ok(event) = event_rx.try_recv() {
            match event.kind {
                StreamEventKind::TextDelta(delta) => text.push_str(&delta),
                StreamEventKind::Done { .. } => done = true,
                _ => {}
            }
        }
        assert_eq!(text, "Hello! I'm Hermes.");
        assert!(done, "the caller sees the turn end");
    }

    /// gap-fd0c0b: a Hermes turn whose reply stopped at the output token
    /// limit fails, saying so, instead of passing off the cut-off text as an
    /// answer. Its usage still counts.
    #[tokio::test]
    async fn hermes_adapter_flags_a_length_truncated_turn() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let sse = concat!(
            "data: {\"id\":\"chatcmpl-hermes-2\",\"choices\":[{\"index\":0,",
            "\"delta\":{\"role\":\"assistant\",\"content\":\"The answer is\"},",
            "\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-hermes-2\",\"choices\":[{\"index\":0,\"delta\":{},",
            "\"finish_reason\":\"length\"}],\"usage\":{\"prompt_tokens\":12,",
            "\"completion_tokens\":64,\"total_tokens\":76}}\n\n",
            "data: [DONE]\n\n",
        );
        let response = ResponseTemplate::new(200).set_body_raw(sse, "text/event-stream");
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(response)
            .mount(&server)
            .await;
        let agent = HermesHttpAgent::new(HermesConfig {
            endpoint: server.uri(),
            ..HermesConfig::default()
        });
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text("hello"))
            .build();
        let (event_tx, _event_rx) = mpsc::channel(256);

        let result = agent.run_streaming(&input, &Context::at(0), event_tx).await;

        assert!(!result.success, "a cut-off reply is no answer");
        let text = result.output.body.as_text().unwrap_or_default();
        assert!(text.contains("output token limit"), "{text}");
        assert!(result.usage.output_tokens > 0, "{:?}", result.usage);
    }

    #[test]
    fn basic_sse_fixture_parses_correctly() {
        let fixture = include_str!("../../tests/fixtures/hermes/http/chat_basic.sse");
        let mut content = String::new();
        let mut saw_done = false;

        for line in fixture.lines() {
            for event in parse_sse_line(line) {
                match &event.kind {
                    StreamEventKind::TextDelta(delta) => content.push_str(delta),
                    StreamEventKind::Done { .. } => saw_done = true,
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

        // Use the shared SSE parser to accumulate complete frames, then
        // dispatch each by its event type. This replaces the previous
        // ad-hoc inline loop that used str::trim (both-ends) instead of
        // the RFC 8895-correct strip_one_space (left-only).
        for frame in parse_sse_text(fixture) {
            if frame.event == "message" {
                // Standard OpenAI-compatible data line.
                for event in parse_sse_line(&format!("data: {}", frame.data)) {
                    if let StreamEventKind::TextDelta(delta) = &event.kind {
                        content.push_str(delta);
                    }
                }
            } else if frame.data != "[DONE]" {
                // Non-standard event — check inspector.
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&frame.data) {
                    if let Some(event) = inspector.inspect(&frame.event, &json) {
                        tool_events.push(event);
                    }
                }
            }
        }

        assert_eq!(content, "Let me check the files.");
        assert_eq!(tool_events.len(), 2);

        match &tool_events[0].kind {
            StreamEventKind::TextDelta(text) => {
                assert!(text.contains("terminal"));
                assert!(text.contains("start"));
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }

        match &tool_events[1].kind {
            StreamEventKind::TextDelta(text) => {
                assert!(text.contains("terminal"));
                assert!(text.contains("done"));
            }
            other => panic!("expected TextDelta, got {other:?}"),
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
        // 3 of the 10 prompt tokens were cached (bug-b72a37).
        assert_eq!(usage.input_tokens, 7);
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
            for event in parse_sse_line(line) {
                match &event.kind {
                    StreamEventKind::TextDelta(delta) => content.push_str(delta),
                    StreamEventKind::Done { .. } => saw_done = true,
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
        assert!(matches!(
            caps.one_shot,
            OneShotMode::HttpJson {
                endpoint: "/v1/chat/completions"
            }
        ));
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
