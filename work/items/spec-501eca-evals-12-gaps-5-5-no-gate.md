+++
id = "spec-501eca"
kind = "spec"
title = "[evals 12-gaps 5.5] No gate-gaming / eval-regression TUI notifications or PostFX triggers"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#5.5 No Gaming Alert Notification"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#5.5 No Gaming Alert Notification"
anchors = ["GateGamingDetector JSONL alerts", "TUI notification history", "PostFX"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Fire a TUI notification when gate gaming is detected or a rung's EMA drops >10 points; trigger the PostFX pipeline for eval-failure feedback.

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#5.5 No Gaming Alert Notification`
- `tmp/archive/evals-audit/15-tui-widget-catalog.md`
- `tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order`

How to verify: Check whether gaming alerts reach the TUI notification queue.
