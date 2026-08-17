# 01: Model Configuration UX

## Problem

To use a model with roko, you currently need to:

1. Know the exact OpenAI/Anthropic model ID (slug)
2. Know which provider it routes through
3. Manually add a `[models.X]` entry to `roko.toml`
4. Know the context window and max output token limits
5. Know whether it supports tools
6. Know whether it uses chat completions vs responses API
7. Know whether `use_max_completion_tokens` applies (required for all modern OpenAI models)

**This is absurd.** No user should need to do any of this.

### Current roko.toml model entry: 20 fields typical

The actual `roko.toml` has 36 `[models.*]` sections, each with 20 fields:

```toml
[models.gpt54]
provider = "openai"
slug = "gpt-5.4"
context_window = 128000
max_output = 16384
supports_tools = true
supports_thinking = false
supports_vision = true
use_max_completion_tokens = true      # ← user must know this
supports_web_search = false
supports_mcp_tools = false
supports_partial = false
supports_grounding = false
supports_code_execution = false
supports_caching = false
tool_format = "openai_json"           # ← user must know this
cost_input_per_m = 2.5
cost_output_per_m = 10.0
supports_search = false
supports_citations = false
supports_async = false
is_embedding_model = false
tier = "premium"
```

That is 20-22 fields per model entry. Zero of them should be required for a known model.

### Failure Transcript

```
$ roko prd plan cursor-composer-backend --model codex-5.5
error: resolve model selection for prd plan: cli override selected unknown model 'codex-5.5'
       (inferred kind 'openai_compat'); add an explicit [models.*] entry for this model
```

The user typed a reasonable model name. Roko rejected it and told them to edit TOML.

## Root Cause Code Paths

### 1. `resolve_model` in `crates/roko-core/src/agent.rs:304`

```rust
pub fn resolve_model(config: &RokoConfig, model_key: &str) -> ResolvedModel {
    // 1. Direct lookup by config key
    if let Some(profile) = config.models.get(model_key) { ... }
    // 2. Fallback: search by slug
    for (key, profile) in &config.models { if profile.slug == model_key { ... } }
    // 3. Prefix match on slug
    ...
    // Unconfigured slug — heuristic fallback.
    // profile = None is set here, which causes UnknownModel error below
    ResolvedModel { ..., profile: None, ... }
}
```

**The problem**: `profile: None` causes `select_provider()` in `model_selection.rs:362` to always return `Error::UnknownModel` when the model isn't in `config.models`.

### 2. `select_provider` in `crates/roko-cli/src/model_selection.rs:335`

```rust
fn select_provider(source, model, resolved, providers) -> Result<...> {
    if let Some(profile) = resolved.profile.as_ref() {
        // ... look up provider from config
    }
    // Falls here if profile is None (unconfigured model)
    Err(Error::UnknownModel { ... })  // line 362
}
```

**The error message** at `model_selection.rs:122`:
```
"{selection_source} selected unknown model '{model}' (inferred kind '{provider_kind}');
 add an explicit [models.*] entry for this model"
```

### 3. `use_max_completion_tokens` propagation

The flag flows:
- `ModelProfile.use_max_completion_tokens` (provider.rs:472)
- Read by `openai_compat.rs:416`: `.with_use_max_completion_tokens(model.use_max_completion_tokens)`
- Used by `openai_compat_backend.rs:315`: selects between `"max_tokens"` and `"max_completion_tokens"` as the JSON key

If the config field is missing (defaults `false`), OpenAI's newer models (o1, o3, o4, gpt-4o, gpt-5.x) return HTTP 400. Users have no way to know this without reading source code.

### 4. `is_provider_available` in `crates/roko-core/src/config/schema.rs:473`

```rust
pub fn is_provider_available(&self, provider: &ProviderConfig) -> bool {
    if matches!(provider.kind, ProviderKind::ClaudeCli | ProviderKind::CursorAcp) {
        return true;
    }
    match provider.api_key_env.as_ref().map(|s| s.trim()) {
        None => false,
        Some("") => true,    // empty string means "no auth required"
        Some(name) => std::env::var(name).is_ok() || self.agent_env_value(name).is_some(),
    }
}
```

This only checks if the env var is SET, not whether the provider is in the registry at all. If `OPENAI_API_KEY` is set but no `[providers.openai]` entry exists, the model is still not usable.

## What a Redesign Looks Like

### Built-in model registry design

New file: `crates/roko-core/src/config/model_registry.rs`

```rust
/// A hardcoded entry for a well-known model.
pub struct BuiltinModel {
    /// Config key aliases (any of these work in --model)
    pub aliases: &'static [&'static str],
    /// Wire slug sent to the API
    pub slug: &'static str,
    /// Provider config key used when the provider is auto-detected
    pub provider_label: &'static str,
    /// Provider kind for auto-constructing a provider entry
    pub provider_kind: ProviderKind,
    /// Well-known base URL (None = use provider default)
    pub base_url: Option<&'static str>,
    /// Environment variable for API key
    pub api_key_env: &'static str,
    pub context_window: u64,
    pub max_output: u64,
    pub supports_tools: bool,
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub tool_format: &'static str,
    /// Auto-detected API quirk: use max_completion_tokens
    pub use_max_completion_tokens: bool,
    pub tier: ModelTier,
    /// Approximate costs (None = unknown)
    pub cost_input_per_m: Option<f64>,
    pub cost_output_per_m: Option<f64>,
}

pub static BUILTIN_MODELS: &[BuiltinModel] = &[
    // ── Anthropic ────────────────────────────────────────────────────────
    BuiltinModel {
        aliases: &["opus", "claude-opus", "claude-opus-4"],
        slug: "claude-opus-4-6",
        provider_label: "anthropic",
        provider_kind: ProviderKind::AnthropicApi,
        base_url: Some("https://api.anthropic.com/v1"),
        api_key_env: "ANTHROPIC_API_KEY",
        context_window: 200_000,
        max_output: 32_768,
        supports_tools: true,
        supports_thinking: true,
        supports_vision: true,
        tool_format: "anthropic_blocks",
        use_max_completion_tokens: false,
        tier: ModelTier::Premium,
        cost_input_per_m: Some(15.0),
        cost_output_per_m: Some(75.0),
    },
    BuiltinModel {
        aliases: &["sonnet", "claude-sonnet", "claude-sonnet-4"],
        slug: "claude-sonnet-4-6",
        provider_label: "anthropic",
        provider_kind: ProviderKind::AnthropicApi,
        base_url: Some("https://api.anthropic.com/v1"),
        api_key_env: "ANTHROPIC_API_KEY",
        context_window: 200_000,
        max_output: 16_384,
        supports_tools: true,
        supports_thinking: false,
        supports_vision: true,
        tool_format: "anthropic_blocks",
        use_max_completion_tokens: false,
        tier: ModelTier::Standard,
        cost_input_per_m: Some(3.0),
        cost_output_per_m: Some(15.0),
    },
    BuiltinModel {
        aliases: &["haiku", "claude-haiku"],
        slug: "claude-haiku-4-5",
        provider_label: "anthropic",
        provider_kind: ProviderKind::AnthropicApi,
        base_url: Some("https://api.anthropic.com/v1"),
        api_key_env: "ANTHROPIC_API_KEY",
        context_window: 200_000,
        max_output: 16_384,
        supports_tools: true,
        supports_thinking: false,
        supports_vision: true,
        tool_format: "anthropic_blocks",
        use_max_completion_tokens: false,
        tier: ModelTier::Fast,
        cost_input_per_m: Some(0.8),
        cost_output_per_m: Some(4.0),
    },
    // ── OpenAI ───────────────────────────────────────────────────────────
    BuiltinModel {
        aliases: &["gpt-5.5", "gpt55"],
        slug: "gpt-5.5",
        provider_label: "openai",
        provider_kind: ProviderKind::OpenAiCompat,
        base_url: Some("https://api.openai.com/v1"),
        api_key_env: "OPENAI_API_KEY",
        context_window: 200_000,
        max_output: 100_000,
        supports_tools: true,
        supports_thinking: false,
        supports_vision: true,
        tool_format: "openai_json",
        use_max_completion_tokens: true,   // ← auto-inferred, not user-facing
        tier: ModelTier::Standard,
        cost_input_per_m: Some(0.4),
        cost_output_per_m: Some(1.6),
    },
    BuiltinModel {
        aliases: &["o3"],
        slug: "o3",
        provider_label: "openai",
        provider_kind: ProviderKind::OpenAiCompat,
        base_url: Some("https://api.openai.com/v1"),
        api_key_env: "OPENAI_API_KEY",
        context_window: 200_000,
        max_output: 100_000,
        supports_tools: true,
        supports_thinking: true,
        supports_vision: false,
        tool_format: "openai_json",
        use_max_completion_tokens: true,
        tier: ModelTier::Premium,
        cost_input_per_m: Some(10.0),
        cost_output_per_m: Some(40.0),
    },
    BuiltinModel {
        aliases: &["o4-mini"],
        slug: "o4-mini",
        provider_label: "openai",
        provider_kind: ProviderKind::OpenAiCompat,
        base_url: Some("https://api.openai.com/v1"),
        api_key_env: "OPENAI_API_KEY",
        context_window: 200_000,
        max_output: 100_000,
        supports_tools: true,
        supports_thinking: true,
        supports_vision: true,
        tool_format: "openai_json",
        use_max_completion_tokens: true,
        tier: ModelTier::Fast,
        cost_input_per_m: Some(1.1),
        cost_output_per_m: Some(4.4),
    },
    // ── Gemini ───────────────────────────────────────────────────────────
    BuiltinModel {
        aliases: &["gemini", "gemini-pro", "gemini-2.5-pro"],
        slug: "gemini-2.5-pro",
        provider_label: "gemini",
        provider_kind: ProviderKind::OpenAiCompat,
        base_url: Some("https://generativelanguage.googleapis.com/v1beta/openai"),
        api_key_env: "GEMINI_API_KEY",
        context_window: 1_048_576,
        max_output: 65_536,
        supports_tools: true,
        supports_thinking: true,
        supports_vision: true,
        tool_format: "openai_json",
        use_max_completion_tokens: false,
        tier: ModelTier::Premium,
        cost_input_per_m: Some(1.25),
        cost_output_per_m: Some(10.0),
    },
    // ... more models
];

/// Look up a builtin model by any of its aliases.
pub fn find_builtin(name: &str) -> Option<&'static BuiltinModel> {
    BUILTIN_MODELS.iter().find(|m| {
        m.aliases.contains(&name) || m.slug == name
    })
}

/// Auto-infer use_max_completion_tokens from slug pattern.
/// This is the provider-level knowledge that should NEVER be user-facing.
pub fn should_use_max_completion_tokens(slug: &str) -> bool {
    slug.starts_with("gpt-4o")
        || slug.starts_with("gpt-5")
        || slug.starts_with("o1")
        || slug.starts_with("o3")
        || slug.starts_with("o4")
        || slug.starts_with("codex")
        // The field is also explicitly set in the builtin registry,
        // but this function handles unknown/future OpenAI models.
}
```

### Updated `resolve_model` with fallthrough to builtin registry

In `crates/roko-core/src/agent.rs`, modify `resolve_model` to try the builtin registry after config miss:

```rust
pub fn resolve_model(config: &RokoConfig, model_key: &str) -> ResolvedModel {
    // 1. Direct lookup by config key
    if let Some(profile) = config.models.get(model_key) { ... }
    // 2. Slug search
    // 3. Prefix match
    // ...existing logic...

    // 4. NEW: Try builtin model registry
    if let Some(builtin) = crate::config::model_registry::find_builtin(model_key) {
        let profile = builtin_to_model_profile(builtin, config);
        // Synthesize a transient provider if not in config
        let provider_config = config.providers.get(builtin.provider_label)
            .cloned()
            .unwrap_or_else(|| builtin_to_provider_config(builtin));
        return ResolvedModel {
            model_key: model_key.to_owned(),
            slug: builtin.slug.to_owned(),
            provider_kind: builtin.provider_kind,
            provider_config: Some(provider_config),
            profile: Some(profile),
            backend: builtin.provider_kind.to_backend(),
        };
    }

    // 5. Heuristic fallback (existing)
    ...
}
```

### Updated `select_provider` — stop rejecting models with no config entry

In `crates/roko-cli/src/model_selection.rs`, `select_provider` currently rejects any model where `resolved.profile.is_none()`. After the registry change, `profile` will be `Some` for builtin models, eliminating most rejections. For truly unknown models, provide a better error with suggestions.

### Auto-infer `use_max_completion_tokens` at the dispatch boundary

In `crates/roko-agent/src/provider/openai_compat.rs:416`, change:

```rust
// Current: reads field from config (defaults false, breaks OpenAI models)
.with_use_max_completion_tokens(model.use_max_completion_tokens)

// Proposed: merge config with auto-inference
.with_use_max_completion_tokens(
    model.use_max_completion_tokens
        || crate::config::model_registry::should_use_max_completion_tokens(&model.slug)
)
```

And in `crates/roko-agent/src/tool_loop/backends/mod.rs:62`:
```rust
// Same pattern — add the auto-inference fallback
.with_use_max_completion_tokens(
    model.use_max_completion_tokens
        || crate::config::model_registry::should_use_max_completion_tokens(&model.slug)
)
```

## Auto-inference from API key environment

When no `[providers.*]` entry exists for a builtin provider, synthesize one if the API key env var is set:

```rust
fn builtin_to_provider_config(builtin: &BuiltinModel) -> ProviderConfig {
    ProviderConfig {
        kind: builtin.provider_kind,
        base_url: builtin.base_url.map(str::to_string),
        api_key_env: Some(builtin.api_key_env.to_string()),
        timeout_ms: default_provider_timeout_ms(),
        ttft_timeout_ms: default_provider_ttft_timeout_ms(),
        connect_timeout_ms: default_provider_connect_timeout_ms(),
        ..ProviderConfig::default()
    }
}
```

The `is_provider_available` check at `schema.rs:473` already works via env var name — so a synthesized provider with `api_key_env = "OPENAI_API_KEY"` will pass the availability check if the env var is set.

## Proposed Solutions (in priority order)

### S5: Auto-detect `use_max_completion_tokens` from slug (HIGHEST PRIORITY)

**Impact**: Eliminates hard HTTP 400 failures for every OpenAI model user.
**File**: `crates/roko-agent/src/provider/openai_compat.rs:416`
**Change**: ~3 lines

### S1: Auto-infer provider from model name (MUST-SHIP)

**Impact**: Eliminates "add an explicit [models.*] entry" error for all builtin models.
**Files**:
- New `crates/roko-core/src/config/model_registry.rs` (builtin registry, ~200 lines)
- `crates/roko-core/src/agent.rs:304` (resolve_model, ~20 lines)
- `crates/roko-cli/src/model_selection.rs:362` (select_provider, ~10 lines)

### S3: Built-in model catalog in binary

**Impact**: Zero TOML editing for any standard OpenAI/Anthropic/Gemini model.
**Implementation**: The `BUILTIN_MODELS` static above. User config overrides take precedence.

### S2: `roko config models add` interactive command

```
$ roko config models add
? Model name (as you'll reference it): gpt55
? API model ID [gpt-5.5]:
? Provider [auto-detected: openai]:
? Context window [200000]:
✓ Added [models.gpt55] to roko.toml
```

### S4: `roko models list` shows available + configured

```
$ roko models list
NAME             PROVIDER    SLUG                    CTX     TIER     KEY STATUS
─────────────────────────────────────────────────────────────────────────────────
opus             anthropic   claude-opus-4-6         200k    premium  ✓ ANTHROPIC_API_KEY
sonnet           anthropic   claude-sonnet-4-6       200k    std      ✓ ANTHROPIC_API_KEY
haiku            anthropic   claude-haiku-4-5        200k    fast     ✓ ANTHROPIC_API_KEY
gpt55            openai      gpt-5.5                 200k    std      ✓ OPENAI_API_KEY
o3               openai      o3                      200k    premium  ✓ OPENAI_API_KEY
gemini           gemini      gemini-2.5-pro          1M      premium  ✗ GEMINI_API_KEY (not set)
llama32          ollama      llama3.2:latest         8k      fast     ⚠ local
─────────────────────────────────────────────────────────────────────────────────
BUILTIN (usable without config):
  claude-opus    anthropic   claude-opus-4-6         ✓ ANTHROPIC_API_KEY
  claude-sonnet  anthropic   claude-sonnet-4-6       ✓ ANTHROPIC_API_KEY
  gpt-5.5        openai      gpt-5.5                 ✓ OPENAI_API_KEY
  ...

Default: gpt54-mini (change with: roko config set agent.default_model <name>)
```

## Current State of `roko.toml`

The current `roko.toml` has:
- **36 `[models.*]` sections** (lines 129–921), each with 20-22 fields = ~700 lines of model config
- **11 `[providers.*]` sections** (lines 24–127), each with 5-7 fields = ~70 lines of provider config
- Total model+provider config: ~770 lines that should ideally be 0 lines for standard providers

After implementing the builtin registry:
- Standard providers (openai, anthropic, gemini): 0 lines needed
- Standard models: 0 lines needed, but can be overridden
- Custom/local models (ollama, zhipu, etc.): still need explicit entries
- Total for a typical user: ~5-10 lines

## Priority

**S5 is the highest priority fix.** It causes hard failures (HTTP 400, cryptic JSON error) for the most common use case (using any OpenAI model). Zero users will figure this out.

**S1 + S3 are must-ship.** They eliminate the most common failure mode (user passes a valid model name, roko rejects it). Implementation: ~250 lines total.

S2 and S4 are nice-to-haves for discoverability.
