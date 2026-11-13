+++
id = "bug-35c1f3"
kind = "bug"
title = "GET /api/plans/{id}/tasks drops the plan's own checks: gap-c1726f's plan_checks and workspace_rungs reach the runtime's record but not the route, which builds its JSON from five named fields, so the portal read CHECKS '0 of 10' on 7.2 where the definition says 11"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "bec6e4deb"
source = "round v8 portal-v8, phase 7.2's capture (2026-11-13)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["crates/roko-serve/src/routes/plans/reads.rs::plan_tasks"]
links = { depends_on = [], blocks = [], related = ["gap-c1726f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_tasks_route_names_the_plans_own_checks' crates/roko-serve/src && cargo test -p roko-serve --lib plan_tasks_route_names_the_plans_own_checks"

[closed]
at = 2026-11-13
at_ts = "2026-11-13T01:22:33Z"
commit = "bec6e4deb"
by = "portal-v8 (round v8)"
executor = "unknown"
forced = false
evidence = "plan_tasks now sends plan_checks and workspace_rungs from the runtime's PlanTasksDto. plan_tasks_route_names_the_plans_own_checks (roko-serve lib, through the route) was red at f72e3f8f6 ({max_parallel:1, plan_id:demo, task_count:0, tasks:[], title:null}) and passes at bec6e4deb in roko-work-bug-36ce72 (heavy.sh, CARGO_INCREMENTAL=0): fmt clean, roko-serve lib filtered plan_tasks|plan_detail|list_plans 5/5 (plan_detail_names_its_runs_cap and plan_detail_names_when_its_run_started included), clippy -D warnings clean."
+++

## Problem

GET /api/plans/{id}/tasks drops the plan's own checks. gap-c1726f gave the plan's tasks record `plan_checks` (its
`[meta] verify` steps) and `workspace_rungs`, and roko-cli fills them in. But the route builds its JSON by hand
from five named fields (`plan_id`, `task_count`, `title`, `max_parallel`, `tasks`), so neither reaches a surface.

The effect showed on phase 7.2's run (583b4be5, roko 4816a2cce): the portal's glass read CHECKS "0 of 10" while it
ran. Its five tasks have two steps each, and the plan has one check of its own, so the agreed definition says 11.
Once the plan check reported, the portal counted it.

## Why it matters

Goal visibility: the three surfaces' CHECKS agree only if each reads the same y from the start. The guide's moment
A is mid-run.

## Where

`crates/roko-serve/src/routes/plans/reads.rs::plan_tasks`.

## Plan

1. Return `plan_checks` and `workspace_rungs` in the route's answer.
2. Test the route itself: `plan_tasks_route_names_the_plans_own_checks`. gap-c1726f tested only the runtime's
   record, and the portal tested against a stubbed route.

## Done when

- The route returns both fields.
- The `[[verify]]` passes.

## Notes

- Round v8 portal-v8: my miss in gap-c1726f, found in 7.2's capture.
