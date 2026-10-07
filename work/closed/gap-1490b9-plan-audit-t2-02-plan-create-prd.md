+++
id = "gap-1490b9"
kind = "gap"
title = "[plan-audit T2-02] `plan create --prd --depends-on --crate`"
status = "superseded"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-02: Add `plan create --prd --depends-on --crate`"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-02: Add `plan create --prd --depends-on --crate`"
anchors = ["cmd_plan_create", "backlog #405"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:20Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Superseded: PRDs were removed on 2026-10-02 (merge bfd36512f), so `plan create` has no PRD to link."
+++
Enrich plan create to wire PRD references, inter-plan dependencies and crate scope at scaffold time. Backlog #405.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-02: Add `plan create --prd --depends-on --crate``
- `tmp/archive/plan-audit-2026-09-23/13-cli-ux-gaps.md`

How to verify: roko plan create --help.
