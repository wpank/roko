+++
id = "gap-5a4ca8"
kind = "gap"
title = "TP2-37 #116: `plan run` does not consume .roko/queue.toml (milestones, maintenance batches, overrides)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/plan"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Roko parity assessment"
discovered_from = "audit:tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Roko parity assessment"
anchors = ["runner/queue_manifest.rs", "RunOverrides", "tui/modals/queue_overview.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Queue manifest parsing and plan queue show/validate/init exist, but plan run never selects the current milestone, runs maintenance batches or applies RunOverrides.model; the TUI queue modal derives milestones from execution waves, not queue.toml.

Imported without verification from:
- `tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Roko parity assessment`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`
- `tmp/workflow-audit/10-STATUS-UPDATE.md#09-MORI-GAPS.md — Current Status`

How to verify: grep queue_manifest consumers under commands/plan.rs and graph_execution/.
