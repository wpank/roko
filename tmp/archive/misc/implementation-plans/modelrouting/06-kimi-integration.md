# 06 — Kimi-K2.5 First-Class Backend Integration

> **Priority**: 🟡 P1 — Strongest open-weight coding model, cheapest frontier option
> **Status**: Not started
> **Depends on**: 02 (registry), 03 (adapters), 04 (translator extensions)
> **Blocks**: None (can be done in parallel with 05, 07)

## Problem Statement

Kimi-K2.5 (Moonshot AI, Jan 2026) leads LiveCodeBench, has 256K context, native multimodal, and costs $0.60/$3.00 per M tokens ($0.38/$1.72 via OpenRouter). Its API is OpenAI-compatible but has extensions: thinking mode, partial continuation, non-standard tool call IDs, vision input, and agent swarm behavior.

Currently `kimi-*` slugs route to Cursor (`is_cursor_slug` at line 82 of `agent.rs`), which is wrong for direct API access.

## Kimi-K2.5 Capabilities Summary

| Capability | API Parameter | Standard? |
|---|---|---|
| Tool calling | `tools: [{type: "function", ...}]` | Yes (OpenAI format) |
| Thinking mode | `thinking: {type: "enabled"}` | No (Kimi-specific) |
| Partial continuation | `partial: true` on assistant messages | No (Kimi-specific) |
| Vision (images) | `content: [{type: "image_url", ...}]` | Partial (base64 only, no URL) |
| Vision (video) | `content: [{type: "video_url", ...}]` | No (Kimi-specific) |
| JSON mode | `response_format: {type: "json_object"}` | Yes |
| Reasoning content | `reasoning_content` in response | No (shared with GLM) |
| Cache key hint | `prompt_cache_key` | No (Kimi-specific) |
| Non-standard tool IDs | `functions.<name>:<idx>` | No (Kimi-specific) |
| Cached tokens | `usage.cached_tokens` | Partial |
| 128 tools max | Limit | Kimi-specific |
| Agent swarm | Emergent (not API param) | Model behavior |

## Endpoints

| Endpoint | URL |
|---|---|
| OpenAI-compat | `https://api.moonshot.ai/v1` |
| Anthropic-compat | `https://api.moonshot.ai/anthropic` |

## Pricing

| Model | Input/M | Output/M | Cache Read/M |
|---|---|---|---|
| kimi-k2.5 | $0.60 | $3.00 | $0.10 |
| kimi-k2.5 (OpenRouter) | $0.38 | $1.72 | — |
| kimi-k2-thinking | $0.60 | $2.50 | $0.15 |

---

## Checklist

### 2E.01 — Add Moonshot provider config example

**File**: `examples/roko-kimi.toml`
**What**: Complete example config for Kimi-K2.5 via Moonshot direct:

```toml
[providers.moonshot]
kind = "openai_compat"
base_url = "https://api.moonshot.ai/v1"
api_key_env = "MOONSHOT_API_KEY"
timeout_ms = 180000

[models.kimi-k2-5]
provider = "moonshot"
slug = "kimi-k2.5"
context_window = 256000
max_output = 65535
supports_tools = true
supports_thinking = true
supports_vision = true
supports_partial = true
tool_format = "openai_json"
cost_input_per_m = 0.60
cost_output_per_m = 3.00
cost_cache_read_per_m = 0.10
max_tools = 128
```

**Acceptance**: Config parses without errors.
**Verification**: `cargo test -p roko-core -- kimi_config_parse`

---

### 2E.02 — Implement Kimi thinking mode injection

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: When creating a request for Kimi-K2.5 with `supports_thinking = true`:

```rust
fn inject_kimi_params(body: &mut Map<String, Value>, model: &ModelProfile, options: &RequestOptions) {
    if model.supports_thinking {
        let thinking = serde_json::json!({
            "type": if options.enable_thinking.unwrap_or(true) { "enabled" } else { "disabled" }
        });
        body.insert("thinking".to_string(), thinking);
    }
    if let Some(ref cache_key) = options.cache_key {
        body.insert("prompt_cache_key".to_string(), Value::String(cache_key.clone()));
    }
}
```

**Context**: Kimi's thinking format is simpler than GLM's (no `clear_thinking`). When thinking is enabled, `temperature` is fixed at 1.0 and `tool_choice` can only be `"auto"` or `"none"`.

**Acceptance**: Request body for Kimi includes `thinking` field.
**Verification**: `cargo test -p roko-agent -- kimi_thinking_injection`

---

### 2E.03 — Normalize Kimi tool call IDs

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: Kimi returns tool call IDs in format `functions.<name>:<idx>` (e.g., `functions.Read:0`). The current `parse_calls()` uses the ID as-is. This is fine — but document it and add a test.

When sending tool results back, the `tool_call_id` must match exactly what Kimi returned. Do NOT normalize the ID.

**Acceptance**: `parse_calls()` preserves Kimi-format IDs. `render_results()` uses the same ID.
**Verification**: `cargo test -p roko-agent -- kimi_tool_call_ids`

---

### 2E.04 — Implement partial continuation support

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: Add support for Kimi's `partial` continuation. When a response has `finish_reason: "length"` (truncated), the next request can include the truncated assistant message with `"partial": true` to continue generation.

```rust
/// Build a continuation message for Kimi's partial mode.
pub fn build_partial_continuation(truncated_content: &str) -> serde_json::Value {
    serde_json::json!({
        "role": "assistant",
        "content": truncated_content,
        "partial": true
    })
}
```

**Context**: This is a Kimi-specific feature. Standard OpenAI has no equivalent. It's useful for very long outputs (>65K tokens) where the model hits max_tokens.

**Acceptance**: `build_partial_continuation("truncated...")` produces valid JSON with `partial: true`.
**Verification**: `cargo test -p roko-agent -- kimi_partial_continuation`

---

### 2E.05 — Handle Kimi vision input (base64-only constraint)

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: Kimi-K2.5 supports image input but ONLY via base64 encoding (no URL passthrough). Add a check:

```rust
fn validate_vision_input(messages: &[ChatMessage], model: &ModelProfile) -> Result<(), AgentCreationError> {
    if !model.supports_vision {
        return Ok(());  // No vision, no validation needed
    }
    for msg in messages {
        if let Some(content_blocks) = msg.content_blocks() {
            for block in content_blocks {
                if block.is_image_url() && !block.is_base64() {
                    return Err(AgentCreationError::MissingConfig(
                        "Kimi requires base64-encoded images, not URLs".into()
                    ));
                }
            }
        }
    }
    Ok(())
}
```

**Context**: Other providers (OpenAI, Anthropic) support image URLs. Kimi requires `data:image/png;base64,...` format only. This constraint should be checked at request build time, not at response parse time.

**Acceptance**: Image URL in a Kimi request produces a clear error.
**Verification**: `cargo test -p roko-agent -- kimi_vision_base64_only`

---

### 2E.06 — Parse Kimi cached_tokens from usage

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: Kimi returns cached tokens at `usage.cached_tokens` (not nested under `prompt_tokens_details`):

```json
"usage": {
  "prompt_tokens": 50000,
  "completion_tokens": 500,
  "cached_tokens": 48000
}
```

Update the usage parser to check both locations:
1. `usage.prompt_tokens_details.cached_tokens` (GLM format)
2. `usage.cached_tokens` (Kimi format)

**Acceptance**: Both formats produce the same `usage.cache_read_tokens` value.
**Verification**: `cargo test -p roko-agent -- kimi_cached_tokens`

---

### 2E.07 — Add Kimi model profile to capability detection

**File**: `crates/roko-agent/src/translate/capability.rs`
**What**: Add Kimi slug patterns:

```rust
if slug.starts_with("kimi-k2") {
    return ModelCapabilities {
        supports_tools: true,
        supports_parallel_tool_calls: true,
        tool_format: ToolFormat::OpenAiJson,
        max_tools_before_degrade: 128,
        supports_thinking: true,
        supports_vision: slug.contains("k2.5") || slug.contains("k2-5"),
        supports_partial: true,
        ..Default::default()
    };
}
```

**Acceptance**: `capabilities_for("kimi-k2.5")` returns correct capabilities.
**Verification**: `cargo test -p roko-agent -- kimi_capabilities`

---

### 2E.08 — Fix from_model() routing for Kimi slugs

**File**: `crates/roko-core/src/agent.rs`
**What**: Currently `is_cursor_slug()` matches `kimi-*`, routing all Kimi models to Cursor. When the new provider system is active (config has `[providers.*]`), this fallback should not be reached. But for safety, update the heuristic:

If `kimi-*` is used without a `[providers.*]` config entry, it should fall through to `Codex` (OpenAI-compat), not Cursor.

Remove `slug.starts_with("kimi-")` from `is_cursor_slug()`.

**Context**: This is a breaking change for anyone using `kimi-*` slugs with the Cursor backend. But that's the wrong behavior — Kimi's API is OpenAI-compat, not Cursor ACP.

**Acceptance**: `AgentBackend::from_model("kimi-k2.5")` returns `Codex`, not `Cursor`.
**Verification**: `cargo test -p roko-core -- kimi_not_cursor`

---

### 2E.09 — Add Kimi cost data to CostTable

**File**: `crates/roko-learn/src/costs_db.rs`
**What**: Add Kimi-K2.5 pricing:

| Model | Input/M | Output/M | Cache/M |
|---|---|---|---|
| kimi-k2.5 | 0.60 | 3.00 | 0.10 |
| kimi-k2-thinking | 0.60 | 2.50 | 0.15 |

**Acceptance**: Cost lookup for `"kimi-k2.5"` returns correct rates.
**Verification**: `cargo test -p roko-learn -- kimi_cost_table`

---

### 2E.10 — Write mock integration test: Kimi full tool loop

**File**: `crates/roko-agent/tests/kimi_tool_loop.rs` (new)
**What**: Test the full tool loop with mock Kimi responses:
1. Send prompt with tools
2. Mock returns thinking + parallel tool_calls (2 concurrent)
3. Parse response, verify `functions.Read:0` and `functions.Edit:1` IDs preserved
4. Render both tool results
5. Mock returns final answer

**Acceptance**: Full loop completes with 2 parallel tool calls handled correctly.
**Verification**: `cargo test -p roko-agent -- kimi_full_tool_loop`

---

### 2E.11 — Write mock test: Kimi partial continuation

**File**: `crates/roko-agent/tests/kimi_partial.rs` (new)
**What**: Test the partial continuation flow:
1. Send prompt
2. Mock returns truncated response with `finish_reason: "length"`
3. Build continuation with `partial: true`
4. Mock returns rest of response with `finish_reason: "stop"`

**Acceptance**: Two-turn continuation produces complete output.
**Verification**: `cargo test -p roko-agent -- kimi_partial_flow`

---

### 2E.12 — Write mock test: Kimi thinking mode with tool calls

**File**: `crates/roko-agent/tests/kimi_thinking_tools.rs` (new)
**What**: Test that thinking mode and tool calls work together:
1. Mock response has both `reasoning_content` and `tool_calls`
2. Verify reasoning is extracted
3. Verify tool calls are parsed
4. Verify `reasoning_content` is included in the next request's message history

**Context**: Per Kimi docs, `reasoning_content` from previous turns must be included unmodified in subsequent messages for coherence.

**Acceptance**: Reasoning preserved across turns.
**Verification**: `cargo test -p roko-agent -- kimi_thinking_with_tools`

---

### 2E.13 — Add Kimi to CascadeRouter

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Same as 2D.11 but for Kimi models.

**Acceptance**: Router with Kimi slugs can select them.
**Verification**: `cargo test -p roko-learn -- cascade_router_kimi`

---

### 2E.14 — Document Kimi-specific constraints

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: Add doc comments documenting Kimi constraints:
- Images: base64 only, no URL
- Thinking: fixes temperature=1.0, top_p=0.95, n=1
- tool_choice: only "auto" or "none" with thinking enabled
- $web_search built-in tool incompatible with thinking mode
- Tool call IDs: `functions.<name>:<idx>` format
- Max 128 tools per request
- 2-hour timeout per request
- `reasoning_content` must be preserved in conversation history

**Acceptance**: Doc comments are comprehensive and mention all 8 constraints.
**Verification**: `cargo doc -p roko-agent --no-deps 2>&1 | grep -c warning` should not increase.
