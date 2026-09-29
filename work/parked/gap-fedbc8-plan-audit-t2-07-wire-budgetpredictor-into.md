+++
id = "gap-fedbc8"
kind = "gap"
title = "[plan-audit T2-07] Wire BudgetPredictor into dispatch token budgeting"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-07: Wire BudgetPredictor to dispatch"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-07: Wire BudgetPredictor to dispatch"
anchors = ["BudgetPredictor::predict", "crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Query BudgetPredictor::predict() at dispatch to set prompt token budget; bootstrap from efficiency.jsonl. Backlog #402.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-07: Wire BudgetPredictor to dispatch`

How to verify: grep BudgetPredictor call sites.
