+++
id = "spec-7d9397"
kind = "spec"
title = "[evals 12-gaps 5.2] No eval health ribbon in F1 Dashboard"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#5.2 No Eval Health Ribbon in Dashboard"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#5.2 No Eval Health Ribbon in Dashboard"
anchors = ["F1 Dashboard header", "tui/widgets/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Add a compact gate-verdict sparkline + overall eval health indicator to the F1 dashboard header (spec in 14-tui-eval-dashboard / 15-tui-widget-catalog).

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#5.2 No Eval Health Ribbon in Dashboard`
- `tmp/archive/evals-audit/14-tui-eval-dashboard.md`
- `tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order`

Warning: every file this item cites is gone (`tui/widgets/`) — likely obsolete or moved.

How to verify: Look for an eval health/verdict sparkline widget in the F1 dashboard view.
