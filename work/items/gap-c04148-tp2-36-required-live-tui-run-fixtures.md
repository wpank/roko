+++
id = "gap-c04148"
kind = "gap"
title = "TP2-36: Required live TUI/run fixtures never executed"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/tui-parity2/36-OPEN-GAPS.md#Required live fixtures"
discovered_from = "audit:tmp/tui-parity2/36-OPEN-GAPS.md#Required live fixtures"
anchors = ["roko plan run --screenshots", "ScreenshotCollector"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Unrun: cold-cache warmup captures, long tool call liveness, long gate line streaming, pause acknowledgement, plan-scoped cancel/recovery, terminal snapshots, resize 80x24-200x60, low-disk capture, 5 cold/5 warm runs; plus #112 live screenshots and the tui-ux 11-item checklist.

Imported without verification from:
- `tmp/tui-parity2/36-OPEN-GAPS.md#Required live fixtures`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Immediate closure order`
- `tmp/archive/tui-ux-audit/30-OPEN-GAPS.md#Verification Checklist`
- `tmp/tui-parity/00-INDEX.md#Dev-audit integration follow-ups (outside the 38-item denominator)`

How to verify: Look for evidence bundles/screenshots from a post-09-14 live run.
