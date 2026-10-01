+++
id = "bug-b72a37"
kind = "bug"
title = "OpenAI-compatible providers price cached input tokens twice"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["agent", "cost"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "27deb61d8"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:03, wk-specs' report on gap-3c430e)"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs", "crates/roko-agent/src/streaming.rs", "crates/roko-core/src/chat_types.rs::Usage::fill_cost_from_pricing"]
lane = "rust-cold"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["gap-ad0d39", "gap-3c430e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn openai_compat_usage_prices_cached_input_once' crates/roko-agent/src/ && cargo test -p roko-agent --lib openai_compat_usage_prices_cached_input_once"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in f16908be6. The OpenAI usage parse takes cached tokens out of prompt_tokens and the shared usage_to_wire writes prompt_tokens = input + cached, so cached input is priced once on the OpenAI-compatible, Codex and Hermes paths (200 uncached + 800 cached now costs $0.00048, was $0.00128). Batch 16a gate on 4169ecaba, re-assembled as 22f4a8791 with only runstate's rustfmt commit (MAIN 27deb61d8 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core --keep-going -D warnings clean; lib tests pass: roko-agent 2270, roko-cli 3204 (one background-writer wait flake, gate_rows_carry_the_attempts_turns_or_unknown, passes alone in 1.2 s), roko-core 1953. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

On OpenAI-compatible providers, `openai_compat_backend.rs` and `streaming.rs` put `prompt_tokens` into `Usage.input_tokens`, and `prompt_tokens` already includes the cached tokens. `Usage::fill_cost_from_pricing`, called through the tool loop, then prices those same tokens again as `cache_read_tokens`. Cached input is billed twice (wk-specs, 2026-09-29).

## Why it matters

Cost per verified success is the headline metric of the thesis, and this error inflates cost exactly where caching saves money. S01 v1.2 makes the token classes disjoint: `tokens_in` counts uncached input only. Epic spec-b7303f.

## Where

- The usage mapping in `crates/roko-agent/src/openai_compat_backend.rs` and `crates/roko-agent/src/streaming.rs`.
- `Usage::fill_cost_from_pricing` in `crates/roko-core/src/chat_types.rs`.

## Current state

Cached input is counted in both input and cache-read.

## Plan

1. Map `prompt_tokens − cached_tokens` to `input_tokens`, and `cached_tokens` to `cache_read_tokens`. Do this in both the streaming and non-streaming paths.
2. Check that the Anthropic path already keeps the classes disjoint.
3. Add the test `openai_compat_usage_prices_cached_input_once`.

## Done when

- [ ] Cached input is priced once, on both paths.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-4c4eea` at `f1c4fcece`; cargo verification deferred to the batch check.
- Both paths went through `translate/openai.rs::parse_usage_observation`, which now subtracts the cached tokens from the
  prompt tokens. The streaming path parses usage twice: once per SSE chunk, then again after
  `tool_loop::collect_stream_to_response` re-encodes it as an OpenAI JSON response. So every usage block roko writes
  itself now keeps the wire's rule, `prompt_tokens` = input + cached. That covers the new
  `translate::openai::usage_to_wire` (the tool loop's re-encode, `OpenAiCompatLlmBackend` and `CursorAgent`) and
  Anthropic's `normalize_usage`. The re-encode also keeps cache-creation and reasoning tokens, which it used to drop, so
  Anthropic's streamed turns now price their cache writes.
- Anthropic's classes were already disjoint (its `input_tokens` leave out cache reads). They round-trip unchanged.
- The same bug was in two adapters that parse OpenAI usage themselves, and both are fixed. `codex_agent.rs` subtracts the
  cached tokens; `hermes/http_adapter.rs` uses the shared parser for inline and run-lookup usage.
- Updated assertions (they expected the double count): `streaming.rs` `sse_parser_reads_usage`, `translate/mod.rs`
  `backend_response_extract_usage_from_openai_json`, `openai_compat_backend.rs`
  `streaming_tool_loop_emits_chunks_and_matches_final_result`, `tool_loop/mod.rs` `collect_stream_usage_is_preserved`,
  `tests/tool_loop_integration.rs` and the Hermes usage test.
- Not fixed, to file: `gemini/native.rs::gemini_observation` maps `promptTokenCount` (which includes cached content) to
  input and `cachedContentTokenCount` to cache reads, the same double count on the native Gemini path. Also,
  `testutil.rs::response_from_stream_events` encodes cache reads as `cache_read_input_tokens`, which the OpenAI parser
  never reads, and that is why the three streaming parity tests are ignored for a usage mismatch. Encoding with
  `usage_to_wire` would likely let them run again.
