+++
id = "gap-67d0e9"
kind = "gap"
title = "[plan-audit T1-08] Add `--max-parallel-plans` to plan run"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-08: Add `--max-parallel-plans` to `plan run`"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-08: Add `--max-parallel-plans` to `plan run`"
anchors = ["PlanCmd::Run in main.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Only --max-tasks exists; add per-run plan concurrency control (depends on wave parallelism T0-03).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-08: Add `--max-parallel-plans` to `plan run``

How to verify: roko plan run --help.
