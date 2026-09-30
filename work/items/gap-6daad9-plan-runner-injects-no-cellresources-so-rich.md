+++
id = "gap-6daad9"
kind = "gap"
title = "plan_runner injects no CellResources, so --rich-topology still stops at every gate"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
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
- 2026-09-30 (wk-integrate): Implemented on `work/bug-453481` at `eddba5f1f` (after merging the working branch at `4d79f0016`); cargo verification deferred to the batch check.
  - In the worktree: `cargo check -p roko-cli -p roko-graph --lib --tests` and `cargo clippy -p roko-cli -p roko-graph -p roko-execution --no-deps -D warnings` clean; `cargo test -p roko-cli --lib` (plan_runner, attempt_workspace, rich_topology): 26 passed.
  - `plan_cell_resources` gives a rich-topology run `CellResources { gates: default_gate_adapter(), workspaces: <the run's WorktreeExecutionWorkspaceProvider> }`, the provider the dispatcher acquires from, and `plan_cell_context` attaches them to each plan's `CellContext`. The default topology gets empty resources.
  - `--rich-topology` without `--worktree-per-task` is refused before anything starts: the plan gate judges only an attempt's own worktree (bug-50caf2). The flag's help and `docs/v3/28-CLI.md` say so.
  - Test `rich_topology_gates_run_with_cell_resources` runs a one-task rich topology through `GraphEngine` with those resources and the real gate adapter: the gate runs its rungs in the handed-on worktree (the compile rung fails on a manifest cargo cannot parse), fails its task, and keeps the worktree instead of accepting it. `rich_topology_needs_worktree_per_task` covers the refusal.
  - Not done: the gate adapter uses `GatesConfig::default()`, not the run's `[gates]`; the rich topology's gate re-runs compile for each rung request (the adapter runs the pipeline up to that rung). A live `--rich-topology --worktree-per-task` run was not made.
