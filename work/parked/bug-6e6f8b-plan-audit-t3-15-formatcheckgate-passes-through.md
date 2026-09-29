+++
id = "bug-6e6f8b"
kind = "bug"
title = "[plan-audit T3-15] FormatCheckGate passes through on format violations"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-gate"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["FormatCheckGate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
FormatCheckGate doc comment says it 'passes through'; cargo fmt --check runs but violations never fail the gate.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: Read FormatCheckGate verdict logic; run on an unformatted file.
