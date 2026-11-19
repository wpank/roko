+++
id = "gap-67aecd"
kind = "gap"
title = "roko.toml prices five routed models at rates no provider page gives (2026-10-27)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-ceffb3 price research, 2026-10-27 (config/prices/2026-10-27.toml)"
discovered_from = "gap-ceffb3"
anchors = ["roko.toml", "crates/roko-core/src/config/model_registry.rs::BUILTIN_PRICING"]
links = { depends_on = [], blocks = [], related = ["gap-ceffb3", "gap-a0043b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import glob,tomllib as t;s={r['slug']:r for r in t.load(open(sorted(glob.glob('config/prices/2*-*-*.toml'))[-1],'rb'))['model']};m=t.load(open('roko.toml','rb'))['models'];b=[k for k,v in m.items() if v.get('cost_input_per_m') is not None and (v['slug'] not in s or abs(v['cost_input_per_m']-s[v['slug']]['input'])>1e-9 or abs(v.get('cost_output_per_m',0)-s[v['slug']]['output'])>1e-9)];print(b);raise SystemExit(bool(b))\""
+++

## Problem

On 2026-10-27, while writing `config/prices/2026-10-27.toml` (gap-ceffb3), no provider page gave a per-token rate
for four models `roko.toml` routes to, and a fifth disagreed with its page. `roko.toml` still prices all five from
its own `[models.*]` rates, so dispatch charges their calls at those rates (`CallPricing::Profile`) rather than
recording an unknown cost:

| `[models.*]` key | slug | roko.toml rate (in / out per 1M) | the provider's page on 2026-10-27 |
|---|---|---|---|
| `cerebras-qwen` | qwen-3.8-27b | $0.10 / $0.10 | $0.99 / $1.49 (inference-docs.cerebras.ai/models/qwen-3.8-27b; the snapshot's row) |
| `cerebras-gemma` | gemma-4-31b | $0.10 / $0.10 | served only on Cerebras Dedicated Inference, with no public rate |
| `glm51` | glm-5v-turbo | $1.40 / $4.40 | not on docs.z.ai/guides/overview/pricing, nor in Z.ai's docs index |
| `groq-llama70b` | llama-3.3-70b-versatile | $0.59 / $0.79 | Groq lists it as Enterprise, "Contact Sales" (console.groq.com/docs/models) |
| `sonar` | sonar | $1.00 / $1.00, $0.005 a request | Perplexity's pricing page no longer lists Sonar Chat Completions, whose support ended on 2026-09-27 |

Also, `kimi-k2-5` (kimi-k2.6: the mid rung's alternate and the first `[routing] fallback_models` entry) closes to
newly registered Moonshot users on 2026-10-31, and Moonshot plans to retire the K2 series by the end of December
2026 (platform.kimi.ai/docs/pricing/chat).

## Why it matters

Goal: truth. A call charged at a rate no provider publishes is neither a measured cost nor an unknown one, and a
budget cap counts it as priced. `glm51` is the second fallback model, and Perplexity is moving Sonar requests to its
Agent API. Since prices-2026-10-27 has a qwen-3.8-27b row, dispatch already prices that model's calls at Cerebras's
rate; roko.toml's $0.10 still feeds anything that reads profile prices, such as the price order of
`RokoConfig::tool_loop_models`.

## Where

- `roko.toml`: `[models.cerebras-qwen]`, `[models.cerebras-gemma]`, `[models.glm51]`, `[models.groq-llama70b]`,
  `[models.sonar]`, `[models.kimi-k2-5]`; `[routing] fallback_models`; `[routing.ladder]` rungs.
- `crates/roko-core/src/config/model_registry.rs::BUILTIN_PRICING`: its sonar rows still say "checked 2026-10-02".

## Current state

`config/prices/2026-10-27.toml` (gap-ceffb3) prices qwen-3.8-27b and every other routed model a provider page
prices; its header names the four it leaves out and why.

## Plan

Will's call, per model:
1. `cerebras-qwen`: set its rates to the snapshot's ($0.99 in and cached, $1.49 out), as gap-a0043b did for the other
   cheap models.
2. gemma-4-31b, glm-5v-turbo, llama-3.3-70b-versatile and sonar: (a) drop their `cost_*` fields, so their calls are
   unknown and a capped run refuses them; (b) drop or replace the models (`glm51` could become glm-5.1, which the
   snapshot prices); or (c) keep the rates with a comment naming where each comes from.
3. Choose kimi-k2.6's successor before the K2 retirement. A K3 model needs a row in a new dated snapshot, and K3
   bills 5-minute and 1-hour cache writes.

## Done when

Every `roko.toml` `[models.*]` entry that sets `cost_input_per_m` has a row in the newest snapshot at the same input
and output rates (the `[[verify]]`).

## Notes

Do not edit `config/prices/*.toml`: a snapshot is immutable, and a new rate needs a new dated file.
