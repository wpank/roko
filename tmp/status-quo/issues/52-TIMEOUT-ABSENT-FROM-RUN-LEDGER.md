# Timeout absent from run ledger

- Severity: high
- Area: durable audit trail

`.roko/state/run-ledger.jsonl` ends with T15 completion at 15:43:23. It contains no timeout, run failure, run summary, blocked tasks, or cleanup outcome at 16:05:19. Only `events.jsonl` records `run.completed`.

Ledger records also lack run IDs, making repeated E01 task records across runs hard to partition safely.

Every ledger entry needs a run ID and attempt ID. Terminal run/plan outcomes must be appended and fsynced before returning an error.

