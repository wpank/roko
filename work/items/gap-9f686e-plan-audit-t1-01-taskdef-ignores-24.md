+++
id = "gap-9f686e"
kind = "gap"
title = "[plan-audit T1-01] TaskDef ignores 24 routing-metadata fields from tasks.toml"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/task_parser"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-01: Wire TaskDef to parse routing metadata fields"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-01: Wire TaskDef to parse routing metadata fields"
anchors = ["crates/roko-cli/src/task_parser.rs TaskDefSerde", "DispatchContext", "backlog #403"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
category, reasoning_level, speed_priority, quality_profile, context_weight, tags, escalate_on_retry (24 roko_core::Task fields) are not parsed by TaskDefSerde, so all tasks get identical model/budget/context. Backlog #403 (#180).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-01: Wire TaskDef to parse routing metadata fields`
- `tmp/archive/plan-audit-2026-09-23/11-task-metadata-schema.md`

How to verify: Compare TaskDefSerde fields with roko_core::Task.
