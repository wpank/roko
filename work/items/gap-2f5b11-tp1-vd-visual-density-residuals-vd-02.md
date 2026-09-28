+++
id = "gap-2f5b11"
kind = "gap"
title = "TP1-VD: Visual density residuals (VD-02/03/07/08/09/10/11/13/16)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/tui-parity/audits/visual-density-effects.md#Live visual re-audit and implementation status (17:37 run)"
discovered_from = "audit:tmp/tui-parity/audits/visual-density-effects.md#Live visual re-audit and implementation status (17:37 run)"
anchors = ["views/dashboard_view.rs", "widgets/header_bar.rs", "widgets/status_bar.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Unresolved density items: idle phase block still reserved, triple-bordered left panel, duplicated header/status values, muted monochrome hues, HALTED text badge, single-color header gradient, four identical waiting panels, 2-state sub-tab colors, static borders.

Imported without verification from:
- `tmp/tui-parity/audits/visual-density-effects.md#Live visual re-audit and implementation status (17:37 run)`
- `tmp/tui-parity/audits/visual-density-effects.md#Important Issues`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`

How to verify: Render 80x24 captures and compare against each VD item.
