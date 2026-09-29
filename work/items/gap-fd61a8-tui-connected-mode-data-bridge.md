+++
id = "gap-fd61a8"
kind = "gap"
title = "TUI Connected-Mode Data Bridge"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/232-tui-connected-mode-data-bridge.md#232 — TUI Connected-Mode Data Bridge"
discovered_from = "audit:tmp/backlog/archive/232-tui-connected-mode-data-bridge.md#232 — TUI Connected-Mode Data Bridge"
anchors = ["crates/roko-cli/src/tui/widgets/token_sparkline.rs::render_token_sparkline", "crates/roko-cli/src/tui/views/dashboard_view.rs:1297"]
links = { depends_on = [], blocks = [], related = ["gap-038eaa"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Widget half fixed at HEAD: tui/widgets/token_sparkline.rs:100-102 falls back to TuiState cumulative token/cost fields in connected mode and the dashboard efficiency line reads tui_state.efficiency_summary (tui/views/dashboard_view.rs:1297). Live values still read 0 because the Graph path never publishes token/cost events; that remainder is gap-038eaa."
+++
During `plan run`, the TUI's efficiency and token widgets read from the file-backed `DashboardData` which is initialized empty and never refreshed in connected mode; the event-driven `TuiState` path has the data but the widgets ignore it.. The TUI has two parallel data paths:

Imported without verification from:
- `tmp/backlog/archive/232-tui-connected-mode-data-bridge.md#232 — TUI Connected-Mode Data Bridge`

Some cited files are gone: `crates/roko-cli/src/tui/state.rs`.

How to verify: Check: During a live `plan run` in connected mode, the token sparkline shows a real-time braille graph with non-zero values.; Token rate displays a non-zero value (e.g. "1.2k/min") in the sparkline widget.; The efficiency summary line shows real… [evidence: own status: Verified (2026-09-03) — sparkline fallback, EMA rates, learning bridge; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | 3 |; (newer evidence overrides…]

Verified 2026-09-28: closed as superseded; see [closed].evidence.
