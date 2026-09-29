+++
id = "gap-84c105"
kind = "gap"
title = "[plan-audit T3-04] No zombie agent reaping (4-hour max age)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["ProcessSupervisor", "graph engine in-flight tracking"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Mori force-clears agents older than 4h from in_flight; roko graph engine has no max-age zombie detection (gap matrix severity High).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: grep for max age / zombie reaping.
