+++
id = "gap-da7307"
kind = "gap"
title = "Plan portal-plan-execution: live task status + agent output streaming in portal plan editor (3 ready)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["portal/plan-editor"]
created = 2026-09-24
updated = 2026-09-28
source = "plans/portal-plan-execution/plan.md"
discovered_from = "audit:plans/portal-plan-execution/plan.md"
anchors = ["portal /work/editor", "StateHub SSE"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Ready plan: after Execute, the /work/editor page goes static; wire StateHub SSE task_started/completed/failed and agent_output_line into live status indicators and a collapsible output panel. May overlap portal-programme/08-portal-run.

Imported without verification from:
- `plans/portal-plan-execution/plan.md`
- `plans/portal-plan-execution/tasks.toml`

How to verify: Compare with plans/portal-programme/08-portal-run/tasks.toml; check portal editor page for SSE subscription.
