+++
id = "spec-6942f7"
kind = "spec"
title = "[evals 12-gaps 5.3/5.4/5.6/5.7] Missing eval TUI visualizations (experiments CIs, SPC alerts, arena leaderboard, benchmarks)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#5.3 No Experiment Visualization with Wilson CIs"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#5.3 No Experiment Visualization with Wilson CIs"
anchors = ["tui/views/", "arena leaderboard", "roko bench results"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
No experiment visualization with Wilson CIs, no SPC alert visualization, no arena leaderboard in TUI, no benchmark run results in TUI.

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#5.3 No Experiment Visualization with Wilson CIs`
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#5.6 No Arena Leaderboard in TUI`
- `tmp/archive/evals-audit/15-tui-widget-catalog.md`

Warning: every file this item cites is gone (`tui/views/`) — likely obsolete or moved.

How to verify: grep TUI views for experiment/SPC/arena/benchmark panels.
