# Plan max_parallel is ignored

- Severity: high
- Status: reproduced
- Area: scheduler / resource control

## Observation

The plan metadata declares `max_parallel = 1`. At 15:15-15:16 the Roko parent had three live Codex children corresponding to E01-T07, T08, and T09 plus a live T11 cargo preflight. `.roko/runtime/agent-pids.json` listed all three agent PIDs.

The log shows overlapping dispatches: T07 remained alive after T08 was spawned at 13:13:26, T09 at 13:15:13, and T11 preflight at 13:15:23.

## Impact

The run exceeds plan-author resource intent, increases memory/CPU pressure, and permits concurrent edits in the same isolated plan worktree.

## Expected

Effective task concurrency should be the minimum of global runner capacity and each plan's `max_parallel`, with preflight/gate jobs included in the same per-plan accounting.

## Crash impact

This was not only a resource-policy violation. Concurrent T15/T16 agents shared one plan-scoped phase machine and one worktree. T16 moved the plan to Gating, causing T15's completion to be ignored; similar ignored completions orphaned T07, T13, and T14. The concurrency defect activated the terminal deadlock.
