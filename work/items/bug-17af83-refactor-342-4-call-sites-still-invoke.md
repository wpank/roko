+++
id = "bug-17af83"
kind = "bug"
title = "[refactor #342] 4 call sites still invoke the deprecated runner::run() stub (fail at runtime)"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli/runner"]
created = 2026-09-14
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#call-sites-using-deprecated-runnerrun-stub-342"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#call-sites-using-deprecated-runnerrun-stub-342"
anchors = ["crates/roko-cli/src/prd.rs::run_generated_plans", "crates/roko-cli/src/commands/do_cmd.rs::run_plan_execution", "crates/roko-cli/src/worker/cloud.rs:568", "crates/roko-cli/src/runner/mod.rs::run"]
links = { depends_on = [], blocks = [], related = ["find-5ffca2", "gap-f1feaf"], supersedes = [], duplicate_of = "bug-96aff4" }

[closed]
at = 2026-09-28
evidence = "duplicate of bug-96aff4 (still true at 91b4745f8 + working tree: 3 runner::run( call sites remain at prd.rs:1161, commands/do_cmd.rs:997 (also reached by roko develop via develop.rs:119) and worker/cloud.rs:568; serve_runtime.rs was migrated, guarded by tests/runner_v2_guard.rs::serve_runtime_uses_graph_engine)"
+++
prd.rs run_generated_plans (auto-plan after PRD publish), serve_runtime.rs HTTP plan execution, commands/do_cmd.rs `roko do`, and worker/cloud.rs cloud worker call the deprecated runner::run() stub, which returns an error at runtime. Tracked as #342; P0 in parity doc.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#call-sites-using-deprecated-runnerrun-stub-342`
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#1-migrate-the-4-remaining-runnerrun-call-sites-p0`

How to verify: grep for runner::run( callers; the in-flight serve plan-execution work (serve_runtime.rs, graph_execution/plan_runner.rs) may have migrated some.

Verified 2026-09-28: still true but duplicate of bug-96aff4 - runner::run( callers at crates/roko-cli/src/prd.rs:1161, crates/roko-cli/src/commands/do_cmd.rs:997, crates/roko-cli/src/worker/cloud.rs:568 (serve_runtime.rs migrated).
