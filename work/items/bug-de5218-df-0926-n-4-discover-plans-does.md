+++
id = "bug-de5218"
kind = "bug"
title = "DF-0926 N-4: `discover_plans` does not recurse; nested plan programmes invisible"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/plan_discovery"]
created = 2026-09-26
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-4. `discover_plans` does not recurse, so nested plan programmes are invisible"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-4. `discover_plans` does not recurse, so nested plan programmes are invisible"
anchors = ["crates/roko-cli/src/orchestrator/plan_discovery.rs::discover_plans", "crates/roko-cli/src/runner/plan_loader.rs::load_plans"]
links = { depends_on = [], blocks = [], related = ["bug-7f457a", "bug-9f340c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --lib -- nested_plan_set_is_discovered_with_its_group load_plans_matches_discovery_for_nested_plan_sets'

[closed]
at = 2026-09-28
commit = "725f21e05"
evidence = "Committed in 725f21e05 by the portal session. Re-checked 2026-09-28: the cited files are clean at HEAD and the named symbols and tests exist there (not rebuilt or retested here). crates/roko-cli/src/orchestrator/plan_discovery.rs:319-332 discover_plans walks nested plan sets via find_plan_dirs (test nested_plan_set_is_discovered_with_its_group at :817) and runner/plan_loader.rs:120-122 load_plans applies the same rule (test load_plans_matches_discovery_for_nested_plan_sets at :708); remaining plan-API residuals are bug-9f340c"
+++
plans/<set>/<plan>/tasks.toml runs via load_plans but is invisible to plan list, GET /api/plans and TUI F2 because orchestrator plan discovery scans one level; the two halves disagree on what a plan directory is.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-4. `discover_plans` does not recurse, so nested plan programmes are invisible`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Still open`
- `tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#Warnings and Data Quality Issues`

How to verify: roko plan list with a nested plan set.

Verified 2026-09-28: fixed in working tree - plan_discovery.rs:332 discover_plans recurses into nested sets.
