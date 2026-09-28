+++
id = "gap-df8c2a"
kind = "gap"
title = "TUI Queue Overview Modal (Milestone Roadmap on F2:Plans)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/216-tui-queue-overview-modal.md#216 — TUI Queue Overview Modal (Milestone Roadmap on F2:Plans)"
discovered_from = "audit:tmp/backlog/archive/216-tui-queue-overview-modal.md#216 — TUI Queue Overview Modal (Milestone Roadmap on F2:Plans)"
anchors = ["crates/roko-cli/src/tui/modals/queue_overview.rs", "crates/roko-cli/src/tui/modals/mod.rs", "crates/roko-cli/src/tui/input.rs", "queue.toml", "completed/total", ".roko/queue.toml", "plans/queue.toml", "crates/roko-cli/src/tui/state.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
nice-to-have visibility; operators managing 30+ plans cannot see milestone-level progress without reading raw TOML. When an operator runs a large batch of plans, there is no TUI surface that shows milestone-level progress. Mori's execution view had a queue overview modal that displayed all…

Imported without verification from:
- `tmp/backlog/archive/216-tui-queue-overview-modal.md#216 — TUI Queue Overview Modal (Milestone Roadmap on F2:Plans)`

Some cited files are gone: `.roko/queue.toml`, `completed/total`, `crates/roko-cli/src/tui/state.rs`, `plans/queue.toml`.

How to verify: Check: Pressing `q` on F2:Plans tab opens the queue overview modal; Modal displays all milestones from `queue.toml` with progress bars; Each milestone shows plan count and completion percentage [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): S | 7 |]
