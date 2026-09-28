+++
id = "gap-36f3fb"
kind = "gap"
title = "Interrupted in-flight task side effects are unrecorded"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
subsystem = ["roko-graph/checkpoint"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned"
anchors = ["activities.jsonl", "crates/roko-cli/src/graph_checkpoint.rs::WORKSPACE_ATTEMPT_EXTENSION", "crates/roko-cli/src/graph_execution/plan_runner.rs:259"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
A task that wrote files but had not reached the Ok arm leaves on-disk changes with no activity record, so workspace and checkpoint diverge on interrupt.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned`

How to verify: Interrupt mid-task and compare git diff with checkpoint.

Check on 2026-09-28 was inconclusive: No per-task 'attempt started' activity record was found in graph_execution or graph_checkpoint.rs. The checkpoint's Running status is plan-level (graph_checkpoint.rs:72, :981), and the interrupt path only cancels or kills in-flight agents (graph_execution/plan_runner.rs:259, :354-366) without recording their workspace changes. However, the checkpoint has a workspace/attempt extension (WORKSPACE_ATTEMPT_EXTENSION roko.workspace.attempt@1, graph_checkpoint.rs:59, lease_id test :2218), and graph_checkpoint.rs has 830 uncommitted added lines. To decide: check whether that extension is written when an attempt starts (before the Ok arm) and whether resume reconciles the retained worktree or in-place diff, or run the item's interrupt-and-compare check.
