# 54 — Graph Engine Runner-v2 Parity

**Priority**: P2 — Graph engine works for provider dispatch but lacks gate, replan, worktree isolation, and merge queue lifecycle, making it unsuitable for production plan execution
**Size**: XL (3-4 weeks)
**Crates**: `crates/roko-graph/` (`roko-graph`), `crates/roko-cli/` (`roko-cli`)
**Depends on**: None

---

## Background

Roko has two plan execution engines: Runner-v2 (`--engine runner-v2`, the default) and the Graph Engine (`--engine graph`). Runner-v2 is the production engine that handles the complete plan lifecycle: dispatch an agent task, run it in an isolated git worktree, run a multi-rung gate pipeline to verify the output, replan on gate failure, enqueue the worktree for merge, and persist full state so execution can resume after a crash. It is implemented in `crates/roko-cli/src/runner/event_loop.rs` (~15,000 lines).

The Graph Engine (`crates/roko-graph/src/engine.rs`, ~3,700 lines) was designed to be an alternative execution backend using a DAG of typed Cells. It handles topological ordering, parallel wave execution, Activity replay for crash-resume, conditional edge routing, cost tracking, a `MergeEnqueuer` trait (already wired), and a `CancellationToken`. It is invoked via `roko plan run --engine graph`.

However, the Graph Engine is missing seven lifecycle features that Runner-v2 has. Without these, Graph execution cannot verify task outputs, recover from bad outputs, isolate task work to avoid corrupting the shared workspace, complete the merge lifecycle, or handle human review before dispatch. This means the two engines are not interchangeable for real plan execution.

The `--approval` flag is currently hard-rejected at startup when `--engine graph` is used (see `validate_graph_execution_options` in `crates/roko-cli/src/commands/plan.rs`, line 1921).

---

## Current State

1. The Graph Engine is at `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs`. It is 3,679 lines. The `FlowHandle` struct is at line 262; `FlowHandle::cancel()` calls a `CancellationToken::cancel()` at line 295.

2. The `GraphSnapshot` struct (line 67) captures per-node status and Activity outputs but does NOT include gate verdicts, revision state, merge queue state, or plan metadata.

3. A `MergeEnqueuer` trait exists at line 54 and can be set via `GraphEngine::with_merge_queue()` at line 432. After a successful full execution, `enqueue()` is called at lines 762 and 1161. This is an enqueue-only call — there is no wait for merge completion, CI result, or merge failure handling.

4. The gate pipeline functions are in Runner-v2: `run_gate_once()` is defined at line 460 of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`. The function signature takes `GatesConfig`, `GateEffectRef`, plan/task IDs, `VerifyStep` list, and other parameters and returns a `GateCompletion`.

5. `build_gate_failure_plan_revision()` is defined at line 15445 of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`. It is called from line 15396 when a gate returns `GateFailureAction::NeedsReplan`. The replan cap is controlled by `gate_failure_replan_cap(config)` at line 15639.

6. `WorktreeManager` is used extensively throughout `event_loop.rs`. It is imported from `crates/roko-cli/src/runner/types.rs` line 25. Per-task worktree checkout, disk-space admission, and cleanup on success/failure are all handled in the runner before and after task dispatch.

7. The approval channel in Runner-v2 uses an IPC file written by the TUI and polled by the runner. The Graph Engine currently rejects `--approval` at line 1922 of `crates/roko-cli/src/commands/plan.rs`.

8. The `ActivityRecorder` and `ActivityReplayer` types are imported from `crates/roko-graph/src/replay.rs` and used in the Graph Engine (lines 336-339) for crash-resume of Activity node outputs only.

---

## Implementation Plan

These phases are ordered by impact. Each phase is independently shippable and testable.

### Phase 1: Gate pipeline after each Activity

After each Activity node completes successfully in the Graph Engine's execution loop (the `execute_with_status_tracking` method, around line 1520):

1. Check whether the completed node has a `verify_steps` field in its `Activity` definition. If it does, construct the gate inputs (`GateEffectRef`, `GatesConfig`, etc.) from the node's task metadata.
2. Call `run_gate_once()` (from `crates/roko-cli/src/runner/gate_dispatch.rs`) with those inputs. This is an `async fn` so the Graph Engine's execution must be in an async context (it already is — `execute_with_status_tracking` is `async`).
3. Store the `GateCompletion` result in a new `gate_results: HashMap<NodeId, GateCompletion>` field on `GraphEngine`.
4. If the gate fails, mark the node as `NodeStatus::Failed` with the gate failure reason. If it passes, proceed normally.
5. Persist gate results in `GraphSnapshot`. Add a `gate_results: HashMap<String, SerializableGateCompletion>` field to the `GraphSnapshot` struct (line 67) and populate it atomically after each gate run.

### Phase 2: Replan on gate failure

When a gate returns `GateFailureAction::NeedsReplan` (mapped from `RunnerFailureKind::Structural` in `gate_dispatch.rs` line 1192):

1. Call `build_gate_failure_plan_revision()` (line 15445 of `event_loop.rs`) to generate a revised task definition.
2. Replace the failed node's task definition in the in-memory `Graph` with the revised version.
3. Reset the node's status to `NodeStatus::Pending` and re-execute it.
4. Track the revision count per node. Respect `gate_failure_replan_cap(config)` (line 15639 of `event_loop.rs`) as the maximum revision count.
5. Persist the revision count in `GraphSnapshot` to survive crash/restart.

### Phase 3: Worktree isolation

Before dispatching each Activity node:

1. Acquire a worktree via `WorktreeManager` (same type used in Runner-v2, defined in `crates/roko-cli/src/runner/types.rs`). Use `WorktreeManager::checkout()` with disk-space admission (free-space check before checkout).
2. Set the Activity's working directory to the worktree path.
3. On gate success, stage the worktree for merge (Phase 4). On gate failure, clean up the worktree with `WorktreeManager::remove()`.
4. Store the worktree handle per node so Phase 7 (cancellation) can clean up in-flight worktrees.

### Phase 4: Merge queue integration

After gate success for a node that has a worktree:

1. Call `MergeEnqueuer::enqueue()` with the node's changed files. This is already wired for the full-execution success case; extend it to per-node post-gate success.
2. Add a `wait_for_merge: bool` option to `GraphEngine`. When `true`, block the node from being marked `Complete` until the merge result is received (via a channel returned by `enqueue()`). When `false` (default), fire-and-forget.
3. On merge failure, mark the node as `NodeStatus::Failed` with merge diagnostics and clean up the worktree.

### Phase 5: Full state persistence

Extend `GraphSnapshot` (line 67 of `engine.rs`) to include:

- `gate_results: HashMap<String, serde_json::Value>` — serialized gate completion per node.
- `revision_counts: HashMap<String, u32>` — replan revision count per node.
- `merge_state: HashMap<String, String>` — merge queue state per node (`"pending"`, `"merged"`, `"failed"`).
- `plan_metadata: Option<serde_json::Value>` — plan-level metadata for resume.

Write snapshots atomically (write to `<path>.tmp`, then `fs::rename`) after each node state change, not only at full-execution completion.

On resume, restore these fields alongside Activity replay so the engine re-enters the correct state for each node.

### Phase 6: Approval channel

1. When `--approval` is set, after the plan DAG is constructed but before any Activity is dispatched, print the full plan (node IDs, titles, roles, dependencies) to stderr and wait for user input on stdin (`[A]pprove / [C]ancel`).
2. Remove the `validate_graph_execution_options` rejection at line 1922 of `crates/roko-cli/src/commands/plan.rs` once this is wired.
3. Alternatively, implement the same IPC file polling used by Runner-v2 so the TUI's approval button works for both engines.

### Phase 7: Process-level cancellation on `FlowHandle::cancel()`

`FlowHandle::cancel()` (line 295) already calls `CancellationToken::cancel()`. The engine's loop already checks `cancel.is_cancelled()` between nodes (line 1561). Extend cleanup:

1. When cancellation is detected, call `WorktreeManager::remove()` for any worktrees held by in-flight nodes.
2. Write a terminal `GraphSnapshot` with the current node statuses before exiting the execution loop.
3. Send a cancellation signal to any in-flight Activity's agent process (via the provider's cancellation handle).

---

## Acceptance Criteria

1. After each Activity completes, `run_gate_once()` is called and the result is recorded in `GraphSnapshot`. A gate failure marks the node `Failed`.
2. A gate failure with `NeedsReplan` triggers `build_gate_failure_plan_revision()`, and the node is retried up to `gate_failure_replan_cap` times.
3. Each Activity executes in an isolated git worktree, not in the shared workspace.
4. On gate success, the worktree is enqueued for merge; on gate failure, the worktree is cleaned up.
5. `GraphSnapshot` includes gate results, revision counts, and merge state, and a resumed execution restores these fields correctly.
6. `roko plan run --engine graph --approval` no longer rejects at startup and prompts for approval before dispatch.
7. `FlowHandle::cancel()` kills in-flight agent processes and cleans up all held worktrees.
8. `cargo test -p roko-graph` passes with all existing tests.
9. A representative plan (e.g., a 3-task plan from `plans/`) produces the same final outcome via `--engine graph` and `--engine runner-v2`.

---

## Verification Checklist

- [ ] Run a 2-task plan with `--engine graph`. Confirm `.roko/immune/` and gate-related logs appear after each task.
- [ ] Introduce a deliberate compile error in a task. Confirm the gate catches it and the node is marked `Failed`.
- [ ] Enable `replan_on_gate_failure = true` in `roko.toml`. Confirm a failing gate triggers a retry with a revised task.
- [ ] Run `roko plan run --engine graph --approval`. Confirm the approval prompt appears before any agent dispatch.
- [ ] Press Ctrl+C during a run. Confirm no orphaned worktrees remain and the snapshot records the cancelled state.
- [ ] Resume a cancelled run (`--resume-plan`). Confirm previously completed nodes are not re-executed.
- [ ] Run `cargo test -p roko-graph` — all pass.

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs` | Add gate invocation after each Activity; add replan loop; add worktree acquisition/cleanup; extend `GraphSnapshot` with gate/revision/merge state; write snapshots atomically after each node; implement process-level cancellation cleanup |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` | Remove `validate_graph_execution_options` rejection of `--approval` once Phase 6 is wired (line 1922) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/replay.rs` | Extend `ActivityRecorder` to record gate and revision state alongside Activity outputs |
