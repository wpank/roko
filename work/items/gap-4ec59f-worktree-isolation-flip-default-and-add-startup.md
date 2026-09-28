+++
id = "gap-4ec59f"
kind = "gap"
title = "Worktree Isolation: Flip Default and Add Startup Repair"
status = "open"
triage = "verified"
severity = "p0"
goal = "core"
subsystem = ["roko-cli/orchestrator"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/400-worktree-isolation-defaults.md#400 — Worktree Isolation: Flip Default and Add Startup Repair"
discovered_from = "audit:tmp/backlog/archive/400-worktree-isolation-defaults.md#400 — Worktree Isolation: Flip Default and Add Startup Repair"
anchors = ["crates/roko-cli/src/orchestrator/executor/mod.rs::ExecutorConfig::default_use_worktrees", "crates/roko-cli/src/graph_execution/plan_runner.rs:1059", "crates/roko-cli/src/graph_task_dispatch.rs:926", "crates/roko-cli/src/serve_runtime.rs:577", "crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stale_locks"]
links = { depends_on = [], blocks = [], related = ["bug-109b5a", "bug-53475e", "gap-d58ae8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -A1 "const fn default_use_worktrees" crates/roko-cli/src/orchestrator/executor/mod.rs | grep -q true && grep -rqn "use_worktrees" crates/roko-cli/src/graph_execution'
+++
Roko's `WorktreeManager` and `WorktreeExecutionWorkspaceProvider` are production-quality infrastructure. The `PreToolUse` git hook blocks in `build_settings_json()` are already implemented and wired for ClaudeCliAgent. None of this matters because `default_use_worktrees()` returns `false`.

Imported without verification from:
- `tmp/backlog/archive/400-worktree-isolation-defaults.md#400 — Worktree Isolation: Flip Default and Add Startup Repair`

How to verify: Check: `default_use_worktrees()` returns `true`.; The test `executor_config_disables_worktrees_by_default` is renamed and; `config.executor.use_worktrees` is read at the graph-engine startup path and [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: still true - orchestrator/executor/mod.rs:225-227 default_use_worktrees() returns false and no Graph-path code reads executor.use_worktrees; Graph per-task worktrees are opt-in only (graph_execution/plan_runner.rs:1059-1073, graph_task_dispatch.rs:926; serve_runtime.rs:577 hard-codes worktree_per_task: false); startup repair helpers clear_stale_locks / clear_stuck_mutation_lock (orchestrator/worktree/cleanup.rs:181/269) have no production caller. Duplicates folded in: gap-1673bb, gap-c8d637, gap-764230.
