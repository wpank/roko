# 05-09 -- Format Translation

> **Implementation status (2026-09):** Shipping. Six translators
> (OpenAI, Ollama, Claude, Gemini, Hermes, ReAct) are live and
> selected automatically from `ModelProfile::tool_format`. The
> `Translator` trait is stable.

---

## Why Format Translation Matters

Research demonstrates 5--30 accuracy-point differences when tool calls
are serialized in a format the model was not trained on:

- **Meta-Harness** (Lee et al., 2026; arXiv:2603.28052) Principle 1:
  "Design tools for the model, not for humans."
- **WildToolBench**: 15--20% accuracy drop for models presented with
  non-native tool schemas.
- **Qwen3-coder**: documented format-switch regression when the tool
  array exceeds 5 entries.

The `Translator` layer ensures every model sees tools in its preferred
wire format. The `ModelProfile::tool_format` field is the selection key.

| Model family | Native format | `tool_format` value |
|---|---|---|
| Claude (API) | Anthropic content blocks | `anthropic_blocks` |
| Claude (CLI) | `--tools=Name,Name` flag | CLI flag |
| OpenAI / GPT | JSON function calling | `openai_json` |
| Ollama / Llama | OpenAI-compatible JSON | `ollama_json` |
| Gemini (native) | `functionDeclarations` / `functionCall` | `gemini_native` |
| Hermes / Qwen 3 | XML `<tool_call>` | `hermes_xml` |
| DeepSeek | OpenAI-compatible JSON | `openai_json` |
| Models without tool support | ReAct in system prompt | `react_text` |

---

## The Translator Trait

Defined at `crates/roko-agent/src/translate/mod.rs`:

```rust
pub trait Translator: Send + Sync {
    /// Which wire format this translator emits/parses.
    fn format(&self) -> ToolFormat;

    /// Serialize the tool catalog into the backend's expected shape.
    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools;

    /// Parse the backend's response into canonical tool calls.
    fn parse_calls(&self, response: &BackendResponse)
        -> Result<Vec<ToolCall>, TranslatorError>;

    /// Serialize tool results for the next turn.
    fn render_results(&self, results: &[(ToolCall, ToolResult)])
        -> RenderedResults;

    /// Extract assistant message for conversation history injection.
    fn render_assistant_message(&self, response: &BackendResponse)
        -> Option<serde_json::Value> {
        None  // Default: no-op
    }
}
```

### Design properties

1. **Sync and pure** -- no I/O, no side effects. Given identical inputs
   the translator produces identical outputs. This makes them trivially
   testable.
2. **One instance per backend** -- the translator is selected once when
   the agent is constructed and reused for every turn.
3. **Bidirectional** -- `render_tools` goes canonical-to-wire;
   `parse_calls` goes wire-to-canonical.
4. **No state** -- translators carry no mutable state between calls.

---

## The Six Translators

### 1. OpenAiTranslator (`translate/openai.rs`)

The workhorse translator. Handles the OpenAI chat completions tool
format used by most providers including DeepSeek, ZhipuAI GLM,
Moonshot Kimi, and OpenRouter.

**`render_tools`** -- converts `ToolDef` to the OpenAI `functions` array:

```json
{
    "type": "function",
    "function": {
        "name": "read_file",
        "description": "Read a file from the filesystem",
        "parameters": { /* JSON Schema from ToolDef */ }
    }
}
```

**`parse_calls`** -- extracts tool calls from
`choices[0].message.tool_calls`. Note: OpenAI returns `arguments` as
a JSON *string*, not a parsed object. The translator parses it into
`serde_json::Value`.

**`render_results`** -- formats tool results as `role: "tool"` messages:

```json
{
    "role": "tool",
    "tool_call_id": "call_abc123",
    "content": "file contents here..."
}
```

A `StrictOpenAiTranslator` variant is also exported, applying stricter
schema enforcement for providers that require it.

### 2. OllamaTranslator (`translate/ollama.rs`)

Similar to OpenAI but handles Ollama's slightly different JSON structure
where messages live under `message` instead of `choices[0].message`.

### 3. ClaudeTranslator (`translate/claude.rs`)

Handles Claude CLI's stream-JSON protocol.

**`render_tools`** -- returns `RenderedTools::CliFlag("Read,Edit,Bash,...
")` for the `--tools` flag.

**`parse_calls`** -- parses `tool_use` blocks from stream-JSON events:

```json
{
    "type": "tool_use",
    "id": "toolu_abc123",
    "name": "read_file",
    "input": { "file_path": "/src/main.rs" }
}
```

**`render_results`** -- returns `RenderedResults::HandledByBackend`
because Claude CLI manages its own tool-call loop internally. Roko does
not feed results back.

### 4. GeminiTranslator (`translate/gemini.rs`)

Handles Google Gemini's native REST format. Gemini uses
`functionDeclarations` for tool specs, `functionCall` for model-emitted
calls, and `functionResponse` for results.

**`render_tools`** -- converts `ToolDef` entries into the Gemini
`functionDeclarations` array structure.

**`parse_calls`** -- extracts `functionCall` entries from the response
body, mapping them into canonical `ToolCall` structs.

**`render_results`** -- produces `functionResponse` messages that
Gemini expects on the next turn.

### 5. HermesXmlTranslator (`translate/hermes.rs`)

Handles the NousResearch Hermes XML tool-call format, also used by
Qwen 3 models. Tool calls are wrapped in `<tool_call>` XML tags.

**`render_tools`** -- produces the Hermes-style XML tool descriptions
embedded in a system prompt fragment.

**`parse_calls`** -- uses regex/XML parsing to extract tool calls from
model output text containing `<tool_call>` blocks.

**`render_results`** -- formats results as `<tool_response>` blocks
for the next turn.

### 6. ReActTranslator (`translate/react.rs`)

Fallback for models without native function-calling support. Embeds
tool schemas directly in the system prompt and parses tool calls from
the model's natural-language output using the `Action:` / `Input:`
convention.

**`render_tools`** -- returns `RenderedTools::SystemPromptBlock(...)`:

```text
You have access to the following tools:

### read_file
Read a file from the filesystem.
Parameters:
- path (string, required): The file path to read

To use a tool, respond with:
Action: tool_name
Input: {"param": "value"}
```

**`parse_calls`** -- uses regex to extract `Action:` and `Input:` lines.

**`render_results`** -- returns `RenderedResults::TextBlock(...)`:

```text
Observation: [file contents here]
```

---

## Wire Format Types

### RenderedTools

```rust
pub enum RenderedTools {
    JsonArray(serde_json::Value),   // OpenAI, Ollama, HTTP APIs
    CliFlag(String),                // Claude CLI (--tools=...)
    SystemPromptBlock(String),      // ReAct fallback
}
```

### RenderedResults

```rust
pub enum RenderedResults {
    JsonMessages(serde_json::Value), // OpenAI, Ollama (tool result msgs)
    TextBlock(String),               // ReAct (Observation: ...)
    HandledByBackend,                // Claude CLI (drives own loop)
}
```

### BackendResponse

```rust
pub enum BackendResponse {
    Json(serde_json::Value),            // Single JSON (HTTP APIs)
    StreamJson(Vec<serde_json::Value>), // Stream events (Claude CLI)
    Text(String),                       // Plain text (ReAct)
}
```

---

## ModelCapabilities and Translator Selection

The `capability` submodule (`translate/capability.rs`) bridges model
profiles and translator selection:

```rust
pub struct ModelCapabilities {
    pub supports_tools: bool,
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub tool_format: ToolFormat,
    pub max_tools: Option<u32>,
}
```

Selection functions:

- `translator_for(format)` -- returns the translator for a given format.
- `translator_for_profile(profile)` -- derives capabilities from a
  `ModelProfile` and selects the translator.
- `translator_for_capabilities(caps)` -- selects from a
  `ModelCapabilities` struct.
- `capabilities_from_profile(profile)` -- extracts capabilities from
  config.

The selection is automatic based on the `tool_format` field in the model
profile.

---

## Reasoning Extraction

`BackendResponse` provides `extract_reasoning()` which handles four
different reasoning wire formats:

1. Anthropic `thinking` blocks in content arrays.
2. OpenAI `reasoning_content` field (o-series, Codex).
3. OpenRouter `reasoning` metadata field.
4. Plain-text reasoning markers in ReAct output.

Reasoning extraction feeds three downstream consumers:

1. **ChatResponse construction** -- the `reasoning` field is populated.
2. **Episode logging** -- reasoning content is logged separately.
3. **Cost computation** -- reasoning tokens may have different pricing.

---

## Finish Reason Normalization

The `normalize_finish_reason` function at `translate/mod.rs` maps
provider-specific stop reasons into canonical `FinishReason` values:

```rust
pub fn normalize_finish_reason(raw: &str) -> FinishReason {
    match raw {
        "stop" | "end_turn" => FinishReason::Stop,
        "length" | "max_tokens" => FinishReason::Length,
        "tool_calls" | "tool_use" => FinishReason::ToolCalls,
        "content_filter" | "sensitive" => FinishReason::ContentFilter,
        "network_error" => FinishReason::Error("network_error".into()),
        "model_context_window_exceeded" =>
            FinishReason::Error("context_overflow".into()),
        other => FinishReason::Error(other.to_string()),
    }
}
```

This normalization ensures the ToolLoop and dispatcher can make
decisions without provider-specific branching.

---

## Format Switching and Tool Truncation

Some models exhibit "format switching" behavior -- they perform well
with a given tool format up to a certain tool count, then degrade.
This is documented for Qwen3-coder (above 5 tools) and some smaller
Llama models (above 10 tools).

The `max_tools` field in `ModelProfile` addresses this. When set, the
adapter truncates the tool array before handing it to the Translator.
Selection criteria (in priority order):

1. Tools explicitly requested by the task definition.
2. Tools matching the agent's role permissions.
3. Most frequently used tools from episode history.

This truncation happens at the adapter level, *before* the Translator
sees the tools. The Translator always receives a tool array within the
model's comfortable range.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-agent/src/translate/mod.rs` | Translator trait, wire format enums, normalize_finish_reason |
| `crates/roko-agent/src/translate/openai.rs` | OpenAiTranslator, StrictOpenAiTranslator |
| `crates/roko-agent/src/translate/claude.rs` | ClaudeTranslator |
| `crates/roko-agent/src/translate/ollama.rs` | OllamaTranslator |
| `crates/roko-agent/src/translate/gemini.rs` | GeminiTranslator |
| `crates/roko-agent/src/translate/hermes.rs` | HermesXmlTranslator |
| `crates/roko-agent/src/translate/react.rs` | ReActTranslator |
| `crates/roko-agent/src/translate/capability.rs` | ModelCapabilities, translator_for, capabilities_from_profile |

---

## Citations

1. Lee, S. Y. et al. (2026). "Meta-Harness: Harness Engineering for LLM
   Agents." arXiv:2603.28052. -- Principle 1: tools for the model.
2. `crates/roko-agent/src/translate/mod.rs` -- Translator trait, wire
   format enums, BackendResponse, FinishReason normalization.
3. `crates/roko-agent/src/translate/capability.rs` -- ModelCapabilities,
   translator selection functions.
