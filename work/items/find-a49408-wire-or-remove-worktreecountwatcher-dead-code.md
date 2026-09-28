+++
id = "find-a49408"
kind = "finding"
title = "Wire or Remove WorktreeCountWatcher Dead Code"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-conductor/watchers"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/395-worktree-count-watcher-dead-code.md#395 — Wire or Remove WorktreeCountWatcher Dead Code"
discovered_from = "audit:tmp/backlog/archive/395-worktree-count-watcher-dead-code.md#395 — Wire or Remove WorktreeCountWatcher Dead Code"
anchors = ["crates/roko-conductor/src/watchers/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
watcher exists but is dead code. Dev-audit found WorktreeCountWatcher exists as dead code. The conductor has 12 watchers registered but WorktreeCountWatcher is not among them. The worktree watcher limit is hardcoded at 8 rather than reading from config.

Imported without verification from:
- `tmp/backlog/archive/395-worktree-count-watcher-dead-code.md#395 — Wire or Remove WorktreeCountWatcher Dead Code`

How to verify: Check whether the gap described in tmp/backlog/archive/395-worktree-count-watcher-dead-code.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
