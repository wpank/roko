# 05-agent/provider-adapters-12 -- All 12 Provider Adapters

> Complete reference for all 12 ProviderKind variants: config keys, capabilities,
> protocol details, construction flow, and provider-specific quirks.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** `crates/roko-agent/src/provider/mod.rs` (factory + dispatch),
per-adapter modules in `crates/roko-agent/src/provider/`

---

## 1. The ProviderAdapter Trait

Each adapter implements the `ProviderAdapter` trait:

```rust
pub trait ProviderAdapter: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError>;
    fn classify_error(&self, status: u16, body: &Value) -> ProviderError;
}
```

All adapters are unit structs instantiated as static constants -- no per-request
state, no allocations on the hot path. The `adapter_for_kind()` function maps
each `ProviderKind` variant to its static adapter instance via an exhaustive match.

---

## 2. AgentOptions

Runtime parameters not in the config registry:

```rust
pub struct AgentOptions {
    pub timeout_ms: Option<u64>,
    pub system_prompt: Option<String>,
    pub tools: Option<String>,
    pub mcp_config: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub extra_args: Vec<String>,
    pub effort: Option<String>,
    pub bare_mode: bool,
    pub dangerously_skip_permissions: bool,
    pub name: String,
}
```

---

## 3. Provider Catalog

### 3.1 AnthropicApi

| Field | Value |
|-------|-------|
| **Config key** | `kind = "anthropic_api"` |
| **Protocol** | Anthropic Messages API over HTTP |
| **Capabilities** | Tool calling, extended thinking (budget_tokens 1K--128K), prompt caching (cache_read/cache_write tokens), vision (image content blocks), streaming (SSE) |
| **Models** | claude-opus-4-6, claude-sonnet-4-6, claude-haiku-4-5, etc. |
| **Tool format** | `anthropic_blocks` -- content blocks with `tool_use`/`tool_result` types |
| **Config** | `base_url` (default: `https://api.anthropic.com`), `api_key_env` (ANTHROPIC_API_KEY), `timeout_ms`, `max_concurrent` |
| **Quirks** | Temperature fixed at 1 when thinking enabled. Tool use with thinking only supports `tool_choice: auto` or `none`. Token-efficient tools via beta header (up to 70% savings). Interleaved thinking beta allows reasoning between tool calls. Cache reads no longer count against ITPM limit. TTL: 5min (Sonnet), 1hr (Haiku) |
| **Tool loop** | Roko's ToolLoop drives multi-turn |

### 3.2 ClaudeCli

| Field | Value |
|-------|-------|
| **Config key** | `kind = "claude_cli"` |
| **Protocol** | Stream-JSON subprocess (`claude` binary) |
| **Capabilities** | Tool calling (internal loop), MCP passthrough (`--mcp-config`), `--system-prompt`/`--append-system-prompt`, `--allowedTools`, `--model`, `--permission-prompt-tool`, `--effort` |
| **Models** | All Anthropic models via `--model` flag |
| **Config** | `command` (default: `"claude"`), `args`, `timeout_ms` |
| **Quirks** | Drives its own internal tool loop -- Roko's ToolDispatcher/SafetyLayer are bypassed (Claude CLI has its own safety). Cost reported natively. `bare_mode = true` replaces built-in prompt via `--system-prompt`; full mode uses `--append-system-prompt` |
| **Tool loop** | Internal (Claude CLI) |

### 3.3 CodexCli

| Field | Value |
|-------|-------|
| **Config key** | `kind = "codex_cli"` (aliases: `CodexCli`, `codex`) |
| **Protocol** | Subprocess (`codex exec --json`) |
| **Capabilities** | Tool calling via Codex internal loop, reasoning tokens (subset of output_tokens) |
| **Models** | OpenAI o-series, codex-mini, etc. |
| **Config** | `command` (default: `"codex"`), `args`, `timeout_ms` |
| **Quirks** | Drives own tool loop like ClaudeCli; Roko's dispatcher bypassed |
| **Tool loop** | Internal (Codex) |

### 3.4 OpenAiCompat

| Field | Value |
|-------|-------|
| **Config key** | `kind = "openai_compat"` (alias: `open_ai_compat`) |
| **Protocol** | OpenAI Chat Completions API (`/v1/chat/completions`) |
| **Capabilities** | Tool calling (function calling), streaming (SSE), reasoning_content parsing, image inputs (when `supports_vision = true`), provider routing (OpenRouter) |
| **Models** | Any model via OpenAI-compatible endpoint: GPT-5, DeepSeek, ZhipuAI GLM, Moonshot Kimi, OpenRouter, local models |
| **Tool format** | `openai_json` |
| **Config** | `base_url`, `api_key_env`, `extra_headers`, `timeout_ms`, `max_concurrent` |
| **Quirks** | The de facto universal LLM wire protocol. Provider-specific extensions (OpenRouter `provider_routing`, ZhipuAI `sensitive` finish reason) captured in `ModelProfile` flags and `ResponseMetadata.extra`. Handles `reasoning_content` field for o-series/DeepSeek models |
| **Tool loop** | Roko's ToolLoop |

### 3.5 CursorAcp

| Field | Value |
|-------|-------|
| **Config key** | `kind = "cursor_acp"` |
| **Protocol** | Agent Client Protocol (JSON-RPC) |
| **Capabilities** | Mutation consent, experiments, Anthropic MCP parity, truthful capabilities, USD budgets, health-aware selection, sandboxing, worktree inspection. 180 ACP tests pass |
| **Models** | Any model available through Cursor's agent runtime |
| **Config** | `command` (e.g., `"cursor-agent"`), `timeout_ms` |
| **Quirks** | Definition-only MCP advertisements fail closed. ACP/serve experiment injection still needs canonical-section/receipt parity with runner |
| **Tool loop** | Internal (Cursor) |

### 3.6 CursorCli

| Field | Value |
|-------|-------|
| **Config key** | `kind = "cursor_cli"` |
| **Protocol** | ACP JSON-RPC over stdio subprocess |
| **Capabilities** | Same as CursorAcp, via subprocess transport |
| **Config** | `command`, `args`, `timeout_ms` |
| **Quirks** | Subprocess variant of CursorAcp; shares its limitations |
| **Tool loop** | Internal (Cursor) |

### 3.7 PerplexityApi

| Field | Value |
|-------|-------|
| **Config key** | `kind = "perplexity_api"` |
| **Protocol** | Perplexity Sonar HTTP API (OpenAI-compatible base, Sonar extensions) |
| **Capabilities** | Grounded web search, citations, `search_context_size` control, async deep research (`supports_async = true`) |
| **Models** | sonar, sonar-pro, sonar-deep-research, sonar-reasoning, sonar-reasoning-pro |
| **Tool format** | `openai_json` |
| **Config** | `base_url` (default: `https://api.perplexity.ai`), `api_key_env` (PERPLEXITY_API_KEY), `timeout_ms` |
| **Quirks** | Per-request fee (`cost_per_request`) in addition to token-based pricing. `search_context_size`: `"low"`, `"medium"`, `"high"`. Citations and search_results returned in response body, captured in `ResponseMetadata::web_search`. Ideal backend for `Researcher` role |
| **Tool loop** | Roko's ToolLoop |

### 3.8 GeminiApi

| Field | Value |
|-------|-------|
| **Config key** | `kind = "gemini_api"` |
| **Protocol** | Google Gemini API (native REST) |
| **Capabilities** | Tool calling, vision, grounding (Google Search), thinking (`thinkingConfig`), safety settings (forwarded from `config.gemini.safety_settings`) |
| **Models** | gemini-2.5-pro, gemini-2.5-flash, gemini-2.0-flash, etc. |
| **Config** | `base_url`, `api_key_env` (GEMINI_API_KEY or GOOGLE_API_KEY), `timeout_ms` |
| **Quirks** | Uses native Gemini format, not OpenAI compat. 1M token context window. Free tier: 15 RPM, 1M TPM, 1500 RPD. Thinking via `thinkingConfig` with `includeThoughts` and `thinkingBudget` (0--32K). Safety settings are per-request configurable. Also available via OpenAI-compat endpoint at `/v1beta/openai/` |
| **Tool loop** | Roko's ToolLoop |

### 3.9 GeminiCli

| Field | Value |
|-------|-------|
| **Config key** | `kind = "gemini_cli"` |
| **Protocol** | `gemini` CLI subprocess with MCP |
| **Capabilities** | Native authenticated Gemini CLI MCP integration |
| **Config** | `command` (default: `"gemini"`), `args`, `timeout_ms` |
| **Quirks** | Subprocess variant; requires local gemini CLI installation |
| **Tool loop** | Internal (Gemini CLI) |

### 3.10 CerebrasApi

| Field | Value |
|-------|-------|
| **Config key** | `kind = "cerebras_api"` |
| **Protocol** | OpenAI-compatible HTTP (ultra-fast inference) |
| **Capabilities** | Tool calling, extremely low latency inference |
| **Models** | llama-3.3-70b, etc. via Cerebras Inference |
| **Config** | `base_url` (default: `https://api.cerebras.ai`), `api_key_env` (CEREBRAS_API_KEY), `timeout_ms` |
| **Quirks** | Specialized for Cerebras-hosted models. Latency advantage is the primary selection criterion |
| **Tool loop** | Roko's ToolLoop |

### 3.11 Hermes

| Field | Value |
|-------|-------|
| **Config key** | `kind = "hermes"` (alias: `Hermes`) |
| **Protocol** | HTTP, CLI one-shot, or ACP |
| **Capabilities** | Gateway-style dispatch to Hermes-managed model fleet |
| **Config** | `base_url` or `command`, `api_key_env`, `timeout_ms` |
| **Quirks** | Requires a running Hermes gateway instance. Multi-transport |
| **Tool loop** | Varies by transport |

### 3.12 OpenClaw

| Field | Value |
|-------|-------|
| **Config key** | `kind = "open_claw"` (aliases: `OpenClaw`, `openclaw`) |
| **Protocol** | CLI one-shot or ACP |
| **Capabilities** | OpenClaw inference runtime integration |
| **Config** | `command`, `args`, `timeout_ms` |
| **Quirks** | WIT/Component-model hostcalls and legacy adapter parity remain separate roadmap work |
| **Tool loop** | Internal |

---

## 4. Error Classification

Each adapter implements `classify_error()` to map provider-specific errors to
canonical `ProviderError` variants:

```rust
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
```

The retry policy is deterministic and provider-agnostic:

| Error | Action |
|-------|--------|
| `RateLimit` | Wait delay (or 5s default), retry same provider |
| `AuthFailure` | Skip (terminal) |
| `Timeout` / `ServerError` | Try fallback provider |
| `ContentPolicy` | Skip (terminal) |
| `ContextOverflow` | Prune context, retry |
| `ModelNotFound` / `Other` | Try fallback |

---

## 5. Capability Matrix

| Capability | Anthropic | Claude CLI | Codex CLI | OpenAI Compat | Cursor ACP | Perplexity | Gemini | Cerebras |
|------------|-----------|------------|-----------|---------------|------------|------------|--------|----------|
| Streaming | SSE | Stream-JSON | No | SSE | JSON-RPC | SSE | SSE | SSE |
| Tool calling | Blocks | Internal | Internal | Functions | Internal | Functions | Native | Functions |
| Thinking | Budget | --effort | Reasoning | reasoning_content | N/A | N/A | thinkingConfig | N/A |
| Vision | Blocks | N/A | N/A | image_url | N/A | N/A | Parts | N/A |
| MCP | Native | --mcp-config | N/A | N/A | N/A | N/A | CLI MCP | N/A |
| Caching | Server 90% | Built-in | N/A | Auto 50% | N/A | N/A | Context API | N/A |
| Web search | Via tools | Via tools | N/A | Native (some) | N/A | Native | Grounding | N/A |
| Max context | 200K | 200K | Varies | 1M (GPT-4.1) | Varies | 200K | 1M | Varies |

---

## 6. The Factory: create_agent_for_model

The unified entry point at `crates/roko-agent/src/provider/mod.rs`:

```
create_agent_for_model(config, model_key, options)
    |
    +-- resolve_model(config, model_key) -> ResolvedModel
    +-- profile.or_else(config.effective_models())
    +-- provider_config.or_else(config.effective_providers())
    +-- adapter_for_kind(resolved.provider_kind) -> &'static dyn ProviderAdapter
    +-- adapter.create_agent(&provider_config, &profile, &options)
    +-- -> Result<Box<dyn Agent>, AgentCreationError>
```

### AgentCreationError

```rust
pub enum AgentCreationError {
    MissingApiKey(String),
    MissingConfig(String),
    InvalidKind(ProviderKind),
}
```

These are construction-time errors, not runtime errors. They indicate incomplete
or invalid configuration.

---

## 7. Inline Image Support

The `ProviderKind::supports_inline_images()` method returns `true` for
`AnthropicApi`, `OpenAiCompat`, and `GeminiApi` only. CLI/ACP subprocess and
harness protocols are excluded: a model's abstract vision capability is not
enough if its configured transport cannot preserve the image bytes to the
provider boundary.

---

## 8. Citations

1. `crates/roko-agent/src/provider/mod.rs` -- Factory, adapter dispatch, static
   adapters.
2. `crates/roko-core/src/agent.rs` -- ProviderKind enum, 12 variants.
3. `crates/roko-agent/src/provider/anthropic_api.rs` -- AnthropicApiAdapter.
4. `crates/roko-agent/src/provider/claude_cli.rs` -- ClaudeCliAdapter.
5. `crates/roko-agent/src/provider/openai_compat.rs` -- OpenAiCompatAdapter.
6. `crates/roko-agent/src/provider/cursor_acp.rs` -- CursorAcpAdapter.
7. `crates/roko-agent/src/provider/perplexity.rs` -- PerplexityApiAdapter.
8. `crates/roko-agent/src/provider/gemini_api.rs` -- GeminiApiAdapter.
9. `crates/roko-agent/src/provider/cerebras.rs` -- CerebrasApiAdapter.
