+++
id = "gap-fe9c7a"
kind = "gap"
title = "[plan-audit T3-07] Role-filtered AGENTS.md injection"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["AGENTS.md role markers"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Mori strips irrelevant sections via <!-- role: X --> markers (~3.5K tokens saved); roko has no role-filtered doc injection.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: grep for role marker filtering.
