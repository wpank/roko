//! Provider contracts, lock-free credential rotation, and live ModelCaller adapter.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::{StreamExt, stream};
use roko_core::foundation::{ModelCallRequest, ModelCaller, ModelStreamEvent};

use crate::{
    GatewayError, GatewayResult, InferenceChunk, InferenceRequest, InferenceResponse,
    ProviderFailureKind, StopReason, TokenUsage,
};

/// Non-empty provider credential ring with lock-free rotation.
pub struct KeyRing {
    keys: Vec<String>,
    active: AtomicUsize,
}

impl KeyRing {
    /// Construct a non-empty ring.
    pub fn new(keys: Vec<String>) -> GatewayResult<Self> {
        if keys.is_empty() {
            return Err(GatewayError::Provider {
                provider: "key-ring".into(),
                kind: ProviderFailureKind::Fatal,
                message: "at least one provider credential is required".into(),
            });
        }
        Ok(Self {
            keys,
            active: AtomicUsize::new(0),
        })
    }

    /// Read the active entry. Construction guarantees the ring is non-empty.
    #[must_use]
    pub fn current(&self) -> &str {
        let index = self.active.load(Ordering::Relaxed) % self.keys.len();
        self.keys.get(index).map_or("", String::as_str)
    }

    /// Advance the active entry without locking.
    pub fn rotate(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }

    /// Current wrapped index for diagnostics without exposing entry material.
    #[must_use]
    pub fn active_index(&self) -> usize {
        self.active.load(Ordering::Relaxed) % self.keys.len()
    }
}

/// Backend selected after CascadeRouter chooses a model.
#[async_trait]
pub trait ProviderBackend: Send + Sync {
    /// Stable provider name used by backpressure and telemetry.
    fn name(&self) -> &str;
    /// Whether this backend can serve the model slug.
    fn supports_model(&self, model: &str) -> bool;
    /// Complete a request.
    async fn complete(&self, request: &InferenceRequest) -> GatewayResult<InferenceResponse>;
    /// Stream a request.
    async fn stream(
        &self,
        request: &InferenceRequest,
    ) -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>>;
    /// Rotate provider credentials after a rate-limit response.
    fn rotate_key(&self) {}
}

/// Production adapter from the existing live provider dispatch boundary.
pub struct ModelCallerBackend {
    name: String,
    model_prefixes: Vec<String>,
    caller: Arc<dyn ModelCaller>,
}

impl ModelCallerBackend {
    /// Bind a provider name and model-family prefixes to a live ModelCaller.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        model_prefixes: Vec<String>,
        caller: Arc<dyn ModelCaller>,
    ) -> Self {
        Self {
            name: name.into(),
            model_prefixes,
            caller,
        }
    }

    /// Anthropic family adapter.
    #[must_use]
    pub fn anthropic(caller: Arc<dyn ModelCaller>) -> Self {
        Self::new("anthropic", vec!["claude-".into()], caller)
    }

    /// OpenAI family adapter.
    #[must_use]
    pub fn openai(caller: Arc<dyn ModelCaller>) -> Self {
        Self::new(
            "openai",
            vec!["gpt-".into(), "o1".into(), "o3-".into(), "o4-".into()],
            caller,
        )
    }

    /// Catch-all adapter for configured providers not covered by built-in families.
    #[must_use]
    pub fn catch_all(name: impl Into<String>, caller: Arc<dyn ModelCaller>) -> Self {
        Self::new(name, Vec::new(), caller)
    }

    fn request(&self, request: &InferenceRequest) -> ModelCallRequest {
        ModelCallRequest {
            model: request.model.clone(),
            messages: request.messages.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            role: request.metadata.agent_role.clone(),
            caller: Some(request.metadata.agent_id.clone()),
            run_id: Some(request.metadata.session_id.clone()),
            budget_remaining: Some(request.metadata.budget_remaining as f64 / 1_000_000.0),
            tools: request.tools.clone().unwrap_or_default(),
            ..ModelCallRequest::default()
        }
    }
}

#[async_trait]
impl ProviderBackend for ModelCallerBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn supports_model(&self, model: &str) -> bool {
        self.model_prefixes.is_empty()
            || self
                .model_prefixes
                .iter()
                .any(|prefix| model.starts_with(prefix))
    }

    async fn complete(&self, request: &InferenceRequest) -> GatewayResult<InferenceResponse> {
        let started = std::time::Instant::now();
        let response = self
            .caller
            .call(self.request(request))
            .await
            .map_err(|error| classify_provider_error(&self.name, error.to_string()))?;
        Ok(InferenceResponse {
            text: response.content,
            stop_reason: parse_stop_reason(response.stop_reason.as_deref()),
            usage: TokenUsage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
                ..TokenUsage::default()
            },
            model: response.model,
            latency_ms: started.elapsed().as_millis() as u64,
            fallback: false,
            original_model: None,
        })
    }

    async fn stream(
        &self,
        request: &InferenceRequest,
    ) -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>> {
        let model = request.model.clone();
        let provider = self.name.clone();
        let stream = self
            .caller
            .stream(self.request(request))
            .await
            .map_err(|error| classify_provider_error(&self.name, error.to_string()))?;
        Ok(Box::pin(stream.filter_map(move |event| {
            let model = model.clone();
            let provider = provider.clone();
            async move {
                match event {
                    ModelStreamEvent::Started { .. } => None,
                    ModelStreamEvent::ContentDelta { text } => Some(Ok(InferenceChunk {
                        delta: text,
                        model,
                        ..InferenceChunk::default()
                    })),
                    ModelStreamEvent::Usage { usage } => Some(Ok(InferenceChunk {
                        usage: Some(TokenUsage {
                            input_tokens: usage.input_tokens,
                            output_tokens: usage.output_tokens,
                            ..TokenUsage::default()
                        }),
                        model,
                        ..InferenceChunk::default()
                    })),
                    ModelStreamEvent::Completed { stop_reason } => Some(Ok(InferenceChunk {
                        stop_reason: Some(parse_stop_reason(stop_reason.as_deref())),
                        model,
                        done: true,
                        ..InferenceChunk::default()
                    })),
                    ModelStreamEvent::Failed { error }
                    | ModelStreamEvent::AttemptFailed { error, .. } => {
                        Some(Err(classify_provider_error(&provider, error)))
                    }
                    ModelStreamEvent::Cancelled => Some(Err(GatewayError::Provider {
                        provider,
                        kind: ProviderFailureKind::Fatal,
                        message: "provider stream cancelled".into(),
                    })),
                }
            }
        })))
    }
}

fn parse_stop_reason(reason: Option<&str>) -> StopReason {
    match reason.unwrap_or_default().to_ascii_lowercase().as_str() {
        "max_tokens" | "length" => StopReason::MaxTokens,
        "tool_use" | "tool_calls" => StopReason::ToolUse,
        "content_filter" | "safety" => StopReason::ContentFilter,
        _ => StopReason::EndTurn,
    }
}

/// Classify existing provider errors into the gateway fallback taxonomy.
#[must_use]
pub fn classify_provider_error(provider: &str, message: String) -> GatewayError {
    let normalized = message.to_ascii_lowercase();
    let kind = if normalized.contains("429") || normalized.contains("rate limit") {
        ProviderFailureKind::RateLimited
    } else if normalized.contains("timeout") || normalized.contains("timed out") {
        ProviderFailureKind::Timeout
    } else if normalized.contains("503")
        || normalized.contains("unavailable")
        || normalized.contains("overloaded")
    {
        ProviderFailureKind::Unavailable
    } else {
        ProviderFailureKind::Fatal
    };
    GatewayError::Provider {
        provider: provider.to_string(),
        kind,
        message,
    }
}

/// Empty stream helper for providers that reject before yielding an item.
pub fn failed_stream(error: GatewayError) -> BoxStream<'static, GatewayResult<InferenceChunk>> {
    Box::pin(stream::once(async move { Err(error) }))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use futures::StreamExt;
    use roko_core::foundation::{ModelCallResponse, TokenUsage as CoreTokenUsage};

    use super::*;

    #[test]
    fn provider_key_ring_rotates_lock_free_and_wraps() {
        let ring = KeyRing::new(vec!["first".into(), "second".into()]).unwrap();
        assert_eq!(ring.active_index(), 0);
        assert_eq!(ring.current(), "first");
        ring.rotate();
        assert_eq!(ring.current(), "second");
        ring.rotate();
        assert_eq!(ring.current(), "first");
    }

    #[test]
    fn provider_key_ring_concurrent_rotation_does_not_lose_updates() {
        let ring = Arc::new(KeyRing::new(vec!["a".into(), "b".into(), "c".into()]).unwrap());
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let ring = Arc::clone(&ring);
                scope.spawn(move || {
                    for _ in 0..1_000 {
                        ring.rotate();
                    }
                });
            }
        });
        assert_eq!(ring.active.load(Ordering::Relaxed), 8_000);
        assert!(HashSet::from(["a", "b", "c"]).contains(ring.current()));
    }

    #[test]
    fn provider_error_classification_controls_fallback() {
        for message in ["HTTP 429", "timed out", "503 unavailable"] {
            let GatewayError::Provider { kind, .. } = classify_provider_error("p", message.into())
            else {
                panic!("provider error expected")
            };
            assert!(kind.is_retryable());
        }
        let GatewayError::Provider { kind, .. } =
            classify_provider_error("p", "invalid request".into())
        else {
            panic!("provider error expected")
        };
        assert!(!kind.is_retryable());
    }

    struct LiveCaller;

    #[async_trait]
    impl ModelCaller for LiveCaller {
        async fn call(&self, request: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
            Ok(ModelCallResponse {
                content: "adapter response".into(),
                model: request.model,
                usage: CoreTokenUsage {
                    input_tokens: 10,
                    output_tokens: 4,
                    total_tokens: 14,
                    cost_usd: 0.0,
                },
                stop_reason: Some("end_turn".into()),
                request_id: None,
            })
        }
    }

    #[tokio::test]
    async fn provider_model_caller_adapter_supports_complete_and_native_stream() {
        let backend = ModelCallerBackend::anthropic(Arc::new(LiveCaller));
        let request = InferenceRequest {
            model: "claude-adapter".into(),
            ..InferenceRequest::default()
        };
        let response = backend.complete(&request).await.unwrap();
        assert_eq!(response.text, "adapter response");
        assert_eq!(response.usage.output_tokens, 4);

        let chunks = backend
            .stream(&request)
            .await
            .unwrap()
            .collect::<Vec<_>>()
            .await;
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].as_ref().unwrap().delta, "adapter response");
        assert!(chunks[2].as_ref().unwrap().done);
    }
}
