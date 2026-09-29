+++
id = "gap-a6de8d"
kind = "gap"
title = "Split roko-serve routes/plans.rs into run-control, authoring, merge and read modules"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W5-contention-parallelism.md (F1, F2, rec 3)"
anchors = ["crates/roko-serve/src/routes/plans.rs", "crates/roko-serve/src/routes/plans/"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f crates/roko-serve/src/routes/plans/run_control.rs && test -f crates/roko-serve/src/routes/plans/authoring.rs && test -f crates/roko-serve/src/routes/plans/merge.rs && test -f crates/roko-serve/src/routes/plans/reads.rs && cargo test -p roko-serve --lib routes::plans && cargo test -p roko-serve --test route_coverage_matrix"
+++

## Problem

`crates/roko-serve/src/routes/plans.rs` is 4,391 lines. It holds all 20 plan route registrations (`routes()`, line
23), every handler (to line 2545) and about 1,845 lines of tests. Changes to run control, plan authoring, reviews and
merges, and the read endpoints all edit this one file. It had the most churn of any serve file since 09-15, and W5
counts 11 open pieces of work queued on it.

## Why it matters

Splitting it lets the serve side of E6 and E7, and the portal work, proceed in parallel. W5 rec 3 notes that nobody
is editing it today: the last change was `188c43c8d`, and no in-flight worktree touches it. So it can be split now
without conflicts. It is part of epic spec-9a3131.

## Where

`routes()` stays in `plans.rs` and composes four new modules under `crates/roko-serve/src/routes/plans/`:

- **`run_control.rs`:** `execute_plans`, `start_plan_run`, `execute_plan`, `plan_status`, `pause_plan`,
  `resume_plan` and `cancel_plan`.
- **`authoring.rs`:** `create_plan`, `generate_plan`, `revise_plan`, `plan_chat`, `get_plan_source`,
  `put_plan_source` and `validate_plan`, plus the slug and PRD helpers.
- **`merge.rs`:** `list_reviews`, `submit_review`, `record_review`, `task_diff` and the git helpers
  (`find_agent_branch`, `diff_summary`, `parse_git_diff`, `merge_branch`).
- **`reads.rs`:** `list_plans`, `get_plan`, `plan_tasks`, `plan_gates`, `plan_costs`, `plan_estimate` and
  `load_efficiency_history`.

`resolve_plan` is shared by several modules, so it stays in `plans.rs`.

## Current state

Unsplit at `41c7ffbd6`.

## Plan

1. Move each handler group, with its tests, into its module. URLs, handler names and behaviour stay unchanged.
2. Keep `pub(crate) fn slug_from_title` reachable at its current path, through a re-export.
3. Run the plan route tests: the `routes::plans` unit tests, and the integration tests `route_coverage_matrix`,
   `plan_execute`, `plan_authoring`, `plan_revision` and `plan_discovery`.

## Done when

- [ ] The four modules exist, and `routes()` registers the same 20 routes.
- [ ] The integration tests listed above pass.
- [ ] The `[[verify]]` command passes.

## Notes

- Serve routes are hot in general, but this file has no writer today, so do it before anyone starts one.
- tldr/05 P0 #5 lists a fix to the diff route; that fix would then land in `merge.rs`.
