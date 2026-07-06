# 21 — Gemini: First-Class Google AI Backend

> **Priority**: 🟡 P1 — 1M context, cheapest reasoning, free tier, grounding, code execution
> **Status**: Not started
> **Depends on**: 02 (provider registry), 03 (provider adapters)
> **Blocks**: None
> **Branch**: TBD

## Problem Statement

Gemini offers capabilities no other provider matches for roko's use cases:

1. **1M token context window** — Gemini 2.5 Pro and 3.x models handle 1M input tokens. This is 5x Claude's context and means entire crate sources can be fed as context without truncation.
2. **Cheapest reasoning at scale** — Gemini 2.5 Flash-Lite at $0.10/M input is 150x cheaper than Claude Opus for mechanical tasks. Gemini 3 Flash at $0.50/M is 30x cheaper.
3. **Free tier** — Gemini 2.5 Flash, 2.5 Flash-Lite, 3 Flash, and 3.1 Flash-Lite are free in the standard tier. This enables unlimited exploration by the CascadeRouter at zero cost.
4. **Grounding with Google Search** — built-in web search with `groundingMetadata` (citations, search queries, source chunks). Complementary to Perplexity for research.
5. **Code execution** — built-in Python sandbox. The model can write + run Python to validate its own work, useful as a self-verification gate.
6. **Native thinking** — configurable reasoning depth (minimal/low/medium/high) per request. CascadeRouter can dial thinking up/down based on task complexity.
7. **OpenAI-compatible endpoint** — `https://generativelanguage.googleapis.com/v1beta/openai/` speaks chat completions, so the `OpenAiCompatAdapter` works for basic use. But grounding, code execution, and thinking require the native API.

The key tension: Gemini has an OpenAI-compat endpoint (works with our existing adapter), but the most valuable features (grounding, code execution, thinking config, context caching) are only available through the native `generateContent` API or via `extra_body` extensions. The spec must handle both paths.

## Models

### Production Models

| Model | Slug | Context | Max Output | Input/M | Output/M | Thinking | Notes |
|---|---|---|---|---|---|---|---|
| Gemini 2.5 Pro | `gemini-2.5-pro` | 1M | 65,536 | $1.25 (≤200K) / $2.50 (>200K) | $10.00 / $15.00 | Yes | Best quality, 1M context |
| Gemini 2.5 Flash | `gemini-2.5-flash` | 1M | 65,536 | $0.30 | $2.50 | Yes | Best price-perf reasoning |
| Gemini 2.5 Flash-Lite | `gemini-2.5-flash-lite` | 1M | 65,536 | $0.10 | $0.40 | No | Cheapest, mechanical tasks |

### Preview Models (Gemini 3.x)

| Model | Slug | Context | Max Output | Input/M | Output/M | Thinking | Notes |
|---|---|---|---|---|---|---|---|
| Gemini 3.1 Pro | `gemini-3.1-pro-preview` | 1M | 64K | $2.00 (≤200K) / $4.00 (>200K) | $12.00 / $18.00 | Dynamic | Most capable, tool combos |
| Gemini 3 Flash | `gemini-3-flash-preview` | 1M | 64K | $0.50 | $3.00 | Dynamic | Frontier perf, low cost |
| Gemini 3.1 Flash-Lite | `gemini-3.1-flash-lite-preview` | 1M | 64K | $0.25 | $1.50 | Dynamic | Budget with dynamic thinking |

### Specialized Models

| Model | Slug | Context | Notes |
|---|---|---|---|
| Computer Use | `gemini-2.5-computer-use-preview` | — | UI automation (future) |
| Deep Research | `gemini-2.5-deep-research-preview` | — | Multi-step autonomous research |
| Embedding 2 | `gemini-embedding-2-preview` | — | Multimodal embeddings |

### Free Tier (Standard)

These models are **free** in the standard (lower-priority) tier:
- `gemini-2.5-flash` — Free input/output
- `gemini-2.5-flash-lite` — Free input/output
- `gemini-3-flash-preview` — Free input/output
- `gemini-3.1-flash-lite-preview` — Free input/output
- `gemini-embedding-2-preview` — Free text input

### Additional Costs

| Feature | Cost |
|---|---|
| Grounding (Google Search) | $14/1K search queries (Gemini 3) |
| Context caching storage | $1.00–$8.10/M tokens/hour (model-dependent) |
| Cached input reads | 75% discount on input token price |
| Code execution | No additional charge |

## Two API Surfaces

### 1. Native Gemini API

- **Endpoint**: `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- **Auth**: `x-goog-api-key: {GEMINI_API_KEY}` header or `?key={GEMINI_API_KEY}` query param
- **Format**: Gemini-native (different from OpenAI)
  - `contents[]` instead of `messages[]`
  - `functionDeclarations[]` instead of `tools[].function`
  - `functionCall` / `functionResponse` instead of `tool_calls` / `tool` role
  - `generationConfig` instead of top-level params
- **Unique features**:
  - `tools: [{google_search: {}}]` — grounding with Google Search
  - `tools: [{code_execution: {}}]` — Python sandbox
  - `tools: [{url_context: {}}]` — URL content extraction
  - `tools: [{google_maps: {}}]` — location grounding
  - Combined built-in + custom tools (Gemini 3)
  - `thinking_config: {thinking_level: "high"}` — reasoning control
  - `cached_content` — context caching reference
  - `safety_settings[]` — per-category harm thresholds
  - `groundingMetadata` in response — search queries, chunks, citations, support spans

### 2. OpenAI-Compatible Endpoint

- **Endpoint**: `POST https://generativelanguage.googleapis.com/v1beta/openai/chat/completions`
- **Auth**: `Authorization: Bearer {GEMINI_API_KEY}`
- **Format**: Standard OpenAI chat completions
- **Supports**: messages, tools (function calling), streaming, structured outputs, vision, embeddings
- **Extensions via `extra_body`**:
  - `cached_content` — context caching
  - `thinking_config` — reasoning
  - `safety_settings` — content filtering
  - `tools: [{google_search: {}}]` — grounding
- **Limitations**: Beta, some features silently ignored, no code execution

### Decision: Dual-Path Adapter

```
GeminiAdapter
├── openai_compat_path()  — for basic chat + function calling
│   Uses existing OpenAiCompatAdapter machinery
│   Works with CascadeRouter as a cheap model option
│
└── native_path()          — for grounding, code execution, thinking, caching
    Custom request/response translation
    Used when model.supports_grounding or model.supports_code_execution
```

The adapter auto-selects the path based on `ModelProfile` flags. Basic tasks (mechanical coding) go through OpenAI-compat (simpler, well-tested). Tasks needing grounding or code execution use the native API.

## Design

### Approach: GeminiApi ProviderKind

Like Perplexity, Gemini's unique features (grounding metadata, code execution results, thinking config, context caching, tiered pricing) warrant a dedicated protocol family:

```rust
pub enum ProviderKind {
    AnthropicApi,
    ClaudeCli,
    OpenAiCompat,
    CursorAcp,
    PerplexityApi,
    GeminiApi,      // NEW
}
```

**Why not just OpenAiCompat?** The OpenAI-compat endpoint works for basic chat, but:
- Grounding metadata (`groundingChunks`, `groundingSupports`, `webSearchQueries`) has no OpenAI equivalent
- Code execution results (`executableCode`, `codeExecutionResult`) are Gemini-specific content parts
- Thinking config (`thinking_level`) maps through `extra_body` but awkwardly
- Context caching requires the native API for cache creation
- Tiered pricing (≤200K vs >200K) needs adapter-level awareness
- The native API's `functionCall.id` tracking (Gemini 3) differs from OpenAI's

The adapter uses OpenAI-compat wire format when possible, native when needed.

### Integration Points

```
┌─────────────────────────────────────────────────────────────────┐
│                         roko-cli                                 │
│                                                                  │
│  orchestrate.rs ──┐                                             │
│                    │   ┌──────────────────────────────────┐     │
│  run.rs ───────────┼──▶│  GeminiAdapter                   │     │
│                    │   │                                  │     │
│  research.rs ──────┘   │  openai_compat()  ← basic chat  │     │
│                        │  native_generate() ← grounding  │     │
│                        │  native_generate() ← code exec  │     │
│                        │  create_cache()    ← caching    │     │
│                        │  embed()           ← embeddings │     │
│                        └──────────────────────────────────┘     │
│                                    │                             │
│                                    ▼                             │
│                        ┌──────────────────────────────────┐     │
│                        │  ChatResponse +                  │     │
│                        │  GeminiMetadata                   │     │
│                        │    .grounding_metadata            │     │
│                        │    .code_execution_results[]      │     │
│                        │    .thinking_content              │     │
│                        │    .safety_ratings[]              │     │
│                        │    .cached_content_usage          │     │
│                        └──────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

### Where Each Model Fits in the Roko Pipeline

| Roko Use Case | Gemini Model | Why |
|---|---|---|
| Mechanical tasks (fast tier) | `gemini-2.5-flash-lite` | $0.10/M, free tier available, 1M context |
| Standard coding (focused tier) | `gemini-2.5-flash` | $0.30/M, thinking, 1M context |
| Complex coding (integrative) | `gemini-2.5-pro` | $1.25/M, deep reasoning, 1M context |
| Architecture (premium) | `gemini-3.1-pro-preview` | $2.00/M, dynamic thinking, tool combos |
| Research + grounding | `gemini-3-flash-preview` + google_search | Grounded answers with citations |
| Self-verification gate | `gemini-2.5-flash` + code_execution | Model writes + runs validation code |
| Embedding (semantic search) | `gemini-embedding-2-preview` | Free, multimodal |
| CascadeRouter exploration | Free tier models | Zero-cost learning about model quality |
| Whole-crate context | Any 1M model | Feed entire crate source as context |

### Config Schema

```toml
[providers.gemini]
kind = "gemini_api"
base_url = "https://generativelanguage.googleapis.com"
api_key_env = "GEMINI_API_KEY"
timeout_ms = 120000

# ── Production models ──────────────────────────────────
[models.gemini-2-5-pro]
provider = "gemini"
slug = "gemini-2.5-pro"
context_window = 1048576
max_output = 65536
supports_tools = true
supports_thinking = true
supports_grounding = true           # Google Search grounding
supports_code_execution = true      # Python sandbox
supports_vision = true
supports_caching = true             # Context caching
tool_format = "gemini_native"       # or "openai_json" for compat path
cost_input_per_m = 1.25
cost_output_per_m = 10.00
cost_input_per_m_high = 2.50       # >200K context pricing
cost_output_per_m_high = 15.00
cost_cache_read_per_m = 0.125
cost_cache_write_per_m = 0.375

[models.gemini-2-5-flash]
provider = "gemini"
slug = "gemini-2.5-flash"
context_window = 1048576
max_output = 65536
supports_tools = true
supports_thinking = true
supports_grounding = true
supports_code_execution = true
supports_vision = true
supports_caching = true
tool_format = "gemini_native"
cost_input_per_m = 0.30
cost_output_per_m = 2.50
cost_cache_read_per_m = 0.0375

[models.gemini-2-5-flash-lite]
provider = "gemini"
slug = "gemini-2.5-flash-lite"
context_window = 1048576
max_output = 65536
supports_tools = true
supports_thinking = false
supports_grounding = false
supports_code_execution = false
supports_vision = true
supports_caching = true
tool_format = "openai_json"         # simple enough for compat path
cost_input_per_m = 0.10
cost_output_per_m = 0.40

[models.gemini-3-flash]
provider = "gemini"
slug = "gemini-3-flash-preview"
context_window = 1048576
max_output = 65536
supports_tools = true
supports_thinking = true
supports_grounding = true
supports_code_execution = true
supports_vision = true
supports_caching = true
thinking_level = "dynamic"          # Gemini 3 dynamic thinking
tool_format = "gemini_native"
cost_input_per_m = 0.50
cost_output_per_m = 3.00

[models.gemini-3-1-pro]
provider = "gemini"
slug = "gemini-3.1-pro-preview"
context_window = 1048576
max_output = 65536
supports_tools = true
supports_thinking = true
supports_grounding = true
supports_code_execution = true
supports_vision = true
supports_caching = true
thinking_level = "dynamic"
tool_format = "gemini_native"
cost_input_per_m = 2.00
cost_output_per_m = 12.00
cost_input_per_m_high = 4.00
cost_output_per_m_high = 18.00

[models.gemini-embed]
provider = "gemini"
slug = "gemini-embedding-2-preview"
is_embedding_model = true
supports_tools = false
supports_vision = true              # multimodal embeddings
cost_input_per_m = 0.00             # free tier

# ── Gemini-specific config ─────────────────────────────
[gemini]
default_model = "gemini-2-5-flash"
grounding_model = "gemini-3-flash"         # model for grounded search
code_exec_model = "gemini-2-5-flash"       # model for code execution gates
embed_model = "gemini-embed"
use_free_tier = true                        # prefer free tier for exploration
thinking_level = "medium"                   # default thinking level
enable_context_caching = true               # cache large contexts

# ── Tier assignments ───────────────────────────────────
[agent.tier_models]
mechanical = "gemini-2-5-flash-lite"       # $0.10/M — 150x cheaper than Opus
focused = "gemini-2-5-flash"               # $0.30/M — thinking enabled
integrative = "gemini-2-5-pro"             # $1.25/M — deep reasoning
architectural = "gemini-3-1-pro"           # $2.00/M — full capabilities
```

---

## Checklist

### Phase A: Core Adapter (depends on 02, 03)

#### 2R.01 — Add GeminiApi variant to ProviderKind

**File**: `crates/roko-core/src/agent.rs`
**What**: Add `GeminiApi` to the `ProviderKind` enum:

```rust
pub enum ProviderKind {
    AnthropicApi,
    ClaudeCli,
    OpenAiCompat,
    CursorAcp,
    PerplexityApi,
    GeminiApi,      // NEW
}
```

Update `label()`, `Display`, and `From<AgentBackend>`.

**Acceptance**: `ProviderKind::GeminiApi` compiles with Serialize/Deserialize.
**Verification**: `cargo test -p roko-core -- provider_kind`

---

#### 2R.02 — Add Gemini-specific fields to ModelProfile

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Extend `ModelProfile` with Gemini-specific capability flags:

```rust
pub struct ModelProfile {
    // ... existing fields ...
    #[serde(default)]
    pub supports_grounding: bool,            // Google Search grounding
    #[serde(default)]
    pub supports_code_execution: bool,       // Python sandbox
    #[serde(default)]
    pub supports_caching: bool,              // context caching
    pub thinking_level: Option<String>,      // "minimal"/"low"/"medium"/"high"/"dynamic"
    pub cost_input_per_m_high: Option<f64>,  // >200K context tier
    pub cost_output_per_m_high: Option<f64>,
    pub cost_cache_write_per_m: Option<f64>,
}
```

**Context**: `cost_input_per_m_high` / `cost_output_per_m_high` handle Gemini's tiered pricing — when context exceeds 200K tokens, the entire request bills at the higher rate. The CostTable needs both tiers for accurate estimates.

**Acceptance**: Existing configs still parse. New fields default correctly.
**Verification**: `cargo test -p roko-core -- model_profile`

---

#### 2R.03 — Add GeminiConfig section to RokoConfig

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add optional `[gemini]` config section:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub default_model: Option<String>,
    pub grounding_model: Option<String>,
    pub code_exec_model: Option<String>,
    pub embed_model: Option<String>,
    #[serde(default)]
    pub use_free_tier: bool,
    #[serde(default = "default_thinking_medium")]
    pub thinking_level: String,
    #[serde(default)]
    pub enable_context_caching: bool,
    #[serde(default)]
    pub safety_settings: Vec<SafetySetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    pub category: String,       // "HARM_CATEGORY_HATE_SPEECH", etc.
    pub threshold: String,      // "BLOCK_NONE", "BLOCK_LOW_AND_ABOVE", etc.
}
```

Add to `RokoConfig`:
```rust
#[serde(default)]
pub gemini: GeminiConfig,
```

**Acceptance**: Existing `roko.toml` still parses. New config with `[gemini]` loads.
**Verification**: `cargo test -p roko-core -- gemini_config`

---

#### 2R.04 — Define Gemini-native types

**File**: `crates/roko-agent/src/gemini/types.rs` (new)
**What**: Types for Gemini's native API:

```rust
// ── Request types ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySettingRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,                        // "user", "model", "function"
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCallPart,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponsePart,
    },
    ExecutableCode {
        #[serde(rename = "executableCode")]
        executable_code: ExecutableCodePart,
    },
    CodeExecutionResult {
        #[serde(rename = "codeExecutionResult")]
        code_execution_result: CodeExecutionResultPart,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineDataPart,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallPart {
    pub name: String,
    pub args: serde_json::Value,
    pub id: Option<String>,                  // Gemini 3: unique call ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponsePart {
    pub name: String,
    pub response: serde_json::Value,
    pub id: Option<String>,                  // must match call ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableCodePart {
    pub language: String,                    // "PYTHON"
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionResultPart {
    pub outcome: String,                     // "OUTCOME_OK", "OUTCOME_ERROR"
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineDataPart {
    pub mime_type: String,
    pub data: String,                        // base64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeminiTool {
    FunctionDeclarations {
        #[serde(rename = "functionDeclarations")]
        function_declarations: Vec<FunctionDeclaration>,
    },
    GoogleSearch {
        google_search: serde_json::Value,    // {} to enable
    },
    CodeExecution {
        code_execution: serde_json::Value,   // {} to enable
    },
    UrlContext {
        url_context: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,       // JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    pub function_calling_config: FunctionCallingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallingConfig {
    pub mode: String,                        // "AUTO", "ANY", "NONE", "VALIDATED"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    pub thinking_level: String,              // "MINIMAL", "LOW", "MEDIUM", "HIGH"
}

// ── Response types ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<String>,       // "STOP", "MAX_TOKENS", "SAFETY", etc.
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
    #[serde(default)]
    pub grounding_metadata: Option<GroundingMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadata {
    /// Search queries the model executed.
    pub web_search_queries: Option<Vec<String>>,
    /// Source chunks from search results.
    pub grounding_chunks: Option<Vec<GroundingChunk>>,
    /// Maps response text spans to source chunks (inline citations).
    pub grounding_supports: Option<Vec<GroundingSupport>>,
    /// HTML/CSS for rendering search suggestions (ToS requirement).
    pub search_entry_point: Option<SearchEntryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingChunk {
    pub web: Option<WebChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChunk {
    pub uri: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupport {
    pub segment: TextSegment,
    pub grounding_chunk_indices: Vec<usize>,
    pub confidence_scores: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSegment {
    pub start_index: usize,
    pub end_index: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntryPoint {
    pub rendered_content: String,            // HTML + CSS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: u64,
    pub candidates_token_count: Option<u64>,
    pub total_token_count: u64,
    pub cached_content_token_count: Option<u64>,
    pub thinking_token_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}

// ── Adapter metadata ──────────────────────────────────

/// Gemini-specific metadata preserved in ChatResponse.metadata.extra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiMetadata {
    pub grounding_metadata: Option<GroundingMetadata>,
    pub code_execution_results: Vec<CodeExecutionResultPart>,
    pub thinking_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub safety_ratings: Vec<SafetyRating>,
}
```

**Acceptance**: All types compile with Serialize/Deserialize. Round-trip test for example Gemini response JSON.
**Verification**: `cargo test -p roko-agent -- gemini_types`

---

#### 2R.05 — Implement GeminiAdapter

**File**: `crates/roko-agent/src/gemini/adapter.rs` (new)
**What**: Core adapter implementing `ProviderAdapter`:

```rust
pub struct GeminiAdapter;

impl ProviderAdapter for GeminiAdapter {
    fn kind(&self) -> ProviderKind { ProviderKind::GeminiApi }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        let api_key = provider.resolve_api_key()
            .ok_or_else(|| AgentCreationError::MissingApiKey(
                provider.api_key_env.clone().unwrap_or_else(|| "GEMINI_API_KEY".into())
            ))?;

        let base_url = provider.base_url.clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());

        if model.is_embedding_model {
            return Ok(Box::new(GeminiEmbedAgent::new(api_key, base_url, model.slug.clone())));
        }

        // Determine which path to use based on required features
        let needs_native = model.supports_grounding
            || model.supports_code_execution
            || model.thinking_level.as_deref() == Some("dynamic");

        if needs_native {
            Ok(Box::new(GeminiNativeAgent::new(
                api_key, base_url, model.clone(), options,
            )))
        } else {
            // OpenAI-compat path for simple models (flash-lite, etc.)
            Ok(Box::new(GeminiCompatAgent::new(
                api_key, base_url, model.clone(), options,
            )))
        }
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        match status {
            429 => ProviderError::RateLimit {
                retry_after_ms: body.pointer("/error/details/0/retryDelay")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.trim_end_matches('s').parse::<f64>().ok())
                    .map(|s| (s * 1000.0) as u64)
            },
            401 | 403 => ProviderError::AuthFailure,
            404 => ProviderError::ModelNotFound,
            400 => {
                // Gemini returns 400 for context overflow
                let msg = body.pointer("/error/message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if msg.contains("exceeds the maximum") || msg.contains("token limit") {
                    ProviderError::ContextOverflow
                } else {
                    ProviderError::Other(format!("Bad request: {msg}"))
                }
            },
            500..=599 => ProviderError::ServerError(status),
            _ => ProviderError::Other(format!("HTTP {status}")),
        }
    }
}
```

**Acceptance**: Adapter returns correct agent type based on model capabilities.
**Verification**: `cargo test -p roko-agent -- gemini_adapter`

---

#### 2R.06 — Implement GeminiNativeAgent (generateContent + extensions)

**File**: `crates/roko-agent/src/gemini/native.rs` (new)
**What**: Agent that calls the native `generateContent` endpoint with full Gemini feature support:

Key responsibilities:
1. **Translate ChatMessage → Content**: Map OpenAI-style messages to Gemini `contents[]` format
2. **Translate ToolDef → FunctionDeclaration**: Map tool definitions
3. **Inject built-in tools**: Add `google_search`, `code_execution` based on model profile + request options
4. **Set thinking config**: Map `thinking_level` from config/request
5. **Parse response**: Extract text, function calls, code execution results, grounding metadata
6. **Preserve metadata**: Store `GroundingMetadata`, `CodeExecutionResultPart[]`, thinking tokens in `ChatResponse.metadata.extra`

```rust
pub struct GeminiNativeAgent {
    api_key: String,
    base_url: String,
    model: ModelProfile,
    thinking_level: Option<String>,
    enable_grounding: bool,
    enable_code_execution: bool,
    safety_settings: Vec<SafetySettingRequest>,
    system_prompt: Option<String>,
    timeout_ms: u64,
}

impl GeminiNativeAgent {
    fn endpoint(&self) -> String {
        format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model.slug
        )
    }

    fn translate_messages(&self, messages: &[ChatMessage]) -> Vec<Content> {
        // user → Content { role: "user", parts: [Text { text }] }
        // assistant → Content { role: "model", parts: [Text { text }] }
        // tool result → Content { role: "function", parts: [FunctionResponse { ... }] }
        // system → extracted to system_instruction (not in contents)
    }

    fn translate_tools(&self, tools: &[ToolDef]) -> Vec<GeminiTool> {
        let mut gemini_tools = vec![];

        // Custom function declarations
        if !tools.is_empty() {
            let declarations = tools.iter().map(|t| FunctionDeclaration {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            }).collect();
            gemini_tools.push(GeminiTool::FunctionDeclarations {
                function_declarations: declarations,
            });
        }

        // Built-in tools
        if self.enable_grounding {
            gemini_tools.push(GeminiTool::GoogleSearch {
                google_search: json!({}),
            });
        }
        if self.enable_code_execution {
            gemini_tools.push(GeminiTool::CodeExecution {
                code_execution: json!({}),
            });
        }

        gemini_tools
    }

    fn parse_response(&self, resp: &GenerateContentResponse) -> ChatResponse {
        let candidate = &resp.candidates[0];
        let mut text_parts = vec![];
        let mut tool_calls = vec![];
        let mut code_results = vec![];

        for part in &candidate.content.parts {
            match part {
                Part::Text { text } => text_parts.push(text.clone()),
                Part::FunctionCall { function_call } => {
                    tool_calls.push(ToolCall {
                        id: function_call.id.clone()
                            .unwrap_or_else(|| format!("call_{}", tool_calls.len())),
                        name: function_call.name.clone(),
                        arguments: function_call.args.to_string(),
                    });
                },
                Part::CodeExecutionResult { code_execution_result } => {
                    code_results.push(code_execution_result.clone());
                },
                _ => {}
            }
        }

        let meta = GeminiMetadata {
            grounding_metadata: candidate.grounding_metadata.clone(),
            code_execution_results: code_results,
            thinking_tokens: resp.usage_metadata.as_ref()
                .and_then(|u| u.thinking_token_count),
            cached_tokens: resp.usage_metadata.as_ref()
                .and_then(|u| u.cached_content_token_count),
            safety_ratings: candidate.safety_ratings.clone(),
        };

        ChatResponse {
            content: text_parts.join(""),
            reasoning: None,
            tool_calls,
            usage: /* from usage_metadata */,
            finish_reason: /* from candidate.finish_reason */,
            metadata: ResponseMetadata {
                extra: Some(serde_json::to_value(&meta).unwrap_or_default()),
                ..Default::default()
            },
        }
    }
}
```

**Acceptance**: Native agent sends correct `generateContent` request, parses grounding metadata and code execution results.
**Verification**: `cargo test -p roko-agent -- gemini_native_agent`

---

#### 2R.07 — Implement GeminiCompatAgent (OpenAI-compat path)

**File**: `crates/roko-agent/src/gemini/compat.rs` (new)
**What**: Thin wrapper that uses the OpenAI-compatible endpoint for simple models (flash-lite, etc.):

```rust
pub struct GeminiCompatAgent {
    inner: CodexAgent,  // reuse existing OpenAI-compat agent
}

impl GeminiCompatAgent {
    pub fn new(api_key: String, base_url: String, model: ModelProfile, options: &AgentOptions) -> Self {
        let compat_url = format!("{}/v1beta/openai", base_url);
        let inner = CodexAgent::new(api_key, model.slug)
            .with_base_url(compat_url)
            .with_timeout_ms(options.timeout_ms.unwrap_or(120_000))
            .with_name(options.name.clone());
        Self { inner }
    }
}

impl Agent for GeminiCompatAgent {
    async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult {
        self.inner.run(input, ctx).await
    }
}
```

**Context**: For models like `gemini-2.5-flash-lite` that don't need grounding or code execution, the OpenAI-compat path is simpler and avoids the native request/response translation.

**Acceptance**: Flash-Lite model works through OpenAI-compat endpoint.
**Verification**: `cargo test -p roko-agent -- gemini_compat_agent`

---

#### 2R.08 — Implement Gemini Translator (native ↔ canonical)

**File**: `crates/roko-agent/src/translate/gemini.rs` (new)
**What**: Translator implementation for Gemini's native tool call format:

```rust
pub struct GeminiTranslator;

impl Translator for GeminiTranslator {
    fn parse_calls(&self, content: &str, raw: &Value) -> Vec<ToolCall> {
        // Parse functionCall parts from Gemini response
        // Handle Gemini 3 id field
    }

    fn render_results(&self, results: &[ToolResult]) -> RenderedResults {
        // Render as functionResponse parts for next turn
        // Include matching id for Gemini 3
    }
}
```

**Context**: Gemini's function calling format differs from OpenAI:
- Request: `functionDeclarations[]` instead of `tools[].function`
- Response: `functionCall { name, args, id }` instead of `tool_calls[].function`
- Result: `functionResponse { name, response, id }` instead of `tool` role message
- Gemini 3 requires matching `id` fields between call and response

**Acceptance**: Tools round-trip correctly through the Gemini translator.
**Verification**: `cargo test -p roko-agent -- gemini_translator`

---

#### 2R.09 — Implement GeminiEmbedAgent

**File**: `crates/roko-agent/src/gemini/embed.rs` (new)
**What**: Embedding agent using Gemini's embedding models:

```rust
pub struct GeminiEmbedAgent {
    api_key: String,
    base_url: String,
    model_slug: String,
}

impl GeminiEmbedAgent {
    pub async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // POST /v1beta/models/{model}:batchEmbedContents
        // or via OpenAI-compat: POST /v1beta/openai/embeddings
    }
}
```

**Context**: `gemini-embedding-2-preview` is free and supports multimodal input (text + images). Useful for indexing research artifacts and code.

**Acceptance**: Embedding returns float vectors.
**Verification**: `cargo test -p roko-agent -- gemini_embed` (mock)

---

#### 2R.10 — Register GeminiAdapter in adapter_for_kind()

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: Add the new adapter:

```rust
pub fn adapter_for_kind(kind: ProviderKind) -> &'static dyn ProviderAdapter {
    match kind {
        ProviderKind::OpenAiCompat => &OpenAiCompatAdapter,
        ProviderKind::ClaudeCli => &ClaudeCliAdapter,
        ProviderKind::AnthropicApi => &AnthropicApiAdapter,
        ProviderKind::CursorAcp => &CursorAcpAdapter,
        ProviderKind::PerplexityApi => &PerplexityAdapter,
        ProviderKind::GeminiApi => &GeminiAdapter,           // NEW
    }
}
```

**Acceptance**: `adapter_for_kind(ProviderKind::GeminiApi)` returns the Gemini adapter.
**Verification**: `cargo test -p roko-agent -- adapter_for_kind`

---

### Phase B: Grounding & Code Execution

#### 2R.11 — Wire Google Search grounding into research pipeline

**File**: `crates/roko-cli/src/research.rs`
**What**: When a Gemini grounding model is configured, use it for research with Google Search:

```rust
pub fn build_research_prompt_gemini(
    workdir: &Path,
    topic: &str,
    mode: ResearchMode,
    gemini_config: &GeminiConfig,
) -> (String, bool) {
    let prompt = build_research_prompt(workdir, topic, "", mode);
    let enable_grounding = gemini_config.grounding_model.is_some();
    (prompt, enable_grounding)
}
```

The agent runs with `google_search` tool enabled. Grounding metadata from the response is post-processed into citation markdown (same format as Perplexity integration).

**Acceptance**: Research with grounding produces responses with `groundingChunks` and `groundingSupports`.
**Verification**: `cargo test -p roko-cli -- research_gemini_grounding`

---

#### 2R.12 — Extract grounding metadata into research citations

**File**: `crates/roko-cli/src/research.rs`
**What**: Convert `GroundingMetadata` into the same markdown citation format as Perplexity:

```rust
pub fn grounding_to_citations(meta: &GroundingMetadata) -> Vec<(String, String)> {
    // (title, url) pairs from groundingChunks
    meta.grounding_chunks.as_ref()
        .map(|chunks| chunks.iter()
            .filter_map(|c| c.web.as_ref())
            .map(|w| (w.title.clone(), w.uri.clone()))
            .collect()
        )
        .unwrap_or_default()
}

pub fn grounding_to_inline_citations(
    text: &str,
    meta: &GroundingMetadata,
) -> String {
    // Replace text spans with [N] citation markers using groundingSupports
}
```

**Context**: Gemini's `groundingSupports` provides character-level source attribution similar to Perplexity's `annotations`. Both get normalized into the same `## Sources` markdown format.

**Acceptance**: Grounding metadata converts to numbered citations with working URLs.
**Verification**: `cargo test -p roko-cli -- grounding_to_citations`

---

#### 2R.13 — Implement code execution gate

**File**: `crates/roko-gate/src/code_exec.rs` (new)
**What**: A gate that uses Gemini's code execution to verify agent output:

```rust
pub struct CodeExecutionGate {
    agent: GeminiNativeAgent,  // configured with code_execution enabled
}

impl Gate for CodeExecutionGate {
    async fn check(&self, output: &Signal, task: &Task) -> GateResult {
        let prompt = format!(
            "Verify this code change is correct by writing and running Python tests:\n\n\
             Task: {}\n\n\
             Changes:\n```\n{}\n```\n\n\
             Write Python code that validates the changes are logically correct.\
             Focus on edge cases and invariants.",
            task.description, output.text()
        );

        let result = self.agent.run_with_code_execution(&prompt).await?;

        // Check if code execution succeeded
        for code_result in &result.metadata.code_execution_results {
            if code_result.outcome == "OUTCOME_ERROR" {
                return GateResult::Fail(format!(
                    "Code execution validation failed: {}", code_result.output
                ));
            }
        }
        GateResult::Pass
    }
}
```

**Context**: This is a novel gate type — the model writes Python to validate the output, runs it in Gemini's sandbox, and the gate passes/fails based on execution. No local code execution needed. Complements existing compile/test/clippy gates.

**Acceptance**: Gate passes when validation code succeeds, fails when it doesn't.
**Verification**: `cargo test -p roko-gate -- code_exec_gate` (mock)

---

### Phase C: Context Caching & Cost Optimization

#### 2R.14 — Implement context caching for large contexts

**File**: `crates/roko-agent/src/gemini/cache.rs` (new)
**What**: Client for Gemini's context caching API:

```rust
pub struct GeminiCacheClient {
    api_key: String,
    base_url: String,
}

impl GeminiCacheClient {
    /// Create a cache entry for reusable context (e.g., entire crate source).
    pub async fn create_cache(
        &self,
        model: &str,
        contents: &[Content],
        ttl_seconds: u64,
    ) -> Result<String> {
        // POST /v1beta/cachedContents
        // Returns cache ID for use in subsequent requests
    }

    /// Delete a cache entry.
    pub async fn delete_cache(&self, cache_id: &str) -> Result<()> {
        // DELETE /v1beta/cachedContents/{id}
    }
}
```

**Context**: Context caching is key for cost efficiency with Gemini's 1M context. When running multiple tasks against the same crate, cache the crate source once and reference it in subsequent requests. Cached reads are 75% cheaper than fresh input.

Use pattern:
1. Before a plan run, cache the target crate's full source
2. Each task references the cache (saves 75% on input tokens)
3. After plan completes, delete cache

**Acceptance**: Cache creation returns an ID. Requests with cached_content reference work.
**Verification**: `cargo test -p roko-agent -- gemini_cache` (mock)

---

#### 2R.15 — Wire context caching into orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: When Gemini is the provider and `enable_context_caching` is true, create a cache before plan execution:

```rust
// Before plan loop
let cache_id = if config.gemini.enable_context_caching && is_gemini_provider(model) {
    let crate_source = read_full_crate_source(task.crate_path)?;
    let cache = gemini_cache.create_cache(
        &model.slug,
        &[Content { role: "user".into(), parts: vec![Part::Text { text: crate_source }] }],
        3600, // 1 hour TTL
    ).await.ok();
    cache
} else {
    None
};

// In task dispatch, pass cached_content to agent
```

**Acceptance**: Crate source is cached once, referenced by all tasks in the plan.
**Verification**: Manual test measuring cost savings.

---

#### 2R.16 — Implement tiered pricing in CostTable

**File**: `crates/roko-learn/src/costs_db.rs`
**What**: Handle Gemini's tiered pricing (≤200K vs >200K):

```rust
pub struct ModelCost {
    pub input_per_m: f64,
    pub output_per_m: f64,
    pub input_per_m_high: Option<f64>,      // NEW: >200K context tier
    pub output_per_m_high: Option<f64>,
    pub per_request: Option<f64>,
    // ... existing fields ...
}

impl ModelCost {
    pub fn estimate_total(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_rate = if input_tokens > 200_000 {
            self.input_per_m_high.unwrap_or(self.input_per_m)
        } else {
            self.input_per_m
        };
        let output_rate = if input_tokens > 200_000 {
            self.output_per_m_high.unwrap_or(self.output_per_m)
        } else {
            self.output_per_m
        };

        (input_tokens as f64 / 1_000_000.0) * input_rate
            + (output_tokens as f64 / 1_000_000.0) * output_rate
            + self.per_request.unwrap_or(0.0)
    }
}
```

**Acceptance**: Cost for 300K-token Gemini request uses high-tier pricing.
**Verification**: `cargo test -p roko-learn -- gemini_tiered_cost`

---

### Phase D: CascadeRouter Integration

#### 2R.17 — Add Gemini models to CascadeRouter static table

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Add Gemini models as routing targets per tier:

```rust
// Static tier assignments (Stage 1)
// Mechanical (Fast) → gemini-2.5-flash-lite ($0.10/M)
// Focused (Standard) → gemini-2.5-flash ($0.30/M)
// Integrative (Complex) → gemini-2.5-pro ($1.25/M)
// Architectural (Premium) → gemini-3.1-pro-preview ($2.00/M)
```

**Context**: Gemini's pricing makes it the cheapest option at every tier except premium (where Claude Opus is preferred for code quality). The CascadeRouter can learn the quality/cost tradeoff by running tasks through Gemini and Claude in parallel during the exploration phase.

**Acceptance**: CascadeRouter routes to Gemini models when configured.
**Verification**: `cargo test -p roko-learn -- cascade_gemini`

---

#### 2R.18 — Add thinking level to RoutingContext

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Extend the routing context to include requested thinking level:

```rust
pub struct RoutingContext {
    // ... existing 7 features ...
    pub thinking_level: Option<String>,  // NEW: affects model selection
}
```

When the task is complex and thinking is requested at "high", the router should prefer models with `supports_thinking = true`. When thinking is "minimal" or disabled, cheaper non-thinking models are viable.

**Acceptance**: Thinking level influences routing decisions.
**Verification**: `cargo test -p roko-learn -- routing_context_thinking`

---

#### 2R.19 — Free tier exploration mode for CascadeRouter

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: When `gemini.use_free_tier = true`, the CascadeRouter can run shadow requests through free-tier Gemini models to gather quality observations at zero cost:

```rust
impl CascadeRouter {
    /// Run a shadow evaluation: send the same prompt to a free-tier model,
    /// compare output quality to the primary model, record observation.
    pub async fn shadow_evaluate(
        &mut self,
        prompt: &str,
        primary_result: &AgentResult,
        free_model: &str,
    ) {
        // 1. Send prompt to free-tier Gemini model
        // 2. Run same gate pipeline on the result
        // 3. Record observation (pass/fail, cost=0)
        // 4. If free model passes, router learns it's viable for this task type
    }
}
```

**Context**: This is how the router learns about Gemini quality without risking task failure. Shadow evaluation runs in parallel, doesn't affect the task outcome, and costs nothing with free-tier models.

**Acceptance**: Shadow evaluations record observations. Router shifts toward cheaper models when they pass.
**Verification**: `cargo test -p roko-learn -- shadow_evaluate`

---

### Phase E: Production & Testing

#### 2R.20 — Write example config: roko-gemini.toml

**File**: `examples/roko-gemini.toml`
**What**: Complete example showing Gemini as primary backend:

```toml
# Gemini as primary backend — cheapest option with 1M context
[providers.gemini]
kind = "gemini_api"
base_url = "https://generativelanguage.googleapis.com"
api_key_env = "GEMINI_API_KEY"

[models.gemini-2-5-flash-lite]
provider = "gemini"
slug = "gemini-2.5-flash-lite"
context_window = 1048576
max_output = 65536
supports_tools = true
cost_input_per_m = 0.10
cost_output_per_m = 0.40

# ... (all models) ...

[gemini]
default_model = "gemini-2-5-flash"
use_free_tier = true
thinking_level = "medium"
enable_context_caching = true

[agent]
default_model = "gemini-2-5-flash"
[agent.tier_models]
mechanical = "gemini-2-5-flash-lite"
focused = "gemini-2-5-flash"
integrative = "gemini-2-5-pro"
architectural = "gemini-3-1-pro"
```

**Acceptance**: Config parses, all models resolve correctly.
**Verification**: `cargo test -p roko-core -- gemini_example_config`

---

#### 2R.21 — Write example config: roko-multi-provider.toml

**File**: `examples/roko-multi-provider.toml`
**What**: Example combining Claude (coding), Gemini (cheap reasoning + grounding), and Perplexity (research):

```toml
# Multi-provider: Claude for architecture, Gemini for coding, Perplexity for research
[providers.claude_cli]
kind = "claude_cli"
command = "claude"
args = ["--print", "--output-format", "stream-json"]

[providers.gemini]
kind = "gemini_api"
api_key_env = "GEMINI_API_KEY"

[providers.perplexity]
kind = "perplexity_api"
api_key_env = "PERPLEXITY_API_KEY"

[agent.tier_models]
mechanical = "gemini-2-5-flash-lite"     # $0.10/M — 150x cheaper than Opus
focused = "gemini-2-5-flash"              # $0.30/M
integrative = "claude-opus"               # $15/M — best code quality
architectural = "claude-opus"             # $15/M

[agent.role_overrides.researcher]
model = "sonar-pro"                       # Perplexity for research

[agent.role_overrides.fact_checker]
model = "sonar"                           # Perplexity for fact-checking
```

**Acceptance**: Three providers configured, router assigns appropriate models per tier/role.
**Verification**: `cargo test -p roko-core -- multi_provider_config`

---

#### 2R.22 — Write mock integration tests

**File**: `crates/roko-agent/tests/gemini_integration.rs` (new)
**What**: Comprehensive mock tests:

1. Native generateContent with function calling
2. Native generateContent with Google Search grounding (parse groundingMetadata)
3. Native generateContent with code execution (parse executableCode + codeExecutionResult)
4. Native generateContent with thinking (parse thinking_token_count)
5. OpenAI-compat path for flash-lite
6. Error classification (429 with retryDelay, 400 context overflow, 403 auth)
7. Tiered pricing calculation (under and over 200K)
8. Gemini 3 function call ID round-trip

**Acceptance**: All 8 test cases pass with mocked HTTP responses.
**Verification**: `cargo test -p roko-agent -- gemini_`

---

#### 2R.23 — Add Gemini health + quality tracking

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Record Gemini-specific observations:

- Thinking token count (efficiency signal — more thinking tokens = harder problem)
- Cached token usage (cost savings tracking)
- Grounding query count (Google Search cost tracking: $14/1K queries)
- Code execution success/failure rate
- Context tier used (≤200K vs >200K for accurate cost tracking)

**Acceptance**: Observations include Gemini-specific metrics.
**Verification**: `cargo test -p roko-learn -- gemini_observations`

---

#### 2R.24 — Update lib.rs exports and module structure

**File**: `crates/roko-agent/src/lib.rs`
**What**: Add `pub mod gemini;` and re-export key types:

```rust
pub mod gemini;
pub use gemini::{
    GeminiNativeAgent,
    GeminiCompatAgent,
    GeminiEmbedAgent,
    GeminiMetadata,
    GroundingMetadata,
    GenerateContentRequest,
    GenerateContentResponse,
};
```

**Acceptance**: `use roko_agent::gemini::GeminiNativeAgent;` compiles from roko-cli.
**Verification**: `cargo check -p roko-cli`

---

## Execution Order

**Phase A** (core, sequential, depends on 02 + 03):
1. 2R.01 — ProviderKind variant
2. 2R.02 — ModelProfile extensions
3. 2R.03 — GeminiConfig
4. 2R.04 — Type definitions
5. 2R.05 — GeminiAdapter
6. 2R.06 — GeminiNativeAgent
7. 2R.07 — GeminiCompatAgent
8. 2R.08 — GeminiTranslator
9. 2R.09 — GeminiEmbedAgent
10. 2R.10 — Register in adapter_for_kind

**Phase B** (grounding + code exec, depends on Phase A):
11. 2R.11 — Grounding in research pipeline
12. 2R.12 — Grounding → citation extraction
13. 2R.13 — Code execution gate

**Phase C** (cost optimization, depends on Phase A):
14. 2R.14 — Context caching client
15. 2R.15 — Wire caching into orchestrate.rs
16. 2R.16 — Tiered pricing model

**Phase D** (routing, parallelizable with B + C):
17. 2R.17 — CascadeRouter static table
18. 2R.18 — Thinking level in RoutingContext
19. 2R.19 — Free tier shadow evaluation

**Phase E** (testing + docs, anytime after Phase A):
20. 2R.20 — Gemini example config
21. 2R.21 — Multi-provider example config
22. 2R.22 — Mock integration tests
23. 2R.23 — Health + quality tracking
24. 2R.24 — Module exports

---

## Why Dedicated Adapter (Not Just OpenAI Compat)

The OpenAI-compat endpoint at `/v1beta/openai/` works for basic chat but would lose:

- **Grounding metadata** — `groundingChunks`, `groundingSupports`, `webSearchQueries` have no OpenAI equivalent. This is Google Search citations — comparable to Perplexity but with different structure.
- **Code execution** — `executableCode` + `codeExecutionResult` parts are Gemini-specific. The model writes and runs Python in a sandbox. No OpenAI equivalent.
- **Thinking config** — `thinking_level` (minimal/low/medium/high/dynamic) is available via `extra_body` but the response `thinking_token_count` isn't in the OpenAI response.
- **Context caching** — native API only for cache creation; OpenAI-compat can reference caches via `extra_body` but can't create them.
- **Tiered pricing** — cost changes at 200K tokens. The adapter needs to know which tier applies for accurate cost tracking.
- **Native tool format** — `functionDeclarations` + `functionCall.id` (Gemini 3) differs from OpenAI format. The Translator must handle this for the native path.

The dual-path approach (compat for simple, native for features) gives the best of both worlds.

## Also Available via OpenRouter

Gemini models are on OpenRouter (`google/gemini-2.5-pro`, `google/gemini-2.5-flash`, etc.) through the OpenAI-compat format. This works with the existing `OpenAiCompatAdapter` but loses grounding, code execution, and caching. For cost-only CascadeRouter exploration, OpenRouter is fine.

```toml
# OpenRouter path (simple, no grounding/code exec)
[models.gemini-flash-or]
provider = "openrouter"
slug = "google/gemini-2.5-flash"
supports_grounding = false
supports_code_execution = false

# Direct path (full features)
[models.gemini-2-5-flash]
provider = "gemini"
slug = "gemini-2.5-flash"
supports_grounding = true
supports_code_execution = true
```
