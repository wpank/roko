+++
id = "gap-be8416"
kind = "gap"
title = "[plan-audit T0-03] Wave-parallel plan execution (plans run sequentially)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-03: Implement wave-parallel plan execution"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-03: Implement wave-parallel plan execution"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1197", "crates/roko-cli/src/runner/plan_dag.rs::CrossPlanDag"]
links = { depends_on = [], blocks = [], related = ["gap-0001a1"], supersedes = [], duplicate_of = "gap-7c9e48" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-7c9e48 (backlog #396; still true: plans run sequentially at crates/roko-cli/src/graph_execution/plan_runner.rs:1197 and CrossPlanDag::compute is only used for display in commands/plan.rs:67/382)"
+++
CrossPlanDag::compute_waves() is display-only; cmd_plan_run_engine() loops plans sequentially. Launch up to max_parallel_plans graph engines per wave. Backlog #396 (#272).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-03: Implement wave-parallel plan execution`
- `tmp/archive/plan-audit-2026-09-23/04-wave-parallel-execution.md`

How to verify: Inspect cmd_plan_run_engine for concurrent per-wave dispatch.

Verified 2026-09-28: still true (graph_execution/plan_runner.rs:1197 sequential loop); duplicate of gap-7c9e48.
