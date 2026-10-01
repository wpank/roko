+++
id = "gap-51deff"
kind = "gap"
title = "In the rich topology a task's files are free between its executor and its gate, so an overlapping task can run in between"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-graph"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "8a88c6267"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-scheduler's report on gap-439794, branch work/gap-4d835d)"
anchors = ["crates/roko-graph/src/topology.rs::ProductionPlanTopology", "crates/roko-graph/src/engine.rs::execute_ready_queue", "crates/roko-graph/src/engine.rs::exclusion_conflict"]
lane = "rust-hot"
parent = "spec-a78d57"
links = { depends_on = ["gap-439794"], blocks = [], related = ["gap-4d835d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_gate_fails_closed_without_worktree' crates/roko-graph/src/cells/plan_gate.rs && cargo test -p roko-graph --lib plan_gate"
+++

## Problem

In the rich topology (`ProductionPlanTopology`, used when `rich_topology` is on), gap-439794 gives both a task's executor node and its gate node the task's `files` as `exclusive` paths. But a node holds its paths only while it runs: `execute_ready_queue` checks a queued node against the running nodes only. So when the executor finishes, the paths are free until the gate node starts. In that window the scheduler can admit an overlapping task's executor, which then edits the files that the gate is about to check, or is still editing them while the gate checks.

The comment in `topology.rs` says both nodes hold the files "so no overlapping task edits them meanwhile". That is not true for the window between the two nodes.

## Why it matters

The gate then checks a mix of two tasks' edits. The result is a false failure, or a pass credited to the wrong task (epic spec-a78d57).

## Where

- `crates/roko-graph/src/topology.rs::ProductionPlanTopology`: builds executor → gate (an edge taken on success), with `exclusive: task.files.clone()` on both nodes.
- `crates/roko-graph/src/engine.rs`: `execute_ready_queue` and `exclusion_conflict` admit a node by checking it against `running_nodes` only.

## Current state

On `work/gap-4d835d` (`c5466b5cb`). The test `executor_and_gate_hold_the_task_files` checks that both nodes carry the paths. It does not check that the paths stay held between the two nodes.

## Plan

1. Hold a task's paths as one lease, from the start of its executor until its gate settles: either the success boundary is reached, or the task fails for good. The lease must also survive a retry that sends the task back to its executor. Two ways to do it:
   - a lease keyed by the task's subgraph, kept in `execute_ready_queue`;
   - a link from the executor to the node that releases the paths.
2. A lease holder that is waiting for a slot must not deadlock other nodes. Like gap-439794's waiting nodes, it holds paths but no slot.
3. Add `a_task_holds_its_files_from_executor_to_gate`. Use two rich-topology tasks with overlapping files, and check that the second task's executor never starts between the end of the first task's executor and the end of its gate.

## Done when

- [ ] No overlapping node runs between a task's executor and its gate.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on gap-439794, which adds `Node.exclusive`. It is not on BASE yet.
- 2026-09-30, checked at `8a88c6267` (wk-scheduler): the premise no longer holds, so no lease was built.
  - Since `72d3c9823` (bug-50caf2), the gate judges the attempt's own isolated checkout.
    `PlanGateCell::attempt_checkout` (`cells/plan_gate.rs`) fails closed when `TaskAttempt.workspace` is `None`.
    `GraphTaskDispatcher` sets `workspace` only from a workspace-provider lease (`graph_task_dispatch.rs`, the
    `TaskAttempt` stamp), which exists only under `--worktree-per-task`.
  - So a rich-topology gate never reads the shared tree. In the shared tree it refuses, and with per-task worktrees no
    other task edits the checkout it judges. A task that runs between a task's executor and its gate cannot put a
    second task's edits into the verdict.
  - Change made: the `topology.rs` comment the item calls untrue now says this, and so does the conversion doc.
    The window itself is harmless, and gap-19e596 clears `exclusive` under `--worktree-per-task` anyway.
  - Suggest closing as `wontfix` (superseded by bug-50caf2). The `[[verify]]` test was not written, because it would
    assert a lease that is no longer needed.
- 2026-09-30 (coordinator): the verify now names bug-50caf2's plan-gate tests, which cover the executor-to-gate handoff that made this item's premise false; the originally named test was never written (no behaviour left to test).
