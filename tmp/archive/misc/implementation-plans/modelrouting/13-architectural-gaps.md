# 13 — Architectural Gaps: Tool Loop, Streaming, Events, Cache, Sessions

> **Priority**: 🔴 P0 — Without Gap A (ToolLoopRunner), HTTP backends cannot use tools at all
> **Status**: Not started
> **Depends on**: 02 (registry), 03 (adapters), 04 (translator extensions)
> **Blocks**: 05 (GLM fully agentic), 06 (Kimi fully agentic)

## Problem Statement

The provider adapter refactor (doc 03) creates agents that can send prompts and receive responses. But for HTTP backends (GLM-5.1, Kimi-K2.5, OpenRouter, Ollama), there is no mechanism to:

1. Parse tool calls from the response
2. Execute tools via ToolDispatcher
3. Build a follow-up request with tool results
4. Send it back to the model
5. Repeat until the model stops calling tools

Without this, HTTP backends can only do single-shot completions — no tool use, which defeats the purpose of integrating them as agentic backends. Claude CLI bypasses this because the `claude` binary owns the tool loop internally.

Additionally, the plans miss: cache layer alignment in prompt assembly, streaming response handling, event-driven architecture for learning loops, session continuity across turns, and prompt normalization for cache optimization.

## What Exists

| Component | Path | Lines | Status |
|---|---|---|---|
| Agent trait | `crates/roko-agent/src/agent.rs` | 94–112 | 🔌 Single-shot interface |
| ToolDispatcher | `crates/roko-agent/src/dispatcher/mod.rs` | ~1200 | 🔌 7-step pipeline |
| Translator trait | `crates/roko-agent/src/translate/mod.rs` | 54–81 | 🔌 Tool format conversion |
| OpenAiTranslator | `crates/roko-agent/src/translate/openai.rs` | ~532 | 🔌 parse_calls, render_results |
| tool_loop module | `crates/roko-agent/src/tool_loop.rs` | — | 🏗️ Exists, check if wired |
| SystemPromptBuilder | `crates/roko-compose/src/system_prompt_builder.rs` | — | 🔌 No cache layers |
| HandlerResolver trait | `crates/roko-agent/src/dispatcher/mod.rs` | — | 🔌 Tool handler lookup |
| Signal type | `crates/roko-core/src/signal.rs` | — | 🔌 Universal data type |

---

> **Cross-references from later rounds of research**:
> - Doc 14 supersedes 2K.05–2K.09 (ToolLoop already exists — wire, don't rebuild)
> - Doc 16 adds timeouts (2N.01–02), retry jitter (2N.03–04), concurrency semaphores (2N.05–06)
>   to the ToolLoop pipeline — these should be wired into the LlmBackend, not the ToolLoop itself
> - ProviderHealthTracker already exists (doc 08 correction, doc 16 note)
> - The `roko-cli/src/config.rs` Config struct (1,911 LOC) wraps `roko-core`'s RokoConfig —
>   new types from 2K.01–2K.04 need to be accessible from both

# Gap A: Multi-Turn Tool Loop for HTTP Backends

## Design

```
Orchestrator
    │
    ├── ClaudeCliAgent (CLI owns the loop — no change needed)
    │
    └── ToolLoopRunner (NEW — for all HTTP backends)
            │
            ├── ProviderAdapter.build_request() → WireRequest
            ├── HTTP POST → WireResponse
            ├── ProviderAdapter.parse_response() → ChatResponse
            │
            ├── if ChatResponse.tool_calls is empty → return
            │
            ├── ToolDispatcher.dispatch_batch(tool_calls) → results
            ├── Translator.render_results(results) → messages
            ├── Append assistant message + tool result messages
            └── Loop back to build_request()
```

The ToolLoopRunner is the **core runtime for agentic HTTP backends**. It is the equivalent of what the `claude` CLI binary does internally, but controlled by roko.

---

### 2K.01 — Define ChatMessage enum

> **TYPE PLACEMENT**: ChatMessage, ChatRequest, ChatResponse MUST go in **roko-core**, not roko-agent.
>
> Reason: roko-compose depends on roko-core but NOT roko-agent. If these types are in roko-agent,
> roko-compose can't use them for prompt assembly (building ChatRequest from prompt sections).
> Placing them in roko-core makes them available to ALL crates without circular deps.
>
> The dependency chain is: roko-cli → roko-serve → roko-agent → roko-core ← roko-compose ← roko-learn
> roko-core has ZERO roko dependencies. It's the kernel. Everything depends on it.
>
> Also note: `ChatRequest` and `ChatResponse` types already exist locally in `codex_agent.rs`
> (L69–91) and `ollama_agent.rs` — these should be replaced by the canonical types from roko-core.

**File**: `crates/roko-core/src/chat_types.rs` (new — in roko-core, NOT roko-agent)
**What**: Canonical message type used by the tool loop and provider adapters:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum ChatMessage {
    #[serde(rename = "system")]
    System { content: String },

    #[serde(rename = "user")]
    User { content: MessageContent },

    #[serde(rename = "assistant")]
    Assistant {
        content: Option<String>,
        reasoning_content: Option<String>,
        tool_calls: Option<Vec<ToolCallMessage>>,
        #[serde(default)]
        partial: bool,  // Kimi partial continuation
    },

    #[serde(rename = "tool")]
    Tool {
        tool_call_id: String,
        content: String,
    },
}

/// Content can be plain text or multimodal blocks (for Kimi vision).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,  // data:image/png;base64,... or https://...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMessage {
    pub id: String,
    pub r#type: String,  // "function"
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,  // JSON-stringified
}
```

**Context**: This is the canonical message format shared by the tool loop, provider adapters, and prompt assembly. It's modeled on OpenAI's format because both GLM-5.1 and Kimi-K2.5 use it natively.

**Acceptance**: All 4 message roles serialize/deserialize correctly. `Assistant` with `tool_calls` produces valid JSON. `User` with `Blocks` supports image content.
**Verification**: `cargo test -p roko-agent -- chat_message_serde`

---

### 2K.02 — Define ChatRequest struct

**File**: `crates/roko-agent/src/chat_types.rs`
**What**: Canonical request type:

```rust
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model_slug: String,
    pub tools: Vec<ToolDef>,
    pub tool_choice: ToolChoice,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub stop: Option<Vec<String>>,
    pub stream: bool,
    pub options: RequestOptions,
}

#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub enable_thinking: Option<bool>,
    pub preserve_thinking: Option<bool>,
    pub enable_tool_streaming: Option<bool>,
    pub cache_key: Option<String>,
    pub response_format: Option<ResponseFormat>,
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Specific { name: String },
}

#[derive(Debug, Clone)]
pub enum ResponseFormat {
    Text,
    JsonObject,
}
```

**Context**: The `extra` map in `RequestOptions` carries provider-specific params that don't fit canonical fields. The adapter's `extra_params()` method populates this.

**Acceptance**: `ChatRequest` can represent requests for all 4 provider types.
**Verification**: `cargo test -p roko-agent -- chat_request`

---

### 2K.03 — Extend ChatResponse (from 2C.02) with tool loop fields

**File**: `crates/roko-agent/src/chat_types.rs`
**What**: Extend the `ChatResponse` from doc 04 task 2C.02 with fields needed by the tool loop:

```rust
#[derive(Debug, Clone, Default)]
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
    pub metadata: ResponseMetadata,
    // Tool loop fields:
    pub raw_assistant_message: Option<ChatMessage>,  // For appending to history
    pub session: SessionState,
}

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub session_id: Option<String>,      // Claude --resume
    pub thread_id: Option<String>,       // Codex thread
    pub conversation_id: Option<String>, // Generic
}

impl ChatResponse {
    /// Convert the response into a ChatMessage::Assistant for appending to history.
    pub fn as_assistant_message(&self) -> ChatMessage {
        ChatMessage::Assistant {
            content: if self.content.is_empty() { None } else { Some(self.content.clone()) },
            reasoning_content: self.reasoning.clone(),
            tool_calls: if self.tool_calls.is_empty() {
                None
            } else {
                Some(self.tool_calls.iter().map(|tc| tc.to_message()).collect())
            },
            partial: false,
        }
    }
}
```

**Acceptance**: `ChatResponse::as_assistant_message()` produces valid `ChatMessage::Assistant` with tool calls preserved.
**Verification**: `cargo test -p roko-agent -- chat_response_to_message`

---

### 2K.04 — Implement Signal ↔ ChatRequest conversion

**File**: `crates/roko-agent/src/chat_types.rs`
**What**: Bridge between roko's universal `Signal` type and the chat types:

```rust
impl ChatRequest {
    /// Build a ChatRequest from a Signal (the current orchestrator interface).
    pub fn from_signal(
        signal: &Signal,
        model_slug: &str,
        system_prompt: Option<&str>,
        tools: Vec<ToolDef>,
        options: RequestOptions,
    ) -> Self {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage::System { content: sys.to_string() });
        }
        let prompt = signal.body.as_text().unwrap_or_default();
        messages.push(ChatMessage::User { content: MessageContent::Text(prompt) });
        ChatRequest {
            messages,
            model_slug: model_slug.to_string(),
            tools,
            tool_choice: ToolChoice::Auto,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            stream: false,
            options,
        }
    }
}

impl ChatResponse {
    /// Convert a ChatResponse back into a Signal for the orchestrator.
    pub fn to_signal(&self) -> Signal {
        Signal::text(&self.content)
            .with_tag("model", self.metadata.model_used.as_deref().unwrap_or("unknown"))
            .with_tag("finish_reason", &format!("{:?}", self.finish_reason))
    }
}
```

**Context**: This bridges the gap between roko's Signal-based orchestrator and the new ChatRequest-based tool loop. The orchestrator continues to work with Signals; the adapter layer works with ChatRequests.

**Acceptance**: Round-trip: Signal → ChatRequest → (adapter) → ChatResponse → Signal preserves content.
**Verification**: `cargo test -p roko-agent -- signal_chat_roundtrip`

---

### ~~2K.05–2K.09 — SUPERSEDED by 14-integration-refinements.md 2L.01–2L.05~~

> **CRITICAL CORRECTION**: A `ToolLoop` already exists at `crates/roko-agent/src/tool_loop/mod.rs`
> with full multi-turn iteration, ToolDispatcher integration, context pruning, and checkpointing.
> Tasks 2K.05–2K.09 would have rebuilt it from scratch. See doc 14 (2L.01–2L.05) for the
> corrected tasks that wire the EXISTING ToolLoop via new `LlmBackend` implementations.
>
> Tasks 2K.01–2K.04 (ChatMessage/ChatRequest/ChatResponse) are still needed.
> Tasks 2K.10+ are still valid.

### ~~2K.05~~ — ~~Define ToolLoopRunner struct~~ → SEE 2L.01

**File**: ~~`crates/roko-agent/src/tool_loop_runner.rs` (new)~~ → Use existing `crates/roko-agent/src/tool_loop/mod.rs`
**What**: ~~The core multi-turn tool loop for HTTP backends:~~ Implement `LlmBackend` trait for HTTP endpoints instead.

```rust
pub struct ToolLoopRunner {
    pub dispatcher: Arc<ToolDispatcher>,
    pub translator: Arc<dyn Translator>,
    pub poster: Arc<dyn HttpPoster>,
    pub provider_config: ProviderConfig,
    pub model_profile: ModelProfile,
    pub max_turns: u32,           // default: 50
    pub max_tokens_budget: u64,   // total tokens before abort
    pub event_tx: Option<mpsc::UnboundedSender<AgentEvent>>,
}

pub struct ToolLoopResult {
    pub final_response: ChatResponse,
    pub turn_count: u32,
    pub total_usage: Usage,
    pub tool_calls_executed: Vec<(ToolCall, ToolResult)>,
    pub session: SessionState,
}
```

**Acceptance**: Struct compiles with all fields.
**Verification**: `cargo check -p roko-agent`

---

### 2K.06 — Implement the tool loop

**File**: `crates/roko-agent/src/tool_loop_runner.rs`
**What**: The actual loop implementation:

```rust
impl ToolLoopRunner {
    pub async fn run(&self, mut request: ChatRequest) -> Result<ToolLoopResult, ToolLoopError> {
        let mut total_usage = Usage::default();
        let mut all_tool_results = Vec::new();
        let mut session = SessionState::default();

        for turn in 0..self.max_turns {
            // 1. Inject provider-specific params
            let extra = inject_model_params(&self.model_profile, &request.options);
            request.options.extra.extend(extra);

            // 2. Build wire request
            let wire_request = build_openai_request(&request, &self.provider_config)?;

            // 3. Send HTTP request
            let wire_response = self.poster.post(
                &self.endpoint(),
                &self.headers(),
                &wire_request,
            ).await?;

            // 4. Parse response
            let response = parse_openai_response(&wire_response, &self.model_profile)?;
            total_usage = total_usage.merge(&response.usage);
            session = response.session.clone();

            // 5. Emit event
            if let Some(ref tx) = self.event_tx {
                let _ = tx.send(AgentEvent::TurnCompleted {
                    turn, usage: response.usage.clone(), tool_call_count: response.tool_calls.len(),
                });
            }

            // 6. Check if done (no tool calls)
            if response.tool_calls.is_empty() {
                return Ok(ToolLoopResult {
                    final_response: response,
                    turn_count: turn + 1,
                    total_usage,
                    tool_calls_executed: all_tool_results,
                    session,
                });
            }

            // 7. Execute tool calls via ToolDispatcher
            let parsed_calls = self.translator.parse_calls(
                &BackendResponse::Json(serde_json::to_value(&wire_response)?)
            )?;
            let results = self.dispatcher.dispatch_batch(&parsed_calls).await?;

            // 8. Append assistant message + tool results to conversation
            request.messages.push(response.as_assistant_message());
            let rendered = self.translator.render_results(&results);
            if let RenderedResults::JsonMessages(msgs) = rendered {
                // Parse rendered messages back into ChatMessage::Tool entries
                for msg in msgs.as_array().unwrap_or(&vec![]) {
                    request.messages.push(ChatMessage::Tool {
                        tool_call_id: msg["tool_call_id"].as_str().unwrap_or("").to_string(),
                        content: msg["content"].as_str().unwrap_or("").to_string(),
                    });
                }
            }

            all_tool_results.extend(results.into_iter().map(|(c, r)| (c, r)));

            // 9. Check token budget
            if total_usage.total_tokens() > self.max_tokens_budget {
                return Err(ToolLoopError::TokenBudgetExhausted {
                    used: total_usage.total_tokens(),
                    budget: self.max_tokens_budget,
                });
            }
        }

        Err(ToolLoopError::MaxTurnsExhausted { turns: self.max_turns })
    }

    fn endpoint(&self) -> String {
        let base = self.provider_config.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        format!("{}/chat/completions", base.trim_end_matches('/'))
    }

    fn headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![];
        if let Some(key) = self.provider_config.resolve_api_key() {
            headers.push(("Authorization".to_string(), format!("Bearer {}", key)));
        }
        if let Some(ref extra) = self.provider_config.extra_headers {
            for (k, v) in extra {
                headers.push((k.clone(), v.clone()));
            }
        }
        headers
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolLoopError {
    #[error("Max turns exhausted ({turns})")]
    MaxTurnsExhausted { turns: u32 },
    #[error("Token budget exhausted (used {used}, budget {budget})")]
    TokenBudgetExhausted { used: u64, budget: u64 },
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Tool dispatch error: {0}")]
    ToolDispatch(String),
}
```

**Context**: This is the core of what makes HTTP backends work as agentic systems. Without it, GLM-5.1 and Kimi-K2.5 return tool_calls that are never executed.

**Acceptance**: A mock HTTP server that returns tool_calls on turn 1 and final content on turn 2 produces a `ToolLoopResult` with 2 turns and 1 tool call executed.
**Verification**: `cargo test -p roko-agent -- tool_loop_basic`

---

### 2K.07 — Preserve reasoning_content across loop turns

**File**: `crates/roko-agent/src/tool_loop_runner.rs`
**What**: Both GLM-5.1 and Kimi-K2.5 require that `reasoning_content` from previous assistant turns is included unmodified in subsequent requests. The `as_assistant_message()` method already preserves it. Verify this with a test.

**Context**: Per GLM docs: "the `reasoning_content` field from the preceding assistant message must be included unmodified in the conversation history." Per Kimi docs: "include the entire `reasoning_content` from previous turns for coherence."

**Acceptance**: Conversation history after 3 turns with thinking contains all 3 reasoning_content values.
**Verification**: `cargo test -p roko-agent -- reasoning_preservation`

---

### 2K.08 — Implement ToolLoopRunner as an Agent impl

**File**: `crates/roko-agent/src/tool_loop_runner.rs`
**What**: Wrap `ToolLoopRunner` in the `Agent` trait so it can be used by the existing orchestrator:

```rust
pub struct ToolLoopAgent {
    runner: ToolLoopRunner,
    system_prompt: Option<String>,
    tools: Vec<ToolDef>,
    options: RequestOptions,
    name: String,
}

#[async_trait]
impl Agent for ToolLoopAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        let request = ChatRequest::from_signal(
            input, &self.runner.model_profile.slug,
            self.system_prompt.as_deref(), self.tools.clone(), self.options.clone(),
        );
        match self.runner.run(request).await {
            Ok(result) => {
                let output = result.final_response.to_signal();
                AgentResult::ok(output)
                    .with_usage(result.total_usage)
            },
            Err(e) => AgentResult::fail(Signal::text(&e.to_string())),
        }
    }

    fn name(&self) -> &str { &self.name }
    fn supports_streaming(&self) -> bool { false }
}
```

**Context**: This is the bridge between the new tool loop and the existing orchestrator. The orchestrator calls `agent.run()` and gets back an `AgentResult`, regardless of whether it's a Claude CLI agent or a ToolLoopAgent running GLM-5.1.

**Acceptance**: `ToolLoopAgent` implements `Agent` trait. Orchestrator can use it interchangeably with `ClaudeCliAgent`.
**Verification**: `cargo test -p roko-agent -- tool_loop_agent_trait`

---

### 2K.09 — Update OpenAiCompatAdapter to create ToolLoopAgent

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: When the model supports tools (`ModelProfile.supports_tools = true`), create a `ToolLoopAgent` instead of a bare `CodexAgent`:

```rust
fn create_agent(&self, provider: &ProviderConfig, model: &ModelProfile, options: &AgentOptions)
    -> Result<Box<dyn Agent>, AgentCreationError>
{
    if model.supports_tools {
        // Create ToolLoopAgent with full tool dispatch
        let dispatcher = ToolDispatcher::new(/* handler_resolver, safety_layer */);
        let translator = translator_for(&model.slug);
        let runner = ToolLoopRunner {
            dispatcher: Arc::new(dispatcher),
            translator,
            poster: Arc::new(ReqwestPoster::new()),
            provider_config: provider.clone(),
            model_profile: model.clone(),
            max_turns: 50,
            max_tokens_budget: model.context_window.unwrap_or(128000),
            event_tx: None,
        };
        Ok(Box::new(ToolLoopAgent::new(runner, options)))
    } else {
        // Fallback: single-shot CodexAgent
        let agent = CodexAgent::new(/* ... */);
        Ok(Box::new(agent))
    }
}
```

**Acceptance**: GLM-5.1 config with `supports_tools = true` produces a `ToolLoopAgent`. Model without tool support produces a `CodexAgent`.
**Verification**: `cargo test -p roko-agent -- adapter_creates_tool_loop`

---

### 2K.10 — Write integration test: GLM-5.1 multi-turn tool loop

**File**: `crates/roko-agent/tests/tool_loop_integration.rs` (new)
**What**: End-to-end test with mock HTTP server:

```
Turn 1: Send "Read the file src/lib.rs"
  Mock returns: { tool_calls: [{name: "Read", args: {path: "src/lib.rs"}}], reasoning_content: "I need to read..." }
  ToolDispatcher executes Read → returns file content
  Append assistant message + tool result to conversation

Turn 2: Send conversation with tool result
  Mock returns: { content: "The file contains...", tool_calls: [], reasoning_content: "Based on the file..." }
  Loop exits.

Verify:
  - 2 HTTP requests were made
  - reasoning_content from turn 1 is in turn 2's request
  - Final content is "The file contains..."
  - Total usage is sum of both turns
  - 1 tool call executed (Read)
```

**Acceptance**: All 5 verifications pass.
**Verification**: `cargo test -p roko-agent -- tool_loop_glm_e2e`

---

# Gap B: Cache Layer Alignment in Prompt Assembly

### 2K.11 — Add cache_layer field to prompt section types

**File**: `crates/roko-compose/src/system_prompt_builder.rs`
**What**: Add cache layer tagging to prompt sections. Check the existing `PromptSection` or equivalent struct and add:

```rust
pub struct PromptSection {
    // ... existing fields (name, content, priority, token_estimate) ...
    pub cache_layer: CacheLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLayer {
    Role = 1,        // System prompt, role instructions, tool defs — stable across all tasks
    Workspace = 2,   // Workspace map, cross-plan context — stable across plan
    Plan = 3,        // Plan content, PRD extract, brief — stable within plan
    Volatile = 0,    // Task TOML, review feedback, error output — unique per turn
}
```

**Context**: From mori-agents. Section ordering must match cache layer ordering (Role first → Workspace → Plan → Volatile). If volatile content appears before stable content, the entire KV cache after it is invalidated. On GLM-5.1, proper ordering means $0.26/M (cache hit) vs $1.40/M (cache miss) — 5.4x cost difference.

**Acceptance**: `CacheLayer` is `Ord` so sections can be sorted by it. `Role < Workspace < Plan` ordering.
**Verification**: `cargo test -p roko-compose -- cache_layer_ordering`

---

### 2K.12 — Sort sections by cache_layer in SystemPromptBuilder

**File**: `crates/roko-compose/src/system_prompt_builder.rs`
**What**: Modify the prompt assembly to sort sections by cache_layer first, then by priority within each layer:

```rust
fn assemble_sections(mut sections: Vec<PromptSection>, budget: usize) -> String {
    // Primary sort: cache_layer ascending (Role=1 first, Volatile=0 last)
    // Secondary sort: priority descending (highest priority first within layer)
    sections.sort_by(|a, b| {
        b.cache_layer.cmp(&a.cache_layer)  // Role before Workspace before Plan
            .then(b.priority.cmp(&a.priority))  // Then highest priority first
    });

    // Assemble within budget, dropping lowest-priority Volatile sections first
    // ...
}
```

**Context**: This is the single highest-ROI change for cost optimization. All providers with automatic prefix caching (GLM, Kimi, OpenAI) benefit from stable prefixes.

**Acceptance**: System prompt + role instructions always appear before task-specific content.
**Verification**: `cargo test -p roko-compose -- section_cache_order`

---

### 2K.13 — Add cache control markers for Anthropic

**File**: `crates/roko-agent/src/translate/claude.rs`
**What**: When building requests for Anthropic's API, inject `cache_control` markers at cache layer boundaries:

```rust
fn inject_cache_markers(messages: &mut Vec<Value>) {
    // After the last Role-layer message, insert cache_control: {type: "ephemeral"}
    // After the last Workspace-layer message, insert cache_control
    // This tells Anthropic's API where to cache
}
```

**Context**: Anthropic's prompt caching requires explicit `cache_control` markers. Other providers (GLM, Kimi, OpenAI) cache automatically based on prefix matching. The cache markers are only injected for Anthropic; other providers ignore them.

**Acceptance**: Anthropic requests include `cache_control` at layer boundaries. Non-Anthropic requests are unaffected.
**Verification**: `cargo test -p roko-agent -- anthropic_cache_markers`

---

### 2K.14 — Implement prompt normalization for cache stability

**File**: `crates/roko-compose/src/system_prompt_builder.rs`
**What**: Normalize prompt content to maximize cache prefix matches:

```rust
pub fn normalize_for_caching(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim_end())           // Remove trailing whitespace
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\r\n", "\n")                  // Normalize line endings
        .replace("\t", "    ")                   // Normalize tabs to spaces
}

pub fn canonical_tool_order(tools: &mut [ToolDef]) {
    tools.sort_by(|a, b| a.name().cmp(b.name()));  // Alphabetical tool ordering
}
```

**Context**: Two prompts that differ by a single whitespace character will NOT share a cache prefix. Tool definition ordering varies between requests, breaking cache prefixes. Normalization ensures identical content produces identical bytes.

**Acceptance**: Same logical prompt with different whitespace/tool ordering produces identical normalized output.
**Verification**: `cargo test -p roko-compose -- prompt_normalization`

---

### 2K.15 — Write test: verify cache prefix stability across tasks

**File**: `crates/roko-compose/tests/cache_stability.rs` (new)
**What**: Generate prompts for 3 consecutive tasks in the same plan. Verify that the first N bytes (covering Role + Workspace layers) are identical:

```rust
#[test]
fn cache_prefix_stable_across_tasks() {
    let prompt_1 = build_prompt(task_1, plan, role);
    let prompt_2 = build_prompt(task_2, plan, role);
    let prompt_3 = build_prompt(task_3, plan, role);

    // Role layer (system prompt + instructions) identical
    let role_len = prompt_1.find("## Plan Context").unwrap();
    assert_eq!(&prompt_1[..role_len], &prompt_2[..role_len]);
    assert_eq!(&prompt_1[..role_len], &prompt_3[..role_len]);

    // Workspace layer (workspace map) identical
    let ws_len = prompt_1.find("## Task").unwrap();
    assert_eq!(&prompt_1[..ws_len], &prompt_2[..ws_len]);
}
```

**Acceptance**: First ~60% of prompt is byte-identical across tasks in same plan.
**Verification**: `cargo test -p roko-compose -- cache_prefix_stable`

---

# Gap C: Streaming Response Pipeline

### 2K.16 — Define StreamChunk enum

**File**: `crates/roko-agent/src/streaming.rs` (new)
**What**: Typed stream events for incremental processing:

```rust
#[derive(Debug, Clone)]
pub enum StreamChunk {
    ReasoningDelta(String),
    ContentDelta(String),
    ToolCallDelta {
        index: usize,
        id_delta: Option<String>,
        name_delta: Option<String>,
        arguments_delta: String,
    },
    Usage(Usage),
    Done(FinishReason),
    Error(String),
}
```

**Acceptance**: All variants cover GLM + Kimi streaming formats.
**Verification**: `cargo test -p roko-agent -- stream_chunk_types`

---

### 2K.17 — Implement SSE parser for OpenAI-compatible streaming

**File**: `crates/roko-agent/src/streaming.rs`
**What**: Parse Server-Sent Events into `StreamChunk`:

```rust
pub fn parse_sse_line(line: &str) -> Option<StreamChunk> {
    let line = line.strip_prefix("data: ")?;
    if line == "[DONE]" { return Some(StreamChunk::Done(FinishReason::Stop)); }

    let json: Value = serde_json::from_str(line).ok()?;
    let delta = &json["choices"][0]["delta"];

    // Check for reasoning_content first (streams before content)
    if let Some(reasoning) = delta["reasoning_content"].as_str() {
        return Some(StreamChunk::ReasoningDelta(reasoning.to_string()));
    }
    if let Some(content) = delta["content"].as_str() {
        return Some(StreamChunk::ContentDelta(content.to_string()));
    }
    if let Some(tool_calls) = delta["tool_calls"].as_array() {
        for tc in tool_calls {
            let index = tc["index"].as_u64().unwrap_or(0) as usize;
            return Some(StreamChunk::ToolCallDelta {
                index,
                id_delta: tc["id"].as_str().map(|s| s.to_string()),
                name_delta: tc["function"]["name"].as_str().map(|s| s.to_string()),
                arguments_delta: tc["function"]["arguments"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    // Check for usage in final chunk
    if let Some(usage) = json.get("usage") {
        return Some(StreamChunk::Usage(parse_usage(usage)));
    }

    None
}
```

**Acceptance**: Parses GLM stream with reasoning → content → tool_calls phases. Handles `[DONE]`.
**Verification**: `cargo test -p roko-agent -- sse_parser`

---

### 2K.18 — Implement streaming accumulator

**File**: `crates/roko-agent/src/streaming.rs`
**What**: Accumulate stream chunks into a complete `ChatResponse`:

```rust
pub struct StreamAccumulator {
    reasoning: String,
    content: String,
    tool_calls: Vec<PartialToolCall>,
    usage: Usage,
    finish_reason: FinishReason,
}

struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,  // accumulated JSON string
}

impl StreamAccumulator {
    pub fn push(&mut self, chunk: StreamChunk) {
        match chunk {
            StreamChunk::ReasoningDelta(s) => self.reasoning.push_str(&s),
            StreamChunk::ContentDelta(s) => self.content.push_str(&s),
            StreamChunk::ToolCallDelta { index, id_delta, name_delta, arguments_delta } => {
                while self.tool_calls.len() <= index {
                    self.tool_calls.push(PartialToolCall::default());
                }
                let tc = &mut self.tool_calls[index];
                if let Some(id) = id_delta { tc.id = id; }
                if let Some(name) = name_delta { tc.name = name; }
                tc.arguments.push_str(&arguments_delta);
            },
            StreamChunk::Usage(u) => self.usage = u,
            StreamChunk::Done(r) => self.finish_reason = r,
            StreamChunk::Error(_) => {},
        }
    }

    pub fn finalize(self) -> ChatResponse { /* build ChatResponse from accumulated state */ }
}
```

**Acceptance**: 100 stream chunks accumulate into a correct ChatResponse with reasoning, content, and tool calls.
**Verification**: `cargo test -p roko-agent -- stream_accumulator`

---

### 2K.19 — Add streaming support to ToolLoopRunner

**File**: `crates/roko-agent/src/tool_loop_runner.rs`
**What**: When `request.stream = true`, use streaming HTTP and emit events in real-time:

```rust
impl ToolLoopRunner {
    pub async fn run_streaming(
        &self,
        mut request: ChatRequest,
        event_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<ToolLoopResult, ToolLoopError> {
        request.stream = true;
        // ... same loop structure as run(), but:
        // 1. Use streaming HTTP POST
        // 2. Parse SSE lines as they arrive
        // 3. Forward StreamChunks to event_tx
        // 4. Accumulate into ChatResponse when Done
        // 5. Execute tools and continue loop
    }
}
```

**Context**: Enables real-time dashboard updates and intra-turn anomaly detection. For GLM's 8-hour sessions, non-streaming would mean waiting hours for each turn.

**Acceptance**: Stream chunks are emitted in real-time. Final result matches non-streaming.
**Verification**: `cargo test -p roko-agent -- streaming_tool_loop`

---

# Gap D: Event Fabric

### 2K.20 — Define AgentEvent enum

**File**: `crates/roko-learn/src/events.rs` (new)
**What**: Unified event type for all learning systems:

```rust
#[derive(Debug, Clone)]
pub enum AgentEvent {
    TurnStarted { task_id: String, model: String, provider: String, timestamp_ms: i64 },
    ToolCallExecuted { tool_name: String, duration_ms: u64, success: bool, result_tokens: u64 },
    TurnCompleted { turn: u32, usage: Usage, tool_call_count: usize, gate_passed: Option<bool>, finish_reason: FinishReason },
    GateResult { gate_name: String, passed: bool, score: f32, duration_ms: u64 },
    ProviderError { provider_id: String, error_class: ErrorClass, status: u16 },
    CostRecorded { model: String, provider: String, cost_usd: f64, tokens: u64 },
    AnomalyDetected { anomaly: Anomaly },
    ExperimentAssigned { experiment_id: String, variant_id: String },
    SessionEstablished { session_id: String, provider: String },
    ModelSelected { model: String, stage: String, score: f64 },
    StreamChunk { chunk: StreamChunk },
}
```

**Acceptance**: All event variants compile.
**Verification**: `cargo test -p roko-learn -- agent_event_types`

---

### 2K.21 — Implement EventBus with broadcast channel

**File**: `crates/roko-learn/src/events.rs`
**What**: Pub/sub event bus using tokio broadcast:

```rust
pub struct EventBus {
    tx: tokio::sync::broadcast::Sender<AgentEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: AgentEvent) {
        let _ = self.tx.send(event);  // Ignore if no subscribers
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AgentEvent> {
        self.tx.subscribe()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self { Self { tx: self.tx.clone() } }
}
```

**Acceptance**: Multiple subscribers receive the same event. Publishing with no subscribers doesn't error.
**Verification**: `cargo test -p roko-learn -- event_bus`

---

### 2K.22 — Create event-driven learning subscriber

**File**: `crates/roko-learn/src/event_subscriber.rs` (new)
**What**: A single subscriber that routes events to all learning systems:

```rust
pub async fn run_learning_subscriber(
    mut rx: tokio::sync::broadcast::Receiver<AgentEvent>,
    health: Arc<ProviderHealthRegistry>,
    latency: Arc<LatencyRegistry>,
    router: Arc<CascadeRouter>,
    anomaly: Arc<Mutex<AnomalyDetector>>,
    costs: Arc<CostsDb>,
    efficiency_path: PathBuf,
) {
    while let Ok(event) = rx.recv().await {
        match &event {
            AgentEvent::TurnCompleted { usage, gate_passed, .. } => {
                // Update router, costs, efficiency
            },
            AgentEvent::ProviderError { provider_id, error_class, .. } => {
                health.record_failure(provider_id, *error_class);
            },
            AgentEvent::ToolCallExecuted { duration_ms, .. } => {
                // Update latency stats
            },
            AgentEvent::CostRecorded { cost_usd, .. } => {
                anomaly.lock().unwrap().check_cost(*cost_usd);
            },
            _ => {},
        }
    }
}
```

**Context**: This replaces the explicit learning calls scattered in orchestrate.rs. Instead of `health.record(); latency.record(); router.observe(); ...` after each turn, the orchestrator publishes one event and the subscriber distributes it.

**Acceptance**: Publishing a `TurnCompleted` event updates router, costs, and efficiency.
**Verification**: `cargo test -p roko-learn -- event_subscriber`

---

### 2K.23 — Wire EventBus into orchestrate.rs and dispatch.rs

**File**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-serve/src/dispatch.rs`
**What**: Replace explicit learning system calls with event publishing:

```rust
// Before (current):
provider_health.record_success(provider_id);
latency_registry.record(model, provider, ttft, total, tokens);
anomaly_detector.check_cost(cost);
cascade_router.record_observation(ctx, slug, reward, success);

// After (event-driven):
event_bus.publish(AgentEvent::TurnCompleted { ... });
// The subscriber handles distribution to all systems
```

**Acceptance**: Removing explicit calls and using event_bus produces identical learning outcomes.
**Verification**: `cargo test -p roko-cli -- event_bus_wiring`

---

# Gap E: Session Continuity

### 2K.24 — Extract session state from provider responses

**File**: `crates/roko-agent/src/tool_loop_runner.rs`
**What**: After each turn, extract session/thread IDs from the response:

```rust
fn extract_session(response: &Value, provider_kind: ProviderKind) -> SessionState {
    match provider_kind {
        ProviderKind::ClaudeCli => SessionState {
            session_id: response.pointer("/session_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            ..Default::default()
        },
        ProviderKind::OpenAiCompat => SessionState {
            conversation_id: response.pointer("/id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            ..Default::default()
        },
        _ => SessionState::default(),
    }
}
```

**Context**: Session IDs enable the provider to reuse server-side KV cache across turns. Without session reuse, each turn resends the full conversation history.

**Acceptance**: Session ID extracted from GLM/Kimi responses when present.
**Verification**: `cargo test -p roko-agent -- session_extraction`

---

### 2K.25 — Pass session state back into subsequent requests

**File**: `crates/roko-agent/src/tool_loop_runner.rs`
**What**: In the tool loop, pass session state from the previous response into the next request:

The session ID doesn't go into the request body for OpenAI-compat (it's implicit via conversation history). But for providers that support it, it can be passed as a header or query parameter. The `RequestOptions.extra` field carries provider-specific session hints.

For Claude CLI (not in the tool loop), session reuse is via `--resume <session-id>`, which is already handled by `ClaudeCliAgent.resume`.

**Acceptance**: ToolLoopRunner preserves session state across turns within a single `run()` call.
**Verification**: `cargo test -p roko-agent -- session_continuity`

---

# Gap F: Learned Conductor (Intervention Policy)

### 2K.26 — Define ConductorPolicy as contextual bandit

**File**: `crates/roko-learn/src/conductor.rs` (new)
**What**: Replace hardcoded intervention heuristics with a learned policy:

```rust
#[derive(Debug, Clone)]
pub struct ConductorState {
    pub iteration: u32,
    pub consecutive_failures: u32,
    pub error_pattern: ErrorPattern,
    pub elapsed_ms: u64,
    pub cost_so_far_usd: f64,
    pub model_tier: String,
    pub task_complexity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorAction {
    Continue,
    InjectHint(HintType),
    SwitchModel,
    Restart,
    Abort,
}

#[derive(Debug, Clone, Copy)]
pub enum HintType {
    ErrorDigest,       // Inject enriched error summary
    SkillSuggestion,   // Inject relevant skill from library
    SimplifyApproach,  // Suggest simpler implementation
}

pub struct ConductorBandit {
    arms: HashMap<ConductorAction, ThompsonArm>,
    context_dim: usize,
}

impl ConductorBandit {
    pub fn select_action(&self, state: &ConductorState) -> ConductorAction {
        // Encode state as feature vector
        // Thompson sample from each action arm
        // Return highest-scoring action
    }

    pub fn record_outcome(&mut self, state: &ConductorState, action: ConductorAction, success: bool) {
        // Update the arm that was selected
    }
}
```

**Context**: From mori-refactor: "Replace hardcoded heuristics with learned policy. State = (iteration, errors, time, cost, model, complexity). Action = continue/nudge/restart/abort. Reward = task outcome."

**Acceptance**: After 100 observations, conductor learns that "abort after 3 failures on mechanical tasks" is better than continuing.
**Verification**: `cargo test -p roko-learn -- conductor_bandit`

---

### 2K.27 — Wire ConductorBandit into dispatch pipeline

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: After each failed gate, consult the conductor bandit:

```rust
if !gate_passed {
    let state = ConductorState {
        iteration, consecutive_failures, error_pattern, elapsed_ms, cost_so_far_usd,
        model_tier: model.tier().label().to_string(),
        task_complexity: task.complexity.label().to_string(),
    };
    let action = conductor.select_action(&state);
    match action {
        ConductorAction::Continue => { /* retry with same model */ },
        ConductorAction::InjectHint(hint) => { /* enrich next prompt */ },
        ConductorAction::SwitchModel => { /* escalate to higher tier */ },
        ConductorAction::Restart => { /* reset iteration, fresh attempt */ },
        ConductorAction::Abort => { /* mark task as failed, move on */ },
    }
}
```

**Persistence**: `.roko/learn/conductor.json`

**Acceptance**: Conductor makes intervention decisions based on learned policy, not hardcoded thresholds.
**Verification**: `cargo test -p roko-cli -- conductor_wiring`

---

# Gap G: TaskRunner Pipeline

### 2K.28 — Define TaskRunner struct

**File**: `crates/roko-agent/src/task_runner.rs` (new)
**What**: Composition point that integrates all pipeline components:

```rust
pub struct TaskRunner {
    pub agent: Box<dyn Agent>,           // ClaudeCliAgent or ToolLoopAgent
    pub event_bus: EventBus,
    pub anomaly: AnomalyDetector,
    pub budget: BudgetGuardrail,
    pub conductor: ConductorBandit,
    pub cost_table: CostTable,
    pub model_slug: String,
    pub provider_id: String,
    pub max_iterations: u32,
}

pub struct TaskResult {
    pub output: Signal,
    pub total_usage: Usage,
    pub total_cost_usd: f64,
    pub iterations: u32,
    pub gate_passed: bool,
    pub conductor_actions: Vec<ConductorAction>,
}
```

**Acceptance**: TaskRunner compiles with all fields.
**Verification**: `cargo check -p roko-agent`

---

### 2K.29 — Implement TaskRunner::run_task()

**File**: `crates/roko-agent/src/task_runner.rs`
**What**: The full task execution pipeline:

```rust
impl TaskRunner {
    pub async fn run_task(&mut self, task_signal: &Signal, ctx: &Context) -> Result<TaskResult, TaskRunnerError> {
        let mut iterations = 0;
        let mut total_usage = Usage::default();
        let mut conductor_actions = Vec::new();

        loop {
            iterations += 1;

            // 1. Budget check
            let budget_action = self.budget.record_cost(total_usage.cost_usd as f64, "task");
            if matches!(budget_action, BudgetAction::Block) {
                return Err(TaskRunnerError::BudgetExhausted);
            }

            // 2. Anomaly check
            let prompt_hash = hash_signal(task_signal);
            if let Some(anomaly) = self.anomaly.check_prompt(prompt_hash) {
                self.event_bus.publish(AgentEvent::AnomalyDetected { anomaly: anomaly.clone() });
                return Err(TaskRunnerError::Anomaly(anomaly));
            }

            // 3. Publish turn start
            self.event_bus.publish(AgentEvent::TurnStarted {
                task_id: ctx.task_id.clone(),
                model: self.model_slug.clone(),
                provider: self.provider_id.clone(),
                timestamp_ms: now_ms(),
            });

            // 4. Run agent
            let result = self.agent.run(task_signal, ctx).await;
            total_usage = total_usage.merge(&result.usage);

            // 5. Calculate cost
            let cost = self.cost_table.calculate(&self.model_slug, &result.usage);

            // 6. Publish turn completed
            self.event_bus.publish(AgentEvent::TurnCompleted {
                turn: iterations,
                usage: result.usage.clone(),
                tool_call_count: 0,
                gate_passed: None,
                finish_reason: FinishReason::Stop,
            });

            // 7. Run gates (caller provides gate pipeline)
            // ... gate execution happens in orchestrator, not here ...

            // 8. If gate passed, return success
            if result.success {
                return Ok(TaskResult {
                    output: result.output,
                    total_usage,
                    total_cost_usd: cost,
                    iterations,
                    gate_passed: true,
                    conductor_actions,
                });
            }

            // 9. Consult conductor for intervention
            let conductor_state = ConductorState { /* ... */ };
            let action = self.conductor.select_action(&conductor_state);
            conductor_actions.push(action);

            match action {
                ConductorAction::Abort => {
                    return Ok(TaskResult { gate_passed: false, /* ... */ });
                },
                ConductorAction::SwitchModel => {
                    // Escalate model (handled by caller)
                    return Err(TaskRunnerError::ModelEscalation);
                },
                _ => { /* Continue or inject hint — loop continues */ },
            }

            if iterations >= self.max_iterations {
                return Ok(TaskResult { gate_passed: false, /* ... */ });
            }
        }
    }
}
```

**Acceptance**: TaskRunner orchestrates the full cycle: budget → anomaly → agent → events → conductor.
**Verification**: `cargo test -p roko-agent -- task_runner_pipeline`

---

### 2K.30 — Wire TaskRunner into orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Replace the ad-hoc agent execution in orchestrate.rs with `TaskRunner::run_task()`:

```rust
// Before: ~50 lines of explicit agent creation, budget checking, anomaly checking, event publishing
// After:
let mut runner = TaskRunner {
    agent: create_agent_for_model(config, model_key, options)?,
    event_bus: event_bus.clone(),
    anomaly: anomaly_detector.clone(),
    budget: budget_guardrail.clone(),
    conductor: conductor.clone(),
    cost_table: cost_table.clone(),
    model_slug: resolved.slug.clone(),
    provider_id: resolved.provider_kind.label().to_string(),
    max_iterations: config.agent.escalation.max_retries,
};
let result = runner.run_task(&task_signal, &ctx).await?;
```

**Acceptance**: Orchestrate.rs uses TaskRunner instead of inline logic. Behavior is identical.
**Verification**:
```bash
cargo run -p roko-cli -- run "echo test" 2>&1 | head -5
# Should produce same output as before
```

---

# Gap H: Generated Test Gates (Verification Investment)

### 2K.31 — Define GeneratedGate trait

**File**: `crates/roko-gate/src/generated.rs` (new)
**What**: Gates generated from acceptance criteria, not hand-written:

```rust
pub trait GateGenerator: Send + Sync {
    /// Generate verification artifacts from acceptance criteria.
    fn generate(
        &self,
        acceptance_criteria: &str,
        task_context: &str,
    ) -> Result<Vec<GeneratedCheck>, GateError>;
}

#[derive(Debug, Clone)]
pub enum GeneratedCheck {
    SymbolExists {
        name: String,
        kind: String,        // "struct", "fn", "trait", "enum"
        visibility: String,  // "pub", "pub(crate)", ""
        module_path: String,
    },
    TestCase {
        name: String,
        code: String,        // Complete #[test] fn
        rung: u32,           // Which gate rung (3=behavioral, 4=property)
    },
}
```

**Context**: From mori-agents' 6-rung gate architecture and GVU theory ("strengthen the verifier"). The implementer agent NEVER sees these generated checks — they're used only by the gate runner.

**Acceptance**: `GeneratedCheck` can represent symbol checks and test cases.
**Verification**: `cargo test -p roko-gate -- generated_check_types`

---

### 2K.32 — Implement symbol existence gate

**File**: `crates/roko-gate/src/generated.rs`
**What**: Check that expected symbols exist in the codebase:

```rust
pub fn check_symbol_exists(check: &GeneratedCheck, workspace: &Path) -> Verdict {
    match check {
        GeneratedCheck::SymbolExists { name, kind, visibility, module_path } => {
            // Use grep or tree-sitter to find the symbol
            // Check visibility matches
            // Return pass/fail with location
        },
        _ => Verdict::skip("not a symbol check"),
    }
}
```

**Context**: This is rung 2 in mori's 6-rung system. Catches: missing exports, renamed types, incorrect visibility. Fast (~10ms) because it's a text search, not compilation.

**Acceptance**: Missing `pub struct Foo` produces a failed verdict with the expected location.
**Verification**: `cargo test -p roko-gate -- symbol_existence_gate`

---

### 2K.33 — Implement tautology filter for generated tests

**File**: `crates/roko-gate/src/generated.rs`
**What**: Run generated tests against the pre-implementation codebase. Discard tests that already pass:

```rust
pub fn filter_tautologies(
    tests: &[GeneratedCheck],
    workspace: &Path,
) -> Vec<GeneratedCheck> {
    // 1. Write all test cases to a temporary file
    // 2. Run cargo test --test generated_tautology_check
    // 3. Collect which tests PASS (these are tautologies)
    // 4. Return only tests that FAIL (these are meaningful)
}
```

**Context**: A test like `assert!(true)` or `assert!(module_exists("foo"))` passes before any implementation. These tests provide zero signal and should be discarded. Only tests that FAIL pre-implementation and PASS post-implementation are valuable.

**Acceptance**: Tests that pass before implementation are filtered out.
**Verification**: `cargo test -p roko-gate -- tautology_filter`

---

## Summary

| Gap | Tasks | IDs | Priority |
|---|---|---|---|
| **A. Tool Loop Runner** | 10 | 2K.01–2K.10 | 🔴 Critical |
| **B. Cache Layer Alignment** | 5 | 2K.11–2K.15 | 🔴 High ROI |
| **C. Streaming Pipeline** | 4 | 2K.16–2K.19 | 🟡 Needed for long sessions |
| **D. Event Fabric** | 4 | 2K.20–2K.23 | 🟡 Architectural cleanliness |
| **E. Session Continuity** | 2 | 2K.24–2K.25 | 🟡 Cost optimization |
| **F. Learned Conductor** | 2 | 2K.26–2K.27 | 🟢 Replaces heuristics |
| **G. TaskRunner Pipeline** | 3 | 2K.28–2K.30 | 🟡 Composition point |
| **H. Generated Test Gates** | 3 | 2K.31–2K.33 | 🟢 GVU verification investment |
| **Total** | **33** | **2K.01–2K.33** | |

## Execution Order (within this doc)

```
Gap A (2K.01–2K.10)  ← FIRST: nothing else works without the tool loop
    │
    ├── Gap B (2K.11–2K.15)  ← cost optimization, can start when A is done
    │
    ├── Gap C (2K.16–2K.19)  ← streaming, extends the tool loop
    │
    └── Gap E (2K.24–2K.25)  ← sessions, extends the tool loop

Gap D (2K.20–2K.23)  ← independent, can start in parallel with A

Gap F (2K.26–2K.27)  ← after Gap D (uses EventBus)

Gap G (2K.28–2K.30)  ← after A + D + F (composes them)

Gap H (2K.31–2K.33)  ← independent, can start anytime
```
