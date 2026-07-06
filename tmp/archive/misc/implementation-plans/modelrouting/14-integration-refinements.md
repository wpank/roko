# 14 — Integration Refinements: Wiring, Token Counting, Rate Limits, MCP Bridge, Concurrency

> **Priority**: 🔴 P0 for Gap A (ToolLoop already exists — wire, don't rebuild); 🟡 P1 for rest
> **Status**: Not started
> **Depends on**: 02, 03, 13-A (which this doc SUPERSEDES for the tool loop)

## Critical Discovery: ToolLoop Already Exists

**`crates/roko-agent/src/tool_loop/mod.rs` already implements the multi-turn tool loop.** It has:
- `ToolLoop` struct with `translator`, `dispatcher`, `backend`, `max_iterations`, `context_token_limit`
- `LlmBackend` trait: `async fn send_turn(&self, messages, tools) -> BackendResponse`
- `run()` method that loops: send → parse_calls → dispatch_batch → render_results → append → repeat
- `resume()` for checkpoint-based resumption
- `Checkpoint` struct for crash recovery
- Integration with `ToolDispatcher.dispatch_batch()` at line 250
- Context pruning when approaching token limit

**What's missing is NOT the loop — it's the `LlmBackend` implementations for HTTP providers.** The `ClaudeCliAgent` and `ExecAgent` don't implement `LlmBackend`. The `OpenAiAgent`/`CodexAgent`/`OllamaAgent` don't either. The trait exists, the loop works, but nobody implements the trait.

**Doc 13 tasks 2K.05–2K.09 should be revised to wire the EXISTING ToolLoop, not build a new ToolLoopRunner.** This doc provides the corrected tasks.

---

## A. Wire Existing ToolLoop via LlmBackend Implementations

### 2L.01 — Implement LlmBackend for OpenAI-compatible HTTP endpoints

**File**: `crates/roko-agent/src/tool_loop/backends/openai_compat.rs` (new)
**What**: The missing piece — an `LlmBackend` impl that sends turns via HTTP to any OpenAI-compatible API:

```rust
use crate::tool_loop::LlmBackend;
use crate::http::HttpPoster;
use crate::translate::RenderedTools;

pub struct OpenAiCompatBackend {
    poster: Arc<dyn HttpPoster>,
    base_url: String,
    api_key: String,
    model_slug: String,
    extra_headers: Vec<(String, String)>,
    extra_body_params: serde_json::Map<String, Value>,  // thinking, tool_stream, etc.
    timeout_ms: u64,
}

#[async_trait]
impl LlmBackend for OpenAiCompatBackend {
    async fn send_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
    ) -> Result<BackendResponse, LlmError> {
        let mut body = serde_json::json!({
            "model": self.model_slug,
            "messages": messages,
        });

        // Add tools
        if let RenderedTools::JsonArray(tools_json) = tools {
            body["tools"] = tools_json.clone();
        }

        // Inject extra params (thinking, tool_stream, etc.)
        if let Some(obj) = body.as_object_mut() {
            for (k, v) in &self.extra_body_params {
                obj.insert(k.clone(), v.clone());
            }
        }

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
        ];
        if !self.api_key.is_empty() {
            headers.push(("Authorization".to_string(), format!("Bearer {}", self.api_key)));
        }
        headers.extend(self.extra_headers.clone());

        let response_text = self.poster.post_json(
            &url, &headers, &serde_json::to_vec(&body)?, self.timeout_ms,
        ).await.map_err(|e| LlmError::Http(e.message))?;

        let json: Value = serde_json::from_str(&response_text)
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        Ok(BackendResponse::Json(json))
    }
}
```

**Context**: This is the ONE piece that connects the existing `ToolLoop` to HTTP providers. Once this exists, GLM-5.1, Kimi-K2.5, OpenRouter, Ollama, and any OpenAI-compatible endpoint can run the full tool loop.

**Acceptance**: `OpenAiCompatBackend` implements `LlmBackend`. A mock HTTP server receives correctly formatted requests with tools.
**Verification**: `cargo test -p roko-agent -- openai_compat_backend`

---

### 2L.02 — Create LlmBackend from ProviderConfig + ModelProfile

**File**: `crates/roko-agent/src/tool_loop/backends/mod.rs` (new)
**What**: Factory function to create the right `LlmBackend` from provider config:

```rust
pub fn create_backend(
    provider: &ProviderConfig,
    model: &ModelProfile,
    poster: Arc<dyn HttpPoster>,
) -> Result<Arc<dyn LlmBackend>, AgentCreationError> {
    match provider.kind {
        ProviderKind::OpenAiCompat => {
            let mut extra_params = serde_json::Map::new();
            // Inject model-specific params
            if model.supports_thinking {
                extra_params.insert("thinking".into(), json!({"type": "enabled"}));
            }
            // ... more model-specific params
            Ok(Arc::new(OpenAiCompatBackend {
                poster,
                base_url: provider.base_url.clone().unwrap_or_default(),
                api_key: provider.resolve_api_key().unwrap_or_default(),
                model_slug: model.slug.clone(),
                extra_headers: provider.extra_headers_vec(),
                extra_body_params: extra_params,
                timeout_ms: provider.timeout_ms.unwrap_or(120_000),
            }))
        },
        ProviderKind::AnthropicApi => {
            // Anthropic Messages API backend
            todo!("Implement AnthropicBackend")
        },
        ProviderKind::ClaudeCli | ProviderKind::CursorAcp => {
            Err(AgentCreationError::MissingConfig(
                "CLI/ACP backends don't use LlmBackend — they own the tool loop".into()
            ))
        },
    }
}
```

**Acceptance**: `create_backend()` returns `OpenAiCompatBackend` for Z.AI config.
**Verification**: `cargo test -p roko-agent -- create_backend_factory`

---

### 2L.03 — Update OpenAiCompatAdapter to use ToolLoop + LlmBackend

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: Revise task 2K.09. When `model.supports_tools = true`, create a `ToolLoop`-based agent instead of a bare `CodexAgent`:

```rust
fn create_agent(&self, provider: &ProviderConfig, model: &ModelProfile, options: &AgentOptions)
    -> Result<Box<dyn Agent>, AgentCreationError>
{
    let poster = Arc::new(ReqwestPoster::new());

    if model.supports_tools {
        let backend = create_backend(provider, model, poster.clone())?;
        let translator = translator_for(&model.slug);
        let registry = build_tool_registry(options);
        let resolver = Arc::new(StdHandlerResolver);  // from roko-std
        let dispatcher = Arc::new(ToolDispatcher::new(registry, resolver));

        let tool_loop = ToolLoop::new(translator, dispatcher, backend)
            .with_max_iterations(50)
            .with_context_token_limit(model.context_window.unwrap_or(128_000) as usize);

        Ok(Box::new(ToolLoopAgent::new(tool_loop, options)))
    } else {
        // Single-shot fallback
        Ok(Box::new(CodexAgent::new(/* ... */)))
    }
}
```

**Context**: This wires the EXISTING `ToolLoop` (not a new one) with the new `OpenAiCompatBackend`. The `ToolLoop` handles the multi-turn iteration, tool dispatch, context pruning, and checkpointing. We just provide the HTTP backend.

**Acceptance**: GLM-5.1 config produces a `ToolLoopAgent` that can execute multi-turn tool calls.
**Verification**: `cargo test -p roko-agent -- adapter_uses_tool_loop`

---

### 2L.04 — Implement ToolLoopAgent wrapping ToolLoop as Agent trait

**File**: `crates/roko-agent/src/tool_loop/agent_wrapper.rs` (new)
**What**: Wrap `ToolLoop` in the `Agent` trait for orchestrator compatibility:

```rust
pub struct ToolLoopAgent {
    tool_loop: ToolLoop,
    system_prompt: Option<String>,
    tools: Vec<ToolDef>,
    name: String,
}

#[async_trait]
impl Agent for ToolLoopAgent {
    async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult {
        let prompt = input.body.as_text().unwrap_or_default();
        let tool_ctx = ToolContext::new(); // from roko-core

        let output = self.tool_loop.run(
            self.system_prompt.as_deref().unwrap_or(""),
            &prompt,
            &self.tools,
            &tool_ctx,
        ).await;

        match output.stop_reason {
            StopReason::Stop => AgentResult::ok(Signal::text(&output.final_text)),
            StopReason::MaxIterations => AgentResult::fail(
                Signal::text(&format!("Max iterations ({}) reached", output.iterations))
            ),
            StopReason::Error(e) => AgentResult::fail(Signal::text(&e)),
            _ => AgentResult::ok(Signal::text(&output.final_text)),
        }
    }

    fn name(&self) -> &str { &self.name }
}
```

**Acceptance**: `ToolLoopAgent` implements `Agent`. Orchestrator can use it interchangeably with `ClaudeCliAgent`.
**Verification**: `cargo test -p roko-agent -- tool_loop_agent_wrapper`

---

### 2L.05 — Add checkpoint persistence for ToolLoop

**File**: `crates/roko-agent/src/tool_loop/checkpoint.rs`
**What**: The `Checkpoint` struct exists but is never serialized. Add persistence:

```rust
impl Checkpoint {
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let cp: Self = serde_json::from_str(&json)?;
        Ok(cp)
    }
}
```

Wire into `ToolLoop::run()`: after each iteration, save checkpoint to `.roko/state/tool-loop-{task_id}.json`. On resume, call `ToolLoop::resume(checkpoint)`.

**Acceptance**: Checkpoint survives process restart. `resume()` continues from saved state.
**Verification**: `cargo test -p roko-agent -- checkpoint_persistence`

---

## B. Token Counting

### 2L.06 — Add TokenCounter abstraction

**File**: `crates/roko-compose/src/token_counter.rs` (new)
**What**: Client-side token counting for prompt budget management:

```rust
pub enum TokenCounter {
    /// OpenAI models — uses tiktoken-rs
    Tiktoken(tiktoken_rs::CoreBPE),
    /// GLM, Kimi, etc. — uses HuggingFace tokenizers crate
    HuggingFace(tokenizers::Tokenizer),
    /// Fallback heuristic (~4 chars per token for English)
    Heuristic { chars_per_token: f64 },
}

impl TokenCounter {
    pub fn count(&self, text: &str) -> usize {
        match self {
            Self::Tiktoken(bpe) => bpe.encode_with_special_tokens(text).len(),
            Self::HuggingFace(tok) => tok.encode(text, false)
                .map(|e| e.get_ids().len())
                .unwrap_or(0),
            Self::Heuristic { chars_per_token } => (text.len() as f64 / chars_per_token) as usize,
        }
    }

    pub fn for_model(slug: &str) -> Self {
        if slug.starts_with("claude-") || slug.starts_with("gpt-") || slug.starts_with("o1") {
            Self::Tiktoken(tiktoken_rs::o200k_base().unwrap())
        } else if slug.starts_with("glm-") {
            // Load from HuggingFace tokenizer.json if available
            Self::try_hf("zai-org/GLM-4.7")
                .unwrap_or(Self::Heuristic { chars_per_token: 3.8 })
        } else if slug.starts_with("kimi-") {
            Self::try_hf("moonshotai/Kimi-K2-Instruct")
                .unwrap_or(Self::Heuristic { chars_per_token: 3.5 })
        } else {
            Self::Heuristic { chars_per_token: 4.0 }
        }
    }
}
```

**Deps**: Add `tiktoken-rs` and `tokenizers` to `roko-compose/Cargo.toml`.

**Context**: The SystemPromptBuilder needs to count tokens BEFORE sending to enforce budgets. Different models have different tokenizers. Using the wrong tokenizer causes budget overflows (prompt too large) or underutilization (wasting context).

**Acceptance**: `TokenCounter::for_model("glm-5.1").count("hello world")` returns a reasonable number. `for_model("claude-opus-4-6")` uses tiktoken.
**Verification**: `cargo test -p roko-compose -- token_counter`

---

### 2L.07 — Wire TokenCounter into SystemPromptBuilder

**File**: `crates/roko-compose/src/system_prompt_builder.rs`
**What**: Use actual token counts instead of character estimates when assembling sections:

```rust
impl SystemPromptBuilder {
    pub fn build_with_counter(&self, counter: &TokenCounter) -> String {
        let sections = self.build_sections();
        let mut result = String::new();
        let mut used_tokens = 0;

        for section in sections.iter().sorted_by(|a, b| {
            b.cache_layer.cmp(&a.cache_layer).then(b.priority.cmp(&a.priority))
        }) {
            let section_tokens = counter.count(&section.content);
            if used_tokens + section_tokens > self.token_budget {
                // Truncate or drop based on priority
                if section.priority >= Priority::Critical {
                    let available = self.token_budget - used_tokens;
                    result.push_str(&truncate_to_tokens(&section.content, available, counter));
                }
                continue;
            }
            result.push_str(&section.content);
            used_tokens += section_tokens;
        }
        result
    }
}
```

**Acceptance**: Prompt never exceeds token budget. High-priority sections are included first.
**Verification**: `cargo test -p roko-compose -- budget_enforcement`

---

## C. Rate Limiting

### 2L.08 — Add per-provider rate limiter

**File**: `crates/roko-agent/src/rate_limit.rs` (new)
**What**: Rate limiting using the `governor` crate:

```rust
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use std::num::NonZeroU32;

pub struct ProviderRateLimiter {
    rpm_limiter: RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>,
}

impl ProviderRateLimiter {
    pub fn new(default_rpm: u32) -> Self {
        Self {
            rpm_limiter: RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(default_rpm).unwrap_or(NonZeroU32::new(60).unwrap()))
            ),
        }
    }

    pub async fn acquire(&self, provider_id: &str) {
        self.rpm_limiter.until_key_ready(&provider_id.to_string()).await;
    }
}
```

**Deps**: Add `governor` to `roko-agent/Cargo.toml`.

**Context**: Without client-side rate limiting, parallel tasks can overwhelm a provider and trigger 429s. The `governor` crate uses the GCRA algorithm (leaky bucket variant), which is the industry standard.

**Acceptance**: 10 rapid requests to the same provider are spread across the rate limit window.
**Verification**: `cargo test -p roko-agent -- rate_limiter`

---

### 2L.09 — Wire rate limiter into OpenAiCompatBackend

**File**: `crates/roko-agent/src/tool_loop/backends/openai_compat.rs`
**What**: Call `rate_limiter.acquire(provider_id)` before each HTTP request in `send_turn()`.

**Acceptance**: Requests are throttled per provider.
**Verification**: `cargo test -p roko-agent -- rate_limited_backend`

---

## D. MCP-to-Function Bridge for HTTP Backends

### 2L.10 — Implement MCP tool discovery and conversion for HTTP backends

**File**: `crates/roko-agent/src/mcp/bridge.rs` (new)
**What**: For Claude CLI, MCP is handled via `--mcp-config`. For HTTP backends, MCP tools must be discovered, converted to function definitions, and added to the tools array:

```rust
use crate::mcp::{McpClient, McpConfig, mcp_to_tool_def};

pub async fn discover_mcp_tools(config: &McpConfig) -> Result<Vec<ToolDef>> {
    let mut tools = Vec::new();
    for server in &config.servers {
        let client = McpClient::connect(server)?;
        let mcp_tools = client.list_tools().await?;
        for mcp_tool in mcp_tools {
            tools.push(mcp_to_tool_def(&mcp_tool));
        }
    }
    Ok(tools)
}
```

**Context**: The function `mcp_to_tool_def()` already exists in `crates/roko-agent/src/mcp/`. The `DynamicToolRegistry` also exists and composes static + MCP tools. This task wires them into the HTTP backend path.

When `ToolLoop` renders tools via `translator.render_tools()`, MCP-discovered tools appear as normal function definitions alongside builtin tools.

**Acceptance**: MCP tools from a local server appear in the HTTP request's `tools` array.
**Verification**: `cargo test -p roko-agent -- mcp_bridge_http`

---

### 2L.11 — Implement MCP tool invocation handler for ToolDispatcher

**File**: `crates/roko-agent/src/mcp/handler.rs` (new or extend existing)
**What**: When the model calls an MCP tool, the `HandlerResolver` needs to route it to the MCP client:

```rust
pub struct McpHandlerResolver {
    static_resolver: Arc<dyn HandlerResolver>,
    mcp_clients: HashMap<String, McpClient>,
}

impl HandlerResolver for McpHandlerResolver {
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        // 1. Try static resolver first (builtin tools)
        if let Some(handler) = self.static_resolver.resolve(name) {
            return Some(handler);
        }
        // 2. Try MCP clients
        for client in self.mcp_clients.values() {
            if client.has_tool(name) {
                return Some(Arc::new(McpToolHandler::new(client.clone(), name.to_string())));
            }
        }
        None
    }
}
```

**Acceptance**: Tool calls for MCP-discovered tools route to the MCP server.
**Verification**: `cargo test -p roko-agent -- mcp_handler_resolver`

---

## E. Fallback Chains

### 2L.12 — Extend CascadeModel with fallback chain

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Replace single `fallback: Option<ModelSpec>` with a chain:

```rust
pub struct CascadeModel {
    pub primary: ModelSpec,
    pub fallback_chain: Vec<ModelSpec>,   // ordered: try each in sequence
    pub context_overflow_fallback: Option<ModelSpec>,  // model with larger context
    pub latency_sla_ms: u64,
    pub stage: CascadeStage,
}
```

**Context**: From LiteLLM — different failure types should route to different fallbacks. Context overflow → larger context model. Rate limit → different provider for same model. General error → next model in chain.

**Acceptance**: `CascadeModel` with 3 fallbacks tries each in sequence on failure.
**Verification**: `cargo test -p roko-learn -- fallback_chain`

---

### 2L.13 — Add error-type-specific fallback routing

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Route to different fallbacks based on error type:

```rust
impl CascadeModel {
    pub fn fallback_for_error(&self, error: &ProviderError) -> Option<&ModelSpec> {
        match error {
            ProviderError::ContextOverflow => self.context_overflow_fallback.as_ref(),
            ProviderError::RateLimit { .. } => {
                // Find first fallback from a DIFFERENT provider
                self.fallback_chain.iter().find(|m| m.provider != self.primary.provider)
            },
            _ => self.fallback_chain.first(),
        }
    }
}
```

**Acceptance**: Context overflow → larger context model. Rate limit → different provider.
**Verification**: `cargo test -p roko-learn -- error_specific_fallback`

---

## F. Model Version Handling

### 2L.14 — Detect model version changes on router load

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: When `load_or_new()` loads persisted state, detect if model slugs have changed:

```rust
fn detect_version_changes(
    persisted_slugs: &[String],
    current_slugs: &[String],
) -> Vec<VersionChange> {
    let mut changes = Vec::new();
    let persisted_set: HashSet<_> = persisted_slugs.iter().collect();
    let current_set: HashSet<_> = current_slugs.iter().collect();

    for slug in &current_set {
        if !persisted_set.contains(slug) {
            // New model — check if it's a version update
            let prefix = slug.rsplit_once('-').map(|(p, _)| p).unwrap_or(slug);
            if let Some(old) = persisted_set.iter().find(|s| s.starts_with(prefix)) {
                changes.push(VersionChange::Upgraded { old: old.to_string(), new: slug.to_string() });
            } else {
                changes.push(VersionChange::Added(slug.to_string()));
            }
        }
    }
    for slug in &persisted_set {
        if !current_set.contains(slug) {
            changes.push(VersionChange::Removed(slug.to_string()));
        }
    }
    changes
}
```

When an upgrade is detected, transfer 50% of old model's stats to the new model (weighted transfer).

**Acceptance**: Loading state with `["glm-5"]` when config has `["glm-5.1"]` detects the upgrade.
**Verification**: `cargo test -p roko-learn -- version_change_detection`

---

## G. Concurrency: RwLock for Router

### 2L.15 — Switch LinUCBRouter from Mutex to RwLock

**File**: `crates/roko-learn/src/model_router.rs`
**What**: The routing path (read) vastly outnumbers observation recording (write). Use `RwLock` to allow concurrent routing:

```rust
pub struct LinUCBRouter {
    state: parking_lot::RwLock<RouterState>,  // was: Mutex<RouterState>
    // ...
}

impl LinUCBRouter {
    pub fn select_features(&self, x: &[f64]) -> ModelSpec {
        let state = self.state.read();  // concurrent readers OK
        // ... LinUCB scoring ...
    }

    pub fn update_features(&self, x: &[f64], model_idx: usize, reward: f64) {
        let mut state = self.state.write();  // exclusive for updates
        // ... matrix update ...
    }
}
```

**Context**: `parking_lot::RwLock` has very low overhead for uncontended reads. With 10 parallel tasks routing simultaneously, this eliminates serialization on the hot path.

**Acceptance**: 10 concurrent `select_features()` calls complete without blocking each other.
**Verification**: `cargo test -p roko-learn -- concurrent_routing`

---

## H. Build System Auto-Detection for Gates

### 2L.16 — Add auto-detection to gate pipeline

**File**: `crates/roko-gate/src/payload.rs` (extend existing `BuildSystem` enum)
**What**: The `BuildSystem` enum already has 6 variants. Add auto-detection:

```rust
impl BuildSystem {
    pub fn detect(workdir: &Path) -> Self {
        if workdir.join("Cargo.toml").exists() { return Self::Cargo; }
        if workdir.join("package.json").exists() { return Self::Npm; }
        if workdir.join("go.mod").exists() { return Self::Go; }
        if workdir.join("pyproject.toml").exists()
           || workdir.join("setup.py").exists() { return Self::Python; }
        if workdir.join("foundry.toml").exists() { return Self::Forge; }
        Self::Make  // fallback
    }
}
```

**Acceptance**: `detect()` correctly identifies Rust, Node, Go, Python, Solidity projects.
**Verification**: `cargo test -p roko-gate -- build_system_detect`

---

## Summary

| Section | Tasks | IDs | What |
|---|---|---|---|
| **A. Wire ToolLoop** | 5 | 2L.01–2L.05 | LlmBackend impl, factory, wrapper, checkpoints |
| **B. Token Counting** | 2 | 2L.06–2L.07 | TokenCounter, SystemPromptBuilder integration |
| **C. Rate Limiting** | 2 | 2L.08–2L.09 | governor crate, per-provider throttling |
| **D. MCP Bridge** | 2 | 2L.10–2L.11 | MCP→function conversion, handler resolver |
| **E. Fallback Chains** | 2 | 2L.12–2L.13 | Multi-level, error-type-specific fallbacks |
| **F. Version Handling** | 1 | 2L.14 | Detect upgrades, transfer stats |
| **G. Concurrency** | 1 | 2L.15 | RwLock for concurrent routing |
| **H. Build Detection** | 1 | 2L.16 | Auto-detect project language for gates |
| **Total** | **16** | **2L.01–2L.16** | |

## Critical Integration Detail: Cascade Router Hardcodes Claude Models

`LearningRuntime` constructor at `runtime_feedback.rs` L277–283 hardcodes:
```rust
CascadeRouter::load_or_new(&paths, vec![
    "claude-sonnet-4-20250514".into(),
    "claude-haiku-4-5-20251001".into(),
]);
```

This must change to load from config (see doc 19 Integration Point #4):
```rust
let slugs = config.effective_models().values().map(|m| m.slug.clone()).collect();
CascadeRouter::load_or_new(&paths, slugs);
```

Without this one-line change, the router never learns about non-Claude models regardless of
how many providers are configured. This is the linchpin for multi-model routing.

## Critical Correction to Doc 13

**Tasks 2K.05–2K.09 in doc 13 should be replaced by 2L.01–2L.04 in this doc.** The existing `ToolLoop` in `crates/roko-agent/src/tool_loop/mod.rs` already implements the multi-turn iteration logic with ToolDispatcher integration, context pruning, and checkpointing. We only need:
1. An `LlmBackend` impl for HTTP endpoints (2L.01)
2. A factory to create it from config (2L.02)
3. Wiring into the adapter (2L.03)
4. An Agent trait wrapper (2L.04)

Tasks 2K.01–2K.04 (ChatMessage, ChatRequest, ChatResponse) are still needed as they define the canonical types. Tasks 2K.10+ are also still valid.
