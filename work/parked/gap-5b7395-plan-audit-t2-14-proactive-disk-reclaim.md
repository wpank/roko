+++
id = "gap-5b7395"
kind = "gap"
title = "[plan-audit T2-14] Proactive disk reclaim during plan execution"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-14: Proactive disk reclaim during execution"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-14: Proactive disk reclaim during execution"
anchors = ["disk-aware worktree admission", "roko doctor disk"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Check free disk before each dispatch: <30 GiB reclaim idle worktrees/stale targets, <15 GiB aggressive cleanup (admission exists; mid-run reclaim does not).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-14: Proactive disk reclaim during execution`

How to verify: grep dispatch path for disk checks/reclaim.
