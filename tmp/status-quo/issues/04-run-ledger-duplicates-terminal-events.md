# Run ledger duplicates terminal events

- Severity: medium
- Status: reproduced
- Area: persistence / accounting

## Observation

`.roko/state/run-ledger.jsonl` contains repeated `task_completed` events for the same plan/task in a single run sequence. Examples include two E01-T01 completions at `09:42:11` and `09:42:12`, then another two at `09:48:22` and `09:48:23`. The current run again emitted two successful E01-T01 gate outcomes at `13:06:51` and `13:06:57`.

The duplicates align with the invalid preflight transition described in issue 01, but persistence currently accepts them without idempotency protection.

## Impact

Ledger-derived success rates, duration statistics, task counts, and cost attribution can be inflated. Replay consumers cannot assume one terminal event per attempt.

## Expected

Terminal persistence should be idempotent by stable run/plan/task/attempt identity, even if the state machine accidentally submits a duplicate.

## Crash-run evidence

The opposite failure also occurs: T08 has a task start and failed gate but no terminal task record. The ledger stops before the run timeout and has no run ID on task records, while events contain 13 attempt starts versus 10 terminals. Persistence needs both deduplication and completeness invariants.
