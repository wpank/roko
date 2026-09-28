+++
id = "spec-8f5e1f"
kind = "spec"
title = "[evals 12-gaps 5.1] No eval-specific TUI view (F7 Inspect Evals sub-view)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#5.1 No Eval-Specific TUI Views"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#5.1 No Eval-Specific TUI Views"
anchors = ["F7 Inspect tab", "QualityScore (roko-gate/src/scoring.rs)", "verdict_scorer.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Implement an Evals sub-view under F7 Inspect: per-rung verdict table, QualityScore 7-axis chart, VerdictScorer output.

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#5.1 No Eval-Specific TUI Views`
- `tmp/archive/evals-audit/14-tui-eval-dashboard.md`
- `tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order`

How to verify: Check F7 inspect view for an evals sub-view.
