# Task timing and exit reconciliation are wrong

- Severity: high
- Area: runner lifecycle / metrics
- Reproduced: 2026-07-11, run `run-1783781910584`, task `SH01-T02`

The runner logged `dispatch_ms=1649`, `agent_ms=399931`, and `gate_ms=189000`, for about 590.6 seconds of task work. The durable `task.attempt.completed.duration_ms` and ledger `task_completed.duration_ms` both recorded only `189000`, incorrectly presenting gate duration as total task duration. ETA, throughput, and per-task efficiency derived from the terminal record are therefore wrong.

The same attempt emitted raw `agent.exited` with `exit_code: null`, followed by normalized `agent.completed` with `exit_code: 0`. Durable views of the same process outcome disagree.

Use one attempt clock from dispatch through terminalization, preserve phase durations separately, and reconcile provider exit state before emitting either terminal event. Add an assertion that the sum of phase timings is consistent with total wall time and that raw/normalized exit codes cannot conflict.
