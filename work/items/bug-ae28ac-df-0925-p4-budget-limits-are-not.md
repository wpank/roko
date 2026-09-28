+++
id = "bug-ae28ac"
kind = "bug"
title = "DF-0925 P4: Daily and agent-lifetime budget limits are not enforced by the Graph engine"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:791", "crates/roko-cli/src/graph_task_dispatch.rs:1074"]
links = { depends_on = [], blocks = [], related = ["gap-d31457", "q-778b4f", "q-e23804"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -A1 "budget.max_daily_usd" crates/roko-cli/src/graph_task_dispatch.rs | grep NOT_ENFORCED'
+++
budget.max_task_usd, max_task_retry_usd, max_daily_usd and max_agent_lifetime_usd are read only by Runner-v2, so `plan run` has no per-task or daily spend protection.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-20: Budget Enforcement Disabled by Default`

How to verify: grep budget config reads under graph_execution/.

Verified 2026-09-28: narrowed - per-task caps are now enforced on the Graph path in the working tree (uncommitted; graph_task_dispatch.rs:1074-1306, budget.max_task_usd x tier multiplier + budget.max_task_retry_usd), but budget.max_daily_usd and budget.max_agent_lifetime_usd are still flagged NOT_ENFORCED by the Graph config audit (graph_task_dispatch.rs:790-799).
