+++
id = "gap-db648b"
kind = "gap"
title = "TP1-PX.9/PX.10: ratatui setup wizard (#223) and `roko undo` verb"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/setup"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)"
discovered_from = "audit:tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)"
anchors = ["commands/setup.rs", "roko undo"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Setup wizard remains stdin-interactive rather than the 5-phase ratatui flow; progressive-formality `roko undo` verb has no enum variant or handler.

Imported without verification from:
- `tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)`
- `tmp/tui-parity/audits/mori-parity-status.md#PARTIAL -- Spec-vs-implementation scope (3 items, Pattern 4)`

How to verify: Check setup.rs UI mode and for an Undo command.
