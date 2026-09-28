+++
id = "find-2ba5ee"
kind = "finding"
title = "[provider F088] Graph engine BudgetTracker uses static estimates, not actual provider costs"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/budget"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F088"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F088"
anchors = ["crates/roko-graph/src/budget.rs", "crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`BudgetTracker` charges `cell.estimated_cost()` (a static configuration constant). The actual provider-reported cost (from `GraphPlanBudgetLedger`) is never reconciled with the budget tracker. `CellContext::budget_remaining` may diverge significantly from actual spend.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F088`
- `tmp/archive/provider-audit/20-graph-integration.md`

How to verify: CLAUDE.md claims paid-failure-aware cost enforcement in roko-graph; check BudgetTracker inputs. Confirm in crates/roko-graph/src/budget.rs, crates/roko-cli/src/graph_task_dispatch.rs whether still true: Graph engine BudgetTracker uses static estimates, not actual provider costs
