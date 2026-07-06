# 19 — Implementation Guide: Exact Wiring for Phase 1

> **Type**: Reference guide (complements task docs — tells implementing agents WHERE to make changes)
> **Audience**: Any agent executing tasks from docs 02–18

## Purpose

This document provides the exact code locations, integration points, and wiring sequence that implementing agents need. The task docs (02–18) say WHAT to build; this doc says WHERE it connects to existing code.

---

## The Five Integration Points

Every task in the plan ultimately connects to one of these 5 places in the existing codebase:

### 1. Agent Creation (orchestrate.rs L420–457)

**Current code:**
```rust
let mut agent = ClaudeCliAgent::new(&cfg.command, &cfg.exec_dir, &cfg.model)
    .with_timeout_ms(cfg.timeout_ms)
    .with_bare_mode(cfg.bare_mode)
    // ... 10+ builder calls
```

**After refactor:**
```rust
let agent = create_agent_for_model(
    &config,           // RokoConfig with [providers.*] and [models.*]
    &model_key,        // "glm-5-1" or "claude-opus" — key into [models.*]
    AgentOptions {
        timeout_ms: Some(cfg.timeout_ms),
        system_prompt: Some(cfg.system_prompt),
        tools: Some(cfg.allowed_tools_csv),
        mcp_config: cfg.mcp_config.clone(),
        env: cfg.env_vars.clone(),
        effort: Some(cfg.effort.clone()),
        bare_mode: cfg.bare_mode,
        name: format!("{}-{}", role.label(), plan_id),
        // ...
    },
)?;
```

**Tasks that modify this point:** 2B.07, 2B.08, 2L.03, 2L.04, 2K.30

---

### 2. Model Selection (orchestrate.rs L3842–3848, L5282–5305)

**Current code:**
```rust
fn effective_model(&self) -> String {
    self.config.agent.model.clone().unwrap_or_else(|| "claude-sonnet-4-6".into())
}

fn next_tier_model_slug(&self, current: &str) -> String {
    if current.contains("haiku") {
        self.config.agent.tier_models.get("focused").cloned().unwrap_or(...)
    } else if current.contains("sonnet") {
        self.config.agent.tier_models.get("architectural").cloned().unwrap_or(...)
    }
    // ...
}
```

**After refactor:**
```rust
fn effective_model(&self) -> String {
    // Use CascadeRouter if enough observations, else config default
    let ctx = RoutingContext { role, task_category, complexity, ... };
    let selection = self.learning.cascade_router.route(&ctx);
    selection.primary.slug
}

fn next_tier_model_slug(&self, current: &str) -> String {
    // Look up model profile to get tier, escalate within config
    let current_profile = self.config.effective_models().get(current);
    let next_tier = escalate_tier(current_profile.tier);
    self.config.agent.tier_models.get(&next_tier.label()).cloned().unwrap_or(...)
}
```

**Tasks that modify this point:** 2A.05, 2G.04, 2O.01, 2O.07

---

### 3. Learning Feedback (runtime_feedback.rs L580–645)

**Current code:**
```rust
fn update_cascade_router(&self, episode: &Episode) -> bool {
    let role_str = extra_string(episode, "role");
    let model_slug = extra_string(episode, "model");
    // ... parse context ...
    let reward = if episode.success { 1.0 } else { 0.0 };
    self.cascade_router.record_observation(&ctx, &slug, reward, episode.success);
}
```

**After refactor:**
```rust
fn update_cascade_router(&self, episode: &Episode) -> bool {
    // ... existing role/model parsing ...

    // NEW: Record provider health
    if let Some(provider) = extra_string(episode, "provider") {
        if episode.success {
            self.provider_health.record_success(&provider);
        } else {
            self.provider_health.record_failure(&provider);
        }
    }

    // NEW: Compute richer reward with latency
    let wall_ms = episode.usage.wall_ms;
    let cost = episode.usage.cost_usd as f64;
    let reward = compute_routing_reward_v2(
        if episode.success { 1.0 } else { 0.0 },
        cost / 5.0,  // normalize vs $5 ceiling
        wall_ms as f64,
        120_000.0,   // 2-minute SLA
    );

    self.cascade_router.record_observation(&ctx, &slug, reward, episode.success);
}
```

**Tasks that modify this point:** 2O.01, 2O.02, 2O.04, 2O.05, 2O.06

---

### 4. LearningRuntime Constructor (runtime_feedback.rs L277–283)

**Current code:**
```rust
let cascade_router = CascadeRouter::load_or_new(
    &paths.cascade_router_json,
    vec![
        "claude-sonnet-4-20250514".into(),
        "claude-haiku-4-5-20251001".into(),
    ],
);
```

**After refactor:**
```rust
// Load model slugs from config instead of hardcoding
let model_slugs: Vec<String> = config.effective_models()
    .values()
    .map(|m| m.slug.clone())
    .collect();

let cascade_router = CascadeRouter::load_or_new(
    &paths.cascade_router_json,
    model_slugs,
);
```

**Tasks that modify this point:** 2A.08, 2D.11, 2E.13, 2L.14

---

### 5. ToolLoop LlmBackend (tool_loop/mod.rs L43–54)

**Current trait (no implementations exist):**
```rust
pub trait LlmBackend: Send + Sync {
    async fn send_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
    ) -> Result<BackendResponse, LlmError>;
}
```

**New implementation (from doc 14 task 2L.01):**
```rust
pub struct OpenAiCompatBackend {
    poster: Arc<dyn HttpPoster>,
    base_url: String,
    api_key: String,
    model_slug: String,
    extra_headers: Vec<(String, String)>,
    extra_body_params: serde_json::Map<String, Value>,
    timeout_ms: u64,
}

impl LlmBackend for OpenAiCompatBackend {
    async fn send_turn(&self, messages: &[Value], tools: &RenderedTools) -> Result<BackendResponse> {
        // POST to {base_url}/chat/completions with messages + tools
    }
}
```

**Tasks that modify this point:** 2L.01, 2L.02, 2L.03, 2N.04, 2N.06

---

## Phase 1 Execution Sequence (Exact Order)

```
Step 1: roko-core types (doc 02)
   ├── Add ProviderKind enum to agent.rs
   ├── Add ProviderConfig, ModelProfile to config/schema.rs
   ├── Add providers/models to RokoConfig (serde(default))
   ├── Add resolve_model() function
   ├── Add backwards-compat effective_providers()/effective_models()
   └── Tests: cargo test -p roko-core

Step 2: roko-cli config (doc 18 task 2P.03)
   ├── Add ProviderLayer, ModelProfileLayer to config.rs
   ├── Add to ConfigLayer merge logic
   └── Tests: cargo test -p roko-cli -- config

Step 3: roko-CORE chat types (doc 13 tasks 2K.01-04) — NOTE: roko-core, NOT roko-agent!
   ├── Add ChatMessage, ChatRequest, ChatResponse to roko-core/src/chat_types.rs
   │   (Must be in roko-core so roko-compose can use them for prompt assembly)
   ├── Add Signal ↔ ChatRequest conversion
   ├── Re-export from roko-core/src/lib.rs
   └── Tests: cargo test -p roko-core -- chat_types

Step 4: roko-agent LlmBackend impl (doc 14 tasks 2L.01-02)
   ├── Implement OpenAiCompatBackend
   ├── Create backend factory
   └── Tests: cargo test -p roko-agent -- openai_compat_backend

Step 5: roko-agent ToolLoopAgent wrapper (doc 14 tasks 2L.03-04)
   ├── Wire ToolLoop + OpenAiCompatBackend into ToolLoopAgent
   ├── Implement Agent trait for ToolLoopAgent
   └── Tests: cargo test -p roko-agent -- tool_loop_agent

Step 6: Provider adapter layer (doc 03 tasks 2B.01-06)
   ├── Define ProviderAdapter trait
   ├── Implement OpenAiCompatAdapter (creates ToolLoopAgent for tool-supporting models)
   ├── Implement ClaudeCliAdapter (wraps existing ClaudeCliAgent)
   ├── Create adapter_for_kind() factory
   ├── Create create_agent_for_model() top-level function
   └── Tests: cargo test -p roko-agent -- provider_adapter

Step 7: ToolDef extension (doc 18 tasks 2P.01-02)
   ├── Add ToolSource field to ToolDef
   ├── Update OpenAiTranslator to render based on source
   └── Tests: cargo test -p roko-core -- tool_source && cargo test -p roko-agent -- render_tools

Step 8: Wire into orchestrate.rs (doc 03 task 2B.08)
   ├── Replace ClaudeCliAgent::new() with create_agent_for_model()
   ├── Update effective_model() to use config models
   ├── Update LearningRuntime constructor to use config model slugs
   └── Tests: cargo run -p roko-cli -- run "echo test"

Step 9: Translator extensions (doc 04 tasks 2C.01-03, 2C.08-09)
   ├── Add reasoning extraction to BackendResponse
   ├── Add FinishReason normalization
   ├── Add cached_tokens extraction from both GLM and Kimi formats
   └── Tests: cargo test -p roko-agent -- translator

Step 10: Example configs (doc 02 task 2A.12, doc 05-07)
   ├── Create examples/roko-glm.toml
   ├── Create examples/roko-kimi.toml
   ├── Create examples/roko-openrouter.toml
   └── Tests: parse each config without errors
```

After Phase 1: GLM-5.1, Kimi-K2.5, and any OpenAI-compatible model can be configured via TOML and will run the full tool loop with tool dispatch, context pruning, and checkpointing.

---

## Config Fields That Must Exist in BOTH Schemas

| Field | roko-core RokoConfig | roko-cli Config (via ConfigLayer) |
|---|---|---|
| `providers` | `HashMap<String, ProviderConfig>` | `Option<HashMap<String, ProviderLayer>>` |
| `models` | `HashMap<String, ModelProfile>` | `Option<HashMap<String, ModelProfileLayer>>` |
| `config_version` | `u32` (default 1) | `Option<u32>` |

The core schema uses concrete types; the CLI schema uses `Option<T>` mirror types for layered merge. On load, CLI merges layers and resolves into the final config. Core reads from disk directly.

---

## Episode Metadata Fields for Provider Tracking

The `Episode.extra` HashMap needs these keys for the provider system:

| Key | Type | Set By | Used By |
|---|---|---|---|
| `"role"` | String | orchestrate.rs | runtime_feedback.rs L604 |
| `"model"` | String | orchestrate.rs | runtime_feedback.rs L605 |
| `"provider"` | String | **NEW** — set by create_agent_for_model | runtime_feedback.rs (2O.01) |
| `"model_key"` | String | **NEW** — the config key (e.g., "glm-5-1") | routing log |
| `"task_category"` | String | orchestrate.rs | runtime_feedback.rs L614 |
| `"complexity_band"` | String | orchestrate.rs | runtime_feedback.rs L616 |
| `"experiment_variant"` | String | orchestrate.rs | runtime_feedback.rs L591 |

---

## Dependency Addition Order

Add dependencies incrementally to avoid CI breakage:

```
Step 1 (Phase 1): No new deps — pure types
Step 2 (Phase 1): No new deps — config parsing
Step 3 (Phase 2): arc-swap (roko-serve only — hot reload)
Step 4 (Phase 3): governor (roko-agent — rate limiting)
Step 5 (Phase 3): tiktoken-rs (roko-compose — token counting)
Step 6 (Phase 3): tokenizers (roko-compose — optional, for GLM/Kimi tokenizers)
Step 7 (tests only): wiremock (roko-agent dev-dep — mock HTTP server)
```

---

## Crate Dependency Graph (Verified)

```
roko-core  (ZERO roko deps — the kernel)
    ↑
    ├── roko-std (core only)
    ├── roko-fs (core only)
    ├── roko-gate (core only)
    ├── roko-agent (core only)
    ├── roko-compose (core only — NOT agent!)
    ├── roko-learn (core + bardo-primitives)
    └── roko-orchestrator (core only)

roko-serve (depends on: core, agent, learn, neuro, gate, fs, compose, std, orchestrator, conductor, plugin, bardo-runtime)
roko-cli   (depends on: everything above + roko-serve + clap + ratatui)
```

### Type Placement Rules

| Type | Crate | Why |
|---|---|---|
| `ChatMessage`, `ChatRequest`, `ChatResponse` | **roko-core** | roko-compose needs them but doesn't depend on roko-agent |
| `ProviderKind`, `ProviderConfig`, `ModelProfile` | **roko-core** | Used by roko-learn (routing) and roko-agent (adapter creation) |
| `ProviderAdapter` trait | **roko-agent** | Creates agents — agent-specific concern |
| `LlmBackend` trait | **roko-agent** | Already here at tool_loop/mod.rs |
| `OpenAiCompatBackend` | **roko-agent** | Agent implementation |
| `ToolLoopAgent` wrapper | **roko-agent** | Agent implementation |
| `ToolSource` enum (on ToolDef) | **roko-core** | ToolDef is already in roko-core |
| `CostTable`, `LatencyRegistry` | **roko-learn** | Learning concern |
| `TokenCounter` | **roko-compose** | Prompt assembly concern |

### Critical Constraint

**roko-compose depends on roko-core but NOT roko-agent.** Any type that prompt assembly needs must be in roko-core. This is why ChatRequest lives in roko-core — the prompt builder constructs it.

### Existing Local ChatRequest/ChatResponse Types

`codex_agent.rs` (L69–91) and `ollama_agent.rs` both define local `ChatRequest`/`ChatResponse` structs. These should be replaced with imports from `roko-core::chat_types` once the canonical types exist.

## ALL Agent Creation Sites (Must All Be Refactored)

There are **7 places** in the codebase that create agents directly. ALL must be updated to use `create_agent_for_model()`:

| # | File | Line | Path | Current |
|---|---|---|---|---|
| 1 | `agent_exec.rs` | 39 | `roko run` agent-exec mode | `ClaudeCliAgent::new("claude", ...)` |
| 2 | `run.rs` | 311 | `roko run` one-shot (Claude) | `ClaudeCliAgent::new(&config.agent.command, ...)` |
| 3 | `run.rs` | 333 | `roko run` one-shot (ExecAgent) | `ExecAgent::new(&config.agent.command, ...)` |
| 4 | `orchestrate.rs` | 428 | Plan execution (Claude) | `ClaudeCliAgent::new(&cfg.command, ...)` |
| 5 | `orchestrate.rs` | 451 | Plan execution (ExecAgent) | `ExecAgent::new(&cfg.command, ...)` |
| 6 | `orchestrate.rs` | 6718 | 2nd plan exec (Claude) | `ClaudeCliAgent::new(...)` |
| 7 | `orchestrate.rs` | 6753 | 2nd plan exec (ExecAgent) | `ExecAgent::new(...)` |
| 8 | `dispatch.rs` | ~varies | roko-serve webhook dispatch | `ClaudeCliAgent::new(...)` |

> **Doc 03 tasks 2B.08–2B.09 only cover orchestrate.rs and dispatch.rs.** The `run.rs` and
> `agent_exec.rs` paths are missed. These need additional tasks or 2B.08 needs to be broadened
> to cover all 8 sites.

The `if cfg.command == "claude"` branch at orchestrate.rs L427 is the dispatch heuristic being replaced by `create_agent_for_model()`. After refactor, all 8 sites collapse to one call pattern.

## ExecAgent: NOT a ProviderKind

ExecAgent (`crates/roko-agent/src/exec.rs`) is a **stdin/stdout pipe** that wraps any CLI (`ollama run`, `mods`, `llm`, `cat`). It has:
- No tool support
- No protocol awareness
- No streaming
- No session management

It **cannot** be wrapped in `ToolLoop` because `ToolLoop` requires `LlmBackend::send_turn()` with tool specs. ExecAgent just pipes text to stdin and captures stdout.

**Correct role**: ExecAgent remains a legacy fallback for testing and non-agentic CLIs. It is NOT a 5th `ProviderKind`. It sits BELOW the provider routing layer — if no provider matches, ExecAgent is the last resort.

In the refactored `create_agent_for_model()`:
```rust
pub fn create_agent_for_model(config, model_key, options) -> Result<Box<dyn Agent>> {
    let resolved = resolve_model(config, model_key);
    if let Some(provider_config) = resolved.provider_config {
        // Route through ProviderAdapter → ToolLoopAgent or ClaudeCliAgent
        let adapter = adapter_for_kind(resolved.provider_kind);
        adapter.create_agent(&provider_config, &resolved.profile, &options)
    } else {
        // Legacy fallback: ExecAgent for unknown commands
        Ok(Box::new(ExecAgent::new(&config.agent.command, config.agent.args.clone())))
    }
}
```

## SystemPromptBuilder: Currently Model-Agnostic

`RoleSystemPromptSpec` and `SystemPromptBuilder` have **zero awareness** of which model/provider will receive the prompt. No `model` field, no conditional logic, no provider-specific formatting.

This means:
- Claude, GLM, and Kimi all receive identical prompts
- Model-specific optimizations (XML tags for Claude, different instruction phrasing for GLM) are not possible
- The `roko-compose` crate has NO dependency on `roko-agent`, so it cannot query provider capabilities

**For Phase 1**: This is acceptable. All target models (GLM, Kimi, Claude) handle the same prompt format adequately.

**For Phase 3+**: Add optional model-aware prompt adaptation — see doc 17 task 2O.12 (model-specific heuristic tagging) and future tasks for provider-specific prompt templates.

## `roko run` vs `roko plan run`: Different Integration Depth

| Aspect | `roko run` (one-shot) | `roko plan run` |
|---|---|---|
| CascadeRouter | NOT used | Used for per-task model selection |
| System prompt | Simple `build_system_prompt()` | Rich `RoleSystemPromptSpec` with domain context |
| Gate pipeline | Minimal (compile, test, clippy) | Full adaptive 6-rung with thresholds |
| Model escalation | Not supported | Tier escalation on failure |
| Learning feedback | Episodes + task metrics only | Full LearningRuntime with all 10+ subsystems |
| Provider routing | Static from config | Dynamic cascade router with bandits |

**For the refactor**: Both paths need `create_agent_for_model()` for agent creation. But `roko run` uses the default model from config directly, while `roko plan run` routes via CascadeRouter. The implementation guide's Step 8 covers orchestrate.rs; task 2B.09 covers run.rs.

## What NOT to Change

These components work correctly and should NOT be modified during the provider refactor:

| Component | Why Not |
|---|---|
| `ToolDispatcher` (dispatcher/mod.rs) | 7-step pipeline is correct. ToolLoop calls it. |
| `SafetyLayer` (safety/*.rs) | Works independently. Attach via ToolDispatcher constructor. |
| `HandlerResolver` / 16 builtin handlers | Static tool handlers. MCP adds dynamic ones via DynamicToolRegistry. |
| `SystemPromptBuilder` layers 1-6 | Cache marker system works. Only add cache layer SORTING (2K.12). |
| `EpisodeLogger` format | JSONL format is stable. Only add new `extra` keys, don't change schema. |
| `ProcessSupervisor` (bardo-runtime) | Tracks processes correctly. ToolLoopAgent is in-process, not subprocess. |
| `ExperimentStore` | UCB1 variant tracking works. Add model experiments alongside, don't modify. |

---

## Error Type Architecture

### Current Error Chain (no unified error type)

```
HttpPoster.post_json()  → Result<String, HttpPostError>
    ↓ (caught by agent impl)
LlmBackend.send_turn()  → Result<BackendResponse, LlmError>
    ↓ (caught by ToolLoop)
Translator.parse_calls() → Result<Vec<ToolCall>, TranslatorError>
    ↓ (caught by ToolLoop)
Agent.run()              → AgentResult { success: bool, output: Signal }
    ↓ (NO Result<> — errors become success=false)
orchestrate.rs           → checks agent_result.success
```

**Critical**: The `Agent` trait returns `AgentResult`, NOT `Result<_, Error>`. Errors are not propagated via `?`. They're wrapped: `AgentResult::fail(Signal::text(&error_message))`.

**Existing error types:**
- `LlmError` (tool_loop/mod.rs L58): `Backend(String)` | `Network(String)` — only 2 variants
- `HttpPostError` (http.rs L25): struct with `status: Option<u16>`, `message: String`
- `TranslatorError` (translate/mod.rs L164): `Malformed(String)` | `UnsupportedFormat(ToolFormat)`
- `McpError` (mcp/client.rs L106): `Json` | `Transport` | `Server`
- `ConfigError` (mcp/config.rs L83): `Io` | `Parse`

**Proposed new types (doc 03):**
- `AgentCreationError` — occurs BEFORE `Agent.run()`, in the factory. Uses `Result<Box<dyn Agent>, AgentCreationError>`.
- `ProviderError` — classifies HTTP errors for health tracking. Used internally, NOT returned to orchestrator.

### How they compose

```
create_agent_for_model() → Result<Box<dyn Agent>, AgentCreationError>
    │                          ↑ Propagated to orchestrator as hard failure
    │
    └── ProviderAdapter.create_agent()
         ├── Missing API key → AgentCreationError::MissingApiKey
         ├── Missing config  → AgentCreationError::MissingConfig
         └── OK → Box<dyn Agent>

agent.run() → AgentResult { success, output, usage }
    │              ↑ No Result<> — always returns
    │
    └── ToolLoopAgent.run()
         └── ToolLoop.run()
              ├── LlmBackend.send_turn() → LlmError
              │    └── classify_error() → ProviderError (for health tracking)
              │    └── retry_policy.should_retry() → retry or fallback
              ├── Translator.parse_calls() → TranslatorError
              └── ToolDispatcher.dispatch_batch() → Vec<ToolResult>

              On terminal error: return AgentResult::fail(Signal::text(&error))
              On max iterations: return AgentResult::fail(...)
```

`ProviderError` is used ONLY for health tracking (circuit breaker, retry decisions). It never leaves the agent layer. The orchestrator sees `AgentResult.success = false` and consults the conductor for intervention.

### Extension: ProviderError should extend LlmError

Rather than creating a separate `ProviderError`, extend `LlmError`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("backend error: {0}")]
    Backend(String),
    #[error("network error: {0}")]
    Network(String),
    // NEW variants for provider-specific errors:
    #[error("rate limited (retry after {retry_after_ms:?}ms)")]
    RateLimit { retry_after_ms: Option<u64> },
    #[error("auth failure")]
    AuthFailure,
    #[error("context overflow")]
    ContextOverflow,
    #[error("content policy violation")]
    ContentPolicy,
    #[error("model not found: {0}")]
    ModelNotFound(String),
}
```

This keeps the error hierarchy flat (one enum per layer boundary) and avoids the proliferation problem.

## roko-plugin Already Has Extension Traits

**Location**: `crates/roko-plugin/src/lib.rs`

Already defines:
- `EventSource` trait: `start(sender, cancel_token) -> Result<()>` — emits signals
- `FeedbackCollector` trait: `collect(since) -> Result<Vec<FeedbackSignal>>` — polls external services
- `PluginBuilder` + `PluginManifest`: registration mechanism

For provider extensions, consider adding a `ProviderPlugin` trait to roko-plugin:
```rust
pub trait ProviderPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn provider_kind(&self) -> ProviderKind;
    fn create_backend(&self, config: &Value) -> Result<Arc<dyn LlmBackend>>;
}
```

This would allow external crates to register providers without modifying roko-agent. However, this is Phase 5+ work — for now, the compile-time trait pattern (rig-rs style) is sufficient.

## Future Work (Phase 5+ — Not in Current Tasks)

These patterns from the research are important but too large for the current refactor:

| Pattern | Source | Why Not Now |
|---|---|---|
| Event-sourced state | OpenHands SDK | Requires migrating from snapshot to event log — major persistence change |
| MCTS solution search | Moatless Tools | Requires search tree infrastructure — separate effort |
| Self-play (SSR/SSP) | Meta, Alibaba | Requires training infrastructure — separate effort |
| GEPA prompt evolution | ICLR 2026 | Complex optimization loop — build on top of existing ExperimentStore later |
| BetterTogether (DSPy) | Stanford | Requires alternating prompt+weight optimization — heavy |
| SEC curriculum | Alibaba | Non-stationary MAB for task ordering — add after routing is stable |
| Optimas full LRF | Stanford | Pairwise ranking loss training — after enough episodes collected |
| WASM plugins | Extism | Heavyweight — MCP-as-plugin covers the immediate need |
| OpenTelemetry export | OTel GenAI SIG | Structured logging works for now; OTel adds external dep + collector |

These should be their own implementation plans after the provider refactor is stable and producing episode data.
