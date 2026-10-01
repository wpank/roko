+++
id = "bug-60ccba"
kind = "bug"
title = "A snapshot's RunCompleted leaves the run's running tasks active with no end time"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard_snapshot"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "c7560e213"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-8a1fb3"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-8a1fb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_completed_ends_running_tasks' crates/roko-core/src/ && cargo test -p roko-core --lib run_completed_ends_running_tasks"
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
