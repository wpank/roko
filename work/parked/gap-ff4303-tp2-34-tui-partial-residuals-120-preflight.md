+++
id = "gap-ff4303"
kind = "gap"
title = "TP2-34: TUI partial residuals (#120 preflight repair, #126 error digest dedupe, #127 F7 fields, #189 agent fields)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial"
discovered_from = "audit:tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial"
anchors = ["runner/preflight.rs", "widgets/error_digest.rs", "views/context_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Preflight warnings lack a tested decision/repair flow; error digest lacks cross-source dedupe/remediation lifecycle; several connected F7 fields are aggregate/inferred/empty; agent panel effort/context/turn fields not reliably populated.

Imported without verification from:
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`

How to verify: Spot-check each widget with a live snapshot.
