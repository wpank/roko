+++
id = "bug-5e7de4"
kind = "bug"
title = "plans/INDEX.md is inaccurate (omits portal-programme, zero task counts, counts junk as executable)"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/plan-index"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "6e5a2d55d"
source = "plans/INDEX.md:12"
discovered_from = "audit:plans/INDEX.md:12"
anchors = ["crates/roko-cli/src/index.rs::count_top_level_tasks", "crates/roko-cli/src/index.rs::collect_plan_index_entries", "plans/INDEX.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'portal-programme/01-backend-plan-service' plans/INDEX.md && grep -qw 'fn plans_index_lists_plans_inside_plan_sets' crates/roko-cli/src/index.rs && grep -qw 'fn plans_index_lists_unrunnable_plans_apart_from_the_backlog' crates/roko-cli/src/index.rs && cargo test -p roko-cli --lib index::tests::plans_index_"

[closed]
at = 2026-09-29
commit = "6e5a2d55d"
evidence = "roko plan index now finds plans with find_plan_dirs (plan sets such as portal-programme/, and plan.md-only dirs), counts [[tasks]] entries, and lists plans roko plan run cannot load (no tasks.toml, [[tasks]] instead of [[task]], no [meta]/meta.plan, no tasks, schema errors) under Not Runnable, outside the executable totals; the master index counts executable plans the same way. plans/INDEX.md regenerated from the tracked plans: 24 executable (18 in portal-programme/), 197 tasks, 6 fixtures; target/debug/roko plan index --check --workdir . passes. Rendering a copy of the main checkout's plans/ (demos/ set, junk moved to demos/_fixtures) gives 24 executable plus 12 fixtures under demos/. Tests: index::tests::plans_index_* (incl. plans_index_lists_plans_inside_plan_sets, plans_index_lists_unrunnable_plans_apart_from_the_backlog) pass."
+++
INDEX lists 15 executable plans/17 tasks incl. junk; shows 0 tasks for plans using [[tasks]] (scratch-test, test-fibonacci, audit-test, modal-test, test-check, test-greeting-module); omits nested portal-programme/01-08 and plan.md-only dirs. CLAUDE.md claims 124/124 across 30 plans.

Imported without verification from:
- `plans/INDEX.md:12`
- `plans/INDEX.md:28`

How to verify: Run `roko plan index --check` (or equivalent); inspect index generator's tasks.toml parsing and recursion.

Verified 2026-09-28 (static check against 3d0ee4d02): count_top_level_tasks (crates/roko-cli/src/index.rs:447-452) reads only the `task` key and returns (0,0,0) otherwise, so the six [[tasks]] plans (audit-test, modal-test, scratch-test, test-check, test-fibonacci, test-greeting-module) show 0 tasks in plans/INDEX.md:13-25, which still counts these scratch/test plans in 'Executable Total: 15 plans, 17 tasks'. Nested portal-programme plans (one-level read_dir, index.rs:347-376) and plan.md-only dirs (audit-gen*, flow-test-gen*, hello-world, test-plan-generation; skipped when tasks.toml is absent, index.rs:366-369) are still omitted. Only the 'CLAUDE.md claims 124/124' sub-point no longer applies (CLAUDE.md no longer carries that table).

Rechecked 2026-09-29 at d9e79e9d8. The committed plans/INDEX.md was regenerated in 5c62bf0d4 and no longer lists the scratch/test plans (Executable Total: 6 plans, 17 tasks). Those plans (audit-test, modal-test, scratch-test, test-check, test-fibonacci, test-greeting-module, test, example-hello-world, flow-test-exec) are untracked local directories and still exist on disk, so the next local plan index run adds them back with 0 tasks. Nothing changed in index.rs. The fix is planned as T02 of tmp/work-management/plans/work-graph/03-tooling-fixes, which closes this item and bug-7f457a; that plan has not run.
