+++
id = "gap-a8f38f"
kind = "gap"
title = "[plan-audit T2-13] Spawn-failure exponential backoff in graph dispatch"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-13: Add spawn failure backoff to graph engine"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-13: Add spawn failure backoff to graph engine"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Add 2s->4s->30s backoff on agent spawn failure and fail the task after N consecutive spawn failures.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-13: Add spawn failure backoff to graph engine`

How to verify: Check spawn error handling for backoff.
