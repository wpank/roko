+++
id = "bug-151964"
kind = "bug"
title = "Tasks replayed on resume publish only task_completed, so the dashboard, portal and snapshot never count them"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph-execution", "roko-core/dashboard", "apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d5c1dc6be"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "session:roko-b6 2026-09-29 portal close-out"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::poll_status_changes", "crates/roko-graph/src/engine.rs:1208", "crates/roko-core/src/dashboard_snapshot.rs::apply_with_ts", "apps/portal/src/lib/runState.ts:723", "crates/roko-cli/src/graph_execution/plan_runner.rs::PLAN_WATCH_INTERVAL"]
links = { depends_on = [], blocks = [], related = ["gap-f59fe9", "bug-7e1b6b", "gap-8a1fb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn replayed_task_counts_as_done' crates/roko-cli/src && cargo test -p roko-cli --lib replayed_task_counts_as_done"
+++

## Problem

When a plan resumes from a checkpoint (`roko plan run --resume-plan`, or any run that finds a matching checkpoint),
the tasks that passed before are replayed, not run. They reach the event stream as `task_completed` with no
`task_started` before them. The StateHub snapshot, the TUI and the portal count a task only after they have seen it
start, so replayed tasks are neither listed nor counted as done. `plan_started` still announces every task, so a
five-task plan resumed after three tasks passed ends showing 2/5 done, although the plan succeeded.

## Why it matters

Goal `visibility`: resume is the normal recovery path, and after it the live views under-report finished work
exactly when someone is checking what is left.

## Where

- `crates/roko-graph/src/engine.rs:1208-1246`: an Activity node with a recorded output is set straight to
  `NodeStatus::Complete`, never `Running` (the replayer is attached with `GraphEngine::with_replayer`).
- `crates/roko-cli/src/runner/graph_tui_bridge.rs::poll_status_changes` (:215-258): emits `TaskStarted` only for a
  transition into `Running`, and `TaskCompleted` for a transition into a terminal status. A node first seen as
  `Complete` gets only `TaskCompleted`. The plan runner calls it every `PLAN_WATCH_INTERVAL` (100 ms,
  `plan_runner.rs:1688`) and once more against the final result (:2250-2262).
- `crates/roko-core/src/dashboard_snapshot.rs::apply_with_ts`: the `TaskCompleted` arm (:1614-1650) updates counters
  only when `self.tasks` already holds the task (inserted by `TaskStarted`, :1574). For an unknown task,
  `newly_terminal` stays false and nothing is counted.
- `apps/portal/src/lib/runState.ts:723-728`: the portal reducer ignores `task_completed` for unknown tasks.
- `plan_runner.rs:2106-2107`: `plan_started` carries all tasks, replayed ones included.

## Current state

Checked at `d5c1dc6be` by reading the code; not reproduced live. The same gap hits any live task that starts and
finishes between two 100 ms polls: it too goes from unseen to terminal. gap-f59fe9 is the same drop for blocked tasks
that never start (Pending→Skipped).

## Plan

1. In `poll_status_changes`, when a node reaches `Complete` or `Failed` and its previous status was not `Running`
   (absent or `Pending`), emit `TaskStarted` (title from `node_titles`) just before its `TaskCompleted`. Every
   consumer then sees a normal start/complete pair. Optionally pass the checkpoint's replayed node ids so that
   replayed tasks start with phase `replayed`.
2. Leave `Skipped` and `ConditionSkipped` to gap-f59fe9, which is designing a blocked-task event for them.
3. Alternative: let `DashboardSnapshot` and `runState.ts` insert an unknown task on `task_completed`. That fixes
   every producer at once, but consumers of the raw event stream still see a completion with no start. gap-f59fe9
   weighs the same option. Choose once for both items.
4. Add `replayed_task_counts_as_done` (roko-cli lib): feed `poll_status_changes` a node that goes from absent to
   `Complete`, apply the published events to a `DashboardSnapshot`, and assert that the task is listed and that the
   plan's `tasks_done` counts it.

## Done when

- After a resumed mock run where T1 is replayed and T2 runs, the snapshot lists both tasks and counts 2 done.
- The `[[verify]]` command passes.

## Notes

Coordinate with gap-f59fe9 and bug-7e1b6b, which change the same bridge and snapshot code. bug-7e1b6b counts
`skipped` as passed today.
