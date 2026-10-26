+++
id = "bug-29014c"
kind = "bug"
title = "roko sends OpenAI no reasoning_effort: [agent] default_effort = \"medium\" never reaches openai_compat; build_extra_body_params (provider/openai_compat.rs:241) adds only sampling, GLM, Kimi and routing params, so the GPT-5.4 family runs at the API default (0 reasoning tokens in all 52 gpt-5.4-mini efficiency events in Undertow, phases 0-3, and in the 2026-10-21 gpt-5.4 probe)"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/provider"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "4a8c6a72d"
source = "round-v7-hermes-pilot#N-gpt54-proof"
discovered_from = "agent:hermes-pilot (round v7, the fenced gpt-5.4 call before phase 4; tmp/hermes-roko-v7/pilot/gpt54-proof)"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs::build_extra_body_params", "crates/roko-agent/src/openai_compat_backend.rs::build_body", "crates/roko-agent/src/tool_loop/backends/mod.rs::create_observed_openai_compat_backend", "crates/roko-core/src/config/provider.rs::ModelProfile", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::request_sampling"]
links = { depends_on = [], blocks = [], related = ["bug-cbbaa8", "gap-13bbbd", "gap-1ba0ac", "bug-35b190"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_openai_reasoning_model_request_carries_reasoning_effort' crates/roko-agent/src && cargo test -p roko-agent --lib -- an_openai_reasoning_model_request_carries_reasoning_effort"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T10:46:47Z"
commit = "e8ca7a243"
by = "roko-effort"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-21T06:13:57Z"
forced = false
evidence = "Merged as 4a8c6a72d (narrowed fix e8ca7a243 on f37c1431f). [models.*] reasoning_effort (roko_core::ReasoningEffort; loader sentinel, ModelProfileLayer); create_agent_for_model, the data-LLM boundary and ACP give a model that names none the request's effort, else [agent] default_effort; build_extra_body_params sends it to gpt-5*, o1, o3, o4 (not -chat, o1-mini, o1-preview; max as high), and OpenAiCompatLlmBackend::build_body leaves it off any request with function tools, which chat completions refuses for the GPT-5.4 family (phase 4 run 16fc88ae's HTTP 400 on f37c1431f). The verdict records executed.reasoning_effort: none for a model that calls tools. Verify an_openai_reasoning_model_request_carries_reasoning_effort passes, with reasoning_effort_goes_only_on_requests_without_function_tools, openai_reasoning_models_take_the_profile_effort, a_models_reasoning_effort_survives_a_load and a_verdict_records_the_reasoning_effort_sent. On 8a3774bcd fmt, clippy (6 crates), check --tests and the lib suites are green: roko-core 2048, roko-agent 2400, roko-learn 1393, roko-cli 3992, roko-serve 1081, roko-acp 237. Real API on roko-effort-e4bd6ad74 (the same request path): the pilot's check.sh PASS (5 tool calls, HTTP 200, none with reasoning_effort, verdict none, $0.00384), and a fenced tool-less gpt-5.4-mini call at medium: HTTP 200, 10 reasoning tokens, verdict medium ($0.00084). Reasoning with tools needs the Responses API: gap-1ba0ac."
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

## Update (2026-10-21): narrowed after phase 4 run 16fc88ae

The first fix (f37c1431f) sent `reasoning_effort` with every request. Phase 4's run `16fc88ae`, on c3f41dc41 (main
plus that fix), failed every OpenAI tool call with HTTP 400 `invalid_request_error`, `param: reasoning_effort`:
"Function tools with reasoning_effort are not supported for gpt-5.4-mini in /v1/chat/completions. To use function
tools, use /v1/responses or set reasoning_effort to 'none'." gpt-5.4's rung probe failed the same way. The $0 proof had
used a stand-in upstream that checks no parameters.

The lead's decision: narrow this item, and file the real fix as gap-1ba0ac.

- **Gate:** `OpenAiCompatLlmBackend::build_body` leaves `reasoning_effort` off any request that carries function
  tools, plain or streamed. A request without tools carries the effort.
- **Unchanged:** the per-model key, the precedence (the model's own, then the request's effort, then
  `[agent] default_effort`), the slug detection (gpt-5*, o1, o3, o4, not their `-chat` variants, o1-mini or
  o1-preview), and `max` sent as `high`.
- **Record:** the verdict's `executed.reasoning_effort` holds what was sent. For a model that calls tools, that is
  `none`: its requests carry function tools, so the API's default ran.
- **Which calls are tool-less** (checked in the code at a9728264a):
  - Every agentic call to an OpenAI model with `supports_tools` (the default) carries function tools, and so no
    effort: plan tries (`AgentDispatchRequest` with a contract), rung probes (`echo`), the planner (Read, Grep,
    Glob), the plan brief (`READ_ONLY_TOOLS`) and the chat.
  - The distiller (`ModelCallService`) and the cheap helpers (error enrichment, gate reflections, the judge:
    `CheapFactoryAgent`, `tools: None`) name no tools. They therefore get the whole local catalog, not none.
  - Tool-less, and so sent the effort: a `[models.*]` entry with `supports_tools = false` (the adapter's tool-less
    `CodexAgent`), and the CaMeL data LLM (`[agent.data_llm]`, off by default), which streams its turns with an empty
    tool list.
- **The real fix**, an agentic GPT-5.x call that reasons with tools, needs the Responses API: gap-1ba0ac.

## Done when

The test in `[[verify]]` passes:
- A tool-loop request for gpt-5.4-mini or gpt-5.4, plain or streamed, carries tools and no `reasoning_effort`.
- A request without tools carries `default_effort` ("medium") when the model sets none, the model's own key when it
  does, and the request's own effort over the default.
- A model outside the detection gets no such field.

`a_verdict_records_the_reasoning_effort_sent` (roko-cli) shows a tool-loop try's verdict recording `none`. The pilot's
real-API check (`tmp/hermes-roko-v7/pilot/gpt54-proof/check.sh`) passes on the binary.
