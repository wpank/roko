+++
id = "gap-2a39f9"
kind = "gap"
title = "[plan-audit T2-10] Compile-fail-repeat, stuck-pattern, spec-weakening monitors in Graph path"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-10: Add compile/stuck/spec-weakening monitors"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-10: Add compile/stuck/spec-weakening monitors"
anchors = ["crates/roko-cli/src/graph_execution/monitors.rs (proposed)", "roko-conductor watchers stuck_pattern spec_drift"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Port mori monitors: CompileFailRepeat (3x same error -> new approach), StuckPattern (3x identical output -> restart), SpecWeakeningDetector (deleted assertion -> abort to Auditor) into graph_execution.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-10: Add compile/stuck/spec-weakening monitors`
- `tmp/archive/plan-audit-2026-09-23/09-safeguards-watchdog.md`

Warning: every file this item cites is gone (`crates/roko-cli/src/graph_execution/monitors.rs`) — likely obsolete or moved.

How to verify: Check whether conductor watchers run during Graph plan execution.
