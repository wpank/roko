+++
id = "gap-32ac2f"
kind = "gap"
title = "[refactor P1-10] EventBus/event-enum consolidation not implemented (audit + design only)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-runtime/events"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt"
anchors = ["crates/roko-runtime/", "crates/roko-core/", "RuntimeEvent", "GraphExecutionEvent", "DashboardEvent", "WorkspaceEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Marked DONE as an audit: 47 event enums across 42 files, 4 EventBus structs, 2 bus trait systems, ~380 variants; costliest overlaps RuntimeEvent/GraphExecutionEvent, ServerEvent/DashboardEvent, three AgentEvent enums. 4-phase migration and P3-09 WorkspaceEvent two-level design not implemented.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt`
- `tmp/archive/refactoring-audit-2026-09-21/P1-10-EVENTBUS-AUDIT.md`
- `tmp/archive/refactoring-audit-2026-09-21/P3-09-EVENT-SCHEMA-DESIGN.md`

How to verify: Count event enums/EventBus structs; grep WorkspaceEvent to see if migration started.
