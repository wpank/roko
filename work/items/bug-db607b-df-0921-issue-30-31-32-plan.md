+++
id = "bug-db607b"
kind = "bug"
title = "Plan validator allows only 7 roles; reviewer role has no template"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/plan_policy"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects \"auditor\" as a valid role"
discovered_from = "audit:tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects \"auditor\" as a valid role"
anchors = ["VALID_ROLES", "AgentRole", "crates/roko-compose/src/templates/", "crates/roko-cli/src/task_parser.rs::PLAN_TASK_ROLES", "crates/roko-cli/src/task_parser.rs::validate_against_schema", "crates/roko-cli/src/plan_validate.rs::parse_task_role", "crates/roko-cli/src/plan_generator.rs:598", "crates/roko-cli/src/prd.rs:2726"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Validator accepts implementer/researcher/strategist/architect/reviewer/quick-reviewer/scribe while 28+ AgentRole variants exist (auditor rejected); role 'reviewer' triggers PLAN_008 no-template warning. Fixes section left empty.

Imported without verification from:
- `tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects "auditor" as a valid role`
- `tmp/dogfood/2026-09-21-session.md#ISSUE-31: "reviewer" role has no template (PLAN_008 warning)`
- `tmp/dogfood/2026-09-21-session.md#ISSUE-32: Plan validator role allowlist is too restrictive`

How to verify: Compare VALID_ROLES with AgentRole and template files.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): ISSUE-31 is fixed in 725f21e05: plan_validate.rs:986 parses 'reviewer' as QuickReviewer, which has a template (role_has_compose_template :956-969), so PLAN_008 no longer fires for it (plan_validate.rs:611-625). ISSUE-30/32 remain: the schema allowlist is still the 7 roles in task_parser.rs:654-662 (PLAN_TASK_ROLES, used at :957/:983), so 'auditor' is rejected as an unknown role with a PLAN_035 Error (plan_validate.rs:368-374) and `plan run` bails (runner/plan_loader.rs:72-83), even though parse_task_role (:984) and the bundled auditor.yaml contract accept it. The other allowlists still disagree: plan_generator.rs:598 includes auditor but not reviewer, and prd.rs:2726 has 6 roles.
