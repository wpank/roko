+++
id = "bug-9f340c"
kind = "bug"
title = "Plan API residuals after directory-plan discovery"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/routes/plans"]
created = 2026-09-26
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
anchors = ["crates/roko-cli/src/serve_runtime.rs::task_to_dto", "crates/roko-cli/src/plan.rs:309", "crates/roko-serve/src/routes/plans.rs::plans_dir", "crates/roko-serve/src/routes/plans.rs::resolve_plan", "crates/roko-cli/src/serve_runtime.rs::load_plan_summary"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'completed: task.status == \"done\"' crates/roko-cli/src/serve_runtime.rs && cargo test -p roko-cli serve_runtime::tests::task_to_dto_treats_passed_and_skipped_as_completed"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:59Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:46Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Already fixed at BASE ebdc0f5d5; the item's 2026-10-01 note gives the evidence."
+++
find_plan still backs nine handlers that 404 for directory plans; plans_dir change redirected them away from legacy .roko/plans; create_plan writes UUID JSON into source-controlled plans/; completed predicate differs from CLI; core logic untested (roko-serve cannot depend on roko-cli).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies`

How to verify: Call each plans route for a directory plan id.

Verified 2026-09-28 (static check against 3d0ee4d02): All R-7 residuals still hold (count now eight, not nine): find_plan (routes/plans.rs:1610) still backs 8 handlers (:483-:1302) and only probes flat {id}.json/.toml; plans_dir (:1755-1761) returns top-level plans/ whenever it exists, so legacy .roko/plans is never searched; create_plan still writes `{uuid}.json` into plans_dir (:208, :227-236). serve_runtime.rs:432 still sets `completed: task.status == "done"` while the CLI predicate is `"done" | "completed" | "passed" | "skipped"` (plan.rs:309). The core logic is still untested: serve_runtime.rs has only 3 extension-startup tests (:1338-1366), nothing calls RokoCliRuntime::load_plan_summary/load_plan_tasks in tests, and roko-serve/tests/plan_discovery.rs still drives a stub PlanDiscoveryRuntime that ignores `_workdir` (:91-136).

**Partial fix landed 2026-09-29 (plan 04 T07):** `POST /api/plans` now writes `plans/<slug>/tasks.toml` (a directory plan) instead of `<uuid>.json`. The created plan is immediately visible to `GET /api/plans` (discovery-based listing) and passes `roko plan validate`. Verified: PASS "POST /api/plans creates a plan (201) and returns its slug", PASS "the created plan is a directory plan", PASS "the created plan is listed". The `find_plan` flat-file lookup backing the other nine handlers (resume, costs, gates, estimate, chat, reviews, diff, validate, source) is addressed by plan 03b T18; those handlers remain open.

Re-verified 2026-09-29 at d9e79e9d8. Fixed: find_plan was removed in 5c62bf0d4 and the seven id-based handlers now resolve plans through the runtime (routes/plans.rs::resolve_plan, :2514), so directory plans no longer 404; create_plan writes plans/<slug>/tasks.toml through runtime.create_plan (6c9b8dc7a). Still open: (1) crates/roko-cli/src/serve_runtime.rs:1614 task_to_dto sets completed only for status "done", while the CLI treats "done" | "completed" | "passed" | "skipped" as complete (crates/roko-cli/src/plan.rs:309); (2) plans_dir (routes/plans.rs:2500-2506) returns plans/ whenever it exists, so plans under legacy .roko/plans are not reachable there; (3) RokoCliRuntime::load_plan_summary / load_plan_tasks (serve_runtime.rs:421, :453) still have no direct tests (serve_runtime.rs has 5 tests, none call them).

## Notes

2026-10-01 (wk-runstate): implemented on work/find-8872ad; cargo verification deferred to the batch check. `task_to_dto` now sets `completed` with `crate::plan::task_status_is_complete`, the predicate the CLI plan listing (`summarize_plan_info`) uses, moved into a function both call: done, completed, passed or skipped. New test `serve_runtime::tests::task_to_dto_treats_passed_and_skipped_as_completed` loads a directory plan through `RokoCliRuntime::load_plan_tasks` and `load_plan_summary` and checks each status, and that the summary counts the same four tasks as done. That covers residual (3), the runtime's untested plan loading. Residual (2) was already fixed at BASE: `plans_dir` delegates to `roko_fs::workspace_plans::workspace_plans_dir` (1e0073605), which keeps a workspace that holds plans in `.roko/plans` there (test `plans_dir_keeps_a_legacy_workspace_in_dotted_roko`, routes/plans.rs:3977).
