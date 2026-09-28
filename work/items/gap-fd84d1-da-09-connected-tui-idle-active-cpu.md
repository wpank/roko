+++
id = "gap-fd84d1"
kind = "gap"
title = "DA-09: Connected TUI idle/active CPU and input latency never measured after redraw fixes"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/dev-audit/09-additional-live-run-findings.md#The connected TUI does unnecessary idle work"
discovered_from = "audit:tmp/dev-audit/09-additional-live-run-findings.md#The connected TUI does unnecessary idle work"
anchors = ["crates/roko-cli/src/tui/app.rs", "crates/roko-cli/src/tui/fs_watch.rs", "tui.refresh_rate_ms"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Redraw/watcher fixes (88c724744: honor refresh cadence, dirty-flag redraw, skip broad .roko watcher when StateHub is authoritative) are source-only; idle/active CPU, draw count, watcher events and input latency measurements remain pending.

Imported without verification from:
- `tmp/dev-audit/09-additional-live-run-findings.md#The connected TUI does unnecessary idle work`

Some cited files are gone: `crates/roko-cli/src/tui/app.rs`.

How to verify: Check app loop honors refresh_rate_ms and dirty redraw; profile idle CPU with a finished run open.
