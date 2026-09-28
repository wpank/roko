+++
id = "gap-69e3b9"
kind = "gap"
title = "TP2-36 P1.3/P1-TUI-G2, #367: Provider transcript completeness and paged output history"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-20
updated = 2026-09-28
source = "tmp/tui-parity2/36-OPEN-GAPS.md#P1 — information needed during a run"
discovered_from = "audit:tmp/tui-parity2/36-OPEN-GAPS.md#P1 — information needed during a run"
anchors = ["#108", "#232", "#367"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Semantic tool segments must be preserved per provider and agent transcripts need canonical paging, historical search and dedupe across live/settled storage (history is memory-bounded only). Still open 09-20.

Imported without verification from:
- `tmp/tui-parity2/36-OPEN-GAPS.md#P1 — information needed during a run`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Missing`
- `tmp/dogfood/2026-09-20-final-session.md#P1 (High)`

How to verify: Check for paged transcript storage/search APIs.
