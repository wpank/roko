+++
id = "gap-8c1330"
kind = "gap"
title = "[plan-audit T3-06] Cherry-pick refresh for stale plan branches"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/merge"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["refresh_plan_to_batch (proposed)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Stale plan branches are never refreshed onto the updated batch; implement refresh_plan_to_batch by cherry-picking unique commits (gap matrix severity High).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: grep cherry-pick in merge/worktree code.
