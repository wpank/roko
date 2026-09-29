+++
id = "gap-b57990"
kind = "gap"
title = "[plan-audit T2-03] `plan run` ignores .roko/queue.toml milestone ordering"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-03: Wire queue manifest consumption into `plan run`"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-03: Wire queue manifest consumption into `plan run`"
anchors = [".roko/queue.toml", "roko plan queue"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
When .roko/queue.toml exists, plan run should consume milestone assignments/ordering instead of alphabetical discovery.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-03: Wire queue manifest consumption into `plan run``

Warning: every file this item cites is gone (`.roko/queue.toml`) — likely obsolete or moved.

How to verify: Check plan run discovery for queue manifest reads.
