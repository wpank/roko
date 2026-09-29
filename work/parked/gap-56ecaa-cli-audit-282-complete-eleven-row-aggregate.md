+++
id = "gap-56ecaa"
kind = "gap"
title = "[cli-audit #282] Complete eleven-row aggregate checkpoint-extension registry and cross-extension fixtures"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/snapshot"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 4 — parity, cutover, and lifecycle UX"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 4 — parity, cutover, and lifecycle UX"
anchors = ["roko-graph/src/snapshot.rs GraphSnapshotV2 CheckpointExtension", "roko-graph/src/hot.rs HotCheckpointOptions", "backlog #282"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Unblocked after #351/#327 but unchecked: #282 lifecycle checkpoint completeness still needs its eleven-row aggregate registry and cross-extension fixtures on GraphSnapshotV2/CheckpointExtension.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 4 — parity, cutover, and lifecycle UX`
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md`

How to verify: Look for an aggregate extension registry and cross-extension fixture tests in roko-graph.
