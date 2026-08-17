# 01 - Hugging Face Surface Map

Research date: 2026-05-01.

This document summarizes the Hugging Face surfaces that matter for Roko. It
also corrects stale assumptions from the older integration note.

## Executive Summary

Hugging Face is useful to Roko in five distinct ways:

1. **Inference Providers:** one HF token and one router for many hosted models
   and providers.
2. **Hub metadata:** model/dataset/Space search, repo metadata, repo creation,
   webhooks, and model cards.
3. **Dataset Viewer:** REST access to rows, splits, search/filter, sizes, and
   Parquet files.
4. **Compute:** Jobs, scheduled Jobs, Inference Endpoints, Spaces, and
   AutoTrain/TRL training.
5. **Agent tools:** official HF MCP server plus MCP-capable Spaces.

The immediate Roko path should use Inference Providers through existing
`openai_compat`, then add dataset and model discovery clients. Jobs, Endpoints,
and training should come after artifact export and benchmark evaluation are
real.

## Current HF Surfaces

| Surface | Base / access | What HF says | Roko use |
|---|---|---|---|
| Inference Providers | `https://router.huggingface.co/v1` for OpenAI-compatible chat; native clients/direct HTTP for other tasks | Many model/provider partners, OpenAI-compatible chat, provider policies such as `:fastest`, `:cheapest`, `:preferred`, and explicit provider suffixes. | Chat model provider, candidate exploration, code model probes. |
| Hub API | `https://huggingface.co/api` and OpenAPI at `/.well-known/openapi.json` | Open endpoints for model/dataset/Space info and repo actions; subject to HF-wide rate limits. | Model discovery, dataset metadata, artifact repo management. |
| Dataset Viewer | `https://datasets-server.huggingface.co` | `/rows` returns up to 100 rows per request for datasets with Parquet exports; `/parquet` lists converted Parquet files. | Rust benchmark loader without Python `datasets` dependency. |
| Webhooks | Hub settings/API | Repo, PR, discussion, and comment events; can trigger Jobs. | Re-run model probes when watched models/datasets change. |
| Jobs | HF CLI, Python client, or Jobs HTTP API | UV/Docker-like jobs on CPUs, GPUs, A100s, and TPUs; pay for seconds used. | Nightly benchmark shards, training jobs, dataset processing. |
| Inference Endpoints | `https://api.endpoints.huggingface.cloud` / endpoint UI/API | Dedicated managed inference deployments with autoscaling and scale-to-zero. | Long-running dedicated models after traffic proves need. |
| Spaces | Git-backed repos, Gradio/Docker/static | Demo apps; can run on upgraded hardware; some Spaces are MCP tools. | Publish demo tools or external tool adapters; not core runtime. |
| HF MCP Server | `hf.co/mcp` through client-specific settings | Search models, datasets, Spaces, papers, docs; run community tools. | Optional research assistant for model/dataset discovery; not trusted execution. |
| Xet storage | Hub repo backend | Large binary/model/dataset files are stored behind pointer files with chunk deduplication. | Publishing model weights, Parquet datasets, artifacts. |
| TRL / AutoTrain | Python libraries, Jobs, Spaces | TRL supports SFT, DPO, GRPO, reward modeling, KTO/ORPO and more; AutoTrain stores data/models privately when it uploads. | Controlled post-training loop after redaction and eval gates. |

## Inference Providers Details

Facts from current docs:

- HF Inference Providers aggregate many provider partners behind one interface.
- Authentication requires an HF token; for Inference Providers, HF recommends a
  fine-grained token with the "Make calls to Inference Providers" permission.
- The OpenAI-compatible endpoint is:

```text
https://router.huggingface.co/v1/chat/completions
```

- Default provider selection is `:fastest`.
- Model id suffixes can request policies or providers:
  - `model:fastest`
  - `model:cheapest`
  - `model:preferred`
  - `model:sambanova` or another provider id
- `GET /v1/models` returns models available through the OpenAI-compatible chat
  surface.
- HF explicitly documents the OpenAI-compatible router as **chat only**. For
  text-to-image, embeddings, speech, and other tasks, use HF inference clients
  or direct task APIs.

### Roko Implication

Use existing `ProviderKind::OpenAiCompat` for chat. A dedicated HF provider kind
is optional metadata, not a new dispatch path.

Minimal config:

```toml
[providers.huggingface]
kind = "openai_compat"
base_url = "https://router.huggingface.co/v1"
api_key_env = "HF_TOKEN"
timeout_ms = 120000

[models.qwen3-coder-hf-fast]
provider = "huggingface"
slug = "Qwen/Qwen3-Coder-480B-A35B-Instruct:fastest"
context_window = 262144
tool_format = "openai"
```

## Dataset Viewer Details

Useful endpoints:

| Endpoint | Use |
|---|---|
| `/rows?dataset=...&config=...&split=...&offset=...&length=...` | Fetch a row slice. Max `length` is 100. |
| `/parquet?dataset=...` | List converted Parquet files with URLs and sizes. |
| `/splits?dataset=...` | List configs/splits. |
| `/size?dataset=...` | Row counts and byte sizes. |
| `/search?dataset=...&query=...` | Search string columns. |
| `/filter?dataset=...&where=...` | Filter rows using a predicate. |

Important constraints:

- `/rows` currently requires datasets with Parquet exports.
- Public datasets get converted to Parquet by the viewer; private datasets need
  the right plan/org ownership.
- `/rows` is a good smoke path; `/parquet` is the scalable bulk path.

### Roko Implication

The benchmark loader should implement both:

- `rows` for smoke tests and small deterministic slices;
- `parquet` for bulk arena runs using a Rust Parquet/Arrow/Polars path.

## Hub API, Webhooks, And Rate Limits

Facts from current docs:

- Hub APIs, resolver URLs, and web pages have distinct rate-limit buckets.
- All quotas are over 5-minute fixed windows.
- HF exposes `RateLimit` and `RateLimit-Policy` headers.
- HF recommends always passing `HF_TOKEN`; anonymous calls get lower limits and
  are more likely to rate-limit.
- Fine-grained tokens are the production recommendation.
- Webhooks can watch repo updates, PRs, discussions, and comments; webhooks can
  trigger Jobs.

### Roko Implication

Every HF client should:

- classify 429 distinctly;
- parse `RateLimit` headers when present;
- use backoff with reset timing;
- cache discovery results;
- never call Hub APIs from hot model dispatch paths;
- use resolver URLs for file downloads where possible.

## Jobs, Endpoints, Spaces, And MCP

### Jobs

HF Jobs are the best fit for batch work:

- run commands in Docker-like or UV-style jobs;
- choose CPU/GPU/TPU flavors;
- pay for seconds used;
- schedule jobs;
- trigger from webhooks.

Use for:

- nightly benchmark shards;
- training data processing;
- TRL fine-tuning;
- model probe matrix runs.

### Inference Endpoints

Inference Endpoints are dedicated deployments. They reduce infra work and scale
with traffic, but they are a heavier commitment than Inference Providers or
Jobs.

Important scale-to-zero behavior:

- endpoints can go idle after inactivity;
- cold start is expected;
- the proxy can return `503` while initializing;
- `X-Scale-Up-Timeout` can ask the proxy to wait for a replica.

Use only after Roko has traffic or batch volume that justifies dedicated
deployment.

### Spaces And MCP

HF has an official MCP server and can expose compatible Spaces as MCP tools.
This is useful for model/dataset research and external tools, but it should not
be a trusted production execution path until Roko has tool provenance, rate
limits, and permission boundaries for remote MCP calls.

## What Changed Since The Older Note

| Older assumption | Current correction |
|---|---|
| "HF router is a broad OpenAI-compatible API." | OpenAI-compatible endpoint is documented as chat-completions only. Other tasks need HF task clients/direct APIs. |
| "Create `roko-hf` with all layers up front." | Build vertical slices first; only extract a crate once there are multiple real consumers. |
| "AutoTrain is the primary fine-tuning loop." | Prefer Jobs + TRL for controlled training. AutoTrain remains useful for simpler managed cases. |
| "Endpoints are the elastic benchmark answer." | Jobs are usually better for batch and training; Endpoints are for dedicated serving. |
| "Publish learned artifacts to Hub" | Correct, but private-by-default with redaction, license checks, and explicit operator approval. |
| "Dynamic model discovery can directly feed routing" | Discovery must create candidates; probes/evals/approval promote candidates. |

## Open Questions Before Implementation

- Which HF namespace should Roko use for private artifact datasets?
- Should HF model discovery run in CLI only, serve only, or both?
- How should discovered model/provider cost be represented if HF routes to
  multiple providers behind one model suffix?
- Do we need a first-class `ProviderKind::HuggingFaceApi`, or are
  `openai_compat` plus metadata enough until non-chat tasks arrive?
- Should benchmark dataset cache live under `.roko/cache/hf`, OS cache dir, or
  user-configured workspace cache?
- What redaction policy is required before episode datasets leave the local
  machine?

## Source Links

- Inference Providers: https://huggingface.co/docs/inference-providers/en/index
- Dataset rows: https://huggingface.co/docs/dataset-viewer/en/rows
- Dataset Parquet: https://huggingface.co/docs/dataset-viewer/parquet
- Hub API: https://huggingface.co/docs/hub/api
- Hub rate limits: https://huggingface.co/docs/hub/rate-limits
- User access tokens: https://huggingface.co/docs/hub/security-tokens
- Webhooks: https://huggingface.co/docs/hub/webhooks
- Jobs: https://huggingface.co/docs/hub/jobs
- Inference Endpoints autoscaling: https://huggingface.co/docs/inference-endpoints/main/guides/autoscaling
- HF MCP Server: https://huggingface.co/docs/hub/hf-mcp-server
- Spaces as MCP servers: https://huggingface.co/docs/hub/spaces-mcp-servers
- Xet storage: https://huggingface.co/docs/hub/xet/index
- TRL: https://huggingface.co/docs/trl
- AutoTrain FAQ: https://huggingface.co/docs/autotrain/faq
