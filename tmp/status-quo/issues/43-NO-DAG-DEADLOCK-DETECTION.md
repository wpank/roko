# No DAG deadlock detection

- Severity: critical
- Area: scheduler termination

`has_pending_dag_tasks` (`event_loop.rs:6603-6630`) asks only whether a nonterminal task exists. It does not determine whether any task can ever become ready. The no-ready branch around `event_loop.rs:4395-4404` silently no-ops.

After upstream failures and orphaned T08 state, all remaining work was blocked. T15/T16 logged `waiting on blocked DAG dependencies`, then the runner emitted nothing for almost 22 minutes.

The scheduler needs a quiescence check: no active attempts + no ready tasks + nonterminal tasks must produce explicit blocked/skipped propagation or a terminal deadlock error with dependency reasons.

