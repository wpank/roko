+++
id = "gap-22f2e9"
kind = "gap"
title = "[plan-audit T2-04] `plan run --milestone` filtering"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-04: Add milestone filtering (`--milestone`)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-04: Add milestone filtering (`--milestone`)"
anchors = ["PlanCmd::Run"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Add --milestone <label> to run only plans in a milestone group (depends on T2-03).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-04: Add milestone filtering (`--milestone`)`

How to verify: roko plan run --help.
