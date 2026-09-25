//! RAG-08: Provider-neutral dense embedding adapter.
//!
//! Defines the [`DenseEmbeddingAdapter`] trait and two built-in implementations:
//!
//! - [`NoopEmbeddingAdapter`] — returns zero-vectors; used when
//!   `[retrieval] dense_embedding_enabled = false` (the default).
//! - [`OpenAiCompatEmbeddingAdapter`] — calls `POST /v1/embeddings` on any
//!   OpenAI-compatible endpoint; enabled when `dense_embedding_enabled = true`.
//!
//! The adapter is selected at runtime by [`dense_embedding_adapter_for_config`],
//! which inspects `RetrievalConfig` and returns a boxed trait object.

use async_trait::async_trait;
use roko_agent::http::{HttpPoster, ReqwestPoster};
use roko_core::config::RetrievalConfig;
use serde_json::{Value, json};

/// Error returned by embedding calls.
#[derive(Debug, thiserror::Error)]
pub enum EmbedAdapterError {
    /// HTTP transport or status error.
    #[error("http error: {0}")]
    Http(String),
    /// Failed to serialize the request body.
    #[error("serialize error: {0}")]
    Serialize(String),
    /// The API returned a non-successful response.
    #[error("api error: {0}")]
    Api(String),
    /// The response could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),
}

/// Provider-neutral interface for generating dense vector embeddings.
///
/// Implementations must be `Send + Sync` so they can be stored behind an `Arc`
/// and shared across async tasks.
///
/// # Enabling
///
/// Set `[retrieval] dense_embedding_enabled = true` in `roko.toml` to activate
/// a real adapter.  The default [`NoopEmbeddingAdapter`] is a zero-cost
/// stand-in that produces zero-vectors without making any network calls.
#[async_trait]
pub trait DenseEmbeddingAdapter: Send + Sync {
    /// Embed a batch of texts, returning one float vector per input.
    ///
    /// The returned vectors are in the same order as `texts`.
    ///
    /// # Errors
    ///
    /// Returns [`EmbedAdapterError`] on transport, serialization, or API errors.
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedAdapterError>;

    /// Human-readable identifier for the underlying model.
    fn model_name(&self) -> &str;

    /// Number of dimensions in the output vectors.
    ///
    /// Returns `0` for the noop adapter.
    fn dimensions(&self) -> usize;
}

// ── NoopEmbeddingAdapter ──────────────────────────────────────────────────────

/// A no-op embedding adapter that returns zero-vectors.
///
/// Used when `[retrieval] dense_embedding_enabled = false` (the default).
/// Callers that gate on the config flag will never reach this adapter in
/// practice, but having a safe default avoids `Option` boilerplate at every
/// call site.
#[derive(Clone, Debug, Default)]
pub struct NoopEmbeddingAdapter;

#[async_trait]
impl DenseEmbeddingAdapter for NoopEmbeddingAdapter {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedAdapterError> {
        // One empty vector per input — callers should check `dimensions() == 0`
        // or the `dense_embedding_enabled` flag before treating these as real
        // embeddings.
        Ok(vec![Vec::new(); texts.len()])
    }

    fn model_name(&self) -> &str {
        "noop"
    }

    fn dimensions(&self) -> usize {
        0
    }
}

// ── OpenAiCompatEmbeddingAdapter ──────────────────────────────────────────────

/// Default model used when none is specified.
const DEFAULT_OPENAI_EMBED_MODEL: &str = "text-embedding-3-small";
/// Output dimensions for the default model.
const DEFAULT_OPENAI_EMBED_DIMS: usize = 1536;
/// Per-request timeout in milliseconds (30 s is plenty for embeddings).
const DEFAULT_EMBED_TIMEOUT_MS: u64 = 30_000;

/// An embedding adapter that calls any OpenAI-compatible `/v1/embeddings`
/// endpoint.
///
/// Compatible with:
/// - OpenAI (`https://api.openai.com/v1`)
/// - Azure OpenAI
/// - Mistral, Together, Fireworks, and other OpenAI-compat hosts
/// - Local servers such as `llama.cpp` with the OpenAI compat layer
///
/// # Configuration
///
/// ```toml
/// [retrieval]
/// dense_embedding_enabled = true
/// ```
///
/// The `base_url` and `api_key` are typically supplied from provider config at
/// the call site; see [`dense_embedding_adapter_for_config`] for the
/// factory helper.
pub struct OpenAiCompatEmbeddingAdapter {
    api_key: String,
    base_url: String,
    model_slug: String,
    dimensions: usize,
    timeout_ms: u64,
    poster: Box<dyn HttpPoster>,
}

impl OpenAiCompatEmbeddingAdapter {
    /// Construct with explicit parameters and a production reqwest poster.
    #[must_use]
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model_slug: impl Into<String>,
        dimensions: usize,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model_slug: model_slug.into(),
            dimensions,
            timeout_ms: DEFAULT_EMBED_TIMEOUT_MS,
            poster: Box::new(ReqwestPoster::new()),
        }
    }

    /// Override the per-request timeout.
    #[must_use]
    pub const fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Override the HTTP poster (useful for tests).
    #[cfg(test)]
    pub fn with_poster(mut self, poster: Box<dyn HttpPoster>) -> Self {
        self.poster = poster;
        self
    }

    fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{base}/v1/embeddings")
    }

    fn auth_headers(&self) -> Vec<(String, String)> {
        vec![
            (
                "Authorization".to_string(),
                format!("Bearer {}", self.api_key),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]
    }

    async fn call_api(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedAdapterError> {
        let body: Value = json!({
            "model": self.model_slug,
            "input": texts,
        });
        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| EmbedAdapterError::Serialize(e.to_string()))?;

        let response_text = self
            .poster
            .post_json(
                &self.endpoint(),
                &self.auth_headers(),
                &body_bytes,
                self.timeout_ms,
            )
            .await
            .map_err(|e| EmbedAdapterError::Http(e.to_string()))?;

        let parsed: Value = serde_json::from_str(&response_text)
            .map_err(|e| EmbedAdapterError::Parse(format!("malformed response: {e}")))?;

        if let Some(err) = parsed.get("error") {
            let msg = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown api error");
            return Err(EmbedAdapterError::Api(msg.to_string()));
        }

        let data = parsed
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| EmbedAdapterError::Parse("response missing 'data' array".to_string()))?;

        let mut result = Vec::with_capacity(data.len());
        for item in data {
            let raw = item
                .get("embedding")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    EmbedAdapterError::Parse("data item missing 'embedding' array".to_string())
                })?;
            let floats: Vec<f32> = raw
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            result.push(floats);
        }
        Ok(result)
    }
}

#[async_trait]
impl DenseEmbeddingAdapter for OpenAiCompatEmbeddingAdapter {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedAdapterError> {
        self.call_api(texts).await
    }

    fn model_name(&self) -> &str {
        &self.model_slug
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

// ── Factory ───────────────────────────────────────────────────────────────────

/// Construct a boxed [`DenseEmbeddingAdapter`] appropriate for the given
/// retrieval config.
///
/// Returns a [`NoopEmbeddingAdapter`] when `dense_embedding_enabled` is `false`
/// (the default).  When enabled, returns an [`OpenAiCompatEmbeddingAdapter`]
/// pointed at `base_url` with the supplied `api_key`.
///
/// The caller is responsible for sourcing credentials from the provider config;
/// this function only selects the implementation based on the retrieval flag.
#[must_use]
pub fn dense_embedding_adapter_for_config(
    config: &RetrievalConfig,
    api_key: &str,
    base_url: &str,
) -> Box<dyn DenseEmbeddingAdapter> {
    if config.dense_embedding_enabled {
        Box::new(OpenAiCompatEmbeddingAdapter::new(
            api_key,
            base_url,
            DEFAULT_OPENAI_EMBED_MODEL,
            DEFAULT_OPENAI_EMBED_DIMS,
        ))
    } else {
        Box::new(NoopEmbeddingAdapter)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::disallowed_types)]
mod tests {
    use super::*;
    use roko_agent::http::HttpPostError;
    use std::sync::{Arc, Mutex};

    // ── Helpers ───────────────────────────────────────────────────────────────

    #[derive(Clone, Debug, Default)]
    struct Captured {
        url: String,
        body: Vec<u8>,
    }

    struct MockPoster {
        captured: Arc<Mutex<Option<Captured>>>,
        response: Result<String, HttpPostError>,
    }

    impl MockPoster {
        fn ok(body: impl Into<String>) -> (Self, Arc<Mutex<Option<Captured>>>) {
            let captured = Arc::new(Mutex::new(None));
            (
                Self {
                    captured: captured.clone(),
                    response: Ok(body.into()),
                },
                captured,
            )
        }

        fn err(msg: impl Into<String>) -> (Self, Arc<Mutex<Option<Captured>>>) {
            let captured = Arc::new(Mutex::new(None));
            (
                Self {
                    captured: captured.clone(),
                    response: Err(HttpPostError::transport(msg)),
                },
                captured,
            )
        }
    }

    #[async_trait]
    impl HttpPoster for MockPoster {
        async fn post_json(
            &self,
            url: &str,
            _headers: &[(String, String)],
            body: &[u8],
            _timeout_ms: u64,
        ) -> Result<String, HttpPostError> {
            *self.captured.lock().expect("lock") = Some(Captured {
                url: url.to_string(),
                body: body.to_vec(),
            });
            self.response.clone()
        }
    }

    fn adapter_with_mock(poster: Box<dyn HttpPoster>) -> OpenAiCompatEmbeddingAdapter {
        OpenAiCompatEmbeddingAdapter::new(
            "sk-test",
            "https://api.openai.com",
            DEFAULT_OPENAI_EMBED_MODEL,
            DEFAULT_OPENAI_EMBED_DIMS,
        )
        .with_poster(poster)
    }

    fn canned_response(vecs: &[Vec<f32>]) -> String {
        let data: Vec<serde_json::Value> = vecs
            .iter()
            .enumerate()
            .map(|(i, v)| {
                json!({
                    "object": "embedding",
                    "index": i,
                    "embedding": v,
                })
            })
            .collect();
        json!({
            "object": "list",
            "data": data,
            "model": DEFAULT_OPENAI_EMBED_MODEL,
            "usage": { "prompt_tokens": 4, "total_tokens": 4 }
        })
        .to_string()
    }

    // ── NoopEmbeddingAdapter ──────────────────────────────────────────────────

    #[tokio::test]
    async fn noop_adapter_returns_empty_vectors() {
        let adapter = NoopEmbeddingAdapter;
        let result = adapter.embed(&["hello", "world"]).await.expect("ok");
        assert_eq!(result.len(), 2);
        assert!(result[0].is_empty());
        assert!(result[1].is_empty());
    }

    #[test]
    fn noop_adapter_dimensions_is_zero() {
        let adapter = NoopEmbeddingAdapter;
        assert_eq!(adapter.dimensions(), 0);
        assert_eq!(adapter.model_name(), "noop");
    }

    // ── OpenAiCompatEmbeddingAdapter ──────────────────────────────────────────

    #[tokio::test]
    async fn openai_compat_returns_float_vectors() {
        let expected = vec![vec![0.1_f32, 0.2, 0.3], vec![0.4_f32, 0.5, 0.6]];
        let (mock, _) = MockPoster::ok(canned_response(&expected));
        let adapter = adapter_with_mock(Box::new(mock));
        let result = adapter.embed(&["a", "b"]).await.expect("ok");
        assert_eq!(result.len(), 2);
        assert!((result[0][0] - 0.1_f32).abs() < 1e-5);
        assert!((result[1][2] - 0.6_f32).abs() < 1e-5);
    }

    #[tokio::test]
    async fn openai_compat_posts_to_v1_embeddings() {
        let (mock, captured) = MockPoster::ok(canned_response(&[vec![0.0]]));
        let adapter = adapter_with_mock(Box::new(mock));
        let _ = adapter.embed(&["test"]).await.expect("ok");
        let cap = captured.lock().expect("lock").clone().expect("captured");
        assert_eq!(cap.url, "https://api.openai.com/v1/embeddings");
        let body: serde_json::Value = serde_json::from_slice(&cap.body).expect("body is json");
        assert_eq!(body["model"], DEFAULT_OPENAI_EMBED_MODEL);
        assert_eq!(body["input"][0], "test");
    }

    #[tokio::test]
    async fn openai_compat_http_error_surfaced() {
        let (mock, _) = MockPoster::err("timeout");
        let adapter = adapter_with_mock(Box::new(mock));
        let err = adapter.embed(&["x"]).await.expect_err("should fail");
        assert!(matches!(err, EmbedAdapterError::Http(_)));
    }

    #[tokio::test]
    async fn openai_compat_api_error_surfaced() {
        let body = json!({ "error": { "message": "invalid key" } }).to_string();
        let (mock, _) = MockPoster::ok(body);
        let adapter = adapter_with_mock(Box::new(mock));
        let err = adapter.embed(&["x"]).await.expect_err("should fail");
        assert!(matches!(err, EmbedAdapterError::Api(_)));
    }

    #[test]
    fn openai_compat_dimensions_and_model() {
        let adapter = OpenAiCompatEmbeddingAdapter::new("key", "http://localhost", "my-model", 768);
        assert_eq!(adapter.model_name(), "my-model");
        assert_eq!(adapter.dimensions(), 768);
    }

    // ── Factory ───────────────────────────────────────────────────────────────

    #[test]
    fn factory_returns_noop_when_disabled() {
        let config = RetrievalConfig::default(); // dense_embedding_enabled = false
        let adapter = dense_embedding_adapter_for_config(&config, "key", "http://localhost");
        assert_eq!(adapter.model_name(), "noop");
        assert_eq!(adapter.dimensions(), 0);
    }

    #[test]
    fn factory_returns_openai_when_enabled() {
        let mut config = RetrievalConfig::default();
        config.dense_embedding_enabled = true;
        let adapter = dense_embedding_adapter_for_config(&config, "key", "https://api.openai.com");
        assert_eq!(adapter.model_name(), DEFAULT_OPENAI_EMBED_MODEL);
        assert_eq!(adapter.dimensions(), DEFAULT_OPENAI_EMBED_DIMS);
    }
}
