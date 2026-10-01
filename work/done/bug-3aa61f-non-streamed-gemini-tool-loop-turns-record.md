+++
id = "bug-3aa61f"
kind = "bug"
title = "Non-streamed Gemini tool-loop turns record no usage: send_turn returns raw Gemini JSON and extract_usage reads only an OpenAI-shaped usage block"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on bug-2b1ddc/find-af6b7f/bug-b9cb83)"
anchors = ["crates/roko-agent/src/tool_loop/backends/gemini_native.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["find-af6b7f", "bug-afcf63"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_non_streamed_gemini_turn_records_its_usage' crates/roko-agent/src/ && cargo test -p roko-agent --lib a_non_streamed_gemini_turn_records_its_usage"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:13Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:14Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
