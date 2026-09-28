+++
id = "gap-5e818f"
kind = "gap"
title = "DF-0925 P2-6: Per-task waste (sync ExperimentStore RMW, double dream advice, unread generated-tests/)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste"
anchors = ["graph_task_dispatch.rs:1833", "graph_task_dispatch.rs:1568 EvalGenerator", "generated-tests/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
ExperimentStore does a full synchronous JSON read-modify-write per task on the reactor, dream routing advice loads twice per task, and EvalGenerator writes generated-tests/ files nothing reads (they litter the tree).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste`

How to verify: Check git status for generated-tests/ files after a run.
