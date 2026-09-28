+++
id = "gap-531c56"
kind = "gap"
title = "Graph DAG, Node Detail, and Hot-Graph TUI Widgets"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/backlog/archive/266-graph-tui-topology-hot-widgets.md#266 — Graph DAG, Node Detail, and Hot-Graph TUI Widgets"
discovered_from = "audit:tmp/backlog/archive/266-graph-tui-topology-hot-widgets.md#266 — Graph DAG, Node Detail, and Hot-Graph TUI Widgets"
anchors = ["crates/roko-cli/src/tui/widgets/", "dashboard_view.rs", "mod.rs", "tmp/engine-audit/19-graph-tui-integration.md", "20-graph-event-emission.md", "crates/roko-cli/src/tui/graph_view_state.rs", "tui/mod.rs", "widgets/graph_dag.rs", "TelemetryEventSink", "GraphEventSink"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked — observability enhancement after connected event parity

Imported without verification from:
- `tmp/backlog/archive/266-graph-tui-topology-hot-widgets.md#266 — Graph DAG, Node Detail, and Hot-Graph TUI Widgets`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.5 Graph engine TUI integration`
- `tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)`
- `tmp/archive/ux-audit-2026-09-21/18-backlog-cross-reference.md#4. GraphEventSink + GraphTuiAdapter (graph engine TUI integration)`
- `tmp/archive/ux-audit-2026-09-21/18-backlog-cross-reference.md#7. ASCII DAG widget for graph visualization`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`

Some cited files are gone: `crates/roko-cli/src/tui/graph_view_state.rs`, `tmp/engine-audit/19-graph-tui-integration.md`, `widgets/graph_dag.rs`.

How to verify: Check: Typical 5-20 node graphs are readable at supported terminal sizes.; Widget rendering performs no filesystem/network I/O.; Unknown authored-graph cells render safely. [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked] / Check for graph_dag/hot_graph widgets and whether #248 envelopes exist.

Merged 2 mined candidates: m1-075, m4-148.
