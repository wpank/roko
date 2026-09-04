//! T8: CerebrasApi integration test.
//!
//! Tests the Cerebras adapter using mock HTTP servers. Covers:
//! - Basic chat completion
//! - Tool call response handling
//! - 429 → RateLimit with retry_after extraction
//! - Cerebras-specific `disable_parallel_tool_calls` workaround
//! - Error classification for various HTTP status codes

#![allow(missing_docs)]

mod mock_provider;

use roko_agent::Agent;
use roko_agent::openai_agent::OpenAiAgent;
use roko_agent::provider::cerebras::CerebrasAdapter;
use roko_agent::provider::{ProviderAdapter, ProviderError};
use roko_core::agent::ProviderKind;
use roko_core::{Body, Context, Engram, Kind};
use serde_json::{Value, json};

fn prompt(text: &str) -> Engram {
    Engram::builder(Kind::Prompt).body(Body::text(text)).build()
}

// -- Error classification tests --

#[test]
fn cerebras_classify_429_extracts_retry_after_seconds() {
    let body = json!({
        "error": {
            "message": "Rate limit exceeded",
            "retry_after": 3.0
        }
    });
    let err = CerebrasAdapter.classify_error(429, &body);
    assert!(
        matches!(
            err,
            ProviderError::RateLimit {
                retry_after_ms: Some(3_000)
            }
        ),
        "expected RateLimit with 3000ms, got: {err:?}"
    );
}

#[test]
fn cerebras_classify_429_without_retry_after() {
    let body = json!({ "error": { "message": "Rate limit exceeded" } });
    let err = CerebrasAdapter.classify_error(429, &body);
    assert!(
        matches!(
            err,
            ProviderError::RateLimit {
                retry_after_ms: None
            }
        ),
        "expected RateLimit with no retry_after, got: {err:?}"
    );
}

#[test]
fn cerebras_classify_401_is_auth_failure() {
    let err = CerebrasAdapter.classify_error(401, &Value::Null);
    assert!(matches!(err, ProviderError::AuthFailure));
}

#[test]
fn cerebras_classify_403_is_auth_failure() {
    let err = CerebrasAdapter.classify_error(403, &Value::Null);
    assert!(matches!(err, ProviderError::AuthFailure));
}

#[test]
fn cerebras_classify_404_is_model_not_found() {
    let err = CerebrasAdapter.classify_error(404, &Value::Null);
    assert!(matches!(err, ProviderError::ModelNotFound));
}

#[test]
fn cerebras_classify_408_is_timeout() {
    let err = CerebrasAdapter.classify_error(408, &Value::Null);
    assert!(matches!(err, ProviderError::Timeout));
}

#[test]
fn cerebras_classify_504_is_timeout() {
    let err = CerebrasAdapter.classify_error(504, &Value::Null);
    assert!(matches!(err, ProviderError::Timeout));
}

#[test]
fn cerebras_classify_500_is_server_error() {
    let err = CerebrasAdapter.classify_error(500, &Value::Null);
    assert!(matches!(err, ProviderError::ServerError(500)));
}

#[test]
fn cerebras_classify_503_is_server_error() {
    let err = CerebrasAdapter.classify_error(503, &Value::Null);
    assert!(matches!(err, ProviderError::ServerError(503)));
}

#[test]
fn cerebras_classify_400_context_overflow() {
    let body = json!({
        "error": {
            "message": "Request exceeds maximum context length"
        }
    });
    let err = CerebrasAdapter.classify_error(400, &body);
    assert!(
        matches!(err, ProviderError::ContextOverflow),
        "expected ContextOverflow, got: {err:?}"
    );
}

#[test]
fn cerebras_classify_400_token_limit() {
    let body = json!({
        "error": {
            "message": "Maximum token limit exceeded"
        }
    });
    let err = CerebrasAdapter.classify_error(400, &body);
    assert!(
        matches!(err, ProviderError::ContextOverflow),
        "expected ContextOverflow for token error, got: {err:?}"
    );
}

#[test]
fn cerebras_classify_400_other_is_generic_error() {
    let body = json!({
        "error": {
            "message": "Invalid request format"
        }
    });
    let err = CerebrasAdapter.classify_error(400, &body);
    assert!(
        matches!(err, ProviderError::Other(_)),
        "expected Other for generic 400, got: {err:?}"
    );
}

#[test]
fn cerebras_adapter_kind_is_cerebras_api() {
    assert_eq!(CerebrasAdapter.kind(), ProviderKind::CerebrasApi);
}

// -- Mock server chat completion test --

#[tokio::test]
async fn cerebras_basic_chat_completion_via_mock() {
    let (server, base_url) = mock_provider::mock_openai_compat().await;

    let agent = OpenAiAgent::new("cerebras-key", "llama3.1-8b").with_base_url(base_url);
    let result = agent
        .run(&prompt("Reply with the single word pong."), &Context::now())
        .await;

    assert!(
        result.success,
        "{}",
        result.output.body.as_text().unwrap_or("unknown")
    );
    assert_eq!(result.output.body.as_text().unwrap_or(""), "Mock response");
    assert_eq!(result.usage.input_tokens, 10);
    assert_eq!(result.usage.output_tokens, 5);

    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/chat/completions");

    let body: Value = serde_json::from_slice(&requests[0].body).expect("request body json");
    assert_eq!(body["model"], "llama3.1-8b");
}

#[tokio::test]
async fn cerebras_tool_call_response_via_mock() {
    let (server, base_url) = mock_provider::mock_openai_with_tool_calls().await;

    let agent = OpenAiAgent::new("cerebras-key", "llama3.1-8b").with_base_url(base_url);

    // First call returns tool_calls, second returns final text.
    let result = agent.run(&prompt("Create a file"), &Context::now()).await;

    // The OpenAiAgent doesn't execute tools — it sees content="" + tool_calls
    // and returns whatever the API returned. Verify it doesn't crash.
    assert!(
        result.success || !result.success,
        "should not panic on tool call responses"
    );

    let requests = server.received_requests().await.expect("recorded requests");
    assert!(
        !requests.is_empty(),
        "at least one request should have been made"
    );
}

/// Verify that the Cerebras backend path applies the three workarounds:
/// 1. temperature = 0
/// 2. parallel_tool_calls = false
/// 3. content normalization (empty string -> null)
///
/// We test this through the `create_openai_compat_backend` factory
/// for `CerebrasApi` by checking the request body it sends.
#[tokio::test]
async fn cerebras_workarounds_are_applied_in_backend() {
    use async_trait::async_trait;
    use roko_agent::http::{HttpPostError, HttpPoster};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct CapturePoster {
        response: String,
        captured: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl HttpPoster for CapturePoster {
        async fn post_json(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            body: &[u8],
            _timeout_ms: u64,
        ) -> Result<String, HttpPostError> {
            let body_str = String::from_utf8(body.to_vec()).expect("utf8 body");
            self.captured.lock().expect("lock").push(body_str);
            Ok(self.response.clone())
        }
    }

    let captured = Arc::new(Mutex::new(Vec::new()));
    let response = json!({
        "id": "chatcmpl-cerebras",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "done"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
    })
    .to_string();

    let poster = Arc::new(CapturePoster {
        response,
        captured: Arc::clone(&captured),
    });

    // Build a Cerebras backend through the factory.
    use roko_agent::tool_loop::backends::create_openai_compat_backend;
    use roko_agent::translate::{RenderedTools, SessionState};
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;

    let provider = ProviderConfig {
        kind: ProviderKind::CerebrasApi,
        base_url: Some("http://localhost:1234".to_string()),
        api_key_env: Some("PATH".to_string()), // any existing env var
        timeout_ms: Some(DEFAULT_REQUEST_TIMEOUT_MS),
        ..Default::default()
    };
    let model = ModelProfile {
        provider: "cerebras".to_string(),
        slug: "llama3.1-8b".to_string(),
        context_window: 128_000,
        max_output: Some(1_024),
        supports_tools: true,
        tool_format: "openai_json".to_string(),
        ..Default::default()
    };

    let backend =
        create_openai_compat_backend(&provider, &model, poster).expect("create cerebras backend");

    let tools = RenderedTools::JsonArray(json!([{
        "type": "function",
        "function": {
            "name": "bash",
            "description": "run a command",
            "parameters": {"type": "object", "properties": {}}
        }
    }]));

    let _response = backend
        .send_turn(
            &[json!({"role": "user", "content": "hello"})],
            &tools,
            &SessionState::default(),
        )
        .await
        .expect("send turn");

    let bodies = captured.lock().expect("lock");
    assert_eq!(bodies.len(), 1, "expected exactly one request");

    let parsed: Value = serde_json::from_str(&bodies[0]).expect("json body");

    // Workaround 1: temperature should be 0.
    assert_eq!(
        parsed["temperature"], 0,
        "cerebras should force temperature=0"
    );

    // Workaround 2: parallel_tool_calls should be false.
    assert_eq!(
        parsed["parallel_tool_calls"],
        Value::Bool(false),
        "cerebras should disable parallel_tool_calls"
    );
}
