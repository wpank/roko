+++
id = "bug-db607b"
kind = "bug"
title = "DF-0921 ISSUE-30/31/32: Plan validator allows only 7 roles; reviewer role has no template"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/plan_policy"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects \"auditor\" as a valid role"
discovered_from = "audit:tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects \"auditor\" as a valid role"
anchors = ["VALID_ROLES", "AgentRole", "crates/roko-compose/src/templates/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Validator accepts implementer/researcher/strategist/architect/reviewer/quick-reviewer/scribe while 28+ AgentRole variants exist (auditor rejected); role 'reviewer' triggers PLAN_008 no-template warning. Fixes section left empty.

Imported without verification from:
- `tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects "auditor" as a valid role`
- `tmp/dogfood/2026-09-21-session.md#ISSUE-31: "reviewer" role has no template (PLAN_008 warning)`
- `tmp/dogfood/2026-09-21-session.md#ISSUE-32: Plan validator role allowlist is too restrictive`

How to verify: Compare VALID_ROLES with AgentRole and template files.
