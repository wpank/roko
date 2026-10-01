+++
id = "bug-3aa61f"
kind = "bug"
title = "Non-streamed Gemini tool-loop turns record no usage: send_turn returns raw Gemini JSON and extract_usage reads only an OpenAI-shaped usage block"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on bug-2b1ddc/find-af6b7f/bug-b9cb83)"
anchors = ["crates/roko-agent/src/tool_loop/backends/gemini_native.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["find-af6b7f", "bug-afcf63"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_non_streamed_gemini_turn_records_its_usage' crates/roko-agent/src/ && cargo test -p roko-agent --lib a_non_streamed_gemini_turn_records_its_usage"
+++

## Problem

gemini_native send_turn returns the raw Gemini JSON (usageMetadata), and BackendResponse::extract_usage reads only an OpenAI-shaped usage block, so non-streamed Gemini tool-loop turns record no usage.

## Why it matters

One settled record per attempt: cost and token figures for Gemini attempts are missing.

## Plan

Map usageMetadata (prompt, candidates, thoughts, cached) in the Gemini backend's response, reusing find-af6b7f's accounting.

## Done when

- [ ] Non-streamed Gemini turns record their usage
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `GeminiNativeBackend::send_turn` keeps the parsed response and, when it has `usageMetadata`, adds an OpenAI-shaped
  `usage` block (`translate::openai::usage_to_wire`) counted by `gemini::native::gemini_observation` (now
  `pub(crate)`), so `extract_usage` reads it: input without the cached tokens, thoughts as output and reasoning. A
  response without `usageMetadata` still reports none. Test: `a_non_streamed_gemini_turn_records_its_usage`.
