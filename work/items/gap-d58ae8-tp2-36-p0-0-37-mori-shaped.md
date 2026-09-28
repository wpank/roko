+++
id = "gap-d58ae8"
kind = "gap"
title = "TP2-36 P0.0/37: Mori-shaped workflow contract unproven end-to-end"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-14
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control"
discovered_from = "audit:tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control"
anchors = ["crates/roko-cli/src/runner/queue_manifest.rs::QueueManifest", "crates/roko-cli/src/graph_execution/delivery.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs:1197", "crates/roko-cli/src/graph_execution/plan_runner.rs:1059"]
links = { depends_on = [], blocks = [], related = ["gap-4ec59f", "gap-0001a1", "gap-7c9e48", "gap-c135ba"], supersedes = [], duplicate_of = "" }
+++
No single proven path for queue selection/milestone unlock, dependency-ready scheduling, bounded parallel attempts in distinct worktrees, serialized merge with conflict recovery, durable restart and batch checkpoints; the parity fixture (2 milestones, conflict, kill/restart, pause/resume) was nev...

Imported without verification from:
- `tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control`
- `tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Required parity evidence`
- `tmp/tui-parity2/00-INDEX.md`
- `tmp/dogfood/2026-09-20-final-session.md#P1 (High)`

How to verify: Build the 37 parity fixture against the Graph engine and record which steps fail.

Verified 2026-09-28: still unproven - queue manifests only feed `roko plan queue show/validate/init` (commands/plan.rs:1551-1658), plans run sequentially (graph_execution/plan_runner.rs:1197), per-task worktrees are opt-in (plan_runner.rs:1059), and no parity fixture test exists in crates/roko-cli/tests. runner/queue_manifest.rs and runner/merge.rs still exist under crates/roko-cli/src/runner/ (the import warning was wrong). Absorbs gap-28ee5a.
