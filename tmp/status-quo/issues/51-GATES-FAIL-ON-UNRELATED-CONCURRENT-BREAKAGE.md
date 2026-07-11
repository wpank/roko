# Gates fail on unrelated concurrent breakage

- Severity: high
- Area: verification isolation

T13's focused MCP test failed because other concurrent changes added `RunStateSnapshot.replan_ledger` and `revised_tasks` without updating constructors in `resume_cycle_e2e.rs:63` and `runner/resume.rs:311`. T07's task-specific worktree tests passed while the default full-repo gate failed.

The runner attributes shared-tree failures to whichever task's gate happens to run. This wastes retries, corrupts model learning, and can reject correct focused work.

Run task gates against the task's base plus owned diff. Report separately any baseline failure that predates the task.

