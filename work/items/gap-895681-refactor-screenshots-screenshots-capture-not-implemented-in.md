+++
id = "gap-895681"
kind = "gap"
title = "[refactor --screenshots] `--screenshots` capture not implemented in Graph engine"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
anchors = ["--screenshots"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Runner-v2 captured screenshots during execution; Graph engine does not implement it (low risk, niche).

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported`

How to verify: Check if the flag still exists and whether it is rejected/warned or silently ignored.
