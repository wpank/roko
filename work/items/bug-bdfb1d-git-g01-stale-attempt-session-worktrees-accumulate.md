+++
id = "bug-bdfb1d"
kind = "bug"
title = "Stale attempt/session worktrees accumulate beyond conductor limit (auto-prune at runner startup)"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/worktree"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/orchestrator/worktree/cleanup.rs::WorktreeManager::reclaim_idle", "crates/roko-cli/src/orchestrator/worktree/cleanup.rs::WorktreeManager::prune", "crates/roko-cli/src/graph_execution/plan_runner.rs:1060", "crates/roko-graph/src/workspace.rs::cleanup_orphans"]
links = { depends_on = [], blocks = [], related = ["gap-9d3f67"], supersedes = [], duplicate_of = "" }
+++
Attempt worktrees from crashed/completed runs persist; WorktreeCountWatcher (max 8) is advisory only. Fix added git worktree prune + orphan scan at runner startup and TTL pre-flight cleanup. Register marks DONE (2026-09-04/05), but the runtime hook was placed in crates/roko-cli/src/runner/event_l...

Imported without verification from:
- `tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register`
- `tmp/archive/git-audit/impl-G01-worktree-prune.md`

A source claims this was fixed; confirm against current code before closing.

Warning: every file this item cites is gone (`crates/roko-cli/src/orchestrator/worktree.rs`, `crates/roko-cli/src/runner/event_loop.rs`) — likely obsolete or moved.

How to verify: grep for 'worktree prune' / orphan scan invoked from Graph plan run startup (graph_execution/), not only from runner-v2 event_loop; check reclaim_idle().

Verified 2026-09-28: The G01 startup prune lived in the deleted runner/event_loop.rs. On the Graph path per-task worktrees are opt-in (--worktree-per-task, crates/roko-cli/src/graph_execution/plan_runner.rs:1060) and nothing outside orchestrator/worktree/tests.rs calls WorktreeManager::reclaim_idle or prune; roko-graph cleanup_orphans only releases in-memory Retained leases. Severity lowered p1 -> p2 because worktree isolation is no longer the default.
