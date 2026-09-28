+++
id = "spec-f830c4"
kind = "spec"
title = "#404 — Batch Branch Integration"
status = "open"
triage = "verified"
severity = "p1"
goal = "features"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/404-batch-branch-integration.md##404 — Batch Branch Integration"
discovered_from = "audit:tmp/backlog/archive/404-batch-branch-integration.md##404 — Batch Branch Integration"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::CliCompletionDeliveryService", "crates/roko-cli/src/orchestrator/merge_queue.rs::MergeQueue", "crates/roko-cli/src/runner/merge.rs", "crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
links = { depends_on = [], blocks = [], related = ["gap-415c54"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'roko/batch/' crates/roko-cli/src"
+++
Roko's graph engine (`cmd_plan_run_engine`) executes plan tasks and runs per-task gate pipelines, but after all tasks pass it does nothing with the branches. The merge queue (`MergeQueue`), merge wrapper (`PlanMerger`), delivery state machine (`CliCompletionDeliveryService`), and git backends…

Imported without verification from:
- `tmp/backlog/archive/404-batch-branch-integration.md##404 — Batch Branch Integration`

How to verify: Check: At `roko plan run` start, a `roko/batch/{run-id}` branch is created (or reset to HEAD); When a plan's tasks complete and all gates pass, its branch is merged into the batch; The `PlanMerger` (with `MergeQueue` serialization, `git merge-tree`… [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: No `roko/batch/` branch is created anywhere. graph_execution/delivery.rs::CliCompletionDeliveryService and its MergeQueue-backed backend are constructed only in delivery.rs tests. On success an isolated attempt's worktree is released with WorkspaceReleasePolicy::Delete (documented as deleting checkout and branch) in graph_task_dispatch.rs, so worktree-mode results are never integrated.
