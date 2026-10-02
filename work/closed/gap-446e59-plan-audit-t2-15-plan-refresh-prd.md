+++
id = "gap-446e59"
kind = "gap"
title = "[plan-audit T2-15] `plan refresh` PRD drift check"
status = "superseded"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-15: Add `plan refresh` (PRD drift check)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-15: Add `plan refresh` (PRD drift check)"
anchors = ["crates/roko-cli/src/commands/plan.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:21Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Superseded: PRDs were removed (merge bfd36512f), so there is no PRD to drift from; `roko plan regenerate` works from the plan's own plan.md."
+++
Add a command that fingerprints PRD source files referenced by each plan and reports stale plans.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-15: Add `plan refresh` (PRD drift check)`

How to verify: roko plan --help for refresh.
