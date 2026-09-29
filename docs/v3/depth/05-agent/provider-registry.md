# 05-agent/provider-registry -- Provider Registry

> Config-driven provider binding: the TOML schema, ProviderConfig/ModelProfile
> structs, model resolution, effective-config merge, health tracking, and
> provider discovery.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** `crates/roko-core/src/config/schema.rs` (ProviderConfig, ModelProfile),
`crates/roko-core/src/agent.rs` (ProviderKind, resolve_model)

---

## 1. Purpose

The provider registry is the config-driven layer that binds model names to
concrete API endpoints. Before this layer, agent backends were inferred from
model slug heuristics (if the slug starts with `claude-`, spawn the Claude CLI).
The heuristic approach cannot handle third-party providers like ZhipuAI, Moonshot,
Perplexity, or Cerebras, because their slugs follow no built-in convention.

Two TOML tables solve this:

- **`[providers.*]`** -- defines *where* to send requests (protocol, URL, auth)
- **`[models.*]`** -- defines *what* to send (model slug, capabilities, cost)

A model entry points at a provider via the `provider` field. At resolve time,
Roko looks up the model, finds the provider, determines the `ProviderKind`, and
uses the appropriate adapter to construct a configured `Box<dyn Agent>`.

---

## 2. ProviderConfig

Defined in `crates/roko-core/src/config/schema.rs`:

```rust
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub timeout_ms: Option<u64>,
    pub extra_headers: Option<HashMap<String, String>>,
    pub max_concurrent: Option<u32>,
}
```

### Field semantics

| Field | Required for | Purpose |
|-------|-------------|---------|
| `kind` | All providers | Selects the `ProviderAdapter` via `adapter_for_kind()` |
| `base_url` | HTTP providers | API endpoint root; the adapter appends the path |
| `api_key_env` | HTTP providers | Env var name; resolved at runtime |
| `command` | CLI providers | Binary name (e.g., `"claude"`, `"gemini"`) |
| `args` | CLI providers | Default arguments appended to every invocation |
| `timeout_ms` | All providers | Per-request timeout; overridable per-agent |
| `extra_headers` | HTTP providers | Injected into every outbound request |
| `max_concurrent` | All providers | Concurrency limiter for the provider semaphore |

### API key resolution

API keys never appear in TOML. The `resolve_api_key()` method reads the named
environment variable at runtime:

```rust
impl ProviderConfig {
    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key_env
            .as_ref()
            .and_then(|env_name| std::env::var(env_name).ok())
    }
}
```

---

## 3. ProviderKind Enum

Defined at `crates/roko-core/src/agent.rs`. All 12 variants:

```rust
pub enum ProviderKind {
    AnthropicApi,   // Anthropic Messages API over HTTP
    ClaudeCli,      // `claude` CLI subprocess
    CodexCli,       // `codex exec --json` subprocess
    OpenAiCompat,   // OpenAI chat completions-compatible HTTP
    CursorAcp,      // Cursor Agent Client Protocol (JSON-RPC)
    CursorCli,      // Cursor ACP over stdio subprocess
    PerplexityApi,  // Perplexity Sonar HTTP API
    GeminiApi,      // Google Gemini native REST API
    GeminiCli,      // `gemini` CLI subprocess
    CerebrasApi,    // Cerebras Inference (OpenAI-compat, ultra-fast)
    Hermes,         // Hermes gateway (HTTP/CLI/ACP)
    OpenClaw,       // OpenClaw inference runtime (CLI/ACP)
}
```

This enum is the **primary dispatch key** for the provider system. When the
factory function needs to construct an agent, it passes the `ProviderKind` to
`adapter_for_kind()`, which returns the static adapter for that protocol family.

Each variant carries serde aliases for TOML flexibility (e.g., `CodexCli` also
accepts `"codex"`, `OpenClaw` also accepts `"openclaw"` and `"open_claw"`).

---

## 4. ModelProfile

Defined in `crates/roko-core/src/config/schema.rs`:

```rust
pub struct ModelProfile {
    pub provider: String,              // Key into [providers.*]
    pub slug: String,                  // Model ID sent to the API
    pub context_window: u64,           // Token window (default: 128_000)
    pub max_output: Option<u64>,       // Output-token cap
    pub supports_tools: bool,          // Tool calling
    pub supports_thinking: bool,       // Reasoning/thinking output
    pub supports_vision: bool,         // Image inputs
    pub supports_web_search: bool,     // Built-in web search
    pub supports_search: bool,         // Grounded search (Perplexity)
    pub supports_citations: bool,      // Response citations
    pub supports_async: bool,          // Async job API (deep research)
    pub tool_format: String,           // Wire format ("openai_json", "anthropic_blocks")
    pub cost_input_per_m: Option<f64>, // $/M input tokens
    pub cost_output_per_m: Option<f64>,// $/M output tokens
    pub cost_cache_read_per_m: Option<f64>,
    pub cost_cache_write_per_m: Option<f64>,
    pub cost_per_request: Option<f64>, // Per-request fee (Perplexity)
    pub search_context_size: Option<String>,
    pub max_tools: Option<u32>,        // Degradation threshold
    pub tokenizer_ratio: Option<f64>,  // vs o200k_base
    pub is_embedding_model: bool,
    pub provider_routing: Option<ProviderRouting>,
}
```

### Capability flags

The `supports_*` flags drive adapter behavior:

- **`supports_tools`** -- Whether the adapter includes a `tools` array. If false,
  tools are omitted entirely (embedding models, degraded tool support).
- **`supports_thinking`** -- Whether to parse `reasoning_content` or `thinking`
  blocks from the response.
- **`supports_vision`** -- Whether the transport supports inline image blocks.
  Only `AnthropicApi`, `OpenAiCompat`, and `GeminiApi` have native image paths.
- **`tool_format`** -- Selects the `Translator` implementation: `"openai_json"`,
  `"anthropic_blocks"`.
- **`max_tools`** -- When set, the adapter truncates the tool array. Some models
  degrade above a threshold.

### Cost fields

Cost metadata feeds into three systems:

1. **`Usage` computation** -- token counts multiplied by cost rates produce `cost_usd`
2. **Budget enforcement** -- per-role `TurnBudget` checks accumulated cost
3. **Model routing** -- CascadeRouter uses cost as one Pareto frontier dimension

---

## 5. Model Resolution

The `resolve_model` function bridges the config-driven and heuristic worlds:

```
resolve_model(config, model_key)
    |
    +-- Step 1: Try config.models.get(model_key)
    |      Found? -> Use profile + provider_config
    |
    +-- Step 2: Fall back to slug heuristic
           AgentBackend::from_model(model_key)
           -> Infer ProviderKind from slug pattern
```

The returned `ResolvedModel` carries:

| Field | Purpose |
|-------|---------|
| `model_key` | Original lookup key |
| `slug` | API-wire model ID |
| `provider_kind` | Which adapter to use |
| `provider_config` | Full provider config (if found) |
| `profile` | Full model profile (if found) |
| `backend` | Legacy backend inference (compatibility) |

This two-phase resolution means bare model slugs (e.g., `"claude-opus-4-6"`)
continue to work via heuristic, while `[providers.*]` and `[models.*]` entries
give full control.

---

## 6. Effective Config Merge

The `RokoConfig` struct provides `effective_providers()` and `effective_models()`
methods that merge built-in defaults with user-provided config.

Merge priority (highest to lowest):

1. User-specified `[providers.*]` / `[models.*]`
2. Built-in model profiles from `profile_for_model()`
3. Slug-heuristic fallback

This means Roko ships with known providers and models that work out of the box,
while users can override any field. The `config doctor` command reports merge
health.

---

## 7. Provider Health Tracking

Provider health is tracked through `roko-learn`'s `ModelCallFeedbackRecorder`.
Each completed agent run records a feedback signal combining:

- Gate pass/fail result
- Token efficiency (output tokens per successful unit of work)
- Latency (wall-clock time)
- Cost (USD)

Unhealthy providers (repeated failures, latency spikes) are filtered during
learned routing by the CascadeRouter's Pareto frontier computation and LinUCB
bandit selection. The health registry persists across sessions.

---

## 8. Provider Discovery

The `config providers discover` command probes the environment for available
providers:

1. Check environment variables for known API keys (`ANTHROPIC_API_KEY`,
   `OPENAI_API_KEY`, `GEMINI_API_KEY`, `PERPLEXITY_API_KEY`, etc.)
2. Check PATH for known CLI binaries (`claude`, `codex`, `gemini`)
3. Report available providers with suggested TOML config

The `config providers health` command queries each configured provider's
endpoint to verify connectivity and authentication.

---

## 9. ProviderRouting (OpenRouter)

The `ProviderRouting` struct enables OpenRouter-specific request shaping:

```rust
pub struct ProviderRouting {
    pub sort: Option<String>,           // "price", "throughput", "latency"
    pub order: Option<Vec<String>>,     // Explicit provider preference
    pub allow_fallbacks: Option<bool>,  // Auto-failover
    pub max_price: Option<f64>,         // Cost ceiling per token
    pub require_parameters: Option<Vec<String>>,
}
```

When set, the `OpenAiCompatAdapter` injects these as OpenRouter-specific headers
or body extensions. The `model_used` field in `ResponseMetadata` captures which
actual model served the request when OpenRouter routes to a different provider.

---

## 10. TOML Configuration Examples

```toml
[providers.anthropic]
kind = "anthropic_api"
base_url = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"
timeout_ms = 120000
max_concurrent = 5

[providers.perplexity]
kind = "perplexity_api"
base_url = "https://api.perplexity.ai"
api_key_env = "PERPLEXITY_API_KEY"

[providers.local-claude]
kind = "claude_cli"
command = "claude"
timeout_ms = 300000

[models.claude-opus]
provider = "anthropic"
slug = "claude-opus-4-6"
context_window = 200000
max_output = 32768
supports_tools = true
supports_thinking = true
supports_vision = true
tool_format = "anthropic_blocks"
cost_input_per_m = 15.00
cost_output_per_m = 75.00

[models.sonar-pro]
provider = "perplexity"
slug = "sonar-pro"
context_window = 200000
supports_tools = true
supports_search = true
supports_citations = true
tool_format = "openai_json"
cost_input_per_m = 3.00
cost_output_per_m = 15.00
cost_per_request = 0.005
search_context_size = "high"
```

---

## 11. Citations

1. `crates/roko-core/src/config/schema.rs` -- ProviderConfig, ModelProfile,
   ProviderRouting source.
2. `crates/roko-core/src/agent.rs` -- ProviderKind enum, resolve_model function.
3. `crates/roko-agent/src/provider/mod.rs` -- create_agent_for_model factory,
   adapter_for_kind dispatch.
4. `crates/roko-learn/src/` -- ModelCallFeedbackRecorder, CascadeRouter health
   filtering.
