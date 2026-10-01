+++
id = "bug-bdfb1d"
kind = "bug"
title = "Stale attempt/session worktrees accumulate beyond conductor limit (auto-prune at runner startup)"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/worktree"]
created = 2026-09-05
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/orchestrator/worktree/cleanup.rs::WorktreeManager::reclaim_idle", "crates/roko-cli/src/orchestrator/worktree/cleanup.rs::WorktreeManager::prune", "crates/roko-cli/src/graph_execution/plan_runner.rs:1155", "crates/roko-graph/src/workspace.rs::cleanup_orphans"]
links = { depends_on = [], blocks = [], related = ["gap-9d3f67"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -nE '\\.(reclaim_idle|prune)\\(' crates/roko-cli/src/graph_execution/plan_runner.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:59Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:57Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Already fixed at BASE ebdc0f5d5; the item's 2026-10-01 note gives the evidence."
+++
Attempt worktrees from crashed/completed runs persist; WorktreeCountWatcher (max 8) is advisory only. Fix added git worktree prune + orphan scan at runner startup and TTL pre-flight cleanup. Register marks DONE (2026-09-04/05), but the runtime hook was placed in crates/roko-cli/src/runner/event_l...

Imported without verification from:
- `tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register`
- `tmp/archive/git-audit/impl-G01-worktree-prune.md`

A source claims this was fixed; confirm against current code before closing.

Warning: every file this item cites is gone (`crates/roko-cli/src/orchestrator/worktree.rs`, `crates/roko-cli/src/runner/event_loop.rs`) — likely obsolete or moved.

How to verify: grep for 'worktree prune' / orphan scan invoked from Graph plan run startup (graph_execution/), not only from runner-v2 event_loop; check reclaim_idle().

Verified 2026-09-28: The G01 startup prune lived in the deleted runner/event_loop.rs. On the Graph path per-task worktrees are opt-in (--worktree-per-task, crates/roko-cli/src/graph_execution/plan_runner.rs:1060) and nothing outside orchestrator/worktree/tests.rs calls WorktreeManager::reclaim_idle or prune; roko-graph cleanup_orphans only releases in-memory Retained leases. Severity lowered p1 -> p2 because worktree isolation is no longer the default.

Rechecked 2026-09-29 at d9e79e9d8: still open. The --worktree-per-task block has moved from plan_runner.rs:1060 to plan_runner.rs:1154-1171. It still calls neither reclaim_idle nor prune. roko-graph cleanup_orphans is called only from tests.

## Notes

2026-10-01 (wk-planrun): already fixed at BASE: the startup prune. A `--worktree-per-task` run calls `repair_worktree_state` before its first dispatch (`crates/roko-cli/src/graph_execution/plan_runner.rs:1186` at `ebdc0f5d5`), which runs `worktrees.prune()`, i.e. `git worktree prune` (`plan_runner.rs:2428`), after clearing stale `index.lock` files and a stuck mutation lock. Added by `8e23f0a79` (gap-4ec59f step 1, merged in `8ae38c669`). The `[[verify]]` grep passes at BASE. Per-task worktrees are the only Graph mode that makes attempt checkouts, so the repair runs exactly where they exist.
Not ported, and not proposed for this item: the old G01 orphan-checkout scan and the TTL eviction. `WorktreeManager::reclaim_idle` still has no production caller, and it only sees this process's worktrees. `roko doctor disk` reports orphaned worktree directories but nothing removes them. Removing checkouts at startup needs a retention decision, because a resumed run re-attaches the checkouts it kept (bug-056b40) and failed attempts keep theirs for inspection. That belongs in a new item if it is wanted.
