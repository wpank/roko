+++
id = "gap-b6e852"
kind = "gap"
title = "Header Bar Enrichment"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/239-header-bar-enrichment.md#239 — Header Bar Enrichment"
discovered_from = "audit:tmp/backlog/archive/239-header-bar-enrichment.md#239 — Header Bar Enrichment"
anchors = ["crates/roko-cli/src/tui/widgets/header_bar.rs", "crates/roko-cli/src/tui/app.rs", "crates/roko-cli/src/tui/state.rs", "widgets/header_bar.rs", "state.rs:1134", "state.rs", "app.rs", "header_bar.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The header bar has space for additional metrics (MCP tool count, network I/O, disk I/O, FPS) that provide operational context, plus a persistent warning bar below it for non-dismissable alerts.. The header bar at `widgets/header_bar.rs` currently has 9 sections: health dot, queue/plan name, wave…

Imported without verification from:
- `tmp/backlog/archive/239-header-bar-enrichment.md#239 — Header Bar Enrichment`

Some cited files are gone: `crates/roko-cli/src/tui/app.rs`, `crates/roko-cli/src/tui/state.rs`.

How to verify: Check: The header bar shows MCP tool count, NET I/O, DSK I/O, and FPS metrics.; When NET/DSK data is unavailable, dashes or "n/a" are shown (no panic).; A yellow/red warning bar appears below the header when workspace warnings exist. [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 4 |]
