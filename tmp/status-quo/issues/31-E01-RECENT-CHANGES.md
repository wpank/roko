# E01 Recent Changes Issues

Commits E01-T01 through E01-T06 on the `main` branch.

## High — Regression

### Resume infinite wait on failed+blocked dependencies (E01-T06)
- `event_loop.rs:3937-3944`: `seed_task_dag_from_run_state` seeds completed tasks only.
- `state.failed_tasks` not persisted in `RunStateSnapshot`, so `plan_failed_tasks` returns empty after restore.
- `task_dag.plan.failed` and `skipped` are never populated for pre-resume failures.
- `has_pending_dag_tasks` returns `true` for tasks blocked by prior failures → infinite wait with "waiting on blocked DAG tasks" log.

### `pending_gate_tasks` lost on resume (E01-T05)
- Purely in-memory HashMap. Not restored from snapshot.
- Gate fires against wrong task_id via `state.current_task` fallback (may be empty/wrong).

## Medium

### `Decompose` strategy sets `split_into` but runner never processes it (E01-T05)
- `revised_task_for_gate_failure` sets `split_into = Some(vec![...subtask ids...])`.
- Runner never reads `split_into`. Subtasks computed, persisted in snapshot, but never injected into task_index or task_dag. Decompose path is dead.

### `max_concurrent_plans` now unbounded above 4 (E01-T04)
- Old: `DEFAULT_RUNNER_MAX_CONCURRENT_PLANS = 4`.
- New: `plans.len().max(1)`. No explicit resource bound above 4.

### Help text still says "Graph Engine, default" (E01-T01)
- `main.rs:1351`: Not updated after changing `default_value` to `runner-v2`.

### Duplicate gate-failure replan logic (E01-T05)
- New `maybe_apply_gate_failure_plan_revision` in event_loop.rs.
- Old `maybe_emit_gate_failure_plan_revision` in orchestrate.rs.
- Different state stores, failure-key hashing, evidence structures. No unit tests for runner-v2 path.

## Low

### `active_agent_attempts` not cleared by `stop_all_agents` (E01-T04)
- Permits implicitly released by drop, but untested.

### merge.rs in-place mode removal
- Unstaged diff removes in-place branch → hard `Structural` failure. No migration for existing no-worktree workflows.
