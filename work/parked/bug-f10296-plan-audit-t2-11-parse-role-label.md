+++
id = "bug-f10296"
kind = "bug"
title = "[plan-audit T2-11] parse_role_label recognizes only 10 of 28 roles"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-11: Fix `parse_role_label` to cover all 28 roles"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-11: Fix `parse_role_label` to cover all 28 roles"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs parse_role_label"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Architect, Auditor and other roles silently fall back to Implementer context limits because parse_role_label only matches 10 role labels.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-11: Fix `parse_role_label` to cover all 28 roles`

How to verify: Read parse_role_label match arms vs role enum.
