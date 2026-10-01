+++
id = "bug-ceb581"
kind = "bug"
title = "TaskExecutorCell starts another attempt after SIGINT: its retry loop never checks cancellation"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-graph/cells", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-verify-loops ea5cb31db"
anchors = ["crates/roko-graph/src/cells/task_executor.rs::TaskExecutorCell", "crates/roko-graph/src/cell.rs::CellContext::is_cancelled", "crates/roko-graph/src/engine.rs::FlowHandle::cancel", "crates/roko-graph/src/engine.rs::execute_ready_queue", "crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents", "crates/roko-cli/src/graph_execution/plan_runner.rs:2074"]
links = { depends_on = [], blocks = [], related = ["gap-b367bf", "bug-4641e3", "gap-09f17a", "bug-2b1ddc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cancelled_run_starts_no_further_attempt' crates/roko-graph/src && cargo test -p roko-graph --lib a_cancelled_run_starts_no_further_attempt && grep -rqw 'fn flow_cancel_reaches_a_running_cell' crates/roko-graph/src && cargo test -p roko-graph --lib flow_cancel_reaches_a_running_cell"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:26Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:15Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

After SIGINT (or SIGTERM, or closing the TUI) a plan run can start a new agent attempt for a task that was
running. On an interrupt the plan runner cancels the graph and SIGTERMs the in-flight agents
(`plan_runner.rs:2160-2179`). The killed agent makes `GraphTaskDispatcher::dispatch` return an error.
`TaskExecutorCell` treats that like any other failed attempt: while `retry < max_retries` it calls `dispatch`
again (`task_executor.rs:658-690`). The only error it does not retry is a non-retryable `Gateway` error
(provider exhaustion, `graph_task_dispatch.rs:5399-5401`). The new attempt reserves budget, builds a prompt and
spawns a new agent. It runs until the 3 s drain deadline (`INTERRUPT_DRAIN_TIMEOUT`, `plan_runner.rs:200`), when
`kill_in_flight_agents` SIGKILLs it and the flow is abandoned.

Expected: once the run is cancelled, a running task finishes or fails its current attempt, and no further attempt
starts.

## Why it matters

Goal `core`. An interrupt should stop spending. Today it can buy one more provider call per running task. That call
is killed a few seconds later, and it also advances the task's attempt numbering, which continues across a resume
since `ea5cb31db`. Related items:
- `gap-b367bf`: an interrupt does not signal a running gate command.
- `bug-4641e3`: forced exit and SIGHUP skip checkpoint finalization.
- `gap-09f17a` (parked): the same retry loop has no backoff.

## Where

- `crates/roko-graph/src/cells/task_executor.rs::TaskExecutorCell`: `execute`, the `Live` arm's retry loop
  (:658-690). It never checks `ctx.is_cancelled()`.
- `crates/roko-graph/src/cell.rs::CellContext`: `cancel_flag` (:81-85) and `is_cancelled` (:217) exist for
  cooperative shutdown (#255). Only `control_adapter.rs` tests set a flag (:627-638).
- `crates/roko-graph/src/engine.rs`:
  - `FlowHandle::cancel` (:267-272) cancels a `CancellationToken` that is documented as "nodes already running
    are not interrupted";
  - `execute_ready_queue` (:1089) checks the token only before it starts nodes (:1129-1134), and hands each node
    a clone of the caller's `CellContext` (:1272), which carries no cancel flag.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - builds the plan's `CellContext` with a run id and pause flag but no cancel flag (:2074-2076);
  - its watch loop calls `flow_handle.cancel()` and `terminate_in_flight_agents()` on interrupt (:2160-2179).
- `crates/roko-cli/src/graph_task_dispatch.rs`: `GraphTaskDispatcher::dispatch` (:3447) starts with the budget
  reservation and has no cancellation check either.

## Current state

Checked at `33e107da1`, by reading the code. Plan graphs run with `SkipFailed` or `FailFast`, so the engine's own
retry helper (`execute_cell_with_retries`, engine.rs:2785) retries 0 times for them (`max_retries`, :2777-2782).
The task's retry budget lives in `TaskExecutorCell`, so that loop is the one that re-dispatches. Not reproduced live.

## Plan

1. Make the engine's cancellation visible to running cells. When `start()` / `execute_ready_queue` hand a node its
   `CellContext`, attach a cancel flag that `FlowHandle::cancel` sets. `CellContext::with_cancel_flag` exists. A
   caller-supplied flag must keep working.
2. In `TaskExecutorCell::execute`, return the error without retrying when `ctx.is_cancelled()`, both before each
   retry and before the first attempt. Log that the task stopped because the run was cancelled.
3. Optionally, make `GraphTaskDispatcher::dispatch` refuse to start when `ctx.is_cancelled()`, before it reserves
   budget.
4. Tests in `roko-graph`:
   - `a_cancelled_run_starts_no_further_attempt`: a dispatcher that sets the flag and fails on its first call is
     called exactly once;
   - `flow_cancel_reaches_a_running_cell`: `FlowHandle::cancel` during a running cell makes that cell's
     `ctx.is_cancelled()` true.

## Done when

- After an interrupt, no task starts a new attempt. A running attempt ends, and the task is recorded as
  interrupted or failed without a retry.
- The `[[verify]]` command passes.
- Manual check: `roko plan run` a plan whose fake agent sleeps. Press Ctrl-C during the first attempt. The run log
  shows no second "TaskExecutorCell provider dispatch failed; retrying" line, and no second agent process starts.

## Notes

- Keep the drain and kill behaviour in `plan_runner.rs`. This item only stops new attempts.
- Also check whether the SIGTERMed attempt is recorded as an ordinary failed attempt in `.roko/learn/costs.jsonl`
  and `.roko/episodes.jsonl`. If so, learning counts interrupts as failures. File that as its own item if it
  holds.
- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  `CellContext` gains `run_cancel`, the graph run's `CancellationToken`. `GraphEngine::start` attaches it, so
  `FlowHandle::cancel` reaches running cells, and `is_cancelled` is true once either it or the caller's cancel flag
  is set. The plan runner's stop flag keeps its meaning: run_watched still drops calls only at the drain deadline.
  `TaskExecutorCell` checks `is_cancelled` before each attempt (a `Cancelled` error) and after a failed one (no retry).
  Tests: `a_cancelled_run_starts_no_further_attempt`, `flow_cancel_reaches_a_running_cell`. The Notes' learning
  question is addressed by bug-28b604 on this branch: once the run began to stop, a SIGTERMed attempt settles as
  cancelled, which teaches nothing.
