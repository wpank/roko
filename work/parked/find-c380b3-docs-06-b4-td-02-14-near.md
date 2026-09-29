+++
id = "find-c380b3"
kind = "finding"
title = "DOCS-06 B4 / TD-02: ~14 near-duplicate type families across crates"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/types"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B4. ~14 conceptual type families need consolidation"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B4. ~14 conceptual type families need consolidation"
anchors = ["DashboardSnapshot", "DashboardData", "TaskStatus"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
DashboardSnapshot/DashboardData, StateHub types, AgentState, TaskStatus, GateFeedback, EventBus, Cell/Node and Plan/Workflow have near-duplicate definitions, risking serialization mismatch (decision: consolidate during v3 docs work).

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B4. ~14 conceptual type families need consolidation`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-02: ~14 Duplicate Type Families`
- `tmp/dogfood/2026-09-19-session.md#Fixes Applied This Session`

How to verify: grep for duplicate enum/struct names across crates.
