+++
id = "bug-96aff4"
kind = "bug"
title = "roko prd, roko do and the cloud worker still call the removed Runner-v2 entry point"
status = "in_progress"
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
+++

`crate::runner::run` is a deprecated stub that always bails ("the legacy Runner-v2 event loop has been removed").
Three production paths still call it: `prd.rs:1161`, `commands/do_cmd.rs:997` and `worker/cloud.rs:568` (deployed worker), so those flows fail at runtime after doing their setup.
Fix: move each to the Graph plan-run entry point (`graph_execution::run_graph_plan`, as serve now does), then delete the stub.

In progress (2026-09-28): a parallel session is migrating all three callers to `graph_execution::run_graph_plan`. Close with the commit once the verify command passes.
