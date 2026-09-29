+++
id = "gap-38a529"
kind = "gap"
title = "WorkflowGraphController is never driven: serve shared runs and ACP build it and mark it skipped"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-execution/workflow", "roko-serve/shared_runs", "roko-acp"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3d-roko-run"
anchors = ["crates/roko-execution/src/workflow/controller.rs::WorkflowGraphController", "crates/roko-serve/src/routes/shared_runs.rs:519", "crates/roko-acp/src/runner.rs:668", "crates/roko-cli/src/explain.rs:268"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "WorkflowTermination::Skipped" crates/roko-serve/src/routes/shared_runs.rs crates/roko-acp/src/runner.rs'
+++

`WorkflowGraphController` (roko-execution) is constructed in two places, and both are stubs. Serve's shared-run route (routes/shared_runs.rs:510-530, commented "#276: WorkflowEngine deleted — resolve template and build a report stub") and the ACP runner (roko-acp/src/runner.rs:668-675) create a controller and immediately set `termination = WorkflowTermination::Skipped`, so no phase ever executes. `roko run` no longer uses it: w3d moved it to a one-task plan run through `run_graph_plan`. CLAUDE.md ("`roko run` uses graph templates via `WorkflowGraphController`") and `roko explain` (explain.rs:268) still describe the controller as the driver.

Fix: execute shared runs and ACP prompts for real (drive the controller's phases, or use `run_graph_plan` as `roko run` does), or remove the controller and correct the docs.
