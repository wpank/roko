+++
id = "bug-e3940b"
kind = "bug"
title = "A streamed length finish reason is Debug-formatted as Length, so hit_length_limit never matches"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
size = "S"
subsystem = ["roko-agent/streaming"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "db49bfd1d"
source = "backlog wave reports 2026-10-02 (PK01 gap-625195)"
discovered_from = "gap-625195 (adjacent to backlog task 1111; stream_without_finish_reason_is_not_stop already shows the capitalized string without flagging it)"
anchors = ["crates/roko-agent/src/streaming.rs::parse_sse_frame", "crates/roko-agent/src/tool_loop/mod.rs::collect_stream_to_response"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn collected_length_finish_reason_is_recognized_as_truncated' crates/roko-agent/ && cargo test -p roko-agent collected_length_finish_reason_is_recognized_as_truncated"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T21:13:16Z"
commit = "db49bfd1d"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-02T19:13:07Z"
forced = false
evidence = "Gate 4b (work/backlog-batch-4b with main and the workflow audit merged in; merged into main as db49bfd1d, which differs from the gated tree only in work/ and one later docs commit): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 12,641 passed over 14 crates, golden-path canaries 13/13, sse_replay 1/1, ViabilityBench suite 513 passed; every [[verify]] passes."
+++

## Problem

A streamed response that hits its output-token limit is not recognized as truncated. The tool loop's blank-answer
diagnosis (`crates/roko-agent/src/tool_loop/mod.rs:1373-1375`) checks the response's finish reason as a raw string:

```
let hit_length_limit = finish_reason_raw.as_deref().is_some_and(|r| r == "length" || r == "max_tokens");
```

But for any backend whose stream is collected via the shared `collect_stream_to_response` helper
(`crates/roko-agent/src/tool_loop/mod.rs:376`), the finish-reason string that reaches this check is never lowercase
`"length"` — it's the `FinishReason` enum's Debug-derived variant name, e.g. `"Length"` (capital L), because the SSE
parser normalizes the wire string into the enum and then immediately re-stringifies it with `{:?}` instead of the
lowercase wire mapping:

```
// crates/roko-agent/src/streaming.rs:367-372
let finish_reason = normalize_finish_reason(reason);
...
finish_reason: format!("{finish_reason:?}"),
```

`collect_stream_to_response` (`tool_loop/mod.rs`, the `StreamEventKind::Done { finish_reason: fr }` arm around line
462-467) then copies that PascalCase string straight into the collected response's `"finish_reason"` JSON field with
no re-lowercasing. So `"length" == "Length"` is always false, `hit_length_limit` is always false for any stream
collected this way, and a blank, length-truncated answer falls into the generic "the model returned no text and no
tool call; failing the run as empty_response" branch instead of the specific, actionable "model hit output token
limit... increase max_output" diagnosis.

This is already locked in by an existing test: `crates/roko-agent/src/openai_compat_backend.rs`'s
`stream_without_finish_reason_is_not_stop` (backlog task 1111, implemented) asserts
`finish_reasons(&items) == vec!["Length".to_string()]` for a chunk whose wire `finish_reason` was `"length"` — the
capitalization is already observed by a test, just not connected to the `hit_length_limit` check's expectation.

## Why it matters

Goal: truth / safe failure diagnosis. Confirmed production call sites of the affected `collect_stream_to_response`
path: `crates/roko-agent-server/src/state.rs:329` (the per-agent HTTP sidecar's real LLM dispatch),
`crates/roko-agent/src/safety/data_llm.rs:428` (safety-screening LLM calls),
`crates/roko-agent/src/hermes/http_adapter.rs:449` (Hermes HTTP adapter). A truncated answer on any of these paths
is misdiagnosed as a generic empty response rather than "hit the output token limit," which hides the actual cause
and the fix (raise `max_output`) from both operators and any automatic retry/escalation logic keyed on the
distinction.

## Where

- Root cause: `crates/roko-agent/src/streaming.rs::parse_sse_frame` (around lines 360-373), the
  `finish_reason: format!("{finish_reason:?}")` line.
- Propagation: `crates/roko-agent/src/tool_loop/mod.rs::collect_stream_to_response` (line 376), the
  `StreamEventKind::Done { finish_reason: fr }` arm around line 462-467 and the final `"finish_reason": finish_reason`
  JSON field it builds.
- Consumer: `crates/roko-agent/src/tool_loop/mod.rs:1373-1375` (`hit_length_limit`).
- Contrast: `crates/roko-agent/src/openai_compat_backend.rs::finish_reason_to_wire` (line 926) does this conversion
  correctly (explicit lowercase match arms) for that backend's own `stream_response_to_json` aggregation path — so
  not every streaming path is affected, only ones that go through the shared `collect_stream_to_response`.

## Current state

Unfixed. The behavior is implicitly pinned by `openai_compat_backend.rs`'s `stream_without_finish_reason_is_not_stop`
test, which asserts the PascalCase string as expected output of the SSE parser layer, without exercising
`collect_stream_to_response` or `hit_length_limit` together in the same test.

## Plan

1. Change `crates/roko-agent/src/streaming.rs`'s finish-reason stringification to use the same lowercase wire
   convention as `openai_compat_backend.rs::finish_reason_to_wire` (`Stop` -> `"stop"`, `Length` -> `"length"`,
   `ToolCalls` -> `"tool_calls"`, `ContentFilter` -> `"content_filter"`, `Error(reason)` -> `reason`), rather than
   `format!("{finish_reason:?}")`.
2. Update `sse_parser_reads_finish_reason` and any other test asserting the PascalCase form (e.g. the
   `"ToolCalls"`/`"Length"` assertions in `streaming.rs` and `openai_compat_backend.rs`) to the lowercase form.
3. Add a regression test exercising the full chain: an SSE stream with `finish_reason: "length"`, collected via
   `collect_stream_to_response`, then checked against `hit_length_limit`'s own logic (or a shared helper),
   confirming a truncated/blank answer is recognized as such.

## Done when

- A stream reporting `finish_reason: "length"` and collected via `collect_stream_to_response` is recognized by
  `hit_length_limit` as a length-limited response.
- The `[[verify]]` command passes.

## Notes

Severity p1: this silently corrupts failure diagnosis on at least three production call sites (agent-server, safety
data-LLM calls, Hermes HTTP adapter), and likely more backends that rely on `collect_stream_to_response`. Not
confirmed whether the CLI's main graph-dispatch path (which appears to go through `openai_compat_backend.rs`'s own,
correctly-lowercased `stream_response_to_json`) is also affected by some other route — check for a second affected
path when fixing this.

## Progress

- bug-e3940b: implemented on `work/bug-e3940b` at 09a103c81; cargo verification deferred to the batch gate. Premise
  re-checked at 248d279c7 and confirmed. `FinishReason::as_str` (roko-core) is now the one text form for a finish
  reason: the SSE parser, `ChatResponse::to_signal` and the four hand-copied wire mappings (`openai_compat_backend`,
  `cursor_agent`, the sidecar's `messaging` and `relay_client`) use it, and `hit_length_limit` reads through
  `normalize_finish_reason`.
- Second affected path (the Notes' question): yes. `ToolLoop::send_turn_streaming`, which `dispatch_v2` and the ACP
  bridge reach through `run_streaming`, collects through `collect_stream_to_response`, so the CLI's Graph dispatch on
  an OpenAI-compatible backend was affected. The "correct" contrast path, `stream_response_to_json` in
  `openai_compat_backend.rs` (and its copy in `cursor_agent.rs`), has no callers.
- Other sites with the same mismatch, fixed: the sidecar's `response_finish_reason` read every streamed reason back as
  an error (`Error("Stop")`, `Error("Length")`) and Gemini's `STOP`/`MAX_TOKENS` as errors; `to_signal` tagged `Stop`.
  roko-cli has no finish-reason comparison of its own (`dispatch_v2` drops the string).
- Tests: `collected_length_finish_reason_is_recognized_as_truncated` (the verify),
  `sse_parser_writes_canonical_finish_reasons`, `finish_reason_text_round_trips`, `finish_reason_text_is_the_wire_form`,
  `streamed_finish_reason_keeps_its_meaning` and `gemini_finish_reasons_read_lower_cased`. Five assertions that pinned
  `ToolCalls`, `Length` or `Stop` now expect the canonical text.
- Behaviour change to expect at the gate and in live runs: a streamed turn that ends on `length` now stops the loop with
  `BackendError("model hit output token limit (finish_reason=length)")`, as a non-streamed turn already did.
- Left for separate items: `extract_finish_reason_raw` reads only `choices[0].finish_reason`, so the non-streaming
  Anthropic tool-loop response (top-level `stop_reason`) and Gemini native (`candidates[0].finishReason`) report no
  finish reason and `hit_length_limit` misses their truncations; the Hermes HTTP adapter and the safety data-LLM never
  read the finish reason, so a truncated answer there is still accepted as complete.
