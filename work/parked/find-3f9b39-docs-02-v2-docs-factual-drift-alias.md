+++
id = "find-3f9b39"
kind = "finding"
title = "DOCS-02: v2 docs factual drift (alias direction, counts, explain refs, DeFi overstatement, Runner-v2-primary depth files)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["docs/v2"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/02-DOCS-VS-CODE.md#Factual Errors to Fix"
discovered_from = "audit:tmp/docs-audit/02-DOCS-VS-CODE.md#Factual Errors to Fix"
anchors = ["docs/v2/01-SIGNAL.md", "docs/v2/24-DEFI.md", "docs/v2-depth/07-agent-runtime/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Listed v2 errors: Signal/Engram alias direction, gate/TUI-tab/route/template/watcher counts, stale `roko explain` references, DeFi overstatement (DeFiRiskEngine/VenueAdapter/ClearingHouse absent), 35 v2-depth agent-runtime files treating Runner-v2 as primary.

Imported without verification from:
- `tmp/docs-audit/02-DOCS-VS-CODE.md#Factual Errors to Fix`
- `tmp/docs-audit/02-DOCS-VS-CODE.md#DeFi Overstatement (v2 24-DEFI.md)`
- `tmp/docs-audit/02-DOCS-VS-CODE.md#v2-depth Files That Need Updating`

How to verify: Spot-check each listed claim in docs/v2 against code counts.
