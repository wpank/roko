# HuggingFace Provider Integration

**Priority**: P3
**Size**: XS for Phase 1 (config only), L for Phase 2 (roko-hf crate)

---

## Problem

HuggingFace hosts thousands of open-source models (Llama, Mistral, Qwen, DeepSeek,
Phi, Gemma, etc.) through an OpenAI-compatible Inference Providers API at
`https://router.huggingface.co/v1`. Roko already has a fully functional
`OpenAiCompat` provider backend that handles arbitrary `base_url` and `api_key_env`
values. Adding HuggingFace as a provider requires zero code changes: it is a pure
config entry pointing the existing backend at HF's router.

No HuggingFace support exists anywhere in the codebase today. Search results for
`huggingface`, `HF_TOKEN`, and `hf_token` return only `tmp/` documentation files —
none in `crates/`.

---

## What already exists

### Provider dispatch path (no changes needed)

`ProviderConfig` in `crates/roko-core/src/config/provider.rs` has two fields that
are all that's needed:

```rust
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    // ...
}
```

`ProviderConfig::resolve_api_key()` reads `std::env::var(api_key_env)` at runtime.

`create_openai_compat_backend` in
`crates/roko-agent/src/tool_loop/backends/mod.rs` (line 48) constructs the full
request path from those two fields for any `kind = "openai_compat"` provider:

```rust
let api_key = resolve_api_key(provider)?;         // reads env var named by api_key_env
let base_url = base_url_for_tool_loop(provider);  // provider.base_url or OpenAI default
let backend = OpenAiCompatBackend::new(api_key, model.slug.clone())
    .with_base_url(base_url)
    // ...
```

`base_url_for_tool_loop` in `crates/roko-agent/src/provider/openai_compat.rs`
(line 221) returns `provider.base_url.clone().unwrap_or("https://api.openai.com/v1")`.

### Existing OpenAI-compat provider examples in roko.toml

`demo/demo-resources/provider-routing/roko.toml` shows the pattern in use:

```toml
[providers.moonshot]
kind = "openai_compat"
base_url = "https://api.moonshot.ai/v1"
api_key_env = "MOONSHOT_API_KEY"
timeout_ms = 180000
```

HuggingFace is structurally identical.

---

## Phase 1: Config-only integration (zero code changes)

Add the following stanza to `roko.toml` (or any per-project config):

```toml
[providers.huggingface]
kind = "openai_compat"
base_url = "https://router.huggingface.co/v1"
api_key_env = "HF_TOKEN"
timeout_ms = 120000
```

Then register models. HuggingFace's router passes slugs directly to backend
providers, so the slug must match HuggingFace's naming convention
(`owner/model-id` or provider-qualified `provider:owner/model-id`):

```toml
# Llama 3.3 70B — routed to fastest available backend
[models.llama-3-3-70b]
provider = "huggingface"
slug = "meta-llama/Llama-3.3-70B-Instruct"
context_window = 131072
max_output = 4096
supports_tools = true
tool_format = "openai_json"
cost_input_per_m = 0.00   # variable; set to HF pricing if known

# DeepSeek-R1 — open weights reasoning model
[models.deepseek-r1]
provider = "huggingface"
slug = "deepseek-ai/DeepSeek-R1"
context_window = 131072
max_output = 8192
supports_tools = false
supports_thinking = true
tool_format = "openai_json"

# Qwen 2.5 72B Instruct — top open-weight coding model
[models.qwen-2-5-72b]
provider = "huggingface"
slug = "Qwen/Qwen2.5-72B-Instruct"
context_window = 131072
max_output = 8192
supports_tools = true
tool_format = "openai_json"

# Mistral Small — compact, fast
[models.mistral-small]
provider = "huggingface"
slug = "mistralai/Mistral-Small-3.1-24B-Instruct-2503"
context_window = 131072
max_output = 4096
supports_tools = true
tool_format = "openai_json"
```

HuggingFace's router also accepts routing policy suffixes on the slug:
- `:fastest` — lowest latency across all backends
- `:cheapest` — lowest cost across all backends
- Explicit backend prefix: `together:meta-llama/Llama-3.3-70B-Instruct`

These are appended to the `slug` field in `[models.*]`.

### HuggingFace Inference Endpoints (custom deployments)

For dedicated GPU instances (scale-to-zero, custom fine-tunes), override `base_url`
per model using a separate provider entry:

```toml
[providers.hf-endpoint-my-model]
kind = "openai_compat"
base_url = "https://ENDPOINT-ID.us-east-1.aws.endpoints.huggingface.cloud/v1"
api_key_env = "HF_TOKEN"
timeout_ms = 300000

[models.my-finetune]
provider = "hf-endpoint-my-model"
slug = "tgi"   # endpoint slug is always "tgi" for TGI-backed endpoints
context_window = 32768
supports_tools = true
tool_format = "openai_json"
```

### Verification (Phase 1)

```bash
# 1. Set credentials
export HF_TOKEN="hf_..."

# 2. Add provider + model to roko.toml (as above)

# 3. Verify provider is visible
cargo run -p roko-cli -- config providers list
# → should show huggingface with kind=openai_compat

# 4. Smoke test with a single prompt
cargo run -p roko-cli -- run "Say hello in one sentence." --model llama-3-3-70b

# 5. Verify model routing
cargo run -p roko-cli -- config models route llama-3-3-70b
# → should show provider=huggingface, base_url=router.huggingface.co/v1
```

---

## Phase 2: `roko-hf` crate (optional, deferred)

Phase 1 covers the primary use case (running HF-hosted models). Phase 2 would add
HuggingFace-specific capabilities that require new code.

### 2a: Hub API client (model discovery)

**File**: `crates/roko-hf/src/hub.rs`

```
GET https://huggingface.co/api/models?search=<query>&library=transformers&sort=downloads
```

Use case: auto-populate `[models.*]` entries in roko.toml from search results,
eliminating manual slug lookup. The Hub API returns context window size and model
card metadata; these map directly to `ModelProfile` fields.

Integration point: `roko config models list --discover-hf <query>` would call the
Hub API and print a ready-to-paste TOML block.

### 2b: Dataset viewer client (benchmark streaming)

**File**: `crates/roko-hf/src/datasets.rs`

```
GET https://datasets-server.huggingface.co/rows?dataset=princeton-nlp/SWE-bench&split=test&offset=0&length=100
```

Use case: stream benchmark instances (SWE-bench, HumanEval, MBPP) directly into
the arena evaluation loop without a Python `datasets` dependency. The REST API
returns Parquet rows as JSON.

Integration point: `roko arena eval --dataset hf://princeton-nlp/SWE-bench`

### 2c: SWE-bench adapter

**File**: `crates/roko-hf/src/swe_bench.rs`

An adapter that:
1. Fetches SWE-bench instances from the dataset viewer
2. Formats each instance as a roko task (apply patch, run tests, report pass/fail)
3. Records outcome to the arena leaderboard

This enables measuring roko's own coding performance against the standard benchmark
using any HF-hosted model.

### 2d: Inference Endpoints lifecycle management

**File**: `crates/roko-hf/src/endpoints.rs`

Spin up and tear down managed GPU instances via the HuggingFace Endpoints API:

```
POST /endpoint → create endpoint (returns endpoint URL)
GET  /endpoint/{id}/status → poll until "running"
DELETE /endpoint/{id} → teardown after batch run
```

Use case: run a batch of benchmark tasks on a dedicated GPU (no cold-start latency),
then tear down to avoid idle billing. Pairs with the SWE-bench adapter.

### Phase 2 acceptance criteria

1. `cargo add roko-hf` works as a library dependency (published crate or workspace member)
2. `roko config models list --discover-hf "qwen 72b"` returns a pasteable TOML block
3. `roko arena eval --dataset hf://princeton-nlp/SWE-bench --limit 10` runs 10 instances
4. `cargo test -p roko-hf` passes with no network calls (mock HTTP in tests)
5. `cargo clippy -p roko-hf -- -D warnings` is clean

---

## Acceptance criteria (Phase 1)

1. `roko.toml` with a `[providers.huggingface]` entry (kind=openai_compat, correct base_url,
   api_key_env=HF_TOKEN) causes `roko config providers list` to show the provider.
2. `roko run "<prompt>" --model llama-3-3-70b` (or any HF-backed model alias) succeeds
   when `HF_TOKEN` is set.
3. No code changes to any `crates/` file are required.
4. Auth error is surfaced clearly when `HF_TOKEN` is missing (existing
   `AgentCreationError::MissingApiKey` path in `resolve_api_key` already handles this).

---

## References

- `crates/roko-agent/src/provider/openai_compat.rs` — `resolve_api_key`,
  `base_url_for_tool_loop`, `build_extra_body_params`
- `crates/roko-agent/src/tool_loop/backends/mod.rs` — `create_openai_compat_backend`
  (line 48), reads `provider.base_url` and `provider.api_key_env` directly
- `crates/roko-core/src/config/provider.rs` — `ProviderConfig` struct with
  `base_url`, `api_key_env`, `resolve_api_key()`
- `demo/demo-resources/provider-routing/roko.toml` — working examples of
  OpenAI-compat providers (moonshot, zai, ollama)
- `tmp/subsystem-audits/gateway/PROVIDERS.md` — HuggingFace layer breakdown
  (Layer 1 = Inference Providers, Layer 2 = Hub API, Layer 3 = Dataset Viewer, etc.)
