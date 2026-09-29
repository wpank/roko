+++
id = "gap-36f3fb"
kind = "gap"
title = "Interrupted in-flight task side effects are unrecorded"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-graph/checkpoint"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned"
anchors = ["crates/roko-graph/src/replay.rs::ActivityRecorder::record", "crates/roko-graph/src/engine.rs::reconcile_running_status", "crates/roko-cli/src/graph_checkpoint.rs::WORKSPACE_ATTEMPT_EXTENSION", "crates/roko-cli/src/graph_execution/workspaces.rs::reconcile", "crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'WORKSPACE_ATTEMPT_EXTENSION' crates/roko-cli/src/graph_execution/ && cargo test -p roko-cli --lib interrupted_attempt_is_recorded_and_reconciled_on_resume"
+++
A task that wrote files but had not reached the Ok arm leaves on-disk changes with no activity record, so workspace and checkpoint diverge on interrupt.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned`

How to verify: Interrupt mid-task and compare git diff with checkpoint.

Check on 2026-09-28 was inconclusive: No per-task 'attempt started' activity record was found in graph_execution or graph_checkpoint.rs. The checkpoint's Running status is plan-level (graph_checkpoint.rs:72, :981), and the interrupt path only cancels or kills in-flight agents (graph_execution/plan_runner.rs:259, :354-366) without recording their workspace changes. However, the checkpoint has a workspace/attempt extension (WORKSPACE_ATTEMPT_EXTENSION roko.workspace.attempt@1, graph_checkpoint.rs:59, lease_id test :2218), and graph_checkpoint.rs has 830 uncommitted added lines. To decide: check whether that extension is written when an attempt starts (before the Ok arm) and whether resume reconciles the retained worktree or in-place diff, or run the item's interrupt-and-compare check.

Checked statically on 2026-09-29 at d9e79e9d8: still true. Activity records are written only when a node completes (roko-graph replay.rs ActivityRecorder::record, called from engine.rs:764, :1216, :1977, :2453). WORKSPACE_ATTEMPT_EXTENSION (graph_checkpoint.rs:59) is used only in tests, so no attempt or lease record is written when an attempt starts. The workspace lease reconcile (graph_execution/workspaces.rs:113) is never called, and on restore engine.rs:1479 resets a Running node to Pending through reconcile_running_status without inspecting the retained worktree or the in-place diff. The interrupt path (plan_runner.rs:355-367, :2100-2111) only signals or kills agent processes. A fix should write the attempt extension (lease path, attempt id, base commit) when an attempt starts and reconcile it on resume.
