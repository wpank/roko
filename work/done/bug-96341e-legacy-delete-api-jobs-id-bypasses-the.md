+++
id = "bug-96341e"
kind = "bug"
title = "Legacy DELETE /api/jobs/{id} bypasses the shared JobExecutionService and never signals a running job"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-2a9ed7"
anchors = ["crates/roko-serve/src/routes/jobs.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-2a9ed7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn legacy_job_delete_signals_running_job' crates/roko-serve/src/routes/jobs.rs && cargo test -p roko-serve --lib legacy_job_delete_signals"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:19Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:15:12Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
