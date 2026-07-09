# 20 — Perplexity Sonar: First-Class Search-Grounded Backend

> **Priority**: 🟡 P1 — Unlocks grounded research, citation-aware reasoning, and deep research jobs
> **Status**: Not started
> **Depends on**: 02 (provider registry), 03 (provider adapters)
> **Blocks**: None
> **Branch**: TBD

## Problem Statement

Roko's research pipeline (`roko research`) relies entirely on the Claude agent's training knowledge — there is no real-time web search. The `RESEARCH_SYSTEM_PROMPT` asks for academic citations but the agent has no search tool to verify they exist. This makes research outputs unreliable: the agent hallucinates paper titles, invents authors, and cannot access anything published after its training cutoff.

Perplexity's Sonar API solves this directly. It provides:
1. **Web-grounded answers** with real citations and source URLs
2. **Academic search mode** for paper discovery
3. **Deep research** for exhaustive, multi-step investigation
4. **Chain-of-thought reasoning** with real-time search
5. **Domain filtering** to restrict or exclude sources
6. **Date filtering** to enforce recency requirements
7. **Structured outputs** (JSON schema) for machine-parseable research
8. **Embedding models** for semantic search over research artifacts

The API is OpenAI-compatible at the chat completions layer, but has significant extensions (citations, search_results, annotations, web_search_options, search filters) that the generic `OpenAiCompatAdapter` would silently drop. A proper integration preserves these.

## What Perplexity Offers

### Models

| Model | Slug | Context | Input/M | Output/M | Per-1K Requests | Best For |
|---|---|---|---|---|---|---|
| Sonar | `sonar` | 127K | $1.00 | $1.00 | $5 (low) – $12 (high) | Fast factual queries, topic summaries |
| Sonar Pro | `sonar-pro` | 200K | $3.00 | $15.00 | $6 (low) – $14 (high) | Multi-step queries, complex research |
| Sonar Pro Search | `sonar-pro-search` | 200K | $3.00 | $15.00 | $6 – $14 | Search-optimized variant of Pro |
| Sonar Reasoning | `sonar-reasoning` | 127K | $1.00 | $5.00 | varies | CoT reasoning + real-time search |
| Sonar Reasoning Pro | `sonar-reasoning-pro` | 128K | $2.00 | $8.00 | varies | Complex analytical reasoning |
| Sonar Deep Research | `sonar-deep-research` | 128K | $2.00 | $8.00 | $5/1K searches | Exhaustive long-form reports (async) |

**Embedding models** (for semantic search):
| Model | Slug | Notes |
|---|---|---|
| Embed V1 4B | `pplx-embed-v1-4b` | 4.96B params, dense retrieval |
| Embed V1 0.6B | `pplx-embed-v1-0.6b` | Lightweight variant |

### Two API Surfaces

Perplexity exposes two distinct APIs:

**1. Chat Completions API** (OpenAI-compatible + extensions)
- Endpoint: `POST https://api.perplexity.ai/chat/completions`
- Standard messages array, tools, streaming
- Extensions: `search_domain_filter`, `search_recency_filter`, `return_images`, `return_related_questions`, `web_search_options`
- Response extensions: `citations[]` (URLs), `search_results[]` ({url, title, content}), `annotations[]` ({start_index, end_index, title, url})

**2. Agent/Responses API** (richer, non-OpenAI format)
- Endpoint: `POST https://api.perplexity.ai/v1/agent`
- `preset` field: `fast-search`, `pro-search`, `deep-research`, `advanced-deep-research`
- Model fallback chains: `models[]` array (up to 5)
- Built-in tools: `web_search`, `fetch_url`
- `instructions` field (system prompt equivalent)
- Async support: `POST /v1/async/sonar` + `GET /v1/async/sonar/{id}` for deep research jobs
- Richer search control: `search_after_date_filter`, `search_before_date_filter`, `last_updated_after_filter`, `last_updated_before_filter`, `user_location`

**3. Search API** (pure search, no generation)
- Endpoint: `POST https://api.perplexity.ai/search`
- Returns structured ranked results with domain/date/region filters
- Multi-query bundling: up to 5 queries per call
- $5 per 1,000 requests, no token costs

**4. Embeddings API**
- Endpoint: `POST https://api.perplexity.ai/v1/embeddings`
- Also: `POST /v1/contextualizedembeddings` for context-aware embeddings

### Response Extensions (unique to Perplexity)

Standard OpenAI responses return `choices[].message.content`. Perplexity adds:

```json
{
  "choices": [{
    "message": {
      "content": "According to [1], the paper by Smith et al. found...",
      "annotations": [
        {
          "start_index": 14,
          "end_index": 17,
          "title": "Smith et al. 2024 - Context Engineering",
          "url": "https://arxiv.org/abs/2401.12345"
        }
      ]
    }
  }],
  "citations": [
    "https://arxiv.org/abs/2401.12345",
    "https://example.com/research"
  ],
  "search_results": [
    {
      "url": "https://arxiv.org/abs/2401.12345",
      "title": "Context Engineering for Large Language Models",
      "content": "We propose a framework for...",
      "date": "2024-01-15",
      "last_updated": "2024-03-01"
    }
  ]
}
```

These fields are **top-level on the response object** (not inside `choices`), except `annotations` which is inside `message`.

## Design

### Approach: PerplexityAdapter (new protocol family)

Perplexity is NOT just another OpenAI-compatible endpoint. The extensions are significant enough to warrant a dedicated adapter that:

1. **Preserves citations** in `ChatResponse.metadata` — the generic adapter would drop them
2. **Injects search parameters** from config + request options
3. **Handles the Agent API** for deep research (different endpoint, async polling)
4. **Maps annotations** to source-linked content for the research pipeline
5. **Routes between APIs** — chat completions for simple queries, agent API for deep research

```
ProviderKind enum:
  AnthropicApi,
  ClaudeCli,
  OpenAiCompat,
  CursorAcp,
+ PerplexityApi,    // NEW — dedicated protocol family
```

This adds a 5th adapter. The tradeoff vs. stuffing everything into `OpenAiCompatAdapter`:
- **Pro dedicated adapter**: citations, search_results, annotations are first-class; agent API support; async deep research; search API; embeddings API
- **Con dedicated adapter**: one more adapter to maintain
- **Decision**: dedicated adapter. The search-grounded features are Perplexity's entire value proposition. Dropping them to fit the OpenAI mold defeats the purpose.

### Integration Points

```
┌─────────────────────────────────────────────────────────────────┐
│                         roko-cli                                 │
│                                                                  │
│  research.rs ─── ResearchMode ──┐                               │
│                                  │   ┌──────────────────────┐   │
│  orchestrate.rs ─── CascadeRouter ──│  PerplexityAdapter    │   │
│                                  │   │                      │   │
│  run.rs ─── one-shot ───────────┘   │  chat_completions()   │   │
│                                      │  agent_response()    │   │
│                                      │  deep_research()     │   │
│                                      │  search()            │   │
│                                      │  embed()             │   │
│                                      └──────────────────────┘   │
│                                              │                   │
│                                              ▼                   │
│                                      ┌──────────────────────┐   │
│                                      │  ChatResponse +      │   │
│                                      │  PerplexityMetadata   │   │
│                                      │    .citations[]       │   │
│                                      │    .search_results[]  │   │
│                                      │    .annotations[]     │   │
│                                      └──────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Where Each Model Fits in the Roko Pipeline

| Roko Use Case | Perplexity Model | Why |
|---|---|---|
| `roko research topic` | `sonar-pro` or `sonar-deep-research` | Needs thorough search + citations |
| `roko research enhance-prd` | `sonar-pro` | Find papers to support/refute PRD claims |
| `roko research enhance-plan` | `sonar-reasoning` | Reason about plan quality with search context |
| `roko research analyze` | `sonar-reasoning-pro` | Complex analytical reasoning over data |
| Fact-checking during gate | `sonar` | Fast, cheap factual verification |
| Research for task context | `sonar` | Quick lookup before agent starts coding |
| Deep architecture research | `sonar-deep-research` (async) | Exhaustive multi-step investigation |
| Embedding research artifacts | `pplx-embed-v1-4b` | Semantic search over `.roko/research/` |

### Config Schema

```toml
[providers.perplexity]
kind = "perplexity_api"
base_url = "https://api.perplexity.ai"
api_key_env = "PERPLEXITY_API_KEY"
timeout_ms = 120000

# ── Search models ──────────────────────────────────────
[models.sonar]
provider = "perplexity"
slug = "sonar"
context_window = 127000
max_output = 16384
supports_tools = false
supports_search = true               # Perplexity-specific
supports_citations = true             # Perplexity-specific
search_context_size = "medium"        # low/medium/high — controls cost/depth tradeoff
tool_format = "openai_json"
cost_input_per_m = 1.00
cost_output_per_m = 1.00
cost_per_request = 0.005              # $5/1K requests at low context

[models.sonar-pro]
provider = "perplexity"
slug = "sonar-pro"
context_window = 200000
max_output = 16384
supports_tools = false
supports_search = true
supports_citations = true
search_context_size = "high"
tool_format = "openai_json"
cost_input_per_m = 3.00
cost_output_per_m = 15.00
cost_per_request = 0.014

[models.sonar-reasoning]
provider = "perplexity"
slug = "sonar-reasoning"
context_window = 127000
max_output = 16384
supports_tools = false
supports_search = true
supports_citations = true
supports_thinking = true              # CoT reasoning
tool_format = "openai_json"
cost_input_per_m = 1.00
cost_output_per_m = 5.00

[models.sonar-reasoning-pro]
provider = "perplexity"
slug = "sonar-reasoning-pro"
context_window = 128000
max_output = 16384
supports_tools = false
supports_search = true
supports_citations = true
supports_thinking = true
tool_format = "openai_json"
cost_input_per_m = 2.00
cost_output_per_m = 8.00

[models.sonar-deep-research]
provider = "perplexity"
slug = "sonar-deep-research"
context_window = 128000
max_output = 65536
supports_tools = false
supports_search = true
supports_citations = true
supports_thinking = true
supports_async = true                 # async polling API
tool_format = "openai_json"
cost_input_per_m = 2.00
cost_output_per_m = 8.00

# ── Embedding models ───────────────────────────────────
[models.pplx-embed-4b]
provider = "perplexity"
slug = "pplx-embed-v1-4b"
context_window = 8192
supports_tools = false
supports_search = false
supports_citations = false
is_embedding_model = true

# ── Perplexity search options (shared across models) ──
[perplexity]
default_search_model = "sonar-pro"
default_research_model = "sonar-deep-research"
default_reasoning_model = "sonar-reasoning-pro"
default_embed_model = "pplx-embed-4b"
search_recency_filter = "year"        # default recency for research
academic_mode = true                  # use search_mode = "academic" by default
search_domain_filter = []             # global domain filter
return_images = false
return_related_questions = true

# ── Role overrides: use Perplexity for research roles ──
[agent.role_overrides.researcher]
model = "sonar-pro"
backend = "perplexity"

[agent.role_overrides.fact_checker]
model = "sonar"
backend = "perplexity"
```

---

## Checklist

### Phase A: Core Adapter (depends on 02, 03)

#### 2Q.01 — Add PerplexityApi variant to ProviderKind

**File**: `crates/roko-core/src/agent.rs`
**What**: Add `PerplexityApi` to the `ProviderKind` enum:

```rust
pub enum ProviderKind {
    AnthropicApi,
    ClaudeCli,
    OpenAiCompat,
    CursorAcp,
    PerplexityApi,  // NEW
}
```

Update `label()`, `Display`, and `From<AgentBackend>` (maps to `PerplexityApi` for any `perplexity` slug).

**Acceptance**: `ProviderKind::PerplexityApi` compiles with Serialize/Deserialize.
**Verification**: `cargo test -p roko-core -- provider_kind`

---

#### 2Q.02 — Add Perplexity-specific fields to ModelProfile

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Extend `ModelProfile` with Perplexity-specific capability flags:

```rust
pub struct ModelProfile {
    // ... existing fields ...
    #[serde(default)]
    pub supports_search: bool,           // web-grounded search
    #[serde(default)]
    pub supports_citations: bool,        // returns citations[]
    #[serde(default)]
    pub supports_async: bool,            // async job API (deep research)
    #[serde(default)]
    pub is_embedding_model: bool,        // embedding, not chat
    pub search_context_size: Option<String>,  // "low"/"medium"/"high"
    pub cost_per_request: Option<f64>,   // per-request fee (not per-token)
}
```

**Context**: These flags let the adapter know which response extensions to parse and which request parameters to inject. `cost_per_request` is unique to Perplexity's pricing model (per-request fee on top of token costs).

**Acceptance**: Existing configs without these fields still parse (all default to false/None).
**Verification**: `cargo test -p roko-core -- model_profile`

---

#### 2Q.03 — Add PerplexityConfig section to RokoConfig

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add optional `[perplexity]` config section:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerplexityConfig {
    pub default_search_model: Option<String>,
    pub default_research_model: Option<String>,
    pub default_reasoning_model: Option<String>,
    pub default_embed_model: Option<String>,
    #[serde(default = "default_recency")]
    pub search_recency_filter: String,        // "hour"/"day"/"week"/"month"/"year"
    #[serde(default)]
    pub academic_mode: bool,
    #[serde(default)]
    pub search_domain_filter: Vec<String>,    // global domain filter
    #[serde(default)]
    pub return_images: bool,
    #[serde(default = "default_true")]
    pub return_related_questions: bool,
}

fn default_recency() -> String { "year".to_string() }
```

Add to `RokoConfig`:
```rust
#[serde(default)]
pub perplexity: PerplexityConfig,
```

**Acceptance**: Existing `roko.toml` still parses. New `roko.toml` with `[perplexity]` section loads correctly.
**Verification**: `cargo test -p roko-core -- perplexity_config`

---

#### 2Q.04 — Define PerplexityMetadata for response extensions

**File**: `crates/roko-agent/src/perplexity/types.rs` (new)
**What**: Types for Perplexity-specific response data:

```rust
/// Citation data from Perplexity search-grounded responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerplexityMetadata {
    /// Source URLs cited in the response.
    pub citations: Vec<String>,
    /// Full search result entries with content snippets.
    pub search_results: Vec<SearchResult>,
    /// Character-level annotation spans linking text to sources.
    pub annotations: Vec<Annotation>,
    /// Related questions suggested by the model.
    pub related_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub date: Option<String>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub start_index: usize,
    pub end_index: usize,
    pub title: String,
    pub url: String,
}

/// Search filter options injected into Perplexity requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    pub search_domain_filter: Option<Vec<String>>,
    pub search_recency_filter: Option<String>,
    pub search_after_date_filter: Option<String>,
    pub search_before_date_filter: Option<String>,
    pub last_updated_after_filter: Option<String>,
    pub last_updated_before_filter: Option<String>,
    pub search_context_size: Option<String>,
    pub search_mode: Option<String>,      // "academic" or "web"
    pub return_images: Option<bool>,
    pub return_related_questions: Option<bool>,
    pub user_location: Option<UserLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLocation {
    pub country: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
}

/// Request for the Agent/Responses API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,           // fast-search, pro-search, deep-research, advanced-deep-research
    pub input: serde_json::Value,         // text or message array
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AgentTool>>,
    #[serde(flatten)]
    pub search_options: SearchOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTool {
    #[serde(rename = "type")]
    pub tool_type: String,                // "web_search", "fetch_url", "function"
    #[serde(flatten)]
    pub config: serde_json::Value,
}

/// Response from the Agent/Responses API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub model: String,
    pub status: String,
    pub output: Vec<AgentOutputItem>,
    pub usage: AgentUsage,
    #[serde(default)]
    pub citations: Vec<String>,
    #[serde(default)]
    pub search_results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutputItem {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}
```

**Acceptance**: All types compile with Serialize/Deserialize. Round-trip test for example Perplexity response JSON.
**Verification**: `cargo test -p roko-agent -- perplexity_types`

---

#### 2Q.05 — Implement PerplexityAdapter

**File**: `crates/roko-agent/src/perplexity/adapter.rs` (new)
**What**: The core adapter implementing `ProviderAdapter`:

```rust
pub struct PerplexityAdapter;

impl ProviderAdapter for PerplexityAdapter {
    fn kind(&self) -> ProviderKind { ProviderKind::PerplexityApi }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        let api_key = provider.resolve_api_key()
            .ok_or_else(|| AgentCreationError::MissingApiKey(
                provider.api_key_env.clone().unwrap_or_else(|| "PERPLEXITY_API_KEY".into())
            ))?;

        let base_url = provider.base_url.clone()
            .unwrap_or_else(|| "https://api.perplexity.ai".to_string());

        // For embedding models, return an embedding-only agent
        if model.is_embedding_model {
            return Ok(Box::new(PerplexityEmbedAgent::new(api_key, base_url, model.slug.clone())));
        }

        // For async models (deep research), return async-capable agent
        if model.supports_async {
            return Ok(Box::new(PerplexityDeepResearchAgent::new(
                api_key, base_url, model.slug.clone(), options,
            )));
        }

        // For standard chat models, return chat agent with search extensions
        Ok(Box::new(PerplexityChatAgent::new(
            api_key, base_url, model.clone(), options,
        )))
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        match status {
            429 => ProviderError::RateLimit {
                retry_after_ms: body.pointer("/retry_after")
                    .and_then(|v| v.as_u64())
                    .map(|s| s * 1000)
            },
            401 | 403 => ProviderError::AuthFailure,
            404 => ProviderError::ModelNotFound,
            408 | 504 => ProviderError::Timeout,
            500..=599 => ProviderError::ServerError(status),
            _ => ProviderError::Other(format!("HTTP {status}")),
        }
    }
}
```

**Acceptance**: `PerplexityAdapter::create_agent()` returns the appropriate agent type based on model capabilities.
**Verification**: `cargo test -p roko-agent -- perplexity_adapter`

---

#### 2Q.06 — Implement PerplexityChatAgent (chat completions + search extensions)

**File**: `crates/roko-agent/src/perplexity/chat.rs` (new)
**What**: Agent that calls `POST /chat/completions` with Perplexity search extensions and preserves citations in the response:

```rust
pub struct PerplexityChatAgent {
    api_key: String,
    base_url: String,
    model: ModelProfile,
    search_options: SearchOptions,
    system_prompt: Option<String>,
    timeout_ms: u64,
}

impl PerplexityChatAgent {
    /// Build the request body with Perplexity-specific search parameters.
    fn build_request(&self, messages: &[ChatMessage]) -> serde_json::Value {
        let mut body = json!({
            "model": self.model.slug,
            "messages": messages,
        });

        // Inject search options from config
        if let Some(ref filter) = self.search_options.search_domain_filter {
            body["search_domain_filter"] = json!(filter);
        }
        if let Some(ref recency) = self.search_options.search_recency_filter {
            body["search_recency_filter"] = json!(recency);
        }
        if let Some(ref mode) = self.search_options.search_mode {
            body["search_mode"] = json!(mode);
        }
        if let Some(images) = self.search_options.return_images {
            body["return_images"] = json!(images);
        }
        if let Some(related) = self.search_options.return_related_questions {
            body["return_related_questions"] = json!(related);
        }
        // web_search_options
        if let Some(ref size) = self.search_options.search_context_size {
            body["web_search_options"] = json!({
                "search_context_size": size
            });
        }
        // Date filters
        if let Some(ref after) = self.search_options.search_after_date_filter {
            body["search_after_date_filter"] = json!(after);
        }
        if let Some(ref before) = self.search_options.search_before_date_filter {
            body["search_before_date_filter"] = json!(before);
        }

        body
    }

    /// Parse response, extracting citations and search_results into metadata.
    fn parse_response(&self, raw: &serde_json::Value) -> ChatResponse {
        // Standard OpenAI fields
        let content = raw.pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Perplexity extensions — top-level fields
        let citations: Vec<String> = raw.get("citations")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let search_results: Vec<SearchResult> = raw.get("search_results")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // Annotations inside message
        let annotations: Vec<Annotation> = raw.pointer("/choices/0/message/annotations")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let pplx_meta = PerplexityMetadata {
            citations,
            search_results,
            annotations,
            related_questions: vec![],
        };

        ChatResponse {
            content,
            reasoning: None,
            tool_calls: vec![],
            usage: /* parse usage */,
            finish_reason: /* parse finish_reason */,
            metadata: ResponseMetadata {
                response_id: raw.get("id").and_then(|v| v.as_str()).map(String::from),
                model_used: raw.get("model").and_then(|v| v.as_str()).map(String::from),
                cached_tokens: None,
                content_filter: None,
                provider_latency_ms: None,
                // Store Perplexity metadata as JSON in the extra field
                extra: Some(serde_json::to_value(&pplx_meta).unwrap_or_default()),
            },
        }
    }
}
```

**Acceptance**: Citations and search_results from a Perplexity response are preserved in `ChatResponse.metadata.extra`.
**Verification**: `cargo test -p roko-agent -- perplexity_chat_agent`

---

#### 2Q.07 — Implement PerplexityDeepResearchAgent (async polling)

**File**: `crates/roko-agent/src/perplexity/deep_research.rs` (new)
**What**: Agent for `sonar-deep-research` that submits async jobs and polls for completion:

```rust
pub struct PerplexityDeepResearchAgent {
    api_key: String,
    base_url: String,
    model_slug: String,
    poll_interval_ms: u64,      // default 5000
    max_poll_attempts: u32,     // default 120 (10 minutes)
}

impl PerplexityDeepResearchAgent {
    /// Submit async research job.
    async fn submit(&self, prompt: &str) -> Result<String> {
        // POST /v1/async/sonar
        // Returns { "request_id": "..." }
    }

    /// Poll for completion.
    async fn poll(&self, request_id: &str) -> Result<Option<AgentResponse>> {
        // GET /v1/async/sonar/{request_id}
        // Returns status: "pending" | "processing" | "completed" | "failed"
    }

    /// Submit + poll loop.
    async fn run_deep_research(&self, prompt: &str) -> Result<AgentResponse> {
        let request_id = self.submit(prompt).await?;
        for _ in 0..self.max_poll_attempts {
            tokio::time::sleep(Duration::from_millis(self.poll_interval_ms)).await;
            if let Some(response) = self.poll(&request_id).await? {
                return Ok(response);
            }
        }
        Err(anyhow!("Deep research timed out after {} attempts", self.max_poll_attempts))
    }
}
```

**Context**: Deep research jobs can take 1-10 minutes. The async API prevents blocking and allows the conductor to manage other tasks while waiting.

**Acceptance**: Submit returns a request_id. Poll loop handles pending/completed/failed states.
**Verification**: `cargo test -p roko-agent -- deep_research_polling` (mock)

---

#### 2Q.08 — Implement PerplexityEmbedAgent

**File**: `crates/roko-agent/src/perplexity/embed.rs` (new)
**What**: Agent wrapper for Perplexity's embedding API:

```rust
pub struct PerplexityEmbedAgent {
    api_key: String,
    base_url: String,
    model_slug: String,
}

impl PerplexityEmbedAgent {
    /// Generate embeddings for a batch of texts.
    pub async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // POST /v1/embeddings
        // { "model": "pplx-embed-v1-4b", "input": ["text1", "text2"] }
    }

    /// Generate contextualized embeddings.
    pub async fn embed_contextualized(
        &self,
        texts: &[&str],
        context: &str,
    ) -> Result<Vec<Vec<f32>>> {
        // POST /v1/contextualizedembeddings
    }
}
```

**Acceptance**: Embedding call returns float vectors.
**Verification**: `cargo test -p roko-agent -- perplexity_embed` (mock)

---

#### 2Q.09 — Implement PerplexitySearchClient (pure search, no generation)

**File**: `crates/roko-agent/src/perplexity/search.rs` (new)
**What**: Client for the Search API (structured results without generation):

```rust
pub struct PerplexitySearchClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub domain_filter: Option<Vec<String>>,
    pub date_range: Option<(String, String)>,  // (after, before)
    pub region: Option<String>,                 // ISO country code
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query: String,
}

impl PerplexitySearchClient {
    /// Execute up to 5 search queries in a single request.
    pub async fn search_batch(&self, queries: &[SearchQuery]) -> Result<Vec<SearchResponse>> {
        // POST /search
        // Multi-query bundling: up to 5 queries per call
    }

    /// Single query convenience method.
    pub async fn search(&self, query: &str) -> Result<SearchResponse> {
        self.search_batch(&[SearchQuery { query: query.to_string(), ..Default::default() }])
            .await
            .map(|mut r| r.pop().unwrap_or_default())
    }
}
```

**Context**: The Search API is $5/1K requests with no token costs — much cheaper than the chat API for cases where you just need search results (e.g., finding papers, verifying URLs exist).

**Acceptance**: Search returns structured results with title, URL, content, date.
**Verification**: `cargo test -p roko-agent -- perplexity_search` (mock)

---

#### 2Q.10 — Register PerplexityAdapter in adapter_for_kind()

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: Add the new adapter to the dispatch function:

```rust
pub fn adapter_for_kind(kind: ProviderKind) -> &'static dyn ProviderAdapter {
    match kind {
        ProviderKind::OpenAiCompat => &OpenAiCompatAdapter,
        ProviderKind::ClaudeCli => &ClaudeCliAdapter,
        ProviderKind::AnthropicApi => &AnthropicApiAdapter,
        ProviderKind::CursorAcp => &CursorAcpAdapter,
        ProviderKind::PerplexityApi => &PerplexityAdapter,  // NEW
    }
}
```

**Acceptance**: `adapter_for_kind(ProviderKind::PerplexityApi)` returns the Perplexity adapter.
**Verification**: `cargo test -p roko-agent -- adapter_for_kind`

---

### Phase B: Research Pipeline Integration

#### 2Q.11 — Add Perplexity search to research prompt builder

**File**: `crates/roko-cli/src/research.rs`
**What**: When a Perplexity provider is configured, modify `build_research_prompt()` to:

1. Remove the instruction to "verify papers exist" (Perplexity does this natively)
2. Add instruction to use `[N]` bracket notation matching Perplexity's citation format
3. For `ResearchMode::Topic`, set `search_mode = "academic"` and `search_recency_filter` from config

```rust
pub fn build_research_prompt_perplexity(
    workdir: &Path,
    topic: &str,
    context: &str,
    mode: ResearchMode,
    pplx_config: &PerplexityConfig,
) -> (String, SearchOptions) {
    let prompt = build_research_prompt(workdir, topic, context, mode);

    let search_opts = SearchOptions {
        search_mode: if pplx_config.academic_mode {
            Some("academic".to_string())
        } else {
            None
        },
        search_recency_filter: Some(pplx_config.search_recency_filter.clone()),
        search_domain_filter: if pplx_config.search_domain_filter.is_empty() {
            None
        } else {
            Some(pplx_config.search_domain_filter.clone())
        },
        return_related_questions: Some(pplx_config.return_related_questions),
        return_images: Some(pplx_config.return_images),
        ..Default::default()
    };

    (prompt, search_opts)
}
```

**Acceptance**: Research prompts include Perplexity-aware instructions. SearchOptions populated from config.
**Verification**: `cargo test -p roko-cli -- research_prompt_perplexity`

---

#### 2Q.12 — Wire Perplexity into `roko research topic` dispatch

**File**: `crates/roko-cli/src/main.rs` (research subcommand handler)
**What**: When `perplexity.default_search_model` is configured, use `PerplexityChatAgent` instead of the Claude CLI agent for research:

```rust
match research_mode {
    ResearchMode::Topic if config.perplexity.default_search_model.is_some() => {
        let model_key = config.perplexity.default_search_model.as_ref().unwrap();
        let agent = create_agent_for_model(&config, model_key, opts)?;
        let (prompt, search_opts) = build_research_prompt_perplexity(...);
        // Inject search_opts into agent options
        let result = agent.run_with_search(prompt, search_opts).await?;
        // Extract citations from result metadata
        save_research_with_citations(workdir, topic, &result)?;
    },
    _ => {
        // Existing Claude CLI path
    }
}
```

**Acceptance**: `roko research topic "X"` with Perplexity configured produces research with real citations and source URLs.
**Verification**: Manual test with `PERPLEXITY_API_KEY` set.

---

#### 2Q.13 — Wire Perplexity deep research into `roko research topic --deep`

**File**: `crates/roko-cli/src/main.rs`
**What**: Add `--deep` flag to `roko research topic` that uses `sonar-deep-research` via the async API:

```rust
#[arg(long, help = "Use Perplexity deep research (async, 1-10 min)")]
deep: bool,
```

When `--deep`:
1. Use `PerplexityDeepResearchAgent`
2. Show progress indicator while polling
3. Save the exhaustive report to `.roko/research/<slug>-deep.md`

**Acceptance**: `roko research topic "X" --deep` produces an exhaustive report with 20+ citations.
**Verification**: Manual test.

---

#### 2Q.14 — Post-process citations into research markdown

**File**: `crates/roko-cli/src/research.rs`
**What**: Add `save_research_with_citations()` that converts Perplexity response + metadata into rich markdown:

```rust
pub fn save_research_with_citations(
    workdir: &Path,
    topic: &str,
    content: &str,
    metadata: &PerplexityMetadata,
) -> Result<PathBuf> {
    let mut doc = String::new();
    writeln!(doc, "# Research: {topic}\n")?;
    writeln!(doc, "> Generated via Perplexity Sonar — {}\n", chrono::Local::now().format("%Y-%m-%d"))?;
    writeln!(doc, "{content}\n")?;

    // Append sources section
    if !metadata.citations.is_empty() {
        writeln!(doc, "\n## Sources\n")?;
        for (i, url) in metadata.citations.iter().enumerate() {
            let title = metadata.search_results.get(i)
                .map(|r| r.title.as_str())
                .unwrap_or("Source");
            writeln!(doc, "{i}. [{title}]({url})")?;
        }
    }

    // Append search results with content for agent context
    if !metadata.search_results.is_empty() {
        writeln!(doc, "\n## Search Context\n")?;
        for result in &metadata.search_results {
            writeln!(doc, "### [{}]({})", result.title, result.url)?;
            if let Some(date) = &result.date {
                writeln!(doc, "> Published: {date}")?;
            }
            writeln!(doc, "\n{}\n", result.content)?;
        }
    }

    let path = research_dir(workdir).join(format!("{}.md", slug(topic)));
    std::fs::write(&path, doc)?;
    Ok(path)
}
```

**Acceptance**: Research output includes numbered citations with working URLs and a search context section.
**Verification**: `cargo test -p roko-cli -- save_research_citations`

---

### Phase C: CascadeRouter Integration

#### 2Q.15 — Add Perplexity models to CascadeRouter static table

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Add Perplexity models to the static routing stage (Stage 1):

```rust
// Research tasks → Perplexity by default when available
("Researcher", "sonar-pro"),
("FactChecker", "sonar"),
```

The CascadeRouter needs to know that Perplexity models exist as routing targets. When `supports_search = true` and the task category is `Research`, bias toward Perplexity models.

**Acceptance**: CascadeRouter routes research tasks to Perplexity models when configured.
**Verification**: `cargo test -p roko-learn -- cascade_perplexity`

---

#### 2Q.16 — Add per-request cost to CostTable

**File**: `crates/roko-learn/src/costs_db.rs`
**What**: Extend cost calculation to include per-request fees:

```rust
pub struct ModelCost {
    pub input_per_m: f64,
    pub output_per_m: f64,
    pub cache_read_per_m: Option<f64>,
    pub cache_write_per_m: Option<f64>,
    pub per_request: Option<f64>,       // NEW: per-request fee
}

impl ModelCost {
    pub fn estimate_total(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let token_cost = (input_tokens as f64 / 1_000_000.0) * self.input_per_m
            + (output_tokens as f64 / 1_000_000.0) * self.output_per_m;
        token_cost + self.per_request.unwrap_or(0.0)
    }
}
```

Add Perplexity model costs:

| Model | Input/M | Output/M | Per Request |
|---|---|---|---|
| sonar | $1.00 | $1.00 | $0.005 |
| sonar-pro | $3.00 | $15.00 | $0.014 |
| sonar-reasoning | $1.00 | $5.00 | $0.005 |
| sonar-reasoning-pro | $2.00 | $8.00 | $0.008 |
| sonar-deep-research | $2.00 | $8.00 | $0.005/search |

**Acceptance**: Cost estimate for a Perplexity request includes both token cost and per-request fee.
**Verification**: `cargo test -p roko-learn -- perplexity_costs`

---

### Phase D: Advanced Features

#### 2Q.17 — Add Perplexity search to pre-task context enrichment

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Before dispatching a coding task, optionally use Perplexity `sonar` (cheap, fast) to search for relevant context:

```rust
async fn enrich_task_context_with_search(
    task: &Task,
    pplx_client: &PerplexitySearchClient,
) -> Option<String> {
    if !task.needs_external_context() { return None; }

    let query = format!(
        "Rust {} {} best practices patterns",
        task.category, task.description_summary()
    );
    let results = pplx_client.search(&query).await.ok()?;

    // Format top 3 results as context
    let context = results.results.iter().take(3)
        .map(|r| format!("### {}\n{}\nSource: {}", r.title, r.content, r.url))
        .collect::<Vec<_>>()
        .join("\n\n");

    Some(context)
}
```

**Context**: This uses the Search API ($5/1K requests, no token costs) — extremely cheap for enriching task context with real-world examples and documentation.

**Acceptance**: Tasks with external context needs get search-enriched context. Tasks without don't make unnecessary API calls.
**Verification**: `cargo test -p roko-cli -- search_context_enrichment` (mock)

---

#### 2Q.18 — Add Perplexity fact-checking gate

**File**: `crates/roko-gate/src/fact_check.rs` (new)
**What**: A new gate that uses Perplexity `sonar` to verify claims in agent output:

```rust
pub struct FactCheckGate {
    search_client: PerplexitySearchClient,
    min_confidence: f64,  // 0.0-1.0, default 0.7
}

impl Gate for FactCheckGate {
    async fn check(&self, output: &Signal) -> GateResult {
        // Extract verifiable claims from output
        let claims = extract_claims(output.text());

        let mut verified = 0;
        let mut total = 0;
        for claim in claims {
            total += 1;
            let result = self.search_client.search(&claim).await?;
            if result.results.iter().any(|r| supports_claim(&r.content, &claim)) {
                verified += 1;
            }
        }

        let confidence = if total > 0 { verified as f64 / total as f64 } else { 1.0 };
        if confidence >= self.min_confidence {
            GateResult::Pass
        } else {
            GateResult::Fail(format!(
                "Fact check: {verified}/{total} claims verified ({:.0}%)",
                confidence * 100.0
            ))
        }
    }
}
```

**Context**: This is optional and off by default — enabled via `[gates.fact_check]` in config. Useful for documentation tasks where factual accuracy matters.

**Acceptance**: Gate passes when claims are web-verifiable, fails when they're not.
**Verification**: `cargo test -p roko-gate -- fact_check_gate` (mock)

---

#### 2Q.19 — Add research embedding index

**File**: `crates/roko-cli/src/research.rs`
**What**: Use Perplexity embeddings to build a semantic index over `.roko/research/` files:

```rust
pub async fn build_research_index(
    workdir: &Path,
    embed_agent: &PerplexityEmbedAgent,
) -> Result<ResearchIndex> {
    let files = list_research(workdir)?;
    let mut index = ResearchIndex::new();

    for file in files {
        let content = std::fs::read_to_string(&file)?;
        let chunks = chunk_markdown(&content, 512);  // ~512 token chunks
        let embeddings = embed_agent.embed(&chunks.iter().map(|c| c.as_str()).collect::<Vec<_>>()).await?;

        for (chunk, embedding) in chunks.into_iter().zip(embeddings) {
            index.add(file.clone(), chunk, embedding);
        }
    }

    Ok(index)
}

pub async fn search_research(
    index: &ResearchIndex,
    embed_agent: &PerplexityEmbedAgent,
    query: &str,
    top_k: usize,
) -> Result<Vec<ResearchHit>> {
    let query_embedding = embed_agent.embed(&[query]).await?;
    index.search(&query_embedding[0], top_k)
}
```

**Context**: This enables semantic search over accumulated research. When building task context in `orchestrate.rs`, relevant research findings can be injected automatically.

**Acceptance**: Research index builds from `.roko/research/` files. Semantic search returns relevant chunks.
**Verification**: `cargo test -p roko-cli -- research_index` (mock embeddings)

---

#### 2Q.20 — Add `roko research search` subcommand

**File**: `crates/roko-cli/src/main.rs`
**What**: Direct web search from CLI using the Search API:

```bash
# Quick web search
roko research search "Rust async trait best practices 2025"

# Search with domain filter
roko research search "actor model patterns" --domains "docs.rs,github.com"

# Search with recency filter
roko research search "Claude API changes" --recency week
```

This is a lightweight alternative to `roko research topic` — returns raw search results without synthesis, at $5/1K requests.

**Acceptance**: `roko research search "X"` returns structured search results with titles, URLs, and content snippets.
**Verification**: Manual test with API key.

---

### Phase E: Production & Testing

#### 2Q.21 — Write example config: roko-perplexity.toml

**File**: `examples/roko-perplexity.toml`
**What**: Complete example config showing Perplexity as research backend alongside Claude as coding backend:

```toml
# Perplexity for research + search, Claude for coding
[providers.perplexity]
kind = "perplexity_api"
base_url = "https://api.perplexity.ai"
api_key_env = "PERPLEXITY_API_KEY"

[providers.claude_cli]
kind = "claude_cli"
command = "claude"
args = ["--print", "--output-format", "stream-json"]

[models.sonar]
provider = "perplexity"
slug = "sonar"
# ... full config ...

[models.sonar-pro]
provider = "perplexity"
slug = "sonar-pro"
# ... full config ...

[models.sonar-deep-research]
provider = "perplexity"
slug = "sonar-deep-research"
# ... full config ...

[models.claude-opus]
provider = "claude_cli"
slug = "claude-opus-4-6"
# ... full config ...

[perplexity]
default_search_model = "sonar"
default_research_model = "sonar-pro"
academic_mode = true
search_recency_filter = "year"
return_related_questions = true

[agent]
default_model = "claude-opus"
[agent.role_overrides.researcher]
model = "sonar-pro"
[agent.role_overrides.fact_checker]
model = "sonar"
```

**Acceptance**: Config parses. Models resolve to correct providers.
**Verification**: `cargo test -p roko-core -- perplexity_example_config`

---

#### 2Q.22 — Write mock integration tests

**File**: `crates/roko-agent/tests/perplexity_integration.rs` (new)
**What**: Comprehensive mock tests:

1. Chat completions with citations in response
2. Chat completions with academic search mode
3. Deep research submit + poll cycle
4. Search API single query
5. Search API batch (5 queries)
6. Embedding single text
7. Error classification (429, 401, 404)
8. Domain filter and recency filter injection

**Acceptance**: All 8 test cases pass with mocked HTTP responses.
**Verification**: `cargo test -p roko-agent -- perplexity_`

---

#### 2Q.23 — Add Perplexity health tracking to learning loops

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Record Perplexity-specific observations:

- Citation count per response (quality signal)
- Search latency (Perplexity-specific, not just LLM latency)
- Per-request cost tracking (token cost + request fee)

These feed into the CascadeRouter's observation pipeline so it can learn when Perplexity models are more cost-effective than Claude for research tasks.

**Acceptance**: Observations include citation count and total cost (tokens + per-request).
**Verification**: `cargo test -p roko-learn -- perplexity_observations`

---

#### 2Q.24 — Update lib.rs exports and module structure

**File**: `crates/roko-agent/src/lib.rs`
**What**: Add `pub mod perplexity;` and re-export key types:

```rust
pub mod perplexity;
pub use perplexity::{
    PerplexityChatAgent,
    PerplexityDeepResearchAgent,
    PerplexityEmbedAgent,
    PerplexitySearchClient,
    PerplexityMetadata,
    SearchOptions,
    SearchResult,
    Annotation,
};
```

**Acceptance**: `use roko_agent::perplexity::PerplexityChatAgent;` compiles from roko-cli.
**Verification**: `cargo check -p roko-cli`

---

## Execution Order

**Phase A** (core, sequential, depends on 02 + 03):
1. 2Q.01 — ProviderKind variant
2. 2Q.02 — ModelProfile extensions
3. 2Q.03 — PerplexityConfig
4. 2Q.04 — Type definitions
5. 2Q.05 — PerplexityAdapter
6. 2Q.06 — PerplexityChatAgent
7. 2Q.07 — PerplexityDeepResearchAgent
8. 2Q.08 — PerplexityEmbedAgent
9. 2Q.09 — PerplexitySearchClient
10. 2Q.10 — Register in adapter_for_kind

**Phase B** (research pipeline, depends on Phase A):
11. 2Q.11 — Research prompt builder
12. 2Q.12 — Wire into `roko research topic`
13. 2Q.13 — Deep research flag
14. 2Q.14 — Citation post-processing

**Phase C** (routing + costs, parallelizable with Phase B):
15. 2Q.15 — CascadeRouter static table
16. 2Q.16 — Per-request cost model

**Phase D** (advanced, after B + C):
17. 2Q.17 — Pre-task context enrichment
18. 2Q.18 — Fact-checking gate
19. 2Q.19 — Research embedding index
20. 2Q.20 — `roko research search` subcommand

**Phase E** (testing + docs, anytime after Phase A):
21. 2Q.21 — Example config
22. 2Q.22 — Mock integration tests
23. 2Q.23 — Health tracking
24. 2Q.24 — Module exports

---

## Why Dedicated Adapter (Not OpenAI Compat)

The `OpenAiCompatAdapter` handles the chat completions wire format but would silently drop:
- `citations[]` — the core value of using Perplexity
- `search_results[]` — structured source data
- `annotations[]` — character-level source linking
- `search_domain_filter` — domain control
- `search_recency_filter` — temporal control
- `web_search_options` — context size tuning
- The entire Agent API (`/v1/agent`) — presets, tools, async
- The Search API (`/search`) — pure search, no generation
- The Embeddings API — semantic search over research

Forcing these through `extra_params()` on the generic adapter would be fragile and would require the caller to parse raw JSON to extract citations. A dedicated adapter makes citations, search results, and annotations first-class in the type system.

## Why Not MCP

An alternative is wrapping Perplexity as an MCP server (like `roko-mcp-slack`). This would:
- **Pro**: No code change to agent dispatch; tool-level integration
- **Con**: Loses citation structure (MCP tool results are strings)
- **Con**: Can't use Perplexity AS the LLM (only as a tool called by another LLM)
- **Con**: Double cost (Claude calls Perplexity tool, pays for both)

The dedicated adapter approach means Perplexity IS the LLM for research tasks — one API call, one cost, full citation fidelity.

## Also Available via OpenRouter

All Perplexity models are available on OpenRouter (`perplexity/sonar`, `perplexity/sonar-pro`, etc.) but OpenRouter strips the search extensions (citations, search_results, annotations). For full feature access, direct API is required. For cost-only routing (where citations don't matter), OpenRouter works fine.

```toml
# OpenRouter fallback (cheaper, no citations)
[models.sonar-or]
provider = "openrouter"
slug = "perplexity/sonar"
supports_search = false       # OpenRouter strips search extensions
supports_citations = false
cost_input_per_m = 1.10       # 10% OpenRouter markup
cost_output_per_m = 1.10

# Direct API (full features)
[models.sonar]
provider = "perplexity"
slug = "sonar"
supports_search = true
supports_citations = true
cost_input_per_m = 1.00
cost_output_per_m = 1.00
```
