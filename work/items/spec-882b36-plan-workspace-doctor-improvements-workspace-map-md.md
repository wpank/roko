+++
id = "spec-882b36"
kind = "spec"
title = "Plan workspace-doctor-improvements: workspace-map.md snapshot + ignored-tests ledger check (2 ready)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/doctor"]
created = 2026-09-24
updated = 2026-09-28
source = "plans/workspace-doctor-improvements/plan.md"
discovered_from = "audit:plans/workspace-doctor-improvements/plan.md"
anchors = ["crates/roko-cli/src/doctor.rs", "generate_workspace_map_pub"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Ready plan: add workspace-map.md generation (generate_workspace_map_pub) to roko doctor and an ignored-tests ledger check; 47 #[ignore] tests exist with no ledger enforcement.

Imported without verification from:
- `plans/workspace-doctor-improvements/plan.md`
- `plans/workspace-doctor-improvements/tasks.toml`

How to verify: Check whether .roko/plans/ignored-tests.md exists and doctor reads it.
