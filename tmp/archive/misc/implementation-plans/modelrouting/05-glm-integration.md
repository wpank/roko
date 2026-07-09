# 05 — GLM-5.1 First-Class Backend Integration

> **Priority**: 🟡 P1 — First non-Anthropic model with full feature support
> **Status**: Not started
> **Depends on**: 02 (registry), 03 (adapters), 04 (translator extensions)
> **Blocks**: None (can be done in parallel with 06, 07)

## Problem Statement

GLM-5.1 (Z.AI, April 2026) is the #1 model on SWE-Bench Pro, open-weight (MIT), and has an OpenAI-compatible API — but with significant extensions: thinking mode, tool streaming, built-in web search, native MCP integration, and 4 tool types. A naive OpenAI-compat integration would miss these features.

## GLM-5.1 Capabilities Summary

| Capability | API Parameter | Standard? |
|---|---|---|
| Tool calling | `tools: [{type: "function", ...}]` | Yes (OpenAI format) |
| Thinking mode | `thinking: {type: "enabled", clear_thinking: false}` | No (GLM-specific) |
| Tool streaming | `tool_stream: true` | No (GLM-specific) |
| Web search tool | `tools: [{type: "web_search", ...}]` | No (GLM-specific) |
| RAG/retrieval | `tools: [{type: "retrieval", ...}]` | No (GLM-specific) |
| Native MCP | `tools: [{type: "mcp", ...}]` | No (GLM-specific) |
| JSON mode | `response_format: {type: "json_object"}` | Yes |
| Reasoning content | `reasoning_content` in response | No (shared with Kimi) |
| Content filters | `content_filter` array in response | No (GLM-specific) |
| Cached tokens | `prompt_tokens_details.cached_tokens` | Partial (OpenAI has similar) |

## Endpoints

| Endpoint | URL | Billing |
|---|---|---|
| Standard API | `https://api.z.ai/api/paas/v4/chat/completions` | Token-based |
| Coding Plan | `https://api.z.ai/api/coding/paas/v4/chat/completions` | Prompt-based |
| Anthropic-compat | `https://api.z.ai/api/anthropic` | Coding Plan |
| China Standard | `https://open.bigmodel.cn/api/paas/v4/chat/completions` | Token-based |

## Pricing

| Model | Input/M | Output/M | Cache Read/M |
|---|---|---|---|
| GLM-5.1 | $1.40 | $4.40 | $0.26 |
| GLM-5 | $1.00 | $3.20 | — |
| GLM-4.7 | $0.60 | $2.20 | — |

---

## Checklist

### 2D.01 — Add Z.AI provider config example

**File**: `examples/roko-glm.toml`
**What**: Complete example config for GLM-5.1 via Z.AI direct:

```toml
[providers.zai]
kind = "openai_compat"
base_url = "https://api.z.ai/api/paas/v4"
api_key_env = "ZAI_API_KEY"
timeout_ms = 180000

[models.glm-5-1]
provider = "zai"
slug = "glm-5.1"
context_window = 200000
max_output = 131072
supports_tools = true
supports_thinking = true
supports_web_search = true
supports_mcp_tools = true
tool_format = "openai_json"
cost_input_per_m = 1.40
cost_output_per_m = 4.40
cost_cache_read_per_m = 0.26
```

**Acceptance**: Config parses without errors.
**Verification**: `cargo test -p roko-core -- glm_config_parse`

---

### 2D.02 — Implement GLM thinking mode injection via extra_params

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: When creating a request for a model with `supports_thinking = true` and the provider is Z.AI, inject the thinking parameters:

```rust
fn inject_glm_params(body: &mut serde_json::Map<String, Value>, model: &ModelProfile, options: &RequestOptions) {
    if model.supports_thinking {
        let thinking = serde_json::json!({
            "type": if options.enable_thinking.unwrap_or(true) { "enabled" } else { "disabled" },
            "clear_thinking": !options.preserve_thinking.unwrap_or(false)
        });
        body.insert("thinking".to_string(), thinking);
    }
    if model.supports_tool_streaming.unwrap_or(false) {
        body.insert("tool_stream".to_string(), Value::Bool(true));
    }
}
```

**Context**: GLM-5.1's thinking is enabled by default. The `clear_thinking` parameter controls whether reasoning from previous turns is preserved in context. Setting it to `false` (preserve = true) improves cache hit rates and coherence across turns.

**Acceptance**: Request body for GLM-5.1 includes `thinking` and `tool_stream` fields.
**Verification**: `cargo test -p roko-agent -- glm_thinking_injection`

---

### 2D.03 — Parse GLM reasoning_content from response

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: In response parsing, extract `reasoning_content` alongside `content`:

```rust
fn parse_glm_response(json: &Value) -> (String, Option<String>) {
    let message = &json["choices"][0]["message"];
    let content = message["content"].as_str().unwrap_or("").to_string();
    let reasoning = message["reasoning_content"].as_str().map(|s| s.to_string());
    (content, reasoning)
}
```

**Acceptance**: Response with `reasoning_content` produces `ChatResponse.reasoning = Some(...)`.
**Verification**: `cargo test -p roko-agent -- glm_reasoning_parse`

---

### 2D.04 — Parse GLM content_filter metadata

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: Extract the GLM-specific `content_filter` array from responses:

```json
"content_filter": [
  {"role": "user", "level": 2},
  {"role": "assistant", "level": 0}
]
```

Store in `ResponseMetadata.content_filter`.

**Context**: Level 0 is most severe (content blocked). Level 3 is least severe. The health tracking system can use this to detect policy-related failures.

**Acceptance**: `content_filter` array is preserved in metadata when present.
**Verification**: `cargo test -p roko-agent -- glm_content_filter`

---

### 2D.05 — Handle GLM non-standard finish_reason values

**File**: `crates/roko-agent/src/translate/mod.rs`
**What**: Ensure `normalize_finish_reason()` handles GLM-specific values:
- `"sensitive"` → `FinishReason::ContentFilter`
- `"network_error"` → `FinishReason::Error("network_error")`
- `"model_context_window_exceeded"` → `FinishReason::Error("context_overflow")`

**Acceptance**: All 3 GLM-specific values are handled.
**Verification**: `cargo test -p roko-agent -- glm_finish_reasons`

---

### 2D.06 — Classify GLM-specific error codes in OpenAiCompatAdapter

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: Extend `classify_error()` to handle Z.AI's business error codes:

```rust
fn classify_zai_error(status: u16, body: &Value) -> ProviderError {
    if let Some(code) = body.pointer("/error/code").and_then(|v| v.as_str()) {
        match code {
            "1302" => ProviderError::RateLimit { retry_after_ms: Some(5000) },
            "1303" | "1304" | "1305" => ProviderError::RateLimit { retry_after_ms: Some(60000) },
            "1301" => ProviderError::ContentPolicy,
            "1000" | "1001" | "1002" | "1003" | "1004" => ProviderError::AuthFailure,
            "1211" => ProviderError::ModelNotFound,
            "1261" => ProviderError::ContextOverflow,
            _ => ProviderError::Other(format!("Z.AI error {}", code)),
        }
    } else {
        // Fall back to HTTP status
        OpenAiCompatAdapter.classify_error(status, body)
    }
}
```

**Context**: Z.AI uses nested business error codes (1000-1313) inside the JSON body, separate from the HTTP status code. The health tracking system needs these classified correctly.

**Acceptance**: `classify_error(429, {"error": {"code": "1302"}})` returns `ProviderError::RateLimit`.
**Verification**: `cargo test -p roko-agent -- zai_error_classify`

---

### 2D.07 — Add GLM web search tool rendering

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: When `ToolDef::WebSearch` is in the tools list, render it as:

```json
{
  "type": "web_search",
  "web_search": {
    "enable": true,
    "search_engine": "search_std",
    "count": 10,
    "content_size": "high"
  }
}
```

**Context**: The web search tool is GLM-specific. It provides real-time web results injected into the model's context. The response includes a `web_search` array with titles, URLs, and content.

**Acceptance**: `render_tools()` with a `WebSearch` tool produces the correct JSON.
**Verification**: `cargo test -p roko-agent -- glm_web_search_render`

---

### 2D.08 — Add GLM native MCP tool rendering

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: When `ToolDef::McpTool` is in the tools list, render it as:

```json
{
  "type": "mcp",
  "mcp": {
    "server_label": "zread",
    "server_url": "https://api.z.ai/api/mcp/zread/mcp",
    "transport_type": "http",
    "allowed_tools": ["search_doc", "read_file"],
    "headers": {"Authorization": "Bearer KEY"}
  }
}
```

**Context**: GLM-5.1 is one of the only LLM APIs with native MCP support at the API level. The model itself can call MCP servers — this is distinct from client-side MCP (which roko handles via `--mcp-config`).

**Acceptance**: `render_tools()` with an `McpTool` produces valid JSON.
**Verification**: `cargo test -p roko-agent -- glm_mcp_tool_render`

---

### 2D.09 — Add GLM model profile to default static table

**File**: `crates/roko-agent/src/translate/capability.rs`
**What**: Add GLM model slug patterns to `capabilities_for()`:

```rust
if slug.starts_with("glm-5") || slug == "glm-5.1" {
    return ModelCapabilities {
        supports_tools: true,
        supports_parallel_tool_calls: true,
        tool_format: ToolFormat::OpenAiJson,
        max_tools_before_degrade: 128,
        supports_thinking: true,
        supports_web_search: true,
        supports_mcp_tools: true,
        supports_tool_streaming: true,
        ..Default::default()
    };
}
```

**Acceptance**: `capabilities_for("glm-5.1")` returns correct capabilities.
**Verification**: `cargo test -p roko-agent -- glm_capabilities`

---

### 2D.10 — Add GLM cost data to CostTable

**File**: `crates/roko-learn/src/costs_db.rs`
**What**: Add GLM-5.1, GLM-5, GLM-4.7 pricing to the default cost table:

| Model | Input/M | Output/M | Cache/M |
|---|---|---|---|
| glm-5.1 | 1.40 | 4.40 | 0.26 |
| glm-5 | 1.00 | 3.20 | — |
| glm-4.7 | 0.60 | 2.20 | — |

**Acceptance**: Cost lookup for `"glm-5.1"` returns correct rates.
**Verification**: `cargo test -p roko-learn -- glm_cost_table`

---

### 2D.11 — Add GLM to CascadeRouter model slug detection

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: When GLM models are in the `model_slugs` list, ensure the router can select them. Update `default_static_table()` to include GLM mappings if GLM slugs are present.

**Acceptance**: Router with `["claude-sonnet-4-6", "glm-5.1"]` can select either model.
**Verification**: `cargo test -p roko-learn -- cascade_router_glm`

---

### 2D.12 — Write mock integration test: GLM-5.1 full tool loop

**File**: `crates/roko-agent/tests/glm_tool_loop.rs` (new)
**What**: Test the full tool loop with a mock GLM-5.1 response:
1. Send prompt with tools [Read, Edit]
2. Mock returns thinking + tool_call
3. Parse response, extract reasoning + tool call
4. Render tool result
5. Mock returns final answer

Use `MockHttpPoster` to simulate Z.AI responses.

**Acceptance**: Full loop completes with reasoning captured and tool call executed.
**Verification**: `cargo test -p roko-agent -- glm_full_tool_loop`

---

### 2D.13 — Write mock test: GLM web search response parsing

**File**: `crates/roko-agent/tests/glm_web_search.rs` (new)
**What**: Test parsing a GLM response that includes the `web_search` results array:

```json
{
  "choices": [...],
  "web_search": [
    {"title": "Result 1", "link": "https://...", "content": "..."}
  ]
}
```

**Acceptance**: Web search results are captured in `ResponseMetadata`.
**Verification**: `cargo test -p roko-agent -- glm_web_search_response`

---

### 2D.14 — Update from_model() to not route GLM slugs to Cursor

**File**: `crates/roko-core/src/agent.rs`
**What**: The current `is_cursor_slug()` function (line 82) does not match `glm-*` slugs, so this may already be correct. Verify and add a test.

If GLM slugs are NOT caught by `is_cursor_slug()`, they fall through to `Codex` (which is `OpenAiCompat`). This is correct behavior for the fallback path.

**Acceptance**: `AgentBackend::from_model("glm-5.1")` returns `Codex` (not Cursor).
**Verification**: `cargo test -p roko-core -- glm_backend_routing`
