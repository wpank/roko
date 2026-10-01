+++
id = "bug-53088d"
kind = "bug"
title = "A profile's default thinking budget can reach or exceed max_tokens (Opus 32768 vs a 16384 default max output); only request-level thinking is clamped"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-agent"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on bug-b9cb83)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-b9cb83"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_profile_thinking_budget_is_clamped_below_max_tokens' crates/roko-agent/src/ && cargo test -p roko-agent --lib the_profile_thinking_budget_is_clamped_below_max_tokens"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:16Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:14Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
