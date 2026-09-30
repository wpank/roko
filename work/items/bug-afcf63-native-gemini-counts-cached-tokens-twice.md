+++
id = "bug-afcf63"
kind = "bug"
title = "Native Gemini counts cached tokens twice: promptTokenCount (which includes them) becomes input, and cachedContentTokenCount becomes cache reads"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/gemini"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report, checked on work/bug-4c4eea at 3cb4a818f)"
anchors = ["crates/roko-agent/src/gemini/native.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-b72a37", "bug-a5f181"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn gemini_native_usage_counts_cached_tokens_once' crates/roko-agent/src/ && cargo test -p roko-agent --lib gemini_native_usage_counts_cached_tokens_once"
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
