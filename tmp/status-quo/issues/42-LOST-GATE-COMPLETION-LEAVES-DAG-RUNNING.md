# Lost gate completion leaves DAG task running forever

- Severity: critical
- Reproduction: E01-T08

T08 was marked DAG-running and its post-agent gate completed failed at 15:32:17, but the run ledger contains no corresponding `task_failed` or `task_completed`. The transition itself failed because the shared plan was in Implementing when `GateFailed` expected another phase.

T08 therefore stayed nonterminal/running, and T10, which depends on T08, never started. The final snapshot does not persist enough task/DAG state to identify or repair this orphan.

Gate completion handling must terminalize the exact task attempt even when a higher-level phase transition fails. Transition errors cannot be warning-only.

