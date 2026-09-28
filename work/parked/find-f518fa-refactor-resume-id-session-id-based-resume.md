+++
id = "find-f518fa"
kind = "finding"
title = "[refactor --resume-id] Session-id based resume (`--resume <id>`) replaced by plan-dir snapshot resume"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
anchors = ["roko resume [run-id]", "--resume-plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Runner-v2 resumed by session ID; Graph uses snapshot-based resume by plan directory (medium risk, different UX paradigm).

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported`

How to verify: Check `roko resume <run-id>` and `plan run --resume` behavior on Graph for run-id selection.
