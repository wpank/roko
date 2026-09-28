+++
id = "gap-1d6bb0"
kind = "gap"
title = "[plan-audit T1-10] Review feedback compression on retry"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-10: Implement review feedback compression"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-10: Implement review feedback compression"
anchors = ["render_gate_feedback", "crates/roko-cli/src/dispatch/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Retries use raw gate/review text (~5K tokens); port mori compress_feedback() to extract unresolved blocking issues into a ~500-char directive.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-10: Implement review feedback compression`

How to verify: grep compress_feedback / feedback compression.
