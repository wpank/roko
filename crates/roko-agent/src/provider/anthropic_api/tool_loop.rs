// STATUS: GATED — Anthropic Messages API tool loop backend.
// Reachable via kind = "anthropic_api" in roko.toml but no default model
// profile uses this kind. To activate: add [providers.anthropic-api] with
// kind = "anthropic_api" and ANTHROPIC_API_KEY. See parent module docs.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::Agent;
use crate::claude_agent::{AnthropicTool, DEFAULT_ANTHROPIC_VERSION, DEFAULT_BASE_URL};
use crate::http::{HttpPoster, ReqwestPoster};
use crate::model_call_service::{ProviderOutcomeRecorder, provider_error_kind};
use crate::provider::openai_compat::tool_registry_for_options;
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderError, ProviderSemaphores, build_tool_dispatcher,
    map_provider_error, tool_loop_max_iterations_for_profile,
};
use crate::rate_limit::ProviderRateLimiter;
use crate::tool_loop::{LlmBackend, LlmError, MultimodalInputFormat, ToolLoop, ToolLoopAgent};
use crate::translate::{
    BackendResponse, RenderedResults, RenderedTools, SessionState, Translator, TranslatorError,
};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::defaults::{DEFAULT_MAX_OUTPUT_TOKENS, DEFAULT_REQUEST_TIMEOUT_MS};
use roko_core::tool::{ToolCall, ToolDef, ToolFormat, ToolResult};

pub(super) fn create_tool_loop_agent(
    api_key: String,
    provider: &ProviderConfig,
    model: &ModelProfile,
    options: &AgentOptions,
) -> Result<Box<dyn Agent>, AgentCreationError> {
    let (registry, tools, resolver) = tool_registry_for_options(model, options)?;
    let dispatcher = build_tool_dispatcher(registry, resolver);
    let translator: Arc<dyn Translator> = Arc::new(AnthropicTranslator);
    let backend = create_tool_loop_backend_with_api_key(
        api_key,
        provider,
        model,
        options,
        Box::new(ReqwestPoster::new()),
    )?;

    let tool_loop = ToolLoop::new(translator, dispatcher, backend)
        .with_max_iterations(tool_loop_max_iterations_for_profile(Some(model)))
        .with_context_token_limit(usize::try_from(model.context_window).unwrap_or(usize::MAX))
        .with_model_profile(model.clone());

    let name = if options.name.is_empty() {
        format!("anthropic-tool-loop:{}", model.slug)
    } else {
        options.name.clone()
    };

    let mut agent = ToolLoopAgent::new(tool_loop)
        .with_tools(tools)
        .with_name(name)
        .with_input_messages(options.input_messages.clone())
        .with_multimodal_input_format(MultimodalInputFormat::Anthropic);
    if let Some(prompt) = &options.system_prompt {
        agent = agent.with_system_prompt(prompt.clone());
    }
    if let Some(ref dir) = options.working_dir {
        agent = agent.with_worktree_path(dir.clone());
    }
    if let Some(root) = options.effective_immune_root() {
        agent = agent.with_immune_root(root);
    }

    Ok(Box::new(agent))
}

pub(crate) fn create_tool_loop_backend(
    provider: &ProviderConfig,
    model: &ModelProfile,
    options: &AgentOptions,
    poster: Box<dyn HttpPoster>,
) -> Result<Arc<dyn LlmBackend>, AgentCreationError> {
    let api_key = provider.resolve_api_key().ok_or_else(|| {
        AgentCreationError::MissingApiKey(provider.api_key_env.clone().unwrap_or_default())
    })?;
    create_tool_loop_backend_with_api_key(api_key, provider, model, options, poster)
}

/// Create an Anthropic Messages API backend + translator pair from a raw API key.
///
/// This is the lightweight entry point for callers that have an API key but
/// no `ProviderConfig` / `ModelProfile` (e.g. `roko run`).
pub fn create_anthropic_backend_simple(
    api_key: String,
    model: &str,
    timeout_ms: u64,
) -> (Arc<dyn LlmBackend>, Arc<dyn Translator>) {
    let backend = AnthropicMessagesBackend::new(api_key, model).with_timeout_ms(timeout_ms);
    let translator: Arc<dyn Translator> = Arc::new(AnthropicTranslator);
    (Arc::new(backend), translator)
}

/// Create an Anthropic Messages backend that shares the caller's canonical
/// provider limiter and circuit-breaker outcome recorder.
///
/// The backend applies both dependencies to every HTTP turn in a native tool
/// loop, rather than only once around the outer multi-turn agent invocation.
pub fn create_anthropic_backend_with_runtime(
    api_key: String,
    model: &str,
    provider_id: &str,
    timeout_ms: u64,
    rate_limiter: Arc<ProviderRateLimiter>,
    outcome_recorder: Arc<dyn ProviderOutcomeRecorder>,
) -> (Arc<dyn LlmBackend>, Arc<dyn Translator>) {
    let backend = AnthropicMessagesBackend::new(api_key, model)
        .with_provider_id(provider_id)
        .with_timeout_ms(timeout_ms)
        .with_rate_limiter(rate_limiter)
        .with_outcome_recorder(outcome_recorder);
    let translator: Arc<dyn Translator> = Arc::new(AnthropicTranslator);
    (Arc::new(backend), translator)
}

fn create_tool_loop_backend_with_api_key(
    api_key: String,
    provider: &ProviderConfig,
    model: &ModelProfile,
    options: &AgentOptions,
    poster: Box<dyn HttpPoster>,
) -> Result<Arc<dyn LlmBackend>, AgentCreationError> {
    let timeout_ms = options
        .timeout_ms
        .or(provider.timeout_ms)
        .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS);

    let mut backend = AnthropicMessagesBackend::new(api_key, model.slug.clone())
        .with_provider_id(model.provider.clone())
        .with_base_url(super::AnthropicApiAdapter::base_url(provider))
        .with_timeout_ms(timeout_ms)
        .with_max_tokens(
            model
                .max_output
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS),
        )
        .with_extra_headers(provider.extra_headers.clone().unwrap_or_default())
        .with_poster(poster);

    if let Some(ref env_var) = provider.api_key_env {
        backend = backend.with_api_key_env(env_var.clone());
    }
    if let Some(provider_semaphores) = options.provider_semaphores.clone() {
        backend = backend.with_provider_semaphores(provider_semaphores);
    }
    if let Some(rate_limiter) = options.rate_limiter.clone() {
        backend = backend.with_rate_limiter(rate_limiter);
    }

    Ok(Arc::new(backend))
}

#[derive(Debug, Default, Clone, Copy)]
struct AnthropicTranslator;

impl Translator for AnthropicTranslator {
    fn format(&self) -> ToolFormat {
        ToolFormat::AnthropicBlocks
    }

    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools {
        let definitions: Vec<AnthropicTool> = tools
            .iter()
            .map(|tool| {
                AnthropicTool::new(
                    tool.name.clone(),
                    tool.description.clone(),
                    tool.parameters.as_value().clone(),
                )
            })
            .collect();

        RenderedTools::JsonArray(json!(definitions))
    }

    fn parse_calls(&self, response: &BackendResponse) -> Result<Vec<ToolCall>, TranslatorError> {
        let BackendResponse::Json(json) = response else {
            return Err(TranslatorError::Malformed("expected json".into()));
        };

        let Some(blocks) = json.get("content").and_then(Value::as_array) else {
            return Ok(Vec::new());
        };

        let mut calls = Vec::new();
        for block in blocks {
            if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                continue;
            }

            let id = block.get("id").and_then(Value::as_str).unwrap_or_default();
            let name = block
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| TranslatorError::Malformed("missing tool_use.name".into()))?
                .to_string();
            let input = block.get("input").cloned().unwrap_or_else(|| json!({}));

            calls.push(ToolCall::new(id, name, input));
        }

        Ok(calls)
    }

    fn render_results(&self, results: &[(ToolCall, ToolResult)]) -> RenderedResults {
        let messages: Vec<Value> = results
            .iter()
            .map(|(call, result)| {
                json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": call.id.clone(),
                        "content": tool_result_content(result),
                        "is_error": matches!(result, ToolResult::Err(_)),
                    }]
                })
            })
            .collect();

        RenderedResults::JsonMessages(json!(messages))
    }

    fn render_assistant_message(&self, response: &BackendResponse) -> Option<Value> {
        let BackendResponse::Json(json) = response else {
            return None;
        };

        let content = json.get("content")?.clone();
        Some(json!({
            "role": "assistant",
            "content": content,
        }))
    }
}

fn tool_result_content(result: &ToolResult) -> Value {
    match result {
        ToolResult::Ok {
            content,
            is_structured,
            ..
        } if *is_structured => {
            serde_json::from_str(content).unwrap_or_else(|_| Value::String(content.clone()))
        }
        ToolResult::Ok { content, .. } => Value::String(content.clone()),
        ToolResult::Err(err) => Value::String(err.to_string()),
    }
}

struct AnthropicMessagesBackend {
    api_key: String,
    model: String,
    provider_id: String,
    base_url: String,
    timeout_ms: u64,
    max_tokens: u32,
    extra_headers: Vec<(String, String)>,
    provider_semaphores: Option<Arc<ProviderSemaphores>>,
    rate_limiter: Option<Arc<ProviderRateLimiter>>,
    outcome_recorder: Option<Arc<dyn ProviderOutcomeRecorder>>,
    poster: Box<dyn HttpPoster>,
    /// Environment variable name for the API key (used in error messages).
    api_key_env: Option<String>,
}

impl AnthropicMessagesBackend {
    fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let model = model.into();
        Self {
            api_key: api_key.into(),
            provider_id: model.clone(),
            model,
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            max_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
            extra_headers: Vec::new(),
            provider_semaphores: None,
            rate_limiter: None,
            outcome_recorder: None,
            poster: Box::new(ReqwestPoster::new()),
            api_key_env: None,
        }
    }

    fn with_api_key_env(mut self, env_var: impl Into<String>) -> Self {
        self.api_key_env = Some(env_var.into());
        self
    }

    fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = provider_id.into();
        self
    }

    fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    fn with_extra_headers(mut self, extra_headers: HashMap<String, String>) -> Self {
        let mut extra_headers: Vec<(String, String)> = extra_headers.into_iter().collect();
        extra_headers.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        self.extra_headers = extra_headers;
        self
    }

    fn with_provider_semaphores(mut self, provider_semaphores: Arc<ProviderSemaphores>) -> Self {
        self.provider_semaphores = Some(provider_semaphores);
        self
    }

    fn with_rate_limiter(mut self, rate_limiter: Arc<ProviderRateLimiter>) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    fn with_outcome_recorder(mut self, outcome_recorder: Arc<dyn ProviderOutcomeRecorder>) -> Self {
        self.outcome_recorder = Some(outcome_recorder);
        self
    }

    fn with_poster(mut self, poster: Box<dyn HttpPoster>) -> Self {
        self.poster = poster;
        self
    }

    fn endpoint(&self) -> String {
        format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
    }

    fn headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("x-api-key".to_owned(), self.api_key.clone()),
            (
                "anthropic-version".to_owned(),
                DEFAULT_ANTHROPIC_VERSION.to_owned(),
            ),
        ];
        headers.extend(self.extra_headers.iter().cloned());
        headers
    }

    fn build_body(&self, messages: &[Value], tools: &RenderedTools) -> Result<Vec<u8>, LlmError> {
        let RenderedTools::JsonArray(tools) = tools else {
            return Err(LlmError::Backend("expected json tool array".into()));
        };

        let mut system_prompt = Vec::new();
        let mut anthropic_messages = Vec::with_capacity(messages.len());

        for message in messages {
            let Some(role) = message.get("role").and_then(Value::as_str) else {
                anthropic_messages.push(message.clone());
                continue;
            };

            if role == "system" {
                if let Some(content) = message.get("content") {
                    if let Some(text) = content.as_str() {
                        system_prompt.push(text.to_string());
                    } else if let Some(parts) = content.as_array() {
                        system_prompt.extend(parts.iter().filter_map(|part| {
                            (part.get("type").and_then(Value::as_str) == Some("text"))
                                .then(|| part.get("text").and_then(Value::as_str))
                                .flatten()
                                .map(str::to_string)
                        }));
                    }
                }
                continue;
            }

            anthropic_messages.push(message.clone());
        }

        crate::translate::claude::inject_cache_markers(&mut anthropic_messages);

        let mut body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": anthropic_messages,
            "tools": tools,
        });

        if !system_prompt.is_empty() {
            let mut system = Value::String(system_prompt.join("\n"));
            let _ = crate::translate::claude::inject_cache_markers_into_content(&mut system);
            body["system"] = system;
        }

        serde_json::to_vec(&body).map_err(|err| LlmError::Backend(format!("serialize: {err}")))
    }

    fn normalize_response(raw: Value) -> Value {
        let content = raw.get("content").cloned().unwrap_or_else(|| json!([]));
        let text = content_as_text(&content);
        let usage = raw
            .get("usage")
            .map(normalize_usage)
            .unwrap_or_else(|| json!({}));

        json!({
            "id": raw.get("id").cloned().unwrap_or(Value::Null),
            "model": raw.get("model").cloned().unwrap_or(Value::Null),
            "stop_reason": raw.get("stop_reason").cloned().unwrap_or(Value::Null),
            "content": content,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": text,
                }
            }],
            "usage": usage,
        })
    }

    fn record_failure(&self, error: &LlmError) {
        let Some(recorder) = &self.outcome_recorder else {
            return;
        };
        let kind = match error {
            LlmError::Provider(ProviderError::RateLimit { .. }) => "rate_limit",
            LlmError::Provider(ProviderError::AuthFailure) => "auth_failure",
            LlmError::Provider(ProviderError::Timeout) | LlmError::Timeout(_) => "timeout",
            LlmError::Provider(ProviderError::ServerError(_)) => "server_error",
            LlmError::Provider(ProviderError::ContentPolicy) => "content_policy",
            LlmError::Provider(ProviderError::ContextOverflow) => "context_overflow",
            LlmError::Provider(ProviderError::ModelNotFound) => "model_not_found",
            LlmError::Provider(ProviderError::Other(message))
            | LlmError::Backend(message)
            | LlmError::Network(message) => provider_error_kind(message),
            LlmError::RetriesExhausted => "retries_exhausted",
        };
        recorder.record_provider_failure(&self.provider_id, kind);
    }
}

#[async_trait]
impl LlmBackend for AnthropicMessagesBackend {
    async fn send_turn(
        &self,
        messages: &[Value],
        tools: &RenderedTools,
        _session: &SessionState,
    ) -> Result<BackendResponse, LlmError> {
        let _permit = match (&self.provider_id, &self.provider_semaphores) {
            (provider_id, Some(provider_semaphores)) => {
                provider_semaphores.acquire(provider_id).await.ok()
            }
            _ => None,
        };

        let body_bytes = self.build_body(messages, tools)?;
        if let Some(limiter) = &self.rate_limiter
            && limiter.try_acquire(&self.provider_id).await.is_err()
        {
            return Err(LlmError::Provider(ProviderError::RateLimit {
                retry_after_ms: None,
            }));
        }

        let raw = match self
            .poster
            .post_json(
                &self.endpoint(),
                &self.headers(),
                &body_bytes,
                self.timeout_ms,
            )
            .await
        {
            Ok(raw) => raw,
            Err(err) => {
                // Propagate Retry-After from 429/529 as structured error so the
                // retry policy (E01-T12) can honour the provider's wait hint.
                let mapped = if let Some(s) = err.status
                    && (s == 429 || s == 529)
                {
                    LlmError::Provider(ProviderError::RateLimit {
                        retry_after_ms: err.retry_after_secs.map(|sec| sec * 1000),
                    })
                } else {
                    let decorated = map_provider_error(
                        ProviderKind::AnthropicApi,
                        &self.provider_id,
                        self.api_key_env.as_deref(),
                        Some(&self.base_url),
                        &err,
                    );
                    LlmError::Network(decorated)
                };
                self.record_failure(&mapped);
                return Err(mapped);
            }
        };

        let json: Value = match serde_json::from_str(&raw) {
            Ok(json) => json,
            Err(err) => {
                let mapped = LlmError::Backend(format!("parse response: {err}"));
                self.record_failure(&mapped);
                return Err(mapped);
            }
        };

        let response = BackendResponse::Json(Self::normalize_response(json));
        if let Some(limiter) = &self.rate_limiter {
            let usage = response.extract_usage();
            limiter
                .record_tokens(
                    &self.provider_id,
                    usage
                        .input_tokens
                        .saturating_add(usage.output_tokens)
                        .into(),
                )
                .await;
        }
        if let Some(recorder) = &self.outcome_recorder {
            recorder.record_provider_success(&self.provider_id);
        }
        Ok(response)
    }

    fn extract_session(&self, response: &BackendResponse) -> SessionState {
        match response {
            BackendResponse::Json(json) => SessionState {
                session_id: None,
                thread_id: None,
                conversation_id: json.get("id").and_then(Value::as_str).map(str::to_string),
            },
            BackendResponse::StreamJson(_) | BackendResponse::Text(_) => SessionState::default(),
        }
    }

    fn backend_id(&self) -> &'static str {
        "claude_api"
    }
}

fn content_as_text(content: &Value) -> String {
    let Some(blocks) = content.as_array() else {
        return content.as_str().unwrap_or_default().to_string();
    };

    let mut text = String::new();
    for block in blocks {
        let Some(block_type) = block.get("type").and_then(Value::as_str) else {
            continue;
        };

        if block_type == "text"
            && let Some(block_text) = block.get("text").and_then(Value::as_str)
        {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(block_text);
        }
    }
    text
}

fn normalize_usage(usage: &Value) -> Value {
    let input_tokens = usage
        .get("input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let output_tokens = usage
        .get("output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cached_tokens = usage
        .get("cache_read_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    json!({
        "prompt_tokens": input_tokens,
        "completion_tokens": output_tokens,
        "total_tokens": input_tokens + output_tokens,
        "prompt_tokens_details": {
            "cached_tokens": cached_tokens,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpPostError;
    use crate::provider::AgentOptions;
    use crate::provider::openai_compat::tool_registry_for_options;
    use crate::tool_loop::{LlmBackend, ToolLoop};
    use crate::translate::Translator;
    use roko_core::tool::{
        ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolHandler, ToolPermission,
    };
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        url: String,
        headers: Vec<(String, String)>,
        body: Value,
        timeout_ms: u64,
    }

    struct MockPoster {
        responses: Mutex<VecDeque<Result<String, HttpPostError>>>,
        requests: Mutex<Vec<RecordedRequest>>,
    }

    #[derive(Default)]
    struct RecordingOutcomes {
        successes: Mutex<Vec<String>>,
        failures: Mutex<Vec<(String, String)>>,
    }

    impl ProviderOutcomeRecorder for RecordingOutcomes {
        fn record_provider_success(&self, provider_id: &str) {
            self.successes
                .lock()
                .expect("success lock")
                .push(provider_id.to_string());
        }

        fn record_provider_failure(&self, provider_id: &str, error_kind: &str) {
            self.failures
                .lock()
                .expect("failure lock")
                .push((provider_id.to_string(), error_kind.to_string()));
        }
    }

    impl MockPoster {
        fn new(responses: Vec<Result<String, HttpPostError>>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl HttpPoster for MockPoster {
        async fn post_json(
            &self,
            url: &str,
            headers: &[(String, String)],
            body: &[u8],
            timeout_ms: u64,
        ) -> Result<String, HttpPostError> {
            let body: Value = serde_json::from_slice(body).expect("request body must be json");
            self.requests
                .lock()
                .expect("requests lock")
                .push(RecordedRequest {
                    url: url.to_string(),
                    headers: headers.to_vec(),
                    body,
                    timeout_ms,
                });
            self.responses
                .lock()
                .expect("responses lock")
                .pop_front()
                .expect("queued response")
        }
    }

    struct EchoHandler;

    #[async_trait]
    impl ToolHandler for EchoHandler {
        fn name(&self) -> &str {
            "echo"
        }

        async fn execute(&self, call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            ToolResult::text(call.arguments.to_string())
        }
    }

    fn tool(name: &str) -> ToolDef {
        ToolDef::new(
            name,
            "test tool",
            ToolCategory::Meta,
            ToolPermission::read_only(),
        )
        .with_concurrency(ToolConcurrency::Parallel)
    }

    #[test]
    fn translator_renders_native_tools_and_results() {
        let tools = [tool("read_file")];
        let RenderedTools::JsonArray(rendered) = AnthropicTranslator.render_tools(&tools) else {
            panic!("expected JsonArray");
        };
        assert_eq!(rendered[0]["name"], "read_file");

        let call = ToolCall::new("call-1", "read_file", json!({"path": "x"}));
        let rendered = AnthropicTranslator.render_results(&[(call, ToolResult::text("ok"))]);
        let RenderedResults::JsonMessages(msgs) = rendered else {
            panic!("expected JsonMessages");
        };
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"][0]["type"], "tool_result");
        assert_eq!(msgs[0]["content"][0]["tool_use_id"], "call-1");
    }

    #[test]
    fn backend_lifts_structured_system_and_preserves_image_blocks() {
        let backend = AnthropicMessagesBackend::new("test-key", "claude-sonnet-4-6");
        let messages = vec![
            json!({
                "role": "system",
                "content": [{"type": "text", "text": "structured system"}]
            }),
            json!({
                "role": "user",
                "content": [
                    {"type": "text", "text": "before"},
                    {"type": "image", "source": {
                        "type": "base64", "media_type": "image/png", "data": "aGVsbG8="
                    }},
                    {"type": "text", "text": "after"}
                ]
            }),
        ];
        let body = backend
            .build_body(&messages, &RenderedTools::JsonArray(json!([])))
            .expect("build body");
        let request: Value = serde_json::from_slice(&body).expect("request json");

        assert_eq!(request["system"], "structured system");
        assert_eq!(request["messages"].as_array().map(Vec::len), Some(1));
        assert_eq!(request["messages"][0]["content"][0]["text"], "before");
        assert_eq!(
            request["messages"][0]["content"][1]["source"]["data"],
            "aGVsbG8="
        );
        assert_eq!(request["messages"][0]["content"][2]["text"], "after");
    }

    #[tokio::test]
    async fn backend_normalizes_anthropic_responses_for_tool_loop() {
        let poster = MockPoster::new(vec![Ok(json!({
            "id": "msg_1",
            "model": "claude-sonnet-4-6",
            "stop_reason": "tool_use",
            "content": [
                { "type": "text", "text": "working" },
                { "type": "tool_use", "id": "t1", "name": "echo", "input": { "value": 1 } }
            ],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 22,
                "cache_read_input_tokens": 3,
                "cache_creation_input_tokens": 4
            }
        })
        .to_string())]);
        let backend = AnthropicMessagesBackend::new("test-key", "claude-sonnet-4-6")
            .with_base_url("https://example.test")
            .with_poster(Box::new(poster));

        let response = backend
            .send_turn(
                &[json!({"role": "user", "content": "hi"})],
                &RenderedTools::JsonArray(json!([])),
                &SessionState::default(),
            )
            .await
            .expect("send turn");

        let text = response.extract_text();
        assert_eq!(text, "working");
        assert_eq!(response.extract_usage().input_tokens, 11);
        assert_eq!(response.extract_usage().cache_read_tokens, 3);
        let calls = AnthropicTranslator
            .parse_calls(&response)
            .expect("parse calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "echo");
    }

    #[tokio::test]
    async fn native_backend_applies_shared_rate_and_outcome_hooks_per_turn() {
        use crate::rate_limit::ProviderLimits;

        let success = json!({
            "id": "msg_ok",
            "model": "claude-sonnet-4-6",
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "ok" }],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 22,
                "cache_read_input_tokens": 0,
                "cache_creation_input_tokens": 0
            }
        })
        .to_string();
        let poster = MockPoster::new(vec![
            Ok(success),
            Err(HttpPostError::http(429, "rate limited")),
        ]);
        let limiter = Arc::new(ProviderRateLimiter::with_provider_limits(
            60_000,
            HashMap::from([(
                "anthropic-primary".to_string(),
                ProviderLimits {
                    rpm: 60_000,
                    tpm: 1_000_000,
                },
            )]),
        ));
        let outcomes = Arc::new(RecordingOutcomes::default());
        let backend = AnthropicMessagesBackend::new("test-key", "claude-sonnet-4-6")
            .with_provider_id("anthropic-primary")
            .with_base_url("https://example.test")
            .with_rate_limiter(Arc::clone(&limiter))
            .with_outcome_recorder(outcomes.clone())
            .with_poster(Box::new(poster));
        let messages = [json!({"role": "user", "content": "hi"})];
        let tools = RenderedTools::JsonArray(json!([]));

        backend
            .send_turn(&messages, &tools, &SessionState::default())
            .await
            .expect("successful turn");
        let error = backend
            .send_turn(&messages, &tools, &SessionState::default())
            .await
            .expect_err("rate-limited turn");

        assert!(matches!(
            error,
            LlmError::Provider(ProviderError::RateLimit { .. })
        ));
        assert_eq!(
            outcomes.successes.lock().expect("success lock").as_slice(),
            ["anthropic-primary"]
        );
        assert_eq!(
            outcomes.failures.lock().expect("failure lock").as_slice(),
            [("anthropic-primary".to_string(), "rate_limit".to_string())]
        );
        let snapshot = limiter
            .snapshot()
            .into_iter()
            .find(|snapshot| snapshot.provider_id == "anthropic-primary")
            .expect("configured provider snapshot");
        assert_eq!(snapshot.rpm_used, 2);
        assert_eq!(snapshot.tpm_used, 33);
    }

    #[tokio::test]
    async fn tool_loop_agent_executes_anthropic_tool_calls() {
        let first_response = json!({
            "id": "msg_1",
            "model": "claude-sonnet-4-6",
            "stop_reason": "tool_use",
            "content": [
                { "type": "tool_use", "id": "t1", "name": "ls", "input": {} }
            ],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 1,
                "cache_read_input_tokens": 0,
                "cache_creation_input_tokens": 0
            }
        })
        .to_string();
        let second_response = json!({
            "id": "msg_2",
            "model": "claude-sonnet-4-6",
            "stop_reason": "end_turn",
            "content": [
                { "type": "text", "text": "anthropic-final" }
            ],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 2,
                "cache_read_input_tokens": 1,
                "cache_creation_input_tokens": 0
            }
        })
        .to_string();
        let poster = MockPoster::new(vec![Ok(first_response), Ok(second_response)]);

        let (registry, tools, resolver) = tool_registry_for_options(
            &ModelProfile {
                provider: "anthropic".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                context_window: 200_000,
                supports_tools: true,
                tool_format: "anthropic_blocks".to_string(),
                ..Default::default()
            },
            &AgentOptions {
                tools: Some("ls".to_string()),
                ..Default::default()
            },
        )
        .expect("tools");
        let dispatcher = build_tool_dispatcher(registry, resolver);
        let backend = AnthropicMessagesBackend::new("test-key", "claude-sonnet-4-6")
            .with_base_url("https://example.test")
            .with_poster(Box::new(poster));
        let tool_loop = ToolLoop::new(Arc::new(AnthropicTranslator), dispatcher, Arc::new(backend));
        let ctx = ToolContext::testing(".");

        let output = tool_loop.run("system", "please use ls", &tools, &ctx).await;

        assert_eq!(output.final_text, "anthropic-final");
        assert_eq!(output.tool_calls.len(), 1);
        assert_eq!(output.tool_calls[0].name, "ls");
    }

    #[test]
    fn content_as_text_joins_text_blocks_only() {
        let content = json!([
            { "type": "text", "text": "hello" },
            { "type": "tool_use", "id": "x", "name": "echo", "input": {} },
            { "type": "text", "text": "world" }
        ]);
        assert_eq!(content_as_text(&content), "hello\nworld");
    }

    #[tokio::test]
    async fn retry_after_header_429_anthropic_maps_to_provider_rate_limit() {
        let poster = MockPoster::new(vec![Err(HttpPostError::http_with_retry_after(
            429,
            "rate limited",
            Some(30),
        ))]);
        let backend = AnthropicMessagesBackend::new("test-key", "claude-sonnet-4-6")
            .with_base_url("https://example.test")
            .with_poster(Box::new(poster));
        let err = backend
            .send_turn(
                &[json!({"role": "user", "content": "hi"})],
                &RenderedTools::JsonArray(json!([])),
                &SessionState::default(),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                LlmError::Provider(ProviderError::RateLimit {
                    retry_after_ms: Some(30_000)
                })
            ),
            "expected Provider(RateLimit {{ 30_000ms }}), got {err:?}"
        );
    }

    #[tokio::test]
    async fn retry_after_header_absent_anthropic_produces_rate_limit_none() {
        let poster = MockPoster::new(vec![Err(HttpPostError::http(429, "no header"))]);
        let backend = AnthropicMessagesBackend::new("test-key", "claude-sonnet-4-6")
            .with_base_url("https://example.test")
            .with_poster(Box::new(poster));
        let err = backend
            .send_turn(
                &[json!({"role": "user", "content": "hi"})],
                &RenderedTools::JsonArray(json!([])),
                &SessionState::default(),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                LlmError::Provider(ProviderError::RateLimit {
                    retry_after_ms: None
                })
            ),
            "expected Provider(RateLimit {{ None }}), got {err:?}"
        );
    }
}
