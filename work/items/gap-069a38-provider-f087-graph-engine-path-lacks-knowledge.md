+++
id = "gap-069a38"
kind = "gap"
title = "[provider F087] Graph engine path lacks knowledge-aware routing, Daimon modulation, and episode recording"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F087"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F087"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`GraphTaskDispatcher` does not query the neuro store, does not consult Daimon state, and does not record episodes or efficiency events. Graph-dispatched tasks are invisible to the learning pipeline.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F087`
- `tmp/archive/provider-audit/20-graph-integration.md`

How to verify: Graph is now the sole engine; check graph dispatch for knowledge routing, Daimon modulation, episode recording. Confirm in crates/roko-cli/src/graph_task_dispatch.rs whether still true: Graph engine path lacks knowledge-aware routing, Daimon modulation, and episode recording
