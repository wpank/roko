+++
id = "gap-da873a"
kind = "gap"
title = "WF-11 P2: Missing plan task CRUD routes (POST add-task, DELETE task)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes/plans"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/workflow-audit/11-FINAL-STATUS.md#P2 — Missing Phase 3 Routes"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#P2 — Missing Phase 3 Routes"
anchors = ["crates/roko-serve/src/routes/plans.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
POST /api/plans/{id}/tasks and DELETE /api/plans/{id}/tasks/{task_id} are not in the plans route table (only GET+PUT and PATCH exist).

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#P2 — Missing Phase 3 Routes`
- `tmp/workflow-audit/07-SERVE-ROUTE-CHANGES.md#Task CRUD for Visual Editor`

How to verify: Inspect plans router for post/delete handlers.
