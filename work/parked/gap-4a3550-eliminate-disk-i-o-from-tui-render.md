+++
id = "gap-4a3550"
kind = "gap"
title = "Eliminate Disk I/O from TUI Render Path"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/387-tui-render-path-disk-io-elimination.md#387 — Eliminate Disk I/O from TUI Render Path"
discovered_from = "audit:tmp/backlog/archive/387-tui-render-path-disk-io-elimination.md#387 — Eliminate Disk I/O from TUI Render Path"
anchors = ["crates/roko-cli/src/tui/views/", "crates/roko-cli/src/tui/views/dashboard_view.rs", "crates/roko-cli/src/tui/views/config_view.rs", "crates/roko-cli/src/tui/views/context_view.rs", "crates/roko-cli/src/tui/state.rs", "dashboard_view.rs:737", "roko.toml", "config_view.rs:65", "context_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
up to 120 file reads per second during rendering. TUI parity audit identified three render functions reading files on every frame (20-60 fps): MCP sub-tab reads roko.toml + MCP config, F7 Inspect reads mcp-stats.json and playbook.json, F6 Config parses TOML on every frame. Some caching was added…

Imported without verification from:
- `tmp/backlog/archive/387-tui-render-path-disk-io-elimination.md#387 — Eliminate Disk I/O from TUI Render Path`
- `tmp/backlog/archive/235-tui-render-path-disk-io-elimination.md#235 — TUI Render-Path Disk I/O Elimination`

Some cited files are gone: `crates/roko-cli/src/tui/state.rs`.

How to verify: Check whether the gap described in tmp/backlog/archive/387-tui-render-path-disk-io-elimination.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)] / Check: No synchronous file I/O occurs during any render frame.; Config changes (via `roko config set` or manual edit) appear in the TUI within 5 seconds.; MCP config changes appear within 5 seconds. [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 4 |]

Merged 2 mined candidates: m1-110, m1-067.
