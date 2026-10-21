+++
id = "gap-1ba0ac"
kind = "gap"
title = "roko's OpenAI tool loop speaks only chat completions, where the GPT-5.4 family refuses reasoning_effort beside function tools (HTTP 400, run 16fc88ae), so no agentic gpt-5.x call reasons: it needs the Responses API (/v1/responses)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "L"
subsystem = ["roko-agent/provider"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7 phase 4 run 16fc88ae (Undertow, 2026-10-21 08:06Z), the lead's decision on bug-29014c"
discovered_from = "agent:roko-effort (round v7, bug-29014c's first fix; the 400 seen by hermes-pilot and the lead)"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs::OpenAiCompatLlmBackend", "crates/roko-agent/src/tool_loop/backends/mod.rs::create_observed_openai_compat_backend", "crates/roko-agent/src/translate/openai.rs", "crates/roko-agent/src/provider/openai_compat.rs::build_extra_body_params"]
links = { depends_on = [], blocks = [], related = ["bug-29014c", "bug-35b190", "bug-cbbaa8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_gpt5_tool_loop_reasons_on_the_responses_api' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_gpt5_tool_loop_reasons_on_the_responses_api"
+++

## Problem

roko's OpenAI tool loop speaks only chat completions. On that endpoint, OpenAI refuses a reasoning effort beside
function tools for the GPT-5.4 family, so no agentic call roko makes to gpt-5.4 or gpt-5.4-mini can reason.

- `OpenAiCompatLlmBackend` posts every tool-loop turn, plain or streamed, to `{base_url}/chat/completions`, and every
  agentic call carries function tools: plan tries, rung probes (`echo`), the planner (Read, Grep, Glob), the plan
  brief, the chat, the distiller and the cheap helpers. The last three name no tools, so they get the whole local
  catalog.
- With `reasoning_effort` set, OpenAI answers HTTP 400 `invalid_request_error`, `param: reasoning_effort`:
  "Function tools with reasoning_effort are not supported for gpt-5.4-mini in /v1/chat/completions. To use function
  tools, use /v1/responses or set reasoning_effort to 'none'." gpt-5.4 gets the same.
  - Seen in round v7, phase 4 run `16fc88ae` (Undertow, 2026-10-21 08:06Z), on bug-29014c's first binary c3f41dc41,
    which sent the field to every request.
  - gpt-5.4's rung probe failed, and T1's four gpt-5.4-mini tries each failed in 0.3 s.
- Without the field, the API default applies, which for this family is no reasoning: 0 reasoning tokens in all 52
  gpt-5.4-mini efficiency events of Undertow's phases 0–3 (bug-29014c).
- bug-29014c, narrowed, sends `reasoning_effort` only on a request without function tools, and records `none` on the
  verdict of a tool-loop try. Today only a `[models.*]` entry with `supports_tools = false` and the CaMeL data LLM
  (`[agent.data_llm]`, off by default) make such requests.

## Why it matters

- Phase 4's ladder runs on OpenAI alone (mid gpt-5.4-mini, strong gpt-5.4). Every try runs unreasoned.
- gpt-5.4-mini, unreasoned, handed back with its own tests red at 13–25 of 120 turns in phase 3 (bug-cbbaa8), and went
  0 of 8 on phase 2's DSP checks.
- `[agent] default_effort` and a model's `reasoning_effort` (bug-29014c) have no effect on any OpenAI agentic call.

## Where

- `crates/roko-agent/src/openai_compat_backend.rs::OpenAiCompatLlmBackend`: `build_body` (the chat completions
  body, and since bug-29014c the gate that leaves `reasoning_effort` off a request with tools), `send_turn`,
  `stream_turn` and the SSE parsing.
- `crates/roko-agent/src/tool_loop/backends/mod.rs::create_observed_openai_compat_backend`: picks the tool-loop
  backend by provider kind. The Responses backend would be chosen here.
- `crates/roko-agent/src/translate/openai.rs`: the chat-format tool calls and messages, and the usage parsing
  (`completion_tokens_details.reasoning_tokens`).
- `crates/roko-agent/src/provider/openai_compat.rs::reasoning_effort_param`: the effort roko would send, by
  bug-29014c's precedence (the model's own key, then the request's effort, then `[agent] default_effort`).

## Current state

What OpenAI's API reference says about the Responses API. None of this has been exercised from roko yet, so check
each point against the live API first.

- **Endpoint:** `POST {base}/v1/responses`, with the same bearer auth.
- **Request:**
  - `model`; `instructions`, the system prompt.
  - `input`: a string or a list of items.
    - Messages: `{"role": "user"|"assistant"|"developer", "content": [{"type": "input_text", "text": …}]}`. An
      assistant's own text is `output_text`.
    - Tool results: `{"type": "function_call_output", "call_id": …, "output": "…"}`.
  - `tools`: function tools, flat: `{"type": "function", "name", "description", "parameters", "strict"}`. There is no
    nested `function` object as in chat completions.
  - `tool_choice`, `parallel_tool_calls`.
  - `reasoning`: `{"effort": "low"|"medium"|"high", "summary": "auto"}`. Some 5.x models also take `none`,
    `minimal` or `xhigh`.
  - `max_output_tokens`, in place of `max_tokens` and `max_completion_tokens`.
  - `stream: true`.
  - State: `store: false` with `include: ["reasoning.encrypted_content"]` keeps it stateless; `previous_response_id`
    relies on OpenAI storing the response.
- **Response:**
  - `output`, a list of items:
    - `{"type": "reasoning", "id", "summary": […], "encrypted_content"?}`;
    - `{"type": "function_call", "id", "call_id", "name", "arguments": "<json string>"}`;
    - `{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text"}]}`.
  - `usage`: `input_tokens`, `input_tokens_details.cached_tokens`, `output_tokens`,
    `output_tokens_details.reasoning_tokens`, `total_tokens`.
  - `status`: `completed`, or `incomplete` with `incomplete_details.reason`, e.g. `max_output_tokens`.
- **Streaming:** typed SSE events, such as `response.created`, `response.output_item.added` / `.done`,
  `response.output_text.delta`, `response.function_call_arguments.delta` / `.done`,
  `response.reasoning_summary_text.delta`, `response.completed` (the whole response, with usage),
  `response.failed` and `error`.
- **Tool turns:** each next request has to carry the previous turn's output items, its reasoning items (with
  `encrypted_content` when `store` is false) and the `function_call_output` items. Otherwise the model loses its
  reasoning between tool calls. `previous_response_id` is the stored alternative.

## Plan

1. **A Responses backend** (an `LlmBackend`) beside `OpenAiCompatLlmBackend`. It translates the tool loop's
   chat-format history:
   - system → `instructions`; user and assistant text → message items;
   - `tool_calls` → `function_call` items; tool results → `function_call_output` items.
   It sends flat tools, `reasoning.effort` by bug-29014c's precedence and `max_output_tokens`. It stays stateless
   (`store: false` and `include: ["reasoning.encrypted_content"]`), so retries and resumes don't depend on
   OpenAI-side state, and it carries each turn's reasoning items into the next.
2. **Streaming:** map the typed events onto the tool loop's `StreamEvent`s: text and reasoning deltas, tool-call
   deltas, and `Done` with a finish reason. TTFT comes from the first delta, and usage from `response.completed`.
3. **Choice:** a provider switch (e.g. `[providers.openai] api = "responses"`), or automatic for OpenAI's own base URL
   with a gpt-5* or o-series slug. Every other OpenAI-compatible provider keeps chat completions.
4. **Usage and price:** reasoning tokens from `output_tokens_details`, cached input from `input_tokens_details`.
5. **Errors:** map them as chat completions does, including bug-35b190's typed refusals.
6. **Live check:** one fenced call like `tmp/hermes-roko-v7/pilot/gpt54-proof/check.sh` (two probes, one streamed
   tool turn), now showing reasoning tokens at effort medium.

## Done when

A gpt-5.4-mini tool-loop call through roko meets all of these:
- it goes to `/v1/responses` with `reasoning.effort` from bug-29014c's precedence;
- a tool call round-trips (`function_call` → `function_call_output` → final text) with its reasoning items carried;
- the verdict and the efficiency events record its reasoning tokens.

Also, the `[[verify]]` test passes, and a fenced live check shows reasoning tokens above 0 at effort medium.

## Notes

- Only OpenAI itself serves `/v1/responses`. z.ai, Cerebras, OpenRouter, Hermes and the other OpenAI-compatible
  providers stay on chat completions.
- OpenAI's documentation says gpt-5 (the first) and the o-series take `reasoning_effort` beside tools on chat
  completions. That is unverified here: bug-29014c's gate leaves the field off every request with tools.
- Related: bug-29014c (the narrow fix this extends), bug-35b190 (the 400 read as a network error), bug-cbbaa8.
