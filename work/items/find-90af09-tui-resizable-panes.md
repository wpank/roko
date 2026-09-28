+++
id = "find-90af09"
kind = "finding"
title = "TUI Resizable Panes"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/199-tui-resizable-panes.md#199 — TUI Resizable Panes"
discovered_from = "audit:tmp/backlog/archive/199-tui-resizable-panes.md#199 — TUI Resizable Panes"
anchors = ["crates/roko-cli/src/tui/views/dashboard_view.rs:64", "crates/roko-cli/src/tui/views/plans_view.rs:46", "crates/roko-cli/src/tui/views/agents_view.rs:51", "crates/roko-cli/src/tui/views/git_view.rs:105", "crates/roko-cli/src/tui/views/marketplace_view.rs:74", "crates/roko-cli/src/tui/views/context_view.rs:150", "crates/roko-cli/src/tui/app.rs:2273", "crates/roko-cli/src/tui/app.rs:2428", "tui/layout.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
UX polish; all split ratios are hardcoded, preventing operators from adjusting views to their terminal size. All horizontal split ratios in Roko's TUI are fixed percentages. Operators with wide terminals waste space, and operators with narrow terminals have cramped left panels. There is no…

Imported without verification from:
- `tmp/backlog/archive/199-tui-resizable-panes.md#199 — TUI Resizable Panes`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-32`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Missing`
- `tmp/tui-parity2/00-INDEX.md`

Some cited files are gone: `crates/roko-cli/src/tui/app.rs`.

How to verify: Check: `PaneConfig` struct exists in `TuiState` with per-tab left-pane percentages; `<` / `>` keys adjust the active tab's pane ratio by 5% increments; Pane ratios are clamped to 20%–80% range [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): S | 7 |] / grep PaneConfig.

Merged 2 mined candidates: m1-052, m4-167.
