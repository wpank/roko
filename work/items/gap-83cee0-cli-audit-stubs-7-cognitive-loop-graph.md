+++
id = "gap-83cee0"
kind = "gap"
title = "[cli-audit stubs] 7 cognitive-loop Graph cells registered as PassthroughCell stubs"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/cells"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/22-stubs-unimplemented.md#4c. PassthroughCell -- cognitive loop graph stubs"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/22-stubs-unimplemented.md#4c. PassthroughCell -- cognitive loop graph stubs"
anchors = ["PassthroughCell", "crates/roko-graph/src/cells/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Seven cognitive-loop cells were registered as PassthroughCell (pass input unchanged), affecting Graph execution. CLAUDE.md now claims seven cognitive Cells are wired in roko-graph.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/22-stubs-unimplemented.md#4c. PassthroughCell -- cognitive loop graph stubs`
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep PassthroughCell registrations in roko-graph; confirm cognitive cells have real implementations.
