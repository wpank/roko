+++
id = "bug-f7f3bb"
kind = "bug"
title = "Under roko serve, status.json keeps showing a finished run as active, because the serve PID is still alive"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["roko-cli/runner/status_file"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "90307ad5e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report, checked on work/bug-4c4eea at 3cb4a818f)"
anchors = ["crates/roko-cli/src/runner/status_file.rs"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = ["bug-4c4eea"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_finished_run_is_not_active_under_serve' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_finished_run_is_not_active_under_serve"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in dc26f23c2. A terminal phase reads as RunnerStatusRead::Finished whatever the writer PID, so roko status shows a finished run as inactive under roko serve. Batch 17 gate: first run on 2c4abe35b (check clean; lib tests roko-agent 2271, roko-cli 3244 (gate_rows writer flake, fixed by bug-779ae7), roko-core 1955, roko-fs 260, roko-learn 1204, roko-serve 988), then re-gated on 53feea92e (same code as MAIN 90307ad5e) after the coordinator's doc-paragraph and rustfmt fix on guard2's branch (3f3a7be84): nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-learn -p roko-serve --keep-going -D warnings clean; roko-fs lib 260; --test secret_canary 11 passed; --test secrets_and_git_guard_canary 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

`.roko/state/status.json` treats a run as stale only when the file is older than 60 seconds and the writer's PID is dead (`crates/roko-cli/src/runner/status_file.rs:11`). Under `roko serve`, the writer is the serve process, which stays alive after a run finishes. So a finished run keeps showing as active.

## Why it matters

A record of how the backlog gets done (epic spec-f2463d): status views and dashboards show runs that ended as still running. p3.

## Where

The staleness rule and the writer in `status_file.rs`.

## Plan

1. Write an explicit terminal state (finished, failed, cancelled) when a run ends, and treat a terminal state as inactive whatever the PID. Or key liveness on a per-run id, not the PID.
2. Add `a_finished_run_is_not_active_under_serve`.

## Done when

- [ ] A run that finishes under serve shows as finished.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-5f0604` at `06679f0b1`; cargo verification deferred to the batch check.
- Plan option 1. `read_runner_status` returns the new `RunnerStatusRead::Finished` for a status whose phase is terminal
  (`completed`, `failed` or `cancelled`, the phases `GraphStatusWriter::finish` writes), before any PID or age check.
  `is_live()` is false for it, and the new `is_finished()` is true.
- `roko status` (`status.rs`) reports a finished run inactive with its terminal phase. Its runner line no longer appends
  ` (stale)`, which read `completed (stale)` for a finished run and doubled `stale/offline (was: …)` for a stale one.
  The TUI's standalone snapshot shows the terminal phase.
- `plan_runner.rs::a_graph_run_writes_status_json` asserted `is_live()` after the run (true then, since the test
  process wrote the file). It now asserts `is_finished()`.
- `a_finished_run_is_not_active_under_serve` (`status.rs`): this process writes a run's status and keeps running, as
  `roko serve` does. `roko status` reports the run active with phase `idle`, then after `finish` inactive with
  `completed`.
