# 139 — Per-Plan Agent-Handle Map for Concurrent Task Execution

**Priority**: P2 — `max_concurrent_tasks` cannot safely be raised above 1 until there is a per-task agent-handle map; the current single-handle design prevents targeted cancellation and allows double-dispatch in edge cases.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/types.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §D-1 (suggested 118)

---

## Background

Runner-v2 currently uses a single `agent_handle: Option<AgentHandle>` to track the running agent. This limits concurrent task execution to 1: only one agent can be running at a time, and cancellation applies to that one agent regardless of which task it belongs to.

To enable `max_concurrent_tasks > 1`, the runner needs a `HashMap<TaskId, AgentHandle>` so that:
- Multiple agents can run concurrently, each tracked independently.
- When a task is cancelled, only its specific agent is cancelled (not all agents).
- When a task's gate fails and requires a retry, only that task's agent is cancelled.
- Double-dispatch detection is accurate: `task_dag.mark_running(task_id)` prevents duplicate dispatch at the DAG level, and the handle map prevents the runner from losing track of a handle.

Mori ran up to 15 concurrent agents in production. Roko's architecture supports concurrency (the `TaskDag` has wave-parallel tasks) but the handle map is the blocking implementation gap.

## Current State

- `crates/roko-cli/src/runner/event_loop.rs` — single `agent_handle: Option<AgentHandle>` field.
- `TaskDag::mark_running(task_id)` — prevents duplicate dispatch at DAG level.
- `max_concurrent_tasks` in `RunConfig` — defaults to 1; raising it above 1 is unsafe without the handle map.
- Cancellation: `agent_handle.cancel()` cancels the only handle (works for single-task; wrong for multi-task).

## Implementation Plan

1. **Replace `agent_handle` with `agent_handles: HashMap<TaskId, AgentHandle>`**:
   - On task dispatch: `agent_handles.insert(task_id, handle)`.
   - On task completion/failure: `agent_handles.remove(&task_id)`.
   - On targeted cancellation: `agent_handles.get(&task_id)?.cancel()`.

2. **Update dispatch loop**: When `max_concurrent_tasks > 1`, the event loop should dispatch up to `max_concurrent_tasks` tasks in parallel. Currently the loop dispatches one task, then waits for completion. The change: after dispatching a task, check if additional tasks are ready (wave-parallel with no handle yet) and dispatch them up to the concurrency limit.

3. **Concurrent completion handling**: The `tokio::select!` loop must handle completions from any active handle. Use `futures::future::select_all(active_handles.values().map(|h| h.wait()))` to wait for the first completion across all active tasks.

4. **Targeted cancellation**: When a `TuiCommand::CancelTask { task_id }` arrives (from TUI recovery keybindings, #119), look up and cancel only that task's handle.

5. **Runner state snapshot**: The snapshot must include the set of in-progress task IDs (tasks with handles) so that on resume, they are recognized as incomplete and not counted as completed.

6. **Guard on `max_concurrent_tasks`**: Until this item is done, add a runtime assertion that `max_concurrent_tasks <= 1`. After this item, raise the configurable ceiling to 4 and remove the assertion.

## Acceptance Criteria

1. `max_concurrent_tasks = 2` dispatches two tasks concurrently without double-dispatch.
2. When task A completes, task A's handle is removed; task B's handle is unaffected.
3. `CancelTask { task_id: "A" }` cancels task A without affecting task B.
4. Crash with two tasks in-flight; resume correctly restarts both tasks (no duplicate completion).
5. `cargo test -p roko-cli` passes with `max_concurrent_tasks = 2` in the test config.

## Verification Checklist

- [ ] Create a plan with two wave-parallel tasks; set `max_concurrent_tasks = 2`; verify both tasks start concurrently (visible in `episodes.jsonl` overlapping timestamps).
- [ ] Cancel task A while task B is running; verify task B completes normally.
- [ ] Crash with two tasks running; resume; verify both tasks restart without duplicate events.
- [ ] Set `max_concurrent_tasks = 1`; verify sequential execution (no concurrent dispatch).

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Replace `agent_handle` with `agent_handles: HashMap`; update dispatch and completion logic |
| `crates/roko-cli/src/runner/types.rs` | Add per-task handle type; update snapshot type |
| `crates/roko-cli/src/runner/resume.rs` | Restore in-progress task set from snapshot |
