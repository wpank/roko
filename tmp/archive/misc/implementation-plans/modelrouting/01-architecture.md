# 01 — Architecture & Design: Extensible Multi-Provider Agent Backends

> **Type**: Reference document (no tasks — design rationale and decisions)
> **Audience**: Any agent or developer working on tasks in docs 02–10

## Problem Statement

Roko has 5 hardcoded LLM backends (`AgentBackend` enum in `crates/roko-core/src/agent.rs` line 30) with slug-based routing (`from_model()` at line 68) that maps model names to backends via string prefix matching. This creates several problems:

1. **Adding a new provider requires code changes** in `from_model()`, a new agent struct, and wiring throughout
2. **Same model, different provider** is impossible — `kimi-k2.5` always routes to Cursor (line 79 `is_cursor_slug`)
3. **No provider-specific features** — GLM-5.1's thinking mode, tool streaming, native MCP; Kimi-K2.5's partial continuation, vision — all require adapter-level support that the flat backend enum can't express
4. **No provider health tracking** — the router picks models by quality, blind to whether the provider endpoint is healthy
5. **Cost comparison across providers is broken** — different tokenizers mean $1.40/M tokens on Z.AI and $15/M on Anthropic aren't directly comparable

## Design Principles

1. **Config-driven, not code-driven** — adding a new OpenAI-compatible provider should require only a TOML entry
2. **Protocol families, not per-model agents** — there are 4 protocol families (Anthropic, OpenAI-compat, Claude CLI, Cursor ACP), not N per model
3. **Model-specific features compose via extension points** — thinking mode, vision, web search inject through `extra_params()` rather than subclassing
4. **The existing Translator trait stays** — it's well-designed; extensions happen in the adapter layer above it
5. **CascadeRouter stays** — LinUCB is a strong routing algorithm; improvements are in the signals it receives
6. **Backwards compatible** — existing `roko.toml` with `[agent]` section continues to work; new `[providers.*]` and `[models.*]` are optional

## Three-Layer Architecture

### Layer 1: Provider Registry (config-driven)

Separates **provider** (where requests go) from **model** (what capabilities it has):

```toml
[providers.zai]
kind = "openai_compat"
base_url = "https://api.z.ai/api/paas/v4"
api_key_env = "ZAI_API_KEY"

[models.glm-5-1]
provider = "zai"
slug = "glm-5.1"
context_window = 200000
max_output = 131072
supports_tools = true
supports_thinking = true
tool_format = "openai_json"
cost_input_per_m = 1.40
cost_output_per_m = 4.40
```

**Key insight from genai crate**: Multiple providers share the same protocol — Z.AI, Moonshot, OpenRouter, Ollama all speak `openai_compat` but with different endpoints and auth.

### Layer 2: Provider Adapters (trait-based dispatch)

Four protocol families, each implementing `ProviderAdapter`:

| Protocol Family | Current Backend(s) | Wire Format |
|---|---|---|
| `anthropic_api` | ClaudeAgent (HTTP) | Anthropic Messages API |
| `claude_cli` | ClaudeCliAgent | subprocess + stream-json |
| `openai_compat` | OpenAiAgent, CodexAgent, OllamaAgent | OpenAI chat completions |
| `cursor_acp` | CursorAgent | ACP `/v1/prompt` |

The adapter trait:

```rust
pub trait ProviderAdapter: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn build_request(&self, config: &ProviderConfig, model: &ModelProfile, request: &ChatRequest) -> Result<WireRequest>;
    fn parse_response(&self, raw: &WireResponse, model: &ModelProfile) -> Result<ChatResponse>;
    fn translator(&self, model: &ModelProfile) -> Arc<dyn Translator>;
    fn extra_params(&self, model: &ModelProfile, options: &RequestOptions) -> serde_json::Map<String, Value>;
    fn classify_error(&self, status: u16, body: &Value) -> ProviderError;
}
```

### Layer 3: Model-Specific Extensions

Provider-specific features (GLM thinking, Kimi partial, etc.) are injected via `extra_params()` based on what `ModelProfile` declares the model supports. This keeps the adapter count at 4 while supporting arbitrary model extensions.

## Model Resolution Flow

```
1. User specifies model in roko.toml or task:  "glm-5-1"
                          │
2. Check [models.*] table in config  ──→  found: provider="zai", slug="glm-5.1"
   │                                            tool_format="openai_json", etc.
   │ (not found)
   │
3. Fall back to slug heuristic:
   │  claude-*    → anthropic_api
   │  ollama/*    → openai_compat (Ollama endpoint)
   │  llama*      → openai_compat (Ollama endpoint)
   │  (default)   → openai_compat
   │
4. Look up ProviderConfig for the resolved provider
                          │
5. Select ProviderAdapter by ProviderKind
                          │
6. Build request with model-specific extra_params
```

## Canonical Types

### ChatRequest (internal, provider-agnostic)

```rust
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model_slug: String,
    pub tools: Vec<ToolDef>,
    pub tool_choice: ToolChoice,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub stop: Option<Vec<String>>,
    pub options: RequestOptions,
}

pub struct RequestOptions {
    pub enable_thinking: Option<bool>,
    pub preserve_thinking: Option<bool>,  // GLM clear_thinking=false
    pub enable_tool_streaming: Option<bool>,  // GLM tool_stream
    pub enable_vision: Option<bool>,
    pub cache_key: Option<String>,  // Kimi prompt_cache_key
    pub extra: HashMap<String, Value>,  // provider-specific passthrough
}
```

### ChatResponse (internal, provider-agnostic)

```rust
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,      // thinking/reasoning_content
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
    pub metadata: ResponseMetadata,
}

pub struct ResponseMetadata {
    pub response_id: Option<String>,
    pub model_used: Option<String>,     // actual model (may differ from requested)
    pub cached_tokens: Option<u64>,
    pub content_filter: Option<Value>,  // GLM content_filter array
    pub provider_latency_ms: Option<u64>,
}

pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error(String),
    // Provider-specific mapped to these canonical values
}
```

## How Tool Calls Work End-to-End

```
ChatRequest with tools: [Read, Edit, Bash]
    │
    ├─ ProviderAdapter::build_request()
    │   ├─ OpenAiCompatAdapter:
    │   │   POST /chat/completions
    │   │   tools: [{type: "function", function: {name, description, parameters}}]
    │   │   + extra_params: {thinking: {type: "enabled"}}  (if GLM + thinking)
    │   │
    │   └─ ClaudeCliAdapter:
    │       spawn `claude --tools Read,Edit,Bash --model claude-opus-4-6`
    │
    ├─ Provider returns response
    │
    ├─ ProviderAdapter::parse_response()
    │   ├─ Extracts content, reasoning_content, tool_calls
    │   ├─ Normalizes tool_call IDs (Kimi "functions.Read:0" → canonical)
    │   └─ Extracts usage, finish_reason, metadata
    │
    ├─ Translator::parse_calls()  (existing, unchanged)
    │   └─ Vec<ToolCall>
    │
    ├─ ToolDispatcher::dispatch_batch()  (existing, unchanged)
    │   └─ 7-step pipeline: validate → resolve → filter → authorize → safety → execute → scrub
    │
    ├─ Translator::render_results()  (existing, unchanged)
    │   └─ RenderedResults for next turn
    │
    └─ Next turn with tool results appended to messages
```

## Why This Design

### Why not one agent per model?

Models come and go (GLM-5, GLM-5.1, GLM-5-Turbo, GLM-4.7...). Protocols are stable. There are ~300 models on OpenRouter but only 4 protocol families. Agent-per-model means N implementations to maintain; adapter-per-protocol means 4.

### Why static dispatch (enum + match) over trait objects for adapters?

Following the genai crate pattern: there are only 4 protocol families. This won't grow unboundedly. Static dispatch via `ProviderKind` enum is faster, more explicit, and easier to debug than dynamic dispatch via `Box<dyn ProviderAdapter>`. The `Translator` trait stays dynamic because translators are selected at runtime based on model capabilities.

### Why extra_params() instead of subclassing?

GLM-5.1 needs `thinking`, `tool_stream`, `do_sample`. Kimi needs `thinking`, `partial`, `prompt_cache_key`. Future models will need other params. Rather than creating GlmAdapter, KimiAdapter, etc., a single `OpenAiCompatAdapter` handles all OpenAI-compatible models and injects model-specific parameters via `extra_params()` based on the `ModelProfile`.

### Why OpenAI as canonical format?

Both GLM-5.1 and Kimi-K2.5 speak OpenAI format natively. So do OpenRouter, Together, Fireworks, Groq, DeepInfra, Ollama, vLLM. Anthropic is the only major provider with a different format. Making OpenAI the lingua franca minimizes translation and matches the industry consensus (LiteLLM, Vercel AI SDK, and OpenRouter all do this).

### Why config-driven model→provider binding?

The current `from_model("kimi-k2.5")` routes to Cursor because `is_cursor_slug()` matches `kimi-*`. This is wrong for API access. Config-driven binding means a user can point `kimi-k2.5` at Moonshot's API, OpenRouter, a self-hosted vLLM, or Cursor — just by changing TOML. No code change.

## References

### Existing Code

| Component | Path | Lines |
|---|---|---|
| AgentBackend enum | `crates/roko-core/src/agent.rs` | 30–44 |
| from_model() | `crates/roko-core/src/agent.rs` | 68–79 |
| AgentConfig | `crates/roko-core/src/config/schema.rs` | ~490–560 |
| Translator trait | `crates/roko-agent/src/translate/mod.rs` | 54–81 |
| translator_for() | `crates/roko-agent/src/translate/capability.rs` | 65–78 |
| CascadeRouter | `crates/roko-learn/src/cascade_router.rs` | 191–201 |
| LinUCBRouter | `crates/roko-learn/src/model_router.rs` | 313–319 |
| AgentEfficiencyEvent | `crates/roko-learn/src/efficiency.rs` | 67–145 |
| CostsDb | `crates/roko-learn/src/costs_db.rs` | 122–316 |

### External References

| System | What to learn from it |
|---|---|
| genai (Rust crate) | AdapterKind enum + static dispatch, resolver traits |
| LiteLLM | BaseConfig transform pattern, cooldown logic, routing strategies |
| Vercel AI SDK | LanguageModelV4 specification versioning |
| RouteLLM | Matrix factorization router, data augmentation with 1500 golden labels |
| OpenRouter | Inverse-price-squared weighting, p50/p75/p90/p99 latency tracking |
| Artificial Analysis | OpenAI token normalization, 3:1 input/output blended cost |
