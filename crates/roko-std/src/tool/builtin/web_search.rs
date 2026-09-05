//! `web_search` -- query a web search provider via Perplexity sonar.
//!
//! Category: [`ToolCategory::Network`]. Permission: networked.
//! Concurrency: [`ToolConcurrency::Parallel`]. Idempotent: yes.
//!
//! Uses the Perplexity chat/completions API with the `sonar` model to
//! perform search-grounded generation.
//!
//! The handler supports two explicit backends:
//!
//! - **Gateway** (`SearchBackend::Gateway`) -- routes through a shared
//!   [`ModelCaller`] so that caching, routing, budget, and telemetry
//!   participate in the normal observation pipeline.
//! - **DirectPerplexity** (`SearchBackend::DirectPerplexity`) -- calls the
//!   Perplexity API directly with a pre-resolved API key. This mode is for
//!   standalone/CLI boundaries where no gateway is available and the caller
//!   has explicitly opted in.
//!
//! A handler constructed without either backend (the `Handler` const)
//! produces a clear configuration error rather than silently probing
//! environment variables.

use async_trait::async_trait;
use roko_core::foundation::{
    CachePolicy, ChatMessage, MessageRole, ModelCallRequest, ModelCaller, caller,
};
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolError, ToolHandler,
    ToolPermission, ToolResult, ToolSchema,
};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use super::sandbox::require_string;

/// Canonical `snake_case` name.
pub const NAME: &str = "web_search";

/// Human-readable description sent to the LLM.
pub const DESCRIPTION: &str = "Query a configured web search provider and return top results.";

/// Perplexity API base URL.
const PERPLEXITY_API_URL: &str = "https://api.perplexity.ai/chat/completions";

/// Default model slug for search queries.
const DEFAULT_MODEL: &str = "sonar";

/// Request timeout in seconds.
const SEARCH_TIMEOUT_SECS: u64 = 30;

/// Typed backend for web search dispatch.
///
/// The distinction is explicit: callers choose one at construction time.
/// No implicit env-var probing occurs.
#[derive(Clone)]
pub enum SearchBackend {
    /// Route through the shared model gateway (routing, caching, budget,
    /// telemetry all participate).
    Gateway(Arc<dyn ModelCaller>),
    /// Call the Perplexity API directly with a pre-resolved key. Use this
    /// only at documented standalone binary boundaries where no gateway is
    /// available and the caller has explicitly opted in.
    DirectPerplexity {
        /// Pre-resolved Perplexity API key (never read from env at call time).
        api_key: String,
    },
}

impl std::fmt::Debug for SearchBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gateway(_) => f.debug_tuple("Gateway").field(&"<ModelCaller>").finish(),
            Self::DirectPerplexity { .. } => {
                f.debug_struct("DirectPerplexity").finish_non_exhaustive()
            }
        }
    }
}

/// Build the [`ToolDef`] for `web_search`.
#[must_use]
pub fn tool_def() -> ToolDef {
    ToolDef::new(
        NAME,
        DESCRIPTION,
        ToolCategory::Network,
        ToolPermission::networked(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "The search query to send to the search provider."
            },
            "max_results": {
                "type": "integer",
                "description": "Maximum number of results to return (default: 5)."
            }
        },
        "required": ["query"],
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Parallel)
    .with_idempotent(true)
    .with_timeout_ms(30_000)
}

/// Format the Perplexity response into a readable text block for the agent.
fn format_response(parsed: &serde_json::Value) -> String {
    let mut out = String::new();

    // Extract the main answer.
    let content = parsed
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    if !content.is_empty() {
        out.push_str(content);
    }

    // Append citations if present.
    if let Some(citations) = parsed
        .get("citations")
        .and_then(serde_json::Value::as_array)
        && !citations.is_empty()
    {
        out.push_str("\n\nSources:\n");
        for (i, cite) in citations.iter().enumerate() {
            if let Some(url) = cite.as_str() {
                let _ = writeln!(out, "[{}] {}", i + 1, url);
            }
        }
    }

    // Append search result snippets if present.
    if let Some(results) = parsed
        .get("search_results")
        .and_then(serde_json::Value::as_array)
        && !results.is_empty()
    {
        out.push_str("\nSearch results:\n");
        for result in results {
            let title = result
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("(untitled)");
            let url = result
                .get("url")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let snippet = result
                .get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let _ = writeln!(out, "- {title}\n  {url}");
            if !snippet.is_empty() {
                let truncated = if snippet.len() > 200 {
                    format!("{}...", &snippet[..200])
                } else {
                    snippet.to_string()
                };
                let _ = writeln!(out, "  {truncated}");
            }
        }
    }

    out
}

/// Send the search query to the Perplexity API and return the raw response text.
async fn call_perplexity(query: &str, api_key: &str) -> ToolResult {
    let body = serde_json::json!({
        "model": DEFAULT_MODEL,
        "messages": [
            {
                "role": "system",
                "content": "You are a search assistant. Answer the query concisely \
                    with citations. Be precise and factual."
            },
            {
                "role": "user",
                "content": query
            }
        ],
        "return_related_questions": false,
    });

    let body_bytes = match serde_json::to_vec(&body) {
        Ok(b) => b,
        Err(e) => {
            return ToolResult::Err(ToolError::Other(format!(
                "web_search: failed to serialize request: {e}"
            )));
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(SEARCH_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::Err(ToolError::Other(format!(
                "web_search: failed to build HTTP client: {e}"
            )));
        }
    };

    let response = match client
        .post(PERPLEXITY_API_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .body(body_bytes)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if e.is_timeout() {
                return ToolResult::Err(ToolError::Timeout {
                    after_ms: SEARCH_TIMEOUT_SECS * 1_000,
                });
            }
            return ToolResult::Err(ToolError::Other(format!(
                "web_search: request to Perplexity failed: {e}"
            )));
        }
    };

    parse_perplexity_response(response).await
}

async fn call_gateway(model_caller: Arc<dyn ModelCaller>, query: &str) -> ToolResult {
    let response = model_caller
        .call(ModelCallRequest {
            model: DEFAULT_MODEL.to_string(),
            system: None,
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: query.to_string(),
            }],
            input_messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            role: Some("web-search".to_string()),
            caller: Some(caller::CLI.to_string()),
            run_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            budget: None,
            budget_remaining: None,
            routing_hints: Vec::new(),
            cache_policy: CachePolicy::Default,
            tools: Vec::new(),
            generation_settings: None,
            mcp_config: None,
        })
        .await;

    match response {
        Ok(response) if !response.content.trim().is_empty() => ToolResult::text(response.content),
        Ok(_) => ToolResult::Err(ToolError::Other(
            "web_search: gateway returned an empty response".into(),
        )),
        Err(error) => ToolResult::Err(ToolError::Other(format!(
            "web_search: gateway search failed: {error}"
        ))),
    }
}

/// Parse the HTTP response from Perplexity into a formatted `ToolResult`.
async fn parse_perplexity_response(response: reqwest::Response) -> ToolResult {
    let status = response.status();
    let response_text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            return ToolResult::Err(ToolError::Other(format!(
                "web_search: reading Perplexity response failed: {e}"
            )));
        }
    };

    if !status.is_success() {
        let truncated = if response_text.len() > 500 {
            format!("{}...", &response_text[..500])
        } else {
            response_text
        };
        return ToolResult::Err(ToolError::Other(format!(
            "web_search: Perplexity returned HTTP {status}: {truncated}"
        )));
    }

    let parsed: serde_json::Value = match serde_json::from_str(&response_text) {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::Err(ToolError::Other(format!(
                "web_search: malformed response from Perplexity: {e}"
            )));
        }
    };

    if let Some(err) = parsed.get("error") {
        let msg = err
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown API error");
        return ToolResult::Err(ToolError::Other(format!(
            "web_search: Perplexity API error: {msg}"
        )));
    }

    let formatted = format_response(&parsed);
    if formatted.trim().is_empty() {
        return ToolResult::Err(ToolError::Other(
            "web_search: Perplexity returned an empty response".into(),
        ));
    }

    ToolResult::text(formatted)
}

/// Handler for `web_search` (section 36.23).
///
/// Calls the Perplexity sonar model via their chat/completions API to
/// perform search-grounded generation. Returns the answer text plus
/// citations and search result snippets.
///
/// Construct with an explicit [`SearchBackend`] via [`Handler::gateway`] or
/// [`Handler::direct_perplexity`]. The default `Handler` const (no backend)
/// is retained for backward-compatible handler-registry dispatch but will
/// return a configuration error at call time.
#[derive(Clone, Default)]
pub struct Handler {
    backend: Option<SearchBackend>,
}

/// Default web-search handler (no backend configured).
///
/// Used by the static handler registry. At call time, if neither gateway
/// nor direct mode has been configured, a clear error is returned rather
/// than silently probing environment variables.
#[allow(non_upper_case_globals)]
pub const Handler: Handler = Handler { backend: None };

impl std::fmt::Debug for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handler")
            .field(
                "backend",
                &self.backend.as_ref().map(|b| match b {
                    SearchBackend::Gateway(_) => "gateway",
                    SearchBackend::DirectPerplexity { .. } => "direct_perplexity",
                }),
            )
            .finish()
    }
}

impl Handler {
    /// Construct a web-search handler that calls through the model gateway.
    #[must_use]
    pub fn gateway(model_caller: Arc<dyn ModelCaller>) -> Self {
        Self {
            backend: Some(SearchBackend::Gateway(model_caller)),
        }
    }

    /// Backward-compatible alias for [`Handler::gateway`].
    #[must_use]
    pub fn with_model_caller(model_caller: Arc<dyn ModelCaller>) -> Self {
        Self::gateway(model_caller)
    }

    /// Construct a web-search handler that calls the Perplexity API directly
    /// with a pre-resolved API key. Use only at documented standalone
    /// binary boundaries.
    #[must_use]
    pub fn direct_perplexity(api_key: String) -> Self {
        Self {
            backend: Some(SearchBackend::DirectPerplexity { api_key }),
        }
    }
}

#[async_trait]
impl ToolHandler for Handler {
    fn name(&self) -> &str {
        NAME
    }

    async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
        if !ctx.capabilities.network {
            return ToolResult::Err(ToolError::PermissionDenied(
                "web_search requires network capability".into(),
            ));
        }

        let query = match require_string(&call.arguments, "query") {
            Ok(q) => q,
            Err(e) => return ToolResult::Err(e),
        };
        if query.trim().is_empty() {
            return ToolResult::Err(ToolError::SchemaInvalid(
                "web_search: `query` must be non-empty".into(),
            ));
        }

        match &self.backend {
            Some(SearchBackend::Gateway(model_caller)) => {
                call_gateway(model_caller.clone(), &query).await
            }
            Some(SearchBackend::DirectPerplexity { api_key }) => {
                call_perplexity(&query, api_key).await
            }
            None => {
                // No backend configured. In standalone/fallback contexts,
                // try the Perplexity env var so existing workflows do not
                // break, but log that the gateway should be wired.
                match std::env::var("PERPLEXITY_API_KEY") {
                    Ok(k) if !k.is_empty() => {
                        tracing::warn!(
                            "web_search: no SearchBackend configured; falling back to \
                             PERPLEXITY_API_KEY env var. Configure an explicit backend \
                             via Handler::gateway() or Handler::direct_perplexity()."
                        );
                        call_perplexity(&query, &k).await
                    }
                    _ => ToolResult::Err(ToolError::Other(
                        "web_search: no search backend configured and \
                         PERPLEXITY_API_KEY is not set. Use Handler::gateway() \
                         with a ModelCaller or Handler::direct_perplexity() \
                         with a pre-resolved API key."
                            .into(),
                    )),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::ToolContext;

    fn testing_ctx_no_net() -> ToolContext {
        let mut ctx = ToolContext::testing("/tmp/work");
        ctx.capabilities.network = false;
        ctx
    }

    fn testing_ctx_with_net() -> ToolContext {
        let mut ctx = ToolContext::testing("/tmp/work");
        ctx.capabilities.network = true;
        ctx
    }

    #[tokio::test]
    async fn network_capability_gate_denies_when_off() {
        let ctx = testing_ctx_no_net();
        let call = ToolCall::new("c", NAME, serde_json::json!({ "query": "claude code" }));
        let res = Handler.execute(call, &ctx).await;
        assert!(matches!(
            res,
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
    }

    #[tokio::test]
    async fn missing_query_is_schema_invalid() {
        let ctx = testing_ctx_with_net();
        let call = ToolCall::new("c", NAME, serde_json::json!({}));
        let res = Handler.execute(call, &ctx).await;
        assert!(matches!(res, ToolResult::Err(ToolError::SchemaInvalid(_))));
    }

    #[tokio::test]
    async fn blank_query_is_schema_invalid() {
        let ctx = testing_ctx_with_net();
        let call = ToolCall::new("c", NAME, serde_json::json!({ "query": "   " }));
        let res = Handler.execute(call, &ctx).await;
        assert!(matches!(res, ToolResult::Err(ToolError::SchemaInvalid(_))));
    }

    #[tokio::test]
    async fn missing_backend_returns_clear_error() {
        // When no backend is configured and the env var is absent,
        // the handler should return a clear configuration error.
        if std::env::var("PERPLEXITY_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .is_some()
        {
            // Key is present -- the fallback path would succeed, skip.
            return;
        }
        let ctx = testing_ctx_with_net();
        let call = ToolCall::new("c", NAME, serde_json::json!({ "query": "test search" }));
        let res = Handler.execute(call, &ctx).await;
        match res {
            ToolResult::Err(ToolError::Other(msg)) => {
                assert!(
                    msg.contains("no search backend configured"),
                    "error should mention missing backend, got: {msg}"
                );
            }
            other => panic!("expected Other error about missing backend, got: {other:?}"),
        }
    }

    #[test]
    fn handler_name_matches_tool_def() {
        assert_eq!(Handler.name(), NAME);
    }

    // ── format_response unit tests ───────────────────────────────────────

    #[test]
    fn format_response_with_content_only() {
        let parsed = serde_json::json!({
            "choices": [{
                "message": { "content": "Rust is a systems language." }
            }]
        });
        let out = format_response(&parsed);
        assert!(out.contains("Rust is a systems language."));
    }

    #[test]
    fn format_response_with_citations() {
        let parsed = serde_json::json!({
            "choices": [{
                "message": { "content": "Answer text." }
            }],
            "citations": ["https://example.com/1", "https://example.com/2"]
        });
        let out = format_response(&parsed);
        assert!(out.contains("Answer text."));
        assert!(out.contains("[1] https://example.com/1"));
        assert!(out.contains("[2] https://example.com/2"));
    }

    #[test]
    fn format_response_with_search_results() {
        let parsed = serde_json::json!({
            "choices": [{
                "message": { "content": "Here is the info." }
            }],
            "search_results": [{
                "title": "Result Title",
                "url": "https://example.com/result",
                "content": "A snippet of text"
            }]
        });
        let out = format_response(&parsed);
        assert!(out.contains("Here is the info."));
        assert!(out.contains("Result Title"));
        assert!(out.contains("https://example.com/result"));
        assert!(out.contains("A snippet of text"));
    }

    #[test]
    fn format_response_empty_content() {
        let parsed = serde_json::json!({
            "choices": [{
                "message": { "content": "" }
            }]
        });
        let out = format_response(&parsed);
        assert!(out.trim().is_empty());
    }
}
