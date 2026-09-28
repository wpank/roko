+++
id = "bug-d7c5dd"
kind = "bug"
title = "WF-11: Plan generation lifecycle not observable (no plan_generating/generated/updated events; silent timeout)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes/plans"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/workflow-audit/11-FINAL-STATUS.md#Known Issues"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#Known Issues"
anchors = ["crates/roko-serve/src/routes/plans.rs", "apps/portal/src/app/work/editor/page.tsx"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
POST /api/plans/generate returns plan_id before tasks exist; only OperationCompleted is emitted, so the portal polls ~20s and times out silently when no provider is available (no user-facing error).

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#Known Issues`
- `tmp/workflow-audit/11-FINAL-STATUS.md#Phase 3: Serve Route Changes — PARTIAL (~65%)`
- `tmp/workflow-audit/07-SERVE-ROUTE-CHANGES.md#SSE Event Changes`

How to verify: Call generate with no provider configured; watch SSE and portal behavior.
