+++
id = "gap-b95f43"
kind = "gap"
title = "Graph dispatch passes no RoutingContext (no cell-model tiers or graph-level cost budget)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/dispatch"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/NOUS-HERMES-INTEGRATION-CHECKLIST.md:1089"
discovered_from = "audit:tmp/NOUS-HERMES-INTEGRATION-CHECKLIST.md:1089"
anchors = ["roko-cli graph_execution dispatch", "RoutingContext"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Graph engine dispatch sets routing_context: None, so the cascade router gets no graph context; no cell→model tier mapping or GraphBudget. Roko-generic defect found in the Nous checklist (B8).

Imported without verification from:
- `tmp/NOUS-HERMES-INTEGRATION-CHECKLIST.md:1089`
- `tmp/NOUS-HERMES-INTEGRATION-CHECKLIST.md:1771`

How to verify: grep graph_execution/ and roko-graph for routing_context construction.
