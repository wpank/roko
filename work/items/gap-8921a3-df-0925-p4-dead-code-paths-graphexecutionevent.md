+++
id = "gap-8921a3"
kind = "gap"
title = "DF-0925 P4: Dead code paths (GraphExecutionEvent sink, duplicate streaming verify path, build_fix_prompt)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/engine"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["engine.rs:323", "graph_task_dispatch.rs:3463-3602", "TaskDef::build_fix_prompt"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The engine stores a GraphExecutionEvent sink but never emits, so --log-file writes nothing; a second streaming verify implementation (graph_task_dispatch.rs:3463-3602) drifts with no cap/feedback/auto-fix; TaskDef::build_fix_prompt is test-only.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`
- `tmp/archive/dogfood-2026-08-25/DOGFOOD-DEBRIEF.md#Issue 2: Log output buffering prevents monitoring (MEDIUM)`

How to verify: Run plan run --log-file and check file content.
