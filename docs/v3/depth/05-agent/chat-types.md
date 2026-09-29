# 05-agent/chat-types -- Chat Types

> Canonical message types shared across prompt assembly, tool loops, and
> provider adapters: ChatMessage, ChatResponse, ChatRequest, Usage,
> FinishReason, ResponseMetadata, and the BackendResponse wire layer.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** `crates/roko-core/src/chat_types.rs`

---

## 1. Design Principle

Chat types are the **lingua franca** between Roko's layers. Every provider
speaks a different wire format (Anthropic content blocks, OpenAI message objects,
Gemini Parts), but internally everything is normalized to these canonical types.

The types live in `roko-core` so that both `roko-compose` (prompt assembly) and
`roko-agent` (provider adapters) can depend on them without circular dependencies.
This resolved the earlier layer problem where `ChatResponse` lived in
`roko-agent::translate` and `roko-compose` could not import it.

---

## 2. ChatMessage

The tagged enum representing one message in a conversation:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum ChatMessage {
    System { content: String },
    User { content: MessageContent },
    Assistant {
        content: Option<String>,
        reasoning_content: Option<String>,
        tool_calls: Option<Vec<ToolCallMessage>>,
        partial: bool,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}
```

### Role semantics

| Role | Purpose | Content type |
|------|---------|-------------|
| `System` | Global instructions, role persona, project context | Plain text |
| `User` | Task prompt, follow-up questions, image inputs | `MessageContent` (text or blocks) |
| `Assistant` | Model response, reasoning traces, tool call requests | Optional text + optional reasoning + optional tool calls |
| `Tool` | Tool execution result, keyed by `tool_call_id` | Plain text |

### Assistant message fields

- **`content`** -- The model's text response. May be `None` when the model
  emits only tool calls without accompanying text.
- **`reasoning_content`** -- Extended thinking / chain-of-thought output.
  Populated when the model supports thinking (Anthropic thinking blocks,
  OpenAI reasoning_content, DeepSeek reasoning).
- **`tool_calls`** -- Parsed tool invocations. Each `ToolCallMessage` carries
  an `id`, `function_name`, and `arguments` (as JSON string).
- **`partial`** -- Set to `true` for streaming partial messages that will be
  continued in subsequent events.

---

## 3. MessageContent and ContentBlock

User messages support multimodal content:

```rust
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

pub enum ContentBlock {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

pub struct ImageUrl {
    pub url: String,        // data URI or https URL
    pub detail: Option<String>,  // "auto", "low", "high"
}
```

### Image handling

- Vision-capable API paths (AnthropicApi, OpenAiCompat, GeminiApi) preserve
  ordered image blocks through to the provider.
- Image data URIs are redacted in `Debug` output to prevent leaking binary
  data into logs.
- Non-vision transports and models fail closed: if the transport does not
  support inline images, the request is rejected rather than silently dropping
  the image content.

---

## 4. ChatResponse

The canonical output of any LLM interaction, regardless of provider:

```rust
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
    pub metadata: ResponseMetadata,
    pub raw_assistant_message: Option<ChatMessage>,
    pub session: SessionState,
}
```

Every adapter parses its provider's wire format into this struct before any
downstream processing occurs. Callers never deal with provider-specific JSON.

### Fields in detail

| Field | Purpose |
|-------|---------|
| `content` | Final text (concatenated from content blocks for Anthropic) |
| `reasoning` | Extended thinking output (three wire formats handled) |
| `tool_calls` | Parsed `ToolCall` structs with id, name, arguments |
| `usage` | Token counts, cost, wall-clock time |
| `finish_reason` | Normalized stop condition |
| `metadata` | Provider-specific extensions |
| `raw_assistant_message` | Original ChatMessage for conversation replay |
| `session` | Session state for continuation |

---

## 5. FinishReason

Normalized stop conditions:

```rust
pub enum FinishReason {
    Stop,           // Model finished normally
    Length,         // Hit max_tokens limit
    ToolCalls,      // Model requested tool execution
    ContentFilter,  // Content policy triggered
    Error(String),  // Provider error
}
```

### Provider mapping

| Roko canonical | OpenAI | Anthropic | ZhipuAI | Perplexity |
|---------------|--------|-----------|---------|------------|
| `Stop` | `"stop"` | `"end_turn"` | `"stop"` | `"stop"` |
| `Length` | `"length"` | `"max_tokens"` | `"length"` | `"length"` |
| `ToolCalls` | `"tool_calls"` | `"tool_use"` | `"tool_calls"` | -- |
| `ContentFilter` | `"content_filter"` | -- | `"sensitive"` | -- |
| `Error(...)` | -- | -- | `"network_error"` | -- |

The `normalize_finish_reason()` function maps raw strings to canonical variants.
This normalization is critical for the ToolLoop: `ToolCalls` means dispatch and
continue, `Stop` means extract final answer.

---

## 6. Usage

Token counts and cost tracking:

```rust
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_create_tokens: u32,
    pub reasoning_tokens: u32,   // o-series/Codex: subset of output_tokens
    pub cost_usd: f32,
    pub wall_ms: u64,
}
```

### Cost computation

`fill_cost_from_pricing(input_per_m, output_per_m, cache_read_per_m,
cache_write_per_m)` computes `cost_usd` from token counts and per-million
pricing. It is a no-op when:

- Cost is already set (e.g., Claude CLI reports cost natively)
- Pricing data is missing (display shows "--")

The `total_tokens()` method returns `input_tokens + output_tokens`.

### Reasoning tokens

For o-series and Codex models, `reasoning_tokens` is a **subset** of
`output_tokens`, not additive. This distinction matters for cost calculation:
reasoning tokens are billed at the output rate.

---

## 7. ChatRequest

The canonical request sent to any provider:

```rust
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model_slug: String,
    pub tools: Vec<ToolDef>,
    pub tool_choice: ToolChoice,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub stream: bool,
    pub options: RequestOptions,
}
```

### ToolChoice

```rust
pub enum ToolChoice {
    Auto,                          // Model decides when to call tools
    None,                          // No tool calling allowed
    Required,                      // Must call at least one tool
    Specific { name: String },     // Must call this specific tool
}
```

---

## 8. ResponseMetadata

Provider-specific extensions that do not fit the canonical model:

```rust
pub struct ResponseMetadata {
    pub response_id: Option<String>,
    pub model_used: Option<String>,
    pub cached_tokens: Option<u64>,
    pub content_filter: Option<serde_json::Value>,
    pub web_search: Option<serde_json::Value>,
    pub provider_latency_ms: Option<u64>,
    pub raw_finish_reason: Option<String>,
    pub extra: HashMap<String, serde_json::Value>,
}
```

Notable fields:

- **`model_used`** -- When using OpenRouter, the actual model may differ from
  the requested one. Captured for cost attribution and quality tracking.
- **`cached_tokens`** -- Anthropic prompt caching returns cache-served token
  count. Feeds into `Usage` cost computation at the cache rate.
- **`web_search`** -- Perplexity `citations` and `search_results`, Gemini
  `grounding_metadata`. Captured as raw JSON.
- **`extra`** -- Open-ended map for provider-specific fields not yet promoted
  to first-class typed fields.

---

## 9. BackendResponse

Below `ChatResponse` sits the raw wire layer:

```rust
pub enum BackendResponse {
    Json(serde_json::Value),           // Single JSON (OpenAI, Anthropic non-streaming)
    StreamJson(Vec<serde_json::Value>),// Stream events (Claude CLI)
    Text(String),                      // Plain text
}
```

Each `Translator` implementation extracts text, reasoning, and tool calls from
its variant. The `extract_reasoning()` method handles four wire formats:

1. OpenAI-style `message.reasoning_content` (DeepSeek, QwQ)
2. Anthropic-style `content` blocks with `type: "thinking"`
3. Stream-JSON `delta.reasoning_content` or `delta.thinking`
4. Content block events with `type == "thinking"`

---

## 10. The Translate Pipeline

```
Provider API Response (JSON/stream/text)
    |
    v
BackendResponse (raw wire)
    |
    +-- extract_text()      -> String
    +-- extract_reasoning() -> Option<String>
    |
    v
Translator::parse_calls() -> Vec<ToolCall>
    |
    v
ChatResponse {
    content:       extract_text(),
    reasoning:     extract_reasoning(),
    tool_calls:    parse_calls(),
    usage:         parsed from response usage block,
    finish_reason: normalize_finish_reason(raw),
    metadata:      provider-specific extensions,
}
```

This pipeline runs once per LLM response in the adapter layer. The ToolLoop
receives `BackendResponse` from `LlmBackend::send_turn()` and uses the
`Translator` to parse tool calls. Full `ChatResponse` assembly happens when
the final result surfaces to the runtime.

---

## 11. Citations

1. `crates/roko-core/src/chat_types.rs` -- ChatMessage, ChatResponse,
   ChatRequest, Usage, FinishReason, MessageContent, ContentBlock.
2. `crates/roko-agent/src/tool_loop/mod.rs` -- BackendResponse, Translator
   trait, LlmBackend.
3. `crates/roko-core/src/agent.rs` -- ProviderKind (determines wire format).
4. `crates/roko-core/src/config/schema.rs` -- ModelProfile capability flags
   (supports_thinking, supports_vision, etc.).
