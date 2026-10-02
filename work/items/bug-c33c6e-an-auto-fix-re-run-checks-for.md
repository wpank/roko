+++
id = "bug-c33c6e"
kind = "bug"
title = "An auto-fix re-run checks for a stop before waiting for the compile lock, so a step can start after the stop"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-82cbef"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-82cbef"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib auto_fix_rerun_stops_after_the_lock_wait"
+++

## Problem

In the auto-fix re-run, the stop check (bug-82cbef) runs before `verify_step_locked` waits for the compile lock. A stop that begins during that wait still lets the step start. The main verify loop checks after the lock.

## Plan

Check for a stop again after the lock is taken, as the main loop does. Add a test named `auto_fix_rerun_stops_after_the_lock_wait`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-scheduler, working on bug-82cbef, during the evening close-out round.
- 2026-10-02 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  `verify_step_locked` takes a `stopped` check, which it runs once it holds the compile lock: a step whose plan run
  began to stop meanwhile does not start, and the call returns the run's cancellation, which the auto-fix re-run
  returns with `?`. The check before the lock wait stays, so a run that is already stopping does not wait. Test:
  `auto_fix_rerun_stops_after_the_lock_wait`.
