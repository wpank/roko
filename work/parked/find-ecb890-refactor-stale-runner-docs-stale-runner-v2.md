+++
id = "find-ecb890"
kind = "finding"
title = "[refactor stale-runner-docs] Stale Runner-v2 references in GAPS.md and docs"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["docs"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#3-delete-stale-runner-v2-documentation-references-p2"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#3-delete-stale-runner-v2-documentation-references-p2"
anchors = [".roko/GAPS.md", "CLAUDE.md", "docs/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
GAPS.md and various docs still reference Runner-v2 behaviors, event_loop.rs extraction plans, and runner-v2 internal state although Graph is the sole engine (CLAUDE.md itself still mentions runner/event_loop.rs and --engine legacy).

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#3-delete-stale-runner-v2-documentation-references-p2`

How to verify: grep -n 'event_loop.rs\|Runner-v2\|runner-v2' .roko/GAPS.md CLAUDE.md docs/.
