+++
id = "gap-b23b9c"
kind = "gap"
title = "[plan-audit T1-06] Wire merge queue into Graph plan execution"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/merge"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-06: Wire merge queue into graph engine"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-06: Wire merge queue into graph engine"
anchors = ["MergeEnqueuer", "MergeQueue", "crates/roko-cli/src/runner/merge.rs", "backlog #404"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
MergeEnqueuer/MergeQueue exist but cmd_plan_run_engine does not enqueue plans for serialized, file-overlap-aware merge after gates/review pass.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-06: Wire merge queue into graph engine`
- `tmp/archive/plan-audit-2026-09-23/12-merge-batch-branch.md`

How to verify: grep MergeEnqueuer call sites in commands/plan.rs / graph_execution.
