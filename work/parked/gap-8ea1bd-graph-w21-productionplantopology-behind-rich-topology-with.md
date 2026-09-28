+++
id = "gap-8ea1bd"
kind = "gap"
title = "[graph W21] ProductionPlanTopology behind `--rich-topology` with stub-warning topology cells"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/topology"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
discovered_from = "audit:tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
anchors = ["crates/roko-graph/src/topology.rs", "ProductionPlanTopology", "--rich-topology"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Marked Done as '--rich-topology flag, topology cells registered with stub warning': the 11-node-per-task subgraph (6 enricher cells, ComposeCell, GateCell) is opt-in and some cells were stubs.

Imported without verification from:
- `tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done`
- `tmp/archive/graph-audit/07-work-items.md#w21`

How to verify: Check whether topology cells still log stub warnings and whether ProductionPlanTopology is used by default.
