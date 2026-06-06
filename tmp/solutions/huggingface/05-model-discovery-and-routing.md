# 05 - Model Discovery And Routing

## Problem

HF makes thousands of models easy to find and many hosted models easy to call.
That is dangerous if discovery feeds production routing directly. The correct
Roko model is:

```text
discover -> candidate -> probe -> evaluate -> promote -> route
```

Discovery is not routing. Probe success is not benchmark success. Benchmark
success is not automatic production promotion.

## Sources Of Model Candidates

| Source | What it gives | Use |
|---|---|---|
| HF router `GET /v1/models` | Chat models available through Inference Providers. | Immediate callable candidate list. |
| Hub API model search | Broader Hub metadata, tags, likes, downloads, licenses, model cards. | Discovery and filtering. |
| Webhooks | Updates when watched repos/orgs change. | Re-probe changed candidates. |
| Operator config | Explicit local models. | Highest trust source. |
| Prior benchmark results | Models that already performed well. | Ranking and promotion evidence. |

## Candidate Model Schema

```rust
pub struct HfModelCandidate {
    pub id: String,
    pub source: HfCandidateSource,
    pub local_alias: Option<String>,
    pub backend_slug: String,
    pub provider_policy: HfProviderPolicy,
    pub explicit_provider: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub capabilities_claimed: ClaimedCapabilities,
    pub capabilities_observed: ObservedCapabilities,
    pub pricing_observed: Option<ModelCost>,
    pub discovered_at: DateTime<Utc>,
    pub last_probed_at: Option<DateTime<Utc>>,
    pub state: CandidateState,
}

pub enum CandidateState {
    Discovered,
    ProbeFailed,
    ProbePassed,
    EvalQueued,
    EvalFailed,
    EvalPassed,
    Promoted,
    Rejected,
}
```

## Provider Policy Handling

HF supports suffix policies:

- `:fastest`
- `:cheapest`
- `:preferred`
- `:<provider-id>`

Roko must preserve the suffix policy as structured metadata. Do not treat
`deepseek-ai/DeepSeek-R1:cheapest` as merely a string slug. It is:

```text
model_id = deepseek-ai/DeepSeek-R1
policy = cheapest
explicit_provider = none
```

For explicit provider:

```text
model_id = openai/gpt-oss-120b
policy = explicit
explicit_provider = sambanova
```

## Probe Matrix

Each candidate must be probed through `ModelCallService`, not direct HTTP.

| Probe | Request | Pass condition |
|---|---|---|
| Auth | minimal call | Missing/invalid token returns typed auth; valid token reaches provider. |
| Non-stream chat | one short message | Response text arrives, model recorded. |
| Streaming | `stream=true` | Emits content deltas and terminal event. |
| Tools | one harmless function schema | Model either emits valid tool call or returns typed unsupported. |
| JSON schema | strict schema response | Response parses and validates. |
| Usage | normal call | Usage parsed or unknown recorded as optional, not zero. |
| Latency | repeated short call | p50/p95 stored with provider/policy. |

Probe failures should record:

- status code;
- provider error code/message;
- retryability;
- observed model field;
- raw body redacted and truncated.

## Promotion Flow

Promotion writes config only after an operator action:

```sh
roko hf models promote candidate-id --alias qwen3-coder-hf-fast
```

Promotion output should be an explicit diff:

```toml
[models.qwen3-coder-hf-fast]
provider = "huggingface"
slug = "Qwen/Qwen3-Coder-480B-A35B-Instruct:fastest"
context_window = 262144
tool_format = "openai"
```

Promotion must not:

- overwrite `default_model`;
- delete existing models;
- silently switch provider policy;
- mark unsupported capabilities as supported.

## CascadeRouter Integration

CascadeRouter should see HF models only after promotion or a controlled eval
experiment. Feed it structured observations:

```rust
pub struct ModelRoutingObservation {
    pub local_alias: String,
    pub backend_slug: String,
    pub provider_id: String,
    pub provider_policy: Option<String>,
    pub actual_response_model: Option<String>,
    pub task_kind: String,
    pub arena: Option<String>,
    pub success: bool,
    pub latency_ms: u64,
    pub usage: UsageObservation,
    pub cost_usd: Option<f64>,
}
```

Do not feed discovery metadata as success data. A model being popular on HF is
not an observation that it solves Roko tasks.

## Telemetry Requirements

Every HF model call should record:

- local Roko model alias;
- provider id (`huggingface`);
- backend model id;
- provider policy suffix;
- actual response `model` field when present;
- route source (`operator_config`, `candidate_probe`, `cascade_router`);
- stream/non-stream;
- capability flags requested;
- usage as `UsageObservation`;
- cost estimate and provider-reported cost if available;
- rate-limit metadata if returned.

## Model Discovery CLI

Commands:

```sh
roko hf models list-router --limit 50
roko hf models search --query coder --task chat-completion --limit 50
roko hf models show Qwen/Qwen3-Coder-480B-A35B-Instruct
roko hf models probe Qwen/Qwen3-Coder-480B-A35B-Instruct --policy fastest
roko hf models candidates
roko hf models promote <candidate-id> --alias <local-alias>
```

First implementation can support only:

```sh
roko hf models list-router
```

and still be valuable if it writes a typed candidate cache.

## Candidate Cache

Location:

```text
.roko/cache/hf/model-candidates.jsonl
```

One JSON object per candidate/probe event:

```json
{
  "kind": "probe_result",
  "candidate_id": "blake3:...",
  "model_id": "Qwen/Qwen3-Coder-480B-A35B-Instruct",
  "policy": "fastest",
  "state": "probe_passed",
  "observed": {
    "streaming": true,
    "tools": "unknown",
    "json_schema": "passed",
    "latency_ms_p50": 820
  }
}
```

Use append-only JSONL for auditability; compact later.

## Acceptance Criteria

- Discovery writes candidates, not active config.
- Probes go through `ModelCallService`.
- Promotion requires explicit command and shows diff.
- Candidate state changes are append-only and auditable.
- CascadeRouter receives only promoted/eval observations, not raw discovery.
- Telemetry preserves model id, provider policy, and actual response model.

## Anti-Patterns

- Do not use HF downloads/likes as routing success.
- Do not set `default_model` from discovery.
- Do not silently fall back from `:cheapest` to `:fastest` or vice versa.
- Do not collapse unknown capabilities to false or true.
- Do not route production traffic to a model that has not passed auth and smoke
  probes.
