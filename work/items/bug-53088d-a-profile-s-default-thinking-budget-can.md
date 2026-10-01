+++
id = "bug-53088d"
kind = "bug"
title = "A profile's default thinking budget can reach or exceed max_tokens (Opus 32768 vs a 16384 default max output); only request-level thinking is clamped"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-agent"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on bug-b9cb83)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-b9cb83"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_profile_thinking_budget_is_clamped_below_max_tokens' crates/roko-agent/src/ && cargo test -p roko-agent --lib the_profile_thinking_budget_is_clamped_below_max_tokens"
+++

## Problem

With no request-level thinking setting, the profile's default budget (Opus 32768, Sonnet 16384) can reach or exceed max_tokens when max_output is unset (DEFAULT_MAX_OUTPUT_TOKENS 16384). The API rejects that. Only the request-level path clamps.

## Why it matters

Calls fail outright for a default configuration.

## Plan

Apply bug-b9cb83's clamp (at least 1024, below max_tokens) to the profile default too.

## Done when

- [ ] Profile thinking budgets are clamped like request-level ones
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  The budget is chosen in `provider/anthropic_api/tool_loop.rs::thinking_budget`, not in the anchored
  `tool_loop/mod.rs`. A profile's default budget now goes through the same clamp as a requested one (at least 1024,
  below `max_tokens`, no thinking when that does not fit). Test: `the_profile_thinking_budget_is_clamped_below_max_tokens`.
