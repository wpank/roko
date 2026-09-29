+++
id = "gap-19e596"
kind = "gap"
title = "Turn file exclusion off under --worktree-per-task, where tasks do not share a tree"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-scheduler's report on gap-439794 (plan step 3), branch work/gap-4d835d)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunContext", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
lane = "rust-hot"
parent = "spec-a78d57"
links = { depends_on = ["gap-439794"], blocks = [], related = ["gap-4ec59f", "spec-f830c4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn worktree_per_task_clears_exclusive_paths' crates/roko-cli/src/ && cargo test -p roko-cli --lib worktree_per_task_clears_exclusive_paths"
+++

## Problem

With `--worktree-per-task`, each task gets its own worktree, so tasks never share a tree. gap-439794's file exclusion (`c5466b5cb` on `work/gap-4d835d`) still applies there, so tasks whose declared files overlap run one at a time. That is safe, but it throws away the parallelism that per-task worktrees pay for. The overlap is resolved when the branches merge anyway.

## Why it matters

Per-task worktrees are the planned default for isolation (gap-4ec59f) and for integration (spec-f830c4). Serializing overlapping tasks there costs throughput and buys nothing (epic spec-a78d57).

## Where

`crates/roko-cli/src/graph_execution/plan_runner.rs`:

- `PlanRunContext` has no worktree flag today. The flag is on `GraphPlanRunParams::worktree_per_task`.
- `run_one_plan` builds the graph with `plan_to_graph` or `ProductionPlanTopology`.

## Current state

gap-439794 skipped its plan step 3 because another worker owned `plan_runner.rs`. Its note says: "Under `--worktree-per-task`, `run_one_plan` should clear each node's `exclusive`, and `PlanRunContext` needs the flag for that."

## Plan

1. Add `worktree_per_task: bool` to `PlanRunContext`, and set it where the context is built.
2. In `run_one_plan`, once the graph is built on either path, clear every node's `exclusive` if the flag is on.
3. Add `worktree_per_task_clears_exclusive_paths`, covering both the simple and the rich topology.

## Done when

- [ ] Under `--worktree-per-task`, no node carries `exclusive` paths. Without it, nodes keep them.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on gap-439794, which adds `Node.exclusive`. It is not on BASE yet.
