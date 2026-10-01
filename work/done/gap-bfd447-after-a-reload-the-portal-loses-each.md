+++
id = "gap-bfd447"
kind = "gap"
title = "After a reload the portal loses each plan's times, cost and accepted count because the snapshot's plan state lacks them"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["roko-core/dashboard", "apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "3fb11e23e"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::PlanDisplayState", "apps/portal/src/lib/runState.ts:1060"]
links = { depends_on = [], blocks = [], related = ["bug-7e1b6b", "gap-4171e8", "gap-082a14", "bug-9f340c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_display_state_carries_times_cost_and_accepted' crates/roko-core/ && cargo test -p roko-core plan_display_state_carries_times_cost_and_accepted"

[[verify]]
command = "grep -rqF --include='*.test.ts' 'keeps plan times, cost and accepted counts' apps/portal/src && (cd apps/portal && npx vitest run src/lib/runState)"

[closed]
at = 2026-09-29
commit = "3fb11e23e"
evidence = "roko-core PlanDisplayState now carries started_at_ms, finished_at_ms, cost_usd and tasks_accepted_with_failures, and portal fromSnapshot maps them. Both [[verify]] commands pass at 3fb11e23e: cargo test -p roko-core plan_display_state_carries_times_cost_and_accepted, and npx vitest run src/lib/runState (runState.reload.test.ts: rail shows actual/elapsed time and amber, status line keeps the cost). roko-serve statehub_snapshot_carries_each_plans_times_cost_and_accepted_count checks GET /api/statehub/snapshot, and a fake-agent server run of live-b served started/finished/cost_usd in the snapshot. Left: a plan that ran before a server restart has no snapshot record, so PlanView still offers Run (gap-082a14, bug-9f340c)."
+++

## Problem

Every page load rebuilds the run state from `GET /api/statehub/snapshot`. A plan there is a
`PlanDisplayState` holding only `plan_id, phase, tasks_total, tasks_done, tasks_failed, active`
(`dashboard_snapshot.rs:487`). `fromSnapshot` therefore sets `tasksAccepted: 0, startedAtMs: null,
finishedAtMs: null, costUsd: 0, title: null` for every plan (`runState.ts:1060-1075`). After a
reload:

- A finished plan's rail time falls back to its `~` estimate, or blank. Design §3 wants the actual
  time once finished. A running plan's rail time is blank.
- Per-plan cost is 0, so the status line's `$` disappears (it is hidden at $0).
- An `accepted_with_failures` plan turns green, against design §6 rule 1 ("green means verified").
- A plan that ran before a server restart has no live record. It shows ▶ Run, not ▶ Run again, and
  its tasks read as pending ("This task has not started."): Graph runs never write task status back
  (gap-082a14), and `task_to_dto` counts only "done" as completed (bug-9f340c).

## Why it matters

Goal `visibility`. Reload is a checked step of the 09 flow. It passed on selection and session only;
these fields were not checked.

## Where

`crates/roko-core/src/dashboard_snapshot.rs::PlanDisplayState`, and the plan loop of `fromSnapshot`
in `apps/portal/src/lib/runState.ts`.

## Plan

Add started and finished timestamps, cost and the accepted count to `PlanDisplayState`, fed by the
events that already carry them, and map them in `fromSnapshot`. For plans absent from the snapshot,
use the Graph checkpoint's status through the plan API.

## Done when

A roko-core test `plan_display_state_carries_times_cost_and_accepted` and a portal test whose name
contains "keeps plan times, cost and accepted counts" pass. Both `[[verify]]` commands run them.

## Notes

Related: bug-7e1b6b (skipped and unverified tasks counted as passed), gap-4171e8 (the snapshot keeps
only the latest gate output per task) and gap-8a1fb3 (event timestamps). Found by plan 09 T04 (see
VERDICT).
