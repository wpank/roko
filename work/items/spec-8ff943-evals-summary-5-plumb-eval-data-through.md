+++
id = "spec-8ff943"
kind = "spec"
title = "[evals SUMMARY #5] Plumb eval data through DashboardSnapshot -> TuiState -> widgets"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order"
discovered_from = "audit:tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order"
anchors = ["DashboardSnapshot", "TuiState", "canonical snapshot loader"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Gate verdicts, c-factor scores and eval records must flow through the canonical snapshot loader into TUI state before any eval view can render. Spec in 16-endpoint-data-flow; not in the implementation checklist.

Imported without verification from:
- `tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order`
- `tmp/archive/evals-audit/16-endpoint-data-flow.md`

How to verify: Check DashboardSnapshot/TuiState for eval/verdict/QualityScore fields fed from gate records.
