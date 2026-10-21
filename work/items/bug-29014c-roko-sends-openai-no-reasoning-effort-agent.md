+++
id = "bug-29014c"
kind = "bug"
title = "roko sends OpenAI no reasoning_effort: [agent] default_effort = \"medium\" never reaches openai_compat; build_extra_body_params (provider/openai_compat.rs:241) adds only sampling, GLM, Kimi and routing params, so the GPT-5.4 family runs at the API default (0 reasoning tokens in all 52 gpt-5.4-mini efficiency events in Undertow, phases 0-3, and in the 2026-10-21 gpt-5.4 probe)"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/provider"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#N-gpt54-proof"
discovered_from = "agent:hermes-pilot (round v7, the fenced gpt-5.4 call before phase 4; tmp/hermes-roko-v7/pilot/gpt54-proof)"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs::build_extra_body_params", "crates/roko-agent/src/openai_compat_backend.rs::build_body", "crates/roko-agent/src/tool_loop/backends/mod.rs::create_observed_openai_compat_backend", "crates/roko-core/src/config/provider.rs::ModelProfile", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::request_sampling"]
links = { depends_on = [], blocks = [], related = ["bug-cbbaa8", "gap-13bbbd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_openai_reasoning_model_request_carries_reasoning_effort' crates/roko-agent/src && cargo test -p roko-agent --lib -- an_openai_reasoning_model_request_carries_reasoning_effort"
+++

## Problem

roko never sends OpenAI a `reasoning_effort`, so OpenAI's reasoning models run at the API's default effort.
`[agent] default_effort = "medium"` doesn't reach the `openai_compat` provider. For the GPT-5.4 family that default
looks like no reasoning at all.

Round v7, Undertow (hermes-pilot, 2026-10-21). The three requests below are from a fenced gpt-5.4 call through roko
(`roko run`, roko 5a9884c95 + bug-5e597a, Undertow's phase 4 config with its base_urls on a local tee):

| Request | Top-level keys | Reasoning tokens (OpenAI's usage) |
|---|---|---|
| rung probe, call 1 (not streamed) | `model`, `max_completion_tokens`, `messages`, `tools` | 0 (180 in, 19 out) |
| rung probe, call 2 (not streamed) | the same | 0 (217 in, 7 out) |
| the task's streamed turn | the same, plus `stream` and `stream_options` | 0 (2,554 in, 4 out) |

- No `reasoning_effort` in any of them.
- Undertow's `.roko/learn/efficiency.jsonl` for phases 0–3:
  - **52 gpt-5.4-mini events, 209,569 output tokens, 0 reasoning tokens in every one.**
  - For comparison, 39 glm-4.7 events report 657,896 reasoning tokens.
  - roko does read OpenAI's `completion_tokens_details.reasoning_tokens` (`translate/openai.rs:327`), so the zeros are
    what OpenAI reported.

## Why it matters

- gpt-5.4-mini was the strong rung through phases 2–3. It handed back with its own tests red at 13–25 of 120 turns
  (bug-cbbaa8), and went 0 of 8 on phase 2's DSP checks. At effort none it barely reasons, which may be why.
- Phase 4's ladder runs on OpenAI alone (Undertow 6089b2a, z.ai out of balance): mid is gpt-5.4-mini and strong is
  gpt-5.4. Both run unreasoned.
- The docs say otherwise. `roko-acp/src/bridge_events/dispatch.rs` (~775) claims "OpenAI-compat: maps to
  `reasoning_effort` request field when the provider supports it", and `roko-core/src/agent.rs`'s `ReasoningEffort`
  calls itself a hint passed to capable backends.

## Evidence

In the roko checkout, under `tmp/hermes-roko-v7/pilot/gpt54-proof/`:
- `live/bodies/0{2,4,5}-request.json` are the request bodies, and `live/tee.jsonl` the tee's record, which carries no
  headers.
- `live-costs.jsonl` is roko's cost record for the streamed turn: `model_reported` gpt-5.4-2026-03-05, priced at
  $0.006445.
- Undertow's `.roko/learn/efficiency.jsonl` holds the events counted above.

## Where

- `crates/roko-agent/src/provider/openai_compat.rs:241` `build_extra_body_params`. It adds only sampling
  (`temperature`, `seed`), GLM params, Kimi params and provider routing.
- `crates/roko-agent/src/openai_compat_backend.rs:547` `build_body`, for the plain and streamed bodies. It already
  switches to `max_completion_tokens` for these models (`should_use_max_completion_tokens`).
- `crates/roko-agent/src/tool_loop/backends/mod.rs` `create_observed_openai_compat_backend`, where the extra body
  params and `use_max_completion_tokens` are set.
- `crates/roko-core/src/config/provider.rs` `ModelProfile`. It has no `reasoning_effort` key. `thinking_level` is
  read only by the Gemini adapters.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs` `request_sampling` (gap-13bbbd), where an attempt records the
  sampling its requests carried.

## Plan (the lead's)

- Send `reasoning_effort` to OpenAI reasoning models, on both the plain and the streamed tool-loop request. Use the
  same detection as `max_completion_tokens` (gpt-5*, o-series), so a model that rejects the field never gets it.
- Take the value from a per-model `reasoning_effort` key in `[models.*]`, else from `[agent] default_effort`.
- Record the effort each try used on its verdict, beside `sampling`.

## Done when

The test in `[[verify]]` passes: a request body for gpt-5.4-mini carries `reasoning_effort`, both plain and streamed.
It holds `default_effort` ("medium") when the model sets none, and the model's own key when it does. A model outside
the detection gets no such field.
