+++
id = "gap-a85a1f"
kind = "gap"
title = "[plan-audit T2-01] Block git checkout/switch/branch -m/push inside plan worktrees"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-01: Add git hook blocks for plan worktrees"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-01: Add git hook blocks for plan worktrees"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "backlog #400"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Inject PreToolUse hooks or tool deny rules so agents cannot escape their isolated worktree via git checkout/switch/rename/push. Backlog #400.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-01: Add git hook blocks for plan worktrees`
- `tmp/archive/plan-audit-2026-09-23/08-worktree-isolation.md`

How to verify: Check worktree dispatch config for git deny rules.
