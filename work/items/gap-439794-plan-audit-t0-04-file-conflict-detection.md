+++
id = "gap-439794"
kind = "gap"
title = "[plan-audit T0-04] File-conflict detection before same-wave task dispatch"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-graph/engine"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-04: Add file-conflict detection in wave dispatch"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-04: Add file-conflict detection in wave dispatch"
anchors = ["crates/roko-graph/src/engine.rs::execute_parallel_at_tick_validated", "crates/roko-cli/src/graph_execution/plan_runner.rs:1059"]
links = { depends_on = [], blocks = [], related = ["gap-4835e7"], supersedes = [], duplicate_of = "" }
+++
Tasks in the same topological wave can write the same file concurrently; check file-set overlap in execute_parallel_at_tick_validated() before launching.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-04: Add file-conflict detection in wave dispatch`

How to verify: Check engine for file-overlap serialization of parallel cells.

Verified 2026-09-28: execute_parallel_at_tick_validated (crates/roko-graph/src/engine.rs:944) launches ready nodes with no file-set overlap check, and nothing in roko-graph or graph_execution/ serializes same-wave tasks by files. Per-task worktree isolation is opt-in (graph_execution/plan_runner.rs:1059, --worktree-per-task), so by default parallel tasks share one workspace.
