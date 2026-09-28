+++
id = "bug-3099fe"
kind = "bug"
title = "DOCS-06 B1 / TD-07: TUI reads files on every frame (MCP sub-tab, F7 Inspect, F6 Config)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B1. Disk I/O in TUI render path"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B1. Disk I/O in TUI render path"
anchors = ["crates/roko-cli/src/tui/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Three render-path functions do disk I/O each frame, degrading TUI performance on large workspaces (decision: cache and refresh on change).

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B1. Disk I/O in TUI render path`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-07: TUI Disk I/O in Render Path`
- `docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items`

How to verify: grep render functions in tui views for std::fs/read_to_string. / grep tui render paths for std::fs::read* calls.

Merged 2 mined candidates: m4-037, m5-096.
