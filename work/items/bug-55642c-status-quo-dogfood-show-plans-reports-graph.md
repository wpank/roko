+++
id = "bug-55642c"
kind = "bug"
title = "[status-quo dogfood] `show plans` reports Graph-executed plans as pending"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/show"]
created = 2026-09-10
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
anchors = [".roko/state/graph/<plan>/checkpoint.json", "roko show plans"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Graph engine does not write completion back to tasks.toml; state lives in .roko/state/graph/<plan>/checkpoint.json, so `show plans` shows every plan as pending.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)`

Warning: every file this item cites is gone (`.roko/state/graph/<plan>/checkpoint.json`) — likely obsolete or moved.

How to verify: Run a plan via Graph then `roko show plans`; check status column.
