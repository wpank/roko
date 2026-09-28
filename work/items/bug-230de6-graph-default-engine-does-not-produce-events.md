+++
id = "bug-230de6"
kind = "bug"
title = "Graph (default) engine does not produce events.jsonl in mock plan runs"
status = "open"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "crates/roko-cli/tests/default_engine.rs:11"
discovered_from = "audit:crates/roko-cli/tests/default_engine.rs:11"
anchors = ["crates/roko-cli/tests/default_engine.rs::default_engine_does_real_work", "crates/roko-cli/src/graph_execution/plan_runner.rs:1159", "crates/roko-cli/src/graph_execution/plan_runner.rs:646", "crates/roko-cli/src/graph_execution/event_log.rs::run_recorded"]
links = { depends_on = [], blocks = [], related = ["bug-f7943a", "gap-8a1fb3", "gap-09e478"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --test default_engine -- --include-ignored default_engine_does_real_work'
+++
default_engine_does_real_work is ignored: 'requires engine-convergence wiring: Graph engine events.jsonl not yet produced by mock plan runs'. The default engine's per-run event log (used by status/diagnose/dashboard) may be missing for Graph runs.

Imported without verification from:
- `crates/roko-cli/tests/default_engine.rs:11`

How to verify: Run the test with --ignored; check whether graph_execution writes .roko/runs/<id>/events.jsonl for mock plans.

Verified 2026-09-28: still true - tests/default_engine.rs:11 is still #[ignore]; the Graph runner only opens a GraphEventLogger when --log-file is passed (graph_execution/plan_runner.rs:1158-1172), so a bare `roko plan run` writes no .roko/events.jsonl task.attempt records.

Verified 2026-09-28 (static check against 3d0ee4d02): crates/roko-cli/tests/default_engine.rs:10-11 is still #[ignore = "...Graph engine events.jsonl not yet produced by mock plan runs"]. run_graph_plan only records events when --log-file is given (graph_execution/plan_runner.rs:646-649 hands off to event_log::run_recorded; the GraphEventLogger block is now at :1129-1143), and no Graph-path code writes .roko/events.jsonl task.attempt records on a bare `roko plan run` (only runner/types.rs event enums and TUI readers reference them).
