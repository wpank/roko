+++
id = "gap-27637c"
kind = "gap"
title = "AgentPool runtime instantiation missing in runner (backlog #55)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/pools"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items"
discovered_from = "audit:docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items"
anchors = ["roko_agent AgentPool"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Pool/multi-pool management and a TUI modal exist, but the runner never instantiates pools (no pool dispatch or warm reuse).

Imported without verification from:
- `docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items`
- `docs/v3/39-ROADMAP.md#2.3 P2 -- Medium (Selected)`

How to verify: grep AgentPool constructors in runner/graph_execution.
