//! `OpenAiCompatLlmBackend` — HTTP adapter implementing [`LlmBackend`]
//! for OpenAI-compatible chat-completions endpoints.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Map, Value};

use crate::http::{HttpPoster, ReqwestPoster};
use crate::model_call_service::{ProviderOutcomeRecorder, provider_error_kind};
use crate::multimodal::wire_messages_contain_images;
use crate::provider::{ProviderError, map_provider_error};
use crate::rate_limit::ProviderRateLimiter;
use crate::streaming::{SseError, SseLine, UNKNOWN_FINISH_REASON, parse_sse_frame};
use crate::tool_loop::{LlmBackend, LlmError, StreamEvent, StreamEventKind};
use crate::translate::{BackendResponse, RenderedTools, SessionState, convert_images_for_openai};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::OpenAiApi;
use roko_core::defaults::{DEFAULT_PROVIDER_RPM, DEFAULT_REQUEST_TIMEOUT_MS};
use roko_core::sse::extract_sse_data;

use crate::openai_responses::{ReasoningLedger, ResponsesRequest, ResponsesTurn};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone, Default)]
struct StreamResponseMetadata {
    response_id: Option<String>,
    session_id: Option<String>,
    thread_id: Option<String>,
}

/// How long a call the rate limiter holds back waits for a slot at its
/// provider before it fails as rate-limited, without calling the provider
/// (bug-33a61f). The limit a refusal taught lapses
/// [`LEARNED_LIMIT_HOLDS_FOR`](crate::rate_limit::LEARNED_LIMIT_HOLDS_FOR)
/// after the provider's last refusal, so a hold rarely lasts this long.
pub const HELD_CALL_WAIT: Duration = Duration::from_secs(300);

/// How often a held call looks for its slot (bug-33a61f).
const HELD_CALL_POLL: Duration = Duration::from_millis(250);

/// How often a streamed Responses call that is reasoning says it is alive,
/// as an empty reasoning delta the stall watchdog counts as progress: the
/// model sends nothing until its reasoning ends (gap-1ba0ac).
pub const REASONING_HEARTBEAT: Duration = Duration::from_secs(30);

/// How long a streamed Responses call may stay silent while its model
/// reasons, against the provider's wait for each next chunk otherwise
/// (gap-1ba0ac). The stream's own ceiling still holds.
pub const REASONING_SILENCE: Duration = Duration::from_secs(10 * 60);

/// The process-wide rate limiter every OpenAI-compatible backend shares,
/// unless given its own ([`OpenAiCompatLlmBackend::with_rate_limiter`]): it
/// learns per endpoint ([`chat_endpoint`]) how many calls at once a provider
/// takes, so parallel tasks share a provider's rate limit (bug-4b6844).
#[must_use]
pub fn shared_rate_limiter() -> Arc<ProviderRateLimiter> {
    static SHARED_RATE_LIMITER: OnceLock<Arc<ProviderRateLimiter>> = OnceLock::new();
    Arc::clone(
        SHARED_RATE_LIMITER
            .get_or_init(|| Arc::new(ProviderRateLimiter::new(DEFAULT_PROVIDER_RPM))),
    )
}

/// Classify an [`HttpPostError`] from `provider` into the appropriate
/// [`LlmError`] variant.
///
/// Status codes are mapped as follows:
/// - 429 / 529 -> `LlmError::Provider(ProviderError::RateLimit { .. })`, or
///   `ProviderExhausted` when the body names a billing or usage-window problem
///   or `Retry-After` spans a usage window
/// - 500..=599 -> `LlmError::Provider(ProviderError::ServerError(status))`
/// - 401       -> `LlmError::Provider(ProviderError::AuthFailure)`
/// - any other 4xx -> the provider refused the request, in its own words
///   ([`classify_http_refusal`](crate::provider::error_classify::classify_http_refusal)):
///   `ProviderExhausted` for a 402 or a can't-pay answer (bug-390e4b),
///   `ContextOverflow`, `Timeout` for a 408, else `RequestRefused`
///   (bug-35b190)
/// - Everything else (transport errors with no status) -> `LlmError::Network`
fn classify_http_error(e: crate::http::HttpPostError, provider: &str) -> LlmError {
    use crate::provider::ProviderError;
    match e.status {
        Some(429 | 529) => {
            LlmError::Provider(crate::provider::error_classify::classify_rate_limited(
                &e.message,
                e.retry_after_secs.map(|sec| sec * 1000),
            ))
        }
        Some(s @ 500..=599) => LlmError::Provider(ProviderError::ServerError(s)),
        Some(401) => LlmError::Provider(ProviderError::AuthFailure),
        Some(status @ 400..=499) => LlmError::Provider(
            crate::provider::error_classify::classify_http_refusal(provider, status, &e.message),
        ),
        _ => LlmError::Network(e.to_string()),
    }
}

/// The turn error a provider `error` object in `provider`'s stream becomes:
/// classified like an HTTP error when its code is a status (rate limit, auth,
/// server, refusal), otherwise a backend error that names it (backlog 1111).
fn sse_error_to_llm_error(error: &SseError, provider: &str) -> LlmError {
    match error.status() {
        Some(status) => classify_http_error(
            crate::http::HttpPostError::http(status, error.to_string()),
            provider,
        ),
        None => LlmError::Backend(error.to_string()),
    }
}

/// What one OpenAI-compatible turn's SSE lines become: every chunk event, an
/// error object as a failed turn, and a `Done` with
/// [`UNKNOWN_FINISH_REASON`] for a stream that ends (or sends `[DONE]`)
/// without naming a finish reason, never a made-up `stop` (backlog 1111).
#[derive(Default)]
struct SseTurn {
    /// A chunk named a finish reason.
    finish_seen: bool,
    /// A `Done` was sent.
    done_sent: bool,
}

impl SseTurn {
    /// The items to send for one SSE line.
    fn line(
        &mut self,
        line: &str,
        provider: &str,
        model: &str,
    ) -> Vec<Result<StreamEvent, LlmError>> {
        match parse_sse_frame(line) {
            SseLine::Events(events) => {
                if events
                    .iter()
                    .any(|event| matches!(event.kind, StreamEventKind::Done { .. }))
                {
                    self.finish_seen = true;
                    self.done_sent = true;
                }
                events.into_iter().map(Ok).collect()
            }
            // `[DONE]` after a real finish reason adds nothing.
            SseLine::Done if self.finish_seen => Vec::new(),
            SseLine::Done => vec![Ok(self.unknown_finish(provider, model, "[DONE]"))],
            SseLine::Error(error) => vec![Err(sse_error_to_llm_error(&error, provider))],
        }
    }

    /// The `Done` to send when the stream ended, unless one was sent.
    fn end(&mut self, provider: &str, model: &str) -> Option<StreamEvent> {
        (!self.done_sent).then(|| self.unknown_finish(provider, model, "end of stream"))
    }

    fn unknown_finish(&mut self, provider: &str, model: &str, at: &str) -> StreamEvent {
        tracing::warn!(
            provider,
            model,
            at,
            "provider stream named no finish reason; reporting it as unknown"
        );
        self.done_sent = true;
        StreamEvent::now(StreamEventKind::Done {
            finish_reason: UNKNOWN_FINISH_REASON.to_string(),
        })
    }
}

/// What a streamed turn's SSE lines are read as: chat-completions chunks, or
/// Responses events (gap-1ba0ac).
enum TurnLines {
    Chat(SseTurn),
    Responses(ResponsesTurn),
}

impl TurnLines {
    /// The items to send for one SSE line.
    fn line(
        &mut self,
        line: &str,
        provider: &str,
        model: &str,
    ) -> Vec<Result<StreamEvent, LlmError>> {
        match self {
            Self::Chat(turn) => turn.line(line, provider, model),
            Self::Responses(turn) => turn
                .line(line)
                .into_iter()
                .map(|item| item.map_err(|error| sse_error_to_llm_error(&error, provider)))
                .collect(),
        }
    }

    /// The `Done` to send when the stream ended, unless one was sent.
    fn end(&mut self, provider: &str, model: &str) -> Option<StreamEvent> {
        match self {
            Self::Chat(turn) => turn.end(provider, model),
            Self::Responses(turn) => turn.end(),
        }
    }

    /// Whether the model is reasoning, and sends nothing until it is done.
    fn reasoning(&self) -> bool {
        matches!(self, Self::Responses(turn) if turn.reasoning_open())
    }
}

/// How a streamed call waits for its next chunk while its model reasons and
/// sends nothing until its reasoning ends (gap-1ba0ac).
#[derive(Debug, Clone, Copy)]
struct ReasoningWait {
    /// How often to say the call is alive ([`REASONING_HEARTBEAT`]).
    heartbeat: Duration,
    /// How long to wait in all ([`REASONING_SILENCE`], or the provider's
    /// wait for a chunk when longer).
    limit: Duration,
    /// When the call started: the stream still ends at its ceiling.
    request_start: std::time::Instant,
}

impl ReasoningWait {
    /// Wait for `chunk`, saying on `tx` every heartbeat that the call is
    /// alive, as an empty reasoning delta the stall watchdog counts as
    /// progress. `None` when the wait ran out, its error sent on `tx`, or
    /// `tx` closed.
    async fn chunk<T>(
        self,
        chunk: impl std::future::Future<Output = T>,
        tx: &tokio::sync::mpsc::Sender<Result<StreamEvent, LlmError>>,
        endpoint: &str,
    ) -> Option<T> {
        let deadline = tokio::time::Instant::now()
            + crate::streaming::chunk_wait(self.limit, self.request_start);
        tokio::pin!(chunk);
        loop {
            let beat = (tokio::time::Instant::now() + self.heartbeat).min(deadline);
            tokio::select! {
                chunk = &mut chunk => return Some(chunk),
                () = tokio::time::sleep_until(beat) => {
                    if beat >= deadline {
                        let error = crate::streaming::chunk_timeout(self.limit, self.request_start);
                        tracing::warn!(
                            endpoint = %endpoint,
                            %error,
                            "streamed call cut while its model reasoned"
                        );
                        let _ = tx.send(Err(error)).await;
                        return None;
                    }
                    let alive = StreamEvent::now(StreamEventKind::ReasoningDelta(String::new()));
                    if tx.send(Ok(alive)).await.is_err() {
                        return None;
                    }
                }
            }
        }
    }
}

fn compute_headers(api_key: &str, extra_headers: &[(String, String)]) -> Vec<(String, String)> {
    let mut headers = Vec::with_capacity(2 + extra_headers.len());
    headers.push(("Content-Type".to_string(), "application/json".to_string()));
    if !api_key.is_empty() {
        headers.push(("Authorization".to_string(), format!("Bearer {api_key}")));
    }
    headers.extend(extra_headers.iter().cloned());
    headers
}

/// HTTP adapter for OpenAI-compatible `/chat/completions` endpoints.
pub struct OpenAiCompatLlmBackend {
    api_key: String,
    model: String,
    provider_id: String,
    base_url: String,
    timeout_ms: u64,
    max_tokens: Option<u32>,
    extra_headers: Vec<(String, String)>,
    extra_body_params: Map<String, Value>,
    rate_limiter: Arc<ProviderRateLimiter>,
    poster: Box<dyn HttpPoster>,
    /// Pre-computed HTTP headers (Content-Type + Auth + extras).
    computed_headers: Vec<(String, String)>,
    /// When true, omit `session_id`, `thread_id`, and `conversation_id` from
    /// request bodies. Strict OpenAI-compatible providers (e.g. Cerebras)
    /// reject unknown top-level fields.
    skip_session_fields: bool,
    /// When true, include `"parallel_tool_calls": false` in request bodies.
    /// Small models (e.g. Llama 3.1 8B via Cerebras) cannot reliably handle
    /// multiple tool calls in a single turn.
    disable_parallel_tool_calls: bool,
    /// When true, normalize `content: ""` to `content: null` on assistant
    /// messages that carry `tool_calls`. Strict providers (e.g. Cerebras)
    /// reject empty-string content alongside tool calls.
    normalize_tool_call_content: bool,
    /// Time-to-first-token timeout in milliseconds for streaming requests.
    /// When set, the first `response.chunk()` call is wrapped with
    /// `tokio::time::timeout` so slow providers fail fast.
    ttft_timeout_ms: Option<u64>,
    /// When true, emit `max_completion_tokens` instead of `max_tokens` in
    /// request bodies. Required for newer OpenAI models (o1, o3, gpt-5.x).
    use_max_completion_tokens: bool,
    /// When true, streaming requests carry `stream_options.include_usage`:
    /// OpenAI streams a usage chunk only when asked (backlog 2101).
    stream_usage: bool,
    /// Provider kind used for error mapping to user-friendly messages.
    provider_kind: ProviderKind,
    /// Environment variable name holding the API key (for error messages).
    api_key_env: Option<String>,
    /// Optional metric registry for emitting TTFT and request duration histograms.
    metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
    /// When true, image blocks in incoming messages are preserved as
    /// `image_url` content parts (data URIs). When false, image input fails
    /// before provider I/O.
    supports_vision: bool,
    /// Optional circuit-breaker outcome recorder.
    outcome_recorder: Option<Arc<dyn ProviderOutcomeRecorder>>,
    /// Where a call that waits for a slot at its provider says it is queued
    /// (bug-33a61f).
    live_output: Option<crate::live_output::LiveOutput>,
    /// How long a held call waits for its slot ([`HELD_CALL_WAIT`]).
    held_call_wait: Duration,
    /// The OpenAI API the backend calls (gap-1ba0ac).
    wire: OpenAiApi,
    /// The run's Responses turns, whose reasoning each next request replays.
    ledger: Arc<ReasoningLedger>,
    /// [`REASONING_HEARTBEAT`] and [`REASONING_SILENCE`].
    reasoning_heartbeat: Duration,
    reasoning_silence: Duration,
}

impl OpenAiCompatLlmBackend {
    /// Construct a backend for `model` with default URL and timeout.
    #[must_use]
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let model = model.into();
        let api_key = api_key.into();
        let computed_headers = compute_headers(&api_key, &[]);
        Self {
            api_key,
            provider_id: model.clone(),
            model,
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            max_tokens: None,
            extra_headers: Vec::new(),
            extra_body_params: Map::new(),
            rate_limiter: shared_rate_limiter(),
            poster: Box::new(ReqwestPoster::new()),
            computed_headers,
            skip_session_fields: false,
            disable_parallel_tool_calls: false,
            normalize_tool_call_content: false,
            ttft_timeout_ms: None,
            use_max_completion_tokens: false,
            stream_usage: true,
            provider_kind: ProviderKind::OpenAiCompat,
            api_key_env: None,
            metrics: None,
            supports_vision: false,
            outcome_recorder: None,
            live_output: None,
            held_call_wait: HELD_CALL_WAIT,
            wire: OpenAiApi::ChatCompletions,
            ledger: Arc::default(),
            reasoning_heartbeat: REASONING_HEARTBEAT,
            reasoning_silence: REASONING_SILENCE,
        }
    }

    /// Attach a metric registry for emitting TTFT and request duration histograms.
    #[must_use]
    pub fn with_metrics(mut self, registry: Arc<roko_core::obs::metrics::MetricRegistry>) -> Self {
        self.metrics = Some(registry);
        self
    }

    /// Control whether image blocks in incoming messages are passed through as
    /// `image_url` data-URI content parts (`true`) or rejected (`false`).
    ///
    /// Must match the model's `supports_vision` capability. Defaults to `false`.
    #[must_use]
    pub const fn with_supports_vision(mut self, supports_vision: bool) -> Self {
        self.supports_vision = supports_vision;
        self
    }

    /// Override the provider identifier used for request throttling.
    #[must_use]
    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = provider_id.into();
        self
    }

    /// Override the provider base URL.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Override the per-turn timeout in milliseconds.
    #[must_use]
    pub const fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Override the max output tokens sent on every request.
    #[must_use]
    pub const fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Inject additional HTTP headers on every request.
    #[must_use]
    pub fn with_extra_headers(mut self, extra_headers: HashMap<String, String>) -> Self {
        let mut extra_headers: Vec<(String, String)> = extra_headers.into_iter().collect();
        extra_headers.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        self.extra_headers = extra_headers;
        self.computed_headers = compute_headers(&self.api_key, &self.extra_headers);
        self
    }

    /// Inject additional JSON fields into the outbound request body.
    #[must_use]
    pub fn with_extra_body_params(mut self, extra_body_params: Map<String, Value>) -> Self {
        self.extra_body_params = extra_body_params;
        self
    }

    /// Override the shared provider rate limiter.
    #[must_use]
    pub fn with_rate_limiter(mut self, rate_limiter: Arc<ProviderRateLimiter>) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }

    /// Say on `live_output`, as queued, while a call waits for a slot at its
    /// provider, so the stall watchdog and the try's time bound skip the wait
    /// (bug-33a61f).
    #[must_use]
    pub fn with_live_output(mut self, live_output: Option<crate::live_output::LiveOutput>) -> Self {
        self.live_output = live_output;
        self
    }

    /// How long a held call waits for its slot before it fails as
    /// rate-limited ([`HELD_CALL_WAIT`] by default).
    #[must_use]
    pub const fn with_held_call_wait(mut self, wait: Duration) -> Self {
        self.held_call_wait = wait;
        self
    }

    /// Call the model through `wire`: chat completions, or OpenAI's
    /// Responses API, where a reasoning model takes a reasoning effort
    /// beside function tools (gap-1ba0ac).
    #[must_use]
    pub const fn with_wire(mut self, wire: OpenAiApi) -> Self {
        self.wire = wire;
        self
    }

    /// While a streamed Responses call reasons, say it is alive every
    /// `heartbeat` and wait up to `silence` for its next chunk
    /// ([`REASONING_HEARTBEAT`], [`REASONING_SILENCE`]).
    #[must_use]
    pub const fn with_reasoning_waits(mut self, heartbeat: Duration, silence: Duration) -> Self {
        self.reasoning_heartbeat = heartbeat;
        self.reasoning_silence = silence;
        self
    }

    /// Inject a custom HTTP poster (for tests or alternate transports).
    #[must_use]
    pub fn with_poster(mut self, poster: Box<dyn HttpPoster>) -> Self {
        self.poster = poster;
        self
    }

    /// Skip `session_id`, `thread_id`, and `conversation_id` in request bodies.
    ///
    /// Strict OpenAI-compatible providers (e.g. Cerebras) reject unknown
    /// top-level fields. Enable this for providers that only accept the
    /// standard OpenAI Chat Completions schema.
    #[must_use]
    pub const fn with_skip_session_fields(mut self, skip: bool) -> Self {
        self.skip_session_fields = skip;
        self
    }

    /// Include `"parallel_tool_calls": false` in request bodies.
    ///
    /// Small models that cannot reliably handle multiple tool calls in a
    /// single turn should have this enabled to constrain the backend to
    /// emitting at most one tool call per response.
    #[must_use]
    pub const fn with_disable_parallel_tool_calls(mut self, disable: bool) -> Self {
        self.disable_parallel_tool_calls = disable;
        self
    }

    /// Normalize `content: ""` to `content: null` on assistant messages
    /// that carry `tool_calls`. Strict providers (e.g. Cerebras) reject
    /// empty-string content alongside tool calls.
    #[must_use]
    pub const fn with_normalize_tool_call_content(mut self, normalize: bool) -> Self {
        self.normalize_tool_call_content = normalize;
        self
    }

    /// Set the time-to-first-token timeout for streaming requests.
    ///
    /// When set, `stream_turn` wraps the first `response.chunk()`
    /// call with `tokio::time::timeout` so slow providers fail fast instead
    /// of hanging until the full request timeout expires.
    #[must_use]
    pub const fn with_ttft_timeout_ms(mut self, ms: Option<u64>) -> Self {
        self.ttft_timeout_ms = ms;
        self
    }

    /// Emit `max_completion_tokens` instead of `max_tokens` in request bodies.
    ///
    /// Newer OpenAI models (o1, o3, gpt-4o, gpt-5.x) reject `max_tokens`
    /// and require `max_completion_tokens`.
    #[must_use]
    pub const fn with_use_max_completion_tokens(mut self, use_it: bool) -> Self {
        self.use_max_completion_tokens = use_it;
        self
    }

    /// Ask for a usage chunk on streaming requests
    /// (`stream_options.include_usage`). Defaults to `true`; turn it off for
    /// a server that rejects the field.
    #[must_use]
    pub const fn with_stream_usage(mut self, stream_usage: bool) -> Self {
        self.stream_usage = stream_usage;
        self
    }

    /// Set the provider kind for user-facing error messages.
    #[must_use]
    pub const fn with_provider_kind(mut self, kind: ProviderKind) -> Self {
        self.provider_kind = kind;
        self
    }

    /// Set the env var name for the API key (used in error messages).
    #[must_use]
    pub fn with_api_key_env(mut self, env_var: impl Into<String>) -> Self {
        self.api_key_env = Some(env_var.into());
        self
    }

    /// Attach a circuit-breaker outcome recorder.
    #[must_use]
    pub fn with_outcome_recorder(mut self, recorder: Arc<dyn ProviderOutcomeRecorder>) -> Self {
        self.outcome_recorder = Some(recorder);
        self
    }

    /// Record a failure into the circuit-breaker outcome recorder.
    fn record_failure(&self, error: &LlmError) {
        let Some(recorder) = &self.outcome_recorder else {
            return;
        };
        let kind = match error {
            LlmError::Provider(ProviderError::RateLimit { .. }) => "rate_limit",
            LlmError::Provider(ProviderError::AuthFailure) => "auth_failure",
            LlmError::Provider(ProviderError::InsufficientCredits) => "insufficient_credits",
            LlmError::Provider(ProviderError::ProviderExhausted { .. }) => "provider_exhausted",
            LlmError::Provider(ProviderError::Timeout) | LlmError::Timeout(_) => "timeout",
            LlmError::Provider(ProviderError::ServerError(_)) => "server_error",
            LlmError::Provider(ProviderError::ContentPolicy) => "content_policy",
            LlmError::Provider(ProviderError::ContextOverflow) => "context_overflow",
            LlmError::Provider(ProviderError::ModelNotFound) => "model_not_found",
            LlmError::Provider(ProviderError::RequestRefused {
                status, message, ..
            }) => crate::provider::error_classify::refusal_class(
                *status,
                &message.to_ascii_lowercase(),
            ),
            LlmError::Provider(ProviderError::Other(message))
            | LlmError::Backend(message)
            | LlmError::Network(message) => provider_error_kind(message),
            LlmError::RetriesExhausted => "retries_exhausted",
        };
        recorder.record_provider_failure(&self.provider_id, kind);
    }

    /// Map a raw error into a user-friendly message using provider context.
    fn decorate_error(&self, raw_err: &dyn std::fmt::Display) -> String {
        map_provider_error(
            self.provider_kind,
            &self.provider_id,
            self.api_key_env.as_deref(),
            Some(&self.base_url),
            raw_err,
        )
    }

    fn endpoint(&self) -> String {
        match self.wire {
            OpenAiApi::ChatCompletions => chat_endpoint(&self.base_url),
            OpenAiApi::Responses => crate::openai_responses::responses_endpoint(&self.base_url),
        }
    }

    /// The key the rate limiter learns the provider's calls at once by: its
    /// chat endpoint, whichever API a call goes to (bug-4b6844).
    fn limiter_key(&self) -> String {
        chat_endpoint(&self.base_url)
    }

    /// Admit a call to the endpoint. While the provider is at the calls at
    /// once it showed it takes, parallel tasks do not all call a provider that
    /// refuses them (bug-4b6844): the call waits for a slot, without calling,
    /// until one of the calls in flight ends, and then goes out in the same
    /// run, its conversation intact (bug-33a61f). The wait is said on the live
    /// output as queued, so the stall watchdog and the try's time bound skip
    /// it. A call held [`Self::with_held_call_wait`] long fails as a rate
    /// limit, still without calling, and the tool loop and dispatch take it
    /// from there as they do a 429.
    async fn admit(&self) -> Result<crate::rate_limit::AdmittedCall, LlmError> {
        let endpoint = self.limiter_key();
        let mut held_since: Option<std::time::Instant> = None;
        let admitted = loop {
            let held = match self.rate_limiter.admit(&endpoint) {
                Ok(call) => break Ok(call),
                Err(held) => held,
            };
            let since = match held_since {
                Some(since) => since,
                None => {
                    tracing::info!(
                        provider = %self.provider_id,
                        in_flight = held.in_flight,
                        limit = held.limit,
                        "holding a call back: its provider rate-limited one with this many calls \
                         in flight, so it waits for one of them to end without calling the \
                         provider"
                    );
                    *held_since.insert(std::time::Instant::now())
                }
            };
            if since.elapsed() >= self.held_call_wait {
                tracing::warn!(
                    provider = %self.provider_id,
                    waited_secs = since.elapsed().as_secs(),
                    "a held call waited its longest for a slot; failing it as rate-limited \
                     without calling the provider"
                );
                break Err(LlmError::Provider(ProviderError::RateLimit {
                    retry_after_ms: None,
                }));
            }
            // Said on every look: the run's own last events, its tool's
            // result among them, reach the live output through the immune
            // boundary's buffer, and one that lands after this would end the
            // queue.
            self.say_queued(true);
            tokio::time::sleep(HELD_CALL_POLL).await;
        };
        if let Some(since) = held_since {
            self.say_queued(false);
            if admitted.is_ok() {
                tracing::info!(
                    provider = %self.provider_id,
                    waited_ms = u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "a held call got its slot"
                );
            }
        }
        admitted
    }

    /// Say on the live output whether a call is `waiting` for a slot at its
    /// provider. The output's channel is never waited on: an event that does
    /// not fit is dropped.
    fn say_queued(&self, waiting: bool) {
        if let Some(live_output) = &self.live_output {
            let _ = live_output
                .sink
                .try_send(crate::live_output::LiveAgentEvent::Queued { waiting });
        }
    }

    /// Note a refusal `error` that is a rate limit with the run's limiter,
    /// which learns from it how many calls at once the provider takes.
    fn note_refusal(&self, error: &LlmError) {
        if matches!(error, LlmError::Provider(ProviderError::RateLimit { .. })) {
            self.rate_limiter.note_rate_limited(&self.limiter_key());
        }
    }

    /// Return a clone of the cached request headers.
    fn headers(&self) -> Vec<(String, String)> {
        self.computed_headers.clone()
    }

    fn build_body(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
        session: &SessionState,
        stream: bool,
    ) -> Result<Vec<u8>, LlmError> {
        // Native tools travel in `tools`. A text tool format (Hermes
        // `<tool_call>` blocks) puts its tool block in the system message
        // instead, and the request carries no `tools` (bug-d0b8b8).
        let (tools, tools_prompt) = match tools {
            RenderedTools::JsonArray(tools) => (Some(tools), None),
            RenderedTools::SystemPromptBlock(block) => (None, Some(block.as_str())),
            RenderedTools::CliFlag(_) => {
                return Err(LlmError::Backend("expected json tool array".into()));
            }
        };
        if !self.supports_vision && wire_messages_contain_images(messages) {
            return Err(LlmError::Backend(
                "model does not support inline image input".into(),
            ));
        }
        // Check if any message has array content (multipart blocks). Array
        // content must be processed to convert or drop image blocks regardless
        // of vision support, since the backend expects the OpenAI wire format.
        let has_array_content = messages
            .iter()
            .any(|msg| msg.get("content").is_some_and(Value::is_array));
        let needs_mutation = has_array_content || self.normalize_tool_call_content;
        let messages = if needs_mutation {
            let mut msgs = messages.to_vec();
            for msg in &mut msgs {
                // Convert/drop image blocks in array-content messages.
                if let Some(content_arr) = msg
                    .get("content")
                    .and_then(Value::as_array)
                    .map(|arr| arr.to_vec())
                {
                    let converted = convert_images_for_openai(&content_arr, self.supports_vision);
                    if let Some(obj) = msg.as_object_mut() {
                        obj.insert("content".to_string(), Value::Array(converted));
                    }
                }
                // Normalize content: "" -> null on assistant messages with tool_calls.
                if self.normalize_tool_call_content
                    && msg.get("role").and_then(Value::as_str) == Some("assistant")
                    && msg.get("tool_calls").is_some_and(Value::is_array)
                    && let Some(content) = msg.get("content")
                    && (content.as_str().is_some_and(str::is_empty) || content.is_null())
                    && let Some(obj) = msg.as_object_mut()
                {
                    obj.insert("content".to_string(), Value::Null);
                }
            }
            std::borrow::Cow::Owned(msgs)
        } else {
            std::borrow::Cow::Borrowed(messages)
        };

        let messages = match tools_prompt {
            Some(block) => std::borrow::Cow::Owned(with_system_block(&messages, block)),
            None => messages,
        };
        // An empty `tools` array is omitted: some endpoints reject it.
        let tools = tools.filter(|tools| tools.as_array().is_none_or(|tools| !tools.is_empty()));

        if self.wire == OpenAiApi::Responses {
            let request = ResponsesRequest {
                model: &self.model,
                messages: &messages,
                tools,
                max_output_tokens: self.max_tokens,
                extra: &self.extra_body_params,
                parallel_tool_calls: (self.disable_parallel_tool_calls && tools.is_some())
                    .then_some(false),
                stream,
            };
            let body = crate::openai_responses::request_body(&request, &self.ledger);
            return serde_json::to_vec(&body)
                .map_err(|e| LlmError::Backend(format!("serialize: {e}")));
        }

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": *messages,
        });

        if let Some(body_obj) = body.as_object_mut() {
            if let Some(tools) = tools {
                body_obj.insert("tools".to_string(), tools.clone());
            }
            if let Some(max_tokens) = self.max_tokens {
                let key = if self.use_max_completion_tokens {
                    "max_completion_tokens"
                } else {
                    "max_tokens"
                };
                body_obj.insert(key.to_string(), Value::from(max_tokens));
            }
            if !self.skip_session_fields {
                if let Some(session_id) = &session.session_id {
                    body_obj.insert("session_id".to_string(), Value::String(session_id.clone()));
                }
                if let Some(thread_id) = &session.thread_id {
                    body_obj.insert("thread_id".to_string(), Value::String(thread_id.clone()));
                }
                if let Some(conversation_id) = &session.conversation_id {
                    body_obj.insert(
                        "conversation_id".to_string(),
                        Value::String(conversation_id.clone()),
                    );
                }
            }
            if self.disable_parallel_tool_calls && tools.is_some() {
                body_obj.insert("parallel_tool_calls".to_string(), Value::Bool(false));
            }
            if stream {
                body_obj.insert("stream".to_string(), Value::Bool(true));
                // Without it OpenAI streams no usage, and the call's spend
                // reads as unknown (backlog 2101).
                if self.stream_usage {
                    body_obj.insert(
                        "stream_options".to_string(),
                        serde_json::json!({ "include_usage": true }),
                    );
                }
            }
            for (key, value) in &self.extra_body_params {
                // Chat completions refuses a reasoning effort beside function
                // tools for the GPT-5.4 family: it reasons with tools only on
                // the Responses API (bug-29014c).
                if key == crate::provider::openai_compat::REASONING_EFFORT && tools.is_some() {
                    continue;
                }
                body_obj.insert(key.clone(), value.clone());
            }
        }

        serde_json::to_vec(&body).map_err(|e| LlmError::Backend(format!("serialize: {e}")))
    }

    fn capture_stream_metadata(line: &[u8], metadata: &mut StreamResponseMetadata) {
        let raw = String::from_utf8_lossy(line);
        let trimmed = raw.trim_end_matches(['\r', '\n']);
        // extract_sse_data strips "data:" with RFC 8895-correct whitespace and
        // returns None for non-data: lines and the [DONE] sentinel.
        let Some(line) = extract_sse_data(trimmed) else {
            return;
        };

        let Ok(json) = serde_json::from_str::<Value>(line) else {
            return;
        };

        if let Some(response_id) = json.get("id").and_then(Value::as_str) {
            metadata.response_id = Some(response_id.to_string());
        }
        if let Some(session_id) = json.get("session_id").and_then(Value::as_str) {
            metadata.session_id = Some(session_id.to_string());
        }
        if let Some(thread_id) = json.get("thread_id").and_then(Value::as_str) {
            metadata.thread_id = Some(thread_id.to_string());
        }
    }

    fn stream_response_to_json(
        response: crate::chat_types::ChatResponse,
        metadata: StreamResponseMetadata,
    ) -> Result<Value, LlmError> {
        let message = response
            .raw_assistant_message
            .clone()
            .unwrap_or_else(|| response.as_assistant_message());
        let message = serde_json::to_value(message)
            .map_err(|e| LlmError::Backend(format!("serialize streamed response: {e}")))?;

        let mut json = serde_json::json!({
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": response.finish_reason.as_str(),
            }],
            "usage": crate::translate::openai::usage_to_wire(&response.usage),
        });
        if let Some(body) = json.as_object_mut() {
            if let Some(response_id) = metadata.response_id {
                body.insert("id".to_string(), Value::String(response_id));
            }
            if let Some(session_id) = metadata.session_id {
                body.insert("session_id".to_string(), Value::String(session_id));
            }
            if let Some(thread_id) = metadata.thread_id {
                body.insert("thread_id".to_string(), Value::String(thread_id));
            }
        }

        Ok(json)
    }
}

/// `messages` with `block` appended to the system message, or put first as a
/// system message of its own when there is none.
fn with_system_block(messages: &[Value], block: &str) -> Vec<Value> {
    let mut messages = messages.to_vec();
    let Some(system) = messages
        .iter()
        .position(|message| message.get("role").and_then(Value::as_str) == Some("system"))
    else {
        messages.insert(0, serde_json::json!({"role": "system", "content": block}));
        return messages;
    };
    let content = match messages[system].get("content") {
        Some(Value::String(content)) if !content.is_empty() => {
            Value::String(format!("{content}\n\n{block}"))
        }
        Some(Value::Array(parts)) => {
            let mut parts = parts.clone();
            parts.push(serde_json::json!({"type": "text", "text": block}));
            Value::Array(parts)
        }
        _ => Value::String(block.to_string()),
    };
    messages[system]["content"] = content;
    messages
}

fn extract_session(response: &Value) -> SessionState {
    SessionState {
        session_id: response
            .pointer("/session_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        thread_id: response
            .pointer("/thread_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        conversation_id: response
            .pointer("/id")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

#[async_trait]
impl LlmBackend for OpenAiCompatLlmBackend {
    async fn send_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
        session: &SessionState,
    ) -> Result<BackendResponse, LlmError> {
        let body_bytes = self.build_body(messages, tools, session, false)?;
        self.rate_limiter.acquire(&self.provider_id).await;
        let _admitted = self.admit().await?;

        let raw = match self
            .poster
            .post_json(
                &self.endpoint(),
                &self.computed_headers,
                &body_bytes,
                self.timeout_ms,
            )
            .await
        {
            Ok(raw) => raw,
            Err(err) => {
                let mapped = classify_http_error(err, &self.provider_id);
                self.note_refusal(&mapped);
                self.record_failure(&mapped);
                return Err(mapped);
            }
        };

        let json: Value = match serde_json::from_str(&raw) {
            Ok(json) => json,
            Err(e) => {
                let mapped = LlmError::Backend(format!("parse response: {e}"));
                self.record_failure(&mapped);
                return Err(mapped);
            }
        };
        // A Responses answer reads as chat completions, and the ledger keeps
        // its reasoning for the next request (gap-1ba0ac).
        let json = if self.wire == OpenAiApi::Responses {
            if let Some(output) = json.get("output").and_then(Value::as_array) {
                self.ledger.record(output);
            }
            match crate::openai_responses::chat_json(&json) {
                Ok(chat) => chat,
                Err(error) => {
                    self.record_failure(&error);
                    return Err(error);
                }
            }
        } else {
            json
        };

        if let Some(recorder) = &self.outcome_recorder {
            recorder.record_provider_success(&self.provider_id);
        }
        Ok(BackendResponse::Json(json))
    }

    async fn stream_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
        session: &SessionState,
        config: &crate::tool_loop::TurnConfig,
    ) -> Result<
        futures::stream::BoxStream<'static, Result<crate::tool_loop::StreamEvent, LlmError>>,
        LlmError,
    > {
        let body_bytes = self.build_body(messages, tools, session, true)?;
        self.rate_limiter.acquire(&self.provider_id).await;
        let admitted = self.admit().await?;

        let mut req = crate::provider::shared_http_client().post(self.endpoint());
        for (key, value) in &self.computed_headers {
            req = req.header(key.as_str(), value.as_str());
        }

        // Capture request start for TTFT and total duration measurement.
        let request_start = std::time::Instant::now();

        // The request has no total timeout: `request_timeout` bounds the wait
        // for the response and each silence in its stream, and the stream
        // ends at `MAX_STREAM_DURATION` (bug-91796d).
        let wait = config.request_timeout;
        let response = tokio::time::timeout(wait, req.body(body_bytes).send())
            .await
            .map_err(|_| crate::streaming::response_timeout(wait))?
            .map_err(|e| LlmError::Network(self.decorate_error(&e)))?;

        let status = response.status();
        if !status.is_success() {
            // Observe request duration on HTTP status error path.
            if let Some(ref registry) = self.metrics {
                let dur = request_start.elapsed().as_secs_f64();
                let dur_labels = roko_core::obs::metrics::LabelSet::from_pairs(&[
                    (roko_core::obs::schema::LABEL_PROVIDER, &self.provider_id),
                    (roko_core::obs::schema::LABEL_MODEL, &self.model),
                ]);
                registry
                    .register_histogram(
                        roko_core::obs::schema::ROKO_LLM_REQUEST_DURATION_SECONDS,
                        "LLM total request duration in seconds",
                        dur_labels,
                        roko_core::obs::histograms::LLM_LATENCY_BUCKETS.to_vec(),
                    )
                    .observe(dur);
            }
            let retry_after = crate::http::extract_retry_after_from_response(&response);
            let text = response
                .text()
                .await
                .unwrap_or_else(|e| format!("read error body failed: {e}"));
            let raw_err = crate::http::HttpPostError::http_with_retry_after(
                status.as_u16(),
                text,
                retry_after,
            );
            let error = classify_http_error(raw_err, &self.provider_id);
            self.note_refusal(&error);
            return Err(error);
        }

        // Build the stream: spawn a task that reads HTTP chunks, parses SSE
        // lines, and sends StreamEvent values through a channel.
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<StreamEvent, LlmError>>(256);
        let ttft_timeout_ms = self
            .ttft_timeout_ms
            .unwrap_or(config.ttft_timeout.as_millis() as u64);
        let endpoint = self.endpoint();
        let metrics_registry = self.metrics.clone();
        let metrics_provider = self.provider_id.clone();
        let metrics_model = self.model.clone();
        let mut sse_turn = match self.wire {
            OpenAiApi::ChatCompletions => TurnLines::Chat(SseTurn::default()),
            OpenAiApi::Responses => {
                TurnLines::Responses(ResponsesTurn::new(Arc::clone(&self.ledger)))
            }
        };
        let heartbeat = self.reasoning_heartbeat;
        let reasoning_silence = self.reasoning_silence;

        tokio::spawn(async move {
            // The call is in flight until its stream ends.
            let _admitted = admitted;
            let mut response = response;
            let mut pending = Vec::new();
            let mut first_chunk = true;
            let mut ttft_recorded = false;

            loop {
                let chunk_fut = response.chunk();
                let chunk = if first_chunk {
                    first_chunk = false;
                    match tokio::time::timeout(Duration::from_millis(ttft_timeout_ms), chunk_fut)
                        .await
                    {
                        Ok(inner) => inner,
                        Err(_) => {
                            let message = format!(
                                "TTFT timeout: no streaming data within {ttft_timeout_ms}ms"
                            );
                            tracing::warn!(
                                endpoint = %endpoint,
                                ttft_timeout_ms,
                                "TTFT timeout -- no first chunk within deadline"
                            );
                            // Observe request duration on TTFT timeout path.
                            if let Some(ref registry) = metrics_registry {
                                let dur = request_start.elapsed().as_secs_f64();
                                let dur_labels = roko_core::obs::metrics::LabelSet::from_pairs(&[
                                    (
                                        roko_core::obs::schema::LABEL_PROVIDER,
                                        metrics_provider.as_str(),
                                    ),
                                    (roko_core::obs::schema::LABEL_MODEL, metrics_model.as_str()),
                                ]);
                                registry
                                    .register_histogram(
                                        roko_core::obs::schema::ROKO_LLM_REQUEST_DURATION_SECONDS,
                                        "LLM total request duration in seconds",
                                        dur_labels,
                                        roko_core::obs::histograms::LLM_LATENCY_BUCKETS.to_vec(),
                                    )
                                    .observe(dur);
                            }
                            let _ = tx.send(Err(LlmError::Timeout(message))).await;
                            return;
                        }
                    }
                } else if sse_turn.reasoning() {
                    let reasoning = ReasoningWait {
                        heartbeat,
                        limit: reasoning_silence.max(wait),
                        request_start,
                    };
                    match reasoning.chunk(chunk_fut, &tx, &endpoint).await {
                        Some(chunk) => chunk,
                        None => return,
                    }
                } else {
                    let next = crate::streaming::chunk_wait(wait, request_start);
                    if let Ok(chunk) = tokio::time::timeout(next, chunk_fut).await {
                        chunk
                    } else {
                        let error = crate::streaming::chunk_timeout(wait, request_start);
                        tracing::warn!(endpoint = %endpoint, %error, "streamed call cut");
                        let _ = tx.send(Err(error)).await;
                        return;
                    }
                };

                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx
                            .send(Err(LlmError::Network(format!("read chunk failed: {e}"))))
                            .await;
                        return;
                    }
                };
                let Some(chunk) = chunk else {
                    break;
                };

                pending.extend_from_slice(&chunk);
                while let Some(newline_idx) = pending.iter().position(|byte| *byte == b'\n') {
                    let line: Vec<u8> = pending.drain(..=newline_idx).collect();
                    let line_str = String::from_utf8_lossy(&line);
                    let line_str = line_str.trim_end_matches(['\r', '\n']);

                    // One chunk can carry several events (backlog 1110), and an
                    // error object fails the turn (backlog 1111).
                    for item in sse_turn.line(line_str, &metrics_provider, &metrics_model) {
                        let event = match item {
                            Ok(event) => event,
                            Err(error) => {
                                let _ = tx.send(Err(error)).await;
                                return;
                            }
                        };
                        // Record TTFT on the first non-error content/tool/reasoning chunk.
                        if !ttft_recorded {
                            let is_content_chunk = match &event.kind {
                                StreamEventKind::TextDelta(_)
                                | StreamEventKind::ToolCallDelta { .. } => true,
                                // An empty one only says the call is alive.
                                StreamEventKind::ReasoningDelta(text) => !text.is_empty(),
                                _ => false,
                            };
                            if is_content_chunk {
                                ttft_recorded = true;
                                if let Some(ref registry) = metrics_registry {
                                    let ttft_secs = request_start.elapsed().as_secs_f64();
                                    let labels = roko_core::obs::metrics::LabelSet::from_pairs(&[
                                        (
                                            roko_core::obs::schema::LABEL_PROVIDER,
                                            metrics_provider.as_str(),
                                        ),
                                        (
                                            roko_core::obs::schema::LABEL_MODEL,
                                            metrics_model.as_str(),
                                        ),
                                    ]);
                                    registry
                                        .register_histogram(
                                            roko_core::obs::schema::ROKO_LLM_TTFT_SECONDS,
                                            "LLM time-to-first-token in seconds",
                                            labels,
                                            roko_core::obs::histograms::LLM_LATENCY_BUCKETS
                                                .to_vec(),
                                        )
                                        .observe(ttft_secs);
                                }
                            }
                        }
                        if tx.send(Ok(event)).await.is_err() {
                            return; // consumer dropped
                        }
                    }
                }
            }

            // Observe total request duration on stream completion.
            if let Some(ref registry) = metrics_registry {
                let dur = request_start.elapsed().as_secs_f64();
                let dur_labels = roko_core::obs::metrics::LabelSet::from_pairs(&[
                    (
                        roko_core::obs::schema::LABEL_PROVIDER,
                        metrics_provider.as_str(),
                    ),
                    (roko_core::obs::schema::LABEL_MODEL, metrics_model.as_str()),
                ]);
                registry
                    .register_histogram(
                        roko_core::obs::schema::ROKO_LLM_REQUEST_DURATION_SECONDS,
                        "LLM total request duration in seconds",
                        dur_labels,
                        roko_core::obs::histograms::LLM_LATENCY_BUCKETS.to_vec(),
                    )
                    .observe(dur);
            }

            // Flush remaining buffer.
            if !pending.is_empty() {
                let line_str = String::from_utf8_lossy(&pending);
                let line_str = line_str.trim_end_matches(['\r', '\n']);
                for item in sse_turn.line(line_str, &metrics_provider, &metrics_model) {
                    let failed = item.is_err();
                    let _ = tx.send(item).await;
                    if failed {
                        return;
                    }
                }
            }

            // Every turn ends with exactly one Done. A stream that named no
            // finish reason ends as `unknown`, not a made-up `stop`.
            if let Some(done) = sse_turn.end(&metrics_provider, &metrics_model) {
                let _ = tx.send(Ok(done)).await;
            }
        });

        // Convert tokio mpsc::Receiver into a futures::Stream without
        // requiring the tokio-stream crate.
        let stream = futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });
        Ok(Box::pin(stream))
    }

    fn extract_session(&self, response: &BackendResponse) -> SessionState {
        match response {
            BackendResponse::Json(json) => extract_session(json),
            BackendResponse::StreamJson(_) | BackendResponse::Text(_) => SessionState::default(),
        }
    }

    fn backend_id(&self) -> &'static str {
        "openai_compat"
    }
}

impl std::fmt::Debug for OpenAiCompatLlmBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatLlmBackend")
            .field("model", &self.model)
            .field("provider_id", &self.provider_id)
            .field("base_url", &self.base_url)
            .field("timeout_ms", &self.timeout_ms)
            .field("max_tokens", &self.max_tokens)
            .finish_non_exhaustive()
    }
}

/// The chat endpoint of the OpenAI-compatible provider at `base_url`: the key
/// [`shared_rate_limiter`] learns that provider's calls at once by
/// (bug-4b6844, bug-62ae1e).
#[must_use]
pub fn chat_endpoint(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration as StdDuration;
    use std::time::Instant;
    use tokio::sync::mpsc;

    use crate::dispatcher::{HandlerResolver, ToolDispatcher};
    use crate::tool_loop::{StopReason, StreamEventKind, ToolLoop, TurnConfig};
    use crate::translate::{OpenAiTranslator, Translator};
    use roko_core::tool::{
        ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolHandler, ToolPermission,
        ToolResult, VecToolRegistry,
    };
    use tokio::time::{Duration, timeout};

    use crate::http::HttpPostError;
    use crate::provider::ProviderError;

    fn test_timeout(ms: u64) -> Duration {
        let scaled = if std::env::var("CI").map(|v| v == "true").unwrap_or(false) {
            ms * 10
        } else {
            ms
        };
        Duration::from_millis(scaled)
    }

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        url: String,
        headers: Vec<(String, String)>,
        body: Value,
        timeout_ms: u64,
    }

    struct MockPoster {
        responses: Mutex<VecDeque<Result<String, HttpPostError>>>,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
    }

    impl MockPoster {
        fn new(
            responses: Vec<Result<String, HttpPostError>>,
        ) -> (Self, Arc<Mutex<Vec<RecordedRequest>>>) {
            let requests = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    responses: Mutex::new(responses.into_iter().collect()),
                    requests: requests.clone(),
                },
                requests,
            )
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
                .expect("mock response queued")
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

    fn test_tools() -> Vec<ToolDef> {
        vec![
            ToolDef::new(
                "echo",
                "echo args",
                ToolCategory::Meta,
                ToolPermission::read_only(),
            )
            .with_concurrency(ToolConcurrency::Parallel),
        ]
    }

    fn make_tool_loop(backend: OpenAiCompatLlmBackend) -> ToolLoop {
        let registry: Arc<dyn roko_core::tool::ToolRegistry> =
            Arc::new(VecToolRegistry::from_tools(test_tools()));
        let resolver: Arc<dyn HandlerResolver> =
            Arc::new(|name: &str| -> Option<Arc<dyn ToolHandler>> {
                if name == "echo" {
                    Some(Arc::new(EchoHandler) as Arc<dyn ToolHandler>)
                } else {
                    None
                }
            });
        let dispatcher = Arc::new(ToolDispatcher::new_unguarded(registry, resolver));
        let translator: Arc<dyn Translator> = Arc::new(OpenAiTranslator);
        ToolLoop::new(translator, dispatcher, Arc::new(backend))
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(StdDuration::from_secs(2)))
            .expect("set read timeout");

        let mut buf = Vec::new();
        let mut chunk = [0_u8; 1024];
        let header_end = loop {
            let read = stream.read(&mut chunk).expect("read request bytes");
            assert!(read > 0, "request closed before headers completed");
            buf.extend_from_slice(&chunk[..read]);

            if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                break pos + 4;
            }
        };

        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);

        while buf.len() < header_end + content_length {
            let read = stream.read(&mut chunk).expect("read request body");
            assert!(read > 0, "request closed before body completed");
            buf.extend_from_slice(&chunk[..read]);
        }

        String::from_utf8(buf).expect("request should be valid utf8")
    }

    fn sse_json_line(value: Value) -> String {
        format!("data: {value}\n\n")
    }

    #[tokio::test]
    async fn openai_compat_backend_posts_expected_request() {
        let (poster, requests) = MockPoster::new(vec![Ok(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "done"
                }
            }]
        })
        .to_string())]);
        let mut extra_body_params = Map::new();
        extra_body_params.insert(
            "thinking".to_string(),
            serde_json::json!({ "type": "enabled" }),
        );
        let backend = OpenAiCompatLlmBackend::new("test-key", "glm-5.1")
            .with_base_url("https://api.z.ai/api/paas/v4/")
            .with_timeout_ms(90_000)
            .with_extra_headers(HashMap::from([(
                "X-Test-Header".to_string(),
                "present".to_string(),
            )]))
            .with_extra_body_params(extra_body_params)
            .with_poster(Box::new(poster));

        let response = backend
            .send_turn(
                &[serde_json::json!({ "role": "user", "content": "hi" })],
                &RenderedTools::JsonArray(serde_json::json!([{
                    "type": "function",
                    "function": {
                        "name": "echo",
                        "description": "echo args",
                        "parameters": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                }])),
                &SessionState::default(),
            )
            .await
            .expect("send turn");
        assert!(matches!(response, BackendResponse::Json(_)));

        let requests = requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].url,
            "https://api.z.ai/api/paas/v4/chat/completions"
        );
        assert_eq!(requests[0].timeout_ms, 90_000);
        assert!(
            requests[0].headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("authorization") && value == "Bearer test-key"
            }),
            "expected authorization header"
        );
        assert!(
            requests[0].headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("x-test-header") && value == "present"
            }),
            "expected extra header"
        );
        assert_eq!(requests[0].body["model"], "glm-5.1");
        assert_eq!(requests[0].body["thinking"]["type"], "enabled");
        assert_eq!(requests[0].body["tools"][0]["function"]["name"], "echo");
    }

    #[test]
    fn build_body_can_use_max_completion_tokens() {
        let backend = OpenAiCompatLlmBackend::new("test-key", "gpt-5.4")
            .with_max_tokens(256)
            .with_use_max_completion_tokens(true);
        let body = backend
            .build_body(
                &[serde_json::json!({"role": "user", "content": "hello"})],
                &RenderedTools::JsonArray(serde_json::json!([])),
                &SessionState::default(),
                false,
            )
            .expect("build body");
        let parsed: Value = serde_json::from_slice(&body).expect("request body json");

        assert_eq!(parsed["model"], "gpt-5.4");
        assert_eq!(parsed["max_completion_tokens"], 256);
        assert!(parsed.get("max_tokens").is_none());
    }

    /// backlog 2101: a streaming request asks for usage, which OpenAI streams
    /// only when asked; a blocking request, or a backend whose provider opted
    /// out, sends no `stream_options`.
    #[test]
    fn streaming_request_asks_for_usage() {
        let body = |backend: &OpenAiCompatLlmBackend, stream: bool| -> Value {
            let body = backend
                .build_body(
                    &[serde_json::json!({"role": "user", "content": "hello"})],
                    &RenderedTools::JsonArray(serde_json::json!([])),
                    &SessionState::default(),
                    stream,
                )
                .expect("build body");
            serde_json::from_slice(&body).expect("request body json")
        };
        let backend = OpenAiCompatLlmBackend::new("test-key", "gpt-5.4-mini");

        let streaming = body(&backend, true);
        assert_eq!(streaming["stream"], true);
        assert_eq!(streaming["stream_options"]["include_usage"], true);

        let blocking = body(&backend, false);
        assert!(blocking.get("stream").is_none(), "{blocking}");
        assert!(blocking.get("stream_options").is_none(), "{blocking}");

        let opted_out = OpenAiCompatLlmBackend::new("test-key", "gpt-5.4-mini");
        let opted_out = body(&opted_out.with_stream_usage(false), true);
        assert_eq!(opted_out["stream"], true);
        assert!(opted_out.get("stream_options").is_none(), "{opted_out}");
    }

    /// bug-29014c: chat completions refuses `reasoning_effort` beside function
    /// tools for the GPT-5.4 family ("Function tools with reasoning_effort
    /// are not supported"), so a request with tools leaves it out, streamed
    /// or not, and a request without them carries it.
    #[test]
    fn reasoning_effort_goes_only_on_requests_without_function_tools() {
        let backend =
            OpenAiCompatLlmBackend::new("test-key", "gpt-5.4-mini").with_extra_body_params(
                Map::from_iter([("reasoning_effort".to_string(), Value::from("medium"))]),
            );
        let body = |tools: Value, stream: bool| -> Value {
            let body = backend
                .build_body(
                    &[serde_json::json!({"role": "user", "content": "hello"})],
                    &RenderedTools::JsonArray(tools),
                    &SessionState::default(),
                    stream,
                )
                .expect("build body");
            serde_json::from_slice(&body).expect("request body json")
        };
        let echo = serde_json::json!([{
            "type": "function",
            "function": {"name": "echo", "parameters": {"type": "object"}}
        }]);

        for stream in [false, true] {
            let with_tools = body(echo.clone(), stream);
            assert!(with_tools.get("tools").is_some(), "{with_tools}");
            assert!(with_tools.get("reasoning_effort").is_none(), "{with_tools}");
            let without_tools = body(serde_json::json!([]), stream);
            assert!(without_tools.get("tools").is_none(), "{without_tools}");
            assert_eq!(without_tools["reasoning_effort"], "medium");
        }
    }

    /// bug-d0b8b8: a text tool format's block joins the system message, or
    /// becomes the first message when there is none, and the request then
    /// carries no `tools`; an empty native `tools` array is omitted too.
    #[test]
    fn build_body_puts_a_system_prompt_tool_block_in_the_system_message() {
        let backend = OpenAiCompatLlmBackend::new("test-key", "hermes-4-70b");
        let block = RenderedTools::SystemPromptBlock("<tools>[]</tools>".to_string());
        let body = |messages: &[Value], tools: &RenderedTools| -> Value {
            let body = backend
                .build_body(messages, tools, &SessionState::default(), false)
                .expect("build body");
            serde_json::from_slice(&body).expect("request body json")
        };

        let with_system = body(
            &[
                serde_json::json!({"role": "system", "content": "Be brief."}),
                serde_json::json!({"role": "user", "content": "hello"}),
            ],
            &block,
        );
        assert!(with_system.get("tools").is_none(), "{with_system}");
        assert_eq!(
            with_system["messages"][0]["content"],
            "Be brief.\n\n<tools>[]</tools>"
        );
        assert_eq!(with_system["messages"][1]["content"], "hello");

        let without_system = body(
            &[serde_json::json!({"role": "user", "content": "hello"})],
            &block,
        );
        assert_eq!(without_system["messages"][0]["role"], "system");
        assert_eq!(
            without_system["messages"][0]["content"],
            "<tools>[]</tools>"
        );
        assert_eq!(without_system["messages"][1]["content"], "hello");

        let no_tools = body(
            &[serde_json::json!({"role": "user", "content": "hello"})],
            &RenderedTools::JsonArray(serde_json::json!([])),
        );
        assert!(no_tools.get("tools").is_none(), "{no_tools}");
    }

    #[test]
    fn build_body_rejects_images_for_non_vision_model() {
        let backend = OpenAiCompatLlmBackend::new("test-key", "text-only");
        let result = backend.build_body(
            &[serde_json::json!({
                "role": "user",
                "content": [{"type": "image_url", "image_url": {
                    "url": "data:image/png;base64,aGVsbG8="
                }}]
            })],
            &RenderedTools::JsonArray(serde_json::json!([])),
            &SessionState::default(),
            false,
        );

        assert!(
            matches!(result, Err(LlmError::Backend(message)) if message.contains("does not support"))
        );
    }

    #[test]
    fn headers_are_cached_and_recomputed_for_extra_headers() {
        let backend = OpenAiCompatLlmBackend::new("test-key", "glm-5.1");

        let first = backend.headers();
        let second = backend.headers();

        assert_eq!(
            first,
            vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer test-key".to_string()),
            ]
        );
        assert_eq!(first, second);

        let updated = backend
            .with_extra_headers(HashMap::from([
                ("X-B".to_string(), "2".to_string()),
                ("X-A".to_string(), "1".to_string()),
            ]))
            .headers();

        assert_eq!(
            updated,
            vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer test-key".to_string()),
                ("X-A".to_string(), "1".to_string()),
                ("X-B".to_string(), "2".to_string()),
            ]
        );
    }

    struct HeaderPointerPoster {
        responses: Mutex<VecDeque<Result<String, HttpPostError>>>,
        auth_ptrs: Arc<Mutex<Vec<usize>>>,
    }

    impl HeaderPointerPoster {
        fn new(responses: Vec<Result<String, HttpPostError>>) -> (Self, Arc<Mutex<Vec<usize>>>) {
            let auth_ptrs = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    responses: Mutex::new(responses.into_iter().collect()),
                    auth_ptrs: auth_ptrs.clone(),
                },
                auth_ptrs,
            )
        }
    }

    #[async_trait]
    impl HttpPoster for HeaderPointerPoster {
        async fn post_json(
            &self,
            _url: &str,
            headers: &[(String, String)],
            _body: &[u8],
            _timeout_ms: u64,
        ) -> Result<String, HttpPostError> {
            let auth_ptr = headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                .map(|(_, value)| value.as_ptr() as usize)
                .expect("authorization header");
            self.auth_ptrs.lock().expect("auth ptr lock").push(auth_ptr);
            self.responses
                .lock()
                .expect("responses lock")
                .pop_front()
                .expect("mock response queued")
        }
    }

    #[tokio::test]
    async fn request_path_reuses_cached_authorization_header() {
        let (poster, auth_ptrs) = HeaderPointerPoster::new(vec![
            Ok(serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "done"
                    }
                }]
            })
            .to_string()),
            Ok(serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "done"
                    }
                }]
            })
            .to_string()),
        ]);
        let backend =
            OpenAiCompatLlmBackend::new("test-key", "glm-5.1").with_poster(Box::new(poster));
        let messages = [serde_json::json!({ "role": "user", "content": "hi" })];
        let tools = RenderedTools::JsonArray(serde_json::json!([]));
        let session = SessionState::default();

        backend
            .send_turn(&messages, &tools, &session)
            .await
            .expect("first send turn");
        backend
            .send_turn(&messages, &tools, &session)
            .await
            .expect("second send turn");

        let auth_ptrs = auth_ptrs.lock().expect("auth ptr lock");
        assert_eq!(auth_ptrs.len(), 2);
        assert_eq!(auth_ptrs[0], auth_ptrs[1]);
    }

    #[tokio::test]
    async fn rate_limited_backend_spreads_same_provider_requests() {
        let limiter = Arc::new(ProviderRateLimiter::new_per_second(1));
        let (poster_a, requests_a) = MockPoster::new(vec![Ok(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "done"
                }
            }]
        })
        .to_string())]);
        let (poster_b, requests_b) = MockPoster::new(vec![Ok(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "done"
                }
            }]
        })
        .to_string())]);

        let backend_a = OpenAiCompatLlmBackend::new("test-key", "glm-5.1")
            .with_provider_id("zai")
            .with_rate_limiter(limiter.clone())
            .with_poster(Box::new(poster_a));
        let backend_b = OpenAiCompatLlmBackend::new("test-key", "glm-5.1")
            .with_provider_id("zai")
            .with_rate_limiter(limiter)
            .with_poster(Box::new(poster_b));
        let messages = [serde_json::json!({ "role": "user", "content": "hi" })];
        let tools = RenderedTools::JsonArray(serde_json::json!([]));
        let session = SessionState::default();

        let start = Instant::now();
        let (response_a, response_b) = tokio::join!(
            backend_a.send_turn(&messages, &tools, &session),
            backend_b.send_turn(&messages, &tools, &session),
        );
        let elapsed = start.elapsed();

        assert!(matches!(response_a, Ok(BackendResponse::Json(_))));
        assert!(matches!(response_b, Ok(BackendResponse::Json(_))));
        assert!(
            elapsed >= StdDuration::from_millis(900),
            "same-provider requests should be throttled, got {elapsed:?}"
        );
        assert!(
            elapsed < StdDuration::from_secs(3),
            "test limiter should finish promptly, got {elapsed:?}"
        );
        assert_eq!(requests_a.lock().expect("requests lock").len(), 1);
        assert_eq!(requests_b.lock().expect("requests lock").len(), 1);
    }

    #[test]
    fn session_extraction_reads_openai_compat_response_ids() {
        let backend = OpenAiCompatLlmBackend::new("test-key", "glm-5.1");
        let response = BackendResponse::Json(serde_json::json!({
            "id": "chatcmpl-glm",
            "session_id": "sess-glm",
            "thread_id": "thread-glm",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "done"
                }
            }]
        }));

        let session = backend.extract_session(&response);

        assert_eq!(session.conversation_id.as_deref(), Some("chatcmpl-glm"));
        assert_eq!(session.session_id.as_deref(), Some("sess-glm"));
        assert_eq!(session.thread_id.as_deref(), Some("thread-glm"));
    }

    #[test]
    fn session_extraction_defaults_when_ids_are_absent() {
        let backend = OpenAiCompatLlmBackend::new("test-key", "kimi-k2.5");
        let response = BackendResponse::Json(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "done"
                }
            }]
        }));

        let session = backend.extract_session(&response);

        assert!(session.session_id.is_none());
        assert!(session.thread_id.is_none());
        assert!(session.conversation_id.is_none());
    }

    #[tokio::test]
    async fn tool_loop_basic() {
        let first_response = serde_json::json!({
            "id": "chatcmpl-1",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": {
                            "name": "echo",
                            "arguments": serde_json::json!({ "value": 1 }).to_string()
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })
        .to_string();
        let second_response = serde_json::json!({
            "id": "chatcmpl-2",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "final answer"
                },
                "finish_reason": "stop"
            }]
        })
        .to_string();
        let (poster, requests) = MockPoster::new(vec![Ok(first_response), Ok(second_response)]);
        let backend = OpenAiCompatLlmBackend::new("test-key", "glm-5.1")
            .with_base_url("https://api.z.ai/api/paas/v4")
            .with_poster(Box::new(poster));
        let tool_loop = make_tool_loop(backend);
        let ctx = ToolContext::testing("/tmp");

        let result = tool_loop
            .run(
                "system prompt",
                "call the tool, then finish",
                &test_tools(),
                &ctx,
            )
            .await;

        assert_eq!(result.stop_reason, StopReason::Stop);
        assert_eq!(result.iterations, 1);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call-1");
        assert_eq!(result.tool_calls[0].name, "echo");
        assert_eq!(result.final_text, "final answer");

        let requests = requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 2, "expected two HTTP turns");
        let second_turn_messages = requests[1]
            .body
            .get("messages")
            .and_then(Value::as_array)
            .expect("second turn messages");
        let assistant_message = second_turn_messages
            .iter()
            .find(|msg| msg["role"] == "assistant")
            .expect("assistant tool-call message");
        assert_eq!(assistant_message["tool_calls"][0]["id"], "call-1");
        let tool_message = second_turn_messages
            .iter()
            .find(|msg| msg.get("tool_call_id").is_some())
            .expect("tool result message");
        assert_eq!(tool_message["tool_call_id"], "call-1");
        assert_eq!(tool_message["content"], "{\"value\":1}");
    }

    #[tokio::test]
    async fn streaming_tool_loop_emits_chunks_and_matches_final_result() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("listener addr");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_server = requests.clone();

        let server = thread::spawn(move || {
            let responses = [
                vec![
                    sse_json_line(serde_json::json!({
                        "choices": [{
                            "delta": {
                                "reasoning_content": "Need to inspect the args. "
                            }
                        }]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{
                            "delta": {
                                "tool_calls": [{
                                    "index": 0,
                                    "id": "call-1",
                                    "function": {
                                        "name": "echo",
                                        "arguments": "{\"value\":"
                                    }
                                }]
                            }
                        }]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{
                            "delta": {
                                "tool_calls": [{
                                    "index": 0,
                                    "function": {
                                        "arguments": "1}"
                                    }
                                }]
                            }
                        }]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{
                            "delta": {},
                            "finish_reason": "tool_calls"
                        }]
                    })),
                    sse_json_line(serde_json::json!({
                        "usage": {
                            "prompt_tokens": 11,
                            "completion_tokens": 7,
                            "prompt_tokens_details": {
                                "cached_tokens": 3
                            }
                        }
                    })),
                    "data: [DONE]\n\n".to_string(),
                ],
                vec![
                    sse_json_line(serde_json::json!({
                        "choices": [{
                            "delta": {
                                "content": "final "
                            }
                        }]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{
                            "delta": {
                                "content": "answer"
                            }
                        }]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{
                            "delta": {},
                            "finish_reason": "stop"
                        }]
                    })),
                    sse_json_line(serde_json::json!({
                        "usage": {
                            "prompt_tokens": 9,
                            "completion_tokens": 4
                        }
                    })),
                    "data: [DONE]\n\n".to_string(),
                ],
            ];

            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept request");
                let request = read_http_request(&mut stream);
                requests_for_server
                    .lock()
                    .expect("requests lock")
                    .push(request);

                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
                )
                .expect("write response headers");
                stream.flush().expect("flush response headers");

                for chunk in response {
                    stream
                        .write_all(chunk.as_bytes())
                        .expect("write response chunk");
                    stream.flush().expect("flush response chunk");
                    thread::sleep(StdDuration::from_millis(40));
                }
            }
        });

        let backend = OpenAiCompatLlmBackend::new("test-key", "glm-5.1")
            .with_base_url(format!("http://{addr}"))
            .with_timeout_ms(5_000);
        let tool_loop = make_tool_loop(backend);
        let tools = test_tools();
        let (event_tx, mut event_rx) = mpsc::channel(roko_core::defaults::DEFAULT_CHANNEL_BUFFER);
        let run = tokio::spawn(async move {
            let ctx = ToolContext::testing("/tmp");
            tool_loop
                .run_streaming(
                    "system prompt",
                    "call the tool, then finish",
                    &tools,
                    &ctx,
                    event_tx,
                )
                .await
        });

        let first_chunk = timeout(test_timeout(500), event_rx.recv())
            .await
            .expect("stream should emit before completion")
            .expect("stream channel open");
        assert!(matches!(
            first_chunk.kind,
            StreamEventKind::ReasoningDelta(_)
        ));
        assert!(
            !run.is_finished(),
            "streaming events should arrive before the tool loop finishes"
        );

        let result = run.await.expect("tool loop task");
        let mut events = vec![first_chunk];
        while let Some(event) = event_rx.recv().await {
            events.push(event);
        }

        assert_eq!(result.stop_reason, StopReason::Stop);
        assert_eq!(result.iterations, 1);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call-1");
        assert_eq!(result.tool_calls[0].name, "echo");
        assert_eq!(result.final_text, "final answer");
        // 11 + 9 prompt tokens, 3 of them cached: 17 uncached (bug-b72a37).
        assert_eq!(result.total_usage.input_tokens, 17);
        assert_eq!(result.total_usage.output_tokens, 11);
        assert_eq!(result.total_usage.cache_read_tokens, 3);

        // The raw ToolCallStart event embeds the index key with the real
        // id after a NUL separator and may carry initial argument fragments
        // after a SOH separator: "__idx_0\0call-1\x01{\"value\":".
        assert!(events.iter().any(|e| matches!(
            &e.kind,
            StreamEventKind::ToolCallStart { id, name }
                if id.contains("call-1") && name == "echo"
        )));
        assert!(events.iter().any(|e| {
            matches!(&e.kind, StreamEventKind::TextDelta(content) if content == "final ")
        }));

        let requests = requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 2, "expected two streamed HTTP turns");
        assert!(
            requests
                .iter()
                .all(|request| request.contains("\"stream\":true"))
        );
        assert!(requests[1].contains("\"tool_call_id\":\"call-1\""));
        assert!(requests[1].contains("\"arguments\":\"{\\\"value\\\":1}\""));

        server.join().expect("server thread");
    }

    /// gap-5fda00: an OpenAI-compatible stream never ends its tool calls, yet
    /// an API model's call reaches the provider boundary's live output as a
    /// tool step before it runs and, trusted, as its result once it has, as
    /// a CLI provider's calls do, both under the call's own id.
    #[tokio::test]
    async fn api_model_tool_calls_reach_the_live_output() {
        use crate::agent::Agent;
        use crate::immune_boundary::ImmuneScreenedAgent;
        use crate::live_output::{LiveAgentEvent, LiveOutput};
        use crate::tool_loop::ToolLoopAgent;
        use roko_core::{Body, Context, Kind, Signal};

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("listener addr");
        let server = thread::spawn(move || {
            let responses = [
                vec![
                    sse_json_line(serde_json::json!({
                        "choices": [{"delta": {"tool_calls": [{
                            "index": 0,
                            "id": "call-1",
                            "function": {"name": "echo", "arguments": "{\"path\":"}
                        }]}}]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{"delta": {"tool_calls": [{
                            "index": 0,
                            "function": {"arguments": "\"src/lib.rs\"}"}
                        }]}}]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{"delta": {}, "finish_reason": "tool_calls"}]
                    })),
                    "data: [DONE]\n\n".to_string(),
                ],
                vec![
                    sse_json_line(serde_json::json!({
                        "choices": [{"delta": {"content": "read it"}}]
                    })),
                    sse_json_line(serde_json::json!({
                        "choices": [{"delta": {}, "finish_reason": "stop"}]
                    })),
                    "data: [DONE]\n\n".to_string(),
                ],
            ];
            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept request");
                read_http_request(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
                )
                .expect("write response headers");
                for chunk in response {
                    stream
                        .write_all(chunk.as_bytes())
                        .expect("write response chunk");
                    stream.flush().expect("flush response chunk");
                }
            }
        });

        let backend = OpenAiCompatLlmBackend::new("test-key", "glm-5.1")
            .with_base_url(format!("http://{addr}"))
            .with_timeout_ms(5_000);
        let agent = ToolLoopAgent::new(make_tool_loop(backend)).with_tools(test_tools());
        let (live_tx, mut live_rx) = mpsc::channel::<LiveAgentEvent>(64);
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(agent),
            "api-model-agent",
            Arc::new(roko_std::MemorySubstrate::new()),
        )
        .with_live_output(LiveOutput {
            sink: live_tx,
            trusted: true,
        });

        let input = Signal::builder(Kind::Prompt)
            .body(Body::text("read src/lib.rs, then finish"))
            .build();
        let result = boundary.run(&input, &Context::now()).await;
        drop(boundary);
        server.join().expect("server thread");
        assert!(result.success, "{:?}", result.output.body);

        let mut events = Vec::new();
        while let Some(event) = live_rx.recv().await {
            events.push(event);
        }
        let steps: Vec<(&str, &str, &str)> = events
            .iter()
            .filter_map(|event| match event {
                LiveAgentEvent::ToolStep { id, name, target } => {
                    Some((id.as_str(), name.as_str(), target.as_str()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(steps, [("call-1", "echo", "src/lib.rs")], "{events:?}");
        let step = events
            .iter()
            .position(|event| matches!(event, LiveAgentEvent::ToolStep { .. }))
            .expect("the call's tool step");
        let tool_result = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    LiveAgentEvent::Unscreened(StreamEventKind::ToolResult {
                        id,
                        output,
                        is_error: false,
                    }) if id == "call-1" && output.contains("src/lib.rs")
                )
            })
            .expect("the call's result, trusted");
        assert!(
            step < tool_result,
            "the step comes before the result: {events:?}"
        );
    }

    /// bug-b72a37: 800 of an answer's 1,000 prompt tokens were cached. The
    /// non-streaming and the streaming path both price 200 uncached input
    /// tokens and 800 cache-read tokens, each once.
    #[tokio::test]
    async fn openai_compat_usage_prices_cached_input_once() {
        use crate::tool_loop::{StreamEvent, collect_stream_to_response};

        let usage_block = serde_json::json!({
            "prompt_tokens": 1_000,
            "completion_tokens": 100,
            "total_tokens": 1_100,
            "prompt_tokens_details": {"cached_tokens": 800}
        });
        // $1/M input, $2/M output, $0.10/M cache read.
        let expected_cost = (200.0 + 100.0 * 2.0 + 800.0 * 0.10) / 1_000_000.0;

        // Non-streaming: the tool loop prices the JSON response's usage.
        let response = BackendResponse::Json(serde_json::json!({ "usage": usage_block.clone() }));
        let non_streaming = response.extract_usage();

        // Streaming: the final chunk's usage, collected into the response the
        // tool loop prices.
        let chunk = serde_json::json!({ "choices": [], "usage": usage_block });
        let usage_event = crate::streaming::parse_sse_line(&format!("data: {chunk}"))
            .into_iter()
            .next()
            .expect("a usage chunk");
        assert!(matches!(usage_event.kind, StreamEventKind::Usage(_)));
        let events = vec![
            Ok(usage_event),
            Ok(StreamEvent::now(StreamEventKind::Done {
                finish_reason: "stop".to_string(),
            })),
        ];
        let stream = Box::pin(futures::stream::iter(events));
        let collected = collect_stream_to_response(stream, Instant::now()).await;
        let streamed = collected.expect("the stream collects").extract_usage();

        for mut usage in [non_streaming, streamed] {
            assert_eq!(
                (
                    usage.input_tokens,
                    usage.cache_read_tokens,
                    usage.output_tokens
                ),
                (200, 800, 100)
            );
            usage.fill_cost_from_pricing(Some(1.0), Some(2.0), Some(0.10), None);
            assert!(
                (f64::from(usage.cost_usd) - expected_cost).abs() < 1e-9,
                "cost {} != {expected_cost}",
                usage.cost_usd
            );
        }
    }

    #[tokio::test]
    async fn openai_compat_backend_requires_json_tools() {
        let (poster, _) = MockPoster::new(vec![]);
        let backend = OpenAiCompatLlmBackend::new("", "test-model").with_poster(Box::new(poster));

        let err = backend
            .send_turn(
                &[serde_json::json!({ "role": "user", "content": "hi" })],
                &RenderedTools::CliFlag("echo".to_string()),
                &SessionState::default(),
            )
            .await
            .expect_err("non-json tools should fail");

        assert!(matches!(err, LlmError::Backend(_)));
    }

    #[test]
    fn debug_impl_mentions_model() {
        let backend = OpenAiCompatLlmBackend::new("test-key", "glm-5.1");
        let s = format!("{backend:?}");
        assert!(s.contains("OpenAiCompatLlmBackend"));
        assert!(s.contains("glm-5.1"));
    }

    /// Live integration test — sends a real request to OpenAI to verify the
    /// request body is well-formed and the model parameter is accepted.
    ///
    /// Requires `OPENAI_API_KEY` in the environment. Skipped otherwise.
    #[tokio::test]
    async fn live_openai_gpt4o_accepts_request() {
        let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
            eprintln!("skipping: OPENAI_API_KEY not set");
            return;
        };

        let backend = OpenAiCompatLlmBackend::new(api_key, "gpt-4o").with_max_tokens(10);
        let result = backend
            .send_turn(
                &[serde_json::json!({ "role": "user", "content": "say hi" })],
                &RenderedTools::JsonArray(serde_json::json!([])),
                &SessionState::default(),
            )
            .await;

        match &result {
            Ok(BackendResponse::Json(json)) => {
                assert!(
                    json.pointer("/choices/0/message/content").is_some(),
                    "expected choices[0].message.content in response: {json}"
                );
            }
            Ok(other) => panic!("unexpected response variant: {other:?}"),
            Err(e) => panic!("request failed: {e:?}"),
        }
    }

    /// Verify TTFT timeout fires when the server sends HTTP headers but delays
    /// the first body chunk beyond the deadline.
    #[tokio::test]
    async fn streaming_ttft_timeout_fires_on_slow_first_chunk() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("listener addr");

        // Server: send response headers immediately, then sleep longer than
        // the TTFT timeout before sending any body data.
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = read_http_request(&mut stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            )
            .expect("write response headers");
            stream.flush().expect("flush response headers");
            // Delay first chunk beyond the 100ms TTFT timeout.
            thread::sleep(StdDuration::from_millis(2000));
            let _ =
                stream.write_all(b"data: {\"choices\":[{\"delta\":{\"content\":\"late\"}}]}\n\n");
        });

        let backend = OpenAiCompatLlmBackend::new("test-key", "test-model")
            .with_base_url(&format!("http://{addr}/v1/"))
            .with_ttft_timeout_ms(Some(100)); // 100ms TTFT — server delays 2s

        let started = Instant::now();

        let config = TurnConfig {
            ttft_timeout: StdDuration::from_millis(100),
            ..TurnConfig::default()
        };

        let stream = backend
            .stream_turn(
                &[serde_json::json!({ "role": "user", "content": "hello" })],
                &RenderedTools::JsonArray(serde_json::json!([])),
                &SessionState::default(),
                &config,
            )
            .await;

        let elapsed = started.elapsed();

        // stream_turn may succeed (returns a stream) but the stream itself
        // should yield a TTFT timeout error. Or stream_turn itself may fail.
        if let Ok(stream) = stream {
            use futures::StreamExt;
            let events: Vec<_> = stream.collect().await;
            let got_timeout = events.iter().any(|e| {
                if let Err(err) = e {
                    format!("{err:?}").contains("TTFT timeout")
                        || format!("{err:?}").contains("Timeout")
                } else {
                    false
                }
            });
            assert!(got_timeout, "expected TTFT timeout in stream events");
        } else {
            let err = stream.err().expect("already checked Err");
            assert!(
                format!("{err:?}").contains("TTFT timeout")
                    || format!("{err:?}").contains("Timeout"),
                "error should mention TTFT: {err:?}"
            );
        }

        assert!(
            elapsed < StdDuration::from_millis(1500),
            "should timeout quickly (~100ms), not wait for delayed body (took {}ms)",
            elapsed.as_millis()
        );

        server.join().expect("server thread");
    }

    /// Serve one streaming response with `body`, then close the connection.
    fn serve_sse_once(body: &'static str) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("listener addr");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = read_http_request(&mut stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            )
            .expect("write response headers");
            stream.write_all(body.as_bytes()).expect("write body");
            stream.flush().expect("flush body");
        });
        (format!("http://{addr}/v1/"), server)
    }

    /// Serve one streaming response that writes a content delta `"x"` after
    /// each of `gaps`, then a `stop` finish and `[DONE]`.
    fn serve_paced_sse(gaps: Vec<StdDuration>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("listener addr");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = read_http_request(&mut stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            )
            .expect("write response headers");
            stream.flush().expect("flush response headers");
            // The client may have gone by the time a write comes.
            for gap in gaps {
                thread::sleep(gap);
                let delta = b"data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\n";
                if stream.write_all(delta).is_err() || stream.flush().is_err() {
                    return;
                }
            }
            let _ = stream.write_all(
                b"data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
            );
        });
        (format!("http://{addr}/v1/"), server)
    }

    /// Every item one streamed turn against `base_url` yields.
    async fn streamed_turn(base_url: &str) -> Vec<Result<StreamEvent, LlmError>> {
        streamed_turn_with(base_url, &TurnConfig::default()).await
    }

    /// Every item one streamed turn against `base_url`, under `config`, yields.
    async fn streamed_turn_with(
        base_url: &str,
        config: &TurnConfig,
    ) -> Vec<Result<StreamEvent, LlmError>> {
        use futures::StreamExt;

        let backend = OpenAiCompatLlmBackend::new("test-key", "test-model").with_base_url(base_url);
        backend
            .stream_turn(
                &[serde_json::json!({ "role": "user", "content": "hello" })],
                &RenderedTools::JsonArray(serde_json::json!([])),
                &SessionState::default(),
                config,
            )
            .await
            .expect("the request is sent")
            .collect()
            .await
    }

    fn streamed_text(items: &[Result<StreamEvent, LlmError>]) -> String {
        items
            .iter()
            .filter_map(|item| match item {
                Ok(StreamEvent {
                    kind: StreamEventKind::TextDelta(text),
                    ..
                }) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn finish_reasons(items: &[Result<StreamEvent, LlmError>]) -> Vec<String> {
        items
            .iter()
            .filter_map(|item| match item {
                Ok(StreamEvent {
                    kind: StreamEventKind::Done { finish_reason },
                    ..
                }) => Some(finish_reason.clone()),
                _ => None,
            })
            .collect()
    }

    /// backlog 1111: an `error` object in the stream fails the turn with the
    /// provider's message instead of being dropped, and no made-up `Done`
    /// follows it.
    #[tokio::test]
    async fn sse_error_object_fails_the_turn() {
        let (base_url, server) = serve_sse_once(
            "data: {\"error\":{\"message\":\"The model is overloaded\",\"type\":\"server_error\",\"code\":\"1305\"}}\n\n",
        );
        let items = streamed_turn(&base_url).await;
        server.join().expect("server thread");

        let error = items
            .iter()
            .find_map(|item| item.as_ref().err())
            .expect("the turn fails");
        assert!(
            error.to_string().contains("The model is overloaded"),
            "{error}"
        );
        assert!(finish_reasons(&items).is_empty(), "{items:?}");

        // An error code that is an HTTP status is classified like one.
        let rate_limited = sse_error_to_llm_error(
            &SseError {
                code: Some("429".to_string()),
                error_type: Some("rate_limit_error".to_string()),
                message: "Rate limit reached".to_string(),
            },
            "zai",
        );
        assert!(
            matches!(
                rate_limited,
                LlmError::Provider(ProviderError::RateLimit { .. })
            ),
            "{rate_limited:?}"
        );
    }

    /// backlog 1111: content and then the end of the stream, with no finish
    /// reason, ends the turn as `unknown`, not `stop`; a named finish reason
    /// followed by `[DONE]` stays the only one, in its canonical text
    /// (`length`, not the `Debug` name `Length`: bug-e3940b).
    #[tokio::test]
    async fn stream_without_finish_reason_is_not_stop() {
        let (base_url, server) =
            serve_sse_once("data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n");
        let items = streamed_turn(&base_url).await;
        server.join().expect("server thread");
        assert_eq!(finish_reasons(&items), vec!["unknown".to_string()]);

        let (base_url, server) = serve_sse_once(
            "data: {\"choices\":[{\"delta\":{\"content\":\"cut\"},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n",
        );
        let items = streamed_turn(&base_url).await;
        server.join().expect("server thread");
        assert_eq!(finish_reasons(&items), vec!["length".to_string()]);

        let (base_url, server) = serve_sse_once(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n",
        );
        let items = streamed_turn(&base_url).await;
        server.join().expect("server thread");
        assert_eq!(finish_reasons(&items), vec!["unknown".to_string()]);
    }

    /// bug-91796d: a streamed call has no total timeout. A reasoning model
    /// that streams for minutes ran into the 120 s `request_timeout` and broke
    /// with "read chunk failed"; the timeout now bounds each silence, so a
    /// stream that keeps delivering runs past it.
    #[tokio::test]
    async fn a_stream_that_keeps_delivering_outlives_the_request_timeout() {
        let config = TurnConfig {
            request_timeout: StdDuration::from_millis(800),
            ..TurnConfig::default()
        };
        let (base_url, server) = serve_paced_sse(vec![StdDuration::from_millis(200); 8]);
        let started = Instant::now();
        let items = streamed_turn_with(&base_url, &config).await;
        let elapsed = started.elapsed();
        server.join().expect("server thread");

        assert!(items.iter().all(Result::is_ok), "{items:?}");
        assert_eq!(streamed_text(&items), "x".repeat(8));
        assert_eq!(finish_reasons(&items), vec!["stop".to_string()]);
        assert!(
            elapsed > config.request_timeout,
            "the stream took {elapsed:?}"
        );
    }

    /// bug-91796d: a stream that goes silent for `request_timeout` fails as
    /// a timeout, with what it streamed before the stall delivered.
    #[tokio::test]
    async fn a_stalled_stream_fails_at_the_request_timeout() {
        let config = TurnConfig {
            request_timeout: StdDuration::from_millis(300),
            ..TurnConfig::default()
        };
        let gaps = vec![
            StdDuration::from_millis(10),
            StdDuration::from_millis(10),
            StdDuration::from_secs(3),
        ];
        let (base_url, server) = serve_paced_sse(gaps);
        let started = Instant::now();
        let items = streamed_turn_with(&base_url, &config).await;
        let elapsed = started.elapsed();

        assert_eq!(streamed_text(&items), "xx");
        let error = items
            .iter()
            .find_map(|item| item.as_ref().err())
            .expect("the stalled stream fails");
        assert!(matches!(error, LlmError::Timeout(_)), "{error:?}");
        assert!(error.to_string().contains("stalled"), "{error}");
        assert!(finish_reasons(&items).is_empty(), "{items:?}");
        assert!(
            elapsed < StdDuration::from_secs(2),
            "the stall was cut after {elapsed:?}"
        );
        server.join().expect("server thread");
    }

    /// The empty reasoning deltas among `items`: the call saying it is alive.
    fn alive_beats(items: &[Result<StreamEvent, LlmError>]) -> usize {
        items
            .iter()
            .filter(|item| {
                matches!(item, Ok(StreamEvent {
                    kind: StreamEventKind::ReasoningDelta(text),
                    ..
                }) if text.is_empty())
            })
            .count()
    }

    /// gap-1ba0ac: a model silent for 200 s while it reasons, past the
    /// provider's 120 s wait for a chunk, keeps its call: the call says every
    /// 30 s that it is alive, and waits up to 10 minutes for its model. A
    /// model silent past that is cut as stalled.
    #[tokio::test(start_paused = true)]
    async fn a_200_second_silent_reasoning_turn_is_not_cut() {
        let provider_wait = TurnConfig::default().request_timeout;
        assert!(provider_wait < StdDuration::from_secs(200));
        let reasoning = ReasoningWait {
            heartbeat: REASONING_HEARTBEAT,
            limit: REASONING_SILENCE.max(provider_wait),
            request_start: Instant::now(),
        };

        let (tx, mut rx) = mpsc::channel(64);
        let answer = async {
            tokio::time::sleep(StdDuration::from_secs(200)).await;
            "the answer"
        };
        let started = tokio::time::Instant::now();
        assert_eq!(
            reasoning.chunk(answer, &tx, "test").await,
            Some("the answer")
        );
        assert_eq!(started.elapsed(), StdDuration::from_secs(200));
        drop(tx);
        let mut items = Vec::new();
        while let Some(item) = rx.recv().await {
            items.push(item);
        }
        assert_eq!(alive_beats(&items), 6, "one each 30 s: {items:?}");
        assert_eq!(items.len(), 6, "{items:?}");

        let (tx, mut rx) = mpsc::channel(64);
        let started = tokio::time::Instant::now();
        let never = std::future::pending::<()>();
        assert_eq!(reasoning.chunk(never, &tx, "test").await, None);
        assert_eq!(started.elapsed(), REASONING_SILENCE);
        drop(tx);
        let mut items = Vec::new();
        while let Some(item) = rx.recv().await {
            items.push(item);
        }
        let Some(Err(error)) = items.pop() else {
            panic!("the call is cut: {items:?}");
        };
        assert!(matches!(error, LlmError::Timeout(_)), "{error:?}");
        assert!(error.to_string().contains("stalled"), "{error}");
        assert_eq!(alive_beats(&items), items.len(), "{items:?}");
        assert_eq!(items.len(), 19, "a beat each 30 s before the cut");
    }

    /// Serve one streamed Responses answer whose model reasons, sends
    /// nothing for `silence`, then answers "ok"; the server hands back the
    /// request it got.
    fn serve_silent_reasoning(silence: StdDuration) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("listener addr");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let request = read_http_request(&mut stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            )
            .expect("write response headers");
            let reasoning = serde_json::json!({
                "type": "reasoning", "id": "rs_1", "summary": [], "encrypted_content": "enc",
            });
            let message = serde_json::json!({
                "type": "message", "id": "msg_1", "role": "assistant", "status": "completed",
                "content": [{ "type": "output_text", "text": "ok" }],
            });
            for event in [
                serde_json::json!({ "type": "response.created",
                                    "response": { "model": "gpt-5.4-mini" } }),
                serde_json::json!({ "type": "response.in_progress", "response": {} }),
                serde_json::json!({ "type": "response.output_item.added",
                                    "item": { "type": "reasoning", "id": "rs_1" } }),
            ] {
                stream
                    .write_all(sse_json_line(event).as_bytes())
                    .expect("write the opening events");
            }
            stream.flush().expect("flush the opening events");
            thread::sleep(silence);
            // The client may have cut the call by now.
            for event in [
                serde_json::json!({ "type": "response.output_item.done", "item": reasoning }),
                serde_json::json!({ "type": "response.output_text.delta", "delta": "ok" }),
                serde_json::json!({ "type": "response.completed", "response": {
                    "model": "gpt-5.4-mini", "status": "completed",
                    "output": [reasoning, message],
                    "usage": { "input_tokens": 50, "output_tokens": 70,
                               "output_tokens_details": { "reasoning_tokens": 64 } },
                }}),
            ] {
                if stream.write_all(sse_json_line(event).as_bytes()).is_err() {
                    break;
                }
            }
            let _ = stream.flush();
            request
        });
        (format!("http://{addr}/v1/"), server)
    }

    /// Every item one streamed Responses turn against `base_url` yields,
    /// when the provider waits 600 ms for a chunk and the call says every
    /// 150 ms that its reasoning model is alive, for up to `silence`.
    async fn silent_reasoning_turn(
        base_url: &str,
        silence: StdDuration,
    ) -> Vec<Result<StreamEvent, LlmError>> {
        use futures::StreamExt;

        let config = TurnConfig {
            request_timeout: StdDuration::from_millis(600),
            ..TurnConfig::default()
        };
        OpenAiCompatLlmBackend::new("test-key", "gpt-5.4-mini")
            .with_base_url(base_url)
            .with_wire(OpenAiApi::Responses)
            .with_reasoning_waits(StdDuration::from_millis(150), silence)
            .stream_turn(
                &[serde_json::json!({ "role": "user", "content": "hello" })],
                &RenderedTools::JsonArray(serde_json::json!([])),
                &SessionState::default(),
                &config,
            )
            .await
            .expect("the request is sent")
            .collect()
            .await
    }

    /// gap-1ba0ac: a streamed Responses call whose model reasons in silence
    /// past the provider's wait for a chunk is not cut. From its first
    /// events on it says it is alive, and it streams the answer that
    /// follows. Silent past its reasoning wait, it is cut as stalled.
    #[tokio::test]
    async fn a_reasoning_responses_stream_outlasts_the_providers_chunk_wait() {
        let (base_url, server) = serve_silent_reasoning(StdDuration::from_millis(1500));
        let items = silent_reasoning_turn(&base_url, StdDuration::from_secs(5)).await;
        let request = server.join().expect("server thread");

        assert!(
            request.starts_with("POST /v1/responses "),
            "{}",
            request.lines().next().unwrap_or_default()
        );
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        assert_eq!(streamed_text(&items), "ok");
        assert_eq!(finish_reasons(&items), vec!["stop".to_string()]);
        // in_progress and the reasoning item's start, then the beats.
        assert!(alive_beats(&items) >= 2 + 4, "{items:?}");

        let (base_url, server) = serve_silent_reasoning(StdDuration::from_millis(2500));
        let started = Instant::now();
        let items = silent_reasoning_turn(&base_url, StdDuration::from_millis(900)).await;
        let elapsed = started.elapsed();
        server.join().expect("server thread");
        let error = items
            .iter()
            .find_map(|item| item.as_ref().err())
            .expect("the silent call is cut");
        assert!(error.to_string().contains("stalled"), "{error}");
        assert!(finish_reasons(&items).is_empty(), "{items:?}");
        assert!(
            elapsed < StdDuration::from_millis(2300),
            "the silence was cut after {elapsed:?}"
        );
    }

    /// Two Responses answers: the model reasons, then calls `echo` with
    /// `{"value":1}`; then it answers "final answer".
    fn responses_answers() -> [Value; 2] {
        let reasoning = serde_json::json!({
            "type": "reasoning", "id": "rs_1", "summary": [], "encrypted_content": "enc-1",
        });
        let call = serde_json::json!({
            "type": "function_call", "id": "fc_1", "call_id": "call_1", "name": "echo",
            "arguments": "{\"value\":1}", "status": "completed",
        });
        let message = serde_json::json!({
            "type": "message", "id": "msg_1", "role": "assistant", "status": "completed",
            "content": [{ "type": "output_text", "text": "final answer", "annotations": [] }],
        });
        [
            serde_json::json!({
                "id": "resp_1", "object": "response", "status": "completed",
                "model": "gpt-5.4-mini-2026-03-17", "output": [reasoning, call],
                "usage": {
                    "input_tokens": 11, "input_tokens_details": { "cached_tokens": 3 },
                    "output_tokens": 7, "output_tokens_details": { "reasoning_tokens": 5 },
                },
            }),
            serde_json::json!({
                "id": "resp_2", "object": "response", "status": "completed",
                "model": "gpt-5.4-mini-2026-03-17", "output": [message],
                "usage": { "input_tokens": 9, "output_tokens": 4 },
            }),
        ]
    }

    /// A tool loop's two Responses requests: stateless, with the system
    /// prompt as instructions, flat tools that are not strict, and the
    /// effort as `reasoning.effort`; the second replays the first answer's
    /// encrypted reasoning before its function call, then the call's output.
    fn assert_the_second_request_replays_the_first(first: &Value, second: &Value) {
        for body in [first, second] {
            assert_eq!(body["model"], "gpt-5.4-mini", "{body}");
            assert_eq!(body["store"], false, "{body}");
            assert_eq!(
                body["include"],
                serde_json::json!(["reasoning.encrypted_content"])
            );
            assert_eq!(body["instructions"], "system prompt", "{body}");
            assert_eq!(body["reasoning"], serde_json::json!({ "effort": "medium" }));
            assert_eq!(body["tools"][0]["type"], "function", "{body}");
            assert_eq!(body["tools"][0]["name"], "echo", "{body}");
            assert_eq!(body["tools"][0]["strict"], false, "{body}");
            for chat_only in ["messages", "reasoning_effort", "stream_options"] {
                assert!(body.get(chat_only).is_none(), "{chat_only}: {body}");
            }
        }
        let kinds = |body: &Value| -> Vec<String> {
            body["input"]
                .as_array()
                .expect("input items")
                .iter()
                .map(|item| item["type"].as_str().unwrap_or("message").to_string())
                .collect()
        };
        assert_eq!(kinds(first), ["message"], "{first}");
        assert_eq!(
            kinds(second),
            [
                "message",
                "reasoning",
                "function_call",
                "function_call_output"
            ],
            "{second}"
        );
        let input = &second["input"];
        assert_eq!(input[1]["encrypted_content"], "enc-1");
        assert_eq!(input[2]["call_id"], "call_1");
        assert_eq!(input[3]["call_id"], "call_1");
        assert_eq!(input[3]["output"], "{\"value\":1}");
    }

    fn medium_effort() -> Map<String, Value> {
        Map::from_iter([(
            crate::provider::openai_compat::REASONING_EFFORT.to_string(),
            serde_json::json!("medium"),
        )])
    }

    /// gap-1ba0ac: on the Responses API a tool loop's model reasons beside
    /// its tools, and each answer reads as chat completions: the call, the
    /// final text, and usage with its cached input and reasoning tokens.
    #[tokio::test]
    async fn a_responses_tool_loop_replays_its_reasoning() {
        let [first, second] = responses_answers();
        let (poster, requests) =
            MockPoster::new(vec![Ok(first.to_string()), Ok(second.to_string())]);
        let backend = OpenAiCompatLlmBackend::new("test-key", "gpt-5.4-mini")
            .with_wire(OpenAiApi::Responses)
            .with_extra_body_params(medium_effort())
            .with_poster(Box::new(poster));
        let result = make_tool_loop(backend)
            .run(
                "system prompt",
                "call the tool, then finish",
                &test_tools(),
                &ToolContext::testing("/tmp"),
            )
            .await;

        assert_eq!(result.stop_reason, StopReason::Stop);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_1");
        assert_eq!(result.tool_calls[0].name, "echo");
        assert_eq!(result.final_text, "final answer");
        let usage = result.total_usage;
        assert_eq!(
            (
                usage.input_tokens,
                usage.cache_read_tokens,
                usage.output_tokens,
                usage.reasoning_tokens
            ),
            (8 + 9, 3, 7 + 4, 5)
        );

        let requests = requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 2);
        for request in requests.iter() {
            assert_eq!(request.url, "https://api.openai.com/v1/responses");
        }
        assert_the_second_request_replays_the_first(&requests[0].body, &requests[1].body);
    }

    /// The SSE lines of `answer` streamed as Responses events, the way
    /// OpenAI streams them: its items as they start and finish, with text
    /// and arguments in deltas, then the completed response.
    fn responses_events(answer: &Value) -> Vec<String> {
        let mut events = vec![
            serde_json::json!({ "type": "response.created",
                                "response": { "id": answer["id"], "status": "in_progress" } }),
            serde_json::json!({ "type": "response.in_progress",
                                "response": { "id": answer["id"], "status": "in_progress" } }),
        ];
        for item in answer["output"].as_array().expect("output items") {
            let id = &item["id"];
            match item["type"].as_str() {
                Some("function_call") => {
                    let mut started = item.clone();
                    started["arguments"] = serde_json::json!("");
                    events.push(serde_json::json!({
                        "type": "response.output_item.added", "item": started }));
                    let arguments = item["arguments"].as_str().expect("arguments");
                    let (head, tail) = arguments.split_at(arguments.len() / 2);
                    for delta in [head, tail] {
                        events.push(serde_json::json!({
                            "type": "response.function_call_arguments.delta",
                            "item_id": id, "delta": delta }));
                    }
                }
                Some("message") => {
                    events.push(serde_json::json!({ "type": "response.output_item.added",
                        "item": { "type": "message", "id": id, "role": "assistant",
                                  "content": [] } }));
                    let text = item["content"][0]["text"].as_str().expect("text");
                    for delta in text.split_inclusive(' ') {
                        events.push(serde_json::json!({
                            "type": "response.output_text.delta",
                            "item_id": id, "delta": delta }));
                    }
                }
                _ => events.push(serde_json::json!({ "type": "response.output_item.added",
                    "item": { "type": item["type"], "id": id, "summary": [] } })),
            }
            events.push(serde_json::json!({ "type": "response.output_item.done", "item": item }));
        }
        events.push(serde_json::json!({ "type": "response.completed", "response": answer }));
        events
            .into_iter()
            .map(|event| {
                let kind = event["type"].as_str().unwrap_or_default().to_string();
                format!("event: {kind}\n{}", sse_json_line(event))
            })
            .collect()
    }

    /// gap-1ba0ac: a streamed tool loop on the Responses API: its calls go to
    /// `/responses` and stream, the reasoning each streamed answer carries is
    /// replayed with the next request, and the events and the result match
    /// the answered loop's.
    #[tokio::test]
    async fn a_streamed_responses_tool_loop_replays_its_reasoning() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("listener addr");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_server = Arc::clone(&requests);
        let server = thread::spawn(move || {
            for answer in responses_answers() {
                let (mut stream, _) = listener.accept().expect("accept request");
                let request = read_http_request(&mut stream);
                requests_for_server
                    .lock()
                    .expect("requests lock")
                    .push(request);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
                )
                .expect("write response headers");
                for event in responses_events(&answer) {
                    stream.write_all(event.as_bytes()).expect("write event");
                    stream.flush().expect("flush event");
                    thread::sleep(StdDuration::from_millis(5));
                }
            }
        });

        let backend = OpenAiCompatLlmBackend::new("test-key", "gpt-5.4-mini")
            .with_base_url(format!("http://{addr}/v1"))
            .with_wire(OpenAiApi::Responses)
            .with_extra_body_params(medium_effort())
            .with_timeout_ms(5_000);
        let tool_loop = make_tool_loop(backend);
        let tools = test_tools();
        let (event_tx, mut event_rx) = mpsc::channel(roko_core::defaults::DEFAULT_CHANNEL_BUFFER);
        let run = tokio::spawn(async move {
            let ctx = ToolContext::testing("/tmp");
            tool_loop
                .run_streaming(
                    "system prompt",
                    "call the tool, then finish",
                    &tools,
                    &ctx,
                    event_tx,
                )
                .await
        });
        let result = run.await.expect("tool loop task");
        let mut events = Vec::new();
        while let Some(event) = event_rx.recv().await {
            events.push(event.kind);
        }
        server.join().expect("server thread");

        assert_eq!(result.stop_reason, StopReason::Stop);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_1");
        assert_eq!(
            result.tool_calls[0].arguments,
            serde_json::json!({ "value": 1 })
        );
        assert_eq!(result.final_text, "final answer");
        let usage = result.total_usage;
        assert_eq!(
            (
                usage.input_tokens,
                usage.cache_read_tokens,
                usage.output_tokens,
                usage.reasoning_tokens
            ),
            (8 + 9, 3, 7 + 4, 5)
        );
        assert!(
            events.iter().any(|kind| matches!(kind,
                StreamEventKind::ToolCallStart { id, name } if id == "call_1" && name == "echo")),
            "{events:?}"
        );
        assert!(
            events
                .iter()
                .any(|kind| matches!(kind, StreamEventKind::TextDelta(text) if text == "final ")),
            "{events:?}"
        );

        let requests = requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 2);
        let bodies: Vec<Value> = requests
            .iter()
            .map(|request| {
                assert!(request.starts_with("POST /v1/responses "), "{request}");
                let (_, body) = request.split_once("\r\n\r\n").expect("a request body");
                serde_json::from_str(body).expect("a JSON body")
            })
            .collect();
        for body in &bodies {
            assert_eq!(body["stream"], true, "{body}");
        }
        assert_the_second_request_replays_the_first(&bodies[0], &bodies[1]);
    }

    #[tokio::test]
    async fn retry_after_header_429_maps_to_provider_rate_limit() {
        let (poster, _reqs) = MockPoster::new(vec![Err(HttpPostError::http_with_retry_after(
            429,
            "rate limited",
            Some(30),
        ))]);
        let backend =
            OpenAiCompatLlmBackend::new("test-key", "model").with_poster(Box::new(poster));
        let err = backend
            .send_turn(
                &[serde_json::json!({"role": "user", "content": "hi"})],
                &RenderedTools::JsonArray(serde_json::json!([])),
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

    /// A poster that answers `answer` once `gate` opens, so its call stays in
    /// flight until then.
    struct GatedPoster {
        gate: Arc<tokio::sync::Notify>,
        answer: String,
    }

    #[async_trait]
    impl HttpPoster for GatedPoster {
        async fn post_json(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            _body: &[u8],
            _timeout_ms: u64,
        ) -> Result<String, HttpPostError> {
            self.gate.notified().await;
            Ok(self.answer.clone())
        }
    }

    /// bug-4b6844, bug-33a61f: parallel calls to one provider share what its
    /// rate limits taught. With one call in flight the provider refused a
    /// second (HTTP 429), so it takes one at a time. A call held past its
    /// wait fails as rate-limited without reaching the provider. The second
    /// caller's next call waits for its slot, said on its live output as
    /// queued, and goes out once the first call has ended.
    #[tokio::test]
    async fn parallel_calls_share_a_providers_rate_limit() {
        let answer = serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "done"}, "finish_reason": "stop"}]
        })
        .to_string();
        let limiter = Arc::new(ProviderRateLimiter::new(6_000));
        let gate = Arc::new(tokio::sync::Notify::new());
        let first = OpenAiCompatLlmBackend::new("test-key", "model")
            .with_rate_limiter(Arc::clone(&limiter))
            .with_poster(Box::new(GatedPoster {
                gate: Arc::clone(&gate),
                answer: answer.clone(),
            }));
        let (poster, requests) = MockPoster::new(vec![
            Err(HttpPostError::http_with_retry_after(
                429,
                "rate limited",
                Some(1),
            )),
            Ok(answer),
        ]);
        let (sink, mut events) = tokio::sync::mpsc::channel(64);
        let second = OpenAiCompatLlmBackend::new("test-key", "model")
            .with_rate_limiter(Arc::clone(&limiter))
            .with_poster(Box::new(poster))
            .with_live_output(Some(crate::live_output::LiveOutput {
                sink,
                trusted: true,
            }));
        let impatient = OpenAiCompatLlmBackend::new("test-key", "model")
            .with_rate_limiter(Arc::clone(&limiter))
            .with_poster(Box::new(MockPoster::new(Vec::new()).0))
            .with_held_call_wait(Duration::from_millis(300));
        let messages = [serde_json::json!({"role": "user", "content": "hi"})];
        let tools = RenderedTools::JsonArray(serde_json::json!([]));
        let session = SessionState::default();

        let in_flight = tokio::spawn(async move {
            let messages = [serde_json::json!({"role": "user", "content": "hi"})];
            first
                .send_turn(
                    &messages,
                    &RenderedTools::JsonArray(serde_json::json!([])),
                    &SessionState::default(),
                )
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let refused = second.send_turn(&messages, &tools, &session).await;
        assert!(
            matches!(
                refused,
                Err(LlmError::Provider(ProviderError::RateLimit { .. }))
            ),
            "{refused:?}"
        );
        let held = impatient.send_turn(&messages, &tools, &session).await;
        assert!(
            matches!(
                held,
                Err(LlmError::Provider(ProviderError::RateLimit {
                    retry_after_ms: None
                }))
            ),
            "{held:?}"
        );

        let waiting = tokio::spawn(async move {
            let messages = [serde_json::json!({"role": "user", "content": "hi"})];
            second
                .send_turn(
                    &messages,
                    &RenderedTools::JsonArray(serde_json::json!([])),
                    &SessionState::default(),
                )
                .await
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            requests.lock().expect("requests lock").len(),
            1,
            "the held calls never reached the provider"
        );
        assert!(matches!(
            events.try_recv(),
            Ok(crate::live_output::LiveAgentEvent::Queued { waiting: true })
        ));

        gate.notify_one();
        in_flight
            .await
            .expect("the first call's task")
            .expect("the first call is served");
        waiting
            .await
            .expect("the held call's task")
            .expect("with the first call ended, the held call goes out");
        assert_eq!(requests.lock().expect("requests lock").len(), 2);
        let mut said = Vec::new();
        while let Ok(crate::live_output::LiveAgentEvent::Queued { waiting }) = events.try_recv() {
            said.push(waiting);
        }
        assert_eq!(said.last(), Some(&false), "{said:?}");
        assert!(
            said[..said.len() - 1].iter().all(|waiting| *waiting),
            "{said:?}"
        );
    }

    #[tokio::test]
    async fn retry_after_header_529_maps_to_provider_rate_limit() {
        let (poster, _reqs) = MockPoster::new(vec![Err(HttpPostError::http_with_retry_after(
            529,
            "overloaded",
            Some(60),
        ))]);
        let backend =
            OpenAiCompatLlmBackend::new("test-key", "model").with_poster(Box::new(poster));
        let err = backend
            .send_turn(
                &[serde_json::json!({"role": "user", "content": "hi"})],
                &RenderedTools::JsonArray(serde_json::json!([])),
                &SessionState::default(),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                LlmError::Provider(ProviderError::RateLimit {
                    retry_after_ms: Some(60_000)
                })
            ),
            "expected Provider(RateLimit {{ 60_000ms }}), got {err:?}"
        );
    }

    #[tokio::test]
    async fn retry_after_header_absent_produces_rate_limit_none() {
        let (poster, _reqs) = MockPoster::new(vec![Err(HttpPostError::http(
            429,
            "rate limited no header",
        ))]);
        let backend =
            OpenAiCompatLlmBackend::new("test-key", "model").with_poster(Box::new(poster));
        let err = backend
            .send_turn(
                &[serde_json::json!({"role": "user", "content": "hi"})],
                &RenderedTools::JsonArray(serde_json::json!([])),
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

    // ── classify_http_error unit tests ───────────────────────────────────

    #[test]
    fn classify_http_error_429_maps_to_rate_limit() {
        let err = classify_http_error(HttpPostError::http(429, "rate limited"), "test");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::RateLimit { .. })),
            "expected Provider(RateLimit), got {err:?}"
        );
    }

    /// bug-390e4b: a 402 is the provider's refusal to bill the account, an
    /// exhaustion with its own words and code, as Cerebras sends it.
    #[test]
    fn classify_http_error_402_is_a_billing_refusal() {
        let body = r#"{"message":"Payment required to access this resource. Visit your billing tab.","type":"payment_required_error","param":"quota","code":"payment_required"}"#;
        let err = classify_http_error(HttpPostError::http(402, body), "test");
        let LlmError::Provider(ProviderError::ProviderExhausted {
            resets_at_ms,
            message,
        }) = &err
        else {
            panic!("expected a billing refusal for 402, got {err:?}");
        };
        assert_eq!(*resets_at_ms, None);
        assert_eq!(
            message,
            "Payment required to access this resource. Visit your billing tab. (code \
             payment_required)"
        );
    }

    #[test]
    fn classify_http_error_429_with_retry_after() {
        let err = classify_http_error(
            HttpPostError::http_with_retry_after(429, "rate limited", Some(30)),
            "test",
        );
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

    #[test]
    fn classify_http_error_529_maps_to_rate_limit() {
        let err = classify_http_error(HttpPostError::http(529, "overloaded"), "test");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::RateLimit { .. })),
            "expected Provider(RateLimit), got {err:?}"
        );
    }

    #[test]
    fn classify_http_error_500_maps_to_server_error() {
        let err = classify_http_error(HttpPostError::http(500, "internal server error"), "test");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::ServerError(500))),
            "expected Provider(ServerError(500)), got {err:?}"
        );
    }

    #[test]
    fn classify_http_error_503_maps_to_server_error() {
        let err = classify_http_error(HttpPostError::http(503, "service unavailable"), "test");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::ServerError(503))),
            "expected Provider(ServerError(503)), got {err:?}"
        );
    }

    #[test]
    fn classify_http_error_401_maps_to_auth_failure() {
        let err = classify_http_error(HttpPostError::http(401, "unauthorized"), "test");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::AuthFailure)),
            "expected Provider(AuthFailure), got {err:?}"
        );
    }

    /// bug-35b190: a provider's HTTP 4xx refusal reads as a refusal, in the
    /// provider's own words, and not as a network error. OpenAI's 400 on
    /// reasoning_effort in phase 4's run 16fc88ae came out as "network error:
    /// http 400: {…}", class `provider_error:unknown`. A 401 stays an auth
    /// failure, a 404 is an unknown model, a 400 about the context a context
    /// overflow, and a transport error or a timeout stays what it was.
    #[test]
    fn a_providers_4xx_refusal_reads_as_a_refusal_not_a_network_error() {
        use crate::provider::error_classify::classify_failure_text;
        // The class of the error as an attempt's failure text carries it.
        let class = |err: &LlmError| {
            classify_failure_text(&format!("agent error (openai): {err}").to_ascii_lowercase())
        };

        let body = r#"{"error":{"message":"Function tools with reasoning_effort are not supported for gpt-5.4-mini in /v1/chat/completions.","type":"invalid_request_error","param":"reasoning_effort","code":null}}"#;
        let err = classify_http_error(HttpPostError::http(400, body), "openai");
        let LlmError::Provider(ProviderError::RequestRefused {
            provider,
            status,
            message,
        }) = &err
        else {
            panic!("expected a refused request, got {err:?}");
        };
        assert_eq!((provider.as_str(), *status), ("openai", 400));
        assert_eq!(
            message,
            "Function tools with reasoning_effort are not supported for gpt-5.4-mini in \
             /v1/chat/completions. (param reasoning_effort)"
        );
        assert_eq!(
            err.to_string(),
            "provider error: openai refused the request (HTTP 400): Function tools with \
             reasoning_effort are not supported for gpt-5.4-mini in /v1/chat/completions. \
             (param reasoning_effort)"
        );
        assert_eq!(class(&err), "request_refused");

        let bad_key =
            r#"{"error":{"message":"Incorrect API key provided.","code":"invalid_api_key"}}"#;
        let err = classify_http_error(HttpPostError::http(401, bad_key), "openai");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::AuthFailure)),
            "{err:?}"
        );
        assert_eq!(class(&err), "auth_failure");

        // A 403 is "you may not do this", most often scoped to the model: a
        // refusal, unless its words say the credential itself is bad
        // (bug-80f0d0).
        let no_access = r#"{"error":{"message":"Project `proj_1` does not have access to model `gpt-5.4`","code":"model_not_found"}}"#;
        let err = classify_http_error(HttpPostError::http(403, no_access), "openai");
        assert_eq!(class(&err), "request_refused", "{err}");
        assert!(
            err.to_string()
                .contains("does not have access to model `gpt-5.4`"),
            "{err}"
        );
        let revoked = r#"{"error":{"message":"Invalid API key: this key has been revoked.","code":"invalid_api_key"}}"#;
        let err = classify_http_error(HttpPostError::http(403, revoked), "openai");
        assert_eq!(class(&err), "auth_failure", "{err}");
        assert!(err.to_string().contains("has been revoked"), "{err}");

        let no_model = r#"{"error":{"message":"The model `gpt-5.4-minii` does not exist or you do not have access to it.","code":"model_not_found"}}"#;
        let err = classify_http_error(HttpPostError::http(404, no_model), "openai");
        assert_eq!(class(&err), "model_not_found", "{err}");

        let too_long = r#"{"error":{"message":"This model's maximum context length is 128000 tokens.","code":"context_length_exceeded"}}"#;
        let err = classify_http_error(HttpPostError::http(400, too_long), "openai");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::ContextOverflow)),
            "{err:?}"
        );

        let err = classify_http_error(
            HttpPostError::transport("tcp connect error: Connection refused"),
            "openai",
        );
        assert!(matches!(err, LlmError::Network(_)), "{err:?}");
        let err = classify_http_error(HttpPostError::transport("operation timed out"), "openai");
        assert!(matches!(err, LlmError::Network(_)), "{err:?}");
        assert_eq!(class(&err), "timeout");
        let err = classify_http_error(HttpPostError::http(408, "Request Timeout"), "openai");
        assert!(
            matches!(err, LlmError::Provider(ProviderError::Timeout)),
            "{err:?}"
        );
    }

    #[test]
    fn classify_http_error_transport_maps_to_network() {
        let err = classify_http_error(HttpPostError::transport("dns failed"), "test");
        assert!(
            matches!(err, LlmError::Network(_)),
            "expected Network, got {err:?}"
        );
    }
}
