+++
id = "gap-074c76"
kind = "gap"
title = "[plan-audit T2-05] Per-task prompt budget scaling from task metadata"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/templates"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-05: Implement per-task budget scaling from metadata"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-05: Implement per-task budget scaling from metadata"
anchors = ["crates/roko-compose/src/templates/common.rs budget_for adaptive_budget_for"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Use context_weight, reasoning_level, speed_priority, quality_profile to scale prompt char budgets (mori scaled_prompt_cap()); depends on T1-01.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-05: Implement per-task budget scaling from metadata`

How to verify: Check budget_for inputs for task metadata.
