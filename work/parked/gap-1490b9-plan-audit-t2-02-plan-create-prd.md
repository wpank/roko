+++
id = "gap-1490b9"
kind = "gap"
title = "[plan-audit T2-02] `plan create --prd --depends-on --crate`"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-02: Add `plan create --prd --depends-on --crate`"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-02: Add `plan create --prd --depends-on --crate`"
anchors = ["cmd_plan_create", "backlog #405"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Enrich plan create to wire PRD references, inter-plan dependencies and crate scope at scaffold time. Backlog #405.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-02: Add `plan create --prd --depends-on --crate``
- `tmp/archive/plan-audit-2026-09-23/13-cli-ux-gaps.md`

How to verify: roko plan create --help.
