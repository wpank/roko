# Resume cannot reacquire an existing isolated worktree

- Severity: high
- Status: reproduced twice
- Area: recovery / git worktrees

## Observation

At `13:01:42`, resume failed with `isolated worktree missing for plan E01-execution-engine`. At `13:02:23`, the retry failed with the opposite condition: Git reported the plan branch was already checked out at `.roko/worktrees/E01-execution-engine`.

`git worktree list --porcelain` confirms that path is a registered worktree on `refs/heads/roko/plan/E01-execution-engine`. Error paths appear in `crates/roko-cli/src/runner/event_loop.rs:412`, `4567`, and `5259`.

## Impact

A valid snapshot cannot recover automatically after timeout/restart. Retrying can oscillate between "missing" and "already checked out," producing immediate failed runs with zero completed/failed task counts.

## Expected

Recovery should validate and reuse the registered worktree when its path and branch match the plan, or repair stale registration deterministically before dispatch.

