# HuggingFace Integration

## API Surface (all REST, all `Authorization: Bearer hf_***`)

### Layer 1: Inference Providers (model inference)

**Base URL**: `https://router.huggingface.co/v1`

OpenAI-compatible chat completions across 18+ backend providers (Cerebras, Groq, Together,
SambaNova, Fireworks, etc.). Same model, different providers — HF routes automatically.

Key features:
- **Provider routing policies** — append `:fastest`, `:cheapest`, `:preferred`, or `:provider-name` to model slug
- **Structured generation** — `response_format: { type: "json_schema", strict: true }`
- **Tool calling** — full OpenAI-compatible `tools` + `tool_choice`
- **Streaming** — SSE-based
- **Vision** — image URLs in message content for VLMs
- **Reasoning effort** — `reasoning_effort` parameter for reasoning models

Pricing: no HF markup, pass-through provider costs. Free tier gives $0.10-$2.00/month.

**Roko integration**: Since the API is OpenAI-compatible, it works today with `kind = "open_ai_compat"`.
A dedicated `HuggingFaceApi` provider kind would add routing policy metadata and HF-specific
error classification.

### Layer 2: Hub API (model/dataset/space management)

**Base URL**: `https://huggingface.co/api`

| Endpoint | What roko could do |
|----------|-------------------|
| `GET /api/models?search=...&filter=...` | Dynamic model discovery for CascadeRouter |
| `GET /api/models/{id}` | Auto-populate ModelProfile from model cards |
| `GET /api/datasets/{id}` | Dataset metadata for benchmark loading |
| `POST /api/repos/create` | Publish learned artifacts (playbooks, episodes, fine-tuned models) |
| Webhooks | React to new model releases — auto-benchmark, auto-adopt |

### Layer 3: Dataset Viewer (benchmark data loading)

**Base URL**: `https://datasets-server.huggingface.co`

| Endpoint | Purpose |
|----------|---------|
| `/rows?dataset=...&split=...&offset=...&length=...` | Stream dataset rows (max 100/request) |
| `/parquet?dataset=...` | Get Parquet file URLs for bulk download |
| `/search?dataset=...&query=...` | Full-text search in string columns |
| `/filter?dataset=...&where=...` | Filter rows by query |
| `/size?dataset=...` | Row count and byte sizes |
| `/splits?dataset=...` | List available splits |

This replaces the Python `datasets` library entirely — pure REST calls from Rust.

### Layer 4: Inference Endpoints (dedicated compute)

**Base URL**: `https://api.endpoints.huggingface.cloud`

CRUD for dedicated GPU instances:
- Create endpoint → spin up A10G/A100/H200 with any HF model
- Scale to zero between runs (no billing when idle)
- Auto-scaling (min_replica / max_replica)
- Pay-by-the-minute ($0.50/hour GPU, $0.03/hour CPU)

Enables: elastic compute for batch benchmark runs. Spin up 4 endpoints, run 200 SWE-bench
instances in parallel, tear down. Cost = only what you use.

### Layer 5: AutoTrain (fine-tuning)

**Base URL**: `http://127.0.0.1:8000` (self-hosted) or HF Spaces

```json
POST /api/create_project
{
  "task": "llm:sft",
  "base_model": "meta-llama/Meta-Llama-3-8B-Instruct",
  "hub_dataset": "your-org/successful-patches",
  "hardware": "spaces-a10g-large",
  "params": { "epochs": 1, "peft": true, "quantization": "int4" }
}
```

Supported tasks: SFT, ORPO, DPO, KTO for LLMs. Pushes fine-tuned model back to Hub.

## What Each Layer Enables

### Inference (Layer 1) — immediate

Roko gains access to every open model on HuggingFace through a single provider config.
CascadeRouter can explore Mistral, Llama, DeepSeek, Qwen, Gemma across multiple backend
providers, finding the cheapest/fastest option for each task type.

### Hub + Datasets (Layers 2-3) — the interesting part

**Dynamic model discovery**: CascadeRouter periodically queries HF for new models matching
task requirements. New models get added as bandit arms, explored, adopted or discarded.
The model roster evolves without human intervention.

**Native dataset loading**: SWE-bench, MBPP, HumanEval, CodeContests — any HF dataset loaded
via REST + Parquet from Rust. No Python dependency.

**Publish results**: Episode logs, efficiency metrics, playbook libraries, fine-tuned models
become shared artifacts on the Hub. Multiple roko instances share learning via the Hub.

### Endpoints (Layer 4) — elastic benchmarking

The SWE-bench grinder doesn't need a fixed GPU. Provision on demand, scale to zero between
batches. A nightly cron job could run benchmark batches for pocket change.

### AutoTrain (Layer 5) — the exponential loop

This is where the cybernetic loop closes in a genuinely novel way:

```
SWE-bench run (batch N)
  → episodes logged (which model, which prompt, pass/fail, the actual patch)
  → successful episodes become training data
  → AutoTrain: fine-tune base model on successful patches
  → push fine-tuned model to Hub
  → CascadeRouter adds fine-tuned model as new arm
  → SWE-bench run (batch N+1) includes fine-tuned model
  → LinUCB explores: does the fine-tuned model win?
  → if yes → more traffic → more successes → more training data → ...
```

Each iteration generates training signal as a byproduct of working, fine-tunes on it,
and deploys the result back into production. The system literally builds the models it
uses to build itself.

## Proposed Crate: `roko-hf`

```
roko-hf/
├── src/
│   ├── inference.rs      # Layer 1: Chat completions via Inference Providers
│   ├── hub.rs            # Layer 2: Model/dataset discovery + publishing
│   ├── datasets.rs       # Layer 2b: Parquet dataset loading
│   ├── endpoints.rs      # Layer 3: Dedicated endpoint lifecycle
│   ├── autotrain.rs      # Layer 4: Fine-tuning trigger + model push
│   └── lib.rs
```

All pure Rust, all REST via `reqwest`. ~1500-2000 lines total.

## Network Effects

If roko publishes learned artifacts to HF:
- **Playbook datasets** — successful task-solving patterns, searchable by other instances
- **Fine-tuned models** — specialized code-fixing models anyone can pull
- **Episode datasets** — training signal for the community
- **Prompt experiment winners** — which prompt sections work best

Multiple roko instances running against different benchmarks share learning via the Hub.
Instance A fine-tunes on Django tasks, Instance B on Flask tasks — both push to Hub, both
pull each other's models. CascadeRouter in each discovers the other's models and explores them.

Each additional roko instance generates training signal that makes all instances better.
