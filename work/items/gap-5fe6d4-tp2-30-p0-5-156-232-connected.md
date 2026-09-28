+++
id = "gap-5fe6d4"
kind = "gap"
title = "TP2-30 P0.5 / #156/#232: Connected learning history and cost-by-model still partial"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/tui-parity2/30-VERIFIED-CLAIM-MATRIX.md#Verified P0-P7 claim matrix"
discovered_from = "audit:tmp/tui-parity2/30-VERIFIED-CLAIM-MATRIX.md#Verified P0-P7 claim matrix"
anchors = ["widgets/cost_by_model.rs", "TuiBridge::efficiency_event"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Aggregates cross the bridge, but typed learning history and authoritative per-model cost events are incomplete (cost-by-model inferred from global totals); overlaps the 09-25 finding that TUI tokens/cost stay $0.00.

Imported without verification from:
- `tmp/tui-parity2/30-VERIFIED-CLAIM-MATRIX.md#Verified P0-P7 claim matrix`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-7. Tokens and cost are structurally $0.00`

How to verify: Run a plan in connected mode and inspect F7/F10 model rows.
