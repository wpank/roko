# 02 — Provider Registry & Config

> **Priority**: 🔴 P0 — Foundation for all other model routing work
> **Status**: Not started
> **Depends on**: None
> **Blocks**: 03 (adapters), 05 (GLM), 06 (Kimi), 07 (OpenRouter), 09 (costs)

## Problem Statement

`roko.toml` has a flat `[agent]` section with a single `model`, `command`, `env` array, and tier_models table. There is no way to configure multiple providers, per-provider auth, per-model capabilities, or per-model cost data. Adding GLM-5.1 or Kimi-K2.5 currently requires editing `from_model()` in Rust code.

> **Cross-references**:
> - Doc 14 (2L.06–2L.07): Token counting needs `ModelProfile` for tokenizer selection
> - Doc 14 (2L.12–2L.13): Fallback chains need per-model provider info
> - Doc 15 (2M.06–2M.07): Config validation needs cross-reference checking
> - Doc 15 (2M.15–2M.16): Config migration auto-generates `[providers.*]` from old `[agent]`
> - Existing `crates/roko-cli/src/config.rs` has its own `Config` struct (1,911 LOC) that wraps
>   `roko-core`'s `RokoConfig`. New fields must be added to BOTH or unified.

## What Exists

| Component | Path | Status |
|---|---|---|
| AgentConfig struct | `crates/roko-core/src/config/schema.rs` ~L490 | 🔌 Flat, single-provider |
| RokoConfig::agent field | `crates/roko-core/src/config/schema.rs` ~L33 | 🔌 Single agent section |
| `[agent]` in roko.toml | `roko.toml` L3–14 | 🔌 Claude-only |
| `[agent.tier_models]` | `roko.toml` L22–26 | 🔌 Claude slugs only |
| RoleOverride struct | `crates/roko-core/src/config/schema.rs` | 🔌 model/backend/effort |
| Env var overrides | `crates/roko-core/src/config/schema.rs` ~L324 | 🔌 ROKO_MODEL, ROKO_BACKEND |

## Target TOML Schema

```toml
# ── Providers: where requests go ──────────────────────────
[providers.anthropic]
kind = "anthropic_api"
base_url = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"
timeout_ms = 120000

[providers.zai]
kind = "openai_compat"
base_url = "https://api.z.ai/api/paas/v4"
api_key_env = "ZAI_API_KEY"
timeout_ms = 180000

[providers.moonshot]
kind = "openai_compat"
base_url = "https://api.moonshot.ai/v1"
api_key_env = "MOONSHOT_API_KEY"

[providers.openrouter]
kind = "openai_compat"
base_url = "https://openrouter.ai/api/v1"
api_key_env = "OPENROUTER_API_KEY"
extra_headers = { "HTTP-Referer" = "roko-agent" }

[providers.ollama]
kind = "openai_compat"
base_url = "http://localhost:11434"
# no api_key_env needed

[providers.claude_cli]
kind = "claude_cli"
command = "claude"
args = ["--print", "--output-format", "stream-json"]

# ── Models: what capabilities they have ───────────────────
[models.claude-opus]
provider = "anthropic"
slug = "claude-opus-4-6"
context_window = 200000
max_output = 32768
supports_tools = true
supports_thinking = false
tool_format = "anthropic_blocks"
cost_input_per_m = 15.00
cost_output_per_m = 75.00

[models.glm-5-1]
provider = "zai"
slug = "glm-5.1"
context_window = 200000
max_output = 131072
supports_tools = true
supports_thinking = true
supports_web_search = true
supports_mcp_tools = true
tool_format = "openai_json"
cost_input_per_m = 1.40
cost_output_per_m = 4.40
cost_cache_read_per_m = 0.26

[models.kimi-k2-5]
provider = "moonshot"
slug = "kimi-k2.5"
context_window = 256000
max_output = 65535
supports_tools = true
supports_thinking = true
supports_vision = true
supports_partial = true
tool_format = "openai_json"
cost_input_per_m = 0.60
cost_output_per_m = 3.00
cost_cache_read_per_m = 0.10

[models.glm-5-1-or]
provider = "openrouter"
slug = "z-ai/glm-5.1"
context_window = 200000
max_output = 131072
supports_tools = true
supports_thinking = true
tool_format = "openai_json"
cost_input_per_m = 1.26
cost_output_per_m = 3.96

# ── Backwards-compatible agent section ────────────────────
[agent]
default_model = "glm-5-1"
fallback_model = "kimi-k2-5"
effort = "high"
bare_mode = true
timeout_ms = 300000

[agent.tier_models]
mechanical = "kimi-k2-5"
focused = "glm-5-1"
integrative = "glm-5-1"
architectural = "claude-opus"
```

---

## Checklist

### 2A.01 — Define ProviderKind enum

**File**: `crates/roko-core/src/agent.rs`
**What**: Add a `ProviderKind` enum representing protocol families. This replaces `AgentBackend` as the primary dispatch mechanism.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    AnthropicApi,   // Anthropic Messages API (HTTP)
    ClaudeCli,      // claude CLI subprocess
    OpenAiCompat,   // OpenAI chat completions (HTTP)
    CursorAcp,      // Cursor Agent Client Protocol
}
```

Add `impl ProviderKind { pub fn label(&self) -> &'static str }` and `Display`.

**Acceptance**: `ProviderKind` compiles, has Serialize/Deserialize, and all 4 variants exist.
**Verification**: `cargo test -p roko-core -- provider_kind`

---

### 2A.02 — Define ProviderConfig struct

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add `ProviderConfig` for a single provider entry from `[providers.*]`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,     // env var name, not the key itself
    pub command: Option<String>,          // for claude_cli
    pub args: Option<Vec<String>>,        // for claude_cli
    pub timeout_ms: Option<u64>,
    pub extra_headers: Option<HashMap<String, String>>,
    pub max_concurrent: Option<u32>,      // concurrent request limit
}
```

**Acceptance**: Struct compiles, serializes/deserializes, all fields optional except `kind`.
**Verification**: `cargo test -p roko-core -- provider_config`

---

### 2A.03 — Define ModelProfile struct

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add `ModelProfile` for a single model entry from `[models.*]`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub provider: String,                 // key into providers table
    pub slug: String,                     // model ID sent to API
    #[serde(default = "default_context_window")]
    pub context_window: u64,
    pub max_output: Option<u64>,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_thinking: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_web_search: bool,
    #[serde(default)]
    pub supports_mcp_tools: bool,
    #[serde(default)]
    pub supports_partial: bool,           // Kimi partial continuation
    #[serde(default = "default_tool_format")]
    pub tool_format: String,              // "openai_json", "anthropic_blocks", "react_text"
    pub cost_input_per_m: Option<f64>,
    pub cost_output_per_m: Option<f64>,
    pub cost_cache_read_per_m: Option<f64>,
    pub cost_cache_write_per_m: Option<f64>,
    pub max_tools: Option<u32>,           // max tools before degradation
    pub tokenizer_ratio: Option<f64>,     // ratio vs OpenAI o200k_base
}
```

**Acceptance**: Struct compiles with serde defaults. `default_context_window` = 128000, `default_tool_format` = `"openai_json"`.
**Verification**: `cargo test -p roko-core -- model_profile`

---

### 2A.04 — Add providers and models to RokoConfig

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add two new optional fields to `RokoConfig`:

```rust
pub struct RokoConfig {
    // ... existing fields ...
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub models: HashMap<String, ModelProfile>,
}
```

Must be `#[serde(default)]` so existing `roko.toml` files without these sections still parse.

**Acceptance**: Existing `roko.toml` (with only `[agent]`) still loads. New `roko.toml` with `[providers.*]` and `[models.*]` tables also loads.
**Verification**:
```bash
# Existing config still works
cargo test -p roko-core -- config_load
# New config with providers loads
cargo test -p roko-core -- config_providers
```

---

### 2A.05 — Implement model resolution: config lookup with slug fallback

**File**: `crates/roko-core/src/agent.rs`
**What**: Add `resolve_model(config: &RokoConfig, model_key: &str) -> ResolvedModel` that:
1. Checks `config.models` for an exact key match → returns `ModelProfile` + `ProviderConfig`
2. Falls back to `AgentBackend::from_model()` heuristic (existing logic)
3. Returns a `ResolvedModel` struct with all resolved info

```rust
pub struct ResolvedModel {
    pub model_key: String,               // the key used in config (e.g., "glm-5-1")
    pub slug: String,                    // API model ID (e.g., "glm-5.1")
    pub provider_kind: ProviderKind,
    pub provider_config: Option<ProviderConfig>,
    pub profile: Option<ModelProfile>,
    pub backend: AgentBackend,           // legacy, for backwards compat
}
```

**Context**: This is the bridge between the old `AgentBackend::from_model()` and the new provider system. When `[models.*]` config exists, it takes priority. When it doesn't, the old slug heuristic kicks in.

**Acceptance**: `resolve_model(config, "glm-5-1")` returns the Z.AI provider config. `resolve_model(config, "claude-sonnet-4-6")` falls back to `AgentBackend::Claude`.
**Verification**: `cargo test -p roko-core -- resolve_model`

---

### 2A.06 — Parse api_key_env from environment at resolution time

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add method `ProviderConfig::resolve_api_key(&self) -> Option<String>` that reads the env var named by `api_key_env`.

```rust
impl ProviderConfig {
    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key_env.as_ref().and_then(|env_name| std::env::var(env_name).ok())
    }
}
```

Never store API keys in the config struct. Only store the env var name.

**Acceptance**: With `ZAI_API_KEY=test123` in env, `config.resolve_api_key()` returns `Some("test123")`. Without the env var, returns `None`.
**Verification**: `cargo test -p roko-core -- resolve_api_key`

---

### 2A.07 — Add default provider configs for backwards compatibility

**File**: `crates/roko-core/src/config/schema.rs`
**What**: When `config.providers` is empty (old-style config), synthesize default providers from the existing `[agent]` section:

```rust
impl RokoConfig {
    pub fn effective_providers(&self) -> HashMap<String, ProviderConfig> {
        if !self.providers.is_empty() {
            return self.providers.clone();
        }
        // Synthesize from [agent] section:
        // - "claude_cli" provider from agent.command + agent.args + agent.env
        // - "anthropic" provider from agent.env ANTHROPIC_BASE_URL if present
        let mut providers = HashMap::new();
        // ... build defaults ...
        providers
    }
}
```

**Context**: The existing `roko.toml` has `agent.command = "claude"` and `agent.env` with `ANTHROPIC_BASE_URL`. This must continue to work when no `[providers.*]` section exists.

**Acceptance**: `RokoConfig` loaded from existing `roko.toml` returns valid `effective_providers()` with a `claude_cli` entry.
**Verification**: `cargo test -p roko-core -- effective_providers_backwards_compat`

---

### 2A.08 — Add default model profiles for backwards compatibility

**File**: `crates/roko-core/src/config/schema.rs`
**What**: When `config.models` is empty, synthesize default model profiles from `[agent.tier_models]`:

```rust
impl RokoConfig {
    pub fn effective_models(&self) -> HashMap<String, ModelProfile> {
        if !self.models.is_empty() {
            return self.models.clone();
        }
        // Synthesize from agent.tier_models + agent.default_model
        let mut models = HashMap::new();
        // Map each tier model slug to a ModelProfile with inferred capabilities
        models
    }
}
```

**Acceptance**: Existing config with `agent.tier_models.mechanical = "claude-haiku-4-5"` produces a `ModelProfile` for `"claude-haiku-4-5"` with `provider = "claude_cli"`.
**Verification**: `cargo test -p roko-core -- effective_models_backwards_compat`

---

### 2A.09 — Add ROKO_PROVIDER and ROKO_MODEL_SLUG env var overrides

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Extend the existing env var override system (L324–361) with:
- `ROKO_PROVIDER` → override the provider for the default model
- `ROKO_MODEL_SLUG` → override the slug sent to the API

These allow runtime override without editing config:
```bash
ROKO_PROVIDER=openrouter ROKO_MODEL_SLUG=z-ai/glm-5.1 cargo run -p roko-cli -- run "task"
```

**Acceptance**: Setting `ROKO_PROVIDER=openrouter` changes the resolved provider for `agent.default_model`.
**Verification**: `cargo test -p roko-core -- env_override_provider`

---

### 2A.10 — Write comprehensive deserialization tests

**File**: `crates/roko-core/src/config/schema.rs` (tests module)
**What**: Add test cases for:
1. Full config with providers + models
2. Minimal config (only `[agent]`, no providers/models)
3. Mixed config (some providers, no models)
4. Invalid provider kind → descriptive error
5. Missing required fields → descriptive error
6. Env var references in api_key_env

Each test uses `toml::from_str::<RokoConfig>(...)` with inline TOML strings.

**Acceptance**: All 6 test cases pass. Error messages include the field name and expected type.
**Verification**: `cargo test -p roko-core -- config_deser`

---

### 2A.11 — Document the new TOML schema in a config reference comment

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add a doc comment block at the top of the `ProviderConfig` and `ModelProfile` structs explaining all fields, their defaults, and examples. This serves as inline documentation for agents editing these types.

**Acceptance**: `cargo doc -p roko-core` generates readable docs for ProviderConfig and ModelProfile.
**Verification**: `cargo doc -p roko-core --no-deps 2>&1 | grep -c warning` should not increase.

---

### 2A.12 — Add example configs for GLM, Kimi, OpenRouter, Ollama

**File**: `examples/roko-glm.toml`, `examples/roko-kimi.toml`, `examples/roko-openrouter.toml`, `examples/roko-ollama.toml`
**What**: Create 4 example config files showing how to configure each provider. These serve as documentation and test fixtures.

Each file should be a complete, valid `roko.toml` with the relevant `[providers.*]` and `[models.*]` entries, plus the standard `[agent]`, `[prompt]`, `[gates]` sections.

**Acceptance**: Each example parses without errors: `cargo test -p roko-core -- example_configs`
**Verification**:
```bash
for f in examples/roko-*.toml; do
  cargo run -p roko-cli -- config show --config "$f" 2>&1 | head -5
done
```
