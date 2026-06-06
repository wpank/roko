# 06 - Training, Jobs, And Artifact Loop

## Thesis

The long-term value of Hugging Face is not just "more models." It is a closed
learning loop:

```text
arena/task execution
  -> verified episode
  -> redacted training/eval dataset
  -> HF private dataset repo
  -> Jobs + TRL or AutoTrain
  -> model repo
  -> held-out eval
  -> candidate model
  -> controlled routing experiment
```

This loop is only useful if it is safe, reproducible, and evaluated. Otherwise
it becomes write-only learning data plus untrusted fine-tuned models.

## Artifact Types

| Artifact | Default visibility | Purpose |
|---|---|---|
| Episode dataset | Private | Training/eval examples from successful or failed runs. |
| Playbook dataset | Private, later public subset | Reusable task-solving patterns. |
| Patch dataset | Private | Code issue, context, patch, tests, result. |
| Eval result dataset | Private or public summary | Model comparison evidence. |
| Fine-tuned model | Private candidate | Model to probe/evaluate before promotion. |
| Model card | Public or private | Provenance, training data summary, eval metrics, limitations. |

## Episode Export Schema

Use JSONL or Parquet. Minimum row:

```json
{
  "schema_version": 1,
  "episode_id": "uuid",
  "created_at": "2026-05-01T00:00:00Z",
  "task": {
    "kind": "swe_bench",
    "instance_id": "django__django-12345",
    "repo": "django/django",
    "base_commit": "..."
  },
  "input": {
    "problem_statement": "...",
    "context_manifest": [
      {
        "path": "django/foo.py",
        "hash": "blake3:...",
        "included": true,
        "license_status": "allowed"
      }
    ]
  },
  "output": {
    "patch": "...",
    "patch_hash": "blake3:...",
    "tests_passed": true,
    "score": 1.0
  },
  "model": {
    "local_alias": "qwen3-coder-hf-fast",
    "backend_slug": "Qwen/Qwen3-Coder-480B-A35B-Instruct",
    "provider": "huggingface",
    "policy": "fastest",
    "actual_response_model": "..."
  },
  "usage": {
    "input_tokens": 1234,
    "output_tokens": 567,
    "cost_usd": null
  },
  "safety": {
    "redaction_passed": true,
    "license_passed": true,
    "secret_scan_passed": true
  }
}
```

## Redaction And License Gates

Before any upload:

- secret scan prompt, patch, logs, env names/values, file content;
- remove private absolute paths;
- remove API keys, tokens, cookies, SSH keys, wallet keys;
- verify repository license allows derived patch/context publication;
- separate public benchmark data from private user workspace data;
- require operator acknowledgement for any raw source snippets;
- default to private Hub repo.

If any gate is unknown, export should stop or produce local-only output.

## Jobs vs AutoTrain vs Inference Endpoints

| Tool | Best use | Roko role |
|---|---|---|
| HF Jobs | Batch scripts, data processing, eval shards, TRL training. | Primary controlled compute path. |
| TRL | SFT, DPO, GRPO, reward modeling, KTO/ORPO, distillation. | Primary post-training framework. |
| AutoTrain | Simple managed training, quick UI/Space-backed fine-tunes. | Optional convenience path. |
| Inference Endpoints | Dedicated serving with autoscale/scale-to-zero. | Later serving path after model proves useful. |
| Spaces | Demo apps, MCP tools, training UIs. | Optional presentation/tool surface. |

## Job Plan

Every remote job should be saved before submission:

```json
{
  "schema_version": 1,
  "kind": "hf_job_plan",
  "name": "roko-sft-qwen-coder-2026-05-01",
  "namespace": "my-org",
  "image": "pytorch/pytorch:2.6.0-cuda12.4-cudnn9-devel",
  "flavor": "a10g-small",
  "command": [
    "python",
    "-m",
    "trl.scripts.sft",
    "--model_name_or_path",
    "Qwen/Qwen2.5-Coder-7B-Instruct",
    "--dataset_name",
    "my-org/roko-code-episodes-private"
  ],
  "env": {
    "WANDB_DISABLED": "true"
  },
  "secrets": [
    "HF_TOKEN"
  ],
  "inputs": {
    "dataset_repo": "my-org/roko-code-episodes-private",
    "dataset_revision": "..."
  },
  "outputs": {
    "model_repo": "my-org/roko-qwen-coder-sft-2026-05-01"
  }
}
```

## Evaluation Gate

A trained model is only a candidate until it passes:

- base smoke prompt;
- formatting/tool-call smoke if needed;
- held-out benchmark subset;
- regression set against tasks the base model solved;
- safety/tool output checks;
- cost/latency threshold;
- license/model-card review.

Promotion requires an explicit config change. Do not auto-promote a model after
training succeeds.

## Webhook Automation

Use webhooks only after manual flow works.

Useful triggers:

- dataset repo changed -> schedule eval/training job;
- model repo changed -> probe model;
- watched upstream model changed -> re-run probes;
- discussion/PR on model repo -> notify, not route.

Webhook handler requirements:

- verify webhook secret/signature when available;
- idempotency key per event;
- replay-safe event processing;
- job creation rate limit;
- local audit log.

## Inference Endpoint Use

Create dedicated endpoints only when:

- model candidate passed evals;
- sustained usage or batch requirements justify it;
- cost estimate beats Inference Providers or external provider direct use;
- cold start behavior is acceptable or `X-Scale-Up-Timeout` is used;
- route has fallback behavior for `503` during scale-up.

Endpoint lifecycle:

```text
candidate promoted to endpoint experiment
  -> create endpoint
  -> wait healthy
  -> run benchmark/probe
  -> route small traffic slice
  -> scale down/delete if no benefit
```

## Acceptance Criteria

- Export is dry-run capable and private-by-default.
- No remote job can be submitted without a saved job plan.
- Every uploaded dataset row has redaction, license, and provenance fields.
- Fine-tuned model output cannot affect production routing until eval and
  promotion.
- Webhook handling is idempotent and replay-safe.
- Endpoint cold starts and `503` are classified as retryable provider state, not
  generic failures.

## Anti-Patterns

- Do not train on benchmark test split and then report that benchmark as an
  unbiased eval.
- Do not upload user workspace code by default.
- Do not use AutoTrain as an opaque black box for core Roko learning. Store
  inputs, config, logs, and evals.
- Do not publish public datasets until private redaction and licensing have
  been proven.
- Do not let training success mutate `default_model`.
