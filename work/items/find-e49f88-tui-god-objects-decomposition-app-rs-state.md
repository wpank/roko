+++
id = "find-e49f88"
kind = "finding"
title = "TUI God Objects Decomposition (`app.rs` / `state.rs` / `dashboard.rs`)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/148-tui-god-objects-decomposition.md#148 — TUI God Objects Decomposition (`app.rs` / `state.rs` / `dashboard.rs`)"
discovered_from = "audit:tmp/backlog/archive/148-tui-god-objects-decomposition.md#148 — TUI God Objects Decomposition (`app.rs` / `state.rs` / `dashboard.rs`)"
anchors = ["crates/roko-cli/src/tui/", "crates/roko-cli/src/tui/app.rs", "crates/roko-cli/src/tui/tabs.rs", "dashboard.rs", "app.rs", "tui/key_handler.rs", "tui/action_dispatcher.rs", "tui/io_coordinator.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Three TUI files (app.rs at 4,576 LOC, state.rs at 5,290 LOC, dashboard.rs at 7,445 LOC) are god objects that make the TUI impossible to modify safely without reading the entire file; decomposition is a prerequisite for sustainable TUI evolution.. The TUI module has three god-object files that…

Imported without verification from:
- `tmp/backlog/archive/148-tui-god-objects-decomposition.md#148 — TUI God Objects Decomposition (`app.rs` / `state.rs` / `dashboard.rs`)`
- `tmp/backlog/_archive/_checklist-gaps.md#§5.1`

Some cited files are gone: `crates/roko-cli/src/tui/app.rs`, `tui/action_dispatcher.rs`, `tui/io_coordinator.rs`, `tui/key_handler.rs`.

How to verify: Check: After Sub-item A: `app.rs` is ≤ 500 LOC. Key handling, action dispatch, and I/O coordination are in separate files.; After Sub-item B: No single state file exceeds 1,500 LOC. Plan, agent, learning, and system state are in separate files… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): L | 6 |]
