# 59 — HuggingFace Provider Integration

**Priority**: P3 — useful capability addition, no blockers on other work
**Size**: XS for Phase 1 (config only, no code changes); L for Phase 2 (new crate)
**Crates**: None for Phase 1; `crates/roko-hf/` (new) for Phase 2
**Depends on**: None

---

## Background

HuggingFace hosts thousands of open-source models (Llama, Mistral, Qwen, DeepSeek, Phi, Gemma, and others) through an OpenAI-compatible Inference Providers API at `https://router.huggingface.co/v1`. The router accepts standard OpenAI-format requests (`POST /chat/completions`) with a HuggingFace token (`HF_TOKEN`) as the bearer credential, and returns standard OpenAI-format responses.

Roko already has a fully functional `OpenAiCompat` provider backend (`ProviderKind::OpenAiCompat`) that handles arbitrary `base_url` and `api_key_env` values. The existing `create_openai_compat_backend` function in `crates/roko-agent/src/tool_loop/backends/mod.rs` reads both fields and constructs the HTTP client. Phase 1 of this item therefore requires zero Rust code changes — it is a pure configuration addition to `roko.toml`.

Phase 2, deferred and optional, would add a `roko-hf` crate with a HuggingFace Hub API client for model discovery, a Dataset Viewer client for streaming benchmark datasets, a SWE-bench adapter for coding evaluation, and an Inference Endpoints lifecycle manager for dedicated GPU instances.

## Current State

1. No HuggingFace support exists anywhere in `crates/`. A search for `huggingface`, `HF_TOKEN`, and `hf_token` in `crates/` returns zero results.

2. `ProviderConfig` in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/provider.rs:275-295` has `kind: ProviderKind`, `base_url: Option<String>`, and `api_key_env: Option<String>`. Setting `kind = "openai_compat"`, `base_url = "https://router.huggingface.co/v1"`, and `api_key_env = "HF_TOKEN"` is all that is needed to route requests through the HF Inference API.

3. `resolve_api_key` in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openai_compat.rs:193-208` reads `std::env::var(api_key_env)` at dispatch time and returns `AgentCreationError::MissingApiKey` when the variable is absent. This surfaces a clear error to the user when `HF_TOKEN` is not set.

4. `base_url_for_tool_loop` at `openai_compat.rs:221-226` returns `provider.base_url.clone().unwrap_or("https://api.openai.com/v1")`. Any non-`None` `base_url` overrides the default, so the HF router URL is used directly.

5. A working example of the same pattern exists in `/Users/will/dev/nunchi/roko/roko/demo/demo-resources/provider-routing/roko.toml:37-41`:
   ```toml
   [providers.moonshot]
   kind = "openai_compat"
   base_url = "https://api.moonshot.ai/v1"
   api_key_env = "MOONSHOT_API_KEY"
   timeout_ms = 180000
   ```
   HuggingFace is structurally identical.

## Implementation Plan

### Phase 1: Config-only (zero code changes)

**Step 1.** Add a `[providers.huggingface]` entry to `/Users/will/dev/nunchi/roko/roko/roko.toml` (or any per-project config file):

```toml
[providers.huggingface]
kind = "openai_compat"
base_url = "https://router.huggingface.co/v1"
api_key_env = "HF_TOKEN"
timeout_ms = 120000
```

**Step 2.** Register one or more models under `[models.*]`. HuggingFace's router passes the slug directly to the backend provider, so the slug must match HuggingFace's naming convention (`owner/model-id`, or with a backend prefix: `together:meta-llama/Llama-3.3-70B-Instruct`):

```toml
# Llama 3.3 70B — fastest available backend
[models.llama-3-3-70b]
provider = "huggingface"
slug = "meta-llama/Llama-3.3-70B-Instruct"
context_window = 131072
max_output = 4096
supports_tools = true
tool_format = "openai_json"
cost_input_per_m = 0.0

# DeepSeek-R1 — open-weight reasoning model
[models.deepseek-r1]
provider = "huggingface"
slug = "deepseek-ai/DeepSeek-R1"
context_window = 131072
max_output = 8192
supports_tools = false
supports_thinking = true
tool_format = "openai_json"

# Qwen 2.5 72B Instruct
[models.qwen-2-5-72b]
provider = "huggingface"
slug = "Qwen/Qwen2.5-72B-Instruct"
context_window = 131072
max_output = 8192
supports_tools = true
tool_format = "openai_json"
```

To pin a specific backend, prefix the slug with the backend name followed by a colon: `slug = "together:meta-llama/Llama-3.3-70B-Instruct"`. To request the cheapest available backend, append `:cheapest`. To request the fastest, append `:fastest`.

**Step 3 (optional).** For dedicated GPU instances (HuggingFace Inference Endpoints), add a separate provider entry with the endpoint-specific URL:

```toml
[providers.hf-endpoint-my-model]
kind = "openai_compat"
base_url = "https://ENDPOINT-ID.us-east-1.aws.endpoints.huggingface.cloud/v1"
api_key_env = "HF_TOKEN"
timeout_ms = 300000

[models.my-finetune]
provider = "hf-endpoint-my-model"
slug = "tgi"
context_window = 32768
supports_tools = true
tool_format = "openai_json"
```

**Step 4.** Verify:

```bash
# Set credentials
export HF_TOKEN="hf_..."

# Confirm provider is visible
cargo run -p roko-cli -- config providers list
# Expected: huggingface listed with kind=openai_compat

# Confirm model routing resolves correctly
cargo run -p roko-cli -- config models route llama-3-3-70b
# Expected: provider=huggingface, base_url contains router.huggingface.co

# Smoke test a single prompt
cargo run -p roko-cli -- run "Say hello in one sentence." --model llama-3-3-70b
# Expected: a one-sentence response from the model

# Confirm missing token surfaces a useful error
unset HF_TOKEN
cargo run -p roko-cli -- run "hello" --model llama-3-3-70b
# Expected: AgentCreationError::MissingApiKey mentioning HF_TOKEN
```

### Phase 2: `roko-hf` crate (optional, deferred)

Phase 2 adds HuggingFace-specific capabilities that require new code. Create a new workspace member `crates/roko-hf/`.

**2a: Hub API client** (`crates/roko-hf/src/hub.rs`)

Query `GET https://huggingface.co/api/models?search=<query>&library=transformers&sort=downloads` to auto-populate `[models.*]` entries in `roko.toml`. The Hub API returns context window size and model card metadata that map directly to `ModelProfile` fields.

Integration: `roko config models list --discover-hf "qwen 72b"` prints a ready-to-paste TOML block.

**2b: Dataset Viewer client** (`crates/roko-hf/src/datasets.rs`)

Query `GET https://datasets-server.huggingface.co/rows?dataset=princeton-nlp/SWE-bench&split=test&offset=0&length=100` to stream benchmark instances (SWE-bench, HumanEval, MBPP) into the arena evaluation loop without a Python `datasets` dependency.

Integration: `roko arena eval --dataset hf://princeton-nlp/SWE-bench`

**2c: SWE-bench adapter** (`crates/roko-hf/src/swe_bench.rs`)

1. Fetch SWE-bench instances from the Dataset Viewer.
2. Format each instance as a roko task (apply patch, run tests, report pass/fail).
3. Record outcome to the arena leaderboard.

**2d: Inference Endpoints lifecycle** (`crates/roko-hf/src/endpoints.rs`)

```
POST   /endpoint                → create endpoint
GET    /endpoint/{id}/status    → poll until "running"
DELETE /endpoint/{id}           → tear down after batch
```

Use case: spin up a dedicated GPU for a benchmark run, tear it down after.

Phase 2 acceptance criteria (separate work):
1. `cargo add roko-hf` works as a workspace member.
2. `roko config models list --discover-hf "qwen 72b"` returns a pasteable TOML block.
3. `roko arena eval --dataset hf://princeton-nlp/SWE-bench --limit 10` runs 10 instances.
4. `cargo test -p roko-hf` passes with zero network calls (mock HTTP in tests).
5. `cargo clippy -p roko-hf -- -D warnings` is clean.

## Acceptance Criteria

1. A `roko.toml` with `[providers.huggingface]` (kind=openai_compat, base_url=https://router.huggingface.co/v1, api_key_env=HF_TOKEN) causes `roko config providers list` to show the provider.
2. `roko run "<prompt>" --model llama-3-3-70b` (or any HF-backed model alias) succeeds when `HF_TOKEN` is set.
3. No code changes to any `crates/` file are required for Phase 1.
4. When `HF_TOKEN` is missing, the error message names the variable (`AgentCreationError::MissingApiKey("HF_TOKEN")`), surfaced through the existing path in `resolve_api_key` at `crates/roko-agent/src/provider/openai_compat.rs:205-206`.

## Verification Checklist

- [ ] Add `[providers.huggingface]` to `roko.toml`
- [ ] Add at least one `[models.*]` entry pointing to that provider
- [ ] Run `cargo run -p roko-cli -- config providers list` and confirm `huggingface` appears
- [ ] Run `cargo run -p roko-cli -- config models route <model-name>` and confirm correct routing
- [ ] Set `HF_TOKEN` and run a single prompt through the model; confirm a valid response
- [ ] Unset `HF_TOKEN` and confirm a clear error message naming the missing variable
- [ ] Run `cargo test --workspace` to confirm no regressions

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/roko.toml` | Add `[providers.huggingface]` stanza and model entries |
| `demo/demo-resources/provider-routing/roko.toml` (optional) | Add HuggingFace to the demo config as a worked example |
