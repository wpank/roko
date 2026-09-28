+++
id = "bug-53475e"
kind = "bug"
title = "[git G07] Failed worktree cleanup retains kernel mutation lock requiring operator intervention"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/worktree"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stuck_mutation_lock", "crates/roko-cli/src/orchestrator/worktree/mod.rs::REPOSITORY_MUTATION_LOCK"]
links = { depends_on = [], blocks = [], related = ["gap-7ed79a", "gap-4ec59f", "q-1faa0c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rn "clear_stuck_mutation_lock" crates/roko-cli/src --include="*.rs" | grep -v "src/orchestrator/worktree/"'
+++
Fix added non-blocking flock probe + 5-min age gate on .roko/roko-worktree-mutation.lock at runner startup. Register marks DONE (2026-09-04/05), but the runtime hook was placed in crates/roko-cli/src/runner/event_loop.rs (Runner-v2), which was retired/deleted 2026-09-06; the Graph plan-execution...

Imported without verification from:
- `tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register`
- `tmp/archive/git-audit/impl-G07-stuck-lock-recovery.md`

A source claims this was fixed; confirm against current code before closing.

Warning: every file this item cites is gone (`.roko/roko-worktree-mutation.lock`, `crates/roko-cli/src/orchestrator/worktree.rs`, `crates/roko-cli/src/runner/event_loop.rs`) — likely obsolete or moved.

How to verify: Check stuck-lock recovery for roko-worktree-mutation.lock is invoked at Graph plan-run startup.

Verified 2026-09-28: still true - WorktreeManager::clear_stuck_mutation_lock (orchestrator/worktree/cleanup.rs:181) has no production caller, so nothing on Graph plan-run startup recovers a stuck roko-worktree-mutation.lock (orchestrator/worktree/mod.rs:81). The worktree code moved from orchestrator/worktree.rs to the orchestrator/worktree/ module.
