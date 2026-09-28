+++
id = "gap-667a3c"
kind = "gap"
title = "TUX 01-08: Per-report TUI checklist residuals not carried into the implementation checklist"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/tui-ux-audit/03-agent-output-streaming.md"
discovered_from = "audit:tmp/archive/tui-ux-audit/03-agent-output-streaming.md"
anchors = ["tui/widgets/stream_output.rs", "views/context_view.rs", "views/logs_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Unmapped items: live/disconnected stream indicator, buffer-rollover truncation marker, per-agent token/cost output footer, task-detail dependency graph and per-task gate results, per-model performance breakdown and knowledge-store stats in Inspect, log quick-filters, human-readable conductor events.

Imported without verification from:
- `tmp/archive/tui-ux-audit/03-agent-output-streaming.md`
- `tmp/archive/tui-ux-audit/05-plan-task-details.md`
- `tmp/archive/tui-ux-audit/07-inspect-tab.md`
- `tmp/archive/tui-ux-audit/08-logs-tab.md`

How to verify: Check each listed element in a live capture.
