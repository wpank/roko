# 13: Config That Shouldn't Need to Exist

## Core Problem

Roko requires users to configure implementation details that should be invisible. A normal user who downloads roko and adds an API key should have EVERYTHING work. They should never see:

- `use_max_completion_tokens = true` (API parameter quirk — OpenAI changed field names in 2024)
- `context_window = 200000` (model metadata — deterministic from model slug)
- `max_output = 16384` (model metadata — deterministic from model slug)
- `supports_tools = true` (model capability — deterministic from model slug)
- `supports_thinking = false` (model capability — deterministic from model slug)
- `supports_vision = true` (model capability — deterministic from model slug)
- `kind = "openai_compat"` (provider transport — deterministic from provider name)
- `base_url = "https://api.openai.com/v1"` (well-known endpoint — deterministic from provider name)
- `api_key_env = "OPENAI_API_KEY"` (obvious from provider name — industry convention)
- `timeout_ms = 120000` (reasonable default — never needs user config)
- `ttft_timeout_ms = 15000` (reasonable default — never needs user config)
- `connect_timeout_ms = 5000` (reasonable default — never needs user config)
- `tool_format = "openai_json"` (provider transport detail — deterministic from provider kind)
- `is_embedding_model = false` (default false — should never be explicit unless true)
- `supports_search = false` (default false — noise)
- `supports_citations = false` (default false — noise)
- `supports_async = false` (default false — noise)

**None of this should be user-facing config.** It's all deterministic metadata about known models/providers.

## Exact Current State

From `roko.toml`, one typical model entry:

```toml
[models.gpt54]
provider = "openai"
slug = "gpt-5.4"
context_window = 128000       # ← deterministic from slug
max_output = 16384            # ← deterministic from slug
supports_tools = true         # ← deterministic from slug
supports_thinking = false     # ← deterministic from slug
supports_vision = true        # ← deterministic from slug
use_max_completion_tokens = true  # ← API quirk, should be auto-detected
supports_web_search = false   # ← default false, noise
supports_mcp_tools = false    # ← default false, noise
supports_partial = false      # ← default false, noise
supports_grounding = false    # ← default false, noise
supports_code_execution = false  # ← default false, noise
supports_caching = false      # ← default false, noise
tool_format = "openai_json"   # ← deterministic from provider kind
cost_input_per_m = 2.5        # ← useful, but optional
cost_output_per_m = 10.0      # ← useful, but optional
supports_search = false       # ← default false, noise
supports_citations = false    # ← default false, noise
supports_async = false        # ← default false, noise
is_embedding_model = false    # ← default false, noise
tier = "premium"              # ← useful, but could be in registry
```

Of 22 fields, **0 are required for a known model**. The entire entry is noise.

One provider entry:

```toml
[providers.openai]
kind = "openai_compat"           # ← deterministic from provider name
base_url = "https://api.openai.com/v1"  # ← well-known, deterministic
api_key_env = "OPENAI_API_KEY"   # ← industry convention, deterministic
timeout_ms = 120000              # ← reasonable default
ttft_timeout_ms = 15000          # ← reasonable default
connect_timeout_ms = 5000        # ← reasonable default
```

Of 6 fields, **0 are required** for a standard provider. The entire entry is noise.

## What Config SHOULD Look Like

### Minimal (90% of users)

```toml
[project]
name = "my-project"

[agent]
default_model = "sonnet"  # or "gpt-5.5" or "opus" — just works

# That's it. Keys come from environment (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc).
```

### With explicit keys (if not using env vars)

```toml
[secrets]
anthropic = "sk-ant-..."
openai = "sk-..."
```

### Power user (optional overrides)

```toml
[agent]
default_model = "sonnet"
timeout_seconds = 300  # override default for slow operations

[models.my-local-llama]  # ONLY needed for custom/local models
provider = "ollama"
slug = "llama-3.2:70b"
```

## What Should Be Automatic

### 1. Provider detection from API key env var

If `ANTHROPIC_API_KEY` is set → `anthropic` provider available (no `[providers.anthropic]` needed).
If `OPENAI_API_KEY` is set → `openai` provider available.
If `GEMINI_API_KEY` is set → `gemini` provider available.
If `PERPLEXITY_API_KEY` is set → `perplexity` provider available.

**Implementation**: Extend `is_provider_available` in `crates/roko-core/src/config/schema.rs:473` to also synthesize providers from well-known env vars when no explicit `[providers.*]` entry exists.

```rust
/// Auto-synthesize well-known providers from standard env vars.
/// Called when `config.providers` is empty or missing the requested provider.
fn synthesize_standard_providers() -> IndexMap<String, ProviderConfig> {
    let mut map = IndexMap::new();

    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        map.insert("anthropic".into(), ProviderConfig {
            kind: ProviderKind::AnthropicApi,
            base_url: Some("https://api.anthropic.com/v1".into()),
            api_key_env: Some("ANTHROPIC_API_KEY".into()),
            timeout_ms: default_provider_timeout_ms(),
            ..ProviderConfig::default()
        });
    }
    if std::env::var("OPENAI_API_KEY").is_ok() {
        map.insert("openai".into(), ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some("https://api.openai.com/v1".into()),
            api_key_env: Some("OPENAI_API_KEY".into()),
            timeout_ms: default_provider_timeout_ms(),
            ..ProviderConfig::default()
        });
    }
    if std::env::var("GEMINI_API_KEY").is_ok() {
        map.insert("gemini".into(), ProviderConfig {
            kind: ProviderKind::GeminiApi,
            base_url: Some("https://generativelanguage.googleapis.com/v1beta/openai".into()),
            api_key_env: Some("GEMINI_API_KEY".into()),
            timeout_ms: default_provider_timeout_ms(),
            ..ProviderConfig::default()
        });
    }
    // ... perplexity, cerebras, etc.
    map
}
```

### 2. Model metadata from a built-in registry

See `01-model-config-ux.md` for the full `BUILTIN_MODELS` static. Core design:

```rust
/// In crates/roko-core/src/config/model_registry.rs
pub struct BuiltinModel {
    pub aliases: &'static [&'static str],
    pub slug: &'static str,
    pub provider_label: &'static str,
    pub provider_kind: ProviderKind,
    pub base_url: Option<&'static str>,
    pub api_key_env: &'static str,
    pub context_window: u64,
    pub max_output: u64,
    pub supports_tools: bool,
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub tool_format: &'static str,
    pub use_max_completion_tokens: bool,  // auto-detected, not user-visible
    pub tier: ModelTier,
    pub cost_input_per_m: Option<f64>,
    pub cost_output_per_m: Option<f64>,
}
```

User's `roko.toml` overrides take precedence, but you only need entries for custom/local models.

### 3. API quirks from provider knowledge — NOT user config

The `use_max_completion_tokens` field is the worst offender. It's a provider wire-format detail that changed when OpenAI moved from the chat completions API to the responses API. Users must never configure this.

**Current flow** (`crates/roko-agent/src/provider/openai_compat.rs:416`):
```rust
// Reads from config — defaults false — breaks modern OpenAI models
.with_use_max_completion_tokens(model.use_max_completion_tokens)
```

**Proposed flow**:
```rust
// Auto-detect from slug pattern, config field becomes optional override only
.with_use_max_completion_tokens(
    model.use_max_completion_tokens
        || should_use_max_completion_tokens(&model.slug)
)

fn should_use_max_completion_tokens(slug: &str) -> bool {
    // All OpenAI models from gpt-4o onward require this parameter name
    slug.starts_with("gpt-4o")
        || slug.starts_with("gpt-5")
        || slug.starts_with("o1")
        || slug.starts_with("o3")
        || slug.starts_with("o4")
        || slug.starts_with("codex")
}
```

This makes the config field an escape hatch (force it off or on) rather than a required field.

The same pattern applies for other provider quirks:
- Gemini requires different tool format when using native API vs. OpenAI-compat endpoint
- Perplexity search API needs different streaming behavior
- Ollama needs different context truncation handling

All of these should be detected from `provider.kind` or `model.slug`, not from user config.

### 4. Error → auto-fix for known recoverable errors

When an API returns a recoverable parameter error, fix and retry rather than failing:

```rust
// In crates/roko-agent/src/openai_compat_backend.rs, around line 315
// The current code picks one key or the other. Add recovery:
match send_request(body).await {
    Err(ApiError::Http { status: 400, ref body, .. })
        if body.to_string().contains("max_tokens") =>
    {
        // OpenAI rejected max_tokens — switch to max_completion_tokens and retry.
        // (Happens when use_max_completion_tokens was false but model requires it)
        tracing::warn!("auto-fix: switching to max_completion_tokens and retrying");
        let mut retry_body = body.clone();
        if let Some(obj) = retry_body.as_object_mut() {
            if let Some(val) = obj.remove("max_tokens") {
                obj.insert("max_completion_tokens".into(), val);
            }
        }
        send_request(retry_body).await
    }
    other => other,
}
```

## Fields in `ModelProfile` That Should Be Removed or Made Optional

In `crates/roko-core/src/config/provider.rs` (`ModelProfile` struct, lines 362-473):

| Field | Current | Should be |
|-------|---------|-----------|
| `use_max_completion_tokens` | `bool` (default false, breaks OpenAI) | Computed from slug, removed from TOML |
| `tool_format` | `String` (required) | Computed from provider.kind |
| `supports_tools` | `bool` (default true) | In builtin registry |
| `supports_thinking` | `bool` (default false) | In builtin registry |
| `supports_vision` | `bool` (default false) | In builtin registry |
| `supports_web_search` | `bool` (default false) | In builtin registry |
| `supports_mcp_tools` | `bool` (default false) | In builtin registry |
| `supports_partial` | `bool` (default false) | Rarely needed, keep but skip serialization |
| `supports_grounding` | `bool` (default false) | Gemini-specific, in builtin registry |
| `supports_code_execution` | `bool` (default false) | In builtin registry |
| `supports_caching` | `bool` (default false) | In builtin registry |
| `is_embedding_model` | `bool` (default false) | Always omit when false |
| `supports_search` | `bool` (default false) | Perplexity-specific, in builtin registry |
| `supports_citations` | `bool` (default false) | Perplexity-specific, in builtin registry |
| `supports_async` | `bool` (default false) | Perplexity-specific, in builtin registry |
| `context_window` | `u64` (default 200k) | In builtin registry |
| `max_output` | `Option<u64>` | In builtin registry |

The fields that SHOULD remain user-configurable:
- `provider` — which `[providers.*]` entry to use (needed when you have multiple of the same kind)
- `slug` — the wire model ID (needed when using non-builtin models)
- `tier` — optional; can be inferred from builtin registry
- `cost_*` — optional; can be in builtin registry but user may want to override
- `max_tool_iterations` — per-model override, reasonable to configure
- `tokenizer_ratio` — advanced override, reasonable to configure
- `thinking_level` — advanced override, reasonable to configure
- `provider_routing` — OpenRouter-specific, advanced

## Migration Path

### Phase 1 (Immediate, ~1 day): Auto-detect `use_max_completion_tokens`

**Files**: `crates/roko-agent/src/provider/openai_compat.rs:416`, `crates/roko-agent/src/tool_loop/backends/mod.rs:62`

Add the `should_use_max_completion_tokens(slug)` fallback. No config changes needed.

### Phase 2 (Short-term, ~3 days): Built-in model registry

**New file**: `crates/roko-core/src/config/model_registry.rs`

Add `BUILTIN_MODELS` static. Modify `resolve_model` in `crates/roko-core/src/agent.rs:304` to consult the registry before returning `profile: None`. Eliminate `Error::UnknownModel` for all builtin models.

### Phase 3 (Medium-term, ~2 days): Standard provider auto-synthesis

**File**: `crates/roko-core/src/config/schema.rs`, `effective_providers()` at line 278.

When `self.providers` is empty (no `[providers.*]` at all), call `synthesize_standard_providers()` to build providers from env vars. Project config still takes precedence.

### Phase 4 (Long-term): `roko.toml` only for project-specific settings

After Phases 1-3, the typical `roko.toml` is:

```toml
[project]
name = "roko-project"
root = "."

[agent]
default_model = "sonnet"
```

Everything else is auto-inferred. Custom/local models still require explicit entries.

## The Golden Rule

> If the information can be determined without asking the user, don't ask the user.

Every config field should pass this test:
- **Is this value different per-user?** → Config (API keys, preferred model name)
- **Is this value deterministic from other known values?** → Auto-infer (model metadata from slug, API quirks from provider kind)
- **Is this a reasonable default that works for 90% of cases?** → Hardcode (timeouts, context windows for unknown models)
- **Is this purely noise (default value that is almost never changed)?** → Remove from serialization (`#[serde(skip_serializing_if = ...)]` already applied to many; also skip from deserialization defaults)

## Files to Change

| File | Change | Priority |
|------|--------|----------|
| `crates/roko-agent/src/provider/openai_compat.rs:416` | Add `should_use_max_completion_tokens` fallback | P0 |
| `crates/roko-agent/src/tool_loop/backends/mod.rs:62` | Same pattern | P0 |
| New: `crates/roko-core/src/config/model_registry.rs` | `BUILTIN_MODELS` registry | P1 |
| `crates/roko-core/src/agent.rs:304` | Consult builtin registry in `resolve_model` | P1 |
| `crates/roko-cli/src/model_selection.rs:362` | Use builtin registry in `select_provider` | P1 |
| `crates/roko-core/src/config/schema.rs:278` | `effective_providers` synthesizes standard providers | P2 |
| `crates/roko-core/src/config/provider.rs` | Remove `use_max_completion_tokens` from user-facing docs | P2 |
