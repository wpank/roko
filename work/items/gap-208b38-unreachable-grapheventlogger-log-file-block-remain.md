+++
id = "gap-208b38"
kind = "gap"
title = "Unreachable GraphEventLogger --log-file block remains in the Graph plan runner"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/graph_execution/event_log.rs::run_recorded"]
links = { depends_on = [], blocks = [], related = ["bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "GraphEventLogger::open" crates/roko-cli/src/graph_execution/plan_runner.rs'
+++

`run_graph_plan` hands every run with `--log-file` to `event_log::run_recorded` (plan_runner.rs:702-705), which writes the JSONL event log and runs the plan with no `log_file` (event_log.rs:126-133). The older block that opens `runner::structured_log::GraphEventLogger` for the same flag (plan_runner.rs:1268) can therefore never see a log file.

Fix: delete the block, and `GraphEventLogger` if nothing else uses it, so there is one `--log-file` writer.
