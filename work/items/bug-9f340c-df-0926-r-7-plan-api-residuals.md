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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
anchors = ["crates/roko-cli/src/serve_runtime.rs::task_to_dto", "crates/roko-cli/src/plan.rs:309", "crates/roko-serve/src/routes/plans.rs::plans_dir", "crates/roko-serve/src/routes/plans.rs::resolve_plan", "crates/roko-cli/src/serve_runtime.rs::load_plan_summary"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'completed: task.status == \"done\"' crates/roko-cli/src/serve_runtime.rs && cargo test -p roko-cli serve_runtime::tests::task_to_dto_treats_passed_and_skipped_as_completed"
+++
find_plan still backs nine handlers that 404 for directory plans; plans_dir change redirected them away from legacy .roko/plans; create_plan writes UUID JSON into source-controlled plans/; completed predicate differs from CLI; core logic untested (roko-serve cannot depend on roko-cli).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies`

How to verify: Call each plans route for a directory plan id.

Verified 2026-09-28 (static check against 3d0ee4d02): All R-7 residuals still hold (count now eight, not nine): find_plan (routes/plans.rs:1610) still backs 8 handlers (:483-:1302) and only probes flat {id}.json/.toml; plans_dir (:1755-1761) returns top-level plans/ whenever it exists, so legacy .roko/plans is never searched; create_plan still writes `{uuid}.json` into plans_dir (:208, :227-236). serve_runtime.rs:432 still sets `completed: task.status == "done"` while the CLI predicate is `"done" | "completed" | "passed" | "skipped"` (plan.rs:309). The core logic is still untested: serve_runtime.rs has only 3 extension-startup tests (:1338-1366), nothing calls RokoCliRuntime::load_plan_summary/load_plan_tasks in tests, and roko-serve/tests/plan_discovery.rs still drives a stub PlanDiscoveryRuntime that ignores `_workdir` (:91-136).

**Partial fix landed 2026-09-29 (plan 04 T07):** `POST /api/plans` now writes `plans/<slug>/tasks.toml` (a directory plan) instead of `<uuid>.json`. The created plan is immediately visible to `GET /api/plans` (discovery-based listing) and passes `roko plan validate`. Verified: PASS "POST /api/plans creates a plan (201) and returns its slug", PASS "the created plan is a directory plan", PASS "the created plan is listed". The `find_plan` flat-file lookup backing the other nine handlers (resume, costs, gates, estimate, chat, reviews, diff, validate, source) is addressed by plan 03b T18; those handlers remain open.

Re-verified 2026-09-29 at d9e79e9d8. Fixed: find_plan was removed in 5c62bf0d4 and the seven id-based handlers now resolve plans through the runtime (routes/plans.rs::resolve_plan, :2514), so directory plans no longer 404; create_plan writes plans/<slug>/tasks.toml through runtime.create_plan (6c9b8dc7a). Still open: (1) crates/roko-cli/src/serve_runtime.rs:1614 task_to_dto sets completed only for status "done", while the CLI treats "done" | "completed" | "passed" | "skipped" as complete (crates/roko-cli/src/plan.rs:309); (2) plans_dir (routes/plans.rs:2500-2506) returns plans/ whenever it exists, so plans under legacy .roko/plans are not reachable there; (3) RokoCliRuntime::load_plan_summary / load_plan_tasks (serve_runtime.rs:421, :453) still have no direct tests (serve_runtime.rs has 5 tests, none call them).
