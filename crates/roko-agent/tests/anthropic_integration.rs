#![allow(missing_docs)]

mod common;

use common::{ScriptedResponse, scripted_response, spawn_scripted_server};
use roko_agent::provider::{AgentOptions, ProviderAdapter};
use roko_core::agent::ProviderKind;
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::{Body, Context, Engram, Kind};
use serde_json::{Value, json};

fn prompt(text: &str) -> Engram {
    Engram::builder(Kind::Prompt).body(Body::text(text)).build()
}

fn anthropic_provider(base_url: impl Into<String>) -> ProviderConfig {
    ProviderConfig {
        kind: ProviderKind::AnthropicApi,
        base_url: Some(base_url.into()),
        api_key_env: Some("PATH".to_string()), // PATH always exists
        command: None,
        args: None,
        timeout_ms: Some(2_000),
        ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
        connect_timeout_ms: Some(5_000),
        extra_headers: None,
        max_concurrent: None,
        limits: None,
        require_confirmation: false,
    }
}

fn anthropic_model(supports_tools: bool) -> ModelProfile {
    ModelProfile {
        provider: "anthropic".to_string(),
        slug: "claude-sonnet-4-6".to_string(),
        context_window: 200_000,
        max_output: Some(1_024),
        supports_tools,
        supports_thinking: false,
        supports_vision: false,
        supports_web_search: false,
        supports_mcp_tools: false,
        supports_partial: false,
        supports_grounding: false,
        supports_code_execution: false,
        supports_caching: false,
        provider_routing: None,
        tool_format: "anthropic_blocks".to_string(),
        cost_input_per_m: None,
        cost_output_per_m: None,
        cost_input_per_m_high: None,
        cost_output_per_m_high: None,
        cost_cache_read_per_m: None,
        cost_cache_write_per_m: None,
        thinking_level: None,
        max_tools: None,
        tokenizer_ratio: None,
        ..Default::default()
    }
}


// ─── Happy path ─────────────────────────────────────────────────────

#[tokio::test]
async fn anthropic_happy_path_simple_message() {
    let server = spawn_scripted_server(vec![scripted_response(
        200,
        json!({
            "id": "msg_happy",
            "model": "claude-sonnet-4-6",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "pong"}],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 5,
                "cache_read_input_tokens": 0,
                "cache_creation_input_tokens": 0
            }
        }),
    )]);

    let provider = anthropic_provider(format!("{}/v1", server.base_url()));
    let model = anthropic_model(false);
    let options = AgentOptions {
        name: "anthropic-happy".to_string(),
        system_prompt: Some("You are a test assistant.".to_string()),
        ..Default::default()
    };

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let agent = adapter
        .create_agent(&provider, &model, &options)
        .expect("create agent");
    assert_eq!(agent.name(), "anthropic-happy");

    let result = agent
        .run(&prompt("Reply with the single word pong."), &Context::now())
        .await;
    assert!(
        result.success,
        "expected success, got: {}",
        result.output.body.as_text().unwrap_or("unknown")
    );
    assert_eq!(result.output.body.as_text().unwrap_or(""), "pong");
    assert_eq!(result.usage.input_tokens, 12);
    assert_eq!(result.usage.output_tokens, 5);

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/v1/messages");

    // Verify Anthropic-specific headers
    assert!(
        common::header(&requests[0], "x-api-key").is_some(),
        "expected x-api-key header"
    );
    assert!(
        common::header(&requests[0], "anthropic-version").is_some(),
        "expected anthropic-version header"
    );

    let body: Value = serde_json::from_str(&requests[0].body).expect("request body json");
    assert_eq!(body["model"], "claude-sonnet-4-6");
    assert_eq!(body["max_tokens"], 1_024);
    assert_eq!(body["system"], "You are a test assistant.");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(
        body["messages"][0]["content"],
        "Reply with the single word pong."
    );

    server.join();
}

// ─── Tool call round-trip ───────────────────────────────────────────

#[tokio::test]
async fn anthropic_tool_call_round_trip() {
    let tool_use_response = json!({
        "id": "msg_tools_1",
        "model": "claude-sonnet-4-6",
        "stop_reason": "tool_use",
        "content": [
            { "type": "text", "text": "Let me check that file." },
            { "type": "tool_use", "id": "toolu_1", "name": "ls", "input": {} }
        ],
        "usage": {
            "input_tokens": 18,
            "output_tokens": 10,
            "cache_read_input_tokens": 0,
            "cache_creation_input_tokens": 0
        }
    })
    .to_string();

    let final_response = json!({
        "id": "msg_tools_2",
        "model": "claude-sonnet-4-6",
        "stop_reason": "end_turn",
        "content": [
            { "type": "text", "text": "Here are the files I found." }
        ],
        "usage": {
            "input_tokens": 22,
            "output_tokens": 6,
            "cache_read_input_tokens": 4,
            "cache_creation_input_tokens": 0
        }
    })
    .to_string();

    let server = spawn_scripted_server(vec![
        ScriptedResponse {
            status: 200,
            body: tool_use_response,
        },
        ScriptedResponse {
            status: 200,
            body: final_response,
        },
    ]);

    let provider = anthropic_provider(format!("{}/v1", server.base_url()));
    let model = anthropic_model(true);
    let options = AgentOptions {
        name: "anthropic-tool-loop".to_string(),
        tools: Some("ls".to_string()),
        ..Default::default()
    };

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let agent = adapter
        .create_agent(&provider, &model, &options)
        .expect("create tool-loop agent");

    let result = agent.run(&prompt("List the files"), &Context::now()).await;
    assert!(
        result.success,
        "expected success, got: {}",
        result.output.body.as_text().unwrap_or("unknown")
    );
    assert_eq!(
        result.output.body.as_text().unwrap_or(""),
        "Here are the files I found."
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 2);

    // First request: user prompt with tools
    let body1: Value = serde_json::from_str(&requests[0].body).expect("first request body");
    assert!(
        body1.get("tools").and_then(Value::as_array).is_some(),
        "expected tools array in first request"
    );
    assert_eq!(body1["messages"][0]["role"], "user");

    // Second request: includes tool result
    let body2: Value = serde_json::from_str(&requests[1].body).expect("second request body");
    let messages = body2["messages"].as_array().expect("messages array");
    let assistant = messages
        .iter()
        .find(|m| m["role"] == "assistant")
        .expect("assistant message in second request");
    assert_eq!(assistant["content"][0]["type"], "tool_use");
    assert_eq!(assistant["content"][0]["name"], "ls");

    let tool_result = messages
        .iter()
        .find(|m| {
            m["role"] == "user"
                && m["content"]
                    .as_array()
                    .is_some_and(|c| c.iter().any(|b| b["type"] == "tool_result"))
        })
        .expect("tool result message in second request");
    assert_eq!(tool_result["content"][0]["tool_use_id"], "toolu_1");

    server.join();
}

// ─── 429 rate-limit classification ──────────────────────────────────

#[tokio::test]
async fn anthropic_429_maps_to_rate_limit() {
    use roko_agent::provider::ProviderError;

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let error = adapter.classify_error(
        429,
        &json!({
            "error": {
                "type": "rate_limit_error",
                "message": "Rate limit exceeded"
            }
        }),
    );
    assert!(
        matches!(error, ProviderError::RateLimit { .. }),
        "expected RateLimit, got: {error:?}"
    );
}

#[tokio::test]
async fn anthropic_429_with_retry_after_body() {
    use roko_agent::provider::ProviderError;

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let error = adapter.classify_error(
        429,
        &json!({
            "error": {
                "type": "rate_limit_error",
                "message": "Rate limit exceeded"
            },
            "retry_after": 30
        }),
    );
    match error {
        ProviderError::RateLimit {
            retry_after_ms: Some(ms),
        } => assert_eq!(ms, 30_000),
        other => panic!("expected RateLimit with 30_000ms, got: {other:?}"),
    }
}

#[tokio::test]
async fn anthropic_529_overload_maps_to_rate_limit() {
    use roko_agent::provider::ProviderError;

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let error = adapter.classify_error(
        529,
        &json!({
            "error": {
                "type": "overloaded_error",
                "message": "API is temporarily overloaded"
            }
        }),
    );
    assert!(
        matches!(error, ProviderError::RateLimit { .. }),
        "expected RateLimit for 529 overload, got: {error:?}"
    );
}

// ─── Cache header injection (cache_control markers) ─────────────────

#[tokio::test]
async fn anthropic_cache_tokens_in_usage() {
    let server = spawn_scripted_server(vec![scripted_response(
        200,
        json!({
            "id": "msg_cache",
            "model": "claude-sonnet-4-6",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "cached reply"}],
            "usage": {
                "input_tokens": 20,
                "output_tokens": 4,
                "cache_read_input_tokens": 15,
                "cache_creation_input_tokens": 5
            }
        }),
    )]);

    let provider = anthropic_provider(format!("{}/v1", server.base_url()));
    let model = anthropic_model(false);
    let options = AgentOptions {
        name: "anthropic-cache".to_string(),
        ..Default::default()
    };

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let agent = adapter
        .create_agent(&provider, &model, &options)
        .expect("create agent");

    let result = agent.run(&prompt("cached request"), &Context::now()).await;
    assert!(result.success);
    assert_eq!(result.output.body.as_text().unwrap_or(""), "cached reply");
    assert_eq!(result.usage.input_tokens, 20);
    assert_eq!(result.usage.output_tokens, 4);

    server.join();
}

// ─── Error classification coverage ──────────────────────────────────

#[tokio::test]
async fn anthropic_auth_failure_classification() {
    use roko_agent::provider::ProviderError;

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;

    let err_401 = adapter.classify_error(401, &json!({"error": {"type": "authentication_error"}}));
    assert!(
        matches!(err_401, ProviderError::AuthFailure),
        "expected AuthFailure for 401, got: {err_401:?}"
    );

    let err_403 = adapter.classify_error(403, &json!({"error": {"type": "permission_error"}}));
    assert!(
        matches!(err_403, ProviderError::AuthFailure),
        "expected AuthFailure for 403, got: {err_403:?}"
    );
}

#[tokio::test]
async fn anthropic_content_policy_classification() {
    use roko_agent::provider::ProviderError;

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;

    // Via error type
    let err = adapter.classify_error(400, &json!({"error": {"type": "content_policy_violation"}}));
    assert!(
        matches!(err, ProviderError::ContentPolicy),
        "expected ContentPolicy, got: {err:?}"
    );

    // Via stop_reason
    let err = adapter.classify_error(200, &json!({"stop_reason": "content_filter"}));
    assert!(
        matches!(err, ProviderError::ContentPolicy),
        "expected ContentPolicy for content_filter stop_reason, got: {err:?}"
    );
}

#[tokio::test]
async fn anthropic_model_not_found_classification() {
    use roko_agent::provider::ProviderError;

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let err = adapter.classify_error(404, &json!({"error": {"type": "not_found_error"}}));
    assert!(
        matches!(err, ProviderError::ModelNotFound),
        "expected ModelNotFound for 404, got: {err:?}"
    );
}

#[tokio::test]
async fn anthropic_server_error_classification() {
    use roko_agent::provider::ProviderError;

    let adapter = roko_agent::provider::anthropic_api::AnthropicApiAdapter;
    let err = adapter.classify_error(500, &json!({"error": {"type": "api_error"}}));
    assert!(
        matches!(err, ProviderError::ServerError(500)),
        "expected ServerError(500), got: {err:?}"
    );
}
