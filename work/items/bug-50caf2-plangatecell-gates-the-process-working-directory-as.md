+++
id = "bug-50caf2"
kind = "bug"
title = "PlanGateCell gates the process working directory as attempt 0"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-graph/cells", "roko-gate"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-graph/src/cells/plan_gate.rs::PlanGateCell::build_request", "crates/roko-graph/src/cells/plan_gate.rs::PlanGateCell::execute", "crates/roko-graph/src/topology.rs::ProductionPlanTopology", "crates/roko-cli/src/graph_task_dispatch.rs:3365", "crates/roko-cli/src/runner/gate_adapter.rs::RunnerProductionGateAdapter"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "std::env::current_dir()" crates/roko-graph/src/cells/plan_gate.rs'

[[verify]]
command = "! grep -q 'std::env::current_dir()' crates/roko-graph/src/cells/plan_gate.rs && ! grep -q 'attempt_id: 0' crates/roko-graph/src/cells/plan_gate.rs && grep -qw 'fn plan_gate_fails_closed_without_worktree' crates/roko-graph/src/cells/plan_gate.rs && cargo test -p roko-graph plan_gate"
+++

## Problem

`PlanGateCell` is the `plan.gate` node in the rich plan topology (`roko plan run <dir> --rich-topology`). It does not
gate the task's work:

- It runs every rung in `std::env::current_dir()`, the directory the roko process started in
  (`crates/roko-graph/src/cells/plan_gate.rs:150`). It does not use the task attempt's worktree. With
  `--worktree-per-task`, the agent's changes live in a separate worktree, so the gate checks the main checkout.
- It sends `attempt_id: 0` on every request (`plan_gate.rs:86`). Retries of the same task look identical to the
  gate service. `RunnerProductionGateAdapter` derives `run_id` and `workspace_fingerprint` as
  `shared:<task>:<attempt>`, so every attempt collides.
- When every rung reports "skipped", the cell reports a pass: `overall_score = 1.0` and `passed = true`
  (`plan_gate.rs:201-205`). The adapter also turns "the pipeline produced no verdict for this rung" into a skip. A
  gate that checked nothing can therefore pass a task.

Expected: the gate runs in the worktree of the exact attempt the executor produced. It sends that attempt's id plus
plan and run context. It fails closed when the worktree or attempt is unknown, or when no rung actually ran.

## Why it matters

- Goal `core` ("Plan runs work reliably"). Gates are the trust boundary of the core loop. A gate that checks the
  wrong tree, or passes when nothing ran, lets unverified agent output reach merge.
- Today the defect is latent (see Current state). It goes live the moment someone wires the gate evaluator into
  rich-topology runs. That is the natural next step for making the rich topology real (`gap-a29711` covers
  production topology restore).
- Related: `gap-2e69b2` (attempt ids repeat across runs in efficiency events). The host executor also hard-codes
  attempt 0 (`graph_task_dispatch.rs:3372`).

## Where

- `crates/roko-graph/src/cells/plan_gate.rs`:
  - `PlanGateCell::build_request` (line 83): builds a `SharedGateRequest` with `attempt_id: 0` and only `plan_id`
    in `context`.
  - `Cell::execute` (line 139): resolves `worktree` from `std::env::current_dir()` (line 150), loops over
    `CANONICAL_RUNGS` (compile, lint, test), and treats all-skipped as a pass (lines 201-205).
  - The test `all_skipped_treated_as_pass` (line 445) pins the all-skipped-as-pass behaviour.
- `crates/roko-graph/src/topology.rs`:
  - `ProductionPlanTopology`: per task, `task-executor` is followed by `plan.gate` on the `Success` edge
    (around lines 293-327).
  - `build_gate_config` (line 452): passes only `task_id`, `plan_id`, `plan_dir` and `files`.
  - `register_topology_cells` (line 487): registers `PlanGateCell`.
- `crates/roko-core/src/foundation.rs::SharedGateRequest` (line 738): `attempt_id: u32`, `worktree_path`,
  `context: HashMap<String, String>`.
- `crates/roko-cli/src/runner/gate_adapter.rs`: `impl SharedGateEvaluator for RunnerProductionGateAdapter` (line 323).
  It maps the request to a `ProductionGateRequest`. Note that it sets `plan_id: request.plan_dir`.
- `crates/roko-cli/src/graph_task_dispatch.rs`: the host `task-executor`.
  - It acquires the worktree with `WorkspaceAttemptId { attempt: 0 }` (lines 3365-3373).
  - It releases the worktree with `WorkspaceReleasePolicy::Delete` on success (around line 3930) before it returns
    its output. The worktree is therefore gone before the `plan.gate` node runs.
- The entry point is `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`. With `ctx.rich_topology` it
  builds the graph from `ProductionPlanTopology` (line 1915) and starts the engine with
  `CellContext::new().with_run_id(..).with_pause_flag(..)` (line 2023).

## Current state

- Re-checked 2026-09-29 at HEAD `a17d9d766`. Both `plan_gate.rs:86` (`attempt_id: 0`) and `plan_gate.rs:150`
  (`current_dir()`) are unchanged. The last commit that touched the file is `9c6ec420c`.
- The bug is latent in production. No production code injects `CellContext.resources.gates`: the only
  `with_resources`/`gates: Some(..)` calls are in tests (`plan_gate.rs:335`, `cell.rs:375`). `run_one_plan` builds
  its `CellContext` without resources. A `--rich-topology` run therefore stops at the first gate node with
  "PlanGateCell: SharedGateEvaluator not injected", which fails closed. The wrong-tree check happens as soon as an
  evaluator is injected, for example `RunnerProductionGateAdapter`, which already implements `SharedGateEvaluator`.
- The default topology (no `--rich-topology`) does not use `PlanGateCell`. The host executor runs the verify steps
  and gates inside the worktree itself.
- The executor's output signal carries no worktree path and no attempt number. Nothing downstream can find the
  attempt.

## Plan

1. Carry the attempt identity from the executor to the gate. In the host executor (`graph_task_dispatch.rs`), tag
   the output `Signal`: `workspace.path`, `workspace.attempt`, `plan_id`, `task_id`, and `run_id` from
   `CellContext.run_id`. Use a real attempt number, not the hard-coded `0`: count re-executions of the node for this
   run, or read it from the engine's retry state. If this grows, split it into its own item.
2. Keep the worktree alive until the gate has run. The executor must not delete it on success when a `plan.gate`
   node follows. Options:
   - Recommended: `ProductionPlanTopology::build_executor_config` sets `keep_workspace = true`. The executor then
     skips the release on success, and the host releases the worktree after the plan finishes (or in a small host
     `plan.workspace-release` cell after the gate). This is the change the original notes asked for, and it keeps the
     gate an independent check.
   - Alternative: in the rich topology, `PlanGateCell` does no gating of its own. It only converts the verdict the
     executor already produced into a `GateVerdict` signal. This is simpler and avoids gating twice, but the gate is
     then no longer an independent step.
3. In `PlanGateCell::execute`, read `workspace.path` and `workspace.attempt` from the input signal tags. Return
   `Err(RokoError::Invalid(..))` when either is missing, or when the path does not exist. Delete the
   `current_dir()` fallback.
4. Change `build_request` to take the attempt id and the extra context: `run_id`, `plan_id` and the task title. Stop
   hard-coding `attempt_id: 0`.
5. Fail closed when no rung ran. If `rung_results` is empty, set `passed = false` and `overall_score = 0.0`, with an
   evidence string such as "no gate rung ran". Rename the test `all_skipped_treated_as_pass` to
   `all_skipped_fails_closed` and invert its assertions.
6. Optional, same area: in `gate_adapter.rs`, set `ProductionGateRequest.plan_id` from `context["plan_id"]`
   instead of `plan_dir`.
7. Add tests in `plan_gate.rs`:
   - `plan_gate_uses_worktree_from_executor_output`: a recording mock evaluator asserts that `worktree_path` and
     `attempt_id` come from the input tags.
   - `plan_gate_fails_closed_without_worktree`: an input with no `workspace.path` tag returns `Err`.

## Done when

- `plan_gate.rs` contains no `std::env::current_dir()` and no hard-coded `attempt_id: 0`.
- `PlanGateCell` gates the worktree named by the executor output, and errors when that is missing.
- A gate where every rung is skipped reports `passed = false`.
- In a rich-topology run with an evaluator injected, the gate sees the agent's changes. Confirm with a unit or
  integration test, or with a manual `--rich-topology --worktree-per-task` run.
- Verify: `! grep -q 'std::env::current_dir()' crates/roko-graph/src/cells/plan_gate.rs && ! grep -q 'attempt_id: 0' crates/roko-graph/src/cells/plan_gate.rs && grep -qw 'fn plan_gate_fails_closed_without_worktree' crates/roko-graph/src/cells/plan_gate.rs && cargo test -p roko-graph plan_gate`

## Notes

- Gates are a safety boundary, so fail closed everywhere: never fall back to the current directory, and never pass
  on zero rungs.
- `graph_task_dispatch.rs` is large and edited often. The executor worktree-lifetime change in step 2 is the risky
  part. Coordinate with items that touch the executor's acquire and release, such as `gap-36f3fb`, and run it in a
  worktree of its own.
- The severity is p1, but the path is only reachable with `--rich-topology` plus an injected evaluator, which no
  production code provides today. Fix it before wiring `resources.gates`, not after.
- Keep `CANONICAL_RUNGS` as they are. Which rungs to run is out of scope.

## Original notes


`PlanGateCell` builds its gate request with `attempt_id: 0` (`cells/plan_gate.rs:86`) and runs gates in `std::env::current_dir()` (`:150`) instead of the attempt's worktree.
In rich-topology plans the gate therefore verifies whatever tree the process started in, not the agent's changes; per a local audit, a run where every rung is skipped counts as a pass.
Fix: take worktree and attempt id from the executor's output (fail closed when absent) and send attempt id plus plan/run context with every gate request.
