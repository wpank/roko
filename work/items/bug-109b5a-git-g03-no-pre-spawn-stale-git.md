+++
id = "bug-109b5a"
kind = "bug"
title = "No pre-spawn stale .git/index.lock cleanup (with .git indirection) before agent dispatch"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-cli/worktree"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stale_locks", "crates/roko-cli/src/graph_execution/workspaces.rs:156"]
links = { depends_on = [], blocks = [], related = ["gap-7ed79a", "gap-4ec59f", "q-1faa0c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rn "clear_stale_locks" crates/roko-cli/src --include="*.rs" | grep -v "src/orchestrator/worktree/"'
+++
Post-crash stale index.lock files can hang agent spawns; fix added 5-min TTL stale lock removal in ensure_attempt_workdir() before turn_start. Register marks DONE (2026-09-04/05), but the runtime hook was placed in crates/roko-cli/src/runner/event_loop.rs (Runner-v2), which was retired/deleted 20...

Imported without verification from:
- `tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register`
- `tmp/archive/git-audit/impl-G03-index-lock-cleanup.md`

A source claims this was fixed; confirm against current code before closing.

Warning: every file this item cites is gone (`crates/roko-cli/src/runner/event_loop.rs`) — likely obsolete or moved.

How to verify: grep for index.lock cleanup reachable from the Graph engine's attempt workdir preparation before provider dispatch.

Verified 2026-09-28: still true - stale index.lock removal exists only as WorktreeManager::clear_stale_locks (orchestrator/worktree/cleanup.rs:261-271) with no production caller; the Graph path only reports WorktreeHealth::StaleLock as a reconcile conflict (graph_execution/workspaces.rs:156), with no pre-dispatch cleanup.
