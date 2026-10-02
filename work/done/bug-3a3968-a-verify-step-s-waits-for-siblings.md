+++
id = "bug-3a3968"
kind = "bug"
title = "A verify step's waits for siblings and for the compile lock don't watch for a stop"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-c33c6e"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-c33c6e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib verify_waits_end_on_a_stop"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:29Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:41:48Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

The verify loop waits before a step: the sibling wait in `begin_step`, and the compile-lock wait. Neither watches for a stop. Behind another process's long cargo run, an attempt can outlast the 3 s drain and 2 s settle; its flow is then abandoned, and the attempt never gets a verdict.

## Plan

Race both waits against the stop signal, and settle a stopped wait as cancelled. Add a test named `verify_waits_end_on_a_stop`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-scheduler, working on bug-c33c6e, during the overnight close-out round.
- 2026-10-02 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  The dispatcher's stop flag is now a `CancellationToken`, which `begin_stop` cancels. `unless_stopped` races a wait
  against it, and the verify loop runs both of a step's waits through it: the sibling wait in `begin_step` and the
  compile-lock wait, in the first run and in the auto-fix re-run (whose `verify_step_locked` now takes the token).
  The re-run of a step after its siblings settle does the same for its compile-lock wait. A stopped wait ends the
  verify with `RokoError::Cancelled`, which settles the attempt as cancelled (bug-82cbef). Both waits drop cleanly:
  the reading guard is made before the sibling wait, and the semaphore and slot waits hold nothing until they win.
  Test: `verify_waits_end_on_a_stop`, a sibling case and a compile-lock case.
