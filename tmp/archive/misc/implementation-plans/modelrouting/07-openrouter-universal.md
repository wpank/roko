# 07 — OpenRouter Universal Backend

> **Priority**: 🟡 P1 — Single endpoint for 300+ models, ideal for CascadeRouter exploration
> **Status**: Not started
> **Depends on**: 02 (registry), 03 (adapters)
> **Blocks**: None

## Problem Statement

OpenRouter provides a single OpenAI-compatible endpoint for 300+ models from 60+ providers. It normalizes tool calling, handles failover, and provides model metadata. Using it as a universal backend means one API key gives access to GLM-5.1 ($1.26/M), Kimi-K2.5 ($0.38/M), Claude ($15/M), GPT ($2/M), and hundreds more — all through the same `openai_compat` adapter.

This is particularly valuable for the CascadeRouter: it can explore many models through one endpoint and learn which performs best per task type, without requiring separate API keys for each provider.

## OpenRouter Specifics

| Feature | Detail |
|---|---|
| Base URL | `https://openrouter.ai/api/v1` |
| Auth | `Authorization: Bearer {OPENROUTER_API_KEY}` |
| Required headers | `HTTP-Referer` (site URL), `X-Title` (app name) |
| Model IDs | `z-ai/glm-5.1`, `moonshotai/kimi-k2.5`, `anthropic/claude-opus-4-6` |
| Tool calling | Standard OpenAI format, auto-translated for all providers |
| Routing params | `provider.sort`, `provider.order`, `provider.allow_fallbacks`, `provider.max_price` |
| Finish reasons | Normalized to: `stop`, `length`, `tool_calls`, `content_filter`, `error` |
| Pricing | Per-model, generally 10% markup over direct |

---

## Checklist

### 2F.01 — Add OpenRouter provider config example

**File**: `examples/roko-openrouter.toml`
**What**: Complete example with multiple models through one provider:

```toml
[providers.openrouter]
kind = "openai_compat"
base_url = "https://openrouter.ai/api/v1"
api_key_env = "OPENROUTER_API_KEY"
extra_headers = { "HTTP-Referer" = "https://github.com/nunchi/roko", "X-Title" = "roko-agent" }

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

[models.kimi-k2-5-or]
provider = "openrouter"
slug = "moonshotai/kimi-k2.5"
context_window = 256000
max_output = 65535
supports_tools = true
supports_thinking = true
tool_format = "openai_json"
cost_input_per_m = 0.38
cost_output_per_m = 1.72

[models.claude-opus-or]
provider = "openrouter"
slug = "anthropic/claude-opus-4-6"
context_window = 200000
max_output = 32768
supports_tools = true
tool_format = "openai_json"
cost_input_per_m = 15.00
cost_output_per_m = 75.00

[agent]
default_model = "glm-5-1-or"
fallback_model = "kimi-k2-5-or"

[agent.tier_models]
mechanical = "kimi-k2-5-or"
focused = "glm-5-1-or"
integrative = "glm-5-1-or"
architectural = "claude-opus-or"
```

**Acceptance**: Config parses. All 3 models resolve to the same openrouter provider.
**Verification**: `cargo test -p roko-core -- openrouter_config`

---

### 2F.02 — Add OpenRouter provider routing parameters

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add optional OpenRouter-specific routing parameters to `ModelProfile`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderRouting {
    pub sort: Option<String>,            // "price", "throughput", "latency"
    pub order: Option<Vec<String>>,      // explicit provider order
    pub allow_fallbacks: Option<bool>,
    pub max_price: Option<f64>,          // max cost per token
    pub require_parameters: Option<Vec<String>>,
}
```

Add to `ModelProfile`:
```rust
pub provider_routing: Option<ProviderRouting>,
```

This maps to the `provider` field in OpenRouter requests:
```json
{
  "model": "z-ai/glm-5.1",
  "provider": {"sort": "price", "allow_fallbacks": true}
}
```

**Acceptance**: `ProviderRouting` serializes to the correct JSON structure.
**Verification**: `cargo test -p roko-core -- provider_routing`

---

### 2F.03 — Inject provider routing into OpenRouter requests

**File**: `crates/roko-agent/src/provider/openai_compat.rs`
**What**: When the provider is OpenRouter and the model has `provider_routing`, inject it into the request body.

**Context**: OpenRouter uses a nested `provider` object in the request body to control routing. This is OpenRouter-specific and should only be injected when the provider base_url matches OpenRouter.

```rust
fn is_openrouter(base_url: &str) -> bool {
    base_url.contains("openrouter.ai")
}
```

**Acceptance**: Request to OpenRouter includes `provider` routing params.
**Verification**: `cargo test -p roko-agent -- openrouter_routing_injection`

---

### 2F.04 — Add OpenRouter model metadata fetching (optional)

**File**: `crates/roko-agent/src/provider/openrouter_meta.rs` (new)
**What**: Optional utility to fetch model capabilities from OpenRouter's `/models` API:

```rust
pub async fn fetch_model_metadata(api_key: &str, model_id: &str) -> Result<ModelProfile> {
    // GET https://openrouter.ai/api/v1/models/{model_id}
    // Parse: context_length, pricing, supported_parameters
    // Return as ModelProfile
}
```

**Context**: OpenRouter's `/models` endpoint returns per-model metadata including supported parameters, pricing, and context limits. This could auto-populate `ModelProfile` entries, reducing manual config.

**Acceptance**: `fetch_model_metadata("z-ai/glm-5.1")` returns a valid `ModelProfile`.
**Verification**: `cargo test -p roko-agent -- openrouter_meta_fetch` (mock test)

---

### 2F.05 — Add OpenRouter cost data

**File**: `crates/roko-learn/src/costs_db.rs`
**What**: Add OpenRouter-specific pricing for common models:

| Model | Input/M | Output/M |
|---|---|---|
| z-ai/glm-5.1 | 1.26 | 3.96 |
| moonshotai/kimi-k2.5 | 0.38 | 1.72 |
| anthropic/claude-opus-4-6 | 15.00 | 75.00 |

**Acceptance**: Cost lookup for OpenRouter model slugs returns correct rates.
**Verification**: `cargo test -p roko-learn -- openrouter_cost_table`

---

### 2F.06 — Write mock integration test: OpenRouter GLM-5.1

**File**: `crates/roko-agent/tests/openrouter_integration.rs` (new)
**What**: Test creating and running a GLM-5.1 agent through OpenRouter. Mock verifies:
- `HTTP-Referer` header is set
- `X-Title` header is set
- Model slug is `z-ai/glm-5.1`
- Provider routing params are included

**Acceptance**: Mock request has correct headers and model slug.
**Verification**: `cargo test -p roko-agent -- openrouter_glm`

---

### 2F.07 — Write mock test: OpenRouter with provider fallback

**File**: `crates/roko-agent/tests/openrouter_integration.rs`
**What**: Test the case where OpenRouter returns a different model than requested (fallback):

```json
{
  "model": "z-ai/glm-5",  // fell back from glm-5.1
  "choices": [...]
}
```

Verify `ChatResponse.metadata.model_used` captures the actual model.

**Acceptance**: Fallback model is recorded in metadata.
**Verification**: `cargo test -p roko-agent -- openrouter_fallback`

---

### 2F.08 — Document OpenRouter multi-model exploration workflow

**File**: `examples/roko-openrouter.toml` (append)
**What**: Add comments to the example config explaining how CascadeRouter uses OpenRouter:

```toml
# OpenRouter Multi-Model Exploration
#
# With this config, CascadeRouter can learn across 3 models through one endpoint:
#   - kimi-k2-5-or  ($0.38/M input)  - cheapest, good for mechanical tasks
#   - glm-5-1-or    ($1.26/M input)  - best SWE-Bench Pro, good for focused tasks
#   - claude-opus-or ($15.00/M input) - premium, for architectural decisions
#
# The router starts with static tier assignments, then learns pass rates via
# Wilson confidence intervals (50-200 observations), and finally uses LinUCB
# contextual bandit (200+ observations) to route optimally.
#
# To watch router learning:
#   cargo run -p roko-cli -- dashboard  (check routing page)
#
# Router state persists to .roko/learn/cascade-router.json
```

**Acceptance**: Comments are accurate and helpful.
**Verification**: File is readable and config still parses.
