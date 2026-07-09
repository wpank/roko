# 03 — Provider Adapter Trait & Implementations

> **Priority**: 🔴 P0 — Core dispatch refactor
> **Status**: Not started
> **Depends on**: 02 (provider registry)
> **Blocks**: 05 (GLM), 06 (Kimi), 07 (OpenRouter)

## Problem Statement

Each of the 5 current agent structs (`ClaudeCliAgent`, `OpenAiAgent`, `CodexAgent`, `OllamaAgent`, `CursorAgent`) implements the `Agent` trait independently with duplicated HTTP logic, error handling, and response parsing. Adding a new protocol requires a new agent struct + wiring in orchestrate.rs. The `Agent` trait (`async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult`) mixes protocol concerns with business logic.

> **Critical discovery (doc 14)**: A `ToolLoop` already exists at `crates/roko-agent/src/tool_loop/mod.rs`
> with a `LlmBackend` trait. The adapter's `create_agent()` should produce a `ToolLoopAgent` that
> wraps the existing `ToolLoop` + a new `LlmBackend` impl, NOT a raw `CodexAgent`. See doc 14
> tasks 2L.01–2L.04 for the wiring.

## What Exists

| Component | Path | Lines | Status |
|---|---|---|---|
| Agent trait | `crates/roko-agent/src/agent.rs` | 94–112 | 🔌 Core interface |
| OpenAiAgent | `crates/roko-agent/src/openai_agent.rs` | 578 lines | 🔌 No tool loop |
| CodexAgent | `crates/roko-agent/src/codex_agent.rs` | 696 lines | 🔌 Similar to OpenAI |
| OllamaAgent | `crates/roko-agent/src/ollama_agent.rs` | 495 lines | 🔌 OpenAI-compat wire |
| CursorAgent | `crates/roko-agent/src/cursor_agent.rs` | 750 lines | 🔌 ACP protocol |
| ClaudeCliAgent | `crates/roko-agent/src/claude_cli_agent.rs` | 702 lines | 🔌 CLI subprocess |
| HttpPoster trait | `crates/roko-agent/src/http.rs` | — | 🔌 Shared HTTP layer |
| Translator trait | `crates/roko-agent/src/translate/mod.rs` | 54–81 | 🔌 Tool format conversion |

## Design

The refactor introduces `ProviderAdapter` as a layer **between** config resolution and the `Agent` trait. Existing agent structs are preserved but their creation is mediated by the adapter layer.

```
RokoConfig → resolve_model() → ResolvedModel
    │
    ├─ ProviderAdapter::for_kind(resolved.provider_kind)
    │   └─ Returns one of 4 adapter impls
    │
    ├─ adapter.create_agent(resolved, provider_config)
    │   └─ Returns Box<dyn Agent>
    │
    └─ agent.run(input, ctx)  // existing Agent trait, unchanged
```

---

## Checklist

> **Error handling note (doc 19)**: The `Agent` trait returns `AgentResult` (not `Result<>`).
> Errors are wrapped as `AgentResult { success: false, output: Signal::text(&error) }`.
> They are NOT propagated via `?`. The proposed `AgentCreationError` only applies to
> `create_agent_for_model()` (BEFORE `agent.run()` is called). The proposed `ProviderError`
> should be new variants on the EXISTING `LlmError` enum (tool_loop/mod.rs L58) rather than
> a separate type. See doc 19 "Error Type Architecture" for the full chain.

### 2B.01 — Define ProviderAdapter trait

**File**: `crates/roko-agent/src/provider.rs` (new file)
**What**: Create the core `ProviderAdapter` trait:

```rust
use crate::{Agent, AgentResult};
use roko_core::config::schema::{ProviderConfig, ModelProfile};
use roko_core::agent::ProviderKind;

/// Adapter for a protocol family. Creates Agent instances configured for
/// a specific provider and model.
pub trait ProviderAdapter: Send + Sync {
    /// Which protocol family this adapter handles.
    fn kind(&self) -> ProviderKind;

    /// Create an Agent instance from provider config and model profile.
    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError>;

    /// Classify an error response into a canonical error type.
    /// Used by health tracking to decide retry vs cooldown vs skip.
    fn classify_error(&self, status: u16, body: &serde_json::Value) -> ProviderError;
}

pub struct AgentOptions {
    pub timeout_ms: Option<u64>,
    pub system_prompt: Option<String>,
    pub tools: Option<String>,           // CSV for CLI, ignored for HTTP
    pub mcp_config: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub extra_args: Vec<String>,
    pub effort: Option<String>,
    pub bare_mode: bool,
    pub dangerously_skip_permissions: bool,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum ProviderError {
    RateLimit { retry_after_ms: Option<u64> },
    AuthFailure,
    Timeout,
    ServerError(u16),
    ContentPolicy,
    ContextOverflow,
    ModelNotFound,
    Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AgentCreationError {
    #[error("Missing API key: env var {0} not set")]
    MissingApiKey(String),
    #[error("Missing required config field: {0}")]
    MissingConfig(String),
    #[error("Invalid provider kind: {0:?}")]
    InvalidKind(ProviderKind),
}
```

**Acceptance**: Trait compiles. `ProviderError` and `AgentCreationError` have Display/Debug.
**Verification**: `cargo check -p roko-agent`

---

### 2B.02 — Implement OpenAiCompatAdapter

**File**: `crates/roko-agent/src/provider/openai_compat.rs` (new file)
**What**: Adapter for all OpenAI-compatible providers (Z.AI, Moonshot, OpenRouter, Together, Fireworks, Ollama, self-hosted vLLM).

```rust
pub struct OpenAiCompatAdapter;

impl ProviderAdapter for OpenAiCompatAdapter {
    fn kind(&self) -> ProviderKind { ProviderKind::OpenAiCompat }

    fn create_agent(&self, provider: &ProviderConfig, model: &ModelProfile, options: &AgentOptions)
        -> Result<Box<dyn Agent>, AgentCreationError>
    {
        let api_key = provider.resolve_api_key()
            .or_else(|| if provider.base_url.as_deref() == Some("http://localhost:11434") {
                Some(String::new())  // Ollama needs no key
            } else {
                None
            })
            .ok_or_else(|| AgentCreationError::MissingApiKey(
                provider.api_key_env.clone().unwrap_or_default()
            ))?;

        let base_url = provider.base_url.clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let timeout = options.timeout_ms
            .or(provider.timeout_ms)
            .unwrap_or(120_000);

        // Use CodexAgent (it has max_tokens support) with the resolved config
        let mut agent = CodexAgent::new(api_key, model.slug.clone())
            .with_base_url(base_url)
            .with_timeout_ms(timeout)
            .with_name(options.name.clone());

        Ok(Box::new(agent))
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
            408 => ProviderError::Timeout,
            500..=599 => ProviderError::ServerError(status),
            _ => ProviderError::Other(format!("HTTP {}", status)),
        }
    }
}
```

**Context**: This single adapter covers Z.AI (`base_url = "https://api.z.ai/api/paas/v4"`), Moonshot (`base_url = "https://api.moonshot.ai/v1"`), OpenRouter (`base_url = "https://openrouter.ai/api/v1"`), Ollama (`base_url = "http://localhost:11434"`), and any future OpenAI-compatible endpoint.

**Acceptance**: `OpenAiCompatAdapter::create_agent()` produces a working agent for a Z.AI provider config.
**Verification**: `cargo test -p roko-agent -- openai_compat_adapter`

---

### 2B.03 — Implement ClaudeCliAdapter

**File**: `crates/roko-agent/src/provider/claude_cli.rs` (new file)
**What**: Adapter that wraps `ClaudeCliAgent` creation. Extracts config from `ProviderConfig.command`, `ProviderConfig.args`, and `AgentOptions`.

**Context**: The existing `ClaudeCliAgent` has 15+ builder methods. The adapter consolidates the creation path.

**Acceptance**: `ClaudeCliAdapter::create_agent()` produces a `ClaudeCliAgent` with all options applied.
**Verification**: `cargo test -p roko-agent -- claude_cli_adapter`

---

### 2B.04 — Implement AnthropicApiAdapter

**File**: `crates/roko-agent/src/provider/anthropic_api.rs` (new file)
**What**: Adapter for the Anthropic Messages API (not Claude CLI). Uses the existing `ClaudeAgent` (HTTP-based, in `claude_agent.rs`).

**Context**: This is distinct from `ClaudeCliAdapter` — it's the HTTP API path. Currently exists as `ClaudeAgent` but isn't used in production (the CLI path is preferred). This adapter makes it available as a first-class option.

**Acceptance**: `AnthropicApiAdapter::create_agent()` produces a working agent that calls Anthropic's `/v1/messages` endpoint.
**Verification**: `cargo test -p roko-agent -- anthropic_api_adapter`

---

### 2B.05 — Implement CursorAcpAdapter

**File**: `crates/roko-agent/src/provider/cursor_acp.rs` (new file)
**What**: Adapter wrapping `CursorAgent` creation.

**Acceptance**: `CursorAcpAdapter::create_agent()` produces a `CursorAgent`.
**Verification**: `cargo test -p roko-agent -- cursor_acp_adapter`

---

### 2B.06 — Create provider module with adapter registry

**File**: `crates/roko-agent/src/provider/mod.rs` (new file)
**What**: Module that exports all adapters and provides a factory function:

```rust
pub fn adapter_for_kind(kind: ProviderKind) -> &'static dyn ProviderAdapter {
    match kind {
        ProviderKind::OpenAiCompat => &OpenAiCompatAdapter,
        ProviderKind::ClaudeCli => &ClaudeCliAdapter,
        ProviderKind::AnthropicApi => &AnthropicApiAdapter,
        ProviderKind::CursorAcp => &CursorAcpAdapter,
    }
}
```

Static dispatch via match — no trait objects, no vtable overhead, deterministic at compile time.

**Acceptance**: `adapter_for_kind(ProviderKind::OpenAiCompat)` returns the OpenAI-compat adapter.
**Verification**: `cargo test -p roko-agent -- adapter_for_kind`

---

### 2B.07 — Create unified agent factory function

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: High-level function that takes `RokoConfig` + model key and produces a ready agent:

```rust
pub fn create_agent_for_model(
    config: &RokoConfig,
    model_key: &str,
    options: AgentOptions,
) -> Result<Box<dyn Agent>, AgentCreationError> {
    let resolved = resolve_model(config, model_key);
    let provider_config = resolved.provider_config
        .ok_or_else(|| AgentCreationError::MissingConfig("provider".into()))?;
    let adapter = adapter_for_kind(resolved.provider_kind);
    adapter.create_agent(&provider_config, &resolved.profile.unwrap_or_default(), &options)
}
```

**Context**: This replaces the ad-hoc agent creation scattered across `orchestrate.rs` and `dispatch.rs`.

**Acceptance**: `create_agent_for_model(config, "glm-5-1", opts)` returns a configured agent.
**Verification**: `cargo test -p roko-agent -- create_agent_for_model`

---

### 2B.08 — Wire create_agent_for_model into orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Replace the existing agent construction code (which directly builds `ClaudeCliAgent`) with a call to `create_agent_for_model()`.

**Context**: Currently, `orchestrate.rs` constructs `ClaudeCliAgent` directly with hardcoded builder calls. After this change, it delegates to the adapter layer, which handles provider resolution, API key lookup, and agent construction.

The existing code path must still work: when `[providers.*]` is absent, the backwards-compat logic synthesizes a `claude_cli` provider from `[agent]`.

**Acceptance**: `cargo run -p roko-cli -- run "hello"` still works with the existing `roko.toml`. With a new config containing `[providers.zai]`, it dispatches to the OpenAI-compat adapter.
**Verification**:
```bash
# Existing config still works
cargo run -p roko-cli -- run "echo test" 2>&1 | head -5

# With env override
ROKO_MODEL=glm-5-1 cargo run -p roko-cli -- run "echo test" 2>&1 | head -5
```

---

### 2B.09 — Wire create_agent_for_model into roko run (run.rs + agent_exec.rs)

**File**: `crates/roko-cli/src/run.rs` (lines 311, 333) AND `crates/roko-cli/src/agent_exec.rs` (line 39)
**What**: The `roko run` one-shot mode has TWO agent creation sites (Claude and ExecAgent branches) plus `agent_exec.rs` has a third. All must be replaced with `create_agent_for_model()`.

**Context**: Doc 03 task 2B.08 covers orchestrate.rs (plan execution). This task covers the separate `roko run` one-shot execution path. Without this, `roko run "hello"` still hardcodes Claude even when providers are configured.

The existing `if config.agent.command == "claude"` branch at run.rs:309 is the dispatch heuristic being replaced.

**Acceptance**: `roko run "hello"` with `default_model = "glm-5-1"` in config uses the GLM-5.1 provider.
**Verification**: `ROKO__AGENT__MODEL=glm-5-1 cargo run -p roko-cli -- run "echo test" 2>&1 | head -5`

---

### 2B.10 — Wire create_agent_for_model into roko-serve dispatch.rs

**File**: `crates/roko-serve/src/dispatch.rs`
**What**: Same change as 2B.08 but for the webhook-driven dispatch path in roko-serve.

**Acceptance**: Webhook-triggered agent dispatch uses the new provider adapter path.
**Verification**: `cargo check -p roko-serve`

---

### 2B.11 — Add extra_headers support to HTTP agents

**File**: `crates/roko-agent/src/openai_agent.rs` and `crates/roko-agent/src/codex_agent.rs`
**What**: Both agents currently only send `Authorization: Bearer {key}`. Add support for extra headers from `ProviderConfig.extra_headers`.

OpenRouter requires `HTTP-Referer: roko-agent` and `X-Title: roko`. This must be injectable from config.

**Acceptance**: Extra headers appear in the HTTP request when configured.
**Verification**: `cargo test -p roko-agent -- extra_headers`

---

### 2B.12 — Preserve AgentBackend for backwards compatibility

**File**: `crates/roko-core/src/agent.rs`
**What**: Keep `AgentBackend` enum and `from_model()` as-is. Add a conversion:

```rust
impl From<AgentBackend> for ProviderKind {
    fn from(backend: AgentBackend) -> Self {
        match backend {
            AgentBackend::Claude => ProviderKind::ClaudeCli,
            AgentBackend::Codex | AgentBackend::OpenAi => ProviderKind::OpenAiCompat,
            AgentBackend::Cursor => ProviderKind::CursorAcp,
            AgentBackend::Ollama => ProviderKind::OpenAiCompat,
        }
    }
}
```

This allows the fallback path (no `[providers.*]` config) to map old backends to new provider kinds.

**Acceptance**: `ProviderKind::from(AgentBackend::Ollama)` returns `ProviderKind::OpenAiCompat`.
**Verification**: `cargo test -p roko-core -- backend_to_provider_kind`

---

### 2B.13 — Add provider selection logging

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: Log which provider and adapter was selected for each agent creation:

```rust
tracing::info!(
    model_key = model_key,
    slug = %resolved.slug,
    provider = %resolved.provider_kind,
    base_url = ?provider_config.base_url,
    "creating agent via provider adapter"
);
```

**Acceptance**: Agent creation emits a tracing::info event with model, provider, and base_url.
**Verification**: `RUST_LOG=info cargo run -p roko-cli -- run "test" 2>&1 | grep "creating agent"`

---

### 2B.14 — Write integration test: GLM-5.1 via Z.AI direct

**File**: `crates/roko-agent/tests/provider_integration.rs` (new)
**What**: Integration test (behind `#[cfg(feature = "integration")]`) that creates an agent for GLM-5.1 via the Z.AI provider and sends a simple prompt.

Uses mock HTTP poster for unit test, real HTTP for integration test.

**Acceptance**: Unit test passes with mock. Integration test passes with real Z.AI key.
**Verification**: `cargo test -p roko-agent -- glm_zai_direct`

---

### 2B.15 — Write integration test: Kimi-K2.5 via Moonshot direct

**File**: `crates/roko-agent/tests/provider_integration.rs`
**What**: Same as 2B.13 but for Kimi-K2.5 via Moonshot.

**Acceptance**: Unit test passes with mock.
**Verification**: `cargo test -p roko-agent -- kimi_moonshot_direct`

---

### 2B.16 — Write integration test: GLM-5.1 via OpenRouter

**File**: `crates/roko-agent/tests/provider_integration.rs`
**What**: Same model (GLM-5.1) but routed through OpenRouter (`slug = "z-ai/glm-5.1"`).

**Acceptance**: Unit test passes with mock.
**Verification**: `cargo test -p roko-agent -- glm_openrouter`

---

### 2B.17 — Write integration test: model via Ollama local

**File**: `crates/roko-agent/tests/provider_integration.rs`
**What**: Test creating an agent for a local Ollama model (e.g., `llama3.1:8b`).

**Acceptance**: Unit test passes with mock (no real Ollama needed).
**Verification**: `cargo test -p roko-agent -- ollama_local`

---

### 2B.18 — Update roko-agent/src/lib.rs exports

**File**: `crates/roko-agent/src/lib.rs`
**What**: Add `pub mod provider;` and re-export `create_agent_for_model`, `ProviderAdapter`, `adapter_for_kind`.

**Acceptance**: `use roko_agent::provider::create_agent_for_model;` compiles from roko-cli.
**Verification**: `cargo check -p roko-cli`

---

### 2B.19 — Write migration guide comment

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: Add a doc comment at the module level explaining:
- How the new provider system relates to the old Agent trait
- When to use `create_agent_for_model` vs direct agent construction
- How to add a new provider (implement ProviderAdapter or add config entry)

**Acceptance**: `cargo doc -p roko-agent --no-deps` generates readable provider module docs.
**Verification**: `cargo doc -p roko-agent --no-deps 2>&1 | grep -c warning` should not increase.
