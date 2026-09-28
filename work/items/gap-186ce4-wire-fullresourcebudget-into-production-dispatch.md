+++
id = "gap-186ce4"
kind = "gap"
title = "Wire FullResourceBudget into Production Dispatch"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/383-full-resource-budget-wiring.md#383 — Wire FullResourceBudget into Production Dispatch"
discovered_from = "audit:tmp/backlog/archive/383-full-resource-budget-wiring.md#383 — Wire FullResourceBudget into Production Dispatch"
anchors = ["crates/roko-cli/src/dispatch/", "crates/roko-core/src/config/", "FullResourceBudget"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
complete 5-dimensional scheduler tested but not wired. Dev-audit found a complete 5-dimensional resource scheduler (agents, tokens, cost, worktrees, memory) that is fully tested but never called from production dispatch. Resource config fields (gc_on_plan_start, gc_on_plan_end…

Imported without verification from:
- `tmp/backlog/archive/383-full-resource-budget-wiring.md#383 — Wire FullResourceBudget into Production Dispatch`

How to verify: Check whether the gap described in tmp/backlog/archive/383-full-resource-budget-wiring.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
