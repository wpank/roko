+++
id = "bug-96aff4"
kind = "bug"
title = "roko prd, roko do and the cloud worker still call the removed Runner-v2 entry point"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/01-gaps-md-audit.md"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-cli/src/prd.rs:1161", "crates/roko-cli/src/commands/do_cmd.rs:997", "crates/roko-cli/src/worker/cloud.rs:568", "crates/roko-cli/src/runner/mod.rs::run"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test -z "$(grep -rn "runner::run(" crates/roko-cli/src --include="*.rs" | grep -v "^crates/roko-cli/src/runner/")"'

[[verify]]
command = 'test -z "$(grep -rn "runner::run(" crates/roko-cli/src --include="*.rs" | grep -v "^crates/roko-cli/src/runner/")"'

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "session roko-b6"
evidence = "Verify re-run here at HEAD 725f21e05: there are no runner::run( call sites outside runner/, and runner/mod.rs no longer defines run. do_cmd::run_plan_execution, prd::run_generated_plans and worker/cloud.rs call graph_execution::run_graph_plan. The portal session reports that crates/roko-cli/tests/runner_v2_guard.rs (no_runner_run_call_sites, plan_execution_callers_use_graph_engine) and tests/graph_plan_callers.rs pass; not re-run here, to avoid cargo contention with that running session."
+++

`crate::runner::run` is a deprecated stub that always bails ("the legacy Runner-v2 event loop has been removed").
Three production paths still call it: `prd.rs:1161`, `commands/do_cmd.rs:997` and `worker/cloud.rs:568` (deployed worker), so those flows fail at runtime after doing their setup.
Fix: move each to the Graph plan-run entry point (`graph_execution::run_graph_plan`, as serve now does), then delete the stub.

Fixed in 725f21e05: all three callers use `graph_execution::run_graph_plan`, and the `runner::run` stub is deleted.
