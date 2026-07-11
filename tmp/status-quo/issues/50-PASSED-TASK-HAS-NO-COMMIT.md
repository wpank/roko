# Passed task has no commit

- Severity: critical
- Reproduction: E01-T15

Events and ledger mark T15 passed at 15:43:23, but the branch has no `[roko] ... E01-T15 completed` commit. HEAD is T16. Dirty files survive without task ownership, and the run snapshot claims T15 is completed.

A task must not become durably completed until its exact diff is committed or otherwise stored. Commit failure/missing diff must be a terminal persistence error, not a successful task.

