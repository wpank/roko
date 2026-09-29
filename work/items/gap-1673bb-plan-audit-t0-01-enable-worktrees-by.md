+++
id = "gap-1673bb"
kind = "gap"
title = "[plan-audit T0-01] Enable worktrees by default for plan execution"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/orchestrator"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-01: Enable worktrees by default"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-01: Enable worktrees by default"
anchors = ["crates/roko-cli/src/orchestrator/executor/mod.rs::ExecutorConfig::default_use_worktrees"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "gap-4ec59f" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-4ec59f (same backlog #400; still true: crates/roko-cli/src/orchestrator/executor/mod.rs:225-227 default_use_worktrees() returns false - executor/config.rs was folded into executor/mod.rs)"
+++
ExecutorConfig::default_use_worktrees() returns false, so concurrent tasks share the working directory with no filesystem isolation. Backlog #400.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-01: Enable worktrees by default`
- `tmp/archive/plan-audit-2026-09-23/08-worktree-isolation.md`

Warning: every file this item cites is gone (`crates/roko-cli/src/orchestrator/executor/config.rs`) — likely obsolete or moved.

How to verify: Read default_use_worktrees(); check roko.toml/executor config default.

Verified 2026-09-28: still true (orchestrator/executor/mod.rs:225); duplicate of gap-4ec59f.
