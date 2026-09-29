+++
id = "find-af6b7f"
kind = "finding"
title = "Gemini thinking tokens not included in UsageObservation.output_tokens"
status = "open"
triage = "verified"
severity = "p1"
size = "S"
goal = "core"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F011"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F011"
anchors = ["crates/roko-agent/src/gemini/types.rs::UsageMetadata", "crates/roko-agent/src/gemini/native.rs::gemini_observation", "crates/roko-agent/src/gemini/types.rs::GeminiMetadata"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -Eq 'thoughtsTokenCount|thoughts_token_count' crates/roko-agent/src/gemini/types.rs && grep -rqw 'fn gemini_observation_counts_thoughts_as_output' crates/roko-agent/src/gemini/ && cargo test -p roko-agent gemini_observation_counts_thoughts_as_output"
+++

## Problem

For Gemini models that think (2.5 and later), roko under-reports output tokens and cost. The Gemini API reports
thinking tokens in `usageMetadata.thoughtsTokenCount`, separate from `candidatesTokenCount`, and bills them at
the output rate. roko's `UsageMetadata` has a field for a made-up name (`thinkingTokenCount`), so the real value
is never read. `gemini_observation` then sets `output_tokens = candidatesTokenCount` and
`reasoning_tokens: None`.

Example: a response with `candidatesTokenCount: 120` and `thoughtsTokenCount: 900` is recorded as 120 output
tokens, and the cost is computed from 120 instead of 1,020. Expected: `output_tokens = 1020`,
`reasoning_tokens = Some(900)`.

## Why it matters

- Goal `core`: cost accounting feeds budgets, `roko show costs`, the efficiency log and routing. Thinking can be
  most of a Gemini response, so Gemini looks far cheaper than it is and the router favours it on false numbers.
- Learning is affected too: `roko-learn`'s cascade router reads `GeminiMetadata.thinking_tokens`
  (`crates/roko-learn/src/cascade_router.rs:1433`, totals at `:1760`), which is always `None` for live
  responses, so `total_gemini_thinking_tokens` stays 0.

## Where

Entry point: any dispatch to a `GeminiApi` model (Graph task dispatch -> `create_agent_for_model` -> the native
Gemini agent).

- `crates/roko-agent/src/gemini/types.rs::UsageMetadata` (`:235-246`): `#[serde(rename_all = "camelCase")]`,
  so field `thinking_token_count` expects JSON `thinkingTokenCount`, which the API never sends.
- `crates/roko-agent/src/gemini/native.rs::gemini_observation` (`:560-586`): builds the `UsageObservation`. It is
  called from the chat path (`:407`, the `ChatResponse.usage`) and the agent run path (`:501`).
- `crates/roko-agent/src/gemini/native.rs` (`:393-400`): fills `GeminiMetadata.thinking_tokens` from the same
  field.
- `crates/roko-core/src/usage.rs` (`:15-21`): `UsageObservation`. `reasoning_tokens` is documented as a subset of
  `output_tokens` (the OpenAI o-series convention).
- `crates/roko-core/src/chat_types.rs::fill_cost_from_pricing` (`~:196-215`): prices `input_tokens`,
  `output_tokens` and the cache fields. `reasoning_tokens` is not priced separately, so thinking must be inside
  `output_tokens` to be billed.
- Fixtures that use the wrong name: `crates/roko-agent/src/gemini/types.rs:345` (asserted at `:370-376`),
  `crates/roko-agent/src/gemini/native.rs:882` (asserted at `:913`),
  `crates/roko-agent/tests/gemini_integration.rs:488` (asserted at `:512`).

## Current state

- Unchanged at HEAD. The last commits on these files (`59a1db0f4`, `72e0a76b8`) did not touch usage parsing.
- The field has never been populated from a live response, because only test fixtures use `thinkingTokenCount`.
- Out of scope, and unknown whether tracked elsewhere: the Gemini CLI adapter
  (`crates/roko-agent/src/provider/gemini_cli.rs`) parses no usage at all, and the OpenAI-compatible Gemini path
  (`crates/roko-agent/src/gemini/compat.rs`) goes through `CodexAgent` usage parsing.

## Plan

1. In `UsageMetadata`, rename the field to `thoughts_token_count` so camelCase gives `thoughtsTokenCount`. Keep
   `#[serde(alias = "thinkingTokenCount")]` so old fixtures and any stored JSON still parse.
2. In `gemini_observation`, when usage is present:
   - `output_tokens = candidates_token_count.unwrap_or(0) + thoughts_token_count.unwrap_or(0)`, and `None` only
     when both are absent;
   - `reasoning_tokens = thoughts_token_count`.
3. Update `GeminiMetadata.thinking_tokens` in `native.rs:398` to read the renamed field.
4. Change the fixtures to the real API name (`thoughtsTokenCount`), and keep one fixture that uses the alias.
5. Add unit test `gemini_observation_counts_thoughts_as_output` in `native.rs` tests: usage
   `{promptTokenCount: 10, candidatesTokenCount: 120, thoughtsTokenCount: 900, totalTokenCount: 1030}` gives
   `output_tokens == Some(1020)` and `reasoning_tokens == Some(900)`. Add a second case with no thoughts, which
   must be unchanged from today.
6. Optional: `toolUsePromptTokenCount` (input tokens used by grounding or code-execution tools) is also ignored
   today. Add it to `input_tokens` only after confirming how Google bills it.

## Done when

- A live-shaped Gemini response with `thoughtsTokenCount` produces `output_tokens = candidates + thoughts` and
  `reasoning_tokens = thoughts`, and the dispatch cost includes the thinking tokens at the output rate.
- `GeminiMetadata.thinking_tokens` is populated from `thoughtsTokenCount`.
- Responses without thoughts behave as before.
- Verify:
  `grep -Eq 'thoughtsTokenCount|thoughts_token_count' crates/roko-agent/src/gemini/types.rs && grep -rqw 'fn gemini_observation_counts_thoughts_as_output' crates/roko-agent/src/gemini/ && cargo test -p roko-agent gemini_observation_counts_thoughts_as_output`

## Notes

- Keep the `UsageObservation` convention: `reasoning_tokens` is a subset of `output_tokens`. Do not also add
  reasoning to cost separately, or it is counted twice.
- The old verify command only checked the field name and the absence of `reasoning_tokens: None`. A fix that set
  `reasoning_tokens` but left `output_tokens` unchanged would have passed. The command above adds a behaviour
  test.
- Small and local to `crates/roko-agent/src/gemini/`. Safe to run in parallel with other work.

## Original notes

Gemini 2.5+ models return `thinkingTokenCount` in usage metadata. This field is not extracted or added to `UsageObservation.output_tokens`. Thinking token costs are therefore not reflected in cost accounting for Gemini models that perform extended thinking.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F011`
- `tmp/archive/provider-audit/05-gemini-api.md`

How to verify: Confirm in crates/roko-agent/src/gemini/native.rs whether still true: Gemini thinking tokens not included in `UsageObservation.output_tokens`

Verified 2026-09-28: still true, and worse than reported. `UsageMetadata` (crates/roko-agent/src/gemini/types.rs:245) deserializes `thinking_token_count` as camelCase `thinkingTokenCount`, but the Gemini API reports `thoughtsTokenCount`, so the field is never populated from live responses. Only fixtures use the made-up name (types.rs:345, native.rs:882, tests/gemini_integration.rs:488). `gemini_observation` (native.rs:560-584) sets `output_tokens = candidates_token_count` and `reasoning_tokens: None`, so thinking tokens are missing from cost accounting.
