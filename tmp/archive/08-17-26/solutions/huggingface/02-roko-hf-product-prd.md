# 02 - Roko Hugging Face Product PRD

## Product Thesis

Roko should treat Hugging Face as the open-model operating layer:

- use HF Inference Providers for quick access to open and hosted models;
- use Dataset Viewer and Parquet as a Rust-native benchmark data source;
- use Hub repos as private/public artifact stores for episodes, playbooks, and
  training datasets;
- use Jobs/TRL for batch evaluation and controlled fine-tuning;
- use webhooks to keep discovered models and datasets fresh.

The integration is valuable only if it plugs into Roko's existing gateway,
arena, learning, and artifact contracts. A disconnected "HF integration" would
repeat the same anti-pattern seen elsewhere in the audit: broad surface area,
local HTTP copies, and no closed loop.

## Users

| User | Need |
|---|---|
| Roko operator | Add open models quickly, compare cost/latency/quality, and keep tokens private. |
| Benchmark runner | Load HF datasets without Python dependency and run repeatable arena shards. |
| Learning/system owner | Export successful episodes safely and turn them into training/eval datasets. |
| Researcher | Discover new models/datasets/Spaces and evaluate them before production use. |
| Future marketplace operator | Publish verified playbooks/models/datasets with provenance and license metadata. |

## Goals

1. Add Hugging Face chat inference without adding a new dispatch path.
2. Load benchmark datasets through typed Rust clients.
3. Discover candidate models and providers without silently changing production
   routing.
4. Export artifacts to private Hub datasets with redaction and provenance.
5. Run training/evaluation jobs from explicit plans.
6. Make every HF network dependency observable, cached, rate-limited, and
   fail-closed.

## Non-Goals

- Do not implement every Hugging Face task API in the first pass.
- Do not publish public datasets or models by default.
- Do not create dedicated Inference Endpoints before there is measured demand.
- Do not make HF MCP tools trusted production tools without a separate security
  design.
- Do not replace existing local/Ollama, Anthropic, OpenAI, Cerebras, Gemini, or
  Perplexity providers.
- Do not put API tokens in agent manifests, prompt context, or generated config.

## Product Capabilities

### Capability 1: HF Chat Provider

As an operator, I can configure:

```toml
[providers.huggingface]
kind = "openai_compat"
base_url = "https://router.huggingface.co/v1"
api_key_env = "HF_TOKEN"
timeout_ms = 120000

[models.deepseek-r1-hf-fast]
provider = "huggingface"
slug = "deepseek-ai/DeepSeek-R1:fastest"
context_window = 128000
tool_format = "openai"
```

Acceptance:

- `ModelCallService::call` and `ModelCallService::stream` can use the provider.
- Missing `HF_TOKEN` fails before request dispatch with a typed auth error.
- The actual configured model id, policy suffix, and response model are recorded
  in telemetry.
- No ACP/chat/serve surface adds local HF request code.

### Capability 2: Dataset Rows And Parquet

As a benchmark runner, I can load a deterministic slice:

```sh
roko hf dataset rows princeton-nlp/SWE-bench_Lite --split test --offset 0 --length 10
```

Acceptance:

- Parses `features` and `rows` into typed Rust structs.
- Enforces HF `/rows` max length of 100.
- Caches results by dataset/config/split/revision/offset/length.
- Handles 404, gated/private, 429, and schema drift with typed errors.

### Capability 3: Model Discovery

As a researcher, I can run:

```sh
roko hf models search --task chat-completion --query coder --limit 20
roko hf providers models --limit 20
```

Acceptance:

- Discovery writes to a candidate cache, not production config.
- Candidates include source, model id, provider/policy, capabilities, license,
  last checked time, and probe status.
- Production routing ignores candidates until they are promoted.

### Capability 4: Probe Matrix

As an operator, I can probe discovered candidates:

```sh
roko hf models probe qwen/Qwen3-Coder --policies fastest,cheapest --tools --stream
```

Acceptance:

- Probes go through `ModelCallService`.
- Stream, tools, JSON schema, vision, and latency are measured separately.
- Probe failure does not mutate `roko.toml`.
- Results can be used by `CascadeRouter` only after explicit promotion.

### Capability 5: Artifact Dataset Export

As a learning owner, I can export verified episodes:

```sh
roko hf export episodes --since 7d --only-passed --repo org/roko-code-episodes-private
```

Acceptance:

- Default visibility is private.
- Export requires redaction pass, license check, and operator acknowledgement.
- Dataset rows include provenance and enough evaluation fields to reproduce the
  result.
- No secrets, environment variables, private file contents, or unapproved code
  leave the machine.

### Capability 6: Training Jobs

As a learning owner, I can create a controlled training job:

```sh
roko hf train sft --dataset org/roko-code-episodes-private --base Qwen/Qwen2.5-Coder-7B-Instruct
```

Acceptance:

- Job plan is saved locally before submission.
- Training repo and dataset repo visibility are explicit.
- Evaluation gates run before a model is eligible for Roko routing.
- The old model remains default unless the candidate beats threshold metrics.

## Phased Delivery

### Phase 0: Docs And Config Examples

Deliver:

- these docs;
- config snippets;
- provider/usage/rate-limit constraints;
- low-tier packets.

Exit criteria:

- future agents can implement without prior chat context.

### Phase 1: HF Chat Through Existing Provider Path

Deliver:

- `openai_compat` config test fixture for HF;
- `HF_TOKEN` auth validation;
- stream and non-stream smoke tests behind mocks;
- docs in `config doctor`.

Exit criteria:

- one HF model call can be made through `ModelCallService` with no new dispatch
  code in chat/ACP/serve.

### Phase 2: Dataset Viewer Client

Deliver:

- `HfDatasetClient`;
- `/rows`, `/splits`, `/size`, `/parquet` support;
- cache and typed errors;
- SWE-bench Lite slice adapter.

Exit criteria:

- a Rust command can load a small SWE-bench slice without Python `datasets`.

### Phase 3: Candidate Model Discovery

Deliver:

- Hub model search client;
- HF router model list client;
- candidate cache;
- probe command;
- promotion command that edits config only after approval.

Exit criteria:

- discovery and probing can populate a local candidate registry without
  affecting production routing.

### Phase 4: Artifact Export

Deliver:

- episode/playbook dataset schemas;
- redaction checks;
- private repo creation/upload plan;
- local dry-run manifest.

Exit criteria:

- export can produce a local dataset bundle and a dry-run Hub upload plan.

### Phase 5: Jobs/Training Loop

Deliver:

- HF Jobs plan builder;
- TRL SFT job template;
- evaluation gate;
- candidate model promotion flow.

Exit criteria:

- a fine-tuned model can be trained, evaluated, and left as a candidate without
  automatically replacing production defaults.

## Product Metrics

| Metric | Target |
|---|---|
| HF chat setup time | One config block plus `HF_TOKEN`. |
| Dataset slice load | 10 SWE-bench rows in under 5 seconds after cache warmup. |
| Probe completeness | Candidate has stream, tools, JSON, latency, usage, and error classification. |
| Routing safety | 0 auto-promoted models without passing probes and operator approval. |
| Artifact safety | 0 exported rows with secrets or private unapproved file content. |
| Training usefulness | Fine-tuned candidate must beat base model on held-out eval before promotion. |

## Risks

| Risk | Mitigation |
|---|---|
| HF provider suffix changes actual provider silently. | Store local alias, backend slug, policy suffix, actual response model, and observed provider separately. |
| Rate limits break discovery/probes. | Cache, parse rate-limit headers, back off, never run discovery in hot dispatch. |
| Dataset schema drift breaks benchmarks. | Store schema fingerprints and typed versioned adapters. |
| Episode export leaks code/secrets. | Private-by-default, redaction, license checks, explicit acknowledgements. |
| Fine-tuning overfits benchmark tasks. | Keep held-out evals, never train on eval split, record data lineage. |
| Broad crate with stubs rots. | Build vertical consumers first; extract shared crate only when repeated clients exist. |

## Acceptance Criteria For The Whole Integration

- Model calls enter through `ModelCallService`.
- Dataset access has typed errors, rate-limit handling, and cache keys.
- Model discovery cannot mutate active model config without explicit promotion.
- Artifact export is dry-run capable and private-by-default.
- Training jobs are reproducible from a saved local plan.
- All phases have static recurrence checks preventing raw provider HTTP in UI or
  route surfaces.
