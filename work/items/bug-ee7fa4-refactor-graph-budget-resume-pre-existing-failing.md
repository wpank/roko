+++
id = "bug-ee7fa4"
kind = "bug"
title = "[refactor graph_budget_resume] Pre-existing failing test graph_budget_resume (2026-09-14)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/budget"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#2026-09-14-session-batch-3"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#2026-09-14-session-batch-3"
anchors = ["graph_budget_resume", "crates/roko-graph/src/budget.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Workspace tests on 2026-09-14: 6,665 passed, 1 failed (pre-existing graph_budget_resume), 35 ignored; no fix recorded.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#2026-09-14-session-batch-3`

How to verify: Locate the graph_budget_resume test and check whether it passes / is ignored.
