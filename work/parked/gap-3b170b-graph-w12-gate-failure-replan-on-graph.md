+++
id = "gap-3b170b"
kind = "gap"
title = "[graph W12] Gate-failure replan on Graph engine only records a replan signal"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
discovered_from = "audit:tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
anchors = ["build_gate_failure_plan_revision", "crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/runner/gate_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Marked Done as 'replan signal recorded on gate failures'; the work item required build_gate_failure_plan_revision to trigger an actual plan revision on gate failure.

Imported without verification from:
- `tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done`
- `tmp/archive/graph-audit/07-work-items.md#w12`

How to verify: Check that a Graph gate failure with replan_on_gate_failure=true produces and applies a plan revision (ReplanController), not just a signal.
