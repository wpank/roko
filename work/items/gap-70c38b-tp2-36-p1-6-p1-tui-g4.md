+++
id = "gap-70c38b"
kind = "gap"
title = "TP2-36 P1.6/P1-TUI-G4 (+MX.8): Unified semantic transcript model"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/dashboard"]
created = 2026-09-20
updated = 2026-09-28
source = "tmp/tui-parity2/36-OPEN-GAPS.md#P1 — information needed during a run"
discovered_from = "audit:tmp/tui-parity2/36-OPEN-GAPS.md#P1 — information needed during a run"
anchors = ["tmp/tool-audit/01-event-schema.md", "tui/widgets/stream_output.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Replace provider-specific/string-tail projections with one canonical event/transcript model preserving tool, reasoning, todo and subagent semantics (tool-audit schema). Transcript widget claimed done 09-19; unified model still open 09-20.

Imported without verification from:
- `tmp/tui-parity2/36-OPEN-GAPS.md#P1 — information needed during a run`
- `tmp/tui-parity2/36-OPEN-GAPS.md#Remaining after 2026-09-04 closure`
- `tmp/tui-parity/00-INDEX.md#3. New items from the UX audit not in this tracker`
- `tmp/dogfood/2026-09-20-final-session.md#P1 (High)`

Some cited files are gone: `tmp/tool-audit/01-event-schema.md`.

How to verify: Check whether all providers emit the same typed transcript events.
