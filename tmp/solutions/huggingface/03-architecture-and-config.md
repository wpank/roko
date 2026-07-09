# 03 - Architecture And Config

## Architecture Rule

Hugging Face must plug into existing Roko contracts:

```text
roko.toml
  -> ProviderDefinition / ModelDefinition
  -> DispatchResolver
  -> ModelCallService
  -> provider adapter
  -> telemetry / usage / learning
```

It must not create parallel provider execution in ACP, chat, serve routes, demo
automation, or benchmark scripts.

## Minimal Provider Config

HF Inference Providers chat can use the existing OpenAI-compatible path:

```toml
[providers.huggingface]
kind = "openai_compat"
base_url = "https://router.huggingface.co/v1"
api_key_env = "HF_TOKEN"
timeout_ms = 120000
connect_timeout_ms = 10000
ttft_timeout_ms = 30000

[models.qwen3-coder-hf-fast]
provider = "huggingface"
slug = "Qwen/Qwen3-Coder-480B-A35B-Instruct:fastest"
context_window = 262144
tool_format = "openai"

[models.deepseek-r1-hf-cheap]
provider = "huggingface"
slug = "deepseek-ai/DeepSeek-R1:cheapest"
context_window = 128000
tool_format = "openai"
```

Required behavior:

- `HF_TOKEN` stays in the environment or secret store, never in generated
  prompts or agent manifests.
- Missing token fails before network dispatch.
- `base_url` is the router root, not a hardcoded endpoint in random call sites.
- Model suffix policy is preserved in telemetry.

## Should Roko Add `ProviderKind::HuggingFaceApi`?

Not in the first slice.

Use `ProviderKind::OpenAiCompat` until at least one of these is true:

- Roko needs non-chat HF task APIs from Rust;
- Roko needs HF-specific response metadata that cannot fit provider metadata;
- Roko needs first-class provider policy validation beyond the model suffix;
- Roko has multiple HF consumers and duplicated code is appearing.

If a provider kind is later added, it should still delegate chat execution to
the OpenAI-compatible adapter or shared model-call implementation. It should
not fork chat completion logic.

## Proposed Config Extensions

These are future extensions, not required for Phase 1:

```toml
[providers.huggingface.hf]
provider_policy = "fastest"          # fastest | cheapest | preferred | explicit
explicit_provider = "sambanova"      # only when provider_policy = "explicit"
bill_to = "my-org"                   # optional HF billing namespace if supported
discovery_enabled = false
allow_non_chat_tasks = false

[models.qwen3-coder-hf-fast.capabilities]
streaming = true
tools = true
json_schema = "probe_required"       # unknown | supported | unsupported | probe_required
vision = "unknown"
reasoning_effort = "unknown"
```

Why this is separate from `slug`:

- local alias is stable for Roko;
- backend slug is the HF model id;
- provider policy chooses infrastructure;
- actual serving provider may vary;
- actual response model may differ with routing/fallbacks.

## Type Model

Add only after a consumer exists:

```rust
pub struct HfModelSelector {
    pub model_id: String,
    pub policy: HfProviderPolicy,
    pub explicit_provider: Option<String>,
}

pub enum HfProviderPolicy {
    Fastest,
    Cheapest,
    Preferred,
    Explicit,
}

impl HfModelSelector {
    pub fn to_router_model(&self) -> String {
        match (&self.policy, &self.explicit_provider) {
            (HfProviderPolicy::Fastest, _) => format!("{}:fastest", self.model_id),
            (HfProviderPolicy::Cheapest, _) => format!("{}:cheapest", self.model_id),
            (HfProviderPolicy::Preferred, _) => format!("{}:preferred", self.model_id),
            (HfProviderPolicy::Explicit, Some(p)) => format!("{}:{}", self.model_id, p),
            (HfProviderPolicy::Explicit, None) => self.model_id.clone(),
        }
    }
}
```

Do not parse and reparse suffixes with ad hoc string logic across call sites.
If suffix parsing exists, it belongs in one module with tests.

## ModelCallService Integration

Required path:

```text
ModelCallRequest
  -> DispatchResolver validates provider/model/auth/capabilities
  -> ModelCallService::call/stream
  -> existing OpenAI-compatible provider adapter
  -> ModelStreamEvent / ModelCallResponse
```

No new code should call:

```text
https://router.huggingface.co/v1/chat/completions
```

outside provider/model-call ownership.

## Capability Validation

HF has multiple providers behind a model id. Support can vary by provider and
model. Roko should probe and record capabilities instead of assuming:

| Capability | Validation |
|---|---|
| Chat | Basic non-stream call succeeds. |
| Streaming | `stream=true` returns usable deltas and terminal event. |
| Tools | Tool schema request returns either tool call or clear supported response. |
| JSON schema | Strict schema probe returns valid JSON matching schema. |
| Vision | Small image-url prompt succeeds if model claims VLM support. |
| Reasoning effort | Request with `reasoning_effort` succeeds or returns typed unsupported error. |
| Usage | Response includes parseable usage or records `UsageObservation` unknowns. |

Unknown is not false. Unknown means do not rely on the feature until probed.

## Error Taxonomy

Map HF errors to Roko typed errors:

| Condition | Roko error |
|---|---|
| Missing `HF_TOKEN` | `ProviderAuthMissing` |
| 401/403 | `ProviderAuthRejected` or `ProviderAccessDenied` |
| 404 model/repo | `ProviderModelNotFound` or `HubResourceNotFound` |
| 429 | `ProviderRateLimited { reset_after }` |
| 503 endpoint scaling | `ProviderColdStart { retry_after }` |
| Unsupported provider capability | `ProviderCapabilityUnsupported` |
| Schema drift | `ExternalSchemaChanged` |
| Tool/JSON malformed | `ProviderProtocolViolation` |

Do not hide these behind generic "api error" strings if the caller needs to
decide retry, downgrade, or fail.

## Rate Limit Handling

HF documents `RateLimit` and `RateLimit-Policy` headers. Every HF client should:

- parse reset seconds when available;
- return typed retry metadata;
- use exponential backoff with jitter when reset is missing;
- cache discovery results aggressively;
- keep discovery out of hot dispatch.

## Auth And Secret Boundaries

Token policy:

| Use | Token role |
|---|---|
| Public dataset/model reads | read or fine-grained read |
| Inference Providers | fine-grained token with Inference Providers call permission |
| Private dataset export | write or fine-grained write to target repo |
| Jobs/training | write plus job permissions required by HF account/org |

Rules:

- Prefer one token per app/workspace.
- Production should use fine-grained tokens.
- Config stores only env var names or secret refs.
- `config doctor` reports token presence and rough scope requirements, not token
  values.

## Crate Boundary

Do not create a dedicated crate immediately. Start with modules owned by current
consumers:

| Consumer | First module |
|---|---|
| CLI dataset command | `crates/roko-cli/src/hf/datasets.rs` or equivalent |
| Provider config doctor | existing config command module |
| Model discovery | `crates/roko-agent` or `crates/roko-cli`, depending on owner |
| Artifact export | learning/runtime crate that owns episode data |

Extract `roko-hf` only when at least two crates need the same typed client.

Target future crate:

```text
crates/roko-hf/
  src/auth.rs
  src/client.rs
  src/datasets.rs
  src/hub.rs
  src/inference_catalog.rs
  src/jobs.rs
  src/repo.rs
  src/rate_limit.rs
```

## Static Checks

Add or extend fitness checks to catch:

```text
router.huggingface.co/v1/chat/completions
api-inference.huggingface.co
datasets-server.huggingface.co
huggingface.co/api
HF_TOKEN
HUGGINGFACE_HUB_TOKEN
```

Allowed locations:

- HF client module;
- config docs/tests;
- test fixtures;
- explicit CLI command module.

Forbidden locations:

- ACP bridge events;
- chat session dispatch;
- serve routes that execute models;
- terminal/demo automation;
- generated agent prompts/manifests.

## Acceptance Criteria

- HF chat config uses `openai_compat` and `ModelCallService`.
- No new raw provider HTTP appears in ACP/chat/serve model execution surfaces.
- Missing/invalid auth is typed and tested.
- Model/provider/policy/actual response model are recorded separately.
- Rate-limit handling returns structured retry info.
- Any new HF client has mockable HTTP and unit tests for status/error parsing.
