+++
id = "gap-c2a751"
kind = "gap"
title = "Consolidate ~14 duplicate type families and unify the event system (47 enums, 4 EventBus structs)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["workspace/types"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.1 Duplicate Type Families (~14)"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.1 Duplicate Type Families (~14)"
anchors = ["roko_runtime event bus", "roko_core::Bus"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Remaining duplicate families: AgentState, TaskStatus, GateFeedback, Cell/Node, Plan/Workflow, event buses (47 event enums, 4 EventBus structs, 2 bus trait systems). Four-phase event migration plan exists.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.1 Duplicate Type Families (~14)`
- `docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items`
- `tmp/refactoring-audit/P1-10-EVENTBUS-AUDIT.md`

How to verify: grep -c 'enum .*Event' across crates; list struct EventBus definitions.
