# 04 — Translator Extensions

> **Priority**: 🟡 P1 — Required for thinking mode, extended tools, rich response metadata
> **Status**: Not started
> **Depends on**: 03 (provider adapters)
> **Blocks**: 05 (GLM), 06 (Kimi)

## Problem Statement

> **Note**: The existing `ToolDef` at `crates/roko-core/src/tool/def.rs` L260 is a **struct**, not
> an enum. Task 2C.04 proposes extending it to an enum for GLM's extended tool types. This is a
> breaking change to a core type — consider a `ToolType` wrapper or parallel `ExtendedToolDef`
> instead of changing `ToolDef` itself. The `ToolRegistry`, `ToolDispatcher`, and all 16 builtin
> handlers reference `ToolDef` as a struct.

The current `Translator` trait and response types don't handle:
1. **Thinking/reasoning content** — GLM-5.1 and Kimi-K2.5 both return `reasoning_content` alongside `content`
2. **Extended tool types** — GLM-5.1 supports `web_search`, `retrieval`, and `mcp` tool types beyond standard `function`
3. **Response metadata** — cached tokens, content filters, non-standard finish reasons
4. **Partial continuation** — Kimi's `partial: true` for continuing truncated output

These all manifest in the response parsing and request building, which currently only support basic text + tool_calls.

## What Exists

| Component | Path | Lines | Status |
|---|---|---|---|
| Translator trait | `crates/roko-agent/src/translate/mod.rs` | 54–81 | 🔌 No thinking support |
| BackendResponse enum | `crates/roko-agent/src/translate/mod.rs` | 116–124 | 🔌 3 variants |
| OpenAiTranslator | `crates/roko-agent/src/translate/openai.rs` | 532 lines | 🔌 Standard function tools only |
| ClaudeTranslator | `crates/roko-agent/src/translate/claude.rs` | — | 🔌 Anthropic blocks |
| ReActTranslator | `crates/roko-agent/src/translate/react.rs` | — | 🔌 Fallback |
| ModelCapabilities | `crates/roko-agent/src/translate/capability.rs` | 32–42 | 🔌 4 fields |
| ToolFormat enum | `crates/roko-agent/src/translate/capability.rs` | — | 🔌 4 variants |
| Usage struct | `crates/roko-agent/src/usage.rs` | — | 🔌 Has cache fields |

---

## Checklist

### 2C.01 — Extend BackendResponse to carry reasoning content

**File**: `crates/roko-agent/src/translate/mod.rs`
**What**: The `BackendResponse` enum currently has `Json`, `StreamJson`, `Text`. The `extract_text()` method only returns the main content. Add a method to extract reasoning:

```rust
impl BackendResponse {
    /// Extract reasoning/thinking content from the response.
    /// GLM-5.1: choices[0].message.reasoning_content
    /// Kimi-K2.5: choices[0].message.reasoning_content
    /// Claude: content blocks with type "thinking"
    pub fn extract_reasoning(&self) -> Option<String> {
        match self {
            Self::Json(v) => v.pointer("/choices/0/message/reasoning_content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            Self::StreamJson(events) => {
                // Concatenate reasoning_content from stream events
                // ...
            },
            Self::Text(_) => None,
        }
    }
}
```

**Context**: Both GLM-5.1 and Kimi-K2.5 return `reasoning_content` as a peer to `content` in the response message. This is not part of the standard OpenAI format but is used by both Chinese providers and by Anthropic (as thinking blocks).

**Acceptance**: `BackendResponse::Json` with a `reasoning_content` field returns `Some(reasoning)`.
**Verification**: `cargo test -p roko-agent -- extract_reasoning`

---

### 2C.02 — Add ChatResponse struct as canonical response type

**File**: `crates/roko-agent/src/translate/mod.rs`
**What**: Add a canonical response type that the adapter layer returns:

```rust
/// Canonical response from any provider, after adapter parsing.
#[derive(Debug, Clone, Default)]
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct ResponseMetadata {
    pub response_id: Option<String>,
    pub model_used: Option<String>,
    pub cached_tokens: Option<u64>,
    pub content_filter: Option<serde_json::Value>,
    pub provider_latency_ms: Option<u64>,
    pub raw_finish_reason: Option<String>,  // provider's original string
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FinishReason {
    #[default]
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error(String),
}
```

**Context**: Currently, agent impls directly parse responses into `AgentResult`. The `ChatResponse` type provides an intermediate canonical form that the adapter layer produces, making it possible to add reasoning and metadata without changing the Agent trait.

**Acceptance**: `ChatResponse` compiles with all fields. `FinishReason` has the 5 variants.
**Verification**: `cargo test -p roko-agent -- chat_response`

---

### 2C.03 — Extend ModelCapabilities with thinking and vision flags

**File**: `crates/roko-agent/src/translate/capability.rs`
**What**: Add fields to `ModelCapabilities`:

```rust
pub struct ModelCapabilities {
    pub supports_tools: bool,
    pub supports_parallel_tool_calls: bool,
    pub tool_format: ToolFormat,
    pub max_tools_before_degrade: u8,
    // New fields:
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub supports_web_search: bool,       // GLM built-in web search
    pub supports_mcp_tools: bool,        // GLM native MCP
    pub supports_partial: bool,          // Kimi partial continuation
    pub supports_tool_streaming: bool,   // GLM tool_stream
}
```

Update `capabilities_for(slug)` to populate these from the model profile when available:

```rust
pub fn capabilities_from_profile(profile: &ModelProfile) -> ModelCapabilities {
    ModelCapabilities {
        supports_tools: profile.supports_tools,
        supports_thinking: profile.supports_thinking,
        supports_vision: profile.supports_vision,
        // ...
    }
}
```

**Acceptance**: `capabilities_from_profile()` correctly maps all ModelProfile flags.
**Verification**: `cargo test -p roko-agent -- capabilities_from_profile`

---

### ~~2C.04~~ — ~~Add ToolDef variants for extended tool types~~ → SUPERSEDED by 2P.01–2P.02

> **SUPERSEDED**: Doc 18 task 2P.01 adds a `ToolSource` enum field to the existing `ToolDef` struct
> instead of converting ToolDef itself to an enum. This is non-breaking — all existing serialized
> data and handlers continue to work. Task 2P.02 updates the translator to render based on source type.
> The `metadata: Option<Value>` bag handles future extensibility without struct changes.

### ~~2C.04~~ — ~~Add ToolDef variants for extended tool types~~ (see 2P.01 instead)

**File**: `crates/roko-core/src/tools.rs` (or wherever ToolDef is defined)
**What**: GLM-5.1 supports 4 tool types: `function`, `web_search`, `retrieval`, `mcp`. Add variants:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolDef {
    #[serde(rename = "function")]
    Function {
        name: String,
        description: String,
        parameters: serde_json::Value,
    },
    #[serde(rename = "web_search")]
    WebSearch {
        #[serde(default)]
        config: WebSearchConfig,
    },
    #[serde(rename = "retrieval")]
    Retrieval {
        knowledge_id: String,
        #[serde(default)]
        prompt_template: Option<String>,
    },
    #[serde(rename = "mcp")]
    McpTool {
        server_label: String,
        server_url: String,
        transport_type: String,
        allowed_tools: Option<Vec<String>>,
        headers: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebSearchConfig {
    pub enable: bool,
    pub search_engine: Option<String>,
    pub count: Option<u32>,
    pub search_recency_filter: Option<String>,
    pub content_size: Option<String>,
}
```

**Context**: Most providers only use `Function`. The extended types are GLM-specific but the enum design allows future providers to add their own tool types.

**Acceptance**: `ToolDef::WebSearch` serializes to `{"type": "web_search", "web_search": {...}}`.
**Verification**: `cargo test -p roko-core -- tool_def_extended`

---

### 2C.05 — Extend OpenAiTranslator to render extended tool types

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: Update `render_tools()` to handle `ToolDef::WebSearch`, `ToolDef::Retrieval`, and `ToolDef::McpTool` in addition to `ToolDef::Function`.

For non-function types, serialize them directly as JSON objects in the tools array:
```json
[
  {"type": "function", "function": {"name": "Read", ...}},
  {"type": "web_search", "web_search": {"enable": true, ...}}
]
```

**Acceptance**: `render_tools()` with a mixed vec of Function + WebSearch produces valid JSON.
**Verification**: `cargo test -p roko-agent -- render_extended_tools`

---

### 2C.06 — Add reasoning to AgentEfficiencyEvent

**File**: `crates/roko-learn/src/efficiency.rs`
**What**: Add an optional `reasoning_tokens` field to track thinking token usage:

```rust
pub struct AgentEfficiencyEvent {
    // ... existing fields ...
    pub reasoning_tokens: u64,  // New: tokens used for reasoning/thinking
}
```

**Context**: GLM-5.1 includes reasoning tokens in `completion_tokens` (no separate field). Kimi-K2.5 also includes them in completion tokens. We need to track them separately for cost analysis since reasoning-heavy tasks consume more output tokens.

**Acceptance**: Events can be created with `reasoning_tokens > 0`. JSONL serialization includes the field.
**Verification**: `cargo test -p roko-learn -- efficiency_reasoning_tokens`

---

### 2C.07 — Add reasoning to Episode

**File**: `crates/roko-learn/src/episode_logger.rs`
**What**: Add optional reasoning content to Episode for auditing:

```rust
pub struct Episode {
    // ... existing fields ...
    pub reasoning_summary: Option<String>,  // First 500 chars of reasoning, for debugging
}
```

**Context**: We don't store full reasoning (it can be huge) but a summary helps debug why a model made a particular decision.

**Acceptance**: Episodes with reasoning are serialized/deserialized from JSONL.
**Verification**: `cargo test -p roko-learn -- episode_reasoning`

---

### 2C.08 — Add finish_reason normalization

**File**: `crates/roko-agent/src/translate/mod.rs`
**What**: Add a function to normalize provider-specific finish reasons to `FinishReason`:

```rust
pub fn normalize_finish_reason(raw: &str) -> FinishReason {
    match raw {
        "stop" | "end_turn" => FinishReason::Stop,
        "length" | "max_tokens" => FinishReason::Length,
        "tool_calls" | "tool_use" => FinishReason::ToolCalls,
        "content_filter" | "sensitive" => FinishReason::ContentFilter,
        "network_error" | "model_context_window_exceeded" => FinishReason::Error(raw.to_string()),
        other => FinishReason::Error(other.to_string()),
    }
}
```

**Context**: GLM-5.1 returns non-standard finish reasons: `"sensitive"`, `"network_error"`, `"model_context_window_exceeded"`. These need to be mapped to canonical types for the health tracking system.

**Acceptance**: `normalize_finish_reason("sensitive")` returns `FinishReason::ContentFilter`.
**Verification**: `cargo test -p roko-agent -- normalize_finish_reason`

---

### 2C.09 — Extract cached_tokens from response usage

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: Update response parsing to extract cached token counts:

GLM-5.1 format:
```json
"usage": {
  "prompt_tokens": 1200,
  "completion_tokens": 300,
  "prompt_tokens_details": { "cached_tokens": 800 }
}
```

Kimi-K2.5 format:
```json
"usage": {
  "prompt_tokens": 1200,
  "completion_tokens": 300,
  "cached_tokens": 800
}
```

**Acceptance**: Both formats produce `usage.cache_read_tokens = 800`.
**Verification**: `cargo test -p roko-agent -- parse_cached_tokens`

---

### 2C.10 — Write end-to-end test: thinking response parsing

**File**: `crates/roko-agent/tests/thinking_response.rs` (new)
**What**: Test that a mock response with `reasoning_content` is correctly parsed:

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "reasoning_content": "Let me think step by step...",
      "content": "The answer is 42.",
      "tool_calls": [{"id": "call_1", "function": {"name": "Read", "arguments": "{}"}}]
    },
    "finish_reason": "tool_calls"
  }],
  "usage": {"prompt_tokens": 100, "completion_tokens": 50, "prompt_tokens_details": {"cached_tokens": 80}}
}
```

Verify:
- `ChatResponse.reasoning` = `Some("Let me think step by step...")`
- `ChatResponse.content` = `"The answer is 42."`
- `ChatResponse.tool_calls` has 1 entry
- `ChatResponse.usage.cache_read_tokens` = 80
- `ChatResponse.finish_reason` = `FinishReason::ToolCalls`

**Acceptance**: All assertions pass.
**Verification**: `cargo test -p roko-agent -- thinking_response_parsing`
