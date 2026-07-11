# Snapshot backup cadence loses run history

- Severity: high

The latest backup predates the hour-long run, while the current snapshot was overwritten at timeout. There are no periodic recoverable checkpoints covering task completions, failures, dirty worktree ownership, or the onset of deadlock.

Use rotating atomic checkpoints at meaningful transitions and retain at least the last known-good, pre-terminal, and terminal snapshots with run IDs.

