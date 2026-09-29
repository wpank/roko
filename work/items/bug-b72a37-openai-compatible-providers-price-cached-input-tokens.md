+++
id = "bug-b72a37"
kind = "bug"
title = "OpenAI-compatible providers price cached input tokens twice"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["agent", "cost"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:03, wk-specs' report on gap-3c430e)"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs", "crates/roko-agent/src/streaming.rs", "crates/roko-core/src/chat_types.rs::Usage::fill_cost_from_pricing"]
lane = "rust-cold"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["gap-ad0d39", "gap-3c430e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn openai_compat_usage_prices_cached_input_once' crates/roko-agent/src/ && cargo test -p roko-agent --lib openai_compat_usage_prices_cached_input_once"
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
