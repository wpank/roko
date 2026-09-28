+++
id = "gap-0fbf8a"
kind = "gap"
title = "[plan-audit T1-12] Delete auto-injected CLAUDE.md/AGENTS.md from worktrees before spawn"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-12: Delete auto-injected CLAUDE.md/AGENTS.md from worktrees"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-12: Delete auto-injected CLAUDE.md/AGENTS.md from worktrees"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Claude CLI auto-discovers per-crate CLAUDE.md and root AGENTS.md (~37K tokens/request) duplicating system prompt content; strip them in worktrees pre-spawn.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-12: Delete auto-injected CLAUDE.md/AGENTS.md from worktrees`
- `tmp/archive/plan-audit-2026-09-23/10-token-optimization.md`

How to verify: Check pre-spawn worktree preparation for CLAUDE.md removal.
