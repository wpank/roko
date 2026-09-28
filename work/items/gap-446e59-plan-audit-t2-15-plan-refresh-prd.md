+++
id = "gap-446e59"
kind = "gap"
title = "[plan-audit T2-15] `plan refresh` PRD drift check"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-15: Add `plan refresh` (PRD drift check)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-15: Add `plan refresh` (PRD drift check)"
anchors = ["crates/roko-cli/src/commands/plan.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Add a command that fingerprints PRD source files referenced by each plan and reports stale plans.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-15: Add `plan refresh` (PRD drift check)`

How to verify: roko plan --help for refresh.
