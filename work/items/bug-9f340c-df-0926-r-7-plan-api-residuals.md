+++
id = "bug-9f340c"
kind = "bug"
title = "Plan API residuals after directory-plan discovery"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/routes/plans"]
created = 2026-09-26
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
anchors = ["crates/roko-serve/src/routes/plans.rs find_plan", "create_plan", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-serve/src/routes/plans.rs::find_plan", "crates/roko-serve/src/routes/plans.rs::create_plan", "crates/roko-serve/src/routes/plans.rs::plans_dir", "crates/roko-cli/src/serve_runtime.rs:432"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
find_plan still backs nine handlers that 404 for directory plans; plans_dir change redirected them away from legacy .roko/plans; create_plan writes UUID JSON into source-controlled plans/; completed predicate differs from CLI; core logic untested (roko-serve cannot depend on roko-cli).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies`

How to verify: Call each plans route for a directory plan id.

Verified 2026-09-28 (static check against 3d0ee4d02): All R-7 residuals still hold (count now eight, not nine): find_plan (routes/plans.rs:1610) still backs 8 handlers (:483-:1302) and only probes flat {id}.json/.toml; plans_dir (:1755-1761) returns top-level plans/ whenever it exists, so legacy .roko/plans is never searched; create_plan still writes `{uuid}.json` into plans_dir (:208, :227-236). serve_runtime.rs:432 still sets `completed: task.status == "done"` while the CLI predicate is `"done" | "completed" | "passed" | "skipped"` (plan.rs:309). The core logic is still untested: serve_runtime.rs has only 3 extension-startup tests (:1338-1366), nothing calls RokoCliRuntime::load_plan_summary/load_plan_tasks in tests, and roko-serve/tests/plan_discovery.rs still drives a stub PlanDiscoveryRuntime that ignores `_workdir` (:91-136).
