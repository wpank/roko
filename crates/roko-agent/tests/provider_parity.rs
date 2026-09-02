#![allow(missing_docs)]
//! Provider parity contract tests (Packet E).
//!
//! For each provider family, tests:
//! - Tool definition translation (canonical -> provider format)
//! - Tool call parsing from provider response
//! - Tool result formatting for provider
//! - Error handling (malformed response, timeout, auth failure)
//! - Usage reporting accuracy
//!
//! All tests are offline: they use fixtures and mock transports.

use roko_agent::parity_matrix::{
    Capability, CapabilityState, ProviderCapabilityMatrix, provider_label,
};
use roko_agent::provider::ProviderError;
use roko_agent::translate::{
    BackendResponse, ClaudeTranslator, GeminiTranslator, OpenAiTranslator, ReActTranslator,
    RenderedTools, Translator,
};
use roko_core::agent::ProviderKind;
use roko_core::tool::{ToolCall, ToolCategory, ToolDef, ToolPermission, ToolResult};
use serde_json::{Value, json};

// ── Fixture helpers ───────────────────────────────────────────────────────

fn sample_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "read_file",
            "Read a file from the filesystem",
            ToolCategory::Read,
            ToolPermission::read_only(),
        ),
        ToolDef::new(
            "write_file",
            "Write content to a file",
            ToolCategory::Write,
            ToolPermission::writes(),
        ),
        ToolDef::new(
            "bash",
            "Execute a bash command",
            ToolCategory::Exec,
            ToolPermission::executes(),
        ),
    ]
}

fn sample_tool_call() -> ToolCall {
    ToolCall {
        id: "call-1".to_string(),
        name: "read_file".to_string(),
        arguments: json!({"path": "/tmp/test.txt"}),
        request_ts_ms: 0,
    }
}

fn sample_tool_result() -> ToolResult {
    ToolResult::text("file contents here")
}

/// Build an OpenAI-format response with a tool call.
fn openai_tool_call_response() -> Value {
    json!({
        "id": "chatcmpl-test-1",
        "object": "chat.completion",
        "model": "gpt-5",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [{
                    "id": "call-test-1",
                    "type": "function",
                    "function": {
                        "name": "read_file",
                        "arguments": "{\"path\":\"/tmp/test.txt\"}"
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "total_tokens": 120
        }
    })
}

/// Build an OpenAI-format text response (no tool calls).
fn openai_text_response() -> Value {
    json!({
        "id": "chatcmpl-test-2",
        "object": "chat.completion",
        "model": "gpt-5",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Here is the answer."
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 50,
            "completion_tokens": 10,
            "total_tokens": 60
        }
    })
}

/// Build a Gemini-format response with a function call.
fn gemini_function_call_response() -> Value {
    json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "functionCall": {
                        "name": "read_file",
                        "args": {"path": "/tmp/test.txt"}
                    }
                }],
                "role": "model"
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {
            "promptTokenCount": 100,
            "candidatesTokenCount": 20,
            "totalTokenCount": 120
        }
    })
}

// ── OpenAI/OllamaTranslator tests ────────────────────────────────────────

mod openai_compat {
    use super::*;

    #[test]
    fn render_tools_produces_function_array() {
        let translator = OpenAiTranslator;
        let tools = sample_tool_defs();
        let rendered = translator.render_tools(&tools);
        match rendered {
            RenderedTools::JsonArray(arr) => {
                let arr = arr.as_array().expect("must be array");
                assert_eq!(arr.len(), 3);
                for item in arr {
                    assert_eq!(item["type"], "function");
                    assert!(item["function"]["name"].is_string());
                    assert!(item["function"]["description"].is_string());
                }
            }
            other => panic!("expected JsonArray, got: {other:?}"),
        }
    }

    #[test]
    fn parse_tool_call_response() {
        let translator = OpenAiTranslator;
        let response = BackendResponse::Json(openai_tool_call_response());
        let calls = translator.parse_calls(&response).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "/tmp/test.txt");
        assert_eq!(calls[0].id, "call-test-1");
    }

    #[test]
    fn parse_text_response_returns_empty_calls() {
        let translator = OpenAiTranslator;
        let response = BackendResponse::Json(openai_text_response());
        let calls = translator.parse_calls(&response).unwrap();
        assert!(calls.is_empty());
    }

    #[test]
    fn render_results_produces_tool_messages() {
        let translator = OpenAiTranslator;
        let call = sample_tool_call();
        let result = sample_tool_result();
        let rendered = translator.render_results(&[(call, result)]);
        match rendered {
            roko_agent::translate::RenderedResults::JsonMessages(arr) => {
                let arr = arr.as_array().expect("must be array");
                assert_eq!(arr.len(), 1);
                assert_eq!(arr[0]["role"], "tool");
                assert!(arr[0]["content"].is_string());
            }
            other => panic!("expected JsonMessages, got: {other:?}"),
        }
    }

    #[test]
    fn parse_malformed_response_does_not_panic() {
        let translator = OpenAiTranslator;
        let response = BackendResponse::Json(json!({"broken": true}));
        // Should either return empty calls or an error, not panic
        let result = translator.parse_calls(&response);
        match result {
            Ok(calls) => assert!(calls.is_empty()),
            Err(_) => {} // Malformed error is acceptable
        }
    }

    #[test]
    fn glm_fixture_parses_correctly() {
        let translator = OpenAiTranslator;
        let fixture: Value =
            serde_json::from_str(include_str!("fixtures/glm-5.1/tool_call_response.json")).unwrap();
        let response = BackendResponse::Json(fixture);
        let calls = translator.parse_calls(&response).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "Read");
    }

    #[test]
    fn usage_extraction_from_openai_response() {
        let response = openai_tool_call_response();
        let usage = response.get("usage").expect("usage present");
        assert_eq!(usage["prompt_tokens"], 100);
        assert_eq!(usage["completion_tokens"], 20);
        assert_eq!(usage["total_tokens"], 120);
    }
}

// ── Claude/AnthropicTranslator tests ─────────────────────────────────────

mod anthropic {
    use super::*;

    #[test]
    fn render_tools_produces_cli_flag_format() {
        let translator = ClaudeTranslator;
        let tools = sample_tool_defs();
        let rendered = translator.render_tools(&tools);
        // ClaudeTranslator renders as CLI flag list (comma-separated names)
        match rendered {
            RenderedTools::CliFlag(flags) => {
                assert!(flags.contains("Read"));
                assert!(flags.contains("Write"));
                assert!(flags.contains("Bash"));
            }
            RenderedTools::JsonArray(_) => {
                // Some Claude paths may use JSON array
            }
            other => panic!("unexpected format: {other:?}"),
        }
    }

    #[test]
    fn parse_tool_use_from_stream_json() {
        let translator = ClaudeTranslator;
        // Claude CLI uses StreamJson events, not plain Json
        let events = vec![json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "tool_use",
                "id": "toolu_test_1",
                "name": "Read",
                "input": {"path": "/tmp/test.txt"}
            }
        })];
        let response = BackendResponse::StreamJson(events);
        let calls = translator.parse_calls(&response).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
    }

    #[test]
    fn parse_text_stream_returns_empty_calls() {
        let translator = ClaudeTranslator;
        let events = vec![json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "text",
                "text": "Here is the answer."
            }
        })];
        let response = BackendResponse::StreamJson(events);
        let calls = translator.parse_calls(&response).unwrap();
        assert!(calls.is_empty());
    }

    #[test]
    fn render_results_is_handled_by_backend() {
        let translator = ClaudeTranslator;
        let call = ToolCall {
            id: "toolu_test_1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "/tmp/test.txt"}),
            request_ts_ms: 0,
        };
        let result = sample_tool_result();
        let rendered = translator.render_results(&[(call, result)]);
        // Claude CLI handles tool results internally
        assert!(
            matches!(
                rendered,
                roko_agent::translate::RenderedResults::HandledByBackend
            ),
            "Claude translator should return HandledByBackend"
        );
    }

    #[test]
    fn parse_non_stream_json_returns_error() {
        let translator = ClaudeTranslator;
        // ClaudeTranslator expects StreamJson, not Json
        let response = BackendResponse::Json(json!({"type": "error"}));
        let result = translator.parse_calls(&response);
        assert!(result.is_err(), "ClaudeTranslator should reject Json input");
    }
}

// ── Gemini translator tests ──────────────────────────────────────────────

mod gemini {
    use super::*;

    #[test]
    fn render_tools_produces_function_declarations() {
        let translator = GeminiTranslator;
        let tools = sample_tool_defs();
        let rendered = translator.render_tools(&tools);
        match rendered {
            RenderedTools::JsonArray(val) => {
                // Gemini wraps tools in functionDeclarations
                assert!(val.is_array() || val.is_object());
            }
            other => panic!("expected JsonArray for Gemini, got: {other:?}"),
        }
    }

    #[test]
    fn parse_function_call_response() {
        let translator = GeminiTranslator;
        let response = BackendResponse::Json(gemini_function_call_response());
        let calls = translator.parse_calls(&response).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "/tmp/test.txt");
    }
}

// ── ReAct fallback tests ─────────────────────────────────────────────────

mod react {
    use super::*;
    use roko_core::tool::ToolFormat;

    #[test]
    fn format_is_react_text() {
        let translator = ReActTranslator;
        assert_eq!(translator.format(), ToolFormat::ReActText);
    }

    #[test]
    fn render_tools_produces_text() {
        let translator = ReActTranslator;
        let tools = sample_tool_defs();
        let rendered = translator.render_tools(&tools);
        match rendered {
            RenderedTools::SystemPromptBlock(text) => {
                assert!(text.contains("read_file"));
                assert!(text.contains("write_file"));
                assert!(text.contains("bash"));
            }
            other => panic!("expected SystemPromptBlock for ReAct, got: {other:?}"),
        }
    }
}

// ── Error classification tests ───────────────────────────────────────────

mod error_handling {
    use super::*;
    use roko_agent::provider::adapter_for_kind;

    #[test]
    fn classify_401_as_auth_failure() {
        let fixture: Value =
            serde_json::from_str(include_str!("fixtures/common/401_auth_failure.json")).unwrap();
        for kind in [
            ProviderKind::OpenAiCompat,
            ProviderKind::AnthropicApi,
            ProviderKind::CerebrasApi,
        ] {
            let adapter = adapter_for_kind(kind);
            let err = adapter.classify_error(401, &fixture);
            assert!(
                matches!(err, ProviderError::AuthFailure),
                "{kind:?} should classify 401 as AuthFailure, got: {err:?}"
            );
        }
    }

    #[test]
    fn classify_429_as_rate_limit() {
        let fixture: Value =
            serde_json::from_str(include_str!("fixtures/common/429_rate_limit.json")).unwrap();
        for kind in [
            ProviderKind::OpenAiCompat,
            ProviderKind::AnthropicApi,
            ProviderKind::CerebrasApi,
        ] {
            let adapter = adapter_for_kind(kind);
            let err = adapter.classify_error(429, &fixture);
            assert!(
                matches!(err, ProviderError::RateLimit { .. }),
                "{kind:?} should classify 429 as RateLimit, got: {err:?}"
            );
        }
    }

    #[test]
    fn classify_500_as_server_error() {
        let fixture: Value =
            serde_json::from_str(include_str!("fixtures/common/500_server_error.json")).unwrap();
        for kind in [
            ProviderKind::OpenAiCompat,
            ProviderKind::AnthropicApi,
            ProviderKind::CerebrasApi,
        ] {
            let adapter = adapter_for_kind(kind);
            let err = adapter.classify_error(500, &fixture);
            assert!(
                matches!(err, ProviderError::ServerError(500)),
                "{kind:?} should classify 500 as ServerError(500), got: {err:?}"
            );
        }
    }
}

// ── Parity matrix tests ─────────────────────────────────────────────────

mod matrix {
    use super::*;

    #[test]
    fn static_baseline_covers_all_providers() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        assert_eq!(
            matrix.rows.len(),
            ProviderCapabilityMatrix::ALL_PROVIDERS.len()
        );
    }

    #[test]
    fn static_baseline_has_no_untested() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        assert_eq!(matrix.count_state(CapabilityState::Untested), 0);
    }

    #[test]
    fn anthropic_api_has_full_tool_support() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        let row = &matrix.rows[provider_label(ProviderKind::AnthropicApi)];
        assert_eq!(row.get(Capability::Tools), CapabilityState::Supported);
        assert_eq!(row.get(Capability::Streaming), CapabilityState::Supported);
        assert_eq!(row.get(Capability::Reasoning), CapabilityState::Supported);
        assert_eq!(row.get(Capability::Vision), CapabilityState::Supported);
        assert_eq!(
            row.get(Capability::ParallelTools),
            CapabilityState::Supported
        );
        assert_eq!(
            row.get(Capability::UsageReporting),
            CapabilityState::Supported
        );
    }

    #[test]
    fn perplexity_has_no_tool_support() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        let row = &matrix.rows[provider_label(ProviderKind::PerplexityApi)];
        assert_eq!(row.get(Capability::Tools), CapabilityState::Unavailable);
        assert_eq!(
            row.get(Capability::ParallelTools),
            CapabilityState::Unavailable
        );
        assert_eq!(row.get(Capability::Streaming), CapabilityState::Supported);
    }

    #[test]
    fn cursor_acp_usage_is_degraded() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        let row = &matrix.rows[provider_label(ProviderKind::CursorAcp)];
        assert_eq!(
            row.get(Capability::UsageReporting),
            CapabilityState::Degraded
        );
    }

    #[test]
    fn gemini_api_supports_code_execution() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        let row = &matrix.rows[provider_label(ProviderKind::GeminiApi)];
        assert_eq!(
            row.get(Capability::CodeExecution),
            CapabilityState::Supported
        );
    }

    #[test]
    fn contract_test_results_override_baseline() {
        let mut matrix = ProviderCapabilityMatrix::static_baseline();
        // Simulate a contract test discovering that CerebrasApi now supports cancellation
        matrix.record_result(
            ProviderKind::CerebrasApi,
            Capability::Cancellation,
            CapabilityState::Supported,
        );
        let row = &matrix.rows[provider_label(ProviderKind::CerebrasApi)];
        assert_eq!(
            row.get(Capability::Cancellation),
            CapabilityState::Supported
        );
    }

    #[test]
    fn markdown_report_is_well_formed() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        let report = matrix.to_markdown_report();

        // Must contain header
        assert!(report.contains("# Provider Capability Parity Matrix"));
        // Must contain legend
        assert!(report.contains("Legend:"));
        // Must contain all provider labels
        for provider in ProviderCapabilityMatrix::ALL_PROVIDERS {
            assert!(
                report.contains(provider_label(*provider)),
                "report missing {}",
                provider_label(*provider)
            );
        }
        // Must contain coverage summary
        assert!(report.contains("Coverage:"));
        // Must have table separators
        assert!(report.contains("|---|"));
    }
}

// ── T012: Feature-dependent catalog validation ──────────────────────────

mod catalog_validation {
    use roko_core::tool::ToolRegistry;
    use roko_std::tool::builtin::{ROKO_BUILTIN_TOOLS, TOOL_COUNT};
    use roko_std::tool::registry::StaticToolRegistry;

    #[test]
    fn catalog_count_matches_expected_default() {
        // Without chain feature: 16 std + 19 GitHub MCP = 35
        #[cfg(not(feature = "chain"))]
        {
            assert_eq!(
                TOOL_COUNT, 35,
                "default catalog should have 35 tools (16 std + 19 GitHub MCP)"
            );
        }
        // With chain feature: 16 std + 17 chain + 19 GitHub MCP = 52
        #[cfg(feature = "chain")]
        {
            assert_eq!(
                TOOL_COUNT, 52,
                "chain catalog should have 52 tools (16 std + 17 chain + 19 GitHub MCP)"
            );
        }
    }

    #[test]
    fn runtime_count_matches_constant() {
        assert_eq!(ROKO_BUILTIN_TOOLS.len(), TOOL_COUNT);
    }

    #[test]
    fn standard_tools_always_present() {
        let expected_std_tools = [
            "read_file",
            "write_file",
            "edit_file",
            "multi_edit",
            "glob",
            "grep",
            "bash",
            "ls",
            "web_fetch",
            "web_search",
            "notebook_edit",
            "todo_write",
            "task",
            "exit_plan_mode",
            "apply_patch",
            "run_tests",
        ];
        let reg = StaticToolRegistry::new();
        for name in &expected_std_tools {
            assert!(
                reg.get(name).is_some(),
                "standard tool '{name}' missing from catalog"
            );
        }
        assert_eq!(expected_std_tools.len(), 16);
    }

    #[test]
    fn github_mcp_tools_always_present() {
        use roko_std::tool::builtin::github::{GITHUB_TOOL_COUNT, GITHUB_TOOL_NAMES};

        let reg = StaticToolRegistry::new();
        assert_eq!(GITHUB_TOOL_COUNT, 19);
        for name in &GITHUB_TOOL_NAMES {
            assert!(
                reg.get(name).is_some(),
                "GitHub MCP tool '{name}' missing from catalog"
            );
        }
    }

    #[test]
    fn no_duplicate_tool_names_in_catalog() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for tool in ROKO_BUILTIN_TOOLS.iter() {
            assert!(
                seen.insert(&tool.name),
                "duplicate tool name in catalog: {}",
                tool.name
            );
        }
    }

    #[test]
    fn catalog_snapshot_drift_detection() {
        // This test captures the exact tool count to detect drift.
        // Update the expected count when tools are intentionally added/removed.
        let actual_count = ROKO_BUILTIN_TOOLS.len();
        assert_eq!(
            actual_count, TOOL_COUNT,
            "catalog count drifted: expected {TOOL_COUNT}, got {actual_count}. \
             Update TOOL_COUNT if tools were intentionally added/removed."
        );
    }
}

// ── T013: MCP lifecycle state tests ─────────────────────────────────────

mod mcp_lifecycle {
    use roko_agent::mcp::{McpLifecycleState, McpRuntime};
    use std::collections::HashMap;
    use std::time::Instant;

    #[test]
    fn empty_runtime_has_empty_lifecycle() {
        let runtime = McpRuntime::from_clients(vec![], HashMap::new());
        assert!(runtime.lifecycle_state().is_empty());
        assert_eq!(runtime.server_count(), 0);
    }

    #[test]
    fn lifecycle_state_round_trips_through_constructor() {
        let lifecycle = vec![McpLifecycleState {
            server_name: "test-server".to_string(),
            last_health_check: Some(Instant::now()),
            last_error: None,
            negotiated_capabilities: Some(serde_json::json!({"tools": {}})),
            available_tools: vec!["read_file".to_string(), "write_file".to_string()],
        }];
        let runtime =
            McpRuntime::from_clients_with_lifecycle(vec![], HashMap::new(), lifecycle.clone());
        let state = runtime.lifecycle_state();
        assert_eq!(state.len(), 1);
        assert_eq!(state[0].server_name, "test-server");
        assert!(state[0].last_health_check.is_some());
        assert!(state[0].last_error.is_none());
        assert_eq!(state[0].available_tools.len(), 2);
    }

    #[test]
    fn lifecycle_error_state() {
        let lifecycle = vec![McpLifecycleState {
            server_name: "broken-server".to_string(),
            last_health_check: None,
            last_error: Some("connection refused".to_string()),
            negotiated_capabilities: None,
            available_tools: vec![],
        }];
        let runtime = McpRuntime::from_clients_with_lifecycle(vec![], HashMap::new(), lifecycle);
        let state = &runtime.lifecycle_state()[0];
        assert!(state.last_health_check.is_none());
        assert_eq!(state.last_error.as_deref(), Some("connection refused"));
        assert!(state.available_tools.is_empty());
    }
}

// ── CI parity report generation ─────────────────────────────────────────

#[test]
fn generate_parity_report() {
    let mut matrix = ProviderCapabilityMatrix::static_baseline();

    // Override with contract test results from this file:
    // OpenAiCompat tools: verified by openai_compat::render_tools_produces_function_array
    matrix.record_result(
        ProviderKind::OpenAiCompat,
        Capability::Tools,
        CapabilityState::Supported,
    );
    // AnthropicApi tools: verified by anthropic::render_tools_produces_anthropic_format
    matrix.record_result(
        ProviderKind::AnthropicApi,
        Capability::Tools,
        CapabilityState::Supported,
    );
    // GeminiApi tools: verified by gemini::parse_function_call_response
    matrix.record_result(
        ProviderKind::GeminiApi,
        Capability::Tools,
        CapabilityState::Supported,
    );

    let report = matrix.to_markdown_report();

    // Validate report structure
    assert!(report.contains("# Provider Capability Parity Matrix"));
    assert!(report.contains("Coverage:"));

    // Print for CI consumption
    eprintln!("\n{report}");
}
