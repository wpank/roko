+++
id = "gap-6daad9"
kind = "gap"
title = "plan_runner injects no CellResources, so --rich-topology still stops at every gate"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, branch work/bug-50caf2 at f0445319f)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["bug-50caf2"], blocks = [], related = ["bug-50caf2", "bug-8835bc", "bug-056b40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rich_topology_gates_run_with_cell_resources' crates/roko-cli/src/ && cargo test -p roko-cli --lib rich_topology_gates_run_with_cell_resources"
+++

## Problem

The rich topology's gate cells need `CellResources`: the gates to run and the workspace to run them in. On bug-50caf2's branch, the only place that provides them is inside task dispatch (`graph_task_dispatch/attempt_workspace.rs:276`, `ctx.clone().with_resources(…)`). `graph_execution/plan_runner.rs` injects none into the engine's context, so a `--rich-topology` run's gate cells have nothing to run, and the run stops at every gate.

## Why it matters

Integration and a whole-plan check (epic spec-a0e40a): `--rich-topology` can't run a plan end to end. Together with bug-8835bc, its gates are both unreachable and non-binding.

## Where

`plan_runner.rs`, which builds the engine's context, and the `CellResources` type in roko-graph.

## Plan

1. Have `plan_runner` build `CellResources` (the gate set and the plan workspace) and attach them to the engine's context for rich-topology runs.
2. Add `rich_topology_gates_run_with_cell_resources`.

## Done when

- [ ] A `--rich-topology` run reaches and runs its gates.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-50caf2's branch. Fix it with bug-8835bc, so that gates both run and gate.
