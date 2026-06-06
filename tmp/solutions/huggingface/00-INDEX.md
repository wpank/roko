# Hugging Face Integration Docs

Research date: 2026-05-01.

This folder expands the older Hugging Face integration notes into a current,
agent-usable design set. The goal is not "add a broad `roko-hf` crate because
HF exists." The goal is to identify the smallest useful vertical slices where
Hugging Face materially improves Roko: model inference, dataset-backed arenas,
model discovery, artifact publishing, and training/job loops.

## Source Inputs

- Older note: `tmp/archive/misc/04-21-26/02-huggingface-integration.md`
- Checklist: `tmp/prds/impl/05-domains-and-arenas/02-domain-extensions-hf-and-market-checklist.md`
- Existing Roko design: `docs/v2/08-GATEWAY.md`
- Existing provider reality: `docs/v2-depth/07-agent-runtime/provider-integrations-and-profiles.md`
- Current code surfaces:
  - `crates/roko-core/src/agent.rs`
  - `crates/roko-core/src/config/provider.rs`
  - `crates/roko-agent/src/model_call_service.rs`
  - `crates/roko-cli/scripts/swebench_run.py`

## Documents

| File | Purpose |
|---|---|
| [01-research-surface-map.md](01-research-surface-map.md) | Current Hugging Face APIs, constraints, and what changed since the old note. |
| [02-roko-hf-product-prd.md](02-roko-hf-product-prd.md) | Product requirements, scope, non-goals, users, and phased outcomes. |
| [03-architecture-and-config.md](03-architecture-and-config.md) | Roko architecture, provider config, auth, capabilities, and error boundaries. |
| [04-benchmark-datasets-and-arena-data.md](04-benchmark-datasets-and-arena-data.md) | Dataset Viewer, Parquet, SWE-bench/Arena ingestion, caching, and scoring flow. |
| [05-model-discovery-and-routing.md](05-model-discovery-and-routing.md) | HF model discovery, provider suffixes, probes, CascadeRouter integration, and telemetry. |
| [06-training-jobs-and-artifact-loop.md](06-training-jobs-and-artifact-loop.md) | Episode export, Hub datasets, Jobs/TRL/AutoTrain, model publication, and evaluation gates. |
| [07-agent-implementation-packets.md](07-agent-implementation-packets.md) | Low-context implementation packets with acceptance criteria and anti-patterns. |

## Recommended Implementation Order

1. **Provider config only:** support Hugging Face Inference Providers through
   the existing `openai_compat` path with `base_url =
   "https://router.huggingface.co/v1"` and `api_key_env = "HF_TOKEN"`.
2. **Dataset vertical slice:** add a Rust `HfDatasetClient` that can load
   rows from `princeton-nlp/SWE-bench_Lite` using Dataset Viewer REST. Keep the
   existing Python SWE-bench scripts as a comparison path.
3. **Model discovery read-only:** list/query HF models and Inference Provider
   chat models into a candidate cache. Do not auto-enable candidates until they
   pass probes.
4. **Probe and route:** convert candidates into `ModelDefinition` entries only
   after auth, capability, stream/tool, and smoke prompts pass through
   `ModelCallService`.
5. **Artifact export:** publish successful episodes/playbooks as private Hub
   datasets only after redaction and license checks.
6. **Training loop:** use HF Jobs with TRL for controlled post-training. Treat
   AutoTrain as a simple UI/managed option, not the core Roko training engine.
7. **Dedicated endpoints later:** only create or scale Inference Endpoints when
   batch demand justifies dedicated compute.

## Core Design Decision

Hugging Face is not a parallel runtime. It is a set of provider, dataset,
artifact, and compute adapters behind existing Roko contracts:

- model calls go through `ModelCallService`;
- provider identity goes through `ProviderDefinition` and `ModelDefinition`;
- benchmark data becomes typed arena input;
- learning artifacts become explicit datasets with provenance;
- external compute is job/endpoint orchestration, not hidden dispatch.

## Things Not To Do

- Do not add raw HF HTTP calls inside ACP, chat, terminal, or serve route
  surfaces. Provider execution belongs behind model-call/provider ownership.
- Do not create a broad `roko-hf` crate full of stubs before a consumer exists.
- Do not auto-add newly discovered models to production routing without probe
  results and operator approval.
- Do not publish episode data to public Hub repositories by default.
- Do not collapse "HF model id", "provider", "routing policy", and "local Roko
  alias" into one string without preserving each part separately.
- Do not assume the OpenAI-compatible HF router supports non-chat tasks. HF
  documents that this endpoint is for chat completions; other tasks need the
  native clients/direct task APIs.

## Current Best First Slice

The most useful and least risky slice is:

1. add config examples and tests proving HF chat works through existing
   `openai_compat`;
2. add a Dataset Viewer client for SWE-bench rows;
3. add one CLI smoke command:

```sh
roko hf dataset rows princeton-nlp/SWE-bench_Lite --split test --offset 0 --length 3
```

This proves auth, rate-limit handling, dataset shape parsing, and local cache
behavior without adding a new dispatch path.

## Official Sources Used

- Inference Providers: https://huggingface.co/docs/inference-providers/en/index
- Chat Completion task: https://huggingface.co/docs/inference-providers/en/tasks/chat-completion
- Dataset Viewer rows: https://huggingface.co/docs/dataset-viewer/en/rows
- Dataset Viewer Parquet: https://huggingface.co/docs/dataset-viewer/parquet
- Hub API: https://huggingface.co/docs/hub/api
- Hub rate limits: https://huggingface.co/docs/hub/rate-limits
- User access tokens: https://huggingface.co/docs/hub/security-tokens
- Webhooks: https://huggingface.co/docs/hub/webhooks
- Jobs: https://huggingface.co/docs/hub/jobs
- Inference Endpoints: https://huggingface.co/docs/inference-endpoints/en/index
- Inference Endpoints autoscaling: https://huggingface.co/docs/inference-endpoints/main/guides/autoscaling
- Hugging Face MCP server: https://huggingface.co/docs/hub/hf-mcp-server
- Spaces as MCP servers: https://huggingface.co/docs/hub/spaces-mcp-servers
- Xet storage: https://huggingface.co/docs/hub/xet/index
- TRL: https://huggingface.co/docs/trl
- AutoTrain FAQ: https://huggingface.co/docs/autotrain/faq
