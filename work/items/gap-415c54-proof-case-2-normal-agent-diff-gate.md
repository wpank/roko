+++
id = "gap-415c54"
kind = "gap"
title = "Proof Case 2: Normal agent diff + gate + merge"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.2 Proof Case 2: Normal agent diff + gate + merge"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.2 Proof Case 2: Normal agent diff + gate + merge"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_execution/delivery.rs::CliCompletionDeliveryService", "crates/roko-graph/src/workspace.rs::WorkspaceReleasePolicy"]
links = { depends_on = [], blocks = [], related = ["spec-f830c4"], supersedes = [], duplicate_of = "" }
+++
Agent makes a real worktree diff and exits normally; gate, commit/merge, summary, and cleanup all terminate without manual intervention.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.2 Proof Case 2: Normal agent diff + gate + merge`

How to verify: Source: Dogfood audit, proof case 2. Check the described code path for: Agent makes a real worktree diff and exits normally; gate, commit/merge, summary, and cleanup all terminate without manual intervention.

Verified 2026-09-28: The diff + gate + summary half ran live in the 2026-09-25 portal run (in place, no worktree). Commit/merge is not wired on the Graph path: with --worktree-per-task a successful attempt's lease is released with WorkspaceReleasePolicy::Delete (graph_task_dispatch.rs 'Worktree isolation: release on success': 'can be merged separately via the delivery pipeline. For now the worktree is cleaned up'), and graph_execution/delivery.rs::CliCompletionDeliveryService is constructed only in its own tests.
