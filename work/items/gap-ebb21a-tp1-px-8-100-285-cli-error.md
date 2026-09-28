+++
id = "gap-ebb21a"
kind = "gap"
title = "TP1-PX.8 #100: ~285 CLI error paths lack recovery hints"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)"
discovered_from = "audit:tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)"
anchors = ["eprintln!", "commands/util.rs recovery hints"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
CLI error-message quality audit counted about 285 eprintln! error paths in roko-cli without actionable recovery hints.

Imported without verification from:
- `tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)`
- `tmp/tui-parity/audits/mori-parity-status.md#PARTIAL -- Other (5 items)`

How to verify: Count eprintln! without hint text in crates/roko-cli/src.
