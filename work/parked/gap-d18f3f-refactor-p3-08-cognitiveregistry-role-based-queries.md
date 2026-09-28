+++
id = "gap-d18f3f"
kind = "gap"
title = "[refactor P3-08] CognitiveRegistry (role-based queries/metrics over CellRegistry) deferred"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/cells"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p3-low-cosmetic-future"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p3-low-cosmetic-future"
anchors = ["crates/roko-graph/", "CellRegistry", "CognitiveRegistry"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Feasibility assessed (1 week): CognitiveRegistry would wrap existing CellRegistry with role-based queries and metrics; not built.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p3-low-cosmetic-future`

How to verify: grep CognitiveRegistry.
