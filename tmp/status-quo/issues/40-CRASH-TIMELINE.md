# E01 crash timeline

- Severity: critical
- Run: `run-1783775119910`
- Terminal condition: controlled wall-clock timeout, not panic or segfault

## Timeline

- 15:05:19.910 CEST: run starts with a fixed 3,600-second deadline.
- 15:15-15:43: multiple tasks and gates overlap in one plan worktree.
- 15:32:17: T08 gate fails, but no terminal T08 event is persisted.
- 15:36-15:40: four sibling agent completions are ignored because the plan is already in Gating.
- 15:40-15:43: T09, T15, and T16 pass; T07, T13, and T14 exhaust/fail.
- 15:43:23: last useful event. Runner logs that it is waiting on blocked DAG dependencies.
- 16:05:19.939: after 21m56s of silence, the fixed run timeout fires.
- 16:05:19.945: `run.completed` records failure after 3,600,007 ms.

## Causal chain

Per-plan concurrency was allowed despite `max_parallel=1`; task lifecycle was stored in a plan-scoped phase machine; a T08 gate completion was lost; stale retry attempts remained active; no-progress DAG state was treated as pending work; the runner silently waited until the global timeout.

