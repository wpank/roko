+++
id = "gap-da873a"
kind = "gap"
title = "WF-11 P2: Missing plan task CRUD routes (POST add-task, DELETE task)"
status = "superseded"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/routes/plans"]
created = 2026-09-25
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/workflow-audit/11-FINAL-STATUS.md#P2 — Missing Phase 3 Routes"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#P2 — Missing Phase 3 Routes"
anchors = ["crates/roko-serve/src/routes/plans.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:18Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Superseded by Will's 2026-09-28 decision: a plan's tasks.toml is edited as text (GET/PUT /api/plans/{id}/source with server-side validation), so there are no per-task CRUD routes."
+++
POST /api/plans/{id}/tasks and DELETE /api/plans/{id}/tasks/{task_id} are not in the plans route table (only GET+PUT and PATCH exist).

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#P2 — Missing Phase 3 Routes`
- `tmp/workflow-audit/07-SERVE-ROUTE-CHANGES.md#Task CRUD for Visual Editor`

How to verify: Inspect plans router for post/delete handlers.
