+++
id = "gap-0001a1"
kind = "gap"
title = "Parallel Plan Queue Execution"
status = "open"
triage = "verified"
severity = "p1"
goal = "features"
subsystem = ["roko-cli/runner"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/272-parallel-plan-queues.md#272 — Parallel Plan Queue Execution"
discovered_from = "audit:tmp/backlog/archive/272-parallel-plan-queues.md#272 — Parallel Plan Queue Execution"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1197", "crates/roko-cli/src/graph_checkpoint.rs:884", "crates/roko-cli/src/graph_execution/plan_runner.rs:1210", "crates/roko-cli/src/graph_execution/plan_set.rs::PlanSetScheduler", "crates/roko-cli/src/graph_checkpoint.rs::resolve_checkpoint_paths"]
links = { depends_on = [], blocks = [], related = ["gap-a80e05", "gap-7c9e48", "gap-be8416", "gap-9084e7"], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked —

Imported without verification from:
- `tmp/backlog/archive/272-parallel-plan-queues.md#272 — Parallel Plan Queue Execution`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.5 Proof Case 5: Concurrent plan runs`

How to verify: Check: Implement the exact files, types, storage paths, and lock scopes above.; Give every run its own checkpoint, Activity ledger, worktree namespace, budget, control channel, and status identity.; Add a bounded queue/supervisor with configurable… [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked]

Verified 2026-09-28: still open - the Graph runner executes plans one at a time in dependency order (graph_execution/plan_runner.rs:1197) and checkpoints live per plan id under .roko/state/graph/<plan>/ (graph_checkpoint.rs:884); there is no run-scoped namespace, bounded queue or supervisor.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Added in 725f21e05: the Graph runner no longer has to run plans strictly one at a time. PlanSetScheduler (crates/roko-cli/src/graph_execution/plan_set.rs:1-8; used at graph_execution/plan_runner.rs:1210-1308) starts plans concurrently, bounded by max_parallel_plans (plan_runner.rs:626-630; [conductor] default 1; flag plumbed through commands/plan.rs:486,550,606) and blocked by depends_on_plan and footprint overlap. What remains of #272: all plans still share the operator's working tree (plan_set.rs:4), checkpoints and activities are still keyed per plan id under .roko/state/graph/<plan>/ (resolve_checkpoint_paths, graph_checkpoint.rs:1059-1112, working tree with large uncommitted edits), and no run lock was found. There is also no run-scoped namespace (checkpoint, activity ledger, worktree namespace, budget, control channel, status identity) and no queue or supervisor for independent concurrent runs.
