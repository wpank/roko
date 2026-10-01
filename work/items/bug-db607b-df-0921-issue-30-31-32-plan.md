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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects \"auditor\" as a valid role"
discovered_from = "audit:tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects \"auditor\" as a valid role"
anchors = ["crates/roko-cli/src/task_parser.rs::PLAN_TASK_ROLES", "crates/roko-cli/src/task_parser.rs::validate_against_schema", "crates/roko-cli/src/plan_validate.rs::parse_task_role", "crates/roko-cli/src/plan_generator.rs:598", "crates/roko-cli/src/prd.rs::validate_and_fix_generated_plan", "crates/roko-cli/src/run.rs:397", "crates/roko-compose/src/templates/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'sed -n "/pub const PLAN_TASK_ROLES/,/];/p" crates/roko-cli/src/task_parser.rs | grep -q "\"auditor\"" && sed -n "/pub const PLAN_TASK_ROLES/,/];/p" crates/roko-cli/src/task_parser.rs | grep -q "\"reviewer\"" && grep -q "VALID_ROLES: &\[&str\] = crate::task_parser::PLAN_TASK_ROLES" crates/roko-cli/src/prd.rs && grep -q "VALID_ROLES: &\[&str\] = crate::task_parser::PLAN_TASK_ROLES" crates/roko-cli/src/plan_generator.rs && cargo test -p roko-cli --lib an_auditor_task_passes_schema_validation'
+++
Validator accepts implementer/researcher/strategist/architect/reviewer/quick-reviewer/scribe while 28+ AgentRole variants exist (auditor rejected); role 'reviewer' triggers PLAN_008 no-template warning. Fixes section left empty.

Imported without verification from:
- `tmp/dogfood/2026-09-21-session.md#ISSUE-30: Plan validator rejects "auditor" as a valid role`
- `tmp/dogfood/2026-09-21-session.md#ISSUE-31: "reviewer" role has no template (PLAN_008 warning)`
- `tmp/dogfood/2026-09-21-session.md#ISSUE-32: Plan validator role allowlist is too restrictive`

How to verify: Compare VALID_ROLES with AgentRole and template files.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): ISSUE-31 is fixed in 725f21e05: plan_validate.rs:986 parses 'reviewer' as QuickReviewer, which has a template (role_has_compose_template :956-969), so PLAN_008 no longer fires for it (plan_validate.rs:611-625). ISSUE-30/32 remain: the schema allowlist is still the 7 roles in task_parser.rs:654-662 (PLAN_TASK_ROLES, used at :957/:983), so 'auditor' is rejected as an unknown role with a PLAN_035 Error (plan_validate.rs:368-374) and `plan run` bails (runner/plan_loader.rs:72-83), even though parse_task_role (:984) and the bundled auditor.yaml contract accept it. The other allowlists still disagree: plan_generator.rs:598 includes auditor but not reviewer, and prd.rs:2726 has 6 roles.

Rechecked 2026-09-29 at d9e79e9d8: the 'reviewer' template part stays fixed (725f21e05). The allowlist part is unchanged. task_parser.rs:654-662 PLAN_TASK_ROLES still lists 7 roles. validate_against_schema (:957, :985) therefore still rejects 'auditor' as PLAN_035, and `roko run --role` (run.rs:397) uses the same list. The other allowlists still disagree. plan_generator.rs:598 accepts auditor/conductor/critic/refactorer/auto-fixer but not reviewer. prd.rs:2820 (validate_and_fix_generated_plan) accepts only 6 roles and silently rewrites any other role, including reviewer and auditor, to implementer.

## Notes

- 2026-10-01 (wk-specq): implemented on work/gap-404fdb; cargo verification deferred to the batch check.
  `PLAN_TASK_ROLES` now lists every role with a bundled safety contract: the 7 before plus `auditor` and
  `auto-fixer`. `prd.rs` (`validate_and_fix_generated_plan`) and `plan_generator.rs` read it in place of their
  own lists, as `validate_against_schema` and `roko run --role` already did. So `auditor` passes validation,
  `reviewer` is no longer rewritten to `implementer`, and generation no longer keeps `conductor`, `critic` or
  `refactorer`, which have no contract and which validation rejects.
- The verify command changed: prd.rs no longer has a literal list to grep. It now checks that both files read
  `PLAN_TASK_ROLES` and that the list has `auditor` and `reviewer`. It also runs
  `an_auditor_task_passes_schema_validation`. `every_plan_role_has_a_contract_that_agrees_on_write` covers the
  two added roles' contracts.
