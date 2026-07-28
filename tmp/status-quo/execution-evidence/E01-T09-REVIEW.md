# E01-T09 independent review

Reviewer verdict: ACCEPTED.

The initial candidate was rejected because it checked only events and the run
ledger. Commit `e16202ed1` repairs that defect: the test now verifies the
isolated bare default invocation and nonempty `.roko/episodes.jsonl`,
`.roko/state/executor.json`, and `.roko/state/state-snapshot.json`.
Those artifacts prove durable Runner-v2 execution and make the Graph dry-run
shape fail the regression.
