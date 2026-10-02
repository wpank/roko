+++
id = "bug-ea9959"
kind = "bug"
title = "Flaky under load: a_verify_failure_left_in_a_sibling_file_blames_the_sibling lets its fake sibling finish before settling starts"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/tests", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/graph-ready-queue 3e7552acd"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::dispatch_beside_editing_sibling", "crates/roko-cli/src/graph_task_dispatch/verification.rs::a_verify_failure_left_in_a_sibling_file_blames_the_sibling", "crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::begin_settle"]
links = { depends_on = [], blocks = [], related = ["gap-439794", "bug-730243"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'while !failed_once.exists()' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib a_verify_failure_left_in_a_sibling_file_blames_the_sibling && cargo test -p roko-cli --lib a_verify_failure_beside_an_editing_sibling_is_rerun_once_it_settles"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:46:00Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:09Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Already fixed at BASE ebdc0f5d5; the item's 2026-10-01 note gives the evidence."
+++

## Problem

`graph_task_dispatch::tests::a_verify_failure_left_in_a_sibling_file_blames_the_sibling` passes alone and
sometimes fails in a loaded full `cargo test -p roko-cli --lib` run. The 3e7552acd commit message records it:
"1 unrelated timing flake (graph_task_dispatch sibling-blame test, passes 5/5 alone)". When it fails, the verify
error does not start with `blocked_by_sibling = T12:`.

## Why it matters

A red full-suite run that passes on retry hides real failures and costs a rerun of the whole roko-cli lib suite.
The pre-commit checks in CLAUDE.md run that suite.

## Where

- The helper `dispatch_beside_editing_sibling` (`graph_task_dispatch.rs:6033-6060`) registers a fake sibling `T12`
  in `dispatcher.in_flight`, then runs `dispatch` beside a future that polls `failed_once.exists()` every 20 ms.
  When it sees the file, it writes `sibling_done` and drops the sibling's registration.
- The test's verify step creates `failed_once` itself, as its first command (`touch …; echo '<tsc error>' >&2; exit 2`).
- The dispatcher looks for siblings only after the step's process has exited and its verdict is built:
  `settle_failed_step` → `begin_settle` (`sibling_settle.rs:205`), called from `settle_task_verification`
  (`graph_task_dispatch.rs:2155`).

## Current state

Checked at 33e107da1. The race: the `touch` runs, and the rest of the step (echo, exit, process reaping, verdict)
has to finish before `begin_settle` runs. If the sibling's next 20 ms poll lands in that window, the sibling has
already dropped its registration. `begin_settle` then finds no writers, the failure stands unsettled and unblamed,
and the assertion fails. Under load the window grows. The companion test
`a_verify_failure_beside_an_editing_sibling_is_rerun_once_it_settles` uses the same helper and has the same race:
with no writer, nothing re-runs the step and its `expect` fails.

## Plan

1. Make the fake sibling finish only after the dispatcher has started settling. Wait until the task's in-flight
   attempt is marked `settling`: add a `#[cfg(test)]` accessor on `InFlightTasks`, or wait on its `changed` watch.
   Do not key the sibling's finish on a file the verify step writes.
2. Keep both tests' assertions as they are.
3. Check that each test passes 20 times in a row under a parallel full lib run.

## Done when

- `dispatch_beside_editing_sibling` no longer polls `failed_once`, and both sibling tests pass reliably under load.
- The `[[verify]]` command passes.

## Notes

`graph_task_dispatch::tests::verified_outcome_drives_output_verdict_and_feedback`, in the same module, is a separate
load flake reported from the same batch. Keep the two fixes apart.

- 2026-10-01 (wk-honestbench): already fixed at BASE `ebdc0f5d5`, by `2c61e9053` (2026-09-30, for bug-779ae7). The
  helper moved to `crates/roko-cli/src/graph_task_dispatch/verification.rs:1752`, and its fake sibling now waits on
  `InFlightTasks::settling_began` (`#[cfg(test)]`, `sibling_settle.rs:394`, woken by the `changed` watch) before it
  finishes (:1769). It no longer polls `failed_once`, which is only asserted to exist (:1770). The `[[verify]]`
  grep finds no `while !failed_once.exists()`, and both tests (:1793, :1829) keep their assertions.
