+++
id = "gap-8d8a4d"
kind = "gap"
title = "[plan-audit T1-11] Prompt cache alignment in SystemPromptBuilder"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-11: Implement prompt cache alignment"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-11: Implement prompt cache alignment"
anchors = ["crates/roko-compose/src/system_prompt_builder.rs", "backlog #402"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Enforce stable section order (role -> conventions -> workspace -> plan -> task -> volatile) with cache-control markers at tier boundaries so providers cache the stable prefix. Backlog #402.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-11: Implement prompt cache alignment`
- `tmp/archive/plan-audit-2026-09-23/10-token-optimization.md`

How to verify: Inspect section ordering and cache_control emission in SystemPromptBuilder.
