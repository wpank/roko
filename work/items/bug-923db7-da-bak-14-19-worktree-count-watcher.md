+++
id = "bug-923db7"
kind = "bug"
title = "DA-bak-14/19: worktree_count watcher Restart kills all agents; stale ring metrics; limit not tied to concurrency"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-conductor"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/archive/dev-audit-2026-09-21/dev-audit-backup/14-conductor-restart-loop.md#Solutions"
discovered_from = "audit:tmp/archive/dev-audit-2026-09-21/dev-audit-backup/14-conductor-restart-loop.md#Solutions"
anchors = ["crates/roko-conductor/src/watchers/worktree_count.rs", "DEFAULT_MAX_LIVE", "WorktreeConfig.max_live"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Conductor worktree_count watcher (DEFAULT_MAX_LIVE=8) maps warnings to a Restart that kills every agent; stale >8 metrics stay in the 512-entry ring and re-trigger; WorktreeConfig.max_live is None with no worktree permit in admission. Cooldown/attempt fixes were Runner-v2 only.

Imported without verification from:
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/14-conductor-restart-loop.md#Solutions`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/19-concurrency-control.md#Solutions`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/22-fix-runbook.md#P0-4: Raise worktree watcher limit`

How to verify: Check watcher severity->action mapping, ring reset after Restart, and how the Graph engine reacts to Restart.
