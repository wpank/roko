+++
id = "bug-60ccba"
kind = "bug"
title = "A snapshot's RunCompleted leaves the run's running tasks active with no end time"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard_snapshot"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-8a1fb3"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-8a1fb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_completed_ends_running_tasks' crates/roko-core/src/ && cargo test -p roko-core --lib run_completed_ends_running_tasks"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:39:00Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:25Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

A snapshot's RunCompleted ends the running plans' clocks, but the run's running tasks stay active, with no end time.

## Plan

End them too, as interrupted or unknown, never passed. Add a test named `run_completed_ends_running_tasks`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-streams, working on gap-8a1fb3, during the evening close-out round.
- 2026-10-02 (wk-streams): implemented on work/gap-b35a57; cargo verification deferred to the batch check.
  `DashboardSnapshot::apply_with_ts` now ends, on `RunCompleted`, every task without an outcome: `cancelled` when the
  run was cancelled, otherwise the new `TASK_OUTCOME_INTERRUPTED` ("interrupted"), with `finished_at_ms` at the run's
  end. They leave `tasks_active` and count as failed: `classify_task_outcome` now reads "interrupt" as a failure
  word (no production path published that outcome before). Test: `run_completed_ends_running_tasks`.
  The portal classifies outcomes itself and may show "interrupted" as an unknown outcome until it learns the word.
