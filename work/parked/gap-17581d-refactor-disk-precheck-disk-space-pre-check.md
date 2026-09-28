+++
id = "gap-17581d"
kind = "gap"
title = "[refactor disk-precheck] Disk-space pre-check (`--force`) missing from Graph preflight"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
anchors = ["--force", "roko doctor disk"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Runner-v2 validated free space before execution (bypassable with --force); Graph preflight does not (resource lifecycle claims disk-aware worktree admission elsewhere).

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported`

How to verify: Check Graph plan-run preflight for free-space admission.
