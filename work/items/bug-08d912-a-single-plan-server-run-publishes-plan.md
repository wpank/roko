+++
id = "bug-08d912"
kind = "bug"
title = "A single-plan server run publishes plan_completed twice, the second after run_completed"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f99e45dba"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["crates/roko-serve/src/routes/plans.rs:552"]
links = { depends_on = [], blocks = [], related = ["gap-8a1fb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo build -p roko-cli && bash -c 'source plans/portal-programme/_harness/lib.sh && require_binary && make_workspace && start_server && start_capture && [ \"$(api POST /api/plans/live-b/execute)\" = 202 ] && wait_idle live-b 120 && sleep 3 && stop_capture && sse count plan_completed plan_id=live-b --eq 1'"
+++

## Problem

A run started with `POST /api/plans/{id}/execute` publishes `plan_completed` from the Graph run
through the hub. After `run_plan` returns, the route's spawned task publishes
`ServerEvent::PlanCompleted { plan_id, success }` a second time (`routes/plans.rs:552`).

Every single-plan capture in the 09 evidence shows `plan_completed, run_completed, plan_completed`:
both runs in `tmp/portal-audit/evidence/portal-check/events.sse`, and the run in
`tmp/portal-audit/evidence/hello-world-real-run1/events.sse`. The set run (`POST /api/plans/execute`,
`par-events.sse`) publishes one per plan.

Reproduced on 2026-09-29: in a harness workspace, execute `live-b`, and
`sse count plan_completed plan_id=live-b --eq 1` reports "2 matches".

## Why it matters

Goal `visibility`. Clients that count completions see two, one after the run has ended. The 03 AS
BUILT record (`tmp/portal-audit/03-CONTRACT.md`) says plan_completed is "live, once per plan".

## Where

`crates/roko-serve/src/routes/plans.rs:552`, in the spawned task of the single-plan execute path.

## Plan

Publish from the route only when the engine did not, for example on the error path where the run
never started. Otherwise drop the route's publish.

## Done when

The `[[verify]]` (fake-agent harness) counts exactly one `plan_completed` for the plan.

## Notes

Found by plan 09 T04 (see VERDICT).
