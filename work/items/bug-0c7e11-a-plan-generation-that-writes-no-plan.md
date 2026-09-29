+++
id = "bug-0c7e11"
kind = "bug"
title = "A plan generation that writes no plan still ends its operation completed"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "session:roko-b6 2026-09-29 portal close-out"
anchors = ["crates/roko-serve/src/routes/plans.rs::generate_plan", "crates/roko-serve/tests/plan_authoring.rs::get_operation_reports_failed_when_generation_writes_no_plan"]
links = { depends_on = [], blocks = [], related = ["gap-a6e2c3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn get_operation_reports_failed_when_generation_writes_no_plan' crates/roko-serve/tests/plan_authoring.rs && cargo test -p roko-serve --test plan_authoring get_operation_reports_"
+++

## Problem

`POST /api/plans/generate` runs generation as a background operation. When the runtime returned no plan
targets, the task set the operation to `Completed` and published `plan_generate.failed` at the same time.
When the runtime returned targets but plan `<slug>` did not load, the operation completed with
`task_count = 0`. Either way `GET /api/operations/{id}` answered `completed`, and the portal
(`apps/portal/src/lib/operation.ts::waitForOperation`) took the slug and opened a plan that does not exist
(`GET /api/plans/{slug}` is 404).

Expected: the operation completes only when the plan it names can be opened. Otherwise it ends `failed`
with an error naming the plan, and the portal shows that error.

## Why it matters

Goal `visibility`: the portal's generate flow trusts the operation status. Related: gap-a6e2c3 (generation
spend) changed the same flow.

## Where

`crates/roko-serve/src/routes/plans.rs::generate_plan`, the spawned task that settles the operation. The
portal reads it through `GET /api/operations/{id}` (`routes/status/dashboard.rs::operation_status`).

## Current state

The task now settles the operation from one result. It is `Completed { slug, task_count }` when the
runtime reports plan targets and `load_plan_summary(<slug>)` finds the plan, and `Failed { error }`
otherwise: the generation failed, reported no targets, or its plan is missing or does not load. The event
log entry (`plan_generate.completed` / `.failed`) and `OperationCompleted.success` agree with the status.
`revise_plan` already ends `failed` when the revision is rejected or the plan disappears, so it needed no
change.

## Plan

Done as described under Current state.

## Done when

The `[[verify]]` passes: `get_operation_reports_failed_when_generation_writes_no_plan` (a stub runtime
reports success without writing the plan; the operation ends `failed` and its error names the plan), and
`get_operation_reports_running_then_completed_with_slug` still passes.

## Notes

The CLI runtime always returns at least one plan target (it falls back to the plans root), so the load
check is what catches a generation that finished without writing its plan where the server looks for it.
