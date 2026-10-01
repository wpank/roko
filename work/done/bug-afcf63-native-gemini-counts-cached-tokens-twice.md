+++
id = "bug-afcf63"
kind = "bug"
title = "Native Gemini counts cached tokens twice: promptTokenCount (which includes them) becomes input, and cachedContentTokenCount becomes cache reads"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/gemini"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "bf40f3269"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report, checked on work/bug-4c4eea at 3cb4a818f)"
anchors = ["crates/roko-agent/src/gemini/native.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-b72a37", "bug-a5f181"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn gemini_native_usage_counts_cached_tokens_once' crates/roko-agent/src/ && cargo test -p roko-agent --lib gemini_native_usage_counts_cached_tokens_once"

[closed]
at = 2026-10-01
by = "coordinator (session 7622b882)"
evidence = "Batch 20b gate on cad1a56e1 (MAIN bf40f3269 has the same crates): check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests roko-agent 2278, roko-cli 3261, roko-core 1956, roko-learn 1207, roko-gate 690, roko-std 227 and roko-cli bin 429 all pass, including gemini_native_usage_counts_cached_tokens_once. Merged 3dfdef519."
+++

## Problem

Gemini's `usageMetadata.promptTokenCount` includes the cached content. `cachedContentTokenCount` is the cached part of it. The native Gemini adapter (`crates/roko-agent/src/gemini/native.rs:556-569`, and :394) maps `prompt_token_count` to input tokens and `cached_content_token_count` to cache reads, so every cached token is counted twice: once as input at the full price, once as a cache read.

## Why it matters

One settled record per attempt (epic spec-b7303f): cost is overstated for every cached Gemini call, and S01 §4.4's token classes (disjoint: `tokens_in` is uncached input only) are violated.

## Where

The usage mapping in `gemini/native.rs`.

## Plan

1. Map input as `promptTokenCount − cachedContentTokenCount`, and cache reads as `cachedContentTokenCount`.
2. Add `gemini_native_usage_counts_cached_tokens_once`.

## Done when

- [ ] A cached native Gemini call records disjoint input and cache-read counts that sum to `promptTokenCount`.
- [ ] The `[[verify]]` command passes.

## Notes

- bug-b72a37 is the same class of error for OpenAI-compatible providers.
- Implemented on `work/bug-739dcc` at `ac35c8cf4`; cargo verification deferred to the batch check. `gemini_native_usage_counts_cached_tokens_once` and `streamed_gemini_usage_counts_cached_tokens_once` (targeted `cargo test` passed). Both native Gemini paths, the adapter's `gemini_observation` and the tool loop's streamed usage (`emit_accumulated_usage`, `tool_loop/backends/gemini_native.rs`), take input as `promptTokenCount` less `cachedContentTokenCount`. Related, not done: find-af6b7f (thinking tokens missing from output tokens).
