+++
id = "gap-631362"
kind = "gap"
title = "[plan-audit T2-08] Cross-plan type registry (CONTEXT.md)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-08: Implement CROSS-PLAN type registry (CONTEXT.md)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-08: Implement CROSS-PLAN type registry (CONTEXT.md)"
anchors = ["context_registry.rs (proposed)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Maintain a workspace-wide listing of types defined by each plan with module paths so agents resolve imports without grepping.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-08: Implement CROSS-PLAN type registry (CONTEXT.md)`

How to verify: Search for a generated CONTEXT.md/type registry.
