+++
id = "gap-dd4826"
kind = "gap"
title = "TaskBlocked is not in the run event log's lifecycle set, so it is not written to disk immediately"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on gap-f59fe9)"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-f59fe9", "bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'TaskBlocked' crates/roko-cli/src/graph_execution/event_log.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:52Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:51Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

event_log.rs (about line 333) syncs lifecycle events immediately and appends the rest relaxed; the new TaskBlocked event is not in the lifecycle set.

## Why it matters

A crash right after a block can lose the event that explains the run's end state.

## Plan

Add TaskBlocked to the lifecycle set.

## Done when

- [ ] TaskBlocked is synced like other lifecycle events
- [ ] The `[[verify]]` command passes.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
The workspace event log's lifecycle set moved into `is_lifecycle_event` (`graph_execution/event_log.rs`) and now includes `TaskBlocked`, so a blocked task is synced to `.roko/events.jsonl` and flushes the run's index as it is written. Test: `workspace_event_log_syncs_a_blocked_task`.
