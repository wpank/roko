+++
id = "bug-3a3b0f"
kind = "bug"
title = "With both stall thresholds at 0 there is no AttemptProgress, so a stopped or cancelled call settles with unknown usage"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on bug-2b1ddc)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-aa2044", "bug-2b1ddc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn usage_is_tracked_with_the_watchdog_off' crates/roko-cli/src/ && cargo test -p roko-cli --lib usage_is_tracked_with_the_watchdog_off"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:13Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:00Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

AttemptProgress (bug-aa2044) exists only when the watchdog runs; with both stall thresholds at 0 a stopped or cancelled call settles with unknown usage.

## Why it matters

One settled record per attempt: the streamed usage is known but dropped.

## Plan

Track AttemptProgress independently of the watchdog thresholds.

## Done when

- [ ] Stopped calls keep their streamed usage with the watchdog off
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  Each attempt now gets an `AttemptProgress` whatever the stall thresholds are: the `StallWatch`'s when one runs,
  else a standalone one. Its live-output tap always runs, and `run_watched` takes the progress, marking it
  interrupted on a stop, a restart or a stall. With both thresholds at 0, a call the plan run stops still settles
  the usage it streamed, marked estimated. Both `dispatch` and `dispatch_streaming` do this. Test:
  `usage_is_tracked_with_the_watchdog_off`.
