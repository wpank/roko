+++
id = "gap-fd0c0b"
kind = "gap"
title = "extract_finish_reason_raw misses Anthropic/Gemini native bodies, and hermes/safety-data-llm never check it at all"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-agent/translate", "roko-agent/streaming"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-4 follow-up reports 2026-10-02 (bug-e3940b's worker)"
discovered_from = "bug-e3940b"
anchors = ["crates/roko-agent/src/translate/mod.rs::extract_finish_reason_raw", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::normalize_response", "crates/roko-agent/src/tool_loop/backends/gemini_native.rs", "crates/roko-agent/src/hermes/http_adapter.rs", "crates/roko-agent/src/safety/data_llm.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn anthropic_native_stop_reason_reaches_extract_finish_reason' crates/roko-agent/ && cargo test -p roko-agent anthropic_native_stop_reason_reaches_extract_finish_reason"

[[verify]]
command = "grep -rqw 'fn hermes_adapter_flags_a_length_truncated_turn' crates/roko-agent/ && cargo test -p roko-agent hermes_adapter_flags_a_length_truncated_turn"
+++

## Problem

Three more paths silently accept a truncated (output-token-limited) answer as complete, beyond the one
bug-e3940b fixed (a streamed `Length` finish reason being Debug-formatted instead of lowercased):

**1. `extract_finish_reason_raw` cannot read two native, non-OpenAI-shaped bodies.**
`BackendResponse::extract_finish_reason_raw` (`crates/roko-agent/src/translate/mod.rs:469`) has three arms: `Json`
reads only `v.pointer("/choices/0/finish_reason")` (OpenAI/Ollama shape); `StreamJson` handles Claude CLI's own
streamed-JSON events (a top-level `stop_reason` per `result` event); `Text` always returns `None`.

- **Anthropic's native (non-streaming) tool loop.** `provider/anthropic_api/tool_loop.rs::normalize_response`
  (line 557) builds the `BackendResponse::Json` this path returns (`let response =
  BackendResponse::Json(Self::normalize_response(json));`, line 672). Its output has a `choices: [{message:
  {role, content}}]` array — so `/choices/0` exists — but that choice object carries no `finish_reason` field;
  the real value is a **sibling** top-level key, `"stop_reason": raw.get("stop_reason")...` (line 568), never
  copied into `choices[0]`. So `extract_finish_reason_raw()` on *any* Anthropic native non-streaming response
  returns `None`, even though the raw stop reason (e.g. `"max_tokens"`) is sitting right there in the same JSON
  value, one level up from where the pointer looks.
- **Gemini native.** `tool_loop/backends/gemini_native.rs:323` builds `BackendResponse::Json(json)` directly from
  the raw `GenerateContentResponse` body, whose finish reason lives at `/candidates/0/finishReason` (confirmed
  correct for the *streaming* path at line 483, which builds `StreamEvent`s directly and is unaffected). Neither
  the key name (`candidates` vs `choices`) nor the field name (`finishReason` vs `finish_reason`) matches the
  `Json` arm's pointer, so any code that calls `extract_finish_reason_raw()` on a `BackendResponse::Json` built
  from a raw Gemini body gets `None` too. (The :323 call site itself is on a pre-execution safety-check path, not
  confirmed to be the main turn-completion path for every caller — whoever fixes this should confirm whether a
  non-streaming Gemini turn ever reaches `extract_finish_reason_raw` through a different route as well.)

**2. Two callers of `collect_stream_to_response` never read the finish reason at all.**
`crates/roko-agent/src/hermes/http_adapter.rs:449` and `crates/roko-agent/src/safety/data_llm.rs:428` both call
`collect_stream_to_response(stream, ...)` (the function bug-e3940b fixed) and get back a `BackendResponse` whose
`finish_reason` field is now correctly lowercased — but neither file calls `extract_finish_reason_raw()` or
anything like `tool_loop/mod.rs`'s `hit_length_limit` on the result anywhere (grepped both files fully for
`finish_reason`: no other occurrence). `http_adapter.rs` (lines 449-466) goes straight from the collected response
to `extract_content`, usage resolution and `AgentResult::ok(...)`. `data_llm.rs` (lines 428-439) goes straight to
`asks_for_tools` and `extract_text().trim()`. A length-truncated answer on either path is indistinguishable from a
complete one.

## Why it matters

Goal: truth (honest failure diagnosis), same class as bug-e3940b (p1, done). Confirmed production impact:
- Anthropic's native non-streaming tool loop is a real, used provider path (not a fallback or test-only route).
- The Hermes HTTP adapter (`roko-agent-server`'s relay to a Hermes-hosted model) and the safety data-LLM (used for
  safety screening judgments, `crates/roko-agent/src/safety/data_llm.rs`) both silently accept a cut-off answer:
  for Hermes that's a user-facing truncated reply reported as done; for the safety data-LLM, a truncated
  screening verdict is trusted as if it were a complete one, which is a correctness risk in a safety-relevant
  path.

## Where

- `crates/roko-agent/src/translate/mod.rs::BackendResponse::extract_finish_reason_raw` (the `Json` arm, line ~472).
- `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::normalize_response` (line 557) and its caller
  (line 672).
- `crates/roko-agent/src/tool_loop/backends/gemini_native.rs` (line 323, and the already-correct streaming
  extraction at line 483 for comparison).
- `crates/roko-agent/src/hermes/http_adapter.rs` (around line 449-466).
- `crates/roko-agent/src/safety/data_llm.rs` (around line 428-439).
- Consumer pattern to reuse: `crates/roko-agent/src/tool_loop/mod.rs:1373-1375`'s `hit_length_limit` check (now
  fixed by bug-e3940b).

## Current state

Unfixed. `bug-e3940b` (done, `db49bfd1d`) fixed the *streamed, Debug-formatted* case specifically for
`collect_stream_to_response`'s own string conversion; it explicitly left open ("Notes: ... check for a second
affected path when fixing this") whether other paths were also affected. These are those other paths: one where
the raw data the extractor needs is present but unreachable by its pointer (Anthropic, Gemini), and two where
nothing ever asks the question at all (Hermes adapter, safety data-LLM).

## Plan

1. `normalize_response` (Anthropic): copy `stop_reason` into `choices[0].finish_reason` too (or change
   `extract_finish_reason_raw`'s `Json` arm to also check a top-level `stop_reason` key when `/choices/0/
   finish_reason` is absent).
2. Gemini: either route non-streaming Gemini calls through the same `GenerateContentResponse` -> `StreamEvent`
   conversion the streaming path already uses (so finish reason flows through the one correct path), or add a
   `/candidates/0/finishReason` check (mapped through the same case-normalization bug-e3940b introduced) to the
   `Json` arm.
3. `hermes/http_adapter.rs` and `safety/data_llm.rs`: after `collect_stream_to_response`, check the result's
   finish reason the same way `tool_loop/mod.rs`'s `hit_length_limit` does (factor that check into a shared
   helper both the tool loop and these two callers can use, so the three don't drift apart again) and surface a
   truncation distinctly (an error variant, or a flagged field on the result) rather than silently returning as
   if complete.
4. Regression tests: an Anthropic-shaped `BackendResponse::Json` with `stop_reason: "max_tokens"` is recognized as
   truncated; a Hermes/safety-data-llm turn whose collected stream reports a length-limited finish reason is not
   treated as a normal, complete result.

## Done when

- An Anthropic native non-streaming response with `stop_reason: "max_tokens"` is recognized as truncated by
  `extract_finish_reason_raw`/`hit_length_limit` (or its equivalent).
- The Hermes HTTP adapter and the safety data-LLM no longer accept a length-truncated collected response as
  complete.
- The `[[verify]]` commands pass.

## Notes

- `discovered_from` bug-e3940b (done): that item's own Notes explicitly anticipated "a second affected path" —
  this item is that path, found by the same worker while implementing the fix, filed separately since the
  mechanism (missing extraction vs. extraction-present-but-unchecked) is distinct from the Debug-formatting bug
  bug-e3940b fixed.
- Severity: p2, not p1 like bug-e3940b — these paths are narrower (one specific non-streaming provider path per
  backend, plus two auxiliary callers), not the main graph-dispatch hot path every attempt goes through.
- Do not weaken `asks_for_tools`' own early-return in `data_llm.rs`; the finish-reason check belongs alongside it,
  not in place of it.

## Progress

- 2026-10-04 (w4-length): implemented on `work/gap-fd0c0b` at 0967833dc; cargo verification deferred to the batch
  gate.
  - `extract_finish_reason_raw`'s `Json` arm falls back from `choices[0].finish_reason` to the top-level
    `stop_reason` (Anthropic native, which `normalize_response` already keeps), then to Gemini's
    `candidates[0].finishReason`, lower-cased. A non-streaming Gemini turn does reach it: `send_turn` returns the
    raw body as `BackendResponse::Json`.
  - `BackendResponse::hit_length_limit` is the shared length check. The tool loop uses it (its now-unused
    imports dropped), and so do the two collectors that never asked. The Hermes HTTP adapter fails a cut-off turn,
    streaming and non-streaming, and keeps its usage. The data LLM withholds a cut-off answer as the new
    `DataLlmWithheld::Truncated`, checked beside `asks_for_tools`, not in its place.
  - Tests: `anthropic_native_stop_reason_reaches_extract_finish_reason`,
    `hermes_adapter_flags_a_length_truncated_turn`, `data_llm_boundary_withholds_a_truncated_answer`, and a Gemini
    case in the finish-reason test.
