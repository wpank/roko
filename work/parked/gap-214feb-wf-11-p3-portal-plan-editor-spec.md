+++
id = "gap-214feb"
kind = "gap"
title = "WF-11 P3: Portal plan editor spec gaps (DAG canvas, verify-step editor, drag reorder, file tags)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["apps/portal"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/workflow-audit/11-FINAL-STATUS.md#P3 — Phase 4 Portal Gaps (polish, not blocking)"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#P3 — Phase 4 Portal Gaps (polish, not blocking)"
anchors = ["apps/portal/src/app/work/editor/page.tsx"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Editor lacks the SVG/dagre DAG canvas with drag-to-connect, description/verify-step/acceptance-criteria editors, model-hint/tier selectors, sidebar drag-to-reorder and tag-style file input.

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#P3 — Phase 4 Portal Gaps (polish, not blocking)`
- `tmp/workflow-audit/06-PORTAL-PLAN-EDITOR.md#Three-Panel Design`

How to verify: Open /work/editor and compare with 06 spec.
