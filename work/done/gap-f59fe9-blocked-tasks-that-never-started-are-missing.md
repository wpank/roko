+++
id = "gap-f59fe9"
kind = "gap"
title = "Blocked tasks that never started are missing from the connected dashboard's task list"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-core/dashboard", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "8a3c530af"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/graph-ready-queue 9ef6f4aad"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::apply_with_ts", "crates/roko-cli/src/runner/graph_tui_bridge.rs::poll_status_changes", "crates/roko-cli/src/graph_execution/plan_runner.rs::task_outcomes", "crates/roko-cli/src/tui/state/snapshot.rs::update_from_dashboard_snapshot", "apps/portal/src/lib/runState.ts:705"]
links = { depends_on = [], blocks = [], related = ["bug-7e1b6b", "gap-4d835d", "gap-bfd447"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_task_blocked_before_it_started_is_listed' crates/roko-core/src && cargo test -p roko-core --lib a_task_blocked_before_it_started_is_listed && grep -rqw 'fn update_from_dashboard_snapshot_lists_blocked_tasks' crates/roko-cli/src && cargo test -p roko-cli --lib update_from_dashboard_snapshot_lists_blocked_tasks"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T15:16:14Z"
by = "coordinator (session 7622b882)"
size = "M"
claimed_at = "2026-10-01T09:06:31Z"
forced = false
evidence = "Batch 20f gate on 2ff1b7891 (MAIN has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/core/graph/serve; lib tests roko-cli 3309, roko-agent 2296, roko-core 1963, roko-serve 992, roko-graph 480 pass; extras: all eight canaries + golden_path_suite + secret_canary 11/11 + C2 2/2 + worktree_task_diff + default_engine pass, bin 429, graph_task_dispatch suite at --test-threads=32 passed 10 of 10, including a_task_blocked_before_it_started_is_listed and a_failed_task_blocks_only_its_dependants; portal tsc clean and vitest 797/797 (wk-runstate). Merged (work/reg-cbfff6 fc2430b99)."
+++

## Problem

Since 3e7552acd and 9ef6f4aad, a failed plan task skips only its own dependants, so a plan can end with tasks that
never started. For example, T4 is blocked by the failed T1. The dashboard state model keeps only tasks that sent
`TaskStarted`. A blocked task never starts, so it never appears in the task list. In the TUI connected to a live
run, a five-task plan whose T1 failed and blocked T4 and T5 counts five tasks but lists only three rows. No row
says that T4 was blocked by T1. The only trace is a `graph.task_blocked` line in the event log.

## Why it matters

Goal `visibility`. With skip-failed as the default, blocked tasks are a normal result, and the task list is where a
person looks to see what did not run and why. `roko diagnose` and the checkpoint record it
(`roko.task.outcome@1`); the live views do not.

## Where

- `crates/roko-cli/src/runner/graph_tui_bridge.rs::poll_status_changes` (:215-258): a node that goes
  Pending→Skipped emits only `TaskCompleted { outcome: "skipped" }` (`node_completed`), with no `TaskStarted` before it.
- `crates/roko-core/src/dashboard_snapshot.rs::apply_with_ts`: the `TaskCompleted` arm (:1584-1620) updates
  `self.tasks.get_mut(&key)` only, so an event for an unknown task is dropped. Tasks enter the map only in the
  `TaskStarted` arm (:1544).
- `crates/roko-cli/src/tui/state/snapshot.rs::update_from_dashboard_snapshot` (:480): in connected mode, each plan's
  task rows are built from `snap.tasks` (:564-605).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::task_outcomes` (:2358): already knows each blocked task and
  its blocker, and each unstarted task and its reason. Today it only logs them (:2264-2283).
- The portal's run state has the same rule: `apps/portal/src/lib/runState.ts:705` ignores `task_completed` for
  unknown tasks, so the portal shows such a task only as "This task has not started.", with no reason.

## Current state

Checked at 33e107da1 by reading the code above. The TUI already has a `TaskStatus::Blocked`
(`tui/state/mod.rs:233`). Note bug-7e1b6b: `classify_task_outcome` counts "skipped" as passed. Inserting blocked
tasks with outcome "skipped" would therefore inflate `tasks_done`.

## Plan

1. Publish blocked and not-started tasks with their reason. Either add a `DashboardEvent` (for example
   `TaskBlocked { plan_id, task_id, title, blocked_by, reason }`) sent from the loop in `plan_runner.rs` that logs
   `graph.task_blocked` / `graph.task_not_started`, or let `TaskCompleted` for an unknown task insert it. The new
   event is clearer, and it carries the title and blocker.
2. In `DashboardSnapshot`, insert such tasks with an outcome that counts as neither done nor failed. Map it to
   `TaskStatus::Blocked` in the TUI, with the blocker in the row.
3. Handle the same event in `apps/portal/src/lib/runState.ts` (and its SSE decoding), so the portal shows "blocked
   by T1".
4. Add `a_task_blocked_before_it_started_is_listed` (roko-core lib: the task is in `snap.tasks` with its blocker and
   is not counted as done) and `update_from_dashboard_snapshot_lists_blocked_tasks` (roko-cli lib: the plan's rows
   include it as Blocked).

## Done when

- After a plan run where T1 fails and T4 depends on it, the TUI's live task list shows T4 as blocked by T1, and
  the plan's counts do not include it as done.
- The `[[verify]]` command passes.

## Notes

Coordinate with bug-7e1b6b, which changes how skipped and unverified outcomes are classified in the same two files.

Implemented on `work/reg-cbfff6` at `ed4f990e9`; cargo verification deferred to the batch check. Portal: `tsc --noEmit` and vitest (80 files, 794 tests) pass.

Plan step 1 took the new event: `DashboardEvent::TaskBlocked { plan_id, task_id, title, blocked_by, reason }`. After the final status poll, `run_one_plan` publishes it for each entry of `task_outcomes.blocked_by`, with the blocker and the reason "blocked by failed task '<id>'". It also publishes it for each entry of `task_outcomes.not_started`, with no blocker and the node's skip reason (a fail-fast abort, or a dispatch stop such as a spent budget). Both show as blocked.
- Snapshot: the outcome and phase are `blocked` (`TASK_OUTCOME_BLOCKED`, `TaskOutcomeClass::Blocked`), and `TaskState` has `blocked_by` and `blocked_reason`. A blocked task counts as neither done nor failed: the event takes back what the poll's earlier `skipped` completion counted, and a repeat counts nothing. A later run's `TaskCompleted` counts again and clears the blocker.
- TUI: the task shows as `TaskStatus::Blocked`, with its blocker in `depends_on` of both the plan's entry and the checklist row.
- Serve: the runs route names the class `blocked`, the health counter knows the event, and the bridge carries it (bug-bfdb9a).
- Portal: `task_blocked` is folded into the run state. It shows as skipped, keeps `blockedBy` and `blockedReason`, counts as neither done nor failed, is not counted as a retry when it starts, and a resume runs it again. `fromSnapshot` carries the two fields.

Still open from plan step 3: no portal view says "blocked by T1" yet. The data is in `TaskRun`, but the portal has no blocked status, and a blocked task's row reads as skipped. Also, the bug-230de6 `EventLogWriter` summary counts outcomes from `TaskCompleted` only, so it counts a blocked task as skipped; the `task_blocked` line is in the log.

`6fc37262d` extends the plan-run test `a_failed_task_blocks_only_its_dependants`: after the run, the hub's snapshot lists T4 as blocked by T1, and the plan counts no skipped task for it. That covers the first Done-when on the real run path.
