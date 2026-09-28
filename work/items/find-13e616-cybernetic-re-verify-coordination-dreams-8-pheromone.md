+++
id = "find-13e616"
kind = "finding"
title = "[cybernetic re-verify: coordination/dreams] 8 pheromone/dream/cross-cut/Inbox closures wired into deleted Runner-v2"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-06
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code"
discovered_from = "audit:tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code"
anchors = ["CrossCutArbitrator", "DreamOutputConsumer", "Inbox publisher", "pheromone deposit"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Checked done 2026-09-06 in runner/: P1-13 pheromone deposits, P1-14 pheromone field in dispatch, P1-16 CrossCutArbitrator, P1-17/P1-18 DreamOutputConsumer, P1-23 production Inbox publisher, P2-25 cross-cut Lens events, P2-33 native agent observation bridge.

Imported without verification from:
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P2 -- Add Missing Observability`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep call sites of these types outside tests; confirm reachable from graph plan dispatch.
