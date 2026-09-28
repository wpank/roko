+++
id = "bug-f43cf8"
kind = "bug"
title = "DF-0925 P0-2: Non-implementer roles are denied the tools their tasks need"
status = "open"
triage = "verified"
severity = "p0"
subsystem = ["roko-agent/safety"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P0-2. Every non-implementer role is denied the tools its task needs"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P0-2. Every non-implementer role is denied the tools its task needs"
anchors = ["crates/roko-agent/src/dispatcher/tool_selector.rs::tools_for_role", "crates/roko-agent/src/safety/contract.rs::BUNDLED_CONTRACTS", "crates/roko-agent/src/safety/contracts/architect.yaml", "crates/roko-cli/src/plan_generate.rs:311", "crates/roko-cli/src/plan_validate.rs:985"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib menus_stay_within_contracts"

[[verify]]
command = "cargo test -p roko-agent --lib quick_reviewer_contract_reads_but_never_edits"

[[verify]]
command = "cargo test -p roko-cli role_tool_table_is_derived_from_enforced_capabilities"
+++
architect maps to the reviewer template and is denied write tools in roles.rs, tool_selector.rs and architect.yaml; quick-reviewer has no contract (deny-all fallback); reviewer has no AgentRole (gets implementer prompt, write-denied). plan_generate.rs tells generators architect can write.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P0-2. Every non-implementer role is denied the tools its task needs`

How to verify: Validate a plan with an architect task that lists files; check tool allowlist at dispatch.

Verified 2026-09-28: still true at HEAD 91b4745f8. HEAD's plan_generate.rs:322 still advertises architect as read/write/exec, the architect menu includes exec, quick-reviewer has no bundled contract, and reviewer has no role mapping. A fix is visibly in progress in the uncommitted working tree (concurrent session). It adds crates/roko-agent/src/safety/contracts/quick-reviewer.yaml (untracked) with its `BUNDLED_CONTRACTS` entry and `RoleCapabilities` (safety/contract.rs), makes the architect menu read-only to match architect.yaml (dispatcher/tool_selector.rs), makes plan_generate.rs:311 tell generators that architect cannot change files, and maps reviewer to `QuickReviewer` (plan_validate.rs:985-986). The verify tests exist only in that working tree. Close this once the fix is committed and they pass.
