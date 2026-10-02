+++
id = "gap-214feb"
kind = "gap"
title = "WF-11 P3: Portal plan editor spec gaps (DAG canvas, verify-step editor, drag reorder, file tags)"
status = "superseded"
triage = "verified"
severity = "p3"
subsystem = ["apps/portal"]
created = 2026-09-25
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/workflow-audit/11-FINAL-STATUS.md#P3 — Phase 4 Portal Gaps (polish, not blocking)"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#P3 — Phase 4 Portal Gaps (polish, not blocking)"
anchors = ["apps/portal/src/app/work/editor/page.tsx"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:18Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Superseded by Will's 2026-09-28 decision: the one-plan-view portal (apps/portal) edits tasks.toml as text instead of a visual DAG editor."
+++
Editor lacks the SVG/dagre DAG canvas with drag-to-connect, description/verify-step/acceptance-criteria editors, model-hint/tier selectors, sidebar drag-to-reorder and tag-style file input.

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#P3 — Phase 4 Portal Gaps (polish, not blocking)`
- `tmp/workflow-audit/06-PORTAL-PLAN-EDITOR.md#Three-Panel Design`

How to verify: Open /work/editor and compare with 06 spec.
