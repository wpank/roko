+++
id = "gap-9980c6"
kind = "gap"
title = "`roko run` drops --effort and --no-cascade on the Graph path, and --serve/--share are untested"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/run"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3d-roko-run"
anchors = ["crates/roko-cli/src/run.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs::GraphPlanRunParams", "crates/roko-cli/src/resolved_overrides.rs"]
links = { depends_on = [], blocks = [], related = ["gap-d60281"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'sed -n "/run_graph_plan(GraphPlanRunParams {/,/^    })/p" crates/roko-cli/src/run.rs | grep -qE "effort|cascade"'
+++

Since w3d, `roko run` executes its one-task plan with `run_graph_plan`. `GraphPlanRunParams` has no effort or cascade field, and only the model override (`cli_model_override`) is passed, so `--effort` and `--no-cascade` (resolved in `resolved_overrides.rs`) are accepted and then dropped. `roko plan run` has the same problem with `--effort` (gap-d60281). The `--serve` and `--share` flags of `roko run` have not been exercised since the switch to the Graph engine.

Fix: carry effort and cascade settings in `GraphPlanRunParams` (shared with gap-d60281), and add a smoke test for `--serve` and `--share`.
