+++
id = "bug-96341e"
kind = "bug"
title = "Legacy DELETE /api/jobs/{id} bypasses the shared JobExecutionService and never signals a running job"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-2a9ed7"
anchors = ["crates/roko-serve/src/routes/jobs.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-2a9ed7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn legacy_job_delete_signals_running_job' crates/roko-serve/src/routes/jobs.rs && cargo test -p roko-serve --lib legacy_job_delete_signals"
+++

## Problem

gap-2a9ed7 made job cancellation go through one shared `JobExecutionService`. The legacy `DELETE /api/jobs/{id}` route still bypasses it, so it sends a running job no signal. Only the runner's re-read keeps the status right.

## Plan

Route the legacy delete through the shared service, or retire it. Add a test named `legacy_job_delete_signals_*`.

## Done when

- `cargo test -p roko-serve --lib legacy_job_delete_signals` passes.

## Notes

- Reported on 2026-10-01 by wk-serve2, working on gap-2a9ed7, during the evening close-out round.
- 2026-10-01 (wk-serve2): implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  `DELETE /api/jobs/{id}` (`cancel_job`) and `POST /api/jobs/{id}/cancel` now share `cancel_through_service`, which
  cancels through `AppState.job_execution`, so a running job's executor is signalled; DELETE still answers with the
  job alone and the same 422 for a terminal job. The now-unused `is_terminal`/`TERMINAL_STATUSES` are gone. The file
  is persisted by `FileJobStore::save` (`status` key, legacy `state` cleared), so the lifecycle test that read
  `state` from disk after a DELETE now reads `status`. Test: `legacy_job_delete_signals_running_job`.
