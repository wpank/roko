+++
id = "gap-732989"
kind = "gap"
title = "TUI Visual Density Improvements"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/241-tui-visual-density-improvements.md#241 — TUI Visual Density Improvements"
discovered_from = "audit:tmp/backlog/archive/241-tui-visual-density-improvements.md#241 — TUI Visual Density Improvements"
anchors = ["crates/roko-cli/src/tui/views/dashboard_view.rs", "dashboard_view.rs:79", "dashboard_view.rs:1919-1940", "dashboard_view.rs", "widgets/wave_progress.rs", "widgets/token_sparkline.rs", "widgets/sys_metrics.rs", "crates/roko-cli/src/tui/widgets/wave_progress.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The bottom ribbon consumes 6 rows (vs mori's 4), the phase indicator wastes space when idle, and triple borders in the left panel reduce usable content area.. On a standard 40-row terminal, every wasted row reduces the plan tree or agent output by 2.5%. Three specific density issues compound:

Imported without verification from:
- `tmp/backlog/archive/241-tui-visual-density-improvements.md#241 — TUI Visual Density Improvements`

How to verify: Check: The bottom ribbon occupies 4 rows instead of 6.; All three sub-widgets (wave, token, sys) remain readable at 4 rows.; The phase indicator row is absent when no phase is active. [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 4 |]
