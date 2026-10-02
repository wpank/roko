+++
id = "bug-6052d8"
kind = "bug"
title = "roko chat's own CLI invocation still sends inert cache markers"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/dispatch_v2"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "find-6ee709"
anchors = ["crates/roko-cli/src/dispatch_v2.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["find-6ee709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib chat_strips_cache_markers"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:13Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:22Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

find-6ee709 strips cache markers from system prompts in `create_agent_for_model` for every provider except the Anthropic API. `roko chat` builds its own CLI invocation in `dispatch_v2`, so it still sends the markers as inert text.

## Plan

Apply the same stripping on the dispatch_v2 path, and add a test named `chat_strips_cache_markers`.

## Done when

- `cargo test -p roko-cli --lib chat_strips_cache_markers` passes.

## Notes

- Reported on 2026-10-01 by the worker on find-6ee709, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `CliProviderConfig::build_invocation` (dispatch_v2.rs) strips the cache markers from the request's system
  prompt before the per-protocol builders see it, with `translate::claude::strip_cache_markers` (now `pub`).
  Test: `chat_strips_cache_markers`.
