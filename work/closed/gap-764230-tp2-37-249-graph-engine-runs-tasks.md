+++
id = "gap-764230"
kind = "gap"
title = "TP2-37 #249: Graph engine runs tasks in the shared checkout (no per-attempt worktree isolation)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Roko parity assessment"
discovered_from = "audit:tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Roko parity assessment"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1059", "crates/roko-cli/src/graph_task_dispatch.rs:926"]
links = { depends_on = [], blocks = [], related = ["gap-d58ae8"], supersedes = [], duplicate_of = "gap-4ec59f" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-4ec59f (still true: Graph per-task worktrees are opt-in only, crates/roko-cli/src/graph_execution/plan_runner.rs:1059-1073 and graph_task_dispatch.rs:926); the #140 serialized merge/conflict proof is tracked by gap-d58ae8"
+++
The 09-01 audit noted the Graph path was explicitly shared-root while Runner-v2 had attempt worktrees; with Graph now the sole engine, the 09-25 portal run wrote task output directly into the working tree. Needs #249 attempt lifecycle plus #140 serialized merge/conflict proof.

Imported without verification from:
- `tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Roko parity assessment`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-8. Self-modification is safe; the cost is elsewhere`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Changes applied in this session`

How to verify: Check whether Graph task dispatch creates attempt worktrees or edits the repo root.

Verified 2026-09-28: still true (graph_execution/plan_runner.rs:1059 opt-in isolation); duplicate of gap-4ec59f.
