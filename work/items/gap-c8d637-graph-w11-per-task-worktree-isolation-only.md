+++
id = "gap-c8d637"
kind = "gap"
title = "[graph W11] Per-task worktree isolation only opt-in (`--worktree-per-task`)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
discovered_from = "audit:tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1059", "crates/roko-cli/src/graph_task_dispatch.rs:926"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "gap-4ec59f" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-4ec59f (still true: --worktree-per-task is the only way to isolate Graph tasks, crates/roko-cli/src/graph_execution/plan_runner.rs:1059-1073, graph_task_dispatch.rs:926)"
+++
Marked Done via opt-in --worktree-per-task flag wiring WorktreeExecutionWorkspaceProvider; original finding was that Graph agents share the main workdir. Default Graph plan runs may still share one workdir.

Imported without verification from:
- `tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done`
- `tmp/archive/graph-audit/07-work-items.md#w11`

How to verify: Check whether Graph plan run isolates per-task worktrees by default or only with --worktree-per-task.

Verified 2026-09-28: still true (opt-in only); duplicate of gap-4ec59f.
