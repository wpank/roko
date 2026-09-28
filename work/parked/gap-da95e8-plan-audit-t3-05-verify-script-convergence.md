+++
id = "gap-da95e8"
kind = "gap"
title = "[plan-audit T3-05] Verify-script convergence detection and regeneration"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/verify"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["verify cells"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Track last 5 error hashes; after 3 identical, regenerate the verify script. Not in the graph engine.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: grep error-hash convergence logic.
